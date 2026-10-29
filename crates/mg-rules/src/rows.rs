//! Fields that hold a 2DA row, and the rows' names: for showing a raw
//! field as what it means ("MusicDay 1" is a tune of ambientmusic.2da)
//! rather than as a number alone.
//!
//! Which table a field's number is a row of depends on the kind of object
//! the field belongs to (`Appearance` is placeables.2da for a placeable,
//! doortypes.2da for a door, waypoint.2da for a waypoint), found from the
//! file's type and the lists the field is under.

use mg_core::{ResType, StrRef};

use crate::{GameData, ObjectKind};

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
    Trigger,
    Waypoint,
    Other,
}

impl Kind {
    /// The kind of a placed object's or blueprint's fields. Sounds and
    /// stores have no field that names a row.
    fn of(object: ObjectKind) -> Kind {
        match object {
            ObjectKind::Creature => Kind::Creature,
            ObjectKind::Door => Kind::Door,
            ObjectKind::Encounter => Kind::Encounter,
            ObjectKind::Item => Kind::Item,
            ObjectKind::Placeable => Kind::Placeable,
            ObjectKind::Trigger => Kind::Trigger,
            ObjectKind::Waypoint => Kind::Waypoint,
            ObjectKind::Sound | ObjectKind::Store => Kind::Other,
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
        // Sounds and stores: their parent's (no field of theirs names a
        // row).
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
        (Kind::Item, "BaseItem") => t("baseitems", "Name", "label"),
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

impl GameData {
    /// A row's name: its talk-table string, else its label; `None` for a
    /// row the table lacks, or a blank one.
    pub fn row_name(&self, table: &RowTable, row: i64) -> Option<String> {
        let row = usize::try_from(row).ok()?;
        let t = self.table(table.table).ok()?;
        let column = |c: Option<&str>| c.and_then(|c| t.column(c));
        let display = t.column(crate::DISPLAY_NAME);
        self.named_row(&t, row, column(table.name), display, column(table.label))
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
    }
}
