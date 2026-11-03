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
//!
//! And whether a placeable's or a door's visual transform (Adjust
//! Location's Scale, rotations and offsets) moves its walkmesh with its
//! model: it doesn't, any part of it (`walkmeshes_by_visual_transforms`,
//! read 2026-11-03), as Moonglow's Object Walkmeshes draw them
//! (`mg_area::walk::ObjectWalkmeshes::faces`).

use std::time::Duration;

use glam::Vec3;
use mg_area::VisualTransform;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{ServerRun, run_server, server_binary};
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
    square_of(model, at, 2.0)
}

/// [`square`], `2 * half` m across.
fn square_of(model: &str, at: [f32; 2], half: f32) -> Vec<u8> {
    let h = half;
    format!(
        "node trimesh {model}_wg\n  parent {model}_pwk\n  position {} {} 0.0\n  \
         orientation 0.0 0.0 0.0 0.0\n  bitmap NULL\n  verts 4\n    -{h:.1} -{h:.1} 0.0\n    \
         {h:.1} -{h:.1} 0.0\n    -{h:.1} {h:.1} 0.0\n    {h:.1} {h:.1} 0.0\n  faces 2\n    \
         2 0 3  1  0 0 0  7\n    1 3 0  1  0 0 0  7\nendnode\n",
        at[0], at[1]
    )
    .into_bytes()
}

/// The walker: the game's commoner with no scripts of its own (its AI
/// would walk it elsewhere), as `mg_walker`.
fn walker(game: &GameData) -> Gff {
    let mut walker =
        Gff::read(&game.resman.get(&ResKey::new(r("nw_bandit001"), ResType::UTC)).unwrap())
            .unwrap();
    for f in walker.root.fields.iter_mut() {
        if f.label.to_string_lossy().starts_with("Script") {
            f.value = Value::ResRef(Vec::new());
        }
    }
    walker.root.set("TemplateResRef", Value::ResRef(b"mg_walker".to_vec()));
    walker
}

/// Lane `lane`'s walker's path (its `MG_AT` lines: x, y every half
/// second).
fn path(run: &ServerRun, lane: &str) -> Vec<(f32, f32)> {
    let path: Vec<(f32, f32)> = run
        .values("MG_AT")
        .iter()
        .filter_map(|v| {
            let p: Vec<&str> = v.split('|').collect();
            (p.first() == Some(&lane)).then(|| (p[2].parse().unwrap(), p[3].parse().unwrap()))
        })
        .collect();
    assert!(!path.is_empty(), "{lane}: no path\n{}", run.log);
    path
}

/// How far lane `lane`'s walker (on x `x`) went aside at most, and where
/// along the lane (y); it must have ended at `end`.
fn walked(run: &ServerRun, lane: &str, x: f32, end: f32) -> (f32, f32) {
    let path = path(run, lane);
    let (aside, at) = path
        .iter()
        .map(|(px, py)| ((px - x).abs(), *py))
        .fold((0.0f32, 0.0f32), |best, a| if a.0 > best.0 { a } else { best });
    let last = path.last().copied().unwrap();
    assert!((last.1 - end).abs() < 0.5, "{lane} ended at {last:?}: {path:?}");
    eprintln!("{lane}: {aside:.2} m aside at most (at y {at:.1})");
    (aside, at)
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

    m.set_gff(ResKey::new(r("mg_walker"), ResType::UTC), &walker(&game)).unwrap();

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
        paths.insert(lane, walked(&run, lane, x, end));
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

/// Lanes for [`walkmeshes_by_visual_transforms`]: its name, the placeable
/// (which square, where it stands, static or not, its visual transform),
/// and the lane's x.
struct VtLane {
    name: &'static str,
    /// Which square: [`CENTERED`], [`NORTH`] or [`BIG`]; or [`DOOR`].
    mesh: usize,
    /// Where the placeable stands (x; every one at y 25, Bearing 0).
    at: f32,
    is_static: bool,
    vt: Option<VisualTransform>,
    /// The visual transform's scale is set by the script, before anyone
    /// walks (`SetObjectVisualTransform`), not by the file.
    scripted: bool,
    /// The walker's x, from y 13 to 43.
    lane: f32,
}

/// The squares, placeables.2da's rows 0 to 2 (their models' walkmeshes
/// replaced): 4 m across at the node's origin, 4 m with the node 6 m
/// north, and 12 m at the origin.
const CENTERED: usize = 0;
const NORTH: usize = 1;
const BIG: usize = 2;
/// The game's generic door (`nw_door_ttr_01`), its own walkmesh.
const DOOR: usize = 3;
/// The closed door's half length (along X), and how far east of it its
/// Scale 3 lane is: 1.15 m clear of its end unscaled, 0.95 m inside it at
/// Scale 3 (3.15 m).
const DOOR_HALF: f32 = 1.05;
const DOOR_BESIDE: f32 = 2.2;
const MESHES: [([f32; 2], f32); 3] = [([0.0, 0.0], 2.0), ([0.0, 6.0], 2.0), ([0.0, 0.0], 6.0)];

const VT_FROM: f32 = 13.0;
const VT_TO: f32 = 43.0;

/// The script that logs every placeable's visual transform as the server
/// has it (`MG_VT`) and walks each lane (`MG_AT`).
fn vt_probe(lanes: &[VtLane]) -> String {
    let mut main = String::from("    object oArea = GetArea(GetObjectByTag(\"MG_0\"));\n");
    for (i, l) in lanes.iter().enumerate() {
        if let (true, Some(v)) = (l.scripted, l.vt) {
            main += &format!(
                "    SetObjectVisualTransform(GetObjectByTag(\"MG_{i}\"), \
                 OBJECT_VISUAL_TRANSFORM_SCALE, {:.1});\n",
                v.scale.x
            );
        }
        main += &format!("    Vt(\"MG_{i}\", \"{}\");\n", l.name);
    }
    for (i, l) in lanes.iter().enumerate() {
        let last = if i + 1 == lanes.len() { "TRUE" } else { "FALSE" };
        main += &format!(
            "    Walk(oArea, \"{}\", {:.1}, {VT_FROM:.1}, {VT_TO:.1}, {last});\n",
            l.name, l.lane
        );
    }
    format!(
        r#"
void Log(string s) {{ WriteTimestampedLogEntry(s); }}
string F(float f) {{ return FloatToString(f, 0, 2); }}

void Track(object o, string sLane, int n, int bLast)
{{
    vector v = GetPosition(o);
    Log("MG_AT " + sLane + "|" + IntToString(n) + "|" + F(v.x) + "|" + F(v.y));
    if (n < 80) DelayCommand(0.5, Track(o, sLane, n + 1, bLast));
    else if (bLast) Log("MG_DONE");
}}

void Walk(object oArea, string sLane, float x, float y0, float y1, int bLast)
{{
    object o = CreateObject(OBJECT_TYPE_CREATURE, "mg_walker",
        Location(oArea, Vector(x, y0, 0.0), 90.0), FALSE, "MW_" + sLane);
    SetAILevel(o, AI_LEVEL_NORMAL);
    AssignCommand(o, ActionMoveToLocation(Location(oArea, Vector(x, y1, 0.0), 90.0)));
    Track(o, sLane, 0, bLast);
}}

void Vt(string sTag, string sLane)
{{
    object o = GetObjectByTag(sTag);
    Log("MG_VT " + sLane + "|" + IntToString(GetIsObjectValid(o)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_SCALE)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_ROTATE_X)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_ROTATE_Y)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_ROTATE_Z)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_TRANSLATE_X)) + "|"
        + F(GetObjectVisualTransform(o, OBJECT_VISUAL_TRANSFORM_TRANSLATE_Y)));
}}

void main()
{{
{main}}}
"#
    )
}

/// Whether a placeable's or a door's visual transform (Adjust Location's
/// Scale, X Rotation, tilts and offsets: `VisTransformList`'s scope 0,
/// as the module's file has it) moves its walkmesh, by creatures' paths,
/// as above. Three squares of the module's own: a 4 m one at its node's
/// origin (placeables.2da row 0's model), one whose node is 6 m north
/// (row 1's) and a 12 m one (row 2's); and the game's generic door. Every
/// object at y 25, Bearing 0, each walker from y 13 to 43, the lanes 16 m
/// apart, on open ground (one tile walkable all over: the area's own
/// rocks and trees turned walkers aside where nothing of ours was).
///
/// - `none`, `scale1`, `scale3`, `static3`, `script3`: the 4 m square
///   with its lane 4 m east of the placeable (2 m clear of its edge): no
///   visual transform, Scale 1, Scale 3 (12 m across: the lane 2 m inside
///   it if the walkmesh is scaled), Scale 3 on a static placeable, and
///   Scale 3 set by a script (`SetObjectVisualTransform`) before anyone
///   walks; `big`, the 12 m square, its control;
/// - `moved`: TranslateX 4 puts the model on the lane; `moved_spot` walks
///   across the spot such a placeable stands on;
/// - `north`: the offset square, walked across at the placeable's x, no
///   transform: the square at y 31;
/// - `turned`, `turned_w`, `turned_e`: X Rotation 90 (about the upright
///   axis, as Bearing turns): the square at y 31 on the placeable's x if
///   the walkmesh doesn't turn, or 6 m west or east at y 25 if it does
///   (walked across 6 m east and west of their placeables);
/// - `tipped`: Y Rotation 90 (tipping it about its X): the square stood
///   up over the placeable if the walkmesh tips with the model;
/// - `door`, `door3`: the door locked across its lane, and one at Scale 3
///   2.2 m west of its lane (1.15 m clear of its end, or 0.95 m inside
///   it at 3.15 m).
///
/// The server has every transform as the file gives it, or the script
/// set it (`GetObjectVisualTransform`), and none moves a walkmesh (read
/// 2026-11-03, paths every half second): `none`, `scale1`, `scale3`,
/// `static3` and `script3` go straight (0.00 m aside), over where a scaled
/// square would be; `big` goes round its square, 2.16 m aside; `moved`
/// goes straight over the model's spot and `moved_spot` round the square
/// at the placeable, 2.15 m; `north`, `turned` and `tipped` go round the
/// square at y 31, 2.16 to 2.17 m aside, and `turned_w` and `turned_e`
/// straight over where a turned one would be. A walker's path goes
/// through a locked door (doors are opened): `door`'s stops at its face
/// (y 24.74) and never passes it, and `door3`'s goes straight on, over
/// where the scaled door's end would be. So Object Walkmeshes draws a
/// walkmesh by the object's position and Bearing alone, the model's
/// visual transform left out.
#[test]
fn walkmeshes_by_visual_transforms() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_walkmesh_visual_transforms");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(&game, "Walkmesh Transforms", &mut rng).unwrap();
    let spec = AreaSpec { name: "Lanes".into(), tileset: r("ttr01"), width: 25, height: 6 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    open_ground(&mut m, &game, area);

    let table = game.table("placeables").unwrap();
    let model = |row: usize| table.get(row, "ModelName").unwrap().to_ascii_lowercase();
    for (row, (at, half)) in MESHES.into_iter().enumerate() {
        let pwk = ResKey::new(r(&model(row)), ResType::PWK);
        assert!(game.resman.contains(&pwk), "{pwk}: the game has one to replace");
        m.set(pwk, square_of(&model(row), at, half));
    }

    let vt = |scale: f32, rotate: [f32; 3], translate: [f32; 3]| {
        Some(VisualTransform {
            scale: Vec3::splat(scale),
            rotate: Vec3::from(rotate),
            translate: Vec3::from(translate),
        })
    };
    let x = |i: usize| 12.0 + 16.0 * i as f32;
    let lane = |i: usize, name, mesh, at: f32, is_static, vt| VtLane {
        name,
        mesh,
        at: x(i) + at,
        is_static,
        vt,
        scripted: false,
        lane: x(i),
    };
    let lanes = [
        lane(0, "none", CENTERED, -4.0, false, None),
        lane(1, "scale1", CENTERED, -4.0, false, vt(1.0, [0.0; 3], [0.0; 3])),
        lane(2, "scale3", CENTERED, -4.0, false, vt(3.0, [0.0; 3], [0.0; 3])),
        lane(3, "static3", CENTERED, -4.0, true, vt(3.0, [0.0; 3], [0.0; 3])),
        lane(4, "moved", CENTERED, -4.0, false, vt(1.0, [0.0; 3], [4.0, 0.0, 0.0])),
        lane(5, "moved_spot", CENTERED, 0.0, false, vt(1.0, [0.0; 3], [4.0, 0.0, 0.0])),
        lane(6, "north", NORTH, 0.0, false, None),
        lane(7, "turned", NORTH, 0.0, false, vt(1.0, [90.0, 0.0, 0.0], [0.0; 3])),
        lane(8, "turned_w", NORTH, 6.0, false, vt(1.0, [90.0, 0.0, 0.0], [0.0; 3])),
        lane(9, "turned_e", NORTH, -6.0, false, vt(1.0, [90.0, 0.0, 0.0], [0.0; 3])),
        lane(10, "tipped", NORTH, 0.0, false, vt(1.0, [0.0, 90.0, 0.0], [0.0; 3])),
        VtLane {
            scripted: true,
            ..lane(11, "script3", CENTERED, -4.0, false, vt(3.0, [0.0; 3], [0.0; 3]))
        },
        lane(12, "big", BIG, -4.0, false, None),
        lane(13, "door", DOOR, 0.0, false, None),
        lane(14, "door3", DOOR, -DOOR_BESIDE, false, vt(3.0, [0.0; 3], [0.0; 3])),
    ];

    let barrel = Gff::read(&game.resman.get(&ResKey::new(r("plc_barrel"), ResType::UTP)).unwrap())
        .unwrap()
        .root;
    let items = |_: ResRef| -> Option<Struct> { None };
    let placing = Placing { game: &game, item: &items };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    // The generic door, its walkmesh the game's: closed, a box 2.1 m
    // along X and 0.3 m across (`t_door01.dwk`); locked, so that walkers
    // go round it rather than through.
    let door =
        Gff::read(&game.resman.get(&ResKey::new(r("nw_door_ttr_01"), ResType::UTD)).unwrap())
            .unwrap()
            .root;
    let closed = mg_area::walk::Walkmesh::of_object(
        &game,
        "t_door01",
        Some(mg_mdl::walkmesh::DoorState::Closed),
    )
    .unwrap();
    assert!((closed.max.x - DOOR_HALF).abs() < 0.01, "the door: {closed:?}");
    for (i, l) in lanes.iter().enumerate() {
        let (t, mut s) = if l.mesh == DOOR {
            let mut s = door.clone();
            s.set("Locked", Value::Byte(1));
            s.set("KeyRequired", Value::Byte(1));
            s.set("OpenLockDC", Value::Byte(250));
            (ResType::UTD, s)
        } else {
            let mut s = barrel.clone();
            s.set("Appearance", Value::Dword(l.mesh as u32));
            s.set("Static", Value::Byte(l.is_static as u8));
            s.set("Useable", Value::Byte(0));
            (ResType::UTP, s)
        };
        s.set("Tag", Value::String(format!("MG_{i}").into_bytes()));
        s.set("Plot", Value::Byte(1));
        let at = Placement { position: [l.at, 25.0, 0.0], rotation: 0.0 };
        let mut placed = instance(&placing, t, &s, at, &[]).unwrap();
        if let (false, Some(v)) = (l.scripted, l.vt) {
            placed.set("VisTransformList", mg_area::edit::visual_transform_list(v));
        }
        let (list, _) = git_list(t).unwrap();
        let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
        objects.push(placed);
        git.root.set(list, Value::List(objects));
    }
    m.set_gff(git_key, &git).unwrap();
    m.set_gff(ResKey::new(r("mg_walker"), ResType::UTC), &walker(&game)).unwrap();

    let probe = compile(&dir, "mg_probe", &vt_probe(&lanes));
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_vtwalk.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_vtwalk", "MG_DONE", Duration::from_secs(240)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);

    // The server has each placeable's visual transform as the file gives
    // it: what it does to the walkmesh is not for want of reading it.
    let read = run.values("MG_VT");
    for l in &lanes {
        let line = read.iter().find(|v| v.split('|').next() == Some(l.name));
        let line = line.unwrap_or_else(|| panic!("{}: no MG_VT\n{}", l.name, run.log));
        eprintln!("{}: valid|scale|rx|ry|rz|tx|ty = {line}", l.name);
        let f: Vec<f32> = line.split('|').skip(1).map(|v| v.parse().unwrap()).collect();
        assert_eq!(f[0], 1.0, "{}: found by its tag: {line}", l.name);
        let v = l.vt.unwrap_or_default();
        let want = [v.scale.x, v.rotate.x, v.rotate.y, v.rotate.z, v.translate.x, v.translate.y];
        for (got, want) in f[1..].iter().zip(want) {
            assert!((got - want).abs() < 0.01, "{}: {line}, wanted {want}", l.name);
        }
    }

    // Where each walker went, and whether it crossed the squares a walkmesh
    // transformed with its model, or left as it is, would be (x0, x1, y0,
    // y1): a walker goes round a walkmesh, never over it.
    let lane = |name: &str| lanes.iter().find(|l| l.name == name).unwrap();
    let aside: std::collections::HashMap<&str, (f32, f32)> = lanes
        .iter()
        .filter(|l| l.name != "door")
        .map(|l| (l.name, walked(&run, l.name, l.lane, VT_TO)))
        .collect();
    // On open ground a walker goes straight unless a walkmesh is on its
    // lane; and round one about 2.2 m aside (the square's half and a
    // little), where it is.
    for name in
        ["none", "scale1", "scale3", "static3", "script3", "moved", "turned_w", "turned_e", "door3"]
    {
        assert!(aside[name].0 < 0.5, "{name}: {:?} aside", aside[name]);
    }
    for (name, y) in
        [("moved_spot", 25.0), ("north", 31.0), ("turned", 31.0), ("tipped", 31.0), ("big", 25.0)]
    {
        let (by, at) = aside[name];
        let reach = if name == "big" { 8.0 } else { 4.0 };
        assert!(by > 1.0 && (at - y).abs() < reach, "{name}: {by} m aside at y {at}");
    }
    let square = |cx: f32, cy: f32, half: f32| [cx - half, cx + half, cy - half, cy + half];
    let walks_over = |name: &str, rect: [f32; 4]| {
        let path = path(&run, name);
        let over = path.windows(2).any(|s| crosses(s[0], s[1], rect));
        eprintln!("{name}: over {rect:?}: {over}");
        over
    };
    // Unscaled, the square (4 m) is 2 m clear of these lanes; at the
    // model's Scale 3 it would be 12 m across, the lane 2 m inside it.
    // Those beside a Scale 3 walk across where a scaled walkmesh would
    // be: the dynamic placeable's, the static one's and the one a script
    // scaled alike.
    // A walkmesh that big (no transform) is gone round.
    assert!(!walks_over("big", square(lane("big").at, 25.0, 6.0)), "big: walked over");
    for name in ["scale3", "static3", "script3"] {
        let scaled = square(lane(name).at, 25.0, 6.0);
        assert!(walks_over(name, scaled), "{name}: the walkmesh was scaled");
    }
    // TranslateX 4: the walker on the model's spot goes over it, and the
    // one on the placeable's own spot goes round the square there.
    let l = lane("moved");
    assert!(walks_over("moved", square(l.at + 4.0, 25.0, 2.0)), "moved: translated");
    let l = lane("moved_spot");
    assert!(!walks_over("moved_spot", square(l.at, 25.0, 2.0)), "moved_spot: not there");
    // The offset square, 6 m north: gone round with no transform, and so
    // with X Rotation 90 and Y Rotation 90; and the places a turned one
    // would be, 6 m west or east, are walked over.
    for name in ["north", "turned", "tipped"] {
        let l = lane(name);
        assert!(!walks_over(name, square(l.at, 31.0, 2.0)), "{name}: the square moved");
    }
    for (name, by) in [("turned_w", -6.0), ("turned_e", 6.0)] {
        let l = lane(name);
        assert!(walks_over(name, square(l.at + by, 25.0, 2.0)), "{name}: the walkmesh turned");
    }
    // A locked door across the lane is not gone round: the walker's path
    // goes through it (doors are opened), and it stops at the door's face,
    // never past it. One at Scale 3 beside the lane doesn't stop its
    // walker, who walks on past where the scaled door's end would be.
    let door = path(&run, "door");
    let stop = door.last().copied().unwrap();
    eprintln!("door: stopped at {stop:?}");
    assert!(door.iter().all(|p| p.1 < 24.85), "door: walked through: {door:?}");
    assert!(stop.1 > 24.0 && (stop.0 - lane("door").lane).abs() < 0.2, "door: stopped at {stop:?}");
    let closed = |l: &VtLane, k: f32| [l.at - k * DOOR_HALF, l.at + k * DOOR_HALF, 24.85, 25.15];
    let l = lane("door3");
    assert!(walks_over("door3", closed(l, 3.0)), "door3: the walkmesh was scaled");
}

/// Lanes for [`baked_walkmeshes_block_as_drawn`]: its name, the
/// placeable's x (every one at y 25, Bearing 0) and the lane's x.
const BAKED_LANES: [(&str, f32, f32); 6] = [
    // The 12 m square at Scale 0.25 (3 m drawn), baked: walked beside, 4 m
    // east (inside the unbaked square, clear of the baked one), and across.
    ("small_beside", 8.0, 12.0),
    ("small_across", 28.0, 28.0),
    // The square 6 m north at Scale 0.5, X Rotation 90, baked: the 2 m
    // square 3 m west (`VisualTransform::matrix`). Walked across where
    // it is, and across where the unbaked one is (6 m north).
    ("turned_across", 47.0, 44.0),
    ("turned_was", 60.0, 60.0),
    // The 12 m square with no transform, unbaked: its control.
    ("control", 72.0, 76.0),
    // A row only the module's own placeables.2da has (row 0's model).
    ("module_row", 92.0, 92.0),
];

/// Readings 2 and 3 of the walkmesh bake proposal: a baked walkmesh
/// blocks as the model is drawn, and whether a module can carry its own
/// placeables.2da. Two squares of the module's own (placeables.2da rows
/// 1 and 2's models' walkmeshes replaced, as above: one 4 m across with
/// its node 6 m north, one 12 m across) are baked by
/// `mg_module::bake::plan` at Scale 0.25 and at Scale 0.5 with X Rotation
/// 90, and the copies and a placeables.2da with their rows put in the
/// scratch user folder's `override`; the objects keep their transforms.
/// The module itself carries a placeables.2da too: the override's with
/// one more row, row 0's model (a 4 m square).
///
/// Read 2026-11-03: the walkers go straight beside the small one, 4 m
/// east, where the 12 m square would turn them, and round it when they
/// walk across it; round the turned one where the bake put it, 3 m west,
/// and straight over where the square was; and round the unbaked 12 m
/// square at 4 m (the control). The module's own row is not used: its
/// walker goes straight over the spot, so the engine reads no
/// placeables.2da of a module (a hak or the override is needed; the
/// module's file is under its haks and over the override, and the server
/// reads it at none of them).
#[test]
fn baked_walkmeshes_block_as_drawn() {
    use mg_module::bake;
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_baked_walkmeshes");
    let user = dir.join("user");
    let over = user.join("override");
    std::fs::create_dir_all(&over).unwrap();
    let mut game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(&game, "Baked Walkmeshes", &mut rng).unwrap();
    let spec = AreaSpec { name: "Lanes".into(), tileset: r("ttr01"), width: 25, height: 6 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    open_ground(&mut m, &game, area);
    let table = game.table("placeables").unwrap();
    let model = |row: usize| table.get(row, "ModelName").unwrap().to_ascii_lowercase();
    for (row, (at, half)) in MESHES.into_iter().enumerate() {
        m.set(ResKey::new(r(&model(row)), ResType::PWK), square_of(&model(row), at, half));
    }
    // The bake reads the module's walkmeshes, as the window does: the
    // module over the game.
    game.resman.add(
        mg_resman::priority::MODULE,
        mg_resman::MODULE_LABEL,
        mg_resman::LayerClass::Erf,
        m.container(),
    );
    let vt = |scale: f32, turn: f32| VisualTransform {
        scale: Vec3::splat(scale),
        rotate: Vec3::new(turn, 0.0, 0.0),
        translate: Vec3::ZERO,
    };
    let (small, turned) = (vt(0.25, 0.0), vt(0.5, 90.0));
    let request = |row: u32, v: VisualTransform| bake::Request {
        appearance: row,
        transform: mg_module::placement::Transform {
            scale: v.scale.into(),
            rotate: v.rotate.into(),
            translate: v.translate.into(),
        },
        matrix: v.matrix().to_cols_array_2d(),
    };
    let rules = bake::Ready {
        rows: bake::Rows::parse("30000-30009").unwrap(),
        hak: "mg_baked".into(),
        names: "mgbk_".into(),
        rows_file: "baked_rows.toml".into(),
    };
    let codepage = game.codepage();
    let plan = bake::plan(
        &rules,
        &Default::default(),
        &game.resman,
        codepage,
        &[request(BIG as u32, small), request(NORTH as u32, turned)],
    )
    .unwrap();
    eprintln!("{}", plan.describe());
    let bakes: Vec<&bake::Bake> = plan.bakes().collect();
    assert_eq!(bakes.len(), 2, "{:?}", plan.each);
    for (key, data) in &plan.files {
        std::fs::write(over.join(key.to_string()), data).unwrap();
    }
    // Where the baked squares are, as the bake drew them: the small one
    // 3 m across at the origin, the turned one 2 m across 3 m west.
    let bounds = |faces: &[[[f32; 3]; 3]]| {
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for p in faces.iter().flatten() {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        [lo[0], hi[0], lo[1], hi[1]]
    };
    let near = |a: [f32; 4], b: [f32; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3);
    assert!(near(bounds(&bakes[0].after), [-1.5, 1.5, -1.5, 1.5]), "{:?}", bounds(&bakes[0].after));
    assert!(
        near(bounds(&bakes[1].after), [-4.0, -2.0, -1.0, 1.0]),
        "{:?}",
        bounds(&bakes[1].after)
    );
    // The override's table: the game's with the rows the build would add.
    let (under, _) = bake::table_under(&game.resman, codepage).unwrap();
    let with_rows = bake::pending_table(&under, &plan.rows).unwrap();
    let bytes = |t: &mg_2da::TwoDa| t.to_bytes_as(codepage, mg_2da::Layout::Aligned).unwrap();
    std::fs::write(over.join("placeables.2da"), bytes(&with_rows)).unwrap();
    // The module's own: one row more, row 0's model.
    let mut own = with_rows.clone();
    let module_row = own.add_row();
    for c in 0..own.columns().len() {
        let _ = own.set(module_row, c, with_rows.cell(CENTERED, c).map(str::to_string));
    }
    m.set(ResKey::new(r("placeables"), ResType::TWODA), bytes(&own));

    let barrel = Gff::read(&game.resman.get(&ResKey::new(r("plc_barrel"), ResType::UTP)).unwrap())
        .unwrap()
        .root;
    let items = |_: ResRef| -> Option<Struct> { None };
    let placing = Placing { game: &game, item: &items };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTP).unwrap();
    let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
    let placed: [(u32, Option<VisualTransform>); 6] = [
        (bakes[0].row, Some(small)),
        (bakes[0].row, Some(small)),
        (bakes[1].row, Some(turned)),
        (bakes[1].row, Some(turned)),
        (BIG as u32, None),
        (module_row as u32, None),
    ];
    let lanes: Vec<VtLane> = BAKED_LANES
        .iter()
        .zip(placed)
        .map(|(&(name, at, lane), (_, vt))| VtLane {
            name,
            mesh: 0,
            at,
            is_static: false,
            vt,
            scripted: false,
            lane,
        })
        .collect();
    for (i, (row, vt)) in placed.into_iter().enumerate() {
        let mut s = barrel.clone();
        s.set("Tag", Value::String(format!("MG_{i}").into_bytes()));
        s.set("Appearance", Value::Dword(row));
        s.set("Static", Value::Byte(0));
        s.set("Useable", Value::Byte(0));
        s.set("Plot", Value::Byte(1));
        let at = Placement { position: [BAKED_LANES[i].1, 25.0, 0.0], rotation: 0.0 };
        let mut p = instance(&placing, ResType::UTP, &s, at, &[]).unwrap();
        if let Some(v) = vt {
            p.set("VisTransformList", mg_area::edit::visual_transform_list(v));
        }
        objects.push(p);
    }
    git.root.set(list, Value::List(objects));
    m.set_gff(git_key, &git).unwrap();
    m.set_gff(ResKey::new(r("mg_walker"), ResType::UTC), &walker(&game)).unwrap();
    let probe = compile(&dir, "mg_probe", &vt_probe(&lanes));
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_baked.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_baked", "MG_DONE", Duration::from_secs(240)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);

    let lane = |name: &str| *BAKED_LANES.iter().find(|l| l.0 == name).unwrap();
    let walks_over = |name: &str, rect: [f32; 4]| {
        let over = path(&run, name).windows(2).any(|s| crosses(s[0], s[1], rect));
        eprintln!("{name}: over {rect:?}: {over}");
        over
    };
    let square = |cx: f32, cy: f32, half: f32| [cx - half, cx + half, cy - half, cy + half];
    for (name, _, x) in BAKED_LANES {
        walked(&run, name, x, VT_TO);
    }
    // Beside the small one, where the 12 m square would be: walked over.
    let (_, at, _) = lane("small_beside");
    assert!(walks_over("small_beside", square(at, 25.0, 6.0)), "small_beside: not baked");
    // Across it: gone round, never over the baked 3 m square.
    let (_, at, _) = lane("small_across");
    assert!(!walks_over("small_across", square(at, 25.0, 1.5)), "small_across: walked over");
    // The turned one: gone round where the bake put it (3 m west), and the
    // spot the unbaked one has (6 m north) walked over.
    let (_, at, _) = lane("turned_across");
    assert!(!walks_over("turned_across", square(at - 3.0, 25.0, 1.0)), "turned: walked over");
    let (_, at, _) = lane("turned_was");
    assert!(walks_over("turned_was", square(at, 31.0, 2.0)), "turned_was: not baked");
    // The control: the 12 m square, gone round at 4 m.
    let (_, at, _) = lane("control");
    assert!(!walks_over("control", square(at, 25.0, 6.0)), "control: walked over");
    // The module's own row: read 2026-11-03, walked over (the engine reads
    // no placeables.2da of a module's).
    // (The placeable is there, its row the module's alone.)
    let vt = run.values("MG_VT");
    let valid = vt.iter().find(|v| v.starts_with("module_row|")).expect("module_row's MG_VT");
    assert!(valid.starts_with("module_row|1|"), "module_row: no placeable: {valid}");
    let (_, at, _) = lane("module_row");
    let read = !walks_over("module_row", square(at, 25.0, 2.0));
    eprintln!("a module's own placeables.2da read by the engine: {read}");
    assert!(!read, "the engine now reads a module's own placeables.2da");
}

/// Whether the segment from `a` to `b` passes through the rectangle
/// `[x0, x1, y0, y1]` (clipped against its slabs).
fn crosses(a: (f32, f32), b: (f32, f32), [x0, x1, y0, y1]: [f32; 4]) -> bool {
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, d, lo, hi) in [(a.0, b.0 - a.0, x0, x1), (a.1, b.1 - a.1, y0, y1)] {
        if d.abs() < 1e-6 {
            if p < lo || p > hi {
                return false;
            }
            continue;
        }
        let (mut s0, mut s1) = ((lo - p) / d, (hi - p) / d);
        if s0 > s1 {
            std::mem::swap(&mut s0, &mut s1);
        }
        t0 = t0.max(s0);
        t1 = t1.min(s1);
        if t0 > t1 {
            return false;
        }
    }
    true
}

/// Gives every tile of `area` one tile of its tileset whose walkmesh is
/// walkable all over (of those the area was filled with), turned 0: a
/// tile's rocks and trees turn walkers aside as a walkmesh of ours would.
fn open_ground(m: &mut mg_module::Module, game: &GameData, area: ResRef) {
    let are_key = ResKey::new(area, ResType::ARE);
    let mut are = m.gff(&are_key).unwrap().unwrap();
    let tileset = are.root.resref("Tileset").unwrap();
    let data = game.resman.get(&ResKey::new(tileset, ResType::SET)).unwrap();
    let set = mg_set::Tileset::parse(&data, game.language.codepage()).unwrap();
    let surfaces = game.table("surfacemat").unwrap();
    let walkable = |m: u32| surfaces.get(m as usize, "Walk").is_some_and(|w| w == "1");
    let mut ids: Vec<i32> = are
        .root
        .list("Tile_List")
        .unwrap()
        .iter()
        .filter_map(|t| t.integer("Tile_ID").map(|i| i as i32))
        .collect();
    ids.sort();
    ids.dedup();
    let open = ids
        .into_iter()
        .find(|&id| {
            let tile = &set.tiles[id as usize];
            mg_area::walk::Walkmesh::of_tile(game, &tile.model).is_some_and(|w| {
                !w.faces.is_empty() && w.faces.iter().all(|f| walkable(f.material))
            })
        })
        .expect("a tile walkable all over");
    eprintln!("open ground: tile {open} ({})", set.tiles[open as usize].model);
    let Some(Value::List(tiles)) = are.root.get_mut("Tile_List") else { panic!("no Tile_List") };
    for t in tiles.iter_mut() {
        t.set("Tile_ID", Value::Int(open));
        t.set("Tile_Orientation", Value::Int(0));
    }
    m.set_gff(are_key, &are).unwrap();
}
