//! Encounter Properties (`TdlgEncounterEdit`): Basic, Creature List,
//! Scripts, Advanced, Comments.

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Command, Edit};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_rules::Choice;

use super::{Form, situated};
use crate::Action;

pub(super) const PAGES: [&str; 5] = ["Basic", "Creature List", "Scripts", "Advanced", "Comments"];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Creature List" => creatures(f, ui),
        "Scripts" => situated::scripts(
            f,
            ui,
            &[
                ("OnEnter", "OnEntered"),
                ("OnExhausted", "OnExhausted"),
                ("OnExit", "OnExit"),
                ("OnHeartbeat", "OnHeartbeat"),
                ("OnUserDefined", "OnUserDefined"),
            ],
        ),
        "Advanced" => advanced(f, ui),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let game = f.app.game.as_deref();
    let difficulties = f.choices("encdifficulty", Some("STRREF"), Some("LABEL"));
    let values: Vec<i64> = game
        .and_then(|g| g.table("encdifficulty").ok())
        .map(|t| (0..t.len()).map(|r| t.get_int(r, "VALUE").map_or(0, i64::from)).collect())
        .unwrap_or_default();
    let spawn = [
        Choice { row: 0, text: "Continuous".into() },
        Choice { row: 1, text: "Single Shot".into() },
    ];
    egui::Grid::new(("ute-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Name");
        f.locstring(ui, "Name", "LocalizedName");
        ui.end_row();
        crate::widgets::field_label(ui, "Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "Difficulty");
        // The index, and the difficulty value it stands for.
        let current = f.int("DifficultyIndex");
        let shown = difficulties
            .iter()
            .find(|c| c.row as i64 == current)
            .map_or_else(|| format!("({current})"), |c| c.text.clone());
        let mut pick = None;
        egui::ComboBox::from_id_salt(("ute-diff", f.key)).selected_text(shown).show_ui(ui, |ui| {
            for c in &difficulties {
                if ui.selectable_label(c.row as i64 == current, &c.text).clicked() {
                    pick = Some(c.row);
                }
            }
        });
        if let Some(row) = pick.filter(|&r| r as i64 != current) {
            let value = values.get(row).copied().unwrap_or(0);
            f.set_many(
                "Difficulty",
                &[
                    ("DifficultyIndex", row as i64, FieldType::Int),
                    ("Difficulty", value, FieldType::Int),
                ],
            );
        }
        ui.end_row();
        crate::widgets::field_label(ui, "Spawn Option");
        f.choice(ui, "Spawn option", "SpawnOption", &spawn, FieldType::Int);
        ui.end_row();
        crate::widgets::field_label(ui, "Minimum Creatures");
        f.number(ui, "Minimum creatures", "RecCreatures", 1..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "Maximum Creatures");
        f.number(ui, "Maximum creatures", "MaxCreatures", 1..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Encounter);
        ui.end_row();
    });
}

/// An encounter's entry for a creature blueprint: its appearance and
/// challenge rating.
pub(crate) fn creature_entry(app: &mut crate::Moonglow, resref: ResRef) -> Struct {
    let utc = super::picker::blueprint(app, BlueprintKind::Creature, resref);
    let mut s = Struct::new(0);
    s.set(
        "Appearance",
        Value::Int(utc.as_ref().and_then(|u| u.integer("Appearance_Type")).unwrap_or(0) as i32),
    );
    s.set("CR", Value::Float(utc.as_ref().and_then(|u| u.float("ChallengeRating")).unwrap_or(0.0)));
    s.set("ResRef", Value::resref(resref));
    s.set("SingleSpawn", Value::Byte(0));
    s
}

fn creatures(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let list: Vec<Struct> = f.root.list("CreatureList").unwrap_or(&[]).to_vec();
    let names = f.blueprint_names(BlueprintKind::Creature);
    // Each creature's tag, from its blueprint.
    let tags: Vec<String> = list
        .iter()
        .map(|c| {
            let r = c.resref("ResRef").unwrap_or(ResRef::EMPTY);
            f.blueprint(BlueprintKind::Creature, r)
                .and_then(|u| u.string("Tag").map(crate::text::decode))
                .unwrap_or_default()
        })
        .collect();
    let mut add = None;
    let mut remove = None;
    let mut unique = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(280.0);
            add = f.palette_picker(ui, BlueprintKind::Creature, "Add Creature");
        });
        ui.separator();
        let sel_id = egui::Id::new(("ute-selected", key));
        let mut selected: Option<usize> =
            ui.data(|d| d.get_temp(sel_id)).filter(|&i: &usize| i < list.len());
        ui.vertical(|ui| {
            {
                egui::Grid::new(("ute-list", key)).num_columns(5).striped(true).show(ui, |ui| {
                    for h in ["CR", "Creature", "Tag", "Blueprint ResRef", "Unique"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for (i, c) in list.iter().enumerate() {
                        let resref = c.resref("ResRef").unwrap_or(ResRef::EMPTY);
                        let name =
                            names.get(&resref).cloned().unwrap_or_else(|| resref.to_string());
                        ui.label(format!("{}", c.float("CR").unwrap_or(0.0)));
                        if ui.selectable_label(selected == Some(i), name).clicked() {
                            selected = Some(i);
                        }
                        ui.label(tags.get(i).cloned().unwrap_or_default());
                        ui.label(resref.to_string());
                        let mut single = c.integer("SingleSpawn").unwrap_or(0) != 0;
                        if ui.checkbox(&mut single, "").changed() {
                            unique = Some((i, single));
                        }
                        ui.end_row();
                    }
                });
            }
            if ui.add_enabled(selected.is_some(), egui::Button::new("Remove Creature")).clicked() {
                remove = selected.take();
            }
        });
        ui.data_mut(|d| match selected {
            Some(i) => {
                d.insert_temp(sel_id, i);
            }
            None => d.remove::<usize>(sel_id),
        });
    });
    let path = base;
    if let Some(r) = add {
        let item = creature_entry(f.app, r);
        f.app.actions.push(Action::Apply(Command::new(
            "Add creature",
            vec![Edit::InsertItem {
                key,
                path: path.clone(),
                list: "CreatureList".into(),
                index: list.len(),
                item,
            }],
        )));
    }
    if let Some(i) = remove {
        f.app.actions.push(Action::Apply(Command::new(
            "Remove creature",
            vec![Edit::RemoveItem {
                key,
                path: path.clone(),
                list: "CreatureList".into(),
                index: i,
            }],
        )));
    }
    if let Some((i, v)) = unique {
        f.app.actions.push(Action::Apply(Command::new(
            "Unique creature",
            vec![Edit::SetField {
                key,
                path: path.item("CreatureList", i),
                label: "SingleSpawn".into(),
                value: Some(Value::Byte(u8::from(v))),
            }],
        )));
    }
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    egui::Grid::new(("ute-advanced", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Blueprint ResRef");
        f.blueprint_resref(ui);
        ui.end_row();
        crate::widgets::field_label(ui, "");
        f.check(ui, "Active", "Active");
        ui.end_row();
        crate::widgets::field_label(ui, "");
        f.check(ui, "Player Triggered Only", "PlayerOnly");
        ui.end_row();
        crate::widgets::field_label(ui, "Faction");
        situated::faction(f, ui, "Faction", "Faction");
        ui.end_row();
        crate::widgets::field_label(ui, "");
        let respawns = f.check(ui, "Encounter Respawns", "Reset");
        ui.end_row();
        ui.add_enabled_ui(respawns, |ui| ui.label("Respawn Time (seconds)"));
        ui.add_enabled_ui(respawns, |ui| f.number(ui, "Respawn time", "ResetTime", 0..=100_000));
        ui.end_row();
        let count = f.int("Respawns");
        let infinite = count == -1;
        ui.add_enabled_ui(respawns, |ui| ui.label("Number of times to respawn"));
        ui.add_enabled_ui(respawns, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!infinite, |ui| f.number(ui, "Respawns", "Respawns", 1..=100));
                let mut inf = infinite;
                if ui.checkbox(&mut inf, "Infinite Respawn").changed() {
                    f.set_int(
                        "Infinite respawn",
                        "Respawns",
                        if inf { -1 } else { 1 },
                        FieldType::Int,
                    );
                }
            });
        });
        ui.end_row();
        crate::widgets::field_label(ui, "Variables");
        f.variables(ui);
        ui.end_row();
        crate::widgets::field_label(ui, "");
        f.update_instances(ui);
        ui.end_row();
    });
}
