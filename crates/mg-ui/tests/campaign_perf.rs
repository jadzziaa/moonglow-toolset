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
const SEARCH: Duration = Duration::from_millis(200);

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
            mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
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

/// The memory the process holds now, and the most it has held, in MiB.
fn memory() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| {
        let line = status.lines().find(|l| l.starts_with(name)).unwrap_or("");
        line.split_whitespace().nth(1).and_then(|n| n.parse::<u64>().ok()).unwrap_or(0) / 1024
    };
    (field("VmRSS:"), field("VmHWM:"))
}

/// A module with a builder's real haks (every one in `MOONGLOW_HAKS`'s
/// folders: The Domains of Dread's 145, 28 GB with CEP's, where the
/// persistent world's are stand-ins): opening it, its frames, the
/// standard palettes with the haks' blueprints and rows, the resource
/// browser over them, and the memory held. Skips without the haks; run in
/// release with them:
/// `MOONGLOW_HAKS=~/Projects/tdod-module/build/hak cargo test --release -p mg-ui --test campaign_perf builder_s_haks -- --ignored --nocapture`.
#[test]
#[ignore]
fn a_module_with_a_builder_s_haks_stays_within_budget() {
    use mg_gff::{Struct, Value};
    use mg_module::palette::BlueprintKind;
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let haks: Vec<PathBuf> = mg_testkit::builder_hak_dirs()
        .iter()
        .flat_map(|d| mg_testkit::files_with_ext(d, "hak"))
        .collect();
    if haks.len() < 10 {
        println!("skipped: {} haks in MOONGLOW_HAKS", haks.len());
        return;
    }
    let bytes: u64 = haks.iter().filter_map(|h| h.metadata().ok()).map(|m| m.len()).sum();
    let dir = mg_testkit::scratch_dir("campaign-perf-haks");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    std::fs::create_dir_all(user.join("modules")).unwrap();
    for h in &haks {
        mg_testkit::link_or_copy(h, &user.join("hak").join(h.file_name().unwrap())).unwrap();
    }
    let install = GameInstall::new(&root, Some(user.clone()), "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut m =
        mg_module::new::new_module(&game, "Haks", &mut fastrand::Rng::with_seed(3)).unwrap();
    let mut info = m.info().unwrap();
    let list = haks
        .iter()
        .map(|h| {
            let mut s = Struct::new(8);
            let name = h.file_stem().unwrap().to_string_lossy();
            s.set("Mod_Hak", Value::String(name.as_bytes().to_vec()));
            s
        })
        .collect();
    info.root.set("Mod_HakList", Value::List(list));
    m.set_info(&info).unwrap();
    let path = user.join("modules/mg_haks.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    drop(game);

    let mut over = Vec::new();
    let mut check = |what: String, took: Duration, budget: Duration| {
        let mark = if took > budget { "OVER" } else { "ok" };
        println!("  {what:52} {took:>12.2?}  (budget {budget:?}) {mark}");
        if took > budget {
            over.push(format!("{what}: {took:?} > {budget:?}"));
        }
    };
    println!("a module with {} haks ({} GB)", haks.len(), bytes / 1_000_000_000);
    let rs = egui_kittest::wgpu::create_render_state(
        mg_ui::graphics_setup(egui_kittest::wgpu::default_wgpu_setup()),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    let ((), took) = time(|| app.open_module(&path));
    let layers = app.game.as_ref().map_or(0, |g| g.resman.layers().len());
    check(format!("open ({layers} layers)"), took, OPEN);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    let ((), took) = time(|| h.run_steps(1));
    check("first frame".into(), took, FRAME * 10);
    let frames: Vec<Duration> = (0..10).map(|_| time(|| h.run_steps(1)).1).collect();
    check("frame (module open)".into(), median(frames), FRAME);
    for (kind, name) in [
        (BlueprintKind::Placeable, "placeable"),
        (BlueprintKind::Item, "item"),
        (BlueprintKind::Creature, "creature"),
    ] {
        let p = &mut h.state_mut().palette;
        (p.kind, p.custom, p.tiles, p.prefabs) = (kind, false, false, false);
        p.filter.clear();
        h.state_mut().actions.push(Action::OpenTab(Tab::Palette));
        let ((), took) = time(|| h.run_steps(1));
        check(format!("standard {name} palette, first frame"), took, PALETTES);
        let frames: Vec<Duration> = (0..10).map(|_| time(|| h.run_steps(1)).1).collect();
        check(format!("standard {name} palette, frame"), median(frames), FRAME);
        // (A search opens every category with a find in it.)
        h.state_mut().palette.filter = "a".into();
        let ((), took) = time(|| h.run_steps(1));
        check(format!("standard {name} palette searched, first frame"), took, PALETTES);
        let frames: Vec<Duration> = (0..10).map(|_| time(|| h.run_steps(1)).1).collect();
        check(format!("standard {name} palette searched, frame"), median(frames), FRAME);
        if std::env::var_os("CAMPAIGN_SHOTS").is_some() {
            h.render().unwrap().save(dir.join(format!("{name}-palette.png"))).unwrap();
        }
    }
    h.state_mut().actions.push(Action::OpenTab(Tab::Resources));
    let ((), took) = time(|| h.run_steps(1));
    check("resource browser, first frame".into(), took, PALETTES);
    let frames: Vec<Duration> = (0..10).map(|_| time(|| h.run_steps(1)).1).collect();
    check("resource browser, frame".into(), median(frames), FRAME);
    let (now, most) = memory();
    println!("  memory: {now} MiB held, {most} MiB at most");
    assert!(over.is_empty(), "over budget:\n{}", over.join("\n"));
}

/// Advanced Search on the Original Campaign's first chapter (5,271
/// resources): a condition, the same again, three together, any field,
/// and a change made to what was found (which finds again). Needs the
/// game, not a GPU; run in release:
/// `cargo test --release -p mg-ui --test campaign_perf advanced_search -- --ignored --nocapture`.
#[test]
#[ignore]
fn advanced_search_stays_within_budget() {
    use mg_ui::search::{Act, AdvancedSearch, Condition, Kind};
    let root = mg_testkit::corpus!();
    let path = root.join("data/nwm/Chapter1.nwm");
    if !path.is_file() {
        println!("Chapter 1: not installed");
        return;
    }
    let mut over = Vec::new();
    let mut check = |what: String, took: Duration, budget: Duration| {
        let mark = if took > budget { "OVER" } else { "ok" };
        println!("  {what:52} {took:>12.2?}  (budget {budget:?}) {mark}");
        if took > budget {
            over.push(format!("{what}: {took:?} > {budget:?}"));
        }
    };
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.open_module(&path);
    println!("Chapter 1 ({} resources)", app.ws.as_ref().unwrap().module.len());
    let search = |app: &mut Moonglow, d: &mut AdvancedSearch| time(|| app.advanced_search(d)).1;
    let variables = || AdvancedSearch {
        first: Condition { kind: Kind::Variable, name: "*".into(), ..Default::default() },
        ..Default::default()
    };
    let mut d = variables();
    let took = search(&mut app, &mut d);
    check(format!("any variable, first search ({} found)", d.hits.len()), took, SEARCH);
    let mut d = variables();
    let took = search(&mut app, &mut d);
    check(format!("the same again ({} found)", d.hits.len()), took, SEARCH);
    // Fighters of 4 to 10 levels with Alertness and a rank in a skill.
    let mut d = AdvancedSearch {
        first: Condition { kind: Kind::Class, row: Some(4), levels: (4, 10), ..Default::default() },
        and: vec![
            Condition { kind: Kind::Feat, row: Some(0), ..Default::default() },
            Condition { kind: Kind::Skill, row: Some(0), ..Default::default() },
        ],
        ..Default::default()
    };
    let took = search(&mut app, &mut d);
    check(format!("three conditions together ({} found)", d.hits.len()), took, SEARCH);
    let mut d = AdvancedSearch {
        first: Condition { kind: Kind::Field, value: "guard".into(), ..Default::default() },
        ..Default::default()
    };
    let took = search(&mut app, &mut d);
    check(format!("any field holding \"guard\" ({} found)", d.hits.len()), took, SEARCH);
    // A variable set on every fighter, then found again.
    let mut d = AdvancedSearch {
        first: Condition { kind: Kind::Class, row: Some(4), ..Default::default() },
        ..Default::default()
    };
    search(&mut app, &mut d);
    d.act.act = Act::SetVariable;
    d.act.name = "nSearchPerf".into();
    d.act.value = "1".into();
    let ((), took) = time(|| app.change_found(&mut d));
    assert!(d.act.said.as_deref().is_some_and(|s| s.starts_with("Set")), "{:?}", d.act.said);
    check(format!("a variable set on {} fighters, found again", d.hits.len()), took, SEARCH * 2);
    let mut d = variables();
    let took = search(&mut app, &mut d);
    check(format!("any variable, after the change ({} found)", d.hits.len()), took, SEARCH);
    assert!(over.is_empty(), "over budget:\n{}", over.join("\n"));
}
