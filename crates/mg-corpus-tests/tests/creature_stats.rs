//! Creature statistics against the game's creatures: the maximum hit points
//! Moonglow derives equal the ones the toolset stored in the base-game
//! blueprints (a few dozen of the 1,582 were stored before their
//! blueprint's abilities or feats last changed, and differ).

use mg_core::ResType;
use mg_gff::Gff;
use mg_resman::{GameInstall, ResKey};
use mg_rules::{CreatureSheet, GameData};
use mg_testkit::corpus;

#[test]
fn max_hit_points_match_the_stored_ones() {
    let root = corpus!();
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let (mut ok, mut bad) = (0, Vec::new());
    for r in gd.resman.list(ResType::UTC) {
        let Ok(d) = gd.resman.get(&ResKey::new(r, ResType::UTC)) else { continue };
        let Ok(g) = Gff::read(&d) else { continue };
        let stats = gd.creature_stats(&CreatureSheet::from_gff(&g.root));
        let stored = g.root.integer("MaxHitPoints").unwrap_or(-1);
        if i64::from(stats.max_hit_points) == stored {
            ok += 1;
        } else {
            bad.push(format!("{r}: stored {stored}, ours {}", stats.max_hit_points));
        }
    }
    eprintln!("{ok} agree, {} differ", bad.len());
    assert!(ok >= 1520 && bad.len() <= 60, "{} differ:\n{}", bad.len(), bad.join("\n"));
}

/// A custom caster class (nwn-issues 566: Aurora's spell lists know only
/// the game's casters) gets its spells by its own `classes.2da` row's
/// `SpellCaster` and `SpellTableColumn`: a hak's Seer, the sorcerer's row
/// but for its column in `spells.2da`, which gives Magic Missile at the
/// first level and nothing at the zeroth.
#[test]
fn a_custom_caster_class_s_spells_are_its_own_column_s() {
    use mg_core::Codepage;
    use mg_resman::{LayerClass, MemContainer, priority};
    let root = corpus!();
    let mut gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let table = |gd: &GameData, name: &str| {
        let data = gd.resman.get(&ResKey::parse(name, ResType::TWODA).unwrap()).unwrap();
        mg_2da::TwoDa::parse(&data, Codepage::WINDOWS_1252).unwrap()
    };
    const SORCERER: usize = 9;
    const MAGIC_MISSILE: usize = 107;
    let mut classes = table(&gd, "classes");
    let seer = classes.add_row();
    for c in 0..classes.columns().len() {
        let cell = classes.cell(SORCERER, c).map(str::to_string);
        classes.set(seer, c, cell).unwrap();
    }
    let column = |t: &mg_2da::TwoDa, name: &str| t.column(name).unwrap();
    classes.set(seer, column(&classes, "Label"), Some("Seer".into())).unwrap();
    classes.set(seer, column(&classes, "SpellTableColumn"), Some("Seer".into())).unwrap();
    let mut spells = table(&gd, "spells");
    spells.add_column("Seer").unwrap();
    spells.set(MAGIC_MISSILE, column(&spells, "Seer"), Some("1".into())).unwrap();
    let mut hak = MemContainer::new();
    for (name, t) in [("classes", &classes), ("spells", &spells)] {
        let bytes = t.to_bytes(Codepage::WINDOWS_1252).unwrap();
        hak.insert(ResKey::parse(name, ResType::TWODA).unwrap(), bytes);
    }
    gd.resman.add(priority::HAK, "hak:seer", LayerClass::Erf, hak);
    gd.invalidate();

    let mut creature = mg_gff::Struct::new(0);
    for (label, v) in [("Str", 10), ("Dex", 10), ("Con", 10), ("Int", 10), ("Wis", 10), ("Cha", 18)]
    {
        creature.set(label, mg_gff::Value::Byte(v));
    }
    creature.set("Race", mg_gff::Value::Byte(6));
    let lists = |c: &mg_gff::Struct| -> Vec<(String, Vec<i64>)> {
        let entry = &c.list("ClassList").unwrap()[0];
        (0..10)
            .filter_map(|n| {
                let label = format!("KnownList{n}");
                let list = entry.list(&label)?;
                Some((label, list.iter().filter_map(|s| s.integer("Spell")).collect()))
            })
            .collect()
    };
    let sorcerer = lists(&gd.level_up(&creature, &[(SORCERER as u32, 5)]));
    assert!(sorcerer.iter().any(|(l, _)| l == "KnownList0"), "the sorcerer's own: {sorcerer:?}");
    let seer = lists(&gd.level_up(&creature, &[(seer as u32, 5)]));
    assert_eq!(seer, [("KnownList1".to_string(), vec![MAGIC_MISSILE as i64])]);
}
