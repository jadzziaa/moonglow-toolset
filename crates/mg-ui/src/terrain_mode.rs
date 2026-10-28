//! The area viewer's terrain mode: painting with the tileset palette's
//! brushes (the palette pane's Tiles, as Aurora's Terrain tab).
//!
//! - A terrain brush or Raise/Lower acts on the lattice corner nearest the
//!   pointer (the right button lowers); the cursor is the square of the four
//!   tiles around it, red where Aurora would refuse the stroke. Dragged, it
//!   marks the corners the pointer passes (run back, it lets them go; with
//!   Shift, the rectangle from the first to the pointer's) and paints them
//!   all, as one command, when the button is let go.
//! - A crosser brush is dragged: the crosser goes on the edge of every
//!   quarter of a tile the pointer passes through (the quarter nearest that
//!   edge), as Aurora draws it. A click chooses the tile again.
//!   A right click on a quarter the crosser already crosses takes that
//!   crosser off the tile, as the Eraser would, leaving other crossers.
//! - The Eraser acts on the tile under the pointer: it removes its crossers
//!   (and those that then fit nothing), or chooses it again; with Shift it
//!   steps through the tiles that fit, in Aurora's order. Dragged, it marks
//!   the tiles the pointer passes, as a terrain brush marks corners, and
//!   erases them all when the button is let go.
//!
//! - A tile chosen from a terrain's or a crosser's pictures (the palette's
//!   Gallery) is placed as it is, turned as a group is (a right click turns
//!   it), the tiles around fitted to it; dragged, it goes in every tile
//!   the pointer passes. With the palette's Exact tile switch on
//!   (`Settings::exact_tiles`, issue #29) it goes in alone, at the height
//!   of the tile it replaces, nothing around it fitted to it
//!   (`Grid::set_exact`, as the plugin API's `set_tile`).
//! - Refine Tile steps a tile to the next that fits there (in Aurora's
//!   order, Shift + right click's: each tile's turns, then the next tile's),
//!   or with a tile chosen in the Gallery makes it that one where it fits.
//!   Dragged, it does so to the first tile, and every tile passed where
//!   that tile fits (the same kind: its terrain and crossers) becomes it
//!   too; others are left.
//! - Next and Previous Tile Appearance give the tile clicked another tile
//!   of its kind, turned as it was where that fits; Rotate Tile turns it a
//!   quarter (Shift: the other way) where it still fits its neighbours,
//!   keeping its lights. The tile menu has the same three. With tiles
//!   selected in Select Tiles, a click on one of the three in the palette
//!   steps the selected tiles at once instead (`tile_select::step_tiles`,
//!   as the tile menu's): nothing is chosen, Select Tiles and the
//!   selection stay, and another click steps them again.
//! - A right click with a terrain, the Eraser, Refine Tile or the tools
//!   above opens the tile menu (`tile_select::context_menu`) for the tile
//!   under the pointer; the brush stays chosen (Escape, or the menu's Put
//!   Down, lets it go). Raise/Lower, a crosser, a group and a tile keep
//!   their own right click.
//!
//! - A group or a feature is one thing: none of the above changes a tile
//!   of one, no group goes over another (the tiles in the way flash red),
//!   and the Eraser takes the whole of it away; the status line says how.
//!   Options › Area › Allow editing tiles of groups and features
//!   (`Grid::free_groups`) lets each tile of one change alone, with a
//!   warning on the status line when a group is left part changed.
//!
//! Every stroke is one undoable command on the ARE (`mg_tiles::paint`,
//! `mg_area::terrain`).

use std::sync::Arc;

use egui::{Color32, Pos2, Stroke as Line};
use glam::Vec3;
use mg_area::terrain::{Brush, PaletteItem, TilesetPalette, grid, tile_edits};
use mg_core::text::Needle;
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_resman::ResKey;
use mg_set::Tileset;
use mg_tiles::paint::{Grid, Rules, Stroke, next_fit};
use mg_tiles::{Placement, TileIndex};

use crate::Moonglow;
use crate::area_view::AreaView;
use crate::tile_select::TileStep;

/// The tileset brush chosen in the palette.
#[derive(Debug, Clone, PartialEq)]
pub struct TileBrush {
    pub tileset: ResRef,
    pub label: String,
    pub brush: Brush,
    /// Refine Tile's tile to make of those it acts on, chosen in the
    /// palette's Gallery (`None`: the next that fits).
    pub target: Option<u32>,
    /// A tile chosen in the Gallery goes in alone, as it is: the palette's
    /// Exact tile switch (`Settings::exact_tiles`), which [`active`] reads
    /// as the brush is used (false as the palette keeps it).
    pub exact: bool,
}

/// Tile `tile` as it fits cell `(x, y)` of `grid` (turned and raised to
/// fit), if it does there: turned as the tile there now is, where it fits
/// so too.
fn fit_of(tools: &Tools, grid: &Grid, (x, y): (u32, u32), tile: u32) -> Option<Placement> {
    let fits = mg_tiles::paint::fits_in_order(&tools.index, &grid.lattice.get_cell(x, y)?);
    let now = grid.get_tile(x, y)?;
    (fits.iter().find(|p| p.tile == tile && p.orientation == now.orientation))
        .or_else(|| fits.iter().find(|p| p.tile == tile))
        .copied()
}

/// What Refine Tile makes of the tile in `cell`: the brush's target tile
/// where it fits, else the next tile that fits there.
fn refined(tools: &Tools, grid: &Grid, brush: &TileBrush, (x, y): (u32, u32)) -> Option<Placement> {
    if grid.locked(&tools.index, x, y) {
        return None;
    }
    match brush.target {
        Some(t) => fit_of(tools, grid, (x, y), t),
        None => next_fit(&tools.index, &grid.lattice.get_cell(x, y)?, grid.get_tile(x, y)?),
    }
}

/// What painting an area's tiles needs from its tileset.
#[derive(Debug)]
pub struct Tools {
    pub tileset: ResRef,
    pub set: Arc<Tileset>,
    pub index: TileIndex,
    pub rules: Rules,
}

impl Tools {
    pub fn new(tileset: ResRef, set: Arc<Tileset>) -> Tools {
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        Tools { tileset, set, index, rules }
    }
}

/// Where the pointer is, for the brushes: the nearest lattice corner, the
/// tile under it and the edge of that tile it is nearest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    pub corner: (u32, u32),
    pub cell: (u32, u32),
    pub edge: usize,
    /// Near the tile's centre, where the quarters meet: a drag passing
    /// there stays in the quarter it was in.
    pub centre: bool,
    /// How far in from that edge (tiles: 0 on it, 0.5 at the centre).
    pub depth: f32,
    pub point: Vec3,
}

/// How near a side's edge (in tiles) a crosser drag must come, within a
/// tile, to turn into that side's quarter: the quarters are triangles
/// meeting at the centre, so a drag straight across a tile that is a little
/// off its middle would otherwise clip a side one.
const SIDE_REACH: f32 = 0.25;

/// Moonglow's own tools in the tiles palette, beside the Eraser: their
/// names and brushes.
const TOOLS: [(&str, Brush); 4] = [
    ("Refine Tile", Brush::Refine),
    ("Next Tile Appearance", Brush::Appearance { back: false }),
    ("Previous Tile Appearance", Brush::Appearance { back: true }),
    ("Rotate Tile", Brush::Rotate),
];

/// The tiles palette's Exact tile switch's name.
pub(crate) const EXACT_TILE: &str = "📌 Exact tile";

/// The tiles palette's Select Tiles row's name.
const SELECT_TILES: &str = "Select Tiles";

/// What a tile tool's click does (`shift`: held, Rotate Tile turning the
/// other way); `None` for other brushes.
fn tile_step(brush: &TileBrush, shift: bool) -> Option<TileStep> {
    match brush.brush {
        Brush::Appearance { back } => Some(TileStep::Appearance { back }),
        Brush::Rotate => Some(TileStep::Rotate { back: shift }),
        _ => None,
    }
}

/// Whether a right click with `brush` opens the tile menu: those with no
/// right click of their own (Raise/Lower lowers, a crosser is taken off,
/// a group or a tile is turned).
pub(crate) fn menu_on_right_click(brush: &TileBrush) -> bool {
    matches!(
        brush.brush,
        Brush::Terrain(_)
            | Brush::Eraser
            | Brush::Refine
            | Brush::Appearance { .. }
            | Brush::Rotate
    )
}

/// A terrain brush (or Raise/Lower, or the Eraser) dragged across the
/// area: the corners (the Eraser's tiles) it has marked, in order, to paint
/// when it is let go.
#[derive(Debug, Clone, Default)]
pub(crate) struct TerrainDrag {
    /// Where the pointer was last seen.
    at: Option<Vec3>,
    /// The corners passed (the path), or the tiles.
    corners: Vec<(u32, u32)>,
    /// The corner (or tile) under the pointer.
    here: Option<(u32, u32)>,
    /// It marks tiles, not corners (the Eraser).
    tiles: bool,
    /// Shift is held: the drag marks the rectangle from its first corner
    /// to the one under the pointer, not its path.
    fill: bool,
}

impl TerrainDrag {
    /// The corners (or tiles) to paint, in order: the path, or with Shift
    /// the rectangle from the first to the pointer's, row by row.
    fn marked(&self) -> Vec<(u32, u32)> {
        match (self.fill, self.corners.first(), self.here) {
            (true, Some(&(x0, y0)), Some((x1, y1))) => (y0.min(y1)..=y0.max(y1))
                .flat_map(|y| (x0.min(x1)..=x0.max(x1)).map(move |x| (x, y)))
                .collect(),
            _ => self.corners.clone(),
        }
    }

    /// What the drag marks at spot `s`: its corner, or its tile.
    fn mark(&self, s: Spot) -> (u32, u32) {
        if self.tiles { s.cell } else { s.corner }
    }

    /// The pointer reaching corner `c`: marked if new; back at the corner
    /// before the last, the last is let go (the drag run back).
    fn reach(&mut self, c: (u32, u32)) {
        let n = self.corners.len();
        if n >= 2 && self.corners[n - 2] == c {
            self.corners.pop();
        } else if !self.corners.contains(&c) {
            self.corners.push(c);
        }
    }
}

/// The spot at ground point `p` of a `width` × `height` area.
pub fn spot(p: Vec3, width: u32, height: u32) -> Option<Spot> {
    if width == 0 || height == 0 {
        return None;
    }
    let (u, v) = (p.x / mg_area::TILE_SIZE, p.y / mg_area::TILE_SIZE);
    if u < -0.5 || v < -0.5 || u > width as f32 + 0.5 || v > height as f32 + 0.5 {
        return None;
    }
    let corner =
        (u.round().clamp(0.0, width as f32) as u32, v.round().clamp(0.0, height as f32) as u32);
    let cell = (
        u.floor().clamp(0.0, (width - 1) as f32) as u32,
        v.floor().clamp(0.0, (height - 1) as f32) as u32,
    );
    let (fu, fv) = (u - cell.0 as f32, v - cell.1 as f32);
    // The nearest edge: south, east, north, west.
    let distances = [fv, 1.0 - fu, 1.0 - fv, fu];
    let edge = (0..4).min_by(|&a, &b| distances[a].total_cmp(&distances[b])).unwrap_or(0);
    let centre = (fu - 0.5).abs().max((fv - 0.5).abs()) < 0.1;
    Some(Spot { corner, cell, edge, centre, depth: distances[edge], point: p })
}

/// The brush that applies to `view`'s area, if one is chosen for its
/// tileset.
pub(crate) fn active(app: &Moonglow, view: &AreaView) -> Option<TileBrush> {
    let mut b = app.palette.tile_brush.clone()?;
    let tools = view.terrain.as_ref()?;
    b.exact = app.settings.exact_tiles && matches!(b.brush, Brush::Tile(_));
    (tools.tileset == b.tileset).then_some(b)
}

/// The area's grid as the workspace has it.
pub(crate) fn current_grid(app: &mut Moonglow, view: &AreaView) -> Option<Grid> {
    let tools = view.terrain.as_ref()?;
    let free = app.settings.edit_group_tiles;
    let ws = app.ws.as_mut()?;
    let are = ws.doc(&ResKey::new(view.area, ResType::ARE)).ok()?;
    let mut g = grid(&are.root, &tools.index)?;
    g.free_groups = free;
    Some(g)
}

/// Why a stroke of a group's tiles is refused, after its label.
pub(crate) const GROUP_LOCKED: &str = "tiles of a group or feature (in red) change only with \
     it whole: the Eraser takes all of it away, or Options › Area › Allow editing tiles of groups \
     and features";

/// The status line for a stroke `label` refused at `blocked` (the tiles
/// in the way, flashing red; `at`, the tile clicked, when none is).
pub(crate) fn refusal(
    tools: &Tools,
    grid: &Grid,
    label: &str,
    blocked: &[(u32, u32)],
    at: Option<(u32, u32)>,
) -> String {
    let locked = |&(x, y): &(u32, u32)| grid.locked(&tools.index, x, y);
    if blocked.iter().any(locked) || (blocked.is_empty() && at.as_ref().is_some_and(locked)) {
        format!("{label}: {GROUP_LOCKED}")
    } else if blocked.is_empty() {
        format!("{label}: no tile fits there")
    } else {
        format!("{label}: no tile fits the tiles shown in red")
    }
}

/// The tiles to flash red for a refused stroke: those in its way, or
/// where none is, the group of the tile clicked (`at`), if it is one's.
fn refused_cells(tools: &Tools, grid: &Grid, at: Option<(u32, u32)>) -> Vec<(u32, u32)> {
    let blocked = grid.blocked();
    match at {
        Some((x, y)) if blocked.is_empty() && grid.locked(&tools.index, x, y) => {
            grid.group_cells(&tools.index, x, y).unwrap_or_else(|| vec![(x, y)])
        }
        _ => blocked,
    }
}

/// The warning after a change with tiles of groups free to edit, where it
/// left `n` groups part changed.
fn broken_warning(label: &str, n: usize) -> String {
    let groups = if n == 1 { "a group or feature".to_string() } else { format!("{n} groups") };
    format!(
        "⚠ {label}: {groups} left part changed (Allow editing tiles of groups and features is on; \
         Undo puts it back)"
    )
}

/// The stroke `brush` makes at `spot`, and a label for it.
fn stroke(
    tools: &Tools,
    grid: &Grid,
    brush: &TileBrush,
    spot: Spot,
    lower: bool,
    cycle: bool,
    turns: u8,
) -> (Option<Stroke>, String) {
    let (x, y) = spot.corner;
    let label = format!("Paint {}", brush.label);
    // (A spot past the grid's edges, as a larger area had it: nothing.)
    if !grid.lattice.has_corner(x, y) || !grid.lattice.has_cell(spot.cell.0, spot.cell.1) {
        return (None, label);
    }
    match brush.brush {
        // Shift + click on a corner of the brush's terrain: the tiles
        // around it each step to their next variant, in Aurora's order.
        Brush::Terrain(t) if cycle && grid.lattice.corner(x, y).terrain == t => {
            let mut fixed = Vec::new();
            let (w, h) = (grid.lattice.width(), grid.lattice.height());
            for (dx, dy) in [(-1i64, -1i64), (0, -1), (-1, 0), (0, 0)] {
                let (cx, cy) = (i64::from(x) + dx, i64::from(y) + dy);
                if cx < 0 || cy < 0 || cx >= i64::from(w) || cy >= i64::from(h) {
                    continue;
                }
                let (cx, cy) = (cx as u32, cy as u32);
                if grid.locked(&tools.index, cx, cy) {
                    continue;
                }
                if let Some(p) =
                    next_fit(&tools.index, &grid.lattice.cell(cx, cy), grid.tile(cx, cy))
                {
                    fixed.push(((cx, cy), p));
                }
            }
            let s = Stroke { lattice: grid.lattice.clone(), cells: Vec::new(), fixed };
            (Some(s), "Next tiles".into())
        }
        Brush::Terrain(t) => (grid.paint(&tools.index, &tools.rules, x, y, t), label),
        Brush::RaiseLower => {
            let what = if lower { "Lower terrain" } else { "Raise terrain" };
            (grid.raise(&tools.index, &tools.rules, x, y, !lower), what.into())
        }
        Brush::Eraser if cycle => (next_tile(tools, grid, spot), "Next tile".into()),
        Brush::Refine => {
            let st = refined(tools, grid, brush, spot.cell).map(|_| Stroke {
                lattice: grid.lattice.clone(),
                cells: vec![spot.cell],
                fixed: Vec::new(),
            });
            (st, "Refine tile".into())
        }
        Brush::Appearance { .. } | Brush::Rotate => {
            let step = tile_step(brush, cycle);
            let st = step.and_then(|step| step.of(tools, grid, spot.cell)).map(|_| Stroke {
                lattice: grid.lattice.clone(),
                cells: vec![spot.cell],
                fixed: Vec::new(),
            });
            (st, step.map_or("", TileStep::label).into())
        }
        // Exact tile: in its cell alone, at the height of the tile there,
        // nothing around it fitted to it (as the plugin API's `set_tile`).
        Brush::Tile(t) if brush.exact => {
            let (cx, cy) = spot.cell;
            let height = grid.tile(cx, cy).height;
            let p = Placement { tile: t, orientation: turns % 4, height };
            let label = format!("Exact tile {}", brush.label);
            (grid.set_exact(&tools.index, cx, cy, p), label)
        }
        Brush::Tile(t) => {
            let p = Placement { tile: t, orientation: turns % 4, height: 0 };
            let (cx, cy) = spot.cell;
            (grid.place_tiles(&tools.index, &[((0, 0), Some(p))], cx, cy), label)
        }
        Brush::Eraser => (grid.erase(&tools.index, spot.cell.0, spot.cell.1), "Erase tile".into()),
        Brush::Crosser(c) => (grid.draw_crosser(&tools.index, &[], &[spot.cell], c), label),
        Brush::Group(g) => {
            let group = &tools.set.groups[g];
            let (cx, cy) = spot.cell;
            (grid.place_group(&tools.index, group, cx, cy, turns), format!("Place {}", brush.label))
        }
    }
}

/// The stroke choosing the tile under `spot` again (the next that fits
/// there, which the commit picks); `None` when no other fits, or it is a
/// group's.
fn next_tile(tools: &Tools, grid: &Grid, spot: Spot) -> Option<Stroke> {
    next_of(tools, grid, spot.cell).map(|_| Stroke {
        lattice: grid.lattice.clone(),
        cells: vec![spot.cell],
        fixed: Vec::new(),
    })
}

/// The next tile that fits cell `(x, y)` in Aurora's order (Shift + right
/// click's), unless the tile there is a group's.
pub(crate) fn next_of(tools: &Tools, grid: &Grid, (x, y): (u32, u32)) -> Option<Placement> {
    if grid.locked(&tools.index, x, y) {
        return None;
    }
    next_fit(&tools.index, &grid.lattice.get_cell(x, y)?, grid.get_tile(x, y)?)
}

/// Cells a stroke changed, and their new tiles.
type Changes = Vec<((u32, u32), Placement)>;

/// Puts a stroke into the ARE as one command; `pick` chooses each cell's
/// tile (at random among those that fit, if `None`).
fn commit(
    app: &mut Moonglow,
    view: &mut AreaView,
    mut grid: Grid,
    stroke: Option<Stroke>,
    label: &str,
    pick: Option<Placement>,
) {
    let Some(tools) = view.terrain.as_ref() else { return };
    let Some(stroke) = stroke else {
        // The tiles in the way flash red, as in Aurora (a group's tile
        // clicked, its group).
        let at = view.spot.map(|s| s.cell);
        view.notice = Some(refusal(tools, &grid, label, &grid.blocked(), at));
        view.refused = Some((refused_cells(tools, &grid, at), std::time::Instant::now()));
        return;
    };
    let before = grid.clone();
    // The tiles the preview showed; the next preview chooses anew.
    let changes = changes(tools, &mut grid, stroke, pick, view.preview_seed);
    view.preview_seed = fastrand::u64(..);
    view.notice = None;
    tile_command(app, view, &before, &changes, label);
}

/// What `stroke` (or `pick`, the tile chosen for its one cell) makes of
/// `grid`'s tiles, choosing among those that fit by `seed`: the cells
/// changed and their new tiles.
fn changes(
    tools: &Tools,
    grid: &mut Grid,
    stroke: Stroke,
    pick: Option<Placement>,
    seed: u64,
) -> Changes {
    match pick {
        Some(p) => {
            let Some(&(x, y)) = stroke.cells.first().filter(|c| grid.lattice.has_cell(c.0, c.1))
            else {
                return Vec::new();
            };
            let w = grid.lattice.width();
            grid.tiles[(y * w + x) as usize] = p;
            vec![((x, y), p)]
        }
        None => grid.apply(&tools.index, stroke, &mut fastrand::Rng::with_seed(seed)),
    }
}

/// What a click with `brush` at `s` does (Shift held or not; `lower`: the
/// right button with Raise/Lower; `turns`: a group's): its stroke and
/// label, and for a click that only steps a tile, the tile it steps to.
fn click(
    tools: &Tools,
    g: &Grid,
    brush: &TileBrush,
    s: Spot,
    shift: bool,
    lower: bool,
    turns: u8,
) -> (Option<Stroke>, String, Option<Placement>) {
    // (A brush's stroke reads only what is its own of `lower`, `cycle` and
    // `turns`.)
    let refine = brush.brush == Brush::Refine;
    let cycle = refine || shift && matches!(brush.brush, Brush::Eraser | Brush::Terrain(_));
    // (Rotate Tile's Shift: the other way round.)
    let cycle = cycle || shift && brush.brush == Brush::Rotate;
    let (st, label) = stroke(tools, g, brush, s, lower, cycle, turns);
    let pick = if refine {
        refined(tools, g, brush, s.cell)
    } else if let Some(step) = tile_step(brush, cycle) {
        step.of(tools, g, s.cell)
    } else if cycle && brush.brush == Brush::Eraser {
        next_of(tools, g, s.cell)
    } else {
        None
    };
    (st, label, pick)
}

/// What a click would make of the tiles under the pointer (Shift held or
/// not), or a drag going on if let go now, to show before it: the changed
/// tiles, as the area's with their new tile, and their indices. Nothing
/// for a stroke that changes nothing.
pub(crate) fn preview(app: &mut Moonglow, view: &mut AreaView, shift: bool) -> Option<Preview> {
    // Worked out anew only when what it depends on changes, not every frame.
    let key = preview_key(app, view, shift);
    if let Some((k, p)) = &view.preview_cache
        && *k == key
    {
        return p.clone();
    }
    let p = work_out_preview(app, view, shift);
    view.preview_cache = Some((key, p.clone()));
    p
}

/// What a tile preview shows: the changed tiles, and the indices of those
/// they replace.
pub(crate) type Preview = (Vec<mg_area::AreaTile>, Vec<usize>);

/// What a tile preview (and the brush's cursor) depends on, with Shift
/// held or not.
fn preview_key(app: &Moonglow, view: &AreaView, shift: bool) -> PreviewKey {
    PreviewKey {
        brush: active(app, view),
        // (Where in it the pointer is, to the millimetre, changes nothing.)
        spot: view.spot.map(|s| Spot { point: Vec3::ZERO, ..s }),
        shift,
        turns: view.group_turns,
        seed: view.preview_seed,
        revision: app.ws.as_ref().map(|ws| ws.revision()),
        marked: view.terrain_drag.as_ref().map(TerrainDrag::marked),
        crossing: crossing_shown(view),
        free_groups: app.settings.edit_group_tiles,
    }
}

/// What a tile preview depends on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PreviewKey {
    brush: Option<TileBrush>,
    spot: Option<Spot>,
    shift: bool,
    turns: u8,
    seed: u64,
    revision: Option<u64>,
    marked: Option<Vec<(u32, u32)>>,
    crossing: Vec<((u32, u32), usize)>,
    /// Options › Area › Allow editing tiles of groups and features.
    free_groups: bool,
}

fn work_out_preview(app: &mut Moonglow, view: &AreaView, shift: bool) -> Option<Preview> {
    let brush = active(app, view)?;
    let mut g = current_grid(app, view)?;
    let (model, tools) = (view.model.as_ref()?, view.terrain.as_ref()?);
    let seed = view.preview_seed;
    let changed = if let Some(drag) = &view.terrain_drag {
        // A drag: what letting go now would paint.
        let size = (model.width, model.height);
        paint_corners(tools, &mut g, &brush, drag, size, seed, view.group_turns).0
    } else if let (false, Brush::Crosser(c)) = (view.crossing.is_empty(), brush.brush) {
        let st = crosser_stroke(tools, &g, crossing_shown(view), c);
        changes(tools, &mut g, st?, None, seed)
    } else {
        let s = view.spot?;
        let (st, _, pick) = click(tools, &g, &brush, s, shift, false, view.group_turns);
        changes(tools, &mut g, st?, pick, seed)
    };
    let (mut tiles, mut hidden) = (Vec::new(), Vec::new());
    for ((x, y), p) in changed {
        let i = (y * model.width + x) as usize;
        let Some(old) = model.tiles.get(i) else { continue };
        let mut t = old.clone();
        t.id = i64::from(p.tile);
        t.model = tools.set.tiles.get(p.tile as usize).map(|t| t.model.to_ascii_lowercase());
        t.orientation = p.orientation;
        t.height = p.height;
        t.position.z = p.height as f32 * model.height_step;
        tiles.push(t);
        hidden.push(i);
    }
    (!tiles.is_empty()).then_some((tiles, hidden))
}

/// One command putting `changes` (cells and their new tiles) into the
/// area, with fresh lights, and the doors the tiles bring and take. Where
/// it leaves a group part changed (tiles of groups free to edit), the
/// status line warns.
pub(crate) fn tile_command(
    app: &mut Moonglow,
    view: &mut AreaView,
    before: &Grid,
    changes: &[((u32, u32), Placement)],
    label: &str,
) {
    tile_command_keeping(app, view, before, changes, label, &[]);
}

/// [`tile_command`], the cells of `kept` keeping those tile structs'
/// lights and animation loops (pasted tiles).
pub(crate) fn tile_command_keeping(
    app: &mut Moonglow,
    view: &mut AreaView,
    before: &Grid,
    changes: &[((u32, u32), Placement)],
    label: &str,
    kept: &[((u32, u32), mg_gff::Struct)],
) {
    tile_command_with(app, view, before, changes, label, kept, Vec::new());
}

/// [`tile_command_keeping`], with `extra` edits in the same command (what
/// stands on a tile raised or lowered, moved with it).
pub(crate) fn tile_command_with(
    app: &mut Moonglow,
    view: &mut AreaView,
    before: &Grid,
    changes: &[((u32, u32), Placement)],
    label: &str,
    kept: &[((u32, u32), mg_gff::Struct)],
    extra: Vec<mg_edit::Edit>,
) {
    if let Some(command) = tile_command_made(app, view, before, changes, label, kept, extra) {
        app.actions.push(crate::Action::Apply(command));
    }
}

/// The command [`tile_command_with`] applies, made against the area as
/// the workspace has it now, for a caller that applies it at once (Tile
/// Properties, whose own step goes over it); `None` when nothing changes.
pub(crate) fn tile_command_made(
    app: &mut Moonglow,
    view: &mut AreaView,
    before: &Grid,
    changes: &[((u32, u32), Placement)],
    label: &str,
    kept: &[((u32, u32), mg_gff::Struct)],
    extra: Vec<mg_edit::Edit>,
) -> Option<Command> {
    let broken = view.terrain.as_ref().map_or(0, |t| before.groups_broken(&t.index, changes).len());
    if broken > 0 {
        view.notice = Some(broken_warning(label, broken));
    }
    let (Some(ws), Some(game), Some(tools)) =
        (app.ws.as_mut(), app.game.as_deref(), view.terrain.as_ref())
    else {
        return None;
    };
    let key = ResKey::new(view.area, ResType::ARE);
    let Ok(are) = ws.doc(&key) else { return None };
    let root = are.root.clone();
    let scheme = root
        .integer("LightingScheme")
        .and_then(|row| mg_module::new::Scheme::read(game, row.max(0) as usize).ok());
    let mut lights_rng = fastrand::Rng::new();
    let mut lights = || scheme.as_ref().map_or([0; 3], |s| s.tile_lights(&mut lights_rng));
    let mut edits = tile_edits(key, &root, &tools.set, changes, &mut lights);
    let width = root.integer("Width").unwrap_or(0).max(0) as u32;
    for ((x, y), copied) in kept {
        let path = mg_edit::GffPath::root().item("Tile_List", (y * width + x) as usize);
        for label in [
            "Tile_MainLight1",
            "Tile_MainLight2",
            "Tile_SrcLight1",
            "Tile_SrcLight2",
            "Tile_AnimLoop1",
            "Tile_AnimLoop2",
            "Tile_AnimLoop3",
        ] {
            if let Some(v) = copied.get(label) {
                edits.push(mg_edit::Edit::SetField {
                    key,
                    path: path.clone(),
                    label: label.into(),
                    value: Some(v.clone()),
                });
            }
        }
    }
    let git_key = ResKey::new(view.area, ResType::GIT);
    if let Ok(git) = ws.doc(&git_key) {
        let git = git.root.clone();
        let read =
            |r: ResRef| mg_module::gff_root(Some(&ws.module), game, &ResKey::new(r, ResType::UTD));
        let step = tools.set.general.transition;
        let old = |(x, y): (u32, u32)| before.tile(x, y);
        edits.extend(mg_area::terrain::door_edits(
            game, git_key, &git, &tools.set, step, &old, changes, &read,
        ));
    }
    // (The extra edits first: they name the area's objects as they are,
    // before the doors the tiles take are removed.)
    let mut all = extra;
    all.extend(edits);
    (!all.is_empty()).then(|| Command::new(label, all))
}

/// Terrain mode's input; `false` when no tileset brush applies to the area
/// (the view then selects and moves objects).
pub(crate) fn input(
    app: &mut Moonglow,
    ui: &egui::Ui,
    view: &mut AreaView,
    response: &egui::Response,
) -> bool {
    let Some(brush) = active(app, view) else {
        view.crossing.clear();
        view.crossing_at = None;
        view.crossing_outline = None;
        view.terrain_drag = None;
        return false;
    };
    let (Some(model), Some(_)) = (view.model.as_ref(), view.terrain.as_ref()) else { return true };
    let (w, h) = (model.width, model.height);
    let at = |view: &AreaView, pos: Pos2| view.ground_at(pos, 0.0).and_then(|p| spot(p, w, h));
    view.spot = response.hover_pos().and_then(|p| at(view, p));
    let shift = ui.input(|i| i.modifiers.shift);
    if response.hovered() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.palette.tile_brush = None;
        view.crossing.clear();
        view.crossing_outline = None;
        view.terrain_drag = None;
        return true;
    }
    // A press begun with Ctrl held moves the view (Ctrl + drag): the brush
    // paints nothing with it, during it or when it is let go.
    let (pressed, command) = ui.input(|i| (i.pointer.any_pressed(), i.modifiers.command));
    if pressed {
        view.camera_press = command;
    }
    if view.camera_press {
        return true;
    }
    match brush.brush {
        Brush::Crosser(c) => {
            let press = ui.input(|i| i.pointer.press_origin());
            if response.drag_started_by(egui::PointerButton::Primary)
                && view.crossing.is_empty()
                && let Some(s) = press.and_then(|p| at(view, p))
            {
                view.crossing = vec![(s.cell, s.edge)];
                view.crossing_at = Some(s.point);
            }
            if view.held.by(egui::PointerButton::Primary)
                && let Some(s) = view.spot
                && let Some(from) = view.crossing_at
            {
                // Every metre of the way, so that no quarter is skipped.
                let steps = (s.point - from).truncate().length().ceil().max(1.0) as u32;
                for k in 1..=steps {
                    let p = from.lerp(s.point, k as f32 / steps as f32);
                    if let Some(q) = spot(p, w, h) {
                        pass(&mut view.crossing, q);
                    }
                }
                view.crossing_at = Some(s.point);
                view.crossing_outline = shift.then_some(s.cell);
            }
            if !view.held.by(egui::PointerButton::Primary) && !view.crossing.is_empty() {
                view.crossing_at = None;
                let edges = crossing_shown(view);
                view.crossing.clear();
                view.crossing_outline = None;
                if let Some(g) = current_grid(app, view) {
                    let tools = view.terrain.as_ref().expect("checked");
                    let s = crosser_stroke(tools, &g, edges, c);
                    commit(app, view, g, s, &format!("Paint {}", brush.label), None);
                }
            } else if response.clicked()
                && let Some(s) = view.spot
                && let Some(g) = current_grid(app, view)
            {
                let tools = view.terrain.as_ref().expect("checked");
                let (st, label, pick) = click(tools, &g, &brush, s, shift, false, 0);
                commit(app, view, g, st, &label, pick);
            } else if response.secondary_clicked()
                && let Some(s) = view.spot
                && let Some(g) = current_grid(app, view)
                && g.lattice
                    .get_cell(s.cell.0, s.cell.1)
                    .is_some_and(|t| t.edges[s.edge] == Some(c))
            {
                // A right click where the crosser is (the cursor blue):
                // the Eraser's click on that tile for this crosser alone
                // (the road, not the stream it crosses), the brush kept.
                let tools = view.terrain.as_ref().expect("checked");
                let st = g.erase_only(&tools.index, s.cell.0, s.cell.1, Some(c));
                commit(app, view, g, st, &format!("Erase {}", brush.label), None);
            }
        }
        Brush::Group(_) => {
            // Right click turns the group a quarter, as in Aurora.
            if response.secondary_clicked() {
                view.group_turns = (view.group_turns + 1) % 4;
            } else if response.clicked()
                && let Some(s) = view.spot
                && let Some(g) = current_grid(app, view)
            {
                let tools = view.terrain.as_ref().expect("checked");
                let (st, label, pick) = click(tools, &g, &brush, s, shift, false, view.group_turns);
                // (It stays chosen, to place another.)
                commit(app, view, g, st, &label, pick);
            }
        }
        // A right click turns a tile chosen alone a quarter, as a group.
        Brush::Tile(_) if response.secondary_clicked() => {
            view.group_turns = (view.group_turns + 1) % 4;
        }
        Brush::Terrain(_) | Brush::RaiseLower | Brush::Eraser | Brush::Refine | Brush::Tile(_)
            if terrain_drag(app, view, response, &brush) => {}
        // The tile tools: a click steps the tile under the pointer, as the
        // tile menu's items do.
        Brush::Appearance { .. } | Brush::Rotate => {
            if response.clicked()
                && let Some(s) = view.spot
                && let Some(step) = tile_step(&brush, shift)
            {
                crate::tile_select::step_tiles(app, view, &[s.cell], step);
            }
        }
        _ => {
            let lower = response.secondary_clicked() && brush.brush == Brush::RaiseLower;
            if (response.clicked() || lower)
                && let Some(s) = view.spot
                && let Some(g) = current_grid(app, view)
            {
                let tools = view.terrain.as_ref().expect("checked");
                let turns = view.group_turns;
                let (st, label, pick) = click(tools, &g, &brush, s, shift, lower, turns);
                commit(app, view, g, st, &label, pick);
            }
            // (Other brushes' right click: the tile menu, which the area
            // view opens. Raise/Lower's lowers: where it finds no ground to
            // lower, a cliff face or past the area's edge, it does nothing.
            // Dropped there, the next click was the object's under the
            // pointer: its menu, or selected.)
        }
    }
    true
}

/// A terrain brush (or Raise/Lower) dragged with the primary button: it
/// marks the corners the pointer passes (the Eraser, the tiles), and when
/// it is let go paints them, in order, as one command; whether a drag is
/// going on.
fn terrain_drag(
    app: &mut Moonglow,
    view: &mut AreaView,
    response: &egui::Response,
    brush: &TileBrush,
) -> bool {
    let Some(model) = view.model.as_ref() else { return false };
    let (w, h) = (model.width, model.height);
    if response.drag_started_by(egui::PointerButton::Primary) && view.terrain_drag.is_none() {
        let press = response.ctx.input(|i| i.pointer.press_origin());
        let from = press.and_then(|p| view.ground_at(p, 0.0));
        let tiles = matches!(brush.brush, Brush::Eraser | Brush::Refine | Brush::Tile(_));
        view.terrain_drag = Some(TerrainDrag { at: from, tiles, ..Default::default() });
    }
    let Some(mut drag) = view.terrain_drag.take() else { return false };
    // Every metre of the way, so that no corner is skipped.
    if let Some(to) = view.spot.map(|s| s.point) {
        let from = drag.at.unwrap_or(to);
        let steps = (to - from).truncate().length().ceil().max(1.0) as u32;
        for k in 0..=steps {
            if let Some(s) = spot(from.lerp(to, k as f32 / steps as f32), w, h) {
                drag.reach(drag.mark(s));
            }
        }
        drag.at = Some(to);
        drag.here = view.spot.map(|s| drag.mark(s));
    }
    if view.held.by(egui::PointerButton::Primary) {
        drag.fill = response.ctx.input(|i| i.modifiers.shift);
        view.terrain_drag = Some(drag);
        return true;
    }
    // Let go: each corner's stroke on the tiles the last one left.
    let (Some(mut g), Some(tools)) = (current_grid(app, view), view.terrain.as_ref()) else {
        return true;
    };
    let before = g.clone();
    // The tiles the drag's preview showed; the next chooses anew.
    let turns = view.group_turns;
    let (changes, label, refused) =
        paint_corners(tools, &mut g, brush, &drag, (w, h), view.preview_seed, turns);
    view.preview_seed = fastrand::u64(..);
    // (A group's tiles among those marked, or around a corner marked.)
    let locked = |&(x, y): &(u32, u32)| {
        let cells = if drag.tiles {
            vec![(x, y)]
        } else {
            [(0, 0), (1, 0), (0, 1), (1, 1)]
                .iter()
                .filter(|&&(dx, dy)| x >= dx && y >= dy && x - dx < w && y - dy < h)
                .map(|&(dx, dy)| (x - dx, y - dy))
                .collect()
        };
        cells.iter().any(|&(cx, cy)| before.locked(&tools.index, cx, cy))
    };
    let grouped = brush.brush != Brush::Eraser && drag.marked().iter().any(locked);
    view.notice = None;
    tile_command(app, view, &before, &changes, &label);
    let what = if drag.tiles { "tiles" } else { "corners" };
    if refused > 0 {
        view.notice = Some(match brush.brush {
            _ if grouped => format!("{label}: {refused} {what} left as they were: {GROUP_LOCKED}"),
            Brush::Refine => format!("{label}: {refused} {what} of another kind left as they were"),
            _ => format!("{label}: no tile fits at {refused} {what}"),
        });
    }
    true
}

/// A crosser drag's stroke along `edges` (its quarters), or for one
/// quarter alone, the tile chosen again.
fn crosser_stroke(
    tools: &Tools,
    g: &Grid,
    mut edges: Vec<((u32, u32), usize)>,
    c: mg_tiles::Crosser,
) -> Option<Stroke> {
    edges.dedup();
    match edges.as_slice() {
        [] => None,
        [(cell, _)] => g.draw_crosser(&tools.index, &[], &[*cell], c),
        _ => g.draw_crosser(&tools.index, &edges, &[], c),
    }
}

/// A terrain drag's strokes, corner (or tile) by corner in order, each on
/// the tiles the last left, choosing among those that fit by `seed`: the
/// cells changed and their new tiles, the command's label, and how many
/// corners no tile fitted.
fn paint_corners(
    tools: &Tools,
    g: &mut Grid,
    brush: &TileBrush,
    drag: &TerrainDrag,
    (w, h): (u32, u32),
    seed: u64,
    turns: u8,
) -> (Changes, String, usize) {
    if brush.brush == Brush::Refine {
        return refine_cells(tools, g, brush, &drag.marked());
    }
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut changed: Vec<(u32, u32)> = Vec::new();
    let (mut label, mut refused) = (format!("Paint {}", brush.label), 0);
    // A tile's spot is its middle.
    let middle = if drag.tiles { 0.5 } else { 0.0 };
    for (x, y) in drag.marked() {
        let at = Vec3::new(
            (x as f32 + middle) * mg_area::TILE_SIZE,
            (y as f32 + middle) * mg_area::TILE_SIZE,
            0.0,
        );
        let Some(s) = spot(at, w, h) else { continue };
        let s = if drag.tiles { s } else { Spot { corner: (x, y), ..s } };
        let (st, l) = stroke(tools, g, brush, s, false, false, turns);
        match st {
            Some(st) => {
                label = l;
                for (cell, _) in g.apply(&tools.index, st, &mut rng) {
                    if !changed.contains(&cell) {
                        changed.push(cell);
                    }
                }
            }
            None => refused += 1,
        }
    }
    let changes = changed.iter().map(|&(x, y)| ((x, y), g.tile(x, y))).collect();
    (changes, label, refused)
}

/// A Refine Tile drag over `cells`, in order: the first becomes what a
/// click would make of it (the next tile that fits there, or the brush's
/// target tile), and every other where that tile fits as it stands (the
/// same kind of tile: its terrain and crossers) becomes it too, turned to
/// fit; the rest, and a group's tiles, are left. The cells changed and
/// their new tiles, the label, and how many were left.
fn refine_cells(
    tools: &Tools,
    g: &mut Grid,
    brush: &TileBrush,
    cells: &[(u32, u32)],
) -> (Changes, String, usize) {
    let label = "Refine tiles".to_string();
    let Some(&first) = cells.first() else { return (Vec::new(), label, 0) };
    // (With a target, the tiles where it fits, the first among them.)
    let first_made = refined(tools, g, brush, first);
    let Some(tile) = brush.target.or(first_made.map(|p| p.tile)) else {
        return (Vec::new(), label, cells.len());
    };
    let w = g.lattice.width();
    let (mut changes, mut left) = (Vec::new(), 0);
    for &(x, y) in cells {
        let made = if (x, y) == first {
            first_made
        } else if g.locked(&tools.index, x, y) {
            None
        } else {
            fit_of(tools, g, (x, y), tile)
        };
        match made {
            Some(p) if p != g.tile(x, y) => {
                g.tiles[(y * w + x) as usize] = p;
                changes.push(((x, y), p));
            }
            Some(_) => {}
            None => left += 1,
        }
    }
    (changes, label, left)
}

/// The quarters a crosser drag has marked: its path, or with Shift the
/// outline of the rectangle from its first tile to the pointer's.
fn crossing_shown(view: &AreaView) -> Vec<((u32, u32), usize)> {
    match (view.crossing.first(), view.crossing_outline) {
        (Some(&(from, _)), Some(to)) => outline(from, to),
        _ => view.crossing.clone(),
    }
}

/// The quarters (both sides of each edge crossed) of a crosser along the
/// outline of the rectangle of tiles from `a` to `b`: round its ring of
/// tiles, or straight along a rectangle one tile wide; the tile alone when
/// they are one.
fn outline(a: (u32, u32), b: (u32, u32)) -> Vec<((u32, u32), usize)> {
    let (x0, x1, y0, y1) = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
    let cells: Vec<(u32, u32)> = if x0 == x1 || y0 == y1 {
        (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).collect()
    } else {
        let mut ring: Vec<(u32, u32)> = (x0..=x1).map(|x| (x, y0)).collect();
        ring.extend((y0 + 1..=y1).map(|y| (x1, y)));
        ring.extend((x0..x1).rev().map(|x| (x, y1)));
        ring.extend((y0..y1).rev().map(|y| (x0, y)));
        ring
    };
    if cells.len() == 1 {
        return vec![(cells[0], mg_tiles::SOUTH)];
    }
    cells
        .windows(2)
        .flat_map(|w| {
            let there = mg_tiles::paint::path_edges(&[w[0], w[1]]).unwrap_or_default();
            let back = mg_tiles::paint::path_edges(&[w[1], w[0]]).unwrap_or_default();
            there.into_iter().chain(back)
        })
        .collect()
}

/// A crosser drag reaching spot `q`: its quarter joins the path when it is
/// a new one, and back in the one before, the last is let go (a drag run
/// back). Near a tile's centre, where the quarters meet, the drag stays in
/// its quarter, or enters by the edge it came across; within a tile it
/// turns into a side quarter only near that side's edge ([`SIDE_REACH`]).
fn pass(path: &mut Vec<((u32, u32), usize)>, q: Spot) {
    let Some(&(last, last_edge)) = path.last() else { return };
    let side = last == q.cell && (last_edge + q.edge) % 2 == 1;
    if side && !q.centre && q.depth > SIDE_REACH {
        return;
    }
    let quarter = if q.centre {
        if last == q.cell {
            return;
        }
        let (dx, dy) = (last.0 as i64 - q.cell.0 as i64, last.1 as i64 - q.cell.1 as i64);
        match (dx, dy) {
            (0, -1) => (q.cell, mg_tiles::SOUTH),
            (1, 0) => (q.cell, mg_tiles::EAST),
            (0, 1) => (q.cell, mg_tiles::NORTH),
            (-1, 0) => (q.cell, mg_tiles::WEST),
            _ => return,
        }
    } else {
        (q.cell, q.edge)
    };
    // Back into the quarter it came from: the drag is run back, and the
    // quarter it leaves is let go.
    if path.len() >= 2 && path[path.len() - 2] == quarter {
        path.pop();
    } else if path.last() != Some(&quarter) {
        path.push(quarter);
    }
}

/// The quarter a crosser drag along `path` takes at spot `q` (the cursor
/// shows it): the one [`pass`] would add, else the drag's last; before a
/// drag, the quarter under the pointer.
fn next_quarter(path: &[((u32, u32), usize)], q: Spot) -> ((u32, u32), usize) {
    let mut next = path.to_vec();
    pass(&mut next, q);
    next.last().copied().unwrap_or((q.cell, q.edge))
}

/// The cursor's colour where a click chooses tiles again (steps through
/// the tiles that fit) rather than paints: the blue of a blueprint about
/// to be placed.
pub const CYCLE: Color32 = Color32::from_rgb(120, 230, 255);

/// The brush's cursor over the view: the square of tiles around the corner
/// (red where the stroke would be refused), the tile for the Eraser, the
/// quarters a crosser drag has passed; blue ([`CYCLE`]) where a click
/// chooses tiles again rather than paints.
pub(crate) fn overlay(app: &mut Moonglow, ui: &egui::Ui, view: &mut AreaView) {
    let shift = ui.input(|i| i.modifiers.shift);
    // Worked out anew only when what it depends on changes.
    let key = preview_key(app, view, shift);
    let shapes = match view.cursor_cache.take() {
        Some((k, shapes)) if k == key => shapes,
        _ => cursor(app, view, shift),
    };
    view.cursor_cache = Some((key, shapes.clone()));
    let painter = ui.painter_at(view.rect);
    for (points, color) in &shapes {
        let screen: Vec<Pos2> = points.iter().filter_map(|p| view.screen_pos(*p)).collect();
        if screen.len() == points.len() {
            crate::area_view::outline(&painter, &screen, Line::new(2.0, *color));
        }
    }
    view.brush_cursor = shapes;
}

/// The brush's cursor ([`overlay`]), with Shift held or not: its outlines,
/// on the ground, and their colours.
fn cursor(app: &mut Moonglow, view: &AreaView, shift: bool) -> Vec<(Vec<Vec3>, Color32)> {
    let shapes = std::cell::RefCell::new(Vec::new());
    cursor_shapes(app, view, shift, &shapes);
    shapes.into_inner()
}

fn cursor_shapes(
    app: &mut Moonglow,
    view: &AreaView,
    shift: bool,
    shapes: &std::cell::RefCell<Vec<(Vec<Vec3>, Color32)>>,
) {
    let Some(brush) = active(app, view) else { return };
    let (Some(model), Some(tools)) = (view.model.as_ref(), view.terrain.as_ref()) else { return };
    let step = model.height_step;
    let Some(g) = current_grid(app, view) else { return };
    let z = |x: u32, y: u32| g.lattice.get_corner(x, y).map_or(0.0, |c| c.height as f32 * step);
    let point =
        |x: f32, y: f32, z: f32| Vec3::new(x * mg_area::TILE_SIZE, y * mg_area::TILE_SIZE, z);
    // On the ground the pointer picks.
    let polygon = |points: &[Vec3], color: Color32| {
        shapes.borrow_mut().push((view.on_ground(points), color));
    };
    let quarter = |(cx, cy): (u32, u32), edge: usize, color: Color32| {
        let (x, y) = (cx as f32, cy as f32);
        let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let (a, b) = (corners[edge], corners[(edge + 1) % 4]);
        let h = z(cx, cy) + 0.05;
        polygon(
            &[point(x + 0.5, y + 0.5, h), point(x + a.0, y + a.1, h), point(x + b.0, y + b.1, h)],
            color,
        );
    };
    let ok = Color32::from_rgb(80, 220, 80);
    let refused = Color32::from_rgb(230, 60, 60);
    for (cell, edge) in crossing_shown(view) {
        quarter(cell, edge, Color32::from_rgb(240, 200, 60));
    }
    // A terrain drag's marked corners (the squares round them), or tiles.
    if let Some(drag) = &view.terrain_drag {
        let o = if drag.tiles { 0.0 } else { -0.5 };
        for (x, y) in drag.marked() {
            let (fx, fy) = (x as f32 + o, y as f32 + o);
            let h = z(x, y) + 0.05;
            let square = crate::area_view::tile_square(fx, fy, h);
            polygon(&square, Color32::from_rgb(240, 200, 60));
        }
    }
    let Some(s) = view.spot else { return };
    match brush.brush {
        Brush::Crosser(c) => {
            // (With Shift, the outline is the drag's.)
            if view.crossing_outline.is_none() {
                let (cell, edge) = next_quarter(&view.crossing, s);
                // Before a drag, over a quarter the crosser already crosses,
                // a click only chooses the tile again; elsewhere a drag from
                // here lays it.
                let there =
                    g.lattice.get_cell(cell.0, cell.1).is_some_and(|t| t.edges[edge] == Some(c));
                let color = if view.crossing.is_empty() && there { CYCLE } else { ok };
                quarter(cell, edge, color);
            }
        }
        Brush::Group(gi) => {
            let (st, _) = stroke(tools, &g, &brush, s, false, false, view.group_turns);
            let color = if st.is_some() { ok } else { refused };
            let group = &tools.set.groups[gi];
            for k in 0..group.tiles.len() {
                let (c, r) = mg_tiles::group_slot(k, group.columns, view.group_turns);
                let (x, y) = ((s.cell.0 as i64 + c) as f32, (s.cell.1 as i64 + r) as f32);
                let h = if x >= 0.0
                    && y >= 0.0
                    && (x as u32) < model.width
                    && (y as u32) < model.height
                {
                    z(x as u32, y as u32) + 0.05
                } else {
                    0.05
                };
                let square = crate::area_view::tile_square(x, y, h);
                polygon(&square, color);
            }
        }
        Brush::Appearance { .. } | Brush::Rotate => {
            let (x, y) = (s.cell.0 as f32, s.cell.1 as f32);
            let h = z(s.cell.0, s.cell.1) + 0.05;
            let square = crate::area_view::tile_square(x, y, h);
            let (st, _) = stroke(tools, &g, &brush, s, false, shift, 0);
            polygon(&square, if st.is_some() { CYCLE } else { refused });
        }
        Brush::Eraser | Brush::Refine => {
            let (x, y) = (s.cell.0 as f32, s.cell.1 as f32);
            let h = z(s.cell.0, s.cell.1) + 0.05;
            let square = crate::area_view::tile_square(x, y, h);
            // Refine Tile, and the Eraser with Shift, step the tile through
            // those that fit (in a drag, Shift fills a rectangle); Refine
            // Tile's target tile is refused where it does not fit.
            let cycle = shift && view.terrain_drag.is_none() || brush.brush == Brush::Refine;
            let fits = brush.target.is_none() || refined(tools, &g, &brush, s.cell).is_some();
            polygon(
                &square,
                if !fits {
                    refused
                } else if cycle {
                    CYCLE
                } else {
                    ok
                },
            );
        }
        Brush::Tile(_) => {
            let (st, _) = stroke(tools, &g, &brush, s, false, false, view.group_turns);
            let (x, y) = (s.cell.0 as f32, s.cell.1 as f32);
            let h = z(s.cell.0, s.cell.1) + 0.05;
            let square = crate::area_view::tile_square(x, y, h);
            polygon(&square, if st.is_some() { ok } else { refused });
        }
        _ => {
            // A click on a corner of the brush's own terrain adds none: it
            // chooses the tiles around it again (with Shift, the next that
            // fit).
            let own = matches!(brush.brush, Brush::Terrain(t)
                if g.lattice.get_corner(s.corner.0, s.corner.1).is_some_and(|c| c.terrain == t));
            let (st, _) = stroke(tools, &g, &brush, s, false, shift && own, 0);
            let cycle = own;
            let (x, y) = (s.corner.0 as f32, s.corner.1 as f32);
            let h = z(s.corner.0, s.corner.1) + 0.05;
            let square = [
                point(x - 0.5, y - 0.5, h),
                point(x + 0.5, y - 0.5, h),
                point(x + 0.5, y + 0.5, h),
                point(x - 0.5, y + 0.5, h),
            ];
            let color = match (st.is_some(), cycle) {
                (false, _) => refused,
                (true, true) => CYCLE,
                (true, false) => ok,
            };
            polygon(&square, color);
        }
    }
}

/// What the status line says about the spot under the pointer.
pub(crate) fn status(view: &AreaView) -> Option<String> {
    let s = view.spot?;
    let model = view.model.as_ref()?;
    view.terrain.as_ref()?;
    let tile = model.tiles.get((s.cell.1 * model.width + s.cell.0) as usize)?;
    let text =
        format!("Tile ({}, {}) {}", s.cell.0, s.cell.1, tile.model.as_deref().unwrap_or("?"),);
    Some(match &view.notice {
        Some(n) => format!("{text} · {n}"),
        None => text,
    })
}

/// The tileset palette of the area shown last, in the palette pane:
/// Features, Groups and Terrain, each brush to choose.
pub(crate) fn palette_ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    let Some(area) = app.palette.area else {
        ui.label("Open an area to paint its tiles.");
        return;
    };
    let tileset = app.area_views.get(&area).and_then(|v| v.terrain.as_ref().map(|t| t.tileset));
    crate::trace::changed("tile palette", || {
        format!(
            "area {area}, its view {}, tileset {tileset:?}",
            if app.area_views.contains_key(&area) { "open" } else { "not open" }
        )
    });
    let Some(tileset) = tileset else {
        ui.label("The area's tileset could not be read.");
        return;
    };
    let palette = match app.palette.tile_palettes.get(&tileset) {
        Some(p) => p.clone(),
        None => {
            let (Some(game), Some(view)) = (app.game.as_deref(), app.area_views.get(&area)) else {
                return;
            };
            let tools = view.terrain.as_ref().expect("checked");
            let p = Arc::new(TilesetPalette::read(game, tileset, &tools.set, &tools.index));
            app.palette.tile_palettes.insert(tileset, p.clone());
            p
        }
    };
    // The row over the list, as the blueprint palettes have it: every
    // heading opened or closed, how many there are (of the search, if
    // any), names or pictures, and the pictures' size.
    let filter = mg_core::text::Needle::new(&app.palette.filter);
    let gallery = app.settings.palette_gallery;
    let shown = {
        let Some(tools) = app.area_views.get(&area).and_then(|v| v.terrain.as_ref()) else {
            return;
        };
        count(&palette.branches, tools, &filter, gallery)
    };
    let mut fold = None;
    ui.horizontal_wrapped(|ui| {
        fold = crate::palette_view::fold_icons(ui, "heading");
        ui.weak(format!("{shown} item{}", if shown == 1 { "" } else { "s" }))
            .on_hover_text(if filter.is_empty() { "In this tileset's palette" } else { "Found" });
        ui.separator();
        if ui.selectable_label(!gallery, "☰").on_hover_text("List: the brushes by name").clicked()
        {
            app.settings.palette_gallery = false;
        }
        if ui
            .selectable_label(gallery, "⊞")
            .on_hover_text(
                "Gallery: each terrain's and crosser's tiles as pictures under it, to choose \
                 one by eye",
            )
            .clicked()
        {
            app.settings.palette_gallery = true;
        }
        // Beside them, for the Gallery's tiles (and a tile chosen from it,
        // the List shown since): an explicit switch, not a key held, as
        // Shift, Ctrl and Alt already do things with a brush (#29).
        let tile_chosen =
            app.palette.tile_brush.as_ref().is_some_and(|b| matches!(b.brush, Brush::Tile(_)));
        if gallery || tile_chosen {
            ui.toggle_value(&mut app.settings.exact_tiles, EXACT_TILE).on_hover_text(
                "Exact tile: a tile chosen from the pictures goes in alone, exactly as chosen \
                 (a right click in the area turns it), at the height of the tile it replaces. \
                 The tiles around aren't adjusted to it: no neighbor, corner terrain, height or \
                 crosser changes, so it may not meet them. Not the Features and Groups pictures, \
                 placed whole as ever (Tile Properties' All tiles puts in a piece of one). Off: \
                 the tiles around fit themselves \
                 to it",
            );
        }
        if gallery {
            // (A slider doesn't wrap of itself: on a row of its own where
            // this one has no room for it.)
            if ui.available_size_before_wrap().x < ui.spacing().slider_width {
                ui.end_row();
            }
            crate::appearance_gallery::size_slider(app, ui);
        }
    });
    ui.separator();
    let mut chosen = app.palette.tile_brush.clone();
    let mut choice = app.palette.tile_choice;
    let side = crate::appearance_gallery::tile_side(app);
    let ready = if gallery { app.thumbnails.every() } else { Default::default() };
    let Some(view) = app.area_views.get(&area) else { return };
    let was_tile_mode = view.tile_mode;
    let mut tile_mode = was_tile_mode;
    let selected = if was_tile_mode { view.tile_selection.len() } else { 0 };
    let Some(tools) = view.terrain.as_ref() else { return };
    let mut at = Place {
        tileset,
        tools,
        chosen: &mut chosen,
        choice: &mut choice,
        tile_mode: &mut tile_mode,
        selected,
        stepped: None,
        // (With its key, as the toolbar's tip has it; its name then is
        // the palette's own.)
        select_tiles: crate::icons::labelled(
            crate::icons::SELECT_TILES,
            &app.keymap.titled(SELECT_TILES, crate::keys::Cmd::SelectTiles, ui.ctx()),
        ),
        fold: fold.or(app.palette.fold),
        filter: &filter,
        gallery: gallery.then_some(Pictures { side, ready: &ready }),
        exact: app.settings.exact_tiles,
        shown: Vec::new(),
        open: None,
    };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for item in &palette.branches {
            show_item(ui, item, &mut at, 0);
        }
    });
    let (shown, open, stepped) = (std::mem::take(&mut at.shown), at.open, at.stepped);
    app.thumbnails.show(shown);
    // A tile tool clicked with tiles selected: the step on them, as the
    // tile menu's (Select Tiles and the selection staying).
    if let Some(step) = stepped
        && let Some(mut view) = app.area_views.remove(&area)
    {
        let cells = view.tile_selection.clone();
        crate::tile_select::step_tiles(app, &mut view, &cells, step);
        app.area_views.insert(area, view);
    }
    if let Some(tile) = open {
        app.actions.push(crate::Action::OpenTab(crate::Tab::Tile { tileset, tile }));
    }
    app.palette.tile_choice = choice;
    if chosen != app.palette.tile_brush {
        if chosen.is_some() {
            app.palette.selected = None;
            // (A brush chosen: out of tile mode, which it would sit on
            // top of.)
            tile_mode = false;
        }
        app.palette.tile_brush = chosen;
    }
    if tile_mode != was_tile_mode
        && let Some(view) = app.area_views.get_mut(&area)
    {
        // As the view's own Select Tiles: what was selected lets go.
        view.tile_mode = tile_mode;
        view.selection.clear();
        view.tile_selection.clear();
        if tile_mode {
            app.palette.tile_brush = None;
            app.palette.selected = None;
            app.paint = false;
        }
    }
}

/// The tiles palette's Select Tiles: the area view's tile mode (its
/// toolbar's Select Tiles, F10), on or off, beside the tools.
fn select_tiles_row(ui: &mut egui::Ui, at: &mut Place) {
    let r = ui.selectable_label(*at.tile_mode, at.select_tiles.as_str()).on_hover_text(
        "Select tiles rather than objects (the area view's Select Tiles): a click selects \
         a tile, a drag a box of them; a right click opens the tile menu",
    );
    if r.clicked() {
        *at.tile_mode = !*at.tile_mode;
        if *at.tile_mode {
            *at.chosen = None;
        }
    }
}

/// A tile or a group dragged from the tiles palette's Gallery: dropped in
/// an area of its tileset, it is placed where it is dropped.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DraggedTile {
    pub tileset: ResRef,
    /// [`Brush::Tile`] or [`Brush::Group`].
    pub brush: Brush,
    pub label: String,
}

/// A tile or a group from the tiles palette dropped in an area view:
/// placed in the tile under the pointer, turned as the brush is, as a
/// click places it.
pub(crate) fn drop_tile(
    app: &mut Moonglow,
    view: &mut AreaView,
    response: &egui::Response,
    dropped: &DraggedTile,
) {
    let (Some(model), Some(tools)) = (view.model.as_ref(), view.terrain.as_ref()) else { return };
    if tools.tileset != dropped.tileset {
        return;
    }
    let (w, h) = (model.width, model.height);
    let Some(s) = response
        .hover_pos()
        .or_else(|| response.ctx.pointer_latest_pos())
        .and_then(|p| view.ground_at(p, 0.0))
        .and_then(|p| spot(p, w, h))
    else {
        return;
    };
    let Some(g) = current_grid(app, view) else { return };
    let tools = view.terrain.as_ref().expect("checked");
    let brush = TileBrush {
        tileset: dropped.tileset,
        label: dropped.label.clone(),
        brush: dropped.brush,
        target: None,
        exact: app.settings.exact_tiles && matches!(dropped.brush, Brush::Tile(_)),
    };
    let (st, label) = stroke(tools, &g, &brush, s, false, false, view.group_turns);
    commit(app, view, g, st, &label, None);
}

/// How many entries the palette shows under `filter`: brushes, or in the
/// Gallery tiles.
fn count(items: &[PaletteItem], tools: &Tools, filter: &Needle, gallery: bool) -> usize {
    items
        .iter()
        .map(|i| match i {
            PaletteItem::Folder { items, .. } => count(items, tools, filter, gallery),
            // (A group is one entry, in the Gallery one picture.)
            item @ PaletteItem::Brush { brush: Brush::Group(_), .. } => {
                usize::from(has_found(item, tools, filter))
            }
            PaletteItem::Brush { label, brush } => {
                let tiles = brush_tiles(&tools.index, *brush);
                let found = found_tiles(tools, label, &tiles, filter);
                match (gallery && !tiles.is_empty(), filter.finds(label)) {
                    (true, _) => found.len(),
                    (false, true) => 1,
                    (false, false) => usize::from(!found.is_empty()),
                }
            }
        })
        .sum()
}

/// The tiles of a brush the search finds: all of them where the brush's
/// name has every word, else those whose model's name has the words its
/// name has not.
fn found_tiles(tools: &Tools, label: &str, tiles: &[u32], filter: &Needle) -> Vec<u32> {
    let label = mg_core::text::fold(label);
    if filter.finds_folded(&label) {
        return tiles.to_vec();
    }
    let model = |t: u32| tools.set.tiles.get(t as usize).map(|t| mg_core::text::fold(&t.model));
    let finds = |m: String| filter.finds_in(&[&label, &m]);
    tiles.iter().copied().filter(|t| model(*t).is_some_and(finds)).collect()
}

/// Whether a palette entry has something the search finds: its name, or
/// the model of a tile it makes or a group has.
fn has_found(item: &PaletteItem, tools: &Tools, filter: &Needle) -> bool {
    match item {
        PaletteItem::Folder { items, .. } => items.iter().any(|i| has_found(i, tools, filter)),
        PaletteItem::Brush { label, brush: Brush::Group(g) } => {
            filter.finds(label) || group_found(tools, *g, filter)
        }
        PaletteItem::Brush { label, brush } => {
            filter.finds(label)
                || !found_tiles(tools, label, &brush_tiles(&tools.index, *brush), filter).is_empty()
        }
    }
}

/// Whether the search finds the model of one of group `g`'s tiles (#35: a
/// tile only a group has is found by its group). Each model is searched
/// alone, not with the group's name: "Inn 1" is not found in Inn 2 by its
/// tiles' 1s. (The group stays one entry, one picture in the Gallery, and
/// no heading of its tiles: [`brush_tiles`] has none for it.)
fn group_found(tools: &Tools, g: usize, filter: &Needle) -> bool {
    let Some(group) = tools.set.groups.get(g) else { return false };
    let model = |t: u32| tools.set.tiles.get(t as usize).map(|t| t.model.as_str());
    group.tiles.iter().flatten().filter_map(|t| model(*t)).any(|m| filter.finds(m))
}

/// The tiles palette's Gallery: its pictures' size, and those made.
struct Pictures<'a> {
    side: f32,
    ready: &'a std::collections::HashMap<crate::model_view::Pictured, Option<egui::TextureId>>,
}

/// What the tiles palette's entries are drawn with, and what they change:
/// the brush chosen, the tile chosen among the pictures, the pictures in
/// sight and the tile whose view a double click asked for.
struct Place<'a> {
    tileset: ResRef,
    tools: &'a Tools,
    chosen: &'a mut Option<TileBrush>,
    choice: &'a mut Option<u32>,
    /// The area view's tile mode (the palette's Select Tiles).
    tile_mode: &'a mut bool,
    /// How many tiles are selected in it (0 out of it): with any, Next
    /// and Previous Tile Appearance and Rotate Tile step them rather than
    /// being chosen.
    selected: usize,
    /// The step one of those tools was clicked for, on the selection.
    stepped: Option<TileStep>,
    /// The Select Tiles row's name.
    select_tiles: String,
    fold: Option<bool>,
    filter: &'a Needle,
    gallery: Option<Pictures<'a>>,
    /// The Exact tile switch is on.
    exact: bool,
    shown: Vec<crate::model_view::Pictured>,
    open: Option<u32>,
}

/// The tiles a terrain or crosser brush can make: those with the terrain
/// at a corner, or the crosser on an edge (none of a group's).
pub(crate) fn brush_tiles(index: &TileIndex, brush: Brush) -> Vec<u32> {
    match brush {
        Brush::Terrain(t) => index.tiles_where(|c| c.corners.iter().any(|k| k.terrain == t)),
        Brush::Crosser(x) => index.tiles_where(|c| c.edges.contains(&Some(x))),
        _ => Vec::new(),
    }
}

fn show_item(ui: &mut egui::Ui, item: &PaletteItem, at: &mut Place, depth: usize) {
    let tileset = at.tileset;
    let searching = !at.filter.is_empty();
    if searching && !has_found(item, at.tools, at.filter) {
        return;
    }
    match item {
        PaletteItem::Folder { label, items } => {
            // (What the search finds shows, opened out.)
            let open = if searching { Some(true) } else { at.fold };
            egui::CollapsingHeader::new(label.as_str())
                .id_salt((tileset, depth, label.as_str()))
                .default_open(
                    depth == 0
                        && items
                            .iter()
                            .any(|i| matches!(i, PaletteItem::Brush { brush: Brush::Eraser, .. })),
                )
                .open(open)
                .show(ui, |ui| {
                    // The tools first (where tilesets list them varies),
                    // with Moonglow's Refine Tile beside the Eraser, then
                    // the rest in the palette's order.
                    let tools = TOOLS
                        .map(|(label, brush)| PaletteItem::Brush { label: label.into(), brush });
                    let mut items: Vec<&PaletteItem> = items.iter().collect();
                    let has_tools = items
                        .iter()
                        .any(|i| matches!(i, PaletteItem::Brush { brush: Brush::Eraser, .. }));
                    if has_tools {
                        // Select Tiles first: not a brush, the view's mode.
                        if !searching || at.filter.finds(SELECT_TILES) {
                            select_tiles_row(ui, at);
                        }
                        items.extend(tools.iter());
                    }
                    items.sort_by_key(|i| match i {
                        PaletteItem::Brush { brush: Brush::Eraser, .. } => 0,
                        PaletteItem::Brush { brush: Brush::Refine, .. } => 1,
                        PaletteItem::Brush { brush: Brush::Appearance { back: false }, .. } => 2,
                        PaletteItem::Brush { brush: Brush::Appearance { back: true }, .. } => 3,
                        PaletteItem::Brush { brush: Brush::Rotate, .. } => 4,
                        PaletteItem::Brush { brush: Brush::RaiseLower, .. } => 5,
                        _ => 6,
                    });
                    // In the Gallery, a folder's groups (its features, its
                    // buildings) are pictures, as many across as fit, as a
                    // category's blueprints are.
                    let mut groups: Vec<(&str, usize)> = Vec::new();
                    for i in items {
                        match i {
                            PaletteItem::Brush { label, brush: Brush::Group(g) }
                                if at.gallery.is_some() =>
                            {
                                if !searching || has_found(i, at.tools, at.filter) {
                                    groups.push((label.as_str(), *g));
                                }
                            }
                            _ => {
                                group_pictures(ui, &std::mem::take(&mut groups), at);
                                show_item(ui, i, at, depth + 1);
                            }
                        }
                    }
                    group_pictures(ui, &groups, at);
                });
        }
        PaletteItem::Brush { label, brush } => {
            let tiles = match at.gallery {
                Some(_) => {
                    found_tiles(at.tools, label, &brush_tiles(&at.tools.index, *brush), at.filter)
                }
                None => Vec::new(),
            };
            if tiles.is_empty() {
                brush_row(ui, label, *brush, at);
                return;
            }
            // In the Gallery, a terrain or crosser is a heading its tiles
            // open and close under, as a category of blueprints; its name
            // chooses the brush.
            let id = ui.make_persistent_id((tileset, "tile-heading", label.as_str()));
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                id,
                false,
            );
            if let Some(open) = if searching { Some(true) } else { at.fold } {
                state.set_open(open);
            }
            state
                .show_header(ui, |ui| brush_row(ui, label, *brush, at))
                .body(|ui| tile_pictures(ui, &tiles, at));
        }
    }
}

/// A brush's row: its name, chosen by a click (Refine Tile naming the
/// tile it makes, one chosen in the Gallery).
fn brush_row(ui: &mut egui::Ui, label: &str, brush: Brush, at: &mut Place) {
    let on = at.chosen.as_ref().is_some_and(|b| b.label == label && b.brush == brush);
    let target = match brush {
        Brush::Refine => *at.choice,
        _ => None,
    };
    let shown = match brush {
        Brush::Eraser => crate::icons::labelled(crate::icons::ERASER, label),
        Brush::RaiseLower => crate::icons::labelled(crate::icons::RAISE_LOWER, label),
        Brush::Refine => {
            let model = target.and_then(|t| at.tools.set.tiles.get(t as usize));
            let to = model.map(|t| format!(" → {}", t.model)).unwrap_or_default();
            crate::icons::labelled(crate::icons::REFINE, &format!("{label}{to}"))
        }
        Brush::Appearance { back: false } => {
            crate::icons::labelled(crate::icons::NEXT_APPEARANCE, label)
        }
        Brush::Appearance { back: true } => {
            crate::icons::labelled(crate::icons::PREVIOUS_APPEARANCE, label)
        }
        Brush::Rotate => crate::icons::labelled(crate::icons::ROTATE, label),
        _ => label.to_string(),
    };
    let tip = match brush {
        Brush::Refine => Some(
            "Click a tile: the next tile that fits there, in Aurora's order (each tile's turns, \
             then the next tile's: its appearance and its rotation change together, as Shift + \
             right click steps them). With a tile chosen in the Gallery: that tile, where it fits",
        ),
        Brush::Appearance { back: false } => Some(
            "Click a tile: another tile of its kind (its terrain and crossers), turned as it was \
             where that fits",
        ),
        Brush::Appearance { back: true } => Some("Click a tile: a step back through them"),
        Brush::Rotate => Some(
            "Click a tile: a quarter turn counter-clockwise (Shift: clockwise), where it still \
             fits its neighbors; its lights stay",
        ),
        _ => None,
    };
    let on_selection = *at.tile_mode && at.selected > 0;
    let mut r = ui.selectable_label(on, shown);
    if let Some(tip) = tip {
        r = match brush {
            Brush::Appearance { .. } | Brush::Rotate => r.on_hover_text(format!(
                "{tip}. With tiles selected (Select Tiles): a click steps every selected tile \
                 at once, each on its own, the selection kept; click again to step them again"
            )),
            _ => r.on_hover_text(tip),
        };
    }
    // With tiles selected, a tile tool is a button on them: the step at
    // once, nothing chosen (Select Tiles and the selection stay).
    if r.clicked()
        && on_selection
        && let Some(step) = tile_step(
            &TileBrush { tileset: at.tileset, label: String::new(), brush, target, exact: false },
            ui.input(|i| i.modifiers.shift),
        )
    {
        at.stepped = Some(step);
    } else if r.clicked() {
        *at.chosen = if on {
            None
        } else {
            Some(TileBrush {
                tileset: at.tileset,
                label: label.to_string(),
                brush,
                target,
                exact: false,
            })
        };
        // (A brush other than Refine Tile lets the tile chosen go.)
        if brush != Brush::Refine {
            *at.choice = None;
        }
    }
}

/// Pictures of a tileset's groups (`(name, index)`), as many across as
/// fit: a click chooses one (placed as a group is, a right click in the
/// area turning it), and one dragged into the area is placed where it is
/// dropped.
fn group_pictures(ui: &mut egui::Ui, groups: &[(&str, usize)], at: &mut Place) {
    let Some(pictures) = &at.gallery else { return };
    if groups.is_empty() {
        return;
    }
    let (asked, ready) = (pictures.side, pictures.ready);
    crate::appearance_gallery::picture_rows(ui, asked, |ui, side| {
        for &(label, g) in groups {
            let brush = Brush::Group(g);
            let pictured = crate::model_view::Pictured::Group(at.tileset, g as u32);
            let made = ready.get(&pictured).copied();
            let on = at.chosen.as_ref().is_some_and(|b| b.label == label && b.brush == brush);
            let (r, in_sight) = crate::widgets::picture_tile(ui, side, label, on, made);
            if in_sight {
                at.shown.push(pictured);
            }
            let choose = |at: &mut Place| {
                *at.choice = None;
                let label = label.to_string();
                *at.chosen = Some(TileBrush {
                    tileset: at.tileset,
                    label,
                    brush,
                    target: None,
                    exact: false,
                });
            };
            if r.drag_started() {
                choose(at);
                let label = label.to_string();
                r.dnd_set_drag_payload(DraggedTile { tileset: at.tileset, brush, label });
            }
            if r.clicked() {
                if on {
                    *at.chosen = None;
                } else {
                    choose(at);
                }
            }
            let size = at.tools.set.groups.get(g).map(|g| (g.columns, g.rows));
            r.on_hover_ui(|ui| {
                ui.strong(label);
                if let Some((columns, rows)) = size {
                    ui.label(format!("A group of {columns} by {rows} tiles"));
                }
                if let Some(Some(id)) = made {
                    ui.image(egui::load::SizedTexture::new(id, egui::vec2(180.0, 180.0)));
                }
                ui.weak(
                    "Click to place it (right click in the area turns it), or drag it into the area",
                );
            });
        }
    });
}

/// A terrain's or crosser's tiles as pictures over their models' names,
/// as many across as fit, as the blueprint palettes' Gallery draws them:
/// a click chooses one (it is painted as it is, and Refine Tile chosen
/// after it makes tiles that one), a double click opens its view, and one
/// dragged into the area is placed where it is dropped.
fn tile_pictures(ui: &mut egui::Ui, tiles: &[u32], at: &mut Place) {
    let Some(pictures) = &at.gallery else { return };
    let (asked, ready) = (pictures.side, pictures.ready);
    crate::appearance_gallery::picture_rows(ui, asked, |ui, side| {
        for &t in tiles {
            let Some(tile) = at.tools.set.tiles.get(t as usize) else { continue };
            let key = ResKey::parse(&tile.model.to_ascii_lowercase(), ResType::MDL);
            let pictured = key.map(|k| crate::model_view::Pictured::Tile(k, 0));
            let made = pictured.and_then(|p| ready.get(&p).copied());
            let current = *at.choice == Some(t);
            let (r, in_sight) = crate::widgets::picture_tile(ui, side, &tile.model, current, made);
            if in_sight {
                at.shown.extend(pictured);
            }
            let choose = |at: &mut Place| {
                *at.choice = Some(t);
                let (brush, label) = (Brush::Tile(t), tile.model.clone());
                *at.chosen = Some(TileBrush {
                    tileset: at.tileset,
                    label,
                    brush,
                    target: None,
                    exact: false,
                });
            };
            if r.drag_started() {
                // (Chosen too: the area shows where it would go.)
                choose(at);
                let (brush, label) = (Brush::Tile(t), tile.model.clone());
                r.dnd_set_drag_payload(DraggedTile { tileset: at.tileset, brush, label });
            }
            if r.double_clicked() {
                at.open = Some(t);
            } else if r.clicked() {
                if current {
                    *at.choice = None;
                    *at.chosen = None;
                } else {
                    choose(at);
                }
            }
            r.on_hover_ui(|ui| {
                ui.strong(&tile.model);
                ui.label(format!("Tile {t} of the tileset"));
                if let Some(Some(id)) = made {
                    let size = 180.0;
                    ui.image(egui::load::SizedTexture::new(id, egui::vec2(size, size)));
                }
                ui.weak(if at.exact {
                    "Click to place it alone (Exact tile: its neighbors are left as they are), \
                     double-click for its view, or drag it into the area"
                } else {
                    "Click to paint with it, double-click for its view, or drag it into the area"
                });
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_find_the_nearest_corner_tile_and_edge() {
        let s = spot(Vec3::new(34.0, 48.0, 0.0), 10, 10).unwrap();
        assert_eq!((s.corner, s.cell, s.edge), ((3, 5), (3, 4), mg_tiles::NORTH));
        let s = spot(Vec3::new(1.0, 5.0, 0.0), 10, 10).unwrap();
        assert_eq!((s.corner, s.cell, s.edge), ((0, 1), (0, 0), mg_tiles::WEST));
        // The far edge belongs to the last tile.
        let s = spot(Vec3::new(100.0, 99.0, 0.0), 10, 10).unwrap();
        assert_eq!((s.corner, s.cell), ((10, 10), (9, 9)));
        assert!(spot(Vec3::new(-20.0, 5.0, 0.0), 10, 10).is_none());
    }

    #[test]
    fn an_outline_rings_the_rectangle() {
        use mg_tiles::{EAST, NORTH, SOUTH, WEST};
        let ring = outline((2, 2), (0, 0));
        // Eight tiles round, each crossing marked on both sides.
        assert_eq!(ring.len(), 16);
        let has = |cell, edge| ring.contains(&(cell, edge));
        assert!(has((0, 0), EAST) && has((0, 0), NORTH));
        assert!(has((1, 0), WEST) && has((1, 0), EAST) && !has((1, 0), NORTH));
        assert!(has((2, 2), SOUTH) && has((2, 2), WEST));
        assert!(!ring.iter().any(|(c, _)| *c == (1, 1)), "nothing inside");
        // One tile high: a straight run.
        let run = outline((0, 1), (3, 1));
        assert_eq!(run.len(), 6);
        assert!(run.contains(&((0, 1), EAST)) && run.contains(&((3, 1), WEST)));
        assert!(!run.iter().any(|(_, e)| *e == NORTH || *e == SOUTH));
        // One tile: itself.
        assert_eq!(outline((1, 1), (1, 1)).len(), 1);
    }

    /// The quarters a crosser drag through `points` (metres) takes, as the
    /// view steps it.
    fn drag(points: &[(f32, f32)]) -> Vec<((u32, u32), usize)> {
        let at = |&(x, y): &(f32, f32)| Vec3::new(x, y, 0.0);
        let first = spot(at(&points[0]), 10, 10).unwrap();
        let mut path = vec![(first.cell, first.edge)];
        for w in points.windows(2) {
            let (from, to) = (at(&w[0]), at(&w[1]));
            let steps = ((to - from).length() * 4.0).ceil() as u32;
            for k in 1..=steps {
                pass(&mut path, spot(from.lerp(to, k as f32 / steps as f32), 10, 10).unwrap());
            }
        }
        path
    }

    #[test]
    fn a_crosser_drag_across_a_tile_need_not_keep_to_its_middle() {
        use mg_tiles::{EAST, NORTH, SOUTH, WEST};
        // South to north through tile (1, 1), 1.5 m and 2.2 m off its middle,
        // and wandering: no side quarter.
        for x in [16.5, 12.8] {
            assert_eq!(drag(&[(x, 10.5), (x, 19.5)]), [((1, 1), SOUTH), ((1, 1), NORTH)]);
        }
        assert_eq!(
            drag(&[(15.5, 10.5), (17.0, 13.0), (13.5, 16.0), (15.0, 19.5)]),
            [((1, 1), SOUTH), ((1, 1), NORTH)]
        );
        // West to east, the same.
        assert_eq!(drag(&[(10.5, 13.5), (19.5, 13.5)]), [((1, 1), WEST), ((1, 1), EAST)]);
        // A turn still turns: in from the south, out by the east edge.
        assert_eq!(
            drag(&[(15.0, 10.5), (15.0, 15.0), (19.5, 15.0)]),
            [((1, 1), SOUTH), ((1, 1), EAST)]
        );
        // The cursor shows the quarter the drag keeps, not a side one it
        // passes over.
        let path = [((1, 1), SOUTH)];
        let over_east = spot(Vec3::new(16.5, 15.0, 0.0), 10, 10).unwrap();
        assert_eq!(over_east.edge, EAST);
        assert_eq!(next_quarter(&path, over_east), ((1, 1), SOUTH));
        let near_east = spot(Vec3::new(19.0, 15.0, 0.0), 10, 10).unwrap();
        assert_eq!(next_quarter(&path, near_east), ((1, 1), EAST));
        assert_eq!(next_quarter(&[], over_east), ((1, 1), EAST), "before a drag");
        // Run back, a drag lets go of what it passed.
        assert_eq!(drag(&[(15.0, 10.5), (15.0, 19.5), (15.0, 10.5)]), [((1, 1), SOUTH)]);
        assert_eq!(
            drag(&[(15.0, 10.5), (15.0, 15.0), (29.5, 15.0), (21.0, 15.0)]),
            [((1, 1), SOUTH), ((1, 1), EAST), ((2, 1), WEST)]
        );
        assert_eq!(
            drag(&[(15.0, 10.5), (15.0, 15.0), (29.5, 15.0), (15.0, 15.0), (15.0, 10.5)]),
            [((1, 1), SOUTH)]
        );
        // And on into the next tile, by its west quarter.
        assert_eq!(
            drag(&[(15.0, 10.5), (15.0, 15.0), (29.5, 15.0)]),
            [((1, 1), SOUTH), ((1, 1), EAST), ((2, 1), WEST), ((2, 1), EAST)]
        );
    }
}
