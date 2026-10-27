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
//! (`target/test-output/frame-shots`). `FRAME_VIEW=<part of a name>` times
//! only those views, `FRAME_COUNT=<frames>` that many frames of each,
//! `FRAME_DETAIL=1` prints each frame's time and who asked for the next,
//! `FRAME_ACCESSKIT=1` builds the accessibility tree every frame, as the app
//! does while an assistive client is connected (a screen reader, a desktop
//! agent), and `FRAME_TARGET=<width>x<height>` sizes the pictures of the areas
//! taken apart at the end. (With another program drawing, the game say,
//! the GPU's times are as much that program's: close it for those.)

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
    // (The desktop app builds the accessibility tree only while an assistive
    // client, a screen reader or a desktop agent, is connected: FRAME_ACCESSKIT
    // times the frames as they are then.)
    if std::env::var_os("FRAME_ACCESSKIT").is_none() {
        ctx.disable_accesskit();
    }
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
/// turned about, and close to the ground. `label` follows its name in what
/// is printed (the Shaders switch, say).
fn area_views(h: &mut App<'_>, r: &mut Report, area: ResRef, tiles: i64, label: &str) {
    let tab = Tab::Area(area);
    // (A view of its own, from the overview: the view of the area opened
    // before keeps its camera where its last views left it, close up.)
    h.state_mut().area_views.remove(&area);
    // Its opening, as the desktop app opens it (the window kept going: the
    // area read on a thread of its own, its textures apart): the frames
    // from the tab's opening until it is drawn whole, a 144th of a second
    // apart.
    let background = h.state().background_jobs;
    h.state_mut().background_jobs = true;
    // (Its textures read anew, as the first time an area is opened: the
    // area a module opens itself was drawn as it opened.)
    if let Some(vp) = h.state_mut().viewport.as_mut() {
        vp.renderer.clear_textures();
    }
    h.state_mut().actions.push(Action::OpenTab(tab.clone()));
    let mut opening = Vec::new();
    let begun = Instant::now();
    let (mut shown, mut whole) = (None, None);
    while begun.elapsed() < Duration::from_secs(120) {
        let t = Instant::now();
        h.run_steps(1);
        let took = t.elapsed();
        opening.push(took);
        let app = h.state();
        let view = app.area_views.get(&area);
        if shown.is_none() && view.is_some_and(|v| v.model.is_some()) {
            shown = Some(begun.elapsed());
        }
        let pending = app.viewport.as_ref().map_or(0, |v| v.renderer.drawn.textures_pending);
        if shown.is_some() && view.is_some_and(|v| v.pictures > 0) && pending == 0 {
            whole = Some(begun.elapsed());
            break;
        }
        std::thread::sleep(Duration::from_secs_f64(1.0 / FPS).saturating_sub(took));
    }
    h.state_mut().background_jobs = background;
    // (The game's programs, made while the frames go on: what is timed
    // after is the view as it stays.)
    let start = Instant::now();
    while h.state().viewport.as_ref().is_some_and(|v| v.renderer.drawn.game_pending > 0)
        && start.elapsed() < Duration::from_secs(60)
    {
        std::thread::sleep(Duration::from_millis(5));
        h.run_steps(1);
    }
    if !r.shots {
        let longest = opening.iter().max().copied().unwrap_or_default();
        let all: Duration = opening.iter().sum();
        let over = opening.iter().filter(|f| **f > BUDGET).count();
        println!(
            "  area {area}{label}: opening, its longest frame {:.1} ms ({} frames, {over} over \
             the budget, {:.0} ms of them in all); shown after {:.0} ms, whole after {:.0} ms",
            ms(longest),
            opening.len(),
            ms(all),
            shown.map_or(f64::NAN, ms),
            whole.map_or(f64::NAN, ms),
        );
        if r.detail {
            let each: Vec<String> = opening.iter().map(|d| format!("{:.1}", ms(*d))).collect();
            println!("      frames: {}", each.join(" "));
        }
    }
    let objects = h
        .state()
        .area_views
        .get(&area)
        .and_then(|v| v.model.as_ref())
        .map_or(0, |m| m.objects.len());
    let name = format!("area {area} ({tiles} tiles, {objects} objects){label}");
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
    // Every object selected (Ctrl+A) and the ground's walkmesh shown,
    // turning: the overlays drawn for each, each frame.
    if let Some(v) = h.state_mut().area_views.get_mut(&area)
        && let Some(m) = v.model.as_ref()
    {
        v.selection = m.objects.iter().map(|o| (o.kind, o.index)).collect();
        v.walkmesh = true;
    }
    view_with(h, r, &format!("{name}, all selected, walkmesh"), Some(&tab), |app, _| {
        if let Some(o) = app.area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
            o.pitch = 0.9;
            o.distance = 120.0;
            o.yaw += 0.01;
        }
    });
    if let Some(v) = h.state_mut().area_views.get_mut(&area) {
        v.selection.clear();
        v.walkmesh = false;
    }
    close(h, &tab);
}

/// `FRAME_HARD_PARTICLES=1`: particles without their fade too
/// (`Renderer::soft_particles` off), with the Shaders switch on.
fn hard_particles() -> bool {
    std::env::var_os("FRAME_HARD_PARTICLES").is_some()
}

/// `FRAME_NO_WATER=1`: [`water_area`] without its water (its fires on
/// grass alone).
fn no_water() -> bool {
    std::env::var_os("FRAME_NO_WATER").is_some()
}

/// `FRAME_OWN_WATER=1`: shiny water by Moonglow's own water too, with the
/// Shaders switch on (by default `fswater` with the framebuffer's copies,
/// [`mg_render::GAME_WATER`]), for comparing.
fn own_water() -> bool {
    std::env::var_os("FRAME_OWN_WATER").is_some()
}

/// The area viewer's frame taken apart, on a renderer of the test's own:
/// the scene built (every tile and object posed), the particles moved on,
/// the draws encoded and sent, and the GPU drawing them; with the Shaders
/// switch as each of `shaders` has it (a renderer of its own each: the
/// first frame, which reads the textures and, with the switch on, makes
/// the game's programs, is printed by itself). With `grass`, the area's
/// grass as the view draws it, and the field's views of
/// [`grass_frames_at_144`] too.
fn area_parts(h: &mut App<'_>, area: ResRef, count: usize, shaders: &[bool], grass: bool) {
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
    let mut scene = mg_area::AreaScene::new(&gpu, &game, &model);
    println!("  area {area}: models loaded in {:.0?}", t.elapsed());
    let mut view = mg_area::View::of(&model);
    // (`FRAME_TARGET=<width>x<height>`: a picture of another size.)
    let (w, h_px) = std::env::var("FRAME_TARGET")
        .ok()
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((3000, 1250));
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let wait = || {
        let _ = gpu.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        });
    };
    let (width, height) = model.size();
    let span = width.max(height);
    let mut configs = vec![
        ("overview, 4x MSAA", span * 1.45, 1.5695, 4, true),
        ("…a draw for each mesh", span * 1.45, 1.5695, 4, false),
        ("overview, no MSAA", span * 1.45, 1.5695, 1, true),
        // (As the turning view sees it zoomed out: the area from the side.)
        ("turned, 4x MSAA", span * 1.45, 0.9, 4, true),
        ("close, 4x MSAA", 25.0, 0.7, 4, true),
    ];
    if grass {
        configs.extend([
            ("across, 4x MSAA", 50.0, 0.3, 4, true),
            ("high, 4x MSAA", 420.0, 0.9, 4, true),
            ("orbiting, 4x MSAA", 150.0, 0.6, 4, true),
        ]);
    }
    // (`FRAME_OWN_WATER`: with the switch on, Moonglow's water as well.)
    let waters: &[bool] = if own_water() { &[true, false] } else { &[true] };
    // (`FRAME_HARD_PARTICLES`: with the switch on, particles without their
    // fade as well, `Renderer::soft_particles`.)
    let softs: &[bool] = if hard_particles() { &[true, false] } else { &[true] };
    for (what, distance, pitch, samples, instancing, on, water, soft) in configs
        .into_iter()
        .flat_map(|c| shaders.iter().map(move |&g| (c.0, c.1, c.2, c.3, c.4, g)))
        .flat_map(|c| waters.iter().map(move |&w| (c.0, c.1, c.2, c.3, c.4, c.5, w)))
        .flat_map(|c| softs.iter().map(move |&f| (c.0, c.1, c.2, c.3, c.4, c.5, c.6, f)))
    {
        // (Moonglow's water, and hard particles, are the switch on's
        // comparisons only, one at a time.)
        if (!on && !(water && soft)) || (!water && !soft) {
            continue;
        }
        // (A draw for each mesh is the switch off's comparison only.)
        if on && !instancing {
            continue;
        }
        // (`FRAME_PARTS=<a name>`: only that one, for a profiler.)
        let named = if on { format!("{what}, Shaders on") } else { what.to_string() };
        if std::env::var("FRAME_PARTS").is_ok_and(|p| p != named) {
            continue;
        }
        let mut renderer = Renderer::new(&gpu, format, samples);
        renderer.instancing = instancing;
        renderer.game_shaders = on;
        renderer.game_water = water;
        renderer.soft_particles = soft;
        let mut first = Duration::ZERO;
        let targets = Targets::new(&gpu, format, samples, w, h_px);
        let mut sims = std::collections::HashMap::new();
        let (mut build, mut particles, mut encode, mut drawn) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut encode_cpu = Vec::new();
        let mut counts = (0, 0, 0, 0);
        let mut yaw = -std::f32::consts::FRAC_PI_2;
        for n in 0..count + 4 {
            let dt = (1.0 / FPS) as f32;
            view.time += dt;
            yaw += 0.01;
            let target = glam::Vec3::new(width / 2.0, height / 2.0, 0.0);
            let mut camera = mg_render::Camera::orbit(target, distance, yaw, pitch);
            camera.far = camera.far.max(distance * 4.0 + 400.0);
            if grass {
                let sight = mg_area::Sight::of(&camera, w as f32 / h_px as f32);
                view.grass = Some(sight);
                // (The grass far off put together first, as it stays once
                // the view has been looked at a moment.)
                let start = Instant::now();
                while n == 0
                    && scene.grow_grass(&gpu, &model, &sight)
                    && start.elapsed() < Duration::from_secs(30)
                {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            wait();
            // (The last frame lists what the game's shaders drew: how many
            // with NO_DISCARD 1.)
            if on && n == count + 3 {
                renderer.game_programs = Some(Vec::new());
            }
            let t0 = Instant::now();
            // (The chunks of grass far off taken up as the area view does.)
            if let Some(sight) = &view.grass {
                scene.grow_grass(&gpu, &model, sight);
            }
            let mut frame = scene.scene(&model, &view);
            let t1 = Instant::now();
            frame.particles = scene.particles(&model, &view, &mut sims, dt, camera.view());
            let t2 = Instant::now();
            let c2 = on_cpu();
            renderer.render_targets(&gpu, &game.resman, &frame, &camera, &targets);
            let t3 = Instant::now();
            let c3 = on_cpu();
            wait();
            let t4 = Instant::now();
            if n == 0 {
                first = t4 - t0;
            }
            // (The first frames load the textures.)
            if n >= 4 {
                build.push(t1 - t0);
                particles.push(t2 - t1);
                encode.push(t3 - t2);
                if let (Some(a), Some(b)) = (c2, c3) {
                    encode_cpu.push(b.saturating_sub(a));
                }
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
        if encode_cpu.is_empty() {
            encode_cpu.push(Duration::ZERO);
        }
        let did = renderer.drawn;
        if let Some(programs) = renderer.game_programs.take() {
            let solid = programs.iter().filter(|p| p.no_discard).count();
            println!("      NO_DISCARD 1 for {solid} of the game's {} meshes", programs.len());
        }
        let what = if soft { what.to_string() } else { format!("{what}, hard particles") };
        let what = match (on, water) {
            (false, _) => what.to_string(),
            (true, true) => format!("{what}, Shaders on"),
            (true, false) => format!("{what}, Shaders on, Moonglow's water"),
        };
        // (`FRAME_PARTS_SHOTS=<folder>`: its last frame saved there, at the
        // clock of the frames before, for comparing pictures.)
        if let Ok(dir) = std::env::var("FRAME_PARTS_SHOTS") {
            let img = gpu.read_rgba(&targets.color);
            let name: String = format!("{area}-{what}")
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect();
            std::fs::create_dir_all(&dir).unwrap();
            let path = std::path::Path::new(&dir).join(format!("{name}.png"));
            image::RgbaImage::from_raw(img.width, img.height, img.data)
                .unwrap()
                .save(path)
                .unwrap();
        }
        println!(
            // (The GPU's least times too: with another program drawing,
            // the usual one is as much that program's.)
            "    {what:32} first frame {:6.0} ms, scene {:5.2} ms, particles {:5.2} ms, draws \
             encoded {:5.2} ms (worst {:5.2}; on the processor {:5.2}), GPU {:5.2} ms (the quickest tenth {:5.2}) \
             ({instances} instances, {meshes} meshes, {lights} lights, {quads} particles; {} \
             meshes in {} draws of {} materials and {} pipelines in turn, {} of the game's, {} \
             meshes out of sight)",
            ms(first),
            ms(pick(build, 0.5)),
            ms(pick(particles, 0.5)),
            ms(pick(encode.clone(), 0.5)),
            ms(pick(encode, 0.99)),
            ms(pick(encode_cpu, 0.5)),
            ms(pick(drawn.clone(), 0.5)),
            ms(pick(drawn, 0.1)),
            did.meshes,
            did.batches,
            did.materials,
            did.pipelines,
            did.game,
            did.hidden,
        );
    }
}

/// How long this thread has been on the processor (Linux's
/// `/proc/thread-self/schedstat`): with other programs running, the time a
/// step took less the time it waited for the processor.
fn on_cpu() -> Option<Duration> {
    let stat = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    stat.split_whitespace().next()?.parse().ok().map(Duration::from_nanos)
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
    click(&mut h, "⏷");
    view(&mut h, &mut r, "module tree, every group open", None);
    click(&mut h, "⏶");

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
        area_views(&mut h, &mut r, *area, *tiles, "");
    }
    if !r.shots {
        for (area, _) in &sized {
            area_parts(&mut h, *area, r.count, &[false], false);
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
    click(&mut h, "⏷");
    view(&mut h, &mut r, "module tree, all 25,000 resources listed", None);
    click(&mut h, "⏶");

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
    // An item's Properties page: every property it may take, by kind.
    let item = Tab::Blueprint(ResKey::parse("pw_i00001", ResType::UTI).unwrap());
    open(&mut h, item.clone());
    if click(&mut h, "Properties") {
        view(&mut h, &mut r, "item blueprint, Properties page", Some(&item));
    }
    close(&mut h, &item);
    // The same store in the Preview window (its stock by page).
    h.state_mut().palette.selected = ResKey::parse("pw_store", ResType::UTM);
    h.state_mut().preview_window = true;
    h.run_steps(3);
    view(&mut h, &mut r, "Preview window, a store of 1,000 items", None);
    h.state_mut().preview_window = false;
    tab(&mut h, &mut r, "resource browser (300,000)", Tab::Resources);
    let areas = h.state().ws.as_ref().unwrap().module.areas().unwrap();
    tab(&mut h, &mut r, "Area Properties (300 areas)", Tab::AreaProperties(areas[0]));

    // placeables.2da with the haks' 4,000 rows, in the 2DA editor, a
    // filter typed.
    let placeables = ResKey::parse("placeables", ResType::TWODA).unwrap();
    if let Some(id) =
        mg_ui::table_view::edit_copy(h.state_mut(), placeables, mg_ui::table_view::Home::Module)
    {
        h.run_steps(3);
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.filter = "plc".into();
        let t = Tab::Table(id);
        view(&mut h, &mut r, "2DA editor, placeables.2da (+4,000 rows) filtered", Some(&t));
        close(&mut h, &t);
        h.state_mut().tables.retain(|d| d.id != id);
    }
    // The Export window, every kind listed, a filter typed.
    h.state_mut().actions.push(Action::ExportDialog(Vec::new()));
    h.run_steps(3);
    if let Some(d) = h.state_mut().export.as_mut() {
        d.filter = "pw_i0".into();
    }
    view(&mut h, &mut r, "Export window (25,000 resources), filtered", None);
    h.state_mut().export = None;
    // The log after a Verify that found much: 10,000 lines, 2,000 with
    // a fix beside them.
    {
        let log = &mut h.state_mut().log;
        log.clear();
        for n in 0..10_000 {
            log.warn(format!("pw_i{n:05}.uti: a finding of Verify Module's, line {n}"));
            if n % 5 == 0 {
                log.fixes.push(mg_ui::OfferedFix {
                    line: n,
                    text: log.entries[n].1.clone(),
                    fix: mg_edit::fix::Fix { label: "Fix".into(), edits: Vec::new() },
                    made: false,
                });
            }
        }
    }
    h.run_steps(2);
    view(&mut h, &mut r, "log of 10,000 lines, 2,000 fixes", None);
    h.state_mut().log.clear();
    println!("over the budget:\n  {}", r.over.join("\n  "));
}

/// The persistent world (`world/mod.rs`) kept as a nasher project, built
/// once into `pw-world/project` and copied for each run: 21,000 source
/// files. The copy's root.
fn world_project(w: &world::World) -> std::path::PathBuf {
    use mg_module::{Module, ModuleLocation};
    let base = w.user.parent().unwrap().join("project");
    let stamp = base.join(".built");
    if !stamp.is_file() {
        let _ = std::fs::remove_dir_all(&base);
        let t = Instant::now();
        let mut m = Module::open(&w.module).unwrap();
        m.save_as(&ModuleLocation::Project { root: base.clone(), target: "default".into() })
            .unwrap();
        std::fs::write(&stamp, "").unwrap();
        println!("kept the world as a nasher project in {:.1?}", t.elapsed());
    }
    let dir = mg_testkit::scratch_dir("frame-reload").join("project");
    let mut todo = vec![(base, dir.clone())];
    while let Some((from, to)) = todo.pop() {
        std::fs::create_dir_all(&to).unwrap();
        for e in std::fs::read_dir(&from).unwrap().flatten() {
            let (f, t) = (e.path(), to.join(e.file_name()));
            if e.file_type().unwrap().is_dir() {
                todo.push((f, t));
            } else {
                std::fs::copy(&f, &t).unwrap();
            }
        }
    }
    dir
}

/// This process's memory now and at its most since [`reset_peak`], in MiB.
fn memory() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        let line = status.lines().find(|l| l.starts_with(name)).unwrap_or("");
        line.split_whitespace().nth(1).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0) / 1024
    };
    (field("VmRSS:"), field("VmHWM:"))
}

fn reset_peak() {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
}

/// A `git pull` on a large nasher project while it is open: the persistent
/// world's 21,000 source files, of which 150 areas (their `.are.json` and
/// `.git.json`), 400 blueprints and 200 scripts are rewritten, 40 scripts
/// added and 30 deleted, in about a second, by another thread, while the
/// window draws; one blueprint has an unsaved change here too, which is
/// to be asked about. Every frame is timed from the pull until it is all
/// in (and 2 s after): the longest, how many over the budget, how long it
/// took, and the memory. `FRAME_RELOAD_AREAS` and `FRAME_RELOAD_BLUEPRINTS`
/// change how many are rewritten. Run alone:
/// `cargo test --release -p mg-ui --test frame_perf reload -- --ignored --nocapture`.
#[test]
#[ignore]
fn reload_frames_at_144() {
    use mg_gff::Value;
    use std::collections::HashMap;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let Some(w) = world::world(&root) else { return };
    let mut r = Report::new();
    let var =
        |name: &str, or: usize| std::env::var(name).ok().and_then(|n| n.parse().ok()).unwrap_or(or);
    let (areas, blueprints, scripts, added, removed) =
        (var("FRAME_RELOAD_AREAS", 150), var("FRAME_RELOAD_BLUEPRINTS", 400), 200, 40, 30);
    let dir = world_project(&w);
    let rs = render_state();
    let install = GameInstall::new(&root, Some(w.user.clone()), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.settings.no_last_area = true;
    let t = Instant::now();
    app.open_module(&dir);
    println!("a large nasher project: opened in {:.1?}", t.elapsed());
    let mut h = harness(app, rs);
    h.run_steps(3);
    close(&mut h, &Tab::Palette);
    close(&mut h, &Tab::ModuleProperties);
    view(&mut h, &mut r, "large nasher project open (21,000 files)", None);

    // What the pull writes: each file's new text, and what the module is to
    // hold after (`None`: gone).
    let mut plan: Vec<(std::path::PathBuf, Option<Vec<u8>>)> = Vec::new();
    let mut expected: HashMap<ResKey, Option<Vec<u8>>> = HashMap::new();
    let (mine, mine_path) = {
        let ws = h.state().ws.as_ref().unwrap();
        let project = ws.module.project.as_ref().unwrap();
        let conversion = project.conversion();
        let mut rewrite = |key: ResKey, n: usize| {
            let path = project.source_path(&key).unwrap().to_path_buf();
            let name = path.display().to_string();
            let mut g = conversion.from_source(&name, &std::fs::read(&path).unwrap()).unwrap();
            if key.restype == ResType::GIT {
                for list in ["Placeable List", "Creature List", "WaypointList"] {
                    for s in g.root.list_mut(list).into_iter().flatten() {
                        for f in ["X", "XPosition"] {
                            if let Some(x) = s.float(f) {
                                s.set(f, Value::Float(x + 0.5));
                            }
                        }
                    }
                }
            } else {
                g.root.set("Tag", Value::String(format!("PULLED{n}").into_bytes()));
            }
            let text = conversion.to_source(key.restype, &g).unwrap().into_bytes();
            let resource = conversion.from_source(&name, &text).unwrap().to_bytes().unwrap();
            plan.push((path, Some(text)));
            expected.insert(key, Some(resource));
        };
        let mut are: Vec<ResKey> = ws.module.keys_of(ResType::ARE).copied().collect();
        are.sort();
        for (n, k) in are.iter().take(areas).enumerate() {
            rewrite(*k, n);
            rewrite(ResKey::new(k.resref, ResType::GIT), n);
        }
        let mut bps: Vec<ResKey> = [ResType::UTI, ResType::UTC, ResType::UTP]
            .iter()
            .flat_map(|t| ws.module.keys_of(*t).copied().collect::<Vec<_>>())
            .filter(|k| k.resref.to_string().starts_with("pw_"))
            .collect();
        bps.sort();
        let step = (bps.len() / blueprints.max(1)).max(1);
        for (n, k) in bps.iter().step_by(step).take(blueprints).enumerate() {
            rewrite(*k, n);
        }
        let mut nss: Vec<ResKey> = ws.module.keys_of(ResType::NSS).copied().collect();
        nss.sort();
        for k in nss.iter().take(scripts) {
            let path = project.source_path(k).unwrap().to_path_buf();
            let mut text = std::fs::read(&path).unwrap();
            text.extend_from_slice(b"\n// pulled\n");
            plan.push((path, Some(text.clone())));
            expected.insert(*k, Some(text));
        }
        for k in nss.iter().rev().take(removed) {
            plan.push((project.source_path(k).unwrap().to_path_buf(), None));
            expected.insert(*k, None);
        }
        for n in 0..added {
            let text = format!("void main() {{ int pulled{n}; }}\n").into_bytes();
            plan.push((dir.join(format!("src/pulled{n:03}.nss")), Some(text.clone())));
            let k = ResKey::parse(&format!("pulled{n:03}"), ResType::NSS).unwrap();
            expected.insert(k, Some(text));
        }
        // (Changed here too, unsaved: kept, and asked about.)
        (bps[0], project.source_path(&bps[0]).unwrap().to_path_buf())
    };
    let edit = mg_edit::Edit::SetField {
        key: mine,
        path: mg_edit::GffPath::root(),
        label: "Tag".into(),
        value: Some(Value::String(b"MINE".to_vec())),
    };
    h.state_mut().actions.push(Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
    h.run_steps(2);
    println!(
        "  the pull: {} files ({areas} areas' .are and .git, {blueprints} blueprints, {scripts} \
         scripts rewritten, {added} added, {removed} deleted)",
        plan.len()
    );

    // The pull, on a thread of its own, over about a second.
    reset_peak();
    let (rss_before, _) = memory();
    let start = Instant::now();
    let pull = std::thread::spawn(move || {
        for (i, (path, text)) in plan.into_iter().enumerate() {
            match text {
                Some(t) => std::fs::write(&path, t).unwrap(),
                None => std::fs::remove_file(&path).unwrap(),
            }
            if i % 2 == 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    });
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN);
    let is_in = |app: &Moonglow, k: &ResKey| {
        let m = &app.ws.as_ref().unwrap().module;
        m.get(k) == expected[k].as_deref()
    };
    let mut times: Vec<(Duration, Duration)> = Vec::new();
    let (mut all_in, mut until) = (None, start + Duration::from_secs(90));
    // (Every file looked at, a few times a second: the last of each kind
    // in says nothing of the others when the watch's first look caught
    // those last ones, and its next look is 3 s on.)
    let mut looked = start;
    while Instant::now() < until {
        let f = frames(&mut h, screen, Pointer::Moves, 1, |_, _| {});
        times.push((start.elapsed(), f.ui[0] + f.tessellate[0]));
        if all_in.is_none() && pull.is_finished() && looked.elapsed() > Duration::from_millis(250) {
            looked = Instant::now();
            if expected.keys().all(|k| *k == mine || is_in(h.state(), k)) {
                all_in = Some(start.elapsed());
                until = Instant::now() + Duration::from_secs(2);
            }
        }
    }
    let (rss_after, peak) = memory();
    pull.join().unwrap();
    let longest = times.iter().map(|(_, d)| *d).max().unwrap_or_default();
    let at = times.iter().find(|(_, d)| *d == longest).map_or(Duration::ZERO, |(t, _)| *t);
    let over = times.iter().filter(|(_, d)| *d > BUDGET).count();
    let cpu: Vec<Duration> = times.iter().map(|(_, d)| *d).collect();
    println!(
        "  reload after a pull: {} frames, median {:.2} ms, longest {:.1} ms (at {:.1} s), {over} \
         over the budget; all in after {}; memory {rss_before} MiB before, {rss_after} after, \
         {peak} at most",
        times.len(),
        ms(pick(cpu, 0.5)),
        ms(longest),
        at.as_secs_f64(),
        all_in.map_or("never (90 s)".into(), |d| format!("{:.1} s", d.as_secs_f64())),
    );
    if r.detail {
        let slow: Vec<String> = (times.iter())
            .filter(|(_, d)| *d > BUDGET)
            .map(|(t, d)| format!("{:.1} ms at {:.2} s", ms(*d), t.as_secs_f64()))
            .collect();
        println!("      over the budget: {}", slow.join(", "));
    }
    // All of it in, the conflict kept and asked about.
    let wrong: Vec<String> = expected
        .keys()
        .filter(|k| **k != mine && !is_in(h.state(), k))
        .map(ToString::to_string)
        .collect();
    assert!(all_in.is_some() && wrong.is_empty(), "not read again: {wrong:?}");
    let ws = h.state_mut().ws.as_mut().unwrap();
    assert_eq!(ws.doc(&mine).unwrap().root.string("Tag"), Some(&b"MINE"[..]), "kept");
    assert_eq!(h.state().outside_conflicts(), [mine_path]);
    if longest > BUDGET {
        r.over.push(format!("reload after a pull: longest frame {:.1} ms", ms(longest)));
    }
    println!("over the budget:\n  {}", r.over.join("\n  "));
}

/// Grass's cost: a 32 by 32 rural field (ttr01, grass on nearly every
/// tile) in the area view, with its grass and without: seen close to the
/// ground, from low down across it, from high up and from straight above
/// (all of it in sight), and orbited from halfway up; those taken apart
/// ([`area_parts`], with the Shaders switch off and on); and the tufts
/// grown again as a drag on Area Properties › Grass grows them each frame.
/// `FRAME_GRASS_DENSITY` sets the field's density (the tileset's 5
/// otherwise), `FRAME_GRASS_SHADERS=0` turns the Shaders switch off (on,
/// as the app has it by default, otherwise). Needs the game and a GPU; run
/// in release:
/// `cargo test --release -p mg-ui --test frame_perf grass -- --ignored --nocapture`.
#[test]
#[ignore]
fn grass_frames_at_144() {
    use mg_area::grass::{AreaGrass, GrassOverride};
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let install = GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Grass", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 32,
        height: 32,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let set = mg_area::tileset(&game, ResRef::from_str("ttr01").unwrap()).ok();
    if let Some(density) = std::env::var("FRAME_GRASS_DENSITY").ok().and_then(|d| d.parse().ok()) {
        let key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&key).unwrap().unwrap();
        let mut o = GrassOverride::new(3, set.as_ref());
        o.look.density = density;
        let props = git.root.child_mut(mg_area::grass::PROPERTIES).unwrap();
        mg_module::grass::set_in(props, &o);
        m.set_gff(key, &git).unwrap();
    }
    let path = mg_testkit::scratch_dir("frame-grass").join("grass.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut r = Report::new();
    let rs = render_state();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    // (The Shaders switch as the app has it by default; `FRAME_GRASS_SHADERS=0`
    // off.)
    let shaders = std::env::var("FRAME_GRASS_SHADERS").map_or(true, |v| v != "0");
    app.settings.game_shaders = Some(shaders);
    println!("  Shaders {}", if shaders { "on" } else { "off" });
    app.set_render_state(rs.clone());
    app.open_module(&path);
    let mut h = harness(app, rs);
    h.run_steps(3);
    let tab = Tab::Area(area);
    open(&mut h, tab.clone());
    h.run_steps(5);
    // (The game's programs made, the grass's among them, before anything
    // is timed or drawn: low over the field, grass in sight.)
    if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
        (o.target, o.pitch, o.distance) = (glam::Vec3::new(160.0, 160.0, 0.0), 0.3, 50.0);
    }
    let start = Instant::now();
    h.run_steps(2);
    while h.state().viewport.as_ref().is_some_and(|v| v.renderer.drawn.game_pending > 0)
        && start.elapsed() < Duration::from_secs(60)
    {
        std::thread::sleep(Duration::from_millis(5));
        h.run_steps(1);
    }
    // The field seen close to the ground, from low down across it (the
    // grass running out to the far end of the view), from high up (all of
    // the field in sight), from straight above as it opens, and orbited
    // from halfway up a little faster; each turning as it is timed. (Name,
    // pitch, distance, turn a frame.)
    let views = [
        ("close", 0.7, 25.0, 0.01),
        ("across", 0.3, 50.0, 0.01),
        ("high", 0.9, 420.0, 0.01),
        ("from above", 1.5695, 464.0, 0.01),
        ("orbiting", 0.6, 150.0, 0.03),
    ];
    for (name, pitch, distance, turn) in views {
        for no_grass in [false, true] {
            h.state_mut().settings.no_grass = no_grass;
            let what = format!("field, {name}, {}", if no_grass { "no grass" } else { "grass" });
            let set = |app: &mut Moonglow, turned: f32| {
                if let Some(o) = app.area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
                    o.target = glam::Vec3::new(160.0, 160.0, 0.0);
                    o.pitch = pitch;
                    o.distance = distance;
                    o.yaw += turned;
                }
            };
            // (A picture of it: the view set, and its grass grown, first.)
            if r.shots {
                set(h.state_mut(), 0.0);
                h.state_mut().area_views.get_mut(&area).unwrap().grid = false;
                if let Some(o) = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut() {
                    o.yaw = -60f32.to_radians();
                }
                h.run_steps(12);
            }
            view_with(&mut h, &mut r, &what, Some(&tab), |app, _| set(app, turn));
        }
    }
    // Taken apart, with the grass.
    if r.only.is_none() && !r.shots {
        area_parts(&mut h, area, r.count, &[false, true], true);
    }
    // Grown again, as each frame of a drag grows it.
    let ws_root = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&ResKey::new(area, ResType::GIT)).unwrap().root.clone()
    };
    let mut grass = AreaGrass::read(&ws_root);
    let mut o = GrassOverride::new(3, set.as_ref());
    let gpu = h.state().viewport.as_ref().unwrap().gpu.clone();
    let model = h.state().area_views[&area].model.clone().unwrap();
    let mut scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let mut times = Vec::new();
    for k in 0..10 {
        o.look.density = 5.0 + k as f32 * 0.1;
        grass.overrides = vec![o.clone()];
        let growing = grass.growing(set.as_ref());
        let t = Instant::now();
        scene.update_grass(&gpu, &game, &model, &growing);
        times.push(t.elapsed());
    }
    println!("  grown again (a drag's frame): {:.2} ms", ms(pick(times, 0.5)));
    for over in &r.over {
        println!("OVER: {over}");
    }
}

/// The Shaders switch's cost (`docs/shaders-proposal.md`, M3): a large
/// area of stock content (the campaign's largest) and a large area of The
/// Domains of Dread's own, with its haks and their custom shaders
/// (`MOONGLOW_HAKS`, [`tdod_area`]), each in the viewer with the switch off
/// and then on (its opening timed: on, the game's programs are made), and
/// taken apart off and on. Needs the game and a GPU; run in release:
/// `cargo test --release -p mg-ui --test frame_perf game_shaders -- --ignored --nocapture`.
/// `FRAME_AREA=stock`, `water` ([`water_area`]: water, grass, fires,
/// a skybox) or `tdod` times only that one.
#[test]
#[ignore]
fn game_shaders_frames_at_144() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let mut r = Report::new();
    let only = std::env::var("FRAME_AREA").ok();
    let wanted = |name: &str| only.as_deref().is_none_or(|o| o == name);
    println!("the Shaders switch: frames at {SCREEN:?}, budget {:.2} ms", ms(BUDGET));
    let path = root.join("data/nwm").join(CAMPAIGN);
    if wanted("stock") && path.is_file() {
        let rs = render_state();
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        app.set_render_state(rs.clone());
        app.open_module(&path);
        let (area, tiles) = *areas(&mut app).last().unwrap();
        let mut h = harness(app, rs);
        shaders_off_and_on(&mut h, &mut r, area, tiles);
        drop(h);
        shaders_opening(&root, None, &path, area);
    }
    if wanted("water") {
        let (module, area, tiles) = water_area(&root);
        let rs = render_state();
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        app.set_render_state(rs.clone());
        app.open_module(&module);
        let mut h = harness(app, rs);
        shaders_off_and_on(&mut h, &mut r, area, tiles);
        drop(h);
    }
    if wanted("tdod") {
        match tdod_area(&root) {
            Some((module, user, area, tiles)) => {
                let rs = render_state();
                let install = GameInstall::new(&root, Some(user.clone()), "en");
                let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
                app.set_render_state(rs.clone());
                app.open_module(&module);
                let mut h = harness(app, rs);
                shaders_off_and_on(&mut h, &mut r, area, tiles);
                drop(h);
                shaders_opening(&root, Some(user), &module, area);
            }
            None => println!("skipped The Domains of Dread: no tdod_t_tap.hak in MOONGLOW_HAKS"),
        }
    }
    println!("over the budget:\n  {}", r.over.join("\n  "));
}

/// A stock area of water, grass and fires (M5's special surfaces): 16 by
/// 16 of the rural tileset (`ttr01`), every other pair of rows all water
/// (tile 20, `ttr01_b20_01`: shiny water), the rest its grassy tiles, a
/// skybox, a medium wind, and 160 braziers, campfires and flames (their
/// particles). The module, the area and its tiles.
fn water_area(root: &std::path::Path) -> (std::path::PathBuf, ResRef, i64) {
    use mg_gff::{Gff, Value};
    use mg_module::ModuleLocation;
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let game = mg_rules::GameData::open(&GameInstall::new(root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Water and fires", &mut rng).unwrap();
    let side = 16;
    let spec = AreaSpec {
        name: "Lake".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: side,
        height: side,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let key = ResKey::new(area, ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    are.root.set("WindPower", Value::Int(1));
    are.root.set("SkyBox", Value::Byte(1));
    for (i, t) in are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
        if (i / side as usize) % 4 < 2 && !no_water() {
            t.set("Tile_ID", Value::Int(20));
            t.set("Tile_Height", Value::Int(0));
        }
    }
    m.set_gff(key, &are).unwrap();
    // Braziers, campfires (with a spit), large and medium flames
    // (placeables.2da rows 57, 58, 60, 149, 150), on the grass.
    let rows = [57u32, 58, 60, 149, 150];
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let span = side as f32 * 10.0;
    let placed: Vec<_> = (0..160)
        .map(|i| {
            let mut root = bp.root.clone();
            root.set("Appearance", Value::Dword(rows[i % rows.len()]));
            // (Rows 2 and 3 of every four: the grass.)
            let band = (i % (side as usize / 4)) as f32 * 40.0 + 20.0 + rng.f32() * 20.0;
            let at = Placement { position: [rng.f32() * span, band, 0.0], rotation: 0.0 };
            instance(&placing, ResType::UTP, &root, at, &[]).unwrap()
        })
        .collect();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set("Placeable List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    let path = mg_testkit::scratch_dir("frame-water").join("water.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    println!("Water and fires: {side} by {side} of ttr01, half water, 160 fires");
    (path, area, i64::from(side) * i64::from(side))
}

/// An area opened with its module in a new window, the switch on from the
/// start (the module's own frames counted: it may open the area itself), its
/// programs' cache folder empty, then again with it filled: the frames of
/// its opening (the longest, and the time until its models show), and the
/// time until the game's programs draw it (their pipelines made while the
/// frames go on).
fn shaders_opening(
    root: &std::path::Path,
    user: Option<std::path::PathBuf>,
    module: &std::path::Path,
    area: ResRef,
) {
    let cache = mg_testkit::scratch_dir("frame-shader-cache");
    for run in ["the cache empty", "the cache filled"] {
        let rs = render_state();
        let install = GameInstall::new(root, user.clone(), "en");
        let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
        app.set_render_state(rs.clone());
        app.shader_cache_dir = Some(cache.clone());
        app.settings.game_shaders = Some(true);
        // (From the module's opening: it may open the area itself.)
        let t0 = Instant::now();
        app.open_module(module);
        let mut h = harness(app, rs);
        let tab = Tab::Area(area);
        h.state_mut().actions.push(Action::OpenTab(tab.clone()));
        let opened = t0.elapsed();
        let (mut longest, mut frames) = (Duration::ZERO, 0);
        let (mut shown_at, mut drawn_at) = (None, None);
        while t0.elapsed() < Duration::from_secs(60) {
            let t = Instant::now();
            h.run_steps(1);
            longest = longest.max(t.elapsed());
            frames += 1;
            let app = h.state();
            if shown_at.is_none() && app.area_views.get(&area).is_some_and(|v| v.model.is_some()) {
                shown_at = Some(t0.elapsed());
            }
            let drawn = app.viewport.as_ref().map(|v| v.renderer.drawn);
            if shown_at.is_some() && drawn.is_some_and(|d| d.game > 0 && d.game_pending == 0) {
                drawn_at = Some(t0.elapsed());
                break;
            }
        }
        let vp = h.state().viewport.as_ref();
        let stats = vp.map(|v| v.renderer.game_shader_stats()).unwrap_or_default();
        let failures = vp.map(|v| v.renderer.game_shader_failures().to_vec()).unwrap_or_default();
        if !failures.is_empty() {
            println!("  the game's shaders that couldn't be drawn: {failures:?}");
        }
        println!(
            "  area {area} opened with the Shaders on, {run}: the module opened in {:.0} ms, then \
             its longest frame {:.0} ms of {frames}; \
             shown after {:.0} ms, drawn by the game's programs after {:.0} ms ({} programs \
             compiled, {} read from the cache, {} pipelines made in {:.0} ms of the worker's)",
            ms(opened),
            ms(longest),
            shown_at.map_or(f64::NAN, ms),
            drawn_at.map_or(f64::NAN, ms),
            stats.compiled,
            stats.cached,
            stats.pipelines,
            stats.seconds * 1000.0,
        );
        close(&mut h, &tab);
    }
}

/// An area of an opened module in the viewer with the switch off, then on;
/// then taken apart, off and on.
fn shaders_off_and_on(h: &mut App<'_>, r: &mut Report, area: ResRef, tiles: i64) {
    h.run_steps(3);
    close(h, &Tab::Palette);
    close(h, &Tab::ModuleProperties);
    let shown: Vec<Tab> = h
        .state()
        .dock
        .iter_all_tabs()
        .map(|(_, t)| t.clone())
        .filter(|t| matches!(t, Tab::Area(_)))
        .collect();
    for t in &shown {
        close(h, t);
    }
    for on in [false, true] {
        h.state_mut().settings.game_shaders = Some(on);
        let label = if on { ", Shaders on" } else { ", Shaders off" };
        area_views(h, r, area, tiles, label);
    }
    // (`FRAME_OWN_WATER=1`: on again, shiny water by Moonglow's own, as
    // before the framebuffer's copies.)
    if own_water() {
        if let Some(vp) = h.state_mut().viewport.as_mut() {
            vp.renderer.game_water = false;
        }
        area_views(h, r, area, tiles, ", Shaders on, Moonglow's water");
        if let Some(vp) = h.state_mut().viewport.as_mut() {
            vp.renderer.game_water = mg_render::GAME_WATER;
        }
    }
    // (`FRAME_HARD_PARTICLES=1`: on again, particles without their fade.)
    if hard_particles() {
        if let Some(vp) = h.state_mut().viewport.as_mut() {
            vp.renderer.soft_particles = false;
        }
        area_views(h, r, area, tiles, ", Shaders on, hard particles");
        if let Some(vp) = h.state_mut().viewport.as_mut() {
            vp.renderer.soft_particles = true;
        }
    }
    if !r.shots {
        area_parts(h, area, r.count, &[false, true], false);
    }
}

/// A large area of The Domains of Dread's (`MOONGLOW_HAKS`): 24 by 24 of its
/// `mst04` tiles (the wind shaders' trees, plants and grass), the set's
/// tiles in turn, and 400 of its placeables whose models are in
/// `tdod_p_tap` and `tdod_p_ham` (grass, leaves, falling water: custom
/// shaders among them), in a scratch module with the shader haks attached
/// and a scratch user folder holding them. `FRAME_TDOD_LAMPS=<n>`: the
/// first n of them the Txpple pack's light-node placeables (`dd_tap_lightn*`:
/// each a light reaching up to 300 m, as in the game), for timing an area
/// full of lights. The module, the user folder, the area and its tiles.
fn tdod_area(
    root: &std::path::Path,
) -> Option<(std::path::PathBuf, std::path::PathBuf, ResRef, i64)> {
    use mg_gff::{Gff, Value};
    use mg_module::ModuleLocation;
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let haks = mg_testkit::builder_hak_dirs()
        .into_iter()
        .find(|d| d.join("tdod_t_tap.hak").is_file() && d.join("tdod_shaders.hak").is_file())?;
    let dir = mg_testkit::scratch_dir("frame-tdod-shaders");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    // (Highest priority first, as the module lists them.)
    let names = ["tdod_shaders", "tdod_t_tap", "tdod_p_tap", "tdod_p_ham", "tdod_top"];
    for name in names {
        let file = format!("{name}.hak");
        mg_testkit::link_or_copy(&haks.join(&file), &user.join("hak").join(&file)).unwrap();
    }
    let install = GameInstall::new(root, Some(user.clone()), "en");
    let mut game = mg_rules::GameData::open(&install).unwrap();
    let mut models = std::collections::HashSet::new();
    for name in names.iter().rev() {
        let c = mg_resman::ErfContainer::open(&haks.join(format!("{name}.hak"))).unwrap();
        if matches!(*name, "tdod_p_tap" | "tdod_p_ham") {
            models.extend(
                c.index()
                    .iter()
                    .filter(|(k, _)| k.restype == ResType::MDL)
                    .map(|(k, _)| k.resref.to_string().to_ascii_lowercase()),
            );
        }
        game.resman.add(
            mg_resman::priority::HAK_USER,
            format!("hak:{name}"),
            mg_resman::LayerClass::Erf,
            c,
        );
    }
    game.invalidate();
    let set = mg_area::tileset(&game, ResRef::from_str("mst04").unwrap()).ok()?;
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "TDoD Shaders", &mut rng).unwrap();
    let side = 24;
    let spec = AreaSpec {
        name: "Woods".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: side,
        height: side,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let key = ResKey::new(area, ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    are.root.set("Tileset", Value::ResRef(b"mst04".to_vec()));
    // (A medium wind: the sway shaders move.)
    are.root.set("WindPower", Value::Int(1));
    let count = set.tiles.len().max(1);
    for (i, t) in are.root.list_mut("Tile_List").unwrap().iter_mut().enumerate() {
        t.set("Tile_ID", Value::Int((i % count) as i32));
        t.set("Tile_Orientation", Value::Int((i / count % 4) as i32));
        t.set("Tile_Height", Value::Int(0));
    }
    m.set_gff(key, &are).unwrap();
    // Its placeables, by their rows of placeables.2da.
    let table = game.table("placeables").ok()?;
    let rows: Vec<usize> = (0..table.len())
        .filter(|&row| {
            table.get(row, "ModelName").is_some_and(|n| models.contains(&n.to_ascii_lowercase()))
        })
        .collect();
    if rows.is_empty() {
        return None;
    }
    let lamps: Vec<usize> = (0..table.len())
        .filter(|&row| {
            table
                .get(row, "ModelName")
                .is_some_and(|n| n.to_ascii_lowercase().starts_with("dd_tap_lightn"))
        })
        .collect();
    let lamp_count: usize =
        std::env::var("FRAME_TDOD_LAMPS").ok().and_then(|n| n.parse().ok()).unwrap_or(0);
    let bp = game.resman.get_named("plc_armoire", ResType::UTP).unwrap();
    let mut bp = Gff::read(&bp).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let span = side as f32 * 10.0;
    let placed: Vec<_> = (0..400)
        .map(|i| {
            let row = if i < lamp_count && !lamps.is_empty() {
                lamps[i % lamps.len()]
            } else {
                rows[i % rows.len()]
            };
            bp.root.set("Appearance", Value::Dword(row as u32));
            let at = Placement {
                position: [rng.f32() * span, rng.f32() * span, 0.0],
                rotation: rng.f32() * 360.0,
            };
            instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap()
        })
        .collect();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    git.root.set("Placeable List", Value::List(placed));
    m.set_gff(git_key, &git).unwrap();
    let ifo_key = ResKey::parse("module", ResType::IFO).unwrap();
    let mut ifo = m.gff(&ifo_key).unwrap().unwrap();
    let list = mg_module::attach::hak_list(&ifo.root, &names.map(String::from));
    ifo.root.set("Mod_HakList", list);
    m.set_gff(ifo_key, &ifo).unwrap();
    let path = dir.join("tdod-shaders.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    println!(
        "The Domains of Dread: {side} by {side} of mst04's {count} tiles, 400 placeables of {} rows \
         ({} of them light-node placeables)",
        rows.len(),
        lamp_count.min(400)
    );
    Some((path, user, area, i64::from(side) * i64::from(side)))
}
