//! The Levelup Wizard against Aurora's: bandits (Fighter 1) levelled up in
//! Aurora (`levelup/bandit-fighter5.mod`: to Fighter 5; `six-after.mod`:
//! Fighter 2, 3 and 10, and Wizard 1, Rogue 3 and Cleric 5 added; made from
//! `examples/popup_probe_module.rs` and `levelup_probe_module.rs`). Hit
//! points, abilities, feats (in order), skills, a new class's memorized
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
                    let spells: Vec<(String, Vec<i64>)> = (0..10)
                        .filter_map(|n| {
                            let l = format!("MemorizedList{n}");
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

#[test]
fn levelling_up_matches_aurora() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    // (capture before, after, each creature's class levels).
    type Case<'a> = (&'a str, &'a str, Vec<Vec<(u32, u32)>>);
    let cases: [Case; 2] = [
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
    ];
    let item = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        mg_gff::Gff::read(&data).ok().map(|g| g.root)
    };
    let mut failures = Vec::new();
    for (before, after, targets) in cases {
        let (before, after) = (
            Module::open(&aurora_capture!(before)).unwrap(),
            Module::open(&aurora_capture!(after)).unwrap(),
        );
        for ((c, aurora), t) in creatures(&before).iter().zip(creatures(&after)).zip(targets) {
            let ours = game.level_up(c, &t);
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
