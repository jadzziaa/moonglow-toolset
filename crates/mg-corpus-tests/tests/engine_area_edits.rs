//! Area edits, in the engine: objects placed as Aurora places them, then
//! moved, turned, raised and deleted with the area viewer's edits
//! (`mg_area::edit`, applied to a workspace and undone and redone once),
//! stand where the engine reports them.

use std::time::Duration;

use glam::Vec3;
use mg_area::edit::{delete_edits, move_edits};
use mg_area::{AreaModel, ObjectKind};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Workspace};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::compile;

const PROBE: &str = r#"
string F(float f) { return FloatToString(f, 0, 2); }

void Report(string sTag)
{
    object o = GetObjectByTag(sTag);
    vector v = GetPosition(o);
    WriteTimestampedLogEntry("MG_AT " + sTag + "|" + IntToString(GetIsObjectValid(o)) + "|" + F(v.x) + "|" + F(v.y) + "|" + F(v.z) + "|" + F(GetFacing(o)));
}

void main()
{
    Report("MG_WP");
    Report("MG_PLC");
    Report("MG_CRE");
    Report("MG_GONE");
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

#[test]
fn moved_turned_and_deleted_objects_stand_where_the_engine_puts_them() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_area_edits");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(3);
    let mut m = new_module(&game, "Area Edits", &mut rng).unwrap();
    let spec = AreaSpec { name: "Edits".into(), tileset: r("ttr01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let blueprint = |name: &str, t: ResType, tag: &str| -> Struct {
        let mut g = Gff::read(&game.resman.get(&ResKey::new(r(name), t)).unwrap()).unwrap();
        g.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
        g.root
    };
    let items = |res: ResRef| {
        let data = game.resman.get(&ResKey::new(res, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let placing = Placing { game: &game, item: &items };

    // Placed as Aurora places them: facing north, on flat ground.
    let mut git = m.gff(&git_key).unwrap().unwrap();
    for (name, t, tag, x) in [
        ("nw_waypoint001", ResType::UTW, "MG_WP", 10.0),
        ("plc_chest1", ResType::UTP, "MG_PLC", 15.0),
        ("nw_bandit001", ResType::UTC, "MG_CRE", 20.0),
        ("nw_waypoint001", ResType::UTW, "MG_GONE", 25.0),
    ] {
        let at = Placement { position: [x, 10.0, 0.0], rotation: 0.0 };
        let placed = instance(&placing, t, &blueprint(name, t, tag), at, &[]).unwrap();
        let (list, _) = git_list(t).unwrap();
        let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
        objects.push(placed);
        git.root.set(list, Value::List(objects));
    }
    m.set_gff(git_key, &git).unwrap();

    // The viewer's edits, through a workspace.
    let mut ws = Workspace::new(m);
    let git_now = |ws: &mut Workspace| ws.doc(&git_key).unwrap().clone();
    let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().clone();
    let model = AreaModel::read(&game, &are.root, &git_now(&mut ws).root, None);
    let object = |kind: ObjectKind, index: usize| model.object(kind, index).unwrap().clone();
    let s = |ws: &mut Workspace, kind: ObjectKind, index: usize| -> Struct {
        git_now(ws).root.list(kind.list()).unwrap()[index].clone()
    };
    let turn = std::f32::consts::FRAC_PI_4;
    let mut edits = Vec::new();
    // The waypoint 5 m east, turned 45° anticlockwise (facing 135°).
    let wp = object(ObjectKind::Waypoint, 0);
    let wp_s = s(&mut ws, ObjectKind::Waypoint, 0);
    edits.extend(move_edits(git_key, &wp, &wp_s, wp.position + Vec3::X * 5.0, turn));
    // The chest 3 m north and 1 m up, turned 45° (facing 135°).
    let chest = object(ObjectKind::Placeable, 0);
    let chest_s = s(&mut ws, ObjectKind::Placeable, 0);
    let up = chest.position + Vec3::new(0.0, 3.0, 1.0);
    edits.extend(move_edits(git_key, &chest, &chest_s, up, turn));
    // The bandit turned to face west (180°).
    let bandit = object(ObjectKind::Creature, 0);
    let bandit_s = s(&mut ws, ObjectKind::Creature, 0);
    let west = std::f32::consts::FRAC_PI_2;
    edits.extend(move_edits(git_key, &bandit, &bandit_s, bandit.position, west));
    ws.apply(Command::new("Move", edits)).unwrap();
    ws.apply(Command::new("Delete", delete_edits(git_key, &[(ObjectKind::Waypoint, 1)]))).unwrap();
    // Undone and redone: the same.
    ws.undo().unwrap();
    ws.undo().unwrap();
    ws.redo().unwrap();
    ws.redo().unwrap();
    ws.flush().unwrap();
    let mut m = ws.module;

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_edits.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_edits", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let got = run.values("MG_AT");
    for e in [
        "MG_WP|1|15.00|10.00|0.00|135.00",
        "MG_PLC|1|15.00|13.00|1.00|135.00",
        "MG_CRE|1|20.00|10.00|0.00|180.00",
        "MG_GONE|0|0.00|0.00|0.00|0.00",
    ] {
        assert!(
            got.contains(&e.to_string()),
            "expected {e}; the engine reported:\n{}",
            got.join("\n")
        );
    }
}
