//! Views and event routes are authoring metadata lowered to stock NUI calls.
use super::*;

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod tests;

fn choose(ui: &mut Ui, label: &str, value: &mut String, choices: &[String]) -> bool {
    let before = value.clone();
    ui.vertical(|ui| {
        ui.label(label);
        let selected = if value.is_empty() {
            "Choose…"
        } else if label == "Target group" && value == "_window_" {
            "Whole window"
        } else {
            value.as_str()
        }
        .to_owned();
        ui.add_enabled_ui(!choices.is_empty(), |ui| {
            egui::ComboBox::from_id_salt(label).selected_text(selected).show_ui(ui, |ui| {
                for item in choices {
                    let text = if label == "Target group" && item == "_window_" {
                        "Whole window"
                    } else {
                        item.as_str()
                    };
                    ui.selectable_value(value, item.clone(), text);
                }
            });
        });
    });
    before != *value
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BindType {
    Text,
    Integer,
    Decimal,
    Boolean,
    Color,
    Point,
    Rectangle,
}

impl BindType {
    const ALL: [Self; 7] = [
        Self::Text,
        Self::Integer,
        Self::Decimal,
        Self::Boolean,
        Self::Color,
        Self::Point,
        Self::Rectangle,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Integer => "Integer",
            Self::Decimal => "Decimal",
            Self::Boolean => "Boolean",
            Self::Color => "Color",
            Self::Point => "Point",
            Self::Rectangle => "Rectangle",
        }
    }
    fn value(self) -> Value {
        match self {
            Self::Text => json!(""),
            Self::Integer => json!(0),
            Self::Decimal => json!(0.0),
            Self::Boolean => json!(false),
            Self::Color => json!({"r":255,"g":255,"b":255,"a":255}),
            Self::Point => json!({"x":0.0,"y":0.0}),
            Self::Rectangle => json!({"x":0.0,"y":0.0,"w":100.0,"h":30.0}),
        }
    }
}

#[derive(Clone)]
struct BindDraft {
    name: String,
    kind: BindType,
    value: Value,
    rows: bool,
}

impl BindDraft {
    fn new(boolean: bool) -> Self {
        let kind = if boolean { BindType::Boolean } else { BindType::Text };
        Self { name: String::new(), kind, value: kind.value(), rows: false }
    }
    fn error(
        &self,
        bindings: &std::collections::BTreeMap<String, Binding>,
    ) -> Option<&'static str> {
        let name = self.name.trim();
        if name.is_empty() {
            Some("Enter a bind name.")
        } else if name.contains('\0') {
            Some("The name cannot contain NUL characters.")
        } else if bindings.contains_key(name) {
            Some("This bind already exists. Choose it above or use another name.")
        } else {
            None
        }
    }
}

fn boolean_binding(binding: &Binding) -> bool {
    binding.value.is_boolean()
        || binding.value.as_array().is_some_and(|a| a.iter().all(Value::is_boolean))
}

/// Draft edits remain in egui state. Creating and selecting the binding happen
/// in one frame, so the containing editor records one workspace Undo command.
pub(super) fn bind_select(
    ui: &mut Ui,
    label: &str,
    selected: &mut String,
    bindings: &mut std::collections::BTreeMap<String, Binding>,
    boolean: bool,
    excluded: Option<&str>,
) -> bool {
    let before = selected.clone();
    let choices: Vec<_> = bindings
        .iter()
        .filter(|(name, binding)| {
            excluded != Some(name.as_str()) && (!boolean || boolean_binding(binding))
        })
        .map(|(name, _)| name.clone())
        .collect();
    choose(ui, label, selected, &choices);
    if choices.is_empty() {
        ui.weak(if bindings.is_empty() {
            "No binds yet. Create a named value for this action."
        } else if boolean {
            "No available boolean binds. Toggle needs a boolean value."
        } else {
            "Choose another bind: a watch action cannot write its own trigger."
        });
    } else if !selected.is_empty() && !choices.contains(selected) {
        ui.colored_label(
            egui::Color32::LIGHT_RED,
            "Choose an available bind to replace this missing or incompatible target.",
        );
    }
    ui.push_id(label, |ui| {
        let id = ui.make_persistent_id("create-bind-draft");
        let mut draft = ui.ctx().data_mut(|d| d.get_temp::<BindDraft>(id));
        let opened = draft.is_none() && ui.small_button("+ Create bind").clicked();
        if opened {
            draft = Some(BindDraft::new(boolean));
        }
        let mut finish = false;
        if let Some(draft) = draft.as_mut() {
            // Changing the action from Set to Toggle must not reuse a text draft.
            if boolean && draft.kind != BindType::Boolean { *draft = BindDraft::new(true); }
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong("New bind");
                let name = ui.label("Bind name");
                let name_edit = ui.add(egui::TextEdit::singleline(&mut draft.name).hint_text("e.g. is_open").desired_width(ui.available_width().clamp(0.0, 220.0))).labelled_by(name.id);
                if opened { name_edit.request_focus(); }
                let before = draft.kind;
                egui::ComboBox::from_label("Type").selected_text(draft.kind.label()).show_ui(ui, |ui| {
                    for kind in BindType::ALL {
                        if !boolean || kind == BindType::Boolean { ui.selectable_value(&mut draft.kind, kind, kind.label()); }
                    }
                });
                if before != draft.kind { draft.value = draft.kind.value(); }
                ui.label("Initial value");
                fields::value(ui, &mut draft.value, 0);
                ui.checkbox(&mut draft.rows, "List row array").on_hover_text("Create a value per list row, starting with one row. Edit more rows in Bindings or Row data.");
                if let Some(error) = draft.error(bindings) { ui.weak(error); }
                ui.horizontal(|ui| {
                    if ui.add_enabled(draft.error(bindings).is_none(), egui::Button::new("Create & select")).clicked() {
                        let name = draft.name.trim().to_owned();
                        let value = if draft.rows { json!([draft.value.clone()]) } else { draft.value.clone() };
                        bindings.insert(name.clone(), Binding { value, ..Default::default() });
                        *selected = name;
                        finish = true;
                    }
                    if ui.button("Cancel").clicked() { finish = true; }
                });
            });
        }
        ui.ctx().data_mut(|d| if finish { d.remove::<BindDraft>(id); } else if let Some(draft) = draft { d.insert_temp(id, draft); });
    });
    before != *selected
}

pub(super) fn ui(ui: &mut Ui, doc: &mut Value, s: &mut Settings, state: &mut State) {
    egui::ScrollArea::vertical().id_salt("nui-workflow").show(ui, |ui| {
        ui.heading("Swap layout variants");
        ui.label("Add a Swap layout in Design, then create and edit its variants in Properties or Layers.");
        let mut remove = None;
        for name in s.views.keys() {
            ui.horizontal(|ui| {
                ui.strong(name);
                let target = layouts::target_for(s, name);
                ui.weak(match target.as_deref() {
                    Some("_window_") => "Whole window".into(),
                    Some(id) => format!("Swap layout · {id}"),
                    None => "No target assigned".into(),
                });
                if ui.button("Edit layout").clicked() {
                    state.layout_target = target.unwrap_or_default();
                    layouts::edit(state, Some(name.clone()));
                }
                let used = s.actions.iter().any(|r| matches!(&r.action, mg_nui::Action::View{view,..} if view==name));
                if ui.add_enabled(!used, egui::Button::new("Remove")).on_hover_text("Remove referencing actions first").clicked() { remove=Some(name.clone()); }
            });
        }
        if let Some(name) = remove { s.views.remove(&name); }
        ui.separator();
        events::editor(ui, doc, s, state, None);
        ui.separator();
        ui.heading("Window identity");
        let mut custom=s.window_id.is_some();
        if ui.checkbox(&mut custom,"Custom window ID").changed(){s.window_id=custom.then(||"my_window".into());}
        if let Some(id)=&mut s.window_id{ui.text_edit_singleline(id);}
        ui.weak("The resource name remains unchanged. A custom ID controls NuiFindWindow/NuiCreate identity.");
    });
}

pub(super) fn presets(ui: &mut Ui, doc: &mut Value, s: &mut Settings, state: &mut State) {
    ui.collapsing("Component presets", |ui| {
        ui.text_edit_singleline(&mut state.preset_name);
        let selected = doc.pointer(&state.selected).filter(|n| n["type"].is_string()).cloned();
        if ui
            .add_enabled(
                selected.is_some() && !state.preset_name.trim().is_empty(),
                egui::Button::new("Save selected as preset"),
            )
            .clicked()
        {
            let presets = s.extra.entry("presets".into()).or_insert_with(|| json!({}));
            if let Some(obj) = presets.as_object_mut() {
                obj.insert(state.preset_name.clone(), selected.unwrap());
                state.preset_name.clear();
            }
        }
        let presets =
            s.extra.get("presets").and_then(Value::as_object).cloned().unwrap_or_default();
        for (name, mut node) in presets {
            ui.horizontal(|ui| {
                if ui.button(format!("Insert {name}")).clicked()
                    && let Some(parent) = design::insertion_parent(doc, &state.selected)
                {
                    assign_ids(&mut node, doc);
                    let target = doc.pointer_mut(&parent).unwrap();
                    design::fit_into(target, &mut node);
                    if target["type"] == "list" {
                        if let Some(a) = target["row_template"].as_array_mut() {
                            a.push(json!([node, 150.0, true]));
                        }
                    } else if let Some(a) = target["children"].as_array_mut() {
                        a.push(node);
                    }
                }
                if ui.small_button("Remove").clicked() {
                    s.extra
                        .get_mut("presets")
                        .and_then(Value::as_object_mut)
                        .map(|o| o.remove(&name));
                }
            });
        }
    });
}

pub(super) fn images(
    ui: &mut Ui,
    node: &mut Value,
    s: &mut Settings,
    state: &mut State,
    assets: &mut skin::Assets,
) {
    let field = match node["type"].as_str() {
        Some("image") => "value",
        Some("button_image") => "label",
        _ => return,
    };
    ui.collapsing("Choose game image", |ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.asset_search).hint_text("Find image resource…"),
        );
        let query = state.asset_search.to_lowercase();
        let names: Vec<_> =
            assets.catalog.iter().filter(|n| n.contains(&query)).take(24).cloned().collect();
        assets.request_images(ui.ctx(), names.iter().cloned().collect());
        ui.small("First 24 matches from the current module and content stack");
        let row_array = node[field]["bind"]
            .as_str()
            .and_then(|n| s.bindings.get(n))
            .is_some_and(|b| b.value.is_array());
        if row_array {
            ui.weak("Choose individual row images in the list's Row data.");
        }
        if !node["image_region"].is_object() && ui.button("Add crop region").clicked() {
            let name = preview::string(&node[field], s, None);
            let size = assets.picture(&name).map_or(egui::vec2(64.0, 64.0), |p| p.size);
            node["image_region"] = json!({"x":0.0,"y":0.0,"w":size.x,"h":size.y});
        }
        egui::ScrollArea::vertical().id_salt("nui-images").max_height(230.0).show(ui, |ui| {
            for name in names {
                ui.horizontal(|ui| {
                    if let Some(pic) = assets.picture(&name) {
                        ui.add(
                            egui::Image::new(&pic.texture)
                                .fit_to_exact_size(egui::vec2(40.0, 32.0)),
                        );
                    }
                    if ui.add_enabled(!row_array, egui::Button::new(&name)).clicked() {
                        if let Some(bind) = node[field]["bind"].as_str() {
                            if let Some(b) = s.bindings.get_mut(bind) {
                                // Row arrays are edited in Row data, never replaced with a scalar.
                                if !b.value.is_array() {
                                    b.value = json!(name);
                                }
                            }
                        } else {
                            node[field] = json!(name);
                        }
                    }
                });
            }
        });
    });
}
