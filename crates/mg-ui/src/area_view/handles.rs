//! The area view's handles as the redesign draws them, "Fine Lines" (the
//! proposal's section 11): every line over a dark edge two points wider,
//! the rings as projected circles of [`RING_DRAWN`] pieces (a tilt ring's
//! half behind its pivot at 40%), solid heads and facing triangles, the
//! scale square on its stalk, the pivot dot; lit, a handle is thicker and
//! brighter, and the rest of the set draws at half strength. Painted with
//! egui's 2D painter over the 3D picture, in layers ([`Canvas`]) so that
//! no edge is drawn over another handle's line.
//!
//! What is drawn where, and who gets which handle, is the view's
//! (`turn_rings`, `tilt_rings`, `axis_arrows`, `scale_handles`); which
//! handle a press takes is [`AreaView::handle_under`]'s, which lights it,
//! names it in a chip and gives the pointer its grab cursor.

use egui::epaint::{CornerRadius, PathShape, PathStroke};
use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2};
use glam::Vec3;
use mg_uikit::Tokens;

use super::{AreaModel, AreaView, Drag, Handle, TiltRing};

/// The straight pieces a ring is drawn as (it is taken by the view's
/// `RING_SEGMENTS`, as before).
pub(super) const RING_DRAWN: usize = 96;

/// A ring's line and its lit width, points (its edge two more).
const RING: (f32, f32) = (1.5, 2.5);

/// An arrow's shaft, to 1.2 r (not a target).
const SHAFT: (f32, f32) = (1.0, 1.5);

/// An arrow's head line, 1.2 r to the head.
const HEAD_LINE: (f32, f32) = (1.5, 2.5);

/// An arrow's head: a solid triangle, length by width (lit).
const HEAD: ((f32, f32), (f32, f32)) = ((12.0, 8.0), (14.0, 10.0));

/// The turning ring's facing triangle, length by width (lit).
const FACING: ((f32, f32), (f32, f32)) = ((8.0, 9.0), (10.0, 11.0));

/// The scale square's side (lit).
const SQUARE: (f32, f32) = (9.0, 11.0);

/// The scale square's stalk (lit), drawn at 75% unlit.
const STALK: (f32, f32) = (1.0, 1.5);

/// The pivot dot's radius.
const PIVOT: f32 = 2.0;

/// The far half of a tilt ring: its strength.
const FAR: f32 = 0.4;

/// The rest of a set while one handle is lit or held: their strength.
const DIMMED: f32 = 0.5;

/// The dark edge's strength (of black).
const EDGE: f32 = 0.55;

/// A drag's wedge (the ring's color) and guide line (the axis's): their
/// strengths.
const WEDGE: f32 = 0.18;
const GUIDE: f32 = 0.35;

/// Where a drag began (a hollow facing triangle, dot or square): its
/// strength.
const START: f32 = 0.85;

/// How far a guide line runs either way from the object, meters (cut at
/// the near plane, and by the view's edge).
const GUIDE_REACH: f32 = 500.0;

/// A snap tick's length out from the ring, points.
const TICK: f32 = 5.0;

/// The selection box's line, its strength and its edge's.
const BOX: (f32, f32, f32) = (1.0, 0.9, 0.45);

/// The layers, painted in this order: the selection boxes, a drag's
/// fills and guide line, the far halves' edges and lines, the handles'
/// edges and lines, those on top (the pivot dots, a drag's start marks),
/// and the chips.
#[derive(Debug, Clone, Copy)]
pub(super) enum Layer {
    Box,
    Fill,
    BackEdge,
    Back,
    Edge,
    Line,
    TopEdge,
    Top,
    Chip,
}

/// A line's pair of layers: its edge's and its own.
#[derive(Debug, Clone, Copy)]
pub(super) enum Lay {
    Back,
    Front,
    Top,
}

impl Lay {
    fn layers(self) -> (Layer, Layer) {
        match self {
            Lay::Back => (Layer::BackEdge, Layer::Back),
            Lay::Front => (Layer::Edge, Layer::Line),
            Lay::Top => (Layer::TopEdge, Layer::Top),
        }
    }
}

/// The handles' shapes, gathered in their layers and painted together.
pub(super) struct Canvas {
    layers: [Vec<Shape>; 9],
    /// The dark edge under every line, head and square (the kit's
    /// `handle-edge`, black at 55%).
    edge: Color32,
}

impl Canvas {
    pub(super) fn new(edge: Color32) -> Canvas {
        Canvas { layers: Default::default(), edge }
    }

    /// The dark edge at `strength`.
    fn edge(&self, strength: f32) -> Color32 {
        self.edge.gamma_multiply(strength)
    }

    pub(super) fn add(&mut self, layer: Layer, shape: Shape) {
        self.layers[layer as usize].push(shape);
    }

    /// Everything gathered, in its layers' order.
    pub(super) fn paint(self, painter: &egui::Painter) {
        painter.extend(self.layers.into_iter().flatten());
    }

    /// A line through `points` (closed back to the first: `closed`), over
    /// its dark edge, `width` in `color` at `strength`.
    pub(super) fn path(
        &mut self,
        points: Vec<Pos2>,
        closed: bool,
        width: f32,
        color: Color32,
        strength: f32,
        lay: Lay,
    ) {
        if points.len() < 2 {
            return;
        }
        let (under, over) = lay.layers();
        let dark = self.edge(strength);
        let shape = |points: Vec<Pos2>, stroke: Stroke| {
            Shape::Path(PathShape {
                points,
                closed,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::from(stroke),
            })
        };
        self.add(under, shape(points.clone(), Stroke::new(width + 2.0, dark)));
        self.add(over, shape(points, Stroke::new(width, color.gamma_multiply(strength))));
    }

    /// A solid triangle over its dark edge.
    fn triangle(&mut self, points: [Pos2; 3], color: Color32, strength: f32, lay: Lay) {
        let (under, over) = lay.layers();
        let dark = self.edge(strength);
        self.add(under, Shape::convex_polygon(points.to_vec(), dark, Stroke::new(2.0, dark)));
        let fill = color.gamma_multiply(strength);
        self.add(over, Shape::convex_polygon(points.to_vec(), fill, Stroke::NONE));
    }

    /// An arrow's head: a solid triangle whose point is at `tip`, pointing
    /// `along` (a unit), `size` long and wide.
    fn head(&mut self, tip: Pos2, along: Vec2, size: (f32, f32), color: Color32, strength: f32) {
        let side = along.rot90();
        let base = tip - along * size.0;
        let points = [tip, base + side * size.1 / 2.0, base - side * size.1 / 2.0];
        self.triangle(points, color, strength, Lay::Front);
    }

    /// A facing triangle standing on the ring at `at`, pointing out (a
    /// unit): solid, or hollow (a closed 1 point line) where a turn began.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn facing(
        &mut self,
        at: Pos2,
        out: Vec2,
        size: (f32, f32),
        color: Color32,
        strength: f32,
        hollow: bool,
        lay: Lay,
    ) {
        let side = out.rot90();
        let base = at - out;
        let points = [base + out * size.0, base + side * size.1 / 2.0, base - side * size.1 / 2.0];
        if hollow {
            self.path(points.to_vec(), true, 1.0, color, strength, lay);
        } else {
            self.triangle(points, color, strength, lay);
        }
    }

    /// The scale square, `side` across at `c`, over its dark edge; hollow,
    /// a 1.25 point outline (where a scale drag began).
    pub(super) fn square(
        &mut self,
        c: Pos2,
        side: f32,
        color: Color32,
        strength: f32,
        hollow: bool,
    ) {
        let rect = Rect::from_center_size(c, Vec2::splat(side));
        if hollow {
            let ring =
                |r: Rect| vec![r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()];
            self.path(ring(rect), true, 1.25, color, strength, Lay::Front);
            return;
        }
        let dark = self.edge(strength);
        self.add(Layer::Edge, Shape::rect_filled(rect.expand(1.0), CornerRadius::same(2), dark));
        let fill = color.gamma_multiply(strength);
        self.add(Layer::Line, Shape::rect_filled(rect, CornerRadius::same(1), fill));
    }

    /// A round dot of radius `r` over its dark edge (a point wider).
    pub(super) fn dot(&mut self, c: Pos2, r: f32, color: Color32, strength: f32, lay: Lay) {
        let (under, over) = lay.layers();
        let dark = self.edge(strength);
        self.add(under, Shape::circle_filled(c, r + 1.0, dark));
        self.add(over, Shape::circle_filled(c, r, color.gamma_multiply(strength)));
    }

    /// A chip (the proposal's section 11.5): `text` in Hack 11 on the
    /// theme's chip, 18 points high, radius 3, a 2 point strip in `strip`
    /// at its left, its bottom-left corner at `corner`.
    pub(super) fn chip(
        &mut self,
        painter: &egui::Painter,
        t: &Tokens,
        corner: Pos2,
        text: String,
        strip: Color32,
    ) {
        let galley = painter.layout_no_wrap(text, egui::FontId::monospace(11.0), t.text);
        let size = egui::vec2(2.0 + 6.0 + galley.size().x + 6.0, CHIP_HEIGHT);
        let bounds = painter.clip_rect().shrink(4.0);
        let mut pos = corner - egui::vec2(0.0, size.y);
        if bounds.width() >= size.x {
            pos.x = pos.x.min(bounds.right() - size.x).max(bounds.left());
        } else {
            pos.x = bounds.left();
        }
        if bounds.height() >= size.y {
            pos.y = pos.y.min(bounds.bottom() - size.y).max(bounds.top());
        } else {
            pos.y = bounds.top();
        }
        let rect = Rect::from_min_size(pos, size);
        let rim = Stroke::new(1.0, t.chip_rim);
        let chip = egui::epaint::RectShape::new(
            rect,
            CornerRadius::same(3),
            t.chip,
            rim,
            egui::StrokeKind::Inside,
        );
        self.add(Layer::Chip, Shape::Rect(chip));
        let edge = Rect::from_min_size(rect.min, egui::vec2(2.0, size.y));
        let round = CornerRadius { nw: 2, sw: 2, ne: 0, se: 0 };
        self.add(Layer::Chip, Shape::rect_filled(edge, round, strip));
        let at = egui::pos2(rect.min.x + 8.0, rect.center().y - galley.size().y / 2.0);
        self.add(Layer::Chip, Shape::galley(at, galley, t.text));
    }
}

/// `v` as a unit, if it has a length.
fn unit(v: Vec2) -> Option<Vec2> {
    (v.length() > 1e-3).then(|| v.normalized())
}

/// A chip's height, points.
pub(super) const CHIP_HEIGHT: f32 = 18.0;

/// Where a name chip stands from the pointer: its bottom-left corner 12
/// right of it and 12 up, as the scale readout's.
pub(super) const NAME_CHIP_OFFSET: Vec2 = egui::vec2(12.0, -12.0);

/// A ring through `points` (in order round it) on screen: the runs of it
/// in front of the eye, each cut at the near plane by `segment`, as
/// lines (closed where the whole ring shows).
pub(super) fn runs(
    points: &[Vec3],
    segment: &dyn Fn(Vec3, Vec3) -> Option<[Pos2; 2]>,
) -> Vec<(Vec<Pos2>, bool)> {
    let n = points.len();
    let pieces: Vec<Option<[Pos2; 2]>> =
        (0..n).map(|k| segment(points[k], points[(k + 1) % n])).collect();
    // (Whether piece `k` goes on from the one before it.)
    let joined = |k: usize| match (pieces[(k + n - 1) % n], pieces[k]) {
        (Some([_, end]), Some([start, _])) => end.distance(start) < 0.01,
        _ => false,
    };
    let Some(first) = (0..n).find(|&k| !joined(k)) else {
        // All of it, round to the first.
        return vec![(pieces.iter().flatten().map(|p| p[0]).collect(), true)];
    };
    let mut out = Vec::new();
    let mut run: Vec<Pos2> = Vec::new();
    for k in (first..first + n).map(|k| k % n) {
        match pieces[k] {
            Some([a, b]) if joined(k) && !run.is_empty() => {
                let _ = a;
                run.push(b);
            }
            Some([a, b]) => {
                if run.len() > 1 {
                    out.push((std::mem::take(&mut run), false));
                }
                run = vec![a, b];
            }
            None => {
                if run.len() > 1 {
                    out.push((std::mem::take(&mut run), false));
                }
                run.clear();
            }
        }
    }
    if run.len() > 1 {
        out.push((run, false));
    }
    out
}

/// The handles' colors (the kit's handle tokens, one value in both
/// themes).
#[derive(Clone, Copy)]
pub(super) struct Colors {
    pub(super) axes: [(Color32, Color32); 3],
    pub(super) turn: (Color32, Color32),
    pub(super) scale: (Color32, Color32),
    /// The start location's ring and arrow, each idle and lit.
    pub(super) start: [(Color32, Color32); 2],
    /// The dark edge, black at [`EDGE`].
    pub(super) edge: Color32,
}

impl Colors {
    pub(super) fn of(t: &Tokens) -> Colors {
        Colors {
            axes: [
                (t.handle_x, t.handle_x_lit),
                (t.handle_y, t.handle_y_lit),
                (t.handle_z, t.handle_z_lit),
            ],
            turn: (t.handle_turn, t.handle_turn_lit),
            scale: (t.handle_scale, t.handle_scale_lit),
            start: [
                (t.handle_start_ring, t.handle_start_ring_lit),
                (t.handle_start_arrow, t.handle_start_arrow_lit),
            ],
            edge: t.handle_edge,
        }
    }

    /// The color of a handle: its idle or its lit one.
    pub(super) fn of_handle(&self, h: &Handle, lit: bool) -> Color32 {
        let pair = match h {
            Handle::Arrow(a) => self.axes[a.axis],
            Handle::Tilt(r) => self.axes[r.axis],
            Handle::Scale(..) => self.scale,
            _ => self.turn,
        };
        if lit { pair.1 } else { pair.0 }
    }
}

/// Which handles are lit: the one under the pointer, or those a drag
/// holds (a turn holds every turning ring, a tilt every ring of its
/// axis, a slide every arrow of its axis, a scale every square).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Lit {
    One(Handle),
    Turning,
    Tilting(usize),
    Sliding(usize),
    Scaling,
}

impl Lit {
    /// What a drag under way holds, if it is a handle's.
    pub(super) fn held(drag: Option<Drag>) -> Option<Lit> {
        Some(match drag? {
            Drag::Spin { .. } => Lit::Turning,
            Drag::Tilt { ring, .. } => Lit::Tilting(ring.axis),
            Drag::Slide { axis, .. } => Lit::Sliding(axis),
            Drag::Scale { .. } => Lit::Scaling,
            Drag::FaceSpawn { object, point, .. } => Lit::One(Handle::SpawnTip(object, point)),
            Drag::MoveSpawn { object, point, .. } => Lit::One(Handle::SpawnFoot(object, point)),
            Drag::Start { turn: true, .. } => Lit::One(Handle::StartTip),
            Drag::Start { turn: false, .. } => Lit::One(Handle::StartBody),
            _ => return None,
        })
    }

    /// Whether `h` is lit.
    pub(super) fn lights(lit: Option<Lit>, h: &Handle) -> bool {
        match (lit, h) {
            (Some(Lit::One(one)), h) => one.same(h),
            (Some(Lit::Turning), Handle::Ring(_)) => true,
            (Some(Lit::Tilting(axis)), Handle::Tilt(r)) => r.axis == axis,
            (Some(Lit::Sliding(axis)), Handle::Arrow(a)) => a.axis == axis,
            (Some(Lit::Scaling), Handle::Scale(..)) => true,
            _ => false,
        }
    }

    /// The strength `h` draws at: whole, or half while another of its set
    /// is lit (section 11.3: the hovered object's own set; a drag dims
    /// every set, as it leads them all). A spawn point or the start
    /// location under the pointer dims no set.
    pub(super) fn strength(lit: Option<Lit>, h: &Handle) -> f32 {
        let other_set = match (lit, h.object()) {
            (Some(Lit::One(one)), Some(of)) => one.object() != Some(of),
            _ => false,
        };
        if lit.is_none() || Lit::lights(lit, h) || other_set { 1.0 } else { DIMMED }
    }
}

/// The selection's handles: the turning rings with no key held, with
/// Shift the tilt rings, the arrows and the scale squares (and, during a
/// drag, as today: the dragged kind, and the turning ring during a scale
/// drag); each object's pivot dot. Today's readout beside what is
/// dragged.
#[allow(clippy::too_many_arguments)]
pub(super) fn selection(
    ui: &egui::Ui,
    view: &AreaView,
    shown: &AreaModel,
    lit: Option<Lit>,
    (t, colors): (&Tokens, &Colors),
    canvas: &mut Canvas,
    painter: &egui::Painter,
    segment: &dyn Fn(Vec3, Vec3) -> Option<[Pos2; 2]>,
) {
    let at = |p: Vec3| view.screen_pos(p);
    // (Where an object's pivot dot goes: about each object with a set.)
    let mut pivots = Pivots::default();
    let tilting = match view.drag {
        Some(Drag::Tilt { .. }) => true,
        Some(_) => false,
        None => ui.input(|i| i.modifiers.shift && !i.modifiers.command && !i.modifiers.alt),
    };
    let scaling = matches!(view.drag, Some(Drag::Scale { .. }));
    if scaling || (tilting && view.drag.is_none()) {
        let pointer = ui.ctx().pointer_latest_pos();
        for (of, pivot, handle) in view.scale_handles_of(shown) {
            let h = Handle::Scale(of, pivot);
            let on = Lit::lights(lit, &h);
            let strength = Lit::strength(lit, &h);
            let color = colors.of_handle(&h, on);
            let drawn = view.scale_handle_drawn(pivot, handle, pointer);
            let stalk = if on { STALK.1 } else { STALK.0 };
            let stalk_strength = strength * if on { 1.0 } else { 0.75 };
            match (scaling, drawn, at(pivot)) {
                (true, Some(c), Some(foot)) => {
                    canvas.path(vec![foot, c], false, stalk, color, stalk_strength, Lay::Front);
                }
                _ => {
                    if let Some([a, b]) = segment(pivot, handle) {
                        canvas.path(vec![a, b], false, stalk, color, stalk_strength, Lay::Front);
                    }
                }
            }
            // Dragged, a hollow square where it began.
            if scaling && let Some(c) = at(handle) {
                canvas.square(c, SQUARE.0, colors.scale.0, START, true);
            }
            if let Some(c) = drawn {
                canvas.square(c, if on { SQUARE.1 } else { SQUARE.0 }, color, strength, false);
            }
        }
        if scaling
            && let Some(c) = ui.ctx().pointer_latest_pos()
            && let Some((_, visual)) = view.scaled().first()
        {
            let text = format!("Scale {:.2}", visual.scale.x);
            canvas.chip(painter, t, c + SCALE_CHIP, text, colors.scale.0);
        }
    }
    let mut rings = if tilting { view.tilt_rings(shown) } else { Vec::new() };
    // (Tilted, the dragged ring alone, as the board draws it: the other
    // axis' rings go.)
    if let Some(Drag::Tilt { ring, .. }) = view.drag {
        rings.retain(|r| r.axis == ring.axis);
    }
    let eye = view.camera().map(|c| c.eye);
    for ring in &rings {
        let h = Handle::Tilt(*ring);
        let on = Lit::lights(lit, &h);
        let strength = Lit::strength(lit, &h);
        let color = colors.of_handle(&h, on);
        let width = if on { RING.1 } else { RING.0 };
        tilt_ring(ring, eye, segment, |points, closed, front| {
            let (strength, lay) =
                if front { (strength, Lay::Front) } else { (strength * FAR, Lay::Back) };
            canvas.path(points, closed, width, color, strength, lay);
        });
        if let Some(c) = at(ring.pivot) {
            pivots.add(c);
        }
    }
    // Tilted: the wedge in the ring's plane from where it was taken to
    // where the model is now (snapped as it is), a hollow dot where the
    // tilt began, a dot and the chip at the grip.
    if let Some(Drag::Tilt { ring, grip, .. }) = view.drag
        && let Some(&(i, visual)) = view.tilted().first()
    {
        let h = Handle::Tilt(ring);
        let was = view.model.as_ref().and_then(|m| m.objects.get(i)).and_then(|o| o.visual);
        let field = ring.field();
        // (The short way round: from 170 by 30 the field reads -160.)
        let by =
            wrapped((visual.rotate[field] - was.unwrap_or_default().rotate[field]).to_radians());
        let (idle, bright) = (colors.of_handle(&h, false), colors.of_handle(&h, true));
        wedge(canvas, &at, ring.pivot, |a| ring.point(a), (grip, grip + by), idle);
        if let Some(c) = at(ring.point(grip)) {
            hollow_dot(canvas, c, 3.0, idle);
        }
        if let Some(c) = at(ring.point(grip + by)) {
            canvas.dot(c, 3.5, bright, 1.0, Lay::Top);
            let name = ["X", "Y", "Z"][field];
            let text = format!("{name} Rotation {:.0}°", visual.rotate[field]);
            canvas.chip(painter, t, c + RING_CHIP, text, idle);
        }
    }
    // The arrows for moving it along one axis: east red, north green, up
    // blue; while one is dragged, it alone.
    let sliding = match view.drag {
        Some(Drag::Slide { axis, .. }) => Some(axis),
        _ => None,
    };
    // Slid: a guide line along the axis through the object across the
    // view, and a hollow dot where it began.
    let arrows =
        (sliding.is_some() || (tilting && view.drag.is_none())).then(|| view.axis_arrows(shown));
    if let Some(Drag::Slide { axis, pivot: began, .. }) = view.drag {
        let color = colors.axes[axis].0;
        let along = Vec3::AXES[axis] * GUIDE_REACH;
        if let Some(arrows) = &arrows {
            for arrow in arrows.iter().filter(|a| a.axis == axis) {
                if let Some(points) = segment(arrow.pivot - along, arrow.pivot + along) {
                    let stroke = Stroke::new(1.0, color.gamma_multiply(GUIDE));
                    canvas.add(Layer::Fill, Shape::LineSegment { points, stroke });
                }
            }
        }
        if let Some(c) = at(began) {
            hollow_dot(canvas, c, 3.5, color);
        }
    }
    if let Some(arrows) = arrows {
        for arrow in arrows {
            if sliding.is_some_and(|axis| axis != arrow.axis) {
                continue;
            }
            let h = Handle::Arrow(arrow);
            let on = Lit::lights(lit, &h);
            let strength = Lit::strength(lit, &h);
            let color = colors.of_handle(&h, on);
            let (neck, tip) = arrow.head();
            if let Some(c) = at(arrow.pivot) {
                pivots.add(c);
            }
            let shaft = if on { SHAFT.1 } else { SHAFT.0 };
            if let Some([a, b]) = segment(arrow.pivot, neck) {
                canvas.path(vec![a, b], false, shaft, color, strength, Lay::Front);
            }
            let (Some(n), Some(end)) = (at(neck), at(tip)) else { continue };
            // (Seen end on, the head has no way to point: its line alone.)
            let Some(along) = unit(end - n) else { continue };
            let size = if on { HEAD.1 } else { HEAD.0 };
            let base = end - along * size.0;
            // (The line to the head's base, a little into it; none where
            // the head is longer than the line.)
            if (base - n).dot(along) > 0.0 {
                let line = if on { HEAD_LINE.1 } else { HEAD_LINE.0 };
                canvas.path(vec![n, base + along * 0.5], false, line, color, strength, Lay::Front);
            }
            canvas.head(end, along, size, color, strength);
            if sliding.is_some() {
                let name = ["X", "Y", "Z"][arrow.axis];
                let text = format!("{name} {:.2} m", arrow.pivot[arrow.axis]);
                canvas.chip(painter, t, end + ARROW_CHIP, text, colors.axes[arrow.axis].0);
            }
        }
    }
    if !tilting
        && !matches!(
            view.drag,
            Some(Drag::Box { .. } | Drag::Move { .. } | Drag::Lift { .. } | Drag::Slide { .. })
        )
    {
        let spinning = match view.drag {
            Some(Drag::Spin { angle, .. }) => Some(angle),
            _ => None,
        };
        // (Turned: each ring as it was, for where the turn began; and the
        // turns a snapped drag lands on.)
        let was = match (spinning, view.model.as_ref()) {
            (Some(_), Some(m)) => view.turn_rings(m),
            _ => Vec::new(),
        };
        let landings = match spinning {
            Some(_) => view.snap_landings(),
            None => Vec::new(),
        };
        let rings = view.turn_rings(shown);
        // (Each ring's lines on screen, kept while the view and the rings
        // are as they were: 5,000 selected, their 96 pieces each were put
        // on screen again every frame.)
        let key = super::OverlayKey::of(view);
        let kept = view.rings_kept.borrow().as_ref().and_then(|(at, was, lines)| {
            (key == Some(*at) && *was == rings).then(|| lines.clone())
        });
        let lines = kept.unwrap_or_else(|| {
            let step = std::f32::consts::TAU / RING_DRAWN as f32;
            let lines: std::sync::Arc<Vec<RingLines>> = std::sync::Arc::new(
                (rings.iter())
                    .map(|ring| {
                        let points: Vec<Vec3> =
                            (0..RING_DRAWN).map(|k| ring.point(k as f32 * step)).collect();
                        runs(&points, segment)
                    })
                    .collect(),
            );
            if let Some(at) = key {
                *view.rings_kept.borrow_mut() = Some((at, rings.clone(), lines.clone()));
            }
            lines
        });
        for (k, ring) in rings.into_iter().enumerate() {
            let h = Handle::Ring(ring);
            let on = Lit::lights(lit, &h);
            let strength = Lit::strength(lit, &h);
            let color = colors.of_handle(&h, on);
            let width = if on { RING.1 } else { RING.0 };
            for (points, closed) in &lines[k] {
                canvas.path(points.clone(), *closed, width, color, strength, Lay::Front);
            }
            // (Where a facing's triangle stands on the ring, and which
            // way is out there.)
            let mark = |a: f32| {
                let ahead = ring.point(a);
                let c = at(ahead)?;
                Some((c, unit(at(ring.pivot + (ahead - ring.pivot) * 1.3)? - c)?))
            };
            // Turned: the wedge from the facing at the press to the
            // facing now, a hollow triangle where it began, ticks at the
            // Turn snap's steps.
            if let (Some(angle), Some(before)) = (spinning, was.get(k)) {
                let idle = colors.turn.0;
                // (The turn as far round as the drag took it, past a
                // half turn too.)
                let turned = angle + wrapped(ring.facing - before.facing - angle);
                let from = ring.facing - turned;
                wedge(canvas, &at, ring.pivot, |a| ring.point(a), (from, ring.facing), idle);
                if let Some((c, out)) = mark(from) {
                    canvas.facing(c, out, FACING.0, idle, START, true, Lay::Top);
                }
                for delta in &landings {
                    // (Each ring's own facing at the press, turned by
                    // as much as the first object's snapped turns are.)
                    if let Some((c, out)) = mark(before.facing + delta) {
                        canvas.path(vec![c, c + out * TICK], false, 1.0, idle, 1.0, Lay::Front);
                    }
                }
            }
            if let Some((c, out)) = mark(ring.facing) {
                let size = if on { FACING.1 } else { FACING.0 };
                canvas.facing(c, out, size, color, strength, false, Lay::Front);
                if spinning.is_some() {
                    let degrees = ring.facing.to_degrees().rem_euclid(360.0);
                    let text = format!("{degrees:.0}°");
                    canvas.chip(painter, t, c + RING_CHIP, text, colors.turn.0);
                }
            }
            if let Some(c) = at(ring.pivot) {
                pivots.add(c);
            }
        }
    }
    for c in pivots.dots {
        canvas.dot(c, PIVOT, colors.scale.0, 1.0, Lay::Top);
    }
}

/// A ring's lines on screen: runs of points, each closed or not.
pub(super) type RingLines = Vec<(Vec<Pos2>, bool)>;

/// The pivots' dots to draw, each once: one within half a point of
/// another is the same. (Found by the half-point squares about it: each
/// dot looked for among all before it, 5,000 objects selected took 12.5
/// million comparisons a frame.)
#[derive(Default)]
struct Pivots {
    dots: Vec<Pos2>,
    near: std::collections::HashMap<(i32, i32), Vec<usize>>,
}

impl Pivots {
    fn add(&mut self, c: Pos2) {
        let cell = |p: Pos2| ((p.x / 0.5).floor() as i32, (p.y / 0.5).floor() as i32);
        let (x, y) = cell(c);
        let taken = (x - 1..=x + 1).any(|i| {
            (y - 1..=y + 1).any(|j| {
                self.near
                    .get(&(i, j))
                    .is_some_and(|at| at.iter().any(|&k| self.dots[k].distance(c) < 0.5))
            })
        });
        if !taken {
            self.near.entry((x, y)).or_default().push(self.dots.len());
            self.dots.push(c);
        }
    }
}

/// Where a readout chip's bottom-left corner stands (the proposal's
/// section 11.5): from a ring's dot, an arrow's tip, the pointer (scale).
pub(super) const RING_CHIP: Vec2 = egui::vec2(10.0, -10.0);
const ARROW_CHIP: Vec2 = egui::vec2(10.0, -4.0);
pub(super) const SCALE_CHIP: Vec2 = egui::vec2(12.0, -12.0);

/// `a` (radians) within half a turn of 0.
fn wrapped(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (a + PI).rem_euclid(TAU) - PI
}

/// A drag's wedge: a fan from `pivot` over the ring's points (`point` at
/// an angle) from one angle to the other, in `color` at [`WEDGE`].
fn wedge(
    canvas: &mut Canvas,
    at: &dyn Fn(Vec3) -> Option<Pos2>,
    pivot: Vec3,
    point: impl Fn(f32) -> Vec3,
    (from, to): (f32, f32),
    color: Color32,
) {
    let step = std::f32::consts::TAU / RING_DRAWN as f32;
    let pieces = ((to - from).abs() / step).ceil() as usize;
    let Some(middle) = at(pivot).filter(|_| pieces > 0) else { return };
    let rim: Option<Vec<Pos2>> =
        (0..=pieces).map(|k| at(point(from + (to - from) * k as f32 / pieces as f32))).collect();
    let Some(rim) = rim else { return };
    let fill = color.gamma_multiply(WEDGE);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(middle, fill);
    for p in &rim {
        mesh.colored_vertex(*p, fill);
    }
    for k in 1..rim.len() as u32 {
        mesh.add_triangle(0, k, k + 1);
    }
    canvas.add(Layer::Fill, Shape::mesh(mesh));
}

/// A hollow dot where a drag began: a ring of radius `r`, 1.5 points over
/// its edge, at [`START`].
fn hollow_dot(canvas: &mut Canvas, c: Pos2, r: f32, color: Color32) {
    let dark = canvas.edge(START);
    canvas.add(Layer::TopEdge, Shape::circle_stroke(c, r, Stroke::new(3.5, dark)));
    let stroke = Stroke::new(1.5, color.gamma_multiply(START));
    canvas.add(Layer::Top, Shape::circle_stroke(c, r, stroke));
}

/// A tilt ring as `draw` is given it: its runs on screen, each in front
/// of its pivot as seen from `eye` or behind it (the half nearer the eye
/// is the front; without an eye, all of it).
fn tilt_ring(
    ring: &TiltRing,
    eye: Option<Vec3>,
    segment: &dyn Fn(Vec3, Vec3) -> Option<[Pos2; 2]>,
    mut draw: impl FnMut(Vec<Pos2>, bool, bool),
) {
    let step = std::f32::consts::TAU / RING_DRAWN as f32;
    let Some(eye) = eye else {
        let points: [Vec3; RING_DRAWN] = std::array::from_fn(|k| ring.point(k as f32 * step));
        for (points, closed) in runs(&points, segment) {
            draw(points, closed, true);
        }
        return;
    };
    // Its halves split where its points are level with the pivot as seen
    // from the eye: from the angle where the ring's point is the nearest
    // the eye's side, a quarter turn either way (each half then exactly
    // half the pieces).
    let toward = eye - ring.pivot;
    let near = toward.dot(ring.v).atan2(toward.dot(ring.u));
    let quarter = RING_DRAWN / 4;
    for (from, front) in [(RING_DRAWN / 2, false), (0, true)] {
        let points: [Vec3; RING_DRAWN / 2 + 1] = std::array::from_fn(|k| {
            ring.point(near + (from + k) as f32 * step - quarter as f32 * step)
        });
        // (A half is open: its runs without the closing piece.)
        for (points, _) in runs_open(&points, segment) {
            draw(points, false, front);
        }
    }
}

/// [`runs`] of an open line through `points`.
fn runs_open(
    points: &[Vec3],
    segment: &dyn Fn(Vec3, Vec3) -> Option<[Pos2; 2]>,
) -> Vec<(Vec<Pos2>, bool)> {
    let mut out = Vec::new();
    let mut run: Vec<Pos2> = Vec::new();
    for w in points.windows(2) {
        match segment(w[0], w[1]) {
            Some([a, b]) if run.last().is_some_and(|l| l.distance(a) < 0.01) => run.push(b),
            Some([a, b]) => {
                if run.len() > 1 {
                    out.push((std::mem::take(&mut run), false));
                }
                run = vec![a, b];
            }
            None => {
                if run.len() > 1 {
                    out.push((std::mem::take(&mut run), false));
                }
                run.clear();
            }
        }
    }
    if run.len() > 1 {
        out.push((run, false));
    }
    out
}

/// Lines on screen, each with its stroke.
type Lines = Vec<([Pos2; 2], Stroke)>;

/// The selection box's edges: each a line of [`BOX`] in the selection's
/// color over its dark edge (the edges all under the lines).
pub(super) fn box_lines(edges: impl Iterator<Item = [Pos2; 2]>, colors: &Colors) -> (Lines, Lines) {
    let dark = Stroke::new(BOX.0 + 2.0, colors.edge.gamma_multiply(BOX.2 / EDGE));
    let line = Stroke::new(BOX.0, colors.turn.0.gamma_multiply(BOX.1));
    edges.map(|e| ((e, dark), (e, line))).unzip()
}

/// A line of the spawn point's or the start location's drawing over its
/// dark edge (selected: [`Canvas::path`]'s), `width` in `color`.
pub(super) fn edged_line(canvas: &mut Canvas, points: [Pos2; 2], width: f32, color: Color32) {
    canvas.path(points.to_vec(), false, width, color, 1.0, Lay::Front);
}

/// A name chip for `h` beside the pointer at `at`, in the handle's own
/// color (the gizmo rulings' 4).
pub(super) fn name_chip(
    canvas: &mut Canvas,
    painter: &egui::Painter,
    t: &Tokens,
    colors: &Colors,
    h: &Handle,
    at: Pos2,
) {
    // (The start location's in the turning ring's amber, as its readout:
    // its round handle's color.)
    let strip = colors.of_handle(h, false);
    canvas.chip(painter, t, at + NAME_CHIP_OFFSET, h.name().to_owned(), strip);
}

/// The spawn point's and the start location's round handle: radius
/// `r` (a point larger lit) in the turning ring's color, over its edge.
pub(super) fn round_handle(canvas: &mut Canvas, colors: &Colors, c: Pos2, r: f32, lit: bool) {
    let (r, color) = if lit { (r + 1.5, colors.turn.1) } else { (r, colors.turn.0) };
    canvas.dot(c, r, color, 1.0, Lay::Front);
}

/// The spawn point's foot: a square `side` across (two larger lit) in the
/// turning ring's color, over its edge.
pub(super) fn foot_handle(canvas: &mut Canvas, colors: &Colors, c: Pos2, side: f32, lit: bool) {
    let (side, color) = if lit { (side + 2.0, colors.turn.1) } else { (side, colors.turn.0) };
    canvas.square(c, side, color, 1.0, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pivot within half a point of one drawn is drawn once, across the
    /// squares it is found by too, and one just past it is drawn.
    #[test]
    fn pivots_within_half_a_point_are_one() {
        let mut p = Pivots::default();
        for c in
            [(10.0, 10.0), (10.3, 10.2), (10.49, 9.99), (9.76, 10.0), (11.0, 10.0), (10.0, 10.6)]
        {
            p.add(egui::pos2(c.0, c.1));
        }
        let drawn: Vec<(f32, f32)> = p.dots.iter().map(|d| (d.x, d.y)).collect();
        assert_eq!(drawn, [(10.0, 10.0), (11.0, 10.0), (10.0, 10.6)]);
    }

    fn line(a: Vec3, b: Vec3) -> Option<[Pos2; 2]> {
        // (Behind the eye: y below 0.)
        (a.y >= -0.01 && b.y >= -0.01).then(|| [egui::pos2(a.x, a.y), egui::pos2(b.x, b.y)])
    }

    fn circle(n: usize) -> Vec<Vec3> {
        (0..n)
            .map(|k| {
                let a = std::f32::consts::TAU * k as f32 / n as f32;
                Vec3::new(a.cos() * 100.0, a.sin() * 100.0, 0.0)
            })
            .collect()
    }

    /// A ring all in front of the eye is one closed line; one the eye cuts
    /// is the open line in front, in one piece across where its points
    /// begin.
    #[test]
    fn a_ring_is_one_line_cut_where_it_passes_behind_the_eye() {
        let whole: Vec<Vec3> = circle(96).into_iter().map(|p| p + Vec3::Y * 200.0).collect();
        let r = runs(&whole, &line);
        assert_eq!(r.len(), 1);
        assert!(r[0].1, "closed");
        assert_eq!(r[0].0.len(), 96);
        let cut = runs(&circle(96), &line);
        assert_eq!(cut.len(), 1, "one run: {cut:?}");
        assert!(!cut[0].1, "open");
        assert!(cut[0].0.iter().all(|p| p.y >= -0.01));
        // (From 0 to 180 degrees, 49 points.)
        assert_eq!(cut[0].0.len(), 49);
        // Where what shows spans the ring's first point (from 270 degrees
        // on round to 90), still one line.
        let turned: Vec<Vec3> = circle(96).into_iter().map(|p| Vec3::new(p.y, p.x, 0.0)).collect();
        let across = runs(&turned, &line);
        assert_eq!(across.len(), 1, "one run: {across:?}");
        assert_eq!(across[0].0.len(), 49);
        assert!(across[0].0.iter().all(|p| p.y >= -0.01));
    }
}
