//! Painted terrain, in the engine: an area painted with Moonglow's brushes
//! (`mg_tiles::paint`, written by `mg_area::terrain`) loads in `nwserver`
//! with the ground where the strokes put it: a corner raised twice stands
//! two height steps up (the corners around it one), a pond painted with
//! four water corners is not grass, and a placed tile group's doors are
//! there.

use std::time::Duration;

use mg_area::terrain::{Brush, TilesetPalette, door_edits, grid, tile_edits};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Workspace};
use mg_gff::Gff;
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};
use mg_tiles::TileIndex;
use mg_tiles::paint::{Grid, Rules, Stroke};

mod common;
use common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

#[test]
fn painted_terrain_stands_where_the_engine_puts_it() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_terrain");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(&game, "Painted", &mut rng).unwrap();
    let spec = AreaSpec { name: "Painted".into(), tileset: r("ttr01"), width: 6, height: 6 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let (are_key, git_key) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let set = mg_area::tileset(&game, r("ttr01")).unwrap();
    let index = TileIndex::new(&set);
    let rules = Rules::new(&index, &set);
    let palette = TilesetPalette::read(&game, r("ttr01"), &set, &index);
    let water = index.terrain("Water").unwrap();
    let Some(Brush::Group(barn)) = palette.brush("Barn 1 2x2") else { panic!("no barn") };

    // The strokes, each one command as the viewer makes them.
    let mut ws = Workspace::new(m);
    enum Do {
        Raise(u32, u32),
        Water(u32, u32),
        Barn(u32, u32),
    }
    let strokes = [
        Do::Raise(4, 4),
        Do::Raise(4, 4),
        Do::Water(1, 4),
        Do::Water(2, 4),
        Do::Water(1, 5),
        Do::Water(2, 5),
        Do::Barn(0, 0),
    ];
    let stroke = |g: &Grid, d: &Do| -> Option<Stroke> {
        match *d {
            Do::Raise(x, y) => g.raise(&index, &rules, x, y, true),
            Do::Water(x, y) => g.paint(&index, &rules, x, y, water),
            Do::Barn(x, y) => g.place_group(&index, &set.groups[barn], x, y, 0),
        }
    };
    let read = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTD)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    for (i, d) in strokes.iter().enumerate() {
        let label = format!("Stroke {i}");
        let are = ws.doc(&are_key).unwrap().root.clone();
        let git = ws.doc(&git_key).unwrap().root.clone();
        let mut g = grid(&are, &index).unwrap();
        let before = g.clone();
        let s = stroke(&g, d).unwrap_or_else(|| panic!("{label}: refused"));
        let changes = g.apply(&index, s, &mut rng);
        let mut edits = tile_edits(are_key, &are, &set, &changes, &mut || [0, 0, 0]);
        let old = |(x, y): (u32, u32)| before.tile(x, y);
        let step = set.general.transition;
        edits.extend(door_edits(&game, git_key, &git, &set, step, &old, &changes, &read));
        ws.apply(Command::new(label, edits)).unwrap();
    }
    ws.flush().unwrap();
    let mut m = ws.module;
    let tag = m
        .gff(&are_key)
        .unwrap()
        .unwrap()
        .root
        .string("Tag")
        .map(|t| String::from_utf8_lossy(t).into_owned())
        .unwrap();

    let probe = format!(
        r#"
string F(float f) {{ return FloatToString(f, 0, 2); }}

void main()
{{
    object a = GetObjectByTag("{tag}");
    WriteTimestampedLogEntry("MG_AT top|" + F(GetGroundHeight(Location(a, Vector(40.0, 40.0, 0.0), 0.0))));
    WriteTimestampedLogEntry("MG_AT ring|" + F(GetGroundHeight(Location(a, Vector(30.0, 30.0, 0.0), 0.0))));
    WriteTimestampedLogEntry("MG_AT flat|" + F(GetGroundHeight(Location(a, Vector(55.0, 15.0, 0.0), 0.0))));
    WriteTimestampedLogEntry("MG_AT pond|" + IntToString(GetSurfaceMaterial(Location(a, Vector(15.0, 45.0, 0.0), 0.0))));
    WriteTimestampedLogEntry("MG_AT grass|" + IntToString(GetSurfaceMaterial(Location(a, Vector(55.0, 15.0, 0.0), 0.0))));
    int n;
    for (n = 0; n < 3; n++)
    {{
        object d = GetObjectByTag("Barn1Door", n);
        vector v = GetPosition(d);
        WriteTimestampedLogEntry("MG_AT door|" + IntToString(GetIsObjectValid(d)) + "|" + F(v.x) + "|" + F(v.y));
    }}
    WriteTimestampedLogEntry("MG_DONE");
}}
"#
    );
    let ncs = compile(&dir, "mg_probe", &probe);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), ncs);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_painted.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_painted", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);
    let got = run.values("MG_AT");
    // Two steps of 5 m up at the top, one around it, none far away; grass
    // (surfacemat.2da row 3) on flat ground and not in the pond (deep
    // water has no walkmesh face); the barn's two doors, on the hooks of
    // tiles 144 and 146 (as Aurora placed them, `aurora_terrain.rs`).
    let want = [
        "top|10.00",
        "ring|5.00",
        "flat|0.00",
        "grass|3",
        "door|1|3.21|13.49",
        "door|1|16.80|13.50",
        "door|0|0.00|0.00",
    ];
    for e in want {
        assert!(
            got.contains(&e.to_string()),
            "expected {e}; the engine reported:\n{}",
            got.join("\n")
        );
    }
    assert!(!got.contains(&"pond|3".to_string()), "the pond is grass:\n{}", got.join("\n"));
}
