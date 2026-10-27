//! Tilesets with holes, as old hand-made ones have: the game reads them
//! tile by tile. A tile whose `[TILEn]` section is missing, or whose
//! section has no `Model=`, has no walkmesh, as a tile whose model doesn't
//! exist has none, and the rest of the area is as with the whole tileset;
//! a model named in capitals is the same model; missing terrain, crosser
//! and rule sections change nothing the server reads. A tileset whose
//! lines end in a bare LF, not CRLF, stops the module loading: the server
//! never gets past "Loading Module".

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::Value;
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{ServerRun, run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::compile;

/// Logs each area's ResRef and, for each of its four tiles, the surface
/// material at its centre (where the tile has no walkmesh, what the
/// engine says then).
const PROBE: &str = r#"
void main()
{
    object a = GetFirstArea();
    while (GetIsObjectValid(a))
    {
        string s = GetResRef(a);
        int i;
        for (i = 0; i < 4; i++)
        {
            vector v = Vector(5.0 + 10.0 * IntToFloat(i % 2), 5.0 + 10.0 * IntToFloat(i / 2), 0.0);
            s += " " + IntToString(GetSurfaceMaterial(Location(a, v, 0.0)));
        }
        WriteTimestampedLogEntry("MG_TILES " + s);
        a = GetNextArea();
    }
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

/// The tiles of each area: three of ttr01's water tiles (whose centres
/// read otherwise than a tile with no walkmesh) and its first.
const TILES: [i32; 4] = [20, 101, 102, 0];

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// `set` without its `[NAME]` section, header and all.
fn without_section(set: &str, name: &str) -> String {
    let header = format!("[{name}]\r\n");
    let at = set.find(&header).unwrap_or_else(|| panic!("no {header}"));
    let rest = at + header.len();
    let end = set[rest..].find("\r\n[").map_or(set.len(), |e| rest + e + 2);
    format!("{}{}", &set[..at], &set[end..])
}

/// A module of an area of [`TILES`] for each tileset named (those given
/// a text in a hak), run in the server.
fn run(
    dir: &std::path::Path,
    sets: &[(&str, String)],
    timeout: Duration,
) -> (ServerRun, Vec<String>) {
    let root = mg_testkit::nwn_root().expect("the game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
    for (name, text) in sets.iter().filter(|(_, text)| !text.is_empty()) {
        hak.add(r(name), ResType::SET, text.clone().into_bytes()).unwrap();
    }
    std::fs::write(user.join("hak/mg_holes.hak"), hak.to_bytes().unwrap()).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Holes", &mut rng).unwrap();
    let mut areas = Vec::new();
    for (name, _) in sets {
        let spec = AreaSpec { name: (*name).into(), tileset: r("ttr01"), width: 2, height: 2 };
        let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
        areas.push(area.to_string());
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        are.root.set("Tileset", Value::ResRef(name.as_bytes().to_vec()));
        let tiles = are.root.list_mut("Tile_List").expect("its tiles");
        for (t, id) in tiles.iter_mut().zip(TILES) {
            t.set("Tile_ID", Value::Int(id));
            t.set("Tile_Height", Value::Int(0));
            t.set("Tile_Orientation", Value::Int(0));
        }
        m.set_gff(key, &are).unwrap();
    }
    let probe = compile(dir, "mg_probe", PROBE);
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    let mut entry = mg_gff::Struct::new(8);
    entry.set("Mod_Hak", Value::String(b"mg_holes".to_vec()));
    info.root.set("Mod_HakList", Value::List(vec![entry]));
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_holes.mod"))).unwrap();
    (run_server(&root, &user, "mg_holes", "MG_DONE", timeout).unwrap(), areas)
}

/// The game's ttr01.set (CRLF line ends); `None` without a server.
fn ttr01(root: &std::path::Path) -> Option<String> {
    if server_binary(root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return None;
    }
    let game = GameData::open(&GameInstall::new(root, None, "en")).unwrap();
    let set = game.resman.get(&ResKey::new(r("ttr01"), ResType::SET)).unwrap();
    let set = String::from_utf8_lossy(&set).into_owned();
    assert!(set.contains("[TILE20]\r\nModel=ttr01_b20_01\r\n"), "ttr01's tile 20 is its water");
    assert!(set.contains("Model=ttr01_k01_01\r\n") && set.contains("Model=ttr01_k02_01\r\n"));
    Some(set)
}

#[test]
fn the_game_reads_a_tileset_with_holes_tile_by_tile() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let Some(ttr01) = ttr01(&root) else { return };
    // Holes: no [TILE20], [TILE101] without its Model=, [TILE102]'s model
    // in capitals, no [TERRAIN1], [CROSSER0] or [PRIMARY RULE0].
    let mut holed = without_section(&ttr01, "TILE20");
    holed = holed.replacen("Model=ttr01_k01_01\r\n", "", 1);
    holed = holed.replacen("Model=ttr01_k02_01\r\n", "Model=TTR01_K02_01\r\n", 1);
    for s in ["TERRAIN1", "CROSSER0", "PRIMARY RULE0"] {
        holed = without_section(&holed, s);
    }
    // The measure of a tile with no walkmesh: models that don't exist.
    let none = ttr01.replacen("Model=ttr01_b20_01\r\n", "Model=mg_no_such1\r\n", 1).replacen(
        "Model=ttr01_k01_01\r\n",
        "Model=mg_no_such2\r\n",
        1,
    );
    let dir = scratch_dir("engine_tileset_holes");
    let sets = [("ttr01", String::new()), ("mghole", holed), ("mgnone", none)];
    let (run, areas) = run(&dir, &sets, Duration::from_secs(120));
    assert!(run.finished, "the server didn't load the module:\n{}", run.log);
    let tiles = run.values("MG_TILES");
    let of = |area: &str| -> Vec<i32> {
        let line = tiles
            .iter()
            .find(|l| l.split(' ').next().is_some_and(|a| a.eq_ignore_ascii_case(area)))
            .unwrap_or_else(|| panic!("no {area} in {tiles:?}"));
        line.split(' ').skip(1).map(|v| v.parse().unwrap()).collect()
    };
    let (whole, holed, none) = (of(&areas[0]), of(&areas[1]), of(&areas[2]));
    let seen = format!("ttr01 {whole:?}, with holes {holed:?}, models missing {none:?}");
    assert!(none[0] != whole[0] && none[1] != whole[1], "the probe tells them apart: {seen}");
    // No section, no Model=: no walkmesh; the model in capitals and the
    // tile without a hole as they were.
    assert_eq!(holed, [none[0], none[1], whole[2], whole[3]], "{seen}");
}

#[test]
fn a_tileset_whose_lines_end_in_lf_stops_the_module_loading() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let Some(ttr01) = ttr01(&root) else { return };
    let dir = scratch_dir("engine_tileset_lf");
    // (The same tileset with CRLF: the module loads.)
    let sets = [("mgcrlf", ttr01.clone())];
    let started = std::time::Instant::now();
    let (run_crlf, _) = run(&dir.join("crlf"), &sets, Duration::from_secs(120));
    assert!(run_crlf.finished, "with CRLF:\n{}", run_crlf.log);
    // A module loads in a second here; three times the whole CRLF run
    // (starting the server, loading, the probe's script) is no load at all.
    // (A fixed minute made this the corpus tests' longest by 50 s.)
    let no_load = (started.elapsed() * 3).max(Duration::from_secs(20));
    let sets = [("mglf", ttr01.replace("\r\n", "\n"))];
    let (run_lf, _) = run(&dir.join("lf"), &sets, no_load);
    assert!(!run_lf.finished, "the module loaded:\n{}", run_lf.log);
    assert!(run_lf.log.contains("Loading Module"), "{}", run_lf.log);
}
