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
    mg_testkit::gpu::hold();
    let gpu = Gpu::headless();
    if gpu.is_none() {
        eprintln!("no GPU: models and rendering not checked");
    }
    let mut renderer = gpu.as_ref().map(|g| Renderer::new(g, wgpu::TextureFormat::Rgba8Unorm, 1));
    let mut failures = Vec::new();
    let (mut areas, mut tiles, mut objects, mut rendered, mut invisible) = (0, 0, 0, 0, 0);
    let start = Instant::now();
    let mut standing: std::collections::BTreeMap<ObjectKind, (usize, usize)> = Default::default();
    let mut hooked = (0usize, 0usize);
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
            // Doors stand on the tiles' door hooks, turned as the hook is
            // (or the other way: a doorway has a hook on each side, and
            // Reverse Door turns a door round).
            for o in model.objects.iter().filter(|o| o.kind == ObjectKind::Door) {
                hooked.1 += 1;
                let turn = |a: f32, b: f32| {
                    let d = (a - b).rem_euclid(std::f32::consts::PI);
                    d.min(std::f32::consts::PI - d)
                };
                if model.hooks_at(o.position).any(|h| turn(h.bearing, o.rotation) < 0.01) {
                    hooked.0 += 1;
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
    let share = hooked.0 as f64 / hooked.1 as f64;
    eprintln!("doors on hooks: {} of {} ({:.1}%)", hooked.0, hooked.1, share * 100.0);
    assert!(share > 0.9, "only {:.1}% of the doors stand on hooks", share * 100.0);
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

/// A waypoint is drawn as the flag of its appearance (waypoint.2da: blue,
/// red, green, yellow); one without an appearance has none, and no problem.
#[test]
fn waypoints_show_their_flags() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let flag = |appearance: u8| {
        let mut w = mg_module::instances::walk_waypoint("WP", [0.0; 3]);
        w.set("Appearance", mg_gff::Value::Byte(appearance));
        let o = mg_area::AreaObject::read(&game, ObjectKind::Waypoint, 0, &w);
        assert_eq!(o.problem, None);
        o.preview.map(|p| p.base.model)
    };
    let flags: Vec<Option<String>> = (1..=4).map(flag).collect();
    let names = ["gi_waypoint01", "gi_waypoint02", "gi_waypoint03", "gi_waypoint04"];
    assert_eq!(flags, names.map(|n| Some(n.to_string())));
    assert_eq!(flag(0), None);
    assert_eq!(flag(200), None);
}

/// A merchant is drawn as the game's marker for one (`gi_store`), a model
/// in the scene, so that what stands in front of it hides it.
#[test]
fn merchants_show_their_marker() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let store = Struct::new(0);
    let o = mg_area::AreaObject::read(&game, ObjectKind::Store, 0, &store);
    assert_eq!(o.problem, None);
    assert_eq!(o.preview.map(|p| p.base.model).as_deref(), Some("gi_store"));
    assert!(ObjectKind::Store.is_marker() && !ObjectKind::Placeable.is_marker());
}

/// A sound is drawn as the game's marker for its kind: heard everywhere in
/// the area, from a random position, or from where it stands.
#[test]
fn sounds_show_their_markers() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let marker = |positional: u8, random: u8| {
        let mut s = Struct::new(0);
        s.set("Positional", mg_gff::Value::Byte(positional));
        s.set("RandomPosition", mg_gff::Value::Byte(random));
        let o = mg_area::AreaObject::read(&game, ObjectKind::Sound, 0, &s);
        assert_eq!(o.problem, None);
        o.preview.map(|p| p.base.model)
    };
    assert_eq!(marker(0, 0).as_deref(), Some("gi_sound_area"));
    assert_eq!(marker(1, 1).as_deref(), Some("gi_sound_rndm"));
    assert_eq!(marker(1, 0).as_deref(), Some("gi_sound_pos"));
    // Sounds have no arrow to fall back to; merchants and waypoints do.
    assert!(ObjectKind::Sound.is_marker() && !ObjectKind::Sound.has_arrow());
    assert!(ObjectKind::Store.has_arrow() && ObjectKind::Waypoint.has_arrow());
}

/// The area view's Animations switch: off, a placed creature holds the
/// pose its idle animation starts with, whatever the time.
#[test]
fn placed_objects_hold_still_when_not_animated() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let root = corpus!();
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU adapter");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(3);
    let mut m = new_module(&game, "Still", &mut rng).unwrap();
    let tileset = ResRef::from_str("ttr01").unwrap();
    let spec = AreaSpec { name: "Field".into(), tileset, width: 2, height: 2 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let key = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    let bandit = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
    let items = |r: ResRef| {
        Gff::read(&game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?).ok().map(|g| g.root)
    };
    let placing = Placing { game: &game, item: &items };
    let at = Placement { position: [10.0, 10.0, 0.0], rotation: 0.0 };
    let placed = instance(&placing, ResType::UTC, &bandit, at, &[]).unwrap();
    let mut git = m.gff(&ResKey::new(area, ResType::GIT)).unwrap().unwrap();
    git.root.set(ObjectKind::Creature.list(), mg_gff::Value::List(vec![placed]));
    let are = m.gff(&ResKey::new(area, ResType::ARE)).unwrap().unwrap();
    let set = mg_area::tileset(&game, tileset).unwrap();
    let model = AreaModel::read(&game, &are.root, &git.root, Some(&set));
    assert_eq!(model.objects.len(), 1);
    let loaded = AreaScene::new(&gpu, &game, &model);
    // The poses of what is drawn where the creature stands.
    let poses = |time: f32, animate: bool| -> Vec<Vec<glam::Mat4>> {
        let view = View { time, animate, fog: false, ..View::of(&model) };
        let here = glam::Vec3::new(10.0, 10.0, 0.0);
        let scene = loaded.scene(&model, &view);
        let stands = scene
            .instances
            .iter()
            .filter(|i| i.transform.transform_point3(glam::Vec3::ZERO).distance(here) < 0.01);
        stands.filter_map(|i| i.pose.as_deref().cloned()).collect()
    };
    assert!(!poses(0.0, true).is_empty(), "the creature is posed");
    assert_ne!(poses(0.4, true), poses(1.3, true), "animated, its pose changes with the time");
    assert_eq!(poses(0.4, false), poses(1.3, false), "still, it doesn't");
    assert_eq!(poses(1.3, false), poses(0.0, true), "it holds its animation's start");
}
