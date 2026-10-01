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
