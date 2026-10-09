//! JUI authoring in the same Workspace/Undo/recovery path as other editors.
//! Only transient selection lives in egui memory; even invalid JSON drafts
//! remain in the module and are recoverable. No lossy fallback widgets.
use crate::{Action, Moonglow, Tab};
use egui::Ui;
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_nui::{Binding, Settings, Severity};
use mg_resman::ResKey;
use serde_json::{Value, json};

mod bindings;
mod design;
mod draw;
mod events;
mod fields;
mod interaction;
#[cfg(test)]
mod interaction_tests;
mod layouts;
mod preview;
mod shortcuts;
pub(crate) mod skin;
mod structure;
mod workflow;

#[derive(Clone, Default)]
struct State {
    selected: String,
    search: String,
    zoom: f32,
    preview_scale: f32,
    preview_state: usize,
    preview_info: bool,
    preview_clean: bool,
    preview_interactive: bool,
    preview_rect: Option<egui::Rect>,
    runtime: Option<interaction::Session>,
    resize: Option<interaction::Resize>,
    collapsed: std::collections::BTreeSet<String>,
    tree_selection: String,
    list_scroll: std::collections::BTreeMap<String, f32>,
    group_scroll_x: std::collections::BTreeMap<String, f32>,
    group_scroll_y: std::collections::BTreeMap<String, f32>,
    text_scroll_x: std::collections::BTreeMap<String, f32>,
    text_scroll_y: std::collections::BTreeMap<String, f32>,
    selected_many: std::collections::BTreeSet<String>,
    layer_search: String,
    move_pending: Option<(structure::Drag, String, structure::Position)>,
    insert_pending: Option<(&'static str, String, structure::Position)>,
    edit_view: Option<String>,
    layout_target: String,
    new_swap_slot: Option<String>,
    layout_groups: std::collections::BTreeSet<String>,
    main_doc: Option<Value>,
    preset_name: String,
    binding_selected: String,
    event_code: Option<events::CodePreview>,
    binding_property: String,
    asset_search: String,
    screen_preview: bool,
    screen_size: egui::Vec2,
    window_move: Option<interaction::WindowMove>,
    issues: bool,
    page: usize,
    view_mode: usize,
    name: String,
    load_search: String,
    checks: Option<Checks>,
}

/// The API checks and build status of what the tab last showed.
#[derive(Clone, Default)]
struct Checks {
    key: u64,
    findings: Vec<mg_nui::Diagnostic>,
    needs_build: bool,
}

impl State {
    fn guides(&self) -> bool {
        !self.preview_clean
    }
}

/// Keep the launcher and documents in one workspace, preserving its window geometry.
pub(crate) fn route_tab(dock: &mut egui_dock::DockState<Tab>, tab: &Tab) {
    if !matches!(tab, Tab::Nui(_)) {
        return;
    }
    if matches!(tab, Tab::Nui(Some(_)))
        && let Some(launcher) = dock.find_tab(&Tab::Nui(None))
    {
        if dock.find_tab(tab).is_some() {
            dock.remove_tab(launcher);
        } else if let Ok(leaf) = dock.leaf_mut(launcher.node_path()) {
            leaf.tabs[launcher.tab.0] = tab.clone();
        }
    }
    if dock.find_tab(tab).is_none()
        && let Some(other) = dock.find_tab_from(|t| matches!(t, Tab::Nui(_)))
    {
        dock.set_focused_node_and_surface(other.node_path());
        dock.push_to_focused_leaf(tab.clone());
    }
}

pub(crate) fn create(app: &mut Moonglow, name: &str) {
    let Some(ws) = &app.ws else { return };
    match mg_nui::create(&ws.module, name) {
        Ok(cmd) => match app.apply(cmd) {
            Ok(()) => {
                app.actions.push(Action::OpenTab(Tab::Nui(Some(mg_nui::key(name, ResType::JUI)))))
            }
            Err(e) => app.log.error(e.to_string()),
        },
        Err(e) => app.log.error(e),
    }
}

pub(crate) fn generate(app: &mut Moonglow, name: &str, export: bool) {
    // The compiler must see the same source the user is looking at. Do not
    // silently compile older saved includes or overwrite an unsaved opener.
    if app.scripts.values().any(|b| b.is_dirty()) {
        app.log.error("Save edited scripts before generating NUI (includes may have changed)");
        return;
    }
    let (Some(ws), Some(game)) = (&app.ws, &app.game) else {
        app.log.error("NUI generation needs the installed game's NWScript API; configure the game folder in Options");
        return;
    };
    let result = mg_nui::generate(&ws.module, name, |n, t| {
        game.resman.get_named(n, t).ok().map(|b| b.into_owned())
    });
    match result {
        Ok(cmd) => match if cmd.edits.is_empty() { Ok(()) } else { app.apply(cmd) } {
            Ok(()) => {
                app.log.info(format!("Validated {name}_o.nss and compiled {name}_e.nss. Include {name}_o in your own script and call Open_{name}(oPlayer). Choose the player and triggering event yourself. Client appearance and events still require Test Module."));
                if export {
                    let mut resources = vec![
                        mg_nui::key(name, ResType::JUI),
                        mg_nui::key(name, ResType::TXT),
                        mg_nui::key(&format!("{name}_o"), ResType::NSS),
                        mg_nui::key(&format!("{name}_e"), ResType::NSS),
                        mg_nui::key(&format!("{name}_e"), ResType::NCS),
                    ];
                    if let Some(ws) = &app.ws
                        && let (Some(doc), Some(settings)) = (
                            ws.module
                                .get(&mg_nui::key(name, ResType::JUI))
                                .and_then(|b| mg_nui::parse(b).ok()),
                            ws.module
                                .get(&mg_nui::key(name, ResType::TXT))
                                .and_then(|b| Settings::parse(b).ok()),
                        )
                    {
                        for image in mg_nui::image_names(&doc, &settings) {
                            for ty in [
                                ResType::PNG,
                                ResType::TGA,
                                ResType::DDS,
                                ResType::JPG,
                                ResType::BMP,
                                ResType::TXI,
                            ] {
                                if let Some(k) =
                                    ResKey::parse(&image, ty).filter(|k| ws.module.contains(k))
                                {
                                    resources.push(k);
                                }
                            }
                        }
                    }
                    app.actions.push(Action::ExportDialog(resources));
                }
            }
            Err(e) => app.log.error(e.to_string()),
        },
        Err(e) => app.log.error(e),
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: Option<ResKey>) {
    let id = ui.id().with(("nui", key));
    // Floating dock windows have no native title bar, so egui permits dragging
    // their background. Absorb body drags before registering controls; child
    // text selections, sliders and canvas handles retain their own drag sense.
    ui.interact(ui.max_rect(), id.with("body-drag-guard"), egui::Sense::drag());
    let mut state = ui.ctx().data_mut(|d| d.get_temp::<State>(id).unwrap_or_default());
    let Some(ws) = &app.ws else {
        ui.label("Open a module first.");
        return;
    };
    let Some(key) = key else {
        let existing: Vec<_> =
            ws.module.keys().filter(|k| k.restype == ResType::JUI).copied().collect();
        design::landing(ui, &mut state, &existing, &mut app.actions);
        ui.ctx().data_mut(|d| d.insert_temp(id, state));
        return;
    };
    let Some(bytes) = ws.module.get(&key) else {
        ui.label("This window is no longer in the module.");
        return;
    };
    let Ok(source) = std::str::from_utf8(bytes) else {
        ui.label(
            "JUI is not UTF-8. Its original bytes are retained; structured editing is unavailable.",
        );
        return;
    };
    let mut raw = source.to_owned();
    let original = raw.clone();
    let name = key.resref.to_string();
    let config_key = ResKey::new(key.resref, ResType::TXT);
    let mut config_raw = if let Some(config_bytes) = ws.module.get(&config_key) {
        match std::str::from_utf8(config_bytes) {
            Ok(s) => s.to_owned(),
            Err(_) => {
                ui.label("Companion TXT is not UTF-8; it is retained without changes.");
                return;
            }
        }
    } else {
        String::from_utf8(Settings::default().bytes()).unwrap()
    };
    let config_original = config_raw.clone();
    let parsed = mg_nui::parse(raw.as_bytes());
    let settings = Settings::parse(config_raw.as_bytes());
    if let Ok(doc) = &parsed {
        state.main_doc = Some(doc.clone());
        state.layout_groups = mg_nui::group_ids(doc);
    }
    // Checking the whole document is not free: only redo it when the window,
    // its settings or its generated scripts change.
    let checks_key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&raw, &config_raw).hash(&mut h);
        for (suffix, ty) in [("_o", ResType::NSS), ("_o", ResType::NCS), ("_e", ResType::NSS)] {
            ResKey::parse(&format!("{name}{suffix}"), ty)
                .and_then(|k| ws.module.get(&k))
                .hash(&mut h);
        }
        ResKey::parse(&format!("{name}_e"), ResType::NCS)
            .is_some_and(|k| ws.module.contains(&k))
            .hash(&mut h);
        h.finish()
    };
    if state.checks.as_ref().is_none_or(|c| c.key != checks_key) {
        let (findings, needs_build) = match (&parsed, &settings) {
            (Ok(v), Ok(s)) => {
                (mg_nui::validate(v, s), !mg_nui::is_current(&ws.module, &name, v, s))
            }
            _ => (Vec::new(), false),
        };
        state.checks = Some(Checks { key: checks_key, findings, needs_build });
    }
    let Checks { findings, needs_build, .. } = state.checks.clone().unwrap_or_default();
    let errors = findings.iter().filter(|d| d.severity == Severity::Error).count()
        + usize::from(parsed.is_err())
        + usize::from(settings.is_err());
    let warnings = findings.iter().filter(|d| d.severity == Severity::Warning).count();
    ui.spacing_mut().button_padding = egui::vec2(10.0, 5.0);
    ui.horizontal_wrapped(|ui| {
        ui.heading("NUI Creator");
        ui.weak("/");
        ui.strong(&name);
        design::document_actions(ui, &mut state, &ws.module, &mut app.actions);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("Export…")
                .on_hover_text("Validate and export the window, bind settings, opener include and compiled event script")
                .clicked()
            {
                app.actions.push(Action::GenerateNui(name.clone(), true));
            }
            if ui
                .add_enabled(
                    errors == 0,
                    egui::Button::new("Build & compile").fill(ui.visuals().selection.bg_fill),
                )
                .on_hover_text(
                    "Validate the opener include and compile the event script. Connect the opener manually in your own script.",
                )
                .clicked()
            {
                app.actions.push(Action::GenerateNui(name.clone(), false));
            }
            if needs_build {
                ui.weak("Not built")
                    .on_hover_text("Build before testing this layout or these bind values in NWN");
            }
        });
    });
    ui.add_space(6.0);
    egui::ScrollArea::horizontal().id_salt("nui-navigation").auto_shrink([false, true]).show(
        ui,
        |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                ui.spacing_mut().interact_size.y = 28.0;
                ui.selectable_value(&mut state.page, 0, "Design");
                ui.selectable_value(&mut state.page, 1, "Bindings");
                ui.selectable_value(&mut state.page, 4, "Views & events");
                if ui.add_enabled(settings.is_ok(), egui::Button::new("Event script…")).clicked()
                    && let Ok(s) = &settings
                {
                    state.event_code =
                        Some(events::CodePreview::new(s, None, "All configured events"));
                }
                if state.page == 0 {
                    egui::ComboBox::from_id_salt("nui-layout-mode")
                        .selected_text(["Editor", "Preview only", "Split source"][state.view_mode])
                        .show_ui(ui, |ui| {
                            for (i, label) in
                                ["Editor", "Preview only", "Split source"].iter().enumerate()
                            {
                                ui.selectable_value(&mut state.view_mode, i, *label);
                            }
                        });
                }
                ui.menu_button("Advanced", |ui| {
                    if ui.selectable_label(state.page == 2, "JUI source").clicked() {
                        state.page = 2;
                        ui.close();
                    }
                    if ui.selectable_label(state.page == 3, "Binding JSON").clicked() {
                        state.page = 3;
                        ui.close();
                    }
                    ui.separator();
                    for (suffix, label) in [("_o", "Open script"), ("_e", "Event script")] {
                        if let Some(k) = ResKey::parse(&format!("{name}{suffix}"), ResType::NSS)
                            && ui
                                .add_enabled(ws.module.contains(&k), egui::Button::new(label))
                                .clicked()
                        {
                            app.actions.push(Action::OpenTab(Tab::Script(k)));
                            ui.close();
                        }
                    }
                });
                ui.separator();
                let label = if errors + warnings == 0 {
                    "Checks passed".to_owned()
                } else {
                    format!("{errors} errors · {warnings} warnings")
                };
                ui.toggle_value(&mut state.issues, label)
                    .on_hover_text("API checks; native NWN testing is still required");
            });
        },
    );
    ui.separator();
    if state.issues || errors > 0 {
        egui::ScrollArea::vertical().id_salt("nui-diagnostics").max_height(90.0).show(ui, |ui| {
            if let Err(e) = &parsed {
                ui.colored_label(egui::Color32::LIGHT_RED, format!("JUI: {e}"));
            }
            if let Err(e) = &settings {
                ui.colored_label(egui::Color32::LIGHT_RED, format!("Settings: {e}"));
            }
            for d in &findings {
                ui.label(format!("{:?} {}: {}", d.severity, d.path, d.message));
                if d.message.starts_with("The mg_close button has no Clicked event")
                    && let Ok(s) = &settings
                    && ui.small_button("Add Clicked → Close window").clicked()
                {
                    let mut s = s.clone();
                    s.actions.push(mg_nui::Route {
                        event: "click".into(),
                        element: "mg_close".into(),
                        action: mg_nui::Action::Close,
                    });
                    config_raw = String::from_utf8(s.bytes()).unwrap();
                }
            }
            if errors + warnings == 0 {
                ui.weak("No API issues found. Verify the final appearance and events in NWN.");
            }
        });
        ui.separator();
    }
    match state.page {
        4 => {
            if let (Ok(mut v), Ok(mut s)) = (parsed, settings) {
                let before = v.clone();
                let config_before = s.clone();
                workflow::ui(ui, &mut v, &mut s, &mut state);
                if v != before {
                    raw = serde_json::to_string_pretty(&v).unwrap();
                }
                if s != config_before {
                    config_raw = String::from_utf8(s.bytes()).unwrap();
                }
            }
        }
        2 | 3 => {
            let is_jui = state.page == 2;
            ui.strong(if is_jui { "JUI source" } else { "Binding JSON" });
            ui.weak("Drafts are included in module Save and Undo, even while incomplete.");
            egui::ScrollArea::both().id_salt(("nui-source", state.page)).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(if is_jui { &mut raw } else { &mut config_raw })
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(26),
                );
            });
        }
        1 => {
            if let Ok(mut s) = settings {
                let before = s.clone();
                design::bindings(ui, &mut s, parsed.as_ref().ok(), &mut state);
                if s != before {
                    config_raw = String::from_utf8(s.bytes()).unwrap();
                }
            } else {
                ui.label("Settings need repair before editing bindings.");
                if ui.button("Open binding JSON").clicked() {
                    state.page = 3;
                }
            }
        }
        _ => {
            if let (Ok(mut v), Ok(mut s)) = (parsed, settings) {
                let before = v.clone();
                let before_settings = s.clone();
                layouts::bar(ui, &s, &mut state);
                let edited_view = state.edit_view.clone();
                let base_root = v["root"].clone();
                if let Some(view) = state.edit_view.as_ref().and_then(|n| s.views.get(n)) {
                    v["root"] = view.clone();
                }
                let (asset_doc, asset_settings) = state
                    .runtime
                    .as_ref()
                    .filter(|r| state.preview_interactive && r.matches(&v, &s))
                    .map_or((&v, &s), |r| (&r.doc, &r.settings));
                app.nui_assets.prepare(
                    ui.ctx(),
                    app.game.as_deref().map(|g| &g.resman),
                    &ws.module,
                    ws.revision(),
                    asset_doc,
                    asset_settings,
                );
                app.nui_assets.prepare_strings(app.game.as_deref(), asset_doc, asset_settings);
                let mut split_raw = None;
                match state.view_mode {
                    1 => preview::canvas(ui, &mut v, &s, &mut state, &mut app.nui_assets),
                    2 => ui.columns(2, |columns| {
                        preview::canvas(
                            &mut columns[0],
                            &mut v,
                            &s,
                            &mut state,
                            &mut app.nui_assets,
                        );
                        let mut source = if state.edit_view.is_some() {
                            serde_json::to_string_pretty(&s).unwrap()
                        } else {
                            serde_json::to_string_pretty(&v).unwrap()
                        };
                        columns[1].strong(if state.edit_view.is_some() {
                            "Views & binding JSON"
                        } else {
                            "JUI source"
                        });
                        egui::ScrollArea::both().id_salt("split-jui").show(&mut columns[1], |ui| {
                            if ui
                                .add(
                                    egui::TextEdit::multiline(&mut source)
                                        .code_editor()
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(28),
                                )
                                .changed()
                            {
                                split_raw = Some(source);
                            }
                        });
                    }),
                    _ => design::editor(
                        ui,
                        &mut v,
                        &mut s,
                        &mut state,
                        &app.keymap,
                        &mut app.nui_assets,
                    ),
                }
                layouts::finish_insert(&v, &mut s, &mut state);
                if let Some(name) = &edited_view {
                    s.views.insert(name.clone(), v["root"].clone());
                    v["root"] = base_root;
                }
                if v != before {
                    raw = serde_json::to_string_pretty(&v).unwrap();
                }
                if s != before_settings {
                    config_raw = String::from_utf8(s.bytes()).unwrap();
                }
                if let Some(source) = split_raw {
                    if edited_view.is_some() {
                        config_raw = source;
                    } else {
                        raw = source;
                    }
                }
            } else {
                ui.add_space(24.0);
                ui.heading("Let's repair the source first");
                ui.label(
                    "Use the source/settings tabs to repair JSON. Original contents are retained.",
                );
                ui.horizontal(|ui| {
                    if ui.button("Open JUI source").clicked() {
                        state.page = 2;
                    }
                    if ui.button("Open binding JSON").clicked() {
                        state.page = 3;
                    }
                });
            }
        }
    }
    events::code_preview(ui.ctx(), app, &name, &mut state.event_code);
    let mut edits = Vec::new();
    if raw != original {
        edits.push(Edit::SetResource { key, data: Some(raw.into_bytes()) });
    }
    if config_raw != config_original {
        edits.push(Edit::SetResource { key: config_key, data: Some(config_raw.into_bytes()) });
    }
    if !edits.is_empty() {
        // Apply before a generation requested in this frame, including a text
        // field losing focus when its button was clicked.
        app.actions.insert(0, Action::Apply(Command::new(format!("Edit NUI {name}"), edits)));
    }
    ui.ctx().data_mut(|d| d.insert_temp(id, state));
}

fn array_position(path: &str) -> Option<(String, usize, bool)> {
    let cell = path.ends_with("/0") && path.rsplit('/').nth(2) == Some("row_template");
    let path = if cell { path.strip_suffix("/0")? } else { path };
    let (parent, index) = path.rsplit_once('/')?;
    if !parent.ends_with("/children") && !parent.ends_with("/row_template") {
        return None;
    }
    Some((parent.into(), index.parse().ok()?, cell))
}

fn rearrange(v: &mut Value, selected: &mut String, op: usize) {
    let Some((parent, i, cell)) = array_position(selected) else { return };
    let Some(a) = v.pointer(&parent).and_then(Value::as_array) else { return };
    if i >= a.len() {
        return;
    }
    // Group must retain its single child. Replace its child's contents through
    // the source editor, or edit the row/column nested inside it.
    let group = parent
        .strip_suffix("/children")
        .and_then(|p| v.pointer(p))
        .is_some_and(|n| n["type"] == "group");
    if group {
        return;
    }
    let mut copy = a[i].clone();
    assign_ids(&mut copy, v);
    let a = v.pointer_mut(&parent).unwrap().as_array_mut().unwrap();
    let mut at = i;
    match op {
        0 if i > 0 => {
            a.swap(i, i - 1);
            at -= 1;
        }
        1 if i + 1 < a.len() => {
            a.swap(i, i + 1);
            at += 1;
        }
        2 => {
            a.insert(i + 1, copy);
            at += 1;
        }
        3 => {
            a.remove(i);
            *selected = if a.is_empty() {
                parent.rsplit_once('/').map_or("/root", |(p, _)| p).into()
            } else {
                format!("{parent}/{}{}", i.min(a.len() - 1), if cell { "/0" } else { "" })
            };
            return;
        }
        _ => {}
    }
    *selected = format!("{parent}/{at}{}", if cell { "/0" } else { "" });
}

fn assign_ids(node: &mut Value, document: &Value) {
    fn ids(v: &Value, set: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(id) = o.get("id").and_then(Value::as_str) {
                    set.insert(id.into());
                }
                for v in o.values() {
                    ids(v, set);
                }
            }
            Value::Array(a) => {
                for v in a {
                    ids(v, set);
                }
            }
            _ => {}
        }
    }
    fn assign(v: &mut Value, set: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(ty) = o.get("type").and_then(Value::as_str) {
                    let mut i = 1;
                    while set.contains(&format!("{ty}_{i}")) {
                        i += 1;
                    }
                    let id = format!("{ty}_{i}");
                    set.insert(id.clone());
                    o.insert("id".into(), id.into());
                }
                for v in o.values_mut() {
                    assign(v, set);
                }
            }
            Value::Array(a) => {
                for v in a {
                    assign(v, set);
                }
            }
            _ => {}
        }
    }
    let mut used = std::collections::BTreeSet::new();
    ids(document, &mut used);
    assign(node, &mut used);
}

fn properties(ui: &mut Ui, node: &mut Value, s: &mut Settings) {
    let Some(obj) = node.as_object_mut() else {
        ui.weak("Edit this value in Advanced > JUI source.");
        return;
    };
    let ty = obj.get("type").and_then(Value::as_str).unwrap_or("window").to_owned();
    let sections: &[(&str, &[&str], bool)] = &[
        ("Content", &["title", "id", "label", "value", "elements"], true),
        (
            "Size & layout",
            &[
                "geometry",
                "width",
                "height",
                "aspect",
                "margin",
                "padding",
                "row_height",
                "row_count",
                "direction",
                "size_constraint",
                "edge_constraint",
            ],
            true,
        ),
        ("Tooltips", &["tooltip", "disabled_tooltip"], true),
        (
            "Appearance",
            &[
                "font",
                "foreground_color",
                "border",
                "text_halign",
                "text_valign",
                "image_aspect",
                "image_halign",
                "image_valign",
                "image_region",
            ],
            false,
        ),
        (
            "Behavior",
            &[
                "enabled",
                "visible",
                "encouraged",
                "scrollbars",
                "collapsed",
                "resizable",
                "closable",
                "transparent",
                "accepts_input",
                "multiline",
                "wordwrap",
                "min",
                "max",
                "step",
            ],
            false,
        ),
    ];
    let reveal_id = ui.id().with("reveal-nui-property");
    let reveal = ui.ctx().data_mut(|d| d.remove_temp::<String>(reveal_id));
    let mut remove = None;
    for (section, keys, open) in sections {
        let layout_sizes = *section == "Size & layout" && ty != "window";
        if !layout_sizes
            && !keys.iter().any(|k| {
                obj.get(*k).is_some_and(|v| !v.is_null() || property_default(&ty, k).is_some())
            })
        {
            continue;
        }
        let revealing = reveal.as_deref().is_some_and(|key| keys.contains(&key));
        egui::CollapsingHeader::new(*section)
            .id_salt(("nui-properties", section))
            .open(revealing.then_some(true))
            .default_open(*open)
            .show(ui, |ui| {
                for key in *keys {
                    if let Some(value) = obj.get_mut(*key)
                        && (!value.is_null() || property_default(&ty, key).is_some())
                    {
                        let field = ui.push_id(*key, |ui| {
                            property(ui, key, &ty, value, s, &mut remove);
                            ui.add_space(8.0);
                        });
                        if reveal.as_deref() == Some(*key) {
                            field.response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                }
                if layout_sizes {
                    for (key, label, fixed) in
                        [("width", "Width", 150.0), ("height", "Height", 30.0)]
                    {
                        if !obj.contains_key(key) {
                            ui.horizontal(|ui| {
                                ui.label(label);
                                if ui
                                    .button("Auto")
                                    .on_hover_text("Use a fixed size instead")
                                    .clicked()
                                {
                                    obj.insert(key.into(), json!(fitting_size(obj, key, fixed)));
                                }
                            });
                        }
                    }
                }
            });
    }
    ui.add_space(8.0);
    let (fit_width, fit_height) =
        (fitting_size(obj, "width", 150.0), fitting_size(obj, "height", 30.0));
    ui.menu_button("+ Add property", |ui| {
        for (k, label, val) in [
            ("id", "Element ID", json!("element")),
            ("width", "Width", json!(fit_width)),
            ("height", "Height", json!(fit_height)),
            // The client's own margin: adding the field changes nothing yet.
            ("margin", "Margin", json!(2.0)),
            ("padding", "Padding", json!(0.0)),
            ("enabled", "Enabled", json!(true)),
            ("visible", "Visible", json!(true)),
            ("tooltip", "Tooltip", json!("")),
            ("disabled_tooltip", "Tooltip when control is disabled", json!("")),
            ("encouraged", "Encouraged", json!(false)),
            ("foreground_color", "Foreground color", json!({"r":255,"g":255,"b":255,"a":255})),
            ("size_constraint", "Window size limits", json!({"x":0.0,"y":0.0,"w":0.0,"h":0.0})),
        ] {
            let applicable = match k {
                "size_constraint" => ty == "window",
                // Window size is edited through geometry; element modifiers are not window options.
                _ => ty != "window",
            };
            if applicable && obj.get(k).is_none_or(Value::is_null) && ui.button(label).clicked() {
                obj.insert(k.into(), val);
                ui.ctx().data_mut(|d| d.insert_temp(reveal_id, k.to_owned()));
                ui.ctx().request_repaint();
                ui.close();
            }
        }
    });
    let advanced_keys: Vec<_> = obj
        .iter()
        .filter_map(|(key, value)| {
            if matches!(key.as_str(), "type" | "version" | "root" | "children" | "row_template") {
                return None;
            }
            // NuiElement's unused label/value slots are null placeholders, not
            // additional authorable properties (e.g. Label.label, Button.value).
            if value.is_null()
                && matches!(key.as_str(), "label" | "value")
                && property_default(&ty, key).is_none()
            {
                return None;
            }
            if sections.iter().any(|(_, keys, _)| keys.contains(&key.as_str()))
                && (!value.is_null() || property_default(&ty, key).is_some())
            {
                return None;
            }
            if (key == "draw_list" && value.is_array())
                || (key == "draw_list_scissor" && value.is_boolean())
            {
                // These have a dedicated Draw layers editor, not a second raw editor.
                return None;
            }
            Some(key.clone())
        })
        .collect();
    if !advanced_keys.is_empty() {
        ui.add_space(8.0);
        egui::CollapsingHeader::new("Advanced properties").show(ui, |ui| {
            for key in &advanced_keys {
                let value = obj.get_mut(key).unwrap();
                ui.push_id(key, |ui| {
                    property(ui, key, &ty, value, s, &mut remove);
                });
            }
        });
    }
    if let Some(key) = remove {
        obj.remove(&key);
    }
}

/// A fixed size for a row's height or a column's width that its children
/// fit in, margins included (the client refuses the window otherwise), and a
/// `fallback`-sized control too; for anything else, `fallback`.
fn fitting_size(node: &serde_json::Map<String, Value>, key: &str, fallback: f64) -> f64 {
    let across = match node.get("type").and_then(Value::as_str) {
        Some("row") => "height",
        Some("col") => "width",
        _ => return fallback,
    };
    if key != across {
        return fallback;
    }
    node.get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let size = c.get(key)?.as_f64()?;
            Some(size + 2.0 * c.get("margin").and_then(Value::as_f64).unwrap_or(2.0))
        })
        // Room for a new control of the default size and its margins too.
        .fold(fallback + 4.0, f64::max)
}

/// A missing value must use the property's type, not a generic JSON type menu.
/// Unknown/custom fields remain losslessly editable in the source editor.
fn property_default(ty: &str, key: &str) -> Option<Value> {
    let template = if ty == "window" {
        mg_nui::window()
    } else if mg_nui::ELEMENTS.contains(&ty) {
        mg_nui::template(ty)
    } else {
        Value::Null
    };
    if let Some(value) = template.get(key).filter(|v| !v.is_null()) {
        return Some(value.clone());
    }
    Some(match key {
        "id" | "tooltip" | "disabled_tooltip" | "font" => json!(""),
        "width" => json!(150.0),
        "height" => json!(30.0),
        "margin" => json!(2.0),
        "padding" => json!(0.0),
        "aspect" => json!(1.0),
        "enabled" | "visible" => json!(true),
        "encouraged" | "collapsed" => json!(false),
        "foreground_color" => json!({"r":255,"g":255,"b":255,"a":255}),
        "size_constraint" | "edge_constraint" | "image_region" => {
            json!({"x":0.0,"y":0.0,"w":0.0,"h":0.0})
        }
        _ => return None,
    })
}

fn bindable(ty: &str, key: &str) -> bool {
    (matches!(ty, "draw" | "chart_slot")
        || matches!(
            key,
            "value"
                | "label"
                | "title"
                | "geometry"
                | "size_constraint"
                | "edge_constraint"
                | "enabled"
                | "visible"
                | "tooltip"
                | "resizable"
                | "collapsed"
                | "closable"
                | "foreground_color"
                | "elements"
                | "row_count"
                | "min"
                | "step"
                | "image_region"
                | "image_aspect"
                | "image_halign"
                | "image_valign"
                | "font"
                | "text_halign"
                | "text_valign"
                | "disabled_tooltip"
                | "encouraged"
        )
        || (key == "border" && ty == "window")
        || (key == "max" && ty != "textedit"))
        && !matches!(key, "type" | "order" | "render" | "arrayBinds")
        && !(key == "value" && ty == "chart")
        && !(key == "elements" && matches!(ty, "options" | "tabbar"))
}

fn property(
    ui: &mut Ui,
    key: &str,
    ty: &str,
    value: &mut Value,
    s: &mut Settings,
    remove: &mut Option<String>,
) {
    let label = match key {
        "id" => "Element ID".into(),
        "disabled_tooltip" => "Tooltip when control is disabled".into(),
        "value" if matches!(ty, "label" | "text" | "textedit") => "Text".into(),
        "label" if matches!(ty, "button" | "button_select" | "check") => "Label".into(),
        "geometry" => "Window bounds".into(),
        "size_constraint" => "Window size limits".into(),
        _ => {
            let mut label = key.replace('_', " ");
            if let Some(c) = label.get_mut(..1) {
                c.make_ascii_uppercase();
            }
            label
        }
    };
    let can_bind = bindable(ty, key);
    let mut bind = value.get("bind").and_then(Value::as_str).map(str::to_owned);
    let heading = ui
        .horizontal(|ui| {
            let heading = if let Value::Bool(flag) = value {
                ui.checkbox(flag, &label)
            } else {
                ui.label(&label)
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button("…", |ui| {
                    if let Some(name) = &bind {
                        if ui.button("Use a constant value").clicked() {
                            *value = s.bindings.get(name).map_or(Value::Null, |b| b.value.clone());
                            ui.close();
                        }
                    } else if can_bind && ui.button("Make dynamic (bind)").clicked() {
                        let mut i = 1;
                        while s.bindings.contains_key(&format!("{key}_{i}")) {
                            i += 1;
                        }
                        let name = format!("{key}_{i}");
                        s.bindings.insert(
                            name.clone(),
                            Binding { value: value.clone(), watch: false, ..Default::default() },
                        );
                        *value = json!({"bind":name});
                        ui.close();
                    }
                    if can_bind && !s.bindings.is_empty() {
                        ui.menu_button("Use existing bind", |ui| {
                            for name in s.bindings.keys() {
                                if ui.selectable_label(bind.as_ref() == Some(name), name).clicked()
                                {
                                    if value.get("bind").is_some() {
                                        value["bind"] = json!(name);
                                    } else {
                                        *value = json!({"bind":name});
                                    }
                                    ui.close();
                                }
                            }
                        });
                    }
                    let text_property = matches!(key, "title" | "tooltip" | "disabled_tooltip")
                        || (key == "value" && matches!(ty, "label" | "text" | "textedit"))
                        || (key == "label"
                            && matches!(ty, "button" | "button_select" | "check" | "textedit"))
                        || (key == "text" && ty == "draw")
                        || (key == "legend" && ty == "chart_slot");
                    if bind.is_none() && text_property {
                        if value.get("strref").is_some() {
                            if ui.button("Use typed text").clicked() {
                                *value = json!("");
                                ui.close();
                            }
                        } else if value.is_string()
                            && ui.button("Use TLK string (StrRef)").clicked()
                        {
                            *value = json!({"strref":0});
                            ui.close();
                        }
                    }
                    if !matches!(key, "geometry" | "title")
                        && ui.button("Remove property").clicked()
                    {
                        *remove = Some(key.into());
                        ui.close();
                    }
                });
            });
            heading
        })
        .inner;
    bind = value.get("bind").and_then(Value::as_str).map(str::to_owned);
    if key == "disabled_tooltip" {
        ui.weak(
            "Shown on hover while this control is disabled, e.g. why an action is unavailable.",
        );
    }
    if let Some(name) = bind {
        ui.label(
            egui::RichText::new(format!("Dynamic · {name}")).color(ui.visuals().selection.bg_fill),
        );
        if let Some(binding) = s.bindings.get_mut(&name) {
            ui.weak("Initial value");
            fields::value(ui, &mut binding.value, 0);
        } else {
            ui.weak("Add an initial value in Bindings.");
        }
        fields::binding_options(ui, value);
    } else if let Value::String(text) = value {
        ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY))
            .labelled_by(heading.id);
    } else if let Some(id) = value.get_mut("strref") {
        ui.horizontal(|ui| {
            ui.label("TLK StrRef");
            scalar(ui, id);
        });
    } else if key == "elements"
        && let Some(elements) = value.as_array_mut()
    {
        fields::options(ui, elements, ty == "combo");
    } else if key == "size_constraint" && value.is_object() {
        for (field, label) in [
            ("x", "Minimum width"),
            ("y", "Minimum height"),
            ("w", "Maximum width"),
            ("h", "Maximum height"),
        ] {
            ui.horizontal(|ui| {
                ui.label(label);
                scalar(ui, &mut value[field]);
            });
        }
        ui.weak("0 leaves that limit unrestricted.");
    } else if value.is_null() {
        if let Some(default) = property_default(ty, key) {
            if ui.button("Set value…").clicked() {
                *value = default;
            }
        } else {
            ui.weak("Unset/custom value. Edit in Advanced > JUI source.");
        }
    } else if !value.is_boolean() {
        scalar(ui, value);
    }
}

fn scalar(ui: &mut Ui, v: &mut Value) {
    match v {
        Value::String(text) => {
            ui.add(egui::TextEdit::singleline(text).desired_width(ui.available_width().max(0.0)));
        }
        Value::Bool(b) => {
            ui.checkbox(b, "On");
        }
        Value::Number(n) => {
            if let Some(mut i) = n.as_i64() {
                if ui.add(egui::DragValue::new(&mut i)).changed() {
                    *v = json!(i);
                }
            } else if let Some(mut f) = n.as_f64()
                && ui.add(egui::DragValue::new(&mut f).speed(0.1)).changed()
            {
                *v = json!(f);
            }
        }
        Value::Object(o) if o.len() <= 4 && o.values().all(Value::is_number) => {
            for (k, v) in o {
                ui.horizontal(|ui| {
                    ui.label(match k.as_str() {
                        "w" => "Width",
                        "h" => "Height",
                        "x" => "X",
                        "y" => "Y",
                        "r" => "Red",
                        "g" => "Green",
                        "b" => "Blue",
                        "a" => "Alpha",
                        _ => k,
                    });
                    scalar(ui, v);
                });
            }
        }
        Value::Null | Value::Array(_) | Value::Object(_) => fields::value(ui, v, 0),
    }
}

fn resolved<'a>(v: &'a Value, s: &'a Settings) -> &'a Value {
    v.get("bind").and_then(Value::as_str).and_then(|b| s.bindings.get(b)).map_or(v, |b| &b.value)
}
#[cfg(test)]
mod tests {
    #[test]
    fn option_labels_are_static_but_selection_and_combo_entries_are_bindable() {
        for ty in ["options", "tabbar"] {
            assert!(!super::bindable(ty, "elements"));
            assert!(super::bindable(ty, "value"));
            assert!(super::bindable(ty, "enabled"));
        }
        assert!(super::bindable("combo", "elements"));
    }

    use super::*;
    use egui_kittest::{
        Harness,
        kittest::{NodeT, Queryable},
    };

    fn app() -> Moonglow {
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
        create(&mut app, "nui_test");
        app.actions.clear();
        app
    }

    #[test]
    fn nui_structure_edits_preserve_extensions_and_avoid_duplicate_ids() {
        let mut v = mg_nui::window();
        v["root"]["children"][0]["id"] = "first".into();
        v["root"]["children"][0]["extension"] = json!({"untouched":[1,2,3]});
        let mut selected = "/root/children/0".to_string();
        rearrange(&mut v, &mut selected, 2);
        assert_eq!(selected, "/root/children/1");
        assert_eq!(v["root"]["children"][1]["extension"], json!({"untouched":[1,2,3]}));
        assert_ne!(v["root"]["children"][0]["id"], v["root"]["children"][1]["id"]);
        rearrange(&mut v, &mut selected, 1);
        assert_eq!(selected, "/root/children/2");
        rearrange(&mut v, &mut selected, 3);
        assert_eq!(v["root"]["children"].as_array().unwrap().len(), 2);
        v["root"] = mg_nui::template("list");
        selected = "/root/row_template/0/0".into();
        v["root"]["row_template"] = json!([[mg_nui::template("label"), 150.0, true]]);
        rearrange(&mut v, &mut selected, 2);
        assert_eq!(v["root"]["row_template"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn nui_palette_adds_to_selected_layout_or_nearest_parent() {
        let mut v = mg_nui::window();
        let mut selected = "/root/children/0".to_owned();
        design::insert(&mut v, &mut selected, "check");
        assert_eq!(selected, "/root/children/2");
        assert_eq!(v.pointer(&selected).unwrap()["type"], "check");
        let first_id = v.pointer(&selected).unwrap()["id"].clone();
        design::insert(&mut v, &mut selected, "check");
        assert_ne!(v.pointer(&selected).unwrap()["id"], first_id);
        v["root"] = mg_nui::template("list");
        selected = "/root".into();
        design::insert(&mut v, &mut selected, "button");
        assert_eq!(selected, "/root/row_template/0/0");
        assert_eq!(v.pointer(&selected).unwrap()["type"], "button");
    }

    #[test]
    fn nui_canvas_selection_and_duplicate_use_workspace_undo() {
        let key = mg_nui::key("nui_test", ResType::JUI);
        let mut h = Harness::builder().with_size(egui::vec2(1200.0, 850.0)).build_ui_state(
            |ui, app: &mut Moonglow| {
                super::ui(app, ui, Some(key));
                app.run_actions();
            },
            app(),
        );
        h.run();
        let before = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        h.get_by_label("Element actions").scroll_to_me();
        h.run();
        h.get_by_label("Element actions").click();
        h.run();
        h.get_by_label_contains("Duplicate element").click();
        h.run();
        let ws = h.state_mut().ws.as_mut().unwrap();
        let doc = mg_nui::parse(ws.module.get(&key).unwrap()).unwrap();
        assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 3);
        assert_ne!(doc["root"]["children"][1]["id"], doc["root"]["children"][2]["id"]);
        ws.undo().unwrap();
        assert_eq!(ws.module.get(&key).unwrap(), before);
    }

    fn keyboard_harness() -> Harness<'static, Moonglow> {
        Harness::builder().with_size(egui::vec2(1200.0, 850.0)).build_ui_state(
            |ui, app: &mut Moonglow| {
                app.shortcuts(ui);
                super::ui(app, ui, Some(mg_nui::key("nui_test", ResType::JUI)));
                app.run_actions();
            },
            app(),
        )
    }

    #[test]
    fn nui_navigation_keeps_one_aligned_row_at_narrow_widths() {
        let mut h = keyboard_harness();
        for width in [1200.0, 820.0, 640.0] {
            h.set_size(egui::vec2(width, 850.0));
            h.run();
            let first = h.get_by_label("Design").rect();
            for label in ["Bindings", "Views & events", "Event script…", "Advanced"] {
                // Some controls are outside the horizontal viewport at 640px;
                // their layout must still stay on the same row.
                let rect = h.get_by_label(label).rect();
                assert!(
                    (rect.center().y - first.center().y).abs() < 0.1,
                    "{width}: {label}: {rect:?}"
                );
                assert!((rect.height() - first.height()).abs() < 0.1, "{width}: {label}: {rect:?}");
            }
            let new = h.get_by_label("New NUI…").rect();
            let load = h.get_by_label("Load NUI…").rect();
            assert!((new.center().y - load.center().y).abs() < 0.1, "{width}: document actions");
            assert!(new.bottom() < first.top(), "{width}: document actions above navigation");
        }
    }

    #[test]
    fn nui_new_and_load_do_not_replace_the_current_resource() {
        let mut state = State::default();
        let mut h = Harness::builder().build_ui_state(
            |ui, app: &mut Moonglow| {
                design::document_actions(
                    ui,
                    &mut state,
                    &app.ws.as_ref().unwrap().module,
                    &mut app.actions,
                );
                app.run_actions();
            },
            app(),
        );
        h.run();
        let key = mg_nui::key("nui_test", ResType::JUI);
        let before = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
        h.get_by_label("New NUI…").click();
        h.run();
        h.get_by_role(egui::accesskit::Role::TextInput).click();
        h.run();
        h.get_by_role(egui::accesskit::Role::TextInput).type_text("nui_test");
        h.run();
        h.get_all_by_value("Resource already exists: nui_test.jui")
            .next()
            .expect("duplicate resource validation is visible");
        h.get_by_label("Cancel").click();
        h.run();
        h.get_by_label("Load NUI…").click();
        h.run();
        h.get_by_label("nui_test").click();
        h.run();
        assert_eq!(h.state().ws.as_ref().unwrap().module.get(&key).unwrap(), before);
        assert!(h.state().dock.find_tab(&Tab::Nui(Some(key))).is_some());
        h.get_by_label("New NUI…").click();
        h.run();
        h.get_by_role(egui::accesskit::Role::TextInput).click();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.get_by_role(egui::accesskit::Role::TextInput).type_text("new_nui");
        h.run();
        h.get_by_label("Create NUI").click();
        h.run();
        let new_key = mg_nui::key("new_nui", ResType::JUI);
        assert!(h.state().ws.as_ref().unwrap().module.contains(&new_key));
        assert_eq!(h.state().ws.as_ref().unwrap().module.get(&key).unwrap(), before);
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        assert!(!h.state().ws.as_ref().unwrap().module.contains(&new_key));
    }

    #[test]
    fn nui_design_creates_connects_watches_and_undoes_without_changing_page() {
        let mut h = keyboard_harness();
        h.run();
        h.get_by_label("Canvas Label · Label").click();
        h.run();
        let before = keyboard_document(&h);
        h.get_by_label("+ New bind for property").click();
        h.run();
        h.get_by_label("Cancel bind").click();
        h.run();
        assert_eq!(keyboard_document(&h), before);
        h.get_by_label("+ New bind for property").click();
        h.run();
        h.get_by_label("Create & connect").click();
        h.run();
        assert_eq!(
            keyboard_document(&h)["root"]["children"][0]["value"],
            json!({"bind":"label_value"})
        );
        h.get_all_by_label("+ Add event").last().unwrap().click();
        h.run();
        h.get_by_value("Choose action…").click();
        h.run();
        h.get_by_label("Close window").click();
        h.run();
        h.get_by_label("Save event").click();
        h.run();
        let read = |h: &Harness<'_, Moonglow>| {
            Settings::parse(
                h.state()
                    .ws
                    .as_ref()
                    .unwrap()
                    .module
                    .get(&mg_nui::key("nui_test", ResType::TXT))
                    .unwrap(),
            )
            .unwrap()
        };
        assert_eq!(read(&h).bindings["label_value"].value, json!("Label"));
        assert!(read(&h).actions.iter().any(|r| r.event == "watch"
            && r.element == "label_value"
            && r.action == mg_nui::Action::Close));
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert!(!read(&h).actions.iter().any(|r| r.event == "watch"));
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.run();
        assert_eq!(keyboard_document(&h), before);
        assert!(!read(&h).bindings.contains_key("label_value"));
        h.get_by_label("Property binding");
        h.get_by_label("Canvas Label · Label");
    }

    #[test]
    fn nui_unused_null_slots_do_not_offer_generic_json_types() {
        for ty in mg_nui::ELEMENTS {
            let mut node = mg_nui::template(ty);
            let mut h = Harness::builder().build_ui(|ui| {
                properties(ui, &mut node, &mut Settings::default());
            });
            h.run();
            assert!(h.query_by_label("Advanced properties").is_none(), "standard template: {ty}");
            if let Some(header) = h.query_by_label("Advanced properties") {
                header.click();
            }
            h.run();
            assert!(h.query_by_label("Set value…").is_none(), "{ty}");
        }
        let mut node = mg_nui::template("button");
        node["label"] = Value::Null;
        let mut h = Harness::builder().build_ui(|ui| {
            properties(ui, &mut node, &mut Settings::default());
        });
        h.run();
        assert!(h.query_by_label("Advanced properties").is_none());
        h.get_by_label("Set value…").click();
        h.run();
        assert!(h.query_by_label("Array").is_none());
        drop(h);
        assert_eq!(node["label"], "Button");
        assert!(node["value"].is_null());
    }

    #[test]
    fn nui_standard_properties_stay_out_of_advanced_and_extensions_survive() {
        for mut node in [mg_nui::window(), mg_nui::template("row"), mg_nui::template("image")] {
            let window = node.get("type").is_none();
            if !window {
                node["aspect"] = json!(1.5);
                node["encouraged"] = json!(true);
            }
            if node["type"] == "image" {
                node["image_region"] = json!({"x":0,"y":0,"w":32,"h":32});
                node["draw_list"] = json!([]);
                node["draw_list_scissor"] = json!(false);
            }
            let before = node.clone();
            let mut h = Harness::builder()
                .build_ui(|ui| properties(ui, &mut node, &mut Settings::default()));
            h.run();
            assert!(h.query_by_label("Advanced properties").is_none());
            assert!(h.query_by_label("Size & layout").is_some());
            if before["type"] == "row" {
                assert_eq!(h.query_all_by_label("Auto").count(), 2);
            }
            drop(h);
            assert_eq!(node, before, "opening properties must not mutate the document");
            node["vendor_extension"] = json!({"payload":[1,2,3]});
            let before = node.clone();
            let mut h = Harness::builder()
                .build_ui(|ui| properties(ui, &mut node, &mut Settings::default()));
            h.run();
            assert!(h.query_by_label("Advanced properties").is_some());
            drop(h);
            assert_eq!(node, before);
        }
    }

    #[test]
    fn nui_event_code_preview_is_read_only_and_distinguishes_module_source() {
        let mut h = keyboard_harness();
        h.run();
        let before = keyboard_document(&h);
        let settings_key = mg_nui::key("nui_test", ResType::TXT);
        let before_settings =
            h.state().ws.as_ref().unwrap().module.get(&settings_key).unwrap().to_vec();
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        h.get_by_label("Preview code").click();
        h.run();
        assert!(h.query_by_label("Clicked · Control: mg_close · selected event only").is_some());
        let generated = mg_nui::event_source(&Settings::parse(&before_settings).unwrap()).unwrap();
        assert!(h.query_by_value(&generated).is_some());
        h.get_by_label("Module script").click();
        h.run();
        assert!(
            h.query_by_label("No event script yet. Use Build & compile to create it.").is_some()
        );
        let script_key = mg_nui::key("nui_test_e", ResType::NSS);
        assert!(!h.state().ws.as_ref().unwrap().module.contains(&script_key));
        h.get_by_label("Close preview").click();
        h.run();
        h.get_by_label("Edit event").click();
        h.run();
        h.get_by_label("Preview code").click();
        h.run();
        assert!(h.query_by_label("Unsaved event draft · selected event only").is_some());
        h.get_by_label("Close preview").click();
        h.run();
        h.get_by_label("Cancel event").click();
        h.run();
        let manual = "// Manually authored handler\nvoid main() {}";
        h.state_mut().ws.as_mut().unwrap().module.set(script_key, manual.as_bytes().to_vec());
        h.get_by_label("Event script…").click();
        h.run();
        assert!(h.query_by_label("All configured events").is_some());
        h.get_by_label("Module script").click();
        h.run();
        assert!(h.query_by_value(manual).is_some());
        assert_eq!(keyboard_document(&h), before);
        assert_eq!(
            h.state().ws.as_ref().unwrap().module.get(&settings_key).unwrap(),
            before_settings
        );
        h.get_by_label("Open in script editor").click();
        h.run();
        assert!(h.state().dock.find_tab(&Tab::Script(script_key)).is_some());
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            crate::script_view::ui(h.state_mut(), ui, script_key);
        });
        output.textures_delta.clear();
        let unsaved = "// Unsaved manual edit\nvoid main() {}";
        h.state_mut().scripts.get_mut(&script_key).unwrap().text = unsaved.into();
        h.get_by_label("Event script…").click();
        h.run();
        h.get_by_label("Module script").click();
        h.run();
        assert!(
            h.query_by_label("Full handler · unsaved changes from the script editor").is_some()
        );
        assert!(h.query_by_value(unsaved).is_some());
        assert_eq!(
            h.state().ws.as_ref().unwrap().module.get(&script_key).unwrap(),
            manual.as_bytes()
        );
    }

    #[test]
    fn nui_inspector_adds_color_and_window_limits_with_undo() {
        let mut h = keyboard_harness();
        h.run();
        h.get_by_label("Canvas Label · Label").click();
        h.run();
        let before = keyboard_document(&h);
        h.get_by_label("+ Add property").click();
        h.run();
        h.get_by_label("Foreground color").click();
        h.run();
        assert!(h.query_by_label("Foreground color").is_some(), "new color field must be expanded");
        let colored = keyboard_document(&h);
        assert_eq!(
            colored["root"]["children"][0]["foreground_color"],
            json!({"r":255,"g":255,"b":255,"a":255})
        );
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        assert_eq!(keyboard_document(&h), before);
        h.run();
        h.get_by_label("Window").click();
        h.run();
        h.get_by_label("+ Add property").scroll_to_me();
        h.run();
        h.get_by_label("+ Add property").click();
        h.run();
        h.query_all_by_label("Window size limits")
            .find(|n| n.accesskit_node().role() != egui::accesskit::Role::Label)
            .expect("Add window limits menu entry")
            .click();
        h.run();
        assert!(h.query_by_label("Minimum width").is_some(), "new limits must be expanded");
        assert!(h.query_by_label("Advanced properties").is_none());
        let limited = keyboard_document(&h);
        assert_eq!(limited["size_constraint"], json!({"x":0.0,"y":0.0,"w":0.0,"h":0.0}));
        assert!(
            !mg_nui::validate(&limited, &Settings::default())
                .iter()
                .any(|d| d.severity == Severity::Error)
        );
    }

    #[test]
    fn nui_draw_layers_start_unclipped_and_preserve_explicit_clipping() {
        let mut h = keyboard_harness();
        h.run();
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        h.get_by_label("Draw layers").scroll_to_me();
        h.run();
        h.get_by_label("Draw layers").click();
        h.run();
        h.get_by_label("+ Draw primitive").scroll_to_me();
        h.run();
        h.get_by_label("+ Draw primitive").click();
        h.run();
        h.get_by_label("Rectangle").click();
        h.run();
        let node = &keyboard_document(&h)["root"]["children"][1];
        assert_eq!(node["draw_list_scissor"], false);
        assert_eq!(node["draw_list"][0]["type"], 7);
        h.get_by_label("On").scroll_to_me();
        h.run();
        h.get_by_label("On").click();
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"][1]["draw_list_scissor"], true);
        h.get_by_label("+ Draw primitive").scroll_to_me();
        h.run();
        h.get_by_label("+ Draw primitive").click();
        h.run();
        h.get_by_label("Line").click();
        h.run();
        let doc = keyboard_document(&h);
        assert_eq!(doc["root"]["children"][1]["draw_list_scissor"], true);
        assert_eq!(doc["root"]["children"][1]["draw_list"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn nui_draw_inspector_does_not_mutate_elements_without_drawings() {
        let mut h = keyboard_harness();
        h.run();
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        let before = keyboard_document(&h);
        h.get_by_label("Draw layers").click();
        h.run();
        assert_eq!(
            keyboard_document(&h),
            before,
            "Opening Draw layers must not add null draw_list"
        );
        h.get_by_label("Canvas Label · Label").click();
        h.run();
        assert_eq!(
            keyboard_document(&h),
            before,
            "Selection with Draw layers open must not alter the document"
        );
    }

    #[test]
    fn nui_inspector_authors_hover_states_with_undo() {
        for (label, key, expected) in [
            ("Tooltip when control is disabled", "disabled_tooltip", json!("")),
            ("Encouraged", "encouraged", json!(false)),
        ] {
            let mut h = keyboard_harness();
            h.run();
            h.get_by_label("Canvas Button · Close").click();
            h.run();
            let before = keyboard_document(&h);
            h.get_by_label("+ Add property").click();
            h.run();
            h.query_by_label(label)
                .expect("Supported element property missing from Add property")
                .click();
            h.run();
            let authored = keyboard_document(&h);
            assert_eq!(authored["root"]["children"][1][key], expected);
            if key == "disabled_tooltip" {
                assert!(h.query_by_label("Tooltips").is_some());
                assert!(
                    h.query_by_label(label).is_some(),
                    "Added tooltip must be visible immediately"
                );
                assert!(h.query_by_label("Advanced properties").is_none());
            }
            assert!(
                !mg_nui::validate(&authored, &Settings::default())
                    .iter()
                    .any(|d| d.severity == Severity::Error)
            );
            h.state_mut().ws.as_mut().unwrap().undo().unwrap();
            assert_eq!(keyboard_document(&h), before);
            h.run();
            h.get_by_label("Window").click();
            h.run();
            h.get_by_label("+ Add property").click();
            h.run();
            assert!(h.query_by_label(label).is_none(), "Element-only property on Window");
        }
    }

    fn keyboard_document(h: &Harness<'_, Moonglow>) -> Value {
        mg_nui::parse(
            h.state()
                .ws
                .as_ref()
                .unwrap()
                .module
                .get(&mg_nui::key("nui_test", ResType::JUI))
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn nui_strref_can_be_authored_then_bound_in_inspector() {
        let mut h = Harness::builder().build_ui_state(
            |ui, state: &mut (Value, Settings)| {
                property(ui, "value", "label", &mut state.0, &mut state.1, &mut None);
            },
            (json!("Text"), Settings::default()),
        );
        h.run();
        h.get_by_label("…").click();
        h.run();
        h.get_by_label("Use TLK string (StrRef)").click();
        h.run();
        assert_eq!(h.state().0, json!({"strref":0}));
        h.get_by_label("…").click();
        h.run();
        h.get_by_label("Make dynamic (bind)").click();
        h.run();
        assert_eq!(h.state().0, json!({"bind":"value_1"}));
        assert_eq!(h.state().1.bindings["value_1"].value, json!({"strref":0}));
    }

    #[test]
    fn nui_inspector_reuses_existing_bind_and_retains_formatting() {
        for initial in [json!("Label"), json!({"bind":"old","number_precision":2,"text_flags":1})] {
            let mut settings = Settings::default();
            settings
                .bindings
                .insert("shared".into(), Binding { value: json!(-12.345), ..Default::default() });
            settings
                .bindings
                .insert("old".into(), Binding { value: json!("Unchanged"), ..Default::default() });
            let original = settings.clone();
            let mut h = Harness::builder().build_ui_state(
                |ui, state: &mut (Value, Settings)| {
                    property(ui, "value", "label", &mut state.0, &mut state.1, &mut None);
                },
                (initial.clone(), settings),
            );
            h.run();
            h.get_by_label("…").click();
            h.run();
            h.query_by_label_contains("Use existing bind")
                .expect("Missing existing-bind selector")
                .click();
            h.run();
            h.get_by_label("shared").click();
            h.run();
            assert_eq!(h.state().0["bind"], "shared");
            if initial.is_object() {
                assert_eq!(h.state().0["number_precision"], 2);
                assert_eq!(h.state().0["text_flags"], 1);
            }
            assert_eq!(h.state().1, original, "Reusing a bind must not create or change bindings");
            let mut doc = mg_nui::window();
            doc["root"]["children"][0]["value"] = h.state().0.clone();
            assert!(
                !mg_nui::validate(&doc, &h.state().1).iter().any(|d| d.severity == Severity::Error)
            );
        }
        let mut settings = Settings::default();
        settings
            .bindings
            .insert("shared".into(), Binding { value: json!(10), ..Default::default() });
        let mut h = Harness::builder().build_ui_state(
            |ui, state: &mut (Value, Settings)| {
                property(ui, "max", "textedit", &mut state.0, &mut state.1, &mut None);
            },
            (json!(128), settings),
        );
        h.run();
        h.get_by_label("…").click();
        h.run();
        assert!(
            h.query_by_label_contains("Use existing bind").is_none(),
            "TextEdit max must remain static"
        );
    }

    #[test]
    fn nui_documents_reuse_launcher_and_share_one_workspace() {
        let mut app = app();
        app.run_now(Action::OpenTab(Tab::Nui(None)));
        let launcher = app.dock.find_tab(&Tab::Nui(None)).unwrap();
        let first = Tab::Nui(Some(mg_nui::key("nui_test", ResType::JUI)));
        app.run_now(Action::OpenTab(first.clone()));
        assert!(app.dock.find_tab(&Tab::Nui(None)).is_none());
        assert_eq!(app.dock.find_tab(&first).unwrap(), launcher);
        // Creating a second document replaces only the launcher, not the first document.
        app.run_now(Action::OpenTab(Tab::Nui(None)));
        create(&mut app, "second");
        app.run_actions();
        let second = Tab::Nui(Some(mg_nui::key("second", ResType::JUI)));
        assert_eq!(app.dock.find_tab(&second).unwrap().node_path(), launcher.node_path());
        assert!(app.dock.find_tab(&first).is_some());
        assert!(app.dock.find_tab(&Tab::Nui(None)).is_none());
        app.run_now(Action::OpenTab(Tab::Nui(None)));
        app.run_now(Action::OpenTab(first.clone()));
        assert!(app.dock.find_tab(&Tab::Nui(None)).is_none());
        let leaf = app.dock.leaf(launcher.node_path()).unwrap();
        assert_eq!(leaf.tabs.len(), 2);
        assert_eq!(leaf.tabs[leaf.active.0], first);
    }

    #[test]
    fn nui_interact_keeps_palette_properties_and_tree_editing_available() {
        let mut h = keyboard_harness();
        h.run();
        h.get_by_label("Interact").click();
        h.run();
        let original = keyboard_document(&h);
        h.get_by_label("Button").click();
        h.run();
        let added = keyboard_document(&h);
        assert_eq!(added["root"]["children"].as_array().unwrap().len(), 3);
        h.get_by_label("Element actions").scroll_to_me();
        h.run();
        h.get_by_label("Element actions").click();
        h.run();
        h.get_by_label_contains("Duplicate element").click();
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 4);
        // Delete over the simulated controls must never delete a design element.
        h.hover_at(h.get_by_label("Canvas Button · Close").rect().center());
        h.run();
        h.key_press(egui::Key::Delete);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 4);
        h.hover_at(h.get_by_label("Layers").rect().center());
        h.run();
        h.key_press(egui::Key::Delete);
        h.run();
        assert_eq!(keyboard_document(&h), added);
        let ws = h.state_mut().ws.as_mut().unwrap();
        for _ in 0..3 {
            ws.undo().unwrap();
        }
        assert_eq!(keyboard_document(&h), original);
    }

    #[test]
    fn nui_window_resize_works_in_interact_and_preserves_one_undo() {
        for live in [false, true] {
            let mut h = keyboard_harness();
            h.run();
            h.get_by_label("Scale").click();
            h.run();
            h.get_by_label("150%").click();
            h.run();
            if live {
                h.get_by_label("Interact").click();
                h.run();
            }
            h.get_by_label("Canvas window").click();
            h.run();
            let doc = keyboard_document(&h);
            let revision = h.state().ws.as_ref().unwrap().revision();
            let header = h.get_by_label("Canvas window").rect();
            let from = h.get_by_label("Resize window width and height").rect().center();
            // Measure the full window, independently of its title-bar buttons.
            let scale = (from.x - header.left()) / doc["geometry"]["w"].as_f64().unwrap() as f32;
            h.hover_at(from);
            h.run();
            h.drag_at(from);
            h.run();
            h.hover_at(from + egui::vec2(12.0, 12.0));
            h.run();
            let to = from + egui::vec2(50.0, 30.0);
            h.hover_at(to);
            h.run();
            assert_eq!(keyboard_document(&h), doc);
            assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision);
            let during = h.get_by_label("Canvas window").rect();
            assert!((during.min - header.min).length() < 1.0, "window origin must stay fixed");
            assert!((during.width() - header.width() - 50.0).abs() < 1.5);
            h.drop_at(to);
            h.run();
            let resized = keyboard_document(&h);
            assert_eq!(resized["geometry"]["w"], (420.0 + 50.0 / scale).round());
            assert_eq!(resized["geometry"]["h"], (240.0 + 30.0 / scale).round());
            assert_eq!(resized["geometry"]["x"], doc["geometry"]["x"]);
            assert_eq!(resized["geometry"]["y"], doc["geometry"]["y"]);
            assert_eq!(resized["resizable"], doc["resizable"]);
            assert!(resized.get("width").is_none());
            assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision + 1);
            let from = h.get_by_label("Resize window height").rect().center();
            h.hover_at(from);
            h.run();
            h.drag_at(from);
            h.run();
            h.hover_at(from + egui::vec2(0.0, 30.0));
            h.run();
            h.key_press(egui::Key::Escape);
            h.run();
            h.drop_at(from + egui::vec2(0.0, 30.0));
            h.run();
            assert_eq!(keyboard_document(&h), resized);
            h.state_mut().ws.as_mut().unwrap().undo().unwrap();
            assert_eq!(keyboard_document(&h), doc);
        }
    }

    #[test]
    fn nui_keyboard_delete_duplicate_move_and_module_undo_redo() {
        let mut h = keyboard_harness();
        h.run();
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::D);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 3);
        let id = keyboard_document(&h)["root"]["children"][2]["id"].clone();
        h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowUp);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"][1]["id"], id);
        h.key_press(egui::Key::Delete);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 2);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 3);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn nui_tree_collapses_and_reveals_canvas_selection() {
        let mut h = keyboard_harness();
        h.run();
        let before = keyboard_document(&h);
        h.get_by_label("Collapse Column").click();
        h.run();
        assert!(h.query_by_label("Button · Close").is_none());
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        assert!(
            h.get_all_by_label("Button · Close")
                .any(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        );
        h.get_by_label("Collapse all").click();
        h.run();
        assert!(h.query_by_label("Collapse Column").is_none());
        assert!(h.query_by_label("Expand Column").is_none());
        h.get_by_label("Expand all").click();
        h.run();
        assert!(
            h.get_all_by_label("Button · Close")
                .any(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        );
        assert_eq!(keyboard_document(&h), before);
    }

    #[test]
    fn nui_resize_list_commits_once_and_escape_cancels() {
        let mut h = keyboard_harness();
        let key = mg_nui::key("nui_test", ResType::JUI);
        let mut doc = mg_nui::window();
        let mut list = mg_nui::template("list");
        list["width"] = 200.0.into();
        list["height"] = 100.0.into();
        doc["root"]["children"] = json!([list]);
        h.state_mut().ws.as_mut().unwrap().module.set(key, serde_json::to_vec(&doc).unwrap());
        h.run();
        h.get_by_label("Canvas List").click();
        h.run();
        let revision = h.state().ws.as_ref().unwrap().revision();
        let from = h.get_by_label("Resize width and height").rect().center();
        h.hover_at(from);
        h.run();
        h.drag_at(from);
        h.run();
        h.hover_at(from + egui::vec2(12.0, 12.0));
        h.run();
        let to = from + egui::vec2(60.0, 50.0);
        h.hover_at(to);
        h.run();
        assert_eq!(keyboard_document(&h), doc, "drag must not write intermediate module revisions");
        assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision);
        h.drop_at(to);
        h.run();
        let resized = keyboard_document(&h);
        assert_eq!(resized["root"]["children"][0]["width"], 260.0);
        assert_eq!(resized["root"]["children"][0]["height"], 150.0);
        assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision + 1);
        let from = h.get_by_label("Resize height").rect().center();
        h.hover_at(from);
        h.run();
        h.drag_at(from);
        h.run();
        h.hover_at(from + egui::vec2(0.0, 30.0));
        h.run();
        h.key_press(egui::Key::Escape);
        h.run();
        h.drop_at(from + egui::vec2(0.0, 30.0));
        h.run();
        assert_eq!(keyboard_document(&h), resized);
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        assert_eq!(keyboard_document(&h), doc);
    }

    #[test]
    fn nui_keyboard_navigation_and_custom_delete_binding() {
        let mut h = keyboard_harness();
        h.state_mut().settings.key_bindings.insert("nui-delete".into(), vec!["F6".into()]);
        h.run();
        h.get_by_label("Button · Close").click();
        h.run();
        h.key_press(egui::Key::Delete);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 2);
        h.key_press(egui::Key::ArrowUp);
        h.run();
        h.key_press(egui::Key::F6);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 1);
        assert_eq!(keyboard_document(&h)["root"]["children"][0]["id"], "mg_close");
    }

    #[test]
    fn nui_keyboard_respects_text_focus_popups_and_editor_bounds() {
        let mut h = keyboard_harness();
        h.run();
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        let before = keyboard_document(&h);
        // Search text gets Delete and clipboard keys even with an element selected.
        h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().click();
        h.run();
        h.key_press(egui::Key::Delete);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::D);
        h.event(egui::Event::Cut);
        h.run();
        assert_eq!(keyboard_document(&h), before);
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        h.get_by_label("Element actions").click();
        h.run();
        h.key_press(egui::Key::Delete);
        h.run();
        assert_eq!(keyboard_document(&h), before);
        h.key_press(egui::Key::Escape);
        h.run();
        h.hover_at(egui::pos2(-10.0, -10.0));
        h.key_press(egui::Key::Delete);
        h.run();
        assert_eq!(keyboard_document(&h), before);
    }

    #[test]
    fn nui_keyboard_native_clipboard_events_cut_paste_and_undo() {
        let mut h = keyboard_harness();
        h.run();
        h.get_by_label("Canvas Button · Close").click();
        h.run();
        h.event(egui::Event::Copy);
        h.run();
        h.event(egui::Event::Paste(String::new()));
        h.run();
        let doc = keyboard_document(&h);
        assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 3);
        assert_ne!(doc["root"]["children"][1]["id"], doc["root"]["children"][2]["id"]);
        h.event(egui::Event::Cut);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 2);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run();
        assert_eq!(keyboard_document(&h)["root"]["children"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn nui_clipboard_keeps_bind_defaults_and_list_cell_metadata() {
        let ctx = egui::Context::default();
        let mut source = mg_nui::window();
        source["root"] = mg_nui::template("list");
        source["root"]["row_template"] = json!([[{"type":"label","value":{"bind":"caption"},"custom":{"unchanged":true}}, 230.5, false]]);
        let mut settings = Settings::default();
        settings.bindings.insert(
            "caption".into(),
            Binding { value: "Source".into(), watch: true, ..Default::default() },
        );
        let mut selected = "/root/row_template/0/0".into();
        shortcuts::clipboard(&ctx, &mut source, &mut settings, &mut selected, 0);
        assert!(ctx.output(|o| o.commands.iter().any(|c| matches!(
            c,
            egui::OutputCommand::CopyText(text) if text == "Moonglow Toolset: NUI element"
        ))));
        let mut target = source.clone();
        let mut target_settings = Settings::default();
        target_settings
            .bindings
            .insert("caption".into(), Binding { value: "Target".into(), ..Default::default() });
        selected = "/root".into();
        shortcuts::clipboard(&ctx, &mut target, &mut target_settings, &mut selected, 2);
        let cell = &target["root"]["row_template"][1];
        assert_eq!(cell[1], 230.5);
        assert_eq!(cell[2], false);
        assert_eq!(cell[0]["custom"], json!({"unchanged":true}));
        assert_eq!(cell[0]["value"]["bind"], "caption_2");
        assert_eq!(target_settings.bindings["caption"].value, "Target");
        assert_eq!(target_settings.bindings["caption_2"], settings.bindings["caption"]);
        assert!(!shortcuts::can_edit(&target, "/root", 3));
        target["root"] = mg_nui::template("group");
        assert!(!shortcuts::can_edit(&target, "/root/children/0", 3));
    }

    #[test]
    fn nui_palette_drag_adds_at_layers_position_and_undoes() {
        for live in [false, true] {
            for (target, offset, index) in
                [("Column", 0.0, 2), ("Button · Close", -7.0, 1), ("Button · Close", 7.0, 2)]
            {
                let mut h = keyboard_harness();
                h.run();
                if live {
                    h.get_by_label("Interact").click();
                    h.run();
                }
                let before = keyboard_document(&h);
                let from = h.get_by_label("Row").rect().center();
                let onto = h.get_all_by_label(target).last().unwrap().rect().center()
                    + egui::vec2(0.0, offset);
                h.hover_at(from);
                h.run();
                h.drag_at(from);
                h.run();
                h.hover_at(from + egui::vec2(14.0, 0.0));
                h.run();
                h.hover_at(onto);
                h.run();
                assert_eq!(keyboard_document(&h), before, "hover must not modify the module");
                h.drop_at(onto);
                h.run();
                let doc = keyboard_document(&h);
                assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 3);
                assert_eq!(doc["root"]["children"][index]["type"], "row");
                h.state_mut().ws.as_mut().unwrap().undo().unwrap();
                h.run();
                assert_eq!(keyboard_document(&h), before);
            }
        }
    }

    #[test]
    fn nui_palette_drag_adds_to_canvas_container() {
        for live in [false, true] {
            let key = mg_nui::key("nui_test", ResType::JUI);
            let mut h = Harness::builder().with_size(egui::vec2(1200.0, 850.0)).build_ui_state(
                |ui, app: &mut Moonglow| {
                    super::ui(app, ui, Some(key));
                    app.run_actions();
                },
                app(),
            );
            h.run();
            if live {
                h.get_by_label("Interact").click();
                h.run();
            }
            let from = h.get_by_label("Row").rect().center();
            let onto =
                h.get_by_label("Canvas Column").rect().right_bottom() - egui::vec2(12.0, 12.0);
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
            let doc =
                mg_nui::parse(h.state().ws.as_ref().unwrap().module.get(&key).unwrap()).unwrap();
            assert_eq!(doc["root"]["children"].as_array().unwrap().len(), 3);
            assert_eq!(doc["root"]["children"][2]["type"], "row");
        }
    }

    #[test]
    fn nui_json_edit_is_saved_to_workspace_and_undoable() {
        let key = mg_nui::key("nui_test", ResType::JUI);
        let mut h = Harness::builder().with_size(egui::vec2(1200.0, 850.0)).build_ui_state(
            |ui, app: &mut Moonglow| {
                super::ui(app, ui, Some(key));
                app.run_actions();
            },
            app(),
        );
        h.run();
        let before = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
        h.get_by_label("Advanced").click();
        h.run();
        h.get_by_label("JUI source").click();
        h.run();
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).next().unwrap().click();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .next()
            .unwrap()
            .type_text("invalid draft");
        h.run();
        let ws = h.state_mut().ws.as_mut().unwrap();
        let draft = ws.module.get(&key).unwrap().to_vec();
        assert_ne!(draft, before);
        assert!(String::from_utf8_lossy(&draft).contains("invalid draft"));
        ws.undo().unwrap();
        assert_eq!(ws.module.get(&key).unwrap(), before);
        ws.redo().unwrap();
        assert_eq!(ws.module.get(&key).unwrap(), draft);
        h.get_by_label("Design").click();
        h.run();
        h.get_by_label(
            "Use the source/settings tabs to repair JSON. Original contents are retained.",
        );
    }

    #[test]
    fn nui_can_open_long_resref_and_invalid_metadata_without_panicking() {
        let mut app = app();
        let key = mg_nui::key("sixteen_charname", ResType::JUI);
        app.ws.as_mut().unwrap().module.set(key, serde_json::to_vec(&mg_nui::window()).unwrap());
        app.ws
            .as_mut()
            .unwrap()
            .module
            .set(ResKey::new(key.resref, ResType::TXT), b"unrelated text".to_vec());
        let mut h = Harness::builder()
            .with_size(egui::vec2(1200.0, 850.0))
            .build_ui_state(|ui, app: &mut Moonglow| super::ui(app, ui, Some(key)), app);
        h.run();
        h.get_by_label("Bindings").click();
        h.run();
    }

    #[test]
    fn nui_export_compiles_the_whole_bundle_without_changing_module_events() {
        let root = mg_testkit::corpus!();
        let mut app = Moonglow::new(
            Some(mg_resman::GameInstall::new(root, None, "en")),
            Box::new(crate::NoDialogs::default()),
        );
        let mut m = mg_module::Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        info.root.set("Mod_OnNuiEvent", mg_gff::Value::ResRef(b"my_event".to_vec()));
        m.set_info(&info).unwrap();
        let ifo = mg_nui::key("module", ResType::IFO);
        let before = m.get(&ifo).unwrap().to_vec();
        app.ws = Some(mg_edit::Workspace::new(m));
        app.run(Action::NewNui("export_ui".into()));
        app.actions.clear();
        app.run(Action::GenerateNui("export_ui".into(), true));
        app.run_actions();
        let exported = &app.export.as_ref().expect("export opened after compilation").selected;
        assert_eq!(exported.len(), 5);
        for key in exported {
            assert!(app.ws.as_ref().unwrap().module.contains(key));
        }
        assert_eq!(app.ws.as_ref().unwrap().module.get(&ifo).unwrap(), before);
        let revision = app.ws.as_ref().unwrap().revision();
        app.run(Action::GenerateNui("export_ui".into(), false));
        assert_eq!(
            app.ws.as_ref().unwrap().revision(),
            revision,
            "unchanged generation must not dirty the module or add an undo step"
        );
        app.ws.as_mut().unwrap().undo().unwrap();
        assert!(
            !app.ws.as_ref().unwrap().module.contains(&mg_nui::key("export_ui_o", ResType::NCS))
        );
    }

    /// A local preview artifact for visual review; not Aurora/NWN proof.
    #[test]
    #[ignore]
    fn nui_editor_screenshot() {
        mg_testkit::gpu::hold();
        let key = mg_nui::key("nui_test", ResType::JUI);
        let root = mg_testkit::corpus!();
        let mut sample = Moonglow::new(
            Some(mg_resman::GameInstall::new(root, None, "en")),
            Box::new(crate::NoDialogs::default()),
        );
        sample.ws = Some(mg_edit::Workspace::new(mg_module::Module::new()));
        create(&mut sample, "nui_test");
        sample.actions.clear();
        let mut window = mg_nui::window();
        window["title"] = "Character settings".into();
        window["geometry"]["w"] = 460.0.into();
        window["geometry"]["h"] = 330.0.into();
        window["root"]["children"][0]["value"] = "Customize your interface".into();
        let mut input = mg_nui::template("textedit");
        input["value"] = json!({"bind":"player_name"});
        let mut check = mg_nui::template("check");
        check["label"] = "Show tooltips".into();
        check["value"] = json!({"bind":"show_tooltips"});
        let mut button = mg_nui::template("button");
        button["label"] = "Save settings".into();
        button["id"] = "save_settings".into();
        let mut combo = mg_nui::template("combo");
        combo["elements"] = json!([["Polski", 0], ["English", 1]]);
        let mut picture = mg_nui::template("image");
        picture["value"] = "nui_button_a".into();
        picture["height"] = 42.0.into();
        window["root"]["children"]
            .as_array_mut()
            .unwrap()
            .splice(1..2, [input, check, combo, mg_nui::template("progress"), picture, button]);
        sample.ws.as_mut().unwrap().module.set(key, serde_json::to_vec_pretty(&window).unwrap());
        let mut settings = Settings::default();
        settings.bindings.insert(
            "player_name".into(),
            Binding { value: "Łukasz — Żółw".into(), ..Default::default() },
        );
        settings
            .bindings
            .insert("show_tooltips".into(), Binding { value: true.into(), ..Default::default() });
        sample
            .ws
            .as_mut()
            .unwrap()
            .module
            .set(mg_nui::key("nui_test", ResType::TXT), settings.bytes());
        let mut h = Harness::builder()
            .with_size(egui::vec2(1200.0, 850.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| super::ui(app, ui, Some(key)), sample);
        h.run_steps(3);
        h.get_by_label("Canvas Button · Save settings").click();
        h.run();
        assert!(h.state().nui_assets.loaded);
        assert!(h.state().nui_assets.issues.is_empty(), "{:?}", h.state().nui_assets.issues);
        assert!(h.state().nui_assets.picture("nui_button_h").is_some());
        assert!(h.state().nui_assets.origins.contains_key("fnt_maintext.ttf"));
        let dir = mg_testkit::scratch_dir("nui-editor-preview");
        h.render().unwrap().save(dir.join("editor.png")).unwrap();
        h.get_by_label("State").click();
        h.run();
        h.get_by_label("Hover").click();
        h.run();
        h.get_by_label("Scale").click();
        h.run();
        h.get_by_label("150%").click();
        h.run();
        h.get_by_label("Clean view").click();
        h.run();
        h.render().unwrap().save(dir.join("game-hover-150.png")).unwrap();
        // The same working fixture now demonstrates list authoring and live inputs.
        let mut list = mg_nui::template("list");
        list["row_count"] = 5.into();
        list["row_height"] = 28.0.into();
        list["row_template"] = json!([
            [{"type":"label","value":{"bind":"items"},"text_halign":1},220.0,true],
            [{"type":"check","label":"Active","value":{"bind":"active"}},110.0,false]
        ]);
        window["geometry"]["h"] = 380.0.into();
        window["root"]["children"] = json!([
            {"type":"label","value":"Inventory settings","height":30.0},
            list,
            mg_nui::template("slider"),
            {"type":"button","label":"Apply","id":"apply","height":30.0}
        ]);
        settings.bindings.insert(
            "items".into(),
            Binding {
                value: json!(["Miecz", "Łuk", "Tarcza", "Zbroja", "Pierścień"]),
                ..Default::default()
            },
        );
        settings.bindings.insert(
            "active".into(),
            Binding { value: json!([true, false, false, true, false]), ..Default::default() },
        );
        h.state_mut()
            .ws
            .as_mut()
            .unwrap()
            .module
            .set(key, serde_json::to_vec_pretty(&window).unwrap());
        h.state_mut()
            .ws
            .as_mut()
            .unwrap()
            .module
            .set(mg_nui::key("nui_test", ResType::TXT), settings.bytes());
        h.get_by_label("Scale").click();
        h.run();
        h.get_all_by_label("100%").last().unwrap().click();
        h.run();
        h.get_by_label("Clean view").click();
        h.run();
        h.get_by_label("Canvas List").click();
        h.run();
        h.get_by_label("Collapse List").click();
        h.run();
        h.render().unwrap().save(dir.join("list-layout.png")).unwrap();
        let source_before = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
        let config_key = mg_nui::key("nui_test", ResType::TXT);
        let config_before =
            h.state().ws.as_ref().unwrap().module.get(&config_key).unwrap().to_vec();
        let revision = h.state().ws.as_ref().unwrap().revision();
        h.get_by_label("Interact").click();
        h.run();
        h.get_by_label("Canvas Checkbox · Active · row 1").click();
        h.run();
        h.render().unwrap().save(dir.join("interaction.png")).unwrap();
        assert_eq!(h.state().ws.as_ref().unwrap().module.get(&key).unwrap(), source_before);
        assert_eq!(h.state().ws.as_ref().unwrap().module.get(&config_key).unwrap(), config_before);
        assert_eq!(h.state().ws.as_ref().unwrap().revision(), revision);
        h.set_size(egui::vec2(1100.0, 800.0));
        h.get_by_label("Canvas window").click();
        h.run();
        h.render().unwrap().save(dir.join("interaction-narrow.png")).unwrap();
        // Extend the same fixture, without copying game content or the module.
        window["title"] = json!("Views, charts and drawing");
        window["geometry"] = json!({"x":-1.0,"y":-1.0,"w":480.0,"h":450.0});
        window["root"]["children"] = json!([
            {"type":"group","id":"view_host","height":70.0,"children":[{"type":"label","value":"Main view · native group"}]},
            {"type":"row","children":[{"type":"button","id":"next","label":"Show details","height":32.0},{"type":"button","id":"mg_close","label":"Close","height":32.0}]},
            {"type":"chart","height":100.0,"value":[{"type":0,"legend":"XP","color":{"r":80,"g":180,"b":240,"a":255},"data":[0.0,2.0,1.0,4.0]}]},
            {"type":"color_picker","height":45.0,"value":{"r":80,"g":180,"b":240,"a":255}},
            {"type":"spacer","height":100.0,"draw_list_scissor":true,"draw_list":[
                {"type":7,"rect":{"x":8.0,"y":8.0,"w":160.0,"h":75.0},"color":{"r":173,"g":142,"b":96,"a":255},"line_thickness":2.0},
                {"type":4,"rect":{"x":18.0,"y":26.0,"w":145.0,"h":40.0},"text":"Draw layer","font":""},
                {"type":0,"points":[195.0,70.0,260.0,20.0,320.0,65.0,395.0,15.0],"color":{"r":80,"g":180,"b":240,"a":255},"line_thickness":3.0}
            ]}
        ]);
        settings.views.insert("details".into(), json!({"type":"label","value":"Details view"}));
        settings.actions = vec![mg_nui::Route {
            event: "click".into(),
            element: "next".into(),
            action: mg_nui::Action::View { group: "view_host".into(), view: "details".into() },
        }];
        h.state_mut()
            .ws
            .as_mut()
            .unwrap()
            .module
            .set(key, serde_json::to_vec_pretty(&window).unwrap());
        h.state_mut().ws.as_mut().unwrap().module.set(config_key, settings.bytes());
        h.set_size(egui::vec2(1350.0, 960.0));
        h.run();
        h.get_by_label("Canvas Button · Show details").click();
        h.run();
        h.render().unwrap().save(dir.join("expanded-controls.png")).unwrap();
        h.get_by_label("Views & events").click();
        h.run();
        h.render().unwrap().save(dir.join("views-events.png")).unwrap();
        let mut landing = Harness::builder()
            .with_size(egui::vec2(1000.0, 700.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| super::ui(app, ui, None), app());
        landing.run_steps(3);
        landing.render().unwrap().save(dir.join("landing.png")).unwrap();
        let mut compact = Harness::builder()
            .with_size(egui::vec2(640.0, 720.0))
            .wgpu()
            .build_ui_state(|ui, app: &mut Moonglow| super::ui(app, ui, Some(key)), app());
        compact.run_steps(3);
        compact.get_by_label("Properties").click();
        compact.run();
        compact.render().unwrap().save(dir.join("compact.png")).unwrap();
    }

    /// The Close button the Creator inserts does nothing in game without its
    /// route; the warning carries the fix.
    #[test]
    fn nui_close_button_warning_adds_its_route() {
        let mut window = mg_nui::window();
        window["root"]["children"] =
            json!([{"type":"button","id":"mg_close","label":"Close","value":null}]);
        let mut app = app();
        let ws = app.ws.as_mut().unwrap();
        ws.module.set(
            mg_nui::key("nui_test", ResType::JUI),
            serde_json::to_vec_pretty(&window).unwrap(),
        );
        ws.module.set(mg_nui::key("nui_test", ResType::TXT), Settings::default().bytes());
        let mut h = Harness::builder().with_size(egui::vec2(1200.0, 850.0)).build_ui_state(
            |ui, app: &mut Moonglow| {
                super::ui(app, ui, Some(mg_nui::key("nui_test", ResType::JUI)));
                app.run_actions();
            },
            app,
        );
        h.run();
        h.get_by_label_contains("warnings").click();
        h.run();
        h.get_by_label("Add Clicked → Close window").click();
        h.run();
        let ws = h.state().ws.as_ref().unwrap();
        let s = Settings::parse(ws.module.get(&mg_nui::key("nui_test", ResType::TXT)).unwrap())
            .unwrap();
        assert!(s.actions.iter().any(|r| r.element == "mg_close"
            && r.event == "click"
            && r.action == mg_nui::Action::Close));
        assert!(h.query_by_label("Add Clicked → Close window").is_none());
    }

    /// Hand-written or foreign JUI reaches every page and Interact: wrong
    /// types, impossible sizes and missing binds are diagnostics, never a panic.
    #[test]
    fn nui_editor_survives_malformed_documents() {
        let mut deep = json!({"type":"label","value":"bottom"});
        for _ in 0..100 {
            deep = json!({"type":"col","children":[deep]});
        }
        let roots = [
            json!({"type":"col","children":5}),
            json!({"type":"group","children":[]}),
            json!({"type":"group","children":[{"type":"label"},{"type":"label"}],"scrollbars":99}),
            json!({"type":"list","row_template":[[1],[{"type":"label"}],"x",[null,-5.0,true]],
                "row_count":-5,"row_height":0.0,"scrollbars":4}),
            json!({"type":"list","row_template":[[{"type":"check","value":{"bind":"rows"}},0.0,true]],
                "row_count":{"bind":"rows"},"row_height":-25.0}),
            json!({"type":"list","row_template":[[{"type":"label","value":"x"},1e30,false]],
                "row_count":1_000_000_000_000_i64}),
            json!({"type":"row","children":[
                {"type":"combo","elements":"abc","value":"x"},
                {"type":"combo","elements":[[1,"a"],["b"]],"value":99},
                {"type":"options","elements":[1,2],"value":-7,"direction":5},
                {"type":"tabbar","elements":{"bind":"missing"},"value":{"bind":"missing"}},
                {"type":"slider","value":5,"min":10,"max":0,"step":0},
                {"type":"sliderf","value":0.5,"min":0.0,"max":1.0,"step":-1.0},
                {"type":"progress","value":"half"},
                {"type":"textedit","max":0,"value":7,"label":{"strref":-1}},
                {"type":"color_picker","value":"red"},
                {"type":"chart","value":"x"},
                {"type":"chart","value":[{"type":9,"data":["a",null]},5]},
                {"type":"image","value":"","image_region":{"x":-5,"y":1e9,"w":-1,"h":0}},
                {"type":"button_image","label":{"strref":999999999}},
                {"type":"text","value":{"strref":-1},"scrollbars":-3},
                {"type":"label","value":{"bind":7},"text_halign":"left"},
                {"type":"canvas","children":"none"},
                {"type":7},
                {"children":[]},
                {"type":"spacer","width":"wide","margin":1e30,"padding":-5.0,"aspect":0.0},
                {"type":"spacer","width":-100.0,"height":-100.0,"aspect":-1.0}
            ]}),
            json!({"type":"col","draw_list":"x","children":[
                {"type":"spacer","draw_list_scissor":true,"draw_list":[
                    {"type":0,"points":[1.0,2.0,3.0]},
                    {"type":0,"points":{"bind":"missing"},"arrayBinds":true},
                    {"type":1},{"type":2,"rect":"x"},{"type":3,"radius":-1.0,"amin":1e30},
                    {"type":4,"text":{"strref":-1},"rect":{"x":0,"y":0,"w":-1,"h":-1}},
                    {"type":5,"image":"","rect":{"x":0,"y":0,"w":1e9,"h":1e9}},
                    {"type":6},{"type":7,"fill":"yes"},{"type":99},{"type":"x"},5,null
                ]}
            ]}),
            deep,
            json!(5),
        ];
        let windows = roots
            .into_iter()
            .map(|root| {
                let mut window = mg_nui::window();
                window["root"] = root;
                window
            })
            .chain([
                json!({"version":"one","root":{"type":"col","children":[]},"geometry":
                    {"x":"a","y":null,"w":-100.0,"h":0.0},"title":{"bind":"missing"},
                    "size_constraint":{"x":900.0,"y":900.0,"w":10.0,"h":10.0}}),
                json!({"geometry":{"bind":"geometry"},"root":{"type":"col","children":[]}}),
                json!([]),
                json!(null),
            ]);
        let mut settings = Settings::default();
        settings.bindings.insert("rows".into(), Binding { value: json!(7), ..Default::default() });
        settings
            .bindings
            .insert("geometry".into(), Binding { value: json!("x"), ..Default::default() });
        settings.views.insert("broken".into(), json!({"type":"group","children":7}));
        for window in windows {
            let mut app = app();
            let ws = app.ws.as_mut().unwrap();
            ws.module.set(
                mg_nui::key("nui_test", ResType::JUI),
                serde_json::to_vec_pretty(&window).unwrap(),
            );
            ws.module.set(mg_nui::key("nui_test", ResType::TXT), settings.bytes());
            let mut h = Harness::builder().with_size(egui::vec2(1200.0, 850.0)).build_ui_state(
                |ui, app: &mut Moonglow| {
                    super::ui(app, ui, Some(mg_nui::key("nui_test", ResType::JUI)));
                    app.run_actions();
                },
                app,
            );
            h.run();
            if let Some(interact) = h.query_by_label("Interact") {
                interact.click();
                h.run();
            }
            for page in ["Bindings", "Views & events", "Design"] {
                h.get_by_label(page).click();
                h.run();
            }
        }
    }
}
