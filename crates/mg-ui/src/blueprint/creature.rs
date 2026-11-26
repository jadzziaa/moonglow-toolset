//! Creature Properties (`TdlgCreatureEdit`): Basic, Statistics, Appearance,
//! Classes, Skills, Feats, Spells, Special Abilities, Inventory, Scripts,
//! Advanced, Variables, Visuals, Comments.
//!
//! Every change also stores the creature's recomputed `MaxHitPoints` and
//! `ChallengeRating` (in the same undoable command), as Aurora does on OK.

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath, lists};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;
use mg_rules::creatures::{ABILITIES, modifier};
use mg_rules::items::part_number;
use mg_rules::{Challenge, Choice, CreatureSheet, GameData};
use mg_uikit::Controls as _;

use super::{Form, creature_lists, situated};
use crate::widgets::{commit_number, spin_number};
use crate::{Action, Moonglow, Tab};

pub(super) const PAGES: [&str; 14] = [
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
    "Variables",
    "Visuals",
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
        "Visuals" => super::visuals::page(f, ui, true),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

/// After a command: the `MaxHitPoints` and `ChallengeRating` of each
/// creature it changed, recomputed, as part of the same command.
pub(crate) fn refresh_hit_points(
    app: &mut Moonglow,
    objects: Vec<(mg_resman::ResKey, mg_edit::GffPath)>,
) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
    let mut edits = Vec::new();
    for (key, path) in objects {
        let Ok(g) = ws.doc(&key) else { continue };
        let Some(creature) = path.get(&g.root).cloned() else { continue };
        let creature = &creature;
        let stats = game.creature_stats(&CreatureSheet::from_gff(creature));
        let max = stats.max_hit_points;
        // The current hit points with the Great Constitution feats', which
        // the game leaves out of those it starts the creature with, where
        // it has them. A creature with fewer than its base is left
        // wounded, and one without them as it is (a stock creature has
        // more than its base without them: `tm_cr_dragblksml`).
        let base = creature.integer("HitPoints").unwrap_or(0);
        let current = creature.integer("CurrentHitPoints");
        if stats.great_hit_points != 0
            && current.is_some_and(|c| c >= base)
            && current != Some(i64::from(stats.current_hit_points))
        {
            edits.push(Edit::SetField {
                key,
                path: path.clone(),
                label: "CurrentHitPoints".into(),
                value: Some(super::integer(
                    creature.get("CurrentHitPoints"),
                    stats.current_hit_points.into(),
                    FieldType::Short,
                )),
            });
        }
        if creature.integer("MaxHitPoints") != Some(i64::from(max)) {
            let old = creature.get("MaxHitPoints");
            edits.push(Edit::SetField {
                key,
                path: path.clone(),
                label: "MaxHitPoints".into(),
                value: Some(super::integer(old, max.into(), FieldType::Short)),
            });
        }
        let rating = challenge(game, &ws.module, creature).rating;
        if creature.float("ChallengeRating") != Some(rating) {
            edits.push(Edit::SetField {
                key,
                path,
                label: "ChallengeRating".into(),
                value: Some(Value::Float(rating)),
            });
        }
    }
    if !edits.is_empty()
        && let Err(e) = ws.amend(edits)
    {
        app.log.error(e.to_string());
    }
}

/// A creature's challenge rating, its gear read from the module's item
/// blueprints, else the game's.
fn challenge(game: &GameData, module: &mg_module::Module, creature: &Struct) -> Challenge {
    let item =
        |r: mg_core::ResRef| mg_module::gff_root(Some(module), game, &ResKey::new(r, ResType::UTI));
    let mut sheet = CreatureSheet::from_gff(creature);
    sheet.gear_value = game.gear_value(creature, &item);
    game.challenge(&sheet)
}

/// The stored rating as Aurora shows it (fractions below 1); under the
/// pointer, the calculation behind it (worked out then: it reads every
/// item the creature wears).
fn rating_label(f: &Form<'_>, ui: &mut Ui) {
    let stored = f.root.float("ChallengeRating").unwrap_or(0.0);
    let r = ui.label(Challenge { calculated: stored, rating: stored }.text());
    if let (Some(game), Some(ws)) = (f.app.game.as_deref(), f.app.ws.as_ref()) {
        r.on_hover_ui(|ui| {
            let c = challenge(game, &ws.module, &f.root);
            ui.set_max_width(ui.spacing().tooltip_width);
            ui.label(format!("Calculated {:.2}, rated {}", c.calculated, c.text()));
        });
    }
}

/// The Basic page's sections, in order (the Creature board's): down the
/// first column and on down the next where the page has two.
const BASIC_SECTIONS: [&str; 5] =
    ["Identity", "Race and Appearance", "Palette", "Portrait", "Conversation"];

fn basic(f: &mut Form<'_>, ui: &mut Ui) {
    // (Those with a name, as Aurora and the Creature Wizard list them: a
    // row without one is `DELETED`, `INVALID_RACE`, CEP's `CEP_RESERVED`.)
    let races = f.choices("racialtypes", Some("Name"), None);
    let appearances = f.choices("appearance", Some("STRING_REF"), Some("LABEL"));
    let appearances = f.appearance_rows(ui.ctx(), mg_rules::ObjectKind::Creature, &appearances);
    let phenotypes = f.choices("phenotype", Some("Name"), Some("Label"));
    let genders = f.choices("gender", Some("NAME"), Some("GENDER"));
    let key = f.key;
    mg_uikit::form::FormColumns::new(f.id("utc-basic")).show(ui, BASIC_SECTIONS.len(), |ui, s| {
        super::section(ui, BASIC_SECTIONS[s], ("utc-basic", key, s), |ui| match s {
            0 => {
                for (text, what, label, last) in [
                    ("First name", "First name", "FirstName", false),
                    ("Last name", "Last name", "LastName", true),
                ] {
                    crate::widgets::field_label(ui, text);
                    f.locstring_then(ui, what, label, |f, ui| {
                        random_name_button(f, ui, what, label, last);
                    });
                    ui.end_row();
                }
                crate::widgets::field_label(ui, "Tag");
                f.tag(ui, "Tag", "Tag");
                ui.end_row();
            }
            1 => {
                crate::widgets::field_label(ui, "Race");
                f.choice(ui, "Race", "Race", &races, FieldType::Byte);
                ui.end_row();
                crate::widgets::field_label(ui, "Appearance");
                super::row(ui, |ui| {
                    let before = f.app.actions.len();
                    f.appearance_choice(ui, "Appearance_Type", &appearances, FieldType::Word, true);
                    if f.app.actions.len() > before
                        && let Some(Action::Apply(mut set)) = f.app.actions.pop()
                    {
                        with_a_body(f.app, &mut set.edits);
                        with_its_portrait(f.app, &mut set.edits);
                        f.app.actions.push(Action::Apply(set));
                    }
                    f.gallery_button(ui, crate::appearance_gallery::Kind::Creature);
                });
                ui.end_row();
                f.model_warning(ui, mg_rules::ObjectKind::Creature);
                crate::widgets::field_label(ui, "Phenotype");
                f.choice(ui, "Phenotype", "Phenotype", &phenotypes, FieldType::Int);
                ui.end_row();
                crate::widgets::field_label(ui, "Gender");
                f.choice(ui, "Gender", "Gender", &genders, FieldType::Byte);
                ui.end_row();
                // (Aurora has it on Advanced: here with the gender, as
                // builders asked.)
                crate::widgets::field_label(ui, "Sound set");
                sound_set(f, ui);
                ui.end_row();
                crate::widgets::field_label(ui, "");
                situated::preview_button(f, ui);
                ui.end_row();
            }
            2 => {
                crate::widgets::field_label(ui, "Challenge rating");
                rating_label(f, ui);
                ui.end_row();
                crate::widgets::field_label(ui, "Category");
                f.category(ui, BlueprintKind::Creature);
                ui.end_row();
                // (Aurora has it on Advanced: here, as builders asked.)
                crate::widgets::field_label(ui, "Faction");
                super::row(ui, |ui| {
                    situated::faction(f, ui, "Faction", "FactionID");
                    if ui.small_button("Edit Factions").clicked() {
                        f.app.actions.push(Action::OpenTab(Tab::Factions));
                    }
                });
                ui.end_row();
            }
            3 => {
                crate::widgets::field_label(ui, "Portrait");
                situated::portrait(f, ui);
                ui.end_row();
            }
            _ => {
                crate::widgets::field_label(ui, "Conversation");
                f.conversation(ui);
                ui.end_row();
                crate::widgets::field_label(ui, "");
                f.mark_mixed(ui, "Interruptable");
                let mut no_interrupt = f.root.integer("Interruptable").unwrap_or(1) == 0;
                if ui.check_box(&mut no_interrupt, "No interrupt").changed() {
                    let v = i64::from(!no_interrupt);
                    f.set_int("No interrupt", "Interruptable", v, FieldType::Byte);
                }
                ui.end_row();
            }
        });
    });
    // (The description across the page, under the columns.)
    ui.add_space(mg_uikit::form::SECTIONS_GAP);
    f.locstring_memo(ui, "Description", "Description");
}

/// The dice after a name: a random name from the race's letter tables.
fn random_name_button(f: &mut Form<'_>, ui: &mut Ui, what: &str, label: &str, last: bool) {
    if !crate::widgets::icon_button(ui, mg_uikit::Icon::Dice, "Random Name", "Random name")
        .clicked()
    {
        return;
    }
    let race = f.root.integer("Race").unwrap_or(6).max(0) as u32;
    let gender = f.root.integer("Gender").unwrap_or(0).clamp(0, 255) as u8;
    let name = f
        .app
        .game
        .as_deref()
        .and_then(|g| g.random_name(race, gender, last, &mut fastrand::Rng::new()));
    if let Some(n) = name {
        let current = f.root.locstring(label).cloned().unwrap_or_default();
        f.set_typed(what, label, Value::LocString(crate::text::with_edited_text(current, &n)));
    }
}

fn statistics(f: &mut Form<'_>, ui: &mut Ui) {
    let Some(stats) =
        f.app.game.as_deref().map(|g| g.creature_stats(&CreatureSheet::from_gff(&f.root)))
    else {
        return;
    };
    let speeds = f.choices("creaturespeed", Some("Name"), Some("Label"));
    // (Ability Scores and Saves, then Armor Class and Hit Points, where
    // the page has two columns, as the Creature board; in one, Ability
    // Scores, Armor Class, Hit Points, Saves, as WideCreature.)
    let columns = mg_uikit::form::form_columns(ui.available_width());
    let order: [usize; 4] = if columns > 1 { [0, 3, 1, 2] } else { [0, 1, 2, 3] };
    mg_uikit::form::FormColumns::new(f.id("utc-statistics")).columns(columns).keys(order).show(
        ui,
        4,
        |ui, s| {
            match order[s] {
                0 => {
                    crate::widgets::section_heading(ui, "Ability Scores");
                    // (In a narrow room, the Properties panel's: the total and its bonus
                    // in one column, "12 (+1)", the racial modifier in its tip.)
                    let narrow = super::narrow(ui);
                    egui::Grid::new(("utc-abilities", f.key, narrow))
                        .num_columns(if narrow { 3 } else { 5 })
                        .spacing([if narrow { 8.0 } else { 16.0 }, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            let heads: &[&str] = match narrow {
                                true => &["", "Base", "Total"],
                                false => &["", "Base", "Racial modifier", "Total", "Bonus"],
                            };
                            for h in heads {
                                ui.strong(*h);
                            }
                            ui.end_row();
                            let names = [
                                "Strength",
                                "Dexterity",
                                "Constitution",
                                "Intelligence",
                                "Wisdom",
                                "Charisma",
                            ];
                            for (i, (name, label)) in names.iter().zip(ABILITIES).enumerate() {
                                crate::widgets::field_label(ui, *name);
                                f.spin(ui, name, label, 3..=100);
                                let (racial, total) = (signed(stats.racial[i]), stats.totals[i]);
                                let bonus = signed(modifier(total));
                                if narrow {
                                    ui.label(format!("{total} ({bonus})")).on_hover_text(format!(
                                        "Racial modifier {racial}, bonus {bonus}"
                                    ));
                                } else {
                                    ui.label(racial);
                                    ui.label(total.to_string());
                                    ui.label(bonus);
                                }
                                ui.end_row();
                            }
                        });
                }
                1 => {
                    crate::widgets::section_heading(ui, "Armor Class");
                    super::form_grid(ui, ("utc-ac", f.key)).labels().spacing([16.0, 4.0]).show(
                        ui,
                        |ui| {
                            crate::widgets::field_label(ui, "Natural AC");
                            // (Aurora takes up to 1000 here, more than the byte the file
                            // keeps; a byte is the most that is kept.)
                            f.spin(ui, "Natural AC", "NaturalAC", 0..=255);
                            ui.end_row();
                            for (label, v) in [
                                ("Base", "10".to_string()),
                                ("Dexterity bonus", signed(stats.ac_dex)),
                                ("Size modifier", signed(stats.ac_size)),
                                ("Total armor class", stats.ac.to_string()),
                            ] {
                                crate::widgets::field_label(ui, label);
                                ui.label(v);
                                ui.end_row();
                            }
                        },
                    );
                }
                2 => {
                    crate::widgets::section_heading(ui, "Hit Points");
                    super::form_grid(ui, ("utc-hp", f.key)).labels().spacing([16.0, 4.0]).show(
                        ui,
                        |ui| {
                            crate::widgets::field_label(ui, "Base hit points");
                            // The current hit points follow the base.
                            f.mark_mixed(ui, "HitPoints");
                            if let Some(v) = spin_number(ui, f.int("HitPoints"), 1..=10_000) {
                                f.set_many(
                                    "Hit points",
                                    &[
                                        ("HitPoints", v, FieldType::Short),
                                        ("CurrentHitPoints", v, FieldType::Short),
                                    ],
                                );
                            }
                            ui.end_row();
                            crate::widgets::field_label(ui, "Hit point bonuses");
                            ui.label(signed(stats.hp_bonus));
                            ui.end_row();
                            crate::widgets::field_label(ui, "Total hit points");
                            ui.label(stats.max_hit_points.to_string());
                            ui.end_row();
                            crate::widgets::field_label(ui, "Movement rate");
                            f.choice(ui, "Movement rate", "WalkRate", &speeds, FieldType::Int);
                            ui.end_row();
                        },
                    );
                }
                _ => {
                    crate::widgets::section_heading(ui, "Saves");
                    // (Narrow: the base and the modifier in the total's tip.)
                    let narrow = super::narrow(ui);
                    let grid = egui::Grid::new(("utc-saves", f.key, narrow));
                    let grid = grid.num_columns(if narrow { 3 } else { 5 });
                    grid.spacing([if narrow { 8.0 } else { 16.0 }, 4.0]).show(ui, |ui| {
                        let heads: &[&str] = match narrow {
                            true => &["", "Bonus", "Total"],
                            false => &["", "Base", "Modifier", "Bonus", "Total"],
                        };
                        for h in heads {
                            ui.strong(*h);
                        }
                        ui.end_row();
                        for (i, (name, label)) in [
                            ("Fortitude", "fortbonus"),
                            ("Reflex", "refbonus"),
                            ("Will", "willbonus"),
                        ]
                        .into_iter()
                        .enumerate()
                        {
                            crate::widgets::field_label(ui, name);
                            if !narrow {
                                ui.label(stats.saves_base[i].to_string());
                                ui.label(signed(stats.saves_modifier[i]));
                            }
                            // (To 250, as Aurora; below 0, which Aurora's field doesn't
                            // go, for the penalties files have.)
                            f.mark_mixed(ui, label);
                            if let Some(v) = spin_number(ui, f.int(label), -100..=250) {
                                f.set_int(name, label, v, FieldType::Short);
                            }
                            let base = (stats.saves_base[i], signed(stats.saves_modifier[i]));
                            let total = ui.label(stats.saves[i].to_string());
                            if narrow {
                                total
                                    .on_hover_text(format!("Base {}, modifier {}", base.0, base.1));
                            }
                            ui.end_row();
                        }
                    });
                }
            }
        },
    );
}

fn signed(v: i32) -> String {
    if v > 0 { format!("+{v}") } else { v.to_string() }
}

/// Body parts: (label, field, capart.2da model name).
const BODY_PARTS: [(&str, &str, &str); 19] = [
    ("Head", "Appearance_Head", "head"),
    ("Neck", "BodyPart_Neck", "neck"),
    ("Torso", "BodyPart_Torso", "chest"),
    ("Pelvis", "BodyPart_Pelvis", "pelvis"),
    ("Belt", "BodyPart_Belt", "belt"),
    ("Right shoulder", "BodyPart_RShoul", "shor"),
    ("Left shoulder", "BodyPart_LShoul", "shol"),
    ("Right bicep", "BodyPart_RBicep", "bicepr"),
    ("Left bicep", "BodyPart_LBicep", "bicepl"),
    ("Right forearm", "BodyPart_RFArm", "forer"),
    ("Left forearm", "BodyPart_LFArm", "forel"),
    ("Right hand", "BodyPart_RHand", "handr"),
    ("Left hand", "BodyPart_LHand", "handl"),
    ("Right thigh", "BodyPart_RThigh", "legr"),
    ("Left thigh", "BodyPart_LThigh", "legl"),
    ("Right shin", "BodyPart_RShin", "shinr"),
    ("Left shin", "BodyPart_LShin", "shinl"),
    // The creature's right foot really is `ArmorPart_RFoot`.
    ("Right foot", "ArmorPart_RFoot", "footr"),
    ("Left foot", "BodyPart_LFoot", "footl"),
];

/// A body for a creature given a part-based appearance that has none: to
/// each edit that sets `Appearance_Type` to a body of parts, on a creature
/// that keeps no body part at all (one made with a single model: the
/// game's bandit), the parts are added as a creature new in Aurora has
/// them: 1, and no belt or shoulders. The game draws no body for such a
/// creature without them (`client_render.rs`, `MG_NO_BODY`), which an
/// armor hides until it is taken off, and no head without its
/// `Appearance_Head` (`heads_look`).
pub(crate) fn with_a_body(app: &mut Moonglow, edits: &mut Vec<Edit>) {
    let Some(game) = app.game.clone() else { return };
    let Ok(appearance) = game.table("appearance") else { return };
    let Some(ws) = app.ws.as_mut() else { return };
    let mut more = Vec::new();
    for e in edits.iter() {
        let Edit::SetField { key, path, label, value: Some(value) } = e else { continue };
        let mut set = Struct::new(0);
        set.set(label, value.clone());
        let row = set.integer("Appearance_Type");
        let parts = row.and_then(|r| appearance.get(usize::try_from(r).ok()?, "MODELTYPE"));
        if !parts.is_some_and(|t| t.to_ascii_uppercase().starts_with('P')) {
            continue;
        }
        let Ok(doc) = ws.doc(key) else { continue };
        let Some(creature) = path.get(&doc.root) else { continue };
        if BODY_PARTS.iter().any(|(_, field, _)| part_number(creature, field).is_some()) {
            continue;
        }
        for (_, field, mdlname) in BODY_PARTS {
            let bare = matches!(mdlname, "belt" | "shor" | "shol");
            more.push(Edit::SetField {
                key: *key,
                path: path.clone(),
                label: field.to_string(),
                value: Some(Value::Byte(u8::from(!bare))),
            });
        }
    }
    edits.extend(more);
}

/// The portrait of a creature's new appearance, with Options › General ›
/// Set a creature's portrait with its appearance (off by default): to
/// each edit that sets `Appearance_Type`,
/// the portraits.2da row its appearance.2da row names
/// ([`mg_rules::appearance::appearance_portrait`]), in `PortraitId` and,
/// where the creature keeps one (older files), `Portrait`, as Select
/// Portrait sets them. A row that names none, or none portraits.2da has,
/// leaves the portrait as it is.
pub(crate) fn with_its_portrait(app: &mut Moonglow, edits: &mut Vec<Edit>) {
    if !app.settings.appearance_portrait {
        return;
    }
    let Some(game) = app.game.clone() else { return };
    let (Ok(appearance), Ok(portraits)) = (game.table("appearance"), game.table("portraits"))
    else {
        return;
    };
    let Some(ws) = app.ws.as_mut() else { return };
    let mut more = Vec::new();
    for e in edits.iter() {
        let Edit::SetField { key, path, label, value: Some(value) } = e else { continue };
        let mut set = Struct::new(0);
        set.set(label, value.clone());
        let Some(row) = set.integer("Appearance_Type").and_then(|r| usize::try_from(r).ok()) else {
            continue;
        };
        let Some(portrait) =
            mg_rules::appearance::appearance_portrait(&appearance, &portraits, row)
        else {
            continue;
        };
        let Ok(doc) = ws.doc(key) else { continue };
        let Some(creature) = path.get(&doc.root) else { continue };
        let field = |label: &str, value| Edit::SetField {
            key: *key,
            path: path.clone(),
            label: label.to_string(),
            value: Some(value),
        };
        let id = super::integer(creature.get("PortraitId"), portrait as i64, FieldType::Word);
        more.push(field("PortraitId", id));
        let base = portraits.get(portrait, "BaseResRef").unwrap_or_default().to_lowercase();
        if creature.get("Portrait").is_some()
            && let Ok(r) = ResRef::from_str(&format!("po_{base}"))
        {
            more.push(field("Portrait", Value::resref(r)));
        }
    }
    edits.extend(more);
}

/// The model prefixes of a part-based creature's body, if it is one: its
/// phenotype's skeleton (`pmh7`), then the phenotype's `DefaultPhenoType`'s
/// (`pmh0`), where the game and the preview look for a part the first
/// lacks ([`mg_rules::models::skeletons`]).
fn body_prefix(f: &Form<'_>, game: &GameData) -> Option<[String; 2]> {
    let appearance = game.table("appearance").ok()?;
    let (gender, phenotype) = (game.table("gender").ok(), game.table("phenotype").ok());
    mg_rules::models::skeletons(&f.root, &appearance, gender.as_deref(), phenotype.as_deref())
}

/// The Appearance page: the wings, tail and colors, and the body parts
/// where the appearance has them; what there is to say about the
/// appearance over them. (The creature as it looks now is the Preview
/// panel's, M4's U11: the Creature board's Right Dock.)
fn appearance(f: &mut Form<'_>, ui: &mut Ui) {
    let Some(game) = f.app.game.clone() else { return };
    let prefix = body_prefix(f, &game);
    if prefix.is_none() {
        ui.weak("This appearance is a single model: it has no body parts.");
    }
    appearance_fields(f, ui, &game, prefix.as_ref());
}

/// The Appearance page's fields: wings, tail and colors, then the body
/// parts of an appearance that has them (`prefix`: its parts' models',
/// [`body_prefix`]).
fn appearance_fields(f: &mut Form<'_>, ui: &mut Ui, game: &GameData, prefix: Option<&[String; 2]>) {
    let wings = f.choices("wingmodel", None, Some("LABEL"));
    let tails = f.choices("tailmodel", None, Some("LABEL"));
    super::form_grid(ui, ("utc-extras", f.key)).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Wings");
        f.choice(ui, "Wings", "Wings_New", &wings, FieldType::Dword);
        ui.end_row();
        crate::widgets::field_label(ui, "Tail");
        f.choice(ui, "Tail", "Tail_New", &tails, FieldType::Dword);
        ui.end_row();
        for (text, label, palette) in [
            ("Skin color", "Color_Skin", "pal_skin01"),
            ("Hair color", "Color_Hair", "pal_hair01"),
            ("Tattoo 1 color", "Color_Tattoo1", "pal_tattoo01"),
            ("Tattoo 2 color", "Color_Tattoo2", "pal_tattoo01"),
        ] {
            crate::widgets::field_label(ui, text);
            f.palette_color(ui, game, text, label, palette);
            ui.end_row();
        }
    });
    let Some(prefix) = prefix else { return };
    ui.add_space(crate::widgets::SECTION_GAP);
    crate::widgets::section_heading(ui, "Body Parts");
    let parts_choices: std::sync::Arc<Vec<Vec<mg_rules::Choice>>> = super::kept_for_key(
        ui.ctx(),
        egui::Id::new(("utc-body-parts", prefix)),
        &f.app.load_order,
        || {
            let single = prefix[0] == prefix[1];
            let load_order = f.app.load_order;
            let built: Vec<Vec<mg_rules::Choice>> = BODY_PARTS
                .iter()
                .map(|(_, _, part)| {
                    let mut numbers: Vec<u16> = Vec::new();
                    for skeleton in prefix.iter().take(if single { 1 } else { 2 }) {
                        let models = format!("{skeleton}_{part}");
                        numbers.extend(
                            super::item::model_numbers(ui, game, load_order, &models).iter(),
                        );
                    }
                    numbers.sort_unstable();
                    numbers.dedup();
                    situated::part_choices(&numbers, |n| n.to_string())
                })
                .collect();
            std::sync::Arc::new(built)
        },
    );
    super::form_grid(ui, ("utc-parts", f.key)).spacing([12.0, 4.0]).show(ui, |ui| {
        for ((text, label, _), choices) in BODY_PARTS.into_iter().zip(parts_choices.iter()) {
            let current = part_number(&f.root, label).unwrap_or(0);
            crate::widgets::field_label(ui, text);
            f.mark_mixed(ui, label);
            if let Some(v) = situated::pick_part(ui, f.picks(), label, choices, current, 220.0) {
                super::item::set_part(f, text, &[label], v);
            }
            ui.end_row();
        }
    });
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
    let all = f.choices("classes", Some("Name"), Some("Label"));
    let packages = f.choices("packages", Some("Name"), Some("Label"));
    let list: Vec<Struct> = f.root.list("ClassList").unwrap_or(&[]).to_vec();
    let domains = f.choices("domains", Some("Name"), Some("Label"));
    let schools = f.choices("spellschools", Some("StringRef"), Some("Label"));
    // What classes.2da says a class picks or has.
    let classes_2da = f.app.game.as_deref().and_then(|g| g.table("classes").ok());
    let class_int = |class: i64, column: &str| {
        let t = classes_2da.as_ref()?;
        t.get_int(usize::try_from(class).ok()?, column).map(i64::from)
    };
    // Alignment: a preset, and the two axes (WideCreature's sections:
    // Alignment, Classes, Associates).
    let (ge, lc) = (f.int("GoodEvil"), f.int("LawfulChaotic"));
    super::section(ui, "Alignment", ("utc-alignment", key), |ui| {
        crate::widgets::field_label(ui, "Alignment");
        let shown = ALIGNMENTS
            .iter()
            .find(|(_, g, l)| *g == ge && *l == lc)
            .map_or_else(|| "(custom)".to_string(), |(n, _, _)| (*n).to_string());
        // (A custom alignment is none of them: the arrows go to the first.)
        let mut pick = (ge, lc);
        let presets = ALIGNMENTS.map(|(_, g, l)| (g, l));
        crate::widgets::dropdown(("utc-align", key)).selected_text(shown).show(
            ui,
            &mut pick,
            &presets,
            |p| ALIGNMENTS.iter().find(|a| (a.1, a.2) == *p).map_or("", |a| a.0),
        );
        if pick != (ge, lc) {
            let (g, l) = pick;
            f.set_many(
                "Alignment",
                &[("GoodEvil", g, FieldType::Byte), ("LawfulChaotic", l, FieldType::Byte)],
            );
        }
        ui.end_row();
        crate::widgets::field_label(ui, "Good–Evil");
        f.number(ui, "Good-evil", "GoodEvil", 0..=100);
        ui.end_row();
        crate::widgets::field_label(ui, "Lawful–Chaotic");
        f.number(ui, "Lawful-chaotic", "LawfulChaotic", 0..=100);
        ui.end_row();
    });
    ui.add_space(mg_uikit::form::SECTIONS_GAP);
    crate::widgets::section_heading(ui, "Classes");
    if f.several() {
        classes_together(f, ui, &all, &domains, &schools, &class_int);
    } else {
        class_list(f, ui, &list, &all, &domains, &schools, &class_int);
    }
    if ui
        .add_enabled(!f.several(), egui::Button::new("Levelup Wizard…"))
        .on_hover_text("Level the creature up by its classes' packages")
        .on_disabled_hover_text("It levels one creature up at a time")
        .clicked()
    {
        crate::levelup_view::open(f.app, key, base);
    }
    ui.add_space(mg_uikit::form::SECTIONS_GAP);
    crate::widgets::section_heading(ui, "Associates");
    associates(f, ui, &list, &class_int);
    super::row(ui, |ui| {
        crate::widgets::field_label(ui, "Default package for autolevelup");
        f.choice(ui, "Package", "StartingPackage", &packages, FieldType::Byte);
    });
}

/// One creature's classes: each with its level, a cleric's domains, a
/// wizard's school; Add Class.
fn class_list(
    f: &mut Form<'_>,
    ui: &mut Ui,
    list: &[Struct],
    all: &[Choice],
    domains: &[Choice],
    schools: &[Choice],
    class_int: &dyn Fn(i64, &str) -> Option<i64>,
) {
    let base = f.path.clone();
    let key = f.key;
    let picks = f.picks();
    // (This one alone, as the rule for several takes them.)
    let this = f.list_targets();
    let mut refused = None;
    let mut edits: Vec<(&str, Vec<Edit>)> = Vec::new();
    // (In a narrow room, the Properties panel's: the level and Remove on a
    // row under the class.)
    let narrow = super::narrow(ui);
    let grid =
        egui::Grid::new(("utc-classes", key, narrow)).num_columns(if narrow { 2 } else { 4 });
    grid.spacing([12.0, 6.0]).show(ui, |ui| {
        for (i, c) in list.iter().enumerate() {
            let path = base.clone().item("ClassList", i);
            ui.label(format!("Class {}", i + 1));
            let class = c.integer("Class").unwrap_or(0);
            if let Some(v) = situated::pick(ui, picks, &format!("class{i}"), all, class) {
                // By the rule several creatures' classes are changed by
                // (`lists::change_class`): not to a class it has already,
                // which this let through, and in the field's own type.
                match i32::try_from(v) {
                    Ok(to) => {
                        let out = lists::change_class(&this, class, to);
                        refused = out.left.first().map(|why| format!("Class not changed: {why}"));
                        if !out.edits.is_empty() {
                            edits.push(("Class", out.edits));
                        }
                    }
                    Err(_) => refused = Some(format!("Class not changed: row {v} is no class")),
                }
            }
            let level = c.integer("ClassLevel").unwrap_or(1);
            let mut row_edits: Vec<(&str, Vec<Edit>)> = Vec::new();
            let mut level_and_remove = |ui: &mut Ui| {
                if let Some(v) = spin_number(ui, level, 1..=60) {
                    row_edits.push((
                        "Class level",
                        vec![set(key, &path, "ClassLevel", Value::Short(v as i16))],
                    ));
                }
                if ui.add_enabled(list.len() > 1, egui::Button::new("Remove").small()).clicked() {
                    row_edits.push((
                        "Remove class",
                        vec![Edit::remove_item(key, base.clone(), "ClassList", i)],
                    ));
                }
            };
            if narrow {
                ui.end_row();
                ui.label("");
                ui.horizontal(|ui| level_and_remove(ui));
            } else {
                level_and_remove(ui);
            }
            edits.extend(row_edits);
            ui.end_row();
            // A cleric's domains, a wizard's school (classes.2da PickDomains,
            // PickSchool); the game reads them (engine_ee_fields.rs). On a
            // row of their own, under the class: beside it, the second
            // domain was at a narrow window's edge.
            if class_int(class, "PickDomains") == Some(1) {
                ui.label("Domains");
                super::row(ui, |ui| {
                    for (n, label) in ["Domain1", "Domain2"].into_iter().enumerate() {
                        let name = format!("class{i}-domain{n}");
                        if let Some(v) = optional_pick(ui, picks, &name, domains, c.integer(label))
                        {
                            match byte_row(v, "domains.2da") {
                                Ok(row) => {
                                    edits.push(("Domain", vec![set(key, &path, label, row)]))
                                }
                                Err(why) => refused = Some(why),
                            }
                        }
                    }
                });
                ui.end_row();
            } else if class_int(class, "PickSchool") == Some(1) {
                ui.label("School");
                let current = c.integer("School");
                if let Some(v) =
                    optional_pick(ui, picks, &format!("class{i}-school"), schools, current)
                {
                    match byte_row(v, "spellschools.2da") {
                        Ok(row) => edits.push(("School", vec![set(key, &path, "School", row)])),
                        Err(why) => refused = Some(why),
                    }
                }
                ui.end_row();
            }
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
            vec![Edit::insert_item(key, base.clone(), "ClassList", list.len(), s)],
        ));
    }
    if let Some(why) = refused {
        f.app.log.warn(why);
    }
    for (what, e) in edits {
        f.app.actions.push(Action::Apply(Command::new(what, e)));
    }
}

/// The classes of several creatures edited together: every class any of
/// them has, ≠ with how many where some do. A level, domain or school is
/// set on each that has the class; the class changed is changed on each
/// (not where it has the new one already); Remove takes it from each
/// (but a creature's only class); Add Class and Give to All add it where
/// it lacks and there is room.
fn classes_together(
    f: &mut Form<'_>,
    ui: &mut Ui,
    all: &[Choice],
    domains: &[Choice],
    schools: &[Choice],
    class_int: &dyn Fn(i64, &str) -> Option<i64>,
) {
    let key = f.key;
    let picks = f.picks();
    let targets = f.list_targets();
    let total = targets.len();
    let merged = lists::merged(&targets, "ClassList", lists::class);
    let mut change: Option<(&str, lists::Outcome, &str)> = None;
    // (In a narrow room: the level and the buttons on a row under the
    // class.)
    let narrow = super::narrow(ui);
    let grid =
        egui::Grid::new(("utc-classes", key, narrow)).num_columns(if narrow { 2 } else { 4 });
    grid.spacing([12.0, 6.0]).show(ui, |ui| {
        for (i, m) in merged.iter().enumerate() {
            let class = m.key;
            let is = move |e: &Struct| lists::class(e) == Some(class);
            let c = &m.entry;
            ui.label(format!("Class {}", i + 1));
            if let Some(v) = situated::pick(ui, picks, &format!("class{i}"), all, class) {
                change = Some(("Class", lists::change_class(&targets, class, v as i32), "Set on"));
            }
            let level = c.integer("ClassLevel").unwrap_or(1);
            let level_of = |e: &Struct| e.integer("ClassLevel");
            let differs = lists::differs(&targets, "ClassList", is, level_of);
            if narrow {
                ui.end_row();
                ui.label("");
            }
            super::row(ui, |ui| {
                f.mark_if(ui, &format!("ClassLevel#{class}"), differs);
                if let Some(v) = spin_number(ui, level, 1..=60) {
                    let out = lists::set(&targets, "ClassList", is, &["ClassLevel"], |e, l| {
                        Some(lists::like(e.get(l), v, Value::Short(0)))
                    });
                    change = Some(("Class level", out, "Set on"));
                }
            });
            if narrow {
                ui.end_row();
                ui.label("");
            }
            super::row(ui, |ui| {
                if ui.small_button("Remove").clicked() {
                    change = Some((
                        "Remove class",
                        lists::remove_class(&targets, class as i32),
                        "Taken from",
                    ));
                }
                super::entry_mark(ui, m.having.len(), total);
                if !m.all(total)
                    && ui
                        .small_button("Give to All")
                        .on_hover_text(super::creature_lists::GIVE_TIP)
                        .clicked()
                {
                    let out = lists::add_class(&targets, class as i32, level as i16);
                    change = Some(("Add class", out, "Added to"));
                }
            });
            ui.end_row();
            // A cleric's domains, a wizard's school, set on each that has
            // the class.
            let mut picked = |ui: &mut Ui, name: &str, label: &str, choices: &[Choice]| {
                let differs = lists::differs(&targets, "ClassList", is, |e| e.integer(label));
                f.mark_if(ui, &format!("{label}#{class}"), differs);
                if let Some(v) = optional_pick(ui, picks, name, choices, c.integer(label)) {
                    let (what, table) = match label {
                        "School" => ("School", "spellschools.2da"),
                        _ => ("Domain", "domains.2da"),
                    };
                    match byte_row(v, table) {
                        Ok(row) => {
                            let out = lists::set(&targets, "ClassList", is, &[label], |_, _| {
                                Some(row.clone())
                            });
                            change = Some((what, out, "Set on"));
                        }
                        Err(why) => f.app.log.warn(why),
                    }
                }
            };
            if class_int(class, "PickDomains") == Some(1) {
                ui.label("Domains");
                super::row(ui, |ui| {
                    for (n, label) in ["Domain1", "Domain2"].into_iter().enumerate() {
                        picked(ui, &format!("class{i}-domain{n}"), label, domains);
                    }
                });
                ui.end_row();
            } else if class_int(class, "PickSchool") == Some(1) {
                ui.label("School");
                picked(ui, &format!("class{i}-school"), "School", schools);
                ui.end_row();
            }
        }
    });
    // A class none of them has, added to each with room.
    if ui.button("Add Class").clicked() {
        let class = all.iter().map(|c| c.row as i64).find(|r| !merged.iter().any(|m| m.key == *r));
        let out = lists::add_class(&targets, class.unwrap_or(0) as i32, 1);
        change = Some(("Add class", out, "Added to"));
    }
    if let Some((what, out, done)) = change {
        f.apply_each(what, out, done);
    }
}

/// Row `row` of `table` as a class's domain or school is stored, a BYTE;
/// or why it can't be: a row past 255 (a server's domains.2da) was cut
/// to the field's width, and another domain written than the one picked
/// (row 300 as 44).
fn byte_row(row: i64, table: &str) -> Result<Value, String> {
    match u8::try_from(row) {
        Ok(row) => Ok(Value::Byte(row)),
        Err(_) => Err(format!(
            "{table}'s row {row} is not set: the field holds a row up to 255, as the game reads it"
        )),
    }
}

/// A 2DA row picked for a field that may be left out (`None`: the game
/// decides, as it does for a cleric without domains); the row picked.
fn optional_pick(
    ui: &mut Ui,
    of: egui::Id,
    name: &str,
    choices: &[Choice],
    current: Option<i64>,
) -> Option<i64> {
    let id = of.with(name);
    crate::widgets::row_choice(id, choices, current).unset("Not set").width(100.0).show(ui)
}

/// The familiar and animal companion (`FamiliarType`, `FamiliarName`,
/// `CompanionType`, `CompanionName`). The game reads a familiar only when a
/// class has one (classes.2da: arcane, MinAssociateLevel not 255) and a
/// companion likewise for a divine class, at any level
/// (`engine_ee_fields.rs`).
fn associates(
    f: &mut Form<'_>,
    ui: &mut Ui,
    list: &[Struct],
    class_int: &dyn Fn(i64, &str) -> Option<i64>,
) {
    let has = |arcane: i64| {
        list.iter().filter_map(|c| c.integer("Class")).any(|class| {
            class_int(class, "Arcane") == Some(arcane)
                && class_int(class, "MinAssociateLevel").is_some_and(|l| l != 255)
        })
    };
    let familiars = f.choices("hen_familiar", Some("STRREF"), Some("NAME"));
    let companions = f.choices("hen_companion", Some("STRREF"), Some("NAME"));
    let mut notes = Vec::new();
    // (In a narrow room, the name on a row of its own.)
    let narrow = super::narrow(ui);
    let grid = egui::Grid::new(("utc-associates", f.key, narrow));
    grid.num_columns(if narrow { 2 } else { 4 }).spacing([12.0, 6.0]).show(ui, |ui| {
        for (title, kinds, type_label, name_label, arcane, who) in [
            (
                "Familiar",
                &familiars,
                "FamiliarType",
                "FamiliarName",
                1,
                "a familiar only for a creature with an arcane class that has one (Wizard, Sorcerer)",
            ),
            (
                "Animal companion",
                &companions,
                "CompanionType",
                "CompanionName",
                0,
                "an animal companion only for a creature with a divine class that has one \
                 (Druid, Ranger)",
            ),
        ] {
            crate::widgets::field_label(ui, title);
            f.mark_mixed(ui, type_label);
            let current = f.root.integer(type_label);
            if let Some(v) = optional_pick(ui, f.picks(), type_label, kinds, current) {
                f.set_int(title, type_label, v, FieldType::Int);
            }
            if narrow {
                ui.end_row();
            }
            crate::widgets::field_label(ui, "Name");
            f.text(ui, title, name_label, 64);
            ui.end_row();
            if !has(arcane) {
                notes.push(format!("The game reads {who}."));
            }
        }
    });
    for n in notes {
        ui.weak(n);
    }
}

fn set(key: ResKey, path: &GffPath, label: &str, value: Value) -> Edit {
    Edit::SetField { key, path: path.clone(), label: label.into(), value: Some(value) }
}

/// The Skills page: a rank for each skill. Several creatures edited
/// together show the first one's ranks, ≠ where the others have another,
/// and a rank typed is set on each.
fn skills(f: &mut Form<'_>, ui: &mut Ui) {
    let key = f.key;
    let skills = f.choices("skills", Some("Name"), Some("Label"));
    let described_by = f.app.game.clone();
    let targets = f.list_targets();
    // Which skills' ranks the others have another of, by row: kept while
    // the targets are (the review's B20: 100 creatures' ranks compared
    // each frame took 0.1 ms).
    let rows = skills.iter().map(|s| s.row + 1).max().unwrap_or(0);
    let differ: std::sync::Arc<Vec<bool>> = super::kept_for(
        ui.ctx(),
        egui::Id::new(("utc-skills-differ", key)),
        &targets,
        &rows,
        || std::sync::Arc::new(skills_differ(&targets, rows)),
    );
    let mut edit = None;
    // (Two lists side by side where the page has the room, as
    // WideCreature's; one in a narrow form or a narrow page.)
    let halves = if !super::narrow(ui) && ui.available_width() >= SKILLS_TWO { 2 } else { 1 };
    let per = skills.len().div_ceil(halves);
    egui::ScrollArea::vertical().id_salt(("utc-skills", key)).show(ui, |ui| {
        let mut grid = |ui: &mut Ui, half: usize| {
            mg_uikit::with_table_rows(ui, |ui| {
                super::form_grid(ui, ("utc-skill-grid", key, half))
                    .striped(true)
                    .spacing([24.0, mg_uikit::metrics::TABLE_RULE])
                    .show(ui, |ui| {
                        ui.strong("Skill");
                        ui.strong("Rank");
                        ui.end_row();
                        for s in skills.iter().skip(half * per).take(per) {
                            let r = ui.label(&s.text);
                            let game = described_by.as_deref();
                            super::creature_lists::described(
                                r,
                                game,
                                "skills",
                                "Description",
                                s.row,
                            );
                            let rank = lists::skill_rank(&targets[0], s.row);
                            let differs = differ.get(s.row).copied().unwrap_or(false);
                            f.mark_if(ui, &format!("Skill#{}", s.row), differs);
                            if let Some(v) = commit_number(ui, rank, 0..=127) {
                                edit = Some((s.row, v));
                            }
                            ui.end_row();
                        }
                    })
            });
        };
        match halves {
            1 => grid(ui, 0),
            _ => {
                ui.columns(2, |cols| {
                    for (half, col) in cols.iter_mut().enumerate() {
                        grid(col, half);
                    }
                });
            }
        }
    });
    if let Some((row, v)) = edit {
        // (The list is indexed by skill: missing entries up to it are
        // added.)
        let out = lists::set_skill_rank(&targets, row, v as u8);
        f.apply_each("Skill rank", out, "Set on");
    }
}

/// Of skill rows up to `rows`, whether each has a rank in any of
/// `targets` other than the first's.
fn skills_differ(targets: &[lists::Target], rows: usize) -> Vec<bool> {
    let Some((first, others)) = targets.split_first() else { return vec![false; rows] };
    (0..rows)
        .map(|row| {
            let rank = lists::skill_rank(first, row);
            others.iter().any(|t| lists::skill_rank(t, row) != rank)
        })
        .collect()
}

/// The width from which a creature's skills stand in two lists side by
/// side.
const SKILLS_TWO: f32 = 400.0;

fn advanced(f: &mut Form<'_>, ui: &mut Ui) {
    let bags = f.choices("bodybag", Some("Name"), Some("LABEL"));
    let ranges = f.choices("ranges", Some("Name"), Some("Label"));
    mg_uikit::form::FormColumns::new(f.id("utc-advanced")).show(ui, 2, |ui, col| {
        if col == 0 {
            super::form_grid(ui, ("utc-adv", f.key)).spacing([12.0, 6.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "Blueprint ResRef");
                f.blueprint_resref(ui);
                ui.end_row();
                crate::widgets::field_label(ui, "Treasure model");
                f.choice(ui, "Treasure model", "BodyBag", &bags, FieldType::Byte);
                ui.end_row();
                crate::widgets::field_label(ui, "Corpse decay time (s)");
                f.millis(ui, "Decay time", "DecayTime", 0.0..=32767.0, 1.0);
                ui.end_row();
                crate::widgets::field_label(ui, "Perception range");
                f.choice(ui, "Perception range", "PerceptionRange", &ranges, FieldType::Byte);
                ui.end_row();
                crate::widgets::field_label(ui, "Subrace");
                f.text(ui, "Subrace", "Subrace", 32);
                ui.end_row();
                crate::widgets::field_label(ui, "Deity");
                f.text(ui, "Deity", "Deity", 32);
                ui.end_row();
                crate::widgets::field_label(ui, "Variables");
                f.variables(ui);
                ui.end_row();
                crate::widgets::field_label(ui, "");
                f.update_instances(ui);
                ui.end_row();
            });
        } else {
            // (Two to a row, as WideCreature's; one under another in a
            // narrow form.)
            let checks = [
                ("Leaves lootable corpse", "Lootable"),
                ("Disarmable", "Disarmable"),
                ("Plot", "Plot"),
                ("No permanent death", "NoPermDeath"),
                ("Immortal", "IsImmortal"),
            ];
            let per_row = if super::narrow(ui) { 1 } else { 2 };
            egui::Grid::new(("utc-adv-checks", f.key, per_row)).num_columns(per_row).show(
                ui,
                |ui| {
                    for (n, (text, label)) in checks.into_iter().enumerate() {
                        f.check(ui, text, label);
                        if n % per_row == per_row - 1 {
                            ui.end_row();
                        }
                    }
                },
            );
            ui.add_space(crate::widgets::SECTION_GAP);
            crate::widgets::section_heading(ui, "Challenge Rating");
            super::form_grid(ui, ("utc-cr", f.key)).spacing([12.0, 4.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "Adjustment");
                // The rating is recalculated with it (`refresh_hit_points`).
                let adjust = f.int("CRAdjust");
                f.mark_mixed(ui, "CRAdjust");
                if let Some(v) = spin_number(ui, adjust, -125..=125) {
                    let adjust_value = super::integer(f.root.get("CRAdjust"), v, FieldType::Int);
                    f.set_fields("CR adjustment", vec![("CRAdjust", adjust_value)]);
                }
                ui.end_row();
                crate::widgets::field_label(ui, "Challenge rating");
                rating_label(f, ui);
                ui.end_row();
            });
        }
    });
}

/// The width of the Sound set row on one line: its two filters, the list
/// and the sample's button.
const SOUND_SET_ROW: f32 = 420.0;

/// The Basic page's Sound set row: Aurora's filters (gender, and
/// soundsettype.2da's type), the list, and Play Sample. The filters stand
/// on a line of their own, the list and its sample under them, where the
/// row is short of all four (a column of pages side by side, M4's U10).
fn sound_set(f: &mut Form<'_>, ui: &mut Ui) {
    let sounds = f.choices("soundset", Some("STRREF"), Some("LABEL"));
    let two = !super::narrow(ui) && super::room_right(ui) < SOUND_SET_ROW;
    ui.vertical(|ui| {
        let filters = super::row(ui, |ui| {
            // Aurora's filters: gender and soundsettype.2da type.
            let id = f.id("utc-soundset-filter");
            let (mut gender, mut kind): (Option<i64>, Option<i64>) =
                ui.data(|d| d.get_temp(id)).unwrap_or_default();
            let types = f.choices("soundsettype", Some("STRREF"), Some("LABEL"));
            let name = |v: Option<i64>, all: &str, names: &dyn Fn(i64) -> String| {
                v.map_or(all.to_string(), names)
            };
            let gender_name =
                |g: &Option<i64>| name(*g, "Both", &|g| ["Male", "Female"][g as usize].into());
            crate::widgets::dropdown(id.with("gender"))
                .selected_text(gender_name(&gender))
                .width(70.0)
                .show(ui, &mut gender, &[None, Some(0), Some(1)], gender_name);
            let list = crate::widgets::row_choice(id.with("type"), &types, kind);
            if let Some(k) = list.none("All").width(90.0).show_optional(ui) {
                kind = k;
            }
            ui.data_mut(|d| d.insert_temp(id, (gender, kind)));
            if !two {
                sound_set_choice(f, ui, &sounds, gender, kind);
            }
            (gender, kind)
        });
        if two {
            let (gender, kind) = filters.inner;
            super::row(ui, |ui| sound_set_choice(f, ui, &sounds, gender, kind));
        }
    });
}

/// The Sound set list, by the filters chosen, and its sample's button.
fn sound_set_choice(
    f: &mut Form<'_>,
    ui: &mut Ui,
    sounds: &[Choice],
    gender: Option<i64>,
    kind: Option<i64>,
) {
    let current = f.root.integer("SoundSetFile").unwrap_or(-1);
    let table = f.app.game.as_deref().and_then(|g| g.table("soundset").ok());
    let shown: Vec<_> = sounds
        .iter()
        .filter(|c| {
            let col = |l: &str| table.as_ref().and_then(|t| t.get_int(c.row, l)).map(i64::from);
            c.row as i64 == current
                || (gender.is_none_or(|g| col("GENDER") == Some(g))
                    && kind.is_none_or(|k| col("TYPE") == Some(k)))
        })
        .cloned()
        .collect();
    f.choice(ui, "Sound set", "SoundSetFile", &shown, FieldType::Word);
    // Aurora's sound set list plays a sample when clicked.
    let set = f.root.integer("SoundSetFile").unwrap_or(-1);
    let tip = "Play a sample of the sound set";
    if crate::widgets::icon_button(ui, mg_uikit::Icon::Play, "Play Sample", tip).clicked()
        && let Some(name) = sound_set_sample(f.app, set)
    {
        f.app.play_sound(crate::audio::Channel::Preview, name, 1.0, false);
    }
}

/// A sound set's sample (soundset.2da `RESREF`'s soundset file): its
/// Selected sound, else its first.
fn sound_set_sample(app: &crate::Moonglow, set: i64) -> Option<ResRef> {
    let game = app.game.as_deref()?;
    let name = game.table("soundset").ok()?.get(usize::try_from(set).ok()?, "RESREF")?.to_string();
    let data = game.resman.get_named(&name, mg_core::ResType::SSF).ok()?;
    sample_of(&mg_ssf::Ssf::read(&data).ok()?)
}

/// The sound a sound set is sampled by: its Selected sound (SSF entry
/// 21), else its first. A sound's name is its field up to the first NUL,
/// as the game reads it ([`mg_ssf::SsfEntry::sound_name`]).
fn sample_of(ssf: &mg_ssf::Ssf) -> Option<ResRef> {
    let sound =
        |e: &mg_ssf::SsfEntry| ResRef::from_bytes(e.sound_name()).ok().filter(|r| !r.is_empty());
    ssf.entries.get(21).and_then(sound).or_else(|| ssf.entries.iter().find_map(sound))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The skills whose ranks the others have another of: a rank left out
    /// of a shorter list is 0, as the page reads it; one creature alone
    /// differs in none.
    #[test]
    fn skills_differ_where_another_has_another_rank() {
        let key = ResKey::parse("area", ResType::GIT).unwrap();
        let creature = |i: usize, ranks: &[u8]| {
            let list = ranks.iter().map(|&r| {
                let mut e = Struct::new(0);
                e.set("Rank", Value::Byte(r));
                e
            });
            let mut s = Struct::new(0);
            s.set("SkillList", Value::List(list.collect()));
            lists::Target::new(key, GffPath::root().item("Creature List", i), s)
        };
        let targets = [creature(0, &[4, 0, 2]), creature(1, &[4, 1]), creature(2, &[4, 0, 2])];
        assert_eq!(skills_differ(&targets, 4), [false, true, true, false]);
        assert_eq!(skills_differ(&targets[..1], 3), [false, false, false]);
        assert_eq!(skills_differ(&[], 2), [false, false]);
    }

    /// A sound set's sample is named by its field up to the first NUL:
    /// bytes a tool left after it are not part of the name (the field
    /// taken whole named no sound, so the button played nothing).
    #[test]
    fn a_sound_set_s_sample_is_its_field_up_to_the_first_nul() {
        let entry = |sound: &[u8]| mg_ssf::SsfEntry {
            sound: sound.to_vec(),
            strref: mg_core::StrRef::NONE,
        };
        let mut ssf = mg_ssf::Ssf { entries: vec![entry(b""), entry(b"vs_first\0junk")] };
        assert_eq!(sample_of(&ssf).unwrap().as_str(), Some("vs_first"));
        ssf.entries.resize(21, entry(b""));
        ssf.entries.push(entry(b"vs_sel\0\0\0\0\0\0xyz\0"));
        assert_eq!(sample_of(&ssf).unwrap().as_str(), Some("vs_sel"), "Selected, not the first");
        assert_eq!(sample_of(&mg_ssf::Ssf { entries: vec![entry(b"\0vs_x")] }), None);
    }

    /// A domain or a school is stored in a BYTE: a row past 255 is said,
    /// not cut to the field's width (the review's B30: row 300 was
    /// written as 44, another domain).
    #[test]
    fn a_domain_s_row_past_the_field_is_said_not_cut() {
        assert_eq!(byte_row(44, "domains.2da"), Ok(Value::Byte(44)));
        assert_eq!(byte_row(255, "domains.2da"), Ok(Value::Byte(255)));
        for row in [300, 256, -1] {
            let said = byte_row(row, "domains.2da").unwrap_err();
            assert!(said.starts_with(&format!("domains.2da's row {row} is not set")), "{said}");
        }
    }

    #[test]
    fn every_right_body_part_has_its_left() {
        // (Aurora's Appearance page: `cbFootRight` and `cbFootLeft`.)
        for (name, ..) in BODY_PARTS {
            if let Some(part) = name.strip_prefix("Right ") {
                let left = format!("Left {part}");
                assert!(BODY_PARTS.iter().any(|(n, ..)| *n == left), "{name} without {left}");
            }
        }
        let fields: Vec<&str> = BODY_PARTS.iter().map(|(_, field, _)| *field).collect();
        assert!(fields.contains(&"BodyPart_LFoot"));
    }
}
