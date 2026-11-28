//! Tiles set anew at run time, as a server's `SetTile` sets them (Moonglow
//! Client's request), in an area of a shipped tileset: the model's tiles
//! set (`AreaModel::set_tiles`), the scene and the ground following them
//! (`AreaScene::update_tiles`, `Ground::update_tiles`), held at each step to
//! the model, scene and ground built afresh from the values with those
//! tiles: the same instances (tiles, the tile border, flames, grass), the
//! same lights, the same walkmesh heights and picking, the same picture;
//! the other tiles' models and grass kept.
//!
//! What one tile set costs against the whole built again, on a large area,
//! is timed by hand: `cargo test -p mg-corpus-tests --release --test areas
//! tile_updates -- --ignored --nocapture`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::{Vec2, Vec3};
use mg_area::pick::Ray;
use mg_area::values::{Area, Tile};
use mg_area::walk::Ground;
use mg_area::{AreaModel, AreaScene, Sight, View, overview};
use mg_core::{ResRef, ResType};
use mg_render::{Gpu, Renderer, Scene};
use mg_rules::GameData;
use mg_set::Tileset;
use mg_testkit::corpus;

use crate::area_common::shared_game;

fn gpu() -> Option<Gpu> {
    mg_testkit::gpu::hold();
    let g = Gpu::headless();
    if g.is_none() {
        eprintln!("skipped: no GPU adapter");
    }
    g
}

/// The tileset's tiles that have a model, by id, each model once, and
/// those of them whose model has a source light's node (`sl1`).
fn tiles_of(game: &GameData, set: &Tileset) -> (Vec<i64>, Vec<i64>) {
    let mut seen = std::collections::HashSet::new();
    let (mut tiles, mut lit) = (Vec::new(), Vec::new());
    for (id, t) in set.tiles.iter().enumerate() {
        let name = t.model.to_ascii_lowercase();
        if t.is_hole() || !seen.insert(name.clone()) {
            continue;
        }
        let Ok(data) = game.resman.get_named(&name, ResType::MDL) else { continue };
        let Ok(model) = mg_mdl::Model::read(&data) else { continue };
        tiles.push(id as i64);
        if mg_area::source_light_node(&model, &name, 1).is_some() {
            lit.push(id as i64);
        }
    }
    (tiles, lit)
}

/// An area's values `size` tiles square of the tileset `name`: tile `i`
/// the `i`th of `ids` (round again), turned by its index, some raised, with
/// main lights.
fn values(name: &str, size: u32, ids: &[i64]) -> Area {
    let tiles = (0..size * size).map(|i| Tile {
        orientation: (i % 4) as u8,
        height: i32::from(i % 7 == 0),
        main_lights: [(i % 5) as u8, (i % 3) as u8],
        ..Tile::new(ids[i as usize % ids.len()])
    });
    Area {
        tileset: ResRef::from_str(name).ok(),
        width: size,
        height: size,
        tiles: tiles.collect(),
        ..Area::default()
    }
}

/// The views a step is compared in: by day and by night, with the tile
/// border and the grass the area's overview sees.
fn views(area: &AreaModel) -> [View; 2] {
    let sight = Sight::of(&overview(area), 4.0 / 3.0);
    let view = |night: bool| View {
        night,
        fog: false,
        border: true,
        grass: Some(sight),
        ..View::of(area)
    };
    [view(false), view(true)]
}

/// What a scene draws, in order: each instance's model, where, and what it
/// is (a tile's, grass), with its material.
fn drawn(scene: &Scene) -> Vec<String> {
    let at = |m: glam::Mat4| m.to_cols_array().map(|v| (v * 1000.0).round() / 1000.0);
    (scene.instances.iter())
        .map(|i| {
            let (name, tile, grass) = (&i.model.model.name, i.tile, i.grass);
            format!("{name} {:?} {tile} {grass} {:?}", at(i.transform), i.material)
        })
        .collect()
}

/// The ground's faces, its tiles' levels, its heights over a grid (nearest
/// the floor and nearest 10 m up) and where rays from above (straight
/// down, and slanting) meet it.
fn ground_facts(ground: &Ground, area: &AreaModel) -> Vec<String> {
    let mut out: Vec<String> = ground.faces().map(|f| format!("{f:?}")).collect();
    out.extend((0..area.tiles.len()).map(|i| format!("level {i} {:?}", ground.tile_level(i))));
    let (w, h) = area.size();
    let mut y = 1.25;
    while y < h {
        let mut x = 1.25;
        while x < w {
            let p = Vec2::new(x, y);
            let down = Ray { origin: Vec3::new(x, y, 50.0), dir: Vec3::NEG_Z };
            let slant =
                Ray { origin: Vec3::new(x, y - 20.0, 40.0), dir: Vec3::new(0.0, 1.0, -2.0) };
            out.push(format!(
                "{p} {:?} {:?} {:?} {:?}",
                ground.height(p, 0.0),
                ground.height(p, 10.0),
                ground.hit(&down),
                ground.hit(&Ray { dir: slant.dir.normalize(), ..slant }),
            ));
            x += 2.5;
        }
        y += 2.5;
    }
    out
}

/// The models the tiles at the tiles' centres draw and the grass over
/// them, by where they stand (to tell what was kept), but those at
/// `changed`.
fn kept_models(scene: &Scene, area: &AreaModel, changed: &[usize]) -> Vec<(String, usize)> {
    let left_out: Vec<Vec3> = changed.iter().map(|&i| area.tiles[i].position).collect();
    let mut out: Vec<(String, usize)> = (scene.instances.iter())
        .filter(|i| i.tile || i.grass)
        .filter_map(|i| {
            let at = i.transform.w_axis.truncate();
            let tile =
                area.tiles.iter().find(|t| t.position.truncate().distance(at.truncate()) < 5.0)?;
            if left_out.iter().any(|p| p.truncate() == tile.position.truncate()) {
                return None;
            }
            let at = at.to_array().map(|v| (v * 100.0).round() as i64);
            Some((format!("{at:?} {}", i.grass), Arc::as_ptr(&i.model) as usize))
        })
        .collect();
    out.sort();
    out
}

/// An area as a client holds it: its values, its model, its scene and its
/// ground.
struct Live {
    values: Area,
    model: AreaModel,
    scene: AreaScene,
    ground: Ground,
}

/// Sets `tiles` in `live` (values, model, scene and ground) and holds it to
/// the area built afresh of its values; the tiles that changed.
fn step(
    live: &mut Live,
    (gpu, game, set): (&Gpu, &GameData, &Tileset),
    what: &str,
    tiles: Vec<(usize, Tile)>,
) -> Vec<usize> {
    let Live { values, model, scene, ground } = live;
    let before = views(model).map(|v| scene.scene(model, &v));
    for &(i, t) in &tiles {
        values.tiles[i] = t;
    }
    let changed = model.set_tiles(game, Some(set), &tiles);
    assert!(!changed.is_empty(), "{what}: a change");
    scene.update_tiles(gpu, game, model, &changed);
    ground.update_tiles(game, model, &changed);

    // As the values with these tiles build it, along another path.
    let fresh_model = AreaModel::from_values(game, values.clone(), Some(set));
    assert_eq!(*model, fresh_model, "{what}: the model");
    let fresh = AreaScene::new(gpu, game, &fresh_model);
    let fresh_ground = Ground::new(game, &fresh_model);
    assert_eq!(scene.missing, fresh.missing, "{what}: missing");
    for (view, before) in views(model).iter().zip(&before) {
        let (now, then) = (scene.scene(model, view), fresh.scene(&fresh_model, view));
        assert_eq!(drawn(&now), drawn(&then), "{what}: the instances");
        assert!(now.instances.iter().any(|i| i.grass), "{what}: grass grows");
        assert_eq!(now.lights, then.lights, "{what}: the lights");
        assert_eq!(now.area, then.area, "{what}: the area's light");
        assert_eq!(
            kept_models(&now, model, &changed),
            kept_models(before, model, &changed),
            "{what}: the other tiles' models and grass kept"
        );
    }
    assert_eq!(
        ground_facts(ground, model),
        ground_facts(&fresh_ground, &fresh_model),
        "{what}: the ground"
    );
    changed
}

/// Each tile set anew in the model, the scene and the ground is as they are
/// built afresh of the values with those tiles, and the other tiles'
/// models and grass are kept.
#[test]
fn a_tile_set_anew_is_as_the_area_built_afresh() {
    let root = corpus!();
    let Some(gpu) = gpu() else { return };
    let game = shared_game(&root);
    let name = "ttr01";
    let set = mg_area::tileset(&game, ResRef::from_str(name).unwrap()).unwrap();
    let (ids, lit) = tiles_of(&game, &set);
    assert!(ids.len() > 20, "{name}: {} tiles", ids.len());
    // (Six models in the area; the others are new to it.)
    let (used, new) = ids.split_at(6);
    let values = values(name, 6, used);
    let model = AreaModel::from_values(&game, values.clone(), Some(&set));
    let scene = AreaScene::new(&gpu, &game, &model);
    let ground = Ground::new(&game, &model);
    assert!(model.border.len() > 50, "the tile border");
    let mut live = Live { values, model, scene, ground };
    let mut renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 1);
    eprintln!("{name}: {} tile models, of which {} with flames", ids.len(), lit.len());
    let with = (&gpu, &game, &set);
    let t = |live: &Live, i: usize| live.values.tiles[i];

    let edge = 3; // (On the first row: the border follows it.)
    let to = Tile { orientation: 2, ..t(&live, edge) };
    step(&mut live, with, "turned alone, on the edge", vec![(edge, to)]);
    let to = Tile { height: 2, ..t(&live, edge) };
    step(&mut live, with, "raised alone, on the edge", vec![(edge, to)]);
    let to = Tile { id: used[1], ..t(&live, 14) };
    step(&mut live, with, "a model in use", vec![(14, to)]);
    let to = Tile { id: new[0], orientation: 3, height: 1, ..t(&live, 15) };
    let changed =
        step(&mut live, with, "a model new to the area, turned and raised", vec![(15, to)]);
    assert_eq!(changed, [15]);
    let lights = |live: &Live| live.scene.scene(&live.model, &views(&live.model)[1]).lights.len();
    let neighbor = Tile { main_lights: [12, 2], id: new[1], ..t(&live, 21) };
    if let Some(&id) = lit.first() {
        let flame = Tile {
            id,
            main_lights: [7, 9],
            source_lights: [std::num::NonZeroU8::new(3), std::num::NonZeroU8::new(5)],
            ..t(&live, 20)
        };
        let before = lights(&live);
        let what = "two at once: a tile with flames, its neighbor's lights";
        step(&mut live, with, what, vec![(20, flame), (21, neighbor)]);
        let after = lights(&live);
        assert!(after > before, "the flames light: {before} to {after}");
    } else {
        eprintln!("{name} has no tile with flames: the source lights not tried");
        let to = Tile { id: new[1], ..t(&live, 20) };
        step(&mut live, with, "two at once", vec![(20, to), (21, neighbor)]);
    }
    let to = Tile { id: 100_000, ..t(&live, 8) };
    step(&mut live, with, "a tile the tileset has not", vec![(8, to)]);
    let back = vec![(8, Tile { id: used[2], ..t(&live, 8) }), (15, t(&live, 14))];
    step(&mut live, with, "set back", back);
    let Live { values, model, scene, .. } = &live;

    // The picture: the same as the area built afresh draws.
    let fresh_model = AreaModel::from_values(&game, values.clone(), Some(&set));
    let fresh = AreaScene::new(&gpu, &game, &fresh_model);
    let camera = overview(model);
    let view = &views(model)[0];
    let (w, h) = (480, 360);
    let now = renderer.render_image(&gpu, &game.resman, &scene.scene(model, view), &camera, w, h);
    let then =
        renderer.render_image(&gpu, &game.resman, &fresh.scene(&fresh_model, view), &camera, w, h);
    let out = mg_testkit::scratch_dir("tile_updates");
    for (file, img) in [("updated.png", &now), ("afresh.png", &then)] {
        let f = std::fs::File::create(out.join(file)).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&img.data).unwrap();
    }
    let first = &now.data[..4];
    assert!(!now.data.chunks(4).all(|p| p == first), "the picture is not blank");
    let differ = now.data.chunks(4).zip(then.data.chunks(4)).filter(|(a, b)| a != b).count();
    assert_eq!(
        differ, 0,
        "pixels differ from the area built afresh (target/test-output/tile_updates)"
    );
}

/// What one tile set anew costs on an area of a thousand tiles of a shipped
/// tileset: the model, scene and ground built again (what a client did),
/// the kept scene and ground followed whole (`update`), and the tile alone
/// (`update_tiles`).
#[test]
#[ignore = "timed by hand, in release"]
fn one_tile_set_anew_costs_little() {
    let root = corpus!();
    let Some(gpu) = gpu() else { return };
    let game = shared_game(&root);
    let name = "ttr01";
    let set = mg_area::tileset(&game, ResRef::from_str(name).unwrap()).unwrap();
    let (ids, _) = tiles_of(&game, &set);
    let (used, new) = ids.split_at(12);
    let mut values = values(name, 32, used);
    let mut model = AreaModel::from_values(&game, values.clone(), Some(&set));
    let t = Instant::now();
    let mut scene = AreaScene::new(&gpu, &game, &model);
    let mut ground = Ground::new(&game, &model);
    eprintln!("built: {:.1} ms", ms(t.elapsed()));
    let median = |mut took: Vec<Duration>| {
        took.sort();
        ms(took[took.len() / 2])
    };
    let (mut afresh, mut whole, mut alone) = (Vec::new(), Vec::new(), Vec::new());
    for round in 0..9 {
        // (A tile at another place each round, to a model new to the area,
        // and back.)
        let i = 33 * round + 100;
        let to = Tile { id: new[round % new.len()], orientation: 1, height: 1, ..values.tiles[i] };
        let back = values.tiles[i];
        values.tiles[i] = to;

        let t = Instant::now();
        let fresh_model = AreaModel::from_values(&game, values.clone(), Some(&set));
        let fresh = AreaScene::new(&gpu, &game, &fresh_model);
        let fresh_ground = Ground::new(&game, &fresh_model);
        afresh.push(t.elapsed());
        drop((fresh, fresh_ground));

        let t = Instant::now();
        let changed = model.set_tiles(&game, Some(&set), &[(i, to)]);
        scene.update(&gpu, &game, &model);
        ground.update(&game, &model);
        whole.push(t.elapsed());
        model.set_tiles(&game, Some(&set), &[(i, back)]);
        scene.update_tiles(&gpu, &game, &model, &[i]);
        ground.update_tiles(&game, &model, &[i]);

        let t = Instant::now();
        let changed2 = model.set_tiles(&game, Some(&set), &[(i, to)]);
        scene.update_tiles(&gpu, &game, &model, &changed2);
        ground.update_tiles(&game, &model, &changed2);
        alone.push(t.elapsed());
        assert_eq!(changed, changed2);
    }
    eprintln!(
        "one tile set on {name}, 32 by 32: built again {:.1} ms; followed whole (update) {:.2} ms; \
         the tile alone (update_tiles) {:.2} ms",
        median(afresh),
        median(whole),
        median(alone)
    );
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
