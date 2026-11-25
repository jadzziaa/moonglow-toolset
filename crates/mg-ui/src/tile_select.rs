//! Selecting tiles in the area viewer (Aurora's Select Terrain mode), and
//! the Tile Properties panel.
//!
//! - In tile mode a click selects the tile under the pointer (Shift + click
//!   adds it, Ctrl + click adds or removes it; past the area's edge, none)
//!   and a drag selects a box of tiles (those of the area it covers; with
//!   Shift, added to those selected).
//! - Delete takes the selected tiles' crossers away (and a group or a
//!   feature whole); Shift + right click steps the tile under the pointer
//!   through the tiles that fit (Aurora's order: each tile's turns, then
//!   the next tile's).
//! - A right click opens the tile menu: Next and Previous Tile Appearance
//!   and Rotate Tile (`TileStep`, as the tiles palette's tools, which
//!   with tiles selected step them too), Delete and Tile Properties. A tile brush without a right click of its own
//!   opens it too, on the tile clicked; out of tile mode the area's menu
//!   starts with the tile's items (`ground_items`) where no object is.
//! - Tile Properties (the Right Dock's lower section while Select Tiles
//!   is on: M4's U5, the AreaTiles board; [`panel`]) shows the selected
//!   tiles, the first one's values, and follows the selection. It sets
//!   their main and source light colors and animation loops (ARE
//!   `Tile_*`), each control enabled only where a tile's model has that
//!   light or loop (`mg_area::TileFeatures`; with several tiles, where
//!   any has it), the others grayed out, as Aurora shows them. Each tile
//!   is written only what its own model has. Defaults puts back the
//!   lighting scheme's first colors and the loops.
//! - Its Replacement texture (EE's `Tile_ReplaceTex`, which Aurora's
//!   window hasn't got) is the `replacetexture.2da` row a mesh of the
//!   tile's model drawn with `replace_tex` is drawn with, enabled where a
//!   tile's model has one: a row chosen is written as its number plus one
//!   (`mg_area::replacement_value`), as the client reads it; a value it
//!   crashes on (0, or past the table) is said so.
//! - Its Height sets the tiles' `Tile_Height` alone (a step with - and +,
//!   or typed), what stands on them moved with them; All tiles (ignore
//!   fit) offers every tile of the tileset, any one put in the tiles as it
//!   is, turned as chosen, nothing around it fitted to it (issue #29); its
//!   Turn turns the tiles chosen at once, each as it is.
//! - There is no OK or Cancel: each change is one step to undo on every
//!   tile chosen, written at once (a Height dragged is one step for the
//!   drag), and only the control changed is written: a control chosen,
//!   even to the value the first tile shows, goes to every tile that has
//!   it; one not touched writes nothing. A tile put in from the pictures,
//!   or turned, is a step of its own. Ctrl+Z takes a change back.
//! - A tile of a group or a feature changes only with its group (the
//!   steps, the variants, Delete takes the whole group), unless Options ›
//!   Area allows editing them (`Grid::free_groups`).

use egui::{Color32, Pos2, Rect};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Value;
use mg_resman::ResKey;
use mg_tiles::paint::step_fit;

use crate::Moonglow;
use crate::area_view::AreaView;
use crate::terrain_mode::current_grid;

/// The cells chosen and the tiles that fit every one of them, each with
/// its model's name (as they fit the first cell: a tile fits another
/// cell at that cell's height).
pub type Variants = (Vec<(u32, u32)>, Vec<(mg_tiles::Placement, String)>);

/// The Tile Properties panel's tiles and values: read from the area as
/// the selection or the area changes ([`panel`]), and changed by its
/// controls; what differs from what was read is written at once.
#[derive(Debug, Clone, PartialEq)]
pub struct TileProps {
    pub area: ResRef,
    /// The tiles it sets, by index in `Tile_List`.
    pub tiles: Vec<usize>,
    /// How many times the ARE's lists had been rewritten as it read them
    /// ([`mg_edit::Workspace::rewrites`]): once they are again, its
    /// indices may be other tiles', and it reads them again.
    pub rewrites: u64,
    /// The workspace's revision as it read them: at another, it reads them
    /// again (a change of its own, Ctrl+Z, a brush, a paste, a plugin).
    pub revision: u64,
    pub main: [u8; 2],
    pub source: [u8; 2],
    pub loops: [bool; 3],
    /// Which controls any of the tiles' models has (Aurora enables those):
    /// main lights, source lights, loops.
    pub has_main: [bool; 2],
    pub has_source: [bool; 2],
    pub has_loops: [bool; 3],
    /// Whether any of the tiles' models has a mesh drawn with
    /// `replace_tex`: Replacement texture is enabled.
    pub has_replace: bool,
    /// Replacement texture: the first tile's `Tile_ReplaceTex` (1, row 0,
    /// for a tile without the field, as the client draws it).
    pub replace: u8,
    /// The replacement texture read.
    pub opened_replace: u8,
    /// What each tile's own model has, by `tiles`' order: what is written
    /// into that tile (the rest of its fields are kept as they are).
    pub has_each: Vec<mg_area::TileFeatures>,
    /// The values read (the first tile's): a control whose value differs
    /// from it is changed, and written.
    pub opened: ([u8; 2], [u8; 2], [bool; 3]),
    /// The controls chosen since they were read, by [`FIELDS`]' order,
    /// even to the value they had: each is written into every tile that
    /// has it, the others keep each tile's own.
    pub touched: [bool; 8],
    /// With one tile chosen, a larger group's or a feature's, which
    /// changes only with its group: its variants aren't offered (a group
    /// of one tile, a staircase, is not one).
    pub grouped: bool,
    /// The light whose color is being chosen: 0, 1 main, 2, 3 source.
    pub picking: Option<usize>,
    /// The cells chosen and the tiles that fit every one of them, in the
    /// order Next Variant steps through them, each with its model.
    pub variants: Option<Variants>,
    /// The tile there now (the first cell's, where every cell has that
    /// tile so turned; none where they differ).
    pub variant: Option<mg_tiles::Placement>,
    /// The tiles' cells, by `tiles`' order (the selection's).
    pub cells: Vec<(u32, u32)>,
    /// Height: the tiles' `Tile_Height`, in the tileset's height steps,
    /// set alone (nothing around them fitted to them); `None` while the
    /// tiles have different heights and none is chosen (shown as mixed).
    pub height: Option<i32>,
    /// The height read.
    pub opened_height: Option<i32>,
    /// The first tile's height, which a mixed Height steps from.
    pub first_height: i32,
    /// The first tile's tile in the tileset (`Tile_ID`), whose model's
    /// textures are listed.
    pub first_id: i64,
    /// One height step, meters (the tileset's `Transition`).
    pub step: f32,
    /// A tile of a group or feature is among them without the rest of its
    /// group, which changes only whole (unless Options › Area allows
    /// editing them): their height can't be set.
    pub height_locked: bool,
    /// All tiles (ignore fit): every tile of the tileset offered as
    /// pictures, any of them put in the tiles chosen as it is, nothing
    /// around fitted to it (#29). Off as the panel first shows tiles.
    pub all_tiles: bool,
    /// The quarter turns (counter-clockwise) a tile chosen among all of
    /// them is put in with.
    pub turn: u8,
    /// Every tile of the tileset, with its model's name.
    pub every: Vec<(u32, String)>,
    /// What All tiles' search finds: tiles by their model's name or their
    /// number in the tileset.
    pub find: String,
    /// The workspace's revision just after a Height being dragged wrote
    /// its step: while the drag goes on, that step is taken back and
    /// written again, one step for the drag.
    pub live: Option<u64>,
}

/// The most height steps up or down Tile Properties' Height sets (the
/// engine stands a tile from -3 to 1000 steps where `Tile_Height` puts it:
/// `engine_terrain.rs`; Moonglow reads the field within ±1024).
pub const HEIGHT_RANGE: i32 = 1024;

/// The tile fields of Tile Properties' lights, loops and replacement
/// texture, in the order of [`TileProps::touched`]: main lights 1 and 2,
/// source lights 1 and 2, animation loops 1 to 3, the replacement texture.
pub const FIELDS: [&str; 8] = [
    "Tile_MainLight1",
    "Tile_MainLight2",
    "Tile_SrcLight1",
    "Tile_SrcLight2",
    "Tile_AnimLoop1",
    "Tile_AnimLoop2",
    "Tile_AnimLoop3",
    "Tile_ReplaceTex",
];

/// [`FIELDS`]' place of the replacement texture.
const REPLACE: usize = 7;

/// What Tile Properties writes into its tiles: the values of the controls
/// set (by [`FIELDS`]' order; the others are left as each tile has them),
/// and the height, if one is set.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub values: [u8; 8],
    pub set: [bool; 8],
    pub height: Option<i32>,
}

impl Draft {
    /// The value of control `k` and whether a tile with features `has`
    /// takes it.
    fn field(&self, k: usize, has: &mg_area::TileFeatures) -> Option<Value> {
        let had = match k {
            0 | 1 => has.main[k],
            2 | 3 => has.source[k - 2],
            REPLACE => has.replace,
            _ => has.loops[k - 4],
        };
        (self.set[k] && had).then_some(Value::Byte(self.values[k]))
    }
}

impl TileProps {
    /// What the panel's controls say to write: those changed since the
    /// tiles were read.
    pub fn draft(&self) -> Draft {
        let (main, source, loops) = self.opened;
        let values = [
            self.main[0],
            self.main[1],
            self.source[0],
            self.source[1],
            u8::from(self.loops[0]),
            u8::from(self.loops[1]),
            u8::from(self.loops[2]),
            self.replace,
        ];
        let opened = [
            main[0],
            main[1],
            source[0],
            source[1],
            u8::from(loops[0]),
            u8::from(loops[1]),
            u8::from(loops[2]),
            self.opened_replace,
        ];
        let has = [
            self.has_main[0],
            self.has_main[1],
            self.has_source[0],
            self.has_source[1],
            self.has_loops[0],
            self.has_loops[1],
            self.has_loops[2],
            self.has_replace,
        ];
        let set = std::array::from_fn(|k| has[k] && (self.touched[k] || values[k] != opened[k]));
        let height =
            self.height.filter(|_| !self.height_locked && self.height != self.opened_height);
        Draft { values, set, height }
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
    crate::area_view::tile_of(view.ground_at(pos, 0.0)?, model.width, model.height)
}

/// The tiles a box dragged from `from` to `to` on screen covers, row by
/// row: those of the area between the ground points under its corners,
/// a corner past the area's edge taking the tiles up to it (none for a
/// box wholly past it). `None` where a corner is on the sky.
fn boxed_tiles(view: &AreaView, from: Pos2, to: Pos2) -> Option<Vec<(u32, u32)>> {
    let model = view.model.as_ref()?;
    let (a, b) = (view.ground_at(from, 0.0)?, view.ground_at(to, 0.0)?);
    let cell = |v: f32| (v / mg_area::TILE_SIZE).floor();
    let (x0, x1) = (cell(a.x.min(b.x)), cell(a.x.max(b.x)));
    let (y0, y1) = (cell(a.y.min(b.y)), cell(a.y.max(b.y)));
    let (w, h) = (model.width as f32, model.height as f32);
    if x1 < 0.0 || y1 < 0.0 || x0 >= w || y0 >= h {
        return Some(Vec::new());
    }
    let (x0, x1) = (x0.max(0.0) as u32, x1.min(w - 1.0) as u32);
    let (y0, y1) = (y0.max(0.0) as u32, y1.min(h - 1.0) as u32);
    Some((y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).collect())
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
    // (Not a press that is the camera's: Ctrl + drag, Alt + drag.)
    if response.drag_started_by(egui::PointerButton::Primary)
        && view.tile_box.is_none()
        && !command
        && !view.camera_press
    {
        let from = ui.input(|i| i.pointer.press_origin()).unwrap_or_default();
        view.tile_box = Some((from, from));
        view.tile_box_adds = shift;
    }
    if let Some((from, _)) = view.tile_box {
        // (Until the left button is let go, whatever the others do.)
        let held = view.held;
        if let Some(now) = held.pos() {
            view.tile_box = Some((from, now));
        }
        if held.cancelled() {
            // (One Escape, one thing: the box goes, and the tiles selected
            // and Select Tiles stay.)
            view.tile_box = None;
            return true;
        } else if !held.by(egui::PointerButton::Primary) {
            view.tile_box = None;
            // (With Shift, held as it began or as it ends: added to those
            // selected, as Shift + click adds a tile.)
            let adds = std::mem::take(&mut view.tile_box_adds) || shift;
            if let Some(tiles) = held.pos().and_then(|to| boxed_tiles(view, from, to)) {
                if !adds {
                    view.tile_selection.clear();
                }
                for t in tiles {
                    if !view.tile_selection.contains(&t) {
                        view.tile_selection.push(t);
                    }
                }
            }
        }
    }
    // A click on an object: back to the objects, that one selected (the
    // tiles under one are still taken by a box dragged over them, and a
    // click with Shift or Ctrl stays with the tiles).
    if response.clicked()
        && !command
        && !shift
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
        && let Some(pos) = response.interact_pointer_pos()
    {
        match tile_at(view, pos) {
            Some(t) if command => match view.tile_selection.iter().position(|s| *s == t) {
                Some(i) => {
                    view.tile_selection.remove(i);
                }
                None => view.tile_selection.push(t),
            },
            // Shift: added to those selected (kept, if it was).
            Some(t) if shift => {
                if !view.tile_selection.contains(&t) {
                    view.tile_selection.push(t);
                }
            }
            Some(t) => view.tile_selection = vec![t],
            // Past the area's edge (or on the sky): no tile, as a click on
            // the ground where no object is selects none.
            None if !command && !shift => view.tile_selection.clear(),
            None => {}
        }
    }
    if response.secondary_clicked()
        && shift
        && let Some(t) = response.interact_pointer_pos().and_then(|p| tile_at(view, p))
    {
        next_variant(app, view, t, false);
    }
    let typing = ui.ctx().memory(|m| m.focused().is_some());
    if !typing && view.tile_box.is_none() && crate::area_view::escaped(ui, response) {
        if !view.tile_selection.is_empty() {
            view.tile_selection.clear();
        } else {
            view.tile_mode = false;
        }
        return true;
    }
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
pub(crate) fn copy_tiles(app: &mut Moonglow, view: &AreaView) -> Option<TileClip> {
    let g = current_grid(app, view)?;
    let tools = view.terrain.as_ref()?;
    let are = app.ws.as_mut()?.doc(&ResKey::new(view.area, ResType::ARE)).ok()?;
    let list = are.root.list("Tile_List")?;
    let w = g.lattice.width();
    let x0 = view.tile_selection.iter().map(|c| c.0).min()?;
    let y0 = view.tile_selection.iter().map(|c| c.1).min()?;
    let selected: Vec<_> = view
        .tile_selection
        .iter()
        .filter_map(|&(x, y)| Some(((x, y), g.get_tile(x, y)?)))
        .collect();
    let low = selected.iter().map(|(_, p)| p.height).min()?;
    let tiles = selected
        .into_iter()
        .filter_map(|((x, y), p)| {
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
    let cancel = crate::area_view::cancelled(ui, response);
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
        mg_uikit::menu_note(ui, "No tile here");
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
            .on_hover_text("Remove crossers, and a group or a feature whole")
            .clicked()
    {
        delete(app, view);
        ui.close();
    }
    // The plugins' commands for tiles, run on them.
    let commands = app.plugin_menu_commands_cached();
    if !commands.is_empty() {
        let tiles = app.tiles_of(view.area, &cells);
        crate::plugins::menu_on(ui, &commands, &tiles, &mut app.actions);
    }
    // Properties last, as in Aurora's menus.
    mg_uikit::menu_separator(ui);
    if ui.button(PROPERTIES_ITEM).on_hover_text(PROPERTIES_TIP).clicked() {
        properties_of(app, view, &cells);
        ui.close();
    }
}

/// The tile menu's item that shows Tile Properties, and its tip.
pub(crate) const PROPERTIES_ITEM: &str = "Tile Properties";
const PROPERTIES_TIP: &str =
    "Select Tiles with these tiles selected, their properties in the Right Dock";

/// Shows Tile Properties for `cells` (the tile menu's item): Select Tiles
/// taken, if it isn't the tool in hand (a brush put down: the shelf's one
/// tool at a time), with `cells` selected, and the Right Dock's Tile
/// Properties panel shown (its header toggle turned on). Nothing in a
/// read-only area, which takes no Select Tiles.
pub(crate) fn properties_of(app: &mut Moonglow, view: &mut AreaView, cells: &[(u32, u32)]) {
    use crate::area_view::tools::{AreaTool, choose, tool_of};
    if tool_of(app, view) != AreaTool::Tiles {
        choose(app, view, AreaTool::Tiles);
    }
    if !view.tile_mode {
        return;
    }
    view.selection.clear();
    view.tile_selection = cells.to_vec();
    app.settings.hide_tile_properties = false;
    app.actions.push(crate::Action::ShowPanel(crate::Panel::TileProperties));
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
            "Another tile of the same kind here, turned as this one where it fits",
            TileStep::Appearance { back: false },
        ),
        (
            "Previous Tile Appearance",
            "A step back through them",
            TileStep::Appearance { back: true },
        ),
        (
            "Rotate Tile",
            "A quarter turn counter-clockwise, where the tile still fits its neighbors",
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
    mg_uikit::menu_heading(ui, format!("Tile ({}, {})", t.0, t.1));
    tile_items(app, view, ui, &[t]);
    // (Offered only in an area that may be changed: `editing_items`.)
    if ui.button(PROPERTIES_ITEM).on_hover_text(PROPERTIES_TIP).clicked() {
        properties_of(app, view, &[t]);
        ui.close();
    }
    mg_uikit::menu_separator(ui);
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
    /// fits there, or turned it no longer fits; a cell outside the grid).
    pub(crate) fn of(
        self,
        tools: &crate::terrain_mode::Tools,
        g: &mg_tiles::paint::Grid,
        (x, y): (u32, u32),
    ) -> Option<mg_tiles::Placement> {
        let now = g.get_tile(x, y)?;
        if g.locked(&tools.index, x, y) {
            return None;
        }
        let cell = g.lattice.get_cell(x, y)?;
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
pub(crate) fn locked_groups(
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
pub(crate) fn next_variant(
    app: &mut Moonglow,
    view: &mut AreaView,
    (x, y): (u32, u32),
    back: bool,
) {
    let Some(g) = current_grid(app, view) else { return };
    let Some(tools) = view.terrain.as_ref() else { return };
    if g.locked(&tools.index, x, y) {
        let shown = locked_groups(tools, &g, &[(x, y)]);
        view.notice = Some(format!("Next tile: {}", crate::terrain_mode::GROUP_LOCKED));
        view.refused = Some((shown, std::time::Instant::now()));
        return;
    }
    let (Some(cell), Some(now)) = (g.lattice.get_cell(x, y), g.get_tile(x, y)) else { return };
    let Some(next) = step_fit(&tools.index, &cell, now, back) else {
        view.notice = Some("No other tile fits there".into());
        return;
    };
    view.notice = None;
    crate::terrain_mode::tile_command(app, view, &g, &[((x, y), next)], "Next tile");
}

/// Delete on the selected tiles (a group's tile: the whole group, unless
/// their tiles are free to edit): also the Eraser's on them, from the
/// tiles palette or its key.
pub(crate) fn delete(app: &mut Moonglow, view: &mut AreaView) {
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
            replace: false,
        },
    }
}

/// A tile's values as Tile Properties shows them, read from its
/// `Tile_List` struct (as the area view reads them).
struct TileValues {
    id: i64,
    main: [u8; 2],
    source: [u8; 2],
    loops: [bool; 3],
    replace: u8,
    height: i32,
}

impl TileValues {
    fn of(t: &mg_gff::Struct) -> TileValues {
        let int = |label: &str| t.integer(label).unwrap_or(0);
        let byte = |label: &str| int(label).clamp(0, 255) as u8;
        TileValues {
            id: int("Tile_ID"),
            main: [byte("Tile_MainLight1"), byte("Tile_MainLight2")],
            source: [byte("Tile_SrcLight1"), byte("Tile_SrcLight2")],
            loops: [1, 2, 3].map(|n| int(&format!("Tile_AnimLoop{n}")) != 0),
            // (Without the field: row 0, as the client draws it.)
            replace: t.integer("Tile_ReplaceTex").map_or(1, |v| v.clamp(0, 255) as u8),
            height: int("Tile_Height").clamp(-1024, 1024) as i32,
        }
    }
}

/// Tile Properties of the selected tiles, with the first one's values,
/// as the workspace has them (the view may not have caught up with a
/// change just made); `None` where they can't be read (no tileset, a
/// tile past the area's list).
fn read_props(app: &mut Moonglow, view: &AreaView) -> Option<TileProps> {
    let tools = view.terrain.as_ref()?;
    let key = ResKey::new(view.area, ResType::ARE);
    let (rewrites, (tiles, base)) = {
        let ws = app.ws.as_mut()?;
        let rewrites = ws.rewrites(&key);
        let are = ws.doc(&key).ok()?;
        let width = are.root.integer("Width").unwrap_or(0).max(0) as u32;
        let list = are.root.list("Tile_List").unwrap_or(&[]);
        let tiles: Vec<usize> =
            view.tile_selection.iter().map(|&(x, y)| (y * width + x) as usize).collect();
        let base: Vec<mg_gff::Struct> =
            tiles.iter().filter_map(|&i| list.get(i).cloned()).collect();
        (rewrites, (tiles, base))
    };
    if base.len() != tiles.len() || base.is_empty() {
        return None;
    }
    let values: Vec<TileValues> = base.iter().map(TileValues::of).collect();
    let first = &values[0];
    // What each tile's model has (a model read once however many tiles
    // have it); the window enables what any of them has, as Aurora does.
    let mut read: std::collections::HashMap<i64, mg_area::TileFeatures> = Default::default();
    let has_each: Vec<mg_area::TileFeatures> = values
        .iter()
        .map(|t| *read.entry(t.id).or_insert_with(|| tile_features(app, tools, t.id)))
        .collect();
    let has = mg_area::TileFeatures::union(has_each.iter().copied());
    let cells = view.tile_selection.clone();
    let grid = current_grid(app, view);
    // Height: the tiles' own, or mixed; not to set where a group's tile is
    // among them without the rest of its group.
    let heights: Vec<i32> = values.iter().map(|t| t.height).collect();
    let height = heights.first().copied().filter(|h| heights.iter().all(|k| k == h));
    let height_locked = grid.as_ref().is_some_and(|g| {
        cells.iter().any(|&(x, y)| {
            g.locked(&tools.index, x, y)
                && !g
                    .group_cells(&tools.index, x, y)
                    .is_some_and(|group| group.iter().all(|c| cells.contains(c)))
        })
    });
    // (A larger group's tile among them: nothing but its group changes
    // it. A group of one tile, a staircase, changes as any tile.)
    let grouped = grid.as_ref().is_some_and(|g| {
        cells.iter().any(|&(x, y)| {
            g.locked(&tools.index, x, y)
                && g.group_cells(&tools.index, x, y).is_some_and(|c| c.len() > 1)
        })
    });
    let every =
        (tools.set.tiles.iter().enumerate()).map(|(i, t)| (i as u32, t.model.clone())).collect();
    // The tiles that fit every cell chosen, to choose among by picture
    // (not for a group's tile, which changes only with its group).
    let variants = grid.as_ref().filter(|_| !grouped && !cells.is_empty()).and_then(|g| {
        let fits = common_fits(&tools.index, g, &cells)?;
        let named = fits
            .into_iter()
            .filter_map(|p| Some((p, tools.set.tiles.get(p.tile as usize)?.model.clone())))
            .collect();
        let now: Vec<_> = cells.iter().map(|&(x, y)| g.get_tile(x, y)).collect::<Option<_>>()?;
        let same = now.iter().all(|p| (p.tile, p.orientation) == (now[0].tile, now[0].orientation));
        Some(((cells.clone(), named), same.then_some(now[0])))
    });
    let (variants, variant) = variants.unzip();
    let variant = variant.flatten();
    // (A loop the tiles don't have shows unchecked, as Aurora shows it.)
    let loops = [0, 1, 2].map(|k| first.loops[k] && has.loops[k]);
    let (main, source, replace) = (first.main, first.source, first.replace);
    let revision = app.ws.as_ref().map_or(0, mg_edit::Workspace::revision);
    Some(TileProps {
        revision,
        area: view.area,
        variants,
        variant,
        tiles,
        rewrites,
        main,
        source,
        loops,
        has_main: has.main,
        has_source: has.source,
        has_loops: has.loops,
        has_replace: has.replace,
        replace,
        opened_replace: replace,
        has_each,
        opened: (main, source, loops),
        touched: [false; 8],
        live: None,
        grouped,
        picking: None,
        cells,
        height,
        opened_height: height,
        first_height: first.height,
        first_id: first.id,
        step: tools.set.general.transition,
        height_locked,
        all_tiles: false,
        turn: 0,
        every,
        find: String::new(),
    })
}

/// lightcolor.2da's toolset colours (TOOLSETRED, GREEN, BLUE, 0 to 1).
pub(crate) fn light_colors(app: &Moonglow) -> Vec<Color32> {
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

fn cached_light_colors(app: &Moonglow, ui: &egui::Ui) -> std::sync::Arc<[Color32]> {
    let key = (app.game.is_some(), app.load_order);
    let id = egui::Id::new("tile-light-colors-cache");
    if let Some((was, cached)) =
        ui.data(|d| d.get_temp::<((bool, u64), std::sync::Arc<[Color32]>)>(id))
        && was == key
    {
        return cached;
    }
    let cached: std::sync::Arc<[Color32]> = light_colors(app).into();
    ui.data_mut(|d| d.insert_temp(id, (key, cached.clone())));
    cached
}

fn cached_replacetexture(app: &Moonglow, ui: &egui::Ui) -> std::sync::Arc<[mg_rules::Choice]> {
    let key = (app.game.is_some(), app.load_order);
    let id = egui::Id::new("tile-replacetexture-cache");
    if let Some((was, cached)) =
        ui.data(|d| d.get_temp::<((bool, u64), std::sync::Arc<[mg_rules::Choice]>)>(id))
        && was == key
    {
        return cached;
    }
    let rows: Vec<mg_rules::Choice> = (app.game.as_deref())
        .and_then(|g| g.table("replacetexture").ok())
        .map(|t| {
            (0..t.len())
                .take_while(|&r| mg_area::replacement_value(r).is_some())
                .map(|r| {
                    let text = format!("{r}: {}", t.get(r, "TEXTURENAME").unwrap_or(""));
                    mg_rules::Choice { row: r, text }
                })
                .collect()
        })
        .unwrap_or_default();
    let cached: std::sync::Arc<[mg_rules::Choice]> = rows.into();
    ui.data_mut(|d| d.insert_temp(id, (key, cached.clone())));
    cached
}

/// A color swatch button (`.swatch`: 36 by 22 of `raised`, `raised-hover`
/// under the pointer, the color 3 in from its edges with a dark outline;
/// the one chosen ringed in `accent`; grayed, `text-off`'s ground).
fn swatch(ui: &mut egui::Ui, color: Color32, enabled: bool, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(36.0, mg_uikit::metrics::CONTROL_HEIGHT),
        if enabled { egui::Sense::click() } else { egui::Sense::hover() },
    );
    let t = mg_uikit::tokens(ui.ctx());
    let ground = if enabled && response.hovered() { t.raised_hover } else { t.raised };
    ui.painter().rect_filled(rect, 3.0, ground);
    let inner = rect.shrink(3.0);
    let fill = if enabled { color } else { color.gamma_multiply(0.35) };
    ui.painter().rect_filled(inner, 2.0, fill);
    let outline = egui::Stroke::new(1.0, Color32::from_black_alpha(90));
    ui.painter().rect_stroke(inner, 2.0, outline, egui::StrokeKind::Outside);
    if selected {
        let ring = egui::Stroke::new(2.0, t.accent);
        ui.painter().rect_stroke(rect, 3.0, ring, egui::StrokeKind::Inside);
    }
    response
}

/// A swatch's name, for the screen reader (and the tests).
fn named(r: &egui::Response, name: &str, enabled: bool) {
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name));
}

/// The model of Tile Properties' first tile as it was last read, and its
/// tileset's environment map (`EnvMap`).
fn tile_model(app: &Moonglow, props: &TileProps) -> Option<(String, Option<String>)> {
    let tools = app.area_views.get(&props.area)?.terrain.as_ref()?;
    let id = usize::try_from(props.first_id).ok()?;
    let tile = tools.set.tiles.get(id)?;
    Some((tile.model.clone(), tools.set.general.env_map.clone()))
}

/// The words after Tile Properties' title in its header (the AreaTiles
/// board's "6 tiles", GalleryTiles' "Tile (4, 4)"): the tiles selected in
/// the area `editor` is, or none.
pub(crate) fn detail(app: &Moonglow, editor: Option<&crate::Tab>) -> String {
    let Some(crate::Tab::Area(area)) = editor else { return String::new() };
    let selected = app.area_views.get(area).map(|v| v.tile_selection.as_slice()).unwrap_or(&[]);
    match selected {
        [] => "none selected".into(),
        [(x, y)] => format!("Tile ({x}, {y})"),
        many => format!("{} tiles", many.len()),
    }
}

/// What the panel says with no tile selected.
pub const NO_TILES: &str = "Click a tile to see its properties here, or drag a box over several.";

/// Tile Properties of the tiles selected in `area`, as the panel is to
/// show them: those it has (with what was changed in it), or read again
/// where the selection, the area or its tiles have changed since (what
/// the panel shows but the tiles don't have: All tiles, its Turn and
/// search, the color being chosen, a Height being dragged, kept for the
/// same tiles). `None` with no tile selected or none to read.
fn props_now(app: &mut Moonglow, area: ResRef) -> Option<TileProps> {
    // (Another area's panel drawn in the frame too, a dragged out area's
    // beside a docked one: its own kept apart, not read again each frame.)
    let was = match app.tile_props.take() {
        Some(p) if p.area != area => {
            app.tile_props_aside.insert(p.area, p);
            app.tile_props_aside.remove(&area)
        }
        was => was,
    };
    let view = app.area_views.get(&area)?;
    let cells = view.tile_selection.clone();
    if cells.is_empty() {
        return None;
    }
    let key = ResKey::new(area, ResType::ARE);
    let revision = app.ws.as_ref().map_or(0, mg_edit::Workspace::revision);
    let rewrites = app.ws.as_ref().map_or(0, |ws| ws.rewrites(&key));
    let same_tiles = |p: &TileProps| p.area == area && p.cells == cells;
    if was
        .as_ref()
        .is_some_and(|p| same_tiles(p) && p.revision == revision && p.rewrites == rewrites)
    {
        return was;
    }
    let view = app.area_views.remove(&area)?;
    let read = read_props(app, &view);
    app.area_views.insert(area, view);
    let mut now = read?;
    if let Some(p) = was.filter(|p| same_tiles(p) && p.rewrites == rewrites) {
        (now.all_tiles, now.turn, now.find) = (p.all_tiles, p.turn, p.find);
        (now.picking, now.live) = (p.picking, p.live);
    }
    Some(now)
}

/// Tile Properties, the Right Dock's panel while Select Tiles is on (or
/// the area's window's, where it is dragged out): the tiles selected in
/// the area `editor` is, and its color picker. Each change is written at
/// once, as one step to undo ([`write_changes`]).
pub(crate) fn panel(app: &mut Moonglow, ui: &mut egui::Ui, editor: Option<&crate::Tab>) {
    let Some(crate::Tab::Area(area)) = editor.cloned() else { return };
    // (The pane F6 is at outlined, as the other panes are.)
    let keys = crate::palette_view::Keys::TileProperties;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    let focus = std::mem::take(&mut app.tile_props_focus);
    let Some(mut props) = props_now(app, area) else {
        let none = app.area_views.get(&area).is_none_or(|v| v.tile_selection.is_empty());
        ui.add_space(4.0);
        ui.weak(if none { NO_TILES } else { "These tiles can't be read." });
        return;
    };
    let ctx = &ui.ctx().clone();
    let mut picked = None;
    let mut pick = None;
    let mut defaults_asked = false;
    let mut dragging = false;
    // (The first control F6 gives the focus to.)
    let mut first: Option<egui::Id> = None;
    let colors = cached_light_colors(app, ui);
    let color = |i: usize| colors.get(i).copied().unwrap_or(Color32::BLACK);
    egui::ScrollArea::vertical().id_salt("tile-props-scroll").auto_shrink([false, false]).show(
        ui,
        |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            mg_uikit::section_heading(ui, "Lights and Loops");
            egui::Grid::new("tile_props").num_columns(3).spacing([10.0, 6.0]).show(ui, |ui| {
                for row in 0..4 {
                    // Main lights 1 and 2, then source lights 1 and 2; the
                    // loops beside the first three.
                    let (source, k) = (row >= 2, row % 2);
                    let has = if source { props.has_source[k] } else { props.has_main[k] };
                    let words = match source {
                        true => format!("Source light {}", k + 1),
                        false => format!("Main light {}", k + 1),
                    };
                    label_cell(ui, &words, has);
                    // A source light's value v shows as lightcolor row 2v.
                    let shown = match source {
                        true => color(2 * usize::from(props.source[k])),
                        false => color(usize::from(props.main[k])),
                    };
                    let r = swatch(ui, shown, has, false);
                    named(&r, &format!("{words} color"), has);
                    if has {
                        first.get_or_insert(r.id);
                    }
                    if r.on_hover_text("Click here to select a color").clicked() {
                        props.picking = Some(row);
                    }
                    if row < 3 {
                        // (One the tiles don't have shows unchecked, as
                        // Aurora's.)
                        let mut none = false;
                        let on =
                            if props.has_loops[row] { &mut props.loops[row] } else { &mut none };
                        let r = ui
                            .add_enabled(
                                props.has_loops[row],
                                mg_uikit::Checkbox::new(on, format!("Animation loop {}", row + 1)),
                            )
                            .on_hover_text("Check to play this tile animation");
                        props.touched[4 + row] |= r.changed();
                    }
                    ui.end_row();
                }
            });
            egui::Grid::new("tile_props_form").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                dragging = height_row(ui, &mut props);
                replacement_row(app, ui, &mut props);
                ui.label("");
                let tip = "The lighting scheme's first colors, and the loops";
                let r = ui.button("Defaults");
                first.get_or_insert(r.id);
                if r.on_hover_text(tip).clicked() {
                    defaults_asked = true;
                }
                ui.end_row();
            });
            // The textures of the tile's model (of the first, of several),
            // as the area has it now (a variant put in shows its own).
            if let Some((model, env)) = tile_model(app, &props) {
                let note = (props.tiles.len() > 1)
                    .then(|| format!("The first of the {} tiles chosen.", props.tiles.len()));
                let place = crate::model_textures::Place {
                    area_env: env.as_deref(),
                    max_height: Some(320.0),
                    open: false,
                    note: None,
                };
                crate::model_textures::section(app, ui, ("tile-props", props.area), &model, place);
                if let Some(note) = note {
                    ui.weak(note);
                }
            }
            if props.grouped {
                ui.weak(
                    "A tile of a group or a feature: it changes only with its group, so other \
                     tiles aren't offered here (Options › Area › Allow editing tiles of groups \
                     and features offers them).",
                );
            }
            // All tiles (ignore fit): an explicit switch, the fit-only
            // variants as ever with it off (#29).
            let free = !props.grouped && !props.cells.is_empty();
            let all = ui
                .add_enabled(free, mg_uikit::Checkbox::new(&mut props.all_tiles, ALL_TILES))
                .on_hover_text(
                    "Offer every tile of the tileset, whether it fits its neighbors or not",
                );
            if !free {
                all.on_disabled_hover_text(
                    "A tile of a group or a feature changes only with its group",
                );
            }
            if props.all_tiles && free {
                ui.horizontal_wrapped(|ui| {
                    crate::widgets::field_label(ui, "Turn");
                    for (k, name) in ["0°", "↺90°", "↺180°", "↺270°"].iter().enumerate() {
                        let r = ui.selectable_value(&mut props.turn, k as u8, *name).on_hover_text(
                            "Quarter turns counter-clockwise, for the tiles chosen and a tile \
                             clicked below",
                        );
                        if r.changed() {
                            pick = Some(Pick::Turn(k as u8));
                        }
                    }
                });
                let side = variant_side(app);
                // (Found by the search: by model name, or by number.)
                let found_list = cached_all_tiles(app, ui, &props);
                let all = props.every.len();
                ui.horizontal(|ui| {
                    let count = match found_list.len() {
                        n if n == all => format!("All tiles ({all} in the tileset)"),
                        n => format!("All tiles ({n} of {all})"),
                    };
                    ui.label(count).on_hover_text(
                        "Every tile of the tileset, turned as chosen. Click one to put it here.",
                    );
                    size_slider(app, ui, side);
                });
                let room = ui.available_width() - 30.0;
                let id = egui::Id::new(("tile-all-search", props.area));
                crate::widgets::SearchField::new(id, &mut props.find)
                    .hint(ALL_TILES_SEARCH)
                    .clear_tip("Show every tile")
                    .find_over()
                    .width(room)
                    .show(ui);
                if found_list.is_empty() {
                    ui.weak("No tile's model or number has that in it.");
                }
                if let Some(p) = all_tiles_pictures(app, ui, &props, &found_list, side) {
                    pick = Some(Pick::Any(p));
                }
            } else if let Some((cells, variants)) = props
                .variants
                .as_ref()
                .filter(|(_, v)| v.len() > 1 || (v.len() == 1 && props.variant.is_none()))
            {
                // The tiles' variants: as many across as the panel is
                // wide, in the panel's own scroll.
                let side = variant_side(app);
                let (label, tip) = if cells.len() == 1 {
                    (
                        format!("Variant ({} fit here)", variants.len()),
                        "The tiles that fit here, each turn of each. Click one to put it here.",
                    )
                } else {
                    (
                        format!("Variant ({} fit every tile chosen)", variants.len()),
                        "The tiles that fit every tile chosen, each turn of each. Click one to \
                         put it in each.",
                    )
                };
                ui.horizontal(|ui| {
                    ui.label(label).on_hover_text(tip);
                    size_slider(app, ui, side);
                });
                // (The kit's small cells, as many across as the panel is
                // wide, 6 apart.)
                let id = egui::Id::new(("tile-variants", props.area));
                if let Some(p) = variant_pictures(app, ui, id, variants, props.variant, side) {
                    picked = Some(p);
                }
            }
        },
    );
    if focus && let Some(id) = first {
        ui.memory_mut(|m| m.request_focus(id));
    }
    // The color picker: lightcolor's 32 colors (16 for source lights).
    if let Some(which) = props.picking {
        let source = which >= 2;
        let current = if source {
            2 * usize::from(props.source[which - 2])
        } else {
            usize::from(props.main[which])
        };
        let mut keep = true;
        crate::widgets::dialog(ctx, "Select a Color", &mut keep)
            .collapsible(false)
            .resizable(false)
            .show_bare(ctx, |ui| {
                egui::Grid::new("light_colors").spacing([4.0, 4.0]).show(ui, |ui| {
                    for row in 0..colors.len().min(32) {
                        if source && row % 2 == 1 {
                            continue;
                        }
                        let r = swatch(ui, color(row), true, row == current);
                        // (Named by lightcolor's row, for a screen reader.)
                        let n = if source { row / 2 } else { row };
                        named(&r, &format!("Light color {n}"), true);
                        if r.clicked() {
                            props.touched[which] = true;
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
    if let Some(p) = picked
        && props.variant != Some(p)
    {
        pick = Some(Pick::Variant(p));
    }
    // A tile put in: a step of its own; what the controls changed is
    // written after it.
    if let Some(what) = pick {
        put_in(app, &mut props, what);
    }
    if defaults_asked {
        defaults(app, &mut props);
    }
    write_changes(app, &mut props, dragging);
    app.tile_props = Some(props);
}

/// A label of Tile Properties' grids, bright where the tiles have what it
/// names and in `text-off` where they don't.
fn label_cell(ui: &mut egui::Ui, words: &str, enabled: bool) {
    if enabled {
        crate::widgets::field_label(ui, words);
    } else {
        ui.add_enabled(false, egui::Label::new(words));
    }
}

/// A variant's picture's side, in points: the usual, and the slider's
/// least and most (the most is the pictures' own size, in pixels): the
/// kit's small variant's.
const VARIANT: f32 = mg_uikit::gallery::SMALL_DEFAULT;
const VARIANT_MIN: f32 = mg_uikit::gallery::SMALL_MIN;
const VARIANT_MAX: f32 = mg_uikit::gallery::SMALL_MAX;
const _: () = assert!(VARIANT_MAX == crate::model_view::THUMBNAIL as f32);

/// The variants' pictures' side, as their slider was last left.
pub(crate) fn variant_side(app: &Moonglow) -> f32 {
    app.settings.tile_variant_side.map_or(VARIANT, f32::from).clamp(VARIANT_MIN, VARIANT_MAX)
}

/// The hint in All tiles' search field.
pub const ALL_TILES_SEARCH: &str = "Search model, number";

/// Tile Properties' All tiles switch's name.
pub const ALL_TILES: &str = "All Tiles (Ignore Fit)";

/// The pictures' Size slider, beside their heading.
fn size_slider(app: &mut Moonglow, ui: &mut egui::Ui, side: f32) {
    // (Tiles much alike are told apart larger.)
    let range = VARIANT_MIN..=VARIANT_MAX;
    if let Some(side) = crate::widgets::picture_size_slider(ui, side, range, Some("Size")) {
        app.settings.tile_variant_side = Some(side);
    }
}

/// Tile Properties' Height, two rows of its form's grid: the tiles'
/// `Tile_Height`, a step down or up with its buttons, or any number of
/// steps typed or dragged, and under it what a step is; set alone,
/// nothing around them fitted to them (#29). Whether the number is being
/// dragged (one step for the drag: [`write_changes`]).
fn height_row(ui: &mut egui::Ui, props: &mut TileProps) -> bool {
    let locked = props.height_locked;
    label_cell(ui, "Height", !locked);
    let mut dragging = false;
    ui.horizontal(|ui| {
        ui.add_enabled_ui(!locked, |ui| {
            // (Joined, as the boards draw the field between its buttons.)
            ui.spacing_mut().item_spacing.x = 1.0;
            let mut v = props.height.unwrap_or(props.first_height);
            let mut set = None;
            let step = |icon, name: &str, tip: &str, first: bool| {
                mg_uikit::KitButton::new(Some(icon), name)
                    .icon_only(true)
                    .look(mg_uikit::ButtonLook::Raised)
                    .joined(first, !first)
                    .tip(tip)
            };
            let tip = "A height step down for these tiles alone";
            let down = ui.add(step(mg_uikit::Icon::Minus, "Lower Height", tip, true));
            if down.clicked() {
                set = Some(v - 1);
            }
            let mut value =
                egui::DragValue::new(&mut v).range(-HEIGHT_RANGE..=HEIGHT_RANGE).speed(0.1);
            if props.height.is_none() {
                // (The tiles have different heights: one typed sets them all.)
                value = value.custom_formatter(|_, _| "Mixed".into());
            }
            let r = ui.add_sized(egui::vec2(48.0, ui.spacing().interact_size.y), value).on_hover_text(
                "The tiles' height in the tileset's height steps. Their neighbors don't change.",
            );
            dragging = r.dragged();
            if r.changed() {
                set = Some(v);
            }
            let tip = "A height step up for these tiles alone";
            let up = ui.add(step(mg_uikit::Icon::Plus, "Raise Height", tip, false));
            if up.clicked() {
                set = Some(v + 1);
            }
            if let Some(h) = set {
                props.height = Some(h.clamp(-HEIGHT_RANGE, HEIGHT_RANGE));
            }
        });
    });
    ui.end_row();
    ui.label("");
    let at = match props.height {
        Some(h) => format!("{} m", h as f32 * props.step),
        None => "mixed".into(),
    };
    ui.weak(format!("steps of {} m: {at}", props.step));
    ui.end_row();
    // (Grayed: why, as a line of its own, the tip saying what to do.)
    if locked {
        ui.label("");
        let why = egui::Label::new(egui::RichText::new(HEIGHT_LOCKED).weak()).wrap();
        ui.add(why)
            .on_hover_text("Select all of a group's or feature's tiles to raise or lower it");
        ui.end_row();
    }
    dragging
}

/// Tile Properties' Replacement texture, a row of its form's grid: the
/// `replacetexture.2da` row the
/// tiles' `replace_tex` is drawn with, chosen from the table's rows, each
/// written as its number plus one (as the client reads the field); a
/// value the client crashes on (0, or past the table) shown as such, said
/// beside it in the error color. Enabled where a tile's model has
/// `replace_tex`.
fn replacement_row(app: &Moonglow, ui: &mut egui::Ui, props: &mut TileProps) {
    let rows = cached_replacetexture(app, ui);
    let enabled = props.has_replace;
    label_cell(ui, REPLACEMENT, enabled);
    ui.horizontal(|ui| {
        let row = mg_area::replacement_row(Some(i64::from(props.replace)), rows.len());
        let r = ui.add_enabled_ui(enabled, |ui| {
            let mut list = crate::widgets::row_choice(
                egui::Id::new("tile_replace"),
                &rows,
                row.map(|r| r as i64),
            )
            .ordered()
            // (To the panel's edge, as the AreaTiles board draws it.)
            .width(ui.available_width())
            .hover_text(
                "What the tile's replace_tex meshes are drawn with: a row of replacetexture.2da",
            );
            if row.is_none() {
                list = list.shown(format!("{}: no row", props.replace));
            }
            list.show(ui)
        });
        if !enabled {
            r.response.on_disabled_hover_text(
                "None of these tiles' models has a mesh drawn with replace_tex",
            );
        }
        let chosen = r.inner.and_then(|r| mg_area::replacement_value(usize::try_from(r).ok()?));
        if let Some(value) = chosen {
            props.replace = value;
            props.touched[REPLACE] = true;
        }
    });
    ui.end_row();
    // (Under the box, in the panel's width: beside it, a dock 280 wide
    // cut it off.)
    if mg_area::replacement_row(Some(i64::from(props.replace)), rows.len()).is_none() {
        ui.label("");
        let red = ui.visuals().error_fg_color;
        ui.add(egui::Label::new(egui::RichText::new(CRASHES).color(red)).wrap());
        ui.end_row();
    }
}

/// What Replacement texture says of a value the game crashes on.
pub const CRASHES: &str = "crashes the game as the area loads";

/// Why Tile Properties' Height is grayed, under it.
pub const HEIGHT_LOCKED: &str = "(a group's tile, without the rest of its group)";

/// Tile Properties' Replacement texture's label.
pub const REPLACEMENT: &str = "Replacement texture";

/// Every tile of the tileset as pictures, turned as Tile Properties'
fn cached_all_tiles(
    app: &Moonglow,
    ui: &egui::Ui,
    props: &TileProps,
) -> std::sync::Arc<Vec<(mg_tiles::Placement, String)>> {
    let tileset =
        app.area_views.get(&props.area).and_then(|v| v.terrain.as_ref().map(|t| t.tileset));
    let key = (tileset, app.load_order, props.find.clone(), props.turn, props.every.len());
    let id = egui::Id::new("tile-all-tiles-cache");
    if let Some((was, cached)) = ui.data(|d| {
        d.get_temp::<(
            (Option<ResRef>, u64, String, u8, usize),
            std::sync::Arc<Vec<(mg_tiles::Placement, String)>>,
        )>(id)
    }) && was == key
    {
        return cached;
    }
    let needle = mg_core::text::Needle::new(&props.find);
    let list: Vec<(mg_tiles::Placement, String)> = props
        .every
        .iter()
        .filter(|(id, model)| needle.finds_in(&[model, &id.to_string()]))
        .map(|(id, model)| {
            (mg_tiles::Placement { tile: *id, orientation: props.turn, height: 0 }, model.clone())
        })
        .collect();
    let cached = std::sync::Arc::new(list);
    ui.data_mut(|d| d.insert_temp(id, (key, cached.clone())));
    cached
}

/// Every tile of the tileset as thumbnails, each turned as All tiles'
/// Turn says, as many across as the tab is wide; only the rows in sight
/// are drawn (and their pictures made), however many the tileset has.
/// The tile clicked, if one is.
fn all_tiles_pictures(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    props: &TileProps,
    list: &[(mg_tiles::Placement, String)],
    side: f32,
) -> Option<mg_tiles::Placement> {
    // (A scroll of its own inside the panel's, as tall as the panel shows
    // under its top: only the rows in sight drawn, however many.)
    let shown = (ui.clip_rect().bottom() - ui.cursor().top()).max(240.0);
    egui::ScrollArea::vertical()
        .id_salt("tile-all")
        .auto_shrink([false, false])
        .max_height(shown)
        .show(ui, |ui| {
            let id = egui::Id::new(("tile-all-pictures", props.area));
            variant_pictures(app, ui, id, list, props.variant, side)
        })
        .inner
}

/// The edits that set the tiles of `heights` (`Tile_List` index and
/// height) of `area` alone, and move what stands on them with them (in
/// height steps of `step` metres), from the area as the workspace has it.
fn height_edits(
    ws: &mut mg_edit::Workspace,
    area: ResRef,
    step: f32,
    heights: &[(usize, i32)],
) -> Vec<Edit> {
    let (are, git) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
    let Ok(are_root) = ws.doc(&are).map(|d| d.root.clone()) else { return Vec::new() };
    let git_root = ws.doc(&git).map(|d| d.root.clone()).unwrap_or_else(|_| mg_gff::Struct::new(0));
    let files = mg_area::edit::AreaFiles { are, are_root: &are_root, git, git_root: &git_root };
    mg_area::edit::tile_height_edits(&files, step, heights)
}

/// The tiles that fit every one of `cells`, in the order they fit the
/// first (Next Variant's), each as it fits there; `None` where a cell is
/// outside the area.
fn common_fits(
    index: &mg_tiles::TileIndex,
    g: &mg_tiles::paint::Grid,
    cells: &[(u32, u32)],
) -> Option<Vec<mg_tiles::Placement>> {
    let mut each = cells
        .iter()
        .map(|&(x, y)| Some(mg_tiles::paint::fits_in_order(index, &g.lattice.get_cell(x, y)?)));
    let mut fits = each.next()??;
    for other in each {
        let other = other?;
        fits.retain(|p| other.iter().any(|q| same_turned(*p, *q)));
    }
    Some(fits)
}

/// One tile, turned alike (at whatever height).
fn same_turned(a: mg_tiles::Placement, b: mg_tiles::Placement) -> bool {
    (a.tile, a.orientation % 4) == (b.tile, b.orientation % 4)
}

/// A tile put in from Tile Properties: a variant that fits, one of all the
/// tileset's, or the tiles chosen turned (All tiles' Turn).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Pick {
    Variant(mg_tiles::Placement),
    Any(mg_tiles::Placement),
    Turn(u8),
}

/// Puts a tile in from Tile Properties, as its own step to undo, at once:
/// - a variant, in each tile chosen, as it fits there (lights chosen
///   afresh);
/// - one of all the tiles, in each tile chosen as it is, turned as it is
///   shown, at that tile's height, nothing around fitted to it;
/// - the tiles chosen turned, each its own tile turned so, as it is
///   (keeping its lights), nothing around fitted to it.
///
/// Both of the last are refused, as the brushes are, over a group's tile.
/// The panel reads the tiles again after it ([`props_now`]); what its
/// controls changed in the frame is written after it.
fn put_in(app: &mut Moonglow, props: &mut TileProps, what: Pick) {
    let Some(mut view) = app.area_views.remove(&props.area) else { return };
    let made = pick_command(app, props, &mut view, what);
    app.area_views.insert(props.area, view);
    let Some((command, first)) = made else { return };
    if let Err(e) = app.apply(command)
        && !std::mem::take(&mut app.claims.held)
    {
        app.log.error(e.to_string());
    }
    if matches!(what, Pick::Variant(_)) {
        props.variant = Some(first);
    }
}

/// The command [`put_in`] applies (and the tile put in first), from the
/// area as the workspace has it; `None` where nothing changes or it is
/// refused (the status line and the log say why).
fn pick_command(
    app: &mut Moonglow,
    props: &TileProps,
    view: &mut AreaView,
    what: Pick,
) -> Option<(Command, mg_tiles::Placement)> {
    let g = current_grid(app, view)?;
    let tools = view.terrain.as_ref()?;
    let w = g.lattice.width();
    let mut changes = Vec::new();
    let mut refused = Vec::new();
    let mut kept = Vec::new();
    let label = match what {
        Pick::Variant(p) => {
            // (In each cell as it fits there: at that cell's height.)
            let (cells, _) = props.variants.as_ref()?;
            for &(x, y) in cells {
                g.get_tile(x, y)?;
                let fits = mg_tiles::paint::fits_in_order(&tools.index, &g.lattice.get_cell(x, y)?);
                let q = fits.into_iter().find(|q| same_turned(*q, p))?;
                changes.push(((x, y), q));
            }
            "Tile variant"
        }
        Pick::Any(_) | Pick::Turn(_) => {
            let are_doc = if matches!(what, Pick::Turn(_)) {
                app.ws.as_mut()?.doc(&ResKey::new(props.area, ResType::ARE)).ok()
            } else {
                None
            };
            let list = are_doc.and_then(|d| d.root.list("Tile_List")).unwrap_or(&[]);
            for &(x, y) in &props.cells {
                let now = g.get_tile(x, y)?;
                let q = match what {
                    Pick::Any(p) => mg_tiles::Placement { height: now.height, ..p },
                    Pick::Turn(t) => mg_tiles::Placement { orientation: t, ..now },
                    Pick::Variant(_) => unreachable!(),
                };
                if matches!(what, Pick::Turn(_)) && q == now {
                    continue;
                }
                match g.set_exact(&tools.index, x, y, q) {
                    Some(_) => {
                        changes.push(((x, y), q));
                        // (Turned, it is the same tile: its lights stay.)
                        if let (Pick::Turn(_), Some(s)) = (what, list.get((y * w + x) as usize)) {
                            kept.push(((x, y), s.clone()));
                        }
                    }
                    None => refused.extend(g.blocked()),
                }
            }
            if matches!(what, Pick::Turn(_)) {
                "Turn tile (ignoring fit)"
            } else {
                "Tile (ignoring fit)"
            }
        }
    };
    if !refused.is_empty() {
        let why = crate::terrain_mode::refusal(tools, &g, label, &refused, None);
        app.log.info(why.clone());
        view.notice = Some(why);
        view.refused = Some((refused, std::time::Instant::now()));
        return None;
    }
    if changes.is_empty() {
        return None;
    }
    view.notice = None;
    let first = changes[0].1;
    let command =
        crate::terrain_mode::tile_command_made(app, view, &g, &changes, label, &kept, Vec::new())?;
    Some((command, first))
}

/// Tile Properties' variants (or All tiles'): each its model's picture,
/// turned as the tile would lie, in the kit's small cells `side` points
/// across, 6 apart, as many across as there is room, only the rows in
/// sight drawn; its model's name under it in Hack, the quarter turns
/// under that where it is turned, the one there now selected; under the
/// pointer, at its largest beside the panel. The one clicked (or taken by
/// Enter), if one is.
fn variant_pictures(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    id: egui::Id,
    list: &[(mg_tiles::Placement, String)],
    now: Option<mg_tiles::Placement>,
    side: f32,
) -> Option<mg_tiles::Placement> {
    use mg_uikit::gallery;
    const TURNS: [&str; 4] = ["", "↺90°", "↺180°", "↺270°"];
    let pictured = |p: mg_tiles::Placement, model: &str| {
        let key = ResKey::parse(&model.to_ascii_lowercase(), ResType::MDL)?;
        Some(crate::model_view::Pictured::Tile(key, p.orientation))
    };
    let turned = list.iter().any(|(p, _)| p.orientation % 4 != 0);
    let current = list.iter().position(|(p, _)| Some(*p) == now);
    let keys = !crate::shell::keys_blocked(ui.ctx());
    let grid = gallery::GalleryGrid::new(id, list.len(), side)
        .small()
        .label(gallery::label_height(true, 1, turned))
        .cursor(current)
        .keys_allowed(keys)
        .name("Variants");
    let panel = ui.clip_rect();
    let (mut picked, mut hovered) = (None, None);
    let out = grid.show(ui, |ui, i, rect, tile, r, cell| {
        let (p, model) = &list[i];
        let made = pictured(*p, model).and_then(|k| app.thumbnails.made(k));
        let turn = TURNS[usize::from(p.orientation % 4)];
        // (Said with its turn, "tcn01_a02_01 ↺90°", as before.)
        let said = if turn.is_empty() { model.clone() } else { format!("{model} {turn}") };
        gallery::GalleryCell::new(model, crate::widgets::cell_picture(made))
            .said(&said)
            .detail((!turn.is_empty()).then_some(turn))
            .kind(mg_uikit::Icon::Tile)
            .model(true)
            .small(true)
            .selected(Some(*p) == now)
            .paint(ui, rect, tile, cell, r);
        if r.clicked() {
            picked = Some(*p);
        }
        if r.hovered() && !r.dragged() {
            hovered = Some((rect, format!("Tile {}: {said}", p.tile), made.flatten()));
        }
    });
    if let Some(i) = out.activated {
        picked = Some(list[i].0);
    }
    // (Larger, beside the panel, where the picture is shown smaller than
    // it was made: never over the cell it tells of.)
    if let Some((cell, title, made)) = hovered {
        // (Its picture's width and the frame's margins; its title alone, a
        // tooltip's width.)
        let width = if made.is_some() && side < VARIANT_MAX { VARIANT_MAX + 16.0 } else { 220.0 };
        // (Measured anew for another tile, or its picture made.)
        let what = (&title, made.is_some(), width as u32);
        gallery::show_beside(ui.ctx(), id.with("enlarged"), cell, panel, width, what, |ui| {
            ui.label(&title);
            if let Some(id) = made.filter(|_| side < VARIANT_MAX) {
                ui.add(egui::Image::new((id, egui::vec2(VARIANT_MAX, VARIANT_MAX))));
            }
        });
    }
    let ahead: Vec<_> = out.ahead.filter_map(|i| pictured(list[i].0, &list[i].1)).collect();
    app.thumbnails.show(ahead);
    picked
}

/// Defaults: the lighting scheme's first colours and every loop on (where
/// the tile has them), each of them set (shown in the area at once); the
/// replacement texture stays as it is.
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
    // (Not the replacement texture, which Aurora's Defaults hasn't got.)
    props.touched[..REPLACE].fill(true);
}

/// The edits that make the tiles what the window's draft says, from the
/// area as the workspace has it: each tile is written only the lights,
/// loops and replacement texture its own model has (its other fields stay), and the height, with
/// what stands on the tiles moved with them.
fn draft_edits(app: &mut Moonglow, props: &TileProps, draft: &Draft) -> Vec<Edit> {
    let key = ResKey::new(props.area, ResType::ARE);
    let Some(ws) = app.ws.as_mut() else { return Vec::new() };
    let Ok(are) = ws.doc(&key) else { return Vec::new() };
    let list = are.root.list("Tile_List").unwrap_or(&[]);
    let mut edits = Vec::new();
    for (n, &i) in props.tiles.iter().enumerate() {
        let Some(current) = list.get(i) else { continue };
        let has = props.has_each.get(n).copied().unwrap_or_default();
        for (k, label) in FIELDS.iter().enumerate() {
            if let Some(value) = draft.field(k, &has)
                && current.get(label) != Some(&value)
            {
                edits.push(Edit::SetField {
                    key,
                    path: GffPath::root().item("Tile_List", i),
                    label: (*label).into(),
                    value: Some(value),
                });
            }
        }
    }
    if let Some(h) = draft.height {
        let heights: Vec<(usize, i32)> = props.tiles.iter().map(|&i| (i, h)).collect();
        edits.extend(height_edits(ws, props.area, props.step, &heights));
    }
    edits
}

/// Writes what the panel's controls changed into the tiles, at once, as
/// one step to undo ("Tile Properties"): each tile only what its own
/// model has, and only where it differs. A Height being dragged
/// (`dragging`) writes one step for the drag: while it goes on, its own
/// step (still the last command) is taken back and written again. The
/// panel reads the tiles again after (the workspace's revision moved).
fn write_changes(app: &mut Moonglow, props: &mut TileProps, dragging: bool) {
    let draft = props.draft();
    let height_alone = !draft.set.iter().any(|s| *s) && draft.height.is_some();
    if !draft.set.iter().any(|s| *s) && draft.height.is_none() {
        if !dragging {
            props.live = None;
        }
        return;
    }
    let Some(ws) = app.ws.as_mut() else { return };
    // (Its own step, still the last command: the revision it left, and
    // the command its.)
    let own = ws.peek_undo().is_some_and(|c| c.label == "Tile Properties");
    let again = dragging && height_alone && own && props.live == Some(ws.revision());
    if again && let Err(e) = ws.withdraw() {
        app.log.error(format!("Tile Properties: {e}"));
    }
    let edits = draft_edits(app, props, &draft);
    props.live = None;
    if !edits.is_empty() {
        let before = app.ws.as_ref().map(mg_edit::Workspace::revision);
        match app.apply(Command::new("Tile Properties", edits)) {
            Ok(()) => {
                // (Only where it was written: `Moonglow::apply` says Ok to
                // a command it leaves, of a resource open only to look at,
                // and the step after must not take back another's.)
                let now = app.ws.as_ref().map(mg_edit::Workspace::revision);
                if dragging && height_alone && now != before {
                    props.live = now;
                }
            }
            Err(e) => {
                if !std::mem::take(&mut app.claims.held) {
                    app.log.error(e.to_string());
                }
            }
        }
    }
    // (Read again at the next frame, what was written then what the
    // panel shows; nothing written leaves the tiles as they are, and the
    // panel shows them so.)
    props.revision = u64::MAX;
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
    // tiles' heights), smoothed, and past `BOXES_MESHED` lines as one mesh
    // (`outlines`).
    // (Kept while the camera, the view, the area and the tiles selected
    // stay as they were.)
    let at = crate::area_view::OverlayKey::of(view);
    let kept = at.and_then(|at| {
        let kept = view.tile_lines_kept.borrow();
        let (was, shapes) = kept.as_ref()?;
        let same =
            (was.at, was.revision) == (at, view.revision) && was.tiles == view.tile_selection;
        same.then(|| shapes.clone())
    });
    let shapes = kept.unwrap_or_else(|| {
        let stroke = egui::Stroke::new(2.0, blue);
        let mut lines = Vec::with_capacity(view.tile_selection.len() * 4);
        for &(x, y) in &view.tile_selection {
            let Some(t) = model.tiles.get((y * model.width + x) as usize) else { continue };
            let Some(screen) = view.square_along_ground(x as f32, y as f32, t.position.z) else {
                continue;
            };
            if crate::area_view::beside(&screen, view.rect) {
                continue;
            }
            for k in 0..screen.len() {
                lines.push(([screen[k], screen[(k + 1) % screen.len()]], stroke));
            }
        }
        let shapes = crate::area_view::outlines(&lines);
        if let Some(at) = at {
            let key = crate::area_view::TileLinesKey {
                at,
                revision: view.revision,
                tiles: view.tile_selection.clone(),
            };
            *view.tile_lines_kept.borrow_mut() = Some((key, shapes.clone()));
        }
        view.tile_lines_made.set(view.tile_lines_made.get() + 1);
        shapes
    });
    painter.extend(shapes.iter().cloned());
    // The pasted block, under the pointer (over the view, not over a
    // window in front of it).
    if view.tile_pasting
        && let (Some(clip), Some(at)) = (clip, view.pointer_over(ui).and_then(|p| tile_at(view, p)))
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
