//! Screenshots of windows for looking at their layout, rendered with wgpu
//! into `target/test-output/screens/`. Not checks; run by hand:
//! `cargo test -p mg-ui --test screens -- --ignored`.

use std::sync::Arc;

use egui_kittest::Harness;
use mg_core::ResType;
use mg_edit::GffPath;
use mg_module::script_wizard::{ClassLevel, Lists, Perform};
use mg_resman::{GameInstall, ResKey};
use mg_ui::script_wizard::{ScriptWizard, Step};
use mg_ui::{Moonglow, NoDialogs};

fn shoot(h: &mut Harness<'_, Moonglow>, dir: &std::path::Path, name: &str) {
    // Steps rather than runs: animated views keep repainting.
    h.run_steps(3);
    let image = h.render().expect("render");
    image.save(dir.join(format!("{name}.png"))).unwrap();
}

#[test]
#[ignore]
fn script_wizard_pages() {
    let root = mg_testkit::corpus!();
    let app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    let dir = mg_testkit::scratch_dir("screens");
    let lists = Arc::new(Lists::load(app.game.as_ref().unwrap()));
    let key = ResKey::parse("dlg", ResType::DLG).unwrap();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    for condition in [true, false] {
        let mut w = ScriptWizard::new(
            key,
            GffPath::root(),
            "Active",
            condition,
            lists.clone(),
            "sc_001".into(),
        );
        w.classes.push(ClassLevel { class: None, level: Some(5) });
        w.items.push("key_1".into());
        w.give_items.push("nw_wswls001".into());
        w.perform = Perform::Store { tag: "store_1".into(), appraise: true };
        let pages = w.pages.len();
        let prefix = if condition { "condition" } else { "action" };
        h.state_mut().script_wizard = Some(w.clone());
        shoot(&mut h, &dir, &format!("wizard-{prefix}-choose"));
        for i in 0..pages {
            w.step = Step::Page(i);
            h.state_mut().script_wizard = Some(w.clone());
            shoot(&mut h, &dir, &format!("wizard-{prefix}-{i:02}"));
        }
        w.step = Step::Name;
        h.state_mut().script_wizard = Some(w.clone());
        shoot(&mut h, &dir, &format!("wizard-{prefix}-name"));
    }
}

#[test]
#[ignore]
fn options_window() {
    use egui_kittest::kittest::Queryable;
    let dir = mg_testkit::scratch_dir("screens-options");
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.settings.script_style.colors[3] = Some([255, 128, 0]);
    app.options = Some(mg_ui::OptionsDraft::from_settings(&app.settings));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 900.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Script Editor").click();
    shoot(&mut h, &dir, "options");
}

#[test]
#[ignore]
fn model_viewer() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-model");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    for name in ["plc_a01", "c_golemerald"] {
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Model(
            ResKey::parse(name, ResType::MDL).unwrap(),
        )));
    }
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    shoot(&mut h, &dir, "model-viewer");
}

#[test]
#[ignore]
fn blueprint_preview() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens-model");
    let rs = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.set_render_state(rs.clone());
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Model(
        ResKey::parse("nw_halfdra002", ResType::UTC).unwrap(),
    )));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .renderer(egui_kittest::wgpu::WgpuTestRenderer::from_render_state(rs))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(4);
    shoot(&mut h, &dir, "blueprint-preview");
}

#[test]
#[ignore]
fn palettes() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens");
    let mut app =
        Moonglow::new(Some(GameInstall::new(&root, None, "en")), Box::new(NoDialogs::default()));
    app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Palette));
    app.palette.filter = "bear".into();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 760.0))
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    shoot(&mut h, &dir, "palettes");
}

#[test]
#[ignore]
fn blueprint_editors() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("screens");
    for (name, t, pages) in [
        ("nw_waypoint001", ResType::UTW, &["Basic", "Advanced"][..]),
        ("animalcriesday", ResType::UTS, &["Basic", "Positioning", "Advanced"][..]),
        ("x0_trapavg_shuri", ResType::UTT, &["Basic", "Trap", "Advanced"][..]),
        ("nw_verminbeet", ResType::UTE, &["Basic", "Creature List", "Advanced"][..]),
        ("nw_storebar01", ResType::UTM, &["Basic", "Inventory", "Restrictions"][..]),
        ("nw_door_ttr_01", ResType::UTD, &["Basic", "Lock", "Area Transition", "Advanced"][..]),
        ("plc_chest1", ResType::UTP, &["Basic", "Scripts", "Advanced"][..]),
        ("nw_wswmls010", ResType::UTI, &["General", "Appearance", "Properties"][..]),
        ("nw_aarcl004", ResType::UTI, &["General", "Appearance"][..]),
        ("nw_maarcl002", ResType::UTI, &["Appearance"][..]),
        (
            "nw_bartender",
            ResType::UTC,
            &["Basic", "Statistics", "Appearance", "Classes", "Skills", "Advanced"][..],
        ),
    ] {
        let mut app = Moonglow::new(
            Some(GameInstall::new(&root, None, "en")),
            Box::new(NoDialogs::default()),
        );
        let game = app.game.as_ref().unwrap();
        let data = game.resman.get_named(name, t).unwrap().into_owned();
        let mut m = mg_module::Module::new();
        m.set(ResKey::parse(name, t).unwrap(), data);
        let path = dir.join(format!("{name}.mod"));
        m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
        app.open_module(&path);
        let key = ResKey::parse(name, t).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(mg_ui::Tab::Blueprint(key)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1000.0, 760.0))
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        for page in pages {
            h.state_mut().blueprint_pages.insert(key, page);
            let page_name = page.to_lowercase().replace(' ', "-");
            shoot(&mut h, &dir, &format!("{name}-{page_name}"));
        }
    }
}
