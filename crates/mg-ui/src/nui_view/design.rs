//! The Design page's editor view: the palette of controls, Layers (the
//! window as a tree, to select, fold, drag and drop into), the canvas, and
//! beside it the properties of what is selected, with their binds. Also
//! where a new control goes ([`insert`], [`insert_at`]) and the Bindings
//! page ([`bindings()`]).
use super::*;
use egui::{Sense, vec2};

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

/// Clicking a palette item adds to the selected container, or to its nearest
/// container ancestor. Selection follows the new node (including list cells).
pub(super) fn insert(v: &mut Value, selected: &mut String, ty: &str) {
    let ty = if ty == "swap" { "group" } else { ty };
    let Some(parent_path) = insertion_parent(v, selected) else { return };
    let mut node = mg_nui::template(ty);
    assign_ids(&mut node, v);
    let Some(parent) = v.pointer_mut(&parent_path) else { return };
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
    // Properties is drawn before Layers: a field being typed in is still there,
    // to be set, in the frame a click in Layers selects another control.
    let inspector_max = (width - 170.0 - MIN_CANVAS).clamp(220.0, 360.0);
    egui::Panel::right(ui.id().with("nui-inspector"))
        .default_size(268.0)
        .size_range(220.0..=inspector_max)
        .resizable(true)
        .frame(egui::Frame::NONE.inner_margin(14).fill(ui.visuals().panel_fill))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("nui-properties")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    inspector(ui, v, s, state, keymap, assets);
                });
        });
    let palette_max = (ui.available_width() - MIN_CANVAS).clamp(170.0, 300.0);
    egui::Panel::left(ui.id().with("nui-palette"))
        .default_size(210.0)
        .size_range(170.0..=palette_max)
        .resizable(true)
        .frame(egui::Frame::NONE.inner_margin(10).fill(ui.visuals().panel_fill))
        .show(ui, |ui| {
            widgets::section_heading(ui, "Components");
            // (Ctrl+F with the pointer over the pane goes to it.)
            let field = ui.id().with("nui-palette-filter");
            widgets::SearchField::new(field, &mut state.search)
                .hint("Find a control…")
                .find_over()
                .show(ui);
            egui::ScrollArea::vertical()
                .id_salt("nui-palette-scroll")
                .max_height((ui.available_height() * 0.48).max(120.0))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    palette(ui, v, s, state);
                });
            ui.separator();
            ui.horizontal(|ui| {
                widgets::table_heading(ui, "Layers");
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
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(12).fill(ui.visuals().extreme_bg_color))
        .show(ui, |ui| {
            preview::canvas(ui, v, s, state, assets);
        });
    shortcuts::keys(ui, v, s, state, keymap);
    structure::finish(v, state);
}

fn palette(ui: &mut Ui, v: &mut Value, s: &mut Settings, state: &mut State) {
    let query = mg_core::text::Needle::new(&state.search);
    let can_add = state.resize.is_none() && insertion_parent(v, &state.selected).is_some();
    for (category, items) in PALETTE {
        let matches: Vec<_> =
            items.iter().filter(|(ty, name, _)| query.finds(&format!("{ty} {name}"))).collect();
        if matches.is_empty() {
            continue;
        }
        widgets::table_heading(ui, *category);
        for (ty, name, description) in matches {
            let response = ui
                .add_enabled(
                    can_add,
                    egui::Button::new(*name)
                        .min_size(vec2(ui.available_width(), ui.spacing().interact_size.y))
                        .sense(Sense::click_and_drag()),
                )
                .on_hover_text(format!(
                    "{description}\nClick to add to the selected layout, or drag into Layers or \
                     the preview."
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
        ("Move up", Op::MoveUp, Cmd::NuiMoveUp),
        ("Move down", Op::MoveDown, Cmd::NuiMoveDown),
        ("Duplicate element", Op::Duplicate, Cmd::NuiDuplicate),
        ("Delete element", Op::Delete, Cmd::NuiDelete),
    ] {
        let button = egui::Button::new(label).shortcut_text(keymap.label(cmd, ui.ctx()));
        if ui.add_enabled(shortcuts::can_edit(v, &state.selected, op), button).clicked() {
            if op == Op::Delete {
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
        ("Cut element", egui::Key::X, 1, shortcuts::can_edit(v, &state.selected, Op::Delete)),
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
    let field = ui.id().with("nui-layers-filter");
    widgets::SearchField::new(field, &mut state.layer_search).hint("Find a layer or ID…").show(ui);
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
    // While a variant is edited, the tree is still the window's own.
    let main = state.edit_view.as_ref().and_then(|_| state.main_doc.take());
    let tree = Tree { s, main: true, active_root: &v["root"] };
    layer(ui, main.as_ref().unwrap_or(v), "", state, 0, tree);
    if main.is_some() {
        state.main_doc = main;
    }
    state.tree_selection.clone_from(&state.selected);
}

/// What the rows of one tree in Layers share.
#[derive(Clone, Copy)]
struct Tree<'a> {
    s: &'a Settings,
    /// The window's own tree, not that of a variant being edited under its
    /// swap layout.
    main: bool,
    /// What is being edited: the window, or the variant in its place.
    active_root: &'a Value,
}

/// A control in Layers with what is under it: its row, its draw layers,
/// then its children (a swap layout: its first contents and its variants).
fn layer(ui: &mut Ui, node: &Value, path: &str, state: &mut State, depth: usize, tree: Tree<'_>) {
    if depth > 64 {
        return;
    }
    let search = mg_core::text::Needle::new(&state.layer_search);
    if !search.finds(&node.to_string()) {
        return;
    }
    layer_row(ui, node, path, state, depth, tree);
    if state.collapsed.contains(path) && search.is_empty() {
        return;
    }
    draw_rows(ui, node, path, state, depth, tree.main);
    if path.is_empty() {
        layer(ui, &node["root"], "/root", state, depth + 1, tree);
    }
    if node["type"] == "group" && layouts::is_swap(tree.s, node) {
        swap_rows(ui, node, path, state, depth, tree);
        return;
    }
    if let Some(children) = node["children"].as_array() {
        for (i, child) in children.iter().enumerate() {
            layer(ui, child, &format!("{path}/children/{i}"), state, depth + 1, tree);
        }
    }
    if let Some(cells) = node["row_template"].as_array() {
        for (i, cell) in cells.iter().enumerate() {
            layer(ui, &cell[0], &format!("{path}/row_template/{i}/0"), state, depth + 1, tree);
        }
    }
}

/// A control's own row: the arrow that folds what is under it, and its
/// name, which selects it, is dragged to move it and takes what is dropped
/// on it.
fn layer_row(
    ui: &mut Ui,
    node: &Value,
    path: &str,
    state: &mut State,
    depth: usize,
    tree: Tree<'_>,
) {
    let Tree { s, main, active_root } = tree;
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
            // (Made when a drag starts: every row copying its control each
            // frame was a sixth of the frame.)
            if !path.is_empty() && (!main || state.edit_view.is_none()) && response.drag_started() {
                egui::DragAndDrop::set_payload(
                    ui.ctx(),
                    structure::Drag { path: path.into(), node: node.clone() },
                );
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
}

/// The rows of a control's draw layers, each one to choose and then move or
/// size on the canvas.
fn draw_rows(ui: &mut Ui, node: &Value, path: &str, state: &mut State, depth: usize, main: bool) {
    if let Some(items) = node["draw_list"].as_array() {
        for (i, item) in items.iter().enumerate() {
            let kind =
                draw::KINDS.get(item["type"].as_u64().unwrap_or(99) as usize).unwrap_or(&"Unknown");
            let chosen = state.selected == path && state.draw_item == Some((path.to_owned(), i));
            ui.horizontal(|ui| {
                ui.add_space(
                    ((depth + 1) as f32 * 14.0 + 18.0).min((ui.available_width() - 90.0).max(0.0)),
                );
                let row = ui.add(
                    egui::Button::selectable(chosen, format!("◇ {kind} {}", i + 1)).truncate(),
                );
                row.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!("Draw layer {} · {kind}", i + 1),
                    )
                });
                if row.clicked() {
                    if main && state.edit_view.is_some() {
                        layouts::edit(state, None);
                    }
                    structure::select(state, path, false);
                    state.draw_item = Some((path.to_owned(), i));
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("nui-reveal-draw"), i));
                }
            });
        }
    }
}

/// What is under a swap layout: its first contents, then its variants,
/// the one being edited with its own tree.
fn swap_rows(
    ui: &mut Ui,
    node: &Value,
    path: &str,
    state: &mut State,
    depth: usize,
    tree: Tree<'_>,
) {
    let Tree { s, main, active_root } = tree;
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
                layer(ui, child, &child_path, state, depth + 2, tree);
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
                    layer(ui, active_root, "/root", state, depth + 2, Tree { main: false, ..tree });
                });
            }
        }
    });
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
    fields::of(ui.ctx(), egui::Id::new((&state.edit_view, &state.selected)));
    let swap = layouts::is_swap(s, node);
    let name = if state.selected.is_empty() {
        "Window".into()
    } else if swap {
        format!("Swap layout · {}", node["id"].as_str().unwrap_or_default())
    } else {
        node_name(node)
    };
    widgets::section_heading(ui, name);
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
        widgets::section_heading(ui, "Bindings");
        ui.label("A bind is a named value the event script can change.");
        ui.weak("Made here, it is chosen for a control's property in Design.");
        workflow::create_bind(ui, &mut s.bindings);
        let mut used = v.as_deref().map(mg_nui::bind_names).unwrap_or_default();
        for view in s.views.values() {
            used.extend(mg_nui::bind_names(view));
        }
        for name in &used {
            if !s.bindings.contains_key(name) {
                ui.horizontal(|ui| {
                    ui.label(format!("{name} needs an initial value"));
                    if ui.button("Add value").clicked() {
                        s.bindings.insert(name.clone(), Binding::default());
                    }
                });
            }
        }
        if s.bindings.is_empty() && used.is_empty() {
            ui.weak("No binds yet.");
        }
        let names: Vec<_> = s.bindings.keys().cloned().collect();
        let doc = v.as_deref().cloned().unwrap_or_else(mg_nui::window);
        for name in names {
            let in_use = used.contains(&name);
            egui::CollapsingHeader::new(&name)
                .id_salt(("bind", &name))
                .default_open(true)
                .show(ui, |ui| bind(ui, &name, in_use, &doc, s, v.as_deref_mut(), state));
        }
        egui::CollapsingHeader::new("Client delivery").show(ui, |ui| {
            ui.checkbox(&mut s.from_resref, "Load JUI from the client by resource name");
            ui.weak(
                "The players' clients must have the JUI then (from a hak or NWSync). Off, the \
                 opener script sends the layout.",
            );
            ui.checkbox(
                &mut s.event_function,
                "Events go to the module's event script (a function to call)",
            );
            ui.weak(
                "Build writes the event script as an include with a function that handles the \
                 window's events: include it in the module's OnNuiEvent script and call it there \
                 (it says whether the event was this window's). Off, the window has an event \
                 script of its own.",
            );
        });
        egui::CollapsingHeader::new("Window identity").show(ui, |ui| {
            let mut custom = s.window_id.is_some();
            if ui.checkbox(&mut custom, "Custom window ID").changed() {
                s.window_id = custom.then(|| "my_window".into());
            }
            if let Some(id) = &mut s.window_id {
                fields::text(ui, id, ui.spacing().text_edit_width);
            }
            ui.weak(
                "The ID NuiCreate gives the window and NuiFindWindow finds it by. The resource \
                 keeps its name.",
            );
        });
    });
}

/// One bind on the Bindings page: its initial value, its handlers, its name
/// and where it is used (`doc` is the window as it was when the page began).
fn bind(
    ui: &mut Ui,
    name: &str,
    in_use: bool,
    doc: &Value,
    s: &mut Settings,
    v: Option<&mut Value>,
    state: &mut State,
) {
    let Some(binding) = s.bindings.get_mut(name) else { return };
    widgets::field_label(ui, "Initial value");
    fields::value(ui, &mut binding.value, 0);
    if !in_use {
        ui.weak("Not used by any property yet: choose it on a property in Design.");
    }
    bindings::watch(ui, name, doc, s, state);
    ui.horizontal(|ui| {
        let draft_id = ui.make_persistent_id(("rename-bind", name));
        let mut to: String =
            ui.ctx().data_mut(|d| d.get_temp(draft_id)).unwrap_or_else(|| name.to_owned());
        let label = widgets::field_label(ui, "Name");
        ui.add(egui::TextEdit::singleline(&mut to).desired_width(160.0))
            .labelled_by(label.id)
            .on_hover_text("A new name for this bind");
        let taken = to != name && s.bindings.contains_key(&to);
        let plain = !to.is_empty() && mg_nui::plain_name(&to);
        let why_not = if taken {
            "Another bind has that name"
        } else {
            "Type a new name: no spaces at either end, no + at the end"
        };
        if ui
            .add_enabled(to != name && plain && !taken, egui::Button::new("Rename"))
            .on_disabled_hover_text(why_not)
            .clicked()
        {
            // Every property, variant and handler using it follows.
            let names = std::collections::BTreeMap::from([(name.to_owned(), to.clone())]);
            if let Some(doc) = v {
                shortcuts::rename_binds(doc, &names);
            }
            for view in s.views.values_mut() {
                shortcuts::rename_binds(view, &names);
            }
            if let Some(b) = s.bindings.remove(name) {
                s.bindings.insert(to.clone(), b);
            }
            for handler in s.actions.iter_mut().filter(|r| r.event == "watch" && r.element == name)
            {
                handler.element = to.clone();
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(draft_id, to));
        if ui
            .add_enabled(!in_use, egui::Button::new("Delete"))
            .on_disabled_hover_text(
                "Properties use it (listed below): give them a constant value first",
            )
            .on_hover_text(
                "Its handlers go too; their code stays in the event script, commented out",
            )
            .clicked()
        {
            s.bindings.remove(name);
            s.actions.retain(|r| !(r.event == "watch" && r.element == name));
        }
    });
    if in_use {
        ui.horizontal_wrapped(|ui| {
            ui.weak("Used by");
            let mut paths = Vec::new();
            bind_uses(doc, name, String::new(), &mut paths);
            for path in paths {
                let (target, place) = super::describe_path(doc, &path);
                if ui.link(place).on_hover_text("Select it").clicked()
                    && let Some(target) = target
                {
                    state.selected = target;
                    state.page = Page::Design;
                }
            }
            if s.views.values().any(|view| mg_nui::bind_names(view).contains(name)) {
                ui.weak("a swap layout variant");
            }
        });
    }
}

/// Where Properties is drawn: no game install or GPU is needed.
#[cfg(test)]
mod tests {
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

    /// Properties stays in its panel: what it shows (found by its labels) is
    /// between the canvas and the window's edge. Content wider than the panel
    /// makes the panel grow, under the canvas.
    fn assert_inspector_fits(h: &Harness<'_, Fixture>) {
        let canvas = h.state().state.preview_rect.unwrap();
        let edge = h.ctx.content_rect().right();
        let mut seen = 0;
        for label in [
            "Content",
            "Size & layout",
            "+ Add property",
            "Option 1 label",
            "Option 2 label",
            "+ Add option",
            "Property binding",
            "Bind formatting",
            "Events",
        ] {
            for rect in h.query_all_by_label(label).map(|n| n.rect()) {
                seen += 1;
                assert!(
                    rect.left() >= canvas.right() && rect.right() <= edge + 1.0,
                    "{label} is outside Properties: {rect:?}, canvas {canvas:?}, edge {edge}"
                );
            }
        }
        assert!(seen >= 3, "Properties shows its sections");
    }

    #[test]
    fn properties_stay_in_their_panel_as_the_window_and_panels_are_sized() {
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
    fn a_long_bound_text_and_deep_layers_fit_narrow_panels() {
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
    fn a_dropdown_s_new_option_has_a_value_the_game_can_hold() {
        for (highest, expected) in [(i32::MAX, 1), (7, 8)] {
            let mut f = fixture();
            f.doc["root"]["children"][0]["children"][0]["elements"][1][1] = json!(highest);
            let mut h =
                Harness::builder().with_size(egui::vec2(1130.0, 760.0)).build_ui_state(show, f);
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
}
