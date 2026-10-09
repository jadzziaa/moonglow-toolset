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
    let mut item = match (cell, to_cell) {
        (true, false) => item[0].clone(),
        (false, true) => json!([item, 150.0, true]),
        _ => item,
    };
    if !to_cell && let Some(parent) = dest.strip_suffix("/children").and_then(|p| next.pointer(p)) {
        design::fit_into(&parent.clone(), &mut item);
    }
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
        let in_group = path
            .rsplit_once("/children/")
            .and_then(|(group, _)| doc.pointer(group))
            .is_some_and(|n| n["type"] == "group");
        if in_group && shortcuts::can_edit(doc, &path, 3) {
            *doc.pointer_mut(&path).unwrap() = mg_nui::template("col");
            state.selected = path;
        } else if shortcuts::can_edit(doc, &path, 3) {
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

    /// A row 30 high holding a 30-high button is a window NWN EE refuses to
    /// build ("The constraint can not be satisfied"): no editing gesture of
    /// the Creator leads there.
    #[test]
    fn nui_edits_never_make_a_window_the_client_refuses() {
        let errors = |d: &Value| {
            mg_nui::validate(d, &Settings::default())
                .into_iter()
                .filter(|x| x.severity == mg_nui::Severity::Error)
                .collect::<Vec<_>>()
        };
        let mut d = mg_nui::window();
        d["root"]["children"] = json!([
            {"type":"row","height":30.0,"children":[]},
            {"type":"col","width":120.0,"children":[]},
            {"type":"button","label":"tall","height":50.0}
        ]);
        // A palette control (30 high, 150 wide) into the fixed row and column.
        let mut selected = "/root/children/0".to_owned();
        design::insert(&mut d, &mut selected, "button");
        assert_eq!(d.pointer(&selected).unwrap()["height"], 26.0);
        let at = design::insert_at(&mut d, "/root/children/1", Position::Inside, "button").unwrap();
        assert_eq!(d.pointer(&at).unwrap()["height"], 30.0);
        assert!(errors(&d).is_empty(), "{:?}", errors(&d));
        // A taller control moved into the row shrinks to fit.
        let moved =
            relocate(&mut d, "/root/children/2", "/root/children/0", Position::Inside).unwrap();
        assert_eq!(d.pointer(&moved).unwrap()["height"], 26.0);
        assert!(errors(&d).is_empty(), "{:?}", errors(&d));
        // Resizing by a handle stops at the room the row leaves.
        let mut resize = interaction::Resize {
            path: moved.clone(),
            source: d.clone(),
            origin: egui::pos2(0.0, 0.0),
            scale: 1.0,
            anchor: egui::Vec2::ZERO,
            initial: egui::vec2(150.0, 26.0),
            size: egui::vec2(150.0, 26.0),
            horizontal: true,
            vertical: true,
        };
        resize.move_to(egui::pos2(40.0, 40.0));
        assert_eq!(resize.size, egui::vec2(190.0, 26.0));
        resize.apply(&mut d);
        assert!(errors(&d).is_empty(), "{:?}", errors(&d));
        // A fixed height given to a row leaves room for its children and margins.
        let row = d["root"]["children"][0].as_object().unwrap();
        assert_eq!(super::super::fitting_size(row, "height", 30.0), 34.0);
        d["root"]["children"][0]["children"][0]["height"] = json!(40.0);
        let row = d["root"]["children"][0].as_object().unwrap();
        assert_eq!(super::super::fitting_size(row, "height", 30.0), 44.0);
        assert_eq!(super::super::fitting_size(row, "width", 150.0), 150.0);
    }

    #[test]
    fn nui_deleting_a_groups_contents_leaves_an_empty_column() {
        // A group (a swap layout too) holds exactly one child in NUI.
        let mut d = mg_nui::window();
        d["root"]["children"] = json!([{"type":"group","id":"swap","children":[
            {"type":"row","children":[{"type":"check","label":"Option"}]}]}]);
        let mut state = State::default();
        let contents = "/root/children/0/children/0";
        assert!(super::super::shortcuts::can_edit(&d, contents, 3));
        assert!(!super::super::shortcuts::can_edit(&d, contents, 2), "no second child");
        select(&mut state, contents, false);
        delete_selected(&mut d, &mut state);
        assert_eq!(d.pointer(contents), Some(&mg_nui::template("col")));
        assert!(!super::super::shortcuts::can_edit(&d, contents, 3), "nothing left to delete");
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
