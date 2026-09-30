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
