//! The content doctor's verdicts on shipped content, checked in the engine:
//! with each campaign's haks loaded, the 2DA cells the doctor found empty or
//! missing (a hak's older table hiding the game's rows) are empty to the
//! engine too, and the cells it read are what the engine reads.

use std::time::Duration;

use mg_2da::TwoDa;
use mg_core::{Codepage, ResRef, ResType};
use mg_module::Module;
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::probe_module;

/// (campaign, table, column, row, empty): cells the doctor's findings rest
/// on (empty), and cells of the game's that the haks leave (not empty).
const CELLS: &[(&str, &str, &str, usize, bool)] = &[
    ("Neverwinter Nights - Wyvern Crown of Cormyr.nwm", "placeables", "ModelName", 524, true),
    ("Neverwinter Nights - Wyvern Crown of Cormyr.nwm", "placeables", "ModelName", 599, true),
    ("Neverwinter Nights - Wyvern Crown of Cormyr.nwm", "placeables", "ModelName", 1, false),
    ("Neverwinter Nights - Darkness over Daggerford.nwm", "appearance", "LABEL", 750, true),
    ("Neverwinter Nights - Darkness over Daggerford.nwm", "appearance", "LABEL", 6, false),
    ("Neverwinter Nights - Dark Dreams of Furiae.nwm", "doortypes", "Model", 5002, true),
    ("Neverwinter Nights - Dark Dreams of Furiae.nwm", "doortypes", "Model", 3, false),
];

#[test]
fn shadowed_rows_are_empty_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_doctor");
    let gi = GameInstall::new(&root, None, "en");
    let mut campaigns: Vec<&str> = CELLS.iter().map(|c| c.0).collect();
    campaigns.dedup();
    for (i, campaign) in campaigns.iter().enumerate() {
        let path = root.join("data/nwm").join(campaign);
        let m = Module::open(&path).unwrap();
        let haks = m.haks().unwrap();
        // What the doctor reads: the game with the haks over it.
        let mut rm = ResMan::for_game(&gi).unwrap();
        for hak in &haks {
            let c = ErfContainer::open(&root.join("data/hk").join(format!("{hak}.hak"))).unwrap();
            rm.add(priority::HAK, format!("hak:{hak}"), LayerClass::Erf, c);
        }
        let cells: Vec<_> = CELLS.iter().filter(|c| c.0 == *campaign).collect();
        let mut script = String::from("void main() {\n");
        let mut want = Vec::new();
        for (n, (_, table, column, row, _)) in cells.iter().enumerate() {
            script.push_str(&format!(
                "  WriteTimestampedLogEntry(\"MG_CELL{n} [\" + Get2DAString(\"{table}\", \"{column}\", {row}) + \"]\");\n"
            ));
            let t = TwoDa::parse(
                &rm.get(&ResKey::new(ResRef::from_str(table).unwrap(), ResType::TWODA)).unwrap(),
                Codepage::default(),
            )
            .unwrap();
            want.push(format!("[{}]", t.get(*row, column).unwrap_or_default()));
        }
        script.push_str("  WriteTimestampedLogEntry(\"MG_DONE\");\n}\n");
        let name = format!("doctor{i}");
        let user = dir.join(format!("user{i}"));
        let hak_names: Vec<&str> = haks.iter().map(String::as_str).collect();
        probe_module(&root, &user, &name, &script, &hak_names, &[]);
        let run = run_server(&root, &user, &name, "MG_DONE", Duration::from_secs(300)).unwrap();
        assert!(run.finished, "{campaign}: the server didn't finish");
        for (n, w) in want.iter().enumerate() {
            let got = run.values(&format!("MG_CELL{n}"));
            assert_eq!(got.first(), Some(w), "{campaign}: {:?}", cells[n]);
        }
        // The cells the findings name are empty; the controls aren't.
        for (cell, w) in cells.iter().zip(&want) {
            assert_eq!(w == "[]", cell.4, "{campaign}: {cell:?} is {w}");
        }
    }
}
