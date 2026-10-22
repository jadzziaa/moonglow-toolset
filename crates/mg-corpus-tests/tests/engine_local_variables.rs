//! Local variables as the engine reads them, for described variables
//! (`variables.toml`): a variable read as another type than it was set
//! is that type's empty value (The Domains of Dread's builders' guide,
//! its first pitfall: an Int read as a String is ""), and a name is
//! matched whatever its case, or not (which decides whether Verify may
//! match a described name ignoring case).

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
    object m = GetModule();
    WriteTimestampedLogEntry("MG_VAR same|" + IntToString(GetLocalInt(m, "mg_Case")));
    WriteTimestampedLogEntry("MG_VAR lower|" + IntToString(GetLocalInt(m, "mg_case")));
    WriteTimestampedLogEntry("MG_VAR upper|" + IntToString(GetLocalInt(m, "MG_CASE")));
    WriteTimestampedLogEntry("MG_VAR int as string|[" + GetLocalString(m, "mg_int") + "]");
    WriteTimestampedLogEntry("MG_VAR int as float|" + FloatToString(GetLocalFloat(m, "mg_int"), 0, 1));
    WriteTimestampedLogEntry("MG_VAR string as int|" + IntToString(GetLocalInt(m, "mg_text")));
    WriteTimestampedLogEntry("MG_VAR both int|" + IntToString(GetLocalInt(m, "mg_both")));
    WriteTimestampedLogEntry("MG_VAR both string|" + GetLocalString(m, "mg_both"));
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

fn var(name: &str, value: Value) -> Struct {
    let kind = match value {
        Value::Int(_) => 1,
        _ => 3,
    };
    let mut v = Struct::new(0);
    v.set("Name", Value::String(name.as_bytes().to_vec()));
    v.set("Type", Value::Dword(kind));
    v.set("Value", value);
    v
}

#[test]
fn a_variable_is_read_by_its_type_and_its_name_s_case() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_local_variables");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(&game, "Local Variables", &mut rng).unwrap();
    let spec = AreaSpec { name: "Vars".into(), tileset: r("ttr01"), width: 2, height: 2 };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    let text = |s: &str| Value::String(s.as_bytes().to_vec());
    info.root.set(
        "VarTable",
        Value::List(vec![
            var("mg_Case", Value::Int(7)),
            var("mg_int", Value::Int(5)),
            var("mg_text", text("12")),
            var("mg_both", Value::Int(3)),
            var("mg_both", text("three")),
        ]),
    );
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_localvars.mod"))).unwrap();
    let run =
        run_server(&root, &user, "mg_localvars", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let got = run.values("MG_VAR");
    eprintln!("{}", got.join("\n"));
    assert_eq!(
        got,
        [
            "same|7",
            "lower|0",
            "upper|0",
            "int as string|[]",
            "int as float|0.0",
            "string as int|0",
            "both int|3",
            "both string|three",
        ],
        "the engine reported:\n{}",
        got.join("\n")
    );
}
