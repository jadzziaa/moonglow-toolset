//! Advanced search: the module's blueprints, placed objects, areas,
//! conversations and the rest, found by what they hold rather than by
//! their tag or name: local variables, item properties, creatures'
//! classes (by level), feats, spells and skills, appearances, the scripts
//! attached, or any field by its label and value (a number in a range).
//! Several conditions together find the objects that meet them all.
//!
//! Every GFF resource of the module is walked, lists included (items in
//! placed creatures' and containers' inventories and equipment, store
//! pages, conversation lines). A blueprint's inventory names its items'
//! blueprints rather than holding them, so an item blueprint that matches
//! is also found where a creature, placeable, store or container
//! blueprint carries it.

use std::collections::HashMap;

use mg_core::{Codepage, ResRef, ResType};
use mg_gff::{FieldAt, Gff, Struct, Value};
use mg_resman::ResKey;
use mg_rules::rows::{RowTable, list_table, row_table};
use mg_rules::{Choice, ChoiceColumns, GameData, ItemProperty, ObjectKind};

use crate::Module;
use crate::query::glob;
use crate::refs::{RefKind, gff_references};
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
    /// is found by each of these.
    pub and: Vec<Search>,
    /// The kinds of object looked at (`utc`, `uti`...; `are` for areas,
    /// `ifo` for the module, `dlg` for conversations); empty: all.
    pub types: Vec<ResType>,
    /// Only placed objects (`Some(true)`), only blueprints (`Some(false)`).
    pub placed: Option<bool>,
}

impl Query {
    pub fn new(search: Search) -> Query {
        Query { search, and: Vec::new(), types: Vec::new(), placed: None }
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

fn integer(v: &Value) -> Option<i64> {
    Some(match *v {
        Value::Byte(x) => x.into(),
        Value::Char(x) => x.into(),
        Value::Word(x) => x.into(),
        Value::Short(x) => x.into(),
        Value::Dword(x) => x.into(),
        Value::Int(x) => x.into(),
        Value::Dword64(x) => i64::try_from(x).ok()?,
        Value::Int64(x) => x,
        _ => return None,
    })
}

/// Whether a number field holds a number in a range: a float compared
/// at its own precision (the ends made floats too).
fn in_range(v: &Value, min: Option<f64>, max: Option<f64>) -> bool {
    let (n, single) = match *v {
        Value::Float(x) => (f64::from(x), true),
        Value::Double(x) => (x, false),
        ref v => match integer(v) {
            Some(n) => (n as f64, false),
            None => return false,
        },
    };
    let end = |e: f64| if single { f64::from(e as f32) } else { e };
    min.is_none_or(|m| n >= end(m)) && max.is_none_or(|m| n <= end(m))
}

/// Whether `text` contains `pattern`, ignoring case, `*` matching any run.
fn contains(pattern: &str, text: &str) -> bool {
    glob(&format!("*{pattern}*"), text)
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
) -> Option<(String, String)> {
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
            let n = integer(f.value)?;
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
            if f.list != Some("ClassList") || f.label != "Class" || integer(f.value) != Some(*class)
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
            let rank = integer(f.value)?;
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
                Some(FieldValue::Number(n)) => integer(f.value) == Some(*n),
                Some(FieldValue::Text(t)) => texts(f.value, cp).iter().any(|s| contains(t, s)),
                Some(FieldValue::Range { min, max }) => in_range(f.value, *min, *max),
            };
            if !hit || (label.is_none() && value.is_none()) {
                return None;
            }
            Some((f.path.to_string(), format!("{} = {}", f.label, shown(f.value, cp))))
        }
    }
}

/// What one resource holds that a search matches, in the order of its
/// fields.
fn resource_hits(
    key: ResKey,
    gff: &Gff,
    search: &Search,
    game: Option<&GameData>,
    cp: Codepage,
) -> Vec<Hit> {
    let file_type = gff.file_type_str();
    let mut found: Vec<(String, String)> = Vec::new();
    if let Search::Script(pattern) = search {
        for r in gff_references(key, gff) {
            if r.kind == RefKind::Script && glob(pattern, &r.target.to_string()) {
                let label = r.path.rsplit('/').next().unwrap_or_default();
                let script = r.target.to_lowercase();
                found.push((r.path.clone(), format!("{}: {script}", event(label))));
            }
        }
    } else {
        gff.root.walk_fields(&mut |f| {
            if let Some(hit) = field_hit(search, &file_type, f, game, cp) {
                found.push(hit);
            }
        });
    }
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
        });
    }
    out
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

/// Everything in the module the query matches: resource by resource (in
/// the order of their names and types), field by field; an item
/// blueprint found is also found in the inventories of the blueprints
/// that carry it (but not for a search of fields, which finds fields).
/// With other conditions, what is found on an object that meets them
/// all, each one's first match told after its own (`Class: Fighter,
/// level 5 · Feat: Alertness`). `game` names what is found as the game
/// does (else by numbers); `cp` reads the module's text.
pub fn search(m: &Module, q: &Query, game: Option<&GameData>, cp: Codepage) -> Vec<Hit> {
    let mut hits = search_one(m, &q.search, game, cp);
    for extra in &q.and {
        // The objects it finds things on, and those holding them.
        let theirs = search_one(m, extra, game, cp);
        let mut own: HashMap<Object, &str> = HashMap::new();
        let mut held: HashMap<Object, &str> = HashMap::new();
        for h in &theirs {
            let objects = h.objects();
            if let Some(o) = objects.first() {
                own.entry(o.clone()).or_insert(&h.matched);
            }
            for o in objects {
                held.entry(o).or_insert(&h.matched);
            }
        }
        // The same object, one holding what it is found on, or one held
        // by it.
        hits.retain_mut(|h| {
            let objects = h.objects();
            let first = objects.first().and_then(|o| held.get(o)).copied();
            let also = first.or_else(|| objects.iter().find_map(|o| own.get(o).copied()));
            if let Some(text) = also {
                h.matched = format!("{} · {text}", h.matched);
            }
            also.is_some()
        });
    }
    hits.retain(|h| wanted(q, h));
    hits
}

/// What one search finds, before the query keeps what it looks at.
fn search_one(m: &Module, s: &Search, game: Option<&GameData>, cp: Codepage) -> Vec<Hit> {
    let mut keys: Vec<ResKey> = m.keys().filter(|k| k.restype.is_gff()).copied().collect();
    keys.sort();
    let mut by_key: Vec<(ResKey, Vec<Hit>)> = Vec::new();
    // The item blueprints the blueprints' inventories name: where.
    let mut carries: Vec<(ResKey, String, ResRef, String)> = Vec::new();
    let carrying = !matches!(s, Search::Field { .. });
    for key in keys {
        let Some(Ok(gff)) = m.gff(&key) else { continue };
        let hits = resource_hits(key, &gff, s, game, cp);
        let blueprint = ObjectKind::from_restype(key.restype).is_some();
        if carrying && blueprint {
            for r in gff_references(key, &gff) {
                if r.kind == RefKind::Blueprint(ResType::UTI) {
                    let place = format!("{key} › carries {}.uti", r.target.to_lowercase());
                    carries.push((key, r.path, r.target, place));
                }
            }
        }
        by_key.push((key, hits));
    }
    // The item blueprints found, and what they matched.
    let items: Vec<(ResRef, Vec<String>)> = by_key
        .iter()
        .filter(|(k, h)| k.restype == ResType::UTI && !h.is_empty())
        .map(|(k, h)| {
            let direct = h.iter().filter(|h| h.carried.is_none() && h.object == ResType::UTI);
            (k.resref, direct.map(|h| h.matched.clone()).collect())
        })
        .collect();
    for (key, path, target, place) in carries {
        let Some((_, matched)) = items.iter().find(|(r, _)| *r == target) else { continue };
        let Some((_, hits)) = by_key.iter_mut().find(|(k, _)| *k == key) else { continue };
        for m in matched {
            hits.push(Hit {
                key,
                path: path.clone(),
                place: place.clone(),
                matched: m.clone(),
                object: ResType::UTI,
                carried: Some(target),
            });
        }
    }
    by_key.into_iter().flat_map(|(_, h)| h).collect()
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
