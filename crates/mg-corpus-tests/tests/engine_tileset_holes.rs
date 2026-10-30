//! Tilesets with holes, as old hand-made ones have: the game reads them
//! tile by tile. A tile whose `[TILEn]` section is missing, or whose
//! section has no `Model=`, has no walkmesh, as a tile whose model doesn't
//! exist has none, and the rest of the area is as with the whole tileset;
//! a model named in capitals is the same model; missing terrain, crosser
//! and rule sections change nothing the server reads. A tileset whose
//! lines end in a bare LF, not CRLF, stops the module loading (the server
//! never gets past "Loading Module") where its last 256 bytes hold no CR.

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

/// `set` with the lines of its `[NAME]` section (header and all) ending in
/// a bare LF, the rest as they were.
fn section_in_lf(set: &str, name: &str) -> String {
    let header = format!("[{name}]\r\n");
    let at = set.find(&header).unwrap_or_else(|| panic!("no {header}"));
    let rest = at + header.len();
    let end = set[rest..].find("\r\n[").map_or(set.len(), |e| rest + e + 2);
    format!("{}{}{}", &set[..at], set[at..end].replace("\r\n", "\n"), &set[end..])
}

/// `set` with its last `n` lines ending in a bare LF.
fn last_lf(set: &str, n: usize) -> String {
    let mut at = set.len();
    for _ in 0..n {
        at = set[..at - 1].rfind('\n').unwrap_or(0);
    }
    format!("{}{}", &set[..at], set[at..].replace("\r\n", "\n"))
}

/// A tileset whose lines end both ways, and long lines. The server
/// refuses (never finishes loading the module) a file with a line of more
/// than 255 characters (its CR and LF not counted), wherever it is and
/// however it ends; and one with more than 256 bytes after its last CRLF
/// (a file all LF is one). Other lines in LF load and read as with CRLF:
/// at its head, in its middle (a section, every tile's section: thousands
/// of lines), before a last CRLF line however many. Verify says so
/// (`set-line-ends`: an error for each of the two, a warning for a mix the
/// server reads).
#[test]
fn a_tileset_whose_lines_end_both_ways() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let Some(ttr01) = ttr01(&root) else { return };
    let dir = scratch_dir("engine_tileset_mixed");
    let started = std::time::Instant::now();
    let (base, _) = run(&dir.join("crlf"), &[("mgcrlf", ttr01.clone())], Duration::from_secs(120));
    assert!(base.finished, "with CRLF:\n{}", base.log);
    let no_load = (started.elapsed() * 3).max(Duration::from_secs(20));
    let tiles_lf = {
        let (a, b) = (ttr01.find("[TILE0]").unwrap(), ttr01.find("[GROUPS]").unwrap());
        format!("{}{}{}", &ttr01[..a], ttr01[a..b].replace("\r\n", "\n"), &ttr01[b..])
    };
    let head_lf = {
        let a = ttr01.find("[TILE0]").unwrap();
        format!("{}{}", ttr01[..a].replace("\r\n", "\n"), &ttr01[a..])
    };
    let first_crlf = ttr01.replace("\r\n", "\n").replacen('\n', "\r\n", 1);
    // After the file, a section of ours: `n` short lines ("X=1"), or one
    // line of `n` characters, each ending in `end`.
    let lines = |n: usize, end: &str| format!("X=1{end}").repeat(n);
    let one = |n: usize, end: &str| format!("X={}{end}", "1".repeat(n - 2));
    let pad = |tail: String| format!("{ttr01}[MGPAD]\r\n{tail}");
    let in_middle = |line: String| {
        let a = ttr01.find("[GROUPS]").unwrap();
        format!("{}[MGPAD]\r\n{line}{}", &ttr01[..a], &ttr01[a..])
    };
    let cases = [
        // Lines in LF: where.
        ("mgtile20", section_in_lf(&ttr01, "TILE20"), true),
        ("mgtiles", tiles_lf, true),
        ("mghead", head_lf, true),
        ("mgfirst", first_crlf, false),
        // After the last CRLF: 251 and 256 bytes, 257 and 264.
        ("mglast27", last_lf(&ttr01, 27), true),
        ("mglast28", last_lf(&ttr01, 28), false),
        ("mgshort64", pad(lines(64, "\n")), true),
        ("mgshort65", pad(lines(65, "\n")), false),
        ("mgone255", pad(one(255, "\n")), true),
        ("mgone256", pad(one(256, "\n")), false),
        // Before a last CRLF line: 600 bytes in LF.
        ("mgbefore", pad(lines(150, "\n") + "Y=1\r\n"), true),
        // A long line, however it ends, wherever it is.
        ("mgcrlf255", pad(one(255, "\r\n")), true),
        ("mgcrlf256", pad(one(256, "\r\n")), false),
        ("mgmid300", in_middle(one(300, "\r\n")), false),
        ("mgmidlf387", in_middle(one(387, "\n")), false),
    ];
    // (Each in a server of its own, side by side: a module that doesn't
    // load is waited on.)
    let seen = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for (name, text, loads) in &cases {
            let (dir, seen, ttr01) = (&dir, &seen, &ttr01);
            s.spawn(move || {
                let after = text.rfind("\r\n").map_or(text.len(), |at| text.len() - at - 2);
                let longest = text.split('\n').map(|l| l.trim_end_matches('\r').len()).max();
                let sets = [("ttr01", String::new()), (*name, text.clone())];
                let (r, areas) = run(&dir.join(name), &sets, no_load);
                let mut said = format!(
                    "{name} ({after} bytes after the last CRLF, the longest line {}): loaded {}",
                    longest.unwrap_or(0),
                    r.finished
                );
                if r.finished && *loads {
                    // Read as with CRLF: the area's tiles as ttr01's.
                    let tiles = r.values("MG_TILES");
                    let of =
                        |a: &str| tiles.iter().find(|l| l.starts_with(&format!("{a} "))).cloned();
                    let rest =
                        |l: Option<String>| l.map(|l| l.split_once(' ').unwrap().1.to_string());
                    let (whole, this) = (rest(of("ttr01")), rest(of(&areas[1])));
                    if whole.is_none() || whole != this {
                        said += &format!("\n  ^ its tiles {this:?}, ttr01's {whole:?}");
                    }
                }
                if r.finished != *loads {
                    said += "\n  ^ not as expected";
                }
                let _ = ttr01;
                seen.lock().unwrap().push(said);
            });
        }
    });
    let mut seen = seen.into_inner().unwrap();
    seen.sort();
    let wrong = seen.iter().any(|l| l.contains("  ^"));
    assert!(!wrong, "{}", seen.join("\n"));
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
