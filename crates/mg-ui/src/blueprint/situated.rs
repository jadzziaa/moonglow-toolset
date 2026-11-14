//! Parts the trigger, door and placeable editors share, as Aurora's
//! situated frames: the trap settings (`TfrmTrap`), event scripts, the
//! faction and the portrait, the area transition, the lock, hit points and
//! saves, and the Advanced page.

use egui::Ui;
use mg_core::ResType;
use mg_gff::{FieldType, Value};
use mg_resman::ResKey;
use mg_rules::Choice;
use mg_uikit::Controls as _;

use super::Form;
use crate::text::decode;
use crate::widgets::spin_number;
use crate::{Action, Moonglow};
use mg_edit::transition::{Connection, Current, Placed, PlanError, Refused};
use mg_rules::ObjectKind;

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
            "Faction 0 is the players' own: the game puts a creature of it in Hostile. Pick \
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
    // (The rows with a portrait's name, as a list's choices: made once
    // for the table, and kept. Every page with a portrait made a name for
    // every row each frame.)
    let list = super::kept_for(ui.ctx(), egui::Id::new("portrait-rows"), &table, &(), || {
        let rows = (0..table.len()).filter_map(|row| Some(Choice { row, text: base(row)?.into() }));
        std::sync::Arc::new(rows.collect::<Vec<Choice>>())
    });
    let resref = f.root.resref("Portrait").filter(|r| !r.is_empty());
    let current = match f.root.integer("PortraitId") {
        Some(id) if id != 0xFFFF => Some(id as usize),
        _ => resref.and_then(|r| {
            let name = r.to_string();
            let named = |c: &&Choice| {
                name.get(..3).is_some_and(|po| po.eq_ignore_ascii_case("po_"))
                    && name[3..].eq_ignore_ascii_case(&c.text)
            };
            list.iter().find(named).map(|c| c.row)
        }),
    };
    let shown = match (current.and_then(base), resref) {
        (Some(b), _) => b.to_string(),
        (None, Some(r)) => r.to_string(),
        (None, None) => "(none)".to_string(),
    };
    let pick_id = f.id("portrait-pick");
    let mut pick = None;
    // A little room below, so the picture doesn't sit on the next row.
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if let Some(b) = current.and_then(base) {
                portrait_image(f, ui, &b.to_lowercase(), 'm', 32.0, egui::Sense::hover());
            }
            let id = f.id("portrait");
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
        // Those with a picture (and the one chosen, whatever it has):
        // looked for once for the table, the module and the load order,
        // not for every row each frame the window is open.
        let ws = f.app.ws.as_ref();
        let read_for = (ws.map(mg_edit::Workspace::revision), f.app.load_order);
        let with_picture =
            super::kept_for(ui.ctx(), pick_id.with("pictured"), &table, &read_for, || {
                let has =
                    |c: &&Choice| game.is_some_and(|g| crate::images::has_portrait(g, ws, &c.text));
                let rows: std::collections::HashSet<usize> =
                    list.iter().filter(has).map(|c| c.row).collect();
                std::sync::Arc::new(rows)
            });
        let pictured = |r: usize| state.chosen == Some(r) || with_picture.contains(&r);
        let rows: Vec<usize> = list
            .iter()
            .map(|c| c.row)
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
        let window = crate::widgets::modal(ui.ctx(), "Select Portrait")
            .id(pick_id.with("modal"))
            .close_as_cancel();
        window.show(ui.ctx(), |ui| {
            ui.set_width(wide);
            ui.horizontal(|ui| {
                for (m, text) in [
                    (0, "Characters and Creatures"),
                    (1, "Placeable Objects and Doors"),
                    (2, "Plot Characters"),
                ] {
                    ui.radio_choice(&mut state.mode, m, text);
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
                                                egui::Stroke::new(
                                                    1.0,
                                                    mg_uikit::tokens(ui.ctx()).accent,
                                                ),
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
        if ok || cancel {
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

/// A door, area transition trigger or waypoint an area transition may
/// lead to.
#[derive(Debug, Clone, PartialEq)]
struct Destination {
    /// Which object it is (two of one tag are two).
    object: Placed,
    area: mg_core::ResRef,
    tag: String,
    /// Its name, as the module tree shows it (empty without one).
    name: String,
    /// As `LinkedToFlags` names it: 1 a door, 2 a waypoint or a trigger
    /// (its waypoint).
    flags: i64,
    /// Where it is drawn on the map, meters: a trigger at the middle of
    /// its box, where its waypoint goes.
    at: glam::Vec2,
}

/// The kinds a transition leads to: each one's list on the window, its
/// name in the map's hover text, its name in the filter.
const LEADS_TO: [(ObjectKind, &str, &str, &str); 3] = [
    (ObjectKind::Door, "Available Doors:", "door", "Doors"),
    (ObjectKind::Trigger, "Available Triggers:", "trigger", "Triggers"),
    (ObjectKind::Waypoint, "Available Waypoints:", "waypoint", "Waypoints"),
];

/// The name of a kind a transition leads to (`door`).
fn kind_name(kind: ObjectKind) -> &'static str {
    LEADS_TO.iter().find(|k| k.0 == kind).map_or("object", |k| k.2)
}

/// Doors, area transition triggers and waypoints with tags in the
/// module's areas, by area.
fn destinations(f: &mut Form<'_>) -> Vec<Destination> {
    let game = f.app.game.clone();
    let Some(ws) = f.app.ws.as_ref() else { return Vec::new() };
    destinations_in(ws, game.as_deref())
}

/// [`destinations`] of a workspace's module. An area the workspace has
/// read is taken as it has it (edited, perhaps); the others are read from
/// the module's bytes, on every core, and not kept: read through the
/// workspace, a click parsed every area's objects on the window's thread
/// and left them all in memory for the session (a thousand areas, in a
/// persistent world's module). Triggers of Type 1 (area transition)
/// alone, as Aurora's: a generic or a trap trigger leads nowhere.
fn destinations_in(ws: &mg_edit::Workspace, game: Option<&mg_rules::GameData>) -> Vec<Destination> {
    let of = |key: &ResKey, root: &mg_gff::Struct| -> Vec<Destination> {
        let mut out = Vec::new();
        for (kind, ..) in LEADS_TO {
            for (index, s) in root.list(kind.list()).unwrap_or(&[]).iter().enumerate() {
                let tag = decode(s.string("Tag").unwrap_or_default());
                if tag.is_empty() || (kind == ObjectKind::Trigger && s.integer("Type") != Some(1)) {
                    continue;
                }
                let at = match kind {
                    ObjectKind::Trigger => mg_edit::transition::waypoint_position(s),
                    _ => mg_module::instances::placement_of(kind.restype(), s).0.position,
                };
                let at = glam::Vec2::new(at[0], at[1]);
                let name = crate::tree::blueprint_name(game, s).trim().to_string();
                let flags = if kind == ObjectKind::Door { 1 } else { 2 };
                let object = Placed::new(key.resref, kind, index);
                out.push(Destination { object, area: key.resref, tag, name, flags, at });
            }
        }
        out
    };
    let mut out = Vec::new();
    let mut unread: Vec<(ResKey, &[u8])> = Vec::new();
    for key in ws.module.keys_of(ResType::GIT) {
        match (ws.loaded(key), ws.module.get(key)) {
            (Some(g), _) => out.extend(of(key, &g.root)),
            (None, Some(data)) => unread.push((*key, data)),
            (None, None) => {}
        }
    }
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let each = unread.len().div_ceil(threads).max(1);
    let of = &of;
    let read: Vec<Vec<Destination>> = std::thread::scope(|s| {
        let readers: Vec<_> = (unread.chunks(each))
            .map(|some| {
                s.spawn(move || {
                    let mut out = Vec::new();
                    for (key, data) in some {
                        if let Ok(g) = mg_gff::Gff::read(data) {
                            out.extend(of(key, &g.root));
                        }
                    }
                    out
                })
            })
            .collect();
        readers.into_iter().map(|r| r.join().unwrap_or_default()).collect()
    });
    out.extend(read.into_iter().flatten());
    out.sort_by(|a, b| (a.area, &a.tag, a.object).cmp(&(b.area, &b.tag, b.object)));
    out
}

/// A line under Setup Area Transition's lists: what OK will do, or why
/// it can't.
#[derive(Debug, Clone, PartialEq)]
enum Said {
    /// What it does.
    Does(String),
    /// Worth a look before OK (a tag shared, a link replaced).
    Warns(String),
    /// Why OK can't be pressed.
    Refuses(String),
}

/// What OK would do for one destination and connection, worked out once
/// for each (and again once the module changes).
#[derive(Debug, Clone)]
struct Worked {
    /// What it is for: the destination, the connection and the
    /// workspace's revision.
    of: (Placed, Connection, u64),
    plan: Option<mg_edit::transition::Plan>,
    said: Vec<Said>,
}

/// The Setup Area Transition window's state (Aurora's
/// `TdlgAreaTransition`): the doors, triggers and waypoints gathered as it
/// opened, the area shown and its map, the one chosen, the connection.
#[derive(Debug, Clone, Default)]
struct TransitionPick {
    open: bool,
    destinations: Vec<Destination>,
    area: Option<mg_core::ResRef>,
    map: Option<crate::minimap::Map>,
    chosen: Option<Placed>,
    /// Which kind the map and lists show (`None`: all).
    showing: Option<ObjectKind>,
    connection: Connection,
    /// The placed objects the window edits (left out of the lists: a
    /// trigger can't lead to itself, nor one of several to one of them).
    editing: Vec<Placed>,
    worked: Option<Worked>,
    /// OK asked for by a double click before what it chose was worked out.
    confirm: bool,
}

/// The side of Setup Area Transition's map, points.
const TRANSITION_MAP: f32 = 320.0;

/// How near the pointer must be to a door, trigger or waypoint on the
/// map to take it, points.
const MARKER_REACH: f32 = 9.0;

/// The object a form sets up a transition from: the one placed object
/// it edits, else each of its blueprints or objects.
fn current_of(f: &Form<'_>) -> Current {
    match Placed::of(&f.key, &f.path) {
        Some(p) if f.also.is_empty() => Current::Placed(p),
        _ => Current::Fields(f.targets().map(|(k, p)| (k, p.clone())).collect()),
    }
}

/// Why a connection can't be chosen, as the window says it.
fn refused_text(r: Refused) -> &'static str {
    match r {
        Refused::WaypointTarget => "A waypoint leads nowhere: Current To Target only",
        Refused::NotOnePlaced => {
            "A blueprint or several objects can't be led back to: Current To Target only"
        }
        Refused::NoCurrentTag => "This object has no tag for the target to lead back to",
        Refused::NoTargetTag => "The target has no tag to lead to",
        Refused::Itself => "An object can't lead to itself",
        Refused::Missing => "That object is no longer there: pick it again",
    }
}

/// Whether placed object `s` (of `kind`) still has its blueprint's tag:
/// the blueprint its `TemplateResRef` names, the module's, else the
/// game's.
fn tag_is_blueprints(app: &mut Moonglow, kind: ObjectKind, s: &mg_gff::Struct) -> bool {
    let Some(r) = s.resref("TemplateResRef").filter(|r| !r.is_empty()) else { return false };
    let key = ResKey::new(r, kind.restype());
    let tag = s.string("Tag").unwrap_or_default();
    let of = |g: &mg_gff::Gff| g.root.string("Tag") == Some(tag);
    if let Some(ws) = app.ws.as_mut()
        && ws.module.contains(&key)
    {
        return ws.doc(&key).ok().is_some_and(of);
    }
    let Some(data) = app.game.as_ref().and_then(|g| g.resman.get(&key).ok()) else { return false };
    mg_gff::Gff::read(&data).ok().is_some_and(|g| of(&g))
}

/// What OK would do from `current` to `to` by `connection`, and the
/// window's lines of it.
fn work_out(app: &mut Moonglow, current: &Current, to: Placed, connection: Connection) -> Worked {
    let revision = app.ws.as_ref().map_or(0, |ws| ws.revision());
    let of = (to, connection, revision);
    let Some(ws) = app.ws.as_mut() else { return Worked { of, plan: None, said: Vec::new() } };
    let plan = match mg_edit::transition::plan(ws, current, to, connection) {
        Ok(plan) => plan,
        Err(PlanError::Refused(r)) => {
            return Worked { of, plan: None, said: vec![Said::Refuses(refused_text(r).into())] };
        }
        Err(PlanError::Edit(e)) => {
            return Worked { of, plan: None, said: vec![Said::Refuses(e.to_string())] };
        }
    };
    let object = |ws: &mut mg_edit::Workspace, p: Placed| {
        ws.doc(&p.git()).ok().and_then(|g| p.path().get(&g.root).cloned())
    };
    let target = object(ws, to);
    let here = match current {
        Current::Placed(p) => object(ws, *p).map(|s| (p.kind, s)),
        Current::Fields(_) => None,
    };
    let mut said = Vec::new();
    let area = |app: &mut Moonglow, a| crate::tabs::area_label(app, a);
    for (w, tag) in &plan.made {
        let name = area(app, w.area);
        said.push(Said::Does(format!("Makes the waypoint {} in {name}", decode(tag))));
    }
    for (w, tag) in &plan.reused {
        let name = area(app, w.area);
        said.push(Said::Does(format!("Uses the waypoint {} already in {name}", decode(tag))));
    }
    // A tag that names a link, still its blueprint's.
    if connection.links_target()
        && let Some((kind, s)) = &here
        && tag_is_blueprints(app, *kind, s)
    {
        let tag = decode(s.string("Tag").unwrap_or_default());
        said.push(Said::Warns(format!(
            "This object's tag ({tag}) is still its blueprint's: other objects may share it"
        )));
    }
    if connection.links_current()
        && to.kind != ObjectKind::Waypoint
        && let Some(s) = &target
        && tag_is_blueprints(app, to.kind, s)
    {
        let tag = decode(s.string("Tag").unwrap_or_default());
        said.push(Said::Warns(format!(
            "The target's tag ({tag}) is still its blueprint's: other objects may share it"
        )));
    }
    const SHOWN: usize = 3;
    for r in plan.replaced.iter().take(SHOWN) {
        let who = if r.tag.is_empty() { "This blueprint".to_string() } else { decode(&r.tag) };
        said.push(Said::Warns(format!(
            "{who} already leads to {}: OK replaces that link",
            decode(&r.was)
        )));
    }
    if plan.replaced.len() > SHOWN {
        let more = plan.replaced.len() - SHOWN;
        let links = if more == 1 { "link" } else { "links" };
        said.push(Said::Warns(format!("And {more} more {links} replaced")));
    }
    for a in &plan.writes {
        if let Some(why) = crate::claims::read_only(app, *a) {
            let name = area(app, *a);
            said.push(Said::Refuses(format!(
                "{name} is read-only here ({why}): this connection would change it"
            )));
        }
    }
    Worked { of, plan: Some(plan), said }
}

/// A trigger's marker on the map: a diamond.
fn diamond(painter: &egui::Painter, at: egui::Pos2, fill: egui::Color32, stroke: egui::Stroke) {
    let r = 5.5;
    let points = vec![
        at + egui::vec2(0.0, -r),
        at + egui::vec2(r, 0.0),
        at + egui::vec2(0.0, r),
        at + egui::vec2(-r, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(points, fill, stroke));
}

/// Setup Area Transition (Aurora's `TdlgAreaTransition`): the target
/// area, its map with its doors (squares), area transition triggers
/// (diamonds) and waypoints (dots) to click, their lists beside, and the
/// Connection Type. OK applies the links (and the waypoints a trigger is
/// given) as one step.
fn setup_transition(f: &mut Form<'_>, ui: &mut Ui, pick_id: egui::Id) {
    let mut state: TransitionPick = ui.data(|d| d.get_temp(pick_id)).unwrap_or_default();
    if !state.open {
        return;
    }
    let current = current_of(f);
    let current_tag = f.root.string("Tag").unwrap_or_default().to_vec();
    let mut areas: Vec<mg_core::ResRef> = state.destinations.iter().map(|d| d.area).collect();
    areas.dedup();
    let names: std::collections::HashMap<mg_core::ResRef, String> =
        areas.iter().map(|a| (*a, crate::tabs::area_label(f.app, *a))).collect();
    // (By name, as the module tree lists them: a builder's report, they
    // were by ResRef.)
    areas.sort_by_cached_key(|a| names.get(a).map(|n| n.to_lowercase()));
    let name = |a: &Option<mg_core::ResRef>| {
        a.map_or_else(String::new, |a| names.get(&a).cloned().unwrap_or_else(|| a.to_string()))
    };
    let chosen_kind = state.chosen.map(|p| p.kind);
    // (A connection the target can't take gives way to Current To Target,
    // as Aurora's choosing a waypoint does.)
    let refusal =
        |c| chosen_kind.and_then(|k| mg_edit::transition::refusal(&current, &current_tag, k, c));
    if refusal(state.connection).is_some() {
        state.connection = Connection::CurrentToTarget;
    }
    // What OK does, worked out again when the choice or the module changes.
    let revision = f.app.ws.as_ref().map_or(0, |ws| ws.revision());
    state.worked = match state.chosen {
        Some(to) => match state.worked.take() {
            Some(w) if w.of == (to, state.connection, revision) => Some(w),
            _ => Some(work_out(f.app, &current, to, state.connection)),
        },
        None => None,
    };
    let ctx = ui.ctx().clone();
    // (A double click in the frame before, on a row or the map: OK, now
    // that what it chose is worked out.)
    let (mut ok, mut cancel) = (std::mem::take(&mut state.confirm), false);
    // (Sized as the builder drags its edges: the lists take what the map
    // leaves.)
    let window = crate::widgets::modal(&ctx, "Setup Area Transition")
        .id(pick_id.with("modal"))
        .resizable(true)
        .default_size([TRANSITION_MAP + 380.0, TRANSITION_MAP + 230.0])
        .min_size([TRANSITION_MAP + 220.0, TRANSITION_MAP + 190.0]);
    window.show(&ctx, |ui| {
        // What it leads to, what OK does, and OK: at the window's foot,
        // the map and the lists taking the room above.
        egui::Panel::bottom(pick_id.with("foot")).frame(egui::Frame::NONE).show(ui, |ui| {
            ui.add_space(4.0);
            let chosen = state.chosen.and_then(|p| state.destinations.iter().find(|d| d.object == p));
            match chosen {
                Some(d) => {
                    ui.label(format!("Leads to the {} tagged {}", kind_name(d.object.kind), d.tag));
                }
                None => {
                    ui.weak(
                        "Click a door (a square), a trigger (a diamond) or a waypoint (a dot) on the \
                         map, or in a list",
                    );
                }
            }
            let worked = state.worked.as_ref().filter(|w| Some(w.of.0) == state.chosen);
            for said in worked.map_or(&[][..], |w| &w.said) {
                match said {
                    Said::Does(t) => {
                        ui.weak(t);
                    }
                    Said::Warns(t) => {
                        ui.colored_label(ui.visuals().warn_fg_color, t);
                    }
                    Said::Refuses(t) => {
                        ui.colored_label(ui.visuals().error_fg_color, t);
                    }
                }
            }
            let can = worked.is_some_and(|w| {
                w.plan.is_some() && !w.said.iter().any(|s| matches!(s, Said::Refuses(_)))
            });
            ui.horizontal(|ui| {
                ok |= ui
                    .add_enabled(can, egui::Button::new("OK"))
                    .on_hover_text("Set up the transition, one step to undo")
                    .clicked()
                    || (can && crate::widgets::enter(ui));
                cancel = crate::widgets::cancel_discard(ui);
            });
        });
        if areas.is_empty() {
            ui.weak("No doors, triggers or waypoints with tags in the module's areas");
        }
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Target Area");
            let options: Vec<Option<mg_core::ResRef>> = areas.iter().copied().map(Some).collect();
            crate::widgets::dropdown(pick_id.with("area"))
                .selected_text(name(&state.area))
                .width(260.0)
                .show(ui, &mut state.area, &options, name);
            ui.separator();
            // Doors, triggers, waypoints, or all, on the map and in the
            // lists (a group named for them: the area toolbar's and the
            // palette's Doors are others).
            ui.horizontal(|ui| {
                mg_uikit::name_group(ui, "Destinations Shown");
                ui.selectable_value(&mut state.showing, None, "All");
                for (kind, .., label) in [LEADS_TO[0], LEADS_TO[2], LEADS_TO[1]] {
                    ui.selectable_value(&mut state.showing, Some(kind), label);
                }
            });
        });
        // Aurora's Connection Type: which ends are linked.
        ui.horizontal(|ui| {
            mg_uikit::name_group(ui, "Connection Type");
            crate::widgets::field_label(ui, "Connection Type");
            for c in Connection::ALL {
                let why = refusal(c);
                let r = ui
                    .add_enabled(
                        why.is_none(),
                        egui::RadioButton::new(state.connection == c, c.label()),
                    )
                    .on_hover_text(match c {
                        Connection::BothWays => "Each leads to the other",
                        Connection::CurrentToTarget => {
                            "This object leads there, the target is unchanged"
                        }
                        Connection::TargetToCurrent => {
                            "The target leads here, this object is unchanged"
                        }
                    });
                if let Some(why) = why {
                    r.clone().on_disabled_hover_text(refused_text(why));
                }
                if r.clicked() {
                    state.connection = c;
                }
            }
        });
        if let Current::Fields(_) = current {
            ui.weak(refused_text(Refused::NotOnePlaced));
        } else if current_tag.is_empty() {
            ui.weak(refused_text(Refused::NoCurrentTag));
        }
        let showing = state.showing;
        let here: Vec<Destination> = (state.destinations.iter())
            .filter(|d| Some(d.area) == state.area && showing.is_none_or(|k| d.object.kind == k))
            .filter(|d| !state.editing.contains(&d.object))
            .cloned()
            .collect();
        let is = |d: &Destination, chosen: &Option<Placed>| *chosen == Some(d.object);
        ui.horizontal_top(|ui| {
            // The map: a click on a door, trigger or waypoint takes it.
            let shown = state.area.and_then(|a| crate::minimap::of(f.app, &ctx, a, &mut state.map));
            // (Smaller where the window is: the lines under it keep
            // their room.)
            let side = ui.available_height().clamp(TRANSITION_MAP / 2.0, TRANSITION_MAP);
            if let Some(map) = shown.cloned() {
                let (rect, r) = ui.allocate_exact_size(egui::Vec2::splat(side), egui::Sense::click());
                r.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "The area's map")
                });
                // (In the middle of its square, a long area's.)
                let placed = map.placed(rect.right_top(), side);
                let rect = placed.translate(egui::vec2(
                    (placed.width() - rect.width()) / 2.0,
                    (rect.height() - placed.height()) / 2.0,
                ));
                let painter = ui.painter_at(rect.expand(6.0));
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
                    let fill = match d.object.kind {
                        _ if lit => egui::Color32::YELLOW,
                        ObjectKind::Door => egui::Color32::from_rgb(90, 160, 255),
                        ObjectKind::Trigger => egui::Color32::from_rgb(230, 140, 60),
                        _ => egui::Color32::from_rgb(90, 220, 120),
                    };
                    let stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);
                    match d.object.kind {
                        ObjectKind::Door => {
                            let square = egui::Rect::from_center_size(at, egui::Vec2::splat(8.0));
                            painter.rect_filled(square, 1.0, fill);
                            painter.rect_stroke(square, 1.0, stroke, egui::StrokeKind::Outside);
                        }
                        ObjectKind::Trigger => diamond(&painter, at, fill, stroke),
                        _ => {
                            painter.circle_filled(at, 4.0, fill);
                            painter.circle_stroke(at, 4.0, stroke);
                        }
                    }
                }
                if let Some(d) = near {
                    r.clone().on_hover_text(format!("{} ({})", d.tag, kind_name(d.object.kind)));
                }
                if r.clicked()
                    && let Some(d) = near
                {
                    state.chosen = Some(d.object);
                }
                if r.double_clicked() && near.is_some() {
                    ok = true;
                }
            }
            // The lists: the area's doors, triggers, then its waypoints.
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt(pick_id.with("lists"))
                    .max_height(ui.available_height())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        mg_uikit::list_rows(ui);
                        for (kind, title, ..) in LEADS_TO {
                            if showing.is_some_and(|k| k != kind) {
                                continue;
                            }
                            ui.strong(title);
                            for d in here.iter().filter(|d| d.object.kind == kind) {
                                // (Its tag, which the transition names,
                                // and its name beside.)
                                let mut text = egui::text::LayoutJob::default();
                                let body = egui::TextFormat::default();
                                text.append(
                                    &d.tag,
                                    0.0,
                                    egui::TextFormat {
                                        color: ui.visuals().text_color(),
                                        font_id: egui::TextStyle::Body.resolve(ui.style()),
                                        ..body.clone()
                                    },
                                );
                                if !d.name.is_empty() && d.name != d.tag {
                                    text.append(
                                        &format!("  {}", d.name),
                                        0.0,
                                        egui::TextFormat {
                                            color: ui.visuals().weak_text_color(),
                                            font_id: egui::TextStyle::Body.resolve(ui.style()),
                                            ..body
                                        },
                                    );
                                }
                                let r = ui
                                    .selectable_label(is(d, &state.chosen), text)
                                    .on_hover_text(format!("Tag {}", d.tag));
                                if r.clicked() {
                                    state.chosen = Some(d.object);
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
    });
    // (A double click on a row or the map: as OK, where OK can be pressed.)
    let plan = state.worked.as_ref().filter(|w| Some(w.of.0) == state.chosen);
    let plan = plan
        .filter(|w| !w.said.iter().any(|s| matches!(s, Said::Refuses(_))))
        .and_then(|w| w.plan.clone());
    // (A double click picks first: worked out in the next frame, OK then.)
    let ok_now = ok && plan.is_some();
    if ok && !ok_now && state.worked.as_ref().is_none_or(|w| Some(w.of.0) != state.chosen) {
        state.confirm = true;
        ctx.request_repaint();
    }
    if let Some(plan) = plan.filter(|_| ok_now) {
        apply_transition(f, plan);
    }
    if ok_now || cancel {
        state = TransitionPick::default();
    }
    ui.data_mut(|d| d.insert_temp(pick_id, state));
}

/// Applies a transition set up, as one step: the links, and the
/// waypoints made announced as Create Waypoint's are (their tags weighed
/// for Duplicate Tags, the plugins told). The Properties window's Cancel
/// leaves it (Undo takes it back whole): set back on the current object
/// alone, it would leave the target leading to an object that leads
/// elsewhere.
fn apply_transition(f: &mut Form<'_>, plan: mg_edit::transition::Plan) {
    let made: Vec<mg_plugin::Selected> = (plan.made.iter())
        .map(|(w, _)| mg_plugin::Selected::Object { area: w.area, kind: w.kind, index: w.index })
        .collect();
    let made = crate::plugin_events::Made::objects("setup-area-transition", made, false);
    f.app.actions.push(Action::ApplyMade(plan.command, made));
    let targets: Vec<(ResKey, mg_edit::GffPath)> =
        f.targets().map(|(k, p)| (k, p.clone())).collect();
    if let Some(changes) = f.app.properties_changed.get_mut(&targets[0]) {
        changes.was.retain(|w| {
            !matches!(w, super::Was::Field(k, p, l, _)
                if targets.iter().any(|(tk, tp)| tk == k && tp == p)
                    && (l == "LinkedTo" || l == "LinkedToFlags"))
        });
    }
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
                if ui.radio_button(flags == v, text).clicked() && flags != v {
                    f.set_int("Destination type", "LinkedToFlags", v, FieldType::Byte);
                }
            }
        });
        ui.end_row();
        crate::widgets::field_label(ui, "");
        // Aurora's Setup Area Transition: a door, trigger or waypoint in
        // an area, picked on its map or from its lists (gathered as the
        // window opens: from every area's objects).
        let pick_id = f.id("transition-setup");
        if ui
            .button("Setup Area Transition…")
            .on_hover_text("Pick the door, trigger or waypoint it leads to, and how")
            .clicked()
        {
            let destinations = destinations(f);
            let editing: Vec<Placed> = f.targets().filter_map(|(k, p)| Placed::of(&k, p)).collect();
            let now = now_leads_to(f, &destinations, &editing);
            let here = editing.first().and_then(|c| destinations.iter().find(|d| d.area == c.area));
            let state = TransitionPick {
                open: true,
                area: now.or(here).or(destinations.first()).map(|d| d.area),
                chosen: now.map(|d| d.object),
                destinations,
                editing,
                ..Default::default()
            };
            ui.data_mut(|d| d.insert_temp(pick_id, state));
        }
        setup_transition(f, ui, pick_id);
        ui.end_row();
        crate::widgets::field_label(ui, "Loading Screen");
        f.load_screen(ui, &loadscreens);
        ui.end_row();
    });
}

/// The destination the object leads to now, to open the window on: a
/// door of its `LinkedTo` (flags 1); a trigger whose waypoint it names
/// (flags 2, `WP_` and the trigger's tag), else a waypoint of that tag.
fn now_leads_to<'d>(
    f: &Form<'_>,
    destinations: &'d [Destination],
    editing: &[Placed],
) -> Option<&'d Destination> {
    let linked = decode(f.root.string("LinkedTo").unwrap_or_default());
    if linked.is_empty() {
        return None;
    }
    let flags = f.root.integer("LinkedToFlags").unwrap_or(0);
    let of = |kind: ObjectKind, tag: &str| {
        destinations
            .iter()
            .find(|d| d.object.kind == kind && d.tag == tag && !editing.contains(&d.object))
    };
    match flags {
        1 => of(ObjectKind::Door, &linked),
        _ => linked
            .strip_prefix("WP_")
            .and_then(|t| of(ObjectKind::Trigger, t))
            .or_else(|| of(ObjectKind::Waypoint, &linked)),
    }
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
            if ui.check_box(&mut no_interrupt, "No Interrupt").changed() {
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
    of: egui::Id,
    name: &str,
    choices: &[Choice],
    current: i64,
) -> Option<i64> {
    pick_list(of, name, choices, current, 220.0).show(ui)
}

/// [`pick`] in the order given, however long (a cost table's values,
/// from the least).
pub(super) fn pick_ordered(
    ui: &mut Ui,
    of: egui::Id,
    name: &str,
    choices: &[Choice],
    current: i64,
) -> Option<i64> {
    pick_list(of, name, choices, current, 220.0).ordered().show(ui)
}

/// [`pick_ordered`], `width` wide, for a body part: the mouse wheel over
/// it, closed, steps through its choices too (a builder's ask: to see each
/// on the model without opening the list).
pub(super) fn pick_part(
    ui: &mut Ui,
    of: egui::Id,
    name: &str,
    choices: &[Choice],
    current: i64,
    width: f32,
) -> Option<i64> {
    pick_list(of, name, choices, current, width).ordered().wheel().show(ui)
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
/// `of`: its window's own id for its lists ([`Form::picks`]). By the
/// document alone, two Properties windows of one area's objects side by
/// side shared a list's state.
pub(super) fn pick_list<'a>(
    of: egui::Id,
    name: &str,
    choices: &'a [Choice],
    current: i64,
    width: f32,
) -> crate::widgets::RowChoice<'a> {
    crate::widgets::row_choice(of.with(name), choices, current).width(width)
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

    /// Setup Area Transition's doors and waypoints are read without
    /// leaving every area's objects in the workspace (the review's B21),
    /// and an area the workspace has read is taken as it is there, edited.
    #[test]
    fn destinations_are_read_without_keeping_every_area() {
        use mg_gff::{Gff, Struct};
        let git = |name: &str| ResKey::parse(name, ResType::GIT).unwrap();
        let area = |tags: &[(&str, &str)]| {
            let mut g = Gff::new(*b"GIT ");
            for (list, tag) in tags {
                let mut s = Struct::new(0);
                s.set("Tag", Value::String(tag.as_bytes().to_vec()));
                // (A trigger's Type by its tag: TT_GO a transition, TT_TRAP
                // a trap, the others generic.)
                if *list == "TriggerList" {
                    let kind = [("TT_GO", 1), ("TT_TRAP", 2)].iter().find(|(t, _)| t == tag);
                    s.set("Type", Value::Int(kind.map_or(0, |k| k.1)));
                }
                let mut all = g.root.list(list).unwrap_or(&[]).to_vec();
                all.push(s);
                g.root.set(list, Value::List(all));
            }
            g.to_bytes().unwrap()
        };
        let mut m = mg_module::Module::new();
        let inn = [
            ("WaypointList", "WP_INN"),
            ("Door List", "DR_INN"),
            ("TriggerList", "TT_GENERIC"),
            ("TriggerList", "TT_GO"),
            ("TriggerList", "TT_TRAP"),
            ("TriggerList", "TT_GO"),
        ];
        m.set(git("inn"), area(&inn));
        m.set(git("road"), area(&[("WaypointList", "WP_ROAD"), ("WaypointList", "")]));
        for i in 0..40 {
            let tag = format!("WP_{i:02}");
            m.set(git(&format!("far{i:02}")), area(&[("WaypointList", tag.as_str())]));
        }
        let mut ws = mg_edit::Workspace::new(m);
        // The inn read and its waypoint's tag changed, not yet written
        // into the module.
        ws.doc(&git("inn")).unwrap();
        let retag = mg_edit::Edit::SetField {
            key: git("inn"),
            path: mg_edit::GffPath::root().item("WaypointList", 0),
            label: "Tag".into(),
            value: Some(Value::String(b"WP_CELLAR".to_vec())),
        };
        ws.apply(mg_edit::Command::new("Tag", vec![retag])).unwrap();
        let found = destinations_in(&ws, None);
        let of = |area: &str| -> Vec<(String, i64)> {
            let area = git(area).resref;
            found.iter().filter(|d| d.area == area).map(|d| (d.tag.clone(), d.flags)).collect()
        };
        // Area transition triggers alone (generic and trap ones lead
        // nowhere), two of one tag as two, each its own object.
        let inn = |t: &str| (t.to_string(), if t.starts_with("DR") { 1 } else { 2 });
        assert_eq!(of("inn"), [inn("DR_INN"), inn("TT_GO"), inn("TT_GO"), inn("WP_CELLAR")]);
        let gos: Vec<Placed> =
            found.iter().filter(|d| d.tag == "TT_GO").map(|d| d.object).collect();
        let tt = |i| Placed::new(git("inn").resref, ObjectKind::Trigger, i);
        assert_eq!(gos, [tt(1), tt(3)]);
        assert_eq!(of("road"), [("WP_ROAD".to_string(), 2)], "one without a tag left out");
        assert_eq!(found.len(), 5 + 40);
        assert!(found.windows(2).all(|w| (w[0].area, &w[0].tag) <= (w[1].area, &w[1].tag)));
        assert!(ws.loaded(&git("inn")).is_some());
        assert!(ws.loaded(&git("road")).is_none(), "read, and not kept");
        assert!((0..40).all(|i| ws.loaded(&git(&format!("far{i:02}"))).is_none()));
    }

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
