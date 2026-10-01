//! Build's Creature CR and Encounters passes against Aurora's: an encounter
//! blueprint and a placed encounter listing the bandit, a custom creature
//! and a creature that does not exist, each entry with a stale CR and
//! appearance (`examples/encounter_probe_module.rs`; Aurora's Build in
//! `encounters/after.mod`). The entries must come out as Aurora's: each
//! creature's rating (the custom one recalculated first) and appearance,
//! the missing creature's entry gone.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct};
use mg_module::Module;
use mg_module::build::{compile_creature_cr, compile_encounters};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{aurora_capture, corpus};

/// An encounter's creature entries: (resref, CR, appearance).
type Entries = Vec<(String, String, i64)>;

fn entries(list: &[Struct]) -> Entries {
    list.iter()
        .map(|e| {
            (
                e.resref("ResRef").unwrap().to_string(),
                format!("{:.3}", e.float("CR").unwrap()),
                e.integer("Appearance").unwrap(),
            )
        })
        .collect()
}

/// The blueprint's and the placed encounter's entries.
fn lists(m: &Module) -> (Entries, Entries) {
    let ute = m.gff(&ResKey::parse("mgp_enc", ResType::UTE).unwrap()).unwrap().unwrap();
    let git = m.gff(&ResKey::parse("field", ResType::GIT).unwrap()).unwrap().unwrap();
    let placed = &git.root.list("Encounter List").unwrap()[0];
    (entries(ute.root.list("CreatureList").unwrap()), entries(placed.list("CreatureList").unwrap()))
}

#[test]
fn encounters_are_compiled_as_aurora_does() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut ours = Module::open(&aurora_capture!("encounters/before.mod")).unwrap();
    let aurora = Module::open(&aurora_capture!("encounters/after.mod")).unwrap();
    let item = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    compile_creature_cr(&mut ours, &game, &item);
    let snapshot = ours.clone();
    let creature = |r: ResRef| {
        let k = ResKey::new(r, ResType::UTC);
        let data = snapshot
            .get(&k)
            .map(<[u8]>::to_vec)
            .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    compile_encounters(&mut ours, &creature);
    assert_eq!(lists(&ours), lists(&aurora));
}
