//! Encounter Properties (`TdlgEncounterEdit`): Basic, Creature List,
//! Scripts, Advanced, Comments.

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_rules::{Choice, ChoiceColumns};

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
    let game = f.app.game.as_ref();
    let difficulties = game
        .and_then(|g| {
            g.choices("encdifficulty", ChoiceColumns { name: Some("STRREF"), label: Some("LABEL") })
                .ok()
        })
        .unwrap_or_default();
    let values: Vec<i64> = game
        .and_then(|g| g.table("encdifficulty").ok())
        .map(|t| (0..t.len()).map(|r| t.get_int(r, "VALUE").map_or(0, i64::from)).collect())
        .unwrap_or_default();
    let spawn = [
        Choice { row: 0, text: "Continuous".into() },
        Choice { row: 1, text: "Single Shot".into() },
    ];
    egui::Grid::new(("ute-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Name");
        f.locstring(ui, "Name", "LocalizedName");
        ui.end_row();
        ui.label("Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        ui.label("Difficulty");
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
        ui.label("Spawn Option");
        f.choice(ui, "Spawn option", "SpawnOption", &spawn, FieldType::Int);
        ui.end_row();
        ui.label("Minimum Creatures");
        f.number(ui, "Minimum creatures", "RecCreatures", 1..=100);
        ui.end_row();
        ui.label("Maximum Creatures");
        f.number(ui, "Maximum creatures", "MaxCreatures", 1..=100);
        ui.end_row();
        ui.label("Category");
        f.category(ui, BlueprintKind::Encounter);
        ui.end_row();
    });
}

/// A creature blueprint's challenge rating and appearance.
fn creature_entry(f: &mut Form<'_>, resref: ResRef) -> Struct {
    let utc = f.blueprint(BlueprintKind::Creature, resref);
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
    let key = f.key;
    let list: Vec<Struct> = f.root.list("CreatureList").unwrap_or(&[]).to_vec();
    let names = f.blueprint_names(BlueprintKind::Creature);
    let mut add = None;
    let mut remove = None;
    let mut unique = None;
    ui.columns(2, |cols| {
        add = f.palette_picker(&mut cols[0], BlueprintKind::Creature, "Add Creature");
        let ui = &mut cols[1];
        egui::Grid::new(("ute-list", key)).num_columns(4).striped(true).show(ui, |ui| {
            ui.strong("Creature");
            ui.strong("CR");
            ui.strong("Unique");
            ui.label("");
            ui.end_row();
            for (i, c) in list.iter().enumerate() {
                let resref = c.resref("ResRef").unwrap_or(ResRef::EMPTY);
                let name = names.get(&resref).cloned().unwrap_or_else(|| resref.to_string());
                ui.label(name).on_hover_text(resref.to_string());
                ui.label(format!("{}", c.float("CR").unwrap_or(0.0)));
                let mut single = c.integer("SingleSpawn").unwrap_or(0) != 0;
                if ui.checkbox(&mut single, "").changed() {
                    unique = Some((i, single));
                }
                if ui.small_button("Remove").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    });
    let path = GffPath::root();
    if let Some(r) = add {
        let item = creature_entry(f, r);
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
        ui.label("Blueprint ResRef");
        f.blueprint_resref(ui);
        ui.end_row();
        ui.label("");
        f.check(ui, "Active", "Active");
        ui.end_row();
        ui.label("");
        f.check(ui, "Player Triggered Only", "PlayerOnly");
        ui.end_row();
        ui.label("Faction");
        situated::faction(f, ui, "Faction", "Faction");
        ui.end_row();
        ui.label("");
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
        ui.label("Variables");
        f.variables(ui);
        ui.end_row();
    });
}
