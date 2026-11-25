//! The area editor's header (M4's U3; the proposal's sections 5.2, 5.3,
//! 8.1 and 9.1; the Main, Narrow and WideArea boards): one row on the
//! kit's [`EditorHeader`], named "Area header".
//!
//! The left group is what you do: the ten Show toggles as a joined group
//! of icons with All and None on the menu beside it (under an editor 990
//! wide, one Show popover whose button carries the count, "Show 5 of
//! 10"), Snap and Turn (their value text gone under 550), Together and
//! Onto Objects (into More under 490). While Paint is the tool in hand
//! (the shelf's, `tools`), Paint's options take the group instead
//! (`paint_mode::header_items`: the mode, Radius, Density, the Brush
//! popover with Snap and Turn), the Show popover in More. The right group
//! is how the view looks: Day | Night (one moon toggle under 990), the
//! Lighting, Overlays and Audio popovers (icons under 990; unfolded into
//! the row where it has room, section 5.3), Area Properties (its icon
//! under 990) and the warning for what could not be read. More holds To
//! Scratch and Save as Prefab…. A right click on the row is the menu of
//! which controls to show (`Settings::hidden_area_tools`, by the names it
//! has always kept; Select Tiles and Paint are the shelf's, Reorient
//! Camera and Go to Start Location the compass's).
//!
//! It yields by the editor's width (`mg_uikit::shell::Yields`), as every
//! editor's header does (Options' old "Editors' toolbar buttons" is
//! retired: M4's U23).

use std::cell::Cell;

use egui::{FontId, Ui, vec2};
use mg_area::ObjectKind;
use mg_core::ResType;
use mg_resman::ResKey;
use mg_uikit::header::{EditorHeader, Item, Joined};
use mg_uikit::popover::{PopAction, PopCheck, Popover};
use mg_uikit::{Controls as _, Icon, KitButton, metrics};

use super::{AreaView, Fade, copy_selection, held_menu, share_shown};
use crate::{Action, Moonglow};

/// The header's accessible name (its group), which tests find its
/// controls in.
pub(crate) const AREA_HEADER: &str = "Area header";

/// The compact header's Show popover, and the full header's menu of All
/// and None beside the kind toggles.
pub(crate) const SHOW: &str = "Show";
pub(crate) const ALL_OR_NONE: &str = "Show All or None";
/// The Lighting popover's master switch (the view's lighting on or off).
pub(crate) const USE_LIGHTING: &str = "Use the area's lighting";
/// The Overlays popover's master switch.
pub(crate) const SHOW_OVERLAYS: &str = "Show overlays";
/// The warning of what could not be read.
pub(crate) const WARNINGS: &str = "Warnings";
/// Together and Onto Objects, one joined group.
pub(crate) const PLACING: &str = "Placing";

/// The width of the arrow at the end of the Show toggles (`.join.kinds`'
/// last button: 16 px).
const ARROW_WIDTH: f32 = 16.0;

/// The gap between a combo box's lead icon and its box.
const LEAD_GAP: f32 = 4.0;

/// `words`' width in the header's text (the kit's body size).
fn text_width(ui: &Ui, words: &str) -> f32 {
    let font = FontId::proportional(metrics::BODY_SIZE);
    ui.painter().layout_no_wrap(words.to_string(), font, egui::Color32::WHITE).size().x
}

/// The width of a combo box showing the widest of `values` (the kit's
/// `button_padding`, 8 at each side; the chevron, 14, 6 after the
/// words), the same in every frame whatever it shows.
fn combo_width(ui: &Ui, values: &[String]) -> f32 {
    let widest = values.iter().map(|v| text_width(ui, v)).fold(0.0, f32::max);
    (widest + 6.0 + 14.0 + 2.0 * metrics::CONTROL_PADDING_X).ceil()
}

/// A combo box's lead icon (`.combo .ic`, `text-2`), `name` its tip.
fn lead_icon(ui: &mut Ui, icon: Icon, name: &str) {
    let side = vec2(metrics::ICON, metrics::CONTROL_HEIGHT);
    let (rect, r) = ui.allocate_exact_size(side, egui::Sense::hover());
    let square = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(metrics::ICON));
    icon.paint(ui.painter(), square, mg_uikit::tokens(ui.ctx()).text_2);
    r.on_hover_text(name);
}

/// Lays out a combo box and its lead icon in `ui`: 4 apart, its words
/// never cut (its width is measured for its widest value).
fn combo_row(ui: &mut Ui) {
    ui.spacing_mut().item_spacing.x = LEAD_GAP;
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
}

/// Names a combo box's node `name` (egui's gives it none): tests and a
/// screen reader find Snap and Turn by it.
fn name_combo(ui: &Ui, response: &egui::Response, name: &str) {
    ui.ctx().accesskit_node_builder(response.id, |node| node.set_label(name));
}

/// The Snap grid's choices, by what they show.
const GRIDS: [Option<u16>; 6] = [None, Some(25), Some(50), Some(100), Some(250), Some(500)];
/// The Turn angle's choices.
const ANGLES: [Option<u16>; 5] = [None, Some(5), Some(15), Some(45), Some(90)];

fn grid_text(v: Option<u16>) -> String {
    v.map_or("off".to_string(), |cm| format!("{} m", f32::from(cm) / 100.0))
}

fn angle_text(v: Option<u16>) -> String {
    v.map_or("free".to_string(), |a| format!("{a}°"))
}

/// The Snap combo box (`value`: the grid, `text` whether it shows it).
fn snap_combo(ui: &mut Ui, value: &Cell<Option<u16>>, width: f32, text: bool) {
    let mut v = value.get();
    let shown = if text { grid_text(v) } else { String::new() };
    combo_row(ui);
    lead_icon(ui, crate::icons::SNAP, "Snap");
    let r = crate::widgets::dropdown("snap-grid")
        .selected_text(shown)
        .width(width)
        .show(ui, &mut v, &GRIDS, |g| grid_text(*g))
        .on_hover_text("Snap: moved and placed objects snap to this grid");
    name_combo(ui, &r, "Snap");
    value.set(v);
}

/// The Turn combo box, with any other angle in whole degrees.
fn turn_combo(ui: &mut Ui, value: &Cell<Option<u16>>, width: f32, text: bool) {
    let mut v = value.get();
    let shown = if text { angle_text(v) } else { String::new() };
    combo_row(ui);
    lead_icon(ui, crate::icons::TURN, "Turn");
    let r = crate::widgets::dropdown("snap-angle")
        .selected_text(shown)
        .width(width)
        .show_with(
            ui,
            &mut v,
            || ANGLES.to_vec(),
            |ui, snap| {
                for a in ANGLES {
                    ui.selectable_value(snap, a, angle_text(a));
                }
                // Any other angle, in whole degrees.
                ui.horizontal(|ui| {
                    ui.label("Other");
                    let mut degrees = snap.unwrap_or(30);
                    let field = egui::DragValue::new(&mut degrees).range(1..=180).suffix("°");
                    if ui.add(field).changed() {
                        *snap = Some(degrees);
                    }
                });
            },
        )
        .on_hover_text(
            "Turn: the angle turned objects snap to, and Q and E turn by (15° when free)",
        );
    name_combo(ui, &r, "Turn");
    value.set(v);
}

/// The Fade Geometry combo box.
fn fade_combo(ui: &mut Ui, value: &Cell<Fade>, width: f32) {
    let mut v = value.get();
    combo_row(ui);
    lead_icon(ui, crate::icons::FADE, "Fade Geometry");
    let tip =
        "Fade geometry: hide the roofs and upper walls the game fades, to see what is under them";
    let r = crate::widgets::dropdown("fade-geometry")
        .selected_text(v.name())
        .width(width)
        .show(ui, &mut v, &Fade::ALL, |f| f.name())
        .on_hover_text(tip);
    name_combo(ui, &r, "Fade Geometry");
    value.set(v);
}

/// What the header changes, read from the view and the settings before
/// it is drawn and written back after (its items hold them while it is).
#[derive(Clone, Copy, PartialEq)]
struct Shadows {
    sun: bool,
    creatures: bool,
    lights: bool,
}

/// Draws the area's header over its view and does what was asked of it.
pub(super) fn header(app: &mut Moonglow, ui: &mut Ui, view: &mut AreaView) {
    let hidden = &app.settings.hidden_area_tools;
    let shown = |name: &str| !hidden.iter().any(|h| h == name);
    let window = ui.ctx().content_rect().width();
    let yields = mg_uikit::shell::Yields::of(window, ui.available_width());
    let compact = yields.header_compact;
    let form = match (compact, yields.wide) {
        (true, _) => PopForm::Compact,
        (false, false) => PopForm::Words,
        (false, true) => PopForm::Unfolds,
    };
    let editable = view.read_only.is_none();

    // (What its items change, as locals: several are kept inverted or
    // elsewhere, `!no_grass`, the shaders' switch.)
    let was_show = (view.show, view.show_start);
    let mut show = view.show;
    let mut show_start = view.show_start;
    let all_or_none: Cell<Option<bool>> = Cell::new(None);
    let night = Cell::new(view.night);
    let mut moon = view.night;
    let mut lit = view.lit;
    let kinds_were =
        Shadows { sun: view.shadows, creatures: view.creature_shadows, lights: view.light_shadows };
    let mut kinds = kinds_were;
    // (The unfolded Shadows toggle's, apart from the popover's rows: one
    // of the two is drawn.)
    let unfolded_kinds = Cell::new(kinds_were);
    let shadows_before = Cell::new(view.shadows_before);
    let quality = Cell::new(app.settings.shadow_quality);
    let mut fog = view.fog;
    let fade = Cell::new(view.fade);
    let mut animated = !app.settings.still_objects;
    let mut grass = !app.settings.no_grass;
    let mut tile_border = app.settings.tile_border;
    let shaders_were = app.game_shaders();
    let mut shaders = shaders_were;
    let mut overlays = !view.hide_overlays;
    let overlays_were = overlays;
    let mut grid = view.grid;
    let mut minimap = view.minimap;
    let walks_were = (view.walkmesh, view.object_walkmesh);
    let (mut ground_walk, mut object_walk) = walks_were;
    let unfolded_walks = Cell::new(walks_were);
    // (A right click on the wide row's Shadows or Walkmesh opens its own
    // menu, not the header's.)
    let own_menu = Cell::new(false);
    let mut sound_ranges = view.sound_ranges;
    let mut placed = !app.settings.no_placed_sounds;
    let mut ambient = app.settings.ambient_sound;
    let mut music = app.settings.ambient_music;
    let snap = Cell::new(app.settings.snap_grid);
    let turn = Cell::new(app.settings.snap_angle);
    let mut together = app.settings.turn_together;
    let mut onto = app.settings.place_onto_objects;
    // (Paint's options take the left group while it is the tool in hand:
    // the AreaPaint board.)
    let painting = app.paint && editable;
    let paint = Cell::new(app.settings.paint);
    // (And Select Tiles' commands while it is the tool in hand: the
    // AreaTiles board.)
    let tiling = view.tile_mode && editable && !painting;
    let mut tile_properties = !app.settings.hide_tile_properties;
    let tile_tips = tiling.then(|| tiles_tips(app, ui.ctx()));
    let selected = view.selection.len();
    let prefab = (selected > 1).then(|| format!("Save {selected} as Prefab…"));
    let warning = warnings(view);

    // The widths of the parts of the caller's, measured as the header
    // measures its own (the kit's text).
    // (As wide as what they show, the Main board's: measured in the frame
    // they are drawn in, so a value always gives the same row.)
    let snap_width = combo_width(ui, &[grid_text(snap.get())]);
    let turn_width = combo_width(ui, &[angle_text(turn.get())]);
    let compact_combo = combo_width(ui, &[String::new()]);
    let lead = metrics::ICON + LEAD_GAP;
    let fade_width = combo_width(ui, &[fade.get().name().to_string()]);
    let day_night = [("Day", "Show the area by day"), ("Night", "Show the area at night")];
    let day_night_width = day_night
        .iter()
        .map(|(w, _)| text_width(ui, w) + 2.0 * metrics::CONTROL_PADDING_X)
        .sum::<f32>()
        .ceil()
        + 1.0;

    let mut h = EditorHeader::new(AREA_HEADER);

    // The left group: what is shown, and how objects are placed.
    static KIND_NAMES: std::sync::OnceLock<[String; 9]> = std::sync::OnceLock::new();
    let kind_names =
        KIND_NAMES.get_or_init(|| ObjectKind::ALL.map(|k| crate::icons::area_tool_kind(k.label())));
    let mut show_in_more = None;
    if compact || painting || tiling {
        let count = show.iter().filter(|s| **s).count() + usize::from(show_start);
        let mut pop = Popover::new(SHOW).title(Some(Icon::Eye), "Show");
        let mut any = false;
        for ((on, kind), name) in show.iter_mut().zip(ObjectKind::ALL).zip(kind_names.iter()) {
            if shown(name) {
                let row = PopCheck::new(on, kind.label()).name(name.as_str());
                pop = pop.check(row.icon(crate::icons::object(kind)));
                any = true;
            }
        }
        if shown("Start") {
            let row = PopCheck::new(&mut show_start, "Start").name("Show Start");
            pop = pop.check(row.icon(crate::icons::START).tip(START_TIP));
            any = true;
        }
        let (all, none) = (shown("All"), shown("None"));
        if all || none {
            pop = pop.separator();
        }
        if all {
            pop = pop.action(PopAction::new("All").icon(crate::icons::SHOW_ALL).tip("Show All"));
        }
        if none {
            let none = PopAction::new("None").icon(crate::icons::SHOW_NONE).tip("Show None");
            pop = pop.action(none);
        }
        if any || all || none {
            let tip =
                format!("Show: which kinds of placed objects the view draws, {count} of 10 shown");
            let item = Item::popover(Some(Icon::Eye), pop).words_icon(true);
            let item = item.count(format!("{count} of 10")).tip(tip);
            // (While Paint is on, in More after its options: the
            // Outliner's eyes are the board's, but for Start, All and
            // None, kept here.)
            match painting || tiling {
                true => show_in_more = Some(item.in_more()),
                false => h = h.left(item),
            }
        }
    } else {
        let mut parts = Vec::new();
        for ((on, kind), name) in show.iter_mut().zip(ObjectKind::ALL).zip(kind_names.iter()) {
            if shown(name) {
                parts.push(Joined::toggle(crate::icons::object(kind), name.as_str(), on));
            }
        }
        if shown("Start") {
            parts.push(
                Joined::toggle(crate::icons::START, "Show Start", &mut show_start).tip(START_TIP),
            );
        }
        if !parts.is_empty() {
            h = h.left(Item::joined(SHOW, parts));
        }
        let (all, none) = (shown("All"), shown("None"));
        if all || none {
            let pick = &all_or_none;
            let item = Item::custom(ALL_OR_NONE, ARROW_WIDTH, move |ui| {
                all_or_none_arrow(ui, all, none, pick);
            })
            .more(120.0, move |ui| all_or_none_rows(ui, all, none, pick));
            h = h.left(item);
        }
    }
    if painting {
        let (snap, turn) = (&snap, &turn);
        let (show_snap, show_turn) = (shown("Snap"), shown("Turn"));
        let placing = move |ui: &mut Ui| {
            if show_snap {
                snap_combo(ui, snap, snap_width, true);
                ui.add_space(LEAD_GAP);
            }
            if show_turn {
                turn_combo(ui, turn, turn_width, true);
            }
        };
        let placing = (show_snap || show_turn).then_some(placing);
        h = crate::paint_mode::header_items(h, ui, &paint, placing);
    }
    if let Some(tile_tips) = tile_tips.as_ref() {
        let has = !view.tile_selection.is_empty();
        let paste = app.tile_clip.as_ref().zip(view.terrain.as_ref());
        let paste = paste.is_some_and(|(c, t)| c.tileset == t.tileset);
        h = tiles_items(h, has, paste, tile_tips, &mut tile_properties);
    }
    h = show_in_more.into_iter().fold(h, EditorHeader::left);
    h = h.left(Item::separator());
    if shown("Snap") && !painting && !tiling {
        let value = &snap;
        let full = lead + snap_width;
        let mut item =
            Item::custom("Snap", full, move |ui| snap_combo(ui, value, snap_width, true))
                .compact(lead + compact_combo, move |ui| {
                    snap_combo(ui, value, compact_combo, false)
                })
                .more(full, move |ui| {
                    ui.horizontal(|ui| snap_combo(ui, value, snap_width, true));
                });
        if !yields.value_text {
            // (Under 550 the value text goes whatever the room: its compact
            // form at its full width's step and the next.)
            item = Item::custom("Snap", lead + compact_combo, move |ui| {
                snap_combo(ui, value, compact_combo, false)
            })
            .more(full, move |ui| {
                ui.horizontal(|ui| snap_combo(ui, value, snap_width, true));
            });
        }
        h = h.left(item);
    }
    if shown("Turn") && !painting && !tiling {
        let value = &turn;
        let full = lead + turn_width;
        let mut item =
            Item::custom("Turn", full, move |ui| turn_combo(ui, value, turn_width, true))
                .compact(lead + compact_combo, move |ui| {
                    turn_combo(ui, value, compact_combo, false)
                })
                .more(full, move |ui| {
                    ui.horizontal(|ui| turn_combo(ui, value, turn_width, true));
                });
        if !yields.value_text {
            item = Item::custom("Turn", lead + compact_combo, move |ui| {
                turn_combo(ui, value, compact_combo, false)
            })
            .more(full, move |ui| {
                ui.horizontal(|ui| turn_combo(ui, value, turn_width, true));
            });
        }
        h = h.left(item);
    }
    // (One joined group, as the boards draw them.)
    let mut placing = Vec::new();
    if shown("Together") {
        let tip = "Together: turn several selected objects about their middle as one. Off, each \
                   turns in place.";
        placing.push(Joined::toggle(crate::icons::TOGETHER, "Together", &mut together).tip(tip));
    }
    if shown("Onto Objects") {
        let tip =
            "Onto objects: place onto the placeable or door under the pointer, not the ground";
        let part = Joined::toggle(crate::icons::ONTO_OBJECTS, "Onto Objects", &mut onto);
        placing.push(part.tip(tip));
    }
    if !placing.is_empty() && !painting && !tiling {
        let item = Item::joined(PLACING, placing);
        h = h.left(if yields.into_more { item.in_more() } else { item });
    }

    // The right group: how the view looks.
    if shown("Night") {
        if compact {
            let item = Item::toggle(Some(crate::icons::NIGHT), "Night", &mut moon)
                .icon_only()
                .tip("Night: show the area at night (off: by day)");
            h = h.right(item);
        } else {
            let value = &night;
            let seg = move |ui: &mut Ui| {
                let mut v = value.get();
                let choices = [(false, "Day", day_night[0].1), (true, "Night", day_night[1].1)];
                mg_uikit::segmented(ui, "Time of day", &mut v, &choices);
                value.set(v);
            };
            let item = Item::custom("Night", day_night_width, seg).more(day_night_width, seg);
            h = h.right(item);
        }
    }
    let mut lighting = Popover::new("Lighting")
        .title(Some(crate::icons::LIGHTING), "Lighting")
        .rows_follow_switch(false);
    let mut lighting_rows = 0;
    if shown("Lighting") {
        lighting = lighting.switch(&mut lit, USE_LIGHTING).switch_words("Lighting");
        lighting_rows += 1;
    }
    if shown("Shadows") {
        let creatures_on = kinds.creatures;
        let shadow_button =
            KitButton::new(Some(crate::icons::SHADOWS), "Shadows").toggle(true).opens(false);
        let shadow_width = shadow_button.width(ui);
        let (unfolded, before, q, own) = (&unfolded_kinds, &shadows_before, &quality, &own_menu);
        lighting = lighting
            .heading("Shadows")
            .folded_only()
            .check(PopCheck::new(&mut kinds.sun, "Sun and moon shadows").tip(SUN_TIP))
            .folded_only()
            .check(PopCheck::new(&mut kinds.creatures, "Creature shadows").tip(CREATURES_TIP))
            .folded_only()
            .check(
                PopCheck::new(&mut kinds.lights, "Light shadows")
                    .sub()
                    .enabled(creatures_on)
                    .tip(if creatures_on { LIGHTS_TIP } else { LIGHTS_OFF_TIP }),
            )
            .folded_only()
            .custom(move |ui| quality_row(ui, q))
            .folded_only()
            .unfolded_only("Shadows", shadow_width, move |ui| {
                shadows_toggle(ui, unfolded, before, q, own);
            });
        lighting_rows += 1;
    }
    if shown("Fog") {
        lighting = lighting
            .separator()
            .folded_only()
            .check(PopCheck::new(&mut fog, "Fog").icon(crate::icons::FOG).unfolded_words(true));
        lighting_rows += 1;
    }
    if shown("Fade Geometry") {
        let value = &fade;
        let full = lead + fade_width;
        lighting = lighting.custom_unfolded(
            "Fade Geometry",
            move |ui| {
                ui.label("Fade geometry");
                fade_combo(ui, value, fade_width);
            },
            full,
            move |ui| fade_combo(ui, value, fade_width),
        );
        lighting_rows += 1;
    }
    let scene = [
        ("Animations", "Animations", crate::icons::ANIMATIONS, &mut animated, ANIMATIONS_TIP),
        ("Grass", "Grass", crate::icons::GRASS, &mut grass, GRASS_TIP),
        (
            "Tile Border",
            "Tile border",
            crate::icons::TILE_BORDER,
            &mut tile_border,
            TILE_BORDER_TIP,
        ),
        ("Shaders", "Shaders", crate::icons::SHADERS, &mut shaders, SHADERS_TIP),
    ];
    let mut first = true;
    for (name, words, icon, value, tip) in scene {
        if !shown(name) {
            continue;
        }
        if first {
            lighting = lighting.separator().folded_only();
            first = false;
        }
        lighting = lighting.check(PopCheck::new(value, words).icon(icon).tip(tip));
        lighting_rows += 1;
    }
    if lighting_rows > 0 {
        let tip = "Lighting, shadows, fog and how the scene is drawn";
        h = h.right(pop_item(form, Icon::Light, lighting, tip));
    } else {
        drop(lighting);
    }
    let mut overlays_pop = Popover::new("Overlays")
        .title(Some(Icon::Overlays), "Overlays")
        .switch(&mut overlays, SHOW_OVERLAYS);
    let mut overlay_rows = 0;
    if shown("Grid") {
        let row =
            PopCheck::new(&mut grid, "Grid").unfolded_icon(crate::icons::GRID).unfolded_words(true);
        overlays_pop = overlays_pop.check(row.tip("Display grid"));
        overlay_rows += 1;
    }
    if shown("Minimap") {
        let row = PopCheck::new(&mut minimap, "Minimap").unfolded_icon(crate::icons::MINIMAP);
        overlays_pop = overlays_pop.check(row.unfolded_words(true).tip(MINIMAP_TIP));
        overlay_rows += 1;
    }
    if shown("Walkmesh…") {
        let walk_button =
            KitButton::new(Some(crate::icons::WALKMESH), "Walkmesh").toggle(true).opens(false);
        let walk_width = walk_button.width(ui);
        let (unfolded, own) = (&unfolded_walks, &own_menu);
        overlays_pop = overlays_pop
            .separator()
            .folded_only()
            .check(PopCheck::new(&mut ground_walk, "Ground walkmesh").tip(GROUND_TIP))
            .folded_only()
            .check(PopCheck::new(&mut object_walk, "Object walkmeshes").tip(OBJECTS_TIP))
            .folded_only()
            .unfolded_only("Walkmesh", walk_width, move |ui| {
                walkmesh_toggle(ui, unfolded, own);
            });
        overlay_rows += 1;
    }
    if shown("Sound Ranges") {
        let row = PopCheck::new(&mut sound_ranges, "Sound ranges")
            .unfolded_icon(crate::icons::SOUND_RANGES);
        overlays_pop =
            overlays_pop.separator().folded_only().check(row.unfolded_words(true).tip(RANGES_TIP));
        overlay_rows += 1;
    }
    // (While the switch is off, the item stays whatever its rows hidden:
    // it is the one way to turn the overlays back on.)
    if overlay_rows > 0 || !overlays_were {
        let tip = "Grid, minimap, walkmeshes and sound ranges";
        h = h.right(pop_item(form, Icon::Overlays, overlays_pop, tip));
    } else {
        drop(overlays_pop);
    }
    let mut audio = Popover::new("Audio").title(Some(Icon::Speaker), "Audio");
    let mut audio_rows = 0;
    if shown("Sounds") {
        let row = PopCheck::new(&mut placed, "Sounds").name("Play Sounds");
        audio =
            audio.check(row.icon(crate::icons::SOUNDS).tip("Play placed sound objects in area"));
        audio_rows += 1;
    }
    if shown("Ambient") {
        let row = PopCheck::new(&mut ambient, "Ambient").icon(crate::icons::AMBIENT);
        audio = audio.check(row.tip("Play ambient sound in area"));
        audio_rows += 1;
    }
    if shown("Music") {
        let row = PopCheck::new(&mut music, "Music").icon(crate::icons::MUSIC);
        audio = audio.check(row.tip("Play ambient music in area"));
        audio_rows += 1;
    }
    if audio_rows > 0 {
        h = h.right(pop_item(form, Icon::Speaker, audio, "Placed sounds, ambient sound and music"));
    } else {
        drop(audio);
    }
    if shown("Area Properties") {
        h = h.right(Item::separator());
        let item =
            Item::button(crate::icons::AREA_PROPERTIES, "Area Properties").tip("Area properties");
        h = h.right(if compact { item.icon_only() } else { item.words_icon(false) });
    }
    if let Some((count, tip)) = &warning {
        let words = count.to_string();
        let width = metrics::ICON + LEAD_GAP + text_width(ui, &words) + 2.0 * LEAD_GAP;
        let (count, tip) = (*count, tip.as_str());
        let item = Item::custom(WARNINGS, width.ceil(), move |ui| warning_mark(ui, &words, tip))
            .more(200.0, move |ui| {
                let t = mg_uikit::tokens(ui.ctx());
                let what = match count {
                    1 => "1 warning".to_string(),
                    n => format!("{n} warnings"),
                };
                let r = ui.colored_label(t.warn, what).on_hover_text(tip);
                r.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Label, true, WARNINGS)
                });
            });
        h = h.right(item);
    }
    if shown("To Scratch") {
        let scratch_tip = crate::transfer::scratch_tip(
            app.settings.scratch_dir.as_deref(),
            "the area and the blueprints, scripts and conversations it uses",
        );
        let item = Item::button(crate::icons::TO_SCRATCH, "To Scratch").tip(scratch_tip);
        h = h.right(item.in_more());
    }
    // Several objects chosen: they can be kept as a group to place again
    // (the palette's Prefabs has them).
    if let Some(prefab) = &prefab {
        let item = Item::button(crate::icons::PREFABS, prefab.as_str())
            .tip("Keep the selected objects as a group, to place again in any module");
        h = h.right(item.in_more());
    }
    let out = h.show(ui);

    // Its right-click menu: which of its controls it shows.
    if !own_menu.get() {
        out.context_menu(|ui| {
            egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                super::toolbar_menu(ui, &mut app.settings.hidden_area_tools, false);
            });
        });
    }

    // What was asked, written back.
    if out.clicked("All") || all_or_none.get() == Some(true) {
        (show, show_start) = ([true; 9], true);
    }
    if out.clicked("None") || all_or_none.get() == Some(false) {
        (show, show_start) = ([false; 9], false);
    }
    view.show = show;
    view.show_start = show_start;
    if (view.show, view.show_start) != was_show {
        share_shown(app, view);
    }
    // Hidden objects cannot stay selected.
    view.selection.retain(|(k, _)| show[k.index()]);
    view.night = if compact { moon } else { night.get() };
    if view.lit != lit {
        view.lit = lit;
        app.settings.unlit_areas = !lit;
    }
    let kinds = if kinds != kinds_were { kinds } else { unfolded_kinds.get() };
    (view.shadows, view.creature_shadows, view.light_shadows) =
        (kinds.sun, kinds.creatures, kinds.lights);
    view.shadows_before = shadows_before.get();
    let s = &mut app.settings;
    if (kinds.sun, kinds.creatures, kinds.lights)
        != (!s.no_shadows, !s.no_creature_shadows, !s.no_light_shadows)
    {
        (s.no_shadows, s.no_creature_shadows, s.no_light_shadows) =
            (!kinds.sun, !kinds.creatures, !kinds.lights);
    }
    s.shadow_quality = quality.get();
    view.fog = fog;
    if view.fade != fade.get() {
        view.fade = fade.get();
        app.settings.fade_geometry = view.fade as u8;
    }
    let s = &mut app.settings;
    s.still_objects = !animated;
    s.no_grass = !grass;
    s.tile_border = tile_border;
    if shaders != shaders_were {
        app.set_game_shaders(shaders);
    }
    if view.hide_overlays == overlays {
        view.hide_overlays = !overlays;
        app.settings.hide_overlays = !overlays;
    }
    view.grid = grid;
    if view.minimap != minimap {
        view.minimap = minimap;
        app.settings.area_minimap = minimap;
    }
    (view.walkmesh, view.object_walkmesh) = if (ground_walk, object_walk) != walks_were {
        (ground_walk, object_walk)
    } else {
        unfolded_walks.get()
    };
    if view.sound_ranges != sound_ranges {
        view.sound_ranges = sound_ranges;
        app.settings.sound_ranges = sound_ranges;
    }
    let s = &mut app.settings;
    s.no_placed_sounds = !placed;
    s.ambient_sound = ambient;
    s.ambient_music = music;
    s.snap_grid = snap.get();
    s.snap_angle = turn.get();
    s.turn_together = together;
    s.place_onto_objects = onto;
    if painting {
        app.settings.paint = paint.get();
    }
    if tiling {
        tiles_asked(app, view, &out, ui, tile_properties);
    }
    if out.clicked("Area Properties") {
        app.actions.push(Action::OpenTab(crate::Tab::AreaProperties(view.area)));
    }
    if out.clicked("To Scratch") {
        app.actions.push(Action::ExportFiles {
            keys: vec![ResKey::new(view.area, ResType::ARE)],
            dependencies: true,
            scratch: true,
        });
    }
    if let Some(prefab) = &prefab
        && out.clicked(prefab)
    {
        app.prefab_save = copy_selection(app, view).map(|clip| (String::new(), clip));
    }
}

/// Select Tiles' commands' names in the header's left group (the
/// AreaTiles board), on the tiles selected.
pub(crate) const REFINE: &str = "Refine Tile";
pub(crate) const PREVIOUS: &str = "Previous Tile Appearance";
pub(crate) const NEXT: &str = "Next Tile Appearance";
pub(crate) const APPEARANCE: &str = "Appearance";
pub(crate) const ROTATE: &str = "Rotate Tile";
pub(crate) const TILE_CLIPBOARD: &str = "Tile Clipboard";
pub(crate) const CUT: &str = "Cut Tiles";
pub(crate) const COPY: &str = "Copy Tiles";
pub(crate) const PASTE: &str = "Paste Tiles";
pub(crate) const DELETE: &str = "Delete Tiles";
pub(crate) const TILE_PROPERTIES: &str = "Tile Properties";

/// Select Tiles' commands' tips, with their keys as bound (the AreaTiles
/// board's words).
struct TilesTips {
    refine: String,
    previous: String,
    next: String,
    rotate: String,
    cut: String,
    copy: String,
    paste: String,
    delete: String,
}

/// Rotate Tile's keys in its tip: `turn` (a quarter counterclockwise)
/// and `clockwise`, each where it is bound.
fn rotate_keys(turn: String, clockwise: String) -> String {
    match (turn, clockwise) {
        (k, c) if c.is_empty() => k,
        (k, c) if k.is_empty() => format!("{c} clockwise"),
        (k, c) => format!("{k}, or {c} clockwise"),
    }
}

/// Delete Tiles' keys in its tip: Delete, and the Eraser's `eraser` where
/// it is bound.
fn delete_keys(eraser: String) -> String {
    match eraser.is_empty() {
        true => "Delete".to_string(),
        false => format!("Delete, or Eraser ({eraser})"),
    }
}

/// The tips of Select Tiles' commands, each with its keys as Options ›
/// Keyboard has them.
fn tiles_tips(app: &Moonglow, ctx: &egui::Context) -> TilesTips {
    use crate::keys::Cmd;
    let key = |cmd: Cmd| app.keymap.label(cmd, ctx);
    let with = |words: &str, keys: String| match keys.is_empty() {
        true => words.to_string(),
        false => format!("{words} ({keys})"),
    };
    let rotate = rotate_keys(key(Cmd::RotateTile), key(Cmd::RotateTileClockwise));
    // (egui's own Copy, Cut and Paste, which Options › Keyboard doesn't
    // bind.)
    let ctrl = if ctx.os() == egui::os::OperatingSystem::Mac { "Cmd" } else { "Ctrl" };
    let edit = |key: char| format!("{ctrl}+{key}");
    TilesTips {
        refine: with(
            "Refine Tile: the first tile the next that fits there, the others of its kind made \
             that tile too",
            key(Cmd::RefineTile),
        ),
        previous: with(
            "Previous Tile Appearance: a step back through them",
            key(Cmd::PreviousTileAppearance),
        ),
        next: with(
            "Next Tile Appearance: another tile of the same kind here, turned as this one where \
             it fits",
            key(Cmd::NextTileAppearance),
        ),
        rotate: with(
            "Rotate Tile: a quarter turn counterclockwise, where the tile still fits its \
             neighbors",
            rotate,
        ),
        cut: with("Cut tiles", edit('X')),
        copy: with("Copy tiles", edit('C')),
        paste: with("Paste tiles: click to place the copies", edit('V')),
        delete: with(
            "Delete: remove crossers, and a group or a feature whole",
            delete_keys(key(Cmd::Eraser)),
        ),
    }
}

/// Select Tiles' commands in the header's left group, in place of the
/// Select tool's options (the AreaTiles board): Refine Tile, the joined ‹
/// Appearance ›, Rotate, a line, Cut, Copy and Paste joined, Delete and
/// the Tile Properties toggle. What acts on the tiles selected is grayed
/// with none (`has`); Paste where no tiles of the area's tileset are
/// copied (`paste`).
fn tiles_items<'a>(
    h: EditorHeader<'a>,
    has: bool,
    paste: bool,
    tips: &'a TilesTips,
    tile_properties: &'a mut bool,
) -> EditorHeader<'a> {
    let none = "Select tiles first";
    let tip = |t: &str| if has { t.to_string() } else { format!("{t}. {none}.") };
    let refine = Item::button(crate::icons::REFINE, REFINE).icon_only().enabled(has);
    let appearance = Item::joined(
        APPEARANCE,
        vec![
            Joined::button(crate::icons::PREVIOUS_APPEARANCE, PREVIOUS).tip(tip(&tips.previous)),
            Joined::words(APPEARANCE),
            Joined::button(crate::icons::NEXT_APPEARANCE, NEXT).tip(tip(&tips.next)),
        ],
    )
    .enabled(has);
    let rotate = Item::button(Icon::Rotate, ROTATE)
        .shown_words("Rotate")
        .enabled(has)
        .tip(tip(&tips.rotate));
    let clipboard = Item::joined(
        TILE_CLIPBOARD,
        vec![
            Joined::button(Icon::Cut, CUT).enabled(has).tip(tip(&tips.cut)),
            Joined::button(Icon::Copy, COPY).enabled(has).tip(tip(&tips.copy)),
            Joined::button(Icon::Paste, PASTE).enabled(paste).tip(match paste {
                true => tips.paste.clone(),
                false => format!("{}. Copy tiles of this tileset first.", tips.paste),
            }),
        ],
    );
    let delete = Item::button(Icon::Delete, DELETE).icon_only().enabled(has).tip(tip(&tips.delete));
    let props = Item::toggle(None, TILE_PROPERTIES, tile_properties)
        .tip("Tile Properties of the tiles selected, in the Right Dock");
    h.left(refine.tip(tip(&tips.refine)))
        .left(appearance)
        .left(rotate)
        .left(Item::separator())
        .left(clipboard)
        .left(delete)
        .left(props)
}

/// What Select Tiles' commands in the header asked, done on the tiles
/// selected in `view`; the Tile Properties toggle's state kept
/// (`Settings::hide_tile_properties`), the panel shown as it is turned on.
fn tiles_asked(
    app: &mut Moonglow,
    view: &mut AreaView,
    out: &mg_uikit::header::HeaderOutput<()>,
    ui: &Ui,
    tile_properties: bool,
) {
    use crate::terrain_mode::{OnSelection, act_on_selection};
    use crate::tile_select::TileStep;
    let shift = ui.input(|i| i.modifiers.shift);
    if out.clicked(REFINE) {
        act_on_selection(app, view, OnSelection::Refine { target: app.palette.tile_choice });
    }
    if out.clicked(PREVIOUS) {
        act_on_selection(app, view, OnSelection::Step(TileStep::Appearance { back: true }));
    }
    if out.clicked(NEXT) {
        act_on_selection(app, view, OnSelection::Step(TileStep::Appearance { back: false }));
    }
    if out.clicked(ROTATE) {
        act_on_selection(app, view, OnSelection::Step(TileStep::Rotate { back: shift }));
    }
    if (out.clicked(CUT) || out.clicked(COPY)) && !view.tile_selection.is_empty() {
        app.tile_clip = crate::tile_select::copy_tiles(app, view);
        if app.tile_clip.is_some() {
            crate::widgets::mark_clipboard(ui.ctx(), "tiles");
        }
        if out.clicked(CUT) {
            crate::tile_select::delete(app, view);
        }
    }
    if out.clicked(PASTE) && app.tile_clip.is_some() {
        view.tile_pasting = true;
    }
    if out.clicked(DELETE) && !view.tile_selection.is_empty() {
        crate::tile_select::delete(app, view);
    }
    if tile_properties == app.settings.hide_tile_properties {
        app.settings.hide_tile_properties = !tile_properties;
        if tile_properties {
            app.actions.push(Action::ShowPanel(crate::Panel::TileProperties));
        }
    }
}

const START_TIP: &str = "Show the start location's marker, a blue ring with a red arrow";
const SUN_TIP: &str =
    "The sun's by day and the moon's at night, cast by tiles and static placeables";
const CREATURES_TIP: &str =
    "Creatures, doors and dynamic placeables cast shadows too, as at the game's best detail";
const LIGHTS_TIP: &str =
    "Creatures cast shadows from the nearest light that casts them, without sun or moon";
/// (Grayed out without Creature shadows, its check kept: the lights'
/// shadows are the creatures', as in the game.)
const LIGHTS_OFF_TIP: &str = "Check Creature shadows first: creatures cast these";
const ANIMATIONS_TIP: &str =
    "Play placed objects' animations, in every area view. Off, they hold still.";
const GRASS_TIP: &str = "Draw the tileset's and the area's own grass, in every area view";
const TILE_BORDER_TIP: &str =
    "Draw the tileset's edge tiles past the area's edge, as the game does";
const SHADERS_TIP: &str =
    "Draw with the game's own shaders. Off, with Moonglow's own (every 3D view).";
const MINIMAP_TIP: &str = "Show the area's map in a corner: click or drag on it to look there";
const GROUND_TIP: &str = "The ground's walkmesh, walkable faces green (Aurora's Render AABB Nodes)";
const OBJECTS_TIP: &str =
    "Where placeables and doors keep creatures out: placeables orange, doors blue";
const RANGES_TIP: &str = "Show each placed sound's circles: full volume inside, silent outside";

/// A popover's item: its icon and chevron in the compact header, its
/// words in the full one, and its rows unfolded into the row where it has
/// room in the wide layout.
fn pop_item<'a>(form: PopForm, icon: Icon, pop: Popover<'a>, tip: &str) -> Item<'a> {
    let item = Item::popover(Some(icon), pop).tip(tip);
    match form {
        PopForm::Compact => item.icon_only(),
        PopForm::Words => item,
        PopForm::Unfolds => item.unfolds(),
    }
}

/// How the header's popovers stand: icons (compact), their words, or
/// unfolded into the row where it has room (the wide layout, a window
/// from 2560: section 5.3; the Main board's row at 1600 keeps them as
/// buttons whatever room it has).
#[derive(Clone, Copy)]
enum PopForm {
    Compact,
    Words,
    Unfolds,
}

/// The arrow after the Show toggles (the Main board's "Show all or
/// none"): a menu of All and None (those not hidden), whichever is
/// chosen set in `pick`.
fn all_or_none_arrow(ui: &mut Ui, all: bool, none: bool, pick: &Cell<Option<bool>>) {
    let t = mg_uikit::tokens(ui.ctx());
    let size = vec2(ARROW_WIDTH, metrics::CONTROL_HEIGHT);
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
    let enabled = ui.is_enabled();
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, ALL_OR_NONE));
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&r));
    let fill = if r.hovered() || open { t.raised_hover } else { t.raised };
    let corners = egui::CornerRadius { nw: 0, sw: 0, ne: metrics::RADIUS, se: metrics::RADIUS };
    ui.painter().rect_filled(rect, corners, fill);
    mg_uikit::controls::paint_chevron(ui.painter(), rect.center(), 1.0, t.text_2);
    let r = r.on_hover_text("Show all or none");
    egui::Popup::menu(&r).style(mg_uikit::menu_style).show(|ui| {
        all_or_none_rows(ui, all, none, pick);
    });
}

/// All and None as rows (the arrow's menu; in More).
fn all_or_none_rows(ui: &mut Ui, all: bool, none: bool, pick: &Cell<Option<bool>>) {
    if all && ui.button("All").on_hover_text("Show All").clicked() {
        pick.set(Some(true));
    }
    if none && ui.button("None").on_hover_text("Show None").clicked() {
        pick.set(Some(false));
    }
}

/// What could not be read for the view (Part 2, decision 8): the count
/// of problems and missing models, and a tip saying what they are.
fn warnings(view: &AreaView) -> Option<(usize, String)> {
    let m = view.model.as_ref()?;
    let missing = view.scene.as_ref().map(|s| s.missing.as_slice()).unwrap_or_default();
    let count = m.problems.len() + missing.len();
    if count == 0 {
        return None;
    }
    let mut tip = Vec::new();
    if !m.problems.is_empty() {
        tip.push(format!("What could not be read in the area:\n{}", m.problems.join("\n")));
    }
    if !missing.is_empty() {
        tip.push(format!("Models not found:\n{}", missing.join("\n")));
    }
    Some((count, tip.join("\n\n")))
}

/// The warning on the row: its icon and count in `warn`, `tip` saying
/// what was read.
fn warning_mark(ui: &mut Ui, words: &str, tip: &str) {
    let t = mg_uikit::tokens(ui.ctx());
    let width = ui.available_width();
    let (rect, r) =
        ui.allocate_exact_size(vec2(width, metrics::CONTROL_HEIGHT), egui::Sense::hover());
    let y = rect.center().y;
    let square = egui::Rect::from_min_size(
        egui::pos2(rect.left() + LEAD_GAP, y - metrics::ICON / 2.0),
        egui::Vec2::splat(metrics::ICON),
    );
    Icon::Warning.paint(ui.painter(), square, t.warn);
    let font = FontId::proportional(metrics::BODY_SIZE);
    let at = egui::pos2(square.right() + LEAD_GAP, y);
    ui.painter().text(at, egui::Align2::LEFT_CENTER, words, font, t.warn);
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, WARNINGS));
    r.on_hover_text(tip);
}

/// The Lighting popover's Quality row (the kit sheet's Lighting cell):
/// Fast | High at the row's right, as in Options › Area.
fn quality_row(ui: &mut Ui, quality: &Cell<crate::settings::ShadowQuality>) {
    use crate::settings::ShadowQuality::{Fast, High};
    ui.label("Quality");
    let choices = [
        (Fast, "Fast", "A shadow map: cheap in every area, its edges stepped up close"),
        (
            High,
            "High",
            "Stencil shadow volumes, as sharp as the game's. Slower in the largest areas.",
        ),
    ];
    let width: f32 = (choices.iter())
        .map(|(_, w, _)| text_width(ui, w) + 2.0 * metrics::CONTROL_PADDING_X)
        .sum::<f32>()
        + 1.0;
    ui.add_space((ui.available_width() - width).max(0.0));
    let mut v = quality.get();
    mg_uikit::segmented(ui, "Quality", &mut v, &choices);
    quality.set(v);
}

/// The unfolded row's Shadows toggle (WideArea): on while any kind of
/// shadow is drawn; a click turns them all off, or on again as they were
/// (all at first); held or right-clicked, the kinds to choose from and
/// their quality.
fn shadows_toggle(
    ui: &mut Ui,
    kinds: &Cell<Shadows>,
    before: &Cell<Option<[bool; 3]>>,
    quality: &Cell<crate::settings::ShadowQuality>,
    own_menu: &Cell<bool>,
) {
    let mut k = kinds.get();
    // (Light Shadows alone draw none: `View::light_shadows_cast`.)
    let lights = mg_area::View::light_shadows_cast(k.creatures, k.lights);
    let on = k.sun || k.creatures || lights;
    let button = KitButton::new(Some(crate::icons::SHADOWS), "Shadows")
        .toggle(on)
        .opens(false)
        .tip("Show the game's shadows. Hold or right-click to choose which, and their quality.");
    let r = ui.add(button);
    own_menu.set(own_menu.get() || r.secondary_clicked());
    let (clicked, menu) = held_menu(ui, &r);
    if clicked {
        let was = [k.sun, k.creatures, k.lights];
        let again = before.take().unwrap_or([true; 3]);
        if on {
            before.set(Some(was));
        }
        [k.sun, k.creatures, k.lights] = if on { [false; 3] } else { again };
    }
    menu.style(mg_uikit::menu_style).show(|ui| {
        ui.check_box(&mut k.sun, "Sun and moon shadows").on_hover_text(SUN_TIP);
        ui.check_box(&mut k.creatures, "Creature shadows").on_hover_text(CREATURES_TIP);
        // (Grayed out without Creature Shadows, its tick kept: the lights'
        // shadows are the creatures', as in the game.)
        let creatures = k.creatures;
        ui.add_enabled(creatures, mg_uikit::Checkbox::new(&mut k.lights, "Light shadows"))
            .on_hover_text(LIGHTS_TIP)
            .on_disabled_hover_text(LIGHTS_OFF_TIP);
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Quality");
            let mut q = quality.get();
            crate::options::shadow_quality_choice(ui, &mut q);
            quality.set(q);
        });
    });
    kinds.set(k);
}

/// The unfolded row's Walkmesh toggle (WideArea): a click shows or hides
/// the ground's (or hides whichever are shown); held, or right-clicked,
/// the two to choose from, the ground's and the objects'.
fn walkmesh_toggle(ui: &mut Ui, walks: &Cell<(bool, bool)>, own_menu: &Cell<bool>) {
    let (mut ground, mut objects) = walks.get();
    let on = ground || objects;
    let button = KitButton::new(Some(crate::icons::WALKMESH), "Walkmesh")
        .toggle(on)
        .opens(false)
        .tip("Show the ground's walkmesh. Hold or right-click to choose which walkmeshes.");
    let r = ui.add(button);
    own_menu.set(own_menu.get() || r.secondary_clicked());
    let (clicked, menu) = held_menu(ui, &r);
    if clicked {
        (ground, objects) = (!on, false);
    }
    menu.style(mg_uikit::menu_style).show(|ui| {
        ui.check_box(&mut ground, "Ground walkmesh").on_hover_text(GROUND_TIP);
        ui.check_box(&mut objects, "Object walkmeshes").on_hover_text(OBJECTS_TIP);
    });
    walks.set((ground, objects));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rotate Tile's tip says which key turns clockwise (not both), and
    /// Delete's names the Eraser only where a key is bound to it.
    #[test]
    fn tile_tips_name_each_key_for_what_it_does() {
        let s = |t: &str| t.to_string();
        assert_eq!(rotate_keys(s("R"), s("Shift+R")), "R, or Shift+R clockwise");
        assert_eq!(rotate_keys(s("R"), s("")), "R");
        assert_eq!(rotate_keys(s(""), s("Shift+R")), "Shift+R clockwise");
        assert_eq!(delete_keys(s("E")), "Delete, or Eraser (E)");
        assert_eq!(delete_keys(s("")), "Delete", "no empty parentheses when Eraser is unbound");
    }
}
