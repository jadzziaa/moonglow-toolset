//! An area's scene following its objects as a game client moves, adds and
//! takes them out (`AreaScene::update_objects`, `AreaScene::remove_object`,
//! `AreaObject::move_to`), held to the scene built afresh of the same area:
//! the same instances, no model read again, the tiles not built again.
//!
//! Its cost on a large area is timed by hand:
//! `cargo test --release -p mg-area --test scene_updates -- --ignored --nocapture`.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use glam::Vec3;
use mg_area::values::{Area, Look, Object, Tile};
use mg_area::{AreaModel, AreaObject, AreaScene, ObjectKind, View};
use mg_core::ResType;
use mg_preview::Preview;
use mg_render::{Gpu, Scene};

fn gpu() -> Option<Gpu> {
    mg_testkit::gpu::hold();
    static BASE: std::sync::OnceLock<Option<Gpu>> = std::sync::OnceLock::new();
    let base = BASE.get_or_init(|| {
        let g = Gpu::headless();
        if g.is_none() {
            eprintln!("skipped: no GPU");
        }
        g
    });
    base.as_ref().map(|g| Gpu::new(g.device.clone(), g.queue.clone()))
}
use mg_resman::{Container, MemContainer, ResError, ResKey};
use mg_rules::GameData;

/// A container that counts the resources read from it.
#[derive(Debug)]
struct Counting {
    inner: MemContainer,
    reads: Arc<AtomicUsize>,
}

impl Container for Counting {
    fn contains(&self, key: &ResKey) -> bool {
        self.inner.contains(key)
    }
    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.read(key)
    }
    fn read_start(&self, key: &ResKey, n: usize) -> Result<Cow<'_, [u8]>, ResError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.read_start(key, n)
    }
    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        self.inner.keys()
    }
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// A model of one square mesh `size` metres across, named `name`.
fn square(name: &str, size: f32) -> String {
    let h = size / 2.0;
    format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
beginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n\
node trimesh {name}_mesh\n  parent {name}\n  bitmap white\n  verts 4\n    -{h} -{h} 0\n    \
{h} -{h} 0\n    {h} {h} 0\n    -{h} {h} 1\n  faces 2\n    0 1 2 1 0 1 2 1\n    0 2 3 1 0 2 3 1\n\
endnode\nendmodelgeom {name}\ndonemodel {name}\n"
    )
}

/// Game data of a tile model and three object models, and the count of
/// what is read from it.
fn game() -> (GameData, Arc<AtomicUsize>) {
    let mut c = MemContainer::new();
    for (name, size) in
        [("tst_tile", 10.0), ("tst_box", 1.0), ("tst_pole", 0.5), ("tst_crate", 2.0)]
    {
        c.insert(ResKey::parse(name, ResType::MDL).unwrap(), square(name, size).into_bytes());
    }
    let reads = Arc::new(AtomicUsize::new(0));
    let mut rm = mg_resman::ResMan::new();
    let counting = Counting { inner: c, reads: reads.clone() };
    rm.add(mg_resman::priority::KEY, "game", mg_resman::LayerClass::Key, counting);
    (GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH)), reads)
}

fn model(name: &str) -> Look {
    Look::Model(Preview::model(name))
}

/// An area `size` tiles square of the tile model, with `objects`.
fn area(game: &GameData, size: u32, objects: Vec<Object>) -> AreaModel {
    let tiles = (0..size * size).map(|i| Tile { orientation: (i % 4) as u8, ..Tile::new(0) });
    let values =
        Area { width: size, height: size, tiles: tiles.collect(), objects, ..Area::default() };
    let mut area = AreaModel::from_values(game, values, None);
    // (No tileset: the tiles are given their model as a tileset would.)
    for t in &mut area.tiles {
        t.model = Some("tst_tile".into());
    }
    area
}

fn view(area: &AreaModel) -> View {
    View { fog: false, shadows: false, ..View::of(area) }
}

/// What a scene draws, in order: each instance's model and where.
fn drawn(scene: &Scene) -> Vec<String> {
    let at = |m: glam::Mat4| m.to_cols_array().map(|v| (v * 1000.0).round() / 1000.0);
    (scene.instances.iter())
        .map(|i| format!("{} {:?} {}", i.model.model.name, at(i.transform), i.tile))
        .collect()
}

/// The tiles' GPU models a scene draws (to tell they were not built again).
fn tile_models(scene: &Scene) -> Vec<usize> {
    let tiles = scene.instances.iter().filter(|i| i.tile);
    tiles.map(|i| Arc::as_ptr(&i.model) as usize).collect()
}

/// The same instances as a scene built afresh of `area`.
fn as_fresh(gpu: &Gpu, game: &GameData, loaded: &AreaScene, area: &AreaModel, what: &str) {
    let fresh = AreaScene::new(gpu, game, area);
    let (now, then) = (loaded.scene(area, &view(area)), fresh.scene(area, &view(area)));
    assert_eq!(drawn(&now), drawn(&then), "{what}: not as the scene built afresh");
    assert_eq!(loaded.missing, fresh.missing, "{what}");
}

#[test]
fn objects_move_come_and_go_as_in_a_scene_built_afresh() {
    let (game, reads) = game();
    let Some(gpu) = gpu() else { return };
    let p = |x: f32, y: f32| Vec3::new(x, y, 0.0);
    let mut area = area(
        &game,
        4,
        vec![
            Object::new(ObjectKind::Creature, p(5.0, 5.0), 0.0, model("tst_pole")),
            Object::new(ObjectKind::Placeable, p(12.0, 7.0), 0.5, model("tst_box")),
            Object::new(ObjectKind::Placeable, p(22.0, 31.0), 1.0, model("tst_box")),
            Object::new(ObjectKind::Placeable, p(35.0, 35.0), 0.0, model("tst_missing")),
        ],
    );
    let mut loaded = AreaScene::new(&gpu, &game, &area);
    assert_eq!(loaded.missing, ["tst_missing"]);
    let tiles = tile_models(&loaded.scene(&area, &view(&area)));
    assert_eq!(tiles.len(), 16);
    let read = |what: &str, since: usize, most: usize| {
        let n = reads.load(Ordering::Relaxed) - since;
        assert!(n <= most, "{what}: {n} resources read");
    };

    // A creature walks: moved in the model, drawn there with no update.
    let before = reads.load(Ordering::Relaxed);
    for step in 0..10 {
        let at = p(5.0 + step as f32 * 0.3, 5.0 + step as f32 * 0.1);
        area.objects[0].move_to(at, step as f32 * 0.2);
        let scene = loaded.scene(&area, &view(&area));
        let creature = scene.instances.iter().find(|i| i.model.model.name == "tst_pole").unwrap();
        assert_eq!(creature.transform, area.objects[0].model_transform());
    }
    read("a creature walking", before, 0);
    as_fresh(&gpu, &game, &loaded, &area, "moved");

    // Its look shown anew (what an update of its look asks for): no model
    // read again, no tile built again.
    let before = reads.load(Ordering::Relaxed);
    loaded.update_objects(&gpu, &game, &area, &[0]);
    read("a look shown anew", before, 0);
    assert_eq!(tile_models(&loaded.scene(&area, &view(&area))), tiles);
    // A look changed to one of a model loaded before, and to a new one.
    area.objects[0].preview = Some(Preview::model("tst_box"));
    loaded.update_objects(&gpu, &game, &area, &[0]);
    read("a look of a model loaded before", before, 0);
    area.objects[0].preview = Some(Preview::model("tst_crate"));
    loaded.update_objects(&gpu, &game, &area, &[0]);
    read("a look of a new model", before, 1);
    as_fresh(&gpu, &game, &loaded, &area, "a new look");

    // Objects come: put at the end of the model's objects, shown by an
    // update of the objects (no tile built again, nothing read again).
    let before = reads.load(Ordering::Relaxed);
    for (n, at) in [p(15.0, 15.0), p(25.0, 5.0)].into_iter().enumerate() {
        let o = Object::new(ObjectKind::Placeable, at, 0.0, model("tst_box"));
        area.objects.push(AreaObject::from_values(10 + n, o));
    }
    loaded.update_objects(&gpu, &game, &area, &[]);
    read("objects added", before, 0);
    assert_eq!(tile_models(&loaded.scene(&area, &view(&area))), tiles);
    as_fresh(&gpu, &game, &loaded, &area, "added");

    // Objects go: taken out of the model and the scene alike.
    let before = reads.load(Ordering::Relaxed);
    area.objects.remove(1);
    loaded.remove_object(&area, 1);
    let missing =
        area.objects.iter().position(|o| o.preview == Some(Preview::model("tst_missing")));
    area.objects.remove(missing.unwrap());
    loaded.remove_object(&area, missing.unwrap());
    read("objects taken out", before, 0);
    assert!(loaded.missing.is_empty(), "the missing model went with its object");
    assert_eq!(tile_models(&loaded.scene(&area, &view(&area))), tiles);
    as_fresh(&gpu, &game, &loaded, &area, "taken out");

    // One taken out of the model alone: the next update of the objects
    // can't tell which, and shows them all anew (nothing read again).
    area.objects.remove(0);
    let before = reads.load(Ordering::Relaxed);
    loaded.update_objects(&gpu, &game, &area, &[]);
    read("objects shown anew", before, 0);
    as_fresh(&gpu, &game, &loaded, &area, "taken out of the model alone");
}

/// Times a frame's scene and the updates of one creature on an area of a
/// thousand tiles and two thousand objects.
#[test]
#[ignore = "timed by hand, in release"]
fn one_creature_moved_costs_little() {
    let (game, reads) = game();
    let Some(gpu) = gpu() else { return };
    let names = ["tst_box", "tst_pole", "tst_crate"];
    let objects = (0..2000)
        .map(|i| {
            let at = Vec3::new((i % 320) as f32 + 0.5, (i / 320) as f32 * 40.0 + 2.0, 0.0);
            // (Two hundred looks: the same models, other texture names.)
            let mut look = Preview::model(names[i % 3]);
            look.base.textures.insert("white".into(), format!("look{}", i % 200));
            let kind = if i == 0 { ObjectKind::Creature } else { ObjectKind::Placeable };
            Object::new(kind, at, 0.0, Look::Model(look))
        })
        .collect();
    let mut area = area(&game, 32, objects);
    let t = Instant::now();
    let mut loaded = AreaScene::new(&gpu, &game, &area);
    println!("built: {:.1} ms", ms(t.elapsed()));
    let frames = 200;
    let median = |mut took: Vec<Duration>| {
        took.sort();
        ms(took[took.len() / 2])
    };
    let before = reads.load(Ordering::Relaxed);
    let mut moved = Vec::new();
    let mut scenes = Vec::new();
    let mut updated = Vec::new();
    for f in 0..frames {
        let t = Instant::now();
        area.objects[0].move_to(Vec3::new(5.0 + f as f32 * 0.01, 5.0, 0.0), 0.0);
        moved.push(t.elapsed());
        let t = Instant::now();
        let scene = loaded.scene(&area, &view(&area));
        scenes.push(t.elapsed());
        assert!(!scene.instances.is_empty());
        let t = Instant::now();
        loaded.update_objects(&gpu, &game, &area, &[0]);
        updated.push(t.elapsed());
    }
    // An object put in and taken out, each frame.
    let (mut added, mut removed) = (Vec::new(), Vec::new());
    for f in 0..frames {
        let o = Object::new(
            ObjectKind::Placeable,
            Vec3::new(f as f32, 9.0, 0.0),
            0.0,
            model("tst_box"),
        );
        let t = Instant::now();
        area.objects.push(AreaObject::from_values(5000, o));
        loaded.update_objects(&gpu, &game, &area, &[]);
        added.push(t.elapsed());
        let t = Instant::now();
        let last = area.objects.len() - 1;
        area.objects.remove(last);
        loaded.remove_object(&area, last);
        removed.push(t.elapsed());
    }
    println!(
        "an object put in (update_objects): {:.3} ms; taken out (remove_object): {:.3} ms",
        median(added),
        median(removed)
    );
    assert_eq!(reads.load(Ordering::Relaxed), before, "a model read again");
    let t = Instant::now();
    loaded.update(&gpu, &game, &area);
    let whole = t.elapsed();
    println!(
        "one creature moved: {:.4} ms; the frame's scene: {:.2} ms; its look shown anew \
         (update_objects): {:.3} ms; every object and tile (update): {:.2} ms",
        median(moved),
        median(scenes),
        median(updated),
        ms(whole)
    );
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
