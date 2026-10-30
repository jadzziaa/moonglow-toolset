//! Whether the engine honors a placeable walkmesh's own node position:
//! Tyrants of the Moonsea's `ptm_candle02.pwk` (in the game's keys) puts
//! its mesh 72 m from the candle, and Moonglow's area view draws it there
//! (`object_walkmeshes.rs`). Creatures made where a mesh is are moved off
//! it (a wagon's), but one made where the candle's would be, at the
//! candle or 72 m off, was not, nor with a module's copy twenty times as
//! wide: so the engine ignored the mesh, or the module's copy, or making
//! a creature doesn't see it. Settled here by creatures' paths: each
//! walks a lane straight across a place, and a mesh there turns it aside.
//!
//! Lanes, each a creature's from y 28 to 52 at its x (every placeable
//! turned 0):
//! - a wagon of the game's (`plc_f09`), at y 40: its own walkmesh, the
//!   path's control;
//! - the game's armoire, its walkmesh replaced by the module's own copy, a
//!   4 m square at its node's origin, at y 40: whether a module's copy is
//!   read;
//! - the same copy put in the scratch user folder's `override`, another
//!   placeable's: whether an override's is;
//! - a third placeable at y 25, its override copy's node at (0, 15): the
//!   square at y 40, on the lane, where Moonglow draws it;
//! - and that placeable's own spot (y 25) walked across, from 13 to 37:
//!   where the square is if the node's position is not taken.
//!
//! The walkers go round all four squares, the moved one where its node
//! puts it, and straight over the placeable's own spot: the engine honors
//! a placeable walkmesh's node position, as Moonglow draws it (read
//! 2026-10-30: about 0.9 m aside at the wagon, 2 to 6 m at the squares,
//! none at the origin). (Making a creature on such a place didn't move it:
//! what a creature is made on is not how it walks.)

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

use crate::common;
use common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// Each lane: its name, x, and where its walk starts and ends (y).
const LANES: [(&str, f32, f32, f32); 5] = [
    ("wagon", 10.0, 28.0, 52.0),
    ("module", 22.0, 28.0, 52.0),
    ("override", 34.0, 28.0, 52.0),
    ("moved", 46.0, 28.0, 52.0),
    ("origin", 58.0, 13.0, 37.0),
];

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

void Track(object o, string sLane, int n)
{
    vector v = GetPosition(o);
    Log("MG_AT " + sLane + "|" + IntToString(n) + "|" + FloatToString(v.x, 0, 2) + "|"
        + FloatToString(v.y, 0, 2));
    if (n < 50) DelayCommand(0.5, Track(o, sLane, n + 1));
    else if (sLane == "origin") Log("MG_DONE");
}

void Walk(object oArea, string sLane, float x, float y0, float y1)
{
    object o = CreateObject(OBJECT_TYPE_CREATURE, "mg_walker",
        Location(oArea, Vector(x, y0, 0.0), 90.0), FALSE, "MG_" + sLane);
    SetAILevel(o, AI_LEVEL_NORMAL);
    AssignCommand(o, ActionMoveToLocation(Location(oArea, Vector(x, y1, 0.0), 90.0)));
    Track(o, sLane, 0);
}

void main()
{
    object oArea = GetArea(GetObjectByTag("MG_WAGON"));
    Walk(oArea, "wagon", 10.0, 28.0, 52.0);
    Walk(oArea, "module", 22.0, 28.0, 52.0);
    Walk(oArea, "override", 34.0, 28.0, 52.0);
    Walk(oArea, "moved", 46.0, 28.0, 52.0);
    Walk(oArea, "origin", 58.0, 13.0, 37.0);
}
"#;

/// A placeable's walkmesh of ours: a 4 m square of non-walkable ground
/// (material 7, as the game's placeables have), its node at `at`.
fn square(model: &str, at: [f32; 2]) -> Vec<u8> {
    format!(
        "node trimesh {model}_wg\n  parent {model}_pwk\n  position {} {} 0.0\n  \
         orientation 0.0 0.0 0.0 0.0\n  bitmap NULL\n  verts 4\n    -2.0 -2.0 0.0\n    \
         2.0 -2.0 0.0\n    -2.0 2.0 0.0\n    2.0 2.0 0.0\n  faces 2\n    2 0 3  1  0 0 0  7\n    \
         1 3 0  1  0 0 0  7\nendnode\n",
        at[0], at[1]
    )
    .into_bytes()
}

#[test]
fn a_placeable_walkmesh_s_node_position_by_creatures_paths() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_placeable_walkmesh");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(&game, "Walkmesh Paths", &mut rng).unwrap();
    let spec = AreaSpec { name: "Lanes".into(), tileset: r("ttr01"), width: 8, height: 8 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();

    // The placeables.2da rows: a wagon's, and three others whose
    // walkmeshes are replaced (their models' names).
    let table = game.table("placeables").unwrap();
    let model = |row: usize| table.get(row, "ModelName").unwrap().to_ascii_lowercase();
    let wagon = (0..table.len()).find(|&i| model(i) == "plc_f09").unwrap();
    let (module_row, override_row, moved_row) = (0usize, 1usize, 2usize);
    for row in [module_row, override_row, moved_row] {
        let pwk = ResKey::new(r(&model(row)), ResType::PWK);
        assert!(game.resman.contains(&pwk), "{pwk}: the game has one to replace");
    }
    m.set(ResKey::new(r(&model(module_row)), ResType::PWK), square(&model(module_row), [0.0, 0.0]));
    let user = dir.join("user");
    let over = user.join("override");
    std::fs::create_dir_all(&over).unwrap();
    let file = |row: usize| over.join(format!("{}.pwk", model(row)));
    std::fs::write(file(override_row), square(&model(override_row), [0.0, 0.0])).unwrap();
    std::fs::write(file(moved_row), square(&model(moved_row), [0.0, 15.0])).unwrap();

    let barrel = Gff::read(&game.resman.get(&ResKey::new(r("plc_barrel"), ResType::UTP)).unwrap())
        .unwrap()
        .root;
    let items = |_: ResRef| -> Option<Struct> { None };
    let placing = Placing { game: &game, item: &items };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTP).unwrap();
    let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
    for (tag, row, x, y) in [
        ("MG_WAGON", wagon, 10.0, 40.0),
        ("MG_MODULE", module_row, 22.0, 40.0),
        ("MG_OVERRIDE", override_row, 34.0, 40.0),
        ("MG_MOVED", moved_row, 46.0, 25.0),
        ("MG_ORIGIN", moved_row, 58.0, 25.0),
    ] {
        let mut s = barrel.clone();
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        s.set("Appearance", Value::Dword(row as u32));
        s.set("Static", Value::Byte(0));
        s.set("Useable", Value::Byte(0));
        s.set("Plot", Value::Byte(1));
        let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
        objects.push(instance(&placing, ResType::UTP, &s, at, &[]).unwrap());
    }
    git.root.set(list, Value::List(objects));
    m.set_gff(git_key, &git).unwrap();

    // The walker: the game's commoner with no scripts of its own (its AI
    // would walk it elsewhere).
    let mut walker =
        Gff::read(&game.resman.get(&ResKey::new(r("nw_bandit001"), ResType::UTC)).unwrap())
            .unwrap();
    for f in walker.root.fields.iter_mut() {
        if f.label.to_string_lossy().starts_with("Script") {
            f.value = Value::ResRef(Vec::new());
        }
    }
    walker.root.set("TemplateResRef", Value::ResRef(b"mg_walker".to_vec()));
    m.set_gff(ResKey::new(r("mg_walker"), ResType::UTC), &walker).unwrap();

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_paths.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_paths", "MG_DONE", Duration::from_secs(180)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);

    // Each lane's path: how far it went aside (and where along it), and
    // where it ended.
    let mut paths = std::collections::HashMap::new();
    for (lane, x, _, end) in LANES {
        let path: Vec<(f32, f32)> = run
            .values("MG_AT")
            .iter()
            .filter_map(|v| {
                let p: Vec<&str> = v.split('|').collect();
                (p.first() == Some(&lane)).then(|| (p[2].parse().unwrap(), p[3].parse().unwrap()))
            })
            .collect();
        assert!(!path.is_empty(), "{lane}: no path\n{}", run.log);
        let (aside, at) = path
            .iter()
            .map(|(px, py)| ((px - x).abs(), *py))
            .fold((0.0f32, 0.0f32), |best, a| if a.0 > best.0 { a } else { best });
        let last = path.last().copied().unwrap();
        assert!((last.1 - end).abs() < 0.5, "{lane} ended at {last:?}: {path:?}");
        eprintln!("{lane}: {aside:.2} m aside at most (at y {at:.1})");
        paths.insert(lane, (aside, at));
    }
    // The wagon's own walkmesh turns its walker aside, and a square of
    // ours as well, the module's copy or the override's: both are read.
    for lane in ["wagon", "module", "override"] {
        assert!(paths[lane].0 > 0.5, "{lane}: {:?}", paths[lane]);
    }
    // The square whose node is 15 m on turns its walker aside there,
    // about y 40, 15 m past the placeable; nothing is at the placeable's
    // own spot. The engine honors the node's position, as Moonglow draws
    // it (`object_walkmeshes.rs`).
    let (aside, at) = paths["moved"];
    assert!(aside > 1.0 && (34.0..46.0).contains(&at), "moved: {aside} at {at}");
    assert!(paths["origin"].0 < 0.2, "origin: {:?}", paths["origin"]);
}
