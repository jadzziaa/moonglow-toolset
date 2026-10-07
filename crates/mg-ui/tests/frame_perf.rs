//! Frame times against a 144 fps budget: 6.94 ms for all a frame does on
//! the CPU (the interface laid out and tessellated, a 3D view's scene posed
//! and its draws encoded), and as long again for the GPU to draw it. Each
//! view is timed as the desktop app runs it (no accessibility tree, the
//! clock moving on by a 144th of a second a frame), at the development
//! machine's screen (3440×1440): with the pointer moving over it, with the
//! wheel turning, and left alone (where it says how soon it asks to be
//! drawn again). Timing only; the views over the budget are named at the
//! end. (An area's picture is kept between its animations' steps, so its
//! view's usual frame is short and the frame that draws it is the worst:
//! "spikes" says that one is over the budget.) Needs the game (Tyrants of the Moonsea) and a GPU; run in release:
//! `cargo test --release -p mg-ui --test frame_perf -- --ignored --nocapture`.
//! `FRAME_SHOTS=1` saves each view as it was timed instead
//! (`target/test-output/frame-shots`).

#[allow(dead_code)]
mod world;

use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use mg_core::{ResRef, ResType};
use mg_module::palette::BlueprintKind;
use mg_resman::{GameInstall, ResKey};
use mg_ui::{Action, Moonglow, NoDialogs, Tab};

const SCREEN: egui::Vec2 = egui::vec2(3440.0, 1400.0);
const FPS: f64 = 144.0;
const BUDGET: Duration = Duration::from_micros(6_944);
const FRAMES: usize = 72;

const CAMPAIGN: &str = "Neverwinter Nights - Tyrants of the Moonsea.nwm";

type App<'a> = Harness<'a, Moonglow>;

/// What the pointer does while a view is timed.
#[derive(Clone, Copy, PartialEq)]
enum Pointer {
    /// Moves across the view.
    Moves,
    /// Rests on the view with the wheel turning (down, then up).
    Scrolls,
    /// Away from the window: nothing happens.
    Away,
}

struct Report {
    over: Vec<String>,
    shots: bool,
    /// `FRAME_VIEW`: only the views with this in their name are timed.
    only: Option<String>,
    /// `FRAME_COUNT`: frames timed of each (for a profiler to sample).
    count: usize,
    /// `FRAME_DETAIL`: each frame's time in order, and who asked for the
    /// next.
    detail: bool,
}

impl Report {
    fn new() -> Report {
        let var = |name: &str| std::env::var(name).ok();
        Report {
            over: Vec::new(),
            shots: var("FRAME_SHOTS").is_some(),
            only: var("FRAME_VIEW"),
            count: var("FRAME_COUNT").and_then(|n| n.parse().ok()).unwrap_or(FRAMES),
            detail: var("FRAME_DETAIL").is_some(),
        }
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn pick(mut v: Vec<Duration>, at: f64) -> Duration {
    v.sort();
    v[((v.len() - 1) as f64 * at).round() as usize]
}

/// Where `tab` is drawn.
fn rect_of(app: &Moonglow, tab: &Tab) -> Option<egui::Rect> {
    app.dock.iter_leaves().find(|(_, leaf)| leaf.tabs.contains(tab)).map(|(_, leaf)| leaf.rect)
}

/// Frames of the application as the desktop runs it, the pointer doing
/// `pointer` within `rect` and `each` run before every frame: the times of
/// the interface's pass, of tessellating it and of the GPU finishing what
/// the pass sent it, the vertices tessellated, and how soon the last frame
/// asked for another.
struct Frames {
    ui: Vec<Duration>,
    tessellate: Vec<Duration>,
    gpu: Vec<Duration>,
    vertices: usize,
    passes: usize,
    again: Duration,
    /// What asked for the frame after the last.
    causes: Vec<String>,
}

fn frames(
    h: &mut App<'_>,
    rect: egui::Rect,
    pointer: Pointer,
    count: usize,
    mut each: impl FnMut(&mut Moonglow, usize),
) -> Frames {
    let ctx = h.ctx.clone();
    ctx.disable_accesskit();
    let device = h.state().viewport.as_ref().map(|v| v.gpu.device.clone());
    let wait = |device: &Option<wgpu::Device>| {
        if let Some(d) = device {
            let _ = d.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            });
        }
    };
    let mut clock = ctx.input(|i| i.time);
    let mut out = Frames {
        ui: Vec::new(),
        tessellate: Vec::new(),
        gpu: Vec::new(),
        vertices: 0,
        passes: 0,
        again: Duration::MAX,
        causes: Vec::new(),
    };
    let inner = rect.shrink2(rect.size() * 0.2);
    for n in 0..count {
        each(h.state_mut(), n);
        clock += 1.0 / FPS;
        // (The harness's own input, so the window is the one its steps
        // laid out.)
        let mut input = h.input().clone();
        input.time = Some(clock);
        input.predicted_dt = (1.0 / FPS) as f32;
        // Back and forth along the view's diagonal.
        let along = (n as f32 / count as f32 * 2.0 - 1.0).abs();
        match pointer {
            Pointer::Moves => {
                input.events.push(egui::Event::PointerMoved(inner.lerp_inside([along, along])));
            }
            Pointer::Scrolls => {
                input.events.push(egui::Event::PointerMoved(rect.center()));
                let down = if n < count / 2 { -1.0 } else { 1.0 };
                input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, 60.0 * down),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            Pointer::Away => {}
        }
        wait(&device);
        let app = h.state_mut();
        let t0 = Instant::now();
        // As the harness's steps wrap the application (`egui_kittest`'s
        // `AppKind::run_ui`): the same ids, so that what the steps opened
        // (a menu, the tree's groups) is what is timed.
        let full = ctx.run_ui(input, |ui| {
            ui.scope_builder(egui::UiBuilder::new(), |ui| {
                egui::Frame::central_panel(ui.style())
                    .outer_margin(8.0)
                    .inner_margin(0.0)
                    .show(ui, |ui| app.ui(ui));
            });
        });
        let t1 = Instant::now();
        let shapes = ctx.tessellate(full.shapes, full.pixels_per_point);
        let t2 = Instant::now();
        wait(&device);
        let t3 = Instant::now();
        out.ui.push(t1 - t0);
        out.tessellate.push(t2 - t1);
        out.gpu.push(t3 - t1);
        out.vertices = shapes
            .iter()
            .map(|s| match &s.primitive {
                egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                egui::epaint::Primitive::Callback(_) => 0,
            })
            .sum();
        out.passes = out.passes.max(full.platform_output.num_completed_passes);
        out.again = full
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |v| v.repaint_delay);
    }
    out.causes = ctx.repaint_causes().iter().map(ToString::to_string).collect();
    ctx.enable_accesskit();
    out
}

fn again(d: Duration) -> String {
    match d {
        Duration::ZERO => "at once".into(),
        Duration::MAX => "never".into(),
        d => format!("{:.0} ms", ms(d)),
    }
}

/// Times a view three ways and prints a line for each.
fn view(h: &mut App<'_>, r: &mut Report, what: &str, tab: Option<&Tab>) {
    view_with(h, r, what, tab, |_, _| {});
}

fn view_with(
    h: &mut App<'_>,
    r: &mut Report,
    what: &str,
    tab: Option<&Tab>,
    mut each: impl FnMut(&mut Moonglow, usize),
) {
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) {
        return;
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN);
    let rect = tab.and_then(|t| rect_of(h.state(), t)).unwrap_or(screen);
    if r.shots {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-output/frame-shots");
        std::fs::create_dir_all(&dir).unwrap();
        let name: String =
            what.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
        h.render().unwrap().save(dir.join(format!("{name}.png"))).unwrap();
        return;
    }
    // (Left alone before the wheel turns: what the wheel scrolls or zooms
    // goes on moving for a while after it.)
    for (pointer, how) in
        [(Pointer::Moves, "pointer"), (Pointer::Away, "idle"), (Pointer::Scrolls, "wheel")]
    {
        // (A few frames to settle in: what a view first lays out is its
        // opening, not its frame.)
        frames(h, rect, pointer, 4, &mut each);
        let f = frames(h, rect, pointer, r.count, &mut each);
        let cpu: Vec<Duration> = f.ui.iter().zip(&f.tessellate).map(|(a, b)| *a + *b).collect();
        let (median, worst) = (pick(cpu.clone(), 0.5), pick(cpu, 0.99));
        let mark = if median > BUDGET {
            "OVER"
        } else if worst > BUDGET {
            "spikes"
        } else {
            "ok"
        };
        println!(
            "  {what:44} {how:8} cpu {:6.2} ms (worst {:6.2}; ui {:6.2}, tessellate {:5.2}; \
             {:6} vertices, {} pass) gpu {:5.2} ms, again {:8} {mark}",
            ms(median),
            ms(worst),
            ms(pick(f.ui.clone(), 0.5)),
            ms(pick(f.tessellate.clone(), 0.5)),
            f.vertices,
            f.passes,
            ms(pick(f.gpu.clone(), 0.5)),
            again(f.again),
        );
        if median > BUDGET {
            r.over.push(format!("{what} ({how}): {:.2} ms", ms(median)));
        }
        if r.detail {
            let each: Vec<String> = f.ui.iter().map(|d| format!("{:.1}", ms(*d))).collect();
            println!("      frames: {}", each.join(" "));
            println!("      asked for the next by: {}", f.causes.join("; "));
        }
    }
}

fn open(h: &mut App<'_>, tab: Tab) {
    h.state_mut().actions.push(Action::OpenTab(tab));
    h.run_steps(3);
}

/// Clicks the first thing labelled `label`, if there is one.
fn click(h: &mut App<'_>, label: &str) -> bool {
    let found = h.query_all_by_label(label).next().map(|node| node.click()).is_some();
    h.run_steps(2);
    found
}

/// Shows a blueprint palette (not the tileset's, which an open area
/// brings up).
fn palette(h: &mut App<'_>, kind: BlueprintKind, custom: bool, filter: &str) {
    let p = &mut h.state_mut().palette;
    (p.kind, p.custom, p.tiles, p.prefabs) = (kind, custom, false, false);
    p.filter = filter.into();
}

fn close(h: &mut App<'_>, tab: &Tab) {
    let dock = &mut h.state_mut().dock;
    if let Some(at) = dock.find_tab(tab) {
        dock.remove_tab(at);
    }
    h.run_steps(2);
}

/// Opens `tab`, times it and closes it.
fn tab(h: &mut App<'_>, r: &mut Report, what: &str, tab: Tab) {
    open(h, tab.clone());
    view(h, r, what, Some(&tab));
    close(h, &tab);
}

/// The module's largest resource of a type.
fn largest(app: &Moonglow, t: ResType) -> Option<ResKey> {
    let m = &app.ws.as_ref()?.module;
    m.keys_of(t).copied().max_by_key(|k| m.get(k).map_or(0, <[u8]>::len))
}

/// The area with the most tiles, and one of the commonest size.
fn areas(app: &mut Moonglow) -> Vec<(ResRef, i64)> {
    let ws = app.ws.as_mut().unwrap();
    let keys: Vec<_> = ws.module.keys_of(ResType::ARE).copied().collect();
    let mut sized: Vec<(ResRef, i64)> = keys
        .into_iter()
        .filter_map(|k| {
            let g = ws.doc(&k).ok()?;
            Some((k.resref, g.root.integer("Width")? * g.root.integer("Height")?))
        })
        .collect();
    sized.sort_by_key(|(r, n)| (*n, *r));
    let (largest, middle) = (*sized.last().unwrap(), sized[sized.len() / 2]);
    vec![middle, largest]
}

fn harness<'a>(app: Moonglow, rs: egui_wgpu::RenderState) -> App<'a> {
    Harness::builder()
        .with_size(SCREEN)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app)
}

fn render_state() -> egui_wgpu::RenderState {
    egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    )
}

/// An area in the viewer: as it opens (all of it in sight, from above),
/// turned about, and close to the ground.
fn area_views(h: &mut App<'_>, r: &mut Report, area: ResRef, tiles: i64) {
    let tab = Tab::Area(area);
    h.state_mut().actions.push(Action::OpenTab(tab.clone()));
    // Its opening: the frames in which its models are read and its
    // textures first drawn, each a frame the window waits for.
    let mut opening = Vec::new();
    for _ in 0..10 {
        let t = Instant::now();
        h.run_steps(1);
        opening.push(t.elapsed());
        if h.state().area_views.get(&area).is_some_and(|v| v.model.is_some()) {
            break;
        }
    }
    for _ in 0..2 {
        let t = Instant::now();
        h.run_steps(1);
        opening.push(t.elapsed());
    }
    if !r.shots {
        let longest = opening.iter().max().copied().unwrap_or_default();
        println!("  area {area}: opening, its longest frame {:.0} ms", ms(longest));
    }
    let objects = h
        .state()
        .area_views
        .get(&area)
        .and_then(|v| v.model.as_ref())
        .map_or(0, |m| m.objects.len());
    let name = format!("area {area} ({tiles} tiles, {objects} objects)");
    view(h, r, &format!("{name}, overview"), Some(&tab));
    view_with(h, r, &format!("{name}, turning"), Some(&tab), |app, _| {
        if let Some(o) = app.area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
            o.pitch = 0.9;
            o.yaw += 0.01;
        }
    });
    view_with(h, r, &format!("{name}, close"), Some(&tab), |app, _| {
        if let Some(o) = app.area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
            o.pitch = 0.7;
            o.distance = 25.0;
            o.yaw += 0.01;
        }
    });
    close(h, &tab);
}

/// The area viewer's frame taken apart, on a renderer of the test's own:
/// the scene built (every tile and object posed), the particles moved on,
/// the draws encoded and sent, and the GPU drawing them.
fn area_parts(h: &mut App<'_>, area: ResRef, count: usize) {
    use mg_render::{Renderer, Targets};
    let app = h.state_mut();
    let Some(vp) = app.viewport.as_ref() else { return };
    let gpu = vp.gpu.clone();
    let game = app.game.clone().unwrap();
    let ws = app.ws.as_mut().unwrap();
    let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().clone();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap().clone();
    let tileset = mg_area::tileset(&game, are.root.resref("Tileset").unwrap()).ok();
    let model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
    let t = Instant::now();
    let scene = mg_area::AreaScene::new(&gpu, &game, &model);
    println!("  area {area}: models loaded in {:.0?}", t.elapsed());
    let mut view = mg_area::View::of(&model);
    let (w, h_px) = (3000, 1250);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let wait = || {
        let _ = gpu.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        });
    };
    let (width, height) = model.size();
    let span = width.max(height);
    for (what, distance, pitch, samples, instancing) in [
        ("overview, 4x MSAA", span * 1.45, 1.5695, 4, true),
        ("…a draw for each mesh", span * 1.45, 1.5695, 4, false),
        ("overview, no MSAA", span * 1.45, 1.5695, 1, true),
        ("close, 4x MSAA", 25.0, 0.7, 4, true),
    ] {
        let mut renderer = Renderer::new(&gpu, format, samples);
        renderer.instancing = instancing;
        let targets = Targets::new(&gpu, format, samples, w, h_px);
        let mut sims = std::collections::HashMap::new();
        let (mut build, mut particles, mut encode, mut drawn) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut counts = (0, 0, 0, 0);
        let mut yaw = -std::f32::consts::FRAC_PI_2;
        for n in 0..count + 4 {
            let dt = (1.0 / FPS) as f32;
            view.time += dt;
            yaw += 0.01;
            let target = glam::Vec3::new(width / 2.0, height / 2.0, 0.0);
            let mut camera = mg_render::Camera::orbit(target, distance, yaw, pitch);
            camera.far = camera.far.max(distance * 4.0 + 400.0);
            wait();
            let t0 = Instant::now();
            let mut frame = scene.scene(&model, &view);
            let t1 = Instant::now();
            frame.particles = scene.particles(&model, &view, &mut sims, dt, camera.view());
            let t2 = Instant::now();
            renderer.render(
                &gpu,
                &game.resman,
                &frame,
                &camera,
                targets.render_view(),
                targets.resolve_view(),
                &targets.depth,
                (w, h_px),
            );
            let t3 = Instant::now();
            wait();
            let t4 = Instant::now();
            // (The first frames load the textures.)
            if n >= 4 {
                build.push(t1 - t0);
                particles.push(t2 - t1);
                encode.push(t3 - t2);
                drawn.push(t4 - t3);
            }
            counts = (
                frame.instances.len(),
                frame.instances.iter().map(|i| i.model.meshes.len()).sum::<usize>(),
                frame.lights.len(),
                frame.particles.iter().map(|b| b.vertices.len() / 4).sum::<usize>(),
            );
        }
        let (instances, meshes, lights, quads) = counts;
        let did = renderer.drawn;
        println!(
            "    {what:20} scene {:5.2} ms, particles {:5.2} ms, draws encoded {:5.2} ms, GPU \
             {:5.2} ms ({instances} instances, {meshes} meshes, {lights} lights, {quads} \
             particles; {} meshes in {} draws of {} materials in turn, {} meshes out of sight)",
            ms(pick(build, 0.5)),
            ms(pick(particles, 0.5)),
            ms(pick(encode, 0.5)),
            ms(pick(drawn, 0.5)),
            did.meshes,
            did.batches,
            did.materials,
            did.hidden,
        );
    }
}

/// A campaign's views: the module with nothing else open, each editor on
/// its largest resource, the palettes and lists, a menu, and areas.
#[test]
#[ignore]
fn frames_at_144() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let path = root.join("data/nwm").join(CAMPAIGN);
    if !path.is_file() {
        println!("skipped: {CAMPAIGN} isn't installed");
        return;
    }
    let mut r = Report::new();
    println!("{CAMPAIGN}: frames at {SCREEN:?}, budget {:.2} ms", ms(BUDGET));
    let rs = render_state();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    let sized = areas(&mut app);
    let (dialog, script, creature, item, placeable, store) = (
        largest(&app, ResType::DLG),
        largest(&app, ResType::NSS),
        largest(&app, ResType::UTC),
        largest(&app, ResType::UTI),
        largest(&app, ResType::UTP),
        largest(&app, ResType::UTM),
    );
    let mut h = harness(app, rs);
    h.run_steps(3);

    // As a module opens: the tree, the log, the palettes, and its last
    // area in the viewer. Then without the area (whose view is drawn every
    // frame, whatever is worked on in front of it), for each view's own
    // cost.
    view(&mut h, &mut r, "module as it opens (an area, the palettes)", None);
    close(&mut h, &Tab::Palette);
    close(&mut h, &Tab::ModuleProperties);
    view(&mut h, &mut r, "module with its area, nothing else", None);
    let shown: Vec<Tab> = h
        .state()
        .dock
        .iter_all_tabs()
        .map(|(_, t)| t.clone())
        .filter(|t| matches!(t, Tab::Area(_)))
        .collect();
    for t in &shown {
        println!("  (closing {t:?})");
        close(&mut h, t);
    }
    view(&mut h, &mut r, "module open, no area", None);

    // The tree with every list opened.
    click(&mut h, "Expand All");
    view(&mut h, &mut r, "module tree, every group open", None);
    click(&mut h, "Collapse All");

    // A menu, opened.
    if click(&mut h, "File") {
        view(&mut h, &mut r, "File menu open", None);
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
    }

    tab(&mut h, &mut r, "Module Properties", Tab::ModuleProperties);
    for (kind, name) in [
        (BlueprintKind::Creature, "creature"),
        (BlueprintKind::Item, "item"),
        (BlueprintKind::Placeable, "placeable"),
    ] {
        for custom in [false, true] {
            palette(&mut h, kind, custom, "");
            let which = if custom { "custom" } else { "standard" };
            tab(&mut h, &mut r, &format!("{which} {name} palette"), Tab::Palette);
        }
    }
    // (A search opens every category with a find in it.)
    palette(&mut h, BlueprintKind::Placeable, false, "a");
    tab(&mut h, &mut r, "standard placeable palette, searched for \"a\"", Tab::Palette);
    palette(&mut h, BlueprintKind::Item, false, "e");
    tab(&mut h, &mut r, "standard item palette, searched for \"e\"", Tab::Palette);
    // The Gallery: its pictures are made a few a frame, then kept.
    h.state_mut().settings.palette_gallery = true;
    for (side, what) in [(128, "Gallery"), (64, "Gallery, smallest pictures")] {
        h.state_mut().settings.gallery_tile = Some(side);
        palette(&mut h, BlueprintKind::Placeable, false, "a");
        open(&mut h, Tab::Palette);
        h.run_steps(300);
        view(&mut h, &mut r, &format!("standard placeable palette, {what}"), Some(&Tab::Palette));
        close(&mut h, &Tab::Palette);
    }
    h.state_mut().settings.palette_gallery = false;
    h.state_mut().settings.gallery_tile = None;
    h.state_mut().palette.filter.clear();

    tab(&mut h, &mut r, "resource browser", Tab::Resources);
    tab(&mut h, &mut r, "Faction Editor", Tab::Factions);
    tab(&mut h, &mut r, "Journal Editor", Tab::Journal);
    tab(&mut h, &mut r, "Placeable Gallery", Tab::PlaceableGallery);
    tab(&mut h, &mut r, "User Manual", Tab::Manual);
    if let Some(k) = dialog {
        tab(&mut h, &mut r, &format!("conversation {}", k.resref), Tab::Dialog(k));
    }
    if let Some(k) = script {
        tab(&mut h, &mut r, &format!("script {}", k.resref), Tab::Script(k));
    }
    for (k, what) in
        [(creature, "creature"), (item, "item"), (placeable, "placeable"), (store, "store")]
    {
        if let Some(k) = k {
            tab(&mut h, &mut r, &format!("{what} {}", k.resref), Tab::Blueprint(k));
            tab(&mut h, &mut r, &format!("{what} {} as GFF", k.resref), Tab::Gff(k));
        }
    }
    if let Some(k) = creature {
        tab(&mut h, &mut r, &format!("model viewer, creature {}", k.resref), Tab::Model(k));
    }
    if let Some((area, _)) = sized.last() {
        tab(&mut h, &mut r, "Area Properties", Tab::AreaProperties(*area));
    }
    for (area, tiles) in &sized {
        area_views(&mut h, &mut r, *area, *tiles);
    }
    if !r.shots {
        for (area, _) in &sized {
            area_parts(&mut h, *area, r.count);
        }
    }
    println!("over the budget:\n  {}", r.over.join("\n  "));
}

/// The persistent world's views (`world/mod.rs`): the lists at the sizes
/// builders report.
#[test]
#[ignore]
fn frames_at_144_in_a_persistent_world() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let Some(w) = world::world(&root) else { return };
    let mut r = Report::new();
    println!("a persistent world: frames at {SCREEN:?}, budget {:.2} ms", ms(BUDGET));
    let rs = render_state();
    let install = GameInstall::new(&root, Some(w.user.clone()), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&w.module);
    let mut h = harness(app, rs);
    h.run_steps(3);
    close(&mut h, &Tab::Palette);
    close(&mut h, &Tab::ModuleProperties);
    let shown: Vec<Tab> = h
        .state()
        .dock
        .iter_all_tabs()
        .map(|(_, t)| t.clone())
        .filter(|t| matches!(t, Tab::Area(_)))
        .collect();
    for t in &shown {
        close(&mut h, t);
    }
    view(&mut h, &mut r, "module open (300 areas in the tree)", None);
    click(&mut h, "Expand All");
    view(&mut h, &mut r, "module tree, all 25,000 resources listed", None);
    click(&mut h, "Collapse All");

    palette(&mut h, BlueprintKind::Item, true, "");
    tab(&mut h, &mut r, "custom item palette (8,000)", Tab::Palette);
    palette(&mut h, BlueprintKind::Item, true, "pw_i");
    tab(&mut h, &mut r, "custom item palette, all 8,000 shown", Tab::Palette);
    palette(&mut h, BlueprintKind::Placeable, false, "pw placeable");
    tab(&mut h, &mut r, "standard placeable palette (+10,000)", Tab::Palette);
    h.state_mut().palette.filter.clear();

    let store = Tab::Blueprint(ResKey::parse("pw_store", ResType::UTM).unwrap());
    open(&mut h, store.clone());
    h.get_by_label("Inventory").click();
    h.run_steps(3);
    view(&mut h, &mut r, "store inventory with 1,000 items", Some(&store));
    close(&mut h, &store);
    tab(&mut h, &mut r, "resource browser (300,000)", Tab::Resources);
    let areas = h.state().ws.as_ref().unwrap().module.areas().unwrap();
    tab(&mut h, &mut r, "Area Properties (300 areas)", Tab::AreaProperties(areas[0]));
    println!("over the budget:\n  {}", r.over.join("\n  "));
}
