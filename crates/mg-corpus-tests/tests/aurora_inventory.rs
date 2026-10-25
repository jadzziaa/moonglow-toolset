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
