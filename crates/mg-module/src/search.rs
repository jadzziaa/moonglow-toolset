//! Advanced search: the module's blueprints, placed objects, areas,
//! conversations and the rest, found by what they hold rather than by
//! their tag or name: local variables, item properties, creatures'
//! classes (by level), feats, spells and skills, appearances, the scripts
//! attached, or any field by its label and value (a number in a range).
//! Several conditions together find the objects that meet them all, or
//! any of them; and objects that meet a condition can be left out.
//!
//! Every GFF resource of the module is walked, lists included (items in
//! placed creatures' and containers' inventories and equipment, store
//! pages, conversation lines). A blueprint's inventory names its items'
//! blueprints rather than holding them, so an item blueprint that matches
//! is also found where a creature, placeable, store or container
//! blueprint carries it.

use std::collections::HashMap;
use std::sync::Arc;

use mg_core::{Codepage, ResRef, ResType};
use mg_gff::{FieldAt, Gff, Struct, Value};
use mg_resman::ResKey;
use mg_rules::rows::{RowTable, list_table, row_table};
use mg_rules::{Choice, ChoiceColumns, GameData, ItemProperty, ObjectKind};
use rayon::prelude::*;

use crate::Module;
use crate::query::like;
use crate::refs::{RefKind, Reference, gff_references};
pub use crate::vars::VarType;

/// What an appearance is of: the field and table it is a row of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppearanceOf {
    /// A creature's `Appearance_Type` (appearance.2da).
    Creature,
    /// A placeable's `Appearance` (placeables.2da).
    Placeable,
    /// A tileset door's `Appearance` (doortypes.2da).
    Door,
    /// A generic door's `GenericType_New` (genericdoors.2da).
    GenericDoor,
    /// A waypoint's `Appearance` (waypoint.2da).
    Waypoint,
    /// An item's `BaseItem` (baseitems.2da).
    Item,
}

impl AppearanceOf {
    pub const ALL: [AppearanceOf; 6] = [
        AppearanceOf::Creature,
        AppearanceOf::Placeable,
        AppearanceOf::Door,
        AppearanceOf::GenericDoor,
        AppearanceOf::Waypoint,
        AppearanceOf::Item,
    ];

    /// For people: `creature`, `generic door`, `item (base type)`...
    pub fn label(self) -> &'static str {
        match self {
            AppearanceOf::Creature => "creature",
            AppearanceOf::Placeable => "placeable",
            AppearanceOf::Door => "door (tileset door)",
            AppearanceOf::GenericDoor => "door (generic door)",
            AppearanceOf::Waypoint => "waypoint",
            AppearanceOf::Item => "item (base type)",
        }
    }

    /// For the command line: `creature`, `placeable`, `door`,
    /// `generic-door`, `waypoint`, `item`.
    pub fn name(self) -> &'static str {
        match self {
            AppearanceOf::Creature => "creature",
            AppearanceOf::Placeable => "placeable",
            AppearanceOf::Door => "door",
            AppearanceOf::GenericDoor => "generic-door",
            AppearanceOf::Waypoint => "waypoint",
            AppearanceOf::Item => "item",
        }
    }

    pub fn from_name(name: &str) -> Option<AppearanceOf> {
        AppearanceOf::ALL.into_iter().find(|a| a.name().eq_ignore_ascii_case(name.trim()))
    }

    /// The file type and field that hold it.
    pub fn field(self) -> (&'static str, &'static str) {
        match self {
            AppearanceOf::Creature => ("UTC ", "Appearance_Type"),
            AppearanceOf::Placeable => ("UTP ", "Appearance"),
            AppearanceOf::Door => ("UTD ", "Appearance"),
            AppearanceOf::GenericDoor => ("UTD ", "GenericType_New"),
            AppearanceOf::Waypoint => ("UTW ", "Appearance"),
            AppearanceOf::Item => ("UTI ", "BaseItem"),
        }
    }

    /// The table its rows are of, and their names' columns.
    pub fn table(self) -> RowTable {
        let (file, label) = self.field();
        row_table(file, &[], label).expect("a known appearance field")
    }
}

/// A field's value to look for.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// A whole number: fields of every integer type holding exactly it.
    Number(i64),
    /// Text that strings, resrefs and localized strings (any language)
    /// contain, ignoring case; `*` matches any run of characters.
    Text(String),
    /// Fields of every integer and floating-point type holding a number
    /// in this range (ends included; none: no end). A float field is
    /// compared at its own precision, so `1.1` finds the float 1.1.
    Range { min: Option<f64>, max: Option<f64> },
}

impl FieldValue {
    /// A number as typed: `103` (a whole number, exactly), `1.5` (a
    /// float), or a range: `5..10`, `5..`, `..10`, `0.5..1.5`.
    pub fn number(text: &str) -> Result<FieldValue, String> {
        let t = text.trim();
        let parse = |s: &str| -> Result<Option<f64>, String> {
            let s = s.trim();
            if s.is_empty() {
                return Ok(None);
            }
            match s.parse::<f64>() {
                Ok(v) if v.is_finite() => Ok(Some(v)),
                _ => Err(format!("{s} is not a number")),
            }
        };
        if let Some((lo, hi)) = t.split_once("..") {
            let (min, max) = (parse(lo)?, parse(hi)?);
            if min.is_none() && max.is_none() {
                return Err("give a range an end: 5..10, 5.. or ..10".into());
            }
            return Ok(FieldValue::Range { min, max });
        }
        if let Ok(n) = t.parse::<i64>() {
            return Ok(FieldValue::Number(n));
        }
        match parse(t) {
            Ok(Some(v)) => Ok(FieldValue::Range { min: Some(v), max: Some(v) }),
            _ => Err(format!("{t} is not a number or a range (5..10)")),
        }
    }
}

/// What to look for.
#[derive(Debug, Clone, PartialEq)]
pub enum Search {
    /// Local variables by name (ignoring case, `*` matching any run of
    /// characters), and if given their type and value (as the name).
    Variable { name: String, kind: Option<VarType>, value: Option<String> },
    /// Item properties of an itempropdef.2da row, and if given of a
    /// subtype, value (the cost table's row) and parameter value.
    ItemProperty { property: u16, subtype: Option<u16>, value: Option<u16>, param: Option<u8> },
    /// Creatures with a level in a class (classes.2da row).
    Class(i64),
    /// Creatures with `min` to `max` levels (ends included) in a class.
    ClassLevel { class: i64, min: i64, max: i64 },
    /// Creatures with a feat (feat.2da row).
    Feat(i64),
    /// Creatures that know or have memorized a spell, or have it as a
    /// special ability (spells.2da row).
    Spell(i64),
    /// Creatures with at least `min_rank` ranks in a skill (skills.2da
    /// row).
    Skill { skill: usize, min_rank: u8 },
    /// Objects of an appearance.
    Appearance { of: AppearanceOf, row: i64 },
    /// Scripts attached to events and conversation lines, by name
    /// (ignoring case, `*` as in variables).
    Script(String),
    /// Any field: by label (ignoring case, `*` as in variables), value,
    /// or both.
    Field { label: Option<String>, value: Option<FieldValue> },
}

impl Search {
    /// The table a search for a row names its rows from.
    pub fn table(&self) -> Option<RowTable> {
        let creature = |label| row_table("UTC ", &[], label);
        match self {
            Search::Class(_) | Search::ClassLevel { .. } => creature("Class"),
            Search::Feat(_) => creature("Feat"),
            Search::Spell(_) => creature("Spell"),
            Search::Skill { .. } => list_table("UTC ", &[], "SkillList"),
            Search::Appearance { of, .. } => Some(of.table()),
            Search::ItemProperty { .. } => row_table("UTI ", &[], "PropertyName"),
            _ => None,
        }
    }

    /// The row of a table it looks for.
    fn row(&self) -> Option<i64> {
        match self {
            Search::Class(r) | Search::Feat(r) | Search::Spell(r) => Some(*r),
            Search::Appearance { row, .. } => Some(*row),
            _ => None,
        }
    }
}

/// A search, and where it looks.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub search: Search,
    /// Conditions the objects found must also meet: what `search` finds
    /// is kept where the same object (or one holding it, or one it holds)
    /// is found by each of these. With `any`, the others it may meet
    /// instead.
    pub and: Vec<Search>,
    /// What `search` and each of `and` find, together (each found where
    /// it is found, an object meeting several once for each), in place of
    /// what `search` finds where the same object meets them all.
    pub any: bool,
    /// Conditions the objects found must not meet: what is found is left
    /// out where the same object (or one holding it, or one it holds) is
    /// found by one of these, as `and` would keep it.
    pub not: Vec<Search>,
    /// The kinds of object looked at (`utc`, `uti`...; `are` for areas,
    /// `ifo` for the module, `dlg` for conversations); empty: all.
    pub types: Vec<ResType>,
    /// Only placed objects (`Some(true)`), only blueprints (`Some(false)`).
    pub placed: Option<bool>,
    /// Its names and texts (a variable's name and value, a script's
    /// name, a field's label and text) are regular expressions, found
    /// anywhere unless anchored (`^…$`), not `*` patterns.
    pub regex: bool,
}

impl Query {
    pub fn new(search: Search) -> Query {
        Query {
            search,
            and: Vec::new(),
            any: false,
            not: Vec::new(),
            types: Vec::new(),
            placed: None,
            regex: false,
        }
    }

    /// Why one of its regular expressions isn't one (with `regex`).
    pub fn check(&self) -> Result<(), String> {
        if !self.regex {
            return Ok(());
        }
        for s in self.searches() {
            let texts: Vec<&str> = match s {
                Search::Variable { name, value, .. } => {
                    std::iter::once(name.as_str()).chain(value.as_deref()).collect()
                }
                Search::Script(name) => vec![name],
                Search::Field { label, value } => {
                    let text = match value {
                        Some(FieldValue::Text(t)) => Some(t.as_str()),
                        _ => None,
                    };
                    label.as_deref().into_iter().chain(text).collect()
                }
                _ => Vec::new(),
            };
            for t in texts {
                crate::query::check_regex(t)?;
            }
        }
        Ok(())
    }
}

/// Something found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// The resource it is in.
    pub key: ResKey,
    /// Where in it (`/Creature List[3]/VarTable[0]`); a GFF path as
    /// [`rename::Usage`](crate::rename::Usage) has.
    pub path: String,
    /// The object, for people: `keep › creature GUARD › item SWORD`,
    /// `sword.uti`, `keep (area)`.
    pub place: String,
    /// What matched, named as the game names it: `Skill Bonus: Hide +5`,
    /// `Feat: Alertness`, `sDeidad (string) = "Ezra"`.
    pub matched: String,
    /// The kind of object it is on (`uti` for an item, also one in an
    /// inventory; `are`, `ifo`, `dlg`...).
    pub object: ResType,
    /// Through a blueprint's inventory, naming the item blueprint found.
    pub carried: Option<ResRef>,
    /// Through the creatures an encounter spawns (an encounter
    /// blueprint's, or a placed encounter's), naming the creature
    /// blueprint found.
    pub spawned: Option<ResRef>,
}

impl Hit {
    /// Whether it is on an object placed in an area (or an item it
    /// holds).
    pub fn placed(&self) -> bool {
        self.instance().is_some()
    }

    /// For placed objects: the GIT list and index of the object.
    pub fn instance(&self) -> Option<(&str, usize)> {
        if self.key.restype != ResType::GIT {
            return None;
        }
        let (list, index) = item(self.path.strip_prefix('/')?.split('/').next()?)?;
        ObjectKind::from_list(list).map(|_| (list, index))
    }

    /// The place, as Find References names one (to go there).
    pub fn usage(&self) -> crate::rename::Usage {
        crate::rename::Usage { from: self.key, path: self.path.clone(), place: self.place.clone() }
    }

    /// The object it is on: a blueprint (an item blueprint found through
    /// another's inventory: the item's), a placed object or an item one
    /// holds, an area (its ARE, also for what its GIT holds of its own),
    /// the module. None for a conversation and the rest.
    pub fn object(&self) -> Option<Object> {
        self.objects().into_iter().next()
    }

    /// The object it is on, then those that hold it, nearest first: an
    /// item held by a placed creature, the creature, its area; an item
    /// blueprint found in a container blueprint's inventory, the
    /// container.
    pub fn objects(&self) -> Vec<Object> {
        let root = |key: ResKey| Object { key, path: "/".into(), kind: key.restype };
        if let Some(item) = self.carried {
            return vec![root(ResKey::new(item, ResType::UTI)), root(self.key)];
        }
        // A creature blueprint an encounter spawns: it, then the encounter
        // (a placed one's, its area after it).
        if let Some(creature) = self.spawned {
            let mut out = vec![root(ResKey::new(creature, ResType::UTC))];
            match self.key.restype {
                ResType::GIT => {
                    let first = self.path.trim_start_matches('/').split('/').next();
                    let path = format!("/{}", first.unwrap_or_default());
                    out.push(Object { key: self.key, path, kind: ResType::UTE });
                    let are = ResKey::new(self.key.resref, ResType::ARE);
                    out.push(Object { key: are, path: "/".into(), kind: ResType::ARE });
                }
                _ => out.push(root(self.key)),
            }
            return out;
        }
        let area = || {
            let key = ResKey::new(self.key.resref, ResType::ARE);
            Object { key, path: "/".into(), kind: ResType::ARE }
        };
        match self.key.restype {
            ResType::ARE => vec![area()],
            ResType::IFO => vec![root(self.key)],
            ResType::GIT => {
                let steps: Vec<&str> = self
                    .path
                    .trim_start_matches('/')
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .collect();
                let mut out = vec![area()];
                for (i, step) in steps.iter().enumerate() {
                    let Some((list, _)) = item(step) else { continue };
                    let kind = if i == 0 {
                        ObjectKind::from_list(list).map(ObjectKind::restype)
                    } else {
                        ITEM_LISTS.contains(&list).then_some(ResType::UTI)
                    };
                    match kind {
                        Some(kind) => {
                            let path = format!("/{}", steps[..=i].join("/"));
                            out.push(Object { key: self.key, path, kind });
                        }
                        // (The GIT's own lists of other things.)
                        None if i == 0 => break,
                        None => {}
                    }
                }
                out.reverse();
                out
            }
            t if ObjectKind::from_restype(t).is_some() => vec![root(self.key)],
            _ => Vec::new(),
        }
    }
}

/// An object something was found on: the resource, and the path of the
/// object's struct in it (`/` for the resource's own, `/Creature List[3]`
/// for a placed creature, `/Creature List[3]/ItemList[0]` for an item it
/// holds), as [`Hit::path`] writes paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Object {
    pub key: ResKey,
    pub path: String,
    /// The kind of object (`utc`, `uti`...; `are` for an area, `ifo` for
    /// the module).
    pub kind: ResType,
}

impl Object {
    /// Whether it is placed in an area (or an item one holds).
    pub fn placed(&self) -> bool {
        self.key.restype == ResType::GIT
    }
}

/// `List[3]`: the list's label and the index.
fn item(step: &str) -> Option<(&str, usize)> {
    let (l, rest) = step.split_once('[')?;
    Some((l, rest.strip_suffix(']')?.parse().ok()?))
}

/// The structs a path steps through: the root's, then the list items
/// (labels and indices) and structs it names, with each step's label.
fn path_labels(path: &str) -> Vec<&str> {
    path.trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.split('[').next().unwrap_or(s))
        .collect()
}

/// Lists whose items are items (in inventories and equipment, and items
/// lying in an area).
const ITEM_LISTS: [&str; 2] = ["ItemList", "Equip_ItemList"];

/// The kind of object the struct at the end of `steps` is (the steps
/// through list items, as a path has them).
fn object_type(key: ResKey, steps: &[&str]) -> ResType {
    // (A GIT's own fields, its `AreaProperties`, are the area's.)
    let mut t = if key.restype == ResType::GIT { ResType::ARE } else { key.restype };
    for (i, step) in steps.iter().enumerate() {
        let Some((list, _)) = item(step) else { continue };
        if key.restype == ResType::GIT && i == 0 {
            if let Some(k) = ObjectKind::from_list(list) {
                t = k.restype();
            }
        } else if ITEM_LISTS.contains(&list) {
            t = ResType::UTI;
        }
    }
    t
}

/// An object's name in a place: its tag, else its blueprint, else its
/// place in the list.
fn object_name(s: &Struct, index: usize, cp: Codepage) -> String {
    let tag = s.string("Tag").map(|t| cp.decode(t).into_owned()).unwrap_or_default();
    if !tag.is_empty() {
        return tag;
    }
    let template = ["TemplateResRef", "InventoryRes", "EquippedRes"]
        .iter()
        .find_map(|l| s.resref(l).filter(|r| !r.is_empty()));
    match template {
        Some(r) => r.to_lowercase().to_string(),
        None => format!("#{}", index + 1),
    }
}

/// The kind of a placed object's list, for people.
fn git_kind(list: &str) -> &'static str {
    ObjectKind::from_list(list).map_or("object", ObjectKind::name)
}

/// The object a path is in, for people: `keep › creature GUARD › item
/// SWORD` (a placed object and the items it holds), `sword.uti`, `keep
/// (area)`, `Module`; a conversation's line as Find References names it.
fn place(key: ResKey, gff: &Gff, path: &str, cp: Codepage) -> String {
    let name = key.resref.to_lowercase().to_string();
    let first = path.trim_start_matches('/').split('/').next().and_then(item);
    let mut out = match key.restype {
        ResType::GIT if first.is_some_and(|(l, _)| ObjectKind::from_list(l).is_some()) => name,
        ResType::GIT => return format!("{name} (area)"),
        ResType::ARE => return format!("{name} (area)"),
        ResType::IFO => return "Module".to_string(),
        t if ObjectKind::from_restype(t).is_some() => key.to_string(),
        _ => return crate::rename::describe(key, Some(gff), path),
    };
    let mut s = &gff.root;
    for (i, step) in path.trim_start_matches('/').split('/').enumerate() {
        let Some((list, index)) = item(step) else {
            match s.get(step) {
                Some(Value::Struct(c)) => s = c,
                _ => break,
            }
            continue;
        };
        let Some(next) = s.list(list).and_then(|l| l.get(index)) else { break };
        let what = if key.restype == ResType::GIT && i == 0 {
            Some(git_kind(list))
        } else if list == "Equip_ItemList" {
            Some("equipped")
        } else if list == "ItemList" {
            Some("item")
        } else {
            None
        };
        if let Some(what) = what {
            out = format!("{out} › {what} {}", object_name(next, index, cp));
        }
        s = next;
    }
    out
}

/// The game's name of a table's row, else its number.
fn row_name(game: Option<&GameData>, table: &RowTable, row: i64) -> String {
    game.and_then(|g| g.row_name(table, row)).unwrap_or_else(|| format!("row {row}"))
}

/// A variable's value as text: numbers as written, a string as it is, an
/// object's id in hexadecimal, a location as `location`.
fn variable_value(var: &Struct, cp: Codepage) -> String {
    match var.get("Value") {
        Some(Value::String(b)) => cp.decode(b).into_owned(),
        Some(Value::Dword(n))
            if VarType::of(var.integer("Type").unwrap_or(0)) == Some(VarType::Object) =>
        {
            format!("{n:#x}")
        }
        Some(Value::Struct(_)) => "location".to_string(),
        Some(v) => crate::query::field_text(v).unwrap_or_default(),
        None => String::new(),
    }
}

/// The texts a field holds, for a text search: a string's, a resref's,
/// each language's of a localized string.
fn texts(v: &Value, cp: Codepage) -> Vec<String> {
    match v {
        Value::String(b) | Value::ResRef(b) => vec![cp.decode(b).into_owned()],
        Value::LocString(ls) => ls.strings.iter().map(|(_, b)| cp.decode(b).into_owned()).collect(),
        _ => Vec::new(),
    }
}

/// A field's value shown: at most 80 characters.
fn shown(v: &Value, cp: Codepage) -> String {
    let text = match v {
        Value::String(_) | Value::ResRef(_) => format!("\"{}\"", texts(v, cp).join("")),
        Value::LocString(ls) => match texts(v, cp).first() {
            Some(t) => format!("\"{t}\""),
            None if !ls.strref.is_none() => format!("StrRef {}", ls.strref.0),
            None => "\"\"".to_string(),
        },
        Value::List(items) => format!("(a list of {})", items.len()),
        Value::Struct(_) => "(a struct)".to_string(),
        v => crate::query::field_text(v).unwrap_or_else(|| "(data)".to_string()),
    };
    let mut out: String = text.chars().take(80).collect();
    if out.len() < text.len() {
        out.push('…');
    }
    out
}

/// Whether a number field holds a number in a range: a float compared
/// at its own precision (the ends made floats too).
fn in_range(v: &Value, min: Option<f64>, max: Option<f64>) -> bool {
    let (n, single) = match *v {
        Value::Float(x) => (f64::from(x), true),
        Value::Double(x) => (x, false),
        ref v => match v.as_integer() {
            Some(n) => (n as f64, false),
            None => return false,
        },
    };
    let end = |e: f64| if single { f64::from(e as f32) } else { e };
    min.is_none_or(|m| n >= end(m)) && max.is_none_or(|m| n <= end(m))
}

/// Whether `text` contains `pattern`, ignoring case, `*` matching any run
/// (with `regex`, a regular expression found anywhere in it).
fn contains(pattern: &str, text: &str, regex: bool) -> bool {
    match regex {
        true => like(pattern, text, true),
        false => like(&format!("*{pattern}*"), text, false),
    }
}

/// An event's name, for people: `ScriptSpawn` is `OnSpawn`.
pub fn event(label: &str) -> String {
    match label.strip_prefix("Script") {
        Some(rest) if !rest.is_empty() => format!("On{rest}"),
        _ => match label {
            "Active" => "Condition".to_string(),
            "Script" => "Action".to_string(),
            l => l.to_string(),
        },
    }
}

/// What one field matches, if it does: the path of what matched and the
/// text that says so.
fn field_hit(
    search: &Search,
    file_type: &str,
    f: &FieldAt<'_>,
    game: Option<&GameData>,
    cp: Codepage,
    regex: bool,
) -> Option<(String, String)> {
    let glob = |p: &str, t: &str| like(p, t, regex);
    // The path of the struct the field is in.
    let owner = || f.path.rsplit_once('/').map_or("", |(p, _)| p).to_string();
    match search {
        Search::Variable { name, kind, value } => {
            if f.list != Some("VarTable") || f.label != "Name" {
                return None;
            }
            let var = f.owner;
            let var_name = cp.decode(var.string("Name")?).into_owned();
            let t = VarType::of(var.integer("Type").unwrap_or(0));
            let text = variable_value(var, cp);
            let hit = glob(name, &var_name)
                && kind.is_none_or(|k| t == Some(k))
                && value.as_ref().is_none_or(|v| glob(v, &text));
            let type_name = t.map_or("?", VarType::name);
            let shown = match t {
                Some(VarType::String) => format!("\"{text}\""),
                _ => text,
            };
            hit.then(|| (owner(), format!("{var_name} ({type_name}) = {shown}")))
        }
        Search::ItemProperty { property, subtype, value, param } => {
            if f.list != Some("PropertiesList") || f.label != "PropertyName" {
                return None;
            }
            let p = ItemProperty::from_gff(f.owner);
            let hit = p.property == *property
                && subtype.is_none_or(|s| p.subtype == s)
                && value.is_none_or(|v| p.cost_value == v)
                && param.is_none_or(|v| p.param1_value == v);
            let text = match game {
                Some(g) => g.property_text(&p),
                None => format!(
                    "Property {} (subtype {}, value {})",
                    p.property, p.subtype, p.cost_value
                ),
            };
            hit.then(|| (owner(), text))
        }
        Search::Class(_) | Search::Feat(_) | Search::Spell(_) | Search::Appearance { .. } => {
            let want = search.table()?;
            let n = f.value.as_integer()?;
            if Some(n) != search.row() {
                return None;
            }
            let labels = path_labels(f.path);
            let table = row_table(file_type, &labels[..labels.len().saturating_sub(1)], f.label)?;
            if table.table != want.table {
                return None;
            }
            let name = row_name(game, &table, n);
            let list = f.list.unwrap_or_default();
            let what = match list {
                "ClassList" => {
                    let level = f.owner.integer("ClassLevel").unwrap_or(0);
                    return Some((owner(), format!("Class: {name}, level {level}")));
                }
                "FeatList" => "Feat",
                "SpecAbilityList" => "Special ability",
                l if l.starts_with("KnownList") => "Known spell",
                l if l.starts_with("MemorizedList") => "Memorized spell",
                _ if matches!(search, Search::Appearance { of: AppearanceOf::Item, .. }) => {
                    "Base item"
                }
                _ => "Appearance",
            };
            Some((f.path.to_string(), format!("{what}: {name}")))
        }
        Search::ClassLevel { class, min, max } => {
            if f.list != Some("ClassList")
                || f.label != "Class"
                || f.value.as_integer() != Some(*class)
            {
                return None;
            }
            let level = f.owner.integer("ClassLevel").unwrap_or(0);
            if level < *min || level > *max {
                return None;
            }
            let name = row_name(game, &search.table()?, *class);
            Some((owner(), format!("Class: {name}, level {level}")))
        }
        Search::Skill { skill, min_rank } => {
            if f.list != Some("SkillList") || f.label != "Rank" {
                return None;
            }
            let labels = path_labels(f.path);
            let at = labels.len().saturating_sub(2);
            let table = list_table(file_type, &labels[..at], "SkillList")?;
            let (_, index) = item(f.path.rsplit('/').nth(1)?)?;
            let rank = f.value.as_integer()?;
            let hit = index == *skill && rank >= i64::from((*min_rank).max(1));
            let name = row_name(game, &table, index as i64);
            hit.then(|| (owner(), format!("Skill: {name}, {rank} rank(s)")))
        }
        Search::Script(_) => None,
        Search::Field { label, value } => {
            if label.as_ref().is_some_and(|l| !glob(l, f.label)) {
                return None;
            }
            let hit = match value {
                None => true,
                Some(FieldValue::Number(n)) => f.value.as_integer() == Some(*n),
                Some(FieldValue::Text(t)) => {
                    texts(f.value, cp).iter().any(|s| contains(t, s, regex))
                }
                Some(FieldValue::Range { min, max }) => in_range(f.value, *min, *max),
            };
            if !hit || (label.is_none() && value.is_none()) {
                return None;
            }
            Some((f.path.to_string(), format!("{} = {}", f.label, shown(f.value, cp))))
        }
    }
}

/// What one resource holds that each of `searches` matches, in the order
/// of its fields (one walk of them for all). A search of the scripts
/// attached reads the resource's references.
fn resource_hits(
    read: &Read<'_>,
    searches: &[&Search],
    game: Option<&GameData>,
    cp: Codepage,
    regex: bool,
) -> Vec<Vec<Hit>> {
    let Read { key, gff, refs } = *read;
    let file_type = gff.file_type_str();
    let mut found: Vec<Vec<(String, String)>> = vec![Vec::new(); searches.len()];
    for (s, found) in searches.iter().zip(&mut found) {
        let Search::Script(pattern) = s else { continue };
        for r in refs {
            if r.kind == RefKind::Script && like(pattern, &r.target.to_string(), regex) {
                let label = r.path.rsplit('/').next().unwrap_or_default();
                let script = r.target.to_lowercase();
                found.push((r.path.clone(), format!("{}: {script}", event(label))));
            }
        }
    }
    if searches.iter().any(|s| !matches!(s, Search::Script(_))) {
        gff.root.walk_fields(&mut |f| {
            for (s, found) in searches.iter().zip(&mut found) {
                if let Some(hit) = field_hit(s, &file_type, f, game, cp, regex) {
                    found.push(hit);
                }
            }
        });
    }
    let hits = |found: Vec<(String, String)>| {
        let mut out: Vec<Hit> = Vec::new();
        for (path, matched) in found {
            let steps: Vec<&str> = path.trim_start_matches('/').split('/').collect();
            let object = object_type(key, &steps);
            // (A struct matched by more than one of its fields once.)
            if out.last().is_some_and(|h| h.path == path) {
                continue;
            }
            out.push(Hit {
                key,
                place: place(key, gff, &path, cp),
                path,
                matched,
                object,
                carried: None,
                spawned: None,
            });
        }
        out
    };
    found.into_iter().map(hits).collect()
}

/// Whether what a search finds on an item blueprint is also found where
/// the blueprints' inventories carry it (not for a search of fields,
/// which finds fields).
fn carrying(s: &Search) -> bool {
    !matches!(s, Search::Field { .. })
}

/// Whether what `searches` find in `key` needs its references: the
/// scripts attached, the items a blueprint carries.
fn needs_refs(key: ResKey, searches: &[&Search]) -> bool {
    // (An area's: its encounters' creatures.)
    let blueprint = ObjectKind::from_restype(key.restype).is_some() || key.restype == ResType::GIT;
    searches.iter().any(|s| matches!(s, Search::Script(_)) || (blueprint && carrying(s)))
}

/// Whether a hit is in what the query looks at.
fn wanted(q: &Query, h: &Hit) -> bool {
    let placed_ok = match q.placed {
        None => true,
        Some(true) => h.placed(),
        Some(false) => ObjectKind::from_restype(h.key.restype).is_some(),
    };
    placed_ok && (q.types.is_empty() || q.types.contains(&h.object))
}

/// A GFF resource of the module as read: its key, its fields, and the
/// references they make (`refs::gff_references`; left out where the
/// searches don't need them).
#[derive(Clone, Copy)]
struct Read<'a> {
    key: ResKey,
    gff: &'a Gff,
    refs: &'a [Reference],
}

/// Everything in the module the query matches: resource by resource (in
/// the order of their names and types), field by field; an item
/// blueprint found is also found in the inventories of the blueprints
/// that carry it (but not for a search of fields, which finds fields).
/// With other conditions, what is found on an object that meets them
/// all, each one's first match told after its own (`Class: Fighter,
/// level 5 · Feat: Alertness`); with `any`, what each finds; less what
/// is on an object meeting one of `not`. `game` names what is found as the game
/// does (else by numbers); `cp` reads the module's text. Each resource
/// is read once, for all the conditions ([`Kept`] keeps what it found).
pub fn search(m: &Module, q: &Query, game: Option<&GameData>, cp: Codepage) -> Vec<Hit> {
    Kept::default().search(m, q, game, cp)
}

/// What the last question's conditions found, resource by resource, kept
/// for the next time it is asked (Advanced Search finds again after a
/// change, and when Find is clicked again): a resource is read again only
/// when the module holds other bytes for it, as
/// [`UsageIndex`](crate::rename::UsageIndex) does. Another question reads
/// the module again, once for all its conditions, on every core. What is
/// found is named with the game data and codepage it was asked with:
/// with others, start anew.
#[derive(Debug, Default)]
pub struct Kept {
    /// The conditions asked.
    searches: Vec<Search>,
    /// Whether they were asked as regular expressions.
    regex: bool,
    /// Each GFF resource, as read, and what they found in it (none: it
    /// doesn't read).
    found: HashMap<ResKey, KeptOne>,
    /// How many resources the last search read.
    pub read: usize,
}

/// A resource's bytes as read, and what the conditions found in it.
type KeptOne = (Arc<[u8]>, Option<Found>);

impl Kept {
    /// [`search`], reading only the resources new or changed since the
    /// last time `q`'s conditions were asked.
    pub fn search(
        &mut self,
        m: &Module,
        q: &Query,
        game: Option<&GameData>,
        cp: Codepage,
    ) -> Vec<Hit> {
        let searches = q.searches();
        if !self.searches.iter().eq(searches.iter().copied()) || self.regex != q.regex {
            self.searches = searches.iter().map(|s| (*s).clone()).collect();
            self.regex = q.regex;
            self.found.clear();
        }
        self.found.retain(|k, _| m.contains(k));
        let mut changed = Vec::new();
        for key in m.keys().filter(|k| k.restype.is_gff()) {
            let Some(data) = m.shared(key) else { continue };
            match self.found.get_mut(key) {
                Some((was, _)) if Arc::ptr_eq(was, data) => continue,
                // (Set again with the same bytes, as a flush of the
                // documents open does: nothing to read.)
                Some((was, _)) if **was == **data => {
                    *was = data.clone();
                    continue;
                }
                _ => {}
            }
            changed.push((*key, data));
        }
        self.read = changed.len();
        // (Each tree let go once walked: what is kept is what was found.)
        let read: Vec<(ResKey, KeptOne)> = changed
            .into_par_iter()
            .map(|(key, data)| {
                let found = Gff::read(data).ok().map(|gff| {
                    let refs = match needs_refs(key, &searches) {
                        true => gff_references(key, &gff),
                        false => Vec::new(),
                    };
                    let read = Read { key, gff: &gff, refs: &refs };
                    Found::of(&read, &searches, game, cp, q.regex)
                });
                (key, (data.clone(), found))
            })
            .collect();
        self.found.extend(read);
        let mut found: Vec<(ResKey, &Found)> =
            (self.found.iter()).filter_map(|(k, (_, f))| Some((*k, f.as_ref()?))).collect();
        found.sort_by_key(|(k, _)| *k);
        joined(&found, q)
    }
}

/// What the query finds, from what each resource holds (`found`, in the
/// order of their keys).
fn joined(found: &[(ResKey, &Found)], q: &Query) -> Vec<Hit> {
    let mut hits = match q.any {
        false => {
            let mut hits = condition_hits(found, 0, &q.search);
            for (c, extra) in q.and.iter().enumerate() {
                let theirs = condition_hits(found, c + 1, extra);
                let near = Near::of(&theirs);
                hits.retain_mut(|h| {
                    let also = near.meets(h);
                    if let Some(text) = also {
                        h.matched = format!("{} · {text}", h.matched);
                    }
                    also.is_some()
                });
            }
            hits
        }
        true => any_of(found, q),
    };
    for (c, not) in q.not.iter().enumerate() {
        let theirs = condition_hits(found, 1 + q.and.len() + c, not);
        let near = Near::of(&theirs);
        hits.retain(|h| near.meets(h).is_none());
    }
    hits.retain(|h| wanted(q, h));
    hits
}

/// What each of the query's first conditions (`search` and `and`) finds,
/// together, resource by resource (each one's in its turn); a place two
/// of them find once, telling what each found (`Feat: Alertness · Class:
/// Fighter, level 5`).
fn any_of(found: &[(ResKey, &Found)], q: &Query) -> Vec<Hit> {
    let mut hits: Vec<Hit> = Vec::new();
    let mut at: HashMap<(ResKey, String, Option<ResRef>, Option<ResRef>), usize> = HashMap::new();
    for (c, s) in std::iter::once(&q.search).chain(&q.and).enumerate() {
        for h in condition_hits(found, c, s) {
            let place = (h.key, h.path.clone(), h.carried, h.spawned);
            match at.get(&place) {
                Some(&i) if !hits[i].matched.split(" · ").any(|m| m == h.matched) => {
                    hits[i].matched = format!("{} · {}", hits[i].matched, h.matched);
                }
                Some(_) => {}
                None => {
                    at.insert(place, hits.len());
                    hits.push(h);
                }
            }
        }
    }
    // (Stable: within a resource, each condition's in the order found.)
    hits.sort_by_key(|h| h.key);
    hits
}

/// The objects a condition finds things on, and those holding them, each
/// with what it first found there.
struct Near<'a> {
    own: HashMap<Object, &'a str>,
    held: HashMap<Object, &'a str>,
}

impl<'a> Near<'a> {
    fn of(theirs: &'a [Hit]) -> Near<'a> {
        let mut own: HashMap<Object, &str> = HashMap::new();
        let mut held: HashMap<Object, &str> = HashMap::new();
        for h in theirs {
            let objects = h.objects();
            if let Some(o) = objects.first() {
                own.entry(o.clone()).or_insert(&h.matched);
            }
            for o in objects {
                held.entry(o).or_insert(&h.matched);
            }
        }
        Near { own, held }
    }

    /// What the condition found on the object `h` is on, on one it holds,
    /// or on one holding it.
    fn meets(&self, h: &Hit) -> Option<&'a str> {
        let objects = h.objects();
        let first = objects.first().and_then(|o| self.held.get(o)).copied();
        first.or_else(|| objects.iter().find_map(|o| self.own.get(o).copied()))
    }
}

impl Query {
    /// Its searches: the first, the others with it, then those it must
    /// not meet.
    fn searches(&self) -> Vec<&Search> {
        std::iter::once(&self.search).chain(&self.and).chain(&self.not).collect()
    }
}

/// What one resource holds for a query: each search's hits, (a
/// blueprint's) the item blueprints its inventory names, and (an
/// encounter blueprint's, an area's encounters') the creature blueprints
/// they spawn, where.
#[derive(Debug)]
struct Found {
    hits: Vec<Vec<Hit>>,
    /// (path, item blueprint, place).
    carries: Vec<(String, ResRef, String)>,
    /// (path, creature blueprint, place).
    spawns: Vec<(String, ResRef, String)>,
}

impl Found {
    fn of(
        read: &Read<'_>,
        searches: &[&Search],
        game: Option<&GameData>,
        cp: Codepage,
        regex: bool,
    ) -> Found {
        let hits = resource_hits(read, searches, game, cp, regex);
        let key = read.key;
        let (mut carries, mut spawns) = (Vec::new(), Vec::new());
        if searches.iter().any(|s| carrying(s)) {
            let blueprint = ObjectKind::from_restype(key.restype).is_some();
            let encounters = matches!(key.restype, ResType::UTE | ResType::GIT);
            for r in read.refs {
                let target = r.target.to_lowercase();
                if blueprint && r.kind == RefKind::Blueprint(ResType::UTI) {
                    let place = format!("{key} › carries {target}.uti");
                    carries.push((r.path.clone(), r.target, place));
                } else if encounters
                    && r.kind == RefKind::Blueprint(ResType::UTC)
                    && spawned_at(key.restype, &r.path)
                {
                    let at = place(key, read.gff, &r.path, cp);
                    spawns.push((r.path.clone(), r.target, format!("{at} › spawns {target}.utc")));
                }
            }
        }
        Found { hits, carries, spawns }
    }
}

/// What search `c` of the query (`s`) finds, before the query keeps what
/// it looks at: each resource's hits, an item blueprint's after them
/// where a blueprint carries it.
fn condition_hits(found: &[(ResKey, &Found)], c: usize, s: &Search) -> Vec<Hit> {
    let mut by_key: Vec<Vec<Hit>> = found.iter().map(|(_, f)| f.hits[c].clone()).collect();
    if carrying(s) {
        // The item blueprints found, and what they matched.
        let items: HashMap<ResRef, Vec<String>> = (found.iter().zip(&by_key))
            .filter(|((k, _), h)| k.restype == ResType::UTI && !h.is_empty())
            .map(|((k, _), h)| {
                let direct = h.iter().filter(|h| h.carried.is_none() && h.object == ResType::UTI);
                (k.resref, direct.map(|h| h.matched.clone()).collect())
            })
            .collect();
        // What each item blueprint carries (a bag's), to follow a found
        // one down through bags in bags.
        let bags: HashMap<ResRef, &[(String, ResRef, String)]> = (found.iter())
            .filter(|(k, f)| k.restype == ResType::UTI && !f.carries.is_empty())
            .map(|(k, f)| (k.resref, f.carries.as_slice()))
            .collect();
        for ((key, f), hits) in found.iter().zip(&mut by_key) {
            for (path, target, place) in &f.carries {
                let own = (key.restype == ResType::UTI).then_some(key.resref);
                for (item, through) in carried_found(*target, own, &items, &bags) {
                    let place = format!("{place}{through}");
                    for m in &items[&item] {
                        hits.push(Hit {
                            key: *key,
                            path: path.clone(),
                            place: place.clone(),
                            matched: m.clone(),
                            object: ResType::UTI,
                            carried: Some(item),
                            spawned: None,
                        });
                    }
                }
            }
        }
        // The creature blueprints found, in the encounters that spawn them.
        let creatures: HashMap<ResRef, Vec<String>> = (found.iter().zip(&by_key))
            .filter(|((k, _), h)| k.restype == ResType::UTC && !h.is_empty())
            .map(|((k, _), h)| {
                let direct = h.iter().filter(|h| h.carried.is_none() && h.object == ResType::UTC);
                (k.resref, direct.map(|h| h.matched.clone()).collect())
            })
            .collect();
        for ((key, f), hits) in found.iter().zip(&mut by_key) {
            for (path, target, place) in &f.spawns {
                let Some(matched) = creatures.get(target) else { continue };
                for m in matched {
                    hits.push(Hit {
                        key: *key,
                        path: path.clone(),
                        place: place.clone(),
                        matched: m.clone(),
                        object: ResType::UTC,
                        carried: None,
                        spawned: Some(*target),
                    });
                }
            }
        }
    }
    by_key.into_iter().flatten().collect()
}

/// Whether a creature blueprint named at `path` of a resource of type
/// `restype` is one an encounter spawns: in an encounter blueprint's
/// `CreatureList`, or a placed encounter's (not a placed creature's
/// `TemplateResRef`).
fn spawned_at(restype: ResType, path: &str) -> bool {
    let steps: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let listed = |s: &str| item(s).is_some_and(|(l, _)| l == "CreatureList");
    match (restype, steps.as_slice()) {
        (ResType::UTE, [list, ..]) => listed(list),
        (ResType::GIT, [placed, list, ..]) => {
            item(placed).is_some_and(|(l, _)| l == "Encounter List") && listed(list)
        }
        _ => false,
    }
}

/// How many bags in bags a found item blueprint is followed up through.
const BAGS_DEEP: usize = 8;

/// The item blueprints found (`items`) that carrying `target` carries: it,
/// if it was found, and those found in what it carries as a bag
/// (`bags`), down to [`BAGS_DEEP`] bags, each once and not back into the
/// carrier (`own`, an item blueprint's); with each, the rest of its place
/// (` › carries sword.uti`, for one in a bag).
fn carried_found(
    target: ResRef,
    own: Option<ResRef>,
    items: &HashMap<ResRef, Vec<String>>,
    bags: &HashMap<ResRef, &[(String, ResRef, String)]>,
) -> Vec<(ResRef, String)> {
    let mut out = Vec::new();
    let mut seen: std::collections::HashSet<ResRef> = own.into_iter().collect();
    let mut walk = vec![(target, String::new())];
    while let Some((at, through)) = walk.pop() {
        if !seen.insert(at) {
            continue;
        }
        if items.contains_key(&at) {
            out.push((at, through.clone()));
        }
        if through.matches(" › ").count() >= BAGS_DEEP {
            continue;
        }
        for (_, inner, _) in bags.get(&at).copied().unwrap_or_default().iter().rev() {
            let step = format!("{through} › carries {}.uti", inner.to_lowercase());
            walk.push((*inner, step));
        }
    }
    out
}

/// The choices of a table, named as the game names them: a talk-table
/// string, else the label.
pub fn table_choices(game: &GameData, table: &RowTable) -> Vec<Choice> {
    let columns = ChoiceColumns { name: table.name, label: table.label };
    game.choices(table.table, columns).unwrap_or_default()
}

/// The tables an item property's parts are rows of: its subtypes, its
/// values (a cost table) and its parameter's values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PropertyParts {
    pub subtypes: Vec<Choice>,
    pub values: Vec<Choice>,
    pub params: Vec<Choice>,
}

/// An item property type's parts (the parameter as it is for `subtype`).
pub fn property_parts(game: &GameData, property: u16, subtype: Option<u16>) -> PropertyParts {
    let Some(t) = game.property_type(property) else { return PropertyParts::default() };
    let named = |table: Option<String>| -> Vec<Choice> {
        let Some(table) = table.filter(|t| !t.is_empty() && t != "****") else {
            return Vec::new();
        };
        let columns = ChoiceColumns { name: Some("Name"), label: Some("Label") };
        game.choices(&table.to_lowercase(), columns).unwrap_or_default()
    };
    let cell = |table: &str, row: usize, column: &str| {
        game.table(table).ok()?.get(row, column).map(str::to_string)
    };
    let values = match t.cost_table {
        0 => Vec::new(),
        c => named(cell("iprp_costtable", c as usize, "Name")),
    };
    let params = game
        .property_param_table(&t, subtype.unwrap_or(0))
        .map(|p| named(cell("iprp_paramtable", p as usize, "TableResRef")))
        .unwrap_or_default();
    PropertyParts { subtypes: named(t.subtypes.clone()), values, params }
}

/// A row of a table, given as its number or its name (the game's name or
/// the label, ignoring case and accents; else the one row whose name
/// contains it).
pub fn row_named(choices: &[Choice], table: &str, text: &str) -> Result<usize, String> {
    let text = text.trim();
    if let Ok(n) = text.parse::<usize>() {
        return Ok(n);
    }
    if choices.is_empty() {
        return Err(format!("{text}: give a row number of {table}.2da (no game data to name it)"));
    }
    // (Case and accents aside.)
    let lower = mg_core::text::fold(text);
    if let Some(c) = choices.iter().find(|c| mg_core::text::fold(&c.text) == lower) {
        return Ok(c.row);
    }
    let some: Vec<&Choice> =
        choices.iter().filter(|c| mg_core::text::fold(&c.text).contains(&lower)).collect();
    match some.as_slice() {
        [one] => Ok(one.row),
        [] => Err(format!("{text}: no row of {table}.2da is named so")),
        many => Err(format!(
            "{text}: {} rows of {table}.2da are named so: {}",
            many.len(),
            many.iter()
                .take(8)
                .map(|c| format!("{} ({})", c.text, c.row))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{Gender, Language, LocString};
    use mg_resman::{LayerClass, MemContainer, ResMan, priority};
    use mg_tlk::{Tlk, TlkEntry};

    use super::*;

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    fn var(name: &str, value: Value) -> Struct {
        let mut v = Struct::new(0);
        v.set("Name", Value::String(name.as_bytes().to_vec()));
        let t = match value {
            Value::Int(_) => 1,
            Value::Float(_) => 2,
            Value::String(_) => 3,
            Value::Dword(_) => 4,
            _ => 5,
        };
        v.set("Type", Value::Dword(t));
        v.set("Value", value);
        v
    }

    fn property(name: u16, subtype: u16, cost_table: u8, value: u16) -> Struct {
        let mut p = Struct::new(0);
        p.set("PropertyName", Value::Word(name));
        p.set("Subtype", Value::Word(subtype));
        p.set("CostTable", Value::Byte(cost_table));
        p.set("CostValue", Value::Word(value));
        p.set("Param1", Value::Byte(255));
        p.set("Param1Value", Value::Byte(0));
        p.set("ChanceAppear", Value::Byte(100));
        p
    }

    fn item(tag: &str, vars: Vec<Struct>, props: Vec<Struct>) -> Struct {
        let mut s = Struct::new(0);
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s.set("BaseItem", Value::Int(1));
        s.set("VarTable", Value::List(vars));
        s.set("PropertiesList", Value::List(props));
        s
    }

    fn creature(tag: &str) -> Struct {
        let mut c = Struct::new(4);
        c.set("Tag", Value::String(tag.as_bytes().to_vec()));
        c.set("TemplateResRef", Value::resref(rr("guard")));
        c.set("Appearance_Type", Value::Word(6));
        c.set("ScriptSpawn", Value::resref(rr("x2_def_spawn")));
        let mut class = Struct::new(2);
        class.set("Class", Value::Int(4));
        class.set("ClassLevel", Value::Short(5));
        let mut known = Struct::new(3);
        known.set("Spell", Value::Word(107));
        class.set("KnownList1", Value::List(vec![known]));
        c.set("ClassList", Value::List(vec![class]));
        let mut feat = Struct::new(1);
        feat.set("Feat", Value::Word(0));
        c.set("FeatList", Value::List(vec![feat]));
        let mut special = Struct::new(4);
        special.set("Spell", Value::Word(58));
        c.set("SpecAbilityList", Value::List(vec![special]));
        let skills = (0..4)
            .map(|i| {
                let mut s = Struct::new(0);
                s.set("Rank", Value::Byte(if i == 3 { 6 } else { 0 }));
                s
            })
            .collect();
        c.set("SkillList", Value::List(skills));
        c
    }

    /// A module: a sword blueprint with a variable and a Hide bonus, a
    /// chest blueprint carrying it, a guard blueprint, and an area with a
    /// guard who carries a sword of his own, and an item lying there.
    fn module() -> Module {
        let mut m = Module::new();
        let mut uti = Gff::new(*b"UTI ");
        uti.root = item(
            "SWORD",
            vec![var("sDeidad", Value::String(b"Ezra".to_vec()))],
            vec![property(52, 3, 25, 5)],
        );
        uti.root.set("TemplateResRef", Value::resref(rr("sword")));
        m.set_gff(ResKey::parse("sword", ResType::UTI).unwrap(), &uti).unwrap();
        let mut utp = Gff::new(*b"UTP ");
        utp.root.set("Tag", Value::String(b"CHEST".to_vec()));
        utp.root.set("Appearance", Value::Dword(103));
        let mut held = Struct::new(0);
        held.set("InventoryRes", Value::resref(rr("sword")));
        utp.root.set("ItemList", Value::List(vec![held]));
        m.set_gff(ResKey::parse("chest", ResType::UTP).unwrap(), &utp).unwrap();
        let mut utc = Gff::new(*b"UTC ");
        utc.root = creature("GUARD");
        m.set_gff(ResKey::parse("guard", ResType::UTC).unwrap(), &utc).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut guard = creature("GUARD_01");
        let own = item(
            "OLD_SWORD",
            vec![var("nDeidadBonus", Value::Int(2))],
            vec![property(52, 3, 25, 2)],
        );
        guard.set("ItemList", Value::List(vec![own]));
        guard.set("VarTable", Value::List(vec![var("sdeidad", Value::Int(103))]));
        git.root.set("Creature List", Value::List(vec![guard]));
        let lying = item("GEM", Vec::new(), vec![property(15, 107, 3, 1)]);
        git.root.set("List", Value::List(vec![lying]));
        let mut audio = Struct::new(100);
        audio.set("MusicDay", Value::Int(103));
        git.root.set("AreaProperties", Value::Struct(audio));
        m.set_gff(ResKey::parse("keep", ResType::GIT).unwrap(), &git).unwrap();
        let mut are = Gff::new(*b"ARE ");
        are.root.set("VarTable", Value::List(vec![var("AREA_DEIDAD", Value::Float(1.5))]));
        m.set_gff(ResKey::parse("keep", ResType::ARE).unwrap(), &are).unwrap();
        m
    }

    fn find(m: &Module, s: Search) -> Vec<Hit> {
        search(m, &Query::new(s), None, Codepage::WINDOWS_1252)
    }

    fn places(hits: &[Hit]) -> Vec<(&str, &str)> {
        hits.iter().map(|h| (h.place.as_str(), h.matched.as_str())).collect()
    }

    #[test]
    fn variables_by_a_wildcard_name_type_and_value() {
        let m = module();
        let v = |name: &str| Search::Variable { name: name.into(), kind: None, value: None };
        let hits = find(&m, v("*sDeidad*"));
        assert_eq!(
            places(&hits),
            [
                ("chest.utp › carries sword.uti", "sDeidad (string) = \"Ezra\""),
                ("keep › creature GUARD_01", "sdeidad (int) = 103"),
                ("sword.uti", "sDeidad (string) = \"Ezra\""),
            ]
        );
        assert_eq!(hits[1].path, "/Creature List[0]/VarTable[0]");
        assert_eq!(hits[1].instance(), Some(("Creature List", 0)));
        assert_eq!(hits[0].carried, Some(rr("sword")));
        // Every variable with Deidad in its name, of the module's areas and
        // items too.
        let all = find(&m, v("*deidad*"));
        assert_eq!(all.len(), 5, "{:?}", places(&all));
        assert!(all.iter().any(|h| h.place == "keep (area)" && h.object == ResType::ARE));
        assert!(
            all.iter().any(|h| h.place == "keep › creature GUARD_01 › item OLD_SWORD"
                && h.object == ResType::UTI)
        );
        // By type and value.
        let int = Search::Variable { name: "*".into(), kind: Some(VarType::Int), value: None };
        assert_eq!(find(&m, int).len(), 2);
        let ezra = Search::Variable { name: "*".into(), kind: None, value: Some("ez*".into()) };
        assert_eq!(find(&m, ezra).len(), 2);
        // Items only, placed or not.
        let items = Query { types: vec![ResType::UTI], ..Query::new(v("*deidad*")) };
        let hits = search(&m, &items, None, Codepage::WINDOWS_1252);
        assert_eq!(hits.len(), 3, "{:?}", places(&hits));
        let placed = Query { placed: Some(true), ..items.clone() };
        assert_eq!(search(&m, &placed, None, Codepage::WINDOWS_1252).len(), 1);
        let blueprints = Query { placed: Some(false), ..items };
        assert_eq!(search(&m, &blueprints, None, Codepage::WINDOWS_1252).len(), 2);
    }

    /// With `regex`, a name or text is a regular expression, found
    /// anywhere unless anchored; one that isn't is said by `check`, and a
    /// search asked again as one is read again (`Kept`).
    #[test]
    fn names_and_texts_by_a_regular_expression() {
        let m = module();
        let regex = |s: Search| Query { regex: true, ..Query::new(s) };
        let v = |name: &str, value: Option<&str>| Search::Variable {
            name: name.into(),
            kind: None,
            value: value.map(str::to_string),
        };
        let all = |q: &Query| search(&m, q, None, Codepage::WINDOWS_1252);
        assert_eq!(all(&regex(v("deidad", None))).len(), 5, "anywhere in the name");
        let exact = all(&Query::new(v("sdeidad", None)));
        assert_eq!(all(&regex(v("^sdeidad$", None))), exact, "case ignored, as a pattern does");
        assert!(all(&regex(v("^deidad", None))).is_empty());
        assert_eq!(all(&regex(v("deidad", Some("^(ezra|103)$")))).len(), 3);
        // As a * pattern, the same text finds nothing.
        assert!(all(&Query::new(v("^sdeidad$", None))).is_empty());
        let text = Search::Field { label: None, value: Some(FieldValue::Text("ez.a".into())) };
        assert!(!all(&regex(text.clone())).is_empty());
        assert!(all(&Query::new(text)).is_empty());
        // Not one: said.
        let bad = regex(v("dei(dad", None));
        assert!(bad.check().unwrap_err().contains("dei(dad"));
        assert!(Query::new(v("dei(dad", None)).check().is_ok());
        // Kept: asked as a pattern, then as an expression, read again.
        let mut kept = Kept::default();
        let q = Query::new(v("^sdeidad$", None));
        assert!(kept.search(&m, &q, None, Codepage::WINDOWS_1252).is_empty());
        let q = Query { regex: true, ..q };
        assert_eq!(kept.search(&m, &q, None, Codepage::WINDOWS_1252), exact);
    }

    #[test]
    fn item_properties_by_type_subtype_and_value() {
        let m = module();
        let p = |subtype, value| Search::ItemProperty { property: 52, subtype, value, param: None };
        let hits = find(&m, p(Some(3), None));
        assert_eq!(
            hits.iter().map(|h| h.place.as_str()).collect::<Vec<_>>(),
            [
                "chest.utp › carries sword.uti",
                "keep › creature GUARD_01 › item OLD_SWORD",
                "sword.uti"
            ]
        );
        assert_eq!(hits[1].path, "/Creature List[0]/ItemList[0]/PropertiesList[0]");
        assert_eq!(hits[2].matched, "Property 52 (subtype 3, value 5)");
        assert_eq!(find(&m, p(Some(3), Some(5))).len(), 2);
        assert!(find(&m, p(Some(4), None)).is_empty());
        let spell = Search::ItemProperty { property: 15, subtype: None, value: None, param: None };
        assert_eq!(places(&find(&m, spell))[0].0, "keep › item GEM");
    }

    #[test]
    fn creatures_by_class_feat_spell_and_skill() {
        let m = module();
        let hits = find(&m, Search::Class(4));
        assert_eq!(
            places(&hits),
            [
                ("guard.utc", "Class: row 4, level 5"),
                ("keep › creature GUARD_01", "Class: row 4, level 5")
            ]
        );
        assert_eq!(hits[0].path, "/ClassList[0]");
        assert!(find(&m, Search::Class(5)).is_empty());
        assert_eq!(find(&m, Search::Feat(0)).len(), 2);
        // A known spell and a special ability.
        assert_eq!(find(&m, Search::Spell(107))[0].matched, "Known spell: row 107");
        assert_eq!(find(&m, Search::Spell(58))[0].matched, "Special ability: row 58");
        // (Cast Spell's subtype 107 is an iprp_spells row, no spell.)
        assert_eq!(find(&m, Search::Spell(107)).len(), 2);
        let skill = |skill, min_rank| Search::Skill { skill, min_rank };
        assert_eq!(find(&m, skill(3, 1))[0].matched, "Skill: row 3, 6 rank(s)");
        assert_eq!(find(&m, skill(3, 6)).len(), 2);
        assert!(find(&m, skill(3, 7)).is_empty());
        // No ranks is no skill.
        assert!(find(&m, skill(0, 0)).is_empty());
    }

    #[test]
    fn appearances_scripts_and_fields() {
        let m = module();
        let a = |of, row| Search::Appearance { of, row };
        assert_eq!(find(&m, a(AppearanceOf::Creature, 6)).len(), 2);
        // Placeable 103 is the chest; a creature's appearance 103 is none.
        assert_eq!(
            places(&find(&m, a(AppearanceOf::Placeable, 103))),
            [("chest.utp", "Appearance: row 103")]
        );
        assert!(find(&m, a(AppearanceOf::Creature, 103)).is_empty());
        assert_eq!(find(&m, a(AppearanceOf::Item, 1)).len(), 4);
        let scripts = find(&m, Search::Script("x2_def_*".into()));
        assert_eq!(
            places(&scripts),
            [
                ("guard.utc", "OnSpawn: x2_def_spawn"),
                ("keep › creature GUARD_01", "OnSpawn: x2_def_spawn")
            ]
        );
        assert_eq!(scripts[1].path, "/Creature List[0]/ScriptSpawn");
        // Every field holding 103: the chest's appearance, a variable, the
        // area's music (its GIT's, no placed object's); not a known spell
        // (107 is not 103).
        let n = Search::Field { label: None, value: Some(FieldValue::Number(103)) };
        let hits = find(&m, n);
        assert_eq!(
            places(&hits),
            [
                ("chest.utp", "Appearance = 103"),
                ("keep › creature GUARD_01", "Value = 103"),
                ("keep (area)", "MusicDay = 103")
            ]
        );
        assert_eq!(hits[1].path, "/Creature List[0]/VarTable[0]/Value");
        assert!(hits[1].placed() && !hits[2].placed());
        assert_eq!((hits[2].object, hits[2].instance()), (ResType::ARE, None));
        // Text, and labels with or without a value.
        let t = Search::Field { label: None, value: Some(FieldValue::Text("ezr".into())) };
        assert_eq!(places(&find(&m, t)), [("sword.uti", "Value = \"Ezra\"")]);
        let l = Search::Field { label: Some("classlev*".into()), value: None };
        assert_eq!(find(&m, l).len(), 2);
        let both = Search::Field {
            label: Some("Tag".into()),
            value: Some(FieldValue::Text("guard".into())),
        };
        assert_eq!(find(&m, both).len(), 2);
        assert!(find(&m, Search::Field { label: None, value: None }).is_empty());
    }

    #[test]
    fn conditions_together_class_levels_and_number_ranges() {
        let m = module();
        let and = |first: Search, also: Vec<Search>| {
            let q = Query { and: also, ..Query::new(first) };
            search(&m, &q, None, Codepage::WINDOWS_1252)
        };
        let var = |name: &str| Search::Variable { name: name.into(), kind: None, value: None };
        // The placed guard has the variable, the guard blueprint hasn't.
        let hits = and(Search::Class(4), vec![var("sdeidad")]);
        assert_eq!(
            places(&hits),
            [("keep › creature GUARD_01", "Class: row 4, level 5 · sdeidad (int) = 103")]
        );
        // Every condition must hold: a feat too, and a skill he hasn't.
        assert_eq!(and(Search::Class(4), vec![var("sdeidad"), Search::Feat(0)]).len(), 1);
        let skill = Search::Skill { skill: 2, min_rank: 1 };
        assert!(and(Search::Class(4), vec![var("sdeidad"), skill]).is_empty());
        // An item held by a fighter: not the blueprint, nor the chest
        // carrying it.
        let bonus = Search::ItemProperty { property: 52, subtype: None, value: None, param: None };
        let held = and(bonus.clone(), vec![Search::Class(4)]);
        assert_eq!(places(&held)[0].0, "keep › creature GUARD_01 › item OLD_SWORD");
        assert_eq!(held.len(), 1);
        assert_eq!(and(bonus, vec![var("nDeidadBonus")]).len(), 1);
        // A creature holding an item with a variable.
        let holding = and(Search::Feat(0), vec![var("nDeidadBonus")]);
        assert_eq!(places(&holding)[0].0, "keep › creature GUARD_01");
        // Two things in one area are not one thing: the gem lying there
        // is no fighter's.
        let gem = Search::ItemProperty { property: 15, subtype: None, value: None, param: None };
        assert!(and(gem, vec![Search::Class(4)]).is_empty());
        // Things in an area with a variable.
        let in_area = and(Search::Class(4), vec![var("AREA_*")]);
        assert_eq!(
            places(&in_area),
            [("keep › creature GUARD_01", "Class: row 4, level 5 · AREA_DEIDAD (float) = 1.5")]
        );

        // A class by level.
        let levels = |min, max| find(&m, Search::ClassLevel { class: 4, min, max });
        assert_eq!(levels(1, 5).len(), 2);
        assert_eq!(levels(5, 5)[0].matched, "Class: row 4, level 5");
        assert!(levels(6, 20).is_empty());
        assert!(find(&m, Search::ClassLevel { class: 5, min: 1, max: 40 }).is_empty());

        // Numbers in a range, floats at their precision.
        assert_eq!(FieldValue::number(" 103 "), Ok(FieldValue::Number(103)));
        let range = |min, max| FieldValue::Range { min, max };
        assert_eq!(FieldValue::number("5..10"), Ok(range(Some(5.0), Some(10.0))));
        assert_eq!(FieldValue::number("..1.5"), Ok(range(None, Some(1.5))));
        assert_eq!(FieldValue::number("1.5"), Ok(range(Some(1.5), Some(1.5))));
        assert!(FieldValue::number("..").is_err());
        assert!(FieldValue::number("x").is_err());
        let value = |text: &str| {
            let v = FieldValue::number(text).unwrap();
            find(&m, Search::Field { label: Some("Value".into()), value: Some(v) })
        };
        assert_eq!(
            places(&value("1.4..2")),
            [
                ("keep (area)", "Value = 1.5"),
                ("keep › creature GUARD_01 › item OLD_SWORD", "Value = 2")
            ]
        );
        assert_eq!(places(&value("1.5")), [("keep (area)", "Value = 1.5")]);
        assert_eq!(value("100..").len(), 1, "the guard's 103");
    }

    /// Conditions any of which is met, and conditions left out: a NOT
    /// leaves out exactly what an AND of it keeps, by the same objects
    /// (the same, one holding it, one held by it).
    #[test]
    fn any_of_the_conditions_and_those_not_met() {
        let m = module();
        let ask = |first: Search, and: Vec<Search>, any: bool, not: Vec<Search>| {
            let q = Query { and, any, not, ..Query::new(first) };
            search(&m, &q, None, Codepage::WINDOWS_1252)
        };
        let and = |first: Search, also: Search| ask(first, vec![also], false, Vec::new());
        let not = |first: Search, out: Search| ask(first, Vec::new(), false, vec![out]);
        let var = |name: &str| Search::Variable { name: name.into(), kind: None, value: None };
        let bonus = Search::ItemProperty { property: 52, subtype: None, value: None, param: None };
        let chest = Search::Appearance { of: AppearanceOf::Placeable, row: 103 };

        // Fighters without the variable: the blueprint, not the placed
        // guard who has it; with a feat they all have, none.
        assert_eq!(
            places(&not(Search::Class(4), var("sdeidad"))),
            [("guard.utc", "Class: row 4, level 5")]
        );
        assert!(not(Search::Class(4), Search::Feat(0)).is_empty());
        // An object holding what the condition finds is left out too (the
        // guard holds an item with the variable), and one held by it (the
        // item a fighter holds), and a blueprint carrying an item found
        // (the chest carries the sword).
        assert_eq!(
            places(&not(Search::Feat(0), var("nDeidadBonus")))[..],
            [("guard.utc", "Feat: row 0")]
        );
        assert_eq!(
            places(&not(bonus.clone(), Search::Class(4))),
            [
                ("chest.utp › carries sword.uti", "Property 52 (subtype 3, value 5)"),
                ("sword.uti", "Property 52 (subtype 3, value 5)"),
            ]
        );
        assert_eq!(and(chest.clone(), bonus.clone()).len(), 1);
        assert!(not(chest, bonus.clone()).is_empty());
        // What a NOT leaves is what its AND doesn't keep.
        for (first, other) in [
            (Search::Class(4), var("*deidad*")),
            (bonus.clone(), Search::Class(4)),
            (var("*"), Search::Feat(0)),
            (Search::Field { label: Some("Tag".into()), value: None }, var("AREA_*")),
        ] {
            let all = find(&m, first.clone()).len();
            let kept = and(first.clone(), other.clone()).len();
            assert_eq!(kept + not(first, other).len(), all);
        }

        // Any of them: each one's, resource by resource.
        let either = ask(Search::Class(4), vec![var("AREA_*")], true, Vec::new());
        assert_eq!(
            places(&either),
            [
                ("guard.utc", "Class: row 4, level 5"),
                ("keep (area)", "AREA_DEIDAD (float) = 1.5"),
                ("keep › creature GUARD_01", "Class: row 4, level 5"),
            ]
        );
        // A place two of them find, once, telling each (a text both find
        // once).
        let both = ask(var("sdeidad"), vec![var("*deidad"), var("x")], true, Vec::new());
        assert_eq!(places(&both), places(&find(&m, var("*deidad"))));
        let typed = Search::Variable { name: "s*".into(), kind: Some(VarType::Int), value: None };
        let both = ask(typed, vec![var("sdeidad")], true, Vec::new());
        let guard = both.iter().find(|h| h.place == "keep › creature GUARD_01").unwrap();
        assert_eq!(guard.matched, "sdeidad (int) = 103", "told once");
        assert_eq!(both.len(), 3, "the sword, the chest carrying it, the guard");
        // Any of them, less what is in an area or on a creature with the
        // variable.
        let some = ask(Search::Class(4), vec![var("AREA_*")], true, vec![var("sdeidad")]);
        assert_eq!(places(&some), [("guard.utc", "Class: row 4, level 5")]);
        // Kept to what is looked at, as ever.
        let q = Query {
            and: vec![var("AREA_*")],
            any: true,
            placed: Some(true),
            ..Query::new(Search::Class(4))
        };
        assert_eq!(search(&m, &q, None, Codepage::WINDOWS_1252).len(), 1);
        // A regular expression in a condition left out is checked too.
        let q = Query { regex: true, not: vec![var("(")], ..Query::new(Search::Class(4)) };
        assert!(q.check().is_err());
    }

    /// An item blueprint found is followed up through bags in bags: a crate
    /// holding a bag that holds the sword lists it, its place said step by
    /// step; a bag holding itself (or a ring of bags) is walked once.
    #[test]
    fn a_found_item_is_followed_through_bags_in_bags() {
        let mut m = module();
        let holding = |what: &[&str]| {
            let held = |r: &&str| {
                let mut s = Struct::new(0);
                s.set("InventoryRes", Value::resref(rr(r)));
                s
            };
            Value::List(what.iter().map(held).collect())
        };
        let mut bag = Gff::new(*b"UTI ");
        bag.root = item("BAG", Vec::new(), Vec::new());
        bag.root.set("ItemList", holding(&["sword", "pouch"]));
        m.set_gff(ResKey::parse("bag", ResType::UTI).unwrap(), &bag).unwrap();
        let mut pouch = Gff::new(*b"UTI ");
        pouch.root = item("POUCH", Vec::new(), Vec::new());
        pouch.root.set("ItemList", holding(&["bag"]));
        m.set_gff(ResKey::parse("pouch", ResType::UTI).unwrap(), &pouch).unwrap();
        let mut crate_ = Gff::new(*b"UTP ");
        crate_.root.set("Tag", Value::String(b"CRATE".to_vec()));
        crate_.root.set("ItemList", holding(&["pouch"]));
        m.set_gff(ResKey::parse("crate", ResType::UTP).unwrap(), &crate_).unwrap();
        let v = Search::Variable { name: "sDeidad".into(), kind: None, value: None };
        let hits = find(&m, v);
        let ezra = "sDeidad (string) = \"Ezra\"";
        assert_eq!(
            places(&hits),
            [
                ("bag.uti › carries sword.uti", ezra),
                ("chest.utp › carries sword.uti", ezra),
                ("crate.utp › carries pouch.uti › carries bag.uti › carries sword.uti", ezra),
                ("keep › creature GUARD_01", "sdeidad (int) = 103"),
                ("pouch.uti › carries bag.uti › carries sword.uti", ezra),
                ("sword.uti", ezra),
            ]
        );
        // What is found is the sword, held by the crate.
        let crate_hit = &hits[2];
        assert_eq!(crate_hit.carried, Some(rr("sword")));
        assert_eq!(crate_hit.path, "/ItemList[0]/InventoryRes", "where the crate names the pouch");
        let objects: Vec<String> = crate_hit.objects().iter().map(|o| o.key.to_string()).collect();
        assert_eq!(objects, ["sword.uti", "crate.utp"]);
    }

    /// A creature blueprint found is followed into the encounters that
    /// spawn it: an encounter blueprint's creatures, and a placed
    /// encounter's (named as placed, its area after it).
    #[test]
    fn a_found_creature_is_followed_into_the_encounters_that_spawn_it() {
        let mut m = Module::new();
        let mut utc = Gff::new(*b"UTC ");
        utc.root = creature("BOSS");
        utc.root.set("VarTable", Value::List(vec![var("MG_BOSS", Value::Int(1))]));
        m.set_gff(ResKey::parse("boss", ResType::UTC).unwrap(), &utc).unwrap();
        let spawning = |what: &str| {
            let mut c = Struct::new(0);
            c.set("ResRef", Value::resref(rr(what)));
            Value::List(vec![c])
        };
        let mut ute = Gff::new(*b"UTE ");
        ute.root.set("Tag", Value::String(b"CAMP".to_vec()));
        ute.root.set("CreatureList", spawning("boss"));
        m.set_gff(ResKey::parse("camp", ResType::UTE).unwrap(), &ute).unwrap();
        let mut other = Gff::new(*b"UTE ");
        other.root.set("CreatureList", spawning("rat"));
        m.set_gff(ResKey::parse("rats", ResType::UTE).unwrap(), &other).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut placed = Struct::new(7);
        placed.set("Tag", Value::String(b"CAMP_01".to_vec()));
        placed.set("CreatureList", spawning("boss"));
        git.root.set("Encounter List", Value::List(vec![placed]));
        m.set_gff(ResKey::parse("keep", ResType::GIT).unwrap(), &git).unwrap();
        let v = Search::Variable { name: "MG_BOSS".into(), kind: None, value: None };
        let hits = find(&m, v);
        let boss = "MG_BOSS (int) = 1";
        assert_eq!(
            places(&hits),
            [
                ("boss.utc", boss),
                ("camp.ute › spawns boss.utc", boss),
                ("keep › encounter CAMP_01 › spawns boss.utc", boss),
            ]
        );
        assert_eq!(hits[1].spawned, Some(rr("boss")));
        assert_eq!(hits[1].object, ResType::UTC);
        let objects = |h: &Hit| {
            h.objects().into_iter().map(|o| format!("{} {}", o.key, o.path)).collect::<Vec<_>>()
        };
        assert_eq!(objects(&hits[1]), ["boss.utc /", "camp.ute /"]);
        assert_eq!(objects(&hits[2]), ["boss.utc /", "keep.git /Encounter List[0]", "keep.are /"]);
        // Not for a search of fields, which finds fields.
        let tag = Search::Field { label: Some("Tag".into()), value: None };
        assert!(find(&m, tag).iter().all(|h| h.spawned.is_none()));
    }

    #[test]
    fn what_holds_what_is_found() {
        let m = module();
        let bonus = Search::ItemProperty { property: 52, subtype: None, value: None, param: None };
        let hits = find(&m, bonus);
        let objects = |h: &Hit| {
            h.objects().into_iter().map(|o| format!("{} {}", o.key, o.path)).collect::<Vec<_>>()
        };
        // Through the chest's inventory: the sword's blueprint, then the
        // chest.
        assert_eq!(objects(&hits[0]), ["sword.uti /", "chest.utp /"]);
        assert_eq!(
            objects(&hits[1]),
            ["keep.git /Creature List[0]/ItemList[0]", "keep.git /Creature List[0]", "keep.are /"]
        );
        assert_eq!(hits[1].object().map(|o| o.kind), Some(ResType::UTI));
        assert!(hits[1].object().unwrap().placed());
        assert_eq!(objects(&hits[2]), ["sword.uti /"]);
        // The area's own, in its GIT or ARE; the module's.
        let music = find(&m, Search::Field { label: Some("MusicDay".into()), value: None });
        assert_eq!(objects(&music[0]), ["keep.are /"]);
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.set("Mod_OnHeartbeat", Value::resref(rr("x2_hb")));
        let mut m = m;
        m.set_gff(ResKey::parse("module", ResType::IFO).unwrap(), &ifo).unwrap();
        let hb = find(&m, Search::Script("x2_hb".into()));
        assert_eq!(objects(&hb[0]), ["module.ifo /"]);
    }

    #[test]
    fn found_things_are_named_as_the_game_names_them() {
        let mut mem = MemContainer::new();
        // A table of `columns` whose rows before the given ones are blank.
        let table = |name: &str, columns: &str, rows: &[(usize, &str)]| {
            let blank = vec!["****"; columns.split_whitespace().count()].join(" ");
            let mut text = format!("2DA V2.0\n\n {columns}\n");
            for n in 0..=rows.iter().map(|r| r.0).max().unwrap_or(0) {
                let row = rows.iter().find(|r| r.0 == n).map_or(blank.as_str(), |r| r.1);
                text.push_str(&format!("{n} {row}\n"));
            }
            (ResKey::parse(name, ResType::TWODA).unwrap(), text.into_bytes())
        };
        let props = "Name Label SubTypeResRef CostTableResRef Param1ResRef GameStrRef";
        for (k, v) in [
            table(
                "classes",
                "Label Name",
                &[
                    (0, "Barbarian ****"),
                    (1, "Bard ****"),
                    (2, "Cleric ****"),
                    (3, "Druid ****"),
                    (4, "Fighter 1"),
                ],
            ),
            table("skills", "Label Name", &[(0, "AnimalEmpathy ****"), (3, "Discipline 2")]),
            table("itempropdef", props, &[(52, "3 SkillBonus IPRP_SKILLS 25 **** 3")]),
            table("iprp_skills", "Name Label", &[(0, "**** AnimalEmpathy"), (3, "2 Discipline")]),
            table("iprp_costtable", "Name Label ClientLoad", &[(25, "IPRP_SKILLCOST SkillCost 0")]),
            table("iprp_skillcost", "Name Label Cost", &[(2, "**** +2 0"), (5, "4 +5 0")]),
        ] {
            mem.insert(k, &v[..]);
        }
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "mem", LayerClass::Key, mem);
        let mut tlk = Tlk::new(Language::ENGLISH);
        for text in ["Bad Strref", "Fighter", "Discipline", "Skill Bonus:", "+5"] {
            tlk.entries.push(TlkEntry::text(text));
        }
        let game = GameData::new(rm, tlk);
        let m = module();
        let named = |s| search(&m, &Query::new(s), Some(&game), Codepage::WINDOWS_1252);
        assert_eq!(named(Search::Class(4))[0].matched, "Class: Fighter, level 5");
        let skill = named(Search::Skill { skill: 3, min_rank: 1 });
        assert_eq!(skill[0].matched, "Skill: Discipline, 6 rank(s)");
        let p = named(Search::ItemProperty {
            property: 52,
            subtype: Some(3),
            value: None,
            param: None,
        });
        assert_eq!(p.last().unwrap().matched, "Skill Bonus: Discipline +5");
        // The choosers' lists, and rows given by name.
        let parts = property_parts(&game, 52, None);
        assert_eq!(parts.subtypes.iter().find(|c| c.row == 3).unwrap().text, "Discipline");
        assert_eq!(parts.values.iter().find(|c| c.row == 5).unwrap().text, "+5");
        assert!(parts.params.is_empty());
        let classes = table_choices(&game, &Search::Class(0).table().unwrap());
        assert_eq!(row_named(&classes, "classes", "fighter"), Ok(4));
        assert_eq!(row_named(&classes, "classes", "7"), Ok(7));
        assert_eq!(row_named(&classes, "classes", "Dru"), Ok(3));
        assert!(
            row_named(&classes, "classes", "B").unwrap_err().contains("Barbarian (0), Bard (1)")
        );
        assert!(row_named(&classes, "classes", "Monk").is_err());
        assert!(row_named(&[], "classes", "Monk").unwrap_err().contains("row number"));
    }

    #[test]
    fn localized_strings_match_in_every_language_and_bad_resources_are_skipped() {
        let mut m = Module::new();
        let mut utw = Gff::new(*b"UTW ");
        let mut name = LocString::from_text(Language::ENGLISH, Gender::Male, "Camp");
        name.strings
            .push((mg_core::LocStringKey::new(Language::GERMAN, Gender::Male), b"Lager".to_vec()));
        utw.root.set("LocalizedName", Value::LocString(name));
        m.set_gff(ResKey::parse("camp", ResType::UTW).unwrap(), &utw).unwrap();
        m.set(ResKey::parse("broken", ResType::UTC).unwrap(), b"UTC V3.2 truncated".to_vec());
        let t = Search::Field { label: None, value: Some(FieldValue::Text("lag".into())) };
        assert_eq!(places(&find(&m, t)), [("camp.utw", "LocalizedName = \"Camp\"")]);
    }
}
