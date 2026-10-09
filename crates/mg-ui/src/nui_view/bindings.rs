//! Contextual binding workflow on the Design inspector. Drafts never mutate the module.
use super::*;

#[derive(Clone)]
struct Draft {
    name: String,
    value: Value,
}

fn label(key: &str, ty: &str) -> String {
    match (key, ty) {
        ("value", "label" | "text" | "textedit") => "Text".into(),
        ("label", "textedit") => "Placeholder".into(),
        ("disabled_tooltip", _) => "Tooltip when disabled".into(),
        _ => {
            let mut text = key.replace('_', " ");
            if let Some(first) = text.get_mut(..1) {
                first.make_ascii_uppercase();
            }
            text
        }
    }
}

fn row_count(doc: &Value, path: &str, settings: &Settings) -> Option<usize> {
    let (parent, _) = path.rsplit_once("/row_template/")?;
    let count = resolved(&doc.pointer(parent)?["row_count"], settings);
    Some(count.as_array().map_or_else(|| count.as_u64().unwrap_or(1) as usize, Vec::len))
}

/// Ask the production validator about this property's use, including list rows.
/// Other invalid properties in an imported document do not affect the chooser.
fn compatible(doc: &Value, path: &str, key: &str, settings: &Settings, candidate: &Value) -> bool {
    let mut probe = doc.clone();
    let Some(node) = probe.pointer_mut(path) else { return false };
    // Only the property's own diagnostics matter: actions and views would
    // repeat whole-document checks once per candidate bind, every frame.
    let mut settings =
        Settings { actions: Vec::new(), views: Default::default(), ..settings.clone() };
    let mut name = "__creator_bind_probe".to_owned();
    while settings.bindings.contains_key(&name) {
        name.push('_');
    }
    node[key] = json!({"bind":name});
    settings.bindings.insert(name, Binding { value: candidate.clone(), ..Default::default() });
    let property_path = format!("{path}/{key}");
    !mg_nui::validate(&probe, &settings)
        .iter()
        .any(|d| d.severity == Severity::Error && d.path == property_path)
}

pub(super) fn inspector(ui: &mut Ui, doc: &mut Value, settings: &mut Settings, state: &mut State) {
    let path = state.selected.clone();
    let Some(node) = doc.pointer(&path) else { return };
    let ty = node["type"].as_str().unwrap_or("window").to_owned();
    let Some(object) = node.as_object() else { return };
    let keys: Vec<_> = object
        .keys()
        .filter(|k| bindable(&ty, k) && (!node[*k].is_null() || property_default(&ty, k).is_some()))
        .cloned()
        .collect();
    if keys.is_empty() {
        return;
    }
    if !keys.contains(&state.binding_property) {
        state.binding_property = ["value", "label", "title"]
            .into_iter()
            .find(|k| keys.iter().any(|v| v == k))
            .unwrap_or(&keys[0])
            .into();
    }
    egui::CollapsingHeader::new("Property binding").default_open(true).show(ui, |ui| {
        ui.weak("Connect this property to a value that can change in game.");
        egui::ComboBox::from_id_salt("bind-property")
            .selected_text(label(&state.binding_property, &ty))
            .width(ui.available_width().max(0.0))
            .show_ui(ui, |ui| {
                for key in &keys {
                    ui.selectable_value(&mut state.binding_property, key.clone(), label(key, &ty));
                }
            });
        let key = state.binding_property.clone();
        let current = doc.pointer(&path).unwrap()[&key].clone();
        let bound = current["bind"].as_str().map(str::to_owned);
        let rows = row_count(doc, &path, settings);
        ui.push_id((&path, &key), |ui| {
            let draft_id = ui.make_persistent_id("property-bind-draft");
            let mut draft = ui.ctx().data_mut(|d| d.get_temp::<Draft>(draft_id));
            let mut chosen = bound.clone().unwrap_or_default();
            egui::ComboBox::from_id_salt("property-bind-source")
                .selected_text(bound.as_deref().unwrap_or("Constant (no bind)"))
                .width(ui.available_width().max(0.0))
                .show_ui(ui, |ui| {
                    // Checked only while the list is open.
                    let choices: Vec<_> = settings
                        .bindings
                        .iter()
                        .filter(|(_, b)| compatible(doc, &path, &key, settings, &b.value))
                        .map(|(n, _)| n.clone())
                        .collect();
                    ui.selectable_value(&mut chosen, String::new(), "Constant (no bind)");
                    for name in &choices {
                        ui.selectable_value(&mut chosen, name.clone(), name);
                    }
                    if choices.is_empty() {
                        ui.weak("No matching binds. Create one below.");
                    }
                });
            if chosen != bound.clone().unwrap_or_default() {
                let target = &mut doc.pointer_mut(&path).unwrap()[&key];
                if chosen.is_empty() {
                    let value = bound
                        .as_ref()
                        .and_then(|n| settings.bindings.get(n))
                        .map(|b| b.value.clone())
                        .unwrap_or(Value::Null);
                    *target =
                        if rows.is_some() {
                            value.as_array().and_then(|a| a.first()).cloned().unwrap_or_else(|| {
                                property_default(&ty, &key).unwrap_or(Value::Null)
                            })
                        } else {
                            value
                        };
                } else {
                    *target = json!({"bind":chosen});
                }
                draft = None;
            }
            if rows.is_some_and(|n| n > 10000) {
                ui.weak("For more than 10000 rows, prepare the bind in Binding JSON and select it here.");
            }
            if draft.is_none() && ui.add_enabled(rows.is_none_or(|n|n<=10000),egui::Button::new("+ New bind for property")).clicked() {
                let source = resolved(&current, settings).clone();
                let source = if source.is_null() {
                    property_default(&ty, &key).unwrap_or(json!(""))
                } else {
                    source
                };
                let initial = if let Some(count) = rows.filter(|_|bound.is_none()) {
                    Value::Array(vec![source; count])
                } else {
                    source
                };
                let base = format!(
                    "{}_{}",
                    doc.pointer(&path).unwrap()["id"].as_str().unwrap_or(&ty),
                    key
                );
                let mut name = base.clone();
                let mut n = 2;
                while settings.bindings.contains_key(&name) {
                    name = format!("{base}_{n}");
                    n += 1;
                }
                draft = Some(Draft { name, value: initial });
            }
            let mut finish = false;
            if let Some(draft) = draft.as_mut() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.strong("New property bind");
                    let heading = ui.label("Bind name");
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.name)
                            .desired_width(ui.available_width().max(0.0)),
                    )
                    .labelled_by(heading.id);
                    ui.label("Initial value");
                    if rows.is_some() {
                        ui.weak("One value per list row.");
                    }
                    fields::value(ui, &mut draft.value, 0);
                    let name = draft.name.trim();
                    let error = if name.is_empty() || name.contains('\0') {
                        Some("Enter a nonempty bind name.")
                    } else if settings.bindings.contains_key(name) {
                        Some("Name already exists. Choose it above or change the name.")
                    } else if !compatible(doc, &path, &key, settings, &draft.value) {
                        Some("This value does not match the property type.")
                    } else {
                        None
                    };
                    if let Some(error) = error {
                        ui.weak(error);
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(error.is_none(), egui::Button::new("Create & connect"))
                            .clicked()
                        {
                            settings.bindings.insert(
                                name.into(),
                                Binding { value: draft.value.clone(), ..Default::default() },
                            );
                            doc.pointer_mut(&path).unwrap()[&key] = json!({"bind":name});
                            finish = true;
                        }
                        if ui.button("Cancel bind").clicked() {
                            finish = true;
                        }
                    });
                });
            }
            ui.ctx().data_mut(|d| {
                if finish {
                    d.remove::<Draft>(draft_id);
                } else if let Some(draft) = draft {
                    d.insert_temp(draft_id, draft);
                } else {
                    d.remove::<Draft>(draft_id);
                }
            });
            if finish { ui.ctx().request_repaint(); }
            if let Some(name) =
                doc.pointer(&path).unwrap()[&key]["bind"].as_str().map(str::to_owned)
            {
                if let Some(binding) = settings.bindings.get_mut(&name) {
                    ui.weak("Initial value · shared by controls using this bind");
                    fields::value(ui, &mut binding.value, 0);
                } else {
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        "Missing bind. Create or choose a value above.",
                    );
                }
                watch(ui, &name, doc, settings, state);
            }
        });
    });
    ui.separator();
}

pub(super) fn watch(ui: &mut Ui, name: &str, doc: &Value, s: &mut Settings, state: &mut State) {
    egui::CollapsingHeader::new("When this value changes").default_open(true).show(ui, |ui| {
        let mut doc = doc.clone();
        events::editor(ui, &mut doc, s, state, Some(events::Target::Bind(name.into())));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_bind_chooser_uses_native_property_types_and_list_context() {
        let mut doc = mg_nui::window();
        let settings = Settings::default();
        let path = "/root/children/1";
        assert!(compatible(&doc, path, "label", &settings, &json!("Buy")));
        assert!(!compatible(&doc, path, "label", &settings, &json!(["Buy"])));
        assert!(!compatible(&doc, path, "label", &settings, &json!(true)));
        assert!(compatible(&doc, path, "enabled", &settings, &json!(true)));
        assert!(!compatible(&doc, path, "enabled", &settings, &json!("yes")));
        doc["root"] = mg_nui::template("list");
        doc["root"]["row_count"] = json!(3);
        doc["root"]["row_template"] = json!([[mg_nui::template("button"), 150.0, true]]);
        let path = "/root/row_template/0/0";
        assert_eq!(row_count(&doc, path, &settings), Some(3));
        assert!(compatible(&doc, path, "label", &settings, &json!(["Buy", "Sell", "Exit"])));
        assert!(!compatible(&doc, path, "label", &settings, &json!("Buy")));
        assert!(!compatible(&doc, path, "label", &settings, &json!([true, false, true])));
    }
}
