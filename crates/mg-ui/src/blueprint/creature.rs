//! Creature Properties (`TdlgCreatureEdit`): Basic, Statistics, Appearance,
//! Classes, Skills, Feats, Spells, Special Abilities, the inventory, Scripts,
//! Advanced, Comments.
//!
//! Every change also stores the creature's recomputed `MaxHitPoints` (in
//! the same undoable command), as Aurora does on OK.

use std::sync::Arc;

use egui::Ui;
use mg_core::ResType;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;
use mg_rules::creatures::{ABILITIES, modifier};
use mg_rules::items::{part_number, wide_label};
use mg_rules::{Choice, ChoiceColumns, CreatureSheet, GameData};

use super::{Form, creature_lists, situated};
use crate::widgets::commit_number;
use crate::{Action, Moonglow, Tab};

pub(super) const PAGES: [&str; 12] = [
    "Basic",
    "Statistics",
    "Appearance",
    "Classes",
    "Skills",
    "Feats",
    "Spells",
    "Special Abilities",
    "Inventory",
    "Scripts",
    "Advanced",
    "Comments",
];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "Basic" => basic(f, ui),
        "Statistics" => statistics(f, ui),
        "Appearance" => appearance(f, ui),
        "Classes" => classes(f, ui),
        "Skills" => skills(f, ui),
        "Feats" => creature_lists::feats(f, ui),
        "Spells" => creature_lists::spells(f, ui),
        "Special Abilities" => creature_lists::special_abilities(f, ui),
        "Inventory" => creature_lists::inventory(f, ui),
        "Scripts" => situated::scripts(
            f,
            ui,
            &[
                ("OnBlocked", "ScriptOnBlocked"),
                ("OnCombatRoundEnd", "ScriptEndRound"),
                ("OnConversation", "ScriptDialogue"),
                ("OnDamaged", "ScriptDamaged"),
                ("OnDeath", "ScriptDeath"),
                ("OnDisturbed", "ScriptDisturbed"),
                ("OnHeartbeat", "ScriptHeartbeat"),
                ("OnPerception", "ScriptOnNotice"),
                ("OnPhysicalAttacked", "ScriptAttacked"),
                ("OnRested", "ScriptRested"),
                ("OnSpawn", "ScriptSpawn"),
                ("OnSpellCastAt", "ScriptSpellAt"),
                ("OnUserDefined", "ScriptUserDefine"),
            ],
        ),
        "Advanced" => advanced(f, ui),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

/// After a command: the `MaxHitPoints` of each creature it changed,
/// recomputed, as part of the same command.
pub(crate) fn refresh_hit_points(app: &mut Moonglow, cmd: &Command) {
    let objects = super::changed_objects(cmd, ResType::UTC, "MaxHitPoints");
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_ref()) else { return };
    let mut edits = Vec::new();
    for (key, path) in objects {
        let Ok(g) = ws.doc(&key) else { continue };
        let Some(creature) = path.get(&g.root) else { continue };
        let max = game.creature_stats(&CreatureSheet::from_gff(creature)).max_hit_points;
        if creature.integer("MaxHitPoints") != Some(i64::from(max)) {
            let old = creature.get("MaxHitPoints");
            edits.push(Edit::SetField {
                key,
                path,
                label: "MaxHitPoints".into(),
                value: Some(super::integer(old, max.into(), FieldType::Short)),
            });
        }
    }
    if !edits.is_empty()
        && let Err(e) = ws.amend(edits)
    {
        app.log.error(e.to_string());
    }
}

fn choices(f: &Form<'_>, table: &str, name: &str, label: &str) -> Vec<Choice> {
    f.app
        .game
        .as_ref()
        .and_then(|g| g.choices(table, ChoiceColumns { name: Some(name), label: Some(label) }).ok())
        .unwrap_or_default()
}

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    let races = choices(f, "racialtypes", "Name", "Label");
    let appearances = choices(f, "appearance", "STRING_REF", "LABEL");
    let phenotypes = choices(f, "phenotype", "Name", "Label");
    let genders = choices(f, "gender", "NAME", "GENDER");
    let cr = f.root.float("ChallengeRating").unwrap_or(0.0);
    egui::Grid::new(("utc-basic", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("First Name");
        f.locstring(ui, "First name", "FirstName");
        ui.end_row();
        ui.label("Last Name");
        f.locstring(ui, "Last name", "LastName");
        ui.end_row();
        ui.label("Tag");
        f.text(ui, "Tag", "Tag", 32);
        ui.end_row();
        ui.label("Race");
        f.choice(ui, "Race", "Race", &races, FieldType::Byte);
        ui.end_row();
        ui.label("Appearance");
        f.choice(ui, "Appearance", "Appearance_Type", &appearances, FieldType::Word);
        ui.end_row();
        ui.label("Phenotype");
        f.choice(ui, "Phenotype", "Phenotype", &phenotypes, FieldType::Int);
        ui.end_row();
        ui.label("Gender");
        f.choice(ui, "Gender", "Gender", &genders, FieldType::Byte);
        ui.end_row();
        ui.label("Challenge Rating");
        ui.label(format!("{cr}"));
        ui.end_row();
        ui.label("Category");
        f.category(ui, BlueprintKind::Creature);
        ui.end_row();
        ui.label("Portrait");
        situated::portrait(f, ui);
        ui.end_row();
        ui.label("Conversation");
        f.conversation(ui);
        ui.end_row();
        ui.label("");
        let mut no_interrupt = f.root.integer("Interruptable").unwrap_or(1) == 0;
        if ui.checkbox(&mut no_interrupt, "No Interrupt").changed() {
            f.set_int("No interrupt", "Interruptable", i64::from(!no_interrupt), FieldType::Byte);
        }
        ui.end_row();
        ui.label("");
        situated::preview_button(f, ui);
        ui.end_row();
    });
    ui.separator();
    f.locstring_memo(ui, "Description", "Description");
}

fn statistics(f: &mut Form<'_>, ui: &mut Ui) {
    let Some(stats) =
        f.app.game.as_ref().map(|g| g.creature_stats(&CreatureSheet::from_gff(&f.root)))
    else {
        return;
    };
    let speeds = choices(f, "creaturespeed", "Name", "Label");
    ui.strong("Ability Scores");
    egui::Grid::new(("utc-abilities", f.key))
        .num_columns(5)
        .spacing([16.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for h in ["", "Base", "Racial Modifier", "Total", "Bonus"] {
                ui.strong(h);
            }
            ui.end_row();
            let names =
                ["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"];
            for (i, (name, label)) in names.iter().zip(ABILITIES).enumerate() {
                ui.label(*name);
                f.number(ui, name, label, 3..=100);
                ui.label(signed(stats.racial[i]));
                ui.label(stats.totals[i].to_string());
                ui.label(signed(modifier(stats.totals[i])));
                ui.end_row();
            }
        });
    ui.separator();
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        ui.strong("Armor Class");
        egui::Grid::new(("utc-ac", f.key)).num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
            ui.label("Natural AC");
            f.number(ui, "Natural AC", "NaturalAC", 0..=100);
            ui.end_row();
            for (label, v) in [
                ("Base", "10".to_string()),
                ("Dexterity Bonus", signed(stats.ac_dex)),
                ("Size Modifier", signed(stats.ac_size)),
                ("Total Armor Class", stats.ac.to_string()),
            ] {
                ui.label(label);
                ui.label(v);
                ui.end_row();
            }
        });
        ui.separator();
        ui.strong("Hit Points");
        egui::Grid::new(("utc-hp", f.key)).num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
            ui.label("Base Hit Points");
            // The current hit points follow the base.
            if let Some(v) = commit_number(ui, f.int("HitPoints"), 1..=10_000) {
                f.set_many(
                    "Hit points",
                    &[
                        ("HitPoints", v, FieldType::Short),
                        ("CurrentHitPoints", v, FieldType::Short),
                    ],
                );
            }
            ui.end_row();
            ui.label("Hit Point Bonuses");
            ui.label(signed(stats.hp_bonus));
            ui.end_row();
            ui.label("Total Hit Points");
            ui.label(stats.max_hit_points.to_string());
            ui.end_row();
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Movement Rate");
            f.choice(ui, "Movement rate", "WalkRate", &speeds, FieldType::Int);
        });

        let ui = &mut cols[1];
        ui.strong("Saves");
        egui::Grid::new(("utc-saves", f.key)).num_columns(5).spacing([16.0, 4.0]).show(ui, |ui| {
            for h in ["", "Base", "Modifier", "Bonus", "Total"] {
                ui.strong(h);
            }
            ui.end_row();
            for (i, (name, label)) in
                [("Fortitude", "fortbonus"), ("Reflex", "refbonus"), ("Will", "willbonus")]
                    .into_iter()
                    .enumerate()
            {
                ui.label(name);
                ui.label(stats.saves_base[i].to_string());
                ui.label(signed(stats.saves_modifier[i]));
                if let Some(v) = commit_number(ui, f.int(label), -100..=100) {
                    f.set_int(name, label, v, FieldType::Short);
                }
                ui.label(stats.saves[i].to_string());
                ui.end_row();
            }
        });
    });
}

fn signed(v: i32) -> String {
    if v > 0 { format!("+{v}") } else { v.to_string() }
}

/// Body parts: (label, field, capart.2da model name).
const BODY_PARTS: [(&str, &str, &str); 18] = [
    ("Head", "Appearance_Head", "head"),
    ("Neck", "BodyPart_Neck", "neck"),
    ("Torso", "BodyPart_Torso", "chest"),
    ("Pelvis", "BodyPart_Pelvis", "pelvis"),
    ("Belt", "BodyPart_Belt", "belt"),
    ("Right Shoulder", "BodyPart_RShoul", "shor"),
    ("Left Shoulder", "BodyPart_LShoul", "shol"),
    ("Right Bicep", "BodyPart_RBicep", "bicepr"),
    ("Left Bicep", "BodyPart_LBicep", "bicepl"),
    ("Right Forearm", "BodyPart_RFArm", "forer"),
    ("Left Forearm", "BodyPart_LFArm", "forel"),
    ("Right Hand", "BodyPart_RHand", "handr"),
    ("Left Hand", "BodyPart_LHand", "handl"),
    ("Right Thigh", "BodyPart_RThigh", "legr"),
    ("Left Thigh", "BodyPart_LThigh", "legl"),
    ("Right Shin", "BodyPart_RShin", "shinr"),
    ("Left Shin", "BodyPart_LShin", "shinl"),
    // The creature's right foot really is `ArmorPart_RFoot`.
    ("Right Foot", "ArmorPart_RFoot", "footr"),
];

/// The part numbers with a model for a body's prefix (`pmh0_chest`), cached.
fn part_numbers(ui: &Ui, game: &GameData, prefix: &str) -> Arc<Vec<u16>> {
    let id = egui::Id::new(("body-parts", prefix));
    if let Some(v) = ui.data(|d| d.get_temp::<Arc<Vec<u16>>>(id)) {
        return v;
    }
    let mut v: Vec<u16> = game
        .resman
        .list(ResType::MDL)
        .iter()
        .filter_map(|r| r.to_string().strip_prefix(prefix)?.parse().ok())
        .collect();
    v.sort_unstable();
    v.dedup();
    let v = Arc::new(v);
    ui.data_mut(|d| d.insert_temp(id, v.clone()));
    v
}

/// The model prefix of a part-based creature's body (`pmh0`), if it is one.
fn body_prefix(f: &Form<'_>, game: &GameData) -> Option<String> {
    let appearance = game.table("appearance").ok()?;
    let row = f.int("Appearance_Type").max(0) as usize;
    if !appearance.get(row, "MODELTYPE")?.to_ascii_uppercase().starts_with('P') {
        return None;
    }
    let race = appearance.get(row, "RACE")?.to_ascii_lowercase();
    let gender = game
        .table("gender")
        .ok()?
        .get(f.int("Gender").max(0) as usize, "GENDER")
        .map(|g| g.to_ascii_lowercase())
        .filter(|g| g == "m" || g == "f")
        .unwrap_or_else(|| "m".into());
    Some(format!("p{gender}{race}{}", f.int("Phenotype").max(0)))
}

fn set_part(f: &mut Form<'_>, what: &str, label: &str, v: i64) {
    let wide = wide_label(label);
    f.set_many(what, &[(label, v.min(255), FieldType::Byte), (&wide, v, FieldType::Word)]);
}

fn appearance(f: &mut Form<'_>, ui: &mut Ui) {
    let Some(game) = f.app.game.take() else { return };
    let prefix = body_prefix(f, &game);
    let wings = game
        .choices("wingmodel", ChoiceColumns { name: None, label: Some("LABEL") })
        .unwrap_or_default();
    let tails = game
        .choices("tailmodel", ChoiceColumns { name: None, label: Some("LABEL") })
        .unwrap_or_default();
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        match &prefix {
            Some(prefix) => {
                egui::Grid::new(("utc-parts", f.key)).num_columns(2).spacing([12.0, 4.0]).show(
                    ui,
                    |ui| {
                        for (text, label, part) in BODY_PARTS {
                            let numbers = part_numbers(ui, &game, &format!("{prefix}_{part}"));
                            let current = part_number(&f.root, label).unwrap_or(0);
                            let choices: Vec<Choice> = numbers
                                .iter()
                                .map(|&n| Choice { row: n as usize, text: n.to_string() })
                                .collect();
                            ui.label(text);
                            if let Some(v) = situated::pick(ui, f.key, label, &choices, current) {
                                set_part(f, text, label, v);
                            }
                            ui.end_row();
                        }
                    },
                );
            }
            None => {
                ui.weak("This appearance is a single model: it has no body parts.");
            }
        }
        let ui = &mut cols[1];
        egui::Grid::new(("utc-extras", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Wings");
            f.choice(ui, "Wings", "Wings_New", &wings, FieldType::Dword);
            ui.end_row();
            ui.label("Tail");
            f.choice(ui, "Tail", "Tail_New", &tails, FieldType::Dword);
            ui.end_row();
            for (text, label) in [
                ("Skin Color", "Color_Skin"),
                ("Hair Color", "Color_Hair"),
                ("Tattoo 1 Color", "Color_Tattoo1"),
                ("Tattoo 2 Color", "Color_Tattoo2"),
            ] {
                ui.label(text);
                f.number(ui, text, label, 0..=175);
                ui.end_row();
            }
            ui.label("");
            situated::preview_button(f, ui);
            ui.end_row();
        });
    });
    f.app.game = Some(game);
}

/// Alignment presets: (name, `GoodEvil`, `LawfulChaotic`).
const ALIGNMENTS: [(&str, i64, i64); 9] = [
    ("Lawful Good", 100, 100),
    ("Neutral Good", 100, 50),
    ("Chaotic Good", 100, 0),
    ("Lawful Neutral", 50, 100),
    ("True Neutral", 50, 50),
    ("Chaotic Neutral", 50, 0),
    ("Lawful Evil", 0, 100),
    ("Neutral Evil", 0, 50),
    ("Chaotic Evil", 0, 0),
];

fn classes(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let all = choices(f, "classes", "Name", "Label");
    let packages = choices(f, "packages", "Name", "Label");
    let list: Vec<Struct> = f.root.list("ClassList").unwrap_or(&[]).to_vec();
    // Alignment: a preset, and the two axes.
    let (ge, lc) = (f.int("GoodEvil"), f.int("LawfulChaotic"));
    ui.horizontal(|ui| {
        ui.label("Alignment");
        let shown = ALIGNMENTS
            .iter()
            .find(|(_, g, l)| *g == ge && *l == lc)
            .map_or_else(|| "(custom)".to_string(), |(n, _, _)| (*n).to_string());
        let mut pick = None;
        egui::ComboBox::from_id_salt(("utc-align", key)).selected_text(shown).show_ui(ui, |ui| {
            for (name, g, l) in ALIGNMENTS {
                if ui.selectable_label(g == ge && l == lc, name).clicked() {
                    pick = Some((g, l));
                }
            }
        });
        if let Some((g, l)) = pick {
            f.set_many(
                "Alignment",
                &[("GoodEvil", g, FieldType::Byte), ("LawfulChaotic", l, FieldType::Byte)],
            );
        }
        ui.label("Good–Evil");
        f.number(ui, "Good-evil", "GoodEvil", 0..=100);
        ui.label("Lawful–Chaotic");
        f.number(ui, "Lawful-chaotic", "LawfulChaotic", 0..=100);
    });
    ui.separator();
    let mut edits: Vec<(&str, Vec<Edit>)> = Vec::new();
    egui::Grid::new(("utc-classes", key)).num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
        for (i, c) in list.iter().enumerate() {
            let path = base.clone().item("ClassList", i);
            ui.label(format!("Class {}", i + 1));
            let class = c.integer("Class").unwrap_or(0);
            if let Some(v) = situated::pick(ui, key, &format!("class{i}"), &all, class) {
                edits.push(("Class", vec![set(key, &path, "Class", Value::Int(v as i32))]));
            }
            let level = c.integer("ClassLevel").unwrap_or(1);
            if let Some(v) = commit_number(ui, level, 1..=60) {
                edits.push((
                    "Class level",
                    vec![set(key, &path, "ClassLevel", Value::Short(v as i16))],
                ));
            }
            if ui.add_enabled(list.len() > 1, egui::Button::new("Remove").small()).clicked() {
                edits.push((
                    "Remove class",
                    vec![Edit::RemoveItem {
                        key,
                        path: base.clone(),
                        list: "ClassList".into(),
                        index: i,
                    }],
                ));
            }
            ui.end_row();
        }
    });
    // EE creatures take up to 8 classes.
    if ui.add_enabled(list.len() < 8, egui::Button::new("Add Class")).clicked() {
        let taken: Vec<i64> = list.iter().filter_map(|c| c.integer("Class")).collect();
        let class = all.iter().map(|c| c.row as i64).find(|r| !taken.contains(r)).unwrap_or(0);
        let mut s = Struct::new(mg_schema::utc::CLASS_LIST.item_id);
        s.set("Class", Value::Int(class as i32));
        s.set("ClassLevel", Value::Short(1));
        edits.push((
            "Add class",
            vec![Edit::InsertItem {
                key,
                path: base.clone(),
                list: "ClassList".into(),
                index: list.len(),
                item: s,
            }],
        ));
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Default Package for Autolevelup");
        f.choice(ui, "Package", "StartingPackage", &packages, FieldType::Byte);
    });
    for (what, e) in edits {
        f.app.actions.push(Action::Apply(Command::new(what, e)));
    }
}

fn set(key: ResKey, path: &GffPath, label: &str, value: Value) -> Edit {
    Edit::SetField { key, path: path.clone(), label: label.into(), value: Some(value) }
}

fn skills(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let skills = choices(f, "skills", "Name", "Label");
    let list: Vec<Struct> = f.root.list("SkillList").unwrap_or(&[]).to_vec();
    let mut edit = None;
    egui::ScrollArea::vertical().id_salt(("utc-skills", key)).show(ui, |ui| {
        egui::Grid::new(("utc-skill-grid", key))
            .num_columns(2)
            .striped(true)
            .spacing([24.0, 4.0])
            .show(ui, |ui| {
                ui.strong("Skill");
                ui.strong("Rank");
                ui.end_row();
                for s in &skills {
                    ui.label(&s.text);
                    let rank = list.get(s.row).and_then(|e| e.integer("Rank")).unwrap_or(0);
                    if let Some(v) = commit_number(ui, rank, 0..=127) {
                        edit = Some((s.row, v));
                    }
                    ui.end_row();
                }
            });
    });
    if let Some((row, v)) = edit {
        // The list is indexed by skill: missing entries up to it are added.
        let mut edits: Vec<Edit> = (list.len()..=row)
            .map(|i| {
                let mut s = Struct::new(mg_schema::utc::SKILL_LIST.item_id);
                s.set("Rank", Value::Byte(0));
                Edit::InsertItem {
                    key,
                    path: base.clone(),
                    list: "SkillList".into(),
                    index: i,
                    item: s,
                }
            })
            .collect();
        edits.push(set(key, &base.item("SkillList", row), "Rank", Value::Byte(v as u8)));
        f.app.actions.push(Action::Apply(Command::new("Skill rank", edits)));
    }
}

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let bags = choices(f, "bodybag", "Name", "LABEL");
    let sounds = choices(f, "soundset", "STRREF", "LABEL");
    let ranges = choices(f, "ranges", "Name", "Label");
    let cr = f.root.float("ChallengeRating").unwrap_or(0.0);
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        egui::Grid::new(("utc-adv", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("Blueprint ResRef");
            f.blueprint_resref(ui);
            ui.end_row();
            ui.label("Faction");
            ui.horizontal(|ui| {
                situated::faction(f, ui, "Faction", "FactionID");
                if ui.small_button("Edit Factions").clicked() {
                    f.app.actions.push(Action::OpenTab(Tab::Factions));
                }
            });
            ui.end_row();
            ui.label("Treasure Model");
            f.choice(ui, "Treasure model", "BodyBag", &bags, FieldType::Byte);
            ui.end_row();
            ui.label("Corpse Decay Time (s)");
            f.millis(ui, "Decay time", "DecayTime", 0.0..=32767.0);
            ui.end_row();
            ui.label("Perception Range");
            f.choice(ui, "Perception range", "PerceptionRange", &ranges, FieldType::Byte);
            ui.end_row();
            ui.label("Sound Set");
            f.choice(ui, "Sound set", "SoundSetFile", &sounds, FieldType::Word);
            ui.end_row();
            ui.label("Subrace");
            f.text(ui, "Subrace", "Subrace", 32);
            ui.end_row();
            ui.label("Deity");
            f.text(ui, "Deity", "Deity", 32);
            ui.end_row();
            ui.label("Variables");
            f.variables(ui);
            ui.end_row();
        });
        let ui = &mut cols[1];
        for (text, label) in [
            ("Leaves Lootable Corpse", "Lootable"),
            ("Disarmable", "Disarmable"),
            ("Plot", "Plot"),
            ("No Permanent Death", "NoPermDeath"),
            ("Immortal", "IsImmortal"),
        ] {
            f.check(ui, text, label);
        }
        ui.separator();
        ui.strong("Challenge Rating");
        egui::Grid::new(("utc-cr", f.key)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Adjustment");
            // The rating is the calculated one plus the adjustment: a new
            // adjustment moves it by the difference.
            let adjust = f.int("CRAdjust");
            if let Some(v) = commit_number(ui, adjust, -100..=100) {
                let rating = (cr + (v - adjust) as f32).max(0.0);
                let adjust_value = super::integer(f.root.get("CRAdjust"), v, FieldType::Int);
                f.set_fields(
                    "CR adjustment",
                    vec![("CRAdjust", adjust_value), ("ChallengeRating", Value::Float(rating))],
                );
            }
            ui.end_row();
            ui.label("Challenge Rating");
            ui.label(format!("{cr}"));
            ui.end_row();
        });
    });
}
