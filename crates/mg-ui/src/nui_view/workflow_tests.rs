use super::*;
use egui_kittest::{Harness, kittest::Queryable};

fn harness(action: mg_nui::Action) -> Harness<'static, Moonglow> {
    harness_with_renderer(action, false)
}

fn harness_with_renderer(action: mg_nui::Action, gpu: bool) -> Harness<'static, Moonglow> {
    let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
    super::super::create(&mut app, "workflow");
    app.actions.clear();
    let settings = Settings {
        actions: vec![mg_nui::Route { event: "click".into(), element: "mg_close".into(), action }],
        ..Default::default()
    };
    app.ws.as_mut().unwrap().module.set(mg_nui::key("workflow", ResType::TXT), settings.bytes());
    let builder = Harness::builder().with_size(egui::vec2(1100.0, 1100.0));
    let builder = if gpu { builder.wgpu() } else { builder };
    let mut h = builder.build_ui_state(
        |ui, app: &mut Moonglow| {
            super::super::ui(app, ui, Some(mg_nui::key("workflow", ResType::JUI)));
            app.run_actions();
        },
        app,
    );
    h.run();
    h.get_by_label("Views & events").click();
    h.run();
    h.get_by_label("Edit event").click();
    h.run();
    h
}

fn settings(h: &Harness<'_, Moonglow>) -> Settings {
    Settings::parse(
        h.state().ws.as_ref().unwrap().module.get(&mg_nui::key("workflow", ResType::TXT)).unwrap(),
    )
    .unwrap()
}

#[test]
fn nui_bindings_page_creates_watch_action_and_undoes_it() {
    let mut h = harness(mg_nui::Action::Close);
    h.get_by_label("Bindings").click();
    h.run();
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("close_when_changed");
    h.run();
    h.get_by_label("Create & select").click();
    h.run();
    assert_eq!(settings(&h).bindings["close_when_changed"].value, json!(""));
    let before = settings(&h);
    h.get_by_label("+ Add event").click();
    h.run();
    h.get_by_value("Choose action…").click();
    h.run();
    h.get_by_label("Close window").click();
    h.run();
    h.get_by_label("Save event").click();
    h.run();
    let after = settings(&h);
    assert!(after.actions.iter().any(|r| r.event == "watch"
        && r.element == "close_when_changed"
        && r.action == mg_nui::Action::Close));
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run();
    assert_eq!(settings(&h), before);
}

#[test]
fn nui_body_drag_does_not_move_floating_window() {
    let app = harness(mg_nui::Action::Close).into_state();
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 1000.0)).build_ui_state(
        |ui, (app, rect): &mut (Moonglow, egui::Rect)| {
            let shown = egui::Window::new("Floating Creator")
                .title_bar(false)
                .default_pos(egui::pos2(80.0, 60.0))
                .default_size(egui::vec2(1100.0, 800.0))
                .show(ui.ctx(), |ui| {
                    super::super::ui(app, ui, Some(mg_nui::key("workflow", ResType::JUI)))
                });
            *rect = shown.unwrap().response.rect;
            app.run_actions();
        },
        (app, egui::Rect::NOTHING),
    );
    h.run();
    let before = h.state().1.min;
    let from = h.get_by_label("NUI Creator").rect().center();
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(60.0, 30.0));
    h.run();
    h.drop_at(from + egui::vec2(60.0, 30.0));
    h.run();
    assert_eq!(h.state().1.min, before);
    h.get_by_label("Bindings").click();
    h.run();
    h.get_by_label("+ Create bind");
}

#[test]
fn nui_action_creates_and_selects_binding_in_one_undo() {
    let mut h = harness(mg_nui::Action::Set { bind: String::new(), value: Value::Null });
    let before = settings(&h);
    h.get_by_label("No binds yet. Create a named value for this action.");
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("  message  ");
    h.run();
    // Typing a draft does not add resources or incomplete bindings to Undo.
    assert_eq!(settings(&h), before);
    h.get_by_label("Create & select").click();
    h.run();
    assert_eq!(settings(&h), before);
    h.get_by_label("Save event").click();
    h.run();
    let after = settings(&h);
    assert_eq!(after.bindings["message"].value, json!(""));
    assert_eq!(
        after.actions[0].action,
        mg_nui::Action::Set { bind: "message".into(), value: json!("") }
    );
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    assert_eq!(settings(&h), before);
    h.state_mut().ws.as_mut().unwrap().redo().unwrap();
    assert_eq!(settings(&h), after);
}

#[test]
fn nui_toggle_creates_boolean_bind_and_cancel_keeps_document() {
    let mut h = harness(mg_nui::Action::Toggle { bind: String::new() });
    let before = settings(&h);
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("is_open");
    h.run();
    h.get_by_label("Cancel").click();
    h.run();
    assert_eq!(settings(&h), before);
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("is_open");
    h.run();
    h.get_by_label("Create & select").click();
    h.run();
    h.get_by_label("Save event").click();
    h.run();
    assert_eq!(settings(&h).bindings["is_open"].value, json!(false));
    assert_eq!(settings(&h).actions[0].action, mg_nui::Action::Toggle { bind: "is_open".into() });
    assert!(
        !mg_nui::validate(&mg_nui::window(), &settings(&h))
            .iter()
            .any(|d| d.severity == Severity::Error)
    );
}

#[test]
fn nui_binding_draft_rejects_duplicates_and_toggle_filters_values() {
    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert("taken".into(), Binding { value: json!(true), ..Default::default() });
    let mut draft = BindDraft::new(false);
    assert!(draft.error(&bindings).is_some());
    draft.name = "  taken  ".into();
    assert!(draft.error(&bindings).is_some());
    draft.name = "bad\0name".into();
    assert!(draft.error(&bindings).is_some());
    draft.name = "fresh".into();
    assert!(draft.error(&bindings).is_none());
    assert!(boolean_binding(&bindings["taken"]));
    for value in [json!("false"), json!(0), json!([true, "false"]), Value::Null] {
        assert!(!boolean_binding(&Binding { value, ..Default::default() }));
    }
    assert!(boolean_binding(&Binding { value: json!([true, false]), ..Default::default() }));
}

#[test]
fn nui_cancel_event_discards_its_new_binding() {
    let mut h = harness(mg_nui::Action::Set { bind: String::new(), value: Value::Null });
    let before = settings(&h);
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("temporary");
    h.run();
    h.get_by_label("Create & select").click();
    h.run();
    assert_eq!(settings(&h), before);
    h.get_by_label("Cancel event").click();
    h.run();
    assert_eq!(settings(&h), before);
    h.get_by_label("Edit event").click();
    h.run();
    h.get_by_label("No binds yet. Create a named value for this action.");
}

/// Local editor rendering; this does not establish NWN runtime parity.
#[test]
#[ignore]
fn nui_action_binding_screenshots() {
    mg_testkit::gpu::hold();
    let mut h = harness_with_renderer(
        mg_nui::Action::Set { bind: String::new(), value: Value::Null },
        true,
    );
    h.set_size(egui::vec2(1100.0, 860.0));
    h.run();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-output/nui-editor-preview");
    std::fs::create_dir_all(&dir).unwrap();
    h.render().unwrap().save(dir.join("action-bind-empty.png")).unwrap();
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("message");
    h.run();
    h.render().unwrap().save(dir.join("action-bind-create.png")).unwrap();
}

#[test]
fn nui_new_binding_types_fit_the_native_property_contract() {
    let mut doc = mg_nui::window();
    let mut settings = Settings::default();
    let mut children = Vec::new();
    for (kind, widget) in [
        (BindType::Text, "label"),
        (BindType::Integer, "combo"),
        (BindType::Decimal, "sliderf"),
        (BindType::Boolean, "check"),
        (BindType::Color, "color_picker"),
    ] {
        let name = kind.label().to_lowercase();
        settings
            .bindings
            .insert(name.clone(), Binding { value: kind.value(), ..Default::default() });
        let mut node = mg_nui::template(widget);
        node["value"] = json!({"bind":name});
        children.push(node);
    }
    settings.bindings.insert(
        "position".into(),
        Binding { value: BindType::Point.value(), ..Default::default() },
    );
    children[0]["draw_list"] = json!([{"type":6,"a":{"bind":"position"},"b":{"x":10.0,"y":10.0},"color":{"r":255,"g":255,"b":255,"a":255},"line_thickness":1.0}]);
    settings.bindings.insert(
        "bounds".into(),
        Binding { value: BindType::Rectangle.value(), ..Default::default() },
    );
    doc["geometry"] = json!({"bind":"bounds"});
    doc["root"]["children"] = children.into();
    let errors: Vec<_> = mg_nui::validate(&doc, &settings)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}
