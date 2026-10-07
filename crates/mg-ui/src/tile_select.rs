//! Selecting tiles in the area viewer (Aurora's Select Terrain mode), and
//! the Tile Properties window.
//!
//! - In tile mode a click selects the tile under the pointer (Ctrl + click
//!   adds or removes it) and a drag selects a box of tiles.
//! - Delete takes the selected tiles' crossers away (and a group's tile out
//!   of its group); Shift + right click steps the tile under the pointer
//!   through the tiles that fit; a right click opens the tile menu.
//! - Tile Properties sets the selected tiles' main and source light colours
//!   and animation loops (ARE `Tile_*`), each control only where the tile's
//!   model has that light or loop (its SET entry), as Aurora shows them.
//!   Defaults puts back the lighting scheme's first colours and the loops.

use egui::{Color32, Pos2, Rect};
use glam::Vec3;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Value;
use mg_resman::ResKey;
use mg_tiles::paint::step_fit;

use crate::area_view::AreaView;
use crate::{Action, Moonglow};

/// A cell and the tiles that fit there, each with its model's name.
pub type Variants = ((u32, u32), Vec<(mg_tiles::Placement, String)>);

/// The Tile Properties window's values.
#[derive(Debug, Clone, PartialEq)]
pub struct TileProps {
    pub area: ResRef,
    /// The tiles it sets, by index in `Tile_List`.
    pub tiles: Vec<usize>,
    pub main: [u8; 2],
    pub source: [u8; 2],
    pub loops: [bool; 3],
    /// Which controls the tiles' models have: main lights, source lights,
    /// loops.
    pub has_main: [bool; 2],
    pub has_source: [bool; 2],
    pub has_loops: [bool; 3],
    /// The light whose colour is being chosen: 0, 1 main, 2, 3 source.
    pub picking: Option<usize>,
    /// `Tile_ReplaceTex`: the `replacetexture.2da` row the model's
    /// `replace_tex` is drawn with (`None`: the field is left out).
    pub replace: Option<u8>,
    /// With one tile chosen: its cell and the tiles that fit there, in
    /// the order Next Variant steps through them, each with its model.
    pub variants: Option<Variants>,
    /// The tile there now.
    pub variant: Option<mg_tiles::Placement>,
    /// A variant just chosen: the window takes the new tile's lights and
    /// loops once the area shows it.
    pub chosen: bool,
}

/// Copied tiles: each at its offset from the block's south-west corner,
/// its height above the block's lowest, and its ARE struct (lights, loops).
#[derive(Debug, Clone, PartialEq)]
pub struct TileClip {
    pub tileset: ResRef,
    pub tiles: Vec<((i64, i64), mg_tiles::Placement, mg_gff::Struct)>,
}

/// The tile under a screen point.
pub(crate) fn tile_at(view: &AreaView, pos: Pos2) -> Option<(u32, u32)> {
    let model = view.model.as_ref()?;
    let p = view.ground_at(pos, 0.0)?;
    let (x, y) = (p.x / mg_area::TILE_SIZE, p.y / mg_area::TILE_SIZE);
    (x >= 0.0 && y >= 0.0 && (x as u32) < model.width && (y as u32) < model.height)
        .then_some((x as u32, y as u32))
}

/// Tile mode's input; `false` when the view is not in tile mode.
pub(crate) fn input(
    app: &mut Moonglow,
    ui: &egui::Ui,
    view: &mut AreaView,
    response: &egui::Response,
) -> bool {
    if !view.tile_mode {
        return false;
    }
    let (command, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
    if response.drag_started_by(egui::PointerButton::Primary) && view.tile_box.is_none() && !command
    {
        let from = ui.input(|i| i.pointer.press_origin()).unwrap_or_default();
        view.tile_box = Some((from, from));
    }
    if let Some((from, _)) = view.tile_box {
        // (Until the left button is let go, whatever the others do.)
        let held = view.held;
        if let Some(now) = held.pos() {
            view.tile_box = Some((from, now));
        }
        if held.cancelled() {
            view.tile_box = None;
        } else if !held.by(egui::PointerButton::Primary) {
            view.tile_box = None;
            let (a, b) = (tile_at(view, from), held.pos().and_then(|p| tile_at(view, p)));
            if let (Some(a), Some(b)) = (a, b) {
                view.tile_selection.clear();
                for y in a.1.min(b.1)..=a.1.max(b.1) {
                    for x in a.0.min(b.0)..=a.0.max(b.0) {
                        view.tile_selection.push((x, y));
                    }
                }
            }
        }
    }
    // A click on an object: back to the objects, that one selected (the
    // tiles under one are still taken by a box dragged over them).
    if response.clicked()
        && !command
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(i) = view.pick(pos)
        && let Some(o) = view.model.as_ref().map(|m| &m.objects[i])
    {
        let chosen = (o.kind, o.index);
        view.tile_mode = false;
        view.tile_selection.clear();
        view.selection = vec![chosen];
        return true;
    }
    if response.clicked()
        && let Some(t) = response.interact_pointer_pos().and_then(|p| tile_at(view, p))
    {
        if command {
            match view.tile_selection.iter().position(|s| *s == t) {
                Some(i) => {
                    view.tile_selection.remove(i);
                }
                None => view.tile_selection.push(t),
            }
        } else {
            view.tile_selection = vec![t];
        }
    }
    if response.secondary_clicked()
        && shift
        && let Some(t) = response.interact_pointer_pos().and_then(|p| tile_at(view, p))
    {
        next_variant(app, view, t, false);
    }
    let typing = ui.ctx().memory(|m| m.focused().is_some());
    if response.hovered()
        && !typing
        && ui.input(|i| i.key_pressed(egui::Key::Delete))
        && !view.tile_selection.is_empty()
    {
        delete(app, view);
    }
    // Copy, cut and paste (egui's events, or the keys where it sends none).
    let (copy, cut, paste) = ui.input(|i| {
        let key = |k: egui::Key| i.modifiers.command && i.key_pressed(k);
        let event = |f: fn(&egui::Event) -> bool| i.events.iter().any(f);
        (
            event(|e| matches!(e, egui::Event::Copy)) || key(egui::Key::C),
            event(|e| matches!(e, egui::Event::Cut)) || key(egui::Key::X),
            event(|e| matches!(e, egui::Event::Paste(_))) || key(egui::Key::V),
        )
    });
    if response.hovered() && !typing {
        if (copy || cut) && !view.tile_selection.is_empty() {
            app.tile_clip = copy_tiles(app, view);
            if app.tile_clip.is_some() {
                crate::widgets::mark_clipboard(ui.ctx(), "tiles");
            }
            if cut {
                delete(app, view);
            }
        }
        if paste && app.tile_clip.is_some() {
            view.tile_pasting = true;
        }
    }
    true
}

/// Copies the selected tiles (Aurora: within a tileset).
fn copy_tiles(app: &mut Moonglow, view: &AreaView) -> Option<TileClip> {
    let g = grid(app, view)?;
    let tools = view.terrain.as_ref()?;
    let are = app.ws.as_mut()?.doc(&ResKey::new(view.area, ResType::ARE)).ok()?;
    let list = are.root.list("Tile_List")?;
    let w = g.lattice.width();
    let x0 = view.tile_selection.iter().map(|c| c.0).min()?;
    let y0 = view.tile_selection.iter().map(|c| c.1).min()?;
    let low = view.tile_selection.iter().map(|&(x, y)| g.tile(x, y).height).min()?;
    let tiles = view
        .tile_selection
        .iter()
        .filter_map(|&(x, y)| {
            let p = g.tile(x, y);
            let s = list.get((y * w + x) as usize)?.clone();
            let offset = (i64::from(x) - i64::from(x0), i64::from(y) - i64::from(y0));
            Some((offset, mg_tiles::Placement { height: p.height - low, ..p }, s))
        })
        .collect();
    Some(TileClip { tileset: tools.tileset, tiles })
}

/// Pasting: the copied tiles follow the pointer (the block's south-west
/// tile under it); a click puts them in as a group goes in, a right click
/// or Escape stops. `false` when not pasting.
pub(crate) fn paste_input(
    app: &mut Moonglow,
    ui: &egui::Ui,
    view: &mut AreaView,
    response: &egui::Response,
) -> bool {
    if !view.tile_pasting {
        return false;
    }
    let same = app
        .tile_clip
        .as_ref()
        .zip(view.terrain.as_ref())
        .is_some_and(|(c, t)| c.tileset == t.tileset);
    if !same {
        view.tile_pasting = false;
        view.notice = Some("Tiles paste only into an area of their tileset".into());
        return true;
    }
    let cancel = response.secondary_clicked()
        || (response.hovered() && ui.input(|i| i.key_pressed(egui::Key::Escape)));
    if cancel {
        view.tile_pasting = false;
        return true;
    }
    if response.clicked()
        && let Some(at) = response.interact_pointer_pos().and_then(|p| tile_at(view, p))
        && let Some(mut g) = grid(app, view)
        && let (Some(clip), Some(tools)) = (app.tile_clip.clone(), view.terrain.as_ref())
    {
        let tiles: Vec<_> = clip.tiles.iter().map(|(o, p, _)| (*o, Some(*p))).collect();
        let before = g.clone();
        match g.place_tiles(&tools.index, &tiles, at.0, at.1) {
            Some(stroke) => {
                let changes = g.apply(&tools.index, stroke, &mut fastrand::Rng::new());
                let kept: Vec<_> = clip
                    .tiles
                    .iter()
                    .map(|((dx, dy), _, s)| {
                        (((at.0 as i64 + dx) as u32, (at.1 as i64 + dy) as u32), s.clone())
                    })
                    .collect();
                crate::terrain_mode::tile_command_keeping(
                    app,
                    view,
                    &before,
                    &changes,
                    "Paste tiles",
                    &kept,
                );
                view.tile_pasting = false;
            }
            None => view.notice = Some("Paste: the tiles do not fit there".into()),
        }
    }
    true
}

/// The tile menu (right click in tile mode).
pub(crate) fn context_menu(
    app: &mut Moonglow,
    view: &mut AreaView,
    ui: &mut egui::Ui,
    at: Option<Pos2>,
) {
    let Some(t) = at.and_then(|p| tile_at(view, p)) else {
        ui.label("No tile here");
        return;
    };
    if !view.tile_selection.contains(&t) {
        view.tile_selection = vec![t];
    }
    if ui.button("Tile Properties…").clicked() {
        open_properties(app, view);
        ui.close();
    }
    if ui.button("Next Variant").on_hover_text("Shift + right click").clicked() {
        next_variant(app, view, t, false);
        ui.close();
    }
    if ui.button("Previous Variant").on_hover_text("A step back through them").clicked() {
        next_variant(app, view, t, true);
        ui.close();
    }
    if ui.button("Delete").on_hover_text("Delete: crossers and group tiles go").clicked() {
        delete(app, view);
        ui.close();
    }
}

fn grid(app: &mut Moonglow, view: &AreaView) -> Option<mg_tiles::paint::Grid> {
    let tools = view.terrain.as_ref()?;
    let are = app.ws.as_mut()?.doc(&ResKey::new(view.area, ResType::ARE)).ok()?;
    mg_area::terrain::grid(&are.root, &tools.index)
}

/// The tile's next variant among those that fit, in Aurora's order; with
/// `back`, the one before.
fn next_variant(app: &mut Moonglow, view: &mut AreaView, (x, y): (u32, u32), back: bool) {
    let Some(g) = grid(app, view) else { return };
    let Some(tools) = view.terrain.as_ref() else { return };
    let Some(next) = step_fit(&tools.index, &g.lattice.cell(x, y), g.tile(x, y), back) else {
        view.notice = Some("No other tile fits there".into());
        return;
    };
    crate::terrain_mode::tile_command(app, view, &g, &[((x, y), next)], "Next tile");
}

/// Delete on the selected tiles.
fn delete(app: &mut Moonglow, view: &mut AreaView) {
    let Some(mut g) = grid(app, view) else { return };
    let Some(tools) = view.terrain.as_ref() else { return };
    let cells = view.tile_selection.clone();
    let before = g.clone();
    match g.delete(&tools.index, &cells) {
        Some(stroke) => {
            let changes = g.apply(&tools.index, stroke, &mut fastrand::Rng::new());
            crate::terrain_mode::tile_command(app, view, &before, &changes, "Delete tiles");
        }
        None => {
            view.notice = Some("Delete: no tile fits there".into());
            view.refused = Some((g.blocked(), std::time::Instant::now()));
        }
    }
}

/// Opens Tile Properties for the selected tiles, with the first one's
/// values.
fn open_properties(app: &mut Moonglow, view: &AreaView) {
    let (Some(model), Some(tools)) = (view.model.as_ref(), view.terrain.as_ref()) else { return };
    let tiles: Vec<usize> =
        view.tile_selection.iter().map(|&(x, y)| (y * model.width + x) as usize).collect();
    let Some(first) = tiles.first().and_then(|&i| model.tiles.get(i)) else { return };
    let mut has_main = [false; 2];
    let mut has_source = [false; 2];
    let mut has_loops = [false; 3];
    for t in tiles.iter().filter_map(|&i| model.tiles.get(i)) {
        if let Some(s) = usize::try_from(t.id).ok().and_then(|id| tools.set.tiles.get(id)) {
            for k in 0..2 {
                has_main[k] |= s.main_lights[k];
                has_source[k] |= s.source_lights[k];
            }
            for (has, on) in has_loops.iter_mut().zip(s.anim_loops) {
                *has |= on;
            }
        }
    }
    let replace = app.ws.as_mut().and_then(|ws| {
        let are = ws.doc(&ResKey::new(view.area, ResType::ARE)).ok()?;
        let t = are.root.list("Tile_List")?.get(*tiles.first()?)?;
        t.integer("Tile_ReplaceTex").and_then(|v| u8::try_from(v).ok())
    });
    // One tile: the others that fit there, to choose among by picture.
    let variants = match view.tile_selection.as_slice() {
        &[(x, y)] => grid(app, view).map(|g| {
            let fits = mg_tiles::paint::fits_in_order(&tools.index, &g.lattice.cell(x, y));
            let named = fits
                .into_iter()
                .filter_map(|p| Some((p, tools.set.tiles.get(p.tile as usize)?.model.clone())))
                .collect();
            (((x, y), named), g.tile(x, y))
        }),
        _ => None,
    };
    let (variants, variant) = variants.unzip();
    app.tile_props = Some(TileProps {
        area: view.area,
        variants,
        variant,
        chosen: false,
        replace,
        tiles,
        main: first.main_lights,
        source: first.source_lights,
        loops: first.anim_loops,
        has_main,
        has_source,
        has_loops,
        picking: None,
    });
}

/// lightcolor.2da's toolset colours (TOOLSETRED, GREEN, BLUE, 0 to 1).
fn light_colors(app: &Moonglow) -> Vec<Color32> {
    let Some(t) = app.game.as_deref().and_then(|g| g.table("lightcolor").ok()) else {
        return Vec::new();
    };
    (0..t.len())
        .map(|r| {
            let c = |col: &str| (t.get_float(r, col).unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8;
            Color32::from_rgb(c("TOOLSETRED"), c("TOOLSETGREEN"), c("TOOLSETBLUE"))
        })
        .collect()
}

/// A colour swatch button.
fn swatch(ui: &mut egui::Ui, color: Color32, enabled: bool, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(28.0, 22.0),
        if enabled { egui::Sense::click() } else { egui::Sense::hover() },
    );
    let fill = if enabled { color } else { ui.visuals().widgets.noninteractive.bg_fill };
    ui.painter().rect_filled(rect, 2.0, fill);
    let stroke = if selected {
        egui::Stroke::new(2.0, ui.visuals().selection.stroke.color)
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    ui.painter().rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
    response
}

/// Tile Properties, in its tab, and its colour picker.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    let ctx = &ui.ctx().clone();
    let Some(mut props) = app.tile_props.take() else { return };
    // A variant chosen: once the area shows it, the window is of that
    // tile (the lights and loops its model has).
    if props.chosen
        && let Some(view) = app.area_views.remove(&props.area)
    {
        let shown = view.model.as_ref().zip(props.tiles.first()).and_then(|(m, &i)| m.tiles.get(i));
        let now = props.variant.map(|p| i64::from(p.tile));
        if shown.is_some_and(|t| Some(t.id) == now) {
            open_properties(app, &view);
            app.area_views.insert(props.area, view);
            self::ui(app, ui);
            return;
        }
        app.area_views.insert(props.area, view);
    }
    let mut picked = None;
    let colors = light_colors(app);
    let color = |i: usize| colors.get(i).copied().unwrap_or(Color32::BLACK);
    let replacements: Vec<String> = app
        .game
        .as_ref()
        .and_then(|g| g.table("replacetexture").ok())
        .map(|t| (0..t.len()).map(|r| t.get(r, "TEXTURENAME").unwrap_or("").to_string()).collect())
        .unwrap_or_default();
    let mut cancel = false;
    let mut done = None;
    {
        {
            egui::Grid::new("tile_props").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
                for k in 0..2 {
                    ui.add_enabled(
                        props.has_main[k],
                        egui::Label::new(format!("Main Light {}", k + 1)),
                    );
                    if swatch(ui, color(usize::from(props.main[k])), props.has_main[k], false)
                        .on_hover_text("Click here to select a color")
                        .clicked()
                    {
                        props.picking = Some(k);
                    }
                    ui.add_enabled(
                        props.has_loops[k],
                        egui::Checkbox::new(
                            &mut props.loops[k],
                            format!("Animation Loop {}", k + 1),
                        ),
                    )
                    .on_hover_text("Check to play this tile animation");
                    ui.end_row();
                }
                for k in 0..2 {
                    ui.add_enabled(
                        props.has_source[k],
                        egui::Label::new(format!("Source Light {}", k + 1)),
                    );
                    // A source light's value v shows as lightcolor row 2v.
                    if swatch(
                        ui,
                        color(2 * usize::from(props.source[k])),
                        props.has_source[k],
                        false,
                    )
                    .on_hover_text("Click here to select a color")
                    .clicked()
                    {
                        props.picking = Some(2 + k);
                    }
                    if k == 0 {
                        ui.add_enabled(
                            props.has_loops[2],
                            egui::Checkbox::new(&mut props.loops[2], "Animation Loop 3"),
                        )
                        .on_hover_text("Check to play this tile animation");
                    }
                    ui.end_row();
                }
            });
            // The tile's replacement texture (EE keeps it; Aurora has no
            // field): for tile models with a texture named replace_tex.
            ui.horizontal(|ui| {
                ui.label("Replacement Texture").on_hover_text(
                    "What a texture named replace_tex in the tile's model is drawn with \
                     (replacetexture.2da)",
                );
                let name = |r: u8| {
                    let t = replacements.get(usize::from(r)).map_or("?", String::as_str);
                    format!("{r}: {t}")
                };
                let shown = props.replace.map_or_else(|| "None".to_string(), name);
                egui::ComboBox::from_id_salt("tile_replace").selected_text(shown).show_ui(
                    ui,
                    |ui| {
                        ui.selectable_value(&mut props.replace, None, "None");
                        for r in 0..replacements.len().min(256) {
                            ui.selectable_value(&mut props.replace, Some(r as u8), name(r as u8));
                        }
                    },
                );
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Defaults").clicked() {
                    done = Some(false);
                }
                if ui.button("OK").on_hover_text("Accept changes").clicked()
                    || crate::widgets::enter(ui)
                {
                    done = Some(true);
                }
                if crate::widgets::cancel_discard(ui) {
                    cancel = true;
                }
            });
            // The tile's variants: as many across as the tab is wide, the
            // rest of its height theirs.
            if let Some((_, variants)) = props.variants.clone().filter(|(_, v)| v.len() > 1) {
                ui.separator();
                ui.label(format!("Variant ({} fit here)", variants.len())).on_hover_text(
                    "The tiles that fit here, in the order Next Variant steps through them. \
                     Click one to put it here",
                );
                let across = ((ui.available_width() - 16.0) / (VARIANT + 6.0)).floor().max(1.0);
                egui::ScrollArea::vertical()
                    .id_salt("tile-variants")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("tile-variants").spacing([6.0, 6.0]).show(ui, |ui| {
                            for (i, (p, model)) in variants.into_iter().enumerate() {
                                let current = props.variant == Some(p);
                                if variant_picture(app, ui, p, &model, current) {
                                    picked = Some(p);
                                }
                                if i % across as usize == across as usize - 1 {
                                    ui.end_row();
                                }
                            }
                        });
                    });
            }
        }
    }
    // The colour picker: lightcolor's 32 colours (16 for source lights).
    if let Some(which) = props.picking {
        let source = which >= 2;
        let current = if source {
            2 * usize::from(props.source[which - 2])
        } else {
            usize::from(props.main[which])
        };
        let mut keep = true;
        egui::Window::new("Select A Color")
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .open(crate::widgets::open_unless_escape(ctx, "Select A Color", &mut keep))
            .show(ctx, |ui| {
                egui::Grid::new("light_colors").spacing([4.0, 4.0]).show(ui, |ui| {
                    for row in 0..colors.len().min(32) {
                        if source && row % 2 == 1 {
                            continue;
                        }
                        if swatch(ui, color(row), true, row == current).clicked() {
                            if source {
                                props.source[which - 2] = (row / 2) as u8;
                            } else {
                                props.main[which] = row as u8;
                            }
                            props.picking = None;
                        }
                        let per_row = if source { 4 } else { 8 };
                        let n = if source { row / 2 } else { row };
                        if n % per_row == per_row - 1 {
                            ui.end_row();
                        }
                    }
                });
            });
        if !keep {
            props.picking = None;
        }
    }
    if let (Some(p), Some(((x, y), _))) = (picked, &props.variants)
        && props.variant != Some(p)
        && let Some(mut view) = app.area_views.remove(&props.area)
    {
        if let Some(g) = grid(app, &view) {
            view.tile_selection = vec![(*x, *y)];
            crate::terrain_mode::tile_command(app, &view, &g, &[((*x, *y), p)], "Tile variant");
            props.variant = Some(p);
            props.chosen = true;
        }
        app.area_views.insert(props.area, view);
    }
    match done {
        Some(true) => {
            apply(app, &props);
            return;
        }
        Some(false) => defaults(app, &mut props),
        None => {}
    }
    if !cancel {
        app.tile_props = Some(props);
    }
}

/// A variant's picture's side, in points.
const VARIANT: f32 = 84.0;

/// One variant in Tile Properties: its model's picture, turned as the
/// tile would lie (the quarter turns written under it too), framed if it
/// is the one there now. Whether it was clicked.
fn variant_picture(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    p: mg_tiles::Placement,
    model: &str,
    current: bool,
) -> bool {
    let key = ResKey::parse(&model.to_ascii_lowercase(), ResType::MDL);
    let pictured = key.map(|k| crate::model_view::Pictured::Tile(k, p.orientation));
    let made = pictured.and_then(|k| crate::model_view::thumbnail_of(app, k));
    let turn = ["", " ↺90°", " ↺180°", " ↺270°"][usize::from(p.orientation % 4)];
    let name = format!("{model}{turn}");
    let r = ui
        .vertical(|ui| {
            ui.set_width(VARIANT);
            let size = egui::vec2(VARIANT, VARIANT);
            let r = match made {
                Some(id) => ui.add(
                    egui::Button::image(egui::Image::new((id, size)))
                        .selected(current)
                        .frame(current),
                ),
                None => ui.add_sized(size, egui::Button::new("…").selected(current)),
            };
            if current {
                let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
                ui.painter().rect_stroke(r.rect, 2.0, stroke, egui::StrokeKind::Outside);
            }
            ui.add(egui::Label::new(egui::RichText::new(&name).small()).truncate());
            r
        })
        .inner;
    r.on_hover_text(format!("Tile {}: {name}", p.tile)).clicked()
}

/// Defaults: the lighting scheme's first colours and every loop on (where
/// the tile has them).
fn defaults(app: &mut Moonglow, props: &mut TileProps) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
    let Ok(are) = ws.doc(&ResKey::new(props.area, ResType::ARE)) else { return };
    let row = are.root.integer("LightingScheme").unwrap_or(0).max(0) as usize;
    if let Ok(t) = game.table("environment") {
        let first = |col: &str| t.get_int(row, col).unwrap_or(0).clamp(0, 255) as u8;
        props.main = [first("MAIN1_COLOR1"), first("MAIN2_COLOR1")];
        let source = first("SECONDARY_COLOR1");
        props.source = [source, source];
    }
    props.loops = props.has_loops;
}

/// Writes the window's values into the selected tiles, as one command.
fn apply(app: &mut Moonglow, props: &TileProps) {
    let key = ResKey::new(props.area, ResType::ARE);
    let Some(ws) = app.ws.as_mut() else { return };
    let Ok(are) = ws.doc(&key) else { return };
    let list = are.root.list("Tile_List").unwrap_or(&[]);
    let mut edits = Vec::new();
    for &i in &props.tiles {
        let Some(current) = list.get(i) else { continue };
        let path = GffPath::root().item("Tile_List", i);
        let mut set = |label: &str, value: Value, on: bool| {
            if on && current.get(label) != Some(&value) {
                edits.push(Edit::SetField {
                    key,
                    path: path.clone(),
                    label: label.into(),
                    value: Some(value),
                });
            }
        };
        set("Tile_MainLight1", Value::Byte(props.main[0]), props.has_main[0]);
        set("Tile_MainLight2", Value::Byte(props.main[1]), props.has_main[1]);
        set("Tile_SrcLight1", Value::Byte(props.source[0]), props.has_source[0]);
        set("Tile_SrcLight2", Value::Byte(props.source[1]), props.has_source[1]);
        for k in 0..3 {
            set(
                &format!("Tile_AnimLoop{}", k + 1),
                Value::Byte(u8::from(props.loops[k])),
                props.has_loops[k],
            );
        }
        let replace = props.replace.map(Value::Byte);
        if current.get("Tile_ReplaceTex") != replace.as_ref() {
            edits.push(Edit::SetField {
                key,
                path: path.clone(),
                label: "Tile_ReplaceTex".into(),
                value: replace,
            });
        }
    }
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Tile Properties", edits)));
    }
}

/// How long the tiles that refused a stroke stay red, in seconds.
const REFUSED_SHOWN: f32 = 1.2;

/// The tiles that refused the terrain stroke asked for last, in red for a
/// moment, fading: what is in the way of a raise, a lowering or a
/// painting, as Aurora flashes it.
pub(crate) fn refused_overlay(ui: &egui::Ui, view: &mut AreaView) {
    let Some((cells, at)) = &view.refused else { return };
    let age = at.elapsed().as_secs_f32();
    let Some(model) = view.model.as_ref().filter(|_| age < REFUSED_SHOWN) else {
        view.refused = None;
        return;
    };
    let strength = 1.0 - age / REFUSED_SHOWN;
    let painter = ui.painter_at(view.rect);
    for &(x, y) in cells {
        let Some(t) = model.tiles.get((y * model.width + x) as usize) else { continue };
        let z = t.position.z;
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(dx, dy)| {
            Vec3::new((x as f32 + dx) * mg_area::TILE_SIZE, (y as f32 + dy) * mg_area::TILE_SIZE, z)
        });
        let corners = view.on_ground(&corners);
        let screen: Vec<Pos2> = corners.iter().filter_map(|p| view.screen_pos(*p)).collect();
        if screen.len() == 4 {
            let fill = Color32::from_rgba_unmultiplied(230, 40, 40, (110.0 * strength) as u8);
            let line = Color32::from_rgba_unmultiplied(255, 60, 60, (255.0 * strength) as u8);
            painter.add(egui::Shape::convex_polygon(screen, fill, egui::Stroke::new(2.0, line)));
        }
    }
    ui.ctx().request_repaint();
}

/// The selected tiles and the box being dragged, over the view.
pub(crate) fn overlay(ui: &egui::Ui, view: &AreaView, clip: Option<&TileClip>) {
    if !view.tile_mode {
        return;
    }
    let Some(model) = view.model.as_ref() else { return };
    let painter = ui.painter_at(view.rect);
    let blue = Color32::from_rgb(70, 130, 255);
    // On the ground the pointer picks (some tilesets build theirs above the
    // tiles' heights).
    for &(x, y) in &view.tile_selection {
        let Some(t) = model.tiles.get((y * model.width + x) as usize) else { continue };
        let z = t.position.z;
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(dx, dy)| {
            Vec3::new((x as f32 + dx) * mg_area::TILE_SIZE, (y as f32 + dy) * mg_area::TILE_SIZE, z)
        });
        let corners = view.on_ground(&corners);
        let screen: Vec<Pos2> = corners.iter().filter_map(|p| view.screen_pos(*p)).collect();
        if screen.len() == 4 {
            painter.add(egui::Shape::closed_line(screen, egui::Stroke::new(2.0, blue)));
        }
    }
    // The pasted block, under the pointer.
    if view.tile_pasting
        && let (Some(clip), Some(at)) =
            (clip, ui.input(|i| i.pointer.hover_pos()).and_then(|p| tile_at(view, p)))
    {
        let gold = Color32::from_rgb(240, 200, 60);
        for ((dx, dy), _, _) in &clip.tiles {
            let (x, y) = (at.0 as f32 + *dx as f32, at.1 as f32 + *dy as f32);
            let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(ex, ey)| {
                Vec3::new((x + ex) * mg_area::TILE_SIZE, (y + ey) * mg_area::TILE_SIZE, 0.0)
            });
            let corners = view.on_ground(&corners);
            let screen: Vec<Pos2> = corners.iter().filter_map(|p| view.screen_pos(*p)).collect();
            if screen.len() == 4 {
                painter.add(egui::Shape::closed_line(screen, egui::Stroke::new(2.0, gold)));
            }
        }
    }
    if let Some((a, b)) = view.tile_box {
        painter.rect_stroke(
            Rect::from_two_pos(a, b),
            0.0,
            egui::Stroke::new(1.0, blue),
            egui::StrokeKind::Inside,
        );
    }
}
