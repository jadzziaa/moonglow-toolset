//! The area viewer's terrain mode: painting with the tileset palette's
//! brushes (the palette pane's Tiles, as Aurora's Terrain tab).
//!
//! - A terrain brush or Raise/Lower acts on the lattice corner nearest the
//!   pointer (the right button lowers); the cursor is the square of the four
//!   tiles around it, red where Aurora would refuse the stroke.
//! - A crosser brush is dragged: the crosser goes on the edge of every
//!   quarter of a tile the pointer passes through (the quarter nearest that
//!   edge), as Aurora draws it. A click chooses the tile again.
//! - The Eraser acts on the tile under the pointer: it removes its crossers
//!   (and those that then fit nothing), or chooses it again; with Shift it
//!   steps through the tiles that fit, in Aurora's order.
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
    pub point: Vec3,
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
    Some(Spot { corner, cell, edge, centre, point: p })
}

/// The brush that applies to `view`'s area, if one is chosen for its
/// tileset.
pub(crate) fn active(app: &Moonglow, view: &AreaView) -> Option<TileBrush> {
    let b = app.palette.tile_brush.clone()?;
    let tools = view.terrain.as_ref()?;
    (tools.tileset == b.tileset).then_some(b)
}

/// The area's grid as the workspace has it.
fn current_grid(app: &mut Moonglow, view: &AreaView) -> Option<Grid> {
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
) -> (Option<Stroke>, String) {
    let (x, y) = spot.corner;
    let label = format!("Paint {}", brush.label);
    match brush.brush {
        Brush::Terrain(t) => (grid.paint(&tools.index, &tools.rules, x, y, t), label),
        Brush::RaiseLower => {
            let what = if lower { "Lower terrain" } else { "Raise terrain" };
            (grid.raise(&tools.index, &tools.rules, x, y, !lower), what.into())
        }
        Brush::Eraser if cycle => {
            let (cx, cy) = spot.cell;
            let cell = grid.lattice.cell(cx, cy);
            let next = next_fit(&tools.index, &cell, grid.tile(cx, cy));
            let s = next.map(|_| Stroke { lattice: grid.lattice.clone(), cells: vec![spot.cell] });
            (s, "Next tile".into())
        }
        Brush::Eraser => (grid.erase(&tools.index, spot.cell.0, spot.cell.1), "Erase tile".into()),
        Brush::Crosser(c) => (grid.draw_crosser(&tools.index, &[], &[spot.cell], c), label),
        Brush::Group(_) => (None, label),
    }
}

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
        view.notice = Some(format!("{label}: no tile fits there"));
        return;
    };
    let Some(tools) = view.terrain.as_ref() else { return };
    let mut rng = fastrand::Rng::new();
    let changes = match pick {
        Some(p) => {
            let cell = stroke.cells[0];
            let w = grid.lattice.width();
            grid.tiles[(cell.1 * w + cell.0) as usize] = p;
            vec![(cell, p)]
        }
        None => grid.apply(&tools.index, stroke, &mut rng),
    };
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_ref()) else { return };
    let key = ResKey::new(view.area, ResType::ARE);
    let Ok(are) = ws.doc(&key) else { return };
    let root = are.root.clone();
    let scheme = root
        .integer("LightingScheme")
        .and_then(|row| mg_module::new::Scheme::read(game, row.max(0) as usize).ok());
    let mut lights_rng = fastrand::Rng::new();
    let mut lights = || scheme.as_ref().map_or([0; 3], |s| s.tile_lights(&mut lights_rng));
    let edits = tile_edits(key, &root, &tools.set, &changes, &mut lights);
    if !edits.is_empty() {
        app.actions.push(crate::Action::Apply(Command::new(label, edits)));
    }
    view.notice = None;
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
        return true;
    }
    match brush.brush {
        Brush::Crosser(c) => {
            let press = ui.input(|i| i.pointer.press_origin());
            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(s) = press.and_then(|p| at(view, p))
            {
                view.crossing = vec![(s.cell, s.edge)];
                view.crossing_at = Some(s.point);
            }
            if response.dragged_by(egui::PointerButton::Primary)
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
            }
            if response.drag_stopped() && !view.crossing.is_empty() {
                view.crossing_at = None;
                let mut edges = std::mem::take(&mut view.crossing);
                edges.dedup();
                if let Some(g) = current_grid(app, view) {
                    let tools = view.terrain.as_ref().expect("checked");
                    let s = if edges.len() > 1 {
                        g.draw_crosser(&tools.index, &edges, &[], c)
                    } else {
                        g.draw_crosser(&tools.index, &[], &[edges[0].0], c)
                    };
                    commit(app, view, g, s, &format!("Paint {}", brush.label), None);
                }
            } else if response.clicked()
                && let Some(s) = view.spot
                && let Some(g) = current_grid(app, view)
            {
                let tools = view.terrain.as_ref().expect("checked");
                let (st, label) = stroke(tools, &g, &brush, s, false, false);
                commit(app, view, g, st, &label, None);
            }
        }
        _ => {
            let lower = response.secondary_clicked() && brush.brush == Brush::RaiseLower;
            if (response.clicked() || lower)
                && let Some(s) = view.spot
                && let Some(g) = current_grid(app, view)
            {
                let tools = view.terrain.as_ref().expect("checked");
                let cycle = shift && brush.brush == Brush::Eraser;
                let (st, label) = stroke(tools, &g, &brush, s, lower, cycle);
                let pick = if cycle {
                    let (cx, cy) = s.cell;
                    next_fit(&tools.index, &g.lattice.cell(cx, cy), g.tile(cx, cy))
                } else {
                    None
                };
                commit(app, view, g, st, &label, pick);
            } else if response.secondary_clicked() {
                app.palette.tile_brush = None;
            }
        }
    }
    true
}

/// A crosser drag reaching spot `q`: its quarter joins the path when it is
/// a new one. Near a tile's centre, where the quarters meet, the drag stays
/// in its quarter, or enters by the edge it came across.
fn pass(path: &mut Vec<((u32, u32), usize)>, q: Spot) {
    let Some(&(last, _)) = path.last() else { return };
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
    if path.last() != Some(&quarter) {
        path.push(quarter);
    }
}

/// The brush's cursor over the view: the square of tiles around the corner
/// (red where the stroke would be refused), the tile for the Eraser, the
/// quarters a crosser drag has passed.
pub(crate) fn overlay(app: &mut Moonglow, ui: &egui::Ui, view: &AreaView) {
    let Some(brush) = active(app, view) else { return };
    let (Some(model), Some(tools)) = (view.model.as_ref(), view.terrain.as_ref()) else { return };
    let painter = ui.painter_at(view.rect);
    let step = model.height_step;
    let Some(g) = grid_of(app, view) else { return };
    let z = |x: u32, y: u32| g.lattice.corner(x, y).height as f32 * step;
    let point =
        |x: f32, y: f32, z: f32| Vec3::new(x * mg_area::TILE_SIZE, y * mg_area::TILE_SIZE, z);
    let polygon = |points: &[Vec3], color: Color32| {
        let screen: Vec<Pos2> = points.iter().filter_map(|p| view.screen_pos(*p)).collect();
        if screen.len() == points.len() {
            painter.add(egui::Shape::closed_line(screen, Line::new(2.0, color)));
        }
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
    for &(cell, edge) in &view.crossing {
        quarter(cell, edge, Color32::from_rgb(240, 200, 60));
    }
    let Some(s) = view.spot else { return };
    match brush.brush {
        Brush::Crosser(_) => quarter(s.cell, s.edge, ok),
        Brush::Eraser => {
            let (x, y) = (s.cell.0 as f32, s.cell.1 as f32);
            let h = z(s.cell.0, s.cell.1) + 0.05;
            let square = [
                point(x, y, h),
                point(x + 1.0, y, h),
                point(x + 1.0, y + 1.0, h),
                point(x, y + 1.0, h),
            ];
            polygon(&square, ok);
        }
        _ => {
            let (st, _) = stroke(tools, &g, &brush, s, false, false);
            let (x, y) = (s.corner.0 as f32, s.corner.1 as f32);
            let h = z(s.corner.0, s.corner.1) + 0.05;
            let square = [
                point(x - 0.5, y - 0.5, h),
                point(x + 0.5, y - 0.5, h),
                point(x + 0.5, y + 0.5, h),
                point(x - 0.5, y + 0.5, h),
            ];
            polygon(&square, if st.is_some() { ok } else { refused });
        }
    }
}

/// The grid for drawing (without a workspace borrow beyond the read).
fn grid_of(app: &mut Moonglow, view: &AreaView) -> Option<Grid> {
    current_grid(app, view)
}

/// What the status line says about the spot under the pointer.
pub(crate) fn status(view: &AreaView) -> Option<String> {
    let s = view.spot?;
    let model = view.model.as_ref()?;
    let tools = view.terrain.as_ref()?;
    let tile = model.tiles.get((s.cell.1 * model.width + s.cell.0) as usize)?;
    let text =
        format!("Tile ({}, {}) {}", s.cell.0, s.cell.1, tile.model.as_deref().unwrap_or("?"),);
    let _ = tools;
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
    let Some(tileset) = tileset else {
        ui.label("The area's tileset could not be read.");
        return;
    };
    let palette = match app.palette.tile_palettes.get(&tileset) {
        Some(p) => p.clone(),
        None => {
            let (Some(game), Some(view)) = (app.game.as_ref(), app.area_views.get(&area)) else {
                return;
            };
            let tools = view.terrain.as_ref().expect("checked");
            let p = Arc::new(TilesetPalette::read(game, tileset, &tools.set, &tools.index));
            app.palette.tile_palettes.insert(tileset, p.clone());
            p
        }
    };
    let mut chosen = app.palette.tile_brush.clone();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for item in &palette.branches {
            show_item(ui, item, tileset, &mut chosen, 0);
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
                .show(ui, |ui| {
                    for i in items {
                        show_item(ui, i, tileset, chosen, depth + 1);
                    }
                });
        }
        PaletteItem::Brush { label, brush } => {
            let on = chosen.as_ref().is_some_and(|b| b.label == *label && b.brush == *brush);
            if ui.selectable_label(on, label.as_str()).clicked() {
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
}
