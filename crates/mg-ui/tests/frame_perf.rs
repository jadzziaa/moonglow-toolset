//! Frame times against a 144 fps budget: 6.94 ms for all a frame does on
//! the CPU (the interface laid out and tessellated, a 3D view's scene posed
//! and its draws encoded), and as long again for the GPU to draw it. Each
//! view is timed as the desktop app runs it (no accessibility tree, the
//! clock moving on by a 144th of a second a frame), at the development
//! machine's screen (3440×1440, in a window 3440×1400: the screen less its
//! taskbar, as a maximized window has it): with the pointer moving over it, with the
//! wheel turning, and left alone (where it says how soon it asks to be
//! drawn again). The views over the budget are named at the end, and a
//! test fails where one is over it by more than [`MARGIN`] and not among
//! those the deferred list holds ([`KNOWN_OVER`]). (An area's picture is kept between its animations' steps, so its
//! view's usual frame is short and the frame that draws it is the worst:
//! "spikes" says that one is over the budget.) Needs the game (Tyrants of the Moonsea) and a GPU; run in release:
//! `cargo test --release -p mg-ui --test frame_perf -- --ignored --nocapture`.
//! `FRAME_SHOTS=1` saves each view as it was timed instead
//! (`target/test-output/frame-shots`; an area's animations at a fixed
//! clock, [`SHOT_CLOCK`], so that two runs' pictures compare). `FRAME_VIEW=<part of a name>` times
//! only those views, `FRAME_COUNT=<frames>` that many frames of each,
//! `FRAME_DETAIL=1` prints each frame's time and who asked for the next,
//! `FRAME_ACCESSKIT=1` builds the accessibility tree every frame, as the app
//! does while an assistive client is connected (a screen reader, a desktop
//! agent), `FRAME_SCREEN=<width>x<height>` sizes the window (3440x1400
//! unless given; the shell yields to it as to any window), and
//! `FRAME_TARGET=<width>x<height>` sizes the pictures of the areas
//! taken apart at the end, and `FRAME_SHADOWS=sun,creatures,lights`
//! (any of them, or `off`) draws those kinds of shadow in every area, and
//! `FRAME_SHADOW_QUALITY=high` draws them as stencil volumes (Options ›
//! Area › Shadows quality High), `FRAME_MEMORY=1` (or `labels`) says what
//! the GPU holds after each view (and what of it), `FRAME_KEEP_TEXTURES=1`
//! keeps the textures of what was shown before an area opens, and
//! `FRAME_STALE_KEPT=<MiB>` keeps that much of those not drawn lately, and
//! `FRAME_TEXTURE_LIMIT=<pixels>` is Options › Area › Largest textures,
//! and `FRAME_UNCOMPRESSED=1` uploads TGAs as RGBA8, not compressed as
//! they are read, and `FRAME_AREAS=<resref,…>` opens those areas instead
//! of the campaign's middle and largest, and `FRAME_SAMPLES=1` (or `2`) is
//! Options › Area › Anti-aliasing Off (or 2×). (With another program drawing, the game say,
//! the GPU's times are as much that program's: close it for those.)

mod render_views;
#[allow(dead_code)]
mod world;

use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable};
use mg_core::{ResRef, ResType};
use mg_module::palette::BlueprintKind;
use mg_resman::{GameInstall, ResKey};
use mg_ui::{Action, Moonglow, NoDialogs, Tab};

/// The window's size: the development machine's screen less its taskbar
/// (3440 by 1400), or `FRAME_SCREEN=<width>x<height>` (the Narrow board's
/// 1280x800, say: the shell then yields as it does there, M3's T14).
fn screen_size() -> egui::Vec2 {
    static SIZE: std::sync::OnceLock<egui::Vec2> = std::sync::OnceLock::new();
    *SIZE.get_or_init(|| {
        std::env::var("FRAME_SCREEN")
            .ok()
            .and_then(|s| {
                let (w, h) = s.split_once('x')?;
                Some(egui::vec2(w.parse().ok()?, h.parse().ok()?))
            })
            .unwrap_or(egui::vec2(3440.0, 1400.0))
    })
}
const FPS: f64 = 144.0;
const BUDGET: Duration = Duration::from_micros(6_944);
const FRAMES: usize = 72;
/// How far past the budget a view's median goes before the test fails:
/// the same build's times move from run to run (a tenth, and more with
/// other programs busy).
const MARGIN: f64 = 1.25;
/// Views known to be over the budget, by a part of their names, and the
/// entry of `docs/deferred.md` that holds each: named, but not failed.
const KNOWN_OVER: &[(&str, &str)] = &[
    // 13 to 16 ms seen whole, 21 to 28 turning, 2026-10-27.
    ("area woods (", "A dense area seen whole is still slower with the switch on"),
    // The wheel zooming over the whole of area084 (16,000 meshes drawn):
    // 7.5 to 9.6 ms, 2026-11-02 (9.1 to 10.8 before that day).
    ("549 objects) (Sound Ranges off), overview (wheel)", LARGEST),
    ("549 objects) (Sound Ranges off), turning (wheel)", LARGEST),
    ("549 objects) (Sound Ranges on), overview (wheel)", LARGEST),
    ("549 objects) (Sound Ranges on), turning (wheel)", LARGEST),
    // The same cost in the views after "close": their wheel zooms out
    // from where the view before left the camera, over the whole of
    // area084, since the area opens with its camera over its ground (M5's
    // V3; before, these views looked from under it and drew little):
    // 8.6 to 10.3 ms, 2026-11-21.
    ("549 objects) (Sound Ranges off), Paint on, two placeables chosen (wheel)", LARGEST),
    ("549 objects) (Sound Ranges off), Select Tiles on, a box of tiles selected (wheel)", LARGEST),
    (
        "549 objects) (Sound Ranges off), a creature's Inventory page in the Properties panel (wheel)",
        LARGEST,
    ),
    (
        "549 objects) (Sound Ranges off), 188 placeables selected, the Properties panel (wheel)",
        LARGEST,
    ),
    ("549 objects) (Sound Ranges off), the Outliner's Sounds open (317) (wheel)", LARGEST),
    ("549 objects) (Sound Ranges off), the Outliner's kinds closed, one selected (wheel)", LARGEST),
    ("549 objects) (Sound Ranges off), tiles palette searched as typed (wheel)", LARGEST),
    (
        "549 objects) (Sound Ranges off), the Right Dock in two columns (741), the Preview's model (wheel)",
        LARGEST,
    ),
    (
        "549 objects) (Sound Ranges off), the Right Dock in one column (320), the Palette in front (wheel)",
        LARGEST,
    ),
    ("549 objects) (Sound Ranges on), Paint on, two placeables chosen (wheel)", LARGEST),
    ("549 objects) (Sound Ranges on), Select Tiles on, a box of tiles selected (wheel)", LARGEST),
    (
        "549 objects) (Sound Ranges on), a creature's Inventory page in the Properties panel (wheel)",
        LARGEST,
    ),
    (
        "549 objects) (Sound Ranges on), 188 placeables selected, the Properties panel (wheel)",
        LARGEST,
    ),
    ("549 objects) (Sound Ranges on), the Outliner's Sounds open (317) (wheel)", LARGEST),
    ("549 objects) (Sound Ranges on), the Outliner's kinds closed, one selected (wheel)", LARGEST),
    ("549 objects) (Sound Ranges on), tiles palette searched as typed (wheel)", LARGEST),
    (
        "549 objects) (Sound Ranges on), the Right Dock in two columns (741), the Preview's model (wheel)",
        LARGEST,
    ),
    (
        "549 objects) (Sound Ranges on), the Right Dock in one column (320), the Palette in front (wheel)",
        LARGEST,
    ),
    // The whole of area084 drawn from 120 m while turning, every object
    // selected and the walkmesh shown: 11.4 to 13.0 ms, 2026-11-21 (the
    // owner's game client running). The views turn the camera every
    // frame, idle too, so the overlays' kept meshes are not used; with
    // the walkmesh and the selection both left out the same views took
    // 9.8 to 11.0 ms: the overlays are about 2 ms of it.
    ("549 objects) (Sound Ranges off), all selected, walkmesh (pointer)", LARGEST),
    ("549 objects) (Sound Ranges off), all selected, walkmesh (idle)", LARGEST),
    ("549 objects) (Sound Ranges off), all selected, walkmesh (wheel)", LARGEST),
    ("549 objects) (Sound Ranges on), all selected, walkmesh (pointer)", LARGEST),
    ("549 objects) (Sound Ranges on), all selected, walkmesh (idle)", LARGEST),
    ("549 objects) (Sound Ranges on), all selected, walkmesh (wheel)", LARGEST),
    // `area_editor_at_scale`, 2026-11-25 (load 9 to 12, other agents
    // building): an area of 5,000 placeables, one dragged, 10.2 to 10.3 ms
    // (its picture drawn again each frame: the largest areas' cost); all
    // of them selected and dragged, 18.7 to 19.1 ms (their boxes made anew
    // each frame, 3 ms, and the Properties panel of 5,000, 2.2 to 2.9).
    // (At load 4 to 5, those at rest are under the budget, 6.3 to 7.0 ms;
    // at load 12 to 16 they crossed its margin, 9.0 to 11.4.)
    ("area of 5,000 placeables, one selected, dragged", LARGEST),
    ("area of 5,000 placeables, all selected, dragged", TIMING),
];
/// The entry of `docs/deferred.md` for the review's costs without a
/// timing view, which holds what the area editor's views left.
const TIMING: &str = "The review of November 2026's performance fixes without a timing view";
/// The entry of `docs/deferred.md` for the largest areas' frames.
const LARGEST: &str = "The largest areas in full view";

const CAMPAIGN: &str = "Neverwinter Nights - Tyrants of the Moonsea.nwm";

/// The clock an area's animations are drawn at for `FRAME_SHOTS`, seconds.
const SHOT_CLOCK: f32 = 10.0;

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
    /// Rests at the point, still (on a menu's row, say, which a pointer
    /// moving across the view would close).
    Rests(egui::Pos2),
    /// Pressed at the first point, moved to the second and let go at the
    /// last frame: a drag (of a selection, in an area's view).
    Drags(egui::Pos2, egui::Pos2),
    /// Rests at the point with the key pressed every so many frames, as a
    /// key held down repeats (E turning the selection, in an area's view).
    Keys(egui::Pos2, egui::Key, usize),
    /// Rests at the point with the wheel turning down by so many points a
    /// frame (a form's pages on one scroll, top to bottom).
    Down(egui::Pos2, f32),
    /// Pressed at the point in the first frame and let go in the second, then
    /// resting there: a click (a button that opens a window).
    Clicks(egui::Pos2),
    /// The text typed in the first frame (into the field that has the
    /// keys), the pointer away.
    Types(&'static str),
}

struct Report {
    over: Vec<String>,
    /// Those over by more than [`MARGIN`] and not [`KNOWN_OVER`].
    failed: Vec<String>,
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
            failed: Vec::new(),
            shots: var("FRAME_SHOTS").is_some(),
            only: var("FRAME_VIEW"),
            count: var("FRAME_COUNT").and_then(|n| n.parse().ok()).unwrap_or(FRAMES),
            detail: var("FRAME_DETAIL").is_some(),
        }
    }

    /// `what` took `took` a frame: named if it is over the budget, failed
    /// if by more than [`MARGIN`] and not [`KNOWN_OVER`].
    fn took(&mut self, what: String, took: Duration) {
        if took <= BUDGET {
            return;
        }
        let known = KNOWN_OVER.iter().find(|(name, _)| what.contains(name));
        let line = format!("{what}: {:.2} ms", ms(took));
        match known {
            Some((_, entry)) => self.over.push(format!("{line} (known: \"{entry}\")")),
            None if took > BUDGET.mul_f64(MARGIN) => {
                self.over.push(line.clone());
                self.failed.push(line);
            }
            None => self.over.push(line),
        }
    }

    /// Names the views over the budget, and fails for those over it by
    /// more than [`MARGIN`] that aren't known.
    fn end(&self) {
        println!("over the budget:\n  {}", self.over.join("\n  "));
        assert!(
            self.failed.is_empty(),
            "over the budget ({:.2} ms) by more than {MARGIN}:\n  {}",
            ms(BUDGET),
            self.failed.join("\n  ")
        );
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
            Pointer::Rests(at) => input.events.push(egui::Event::PointerMoved(at)),
            Pointer::Keys(at, key, every) => {
                input.events.push(egui::Event::PointerMoved(at));
                if n % every.max(1) == 0 {
                    input.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: n > 0,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
            Pointer::Down(at, by) => {
                input.events.push(egui::Event::PointerMoved(at));
                input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -by),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            Pointer::Clicks(at) => {
                input.events.push(egui::Event::PointerMoved(at));
                if n < 2 {
                    input.events.push(egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed: n == 0,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
            Pointer::Types(text) => {
                if n == 0 {
                    input.events.push(egui::Event::Text(text.into()));
                }
            }
            Pointer::Drags(from, to) => {
                let at = from.lerp(to, n as f32 / (count - 1).max(1) as f32);
                input.events.push(egui::Event::PointerMoved(at));
                let last = n + 1 == count;
                if n == 0 || last {
                    input.events.push(egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed: !last,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
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
        upload(full.textures_delta);
        out.ui.push(t1 - t0);
        out.tessellate.push(t2 - t1);
        out.gpu.push(t3 - t2);
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

/// What the GPU holds of one kind: its name, MiB and how many.
type Held = (String, f64, usize);

/// What the GPU's allocator holds (wgpu's report: Vulkan and Direct3D 12),
/// MiB in all, and by what it is, largest first: a game texture under its
/// own name is counted among "textures by name", egui's textures together,
/// the rest by their labels (render targets, shadow maps, the meshes'
/// chunks, the draws' buffers).
fn vram(h: &App<'_>) -> Option<(f64, f64, Vec<Held>)> {
    let device = &h.state().viewport.as_ref()?.gpu.device;
    let report = device.generate_allocator_report()?;
    let mib = |b: u64| b as f64 / (1024.0 * 1024.0);
    let mut by: std::collections::HashMap<String, (u64, usize)> = Default::default();
    for a in &report.allocations {
        let words = ["color", "msaa", "depth", "thumbnail", "watch", "white", "game", "readback"];
        let what = if a.name.starts_with("egui") {
            "egui".to_string()
        } else if a.name.contains(' ') || words.contains(&a.name.as_str()) || a.name.is_empty() {
            a.name.clone()
        } else {
            "textures by name".to_string()
        };
        let e = by.entry(what).or_default();
        e.0 += a.size;
        e.1 += 1;
    }
    let mut by: Vec<(String, f64, usize)> =
        by.into_iter().map(|(k, (b, n))| (k, mib(b), n)).collect();
    by.sort_by(|a, b| b.1.total_cmp(&a.1));
    Some((mib(report.total_allocated_bytes), mib(report.total_reserved_bytes), by))
}

/// `FRAME_MEMORY=1`: a line of what the GPU holds after each view, and with
/// `FRAME_MEMORY=labels` what it is.
fn said_memory(h: &App<'_>, what: &str) {
    let Ok(how) = std::env::var("FRAME_MEMORY") else { return };
    let Some((allocated, reserved, by)) = vram(h) else {
        println!("  {what:44} memory: no allocator report on this backend");
        return;
    };
    println!("  {what:44} memory: {allocated:7.1} MiB allocated ({reserved:.1} MiB reserved)");
    if let Some(vp) = h.state().viewport.as_ref() {
        let t = vp.renderer.texture_memory();
        let mib = |b: u64| b as f64 / (1024.0 * 1024.0);
        let m = vp.gpu.mesh_memory();
        println!(
            "      meshes: {} chunks of {:.1} MiB, {:.1} MiB of them used",
            m.chunks,
            mib(m.held),
            mib(m.used)
        );
        println!(
            "      textures: {} of {:.1} MiB, {:.1} MiB uncompressed, {:.1} MiB stale, {} let go",
            t.textures,
            mib(t.bytes),
            mib(t.uncompressed),
            mib(t.stale),
            t.let_go
        );
    }
    if how == "labels" {
        // (The 14 largest, and the stencil volumes' whatever their size.)
        for (i, (label, mib, n)) in by.iter().enumerate() {
            if i < 14 || label.starts_with("shadow volume") {
                println!("      {mib:8.1} MiB  {label} ({n})");
            }
        }
    }
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
    each: impl FnMut(&mut Moonglow, usize),
) {
    let rect = tab.and_then(|t| rect_of(h.state(), t));
    view_in(h, r, what, rect, each);
}

/// [`view_with`] with the pointer moved and the wheel turned over `rect`
/// (the whole screen where `None`).
fn view_in(
    h: &mut App<'_>,
    r: &mut Report,
    what: &str,
    rect: Option<egui::Rect>,
    mut each: impl FnMut(&mut Moonglow, usize),
) {
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) {
        return;
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
    let rect = rect.unwrap_or(screen);
    if r.shots {
        // (The view as its frames have it: its first frame's change made;
        // an area's animations at a fixed clock, so that a run's pictures
        // are the run before's where tiles animate.)
        each(h.state_mut(), 0);
        h.run_steps(2);
        for v in h.state_mut().area_views.values_mut() {
            v.set_clock(SHOT_CLOCK);
        }
        h.run_steps(1);
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
        said(r, what, how, &f);
    }
    said_memory(h, what);
}

/// An area's camera once the zoom of the views before has stopped (the
/// wheel's zoom goes on a while after its frames).
fn resting_camera(h: &mut App<'_>, area: ResRef) -> Option<mg_ui::area_view::Orbit> {
    let orbit = |h: &App<'_>| h.state().area_views.get(&area).and_then(|v| v.orbit);
    for _ in 0..1000 {
        let was = orbit(h);
        h.run_steps(1);
        if orbit(h) == was {
            break;
        }
    }
    orbit(h)
}

/// Gives an area's camera back as [`resting_camera`] had it (`rest`): the
/// views after a view added among others are then timed from where they
/// were before it.
fn rest_camera(h: &mut App<'_>, area: ResRef, rest: Option<mg_ui::area_view::Orbit>) {
    resting_camera(h, area);
    if let (Some(rest), Some(v)) = (rest, h.state_mut().area_views.get_mut(&area)) {
        v.orbit = Some(rest);
    }
    h.run_steps(2);
    let now = h.state().area_views.get(&area).and_then(|v| v.orbit);
    assert_eq!(now, rest, "the camera given back");
}

/// Under 1400 an editor's Right Dock sections fold into one tab bar:
/// asserts the tabs named in order along the dock's top and `front`, a
/// label of the front panel's own, drawn in the dock under them.
fn tabs_folded(h: &App<'_>, tabs: &[&str], front: &str) {
    let dock = h.state().right_dock_rect().expect("no Right Dock");
    let drawn: Vec<(String, egui::Rect)> = h
        .query_all_by(|n| n.label().is_some() && n.bounding_box().is_some())
        .filter(|n| dock.contains_rect(n.rect()))
        .map(|n| (n.accesskit_node().label().unwrap_or_default().to_string(), n.rect()))
        .collect();
    let at = |name: &str| drawn.iter().find(|(l, _)| l == name).map(|(_, r)| *r);
    let mut was: Option<egui::Rect> = None;
    for name in tabs {
        let r = at(name).unwrap_or_else(|| panic!("no {name} tab in the folded Right Dock"));
        assert!(r.top() < dock.top() + 40.0, "{name} not in the dock's tab bar: {r:?}");
        if let Some(was) = was {
            assert!(r.left() > was.right() - 1.0, "{name} not after the tab before");
            assert!((r.center().y - was.center().y).abs() < 2.0, "{name} not on one row");
        }
        was = Some(r);
    }
    let bar = was.map_or(dock.top(), |r| r.bottom());
    assert!(
        drawn.iter().any(|(l, r)| l.contains(front) && r.top() >= bar),
        "{front:?} not drawn in the front panel"
    );
}

/// A line for a view's frames, timed one way (`how`).
fn said(r: &mut Report, what: &str, how: &str, f: &Frames) {
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
    r.took(format!("{what} ({how})"), median);
    if r.detail {
        let each: Vec<String> = f.ui.iter().map(|d| format!("{:.1}", ms(*d))).collect();
        println!("      frames: {}", each.join(" "));
        println!("      asked for the next by: {}", f.causes.join("; "));
    }
}

/// An object of an area in sight close up selected and dragged a few
/// meters, and the move taken back: the frames of the
/// drag (each moves the selection's copies in the area's model).
fn drag_selection(h: &mut App<'_>, r: &mut Report, what: &str, area: ResRef) {
    let tab = Tab::Area(area);
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) || r.shots {
        return;
    }
    let Some(rect) = rect_of(h.state(), &tab) else { return };
    // (Once the camera rests: the wheel's zoom of the views before goes on
    // a while, and the object would slip from under the press.)
    let orbit = |h: &App<'_>| h.state().area_views.get(&area).and_then(|v| v.orbit);
    for _ in 0..1000 {
        let was = orbit(h);
        h.run_steps(1);
        if orbit(h) == was {
            break;
        }
    }
    if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
        o.pitch = 0.7;
        o.distance = 25.0;
    }
    h.run_steps(2);
    assert_eq!(orbit(h).map(|o| o.distance), Some(25.0), "{what}: the camera rests");
    let inner = rect.shrink2(rect.size() * 0.2);
    let Some(v) = h.state_mut().area_views.get_mut(&area) else { return };
    let Some(m) = v.model.as_ref() else { return };
    // (Those picked where they stand: not a sound's or a trigger's.)
    use mg_area::ObjectKind as K;
    let seen: Vec<_> = (m.objects.iter().enumerate())
        .filter(|(_, o)| matches!(o.kind, K::Placeable | K::Creature | K::Waypoint | K::Item))
        .filter(|(_, o)| v.screen_pos(o.position).is_some_and(|p| inner.contains(p)))
        .map(|(i, o)| (i, (o.kind, o.index), o.position))
        .collect();
    if seen.is_empty() {
        println!("  {what}: no object in sight close up");
        return;
    }
    let what = format!("{what} (one of {} in sight)", seen.len());
    // The first that can be held away from its handles (its feet are its
    // turning ring's: pressed there, the drag turned it), selected alone
    // (the turning rings of others about it would turn them instead), and
    // dragged 5 m.
    let mut grip = None;
    for &(i, key, at) in &seen {
        let Some(v) = h.state_mut().area_views.get_mut(&area) else { return };
        v.selection = vec![key];
        h.run_steps(2);
        let v = &h.state().area_views[&area];
        let step = v.screen_pos(at + glam::Vec3::new(4.0, 3.0, 0.0)).zip(v.screen_pos(at));
        if let (Some(from), Some((ahead, foot))) = (v.move_grip(i), step) {
            grip = Some((from, from + (ahead - foot), at));
            break;
        }
    }
    let Some((from, to, pivot)) = grip else {
        println!("  {what}: none to hold away from its handles");
        return;
    };
    let f = frames(h, rect, Pointer::Drags(from, to), r.count, |_, _| {});
    said(r, &what, "drag", &f);
    // (The move taken back.)
    h.run_steps(2);
    let undo = |h: &App<'_>| h.state().ws.as_ref().and_then(|w| w.can_undo().map(str::to_owned));
    assert_eq!(undo(h).as_deref(), Some("Move"), "{what}: the drag moved the selection");
    h.state_mut().actions.push(Action::Undo);
    h.run_steps(2);
    // A command moving it every few frames (as a script, a plugin or the
    // arrow keys' nudges might): the frames that read the area again.
    let selected = h.state().area_views[&area].selection.first().copied();
    if let Some((kind, index)) = selected {
        let git = ResKey::new(area, ResType::GIT);
        let path = mg_edit::GffPath::root().item(kind.list(), index);
        let x = h
            .state_mut()
            .ws
            .as_mut()
            .and_then(|w| w.doc(&git).ok()?.root.list(kind.list())?.get(index)?.float("XPosition"))
            .unwrap_or(0.0);
        let f = frames(h, rect, Pointer::Moves, r.count, |app, n| {
            if n % 6 == 0 {
                let edit = mg_edit::Edit::SetField {
                    key: git,
                    path: path.clone(),
                    label: "XPosition".into(),
                    value: Some(mg_gff::Value::Float(x + 0.01 * n as f32)),
                };
                app.actions.push(Action::Apply(mg_edit::Command::new("Nudge", vec![edit])));
            }
        });
        said(r, &what, "moved by a command every 6 frames", &f);
        while undo(h).as_deref() == Some("Nudge") {
            h.state_mut().actions.push(Action::Undo);
            h.run_steps(1);
        }
    }
    // E held on it, repeating as a held key does (about 30 a second): each
    // turns the selection as the ring's drag does.
    let every = 5;
    let f = frames(h, rect, Pointer::Keys(from, egui::Key::E, every), r.count, |_, _| {});
    said(r, &what, "turned by E held", &f);
    // (Made one command once the key rests.)
    for _ in 0..2000 {
        if !h.state().area_views.get(&area).is_some_and(|v| v.key_turning()) {
            break;
        }
        h.run_steps(1);
    }
    h.run_steps(2);
    assert_eq!(undo(h).as_deref(), Some("Rotate"), "{what}: E turned the selection");
    while undo(h).as_deref() == Some("Rotate") {
        h.state_mut().actions.push(Action::Undo);
        h.run_steps(1);
    }
    // The turning ring led round, the Turn snap at 15 degrees (M4's U8:
    // the wedge, the snap's ticks, the readout chip), and with Shift the
    // east arrow led 3 m along (the guide line across the view, the
    // chip): each taken back.
    let handles = h.state().area_views.get(&area).and_then(|v| {
        let r = v.handle_radius_at(pivot)?;
        let ring = |a: f32| v.screen_pos(pivot + glam::Vec3::new(a.cos(), a.sin(), 0.0) * r);
        let head = pivot + glam::Vec3::X * r * 1.45;
        Some((
            (ring(0.3)?, ring(1.5)?),
            (v.screen_pos(head)?, v.screen_pos(head + glam::Vec3::X * 3.0)?),
        ))
    });
    if let Some(((ring_from, ring_to), (arrow_from, arrow_to))) = handles {
        let snap = h.state().settings.snap_angle;
        h.state_mut().settings.snap_angle = Some(15);
        h.run_steps(2);
        let f = frames(h, rect, Pointer::Drags(ring_from, ring_to), r.count, |_, _| {});
        said(r, &what, "its turning ring dragged", &f);
        h.state_mut().settings.snap_angle = snap;
        h.run_steps(2);
        assert_eq!(undo(h).as_deref(), Some("Rotate"), "{what}: the ring turned it");
        h.state_mut().actions.push(Action::Undo);
        h.run_steps(2);
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
        h.run_steps(1);
        let f = frames(h, rect, Pointer::Drags(arrow_from, arrow_to), r.count, |_, _| {});
        said(r, &what, "its east arrow dragged", &f);
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run_steps(2);
        assert_eq!(undo(h).as_deref(), Some("Move"), "{what}: the arrow moved it");
        h.state_mut().actions.push(Action::Undo);
        h.run_steps(2);
    }
    // (The selection let go.)
    if let Some(v) = h.state_mut().area_views.get_mut(&area) {
        v.selection.clear();
    }
}

fn open(h: &mut App<'_>, tab: Tab) {
    h.state_mut().actions.push(Action::OpenTab(tab));
    h.run_steps(3);
}

/// Clicks the first thing labelled `label`. There must be one: a control
/// renamed in the interface fails the test here, rather than a view going
/// untimed (or timed in the wrong state) without a word.
fn click(h: &mut App<'_>, label: &str) {
    let found = h.query_all_by_label(label).next().map(|node| node.click()).is_some();
    assert!(found, "no {label:?} to click: was the control renamed?");
    h.run_steps(2);
}

/// The row `row` of the Top Bar's menu `menu`, clicked (M4's U28: Find
/// Instance and Build Module as a builder opens them).
fn menu(h: &mut App<'_>, menu: &str, row: &str) {
    h.get_by_label("Top Bar").get_by_label(menu).click();
    h.run_steps(2);
    let keyed = format!("{row} ");
    let at = h
        .query_all_by(|n| n.label().is_some_and(|l| l == row || l.starts_with(&keyed)))
        .next()
        .unwrap_or_else(|| panic!("no {row:?} in {menu}: was the command renamed?"))
        .rect();
    let (pos, button, modifiers) = (at.center(), egui::PointerButton::Primary, Default::default());
    h.hover_at(pos);
    h.event(egui::Event::PointerButton { pos, button, pressed: true, modifiers });
    h.event(egui::Event::PointerButton { pos, button, pressed: false, modifiers });
    h.run_steps(3);
}

/// [`click`] on the first thing labelled `label` that is not in the group
/// `group` (the Module panel's Properties, in the "Left Dock", beside a
/// blueprint's Properties page).
fn click_outside(h: &mut App<'_>, label: &str, group: &str) {
    let inside = |n: &egui_kittest::Node<'_>| {
        let mut up = n.accesskit_node().parent();
        while let Some(p) = up {
            if p.label().as_deref() == Some(group) {
                return true;
            }
            up = p.parent();
        }
        false
    };
    let found = h.query_all_by_label(label).find(|n| !inside(n)).map(|n| n.click()).is_some();
    assert!(found, "no {label:?} outside {group:?} to click: was the control renamed?");
    h.run_steps(2);
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

/// Shows the Left Dock's Resources panel (the resource browser), times it
/// with the pointer and the wheel over it, and shows the Module panel
/// again. Loudly: the panel must be shown and its list there.
fn resources_panel(h: &mut App<'_>, r: &mut Report, what: &str) {
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Resources));
    h.run_steps(3);
    assert_eq!(h.state().left_dock_shows(), Some(mg_ui::Panel::Resources), "{what}: shown");
    // (The panel: under the dock's tab bar, from its first tab to its
    // edge, down to the Status Bar.)
    let tab = h.get_by_label("Left Dock").get_by_label("Module").rect();
    let edge = h.state().left_dock_rect().expect("the Left Dock").right();
    h.get_by_label("Filter Resources");
    let bottom = screen_size().y - mg_uikit::metrics::STATUS_BAR;
    let panel =
        egui::Rect::from_min_max(egui::pos2(tab.left(), tab.bottom()), egui::pos2(edge, bottom));
    view_in(h, r, what, Some(panel), |_, _| {});
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Module));
    h.run_steps(2);
}

/// Opens `tab`, times it and closes it.
/// Shows the palettes: the Right Dock unfolded on its Palette panel, as
/// Tools › Palettes shows it. Loudly: they must be shown.
fn show_palette(h: &mut App<'_>) {
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Palette));
    h.run_steps(3);
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Palette), "palettes not shown");
    assert!(palette_rect(h).is_some(), "the Right Dock is not drawn");
}

/// Shows the Preview: the Right Dock unfolded on its Preview panel, as
/// Tools › Preview shows it. Loudly: it must be drawn.
fn show_preview(h: &mut App<'_>) {
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Preview));
    // (Until its body is drawn: an area's view keeps the window asking
    // for frames, so not until it stops.)
    let started = std::time::Instant::now();
    while h.state().preview_panel_rect().is_none() {
        h.step();
        assert!(started.elapsed().as_secs() < 30, "the Preview panel is not drawn");
    }
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Preview), "Preview not shown");
}

/// Folds the palettes away: the Right Dock folded, as View › Right Dock
/// folds it.
fn fold_palette(h: &mut App<'_>) {
    h.state_mut().actions.push(Action::FoldPalettes);
    h.run_steps(2);
}

/// Where the palettes are drawn, while they are shown: the Right Dock's
/// panel, under its tab bar (loudly: its fold button and tab must be
/// there).
fn palette_rect(h: &App<'_>) -> Option<egui::Rect> {
    let dock = h.state().right_dock_rect()?;
    h.query_by_label("Fold the Right Dock")?;
    // (Under its tab bar: the Palette tab's.)
    let tab =
        (h.query_all_by_label("Palette")).map(|n| n.rect()).find(|r| dock.contains(r.center()))?;
    // (Its own section, M4's U6: in the wide layout's two columns the
    // first, Properties beside it; stacked, over Properties, whose header
    // ends with Open in Tab; else the dock.)
    let open = h.query_by_label(mg_ui::properties_panel::OPEN_IN_TAB).map(|n| n.rect());
    let dock = match open {
        Some(open) if (open.center().y - tab.center().y).abs() < 4.0 => {
            let first = mg_uikit::shell::Yields::right_first_column(dock.width());
            dock.with_max_x(dock.left() + first)
        }
        Some(open) => dock.with_max_y(open.top() - 2.0),
        None => dock,
    };
    // (120 points from its left: the wheel turns at its middle, 60 points
    // in, on the rows' words at every width of the dock, the worst case:
    // M3 T14's review found the pointer past their end in a 320 dock and
    // on them in a 280 one.)
    let panel = dock.with_min_y(tab.bottom());
    Some(panel.with_max_x(panel.left() + 120.0))
}

/// Times the palettes with the pointer and the wheel over them: shown,
/// timed, folded away again.
fn palette_view(h: &mut App<'_>, r: &mut Report, what: &str) {
    show_palette(h);
    let rect = palette_rect(h);
    view_in(h, r, what, rect, |_, _| {});
    fold_palette(h);
}

/// A blueprint's pages on their one scroll (M4's U9), scrolled from the
/// first to the last by the wheel over the pages: each frame draws the
/// pages in sight, and those come into sight measured as they come.
fn pages_scrolled(h: &mut App<'_>, r: &mut Report, what: &str, key: ResKey) {
    let what = format!("{what}, its pages scrolled top to bottom");
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) || r.shots {
        return;
    }
    let tab = Tab::Blueprint(key);
    open(h, tab.clone());
    let rect = rect_of(h.state(), &tab).expect("the blueprint's window");
    let pages = mg_ui::blueprint::pages(key.restype);
    let at = |h: &App<'_>| h.state().blueprint_pages.get(&(key, mg_edit::GffPath::root())).copied();
    // (Over the pages, in their left margin beside the list, the
    // window's own margin before it: a list on a page takes the wheel
    // over it.)
    let over = egui::pos2(rect.left() + mg_uikit::pages::PAGE_LIST + 12.0, rect.center().y);
    // (Enough frames to reach the last page, at 120 points a frame.)
    let mut f = frames(h, rect, Pointer::Down(over, 120.0), 4, |_, _| {});
    let mut more = 0;
    while at(h) != pages.last().copied() && more < 40 {
        let g = frames(h, rect, Pointer::Down(over, 120.0), r.count, |_, _| {});
        f.ui.extend(g.ui);
        f.tessellate.extend(g.tessellate);
        f.gpu.extend(g.gpu);
        f.vertices = f.vertices.max(g.vertices);
        f.passes = f.passes.max(g.passes);
        more += 1;
    }
    assert_eq!(at(h), pages.last().copied(), "{what}: the wheel reached the last page");
    said(r, &what, "wheel", &f);
    close(h, &tab);
}

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
    // (`FRAME_AREAS=zhentkeepwar,…`: those areas instead, by resref; the
    // campaign's with the most creatures is zhentkeepwar, 67.)
    if let Ok(named) = std::env::var("FRAME_AREAS") {
        let named: Vec<&str> = named.split(',').map(str::trim).collect();
        return sized
            .into_iter()
            .filter(|(r, _)| r.as_str().is_some_and(|s| named.contains(&s)))
            .collect();
    }
    let (largest, middle) = (*sized.last().unwrap(), sized[sized.len() / 2]);
    vec![middle, largest]
}

fn harness<'a>(mut app: Moonglow, rs: egui_wgpu::RenderState) -> App<'a> {
    // (`FRAME_STALE_KEPT=<MiB>`: so much of the textures not drawn lately
    // kept, as `Renderer::set_stale_textures_kept`.)
    if let Some(mib) = std::env::var("FRAME_STALE_KEPT").ok().and_then(|m| m.parse::<u64>().ok())
        && let Some(vp) = app.viewport.as_mut()
    {
        vp.renderer.set_stale_textures_kept(mib << 20);
    }
    // (`FRAME_SHADOW_QUALITY=high`: the area views' shadows as stencil
    // volumes, Options › Area's High.)
    // (`FRAME_SAMPLES=1` or `2`: Options › Area › Anti-aliasing.)
    app.settings.anti_aliasing = std::env::var("FRAME_SAMPLES").ok().and_then(|n| n.parse().ok());
    if std::env::var("FRAME_SHADOW_QUALITY").is_ok_and(|q| q.eq_ignore_ascii_case("high")) {
        app.settings.shadow_quality = mg_ui::settings::ShadowQuality::High;
    }
    // (`FRAME_AREA_RESOLUTION=<percent>`: Options › Area › Area view
    // resolution.)
    if let Some(p) = std::env::var("FRAME_AREA_RESOLUTION").ok().and_then(|p| p.parse().ok()) {
        app.settings.area_resolution = Some(p);
    }
    // (`FRAME_TEXTURE_LIMIT=<pixels>`: Options › Area › Largest textures.)
    if let Some(px) = std::env::var("FRAME_TEXTURE_LIMIT").ok().and_then(|p| p.parse().ok()) {
        app.settings.texture_limit = Some(px);
        if let Some(vp) = app.viewport.as_ref() {
            vp.gpu.set_texture_limit(Some(u32::from(px)));
        }
    }
    // (A step a 144th of a second, as the window's frames go: with the
    // harness's quarter of a second, every frame of an area's opening
    // was an animation step's, the picture drawn again in each.)
    let mut h = Harness::builder()
        .with_size(screen_size())
        .with_step_dt((1.0 / FPS) as f32)
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    // (In a window under 1500, `FRAME_SCREEN`, the Left Dock unfolded by
    // hand, as a builder who works in the module tree there has it: the
    // views click its buttons.)
    if screen_size().x < mg_uikit::shell::LEFT_DOCK_FROM {
        h.run_steps(2);
        h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Module));
        h.run_steps(2);
    }
    h
}

/// A device as the desktop app asks for one (`apps/moonglow`): with BC
/// textures, which the game's are mostly (without, each was uploaded as
/// RGBA8, four to eight times its size: The Domains of Dread's woods took
/// 4.4 GB of textures here, where the app takes a quarter of it).
fn render_state() -> egui_wgpu::RenderState {
    let mut setup = egui_kittest::wgpu::default_wgpu_setup();
    if let egui_wgpu::WgpuSetup::CreateNew(new) = &mut setup {
        new.device_descriptor = std::sync::Arc::new(|adapter| wgpu::DeviceDescriptor {
            label: Some("moonglow"),
            required_features: adapter.features() & mg_ui::OPTIONAL_FEATURES,
            required_limits: adapter.limits(),
            // (`FRAME_MEMORY_HINTS=performance`: wgpu's default, blocks of
            // 128 to 256 MB.)
            memory_hints: match std::env::var("FRAME_MEMORY_HINTS").as_deref() {
                Ok("performance") => wgpu::MemoryHints::Performance,
                _ => wgpu::MemoryHints::MemoryUsage,
            },
            ..Default::default()
        });
    }
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(setup),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    RENDER_STATE.with(|r| *r.borrow_mut() = Some(rs.clone()));
    rs
}

thread_local! {
    /// The test's render state ([`render_state`]): the timed frames, run
    /// outside the harness, hand it their textures' changes.
    static RENDER_STATE: std::cell::RefCell<Option<egui_wgpu::RenderState>> =
        const { std::cell::RefCell::new(None) };
}

/// A timed frame's textures' changes, to the renderer the harness draws
/// with (not timed): dropped, a font atlas grown in one of them was never
/// uploaded, and the next frame's glyphs were written past its end.
fn upload(mut delta: egui::TexturesDelta) {
    RENDER_STATE.with(|r| {
        if let Some(rs) = r.borrow().as_ref() {
            let mut renderer = rs.renderer.write();
            for (id, images) in delta.set.drain() {
                for image in images {
                    renderer.update_texture(&rs.device, &rs.queue, id, &image);
                }
            }
            for id in delta.free.drain() {
                renderer.free_texture(&id);
            }
        }
    });
    delta.clear();
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
    // area a module opens itself was drawn as it opened. With
    // `FRAME_KEEP_TEXTURES`, those of what was shown before kept, as a
    // session keeps them.)
    // (`FRAME_UNCOMPRESSED=1`: the TGAs and PLTs uploaded as RGBA8, as
    // before 2026-11-25, not compressed as they are read.)
    if let Some(vp) = h.state_mut().viewport.as_mut() {
        vp.gpu.set_compress_read_textures(std::env::var_os("FRAME_UNCOMPRESSED").is_none());
    }
    if let Some(vp) = h.state_mut().viewport.as_mut()
        && std::env::var_os("FRAME_KEEP_TEXTURES").is_none()
    {
        vp.renderer.clear_textures();
    }
    h.state_mut().actions.push(Action::OpenTab(tab.clone()));
    let mut opening = Vec::new();
    let begun = Instant::now();
    let (mut shown, mut whole) = (None, None);
    // (The frames drawn while casters waited for their stencil volumes,
    // made apart with High: from the first such frame to the last.)
    let (mut volumes_from, mut volumes_until, mut volume_frames) = (None, None, 0);
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
        let drawn = app.viewport.as_ref().map(|v| v.renderer.drawn).unwrap_or_default();
        let pending = drawn.textures_pending + drawn.volumes_pending;
        // (`FRAME_DETAIL`: each frame of the opening, what the view had and
        // waited for by its end.)
        if r.detail {
            println!(
                "      opening frame at {:.1} ms, {:.1} ms: shown {}, {} pictures, \
                 {} textures waited for ({} read), {} volumes, {} programs",
                ms(begun.elapsed()),
                ms(took),
                view.is_some_and(|v| v.model.is_some()),
                view.map_or(0, |v| v.pictures),
                drawn.textures_pending,
                app.viewport.as_ref().map_or(0, |v| v.renderer.textures_read()),
                drawn.volumes_pending,
                drawn.game_pending,
            );
        }
        if drawn.volumes_pending > 0 {
            volumes_from.get_or_insert(begun.elapsed());
            volumes_until = Some(begun.elapsed());
            volume_frames += 1;
        }
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
        if let (Some(from), Some(until)) = (volumes_from, volumes_until) {
            println!(
                "      casters waited for their stencil volumes in {volume_frames} frames, \
                 {:.0} ms from the first to the last",
                ms(until - from)
            );
        }
        if r.detail {
            let each: Vec<String> = opening.iter().map(|d| format!("{:.1}", ms(*d))).collect();
            println!("      frames: {}", each.join(" "));
        }
    }
    // An edit of it (its Comments: nothing drawn changes), and its Undo:
    // the frame each is seen in (the one after it is made), the area read
    // again and drawn there (against what it costs drawn alone: the
    // overview's frames below).
    if !r.shots {
        let are = ResKey::new(area, ResType::ARE);
        let comment = mg_gff::Value::String(b"timed".to_vec());
        let edit = mg_edit::Edit::SetField {
            key: are,
            path: mg_edit::GffPath::root(),
            label: "Comments".into(),
            value: Some(comment),
        };
        let mut took = Vec::new();
        for action in [Action::Apply(mg_edit::Command::new("Timed", vec![edit])), Action::Undo] {
            h.state_mut().actions.push(action);
            h.run_steps(1);
            let t = Instant::now();
            h.run_steps(1);
            took.push(ms(t.elapsed()));
        }
        println!(
            "  area {area}{label}: an edit's frame {:.1} ms, its Undo's {:.1} ms",
            took[0], took[1]
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
    // Paint on with two placeables chosen and the palette shown (M4's
    // U4: its settings in the header's left group and the Brush popover,
    // the palette's Brush section; before it, the strip over the view):
    // the brush's circle and its next dab follow the pointer.
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        app.palette.kind = BlueprintKind::Placeable;
        app.palette.selected = ResKey::parse("plc_chest1", ResType::UTP);
        app.palette.chosen = ResKey::parse("plc_tree", ResType::UTP).into_iter().collect();
        app.paint = true;
    }
    show_palette(h);
    view(h, r, &format!("{name}, Paint on, two placeables chosen"), Some(&tab));
    {
        let app = h.state_mut();
        app.paint = false;
        app.palette.selected = None;
        app.palette.chosen.clear();
    }
    fold_palette(h);
    // Select Tiles on with a box of tiles selected (six by six at most)
    // and the tiles palette shown (M4's U5: Select Tiles' commands in the
    // header's left group and Tile Properties the Right Dock's lower
    // section, reading the tiles once and drawing their lights, height
    // and textures; before it, the Select tool's group and no panel).
    show_palette(h);
    {
        let app = h.state_mut();
        app.palette.tiles = true;
        if let Some(v) = app.area_views.get_mut(&area) {
            let (w, d) = v.model.as_ref().map_or((0, 0), |m| (m.width, m.height));
            v.selection.clear();
            v.tile_mode = true;
            v.tile_selection =
                (0..d.min(6)).flat_map(|y| (0..w.min(6)).map(move |x| (x, y))).collect();
        }
    }
    view(h, r, &format!("{name}, Select Tiles on, a box of tiles selected"), Some(&tab));
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        if let Some(v) = app.area_views.get_mut(&area) {
            v.tile_mode = false;
            v.tile_selection.clear();
        }
    }
    fold_palette(h);
    // (The camera as the views above leave it, given back after the two
    // Properties panel views: their wheel frames zoom it out, and the
    // views after them would otherwise be timed from farther away, which
    // made their wheel rows 18 percent dearer than before M4's U6 with no
    // change of their own; U30's review.)
    let rest = resting_camera(h, area);
    // A creature selected, its Inventory page in the Properties panel
    // under the palettes (M4's U6: the Properties window's page drawn in
    // the Right Dock's lower section every frame; before it, the creature
    // selected and no panel).
    let creature = (h.state().area_views.get(&area).and_then(|v| v.model.as_ref()))
        .and_then(|m| m.objects.iter().find(|o| o.kind == mg_area::ObjectKind::Creature))
        .map(|o| (o.kind, o.index));
    if let Some(creature) = creature.filter(|_| std::env::var_os("FRAME_NO_PANEL_VIEWS").is_none())
    {
        show_palette(h);
        if let Some(v) = h.state_mut().area_views.get_mut(&area) {
            v.selection = vec![creature];
        }
        h.run_steps(3);
        if let Some(page) = h.query_by_label("Pages").and_then(|p| p.query_by_label("Inventory")) {
            page.click();
        }
        h.run_steps(3);
        view(
            h,
            r,
            &format!("{name}, a creature's Inventory page in the Properties panel"),
            Some(&tab),
        );
        if let Some(v) = h.state_mut().area_views.get_mut(&area) {
            v.selection.clear();
        }
        fold_palette(h);
    }
    // Every placeable selected (edited together in the Properties panel,
    // its Basic page, every frame; before M4's U6, selected and no panel).
    let placeables: Vec<_> = (h.state().area_views.get(&area).and_then(|v| v.model.as_ref()))
        .map(|m| m.objects.iter().filter(|o| o.kind == mg_area::ObjectKind::Placeable))
        .map(|all| all.map(|o| (o.kind, o.index)).collect())
        .unwrap_or_default();
    if placeables.len() > 1 && std::env::var_os("FRAME_NO_PANEL_VIEWS").is_none() {
        show_palette(h);
        if let Some(v) = h.state_mut().area_views.get_mut(&area) {
            v.selection = placeables.clone();
        }
        h.run_steps(3);
        if let Some(page) = h.query_by_label("Pages").and_then(|p| p.query_by_label("Basic")) {
            page.click();
        }
        h.run_steps(3);
        let what =
            format!("{name}, {} placeables selected, the Properties panel", placeables.len());
        view(h, r, &what, Some(&tab));
        if let Some(v) = h.state_mut().area_views.get_mut(&area) {
            v.selection.clear();
        }
        fold_palette(h);
    }
    rest_camera(h, area, rest);
    // The Outliner (M3's T13) with a kind of 200 objects or more opened
    // (one of them selected: the Outliner opens its kind and brings it
    // into sight), against its kinds closed, the same one selected.
    let largest = h.state().area_views.get(&area).and_then(|v| v.model.as_ref()).and_then(|m| {
        let count = |k: mg_area::ObjectKind| m.objects.iter().filter(|o| o.kind == k).count();
        let kind = mg_area::ObjectKind::ALL.into_iter().max_by_key(|k| count(*k))?;
        let first = m.objects.iter().find(|o| o.kind == kind)?;
        Some((kind, first.index, count(kind)))
    });
    if let Some((kind, index, n)) = largest.filter(|l| l.2 >= 200) {
        h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(kind, index)];
        h.run_steps(30);
        let open = format!("{name}, the Outliner's {} open ({n})", kind.label());
        view(h, r, &open, Some(&tab));
        h.state_mut().outliner_fold_all(false);
        h.run_steps(30);
        view(h, r, &format!("{name}, the Outliner's kinds closed, one selected"), Some(&tab));
        h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    }
    // The tiles palette beside it with a search being typed: a letter
    // more each frame (the tileset searched again as each is).
    show_palette(h);
    view_with(h, r, &format!("{name}, tiles palette searched as typed"), None, |app, n| {
        const TYPED: &str = "corner wall";
        let p = &mut app.palette;
        (p.tiles, p.prefabs) = (true, false);
        p.filter = TYPED[..1 + n % TYPED.len()].to_string();
    });
    {
        let p = &mut h.state_mut().palette;
        (p.tiles, p.filter) = (false, String::new());
    }
    // The Right Dock in two columns (M3's T14: the wide layout gives it
    // 741 at 3440; since M4's U6 the Palette and the Preview as tabs of the
    // first, Properties the second), a blueprint chosen and the Preview's
    // tab in front so that it draws its model beside the area each frame,
    // against one column of 320 with the Palette in front (over
    // Properties): the largest area alone, in a window of the wide layout.
    if objects >= 500 && screen_size().x >= mg_uikit::shell::WIDE_FROM {
        let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        h.state_mut().palette.selected = Some(chest);
        h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Preview));
        let started = Instant::now();
        while h.state().preview_panel_rect().is_none() {
            assert!(started.elapsed().as_secs() < 30, "the Preview's column is not drawn");
            h.step();
        }
        h.run_steps(30);
        let both = format!("{name}, the Right Dock in two columns (741), the Preview's model");
        view(h, r, &both, Some(&tab));
        // The wide layout's own width (`right_dock_width_wide`).
        h.state_mut().settings.right_dock_width_wide = Some(320);
        h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Palette));
        let started = Instant::now();
        while h.state().preview_panel_rect().is_some() {
            assert!(started.elapsed().as_secs() < 30, "the Right Dock still in two columns");
            h.step();
        }
        let one = format!("{name}, the Right Dock in one column (320), the Palette in front");
        view(h, r, &one, Some(&tab));
        h.state_mut().settings.right_dock_width_wide = None;
        h.state_mut().palette.selected = None;
    }
    fold_palette(h);
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
    // 32 objects selected (as many as get handles of their own) and Shift
    // held, seen close: each one's arrows, tilt rings and scale handle (M4's
    // U7: each line over its edge, the rings in 96 pieces), lit under the
    // pointer as it goes by. The camera on the object with the most others
    // within 12 m, the 32 nearest it selected: their handles in sight (the
    // view's own middle had few in some areas; U7's review).
    if let Some(v) = h.state_mut().area_views.get_mut(&area)
        && let (Some(m), Some(orbit)) = (v.model.as_ref(), v.orbit.as_mut())
    {
        let handled: Vec<_> = (m.objects.iter())
            .filter(|o| o.kind != mg_area::ObjectKind::Sound && !o.locked)
            .collect();
        let crowd = |o: &mg_area::AreaObject| {
            handled.iter().filter(|p| (p.position - o.position).length() < 12.0).count()
        };
        if let Some(middle) = handled.iter().max_by_key(|o| crowd(o)).map(|o| o.position) {
            orbit.target = middle;
        }
        orbit.pitch = 0.7;
        orbit.distance = 25.0;
        let target = orbit.target;
        let mut near: Vec<_> =
            (handled.iter()).map(|o| ((o.position - target).length(), (o.kind, o.index))).collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        v.selection = near.into_iter().take(32).map(|(_, key)| key).collect();
    }
    // (Held from here: the window keeps the modifiers its events gave.)
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    h.run_steps(1);
    view(h, r, &format!("{name}, 32 selected, Shift held"), Some(&tab));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(1);
    if let Some(v) = h.state_mut().area_views.get_mut(&area) {
        v.selection.clear();
    }
    drag_selection(h, r, &format!("{name}, a selection dragged"), area);
    close(h, &tab);
    if r.only.as_ref().is_none_or(|only| name.contains(only.as_str())) {
        said_memory(h, &format!("{name}, closed"));
    }
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
    let mut model = mg_area::AreaModel::read(&game, &are.root, &git.root, tileset.as_ref());
    let t = Instant::now();
    let mut scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let mut view = mg_area::View::of(&model);
    // (`FRAME_SHADOWS`: the kinds drawn, of `sun`, `creatures`, `lights`
    // (by commas; `off` none), each area made to have the sun's and moon's
    // shadows at its opacity or 50.)
    if let Ok(kinds) = std::env::var("FRAME_SHADOWS") {
        view.shadows = kinds.contains("sun");
        view.creature_shadows = kinds.contains("creatures");
        view.light_shadows = kinds.contains("lights");
        // (The lights' shadows only where the area has no sun's.)
        let l = &mut model.lighting;
        let sky = !view.light_shadows;
        (l.sun.shadows, l.moon.shadows) = (sky, sky);
        if l.shadow_opacity == 0 {
            l.shadow_opacity = 50;
        }
    }
    println!(
        "  area {area}: models loaded in {:.0?}; shadows {:?}, creatures {}, lights {}",
        t.elapsed(),
        view.shadows_of(&model.lighting),
        view.creature_shadows,
        view.light_shadows
    );
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
        renderer.shadow_volumes =
            std::env::var("FRAME_SHADOW_QUALITY").is_ok_and(|q| q.eq_ignore_ascii_case("high"));
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
             meshes in {} draws of {} materials, {} pipelines and {} chunks in turn, {} of the game's, {} \
             meshes out of sight; {} casting the sun's shadows, {} creatures the lights')",
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
            did.chunks,
            did.game,
            did.hidden,
            did.sun_casters,
            did.light_shadows,
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

/// What reading an area again after a command costs, in the campaign's
/// most crowded areas: read whole (its model from its ARE and GIT, its
/// ground and its scene, as `area_view::refresh` did after every edit),
/// and with one object moved, that object alone read again with its
/// ground and scene (as it does now, the workspace telling it what
/// changed: moved, its look kept; its look changed, read again), and the
/// GIT compared with what was read (its fallback, where the workspace
/// can't tell). Medians of a few runs, the models loaded already.
/// `REFRESH_KINDS=1` times reading the first object of each kind alone. By hand: `cargo test --release -p mg-ui --test frame_perf --
/// --ignored --nocapture refresh_parts`.
#[test]
#[ignore]
fn refresh_parts() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let path = root.join("data/nwm").join(CAMPAIGN);
    if !path.is_file() {
        println!("skipped: {CAMPAIGN} isn't installed");
        return;
    }
    let rs = render_state();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs);
    app.open_module(&path);
    let game = app.game.clone().unwrap();
    let gpu = app.viewport.as_ref().unwrap().gpu.clone();
    let ws = app.ws.as_mut().unwrap();
    let areas: Vec<ResRef> = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
    let count = |g: &mg_gff::Gff| -> usize {
        mg_area::ObjectKind::ALL.iter().map(|k| g.root.list(k.list()).map_or(0, <[_]>::len)).sum()
    };
    let mut read: Vec<(ResRef, mg_gff::Gff, mg_gff::Gff)> = areas
        .into_iter()
        .filter_map(|a| {
            let are = ws.doc(&ResKey::new(a, ResType::ARE)).ok()?.clone();
            let git = ws.doc(&ResKey::new(a, ResType::GIT)).ok()?.clone();
            Some((a, are, git))
        })
        .collect();
    read.sort_by_key(|r| std::cmp::Reverse(count(&r.2)));
    let median = |mut v: Vec<Duration>| {
        v.sort();
        v[v.len() / 2]
    };
    for (area, are, git) in read.into_iter().take(3) {
        let tileset_ref = are.root.resref("Tileset").unwrap_or(ResRef::EMPTY);
        let set = mg_area::tileset(&game, tileset_ref).ok();
        let model = mg_area::AreaModel::read(&game, &are.root, &git.root, set.as_ref());
        let mut ground = mg_area::walk::Ground::new(&game, &model);
        let mut scene = mg_area::AreaScene::new(&gpu, &game, &model);
        let (mut m, mut g, mut sc) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..5 {
            let t = Instant::now();
            let model = mg_area::AreaModel::read(&game, &are.root, &git.root, set.as_ref());
            m.push(t.elapsed());
            let t = Instant::now();
            ground.update(&game, &model);
            g.push(t.elapsed());
            let t = Instant::now();
            scene.update(&gpu, &game, &model);
            sc.push(t.elapsed());
        }
        // One object moved.
        let mut moved = git.clone();
        let kind = mg_area::ObjectKind::ALL
            .into_iter()
            .find(|k| moved.root.list(k.list()).is_some_and(|l| !l.is_empty()))
            .unwrap();
        let o = &mut moved.root.list_mut(kind.list()).unwrap()[0];
        let x = o.float("XPosition").unwrap_or(0.0);
        o.set("XPosition", mg_gff::Value::Float(x + 1.0));
        let changed = vec![(kind, 0)];
        let (mut one, mut placed, mut compare) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..5 {
            // Moved (its look kept), and its look changed (read again).
            let mut model = model.clone();
            let t = Instant::now();
            model.place_objects(&moved.root, &changed);
            ground.update(&game, &model);
            placed.push(t.elapsed());
            let t = Instant::now();
            let at = model.read_objects(&game, &moved.root, &changed);
            ground.update(&game, &model);
            scene.update_objects(&gpu, &game, &model, &at);
            one.push(t.elapsed());
            let t = Instant::now();
            let found = mg_area::AreaModel::changed_objects(&git.root, &moved.root).unwrap();
            compare.push(t.elapsed());
            assert_eq!(found, changed);
        }
        // (Each kind's first object read alone: what one costs.)
        if std::env::var_os("REFRESH_KINDS").is_some() {
            for k in mg_area::ObjectKind::ALL {
                if moved.root.list(k.list()).is_some_and(|l| !l.is_empty()) {
                    let mut model = model.clone();
                    let t = Instant::now();
                    model.read_objects(&game, &moved.root, &[(k, 0)]);
                    println!("      {k:?} 0: {:.2} ms", ms(t.elapsed()));
                }
            }
        }
        println!(
            "  area {area} ({} objects, {} tiles): read whole: model {:6.2} ms, ground {:5.2} ms, \
             scene {:5.2} ms; its first {kind:?} moved {:5.2} ms, its look changed {:5.2} ms \
             (the GIT compared, where the workspace can't tell: {:5.2} ms)",
            count(&git),
            model.tiles.len(),
            ms(median(m)),
            ms(median(g)),
            ms(median(sc)),
            ms(median(placed)),
            ms(median(one)),
            ms(median(compare)),
        );
    }
}

/// A campaign's views: the module with nothing else open, each editor on
/// its largest resource, the palettes and lists, a menu, and areas.
/// A placed placeable's tint set again and again from its Properties
/// window's Variables page, as dragging its color swatch does (the
/// owner's report: laggy, as if the whole module were walked at each
/// change): each frame after the command, timed, in the campaign's area
/// with the most objects. `TINT_PAGE=Visuals` with the Visuals page shown.
#[test]
#[ignore]
fn tint_edits() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_gff::{Struct, Value};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    // (`TINT_MODULE`, `TINT_USER`: a builder's module, opened from a copy,
    // and the user folder with its haks.)
    let path = match std::env::var_os("TINT_MODULE") {
        // (A project's folder is taken as it is: a copy made for it.)
        Some(m) if std::path::Path::new(&m).is_dir() => m.into(),
        Some(m) => {
            let copy = mg_testkit::scratch_dir("frame-tint").join("module.mod");
            std::fs::copy(&m, &copy).unwrap();
            copy
        }
        None => root.join("data/nwm").join(CAMPAIGN),
    };
    if !path.exists() {
        println!("skipped: {} isn't there", path.display());
        return;
    }
    let user = std::env::var_os("TINT_USER").map(std::path::PathBuf::from);
    let rs = render_state();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, user, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    let ws = app.ws.as_mut().unwrap();
    let keys: Vec<ResKey> = ws.module.keys_of(ResType::GIT).copied().collect();
    let placeables = |ws: &mut mg_edit::Workspace, k: &ResKey| {
        let count = |g: &mg_gff::Struct| -> usize {
            mg_area::ObjectKind::ALL.iter().map(|o| g.list(o.list()).map_or(0, <[_]>::len)).sum()
        };
        ws.doc(k).ok().map(|d| count(&d.root)).unwrap_or(0)
    };
    // (A placeable whose model takes a tint, where one is placed: the
    // area with the most placeables first.)
    let mut keys = keys;
    keys.sort_by_key(|k| std::cmp::Reverse(placeables(ws, k)));
    let game = app.game.clone().unwrap();
    let tintable = keys.iter().find_map(|k| {
        let doc = ws.doc(k).ok()?;
        let list = doc.root.list("Placeable List")?;
        let i = list.iter().position(|p| !mg_preview::tintable_materials(&game, p).is_empty())?;
        Some((*k, i))
    });
    println!("  a tintable placeable: {tintable:?}");
    // (None placed: a copy of the busiest area's first placeable, of the
    // first appearance whose model takes one, put at the end of its list.)
    let (git, index, setup) = match tintable {
        Some((git, index)) => (git, index, None),
        None => {
            let git = keys[0];
            let doc = ws.doc(&git).unwrap();
            let list = doc.root.list("Placeable List").unwrap();
            let mut copy = list[0].clone();
            let rows = game.table("placeables").map(|t| t.len()).unwrap_or(0);
            let row = (0..rows).find(|&r| {
                copy.set("Appearance", Value::Dword(r as u32));
                !mg_preview::tintable_materials(&game, &copy).is_empty()
            });
            println!("  placed: appearance {row:?}");
            copy.set("Appearance", Value::Dword(row.unwrap_or(0) as u32));
            copy.set("Static", Value::Byte(0));
            let index = list.len();
            let insert = Edit::InsertItem {
                key: git,
                path: GffPath::root(),
                list: "Placeable List".into(),
                index,
                item: copy,
            };
            (git, index, Some(Command::new("Place", vec![insert])))
        }
    };
    if let Some(cmd) = setup {
        app.actions.push(Action::Apply(cmd));
    }
    let area = git.resref;
    let path = GffPath::root().item("Placeable List", index);
    let page = std::env::var("TINT_PAGE").unwrap_or_else(|_| "Variables".into());
    let page: &'static str = Box::leak(page.into_boxed_str());
    app.blueprint_pages.insert((git, path.clone()), page);
    let mut h = harness(app, rs);
    open(&mut h, Tab::Area(area));
    h.run_steps(20);
    open(&mut h, Tab::Instance { area, path: path.clone() });
    h.run_steps(10);
    // (Its other variables kept, as the page keeps them.)
    let others: Vec<Struct> = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let doc = ws.doc(&git).unwrap();
        let list = path.get(&doc.root).and_then(|s| s.list("VarTable")).unwrap_or(&[]);
        let tint = |v: &&Struct| v.string("Name").is_some_and(|n| n.starts_with(b"tdod_tint"));
        list.iter().filter(|v| !tint(v)).cloned().collect()
    };
    let tinted = |i: usize| {
        let mut v = Struct::new(0);
        v.set("Name", Value::String(b"tdod_tint".to_vec()));
        v.set("Type", Value::Dword(3));
        v.set("Value", Value::String(format!("#{:02x}4d8f", (i * 8) % 256).into_bytes()));
        let edit = Edit::SetField {
            key: git,
            path: path.clone(),
            label: "VarTable".into(),
            value: Some(Value::List(others.iter().cloned().chain([v]).collect())),
        };
        Action::Apply(Command::new("Edit variables", vec![edit]))
    };
    let rect = rect_of(h.state(), &Tab::Area(area)).unwrap_or(egui::Rect::EVERYTHING);
    // (`TINT_SHOT`: pictures of it, here.)
    let shots = || mg_testkit::scratch_dir("frame-tint-shot");
    println!(
        "  the area's rect {rect:?}, the window's {:?}",
        rect_of(h.state(), &Tab::Instance { area, path: path.clone() })
    );
    if std::env::var_os("TINT_SHOT").is_some() {
        let dir = shots();
        h.render().unwrap().save(dir.join("before.png")).unwrap();
    }
    let quiet = frames(&mut h, rect, Pointer::Away, 40, |_, _| {});
    // The page's own swatch: a tint row added, its picker opened and
    // dragged across, a frame a step (`TINT_DRAG`).
    if std::env::var_os("TINT_DRAG").is_some() {
        h.state_mut().actions.push(tinted(0));
        h.run_steps(4);
        let well = h
            .get_all_by(|n| n.role() == egui::accesskit::Role::ColorWell)
            .next()
            .expect("the tint's swatch")
            .rect();
        h.get_all_by(|n| n.role() == egui::accesskit::Role::ColorWell).next().unwrap().click();
        h.run_steps(3);
        if std::env::var_os("TINT_SHOT").is_some() {
            let dir = shots();
            h.render().unwrap().save(dir.join("picker.png")).unwrap();
        }
        // (The picker's square under the swatch: its middle row.)
        let at = |x: f32| egui::pos2(well.left() + 20.0 + x, well.bottom() + 120.0);
        h.event(egui::Event::PointerMoved(at(0.0)));
        h.event(egui::Event::PointerButton {
            pos: at(0.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
        let undo_before = h.state().ws.as_ref().unwrap().undo_depth();
        let mut steps = Vec::new();
        for i in 1..=40 {
            h.event(egui::Event::PointerMoved(at(i as f32 * 4.0)));
            let t = Instant::now();
            h.run_steps(1);
            steps.push(t.elapsed());
        }
        h.event(egui::Event::PointerButton {
            pos: at(160.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(2);
        let made = h.state().ws.as_ref().unwrap().undo_depth() - undo_before;
        let mut s = steps.clone();
        s.sort();
        println!(
            "  dragging the page's swatch: a step {:.2} ms (median), {:.2} worst; {made} commands",
            ms(s[s.len() / 2]),
            ms(*s.last().unwrap())
        );
        if std::env::var_os("TINT_SHOT").is_some() {
            let dir = shots();
            h.render().unwrap().save(dir.join("after.png")).unwrap();
        }
    }
    // A command each frame, as a drag across the swatch's picker made.
    let dragged = frames(&mut h, rect, Pointer::Away, 40, |app, n| app.actions.push(tinted(n)));
    let med = |mut v: Vec<Duration>| {
        v.sort();
        (ms(v[v.len() / 2]), ms(*v.last().unwrap()))
    };
    println!("{area} ({page} page): (median, worst) ms");
    for (what, f) in [("quiet", quiet), ("a tint's command each frame", dragged)] {
        println!("  {what}: ui {:?}, gpu {:?}", med(f.ui), med(f.gpu));
    }
}

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
    println!("{CAMPAIGN}: frames at {:?}, budget {:.2} ms", screen_size(), ms(BUDGET));
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
    fold_palette(&mut h);
    close(&mut h, &Tab::ModuleProperties);
    view(&mut h, &mut r, "module with its area, nothing else", None);
    // The area beside the Right Dock with a creature chosen in the palette:
    // the Preview's tab in front (its 3D view drawn again as it moves),
    // then the Palette's: what the Preview costs.
    h.state_mut().palette.selected = ResKey::parse("nw_bandit001", ResType::UTC);
    show_preview(&mut h);
    view(&mut h, &mut r, "area, a creature in the Preview tab", None);
    show_palette(&mut h);
    view(&mut h, &mut r, "area, a creature, the Palette tab in front", None);
    // The creature's model in a tab of its own, the Preview tab showing the
    // picture the tab draws: one picture a frame, shown in both (the
    // owner's ruling 19, M5's W6).
    let bandit = ResKey::parse("nw_bandit001", ResType::UTC).unwrap();
    let t = Tab::Model(bandit);
    open(&mut h, t.clone());
    show_preview(&mut h);
    view(&mut h, &mut r, "a creature's model tab, the Preview tab sharing its picture", Some(&t));
    close(&mut h, &t);
    h.state_mut().palette.selected = None;
    fold_palette(&mut h);
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
    click(&mut h, "File");
    view(&mut h, &mut r, "File menu open", None);
    h.key_press(egui::Key::Escape);
    h.run_steps(2);

    tab(&mut h, &mut r, "Module Properties", Tab::ModuleProperties);
    for (kind, name) in [
        (BlueprintKind::Creature, "creature"),
        (BlueprintKind::Item, "item"),
        (BlueprintKind::Placeable, "placeable"),
    ] {
        for custom in [false, true] {
            palette(&mut h, kind, custom, "");
            let which = if custom { "custom" } else { "standard" };
            palette_view(&mut h, &mut r, &format!("{which} {name} palette"));
        }
    }
    // (A search opens every category with a find in it.)
    palette(&mut h, BlueprintKind::Placeable, false, "a");
    palette_view(&mut h, &mut r, "standard placeable palette, searched for \"a\"");
    palette(&mut h, BlueprintKind::Item, false, "e");
    palette_view(&mut h, &mut r, "standard item palette, searched for \"e\"");
    // The Gallery: its pictures are made a few a frame, then kept; their
    // models and textures read apart, as the desktop app reads them.
    let background = h.state().background_jobs;
    h.state_mut().background_jobs = true;
    h.state_mut().settings.palette_gallery = true;
    for (side, what) in [(128, "Gallery"), (64, "Gallery, smallest pictures")] {
        h.state_mut().settings.gallery_tile = Some(side);
        palette(&mut h, BlueprintKind::Placeable, false, "a");
        show_palette(&mut h);
        h.run_steps(300);
        let rect = palette_rect(&h);
        view_in(&mut h, &mut r, &format!("standard placeable palette, {what}"), rect, |_, _| {});
        fold_palette(&mut h);
    }
    // The large palette (M4's U25): the standard placeables' Gallery with
    // every category open, no search, its pictures made, the wheel
    // turned over it from the top on.
    h.state_mut().settings.gallery_tile = Some(64);
    palette(&mut h, BlueprintKind::Placeable, false, "");
    show_palette(&mut h);
    let dock = h.state().right_dock_rect().expect("the Right Dock");
    let expand = h.query_all_by_label("Expand All").find(|n| dock.contains(n.rect().center()));
    expand.expect("the palette's Expand All").click();
    h.run_steps(300);
    let rect = palette_rect(&h);
    view_in(&mut h, &mut r, "standard placeable palette, Gallery, every category", rect, |_, _| {});
    fold_palette(&mut h);
    h.state_mut().settings.palette_open.clear();
    h.state_mut().settings.palette_gallery = false;
    h.state_mut().settings.gallery_tile = None;
    // An armor's Appearance page (M4's U25): the game's nw_aarcl001
    // viewed, its first part's pictures under its lists, made.
    let armor = ResKey::parse("nw_aarcl001", ResType::UTI).unwrap();
    let gff = {
        let game = h.state().game.as_deref().expect("the game's data");
        mg_gff::Gff::read(&game.resman.get(&armor).expect("nw_aarcl001")).unwrap()
    };
    h.state_mut().ws.as_mut().expect("the module").view(armor, gff);
    h.state_mut().blueprint_pages.insert((armor, mg_edit::GffPath::root()), "Appearance");
    open(&mut h, Tab::Blueprint(armor));
    h.run_steps(300);
    assert!(
        h.query_all_by_label_contains("Pictures of").next().is_some(),
        "the armor's part pictures"
    );
    view(
        &mut h,
        &mut r,
        "armor nw_aarcl001, Appearance page pictures",
        Some(&Tab::Blueprint(armor)),
    );
    close(&mut h, &Tab::Blueprint(armor));
    h.state_mut().settings.palette_gallery = false;
    h.state_mut().settings.gallery_tile = None;
    h.state_mut().background_jobs = background;
    h.state_mut().palette.filter.clear();

    resources_panel(&mut h, &mut r, "resource browser");
    factions(&mut h, &mut r);
    journal(&mut h, &mut r);
    tab(&mut h, &mut r, "Placeable Gallery", Tab::PlaceableGallery);
    tab(&mut h, &mut r, "User Manual", Tab::Manual);
    talk_table(&mut h, &mut r);
    tileset(&mut h, &mut r);
    hak(&mut h, &mut r);
    if let Some(k) = dialog {
        // (With five bookmarks, listed in the Outliner beside it since M4's
        // U15, its tree open with its indent guides.)
        let t = Tab::Dialog(k);
        open(&mut h, t.clone());
        let marks = (0..5).map(|i| (mg_module::dialog::Kind::Entry, i)).collect();
        h.state_mut().dialog_views.entry(k).or_default().bookmarks = marks;
        h.run_steps(3);
        view(&mut h, &mut r, &format!("conversation {}", k.resref), Some(&t));
        close(&mut h, &t);
    }
    if let Some(k) = script {
        tab(&mut h, &mut r, &format!("script {}", k.resref), Tab::Script(k));
    }
    for (k, what) in
        [(creature, "creature"), (item, "item"), (placeable, "placeable"), (store, "store")]
    {
        if let Some(k) = k {
            tab(&mut h, &mut r, &format!("{what} {}", k.resref), Tab::Blueprint(k));
            // (With its own Preview unfolded beside it, its model drawn
            // again as it moves: M4's U11.)
            if what != "store" {
                let t = Tab::Blueprint(k);
                open(&mut h, t.clone());
                show_preview(&mut h);
                view(&mut h, &mut r, &format!("{what} {}, its Preview", k.resref), Some(&t));
                fold_palette(&mut h);
                close(&mut h, &t);
            }
            if what == "creature" {
                pages_scrolled(&mut h, &mut r, &format!("creature {}", k.resref), k);
            }
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
    r.end();
}

/// A copy of the game's tcn01 in the tileset editor (M4's U20), docked
/// As Tabs: its tile table of 408 rows (the rows in sight alone laid out)
/// on the Tiles page, tile 16 chosen; Check run on it (two faults put in:
/// tile 17's model one nowhere, group 0's first tile -1), its findings in
/// Problems, in front of the Bottom Dock; the Preview and Textures in the
/// Right Dock (the tile's 3D view paused, as a tile's starts), its parts
/// in the Outliner. (The tip's copy, 3229ef5c, takes
/// the same steps, its findings listed under its row; it has no Problems
/// to show.)
fn tileset(h: &mut App<'_>, r: &mut Report) {
    let what = "tileset tcn01, its tile table of 408 rows";
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) {
        return;
    }
    let was = h.state().settings.open_editors;
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    let key = ResKey::parse("tcn01", ResType::SET).unwrap();
    let set = {
        let game = h.state().game.as_deref().expect("the game's data");
        game.resman.get(&key).unwrap().into_owned()
    };
    let mut text = String::from_utf8_lossy(&set).into_owned();
    let t = mg_set::Tileset::parse(&set, mg_core::Codepage::default()).unwrap();
    text = text.replacen(&format!("Model={}", t.tiles[17].model), "Model=tcn01_zz17", 1);
    let group = text.find("[GROUP0]").unwrap();
    let tile0 = group + text[group..].find("Tile0=").unwrap();
    let end = tile0 + text[tile0..].find("\r\n").unwrap();
    text.replace_range(tile0..end, "Tile0=-1");
    let path = mg_testkit::scratch_dir("frame-tileset").join("tcn01.set");
    std::fs::write(&path, text).unwrap();
    mg_ui::tileset_view::open_path(h.state_mut(), path);
    h.run_steps(3);
    let id = h.state().tilesets.last().expect("the tileset open").id;
    let t = Tab::Tileset(id);
    {
        let d = h.state_mut().tilesets.iter_mut().find(|d| d.id == id).unwrap();
        d.page = mg_ui::tileset_view::Page::Tiles;
        d.tile = Some(16);
    }
    h.run_steps(3);
    click(h, "Check");
    // (Its panels shown, loudly: the Right Dock may be folded by the views
    // before, the Bottom Dock closed.)
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Preview));
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Problems));
    h.run_steps(5);
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Problems), "Problems not shown");
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Preview), "Preview not shown");
    // (Under 1400 Preview and Textures fold into one tab bar, Preview's in
    // front.)
    if screen_size().x >= 1400.0 {
        assert!(h.query_by_label_contains("Textures, of ").is_some(), "Textures not shown");
    } else {
        tabs_folded(h, &["Preview", "Textures"], "Tile diagram: tile 16");
    }
    assert!(h.query_by_label("408 tiles").is_some(), "the Outliner's parts not shown");
    let tiles = h
        .state()
        .tilesets
        .iter()
        .find(|d| d.id == id)
        .map_or(0, |d| d.tileset().map_or(0, |t| t.tiles.len()));
    let found: usize = h.state().problems.sets().iter().map(|s| s.problems.len()).sum();
    println!(
        "  ({tiles} tiles; {found} found by Check; the Right Dock drawn: {}; the Outliner's parts: {})",
        h.state().right_dock_rect().is_some(),
        h.query_by_label("408 tiles").is_some()
    );
    view(h, r, what, Some(&t));
    close(h, &t);
    h.state_mut().settings.open_editors = was;
}

/// The game's `id_resources.hak` in the hak editor (M4's U21), docked As
/// Tabs: its 2,706 resources sorted by type (the rows in sight alone laid
/// out), `c_masterius.mdl` chosen; the Preview (its model's 3D view
/// paused, as a model's starts) and Details in the Right Dock, its types in
/// the Outliner. (The tip's copy, 549b08f3, takes the same steps without
/// the panels' assertions: it has none of them.)
fn hak(h: &mut App<'_>, r: &mut Report) {
    let what = "hak id_resources.hak, its 2706 rows sorted by type";
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) {
        return;
    }
    let Some(root) = h.state().install.as_ref().map(|gi| gi.root.clone()) else { return };
    let was = h.state().settings.open_editors;
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    let id = mg_ui::hak_view::open_at(h.state_mut(), &root.join("data/hk/id_resources.hak"))
        .expect("the game's hak");
    let t = Tab::Hak(id);
    let masterius = ResKey::parse("c_masterius", ResType::MDL).unwrap();
    // (A file added, Hak.png's `mg_chair.mdl`: its 2,706 rows.)
    let chair = mg_testkit::scratch_dir("frame-hak").join("mg_chair.mdl");
    std::fs::write(&chair, b"chair").unwrap();
    {
        let d = h.state_mut().haks.iter_mut().find(|d| d.id == id).unwrap();
        d.hak.add_files(&[chair]);
        d.sort = mg_ui::hak_view::Sort::Type;
        d.selected = [masterius].into();
    }
    h.run_steps(3);
    // (Its panels shown, loudly: the Right Dock may be folded by the views
    // before.)
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Preview));
    h.run_steps(5);
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Preview), "Preview not shown");
    // (Under 1400 Preview and Details fold into one tab bar, Preview's in
    // front.)
    if screen_size().x >= 1400.0 {
        assert!(h.query_by_label_contains("Details, ").is_some(), "Details not shown");
    } else {
        tabs_folded(h, &["Preview", "Details"], "Animation");
    }
    assert!(h.query_by_label_contains(".mdl, 9").is_some(), "no Outliner");
    let rows = h.state().haks.iter().find(|d| d.id == id).map_or(0, |d| d.hak.items().len());
    println!(
        "  ({rows} resources; the Right Dock drawn: {}; the Outliner's types: {})",
        h.state().right_dock_rect().is_some(),
        h.query_all_by_label_contains(".mdl, ").next().is_some()
    );
    view(h, r, what, Some(&t));
    close(h, &t);
    h.state_mut().settings.open_editors = was;
}

/// The Faction Editor (M4's U22), docked As Tabs: Advanced's matrix with
/// its frozen headers, the second faction's cell toward the first chosen,
/// Faction, Reaction and Reactions in the Right Dock and the factions in
/// the Outliner (asserted shown). How many factions it draws is printed.
fn factions(h: &mut App<'_>, r: &mut Report) {
    let what = "Faction Editor";
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) {
        return;
    }
    let was = h.state().settings.open_editors;
    let folded = h.state().right_dock_shows().is_none();
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    let t = Tab::Factions;
    {
        let v = &mut h.state_mut().faction_view;
        v.advanced = true;
        v.selected = 2;
        v.target = 1;
    }
    open(h, t.clone());
    // (Its panels shown, loudly: the Right Dock may be folded by the views
    // before.)
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Reaction));
    h.run_steps(5);
    assert!(h.state().dock.find_tab(&t).is_some_and(|at| at.surface.is_main()), "not docked");
    assert!(h.state().right_dock_rect().is_some(), "no Right Dock");
    if screen_size().x >= 1400.0 {
        assert!(h.query_by_label("Global effect").is_some(), "Faction not shown");
        assert!(h.query_by_label("Full detail").is_some(), "Reactions not shown");
    } else {
        // (Under 1400 the three panels fold into one tab bar, Reaction's in
        // front.)
        assert_eq!(
            h.state().right_dock_shows(),
            Some(mg_ui::Panel::Reaction),
            "Reaction not shown"
        );
        // (Reaction's own slider: the matrix's cells, " toward ", have
        // rects past the editor's edge, under the dock, drawn or not.)
        tabs_folded(h, &["Faction", "Reaction", "Reactions"], "Reputation value");
    }
    let outliner =
        h.query_all_by_label(mg_ui::OUTLINER).any(|g| g.query_by_label("Hostile").is_some());
    assert!(outliner, "no Outliner of the factions");
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    let count = mg_module::factions::Factions::of_module(&ws.module).factions.len();
    let cells = h
        .query_by_label(mg_ui::faction_view::MATRIX)
        .map_or(0, |m| m.query_all_by_label_contains(" toward ").count());
    println!(
        "  ({count} factions, {cells} cells laid out; the Right Dock drawn: {}; the Outliner: {outliner})",
        h.state().right_dock_rect().is_some(),
    );
    view(h, r, what, Some(&t));
    close(h, &t);
    h.state_mut().settings.open_editors = was;
    // (The Right Dock folded again as it was found: the views after
    // unfold it on the panel they ask for.)
    if folded {
        fold_palette(h);
    }
}

/// The Journal Editor (M4's U22), docked As Tabs: its first category opened
/// and its first entry chosen, Entry and Category in the Right Dock
/// (asserted shown). How many categories and entries it has is printed.
fn journal(h: &mut App<'_>, r: &mut Report) {
    let what = "Journal Editor";
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) {
        return;
    }
    let was = h.state().settings.open_editors;
    let folded = h.state().right_dock_shows().is_none();
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    let t = Tab::Journal;
    {
        let v = &mut h.state_mut().journal_view;
        v.open = [0].into();
        v.selected = Some(mg_ui::journal_view::Node::Entry(0, 0));
    }
    open(h, t.clone());
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Entry));
    h.run_steps(5);
    assert!(h.state().dock.find_tab(&t).is_some_and(|at| at.surface.is_main()), "not docked");
    assert!(h.state().right_dock_rect().is_some(), "no Right Dock");
    assert!(h.query_by_label("Finish category").is_some(), "Entry not shown");
    let jrl = ResKey::parse("module", ResType::JRL).unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let cats: Vec<mg_gff::Struct> = ws
        .doc(&jrl)
        .ok()
        .and_then(|g| g.root.list("Categories").map(<[_]>::to_vec))
        .unwrap_or_default();
    let entries: usize = cats.iter().map(|c| mg_module::journal::entries(c).len()).sum();
    println!(
        "  ({} categories, {entries} entries; the Right Dock drawn: {})",
        cats.len(),
        h.state().right_dock_rect().is_some(),
    );
    view(h, r, what, Some(&t));
    close(h, &t);
    h.state_mut().settings.open_editors = was;
    if folded {
        fold_palette(h);
    }
}

/// The game's `dialog.tlk` (M4's U17), docked As Tabs: its 112,228 lines
/// scrolled by the wheel (the rows in sight alone laid out), line 18608
/// chosen, String, References and Talk Table in the Right Dock beside it.
/// The rows in sight before and after are printed: the wheel scrolls them.
fn talk_table(h: &mut App<'_>, r: &mut Report) {
    let what = "talk table dialog.tlk, docked";
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) {
        return;
    }
    let was = h.state().settings.open_editors;
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    // (Shown in the Talk Table tab, as the Resources panel's row shows it.)
    let t = Tab::TalkTable;
    h.state_mut().actions.push(Action::ShowGameTalk);
    h.run_steps(3);
    if let Some(g) = h.state_mut().game_talk.as_mut() {
        g.view.go_to_line(18608);
    }
    // (Its panels in the Right Dock, unfolded on String: loudly.)
    h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::String));
    h.run_steps(5);
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::String), "String not shown");
    let seen = |h: &App<'_>| {
        let id = egui::Id::new(("talk-rows-seen", mg_ui::talk_view::Which::Game));
        h.ctx.data(|d| d.get_temp::<(usize, usize)>(id))
    };
    let lines = h.state().game_talk.as_ref().map_or(0, |g| g.table.len());
    let panels = h.state().right_dock_rect().is_some();
    println!("  ({lines} lines; the Right Dock drawn: {panels}; rows in sight {:?})", seen(h));
    view(h, r, what, Some(&t));
    println!("  (rows in sight after the wheel: {:?})", seen(h));
    // Only lines with text switched each frame (the list made anew of
    // 112,228 lines each time), and Find looking for "the" and "and" by
    // turns (begun again each frame: its slice of a frame every frame):
    // what a frame costs at their worst.
    let switch = |app: &mut mg_ui::Moonglow, n: usize| {
        if let Some(g) = app.game_talk.as_mut() {
            g.view.only_text = n.is_multiple_of(2);
        }
    };
    view_with(h, r, "talk table dialog.tlk, Only lines with text switched", Some(&t), switch);
    if let Some(g) = h.state_mut().game_talk.as_mut() {
        g.view.only_text = false;
    }
    let find = |app: &mut mg_ui::Moonglow, n: usize| {
        if let Some(g) = app.game_talk.as_mut() {
            g.view.filter = if n.is_multiple_of(2) { "the".into() } else { "and".into() };
        }
    };
    view_with(h, r, "talk table dialog.tlk, Find begun again each frame", Some(&t), find);
    if let Some(g) = h.state_mut().game_talk.as_mut() {
        g.view.filter.clear();
    }
    // Find "e" with Only lines with text on (M4's U17 review): each frame
    // of it, from its first to its last (its last fifth, where the lines
    // found so far are most), a frame (`step`: its pass, tessellated)
    // timed on its own.
    if let Some(g) = h.state_mut().game_talk.as_mut() {
        g.view.only_text = true;
        g.view.filter = "e".into();
    }
    let sweeping = |h: &App<'_>| h.state().game_talk.as_ref().is_some_and(|g| g.view.sweeping());
    let mut times = Vec::new();
    let began = Instant::now();
    loop {
        let at = Instant::now();
        h.step();
        times.push(at.elapsed());
        if (!sweeping(h) && times.len() > 1) || began.elapsed() > Duration::from_secs(60) {
            break;
        }
    }
    let late = times[times.len() * 4 / 5..].to_vec();
    let worst = times.iter().max().copied().unwrap_or_default();
    println!(
        "  talk table dialog.tlk, Find \"e\" with Only lines with text: {} frames, the last fifth's median {:.2} ms (worst {:.2}), the worst of all {:.2} ms",
        times.len(),
        ms(pick(late.clone(), 0.5)),
        ms(late.iter().max().copied().unwrap_or_default()),
        ms(worst),
    );
    if let Some(g) = h.state_mut().game_talk.as_mut() {
        g.view.only_text = false;
        g.view.filter.clear();
    }
    close(h, &t);
    fold_palette(h);
    h.state_mut().settings.open_editors = was;
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
    println!("a persistent world: frames at {:?}, budget {:.2} ms", screen_size(), ms(BUDGET));
    let rs = render_state();
    let install = GameInstall::new(&root, Some(w.user.clone()), "en");
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&w.module);
    let mut h = harness(app, rs);
    h.run_steps(3);
    fold_palette(&mut h);
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
    // A word in the tree's Filter: every area's placed objects are read
    // for it (over some frames), and what it finds is then kept, not
    // looked for each frame (the review's S19).
    let field = h
        .query_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .map(|n| n.click());
    assert!(field.is_some(), "the tree's Filter field");
    h.run_steps(2);
    h.event(egui::Event::Text("door".into()));
    h.run_steps(300);
    view(&mut h, &mut r, "module tree, a word in the Filter (300 areas' objects)", None);
    // Cleared, and the groups the word opened closed again, as the
    // tree was for the views after (open, they cost what the tree
    // listed whole does).
    click(&mut h, "Clear Filter");
    click(&mut h, "Collapse All");

    palette(&mut h, BlueprintKind::Item, true, "");
    palette_view(&mut h, &mut r, "custom item palette (8,000)");
    palette(&mut h, BlueprintKind::Item, true, "pw_i");
    palette_view(&mut h, &mut r, "custom item palette, all 8,000 shown");
    palette(&mut h, BlueprintKind::Placeable, false, "pw placeable");
    palette_view(&mut h, &mut r, "standard placeable palette (+10,000)");
    h.state_mut().palette.filter.clear();

    let store = Tab::Blueprint(ResKey::parse("pw_store", ResType::UTM).unwrap());
    open(&mut h, store.clone());
    h.state_mut().blueprint_pages.insert(
        (ResKey::parse("pw_store", ResType::UTM).unwrap(), mg_edit::GffPath::root()),
        "Inventory",
    );
    h.run_steps(3);
    view(&mut h, &mut r, "store inventory with 1,000 items", Some(&store));
    // Its picker on the custom palette, searched for what all 8,000 have:
    // every blueprint folded into the tree, every match drawn.
    click(&mut h, "Custom Palette");
    let found = egui_kittest::kittest::by().predicate(|n| n.placeholder() == Some("Find"));
    let find = h.query_all(found).last().expect("the picker's Find field");
    find.click();
    h.run_steps(2);
    h.event(egui::Event::Text("pw_i".into()));
    h.run_steps(3);
    view(&mut h, &mut r, "store inventory, picker searched (8,000 found)", Some(&store));
    close(&mut h, &store);
    // An item's Properties page: every property it may take, by kind.
    let item = Tab::Blueprint(ResKey::parse("pw_i00001", ResType::UTI).unwrap());
    open(&mut h, item.clone());
    click_outside(&mut h, "Properties", "Left Dock");
    view(&mut h, &mut r, "item blueprint, Properties page", Some(&item));
    close(&mut h, &item);
    // The same store in the Right Dock's Preview panel (its stock by
    // page), under the palette.
    h.state_mut().palette.selected = ResKey::parse("pw_store", ResType::UTM);
    show_preview(&mut h);
    view(&mut h, &mut r, "Preview panel, a store of 1,000 items", None);
    show_palette(&mut h);
    fold_palette(&mut h);
    resources_panel(&mut h, &mut r, "resource browser (300,000)");
    let areas = h.state().ws.as_ref().unwrap().module.areas().unwrap();
    tab(&mut h, &mut r, "Area Properties (300 areas)", Tab::AreaProperties(areas[0]));
    // Advanced Search for any variable, every hit ticked: the rows in
    // sight laid out, not one a hit (the review's D23).
    {
        use mg_ui::search::{AdvancedSearch, Condition, Kind};
        let mut d = AdvancedSearch {
            first: Condition { kind: Kind::Variable, name: "*".into(), ..Default::default() },
            ..Default::default()
        };
        h.state_mut().advanced_search(&mut d);
        let found = d.hits.len();
        d.ticked = vec![true; found];
        h.state_mut().advanced_search = d;
        tab(&mut h, &mut r, &format!("Advanced Search, {found} hits ticked"), Tab::AdvancedSearch);
    }
    // The Appearance Gallery of creatures over the haks' appearance.2da
    // (17,100 rows): its names sorted once for the table, not a frame
    // (the review's B22).
    {
        let mut gallery = mg_ui::appearance_gallery::Gallery::default();
        gallery.kind = mg_ui::appearance_gallery::Kind::Creature;
        h.state_mut().placeable_gallery = Some(gallery);
        tab(&mut h, &mut r, "Appearance Gallery, creatures (17,100 rows)", Tab::PlaceableGallery);
        h.state_mut().placeable_gallery = None;
    }
    // Find Instance with nothing typed: every placed object of the 300
    // areas found, the rows in sight laid out (the review's A18). (M4's
    // U28: opened by Edit's menu, its form and hits in Search Results in
    // the Bottom Dock.)
    menu(&mut h, "Edit", "Find Instance…");
    assert_eq!(
        h.state().bottom_dock_shows(),
        Some(mg_ui::Panel::SearchResults),
        "Search Results not shown"
    );
    click(&mut h, "Search");
    // (A job since U28: until it ends.)
    let started = std::time::Instant::now();
    while h.state().busy() {
        h.step();
        assert!(started.elapsed().as_secs() < 120, "the search did not end");
    }
    h.run_steps(3);
    let found = h.state().find_instance.as_ref().map_or(0, |f| f.results.len());
    view(&mut h, &mut r, &format!("Find Instance, {found} found"), None);
    h.state_mut().find_instance = None;
    // Build Module (Unused and Missing Resources, not Compile) over the
    // 300 areas: its findings in Problems in the Bottom Dock, the dialog
    // over the view.
    menu(&mut h, "Build", "Build Module…");
    if let Some(w) = h.state_mut().build.as_mut() {
        (w.unused, w.missing, w.compile) = (true, true, false);
    }
    h.run_steps(2);
    h.query_all_by_label("Build").last().expect("Build").click_accesskit();
    // (Until the build's job ends.)
    let started = std::time::Instant::now();
    h.run_steps(2);
    while h.state().busy() {
        h.step();
        assert!(started.elapsed().as_secs() < 120, "the build did not end");
    }
    h.run_steps(3);
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Problems), "Problems not shown");
    let found =
        h.state().problems.of(&mg_ui::problems::Source::Build).map_or(0, |s| s.problems.len());
    view(&mut h, &mut r, &format!("Build Module, {found} found, in Problems"), None);
    h.state_mut().build = None;

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
    // Options, on the page it opens on: Folders, which looks at the user
    // folder and its nwn.ini (once a while, not seven reads a frame: the
    // review's S21).
    h.state_mut().actions.push(Action::OptionsDialog);
    h.run_steps(3);
    view(&mut h, &mut r, "Options, Folders", Some(&Tab::Options));
    close(&mut h, &Tab::Options);
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
                    fix: mg_edit::fix::Fix { label: "Fix".into(), edits: Vec::new(), asks: None },
                    made: false,
                    check: "a-check".into(),
                    step: None,
                    record: None,
                });
            }
        }
    }
    h.run_steps(2);
    view(&mut h, &mut r, "log of 10,000 lines, 2,000 fixes", None);
    h.state_mut().log.clear();
    r.end();
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
    fold_palette(&mut h);
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
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
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
    r.took("reload after a pull, its longest frame".into(), longest);
    r.end();
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
    // (In a strong wind: the grass sways, each tile's posed.)
    {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        are.root.set("WindPower", mg_gff::Value::Int(2));
        m.set_gff(key, &are).unwrap();
    }
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
    r.end();
}

/// Sound Ranges' cost: the campaign's area with the most placed sounds,
/// in the viewer with the switch off and then on (each distance of each
/// positional sound a dotted sphere). Needs the game and a GPU; run in
/// release: `cargo test --release -p mg-ui --test frame_perf sound_ranges
/// -- --ignored --nocapture`.
#[test]
#[ignore]
fn sound_ranges_frames_at_144() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let path = root.join("data/nwm").join(CAMPAIGN);
    if !path.is_file() {
        println!("skipped: {CAMPAIGN} isn't installed");
        return;
    }
    let mut r = Report::new();
    let rs = render_state();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    // The area with the most positional sounds, and its size in tiles.
    let (area, tiles, sounds) = {
        let ws = app.ws.as_mut().unwrap();
        let keys: Vec<_> = ws.module.keys_of(ResType::GIT).copied().collect();
        let mut best = None;
        for k in keys {
            let count = ws.doc(&k).ok().map_or(0, |g| {
                let list = g.root.list("SoundList").unwrap_or(&[]);
                list.iter().filter(|s| s.integer("Positional").unwrap_or(0) != 0).count()
            });
            let are = ws.doc(&ResKey::new(k.resref, ResType::ARE)).ok();
            let size = are.and_then(|a| Some(a.root.integer("Width")? * a.root.integer("Height")?));
            if best.as_ref().is_none_or(|(_, _, n)| count > *n) {
                best = Some((k.resref, size.unwrap_or(0), count));
            }
        }
        best.unwrap()
    };
    println!(
        "Sound Ranges: {CAMPAIGN}'s {area}, {sounds} positional sounds, frames at {:?}, \
         budget {:.2} ms",
        screen_size(),
        ms(BUDGET)
    );
    let mut h = harness(app, rs);
    h.run_steps(3);
    for on in [false, true] {
        h.state_mut().settings.sound_ranges = on;
        let label = if on { " (Sound Ranges on)" } else { " (Sound Ranges off)" };
        area_views(&mut h, &mut r, area, tiles, label);
    }
    // What the GPU holds once the area is closed (`FRAME_MEMORY`): with
    // `FRAME_SHADOW_QUALITY=high`, the stencil volumes' target and the
    // meshes' volumes let go of (the review's R5), and the framebuffer's
    // copies (R10).
    if std::env::var_os("FRAME_MEMORY").is_some() {
        let tab = Tab::Area(area);
        said_memory(&h, &format!("area {area}, open"));
        close(&mut h, &tab);
        // (Past the time a view not drawn lets go of its targets, and a
        // closed one of its scene: three seconds of the harness's clock.)
        h.run_steps(3 * FPS as usize);
        let held = h.state().area_views.get(&area).is_some_and(|v| v.holds_scene());
        said_memory(&h, &format!("area {area}, closed{}", if held { "" } else { ", let go of" }));
        if let Some(vp) = h.state().viewport.as_ref() {
            println!("      shadow volumes kept: {}", vp.renderer.shadow_volumes_kept());
        }
    }
    r.end();
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
    println!("the Shaders switch: frames at {:?}, budget {:.2} ms", screen_size(), ms(BUDGET));
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
    r.end();
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
/// frames go on). Then with the pipelines the module's areas drew with
/// kept from the run before (`mg_ui::pipelines_kept`), made as the module
/// opens: the area opened at once, and opened a second later (a builder's
/// first moment in the module).
fn shaders_opening(
    root: &std::path::Path,
    user: Option<std::path::PathBuf>,
    module: &std::path::Path,
    area: ResRef,
) {
    let cache = mg_testkit::scratch_dir("frame-shader-cache");
    let kept = mg_testkit::scratch_dir("frame-pipelines-kept");
    let runs = [
        ("the cache empty", false, false),
        ("the cache filled", true, false),
        ("the cache filled, the pipelines kept", true, false),
        ("the cache filled, the pipelines kept, the area opened a second later", true, true),
    ];
    for (run, keep, later) in runs {
        let rs = render_state();
        let install = GameInstall::new(root, user.clone(), "en");
        let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
        app.set_render_state(rs.clone());
        app.shader_cache_dir = Some(cache.clone());
        app.pipelines_dir = keep.then(|| kept.clone());
        app.settings.game_shaders = Some(true);
        // (From the module's opening: it may open the area itself.)
        let t0 = Instant::now();
        app.open_module(module);
        let mut h = harness(app, rs);
        if later {
            while t0.elapsed() < Duration::from_secs(1) {
                h.run_steps(1);
            }
        }
        let t0 = if later { Instant::now() } else { t0 };
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
        // (A frame more: the pipelines made kept for the next run.)
        h.run_steps(2);
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
    fold_palette(h);
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

/// M4's U23: the read-only tab of an authored GFF of 2,100 fields (30
/// structs of 70, each unfolded: clicked from the last up, the rows
/// above staying where they were), from a scratch user folder's
/// `override`, docked As Tabs; then Options on its General page, docked.
/// Prints how many of the GFF's field rows were laid out (AccessKit nodes
/// named, or plain labels valued, `f..`: the rows in sight alone where
/// only they are drawn).
/// `FRAME_VIEW` picks one ("read-only GFF", "Options General").
#[test]
#[ignore]
fn read_only_and_options() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let user = mg_testkit::scratch_dir("frame-perf-u23").join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    let mut gff = mg_gff::Gff::new(*b"UTC ");
    for s in 0..30u32 {
        let mut child = mg_gff::Struct { id: s, ..Default::default() };
        for f in 0..70 {
            child.set(&format!("f{s:02}_{f:02}"), mg_gff::Value::Int(f));
        }
        gff.root.set(&format!("S{s:02}"), mg_gff::Value::Struct(child));
    }
    std::fs::write(user.join("override/mg_perf_big.utc"), gff.to_bytes().unwrap()).unwrap();
    let mut r = Report::new();
    println!("read-only GFF and Options: frames at {:?}", screen_size());
    let rs = render_state();
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.set_render_state(rs.clone());
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    let mut h = harness(app, rs);
    h.run_steps(3);
    let wanted = |r: &Report, what: &str| r.only.as_ref().is_none_or(|o| what.contains(o.as_str()));
    let what = "read-only GFF, 2,100 fields unfolded, docked";
    if wanted(&r, what) {
        let key = ResKey::parse("mg_perf_big", ResType::UTC).unwrap();
        let t = Tab::Resource(key);
        open(&mut h, t.clone());
        h.run_steps(5);
        // (A short window shows the rows only scrolled to: the view builds
        // the rows in sight alone. Unfolded from the last up, a row unfolded
        // leaves those above it where they are; scrolled up to as needed.)
        let at = h.query_all_by_label_contains("S00").next().map(|n| n.rect().center());
        let at = at.expect("no S00 row");
        let wheel = |h: &mut App<'_>, by: f32| {
            h.event(egui::Event::PointerMoved(at));
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, by),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            });
            h.run_steps(2);
        };
        for s in (0..30).rev() {
            let name = format!("S{s:02}");
            let by = if s == 29 { -60.0 } else { 60.0 };
            for _ in 0..200 {
                if h.query_all_by_label_contains(&name).next().is_some() {
                    break;
                }
                wheel(&mut h, by);
            }
            let found = h.query_all_by_label_contains(&name).next().map(|n| n.click()).is_some();
            assert!(found, "no {name} row");
            h.run_steps(2);
        }
        let fields = |h: &App<'_>| {
            h.query_all(egui_kittest::kittest::by().predicate(|n| {
                // (A row's name: its label, or a plain label's value.)
                let name = n.label().or_else(|| n.value());
                name.is_some_and(|l| l.starts_with('f') && l.as_bytes().get(3) == Some(&b'_'))
            }))
            .count()
        };
        let laid = fields(&h);
        assert!(laid > 0, "no field rows: not unfolded");
        println!("  ({laid} of the 2,100 field rows laid out)");
        view(&mut h, &mut r, what, Some(&t));
        close(&mut h, &t);
    }
    let what = "Options General, docked";
    if wanted(&r, what) {
        h.state_mut().actions.push(Action::OptionsDialog);
        h.run_steps(5);
        click(&mut h, "General");
        h.run_steps(3);
        assert!(h.query_by_label_contains("Create backups of modules").is_some(), "not General");
        view(&mut h, &mut r, what, Some(&Tab::Options));
    }
    r.end();
}

/// M4's U24: the Welcome page with 200 recent modules (paths alone, made
/// up and never read: none of them is there; a `.mod`, a `.nwm`, a module
/// folder and a nasher project in turn), no module open. Prints how many
/// of the 200 rows were laid out (AccessKit nodes whose names hold
/// "Recent module ", the rows in sight alone where only they are drawn).
/// `FRAME_VIEW=Welcome` picks it.
#[test]
#[ignore]
fn welcome_page() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let mut r = Report::new();
    println!("Welcome: frames at {:?}", screen_size());
    let rs = render_state();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.settings.recent = (0..200)
        .map(|i| {
            let place = format!("/srv/nwn/modules/set {i:03}");
            std::path::PathBuf::from(match i % 4 {
                0 => format!("{place}/Recent module {i}.mod"),
                1 => format!("{place}/Recent module {i}.nwm"),
                2 => format!("{place}/Recent module {i}"),
                _ => format!("{place}/Recent module {i} project"),
            })
        })
        .collect();
    let mut h = harness(app, rs);
    h.run_steps(3);
    let what = "Welcome, 200 recent modules, no module";
    if r.only.as_ref().is_none_or(|o| what.contains(o.as_str())) {
        let laid = h.query_all_by_label_contains("Recent module ").count();
        assert!(laid > 0, "no recent rows: not the Welcome page");
        println!("  ({laid} of the 200 recent rows laid out)");
        view(&mut h, &mut r, what, Some(&Tab::Welcome));
    }
    r.end();
}

/// M4's U29 (the review's A18, `docs/deferred.md`): Edit Areas Together
/// over an authored module of 3,000 areas (each a 1 by 1 area of the
/// game's Rural tileset, a third of them interior, a third underground),
/// opened by Edit's menu: the 3,000 listed, then all ticked (Tick Shown),
/// then their shared Visual page (before U29 the Area Properties window
/// of the 3,000, opened by its Edit 3000 Together; since U29 the Right
/// Dock's Edit Areas Together section, its Visual page chosen on its
/// rail). `FRAME_VIEW=Edit Areas Together` picks them.
#[test]
#[ignore]
fn edit_areas_together_3000() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    const COUNT: usize = 3_000;
    let dir = mg_testkit::scratch_dir("frame-perf-u29");
    let path = dir.join("areas3000.mod");
    {
        use mg_gff::{Gff, Struct, Value};
        let mut m = mg_module::Module::new();
        let mut info = Gff::new(*b"IFO ");
        let mut list = Vec::new();
        for n in 0..COUNT {
            let name = format!("area{n:04}");
            let r = ResRef::from_str(&name).unwrap();
            let mut item = Struct::new(6);
            item.set("Area_Name", Value::resref(r));
            list.push(item);
            let mut are = Gff::new(*b"ARE ");
            let title = mg_core::LocString::from_text(
                mg_core::Language::ENGLISH,
                mg_core::Gender::Male,
                format!("Area {n}"),
            );
            are.root.set("Name", Value::LocString(title));
            are.root.set("ResRef", Value::resref(r));
            are.root.set("Tag", Value::String(name.to_uppercase().into_bytes()));
            are.root.set("Tileset", Value::resref(ResRef::from_str("ttr01").unwrap()));
            are.root.set("Width", Value::Int(1));
            are.root.set("Height", Value::Int(1));
            are.root.set("Flags", Value::Dword([0, 1, 3][n % 3]));
            are.root.set("DayNightCycle", Value::Byte(1));
            let mut tile = Struct::new(1);
            tile.set("Tile_ID", Value::Int(0));
            tile.set("Tile_Orientation", Value::Int(0));
            tile.set("Tile_Height", Value::Int(0));
            are.root.set("Tile_List", Value::List(vec![tile]));
            m.set(ResKey::new(r, ResType::ARE), are.to_bytes().unwrap());
            let mut git = Gff::new(*b"GIT ");
            git.root.set("AreaProperties", Value::Struct(Struct::new(100)));
            m.set(ResKey::new(r, ResType::GIT), git.to_bytes().unwrap());
        }
        info.root.set("Mod_Area_list", Value::List(list));
        m.set_info(&info).unwrap();
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    }
    let mut r = Report::new();
    println!("Edit Areas Together over {COUNT} areas: frames at {:?}", screen_size());
    let rs = render_state();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.open_module(&path);
    let mut h = harness(app, rs);
    h.run_steps(3);
    // (The areas the module opened with, closed: the views time the
    // section or the window, not an area's view beside them.)
    let shown: Vec<Tab> = (h.state().dock.iter_all_tabs())
        .map(|(_, t)| t.clone())
        .filter(|t| matches!(t, Tab::Area(_) | Tab::ModuleProperties))
        .collect();
    for t in &shown {
        close(&mut h, t);
    }
    menu(&mut h, "Edit", "Edit Areas Together…");
    h.run_steps(3);
    // (A row is named "Area 0 (area0000)" or, since U29, "area0000 (Area 0)".)
    let listed = h.query_all_by_label_contains("(area").count()
        + h.query_all_by_label_contains("(Area ").count();
    assert!(listed > 0, "no area rows: Edit Areas Together not shown");
    println!("  ({listed} of the {COUNT} area rows laid out)");
    // (The pointer moved and the wheel turned over the list's rows, as
    // they lie: the window's before U29, the Right Dock's section since.
    // Over the whole screen, the wheel was turned at its middle, which
    // since U29 is the empty editor area, not the list: the review of
    // U29.)
    let rows = rows_rect(&h);
    view_in(&mut h, &mut r, &format!("Edit Areas Together, {COUNT} areas listed"), rows, |_, _| {});
    click(&mut h, "Check Shown");
    h.run_steps(2);
    let rows = rows_rect(&h);
    view_in(&mut h, &mut r, &format!("Edit Areas Together, {COUNT} areas ticked"), rows, |_, _| {});
    // Their shared pages: the window (before U29), or the section's
    // Visual page (since).
    let together = format!("Edit {COUNT} Together");
    if h.query_by_label(&together).is_some() {
        click(&mut h, &together);
    } else {
        click(&mut h, "Visual");
    }
    h.run_steps(3);
    assert!(
        h.query_by_label_contains("Always Dark").is_some(),
        "not on the Visual page of the {COUNT} areas"
    );
    // (Over the page, round its Always Dark.)
    let page = h.query_by_label_contains("Always Dark").map(|n| {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
        n.rect().expand2(egui::vec2(100.0, 120.0)).intersect(screen)
    });
    view_in(
        &mut h,
        &mut r,
        &format!("Edit Areas Together, {COUNT} areas, Visual page"),
        page,
        |_, _| {},
    );
    r.end();
}

/// Where the list of Edit Areas Together's rows lies on the screen (the
/// rows laid out, in sight).
fn rows_rect(h: &App<'_>) -> Option<egui::Rect> {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
    h.query_all_by(|n| {
        n.label().is_some_and(|l| l.contains("(Area ") || l.contains("(area"))
            && n.bounding_box().is_some()
    })
    .map(|n| n.rect().intersect(screen))
    .filter(|r| r.is_positive())
    .reduce(|a, b| a.union(b))
}

/// The review of November 2026's area editor findings at the sizes they
/// name (A16, A17, A18's Update Instances, A20, A21, A22, A24: the deferred
/// list's "performance fixes without a timing view"), each against the
/// same view at rest, timed just before it: a 32 by 32 area of the Rural
/// tileset (ttr01) with 5,000 placeables, every one selected (the
/// selection's overlays), dragged, with Adjust Location open (a value
/// changed, so that the selection is posed anew each frame) and with the
/// context menu open, and 2,000 of them with the Position menu open; an
/// empty 32 by 32 area with Paint at radius 25 m and density 50 (the
/// pointer still: the next dab's ghosts), a light being placed and the
/// area's menu open (Add Light Here) with a placeables.2da of 30,000 more
/// rows (copies of the game's rows that are not lights, in the scratch
/// user folder's `override`); a 32 by 32 area of the Rural tileset's
/// hills, every tile selected (Select Tiles), still and turning; Update
/// Instances of the 30,000 placed in six areas; and the tileset editor's
/// Tiles page of a 2,000-tile set (the Rural tileset's tiles copied),
/// its first and its last tile chosen. Each line ends with the view's
/// interface time against its resting one's: over by more than
/// [`UI_LINE`] is a cost to find. `FRAME_VIEW=<part of a name>` picks them
/// ("5,000 placeables", "empty area", "hills", "Update Instances",
/// "2,000-tile").
#[test]
#[ignore]
fn area_editor_at_scale() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("frame-perf-area-scale");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    let (scale, light, set_path) = {
        let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
        let light = big_placeables(&game, &user);
        (scale_module(&game, &dir), light, big_tileset(&game, &dir))
    };
    let mut r = Report::new();
    let mut lines = Vec::new();
    println!("the area editor at scale: frames at {:?}", screen_size());
    let rs = render_state();
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.set_render_state(rs.clone());
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.open_module(&scale.path);
    let mut h = harness(app, rs);
    h.run_steps(3);
    let rows = h.state().game.as_deref().and_then(|g| g.table("placeables").ok()).map(|t| t.len());
    println!("  (placeables.2da: {rows:?} rows; light row {light})");
    close_areas(&mut h);
    // (A part of a group's name, or a view's whole name, which begins with it.)
    let wanted = |r: &Report, what: &str| {
        r.only.as_ref().is_none_or(|o| what.contains(o.as_str()) || o.starts_with(what))
    };

    // A16 and A20: the area of 5,000 placeables.
    let name = "area of 5,000 placeables";
    if wanted(&r, name) {
        let area = scale.placed;
        let rect = open_area(&mut h, area);
        let placeables: Vec<_> = h.state().area_views[&area]
            .model
            .as_ref()
            .map(|m| m.objects.iter().map(|o| (o.kind, o.index)).collect())
            .unwrap_or_default();
        println!("  ({} objects in {name})", placeables.len());
        let Some((one, grip)) = grip_near(&h, area, rect, |_| true) else {
            panic!("{name}: no placeable to hold in sight")
        };
        let select = |h: &mut App<'_>, keys: &[(mg_area::ObjectKind, usize)]| {
            h.state_mut().area_views.get_mut(&area).unwrap().selection = keys.to_vec();
            h.hover_at(grip);
            h.run_steps(3);
        };
        let one_key = placeables[one];
        // The noise floor: the resting view twice.
        select(&mut h, &[one_key]);
        let rest = timed(
            &mut h,
            &mut r,
            &format!("{name}, one selected"),
            "still",
            rect,
            Pointer::Rests(grip),
        );
        let again = timed(
            &mut h,
            &mut r,
            &format!("{name}, one selected, again"),
            "still",
            rect,
            Pointer::Rests(grip),
        );
        against(&mut lines, &format!("{name}, one selected, timed again (the noise)"), rest, again);
        select(&mut h, &placeables);
        let all = timed(
            &mut h,
            &mut r,
            &format!("{name}, all selected"),
            "still",
            rect,
            Pointer::Rests(grip),
        );
        against(&mut lines, &format!("{name}, all selected"), rest, all);
        let moving = timed(
            &mut h,
            &mut r,
            &format!("{name}, all selected"),
            "pointer",
            rect,
            Pointer::Moves,
        );
        select(&mut h, &[one_key]);
        let moving_one = timed(
            &mut h,
            &mut r,
            &format!("{name}, one selected"),
            "pointer",
            rect,
            Pointer::Moves,
        );
        against(
            &mut lines,
            &format!("{name}, all selected, the pointer moving"),
            moving_one,
            moving,
        );
        // Dragged: one, then all, 60 points east (the last frame lets go:
        // the move's command made).
        let undo =
            |h: &App<'_>| h.state().ws.as_ref().and_then(|w| w.can_undo().map(str::to_owned));
        let dragged = |h: &mut App<'_>, r: &mut Report, what: &str| {
            if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) || r.shots {
                return None;
            }
            let grip = h.state().area_views[&area].move_grip(one).expect("a grip");
            let to = grip + egui::vec2(60.0, 0.0);
            let f = frames(h, rect, Pointer::Drags(grip, to), r.count, |_, _| {});
            said(r, what, "drag", &f);
            h.run_steps(2);
            assert_eq!(undo(h).as_deref(), Some("Move"), "{what}: the drag moved them");
            h.state_mut().actions.push(Action::Undo);
            h.run_steps(3);
            Some(pick(f.ui, 0.5))
        };
        let one_drag = dragged(&mut h, &mut r, &format!("{name}, one selected, dragged"));
        select(&mut h, &placeables);
        let all_drag = dragged(&mut h, &mut r, &format!("{name}, all selected, dragged"));
        against(&mut lines, &format!("{name}, all selected, dragged"), one_drag, all_drag);
        // At rest again after the drag and its undo: what the next two
        // are set against (timed just before them, as the others are).
        h.hover_at(grip);
        h.run_steps(3);
        let all_again = timed(
            &mut h,
            &mut r,
            &format!("{name}, all selected, after the drag"),
            "still",
            rect,
            Pointer::Rests(grip),
        );
        // Adjust Location open on all of them, X changed (posed anew).
        let adjust = mg_ui::area_tools::adjust(&h.state().area_views[&area]).map(|mut a| {
            a.position.x += 3.0;
            a.changed[0] = true;
            a
        });
        assert!(adjust.is_some(), "{name}: no Adjust Location for the selection");
        h.state_mut().adjust = adjust;
        h.run_steps(3);
        let adjusting = timed(
            &mut h,
            &mut r,
            &format!("{name}, all selected, Adjust Location open"),
            "still",
            rect,
            Pointer::Rests(grip),
        );
        assert!(h.state().adjust.is_some(), "{name}: Adjust Location closed");
        against(
            &mut lines,
            &format!("{name}, all selected, Adjust Location open"),
            all_again,
            adjusting,
        );
        h.state_mut().adjust = None;
        h.run_steps(3);
        // The context menu open on them, the pointer on its first row.
        if let Some(at) = menu_on(&mut h, grip, "Adjust Location…") {
            let menu = timed(
                &mut h,
                &mut r,
                &format!("{name}, all selected, the context menu open"),
                "still",
                rect,
                Pointer::Rests(at),
            );
            h.run_steps(1);
            assert!(h.query_by_label("Adjust Location…").is_some(), "{name}: the menu closed");
            against(
                &mut lines,
                &format!("{name}, all selected, the context menu open"),
                all_again,
                menu,
            );
            close_menu(&mut h);
        }
        // And once more, the menu closed (the noise of the two).
        h.hover_at(grip);
        h.run_steps(3);
        let all_last = timed(
            &mut h,
            &mut r,
            &format!("{name}, all selected, the menu closed"),
            "still",
            rect,
            Pointer::Rests(grip),
        );
        against(
            &mut lines,
            &format!("{name}, all selected, timed again after the menu (the noise)"),
            all_again,
            all_last,
        );
        // A20: 2,000 of them, the Position menu open (the pointer on it).
        let two: Vec<_> = placeables.iter().copied().take(2_000).collect();
        let Some((_, grip2)) = grip_near(&h, area, rect, |i| i < 2_000) else {
            panic!("{name}: none of the 2,000 in sight")
        };
        h.state_mut().area_views.get_mut(&area).unwrap().selection = two.clone();
        h.hover_at(grip2);
        h.run_steps(3);
        let rest2 = timed(
            &mut h,
            &mut r,
            &format!("{name}, 2,000 selected"),
            "still",
            rect,
            Pointer::Rests(grip2),
        );
        if let Some(at) = menu_on(&mut h, grip2, "Position") {
            h.hover_at(at);
            h.run_steps(5);
            assert!(
                h.query_by_label_contains("Random Facing").is_some(),
                "{name}: no Position menu"
            );
            let menu = timed(
                &mut h,
                &mut r,
                &format!("{name}, 2,000 selected, the Position menu open"),
                "still",
                rect,
                Pointer::Rests(at),
            );
            h.run_steps(1);
            assert!(
                h.query_by_label_contains("Random Facing").is_some(),
                "{name}: Position closed"
            );
            against(
                &mut lines,
                &format!("{name}, 2,000 selected, the Position menu open"),
                rest2,
                menu,
            );
            close_menu(&mut h);
        }
        h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
        close(&mut h, &Tab::Area(area));
    }

    // A17 and A21: the empty area.
    let name = "empty area";
    if wanted(&r, name) {
        let area = scale.empty;
        let rect = open_area(&mut h, area);
        let at = rect.center();
        h.hover_at(at);
        h.run_steps(3);
        let rest = timed(
            &mut h,
            &mut r,
            &format!("{name}, the pointer still"),
            "still",
            rect,
            Pointer::Rests(at),
        );
        // Paint: radius 25 m, density 50 a tile, one placeable chosen.
        {
            let app = h.state_mut();
            app.palette.tiles = false;
            app.palette.kind = BlueprintKind::Placeable;
            app.palette.chosen = ResKey::parse("plc_tree", ResType::UTP).into_iter().collect();
            let p = &mut app.settings.paint;
            (p.row, p.fill, p.erase) = (false, false, false);
            (p.radius_cm, p.density) = (2_500, 50);
            app.paint = true;
        }
        h.run_steps(5);
        let painting = timed(
            &mut h,
            &mut r,
            &format!("{name}, Paint at 25 m and 50 a tile, the pointer still"),
            "still",
            rect,
            Pointer::Rests(at),
        );
        let paint = &h.state().area_views[&area].paint;
        println!("  (Paint's next dab: {} ghosts shown, {} more)", paint.shown, paint.more);
        assert!(paint.shown > 0, "{name}: Paint shows no next dab");
        against(
            &mut lines,
            &format!("{name}, Paint at 25 m and 50 a tile, the pointer still"),
            rest,
            painting,
        );
        {
            let app = h.state_mut();
            app.paint = false;
            app.palette.chosen.clear();
        }
        h.run_steps(3);
        // A light being placed (its status line asks for the lights).
        h.state_mut().palette.placing_light = Some(light);
        h.run_steps(3);
        let placing = timed(
            &mut h,
            &mut r,
            &format!("{name}, a light being placed, placeables.2da +30,000"),
            "still",
            rect,
            Pointer::Rests(at),
        );
        // (Still in hand, and the area's status line worked out: its
        // hints name the light from the table of 46,500.)
        assert_eq!(h.state().palette.placing_light, Some(light), "{name}: no light in hand");
        assert!(h.query_by_label("32 by 32 tiles").is_some(), "{name}: no status line");
        against(
            &mut lines,
            &format!("{name}, a light being placed, placeables.2da +30,000"),
            rest,
            placing,
        );
        h.state_mut().palette.placing_light = None;
        h.run_steps(3);
        // The area's menu (on the ground): Add Light Here.
        if let Some(on) = menu_on(&mut h, at, "Add Light Here") {
            let menu = timed(
                &mut h,
                &mut r,
                &format!("{name}, its menu open, placeables.2da +30,000"),
                "still",
                rect,
                Pointer::Rests(on),
            );
            against(
                &mut lines,
                &format!("{name}, its menu open, placeables.2da +30,000"),
                rest,
                menu,
            );
            close_menu(&mut h);
        }
        // The resting view once more, after the others (the noise).
        h.hover_at(at);
        h.run_steps(3);
        let again = timed(
            &mut h,
            &mut r,
            &format!("{name}, the pointer still, again"),
            "still",
            rect,
            Pointer::Rests(at),
        );
        against(&mut lines, &format!("{name}, timed again (the noise)"), rest, again);
        close(&mut h, &Tab::Area(area));
    }

    // A24: the hills, every tile selected.
    let name = "hills";
    if wanted(&r, name) {
        let area = scale.hills;
        println!("  ({} of the 1,024 tiles of the hills' area sloped)", scale.sloped);
        let rect = open_area(&mut h, area);
        let at = rect.center();
        {
            let v = h.state_mut().area_views.get_mut(&area).unwrap();
            v.selection.clear();
            v.tile_mode = true;
            v.tile_selection.clear();
        }
        h.state_mut().palette.tiles = true;
        h.hover_at(at);
        h.run_steps(3);
        let turn = |app: &mut Moonglow, _: usize| {
            if let Some(o) = app.area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
                o.yaw += 0.01;
            }
        };
        let rest = timed(
            &mut h,
            &mut r,
            &format!("{name}, Select Tiles on, none selected"),
            "still",
            rect,
            Pointer::Rests(at),
        );
        let rest_turning = timed_with(
            &mut h,
            &mut r,
            &format!("{name}, Select Tiles on, none selected"),
            "turning",
            rect,
            Pointer::Rests(at),
            turn,
        );
        h.state_mut().area_views.get_mut(&area).unwrap().tile_selection =
            (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).collect();
        h.run_steps(3);
        let all = timed(
            &mut h,
            &mut r,
            &format!("{name}, Select Tiles on, all 1,024 selected"),
            "still",
            rect,
            Pointer::Rests(at),
        );
        let all_turning = timed_with(
            &mut h,
            &mut r,
            &format!("{name}, Select Tiles on, all 1,024 selected"),
            "turning",
            rect,
            Pointer::Rests(at),
            turn,
        );
        against(&mut lines, &format!("{name}, every tile selected"), rest, all);
        against(
            &mut lines,
            &format!("{name}, every tile selected, turning"),
            rest_turning,
            all_turning,
        );
        {
            let v = h.state_mut().area_views.get_mut(&area).unwrap();
            v.tile_mode = false;
            v.tile_selection.clear();
        }
        h.state_mut().palette.tiles = false;
        close(&mut h, &Tab::Area(area));
    }

    // A18's rest: Update Instances of the 30,000, no area open.
    let name = "Update Instances";
    if wanted(&r, name) {
        close_areas(&mut h);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
        let rest = timed(
            &mut h,
            &mut r,
            "Update Instances' resting view (nothing open)",
            "pointer",
            screen,
            Pointer::Moves,
        );
        let key = ResKey::parse("plc_armoire", ResType::UTP).unwrap();
        h.state_mut().update_instances_of(vec![key]);
        h.run_steps(5);
        let listed = h.state().update_draft.as_ref().map_or(0, |d| d.objects.len());
        assert_eq!(listed, 30_000, "{name}: not the 30,000 listed");
        assert!(h.query_by_label("Update 30000").is_some(), "{name}: its foot not drawn");
        let rows = h
            .query_all_by(|n| n.label().is_some_and(|l| l.contains("(plc_armoire)")))
            .map(|n| n.rect())
            .reduce(|a, b| a.union(b))
            .unwrap_or(screen);
        let list = timed(
            &mut h,
            &mut r,
            &format!("{name}, 30,000 listed"),
            "pointer",
            rows,
            Pointer::Moves,
        );
        against(&mut lines, &format!("{name}, 30,000 listed"), rest, list);
        let _ = timed(
            &mut h,
            &mut r,
            &format!("{name}, 30,000 listed"),
            "wheel",
            rows,
            Pointer::Scrolls,
        );
        h.state_mut().update_draft = None;
        h.run_steps(3);
    }

    // A22: the 2,000-tile set's Tiles page.
    let name = "2,000-tile set";
    if wanted(&r, name) {
        mg_ui::tileset_view::open_path(h.state_mut(), set_path);
        h.run_steps(3);
        let id = h.state().tilesets.last().expect("the tileset open").id;
        let t = Tab::Tileset(id);
        let tiles = h
            .state()
            .tilesets
            .iter()
            .find(|d| d.id == id)
            .and_then(|d| d.tileset())
            .map_or(0, |s| s.tiles.len());
        assert_eq!(tiles, 2_000, "{name}: not 2,000 tiles");
        let choose = |h: &mut App<'_>, tile: usize| {
            let d = h.state_mut().tilesets.iter_mut().find(|d| d.id == id).unwrap();
            d.page = mg_ui::tileset_view::Page::Tiles;
            d.tile = Some(tile);
            h.run_steps(5);
        };
        let rect = rect_of(h.state(), &t).expect("the tileset's tab");
        choose(&mut h, 0);
        let first = timed(
            &mut h,
            &mut r,
            &format!("{name}, Tiles page, its first tile chosen"),
            "pointer",
            rect,
            Pointer::Moves,
        );
        choose(&mut h, 1_999);
        assert!(
            h.query_all_by_label_contains("tile 1999").next().is_some(),
            "{name}: tile 1999 not shown"
        );
        let last = timed(
            &mut h,
            &mut r,
            &format!("{name}, Tiles page, its last tile chosen"),
            "pointer",
            rect,
            Pointer::Moves,
        );
        against(&mut lines, &format!("{name}, Tiles page, its last tile chosen"), first, last);
        let _ = timed(
            &mut h,
            &mut r,
            &format!("{name}, Tiles page, its last tile chosen"),
            "idle",
            rect,
            Pointer::Away,
        );
        close(&mut h, &t);
    }
    println!(
        "against their resting views (the line {} µs):\n  {}",
        UI_LINE.as_micros(),
        lines.join("\n  ")
    );
    r.end();
}

/// How much more of the interface's time a view may take than the same
/// view at rest before its cost is one to find (a hundredth of the
/// budget, the review's line).
const UI_LINE: Duration = Duration::from_micros(70);

/// A view's frames timed one way (`how`) after a few to settle in, the
/// pointer doing `pointer` within `rect`: the median of the interface's
/// pass. `None` where `FRAME_VIEW` leaves it out.
fn timed(
    h: &mut App<'_>,
    r: &mut Report,
    what: &str,
    how: &str,
    rect: egui::Rect,
    pointer: Pointer,
) -> Option<Duration> {
    timed_with(h, r, what, how, rect, pointer, |_, _| {})
}

fn timed_with(
    h: &mut App<'_>,
    r: &mut Report,
    what: &str,
    how: &str,
    rect: egui::Rect,
    pointer: Pointer,
    mut each: impl FnMut(&mut Moonglow, usize),
) -> Option<Duration> {
    if r.only.as_ref().is_some_and(|only| !what.contains(only.as_str())) || r.shots {
        return None;
    }
    frames(h, rect, pointer, 4, &mut each);
    let f = frames(h, rect, pointer, r.count, &mut each);
    said(r, what, how, &f);
    Some(pick(f.ui, 0.5))
}

/// A line for a view's interface time against its resting view's: over
/// by more than [`UI_LINE`] is "find".
fn against(lines: &mut Vec<String>, what: &str, rest: Option<Duration>, it: Option<Duration>) {
    let (Some(rest), Some(it)) = (rest, it) else { return };
    let micros = |d: Duration| d.as_secs_f64() * 1e6;
    let by = micros(it) - micros(rest);
    let verdict = if by > micros(UI_LINE) { "find" } else { "within" };
    let line =
        format!("{what}: ui {:.3} ms, at rest {:.3} ms: {by:+.0} µs, {verdict}", ms(it), ms(rest));
    println!("  {line}");
    lines.push(line);
}

/// Closes every area's tab.
fn close_areas(h: &mut App<'_>) {
    let shown: Vec<Tab> = (h.state().dock.iter_all_tabs())
        .map(|(_, t)| t.clone())
        .filter(|t| matches!(t, Tab::Area(_)))
        .collect();
    for t in &shown {
        close(h, t);
    }
}

/// Opens an area and waits until it is drawn whole, its camera over its
/// middle from 120 m, looking down steeply: where its view is.
fn open_area(h: &mut App<'_>, area: ResRef) -> egui::Rect {
    open(h, Tab::Area(area));
    let begun = Instant::now();
    loop {
        h.run_steps(1);
        let app = h.state();
        let view = app.area_views.get(&area);
        let pending = app
            .viewport
            .as_ref()
            .map_or(0, |v| v.renderer.drawn.textures_pending + v.renderer.drawn.game_pending);
        if view.is_some_and(|v| v.model.is_some() && v.pictures > 0) && pending == 0 {
            break;
        }
        assert!(begun.elapsed() < Duration::from_secs(120), "area {area} not drawn");
    }
    let middle = (h.state().area_views[&area].model.as_ref())
        .map(|m| glam::Vec3::new(m.width as f32 * 5.0, m.height as f32 * 5.0, 0.0))
        .unwrap_or_default();
    if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
        (o.target, o.pitch, o.distance) = (middle, 0.9, 120.0);
    }
    h.run_steps(5);
    rect_of(h.state(), &Tab::Area(area)).expect("the area's tab")
}

/// An object `keep` takes (by its place in the model) nearest the view's
/// middle that a press there would move: its place and where to press.
fn grip_near(
    h: &App<'_>,
    area: ResRef,
    rect: egui::Rect,
    keep: impl Fn(usize) -> bool,
) -> Option<(usize, egui::Pos2)> {
    let v = h.state().area_views.get(&area)?;
    let m = v.model.as_ref()?;
    let mut near: Vec<(f32, usize)> = (m.objects.iter().enumerate())
        .filter(|(i, o)| keep(*i) && o.kind == mg_area::ObjectKind::Placeable)
        .filter_map(|(i, o)| Some((v.screen_pos(o.position)?.distance(rect.center()), i)))
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.iter().take(60).find_map(|&(_, i)| Some((i, v.move_grip(i)?)))
}

/// Opens the area's context menu with a right click at `at` and gives
/// where its row `row` is (`None`, said, where it is not there).
fn menu_on(h: &mut App<'_>, at: egui::Pos2, row: &str) -> Option<egui::Pos2> {
    h.hover_at(at);
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    let on = h.query_all_by_label_contains(row).next().map(|n| n.rect().center());
    if on.is_none() {
        println!("  (no {row:?} in the area's menu)");
    }
    let on = on?;
    h.hover_at(on);
    h.run_steps(3);
    Some(on)
}

fn close_menu(h: &mut App<'_>) {
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
}

/// The areas [`area_editor_at_scale`] times, in a module of its own.
struct Scale {
    path: std::path::PathBuf,
    /// 32 by 32, 5,000 placeables (`plc_armoire`).
    placed: ResRef,
    /// 32 by 32, nothing placed.
    empty: ResRef,
    /// 32 by 32 of the Rural tileset's hills.
    hills: ResRef,
    /// How many of the hills' tiles are sloped (their corners not all at
    /// one height).
    sloped: usize,
}

/// The module of [`Scale`]: its three areas and five more of 8 by 8 with
/// 5,000 placeables each (30,000 placed from `plc_armoire` in all).
fn scale_module(game: &mg_rules::GameData, dir: &std::path::Path) -> Scale {
    use mg_gff::{Gff, Value};
    use mg_module::ModuleLocation;
    use mg_module::new::{AreaSpec, add_area, new_module};
    let mut rng = fastrand::Rng::with_seed(16);
    let mut m = new_module(game, "Area editor at scale", &mut rng).unwrap();
    let ttr01 = ResRef::from_str("ttr01").unwrap();
    let spec = |name: &str, side: u32| AreaSpec {
        name: name.into(),
        tileset: ttr01,
        width: side,
        height: side,
    };
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let placed = add_area(&mut m, game, &spec("Placed", 32), &mut rng).unwrap();
    place(&mut m, game, &bp.root, placed, 5_000, 320.0, &mut rng);
    for n in 0..5 {
        let more = add_area(&mut m, game, &spec(&format!("More {n}"), 8), &mut rng).unwrap();
        place(&mut m, game, &bp.root, more, 5_000, 80.0, &mut rng);
    }
    let empty = add_area(&mut m, game, &spec("Empty", 32), &mut rng).unwrap();
    // The hills: two by two corners raised a step in every four by four,
    // over the tileset's default terrain.
    let hills = add_area(&mut m, game, &spec("Hills", 32), &mut rng).unwrap();
    let set = game.resman.get_named("ttr01", ResType::SET).unwrap();
    let set = mg_set::Tileset::parse(&set, game.codepage()).unwrap();
    let index = mg_tiles::TileIndex::new(&set);
    let mut lattice = mg_tiles::new_area(&index, &set, 32, 32).unwrap();
    let ground = index.terrain(&set.general.default).unwrap();
    let up = |v: u32| v % 4 == 1 || v % 4 == 2;
    for y in 1..32 {
        for x in 1..32 {
            let height = i32::from(up(x) && up(y));
            lattice.set_corner(x, y, mg_tiles::Corner { terrain: ground, height });
        }
    }
    let tiles = mg_tiles::fill(&index, &lattice, &mut rng).unwrap();
    let mut sloped = 0;
    for y in 0..32 {
        for x in 0..32 {
            let h = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .map(|(dx, dy)| lattice.corner(x + dx, y + dy).height);
            sloped += usize::from(h.iter().any(|v| *v != h[0]));
        }
    }
    let key = ResKey::new(hills, ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    for (t, p) in are.root.list_mut("Tile_List").unwrap().iter_mut().zip(&tiles) {
        t.set("Tile_ID", Value::Int(p.tile as i32));
        t.set("Tile_Orientation", Value::Int(i32::from(p.orientation)));
        t.set("Tile_Height", Value::Int(p.height));
    }
    m.set_gff(key, &are).unwrap();
    let path = dir.join("scale.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    Scale { path, placed, empty, hills, sloped }
}

/// Places `count` of blueprint `bp` in `area` at random within `span`
/// meters of its south-west corner.
fn place(
    m: &mut mg_module::Module,
    game: &mg_rules::GameData,
    bp: &mg_gff::Struct,
    area: ResRef,
    count: usize,
    span: f32,
    rng: &mut fastrand::Rng,
) {
    use mg_module::instances::{Placement, Placing, instance};
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let placed: Vec<_> = (0..count)
        .map(|_| {
            let at = Placement {
                position: [rng.f32() * span, rng.f32() * span, 0.0],
                rotation: rng.f32() * 360.0,
            };
            instance(&placing, ResType::UTP, bp, at, &[]).unwrap()
        })
        .collect();
    let key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&key).unwrap().unwrap();
    git.root.set("Placeable List", mg_gff::Value::List(placed));
    m.set_gff(key, &git).unwrap();
}

/// The game's placeables.2da with 30,000 more rows (its rows that are not
/// lights, copied in turn), in the scratch user folder's `override`: the
/// row of one of the game's lights ("Light, White" or the first).
fn big_placeables(game: &mg_rules::GameData, user: &std::path::Path) -> usize {
    let data = game.resman.get_named("placeables", ResType::TWODA).unwrap();
    let mut t = mg_2da::TwoDa::parse(&data, game.codepage()).unwrap();
    let is_light = |t: &mg_2da::TwoDa, row: usize| {
        t.get(row, "ModelName").is_some_and(|m| m.eq_ignore_ascii_case("dag_invisible"))
            && t.get_int(row, "LightColor").is_some_and(|c| c > 0)
    };
    let lights: Vec<usize> = (0..t.len()).filter(|&row| is_light(&t, row)).collect();
    let others: Vec<usize> = (0..t.len())
        .filter(|&row| !is_light(&t, row) && t.get(row, "ModelName").is_some())
        .collect();
    let columns = t.columns().len();
    for k in 0..30_000 {
        let from = others[k % others.len()];
        let row = t.add_row();
        for c in 0..columns {
            let value = t.cell(from, c).map(str::to_string);
            t.set(row, c, value).unwrap();
        }
    }
    std::fs::write(user.join("override/placeables.2da"), t.to_bytes(game.codepage()).unwrap())
        .unwrap();
    let white = lights.iter().copied().find(|&row| t.get(row, "Label") == Some("Light, White"));
    white.or(lights.first().copied()).expect("a light in placeables.2da")
}

/// The Rural tileset's `.set` with its tiles copied in turn to 2,000 (the
/// copies' sections after its groups, where a scan from the first line
/// goes farthest), in the scratch folder.
fn big_tileset(game: &mg_rules::GameData, dir: &std::path::Path) -> std::path::PathBuf {
    let data = game.resman.get_named("ttr01", ResType::SET).unwrap();
    let mut f = mg_set::edit::SetFile::parse(&String::from_utf8_lossy(&data));
    let n: usize = f
        .entries("TILES")
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Count"))
        .and_then(|(_, v)| v.trim().parse().ok())
        .expect("a tile count");
    for j in n..2_000 {
        let entries = f.entries(&format!("TILE{}", j % n));
        f.add_section(&format!("TILE{j}"), &entries);
    }
    f.set("TILES", "Count", "2000");
    let path = dir.join("set").join("ttr01.set");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, f.text()).unwrap();
    path
}

/// The review of November 2026's blueprint editor findings at the sizes
/// they name (B20, B21, B22's rest, B23 to B26, G4's half and the last
/// pass's item 17: the deferred list's "performance fixes without a timing
/// view"), each against the same view at a small size, timed just before
/// it, in a scratch copy of the persistent world (`world/mod.rs`: 300
/// areas, 8,000 custom items, the haks' 100,000 textures and 17,100-row
/// appearance.2da) with 100 creatures of 20 items each placed in its first
/// area: the Instances window of 2 and of 100 of them on their Inventory
/// and Feats pages (B20); a door's Area Transition page and Setup Area
/// Transition opened on it over the 300 areas, the click's frame and the
/// areas' objects the workspace holds after (B21); the Custom palette of a
/// kind with few blueprints and of the 8,000 items, List and Gallery,
/// Recent and Favorites filled from the standard items, a search typed,
/// and with the search an edit of an area made every frame (B23, B24); an
/// item blueprint's Properties page with no search, a search that finds
/// nothing (every choice tested) and one that finds some (G4); the Sound
/// and the Placeable Wizard's category pages (item 17); Select Texture
/// opened and searched (B26); a creature's Feats page, Select Portrait, an
/// item's Appearance page of icons and the Creature Wizard's appearance
/// page, with the install's tables and then, after Reload Resources, with
/// feat.2da of 20,000 rows, portraits.2da of 32,000, appearance.2da of
/// 30,000 and 999 icons of a simple item in the scratch user folder's
/// `development` (B22's rest, B26; the window with nothing open, timed on
/// both sides, tells a drift from a cost); and the Appearance Gallery of
/// creatures scrolled to its end, a long way a frame as its scroll bar
/// dragged, and the frames until the pictures in sight are made (B25).
/// The frames a click, a key or an opening lands in are held to the
/// budget. `FRAME_VIEW=<part of a name>` picks a group ("Instances",
/// "Setup Area Transition", "custom palette", "Properties page",
/// "Blueprint Wizard", "Select Texture", "tables", "Appearance Gallery").
#[test]
#[ignore]
fn blueprint_editors_at_scale() {
    use mg_edit::{Command, Edit, GffPath};
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let Some(w) = world::world(&root) else { return };
    let dir = mg_testkit::scratch_dir("frame-perf-blueprints");
    let user = dir.join("user");
    let dev = user.join("development");
    std::fs::create_dir_all(&dev).unwrap();
    link_dir(&w.user.join("hak"), &user.join("hak"));
    let made = {
        let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
        blueprints_module(&game, &w.module, &dir)
    };
    let mut r = Report::new();
    let mut lines = Vec::new();
    println!("the blueprint editors at scale: frames at {:?}", screen_size());
    let rs = render_state();
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user), "en")),
        Box::new(NoDialogs::default()),
    );
    app.set_render_state(rs.clone());
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.open_module(&made.path);
    let mut h = harness(app, rs);
    h.run_steps(3);
    // (Pictures read apart, as the desktop app reads them.)
    h.state_mut().background_jobs = true;
    fold_palette(&mut h);
    close(&mut h, &Tab::ModuleProperties);
    close_areas(&mut h);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
    let wanted = |r: &Report, what: &str| {
        r.only.as_ref().is_none_or(|o| what.contains(o.as_str()) || o.starts_with(what))
    };
    // The worst of the frames a click, a key or an opening lands in, the
    // pointer doing `pointer`: a line against the budget.
    let landed = |h: &mut App<'_>, lines: &mut Vec<String>, what: &str, pointer: Pointer| {
        let f = frames(h, screen, pointer, 4, |_, _| {});
        let worst = f.ui.iter().zip(&f.tessellate).map(|(a, b)| *a + *b).max().unwrap();
        let verdict = if worst > BUDGET { "over the budget" } else { "within the budget" };
        let line = format!("{what}: its worst frame {:.2} ms, {verdict}", ms(worst));
        println!("  {line}");
        lines.push(line);
        worst
    };

    // B20: the Instances window of 2 and of 100 creatures.
    let name = "Instances of creatures";
    if wanted(&r, name) {
        let git = ResKey::new(made.area, ResType::GIT);
        let first = made.creatures[0].clone();
        for page in ["Inventory", "Feats"] {
            let mut took = Vec::new();
            // (The two timed again after the hundred: the noise of the run.)
            for (i, n) in [2, 100, 2].into_iter().enumerate() {
                let t = Tab::Instances {
                    objects: made.creatures[..n].iter().map(|p| (made.area, p.clone())).collect(),
                };
                h.state_mut().blueprint_pages.insert((git, first.clone()), page);
                open(&mut h, t.clone());
                h.run_steps(5);
                let rect = rect_of(h.state(), &t).expect("the Instances window");
                let again = if i == 2 { ", again" } else { "" };
                let what = format!("{name}, {n} on the {page} page{again}");
                took.push(timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves));
                assert!(h.state().dock.find_tab(&t).is_some(), "{what}: the window closed");
                close(&mut h, &t);
            }
            against(&mut lines, &format!("{name}, 100 on the {page} page"), took[0], took[1]);
            let noise = format!("{name}, 2 on the {page} page timed again (the noise)");
            against(&mut lines, &noise, took[0], took[2]);
        }
    }

    // B21: Setup Area Transition over the 300 areas.
    let name = "Setup Area Transition";
    if wanted(&r, name) {
        let t = Tab::Blueprint(made.door);
        h.state_mut().blueprint_pages.insert((made.door, GffPath::root()), "Area Transition");
        open(&mut h, t.clone());
        h.run_steps(5);
        let held = |h: &App<'_>| {
            let ws = h.state().ws.as_ref().unwrap();
            ws.module.keys_of(ResType::GIT).filter(|k| ws.loaded(k).is_some()).count()
        };
        let areas = h.state().ws.as_ref().unwrap().module.keys_of(ResType::GIT).count();
        let before = held(&h);
        let rest = timed(
            &mut h,
            &mut r,
            &format!("{name}'s door, its Area Transition page"),
            "pointer",
            screen,
            Pointer::Moves,
        );
        let button = h.get_by_label("Setup Area Transition…").rect().center();
        let mut clicks = Vec::new();
        for n in 0..5 {
            let what = format!("{name}, the click that opens it over {areas} areas ({})", n + 1);
            clicks.push(landed(&mut h, &mut lines, &what, Pointer::Clicks(button)));
            h.run_steps(3);
            assert!(h.query_by_label("Target area").is_some(), "{name}: the window not open");
            if n < 4 {
                close_menu(&mut h);
                assert!(h.query_by_label("Target area").is_none(), "{name}: the window not closed");
            }
        }
        let after = held(&h);
        let line = format!(
            "{name}: the areas' objects held by the workspace: {before} before, {after} after \
             five openings"
        );
        println!("  {line}");
        lines.push(line);
        let open_ = timed(
            &mut h,
            &mut r,
            &format!("{name} open over {areas} areas"),
            "pointer",
            screen,
            Pointer::Moves,
        );
        assert!(h.query_by_label("Target area").is_some(), "{name}: the window closed");
        against(&mut lines, &format!("{name} open over {areas} areas"), rest, open_);
        close_menu(&mut h);
        close(&mut h, &t);
    }

    // B23 and B24: the Custom palette, few blueprints and 8,000.
    let name = "custom palette";
    if wanted(&r, name) {
        let standard: Vec<String> = {
            let game = h.state().game.as_deref().unwrap();
            let mut items = game.resman.list(ResType::UTI);
            items.sort();
            items
                .iter()
                .filter(|r| r.to_string().starts_with("nw_"))
                .take(20)
                .map(|r| format!("uti:{r}"))
                .collect()
        };
        assert_eq!(standard.len(), 20, "{name}: twenty standard items");
        h.state_mut().settings.palette_recent = standard.clone();
        h.state_mut().settings.palette_favorites = standard[..10].to_vec();
        let count = |h: &App<'_>, kind: BlueprintKind| {
            h.state().ws.as_ref().unwrap().module.keys_of(kind.restype()).count()
        };
        let few = [BlueprintKind::Waypoint, BlueprintKind::Sound, BlueprintKind::Trigger]
            .into_iter()
            .filter(|k| count(&h, *k) > 0)
            .min_by_key(|k| count(&h, *k))
            .expect("a kind with a few custom blueprints");
        let kinds = [(few, count(&h, few)), (BlueprintKind::Item, count(&h, BlueprintKind::Item))];
        let palcus = |h: &App<'_>, kind: BlueprintKind| {
            let m = &h.state().ws.as_ref().unwrap().module;
            m.contains(&kind.custom_key())
        };
        println!(
            "  ({name}: {} {:?} blueprints, its palcus {}; {} items, theirs {})",
            kinds[0].1,
            few,
            palcus(&h, few),
            kinds[1].1,
            palcus(&h, BlueprintKind::Item)
        );
        let area = ResKey::new(made.area, ResType::ARE);
        let edit = |app: &mut Moonglow, n: usize| {
            let edit = Edit::SetField {
                key: area,
                path: GffPath::root(),
                label: "Comments".into(),
                value: Some(mg_gff::Value::String(format!("frame {n}").into_bytes())),
            };
            app.actions.push(Action::Apply(Command::new("Edit the area's comment", vec![edit])));
        };
        // Unsearched, a kind of few blueprints against the 8,000 items;
        // searched, the items with a search that finds 100 against one
        // that finds them all (as many rows drawn).
        let items = kinds[1];
        for (gallery, searched) in [(false, false), (true, false), (false, true), (true, true)] {
            h.state_mut().settings.palette_gallery = gallery;
            h.state_mut().settings.gallery_tile = gallery.then_some(64);
            let shown = if gallery { "Gallery" } else { "List" };
            let pair = match searched {
                false => [(kinds[0], ""), (items, "")],
                true => [(items, "pw_i000"), (items, "pw")],
            };
            let mut took = Vec::new();
            let mut edited = Vec::new();
            let mut whats = Vec::new();
            for ((kind, n), search) in pair {
                palette(&mut h, kind, true, search);
                show_palette(&mut h);
                h.run_steps(if gallery { 300 } else { 5 });
                let rect = palette_rect(&h).expect("the palette");
                let what = match search {
                    "" => format!("{name} of {n} {}, {shown}", kind.label().to_lowercase()),
                    s => format!("{name} of {n} items, {shown}, \"{s}\" searched"),
                };
                took.push(timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves));
                assert_eq!(h.state().palette.filter, search, "{what}: the search kept");
                if search == "pw_i000" {
                    assert!(h.query_by_label("100 items").is_some(), "{what}: not 100 found");
                }
                if searched {
                    edited.push(timed_with(
                        &mut h,
                        &mut r,
                        &format!("{what}, an edit each frame"),
                        "pointer",
                        rect,
                        Pointer::Moves,
                        edit,
                    ));
                }
                fold_palette(&mut h);
                whats.push(what);
            }
            against(&mut lines, &whats[1], took[0], took[1]);
            if let [small, large] = edited[..] {
                against(&mut lines, &format!("{}, an edit each frame", whats[1]), small, large);
            }
        }
        h.state_mut().settings.palette_gallery = false;
        h.state_mut().settings.gallery_tile = None;
        h.state_mut().settings.palette_recent.clear();
        h.state_mut().settings.palette_favorites.clear();
        h.state_mut().palette.filter.clear();
        // (The edits taken back.)
        while h.state().ws.as_ref().and_then(|w| w.can_undo()) == Some("Edit the area's comment") {
            h.state_mut().actions.push(Action::Undo);
            h.run_steps(1);
        }
    }

    // G4's half: an item's Properties page with a search.
    let name = "item blueprint's Properties page";
    if wanted(&r, name) {
        let item = ResKey::parse("pw_i00001", ResType::UTI).unwrap();
        let t = Tab::Blueprint(item);
        h.state_mut().blueprint_pages.insert((item, GffPath::root()), "Properties");
        open(&mut h, t.clone());
        h.run_steps(5);
        let rect = rect_of(h.state(), &t).expect("the item's tab");
        let rest =
            timed(&mut h, &mut r, &format!("{name}, no search"), "pointer", rect, Pointer::Moves);
        let field = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::TextInput
                && (n.placeholder() == Some("Search properties")
                    || n.value().as_deref() == Some("zzzz"))
        };
        for (text, what) in [("zzzz", "a search that finds nothing"), ("fire", "\"fire\" searched")]
        {
            h.get(egui_kittest::kittest::by().predicate(field)).click();
            h.run_steps(2);
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
            h.run_steps(1);
            h.event(egui::Event::Text(text.into()));
            h.run_steps(3);
            let what = format!("{name}, {what}");
            let it = timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves);
            assert!(h.query_all_by_value(text).next().is_some(), "{what}: the search kept");
            let none = h.query_by_label("No property has that.").is_some();
            assert_eq!(none, text == "zzzz", "{what}: found what it should");
            against(&mut lines, &what, rest, it);
        }
        close(&mut h, &t);
    }

    // The last pass's item 17: the Blueprint Wizard's category page.
    let name = "Blueprint Wizard's category page";
    if wanted(&r, name) {
        let mut took = Vec::new();
        for (kind, wizard) in
            [(BlueprintKind::Sound, "Sound"), (BlueprintKind::Placeable, "Placeable")]
        {
            h.state_mut().blueprint_wizard =
                Some(mg_ui::blueprint_wizard::BlueprintWizard::new(kind));
            h.run_steps(5);
            let what = format!("{name}, the {wizard} Wizard");
            took.push(timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves));
            assert!(
                h.query_all_by_label_contains("Choose the palette category").next().is_some(),
                "{what}: not its category page"
            );
        }
        against(&mut lines, &format!("{name}, the Placeable Wizard"), took[0], took[1]);
        h.state_mut().blueprint_wizard = None;
        h.run_steps(3);
    }

    // B26: Select Texture over the haks' textures, opened and searched.
    let name = "Select Texture";
    if wanted(&r, name) {
        let chest = view_game_blueprint(&mut h, "plc_chest1", ResType::UTP);
        let textures = {
            let game = h.state().game.as_deref().unwrap();
            game.resman.list(ResType::TGA).len() + game.resman.list(ResType::DDS).len()
        };
        let t = Tab::Blueprint(chest);
        h.state_mut().blueprint_pages.insert((chest, GffPath::root()), "Visuals");
        open(&mut h, t.clone());
        h.run_steps(10);
        let button = h.get_all_by_label("Select Texture…").next().unwrap().rect().center();
        landed(
            &mut h,
            &mut lines,
            &format!("{name}, the click that opens it ({textures} textures)"),
            Pointer::Clicks(button),
        );
        h.run_steps(300);
        let find = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::TextInput && n.placeholder() == Some("Find")
        };
        assert!(h.query_by_label("Select Texture").is_some(), "{name}: not open");
        let open_ = timed(
            &mut h,
            &mut r,
            &format!("{name} open ({textures} textures)"),
            "pointer",
            screen,
            Pointer::Moves,
        );
        h.query_all(egui_kittest::kittest::by().predicate(find)).last().unwrap().click();
        h.run_steps(2);
        landed(&mut h, &mut lines, &format!("{name}, \"wood\" typed"), Pointer::Types("wood"));
        h.run_steps(300);
        let what = format!("{name}, \"wood\" searched ({textures} textures)");
        let searched = timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves);
        assert!(h.query_all_by_value("wood").next().is_some(), "{what}: the search kept");
        against(&mut lines, &what, open_, searched);
        h.get_all_by_label("Cancel").last().unwrap().click();
        h.run_steps(3);
        close(&mut h, &t);
    }

    // B22's rest and B26: the tables' views, with the install's tables
    // and grown.
    let name = "tables";
    if wanted(&r, name) {
        let small = tables_views(&mut h, &mut r, &mut lines, &made, "the install's tables");
        let sizes = grow_tables(&h, &dev);
        h.state_mut().actions.push(Action::ReloadResources);
        let begun = Instant::now();
        loop {
            h.run_steps(1);
            let game = h.state().game.as_deref().unwrap();
            if game.table("feat").is_ok_and(|t| t.len() == sizes.feats) {
                break;
            }
            assert!(begun.elapsed() < Duration::from_secs(60), "{name}: the reload not taken");
        }
        h.run_steps(5);
        {
            let game = h.state().game.as_deref().unwrap();
            let len = |t: &str| game.table(t).map_or(0, |t| t.len());
            assert_eq!(len("portraits"), sizes.portraits, "{name}: portraits.2da not grown");
            assert_eq!(len("appearance"), sizes.appearance, "{name}: appearance.2da not grown");
            assert_eq!(icons_of(game, &made.icon_prefix), 999, "{name}: not 999 icons");
        }
        let large = tables_views(&mut h, &mut r, &mut lines, &made, "grown tables");
        for (what, a, b) in [
            ("the window with nothing open (the control)", small.control, large.control),
            ("a creature's Feats page", small.feats, large.feats),
            ("Select Portrait", small.portrait, large.portrait),
            ("an item's Appearance page of icons", small.icons, large.icons),
            ("the Creature Wizard's appearance page", small.wizard, large.wizard),
        ] {
            against(&mut lines, &format!("{name}, {what}, grown"), a, b);
        }
        against(
            &mut lines,
            &format!("{name}, the control timed again (the noise)"),
            large.control,
            large.control_again,
        );
    }

    // B25: the Appearance Gallery scrolled to its end.
    let name = "Appearance Gallery";
    if wanted(&r, name) {
        let rows = h.state().game.as_deref().unwrap().table("appearance").map_or(0, |t| t.len());
        let mut gallery = mg_ui::appearance_gallery::Gallery::default();
        gallery.kind = mg_ui::appearance_gallery::Kind::Creature;
        h.state_mut().placeable_gallery = Some(gallery);
        let t = Tab::PlaceableGallery;
        open(&mut h, t.clone());
        let rect = rect_of(h.state(), &t).expect("the gallery's tab");
        let at = rect.center();
        h.hover_at(at);
        let made_all = |h: &mut App<'_>| {
            let (begun, mut frames_, mut ui, mut worst) =
                (Instant::now(), 0, Duration::ZERO, Duration::ZERO);
            loop {
                let f = frames(h, rect, Pointer::Rests(at), 1, |_, _| {});
                frames_ += 1;
                ui += f.ui[0];
                worst = worst.max(f.ui[0]);
                let th = &h.state().thumbnails;
                if th.waiting() == 0 && th.reading() == 0 && frames_ > 8 {
                    break (frames_, begun.elapsed(), ui, worst);
                }
                assert!(begun.elapsed() < Duration::from_secs(300), "{name}: pictures not made");
            }
        };
        made_all(&mut h);
        let what = format!("{name} of creatures ({rows} rows)");
        let rest = timed(
            &mut h,
            &mut r,
            &format!("{what}, at its top"),
            "still",
            rect,
            Pointer::Rests(at),
        );
        // (Down a long way a frame, as the scroll bar dragged to the end:
        // farther in all than the rows reach, at 150 points a row of
        // pictures, so the last frames rest at the end.)
        let across = (rect.width() / 150.0).floor().max(1.0);
        let by = 6_000.0;
        let count = ((rows as f32 / across * 150.0 / by).ceil() as usize + 20).min(2_000);
        let f = frames(&mut h, rect, Pointer::Down(at, by), count, |_, _| {});
        said(&mut r, &format!("{what}, scrolled to its end"), "wheel", &f);
        let worst = f.ui.iter().max().copied().unwrap_or_default();
        let (waiting, reading) = (h.state().thumbnails.waiting(), h.state().thumbnails.reading());
        let (frames_, took, ui, slowest) = made_all(&mut h);
        let line = format!(
            "{what}: scrolled to its end in {count} frames (their worst {:.2} ms), then {waiting} \
             pictures waiting and {reading} being read; the pictures in sight made in {frames_} \
             frames, {:.0} ms ({:.2} ms of the interface's, the slowest frame {:.2} ms)",
            ms(worst),
            ms(took),
            ms(ui),
            ms(slowest),
        );
        println!("  {line}");
        lines.push(line);
        let end = timed(
            &mut h,
            &mut r,
            &format!("{what}, at its end"),
            "still",
            rect,
            Pointer::Rests(at),
        );
        against(&mut lines, &format!("{what}, at its end"), rest, end);
        close(&mut h, &t);
        h.state_mut().placeable_gallery = None;
    }
    println!(
        "against their resting views (the line {} µs):\n  {}",
        UI_LINE.as_micros(),
        lines.join("\n  ")
    );
    r.end();
}

/// What [`tables_views`] timed.
#[derive(Default)]
struct TablesTimed {
    control: Option<Duration>,
    control_again: Option<Duration>,
    feats: Option<Duration>,
    portrait: Option<Duration>,
    icons: Option<Duration>,
    wizard: Option<Duration>,
}

/// The views whose cost grew with a table, timed with the tables `when`
/// says: a creature's Feats page, Select Portrait (and the click's frame),
/// an item's Appearance page of icons (and its opening's frames), the
/// Creature Wizard's appearance page; and the window with nothing open
/// before and after them (the control).
fn tables_views(
    h: &mut App<'_>,
    r: &mut Report,
    lines: &mut Vec<String>,
    made: &Blueprints,
    when: &str,
) -> TablesTimed {
    use mg_edit::GffPath;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
    let mut out = TablesTimed::default();
    let lens = {
        let game = h.state().game.as_deref().unwrap();
        let len = |t: &str| game.table(t).map_or(0, |t| t.len());
        format!(
            "feat.2da {}, portraits.2da {}, appearance.2da {}, {} icons",
            len("feat"),
            len("portraits"),
            len("appearance"),
            icons_of(game, &made.icon_prefix)
        )
    };
    println!("  ({when}: {lens})");
    let what = |view: &str| format!("tables, {view}, {when}");
    out.control = timed(h, r, &what("nothing open"), "pointer", screen, Pointer::Moves);
    // A creature's Feats page.
    let creature = ResKey::parse("pw_c00001", ResType::UTC).unwrap();
    let t = Tab::Blueprint(creature);
    h.state_mut().blueprint_pages.insert((creature, GffPath::root()), "Feats");
    open(h, t.clone());
    h.run_steps(5);
    let rect = rect_of(h.state(), &t).expect("the creature's tab");
    out.feats = timed(h, r, &what("a creature's Feats page"), "pointer", rect, Pointer::Moves);
    close(h, &t);
    // Select Portrait, from a placeable's Advanced page.
    let chest = view_game_blueprint(h, "plc_chest1", ResType::UTP);
    let t = Tab::Blueprint(chest);
    h.state_mut().blueprint_pages.insert((chest, GffPath::root()), "Advanced");
    open(h, t.clone());
    h.run_steps(5);
    h.get_by_label("Portraits…").scroll_to_me();
    h.run_steps(2);
    let button = h.get_by_label("Portraits…").rect().center();
    let f = frames(h, screen, Pointer::Clicks(button), 4, |_, _| {});
    let worst = f.ui.iter().zip(&f.tessellate).map(|(a, b)| *a + *b).max().unwrap();
    let verdict = if worst > BUDGET { "over the budget" } else { "within the budget" };
    let line = format!(
        "{}: its worst frame {:.2} ms, {verdict}",
        what("the click that opens Select Portrait"),
        ms(worst)
    );
    println!("  {line}");
    lines.push(line);
    h.run_steps(300);
    assert!(h.query_by_label("Plot Characters").is_some(), "Select Portrait not open ({when})");
    out.portrait = timed(h, r, &what("Select Portrait"), "pointer", screen, Pointer::Moves);
    close_menu(h);
    close(h, &t);
    // An item's Appearance page of icons, opened.
    let t = Tab::Blueprint(made.icons);
    h.state_mut().blueprint_pages.insert((made.icons, GffPath::root()), "Appearance");
    h.state_mut().actions.push(Action::OpenTab(t.clone()));
    let f = frames(h, screen, Pointer::Away, 4, |_, _| {});
    let worst = f.ui.iter().zip(&f.tessellate).map(|(a, b)| *a + *b).max().unwrap();
    let verdict = if worst > BUDGET { "over the budget" } else { "within the budget" };
    let line = format!(
        "{}: its worst frame {:.2} ms, {verdict}",
        what("an item's Appearance page of icons, opened"),
        ms(worst)
    );
    println!("  {line}");
    lines.push(line);
    h.run_steps(300);
    let rect = rect_of(h.state(), &t).expect("the item's tab");
    out.icons =
        timed(h, r, &what("an item's Appearance page of icons"), "pointer", rect, Pointer::Moves);
    assert!(h.query_all_by_label_contains("Appearance 1").next().is_some(), "no icons ({when})");
    close(h, &t);
    // The Creature Wizard's appearance page.
    h.state_mut().creature_wizard = Some(mg_ui::creature_wizard::CreatureWizard {
        page: 3,
        race: Some(6),
        classes: vec![(4, 4)],
        chosen_class: Some(4),
        portrait: Some(1),
        first_name: "Perf".into(),
        category: Some(0),
        ..Default::default()
    });
    h.run_steps(300);
    out.wizard = timed(
        h,
        r,
        &what("the Creature Wizard's appearance page"),
        "pointer",
        screen,
        Pointer::Moves,
    );
    h.state_mut().creature_wizard = None;
    h.run_steps(3);
    out.control_again =
        timed(h, r, &what("nothing open, again"), "pointer", screen, Pointer::Moves);
    out
}

/// What [`blueprints_module`] made.
struct Blueprints {
    path: std::path::PathBuf,
    /// The area of the 100 creatures.
    area: ResRef,
    /// Where they are in its GIT.
    creatures: Vec<mg_edit::GffPath>,
    /// A door blueprint (its Area Transition page).
    door: ResKey,
    /// A simple item of the base item [`Blueprints::icon_prefix`] names.
    icons: ResKey,
    /// Its icons' names before their number (`iit_midmisc_`).
    icon_prefix: String,
}

/// A copy of the persistent world's module with 100 creatures (the game's
/// `nw_bandit001`, each holding 20 of the game's items) placed in its first
/// area, a door blueprint and a simple item of the misc medium base item
/// (`mg_door`, `mg_icons`), in `dir`.
fn blueprints_module(
    game: &mg_rules::GameData,
    world: &std::path::Path,
    dir: &std::path::Path,
) -> Blueprints {
    use mg_gff::{Gff, Struct, Value};
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::{Module, ModuleLocation};
    let mut m = Module::open(world).unwrap();
    let area = m.areas().unwrap()[0];
    let read =
        |name: &str, t: ResType| Gff::read(&game.resman.get_named(name, t).unwrap()).unwrap();
    // The creatures, each with 20 kinds of item.
    let mut items = game.resman.list(ResType::UTI);
    items.sort();
    let items: Vec<ResRef> =
        items.into_iter().filter(|r| r.to_string().starts_with("nw_")).take(20).collect();
    let mut bandit = read("nw_bandit001", ResType::UTC).root;
    let held: Vec<Struct> = items
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut e = Struct::new(i as u32);
            e.set("InventoryRes", Value::resref(*r));
            e.set("Repos_PosX", Value::Word((i % 10) as u16));
            e.set("Repos_Posy", Value::Word((i / 10) as u16));
            e
        })
        .collect();
    bandit.set("ItemList", Value::List(held));
    let item = |r: ResRef| {
        game.resman
            .get(&ResKey::new(r, ResType::UTI))
            .ok()
            .and_then(|d| Gff::read(&d).ok())
            .map(|g| g.root)
    };
    let placing = Placing { game, item: &item };
    let mut rng = fastrand::Rng::with_seed(20);
    let key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&key).unwrap().unwrap();
    let mut list = git.root.list("Creature List").unwrap_or(&[]).to_vec();
    let from = list.len();
    for _ in 0..100 {
        let at = Placement { position: [rng.f32() * 20.0, rng.f32() * 20.0, 0.0], rotation: 0.0 };
        list.push(instance(&placing, ResType::UTC, &bandit, at, &[]).unwrap());
    }
    assert!(list[from].list("ItemList").is_some_and(|l| l.len() == 20), "the creatures' items");
    git.root.set("Creature List", Value::List(list));
    m.set_gff(key, &git).unwrap();
    let creatures =
        (from..from + 100).map(|i| mg_edit::GffPath::root().item("Creature List", i)).collect();
    // A door blueprint.
    let door_src = game.resman.list(ResType::UTD)[0];
    let door = ResKey::parse("mg_door", ResType::UTD).unwrap();
    let mut d = Gff::read(&game.resman.get(&ResKey::new(door_src, ResType::UTD)).unwrap()).unwrap();
    d.root.set("TemplateResRef", Value::resref(door.resref));
    m.set_gff(door, &d).unwrap();
    // A simple misc medium item.
    let base = game.table("baseitems").unwrap();
    let row = (0..base.len())
        .find(|&r| {
            base.get(r, "ItemClass").is_some_and(|c| c.eq_ignore_ascii_case("it_midmisc"))
                && base.get_int(r, "ModelType") == Some(0)
        })
        .expect("a misc medium base item");
    let icons = ResKey::parse("mg_icons", ResType::UTI).unwrap();
    let mut i = read(&items[0].to_string(), ResType::UTI);
    i.root.set("TemplateResRef", Value::resref(icons.resref));
    i.root.set("BaseItem", Value::Int(row as i32));
    i.root.set("ModelPart1", Value::Byte(1));
    m.set_gff(icons, &i).unwrap();
    let path = dir.join("blueprints.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    Blueprints { path, area, creatures, door, icons, icon_prefix: "iit_midmisc_".into() }
}

/// The sizes [`grow_tables`] grew the tables to.
struct Grown {
    feats: usize,
    portraits: usize,
    appearance: usize,
}

/// feat.2da of 20,000 rows, portraits.2da of 32,000 and appearance.2da of
/// 30,000 (the game data's as the window has them, their rows copied in
/// turn), and an icon for every number to 999 of the simple item's, in the
/// scratch user folder's `development` (`dev`), over the haks.
fn grow_tables(h: &App<'_>, dev: &std::path::Path) -> Grown {
    let game = h.state().game.as_deref().unwrap();
    let grown = Grown { feats: 20_000, portraits: 32_000, appearance: 30_000 };
    for (name, rows) in
        [("feat", grown.feats), ("portraits", grown.portraits), ("appearance", grown.appearance)]
    {
        let mut t = (*game.table(name).unwrap()).clone();
        let (had, columns) = (t.len(), t.columns().len());
        for k in had..rows {
            let from = k % had;
            let row = t.add_row();
            for c in 0..columns {
                let value = t.cell(from, c).map(str::to_string);
                t.set(row, c, value).unwrap();
            }
        }
        std::fs::write(dev.join(format!("{name}.2da")), t.to_bytes(game.codepage()).unwrap())
            .unwrap();
    }
    // (A 32 by 32 picture, its color from the number.)
    for n in 1..=999u32 {
        let name = format!("iit_midmisc_{n:03}");
        if game.resman.get_named(&name, ResType::TGA).is_ok()
            || game.resman.get_named(&name, ResType::DDS).is_ok()
        {
            continue;
        }
        let mut v = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 0, 32, 0, 32, 8];
        for _ in 0..32 * 32 {
            v.extend_from_slice(&[(n % 251) as u8, (n / 251 * 60) as u8, 128, 255]);
        }
        std::fs::write(dev.join(format!("{name}.tga")), v).unwrap();
    }
    grown
}

/// How many icons the game data has of `prefix` (TGA, DDS or PLT), each
/// number once.
fn icons_of(game: &mg_rules::GameData, prefix: &str) -> usize {
    let mut n: Vec<String> = [ResType::TGA, ResType::DDS, ResType::PLT]
        .iter()
        .flat_map(|t| game.resman.list(*t))
        .map(|r| r.to_string())
        .filter(|r| {
            r.strip_prefix(prefix)
                .is_some_and(|d| d.len() == 3 && d.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    n.sort();
    n.dedup();
    n.len()
}

/// A blueprint of the game's given to the window to look at (never the
/// module's: `Workspace::view`): its key.
fn view_game_blueprint(h: &mut App<'_>, name: &str, t: ResType) -> ResKey {
    let key = ResKey::parse(name, t).unwrap();
    let gff = {
        let game = h.state().game.as_deref().expect("the game's data");
        mg_gff::Gff::read(&game.resman.get(&key).expect("the game's blueprint")).unwrap()
    };
    h.state_mut().ws.as_mut().expect("the module").view(key, gff);
    key
}

/// `to` made to show `from` (a link where the system has them, else a
/// copy), read-only use: the world's haks in a scratch user folder.
fn link_dir(from: &std::path::Path, to: &std::path::Path) {
    let _ = std::fs::remove_file(to);
    let _ = std::fs::remove_dir_all(to);
    #[cfg(unix)]
    std::os::unix::fs::symlink(from, to).unwrap();
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
        }
    }
}

/// The worst of the first `count` frames a click, a key or an opening
/// lands in, the pointer doing `pointer`: a line against the budget.
fn landed(
    h: &mut App<'_>,
    lines: &mut Vec<String>,
    what: &str,
    pointer: Pointer,
    count: usize,
) -> Duration {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
    let f = frames(h, screen, pointer, count, |_, _| {});
    let worst = f.ui.iter().zip(&f.tessellate).map(|(a, b)| *a + *b).max().unwrap();
    let verdict = if worst > BUDGET { "over the budget" } else { "within the budget" };
    let line = format!("{what}: its worst frame {:.2} ms, {verdict}", ms(worst));
    println!("  {line}");
    lines.push(line);
    worst
}

/// [`against`] for a view whose rest is in another window (a module of
/// another size open): each taken over its own window's control (the
/// window with nothing open), so that what the two modules cost apart
/// from the view is left out. `rest` and `it` are (the view, the control).
fn against_controlled(
    lines: &mut Vec<String>,
    what: &str,
    rest: (Option<Duration>, Option<Duration>),
    it: (Option<Duration>, Option<Duration>),
) {
    let (Some(rest), Some(rest_control), Some(it), Some(it_control)) = (rest.0, rest.1, it.0, it.1)
    else {
        return;
    };
    let micros = |d: Duration| d.as_secs_f64() * 1e6;
    let by = (micros(it) - micros(it_control)) - (micros(rest) - micros(rest_control));
    let verdict = if by > micros(UI_LINE) { "find" } else { "within" };
    let line = format!(
        "{what}: ui {:.3} ms over its control {:.3}, the small module's {:.3} over {:.3}: \
         {by:+.0} µs, {verdict}",
        ms(it),
        ms(it_control),
        ms(rest),
        ms(rest_control)
    );
    println!("  {line}");
    lines.push(line);
}

/// What [`editors_project`] made: a module folder with its project's
/// files (a claims table of its areas, a talk map, described variables)
/// and its own custom talk table in the scratch user folder.
struct EditorsProject {
    path: std::path::PathBuf,
    /// Its areas.
    areas: usize,
    /// An area of many placeables (3,000 of `plc_armoire`; 20 in the small
    /// project), and one of 20.
    placed: ResRef,
    few: ResRef,
    placed_count: usize,
    /// A conversation of lines "The road, line 00000." at its root.
    talk: ResKey,
    talk_lines: usize,
    /// A creature blueprint (the game's `nw_bandit001`): its Basic page.
    creature: ResKey,
    /// The lines of its custom talk table and of its talk map.
    tlk_lines: usize,
    map_lines: usize,
    /// The talk map's file.
    map: std::path::PathBuf,
}

/// The project [`editors_at_scale`] times in: `large`, a copy of the
/// persistent world's module (`world/mod.rs`) grown to 1,400 areas, with
/// an area of 3,000 placeables, a conversation of 20,000 lines, a custom
/// talk table of 50,000 lines and a talk map of 20,000; else a new module
/// of four areas, 20 placeables, 20 lines, a table of 100 and a map of
/// 20, the world's haks named by both (the same game data). Each has a
/// claims table of every area (every other claimed by `cd`; the builder is
/// `ab`) and two variables described as rows of `racialtypes.2da` and two
/// of `placeables.2da`.
fn editors_project(
    game: &mg_rules::GameData,
    world: &std::path::Path,
    dir: &std::path::Path,
    user: &std::path::Path,
    large: bool,
) -> EditorsProject {
    use mg_gff::{Gff, Value};
    use mg_module::ModuleLocation;
    use mg_module::dialog::{Parent, add_node, new_dialog};
    use mg_module::new::{AreaSpec, add_area_of, new_module};
    let mut rng = fastrand::Rng::with_seed(24);
    let w = mg_module::Module::open(world).unwrap();
    let mut m = match large {
        true => w,
        false => {
            let mut m = new_module(game, "Editors, small", &mut rng).unwrap();
            let haks = w.info().unwrap().root.list("Mod_HakList").unwrap_or(&[]).to_vec();
            let mut info = m.info().unwrap();
            info.root.set("Mod_HakList", Value::List(haks));
            m.set_info(&info).unwrap();
            m
        }
    };
    let (total, placed_count, talk_lines, tlk_lines, map_lines) = match large {
        true => (1_400, 3_000, 20_000, 50_000, 20_000),
        false => (4, 20, 20, 100, 20),
    };
    let ttr01 = ResRef::from_str("ttr01").unwrap();
    let set = game.resman.get_named("ttr01", ResType::SET).unwrap();
    let set = mg_set::Tileset::parse(&set, game.codepage()).unwrap();
    let spec = |name: &str, side: u32| AreaSpec {
        name: name.into(),
        tileset: ttr01,
        width: side,
        height: side,
    };
    let bp = Gff::read(&game.resman.get_named("plc_armoire", ResType::UTP).unwrap()).unwrap();
    let placed = add_area_of(&mut m, game, &spec("A Placed Many", 8), &set, &mut rng).unwrap();
    place(&mut m, game, &bp.root, placed, placed_count, 80.0, &mut rng);
    let few = add_area_of(&mut m, game, &spec("A Placed Few", 8), &set, &mut rng).unwrap();
    place(&mut m, game, &bp.root, few, 20, 80.0, &mut rng);
    let had = m.areas().unwrap().len();
    for n in had..total {
        add_area_of(&mut m, game, &spec(&format!("Filler {n:04}"), 2), &set, &mut rng).unwrap();
    }
    // The conversation.
    let talk = ResKey::parse("mg_talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    for i in 0..talk_lines {
        add_node(&mut g, Parent::Root, &format!("The road, line {i:05}."));
    }
    m.set_gff(talk, &g).unwrap();
    // The creature.
    let creature = ResKey::parse("mg_crea", ResType::UTC).unwrap();
    let mut c = Gff::read(&game.resman.get_named("nw_bandit001", ResType::UTC).unwrap()).unwrap();
    c.root.set("TemplateResRef", Value::resref(creature.resref));
    m.set_gff(creature, &c).unwrap();
    // The custom talk table, in the user folder's `tlk`.
    let tlk = if large { "mg_big" } else { "mg_small" };
    let mut info = m.info().unwrap();
    info.root.set("Mod_CustomTlk", Value::String(tlk.as_bytes().to_vec()));
    m.set_info(&info).unwrap();
    let mut t = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
    t.entries = (0..tlk_lines)
        .map(|i| mg_tlk::TlkEntry {
            flags: mg_tlk::FLAG_TEXT,
            text: format!("Line {i} of the custom table").into_bytes(),
            ..Default::default()
        })
        .collect();
    std::fs::write(user.join("tlk").join(format!("{tlk}.tlk")), t.to_bytes().unwrap()).unwrap();
    let path = dir.join(if large { "large" } else { "small" });
    m.save_as(&ModuleLocation::Folder(path.clone())).unwrap();
    let areas = m.areas().unwrap();
    // The project's files.
    let mut claims =
        String::from("2DA V2.0\n\n    Area             Builder  Status   Date        Tag\n");
    for (i, a) in areas.iter().enumerate() {
        let (who, status) = if i % 2 == 0 { ("cd", "claimed") } else { ("****", "****") };
        claims += &format!("{i}   {a:<16} {who:<8} {status:<8} 2026-10-20  ****\n");
    }
    std::fs::write(path.join("claims.2da"), claims).unwrap();
    std::fs::write(
        path.join("moonglow.toml"),
        "[claims]\nfile = \"claims.2da\"\n\n[talk]\nlines = \"0-99999\"\nmap = \"tlk-map.json\"\n",
    )
    .unwrap();
    let map = path.join("tlk-map.json");
    write_map(&map, map_lines);
    let mut variables = String::new();
    for (name, table) in [("mg_race", "racialtypes"), ("mg_look", "placeables")] {
        for name in [name.to_string(), format!("{name}_too")] {
            variables += &format!(
                "[{name}]\ntype = \"int\"\non = [\"placeable\"]\ntable = \"{table}\"\n\
                 help = \"A row of {table}.2da\"\n\n"
            );
        }
    }
    std::fs::write(path.join("variables.toml"), variables).unwrap();
    mg_module::project_file::ProjectFile::set_builder_code(&path, Some("ab")).unwrap();
    EditorsProject {
        path,
        areas: areas.len(),
        placed,
        few,
        placed_count,
        talk,
        talk_lines,
        creature,
        tlk_lines,
        map_lines,
        map,
    }
}

/// A talk map (`mg tlk-alloc`'s) of `lines` keys, a line each from 0.
fn write_map(path: &std::path::Path, lines: usize) {
    let map = mg_module::tlk_alloc::Map {
        lines: (0..lines).map(|i| (format!("mg.line.{i:05}"), i as u32)).collect(),
        retired: Default::default(),
    };
    std::fs::write(path, map.to_json()).unwrap();
}

/// What [`module_views`] timed.
#[derive(Default)]
struct ModuleTimed {
    control: Option<Duration>,
    claims: Option<Duration>,
    save: Option<Duration>,
    typed: Option<Duration>,
}

/// The views timed in both projects of [`editors_at_scale`], the small and
/// the large: the window with nothing open (the control), the Claims Table
/// of every area, the frames a hak's save lands in (a copy of the game's
/// `id_resources.hak`, a file added), and those a name typed into the talk
/// table lands in (a creature blueprint's First Name, on its Basic page).
fn module_views(
    h: &mut App<'_>,
    r: &mut Report,
    lines: &mut Vec<String>,
    p: &EditorsProject,
    when: &str,
    hak: &std::path::Path,
) -> ModuleTimed {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
    let control =
        timed(h, r, &format!("editors, nothing open, {when}"), "pointer", screen, Pointer::Moves);
    let mut out = ModuleTimed { control, ..Default::default() };
    // D24: the Claims Table.
    let name = "Claims Table";
    if editors_wanted(r, name) {
        menu(h, "Edit", "Claims Table…");
        h.run_steps(5);
        assert_eq!(
            h.state().bottom_dock_shows(),
            Some(mg_ui::Panel::ClaimsTable),
            "{name} not shown ({when})"
        );
        assert!(
            h.query_by_label_contains("claims.2da: whose each area is").is_some(),
            "{name}: not its table ({when})"
        );
        let what = format!("{name} of {} areas, {when}", p.areas);
        out.claims = timed(h, r, &what, "pointer", screen, Pointer::Moves);
        click(h, "Close Report");
        h.run_steps(3);
    }
    // The last pass's item 17: a hak saved with the module open.
    let name = "hak saved";
    if editors_wanted(r, name) {
        let id = mg_ui::hak_view::open_at(h.state_mut(), hak).expect("the hak");
        let t = Tab::Hak(id);
        let extra = hak.with_file_name("mg_extra.mdl");
        std::fs::write(&extra, b"extra").unwrap();
        h.state_mut().haks.iter_mut().find(|d| d.id == id).unwrap().hak.add_files(&[extra]);
        h.run_steps(5);
        let rect = rect_of(h.state(), &t).expect("the hak's tab");
        let button = h
            .query_all_by_label("Save")
            .map(|n| n.rect())
            .find(|b| rect.contains(b.center()))
            .expect("the hak editor's Save")
            .center();
        let resources = h.state().ws.as_ref().map_or(0, |ws| ws.module.len());
        let what = format!("{name}, the click, {when} ({resources} resources)");
        let click = landed(h, lines, &what, Pointer::Clicks(button), 4);
        // (Then the frames until it is written and taken as saved, and a
        // few after.)
        let (begun, mut worst, mut count, mut after) = (Instant::now(), Duration::ZERO, 0, 0);
        while after < 4 {
            let f = frames(h, screen, Pointer::Away, 1, |_, _| {});
            worst = worst.max(f.ui[0] + f.tessellate[0]);
            count += 1;
            after += usize::from(!h.state().busy());
            assert!(begun.elapsed() < Duration::from_secs(120), "{name}: not saved ({when})");
        }
        let verdict = if worst > BUDGET { "over the budget" } else { "within the budget" };
        let line = format!(
            "{name}, the frames until it is saved, {when}: {count}, the worst {:.2} ms, {verdict}",
            ms(worst)
        );
        println!("  {line}");
        lines.push(line);
        out.save = Some(click.max(worst));
        h.run_steps(3);
        let failed = h.state().log.entries.iter().any(|(_, m)| m.contains("Could not save"));
        assert!(!failed, "{name}: {:?}", h.state().log.entries);
        close(h, &t);
    }
    // D27: a name typed into the talk table.
    let name = "typed line";
    if editors_wanted(r, name) {
        out.typed =
            Some(typed_commit(h, lines, p, &format!("{name}, {when}"), ("Bandit", "Typed In")));
    }
    out
}

/// `FRAME_VIEW`'s pick, for [`editors_at_scale`]'s groups.
fn editors_wanted(r: &Report, what: &str) -> bool {
    r.only.as_ref().is_none_or(|o| what.contains(o.as_str()) || o.starts_with(what))
}

/// The creature blueprint's First Name (`from`) typed anew (`text`) and let go
/// (Enter), on its Basic page: the worst of the frames that lands in. The
/// window's Options › Language says whether the text goes into the talk
/// table.
fn typed_commit(
    h: &mut App<'_>,
    lines: &mut Vec<String>,
    p: &EditorsProject,
    what: &str,
    (from, text): (&str, &'static str),
) -> Duration {
    use mg_edit::GffPath;
    let t = Tab::Blueprint(p.creature);
    h.state_mut().blueprint_pages.insert((p.creature, GffPath::root()), "Basic");
    open(h, t.clone());
    h.run_steps(5);
    let first = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some(from)
    };
    h.query(egui_kittest::kittest::by().predicate(first))
        .unwrap_or_else(|| panic!("{what}: no field of {from:?}"))
        .click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run_steps(1);
    h.event(egui::Event::Text(text.into()));
    h.run_steps(2);
    let away = egui::pos2(screen_size().x - 20.0, screen_size().y - 20.0);
    let worst = landed(h, lines, what, Pointer::Keys(away, egui::Key::Enter, 1_000), 8);
    h.run_steps(5);
    assert!(h.query_all_by_value(text).next().is_some(), "{what}: the name not set");
    close(h, &t);
    worst
}

/// The review of November 2026's editor findings at the sizes they name
/// (the deferred list's "performance fixes without a timing view": D20's,
/// D21, D23's rest, D24, D25, D27, D29, S20 and the last pass's item 17),
/// each against the same view at a small size, timed just before it, in a
/// project made from the persistent world's module (`editors_project`:
/// 1,400 areas, the world's haks): `spells.2da` in the 2DA editor at the
/// window's full width, its copy under another name (no description) at
/// full width against the copy cut to the columns in sight (D20);
/// `placeables.2da`'s ModelName chosen with Choose Model open against
/// closed, over the haks' models, and a described table of 20,000 rows
/// whose `PREREQFEAT1` names a row of itself, a cell of it chosen against
/// one of its Label (D21); Advanced Search with every one of the world's
/// variables ticked, with a plugin whose command is for them against
/// without (D23); a conversation's Search finding 20,000 lines against 10
/// (and the click of Find), Build Module's 20,000 results in Problems
/// against 20, an area's Placeable List of 3,000 opened in the raw fields
/// view against 20 (D24); Module Properties › Custom Content with a talk
/// map of 20,000 lines against 20 (D25); the read-only viewer on
/// `nwscript.nss` against a short script (D29); the Variables window with
/// a variable of `placeables.2da`'s rows against one of `racialtypes.2da`,
/// its list closed and open (S20, item 17); the module tree with the area
/// of 3,000 opened out against the area of 20 (item 17). In both
/// projects, the large and a small one (`module_views`): the Claims Table
/// (1,400 areas against 4, each over its window's control), the click
/// that saves a hak (item 17) and a name typed into the custom talk table
/// (50,000 lines against 100; in the large also with the text kept in the
/// field, D27), the frames they land in held to the budget.
/// `FRAME_VIEW=<part of a name>` picks a group ("2DA editor", "Advanced
/// Search", "Claims Table", "conversation's Search", "Build Module", "raw
/// fields", "Custom Content", "read-only viewer", "Variables window",
/// "module tree", "hak saved", "typed line").
#[test]
#[ignore]
fn editors_at_scale() {
    use mg_edit::GffPath;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let Some(w) = world::world(&root) else { return };
    let dir = mg_testkit::scratch_dir("frame-perf-editors");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("tlk")).unwrap();
    link_dir(&w.user.join("hak"), &user.join("hak"));
    let begun = Instant::now();
    let (small, large) = {
        let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
        (
            editors_project(&game, &w.module, &dir, &user, false),
            editors_project(&game, &w.module, &dir, &user, true),
        )
    };
    println!("the editors at scale: the projects made in {:.1?}", begun.elapsed());
    let haks = dir.join("haks");
    std::fs::create_dir_all(&haks).unwrap();
    let hak = haks.join("mg_save.hak");
    std::fs::copy(root.join("data/hk/id_resources.hak"), &hak).unwrap();
    let mut r = Report::new();
    let mut lines = Vec::new();
    println!("the editors at scale: frames at {:?}", screen_size());
    let rs = render_state();
    let mut app = Moonglow::new(
        Some(GameInstall::new(&root, Some(user.clone()), "en")),
        Box::new(NoDialogs::default()),
    );
    app.set_render_state(rs.clone());
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.settings.no_last_area = true;
    app.settings.talk_typed = true;
    app.open_module(&small.path);
    let mut h = harness(app, rs);
    h.run_steps(3);
    // (Pictures read apart, as the desktop app reads them.)
    h.state_mut().background_jobs = true;
    fold_palette(&mut h);
    close(&mut h, &Tab::ModuleProperties);
    close_areas(&mut h);
    h.run_steps(5);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size());
    let corner = egui::pos2(screen.right() - 20.0, screen.bottom() - 20.0);

    // The small project's views, then the large's.
    let few = module_views(&mut h, &mut r, &mut lines, &small, "the small module", &hak);
    h.state_mut().open_module(&large.path);
    h.run_steps(5);
    close(&mut h, &Tab::ModuleProperties);
    close_areas(&mut h);
    h.run_steps(5);
    let areas = h.state().ws.as_ref().unwrap().module.areas().unwrap().len();
    assert_eq!(areas, large.areas, "the large module's areas");
    let control = timed(
        &mut h,
        &mut r,
        "editors, nothing open, the large module",
        "pointer",
        screen,
        Pointer::Moves,
    );

    // D20's rest: spells.2da at the window's width.
    let name = "2DA editor";
    if editors_wanted(&r, name) {
        let tables = dir.join("tables");
        std::fs::create_dir_all(&tables).unwrap();
        let game = h.state().game.clone().unwrap();
        let cp = game.codepage();
        let spells = game.table("spells").unwrap();
        let open_table = |h: &mut App<'_>, path: &std::path::Path| -> (u32, Tab) {
            let id = mg_ui::table_view::open_path(h.state_mut(), path).expect("the table");
            h.run_steps(5);
            (id, Tab::Table(id))
        };
        std::fs::write(tables.join("spells.2da"), spells.to_bytes(cp).unwrap()).unwrap();
        let (_, t) = open_table(&mut h, &tables.join("spells.2da"));
        let rect = rect_of(h.state(), &t).expect("the table's tab");
        let columns = spells.columns().to_vec();
        let what = format!("{name}, spells.2da ({} columns, {} rows)", columns.len(), spells.len());
        timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves);
        // The columns in sight: their heads in the tab, beside the first's.
        let head = h
            .query_all_by_label(&columns[0])
            .map(|n| n.rect())
            .find(|b| rect.contains(b.center()))
            .expect("the first column's head");
        let in_sight = columns
            .iter()
            .take_while(|c| {
                h.query_all_by_label(c).any(|n| {
                    let b = n.rect();
                    (b.center().y - head.center().y).abs() < 3.0 && b.left() < rect.right()
                })
            })
            .count();
        println!("  ({name}: {in_sight} of spells.2da's {} columns in sight)", columns.len());
        assert!(in_sight > 3 && in_sight < columns.len(), "{name}: {in_sight} columns in sight");
        close(&mut h, &t);
        // The same under another name (no description, no game copy):
        // cut to the columns in sight, then whole.
        let mut cut = (*spells).clone();
        while cut.columns().len() > in_sight {
            cut.remove_column(cut.columns().len() - 1);
        }
        std::fs::write(tables.join("mg_cut.2da"), cut.to_bytes(cp).unwrap()).unwrap();
        std::fs::write(tables.join("mg_wide.2da"), spells.to_bytes(cp).unwrap()).unwrap();
        let mut took = Vec::new();
        for (file, what) in [
            ("mg_cut.2da", format!("its {in_sight} columns in sight")),
            ("mg_wide.2da", format!("all {} columns", columns.len())),
        ] {
            let (_, t) = open_table(&mut h, &tables.join(file));
            let rect = rect_of(h.state(), &t).expect("the table's tab");
            let what = format!("{name}, spells.2da's rows as {file}, {what}");
            took.push(timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves));
            close(&mut h, &t);
        }
        against(&mut lines, &format!("{name}, spells.2da at the full width"), took[0], took[1]);

        // D21: Choose Model open over the haks' models.
        let placeables = game.table("placeables").unwrap();
        std::fs::write(tables.join("placeables.2da"), placeables.to_bytes(cp).unwrap()).unwrap();
        let (id, t) = open_table(&mut h, &tables.join("placeables.2da"));
        let model = placeables.column("ModelName").expect("ModelName");
        h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(0, model);
        // (The Cell panel shown, loudly: the Right Dock may be folded.)
        h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Cell));
        h.run_steps(5);
        assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Cell), "{name}: no Cell panel");
        assert!(h.query_by_label("Choose Model…").is_some(), "{name}: no Choose Model…");
        let what = format!("{name}, placeables.2da's ModelName chosen ({} rows)", placeables.len());
        let rest = timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves);
        click(&mut h, "Choose Model…");
        h.run_steps(5);
        // (Its list: the first of the models by name, in the Right Dock,
        // those in sight laid out.)
        let dock = h.state().right_dock_rect().expect("the Right Dock");
        let listed = h
            .query_all_by(|n| n.label().is_some())
            .filter(|n| dock.contains(n.rect().center()))
            .skip_while(|n| n.accesskit_node().label().as_deref() != Some("Choose Model…"))
            .count();
        assert!(listed > 8, "{name}: Choose Model's list not shown ({listed})");
        let models = game.resman.list(ResType::MDL).len();
        let of = format!("{models} models");
        let what = format!("{name}, Choose Model open ({of})");
        let it = timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves);
        against(&mut lines, &what, rest, it);
        click(&mut h, "Choose Model…");
        close(&mut h, &t);

        // D21: a described table whose Row column names its own rows.
        let own = dir.join("described");
        std::fs::create_dir_all(&own).unwrap();
        std::fs::write(own.join("nasher.cfg"), "").unwrap();
        let mut text = String::from("2DA V2.0\n\n  Label PREREQFEAT1 Other\n");
        let rows = 20_000;
        for i in 0..rows {
            text += &format!("{i} Feat_{i} {} {i}\n", (i * 7) % rows);
        }
        std::fs::write(own.join("myfeat.2da"), text).unwrap();
        std::fs::write(
            own.join("myfeat.2da.toml"),
            "[columns.PREREQFEAT1]\nkind = \"row\"\ntable = \"myfeat\"\n",
        )
        .unwrap();
        let (id, t) = open_table(&mut h, &own.join("myfeat.2da"));
        let begun = Instant::now();
        let described = |h: &App<'_>| {
            let doc = h.state().tables.iter().find(|d| d.id == id).unwrap();
            doc.description().is_some_and(|d| d.column("PREREQFEAT1").is_some())
        };
        while !described(&h) {
            h.run_steps(1);
            assert!(begun.elapsed() < Duration::from_secs(60), "{name}: myfeat.2da not described");
        }
        let mut took = Vec::new();
        for (column, what) in [(0, "its Label"), (1, "its PREREQFEAT1, a row of itself")] {
            h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(5, column);
            h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Cell));
            h.run_steps(5);
            assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Cell), "{name}: no Cell");
            let what = format!("{name}, a described table of {rows} rows, a cell of {what}");
            took.push(timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves));
        }
        let what = format!("{name}, a described table of {rows} rows, a cell naming its own row");
        against(&mut lines, &what, took[0], took[1]);
        close(&mut h, &t);
    }

    // D23's rest: Advanced Search with a plugin's command for the hits.
    let name = "Advanced Search";
    if editors_wanted(&r, name) {
        use mg_ui::search::{AdvancedSearch, Condition, Kind};
        let mut d = AdvancedSearch {
            first: Condition { kind: Kind::Variable, name: "*".into(), ..Default::default() },
            // (Placed objects' alone: a plugin's command is offered for the
            // ticked hits only where each is something a command takes,
            // which the module's and the areas' own variables are not.)
            placed: Some(true),
            ..Default::default()
        };
        h.state_mut().advanced_search(&mut d);
        let found = d.hits.len();
        assert!(found > 1_000, "{name}: {found} hits");
        d.ticked = vec![true; found];
        h.state_mut().advanced_search = d;
        open(&mut h, Tab::AdvancedSearch);
        h.run_steps(5);
        let rect = rect_of(h.state(), &Tab::AdvancedSearch).expect("Advanced Search's tab");
        let what = format!("{name}, {found} placed objects' variables ticked");
        let rest = timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves);
        let plugins = dir.join("plugins");
        let plugin = plugins.join("each");
        std::fs::create_dir_all(&plugin).unwrap();
        std::fs::write(
            plugin.join("plugin.cfg"),
            "[plugin]\nid = \"test.each\"\nname = \"Each\"\nversion = \"1\"\napi = \"0.5\"\n\
             license = \"MIT\"\n\n[command]\nid = \"each\"\ntitle = \"Each Kind\"\nfor = \"area, \
             dlg, nss, jui, jrl, fac, creature, door, encounter, item, placeable, sound, store, \
             trigger, waypoint\"\n",
        )
        .unwrap();
        std::fs::write(
            plugin.join("main.luau"),
            "local mg = require(\"@moonglow\")\nmg.command(\"each\", function(ctx) end)\n",
        )
        .unwrap();
        h.state_mut().plugin_dir = Some(plugins);
        h.state_mut().load_plugins();
        h.state_mut().enable_plugin("test.each", true);
        h.run_steps(5);
        let what = format!(
            "{name}, {found} placed objects' variables ticked, a plugin's command for them"
        );
        let it = timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves);
        against(&mut lines, &what, rest, it);
        // (The command offered on a ticked hit: its menu.)
        let hit = h
            .query_all_by_label_contains(" › ")
            .map(|n| n.rect())
            .find(|b| rect.contains(b.center()))
            .expect("a hit's row");
        h.hover_at(hit.center());
        h.run_steps(1);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: hit.center(),
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
        assert!(
            h.query_by_label("Each Kind").is_some(),
            "{name}: the plugin's command not offered"
        );
        close_menu(&mut h);
        h.state_mut().enable_plugin("test.each", false);
        close(&mut h, &Tab::AdvancedSearch);
    }

    // D24: a conversation's Search.
    let name = "conversation's Search";
    if editors_wanted(&r, name) {
        let t = Tab::Dialog(large.talk);
        open(&mut h, t.clone());
        h.run_steps(5);
        h.state_mut().show_conversation_search(large.talk);
        h.run_steps(3);
        let mut took = Vec::new();
        for (find, n) in [("line 0001", 10), ("road", large.talk_lines)] {
            {
                let view = h.state_mut().dialog_views.get_mut(&large.talk).unwrap();
                view.search.find = find.into();
                view.search.all_files = false;
            }
            h.run_steps(2);
            let button = h.query_all_by_label("Find").last().expect("Find").rect().center();
            let what = format!("{name}, the click of Find ({n} found)");
            landed(&mut h, &mut lines, &what, Pointer::Clicks(button), 4);
            h.run_steps(3);
            let results = h.state().dialog_views[&large.talk].search.results.len();
            assert_eq!(results, n, "{name}: found for {find:?}");
            let what = format!("{name}, {n} found");
            took.push(timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves));
        }
        against(&mut lines, &format!("{name}, {} found", large.talk_lines), took[0], took[1]);
        close(&mut h, &t);
    }

    // D24: Build Module's results, in Problems.
    let name = "Build Module";
    if editors_wanted(&r, name) {
        use mg_ui::problems::{Go, Problem, Source};
        let made = |n: usize| -> Vec<Problem> {
            (0..n)
                .map(|i| {
                    let key = ResKey::parse(&format!("mg_unused{i:05}"), ResType::NSS).unwrap();
                    let mut p = Problem::new(
                        mg_module::doctor::Severity::Warning,
                        format!("mg_unused{i:05}.nss"),
                        "Unused: nothing in the module runs this script".into(),
                        Some(Go::Resource(key)),
                    );
                    p.unused = Some(mg_module::verify::Unused { key, script: None });
                    p
                })
                .collect()
        };
        let mut took = Vec::new();
        for n in [20, 20_000] {
            h.state_mut().problems.replace(Source::Build, made(n), None);
            h.state_mut().problems.select(Some((Source::Build, 0)));
            h.state_mut().actions.push(Action::ShowPanel(mg_ui::Panel::Problems));
            h.run_steps(5);
            assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Problems));
            let what = format!("{name}'s {n} results in Problems, one chosen");
            took.push(timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves));
        }
        against(&mut lines, &format!("{name}'s 20000 results in Problems"), took[0], took[1]);
        h.state_mut().problems.remove(&Source::Build);
        h.run_steps(3);
    }

    // D24: an area's Placeable List in the raw fields view.
    let name = "raw fields";
    if editors_wanted(&r, name) {
        let mut took = Vec::new();
        for (area, n) in [(large.few, 20), (large.placed, large.placed_count)] {
            let t = Tab::Gff(ResKey::new(area, ResType::GIT));
            open(&mut h, t.clone());
            h.run_steps(5);
            click(&mut h, &format!("{n} items"));
            h.run_steps(3);
            assert!(h.query_by_label_contains("[0] struct").is_some(), "{name}: the list not open");
            let rect = rect_of(h.state(), &t).expect("the fields' tab");
            let what = format!("{name} view, an area's Placeable List of {n} opened");
            took.push(timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves));
            close(&mut h, &t);
        }
        let what =
            format!("{name} view, an area's Placeable List of {} opened", large.placed_count);
        against(&mut lines, &what, took[0], took[1]);
    }

    // D25: Module Properties › Custom Content with a talk map.
    let name = "Custom Content";
    if editors_wanted(&r, name) {
        open(&mut h, Tab::ModuleProperties);
        h.run_steps(3);
        click(&mut h, "Custom Content");
        h.run_steps(5);
        let mut took = Vec::new();
        for n in [20, large.map_lines] {
            write_map(&large.map, n);
            h.run_steps(5);
            assert!(h.query_by_label_contains("Next: line").is_some(), "{name}: no room said");
            let what = format!(
                "Module Properties › {name}, a talk map of {n} lines ({} in the table)",
                large.tlk_lines
            );
            took.push(timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves));
        }
        let what = format!("Module Properties › {name}, a talk map of {} lines", large.map_lines);
        against(&mut lines, &what, took[0], took[1]);
        close(&mut h, &Tab::ModuleProperties);
    }

    // D29: the read-only viewer on nwscript.nss.
    let name = "read-only viewer";
    if editors_wanted(&r, name) {
        let mut took = Vec::new();
        let mut whats = Vec::new();
        for script in ["nw_c2_default1", "nwscript"] {
            let key = ResKey::parse(script, ResType::NSS).unwrap();
            let lines_of = {
                let game = h.state().game.as_deref().unwrap();
                let data = game.resman.get(&key).expect("the game's script");
                data.iter().filter(|b| **b == b'\n').count()
            };
            let t = Tab::Resource(key);
            open(&mut h, t.clone());
            h.run_steps(10);
            let rect = rect_of(h.state(), &t).expect("the viewer's tab");
            let what = format!("{name}, {script}.nss ({lines_of} lines)");
            took.push(timed(&mut h, &mut r, &what, "pointer", rect, Pointer::Moves));
            whats.push(what);
            close(&mut h, &t);
        }
        against(&mut lines, &whats[1], took[0], took[1]);
    }

    // S20 and item 17: the Variables window on a variable of a table's rows.
    let name = "Variables window";
    if editors_wanted(&r, name) {
        use mg_ui::widgets::{FieldTarget, VarRow, VarTableEdit};
        let git = ResKey::new(large.placed, ResType::GIT);
        let path = GffPath::root().item("Placeable List", 0);
        let (mut closed, mut opened, mut whats) = (Vec::new(), Vec::new(), Vec::new());
        for (var, table) in [("mg_race", "racialtypes"), ("mg_look", "placeables")] {
            let rows = h.state().game.as_deref().unwrap().table(table).map_or(0, |t| t.len());
            let mut edit = VarTableEdit::new(FieldTarget::new(git, path.clone(), "VarTable"), &[]);
            edit.rows = vec![VarRow::new(var, 1, "5"), VarRow::new(&format!("{var}_too"), 1, "")];
            h.state_mut().var_edit = Some(edit);
            h.run_steps(5);
            let what = format!("{name}, a variable of {table}.2da's {rows} rows");
            closed.push(timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves));
            let choose = h.query_all_by_value("(choose)").next().expect("the second row's values");
            choose.click();
            h.run_steps(3);
            let filter = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
                n.role() == egui::accesskit::Role::TextInput && n.placeholder() == Some("Filter")
            };
            let shown = h.query_all(egui_kittest::kittest::by().predicate(filter)).next().is_some();
            assert!(shown, "{name}: the values' list not open ({table})");
            let what = format!("{what}, its list open");
            opened.push(timed(&mut h, &mut r, &what, "still", screen, Pointer::Rests(corner)));
            whats.push(what);
            h.key_press(egui::Key::Escape);
            h.run_steps(3);
            h.state_mut().var_edit = None;
            h.run_steps(3);
        }
        let what = format!("{name}, a variable of placeables.2da's rows");
        against(&mut lines, &what, closed[0], closed[1]);
        against(&mut lines, &whats[1], opened[0], opened[1]);
    }

    // Item 17: the module tree with an area opened out.
    let name = "module tree";
    if editors_wanted(&r, name) {
        let mut took = Vec::new();
        for (area, n) in [(large.few, 20), (large.placed, large.placed_count)] {
            let arrow = format!("Placed Objects in {area}");
            click(&mut h, &arrow);
            h.run_steps(5);
            let kind = format!("Placeables ({n})");
            click(&mut h, &kind);
            h.run_steps(5);
            // (Its placeables in sight in the tree, under the area's row:
            // a row out of sight costs nothing.)
            let tree = h.get_by_label("Module Tree Rows").rect();
            let rows = h
                .query_all_by_label("Armoire")
                .filter(|n| tree.contains(n.rect().center()))
                .count();
            assert!(rows >= 20, "{name}: {rows} of the area's placeables in sight");
            let what =
                format!("{name}, an area of {n} placeables opened out ({rows} rows in sight)");
            took.push(timed(&mut h, &mut r, &what, "pointer", screen, Pointer::Moves));
            // (Closed again: the next area's rows where these were.)
            click(&mut h, &arrow);
            h.run_steps(5);
        }
        let what = format!("{name}, an area of {} placeables opened out", large.placed_count);
        against(&mut lines, &what, took[0], took[1]);
    }

    let many = module_views(&mut h, &mut r, &mut lines, &large, "the large module", &hak);
    // D27: the same name typed with the text kept in the field (Options ›
    // Language's switch off), against into the table.
    let name = "typed line";
    if editors_wanted(&r, name) {
        h.state_mut().settings.talk_typed = false;
        let what = format!("{name}, the large module, the text kept in the field");
        typed_commit(&mut h, &mut lines, &large, &what, ("Typed In", "Typed Kept"));
        h.state_mut().settings.talk_typed = true;
    }
    against_controlled(
        &mut lines,
        &format!("Claims Table of {} areas", large.areas),
        (few.claims, few.control),
        (many.claims, many.control),
    );
    let _ = control;
    println!(
        "against their resting views (the line {} µs):\n  {}",
        UI_LINE.as_micros(),
        lines.join("\n  ")
    );
    r.end();
}
