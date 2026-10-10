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
//! Every stroke is one undoable command on the ARE (`mg_tiles::paint`,
//! `mg_area::terrain`).

use std::sync::Arc;

use egui::{Color32, Pos2, Stroke as Line};
use glam::Vec3;
use mg_area::terrain::{Brush, PaletteItem, TilesetPalette, grid, tile_edits};
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_resman::ResKey;
use mg_set::Tileset;
use mg_tiles::paint::{Grid, Rules, Stroke, next_fit};
use mg_tiles::{Placement, TileIndex};

use crate::Moonglow;
use crate::area_view::AreaView;

/// The tileset brush chosen in the palette.
#[derive(Debug, Clone, PartialEq)]
pub struct TileBrush {
    pub tileset: ResRef,
    pub label: String,
    pub brush: Brush,
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

/// Refine Tile's name in the palette.
const REFINE: &str = "Refine Tile";

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
    let b = app.palette.tile_brush.clone()?;
    let tools = view.terrain.as_ref()?;
    (tools.tileset == b.tileset).then_some(b)
}

/// The area's grid as the workspace has it.
pub(crate) fn current_grid(app: &mut Moonglow, view: &AreaView) -> Option<Grid> {
    let tools = view.terrain.as_ref()?;
    let ws = app.ws.as_mut()?;
    let are = ws.doc(&ResKey::new(view.area, ResType::ARE)).ok()?;
    grid(&are.root, &tools.index)
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
                if tools.index.is_grouped(grid.tile(cx, cy).tile) {
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
        Brush::Refine => (next_tile(tools, grid, spot), "Refine tile".into()),
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
/// there, which the commit picks); `None` when no other fits.
fn next_tile(tools: &Tools, grid: &Grid, spot: Spot) -> Option<Stroke> {
    let (cx, cy) = spot.cell;
    let cell = grid.lattice.cell(cx, cy);
    next_fit(&tools.index, &cell, grid.tile(cx, cy)).map(|_| Stroke {
        lattice: grid.lattice.clone(),
        cells: vec![spot.cell],
        fixed: Vec::new(),
    })
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
    let Some(stroke) = stroke else {
        // The tiles in the way flash red, as in Aurora.
        let blocked = grid.blocked();
        view.notice = Some(if blocked.is_empty() {
            format!("{label}: no tile fits there")
        } else {
            format!("{label}: no tile fits the tiles shown in red")
        });
        view.refused = Some((blocked, std::time::Instant::now()));
        return;
    };
    let Some(tools) = view.terrain.as_ref() else { return };
    let before = grid.clone();
    // The tiles the preview showed; the next preview chooses anew.
    let changes = changes(tools, &mut grid, stroke, pick, view.preview_seed);
    view.preview_seed = fastrand::u64(..);
    tile_command(app, view, &before, &changes, label);
    view.notice = None;
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
            let cell = stroke.cells[0];
            let w = grid.lattice.width();
            grid.tiles[(cell.1 * w + cell.0) as usize] = p;
            vec![(cell, p)]
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
    let (st, label) = stroke(tools, g, brush, s, lower, cycle, turns);
    let pick = if cycle && matches!(brush.brush, Brush::Eraser | Brush::Refine) {
        let (cx, cy) = s.cell;
        next_fit(&tools.index, &g.lattice.cell(cx, cy), g.tile(cx, cy))
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
}

fn work_out_preview(app: &mut Moonglow, view: &AreaView, shift: bool) -> Option<Preview> {
    let brush = active(app, view)?;
    let mut g = current_grid(app, view)?;
    let (model, tools) = (view.model.as_ref()?, view.terrain.as_ref()?);
    let seed = view.preview_seed;
    let changed = if let Some(drag) = &view.terrain_drag {
        // A drag: what letting go now would paint.
        let size = (model.width, model.height);
        paint_corners(tools, &mut g, &brush, drag, size, seed).0
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
/// area, with fresh lights, and the doors the tiles bring and take.
pub(crate) fn tile_command(
    app: &mut Moonglow,
    view: &AreaView,
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
    view: &AreaView,
    before: &Grid,
    changes: &[((u32, u32), Placement)],
    label: &str,
    kept: &[((u32, u32), mg_gff::Struct)],
) {
    let (Some(ws), Some(game), Some(tools)) =
        (app.ws.as_mut(), app.game.as_deref(), view.terrain.as_ref())
    else {
        return;
    };
    let key = ResKey::new(view.area, ResType::ARE);
    let Ok(are) = ws.doc(&key) else { return };
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
    if !edits.is_empty() {
        app.actions.push(crate::Action::Apply(Command::new(label, edits)));
    }
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
                && g.lattice.cell(s.cell.0, s.cell.1).edges[s.edge] == Some(c)
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
        Brush::Terrain(_) | Brush::RaiseLower | Brush::Eraser
            if terrain_drag(app, view, response, &brush) => {}
        _ => {
            let lower = response.secondary_clicked() && brush.brush == Brush::RaiseLower;
            if (response.clicked() || lower)
                && let Some(s) = view.spot
                && let Some(g) = current_grid(app, view)
            {
                let tools = view.terrain.as_ref().expect("checked");
                let (st, label, pick) = click(tools, &g, &brush, s, shift, lower, 0);
                commit(app, view, g, st, &label, pick);
            } else if response.secondary_clicked() && brush.brush != Brush::RaiseLower {
                // (Raise/Lower's right click lowers: where it finds no
                // ground to lower, a cliff face or past the area's edge,
                // it does nothing. Dropped there, the next click was the
                // object's under the pointer: its menu, or selected.)
                crate::trace::note(format!("tile brush {} dropped by a right click", brush.label));
                app.palette.tile_brush = None;
            }
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
        let tiles = brush.brush == Brush::Eraser;
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
    let (changes, label, refused) =
        paint_corners(tools, &mut g, brush, &drag, (w, h), view.preview_seed);
    view.preview_seed = fastrand::u64(..);
    tile_command(app, view, &before, &changes, &label);
    let what = if drag.tiles { "tiles" } else { "corners" };
    view.notice = (refused > 0).then(|| format!("{label}: no tile fits at {refused} {what}"));
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
) -> (Changes, String, usize) {
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
        let (st, l) = stroke(tools, g, brush, s, false, false, 0);
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
    let z = |x: u32, y: u32| g.lattice.corner(x, y).height as f32 * step;
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
            let square = [
                point(fx, fy, h),
                point(fx + 1.0, fy, h),
                point(fx + 1.0, fy + 1.0, h),
                point(fx, fy + 1.0, h),
            ];
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
                let there = g.lattice.cell(cell.0, cell.1).edges[edge] == Some(c);
                let color = if view.crossing.is_empty() && there { CYCLE } else { ok };
                quarter(cell, edge, color);
            }
        }
        Brush::Group(gi) => {
            let (st, _) = stroke(tools, &g, &brush, s, false, false, view.group_turns);
            let color = if st.is_some() { ok } else { refused };
            let group = &tools.set.groups[gi];
            let columns = group.columns.max(1) as i64;
            for k in 0..group.tiles.len() as i64 {
                let (mut c, mut r) = (k % columns, k / columns);
                for _ in 0..view.group_turns {
                    (c, r) = (-r, c);
                }
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
                let square = [
                    point(x, y, h),
                    point(x + 1.0, y, h),
                    point(x + 1.0, y + 1.0, h),
                    point(x, y + 1.0, h),
                ];
                polygon(&square, color);
            }
        }
        Brush::Eraser | Brush::Refine => {
            let (x, y) = (s.cell.0 as f32, s.cell.1 as f32);
            let h = z(s.cell.0, s.cell.1) + 0.05;
            let square = [
                point(x, y, h),
                point(x + 1.0, y, h),
                point(x + 1.0, y + 1.0, h),
                point(x, y + 1.0, h),
            ];
            // Refine Tile, and the Eraser with Shift, step the tile through
            // those that fit (in a drag, Shift fills a rectangle).
            let cycle = shift && view.terrain_drag.is_none() || brush.brush == Brush::Refine;
            polygon(&square, if cycle { CYCLE } else { ok });
        }
        _ => {
            // A click on a corner of the brush's own terrain adds none: it
            // chooses the tiles around it again (with Shift, the next that
            // fit).
            let own = matches!(brush.brush, Brush::Terrain(t)
                if g.lattice.corner(s.corner.0, s.corner.1).terrain == t);
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
    let mut chosen = app.palette.tile_brush.clone();
    let fold = app.palette.fold;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for item in &palette.branches {
            show_item(ui, item, tileset, &mut chosen, 0, fold);
        }
    });
    if chosen != app.palette.tile_brush {
        if chosen.is_some() {
            app.palette.selected = None;
        }
        app.palette.tile_brush = chosen;
    }
}

fn show_item(
    ui: &mut egui::Ui,
    item: &PaletteItem,
    tileset: ResRef,
    chosen: &mut Option<TileBrush>,
    depth: usize,
    fold: Option<bool>,
) {
    match item {
        PaletteItem::Folder { label, items } => {
            egui::CollapsingHeader::new(label.as_str())
                .id_salt((tileset, depth, label.as_str()))
                .default_open(
                    depth == 0
                        && items
                            .iter()
                            .any(|i| matches!(i, PaletteItem::Brush { brush: Brush::Eraser, .. })),
                )
                .open(fold)
                .show(ui, |ui| {
                    // The tools first (where tilesets list them varies),
                    // with Moonglow's Refine Tile beside the Eraser, then
                    // the rest in the palette's order.
                    let refine = PaletteItem::Brush { label: REFINE.into(), brush: Brush::Refine };
                    let mut items: Vec<&PaletteItem> = items.iter().collect();
                    if items
                        .iter()
                        .any(|i| matches!(i, PaletteItem::Brush { brush: Brush::Eraser, .. }))
                    {
                        items.push(&refine);
                    }
                    items.sort_by_key(|i| match i {
                        PaletteItem::Brush { brush: Brush::Eraser, .. } => 0,
                        PaletteItem::Brush { brush: Brush::Refine, .. } => 1,
                        PaletteItem::Brush { brush: Brush::RaiseLower, .. } => 2,
                        _ => 3,
                    });
                    for i in items {
                        show_item(ui, i, tileset, chosen, depth + 1, fold);
                    }
                });
        }
        PaletteItem::Brush { label, brush } => {
            let on = chosen.as_ref().is_some_and(|b| b.label == *label && b.brush == *brush);
            let shown = match brush {
                Brush::Eraser => crate::icons::labelled(crate::icons::ERASER, label),
                Brush::RaiseLower => crate::icons::labelled(crate::icons::RAISE_LOWER, label),
                Brush::Refine => crate::icons::labelled(crate::icons::REFINE, label),
                _ => label.clone(),
            };
            if ui.selectable_label(on, shown).clicked() {
                *chosen = if on {
                    None
                } else {
                    Some(TileBrush { tileset, label: label.clone(), brush: *brush })
                };
            }
        }
    }
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
