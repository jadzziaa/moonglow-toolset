//! Creature statistics compared with Aurora's: 22 creatures (the bandit with
//! one thing changed: class and level, several classes, abilities, natural
//! AC, hit points, special abilities, no feats) opened and confirmed in
//! Aurora's Creature Properties, which stores what it computes (the oracle
//! capture `creature-probes.mod`, made from
//! `examples/cr_probe_module.rs`). Moonglow's maximum hit points must be
//! Aurora's.
//!
//! The same capture holds Aurora's challenge ratings, which Moonglow does
//! not compute yet.

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
