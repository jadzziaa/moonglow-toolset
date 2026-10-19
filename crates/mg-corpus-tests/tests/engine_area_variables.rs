//! An area's local variables, in the engine: the game reads them from
//! the GIT's root `VarTable`, not from the ARE's (a builder's report: a
//! variable set on the Advanced page read 0 in the area's OnEnter, where
//! Moonglow had written it into the ARE).

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::compile;

const PROBE: &str = r#"
void main()
{
    object oArea = GetFirstArea();
    WriteTimestampedLogEntry("MG_VAR are|" + IntToString(GetLocalInt(oArea, "iInAre")));
    WriteTimestampedLogEntry("MG_VAR git|" + IntToString(GetLocalInt(oArea, "iInGit")));
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// A VarTable of one int variable.
fn var_table(name: &str, value: i32) -> Value {
    let mut v = Struct::new(0);
    v.set("Name", Value::String(name.as_bytes().to_vec()));
    v.set("Type", Value::Dword(1));
    v.set("Value", Value::Int(value));
    Value::List(vec![v])
}

#[test]
fn an_area_s_variables_are_read_from_its_git() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_area_variables");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Area Variables", &mut rng).unwrap();
    let spec = AreaSpec { name: "Vars".into(), tileset: r("ttr01"), width: 2, height: 2 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    for (t, name, value) in [(ResType::ARE, "iInAre", 1), (ResType::GIT, "iInGit", 2)] {
        let key = ResKey::new(area, t);
        let mut g = m.gff(&key).unwrap().unwrap();
        g.root.set("VarTable", var_table(name, value));
        m.set_gff(key, &g).unwrap();
    }
    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_areavars.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_areavars", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let got = run.values("MG_VAR");
    assert_eq!(got, ["are|0", "git|2"], "the engine reported:\n{}", got.join("\n"));
}
