//! The area view's placeable brush (Paint, on its tool shelf): placeables
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

use mg_uikit::Controls as _;
use std::collections::HashMap;
use std::sync::Arc;

use egui::{Color32, Stroke};
use glam::{Vec2, Vec3};
use mg_area::{AreaObject, ObjectKind};
use mg_core::ResType;
use mg_edit::Command;
use mg_resman::ResKey;

use crate::area_view::AreaView;
use crate::settings::PaintSettings;
use crate::{Action, Moonglow};
use mg_uikit::status::Hint;

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
    /// How many of those were not drawn (past [`GHOSTS_MAX`]): the status
    /// line says so. They are laid all the same.
    pub more: usize,
    /// The next dab as last worked out, and what for: the pointer resting
    /// on the view asks for the same one every frame.
    next: Option<(DabKey, Vec<Painted>)>,
    /// How many times the next dab was worked out (for tests: not every
    /// frame the pointer rests).
    pub worked: u64,
    /// Bumped whenever `pending` changes.
    pending_gen: u64,
    /// Bumped whenever `templates` or `bounds` changes or clears.
    templates_gen: u64,
    /// Monotonic stamp allocated whenever `ghosts_cache` is rebuilt.
    ghost_gen: u64,
    /// Cached `row_to` result while pointer/settings stay fixed.
    row_cache: Option<(RowKey, (Vec<Painted>, Vec3))>,
    /// Cached built ghost `AreaObject`s, counts, and stamp.
    ghosts_cache: Option<GhostsCache>,
    /// Cached 64-point ground circle in `overlay`.
    circle_cache: Option<(CircleKey, Vec<Vec3>)>,
    /// Cached eraser hit indices in `overlay`.
    eraser_cache: Option<(EraserKey, Vec<usize>)>,
}

type GhostsCache = (GhostBuildKey, Arc<[AreaObject]>, usize, usize, u64);

#[derive(Debug, Clone, PartialEq)]
struct RowKey {
    to: [u32; 3],
    row_from: Option<[u32; 3]>,
    row_end: Option<[u32; 3]>,
    settings: PaintSettings,
    keys: Vec<ResKey>,
    templates_gen: u64,
    turn: u32,
    snap_angle: Option<u32>,
    snap_grid: Option<u32>,
    seed: u64,
    area: (Option<u64>, bool),
}

#[derive(Debug, Clone, PartialEq)]
struct GhostBuildKey {
    pending_gen: u64,
    templates_gen: u64,
    painting: bool,
    row: bool,
    preview_row: Option<RowKey>,
    preview_dab: Option<DabKey>,
}

#[derive(Debug, Clone, PartialEq)]
struct CircleKey {
    at: [u32; 3],
    radius_cm: u16,
    area: (Option<u64>, bool),
}

#[derive(Debug, Clone, PartialEq)]
struct EraserKey {
    at: [u32; 3],
    radius_cm: u16,
    keys: Vec<ResKey>,
    area: Option<u64>,
    shows: bool,
}

/// What the next dab under the pointer is worked out from: where, by
/// which seed and settings, with which blueprints (and how many of their
/// models are read), how turned and snapped, in the area as read at which
/// revision (with its scene or without), over how much of a stroke.
#[derive(Debug, Clone, PartialEq)]
struct DabKey {
    center: [u32; 3],
    seed: u64,
    settings: PaintSettings,
    keys: Vec<ResKey>,
    sized: usize,
    turn: u32,
    snap: Option<u32>,
    area: (Option<u64>, bool, bool),
    /// Whether the area's placeables are shown (those hidden are not kept
    /// a distance from).
    shows: bool,
    pending: usize,
}

/// The most see-through placeables the brush draws: a dab of a forest at
/// its widest is tens of thousands, each a model posed every frame.
const GHOSTS_MAX: usize = 400;

impl PaintState {
    /// Drops the stroke going on: nothing of it is laid or taken out.
    pub(crate) fn drop_stroke(&mut self) {
        if !self.pending.is_empty() {
            self.pending.clear();
            self.pending_gen = self.pending_gen.wrapping_add(1);
        }
        self.erasing.clear();
        self.painting = false;
        self.row_from = None;
    }

    /// Clears cached blueprint objects, bounds, and next-dab previews after
    /// resources are reloaded.
    pub(crate) fn clear_caches(&mut self) {
        self.templates.clear();
        self.bounds.clear();
        self.templates_gen = self.templates_gen.wrapping_add(1);
        self.next = None;
        self.row_cache = None;
        self.ghosts_cache = None;
        self.circle_cache = None;
        self.eraser_cache = None;
    }
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
    chosen_keys(p.selected, &p.chosen)
}

/// The placeables among the blueprint clicked in the palette
/// (`selected`) and those chosen with Ctrl + click: what the brush
/// paints with, where the palette shows placeables.
pub(crate) fn chosen_keys(selected: Option<ResKey>, chosen: &[ResKey]) -> Vec<ResKey> {
    let mut keys: Vec<ResKey> = selected.into_iter().chain(chosen.iter().copied()).collect();
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

/// The places a dab keeps its distance from, each with how far its sides
/// reach: asked whether a new one at a point would be too near any. In
/// squares as wide as the farthest two can be and still be too near, so
/// that a point is held against those of its square and the eight around
/// it, not against every one (a thousand placeables a dab, thirty tries
/// each).
struct Taken {
    /// The least gap between two, before their reaches.
    spacing: f32,
    /// A square's side: `spacing` and twice the longest reach.
    cell: f32,
    cells: HashMap<(i32, i32), Vec<(Vec2, f32)>>,
}

impl Taken {
    /// `longest`: the longest reach any of them, or one asked about,
    /// has.
    fn new(spacing: f32, longest: f32) -> Taken {
        let cell = (spacing + 2.0 * longest).max(0.01);
        Taken { spacing, cell, cells: HashMap::new() }
    }

    fn square(&self, p: Vec2) -> (i32, i32) {
        ((p.x / self.cell).floor() as i32, (p.y / self.cell).floor() as i32)
    }

    fn push(&mut self, p: Vec2, reach: f32) {
        self.cells.entry(self.square(p)).or_default().push((p, reach));
    }

    /// Whether one at `p` reaching `reach` would be nearer than the
    /// spacing to one of them.
    fn too_near(&self, p: Vec2, reach: f32) -> bool {
        let (x, y) = self.square(p);
        (-1..=1).flat_map(|dx| (-1..=1).map(move |dy| (dx, dy))).any(|(dx, dy)| {
            let at = (x.saturating_add(dx), y.saturating_add(dy));
            self.cells.get(&at).is_some_and(|there| {
                there.iter().any(|(q, rq)| q.distance(p) < self.spacing + rq + reach)
            })
        })
    }
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
    let (low, high) = (s.scale_min.min(s.scale_max), s.scale_max.max(s.scale_min));
    let near =
        || around.iter().filter(|n| n.at.distance(middle) <= radius + spacing + sized(n.reach));
    // (The longest reach of those around and of what the dab may lay.)
    let largest = keys.iter().map(|k| size(*k)).fold(0.0, f32::max) * f32::from(high) / 100.0;
    let longest = near().map(|n| sized(n.reach)).fold(sized(largest), f32::max);
    let mut taken = Taken::new(spacing, longest);
    for n in near() {
        taken.push(n.at, sized(n.reach));
    }
    let spread = f32::from(s.turn_spread.min(360)).to_radians();
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
        if taken.too_near(p, reach) {
            continue;
        }
        let Some(z) = ground(p) else { continue };
        let turn = mg_area::arrange::snap_rotation(turn + (rng.f32() - 0.5) * spread, snap);
        taken.push(p, reach);
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

fn row_key(app: &Moonglow, view: &AreaView, keys: &[ResKey], to: Vec3) -> RowKey {
    RowKey {
        to: to.to_array().map(f32::to_bits),
        row_from: view.paint.row_from.map(|p| p.to_array().map(f32::to_bits)),
        row_end: view.paint.row_end.map(|p| p.to_array().map(f32::to_bits)),
        settings: app.settings.paint,
        keys: keys.to_vec(),
        templates_gen: view.paint.templates_gen,
        turn: view.ghost_turn().to_bits(),
        snap_angle: view.snap_angle().map(f32::to_bits),
        snap_grid: view.snap_grid().map(f32::to_bits),
        seed: view.paint.seed,
        area: (view.read_at(), view.ground().is_some()),
    }
}

fn row_to_cached(
    app: &Moonglow,
    view: &mut AreaView,
    keys: &[ResKey],
    to: Vec3,
) -> (Vec<Painted>, Vec3) {
    let key = row_key(app, view, keys, to);
    if let Some((was, out)) = &view.paint.row_cache
        && *was == key
    {
        return out.clone();
    }
    let out = row_to(app, view, keys, to);
    view.paint.row_cache = Some((key, out.clone()));
    out
}

/// The ghost `AreaObject`s for the current paint stroke/preview, and an opaque stamp
/// identifying that batch (`0` when empty).
pub(crate) fn ghosts(
    app: &mut Moonglow,
    ui: &egui::Ui,
    view: &mut AreaView,
) -> (Arc<[AreaObject]>, u64) {
    let Some(keys) = active(app) else {
        // (Read anew when it is next on: the blueprints may have changed.)
        if !view.paint.templates.is_empty() || !view.paint.bounds.is_empty() {
            view.paint.templates.clear();
            view.paint.bounds.clear();
            view.paint.templates_gen = view.paint.templates_gen.wrapping_add(1);
        }
        (view.paint.shown, view.paint.more) = (0, 0);
        view.paint.next = None;
        view.paint.row_cache = None;
        view.paint.ghosts_cache = None;
        return (Arc::from([]), 0);
    };
    view.read_walkable(app);
    for key in &keys {
        if !view.paint.templates.contains_key(key) {
            let template = crate::area_view::template(app, *key);
            if let Some(bounds) = template.as_ref().and_then(|t| view.bounds_of(app, t)) {
                view.paint.bounds.insert(*key, bounds);
            }
            view.paint.templates.insert(*key, template);
            view.paint.templates_gen = view.paint.templates_gen.wrapping_add(1);
        }
    }
    if app.settings.paint.erase {
        // (The eraser shows nothing see-through: what it takes out is
        // marked, `overlay`.)
        (view.paint.shown, view.paint.more) = (0, 0);
        return (Arc::from([]), 0);
    }
    let at = pointer(ui, view);
    let row = app.settings.paint.by_drag();
    if row && view.paint.painting {
        // The row as far as the pointer now.
        if let Some(at) = at {
            let laid = row_to_cached(app, view, &keys, at).0;
            if view.paint.pending != laid {
                view.paint.pending = laid;
                view.paint.pending_gen = view.paint.pending_gen.wrapping_add(1);
            }
        }
    }
    let (preview_row, preview_dab) = match at.filter(|_| !view.paint.painting) {
        Some(at) if row => (Some(row_key(app, view, &keys, at)), None),
        Some(at) => {
            // (The same dab while the pointer rests and nothing it is
            // worked out from changes: not worked out every frame.)
            let key = DabKey {
                center: at.to_array().map(f32::to_bits),
                seed: view.paint.seed,
                settings: app.settings.paint,
                sized: view.paint.bounds.len(),
                turn: view.ghost_turn().to_bits(),
                snap: view.snap_angle().map(f32::to_bits),
                area: (view.read_at(), view.holds_scene(), view.ground().is_some()),
                shows: view.show[ObjectKind::Placeable.index()],
                pending: view.paint.pending.len(),
                keys: keys.clone(),
            };
            if !view.paint.next.as_ref().is_some_and(|(k, _)| *k == key) {
                let dab = next_dab(app, view, &keys, at);
                view.paint.next = Some((key.clone(), dab));
                view.paint.worked += 1;
            }
            (None, Some(key))
        }
        None => (None, None),
    };
    let build_key = GhostBuildKey {
        pending_gen: view.paint.pending_gen,
        templates_gen: view.paint.templates_gen,
        painting: view.paint.painting,
        row,
        preview_row,
        preview_dab,
    };
    if let Some((was, objects, total, more, stamp)) = &view.paint.ghosts_cache
        && *was == build_key
    {
        view.paint.shown = *total;
        view.paint.more = *more;
        return (objects.clone(), *stamp);
    }
    let preview: &[Painted] = if view.paint.painting {
        &[]
    } else if row {
        match at {
            Some(at) => {
                let _ = row_to_cached(app, view, &keys, at);
                view.paint.row_cache.as_ref().map_or(&[], |(_, (laid, _))| laid.as_slice())
            }
            None => &[],
        }
    } else if at.is_some() {
        view.paint.next.as_ref().map_or(&[], |(_, dab)| dab.as_slice())
    } else {
        &[]
    };
    let templates = &view.paint.templates;
    let known = |p: &&Painted| templates.get(&p.key).is_some_and(|t| t.is_some());
    let total = view.paint.pending.iter().chain(preview).filter(known).count();
    let objects: Arc<[AreaObject]> = if total == 0 {
        Arc::from([])
    } else {
        view.paint
            .pending
            .iter()
            .chain(preview)
            .filter_map(|p| templates.get(&p.key)?.as_ref().map(|t| object(t, p)))
            .take(GHOSTS_MAX)
            .collect::<Vec<_>>()
            .into()
    };
    let more = total - objects.len();
    let stamp = if objects.is_empty() {
        0
    } else {
        view.paint.ghost_gen = view.paint.ghost_gen.wrapping_add(1).max(1);
        view.paint.ghost_gen
    };
    view.paint.ghosts_cache = Some((build_key, objects.clone(), total, more, stamp));
    view.paint.shown = total;
    view.paint.more = more;
    (objects, stamp)
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
/// is on with placeables chosen, or Escape or a right click put Paint
/// away), and nothing else is to take them.
pub(crate) fn input(
    app: &mut Moonglow,
    ui: &egui::Ui,
    view: &mut AreaView,
    response: &egui::Response,
) -> bool {
    let Some(keys) = active(app) else {
        if !view.paint.pending.is_empty() {
            view.paint.pending.clear();
            view.paint.pending_gen = view.paint.pending_gen.wrapping_add(1);
        }
        view.paint.erasing.clear();
        view.paint.painting = false;
        view.paint.row_from = None;
        view.paint.row_end = None;
        // Paint on with no placeable chosen: Escape or a right click
        // puts it away. The rest is the view's as with Paint off, but for
        // a click that would place what is in hand, which places nothing
        // (`area_view::input`). A paste, an outline drawn anew or a tile
        // paste in hand is dropped first (the innermost thing, as the
        // view's own order has it), and Paint stays on.
        let inner = view.pasting || view.redraw.is_some() || view.tile_pasting;
        if app.paint && !inner && crate::area_view::cancelled(ui, response) {
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
            view.paint.drop_stroke();
        } else {
            app.paint = false;
        }
        return true;
    }
    // A press with Ctrl or Alt held is the camera's (Ctrl + drag, Alt +
    // drag move the view): the brush lays nothing with it.
    let (pressed, down, now) =
        ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down(), i.time));
    let camera = view.camera_press;
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
            let (laid, at) = row_to_cached(app, view, &keys, to);
            if view.paint.pending != laid {
                view.paint.pending = laid;
                view.paint.pending_gen = view.paint.pending_gen.wrapping_add(1);
            }
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
            if !laid.is_empty() {
                view.paint.pending.extend(laid);
                view.paint.pending_gen = view.paint.pending_gen.wrapping_add(1);
            }
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
    view.paint.pending_gen = view.paint.pending_gen.wrapping_add(1);
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
pub(crate) fn overlay(app: &Moonglow, ui: &egui::Ui, view: &mut AreaView) {
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
    let circle_key = CircleKey {
        at: at.to_array().map(f32::to_bits),
        radius_cm: s.radius_cm,
        area: (view.read_at(), view.ground().is_some()),
    };
    if !view.paint.circle_cache.as_ref().is_some_and(|(k, _)| *k == circle_key) {
        let circle: Vec<Vec3> = {
            // On the walkable ground nearest the pointer's level, as a dab finds
            // it (not up a tower's wall beside it); level where there is none.
            let ground = ground(view, at);
            (0..POINTS)
                .map(|k| {
                    let a = k as f32 / POINTS as f32 * std::f32::consts::TAU;
                    at + Vec3::new(a.cos(), a.sin(), 0.0) * radius
                })
                .map(|p| p.truncate().extend(ground(p.truncate()).unwrap_or(at.z) + 0.05))
                .collect()
        };
        view.paint.circle_cache = Some((circle_key, circle));
    }
    let circle = &view.paint.circle_cache.as_ref().expect("just set").1;
    for k in 0..POINTS {
        if let Some(line) = view.screen_segment(circle[k], circle[(k + 1) % POINTS]) {
            painter.line_segment(line, Stroke::new(2.0, color));
        }
    }
    if let Some(middle) = view.screen_pos(at) {
        painter.circle_filled(middle, 2.5, color);
    }
    if s.erase {
        let eraser_key = EraserKey {
            at: at.to_array().map(f32::to_bits),
            radius_cm: s.radius_cm,
            keys,
            area: view.read_at(),
            shows: view.show[ObjectKind::Placeable.index()],
        };
        if !view.paint.eraser_cache.as_ref().is_some_and(|(k, _)| *k == eraser_key) {
            let under = under_eraser(app, view, &eraser_key.keys, at);
            view.paint.eraser_cache = Some((eraser_key, under));
        }
        let under = view.paint.eraser_cache.as_ref().map_or(&[][..], |(_, u)| u.as_slice());
        // What goes: what the stroke passed over, and what the circle is
        // over now.
        let model = view.model.as_ref();
        for i in view.paint.erasing.iter().copied().chain(under.iter().copied()) {
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

/// The Status Bar's hints while Paint is on (the AreaPaint board's): what
/// the brush is doing as a notice, then its keys.
pub(crate) fn hints(app: &Moonglow, k: &crate::shell::HintKeys) -> Option<Vec<Hint>> {
    if !app.paint {
        return None;
    }
    use crate::keys::Cmd;
    let keys = keys(app);
    let s = &app.settings.paint;
    let several = Hint::keys([k.ctrl(), "Click"], "Choose several in the palette");
    let stop = Hint::either(["Right Click", "Esc"], "Stop");
    let size = Hint::keys(["Shift", "Wheel"], "Brush size");
    let what = match keys.len() {
        0 => {
            let note = if app.palette.selected.is_some() {
                "Paint works with placeables: choose some in the palette"
            } else {
                "Paint: choose placeables in the palette"
            };
            let choose = Hint::note(note);
            return Some(vec![choose, several, Hint::keys(["Esc"], "Stop")]);
        }
        1 => keys[0].resref.to_string(),
        n => format!("{n} placeables"),
    };
    let turn = |word: &str| k.either(Cmd::TurnLeft, Cmd::TurnRight, word);
    let mut out = Vec::new();
    if s.erase {
        out.push(Hint::note(format!("Erasing {what}")));
        out.push(Hint::keys(["Drag"], "Take them out (not locked ones)"));
        out.push(size);
        out.push(stop);
    } else if s.fill {
        out.push(Hint::note(format!("Filling with {what}")));
        out.push(Hint::keys(["Drag"], "Fill a rectangle"));
        out.extend(turn("Turn the grid"));
        out.push(stop);
    } else if s.row {
        out.push(Hint::note(format!("Laying a row of {what}")));
        out.push(Hint::keys(["Drag"], "Lay a row, or go on from the last one's ring"));
        out.extend(turn("Turn the pieces"));
        out.push(stop);
    } else {
        out.push(Hint::note(format!("Painting {what}")));
        out.push(Hint::keys(["Drag"], "Paint"));
        out.push(size);
        out.extend(turn("Turn"));
        out.push(stop);
        out.push(several);
    }
    Some(out)
}

/// A length kept in centimeters, edited in meters, in a field `width`
/// wide named `name` (its label is drawn apart).
fn meters(
    ui: &mut egui::Ui,
    cm: &mut i32,
    range: (f32, f32),
    (name, width): (&str, f32),
) -> egui::Response {
    let mut m = *cm as f32 / 100.0;
    let field = egui::DragValue::new(&mut m).speed(0.05).range(range.0..=range.1).fixed_decimals(2);
    let r = number(ui, name, width, field.suffix(" m"));
    if r.changed() {
        *cm = (m * 100.0).round() as i32;
    }
    r
}

/// A number field `width` wide and a control high, named `name` for a
/// screen reader (and tests): its words beside it are drawn, not a label
/// of their own.
fn number(ui: &mut egui::Ui, name: &str, width: f32, field: egui::DragValue<'_>) -> egui::Response {
    let r = ui.add_sized([width, mg_uikit::metrics::CONTROL_HEIGHT], field);
    ui.ctx().accesskit_node_builder(r.id, |node| node.set_label(name));
    r
}

/// A field's words, `width` wide (its own where 0), in `color`: drawn,
/// so that the field beside it is what its name names.
fn words(ui: &mut egui::Ui, words: &str, width: f32, color: Color32) {
    let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
    let galley = ui.painter().layout_no_wrap(words.to_string(), font, color);
    let w = if width > 0.0 { width } else { galley.size().x };
    let size = egui::vec2(w, mg_uikit::metrics::CONTROL_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let at = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, color);
}

/// The width of `w` in the body's text.
fn text_width(ui: &egui::Ui, w: &str) -> f32 {
    let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
    ui.painter().layout_no_wrap(w.to_string(), font, Color32::WHITE).size().x
}

/// The header's segmented control of how the brush paints (its group's
/// name).
pub(crate) const MODE: &str = "How the brush paints";
/// The Brush popover's name and words.
pub(crate) const BRUSH: &str = "Brush";
/// The palette's Brush section's group (named apart from the popover).
pub(crate) const BRUSH_SECTION: &str = "Brush blueprints";

/// The modes, as `(row, fill, erase)`, with their words and tips.
const MODES: [((bool, bool, bool), &str, &str); 4] = [
    ((false, false, false), "Scatter", "Paint at random in a circle"),
    ((true, false, false), "Row", "Lay pieces end to end along a drag, such as fences and walls"),
    (
        (false, true, false),
        "Fill",
        "Fill the rectangle dragged with rows of pieces, such as floors",
    ),
    (
        (false, false, true),
        "Erase",
        "Remove the chosen blueprints' placeables under the circle, except locked ones",
    ),
];

fn mode_of(s: &PaintSettings) -> (bool, bool, bool) {
    (s.row && !s.erase, s.fill && !s.erase, s.erase)
}

/// The words of the mode `s` paints in (the Brush popover's title note).
fn mode_words(s: &PaintSettings) -> &'static str {
    MODES.iter().find(|(m, ..)| *m == mode_of(s)).map_or("Scatter", |(_, w, _)| w)
}

/// A header field's width (the AreaPaint board's `.hnum`).
const HEADER_FIELD: f32 = 58.0;
/// The gap between a header field's words and the field (`.grp`: 6).
const HEADER_GAP: f32 = 6.0;
/// The Brush popover's width, its label column and its fields' (the
/// board's `.brush-pop`, `--lbl-w` and `.num`).
const BRUSH_WIDTH: f32 = 286.0;
const BRUSH_LABEL: f32 = 64.0;
const BRUSH_FIELD: f32 = 76.0;
/// The Brush popover's Spacing field (the board's, wider than the rest).
const BRUSH_SPACING: f32 = 120.0;
/// A Brush popover row's height inside its 1 point margins.
const BRUSH_ROW: f32 = 26.0;

/// Paint's options in the area header's left group while Paint is the
/// tool in hand (the AreaPaint board; the proposal's section 6.1): the
/// mode as a segmented control (Scatter, Row, Fill, Erase), Radius and
/// Density where the mode takes them (Scatter both, Erase the radius
/// alone), and the Brush popover with the rest: Count all, Spacing and
/// By size, Flow, Turn, Scale (Scatter), Gap (Row and Fill), Height and
/// Lock (all but Erase), the note on which mode takes which, and Snap and
/// Turn "as with the Select tool". Each moves into More where the row
/// has no room. `s` is the settings, written back by the caller after
/// the header is drawn; `placing` the Select tool's Snap and Turn (none
/// where both are hidden).
pub(crate) fn header_items<'a>(
    mut h: mg_uikit::header::EditorHeader<'a>,
    ui: &egui::Ui,
    s: &'a std::cell::Cell<PaintSettings>,
    placing: Option<impl FnOnce(&mut egui::Ui) + 'a>,
) -> mg_uikit::header::EditorHeader<'a> {
    use mg_uikit::header::Item;
    use mg_uikit::metrics::CONTROL_PADDING_X;
    let now = s.get();
    // The mode.
    let seg_width =
        MODES.iter().map(|(_, w, _)| text_width(ui, w) + 2.0 * CONTROL_PADDING_X).sum::<f32>()
            + (MODES.len() - 1) as f32;
    let seg_width = seg_width.ceil();
    let seg = move |ui: &mut egui::Ui| {
        let mut p = s.get();
        let mut m = mode_of(&p);
        let choices = MODES.map(|(m, w, t)| (m, w, t));
        mg_uikit::segmented(ui, MODE, &mut m, &choices);
        (p.row, p.fill, p.erase) = m;
        s.set(p);
    };
    h = h.left(Item::custom(MODE, seg_width, seg).more(seg_width, move |ui| {
        ui.horizontal(seg);
    }));
    // Radius and Density.
    let t = mg_uikit::tokens(ui.ctx());
    let label = t.text_2;
    if !now.by_drag() {
        let width = (text_width(ui, "Radius") + HEADER_GAP + HEADER_FIELD).ceil();
        let radius = move |ui: &mut egui::Ui| {
            ui.spacing_mut().item_spacing.x = HEADER_GAP;
            words(ui, "Radius", 0.0, label);
            let mut p = s.get();
            let mut cm = i32::from(p.radius_cm);
            let (low, high) = (f32::from(RADIUS.0) / 100.0, f32::from(RADIUS.1) / 100.0);
            meters(ui, &mut cm, (low, high), ("Radius", HEADER_FIELD))
                .on_hover_text("The circle's size (Shift+wheel)");
            p.radius_cm = cm.clamp(i32::from(RADIUS.0), i32::from(RADIUS.1)) as u16;
            s.set(p);
        };
        h = h.left(Item::custom("Radius", width, radius).more(width, move |ui| {
            ui.horizontal(radius);
        }));
    }
    if !now.by_drag() && !now.erase {
        let width = (text_width(ui, "Density") + HEADER_GAP + HEADER_FIELD).ceil();
        let density = move |ui: &mut egui::Ui| {
            ui.spacing_mut().item_spacing.x = HEADER_GAP;
            words(ui, "Density", 0.0, label);
            let mut p = s.get();
            let field = egui::DragValue::new(&mut p.density).range(1..=400).suffix(" a tile");
            number(ui, "Density", HEADER_FIELD, field)
                .on_hover_text("How many placeables a 10 by 10 m tile holds once painted");
            s.set(p);
        };
        h = h.left(Item::custom("Density", width, density).more(width, move |ui| {
            ui.horizontal(density);
        }));
    }
    // The Brush popover.
    let text = t.text;
    let row = move |ui: &mut egui::Ui, label: &str| {
        ui.spacing_mut().item_spacing.x = 8.0;
        // (Rows 28 apart, the board's: 26 and the popover's 1 at each
        // side.)
        ui.set_min_height(BRUSH_ROW);
        words(ui, label, BRUSH_LABEL, text);
    };
    let mut pop = mg_uikit::popover::Popover::new(BRUSH)
        .title(Some(mg_uikit::Icon::Brush), BRUSH)
        .title_note(mode_words(&now))
        .min_width(BRUSH_WIDTH);
    if !now.by_drag() && !now.erase {
        pop = pop
            .custom(move |ui| {
                row(ui, "");
                let mut p = s.get();
                ui.check_box(&mut p.count_all, "Count all").on_hover_text(
                    "Density counts every placeable in the circle, not only these blueprints'",
                );
                s.set(p);
            })
            .custom(move |ui| {
                row(ui, "Spacing");
                let mut p = s.get();
                let mut cm = i32::from(p.spacing_cm);
                meters(ui, &mut cm, (0.0, 20.0), ("Spacing", BRUSH_SPACING)).on_hover_text(
                    "The least distance between placeables, middle to middle (side to side with \
                     By size)",
                );
                p.spacing_cm = cm.clamp(0, 2000) as u16;
                ui.check_box(&mut p.by_size, "By size").on_hover_text(
                    "Keep spacing between the placeables' sides, so large ones stand further \
                     apart",
                );
                s.set(p);
            })
            .custom(move |ui| {
                row(ui, "Flow");
                let mut p = s.get();
                let field = egui::DragValue::new(&mut p.flow).range(1..=100).suffix(" %");
                number(ui, "Flow", BRUSH_FIELD, field).on_hover_text(
                    "How much of what the circle lacks each dab adds. 100% fills it at once.",
                );
                s.set(p);
            })
            .custom(move |ui| {
                row(ui, "Turn");
                let mut p = s.get();
                let field = egui::DragValue::new(&mut p.turn_spread).range(0..=360).suffix("°");
                number(ui, TURN_SPREAD, BRUSH_FIELD, field)
                    .on_hover_text("How far each facing strays from the brush's: 360° is any way");
                s.set(p);
            })
            .custom(move |ui| {
                row(ui, "Scale");
                let mut p = s.get();
                let low = egui::DragValue::new(&mut p.scale_min).range(10..=500).suffix(" %");
                number(ui, "Scale", BRUSH_FIELD, low);
                ui.label(egui::RichText::new("to").color(t.text_2));
                let high = egui::DragValue::new(&mut p.scale_max).range(10..=500).suffix(" %");
                number(ui, "Scale to", BRUSH_FIELD, high).on_hover_text(
                    "Each one's scale, picked between these. Static placeables take none.",
                );
                s.set(p);
            });
    }
    if now.by_drag() {
        pop = pop.custom(move |ui| {
            row(ui, "Gap");
            let mut p = s.get();
            let mut cm = i32::from(p.gap_cm);
            meters(ui, &mut cm, (-10.0, 10.0), ("Gap", BRUSH_FIELD))
                .on_hover_text("Room between pieces, by their models' boxes (below 0, an overlap)");
            p.gap_cm = cm.clamp(-1000, 1000) as i16;
            s.set(p);
        });
    }
    if !now.erase {
        pop = pop
            .custom(move |ui| {
                row(ui, "Height");
                let mut p = s.get();
                let mut cm = i32::from(p.lift_cm);
                meters(ui, &mut cm, (-10.0, 10.0), ("Height", BRUSH_FIELD))
                    .on_hover_text("How far above the ground they stand (below it, less than 0)");
                p.lift_cm = cm.clamp(-1000, 1000) as i16;
                s.set(p);
            })
            .custom(move |ui| {
                row(ui, "");
                let mut p = s.get();
                ui.check_box(&mut p.lock, "Lock").on_hover_text(
                    "Lock what is painted, so later clicks and boxes pass through it",
                );
                s.set(p);
            });
    }
    let note = t.text_3;
    pop = pop.custom(move |ui| {
        let width = ui.available_width();
        ui.vertical(|ui| {
            ui.set_width(width);
            let words = "Row and Fill take Gap, Height and Lock, Erase the radius alone.";
            ui.add(egui::Label::new(egui::RichText::new(words).color(note)).wrap());
        });
    });
    // (Snap and Turn, where the header shows them: no heading over none.)
    if let Some(placing) = placing {
        pop = pop.separator().heading("Placing, as with the Select tool").custom(placing);
    }
    let tip = "The brush's other settings, and Snap and Turn";
    h.left(Item::popover(Some(mg_uikit::Icon::Brush), pop).words_icon(false).tip(tip))
}

/// The name of the Brush popover's Turn field (how far facings stray),
/// apart from the Turn combo box under it (the angle turned objects snap
/// to).
pub(crate) const TURN_SPREAD: &str = "Turn spread";

/// The palette's Brush section while Paint is on (the AreaPaint board):
/// "Brush" with how many blueprints are chosen, a row for each of `keys`
/// (its name by `names`, its ResRef) with × to take it out of the brush,
/// and a note on choosing more; with none chosen, how to choose them. The
/// blueprint whose × was clicked.
pub(crate) fn brush_section(
    ui: &mut egui::Ui,
    keys: &[ResKey],
    names: &dyn Fn(ResKey) -> Option<String>,
) -> Option<ResKey> {
    let t = mg_uikit::tokens(ui.ctx());
    mg_uikit::name_group(ui, BRUSH_SECTION);
    // (The board's `.sec-h`: its heading, the note beside it, a rule.)
    ui.horizontal(|ui| {
        mg_uikit::table_heading(ui, BRUSH);
        let chosen = match keys.len() {
            0 => "none chosen".to_string(),
            n => format!("{n} chosen"),
        };
        ui.label(egui::RichText::new(chosen).color(t.text_3));
    });
    let (rule, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().hline(rule.x_range(), rule.center().y, Stroke::new(1.0, t.line_soft));
    ui.add_space(mg_uikit::metrics::GAP_S);
    let mut out = None;
    for key in keys {
        ui.horizontal(|ui| {
            ui.set_min_height(mg_uikit::metrics::ROW_HEIGHT);
            let square = egui::Vec2::splat(mg_uikit::metrics::ICON);
            let (rect, _) = ui.allocate_exact_size(square, egui::Sense::hover());
            crate::icons::object(mg_module::palette::BlueprintKind::Placeable).paint(
                ui.painter(),
                rect,
                t.text_2,
            );
            let name = names(*key).unwrap_or_else(|| key.resref.to_string());
            ui.label(egui::RichText::new(name).color(t.text));
            // (The board's room between the name and the ResRef.)
            ui.add_space(8.0);
            ui.label(egui::RichText::new(key.resref.to_string()).monospace().color(t.text_3));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let x = mg_uikit::icon_button(ui, mg_uikit::Icon::Close, &take_out_name(*key));
                if x.clicked() {
                    out = Some(*key);
                }
            });
        });
    }
    let words = match keys.len() {
        0 => "Choose placeables in the palette (Ctrl + click for several).",
        _ => {
            "Ctrl + click in the palette for more, or fewer. The brush's numbers are in the \
             header."
        }
    };
    let width = ui.available_width();
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.add(egui::Label::new(egui::RichText::new(words).color(t.text_3)).wrap());
    });
    out
}

/// The accessible name and tip of a Brush row's ×: "Take x3_plc_bush001
/// out of the brush (Ctrl+click it in the palette)".
pub(crate) fn take_out_name(key: ResKey) -> String {
    format!("Take {} out of the brush (Ctrl+click it in the palette)", key.resref)
}

/// `key` taken out of the brush: out of those chosen with Ctrl + click
/// (`chosen`), or, the one clicked (`selected`), the next chosen in its
/// place.
pub(crate) fn take_out(selected: &mut Option<ResKey>, chosen: &mut Vec<ResKey>, key: ResKey) {
    if *selected == Some(key) {
        *selected = if chosen.is_empty() { None } else { Some(chosen.remove(0)) };
    }
    chosen.retain(|k| *k != key);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The review's A17: a dab asks of each place it tries whether it is
    /// too near what is there, of squares of them, not of every one. The
    /// squares answer as going through them all does, whatever the
    /// spacing and the reaches (none, small, larger than the spacing).
    #[test]
    fn the_places_taken_answer_by_squares_as_gone_through() {
        let mut rng = fastrand::Rng::with_seed(11);
        for (spacing, longest) in [(0.0, 0.0), (0.4, 0.0), (0.0, 1.5), (0.75, 2.0), (3.0, 0.3)] {
            let mut taken = Taken::new(spacing, longest);
            let mut all: Vec<(Vec2, f32)> = Vec::new();
            let point = |rng: &mut fastrand::Rng| {
                Vec2::new(rng.f32() * 40.0 - 20.0, rng.f32() * 40.0 - 20.0)
            };
            let mut near = 0;
            for _ in 0..600 {
                let (p, reach) = (point(&mut rng), rng.f32() * longest);
                let scan = all.iter().any(|(q, rq)| q.distance(p) < spacing + rq + reach);
                assert_eq!(taken.too_near(p, reach), scan, "{p} reaching {reach}");
                near += usize::from(scan);
                if !scan || rng.bool() {
                    taken.push(p, reach);
                    all.push((p, reach));
                }
            }
            assert_eq!(near > 0, spacing + longest > 0.0, "spacing {spacing}, reach {longest}");
        }
    }

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
