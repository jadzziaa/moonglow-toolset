//! The engine's default area light directions, which the renderer uses for
//! areas whose scripts do not set one: `GetAreaLightDirection` in a new
//! area, for the sun and the moon.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::compile;

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
string V(vector v)
{
    return FloatToString(v.x, 0, 4) + " " + FloatToString(v.y, 0, 4) + " " + FloatToString(v.z, 0, 4);
}
void main()
{
    object area = GetFirstArea();
    Log("MG_SUN " + V(GetAreaLightDirection(AREA_LIGHT_DIRECTION_SUN, area)));
    Log("MG_MOON " + V(GetAreaLightDirection(AREA_LIGHT_DIRECTION_MOON, area)));
    Log("MG_DONE");
    int i;
    for (i = 0; i < 2000; i++)
        Log("MG_PAD ................................................................");
}
"#;

#[test]
fn default_area_light_directions() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_area_light");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(3);
    let mut m = new_module(&game, "Light", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Room".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 2,
        height: 2,
    };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_light.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_light", "MG_DONE", Duration::from_secs(300)).unwrap();
    assert!(run.finished, "the server did not finish");
    let parse = |v: &[String]| -> Vec<f32> {
        v[0].split_whitespace().map(|x| x.parse().unwrap()).collect()
    };
    let sun = parse(&run.values("MG_SUN"));
    let moon = parse(&run.values("MG_MOON"));
    eprintln!("sun {sun:?}, moon {moon:?}");
    // The renderer's default: towards (4000, 4500, 7000).
    let ours = [4000.0f32, 4500.0, 7000.0];
    let len = ours.iter().map(|x| x * x).sum::<f32>().sqrt();
    let ours: Vec<f32> = ours.iter().map(|x| x / len).collect();
    let dot: f32 = sun.iter().zip(&ours).map(|(a, b)| a * b).sum::<f32>()
        / sun.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(dot > 0.999, "the engine's sun direction is {sun:?}");
}
