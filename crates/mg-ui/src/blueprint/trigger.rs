//! Trigger Properties (`TdlgTriggerEdit`): Basic, Area Transition, Trap,
//! Scripts, Advanced, Comments. The Area Transition and Trap pages apply to
//! those trigger types.

use egui::Ui;
use mg_gff::FieldType;
use mg_module::palette::BlueprintKind;
use mg_rules::Choice;

use super::{Form, situated};

pub(super) const PAGES: [&str; 8] =
    ["Basic", "Area Transition", "Trap", "Scripts", "Advanced", "Variables", "Visuals", "Comments"];

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
        "Visuals" => super::visuals::page(f, ui, false),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let types: Vec<Choice> = TYPES
        .iter()
        .map(|(row, text)| Choice { row: *row as usize, text: (*text).into() })
        .collect();
    egui::Grid::new(("utt-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        f.name_and_tag(ui, "LocalizedName");
        crate::widgets::field_label(ui, "Trigger Type");
        trigger_type(f, ui, &types);
        ui.end_row();
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Trigger);
        ui.end_row();
    });
}

/// Trigger Type, as Aurora's: a trigger made an area transition takes the
/// transition cursor too (`Cursor` 1) where it had none (0), in the same
/// step; made generic or a trap, only its type changes (captured:
/// `transition-triggers/06-typechange.mod`). Each trigger edited together
/// by its own cursor.
fn trigger_type(f: &mut Form<'_>, ui: &mut Ui, types: &[Choice]) {
    f.mark_mixed(ui, "Type");
    let current = f.int("Type");
    let Some(v) = crate::widgets::row_choice(f.id("Type"), types, current).width(220.0).show(ui)
    else {
        return;
    };
    let typed =
        super::integer(f.root.get("Type"), v, f.schema_type("Type").unwrap_or(FieldType::Int));
    let cursor_type = f.schema_type("Cursor").unwrap_or(FieldType::Byte);
    let targets: Vec<(mg_resman::ResKey, mg_gff::GffPath)> =
        f.targets().map(|(k, p)| (k, p.clone())).collect();
    let mut edits = Vec::new();
    for (key, path) in targets {
        let cursor = match key == f.key && path == f.path {
            true => f.root.get("Cursor").cloned(),
            false => (f.app.ws.as_mut())
                .and_then(|ws| ws.doc(&key).ok())
                .and_then(|d| path.get(&d.root))
                .and_then(|s| s.get("Cursor").cloned()),
        };
        let set = |label: &str, value| mg_edit::Edit::SetField {
            key,
            path: path.clone(),
            label: label.into(),
            value: Some(value),
        };
        edits.push(set("Type", typed.clone()));
        let none = cursor.as_ref().is_none_or(|c| c.as_integer() == Some(0));
        if v == 1 && none {
            edits.push(set("Cursor", super::integer(cursor.as_ref(), 1, cursor_type)));
        }
    }
    f.app.actions.push(crate::Action::Apply(mg_edit::Command::new("Trigger type", edits)));
}

fn transition(f: &mut Form<'_>, ui: &mut Ui) {
    if f.int("Type") == 1 {
        situated::transition(f, ui);
    } else {
        ui.weak("These settings apply to Area Transition triggers (Basic › Trigger Type).");
    }
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let cursors = f.choices("cursors", None, Some("Label"));
    egui::Grid::new(("utt-advanced", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Blueprint ResRef");
        f.blueprint_resref(ui);
        ui.end_row();
        crate::widgets::field_label(ui, "Faction");
        situated::faction(f, ui, "Faction", "Faction");
        ui.end_row();
        crate::widgets::field_label(ui, "Key Tag");
        f.text(ui, "Key tag", "KeyName", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "");
        f.check(ui, "Auto Remove Key", "AutoRemoveKey");
        ui.end_row();
        crate::widgets::field_label(ui, "Cursor");
        f.choice(ui, "Cursor", "Cursor", &cursors, FieldType::Byte);
        ui.end_row();
        crate::widgets::field_label(ui, "Portrait");
        situated::portrait(f, ui);
        ui.end_row();
        crate::widgets::field_label(ui, "Highlight Height");
        f.spin_float(ui, "Highlight height", "HighlightHeight", 0.0..=10.0, 0.1, 1.0);
        ui.end_row();
        crate::widgets::field_label(ui, "Variables");
        f.variables(ui);
        ui.end_row();
        crate::widgets::field_label(ui, "");
        f.update_instances(ui);
        ui.end_row();
    });
}
