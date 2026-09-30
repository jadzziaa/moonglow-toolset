//! Tile grids of every shipped area: with orientation as counter-clockwise
//! quarter turns, neighbouring tiles agree about the corners and edges they
//! share, and the terrain they describe is fitted by the tileset's tiles.

use std::collections::HashMap;

use mg_core::{Codepage, ResType};
use mg_gff::Gff;
use mg_module::Module;
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_schema::{StructExt, are};
use mg_set::Tileset;
use mg_testkit::{bundled_modules, corpus};
use mg_tiles::{Lattice, Placement, TileIndex};

#[test]
fn shipped_tile_grids_are_consistent() {
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let mut indexes: HashMap<String, Option<TileIndex>> = HashMap::new();
    let (mut areas, mut cells, mut mismatched, mut unfitted) = (0, 0, 0, 0);
    let mut worst: Vec<(f64, String)> = Vec::new();
    for path in bundled_modules(&root) {
        let m = Module::open(&path).unwrap();
        let mut rm = ResMan::for_game(&install).unwrap();
        let haks = m.haks().unwrap();
        rm.add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
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
    }
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    eprintln!(
        "{areas} areas, {cells} tiles: {mismatched} disagree with a neighbour, {unfitted} cells fit no free tile; worst {:?}",
        &worst[..worst.len().min(5)]
    );
    // 1,462 areas, 102,182 tiles: 69 seams (0.07%), and 103 cells (group
    // tiles) that no free tile fits. Ignoring Tile_Height gives 6% seams.
    assert!(areas > 1000);
    assert!((mismatched as f64) < cells as f64 * 0.001, "{mismatched} of {cells}");
    assert!((unfitted as f64) < cells as f64 * 0.002, "{unfitted} of {cells}");
}
