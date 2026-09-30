//! How an area's instances face, in the engine: a door's or placeable's
//! `Bearing` (radians) is its facing less 90° (so, with the game turning a
//! model by facing − 90°, the model turns by the bearing itself); a
//! waypoint's (or creature's, item's, store's) orientation vector points
//! along its facing.
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, git_list, instance};
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
        Gff::read(&game.resman.get(&ResKey::new(ResRef::from_str(n).unwrap(), t)).unwrap()).unwrap()
    };
    let mut door = get("nw_door_ttr_01", ResType::UTD);
    door.root.set("Tag", Value::String(b"D1".to_vec()));
    let mut placed = git.root.list("Door List").unwrap_or(&[]).to_vec();
    placed.push(
        instance(
            ResType::UTD,
            &door.root,
            Placement { position: [10.0, 10.0, 0.0], facing: 0.5 },
            &[],
        )
        .unwrap(),
    );
    git.root.set("Door List", Value::List(placed));
    let mut wp = get("nw_waypoint001", ResType::UTW);
    wp.root.set("Tag", Value::String(b"W1".to_vec()));
    let (list, _) = git_list(ResType::UTW).unwrap();
    git.root.set(
        list,
        Value::List(vec![
            instance(
                ResType::UTW,
                &wp.root,
                Placement { position: [20.0, 20.0, 0.0], facing: 0.5 },
                &[],
            )
            .unwrap(),
        ]),
    );
    // A placeable with a bearing, by hand (placeables are not in instances yet).
    let mut p = get("plc_chest1", ResType::UTP).root;
    p.id = 9;
    p.set("Tag", Value::String(b"P1".to_vec()));
    for (l, v) in [("X", 15.0f32), ("Y", 15.0), ("Z", 0.0), ("Bearing", 0.5)] {
        p.set(l, Value::Float(v));
    }
    git.root.set("Placeable List", Value::List(vec![p]));
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
    // Bearing and orientation 0.5 rad (28.6°); the engine rounds facings.
    let got: Vec<String> = run.values("MG_F")[0].split_whitespace().map(str::to_string).collect();
    assert_eq!(got, ["119.000000000", "119.000000000", "29.000000000"]);
}
