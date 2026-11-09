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
/// (`"GIT"`, `"IFO"`...), for a list that is in no item of another (an
/// area's placed objects): [`list_rule_in`] for one that is.
pub fn list_rule(file_type: &str, label: &str) -> ListRule {
    list_rule_in(file_type, None, label)
}

/// [`list_rule`] for a list in an item of the list `inside` (`None`: in
/// none). An area's lists of placed objects are those of the file's own
/// struct: a placed store's `StoreList` (its panes) has the label of the
/// area's list of stores, and is a store's.
pub fn list_rule_in(file_type: &str, inside: Option<&str>, label: &str) -> ListRule {
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
        ("GIT", l) if inside.is_none() && PLACED.contains(&l) => ListRule::Placed,
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
    mg_gff::merge::merge(base, ours, theirs, prefer, &list_rule_in)
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

    /// A store placed in an area: its blueprint in `ResRef`, its panes in
    /// a `StoreList` of its own, pane `id` holding `items`.
    fn placed_store(x: f32, markup: i32, pane: u32, items: &[(&str, u16)]) -> Struct {
        let mut s = Struct::new(11);
        s.set("ResRef", Value::ResRef("nw_storgenral001".into()));
        s.set("Tag", Value::String("STORE".into()));
        s.set("XPosition", Value::Float(x));
        s.set("YPosition", Value::Float(4.0));
        s.set("ZPosition", Value::Float(0.0));
        s.set("MarkUp", Value::Int(markup));
        let panes = (0..5u32).map(|id| {
            let mut p = Struct::new(id);
            let held: Vec<Struct> = (items.iter().enumerate())
                .filter(|_| id == pane)
                .map(|(n, (res, at))| {
                    let mut i = Struct::new(n as u32);
                    i.set("TemplateResRef", Value::ResRef((*res).into()));
                    i.set("Repos_PosX", Value::Word(*at));
                    i.set("Repos_Posy", Value::Word(0));
                    i
                })
                .collect();
            p.set("ItemList", Value::List(held));
            p
        });
        s.set("StoreList", Value::List(panes.collect()));
        s
    }

    fn area(stores: Vec<Struct>) -> Gff {
        let mut g = Gff::new(*b"GIT ");
        g.root.set("StoreList", Value::List(stores));
        g
    }

    /// Each side adds an item to one pane of a placed store: the pane is
    /// one, with both items (the panes took the placed objects' rule, by
    /// the label they share with the area's list of stores, and came out
    /// twice).
    #[test]
    fn a_placed_store_s_panes_merge_by_pane() {
        let (torch, sword, bow) = (("nw_it_torch001", 0), ("nw_wswls001", 2), ("nw_wbwsh001", 4));
        let b = area(vec![placed_store(10.0, 100, 4, &[torch])]);
        let o = area(vec![placed_store(10.0, 100, 4, &[torch, sword])]);
        let t = area(vec![placed_store(10.0, 100, 4, &[torch, bow])]);
        let m = merge(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        let stores = m.gff.root.list("StoreList").unwrap();
        assert_eq!(stores.len(), 1);
        let panes = stores[0].list("StoreList").unwrap();
        assert_eq!(panes.iter().map(|p| p.id).collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
        let held: Vec<_> = (panes[4].list("ItemList").unwrap().iter())
            .map(|i| i.resref("TemplateResRef").unwrap().to_string())
            .collect();
        assert_eq!(held, ["nw_it_torch001", "nw_wswls001", "nw_wbwsh001"]);
    }

    /// One side moves a placed store and the other changes its markup: it
    /// is one store, moved and marked up (its blueprint is in `ResRef`, so
    /// no step of the match knew it and it came out twice).
    #[test]
    fn a_placed_store_moved_and_changed_is_one_store() {
        let b = area(vec![placed_store(10.0, 100, 4, &[])]);
        let o = area(vec![placed_store(25.0, 100, 4, &[])]);
        let t = area(vec![placed_store(10.0, 150, 4, &[])]);
        let m = merge(&b, &o, &t, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(m.gff, area(vec![placed_store(25.0, 150, 4, &[])]));
    }

    #[test]
    fn lists_are_told_apart_by_what_their_items_are() {
        // An area's stores are placed objects, a placed store's panes its
        // own, by their struct ids.
        assert_eq!(list_rule("GIT", "StoreList"), ListRule::Placed);
        assert_eq!(list_rule_in("GIT", Some("StoreList"), "StoreList"), ListRule::ByStructId);
        assert_eq!(
            list_rule_in("GIT", Some("Creature List"), "FeatList"),
            ListRule::ByKey(&["Feat"])
        );
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
