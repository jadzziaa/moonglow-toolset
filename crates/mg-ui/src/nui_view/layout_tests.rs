//! Panel size regression coverage: no game install or GPU is required.
use super::*;
use egui_kittest::{Harness, kittest::Queryable};

struct Fixture {
    doc: Value,
    settings: Settings,
    state: State,
    assets: skin::Assets,
    widths: Option<(f32, f32)>,
}

fn fixture() -> Fixture {
    let mut doc = mg_nui::window();
    doc["root"]["children"] = json!([{
        "type":"row","children":[{
            "type":"combo","id":"language","value":0,"height":30.0,
            "elements":[["First",0],["An intentionally long option label that must remain editable inside Properties",2147483647]]
        }]
    }]);
    Fixture {
        doc,
        settings: Settings::default(),
        state: State { selected: "/root/children/0/children/0".into(), ..Default::default() },
        assets: skin::Assets::default(),
        widths: None,
    }
}

fn show(ui: &mut Ui, f: &mut Fixture) {
    if let Some((left, right)) = f.widths.take() {
        for (name, width) in [("nui-palette", left), ("nui-inspector", right)] {
            let id = ui.id().with(name);
            ui.ctx().data_mut(|d| {
                d.insert_persisted(
                    id,
                    egui::containers::panel::PanelState {
                        outer_rect: egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 700.0),
                        ),
                    },
                )
            });
        }
    }
    editor(
        ui,
        &mut f.doc,
        &mut f.settings,
        &mut f.state,
        &crate::keys::Keymap::default(),
        &mut f.assets,
    );
}

fn assert_inspector_fits(h: &Harness<'_, Fixture>) {
    let (rect, content) = h.ctx.data_mut(|d| {
        d.get_temp::<(egui::Rect, egui::Vec2)>(egui::Id::new("nui-inspector-layout-test")).unwrap()
    });
    assert!(
        content.x <= rect.width() + 1.0,
        "Properties content grew beyond its allocated panel: content={content:?}, panel={rect:?}"
    );
    let heading = h.get_by_label("PROPERTIES").rect();
    let canvas = h.state().state.preview_rect.unwrap();
    assert!(
        heading.left() >= canvas.right(),
        "Properties is under the canvas: {heading:?} vs {canvas:?}"
    );
}

#[test]
fn nui_inspector_does_not_grow_under_canvas_with_dropdown_options() {
    let mut h =
        Harness::builder().with_size(egui::vec2(1130.0, 760.0)).build_ui_state(show, fixture());
    h.run();
    assert_inspector_fits(&h);
    let before = h.state().doc.clone();
    for (width, left, right) in
        [(1130.0, 300.0, 220.0), (800.0, 300.0, 360.0), (1380.0, 300.0, 360.0)]
    {
        h.state_mut().widths = Some((left, right));
        h.set_size(egui::vec2(width, 760.0));
        h.run();
        assert_inspector_fits(&h);
    }
    assert_eq!(h.state().doc, before, "Resizing panels must not change authored NUI");
}

#[test]
fn nui_inspector_bound_values_and_deep_layers_fit_narrow_panels() {
    let mut f = fixture();
    let mut node = json!({"type":"label","value":{"bind":"caption"},"height":30.0});
    f.state.selected = "/root".into();
    for _ in 0..12 {
        node = json!({"type":"col","children":[node]});
        f.state.selected.push_str("/children/0");
    }
    f.doc["root"] = node;
    f.settings.bindings.insert(
        "caption".into(),
        Binding {
            value: json!(
                "A very long bound string that should be edited without growing the panel sideways"
            ),
            ..Default::default()
        },
    );
    f.widths = Some((170.0, 220.0));
    let mut h = Harness::builder().with_size(egui::vec2(800.0, 760.0)).build_ui_state(show, f);
    h.run();
    assert_inspector_fits(&h);
    h.get_by_label("Bind formatting").click();
    h.run();
    assert_inspector_fits(&h);
}

#[test]
fn nui_dropdown_add_option_stays_within_nwn_integer_range() {
    for (highest, expected) in [(i32::MAX, 1), (7, 8)] {
        let mut f = fixture();
        f.doc["root"]["children"][0]["children"][0]["elements"][1][1] = json!(highest);
        let mut h = Harness::builder().with_size(egui::vec2(1130.0, 760.0)).build_ui_state(show, f);
        h.run();
        h.get_by_label("+ Add option").click();
        h.run();
        let elements = &h.state().doc["root"]["children"][0]["children"][0]["elements"];
        assert_eq!(elements[0][1], 0);
        assert_eq!(elements[1][1], highest, "Keep existing user IDs");
        assert_eq!(elements[2], json!(["Option", expected]));
        let errors: Vec<_> = mg_nui::validate(&h.state().doc, &h.state().settings)
            .into_iter()
            .filter(|d| d.severity == mg_nui::Severity::Error)
            .collect();
        assert!(errors.is_empty(), "Adding an option must produce valid NUI: {errors:?}");
        assert_inspector_fits(&h);
    }
}

/// Local editor rendering only; this does not prove NWN runtime parity.
#[test]
#[ignore]
fn nui_inspector_layout_screenshot() {
    mg_testkit::gpu::hold();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1130.0, 760.0))
        .wgpu()
        .build_ui_state(show, fixture());
    h.run();
    assert_inspector_fits(&h);
    // Add to the canonical preview set; scratch_dir would remove the other
    // screenshot test's evidence from this shared directory.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-output/nui-editor-preview");
    std::fs::create_dir_all(&dir).unwrap();
    h.render().unwrap().save(dir.join("inspector-options.png")).unwrap();
    h.state_mut().widths = Some((170.0, 220.0));
    h.set_size(egui::vec2(800.0, 760.0));
    h.run();
    assert_inspector_fits(&h);
    h.render().unwrap().save(dir.join("inspector-options-narrow.png")).unwrap();
}

/// Render an existing Creator-authored module without generating or editing NUI.
/// Explicit input/output paths keep this separate from synthetic test fixtures.
#[test]
#[ignore]
fn nui_saved_module_preview() {
    mg_testkit::gpu::hold();
    let module = std::env::var("NUI_PREVIEW_MODULE").expect("NUI_PREVIEW_MODULE");
    let name = std::env::var("NUI_PREVIEW_RESREF").expect("NUI_PREVIEW_RESREF");
    let output = std::env::var("NUI_PREVIEW_PNG").expect("NUI_PREVIEW_PNG");
    let path = std::path::Path::new(&module);
    let before = std::fs::read(path).unwrap();
    let mut app = Moonglow::new(
        Some(mg_resman::GameInstall::new(mg_testkit::corpus!(), None, "en")),
        Box::new(crate::NoDialogs::default()),
    );
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::open(path).unwrap()));
    let key = mg_nui::key(&name, ResType::JUI);
    assert!(app.ws.as_ref().unwrap().module.get(&key).is_some());
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 1040.0))
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| super::ui(app, ui, Some(key)), app);
    h.run_steps(3);
    assert!(h.state().nui_assets.loaded);
    assert!(h.state().nui_assets.issues.is_empty(), "{:?}", h.state().nui_assets.issues);
    h.render().unwrap().save(output).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), before, "Preview must not rewrite the module");
}
