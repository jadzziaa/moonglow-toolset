//! Binds in the Design page's properties: which property of the control
//! selected is bound, to a bind there is or a new one (a draft until it is
//! made; only binds whose value the property can take are offered), and the
//! handler of a bind's changes ([`watch`]).
use super::*;

#[derive(Clone, Default)]
struct Draft {
    name: String,
    value: Value,
    /// Whether the property takes the value, as last asked of the
    /// validator, and the value it was asked for ([`stamp`]): asked again
    /// when the value changes, and before the bind is made, not every
    /// frame (a copy of the window and its settings, validated, each).
    fits: Option<(u64, bool)>,
}

/// A value as a number that changes when it does, read where it is.
fn stamp(v: &Value) -> u64 {
    use std::hash::{Hash, Hasher};
    fn feed(v: &Value, h: &mut impl Hasher) {
        match v {
            Value::Null => 0u8.hash(h),
            Value::Bool(b) => (1u8, b).hash(h),
            Value::Number(n) => (2u8, n.as_f64().map(f64::to_bits), n.as_i64()).hash(h),
            Value::String(s) => (3u8, s).hash(h),
            Value::Array(a) => {
                (4u8, a.len()).hash(h);
                a.iter().for_each(|v| feed(v, h));
            }
            Value::Object(o) => {
                (5u8, o.len()).hash(h);
                for (k, v) in o {
                    k.hash(h);
                    feed(v, h);
                }
            }
        }
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    feed(v, &mut h);
    h.finish()
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
        .filter(|k| bindable(&ty, k) && (!node[*k].is_null() || has_property_default(&ty, k)))
        .cloned()
        .collect();
    if keys.is_empty() {
        return;
    }
    // (A bound property's chip clicked: its property shown here, the
    // section opened and brought into sight, M4's U19.)
    let asked_id = form::chip_asked(ui.ctx());
    let asked = ui.ctx().data_mut(|d| d.remove_temp::<String>(asked_id));
    if let Some(key) = asked.as_ref().filter(|k| keys.contains(k)) {
        state.binding_property = key.clone();
    }
    if !keys.contains(&state.binding_property) {
        state.binding_property = ["value", "label", "title"]
            .into_iter()
            .find(|k| keys.iter().any(|v| v == k))
            .unwrap_or(&keys[0])
            .into();
    }
    let tip = "A bound property takes its value from a bind, which a script can change";
    let open = asked.is_some().then_some(true);
    let section = form::section(
        ui,
        "nui-property-binding",
        "Property Binding",
        true,
        open,
        Some(tip),
        |ui| {
            {
                form::row(ui, "Property", None, |ui, lead| {
                    widgets::dropdown("bind-property")
                        .selected_text(label(&state.binding_property, &ty))
                        .width(ui.available_width().max(0.0))
                        .show(ui, &mut state.binding_property, &keys, |key| label(key, &ty))
                        .labelled_by(lead.id);
                });
                let key = state.binding_property.clone();
                let Some(bound) =
                    doc.pointer(&path).map(|n| n[&key]["bind"].as_str().map(str::to_owned))
                else {
                    return;
                };
                let rows = row_count(doc, &path, settings);
                ui.push_id((&path, &key), |ui| {
                let draft_id = ui.make_persistent_id("property-bind-draft");
                // (Taken out for the frame and put back, not copied: one
                // value a row, up to 10,000.)
                let mut draft = ui.ctx().data_mut(|d| d.remove_temp::<Draft>(draft_id));
                let mut chosen = bound.clone().unwrap_or_default();
                // Checked only while the list is used (open, or with the keys).
                let choices = || -> Vec<String> {
                    (settings.bindings.iter())
                        .filter(|(_, b)| compatible(doc, &path, &key, settings, &b.value))
                        .map(|(n, _)| n.clone())
                        .collect()
                };
                let options = || std::iter::once(String::new()).chain(choices()).collect();
                // Source: a compatible bind or Constant, and beside it the
                // board's plus that makes a new bind of the property's value.
                let can_new = draft.is_none() && rows.is_none_or(|n| n <= 10000);
                let tip = "A compatible bind, or Constant (no bind)";
                let new_bind = form::row(ui, "Source", Some(tip), |ui, lead| {
                    // (The dropdown the room the plus leaves, the plus at
                    // the row's right.)
                    let plus =
                        mg_uikit::IconButton::new(mg_uikit::Icon::Plus, "New Bind for a Property")
                            .icon_only(true)
                            .framed(true);
                    let room = ui.spacing().interact_size.y.max(mg_uikit::metrics::ICON)
                        + ui.spacing().item_spacing.x;
                    // (A long bind's name cut short, not the row widened.)
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    let size = egui::vec2(
                        (ui.available_width() - room).max(0.0),
                        ui.spacing().interact_size.y,
                    );
                    ui.allocate_ui(size, |ui| {
                        widgets::dropdown("property-bind-source")
                            .selected_text(bound.as_deref().unwrap_or("Constant (no bind)"))
                            .width(ui.available_width())
                            .show_with(ui, &mut chosen, options, |ui, chosen| {
                                let choices = choices();
                                ui.selectable_value(chosen, String::new(), "Constant (no bind)");
                                for name in &choices {
                                    ui.selectable_value(chosen, name.clone(), name);
                                }
                                if choices.is_empty() {
                                    ui.weak(
                                        "No bind holds a value of this kind: make one with the plus.",
                                    );
                                }
                            })
                            .labelled_by(lead.id);
                    });
                    ui.add_enabled(can_new, plus)
                        .on_hover_text("New Bind for a Property: named and valued from the control")
                        .clicked()
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
                        *target = if rows.is_some() {
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
                if new_bind {
                    let current = doc.pointer(&path).map_or(&Value::Null, |n| &n[&key]);
                    let source = resolved(current, settings).clone();
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
                    draft = Some(Draft { name, value: initial, fits: None });
                }
                let mut finish = false;
                if let Some(draft) = draft.as_mut() {
                    let mut open = true;
                    let window = widgets::dialog(ui.ctx(), "New Bind for a Property", &mut open)
                        .collapsible(false)
                        .resizable(false);
                    let footed = window.show_foot(ui.ctx(), |ui| {
                        let heading = widgets::field_label(ui, "Bind name");
                        let field =
                            ui.add(mg_uikit::text_field(&mut draft.name)).labelled_by(heading.id);
                        widgets::autofocus(ui, &field);
                        widgets::field_label(ui, "Initial value");
                        if rows.is_some() {
                            ui.weak("One value per list row.");
                        }
                        fields::value(ui, &mut draft.value, 0);
                        // (Its name by the rule New Bind and Rename have.)
                        let named = workflow::new_bind_name(&draft.name, &settings.bindings);
                        // (A frame with no input changes no field: the
                        // value, one a row, is not stamped again then.)
                        let still = ui.input(|i| i.events.is_empty());
                        let (now, fits) = match draft.fits {
                            Some(asked) if still => asked,
                            asked => {
                                let now = stamp(&draft.value);
                                match asked {
                                    Some((at, fits)) if at == now => (now, fits),
                                    _ => (
                                        now,
                                        compatible(doc, &path, &key, settings, &draft.value),
                                    ),
                                }
                            }
                        };
                        draft.fits = Some((now, fits));
                        let error = match named {
                            Err(why) => Some(why),
                            Ok(_) if !fits => {
                                Some("The property doesn't take a value of this kind.")
                            }
                            Ok(_) => None,
                        };
                        let name = named.unwrap_or_default();
                        if let Some(error) = error {
                            ui.weak(error);
                        }
                        let foot = widgets::Foot::new()
                            .main("Create", error.is_none())
                            .tip("Makes the bind and binds the property to it")
                            .cancel();
                        (name.to_string(), foot)
                    });
                    if let Some(footed) = footed {
                        if footed.pressed.main()
                            && let Some(name) = footed.inner
                            // (Asked once more as it is made: the window
                            // may have changed under the open dialog.)
                            && compatible(doc, &path, &key, settings, &draft.value)
                        {
                            settings.bindings.insert(
                                name.as_str().into(),
                                Binding { value: draft.value.clone(), ..Default::default() },
                            );
                            if let Some(node) = doc.pointer_mut(&path) {
                                node[&key] = json!({"bind":name});
                            }
                            finish = true;
                        }
                        if footed.pressed.cancelled() {
                            finish = true;
                        }
                    }
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
                        form::label(
                            ui,
                            "Initial value",
                            Some("Shared by the controls that use this bind"),
                        );
                        fields::value(ui, &mut binding.value, 0);
                    } else {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            "There is no such bind: make it, or choose another above.",
                        );
                    }
                    // (Its Bind Formatting, under its row before M4's U19,
                    // where its chip is now.)
                    if let Some(bound) = doc.pointer_mut(&path).map(|n| &mut n[&key]) {
                        fields::binding_options(ui, bound);
                    }
                }
            });
            }
        },
    );
    if asked.is_some() {
        section.scroll_to_me(Some(egui::Align::TOP));
    }
}

pub(super) fn watch(ui: &mut Ui, name: &str, doc: &Value, s: &mut Settings, state: &mut State) {
    egui::CollapsingHeader::new("When This Value Changes")
        .icon(mg_uikit::collapsing_icon)
        .default_open(true)
        .show(ui, |ui| {
            events::editor_reading(ui, doc, s, state, Some(events::Target::Bind(name.into())));
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
