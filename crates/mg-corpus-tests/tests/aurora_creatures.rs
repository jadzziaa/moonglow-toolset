//! Creature statistics compared with Aurora's: 22 creatures (the bandit with
//! one thing changed: class and level, several classes, abilities, natural
//! AC, hit points, special abilities, no feats) opened and confirmed in
//! Aurora's Creature Properties, which stores what it computes (the oracle
//! capture `creature-probes.mod`, made from
//! `examples/cr_probe_module.rs`). Moonglow's maximum hit points must be
//! Aurora's.
//!
//! The same capture holds Aurora's challenge ratings, computed as the
//! creatures opened; so do four captures of Aurora's Build (Compile ›
//! Creature CR) on probe creatures (`docs/research/notes_creature_cr.md`).

use mg_core::ResType;
use mg_module::Module;
use mg_resman::GameInstall;
use mg_rules::{CreatureSheet, GameData};
use mg_testkit::{aurora_capture, corpus};

#[test]
fn max_hit_points_match_aurora() {
    let root = corpus!();
    let capture = Module::open(&aurora_capture!("creature-probes.mod")).unwrap();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut n = 0;
    for key in capture.keys_of(ResType::UTC) {
        let g = capture.gff(key).unwrap().unwrap();
        let ours = game.creature_stats(&CreatureSheet::from_gff(&g.root)).max_hit_points;
        assert_eq!(Some(i64::from(ours)), g.root.integer("MaxHitPoints"), "{key}");
        n += 1;
    }
    assert_eq!(n, 22);
}

/// Challenge ratings against Aurora's Build (Compile › Creature CR) on
/// 4182 creatures: the bandit made plain (no feats, abilities 10, natural
/// AC 0, Fighter 5, base HP 25) with one thing changed (levels, hit points,
/// natural AC, feats, special abilities, abilities, every class and race,
/// save bonuses, skills, sizes, several classes), hit points swept against
/// level for five hit dice, and movement rates, spell lists and gear over
/// hit points 1 to 90 (`examples/cr_probe2_module.rs`, modes default, `hp`,
/// `fine` and `terms`); and the 22 creatures of `creature-probes.mod`,
/// rated as Aurora opened them.
#[test]
fn challenge_ratings_match_aurora() {
    let root = corpus!();
    let captures = [
        aurora_capture!("creature-cr-build.mod"),
        aurora_capture!("creature-cr-hp.mod"),
        aurora_capture!("creature-cr-fine.mod"),
        aurora_capture!("creature-cr-terms.mod"),
        aurora_capture!("creature-probes.mod"),
    ];
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let item = |r: mg_core::ResRef| {
        let data = game.resman.get(&mg_resman::ResKey::new(r, ResType::UTI)).ok()?;
        mg_gff::Gff::read(&data).ok().map(|g| g.root)
    };
    let (mut n, mut wrong) = (0, Vec::new());
    for path in captures {
        let capture = Module::open(&path).unwrap();
        for key in capture.keys_of(ResType::UTC) {
            let g = capture.gff(key).unwrap().unwrap();
            let mut sheet = CreatureSheet::from_gff(&g.root);
            sheet.gear_value = game.gear_value(&g.root, &item);
            let ours = game.challenge(&sheet).rating;
            let theirs = g.root.float("ChallengeRating").unwrap();
            if ours != theirs {
                wrong.push(format!("{key}: Aurora {theirs}, Moonglow {ours}"));
            }
            n += 1;
        }
    }
    assert!(wrong.is_empty(), "{} of {n} differ:\n{}", wrong.len(), wrong.join("\n"));
    assert_eq!(n, 4182 + 22);
}
