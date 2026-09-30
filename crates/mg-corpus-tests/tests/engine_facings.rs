//! How an area's instances face, in the engine: a door's or placeable's
//! `Bearing` (radians) is its facing less 90° (so, with the game turning a
//! model by facing − 90°, the model turns by the bearing itself); a
//! waypoint's (or creature's, item's, store's) orientation vector points
//! along its facing. Instances placed with the same turn face the same way.
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, scratch_dir};

mod common;
use common::compile;

#[test]
fn bearings_and_orientations_face_as_the_engine_reports() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        return;
    }
    let dir = scratch_dir("engine_facing_probe");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::new();
    let mut m = new_module(&game, "F", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "A".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let get = |n: &str, t| {
        Gff::read(&game.resman.get(&ResKey::new(ResRef::from_str(n).ok()?, t)).ok()?).ok()
    };
    let items = |r: ResRef| get(&r.to_string(), ResType::UTI).map(|g| g.root);
    let placing = Placing { game: &game, item: &items };
    // A door, a placeable and a waypoint, each turned 0.5 rad (28.6°).
    for (from, t, tag, x) in [
        ("nw_door_ttr_01", ResType::UTD, "D1", 10.0),
        ("plc_chest1", ResType::UTP, "P1", 15.0),
        ("nw_waypoint001", ResType::UTW, "W1", 20.0),
    ] {
        let mut bp = get(from, t).unwrap();
        bp.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
        let at = Placement { position: [x, x, 0.0], rotation: 0.5 };
        let placed = instance(&placing, t, &bp.root, at, &[]).unwrap();
        let (list, _) = git_list(t).unwrap();
        let mut items = git.root.list(list).unwrap_or(&[]).to_vec();
        items.push(placed);
        git.root.set(list, Value::List(items));
    }
    m.set_gff(git_key, &git).unwrap();
    let probe = compile(
        &dir,
        "mg_probe",
        r#"
void main()
{
    WriteTimestampedLogEntry("MG_F " + FloatToString(GetFacing(GetObjectByTag("D1"))) + " " + FloatToString(GetFacing(GetObjectByTag("P1"))) + " " + FloatToString(GetFacing(GetObjectByTag("W1"))));
    WriteTimestampedLogEntry("MG_DONE");
}
"#,
    );
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_f.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_f", "MG_DONE", Duration::from_secs(60)).unwrap();
    assert!(run.finished, "the server did not finish");
    // Turned 0.5 rad (28.6°): facing 118.6°, which the engine rounds.
    let got: Vec<String> = run.values("MG_F")[0].split_whitespace().map(str::to_string).collect();
    assert_eq!(got, ["119.000000000", "119.000000000", "119.000000000"]);
}
