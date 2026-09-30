//! Parts the trigger, door and placeable editors share, as Aurora's
//! situated frames: the trap settings (`TfrmTrap`), event scripts, the
//! faction and the portrait.

use egui::Ui;
use mg_gff::FieldType;
use mg_resman::ResKey;
use mg_rules::{Choice, ChoiceColumns};

use super::Form;
use crate::text::decode;

/// The Trap tab: Is Trapped, then the trap's settings (enabled when
/// trapped): type (traps.2da, with its rogue modifiers shown), disarmable,
/// detectable, one shot, the DCs and the trap scripts.
pub(super) fn trap(f: &mut Form<'_>, ui: &mut Ui) {
    let trapped = f.check(ui, "Is Trapped", "TrapFlag");
    ui.separator();
    let traps = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("traps", ChoiceColumns { name: Some("TrapName"), label: Some("Label") }).ok()
        })
        .unwrap_or_default();
    let row = f.int("TrapType");
    let table = f.app.game.as_ref().and_then(|g| g.table("traps").ok());
    let cell = |col: &str| {
        table
            .as_ref()
            .and_then(|t| t.get(row.max(0) as usize, col))
            .filter(|v| *v != "****")
            .unwrap_or("")
            .to_string()
    };
    ui.add_enabled_ui(trapped, |ui| {
        egui::Grid::new(("trap", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Trap Type");
            f.choice(ui, "Trap type", "TrapType", &traps, FieldType::Byte);
            ui.end_row();
            ui.label("Detect DC Mod. when set by Rogue");
            ui.label(cell("DetectDCMod"));
            ui.end_row();
            ui.label("Disarm DC Mod. when set by Rogue");
            ui.label(cell("DisarmDCMod"));
            ui.end_row();
            ui.label("Set DC");
            ui.label(cell("SetDC"));
            ui.end_row();
            ui.label("");
            ui.horizontal(|ui| {
                f.check(ui, "Disarmable", "TrapDisarmable");
                f.check(ui, "Detectable", "TrapDetectable");
                f.check(ui, "One Shot", "TrapOneShot");
            });
            ui.end_row();
            ui.label("Disarm DC");
            f.number(ui, "Disarm DC", "DisarmDC", 0..=100);
            ui.end_row();
            ui.label("Detection DC");
            f.number(ui, "Detection DC", "TrapDetectDC", 0..=100);
            ui.end_row();
            ui.label("OnDisarm");
            f.script(ui, "OnDisarm", "OnDisarm");
            ui.end_row();
            ui.label("OnTrapTriggered");
            f.script(ui, "OnTrapTriggered", "OnTrapTriggered");
            ui.end_row();
        });
    });
}

/// Event scripts: (label, field) rows.
pub(super) fn scripts(f: &mut Form<'_>, ui: &mut Ui, events: &[(&str, &str)]) {
    egui::Grid::new(("scripts", f.key)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
        for (label, field) in events {
            ui.label(*label);
            f.script(ui, label, field);
            ui.end_row();
        }
    });
}

/// The module's factions (repute.fac), as choices by index.
pub(super) fn factions(f: &mut Form<'_>) -> Vec<Choice> {
    let key = ResKey::parse("repute", mg_core::ResType::FAC).expect("valid");
    let Some(ws) = f.app.ws.as_mut() else { return Vec::new() };
    let Ok(g) = ws.doc(&key) else { return Vec::new() };
    g.root
        .list("FactionList")
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .map(|(row, s)| Choice { row, text: decode(s.string("FactionName").unwrap_or_default()) })
        .collect()
}

/// A faction field.
pub(super) fn faction(f: &mut Form<'_>, ui: &mut Ui, what: &str, label: &str) {
    let choices = factions(f);
    f.choice(ui, what, label, &choices, FieldType::Dword);
}

/// The portrait (portraits.2da row, by its base resref).
pub(super) fn portrait(f: &mut Form<'_>, ui: &mut Ui) {
    let portraits = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("portraits", ChoiceColumns { name: None, label: Some("BaseResRef") }).ok()
        })
        .unwrap_or_default();
    f.choice(ui, "Portrait", "PortraitId", &portraits, FieldType::Word);
}
