//! Parts the trigger, door and placeable editors share, as Aurora's
//! situated frames: the trap settings (`TfrmTrap`), event scripts, the
//! faction and the portrait, the area transition, the lock, hit points and
//! saves, and the Advanced page.

use egui::Ui;
use mg_core::ResType;
use mg_gff::{FieldType, Value};
use mg_resman::ResKey;
use mg_rules::Choice;

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
    f.mark_mixed(ui, "PortraitId");
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
            crate::widgets::table_choices(game, t, Some(name), label)
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

/// A door or waypoint an area transition may lead to.
#[derive(Debug, Clone, PartialEq)]
struct Destination {
    area: mg_core::ResRef,
    tag: String,
    /// As `LinkedToFlags`: 1 a door, 2 a waypoint.
    flags: i64,
    /// Where it stands in its area, metres.
    at: glam::Vec2,
}

/// Doors and waypoints with tags in the module's areas, by area.
fn destinations(f: &mut Form<'_>) -> Vec<Destination> {
    let Some(ws) = f.app.ws.as_mut() else { return Vec::new() };
    let gits: Vec<_> = ws.module.keys_of(ResType::GIT).copied().collect();
    let mut out = Vec::new();
    for key in gits {
        let Ok(g) = ws.doc(&key) else { continue };
        for (list, restype, flags) in
            [("Door List", ResType::UTD, 1), ("WaypointList", ResType::UTW, 2)]
        {
            for s in g.root.list(list).unwrap_or(&[]) {
                let tag = decode(s.string("Tag").unwrap_or_default());
                if !tag.is_empty() {
                    let p = mg_module::instances::placement_of(restype, s).0.position;
                    let at = glam::Vec2::new(p[0], p[1]);
                    out.push(Destination { area: key.resref, tag, flags, at });
                }
            }
        }
    }
    out.sort_by(|a, b| (a.area, &a.tag, a.flags).cmp(&(b.area, &b.tag, b.flags)));
    out
}

/// The Setup Area Transition window's state (Aurora's
/// `TdlgAreaTransition`): the doors and waypoints gathered as it opened,
/// the area shown and its map, and the one chosen (its tag and
/// `LinkedToFlags`).
#[derive(Debug, Clone, Default)]
struct TransitionPick {
    open: bool,
    destinations: Vec<Destination>,
    area: Option<mg_core::ResRef>,
    map: Option<crate::minimap::Map>,
    chosen: Option<(String, i64)>,
}

/// The side of Setup Area Transition's map, points.
const TRANSITION_MAP: f32 = 320.0;

/// How near the pointer must be to a door or waypoint on the map to take
/// it, points.
const MARKER_REACH: f32 = 9.0;

/// Setup Area Transition (Aurora's `TdlgAreaTransition`): the target
/// area, its map with its doors (squares) and waypoints (dots) to click,
/// and their lists beside. What OK says the transition leads to, or
/// `None`.
fn setup_transition(f: &mut Form<'_>, ui: &mut Ui, pick_id: egui::Id) -> Option<(String, i64)> {
    let mut state: TransitionPick = ui.data(|d| d.get_temp(pick_id)).unwrap_or_default();
    if !state.open {
        return None;
    }
    let mut areas: Vec<mg_core::ResRef> = state.destinations.iter().map(|d| d.area).collect();
    areas.dedup();
    let names: std::collections::HashMap<mg_core::ResRef, String> =
        areas.iter().map(|a| (*a, crate::tabs::area_label(f.app, *a))).collect();
    let name = |a: &Option<mg_core::ResRef>| {
        a.map_or_else(String::new, |a| names.get(&a).cloned().unwrap_or_else(|| a.to_string()))
    };
    let ctx = ui.ctx().clone();
    let (mut ok, mut cancel) = (false, false);
    let modal = egui::Modal::new(pick_id.with("modal")).show(&ctx, |ui| {
        ui.heading("Setup Area Transition");
        if areas.is_empty() {
            ui.weak("No doors or waypoints with tags in the module's areas");
        }
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Target Area");
            let options: Vec<Option<mg_core::ResRef>> = areas.iter().copied().map(Some).collect();
            crate::widgets::dropdown(pick_id.with("area"))
                .selected_text(name(&state.area))
                .width(260.0)
                .show(ui, &mut state.area, &options, name);
        });
        let here: Vec<Destination> =
            state.destinations.iter().filter(|d| Some(d.area) == state.area).cloned().collect();
        let is = |d: &Destination, chosen: &Option<(String, i64)>| {
            chosen.as_ref().is_some_and(|(t, k)| *t == d.tag && *k == d.flags)
        };
        ui.horizontal_top(|ui| {
            // The map: a click on a door or waypoint takes it.
            let shown = state.area.and_then(|a| crate::minimap::of(f.app, &ctx, a, &mut state.map));
            if let Some(map) = shown.cloned() {
                let (rect, r) =
                    ui.allocate_exact_size(egui::Vec2::splat(TRANSITION_MAP), egui::Sense::click());
                r.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "The area's map")
                });
                // (In the middle of its square, a long area's.)
                let placed = map.placed(rect.right_top(), TRANSITION_MAP);
                let rect = placed.translate(egui::vec2(
                    (placed.width() - rect.width()) / 2.0,
                    (rect.height() - placed.height()) / 2.0,
                ));
                let painter = ui.painter_at(rect.expand(3.0));
                map.draw(&painter, rect);
                let near = r.hover_pos().and_then(|p| {
                    let d = |x: &Destination| map.to_map(rect, x.at).distance(p);
                    here.iter()
                        .filter(|x| d(x) <= MARKER_REACH)
                        .min_by(|a, b| d(a).total_cmp(&d(b)))
                });
                for d in &here {
                    let at = map.to_map(rect, d.at);
                    let lit = is(d, &state.chosen) || near == Some(d);
                    let (fill, edge) = if lit {
                        (egui::Color32::YELLOW, egui::Color32::BLACK)
                    } else if d.flags == 1 {
                        (egui::Color32::from_rgb(90, 160, 255), egui::Color32::BLACK)
                    } else {
                        (egui::Color32::from_rgb(90, 220, 120), egui::Color32::BLACK)
                    };
                    let stroke = egui::Stroke::new(1.0, edge);
                    if d.flags == 1 {
                        let square = egui::Rect::from_center_size(at, egui::Vec2::splat(8.0));
                        painter.rect_filled(square, 1.0, fill);
                        painter.rect_stroke(square, 1.0, stroke, egui::StrokeKind::Outside);
                    } else {
                        painter.circle_filled(at, 4.0, fill);
                        painter.circle_stroke(at, 4.0, stroke);
                    }
                }
                if let Some(d) = near {
                    let kind = if d.flags == 1 { "door" } else { "waypoint" };
                    r.clone().on_hover_text(format!("{} ({kind})", d.tag));
                }
                if r.clicked()
                    && let Some(d) = near
                {
                    state.chosen = Some((d.tag.clone(), d.flags));
                }
                if r.double_clicked() && near.is_some() {
                    ok = true;
                }
            }
            // The lists: the area's doors, then its waypoints.
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt(pick_id.with("lists"))
                    .max_height(TRANSITION_MAP)
                    .min_scrolled_width(200.0)
                    .show(ui, |ui| {
                        for (flags, title) in [(1, "Available Doors:"), (2, "Available Waypoints:")]
                        {
                            ui.strong(title);
                            for d in here.iter().filter(|d| d.flags == flags) {
                                let r = ui.selectable_label(is(d, &state.chosen), &d.tag);
                                if r.clicked() {
                                    state.chosen = Some((d.tag.clone(), d.flags));
                                }
                                if r.double_clicked() {
                                    ok = true;
                                }
                            }
                            ui.add_space(6.0);
                        }
                    });
            });
        });
        match &state.chosen {
            Some((tag, flags)) => {
                let kind = if *flags == 1 { "door" } else { "waypoint" };
                ui.label(format!("Leads to the {kind} tagged {tag}"));
            }
            None => {
                ui.weak("Click a door (a square) or a waypoint (a dot) on the map, or in a list");
            }
        }
        ui.horizontal(|ui| {
            ok |= ui
                .add_enabled(state.chosen.is_some(), egui::Button::new("OK"))
                .on_hover_text("Lead the transition there")
                .clicked()
                || (state.chosen.is_some() && crate::widgets::enter(ui));
            cancel = crate::widgets::cancel_discard(ui);
        });
    });
    let chosen = if ok { state.chosen.clone() } else { None };
    if ok || cancel || modal.should_close() {
        state = TransitionPick::default();
    }
    ui.data_mut(|d| d.insert_temp(pick_id, state));
    chosen
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
        f.mark_mixed(ui, "LinkedToFlags");
        ui.horizontal(|ui| {
            for (text, v) in [("Door", 1), ("Waypoint", 2), ("None", 0)] {
                if ui.radio(flags == v, text).clicked() && flags != v {
                    f.set_int("Destination type", "LinkedToFlags", v, FieldType::Byte);
                }
            }
        });
        ui.end_row();
        crate::widgets::field_label(ui, "");
        // Aurora's Setup Area Transition: a door or waypoint in an area,
        // picked on its map or from its lists (gathered as the window
        // opens: from every area's objects).
        let pick_id = f.id("transition-setup");
        if ui
            .button("Setup Area Transition…")
            .on_hover_text("Pick the door or waypoint it leads to, on its area's map or in a list")
            .clicked()
        {
            let destinations = destinations(f);
            let linked = decode(f.root.string("LinkedTo").unwrap_or_default());
            let now = destinations.iter().find(|d| d.tag == linked).cloned();
            let state = TransitionPick {
                open: true,
                area: now.as_ref().or(destinations.first()).map(|d| d.area),
                chosen: now.map(|d| (d.tag, d.flags)),
                destinations,
                map: None,
            };
            ui.data_mut(|d| d.insert_temp(pick_id, state));
        }
        if let Some((tag, flags)) = setup_transition(f, ui, pick_id) {
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
    f.mark_mixed(ui, "HP");
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
            f.mark_mixed(ui, "Interruptable");
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
/// column `side` wide on the left; the model has the rest of the window.
/// A window too narrow for both, the model at
/// least [`MODEL_LEAST`] wide, has the fields alone, where they were, and
/// they are told so (`false`): Preview shows the model, or the page finds
/// it a place among its fields.
pub(super) fn beside_model(
    f: &mut Form<'_>,
    ui: &mut Ui,
    side: f32,
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
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(side);
            egui::ScrollArea::vertical()
                .id_salt(("beside-model", f.key))
                .auto_shrink([false, false])
                .show(ui, |ui| fields(f, ui, true));
        });
        ui.vertical(|ui| inline_preview(f, ui, size));
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
