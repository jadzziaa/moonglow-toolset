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
