//! What an item of each of the game's lists is, for merging two copies of
//! a resource made from a third ([`mg_gff::merge`]): `mg merge-gff`, the
//! git merge driver.
//!
//! | List | An item is |
//! | --- | --- |
//! | an area's placed objects (`.git`: creatures, doors, placeables, waypoints, triggers, encounters, sounds, stores, items) | the object, matched in steps ([`ListRule::Placed`]) |
//! | `VarTable`, anywhere | the variable: its `Name` and `Type` (an int and a string of one name are two) |
//! | `Mod_Area_list` | the area (`Area_Name`) |
//! | `Mod_HakList` | the hak (`Mod_Hak`) |
//! | `Mod_CacheNSSList` | the script (`ResRef`) |
//! | `FeatList` | the feat (`Feat`) |
//! | `ClassList` | the class (`Class`) |
//! | `KnownList0`–`9`, `MemorizedList0`–`9` | the spell (`Spell`; a spell memorized twice, in order) |
//! | `Equip_ItemList` | the slot (the struct id) |
//! | a journal's `Categories` | the quest (`Tag`), and its `EntryList` the entry (`ID`) |
//! | a faction file's `RepList` | the pair of factions (`FactionID1`, `FactionID2`) |
//! | an inventory's `ItemList` (a creature's, a placeable's, a store pane's) | the item and its place in the grid (`InventoryRes`, in an area's file `TemplateResRef`, and `Repos_PosX`, `Repos_Posy`) |
//! | a store's `StoreList` | the pane (the struct id) |
//! | an encounter's `CreatureList` | the creature (`ResRef`; one listed twice, in order) |
//! | an item's `PropertiesList` | the property and its subtype (`PropertyName`, `Subtype`) |
//! | a creature's `SpecAbilityList` | the ability (`Spell`; one listed twice, in order) |
//! | any other list (tiles, skills, conversation nodes) | its place in the list |

use mg_gff::Gff;
pub use mg_gff::merge::{Conflict, ListRule, Merged, Prefer};

use crate::{fac, git, ifo, jrl, utc, ute, uti, utm};

/// What an item of the list `label` is, in a file of type `file_type`
/// (`"GIT"`, `"IFO"`...).
pub fn list_rule(file_type: &str, label: &str) -> ListRule {
    const PLACED: [&str; 9] = [
        git::CREATURE_LIST.label,
        git::DOOR_LIST.label,
        git::PLACEABLE_LIST.label,
        git::WAYPOINT_LIST.label,
        git::TRIGGER_LIST.label,
        git::ENCOUNTER_LIST.label,
        git::SOUND_LIST.label,
        git::STORE_LIST.label,
        git::LIST.label,
    ];
    let spells = |prefix: &str| {
        label.strip_prefix(prefix).is_some_and(|n| n.len() == 1 && n.as_bytes()[0].is_ascii_digit())
    };
    match (file_type, label) {
        ("GIT", l) if PLACED.contains(&l) => ListRule::Placed,
        (_, l) if l == git::VAR_TABLE.label => ListRule::ByKey(&["Name", "Type"]),
        ("IFO", l) if l == ifo::MOD_AREA_LIST.label => ListRule::ByKey(&["Area_Name"]),
        ("IFO", l) if l == ifo::MOD_HAK_LIST.label => ListRule::ByKey(&["Mod_Hak"]),
        ("IFO", l) if l == ifo::MOD_CACHE_NSS_LIST.label => ListRule::ByKey(&["ResRef"]),
        ("JRL", l) if l == jrl::CATEGORIES.label => ListRule::ByKey(&["Tag"]),
        ("JRL", l) if l == jrl::categories::ENTRY_LIST.label => ListRule::ByKey(&["ID"]),
        ("FAC", l) if l == fac::REP_LIST.label => ListRule::ByKey(&["FactionID1", "FactionID2"]),
        (_, l) if l == utc::FEAT_LIST.label => ListRule::ByKey(&["Feat"]),
        (_, l) if l == utc::CLASS_LIST.label => ListRule::ByKey(&["Class"]),
        (_, l) if l == utc::EQUIP_ITEM_LIST.label => ListRule::ByStructId,
        ("GIT", l) if l == utc::ITEM_LIST.label => {
            ListRule::ByKey(&["TemplateResRef", "Repos_PosX", "Repos_Posy"])
        }
        (_, l) if l == utc::ITEM_LIST.label => {
            ListRule::ByKey(&["InventoryRes", "Repos_PosX", "Repos_Posy"])
        }
        (_, l) if l == utm::STORE_LIST.label => ListRule::ByStructId,
        (_, l) if l == ute::CREATURE_LIST.label => ListRule::ByKey(&["ResRef"]),
        (_, l) if l == uti::PROPERTIES_LIST.label => ListRule::ByKey(&["PropertyName", "Subtype"]),
        (_, l) if l == utc::SPEC_ABILITY_LIST.label => ListRule::ByKey(&["Spell"]),
        _ if spells("KnownList") || spells("MemorizedList") => ListRule::ByKey(&["Spell"]),
        _ => ListRule::ByIndex,
    }
}

/// Merges `ours` and `theirs`, both made from `base`, each list by what
/// its items are ([`list_rule`]).
pub fn merge(base: &Gff, ours: &Gff, theirs: &Gff, prefer: Prefer) -> Merged {
    mg_gff::merge::merge(base, ours, theirs, prefer, &list_rule)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_gff::{Struct, Value};

    /// Two builders each putting an item into one creature's inventory,
    /// in different places, both are kept; one's item property changed and
    /// the other's added merge too.
    #[test]
    fn inventories_and_item_properties_merge_by_item() {
        let item = |res: &str, x: u16| {
            let mut s = Struct::new(x.into());
            s.set("InventoryRes", Value::ResRef(res.into()));
            s.set("Repos_PosX", Value::Word(x));
            s.set("Repos_Posy", Value::Word(0));
            s
        };
        let utc = |items: Vec<Struct>| {
            let mut g = Gff::new(*b"UTC ");
            g.root.set("ItemList", Value::List(items));
            g
        };
        let b = utc(vec![item("nw_it_mpotion001", 0)]);
        let o = utc(vec![item("nw_it_mpotion001", 0), item("nw_wswls001", 2)]);
        let t = utc(vec![item("nw_it_mpotion001", 0), item("nw_aarcl001", 4)]);
        let m = merge(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        let kept: Vec<_> = m.gff.root.list("ItemList").unwrap().iter().map(|s| s.id).collect();
        assert_eq!(kept, [0, 2, 4]);
        // The same item put in one place by both is one.
        let m = merge(&b, &o, &o, Prefer::Ours);
        assert_eq!(m.gff, o);

        let property = |name: u16, sub: u16, cost: u8| {
            let mut s = Struct::new(0);
            s.set("PropertyName", Value::Word(name));
            s.set("Subtype", Value::Word(sub));
            s.set("CostValue", Value::Byte(cost));
            s
        };
        let uti = |props: Vec<Struct>| {
            let mut g = Gff::new(*b"UTI ");
            g.root.set("PropertiesList", Value::List(props));
            g
        };
        let b = uti(vec![property(6, 0, 1)]);
        let o = uti(vec![property(6, 0, 2)]);
        let t = uti(vec![property(6, 0, 1), property(16, 4, 1)]);
        let m = merge(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(m.gff, uti(vec![property(6, 0, 2), property(16, 4, 1)]));
    }

    #[test]
    fn lists_are_told_apart_by_what_their_items_are() {
        assert_eq!(list_rule("GIT", "Placeable List"), ListRule::Placed);
        assert_eq!(list_rule("GIT", "List"), ListRule::Placed);
        // A placed creature's own lists are a creature's.
        assert_eq!(list_rule("GIT", "FeatList"), ListRule::ByKey(&["Feat"]));
        assert_eq!(
            list_rule("GIT", "ItemList"),
            ListRule::ByKey(&["TemplateResRef", "Repos_PosX", "Repos_Posy"])
        );
        assert_eq!(
            list_rule("UTC", "ItemList"),
            ListRule::ByKey(&["InventoryRes", "Repos_PosX", "Repos_Posy"])
        );
        assert_eq!(list_rule("UTM", "StoreList"), ListRule::ByStructId);
        assert_eq!(list_rule("UTE", "CreatureList"), ListRule::ByKey(&["ResRef"]));
        assert_eq!(
            list_rule("UTI", "PropertiesList"),
            ListRule::ByKey(&["PropertyName", "Subtype"])
        );
        assert_eq!(list_rule("UTC", "SpecAbilityList"), ListRule::ByKey(&["Spell"]));
        assert_eq!(list_rule("UTC", "List"), ListRule::ByIndex);
        assert_eq!(list_rule("ARE", "VarTable"), ListRule::ByKey(&["Name", "Type"]));
        assert_eq!(list_rule("IFO", "Mod_Area_list"), ListRule::ByKey(&["Area_Name"]));
        assert_eq!(list_rule("UTC", "MemorizedList3"), ListRule::ByKey(&["Spell"]));
        assert_eq!(list_rule("UTC", "KnownList10"), ListRule::ByIndex);
        assert_eq!(list_rule("UTC", "Equip_ItemList"), ListRule::ByStructId);
        assert_eq!(list_rule("JRL", "EntryList"), ListRule::ByKey(&["ID"]));
        assert_eq!(list_rule("DLG", "EntryList"), ListRule::ByIndex);
        assert_eq!(list_rule("ARE", "Tile_List"), ListRule::ByIndex);
    }
}
