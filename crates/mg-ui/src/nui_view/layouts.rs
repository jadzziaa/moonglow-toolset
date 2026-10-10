//! Layout variants and swap actions share one contextual authoring workflow.
use super::*;

#[derive(Clone)]
struct Draft {
    name: String,
    root: Value,
}

fn unique(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut name = base.to_owned();
    let mut n = 2;
    while taken(&name) {
        name = format!("{base}_{n}");
        n += 1;
    }
    name
}

pub(super) fn edit(state: &mut State, view: Option<String>) {
    state.edit_view = view;
    state.page = 0;
    state.selected = "/root".into();
    state.selected_many.clear();
    state.runtime = None;
    state.preview_interactive = false;
}

// Editor metadata associates a newly authored variant with its intended host.
// It does not create a runtime action; the user must save an explicit event.
fn remember_target(s: &mut Settings, view: &str, group: &str) {
    let targets = s.extra.entry("view_targets".into()).or_insert_with(|| json!({}));
    if let Some(targets) = targets.as_object_mut() {
        targets.insert(view.into(), json!(group));
    }
}

pub(super) fn target_for(s: &Settings, view: &str) -> Option<String> {
    let targets: std::collections::BTreeSet<_> = s
        .actions
        .iter()
        .filter_map(|r| {
            if let mg_nui::Action::View { group, view: name } = &r.action
                && name == view
            {
                Some(group.clone())
            } else {
                None
            }
        })
        .collect();
    if targets.len() == 1 {
        return targets.into_iter().next();
    }
    s.extra.get("view_targets")?.get(view)?.as_str().map(str::to_owned)
}

fn group_mut<'a>(v: &'a mut Value, id: &str) -> Option<&'a mut Value> {
    if v["type"] == "group" && v["id"] == id {
        return Some(v);
    }
    match v {
        Value::Object(o) => o.values_mut().find_map(|v| group_mut(v, id)),
        Value::Array(a) => a.iter_mut().find_map(|v| group_mut(v, id)),
        _ => None,
    }
}

pub(super) fn context_document(
    doc: &Value,
    s: &Settings,
    view: &str,
    target: &str,
) -> Option<Value> {
    context_document_inner(doc, s, view, target, &mut std::collections::BTreeSet::new())
}

pub(super) fn variants(s: &Settings, group: &str) -> Vec<String> {
    s.views.keys().filter(|name| target_for(s, name).as_deref() == Some(group)).cloned().collect()
}

pub(super) fn is_swap(s: &Settings, node: &Value) -> bool {
    node["type"] == "group"
        && node["id"].as_str().is_some_and(|id| {
            !variants(s, id).is_empty()
                || s.extra
                    .get("swap_hosts")
                    .and_then(Value::as_array)
                    .is_some_and(|hosts| hosts.iter().any(|host| host == id))
        })
}

pub(super) fn insert_slot(doc: &mut Value, s: &mut Settings, state: &mut State) {
    design::insert(doc, &mut state.selected, "group");
    state.new_swap_slot = Some(state.selected.clone());
    finish_insert(doc, s, state);
}

pub(super) fn finish_insert(doc: &Value, s: &mut Settings, state: &mut State) {
    if let Some(path) = state.new_swap_slot.take()
        && let Some(node) = doc.pointer(&path)
        && node["type"] == "group"
        && let Some(id) = node["id"].as_str()
    {
        let hosts = s.extra.entry("swap_hosts".into()).or_insert_with(|| json!([]));
        if let Some(hosts) = hosts.as_array_mut() {
            hosts.push(json!(id));
        }
    }
}

/// The canvas is a projection only: variant edits still address /root in the
/// separate authoring document. Surrounding controls are never written back.
pub(super) fn canvas_context(doc: &Value, s: &Settings, state: &State) -> Option<(Value, String)> {
    let view = state.edit_view.as_deref()?;
    let main = state.main_doc.as_ref()?;
    let target = target_for(s, view)?;
    let mut current = s.clone();
    current.views.insert(view.into(), doc["root"].clone());
    let shown = context_document(main, &current, view, &target)?;
    let prefix = if target == "_window_" {
        "/root".into()
    } else {
        format!("{}/children/0", group_path(&shown, &target, "")?)
    };
    Some((shown, prefix))
}

fn group_path(node: &Value, id: &str, path: &str) -> Option<String> {
    if node["type"] == "group" && node["id"] == id {
        return Some(path.into());
    }
    match node {
        Value::Object(o) => o.iter().find_map(|(k, v)| {
            group_path(v, id, &format!("{path}/{}", k.replace('~', "~0").replace('/', "~1")))
        }),
        Value::Array(a) => {
            a.iter().enumerate().find_map(|(i, v)| group_path(v, id, &format!("{path}/{i}")))
        }
        _ => None,
    }
}

pub(super) fn canvas_path<'a>(
    path: &'a str,
    prefix: Option<&str>,
) -> Option<std::borrow::Cow<'a, str>> {
    match prefix {
        None => Some(path.into()),
        Some(prefix) if path == prefix => Some("/root".into()),
        Some(prefix) => {
            path.strip_prefix(&format!("{prefix}/")).map(|tail| format!("/root/{tail}").into())
        }
    }
}

fn context_document_inner(
    doc: &Value,
    s: &Settings,
    view: &str,
    target: &str,
    visited: &mut std::collections::BTreeSet<String>,
) -> Option<Value> {
    if !visited.insert(view.into()) {
        return None;
    }
    let layout = s.views.get(view)?;
    let mut result = doc.clone();
    if target == "_window_" {
        result["root"] = layout.clone();
    } else {
        // A target may itself exist only in another alternate layout.
        if group_mut(&mut result["root"], target).is_none() {
            result = s
                .views
                .iter()
                .filter(|(n, v)| n.as_str() != view && mg_nui::group_ids(v).contains(target))
                .find_map(|(parent, _)| {
                    let parent_target = target_for(s, parent)?;
                    context_document_inner(doc, s, parent, &parent_target, &mut visited.clone())
                })?;
        }
        group_mut(&mut result["root"], target)?["children"] = json!([layout]);
    }
    Some(result)
}

fn make_group(doc: &mut Value, s: &mut Settings, path: &str) -> Option<String> {
    let node = doc.pointer(path)?;
    if !node["type"].is_string() {
        return None;
    }
    let mut ids = mg_nui::element_ids(doc);
    for layout in s.views.values() {
        ids.extend(mg_nui::element_ids(layout));
    }
    let existing_group = node["type"] == "group";
    let id = if existing_group {
        node["id"].as_str().filter(|id| !id.is_empty()).map(str::to_owned)
    } else {
        None
    }
    .unwrap_or_else(|| unique("swap_group", |name| ids.contains(name)));
    let old = doc.pointer_mut(path)?;
    if existing_group {
        old["id"] = json!(id);
    } else {
        let contents = if matches!(old["type"].as_str(), Some("row" | "col")) {
            old.clone()
        } else {
            // A layout root keeps palette insertion available when wrapping a leaf.
            json!({"type":"col", "children":[old.clone()]})
        };
        let mut group =
            json!({"type":"group", "id":id,"border":false,"scrollbars":0,"children":[contents]});
        // Keep the slot's explicit dimensions when adding the new container.
        for property in ["width", "height"] {
            if let Some(size) = old.get(property) {
                group[property] = size.clone();
            }
        }
        *old = group;
    }
    let hosts = s.extra.entry("swap_hosts".into()).or_insert_with(|| json!([]));
    if let Some(hosts) = hosts.as_array_mut() {
        hosts.push(json!(id));
    }
    Some(id)
}

/// Returns the name only on commit. Draft typing and cancelling do not alter Undo.
fn create(ui: &mut Ui, s: &mut Settings, copy: Option<&Value>) -> Option<String> {
    let id = ui.make_persistent_id("new-layout-draft");
    let mut draft = ui.ctx().data_mut(|d| d.get_temp::<Draft>(id));
    if draft.is_none() {
        ui.horizontal_wrapped(|ui| {
            let empty = ui.button("+ Add variant").clicked();
            let duplicate = copy.is_some() && ui.button("Copy initial contents").clicked();
            if empty || duplicate {
                draft = Some(Draft {
                    name: unique("variant", |n| s.views.contains_key(n)),
                    root: if duplicate { copy.unwrap().clone() } else { mg_nui::template("col") },
                });
            }
        });
    }
    let mut result = None;
    let mut finish = false;
    if let Some(draft) = draft.as_mut() {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong("New variant");
            let name = ui.label("Variant name");
            ui.add(egui::TextEdit::singleline(&mut draft.name).desired_width(ui.available_width()))
                .labelled_by(name.id);
            if matches!(draft.root["type"].as_str(), Some("col" | "row")) {
                let mut horizontal = draft.root["type"] == "row";
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut horizontal, false, "Vertical");
                    ui.selectable_value(&mut horizontal, true, "Horizontal");
                });
                draft.root["type"] = json!(if horizontal { "row" } else { "col" });
            }
            let name = draft.name.trim();
            let valid = !name.is_empty() && !name.contains('\0') && !s.views.contains_key(name);
            if !valid {
                ui.weak("Use a nonempty, unique variant name.");
            }
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(valid, egui::Button::new("Create variant")).clicked() {
                    s.views.insert(name.into(), draft.root.clone());
                    result = Some(name.to_owned());
                    finish = true;
                }
                if ui.button("Cancel variant").clicked() {
                    finish = true;
                }
            });
        });
    }
    ui.ctx().data_mut(|d| {
        if finish {
            d.remove::<Draft>(id);
        } else if let Some(draft) = draft {
            d.insert_temp(id, draft);
        }
    });
    if finish {
        ui.ctx().request_repaint();
    }
    result
}

fn select_host(state: &mut State, s: &Settings, target: &str) -> bool {
    if let Some(path) = state.main_doc.as_ref().and_then(|main| group_path(main, target, "")) {
        edit(state, None);
        state.selected = path;
        return true;
    }
    if let Some((name, path)) = s
        .views
        .iter()
        .find_map(|(name, root)| group_path(root, target, "/root").map(|path| (name.clone(), path)))
    {
        state.layout_target = target_for(s, &name).unwrap_or_default();
        edit(state, Some(name));
        state.selected = path;
        return true;
    }
    false
}

pub(super) fn bar(ui: &mut Ui, s: &Settings, state: &mut State) {
    if state.edit_view.as_ref().is_some_and(|name| !s.views.contains_key(name)) {
        edit(state, None);
    }
    if let Some(name) = state.edit_view.clone() {
        let target = target_for(s, &name).unwrap_or_default();
        state.layout_target.clone_from(&target);
        ui.horizontal_wrapped(|ui| {
            if !target.is_empty() && target != "_window_" {
                if ui.button(format!("Swap layout · {target}")).clicked() {
                    select_host(state, s, &target);
                }
                ui.label("/");
                ui.strong(format!("Editing variant: {name}"));
                ui.weak("Editing this area only");
            } else {
                ui.strong(format!("Editing variant: {name}"));
                ui.weak(if target.is_empty() { "No target assigned" } else { "Whole window" });
            }
            if ui.button("Back to main window").clicked() {
                edit(state, None);
            }
        });
    }
}

pub(super) fn inspector(ui: &mut Ui, doc: &mut Value, s: &mut Settings, state: &mut State) {
    let path = state.selected.clone();
    if path == "/root"
        && let Some(name) = state.edit_view.clone()
    {
        ui.strong(format!("Editing variant: {name}"));
        if let Some(target) = target_for(s, &name) {
            ui.label(format!("Belongs to Swap layout · {target}"));
            if ui.button("Swap layout properties").clicked() {
                select_host(state, s, &target);
            }
        }
        ui.separator();
        return;
    }
    let Some(node) = doc.pointer(&path) else { return };
    let ty = node["type"].as_str().unwrap_or_default();
    if !ty.is_empty() && (ty != "group" || !is_swap(s, node)) {
        if ui.button("Convert to Swap layout").on_hover_text(
            "Wrap this fragment in an unbordered Group so an event can replace only its contents.").clicked()
            && let Some(id) = make_group(doc, s, &path) {
            state.layout_target = id;
        }
        return;
    }
    if ty == "group" {
        let contents = node["children"].get(0).cloned();
        let mut group = node["id"].as_str().unwrap_or_default().to_owned();
        egui::CollapsingHeader::new("Variants").default_open(true).show(ui, |ui| {
            ui.weak("One area, multiple contents. Its position and size stay the same.");
            if group.is_empty() {
                ui.weak("A target ID will be assigned when you create a layout.");
            } else {
                ui.label(format!("Target: {group}"));
            }
            if ui.button("Edit initial contents").clicked() {
                state.selected = format!("{path}/children/0");
            }
            for name in variants(s, &group) {
                if ui.button(format!("Edit variant · {name}")).clicked() {
                    state.layout_target = group.clone();
                    edit(state, Some(name));
                }
            }
            if let Some(name) = create(ui, s, contents.as_ref()) {
                if group.is_empty() {
                    let mut ids = mg_nui::element_ids(doc);
                    for v in s.views.values() {
                        ids.extend(mg_nui::element_ids(v));
                    }
                    group = unique("swap_group", |n| ids.contains(n));
                    doc.pointer_mut(&path).unwrap()["id"] = json!(group);
                }
                // Registration is editor metadata only; generation still uses Action::View.
                remember_target(s, &name, &group);
                state.layout_target = group.clone();
                edit(state, Some(name));
            }
            ui.weak("To switch: add a Clicked event on a Button with Replace group layout. Choose this area and a variant.");
        });
        ui.separator();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    fn harness(settings: Settings) -> Harness<'static, Moonglow> {
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
        super::super::create(&mut app, "swap_test");
        app.actions.clear();
        let ws = app.ws.as_mut().unwrap();
        let mut doc = mg_nui::window();
        doc["root"]["children"][0] = json!({"type":"group","height":100.0,"children":[{"type":"col","children":[{"type":"label","value":"Original content"}]}]});
        ws.module.set(mg_nui::key("swap_test", ResType::JUI), serde_json::to_vec(&doc).unwrap());
        ws.module.set(mg_nui::key("swap_test", ResType::TXT), settings.bytes());
        Harness::builder().with_size(egui::vec2(1300.0, 1200.0)).build_ui_state(
            |ui, app: &mut Moonglow| {
                super::super::ui(app, ui, Some(mg_nui::key("swap_test", ResType::JUI)));
                app.run_actions();
            },
            app,
        )
    }

    fn read(h: &Harness<'_, Moonglow>) -> (Value, Settings) {
        let module = &h.state().ws.as_ref().unwrap().module;
        (
            mg_nui::parse(module.get(&mg_nui::key("swap_test", ResType::JUI)).unwrap()).unwrap(),
            Settings::parse(module.get(&mg_nui::key("swap_test", ResType::TXT)).unwrap()).unwrap(),
        )
    }

    #[test]
    fn swap_palette_adds_native_group_without_implicit_event() {
        let mut h = harness(Settings::default());
        h.run();
        h.get_by_label("Swap layout").click();
        h.run();
        let (doc, s) = read(&h);
        let slot = doc["root"]["children"].as_array().unwrap().last().unwrap();
        assert_eq!(slot["type"], "group");
        assert!(is_swap(&s, slot));
        assert!(s.views.is_empty());
        assert!(s.actions.is_empty());
        assert!(mg_nui::validate(&doc, &s).iter().all(|d| d.severity != Severity::Error));
    }

    #[test]
    fn swap_has_one_contextual_creation_flow_and_returns_to_its_host() {
        let mut h = harness(Settings::default());
        h.run();
        assert!(h.query_by_label("Layouts…").is_none());
        assert!(h.query_by_value("Editing layout:").is_none());
        h.get_all_by_label("Group").last().unwrap().click();
        h.run();
        assert!(
            h.query_by_label("+ Add variant").is_none(),
            "Plain Group must not masquerade as a Swap layout"
        );
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        h.get_by_label("Swap layout").click();
        h.run();
        let initial = read(&h);
        let slot = initial.0["root"]["children"].as_array().unwrap().last().unwrap();
        let id = slot["id"].as_str().unwrap();
        assert!(is_swap(&initial.1, slot));
        h.get_by_label("+ Add variant").click();
        h.run();
        h.get_by_label("Cancel variant").click();
        h.run();
        assert_eq!(read(&h), initial);
        h.get_by_label("+ Add variant").click();
        h.run();
        h.get_by_label("Horizontal").click();
        h.run();
        h.get_by_label("Create variant").click();
        h.run();
        let authored = read(&h);
        assert_eq!(authored.0, initial.0);
        assert_eq!(target_for(&authored.1, "variant").as_deref(), Some(id));
        assert_eq!(authored.1.views["variant"]["type"], "row");
        assert!(authored.1.actions.is_empty(), "Creating variants must not install a switch event");
        h.get_by_label("Swap layout properties").click();
        h.run();
        h.get_by_label("Edit variant · variant");
        assert_eq!(read(&h), authored, "Returning to the slot must not edit its contents");
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert_eq!(read(&h), initial);
    }

    #[test]
    fn returning_to_a_nested_swap_host_selects_the_parent_variant() {
        let mut s = Settings::default();
        s.views.insert(
            "outer".into(),
            json!({"type":"col","children":[
            {"type":"group","id":"inner","children":[{"type":"col","children":[]}]}]}),
        );
        remember_target(&mut s, "outer", "content");
        let mut state = State {
            main_doc: Some(json!({"root":{"type":"group","id":"content"}})),
            ..Default::default()
        };
        let original = s.clone();
        assert!(select_host(&mut state, &s, "inner"));
        assert_eq!(state.edit_view.as_deref(), Some("outer"));
        assert_eq!(state.selected, "/root/children/0");
        assert_eq!(s, original);
        assert!(select_host(&mut state, &s, "content"));
        assert!(state.edit_view.is_none());
        assert_eq!(state.selected, "/root");
        assert!(!select_host(&mut state, &s, "missing"));
    }

    #[test]
    fn palette_drop_into_variant_tree_does_not_modify_main_layout() {
        let mut s = Settings::default();
        s.views.insert(
            "details".into(),
            json!({"type":"col","children":[{"type":"label","value":"Variant only"}]}),
        );
        remember_target(&mut s, "details", "content");
        let mut h = harness(s);
        let mut doc = mg_nui::window();
        doc["root"]["children"] = json!([
            {"type":"group","id":"content","height":100.0,"children":[{"type":"col","children":[]}]},
            {"type":"label","value":"Fixed footer"}
        ]);
        h.state_mut()
            .ws
            .as_mut()
            .unwrap()
            .module
            .set(mg_nui::key("swap_test", ResType::JUI), serde_json::to_vec(&doc).unwrap());
        h.run();
        let original = read(&h);
        h.get_by_label("Variant · details").click();
        h.run();
        let from = h.get_by_label("Row").rect().center();
        let onto = h.get_by_label("Label · Variant only").rect().center();
        h.hover_at(from);
        h.run();
        h.drag_at(from);
        h.run();
        h.hover_at(from + egui::vec2(14.0, 0.0));
        h.run();
        h.hover_at(onto);
        h.run();
        h.drop_at(onto);
        h.run();
        let edited = read(&h);
        assert_eq!(edited.0, original.0);
        assert_eq!(edited.1.views["details"]["children"][1]["type"], "row");
        assert!(edited.1.actions.is_empty());
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert_eq!(read(&h), original);
    }

    #[test]
    fn tree_variant_is_edited_in_its_slot_without_writing_surroundings() {
        let mut s = Settings::default();
        s.views.insert(
            "details".into(),
            json!({"type":"row","children":[
            {"type":"label","value":"Left","width":150.0,"height":30.0},
            {"type":"label","value":"Right","width":150.0,"height":30.0}]}),
        );
        remember_target(&mut s, "details", "content");
        s.views.insert("other".into(), json!({"type":"col","children":[]}));
        remember_target(&mut s, "other", "content");
        let mut h = harness(s);
        let mut doc = mg_nui::window();
        doc["root"]["children"] = json!([
            {"type":"label","value":"Header","height":30.0},
            {"type":"group","id":"content","width":360.0,"height":100.0,
                "children":[{"type":"col","children":[{"type":"label","value":"Initial"}]}]},
            {"type":"label","value":"Footer","height":30.0}]);
        h.state_mut()
            .ws
            .as_mut()
            .unwrap()
            .module
            .set(mg_nui::key("swap_test", ResType::JUI), serde_json::to_vec(&doc).unwrap());
        h.run();
        let original = read(&h);
        h.get_by_label("Swap layout · content");
        // What a swap layout holds sits under the contents it belongs to.
        let rows =
            |h: &Harness<'_, Moonglow>, labels: [&str; 3]| labels.map(|l| h.get_by_label(l).rect());
        let [start, initial, first] =
            rows(&h, ["Initial contents", "Label · Initial", "Variant · details"]);
        assert!(start.top() < initial.top() && initial.top() < first.top());
        assert!(initial.left() > start.left());
        h.get_by_label("Variant · details").click();
        h.run();
        let [details, left, other] =
            rows(&h, ["Variant · details", "Label · Left", "Variant · other"]);
        assert!(details.top() < left.top() && left.top() < other.top());
        assert!(left.left() > details.left());
        h.get_by_label("Context Label · Header");
        h.get_by_label("Context Label · Footer");
        let host = h.get_by_label("Context Group").rect();
        let right = h.get_by_label("Canvas Label · Right").rect();
        assert!(host.contains_rect(right), "Variant rendered outside its slot");
        assert_eq!(read(&h), original, "Variant navigation changed output");
        h.get_by_label("Canvas Label · Right").click();
        h.run();
        h.get_by_label("Button").click();
        h.run();
        let edited = read(&h);
        assert_eq!(edited.0, original.0, "Authoring overwrote the surrounding main document");
        assert_eq!(edited.1.views["details"]["children"].as_array().unwrap().len(), 3);
        assert!(edited.1.actions.is_empty(), "Editing a variant created an event");
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert_eq!(read(&h), original);
        h.get_by_label("Back to main window").click();
        h.run();
        h.get_by_label("Canvas Label · Initial");
        assert_eq!(read(&h), original);
    }

    #[test]
    fn context_path_mapping_does_not_admit_siblings_or_prefix_collisions() {
        let prefix = "/root/children/1/children/0";
        assert_eq!(canvas_path(prefix, Some(prefix)).as_deref(), Some("/root"));
        assert_eq!(
            canvas_path(&format!("{prefix}/children/2"), Some(prefix)).as_deref(),
            Some("/root/children/2")
        );
        for outside in
            ["/root", "/root/children/1", "/root/children/1/children/01", "/root/children/2"]
        {
            assert!(canvas_path(outside, Some(prefix)).is_none());
        }
    }

    #[test]
    fn wrapping_nested_fragment_preserves_ids_dimensions_and_siblings() {
        let mut doc = json!({"version":1,"root":{"type":"col","children":[
            {"type":"label","value":"Header"},
            {"type":"row","children":[{"type":"button","id":"keep","label":"Keep"},
                {"type":"col","width":240.0,"height":160.0,"children":[
                    {"type":"button","id":"swap_group","label":"Inner"}]}]},
            {"type":"label","value":"Footer"}]}});
        let original = doc.clone();
        let mut settings = Settings::default();
        let path = "/root/children/1/children/1";
        let id = make_group(&mut doc, &mut settings, path).unwrap();
        assert_eq!(id, "swap_group_2");
        let group = doc.pointer(path).unwrap();
        assert_eq!(group["children"][0], original.pointer(path).unwrap().clone());
        assert_eq!(group["width"], 240.0);
        assert_eq!(group["height"], 160.0);
        assert_eq!(group["border"], false);
        assert_eq!(doc["root"]["children"][0], original["root"]["children"][0]);
        assert_eq!(
            doc["root"]["children"][1]["children"][0],
            original["root"]["children"][1]["children"][0]
        );
        assert_eq!(doc["root"]["children"][2], original["root"]["children"][2]);
        assert!(settings.actions.is_empty());
        assert!(mg_nui::validate(&doc, &settings).iter().all(|d| d.severity != Severity::Error));
    }

    #[test]
    fn contextual_variant_changes_only_nested_group_contents_and_supports_groups_in_views() {
        let doc = json!({"root":{"type":"col","children":[
            {"type":"label","value":"Header"},
            {"type":"row","children":[{"type":"label","value":"Sidebar"},
                {"type":"group","id":"content","width":240.0,"height":160.0,"border":true,
                    "children":[{"type":"label","value":"Before"}]}]},
            {"type":"button","id":"close","label":"Close"}]}});
        let mut settings = Settings::default();
        settings.views.insert(
            "details".into(),
            json!({"type":"row","children":[
            {"type":"group","id":"nested","children":[{"type":"label","value":"Inner before"}]}]}),
        );
        settings.views.insert("inner".into(), json!({"type":"label","value":"Inner after"}));
        remember_target(&mut settings, "details", "content");
        let original = settings.clone();
        let result = context_document(&doc, &settings, "details", "content").unwrap();
        let mut expected = doc.clone();
        expected["root"]["children"][1]["children"][1]["children"] =
            json!([settings.views["details"]]);
        assert_eq!(result, expected);
        let nested = context_document(&doc, &settings, "inner", "nested").unwrap();
        assert_eq!(
            nested["root"]["children"][1]["children"][1]["children"][0]["children"][0]["children"]
                [0],
            settings.views["inner"]
        );
        assert_eq!(nested["root"]["children"][0], doc["root"]["children"][0]);
        assert!(context_document(&doc, &settings, "details", "missing").is_none());
        assert_eq!(settings, original);
    }

    #[test]
    fn wrap_selected_control_and_author_variant_is_explicit_and_undoable() {
        let mut h = harness(Settings::default());
        h.run();
        let before = read(&h);
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        h.get_by_label("Convert to Swap layout").click();
        h.run();
        let wrapped = read(&h);
        assert_eq!(
            wrapped.0["root"]["children"][1]["children"][0]["children"][0],
            before.0["root"]["children"][1]
        );
        assert!(wrapped.1.actions.is_empty());
        assert!(is_swap(&wrapped.1, &wrapped.0["root"]["children"][1]));
        h.get_by_label("Copy initial contents").click();
        h.run();
        h.get_by_label("Create variant").click();
        h.run();
        let authored = read(&h);
        assert!(authored.1.actions.is_empty());
        assert_eq!(target_for(&authored.1, "variant").as_deref(), Some("swap_group"));
        h.get_by_label("Context Label · Original content");
        assert_eq!(read(&h), authored, "Context projection must never alter the module");
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert_eq!(read(&h), wrapped);
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert_eq!(read(&h), before);
    }

    #[test]
    fn swap_group_copies_contents_assigns_id_and_navigates_without_overwriting_layout() {
        let mut h = harness(Settings::default());
        h.run();
        let before = read(&h);
        // The Group's canvas centre belongs to its child Column; select the
        // explicit Layers row (the earlier Group button is the palette).
        h.get_all_by_label("Group").last().unwrap().click();
        h.run();
        h.get_by_label("Convert to Swap layout").click();
        h.run();
        let converted = read(&h);
        h.get_by_label("Copy initial contents").click();
        h.run();
        h.get_by_label("Cancel variant").click();
        h.run();
        assert_eq!(read(&h), converted);
        h.get_by_label("Copy initial contents").click();
        h.run();
        h.get_by_label("Create variant").click();
        h.run();
        let (doc, settings) = read(&h);
        assert_eq!(settings.views["variant"], before.0["root"]["children"][0]["children"][0]);
        assert_eq!(doc["root"]["children"][0]["id"], "swap_group");
        assert_eq!(doc["root"]["children"][1], before.0["root"]["children"][1]);
        h.get_by_label("Back to main window").click();
        h.run();
        assert_eq!(read(&h), (doc, settings));
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert_eq!(read(&h), converted);
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert_eq!(read(&h), before);
    }
}
