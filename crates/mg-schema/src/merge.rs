//! What an item of each of the game's lists is, for merging two copies of
//! a resource made from a third ([`mg_gff::merge`]): `mg merge-gff`, the
//! git merge driver.
//!
//! | List | An item is |
//! | --- | --- |
//! | an area's placed objects (`.git`: creatures, doors, placeables, waypoints, triggers, encounters, sounds, stores, items) | the object, matched in steps ([`ListRule::Placed`]) |
//! | `VarTable`, anywhere | the variable's `Name` |
//! | `Mod_Area_list` | the area (`Area_Name`) |
//! | `Mod_HakList` | the hak (`Mod_Hak`) |
//! | `Mod_CacheNSSList` | the script (`ResRef`) |
//! | `FeatList` | the feat (`Feat`) |
//! | `ClassList` | the class (`Class`) |
//! | `KnownList0`–`9`, `MemorizedList0`–`9` | the spell (`Spell`; a spell memorized twice, in order) |
//! | `Equip_ItemList` | the slot (the struct id) |
//! | a journal's `Categories` | the quest (`Tag`), and its `EntryList` the entry (`ID`) |
//! | a faction file's `RepList` | the pair of factions (`FactionID1`, `FactionID2`) |
//! | any other list (tiles, skills, inventories, conversation nodes, item properties) | its place in the list |

use mg_gff::Gff;
pub use mg_gff::merge::{Conflict, ListRule, Merged, Prefer};

use crate::{fac, git, ifo, jrl, utc};

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
        (_, l) if l == git::VAR_TABLE.label => ListRule::ByKey(&["Name"]),
        ("IFO", l) if l == ifo::MOD_AREA_LIST.label => ListRule::ByKey(&["Area_Name"]),
        ("IFO", l) if l == ifo::MOD_HAK_LIST.label => ListRule::ByKey(&["Mod_Hak"]),
        ("IFO", l) if l == ifo::MOD_CACHE_NSS_LIST.label => ListRule::ByKey(&["ResRef"]),
        ("JRL", l) if l == jrl::CATEGORIES.label => ListRule::ByKey(&["Tag"]),
        ("JRL", l) if l == jrl::categories::ENTRY_LIST.label => ListRule::ByKey(&["ID"]),
        ("FAC", l) if l == fac::REP_LIST.label => ListRule::ByKey(&["FactionID1", "FactionID2"]),
        (_, l) if l == utc::FEAT_LIST.label => ListRule::ByKey(&["Feat"]),
        (_, l) if l == utc::CLASS_LIST.label => ListRule::ByKey(&["Class"]),
        (_, l) if l == utc::EQUIP_ITEM_LIST.label => ListRule::ByStructId,
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

    #[test]
    fn lists_are_told_apart_by_what_their_items_are() {
        assert_eq!(list_rule("GIT", "Placeable List"), ListRule::Placed);
        assert_eq!(list_rule("GIT", "List"), ListRule::Placed);
        // A placed creature's own lists are a creature's.
        assert_eq!(list_rule("GIT", "FeatList"), ListRule::ByKey(&["Feat"]));
        assert_eq!(list_rule("GIT", "ItemList"), ListRule::ByIndex);
        assert_eq!(list_rule("UTC", "List"), ListRule::ByIndex);
        assert_eq!(list_rule("ARE", "VarTable"), ListRule::ByKey(&["Name"]));
        assert_eq!(list_rule("IFO", "Mod_Area_list"), ListRule::ByKey(&["Area_Name"]));
        assert_eq!(list_rule("UTC", "MemorizedList3"), ListRule::ByKey(&["Spell"]));
        assert_eq!(list_rule("UTC", "KnownList10"), ListRule::ByIndex);
        assert_eq!(list_rule("UTC", "Equip_ItemList"), ListRule::ByStructId);
        assert_eq!(list_rule("JRL", "EntryList"), ListRule::ByKey(&["ID"]));
        assert_eq!(list_rule("DLG", "EntryList"), ListRule::ByIndex);
        assert_eq!(list_rule("ARE", "Tile_List"), ListRule::ByIndex);
    }
}
