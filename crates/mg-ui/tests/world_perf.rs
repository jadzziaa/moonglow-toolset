//! Performance budgets at persistent-world scale (`world/mod.rs`): what
//! builders report Aurora slow or failing at with 8,000 item blueprints,
//! 300 areas and 50 haks (inventories taking 20–30 s to open, Area
//! Properties freezing, palettes with CEP, haks over 2 GiB), and the rest of
//! a builder's day: the resource browser, verify, the content doctor,
//! where-used, script references, compiling, saving. Each step has a
//! budget, set at three to five times the first measurement (October 2026,
//! `docs/research/notes_scale.md`); the timings are printed. Needs the game (Tyrants of the Moonsea)
//! and a GPU; run in release:
//! `cargo test --release -p mg-ui --test world_perf -- --ignored --nocapture`.

mod world;

use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use mg_core::ResType;
use mg_module::palette::BlueprintKind;
use mg_resman::{GameInstall, ResKey};
use mg_ui::{Action, Moonglow, NoDialogs, Tab};

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

fn time<R>(f: impl FnOnce() -> R) -> (R, Duration) {
    let t = Instant::now();
    let r = f();
    (r, t.elapsed())
}

struct Budgets {
    over: Vec<String>,
}

impl Budgets {
    fn check(&mut self, what: impl Into<String>, took: Duration, budget: Duration) {
        let what = what.into();
        let mark = if took > budget { "OVER" } else { "ok" };
        println!("  {what:52} {took:>12.2?}  (budget {budget:?}) {mark}");
        if took > budget {
            self.over.push(format!("{what}: {took:?} > {budget:?}"));
        }
    }
}

/// The first frame after `setup` (which opens or changes something), then
/// the median of ten more.
fn frames(
    h: &mut Harness<'_, Moonglow>,
    b: &mut Budgets,
    what: &str,
    first: Duration,
    steady: Duration,
    setup: impl FnOnce(&mut Moonglow),
) {
    setup(h.state_mut());
    // The frame that opens it, and the one that first draws it.
    let ((), took) = time(|| h.run_steps(2));
    b.check(format!("{what}: first frames"), took, first);
    let v: Vec<Duration> = (0..10).map(|_| time(|| h.run_steps(1)).1).collect();
    b.check(format!("{what}: frame"), median(v), steady);
    // To see what was timed: WORLD_SHOTS=1 saves each view.
    if std::env::var_os("WORLD_SHOTS").is_some() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-output/world-shots");
        std::fs::create_dir_all(&dir).unwrap();
        let name: String =
            what.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
        h.render().unwrap().save(dir.join(format!("{name}.png"))).unwrap();
    }
}

fn close(h: &mut Harness<'_, Moonglow>, tab: &Tab) {
    let dock = &mut h.state_mut().dock;
    if let Some(at) = dock.find_tab(tab) {
        dock.remove_tab(at);
    }
    h.run_steps(1);
}

const S: fn(u64) -> Duration = Duration::from_secs;
const MS: fn(u64) -> Duration = Duration::from_millis;

#[test]
#[ignore]
fn persistent_world_stays_within_budget() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let Some(w) = world::world(&root) else { return };
    let mut b = Budgets { over: Vec::new() };
    println!("a persistent world ({} MB)", w.module.metadata().unwrap().len() / 1_000_000);

    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let install = GameInstall::new(&root, Some(w.user.clone()), "en");
    let (mut app, took) = time(|| Moonglow::new(Some(install), Box::new(NoDialogs::default())));
    b.check("index the game", took, S(1));
    app.set_render_state(rs.clone());
    let ((), took) = time(|| app.open_module(&w.module));
    let ws = app.ws.as_ref().unwrap();
    let layers = app.game.as_ref().unwrap().resman.layers().len();
    b.check(format!("open ({} resources, {layers} layers)", ws.module.len()), took, S(1));
    for (_, e) in app.log.entries.iter().take(6) {
        println!("    {e}");
    }

    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    frames(&mut h, &mut b, "module open (Module Properties)", MS(500), MS(33), |_| {});
    // The tree with its items listed.
    h.get_by_label_contains("Items (").click();
    frames(&mut h, &mut b, "module tree with 8,000 items", MS(500), MS(33), |_| {});
    h.get_by_label_contains("Items (").click();

    // Palettes.
    frames(&mut h, &mut b, "custom item palette (8,000)", MS(500), MS(33), |app| {
        app.actions.push(Action::OpenTab(Tab::Palette));
        app.palette.kind = BlueprintKind::Item;
        app.palette.custom = true;
    });
    // Every item listed: a filter that matches them all opens every
    // category.
    frames(&mut h, &mut b, "custom item palette, all 8,000 shown", MS(500), MS(33), |app| {
        app.palette.filter = "pw_i".into();
    });
    frames(&mut h, &mut b, "custom item palette, filtered", MS(500), MS(33), |app| {
        app.palette.filter = "pw_i07".into();
    });
    frames(&mut h, &mut b, "standard placeable palette (+10,000)", MS(500), MS(33), |app| {
        app.palette.filter = "pw placeable".into();
        app.palette.kind = BlueprintKind::Placeable;
        app.palette.custom = false;
    });
    h.state_mut().palette.filter.clear();
    close(&mut h, &Tab::Palette);

    // Inventories: a store with 1,000 items, its picker over 8,000.
    let store = ResKey::parse("pw_store", ResType::UTM).unwrap();
    h.state_mut().actions.push(Action::OpenTab(Tab::Blueprint(store)));
    h.run_steps(2);
    h.get_by_label("Inventory").click();
    frames(&mut h, &mut b, "store inventory with 1,000 items", S(2), MS(50), |_| {});
    close(&mut h, &Tab::Blueprint(store));
    // Appearance lists over the haks' grown 2DAs.
    let game_only = mg_resman::ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let (placeable_row, appearance_row) = world::first_new_rows(&game_only);
    let placeable = ResKey::parse("pw_p00000", ResType::UTP).unwrap();
    frames(&mut h, &mut b, "placeable", MS(500), MS(33), |app| {
        app.actions.push(Action::OpenTab(Tab::Blueprint(placeable)));
    });
    h.get_by_value(&format!("PW_Placeable_{placeable_row}")).click();
    frames(&mut h, &mut b, "placeable's 20,500 appearances listed", MS(500), MS(50), |_| {});
    h.key_press(egui::Key::Escape);
    close(&mut h, &Tab::Blueprint(placeable));
    let creature = ResKey::parse("pw_c00000", ResType::UTC).unwrap();
    frames(&mut h, &mut b, "creature", MS(500), MS(33), |app| {
        app.actions.push(Action::OpenTab(Tab::Blueprint(creature)));
    });
    h.get_by_value(&format!("PW_Creature_{appearance_row}")).click();
    frames(&mut h, &mut b, "creature's 17,100 appearances listed", MS(500), MS(50), |_| {});
    h.key_press(egui::Key::Escape);
    close(&mut h, &Tab::Blueprint(creature));

    // Areas: Area Properties, and the last area in the viewer (Aurora's
    // 32-bit build fails at the 152nd).
    let areas = h.state().ws.as_ref().unwrap().module.areas().unwrap();
    let (area, last) = (areas[0], *areas.last().unwrap());
    frames(&mut h, &mut b, "Area Properties (300 areas)", MS(500), MS(33), |app| {
        app.actions.push(Action::OpenTab(Tab::AreaProperties(area)));
    });
    close(&mut h, &Tab::AreaProperties(area));
    h.state_mut().actions.push(Action::OpenTab(Tab::Area(last)));
    let tiles = |h: &Harness<'_, Moonglow>| {
        h.state().area_views.get(&last).and_then(|v| v.model.as_ref()).map(|m| m.tiles.len())
    };
    let t = Instant::now();
    for _ in 0..10 {
        if tiles(&h).is_some() {
            break;
        }
        h.run_steps(1);
    }
    let took = t.elapsed();
    let n = tiles(&h).expect("the area view loads");
    b.check(format!("area {} ({last}, {n} tiles) in the viewer", areas.len()), took, S(1));
    let v: Vec<Duration> = (0..10).map(|_| time(|| h.run_steps(1)).1).collect();
    b.check("frame (area view)", median(v), MS(50));
    close(&mut h, &Tab::Area(last));

    // The resource browser over everything.
    frames(&mut h, &mut b, "resource browser", S(1), MS(33), |app| {
        app.actions.push(Action::OpenTab(Tab::Resources));
    });
    close(&mut h, &Tab::Resources);

    // Whole-module work.
    let app = h.state_mut();
    let (game, ws) = (app.game.as_ref().unwrap(), app.ws.as_mut().unwrap());
    let (r, took) = time(|| mg_module::palette::rebuild_custom_palettes(&mut ws.module, game));
    r.unwrap();
    b.check("rebuild custom palettes", took, S(1));
    let (missing, took) = time(|| mg_module::verify::missing(&ws.module, &game.resman));
    let (unused, t2) = time(|| mg_module::verify::unused(&ws.module));
    b.check(
        format!("verify ({} missing, {} unused)", missing.len(), unused.len()),
        took + t2,
        S(6),
    );
    let tlk = mg_module::doctor::TalkTables { base: game.tlk().entries.len(), custom: None };
    let (findings, took) = time(|| mg_module::doctor::examine(&ws.module, &game.resman, tlk));
    b.check(format!("content doctor ({} findings)", findings.len()), took, S(3));
    assert!(
        findings.iter().any(|f| f.check == "erf-size" && f.source == "hak:pw_hak49"),
        "the doctor names the hak over 2 GiB"
    );
    let item = ResKey::parse("pw_i00000", ResType::UTI).unwrap();
    let (usages, took) = time(|| mg_module::rename::usages(&ws.module, item));
    b.check(format!("where-used of an item ({} uses)", usages.len()), took, S(2));
    let (changed, took) = time(|| game.resman.changed_layers(|_| true));
    assert!(changed.is_empty());
    b.check("look for changed haks (Reload Resources)", took, MS(100));

    // Script references: every use of a widely used function.
    let script = ws
        .module
        .keys_of(ResType::NSS)
        .copied()
        .find(|k| ws.module.get(k).is_some_and(|d| d.windows(15).any(|w| w == b"GetObjectByTag(")))
        .unwrap();
    let text = String::from_utf8_lossy(ws.module.get(&script).unwrap()).into_owned();
    let at = text[..text.find("GetObjectByTag(").unwrap()].chars().count() + 2;
    app.actions.push(Action::OpenTab(Tab::Script(script)));
    h.run_steps(2);
    let app = h.state_mut();
    let (decl, took) = time(|| app.declaration_at(script, at));
    let decl = decl.expect("GetObjectByTag's declaration");
    b.check("go to definition", took, MS(100));
    let ((), took) = time(|| app.show_references(&decl));
    b.check("references of GetObjectByTag (every script)", took, S(1));
    let ((), took) = time(|| app.show_references(&decl));
    b.check("references again (indexed)", took, MS(200));
    close(&mut h, &Tab::Script(script));

    let scripts = h.state().ws.as_ref().unwrap().module.keys_of(ResType::NSS).count();
    let ((), took) = time(|| {
        h.state_mut().actions.push(Action::CompileScripts);
        h.run_steps(1);
    });
    b.check(format!("compile every script ({scripts})"), took, S(15));

    let ws = h.state_mut().ws.as_mut().unwrap();
    let dir = mg_testkit::scratch_dir("world-perf");
    let (bytes, took) = time(|| ws.snapshot().unwrap().to_archive_bytes().unwrap());
    std::fs::write(dir.join("world.mod"), &bytes).unwrap();
    b.check(format!("write the module ({} MB)", bytes.len() / 1_000_000), took, S(1));

    assert!(b.over.is_empty(), "over budget:\n{}", b.over.join("\n"));
}
