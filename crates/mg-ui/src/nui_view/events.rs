//! Contextual, transactional event authoring; all routes use stock NUI events.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Target {
    Window,
    Control(String), // JSON pointer in the currently edited layout
    Id(String),
    Bind(String),
}

#[derive(Clone)]
pub(super) struct CodePreview {
    generated: Result<String, String>,
    scope: String,
    module_source: bool,
}

impl CodePreview {
    pub(super) fn new(settings: &Settings, route: Option<usize>, scope: &str) -> Self {
        let mut settings = settings.clone();
        if let Some(i) = route {
            settings.actions = settings.actions.get(i).cloned().into_iter().collect();
        }
        Self {
            generated: mg_nui::event_source(&settings),
            scope: scope.into(),
            module_source: false,
        }
    }
}

/// Read-only inspection. Neither preview nor Copy builds or saves a resource.
pub(super) fn code_preview(
    ctx: &egui::Context,
    app: &mut Moonglow,
    name: &str,
    preview: &mut Option<CodePreview>,
) {
    let Some(code) = preview.as_mut() else { return };
    let key = ResKey::parse(&format!("{name}_e"), ResType::NSS);
    let dirty = key.as_ref().and_then(|key| app.scripts.get(key)).filter(|b| b.is_dirty());
    let actual = dirty
        .map(|b| b.text.clone())
        .or_else(|| app.ws.as_ref()?.module.get(key.as_ref()?).map(crate::text::decode));
    let mut open = true;
    let mut dismiss = false;
    egui::Window::new(format!("Event script — {name}_e.nss"))
        .id(egui::Id::new(("nui-event-code", name)))
        .open(&mut open)
        .default_size(egui::vec2(780.0, 480.0))
        .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut code.module_source, false, "Generated code");
                ui.selectable_value(&mut code.module_source, true, "Module script");
            });
            let source = if code.module_source {
                ui.label(if dirty.is_some() {
                    "Full handler · unsaved changes from the script editor"
                } else {
                    "Full handler · current source in the module"
                });
                actual.as_deref().ok_or("No event script yet. Use Build & compile to create it.")
            } else {
                ui.label(&code.scope);
                ui.label("Generated preview at the time of opening. Not saved or compiled.");
                ui.weak("Module script shows the actual handler, including manual edits. Unsaved event drafts are included only when previewed from their form.");
                code.generated.as_deref().map_err(String::as_str)
            };
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(source.is_ok(), egui::Button::new("Copy code")).clicked()
                    && let Ok(text) = source
                {
                    ui.ctx().copy_text(text.to_owned());
                }
                if ui.add_enabled(actual.is_some(), egui::Button::new("Open in script editor")).clicked()
                    && let Some(key) = key
                {
                    app.actions.push(Action::OpenTab(Tab::Script(key)));
                    dismiss = true;
                }
                dismiss |= ui.button("Close preview").clicked();
            });
            ui.separator();
            match source {
                Ok(mut text) => {
                    let palette = crate::script_view::Palette::for_ui(&app.settings.script_style, ui);
                    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _: f32| {
                        let job = crate::script_view::highlight(text.as_str(), &palette);
                        ui.fonts_mut(|fonts| fonts.layout_job(job))
                    };
                    egui::ScrollArea::both().id_salt(("event-code", code.module_source)).show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut text)
                            .code_editor()
                            .layouter(&mut layouter)
                            .desired_width(f32::INFINITY)
                            .desired_rows(22));
                    });
                }
                Err(error) => { ui.label(error); }
            }
        });
    if !open || dismiss {
        *preview = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(doc: &Value, s: &Settings, target: Target) -> Draft {
        Draft {
            event: choices(&target)[0].into(),
            target: Some(target),
            action: Some(mg_nui::Action::Close),
            index: None,
            base: s.clone(),
            staged: s.clone(),
            doc: doc.clone(),
        }
    }

    #[test]
    fn event_code_preview_selects_one_route_without_changing_settings() {
        let s = Settings {
            actions: vec![
                mg_nui::Route {
                    event: "click".into(),
                    element: "close_button".into(),
                    action: mg_nui::Action::Close,
                },
                mg_nui::Route {
                    event: "click".into(),
                    element: "toggle_button".into(),
                    action: mg_nui::Action::Toggle { bind: "checked".into() },
                },
            ],
            ..Default::default()
        };
        let before = s.clone();
        let code = CodePreview::new(&s, Some(1), "Selected").generated.unwrap();
        assert!(code.contains("NuiSetBind"));
        assert!(!code.contains("NuiDestroy"));
        assert_eq!(
            CodePreview::new(&s, None, "All").generated.unwrap(),
            mg_nui::event_source(&s).unwrap()
        );
        assert_eq!(s, before);
    }

    #[test]
    fn event_targets_expose_ten_native_events_without_mixing_watch_and_window() {
        assert_eq!(choices(&Target::Window), ["open", "close"]);
        assert_eq!(choices(&Target::Bind("value".into())), ["watch"]);
        let control = choices(&Target::Id("button".into()));
        assert_eq!(control.len(), 7);
        for e in
            control.iter().chain(choices(&Target::Window)).chain(choices(&Target::Bind("v".into())))
        {
            assert_ne!(label(e), *e);
        }
    }

    #[test]
    fn event_candidate_assigns_unique_id_only_on_save_and_rejects_stale_or_duplicate() {
        let doc = json!({"root":{"type":"col", "children":[
            {"type":"button", "label":"Run"},
            {"type":"button", "label":"Other", "id":"button_event"}
        ]}});
        let s = Settings::default();
        let mut d = draft(&doc, &s, Target::Control("/root/children/0".into()));
        let state = State::default();
        let (saved, settings) = candidate(&d, &doc, &s, &state).unwrap();
        assert!(doc["root"]["children"][0].get("id").is_none());
        assert_eq!(saved["root"]["children"][0]["id"], "button_event_2");
        assert_eq!(settings.actions[0].element, "button_event_2");
        assert!(candidate(&d, &saved, &s, &state).unwrap_err().contains("document changed"));
        d = draft(&saved, &settings, Target::Control("/root/children/0".into()));
        assert!(candidate(&d, &saved, &settings, &state).unwrap_err().contains("Duplicate"));
        d.index = Some(0);
        assert!(candidate(&d, &saved, &settings, &state).is_ok());
        d.action = None;
        assert!(candidate(&d, &saved, &settings, &state).is_err());
    }

    #[test]
    fn event_candidate_validates_watch_cycles_and_alternate_layout_ids() {
        let doc = mg_nui::window();
        let mut s = Settings::default();
        s.bindings.insert(
            "checked".into(),
            mg_nui::Binding { value: json!(false), ..Default::default() },
        );
        let mut d = draft(&doc, &s, Target::Bind("checked".into()));
        d.action = Some(mg_nui::Action::Toggle { bind: "checked".into() });
        assert!(candidate(&d, &doc, &s, &State::default()).is_err());
        let alt = json!({"root":{"type":"button", "label":"Alternate"}});
        s.views.insert("alternate".into(), alt["root"].clone());
        let state = State {
            main_doc: Some(doc),
            edit_view: Some("alternate".into()),
            ..Default::default()
        };
        let d = draft(&alt, &s, Target::Control("/root".into()));
        let (saved, result) = candidate(&d, &alt, &s, &state).unwrap();
        assert_eq!(result.views["alternate"], saved["root"]);
        assert_eq!(result.actions[0].element, saved["root"]["id"].as_str().unwrap());
    }

    #[test]
    fn layout_event_requires_explicit_existing_group_or_whole_window() {
        let doc = mg_nui::window();
        let mut s = Settings::default();
        s.views.insert("details".into(), mg_nui::template("col"));
        let mut d = draft(&doc, &s, Target::Window);
        d.action = Some(mg_nui::Action::View { group: String::new(), view: "details".into() });
        assert!(candidate(&d, &doc, &s, &State::default()).is_err());
        d.action = Some(mg_nui::Action::View { group: "_window_".into(), view: "details".into() });
        assert!(candidate(&d, &doc, &s, &State::default()).is_ok());
    }
}

#[derive(Clone)]
struct Draft {
    target: Option<Target>,
    event: String,
    action: Option<mg_nui::Action>,
    index: Option<usize>,
    base: Settings,
    staged: Settings,
    doc: Value,
}

fn label(event: &str) -> &str {
    match event {
        "click" => "Clicked",
        "open" => "Window opened",
        "close" => "Window closed",
        "watch" => "Value changed",
        "mousedown" => "Mouse pressed",
        "mouseup" => "Mouse released",
        "mousescroll" => "Mouse wheel",
        "focus" => "Focus gained",
        "blur" => "Focus lost",
        "range" => "Visible range changed",
        other => other,
    }
}

fn explanation(event: &str) -> &str {
    match event {
        "click" => "Runs when this control sends a click event.",
        "open" => "Runs when the window opens. No control ID is needed.",
        "close" => "Runs after the window closes; it is not the action that closes it.",
        "watch" => "Runs when the bind value changes. Watching is enabled automatically on build.",
        "focus" | "blur" => "Focus events come from controls that support keyboard focus.",
        "range" => "List range events report the visible row range. Verify the behavior in NWN.",
        _ => "Native mouse event. Availability depends on the control and game client.",
    }
}

fn choices(target: &Target) -> &'static [&'static str] {
    match target {
        Target::Window => &["open", "close"],
        Target::Bind(_) => &["watch"],
        _ => &["click", "mousedown", "mouseup", "mousescroll", "focus", "blur", "range"],
    }
}

fn route_target(route: &mg_nui::Route) -> Target {
    match route.event.as_str() {
        "open" | "close" => Target::Window,
        "watch" => Target::Bind(route.element.clone()),
        _ => Target::Id(route.element.clone()),
    }
}

fn element(target: &Target, doc: &Value) -> String {
    match target {
        Target::Window => String::new(),
        Target::Id(n) | Target::Bind(n) => n.clone(),
        Target::Control(path) => {
            doc.pointer(path).and_then(|n| n["id"].as_str()).unwrap_or_default().into()
        }
    }
}

fn target_name(target: &Target, doc: &Value) -> String {
    match target {
        Target::Window => "Window".into(),
        Target::Bind(n) => format!("Bind: {n}"),
        Target::Id(n) => format!("Control: {n}"),
        Target::Control(p) => doc.pointer(p).map_or("Missing control".into(), design::node_name),
    }
}

fn action_name(action: &mg_nui::Action) -> String {
    match action {
        mg_nui::Action::Close => "Close window".into(),
        mg_nui::Action::Toggle { bind } => format!("Toggle {bind}"),
        mg_nui::Action::Set { bind, .. } => format!("Set {bind}"),
        mg_nui::Action::View { group, view } => {
            format!("Show {view} in {}", if group == "_window_" { "whole window" } else { group })
        }
    }
}

fn action_editor(ui: &mut Ui, draft: &mut Draft, doc: &Value, state: &mut State) {
    ui.strong("Then");
    let mut kind = match draft.action {
        None => 0,
        Some(mg_nui::Action::Close) => 1,
        Some(mg_nui::Action::Set { .. }) => 2,
        Some(mg_nui::Action::Toggle { .. }) => 3,
        Some(mg_nui::Action::View { .. }) => 4,
    };
    let old = kind;
    let names =
        ["Choose action…", "Close window", "Set bind value", "Toggle bind", "Replace group layout"];
    egui::ComboBox::from_id_salt("event-action")
        .selected_text(names[kind])
        .width(ui.available_width().clamp(0.0, 320.0))
        .show_ui(ui, |ui| {
            for (i, name) in names.iter().enumerate().skip(1) {
                ui.selectable_value(&mut kind, i, *name);
            }
        });
    if kind != old {
        draft.action = Some(match kind {
            2 => mg_nui::Action::Set { bind: String::new(), value: Value::Null },
            3 => mg_nui::Action::Toggle { bind: String::new() },
            4 => mg_nui::Action::View { group: String::new(), view: String::new() },
            _ => mg_nui::Action::Close,
        });
    }
    let excluded =
        if let Some(Target::Bind(name)) = &draft.target { Some(name.as_str()) } else { None };
    match &mut draft.action {
        Some(mg_nui::Action::Set { bind, value }) => {
            if workflow::bind_select(
                ui,
                "Target bind",
                bind,
                &mut draft.staged.bindings,
                false,
                excluded,
            ) && let Some(b) = draft.staged.bindings.get(bind)
            {
                *value = b.value.clone();
            }
            if draft.staged.bindings.contains_key(bind) {
                ui.label("Set to");
                fields::value(ui, value, 0);
            }
        }
        Some(mg_nui::Action::Toggle { bind }) => {
            workflow::bind_select(
                ui,
                "Target bind",
                bind,
                &mut draft.staged.bindings,
                true,
                excluded,
            );
        }
        Some(mg_nui::Action::View { group, view }) => {
            layouts::target_editor_draft(ui, doc, &mut draft.staged, state, group, view);
        }
        Some(mg_nui::Action::Close) => {
            ui.weak("Closes this NUI window.");
        }
        None => {}
    }
}

fn candidate(
    draft: &Draft,
    doc: &Value,
    s: &Settings,
    state: &State,
) -> Result<(Value, Settings), String> {
    if &draft.base != s || &draft.doc != doc {
        return Err(
            "The document changed. Cancel and reopen this event to keep those changes.".into()
        );
    }
    let target = draft.target.as_ref().ok_or("Choose who receives this event.")?;
    let action = draft.action.clone().ok_or("Choose what this event should do.")?;
    if !choices(target).contains(&draft.event.as_str()) {
        return Err("Choose an event for this target.".into());
    }
    let mut doc = doc.clone();
    let mut element = element(target, &doc);
    if let Target::Control(path) = target {
        let node = doc.pointer(path).ok_or("This control no longer exists.")?;
        if element.is_empty() {
            let base = format!("{}_event", node["type"].as_str().unwrap_or("control"));
            let mut ids = mg_nui::element_ids(state.main_doc.as_ref().unwrap_or(&doc));
            ids.extend(mg_nui::element_ids(&doc));
            for v in draft.staged.views.values() {
                ids.extend(mg_nui::element_ids(v));
            }
            element = base.clone();
            let mut n = 2;
            while ids.contains(&element) {
                element = format!("{base}_{n}");
                n += 1;
            }
            doc.pointer_mut(path).unwrap()["id"] = json!(element);
        }
    }
    let route = mg_nui::Route { event: draft.event.clone(), element, action };
    let mut result = draft.staged.clone();
    if let Some(i) = draft.index {
        result.actions[i] = route;
    } else {
        result.actions.push(route);
    }
    let validation_doc = if let Some(view) = &state.edit_view {
        result.views.insert(view.clone(), doc["root"].clone());
        state.main_doc.as_ref().unwrap_or(&doc)
    } else {
        &doc
    };
    if let Some(error) = mg_nui::validate(validation_doc, &result)
        .into_iter()
        .find(|d| d.severity == Severity::Error && d.path == "/actions")
    {
        return Err(error.message);
    }
    Ok((doc, result))
}

pub(super) fn inspector(ui: &mut Ui, doc: &mut Value, s: &mut Settings, state: &mut State) {
    let target = if state.selected.is_empty() {
        Target::Window
    } else {
        Target::Control(state.selected.clone())
    };
    egui::CollapsingHeader::new("Events")
        .default_open(true)
        .show(ui, |ui| editor(ui, doc, s, state, Some(target)));
    ui.separator();
}

pub(super) fn editor(
    ui: &mut Ui,
    doc: &mut Value,
    s: &mut Settings,
    state: &mut State,
    scope: Option<Target>,
) {
    ui.push_id(("events", format!("{:?}", scope), &state.edit_view.clone()), |ui| {
        let draft_id = ui.make_persistent_id("draft");
        let mut draft = ui.ctx().data_mut(|d| d.get_temp::<Draft>(draft_id));
        let indices: Vec<_> = s
            .actions
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                scope.as_ref().is_none_or(|t| match t {
                    Target::Control(_) => {
                        !element(t, doc).is_empty()
                            && !matches!(r.event.as_str(), "open" | "close" | "watch")
                            && r.element == element(t, doc)
                    }
                    _ => route_target(r) == *t,
                })
            })
            .map(|(i, _)| i)
            .collect();
        if scope.is_none() {
            ui.heading("Events & actions");
        }
        if draft.is_none() {
            if indices.is_empty() {
                ui.weak("No events yet.");
            }
            for i in indices {
                let route = s.actions[i].clone();
                ui.push_id(i, |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.strong(label(&route.event));
                        if scope.is_none() {
                            ui.label(target_name(&route_target(&route), doc));
                        }
                        ui.label(action_name(&route.action));
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Preview code").clicked() {
                                state.event_code = Some(CodePreview::new(
                                    s,
                                    Some(i),
                                    &format!(
                                        "{} · {} · selected event only",
                                        label(&route.event),
                                        target_name(&route_target(&route), doc)
                                    ),
                                ));
                            }
                            if ui.button("Edit event").clicked() {
                                draft = Some(Draft {
                                    target: Some(
                                        scope.clone().unwrap_or_else(|| route_target(&route)),
                                    ),
                                    event: route.event.clone(),
                                    action: Some(route.action.clone()),
                                    index: Some(i),
                                    base: s.clone(),
                                    staged: s.clone(),
                                    doc: doc.clone(),
                                });
                            }
                            if ui.button("Remove event").clicked() {
                                // Defer deletion so the other row indices remain valid this frame.
                                ui.ctx().data_mut(|d| d.insert_temp(draft_id.with("remove"), i));
                            }
                        });
                        if let mg_nui::Action::View { group, view } = &route.action
                            && s.views.contains_key(view)
                            && ui.button("Edit layout on canvas").clicked()
                        {
                            state.layout_target = group.clone();
                            layouts::edit(state, Some(view.clone()));
                        }
                    })
                });
            }
            if ui.button("+ Add event").clicked() {
                draft = Some(Draft {
                    event: scope.as_ref().map_or("click", |t| choices(t)[0]).into(),
                    target: scope.clone(),
                    action: None,
                    index: None,
                    base: s.clone(),
                    staged: s.clone(),
                    doc: doc.clone(),
                });
            }
        }
        let mut finish = false;
        if let Some(draft) = draft.as_mut() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(if draft.index.is_some() { "Edit event" } else { "New event" });
                if scope.is_none() {
                    ui.label("For");
                    let mut target = draft.target.clone();
                    egui::ComboBox::from_id_salt("event-target")
                        .selected_text(
                            target
                                .as_ref()
                                .map_or("Choose target…".into(), |t| target_name(t, doc)),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut target, Some(Target::Window), "Window");
                            let mut ids = mg_nui::element_ids(doc);
                            for v in s.views.values() {
                                ids.extend(mg_nui::element_ids(v));
                            }
                            for id in ids {
                                ui.selectable_value(
                                    &mut target,
                                    Some(Target::Id(id.clone())),
                                    format!("Control: {id}"),
                                );
                            }
                            for name in s.bindings.keys() {
                                ui.selectable_value(
                                    &mut target,
                                    Some(Target::Bind(name.clone())),
                                    format!("Bind: {name}"),
                                );
                            }
                        });
                    if target != draft.target {
                        draft.event = target.as_ref().map_or("click", |t| choices(t)[0]).into();
                        draft.target = target;
                    }
                    ui.weak("You can also select a control in Design to add its event directly.");
                } else if let Some(scope) = &scope {
                    ui.label(target_name(scope, doc));
                }
                if let Some(target) = &draft.target {
                    ui.strong("When");
                    egui::ComboBox::from_id_salt("event-trigger")
                        .selected_text(label(&draft.event))
                        .width(ui.available_width().clamp(0.0, 320.0))
                        .show_ui(ui, |ui| {
                            for name in choices(target) {
                                ui.selectable_value(&mut draft.event, (*name).into(), label(name));
                            }
                        });
                    ui.weak(explanation(&draft.event));
                    action_editor(ui, draft, doc, state);
                }
                let result = candidate(draft, doc, s, state);
                if let Err(error) = &result {
                    ui.weak(error);
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.add_enabled(result.is_ok(), egui::Button::new("Preview code")).clicked()
                        && let Ok((_, settings)) = &result
                    {
                        let index = draft.index.unwrap_or(settings.actions.len() - 1);
                        state.event_code = Some(CodePreview::new(
                            settings,
                            Some(index),
                            "Unsaved event draft · selected event only",
                        ));
                    }
                    if ui.add_enabled(result.is_ok(), egui::Button::new("Save event")).clicked()
                        && let Ok((new_doc, settings)) = result
                    {
                        *doc = new_doc;
                        *s = settings;
                        finish = true;
                    }
                    if ui.button("Cancel event").clicked() {
                        finish = true;
                    }
                });
                ui.weak("New binds and layouts are saved together with this event.");
            });
        }
        if let Some(i) = ui.ctx().data_mut(|d| d.remove_temp::<usize>(draft_id.with("remove"))) {
            s.actions.remove(i);
        }
        ui.ctx().data_mut(|d| {
            if finish {
                d.remove::<Draft>(draft_id);
            } else if let Some(draft) = draft {
                d.insert_temp(draft_id, draft);
            }
        });
        if finish {
            ui.ctx().request_repaint();
        }
    });
}
