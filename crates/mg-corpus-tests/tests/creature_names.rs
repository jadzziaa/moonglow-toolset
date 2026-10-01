//! Random names from the game's own letter tables: every playable race's
//! male, female and last names come out (4 to 13 letters of the tables'
//! alphabet, capitalised).

use mg_resman::GameInstall;
use mg_rules::GameData;
use mg_testkit::corpus;

#[test]
fn every_playable_race_has_random_names() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(3);
    for race in 0..=6 {
        for (gender, last) in [(0, false), (1, false), (0, true)] {
            for _ in 0..20 {
                let name = game.random_name(race, gender, last, &mut rng).unwrap_or_else(|| {
                    panic!("race {race}, gender {gender}, last {last}: no name")
                });
                assert!((4..=13).contains(&name.len()), "{name}");
                assert!(name.chars().next().unwrap().is_ascii_uppercase(), "{name}");
                assert!(
                    name.chars().all(|c| c.is_ascii_alphabetic() || c == '\'' || c == '-'),
                    "{name}"
                );
            }
        }
    }
}
