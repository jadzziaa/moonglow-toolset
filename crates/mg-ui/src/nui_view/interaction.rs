//! What the canvas does without changing the window: a preview session
//! ([`Session`]), in which controls take values and handlers' ready-made
//! actions are followed (no NWScript is run), and the gestures that do
//! change it once let go: moving the window ([`WindowMove`]) and sizing a
//! control or the window ([`Resize`], [`resize_handles`]).
use super::*;
use egui::{Pos2, Vec2};

#[derive(Clone)]
pub(super) struct Session {
    source: Value,
    initial: Settings,
    pub doc: Value,
    pub settings: Settings,
    pub last_event: String,
    pub closed: bool,
    pub row_values: std::collections::BTreeMap<(String, usize), Value>,
}

pub(super) struct Change {
    pub path: String,
    pub row: Option<usize>,
    pub value: Option<Value>,
    pub normalization: bool,
}

#[derive(Clone)]
pub(super) struct WindowMove {
    pub source: Value,
    pub origin: Pos2,
    pub initial: Vec2,
    pub position: Vec2,
    pub scale: f32,
}

/// `value[key]` to write to, as `serde_json` gives it (an absent key is
/// added, null becomes an object), but `None` and no panic where `value` is
/// something else: a window written by hand may hold a string or a number
/// where an object belongs.
pub(super) fn field<'a>(value: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    if value.is_null() {
        *value = Value::Object(Default::default());
    }
    Some(value.as_object_mut()?.entry(key).or_insert(Value::Null))
}

/// `value[key] = new`, where `value` can hold it ([`field`]).
pub(super) fn set(value: &mut Value, key: &str, new: Value) {
    if let Some(slot) = field(value, key) {
        *slot = new;
    }
}

/// Whether the simulation has where to write a value of control `node`
/// (in list row `row`): the control's own `value`, or its bind's first
/// value, and that row of it for a list's. [`Session::apply`] writes
/// nowhere else, and says what is missing.
pub(super) fn has_place(node: &Value, s: &Settings, row: Option<usize>) -> bool {
    let Some(bind) = node["value"]["bind"].as_str() else { return true };
    let Some(binding) = s.bindings.get(bind) else { return false };
    row.is_none_or(|row| binding.value.as_array().is_some_and(|rows| row < rows.len()))
}

impl WindowMove {
    pub(super) fn update(&mut self, pointer: Pos2) {
        self.position =
            (self.initial + (pointer - self.origin) / self.scale).round().max(Vec2::ZERO);
    }
    pub(super) fn apply(&self, doc: &mut Value) {
        if let Some(geometry) = field(doc, "geometry") {
            set(geometry, "x", json!(self.position.x));
            set(geometry, "y", json!(self.position.y));
        }
    }
}

impl Session {
    pub(super) fn new(doc: &Value, settings: &Settings) -> Self {
        let mut session = Self {
            source: doc.clone(),
            initial: settings.clone(),
            doc: doc.clone(),
            settings: settings.clone(),
            last_event: String::new(),
            closed: false,
            row_values: Default::default(),
        };
        session.dispatch("open", "", None, 0);
        session
    }

    pub(super) fn matches(&self, doc: &Value, settings: &Settings) -> bool {
        &self.source == doc && &self.initial == settings
    }

    /// The client writes the constrained dimensions back to a geometry bind.
    /// Screen placement and arbitrary engine watch scheduling are not simulated here.
    pub(super) fn constrain_geometry(&mut self) {
        if !resolved(&self.doc["size_constraint"], &self.settings).is_object() {
            return;
        }
        let size = preview::window_size(&self.doc, &self.settings);
        let Some(bind) = self.doc["geometry"]["bind"].as_str() else { return };
        let Some(binding) = self.settings.bindings.get_mut(bind) else { return };
        if binding.value.is_object() {
            set(&mut binding.value, "w", json!(size.x));
            set(&mut binding.value, "h", json!(size.y));
        }
    }

    pub(super) fn collapse(&mut self, collapsed: bool) {
        if let Some(bind) = self.doc["collapsed"]["bind"].as_str().map(str::to_owned) {
            if let Some(binding) = self.settings.bindings.get_mut(&bind) {
                binding.value = json!(collapsed);
                self.dispatch("watch", &bind, None, 0);
            }
        } else {
            set(&mut self.doc, "collapsed", json!(collapsed));
        }
    }

    pub(super) fn apply(&mut self, change: Change) {
        let Some(node) = self.doc.pointer_mut(&change.path) else { return };
        let id = node["id"].as_str().map_or_else(|| design::node_name(node), str::to_owned);
        let clicked = !change.normalization
            && (change.value.is_none()
                || matches!(
                    node["type"].as_str(),
                    Some("check" | "button_select" | "combo" | "options" | "tabbar")
                ));
        let watched =
            change.value.as_ref().and_then(|_| node["value"]["bind"].as_str()).map(str::to_owned);
        let kind = if change.value.is_some() { "change" } else { "click" };
        if let Some(value) = change.value {
            if let Some(bind) = node["value"]["bind"].as_str() {
                let Some(binding) = self.settings.bindings.get_mut(bind) else {
                    self.last_event = format!("Missing preview bind: {bind}");
                    return;
                };
                if let Some(row) = change.row {
                    let Some(target) = binding.value.as_array_mut().and_then(|a| a.get_mut(row))
                    else {
                        self.last_event = format!("Missing preview row {row} in bind: {bind}");
                        return;
                    };
                    *target = value;
                } else {
                    binding.value = value;
                }
            } else {
                // Literal row values also belong to one rendered instance, not every row.
                if let Some(row) = change.row {
                    self.row_values.insert((change.path.clone(), row), value);
                } else {
                    set(node, "value", value);
                }
            }
        }
        let row = change.row.map_or(String::new(), |r| format!(", row {r}"));
        self.last_event = format!("Local {kind}: {id}{row}");
        if let Some(bind) = watched {
            self.dispatch("watch", &bind, change.row, 0);
        }
        if clicked {
            self.dispatch("click", &id, change.row, 0);
        }
    }

    /// Models only declarative editor actions, never arbitrary user NWScript.
    pub(super) fn dispatch(
        &mut self,
        event: &str,
        element: &str,
        row: Option<usize>,
        depth: usize,
    ) {
        if depth >= 32 {
            self.last_event = "Action recursion stopped; look at the watch handlers".into();
            return;
        }
        let action = self
            .settings
            .handlers
            .iter()
            .find(|r| {
                r.event == event && (matches!(event, "open" | "close") || r.element == element)
            })
            .map(|r| r.action.clone());
        let Some(action) = action else { return };
        let mut changed = None;
        match action {
            // Your code runs in the game; the preview can't run NWScript.
            mg_nui::Action::Code => {
                self.last_event = format!("{} {element}: your handler runs in the game", event);
            }
            // (An action this version does not know does nothing here.)
            mg_nui::Action::Other(_) => {}
            mg_nui::Action::Close => self.closed = true,
            mg_nui::Action::Toggle { bind } => {
                if let Some(binding) = self.settings.bindings.get_mut(&bind) {
                    let value = if binding.value.is_array() {
                        row.and_then(|i| binding.value.get_mut(i))
                    } else {
                        Some(&mut binding.value)
                    };
                    if let Some(v) = value
                        && let Some(on) = v.as_bool()
                    {
                        *v = json!(!on);
                        changed = Some(bind);
                    }
                }
            }
            mg_nui::Action::Set { bind, value } => {
                if let Some(b) = self.settings.bindings.get_mut(&bind) {
                    b.value = value;
                    changed = Some(bind);
                }
            }
            mg_nui::Action::Variant { group, variant: view } => {
                if let Some(layout) = self.settings.variants.get(&view) {
                    fn replace(node: &mut Value, id: &str, layout: &Value) {
                        if node["type"] == "group" && node["id"] == id {
                            set(node, "children", json!([layout]));
                            return;
                        }
                        match node {
                            Value::Object(o) => {
                                for v in o.values_mut() {
                                    replace(v, id, layout)
                                }
                            }
                            Value::Array(a) => {
                                for v in a {
                                    replace(v, id, layout)
                                }
                            }
                            _ => {}
                        }
                    }
                    if group == "_window_" {
                        set(&mut self.doc, "root", layout.clone());
                    } else {
                        if let Some(root) = self.doc.get_mut("root") {
                            replace(root, &group, layout);
                        }
                    }
                    self.row_values.clear();
                }
            }
        }
        if let Some(bind) = changed {
            self.dispatch("watch", &bind, None, depth + 1);
        }
    }
}

pub(super) fn resize_handles(
    ui: &mut Ui,
    doc: &Value,
    state: &mut State,
    selected: Option<egui::Rect>,
    scale: f32,
    canvas_origin: Pos2,
) {
    if !state.guides() {
        return;
    }
    let Some(rect) = selected else { return };
    let path = state.selected.clone();
    // List cells are allocated by the row template; resizing the list is supported.
    if path.contains("/row_template/") {
        return;
    }
    let Some(node) = doc.pointer(&path) else { return };
    let window = path.is_empty();
    // A geometry bind remains a bind; edit its preview value in Bindings.
    if window
        && (node["geometry"]["bind"].is_string()
            || !node["geometry"]["w"].is_number()
            || !node["geometry"]["h"].is_number())
    {
        return;
    }
    for (x, y, position, name, cursor) in [
        (true, false, rect.right_center(), "Resize width", egui::CursorIcon::ResizeHorizontal),
        (false, true, rect.center_bottom(), "Resize height", egui::CursorIcon::ResizeVertical),
        (true, true, rect.right_bottom(), "Resize width and height", egui::CursorIcon::ResizeNwSe),
    ] {
        if !window && ((x && node["width"].is_object()) || (y && node["height"].is_object())) {
            continue;
        }
        let hit = egui::Rect::from_center_size(position, egui::Vec2::splat(11.0));
        let response = ui
            .interact(hit, ui.id().with(("resize", &path, x, y)), egui::Sense::drag())
            .on_hover_cursor(cursor);
        let name = if window { name.replacen("Resize", "Resize window", 1) } else { name.into() };
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
        ui.painter().rect_filled(hit.shrink(2.0), 1, ui.visuals().selection.bg_fill);
        ui.painter().rect_stroke(
            hit.shrink(2.0),
            1,
            egui::Stroke::new(1.0, egui::Color32::WHITE),
            egui::StrokeKind::Inside,
        );
        if response.drag_started()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            state.resize = Some(Resize {
                path: path.clone(),
                source: doc.clone(),
                origin,
                scale,
                anchor: rect.min - canvas_origin,
                initial: rect.size() / scale,
                size: rect.size() / scale,
                horizontal: x,
                vertical: y,
            });
        }
        response.on_hover_text("Drag to resize, Esc to cancel. One Undo restores the size.");
    }
}

/// A drag only changes the rendered copy until release: one module Undo step.
#[derive(Clone)]
pub(super) struct Resize {
    pub path: String,
    pub source: Value,
    pub origin: Pos2,
    pub scale: f32,
    pub anchor: Vec2,
    pub initial: Vec2,
    pub size: Vec2,
    pub horizontal: bool,
    pub vertical: bool,
}

/// The largest size a child may take across its row (height) or column
/// (width) and still let the client build the window: its size and margins
/// within the parent's (NWN refuses the window past that; see mg_nui::validate).
pub(super) fn cross_room(doc: &Value, path: &str) -> Vec2 {
    let mut room = Vec2::splat(4096.0);
    let Some((parent, _)) = path.rsplit_once("/children/") else { return room };
    let (Some(parent), Some(node)) = (doc.pointer(parent), doc.pointer(path)) else {
        return room;
    };
    let margin = node["margin"].as_f64().unwrap_or(2.0) as f32;
    match parent["type"].as_str() {
        Some("row") => {
            if let Some(h) = parent["height"].as_f64() {
                room.y = h as f32 - 2.0 * margin;
            }
        }
        Some("col") => {
            if let Some(w) = parent["width"].as_f64() {
                room.x = w as f32 - 2.0 * margin;
            }
        }
        _ => {}
    }
    room
}

impl Resize {
    pub(super) fn move_to(&mut self, pointer: Pos2) {
        let delta = (pointer - self.origin) / self.scale.max(0.001);
        let minimum = if self.path.is_empty() { 80.0 } else { 8.0 };
        let room = cross_room(&self.source, &self.path).max(Vec2::splat(minimum));
        // An axis the pointer has not moved a point along keeps the size
        // it had. (The size drawn is seldom whole: three labels sharing a
        // row are 126.67 each, and any size is a fraction under Zoom Fit.
        // Rounded as soon as a handle was pressed, a click on one wrote a
        // width, and the label no longer shared its row.)
        let sized = |initial: f32, moved: f32, room: f32| match moved.round() == 0.0 {
            true => initial,
            false => (initial + moved).round().clamp(minimum, room),
        };
        if self.horizontal {
            self.size.x = sized(self.initial.x, delta.x, room.x);
        }
        if self.vertical {
            self.size.y = sized(self.initial.y, delta.y, room.y);
        }
    }

    /// The gesture made the control (or the window) another size.
    pub(super) fn changed(&self) -> bool {
        self.size != self.initial
    }

    /// The size as the gesture has it, on the axes it changed: the other
    /// keeps what the window says of it (none, or a bind).
    pub(super) fn apply(&self, doc: &mut Value) {
        let Some(node) = doc.pointer_mut(&self.path) else { return };
        let (node, width, height) = if self.path.is_empty() {
            (field(node, "geometry"), "w", "h")
        } else {
            (Some(node), "width", "height")
        };
        let Some(node) = node else { return };
        if self.horizontal && self.size.x != self.initial.x {
            set(node, width, json!(self.size.x));
        }
        if self.vertical && self.size.y != self.initial.y {
            set(node, height, json!(self.size.y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    /// A handle pressed and let go where it was writes nothing, whatever
    /// size the control was drawn at; moved along one axis, it sizes that
    /// axis alone. (Three labels sharing a row are 126.67 wide each: a
    /// click on one's handle wrote "width": 127.0.)
    #[test]
    fn a_handle_not_moved_leaves_the_size_alone() {
        let doc = json!({"root": {"type": "row", "children": [
            {"type": "label"}, {"type": "label"}, {"type": "label"}]}});
        let resize = |scale: f32, initial: Vec2| Resize {
            path: "/root/children/1".into(),
            source: doc.clone(),
            origin: pos2(300.0, 200.0),
            scale,
            anchor: vec2(20.0, 20.0),
            initial,
            size: initial,
            horizontal: true,
            vertical: true,
        };
        for (scale, initial) in [(1.0, vec2(126.67, 30.0)), (0.73, vec2(173.972_6, 41.095_9))] {
            let mut r = resize(scale, initial);
            // (And a hair off, as a press that trembles.)
            for at in [pos2(300.0, 200.0), pos2(300.3, 199.7)] {
                r.move_to(at);
                assert!(!r.changed(), "{scale} {initial:?} {at:?}: {:?}", r.size);
                let mut after = doc.clone();
                r.apply(&mut after);
                assert_eq!(after, doc);
            }
            // Along the row only: a width, whole, and no height.
            r.move_to(pos2(300.0 + 10.0 * scale, 200.0));
            assert!(r.changed());
            let mut after = doc.clone();
            r.apply(&mut after);
            let label = &after["root"]["children"][1];
            assert_eq!(label["width"], json!((initial.x + 10.0).round()));
            assert!(label.get("height").is_none(), "{label}");
        }
    }
}
