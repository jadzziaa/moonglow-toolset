//! Trigger Properties (`TdlgTriggerEdit`): Basic, Area Transition, Trap,
//! Scripts, Advanced, Comments. The Area Transition and Trap pages apply to
//! those trigger types.

use egui::Ui;
use mg_core::ResType;
use mg_gff::{FieldType, Value};
use mg_module::palette::BlueprintKind;
use mg_rules::{Choice, ChoiceColumns};

use super::{Form, situated};
use crate::text::{decode, encode};

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

fn transition(f: &mut Form<'_>, ui: &mut Ui) {
    if f.int("Type") != 1 {
        ui.weak("These settings apply to Area Transition triggers (Basic › Trigger Type).");
        return;
    }
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
