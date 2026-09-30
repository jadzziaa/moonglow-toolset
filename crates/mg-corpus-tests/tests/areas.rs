//! Every shipped area as the area editor shows it: its tileset and every
//! tile's model load; every creature, door, item and placeable has a
//! preview; objects stand on the ground its walkmeshes make (sounds 1.5 m
//! above it); and, with a GPU, every model loads and the area renders.
//!
//! Some shipped placeables show nothing in the game either: Wyvern Crown of
//! Cormyr's hak blanks the placeables.2da rows its trees and benches use,
//! and Doom of Icewind Dale places a reserved row (model `USER`). A
//! placeable without a preview must be one of those: its row names no
//! model, or one that does not exist.

use std::collections::HashMap;
use std::time::Instant;

use mg_area::{AreaModel, AreaScene, ObjectKind, View, overview};
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct};
use mg_module::Module;
use mg_render::{Gpu, Renderer};
use mg_resman::{GameInstall, LayerClass, ResKey, priority};
use mg_rules::GameData;
use mg_set::Tileset;
use mg_testkit::{bundled_modules, corpus};

#[test]
fn every_shipped_area_opens_and_renders() {
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let gpu = Gpu::headless();
    if gpu.is_none() {
        eprintln!("no GPU: models and rendering not checked");
    }
    let mut renderer = gpu.as_ref().map(|g| Renderer::new(g, wgpu::TextureFormat::Rgba8Unorm, 1));
    let mut failures = Vec::new();
    let (mut areas, mut tiles, mut objects, mut rendered, mut invisible) = (0, 0, 0, 0, 0);
    let start = Instant::now();
    let mut standing: std::collections::BTreeMap<ObjectKind, (usize, usize)> = Default::default();
    for path in bundled_modules(&root) {
        let module_name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let m = Module::open(&path).unwrap();
        let mut game = GameData::open(&install).unwrap();
        let haks = m.haks().unwrap();
        let missing_haks = game
            .resman
            .add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>())
            .unwrap();
        assert!(missing_haks.is_empty(), "{module_name}: haks missing: {missing_haks:?}");
        game.resman.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        if let Some(r) = &mut renderer {
            r.clear_textures();
        }
        let mut tilesets: HashMap<ResRef, Option<Tileset>> = HashMap::new();
        for area in m.areas().unwrap() {
            let at = format!("{module_name}/{area}");
            let gff = |t: ResType| {
                let data = game.resman.get(&ResKey::new(area, t)).ok()?;
                Gff::read(&data).ok()
            };
            let (Some(are), Some(git)) = (gff(ResType::ARE), gff(ResType::GIT)) else {
                failures.push(format!("{at}: ARE or GIT missing or unreadable"));
                continue;
            };
            let tileset = are.root.resref("Tileset").and_then(|r| {
                tilesets.entry(r).or_insert_with(|| mg_area::tileset(&game, r).ok()).clone()
            });
            let model = AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
            areas += 1;
            tiles += model.tiles.len();
            objects += model.objects.len();
            for p in &model.problems {
                failures.push(format!("{at}: {p}"));
            }
            // Objects stand on the ground: the walkmesh under them is at
            // their height.
            let ground = mg_area::walk::Ground::new(&game, &model);
            for o in &model.objects {
                if o.kind.has_outline() {
                    continue;
                }
                let e = standing.entry(o.kind).or_insert((0, 0));
                e.1 += 1;
                // Sounds stand 1.5 m above it.
                let lift = if o.kind == ObjectKind::Sound { mg_area::SOUND_HEIGHT } else { 0.0 };
                let h = ground.height(o.position.truncate(), o.position.z - lift);
                if h.is_some_and(|z| (z + lift - o.position.z).abs() < 0.02) {
                    e.0 += 1;
                }
            }
            let placeables = git.root.list(ObjectKind::Placeable.list()).unwrap_or(&[]);
            for o in &model.objects {
                let Some(p) = &o.problem else { continue };
                if o.kind == ObjectKind::Placeable && shows_nothing(&game, &placeables[o.index]) {
                    invisible += 1;
                    continue;
                }
                failures.push(format!("{at}: {:?} {} ({:?}): {p}", o.kind, o.index, o.tag));
            }
            let (Some(gpu), Some(r)) = (&gpu, &mut renderer) else { continue };
            let loaded = AreaScene::new(gpu, &game, &model);
            if !loaded.missing.is_empty() {
                failures.push(format!("{at}: models not found: {:?}", loaded.missing));
            }
            let scene = loaded.scene(&model, &View { fog: false, ..View::of(&model) });
            let img = r.render_image(gpu, &game.resman, &scene, &overview(&model), 96, 72);
            let first = &img.data[..4];
            if img.data.chunks(4).all(|p| p == first) {
                failures.push(format!("{at}: renders blank"));
            }
            rendered += 1;
        }
    }
    eprintln!(
        "{areas} areas ({tiles} tiles, {objects} objects, {invisible} placeables the data leaves \
         invisible), {rendered} rendered, in {:.0?}",
        start.elapsed()
    );
    // Creatures, waypoints and stores are placed on the ground; sounds
    // above it; placeables, doors and items are often lifted (onto tables,
    // walls) or on door hooks at the area's edge.
    for (kind, (on, all)) in &standing {
        let share = *on as f64 / *all as f64;
        eprintln!("{kind:?}: {on} of {all} on the ground ({:.1}%)", share * 100.0);
        let least = match kind {
            ObjectKind::Creature | ObjectKind::Waypoint | ObjectKind::Store => 0.95,
            ObjectKind::Sound => 0.7,
            _ => 0.6,
        };
        assert!(share >= least, "{kind:?}: only {:.1}% on the ground", share * 100.0);
    }
    assert!(areas > 1000, "only {areas} areas");
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// Whether the game shows nothing for a placeable: its placeables.2da row
/// names no model, or a model that does not exist.
fn shows_nothing(game: &GameData, placeable: &Struct) -> bool {
    let Ok(table) = game.table("placeables") else { return false };
    let row = placeable.integer("Appearance").unwrap_or(-1);
    let model = usize::try_from(row).ok().and_then(|r| table.get(r, "ModelName"));
    match model.map(str::trim) {
        None | Some("" | "****") => true,
        Some(m) => game.resman.get_named(m, ResType::MDL).is_err(),
    }
}
