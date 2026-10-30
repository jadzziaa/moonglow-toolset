//! Changes to what Advanced Search found (`mg_module::search`): a local
//! variable set or removed, an event's script set, or a field set, on
//! each object found, as the edits of one command.
//!
//! An area's variables are its GIT's (the game reads none from its ARE,
//! `engine_area_variables`); its events and fields are its ARE's. A
//! field or script found is set where it was found (a conversation line's
//! script, a field in a list); otherwise the object's own field of that
//! label is, where it has one.

use std::collections::{HashMap, HashSet};

use mg_core::{Codepage, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_module::search::{Hit, Object, VarType};
use mg_resman::ResKey;

use crate::{Edit, GffPath, Step};

/// What to change on each object found.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A local variable: added, or its type and value replaced.
    SetVariable { name: String, kind: VarType, value: String },
    /// The local variables of this name removed.
    RemoveVariable { name: String },
    /// An event's script (`ScriptSpawn`, `OnEnter`... the field's label);
    /// an empty name clears it.
    SetScript { event: String, script: String },
    /// A field (by label) set to a value typed as its type reads it.
    SetField { label: String, value: String },
}

impl Change {
    /// For the command's name: `Set variable nQuest`.
    pub fn describe(&self) -> String {
        match self {
            Change::SetVariable { name, .. } => format!("Set variable {name}"),
            Change::RemoveVariable { name } => format!("Remove variable {name}"),
            Change::SetScript { event, .. } => {
                format!("Set {} script", mg_module::search::event(event))
            }
            Change::SetField { label, .. } => format!("Set {label}"),
        }
    }
}

/// The edits a change makes, and what it couldn't change.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outcome {
    pub edits: Vec<Edit>,
    /// The objects (or fields) changed.
    pub changed: usize,
    /// What was left as it was, and why: `keep › creature GUARD: no
    /// OnSpawn`.
    pub skipped: Vec<String>,
}

/// Resources read once.
struct Reader<'a> {
    module: &'a Module,
    read: HashMap<ResKey, Option<Gff>>,
}

impl Reader<'_> {
    fn get(&mut self, key: ResKey, path: &GffPath) -> Option<&Struct> {
        let m = self.module;
        let gff = self.read.entry(key).or_insert_with(|| m.gff(&key).and_then(Result::ok));
        path.get(&gff.as_ref()?.root)
    }
}

/// Where an object's own fields are: an area's in its ARE, its variables
/// in its GIT.
fn variables_of(o: &Object) -> (ResKey, GffPath) {
    match o.kind {
        ResType::ARE if o.key.restype == ResType::ARE => {
            (ResKey::new(o.key.resref, ResType::GIT), GffPath::root())
        }
        _ => (o.key, o.path.clone()),
    }
}

/// The objects the hits are on, each once, in their order.
fn objects(hits: &[&Hit]) -> Vec<(Object, String)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for h in hits {
        if let Some(o) = h.object()
            && seen.insert(o.clone())
        {
            // (Named as found: the place of what it is on.)
            let place = match (h.carried, h.spawned) {
                (Some(r), _) => format!("{}.uti", r.to_lowercase()),
                (None, Some(r)) => format!("{}.utc", r.to_lowercase()),
                (None, None) => h.place.clone(),
            };
            out.push((o, place));
        }
    }
    out
}

/// The field a hit was found at, if it was a field (its struct's path and
/// its label).
fn field_found(h: &Hit) -> Option<(GffPath, String)> {
    let mut path = h.path.clone();
    match path.0.pop()? {
        Step::Field(label) if h.carried.is_none() && h.spawned.is_none() => Some((path, label)),
        _ => None,
    }
}

/// The edits that make `change` on what `hits` found (in `module`, its
/// text in `codepage`).
pub fn edits(
    module: &Module,
    hits: &[&Hit],
    change: &Change,
    codepage: Codepage,
) -> Result<Outcome, String> {
    let mut r = Reader { module, read: HashMap::new() };
    let mut out = Outcome::default();
    match change {
        Change::SetVariable { name, kind, value } => {
            let value = kind.value(value, codepage)?;
            let name_bytes = codepage
                .encode(name)
                .ok_or_else(|| format!("{name:?} has characters the module can't hold"))?
                .into_owned();
            if name.trim().is_empty() {
                return Err("Give the variable a name".into());
            }
            for (o, place) in objects(hits) {
                let (key, path) = variables_of(&o);
                let Some(s) = r.get(key, &path) else {
                    out.skipped.push(format!("{place}: no {key} to hold variables"));
                    continue;
                };
                let vars = s.list("VarTable").unwrap_or(&[]);
                let at = vars.iter().position(|v| v.string("Name") == Some(&name_bytes[..]));
                match at {
                    Some(i) => {
                        let var = path.item("VarTable", i);
                        let set = |label: &str, v: Value| Edit::SetField {
                            key,
                            path: var.clone(),
                            label: label.into(),
                            value: Some(v),
                        };
                        out.edits.push(set("Type", Value::Dword(kind.code())));
                        out.edits.push(set("Value", value.clone()));
                    }
                    None => {
                        let mut var = Struct::new(0);
                        var.set("Name", Value::String(name_bytes.clone()));
                        var.set("Type", Value::Dword(kind.code()));
                        var.set("Value", value.clone());
                        out.edits.push(Edit::insert_item(key, path, "VarTable", vars.len(), var));
                    }
                }
                out.changed += 1;
            }
        }
        Change::RemoveVariable { name } => {
            let name_bytes = codepage.encode(name).map(|b| b.into_owned()).unwrap_or_default();
            for (o, place) in objects(hits) {
                let (key, path) = variables_of(&o);
                let vars = r.get(key, &path).and_then(|s| s.list("VarTable")).unwrap_or(&[]);
                let at: Vec<usize> = (0..vars.len())
                    .filter(|&i| vars[i].string("Name") == Some(&name_bytes[..]))
                    .collect();
                if at.is_empty() {
                    out.skipped.push(format!("{place}: no variable {name}"));
                    continue;
                }
                for i in at.into_iter().rev() {
                    out.edits.push(Edit::remove_item(key, path.clone(), "VarTable", i));
                }
                out.changed += 1;
            }
        }
        Change::SetScript { event, script } => {
            let script = ResRef::from_str(script.trim())
                .map_err(|e| format!("{script:?} is not a script's name: {e}"))?;
            let mut set =
                |old: &Value| matches!(old, Value::ResRef(_)).then(|| Value::resref(script));
            set_fields(&mut r, hits, event, &mut set, &mut out);
        }
        Change::SetField { label, value } => {
            // (Checked on the first field, so a typing error says so.)
            let mut problem = None;
            let mut set = |old: &Value| match Value::from_text(
                old.field_type(),
                value,
                Some(old),
                codepage,
            ) {
                Ok(v) => Some(v),
                Err(e) => {
                    problem.get_or_insert(e);
                    None
                }
            };
            set_fields(&mut r, hits, label, &mut set, &mut out);
            if out.changed == 0
                && let Some(e) = problem
            {
                return Err(e);
            }
        }
    }
    Ok(out)
}

/// Sets field `label` where each hit found it, else on each object's own
/// struct, to what `value` makes of the old value (`None`: it can't hold
/// it).
fn set_fields(
    r: &mut Reader<'_>,
    hits: &[&Hit],
    label: &str,
    value: &mut dyn FnMut(&Value) -> Option<Value>,
    out: &mut Outcome,
) {
    let mut done: HashSet<(ResKey, GffPath)> = HashSet::new();
    let mut objects_done: HashSet<Object> = HashSet::new();
    for h in hits {
        let (key, path) = match field_found(h) {
            Some((path, l)) if l.eq_ignore_ascii_case(label) => (h.key, path),
            _ => {
                let Some(o) = h.object() else { continue };
                if !objects_done.insert(o.clone()) {
                    continue;
                }
                (o.key, o.path.clone())
            }
        };
        if !done.insert((key, path.clone())) {
            continue;
        }
        let place = h.place.clone();
        let Some(s) = r.get(key, &path) else { continue };
        // (The label as the struct spells it.)
        let Some(f) =
            s.fields.iter().find(|f| f.label.to_string_lossy().eq_ignore_ascii_case(label))
        else {
            out.skipped.push(format!("{place}: no {label}"));
            continue;
        };
        let real = f.label.to_string_lossy().to_string();
        match value(&f.value) {
            Some(v) => {
                out.edits.push(Edit::SetField { key, path, label: real, value: Some(v) });
                out.changed += 1;
            }
            None => out.skipped.push(format!("{place}: {real} can't hold it")),
        }
    }
}

/// The events (script fields' labels) the objects found have, and those
/// found, sorted.
pub fn events(module: &Module, hits: &[&Hit]) -> Vec<String> {
    let mut r = Reader { module, read: HashMap::new() };
    let mut out: Vec<String> = Vec::new();
    for h in hits {
        if let Some((_, l)) = field_found(h)
            && mg_module::refs::is_script_field(&l)
        {
            out.push(l);
        }
    }
    for (o, _) in objects(hits) {
        let Some(s) = r.get(o.key, &o.path) else { continue };
        for f in &s.fields {
            let l = f.label.to_string_lossy();
            if matches!(f.value, Value::ResRef(_)) && mg_module::refs::is_script_field(&l) {
                out.push(l.to_string());
            }
        }
    }
    out.sort_by_key(|l| mg_module::search::event(l).to_lowercase());
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use mg_module::search::{self, Query, Search};

    use super::*;

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    fn var(name: &str, v: i32) -> Struct {
        let mut s = Struct::new(0);
        s.set("Name", Value::String(name.as_bytes().to_vec()));
        s.set("Type", Value::Dword(1));
        s.set("Value", Value::Int(v));
        s
    }

    fn creature(tag: &str, vars: Vec<Struct>) -> Struct {
        let mut c = Struct::new(4);
        c.set("Tag", Value::String(tag.as_bytes().to_vec()));
        c.set("Appearance_Type", Value::Word(6));
        c.set("ScriptSpawn", Value::resref(rr("x2_def_spawn")));
        let mut class = Struct::new(2);
        class.set("Class", Value::Int(4));
        class.set("ClassLevel", Value::Short(5));
        c.set("ClassList", Value::List(vec![class]));
        c.set("VarTable", Value::List(vars));
        c
    }

    /// A guard blueprint, an area with a placed guard who holds an item,
    /// and the area's own variable in its ARE (as Moonglow wrote them
    /// before 1.34.3).
    fn module() -> Module {
        let mut m = Module::new();
        let mut utc = Gff::new(*b"UTC ");
        utc.root = creature("GUARD", vec![var("nOld", 1)]);
        m.set_gff(ResKey::parse("guard", ResType::UTC).unwrap(), &utc).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut placed = creature("GUARD_01", Vec::new());
        let mut held = Struct::new(0);
        held.set("Tag", Value::String(b"SWORD".to_vec()));
        held.set("VarTable", Value::List(vec![var("nOld", 2)]));
        placed.set("ItemList", Value::List(vec![held]));
        git.root.set("Creature List", Value::List(vec![placed]));
        m.set_gff(ResKey::parse("keep", ResType::GIT).unwrap(), &git).unwrap();
        let mut are = Gff::new(*b"ARE ");
        are.root.set("OnEnter", Value::resref(rr("")));
        are.root.set("Tag", Value::String(b"KEEP".to_vec()));
        m.set_gff(ResKey::parse("keep", ResType::ARE).unwrap(), &are).unwrap();
        m
    }

    fn find(m: &Module, s: Search) -> Vec<Hit> {
        search::search(m, &Query::new(s), None, Codepage::WINDOWS_1252)
    }

    fn apply(m: &Module, hits: &[Hit], change: Change) -> (Module, Outcome) {
        let hits: Vec<&Hit> = hits.iter().collect();
        let out = edits(m, &hits, &change, Codepage::WINDOWS_1252).unwrap();
        let mut ws = crate::Workspace::new(m.clone());
        ws.apply(crate::Command::new(change.describe(), out.edits.clone())).unwrap();
        ws.flush().unwrap();
        (ws.module.clone(), out)
    }

    fn gff(m: &Module, name: &str, t: ResType) -> Gff {
        m.gff(&ResKey::new(rr(name), t)).unwrap().unwrap()
    }

    #[test]
    fn variables_set_and_removed_on_each_object_found() {
        let m = module();
        // The fighters: the blueprint and the placed guard.
        let fighters = find(&m, Search::Class(4));
        assert_eq!(fighters.len(), 2);
        let set =
            Change::SetVariable { name: "nOld".into(), kind: VarType::String, value: "yes".into() };
        let (after, out) = apply(&m, &fighters, set);
        assert_eq!(out.changed, 2);
        // The blueprint's replaced (type and value), the placed one's added.
        let utc = gff(&after, "guard", ResType::UTC);
        let v = &utc.root.list("VarTable").unwrap()[0];
        assert_eq!((v.integer("Type"), v.string("Value")), (Some(3), Some(&b"yes"[..])));
        let git = gff(&after, "keep", ResType::GIT);
        let placed = &git.root.list("Creature List").unwrap()[0];
        assert_eq!(placed.list("VarTable").unwrap().len(), 1);
        // The item he holds kept its own.
        let held = &placed.list("ItemList").unwrap()[0];
        assert_eq!(held.list("VarTable").unwrap()[0].integer("Value"), Some(2));

        // Removed where they are, said where they aren't.
        let remove = Change::RemoveVariable { name: "nOld".into() };
        let (after, out) = apply(&after, &find(&after, Search::Class(4)), remove);
        assert_eq!(out.changed, 2);
        assert!(gff(&after, "guard", ResType::UTC).root.list("VarTable").unwrap().is_empty());
        let (_, out) = apply(
            &after,
            &find(&after, Search::Class(4)),
            Change::RemoveVariable { name: "nOld".into() },
        );
        assert_eq!(
            out.skipped,
            ["guard.utc: no variable nOld", "keep › creature GUARD_01: no variable nOld"]
        );

        // An area's variable goes to its GIT.
        let area = find(&m, Search::Field { label: Some("Tag".into()), value: None });
        let area: Vec<Hit> = area.into_iter().filter(|h| h.key.restype == ResType::ARE).collect();
        let set =
            Change::SetVariable { name: "nArea".into(), kind: VarType::Int, value: "4".into() };
        let (after, _) = apply(&m, &area, set);
        let git = gff(&after, "keep", ResType::GIT);
        assert_eq!(git.root.list("VarTable").unwrap()[0].integer("Value"), Some(4));
        assert!(gff(&after, "keep", ResType::ARE).root.list("VarTable").is_none());
        // A value its type can't hold.
        let bad = Change::SetVariable { name: "n".into(), kind: VarType::Int, value: "x".into() };
        let hits: Vec<&Hit> = fighters.iter().collect();
        assert!(edits(&m, &hits, &bad, Codepage::WINDOWS_1252).is_err());
    }

    #[test]
    fn scripts_and_fields_set_where_found_or_on_the_object() {
        let m = module();
        let fighters = find(&m, Search::Class(4));
        let set = Change::SetScript { event: "ScriptSpawn".into(), script: "my_spawn".into() };
        let (after, out) = apply(&m, &fighters, set);
        assert_eq!(out.changed, 2);
        let utc = gff(&after, "guard", ResType::UTC);
        assert_eq!(utc.root.resref("ScriptSpawn"), Some(rr("my_spawn")));
        // An event the object doesn't have.
        let set = Change::SetScript { event: "OnEnter".into(), script: "x".into() };
        let hits: Vec<&Hit> = fighters.iter().collect();
        let out = edits(&m, &hits, &set, Codepage::WINDOWS_1252).unwrap();
        assert_eq!(out.changed, 0);
        assert_eq!(out.skipped.len(), 2);
        // The events offered: the objects'.
        let all = find(&m, Search::Field { label: Some("Tag".into()), value: None });
        let all: Vec<&Hit> = all.iter().collect();
        assert_eq!(events(&m, &all), ["OnEnter", "ScriptSpawn"]);

        // A field found is set where found: the class's level; the
        // object's own by label otherwise, kept of its type.
        let levels = find(&m, Search::Field { label: Some("ClassLevel".into()), value: None });
        let set = Change::SetField { label: "ClassLevel".into(), value: "9".into() };
        let (after, out) = apply(&m, &levels, set);
        assert_eq!(out.changed, 2);
        let utc = gff(&after, "guard", ResType::UTC);
        assert_eq!(
            utc.root.list("ClassList").unwrap()[0].get("ClassLevel"),
            Some(&Value::Short(9))
        );
        let set = Change::SetField { label: "appearance_type".into(), value: "7".into() };
        let (after, _) = apply(&m, &fighters, set);
        let utc = gff(&after, "guard", ResType::UTC);
        assert_eq!(utc.root.get("Appearance_Type"), Some(&Value::Word(7)));
        // A value the field can't hold.
        let bad = Change::SetField { label: "Appearance_Type".into(), value: "-1".into() };
        assert!(edits(&m, &hits, &bad, Codepage::WINDOWS_1252).is_err());
    }
}
