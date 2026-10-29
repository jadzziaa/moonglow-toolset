//! The editing core, independent of any UI: a [`Workspace`] holds an open
//! module, caches its parsed GFF documents, and applies every change as a
//! [`Command`] of primitive [`Edit`]s. Applying an edit records its inverse,
//! so undo and redo are exact, and every editor, the CLI and the tests change
//! modules through the same path.

use std::collections::{HashMap, HashSet};
use std::fmt;

use mg_gff::{Gff, Label, Struct, Value};
use mg_module::{Module, ModuleError};
use mg_resman::ResKey;
use thiserror::Error;

pub mod fix;
pub mod found;
pub mod terrain;
pub mod wire;

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

    /// The struct this path leads to from `s`, to change.
    pub fn get_mut<'a>(&self, mut s: &'a mut Struct) -> Option<&'a mut Struct> {
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
            // (Written so that it reads back: `wire`'s `FromStr`.)
            match step {
                Step::Field(l) => write!(f, "/{}", wire::escape(l))?,
                Step::Item(l, i) => write!(f, "/{}[{i}]", wire::escape(l))?,
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

impl Edit {
    /// [`Edit::InsertItem`]: `item` into list `list` of the struct at
    /// `path`, at `index`.
    pub fn insert_item(
        key: ResKey,
        path: GffPath,
        list: impl Into<String>,
        index: usize,
        item: Struct,
    ) -> Edit {
        Edit::InsertItem { key, path, list: list.into(), index, item }
    }

    /// [`Edit::RemoveItem`]: item `index` of list `list` of the struct at
    /// `path`.
    pub fn remove_item(key: ResKey, path: GffPath, list: impl Into<String>, index: usize) -> Edit {
        Edit::RemoveItem { key, path, list: list.into(), index }
    }

    /// The resource it changes.
    pub fn key(&self) -> &ResKey {
        match self {
            Edit::SetField { key, .. }
            | Edit::InsertItem { key, .. }
            | Edit::RemoveItem { key, .. }
            | Edit::SetResource { key, .. } => key,
        }
    }
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

/// The edits that make module `before` into `after`: a resource set where
/// `after` has another or a new one, removed where it has none.
pub fn edits_between(before: &Module, after: &Module) -> Vec<Edit> {
    let changed = after
        .keys()
        .filter(|k| after.get(k) != before.get(k))
        .map(|k| Edit::SetResource { key: *k, data: after.get(k).map(<[u8]>::to_vec) });
    let gone = before
        .keys()
        .filter(|k| !after.contains(k))
        .map(|k| Edit::SetResource { key: *k, data: None });
    changed.chain(gone).collect()
}

/// An open module with its parsed documents and edit history.
#[derive(Debug)]
pub struct Workspace {
    pub module: Module,
    docs: HashMap<ResKey, Gff>,
    /// The documents edited since the last [`Workspace::flush`]: only
    /// those are written back (a document only read stays as the module
    /// has it, and costs nothing to flush).
    edited: HashSet<ResKey>,
    /// Documents shown that aren't the module's (the game's blueprints, to
    /// look at): [`Workspace::doc`] gives them, nothing edits or saves them.
    viewed: HashMap<ResKey, Gff>,
    undo: Vec<Command>,
    redo: Vec<Command>,
    /// Commands applied since the last save (undo past it counts down).
    changes_since_save: isize,
    /// Moves on every change (apply, undo, redo), for views that cache.
    revision: u64,
    /// [`Workspace::rewrites`], by resource.
    rewrites: HashMap<ResKey, u64>,
    /// The resources as opened or last saved ([`Workspace::saved`]).
    saved: Module,
}

/// A module's resources alone, sharing their data.
fn resources_of(module: &Module) -> Module {
    let mut out = Module::new();
    for key in module.keys() {
        if let Some(data) = module.shared(key) {
            out.set(*key, data.clone());
        }
    }
    out
}

/// Where a new workspace's revisions start: each its own range (a change
/// moves it by one), so a view kept from another module never takes this
/// one's area of the same name for its own (Jasperre's report).
fn first_revision() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) << 32
}

impl Workspace {
    pub fn new(module: Module) -> Workspace {
        Workspace {
            saved: resources_of(&module),
            module,
            docs: HashMap::new(),
            edited: HashSet::new(),
            viewed: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            changes_since_save: 0,
            revision: first_revision(),
            rewrites: HashMap::new(),
        }
    }

    /// A number that changes whenever the module does (through commands),
    /// and that no other workspace has had.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// How many times resource `key` was replaced whole or had a list at
    /// its root rewritten (set as a whole, an item put in or taken out),
    /// by a command, its undo or redo, or a file read again
    /// ([`Workspace::adopt`]): what holds an item of such a list by its
    /// place in it (an area's tile by its index in `Tile_List`) may hold
    /// another since the number moved. Edits inside the items don't count,
    /// nor [`Workspace::derive`] (what it makes is made from the others).
    pub fn rewrites(&self, key: &ResKey) -> u64 {
        self.rewrites.get(key).copied().unwrap_or(0)
    }

    fn rewritten(&mut self, key: &ResKey) {
        *self.rewrites.entry(*key).or_default() += 1;
    }

    /// The module's resources as it was opened or last saved (with the
    /// files changed outside that were taken in since, [`Workspace::adopt`],
    /// but blueprints): what a save compares the module with, to change
    /// what is made from it only where something changed (the custom
    /// palettes, `mg_module::palette::update_custom_palettes`). A blueprint
    /// added, changed or deleted outside counts as changed, so that its
    /// palette entry follows it.
    pub fn saved(&self) -> &Module {
        &self.saved
    }

    /// A GFF document, parsed on first use and cached: the module's, else
    /// one given to look at ([`Workspace::view`]).
    pub fn doc(&mut self, key: &ResKey) -> Result<&Gff, EditError> {
        if self.is_viewed(key) {
            return Ok(&self.viewed[key]);
        }
        self.doc_mut(key).map(|g| &*g)
    }

    /// Gives a document that isn't the module's to look at under `key`:
    /// [`Workspace::doc`] reads it while the module has no resource of that
    /// name. It is never edited (an edit of it fails, as of any resource
    /// the module lacks) and never saved.
    pub fn view(&mut self, key: ResKey, gff: Gff) {
        self.viewed.insert(key, gff);
    }

    /// Whether `key` is a document only looked at ([`Workspace::view`]).
    pub fn is_viewed(&self, key: &ResKey) -> bool {
        self.viewed.contains_key(key) && !self.module.contains(key)
    }

    /// A GFF document if it has been used (and so may differ from the
    /// module's stored copy), without parsing it otherwise.
    pub fn loaded(&self, key: &ResKey) -> Option<&Gff> {
        self.docs.get(key)
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

    /// The edits [`Workspace::undo`] would run next (the last command's
    /// inverse), to see what they move before they are run.
    pub fn peek_undo(&self) -> Option<&Command> {
        self.undo.last()
    }

    /// The edits [`Workspace::redo`] would run next.
    pub fn peek_redo(&self) -> Option<&Command> {
        self.redo.last()
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

    /// Undoes the last command; returns its label. On failure nothing is
    /// changed and the command stays to be undone.
    pub fn undo(&mut self) -> Result<Option<String>, EditError> {
        let Some(inverse) = self.undo.pop() else { return Ok(None) };
        let forward = match self.run(&inverse) {
            Ok(forward) => forward,
            Err(e) => {
                self.undo.push(inverse);
                return Err(e);
            }
        };
        let label = forward.label.clone();
        self.redo.push(forward);
        self.changes_since_save -= 1;
        self.revision += 1;
        Ok(Some(label))
    }

    /// Takes the last command back and forgets it: undone, with nothing to
    /// redo (a draft withdrawn, such as a window's changes shown as they
    /// are made and then cancelled). Returns its label.
    pub fn withdraw(&mut self) -> Result<Option<String>, EditError> {
        let Some(inverse) = self.undo.pop() else { return Ok(None) };
        let label = match self.run(&inverse) {
            Ok(forward) => forward.label,
            Err(e) => {
                self.undo.push(inverse);
                return Err(e);
            }
        };
        self.changes_since_save -= 1;
        self.revision += 1;
        Ok(Some(label))
    }

    /// Redoes the last undone command; returns its label. On failure
    /// nothing is changed and the command stays to be redone.
    pub fn redo(&mut self) -> Result<Option<String>, EditError> {
        let Some(forward) = self.redo.pop() else { return Ok(None) };
        let inverse = match self.run(&forward) {
            Ok(inverse) => inverse,
            Err(e) => {
                self.redo.push(forward);
                return Err(e);
            }
        };
        let label = inverse.label.clone();
        self.undo.push(inverse);
        self.changes_since_save += 1;
        self.revision += 1;
        Ok(Some(label))
    }

    /// Runs the edits in order and returns the command that reverses them.
    /// If one fails, the ones already run are reverted.
    fn run(&mut self, cmd: &Command) -> Result<Command, EditError> {
        let mut inverses: Vec<Vec<Edit>> = Vec::with_capacity(cmd.edits.len());
        for e in &cmd.edits {
            match self.run_edit(e) {
                Ok(inv) => inverses.push(inv),
                Err(err) => {
                    for inv in inverses.into_iter().rev().flatten() {
                        let _ = self.run_edit(&inv);
                    }
                    return Err(err);
                }
            }
        }
        let edits = inverses.into_iter().rev().flatten().collect();
        Ok(Command { label: cmd.label.clone(), edits })
    }

    /// Runs one edit; returns the edits that take it back, in the order
    /// they run (most often one).
    fn run_edit(&mut self, e: &Edit) -> Result<Vec<Edit>, EditError> {
        match e {
            Edit::SetResource { key, data } => {
                // The cached document may hold edits not yet in the module;
                // the undo copy must include them.
                if let Some(doc) = self.docs.remove(key)
                    && self.edited.remove(key)
                {
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
                self.rewritten(key);
                Ok(vec![Edit::SetResource { key: *key, data: old }])
            }
            Edit::SetField { key, path, label, value: None }
                if let Some(back) = self.removed_in_place(key, path, label)? =>
            {
                let s = self.struct_at(key, path)?;
                let old = s.remove(label);
                if path.0.is_empty() && matches!(old, Some(Value::List(_))) {
                    self.rewritten(key);
                }
                Ok(back)
            }
            Edit::SetField { key, path, label, value } => {
                let s = self.struct_at(key, path)?;
                let old = s.get(label).cloned();
                let list = |v: &Option<Value>| matches!(v, Some(Value::List(_)));
                let rewrites = path.0.is_empty() && (list(&old) || list(value));
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
                if rewrites {
                    self.rewritten(key);
                }
                Ok(vec![Edit::SetField {
                    key: *key,
                    path: path.clone(),
                    label: label.clone(),
                    value: old,
                }])
            }
            Edit::InsertItem { key, path, list, index, item } => {
                let s = self.struct_at(key, path)?;
                // (A list made for the item goes again with it.)
                let made = s.get(list).is_none();
                if made {
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
                if path.0.is_empty() {
                    self.rewritten(key);
                }
                if made {
                    let (path, label) = (path.clone(), list.clone());
                    return Ok(vec![Edit::SetField { key: *key, path, label, value: None }]);
                }
                Ok(vec![Edit::RemoveItem {
                    key: *key,
                    path: path.clone(),
                    list: list.clone(),
                    index: *index,
                }])
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
                if path.0.is_empty() {
                    self.rewritten(key);
                }
                Ok(vec![Edit::InsertItem {
                    key: *key,
                    path: path.clone(),
                    list: list.clone(),
                    index: *index,
                    item,
                }])
            }
        }
    }

    /// What takes back the removal of field `label` from the struct at
    /// `path` where putting the field back (appended) would not: one that
    /// is not the struct's last, or one of several of that label. The
    /// struct is put back whole: as its list's item, as its parent's
    /// field, or the root with the resource. `None`: no such field, or the
    /// plain inverse is exact.
    fn removed_in_place(
        &mut self,
        key: &ResKey,
        path: &GffPath,
        label: &str,
    ) -> Result<Option<Vec<Edit>>, EditError> {
        let s = self.struct_at(key, path)?;
        let same = |f: &&mg_gff::Field| f.label.as_bytes() == label.as_bytes();
        let last = s.fields.last().is_some_and(|f| same(&f));
        if s.fields.iter().filter(same).count() == usize::from(last) {
            return Ok(None);
        }
        let old = s.clone();
        let Some((step, parent)) = path.0.split_last() else {
            let doc = self.doc_mut(key)?;
            let bytes = doc
                .to_bytes()
                .map_err(|e| EditError::Invalid { key: *key, message: e.to_string() })?;
            return Ok(Some(vec![Edit::SetResource { key: *key, data: Some(bytes) }]));
        };
        let parent = GffPath(parent.to_vec());
        Ok(Some(match step {
            Step::Field(l) => vec![Edit::SetField {
                key: *key,
                path: parent,
                label: l.clone(),
                value: Some(Value::Struct(old)),
            }],
            Step::Item(l, i) => vec![
                Edit::RemoveItem { key: *key, path: parent.clone(), list: l.clone(), index: *i },
                Edit::InsertItem { key: *key, path: parent, list: l.clone(), index: *i, item: old },
            ],
        }))
    }

    /// The struct at `path` of document `key`, to change: the document
    /// counts as edited ([`Workspace::flush`] writes it back).
    fn struct_at(&mut self, key: &ResKey, path: &GffPath) -> Result<&mut Struct, EditError> {
        self.edited.insert(*key);
        let doc = self.doc_mut(key)?;
        path.get_mut(&mut doc.root)
            .ok_or_else(|| EditError::BadPath { key: *key, path: path.clone() })
    }

    /// Writes the documents edited since the last flush back into the
    /// module (those only read are as the module has them already): its
    /// cost is what was edited, not what a session has opened.
    pub fn flush(&mut self) -> Result<(), EditError> {
        let edited: Vec<ResKey> = self.edited.iter().copied().collect();
        for key in edited {
            if let Some(gff) = self.docs.get(&key) {
                let bytes = gff
                    .to_bytes()
                    .map_err(|e| EditError::Invalid { key, message: e.to_string() })?;
                if self.module.get(&key) != Some(&bytes[..]) {
                    self.module.set(key, bytes);
                }
            }
            self.edited.remove(&key);
        }
        Ok(())
    }

    /// How many documents hold edits [`Workspace::flush`] has not yet
    /// written into the module.
    pub fn unflushed(&self) -> usize {
        self.edited.len()
    }

    /// Flushes and saves the module to where it was opened from.
    pub fn save(&mut self) -> Result<(), EditError> {
        self.flush()?;
        self.module.save()?;
        self.changes_since_save = 0;
        self.saved = resources_of(&self.module);
        Ok(())
    }

    /// Flushes and saves the module to another place, which becomes its
    /// location.
    pub fn save_as(&mut self, location: &mg_module::ModuleLocation) -> Result<(), EditError> {
        self.flush()?;
        self.module.save_as(location)?;
        self.changes_since_save = 0;
        self.saved = resources_of(&self.module);
        Ok(())
    }

    /// The module as it is now, edits included (for a recovery copy).
    pub fn snapshot(&mut self) -> Result<Module, EditError> {
        self.flush()?;
        Ok(self.module.clone())
    }

    /// The edits that make the module `staged`: a resource set where
    /// `staged` has another or a new one, removed where it has none. For an
    /// operation written to change a module directly: run it on a copy
    /// ([`Workspace::snapshot`]), then apply the difference as a command.
    /// Flush first; documents changed since are not in the comparison.
    pub fn edits_to(&self, staged: &Module) -> Vec<Edit> {
        edits_between(&self.module, staged)
    }

    /// Changes the module outside the edit history, for what is made from
    /// it when it is saved or built (compiled scripts, custom palettes):
    /// Undo does not take it back, and it does not count as unsaved work,
    /// but the revision moves, so that views show it.
    pub fn derive<R>(&mut self, make: impl FnOnce(&mut Module) -> R) -> Result<R, EditError> {
        self.flush()?;
        let made = make(&mut self.module);
        // (They are read again from what `make` left.)
        self.docs.clear();
        self.edited.clear();
        self.revision += 1;
        Ok(made)
    }

    /// Replaces resources (`None`: removes them) outside the edit history:
    /// files of the module changed by another program, read again. It does
    /// not count as unsaved work. Undo could not take an earlier change
    /// to one of them back from what it is now: if the history has any,
    /// the history is dropped, and `true` returned. What a save compares
    /// with ([`Workspace::saved`]) takes them too, but blueprints: their
    /// entries in the custom palettes are made right as for a blueprint
    /// changed here (shown at once, written at the next save).
    pub fn adopt(
        &mut self,
        changes: &[(ResKey, Option<std::sync::Arc<[u8]>>)],
    ) -> Result<bool, EditError> {
        self.flush()?;
        for (key, data) in changes {
            self.docs.remove(key);
            self.edited.remove(key);
            self.rewritten(key);
            // (Not the builder's change here: what a save compares with
            // takes it too. Not a blueprint: its palette entry follows it,
            // as a builder copying one into the module expects.)
            let blueprint = mg_rules::ObjectKind::from_restype(key.restype).is_some();
            let saved = (!blueprint).then_some(&mut self.saved);
            for m in std::iter::once(&mut self.module).chain(saved) {
                match data {
                    Some(d) => m.set(*key, d.clone()),
                    None => {
                        m.remove(key);
                    }
                }
            }
        }
        let touched =
            |c: &Command| c.edits.iter().any(|e| changes.iter().any(|(k, _)| k == e.key()));
        let dropped = self.undo.iter().chain(&self.redo).any(touched);
        if dropped {
            self.undo.clear();
            self.redo.clear();
        }
        self.revision += 1;
        Ok(dropped)
    }

    /// Counts the module as changed since it was saved (recovered work
    /// that is not in its file yet).
    pub fn mark_modified(&mut self) {
        self.changes_since_save = self.changes_since_save.max(1);
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

    /// A document given to look at is read, not edited, and gives way to
    /// the module's own of that name.
    #[test]
    fn a_viewed_document_is_read_and_never_edited() {
        let mut ws = Workspace::new(Module::default());
        let key = ResKey::parse("nw_goblina", mg_core::ResType::UTC).unwrap();
        let mut gff = Gff::new(*b"UTC ");
        gff.root.set("Tag", Value::String(b"NW_GOBLINA".to_vec()));
        ws.view(key, gff.clone());
        assert!(ws.is_viewed(&key));
        assert_eq!(ws.doc(&key).unwrap().root.string("Tag"), Some(&b"NW_GOBLINA"[..]));
        let edit = Edit::SetField {
            key,
            path: GffPath::root(),
            label: "Tag".into(),
            value: Some(Value::String(b"MINE".to_vec())),
        };
        assert!(ws.apply(Command::new("edit", vec![edit])).is_err());
        assert!(!ws.is_modified());
        let own = Edit::SetResource { key, data: Some(gff.to_bytes().unwrap()) };
        ws.apply(Command::new("copy", vec![own])).unwrap();
        assert!(!ws.is_viewed(&key), "the module's own is the one edited");
    }

    /// What a save compares with (the custom palettes': only what
    /// changed is made again): the resources as opened, then as saved,
    /// with the files changed outside taken in, not the edits; a
    /// blueprint changed outside is not taken in (its palette entry
    /// follows it, Troubletcat's report).
    #[test]
    fn the_saved_resources_are_those_opened_saved_or_taken_in() {
        let mut ws = workspace();
        let ifo_key = key("module", ResType::IFO);
        let opened = ws.module.get(&ifo_key).unwrap().to_vec();
        let edit = Edit::SetField {
            key: ifo_key,
            path: GffPath::root(),
            label: "Mod_XPScale".into(),
            value: Some(Value::Byte(50)),
        };
        ws.apply(Command::new("XP scale", vec![edit])).unwrap();
        ws.flush().unwrap();
        assert_ne!(ws.module.get(&ifo_key).unwrap(), &opened[..]);
        assert_eq!(ws.saved().get(&ifo_key).unwrap(), &opened[..], "an edit is not saved");
        // A file changed outside: taken in as saved.
        let outside = key("outside", ResType::NSS);
        let data: std::sync::Arc<[u8]> = b"void main() {}\n".to_vec().into();
        ws.adopt(&[(outside, Some(data.clone()))]).unwrap();
        assert_eq!(ws.saved().get(&outside), Some(&data[..]));
        ws.adopt(&[(outside, None)]).unwrap();
        assert!(!ws.saved().contains(&outside));
        // A blueprint: the module's, not taken in (a change, for its
        // palette entry).
        let item = key("outside", ResType::UTI);
        let data: std::sync::Arc<[u8]> = Gff::new(*b"UTI ").to_bytes().unwrap().into();
        ws.adopt(&[(item, Some(data.clone()))]).unwrap();
        assert_eq!(ws.module.get(&item), Some(&data[..]));
        assert!(!ws.saved().contains(&item));
        ws.adopt(&[(item, None)]).unwrap();
        assert!(!ws.module.contains(&item));
        // Saved: what was saved.
        let dir = std::env::temp_dir().join(format!("mg-edit-saved-{}", std::process::id()));
        ws.save_as(&mg_module::ModuleLocation::Archive(dir.join("m.mod"))).unwrap();
        assert_eq!(ws.saved().get(&ifo_key), ws.module.get(&ifo_key));
        let _ = std::fs::remove_dir_all(dir);
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

    /// Undo puts a removed field back where it was, with any other of its
    /// label (as a resource that is undone back to as it was saves as it
    /// was), whether the struct is a list's item, a field or the root; an
    /// item put in a list it made takes the list away again.
    #[test]
    fn undo_puts_a_removed_field_back_in_its_place() {
        let mut ws = workspace();
        let k = key("area", ResType::GIT);
        let labels = |ws: &mut Workspace, path: &GffPath| -> Vec<String> {
            let doc = ws.doc(&k).unwrap();
            let s = path.get(&doc.root).unwrap();
            s.fields.iter().map(|f| f.label.to_string()).collect()
        };
        let item = GffPath::root().item("Creature List", 0);
        let mut inner = Struct::new(7);
        for (l, v) in [("A", 1), ("B", 2), ("A", 3), ("C", 4)] {
            inner.fields.push(mg_gff::Field {
                label: mg_gff::Label::new(l).unwrap(),
                value: Value::Int(v),
            });
        }
        let setup = vec![
            Edit::SetField {
                key: k,
                path: item.clone(),
                label: "Inner".into(),
                value: Some(Value::Struct(inner)),
            },
            Edit::SetField {
                key: k,
                path: GffPath::root(),
                label: "Note".into(),
                value: Some(Value::Int(1)),
            },
        ];
        ws.apply(Command::new("setup", setup)).unwrap();
        let root_before = labels(&mut ws, &GffPath::root());
        ws.apply(Command::new(
            "note last",
            vec![Edit::SetField {
                key: k,
                path: GffPath::root(),
                label: "Creature List".into(),
                value: None,
            }],
        ))
        .unwrap();
        ws.undo().unwrap();
        assert_eq!(labels(&mut ws, &GffPath::root()), root_before, "the root's");
        for path in [item.clone(), item.field("Inner")] {
            let before = labels(&mut ws, &path);
            let removed = Edit::SetField {
                key: k,
                path: path.clone(),
                label: before[0].clone(),
                value: None,
            };
            ws.apply(Command::new("remove", vec![removed])).unwrap();
            assert!(!labels(&mut ws, &path).contains(&before[0]));
            ws.undo().unwrap();
            assert_eq!(labels(&mut ws, &path), before, "{path}");
            ws.redo().unwrap();
            assert!(!labels(&mut ws, &path).contains(&before[0]), "redone: {path}");
            ws.undo().unwrap();
        }
        let inner = item.field("Inner");
        let doc = ws.doc(&k).unwrap();
        let values: Vec<_> =
            inner.get(&doc.root).unwrap().fields.iter().map(|f| f.value.clone()).collect();
        assert_eq!(values, [1, 2, 3, 4].map(Value::Int), "both A's, in their places");
        // An item put in where there was no list: undone, no list.
        let added = Edit::insert_item(k, item.clone(), "ItemList", 0, Struct::new(0));
        let before = labels(&mut ws, &item);
        ws.apply(Command::new("add", vec![added])).unwrap();
        ws.undo().unwrap();
        assert_eq!(labels(&mut ws, &item), before);
    }

    /// A list at a resource's root rewritten (set whole, an item put in or
    /// taken out), the resource replaced or read again, is counted, by
    /// command, undo and redo; an edit inside an item or of another field
    /// is not (an area's tiles painted keep their places, Rotate Area
    /// moves them: GitHub issue 31).
    #[test]
    fn rewrites_of_a_resource_s_lists_are_counted() {
        let mut ws = workspace();
        let k = key("area", ResType::GIT);
        let run = |ws: &mut Workspace, e: Edit| {
            ws.apply(Command::new("edit", vec![e])).unwrap();
            ws.rewrites(&k)
        };
        assert_eq!(ws.rewrites(&k), 0);
        let set = |path: GffPath, label: &str, value: Value| Edit::SetField {
            key: k,
            path,
            label: label.into(),
            value: Some(value),
        };
        let inside = GffPath::root().item("Creature List", 0);
        assert_eq!(run(&mut ws, set(inside, "Tag", Value::String(b"HEN".to_vec()))), 0);
        assert_eq!(run(&mut ws, set(GffPath::root(), "Note", Value::Int(1))), 0);
        let added = Edit::insert_item(k, GffPath::root(), "Creature List", 1, Struct::new(4));
        assert_eq!(run(&mut ws, added), 1);
        ws.undo().unwrap();
        assert_eq!(ws.rewrites(&k), 2, "undone");
        ws.redo().unwrap();
        assert_eq!(ws.rewrites(&k), 3, "redone");
        assert_eq!(run(&mut ws, set(GffPath::root(), "Creature List", Value::List(vec![]))), 4);
        assert_eq!(run(&mut ws, Edit::SetResource { key: k, data: None }), 5);
        ws.adopt(&[(k, None)]).unwrap();
        assert_eq!(ws.rewrites(&k), 6, "read again");
        assert_eq!(ws.rewrites(&key("module", ResType::IFO)), 0, "another resource");
    }

    /// What Undo and Redo will run is seen before they run it: the
    /// inverse, then the command again.
    #[test]
    fn the_next_undo_and_redo_are_seen_before_they_run() {
        let mut ws = workspace();
        let k = key("area", ResType::GIT);
        assert!(ws.peek_undo().is_none() && ws.peek_redo().is_none());
        let added = Edit::insert_item(k, GffPath::root(), "Creature List", 0, Struct::new(4));
        ws.apply(Command::new("Add", vec![added.clone()])).unwrap();
        let removal = Edit::remove_item(k, GffPath::root(), "Creature List", 0);
        assert_eq!(ws.peek_undo().map(|c| &c.edits[..]), Some(&[removal][..]));
        ws.undo().unwrap();
        assert!(ws.peek_undo().is_none());
        assert_eq!(ws.peek_redo().map(|c| &c.edits[..]), Some(&[added][..]));
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
    fn a_changed_copy_becomes_one_command() {
        let mut ws = workspace();
        ws.flush().unwrap();
        let before = ws.module.clone();
        // An operation written to change a module: on a copy.
        let mut staged = ws.snapshot().unwrap();
        staged.set(key("new", ResType::NSS), b"void main() { }".to_vec());
        staged.remove(&key("area", ResType::GIT));
        let mut info = staged.info().unwrap();
        info.root.write(&ifo::MOD_XP_SCALE, 75);
        staged.set_info(&info).unwrap();
        let edits = ws.edits_to(&staged);
        assert_eq!(edits.len(), 3, "{edits:?}");
        ws.apply(Command::new("Operation", edits)).unwrap();
        ws.flush().unwrap();
        let keys = |m: &Module| m.keys().copied().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys(&ws.module), keys(&staged));
        assert!(keys(&staged).iter().all(|k| ws.module.get(k) == staged.get(k)));
        // Nothing to do: nothing.
        assert!(ws.edits_to(&staged).is_empty());
        ws.undo().unwrap();
        ws.flush().unwrap();
        assert_eq!(keys(&ws.module), keys(&before));
        assert!(keys(&before).iter().all(|k| ws.module.get(k) == before.get(k)));
    }

    #[test]
    fn what_is_made_from_the_module_is_outside_its_history() {
        let mut ws = workspace();
        let k = key("module", ResType::IFO);
        ws.apply(Command::new(
            "XP scale",
            vec![Edit::SetField {
                key: k,
                path: GffPath::root(),
                label: "Mod_XPScale".into(),
                value: Some(Value::Byte(50)),
            }],
        ))
        .unwrap();
        let (revision, compiled) = (ws.revision(), key("made", ResType::NCS));
        // It sees the edits made so far, and may replace what is cached.
        let scale = ws
            .derive(|m| {
                m.set(compiled, vec![1, 2, 3]);
                m.info().unwrap().root.read(&ifo::MOD_XP_SCALE)
            })
            .unwrap();
        assert_eq!(scale, 50);
        assert!(ws.revision() > revision, "views are told");
        assert_eq!(ws.can_undo(), Some("XP scale"), "no step of its own");
        ws.undo().unwrap();
        assert_eq!(ws.doc(&k).unwrap().root.read(&ifo::MOD_XP_SCALE), 10);
        assert_eq!(ws.module.get(&compiled), Some(&[1u8, 2, 3][..]), "and Undo leaves it");
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
    fn a_withdrawn_command_leaves_nothing_to_redo() {
        let mut ws = workspace();
        let k = key("module", ResType::IFO);
        let set = |label: &str, v: i32| Edit::SetField {
            key: k,
            path: GffPath::root(),
            label: label.into(),
            value: Some(Value::Int(v)),
        };
        assert_eq!(ws.withdraw().unwrap(), None, "nothing to withdraw");
        ws.apply(Command::new("Set A", vec![set("A", 1)])).unwrap();
        ws.apply(Command::new("Draft", vec![set("A", 2), set("B", 3)])).unwrap();
        ws.amend(vec![set("B", 4)]).unwrap();
        let revision = ws.revision();
        assert_eq!(ws.withdraw().unwrap().as_deref(), Some("Draft"));
        assert!(ws.revision() > revision);
        let r = &ws.doc(&k).unwrap().root;
        assert_eq!((r.integer("A"), r.integer("B")), (Some(1), None), "as before the draft");
        assert_eq!(ws.can_undo(), Some("Set A"));
        assert_eq!(ws.can_redo(), None, "nothing to redo");
        assert!(ws.is_modified(), "Set A is still a change");
        ws.undo().unwrap();
        assert!(!ws.is_modified());
    }

    /// An undo (or redo) whose edits fail changes nothing and keeps the
    /// command where it was, as a failed withdraw does: it is not lost.
    #[test]
    fn a_failed_undo_or_redo_keeps_its_command() {
        let mut ws = workspace();
        let k = key("area", ResType::GIT);
        let bad = || {
            Command::new(
                "Bad",
                vec![Edit::RemoveItem {
                    key: k,
                    path: GffPath::root(),
                    list: "Creature List".into(),
                    index: 7,
                }],
            )
        };
        let before = ws.doc(&k).unwrap().clone();
        ws.undo.push(bad());
        assert!(ws.undo().is_err());
        assert_eq!(ws.can_undo(), Some("Bad"));
        assert_eq!(ws.can_redo(), None);
        ws.undo.clear();
        ws.redo.push(bad());
        assert!(ws.redo().is_err());
        assert_eq!(ws.can_redo(), Some("Bad"));
        assert_eq!(ws.can_undo(), None);
        assert_eq!(ws.doc(&k).unwrap(), &before);
        assert!(!ws.is_modified());
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

    /// A flush writes what was edited since the last one, however it was
    /// edited (a field set, an item put in or taken out, a field taken out
    /// from among others, an amendment, an undo, a redo), and only that: a
    /// document only read is left as the module has it.
    #[test]
    fn a_flush_writes_every_edit_and_only_what_was_edited() {
        type Step = (&'static str, Box<dyn Fn(&mut Workspace)>, Box<dyn Fn(&Gff) -> bool>);
        let mut ws = workspace();
        let (area, info) = (key("area", ResType::GIT), key("module", ResType::IFO));
        let stored = |ws: &Workspace, k: &ResKey| Gff::read(ws.module.get(k).unwrap()).unwrap();
        let field = move |label: &str, value: Option<i32>| Edit::SetField {
            key: area,
            path: GffPath::root(),
            label: label.into(),
            value: value.map(Value::Int),
        };
        let creatures = |g: &Gff| g.root.list("Creature List").map_or(0, <[Struct]>::len);
        // Read, not edited: nothing to write, and what the module holds
        // for it stays (bytes set there since are not written over).
        ws.doc(&info).unwrap();
        ws.doc(&area).unwrap();
        assert_eq!(ws.unflushed(), 0);
        let other = {
            let mut g = Gff::new(*b"IFO ");
            g.root.set("Other", Value::Int(1));
            g.to_bytes().unwrap()
        };
        ws.module.set(info, other.clone());
        ws.flush().unwrap();
        assert_eq!(ws.module.get(&info), Some(&other[..]), "a document only read isn't written");
        // Each way of editing is written by the next flush.
        let steps: Vec<Step> = vec![
            (
                "a field set",
                Box::new(move |ws| ws.apply(Command::new("A", vec![field("A", Some(1))])).unwrap()),
                Box::new(|g| g.root.int("A") == Some(1)),
            ),
            (
                "an amendment",
                Box::new(move |ws| ws.amend(vec![field("B", Some(2))]).unwrap()),
                Box::new(|g| g.root.int("B") == Some(2)),
            ),
            (
                "an item taken out",
                Box::new(move |ws| {
                    let edit = Edit::remove_item(area, GffPath::root(), "Creature List", 0);
                    ws.apply(Command::new("Out", vec![edit])).unwrap()
                }),
                Box::new(move |g| creatures(g) == 0),
            ),
            (
                "an undo",
                Box::new(|ws| _ = ws.undo().unwrap()),
                Box::new(move |g| creatures(g) == 1),
            ),
            ("a redo", Box::new(|ws| _ = ws.redo().unwrap()), Box::new(move |g| creatures(g) == 0)),
            (
                "an item put in",
                Box::new(move |ws| {
                    let item = Struct::default();
                    let edit = Edit::insert_item(area, GffPath::root(), "Creature List", 0, item);
                    ws.apply(Command::new("In", vec![edit])).unwrap()
                }),
                Box::new(move |g| creatures(g) == 1),
            ),
            (
                "a field taken out from among others",
                Box::new(move |ws| ws.apply(Command::new("A", vec![field("A", None)])).unwrap()),
                Box::new(|g| !g.root.contains("A") && g.root.int("B") == Some(2)),
            ),
        ];
        for (what, edit, written) in &steps {
            edit(&mut ws);
            assert_eq!(ws.unflushed(), 1, "{what}: the one document edited");
            assert!(!written(&stored(&ws, &area)), "{what}: not in the module before the flush");
            ws.flush().unwrap();
            assert!(written(&stored(&ws, &area)), "{what}: written by the flush");
            assert_eq!(ws.unflushed(), 0);
        }
        assert_eq!(ws.module.get(&info), Some(&other[..]), "the document only read, still not");
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
