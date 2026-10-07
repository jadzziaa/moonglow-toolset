//! Terrain painting compared with Aurora: the scenarios of
//! `data/terrain_scenarios.json`, painted in Aurora by
//! `tools/aurora/capture_terrain.py` (the module saved after every step),
//! replayed step by step with Moonglow's brushes (`mg_tiles::paint`).
//!
//! Each step starts from Aurora's area before it; the corners (terrain and
//! height) and crossers Moonglow's stroke leaves must equal Aurora's after
//! it, refused strokes included, and every tile Aurora changed must be one
//! the stroke chose again. Tiles themselves are chosen at random, so only
//! what they make is compared. The palette's Terrain branch must list what
//! Aurora's lists.

use std::path::Path;

use mg_area::terrain::{Brush, PaletteItem, TilesetPalette, grid};
use mg_core::{ResRef, ResType};
use mg_module::Module;
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::corpus;
use mg_tiles::paint::{Grid, Rules, Stroke, path_edges};
use mg_tiles::{Lattice, TileIndex};
use serde_json::Value;

fn scenarios() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/terrain_scenarios.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn area_grid(module: &Path, area: &str, index: &TileIndex) -> Grid {
    let m = Module::open(module).unwrap();
    let key = ResKey::new(ResRef::from_str(area).unwrap(), ResType::ARE);
    let are = m.gff(&key).unwrap().expect("the capture has the area");
    grid(&are.root, index).expect("the capture's tiles are the tileset's")
}

/// A lattice as text, north up: corners as terrain letter and height,
/// crossers as their first letter on the edges.
fn render(index: &TileIndex, l: &Lattice) -> String {
    let mut out = String::new();
    let name = |c: mg_tiles::Crosser| index.crosser_name(c).chars().next().unwrap_or('?');
    for y in (0..=l.height()).rev() {
        for x in 0..=l.width() {
            let c = l.corner(x, y);
            let t: String = index.terrain_name(c.terrain).chars().take(2).collect();
            out += &format!("{t:<2}{}", c.height);
            if x < l.width() {
                let edge = if y < l.height() {
                    l.cell(x, y).edges[mg_tiles::SOUTH]
                } else {
                    l.cell(x, y - 1).edges[mg_tiles::NORTH]
                };
                out += &format!(" {} ", edge.map_or('-', name));
            }
        }
        out += "\n";
        if y > 0 {
            for x in 0..=l.width() {
                let edge = if x < l.width() {
                    l.cell(x, y - 1).edges[mg_tiles::WEST]
                } else {
                    l.cell(x - 1, y - 1).edges[mg_tiles::EAST]
                };
                out += &format!(" {}    ", edge.map_or('|', name));
            }
            out += "\n";
        }
    }
    out
}

fn terrain_labels(palette: &TilesetPalette) -> Vec<String> {
    let terrain = palette.branches.iter().find_map(|b| match b {
        PaletteItem::Folder { items, .. }
            if items
                .iter()
                .any(|i| matches!(i, PaletteItem::Brush { brush: Brush::Eraser, .. })) =>
        {
            Some(items)
        }
        _ => None,
    });
    terrain.map_or_else(Vec::new, |items| items.iter().map(|i| i.label().to_string()).collect())
}

/// What one action of a step does to `grid`.
fn act(
    index: &TileIndex,
    rules: &Rules,
    set: &mg_set::Tileset,
    palette: &TilesetPalette,
    grid: &Grid,
    step: &Value,
    area: &str,
) -> Option<Stroke> {
    let at = |v: &Value| (v[0].as_u64().unwrap() as u32, v[1].as_u64().unwrap() as u32);
    let label = step["brush"].as_str().or_else(|| step["group"].as_str()).unwrap();
    let brush = palette.brush(label).unwrap_or_else(|| panic!("{area}: no brush {label}"));
    match brush {
        Brush::Group(g) => {
            let (x, y) = at(&step["cell"]);
            let turns = step["turns"].as_u64().unwrap_or(0) as u8;
            grid.place_group(index, &set.groups[g], x, y, turns)
        }
        Brush::Crosser(c) => {
            let path: Vec<(u32, u32)> = step["path"].as_array().unwrap().iter().map(at).collect();
            let edges = path_edges(&path).expect("a path of neighbouring cells");
            grid.draw_crosser(index, &edges, &path[..1], c)
        }
        Brush::Eraser => {
            let (x, y) = at(&step["cell"]);
            grid.erase(index, x, y)
        }
        Brush::Terrain(t) => {
            let (x, y) = at(&step["at"]);
            grid.paint(index, rules, x, y, t)
        }
        Brush::RaiseLower => {
            let (x, y) = at(&step["at"]);
            grid.raise(index, rules, x, y, !step["lower"].as_bool().unwrap_or(false))
        }
        Brush::Refine => panic!("{area}: Refine Tile is Moonglow's, in no tileset's palette"),
    }
}

#[test]
fn painting_matches_aurora() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let spec = scenarios();
    let mut failures = Vec::new();
    let mut steps = 0;
    for scenario in spec["scenarios"].as_array().unwrap() {
        let area = scenario["area"].as_str().unwrap();
        let name = scenario["captures"].as_str().unwrap_or(area);
        let capture = |i: usize| mg_testkit::aurora_capture(&format!("terrain/{name}/{i:02}.mod"));
        let Some(first) = capture(0) else {
            assert!(!mg_testkit::corpus_required(), "Aurora capture terrain/{name} not found");
            eprintln!("skipped {name}: no Aurora capture (tools/aurora/capture_terrain.py)");
            continue;
        };
        let resref = ResRef::from_str(area).unwrap();
        let set = mg_area::tileset(&game, resref).unwrap();
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let palette = TilesetPalette::read(&game, resref, &set, &index);
        let expected: Vec<String> = scenario["palette"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l.as_str().unwrap().to_string())
            .collect();
        assert_eq!(terrain_labels(&palette), expected, "{area}: the Terrain palette");

        let mut before = area_grid(&first, area, &index);
        for (i, step) in scenario["steps"].as_array().unwrap().iter().enumerate() {
            let Some(path) = capture(i + 1) else {
                failures.push(format!("{area} step {}: no capture", i + 1));
                break;
            };
            let after = area_grid(&path, area, &index);
            let (_, bad) = Lattice::from_tiles(
                &index,
                after.lattice.width(),
                after.lattice.height(),
                &after.tiles,
            )
            .unwrap();
            if !bad.is_empty() {
                failures.push(format!("{area} step {}: Aurora's tiles disagree at {bad:?}", i + 1));
            }
            // The step's actions (several when they were saved as one),
            // each applied to the grid in turn; `None` when the last was
            // refused.
            let actions: Vec<&Value> = match step["steps"].as_array() {
                Some(list) => list.iter().collect(),
                None => vec![step],
            };
            let mut grid = before.clone();
            let mut stroke = None;
            let mut cells = Vec::new();
            for action in actions {
                stroke = act(&index, &rules, &set, &palette, &grid, action, area);
                if let Some(s) = &stroke {
                    cells.extend(s.cells.iter().copied());
                    cells.extend(s.fixed.iter().map(|(c, _)| *c));
                    grid.apply(&index, s.clone(), &mut fastrand::Rng::with_seed(1));
                }
            }
            steps += 1;
            let ours = &grid.lattice;
            let what = format!("{name} step {} ({step})", i + 1);
            if *ours != after.lattice {
                failures.push(format!(
                    "{what}{}:\nMoonglow:\n{}Aurora:\n{}",
                    if stroke.is_none() { ", refused by Moonglow" } else { "" },
                    render(&index, ours),
                    render(&index, &after.lattice)
                ));
            } else {
                // Every tile Aurora chose again is one the stroke chooses.
                let w = after.lattice.width();
                let changed: Vec<(u32, u32)> = (0..after.tiles.len() as u32)
                    .filter(|&i| after.tiles[i as usize] != before.tiles[i as usize])
                    .map(|i| (i % w, i / w))
                    .filter(|c| !cells.contains(c))
                    .collect();
                if !changed.is_empty() {
                    failures.push(format!("{what}: Aurora also changed the tiles of {changed:?}"));
                }
            }
            before = after;
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {steps} steps differ:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
    eprintln!("{steps} steps as Aurora paints them");
}

/// Resize Area and Rotate Area against Aurora's (captured by hand in the
/// probe module's ttr01 area: grown from 10 by 10 to 13 by 12, grown a
/// column past water on the east edge, shrunk to 9 by 8 through three tile
/// groups, rotated a quarter turn counter-clockwise).
#[test]
fn resize_and_rotate_match_aurora() {
    use mg_area::reshape::{resize, rotate};
    use mg_edit::{Command, Workspace};
    let root = corpus!();
    let names = ["resize-00", "resize-01-grow", "resize-02-water", "resize-03-grow-east"];
    let names = names.iter().chain(&["resize-04-shrink", "resize-05-rotate-ccw90"]);
    let mut paths = Vec::new();
    for n in names {
        let Some(p) = mg_testkit::aurora_capture(&format!("terrain/{n}.mod")) else {
            assert!(!mg_testkit::corpus_required(), "Aurora capture terrain/{n} not found");
            eprintln!("skipped: no Aurora capture terrain/{n}");
            return;
        };
        paths.push(p);
    }
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let area = ResRef::from_str("ttr01").unwrap();
    let (are_key, git_key) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let set = mg_area::tileset(&game, area).unwrap();
    let index = TileIndex::new(&set);
    let docs = |p: &Path| {
        let m = Module::open(p).unwrap();
        (m.gff(&are_key).unwrap().unwrap().root, m.gff(&git_key).unwrap().unwrap().root)
    };
    // Applies the edits to the module at `from` and returns its ARE and GIT.
    let after = |from: &Path, edits: Vec<mg_edit::Edit>| {
        let mut ws = Workspace::new(Module::open(from).unwrap());
        ws.apply(Command::new("Reshape", edits)).unwrap();
        (ws.doc(&are_key).unwrap().root.clone(), ws.doc(&git_key).unwrap().root.clone())
    };
    let doors = |git: &mg_gff::Struct| -> Vec<[f32; 4]> {
        git.list("Door List")
            .unwrap_or(&[])
            .iter()
            .map(|d| ["X", "Y", "Z", "Bearing"].map(|l| d.float(l).unwrap()))
            .collect()
    };
    let close = |a: &[[f32; 4]], b: &[[f32; 4]]| {
        a.len() == b.len()
            && a.iter().zip(b).all(|(p, q)| p.iter().zip(q).all(|(u, v)| (u - v).abs() < 1e-3))
    };
    let mut rng = fastrand::Rng::with_seed(4);
    for (from, to, size) in [(0, 1, (13, 12)), (2, 3, (14, 12)), (3, 4, (9, 8))] {
        let (are, git) = docs(&paths[from]);
        let (aurora_are, aurora_git) = docs(&paths[to]);
        let r = resize(
            are_key,
            git_key,
            &are,
            &git,
            &set,
            &index,
            size.0,
            size.1,
            &mut rng,
            &mut || [0; 3],
        )
        .unwrap();
        let (ours_are, ours_git) = after(&paths[from], r.edits);
        let (ours, theirs) = (grid(&ours_are, &index).unwrap(), grid(&aurora_are, &index).unwrap());
        assert_eq!(
            ours.lattice,
            theirs.lattice,
            "{:?}: Moonglow:\n{}Aurora:\n{}",
            size,
            render(&index, &ours.lattice),
            render(&index, &theirs.lattice)
        );
        // The tiles Aurora kept, Moonglow keeps.
        let old = grid(&are, &index).unwrap();
        for y in 0..size.1.min(old.lattice.height()) {
            for x in 0..size.0.min(old.lattice.width()) {
                if theirs.tile(x, y) == old.tile(x, y) {
                    assert_eq!(ours.tile(x, y), old.tile(x, y), "{size:?}: tile ({x}, {y})");
                }
            }
        }
        assert!(
            close(&doors(&ours_git), &doors(&aurora_git)),
            "{size:?}: doors {:?}",
            doors(&ours_git)
        );
    }
    // Rotation: tiles and doors exactly where Aurora puts them.
    let (are, git) = docs(&paths[4]);
    let (aurora_are, aurora_git) = docs(&paths[5]);
    let (ours_are, ours_git) = after(&paths[4], rotate(are_key, git_key, &are, &git, 1));
    for label in ["Width", "Height", "Tile_List"] {
        assert_eq!(ours_are.get(label), aurora_are.get(label), "{label}");
    }
    assert!(close(&doors(&ours_git), &doors(&aurora_git)), "doors {:?}", doors(&ours_git));
}

/// The doors a placed group brings, against Aurora's (the first barn of
/// the groups scenario: two doors on its typed hooks), field for field.
#[test]
fn group_doors_match_aurora() {
    let root = corpus!();
    let (Some(start), Some(placed)) = (
        mg_testkit::aurora_capture("terrain/ttr01-groups/00.mod"),
        mg_testkit::aurora_capture("terrain/ttr01-groups/01.mod"),
    ) else {
        assert!(!mg_testkit::corpus_required(), "Aurora capture terrain/ttr01-groups not found");
        eprintln!("skipped: no Aurora capture terrain/ttr01-groups");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let area = ResRef::from_str("ttr01").unwrap();
    let (are_key, git_key) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let set = mg_area::tileset(&game, area).unwrap();
    let index = TileIndex::new(&set);
    let m = Module::open(&start).unwrap();
    let (are, git) =
        (m.gff(&are_key).unwrap().unwrap().root, m.gff(&git_key).unwrap().unwrap().root);
    let mut g = grid(&are, &index).unwrap();
    let before = g.clone();
    let palette = TilesetPalette::read(&game, area, &set, &index);
    let Some(Brush::Group(barn)) = palette.brush("Barn 1 2x2") else { panic!("no barn") };
    let stroke = g.place_group(&index, &set.groups[barn], 5, 5, 0).unwrap();
    let changes = g.apply(&index, stroke, &mut fastrand::Rng::with_seed(1));
    let read = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTD)).ok()?;
        mg_gff::Gff::read(&data).ok().map(|g| g.root)
    };
    let old = |(x, y): (u32, u32)| before.tile(x, y);
    let edits = mg_area::terrain::door_edits(
        &game,
        git_key,
        &git,
        &set,
        set.general.transition,
        &old,
        &changes,
        &read,
    );
    let ours: Vec<mg_gff::Struct> = edits
        .into_iter()
        .filter_map(|e| match e {
            mg_edit::Edit::InsertItem { item, .. } => Some(item),
            _ => None,
        })
        .collect();
    let aurora = Module::open(&placed).unwrap().gff(&git_key).unwrap().unwrap().root;
    let theirs = aurora.list("Door List").unwrap();
    assert_eq!(ours.len(), theirs.len());
    for (o, t) in ours.iter().zip(theirs) {
        let labels = |s: &mg_gff::Struct| -> Vec<String> {
            s.fields.iter().map(|f| f.label.to_string_lossy()).collect()
        };
        assert_eq!(labels(o), labels(t), "field order");
        for f in &t.fields {
            let label = f.label.to_string_lossy();
            match (&f.value, o.get(&label)) {
                (mg_gff::Value::Float(a), Some(mg_gff::Value::Float(b))) => {
                    assert!((a - b).abs() < 1e-3, "{label}: Aurora {a}, Moonglow {b}")
                }
                (a, b) => assert_eq!(Some(a), b, "{label}"),
            }
        }
    }
}

/// A group is one thing: the Eraser on any of its tiles takes all of it
/// away (as in Aurora, driven by hand on the oracle), and a group placed
/// over part of another takes the other away whole (no half a barn left
/// beside the new one). Aurora refuses that placement instead (the
/// outline turns red, the click places nothing): replacing in place was
/// chosen over it.
#[test]
fn groups_go_whole() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let area = ResRef::from_str("ttr01").unwrap();
    let set = mg_area::tileset(&game, area).unwrap();
    let index = TileIndex::new(&set);
    let grass = mg_tiles::Corner { terrain: index.terrain("Grass").unwrap(), height: 0 };
    let lattice = mg_tiles::Lattice::new(8, 8, grass);
    let tiles = mg_tiles::fill(&index, &lattice, &mut fastrand::Rng::with_seed(1)).unwrap();
    let mut g = mg_tiles::paint::Grid::new(&index, 8, 8, tiles).unwrap();
    let palette = TilesetPalette::read(&game, area, &set, &index);
    let Some(Brush::Group(barn)) = palette.brush("Barn 1 2x2") else { panic!("no barn") };
    let grouped = |g: &mg_tiles::paint::Grid| -> Vec<(u32, u32)> {
        let cells = (0..8).flat_map(|y| (0..8).map(move |x| (x, y)));
        cells.filter(|&(x, y)| index.is_grouped(g.tile(x, y).tile)).collect()
    };
    let mut rng = fastrand::Rng::with_seed(2);
    let stroke = g.place_group(&index, &set.groups[barn], 2, 2, 0).unwrap();
    g.apply(&index, stroke, &mut rng);
    assert_eq!(grouped(&g), [(2, 2), (3, 2), (2, 3), (3, 3)]);
    assert_eq!(g.group_cells(&index, 3, 3).map(|c| c.len()), Some(4));
    // Another over its east half: the first goes, all of it.
    let stroke = g.place_group(&index, &set.groups[barn], 3, 2, 0).unwrap();
    g.apply(&index, stroke, &mut rng);
    assert_eq!(grouped(&g), [(3, 2), (4, 2), (3, 3), (4, 3)]);
    // The Eraser on one of its tiles: none of it is left.
    let stroke = g.erase(&index, 4, 3).unwrap();
    g.apply(&index, stroke, &mut rng);
    assert_eq!(grouped(&g), []);
}
