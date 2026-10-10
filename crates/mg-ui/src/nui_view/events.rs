//! Event handlers. Each one is a section of the window's event script
//! (`<name>_e.nss`), where you write what happens; the stock NUI event
//! types decide when it runs.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Target {
    Window,
    Control(String), // JSON pointer in the currently edited layout
    Id(String),
    Bind(String),
}

/// Opens the event script, at a handler, after adding code to it.
#[derive(Clone)]
pub(super) struct EventScript {
    /// The handlers, as they are this frame.
    pub(super) settings: Settings,
    pub(super) handler: Option<String>,
    pub(super) code: Option<String>,
}

/// Brings the event script up to date with the handlers (keeping what is
/// written in them), adds any code asked for, and opens it in the script
/// editor at the handler. A script written by hand opens as it is.
pub(super) fn open_script(app: &mut Moonglow, name: &str, ask: EventScript) -> Option<Edit> {
    let key = ResKey::parse(&format!("{name}_e"), ResType::NSS)?;
    let current = app
        .scripts
        .get(&key)
        .map(|b| b.text.clone())
        .or_else(|| app.ws.as_ref()?.module.get(&key).map(crate::text::decode));
    let ours = current.as_deref().is_none_or(|t| t.contains(mg_nui::BEGIN.trim_end()));
    let mut text = match &current {
        Some(text) if !ours => text.clone(),
        _ => mg_nui::merge_events(&ask.settings, current.as_deref()).ok()?,
    };
    if let (Some(handler), Some(code)) = (&ask.handler, &ask.code)
        && let Some(with) = mg_nui::insert_code(&text, handler, code)
    {
        text = with;
    }
    let jump = ask.handler.as_deref().and_then(|h| mg_nui::handler_offset(&text, h));
    app.script_tools.jump = Some((key, jump.unwrap_or(0)));
    app.actions.push(Action::OpenTab(Tab::Script(key)));
    if current.as_deref() == Some(text.as_str()) {
        return None;
    }
    // Unsaved work in an open editor stays there; otherwise the module changes.
    if let Some(buf) = app.scripts.get_mut(&key).filter(|b| b.is_dirty()) {
        buf.text = text;
        return None;
    }
    Some(Edit::SetResource { key, data: Some(crate::text::encode(&text)) })
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
        "click" => "Runs when the button is clicked.",
        "open" => "Runs when the window opens.",
        "close" => {
            "Runs when the player closes the window with its X. Closing it from a script does not send it."
        }
        "watch" => {
            "Runs when the value changes, from the player or from a script setting it (at once, inside that script). sElement is the bind's name."
        }
        "focus" | "blur" => "Runs when the text input gains or loses the keyboard.",
        "range" => "Runs when the rows in view change (scrolling, and when the window opens).",
        _ => "Runs on the mouse over this control; NuiGetEventPayload has the button and position.",
    }
}

/// The node an element ID names, in the window or one of its layouts.
fn node_by_id<'a>(doc: &'a Value, s: &'a Settings, id: &str) -> Option<&'a Value> {
    fn find<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
        match v {
            Value::Object(o) if o.get("id").and_then(Value::as_str) == Some(id) => Some(v),
            Value::Object(o) => o.values().find_map(|v| find(v, id)),
            Value::Array(a) => a.iter().find_map(|v| find(v, id)),
            _ => None,
        }
    }
    find(doc, id).or_else(|| s.views.values().find_map(|v| find(v, id)))
}

fn node<'a>(target: &Target, doc: &'a Value, s: &'a Settings) -> Option<&'a Value> {
    match target {
        Target::Control(path) => doc.pointer(path),
        Target::Id(id) => node_by_id(doc, s, id),
        _ => None,
    }
}

/// The events a target sends in NWN EE 8193.37: a button's click, a text
/// input's focus, a list's range, the mouse over any control, and a value's
/// change for a control whose value is bound.
fn choices(target: &Target, doc: &Value, s: &Settings) -> Vec<&'static str> {
    match target {
        Target::Window => vec!["open", "close"],
        Target::Bind(_) => vec!["watch"],
        _ => {
            let node = node(target, doc, s);
            let mut events = Vec::new();
            if node.is_some_and(|n| n["value"]["bind"].is_string()) {
                events.push("watch");
            }
            match node.and_then(|n| n["type"].as_str()) {
                Some("button" | "button_image" | "button_select") => events.push("click"),
                Some("textedit") => events.extend(["focus", "blur"]),
                Some("list") => events.push("range"),
                _ => {}
            }
            events.extend(["mousedown", "mouseup", "mousescroll"]);
            events
        }
    }
}

fn route_target(route: &mg_nui::Route) -> Target {
    match route.event.as_str() {
        "open" | "close" => Target::Window,
        "watch" => Target::Bind(route.element.clone()),
        _ => Target::Id(route.element.clone()),
    }
}

/// The element a new handler of this target names: the bind for a value's
/// change, else the control's ID (one made up for it if it has none).
fn element(target: &Target, event: &str, doc: &mut Value, s: &Settings, state: &State) -> String {
    if event == "watch"
        && let Some(bind) = node(target, doc, s).and_then(|n| n["value"]["bind"].as_str())
    {
        return bind.into();
    }
    match target {
        Target::Window => String::new(),
        Target::Id(n) | Target::Bind(n) => n.clone(),
        Target::Control(path) => {
            let Some(node) = doc.pointer(path) else { return String::new() };
            if let Some(id) = node["id"].as_str() {
                return id.into();
            }
            let base = format!("{}_event", node["type"].as_str().unwrap_or("control"));
            let mut ids = mg_nui::element_ids(state.main_doc.as_ref().unwrap_or(doc));
            ids.extend(mg_nui::element_ids(doc));
            for v in s.views.values() {
                ids.extend(mg_nui::element_ids(v));
            }
            let mut id = base.clone();
            let mut n = 2;
            while ids.contains(&id) {
                id = format!("{base}_{n}");
                n += 1;
            }
            doc.pointer_mut(path).unwrap()["id"] = json!(id);
            id
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

/// Ready-made code to add to a handler: what the old actions did.
fn insert_menu(ui: &mut Ui, route: &mg_nui::Route, s: &Settings) -> Option<String> {
    let with = |action| mg_nui::handler_code(&mg_nui::Route { action, ..route.clone() }, s).ok();
    let mut code = None;
    ui.menu_button("Insert…", |ui| {
        if ui.button("Close window").clicked() {
            code = with(mg_nui::Action::Close);
        }
        ui.menu_button("Set bind", |ui| {
            for (bind, b) in &s.bindings {
                if ui.button(bind).clicked() {
                    code = with(mg_nui::Action::Set { bind: bind.clone(), value: b.value.clone() });
                }
            }
        });
        ui.menu_button("Toggle bind", |ui| {
            for (bind, b) in &s.bindings {
                let flag = b.value.is_boolean()
                    || b.value.as_array().is_some_and(|a| a.iter().all(Value::is_boolean));
                if flag && ui.button(bind).clicked() {
                    code = with(mg_nui::Action::Toggle { bind: bind.clone() });
                }
            }
        });
        ui.menu_button("Show layout", |ui| {
            for view in s.views.keys() {
                if ui.button(view).clicked() {
                    let group = layouts::target_for(s, view).unwrap_or_else(|| "_window_".into());
                    code = with(mg_nui::Action::View { group, view: view.clone() });
                }
            }
        });
    });
    code
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
        if scope.is_none() {
            ui.heading("Events");
            ui.weak("Each event you handle is a section of the event script, where you write what it does.");
        }
        let shown = |r: &mg_nui::Route, doc: &Value| match &scope {
            None => true,
            Some(Target::Control(path)) => {
                let node = doc.pointer(path);
                let id = node.and_then(|n| n["id"].as_str());
                let bind = node.and_then(|n| n["value"]["bind"].as_str());
                (r.event == "watch" && bind == Some(r.element.as_str()))
                    || (!matches!(r.event.as_str(), "open" | "close" | "watch")
                        && id == Some(r.element.as_str()))
            }
            Some(t) => route_target(r) == *t,
        };
        let mut remove = None;
        for (i, route) in s.actions.clone().iter().enumerate().filter(|(_, r)| shown(r, doc)) {
            ui.push_id(i, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(label(&route.event));
                    if scope.is_none() || route.event == "watch" {
                        ui.weak(target_name(&route_target(route), doc));
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    let handler = mg_nui::handler_key(route);
                    if ui.button("Edit code").on_hover_text("Open the event script here").clicked() {
                        state.event_script = Some(EventScript {
                            settings: s.clone(),
                            handler: Some(handler.clone()),
                            code: None,
                        });
                    }
                    if let Some(code) = insert_menu(ui, route, s) {
                        state.event_script =
                            Some(EventScript { settings: s.clone(), handler: Some(handler), code: Some(code) });
                    }
                    if ui.button("Remove").on_hover_text("What you wrote for it stays in the script, commented out").clicked() {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            s.actions.remove(i);
        }
        // Adding a handler: who sends the event, and which one.
        let add_id = ui.make_persistent_id("add");
        let (mut target, mut event) = ui
            .ctx()
            .data_mut(|d| d.get_temp::<(Option<Target>, String)>(add_id))
            .unwrap_or_default();
        if scope.is_some() {
            target = scope.clone();
        }
        ui.horizontal_wrapped(|ui| {
            if scope.is_none() {
                egui::ComboBox::from_id_salt("event-target")
                    .selected_text(target.as_ref().map_or("Choose…".into(), |t| target_name(t, doc)))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut target, Some(Target::Window), "Window");
                        let mut ids = mg_nui::element_ids(doc);
                        for v in s.views.values() {
                            ids.extend(mg_nui::element_ids(v));
                        }
                        for id in ids {
                            ui.selectable_value(&mut target, Some(Target::Id(id.clone())), format!("Control: {id}"));
                        }
                        for name in s.bindings.keys() {
                            ui.selectable_value(&mut target, Some(Target::Bind(name.clone())), format!("Bind: {name}"));
                        }
                    });
            }
            let Some(t) = &target else { return };
            // Only events not handled yet.
            let free: Vec<_> = choices(t, doc, s)
                .into_iter()
                .filter(|e| {
                    let element = match (t, *e) {
                        (Target::Window, _) => String::new(),
                        (_, "watch") => node(t, doc, s)
                            .and_then(|n| n["value"]["bind"].as_str())
                            .map_or_else(|| if let Target::Bind(b) = t { b.clone() } else { String::new() }, Into::into),
                        (Target::Control(p), _) => doc.pointer(p).and_then(|n| n["id"].as_str()).unwrap_or_default().into(),
                        (Target::Id(id) | Target::Bind(id), _) => id.clone(),
                    };
                    let key = mg_nui::handler_key(&mg_nui::Route {
                        event: (*e).into(),
                        element,
                        action: mg_nui::Action::Code,
                    });
                    !s.actions.iter().any(|r| mg_nui::handler_key(r) == key)
                })
                .collect();
            if free.is_empty() {
                ui.weak("Every event it sends is handled.");
                return;
            }
            if !free.contains(&event.as_str()) {
                event = free[0].into();
            }
            egui::ComboBox::from_id_salt("event-type")
                .selected_text(label(&event))
                .show_ui(ui, |ui| {
                    for e in &free {
                        ui.selectable_value(&mut event, (*e).into(), label(e)).on_hover_text(explanation(e));
                    }
                });
            if ui.button("+ Add handler").on_hover_text(explanation(&event)).clicked() {
                let element = element(t, &event, doc, s, state);
                s.actions.push(mg_nui::Route { event: event.clone(), element, action: mg_nui::Action::Code });
            }
        });
        if scope.as_ref().is_some_and(|t| matches!(t, Target::Control(_)))
            && node(scope.as_ref().unwrap(), doc, s).is_some_and(|n| !n["value"]["bind"].is_string() && n.get("value").is_some_and(|v| !v.is_null()))
        {
            ui.weak("Bind its value to react when it changes.");
        }
        ui.ctx().data_mut(|d| d.insert_temp(add_id, (target, event)));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_offer_the_events_the_client_sends() {
        let doc = json!({"root":{"type":"col","children":[
            {"type":"button","id":"b"},
            {"type":"check","id":"c","value":{"bind":"on"}},
            {"type":"check","id":"fixed","value":false},
            {"type":"textedit","id":"t"},
            {"type":"list","id":"l"}]}});
        let s = Settings::default();
        let of = |id: &str| choices(&Target::Id(id.into()), &doc, &s);
        assert_eq!(of("b")[0], "click");
        assert_eq!(of("c")[0], "watch");
        assert!(!of("c").contains(&"click"), "a check box sends no click");
        assert!(!of("fixed").contains(&"watch"), "nothing to watch without a bind");
        assert!(of("t").contains(&"focus") && of("t").contains(&"blur"));
        assert!(of("l").contains(&"range"));
        assert!(!of("b").contains(&"range"));
        assert_eq!(choices(&Target::Window, &doc, &s), ["open", "close"]);
        for e in mg_nui::EVENT_TYPES {
            assert_ne!(label(e), *e);
        }
    }

    #[test]
    fn a_value_change_watches_the_controls_bind_and_ids_are_made_when_needed() {
        let mut doc = json!({"root":{"type":"col","children":[
            {"type":"slider","value":{"bind":"volume"}},
            {"type":"button","label":"Run"},
            {"type":"button","id":"button_event"}]}});
        let s = Settings::default();
        let state = State::default();
        let slider = Target::Control("/root/children/0".into());
        assert_eq!(element(&slider, "watch", &mut doc, &s, &state), "volume");
        let button = Target::Control("/root/children/1".into());
        assert_eq!(element(&button, "click", &mut doc, &s, &state), "button_event_2");
        assert_eq!(doc["root"]["children"][1]["id"], "button_event_2");
    }
}
