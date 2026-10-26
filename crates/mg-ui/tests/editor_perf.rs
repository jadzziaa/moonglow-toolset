//! How fast the script editor is on a very large script (the game's
//! nwscript.nss, ~14,000 lines). Timing only; run with
//! `cargo test --release -p mg-ui --test editor_perf -- --ignored --nocapture`.

use std::time::Instant;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use mg_core::ResType;
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey};
use mg_ui::{Moonglow, NoDialogs, Tab};

#[test]
#[ignore]
fn editing_a_huge_script() {
    let root = mg_testkit::corpus!();
    let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let text = game.resman.get_named("nwscript", ResType::NSS).unwrap().into_owned();
    let dir = mg_testkit::scratch_dir("editor-perf");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let key = ResKey::parse("big", ResType::NSS).unwrap();
    m.set(key, text.clone());
    let path = dir.join("perf.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    let t = Instant::now();
    h.run();
    println!("open: {:?} ({} lines)", t.elapsed(), text.split(|b| *b == b'\n').count());

    let is_editor = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput
    };
    let editor =
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).find(is_editor).is_some();
    assert!(editor);
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).next().unwrap().click();
    h.run();
    let mut frames = Vec::new();
    for _ in 0..20 {
        let t = Instant::now();
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).next().unwrap().type_text("x");
        h.step();
        frames.push(t.elapsed());
    }
    frames.sort();
    println!(
        "typing a character: median {:?}, worst {:?}",
        frames[frames.len() / 2],
        frames.last().unwrap()
    );
    let t = Instant::now();
    h.step();
    println!("idle frame: {:?}", t.elapsed());

    // The same without the accessibility tree (which the desktop app builds
    // only for a screen reader), with tessellation.
    let mut app = h.into_state();
    let ctx = egui::Context::default();
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        ..Default::default()
    };
    let mut frames = Vec::new();
    for _ in 0..10 {
        let t = Instant::now();
        let out = ctx.run_ui(input(), |ui| app.ui(ui));
        let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
        frames.push(t.elapsed());
    }
    println!("plain egui frames: first {:?}, then {:?}", frames[0], &frames[5..]);
    ctx.memory_mut(|m| m.request_focus(egui::Id::new(("script", key))));
    let _ = ctx.run_ui(input(), |ui| app.ui(ui));
    let mut typing = Vec::new();
    for _ in 0..10 {
        let mut i = input();
        i.events.push(egui::Event::Text("x".into()));
        let t = Instant::now();
        let out = ctx.run_ui(i, |ui| app.ui(ui));
        let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
        typing.push(t.elapsed());
    }
    println!("plain egui, typing a character, in order: {typing:?}");
    let buf_len = app.ws.as_ref().unwrap().module.get(&key).map(|b| b.len());
    println!("module copy {buf_len:?} bytes (unsaved edits stay in the editor)");
}

/// Opening Select Texture on a placeable's Visuals page (GitHub issue 32):
/// the frame that lists every texture of the game, then the frames that
/// make the pictures in sight (a few milliseconds' worth each), and one
/// after typing a name to find.
#[test]
#[ignore]
fn opening_select_texture() {
    let root = mg_testkit::corpus!();
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let game = app.game.as_deref().unwrap();
    let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let data = game.resman.get_named("plc_chest1", ResType::UTP).unwrap().into_owned();
    let dir = mg_testkit::scratch_dir("select-texture-perf");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.set(key, data);
    let path = dir.join("perf.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 1000.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), "Visuals");
    let t = Instant::now();
    h.run();
    println!("the Visuals page, first frame (its models read): {:?}", t.elapsed());
    let t = Instant::now();
    h.step();
    println!("the Visuals page, a frame after: {:?}", t.elapsed());

    h.get_all_by_label("Select Texture…").next().unwrap().click();
    let t = Instant::now();
    h.step();
    println!("the click's frame: {:?}", t.elapsed());
    let t = Instant::now();
    h.step();
    let opened = t.elapsed();
    // (Frames until three in a row make nothing: quick ones.)
    let (mut frames, started) = (Vec::new(), Instant::now());
    while frames.len() < 300
        && (frames.len() < 3
            || frames[frames.len() - 3..].iter().any(|d| *d > std::time::Duration::from_millis(2)))
    {
        let t = Instant::now();
        h.step();
        frames.push(t.elapsed());
    }
    let making = &frames[..frames.len() - 3];
    let worst = making.iter().max().copied().unwrap_or_default();
    println!(
        "opened (every texture listed): {opened:?}; pictures in sight made over {} more \
         frames in {:?}, the longest {worst:?}",
        making.len(),
        started.elapsed() - frames[frames.len() - 3..].iter().sum::<std::time::Duration>()
    );
    let t = Instant::now();
    h.step();
    println!("a frame with them all made: {:?}", t.elapsed());
    // Scrolled a page at a time: the new pictures in sight made.
    let at = h.get_by_label("Select Texture").rect().left_top() + egui::vec2(300.0, 300.0);
    h.hover_at(at);
    let mut scrolled = Vec::new();
    for _ in 0..20 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -700.0),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        for _ in 0..3 {
            let t = Instant::now();
            h.step();
            scrolled.push(t.elapsed());
        }
    }
    scrolled.sort();
    println!(
        "scrolling a page at a time, 20 pages: frames median {:?}, worst {:?}",
        scrolled[scrolled.len() / 2],
        scrolled.last().unwrap()
    );
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    h.step();
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text("wood");
    let t = Instant::now();
    h.step();
    println!("a frame after typing \"wood\": {:?}", t.elapsed());
}
