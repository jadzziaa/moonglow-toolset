//! The nine types of object placed in an area: their blueprint type, the
//! GIT list they are placed in (its label and struct id, from the schema
//! observed in the game's files) and their names. Every list of them in
//! the toolset goes through [`ObjectKind`]: the area editor's (as
//! `mg_area::ObjectKind`), the palettes' (`mg_module::palette::BlueprintKind`),
//! verify's categories, references, search and the bulk edits.

use mg_core::ResType;
use mg_resman::ResKey;
use mg_schema::{ListField, git};

/// A type of object placed in an area, and of blueprint with a palette.
/// In the order of [`ObjectKind::ALL`] (the GIT's lists in alphabetical
/// order; the area editor's filters are stored by it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectKind {
    Creature,
    Door,
    Encounter,
    Item,
    Placeable,
    Sound,
    Store,
    Trigger,
    Waypoint,
}

impl ObjectKind {
    pub const ALL: [ObjectKind; 9] = [
        ObjectKind::Creature,
        ObjectKind::Door,
        ObjectKind::Encounter,
        ObjectKind::Item,
        ObjectKind::Placeable,
        ObjectKind::Sound,
        ObjectKind::Store,
        ObjectKind::Trigger,
        ObjectKind::Waypoint,
    ];

    /// Its position in [`ObjectKind::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Its blueprint type.
    pub const fn restype(self) -> ResType {
        match self {
            ObjectKind::Creature => ResType::UTC,
            ObjectKind::Door => ResType::UTD,
            ObjectKind::Encounter => ResType::UTE,
            ObjectKind::Item => ResType::UTI,
            ObjectKind::Placeable => ResType::UTP,
            ObjectKind::Sound => ResType::UTS,
            ObjectKind::Store => ResType::UTM,
            ObjectKind::Trigger => ResType::UTT,
            ObjectKind::Waypoint => ResType::UTW,
        }
    }

    /// Its blueprint type alone, as a list (`[UTC]`; for tables of the
    /// types that go together).
    pub const fn restypes(self) -> &'static [ResType] {
        static TYPES: [ResType; 9] = {
            let mut types = [ResType::UTC; 9];
            let mut i = 0;
            while i < types.len() {
                types[i] = ObjectKind::ALL[i].restype();
                i += 1;
            }
            types
        };
        std::slice::from_ref(&TYPES[self.index()])
    }

    /// The kind placed from a blueprint type.
    pub fn from_restype(t: ResType) -> Option<ObjectKind> {
        ObjectKind::ALL.into_iter().find(|k| k.restype() == t)
    }

    /// The GIT list its instances are in.
    pub const fn list_field(self) -> ListField {
        match self {
            ObjectKind::Creature => git::CREATURE_LIST,
            ObjectKind::Door => git::DOOR_LIST,
            ObjectKind::Encounter => git::ENCOUNTER_LIST,
            ObjectKind::Item => git::LIST,
            ObjectKind::Placeable => git::PLACEABLE_LIST,
            ObjectKind::Sound => git::SOUND_LIST,
            ObjectKind::Store => git::STORE_LIST,
            ObjectKind::Trigger => git::TRIGGER_LIST,
            ObjectKind::Waypoint => git::WAYPOINT_LIST,
        }
    }

    /// The label of the GIT list its instances are in (`Creature List`,
    /// `SoundList`; items lying in an area are in `List`).
    pub const fn list(self) -> &'static str {
        self.list_field().label
    }

    /// The struct id of its instances in their GIT list.
    pub const fn struct_id(self) -> u32 {
        self.list_field().item_id
    }

    /// The kind a GIT list holds (`None`: not one of the GIT's lists of
    /// placed objects).
    pub fn from_list(list: &str) -> Option<ObjectKind> {
        ObjectKind::ALL.into_iter().find(|k| k.list() == list)
    }

    /// What it is called in one word, lowercase: `creature`, `store`. As
    /// in the palette file names (`<name>pal.itp`), and placed objects'
    /// descriptions (`keep › creature GUARD`).
    pub const fn name(self) -> &'static str {
        match self {
            ObjectKind::Creature => "creature",
            ObjectKind::Door => "door",
            ObjectKind::Encounter => "encounter",
            ObjectKind::Item => "item",
            ObjectKind::Placeable => "placeable",
            ObjectKind::Sound => "sound",
            ObjectKind::Store => "store",
            ObjectKind::Trigger => "trigger",
            ObjectKind::Waypoint => "waypoint",
        }
    }

    /// What the toolset calls them, in the plural: the palettes' tabs, the
    /// area view's filters, the module tree, Verify. Stores are `Stores`
    /// everywhere, where Aurora says Merchants on its area filters and
    /// palette buttons and Stores elsewhere.
    pub const fn label(self) -> &'static str {
        match self {
            ObjectKind::Creature => "Creatures",
            ObjectKind::Door => "Doors",
            ObjectKind::Encounter => "Encounters",
            ObjectKind::Item => "Items",
            ObjectKind::Placeable => "Placeables",
            ObjectKind::Sound => "Sounds",
            ObjectKind::Store => "Stores",
            ObjectKind::Trigger => "Triggers",
            ObjectKind::Waypoint => "Waypoints",
        }
    }

    /// The field that holds a blueprint's category (stores call it `ID`).
    pub fn palette_field(self) -> &'static str {
        if self == ObjectKind::Store { "ID" } else { "PaletteID" }
    }

    /// The field that holds a blueprint's own resref, and names a placed
    /// object's blueprint (stores: `ResRef`).
    pub fn resref_field(self) -> &'static str {
        if self == ObjectKind::Store { "ResRef" } else { "TemplateResRef" }
    }

    /// [`ObjectKind::resref_field`] of a blueprint type; `TemplateResRef`
    /// for the other types.
    pub fn resref_field_of(t: ResType) -> &'static str {
        ObjectKind::from_restype(t).map_or("TemplateResRef", ObjectKind::resref_field)
    }

    /// The field that holds a blueprint's name (creatures: first name).
    pub fn name_field(self) -> &'static str {
        match self {
            ObjectKind::Creature => "FirstName",
            ObjectKind::Encounter
            | ObjectKind::Item
            | ObjectKind::Trigger
            | ObjectKind::Waypoint => "LocalizedName",
            _ => "LocName",
        }
    }

    /// The module's custom palette resource.
    pub fn custom_key(self) -> ResKey {
        ResKey::parse(&format!("{}palcus", self.name()), ResType::ITP).expect("valid")
    }

    /// The palette skeleton (`<type>pal.itp`): the categories blueprints
    /// of the type go in. The game has one; a module (or a hak) may have
    /// its own, with categories of its own.
    pub fn skeleton_key(self) -> ResKey {
        ResKey::parse(&format!("{}pal", self.name()), ResType::ITP).expect("valid")
    }

    /// Whether it is placed by `X`, `Y`, `Z` and `Bearing` (doors and
    /// placeables) rather than by position and orientation fields.
    pub fn has_bearing(self) -> bool {
        matches!(self, ObjectKind::Door | ObjectKind::Placeable)
    }

    /// Whether its model is the toolset's marker for it, not something
    /// the game shows: a waypoint's flag, a store's sign, a sound's
    /// speaker.
    pub fn is_marker(self) -> bool {
        matches!(self, ObjectKind::Waypoint | ObjectKind::Store | ObjectKind::Sound)
    }

    /// Whether it is drawn as an arrow along its facing when it has no
    /// marker model (or, a store, when arrows are preferred).
    pub fn has_arrow(self) -> bool {
        matches!(self, ObjectKind::Waypoint | ObjectKind::Store)
    }

    /// Whether it has an outline (`Geometry`) around its position.
    pub fn has_outline(self) -> bool {
        matches!(self, ObjectKind::Trigger | ObjectKind::Encounter)
    }

    /// The field with the faction (a `repute.fac` row) it belongs to, for
    /// the kinds that belong to one.
    pub fn faction_field(self) -> Option<&'static str> {
        match self {
            ObjectKind::Creature => Some("FactionID"),
            ObjectKind::Door
            | ObjectKind::Encounter
            | ObjectKind::Placeable
            | ObjectKind::Trigger => Some("Faction"),
            _ => None,
        }
    }
}

/// Faction 0, the PC faction: player characters' alone, not to be given
/// an object (Jasperre's report). In the game a creature of it is put in
/// faction 1, Hostile, as it loads; a placeable, door or trigger keeps it,
/// and creatures regard it as they regard players (`engine_factions.rs`).
pub const PC_FACTION: i64 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_is_found_again_by_its_type_and_its_list() {
        for (i, k) in ObjectKind::ALL.into_iter().enumerate() {
            assert_eq!(k.index(), i);
            assert_eq!(ObjectKind::from_restype(k.restype()), Some(k));
            assert_eq!(k.restypes(), [k.restype()]);
            assert_eq!(ObjectKind::from_list(k.list()), Some(k));
        }
        // Inventories and other files' lists are not the GIT's.
        for list in ["ItemList", "Equip_ItemList", "CreatureList", "object", ""] {
            assert_eq!(ObjectKind::from_list(list), None, "{list}");
        }
        assert_eq!(ObjectKind::from_restype(ResType::GIT), None);
    }

    #[test]
    fn the_lists_and_struct_ids_are_the_games() {
        let lists: Vec<_> = ObjectKind::ALL.map(|k| (k.list(), k.struct_id())).into();
        assert_eq!(
            lists,
            [
                ("Creature List", 4),
                ("Door List", 8),
                ("Encounter List", 7),
                ("List", 0),
                ("Placeable List", 9),
                ("SoundList", 6),
                ("StoreList", 11),
                ("TriggerList", 1),
                ("WaypointList", 5),
            ]
        );
        // The order of ALL is the lists' alphabetical order.
        let mut sorted = lists.clone();
        sorted.sort();
        assert_eq!(lists, sorted);
    }

    #[test]
    fn a_store_names_its_blueprint_and_category_otherwise() {
        for k in ObjectKind::ALL {
            let store = k == ObjectKind::Store;
            assert_eq!(k.resref_field(), if store { "ResRef" } else { "TemplateResRef" });
            assert_eq!(k.palette_field(), if store { "ID" } else { "PaletteID" });
            assert_eq!(ObjectKind::resref_field_of(k.restype()), k.resref_field());
        }
        assert_eq!(ObjectKind::resref_field_of(ResType::GIT), "TemplateResRef");
        assert_eq!(ObjectKind::Store.label(), "Stores");
        assert_eq!(ObjectKind::Store.name(), "store");
        assert_eq!(ObjectKind::Sound.skeleton_key().to_string(), "soundpal.itp");
        assert_eq!(ObjectKind::Item.custom_key().to_string(), "itempalcus.itp");
    }
}
