//! Trigger Properties (`TdlgTriggerEdit`): Basic, Area Transition, Trap,
//! Scripts, Advanced, Comments. The Area Transition and Trap pages apply to
//! those trigger types.

use egui::Ui;
use mg_gff::FieldType;
use mg_module::palette::BlueprintKind;
use mg_rules::{Choice, ChoiceColumns};

use super::{Form, situated};

pub(super) const PAGES: [&str; 6] =
    ["Basic", "Area Transition", "Trap", "Scripts", "Advanced", "Comments"];

/// Trigger types (`Type`).
const TYPES: [(i64, &str); 3] = [(0, "Generic"), (1, "Area Transition"), (2, "Trap")];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Area Transition" => transition(f, ui),
        "Trap" => {
            if f.int("Type") == 2 {
                situated::trap(f, ui);
            } else {
                ui.weak("Trap settings apply to Trap triggers (Basic › Trigger Type).");
            }
        }
        "Scripts" => situated::scripts(
            f,
            ui,
            &[
                ("OnClick", "OnClick"),
                ("OnEnter", "ScriptOnEnter"),
                ("OnExit", "ScriptOnExit"),
                ("OnHeartbeat", "ScriptHeartbeat"),
                ("OnUserDefined", "ScriptUserDefine"),
            ],
        ),
        "Advanced" => advanced(f, ui),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let types: Vec<Choice> = TYPES
        .iter()
        .map(|(row, text)| Choice { row: *row as usize, text: (*text).into() })
        .collect();
    egui::Grid::new(("utt-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Name");
        f.locstring(ui, "Name", "LocalizedName");
        ui.end_row();
        ui.label("Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        ui.label("Trigger Type");
        f.choice(ui, "Trigger type", "Type", &types, FieldType::Int);
        ui.end_row();
        ui.label("Category");
        f.category(ui, BlueprintKind::Trigger);
        ui.end_row();
    });
}

fn transition(f: &mut Form<'_>, ui: &mut Ui) {
    if f.int("Type") == 1 {
        situated::transition(f, ui);
    } else {
        ui.weak("These settings apply to Area Transition triggers (Basic › Trigger Type).");
    }
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let cursors = f
        .app
        .game
        .as_ref()
        .and_then(|g| g.choices("cursors", ChoiceColumns { name: None, label: Some("Label") }).ok())
        .unwrap_or_default();
    egui::Grid::new(("utt-advanced", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Blueprint ResRef");
        f.blueprint_resref(ui);
        ui.end_row();
        ui.label("Faction");
        situated::faction(f, ui, "Faction", "Faction");
        ui.end_row();
        ui.label("Key Tag");
        f.text(ui, "Key tag", "KeyName", 32);
        ui.end_row();
        ui.label("");
        f.check(ui, "Auto Remove Key", "AutoRemoveKey");
        ui.end_row();
        ui.label("Cursor");
        f.choice(ui, "Cursor", "Cursor", &cursors, FieldType::Byte);
        ui.end_row();
        ui.label("Portrait");
        situated::portrait(f, ui);
        ui.end_row();
        ui.label("Highlight Height");
        f.float(ui, "Highlight height", "HighlightHeight", 0.0..=100.0, 0.1);
        ui.end_row();
        ui.label("Variables");
        f.variables(ui);
        ui.end_row();
    });
}
