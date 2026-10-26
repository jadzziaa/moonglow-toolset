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
use crate::widgets::spin_number;

/// The most Aurora takes in the fields the files keep as bytes (DCs,
/// hardness, an object's saves): measured in Aurora, whose forms' own
/// limit of 100 is only what they start with. (Hit points: 10000.)
pub(super) const BYTE_MAX: i64 = 250;

/// The Trap tab: Is Trapped, then the trap's settings (enabled when
/// trapped): type (traps.2da, with its rogue modifiers shown), disarmable,
/// detectable, one shot, the DCs and the trap scripts.
pub(super) fn trap(f: &mut Form<'_>, ui: &mut Ui) {
    let trapped = f.check(ui, "Is Trapped", "TrapFlag");
    ui.separator();
    let traps = f.choices("traps", Some("TrapName"), Some("Label"));
    let row = f.int("TrapType");
    let table = f.app.game.as_deref().and_then(|g| g.table("traps").ok());
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
            crate::widgets::field_label(ui, "Trap Type");
            f.choice(ui, "Trap type", "TrapType", &traps, FieldType::Byte);
            ui.end_row();
            crate::widgets::field_label(ui, "Detect DC Mod. when set by Rogue");
            ui.label(cell("DetectDCMod"));
            ui.end_row();
            crate::widgets::field_label(ui, "Disarm DC Mod. when set by Rogue");
            ui.label(cell("DisarmDCMod"));
            ui.end_row();
            crate::widgets::field_label(ui, "Set DC");
            ui.label(cell("SetDC"));
            ui.end_row();
            crate::widgets::field_label(ui, "");
            ui.horizontal(|ui| {
                f.check(ui, "Disarmable", "TrapDisarmable");
                f.check(ui, "Detectable", "TrapDetectable");
                f.check(ui, "One Shot", "TrapOneShot");
            });
            ui.end_row();
            crate::widgets::field_label(ui, "Disarm DC");
            f.spin(ui, "Disarm DC", "DisarmDC", 0..=BYTE_MAX);
            ui.end_row();
            crate::widgets::field_label(ui, "Detection DC");
            f.spin(ui, "Detection DC", "TrapDetectDC", 0..=BYTE_MAX);
            ui.end_row();
            crate::widgets::field_label(ui, "OnDisarm");
            f.script(ui, "OnDisarm", "OnDisarm");
            ui.end_row();
            crate::widgets::field_label(ui, "OnTrapTriggered");
            f.script(ui, "OnTrapTriggered", "OnTrapTriggered");
            ui.end_row();
        });
    });
}

/// Event scripts: (label, field) rows.
pub(super) fn scripts(f: &mut Form<'_>, ui: &mut Ui, events: &[(&str, &str)]) {
    egui::Grid::new(("scripts", f.key)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
        for (label, field) in events {
            crate::widgets::field_label(ui, *label);
            f.script(ui, label, field);
            ui.end_row();
        }
    });
    f.script_set_buttons(ui, events);
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
    object_factions(names)
}

/// The factions an object may belong to, of the module's factions by row:
/// all but the PC faction, the players' alone (Jasperre's report; the
/// game puts a creature of it in Hostile: `mg_rules::objects::PC_FACTION`).
pub(crate) fn object_factions(names: Vec<String>) -> Vec<Choice> {
    let pc = mg_rules::objects::PC_FACTION as usize;
    names
        .into_iter()
        .enumerate()
        .filter(|(row, _)| *row != pc)
        .map(|(row, text)| Choice { row, text })
        .collect()
}

/// What a faction field shows for an object of the PC faction, which its
/// file keeps until the builder picks another.
pub(crate) const PC_FACTION_SHOWN: &str = "PC (not for objects)";

/// A faction field: the module's factions but the PC faction.
pub(super) fn faction(f: &mut Form<'_>, ui: &mut Ui, what: &str, label: &str) {
    let choices = factions(f);
    f.mark_mixed(ui, label);
    let current = f.int(label);
    let mut list = crate::widgets::row_choice(f.id(label), &choices, current).width(220.0);
    if current == mg_rules::objects::PC_FACTION {
        list = list.shown(PC_FACTION_SHOWN).hover_text(
            "Faction 0 is the players' own: the game puts a creature of it in Hostile, and \
             creatures regard a placeable, door or trigger of it as they regard players. Pick \
             another.",
        );
    }
    if let Some(v) = list.show(ui) {
        f.set_int(what, label, v, FieldType::Dword);
    }
}

/// The Select Portrait window's state (Aurora's `TdlgPortrait`).
#[derive(Debug, Clone, Default)]
struct PortraitPick {
    open: bool,
    /// 0 Characters and Creatures, 1 Placeable Objects and Doors, 2 Plot
    /// Characters.
    mode: u8,
    race: Option<i32>,
    gender: Option<i32>,
    category: Option<i32>,
    chosen: Option<usize>,
}

/// A portrait's image (`Loader::portrait`).
fn portrait_image(
    f: &mut Form<'_>,
    ui: &mut Ui,
    base: &str,
    size: char,
    width: f32,
    sense: egui::Sense,
) -> egui::Response {
    match f.app.loader() {
        Some(mut l) => l.portrait(ui, base, size, width, sense),
        None => ui.add_sized(
            egui::vec2(width, width * 100.0 / 64.0),
            egui::Label::new(format!("({base})")).sense(sense),
        ),
    }
}

/// The portrait: a portraits.2da row (`PortraitId`), else, in older
/// blueprints, a resref (`Portrait`, `po_` and the row's base resref). A
/// choice sets the row, and the resref where the blueprint has one. Its
/// image shows; … opens Select Portrait (thumbnails, filters).
pub(super) fn portrait(f: &mut Form<'_>, ui: &mut Ui) {
    let Some(table) = f.app.game.as_deref().and_then(|g| g.table("portraits").ok()) else {
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
    let pick_id = egui::Id::new(("portrait-pick", f.key));
    let mut pick = None;
    let list: Vec<Choice> =
        choices.iter().map(|&(row, b)| Choice { row, text: b.to_string() }).collect();
    // A little room below, so the picture doesn't sit on the next row.
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if let Some(b) = current.and_then(base) {
                portrait_image(f, ui, &b.to_lowercase(), 'm', 32.0, egui::Sense::hover());
            }
            let id = egui::Id::new(("portrait", f.key));
            let row = current.map(|r| r as i64);
            let list = crate::widgets::row_choice(id, &list, row).width(200.0).shown(shown);
            pick = list.show(ui).and_then(|r| usize::try_from(r).ok());
            if ui.button("Portraits…").on_hover_text("Select Portrait").clicked() {
                // Placeables and doors start on their own portraits.
                let inanimate = f.root.get("Appearance_Type").is_none();
                let mode = if inanimate { 1 } else { 0 };
                let state =
                    PortraitPick { open: true, mode, chosen: current, ..Default::default() };
                ui.data_mut(|d| d.insert_temp(pick_id, state));
            }
        });
        ui.add_space(6.0);
    });
    let mut state: PortraitPick = ui.data(|d| d.get_temp(pick_id)).unwrap_or_default();
    if state.open {
        let game = f.app.game.as_deref();
        let names = |t: &str, name: &str, label: Option<&str>| {
            game.and_then(|g| g.choices(t, ChoiceColumns { name: Some(name), label }).ok())
                .unwrap_or_default()
        };
        // (Races with a name alone: a row without one is `DELETED`,
        // `INVALID_RACE`, CEP's `CEP_RESERVED`.)
        let (races, genders, kinds) = (
            names("racialtypes", "Name", None),
            names("gender", "NAME", Some("CONSTANT")),
            names("placeabletypes", "StrRef", Some("Label")),
        );
        let int = |r: usize, c: &str| table.get_int(r, c);
        // Those with a picture (and the one chosen, whatever it has).
        let ws = f.app.ws.as_ref();
        let pictured = |r: usize| {
            state.chosen == Some(r)
                || game.zip(base(r)).is_some_and(|(g, b)| crate::images::has_portrait(g, ws, b))
        };
        let rows: Vec<usize> = choices
            .iter()
            .map(|c| c.0)
            .filter(|&r| {
                let (plot, inanimate) = (int(r, "Plot") == Some(1), int(r, "InanimateType"));
                let kind = match state.mode {
                    0 => inanimate.is_none() && !plot,
                    1 => inanimate.is_some(),
                    _ => plot,
                };
                kind && if state.mode == 1 {
                    state.category.is_none_or(|c| inanimate == Some(c))
                } else {
                    state.race.is_none_or(|x| int(r, "Race") == Some(x))
                        && state.gender.is_none_or(|g| int(r, "Sex") == Some(g))
                }
            })
            .filter(|&r| pictured(r))
            .collect();
        let (mut ok, mut cancel) = (false, false);
        // Most of the screen's width, the grid as many to a row as it holds.
        let wide = (ui.ctx().content_rect().width() * 0.8).clamp(480.0, 900.0);
        let modal = egui::Modal::new(pick_id.with("modal")).show(ui.ctx(), |ui| {
            ui.set_width(wide);
            ui.heading("Select Portrait");
            ui.horizontal(|ui| {
                for (m, text) in [
                    (0, "Characters and Creatures"),
                    (1, "Placeable Objects and Doors"),
                    (2, "Plot Characters"),
                ] {
                    ui.radio_value(&mut state.mode, m, text);
                }
            });
            ui.horizontal(|ui| {
                let combo =
                    |ui: &mut Ui, id: &str, value: &mut Option<i32>, list: &[Choice]| {
                        let text = value
                            .and_then(|v| list.iter().find(|c| c.row as i32 == v))
                            .map_or("Any".to_string(), |c| c.text.clone());
                        let options: Vec<Option<i32>> = std::iter::once(None)
                            .chain(list.iter().map(|c| Some(c.row as i32)))
                            .collect();
                        crate::widgets::dropdown((id, pick_id))
                            .selected_text(text)
                            .width(140.0)
                            .show(ui, value, &options, |v| {
                                v.and_then(|v| list.iter().find(|c| c.row as i32 == v))
                                    .map_or("Any".to_string(), |c| c.text.clone())
                            });
                    };
                if state.mode == 1 {
                    ui.label("Category:");
                    combo(ui, "category", &mut state.category, &kinds);
                } else {
                    ui.label("Race:");
                    combo(ui, "race", &mut state.race, &races);
                    ui.label("Gender:");
                    combo(ui, "gender", &mut state.gender, &genders);
                }
            });
            ui.separator();
            ui.horizontal_top(|ui| {
                let preview = 140.0;
                let width = wide - preview - 2.0 * ui.spacing().item_spacing.x;
                let (per_row, row_height) = crate::images::portrait_grid(ui, width);
                let height = (ui.ctx().content_rect().height() * 0.55).clamp(320.0, 640.0);
                let grid = egui::vec2(width, height);
                let layout = egui::Layout::top_down(egui::Align::Min);
                ui.allocate_ui_with_layout(grid, layout, |ui| {
                    ui.set_width(grid.x);
                    egui::ScrollArea::vertical()
                        .max_height(height)
                        .auto_shrink([false, true])
                        .id_salt(pick_id.with("grid"))
                        .show_rows(ui, row_height, rows.len().div_ceil(per_row), |ui, range| {
                            for line in range {
                                ui.horizontal(|ui| {
                                    for &r in rows.iter().skip(line * per_row).take(per_row) {
                                        let b = base(r).unwrap_or_default().to_lowercase();
                                        let resp = portrait_image(
                                            f,
                                            ui,
                                            &b,
                                            'm',
                                            crate::images::PORTRAIT_THUMB,
                                            egui::Sense::click(),
                                        )
                                        .on_hover_text(b.as_str());
                                        if state.chosen == Some(r) {
                                            ui.painter().rect_stroke(
                                                resp.rect.expand(1.0),
                                                2.0,
                                                ui.visuals().selection.stroke,
                                                egui::StrokeKind::Outside,
                                            );
                                        }
                                        if resp.clicked() {
                                            state.chosen = Some(r);
                                        }
                                        if resp.double_clicked() {
                                            state.chosen = Some(r);
                                            ok = true;
                                        }
                                    }
                                });
                            }
                        });
                });
                ui.vertical(|ui| {
                    ui.set_width(preview);
                    if let Some(b) = state.chosen.and_then(base) {
                        let b = b.to_lowercase();
                        if f.app.picture(ui.ctx(), &format!("po_{b}l")).is_some() {
                            portrait_image(f, ui, &b, 'l', 128.0, egui::Sense::hover());
                        } else {
                            portrait_image(f, ui, &b, 'm', 128.0, egui::Sense::hover());
                        }
                        ui.label(b);
                    }
                });
            });
            ui.label(format!("{} portraits", rows.len()));
            ui.horizontal(|ui| {
                ok |= ui
                    .add_enabled(state.chosen.is_some(), egui::Button::new("OK"))
                    .on_hover_text("Accept changes")
                    .clicked()
                    || (state.chosen.is_some() && crate::widgets::enter(ui));
                cancel = crate::widgets::cancel_discard(ui);
            });
        });
        if ok {
            pick = state.chosen;
        }
        if ok || cancel || modal.should_close() {
            state.open = false;
        }
        ui.data_mut(|d| d.insert_temp(pick_id, state));
    }
    if let Some(row) = pick.filter(|row| current != Some(*row))
        && let Some(b) = base(row)
    {
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
    let loadscreens = f.choices("loadscreens", Some("StrRef"), Some("Label"));
    egui::Grid::new(("utt-transition", f.key)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        crate::widgets::field_label(ui, "Destination Tag");
        f.text(ui, "Destination tag", "LinkedTo", 32);
        ui.end_row();
        crate::widgets::field_label(ui, "Destination is a");
        let flags = f.int("LinkedToFlags");
        ui.horizontal(|ui| {
            for (text, v) in [("Door", 1), ("Waypoint", 2), ("None", 0)] {
                if ui.radio(flags == v, text).clicked() && flags != v {
                    f.set_int("Destination type", "LinkedToFlags", v, FieldType::Byte);
                }
            }
        });
        ui.end_row();
        crate::widgets::field_label(ui, "");
        // Aurora's Setup Area Transition: a door or waypoint in an area
        // (gathered while the list is open: from every area's objects).
        let mut pick = None;
        // (Commands, not a choice: no arrow keys.)
        crate::widgets::dropdown(("utt-setup", f.key))
            .selected_text("Setup Area Transition…")
            .width(260.0)
            .show_ui(ui, |ui| {
                let dests = destinations(f);
                // (Areas by name, when the module tree lists them so.)
                let mut areas: std::collections::HashMap<_, String> = Default::default();
                for (area, _, _) in &dests {
                    let named = area.parse().map(|r| crate::tabs::area_label(f.app, r));
                    areas
                        .entry(area.clone())
                        .or_insert_with(|| named.unwrap_or_else(|_| area.clone()));
                }
                if dests.is_empty() {
                    ui.weak("No doors or waypoints with tags in the module's areas");
                }
                for (area, tag, flags) in &dests {
                    let kind = if *flags == 1 { "door" } else { "waypoint" };
                    let area = &areas[area];
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
        crate::widgets::field_label(ui, "Loading Screen");
        f.load_screen(ui, &loadscreens);
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
        crate::widgets::field_label(ui, "Open Lock DC");
        f.spin(ui, "Open lock DC", "OpenLockDC", 0..=BYTE_MAX);
        ui.end_row();
        crate::widgets::field_label(ui, "Close Lock DC");
        f.spin(ui, "Close lock DC", "CloseLockDC", 0..=BYTE_MAX);
        ui.end_row();
        crate::widgets::field_label(ui, "Key Tag");
        f.text(ui, "Key tag", "KeyName", 32);
        ui.end_row();
    });
}

/// Hit points, hardness, saves and plot (`TfraSituatedBasic`), as grid rows.
pub(super) fn durability(f: &mut Form<'_>, ui: &mut Ui) {
    crate::widgets::field_label(ui, "Hit Points");
    // The blueprint's current hit points follow its maximum.
    if let Some(v) = spin_number(ui, f.int("HP"), 1..=10_000) {
        f.set_many(
            "Hit points",
            &[("HP", v, FieldType::Short), ("CurrentHP", v, FieldType::Short)],
        );
    }
    ui.end_row();
    crate::widgets::field_label(ui, "Hardness");
    f.spin(ui, "Hardness", "Hardness", 0..=BYTE_MAX);
    ui.end_row();
    crate::widgets::field_label(ui, "Fortitude Save");
    f.spin(ui, "Fortitude save", "Fort", 0..=BYTE_MAX);
    ui.end_row();
    crate::widgets::field_label(ui, "Reflex Save");
    f.spin(ui, "Reflex save", "Ref", 0..=BYTE_MAX);
    ui.end_row();
    crate::widgets::field_label(ui, "Will Save");
    f.spin(ui, "Will save", "Will", 0..=BYTE_MAX);
    ui.end_row();
    crate::widgets::field_label(ui, "");
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
            crate::widgets::field_label(ui, "Blueprint ResRef");
            f.blueprint_resref(ui);
            ui.end_row();
            crate::widgets::field_label(ui, "Belongs to Faction");
            faction(f, ui, "Faction", "Faction");
            ui.end_row();
            crate::widgets::field_label(ui, "Conversation");
            f.conversation(ui);
            ui.end_row();
            crate::widgets::field_label(ui, "Portrait");
            portrait(f, ui);
            ui.end_row();
            crate::widgets::field_label(ui, "Initial State");
            f.choice(ui, "Initial state", "AnimationState", &states, FieldType::Byte);
            ui.end_row();
            crate::widgets::field_label(ui, "");
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
            crate::widgets::field_label(ui, "Variables");
            f.variables(ui);
            ui.end_row();
            crate::widgets::field_label(ui, "");
            f.update_instances(ui);
            ui.end_row();
        },
    );
}

/// A choice among `choices` by row, outside a field (`current` is the
/// row); the row picked. (A [`row_choice`](crate::widgets::row_choice):
/// a long list by name, with a Filter field.)
pub(super) fn pick(
    ui: &mut Ui,
    key: ResKey,
    name: &str,
    choices: &[Choice],
    current: i64,
) -> Option<i64> {
    pick_list(key, name, choices, current, 220.0).show(ui)
}

/// [`pick`] in the order given, however long (a cost table's values,
/// from the least).
pub(super) fn pick_ordered(
    ui: &mut Ui,
    key: ResKey,
    name: &str,
    choices: &[Choice],
    current: i64,
) -> Option<i64> {
    pick_list(key, name, choices, current, 220.0).ordered().show(ui)
}

/// [`pick_ordered`], `width` wide, for a body part: the mouse wheel over
/// it, closed, steps through its choices too (a builder's ask: to see each
/// on the model without opening the list).
pub(super) fn pick_part(
    ui: &mut Ui,
    key: ResKey,
    name: &str,
    choices: &[Choice],
    current: i64,
    width: f32,
) -> Option<i64> {
    pick_list(key, name, choices, current, width).ordered().wheel().show(ui)
}

/// A body or armor part's choices: none first (0, no part there: a
/// creature's draws nothing, an armor's leaves its wearer's own part
/// showing, as the game does), then `numbers`, each as `text(n)`. (0 is
/// offered whether or not a list's rows have it: the game's parts_*.2da
/// leave row 0's `ACBONUS` blank in most of them.)
pub(super) fn part_choices(numbers: &[u16], text: impl Fn(u16) -> String) -> Vec<Choice> {
    let none = Choice { row: 0, text: "0 (none)".into() };
    let some = numbers.iter().filter(|n| **n != 0);
    std::iter::once(none).chain(some.map(|&n| Choice { row: n as usize, text: text(n) })).collect()
}

/// The list [`pick`] and the others show, to add to (a row's picture).
pub(super) fn pick_list<'a>(
    key: ResKey,
    name: &str,
    choices: &'a [Choice],
    current: i64,
    width: f32,
) -> crate::widgets::RowChoice<'a> {
    crate::widgets::row_choice(egui::Id::new(("pick", key, name)), choices, current).width(width)
}

/// The model viewer's tab for the form's object: a placed object as it is
/// placed (its GIT entry), else the blueprint.
fn preview_tab(f: &Form<'_>) -> crate::Tab {
    if f.key.restype == ResType::GIT {
        crate::Tab::InstanceModel { area: f.key.resref, path: f.path.clone() }
    } else {
        crate::Tab::Model(f.key)
    }
}

/// The model viewer in the page, `size` large, as Aurora shows a creature
/// in its Appearance page; Pop Out, among its buttons, moves it to a window
/// of its own (to keep beside other pages). While it is there, the page
/// says so and offers to bring it back.
pub(super) fn inline_preview(f: &mut Form<'_>, ui: &mut Ui, size: egui::Vec2) {
    let tab = preview_tab(f);
    let size = size.max(egui::vec2(120.0, 120.0));
    let popped = f.app.open_models.contains(&tab);
    let source = match &tab {
        crate::Tab::InstanceModel { area, path } => {
            crate::model_view::Source::Instance { area: *area, path: path.clone() }
        }
        _ => crate::model_view::Source::Resource(f.key),
    };
    ui.allocate_ui(size, |ui| {
        ui.set_min_size(size);
        ui.set_max_size(size);
        if popped {
            ui.weak("The preview is in a window of its own.");
            if ui.button("Bring Back").on_hover_text("Close its window and show it here").clicked()
            {
                f.app.actions.push(crate::Action::CloseTab(tab));
            }
        } else if crate::model_view::embedded(f.app, ui, source) {
            f.app.actions.push(crate::Action::OpenTab(tab));
        }
    });
}

/// A page that shows the object's model beside its fields, which take a
/// column `side` wide; the model has the rest of the window.
///
/// `model_first`: the model on the left (a page about how the object
/// looks), else the fields (a page of names and tags that happens to
/// choose the appearance). A window too narrow for both, the model at
/// least [`MODEL_LEAST`] wide, has the fields alone, where they were, and
/// they are told so (`false`): Preview shows the model, or the page finds
/// it a place among its fields.
pub(super) fn beside_model(
    f: &mut Form<'_>,
    ui: &mut Ui,
    side: f32,
    model_first: bool,
    fields: impl FnOnce(&mut Form<'_>, &mut Ui, bool),
) {
    let gap = ui.spacing().item_spacing.x;
    // What is left of the page's visible height, and its width. (The
    // visible width: a page once wider than the window would keep that.)
    let room = egui::vec2(
        ui.available_width().min(ui.clip_rect().right() - ui.cursor().left()),
        (ui.clip_rect().bottom() - ui.cursor().top()).max(MODEL_LEAST.y),
    );
    if room.x < side + gap + MODEL_LEAST.x {
        fields(f, ui, false);
        return;
    }
    // (The fields keep their width, their scroll bar included.)
    let width = room.x - side - 3.0 * gap - ui.spacing().scroll.bar_width;
    let size = egui::vec2(width, room.y - gap);
    let column = |f: &mut Form<'_>, ui: &mut Ui| {
        ui.vertical(|ui| {
            ui.set_width(side);
            egui::ScrollArea::vertical()
                .id_salt(("beside-model", f.key))
                .auto_shrink([false, false])
                .show(ui, |ui| fields(f, ui, true));
        });
    };
    ui.horizontal_top(|ui| {
        if model_first {
            ui.vertical(|ui| inline_preview(f, ui, size));
            column(f, ui);
        } else {
            column(f, ui);
            ui.vertical(|ui| inline_preview(f, ui, size));
        }
    });
}

/// The least the model takes of a page that shows it.
pub(super) const MODEL_LEAST: egui::Vec2 = egui::vec2(340.0, 300.0);

/// Preview: the blueprint's model in the model viewer (Aurora shows it in
/// the dialog).
pub(super) fn preview_button(f: &mut Form<'_>, ui: &mut Ui) {
    if ui.button("Preview").on_hover_text("Show the model").clicked() {
        let tab = preview_tab(f);
        f.app.actions.push(crate::Action::OpenTab(tab));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_can_be_none() {
        let texts = |numbers: &[u16]| -> Vec<String> {
            part_choices(numbers, |n| format!("{n}!")).into_iter().map(|c| c.text).collect()
        };
        assert_eq!(texts(&[1, 2, 5]), ["0 (none)", "1!", "2!", "5!"]);
        // (A list whose rows have 0 already: once, as none.)
        assert_eq!(texts(&[0, 1]), ["0 (none)", "1!"]);
        assert_eq!(texts(&[]), ["0 (none)"]);
    }
}
