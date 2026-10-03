//! Performance budgets on the largest campaigns the game ships (Tyrants of
//! the Moonsea, Darkness over Daggerford, Chapter 2): opening the module,
//! the editor's frames, its largest area in the viewer, rebuilding the
//! palettes, verifying, compiling every script and writing the module.
//! Each step has a budget; the timings are printed. Needs the game and a
//! GPU; run in release:
//! `cargo test --release -p mg-ui --test campaign_perf -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use mg_core::ResType;
use mg_resman::GameInstall;
use mg_ui::{Action, Moonglow, NoDialogs, Tab};

/// The campaigns, largest first.
const CAMPAIGNS: [&str; 3] = [
    "Neverwinter Nights - Tyrants of the Moonsea.nwm",
    "Neverwinter Nights - Darkness over Daggerford.nwm",
    "Chapter2.nwm",
];

/// Budgets (release build).
const OPEN: Duration = Duration::from_secs(3);
const FRAME: Duration = Duration::from_millis(33);
const AREA_LOAD: Duration = Duration::from_secs(5);
const AREA_FRAME: Duration = Duration::from_millis(50);
const PALETTES: Duration = Duration::from_secs(3);
const VERIFY: Duration = Duration::from_secs(10);
const COMPILE: Duration = Duration::from_secs(120);
const WRITE: Duration = Duration::from_secs(5);

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

/// Times `f`.
fn time<R>(f: impl FnOnce() -> R) -> (R, Duration) {
    let t = Instant::now();
    let r = f();
    (r, t.elapsed())
}

/// The campaign's area with the most tiles.
fn largest_area(app: &mut Moonglow) -> Option<mg_core::ResRef> {
    let ws = app.ws.as_mut()?;
    let keys: Vec<_> = ws.module.keys_of(ResType::ARE).copied().collect();
    keys.into_iter()
        .filter_map(|k| {
            let g = ws.doc(&k).ok()?;
            let w = g.root.integer("Width")?;
            let h = g.root.integer("Height")?;
            Some((w * h, k.resref))
        })
        .max_by_key(|(n, _)| *n)
        .map(|(_, r)| r)
}

#[test]
#[ignore]
fn largest_campaigns_stay_within_budget() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let nwm = root.join("data/nwm");
    let mut over = Vec::new();
    let mut check = |what: String, took: Duration, budget: Duration| {
        let mark = if took > budget { "OVER" } else { "ok" };
        println!("  {what:42} {took:>12.2?}  (budget {budget:?}) {mark}");
        if took > budget {
            over.push(format!("{what}: {took:?} > {budget:?}"));
        }
    };
    for name in CAMPAIGNS {
        let path: PathBuf = nwm.join(name);
        if !path.is_file() {
            println!("{name}: not installed");
            continue;
        }
        println!("{name} ({} MB)", path.metadata().unwrap().len() / 1_000_000);
        let rs = egui_kittest::wgpu::create_render_state(
            egui_kittest::wgpu::default_wgpu_setup(),
            egui_wgpu::RendererOptions::PREDICTABLE,
        );
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        app.set_render_state(rs.clone());
        let ((), took) = time(|| app.open_module(Path::new(&path)));
        check(format!("open ({} resources)", app.ws.as_ref().unwrap().module.len()), took, OPEN);
        let area = largest_area(&mut app);
        let mut h = Harness::builder()
            .with_size(egui::vec2(1280.0, 800.0))
            .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        let ((), took) = time(|| h.run_steps(1));
        check("first frame".into(), took, FRAME * 10);
        let frames: Vec<Duration> = (0..10).map(|_| time(|| h.run_steps(1)).1).collect();
        check("frame (module open)".into(), median(frames), FRAME);
        if let Some(area) = area {
            h.state_mut().actions.push(Action::OpenTab(Tab::Area(area)));
            // The tab opens after the frame; the view loads the area as
            // it first draws.
            let tiles = |h: &Harness<'_, Moonglow>| {
                h.state()
                    .area_views
                    .get(&area)
                    .and_then(|v| v.model.as_ref())
                    .map(|m| m.tiles.len())
            };
            let t = Instant::now();
            let mut frames = 0;
            while tiles(&h).is_none() && frames < 10 {
                h.run_steps(1);
                frames += 1;
            }
            let took = t.elapsed();
            let tiles = tiles(&h).expect("the area view loads");
            check(format!("open its largest area ({area}, {tiles} tiles)"), took, AREA_LOAD);
            let frames: Vec<Duration> = (0..10).map(|_| time(|| h.run_steps(1)).1).collect();
            check("frame (area view)".into(), median(frames), AREA_FRAME);
        }
        let app = h.state_mut();
        let (game, ws) = (app.game.as_deref().unwrap(), app.ws.as_mut().unwrap());
        let (result, took) =
            time(|| mg_module::palette::rebuild_custom_palettes(&mut ws.module, game));
        result.unwrap();
        check("rebuild custom palettes".into(), took, PALETTES);
        let (missing, took) = time(|| mg_module::verify::missing(&ws.module, &game.resman));
        let (unused, t2) = time(|| mg_module::verify::unused(&ws.module));
        check(
            format!("verify ({} missing, {} unused)", missing.len(), unused.len()),
            took + t2,
            VERIFY,
        );
        let scripts = ws.module.keys_of(ResType::NSS).count();
        let ((), took) = time(|| {
            h.state_mut().actions.push(Action::CompileScripts);
            h.run_steps(1);
        });
        check(format!("compile every script ({scripts})"), took, COMPILE);
        for (_, e) in h.state().log.entries.iter().rev().take(8) {
            println!("    {e}");
        }
        let app = h.state_mut();
        let ws = app.ws.as_mut().unwrap();
        let dir = mg_testkit::scratch_dir("campaign-perf");
        let (bytes, took) = time(|| ws.snapshot().unwrap().to_archive_bytes().unwrap());
        std::fs::write(dir.join("campaign.mod"), &bytes).unwrap();
        check(format!("write the module ({} MB)", bytes.len() / 1_000_000), took, WRITE);
    }
    assert!(over.is_empty(), "over budget:\n{}", over.join("\n"));
}
