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

/// Stands for the window's name in inserted code until it is written.
const WINDOW: &str = "@window@";

/// Opens the event script, at a handler, after adding code to it.
#[derive(Clone)]
pub(super) struct EventScript {
    /// The handlers, as they are this frame.
    pub(super) settings: Settings,
    pub(super) handler: Option<String>,
    pub(super) code: Option<String>,
}

/// The event script as it is now: an editor's unsaved text, else the
/// module's. (An editor's text without changes may be older than the module:
/// it is read again only while its tab shows.)
pub(super) fn current_script(app: &Moonglow, key: &ResKey) -> Option<String> {
    app.scripts
        .get(key)
        .filter(|b| b.is_dirty())
        .map(|b| b.text.clone())
        .or_else(|| app.ws.as_ref()?.module.get(key).map(crate::text::decode))
}

/// The script's new text as an edit, or into an editor holding unsaved work.
fn write_script(app: &mut Moonglow, key: ResKey, text: String) -> Option<Edit> {
    if let Some(buf) = app.scripts.get_mut(&key).filter(|b| b.is_dirty()) {
        buf.text = text;
        return None;
    }
    Some(Edit::SetResource { key, data: Some(crate::text::encode(&text)) })
}

/// The script rebuilt around `settings`' handlers as Build would (one
/// Moonglow wrote, an older one unchanged since included); else why not
/// (logged by the caller).
fn rebuilt(name: &str, settings: &Settings, current: Option<&str>) -> Result<String, String> {
    let bytes = current.map(crate::text::encode);
    mg_nui::rebuild_events(name, settings, bytes.as_deref()).map(|b| crate::text::decode(&b))
}

/// The event script follows the handlers as they are added and removed, so
/// a new one's section is there to write in. One that isn't Moonglow's is
/// left as it is: Build says why.
pub(super) fn sync_script(
    app: &mut Moonglow,
    name: &str,
    old_config: &str,
    config: &str,
) -> Option<Edit> {
    let (old, new) =
        (Settings::parse(old_config.as_bytes()).ok()?, Settings::parse(config.as_bytes()).ok()?);
    let keys = |s: &Settings| s.actions.iter().map(mg_nui::handler_key).collect::<Vec<_>>();
    if keys(&old) == keys(&new) {
        return None;
    }
    let key = ResKey::parse(&format!("{name}_e"), ResType::NSS)?;
    let current = current_script(app, &key);
    let text = rebuilt(name, &new, current.as_deref()).ok()?;
    if current.as_deref() == Some(text.as_str()) {
        return None;
    }
    write_script(app, key, text)
}

/// Brings the event script up to date with the handlers (keeping what is
/// written in them) and adds any code asked for, staying in the Creator;
/// without code, opens it in the script editor at the handler. A script
/// written by hand, or changed outside its sections, is left as it is: a
/// rebuild would lose what is outside them.
pub(super) fn open_script(app: &mut Moonglow, name: &str, ask: EventScript) -> Option<Edit> {
    let key = ResKey::parse(&format!("{name}_e"), ResType::NSS)?;
    let current = current_script(app, &key);
    let mut text = match rebuilt(name, &ask.settings, current.as_deref()) {
        Ok(text) => text,
        Err(why) => {
            app.log.error(why);
            current.clone()?
        }
    };
    if let (Some(handler), Some(code)) = (&ask.handler, &ask.code) {
        match mg_nui::insert_code(&text, handler, &code.replace(WINDOW, name)) {
            Some(with) => {
                text = with;
                app.log.info(format!("Added to the {handler} handler of {name}_e.nss"));
            }
            None => app.log.error(format!("{name}_e.nss has no section for {handler}")),
        }
    } else {
        let jump = ask.handler.as_deref().and_then(|h| mg_nui::handler_offset(&text, h));
        app.script_tools.jump = Some((key, jump.unwrap_or(0)));
        app.actions.push(Action::OpenTab(Tab::Script(key)));
    }
    if current.as_deref() == Some(text.as_str()) {
        return None;
    }
    write_script(app, key, text)
}

/// Handlers follow their controls: when an ID is renamed (one goes, one
/// comes) its handlers take the new one; when a control is deleted (fewer
/// controls have IDs) its handlers go, their code staying in the script,
/// commented out. A renamed handler's section, a renamed ID's or bind's
/// name quoted in the code written in sections, and a renamed variant's
/// function, follow too. Returns the script's edit.
pub(super) fn follow_controls(
    app: &mut Moonglow,
    name: &str,
    old_doc: &str,
    new_doc: &str,
    old_config: &str,
    config: &mut String,
) -> Option<Edit> {
    // Nothing follows a document that doesn't read (a JSON edit half done).
    let (old_window, new_window) =
        (mg_nui::parse(old_doc.as_bytes()).ok()?, mg_nui::parse(new_doc.as_bytes()).ok()?);
    let (old, mut s) =
        (Settings::parse(old_config.as_bytes()).ok()?, Settings::parse(config.as_bytes()).ok()?);
    let ids = |doc: &Value, s: &Settings| {
        let mut ids = Vec::new();
        id_list(doc, &mut ids);
        for v in s.views.values() {
            id_list(v, &mut ids);
        }
        ids
    };
    let (before, after) = (ids(&old_window, &old), ids(&new_window, &s));
    let gone: Vec<_> = before.iter().filter(|i| !after.contains(i)).cloned().collect();
    let came: Vec<_> = after.iter().filter(|i| !before.contains(i)).cloned().collect();
    let controls = |r: &mg_nui::Route| !matches!(r.event.as_str(), "open" | "close" | "watch");
    // Names code spells, renamed: an ID, a bind.
    let mut names = Vec::new();
    if let ([from], [to]) = (gone.as_slice(), came.as_slice()) {
        names.push((from.clone(), to.clone()));
        for route in s.actions.iter_mut().filter(|r| controls(r) && r.element == *from) {
            route.element = to.clone();
        }
    } else if after.len() < before.len() {
        s.actions.retain(|r| !(controls(r) && gone.contains(&r.element)));
    }
    if s != Settings::parse(config.as_bytes()).ok()? {
        *config = String::from_utf8(s.bytes()).ok()?;
    }
    let one_renamed = |a: Vec<&String>, b: Vec<&String>| {
        let from: Vec<_> = a.iter().filter(|k| !b.contains(k)).collect();
        let to: Vec<_> = b.iter().filter(|k| !a.contains(k)).collect();
        match (from.as_slice(), to.as_slice()) {
            ([f], [t]) => Some(((**f).clone(), (**t).clone())),
            _ => None,
        }
    };
    names.extend(one_renamed(old.bindings.keys().collect(), s.bindings.keys().collect()));
    let calls: Vec<_> = one_renamed(old.views.keys().collect(), s.views.keys().collect())
        .map(|(f, t)| (mg_nui::variant_function(name, &f), mg_nui::variant_function(name, &t)))
        .into_iter()
        .collect();
    // Handlers renamed in place: by this frame's edit (a bind renamed) or
    // just above (a control's ID).
    let mut renamed = Vec::new();
    if old.actions.len() == s.actions.len() {
        for (a, b) in old.actions.iter().zip(&s.actions) {
            if a.event == b.event && a.element != b.element {
                renamed.push((a, b));
            }
        }
    }
    if renamed.is_empty() && names.is_empty() && calls.is_empty() {
        return None;
    }
    let key = ResKey::parse(&format!("{name}_e"), ResType::NSS)?;
    let current = current_script(app, &key)?;
    let mut text = current.replace("\r\n", "\n");
    if !text.contains(mg_nui::BEGIN.trim_end()) || mg_nui::edited_outside(name, &text) {
        return None;
    }
    for (a, b) in &renamed {
        let (from, to) = (mg_nui::handler_key(a), mg_nui::handler_key(b));
        text = text.replacen(
            &format!("{}{from}\n", mg_nui::BEGIN),
            &format!("{}{to}\n", mg_nui::BEGIN),
            1,
        );
    }
    for (old_name, new_name) in &names {
        // The name as code spells it: plain, or inside a JSON string.
        text = text
            .replace(&format!("\"{old_name}\""), &format!("\"{new_name}\""))
            .replace(&format!("\\\"{old_name}\\\""), &format!("\\\"{new_name}\\\""));
    }
    for (from, to) in &calls {
        text = text.replace(&format!("{from}("), &format!("{to}("));
    }
    // The guards are rebuilt for the new names.
    let text = mg_nui::merge_events(name, &s, Some(&text)).ok()?;
    if text == current {
        return None;
    }
    write_script(app, key, text)
}

/// Every element ID, as often as it occurs.
fn id_list(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(o) => {
            if let Some(id) = o.get("id").and_then(Value::as_str) {
                out.push(id.to_owned());
            }
            for v in o.values() {
                id_list(v, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|v| id_list(v, out)),
        _ => {}
    }
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

/// Ready-made code to add to a handler: what the old actions did. A menu
/// with nothing to offer is off, saying why.
fn insert_menu(ui: &mut Ui, route: &mg_nui::Route, s: &Settings) -> Option<String> {
    let with =
        |action| mg_nui::handler_code(WINDOW, &mg_nui::Route { action, ..route.clone() }, s).ok();
    let flags: Vec<_> = s
        .bindings
        .iter()
        .filter(|(_, b)| {
            b.value.is_boolean()
                || b.value.as_array().is_some_and(|a| a.iter().all(Value::is_boolean))
        })
        .map(|(n, _)| n.clone())
        .collect();
    let groups: std::collections::BTreeSet<_> =
        s.views.keys().filter_map(|v| layouts::target_for(s, v)).collect();
    let mut code = None;
    ui.menu_button("Insert…", |ui| {
        if ui.button("Close window").clicked() {
            code = with(mg_nui::Action::Close);
        }
        let submenu =
            |ui: &mut Ui, label: &str, empty: bool, why: &str, add: &mut dyn FnMut(&mut Ui)| {
                ui.add_enabled_ui(!empty, |ui| {
                    ui.menu_button(label, |ui| add(ui));
                })
                .response
                .on_disabled_hover_text(why);
            };
        submenu(
            ui,
            "Set bind",
            s.bindings.is_empty(),
            "No binds yet: create one in Bindings",
            &mut |ui| {
                for (bind, b) in &s.bindings {
                    if ui.button(bind).clicked() {
                        code = with(mg_nui::Action::Set {
                            bind: bind.clone(),
                            value: b.value.clone(),
                        });
                    }
                }
            },
        );
        submenu(ui, "Toggle bind", flags.is_empty(), "No true/false binds yet", &mut |ui| {
            for bind in &flags {
                if ui.button(bind).clicked() {
                    code = with(mg_nui::Action::Toggle { bind: bind.clone() });
                }
            }
        });
        submenu(
            ui,
            "Show variant",
            s.views.is_empty(),
            "No swap layout variants yet: add one in Design",
            &mut |ui| {
                for view in s.views.keys() {
                    if ui.button(view).clicked() {
                        let group =
                            layouts::target_for(s, view).unwrap_or_else(|| "_window_".into());
                        code = with(mg_nui::Action::View { group, view: view.clone() });
                    }
                }
                for group in &groups {
                    if ui.button(format!("Initial contents · {group}")).clicked() {
                        let function = mg_nui::initial_function(WINDOW, group);
                        code = Some(format!(
                            "        NuiSetGroupLayout(oPlayer, nToken, {}, {function}());\n",
                            mg_nui::string_expr(group)
                        ));
                    }
                }
            },
        );
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
                    if ui.small_button("Remove").on_hover_text("What you wrote for it stays in the script, commented out").clicked() {
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
        // The events offered last frame: when they change (a value just
        // bound offers its change first), the first is chosen again.
        let (mut target, mut event, mut offered) = ui
            .ctx()
            .data_mut(|d| d.get_temp::<(Option<Target>, String, Vec<String>)>(add_id))
            .unwrap_or_default();
        if scope.is_some() {
            target = scope.clone();
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Add:");
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
            let now: Vec<String> = free.iter().map(|e| (*e).to_owned()).collect();
            if !free.contains(&event.as_str()) || now != offered {
                event = free[0].into();
            }
            offered = now;
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
        ui.ctx().data_mut(|d| d.insert_temp(add_id, (target, event, offered)));
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
