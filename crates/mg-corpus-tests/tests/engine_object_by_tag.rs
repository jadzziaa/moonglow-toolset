//! Which object `GetObjectByTag` takes of several with one tag, across
//! areas and kinds (`engine_static_placeables.rs` has two in one area: the
//! second placed is found first). The one the engine loaded last, and
//! `nNth` counts back from it: the areas in the module's order, an area's
//! placeables before its waypoints, each kind in the order placed.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

const PROBE: &str = r#"
void main()
{
    string s = "";
    int n;
    for (n = 0; n < 7; n++)
    {
        object o = GetObjectByTag("MG_SAME", n);
        s += (n > 0 ? " " : "") + (GetIsObjectValid(o) ? IntToString(GetLocalInt(o, "mg_n")) : "-");
    }
    WriteTimestampedLogEntry("MG_FOUND " + s);
    object oFirst = GetFirstArea();
    WriteTimestampedLogEntry("MG_AREAS " + GetResRef(oFirst) + " " + GetResRef(GetNextArea()));
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

#[test]
fn get_object_by_tag_takes_the_last_area_s_last_placed_first() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_object_by_tag");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Object By Tag", &mut rng).unwrap();
    let mut area = |name: &str| {
        let spec = AreaSpec { name: name.into(), tileset: r("tic01"), width: 4, height: 4 };
        add_area(&mut m, &game, &spec, &mut rng).unwrap()
    };
    let (first, second) = (area("First"), area("Second"));
    let barrel = Gff::read(&game.resman.get(&ResKey::new(r("plc_barrel"), ResType::UTP)).unwrap())
        .unwrap()
        .root;
    let var = |n: i32| {
        let mut v = Struct::new(0);
        v.set("Name", Value::String(b"mg_n".to_vec()));
        v.set("Type", Value::Dword(1));
        v.set("Value", Value::Int(n));
        Value::List(vec![v])
    };
    let items = |_: ResRef| -> Option<Struct> { None };
    let placing = Placing { game: &game, item: &items };
    let (list, _) = git_list(ResType::UTP).unwrap();
    // Three in the first area (1, 2, 3 in the order placed), two in the
    // second (4, 5).
    for (area, numbers) in [(first, &[1, 2, 3][..]), (second, &[4, 5][..])] {
        let key = ResKey::new(area, ResType::GIT);
        let mut doc = m.gff(&key).unwrap().unwrap();
        let mut objects = doc.root.list(list).unwrap_or(&[]).to_vec();
        for (i, n) in numbers.iter().enumerate() {
            let mut s = barrel.clone();
            s.set("Tag", Value::String(b"MG_SAME".to_vec()));
            s.set("VarTable", var(*n));
            let at = Placement { position: [3.0 + 3.0 * i as f32, 20.0, 0.0], rotation: 0.0 };
            objects.push(instance(&placing, ResType::UTP, &s, at, &[]).unwrap());
        }
        doc.root.set(list, Value::List(objects));
        m.set_gff(key, &doc).unwrap();
    }
    // And a waypoint of the tag in the first area (9), placed after its
    // barrels.
    let key = ResKey::new(first, ResType::GIT);
    let mut doc = m.gff(&key).unwrap().unwrap();
    let (wp_list, _) = git_list(ResType::UTW).unwrap();
    let mut waypoints = doc.root.list(wp_list).unwrap_or(&[]).to_vec();
    let mut wp =
        Gff::read(&game.resman.get(&ResKey::new(r("nw_waypoint001"), ResType::UTW)).unwrap())
            .unwrap()
            .root;
    wp.set("Tag", Value::String(b"MG_SAME".to_vec()));
    wp.set("VarTable", var(9));
    let at = Placement { position: [12.0, 30.0, 0.0], rotation: 0.0 };
    waypoints.push(instance(&placing, ResType::UTW, &wp, at, &[]).unwrap());
    doc.root.set(wp_list, Value::List(waypoints));
    m.set_gff(key, &doc).unwrap();
    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_bytag.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_bytag", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    assert_eq!(run.values("MG_AREAS"), ["first second"], "the module's order of areas");
    // The last loaded is found first: the second area's barrels last
    // placed first, then the first area's waypoint (an area's waypoints
    // are loaded after its placeables, whatever the order placed), then
    // its barrels, the last placed first.
    assert_eq!(run.values("MG_FOUND"), ["5 4 9 3 2 1 -"], "{}", run.log);
}
