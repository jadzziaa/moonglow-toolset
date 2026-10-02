//! Every shipped area's minimap composes (`mg_module::minimap`; its layout
//! is checked against the game client in `client_minimap.rs`), and nearly
//! every tile has its picture.

use mg_core::ResType;
use mg_gff::Gff;
use mg_module::Module;
use mg_resman::{GameInstall, LayerClass, ResKey, priority};
use mg_rules::GameData;
use mg_testkit::{bundled_modules, corpus};

#[test]
fn every_shipped_area_has_a_minimap() {
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let (mut areas, mut tiles, mut missing) = (0usize, 0usize, 0usize);
    let mut failures = Vec::new();
    for path in bundled_modules(&root) {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let m = Module::open(&path).unwrap();
        let mut game = GameData::open(&install).unwrap();
        let haks = m.haks().unwrap();
        game.resman
            .add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>())
            .unwrap();
        game.resman.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        for area in m.areas().unwrap() {
            let Ok(data) = game.resman.get(&ResKey::new(area, ResType::ARE)) else { continue };
            let Ok(are) = Gff::read(&data) else { continue };
            let set = match mg_module::minimap::tileset(&game.resman, &are.root) {
                Ok(s) => s,
                Err(e) => {
                    failures.push(format!("{name}/{area}: {e}"));
                    continue;
                }
            };
            if let Err(e) = mg_module::minimap::minimap(&game.resman, &are.root, &set, Some(4)) {
                failures.push(format!("{name}/{area}: {e}"));
                continue;
            }
            areas += 1;
            let Some(mg_gff::Value::List(list)) = are.root.get("Tile_List") else { continue };
            for t in list {
                tiles += 1;
                let picture = usize::try_from(t.integer("Tile_ID").unwrap_or(-1))
                    .ok()
                    .and_then(|i| set.tiles.get(i))
                    .and_then(|t| t.image_map_2d.clone())
                    .and_then(|n| mg_core::ResRef::from_str(&n.to_ascii_lowercase()).ok())
                    .and_then(|r| game.resman.texture(r));
                missing += usize::from(picture.is_none());
            }
        }
    }
    println!("{areas} areas, {tiles} tiles, {missing} without a picture");
    assert!(areas > 1000, "{areas}");
    assert!(missing * 50 < tiles, "{missing} of {tiles} tiles without a picture");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
