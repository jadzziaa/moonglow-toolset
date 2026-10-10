//! Authoring interactions and an explicitly approximate layout sketch.
//! This does not replace the game's NUI layout solver or execute events.
use super::*;
use egui::{RichText, Sense, vec2};

#[derive(Clone)]
pub(super) struct Insert(pub(super) &'static str);

type Component = (&'static str, &'static str, &'static str);
type ComponentGroup = (&'static str, &'static [Component]);

const PALETTE: &[ComponentGroup] = &[
    (
        "Layout",
        &[
            ("col", "Column", "Stack vertically"),
            ("row", "Row", "Arrange side by side"),
            ("group", "Group", "A bordered container"),
            ("swap", "Swap layout", "A Group slot with alternate layouts"),
            ("spacer", "Spacer", "Leave space between controls"),
        ],
    ),
    (
        "Text & actions",
        &[
            ("label", "Label", "A short line of text"),
            ("text", "Text block", "Longer, scrollable text"),
            ("button", "Button", "A clickable action"),
            ("button_select", "Toggle button", "A selectable action"),
        ],
    ),
    (
        "Inputs",
        &[
            ("textedit", "Text input", "Let players type"),
            ("check", "Checkbox", "An on/off option"),
            ("combo", "Dropdown", "Choose from a list"),
            ("slider", "Integer slider", "Choose a whole number"),
            ("sliderf", "Decimal slider", "Choose a decimal value"),
            ("color_picker", "Color picker", "Choose a color"),
        ],
    ),
    (
        "Display & navigation",
        &[
            ("image", "Image", "Display a game resource"),
            ("button_image", "Image button", "An action with an image"),
            ("progress", "Progress bar", "Show progress"),
            ("list", "List", "Repeat a row template"),
            ("options", "Options", "A set of choices"),
            ("tabbar", "Tabs", "Switch between choices"),
            ("chart", "Chart", "Plot a data series"),
        ],
    ),
];

fn title(ty: &str) -> &str {
    PALETTE
        .iter()
        .flat_map(|(_, items)| *items)
        .find(|(key, _, _)| *key == ty)
        .map_or(ty, |(_, name, _)| name)
}

/// Where a document binds a property to `name`: the bound properties' paths.
fn bind_uses(v: &Value, name: &str, at: String, out: &mut Vec<String>) {
    match v {
        Value::Object(o) if o.get("bind").and_then(Value::as_str) == Some(name) => out.push(at),
        Value::Object(o) => {
            for (k, v) in o {
                bind_uses(v, name, format!("{at}/{k}"), out);
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                bind_uses(v, name, format!("{at}/{i}"), out);
            }
        }
        _ => {}
    }
}

pub(super) fn node_name(node: &Value) -> String {
    let kind = title(node["type"].as_str().unwrap_or("Window"));
    let text = node["label"].as_str().or_else(|| node["value"].as_str()).filter(|s| !s.is_empty());
    match text {
        Some(t) => format!("{kind} · {}", t.chars().take(28).collect::<String>()),
        None => kind.into(),
    }
}

pub(super) fn document_actions(
    ui: &mut Ui,
    state: &mut State,
    module: &mg_module::Module,
    actions: &mut Vec<Action>,
) {
    use egui::containers::menu::{MenuButton, MenuConfig};
    let config = MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    MenuButton::new("New NUI…").config(config.clone()).ui(ui, |ui| {
        ui.set_width(280.0);
        ui.strong("New NUI");
        ui.label("Resource name");
        let input = ui.add(
            egui::TextEdit::singleline(&mut state.name)
                .hint_text("e.g. character_menu")
                .desired_width(f32::INFINITY),
        );
        // Use the same name/collision rules as the actual creation command,
        // including companion scripts; never silently replace a resource.
        let validation = mg_nui::create(module, &state.name);
        if !state.name.is_empty()
            && let Err(error) = &validation
        {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let valid = validation.is_ok();
        ui.horizontal(|ui| {
            if ui.add_enabled(valid, egui::Button::new("Create NUI")).clicked()
                || (valid && input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                actions.push(Action::NewNui(std::mem::take(&mut state.name)));
                ui.close();
            }
            if ui.button("Cancel").clicked() {
                ui.close();
            }
        });
    });
    MenuButton::new("Load NUI…").config(config).ui(ui, |ui| {
        ui.set_width(280.0);
        ui.strong("NUI in this module");
        ui.add(
            egui::TextEdit::singleline(&mut state.load_search)
                .hint_text("Find a NUI…")
                .desired_width(f32::INFINITY),
        );
        let query = state.load_search.to_lowercase();
        let existing: Vec<_> = module
            .keys()
            .filter(|key| key.restype == ResType::JUI && key.resref.to_string().contains(&query))
            .copied()
            .collect();
        if existing.is_empty() {
            ui.weak(if query.is_empty() {
                "No NUI in this module yet."
            } else {
                "No matching NUI."
            });
        }
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for key in existing {
                if ui.button(key.resref.to_string()).clicked() {
                    actions.push(Action::OpenTab(Tab::Nui(Some(key))));
                    ui.close();
                }
            }
        });
    });
}

pub(super) fn landing(
    ui: &mut Ui,
    state: &mut State,
    module: &mg_module::Module,
    existing: &[ResKey],
    actions: &mut Vec<Action>,
) {
    ui.set_min_height(ui.available_height());
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(32.0);
        ui.vertical_centered(|ui| {
            ui.heading(RichText::new("NUI Creator").size(28.0));
            ui.add_space(6.0);
            ui.label("Build a window for your players.");
            ui.weak("Add controls, arrange the layout, then compile it for your module.");
            ui.add_space(24.0);
            let width = (ui.available_width() - 32.0).clamp(220.0, 440.0);
            egui::Frame::group(ui.style()).inner_margin(22).corner_radius(8).show(ui, |ui| {
                ui.set_width(width);
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.strong("New NUI");
                    ui.add_space(12.0);
                    ui.label("Resource name");
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut state.name)
                            .hint_text("e.g. character_menu")
                            .desired_width(f32::INFINITY),
                    );
                    ui.weak("Use 1–14 lowercase letters, numbers or underscores.");
                    // The same checks as New NUI…: a name in use is said here.
                    let validation = mg_nui::create(module, &state.name);
                    if !state.name.is_empty()
                        && let Err(error) = &validation
                    {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    ui.add_space(14.0);
                    let valid = validation.is_ok();
                    let create = ui
                        .add_enabled_ui(valid, |ui| {
                            ui.add_sized(
                                vec2(ui.available_width(), 36.0),
                                egui::Button::new("Create NUI")
                                    .fill(ui.visuals().selection.bg_fill),
                            )
                        })
                        .inner;
                    if create.clicked()
                        || (valid
                            && input.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        actions.push(Action::NewNui(state.name.clone()));
                    }
                });
            });
            if !existing.is_empty() {
                ui.add_space(28.0);
                ui.strong("Load NUI from this module");
                ui.add_space(8.0);
                for key in existing {
                    if ui
                        .add_sized(
                            vec2(width, 32.0),
                            egui::Button::new(format!("Open  {}", key.resref)),
                        )
                        .clicked()
                    {
                        actions.push(Action::OpenTab(Tab::Nui(Some(*key))));
                    }
                }
            }
        });
    });
}

/// Clicking a palette item adds to the selected container, or to its nearest
/// container ancestor. Selection follows the new node (including list cells).
pub(super) fn insert(v: &mut Value, selected: &mut String, ty: &str) {
    let ty = if ty == "swap" { "group" } else { ty };
    let Some(parent_path) = insertion_parent(v, selected) else { return };
    let mut node = mg_nui::template(ty);
    assign_ids(&mut node, v);
    let parent = v.pointer_mut(&parent_path).unwrap();
    fit_into(parent, &mut node);
    if parent["type"] == "list" {
        if let Some(cells) = parent["row_template"].as_array_mut() {
            *selected = format!("{parent_path}/row_template/{}/0", cells.len());
            cells.push(json!([node, 150.0, true]));
        }
    } else if let Some(children) = parent["children"].as_array_mut() {
        *selected = format!("{parent_path}/children/{}", children.len());
        children.push(node);
    }
}

/// Shrink a control put in a row of a fixed height (or a column of a fixed
/// width) to what fits there with its margins: the client refuses the whole
/// window if it doesn't (mg_nui::validate). Too little room: no fixed size.
pub(super) fn fit_into(parent: &Value, node: &mut Value) {
    let key = match parent["type"].as_str() {
        Some("row") => "height",
        Some("col") => "width",
        _ => return,
    };
    let (Some(room), Some(size)) = (parent[key].as_f64(), node[key].as_f64()) else { return };
    let margin = node["margin"].as_f64().unwrap_or(2.0);
    if size + 2.0 * margin > room
        && let Some(object) = node.as_object_mut()
    {
        let fits = room - 2.0 * margin;
        if fits >= 8.0 {
            object.insert(key.into(), json!(fits));
        } else {
            object.remove(key);
        }
    }
}

/// Insert a new palette element without ever replacing a Group's single child.
pub(super) fn insert_at(
    doc: &mut Value,
    target: &str,
    position: structure::Position,
    ty: &str,
) -> Option<String> {
    let (dest, index, cell) = structure::destination(doc, target, position)?;
    let mut node = mg_nui::template(if ty == "swap" { "group" } else { ty });
    assign_ids(&mut node, doc);
    if let Some(parent) = dest.strip_suffix("/children").and_then(|p| doc.pointer(p)) {
        fit_into(parent, &mut node);
    }
    let array = doc.pointer_mut(&dest)?.as_array_mut()?;
    if index > array.len() {
        return None;
    }
    array.insert(index, if cell { json!([node, 150.0, true]) } else { node });
    Some(format!("{dest}/{index}{}", if cell { "/0" } else { "" }))
}

pub(super) fn insertion_parent(v: &Value, selected: &str) -> Option<String> {
    let mut path = if selected.is_empty() { "/root" } else { selected }.to_owned();
    loop {
        let node = v.pointer(&path)?;
        if (matches!(node["type"].as_str(), Some("col" | "row")) && node["children"].is_array())
            || (node["type"] == "list" && node["row_template"].is_array())
        {
            return Some(path);
        }
        if node["type"] == "group" && node["children"][0]["children"].is_array() {
            return Some(format!("{path}/children/0"));
        }
        let (parent, _, _) = array_position(&path)?;
        path = parent.rsplit_once('/')?.0.to_owned();
    }
}

pub(super) fn editor(
    ui: &mut Ui,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
    keymap: &crate::keys::Keymap,
    assets: &mut skin::Assets,
) {
    if v.pointer(&state.selected).is_none() {
        state.selected.clear();
    }
    if !state.selected_many.contains(&state.selected) {
        state.selected_many.clear();
    }
    let width = ui.available_width();
    let compact = width < 760.0;
    if compact {
        ui.horizontal(|ui| {
            ui.menu_button("Add element", |ui| {
                palette(ui, v, s, state);
            });
            ui.menu_button("Layers", |ui| {
                layers(ui, v, s, state);
            });
            ui.menu_button("Properties", |ui| {
                ui.set_width(260.0);
                let height = ui.ctx().content_rect().height() * 0.8;
                egui::ScrollArea::vertical().max_height(height).show(ui, |ui| {
                    inspector(ui, v, s, state, keymap, assets);
                });
            });
        });
        preview::canvas(ui, v, s, state, assets);
        structure::finish(v, state);
        shortcuts::keys(ui, v, s, state, keymap);
        return;
    }
    // Remembered panel widths must leave a usable canvas after the host window
    // narrows. Otherwise even its toolbar can push into the inspector.
    const MIN_CANVAS: f32 = 280.0;
    let palette_max = (width - 220.0 - MIN_CANVAS).clamp(170.0, 300.0);
    egui::Panel::left(ui.id().with("nui-palette"))
        .default_size(210.0)
        .size_range(170.0..=palette_max)
        .resizable(true)
        .frame(egui::Frame::NONE.inner_margin(10).fill(ui.visuals().panel_fill))
        .show(ui, |ui| {
            ui.strong("Components");
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::singleline(&mut state.search)
                    .hint_text("Find a control…")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .id_salt("nui-palette-scroll")
                .max_height((ui.available_height() * 0.48).max(120.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    palette(ui, v, s, state);
                });
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.strong("Layers");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("…", |ui| {
                        edit_actions(ui, v, s, state, keymap);
                    });
                });
            });
            egui::ScrollArea::vertical().id_salt("nui-layers").auto_shrink([false, false]).show(
                ui,
                |ui| {
                    layers(ui, v, s, state);
                },
            );
        });
    let inspector_max = (ui.available_width() - MIN_CANVAS).clamp(220.0, 360.0);
    egui::Panel::right(ui.id().with("nui-inspector"))
        .default_size(268.0)
        .size_range(220.0..=inspector_max)
        .resizable(true)
        .frame(egui::Frame::NONE.inner_margin(14).fill(ui.visuals().panel_fill))
        .show(ui, |ui| {
            #[cfg(test)]
            let allocated_rect = ui.max_rect();
            let scroll = egui::ScrollArea::vertical()
                .id_salt("nui-properties")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    inspector(ui, v, s, state, keymap, assets);
                });
            #[cfg(test)]
            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    egui::Id::new("nui-inspector-layout-test"),
                    (allocated_rect, scroll.content_size),
                );
            });
            let _ = scroll;
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(12).fill(ui.visuals().extreme_bg_color))
        .show(ui, |ui| {
            preview::canvas(ui, v, s, state, assets);
        });
    shortcuts::keys(ui, v, s, state, keymap);
    structure::finish(v, state);
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;

fn palette(ui: &mut Ui, v: &mut Value, s: &mut Settings, state: &mut State) {
    let query = state.search.to_lowercase();
    let can_add = state.resize.is_none() && insertion_parent(v, &state.selected).is_some();
    for (category, items) in PALETTE {
        let matches: Vec<_> = items
            .iter()
            .filter(|(ty, name, _)| format!("{ty} {name}").to_lowercase().contains(&query))
            .collect();
        if matches.is_empty() {
            continue;
        }
        ui.add_space(8.0);
        ui.label(RichText::new(*category).small().color(ui.visuals().weak_text_color()));
        for (ty, name, description) in matches {
            let response = ui
                .add_enabled(
                    can_add,
                    egui::Button::new(*name)
                        .min_size(vec2(ui.available_width(), 28.0))
                        .sense(Sense::click_and_drag()),
                )
                .on_hover_text(format!(
                    "{description}\nClick to add to the selected layout, or drag into Layers or the preview."
                ));
            response.dnd_set_drag_payload(Insert(ty));
            if response.clicked() {
                if *ty == "swap" {
                    layouts::insert_slot(v, s, state);
                } else {
                    insert(v, &mut state.selected, ty);
                }
            }
        }
    }
    if !can_add {
        ui.weak("Select a row, column or list to add controls.");
    }
}

fn edit_actions(
    ui: &mut Ui,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
    keymap: &crate::keys::Keymap,
) {
    use crate::keys::Cmd;
    if state.resize.is_some() {
        ui.weak("Finish resizing to change elements.");
        return;
    }
    for (label, op, cmd) in [
        ("Move up", 0, Cmd::NuiMoveUp),
        ("Move down", 1, Cmd::NuiMoveDown),
        ("Duplicate element", 2, Cmd::NuiDuplicate),
        ("Delete element", 3, Cmd::NuiDelete),
    ] {
        let button = egui::Button::new(label).shortcut_text(keymap.label(cmd, ui.ctx()));
        if ui.add_enabled(shortcuts::can_edit(v, &state.selected, op), button).clicked() {
            if op == 3 {
                structure::delete_selected(v, state);
            } else {
                rearrange(v, &mut state.selected, op);
            }
            ui.close();
        }
    }
    for (label, ty) in
        [("Wrap in column", "col"), ("Wrap in row", "row"), ("Wrap in group", "group")]
    {
        if ui.add_enabled(!state.selected.is_empty(), egui::Button::new(label)).clicked() {
            structure::wrap(v, &state.selected, ty);
            ui.close();
        }
    }
    ui.separator();
    for (label, key, op, enabled) in [
        ("Copy element", egui::Key::C, 0, !state.selected.is_empty()),
        ("Cut element", egui::Key::X, 1, shortcuts::can_edit(v, &state.selected, 3)),
        (
            "Paste element",
            egui::Key::V,
            2,
            insertion_parent(v, &state.selected).is_some() && shortcuts::has_clipboard(ui.ctx()),
        ),
    ] {
        let shortcut =
            ui.ctx().format_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key));
        if ui.add_enabled(enabled, egui::Button::new(label).shortcut_text(shortcut)).clicked() {
            shortcuts::clipboard(ui.ctx(), v, s, &mut state.selected, op);
            ui.close();
        }
    }
}

fn layers(ui: &mut Ui, v: &mut Value, s: &Settings, state: &mut State) {
    ui.add(
        egui::TextEdit::singleline(&mut state.layer_search)
            .hint_text("Find a layer or ID…")
            .desired_width(ui.available_width()),
    );
    if state.selected_many.len() > 1 {
        ui.weak(format!("{} selected · Delete removes selection", state.selected_many.len()));
    }
    if state.tree_selection != state.selected {
        state.collapsed.retain(|p| !p.is_empty() && !state.selected.starts_with(&format!("{p}/")));
        state.tree_selection.clone_from(&state.selected);
    }
    ui.horizontal_wrapped(|ui| {
        if ui.small_button("Expand all").clicked() {
            state.collapsed.clear();
        }
        if ui.small_button("Collapse all").clicked() {
            state.collapsed.extend(tree_paths(v, &Default::default()));
            state.selected.clear();
        }
    });
    let tree = if state.edit_view.is_some() {
        state.main_doc.as_ref().unwrap_or(v).clone()
    } else {
        v.clone()
    };
    layer(ui, &tree, "", s, state, 0, true, &v["root"]);
    state.tree_selection.clone_from(&state.selected);
}

#[allow(clippy::too_many_arguments)]
fn layer(
    ui: &mut Ui,
    node: &Value,
    path: &str,
    s: &Settings,
    state: &mut State,
    depth: usize,
    main: bool,
    active_root: &Value,
) {
    if depth > 64 {
        return;
    }
    let search = state.layer_search.to_lowercase();
    if !search.is_empty() && !node.to_string().to_lowercase().contains(&search) {
        return;
    }
    ui.push_id(path, |ui| {
        ui.horizontal(|ui| {
            ui.add_space((depth as f32 * 14.0).min((ui.available_width() - 90.0).max(0.0)));
            let name = if path.is_empty() {
                "Window".into()
            } else if layouts::is_swap(s, node) {
                format!("Swap layout · {}", node["id"].as_str().unwrap_or_default())
            } else {
                node_name(node)
            };
            let branch =
                path.is_empty() || node["children"].is_array() || node["row_template"].is_array();
            if branch {
                let closed = state.collapsed.contains(path);
                let arrow = ui.add_sized(
                    egui::vec2(18.0, 18.0),
                    egui::Button::new(if closed { "⏵" } else { "⏷" }).frame(false),
                );
                arrow.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!("{} {name}", if closed { "Expand" } else { "Collapse" }),
                    )
                });
                if arrow.clicked() {
                    if closed {
                        state.collapsed.remove(path);
                    } else {
                        state.collapsed.insert(path.into());
                        if path.is_empty() || state.selected.starts_with(&format!("{path}/")) {
                            state.selected = path.into();
                        }
                    }
                }
            } else {
                ui.add_space(18.0);
            }
            // The selected control gets the fill; the variant being edited only its colour.
            let response = ui.add(
                egui::Button::selectable(
                    (!main || state.edit_view.is_none())
                        && (state.selected == path || state.selected_many.contains(path)),
                    &name,
                )
                .truncate()
                .sense(Sense::click_and_drag()),
            );
            if !path.is_empty() && (!main || state.edit_view.is_none()) {
                response.dnd_set_drag_payload(structure::Drag {
                    path: path.into(),
                    node: node.clone(),
                });
            }
            if !main || state.edit_view.is_none() {
                structure::drag_target(ui, &response, path, state, false);
                palette_target(ui, &response, path, active_root, state);
            }
            if response.clicked() {
                if main && state.edit_view.is_some() {
                    layouts::edit(state, None);
                }
                structure::select(state, path, ui.input(|i| i.modifiers.command));
            }
            response.on_hover_text(match node["id"].as_str() {
                Some(id) => format!("{name}\nElement ID: {id}"),
                None => name,
            });
        });
    });
    if state.collapsed.contains(path) && search.is_empty() {
        return;
    }
    if path.is_empty() {
        layer(ui, &node["root"], "/root", s, state, depth + 1, main, active_root);
    }
    if node["type"] == "group" && layouts::is_swap(s, node) {
        let group = node["id"].as_str().unwrap_or_default();
        ui.push_id(("swap-variants", main, path), |ui| {
            ui.horizontal(|ui| {
                ui.add_space((depth + 1) as f32 * 14.0 + 18.0);
                if ui
                    .add(
                        egui::Button::new("Initial contents")
                            .selected(state.edit_view.is_none())
                            .frame(false),
                    )
                    .clicked()
                {
                    layouts::edit(state, None);
                    state.selected = format!("{path}/children/0");
                }
            });
            // What a swap layout holds sits under the contents it belongs to.
            if let Some(children) = node["children"].as_array() {
                for (i, child) in children.iter().enumerate() {
                    let child_path = format!("{path}/children/{i}");
                    layer(ui, child, &child_path, s, state, depth + 2, main, active_root);
                }
            }
            for name in layouts::variants(s, group) {
                let active = state.edit_view.as_deref() == Some(name.as_str());
                ui.horizontal(|ui| {
                    ui.add_space((depth + 1) as f32 * 14.0 + 18.0);
                    if ui
                        .add(
                            egui::Button::new(format!("Variant · {name}"))
                                .selected(active)
                                .frame(false),
                        )
                        .clicked()
                    {
                        state.layout_target = group.into();
                        layouts::edit(state, Some(name.clone()));
                    }
                });
                if active && layouts::target_for(s, &name).as_deref() == Some(group) {
                    ui.push_id(("variant-tree", &name), |ui| {
                        layer(ui, active_root, "/root", s, state, depth + 2, false, active_root);
                    });
                }
            }
        });
        return;
    }
    if let Some(children) = node["children"].as_array() {
        for (i, child) in children.iter().enumerate() {
            layer(
                ui,
                child,
                &format!("{path}/children/{i}"),
                s,
                state,
                depth + 1,
                main,
                active_root,
            );
        }
    }
    if let Some(cells) = node["row_template"].as_array() {
        for (i, cell) in cells.iter().enumerate() {
            layer(
                ui,
                &cell[0],
                &format!("{path}/row_template/{i}/0"),
                s,
                state,
                depth + 1,
                main,
                active_root,
            );
        }
    }
}

fn palette_target(
    ui: &mut Ui,
    response: &egui::Response,
    path: &str,
    root: &Value,
    state: &mut State,
) {
    if state.resize.is_some() {
        return;
    }
    if let Some(payload) = response.dnd_hover_payload::<Insert>() {
        let doc = json!({"root": root});
        let mut mode = structure::drop_position(ui, response, false);
        // A leaf cannot own children: its middle behaves as an insertion after it.
        if mode == structure::Position::Inside
            && !doc.pointer(path).is_some_and(|n| {
                matches!(n["type"].as_str(), Some("col" | "row" | "group" | "list"))
            })
            && !path.is_empty()
        {
            mode = structure::Position::After;
        }
        if structure::destination(&doc, path, mode).is_some() {
            structure::paint_drop(ui, response, mode);
            if response.dnd_release_payload::<Insert>().is_some() {
                state.insert_pending = Some((payload.0, path.into(), mode));
            }
        }
    }
}

pub(super) fn tree_paths(v: &Value, collapsed: &std::collections::BTreeSet<String>) -> Vec<String> {
    fn visit(
        v: &Value,
        path: String,
        collapsed: &std::collections::BTreeSet<String>,
        out: &mut Vec<String>,
        depth: usize,
    ) {
        if depth > 64 {
            return;
        }
        out.push(path.clone());
        if collapsed.contains(&path) {
            return;
        }
        if path.is_empty() {
            visit(&v["root"], "/root".into(), collapsed, out, depth + 1);
        }
        if let Some(a) = v["children"].as_array() {
            for (i, c) in a.iter().enumerate() {
                visit(c, format!("{path}/children/{i}"), collapsed, out, depth + 1);
            }
        }
        if let Some(a) = v["row_template"].as_array() {
            for (i, c) in a.iter().enumerate() {
                visit(&c[0], format!("{path}/row_template/{i}/0"), collapsed, out, depth + 1);
            }
        }
    }
    let mut out = Vec::new();
    visit(v, String::new(), collapsed, &mut out, 0);
    out
}

fn inspector(
    ui: &mut Ui,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
    keymap: &crate::keys::Keymap,
    assets: &mut skin::Assets,
) {
    let Some(node) = v.pointer(&state.selected) else { return };
    let swap = layouts::is_swap(s, node);
    ui.heading(if state.selected.is_empty() {
        "Window".into()
    } else if swap {
        format!("Swap layout · {}", node["id"].as_str().unwrap_or_default())
    } else {
        node_name(node)
    });
    ui.add_space(8.0);
    // What the selection is first: a swap layout's variants, any control's
    // own properties; then how it changes (binds, events, drawing).
    if swap {
        ui.add_enabled_ui(state.resize.is_none(), |ui| layouts::inspector(ui, v, s, state));
    }
    let Some(node) = v.pointer_mut(&state.selected) else { return };
    ui.add_enabled_ui(state.resize.is_none(), |ui| properties(ui, node, s));
    fields::list(ui, v, &state.selected, s);
    if !state.selected.is_empty()
        && let Some(node) = v.pointer_mut(&state.selected)
    {
        workflow::images(ui, node, s, state, assets);
        fields::chart(ui, node, s);
    }
    ui.add_enabled_ui(state.resize.is_none(), |ui| bindings::inspector(ui, v, s, state));
    ui.add_enabled_ui(state.resize.is_none(), |ui| events::inspector(ui, v, s, state));
    if !state.selected.is_empty()
        && let Some(node) = v.pointer_mut(&state.selected)
    {
        draw::editor(ui, node, s, assets);
    }
    if !swap {
        ui.add_enabled_ui(state.resize.is_none(), |ui| layouts::inspector(ui, v, s, state));
    }
    if !state.selected.is_empty() {
        ui.add_space(12.0);
        ui.separator();
        ui.add_enabled_ui(state.resize.is_none(), |ui| {
            ui.menu_button("Element actions", |ui| {
                edit_actions(ui, v, s, state, keymap);
            })
        });
    }
}

pub(super) fn bindings(
    ui: &mut Ui,
    s: &mut Settings,
    mut v: Option<&mut Value>,
    state: &mut State,
) {
    egui::ScrollArea::vertical().id_salt("nui-bindings").show(ui, |ui| {
        ui.add_space(10.0);
        ui.heading("Bindings");
        ui.label("Bind a property to a value your event script can change.");
        ui.weak("Create a named value here, then choose it for a control property in Design.");
        workflow::create_bind(ui, &mut s.bindings);
        ui.add_space(16.0);
        let mut used = v.as_deref().map(mg_nui::bind_names).unwrap_or_default();
        for view in s.views.values() { used.extend(mg_nui::bind_names(view)); }
        for name in &used {
            if !s.bindings.contains_key(name) {
                ui.horizontal(|ui| {
                    ui.label(format!("{name} needs an initial value"));
                    if ui.button("Add value").clicked() { s.bindings.insert(name.clone(), Binding::default()); }
                });
            }
        }
        if s.bindings.is_empty() && used.is_empty() {
            egui::Frame::group(ui.style()).inner_margin(18).show(ui, |ui| {
                ui.strong("No dynamic values yet");
                ui.label("Start with a static layout. Add binds when a control needs to change during play.");
            });
        }
        let names: Vec<_> = s.bindings.keys().cloned().collect();
        let doc = v.as_deref().cloned().unwrap_or_else(mg_nui::window);
        for name in names {
            egui::CollapsingHeader::new(&name).id_salt(("bind", &name)).default_open(true).show(ui, |ui| {
                let binding = s.bindings.get_mut(&name).unwrap();
                ui.label("Initial value");
                fields::value(ui, &mut binding.value, 0);
                if !used.contains(&name) { ui.weak("Not used by any property yet: choose it on a property in Design."); }
                bindings::watch(ui, &name, &doc, s, state);
                ui.horizontal(|ui| {
                    let draft_id = ui.make_persistent_id(("rename-bind", &name));
                    let mut to: String = ui.ctx().data_mut(|d| d.get_temp(draft_id)).unwrap_or_else(|| name.clone());
                    let label = ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut to).desired_width(160.0))
                        .labelled_by(label.id)
                        .on_hover_text("A new name for this bind");
                    let taken = to != name && s.bindings.contains_key(&to);
                    let plain = !to.is_empty() && mg_nui::plain_name(&to);
                    if ui.add_enabled(to != name && plain && !taken, egui::Button::new("Rename")).on_disabled_hover_text(if taken { "Another bind has that name" } else { "Type a new name: no spaces at either end, no + at the end" }).clicked() {
                        // Every property, variant and handler using it follows.
                        let names = std::collections::BTreeMap::from([(name.clone(), to.clone())]);
                        if let Some(doc) = v.as_deref_mut() { shortcuts::rename_binds(doc, &names); }
                        for view in s.views.values_mut() { shortcuts::rename_binds(view, &names); }
                        if let Some(b) = s.bindings.remove(&name) { s.bindings.insert(to.clone(), b); }
                        for route in s.actions.iter_mut().filter(|r| r.event == "watch" && r.element == name) {
                            route.element = to.clone();
                        }
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(draft_id, to));
                    let use_count = used.contains(&name);
                    if ui.add_enabled(!use_count, egui::Button::new("Delete")).on_disabled_hover_text("Properties use it (listed below): give them a constant value first").on_hover_text("Its handlers go too; their code stays in the event script, commented out").clicked() {
                        s.bindings.remove(&name);
                        s.actions.retain(|r| !(r.event == "watch" && r.element == name));
                    }
                });
                if used.contains(&name) {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak("Used by");
                        let mut paths = Vec::new();
                        bind_uses(&doc, &name, String::new(), &mut paths);
                        for path in paths {
                            let (target, place) = super::describe_path(&doc, &path);
                            if ui.link(place).on_hover_text("Select it").clicked() && let Some(target) = target {
                                state.selected = target;
                                state.page = 0;
                            }
                        }
                        if s.views.values().any(|view| mg_nui::bind_names(view).contains(&name)) {
                            ui.weak("a swap layout variant");
                        }
                    });
                }
            });
            ui.add_space(8.0);
        }
        ui.add_space(16.0);
        egui::CollapsingHeader::new("Client delivery").show(ui, |ui| {
            ui.checkbox(&mut s.from_resref, "Load JUI from the client by resource name");
            ui.weak("Requires distributing the JUI to clients. Leave off to send the layout from the opener script.");
        });
        egui::CollapsingHeader::new("Window identity").show(ui, |ui| {
            let mut custom = s.window_id.is_some();
            if ui.checkbox(&mut custom, "Custom window ID").changed() {
                s.window_id = custom.then(|| "my_window".into());
            }
            if let Some(id) = &mut s.window_id {
                ui.text_edit_singleline(id);
            }
            ui.weak("The resource name remains unchanged. A custom ID controls NuiFindWindow/NuiCreate identity.");
        });
    });
}
