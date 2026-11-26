//! The Levelup Wizard against Aurora's: bandits (Fighter 1) levelled up in
//! Aurora (`levelup/bandit-fighter5.mod`: to Fighter 5; `six-after.mod`:
//! Fighter 2, 3 and 10, and Wizard 1, Rogue 3 and Cleric 5 added; made from
//! `examples/popup_probe_module.rs` and `levelup_probe_module.rs`;
//! `spells-after.mod`: Paladin 4 added at Wisdom 14, Sorcerer 1 and Bard 1
//! added, from `levelup_probe_module.rs spells`). Hit
//! points, abilities, feats (in order), skills, a new class's memorized and known
//! spells and package equipment (equipped or where in the backpack), and
//! maximum hit points must be Aurora's.

use mg_core::{ResRef, ResType};
use mg_gff::Struct;
use mg_module::Module;
use mg_resman::{GameInstall, ResKey};
use mg_rules::levelup::GearPlace;
use mg_rules::{CreatureSheet, GameData};
use mg_testkit::{aurora_capture, corpus};

fn creatures(m: &Module) -> Vec<Struct> {
    let git = m.gff(&ResKey::parse("field", ResType::GIT).unwrap()).unwrap().unwrap();
    git.root.list("Creature List").unwrap().to_vec()
}

/// What levelling up changes, one line each.
fn summary(game: &GameData, c: &Struct) -> Vec<String> {
    let ints = |l: &str| c.integer(l).unwrap_or(0);
    let mut out = vec![
        format!(
            "classes {:?}",
            c.list("ClassList")
                .unwrap()
                .iter()
                .map(|e| {
                    let spells: Vec<(String, Vec<i64>)> = ["MemorizedList", "KnownList"]
                        .iter()
                        .flat_map(|kind| (0..10).map(move |n| format!("{kind}{n}")))
                        .filter_map(|l| {
                            let list = e.list(&l)?;
                            Some((l, list.iter().filter_map(|s| s.integer("Spell")).collect()))
                        })
                        .collect();
                    (e.integer("Class").unwrap(), e.integer("ClassLevel").unwrap(), spells)
                })
                .collect::<Vec<_>>()
        ),
        format!("hit points {} {}", ints("HitPoints"), ints("CurrentHitPoints")),
        format!("abilities {:?}", ["Str", "Dex", "Con", "Int", "Wis", "Cha"].map(ints)),
        format!(
            "feats {:?}",
            c.list("FeatList")
                .unwrap()
                .iter()
                .map(|f| f.integer("Feat").unwrap())
                .collect::<Vec<_>>()
        ),
        format!(
            "skills {:?}",
            c.list("SkillList")
                .unwrap()
                .iter()
                .map(|s| s.integer("Rank").unwrap())
                .collect::<Vec<_>>()
        ),
    ];
    out.push(format!(
        "max hit points {}",
        game.creature_stats(&CreatureSheet::from_gff(c)).max_hit_points
    ));
    out
}

/// A creature's feats sorted by their names, as Moonglow's creature
/// Properties window writes them at its OK, as Aurora's does.
fn sort_feats_by_name(game: &GameData, c: &mut Struct) {
    if let Some(list) = mg_edit::lists::feats_by_name(c, mg_edit::lists::feat_name(game)) {
        c.set("FeatList", mg_gff::Value::List(list));
    }
}

fn feats(c: &Struct) -> Vec<i64> {
    c.list("FeatList").unwrap().iter().filter_map(|f| f.integer("Feat")).collect()
}

/// Aurora's creature Properties window writes the feats sorted by name at
/// its OK, changed or not (`levelup/featsort-after.mod`, 2026-11-26: the
/// first of `featsort-before.mod`'s bandits OK'd, the others left; and in
/// `featsort-palette.mod`, from `spells-before.mod`, the first OK'd, the
/// second OK'd on its Feats page, the third added to the palette, whose
/// window OK'd its blueprint), and Moonglow's sort is its order: Windows'
/// word sort (Bullheaded before Bull's Strength, Halflings before
/// Half-Orcs).
#[test]
fn a_creature_s_properties_ok_sorts_its_feats_as_aurora_s() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let (before, after, palette) = (
        Module::open(&aurora_capture!("levelup/featsort-before.mod")).unwrap(),
        Module::open(&aurora_capture!("levelup/featsort-after.mod")).unwrap(),
        Module::open(&aurora_capture!("levelup/featsort-palette.mod")).unwrap(),
    );
    let ours = |c: &Struct| {
        let mut c = c.clone();
        sort_feats_by_name(&game, &mut c);
        feats(&c)
    };
    let (b, a) = (creatures(&before), creatures(&after));
    assert_eq!(feats(&a[0]), ours(&b[0]));
    assert_ne!(feats(&a[0]), feats(&b[0]), "the capture's feats were out of order");
    assert_eq!(feats(&a[1]), feats(&b[1]), "one not opened is left");
    let p = creatures(&palette);
    for c in &p[..2] {
        assert_eq!(feats(c), ours(c), "OK'd");
    }
    let blueprint =
        palette.gff(&ResKey::parse("bandit002", ResType::UTC).unwrap()).unwrap().unwrap();
    assert_eq!(feats(&blueprint.root), ours(&p[2]), "the palette's copy, OK'd");
    assert_ne!(feats(&p[2]), ours(&p[2]), "the placed one is left");
}

#[test]
fn levelling_up_matches_aurora() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    // (capture before, after, each creature's class levels).
    type Case<'a> = (&'a str, &'a str, Vec<Vec<(u32, u32)>>);
    let cases: [Case; 3] = [
        ("levelup/bandit-before.mod", "levelup/bandit-fighter5.mod", vec![vec![(4, 5)]]),
        (
            "levelup/six-before.mod",
            "levelup/six-after.mod",
            vec![
                vec![(4, 2)],
                vec![(4, 3)],
                vec![(4, 1), (10, 1)],
                vec![(4, 1), (8, 3)],
                vec![(4, 10)],
                vec![(4, 1), (2, 5)],
            ],
        ),
        // (Paladin 4 at Wisdom 14, lawful good: a level whose slots are
        // bonus slots alone; Sorcerer 1 and Bard 1: classes that know
        // their spells.)
        (
            "levelup/spells-before.mod",
            "levelup/spells-after.mod",
            vec![vec![(4, 1), (6, 4)], vec![(4, 1), (9, 1)], vec![(4, 1), (1, 1)]],
        ),
    ];
    let item = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        mg_gff::Gff::read(&data).ok().map(|g| g.root)
    };
    let mut failures = Vec::new();
    for (before, after, targets) in cases {
        // (These were levelled through the creature's Properties window,
        // whose OK writes the feats sorted by name, as Moonglow's does:
        // Moonglow's are so sorted to be compared.)
        let by_name = before.starts_with("levelup/spells");
        let (before, after) = (
            Module::open(&aurora_capture!(before)).unwrap(),
            Module::open(&aurora_capture!(after)).unwrap(),
        );
        for ((c, aurora), t) in creatures(&before).iter().zip(creatures(&after)).zip(targets) {
            let mut ours = game.level_up(c, &t);
            if by_name {
                sort_feats_by_name(&game, &mut ours);
            }
            let (a, b) = (summary(&game, &ours), summary(&game, &aurora));
            for (x, y) in a.iter().zip(&b) {
                if x != y {
                    failures.push(format!("{t:?}:\n  Moonglow {x}\n  Aurora   {y}"));
                }
            }
            // A new class's equipment: what Aurora added, in order.
            let had: Vec<i64> =
                c.list("ClassList").unwrap().iter().filter_map(|e| e.integer("Class")).collect();
            let mut ours = Vec::new();
            for &(class, _) in t.iter().filter(|(k, _)| !had.contains(&i64::from(*k))) {
                ours.extend(game.new_class_gear(c, class, &item).into_iter().map(
                    |(r, p)| match p {
                        GearPlace::Equip(slot) => format!("{r} equipped {slot}"),
                        GearPlace::Carry(x, y) => format!("{r} carried at {x}, {y}"),
                    },
                ));
            }
            let before_slots: Vec<u32> =
                c.list("Equip_ItemList").unwrap_or(&[]).iter().map(|e| e.id).collect();
            let resref = |e: &Struct| e.resref("TemplateResRef").unwrap_or(ResRef::EMPTY);
            let mut theirs: Vec<String> = aurora
                .list("Equip_ItemList")
                .unwrap_or(&[])
                .iter()
                .filter(|e| !before_slots.contains(&e.id))
                .map(|e| format!("{} equipped {}", resref(e), e.id))
                .collect();
            theirs.extend(aurora.list("ItemList").unwrap_or(&[]).iter().map(|e| {
                let at = |l: &str| e.integer(l).unwrap_or(0);
                format!("{} carried at {}, {}", resref(e), at("Repos_PosX"), at("Repos_Posy"))
            }));
            // Equipped first, then carried: Aurora's lists keep them apart.
            ours.sort_by_key(|l| !l.contains("equipped"));
            if ours != theirs {
                failures.push(format!("{t:?} gear:\n  Moonglow {ours:?}\n  Aurora   {theirs:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
