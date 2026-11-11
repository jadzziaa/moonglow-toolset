//! Encounter Properties (`TdlgEncounterEdit`): Basic, Creature List,
//! Scripts, Advanced, Comments.

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Command, Edit, lists};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_rules::Choice;
use mg_uikit::Controls as _;

use super::{Form, situated};
use crate::Action;

pub(super) const PAGES: [&str; 6] =
    ["Basic", "Creature List", "Scripts", "Advanced", "Variables", "Comments"];

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
        f.name_and_tag(ui, "LocalizedName");
        crate::widgets::field_label(ui, "Difficulty");
        // The index, and the difficulty value it stands for.
        f.mark_mixed(ui, "DifficultyIndex");
        let current = f.int("DifficultyIndex");
        let id = f.id("ute-diff");
        if let Some(row) = crate::widgets::row_choice(id, &difficulties, current).show(ui) {
            let value = usize::try_from(row).ok().and_then(|r| values.get(r)).copied();
            f.set_many(
                "Difficulty",
                &[
                    ("DifficultyIndex", row, FieldType::Int),
                    ("Difficulty", value.unwrap_or(0), FieldType::Int),
                ],
            );
        }
        ui.end_row();
        crate::widgets::field_label(ui, "Spawn Option");
        f.choice(ui, "Spawn option", "SpawnOption", &spawn, FieldType::Int);
        ui.end_row();
        crate::widgets::field_label(ui, "Minimum Creatures");
        minimum_creatures(f, ui);
        ui.end_row();
        crate::widgets::field_label(ui, "Maximum Creatures");
        f.spin(ui, "Maximum creatures", "MaxCreatures", 1..=8);
        ui.end_row();
        crate::widgets::field_label(ui, "Category");
        f.category(ui, BlueprintKind::Encounter);
        ui.end_row();
    });
}

/// Minimum Creatures, which raises Maximum Creatures to a minimum typed
/// above it, as Aurora's does, in one command (each encounter edited
/// together by its own: one whose maximum is higher keeps it).
fn minimum_creatures(f: &mut Form<'_>, ui: &mut Ui) {
    f.mark_mixed(ui, "RecCreatures");
    let Some(v) = crate::widgets::spin_number(ui, f.int("RecCreatures"), 1..=8) else { return };
    let restype = f.restype();
    let ty = |label| mg_schema::root_field_type(restype, label).unwrap_or(FieldType::Int);
    let (rec_type, max_type) = (ty("RecCreatures"), ty("MaxCreatures"));
    let targets: Vec<_> = f.targets().map(|(k, p)| (k, p.clone())).collect();
    let mut edits = Vec::new();
    for (key, path) in targets {
        let fields = f.app.ws.as_mut().and_then(|ws| {
            let s = path.get(&ws.doc(&key).ok()?.root)?;
            Some((s.get("RecCreatures").cloned(), s.get("MaxCreatures").cloned()))
        });
        let (rec, max) = fields.unwrap_or_default();
        let set = |label: &str, old: Option<Value>, t| Edit::SetField {
            key,
            path: path.clone(),
            label: label.to_string(),
            value: Some(super::integer(old.as_ref(), v, t)),
        };
        if max.as_ref().and_then(Value::as_integer).is_none_or(|m| m < v) {
            edits.push(set("MaxCreatures", max, max_type));
        }
        edits.push(set("RecCreatures", rec, rec_type));
    }
    f.app.actions.push(Action::Apply(Command::new("Minimum creatures", edits)));
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
    if f.several() {
        creatures_together(f, ui);
        return;
    }
    let base = f.path.clone();
    let key = f.key;
    let list: Vec<Struct> = f.root.list("CreatureList").unwrap_or(&[]).to_vec();
    let names = f.blueprint_names(BlueprintKind::Creature);
    // Each creature's tag, from its blueprint.
    let tags: Vec<String> = list
        .iter()
        .map(|c| {
            let r = c.resref("ResRef").unwrap_or(ResRef::EMPTY);
            f.kept_blueprint(BlueprintKind::Creature, r)
                .and_then(|u| u.string("Tag").map(crate::text::decode))
                .unwrap_or_default()
        })
        .collect();
    let mut add = None;
    let mut remove = None;
    let mut unique = None;
    let selected_id = f.id("ute-selected");
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(280.0);
            add = f.palette_picker(ui, BlueprintKind::Creature, "Add Creature");
        });
        ui.separator();
        let sel_id = selected_id;
        let mut selected: Option<usize> =
            ui.data(|d| d.get_temp(sel_id)).filter(|&i: &usize| i < list.len());
        ui.vertical(|ui| {
            {
                mg_uikit::with_rows(ui, |ui| {
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
                            if ui.check_box(&mut single, "").changed() {
                                unique = Some((i, single));
                            }
                            ui.end_row();
                        }
                    })
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
            vec![Edit::insert_item(key, path.clone(), "CreatureList", list.len(), item)],
        )));
    }
    if let Some(i) = remove {
        f.app.actions.push(Action::Apply(Command::new(
            "Remove creature",
            vec![Edit::remove_item(key, path.clone(), "CreatureList", i)],
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

/// The creature blueprint an encounter's entry spawns.
fn spawned(e: &Struct) -> Option<ResRef> {
    e.resref("ResRef").filter(|r| !r.is_empty())
}

/// The Creature List of several encounters edited together: every
/// creature any of them spawns, ≠ with how many where some do. A creature
/// added is added to those that lack it, one removed is taken from those
/// that have it, and Unique is set on each that has it.
fn creatures_together(f: &mut Form<'_>, ui: &mut Ui) {
    let key = f.key;
    let targets = f.list_targets();
    let total = targets.len();
    let merged = lists::merged(&targets, "CreatureList", spawned);
    let names = f.blueprint_names(BlueprintKind::Creature);
    let tags: Vec<String> = merged
        .iter()
        .map(|m| {
            f.kept_blueprint(BlueprintKind::Creature, m.key)
                .and_then(|u| u.string("Tag").map(crate::text::decode))
                .unwrap_or_default()
        })
        .collect();
    let mut add = None;
    let mut change: Option<(&str, lists::Outcome, &str)> = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(280.0);
            add = f.palette_picker(ui, BlueprintKind::Creature, "Add Creature");
        });
        ui.separator();
        let sel_id = f.id("ute-selected-together");
        let mut selected: Option<ResRef> =
            ui.data(|d| d.get_temp(sel_id)).filter(|r: &ResRef| merged.iter().any(|m| m.key == *r));
        ui.vertical(|ui| {
            mg_uikit::with_rows(ui, |ui| {
                egui::Grid::new(("ute-list", key)).num_columns(5).striped(true).show(ui, |ui| {
                    for h in ["CR", "Creature", "Tag", "Blueprint ResRef", "Unique"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for (m, tag) in merged.iter().zip(&tags) {
                        let resref = m.key;
                        let is = move |e: &Struct| spawned(e) == Some(resref);
                        let name =
                            names.get(&resref).cloned().unwrap_or_else(|| resref.to_string());
                        ui.label(format!("{}", m.entry.float("CR").unwrap_or(0.0)));
                        ui.horizontal(|ui| {
                            if ui.selectable_label(selected == Some(resref), name).clicked() {
                                selected = Some(resref);
                            }
                            super::entry_mark(ui, m.having.len(), total);
                        });
                        ui.label(tag);
                        ui.label(resref.to_string());
                        let single = |e: &Struct| e.integer("SingleSpawn").unwrap_or(0) != 0;
                        let differs = lists::differs(&targets, "CreatureList", is, single);
                        f.mark_if(ui, &format!("SingleSpawn#{resref}"), differs);
                        let mut on = single(&m.entry);
                        if ui.check_box(&mut on, "").changed() {
                            let out = lists::set(
                                &targets,
                                "CreatureList",
                                is,
                                &["SingleSpawn"],
                                |e, l| Some(lists::like(e.get(l), i64::from(on), Value::Byte(0))),
                            );
                            change = Some(("Unique creature", out, "Set on"));
                        }
                        ui.end_row();
                    }
                })
            });
            // (Under the list, for the creature chosen: beside each row,
            // they left a narrow window's edge.)
            let chosen = selected.and_then(|r| merged.iter().find(|m| m.key == r));
            ui.horizontal(|ui| {
                if ui.add_enabled(chosen.is_some(), egui::Button::new("Remove Creature")).clicked()
                    && let Some(r) = selected.take()
                {
                    let is = |e: &Struct| spawned(e) == Some(r);
                    let out = lists::remove(&targets, "CreatureList", is, |_| None);
                    change = Some(("Remove creature", out, "Taken from"));
                }
                let some = chosen.filter(|m| m.having.len() < total);
                let give = ui
                    .add_enabled(some.is_some(), egui::Button::new("Give to All"))
                    .on_hover_text(super::creature_lists::GIVE_TIP)
                    .on_disabled_hover_text("Choose a creature some of them lack");
                if give.clicked()
                    && let Some(m) = some
                {
                    let r = m.key;
                    let entry = m.entry.clone();
                    let is = |e: &Struct| spawned(e) == Some(r);
                    let out = lists::add(&targets, "CreatureList", is, |_| Ok(entry.clone()));
                    change = Some(("Add creature", out, "Added to"));
                }
            });
        });
        ui.data_mut(|d| match selected {
            Some(r) => {
                d.insert_temp(sel_id, r);
            }
            None => d.remove::<ResRef>(sel_id),
        });
    });
    if let Some(r) = add {
        let entry = creature_entry(f.app, r);
        let out =
            lists::add(&targets, "CreatureList", |e| spawned(e) == Some(r), |_| Ok(entry.clone()));
        change = Some(("Add creature", out, "Added to"));
    }
    if let Some((what, out, done)) = change {
        f.apply_each(what, out, done);
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
        ui.add_enabled_ui(respawns, |ui| f.spin(ui, "Respawn time", "ResetTime", 1..=32_000));
        ui.end_row();
        let count = f.int("Respawns");
        let infinite = count == -1;
        ui.add_enabled_ui(respawns, |ui| ui.label("Number of times to respawn"));
        ui.add_enabled_ui(respawns, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!infinite, |ui| f.spin(ui, "Respawns", "Respawns", 0..=32_000));
                let mut inf = infinite;
                if ui.check_box(&mut inf, "Infinite Respawn").changed() {
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
