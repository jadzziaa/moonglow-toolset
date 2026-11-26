//! The renderer's fixes of the review of November 2026 in timing views of
//! their own (`docs/deferred.md`, "The review of November 2026's
//! performance fixes without a timing view"): each view draws an area of
//! its own on a renderer of the test's own (as `area_parts` does), with
//! what the fix was about (the trigger) and without it (the control) on
//! alternate frames of one run, each with its own renderer and targets,
//! so that the machine's load and the GPU's clock level are the same for
//! both and their difference is the cost. Its figures are an area's draw
//! alone, not a window's frame (no interface): the scene posed, the
//! particles moved on, the draws encoded (and the time the thread had the
//! processor for them, beside the clock's), and the GPU drawing them (its
//! median and its quickest tenth).
//!
//! - R9: a second view (a two-tile area with a creature, as the model
//!   viewer shows one, with its ground's shadow) drawn through the same
//!   renderer between the frames of a large area with the sun's shadows,
//!   seen from a camera that stays put.
//! - R15: a large area with the sun's shadows, the camera at rest in one
//!   corner, and an animated creature with a skin (the meshes a cloak's
//!   are) at the far corner, out of the map's region.
//!
//!   Both on tiles that play no animation: a swaying tree moves its
//!   caster, and the sun's map is then drawn again every frame whatever
//!   else happens (the first run of these views, on all of ttr01's tiles,
//!   showed nothing of R15's fix for that).
//! - The views' pictures: `FRAME_SHOTS=1` (`target/test-output/render-views`).
//! - R16: an interior with a flame on every tile (256: as many as are
//!   simulated).
//! - R22: The Domains of Dread's tinted placeables, 2,000 in 50
//!   colorways (`MOONGLOW_HAKS` with its haks; skipped without them).
//! - R12 and R13: 60 part-based NPCs, each colored its own way, with heads
//!   whose texture is no PLT where the load order has them (The Domains
//!   of Dread's, with its haks): the textures kept and their memory, and
//!   the first frame, which reads them all (an area's opening).
//!
//! `FRAME_VIEW=<part of a name>` (R9, R12, R15, R16, R22) runs those
//! alone, `FRAME_COUNT=<frames>` times that many frames of each arm, and
//! `FRAME_DETAIL=1` prints each frame's times. Needs the game and a GPU;
//! run in release: `cargo test --release -p mg-ui --test frame_perf --
//! --ignored --nocapture renderer_fixes`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use glam::Vec3;
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_render::{Camera, Gpu, Renderer, Targets};
use mg_resman::GameInstall;
use mg_rules::GameData;

const FPS: f32 = 144.0;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The area views' size (as `area_parts` draws them): the window's
/// central part at 3440 by 1400.
const SIZE: (u32, u32) = (3000, 1250);

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn pick(v: &[Duration], at: f64) -> Duration {
    if v.is_empty() {
        return Duration::ZERO;
    }
    let mut v = v.to_vec();
    v.sort();
    v[((v.len() - 1) as f64 * at).round() as usize]
}

/// The machine's load (one, five and fifteen minutes).
fn load() -> String {
    let s = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    s.split_whitespace().take(3).collect::<Vec<_>>().join(" ")
}

/// How long this thread has been on the processor (as `frame_perf`'s).
fn on_cpu() -> Option<Duration> {
    let stat = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    stat.split_whitespace().next()?.parse().ok().map(Duration::from_nanos)
}

fn wanted(name: &str) -> bool {
    std::env::var("FRAME_VIEW").ok().is_none_or(|v| name.contains(v.as_str()))
}

fn count() -> usize {
    std::env::var("FRAME_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(144)
}

fn detail() -> bool {
    std::env::var_os("FRAME_DETAIL").is_some()
}

/// `FRAME_SHOTS=1`: a view's picture saved as it is timed (in
/// `target/test-output/render-views`), to see what is drawn.
fn shot(gpu: &Gpu, targets: &Targets, name: &str) {
    if std::env::var_os("FRAME_SHOTS").is_none() {
        return;
    }
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-output/render-views");
    std::fs::create_dir_all(&dir).unwrap();
    let name: String =
        name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let img = gpu.read_rgba(&targets.color);
    image::RgbaImage::from_raw(img.width, img.height, img.data)
        .unwrap()
        .save(dir.join(format!("{name}.png")))
        .unwrap();
}

fn wait(gpu: &Gpu) {
    let _ = gpu.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    });
}

/// An area of `side` by `side` tiles of `tileset`, `tile` giving each of
/// its tiles (by its index) what it is, and `objects` placed in it (a GIT
/// list's name and its entries).
fn area(
    game: &GameData,
    tileset: &str,
    side: u32,
    mut tile: impl FnMut(usize, &mut Struct),
    objects: Vec<(&str, Vec<Struct>)>,
) -> mg_area::AreaModel {
    use mg_module::new::{AreaSpec, add_area, new_module};
    let mut rng = fastrand::Rng::with_seed(11);
    let mut m = new_module(game, "Renderer views", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "View".into(),
        tileset: ResRef::from_str(tileset).unwrap(),
        width: side,
        height: side,
    };
    let resref = add_area(&mut m, game, &spec, &mut rng).unwrap();
    let key = mg_resman::ResKey::new(resref, ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    // (Day, the sun casting.)
    are.root.set("IsNight", Value::Byte(0));
    are.root.set("DayNightCycle", Value::Byte(0));
    for (i, t) in are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
        tile(i, t);
    }
    let git_key = mg_resman::ResKey::new(resref, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    for (list, entries) in objects {
        git.root.set(list, Value::List(entries));
    }
    let set = mg_area::tileset(game, ResRef::from_str(tileset).unwrap()).ok();
    let mut model = mg_area::AreaModel::read(game, &are.root, &git.root, set.as_ref());
    let l = &mut model.lighting;
    (l.sun.shadows, l.moon.shadows) = (true, true);
    l.shadow_opacity = l.shadow_opacity.max(50);
    model
}

/// What one area is drawn with: its model and scene, the camera, the
/// targets and its particles' simulations.
struct Pane {
    model: mg_area::AreaModel,
    scene: mg_area::AreaScene,
    view: mg_area::View,
    camera: Camera,
    targets: Targets,
    sims: HashMap<(usize, usize), mg_render::particles::Particles>,
}

impl Pane {
    fn new(
        gpu: &Gpu,
        game: &GameData,
        model: mg_area::AreaModel,
        camera: Camera,
        size: (u32, u32),
    ) -> Pane {
        let scene = mg_area::AreaScene::new(gpu, game, &model);
        let mut view = mg_area::View::of(&model);
        // (The sun's shadows and the creatures', as Options › Area's
        // defaults have them.)
        view.shadows = true;
        view.creature_shadows = true;
        view.light_shadows = false;
        let targets = Targets::new(gpu, FORMAT, 4, size.0, size.1);
        Pane { model, scene, view, camera, targets, sims: HashMap::new() }
    }
}

/// The camera over an area: at `target`, `distance` away, turned `yaw`
/// and looking down `pitch`.
fn camera(target: Vec3, distance: f32, yaw: f32, pitch: f32) -> Camera {
    let mut c = Camera::orbit(target, distance, yaw, pitch);
    c.far = c.far.max(distance * 4.0 + 400.0);
    c
}

/// A renderer as the area view sets one (its switches all given: their
/// defaults may move between the builds compared).
fn renderer(gpu: &Gpu, game_shaders: bool) -> Renderer {
    let mut r = Renderer::new(gpu, FORMAT, 4);
    r.instancing = true;
    r.game_shaders = game_shaders;
    r.game_water = mg_render::GAME_WATER;
    r.soft_particles = true;
    r.shadow_volumes = false;
    r.textures_apart = false;
    r
}

#[derive(Default)]
struct Times {
    scene: Vec<Duration>,
    particles: Vec<Duration>,
    encode: Vec<Duration>,
    encode_cpu: Vec<Duration>,
    gpu: Vec<Duration>,
}

/// One arm of a view: a renderer, the area it times and, where `after`
/// is given, a second view drawn through the same renderer after each of
/// its frames (not timed).
struct Arm {
    name: &'static str,
    renderer: Renderer,
    pane: Pane,
    after: Option<Pane>,
    t: Times,
    /// What its last frame drew.
    counts: String,
}

impl Arm {
    fn frame(&mut self, gpu: &Gpu, game: &GameData, record: bool) {
        let dt = 1.0 / FPS;
        let p = &mut self.pane;
        wait(gpu);
        p.view.time += dt;
        let t0 = Instant::now();
        let mut frame = p.scene.scene(&p.model, &p.view);
        let t1 = Instant::now();
        frame.particles = p.scene.particles(&p.model, &p.view, &mut p.sims, dt, p.camera.view());
        let t2 = Instant::now();
        let c2 = on_cpu();
        self.renderer.render_targets(gpu, &game.resman, &frame, &p.camera, &p.targets);
        let t3 = Instant::now();
        let c3 = on_cpu();
        wait(gpu);
        let t4 = Instant::now();
        if record {
            self.t.scene.push(t1 - t0);
            self.t.particles.push(t2 - t1);
            self.t.encode.push(t3 - t2);
            if let (Some(a), Some(b)) = (c2, c3) {
                self.t.encode_cpu.push(b.saturating_sub(a));
            }
            self.t.gpu.push(t4 - t3);
        }
        let d = self.renderer.drawn;
        self.counts = format!(
            "{} instances, {} lights, {} particles; {} meshes drawn in {} draws, {} of the \
             game's, {} casting the sun's shadows",
            frame.instances.len(),
            frame.lights.len(),
            frame.particles.iter().map(|b| b.vertices.len() / 4).sum::<usize>(),
            d.meshes,
            d.batches,
            d.game,
            d.sun_casters,
        );
        if let Some(b) = self.after.as_mut() {
            b.view.time += dt;
            let mut frame = b.scene.scene(&b.model, &b.view);
            frame.particles =
                b.scene.particles(&b.model, &b.view, &mut b.sims, dt, b.camera.view());
            self.renderer.render_targets(gpu, &game.resman, &frame, &b.camera, &b.targets);
            wait(gpu);
        }
    }

    /// Frames until its textures are read and the game's programs made
    /// (at least eight), none timed.
    fn settle(&mut self, gpu: &Gpu, game: &GameData) {
        let start = Instant::now();
        for n in 0.. {
            self.frame(gpu, game, false);
            let d = self.renderer.drawn;
            let done = d.game_pending == 0 && d.textures_pending == 0;
            if (n >= 8 && done) || start.elapsed() > Duration::from_secs(120) {
                break;
            }
            if !done {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        shot(gpu, &self.pane.targets, self.name);
        if let Some(b) = &self.after {
            shot(gpu, &b.targets, &format!("{}, the second view", self.name));
        }
    }
}

/// The arms' frames in turn (`count` each, the order swapped every other
/// round), then a line for each and their difference (the first's less
/// the second's).
fn alternate(view: &str, gpu: &Gpu, game: &GameData, arms: &mut [Arm]) {
    for a in arms.iter_mut() {
        a.settle(gpu, game);
    }
    let from = load();
    for n in 0..count() {
        let order: Vec<usize> =
            if n % 2 == 0 { (0..arms.len()).collect() } else { (0..arms.len()).rev().collect() };
        for i in order {
            arms[i].frame(gpu, game, true);
        }
    }
    println!("  {view} (load {from} to {})", load());
    for a in arms.iter() {
        let t = &a.t;
        println!(
            "    {:34} scene {:5.2} ms, particles {:5.2} ms, draws encoded {:5.2} ms (on the \
             processor {:5.2}), GPU {:5.2} ms (the quickest tenth {:5.2}) ({})",
            a.name,
            ms(pick(&t.scene, 0.5)),
            ms(pick(&t.particles, 0.5)),
            ms(pick(&t.encode, 0.5)),
            ms(pick(&t.encode_cpu, 0.5)),
            ms(pick(&t.gpu, 0.5)),
            ms(pick(&t.gpu, 0.1)),
            a.counts,
        );
        if detail() {
            let each = |v: &[Duration]| {
                v.iter().map(|d| format!("{:.2}", ms(*d))).collect::<Vec<_>>().join(" ")
            };
            println!("      encoded: {}", each(&t.encode));
            println!("      GPU: {}", each(&t.gpu));
        }
    }
    if let [a, b, ..] = arms {
        let d = |x: &[Duration], y: &[Duration], at: f64| ms(pick(x, at)) - ms(pick(y, at));
        println!(
            "    {:34} scene {:+5.2} ms, particles {:+5.2} ms, draws encoded {:+5.2} ms (on the \
             processor {:+5.2}), GPU {:+5.2} ms (the quickest tenth {:+5.2})",
            format!("{} less {}", a.name, b.name),
            d(&a.t.scene, &b.t.scene, 0.5),
            d(&a.t.particles, &b.t.particles, 0.5),
            d(&a.t.encode, &b.t.encode, 0.5),
            d(&a.t.encode_cpu, &b.t.encode_cpu, 0.5),
            d(&a.t.gpu, &b.t.gpu, 0.5),
            d(&a.t.gpu, &b.t.gpu, 0.1),
        );
    }
}

/// The tiles of a tileset in turn, each turned (as `tdod_area` lays them).
fn every_tile(count: usize) -> impl FnMut(usize, &mut Struct) {
    move |i, t| {
        t.set("Tile_ID", Value::Int((i % count.max(1)) as i32));
        t.set("Tile_Orientation", Value::Int((i / count.max(1) % 4) as i32));
        t.set("Tile_Height", Value::Int(0));
    }
}

/// The tiles of `tileset` whose models play no animation (no tree
/// swaying, no water): a ground whose shadow map stays as it was drawn
/// while nothing on it moves. Their indices.
fn still_tiles(game: &GameData, tileset: &str) -> Vec<usize> {
    let set = tiles_of(game, tileset);
    (0..set.tiles.len())
        .filter(|&i| {
            let name = &set.tiles[i].model;
            let Ok(data) = game.resman.get_named(name, ResType::MDL) else { return false };
            mg_mdl::Model::read(&data).is_ok_and(|m| m.animations.is_empty())
        })
        .collect()
}

/// `tiles` in turn, each turned.
fn these_tiles(tiles: Vec<usize>) -> impl FnMut(usize, &mut Struct) {
    move |i, t| {
        let n = tiles.len().max(1);
        t.set("Tile_ID", Value::Int(tiles.get(i % n).copied().unwrap_or(0) as i32));
        t.set("Tile_Orientation", Value::Int((i / n % 4) as i32));
        t.set("Tile_Height", Value::Int(0));
    }
}

fn tiles_of(game: &GameData, tileset: &str) -> mg_set::Tileset {
    mg_area::tileset(game, ResRef::from_str(tileset).unwrap()).unwrap()
}

/// A creature of the game's (the bandit) made to look as `set` says, with
/// nothing equipped, at `at`.
fn creature(game: &GameData, at: [f32; 3], set: impl FnOnce(&mut Struct)) -> Struct {
    use mg_module::instances::{Placement, Placing, instance};
    let bp = Gff::read(&game.resman.get_named("nw_bandit001", ResType::UTC).unwrap()).unwrap();
    let mut root = bp.root.clone();
    root.set("Equip_ItemList", Value::List(Vec::new()));
    root.set("ItemList", Value::List(Vec::new()));
    set(&mut root);
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    instance(&placing, ResType::UTC, &root, Placement { position: at, rotation: 45.0 }, &[])
        .unwrap()
}

/// The first row of appearance.2da drawn as one model (not of parts) whose
/// model has a skin mesh: its row and model.
fn skinned_appearance(game: &GameData) -> Option<(u16, String)> {
    let table = game.table("appearance").ok()?;
    (0..table.len()).find_map(|row| {
        if table.get(row, "MODELTYPE").is_none_or(|t| t.eq_ignore_ascii_case("P")) {
            return None;
        }
        let name = table.get(row, "RACE")?.to_string();
        let data = game.resman.get_named(&name, ResType::MDL).ok()?;
        let model = mg_mdl::Model::read(&data).ok()?;
        let skinned = model.nodes.iter().any(|n| n.kind.type_name() == "skin");
        skinned.then(|| (u16::try_from(row).ok(), name)).and_then(|(r, n)| Some((r?, n)))
    })
}

/// R9: a large area with the sun's shadows, still, with and without a
/// second view drawn through its renderer between its frames.
fn second_view(gpu: &Gpu, game: &GameData) {
    let still = still_tiles(game, "ttr01");
    let side = 24;
    let big = || area(game, "ttr01", side, these_tiles(still.clone()), Vec::new());
    let span = side as f32 * 10.0;
    let at_rest = camera(Vec3::new(span / 2.0, span / 2.0, 0.0), span * 1.2, -1.2, 0.9);
    // The second view: two tiles by two and a creature on them, close, in a
    // pane of the model viewer's size.
    let small = || {
        let c = creature(game, [10.0, 10.0, 0.0], |_| {});
        area(game, "ttr01", 2, |_, _| {}, vec![("Creature List", vec![c])])
    };
    let near = camera(Vec3::new(10.0, 10.0, 1.0), 5.0, 0.4, 0.3);
    let mut arms = [
        Arm {
            name: "a second view between its frames",
            renderer: renderer(gpu, true),
            pane: Pane::new(gpu, game, big(), at_rest, SIZE),
            after: Some(Pane::new(gpu, game, small(), near, (700, 1000))),
            t: Times::default(),
            counts: String::new(),
        },
        Arm {
            name: "the area alone",
            renderer: renderer(gpu, true),
            pane: Pane::new(gpu, game, big(), at_rest, SIZE),
            after: None,
            t: Times::default(),
            counts: String::new(),
        },
    ];
    println!(
        "R9: {side} by {side} of ttr01's {} tiles that play no animation, the sun's shadows, the \
         camera still; Shaders on",
        still.len()
    );
    alternate("R9, a model view drawn beside the area", gpu, game, &mut arms);
}

/// R15: a large area with the sun's shadows, the camera at rest in a
/// corner, with and without an animated skinned creature far off.
fn far_creature(gpu: &Gpu, game: &GameData) {
    let Some((row, model)) = skinned_appearance(game) else {
        println!("R15 skipped: no appearance of one skinned model");
        return;
    };
    let still = still_tiles(game, "ttr01");
    let side = 32;
    let span = side as f32 * 10.0;
    let make = |with: bool| {
        let objects = if with {
            let far = [span - 15.0, span - 15.0, 0.0];
            let c = creature(game, far, |c| c.set("Appearance_Type", Value::Word(row)));
            vec![("Creature List", vec![c])]
        } else {
            Vec::new()
        };
        area(game, "ttr01", side, these_tiles(still.clone()), objects)
    };
    let corner = camera(Vec3::new(25.0, 25.0, 0.0), 25.0, 0.6, 0.7);
    let mut arms = [
        Arm {
            name: "a creature animating far off",
            renderer: renderer(gpu, true),
            pane: Pane::new(gpu, game, make(true), corner, SIZE),
            after: None,
            t: Times::default(),
            counts: String::new(),
        },
        Arm {
            name: "no creature",
            renderer: renderer(gpu, true),
            pane: Pane::new(gpu, game, make(false), corner, SIZE),
            after: None,
            t: Times::default(),
            counts: String::new(),
        },
    ];
    println!(
        "R15: {side} by {side} of ttr01's {} tiles that play no animation, the sun's shadows, \
         the camera still in a corner; appearance {row} ({model}, a skin) at the far corner; \
         Shaders on",
        still.len()
    );
    alternate("R15, an animated creature out of the map's region", gpu, game, &mut arms);
}

/// R16: an interior with a flame at every tile's first source light,
/// against the same tiles without.
fn flames(gpu: &Gpu, game: &GameData) {
    // (The first interior tileset of these with tiles that have a source
    // light's place.)
    let found = ["tic01", "tin01", "tdc01", "tcn01", "tdm01"].into_iter().find_map(|name| {
        let set = mg_area::tileset(game, ResRef::from_str(name).unwrap()).ok()?;
        // (Those whose model has the node: the set's flag alone is not it.)
        let lit: Vec<usize> = (0..set.tiles.len())
            .filter(|&i| {
                let name = &set.tiles[i].model;
                let Ok(data) = game.resman.get_named(name, ResType::MDL) else { return false };
                mg_mdl::Model::read(&data)
                    .is_ok_and(|m| mg_area::TileFeatures::of(&m, name).source[0])
            })
            .collect();
        (lit.len() >= 4).then_some((name, lit))
    });
    let Some((tileset, lit)) = found else {
        println!("R16 skipped: no interior tileset with source lights");
        return;
    };
    let side = 16;
    let make = |with: bool| {
        let lit = lit.clone();
        let tile = move |i: usize, t: &mut Struct| {
            t.set("Tile_ID", Value::Int(lit[i % lit.len()] as i32));
            t.set("Tile_Orientation", Value::Int((i / lit.len() % 4) as i32));
            t.set("Tile_Height", Value::Int(0));
            let value = if with { (i % 15 + 1) as u8 } else { 0 };
            t.set("Tile_SrcLight1", Value::Byte(value));
            t.set("Tile_SrcLight2", Value::Byte(0));
        };
        area(game, tileset, side, tile, Vec::new())
    };
    let span = side as f32 * 10.0;
    let whole = camera(Vec3::new(span / 2.0, span / 2.0, 0.0), span * 1.2, -1.2, 1.1);
    let mut arms = [
        Arm {
            name: "a flame on every tile",
            renderer: renderer(gpu, true),
            pane: Pane::new(gpu, game, make(true), whole, SIZE),
            after: None,
            t: Times::default(),
            counts: String::new(),
        },
        Arm {
            name: "no flames",
            renderer: renderer(gpu, true),
            pane: Pane::new(gpu, game, make(false), whole, SIZE),
            after: None,
            t: Times::default(),
            counts: String::new(),
        },
    ];
    println!(
        "R16: {side} by {side} of {tileset} ({} of its tiles with a source light's place), seen \
         whole; Shaders on",
        lit.len()
    );
    alternate("R16, an interior with a flame on every tile", gpu, game, &mut arms);
}

/// The game with The Domains of Dread's haks (`MOONGLOW_HAKS`), read where
/// they are: its placeables', its tints' and shaders', and its heads.
fn tdod_game(root: &std::path::Path) -> Option<GameData> {
    let dir = mg_testkit::builder_hak_dirs()
        .into_iter()
        .find(|d| d.join("tdod_top.hak").is_file() && d.join("tdod_shaders.hak").is_file())?;
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with("tdod_p_") && n.ends_with(".hak"))
        .collect();
    // (Lowest first, each added over those before: the module's list,
    // highest first, has tdod_top, the placeables' haks by name, then
    // tdod_heads and tdod_shaders last.)
    names.sort();
    names.reverse();
    let mut names: Vec<String> = ["tdod_shaders.hak", "tdod_heads.hak"]
        .map(String::from)
        .into_iter()
        .chain(names)
        .chain(["tdod_top.hak".to_string()])
        .collect();
    names.retain(|n| dir.join(n).is_file());
    let mut game = GameData::open(&GameInstall::new(root, None, "en")).ok()?;
    for name in &names {
        let Ok(c) = mg_resman::ErfContainer::open(&dir.join(name)) else { continue };
        let layer = format!("hak:{}", name.trim_end_matches(".hak"));
        game.resman.add(mg_resman::priority::HAK_USER, layer, mg_resman::LayerClass::Erf, c);
    }
    game.invalidate();
    println!("The Domains of Dread's haks: {} of them, from {}", names.len(), dir.display());
    Some(game)
}

/// R22: 2,000 of The Domains of Dread's tintable placeables in 50
/// colorways, against the same untinted.
fn tints(gpu: &Gpu, game: &GameData) {
    use mg_module::instances::{Placement, Placing, instance};
    let Ok(table) = game.table("placeables") else { return };
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let mut probe = bp.root.clone();
    probe.set("Static", Value::Byte(0));
    // (Rows of TDoD's own models whose materials take a tint.)
    let rows: Vec<u32> = (0..table.len())
        .filter(|&r| {
            table.get(r, "ModelName").is_some_and(|m| m.to_ascii_lowercase().starts_with("dd_"))
        })
        .filter(|&r| {
            probe.set("Appearance", Value::Dword(r as u32));
            !mg_preview::tintable_materials(game, &probe).is_empty()
        })
        .take(40)
        .map(|r| r as u32)
        .collect();
    if rows.is_empty() {
        println!("R22 skipped: no tintable placeables");
        return;
    }
    let side = 24;
    let span = side as f32 * 10.0;
    let set = tiles_of(game, "ttr01");
    let make = |tinted: bool| {
        let mut rng = fastrand::Rng::with_seed(3);
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let placed: Vec<Struct> = (0..2000)
            .map(|i| {
                let mut root = probe.clone();
                root.set("Appearance", Value::Dword(rows[i % rows.len()]));
                let at = Placement {
                    position: [rng.f32() * span, rng.f32() * span, 0.0],
                    rotation: rng.f32() * 360.0,
                };
                let mut p = instance(&placing, ResType::UTP, &root, at, &[]).unwrap();
                if tinted {
                    let way = i % 50;
                    let mut v = Struct::new(0);
                    v.set("Name", Value::String(b"tdod_tint".to_vec()));
                    v.set("Type", Value::Dword(3));
                    let color = format!("#{:02x}{:02x}{:02x}", way * 5, 255 - way * 5, 128);
                    v.set("Value", Value::String(color.into_bytes()));
                    p.set("VarTable", Value::List(vec![v]));
                }
                p
            })
            .collect();
        area(game, "ttr01", side, every_tile(set.tiles.len()), vec![("Placeable List", placed)])
    };
    let whole = camera(Vec3::new(span / 2.0, span / 2.0, 0.0), span * 1.2, -1.2, 0.9);
    let mut arms = [
        Arm {
            name: "tinted, 50 colorways",
            renderer: renderer(gpu, true),
            pane: Pane::new(gpu, game, make(true), whole, SIZE),
            after: None,
            t: Times::default(),
            counts: String::new(),
        },
        Arm {
            name: "untinted",
            renderer: renderer(gpu, true),
            pane: Pane::new(gpu, game, make(false), whole, SIZE),
            after: None,
            t: Times::default(),
            counts: String::new(),
        },
    ];
    println!(
        "R22: {side} by {side} of ttr01, 2,000 placeables of {} tintable rows of The Domains of \
         Dread's, seen whole; Shaders on",
        rows.len()
    );
    alternate("R22, a TDoD area of tinted placeables", gpu, game, &mut arms);
}

/// The heads of human males and females whose texture is no PLT (a DDS or
/// a TGA): by gender (0 male, 1 female), their numbers.
fn plain_heads(game: &GameData) -> [Vec<u8>; 2] {
    let key = |name: &str, t: ResType| {
        ResRef::from_str(name).is_ok_and(|r| game.resman.contains(&mg_resman::ResKey::new(r, t)))
    };
    ["m", "f"].map(|g| {
        (1..=255u8)
            .filter(|n| {
                let name = format!("p{g}h0_head{n:03}");
                let Ok(data) = game.resman.get_named(&name, ResType::MDL) else { return false };
                let Ok(model) = mg_mdl::Model::read(&data) else { return false };
                let bitmap = model
                    .nodes
                    .iter()
                    .filter_map(|n| n.mesh().filter(|m| m.render))
                    .find_map(|m| m.textures[0].clone());
                bitmap.is_some_and(|b| {
                    let b = b.to_ascii_lowercase();
                    !key(&b, ResType::PLT) && (key(&b, ResType::DDS) || key(&b, ResType::TGA))
                })
            })
            .collect()
    })
}

/// R12 and R13: 60 human NPCs of parts, each colored its own way (or all
/// one way), heads whose texture is no PLT among them where there are
/// any: the textures their first frame reads and keeps, and how long it
/// takes (three times, a renderer anew each).
fn colored_npcs(gpu: &Gpu, game: &GameData, label: &str) {
    let heads = plain_heads(game);
    let make = |own_colors: bool| {
        let mut plain = 0;
        let npcs: Vec<Struct> = (0..60usize)
            .map(|i| {
                let g = i % 2;
                let head = if heads[g].is_empty() {
                    1
                } else {
                    plain += 1;
                    heads[g][i / 2 % heads[g].len()]
                };
                let at = [5.0 + (i % 8) as f32 * 2.5, 5.0 + (i / 8) as f32 * 2.5, 0.0];
                creature(game, at, |c| {
                    c.set("Appearance_Type", Value::Word(6));
                    c.set("Gender", Value::Byte(g as u8));
                    c.set("Phenotype", Value::Int(0));
                    c.set("Appearance_Head", Value::Byte(head));
                    // (Its parts the first of each, bare where there is a
                    // part 0: the belt and the shoulders.)
                    for part in [
                        "Belt", "LBicep", "LFArm", "LFoot", "LHand", "LShin", "LShoul", "LThigh",
                        "Neck", "Pelvis", "RBicep", "RFArm", "RHand", "RShin", "RShoul", "RThigh",
                        "Torso",
                    ] {
                        let bare = matches!(part, "Belt" | "LShoul" | "RShoul");
                        c.set(&format!("BodyPart_{part}"), Value::Byte(u8::from(!bare)));
                    }
                    c.set("ArmorPart_RFoot", Value::Byte(1));
                    let n = if own_colors { i } else { 0 };
                    c.set("Color_Skin", Value::Byte((n % 16) as u8));
                    c.set("Color_Hair", Value::Byte((n * 5 % 64) as u8));
                    c.set("Color_Tattoo1", Value::Byte(n as u8));
                    c.set("Color_Tattoo2", Value::Byte((n * 3 % 64) as u8));
                })
            })
            .collect();
        (area(game, "ttr01", 4, |_, _| {}, vec![("Creature List", npcs)]), plain)
    };
    let seen = camera(Vec3::new(14.0, 14.0, 1.0), 26.0, -1.2, 0.9);
    println!(
        "R12, R13 ({label}): 60 human NPCs of parts, nothing worn; heads with no PLT: {} male, {} \
         female; Shaders off",
        heads[0].len(),
        heads[1].len()
    );
    for (what, own) in [("each colored its own way", true), ("all colored one way", false)] {
        let (model, plain) = make(own);
        let mut pane = Pane::new(gpu, game, model, seen, SIZE);
        let from = load();
        let mut opened = Vec::new();
        let mut memory = None;
        for _ in 0..3 {
            let mut r = renderer(gpu, false);
            wait(gpu);
            let frame = pane.scene.scene(&pane.model, &pane.view);
            let (t0, c0) = (Instant::now(), on_cpu());
            r.render_targets(gpu, &game.resman, &frame, &pane.camera, &pane.targets);
            wait(gpu);
            let (t1, c1) = (Instant::now(), on_cpu());
            let cpu = c0.zip(c1).map(|(a, b)| b.saturating_sub(a));
            opened.push((t1 - t0, cpu.unwrap_or_default()));
            shot(gpu, &pane.targets, &format!("R12 {label} {what}"));
            let m = r.texture_memory();
            memory = Some((m.textures, m.bytes, r.drawn.meshes));
            pane.view.time += 1.0 / FPS;
        }
        let (textures, bytes, meshes) = memory.unwrap_or_default();
        let each: Vec<String> =
            opened.iter().map(|(w, c)| format!("{:.0} ({:.0})", ms(*w), ms(*c))).collect();
        println!(
            "  R12, R13, {what} (load {from} to {}): {plain} with a plain head; {textures} \
             textures kept, {:.1} MiB, {meshes} meshes drawn; the first frame {} ms (on the \
             processor)",
            load(),
            bytes as f64 / (1024.0 * 1024.0),
            each.join(", ")
        );
    }
}

#[test]
#[ignore]
fn renderer_fixes_at_scale() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let Some(gpu) = Gpu::headless() else {
        println!("skipped: no GPU adapter");
        return;
    };
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    println!("the renderer's fixes of the review: {} frames an arm, budget 6.94 ms", count());
    if wanted("R9") {
        second_view(&gpu, &game);
    }
    if wanted("R15") {
        far_creature(&gpu, &game);
    }
    if wanted("R16") {
        flames(&gpu, &game);
    }
    let tdod = (wanted("R22") || wanted("R12")).then(|| tdod_game(&root)).flatten();
    if wanted("R22") {
        match &tdod {
            Some(g) => tints(&gpu, g),
            None => println!("R22 skipped: no tdod_top.hak in MOONGLOW_HAKS"),
        }
    }
    if wanted("R12") {
        colored_npcs(&gpu, &game, "the game's");
        if let Some(g) = &tdod {
            colored_npcs(&gpu, g, "with The Domains of Dread's haks");
        }
    }
}
