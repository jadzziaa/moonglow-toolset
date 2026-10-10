//! Structured values stay native JSON. UI helpers never add fields to runtime JUI.
use super::*;

/// Keep option labels and values editable at the inspector's actual width.
/// A text field using the whole row before a DragValue made the vertical scroll
/// area grow sideways and displaced the right panel underneath the canvas.
pub(super) fn options(ui: &mut Ui, elements: &mut Vec<Value>, combo: bool) {
    let mut remove = None;
    for (i, element) in elements.iter_mut().enumerate() {
        ui.push_id(i, |ui| {
            let entry = if element.is_array() { element.get_mut(0) } else { Some(&mut *element) };
            if let Some(Value::String(text)) = entry {
                ui.add(
                    egui::TextEdit::singleline(text)
                        .desired_width(ui.available_width())
                        .hint_text("Option label"),
                )
                .widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::TextEdit,
                        true,
                        format!("Option {} label", i + 1),
                    )
                });
            }
            ui.horizontal_wrapped(|ui| {
                if let Some(n) = element.get_mut(1) {
                    ui.weak("Value");
                    scalar(ui, n);
                }
                if ui.small_button("Remove").on_hover_text("Remove option").clicked() {
                    remove = Some(i);
                }
            });
            ui.add_space(4.0);
        });
    }
    if let Some(i) = remove {
        elements.remove(i);
    }
    let next = next_option_value(elements);
    if ui.add_enabled(!combo || next.is_some(), egui::Button::new("+ Add option").small()).clicked()
    {
        if combo {
            elements.push(json!(["Option", next.unwrap()]));
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
                        ui.label(key);
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
        _ => scalar(ui, v),
    }
}

fn values_simple(v: &Value) -> bool {
    !v.is_array() && !v.is_object()
}

pub(super) fn chart(ui: &mut Ui, node: &mut Value, s: &mut Settings) {
    if node["type"] != "chart" {
        return;
    }
    ui.collapsing("Chart series",|ui| {
        let Some(slots)=node["value"].as_array_mut() else {ui.weak("Edit bound series in Bindings.");return};
        let mut remove=None;
        for (i,slot) in slots.iter_mut().enumerate() {
            ui.push_id(i,|ui| {
                ui.strong(format!("Series {}",i+1));
                let before=slot["type"].as_i64().unwrap_or(0);
                let mut kind=before;
                ui.horizontal(|ui|{ui.selectable_value(&mut kind,0,"Lines");ui.selectable_value(&mut kind,1,"Columns");});
                // Only a choice writes: looking at a series never edits it.
                if kind!=before {slot["type"]=json!(kind);}
                let mut ignored=None;
                for field in ["legend","color","data"]{ui.push_id(field,|ui|property(ui,field,"chart_slot",&mut slot[field],s,&mut ignored));}
                if ui.button("Remove series").clicked(){remove=Some(i);}
            });
        }
        if let Some(i)=remove {slots.remove(i);}
        if ui.button("+ Add series").clicked(){slots.push(json!({"type":0,"legend":"Series","color":{"r":80,"g":180,"b":240,"a":255},"data":[0.0,1.0]}));}
    });
}

pub(super) fn list(ui: &mut Ui, doc: &mut Value, selected: &str, s: &mut Settings) {
    if let Some((parent, i, true)) = array_position(selected)
        && let Some(cell) = doc.pointer_mut(&parent).and_then(|v| v.get_mut(i))
    {
        egui::CollapsingHeader::new("List cell").default_open(true).show(ui, |ui| {
            ui.label("Cell width");
            scalar(ui, &mut cell[1]);
            if let Some(variable) = cell[2].as_bool() {
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
            .filter_map(|n| s.bindings[n].value.as_array().map(Vec::len))
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
                            ui.label(name);
                            let a = s.bindings.get_mut(name).unwrap().value.as_array_mut().unwrap();
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
                let a = s.bindings.get_mut(&name).unwrap().value.as_array_mut().unwrap();
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
            ui.label("Decimal places");
            let mut n = v["number_precision"].as_i64().unwrap_or(0);
            if ui.add(egui::DragValue::new(&mut n).range(0..=i32::MAX)).changed() {
                v["number_precision"] = n.into();
            }
        });
    });
}

#[cfg(test)]
mod row_tests {
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
