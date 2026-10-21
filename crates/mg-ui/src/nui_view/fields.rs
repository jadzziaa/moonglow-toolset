//! The fields of the Creator's forms: text, numbers, colors, a combo's
//! options, a chart's series, a list's rows. Each edits a value of the
//! window's JSON as it is: nothing of the editor's is written into the JUI.
//!
//! The form's fields are set as the other editors' are: text when its field
//! is left, a number when its drag is let go, a color when its picker
//! closes, each one Undo step. Until then the field's value shows what is
//! typed, dragged to or picked, for the canvas drawn after it, and
//! [`put_back`] takes it out again before the module is written.
use super::*;

/// What the form's text fields hold while they are typed in.
#[derive(Clone, Default)]
struct Typed(std::collections::HashMap<egui::Id, String>);

/// The values shown this frame that are not set yet: what the module holds,
/// and what the field shows in its place.
#[derive(Clone, Default)]
struct Unset(Vec<(Value, Value)>);

fn unset_id() -> egui::Id {
    egui::Id::new("nui-fields-unset")
}

fn shows_unset(ui: &Ui, set: Value, shown: Value) {
    ui.data_mut(|d| d.get_temp_mut_or_default::<Unset>(unset_id()).0.push((set, shown)));
}

fn of_id() -> egui::Id {
    egui::Id::new("nui-fields-of")
}

/// Once a frame, before the form is drawn.
pub(super) fn begin(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        d.remove::<Unset>(unset_id());
        d.remove::<egui::Id>(of_id());
    });
}

/// Says whose fields follow (the selected control's): a field being typed
/// in is another control's field of the same name when the selection
/// changes, not the same field showing another value, which would take what
/// was typed.
pub(super) fn of(ctx: &egui::Context, whose: egui::Id) {
    ctx.data_mut(|d| d.insert_temp(of_id(), whose));
}

/// `add` with ids of their own for the fields of what [`of`] named.
fn scoped<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    match ui.data(|d| d.get_temp::<egui::Id>(of_id())) {
        Some(whose) => ui.push_id(whose, add).inner,
        None => add(ui),
    }
}

/// `now` (what the form left of `was`) without the values it only shows:
/// those are not written. A change made in the same frame stays.
pub(super) fn put_back(ctx: &egui::Context, was: &Value, now: &mut Value) {
    fn walk(unset: &[(Value, Value)], was: &Value, now: &mut Value) {
        if was == now {
            return;
        }
        if unset.iter().any(|(set, shown)| set == was && shown == now) {
            now.clone_from(was);
            return;
        }
        match (was, now) {
            (Value::Object(was), Value::Object(now)) => {
                for (key, now) in now.iter_mut() {
                    if let Some(was) = was.get(key) {
                        walk(unset, was, now);
                    }
                }
            }
            (Value::Array(was), Value::Array(now)) if was.len() == now.len() => {
                for (was, now) in was.iter().zip(now) {
                    walk(unset, was, now);
                }
            }
            _ => {}
        }
    }
    let Some(unset) = ctx.data(|d| d.get_temp::<Unset>(unset_id())) else { return };
    walk(&unset.0, was, now);
}

/// [`put_back`] for the settings: the binds' values, the variants and the
/// window's ID.
pub(super) fn put_back_settings(ctx: &egui::Context, was: &Settings, now: &mut Settings) {
    if ctx.data(|d| d.get_temp::<Unset>(unset_id())).is_none() {
        return;
    }
    for (name, binding) in &mut now.bindings {
        if let Some(was) = was.bindings.get(name) {
            put_back(ctx, &was.value, &mut binding.value);
        }
    }
    for (name, view) in &mut now.views {
        if let Some(was) = was.views.get(name) {
            put_back(ctx, was, view);
        }
    }
    if let (Some(was), Some(now)) = (&was.window_id, &mut now.window_id) {
        let (was, mut id) = (Value::from(was.as_str()), Value::from(now.as_str()));
        put_back(ctx, &was, &mut id);
        if let Value::String(id) = id {
            *now = id;
        }
    }
}

/// A one-line text field of the form (`widgets::commit_text_in`): `text` is
/// set when the field is left, and shows what is typed until then. The
/// field's response, to label it.
pub(super) fn text(ui: &mut Ui, text: &mut String, width: f32) -> Option<egui::Response> {
    scoped(ui, |ui| text_field(ui, text, width))
}

fn text_field(ui: &mut Ui, text: &mut String, width: f32) -> Option<egui::Response> {
    let id = ui.next_auto_id().with("nui-text");
    let held = egui::Id::new("nui-fields-typed");
    let mut typed = ui.data_mut(|d| d.remove_temp::<Typed>(held)).unwrap_or_default();
    let set = crate::widgets::commit_text_in(&mut typed.0, ui, id, text, None, width, None);
    let has_keys = ui.memory(|m| m.has_focus(id));
    match set {
        Some(set) => *text = set,
        None => {
            if has_keys && let Some(shown) = typed.0.get(&id).filter(|shown| *shown != text) {
                shows_unset(ui, Value::from(text.as_str()), Value::from(shown.as_str()));
                text.clone_from(shown);
            }
        }
    }
    // (Kept only while it is typed in.)
    if !has_keys {
        typed.0.remove(&id);
    }
    ui.data_mut(|d| d.insert_temp(held, typed));
    ui.ctx().read_response(id)
}

/// A number field of the form (`widgets::drag_number_shown`): set when it is
/// typed or when a drag across it is let go, and showing what the drag has
/// reached until then.
pub(super) fn number<T>(
    ui: &mut Ui,
    v: &mut Value,
    current: T,
    make: impl for<'a> FnOnce(egui::DragValue<'a>) -> egui::DragValue<'a>,
) -> egui::Response
where
    T: egui::emath::Numeric,
    Value: From<T>,
{
    // (Typed, it is set when the field is left, not at each digit.)
    let (set, shown, r) = scoped(ui, |ui| {
        crate::widgets::drag_number_shown(ui, current, |d| make(d).update_while_editing(false))
    });
    if let Some(set) = set {
        *v = Value::from(set);
    } else if shown != current {
        let shown = Value::from(shown);
        shows_unset(ui, v.clone(), shown.clone());
        *v = shown;
    }
    r
}

/// A color's swatch (`o` holds its `r`, `g`, `b` and `a`), which opens a
/// picker: what is picked is set when the picker closes, as a blueprint's
/// colors are, and shown until then.
pub(super) fn color(ui: &mut Ui, o: &mut serde_json::Map<String, Value>) {
    const CHANNELS: [&str; 4] = ["r", "g", "b", "a"];
    let current = CHANNELS.map(|k| o.get(k).and_then(Value::as_u64).unwrap_or(255).min(255) as u8);
    let whose = ui.data(|d| d.get_temp::<egui::Id>(of_id()));
    let pending_id = ui.next_auto_id().with(whose).with("nui-color");
    let pending: Option<[u8; 4]> = ui.data(|d| d.get_temp(pending_id));
    let mut rgba = pending.unwrap_or(current);
    let (changed, open) = ui
        .push_id(pending_id, |ui| {
            let popup = ui.auto_id_with("popup");
            let changed = ui.color_edit_button_srgba_unmultiplied(&mut rgba).changed();
            (changed, egui::Popup::is_id_open(ui.ctx(), popup))
        })
        .inner;
    let Some(picked) = changed.then_some(rgba).or(pending) else { return };
    let was = Value::Object(o.clone());
    if picked != current {
        for (k, c) in CHANNELS.iter().zip(picked) {
            o.insert((*k).into(), json!(c));
        }
    }
    if open {
        ui.data_mut(|d| d.insert_temp(pending_id, picked));
        if picked != current {
            shows_unset(ui, was, Value::Object(o.clone()));
        }
    } else {
        ui.data_mut(|d| d.remove::<[u8; 4]>(pending_id));
    }
}

/// Keep option labels and values editable at the inspector's actual width.
/// A text field using the whole row before a DragValue made the vertical scroll
/// area grow sideways and displaced the right panel underneath the canvas.
pub(super) fn options(ui: &mut Ui, elements: &mut Vec<Value>, combo: bool) {
    let mut remove = None;
    for (i, element) in elements.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            let entry = if element.is_array() { element.get_mut(0) } else { Some(&mut *element) };
            if let Some(Value::String(label)) = entry
                && let Some(field) = text(ui, label, ui.available_width())
            {
                field.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::TextEdit,
                        true,
                        format!("Option {} label", i + 1),
                    )
                });
            }
            ui.horizontal_wrapped(|ui| {
                if let Some(n) = element.get_mut(1) {
                    widgets::field_label(ui, "Value");
                    scalar(ui, n);
                }
                if ui.small_button("Remove").on_hover_text("Remove option").clicked() {
                    remove = Some(i);
                }
            });
        });
    }
    if let Some(i) = remove {
        elements.remove(i);
    }
    let next = next_option_value(elements);
    if ui.add_enabled(!combo || next.is_some(), egui::Button::new("+ Add option").small()).clicked()
    {
        if combo {
            if let Some(next) = next {
                elements.push(json!(["Option", next]));
            }
        } else {
            elements.push("Option".into());
        }
    }
}

fn next_option_value(elements: &[Value]) -> Option<i32> {
    let max = elements.iter().filter_map(|e| e[1].as_i64()).max().unwrap_or(-1);
    if let Some(next) = max.checked_add(1).and_then(|n| i32::try_from(n).ok()) {
        return Some(next);
    }
    // NWScript ints are signed 32-bit. When max + 1 would overflow, use the
    // first unused nonnegative value instead of introducing an invalid ID.
    let used: std::collections::BTreeSet<_> = elements
        .iter()
        .filter_map(|e| e[1].as_i64().and_then(|n| i32::try_from(n).ok()))
        .filter(|n| *n >= 0)
        .collect();
    let mut next = 0_i32;
    for id in used {
        if id != next {
            break;
        }
        next = next.checked_add(1)?;
    }
    Some(next)
}

pub(super) fn value(ui: &mut Ui, v: &mut Value, depth: usize) {
    if depth > 12 {
        ui.weak("Nested value: use JUI source");
        return;
    }
    match v {
        Value::Array(values) => {
            let mut remove = None;
            for (i, item) in values.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    egui::CollapsingHeader::new(format!("Item {}", i + 1))
                        .default_open(values_simple(item))
                        .show(ui, |ui| {
                            value(ui, item, depth + 1);
                            if ui.small_button("Remove item").clicked() {
                                remove = Some(i);
                            }
                        });
                });
            }
            if let Some(i) = remove {
                values.remove(i);
            }
            if ui.small_button("+ Add item").clicked() {
                values.push(values.last().cloned().unwrap_or(json!("")));
            }
        }
        Value::Object(o)
            if o.len() == 4
                && ["r", "g", "b", "a"].iter().all(|k| o.get(*k).is_some_and(Value::is_number)) =>
        {
            scalar(ui, v);
        }
        Value::Object(o) => {
            for (key, item) in o.iter_mut() {
                ui.push_id(key, |ui| {
                    if item.is_array() || item.is_object() {
                        egui::CollapsingHeader::new(key).show(ui, |ui| value(ui, item, depth + 1));
                    } else {
                        widgets::field_label(ui, key);
                        value(ui, item, depth + 1);
                    }
                });
            }
        }
        Value::Null => {
            ui.menu_button("Set value…", |ui| {
                for (label, next) in [
                    ("Text", json!("")),
                    ("Number", json!(0.0)),
                    ("Boolean", json!(false)),
                    ("Array", json!([])),
                    ("Color", json!({"r":255,"g":255,"b":255,"a":255})),
                ] {
                    if ui.button(label).clicked() {
                        *v = next;
                        ui.close();
                    }
                }
            });
        }
        _ => {
            scalar(ui, v);
        }
    }
}

fn values_simple(v: &Value) -> bool {
    !v.is_array() && !v.is_object()
}

pub(super) fn chart(ui: &mut Ui, node: &mut Value, s: &mut Settings) {
    if node["type"] != "chart" {
        return;
    }
    ui.collapsing("Chart series", |ui| {
        let Some(slots) = node["value"].as_array_mut() else {
            ui.weak("Bound series are edited in Bindings.");
            return;
        };
        let mut remove = None;
        for (i, slot) in slots.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                widgets::table_heading(ui, format!("Series {}", i + 1));
                // (A series is an object; one written as something else
                // in the source is shown, and set there.)
                if !slot.is_object() {
                    ui.weak("Not a series: see Advanced › JUI source.");
                    if ui.button("Remove series").clicked() {
                        remove = Some(i);
                    }
                    return;
                }
                let before = slot["type"].as_i64().unwrap_or(0);
                let mut kind = before;
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut kind, 0, "Lines");
                    ui.selectable_value(&mut kind, 1, "Columns");
                });
                // Only a choice writes: looking at a series never edits it.
                if kind != before {
                    slot["type"] = json!(kind);
                }
                let mut ignored = None;
                for field in ["legend", "color", "data"] {
                    // (A field the series lacks is shown empty, and made
                    // only when it is given a value.)
                    let mut value = slot.get(field).cloned().unwrap_or(Value::Null);
                    let was = value.clone();
                    ui.push_id(field, |ui| {
                        property(ui, field, "chart_slot", &mut value, s, &mut ignored)
                    });
                    if value != was {
                        slot[field] = value;
                    }
                }
                if ui.button("Remove series").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            slots.remove(i);
        }
        if ui.button("+ Add series").clicked() {
            slots.push(json!({
                "type":0,"legend":"Series","color":{"r":80,"g":180,"b":240,"a":255},
                "data":[0.0,1.0]
            }));
        }
    });
}

pub(super) fn list(ui: &mut Ui, doc: &mut Value, selected: &str, s: &mut Settings) {
    if let Some((parent, i, true)) = array_position(selected)
        && let Some(cell) = doc.pointer_mut(&parent).and_then(|v| v.get_mut(i))
    {
        egui::CollapsingHeader::new("List cell").default_open(true).show(ui, |ui| {
            if let Some(width) = cell.get_mut(1) {
                widgets::field_label(ui, "Cell width");
                scalar(ui, width);
            }
            if let Some(variable) = cell.get(2).and_then(Value::as_bool) {
                let mut variable = variable;
                if ui.checkbox(&mut variable, "Variable width").changed() {
                    cell[2] = variable.into();
                }
            }
        });
    }
    let Some(node) = doc.pointer_mut(selected).filter(|n| n["type"] == "list") else { return };
    egui::CollapsingHeader::new("Row data").default_open(true).show(ui, |ui| {
        let names = row_binds(&node["row_template"]);
        if names.is_empty() {
            ui.weak("Bind a cell's value to edit its rows here.");
            return;
        }
        // Never reinterpret a scalar shared bind as a row array without an explicit choice.
        let mut arrays = Vec::new();
        for name in &names {
            if s.bindings.get(name).is_some_and(|b| b.value.is_array()) {
                arrays.push(name.clone());
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Label::new(name).wrap());
                    if ui.small_button("Use row array").clicked() {
                        let b = s.bindings.entry(name.clone()).or_default();
                        b.value = json!([b.value.clone()]);
                    }
                });
            }
        }
        let count = arrays
            .iter()
            .filter_map(|n| s.bindings.get(n)?.value.as_array().map(Vec::len))
            .max()
            .unwrap_or(0);
        let mut remove = None;
        let mut duplicate = None;
        for row in 0..count {
            egui::CollapsingHeader::new(format!("Row {}", row + 1)).id_salt((selected, row)).show(
                ui,
                |ui| {
                    for name in &arrays {
                        ui.push_id(name, |ui| {
                            widgets::field_label(ui, name);
                            let Some(a) =
                                s.bindings.get_mut(name).and_then(|b| b.value.as_array_mut())
                            else {
                                return;
                            };
                            if let Some(v) = a.get_mut(row) {
                                value(ui, v, 0);
                            } else if ui.small_button("Fill missing cell").clicked() {
                                a.resize(row + 1, Value::Null);
                            }
                        });
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("Duplicate row").clicked() {
                            duplicate = Some(row);
                        }
                        if ui.small_button("Delete row").clicked() {
                            remove = Some(row);
                        }
                    });
                },
            );
        }
        let add = ui.add_enabled(!arrays.is_empty(), egui::Button::new("+ Add row")).clicked();
        if add || remove.is_some() || duplicate.is_some() {
            for name in arrays {
                let Some(a) = s.bindings.get_mut(&name).and_then(|b| b.value.as_array_mut()) else {
                    continue;
                };
                a.resize(count, Value::Null);
                if let Some(row) = remove {
                    a.remove(row);
                } else if let Some(row) = duplicate {
                    a.insert(row + 1, a[row].clone());
                } else {
                    a.push(a.last().cloned().unwrap_or(Value::Null));
                }
            }
            let rows =
                count + usize::from(add || duplicate.is_some()) - usize::from(remove.is_some());
            if node["row_count"].is_number() {
                node["row_count"] = json!(rows);
            } else if let Some(bind) = node["row_count"]["bind"].as_str()
                && let Some(b) = s.bindings.get_mut(bind).filter(|b| b.value.is_u64())
            {
                // A row count bound to a number follows the rows too.
                b.value = json!(rows);
            }
        }
    });
}

/// The binds a list's rows give values to: those of its cells, and of draw
/// layers only where they repeat per row (a polyline's bound points without
/// it are one array, not rows).
fn row_binds(template: &Value) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    fn visit(v: &Value, names: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(name) = o.get("bind").and_then(Value::as_str) {
                    names.insert(name.to_owned());
                }
                for (key, item) in o {
                    if key == "draw_list" {
                        for layer in item.as_array().into_iter().flatten() {
                            if layer["arrayBinds"] == true {
                                names.extend(mg_nui::bind_names(layer));
                            }
                        }
                    } else {
                        visit(item, names);
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|v| visit(v, names)),
            _ => {}
        }
    }
    visit(template, &mut names);
    names
}

pub(super) fn binding_options(ui: &mut Ui, v: &mut Value) {
    if v.get("bind").is_none() {
        return;
    }
    egui::CollapsingHeader::new("Bind formatting").show(ui, |ui| {
        // The NUI_NUMBER_FLAG_* and NUI_TEXT_FLAG_* bits of nw_inc_nui.
        for (key, bit, label) in [
            ("number_flags", 1, "Numbers in hexadecimal"),
            ("text_flags", 1, "Text in lowercase"),
            ("text_flags", 2, "Text in uppercase"),
        ] {
            let flags = v[key].as_i64().unwrap_or(0);
            let mut on = flags & bit != 0;
            if ui.checkbox(&mut on, label).changed() {
                v[key] = json!(if on { flags | bit } else { flags & !bit });
            }
        }
        ui.horizontal_wrapped(|ui| {
            widgets::field_label(ui, "Decimal places");
            let places = v["number_precision"].as_i64().unwrap_or(0);
            let set = widgets::drag_number(ui, places, |d| {
                d.range(0..=i64::from(i32::MAX)).update_while_editing(false)
            });
            if let Some(places) = set {
                v["number_precision"] = places.into();
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_data_skips_a_draw_layers_flat_points_unless_they_repeat_per_row() {
        let cell = |array_binds: bool| {
            json!([[{"type":"spacer","draw_list":[{"type":0,"arrayBinds":array_binds,
                "points":{"bind":"line"}}]}, 0.0, true],
                [{"type":"label","value":{"bind":"names"}}, 0.0, true]])
        };
        let names = |b| row_binds(&cell(b)).into_iter().collect::<Vec<_>>();
        assert_eq!(names(false), ["names"]);
        assert_eq!(names(true), ["line", "names"]);
    }
}
