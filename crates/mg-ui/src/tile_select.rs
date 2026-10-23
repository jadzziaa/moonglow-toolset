//! Selecting tiles in the area viewer (Aurora's Select Terrain mode), and
//! the Tile Properties window.
//!
//! - In tile mode a click selects the tile under the pointer (Ctrl + click
//!   adds or removes it) and a drag selects a box of tiles.
//! - Delete takes the selected tiles' crossers away (and a group or a
//!   feature whole); Shift + right click steps the tile under the pointer
//!   through the tiles that fit (Aurora's order: each tile's turns, then
//!   the next tile's).
//! - A right click opens the tile menu: Next and Previous Tile Appearance
//!   and Rotate Tile (`TileStep`, as the tiles palette's tools), Delete
//!   and Tile Properties. A tile brush without a right click of its own
//!   opens it too, on the tile clicked; out of tile mode the area's menu
//!   starts with the tile's items (`ground_items`) where no object is.
//! - Tile Properties sets the selected tiles' main and source light colours
//!   and animation loops (ARE `Tile_*`), each control enabled only where a
//!   tile's model has that light or loop (`mg_area::TileFeatures`; with
//!   several tiles, where any has it), the others greyed out, as Aurora
//!   shows them. Each tile is written only what its own model has.
//!   Defaults puts back the lighting scheme's first colours and the loops.
//!   Closed with changes not applied (its X, Escape), it asks: Save, Don't
//!   Save or Cancel; its Cancel button discards them.
//! - Its Height sets the tiles' `Tile_Height` alone (a step with - and +,
//!   or typed), what stands on them moved with them; All tiles (ignore
//!   fit) offers every tile of the tileset, any one put in the tiles as it
//!   is, turned as chosen, nothing around it fitted to it (issue #29).
//! - A tile of a group or a feature changes only with its group (the
//!   steps, the variants, Delete takes the whole group), unless Options ›
//!   Area allows editing them (`Grid::free_groups`).

use egui::{Color32, Pos2, Rect};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Value;
use mg_resman::ResKey;
use mg_tiles::paint::step_fit;

use crate::area_view::AreaView;
use crate::terrain_mode::current_grid;
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
    /// Which controls any of the tiles' models has (Aurora enables those):
    /// main lights, source lights, loops.
    pub has_main: [bool; 2],
    pub has_source: [bool; 2],
    pub has_loops: [bool; 3],
    /// What each tile's own model has, by `tiles`' order: what is written
    /// into that tile (the rest of its fields are kept as they are).
    pub has_each: Vec<mg_area::TileFeatures>,
    /// The values the window opened with: closed with others not applied,
    /// it asks first.
    pub opened: ([u8; 2], [u8; 2], [bool; 3]),
    /// Closed with changes: Save, Don't Save or Cancel is asked.
    pub closing: bool,
    /// With one tile chosen, a group's or a feature's, which changes only
    /// with its group: its variants aren't offered.
    pub grouped: bool,
    /// The light whose colour is being chosen: 0, 1 main, 2, 3 source.
    pub picking: Option<usize>,
    /// With one tile chosen: its cell and the tiles that fit there, in
    /// the order Next Variant steps through them, each with its model.
    pub variants: Option<Variants>,
    /// The tile there now.
    pub variant: Option<mg_tiles::Placement>,
    /// A variant just chosen: the window takes the new tile's lights and
    /// loops once the area shows it.
    pub chosen: bool,
    /// The tiles' cells, by `tiles`' order.
    pub cells: Vec<(u32, u32)>,
    /// Height: the tiles' `Tile_Height`, in the tileset's height steps,
    /// set alone (nothing around them fitted to them); `None` while the
    /// tiles have different heights and none is chosen (shown as mixed).
    pub height: Option<i32>,
    /// The height the window opened with.
    pub opened_height: Option<i32>,
    /// The first tile's height, which a mixed Height steps from.
    pub first_height: i32,
    /// One height step, metres (the tileset's `Transition`).
    pub step: f32,
    /// A tile of a group or feature is among them without the rest of its
    /// group, which changes only whole (unless Options › Area allows
    /// editing them): their height can't be set.
    pub height_locked: bool,
    /// All tiles (ignore fit): every tile of the tileset offered as
    /// pictures, any of them put in the tiles chosen as it is, nothing
    /// around fitted to it (#29). Off as the window opens.
    pub all_tiles: bool,
    /// The quarter turns (counter-clockwise) a tile chosen among all of
    /// them is put in with.
    pub turn: u8,
    /// Every tile of the tileset, with its model's name.
    pub every: Vec<(u32, String)>,
}

/// The most height steps up or down Tile Properties' Height sets (the
/// engine stands a tile from -3 to 1000 steps where `Tile_Height` puts it:
/// `engine_terrain.rs`; Moonglow reads the field within ±1024).
pub const HEIGHT_RANGE: i32 = 1024;

impl TileProps {
    /// Whether a value the tiles have a control for differs from what the
    /// window opened with (not yet applied).
    pub fn changed(&self) -> bool {
        let (main, source, loops) = self.opened;
        (0..2).any(|k| self.has_main[k] && self.main[k] != main[k])
            || (0..2).any(|k| self.has_source[k] && self.source[k] != source[k])
            || (0..3).any(|k| self.has_loops[k] && self.loops[k] != loops[k])
            || (!self.height_locked && self.height != self.opened_height)
    }
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
    let (copy, cut, paste) = crate::widgets::copy_cut_paste(ui);
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
    let g = current_grid(app, view)?;
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
        && let Some(mut g) = current_grid(app, view)
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
            None => {
                let blocked = g.blocked();
                let tools = view.terrain.as_ref().expect("checked");
                view.notice =
                    Some(crate::terrain_mode::refusal(tools, &g, "Paste", &blocked, None));
                view.refused = Some((blocked, std::time::Instant::now()));
            }
        }
    }
    true
}

/// The tile menu: a right click on a tile in tile mode, or with a tile
/// brush that has no right click of its own (a terrain, the Eraser, the
/// tile tools). In tile mode it acts on the selected tiles (the tile
/// clicked, if it isn't among them); with a brush, on the tile clicked.
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
    // In tile mode, the selection (made the tile clicked, if it isn't
    // in it); with a brush, the tile clicked.
    if view.tile_mode && !view.tile_selection.contains(&t) {
        view.tile_selection = vec![t];
    }
    let cells = if view.tile_mode { view.tile_selection.clone() } else { vec![t] };
    // (A brush's right click no longer puts it down, as Aurora's does:
    // here, and Escape.)
    if let Some(b) = crate::terrain_mode::active(app, view)
        && ui
            .button(format!("Put Down {}", b.label))
            .on_hover_text("Stop painting with it (Escape)")
            .clicked()
    {
        app.palette.tile_brush = None;
        ui.close();
        return;
    }
    tile_items(app, view, ui, &cells);
    if view.tile_mode
        && ui
            .button("Delete")
            .on_hover_text("Delete: crossers go, and a group or a feature goes whole")
            .clicked()
    {
        delete(app, view);
        ui.close();
    }
    // Properties last, as in Aurora's menus.
    ui.separator();
    if ui.button("Tile Properties…").clicked() {
        properties_of(app, view, &cells);
        ui.close();
    }
}

/// Opens Tile Properties for `cells`: out of tile mode, the tile selection
/// (which the window is opened on) is left as it was, so that the view
/// doesn't show a tile selected that nothing selects.
fn properties_of(app: &mut Moonglow, view: &mut AreaView, cells: &[(u32, u32)]) {
    let kept = std::mem::replace(&mut view.tile_selection, cells.to_vec());
    open_properties(app, view);
    if !view.tile_mode {
        view.tile_selection = kept;
    }
}

/// The tile menu's steps, on the tiles in `cells`: as the tiles
/// palette's tools of those names.
pub(crate) fn tile_items(
    app: &mut Moonglow,
    view: &mut AreaView,
    ui: &mut egui::Ui,
    cells: &[(u32, u32)],
) {
    let items = [
        (
            "Next Tile Appearance",
            "Another tile of the same kind here (its terrain and crossers), turned as this one \
             where it fits so",
            TileStep::Appearance { back: false },
        ),
        (
            "Previous Tile Appearance",
            "A step back through them",
            TileStep::Appearance { back: true },
        ),
        (
            "Rotate Tile",
            "A quarter turn counter-clockwise, where the tile still fits its neighbors (a half \
             turn where only that does); its lights stay",
            TileStep::Rotate { back: false },
        ),
    ];
    for (label, tip, step) in items {
        if ui.button(label).on_hover_text(tip).clicked() {
            step_tiles(app, view, cells, step);
            ui.close();
        }
    }
}

/// The tile items of the area's own menu (a right click on the ground
/// with no object there, out of tile mode): the steps and Tile
/// Properties, on the tile clicked, as Aurora's menu has Tile Properties
/// for a tile.
pub(crate) fn ground_items(
    app: &mut Moonglow,
    view: &mut AreaView,
    ui: &mut egui::Ui,
    t: (u32, u32),
) {
    ui.label(egui::RichText::new(format!("Tile ({}, {})", t.0, t.1)).weak());
    tile_items(app, view, ui, &[t]);
    if ui.button("Tile Properties…").clicked() {
        properties_of(app, view, &[t]);
        ui.close();
    }
    ui.separator();
}

/// A step of a tile's look that keeps its terrain and crossers: the tiles
/// palette's Next and Previous Tile Appearance and Rotate Tile, and the
/// tile menu's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TileStep {
    /// Another tile of the same kind (with `back`, the one before).
    Appearance { back: bool },
    /// The same tile a quarter turned (with `back`, clockwise).
    Rotate { back: bool },
}

impl TileStep {
    /// The step's command's name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            TileStep::Appearance { back: false } => "Next tile appearance",
            TileStep::Appearance { back: true } => "Previous tile appearance",
            TileStep::Rotate { .. } => "Rotate tile",
        }
    }

    /// What the step makes of the tile in cell `(x, y)`; `None` where it
    /// can't (a group's tile, unless they are free to edit; no other tile
    /// fits there, or turned it no longer fits).
    pub(crate) fn of(
        self,
        tools: &crate::terrain_mode::Tools,
        g: &mg_tiles::paint::Grid,
        (x, y): (u32, u32),
    ) -> Option<mg_tiles::Placement> {
        let now = g.tile(x, y);
        if g.locked(&tools.index, x, y) {
            return None;
        }
        let cell = g.lattice.cell(x, y);
        match self {
            TileStep::Appearance { back } => {
                mg_tiles::paint::step_appearance(&tools.index, &cell, now, back)
            }
            TileStep::Rotate { back } => mg_tiles::paint::turn_fit(&tools.index, &cell, now, back),
        }
    }

    /// Why nothing changed, for the status line.
    fn refused(self, grouped: bool) -> String {
        match (self, grouped) {
            (_, true) => format!("{}: {}", self.label(), crate::terrain_mode::GROUP_LOCKED),
            (TileStep::Appearance { .. }, false) => "No other tile fits there".into(),
            (TileStep::Rotate { .. }, false) => {
                "This tile fits here only as it lies: turned, it would not match its neighbors"
                    .into()
            }
        }
    }
}

/// The cells of the groups of the tiles in `cells` that are locked (a
/// group's, not free to edit): what flashes red when they refuse a step.
fn locked_groups(
    tools: &crate::terrain_mode::Tools,
    g: &mg_tiles::paint::Grid,
    cells: &[(u32, u32)],
) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &(x, y) in cells.iter().filter(|&&(x, y)| g.locked(&tools.index, x, y)) {
        for c in g.group_cells(&tools.index, x, y).unwrap_or_else(|| vec![(x, y)]) {
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}

/// Steps the tiles in `cells` (each on its own: what fits there), as one
/// command. A turned tile keeps its lights and animation loops (it is the
/// same tile); another tile gets fresh lights, as a painted one does.
pub(crate) fn step_tiles(
    app: &mut Moonglow,
    view: &mut AreaView,
    cells: &[(u32, u32)],
    step: TileStep,
) {
    let Some(g) = current_grid(app, view) else { return };
    let Some(tools) = view.terrain.as_ref() else { return };
    let changes: Vec<_> = cells.iter().filter_map(|&c| Some((c, step.of(tools, &g, c)?))).collect();
    if changes.is_empty() {
        let grouped = cells.iter().all(|&(x, y)| g.locked(&tools.index, x, y));
        let shown = if grouped { locked_groups(tools, &g, cells) } else { cells.to_vec() };
        let why = step.refused(grouped);
        // (Out of Select Tiles and with no brush, the area's own menu: the
        // status line is the objects', so the log says why.)
        if !view.tile_mode && crate::terrain_mode::active(app, view).is_none() {
            app.log.info(why.clone());
        }
        view.notice = Some(why);
        view.refused = Some((shown, std::time::Instant::now()));
        return;
    }
    let kept: Vec<_> = match step {
        TileStep::Rotate { .. } => {
            let are =
                app.ws.as_mut().and_then(|ws| ws.doc(&ResKey::new(view.area, ResType::ARE)).ok());
            let list = are.and_then(|a| a.root.list("Tile_List")).unwrap_or(&[]);
            let w = g.lattice.width();
            changes
                .iter()
                .filter_map(|&((x, y), _)| Some(((x, y), list.get((y * w + x) as usize)?.clone())))
                .collect()
        }
        TileStep::Appearance { .. } => Vec::new(),
    };
    view.notice = None;
    crate::terrain_mode::tile_command_keeping(app, view, &g, &changes, step.label(), &kept);
}

/// The tile's next variant among those that fit, in Aurora's order; with
/// `back`, the one before. Not a group's tile (unless they are free to
/// edit).
fn next_variant(app: &mut Moonglow, view: &mut AreaView, (x, y): (u32, u32), back: bool) {
    let Some(g) = current_grid(app, view) else { return };
    let Some(tools) = view.terrain.as_ref() else { return };
    if g.locked(&tools.index, x, y) {
        let shown = locked_groups(tools, &g, &[(x, y)]);
        view.notice = Some(format!("Next tile: {}", crate::terrain_mode::GROUP_LOCKED));
        view.refused = Some((shown, std::time::Instant::now()));
        return;
    }
    let Some(next) = step_fit(&tools.index, &g.lattice.cell(x, y), g.tile(x, y), back) else {
        view.notice = Some("No other tile fits there".into());
        return;
    };
    view.notice = None;
    crate::terrain_mode::tile_command(app, view, &g, &[((x, y), next)], "Next tile");
}

/// Delete on the selected tiles (a group's tile: the whole group, unless
/// their tiles are free to edit).
fn delete(app: &mut Moonglow, view: &mut AreaView) {
    let Some(mut g) = current_grid(app, view) else { return };
    let Some(tools) = view.terrain.as_ref() else { return };
    let cells = view.tile_selection.clone();
    let before = g.clone();
    match g.delete(&tools.index, &cells) {
        Some(stroke) => {
            let changes = g.apply(&tools.index, stroke, &mut fastrand::Rng::new());
            view.notice = None;
            crate::terrain_mode::tile_command(app, view, &before, &changes, "Delete tiles");
        }
        None => {
            view.notice = Some("Delete: no tile fits there".into());
            view.refused = Some((g.blocked(), std::time::Instant::now()));
        }
    }
}

/// What tile `id` of the area's tileset has of lights and loops: its
/// model's (the module's copy first, then the game's), as Aurora reads
/// them; for a model not found or not readable, what its SET entry says.
fn tile_features(
    app: &Moonglow,
    tools: &crate::terrain_mode::Tools,
    id: i64,
) -> mg_area::TileFeatures {
    let Some(entry) = usize::try_from(id).ok().and_then(|id| tools.set.tiles.get(id)) else {
        return mg_area::TileFeatures::default();
    };
    let name = entry.model.to_ascii_lowercase();
    let model = ResKey::parse(&name, ResType::MDL).and_then(|k| {
        let data = (app.ws.as_ref().and_then(|w| w.module.get(&k).map(<[u8]>::to_vec)))
            .or_else(|| app.game.as_deref()?.resman.get(&k).ok().map(|d| d.into_owned()))?;
        mg_mdl::Model::read(&data).ok()
    });
    match model {
        Some(m) => mg_area::TileFeatures::of(&m, &name),
        None => mg_area::TileFeatures {
            main: entry.main_lights,
            source: entry.source_lights,
            loops: entry.anim_loops,
        },
    }
}

/// Opens Tile Properties for the selected tiles, with the first one's
/// values.
fn open_properties(app: &mut Moonglow, view: &AreaView) {
    let (Some(model), Some(tools)) = (view.model.as_ref(), view.terrain.as_ref()) else { return };
    let tiles: Vec<usize> =
        view.tile_selection.iter().map(|&(x, y)| (y * model.width + x) as usize).collect();
    let Some(first) = tiles.first().and_then(|&i| model.tiles.get(i)) else { return };
    // What each tile's model has (a model read once however many tiles
    // have it); the window enables what any of them has, as Aurora does.
    let mut read: std::collections::HashMap<i64, mg_area::TileFeatures> = Default::default();
    let has_each: Vec<mg_area::TileFeatures> = tiles
        .iter()
        .map(|&i| {
            let id = model.tiles.get(i).map_or(-1, |t| t.id);
            *read.entry(id).or_insert_with(|| tile_features(app, tools, id))
        })
        .collect();
    let has = mg_area::TileFeatures::union(has_each.iter().copied());
    let cells = view.tile_selection.clone();
    let grid = current_grid(app, view);
    // Height: the tiles' own, or mixed; not to set where a group's tile is
    // among them without the rest of its group.
    let heights: Vec<i32> =
        tiles.iter().filter_map(|&i| model.tiles.get(i)).map(|t| t.height).collect();
    let height = heights.first().copied().filter(|h| heights.iter().all(|k| k == h));
    let height_locked = grid.as_ref().is_some_and(|g| {
        cells.iter().any(|&(x, y)| {
            g.locked(&tools.index, x, y)
                && !g
                    .group_cells(&tools.index, x, y)
                    .is_some_and(|group| group.iter().all(|c| cells.contains(c)))
        })
    });
    // One tile: the others that fit there, to choose among by picture
    // (not for a group's tile, which changes only with its group).
    let one = match view.tile_selection.as_slice() {
        &[(x, y)] => grid.clone().map(|g| ((x, y), g)),
        _ => None,
    };
    // (A group's tile among them: nothing but its group changes it.)
    let grouped =
        grid.as_ref().is_some_and(|g| cells.iter().any(|&(x, y)| g.locked(&tools.index, x, y)));
    let every =
        (tools.set.tiles.iter().enumerate()).map(|(i, t)| (i as u32, t.model.clone())).collect();
    let variants = one.filter(|_| !grouped).map(|((x, y), g)| {
        let fits = mg_tiles::paint::fits_in_order(&tools.index, &g.lattice.cell(x, y));
        let named = fits
            .into_iter()
            .filter_map(|p| Some((p, tools.set.tiles.get(p.tile as usize)?.model.clone())))
            .collect();
        (((x, y), named), g.tile(x, y))
    });
    let (variants, variant) = variants.unzip();
    // (A loop the tiles don't have shows unchecked, as Aurora shows it.)
    let loops = [0, 1, 2].map(|k| first.anim_loops[k] && has.loops[k]);
    app.tile_props = Some(TileProps {
        area: view.area,
        variants,
        variant,
        chosen: false,
        tiles,
        main: first.main_lights,
        source: first.source_lights,
        loops,
        has_main: has.main,
        has_source: has.source,
        has_loops: has.loops,
        has_each,
        opened: (first.main_lights, first.source_lights, loops),
        closing: false,
        grouped,
        picking: None,
        cells,
        height,
        opened_height: height,
        first_height: first.height,
        step: model.height_step,
        height_locked,
        all_tiles: false,
        turn: 0,
        every,
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

/// A swatch's name, for the screen reader (and the tests).
fn named(r: &egui::Response, name: &str, enabled: bool) {
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name));
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
            // (Still showing every tile, turned as chosen.)
            if let Some(p) = app.tile_props.as_mut() {
                (p.all_tiles, p.turn) = (props.all_tiles, props.turn);
            }
            app.area_views.insert(props.area, view);
            self::ui(app, ui);
            return;
        }
        app.area_views.insert(props.area, view);
    }
    let mut picked = None;
    let colors = light_colors(app);
    let color = |i: usize| colors.get(i).copied().unwrap_or(Color32::BLACK);
    let mut cancel = false;
    let mut escape = false;
    let mut done = None;
    {
        {
            egui::Grid::new("tile_props").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
                for k in 0..2 {
                    ui.add_enabled(
                        props.has_main[k],
                        egui::Label::new(format!("Main Light {}", k + 1)),
                    );
                    let main =
                        swatch(ui, color(usize::from(props.main[k])), props.has_main[k], false);
                    named(&main, &format!("Main Light {} Color", k + 1), props.has_main[k]);
                    if main.on_hover_text("Click here to select a color").clicked() {
                        props.picking = Some(k);
                    }
                    // (One the tiles don't have shows unchecked, as Aurora's.)
                    let mut none = false;
                    let on = if props.has_loops[k] { &mut props.loops[k] } else { &mut none };
                    ui.add_enabled(
                        props.has_loops[k],
                        egui::Checkbox::new(on, format!("Animation Loop {}", k + 1)),
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
                    let source = swatch(
                        ui,
                        color(2 * usize::from(props.source[k])),
                        props.has_source[k],
                        false,
                    );
                    named(&source, &format!("Source Light {} Color", k + 1), props.has_source[k]);
                    if source.on_hover_text("Click here to select a color").clicked() {
                        props.picking = Some(2 + k);
                    }
                    if k == 0 {
                        let mut none = false;
                        let on = if props.has_loops[2] { &mut props.loops[2] } else { &mut none };
                        ui.add_enabled(
                            props.has_loops[2],
                            egui::Checkbox::new(on, "Animation Loop 3"),
                        )
                        .on_hover_text("Check to play this tile animation");
                    }
                    ui.end_row();
                }
            });
            height_row(ui, &mut props);
            // (No Replacement Texture: `Tile_ReplaceTex` is BioWare's
            // unused field, which no tile of the game's tilesets has a
            // texture for, and the client wasn't seen drawing it. One in a
            // file is kept, among the raw fields: docs/deferred.md.)
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
                // (Its Cancel discards what changed; Escape, as its X
                // does, asks first: `may_close`.)
                if ui.button("Cancel").on_hover_text("Discard changes").clicked() {
                    cancel = true;
                }
            });
            if crate::widgets::escape_here(ui) {
                escape = true;
            }
            if props.grouped {
                ui.separator();
                ui.weak(
                    "A tile of a group or a feature: it changes only with its group, so other \
                     tiles aren't offered here (Options › Area › Allow editing tiles of groups \
                     and features offers them).",
                );
            }
            // All tiles (ignore fit): an explicit switch, the fit-only
            // variants as ever with it off (#29).
            let free = !props.grouped && !props.cells.is_empty();
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                let all = ui
                    .add_enabled(free, egui::Checkbox::new(&mut props.all_tiles, ALL_TILES))
                    .on_hover_text(
                        "Every tile of the tileset, as pictures: click one to put it in the tiles \
                         chosen as it is, turned as chosen beside and at the Height above, \
                         whether it fits its neighbors or not. No neighbor changes. Off: only \
                         the tiles that fit are offered",
                    );
                if !free {
                    all.on_disabled_hover_text(
                        "A tile of a group or a feature: it changes only with its group \
                         (Options › Area › Allow editing tiles of groups and features)",
                    );
                }
                if props.all_tiles && free {
                    ui.separator();
                    crate::widgets::field_label(ui, "Turn");
                    for (k, name) in ["0°", "↺90°", "↺180°", "↺270°"].iter().enumerate() {
                        ui.selectable_value(&mut props.turn, k as u8, *name)
                            .on_hover_text("Quarter turns counter-clockwise");
                    }
                }
            });
            if props.all_tiles && free {
                ui.separator();
                let side = variant_side(app);
                ui.horizontal(|ui| {
                    ui.label(format!("All tiles ({} in the tileset)", props.every.len()))
                        .on_hover_text(
                            "Every tile of the tileset, in its order, turned as chosen. Click \
                             one to put it here as it is",
                        );
                    size_slider(app, ui, side);
                });
                if let Some(p) = all_tiles_pictures(app, ui, &props, side) {
                    put_any(app, &mut props, p);
                }
            } else if let Some((_, variants)) = props.variants.clone().filter(|(_, v)| v.len() > 1)
            {
                // The tile's variants: as many across as the tab is wide,
                // the rest of its height theirs.
                ui.separator();
                let side = variant_side(app);
                ui.horizontal(|ui| {
                    ui.label(format!("Variant ({} fit here)", variants.len())).on_hover_text(
                        "The tiles that fit here, each turn of each, in the order Shift + right \
                         click steps through them. Click one to put it here",
                    );
                    size_slider(app, ui, side);
                });
                // (A picture's column: its side, the grid's gap and the
                // spacing after it; the scroll bar's room off the width.)
                let stride = side + 6.0 + ui.spacing().item_spacing.x;
                let across = ((ui.available_width() - 16.0 + 6.0) / stride).floor().max(1.0);
                egui::ScrollArea::vertical()
                    .id_salt("tile-variants")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("tile-variants").spacing([6.0, 6.0]).show(ui, |ui| {
                            for (i, (p, model)) in variants.into_iter().enumerate() {
                                let current = props.variant == Some(p);
                                if variant_picture(app, ui, p, &model, current, side) {
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
        crate::widgets::dialog(ctx, "Select A Color", &mut keep)
            .collapsible(false)
            .resizable(false)
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
        if let Some(g) = current_grid(app, &view) {
            view.tile_selection = vec![(*x, *y)];
            crate::terrain_mode::tile_command(app, &mut view, &g, &[((*x, *y), p)], "Tile variant");
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
    if escape {
        // (Not while the question is asked: its own Cancel takes Escape.)
        if props.changed() {
            props.closing = true;
        } else {
            cancel = true;
        }
    }
    if !cancel {
        app.tile_props = Some(props);
    }
}

/// Tile Properties closed (its X, Escape) with changes not applied: the
/// question a side editor's tab asks, Save, Don't Save or Cancel. Save
/// applies them as OK does; Cancel keeps the window open.
pub(crate) fn closing_window(app: &mut Moonglow, ctx: &egui::Context) {
    if !app.tile_props.as_ref().is_some_and(|p| p.closing) {
        return;
    }
    enum Answer {
        Save,
        DontSave,
        Cancel,
    }
    let mut answer = None;
    crate::widgets::window("Unsaved Changes").id(egui::Id::new("tile-props-closing")).show(
        ctx,
        |ui| {
            ui.label("Save changes to Tile Properties?");
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    answer = Some(Answer::Save);
                }
                if ui.button("Don't Save").clicked() {
                    answer = Some(Answer::DontSave);
                }
                if crate::widgets::cancel(ui) {
                    answer = Some(Answer::Cancel);
                }
            });
        },
    );
    match answer {
        None => {}
        Some(Answer::Save) => {
            if let Some(props) = app.tile_props.take() {
                apply(app, &props);
            }
        }
        Some(Answer::DontSave) => app.tile_props = None,
        Some(Answer::Cancel) => {
            if let Some(p) = app.tile_props.as_mut() {
                p.closing = false;
            }
        }
    }
}

/// A variant's picture's side, in points: the usual, and the slider's
/// least and most (the most is the pictures' own size, in pixels).
const VARIANT: f32 = 84.0;
const VARIANT_MIN: f32 = 64.0;
const VARIANT_MAX: f32 = crate::model_view::THUMBNAIL as f32;

/// The variants' pictures' side, as their slider was last left.
pub(crate) fn variant_side(app: &Moonglow) -> f32 {
    app.settings.tile_variant_side.map_or(VARIANT, f32::from).clamp(VARIANT_MIN, VARIANT_MAX)
}

/// Tile Properties' All tiles switch's name.
pub const ALL_TILES: &str = "All tiles (ignore fit)";

/// The pictures' Size slider, beside their heading.
fn size_slider(app: &mut Moonglow, ui: &mut egui::Ui, side: f32) {
    // (Tiles much alike are told apart larger.)
    let mut size = side;
    let slider =
        egui::Slider::new(&mut size, VARIANT_MIN..=VARIANT_MAX).show_value(false).text("Size");
    if ui.add(slider).on_hover_text("How large the pictures are").changed() {
        app.settings.tile_variant_side = Some(size.round() as u16);
    }
}

/// Tile Properties' Height: the tiles' `Tile_Height`, a step down or up
/// with its buttons, or any number of steps typed or dragged; set alone,
/// nothing around them fitted to them (#29).
fn height_row(ui: &mut egui::Ui, props: &mut TileProps) {
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Height");
        let locked = props.height_locked;
        ui.add_enabled_ui(!locked, |ui| {
            let mut v = props.height.unwrap_or(props.first_height);
            let mut set = None;
            let down = ui.button("-").on_hover_text(
                "A height step down, these tiles alone: their neighbors stay as they are",
            );
            named(&down, "Lower Height", !locked);
            if down.clicked() {
                set = Some(v - 1);
            }
            let mut value = egui::DragValue::new(&mut v).range(-HEIGHT_RANGE..=HEIGHT_RANGE);
            if props.height.is_none() {
                // (The tiles have different heights: one typed sets them all.)
                value = value.custom_formatter(|_, _| "Mixed".into());
            }
            let r = ui.add(value).on_hover_text(
                "The tiles' height, in the tileset's height steps (Tile_Height): type or drag \
                 any number of steps, below 0 too. Nothing around them changes, so a tile \
                 raised alone stands above its neighbors, with gaps between; what stands on \
                 it rises with it",
            );
            if r.changed() {
                set = Some(v);
            }
            let up = ui.button("+").on_hover_text(
                "A height step up, these tiles alone: their neighbors stay as they are",
            );
            named(&up, "Raise Height", !locked);
            if up.clicked() {
                set = Some(v + 1);
            }
            if let Some(h) = set {
                props.height = Some(h.clamp(-HEIGHT_RANGE, HEIGHT_RANGE));
            }
            let at = match props.height {
                Some(h) => format!("{} m", h as f32 * props.step),
                None => "mixed".into(),
            };
            ui.weak(format!("steps of {} m: {at}", props.step));
        });
        if locked {
            ui.weak("(a group's tile, without the rest of its group)").on_hover_text(
                "A tile of a group or a feature changes only with it whole: select all of its \
                 tiles to raise or lower it (or Options › Area › Allow editing tiles of groups \
                 and features)",
            );
        }
    });
}

/// Every tile of the tileset as pictures, turned as Tile Properties'
/// Turn says, as many across as the tab is wide; only the rows in sight
/// are drawn (and their pictures made), however many the tileset has.
/// The tile clicked, if one is.
fn all_tiles_pictures(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    props: &TileProps,
    side: f32,
) -> Option<mg_tiles::Placement> {
    let stride = side + 6.0 + ui.spacing().item_spacing.x;
    let across = ((ui.available_width() - 16.0 + 6.0) / stride).floor().max(1.0) as usize;
    let rows = props.every.len().div_ceil(across);
    // (A picture's button, its padding, and its name under it.)
    let pad = ui.spacing().button_padding.y * 2.0;
    let name = ui.text_style_height(&egui::TextStyle::Small);
    let row = side + pad + ui.spacing().item_spacing.y + name;
    let mut picked = None;
    egui::ScrollArea::vertical().id_salt("tile-all").auto_shrink([false, false]).show_rows(
        ui,
        row,
        rows,
        |ui, range| {
            for r in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0 + ui.spacing().item_spacing.x;
                    for (id, model) in props.every.iter().skip(r * across).take(across) {
                        let p =
                            mg_tiles::Placement { tile: *id, orientation: props.turn, height: 0 };
                        let now = props
                            .variant
                            .is_some_and(|v| v.tile == *id && v.orientation == props.turn);
                        ui.vertical(|ui| {
                            ui.set_min_height(row);
                            if variant_picture(app, ui, p, model, now, side) {
                                picked = Some(p);
                            }
                        });
                    }
                });
            }
        },
    );
    picked
}

/// The edits that set the tiles of `heights` (`Tile_List` index and
/// height) of `area` alone, and move what stands on them with them
/// (`objects`, the area view's, in height steps of `step` metres).
fn height_edits(
    ws: &mut mg_edit::Workspace,
    area: ResRef,
    objects: &[mg_area::AreaObject],
    step: f32,
    heights: &[(usize, i32)],
) -> Vec<Edit> {
    let (are, git) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let Ok(are_root) = ws.doc(&are).map(|d| d.root.clone()) else { return Vec::new() };
    let git_root = ws.doc(&git).map(|d| d.root.clone()).unwrap_or_else(|_| mg_gff::Struct::new(0));
    let files = mg_area::edit::AreaFiles { are, are_root: &are_root, git, git_root: &git_root };
    mg_area::edit::tile_height_edits(&files, objects, step, heights)
}

/// A tile chosen among all of the tileset's put in the tiles chosen, each
/// as it is (turned as chosen, at the Height), nothing around fitted to
/// it, as one step to undo; what stands on a tile it raises or lowers
/// goes with it. Refused, as the brushes are, over a group's tile.
fn put_any(app: &mut Moonglow, props: &mut TileProps, p: mg_tiles::Placement) {
    let Some(mut view) = app.area_views.remove(&props.area) else { return };
    if let (Some(g), Some(tools), Some(model)) =
        (current_grid(app, &view), view.terrain.as_ref(), view.model.as_ref())
    {
        let w = g.lattice.width();
        let mut changes = Vec::new();
        let mut refused = Vec::new();
        for &(x, y) in &props.cells {
            let height = props.height.unwrap_or(g.tile(x, y).height);
            let q = mg_tiles::Placement { height, ..p };
            match g.set_exact(&tools.index, x, y, q) {
                Some(_) => changes.push(((x, y), q)),
                None => refused.extend(g.blocked()),
            }
        }
        if refused.is_empty() && !changes.is_empty() {
            let heights: Vec<(usize, i32)> =
                changes.iter().map(|&((x, y), q)| ((y * w + x) as usize, q.height)).collect();
            let git = ResKey::new(props.area, ResType::GIT);
            let (objects, step) = (model.objects.clone(), model.height_step);
            let moves: Vec<Edit> = app.ws.as_mut().map_or(Vec::new(), |ws| {
                height_edits(ws, props.area, &objects, step, &heights)
                    .into_iter()
                    .filter(|e| matches!(e, Edit::SetField { key, .. } if *key == git))
                    .collect()
            });
            view.notice = None;
            view.tile_selection = props.cells.clone();
            let label = "Tile (ignoring fit)";
            crate::terrain_mode::tile_command_with(app, &mut view, &g, &changes, label, &[], moves);
            props.variant = Some(changes[0].1);
            props.chosen = true;
            props.opened_height = props.height;
        } else {
            let tools = view.terrain.as_ref().expect("checked");
            let why = crate::terrain_mode::refusal(tools, &g, "Tile", &refused, None);
            app.log.info(why.clone());
            view.notice = Some(why);
            view.refused = Some((refused, std::time::Instant::now()));
        }
    }
    app.area_views.insert(props.area, view);
}

/// One variant in Tile Properties: its model's picture, turned as the
/// tile would lie (the quarter turns written under it too), framed if it
/// is the one there now, `side` points across; under the pointer, at its
/// largest. Whether it was clicked.
fn variant_picture(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    p: mg_tiles::Placement,
    model: &str,
    current: bool,
    side: f32,
) -> bool {
    let key = ResKey::parse(&model.to_ascii_lowercase(), ResType::MDL);
    let pictured = key.map(|k| crate::model_view::Pictured::Tile(k, p.orientation));
    let made = pictured.and_then(|k| app.thumbnails.get(k)).flatten();
    let turn = ["", " ↺90°", " ↺180°", " ↺270°"][usize::from(p.orientation % 4)];
    let name = format!("{model}{turn}");
    let r = ui
        .vertical(|ui| {
            ui.set_width(side);
            let size = egui::vec2(side, side);
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
    let title = format!("Tile {}: {name}", p.tile);
    let r = match made {
        // (Larger, where the picture is shown smaller than it was made.)
        Some(id) if side < VARIANT_MAX => r.on_hover_ui(|ui| {
            ui.label(&title);
            ui.add(egui::Image::new((id, egui::vec2(VARIANT_MAX, VARIANT_MAX))));
        }),
        _ => r.on_hover_text(title),
    };
    r.clicked()
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
    // Height, set alone (with what stands on the tiles).
    let raised =
        match (props.height.filter(|_| !props.height_locked), app.area_views.get(&props.area)) {
            (Some(h), Some(view)) if props.height != props.opened_height => {
                let objects = view.model.as_ref().map_or(Vec::new(), |m| m.objects.clone());
                let step = view.model.as_ref().map_or(0.0, |m| m.height_step);
                let heights: Vec<(usize, i32)> = props.tiles.iter().map(|&i| (i, h)).collect();
                app.ws
                    .as_mut()
                    .map_or(Vec::new(), |ws| height_edits(ws, props.area, &objects, step, &heights))
            }
            _ => Vec::new(),
        };
    let Some(ws) = app.ws.as_mut() else { return };
    let Ok(are) = ws.doc(&key) else { return };
    let list = are.root.list("Tile_List").unwrap_or(&[]);
    let mut edits = Vec::new();
    for (n, &i) in props.tiles.iter().enumerate() {
        let Some(current) = list.get(i) else { continue };
        // (Each tile gets what its own model has; its other fields stay.)
        let has = props.has_each.get(n).copied().unwrap_or_default();
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
        set("Tile_MainLight1", Value::Byte(props.main[0]), has.main[0]);
        set("Tile_MainLight2", Value::Byte(props.main[1]), has.main[1]);
        set("Tile_SrcLight1", Value::Byte(props.source[0]), has.source[0]);
        set("Tile_SrcLight2", Value::Byte(props.source[1]), has.source[1]);
        for k in 0..3 {
            let on = Value::Byte(u8::from(props.loops[k]));
            set(&format!("Tile_AnimLoop{}", k + 1), on, has.loops[k]);
        }
    }
    edits.extend(raised);
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
        if let Some(screen) = view.square_on_screen(x as f32, y as f32, t.position.z) {
            let fill = Color32::from_rgba_unmultiplied(230, 40, 40, (110.0 * strength) as u8);
            let line = Color32::from_rgba_unmultiplied(255, 60, 60, (255.0 * strength) as u8);
            // (The outline edge by edge: see `area_view::outline`.)
            painter.add(egui::Shape::convex_polygon(screen.clone(), fill, egui::Stroke::NONE));
            crate::area_view::outline(&painter, &screen, egui::Stroke::new(2.0, line));
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
        if let Some(screen) = view.square_on_screen(x as f32, y as f32, t.position.z) {
            crate::area_view::outline(&painter, &screen, egui::Stroke::new(2.0, blue));
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
            if let Some(screen) = view.square_on_screen(x, y, 0.0) {
                crate::area_view::outline(&painter, &screen, egui::Stroke::new(2.0, gold));
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
