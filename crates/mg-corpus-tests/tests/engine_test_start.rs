//! Test From Here: a module whose start Moonglow moved (Module::set_start)
//! starts players there in the engine: the area, the point and the facing
//! GetStartingLocation reports.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::compile;

const PROBE: &str = r#"
void main()
{
    location l = GetStartingLocation();
    vector v = GetPositionFromLocation(l);
    WriteTimestampedLogEntry("MG_START " + GetResRef(GetAreaFromLocation(l)) + "|"
        + FloatToString(v.x, 0, 2) + "," + FloatToString(v.y, 0, 2) + "," + FloatToString(v.z, 0, 2)
        + "|" + FloatToString(GetFacingFromLocation(l), 0, 1));
    WriteTimestampedLogEntry("MG_DONE");
    int i; for (i = 0; i < 40; i++) WriteTimestampedLogEntry("MG_PAD");
}
"#;

#[test]
fn a_moved_start_is_where_the_engine_starts_players() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_test_start");
    let probe = compile(&dir, "mg_start", PROBE);
    let mut m = Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let area = m.areas().unwrap()[0];
    // Facing south-west: 225 degrees.
    m.set_start(area, [22.5, 31.25, 0.0], 225f32.to_radians()).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_start").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_start").unwrap(), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mgstart.mod"))).unwrap();
    let run = run_server(&root, &user, "mgstart", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished);
    let got = run.values("MG_START");
    assert_eq!(
        got.first().map(|s| s.to_ascii_lowercase()),
        Some(format!("{}|22.50,31.25,0.00|225.0", area.to_string().to_ascii_lowercase()))
    );
}
