//! Changes to what Advanced Search found (`mg_module::search`): a local
//! variable set or removed, an event's script set, a field set, or an
//! entry of a list added or removed (a feat, a class, an item property,
//! an item held: through `crate::lists`, as Edit Together's pages make
//! them), on each object found, as the edits of one command.
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
use mg_rules::{GameData, ItemProperty};

use crate::{Edit, GffPath, Step, lists};

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
    /// An entry added to each object found that lacks it (`name`: what
    /// the command is named by, "Alertness").
    AddEntry { entry: Entry, name: String },
    /// An entry taken from each object found that has it.
    RemoveEntry { entry: Entry, name: String },
}

/// An entry of an object's list, as Advanced Search adds or removes it.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// A creature's feat (feat.2da row).
    Feat(u16),
    /// A creature's class (classes.2da row), added at `level`.
    Class { class: i32, level: i16 },
    /// An item's property: `property` as added; removed where an item's
    /// is of its type and of the subtype, value and parameter given (a
    /// part not given: any).
    Property { property: ItemProperty, subtype: Option<u16>, value: Option<u16>, param: Option<u8> },
    /// An item of this blueprint held: added (one, at its first free
    /// place; a store's on the page of its kind) or taken (every one, from
    /// a creature's backpack, a placeable's inventory, a store's pages).
    Item(ResRef),
}

impl Entry {
    /// The kinds of object that have the list.
    fn kinds(&self) -> &'static [ResType] {
        match self {
            Entry::Feat(_) | Entry::Class { .. } => &[ResType::UTC],
            Entry::Property { .. } => &[ResType::UTI],
            Entry::Item(_) => &[ResType::UTC, ResType::UTP, ResType::UTM],
        }
    }

    /// What an object of another kind lacks.
    fn lacking(&self) -> &'static str {
        match self {
            Entry::Feat(_) => "no feats",
            Entry::Class { .. } => "no classes",
            Entry::Property { .. } => "no item properties",
            Entry::Item(_) => "no inventory",
        }
    }
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
            Change::AddEntry { entry, name } => format!("Add {} {name}", entry_kind(entry)),
            Change::RemoveEntry { entry, name } => format!("Remove {} {name}", entry_kind(entry)),
        }
    }
}

/// What an entry is called in a command's name.
fn entry_kind(e: &Entry) -> &'static str {
    match e {
        Entry::Feat(_) => "feat",
        Entry::Class { .. } => "class",
        Entry::Property { .. } => "property",
        Entry::Item(_) => "item",
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
/// text in `codepage`; the game's data, `game`, for an item's place and
/// what a base item takes).
pub fn edits(
    module: &Module,
    hits: &[&Hit],
    change: &Change,
    codepage: Codepage,
    game: Option<&GameData>,
) -> Result<Outcome, String> {
    let mut r = Reader { module, read: HashMap::new() };
    let mut out = Outcome::default();
    // (A hak's or the game's blueprint found is read-only: none of its
    // copy is the module's to change.)
    let hits: Vec<&Hit> = hits.iter().copied().filter(|h| h.layer.is_none()).collect();
    let hits = &hits[..];
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
            // (A name the module can't hold names no variable: not those
            // with no name.)
            let name_bytes = codepage
                .encode(name)
                .ok_or_else(|| format!("{name:?} has characters the module can't hold"))?
                .into_owned();
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
        Change::AddEntry { entry, .. } | Change::RemoveEntry { entry, .. } => {
            let add = matches!(change, Change::AddEntry { .. });
            // The objects found that have the list.
            let mut targets = Vec::new();
            for (o, place) in objects(hits) {
                if !entry.kinds().contains(&o.kind) {
                    out.skipped.push(format!("{place}: {}", entry.lacking()));
                    continue;
                }
                let Some(s) = r.get(o.key, &o.path) else { continue };
                let mut t = lists::Target::new(o.key, o.path.clone(), s.clone());
                t.name = place;
                targets.push(t);
            }
            let done = list_entry(module, game, &targets, entry, add)?;
            out.edits = done.edits;
            out.changed = done.changed;
            out.skipped.extend(done.left);
        }
    }
    Ok(out)
}

/// The edits that add `entry` to each of `targets` that lacks it, or (not
/// `add`) take it from each that has it.
fn list_entry(
    module: &Module,
    game: Option<&GameData>,
    targets: &[lists::Target],
    entry: &Entry,
    add: bool,
) -> Result<lists::Outcome, String> {
    Ok(match (entry, add) {
        (Entry::Feat(row), true) => lists::add_feat(targets, *row),
        (Entry::Feat(row), false) => lists::remove_feat(targets, *row),
        (Entry::Class { class, level }, true) => lists::add_class(targets, *class, *level),
        (Entry::Class { class, .. }, false) => lists::remove_class(targets, *class),
        (Entry::Property { property, .. }, true) => {
            let takes = |t: &lists::Target| {
                let base = t.at.integer("BaseItem").unwrap_or(0).max(0) as u32;
                game.is_none_or(|g| {
                    g.available_properties(base).iter().any(|a| a.row == property.property)
                })
            };
            lists::add_property(targets, property, takes)
        }
        (Entry::Property { property, subtype, value, param }, false) => {
            let is = |e: &Struct| {
                let p = ItemProperty::from_gff(e);
                p.property == property.property
                    && subtype.is_none_or(|s| s == p.subtype)
                    && value.is_none_or(|v| v == p.cost_value)
                    && param.is_none_or(|v| v == p.param1_value)
            };
            lists::remove(targets, "PropertiesList", is, |_| None)
        }
        (Entry::Item(r), true) => {
            let game = game.ok_or("Adding an item needs the game's data (its size)")?;
            add_item(module, game, targets, *r)
        }
        (Entry::Item(r), false) => remove_item(targets, *r),
    })
}

/// The places an object keeps its items in: its `ItemList` (a
/// creature's backpack), or each of a store's pages'.
fn item_bags(t: &lists::Target) -> Vec<(GffPath, &[Struct])> {
    if t.at.get("StoreList").is_some() {
        let pages = t.list("StoreList").iter().enumerate();
        pages
            .map(|(i, p)| (t.path.clone().item("StoreList", i), p.list("ItemList").unwrap_or(&[])))
            .collect()
    } else {
        vec![(t.path.clone(), t.list("ItemList"))]
    }
}

/// Every item of blueprint `r` taken from each target.
fn remove_item(targets: &[lists::Target], r: ResRef) -> lists::Outcome {
    let mut out = lists::Outcome::default();
    for t in targets {
        let mut any = false;
        for (path, items) in item_bags(t) {
            let at = items.iter().enumerate().rev().filter(|(_, e)| lists::item_of(e) == Some(r));
            for (i, _) in at {
                out.edits.push(Edit::remove_item(t.key, path.clone(), "ItemList", i));
                any = true;
            }
        }
        out.changed += usize::from(any);
    }
    out
}

/// One item of blueprint `r` added to each target, at the first free
/// place of its inventory (a store's on the page of its kind, made where
/// it has none): a placed object holds the whole item, a blueprint names
/// its blueprint.
fn add_item(
    module: &Module,
    game: &GameData,
    targets: &[lists::Target],
    r: ResRef,
) -> lists::Outcome {
    let mut out = lists::Outcome::default();
    let read = |r: ResRef| mg_module::gff_root(Some(module), game, &ResKey::new(r, ResType::UTI));
    let Some(bp) = read(r) else {
        for t in targets {
            out.left.push(format!("{}: no item blueprint {r}", t.name));
        }
        return out;
    };
    let base_of = |e: &Struct| -> i64 {
        e.integer("BaseItem")
            .or_else(|| read(lists::item_of(e)?)?.integer("BaseItem"))
            .unwrap_or(-1)
    };
    let cell = |base: i64, col: &str| -> Option<u32> {
        let t = game.table("baseitems").ok()?;
        t.get_int(usize::try_from(base).ok()?, col).and_then(|v| u32::try_from(v).ok())
    };
    // (The room an item takes, by the one reading of it.)
    let size = |base: i64| {
        let (w, h) = usize::try_from(base).map_or((1, 1), |b| game.inventory_size(b));
        (u32::from(w), u32::from(h))
    };
    let base = bp.integer("BaseItem").unwrap_or(-1);
    let (w, h) = size(base);
    for t in targets {
        let store = t.at.get("StoreList").is_some();
        let pages = t.list("StoreList");
        let (path, items) = if store {
            let panel = cell(base, "StorePanel").unwrap_or(4);
            let id = lists::STORE_PANEL_PAGE.get(panel as usize).copied().unwrap_or(1);
            match pages.iter().position(|p| p.id == id) {
                Some(i) => (
                    t.path.clone().item("StoreList", i),
                    pages[i].list("ItemList").unwrap_or(&[]).to_vec(),
                ),
                None => {
                    let at = pages.len();
                    out.edits.push(Edit::insert_item(
                        t.key,
                        t.path.clone(),
                        "StoreList",
                        at,
                        Struct::new(id),
                    ));
                    (t.path.clone().item("StoreList", at), Vec::new())
                }
            }
        } else {
            (t.path.clone(), t.list("ItemList").to_vec())
        };
        let taken: Vec<(u32, u32, u32, u32)> = items
            .iter()
            .map(|e| {
                let (ew, eh) = size(base_of(e));
                let (x, y) = lists::placed_at(e);
                (x, y, ew, eh)
            })
            .collect();
        let width = u32::from(mg_rules::levelup::INVENTORY_WIDTH);
        let (x, y) = lists::place(&taken, width, w, h);
        let id = items.len() as u32;
        let item = if t.key.restype == ResType::GIT {
            mg_module::instances::with_placing(game, Some(module), |placing, _| {
                mg_module::instances::held(placing, &bp, id)
            })
        } else {
            let mut e = Struct::new(id);
            e.set("InventoryRes", Value::resref(r));
            e
        };
        let mut item = mg_module::instances::listed_item(&item, id, x as u16, y as u16);
        if store && !item.contains("Infinite") {
            item.set("Infinite", Value::Byte(0));
        }
        out.edits.push(Edit::insert_item(t.key, path, "ItemList", items.len(), item));
        out.changed += 1;
    }
    out
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
        let out = edits(m, &hits, &change, Codepage::WINDOWS_1252, None).unwrap();
        let mut ws = crate::Workspace::new(m.clone());
        ws.apply(crate::Command::new(change.describe(), out.edits.clone())).unwrap();
        ws.flush().unwrap();
        (ws.module.clone(), out)
    }

    fn gff(m: &Module, name: &str, t: ResType) -> Gff {
        m.gff(&ResKey::new(rr(name), t)).unwrap().unwrap()
    }

    /// A name the module's codepage cannot hold is refused, as setting
    /// one is: it was read as no name, and took the variables without a
    /// name off every object found.
    #[test]
    fn a_variable_s_name_the_module_cannot_hold_removes_none() {
        let mut m = module();
        let key = ResKey::parse("guard", ResType::UTC).unwrap();
        let mut utc = gff(&m, "guard", ResType::UTC);
        utc.root.set("VarTable", Value::List(vec![var("", 1), var("nOld", 2)]));
        m.set_gff(key, &utc).unwrap();
        let found = find(&m, Search::Class(4));
        let hits: Vec<&Hit> = found.iter().collect();
        let remove = Change::RemoveVariable { name: "\u{438}\u{43c}\u{44f}".into() };
        let out = edits(&m, &hits, &remove, Codepage::WINDOWS_1252, None);
        assert!(out.as_ref().is_err_and(|e| e.contains("can't hold")), "{out:?}");
        // (The variable with no name is still one to remove by it.)
        let none = Change::RemoveVariable { name: String::new() };
        let out = edits(&m, &hits, &none, Codepage::WINDOWS_1252, None).unwrap();
        assert_eq!((out.changed, out.edits.len()), (1, 1));
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
        assert!(edits(&m, &hits, &bad, Codepage::WINDOWS_1252, None).is_err());
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
        let out = edits(&m, &hits, &set, Codepage::WINDOWS_1252, None).unwrap();
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
        assert!(edits(&m, &hits, &bad, Codepage::WINDOWS_1252, None).is_err());
    }

    /// Feats, classes and items held, added to and taken from the
    /// creatures found by the list rules Edit Together's pages use: where
    /// they lack it, where there is room, never a creature's only class;
    /// other kinds of object found are left, said.
    #[test]
    fn list_entries_added_and_taken_on_each_object_found() {
        let m = module();
        let fighters = find(&m, Search::Class(4));
        let feats = |m: &Module| -> Vec<Vec<i64>> {
            let utc = gff(m, "guard", ResType::UTC).root;
            let git = gff(m, "keep", ResType::GIT).root;
            let placed = git.list("Creature List").unwrap()[0].clone();
            [utc, placed]
                .iter()
                .map(|c| c.list("FeatList").unwrap_or(&[]).iter().filter_map(lists::feat).collect())
                .collect()
        };
        let add = Change::AddEntry { entry: Entry::Feat(7), name: "Alertness".into() };
        assert_eq!(add.describe(), "Add feat Alertness");
        let (after, out) = apply(&m, &fighters, add.clone());
        assert_eq!(out.changed, 2);
        assert_eq!(feats(&after), [vec![7], vec![7]]);
        // Again: each has it already.
        let (again, out) = apply(&after, &find(&after, Search::Class(4)), add);
        assert_eq!(out.changed, 0);
        assert_eq!(
            out.skipped,
            ["guard.utc: has it already", "keep › creature GUARD_01: has it already"]
        );
        assert_eq!(feats(&again), feats(&after));
        let remove = Change::RemoveEntry { entry: Entry::Feat(7), name: "Alertness".into() };
        let (taken, _) = apply(&after, &find(&after, Search::Class(4)), remove);
        assert_eq!(feats(&taken), [Vec::<i64>::new(), Vec::new()]);

        // A class added at its level; the fighter class, their only one,
        // never taken.
        let rogue =
            Change::AddEntry { entry: Entry::Class { class: 8, level: 3 }, name: "Rogue".into() };
        let (after, out) = apply(&m, &fighters, rogue);
        assert_eq!(out.changed, 2);
        let utc = gff(&after, "guard", ResType::UTC);
        let classes = utc.root.list("ClassList").unwrap();
        assert_eq!(classes[1].integer("Class"), Some(8));
        assert_eq!(classes[1].integer("ClassLevel"), Some(3));
        let fighter = Change::RemoveEntry {
            entry: Entry::Class { class: 4, level: 1 },
            name: "Fighter".into(),
        };
        let hits: Vec<&Hit> = fighters.iter().collect();
        let out = edits(&m, &hits, &fighter, Codepage::WINDOWS_1252, None).unwrap();
        assert!(out.edits.is_empty());
        assert_eq!(out.skipped.len(), 2);
        assert!(out.skipped[0].ends_with("its only class"));

        // An item the placed guard holds (a whole item: its tag only here,
        // and no blueprint), taken by its blueprint: none of it here.
        let sword = Change::RemoveEntry { entry: Entry::Item(rr("nw_wswls001")), name: "x".into() };
        let out = edits(&m, &hits, &sword, Codepage::WINDOWS_1252, None).unwrap();
        assert_eq!(out.changed, 0);
        // Adding one needs the game's data, for its size.
        let sword = Change::AddEntry { entry: Entry::Item(rr("nw_wswls001")), name: "x".into() };
        assert!(edits(&m, &hits, &sword, Codepage::WINDOWS_1252, None).is_err());
        // An area found has no feats: left, said.
        let area = find(&m, Search::Field { label: Some("Tag".into()), value: None });
        let area: Vec<&Hit> = area.iter().filter(|h| h.key.restype == ResType::ARE).collect();
        let add = Change::AddEntry { entry: Entry::Feat(7), name: "Alertness".into() };
        let out = edits(&m, &area, &add, Codepage::WINDOWS_1252, None).unwrap();
        assert_eq!(out.changed, 0);
        assert!(out.skipped[0].ends_with("no feats"), "{:?}", out.skipped);
    }

    /// Item properties added where an item lacks one of that type,
    /// subtype and value, and taken by the parts given (one not given:
    /// any).
    #[test]
    fn item_properties_added_and_taken_on_each_item_found() {
        let mut m = module();
        let p = |cost: u16| ItemProperty {
            property: 6,
            subtype: 0,
            cost_table: 2,
            cost_value: cost,
            param1: 255,
            param1_value: 0,
            chance: 100,
        };
        for (name, props) in [("sword_a", vec![p(1)]), ("sword_b", vec![p(2)])] {
            let mut uti = Gff::new(*b"UTI ");
            uti.root.set("Tag", Value::String(b"SWORD".to_vec()));
            uti.root.set("BaseItem", Value::Int(1));
            let list = props.iter().map(ItemProperty::to_gff).collect();
            uti.root.set("PropertiesList", Value::List(list));
            m.set_gff(ResKey::new(rr(name), ResType::UTI), &uti).unwrap();
        }
        let swords =
            find(&m, Search::ItemProperty { property: 6, subtype: None, value: None, param: None });
        let swords: Vec<Hit> =
            swords.into_iter().filter(|h| h.key.restype == ResType::UTI).collect();
        assert_eq!(swords.len(), 2);
        let costs = |m: &Module, n: &str| -> Vec<i64> {
            let uti = gff(m, n, ResType::UTI).root;
            let list = uti.list("PropertiesList").unwrap_or(&[]).to_vec();
            list.iter().filter_map(|e| e.integer("CostValue")).collect()
        };
        let entry = Entry::Property { property: p(1), subtype: None, value: Some(1), param: None };
        let add = Change::AddEntry { entry: entry.clone(), name: "+1".into() };
        let (after, out) = apply(&m, &swords, add);
        assert_eq!(out.changed, 1, "the +2 sword gains a +1");
        assert_eq!(costs(&after, "sword_b"), [2, 1]);
        // Any value: every enhancement taken.
        let any = Entry::Property { property: p(1), subtype: None, value: None, param: None };
        let (taken, out) =
            apply(&after, &swords, Change::RemoveEntry { entry: any, name: "x".into() });
        assert_eq!(out.changed, 2);
        assert!(costs(&taken, "sword_a").is_empty() && costs(&taken, "sword_b").is_empty());
    }
}
