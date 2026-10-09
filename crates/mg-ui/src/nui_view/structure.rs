use super::*;

#[derive(Clone)]
pub(super) struct Drag {
    pub path: String,
    pub node: Value,
}
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Position {
    Before,
    Inside,
    After,
}

pub(super) fn select(state: &mut State, path: &str, additive: bool) {
    if !additive {
        state.selected_many.clear();
    }
    if additive && state.selected_many.contains(path) {
        state.selected_many.remove(path);
        state.selected = state.selected_many.last().cloned().unwrap_or_else(|| path.into());
    } else {
        state.selected_many.insert(path.into());
        state.selected = path.into();
    }
}

fn after_remove(path: &str, parent: &str, index: usize) -> String {
    let Some(rest) = path.strip_prefix(&format!("{parent}/")) else { return path.into() };
    let (first, tail) = rest.split_once('/').unwrap_or((rest, ""));
    let Ok(n) = first.parse::<usize>() else { return path.into() };
    format!(
        "{parent}/{}{}{}",
        if n > index { n - 1 } else { n },
        if tail.is_empty() { "" } else { "/" },
        tail
    )
}

/// Transactional relocation. Never allow cycles, remove a required group child,
/// or discard list-cell allocation metadata when moving between lists.
pub(super) fn relocate(doc: &mut Value, from: &str, target: &str, pos: Position) -> Option<String> {
    if from == target
        || target.starts_with(&format!("{from}/"))
        || !shortcuts::can_edit(doc, from, 3)
    {
        return None;
    }
    let (parent, index, cell) = array_position(from)?;
    let mut next = doc.clone();
    let item = next.pointer_mut(&parent)?.as_array_mut()?.remove(index);
    let target = after_remove(target, &parent, index);
    let (dest, at, to_cell) = destination(&next, &target, pos)?;
    let item = match (cell, to_cell) {
        (true, false) => item[0].clone(),
        (false, true) => json!([item, 150.0, true]),
        _ => item,
    };
    let array = next.pointer_mut(&dest)?.as_array_mut()?;
    if at > array.len() {
        return None;
    }
    array.insert(at, item);
    *doc = next;
    Some(format!("{dest}/{at}{}", if to_cell { "/0" } else { "" }))
}

pub(super) fn destination(
    doc: &Value,
    target: &str,
    pos: Position,
) -> Option<(String, usize, bool)> {
    Some(if pos == Position::Inside {
        let p = design::insertion_parent(doc, target)?;
        let n = doc.pointer(&p)?;
        if n["type"] == "list" {
            (format!("{p}/row_template"), n["row_template"].as_array()?.len(), true)
        } else {
            (format!("{p}/children"), n["children"].as_array()?.len(), false)
        }
    } else {
        let (p, i, c) = array_position(target)?;
        if p.strip_suffix("/children")
            .and_then(|p| doc.pointer(p))
            .is_some_and(|n| n["type"] == "group")
        {
            return None;
        }
        (p, i + usize::from(pos == Position::After), c)
    })
}

pub(super) fn drag_target(
    ui: &mut Ui,
    response: &egui::Response,
    path: &str,
    state: &mut State,
    inside_only: bool,
) {
    if let Some(payload) = response.dnd_hover_payload::<Drag>() {
        let mode = drop_position(ui, response, inside_only);
        if payload.path != path && !path.starts_with(&format!("{}/", payload.path)) {
            paint_drop(ui, response, mode);
            if response.dnd_release_payload::<Drag>().is_some() {
                state.move_pending = Some(((*payload).clone(), path.into(), mode));
            }
        }
    }
}

pub(super) fn drop_position(ui: &Ui, response: &egui::Response, inside_only: bool) -> Position {
    let pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(response.rect.center());
    if inside_only {
        Position::Inside
    } else if pos.y < response.rect.top() + response.rect.height() / 3.0 {
        Position::Before
    } else if pos.y > response.rect.bottom() - response.rect.height() / 3.0 {
        Position::After
    } else {
        Position::Inside
    }
}

pub(super) fn paint_drop(ui: &Ui, response: &egui::Response, mode: Position) {
    let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
    if mode == Position::Inside {
        ui.painter().rect_stroke(response.rect, 0, stroke, egui::StrokeKind::Inside);
    } else {
        let y = if mode == Position::Before { response.rect.top() } else { response.rect.bottom() };
        ui.painter().hline(response.rect.x_range(), y, stroke);
    }
}

pub(super) fn finish(doc: &mut Value, state: &mut State) {
    if let Some((ty, target, pos)) = state.insert_pending.take()
        && let Some(path) = design::insert_at(doc, &target, pos, ty)
    {
        select(state, &path, false);
        if ty == "swap" {
            state.new_swap_slot = Some(path);
        }
    }
    if let Some((drag, target, pos)) = state.move_pending.take()
        && doc.pointer(&drag.path) == Some(&drag.node)
        && let Some(path) = relocate(doc, &drag.path, &target, pos)
    {
        select(state, &path, false);
    }
}

pub(super) fn delete_selected(doc: &mut Value, state: &mut State) {
    let mut paths: Vec<_> = state
        .selected_many
        .iter()
        .filter(|p| {
            !state
                .selected_many
                .iter()
                .any(|parent| *p != parent && p.starts_with(&format!("{parent}/")))
        })
        .cloned()
        .collect();
    if paths.is_empty() {
        paths.push(state.selected.clone());
    }
    paths.sort_by(|a, b| {
        let key = |s: &str| {
            s.split('/')
                .map(|p| p.parse::<usize>().map_or_else(|_| p.to_owned(), |n| format!("{n:020}")))
                .collect::<Vec<_>>()
        };
        key(b).cmp(&key(a))
    });
    for mut path in paths {
        if shortcuts::can_edit(doc, &path, 3) {
            rearrange(doc, &mut path, 3);
            state.selected = path;
        }
    }
    state.selected_many.clear();
}

pub(super) fn wrap(doc: &mut Value, selected: &str, ty: &str) {
    let Some(node) = doc.pointer(selected).cloned() else { return };
    if selected.is_empty() {
        return;
    }
    let mut wrapper = mg_nui::template(ty);
    assign_ids(&mut wrapper, doc);
    wrapper["children"] = json!([node]);
    *doc.pointer_mut(selected).unwrap() = wrapper;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palette_insertion_preserves_list_cells_and_required_group_child() {
        let mut doc = mg_nui::window();
        doc["root"]["children"] = json!([
            {"type":"list","row_template":[[{"type":"label","id":"keep"},222.0,false]]},
            {"type":"group","children":[{"type":"col","children":[]}]}
        ]);
        let original_cell = doc["root"]["children"][0]["row_template"][0].clone();
        let path = design::insert_at(
            &mut doc,
            "/root/children/0/row_template/0/0",
            Position::Before,
            "button",
        )
        .unwrap();
        assert_eq!(doc.pointer(&path).unwrap()["type"], "button");
        assert_eq!(doc["root"]["children"][0]["row_template"][1], original_cell);
        let before = doc.clone();
        assert!(
            design::insert_at(&mut doc, "/root/children/1/children/0", Position::After, "row")
                .is_none()
        );
        assert_eq!(doc, before);
        let mut state = State {
            insert_pending: Some(("swap", "/root/children/1".into(), Position::Inside)),
            ..Default::default()
        };
        finish(&mut doc, &mut state);
        let mut settings = Settings::default();
        layouts::finish_insert(&doc, &mut settings, &mut state);
        assert_eq!(doc["root"]["children"][1]["children"].as_array().unwrap().len(), 1);
        assert!(layouts::is_swap(&settings, doc.pointer(&state.selected).unwrap()));
        assert!(settings.actions.is_empty());
    }

    #[test]
    fn nui_reparent_is_atomic_preserves_cells_and_rejects_cycles() {
        let mut d = mg_nui::window();
        d["root"]["children"] = json!([
            {"type":"list","row_template":[[{"type":"label","id":"keep","custom":17},222.0,false]]},
            {"type":"list","row_template":[]}, {"type":"col","children":[]}
        ]);
        let path = relocate(
            &mut d,
            "/root/children/0/row_template/0/0",
            "/root/children/1",
            Position::Inside,
        )
        .unwrap();
        assert_eq!(d["root"]["children"][1]["row_template"][0][1], 222.0);
        assert_eq!(d.pointer(&path).unwrap()["custom"], 17);
        let before = d.clone();
        assert!(relocate(&mut d, "/root/children/1", &path, Position::Inside).is_none());
        assert_eq!(d, before);
        relocate(&mut d, "/root/children/2", "/root/children/0", Position::Before).unwrap();
        assert_eq!(d["root"]["children"][0]["type"], "col");
    }

    #[test]
    fn nui_multi_delete_handles_numeric_indices_and_nested_selections() {
        let mut d = mg_nui::window();
        d["root"]["children"]=Value::Array((0..12).map(|i|json!({"type":"col","id":format!("node_{i}"),"children":[{"type":"label","value":"keep"}]})).collect());
        let mut state = State::default();
        for path in ["/root/children/2", "/root/children/10", "/root/children/2/children/0"] {
            select(&mut state, path, true);
        }
        delete_selected(&mut d, &mut state);
        let ids = mg_nui::element_ids(&d);
        assert!(!ids.contains("node_2"));
        assert!(!ids.contains("node_10"));
        assert!(ids.contains("node_11"));
        assert_eq!(d["root"]["children"].as_array().unwrap().len(), 10);
    }
}
