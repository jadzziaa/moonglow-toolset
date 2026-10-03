//! The content doctor on real content: every shipped module with its haks,
//! the haks taken as the user's (custom content, as a builder's would be).
//! What the doctor finds was checked by hand: the defects BioWare and
//! Beamdog left in are listed in `KNOWN`, by module, check and count; any
//! other finding is a false alarm to fix (calibrating this list removed the
//! row-level model check, hiding the game's own rows, and walkmeshes past
//! the tile, which the game's own tiles have).

use mg_module::Module;
use mg_module::doctor::{Severity, TalkTables, examine};
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResMan, priority};
use mg_testkit::{bundled_modules, corpus};

/// Findings in shipped content, checked by hand and real, counted by
/// module, check and source: (module, check, source, count).
const KNOWN: &[(&str, &str, &str, usize)] = &[
    // A tile whose model Beamdog never shipped.
    ("Kingmaker", "set-model", "hak:km_resources", 1),
    // tcm02.set: two tiles without models, a group starting with no tile,
    // tiles declaring doors without door sections, and door types (5001 on)
    // past the hak's own doortypes.2da (1451 rows, hiding the game's).
    ("Neverwinter Nights - Dark Dreams of Furiae", "set-model", "hak:ddf", 2),
    ("Neverwinter Nights - Dark Dreams of Furiae", "set-group", "hak:ddf", 1),
    ("Neverwinter Nights - Dark Dreams of Furiae", "set-data", "hak:ddf", 8),
    ("Neverwinter Nights - Dark Dreams of Furiae", "set-door", "hak:ddf", 17),
    // A blueprint for appearance 750, past the hak's 512-row appearance.2da;
    // world-map pin placeables whose models were never shipped.
    ("Neverwinter Nights - Darkness over Daggerford", "object-row", "module", 11),
    // tno01.set: two groups named "cliff_path1" start with the same tile.
    ("Neverwinter Nights - Darkness over Daggerford", "set-group", "hak:dodee_tno_addons", 1),
    // A placeable on a reserved placeables.2da row (model USER), placed twice
    // in two areas.
    ("Neverwinter Nights - Doom of Icewind Dale", "object-row", "module", 4),
    // A blueprint for appearance 15028, past the hak's appearance.2da.
    ("Neverwinter Nights - Tyrants of the Moonsea", "object-row", "module", 1),
    ("Neverwinter Nights - Tyrants of the Moonsea", "set-data", "hak:tm_race_2da", 2),
    // Placed trees, benches and braziers on placeables.2da rows (500 to 700)
    // that the hak's copy leaves empty, hiding the game's.
    ("Neverwinter Nights - Wyvern Crown of Cormyr", "object-row", "module", 29),
];

#[test]
fn shipped_modules_have_only_known_findings() {
    let root = corpus!();
    let gi = GameInstall::new(&root, None, "en");
    let base =
        mg_tlk::Tlk::read(&std::fs::read(gi.talk_table(false)).unwrap()).unwrap().entries.len();
    let mut unexpected = Vec::new();
    let mut total = 0;
    for path in bundled_modules(&root) {
        let m = Module::open(&path).unwrap();
        let mut rm = ResMan::for_game(&gi).unwrap();
        for hak in m.haks().unwrap() {
            let p = root.join("data/hk").join(format!("{hak}.hak"));
            match ErfContainer::open(&p) {
                Ok(c) => rm.add(priority::HAK_USER, format!("hak:{hak}"), LayerClass::Erf, c),
                Err(_) => eprintln!("{}: hak {hak} not found", path.display()),
            }
        }
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        let custom = m.custom_tlk().ok().flatten().and_then(|name| {
            gi.tlk_dirs().iter().find_map(|d| {
                let data = std::fs::read(d.join(format!("{name}.tlk"))).ok()?;
                Some(mg_tlk::Tlk::read(&data).ok()?.entries.len())
            })
        });
        let found = examine(&m, &rm, TalkTables { base, custom });
        total += found.len();
        let label = path.file_stem().unwrap().to_string_lossy().to_string();
        let mut counts: std::collections::BTreeMap<(&str, &str), usize> = Default::default();
        for f in &found {
            *counts.entry((f.check.as_ref(), f.source.as_str())).or_default() += 1;
        }
        for ((check, source), n) in &counts {
            let known =
                KNOWN.iter().any(|k| k.0 == label && k.1 == *check && k.2 == *source && k.3 == *n);
            if !known {
                unexpected.push(format!("(\"{label}\", \"{check}\", \"{source}\", {n}),"));
                for f in found.iter().filter(|f| f.check == *check && f.source == *source).take(3) {
                    let sev = if f.severity == Severity::Error { "error" } else { "warning" };
                    unexpected.push(format!("    // {sev} {} {}: {}", f.resource, f.at, f.message));
                }
            }
        }
    }
    eprintln!("{total} findings in {} modules", bundled_modules(&root).len());
    assert!(
        unexpected.is_empty(),
        "{} unexpected findings:\n{}",
        unexpected.len(),
        unexpected.join("\n")
    );
}
