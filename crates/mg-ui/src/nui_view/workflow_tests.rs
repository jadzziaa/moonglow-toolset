use super::*;
use egui_kittest::{Harness, kittest::Queryable};

fn harness(action: mg_nui::Action) -> Harness<'static, Moonglow> {
    let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
    app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
    super::super::create(&mut app, "workflow");
    app.actions.clear();
    let settings = Settings {
        actions: vec![mg_nui::Route { event: "click".into(), element: "mg_close".into(), action }],
        ..Default::default()
    };
    app.ws.as_mut().unwrap().module.set(mg_nui::key("workflow", ResType::TXT), settings.bytes());
    let mut h = Harness::builder().with_size(egui::vec2(1100.0, 1100.0)).build_ui_state(
        |ui, app: &mut Moonglow| {
            super::super::ui(app, ui, Some(mg_nui::key("workflow", ResType::JUI)));
            app.run_actions();
        },
        app,
    );
    h.run();
    h.get_by_label("Events & variants").click();
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
fn nui_bindings_page_adds_a_value_changed_handler_and_undoes_it() {
    let mut h = harness(mg_nui::Action::Close);
    h.get_by_label("Bindings").click();
    h.run();
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("volume");
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let before = settings(&h);
    h.get_by_label("+ Add handler").click();
    h.run();
    let after = settings(&h);
    assert!(
        after.actions.iter().any(|r| r.event == "watch"
            && r.element == "volume"
            && r.action == mg_nui::Action::Code)
    );
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run();
    assert_eq!(settings(&h), before);
}

#[test]
fn nui_binds_are_renamed_and_deleted_with_their_handlers() {
    let mut h = harness(mg_nui::Action::Close);
    h.get_by_label("Bindings").click();
    h.run();
    h.get_by_label("+ Create bind").click();
    h.run();
    h.get_by_label("Bind name").click();
    h.run();
    h.get_by_label("Bind name").type_text("volume");
    h.run();
    h.get_by_label("Create").click();
    h.run();
    h.get_by_label("+ Add handler").click();
    h.run();
    h.get_by_label("Name").click();
    h.run();
    h.get_by_label("Name").type_text("_2");
    h.run();
    h.get_by_label("Rename").click();
    h.run();
    let renamed = settings(&h);
    assert!(renamed.bindings.contains_key("volume_2") && !renamed.bindings.contains_key("volume"));
    assert!(renamed.actions.iter().any(|r| r.event == "watch" && r.element == "volume_2"));
    h.get_by_label("Delete").click();
    h.run();
    let deleted = settings(&h);
    assert!(deleted.bindings.is_empty());
    assert!(!deleted.actions.iter().any(|r| r.event == "watch"));
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
fn nui_binding_draft_rejects_duplicates() {
    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert("taken".into(), Binding { value: json!(true), ..Default::default() });
    let mut draft = BindDraft::new();
    assert!(draft.error(&bindings).is_some());
    draft.name = "  taken  ".into();
    assert!(draft.error(&bindings).is_some());
    draft.name = "bad\0name".into();
    assert!(draft.error(&bindings).is_some());
    draft.name = "fresh".into();
    assert!(draft.error(&bindings).is_none());
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
