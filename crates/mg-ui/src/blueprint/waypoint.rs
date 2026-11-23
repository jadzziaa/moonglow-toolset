//! Waypoint Properties (`TdlgWaypointEdit`): Basic, Advanced, Variables,
//! Description, Comments.

use egui::Ui;
use mg_module::palette::BlueprintKind;

use super::Form;

pub(super) const PAGES: [&str; 5] = ["Basic", "Advanced", "Variables", "Description", "Comments"];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Advanced" => advanced(f, ui),
        "Description" => f.locstring_memo(ui, "Description", "Description"),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let appearances = f.choices("waypoint", Some("STRREF"), Some("LABEL"));
    let appearances = f.appearance_rows(ui.ctx(), mg_rules::ObjectKind::Waypoint, &appearances);
    super::form_grid(ui, ("utw-basic", f.key)).spacing([12.0, 6.0]).show(ui, |ui| {
        f.name_and_tag(ui, "LocalizedName");
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Waypoint);
        ui.end_row();
        crate::widgets::field_label(ui, "Appearance type");
        f.choice(ui, "Appearance", "Appearance", &appearances, mg_gff::FieldType::Byte);
        ui.end_row();
        f.model_warning(ui, mg_rules::ObjectKind::Waypoint);
    });
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let has_note = f.check(ui, "Waypoint contains a map note", "HasMapNote");
    ui.add_enabled_ui(has_note, |ui| {
        f.check(ui, "Map note enabled", "MapNoteEnabled");
        super::row(ui, |ui| {
            crate::widgets::field_label(ui, "Map note text");
            f.locstring(ui, "Map note", "MapNote");
        });
    });
    ui.separator();
    super::form_grid(ui, ("utw-advanced", f.key)).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Blueprint ResRef");
        f.blueprint_resref(ui);
        ui.end_row();
        crate::widgets::field_label(ui, "Variables");
        f.variables(ui);
        ui.end_row();
    });
}
