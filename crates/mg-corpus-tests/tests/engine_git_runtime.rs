//! The runtime fields a saved game keeps in an area's GIT, in a module's
//! GIT: which the engine reads when the module loads, as it reads the
//! grass (`engine_grass.rs`). In `AreaProperties`, the name `SetName`
//! gives the area (`DisplayName`) and the sun's and moon's directions are
//! read; the weather at the root (`CurrentWeather`, `WeatherStarted`) is
//! not: rain there is clear at the load and six seconds later, the
//! weather starting by the ARE's chances. A script reads the weather and
//! the name; the directions no script reads, so the game is saved and its
//! GIT read: the engine kept the module's. (Whether the client lights by
//! them is the client's to say.)

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, save_game, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::compile;

const PROBE: &str = r#"
void Report(string sWhen)
{
    object oArea = GetFirstArea();
    while (GetIsObjectValid(oArea))
    {
        WriteTimestampedLogEntry("MG_AREA " + sWhen + "|" + GetResRef(oArea) + "|"
            + IntToString(GetWeather(oArea)) + "|" + GetName(oArea));
        oArea = GetNextArea();
    }
}

void Later()
{
    Report("later");
    WriteTimestampedLogEntry("MG_DONE");
}

void main()
{
    Report("load");
    DelayCommand(6.0, Later());
}
"#;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

const SUN: [f32; 3] = [0.0, 0.6, 0.8];
const MOON: [f32; 3] = [0.8, 0.0, 0.6];

#[test]
fn an_area_s_name_and_sun_are_read_from_its_git_not_its_weather() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_git_runtime");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Git Runtime", &mut rng).unwrap();
    let mut area = |name: &str| {
        let spec = AreaSpec { name: name.into(), tileset: r("ttr01"), width: 2, height: 2 };
        add_area(&mut m, &game, &spec, &mut rng).unwrap()
    };
    // (Neither area's ARE has a chance of rain: rain is the GIT's.)
    let (wet, plain) = (area("Wet"), area("Plain"));
    let key = ResKey::new(wet, ResType::GIT);
    let mut git = m.gff(&key).unwrap().unwrap();
    git.root.set("CurrentWeather", Value::Byte(1));
    git.root.set("WeatherStarted", Value::Byte(1));
    let mut props = match git.root.get("AreaProperties") {
        Some(Value::Struct(s)) => s.clone(),
        _ => Struct::new(100),
    };
    props.set("DisplayName", Value::String(b"Shown Name".to_vec()));
    for (axis, (s, m)) in ["X", "Y", "Z"].iter().zip(SUN.iter().zip(MOON)) {
        props.set(&format!("SunDirection{axis}"), Value::Float(*s));
        props.set(&format!("MoonDirection{axis}"), Value::Float(m));
    }
    git.root.set("AreaProperties", Value::Struct(props));
    m.set_gff(key, &git).unwrap();
    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_gitrun.mod"))).unwrap();

    let run = run_server(&root, &user, "mg_gitrun", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let mut got = run.values("MG_AREA");
    got.sort();
    // Weather 0 is clear: the GIT's rain is not taken, at the load or
    // later; the name is, and the plain area's is its ARE's.
    let wanted = [
        "later|plain|0|Plain",
        "later|wet|0|Shown Name",
        "load|plain|0|Plain",
        "load|wet|0|Shown Name",
    ];
    assert_eq!(got, wanted, "the engine reported:\n{}", got.join("\n"));

    let sav = save_game(&root, &user, "mg_gitrun", "MG_DONE", Duration::from_secs(120))
        .unwrap_or_else(|e| panic!("the server did not save the game: {e}"));
    let data = std::fs::read(&sav).unwrap();
    let erf = mg_erf::Erf::read(&data).unwrap();
    let directions = |area: ResRef| {
        let entry = erf.find(&area, ResType::GIT).expect("the save has the area's GIT");
        let git = Gff::read(&erf.data(entry).unwrap()).unwrap();
        assert_eq!(git.root.get("CurrentWeather"), Some(&Value::Byte(0)), "{area}'s weather");
        let Some(Value::Struct(p)) = git.root.get("AreaProperties") else {
            panic!("the saved GIT has no AreaProperties")
        };
        let read = |which: &str| {
            ["X", "Y", "Z"].map(|axis| match p.get(&format!("{which}Direction{axis}")) {
                Some(Value::Float(f)) => *f,
                v => panic!("{which}Direction{axis}: {v:?}"),
            })
        };
        (read("Sun"), read("Moon"))
    };
    assert_eq!(directions(wet), (SUN, MOON), "the engine kept the module's directions");
    assert_ne!(directions(plain).0, SUN, "the plain area's are the engine's own");
}
