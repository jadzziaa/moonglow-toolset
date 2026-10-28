//! The Events & variants page ([`ui`]): the window's handlers and its swap
//! layout variants in one list. Also the window that makes a bind
//! ([`create_bind`]) and the pictures a window names ([`images()`]).
use super::*;

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
                "The name can't hold a line break, a tab or another control character: the \
                 event script couldn't name it.",
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
        if ui.button("+ Create bind").clicked() && draft.is_none() {
            draft = Some(BindDraft::new());
        }
        let mut finish = false;
        if let Some(draft) = draft.as_mut() {
            let mut open = true;
            let window = widgets::dialog(ui.ctx(), "New Bind", &mut open)
                .collapsible(false)
                .resizable(false);
            window.show(ui.ctx(), |ui| {
                let name = widgets::field_label(ui, "Bind name");
                let name_edit = ui
                    .add(egui::TextEdit::singleline(&mut draft.name).hint_text("e.g. is_open"))
                    .labelled_by(name.id);
                widgets::autofocus(ui, &name_edit);
                let before = draft.kind;
                let kind = widgets::field_label(ui, "Type");
                widgets::dropdown("bind-type")
                    .selected_text(draft.kind.label())
                    .show(ui, &mut draft.kind, &BindType::ALL, |k| k.label())
                    .labelled_by(kind.id);
                if before != draft.kind {
                    draft.value = draft.kind.value();
                }
                widgets::field_label(ui, "Initial value");
                fields::value(ui, &mut draft.value, 0);
                ui.checkbox(&mut draft.rows, "List row array").on_hover_text(
                    "A value for each row of a list, starting with one row. Bindings and Row \
                     data edit the rows.",
                );
                if let Some(error) = draft.error(bindings) {
                    ui.weak(error);
                }
                ui.horizontal(|ui| {
                    let ok = draft.error(bindings).is_none();
                    if ui.add_enabled(ok, egui::Button::new("Create")).clicked()
                        || (ok && widgets::enter(ui))
                    {
                        let name = draft.name.trim().to_owned();
                        let value = if draft.rows {
                            json!([draft.value.clone()])
                        } else {
                            draft.value.clone()
                        };
                        bindings.insert(name.clone(), Binding { value, ..Default::default() });
                        created = Some(name);
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
                d.remove::<BindDraft>(id);
            } else if let Some(draft) = draft {
                d.insert_temp(id, draft);
            }
        });
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
        widgets::section_heading(ui, "Swap layout variants");
        ui.label(
            "A Swap layout is added in Design; its variants are made and edited in Properties \
             or Layers.",
        );
        let (mut remove, mut rename) = (None, None);
        for name in s.variants.keys() {
            ui.horizontal(|ui| {
                widgets::field_label(ui, name);
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
                    let mut to: String =
                        ui.ctx().data_mut(|d| d.get_temp(id)).unwrap_or_else(|| name.clone());
                    let label = widgets::field_label(ui, "Name");
                    ui.text_edit_singleline(&mut to).labelled_by(label.id);
                    let taken = to != *name && s.variants.contains_key(&to);
                    let ok = to != *name && !to.is_empty() && mg_nui::plain_name(&to) && !taken;
                    if ui
                        .add_enabled(ok, egui::Button::new("Rename"))
                        .on_disabled_hover_text(if taken {
                            "Another variant has that name"
                        } else {
                            "Type a new name: no spaces at either end"
                        })
                        .clicked()
                    {
                        rename = Some((name.clone(), to.clone()));
                        ui.close();
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(id, to));
                });
                // A handler that shows it calls its function: removing it would break the build.
                let call = format!("{}(", mg_nui::variant_function(window, name));
                let shown = script.is_some_and(|t| {
                    t.lines().any(|l| !l.trim_start().starts_with("//") && l.contains(&call))
                }) || s
                    .handlers
                    .iter()
                    .any(|r| matches!(&r.action, mg_nui::Action::Variant { variant: view, .. } if view==name));
                if ui
                    .add_enabled(!shown, egui::Button::new("Remove"))
                    .on_disabled_hover_text(
                        "A handler shows it: take that call out of the event script first",
                    )
                    .clicked()
                {
                    remove = Some(name.clone());
                }
            });
        }
        if let Some(name) = remove {
            s.variants.remove(&name);
            s.variant_targets.remove(&name);
        }
        if let Some((from, to)) = rename
            && let Some(view) = s.variants.remove(&from)
        {
            s.variants.insert(to.clone(), view);
            if let Some(target) = s.variant_targets.remove(&from) {
                s.variant_targets.insert(to.clone(), target);
            }
            for handler in &mut s.handlers {
                if let mg_nui::Action::Variant { variant: view, .. } = &mut handler.action
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
    // A file from disk goes into the module and is the picture, as a name chosen below.
    if let Some(name) = images::from_disk(ui) {
        if let Some(bind) = node[field]["bind"].as_str() {
            if let Some(b) = s.bindings.get_mut(bind).filter(|b| !b.value.is_array()) {
                b.value = json!(name);
            }
        } else {
            node[field] = json!(name);
        }
    }
    ui.collapsing("Choose image", |ui| {
        let filter = ui.id().with("nui-image-filter");
        let width = ui.spacing().text_edit_width;
        widgets::SearchField::new(filter, &mut state.asset_search)
            .hint("Find a picture…")
            .width(width)
            .show(ui);
        let query = mg_core::text::Needle::new(&state.asset_search);
        let names: Vec<_> =
            assets.catalog.iter().filter(|n| query.finds(n)).take(24).cloned().collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bind_s_name_that_is_taken_empty_or_unreadable_is_refused() {
        let mut bindings = std::collections::BTreeMap::new();
        bindings.insert("taken".into(), Binding { value: json!(true), ..Default::default() });
        let mut draft = BindDraft::new();
        assert!(draft.error(&bindings).is_some());
        draft.name = "  taken  ".into();
        assert!(draft.error(&bindings).is_some());
        draft.name = "bad\0name".into();
        assert!(draft.error(&bindings).is_some());
        draft.name = "fresh".into();
        assert!(draft.error(&bindings).is_none());
    }

    #[test]
    fn each_kind_of_new_bind_validates_where_it_is_used() {
        let mut doc = mg_nui::window();
        let mut settings = Settings::default();
        let mut children = Vec::new();
        for (kind, widget) in [
            (BindType::Text, "label"),
            (BindType::Integer, "combo"),
            (BindType::Decimal, "sliderf"),
            (BindType::Boolean, "check"),
            (BindType::Color, "color_picker"),
        ] {
            let name = kind.label().to_lowercase();
            settings
                .bindings
                .insert(name.clone(), Binding { value: kind.value(), ..Default::default() });
            let mut node = mg_nui::template(widget);
            node["value"] = json!({"bind":name});
            children.push(node);
        }
        settings.bindings.insert(
            "position".into(),
            Binding { value: BindType::Point.value(), ..Default::default() },
        );
        children[0]["draw_list"] = json!([{"type":6,"a":{"bind":"position"},"b":{"x":10.0,"y":10.0},"color":{"r":255,"g":255,"b":255,"a":255},"line_thickness":1.0}]);
        settings.bindings.insert(
            "bounds".into(),
            Binding { value: BindType::Rectangle.value(), ..Default::default() },
        );
        doc["geometry"] = json!({"bind":"bounds"});
        doc["root"]["children"] = children.into();
        let errors: Vec<_> = mg_nui::validate(&doc, &settings)
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{errors:?}");
    }
}
