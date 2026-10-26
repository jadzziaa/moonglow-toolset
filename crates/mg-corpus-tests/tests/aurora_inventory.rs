//! Equipping what needs a feat, against Aurora: two featless Fighter 1
//! bandits (`examples/spell_warning_probe_module.rs inventory`, captured as
//! `inventory/before.mod`), given a longsword and banded mail in Aurora's
//! inventory and Yes to its question (`after.mod`). Aurora listed
//! Weapon Proficiency (martial) and (Elf) for the longsword and Armor
//! Proficiency (heavy) for the banded mail (AC 6), and added the first.

use mg_core::ResType;
use mg_gff::Struct;
use mg_module::Module;
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

fn creature(m: &Module, tag: &str) -> Struct {
    let git = m.gff(&ResKey::parse("field", ResType::GIT).unwrap()).unwrap().unwrap();
    let list = git.root.list("Creature List").unwrap();
    list.iter().find(|c| c.string("Tag") == Some(tag.as_bytes())).unwrap().clone()
}

fn feats(c: &Struct) -> Vec<u16> {
    c.list("FeatList")
        .unwrap_or(&[])
        .iter()
        .filter_map(|f| f.integer("Feat"))
        .map(|f| f as u16)
        .collect()
}

#[test]
fn equipping_asks_for_aurora_s_feats() {
    let root = corpus!();
    let before = Module::open(&aurora_capture!("inventory/before.mod")).unwrap();
    let after = Module::open(&aurora_capture!("inventory/after.mod")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    for (tag, slot, listed) in [("SPW0", 0x10, vec![45, 256]), ("SPW1", 0x2, vec![2])] {
        let (was, now) = (creature(&before, tag), creature(&after, tag));
        assert!(feats(&was).is_empty());
        let item = now.list("Equip_ItemList").unwrap().iter().find(|s| s.id == slot).unwrap();
        assert_eq!(game.missing_feats(item, &feats(&was)), listed, "{tag}");
        assert_eq!(feats(&now), [listed[0]], "{tag}: the first added");
        assert!(game.missing_feats(item, &feats(&now)).is_empty());
    }
}

/// An item Aurora equips has no `Dropable` or `Pickpocketable` (neither
/// dropped nor stolen: what the engine reads when they are left out), and
/// neither has the item Moonglow equips on a placed creature
/// (`instances::held`). The capture equips items only; the backpack's are
/// written the same way in the game's own modules (set: a byte 1; not set:
/// left out).
#[test]
fn an_item_aurora_equips_is_neither_dropable_nor_pickpocketable() {
    let root = corpus!();
    let after = Module::open(&aurora_capture!("inventory/after.mod")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let blueprint = |r: mg_core::ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        Some(mg_gff::Gff::read(&data).ok()?.root)
    };
    let placing = mg_module::instances::Placing { game: &game, item: &blueprint };
    for (tag, slot) in [("SPW0", 0x10), ("SPW1", 0x2)] {
        let now = creature(&after, tag);
        let item = now.list("Equip_ItemList").unwrap().iter().find(|s| s.id == slot).unwrap();
        for flag in ["Dropable", "Pickpocketable"] {
            assert!(item.get(flag).is_none(), "{tag}: Aurora wrote {flag}");
        }
        let r = item.resref("TemplateResRef").unwrap();
        let ours = mg_module::instances::held(&placing, &blueprint(r).unwrap(), slot);
        for flag in ["Dropable", "Pickpocketable"] {
            assert_eq!(ours.get(flag), item.get(flag), "{tag}: {flag}");
        }
    }
}

/// A placed creature's own item moved in Aurora's inventory
/// (`inventory-moves/before.mod`: `inventory/before.mod` with an amulet of
/// SPW0's own in its backpack, tagged `MG_MOVED_EQ` and named "Moved
/// Probe", and a ring equipped on SPW1, `MG_MOVED_BP`, "Stowed Probe",
/// beside a necklace in its backpack; each dropable, pickpocketable and
/// with an INT field the game doesn't know, `MG_Probe`. `after.mod`: the
/// amulet dragged onto its slot, the ring into the backpack). Aurora
/// moves the item itself: its own name, tag, flags and the rest are kept;
/// only where it is held changes (its struct id, the slot or its place in
/// the list; `Repos_PosX`/`Repos_Posy`, only in the backpack, the first
/// free place). Moonglow's moves make what Aurora wrote, but for what
/// Aurora does to every item it writes: it adds `xModelPart1` and drops
/// fields it doesn't know (which Moonglow keeps).
#[test]
fn an_item_aurora_moves_is_kept_as_it_is() {
    use mg_module::instances::{equipped_item, listed_item};
    let before = Module::open(&aurora_capture!("inventory-moves/before.mod")).unwrap();
    let after = Module::open(&aurora_capture!("inventory-moves/after.mod")).unwrap();
    let find = |m: &Module, who: &str, list: &str, tag: &[u8]| {
        let c = creature(m, who);
        let items = c.list(list).unwrap_or(&[]);
        items.iter().find(|s| s.string("Tag") == Some(tag)).cloned()
    };
    // (Without what Aurora adds to or drops from every item it writes, its
    // fields in one order.)
    let aurora = |s: &Struct| {
        let mut s = s.clone();
        s.remove("xModelPart1");
        s.remove("MG_Probe");
        s.fields.sort_by_key(|f| f.label.to_string_lossy());
        s
    };
    let was = find(&before, "SPW0", "ItemList", b"MG_MOVED_EQ").unwrap();
    assert!(find(&after, "SPW0", "ItemList", b"MG_MOVED_EQ").is_none());
    let worn = find(&after, "SPW0", "Equip_ItemList", b"MG_MOVED_EQ").unwrap();
    assert_eq!(worn.id, 0x200, "the amulet's slot");
    assert_eq!(aurora(&equipped_item(&was, 0x200)), aurora(&worn));
    let was = find(&before, "SPW1", "Equip_ItemList", b"MG_MOVED_BP").unwrap();
    let packed = find(&after, "SPW1", "ItemList", b"MG_MOVED_BP").unwrap();
    let at = |l: &str| packed.integer(l).unwrap() as u16;
    // (At the backpack's end, beside the necklace at 0, 0.)
    assert_eq!((packed.id, at("Repos_PosX"), at("Repos_Posy")), (1, 1, 0));
    assert_eq!(aurora(&listed_item(&was, 1, 1, 0)), aurora(&packed));
}

/// An item dropped on a slot that is taken, in Aurora's inventory
/// (`inventory-slots/before.mod`: `inventory/before.mod` with items of
/// the bandits' own; `after-a.mod`: SPW2's backpack amulet dragged onto
/// its worn one, an amulet of the palette onto SPW3's, and SPW4's left
/// ring onto its right ring's slot). Aurora asks nothing: the slot takes
/// the item dropped, and what it held goes to the backpack as it is, at
/// the list's end and the first free place (where a backpack item left,
/// that place). A ring moved to the other ring's slot sends that ring to
/// the backpack; the two are not swapped.
#[test]
fn an_item_aurora_drops_on_a_taken_slot_sends_its_item_to_the_backpack() {
    use mg_module::instances::listed_item;
    let before = Module::open(&aurora_capture!("inventory-slots/before.mod")).unwrap();
    let after = Module::open(&aurora_capture!("inventory-slots/after-a.mod")).unwrap();
    let find = |m: &Module, who: &str, list: &str, tag: &[u8]| {
        let c = creature(m, who);
        let items = c.list(list).unwrap_or(&[]);
        items.iter().find(|s| s.string("Tag") == Some(tag)).cloned()
    };
    // (Without what Aurora adds to every item it writes, its fields in one
    // order.)
    let aurora = |s: &Struct| {
        let mut s = s.clone();
        s.remove("xModelPart1");
        s.fields.sort_by_key(|f| f.label.to_string_lossy());
        s
    };
    for (who, worn, slot, dropped) in [
        ("SPW2", &b"MG_OCC_NECK"[..], 0x200, Some(&b"MG_NEW_NECK"[..])),
        ("SPW3", b"MG_OCC_PAL", 0x200, None),
        ("SPW4", b"MG_RING_R", 0x100, Some(b"MG_RING_L")),
    ] {
        let was = find(&before, who, "Equip_ItemList", worn).unwrap();
        assert_eq!(was.id, slot);
        assert!(find(&after, who, "Equip_ItemList", worn).is_none(), "{who}: off");
        let packed = find(&after, who, "ItemList", worn).unwrap();
        assert_eq!(creature(&after, who).list("ItemList").unwrap().len(), 1, "{who}");
        assert_eq!(aurora(&listed_item(&was, 0, 0, 0)), aurora(&packed), "{who}: as it was");
        let now = creature(&after, who);
        let on = now.list("Equip_ItemList").unwrap().iter().find(|s| s.id == slot).unwrap();
        match dropped {
            Some(tag) => assert_eq!(on.string("Tag"), Some(tag), "{who}"),
            None => assert_ne!(on.string("Tag"), Some(worn), "{who}: the palette's"),
        }
    }
}
