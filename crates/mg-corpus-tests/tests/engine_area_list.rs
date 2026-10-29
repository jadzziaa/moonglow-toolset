//! Which of a module's areas the game loads: those `Mod_Area_list` (the
//! IFO's list) names, not every ARE the module holds. An ARE the list
//! leaves out is no area in the game: `GetFirstArea`/`GetNextArea` don't
//! reach it and its tag finds nothing. Moonglow's tools see every ARE and
//! say which the list leaves out (`mg_module::areas::AreaInfo::listed`).

use std::time::Duration;

use mg_core::{ResRef, ResType};
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
    while (GetIsObjectValid(oArea))
    {
        WriteTimestampedLogEntry("MG_AREA loaded|" + GetResRef(oArea));
        oArea = GetNextArea();
    }
    object oTagged = GetObjectByTag("MG_UNLISTED");
    WriteTimestampedLogEntry("MG_AREA by tag|" + IntToString(GetIsObjectValid(oTagged)));
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

#[test]
fn the_game_loads_the_areas_its_list_names() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_area_list");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Area List", &mut rng).unwrap();
    let spec =
        |name: &str| AreaSpec { name: name.into(), tileset: r("ttr01"), width: 2, height: 2 };
    let listed = add_area(&mut m, &game, &spec("Listed"), &mut rng).unwrap();
    let unlisted = add_area(&mut m, &game, &spec("Unlisted"), &mut rng).unwrap();
    let key = ResKey::new(unlisted, ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    are.root.set("Tag", mg_gff::Value::String(b"MG_UNLISTED".to_vec()));
    m.set_gff(key, &are).unwrap();
    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    // The second area's ARE and GIT stay; the list names the first alone.
    (info.root.items_mut(&ifo::MOD_AREA_LIST))
        .retain(|a| a.try_read(&ifo::mod_area_list::AREA_NAME) != Some(unlisted));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    assert_eq!(m.areas().unwrap(), [listed]);
    assert!(m.contains(&ResKey::new(unlisted, ResType::GIT)));
    let infos = mg_module::areas::list(&m);
    assert_eq!(
        infos.iter().map(|a| (a.resref, a.listed)).collect::<Vec<_>>(),
        [(listed, true), (unlisted, false)]
    );

    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_arealist.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_arealist", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let got = run.values("MG_AREA");
    let want = [format!("loaded|{listed}"), "by tag|0".to_string()];
    assert_eq!(got, want, "the engine reported:\n{}", got.join("\n"));
}
