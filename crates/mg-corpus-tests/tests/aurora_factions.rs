//! The Faction Editor's operations compared with Aurora's: starting from the
//! standard factions, Aurora added Guards (parent Defender, global) and
//! Bandits (parent Hostile, not global), set how Guards feel about Defender
//! to 10, saved (capture `factions/added-guards-bandits-edit.fac`), then
//! removed Guards and saved (`factions/removed-guards.fac`).

use mg_gff::Gff;
use mg_module::factions::Factions;
use mg_module::new::default_factions;
use mg_testkit::aurora_capture;

fn read(path: std::path::PathBuf) -> Gff {
    Gff::read(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn faction_editor_matches_aurora() {
    let added = read(aurora_capture!("factions/added-guards-bandits-edit.fac"));
    let removed = read(aurora_capture!("factions/removed-guards.fac"));
    let mut f = Factions::read(&default_factions());
    assert_eq!(f.add("Guards", true, 4), 5);
    assert_eq!(f.add("Bandits", false, 1), 6);
    f.set_reputation(5, 4, 10);
    assert_eq!(f.to_gff(), added);
    assert!(f.remove(5).is_some());
    assert_eq!(f.to_gff(), removed);
}
