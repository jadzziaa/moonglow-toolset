//! Fields that hold a 2DA row, and the rows' names: for showing a raw
//! field as what it means ("MusicDay 1" is a tune of ambientmusic.2da)
//! rather than as a number alone.
//!
//! Which table a field's number is a row of depends on the kind of object
//! the field belongs to (`Appearance` is placeables.2da for a placeable,
//! doortypes.2da for a door, waypoint.2da for a waypoint), found from the
//! file's type and the lists the field is under; and for an item
//! property's subtype, value and parameter, on its other fields
//! ([`meaning`]). Some numbers name no 2DA's row: a faction (the module's
//! `repute.fac`), a conversation line's animation, a talk-table string
//! held as a plain number, a store's page by its struct's id.

use std::sync::Arc;

use mg_core::{ResType, StrRef};

use crate::{ChoiceColumns, GameData, ObjectKind};

/// The field a door's generic appearance (a genericdoors.2da row, for a
/// door whose `Appearance` is 0) is read from: Enhanced Edition's
/// `GenericType_New` where the door has it, else the older byte
/// `GenericType`. Verify read the older alone, which EE's doors (and
/// Moonglow's own) lack.
pub fn generic_door_field(utd: &mg_gff::Struct) -> &'static str {
    if utd.get("GenericType_New").is_some() { "GenericType_New" } else { "GenericType" }
}

/// A 2DA whose rows a field names, and where a row's name is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowTable {
    pub table: &'static str,
    /// The column holding the row's name as a talk-table string.
    pub name: Option<&'static str>,
    /// The column holding a label (used when there is no name).
    pub label: Option<&'static str>,
}

const fn t(table: &'static str, name: &'static str, label: &'static str) -> RowTable {
    RowTable {
        table,
        name: if name.is_empty() { None } else { Some(name) },
        label: if label.is_empty() { None } else { Some(label) },
    }
}

/// The kind of object a struct describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Area,
    Audio,
    Creature,
    Door,
    Encounter,
    Item,
    Placeable,
    Store,
    Trigger,
    Waypoint,
    Other,
}

impl Kind {
    /// The kind of a placed object's or blueprint's fields. Sounds have no
    /// field that names a row (a store's are the base items it buys or
    /// won't).
    fn of(object: ObjectKind) -> Kind {
        match object {
            ObjectKind::Creature => Kind::Creature,
            ObjectKind::Door => Kind::Door,
            ObjectKind::Encounter => Kind::Encounter,
            ObjectKind::Item => Kind::Item,
            ObjectKind::Placeable => Kind::Placeable,
            ObjectKind::Trigger => Kind::Trigger,
            ObjectKind::Waypoint => Kind::Waypoint,
            ObjectKind::Store => Kind::Store,
            ObjectKind::Sound => Kind::Other,
        }
    }
}

/// The kind a file of this type describes at its root.
fn file_kind(file_type: &str) -> Kind {
    match file_type.trim().to_ascii_lowercase().as_str() {
        "are" => Kind::Area,
        "bic" => Kind::Creature,
        t => ResType::from_extension(t)
            .and_then(ObjectKind::from_restype)
            .map_or(Kind::Other, Kind::of),
    }
}

/// The kind of the structs under a field or list of this label, where that
/// differs from its parent's: a GIT's lists of placed objects and its
/// `AreaProperties`, and the items objects carry.
fn child_kind(label: &str) -> Option<Kind> {
    Some(match label {
        "AreaProperties" => Kind::Audio,
        "ItemList" | "Equip_ItemList" => Kind::Item,
        // Sounds: their parent's (no field of theirs names a row).
        l => match Kind::of(ObjectKind::from_list(l)?) {
            Kind::Other => return None,
            k => k,
        },
    })
}

/// The table `label` names a row of, for a field of a struct reached from
/// the root of a file of type `file_type` (`GIT `, `UTC `…) through the
/// fields and lists `path` (their labels, outermost first). `None`: not a
/// row of a table known here.
pub fn row_table(file_type: &str, path: &[&str], label: &str) -> Option<RowTable> {
    let kind = path.iter().rev().find_map(|l| child_kind(l)).unwrap_or(file_kind(file_type));
    let traps = t("traps", "TrapName", "Label");
    let screens = t("loadscreens", "StrRef", "Label");
    Some(match (kind, label) {
        (Kind::Audio, "AmbientSndDay" | "AmbientSndNight") => {
            t("ambientsound", "Description", "Resource")
        }
        (Kind::Audio, "MusicDay" | "MusicNight" | "MusicBattle") => {
            t("ambientmusic", "Description", "Resource")
        }
        (Kind::Audio, "EnvAudio") => t("soundeax", "Description", "Label"),
        (Kind::Area, "LoadScreenID") => screens,
        (Kind::Area, "PlayerVsPlayer") => t("pvpsettings", "strref", "label"),
        (Kind::Area, "LightingScheme") => t("environment", "STRREF", "LABEL"),
        (Kind::Area, "SkyBox") => t("skyboxes", "STRING_REF", "LABEL"),
        (
            Kind::Area,
            "Tile_MainLight1" | "Tile_MainLight2" | "Tile_SrcLight1" | "Tile_SrcLight2",
        ) => t("lightcolor", "", "LABEL"),
        (Kind::Creature, "Race") => t("racialtypes", "Name", "Label"),
        (Kind::Creature, "Appearance_Type") => t("appearance", "STRING_REF", "LABEL"),
        (Kind::Creature, "Gender") => t("gender", "NAME", "GENDER"),
        (Kind::Creature, "Phenotype") => t("phenotype", "Name", "Label"),
        (Kind::Creature, "PortraitId") => t("portraits", "", "BaseResRef"),
        (Kind::Creature, "SoundSetFile") => t("soundset", "STRREF", "LABEL"),
        (Kind::Creature, "Class") => t("classes", "Name", "Label"),
        (Kind::Creature, "Feat") => t("feat", "FEAT", "LABEL"),
        (Kind::Creature, "Spell") => t("spells", "Name", "Label"),
        (Kind::Creature, "WalkRate") => t("creaturespeed", "Name", "Label"),
        (Kind::Creature, "Wings_New" | "Wings") => t("wingmodel", "", "LABEL"),
        (Kind::Creature, "Tail_New" | "Tail") => t("tailmodel", "", "LABEL"),
        (Kind::Creature, "FamiliarType") => t("hen_familiar", "STRREF", "NAME"),
        (Kind::Creature, "CompanionType") => t("hen_companion", "STRREF", "NAME"),
        (Kind::Creature, "Domain1" | "Domain2") => t("domains", "Name", "Label"),
        (Kind::Creature, "School") => t("spellschools", "StringRef", "Label"),
        (Kind::Creature, "PerceptionRange") => t("ranges", "", "Label"),
        (Kind::Creature, "StartingPackage") => t("packages", "Name", "Label"),
        (Kind::Creature | Kind::Placeable, "BodyBag") => t("bodybag", "Name", "LABEL"),
        (Kind::Item | Kind::Store, "BaseItem") => t("baseitems", "Name", "label"),
        (Kind::Item, "PropertyName") => t("itempropdef", "Name", "Label"),
        (Kind::Placeable, "Appearance") => t("placeables", "StrRef", "Label"),
        (Kind::Door, "Appearance") => t("doortypes", "StringRefGame", "Label"),
        (Kind::Door, "GenericType_New" | "GenericType") => t("genericdoors", "Name", "Label"),
        (Kind::Waypoint, "Appearance") => t("waypoint", "STRREF", "LABEL"),
        (Kind::Door | Kind::Placeable | Kind::Trigger, "TrapType") => traps,
        (Kind::Door | Kind::Trigger, "LoadScreenID") => screens,
        (Kind::Trigger, "Cursor") => t("cursors", "", "Label"),
        (Kind::Encounter, "DifficultyIndex") => t("encdifficulty", "STRREF", "LABEL"),
        _ => return None,
    })
}

/// The table whose rows the items of a list stand for, by their place in
/// it: a creature's `SkillList` has one item per row of skills.2da.
pub fn list_table(file_type: &str, path: &[&str], list: &str) -> Option<RowTable> {
    let kind = path.iter().rev().find_map(|l| child_kind(l)).unwrap_or(file_kind(file_type));
    (kind == Kind::Creature && list == "SkillList").then(|| t("skills", "Name", "Label"))
}

/// A store's pages, by the id of their struct in its `StoreList` (as
/// Aurora writes them, in the order its Inventory shows them).
pub const STORE_PAGES: [(u32, &str); 5] = [
    (0, "Armor"),
    (4, "Weapons"),
    (2, "Potions & Scrolls"),
    (3, "Rings & Amulets"),
    (1, "Miscellaneous"),
];

/// A conversation line's animations (`Animation`), as Aurora's Other
/// Actions page lists them, in its order: (name, value).
pub const DIALOG_ANIMATIONS: [(&str, u32); 19] = [
    ("Default", 0),
    ("No Animation", 88),
    ("Taunt", 28),
    ("Greeting", 29),
    ("Listen", 30),
    ("Worship", 33),
    ("Salute", 34),
    ("Bow", 35),
    ("Steal", 37),
    ("Talk Normal", 38),
    ("Talk Pleading", 39),
    ("Talk Forceful", 40),
    ("Talk Laughing", 41),
    ("Victory 1", 44),
    ("Victory 2", 45),
    ("Victory 3", 46),
    ("Look Far", 48),
    ("Drink", 70),
    ("Read", 71),
];

/// What a field's number stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Meaning {
    /// A row of a table the field's label names ([`row_table`]).
    Row(RowTable),
    /// A row of a table another field names: an item property's subtype
    /// (`Subtype`, of the table its `PropertyName` has), value
    /// (`CostValue`, of the cost table `CostTable` names) and parameter's
    /// value (`Param1Value`, of the table `Param1` names). The table's
    /// name lower case.
    TableOf { table: String, name: Option<&'static str>, label: Option<&'static str> },
    /// A faction of the module: the place of its entry in `repute.fac`'s
    /// `FactionList`.
    Faction,
    /// A conversation line's animation ([`DIALOG_ANIMATIONS`]).
    Animation,
    /// A talk-table string, held as a plain number (a palette's `STRREF`).
    StrRef,
}

impl Meaning {
    /// The table and its name and label columns, for a row of one.
    pub fn table(&self) -> Option<(&str, ChoiceColumns<'static>)> {
        match self {
            Meaning::Row(t) => Some((t.table, ChoiceColumns { name: t.name, label: t.label })),
            Meaning::TableOf { table, name, label } => {
                Some((table, ChoiceColumns { name: *name, label: *label }))
            }
            _ => None,
        }
    }
}

/// Whether a field holds a talk-table string's number as a plain number
/// (not a localized string): a palette's `STRREF` (a category's or a
/// blueprint's name), a GUI layout's caption (`strref`).
pub fn is_strref_field(file_type: &str, label: &str) -> bool {
    match file_type.trim().to_ascii_uppercase().as_str() {
        "ITP" => label == "STRREF",
        "GUI" => label.eq_ignore_ascii_case("strref"),
        _ => false,
    }
}

/// What the number in field `label` stands for, for a field of a struct
/// reached from the root of a file of type `file_type` through the fields
/// and lists `path` (as [`row_table`] takes them); `sibling` gives the
/// struct's other number fields (an item property's subtype table is its
/// type's). The tables an item property's parts are rows of are read from
/// `game` (without it, none). `None`: a plain number, as far as is known.
pub fn meaning(
    game: Option<&GameData>,
    file_type: &str,
    path: &[&str],
    label: &str,
    sibling: &dyn Fn(&str) -> Option<i64>,
) -> Option<Meaning> {
    if let Some(t) = row_table(file_type, path, label) {
        return Some(Meaning::Row(t));
    }
    let kind = path.iter().rev().find_map(|l| child_kind(l)).unwrap_or(file_kind(file_type));
    let named = |table: String, label: &'static str| {
        let table = table.trim().to_lowercase();
        (!table.is_empty() && table != "****").then_some(Meaning::TableOf {
            table,
            name: Some("Name"),
            label: Some(label),
        })
    };
    let property = kind == Kind::Item && path.last() == Some(&"PropertiesList");
    match label {
        "Subtype" if property => {
            let t = game?.property_type(u16::try_from(sibling("PropertyName")?).ok()?)?;
            named(t.subtypes?, "Label")
        }
        "CostTable" if property => Some(Meaning::TableOf {
            table: "iprp_costtable".into(),
            name: None,
            label: Some("Label"),
        }),
        "CostValue" if property => {
            let table = usize::try_from(sibling("CostTable")?).ok().filter(|t| *t != 0)?;
            named(game?.table("iprp_costtable").ok()?.get(table, "Name")?.to_string(), "Label")
        }
        // (iprp_paramtable.2da's label column is spelled so.)
        "Param1" if property && sibling("Param1") != Some(255) => {
            named("iprp_paramtable".into(), "Lable")
        }
        "Param1Value" if property => {
            let table = usize::try_from(sibling("Param1")?).ok().filter(|t| *t != 255)?;
            let name = game?.table("iprp_paramtable").ok()?.get(table, "TableResRef")?.to_string();
            named(name, "Label")
        }
        "FactionID" if kind == Kind::Creature => Some(Meaning::Faction),
        "Faction"
            if matches!(kind, Kind::Door | Kind::Placeable | Kind::Trigger | Kind::Encounter) =>
        {
            Some(Meaning::Faction)
        }
        "Animation"
            if file_type.trim().eq_ignore_ascii_case("DLG")
                && matches!(path.last(), Some(&("EntryList" | "ReplyList"))) =>
        {
            Some(Meaning::Animation)
        }
        l if is_strref_field(file_type, l) => Some(Meaning::StrRef),
        _ => None,
    }
}

/// The name of an item of the list `list` by itself rather than by a
/// field of it: an item of a list with one per row of a table
/// ([`list_table`]) by its place, a store's page by its struct's id.
pub fn list_item_name(
    game: Option<&GameData>,
    file_type: &str,
    path: &[&str],
    list: &str,
    (index, struct_id): (usize, u32),
) -> Option<String> {
    let kind = path.iter().rev().find_map(|l| child_kind(l)).unwrap_or(file_kind(file_type));
    if kind == Kind::Store && list == "StoreList" {
        return STORE_PAGES.iter().find(|p| p.0 == struct_id).map(|p| p.1.to_string());
    }
    game?.row_name(&list_table(file_type, path, list)?, index as i64)
}

/// A faction's or an animation's name, which needs no game data.
pub fn plain_name(meaning: &Meaning, factions: &[String], value: i64) -> Option<String> {
    match meaning {
        Meaning::Faction => factions.get(usize::try_from(value).ok()?).cloned(),
        Meaning::Animation => {
            let value = u32::try_from(value).ok()?;
            DIALOG_ANIMATIONS.iter().find(|a| a.1 == value).map(|a| a.0.to_string())
        }
        _ => None,
    }
}

/// The factions or the animations, to choose from.
pub fn plain_choices(meaning: &Meaning, factions: &[String]) -> Vec<crate::Choice> {
    let choice = |row: usize, text: &str| crate::Choice { row, text: text.to_string() };
    match meaning {
        Meaning::Faction => factions.iter().enumerate().map(|(i, f)| choice(i, f)).collect(),
        Meaning::Animation => {
            DIALOG_ANIMATIONS.iter().map(|(n, v)| choice(*v as usize, n)).collect()
        }
        _ => Vec::new(),
    }
}

impl GameData {
    /// A row's name: its talk-table string, else its label; `None` for a
    /// row the table lacks, or a blank one.
    pub fn row_name(&self, table: &RowTable, row: i64) -> Option<String> {
        self.row_name_in(table.table, ChoiceColumns { name: table.name, label: table.label }, row)
    }

    /// [`row_name`](Self::row_name) of a table given by its name.
    pub fn row_name_in(&self, table: &str, columns: ChoiceColumns<'_>, row: i64) -> Option<String> {
        let row = usize::try_from(row).ok()?;
        let t = self.table(table).ok()?;
        let column = |c: Option<&str>| c.and_then(|c| t.column(c));
        let display = t.column(crate::DISPLAY_NAME);
        self.named_row(&t, row, column(columns.name), display, column(columns.label))
    }

    /// What a field's number `value` stands for, by name: a row's name, a
    /// faction's (of `factions`, the module's, by their places), an
    /// animation's, a talk-table string. `None` where it names nothing
    /// (a row the table lacks, a blank string).
    pub fn meaning_name(
        &self,
        meaning: &Meaning,
        factions: &[String],
        value: i64,
    ) -> Option<String> {
        match meaning {
            Meaning::StrRef => self.string(StrRef(u32::try_from(value).ok()?)),
            Meaning::Faction | Meaning::Animation => plain_name(meaning, factions, value),
            m => {
                let (table, columns) = m.table()?;
                self.row_name_in(table, columns, value)
            }
        }
    }

    /// What a field of this meaning may be chosen from, by name (a talk
    /// table's strings are too many to list: none).
    pub fn meaning_choices(
        &self,
        meaning: &Meaning,
        factions: &[String],
    ) -> Arc<Vec<crate::Choice>> {
        match meaning {
            Meaning::StrRef => Arc::default(),
            Meaning::Faction | Meaning::Animation => Arc::new(plain_choices(meaning, factions)),
            m => m
                .table()
                .and_then(|(table, columns)| self.choice_list(table, columns).ok())
                .unwrap_or_default(),
        }
    }

    /// A row's name by the columns of its table: the talk-table string the
    /// `name` column gives ([`cell_string`](Self::cell_string)), else
    /// a name written out in `display` (ambientmusic.2da's
    /// `DisplayName`, for music a hak adds), else the `label`. A blank
    /// or `****` cell is none. What a field's row is called and what the
    /// lists of a table's rows offer ([`choice_list`](Self::choice_list)).
    pub(crate) fn named_row(
        &self,
        t: &mg_2da::TwoDa,
        row: usize,
        name: Option<usize>,
        display: Option<usize>,
        label: Option<usize>,
    ) -> Option<String> {
        let cell = |column: Option<usize>| {
            t.cell(row, column?).map(str::trim).filter(|v| !v.is_empty() && *v != "****")
        };
        name.and_then(|c| self.cell_string(t, row, c))
            .or_else(|| cell(display).map(str::to_string))
            .or_else(|| cell(label).map(str::to_string))
    }

    /// The talk-table string a table's cell names by its StrRef, if the
    /// cell holds one (a whole number, not negative) and the string has
    /// text.
    pub fn cell_string(&self, t: &mg_2da::TwoDa, row: usize, column: usize) -> Option<String> {
        let strref = t.cell(row, column).and_then(mg_2da::parse_int)?;
        self.string(StrRef(u32::try_from(strref).ok()?)).filter(|s| !s.trim().is_empty())
    }

    /// The row a text names, as a command line takes one for a field
    /// that holds a row (`MusicDay=mus_cave`): a number is the row itself;
    /// else the row whose label is the text (case set aside), else the one
    /// whose name ([`row_name`](Self::row_name)) is. Two rows of that
    /// name, or none, are an error that says so.
    pub fn row_of(&self, table: &RowTable, text: &str) -> Result<i64, String> {
        let text = text.trim();
        if let Some(n) = mg_2da::parse_int(text) {
            return Ok(i64::from(n));
        }
        let t = self.table(table.table).map_err(|e| format!("{}.2da: {e}", table.table))?;
        let label = |row: usize| table.label.and_then(|c| t.get(row, c)).map(str::trim);
        if let Some(row) =
            (0..t.len()).find(|&r| label(r).is_some_and(|l| l.eq_ignore_ascii_case(text)))
        {
            return Ok(row as i64);
        }
        let named: Vec<usize> = (0..t.len())
            .filter(|&r| {
                self.row_name(table, r as i64).is_some_and(|n| n.eq_ignore_ascii_case(text))
            })
            .collect();
        match named[..] {
            [row] => Ok(row as i64),
            [] => Err(format!("{}.2da has no row named {text:?}", table.table)),
            ref rows => Err(format!(
                "{}.2da has {} rows named {text:?} ({}): give its number or its label",
                table.table,
                rows.len(),
                rows.iter().take(8).map(ToString::to_string).collect::<Vec<_>>().join(", ")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{Language, ResType};
    use mg_resman::{LayerClass, MemContainer, ResKey, ResMan, priority};
    use mg_tlk::{Tlk, TlkEntry};

    use super::*;

    #[test]
    fn a_row_is_named_by_its_string_then_its_label() {
        let mut mem = MemContainer::new();
        mem.insert(
            ResKey::parse("ambientmusic", ResType::TWODA).unwrap(),
            &b"2DA V2.0\n\n  Description Resource DisplayName\n0 **** **** ****\n1 1 mus_ruralday1 ****\n2 99 mus_cave ****\n3 **** **** ****\n4 **** mus_mine \"My Theme\"\n"[..],
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "mem", LayerClass::Key, mem);
        let mut tlk = Tlk::new(Language::ENGLISH);
        tlk.entries.push(TlkEntry::text("Bad Strref"));
        tlk.entries.push(TlkEntry::text("Rural Day 1"));
        let game = GameData::new(rm, tlk);
        let music = row_table("GIT ", &["AreaProperties"], "MusicDay").unwrap();
        assert_eq!(game.row_name(&music, 1).as_deref(), Some("Rural Day 1"));
        // A string the talk table lacks: the label.
        assert_eq!(game.row_name(&music, 2).as_deref(), Some("mus_cave"));
        // No talk-table string, but a name written out (custom music).
        assert_eq!(game.row_name(&music, 4).as_deref(), Some("My Theme"));
        let listed = game
            .choices(
                "ambientmusic",
                crate::ChoiceColumns { name: Some("Description"), label: Some("Resource") },
            )
            .unwrap();
        assert_eq!(listed.iter().find(|c| c.row == 4).map(|c| c.text.as_str()), Some("My Theme"));
        // A blank row, one past the table, a negative number: no name.
        for row in [0, 3, 40, -1] {
            assert_eq!(game.row_name(&music, row), None, "row {row}");
        }
        // A table the game lacks.
        let feat = row_table("UTC ", &["FeatList"], "Feat").unwrap();
        assert_eq!(game.row_name(&feat, 1), None);
        // Back from a text: a number, a label, a name.
        assert_eq!(game.row_of(&music, "4"), Ok(4));
        assert_eq!(game.row_of(&music, "MUS_CAVE"), Ok(2));
        assert_eq!(game.row_of(&music, "rural day 1"), Ok(1));
        assert_eq!(game.row_of(&music, "My Theme"), Ok(4));
        assert!(game.row_of(&music, "Jazz").unwrap_err().contains("no row named \"Jazz\""));
        assert!(game.row_of(&feat, "Cleave").unwrap_err().starts_with("feat.2da"));
    }

    /// A field's row and a list of the table's rows name each row alike:
    /// a blank string, a negative StrRef and a `****` label are none.
    #[test]
    fn a_row_s_name_and_its_choice_are_one() {
        let mut mem = MemContainer::new();
        mem.insert(
            ResKey::parse("things", ResType::TWODA).unwrap(),
            &b"2DA V2.0\n\n  Name Label\n0 1 Sword\n1 2 Blank\n2 -1 Minus\n3 **** ****\n4 x Axe\n5 **** \"\"\n"[..],
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "mem", LayerClass::Key, mem);
        let mut tlk = Tlk::new(Language::ENGLISH);
        tlk.entries.push(TlkEntry::text("Bad Strref"));
        tlk.entries.push(TlkEntry::text("Longsword"));
        tlk.entries.push(TlkEntry::text("   "));
        let game = GameData::new(rm, tlk);
        let things = RowTable { table: "things", name: Some("Name"), label: Some("Label") };
        let named: Vec<(usize, String)> =
            (0..7).filter_map(|r| Some((r, game.row_name(&things, r as i64)?))).collect();
        let want = [(0, "Longsword"), (1, "Blank"), (2, "Minus"), (4, "Axe")];
        assert_eq!(named, want.map(|(r, t)| (r, t.to_string())));
        let listed = game
            .choices("things", crate::ChoiceColumns { name: Some("Name"), label: Some("Label") })
            .unwrap();
        let listed: Vec<(usize, String)> = listed.iter().map(|c| (c.row, c.text.clone())).collect();
        assert_eq!(listed, named);
        let t = game.table("things").unwrap();
        assert_eq!(game.cell_string(&t, 0, 0).as_deref(), Some("Longsword"));
        assert_eq!(game.cell_string(&t, 1, 0), None, "a blank string");
        assert_eq!(game.cell_string(&t, 2, 0), None, "a negative StrRef");
    }

    #[test]
    fn a_fields_table_depends_on_what_it_belongs_to() {
        let table = |file, path: &[&str], label| row_table(file, path, label).map(|t| t.table);
        // An area's audio, in its GIT.
        assert_eq!(table("GIT ", &["AreaProperties"], "MusicDay"), Some("ambientmusic"));
        assert_eq!(table("GIT ", &["AreaProperties"], "AmbientSndNight"), Some("ambientsound"));
        assert_eq!(table("GIT ", &["AreaProperties"], "MusicDelay"), None);
        // Appearance: by the kind of object.
        assert_eq!(table("UTP ", &[], "Appearance"), Some("placeables"));
        assert_eq!(table("UTD ", &[], "Appearance"), Some("doortypes"));
        assert_eq!(table("GIT ", &["WaypointList"], "Appearance"), Some("waypoint"));
        assert_eq!(table("GIT ", &["Placeable List"], "Appearance"), Some("placeables"));
        assert_eq!(table("UTS ", &[], "Appearance"), None);
        assert_eq!(table("GIT ", &["SoundList"], "Appearance"), None);
        assert_eq!(table("GIT ", &["StoreList", "ItemList"], "BaseItem"), Some("baseitems"));
        assert_eq!(table("BIC ", &[], "Race"), Some("racialtypes"));
        assert_eq!(table("GIT ", &["List"], "BaseItem"), Some("baseitems"));
        // A creature's lists; the items it carries are items.
        assert_eq!(table("UTC ", &["FeatList"], "Feat"), Some("feat"));
        assert_eq!(table("utc", &["ClassList", "KnownList0"], "Spell"), Some("spells"));
        assert_eq!(table("GIT ", &["Creature List", "ClassList"], "Class"), Some("classes"));
        assert_eq!(table("UTC ", &["Equip_ItemList"], "BaseItem"), Some("baseitems"));
        assert_eq!(table("UTC ", &["Equip_ItemList"], "Race"), None);
        assert_eq!(table("UTI ", &["PropertiesList"], "PropertyName"), Some("itempropdef"));
        assert_eq!(list_table("UTC ", &[], "SkillList").map(|t| t.table), Some("skills"));
        assert_eq!(list_table("UTI ", &[], "SkillList"), None);
        // What a store buys and won't.
        assert_eq!(table("UTM ", &["WillNotBuy"], "BaseItem"), Some("baseitems"));
        assert_eq!(table("GIT ", &["StoreList", "WillOnlyBuy"], "BaseItem"), Some("baseitems"));
    }

    /// Numbers that name no fixed table's row: an item property's
    /// subtype, value and parameter (of the tables its other fields name),
    /// a faction, a conversation line's animation, a palette's talk-table
    /// string, a store's page.
    #[test]
    fn a_field_s_meaning_can_depend_on_the_struct_s_other_fields() {
        let tables: [(&str, &str); 6] = [
            (
                "itempropdef",
                "Name Label SubTypeResRef CostTableResRef Param1ResRef\n\
                 0 1 Ability iprp_abilities 1 ****\n1 **** Light **** 18 5\n",
            ),
            ("iprp_abilities", "Name Label\n0 2 Str\n1 **** Dex\n"),
            ("iprp_costtable", "Name Label ClientLoad\n0 **** Base 0\n1 IPRP_BONUSCOST Bonus 0\n"),
            ("iprp_bonuscost", "Name Label Cost\n0 **** zero 0\n1 **** one 1\n2 3 two 2\n"),
            (
                "iprp_paramtable",
                "Name Lable TableResRef\n0 **** Damage iprp_dmg\n1 4 Color iprp_color\n",
            ),
            ("iprp_color", "Name Label\n0 **** Blue\n1 5 Yellow\n"),
        ];
        let mut mem = MemContainer::new();
        for (name, body) in tables {
            let text = format!("2DA V2.0\n\n{body}");
            mem.insert(ResKey::parse(name, ResType::TWODA).unwrap(), text.as_bytes());
        }
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "mem", LayerClass::Key, mem);
        let mut tlk = Tlk::new(Language::ENGLISH);
        for text in ["Bad Strref", "Ability Bonus", "Strength", "+2", "Light Color", "Yellow"] {
            tlk.entries.push(TlkEntry::text(text));
        }
        let game = GameData::new(rm, tlk);
        let property = |values: &[(&str, i64)]| {
            let values: Vec<(String, i64)> =
                values.iter().map(|(l, v)| (l.to_string(), *v)).collect();
            move |l: &str| values.iter().find(|(k, _)| k == l).map(|(_, v)| *v)
        };
        let ability = property(&[
            ("PropertyName", 0),
            ("Subtype", 0),
            ("CostTable", 1),
            ("CostValue", 2),
            ("Param1", 255),
        ]);
        let at = ["PropertiesList"];
        let named = |label: &str, sibling: &dyn Fn(&str) -> Option<i64>| {
            let m = meaning(Some(&game), "UTI ", &at, label, sibling)?;
            game.meaning_name(&m, &[], sibling(label)?)
        };
        assert_eq!(named("PropertyName", &ability).as_deref(), Some("Ability Bonus"));
        assert_eq!(named("Subtype", &ability).as_deref(), Some("Strength"));
        assert_eq!(named("CostTable", &ability).as_deref(), Some("Bonus"));
        assert_eq!(named("CostValue", &ability).as_deref(), Some("+2"));
        assert_eq!(meaning(Some(&game), "UTI ", &at, "Param1", &ability), None, "255: none");
        assert_eq!(meaning(Some(&game), "UTI ", &at, "Param1Value", &ability), None);
        // A parameter: of the table iprp_paramtable.2da names (its label
        // column spelled "Lable").
        let light =
            property(&[("PropertyName", 1), ("CostTable", 0), ("Param1", 1), ("Param1Value", 1)]);
        assert_eq!(named("Param1", &light).as_deref(), Some("Light Color"));
        assert_eq!(named("Param1Value", &light).as_deref(), Some("Yellow"));
        assert_eq!(meaning(Some(&game), "UTI ", &at, "CostValue", &light), None, "cost table 0");
        assert_eq!(meaning(Some(&game), "UTI ", &at, "Subtype", &light), None, "no subtypes");
        // A row the table lacks: no name; without the game, no table.
        let past = property(&[("PropertyName", 0), ("Subtype", 7)]);
        assert_eq!(named("Subtype", &past), None);
        assert_eq!(meaning(None, "UTI ", &at, "Subtype", &ability), None);
        // Not an item property's: none.
        assert_eq!(meaning(Some(&game), "UTI ", &[], "Subtype", &ability), None);
        // An item a store or creature carries has them too.
        let carried = ["Equip_ItemList", "PropertiesList"];
        assert!(meaning(Some(&game), "UTC ", &carried, "Subtype", &ability).is_some());

        let none = |_: &str| None;
        let factions = ["PC".to_string(), "Hostile".to_string()];
        let m = |file: &str, path: &[&str], label: &str| meaning(None, file, path, label, &none);
        assert_eq!(m("UTC ", &[], "FactionID"), Some(Meaning::Faction));
        assert_eq!(m("GIT ", &["Door List"], "Faction"), Some(Meaning::Faction));
        assert_eq!(m("UTE ", &[], "Faction"), Some(Meaning::Faction));
        assert_eq!(m("UTC ", &[], "Faction"), None, "a creature's is FactionID");
        assert_eq!(plain_name(&Meaning::Faction, &factions, 1).as_deref(), Some("Hostile"));
        assert_eq!(plain_name(&Meaning::Faction, &factions, 2), None);
        assert_eq!(m("DLG ", &["EntryList"], "Animation"), Some(Meaning::Animation));
        assert_eq!(m("DLG ", &["ReplyList"], "Animation"), Some(Meaning::Animation));
        assert_eq!(m("DLG ", &[], "Animation"), None);
        assert_eq!(plain_name(&Meaning::Animation, &[], 28).as_deref(), Some("Taunt"));
        assert_eq!(plain_choices(&Meaning::Animation, &[]).len(), DIALOG_ANIMATIONS.len());
        assert_eq!(m("ITP ", &["MAIN", "LIST"], "STRREF"), Some(Meaning::StrRef));
        assert_eq!(m("UTC ", &[], "STRREF"), None);
        assert_eq!(game.meaning_name(&Meaning::StrRef, &[], 2).as_deref(), Some("Strength"));
        // A store's pages, by their structs' ids.
        let page = |id| list_item_name(None, "UTM ", &[], "StoreList", (0, id));
        assert_eq!(page(4).as_deref(), Some("Weapons"));
        assert_eq!(page(9), None);
        assert_eq!(
            list_item_name(None, "GIT ", &["StoreList"], "StoreList", (1, 1)).as_deref(),
            Some("Miscellaneous")
        );
        assert_eq!(list_item_name(None, "UTC ", &[], "StoreList", (0, 0)), None);
    }
}
