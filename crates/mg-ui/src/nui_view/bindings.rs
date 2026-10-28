//! Binds in the Design page's properties: which property of the control
//! selected is bound, to a bind there is or a new one (a draft until it is
//! made; only binds whose value the property can take are offered), and the
//! handler of a bind's changes ([`watch`]).
use super::*;

#[derive(Clone)]
struct Draft {
    name: String,
    value: Value,
}

fn label(key: &str, ty: &str) -> String {
    super::display_label(key, ty)
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
        Settings { handlers: Vec::new(), variants: Default::default(), ..settings.clone() };
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
        ui.weak("A bound property takes its value from a bind, which a script can change.");
        let property = widgets::field_label(ui, "Property");
        widgets::dropdown("bind-property")
            .selected_text(label(&state.binding_property, &ty))
            .width(ui.available_width().max(0.0))
            .show(ui, &mut state.binding_property, &keys, |key| label(key, &ty))
            .labelled_by(property.id);
        let key = state.binding_property.clone();
        let Some(current) = doc.pointer(&path).map(|n| n[&key].clone()) else { return };
        let bound = current["bind"].as_str().map(str::to_owned);
        let rows = row_count(doc, &path, settings);
        ui.push_id((&path, &key), |ui| {
            let draft_id = ui.make_persistent_id("property-bind-draft");
            let mut draft = ui.ctx().data_mut(|d| d.get_temp::<Draft>(draft_id));
            let mut chosen = bound.clone().unwrap_or_default();
            widgets::field_label(ui, "Value from");
            // Checked only while the list is used (open, or with the keys).
            let choices = || -> Vec<String> {
                (settings.bindings.iter())
                    .filter(|(_, b)| compatible(doc, &path, &key, settings, &b.value))
                    .map(|(n, _)| n.clone())
                    .collect()
            };
            let options = || std::iter::once(String::new()).chain(choices()).collect();
            widgets::dropdown("property-bind-source")
                .selected_text(bound.as_deref().unwrap_or("Constant (no bind)"))
                .width(ui.available_width().max(0.0))
                .show_with(ui, &mut chosen, options, |ui, chosen| {
                    let choices = choices();
                    ui.selectable_value(chosen, String::new(), "Constant (no bind)");
                    for name in &choices {
                        ui.selectable_value(chosen, name.clone(), name);
                    }
                    if choices.is_empty() {
                        ui.weak("No bind holds a value of this kind: make one below.");
                    }
                });
            if chosen != bound.clone().unwrap_or_default() {
                let Some(node) = doc.pointer_mut(&path) else { return };
                let target = &mut node[&key];
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
                ui.weak(
                    "For more than 10000 rows, write the bind in Binding JSON and choose it here.",
                );
            }
            if draft.is_none()
                && ui
                    .add_enabled(
                        rows.is_none_or(|n| n <= 10000),
                        egui::Button::new("+ New bind for property"),
                    )
                    .clicked()
            {
                let source = resolved(&current, settings).clone();
                let source = if source.is_null() {
                    property_default(&ty, &key).unwrap_or(json!(""))
                } else {
                    source
                };
                let initial = if let Some(count) = rows.filter(|_| bound.is_none()) {
                    Value::Array(vec![source; count])
                } else {
                    source
                };
                let base = format!(
                    "{}_{}",
                    doc.pointer(&path).and_then(|n| n["id"].as_str()).unwrap_or(&ty),
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
                let mut open = true;
                let window = widgets::dialog(ui.ctx(), "New Bind for a Property", &mut open)
                    .collapsible(false)
                    .resizable(false);
                window.show(ui.ctx(), |ui| {
                    let heading = widgets::field_label(ui, "Bind name");
                    let field =
                        ui.add(egui::TextEdit::singleline(&mut draft.name)).labelled_by(heading.id);
                    widgets::autofocus(ui, &field);
                    widgets::field_label(ui, "Initial value");
                    if rows.is_some() {
                        ui.weak("One value per list row.");
                    }
                    fields::value(ui, &mut draft.value, 0);
                    let name = draft.name.trim();
                    let error = if name.is_empty() || name.contains('\0') {
                        Some("The bind needs a name.")
                    } else if settings.bindings.contains_key(name) {
                        Some("A bind has that name: choose it above, or another name.")
                    } else if !compatible(doc, &path, &key, settings, &draft.value) {
                        Some("The property doesn't take a value of this kind.")
                    } else {
                        None
                    };
                    if let Some(error) = error {
                        ui.weak(error);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(error.is_none(), egui::Button::new("Create"))
                            .on_hover_text("Makes the bind and binds the property to it")
                            .clicked()
                            || (error.is_none() && widgets::enter(ui))
                        {
                            settings.bindings.insert(
                                name.into(),
                                Binding { value: draft.value.clone(), ..Default::default() },
                            );
                            if let Some(node) = doc.pointer_mut(&path) {
                                node[&key] = json!({"bind":name});
                            }
                            finish = true;
                        }
                        if widgets::cancel(ui) {
                            finish = true;
                        }
                    });
                });
                finish |= !open;
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
            if finish {
                ui.ctx().request_repaint();
            }
            if let Some(name) =
                doc.pointer(&path).and_then(|n| n[&key]["bind"].as_str()).map(str::to_owned)
            {
                if let Some(binding) = settings.bindings.get_mut(&name) {
                    widgets::field_label(ui, "Initial value");
                    ui.weak("Shared by the controls that use this bind.");
                    fields::value(ui, &mut binding.value, 0);
                } else {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        "There is no such bind: make it, or choose another above.",
                    );
                }
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
