//! Tile grids of every shipped area: with orientation as counter-clockwise
//! quarter turns, neighbouring tiles agree about the corners and edges they
//! share, and the terrain they describe is fitted by the tileset's tiles.

use std::collections::HashMap;

use mg_core::{Codepage, ResType};
use mg_gff::Gff;
use mg_resman::ResKey;
use mg_schema::{StructExt, are};
use mg_set::Tileset;
use mg_testkit::corpus;

use crate::area_common::each_module;
use mg_tiles::{Lattice, Placement, TileIndex};

#[test]
fn shipped_tile_grids_are_consistent() {
    let root = corpus!();
    let mut indexes: HashMap<String, Option<TileIndex>> = HashMap::new();
    let (mut areas, mut cells, mut mismatched, mut unfitted) = (0, 0, 0, 0);
    let (mut area_seams, mut group_seams) = (Vec::new(), 0);
    let mut worst: Vec<(f64, String)> = Vec::new();
    each_module(&root, |opened| {
        let (path, m, rm, haks) = (&opened.path, &opened.module, &opened.game.resman, &opened.haks);
        for area in m.areas().unwrap() {
            let Ok(data) = rm.get(&ResKey::new(area, ResType::ARE)) else { continue };
            let g = Gff::read(&data).unwrap();
            let (w, h) = (g.root.read(&are::WIDTH) as u32, g.root.read(&are::HEIGHT) as u32);
            let tileset = g.root.read(&are::TILESET).to_lowercase().to_string();
            let key = format!("{}:{tileset}", haks.join(","));
            let index = indexes.entry(key).or_insert_with(|| {
                let data = rm.get(&ResKey::parse(&tileset, ResType::SET)?).ok()?;
                Some(TileIndex::new(&Tileset::parse(&data, Codepage::WINDOWS_1252).ok()?))
            });
            let Some(index) = index else { continue };
            let tiles: Vec<Placement> = g
                .root
                .items(&are::TILE_LIST)
                .iter()
                .map(|t| Placement {
                    tile: t.read(&are::tile_list::TILE_ID) as u32,
                    orientation: t.read(&are::tile_list::TILE_ORIENTATION) as u8,
                    height: t.read(&are::tile_list::TILE_HEIGHT),
                })
                .collect();
            let Some((lattice, bad)) = Lattice::from_tiles(index, w, h, &tiles) else { continue };
            areas += 1;
            cells += tiles.len();
            mismatched += bad.len();
            let seams = mg_tiles::seams(index, w, h, &tiles);
            group_seams += seams.groups.len();
            let file = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let file = file.trim_start_matches("Neverwinter Nights - ").to_string();
            for seam in seams.area {
                area_seams.push(format!("{file}:{area} {:?}-{:?}", seam.cell, seam.neighbor));
            }
            for y in 0..h {
                for x in 0..w {
                    if index.fits(&lattice.cell(x, y)).is_empty() {
                        unfitted += 1;
                    }
                }
            }
            worst.push((
                bad.len() as f64 / tiles.len() as f64,
                format!("{}:{area}", path.display()),
            ));
        }
    });
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    eprintln!(
        "{areas} areas, {cells} tiles: {mismatched} disagree with a neighbour, {unfitted} cells fit no free tile; worst {:?}",
        &worst[..worst.len().min(5)]
    );
    eprintln!("seams: {group_seams} a group's own, the area's {area_seams:?}");
    // 1,462 areas, 102,182 tiles: 69 seams (0.07%), and 103 cells (group
    // tiles) that no free tile fits. Ignoring Tile_Height gives 6% seams.
    assert!(areas > mg_testkit::at_least(1000, 6));
    assert!((mismatched as f64) < cells as f64 * 0.001, "{mismatched} of {cells}");
    assert!((unfitted as f64) < cells as f64 * 0.002, "{unfitted} of {cells}");
    // Verify's tile-seam: of 75 neighbors that disagree (the 69 cells),
    // 66 are a group's own (ttu01's 299 and 300 disagree with each other;
    // trs02's path is on one group's tile alone), and nine the area's,
    // where tiles that fit are in the tileset: Tyrants of the Moonsea's
    // ship puts its plain deck (tile 0) beside a group's gangplank, where
    // tiles 392 to 403 are the same deck with one; Darkness over
    // Daggerford ends a sandbank at a cliff tile. The game loads them as
    // they are.
    const SHIPPED: [&str; 9] = [
        "Darkness over Daggerford.nwm:c1ar2000_northco (2, 5)-(3, 5)",
        "Tyrants of the Moonsea.nwm:moonseablockade (14, 7)-(15, 7)",
        "Tyrants of the Moonsea.nwm:moonseablockade (12, 11)-(13, 11)",
        "Tyrants of the Moonsea.nwm:moonseablockade (15, 11)-(16, 11)",
        "Tyrants of the Moonsea.nwm:moonseablockade (17, 14)-(18, 14)",
        "Tyrants of the Moonsea.nwm:moonseablockade (17, 17)-(17, 18)",
        "Tyrants of the Moonsea.nwm:moonseabattle (15, 11)-(16, 11)",
        "Tyrants of the Moonsea.nwm:moonseabattle (21, 15)-(22, 15)",
        "Tyrants of the Moonsea.nwm:moonseabattle (17, 17)-(17, 18)",
    ];
    let new: Vec<&String> = area_seams.iter().filter(|s| !SHIPPED.contains(&s.as_str())).collect();
    assert!(new.is_empty(), "seams in shipped areas beside the known nine: {new:?}");
}

/// Raise/Lower dragged with Shift over a rectangle of corners (the area
/// view's `paint_corners`: each corner raised in turn, row by row, on
/// what the last left) makes a plateau a step up, its edges fitted, in
/// every shipped tileset with heights but Frozen Wastes (tti01), whose
/// open ground has no tiles for a plateau's edges.
#[test]
fn a_rectangle_raised_corner_by_corner_is_a_plateau() {
    use mg_module::new::{AreaSpec, add_area, new_module};
    use mg_resman::GameInstall;
    use mg_rules::GameData;
    use mg_tiles::paint::Rules;

    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Plateau", &mut rng).unwrap();
    let (mut tried, mut wrong) = (0, Vec::new());
    for tileset in game.resman.list(ResType::SET) {
        let Ok(set) = mg_area::tileset(&game, tileset) else { continue };
        if !set.general.has_height_transition || tileset.to_string() == "tti01" {
            continue;
        }
        let spec = AreaSpec { name: format!("p {tileset}"), tileset, width: 8, height: 8 };
        let Ok(area) = add_area(&mut m, &game, &spec, &mut rng) else { continue };
        let are = m.gff(&ResKey::new(area, ResType::ARE)).unwrap().unwrap().root;
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let Some(mut g) = mg_area::terrain::grid(&are, &index) else { continue };
        tried += 1;
        // Corners 3 to 5 each way, clear of the area's border.
        let rect: Vec<(u32, u32)> = (3..=5).flat_map(|y| (3..=5).map(move |x| (x, y))).collect();
        let mut refused = 0;
        for &(x, y) in &rect {
            match g.raise(&index, &rules, x, y, true) {
                Some(s) => drop(g.apply(&index, s, &mut rng)),
                None => refused += 1,
            }
        }
        let corners = || (0..=8).flat_map(|y| (0..=8).map(move |x| (x, y)));
        let plateau = corners()
            .all(|(x, y)| g.lattice.corner(x, y).height == i32::from(rect.contains(&(x, y))));
        let seams = mg_tiles::seams(&index, 8, 8, &g.tiles);
        let seams = seams.area.len() + seams.groups.len();
        if refused > 0 || !plateau || seams > 0 {
            wrong.push(format!("{tileset}: {refused} refused, plateau {plateau}, {seams} seams"));
        }
    }
    assert!(tried >= 10, "{tried} tilesets with heights");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Change Tileset (`mg_area::retile`): a rural area with a plateau and a
/// road, laid in every shipped tileset. Each new layout is the new
/// tileset's tiles, fitted with no seams, as many as the area has; laid
/// in its own tileset again, nothing of the layout is lost.
#[test]
fn an_area_s_layout_is_laid_in_another_tileset() {
    use mg_area::retile::{Mapping, Set, retile};
    use mg_core::ResRef;
    use mg_edit::Edit;
    use mg_module::new::{AreaSpec, add_area, new_module};
    use mg_resman::GameInstall;
    use mg_rules::GameData;
    use mg_tiles::paint::Rules;

    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(&game, "Retile", &mut rng).unwrap();
    let rural = ResRef::from_str("ttr01").unwrap();
    let spec = AreaSpec { name: "Field".into(), tileset: rural, width: 8, height: 8 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let (are_key, git_key) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let set = mg_area::tileset(&game, rural).unwrap();
    let index = TileIndex::new(&set);
    let rules = Rules::new(&index, &set);
    let mut are = m.gff(&are_key).unwrap().unwrap();
    let mut g = mg_area::terrain::grid(&are.root, &index).unwrap();
    // A plateau, and a road across the area's middle row.
    for (x, y) in (3..=5).flat_map(|y| (3..=5).map(move |x| (x, y))) {
        if let Some(s) = g.raise(&index, &rules, x, y, true) {
            g.apply(&index, s, &mut rng);
        }
    }
    // Trees and water in two corners of it.
    for (name, at) in [("trees", (7, 6)), ("water", (1, 6))] {
        let t = index.terrain(name).unwrap_or_else(|| panic!("ttr01's {name}"));
        if let Some(s) = g.paint(&index, &rules, at.0, at.1, t) {
            g.apply(&index, s, &mut rng);
        }
    }
    let road = index.crosser("road").expect("ttr01's road");
    let edges: Vec<_> = (0..7).map(|x| ((x, 1), mg_tiles::EAST)).collect();
    let (stroke, _) = g.lay_crosser(&index, &edges, &[], road);
    g.apply(&index, stroke.expect("the road laid"), &mut rng);
    let list = are.root.list_mut("Tile_List").unwrap();
    for (i, p) in g.tiles.iter().enumerate() {
        list[i].set("Tile_ID", mg_gff::Value::Int(p.tile as i32));
        list[i].set("Tile_Orientation", mg_gff::Value::Int(i32::from(p.orientation)));
        list[i].set("Tile_Height", mg_gff::Value::Int(p.height));
    }
    let git = m.gff(&git_key).unwrap().map(|g| g.root).unwrap_or_default();
    let old = Set { resref: rural, set: &set, index: &index };
    let none = |_: ResRef| None;
    let (mut tried, mut wrong) = (0, Vec::new());
    for tileset in game.resman.list(ResType::SET) {
        let Ok(new_set) = mg_area::tileset(&game, tileset) else { continue };
        let new_index = TileIndex::new(&new_set);
        let new = Set { resref: tileset, set: &new_set, index: &new_index };
        let Some(mapping) = Mapping::of(&old, &new, &are.root) else { continue };
        let Some(r) =
            retile(&game, are_key, git_key, &are.root, &git, &old, &new, &mapping, &none, &mut rng)
        else {
            continue;
        };
        tried += 1;
        let field = |label: &str| {
            r.edits.iter().find_map(|e| match e {
                Edit::SetField { label: l, value: Some(v), .. } if l == label => Some(v.clone()),
                _ => None,
            })
        };
        let Some(mg_gff::Value::List(tiles)) = field("Tile_List") else { panic!("no tiles") };
        let placed: Vec<mg_tiles::Placement> = tiles
            .iter()
            .map(|t| mg_tiles::Placement {
                tile: t.integer("Tile_ID").unwrap() as u32,
                orientation: t.integer("Tile_Orientation").unwrap() as u8,
                height: t.integer("Tile_Height").unwrap() as i32,
            })
            .collect();
        let foreign = placed.iter().filter(|p| new_index.cell(**p).is_none()).count();
        let seams = mg_tiles::seams(&new_index, 8, 8, &placed);
        let seams = seams.area.len() + seams.groups.len();
        let same_set = field("Tileset") == Some(mg_gff::Value::resref(tileset));
        if placed.len() != 64 || foreign > 0 || seams > 0 || !same_set {
            wrong.push(format!(
                "{tileset}: {} tiles, {foreign} not its own, {seams} seams",
                placed.len()
            ));
        }
        if tileset == rural {
            let terrains: Vec<&str> = mapping.terrains.iter().map(|(o, _)| o.as_str()).collect();
            assert!(terrains.contains(&"trees") && terrains.contains(&"water"), "{terrains:?}");
            assert!(mapping.terrains.iter().all(|(o, n)| o == n), "{mapping:?}");
            assert_eq!(mapping.crossers, [("road".to_string(), Some("road".to_string()))]);
        }
        if tileset == rural && (r.corners_missed > 0 || r.crossers_missed > 0) {
            wrong.push(format!(
                "ttr01 again: {} corners and {} crossers lost",
                r.corners_missed, r.crossers_missed
            ));
        }
        eprintln!(
            "{tileset}: {} corners and {} crosser edges not laid; {:?} {:?}",
            r.corners_missed, r.crossers_missed, mapping.terrains, mapping.crossers
        );
    }
    assert!(tried >= 15, "{tried} tilesets");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
