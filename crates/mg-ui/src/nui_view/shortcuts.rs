//! The Creator's keys, taken while the pointer is over the editor as the
//! other editors take theirs ([`keys`]): moving through the tree, delete,
//! duplicate, move, and cut, copy and paste of a control with the binds it
//! names ([`clipboard`]).
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

pub(super) fn can_edit(v: &Value, selected: &str, op: Op) -> bool {
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
        return op == Op::Delete && a[i] != mg_nui::template("col");
    }
    match op {
        Op::MoveUp => i > 0,
        Op::MoveDown => i + 1 < a.len(),
        Op::Duplicate | Op::Delete => true,
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
        if selected.is_empty() || (op == 1 && !can_edit(v, selected, Op::Delete)) {
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
            rearrange(v, selected, Op::Delete);
        }
        return;
    }
    let Some(parent) = design::insertion_parent(v, selected) else { return };
    // (Before anything is added: a paste that has nowhere to go adds no binds.)
    if v.pointer(&parent).is_none() {
        return;
    }
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
    let Some(container) = v.pointer_mut(&parent) else { return };
    if container["type"] == "list" {
        if let Some(cells) = container.get_mut("row_template").and_then(Value::as_array_mut) {
            let mut cell = copy.cell.unwrap_or_else(|| json!([null, 150.0, true]));
            cell[0] = copy.node;
            *selected = format!("{parent}/row_template/{}/0", cells.len());
            cells.push(cell);
        }
    } else if let Some(children) = container.get_mut("children").and_then(Value::as_array_mut) {
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
    // (Nothing to move through: a window whose root is not there.)
    let Some(last) = paths.len().checked_sub(1) else { return };
    let at = paths.iter().position(|p| p == &state.selected).unwrap_or(0);
    let next = if forward { (at + 1).min(last) } else { at.saturating_sub(1) };
    state.selected = paths[next].clone();
}

pub(super) fn keys(
    ui: &mut Ui,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
    keymap: &Keymap,
) {
    // Keys, with the pointer over the editor and no text being typed:
    // Options › Keyboard's (left for others, as the other editors' are).
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
    for (cmd, op) in [
        (Cmd::NuiDelete, Op::Delete),
        (Cmd::NuiDuplicate, Op::Duplicate),
        (Cmd::NuiMoveUp, Op::MoveUp),
        (Cmd::NuiMoveDown, Op::MoveDown),
    ] {
        if ui.input(|i| keymap.pressed(i, cmd)) && can_edit(v, &state.selected, op) {
            if op == Op::Delete {
                structure::delete_selected(v, state);
            } else {
                rearrange(v, &mut state.selected, op);
            }
        }
    }
    for (cmd, forward) in [(Cmd::NuiPrevious, false), (Cmd::NuiNext, true)] {
        if ui.input(|i| keymap.pressed(i, cmd)) {
            navigate(v, state, forward);
        }
    }
    let branch = v.pointer(&state.selected).is_some_and(|n| {
        state.selected.is_empty() || n["children"].is_array() || n["row_template"].is_array()
    });
    if ui.input(|i| keymap.pressed(i, Cmd::NuiFold)) {
        if branch && !state.collapsed.contains(&state.selected) {
            state.collapsed.insert(state.selected.clone());
        } else if let Some((parent, _, _)) = array_position(&state.selected) {
            state.selected = parent.rsplit_once('/').map_or("", |(p, _)| p).into();
        } else {
            state.selected.clear();
        }
    }
    if ui.input(|i| keymap.pressed(i, Cmd::NuiUnfold))
        && branch
        && !state.collapsed.remove(&state.selected)
    {
        navigate(v, state, true);
    }
    // The platform's Copy, Cut and Paste, as the conversation editor takes them.
    let (copy, cut, paste) = crate::widgets::copy_cut_paste(ui);
    let clip = [copy, cut, paste];
    for (op, pressed) in clip.into_iter().enumerate() {
        if pressed {
            clipboard(ui.ctx(), v, s, &mut state.selected, op);
        }
    }
}
