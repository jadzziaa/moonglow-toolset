//! NUI editing keys follow Moonglow's pointer-over-editor convention.
use super::*;
use crate::keys::{Cmd, Keymap};
use std::collections::BTreeMap;

#[derive(Clone)]
struct Clipboard {
    node: Value,
    cell: Option<Value>,
    bindings: BTreeMap<String, Binding>,
}

fn clipboard_id() -> egui::Id {
    egui::Id::new("moonglow-nui-element-clipboard")
}

pub(super) fn has_clipboard(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Clipboard>(clipboard_id()).is_some())
}

pub(super) fn can_edit(v: &Value, selected: &str, op: usize) -> bool {
    let Some((parent, i, _)) = array_position(selected) else { return false };
    let Some(a) = v.pointer(&parent).and_then(Value::as_array) else { return false };
    if i >= a.len() {
        return false;
    }
    // A group holds exactly one child: deleting it leaves an empty column
    // (unless it is one already); it can't move or be duplicated.
    if parent
        .strip_suffix("/children")
        .and_then(|p| v.pointer(p))
        .is_some_and(|n| n["type"] == "group")
    {
        return op == 3 && a[i] != mg_nui::template("col");
    }
    match op {
        0 => i > 0,
        1 => i + 1 < a.len(),
        _ => true,
    }
}

pub(super) fn clipboard(
    ctx: &egui::Context,
    v: &mut Value,
    s: &mut Settings,
    selected: &mut String,
    op: usize,
) {
    if op < 2 {
        if selected.is_empty() || (op == 1 && !can_edit(v, selected, 3)) {
            return;
        }
        let Some(node) = v.pointer(selected) else { return };
        let cell = array_position(selected)
            .filter(|(_, _, cell)| *cell)
            .and_then(|(p, i, _)| v.pointer(&p)?.get(i).cloned())
            .filter(|cell| cell.as_array().is_some_and(|a| !a.is_empty()));
        let bindings = mg_nui::bind_names(node)
            .into_iter()
            .filter_map(|name| s.bindings.get(&name).map(|b| (name, b.clone())))
            .collect();
        ctx.data_mut(|d| {
            d.insert_temp(clipboard_id(), Clipboard { node: node.clone(), cell, bindings })
        });
        crate::widgets::mark_clipboard(ctx, "NUI element");
        if op == 1 {
            rearrange(v, selected, 3);
        }
        return;
    }
    let Some(parent) = design::insertion_parent(v, selected) else { return };
    let Some(mut copy) = ctx.data(|d| d.get_temp::<Clipboard>(clipboard_id())) else { return };
    // Keep existing target binds intact when pasting between different windows.
    let mut names = BTreeMap::new();
    for (name, binding) in copy.bindings {
        let mut target = name.clone();
        if s.bindings.get(&name).is_some_and(|existing| existing != &binding) {
            let mut suffix = 2;
            while s.bindings.contains_key(&format!("{name}_{suffix}")) {
                suffix += 1;
            }
            target = format!("{name}_{suffix}");
        }
        names.insert(name, target.clone());
        s.bindings.entry(target).or_insert(binding);
    }
    rename_binds(&mut copy.node, &names);
    assign_ids(&mut copy.node, v);
    let container = v.pointer_mut(&parent).unwrap();
    if container["type"] == "list" {
        if let Some(cells) = container["row_template"].as_array_mut() {
            let mut cell = copy.cell.unwrap_or_else(|| json!([null, 150.0, true]));
            cell[0] = copy.node;
            *selected = format!("{parent}/row_template/{}/0", cells.len());
            cells.push(cell);
        }
    } else if let Some(children) = container["children"].as_array_mut() {
        *selected = format!("{parent}/children/{}", children.len());
        children.push(copy.node);
    }
}

pub(super) fn rename_binds(v: &mut Value, names: &BTreeMap<String, String>) {
    match v {
        Value::Object(o) => {
            if let Some(name) =
                o.get("bind").and_then(Value::as_str).and_then(|name| names.get(name))
            {
                o.insert("bind".into(), name.clone().into());
            }
            for child in o.values_mut() {
                rename_binds(child, names);
            }
        }
        Value::Array(a) => {
            for child in a {
                rename_binds(child, names);
            }
        }
        _ => {}
    }
}

fn navigate(v: &Value, state: &mut State, forward: bool) {
    let paths = design::tree_paths(v, &state.collapsed);
    let at = paths.iter().position(|p| p == &state.selected).unwrap_or(0);
    let next = if forward { (at + 1).min(paths.len() - 1) } else { at.saturating_sub(1) };
    state.selected = paths[next].clone();
}
pub(super) fn keys(
    ui: &mut Ui,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
    keymap: &Keymap,
) {
    if (state.preview_interactive
        && state.preview_rect.is_some_and(|rect| ui.rect_contains_pointer(rect)))
        || state.resize.is_some()
        || !ui.is_enabled()
        || !ui.rect_contains_pointer(ui.max_rect())
        || ui.memory(|m| m.focused().is_some())
        || egui::Popup::is_any_open(ui.ctx())
        || ui.ctx().egui_is_using_pointer()
    {
        return;
    }
    for (cmd, op) in
        [(Cmd::NuiDelete, 3), (Cmd::NuiDuplicate, 2), (Cmd::NuiMoveUp, 0), (Cmd::NuiMoveDown, 1)]
    {
        if ui.input_mut(|i| keymap.consume(i, cmd)) && can_edit(v, &state.selected, op) {
            if op == 3 {
                structure::delete_selected(v, state);
            } else {
                rearrange(v, &mut state.selected, op);
            }
        }
    }
    for (cmd, forward) in [(Cmd::NuiPrevious, false), (Cmd::NuiNext, true)] {
        if ui.input_mut(|i| keymap.consume(i, cmd)) {
            navigate(v, state, forward);
        }
    }
    let branch = v.pointer(&state.selected).is_some_and(|n| {
        state.selected.is_empty() || n["children"].is_array() || n["row_template"].is_array()
    });
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)) {
        if branch && !state.collapsed.contains(&state.selected) {
            state.collapsed.insert(state.selected.clone());
        } else if let Some((parent, _, _)) = array_position(&state.selected) {
            state.selected = parent.rsplit_once('/').map_or("", |(p, _)| p).into();
        } else {
            state.selected.clear();
        }
    }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight))
        && branch
        && !state.collapsed.remove(&state.selected)
    {
        navigate(v, state, true);
    }
    let mut clip = [false; 3];
    ui.input_mut(|i| {
        i.events.retain(|event| {
            let op = match event {
                egui::Event::Copy => Some(0),
                egui::Event::Cut => Some(1),
                egui::Event::Paste(_) => Some(2),
                egui::Event::Key { key, modifiers, pressed: true, .. }
                    if modifiers.command && !modifiers.shift && !modifiers.alt =>
                {
                    match key {
                        egui::Key::C => Some(0),
                        egui::Key::X => Some(1),
                        egui::Key::V => Some(2),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(op) = op {
                clip[op] = true;
                false
            } else {
                true
            }
        })
    });
    for (op, pressed) in clip.into_iter().enumerate() {
        if pressed {
            clipboard(ui.ctx(), v, s, &mut state.selected, op);
        }
    }
}
