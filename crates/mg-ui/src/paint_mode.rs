//! The area view's placeable brush (Paint, on its toolbar): placeables
//! chosen in the palette (Ctrl + click for several) are painted onto the
//! ground, as builders asked for trees, rocks and bushes.
//!
//! - Scatter: a circle on the ground (Shift + wheel sizes it). A press lays
//!   a dab, and so does a drag every so far, or the button held still every
//!   so often: each dab adds a share (Flow) of what the circle lacks of its
//!   Density, at random places on the walkmesh no nearer than Spacing to
//!   another placeable (between their middles, or with By Size between
//!   their sides, each by its model's box), a random pick of the chosen
//!   blueprints, each turned as Q and E turned the brush give or take
//!   Turn, scaled at random between Scale's two. The placeables of the
//!   chosen blueprints already in the circle count (every placeable there,
//!   with Count All), so going over a painted place again fills it up and
//!   no more. Before a press the next dab shows, see-through.
//! - Row: a drag lays pieces end to end from where it began to the pointer,
//!   each facing along the row (Q and E turn them on it), one after the
//!   other by its model's length along the row, and Gap. A drag begun at
//!   the last row's end (its ring) goes on from it: a row of several legs,
//!   round corners.
//! - Fill: a drag fills the rectangle from where it began to the pointer
//!   with rows of pieces (a floor), on a grid Q and E turn, each piece
//!   facing along it: in a row end to end by each one's length and Gap,
//!   their middles in a line, the next row past the deepest of the one
//!   before.
//! - Erase: the circle takes the chosen blueprints' placeables out of the
//!   area as it is dragged over them (not locked ones).
//!
//! What a stroke puts down shows see-through until the button is let go,
//! and goes into the area then as one command (locked, with Lock): one
//! Undo takes it away. What an eraser's stroke takes out is marked until
//! then.

use std::collections::HashMap;

use egui::{Color32, Stroke};
use glam::{Vec2, Vec3};
use mg_area::{AreaObject, ObjectKind};
use mg_core::ResType;
use mg_edit::Command;
use mg_resman::ResKey;

use crate::area_view::AreaView;
use crate::settings::PaintSettings;
use crate::{Action, Moonglow};

/// A placeable the brush puts down (or would): of which blueprint, where,
/// turned how far from as its blueprint stands (radians, anticlockwise),
/// and its scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Painted {
    pub key: ResKey,
    pub position: Vec3,
    pub turn: f32,
    pub scale: f32,
}

/// An area view's brush: the stroke going on and what it shows.
#[derive(Debug, Default)]
pub struct PaintState {
    /// What the stroke going on has put down: into the area when the
    /// button is let go.
    pub(crate) pending: Vec<Painted>,
    /// What an eraser's stroke takes out: places in `Placeable List`, out
    /// of the area when the button is let go.
    pub erasing: Vec<usize>,
    /// The button is down on the view, painting.
    pub(crate) painting: bool,
    /// Where and when (egui's time) the stroke's last dab was laid.
    last: Option<(Vec3, f64)>,
    /// Chooses the next dab, the one shown before it is laid.
    seed: u64,
    /// Where the row being laid begins.
    row_from: Option<Vec3>,
    /// Where the last row laid ends: a drag begun there goes on from it.
    pub row_end: Option<Vec3>,
    /// Each blueprint as an unplaced object (`None`: it can't be read).
    templates: HashMap<ResKey, Option<AreaObject>>,
    /// Each blueprint's model's box (its own space, unturned): how long a
    /// piece of a row is.
    bounds: HashMap<ResKey, (Vec3, Vec3)>,
    /// What the last frame showed see-through: the stroke's placeables
    /// and the next dab.
    pub shown: usize,
}

/// The circle's color, a dab's green.
const CIRCLE: Color32 = Color32::from_rgb(110, 230, 110);

/// The eraser's circle, and the marks on what it takes out.
const ERASER: Color32 = Color32::from_rgb(240, 90, 80);

/// How near the last row's end a drag must begin to go on from it,
/// meters.
const GO_ON: f32 = 1.0;

/// How far the pointer goes on the ground, as a share of the radius,
/// between a drag's dabs.
const DAB_STEP: f32 = 0.4;

/// How often a button held still lays a dab, in seconds.
const DAB_EVERY: f64 = 0.2;

/// How many places a dab tries for each placeable it lays, before it
/// gives up on the rest (the circle is as full as Spacing lets it be).
const TRIES: usize = 30;

/// The radius's bounds, in centimeters.
const RADIUS: (u16, u16) = (25, 5000);

/// The least a row steps from one piece to the next, in meters (a piece
/// with no length, or a gap that overlaps it all).
const ROW_STEP: f32 = 0.05;

/// The most pieces a row lays.
const ROW_MAX: usize = 500;

/// The most pieces a fill lays.
const FILL_MAX: usize = 2500;

/// How near a fill's last piece may end to the rectangle's side and be
/// taken as reaching it, meters (a turned grid's sides are a rounding
/// error off).
const EDGE: f32 = 1e-3;

/// The placeables chosen in the palette to paint with: the one clicked and
/// those chosen with Ctrl + click.
pub(crate) fn keys(app: &Moonglow) -> Vec<ResKey> {
    let p = &app.palette;
    if p.tiles || p.prefabs {
        return Vec::new();
    }
    let mut keys: Vec<ResKey> = p.selected.into_iter().chain(p.chosen.iter().copied()).collect();
    keys.retain(|k| k.restype == ResType::UTP);
    keys.dedup();
    keys
}

/// The blueprints the brush paints with, when Paint is on and placeables
/// are chosen.
pub(crate) fn active(app: &Moonglow) -> Option<Vec<ResKey>> {
    let keys = keys(app);
    (app.paint && !keys.is_empty()).then_some(keys)
}

/// A placeable near a dab, which it keeps its distance from: where it
/// stands, whether it is of the brush's blueprints (counted towards
/// Density), and how far its sides reach from its middle (its size, for
/// Spacing by size).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Near {
    pub at: Vec2,
    pub ours: bool,
    pub reach: f32,
}

/// How far the sides of a model of box `(min, max)` (its own space)
/// reach from its middle seen from above, at `scale`: the farthest of
/// its sides, so that two kept that far apart don't overlap whichever
/// way they face.
pub(crate) fn reach_of((min, max): (Vec3, Vec3), scale: f32) -> f32 {
    [min.x, max.x, min.y, max.y].into_iter().map(f32::abs).fold(0.0, f32::max) * scale
}

/// The ground's height at a point (`None`: none there).
pub(crate) type Ground<'a> = &'a dyn Fn(Vec2) -> Option<f32>;

/// How far a blueprint's sides reach from its middle, at scale 1.
pub(crate) type Size<'a> = &'a dyn Fn(ResKey) -> f32;

/// What one dab at `center` lays, by `seed`: Flow's share of what the
/// circle lacks of its Density (counting `around`'s placeables of the
/// brush's blueprints, or all of them with Count All), each no nearer
/// than Spacing to any of `around` or to another of the dab's (between
/// their sides, by `size`, with Spacing by size: how far a blueprint's
/// sides reach at scale 1). `ground` is the walkmesh's height at a point
/// (`None` off it: nothing is laid there); `turn` the brush's facing,
/// `snap` the angle facings snap to (degrees).
#[allow(clippy::too_many_arguments)]
pub(crate) fn dab(
    s: &PaintSettings,
    center: Vec3,
    keys: &[ResKey],
    around: &[Near],
    turn: f32,
    snap: Option<f32>,
    seed: u64,
    (ground, size): (Ground<'_>, Size<'_>),
) -> Vec<Painted> {
    if keys.is_empty() {
        return Vec::new();
    }
    let radius = f32::from(s.radius_cm) / 100.0;
    let spacing = f32::from(s.spacing_cm) / 100.0;
    let middle = center.truncate();
    let inside = |p: Vec2| p.distance(middle) <= radius;
    let wanted = f32::from(s.density) * std::f32::consts::PI * radius * radius / 100.0;
    let have = around.iter().filter(|n| (n.ours || s.count_all) && inside(n.at)).count();
    let lacking = (wanted.round() as usize).saturating_sub(have);
    if lacking == 0 {
        return Vec::new();
    }
    let flow = f32::from(s.flow.clamp(1, 100)) / 100.0;
    let count = ((lacking as f32 * flow).ceil() as usize).clamp(1, lacking);
    let mut rng = fastrand::Rng::with_seed(seed);
    // (Each one's middle, and how far its sides reach: none, unless by size.)
    let sized = |reach: f32| if s.by_size { reach } else { 0.0 };
    let mut taken: Vec<(Vec2, f32)> = around
        .iter()
        .filter(|n| n.at.distance(middle) <= radius + spacing + sized(n.reach))
        .map(|n| (n.at, sized(n.reach)))
        .collect();
    let spread = f32::from(s.turn_spread.min(360)).to_radians();
    let (low, high) = (s.scale_min.min(s.scale_max), s.scale_max.max(s.scale_min));
    let lift = f32::from(s.lift_cm) / 100.0;
    let mut out = Vec::new();
    for _ in 0..count * TRIES {
        if out.len() == count {
            break;
        }
        // Evenly over the circle.
        let (r, a) = (radius * rng.f32().sqrt(), rng.f32() * std::f32::consts::TAU);
        let p = middle + Vec2::new(a.cos(), a.sin()) * r;
        let key = keys[rng.usize(..keys.len())];
        let scale = f32::from(low + rng.u16(..=high - low)) / 100.0;
        let reach = sized(size(key) * scale);
        if taken.iter().any(|(q, rq)| q.distance(p) < spacing + rq + reach) {
            continue;
        }
        let Some(z) = ground(p) else { continue };
        let turn = mg_area::arrange::snap_rotation(turn + (rng.f32() - 0.5) * spread, snap);
        taken.push((p, reach));
        out.push(Painted { key, position: p.extend(z + lift), turn, scale });
    }
    out
}

/// A blueprint as a row's piece: which, how it stands unturned (its
/// object's rotation), and its model's box.
pub(crate) type Piece = (ResKey, f32, (Vec3, Vec3));

/// What a row from `from` to `to` lays, by `seed`: pieces picked from
/// `pieces`, each facing along the row (snapped to `snap` degrees) and
/// turned on it by `turn`, end to end by their boxes' length along it with
/// `gap` between (an overlap, less than 0), the first one's back at `from`,
/// until the row reaches `to` (one piece, a row with no length); and where
/// it ends (past its last piece and `gap`: where a row going on from it
/// begins). `ground` is the height at a point, `None` where there is none
/// (`from`'s, then).
#[allow(clippy::too_many_arguments)]
pub(crate) fn row(
    from: Vec3,
    to: Vec3,
    pieces: &[Piece],
    turn: f32,
    snap: Option<f32>,
    (gap, lift): (f32, f32),
    seed: u64,
    ground: &dyn Fn(Vec2) -> Option<f32>,
) -> (Vec<Painted>, Vec3) {
    if pieces.is_empty() {
        return (Vec::new(), from);
    }
    let along = (to - from).truncate();
    let length = along.length();
    let angle = if length < 0.1 {
        0.0
    } else {
        mg_area::arrange::snap_rotation(along.y.atan2(along.x), snap)
    };
    let d = Vec2::new(angle.cos(), angle.sin());
    // The row's way, in a piece's own space: turned back by `turn`.
    let (c, s) = (turn.cos(), turn.sin());
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut at = 0.0;
    let mut out = Vec::new();
    loop {
        let (key, rotation, (min, max)) = pieces[rng.usize(..pieces.len())];
        let ends = [(min.x, min.y), (min.x, max.y), (max.x, min.y), (max.x, max.y)]
            .map(|(x, y)| x * c - y * s);
        let lo = ends.into_iter().fold(f32::MAX, f32::min);
        let hi = ends.into_iter().fold(f32::MIN, f32::max);
        let p = from.truncate() + d * (at - lo);
        let z = ground(p).unwrap_or(from.z) + lift;
        out.push(Painted { key, position: p.extend(z), turn: angle + turn - rotation, scale: 1.0 });
        at += (hi - lo + gap).max(ROW_STEP);
        if at >= length || out.len() >= ROW_MAX {
            let end = from.truncate() + d * at;
            return (out, end.extend(ground(end).unwrap_or(from.z)));
        }
    }
}

/// What a fill of the rectangle from `from` to `to` lays, by `seed`: a
/// grid turned by `turn` (snapped to `snap` degrees), the rectangle's
/// sides along it, and in it rows of pieces picked from `pieces`, each
/// facing along the grid. A row lays its pieces end to end, each by its
/// own length and `gap` (an overlap, less than 0), their middles across
/// it in a line (pieces of different depths lined up by their middles,
/// not by their near sides); the next row begins past the deepest piece
/// of the one before and `gap`. The first piece's corner is at `from`
/// (the deepest's, in the first row), and rows go on until the rectangle
/// is covered (one piece, a press without a drag). `ground` is the height
/// at a point, `None` where there is none (`from`'s, then).
#[allow(clippy::too_many_arguments)]
pub(crate) fn fill(
    from: Vec3,
    to: Vec3,
    pieces: &[Piece],
    turn: f32,
    snap: Option<f32>,
    (gap, lift): (f32, f32),
    seed: u64,
    ground: &dyn Fn(Vec2) -> Option<f32>,
) -> Vec<Painted> {
    if pieces.is_empty() {
        return Vec::new();
    }
    let facing = mg_area::arrange::snap_rotation(turn, snap);
    // The grid's two ways: along a row (each piece's own X, as it faces
    // along the grid) and across the rows (its Y).
    let (along, across) = grid_axes(facing);
    let span = (to - from).truncate();
    let (length, depth) = (span.dot(along), span.dot(across));
    // The way the drag went, along each.
    let sign = |d: f32| if d < 0.0 { -1.0 } else { 1.0 };
    let (way_along, way_across) = (sign(length), sign(depth));
    // Where a piece's middle goes for its near side to be `at` from the
    // start, going `way`: its box from `lo` to `hi` that way.
    let place = |at: f32, way: f32, lo: f32, hi: f32| if way > 0.0 { at - lo } else { -at - hi };
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut out = Vec::new();
    let mut row_at = 0.0;
    'rows: loop {
        // (A row's pieces first, then where they go across it: by the
        // deepest of them.)
        let mut row = Vec::new();
        let mut at = 0.0;
        loop {
            if out.len() + row.len() >= FILL_MAX {
                break;
            }
            let piece @ (_, _, (min, max)) = pieces[rng.usize(..pieces.len())];
            row.push((piece, place(at, way_along, min.x, max.x)));
            at += (max.x - min.x + gap).max(ROW_STEP);
            if at >= length.abs() - EDGE {
                break;
            }
        }
        let deepest = row.iter().map(|((_, _, (min, max)), _)| max.y - min.y).fold(0.0, f32::max);
        // The row's middle line, from the start across: each piece's box's
        // middle on it.
        let middle = way_across * (row_at + deepest / 2.0);
        for ((key, rotation, (min, max)), u) in row {
            let v = middle - (min.y + max.y) / 2.0;
            let p = from.truncate() + along * u + across * v;
            let z = ground(p).unwrap_or(from.z) + lift;
            out.push(Painted { key, position: p.extend(z), turn: facing - rotation, scale: 1.0 });
        }
        row_at += (deepest + gap).max(ROW_STEP);
        if row_at >= depth.abs() - EDGE || out.len() >= FILL_MAX {
            break 'rows;
        }
    }
    out
}

/// A fill's grid turned `facing` (radians, anticlockwise from east): the
/// way along its rows and the way across them.
fn grid_axes(facing: f32) -> (Vec2, Vec2) {
    let along = Vec2::new(facing.cos(), facing.sin());
    (along, along.perp())
}

/// The placeables near `center` (within `reach` of it, and of their
/// sides) a dab must keep its distance from, the stroke's among them.
fn around(view: &AreaView, keys: &[ResKey], center: Vec3, reach: f32) -> Vec<Near> {
    let middle = center.truncate();
    let ours = |o: &AreaObject| o.template.is_some_and(|t| keys.iter().any(|k| k.resref == t));
    let objects = view.model.iter().flat_map(|m| m.objects.iter().enumerate());
    let placed = objects
        .filter(|(_, o)| o.kind == ObjectKind::Placeable && view.show[o.kind.index()])
        .map(|(i, o)| {
            let scale = o.visual.map_or(1.0, |v| v.scale.x);
            let size = view.object_bounds(i).map_or(0.0, |b| reach_of(b, scale));
            Near { at: o.position.truncate(), ours: ours(o), reach: size }
        });
    let unknown = (Vec3::ZERO, Vec3::ZERO);
    let pending = view.paint.pending.iter().map(|p| {
        let size = reach_of(view.paint.bounds.get(&p.key).copied().unwrap_or(unknown), p.scale);
        Near { at: p.position.truncate(), ours: true, reach: size }
    });
    placed.chain(pending).filter(|n| n.at.distance(middle) <= reach + n.reach).collect()
}

/// How far the sides of `key`'s model reach from its middle, at scale 1
/// (as read for the brush; none, before it is).
fn size_of(view: &AreaView, key: ResKey) -> f32 {
    view.paint.bounds.get(&key).map_or(0.0, |b| reach_of(*b, 1.0))
}

/// The ground's height at `p` near `center`'s: on the walkmesh's walkable
/// faces (not on a roof, in water, inside a wall), `None` off them; the
/// plane at `center` where the area has no walkmesh read.
fn ground(view: &AreaView, center: Vec3) -> impl Fn(Vec2) -> Option<f32> + '_ {
    move |p| match view.ground() {
        Some(g) => g.height_on(p, center.z, |m| view.walked_on(m)),
        None => Some(center.z),
    }
}

/// The dab `view`'s brush would lay at `center` now, by its seed.
fn next_dab(app: &Moonglow, view: &AreaView, keys: &[ResKey], center: Vec3) -> Vec<Painted> {
    let s = &app.settings.paint;
    let mut reach = f32::from(s.radius_cm) / 100.0 + f32::from(s.spacing_cm) / 100.0;
    if s.by_size {
        // (As far again as the largest of the brush's own reaches.)
        let scale = f32::from(s.scale_min.max(s.scale_max)) / 100.0;
        reach += keys.iter().map(|k| size_of(view, *k) * scale).fold(0.0, f32::max);
    }
    let around = around(view, keys, center, reach);
    let ground = ground(view, center);
    let size = |k: ResKey| size_of(view, k);
    let (turn, snap) = (view.ghost_turn(), view.snap_angle());
    dab(s, center, keys, &around, turn, snap, view.paint.seed, (&ground, &size))
}

/// The pieces of a row the brush lays with `keys` (those it has read).
fn pieces(view: &AreaView, keys: &[ResKey]) -> Vec<Piece> {
    let unknown = (Vec3::splat(-0.5), Vec3::splat(0.5));
    keys.iter()
        .filter_map(|k| {
            let template = view.paint.templates.get(k)?.as_ref()?;
            let bounds = view.paint.bounds.get(k).copied().unwrap_or(unknown);
            Some((*k, template.rotation, bounds))
        })
        .collect()
}

/// The row or fill `view`'s brush lays from where it began (or `to`,
/// before a press) to `to`; and where a row ends.
fn row_to(app: &Moonglow, view: &AreaView, keys: &[ResKey], to: Vec3) -> (Vec<Painted>, Vec3) {
    let s = &app.settings.paint;
    let from = view.paint.row_from.unwrap_or_else(|| start_at(view, to, !s.fill));
    // On the ground, walkable or not (a floor over water), else level.
    let ground = |p: Vec2| view.ground().and_then(|g| g.height(p, from.z));
    let space = (f32::from(s.gap_cm) / 100.0, f32::from(s.lift_cm) / 100.0);
    let pieces = pieces(view, keys);
    let (turn, snap) = (view.ghost_turn(), view.snap_angle());
    let seed = view.paint.seed;
    if s.fill {
        (fill(from, to, &pieces, turn, snap, space, seed, &ground), to)
    } else {
        row(from, to, &pieces, turn, snap, space, seed, &ground)
    }
}

/// Where a row or a fill pressed at `at` begins: at the last row's end,
/// a Row pressed near it (it goes on from there), else where the press
/// snaps to.
fn start_at(view: &AreaView, at: Vec3, row: bool) -> Vec3 {
    match view.paint.row_end {
        Some(end) if row && end.truncate().distance(at.truncate()) <= GO_ON => end,
        _ => view.snapped(at),
    }
}

/// The ground under the pointer, if it is over the view (not over a
/// window in front of it).
fn pointer(ui: &egui::Ui, view: &AreaView) -> Option<Vec3> {
    // (The latest position: a stroke goes on to where the pointer was
    // last.)
    let pos = ui.ctx().pointer_latest_pos().filter(|p| view.is_over(ui, *p))?;
    view.ground_at(pos, 0.0)
}

/// `p` as its blueprint's object would show it there.
fn object(template: &AreaObject, p: &Painted) -> AreaObject {
    let mut o = template.clone();
    o.position = p.position;
    o.rotation = template.rotation + p.turn;
    if p.scale != 1.0 {
        let visual = mg_area::VisualTransform { scale: Vec3::splat(p.scale), ..Default::default() };
        o.visual = Some(visual);
    }
    o
}

/// What the brush shows see-through this frame: the stroke's placeables,
/// and before a press, the next dab under the pointer.
pub(crate) fn ghosts(app: &mut Moonglow, ui: &egui::Ui, view: &mut AreaView) -> Vec<AreaObject> {
    let Some(keys) = active(app) else {
        // (Read anew when it is next on: the blueprints may have changed.)
        view.paint.templates.clear();
        view.paint.bounds.clear();
        view.paint.shown = 0;
        return Vec::new();
    };
    view.read_walkable(app);
    for key in &keys {
        if !view.paint.templates.contains_key(key) {
            let template = crate::area_view::template(app, *key);
            if let Some(bounds) = template.as_ref().and_then(|t| view.bounds_of(app, t)) {
                view.paint.bounds.insert(*key, bounds);
            }
            view.paint.templates.insert(*key, template);
        }
    }
    if app.settings.paint.erase {
        // (The eraser shows nothing see-through: what it takes out is
        // marked, `overlay`.)
        view.paint.shown = 0;
        return Vec::new();
    }
    let at = pointer(ui, view);
    let row = app.settings.paint.by_drag();
    if row && view.paint.painting {
        // The row as far as the pointer now.
        if let Some(at) = at {
            view.paint.pending = row_to(app, view, &keys, at).0;
        }
    }
    let mut shown = view.paint.pending.clone();
    match at.filter(|_| !view.paint.painting) {
        Some(at) if row => shown.extend(row_to(app, view, &keys, at).0),
        Some(at) => shown.extend(next_dab(app, view, &keys, at)),
        None => {}
    }
    let templates = &view.paint.templates;
    let objects: Vec<AreaObject> = shown
        .iter()
        .filter_map(|p| templates.get(&p.key)?.as_ref().map(|t| object(t, p)))
        .collect();
    view.paint.shown = objects.len();
    objects
}

/// Shift + wheel over the view sizes the brush's circle: whether the wheel
/// is the brush's (it doesn't zoom then).
pub(crate) fn wheel(app: &mut Moonglow, ui: &egui::Ui, response: &egui::Response) -> bool {
    if active(app).is_none() || app.settings.paint.by_drag() || !response.hovered() {
        return false;
    }
    let (shift, d) = ui.input(|i| (i.modifiers.shift, i.smooth_scroll_delta));
    if !shift {
        return false;
    }
    // (egui turns the wheel sideways with Shift held.)
    let scroll = if d.y == 0.0 { d.x } else { d.y };
    if scroll != 0.0 {
        let r = &mut app.settings.paint.radius_cm;
        let to = (f32::from(*r) * (scroll * 0.003).exp()).round() as u16;
        // At least a centimeter a step, so that a small one tells.
        let to = match to.cmp(r) {
            std::cmp::Ordering::Equal if scroll > 0.0 => to + 1,
            std::cmp::Ordering::Equal => to.saturating_sub(1),
            _ => to,
        };
        *r = to.clamp(RADIUS.0, RADIUS.1);
    }
    true
}

/// The placeables the eraser at `center` takes out: those of the chosen
/// blueprints in its circle (shown, not locked), by their places in
/// `Placeable List`.
fn under_eraser(app: &Moonglow, view: &AreaView, keys: &[ResKey], center: Vec3) -> Vec<usize> {
    let radius = f32::from(app.settings.paint.radius_cm) / 100.0;
    let middle = center.truncate();
    let ours = |o: &AreaObject| o.template.is_some_and(|t| keys.iter().any(|k| k.resref == t));
    view.model
        .iter()
        .flat_map(|m| &m.objects)
        .filter(|o| o.kind == ObjectKind::Placeable && view.show[o.kind.index()] && !o.locked)
        .filter(|o| ours(o) && o.position.truncate().distance(middle) <= radius)
        .map(|o| o.index)
        .collect()
}

/// The brush's clicks and drags on the view: whether they were its (Paint
/// is on with placeables chosen), and nothing else is to take them.
pub(crate) fn input(
    app: &mut Moonglow,
    ui: &egui::Ui,
    view: &mut AreaView,
    response: &egui::Response,
) -> bool {
    let Some(keys) = active(app) else {
        view.paint.pending.clear();
        view.paint.erasing.clear();
        view.paint.painting = false;
        view.paint.row_from = None;
        view.paint.row_end = None;
        // Paint on with nothing chosen to place: Escape or a right click
        // puts it away, as the status line says.
        let stop = crate::area_view::cancelled(ui, response);
        if app.paint && app.palette.selected.is_none() && stop {
            app.paint = false;
            return true;
        }
        return false;
    };
    view.read_walkable(app);
    let hovered = response.hovered();
    // Escape or a right click: the stroke going on is dropped, else Paint
    // goes off (the placeables stay chosen).
    let stop = crate::area_view::cancelled(ui, response);
    if stop {
        if view.paint.painting {
            view.paint.pending.clear();
            view.paint.erasing.clear();
            view.paint.painting = false;
            view.paint.row_from = None;
        } else {
            app.paint = false;
        }
        return true;
    }
    // A press with Ctrl or Alt held is the camera's (Ctrl + drag, Alt +
    // drag move the view): the brush lays nothing with it.
    let (pressed, down, camera, now) = ui.input(|i| {
        let p = &i.pointer;
        (p.primary_pressed(), p.primary_down(), i.modifiers.command || i.modifiers.alt, i.time)
    });
    let s = app.settings.paint;
    if pressed && hovered && !camera {
        view.paint.painting = true;
        view.paint.last = None;
        view.paint.row_from =
            pointer(ui, view).filter(|_| s.by_drag()).map(|p| start_at(view, p, !s.fill));
    }
    if !view.paint.painting {
        return true;
    }
    if s.erase {
        // What the circle passes over goes, when let go.
        if let Some(at) = pointer(ui, view) {
            for i in under_eraser(app, view, &keys, at) {
                if !view.paint.erasing.contains(&i) {
                    view.paint.erasing.push(i);
                }
            }
        }
        if !down {
            view.paint.painting = false;
            let gone: Vec<(ObjectKind, usize)> =
                view.paint.erasing.drain(..).map(|i| (ObjectKind::Placeable, i)).collect();
            if !gone.is_empty() {
                crate::area_view::delete_objects(app, view, &gone, "Erase Placeables");
            }
        }
        return true;
    }
    if s.by_drag() {
        // A row or a fill: from where the press was to the pointer, laid
        // when let go.
        let mut end = None;
        if let Some(to) = pointer(ui, view) {
            let (laid, at) = row_to(app, view, &keys, to);
            view.paint.pending = laid;
            end = Some(at);
        }
        if !down {
            view.paint.painting = false;
            let laid = !view.paint.pending.is_empty();
            commit(app, view);
            // (A row's end, where the next leg may go on from.)
            view.paint.row_end = end.filter(|_| s.row && !s.fill && laid);
            view.paint.row_from = None;
            view.paint.seed = fastrand::u64(..);
        }
        return true;
    }
    if let Some(at) = pointer(ui, view) {
        let step = (f32::from(s.radius_cm) / 100.0 * DAB_STEP).max(0.25);
        let due = view.paint.last.is_none_or(|(p, t)| {
            p.truncate().distance(at.truncate()) >= step || now - t >= DAB_EVERY
        });
        if due {
            let laid = next_dab(app, view, &keys, at);
            view.paint.pending.extend(laid);
            view.paint.seed = fastrand::u64(..);
            view.paint.last = Some((at, now));
        }
    }
    if down {
        // Held still, it goes on laying dabs.
        ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(DAB_EVERY));
    } else {
        view.paint.painting = false;
        commit(app, view);
    }
    true
}

/// Puts the stroke's placeables into the area, as one command: each
/// locked, with Lock.
fn commit(app: &mut Moonglow, view: &mut AreaView) {
    use mg_module::instances::{Placement, instance};
    let pending = std::mem::take(&mut view.paint.pending);
    if pending.is_empty() {
        return;
    }
    let git = view.git();
    let list = ObjectKind::Placeable.list();
    let lock = app.settings.paint.lock;
    let Some(ws) = app.ws.as_mut() else { return };
    let Some(mut index) = ws.doc(&git).ok().map(|g| g.root.list(list).map_or(0, <[_]>::len)) else {
        return;
    };
    let made = crate::area_view::placing(app, |placing, read| {
        // (Each blueprint read once a stroke, however many it lays.)
        let mut blueprints = HashMap::new();
        let mut edits = Vec::new();
        for p in pending {
            let Some(blueprint) = blueprints.entry(p.key).or_insert_with(|| read(p.key)) else {
                continue;
            };
            let placement = Placement { position: p.position.to_array(), rotation: p.turn };
            let Some(mut item) = instance(placing, ResType::UTP, blueprint, placement, &[]) else {
                continue;
            };
            // (A static placeable takes no scale: the game gives it none.)
            if p.scale != 1.0 && item.integer("Static").unwrap_or(0) == 0 {
                let visual =
                    mg_area::VisualTransform { scale: Vec3::splat(p.scale), ..Default::default() };
                item.set("VisTransformList", mg_area::edit::visual_transform_list(visual));
            }
            if lock {
                item.set(mg_area::LOCKED, mg_gff::Value::Byte(1));
            }
            let root = mg_edit::GffPath::root();
            edits.push(mg_edit::Edit::insert_item(git, root, list, index, item));
            index += 1;
        }
        edits
    });
    if let Some(edits) = made.filter(|e| !e.is_empty()) {
        app.actions.push(Action::Apply(Command::new("Paint Placeables", edits)));
    }
}

/// The brush's circle, on the ground around the pointer (the eraser's in
/// red, and what it takes out marked); a row's line, a fill's rectangle,
/// and the last row's end, which a drag may go on from.
pub(crate) fn overlay(app: &Moonglow, ui: &egui::Ui, view: &AreaView) {
    let Some(keys) = active(app) else { return };
    let painter = ui.painter_at(view.rect);
    let s = &app.settings.paint;
    if s.row
        && !s.fill
        && !s.erase
        && !view.paint.painting
        && let Some(end) = view.paint.row_end.and_then(|e| view.screen_pos(e))
    {
        painter.circle_stroke(end, 6.0, Stroke::new(2.0, CIRCLE));
    }
    let Some(at) = pointer(ui, view) else { return };
    if s.by_drag() {
        // The row's line, or the fill's rectangle, from where it began to
        // the pointer.
        if let Some(from) = view.paint.row_from.filter(|_| view.paint.painting) {
            let outline = if s.fill {
                // The rectangle's sides along the grid, as the brush
                // turns it.
                let facing = mg_area::arrange::snap_rotation(view.ghost_turn(), view.snap_angle());
                let (along, across) = grid_axes(facing);
                let span = (at - from).truncate();
                let (u, v) = (along * span.dot(along), across * span.dot(across));
                let corner = |d: Vec2| (from.truncate() + d).extend(from.z);
                vec![corner(Vec2::ZERO), corner(u), corner(u + v), corner(v), corner(Vec2::ZERO)]
            } else {
                vec![from, at]
            };
            // On the ground the pieces stand on, nearest the press's
            // level (not up a wall beside a corner), a little above it.
            let level = |p: &Vec3| {
                let ground = view.ground().and_then(|g| g.height(p.truncate(), from.z));
                p.truncate().extend(ground.unwrap_or(from.z) + 0.05)
            };
            let ends: Vec<Vec3> = outline.iter().map(level).collect();
            for pair in ends.windows(2) {
                if let Some(line) = view.screen_segment(pair[0], pair[1]) {
                    painter.line_segment(line, Stroke::new(2.0, CIRCLE));
                }
            }
        }
        return;
    }
    let color = if s.erase { ERASER } else { CIRCLE };
    let radius = f32::from(s.radius_cm) / 100.0;
    const POINTS: usize = 64;
    let circle: Vec<Vec3> = (0..POINTS)
        .map(|k| {
            let a = k as f32 / POINTS as f32 * std::f32::consts::TAU;
            at + Vec3::new(a.cos(), a.sin(), 0.0) * radius
        })
        .collect();
    // On the walkable ground nearest the pointer's level, as a dab finds
    // it (not up a tower's wall beside it); level where there is none.
    let ground = ground(view, at);
    let circle: Vec<Vec3> = circle
        .into_iter()
        .map(|p| p.truncate().extend(ground(p.truncate()).unwrap_or(at.z) + 0.05))
        .collect();
    for k in 0..POINTS {
        if let Some(line) = view.screen_segment(circle[k], circle[(k + 1) % POINTS]) {
            painter.line_segment(line, Stroke::new(2.0, color));
        }
    }
    if let Some(middle) = view.screen_pos(at) {
        painter.circle_filled(middle, 2.5, color);
    }
    if s.erase {
        // What goes: what the stroke passed over, and what the circle is
        // over now.
        let mut gone = view.paint.erasing.clone();
        gone.extend(under_eraser(app, view, &keys, at));
        let model = view.model.as_ref();
        for i in gone {
            let o = model.and_then(|m| m.object(ObjectKind::Placeable, i));
            if let Some(p) = o.and_then(|o| view.screen_pos(o.position)) {
                let d = egui::vec2(5.0, 5.0);
                painter.line_segment([p - d, p + d], Stroke::new(2.5, ERASER));
                painter.line_segment(
                    [p + egui::vec2(-5.0, 5.0), p + egui::vec2(5.0, -5.0)],
                    Stroke::new(2.5, ERASER),
                );
            }
        }
    }
}

/// The status line while Paint is on.
pub(crate) fn status(app: &Moonglow) -> Option<String> {
    if !app.paint {
        return None;
    }
    let keys = keys(app);
    let s = &app.settings.paint;
    let what = match keys.len() {
        0 => {
            return Some(
                "Paint: choose placeables in the palette (Ctrl + click for several); Escape: stop"
                    .to_string(),
            );
        }
        1 => keys[0].resref.to_string(),
        n => format!("{n} placeables"),
    };
    Some(if s.erase {
        format!(
            "Erasing {what}: drag over them to take them out (not locked ones), Shift + wheel: \
             the brush's size; right click or Escape: stop"
        )
    } else if s.fill {
        format!(
            "Filling with {what}: drag a rectangle from one corner to the other, Q and E: turn \
             the grid; right click or Escape: stop"
        )
    } else if s.row {
        format!(
            "Laying a row of {what}: drag from one end to the other (from the last row's end, \
             its ring, to go on from it), Q and E: turn the pieces on the row; right click or \
             Escape: stop"
        )
    } else {
        format!(
            "Painting {what}: drag to paint, Shift + wheel: the brush's size, Q and E: turn; \
             right click or Escape: stop"
        )
    })
}

/// A length kept in centimeters, edited in meters.
fn meters(ui: &mut egui::Ui, cm: &mut i32, range: (f32, f32)) -> egui::Response {
    let mut m = *cm as f32 / 100.0;
    let r = ui.add(
        egui::DragValue::new(&mut m)
            .speed(0.05)
            .range(range.0..=range.1)
            .fixed_decimals(2)
            .suffix(" m"),
    );
    if r.changed() {
        *cm = (m * 100.0).round() as i32;
    }
    r
}

/// The strip of the brush's settings, above the view while Paint is on.
pub(crate) fn strip(app: &mut Moonglow, ui: &mut egui::Ui) {
    use crate::widgets::field_label;
    let chosen = keys(app).len();
    let s = &mut app.settings.paint;
    ui.horizontal_wrapped(|ui| {
        let mode = |s: &PaintSettings| (s.row && !s.erase, s.fill && !s.erase, s.erase);
        for ((row, fill, erase), label, tip) in [
            ((false, false, false), "Scatter", "Paint at random in a circle"),
            (
                (true, false, false),
                "Row",
                "Lay pieces end to end along a drag (fences, walls), each facing along the row; \
                 a drag from the last row's end goes on from it",
            ),
            (
                (false, true, false),
                "Fill",
                "Fill the rectangle dragged with rows of pieces (floors), on a grid Q and E \
                 turn, each piece by its own size",
            ),
            (
                (false, false, true),
                "Erase",
                "Take the chosen blueprints' placeables out of the circle as it is dragged \
                 over them (not locked ones)",
            ),
        ] {
            let on = mode(s) == (row, fill, erase);
            if ui.selectable_label(on, label).on_hover_text(tip).clicked() {
                (s.row, s.fill, s.erase) = (row, fill, erase);
            }
        }
        ui.separator();
        if !s.by_drag() {
            field_label(ui, "Radius");
            let mut cm = i32::from(s.radius_cm);
            let (low, high) = (f32::from(RADIUS.0) / 100.0, f32::from(RADIUS.1) / 100.0);
            meters(ui, &mut cm, (low, high)).on_hover_text("The circle's size (Shift + wheel)");
            s.radius_cm = cm.clamp(i32::from(RADIUS.0), i32::from(RADIUS.1)) as u16;
        }
        if !s.by_drag() && !s.erase {
            field_label(ui, "Density");
            ui.add(egui::DragValue::new(&mut s.density).range(1..=400).suffix(" a tile"))
                .on_hover_text(
                    "How many placeables a tile's ground (10 by 10 m) holds once painted: the \
                     brush adds until the circle has its share, counting those of these \
                     blueprints already there (every placeable there, with Count All)",
                );
            ui.checkbox(&mut s.count_all, "Count All").on_hover_text(
                "Density counts every placeable in the circle, not only these blueprints'",
            );
            field_label(ui, "Spacing");
            let mut cm = i32::from(s.spacing_cm);
            meters(ui, &mut cm, (0.0, 20.0)).on_hover_text(
                "The least distance from one placeable to the next: between their middles, or \
                 between their sides with By Size",
            );
            s.spacing_cm = cm.clamp(0, 2000) as u16;
            ui.checkbox(&mut s.by_size, "By Size").on_hover_text(
                "Spacing is kept between the placeables' sides, each by its model's box (as \
                 scaled), rather than between their middles: large ones stand further apart",
            );
            field_label(ui, "Flow");
            ui.add(egui::DragValue::new(&mut s.flow).range(1..=100).suffix("%")).on_hover_text(
                "How much of what the circle lacks each dab adds: 100% fills it at once; less \
                 fills it as you go over it, or hold the button",
            );
            field_label(ui, "Turn");
            ui.add(egui::DragValue::new(&mut s.turn_spread).range(0..=360).suffix("°"))
                .on_hover_text(
                    "How far each one's facing strays from the brush's (Q and E turn it): 360° \
                     is any way",
                );
            field_label(ui, "Scale");
            ui.add(egui::DragValue::new(&mut s.scale_min).range(10..=500).suffix("%"));
            ui.label("to");
            ui.add(egui::DragValue::new(&mut s.scale_max).range(10..=500).suffix("%"))
                .on_hover_text(
                    "Each one's scale, picked between these. Static placeables take none (the \
                     game gives them none)",
                );
        }
        if s.by_drag() {
            field_label(ui, "Gap");
            let mut cm = i32::from(s.gap_cm);
            meters(ui, &mut cm, (-10.0, 10.0)).on_hover_text(
                "Room between pieces (an overlap, less than 0): a piece's size is its model's \
                 box, which parts that stick out make larger",
            );
            s.gap_cm = cm.clamp(-1000, 1000) as i16;
        }
        if !s.erase {
            field_label(ui, "Height");
            let mut cm = i32::from(s.lift_cm);
            meters(ui, &mut cm, (-10.0, 10.0))
                .on_hover_text("How far above the ground they stand (below it, less than 0)");
            s.lift_cm = cm.clamp(-1000, 1000) as i16;
            ui.checkbox(&mut s.lock, "Lock").on_hover_text(
                "Lock what is painted (as the selection's Lock does): later clicks and boxes \
                 pass through it to what is beneath",
            );
        }
        ui.separator();
        if chosen == 0 {
            ui.weak("Choose placeables in the palette (Ctrl + click for several)");
        } else {
            ui.weak(format!("{chosen} chosen"))
                .on_hover_text("Ctrl + click in the palette for more, or fewer");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> ResKey {
        ResKey::parse(name, ResType::UTP).unwrap()
    }

    fn settings() -> PaintSettings {
        PaintSettings {
            radius_cm: 500,
            density: 10,
            spacing_cm: 100,
            flow: 100,
            ..Default::default()
        }
    }

    fn flat(_: Vec2) -> Option<f32> {
        Some(2.0)
    }

    fn no_size(_: ResKey) -> f32 {
        0.0
    }

    fn near(x: f32, y: f32, ours: bool) -> Near {
        Near { at: Vec2::new(x, y), ours, reach: 0.0 }
    }

    #[test]
    fn a_full_dab_fills_the_circle_to_its_density() {
        let s = settings();
        let keys = [key("plc_tree"), key("plc_shrub")];
        let at = Vec3::new(20.0, 20.0, 0.0);
        let laid = dab(&s, at, &keys, &[], 0.0, None, 1, (&flat, &no_size));
        // 10 a tile on 78.5 m²: 8.
        assert_eq!(laid.len(), 8);
        for (i, p) in laid.iter().enumerate() {
            assert!(p.position.truncate().distance(at.truncate()) <= 5.0, "inside the circle");
            assert_eq!(p.position.z, 2.0, "on the ground");
            assert!(keys.contains(&p.key));
            for q in &laid[i + 1..] {
                assert!(p.position.distance(q.position) >= 1.0, "kept apart");
            }
        }
        // The same seed, the same dab: what was shown is what is laid.
        assert_eq!(dab(&s, at, &keys, &[], 0.0, None, 1, (&flat, &no_size)), laid);
    }

    #[test]
    fn a_dab_counts_what_is_there_and_keeps_its_distance() {
        let s = settings();
        let keys = [key("plc_tree")];
        let at = Vec3::new(20.0, 20.0, 0.0);
        // Five of the brush's already there, and a chest it isn't painting.
        let mut around: Vec<Near> = (0..5).map(|k| near(18.0 + k as f32, 20.0, true)).collect();
        around.push(near(20.0, 22.0, false));
        let laid = dab(&s, at, &keys, &around, 0.0, None, 3, (&flat, &no_size));
        assert_eq!(laid.len(), 3, "8 wanted, 5 there");
        for p in &laid {
            for q in &around {
                assert!(p.position.truncate().distance(q.at) >= 1.0);
            }
        }
        // Full: nothing more.
        let mut full = around.clone();
        full.extend(laid.iter().map(|p| near(p.position.x, p.position.y, true)));
        assert!(dab(&s, at, &keys, &full, 0.0, None, 4, (&flat, &no_size)).is_empty());
    }

    #[test]
    fn flow_lays_a_share_and_nothing_goes_off_the_walkmesh() {
        let s = PaintSettings { flow: 25, ..settings() };
        let keys = [key("plc_tree")];
        let at = Vec3::new(20.0, 20.0, 0.0);
        assert_eq!(
            dab(&s, at, &keys, &[], 0.0, None, 1, (&flat, &no_size)).len(),
            2,
            "a quarter of 8"
        );
        // Walkmesh only east of x = 20.
        let half = |p: Vec2| (p.x >= 20.0).then_some(0.0);
        let s = settings();
        let laid = dab(&s, at, &keys, &[], 0.0, None, 5, (&half, &no_size));
        assert!(!laid.is_empty() && laid.iter().all(|p| p.position.x >= 20.0));
    }

    #[test]
    fn a_row_lays_pieces_end_to_end_by_their_length() {
        // A piece 2 m long (east to west) and 1 m wide.
        let pieces =
            [(key("plc_floor"), 0.0, (Vec3::new(-1.0, -0.5, 0.0), Vec3::new(1.0, 0.5, 1.0)))];
        let from = Vec3::new(10.0, 10.0, 0.0);
        let east = |to: f32, turn: f32, gap: f32| {
            let to = Vec3::new(to, 10.0, 0.0);
            row(from, to, &pieces, turn, None, (gap, 0.0), 1, &flat).0
        };
        let laid = east(20.0, 0.0, 0.0);
        let xs: Vec<f32> = laid.iter().map(|p| p.position.x).collect();
        assert_eq!(xs, [11.0, 13.0, 15.0, 17.0, 19.0], "the first one's back at the start");
        assert!(laid.iter().all(|p| p.turn == 0.0 && p.position.y == 10.0 && p.position.z == 2.0));
        // Turned a quarter on the row: 1 m a piece; with a gap of 0.5 m, 1.5.
        let laid = east(20.0, std::f32::consts::FRAC_PI_2, 0.5);
        assert_eq!(laid.len(), 7);
        assert!((laid[1].position.x - laid[0].position.x - 1.5).abs() < 1e-4);
        assert!((laid[0].position.x - 10.5).abs() < 1e-4);
        // No length: one piece.
        assert_eq!(east(10.0, 0.0, 0.0).len(), 1);
        // A model whose box isn't around its middle, turned a quarter: its
        // own north (0 to 1 m) lies back along the row.
        let north =
            [(key("plc_floor"), 0.0, (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)))];
        let to = Vec3::new(13.0, 10.0, 0.0);
        let laid = row(from, to, &north, std::f32::consts::FRAC_PI_2, None, (0.0, 0.0), 1, &flat).0;
        let xs: Vec<f32> = laid.iter().map(|p| p.position.x).collect();
        assert_eq!(xs, [11.0, 12.0, 13.0]);
        // Northeast, snapped to 45°: each faces along it.
        let to = Vec3::new(17.0, 16.5, 0.0);
        let laid = row(from, to, &pieces, 0.0, Some(45.0), (0.0, 0.0), 1, &flat).0;
        let way = std::f32::consts::FRAC_PI_4;
        assert!(laid.iter().all(|p| (p.turn - way).abs() < 1e-4));
        let step = laid[1].position - laid[0].position;
        assert!((step.x - step.y).abs() < 1e-4 && (step.length() - 2.0).abs() < 1e-4);
    }

    /// A fill lays rows and columns over the rectangle dragged, the first
    /// piece's corner at the press, the way the drag went, a step apart
    /// the piece's size across each axis (a builder's ask: floors).
    #[test]
    fn a_fill_covers_the_rectangle_in_rows_and_columns() {
        // A piece 2 m east to west and 1 m south to north.
        let pieces =
            [(key("plc_floor"), 0.0, (Vec3::new(-1.0, -0.5, 0.0), Vec3::new(1.0, 0.5, 1.0)))];
        let from = Vec3::new(10.0, 10.0, 0.0);
        let at = |to: (f32, f32), turn: f32, gap: f32| {
            let to = Vec3::new(to.0, to.1, 0.0);
            fill(from, to, &pieces, turn, None, (gap, 0.0), 1, &flat)
        };
        let places = |laid: &[Painted]| {
            let mut v: Vec<(f32, f32)> =
                laid.iter().map(|p| (p.position.x, p.position.y)).collect();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v
        };
        // 6 m by 3 m: three columns of 2 m, three rows of 1 m.
        let laid = at((16.0, 13.0), 0.0, 0.0);
        let mut want = Vec::new();
        for x in [11.0, 13.0, 15.0] {
            for y in [10.5, 11.5, 12.5] {
                want.push((x, y));
            }
        }
        assert_eq!(places(&laid), want);
        assert!(laid.iter().all(|p| p.turn == 0.0 && p.position.z == 2.0));
        // Dragged the other way: from the press towards the south west.
        let laid = at((6.0, 9.0), 0.0, 0.0);
        assert_eq!(places(&laid), [(7.0, 9.5), (9.0, 9.5)]);
        // No drag: one piece; a quarter turn: 1 m across, 2 m down.
        assert_eq!(at((10.0, 10.0), 0.0, 0.0).len(), 1);
        let turned = at((13.0, 14.0), std::f32::consts::FRAC_PI_2, 0.0);
        assert_eq!(turned.len(), 3 * 2);
        let xs: Vec<f32> = places(&turned).iter().map(|p| p.0).collect();
        assert!((xs[2] - xs[0] - 1.0).abs() < 1e-4, "{xs:?}");
        // A gap between them.
        let gapped = at((16.0, 10.0), 0.0, 0.5);
        assert_eq!(places(&gapped).iter().map(|p| p.0).collect::<Vec<_>>(), [11.0, 13.5, 16.0]);
    }

    /// The fill's grid turns with the brush: its rows run the way Q and E
    /// turned it, each piece facing along them, the rectangle's sides
    /// along the grid.
    #[test]
    fn a_fill_s_grid_turns_with_the_brush() {
        let pieces =
            [(key("plc_floor"), 0.0, (Vec3::new(-1.0, -0.5, 0.0), Vec3::new(1.0, 0.5, 1.0)))];
        let from = Vec3::new(10.0, 10.0, 0.0);
        let way = std::f32::consts::FRAC_PI_4;
        let (along, across) = grid_axes(way);
        // 4 m along the grid, 2 m across it.
        let to = from + (along * 4.0 + across * 2.0).extend(0.0);
        let laid = fill(from, to, &pieces, way, None, (0.0, 0.0), 1, &flat);
        assert_eq!(laid.len(), 2 * 2);
        for p in &laid {
            assert!((p.turn - way).abs() < 1e-4, "facing along the grid");
            let d = (p.position - from).truncate();
            let (u, v) = (d.dot(along), d.dot(across));
            assert!((u - 1.0).abs() < 1e-4 || (u - 3.0).abs() < 1e-4, "{u}");
            assert!((v - 0.5).abs() < 1e-4 || (v - 1.5).abs() < 1e-4, "{v}");
        }
        // Snapped to 45°: a turn of 40° lays the same.
        let snapped = fill(from, to, &pieces, 40f32.to_radians(), Some(45.0), (0.0, 0.0), 1, &flat);
        assert_eq!(snapped, laid);
    }

    /// Pieces of different sizes in a fill: each by its own length along
    /// its row, their middles in a line across it (not their near sides:
    /// a row of deep and shallow pieces is centered on one line), the next
    /// row past the deepest of the one before.
    #[test]
    fn a_fill_packs_each_piece_by_its_own_size() {
        // A 2 m by 1 m piece and a 1 m by 2 m one; a third whose box isn't
        // about its middle (0 to 2 m north).
        let long = (key("plc_long"), 0.0, (Vec3::new(-1.0, -0.5, 0.0), Vec3::new(1.0, 0.5, 1.0)));
        let deep = (key("plc_deep"), 0.0, (Vec3::new(-0.5, -1.0, 0.0), Vec3::new(0.5, 1.0, 1.0)));
        let north = (key("plc_north"), 0.0, (Vec3::new(-0.5, 0.0, 0.0), Vec3::new(0.5, 2.0, 1.0)));
        let pieces = [long, deep, north];
        let from = Vec3::new(0.0, 0.0, 0.0);
        let to = Vec3::new(9.0, 6.0, 0.0);
        let laid = fill(from, to, &pieces, 0.0, None, (0.0, 0.0), 7, &flat);
        assert!(pieces.iter().all(|piece| laid.iter().any(|p| p.key == piece.0)));
        let boxed = |k: ResKey| pieces.iter().find(|p| p.0 == k).unwrap().2;
        // The middle of a piece's box, where it stands.
        let middle = |p: &Painted| {
            let (min, max) = boxed(p.key);
            p.position.truncate() + (min + max).truncate() / 2.0
        };
        // Rows by their middle line.
        let mut rows: Vec<(f32, Vec<&Painted>)> = Vec::new();
        for p in &laid {
            let y = middle(p).y;
            match rows.iter_mut().find(|(at, _)| (at - y).abs() < 1e-4) {
                Some((_, row)) => row.push(p),
                None => rows.push((y, vec![p])),
            }
        }
        let size = |p: &Painted| {
            let (min, max) = boxed(p.key);
            (max - min).truncate()
        };
        let mut row_at = 0.0;
        for (line, row) in &rows {
            let deepest = row.iter().map(|p| size(p).y).fold(0.0, f32::max);
            assert!(
                (line - (row_at + deepest / 2.0)).abs() < 1e-4,
                "a row's middles past the last row, by its deepest: {line}, {row_at}"
            );
            // End to end, each by its own length, from the left side.
            let mut at = 0.0;
            for p in row {
                let w = size(p).x;
                assert!((middle(p).x - (at + w / 2.0)).abs() < 1e-4, "{} at {at}", middle(p).x);
                at += w;
            }
            assert!(at >= 9.0 && at - 9.0 < 2.0, "the row covers the rectangle: {at}");
            row_at += deepest;
        }
        assert!(rows.len() >= 3, "{} rows", rows.len());
        assert!(row_at >= 6.0 && row_at - 6.0 < 2.0, "the rows cover it: {row_at}");
    }

    /// Spacing by size keeps the placeables' sides apart, each by its
    /// model's box as scaled; Count All counts every placeable in the
    /// circle towards Density, not only the brush's.
    #[test]
    fn spacing_by_size_and_density_counting_every_placeable() {
        let keys = [key("plc_tree")];
        let at = Vec3::new(20.0, 20.0, 0.0);
        // A tree's box reaches 1.5 m from its middle.
        let tree = |_: ResKey| 1.5;
        assert_eq!(reach_of((Vec3::new(-1.0, -1.5, 0.0), Vec3::new(1.0, 0.5, 4.0)), 1.0), 1.5);
        assert_eq!(reach_of((Vec3::new(-1.0, -1.5, 0.0), Vec3::new(1.0, 0.5, 4.0)), 2.0), 3.0);
        let s = PaintSettings { density: 40, by_size: true, ..settings() };
        let laid = dab(&s, at, &keys, &[], 0.0, None, 1, (&flat, &tree));
        assert!(laid.len() >= 3, "{}", laid.len());
        for (i, p) in laid.iter().enumerate() {
            for q in &laid[i + 1..] {
                let d = p.position.distance(q.position);
                assert!(d >= 1.0 + 1.5 + 1.5 - 1e-4, "sides 1 m apart: {d}");
            }
        }
        // Without By Size, as near as Spacing (1 m): more of them.
        let s = PaintSettings { density: 40, ..settings() };
        let even = dab(&s, at, &keys, &[], 0.0, None, 1, (&flat, &tree));
        assert!(even.len() > laid.len());
        // A big rock of another blueprint beside the circle's middle, 3 m
        // across: by size, nothing within its reach and Spacing.
        let rock = [Near { at: Vec2::new(20.0, 20.0), ours: false, reach: 3.0 }];
        let s = PaintSettings { density: 40, by_size: true, ..settings() };
        for p in dab(&s, at, &keys, &rock, 0.0, None, 2, (&flat, &tree)) {
            assert!(p.position.truncate().distance(rock[0].at) >= 3.0 + 1.0 + 1.5 - 1e-4);
        }
        // Count All: eight wanted (10 a tile on 78.5 m²), five there, of
        // another blueprint: three; counting only the brush's, eight.
        let others: Vec<Near> = (0..5).map(|k| near(16.0 + 2.0 * k as f32, 22.0, false)).collect();
        let s = PaintSettings { count_all: true, ..settings() };
        assert_eq!(dab(&s, at, &keys, &others, 0.0, None, 3, (&flat, &no_size)).len(), 3);
        let s = settings();
        assert_eq!(dab(&s, at, &keys, &others, 0.0, None, 3, (&flat, &no_size)).len(), 8);
    }

    /// A row says where it ends (past its last piece and the gap), which
    /// the next leg of a path begins at: two legs round a corner, end to
    /// end.
    #[test]
    fn a_row_ends_where_the_next_leg_begins() {
        let pieces =
            [(key("plc_wall"), 0.0, (Vec3::new(-1.0, -0.25, 0.0), Vec3::new(1.0, 0.25, 1.0)))];
        let from = Vec3::new(10.0, 10.0, 0.0);
        let (east, end) =
            row(from, Vec3::new(15.0, 10.0, 0.0), &pieces, 0.0, None, (0.5, 0.0), 1, &flat);
        assert_eq!(east.len(), 2, "2 m pieces and a gap of 0.5 m on 5 m");
        assert!((end - Vec3::new(15.0, 10.0, 2.0)).length() < 1e-4, "past the last: {end}");
        // Northwards from there: its first piece's back at the end.
        let (north, _) = row(end, end + Vec3::Y * 4.0, &pieces, 0.0, None, (0.5, 0.0), 1, &flat);
        assert!((north[0].position - Vec3::new(15.0, 11.0, 2.0)).length() < 1e-4);
        assert!((north[0].turn - std::f32::consts::FRAC_PI_2).abs() < 1e-4, "facing along it");
    }

    #[test]

    fn facings_stray_by_turn_and_scales_stay_between_scale() {
        let s = PaintSettings {
            turn_spread: 90,
            scale_min: 80,
            scale_max: 120,
            lift_cm: -30,
            ..settings()
        };
        let keys = [key("plc_tree")];
        let laid = dab(&s, Vec3::new(20.0, 20.0, 0.0), &keys, &[], 1.0, None, 9, (&flat, &no_size));
        assert!(!laid.is_empty());
        for p in &laid {
            assert!((p.turn - 1.0).abs() <= 45f32.to_radians() + 1e-4, "{}", p.turn);
            assert!((0.8..=1.2).contains(&p.scale));
            assert!((p.position.z - 1.7).abs() < 1e-4, "lowered by Height");
        }
        // Snapped to 45°: each facing a multiple of it.
        let s = PaintSettings { turn_spread: 360, ..settings() };
        for p in
            dab(&s, Vec3::new(20.0, 20.0, 0.0), &keys, &[], 0.0, Some(45.0), 2, (&flat, &no_size))
        {
            let steps = p.turn / 45f32.to_radians();
            assert!((steps - steps.round()).abs() < 1e-3, "{}", p.turn);
        }
    }
}
