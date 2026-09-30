//! The editing core, independent of any UI: a [`Workspace`] holds an open
//! module, caches its parsed GFF documents, and applies every change as a
//! [`Command`] of primitive [`Edit`]s. Applying an edit records its inverse,
//! so undo and redo are exact, and every editor, the CLI and the tests change
//! modules through the same path.

use std::collections::HashMap;
use std::fmt;

use mg_gff::{Gff, Label, Struct, Value};
use mg_module::{Module, ModuleError};
use mg_resman::ResKey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EditError {
    #[error("{0} is not in the module")]
    NoResource(ResKey),
    #[error("{0} is not a GFF resource: {1}")]
    NotGff(ResKey, String),
    #[error("{key}: no struct at {path}")]
    BadPath { key: ResKey, path: GffPath },
    #[error("{key}: {message}")]
    Invalid { key: ResKey, message: String },
    #[error("no command to amend")]
    NothingToAmend,
    #[error(transparent)]
    Module(#[from] ModuleError),
}

/// One step into a GFF tree: a struct field, or an item of a list field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Step {
    Field(String),
    Item(String, usize),
}

/// A path from a GFF's root to a struct inside it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct GffPath(pub Vec<Step>);

impl GffPath {
    pub fn root() -> GffPath {
        GffPath(Vec::new())
    }

    /// This path extended into item `index` of list `label`.
    pub fn item(&self, label: &str, index: usize) -> GffPath {
        let mut p = self.clone();
        p.0.push(Step::Item(label.to_string(), index));
        p
    }

    /// This path extended into struct field `label`.
    pub fn field(&self, label: &str) -> GffPath {
        let mut p = self.clone();
        p.0.push(Step::Field(label.to_string()));
        p
    }

    /// The struct this path leads to from `s`.
    pub fn get<'a>(&self, mut s: &'a Struct) -> Option<&'a Struct> {
        for step in &self.0 {
            s = match step {
                Step::Field(l) => s.child(l)?,
                Step::Item(l, i) => s.list(l)?.get(*i)?,
            };
        }
        Some(s)
    }

    fn resolve<'a>(&self, mut s: &'a mut Struct) -> Option<&'a mut Struct> {
        for step in &self.0 {
            s = match step {
                Step::Field(l) => s.child_mut(l)?,
                Step::Item(l, i) => s.list_mut(l)?.get_mut(*i)?,
            };
        }
        Some(s)
    }
}

impl fmt::Display for GffPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("/");
        }
        for step in &self.0 {
            match step {
                Step::Field(l) => write!(f, "/{l}")?,
                Step::Item(l, i) => write!(f, "/{l}[{i}]")?,
            }
        }
        Ok(())
    }
}

/// A primitive change. Each has an exact inverse.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Set a field of the struct at `path` (`None` removes it). The field
    /// keeps its position when it exists; a new field is appended.
    SetField { key: ResKey, path: GffPath, label: String, value: Option<Value> },
    /// Insert `item` into list `list` of the struct at `path` at `index`
    /// (creating the list if needed).
    InsertItem { key: ResKey, path: GffPath, list: String, index: usize, item: Struct },
    /// Remove item `index` of list `list`.
    RemoveItem { key: ResKey, path: GffPath, list: String, index: usize },
    /// Add or replace a whole resource (`None` removes it).
    SetResource { key: ResKey, data: Option<Vec<u8>> },
}

/// A named group of edits, applied and undone as one.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub label: String,
    pub edits: Vec<Edit>,
}

impl Command {
    pub fn new(label: impl Into<String>, edits: Vec<Edit>) -> Command {
        Command { label: label.into(), edits }
    }
}

/// An open module with its parsed documents and edit history.
#[derive(Debug)]
pub struct Workspace {
    pub module: Module,
    docs: HashMap<ResKey, Gff>,
    undo: Vec<Command>,
    redo: Vec<Command>,
    /// Commands applied since the last save (undo past it counts down).
    changes_since_save: isize,
    /// Counts every change (apply, undo, redo), for views that cache.
    revision: u64,
}

impl Workspace {
    pub fn new(module: Module) -> Workspace {
        Workspace {
            module,
            docs: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            changes_since_save: 0,
            revision: 0,
        }
    }

    /// A number that changes whenever the module does (through commands).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// A GFF document, parsed on first use and cached.
    pub fn doc(&mut self, key: &ResKey) -> Result<&Gff, EditError> {
        self.doc_mut(key).map(|g| &*g)
    }

    fn doc_mut(&mut self, key: &ResKey) -> Result<&mut Gff, EditError> {
        if !self.docs.contains_key(key) {
            let bytes = self.module.get(key).ok_or(EditError::NoResource(*key))?;
            let gff = Gff::read(bytes).map_err(|e| EditError::NotGff(*key, e.to_string()))?;
            self.docs.insert(*key, gff);
        }
        Ok(self.docs.get_mut(key).expect("just inserted"))
    }

    /// Whether there are changes since opening or the last save.
    pub fn is_modified(&self) -> bool {
        self.changes_since_save != 0
    }

    pub fn can_undo(&self) -> Option<&str> {
        self.undo.last().map(|c| c.label.as_str())
    }

    pub fn can_redo(&self) -> Option<&str> {
        self.redo.last().map(|c| c.label.as_str())
    }

    /// Applies a command; on failure nothing is changed.
    pub fn apply(&mut self, cmd: Command) -> Result<(), EditError> {
        let inverse = self.run(&cmd)?;
        self.undo.push(inverse);
        self.redo.clear();
        self.changes_since_save += 1;
        self.revision += 1;
        Ok(())
    }

    /// Runs more edits as part of the last command, so that one undo
    /// reverts both (a value derived from what the command changed, such as
    /// an item's cost). On failure nothing is changed.
    pub fn amend(&mut self, edits: Vec<Edit>) -> Result<(), EditError> {
        let Some(label) = self.undo.last().map(|c| c.label.clone()) else {
            return Err(EditError::NothingToAmend);
        };
        let inverse = self.run(&Command::new(label, edits))?;
        let last = self.undo.last_mut().expect("checked above");
        // Undo reverts the amendment first, then the command.
        let earlier = std::mem::replace(&mut last.edits, inverse.edits);
        last.edits.extend(earlier);
        self.revision += 1;
        Ok(())
    }

    /// Undoes the last command; returns its label.
    pub fn undo(&mut self) -> Result<Option<String>, EditError> {
        let Some(inverse) = self.undo.pop() else { return Ok(None) };
        let forward = self.run(&inverse)?;
        let label = forward.label.clone();
        self.redo.push(forward);
        self.changes_since_save -= 1;
        self.revision += 1;
        Ok(Some(label))
    }

    /// Redoes the last undone command; returns its label.
    pub fn redo(&mut self) -> Result<Option<String>, EditError> {
        let Some(forward) = self.redo.pop() else { return Ok(None) };
        let inverse = self.run(&forward)?;
        let label = inverse.label.clone();
        self.undo.push(inverse);
        self.changes_since_save += 1;
        self.revision += 1;
        Ok(Some(label))
    }

    /// Runs the edits in order and returns the command that reverses them.
    /// If one fails, the ones already run are reverted.
    fn run(&mut self, cmd: &Command) -> Result<Command, EditError> {
        let mut inverses = Vec::with_capacity(cmd.edits.len());
        for e in &cmd.edits {
            match self.run_edit(e) {
                Ok(inv) => inverses.push(inv),
                Err(err) => {
                    for inv in inverses.into_iter().rev() {
                        let _ = self.run_edit(&inv);
                    }
                    return Err(err);
                }
            }
        }
        inverses.reverse();
        Ok(Command { label: cmd.label.clone(), edits: inverses })
    }

    fn run_edit(&mut self, e: &Edit) -> Result<Edit, EditError> {
        match e {
            Edit::SetResource { key, data } => {
                // The cached document may hold edits not yet in the module;
                // the undo copy must include them.
                if let Some(doc) = self.docs.remove(key) {
                    let bytes = doc
                        .to_bytes()
                        .map_err(|e| EditError::Invalid { key: *key, message: e.to_string() })?;
                    self.module.set(*key, bytes);
                }
                let old = self.module.get(key).map(<[u8]>::to_vec);
                match data {
                    Some(d) => self.module.set(*key, d.clone()),
                    None => {
                        self.module.remove(key);
                    }
                }
                Ok(Edit::SetResource { key: *key, data: old })
            }
            Edit::SetField { key, path, label, value } => {
                let s = self.struct_at(key, path)?;
                let old = s.get(label).cloned();
                match value {
                    Some(v) => {
                        if Label::new(label).is_err() {
                            return Err(EditError::Invalid {
                                key: *key,
                                message: format!("label {label:?} is too long"),
                            });
                        }
                        s.set(label, v.clone())
                    }
                    None => {
                        s.remove(label);
                    }
                }
                Ok(Edit::SetField {
                    key: *key,
                    path: path.clone(),
                    label: label.clone(),
                    value: old,
                })
            }
            Edit::InsertItem { key, path, list, index, item } => {
                let s = self.struct_at(key, path)?;
                if s.get(list).is_none() {
                    s.set(list, Value::List(Vec::new()));
                }
                let items = s.list_mut(list).ok_or_else(|| EditError::Invalid {
                    key: *key,
                    message: format!("{path}/{list} is not a list"),
                })?;
                if *index > items.len() {
                    return Err(EditError::Invalid {
                        key: *key,
                        message: format!("{path}/{list}: no position {index}"),
                    });
                }
                items.insert(*index, item.clone());
                Ok(Edit::RemoveItem {
                    key: *key,
                    path: path.clone(),
                    list: list.clone(),
                    index: *index,
                })
            }
            Edit::RemoveItem { key, path, list, index } => {
                let s = self.struct_at(key, path)?;
                let items = s.list_mut(list).filter(|l| *index < l.len()).ok_or_else(|| {
                    EditError::Invalid {
                        key: *key,
                        message: format!("{path}/{list}: no item {index}"),
                    }
                })?;
                let item = items.remove(*index);
                Ok(Edit::InsertItem {
                    key: *key,
                    path: path.clone(),
                    list: list.clone(),
                    index: *index,
                    item,
                })
            }
        }
    }

    fn struct_at(&mut self, key: &ResKey, path: &GffPath) -> Result<&mut Struct, EditError> {
        let doc = self.doc_mut(key)?;
        path.resolve(&mut doc.root)
            .ok_or_else(|| EditError::BadPath { key: *key, path: path.clone() })
    }

    /// Writes changed documents back into the module.
    pub fn flush(&mut self) -> Result<(), EditError> {
        for (key, gff) in &self.docs {
            let bytes = gff
                .to_bytes()
                .map_err(|e| EditError::Invalid { key: *key, message: e.to_string() })?;
            if self.module.get(key) != Some(&bytes[..]) {
                self.module.set(*key, bytes);
            }
        }
        Ok(())
    }

    /// Flushes and saves the module to where it was opened from.
    pub fn save(&mut self) -> Result<(), EditError> {
        self.flush()?;
        self.module.save()?;
        self.changes_since_save = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{ResRef, ResType};
    use mg_schema::{StructExt, git, ifo};

    use super::*;

    fn key(n: &str, t: ResType) -> ResKey {
        ResKey::new(ResRef::from_str(n).unwrap(), t)
    }

    fn workspace() -> Workspace {
        let mut m = Module::new();
        let mut info = Gff::new(*b"IFO ");
        info.root.write(&ifo::MOD_XP_SCALE, 10);
        m.set_info(&info).unwrap();
        let mut g = Gff::new(*b"GIT ");
        let mut c = git::CREATURE_LIST.new_item();
        c.write(&git::creature_list::TEMPLATE_RES_REF, ResRef::from_str("chicken").unwrap());
        g.root.items_mut(&git::CREATURE_LIST).push(c);
        m.set_gff(key("area", ResType::GIT), &g).unwrap();
        Workspace::new(m)
    }

    #[test]
    fn set_field_undo_redo() {
        let mut ws = workspace();
        let ifo_key = key("module", ResType::IFO);
        ws.apply(Command::new(
            "XP scale",
            vec![Edit::SetField {
                key: ifo_key,
                path: GffPath::root(),
                label: "Mod_XPScale".into(),
                value: Some(Value::Byte(50)),
            }],
        ))
        .unwrap();
        assert_eq!(ws.doc(&ifo_key).unwrap().root.read(&ifo::MOD_XP_SCALE), 50);
        assert!(ws.is_modified());
        assert_eq!(ws.undo().unwrap().as_deref(), Some("XP scale"));
        assert_eq!(ws.doc(&ifo_key).unwrap().root.read(&ifo::MOD_XP_SCALE), 10);
        assert!(!ws.is_modified(), "undone back to the saved state");
        ws.redo().unwrap();
        assert_eq!(ws.doc(&ifo_key).unwrap().root.read(&ifo::MOD_XP_SCALE), 50);
    }

    #[test]
    fn list_items_and_nested_paths() {
        let mut ws = workspace();
        let k = key("area", ResType::GIT);
        let mut chicken2 = git::CREATURE_LIST.new_item();
        chicken2
            .write(&git::creature_list::TEMPLATE_RES_REF, ResRef::from_str("chicken2").unwrap());
        ws.apply(Command::new(
            "Add and rename",
            vec![
                Edit::InsertItem {
                    key: k,
                    path: GffPath::root(),
                    list: "Creature List".into(),
                    index: 1,
                    item: chicken2,
                },
                Edit::SetField {
                    key: k,
                    path: GffPath::root().item("Creature List", 0),
                    label: "Tag".into(),
                    value: Some(Value::String(b"FIRST".to_vec())),
                },
            ],
        ))
        .unwrap();
        let root = &ws.doc(&k).unwrap().root;
        assert_eq!(root.items(&git::CREATURE_LIST).len(), 2);
        assert_eq!(root.items(&git::CREATURE_LIST)[0].string("Tag"), Some(&b"FIRST"[..]));
        ws.undo().unwrap();
        let root = &ws.doc(&k).unwrap().root;
        assert_eq!(root.items(&git::CREATURE_LIST).len(), 1);
        assert!(!root.items(&git::CREATURE_LIST)[0].contains("Tag"));
    }

    #[test]
    fn amendments_undo_and_redo_with_their_command() {
        let mut ws = workspace();
        let k = key("module", ResType::IFO);
        let set = |label: &str, v: i32| Edit::SetField {
            key: k,
            path: GffPath::root(),
            label: label.into(),
            value: Some(Value::Int(v)),
        };
        assert!(matches!(ws.amend(vec![set("B", 1)]), Err(EditError::NothingToAmend)));
        ws.apply(Command::new("Set A", vec![set("A", 1)])).unwrap();
        ws.amend(vec![set("B", 2), set("A", 3)]).unwrap();
        let root = |ws: &mut Workspace| {
            let r = &ws.doc(&k).unwrap().root;
            (r.integer("A"), r.integer("B"))
        };
        assert_eq!(root(&mut ws), (Some(3), Some(2)));
        assert_eq!(ws.can_undo(), Some("Set A"));
        assert_eq!(ws.undo().unwrap().as_deref(), Some("Set A"));
        assert_eq!(root(&mut ws), (None, None));
        ws.redo().unwrap();
        assert_eq!(root(&mut ws), (Some(3), Some(2)));
    }

    #[test]
    fn failed_commands_change_nothing() {
        let mut ws = workspace();
        let k = key("area", ResType::GIT);
        let before = ws.doc(&k).unwrap().clone();
        let err = ws.apply(Command::new(
            "Half bad",
            vec![
                Edit::SetField {
                    key: k,
                    path: GffPath::root(),
                    label: "A".into(),
                    value: Some(Value::Int(1)),
                },
                Edit::RemoveItem {
                    key: k,
                    path: GffPath::root(),
                    list: "Creature List".into(),
                    index: 7,
                },
            ],
        ));
        assert!(err.is_err());
        assert_eq!(*ws.doc(&k).unwrap(), before);
        assert!(ws.can_undo().is_none());
    }

    #[test]
    fn replacing_a_resource_keeps_unflushed_edits_for_undo() {
        let mut ws = workspace();
        let k = key("area", ResType::GIT);
        let set = |v| Edit::SetField {
            key: k,
            path: GffPath::root(),
            label: "N".into(),
            value: Some(Value::Int(v)),
        };
        ws.apply(Command::new("N=1", vec![set(1)])).unwrap();
        ws.apply(Command::new(
            "Replace",
            vec![Edit::SetResource { key: k, data: Some(Gff::new(*b"GIT ").to_bytes().unwrap()) }],
        ))
        .unwrap();
        assert!(!ws.doc(&k).unwrap().root.contains("N"));
        ws.undo().unwrap();
        assert_eq!(
            ws.doc(&k).unwrap().root.int("N"),
            Some(1),
            "the edit made before the replacement is back"
        );
        ws.undo().unwrap();
        assert!(!ws.doc(&k).unwrap().root.contains("N"));
    }

    #[test]
    fn flush_and_resource_edits() {
        let mut ws = workspace();
        let k = key("area", ResType::GIT);
        ws.apply(Command::new(
            "Tag",
            vec![Edit::SetField {
                key: k,
                path: GffPath::root(),
                label: "Extra".into(),
                value: Some(Value::Int(3)),
            }],
        ))
        .unwrap();
        ws.flush().unwrap();
        assert_eq!(Gff::read(ws.module.get(&k).unwrap()).unwrap().root.int("Extra"), Some(3));
        let script = key("new_script", ResType::NSS);
        ws.apply(Command::new(
            "New script",
            vec![Edit::SetResource { key: script, data: Some(b"void main(){}".to_vec()) }],
        ))
        .unwrap();
        assert!(ws.module.contains(&script));
        ws.undo().unwrap();
        assert!(!ws.module.contains(&script));
    }
}
