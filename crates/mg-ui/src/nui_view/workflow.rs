//! Views and event routes are authoring metadata lowered to stock NUI calls.
use super::*;

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod tests;

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
    fn new() -> Self {
        let kind = BindType::Text;
        Self { name: String::new(), kind, value: kind.value(), rows: false }
    }
    fn error(
        &self,
        bindings: &std::collections::BTreeMap<String, Binding>,
    ) -> Option<&'static str> {
        let name = self.name.trim();
        if name.is_empty() {
            Some("Enter a bind name.")
        } else if !mg_nui::plain_name(name) {
            Some(
                "The name can't end with + or hold a line break or tab: the event script couldn't name it.",
            )
        } else if bindings.contains_key(name) {
            Some("This bind already exists. Choose it above or use another name.")
        } else {
            None
        }
    }
}

/// A new named, typed value: its draft stays in egui state until **Create**,
/// so one Undo takes it back.
pub(super) fn create_bind(
    ui: &mut Ui,
    bindings: &mut std::collections::BTreeMap<String, Binding>,
) -> Option<String> {
    let mut created = None;
    ui.push_id("create-bind", |ui| {
        let id = ui.make_persistent_id("create-bind-draft");
        let mut draft = ui.ctx().data_mut(|d| d.get_temp::<BindDraft>(id));
        let opened = draft.is_none() && ui.button("+ Create bind").clicked();
        if opened {
            draft = Some(BindDraft::new());
        }
        let mut finish = false;
        if let Some(draft) = draft.as_mut() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong("New bind");
                let name = ui.label("Bind name");
                let name_edit = ui.add(egui::TextEdit::singleline(&mut draft.name).hint_text("e.g. is_open").desired_width(ui.available_width().clamp(0.0, 220.0))).labelled_by(name.id);
                if opened { name_edit.request_focus(); }
                let before = draft.kind;
                egui::ComboBox::from_label("Type").selected_text(draft.kind.label()).show_ui(ui, |ui| {
                    for kind in BindType::ALL {
                        ui.selectable_value(&mut draft.kind, kind, kind.label());
                    }
                });
                if before != draft.kind { draft.value = draft.kind.value(); }
                ui.label("Initial value");
                fields::value(ui, &mut draft.value, 0);
                ui.checkbox(&mut draft.rows, "List row array").on_hover_text("Create a value per list row, starting with one row. Edit more rows in Bindings or Row data.");
                if let Some(error) = draft.error(bindings) { ui.weak(error); }
                ui.horizontal(|ui| {
                    if ui.add_enabled(draft.error(bindings).is_none(), egui::Button::new("Create")).clicked() {
                        let name = draft.name.trim().to_owned();
                        let value = if draft.rows { json!([draft.value.clone()]) } else { draft.value.clone() };
                        bindings.insert(name.clone(), Binding { value, ..Default::default() });
                        created = Some(name);
                        finish = true;
                    }
                    if ui.button("Cancel").clicked() { finish = true; }
                });
            });
        }
        ui.ctx().data_mut(|d| if finish { d.remove::<BindDraft>(id); } else if let Some(draft) = draft { d.insert_temp(id, draft); });
    });
    created
}

pub(super) fn ui(
    ui: &mut Ui,
    doc: &mut Value,
    s: &mut Settings,
    state: &mut State,
    window: &str,
    script: Option<&str>,
) {
    egui::ScrollArea::vertical().id_salt("nui-workflow").show(ui, |ui| {
        ui.heading("Swap layout variants");
        ui.label("Add a Swap layout in Design, then create and edit its variants in Properties or Layers.");
        let (mut remove, mut rename) = (None, None);
        for name in s.views.keys() {
            ui.horizontal(|ui| {
                ui.strong(name);
                let target = layouts::target_for(s, name);
                ui.weak(match target.as_deref() {
                    Some("_window_") => "Whole window".into(),
                    Some(id) => format!("Swap layout · {id}"),
                    None => "No target assigned".into(),
                });
                if ui.button("Edit variant").clicked() {
                    state.layout_target = target.unwrap_or_default();
                    layouts::edit(state, Some(name.clone()));
                }
                // The handlers showing it, and its function's calls, follow.
                ui.menu_button("Rename…", |ui| {
                    let id = ui.make_persistent_id(("rename-variant", name));
                    let mut to: String = ui.ctx().data_mut(|d| d.get_temp(id)).unwrap_or_else(|| name.clone());
                    let label = ui.label("Name");
                    ui.text_edit_singleline(&mut to).labelled_by(label.id);
                    let taken = to != *name && s.views.contains_key(&to);
                    let ok = to != *name && !to.is_empty() && mg_nui::plain_name(&to) && !taken;
                    if ui
                        .add_enabled(ok, egui::Button::new("Rename"))
                        .on_disabled_hover_text(if taken { "Another variant has that name" } else { "Type a new name: no spaces at either end" })
                        .clicked()
                    {
                        rename = Some((name.clone(), to.clone()));
                        ui.close();
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(id, to));
                });
                // A handler that shows it calls its function: removing it would break the build.
                let call = format!("{}(", mg_nui::variant_function(window, name));
                let shown = script.is_some_and(|t| t.lines().any(|l| !l.trim_start().starts_with("//") && l.contains(&call)))
                    || s.actions.iter().any(|r| matches!(&r.action, mg_nui::Action::View{view,..} if view==name));
                if ui.add_enabled(!shown, egui::Button::new("Remove"))
                    .on_disabled_hover_text("A handler shows it: take that call out of the event script first")
                    .clicked() { remove=Some(name.clone()); }
            });
        }
        if let Some(name) = remove {
            s.views.remove(&name);
            if let Some(targets) = s.extra.get_mut("view_targets").and_then(Value::as_object_mut) {
                targets.remove(&name);
            }
        }
        if let Some((from, to)) = rename
            && let Some(view) = s.views.remove(&from)
        {
            s.views.insert(to.clone(), view);
            if let Some(targets) = s.extra.get_mut("view_targets").and_then(Value::as_object_mut)
                && let Some(target) = targets.remove(&from)
            {
                targets.insert(to.clone(), target);
            }
            for route in &mut s.actions {
                if let mg_nui::Action::View { view, .. } = &mut route.action
                    && *view == from
                {
                    *view = to.clone();
                }
            }
            if state.edit_view.as_ref() == Some(&from) {
                state.edit_view = Some(to);
            }
        }
        ui.separator();
        events::editor(ui, doc, s, state, None);
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
