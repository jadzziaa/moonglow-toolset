//! Parts the trigger, door and placeable editors share, as Aurora's
//! situated frames: the trap settings (`TfrmTrap`), event scripts, the
//! faction and the portrait, the area transition, the lock, hit points and
//! saves, and the Advanced page.

use egui::Ui;
use mg_core::ResType;
use mg_gff::{FieldType, Value};
use mg_resman::ResKey;
use mg_rules::{Choice, ChoiceColumns};

use super::Form;
use crate::text::{decode, encode};
use crate::widgets::commit_number;

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

/// The module's factions (repute.fac), as choices by index; without one,
/// the standard factions every module starts with.
pub(super) fn factions(f: &mut Form<'_>) -> Vec<Choice> {
    let key = ResKey::parse("repute", ResType::FAC).expect("valid");
    let names: Vec<String> = match f.app.ws.as_mut().map(|ws| ws.doc(&key)) {
        Some(Ok(g)) => g
            .root
            .list("FactionList")
            .unwrap_or(&[])
            .iter()
            .map(|s| decode(s.string("FactionName").unwrap_or_default()))
            .collect(),
        _ => mg_module::factions::STANDARD.iter().map(|n| n.to_string()).collect(),
    };
    names.into_iter().enumerate().map(|(row, text)| Choice { row, text }).collect()
}

/// A faction field.
pub(super) fn faction(f: &mut Form<'_>, ui: &mut Ui, what: &str, label: &str) {
    let choices = factions(f);
    f.choice(ui, what, label, &choices, FieldType::Dword);
}

/// The portrait: a portraits.2da row (`PortraitId`), else, in older
/// blueprints, a resref (`Portrait`, `po_` and the row's base resref). A
/// choice sets the row, and the resref where the blueprint has one.
pub(super) fn portrait(f: &mut Form<'_>, ui: &mut Ui) {
    let Some(table) = f.app.game.as_ref().and_then(|g| g.table("portraits").ok()) else {
        return;
    };
    let base = |row: usize| table.get(row, "BaseResRef").filter(|b| *b != "****");
    let choices: Vec<(usize, &str)> =
        (0..table.len()).filter_map(|row| base(row).map(|b| (row, b))).collect();
    let resref = f.root.resref("Portrait").filter(|r| !r.is_empty());
    let current = match f.root.integer("PortraitId") {
        Some(id) if id != 0xFFFF => Some(id as usize),
        _ => resref.and_then(|r| {
            let name = r.to_string();
            choices.iter().find(|(_, b)| format!("po_{b}").eq_ignore_ascii_case(&name)).map(|c| c.0)
        }),
    };
    let shown = match (current.and_then(base), resref) {
        (Some(b), _) => b.to_string(),
        (None, Some(r)) => r.to_string(),
        (None, None) => "(none)".to_string(),
    };
    let filter_id = egui::Id::new(("portrait-filter", f.key));
    let mut pick = None;
    egui::ComboBox::from_id_salt(("portrait", f.key)).selected_text(shown).width(220.0).show_ui(
        ui,
        |ui| {
            let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
            ui.add(egui::TextEdit::singleline(&mut filter).hint_text("Filter"));
            ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
            let filter = filter.to_lowercase();
            for (row, b) in choices.iter().filter(|(_, b)| b.to_lowercase().contains(&filter)) {
                if ui.selectable_label(current == Some(*row), *b).clicked() {
                    pick = Some((*row, b.to_string()));
                }
            }
        },
    );
    if let Some((row, b)) = pick.filter(|(row, _)| current != Some(*row)) {
        let id = super::integer(f.root.get("PortraitId"), row as i64, FieldType::Word);
        let mut fields = vec![("PortraitId", id)];
        if f.root.get("Portrait").is_some()
            && let Ok(r) = mg_core::ResRef::from_str(&format!("po_{}", b.to_lowercase()))
        {
            fields.push(("Portrait", Value::resref(r)));
        }
        f.set_fields("Portrait", fields);
    }
}

/// Doors and waypoints in the module's areas: (area, tag, flags) with
/// flags as `LinkedToFlags` (1 door, 2 waypoint).
fn destinations(f: &mut Form<'_>) -> Vec<(String, String, i64)> {
    let Some(ws) = f.app.ws.as_mut() else { return Vec::new() };
    let gits: Vec<_> = ws.module.keys_of(ResType::GIT).copied().collect();
    let mut out = Vec::new();
    for key in gits {
        let Ok(g) = ws.doc(&key) else { continue };
        for (list, flags) in [("Door List", 1), ("WaypointList", 2)] {
            for s in g.root.list(list).unwrap_or(&[]) {
                let tag = decode(s.string("Tag").unwrap_or_default());
                if !tag.is_empty() {
                    out.push((key.resref.to_string(), tag, flags));
                }
            }
        }
    }
    out.sort();
    out
}

/// The Area Transition page of doors and transition triggers: the
/// destination's tag and type, Setup Area Transition, the loading screen.
pub(super) fn transition(f: &mut Form<'_>, ui: &mut Ui) {
    let loadscreens = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("loadscreens", ChoiceColumns { name: Some("StrRef"), label: Some("Label") })
                .ok()
        })
        .unwrap_or_default();
    egui::Grid::new(("utt-transition", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Destination Tag");
        f.text(ui, "Destination tag", "LinkedTo", 32);
        ui.end_row();
        ui.label("Destination is a");
        let flags = f.int("LinkedToFlags");
        ui.horizontal(|ui| {
            for (text, v) in [("Door", 1), ("Waypoint", 2), ("None", 0)] {
                if ui.radio(flags == v, text).clicked() && flags != v {
                    f.set_int("Destination type", "LinkedToFlags", v, FieldType::Byte);
                }
            }
        });
        ui.end_row();
        ui.label("");
        // Aurora's Setup Area Transition: a door or waypoint in an area.
        let dests = destinations(f);
        let mut pick = None;
        egui::ComboBox::from_id_salt(("utt-setup", f.key))
            .selected_text("Setup Area Transition…")
            .width(260.0)
            .show_ui(ui, |ui| {
                if dests.is_empty() {
                    ui.weak("No doors or waypoints with tags in the module's areas");
                }
                for (area, tag, flags) in &dests {
                    let kind = if *flags == 1 { "door" } else { "waypoint" };
                    if ui.selectable_label(false, format!("{area}: {tag} ({kind})")).clicked() {
                        pick = Some((tag.clone(), *flags));
                    }
                }
            });
        if let Some((tag, flags)) = pick {
            f.set_fields(
                "Area transition",
                vec![
                    ("LinkedTo", Value::String(encode(&tag))),
                    (
                        "LinkedToFlags",
                        super::integer(f.root.get("LinkedToFlags"), flags, FieldType::Byte),
                    ),
                ],
            );
        }
        ui.end_row();
        ui.label("Loading Screen");
        f.choice(ui, "Loading screen", "LoadScreenID", &loadscreens, FieldType::Word);
        ui.end_row();
    });
}

/// The Lock page (`TfraSituatedLock`).
pub(super) fn lock(f: &mut Form<'_>, ui: &mut Ui) {
    f.check(ui, "Locked", "Locked");
    f.check(ui, "Can be relocked", "Lockable");
    f.check(ui, "Key required to unlock or lock", "KeyRequired");
    f.check(ui, "Automatically remove key after use", "AutoRemoveKey");
    ui.separator();
    egui::Grid::new(("lock", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Open Lock DC");
        f.number(ui, "Open lock DC", "OpenLockDC", 0..=100);
        ui.end_row();
        ui.label("Close Lock DC");
        f.number(ui, "Close lock DC", "CloseLockDC", 0..=100);
        ui.end_row();
        ui.label("Key Tag");
        f.text(ui, "Key tag", "KeyName", 32);
        ui.end_row();
    });
}

/// Hit points, hardness, saves and plot (`TfraSituatedBasic`), as grid rows.
pub(super) fn durability(f: &mut Form<'_>, ui: &mut Ui) {
    ui.label("Hit Points");
    // The blueprint's current hit points follow its maximum.
    if let Some(v) = commit_number(ui, f.int("HP"), 1..=32_767) {
        f.set_many(
            "Hit points",
            &[("HP", v, FieldType::Short), ("CurrentHP", v, FieldType::Short)],
        );
    }
    ui.end_row();
    ui.label("Hardness");
    f.number(ui, "Hardness", "Hardness", 0..=100);
    ui.end_row();
    ui.label("Fortitude Save");
    f.number(ui, "Fortitude save", "Fort", 0..=100);
    ui.end_row();
    ui.label("Reflex Save");
    f.number(ui, "Reflex save", "Ref", 0..=100);
    ui.end_row();
    ui.label("Will Save");
    f.number(ui, "Will save", "Will", 0..=100);
    ui.end_row();
    ui.label("");
    f.check(ui, "Plot", "Plot");
    ui.end_row();
}

/// The Advanced page (`TfraSituatedAdvanced`): blueprint resref, faction,
/// conversation, portrait, No Interrupt, initial state (`states`), and the
/// caller's `extra` rows, then Variables.
pub(super) fn advanced(
    f: &mut Form<'_>,
    ui: &mut Ui,
    states: &[(i64, &str)],
    extra: impl FnOnce(&mut Form<'_>, &mut Ui),
) {
    let states: Vec<Choice> = states
        .iter()
        .map(|(row, text)| Choice { row: *row as usize, text: (*text).into() })
        .collect();
    egui::Grid::new(("situated-advanced", f.key)).num_columns(2).spacing([12.0, 6.0]).show(
        ui,
        |ui| {
            ui.label("Blueprint ResRef");
            f.blueprint_resref(ui);
            ui.end_row();
            ui.label("Belongs to Faction");
            faction(f, ui, "Faction", "Faction");
            ui.end_row();
            ui.label("Conversation");
            f.conversation(ui);
            ui.end_row();
            ui.label("Portrait");
            portrait(f, ui);
            ui.end_row();
            ui.label("Initial State");
            f.choice(ui, "Initial state", "AnimationState", &states, FieldType::Byte);
            ui.end_row();
            ui.label("");
            // Aurora's No Interrupt is the inverse of Interruptable.
            let mut no_interrupt = f.root.integer("Interruptable").unwrap_or(1) == 0;
            if ui.checkbox(&mut no_interrupt, "No Interrupt").changed() {
                f.set_int(
                    "No interrupt",
                    "Interruptable",
                    i64::from(!no_interrupt),
                    FieldType::Byte,
                );
            }
            ui.end_row();
            extra(f, ui);
            ui.label("Variables");
            f.variables(ui);
            ui.end_row();
            ui.label("");
            f.update_instances(ui);
            ui.end_row();
        },
    );
}

/// A choice among `choices` by row, outside a field (`current` is the
/// row); the row picked.
pub(super) fn pick(
    ui: &mut Ui,
    key: ResKey,
    name: &str,
    choices: &[Choice],
    current: i64,
) -> Option<i64> {
    let shown = choices
        .iter()
        .find(|c| c.row as i64 == current)
        .map_or_else(|| format!("({current})"), |c| c.text.clone());
    let mut pick = None;
    egui::ComboBox::from_id_salt(("pick", key, name)).selected_text(shown).width(220.0).show_ui(
        ui,
        |ui| {
            for c in choices {
                if ui.selectable_label(c.row as i64 == current, &c.text).clicked() {
                    pick = Some(c.row as i64);
                }
            }
        },
    );
    pick.filter(|&v| v != current)
}

/// Preview: the blueprint's model in the model viewer (Aurora shows it in
/// the dialog).
pub(super) fn preview_button(f: &mut Form<'_>, ui: &mut Ui) {
    if ui.button("Preview").on_hover_text("Show the model").clicked() {
        f.app.actions.push(crate::Action::OpenTab(crate::Tab::Model(f.key)));
    }
}
