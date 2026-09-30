//! Engine checks of 2DA parsing questions, run in the game's headless server.

use std::process::Command;
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_erf::{Erf, ErfWriter};
use mg_gff::{Gff, Struct, Value};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

const SCRIPT: &str = r#"
void main()
{
    WriteTimestampedLogEntry("MG_RESULT ADJECTIVE=[" + Get2DAString("random_hostile", "ADJECTIVE", 10) + "]");
    WriteTimestampedLogEntry("MG_RESULT GREETING=[" + Get2DAString("random_hostile", "GREETING", 10) + "]");
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

/// `""` in a 2DA row is an empty cell in place (Moonglow's reading), not
/// skipped (nwn_twoda's): the engine returns "trivial" for ADJECTIVE and ""
/// for GREETING on row 10 of Beamdog's random_hostile.2da.
#[test]
fn engine_reads_quoted_empty_2da_cells_in_place() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let compiler = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_2da");
    std::fs::create_dir_all(dir.join("modules")).unwrap();

    // Compile the probe script.
    std::fs::write(dir.join("mg_probe.nss"), SCRIPT).unwrap();
    let out = Command::new(&compiler)
        .args([
            "-o",
            dir.join("mg_probe.ncs").to_str().unwrap(),
            dir.join("mg_probe.nss").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "compile failed: {}", String::from_utf8_lossy(&out.stderr));

    // A shipped module, with the probe as OnModuleLoad and the hak attached.
    let data = std::fs::read(root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let erf = Erf::read(&data).unwrap();
    let mut w = ErfWriter::new(*b"MOD ");
    for e in &erf.entries {
        let bytes = erf.data(e).unwrap().into_owned();
        let bytes = if e.restype == ResType::IFO {
            let mut ifo = Gff::read(&bytes).unwrap();
            ifo.root.set("Mod_OnModLoad", Value::ResRef(b"mg_probe".to_vec()));
            let mut hak = Struct::new(8);
            hak.set("Mod_Hak", Value::String(b"id_resources".to_vec()));
            ifo.root.set("Mod_HakList", Value::List(vec![hak]));
            ifo.to_bytes().unwrap()
        } else {
            bytes
        };
        w.add(e.resref, e.restype, bytes).unwrap();
    }
    let probe = std::fs::read(dir.join("mg_probe.ncs")).unwrap();
    w.add(ResRef::from_str("mg_probe").unwrap(), ResType::NCS, probe).unwrap();
    std::fs::write(dir.join("modules/mg_probe.mod"), w.to_bytes().unwrap()).unwrap();

    let run = run_server(&root, &dir, "mg_probe", "MG_DONE", Duration::from_secs(60)).unwrap();
    assert!(run.finished, "server did not finish; log:\n{}", run.log);
    let values = run.values("MG_RESULT");
    assert_eq!(values, ["ADJECTIVE=[trivial]", "GREETING=[]"], "log:\n{}", run.log);
}
