//! Waypoint Properties (`TdlgWaypointEdit`): Basic, Advanced, Description,
//! Comments.

use egui::Ui;
use mg_module::palette::BlueprintKind;
use mg_rules::ChoiceColumns;

use super::Form;

pub(super) const PAGES: [&str; 4] = ["Basic", "Advanced", "Description", "Comments"];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Advanced" => advanced(f, ui),
        "Description" => f.locstring_memo(ui, "Description", "Description"),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let appearances = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("waypoint", ChoiceColumns { name: Some("STRREF"), label: Some("LABEL") }).ok()
        })
        .unwrap_or_default();
    egui::Grid::new(("utw-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Name");
        f.locstring(ui, "Name", "LocalizedName");
        ui.end_row();
        crate::widgets::field_label(ui, "Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Waypoint);
        ui.end_row();
        crate::widgets::field_label(ui, "Appearance Type");
        f.choice(ui, "Appearance", "Appearance", &appearances, mg_gff::FieldType::Byte);
        ui.end_row();
    });
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let has_note = f.check(ui, "Waypoint Contains a Map Note", "HasMapNote");
    ui.add_enabled_ui(has_note, |ui| {
        f.check(ui, "Map Note Enabled", "MapNoteEnabled");
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Map Note Text");
            f.locstring(ui, "Map note", "MapNote");
        });
    });
    ui.separator();
    egui::Grid::new(("utw-advanced", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Blueprint ResRef");
        f.blueprint_resref(ui);
        ui.end_row();
        crate::widgets::field_label(ui, "Variables");
        f.variables(ui);
        ui.end_row();
    });
}
