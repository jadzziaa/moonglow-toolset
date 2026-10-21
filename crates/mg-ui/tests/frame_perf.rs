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
//! and `FRAME_TARGET=<width>x<height>` sizes the pictures of the areas
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
/// turned about, and close to the ground. `label` follows its name in what
/// is printed (the Shaders switch, say).
fn area_views(h: &mut App<'_>, r: &mut Report, area: ResRef, tiles: i64, label: &str) {
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
        let all: Duration = opening.iter().sum();
        println!(
            "  area {area}{label}: opening, its longest frame {:.0} ms ({} frames, {:.0} ms in all)",
            ms(longest),
            opening.len(),
            ms(all)
        );
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
    close(h, &tab);
}

/// The area viewer's frame taken apart, on a renderer of the test's own:
/// the scene built (every tile and object posed), the particles moved on,
/// the draws encoded and sent, and the GPU drawing them; with the Shaders
/// switch as each of `shaders` has it (a renderer of its own each: the
/// first frame, which reads the textures and, with the switch on, makes
/// the game's programs, is printed by itself).
fn area_parts(h: &mut App<'_>, area: ResRef, count: usize, shaders: &[bool]) {
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
    let configs = [
        ("overview, 4x MSAA", span * 1.45, 1.5695, 4, true),
        ("…a draw for each mesh", span * 1.45, 1.5695, 4, false),
        ("overview, no MSAA", span * 1.45, 1.5695, 1, true),
        ("close, 4x MSAA", 25.0, 0.7, 4, true),
    ];
    for (what, distance, pitch, samples, instancing, on) in
        configs.into_iter().flat_map(|c| shaders.iter().map(move |&g| (c.0, c.1, c.2, c.3, c.4, g)))
    {
        // (A draw for each mesh is the switch off's comparison only.)
        if on && !instancing {
            continue;
        }
        let mut renderer = Renderer::new(&gpu, format, samples);
        renderer.instancing = instancing;
        renderer.game_shaders = on;
        let mut first = Duration::ZERO;
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
            if n == 0 {
                first = t4 - t0;
            }
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
        let what = if on { format!("{what}, Shaders on") } else { what.to_string() };
        println!(
            // (The GPU's least times too: with another program drawing,
            // the usual one is as much that program's.)
            "    {what:32} first frame {:6.0} ms, scene {:5.2} ms, particles {:5.2} ms, draws \
             encoded {:5.2} ms (worst {:5.2}), GPU {:5.2} ms (the quickest tenth {:5.2}) \
             ({instances} instances, {meshes} meshes, {lights} lights, {quads} particles; {} \
             meshes in {} draws of {} materials in turn, {} of the game's, {} meshes out of \
             sight)",
            ms(first),
            ms(pick(build, 0.5)),
            ms(pick(particles, 0.5)),
            ms(pick(encode.clone(), 0.5)),
            ms(pick(encode, 0.99)),
            ms(pick(drawn.clone(), 0.5)),
            ms(pick(drawn, 0.1)),
            did.meshes,
            did.batches,
            did.materials,
            did.game,
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
            area_parts(&mut h, *area, r.count, &[false]);
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
    tab(&mut h, &mut r, "resource browser (300,000)", Tab::Resources);
    let areas = h.state().ws.as_ref().unwrap().module.areas().unwrap();
    tab(&mut h, &mut r, "Area Properties (300 areas)", Tab::AreaProperties(areas[0]));
    println!("over the budget:\n  {}", r.over.join("\n  "));
}

/// Grass's cost: a 32 by 32 rural field (ttr01, grass on nearly every
/// tile) seen close to the ground, with the area view's grass and without,
/// and the tufts grown again as a drag on Area Properties › Grass grows
/// them each frame. `FRAME_GRASS_DENSITY` sets the field's density (the
/// tileset's 5 otherwise). Needs the game and a GPU; run in release:
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
    app.set_render_state(rs.clone());
    app.open_module(&path);
    let mut h = harness(app, rs);
    h.run_steps(3);
    let tab = Tab::Area(area);
    open(&mut h, tab.clone());
    h.run_steps(5);
    let close_in = |app: &mut Moonglow, _: usize| {
        if let Some(o) = app.area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
            o.target = glam::Vec3::new(160.0, 160.0, 0.0);
            o.pitch = 0.7;
            o.distance = 25.0;
            o.yaw += 0.01;
        }
    };
    for no_grass in [false, true] {
        h.state_mut().settings.no_grass = no_grass;
        let what = if no_grass { "field, close, no grass" } else { "field, close, grass" };
        view_with(&mut h, &mut r, what, Some(&tab), close_in);
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
/// `FRAME_AREA=stock` or `tdod` times only that one.
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
        app.open_module(module);
        // (From the module's first frame: it may open the area itself.)
        let mut h = harness(app, rs);
        let tab = Tab::Area(area);
        h.state_mut().actions.push(Action::OpenTab(tab.clone()));
        let t0 = Instant::now();
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
            "  area {area} opened with the Shaders on, {run}: its longest frame {:.0} ms of {frames}; \
             shown after {:.0} ms, drawn by the game's programs after {:.0} ms ({} programs \
             compiled, {} read from the cache, {} pipelines made in {:.0} ms of the worker's)",
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
    if !r.shots {
        area_parts(h, area, r.count, &[false, true]);
    }
}

/// A large area of The Domains of Dread's (`MOONGLOW_HAKS`): 24 by 24 of its
/// `mst04` tiles (the wind shaders' trees, plants and grass), the set's
/// tiles in turn, and 400 of its placeables whose models are in
/// `tdod_p_tap` and `tdod_p_ham` (grass, leaves, falling water: custom
/// shaders among them), in a scratch module with the shader haks attached
/// and a scratch user folder holding them. The module, the user folder,
/// the area and its tiles.
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
    let bp = game.resman.get_named("plc_armoire", ResType::UTP).unwrap();
    let mut bp = Gff::read(&bp).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let span = side as f32 * 10.0;
    let placed: Vec<_> = (0..400)
        .map(|i| {
            bp.root.set("Appearance", Value::Dword(rows[i % rows.len()] as u32));
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
        "The Domains of Dread: {side} by {side} of mst04's {count} tiles, 400 placeables of {} rows",
        rows.len()
    );
    Some((path, user, area, i64::from(side) * i64::from(side)))
}
