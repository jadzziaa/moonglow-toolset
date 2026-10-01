//! The Levelup Wizard against Aurora's: bandits (Fighter 1) levelled up in
//! Aurora (`levelup/bandit-fighter5.mod`: to Fighter 5; `six-after.mod`:
//! Fighter 2, 3 and 10, and Wizard 1, Rogue 3 and Cleric 5 added; made from
//! `examples/popup_probe_module.rs` and `levelup_probe_module.rs`). Hit
//! points, abilities, feats (in order), skills and maximum hit points must
//! be Aurora's.

use mg_core::ResType;
use mg_gff::Struct;
use mg_module::Module;
use mg_resman::{GameInstall, ResKey};
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
                .map(|e| (e.integer("Class").unwrap(), e.integer("ClassLevel").unwrap()))
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
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
