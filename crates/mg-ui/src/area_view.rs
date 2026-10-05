//! The area viewer: an area's tiles and objects drawn with its lighting,
//! a camera to move around it, objects to select, move, turn, raise and
//! delete, and blueprints from the palette to place, each change one
//! undoable command on the area's GIT.
//!
//! Aurora's bindings: a click selects, a drag on the ground selects what is
//! in the box, a drag moves the selection over the ground, Shift + right
//! drag turns it and Alt + drag raises or lowers it; Ctrl + drag moves the
//! camera, a right (or Ctrl + right) or middle drag turns it, the wheel zooms (Shift or
//! Ctrl: slowly), numpad 4, 6, 8, 2 (or W, A, S, D) move it, 7, 9, 1, 3 turn it and 5
//! looks straight down. With a blueprint chosen in the palette, a click
//! places it (Shift + click keeps it chosen; right click or Escape lets it
//! go); triggers and encounters are drawn point by point, a double click
//! closing the outline. Also: a middle drag turns the camera (Shift:
//! moves it), the arrow keys move it, Z and C (or Ctrl + Shift + the wheel
//! or a middle drag) move it up and down, Ctrl + click adds to the selection
//! and Delete deletes it.

use std::f32::consts::FRAC_PI_2;

use egui::{Color32, Pos2, Rect, Stroke};
use glam::{Vec2, Vec3};
use mg_area::pick::{Ray, pick, project};
use mg_area::walk::Ground;
use mg_area::{AreaModel, AreaScene, ObjectKind, TILE_SIZE, View};
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_gff::{Gff, Value};
use mg_render::{Camera, Targets};
use mg_resman::ResKey;
use mg_schema::{StructExt, ifo};
use mg_set::Tileset;

use crate::{Action, Moonglow};

/// Multisampling for the area view.
const SAMPLES: u32 = 4;

/// How far from a door hook a click places a door on it, metres.
const DOOR_REACH: f32 = 4.0;

/// The steepest the camera looks down (just short of straight down, where
/// "up" on screen would be undefined).
const MAX_PITCH: f32 = 1.5695;

/// The camera: orbiting a point on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
    pub target: Vec3,
    /// Around Z from +X, radians: −90° looks north.
    pub yaw: f32,
    /// Up from the ground, radians.
    pub pitch: f32,
    pub distance: f32,
}

impl Orbit {
    /// Looking straight down (north up) at the area's centre from where all
    /// of it shows, as Aurora first shows an area.
    pub fn overview(area: &AreaModel) -> Orbit {
        let (w, h) = area.size();
        let span = w.max(h).max(TILE_SIZE);
        Orbit {
            target: Vec3::new(w / 2.0, h / 2.0, 0.0),
            yaw: -FRAC_PI_2,
            pitch: MAX_PITCH,
            distance: span * 1.45,
        }
    }

    pub fn camera(&self) -> Camera {
        let mut c = Camera::orbit(self.target, self.distance, self.yaw, self.pitch);
        c.far = c.far.max(self.distance * 4.0 + 400.0);
        c
    }
}

/// Objects copied from an area (Ctrl+C), to paste in any area.
#[derive(Debug, Clone)]
pub struct ObjectClip {
    /// Each object as it was: its GIT struct, where it stood, and how high
    /// above the ground.
    pub objects: Vec<(mg_area::AreaObject, mg_gff::Struct, f32)>,
    /// Where the first stood: the others keep their places around it.
    pub anchor: Vec3,
}

/// The press the view has. egui ends a widget's drag when any button is
/// let go, so the release of a second button (the middle one's, after a
/// swing of the camera) would end what the first is doing: here a press
/// goes on until every button is up.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Held {
    on: bool,
    /// The left, right and middle buttons.
    down: [bool; 3],
    /// How far the pointer moved this frame.
    delta: egui::Vec2,
    pos: Option<Pos2>,
    /// Escape ended it: what was being done is dropped.
    cancelled: bool,
}

impl Held {
    fn read(ui: &egui::Ui, response: &egui::Response, was: Held) -> Held {
        use egui::PointerButton::{Middle, Primary, Secondary};
        ui.input(|i| {
            let escape = i.key_pressed(egui::Key::Escape);
            let on = response.dragged() || (was.on && i.pointer.any_down() && !escape);
            Held {
                on,
                down: [Primary, Secondary, Middle].map(|b| on && i.pointer.button_down(b)),
                delta: if on { i.pointer.delta() } else { egui::Vec2::ZERO },
                pos: i.pointer.latest_pos(),
                cancelled: was.on && escape,
            }
        })
    }

    /// The press goes on with `button` down.
    pub(crate) fn by(&self, button: egui::PointerButton) -> bool {
        match button {
            egui::PointerButton::Primary => self.down[0],
            egui::PointerButton::Secondary => self.down[1],
            egui::PointerButton::Middle => self.down[2],
            _ => false,
        }
    }

    /// Where the pointer is.
    pub(crate) fn pos(&self) -> Option<Pos2> {
        self.pos
    }

    pub(crate) fn cancelled(&self) -> bool {
        self.cancelled
    }
}

/// What a drag does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    /// Moves the selection by the travel of the ground point under the
    /// pointer since `from` (where it was pressed).
    Move { from: Vec3, offset: Vec2 },
    /// Turns each selected object about itself.
    Turn { angle: f32 },
    /// Turns them as the ring is led around `pivot`: by the
    /// angle the pointer has gone round it since `grip`.
    Spin { pivot: Vec3, grip: f32, angle: f32 },
    /// Moves them along one axis (0 east, 1 north, 2 up) as that axis's
    /// arrow is led along it: by the pointer's travel along the line
    /// through `pivot` since `grip` (a distance along it).
    /// With `screen`, by the pointer's travel on screen from where it was
    /// pressed instead (each point of it times this): for an arrow that
    /// points at the camera, whose line the pointer can't follow.
    Slide { axis: usize, pivot: Vec3, grip: f32, by: f32, screen: Option<(Pos2, egui::Vec2)> },
    /// Tilts their models (the visual transform's rotation about X or Y)
    /// as a tilt ring is led round: by the angle the pointer has gone
    /// round it, in the ring's plane, since `grip`.
    /// `screen`, as for an arrow: for a ring seen edge-on, the pointer's
    /// travel along the ring's line on screen.
    Tilt { ring: TiltRing, grip: f32, angle: f32, screen: Option<(Pos2, egui::Vec2)> },
    /// Raises or lowers the selection (not creatures: they stand on the
    /// ground).
    Lift { by: f32 },
    /// Turns spawn point `point` of the encounter at `object` (its place
    /// in the model) to face the pointer: `facing`, radians.
    FaceSpawn { object: usize, point: usize, pivot: Vec3, facing: f32 },
    /// Moves the start location (held by its ring or its arrow's shaft,
    /// `grip` from its middle) or, held by the arrow's tip, turns it:
    /// where it is and faces now, and where it was.
    Start { turn: bool, grip: Vec2, at: Vec3, facing: f32, was: (Vec3, f32) },
    /// Selects what is inside the box.
    Box { from: Pos2, to: Pos2 },
}

/// The ring around the selection for turning it: taken anywhere on it and
/// led round with the pointer.
#[derive(Debug, Clone, Copy)]
struct TurnRing {
    /// The object's place: the ring lies level around it.
    pivot: Vec3,
    radius: f32,
    /// The way the object faces: a mark on the ring is there.
    facing: f32,
}

/// A ring for tilting the selection's models: upright around the first
/// one's, about the axis its visual transform turns it on. Shown while
/// Shift is held.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TiltRing {
    /// 0: about X, 1: about Y (of the visual transform's rotation).
    axis: usize,
    pivot: Vec3,
    radius: f32,
    /// The ring's plane: `u` × `v` is the axis it turns about.
    u: Vec3,
    v: Vec3,
}

impl TiltRing {
    fn point(&self, angle: f32) -> Vec3 {
        self.pivot + (self.u * angle.cos() + self.v * angle.sin()) * self.radius
    }

    fn segment(&self, k: usize) -> (Vec3, Vec3) {
        let at = |k: usize| self.point(std::f32::consts::TAU * k as f32 / RING_SEGMENTS as f32);
        (at(k), at(k + 1))
    }

    fn color(&self) -> Color32 {
        // Red about X, green about Y, as 3D editors have them.
        if self.axis == 0 {
            Color32::from_rgb(240, 90, 90)
        } else {
            Color32::from_rgb(130, 215, 90)
        }
    }
}

/// An arrow for moving the selection along one axis alone: east (red),
/// north (green) or up (blue), from the first selected object, taken by
/// its head. Shown while Shift is held, with the tilt rings.
#[derive(Debug, Clone, Copy, PartialEq)]
struct AxisArrow {
    /// 0 east, 1 north, 2 up.
    axis: usize,
    pivot: Vec3,
    radius: f32,
}

impl AxisArrow {
    fn along(&self) -> Vec3 {
        Vec3::AXES[self.axis]
    }

    /// Its head's ends: clear of the rings, which reach the radius.
    fn head(&self) -> (Vec3, Vec3) {
        let along = self.along() * self.radius;
        (self.pivot + along * 1.2, self.pivot + along * 1.7)
    }

    /// Red, green and blue, as 3D editors have X, Y and Z.
    fn color(&self) -> Color32 {
        [
            Color32::from_rgb(240, 90, 90),
            Color32::from_rgb(130, 215, 90),
            Color32::from_rgb(90, 150, 255),
        ][self.axis]
    }
}

/// The most selected objects that get rings and arrows of their own.
const TOOLS_MAX: usize = 32;

/// How near the ring's line the pointer takes it, points: it is a thin
/// target otherwise.
const RING_REACH: f32 = 11.5;

/// The straight pieces the ring is drawn (and taken) as.
const RING_SEGMENTS: usize = 48;

impl TurnRing {
    fn point(&self, angle: f32) -> Vec3 {
        self.pivot + Vec3::new(angle.cos(), angle.sin(), 0.0) * self.radius
    }

    /// The ends of its `k`th piece.
    fn segment(&self, k: usize) -> (Vec3, Vec3) {
        let at = |k: usize| self.point(std::f32::consts::TAU * k as f32 / RING_SEGMENTS as f32);
        (at(k), at(k + 1))
    }
}

/// A brush cursor's outlines (their points on the ground) and colours.
pub type CursorShapes = Vec<(Vec<Vec3>, Color32)>;

/// An open area's view.
#[derive(Debug)]
pub struct AreaView {
    pub area: ResRef,
    pub model: Option<AreaModel>,
    scene: Option<AreaScene>,
    /// The workspace revision `model` was read at.
    revision: Option<u64>,
    tileset: Option<(ResRef, Option<Tileset>)>,
    /// The tileset's tile index and rules, for painting.
    pub(crate) terrain: Option<crate::terrain_mode::Tools>,
    /// Where the pointer is, for the tileset brushes.
    pub(crate) spot: Option<crate::terrain_mode::Spot>,
    /// The quarters of tiles a crosser drag has passed (tile and edge).
    pub(crate) crossing: Vec<((u32, u32), usize)>,
    /// Where the crosser drag's pointer was last (on the ground).
    pub(crate) crossing_at: Option<Vec3>,
    /// Shift is held in a crosser drag: the tile under the pointer, the
    /// far corner of the rectangle whose outline the crosser follows.
    pub(crate) crossing_outline: Option<(u32, u32)>,
    /// The walkmesh is drawn over the view (Aurora's Render AABB Nodes).
    pub walkmesh: bool,
    /// surfacemat.2da's `Walk` by row, read once.
    walkable: Option<Vec<bool>>,
    /// Placeables' and doors' walkmeshes (`.pwk`, `.dwk`) are drawn over
    /// the view.
    pub object_walkmesh: bool,
    object_walks: mg_area::walk::ObjectWalkmeshes,
    /// Their faces in the area, with each one's object; read again when the
    /// area is.
    object_faces: Option<Vec<([Vec3; 3], usize)>>,
    /// Tiles are selected rather than objects (Aurora's Select Terrain).
    pub tile_mode: bool,
    /// The selected tiles (column, row).
    pub tile_selection: Vec<(u32, u32)>,
    /// The box being dragged to select tiles (screen points).
    pub(crate) tile_box: Option<(Pos2, Pos2)>,
    /// Copied tiles follow the pointer, to be placed with a click.
    pub tile_pasting: bool,
    /// Where the tile menu was opened.
    pub(crate) tile_menu_at: Option<Pos2>,
    /// How far a tile group being placed is turned (quarter turns).
    pub(crate) group_turns: u8,
    /// Why the last stroke did nothing.
    pub(crate) notice: Option<String>,
    /// The tiles' walkmeshes.
    ground: Option<Ground>,
    pub error: Option<String>,
    pub orbit: Option<Orbit>,
    /// Selected objects: kind and index in its GIT list.
    pub selection: Vec<(ObjectKind, usize)>,
    pub night: bool,
    pub fog: bool,
    /// Lit by the area's lighting (off: an even working light).
    pub lit: bool,
    /// Circles where each placed sound is heard at full volume and where
    /// it stops.
    pub sound_ranges: bool,
    pub grid: bool,
    /// Which kinds of object are shown, by [`ObjectKind::index`].
    pub show: [bool; 9],
    pub show_start: bool,
    /// Whether the start location's marker is selected (with no object).
    pub start_selected: bool,
    drag: Option<Drag>,
    /// The press on the view, as of this frame.
    pub(crate) held: Held,
    /// Options › Area: the turning ring shows around the selection.
    pub turn_ring: bool,
    /// The outline being drawn for a trigger or encounter.
    pub outline: Vec<Vec3>,
    /// The copied objects follow the pointer, to be placed with a click.
    pub pasting: bool,
    /// How many of them were drawn, see-through, last frame.
    pub pasted_shown: usize,
    /// The blueprint about to be placed (dragged over the view, or chosen
    /// in the palette), as an object to show where it would go; `None` in
    /// it when it can't be placed.
    ghost: Option<(ResKey, Option<mg_area::AreaObject>)>,
    /// The blueprint about to be placed as the last frame showed it.
    pub ghost_shown: Option<mg_area::AreaObject>,
    /// How far Q and E have turned the blueprint about to be placed
    /// (radians, anticlockwise); it is placed so.
    ghost_turn: f32,
    /// Where the context menu was opened (on the ground).
    menu_at: Option<Vec3>,
    /// Snapping (the settings', copied each frame): grid in meters, angle in
    /// degrees.
    snap: (Option<f32>, Option<f32>),
    /// The ground point under the pointer, shown in the corner.
    pub pointer: Option<Vec3>,
    /// The tileset brush's cursor as the last frame drew it: its shapes'
    /// points on the ground, and their colours.
    pub brush_cursor: CursorShapes,
    /// The tiles a tile brush's click would make, as the last frame showed
    /// them under the pointer.
    pub tile_preview: Vec<mg_area::AreaTile>,
    /// Chooses among the tiles that fit for the preview, and so for the
    /// click it shows (then anew).
    pub(crate) preview_seed: u64,
    /// The last tile preview worked out, and what it was worked out for.
    pub(crate) preview_cache:
        Option<(crate::terrain_mode::PreviewKey, Option<crate::terrain_mode::Preview>)>,
    /// The brush's cursor last worked out, and what for.
    pub(crate) cursor_cache: Option<(crate::terrain_mode::PreviewKey, CursorShapes)>,
    /// A terrain brush's drag: the corners it is painting.
    pub(crate) terrain_drag: Option<crate::terrain_mode::TerrainDrag>,
    /// A trigger or encounter whose outline is being drawn anew.
    pub redraw: Option<(ObjectKind, usize)>,
    /// The Create Set window's name, while it is open.
    pub set_name: Option<String>,
    targets: Option<(Targets, egui::TextureId)>,
    time: f32,
    last_frame: Option<f64>,
    /// Where the view was last drawn, in points.
    pub rect: Rect,
}

impl AreaView {
    /// Reads the area's tiles and objects again from the game data (after
    /// Reload Resources), keeping the camera.
    pub(crate) fn reload(&mut self) {
        self.revision = None;
        self.tileset = None;
        self.terrain = None;
        self.scene = None;
        self.ground = None;
        self.walkable = None;
        self.object_walks = Default::default();
        self.object_faces = None;
    }

    fn new(area: ResRef) -> AreaView {
        AreaView {
            area,
            model: None,
            scene: None,
            revision: None,
            tileset: None,
            terrain: None,
            spot: None,
            crossing: Vec::new(),
            crossing_at: None,
            crossing_outline: None,
            walkmesh: false,
            walkable: None,
            object_walkmesh: false,
            object_walks: Default::default(),
            object_faces: None,
            tile_mode: false,
            tile_selection: Vec::new(),
            tile_box: None,
            tile_pasting: false,
            tile_menu_at: None,
            group_turns: 0,
            notice: None,
            ground: None,
            error: None,
            orbit: None,
            selection: Vec::new(),
            night: false,
            fog: false,
            lit: true,
            sound_ranges: false,
            grid: true,
            show: [true; 9],
            show_start: true,
            start_selected: false,
            drag: None,
            held: Held::default(),
            turn_ring: true,
            outline: Vec::new(),
            pasting: false,
            pasted_shown: 0,
            ghost: None,
            ghost_shown: None,
            ghost_turn: 0.0,
            menu_at: None,
            snap: (None, None),
            pointer: None,
            brush_cursor: Vec::new(),
            terrain_drag: None,
            tile_preview: Vec::new(),
            preview_seed: fastrand::u64(..),
            preview_cache: None,
            cursor_cache: None,
            redraw: None,
            set_name: None,
            targets: None,
            time: 0.0,
            last_frame: None,
            rect: Rect::NOTHING,
        }
    }

    fn git(&self) -> ResKey {
        ResKey::new(self.area, ResType::GIT)
    }

    /// The camera.
    pub fn camera(&self) -> Option<Camera> {
        self.orbit.map(|o| o.camera())
    }

    /// Where a point shows on screen (last drawn view).
    pub fn screen_pos(&self, p: Vec3) -> Option<Pos2> {
        let camera = self.camera()?;
        let at = project(&camera, self.aspect(), p)?;
        Some(self.rect.min + egui::vec2(at.x * self.rect.width(), at.y * self.rect.height()))
    }

    fn aspect(&self) -> f32 {
        (self.rect.width() / self.rect.height().max(1.0)).max(1e-3)
    }

    fn ray(&self, pos: Pos2) -> Option<Ray> {
        let camera = self.camera()?;
        let at = Vec2::new(
            (pos.x - self.rect.min.x) / self.rect.width().max(1.0),
            (pos.y - self.rect.min.y) / self.rect.height().max(1.0),
        );
        Some(Ray::from_screen(&camera, self.aspect(), at))
    }

    /// The position of object (kind, index) in `model.objects`.
    fn object_at(&self, kind: ObjectKind, index: usize) -> Option<usize> {
        self.model.as_ref()?.objects.iter().position(|o| o.kind == kind && o.index == index)
    }

    fn selected(&self, i: usize) -> bool {
        let Some(o) = self.model.as_ref().and_then(|m| m.objects.get(i)) else { return false };
        self.selection.contains(&(o.kind, o.index))
    }

    /// The object under the pointer.
    fn pick(&self, pos: Pos2) -> Option<usize> {
        let (model, scene, ray) = (self.model.as_ref()?, self.scene.as_ref()?, self.ray(pos)?);
        let found = pick(model, &ray, &|i| scene.bounds(model, i), &|k| self.show[k.index()])?;
        // Locked objects can't be picked.
        (!model.objects[found].locked).then_some(found)
    }

    /// The ground point under the pointer: on the walkmesh, else on the
    /// plane at height `z`.
    pub(crate) fn ground_at(&self, pos: Pos2, z: f32) -> Option<Vec3> {
        let ray = self.ray(pos)?;
        self.ground.as_ref().and_then(|g| g.hit(&ray)).or_else(|| ray.at_height(z))
    }

    /// The ground's height at `p` (on its walkmesh, nearest the level of
    /// the tile `p` is in), else `p`'s own: where the brushes' cursors are
    /// drawn, so that they lie on the ground the pointer picks.
    pub(crate) fn ground_height(&self, p: Vec3) -> f32 {
        let Some(g) = &self.ground else { return p.z };
        let index = self.model.as_ref().and_then(|m| {
            let (x, y) = ((p.x / TILE_SIZE).floor(), (p.y / TILE_SIZE).floor());
            let inside = x >= 0.0 && y >= 0.0 && (x as u32) < m.width && (y as u32) < m.height;
            inside.then(|| (y as u32 * m.width + x as u32) as usize)
        });
        let near = index.and_then(|i| g.tile_level(i)).unwrap_or(p.z);
        g.height(p.truncate(), near).unwrap_or(near)
    }

    /// A shape's corners laid on the ground ([`ground_height`](Self::ground_height),
    /// a little above it): each corner's height taken a little inside the
    /// shape, so that at a cliff's edge it takes this side's. Some
    /// tilesets build their ground above the tiles' heights, and slopes and
    /// cliffs are not at a corner's.
    pub(crate) fn on_ground(&self, points: &[Vec3]) -> Vec<Vec3> {
        let middle = points.iter().copied().sum::<Vec3>() / points.len().max(1) as f32;
        points
            .iter()
            .map(|p| p.truncate().extend(self.ground_height(p.lerp(middle, 0.02)) + 0.05))
            .collect()
    }

    /// Where an object moved by `offset` stands: at the same height above
    /// the ground as before (on it, for most; creatures always on it).
    /// Outlines keep their height.
    fn moved(&self, o: &mg_area::AreaObject, offset: Vec2) -> Vec3 {
        let to = o.position.truncate() + offset;
        let ground = self.ground.as_ref().filter(|_| !o.kind.has_outline());
        let Some(g) = ground else { return to.extend(o.position.z) };
        let ground_z = g.height(o.position.truncate(), o.position.z);
        let lift = match o.kind {
            ObjectKind::Creature => 0.0,
            _ => ground_z.map_or(0.0, |z| o.position.z - z),
        };
        let near = o.position.z - lift;
        g.height(to, near).map_or(to.extend(o.position.z), |z| to.extend(z + lift))
    }

    /// A point snapped to the grid, on the ground there.
    fn snapped(&self, at: Vec3) -> Vec3 {
        if self.snap.0.is_none() {
            return at;
        }
        let xy = mg_area::arrange::snap_point(at.truncate(), self.snap.0);
        let z = self.ground.as_ref().and_then(|g| g.height(xy, at.z)).unwrap_or(at.z);
        xy.extend(z)
    }

    /// The selected objects that are shown and not locked, as `model` has
    /// them: at most [`TOOLS_MAX`], the first selected first (a selection of
    /// hundreds would bury the view in rings).
    fn chosen<'a>(&'a self, model: &'a AreaModel) -> impl Iterator<Item = &'a mg_area::AreaObject> {
        self.selection
            .iter()
            .filter_map(|&(k, i)| model.objects.iter().find(|o| o.kind == k && o.index == i))
            .filter(|o| self.show[o.kind.index()] && !o.locked)
            .take(TOOLS_MAX)
    }

    /// The rings for turning the selection, one around each selected
    /// object that turns (as `model` has it): about the same size on
    /// screen however far the camera is.
    fn turn_rings(&self, model: &AreaModel) -> Vec<TurnRing> {
        let Some(camera) = self.camera().filter(|_| self.turn_ring) else { return Vec::new() };
        self.chosen(model)
            .filter(|o| turns(o.kind))
            .map(|o| {
                let radius = ((camera.eye - o.position).length() * 0.0765).max(0.5);
                TurnRing { pivot: o.position, radius, facing: o.facing() }
            })
            .collect()
    }

    /// The ring whose line `pos` is on (or within [`RING_REACH`] of).
    fn ring_at(&self, pos: Pos2) -> Option<TurnRing> {
        self.turn_rings(self.model.as_ref()?).into_iter().find(|ring| self.on_ring(ring, pos))
    }

    fn on_ring(&self, ring: &TurnRing, pos: Pos2) -> bool {
        self.on_line(&|k| ring.segment(k), pos)
    }

    /// The rings for tilting the selection's models, about X and about Y:
    /// around each selected object whose model tilts (as `model` has it).
    /// Each lies across the axis its angle turns the model on, with the
    /// angles before it applied (Z, then Y, then X).
    fn tilt_rings(&self, model: &AreaModel) -> Vec<TiltRing> {
        let Some(camera) = self.camera().filter(|_| self.turn_ring) else { return Vec::new() };
        self.chosen(model)
            .filter(|o| o.takes_visual_transform())
            .flat_map(|o| {
                let visual = o.visual.unwrap_or_default();
                let r = visual.rotate * (std::f32::consts::PI / 180.0);
                let stands = glam::Quat::from_rotation_z(o.rotation);
                let about_y = stands * glam::Quat::from_rotation_z(r.z);
                let about_x = about_y * glam::Quat::from_rotation_y(r.y);
                let pivot = o.transform().transform_point3(visual.translate);
                let radius = ((camera.eye - pivot).length() * 0.0765).max(0.5);
                let ring = |axis: usize, n: Vec3| {
                    let u = n.any_orthonormal_vector();
                    TiltRing { axis, pivot, radius, u, v: n.cross(u) }
                };
                [ring(0, about_x * Vec3::X), ring(1, about_y * Vec3::Y)]
            })
            .collect()
    }

    /// The arrows for moving the selection along an axis (as `model` has
    /// it): east and north from each selected object, and up from each
    /// that lifts (not creatures, which stand on the ground, nor
    /// outlines).
    fn axis_arrows(&self, model: &AreaModel) -> Vec<AxisArrow> {
        let Some(camera) = self.camera().filter(|_| self.turn_ring) else { return Vec::new() };
        let mut arrows = Vec::new();
        for o in self.chosen(model) {
            let radius = ((camera.eye - o.position).length() * 0.0765).max(0.5);
            let axes = if lifts(o.kind) { 0..3 } else { 0..2 };
            arrows.extend(axes.map(|axis| AxisArrow { axis, pivot: o.position, radius }));
        }
        arrows
    }

    /// The arrow whose head `pos` is on.
    fn axis_arrow_at(&self, pos: Pos2) -> Option<AxisArrow> {
        self.axis_arrows(self.model.as_ref()?).into_iter().find(|arrow| {
            let (a, b) = arrow.head();
            let (Some(a), Some(b)) = (self.screen_pos(a), self.screen_pos(b)) else { return false };
            near_segment(a, b, pos) <= RING_REACH
        })
    }

    /// How far along the line from `pivot` the way `along` goes its point
    /// nearest the pointer's ray is (`None` looking straight down it).
    fn distance_along(&self, pivot: Vec3, along: Vec3, pos: Pos2) -> Option<f32> {
        let ray = self.ray(pos)?;
        let dir = ray.dir.normalize_or_zero();
        let (b, w) = (along.dot(dir), pivot - ray.origin);
        let across = 1.0 - b * b;
        (across > 0.02).then(|| (b * dir.dot(w) - along.dot(w)) / across)
    }

    /// The tilt ring whose line `pos` is on (the nearer the camera sees
    /// more of, when on both).
    fn tilt_ring_at(&self, pos: Pos2) -> Option<TiltRing> {
        let rings = self.tilt_rings(self.model.as_ref()?);
        let ray = self.ray(pos)?;
        rings.into_iter().filter(|r| self.on_line(&|k| r.segment(k), pos)).max_by(|a, b| {
            let facing = |r: &TiltRing| r.u.cross(r.v).dot(ray.dir).abs();
            facing(a).total_cmp(&facing(b))
        })
    }

    /// The angle around a tilt ring of the point under `pos`, in the
    /// ring's plane.
    fn angle_on(&self, ring: &TiltRing, pos: Pos2) -> Option<f32> {
        let ray = self.ray(pos)?;
        let n = ring.u.cross(ring.v);
        let toward = ray.dir.dot(n);
        // (Seen edge-on, the plane has no point under the pointer.)
        if toward.abs() < 0.05 {
            return None;
        }
        let t = (ring.pivot - ray.origin).dot(n) / toward;
        let d = ray.origin + ray.dir * t - ring.pivot;
        (t > 0.0 && d.length() > 1e-3).then(|| d.dot(ring.v).atan2(d.dot(ring.u)))
    }

    /// The spawn point whose arrow's tip is under `pos`, of a selected
    /// encounter: the encounter's place in the model, the point's in its
    /// list, and where the point is.
    fn spawn_arrow_at(&self, pos: Pos2) -> Option<(usize, usize, Vec3)> {
        let model = self.model.as_ref()?;
        model
            .objects
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                o.kind == ObjectKind::Encounter
                    && self.selected(*i)
                    && !o.locked
                    && self.show[o.kind.index()]
            })
            .flat_map(|(i, o)| o.spawn_points.iter().enumerate().map(move |(k, p)| (i, o, k, *p)))
            .find(|(_, o, k, p)| {
                let facing = o.spawn_facings.get(*k).copied().unwrap_or(0.0);
                self.screen_pos(spawn_arrow_tip(*p, facing))
                    .is_some_and(|tip| tip.distance(pos) <= RING_REACH)
            })
            .map(|(i, _, k, p)| (i, k, p))
    }

    /// Whether the start location's marker is what is selected.
    fn start_is_selected(&self) -> bool {
        self.start_selected && self.selection.is_empty()
    }

    /// How a press at `pos` takes hold of the start location's marker, at
    /// `p` facing `facing`: by the arrow's tip to turn it (true), by the
    /// ring or the arrow's lines to move it.
    fn start_grab(&self, (p, facing): (Vec3, f32), pos: Pos2) -> Option<bool> {
        if !self.show_start {
            return None;
        }
        let m = StartMarker::new(p, facing);
        // (The tip is a handle once the marker is selected.)
        if self.start_is_selected()
            && self.screen_pos(m.tip).is_some_and(|tip| tip.distance(pos) <= RING_REACH)
        {
            return Some(true);
        }
        let near = |a: Vec3, b: Vec3| {
            self.screen_pos(a)
                .zip(self.screen_pos(b))
                .is_some_and(|(a, b)| near_segment(a, b, pos) <= RING_REACH)
        };
        let step = std::f32::consts::TAU / RING_SEGMENTS as f32;
        let ring = |k: usize| m.around(k as f32 * step);
        (near(m.tail, m.tip)
            || near(m.guard.0, m.guard.1)
            || (0..RING_SEGMENTS).any(|k| near(ring(k), ring(k + 1))))
        .then_some(false)
    }

    /// How a press at `pos` leads a tilt ring: the angle taken hold of,
    /// and, for a ring seen too nearly edge-on to follow in its plane, how
    /// the pointer's travel on screen turns it (along the ring's line
    /// there).
    fn tilt_grip(&self, ring: &TiltRing, pos: Pos2) -> Option<(f32, Option<(Pos2, egui::Vec2)>)> {
        let facing = self.ray(pos)?.dir.normalize_or_zero().dot(ring.u.cross(ring.v)).abs();
        if facing >= 0.3
            && let Some(grip) = self.angle_on(ring, pos)
        {
            return Some((grip, None));
        }
        // The ring's point nearest the pointer, and its line there.
        let step = std::f32::consts::TAU / RING_SEGMENTS as f32;
        let (grip, at) = (0..RING_SEGMENTS)
            .map(|k| k as f32 * step)
            .filter_map(|a| Some((a, self.screen_pos(ring.point(a))?)))
            .min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)))?;
        let ahead = self.screen_pos(ring.point(grip + 0.05))?;
        let per_radian = (ahead - at) / 0.05;
        (per_radian.length() > 2.0)
            .then(|| (grip, Some((pos, per_radian / per_radian.length_sq()))))
    }

    /// How a press at `pos` leads an arrow: the distance along it taken
    /// hold of, and, for an arrow pointing too nearly at the camera, how
    /// the pointer's travel on screen moves it (along the arrow there, or
    /// up and down the screen for one seen end on).
    fn slide_grip(&self, arrow: &AxisArrow, pos: Pos2) -> (f32, Option<(Pos2, egui::Vec2)>) {
        let along = arrow.along();
        let toward = self.ray(pos).map_or(1.0, |r| r.dir.normalize_or_zero().dot(along).abs());
        if toward < 0.92
            && let Some(grip) = self.distance_along(arrow.pivot, along, pos)
        {
            return (grip, None);
        }
        let per_meter = self
            .screen_pos(arrow.pivot)
            .zip(self.screen_pos(arrow.pivot + along))
            .map(|(a, b)| b - a)
            .filter(|v| v.length() > 8.0);
        let per_point = match per_meter {
            Some(v) => v / v.length_sq(),
            // End on: up the screen is along it, as Alt + drag raises.
            None => egui::vec2(0.0, -self.orbit.map_or(0.02, |o| o.distance * 0.002)),
        };
        (0.0, Some((pos, per_point)))
    }

    /// Each selected object's visual transform with the tilt drag applied.
    fn tilted(&self) -> Vec<(usize, mg_area::VisualTransform)> {
        let (Some(model), Some(Drag::Tilt { ring, angle, .. })) = (&self.model, self.drag) else {
            return Vec::new();
        };
        let tilts: Vec<usize> = self
            .selection
            .iter()
            .filter_map(|&(k, i)| self.object_at(k, i))
            .filter(|&i| model.objects[i].takes_visual_transform() && !model.objects[i].locked)
            .collect();
        // Snapping turns the first to the angle; the others by as much.
        let mut by = angle.to_degrees();
        if let (Some(&first), Some(_)) = (tilts.first(), self.snap.1) {
            let was = model.objects[first].visual.unwrap_or_default().rotate[ring.axis];
            let to = mg_area::arrange::snap_rotation((was + by).to_radians(), self.snap.1);
            by = to.to_degrees() - was;
        }
        tilts
            .into_iter()
            .map(|i| {
                let mut v = model.objects[i].visual.unwrap_or_default();
                // Within half a turn either way, to a hundredth of a degree.
                let to = (v.rotate[ring.axis] + by + 180.0).rem_euclid(360.0) - 180.0;
                v.rotate[ring.axis] = (to * 100.0).round() / 100.0;
                (i, v)
            })
            .collect()
    }

    /// Whether `pos` is on the line of the pieces `segment` gives (or
    /// within [`RING_REACH`] of it).
    fn on_line(&self, segment: &dyn Fn(usize) -> (Vec3, Vec3), pos: Pos2) -> bool {
        (0..RING_SEGMENTS).any(|k| {
            let (a, b) = segment(k);
            let (Some(a), Some(b)) = (self.screen_pos(a), self.screen_pos(b)) else { return false };
            near_segment(a, b, pos) <= RING_REACH
        })
    }

    /// The angle around `pivot` of the point under `pos` (on the level
    /// plane through it).
    fn angle_about(&self, pivot: Vec3, pos: Pos2) -> Option<f32> {
        let d = self.ray(pos)?.at_height(pivot.z)? - pivot;
        (d.truncate().length() > 1e-3).then(|| d.y.atan2(d.x))
    }

    /// Where each selected object stands and turns with the drag applied.
    fn dragged(&self) -> Vec<(usize, Vec3, f32)> {
        let Some(model) = &self.model else { return Vec::new() };
        let moves = |d: &Drag| {
            !matches!(
                d,
                Drag::Box { .. } | Drag::Tilt { .. } | Drag::FaceSpawn { .. } | Drag::Start { .. }
            )
        };
        let Some(mut drag) = self.drag.filter(moves) else {
            return Vec::new();
        };
        if let Drag::Spin { angle, .. } = drag {
            drag = Drag::Turn { angle };
        }
        // An arrow's drag: along its axis alone.
        let mut locked = None;
        if let Drag::Slide { axis, pivot, by, .. } = drag {
            drag = if axis == 2 {
                Drag::Lift { by }
            } else {
                locked = Some(axis);
                Drag::Move { from: pivot, offset: Vec3::AXES[axis].truncate() * by }
            };
        }
        // Snapping moves and turns the first selected object to the grid or
        // angle; the others keep their places and turns relative to it.
        let first = self.selection.first().and_then(|&(k, i)| self.object_at(k, i));
        if let Some(o) = first.map(|i| &model.objects[i]) {
            use mg_area::arrange::{snap_point, snap_rotation};
            match &mut drag {
                Drag::Move { offset, .. } if self.snap.0.is_some() => {
                    let to = snap_point(o.position.truncate() + *offset, self.snap.0);
                    *offset = to - o.position.truncate();
                    if let Some(axis) = locked {
                        offset[1 - axis] = 0.0;
                    }
                }
                Drag::Turn { angle } if self.snap.1.is_some() => {
                    *angle = snap_rotation(o.rotation + *angle, self.snap.1) - o.rotation;
                }
                _ => {}
            }
        }
        self.selection
            .iter()
            .filter_map(|&(k, i)| self.object_at(k, i))
            .map(|i| {
                let o = &model.objects[i];
                match drag {
                    Drag::Move { offset, .. } => (i, self.moved(o, offset), o.rotation),
                    Drag::Turn { angle } | Drag::Spin { angle, .. } => {
                        (i, o.position, if turns(o.kind) { o.rotation + angle } else { o.rotation })
                    }
                    Drag::Lift { by } | Drag::Slide { by, .. } => {
                        let by = if lifts(o.kind) { by } else { 0.0 };
                        (i, o.position + Vec3::Z * by, o.rotation)
                    }
                    Drag::Box { .. }
                    | Drag::Tilt { .. }
                    | Drag::FaceSpawn { .. }
                    | Drag::Start { .. } => (i, o.position, o.rotation),
                }
            })
            .collect()
    }
}

/// The start location's marker: a ring on the ground and an arrow the way
/// the player faces, with a cross-guard.
struct StartMarker {
    middle: Vec3,
    tail: Vec3,
    tip: Vec3,
    ahead: Vec3,
    side: Vec3,
    guard: (Vec3, Vec3),
}

impl StartMarker {
    /// The arrow's barbs and half its cross-guard, meters.
    const BARB: f32 = START_RING * 0.35;

    fn new(p: Vec3, facing: f32) -> Self {
        let middle = p + Vec3::Z * 0.05;
        let ahead = Vec3::new(facing.cos(), facing.sin(), 0.0);
        let side = Vec3::new(-ahead.y, ahead.x, 0.0);
        Self {
            middle,
            tail: middle - ahead * START_RING * 0.75,
            tip: middle + ahead * START_RING * 0.75,
            ahead,
            side,
            guard: (middle - side * Self::BARB, middle + side * Self::BARB),
        }
    }

    /// The ring's point at angle `a`.
    fn around(&self, a: f32) -> Vec3 {
        self.middle + Vec3::new(a.cos(), a.sin(), 0.0) * START_RING
    }
}

/// The tip of a spawn point's arrow (a meter along its facing, a little
/// above the ground).
fn spawn_arrow_tip(point: Vec3, facing: f32) -> Vec3 {
    point + Vec3::Z * 0.1 + Vec3::new(facing.cos(), facing.sin(), 0.0)
}

/// Whether objects of `kind` are raised and lowered: not creatures, which
/// stand on the ground, nor outlines.
fn lifts(kind: ObjectKind) -> bool {
    kind != ObjectKind::Creature && !kind.has_outline()
}

/// How far `pos` is from the piece from `a` to `b`.
fn near_segment(a: Pos2, b: Pos2, pos: Pos2) -> f32 {
    let (ab, ap) = (b - a, pos - a);
    let t = if ab.length_sq() > 0.0 { (ap.dot(ab) / ab.length_sq()).clamp(0.0, 1.0) } else { 0.0 };
    (a + ab * t).distance(pos)
}

/// Reads the area again when the workspace changed.
fn refresh(app: &mut Moonglow, view: &mut AreaView) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
    if view.revision == Some(ws.revision()) {
        return;
    }
    view.revision = Some(ws.revision());
    let area = view.area;
    let started = std::time::Instant::now();
    let step = |what: &str| {
        crate::trace::note(format!("area {area}: {what} ({:.0?} in)", started.elapsed()));
    };
    step(&format!("reading at revision {}", ws.revision()));
    let are_key = ResKey::new(view.area, ResType::ARE);
    let are = match ws.doc(&are_key) {
        Ok(g) => g.clone(),
        Err(e) => {
            step(&format!("its ARE can't be read: {e}"));
            view.error = Some(e.to_string());
            return;
        }
    };
    // An area without a GIT has no objects yet.
    let git = ws.doc(&view.git()).cloned().unwrap_or_else(|_| Gff::new(*b"GIT "));
    let tileset_ref = are.root.resref("Tileset").unwrap_or(ResRef::EMPTY);
    step(&format!(
        "tileset {tileset_ref}, GIT {}",
        if ws.doc(&view.git()).is_ok() { "read" } else { "none" }
    ));
    if view.tileset.as_ref().is_none_or(|(r, _)| *r != tileset_ref) {
        let set = mg_area::tileset(game, tileset_ref);
        if let Err(e) = &set {
            step(&format!("the tileset can't be read: {e}"));
        }
        let set = set.ok();
        view.terrain = set
            .as_ref()
            .map(|t| crate::terrain_mode::Tools::new(tileset_ref, std::sync::Arc::new(t.clone())));
        view.tileset = Some((tileset_ref, set));
    }
    let tileset = view.tileset.as_ref().and_then(|(_, t)| t.as_ref());
    let model = AreaModel::read(game, &are.root, &git.root, tileset);
    step(&format!(
        "model read: {} by {} tiles, {} objects, tileset {}",
        model.width,
        model.height,
        model.objects.len(),
        if tileset.is_some() { "read" } else { "missing" }
    ));
    view.object_faces = None;
    match &mut view.ground {
        Some(g) => g.update(game, &model),
        None => view.ground = Some(Ground::new(game, &model)),
    }
    step("ground (walkmeshes) made");
    if view.orbit.is_none() {
        view.orbit = Some(Orbit::overview(&model));
        view.night = model.lighting.night_by_default();
    }
    if let Some(vp) = &app.viewport {
        match &mut view.scene {
            Some(s) => s.update(&vp.gpu, game, &model),
            None => view.scene = Some(AreaScene::new(&vp.gpu, game, &model)),
        }
        step("scene (models and textures) made");
    } else {
        step("no 3D viewport: no scene");
    }
    // Objects that went away are no longer selected.
    view.selection.retain(|&(k, i)| model.object(k, i).is_some());
    view.error = None;
    view.model = Some(model);
}

/// The module's start location, if it is in this area: position and facing.
fn start_location(app: &mut Moonglow, area: ResRef) -> Option<(Vec3, f32)> {
    let ws = app.ws.as_mut()?;
    let info = ws.doc(&ResKey::parse("module", ResType::IFO)?).ok()?;
    let r = &info.root;
    if r.read(&ifo::MOD_ENTRY_AREA) != area {
        return None;
    }
    let p =
        Vec3::new(r.read(&ifo::MOD_ENTRY_X), r.read(&ifo::MOD_ENTRY_Y), r.read(&ifo::MOD_ENTRY_Z));
    Some((p, r.read(&ifo::MOD_ENTRY_DIR_Y).atan2(r.read(&ifo::MOD_ENTRY_DIR_X))))
}

/// The start location as the view shows it: on the ground above or below
/// the stored point (a new module's start is at height 0, under the ground
/// of tilesets that build it higher; the game puts players on the walkmesh).
fn start_on_ground(app: &mut Moonglow, view: &AreaView) -> Option<(Vec3, f32)> {
    let (p, facing) = start_location(app, view.area)?;
    let z = view.ground.as_ref().and_then(|g| g.height(p.truncate(), p.z)).unwrap_or(p.z);
    Some((p.with_z(z), facing))
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui, area: ResRef) {
    let mut view = app.area_views.remove(&area).unwrap_or_else(|| {
        // Lighting and Sound Ranges as they were last left.
        let mut view = AreaView::new(area);
        view.lit = !app.settings.unlit_areas;
        view.sound_ranges = app.settings.sound_ranges;
        view
    });
    refresh(app, &mut view);
    crate::trace::changed(&format!("area {area} view"), || {
        format!(
            "in {:?}; error {:?}; model {}; tileset {}; ground {}; scene {}; camera {}; \
             drawn at {:?}; game data {}; 3D viewport {}",
            ui.max_rect(),
            view.error,
            view.model.as_ref().map_or("none".into(), |m| format!("{} objects", m.objects.len())),
            match &view.tileset {
                Some((r, Some(_))) => format!("{r} read"),
                Some((r, None)) => format!("{r} NOT read"),
                None => "none".into(),
            },
            view.ground.is_some(),
            view.scene.is_some(),
            view.orbit.is_some(),
            view.targets.as_ref().map(|(t, _)| t.size),
            app.game.is_some(),
            app.viewport.is_some(),
        )
    });
    app.palette.area = Some(area);
    view.snap = (
        app.settings.snap_grid.map(|cm| f32::from(cm) / 100.0),
        app.settings.snap_angle.map(f32::from),
    );
    // An object to go to (from Find Instance).
    if let Some((a, kind, index)) = app.area_focus
        && a == area
        && let Some(o) = view.model.as_ref().and_then(|m| m.object(kind, index))
    {
        let at = o.position;
        view.selection = vec![(kind, index)];
        if let Some(orbit) = &mut view.orbit {
            orbit.target = at;
            orbit.distance = orbit.distance.min(25.0);
        }
        app.area_focus = None;
    }
    toolbar(app, ui, &mut view);
    let start = start_on_ground(app, &view);
    if let Some(e) = &view.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    } else if app.game.is_none() {
        ui.label("No game data: the area viewer needs the game's tilesets and models.");
    } else if app.viewport.is_none() {
        ui.label("No GPU: the area viewer needs one.");
    } else {
        viewport(app, ui, &mut view, start);
    }
    set_window(app, ui, &mut view);
    // Its sounds are heard from where the view looks (Options › Sounds).
    if let Some(o) = &view.orbit {
        app.heard = Some((area, o.target, view.night));
    }
    app.area_views.insert(area, view);
}

fn toolbar(app: &mut Moonglow, ui: &mut egui::Ui, view: &mut AreaView) {
    ui.horizontal_wrapped(|ui| {
        use crate::icons::{self, labelled};
        for kind in ObjectKind::ALL {
            ui.toggle_value(
                &mut view.show[kind.index()],
                labelled(icons::object(kind), kind.plural()),
            )
            .on_hover_text(format!("Show {}", kind.plural()));
        }
        ui.toggle_value(&mut view.show_start, labelled(icons::START, "Start")).on_hover_text(
            "Show the start location's marker (a blue ring with a red arrow). To set it: \
                 right-click the ground, Set Start Location Here",
        );
        if ui.button("All").on_hover_text("Show All").clicked() {
            view.show = [true; 9];
            view.show_start = true;
        }
        if ui.button("None").on_hover_text("Show None").clicked() {
            view.show = [false; 9];
            view.show_start = false;
        }
        // Hidden objects cannot stay selected.
        let show = view.show;
        view.selection.retain(|(k, _)| show[k.index()]);
        ui.separator();
        ui.toggle_value(&mut view.night, labelled(icons::NIGHT, "Night"))
            .on_hover_text("Show the area at night");
        if ui
            .toggle_value(&mut view.lit, "💡 Lighting")
            .on_hover_text(
                "Use the area's lighting; off, everything is evenly lit (for working in dark \
                 areas)",
            )
            .changed()
        {
            app.settings.unlit_areas = !view.lit;
        }
        ui.toggle_value(&mut view.fog, labelled(icons::FOG, "Fog"));
        let mut animated = !app.settings.still_objects;
        if ui
            .toggle_value(&mut animated, "▶ Animations")
            .on_hover_text(
                "Creatures and other placed objects play their animations; off, they hold \
                 still (in every area view)",
            )
            .changed()
        {
            app.settings.still_objects = !animated;
        }
        ui.toggle_value(&mut view.grid, labelled(icons::GRID, "Grid"))
            .on_hover_text("Display Grid");
        // Aurora's Play Placed Sounds, Play Ambient Sound, Play Ambient
        // Music (the Options › Sounds settings).
        let mut placed = !app.settings.no_placed_sounds;
        if ui
            .toggle_value(&mut placed, "🔊 Sounds")
            .on_hover_text("Play placed sound objects in area")
            .changed()
        {
            app.settings.no_placed_sounds = !placed;
        }
        if ui
            .toggle_value(&mut view.sound_ranges, "◎ Sound Ranges")
            .on_hover_text(
                "Where each placed sound is heard: at full volume inside the inner circle, not \
                 at all outside the outer one",
            )
            .changed()
        {
            app.settings.sound_ranges = view.sound_ranges;
        }
        ui.toggle_value(&mut app.settings.ambient_sound, labelled(icons::AMBIENT, "Ambient"))
            .on_hover_text("Play ambient sound in area");
        ui.toggle_value(&mut app.settings.ambient_music, labelled(icons::MUSIC, "Music"))
            .on_hover_text("Play ambient music in area");
        ui.separator();
        let grid = |v: Option<u16>| {
            v.map_or("Snap: off".to_string(), |cm| format!("Snap: {} m", f32::from(cm) / 100.0))
        };
        egui::ComboBox::from_id_salt("snap-grid")
            .selected_text(grid(app.settings.snap_grid))
            .show_ui(ui, |ui| {
                for g in [None, Some(25), Some(50), Some(100), Some(250), Some(500)] {
                    ui.selectable_value(&mut app.settings.snap_grid, g, grid(g));
                }
            })
            .response
            .on_hover_text("Moved and placed objects snap to this grid");
        let angle = |v: Option<u16>| v.map_or("Turn: free".to_string(), |a| format!("Turn: {a}°"));
        egui::ComboBox::from_id_salt("snap-angle")
            .selected_text(angle(app.settings.snap_angle))
            .show_ui(ui, |ui| {
                for a in [None, Some(5), Some(15), Some(45), Some(90)] {
                    ui.selectable_value(&mut app.settings.snap_angle, a, angle(a));
                }
            })
            .response
            .on_hover_text("Turned objects snap to this angle; Q and E turn by it (15° when free)");
        ui.toggle_value(&mut view.walkmesh, labelled(icons::WALKMESH, "Walkmesh"))
            .on_hover_text("Render AABB Nodes: the ground's walkmesh, walkable faces green");
        let object_walkmeshes = labelled(icons::WALKMESH, "Object Walkmeshes");
        ui.toggle_value(&mut view.object_walkmesh, object_walkmeshes).on_hover_text(
            "Where placeables (.pwk) and doors (.dwk) keep creatures out: placeables orange, \
             doors blue (in their open or closed state)",
        );
        if ui
            .toggle_value(&mut view.tile_mode, labelled(icons::SELECT_TILES, "Select Tiles"))
            .on_hover_text(app.keymap.titled(
                "Select tiles rather than objects (Aurora's Select Terrain)",
                crate::keys::Cmd::SelectTiles,
                ui.ctx(),
            ))
            .changed()
        {
            view.selection.clear();
            view.tile_selection.clear();
        }
        if ui.button(labelled(icons::PROPERTIES, "Area Properties")).clicked() {
            app.actions.push(Action::OpenTab(crate::Tab::AreaProperties(view.area)));
        }
        if ui.button(labelled(icons::CAMERA, "Reorient Camera")).clicked()
            && let Some(o) = &mut view.orbit
        {
            o.yaw = -FRAC_PI_2;
        }
        if ui.button(labelled(icons::GO_TO_START, "Go to Start Location")).clicked()
            && let (Some((p, _)), Some(o)) = (start_on_ground(app, view), &mut view.orbit)
        {
            o.target = p;
            o.distance = o.distance.min(30.0);
        }
        let tip = crate::transfer::scratch_tip(
            app.settings.scratch_dir.as_deref(),
            "the area as it is now (its .are, .git and .gic)",
        );
        if ui.button("To Scratch").on_hover_text(tip).clicked() {
            app.actions.push(Action::ExportFiles {
                keys: vec![ResKey::new(view.area, ResType::ARE)],
                dependencies: false,
                scratch: true,
            });
        }
        // Several objects chosen: they can be kept as a group to place
        // again (the palette's Prefabs has them).
        if view.selection.len() > 1
            && ui
                .button(format!("Save {} as Prefab…", view.selection.len()))
                .on_hover_text(
                    "Keep the selected objects as a group, under a name, to place again in any \
                     area or module (the palette's Prefabs, or Edit › Prefabs)",
                )
                .clicked()
        {
            app.prefab_save = copy_selection(app, view).map(|clip| (String::new(), clip));
        }
        if let Some(m) = &view.model {
            ui.separator();
            let n = m.objects.iter().filter(|o| view.show[o.kind.index()]).count();
            ui.weak(format!("{} by {} tiles, {n} objects", m.width, m.height));
            if !m.problems.is_empty() {
                ui.colored_label(ui.visuals().warn_fg_color, "problems")
                    .on_hover_text(m.problems.join("\n"));
            }
            if let Some(s) = view.scene.as_ref().filter(|s| !s.missing.is_empty()) {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!("{} missing", s.missing.len()),
                )
                .on_hover_text(format!("Models not found:\n{}", s.missing.join("\n")));
            }
        }
    });
    // One line, never wrapped, so that the view does not move when the
    // selection changes.
    let status = if let Some(b) = crate::terrain_mode::active(app, view) {
        let how = match b.brush {
            mg_area::terrain::Brush::Crosser(_) => "drag across tiles",
            mg_area::terrain::Brush::RaiseLower => "click to raise, right click to lower",
            mg_area::terrain::Brush::Eraser => "click a tile (Shift + click: its next variant)",
            mg_area::terrain::Brush::Refine => "click a tile for the next that fits there",
            mg_area::terrain::Brush::Group(_) => "click to place, right click to turn",
            _ => "click a corner",
        };
        let at = crate::terrain_mode::status(view).unwrap_or_default();
        format!("{}: {how}; Escape: stop. {at}", b.label)
    } else if view.redraw.is_some() {
        "Redraw Polygon: click its corners, double click to close; right click or Escape: stop"
            .to_string()
    } else if view.pasting {
        "Pasting: click to place the copies; right click or Escape: stop".to_string()
    } else if let Some(key) = brush(app) {
        let what = match ObjectKind::from_restype(key.restype) {
            Some(k) if k.has_outline() => {
                "click its corners, double click to close; right click or Escape: stop"
            }
            _ => "click to place (Shift + click: place more); right click or Escape: stop",
        };
        format!("Placing {}: {what}", key.resref)
    } else if let ([(kind, index)], Some(m)) = (view.selection.as_slice(), &view.model)
        && let Some(o) = m.object(*kind, *index)
    {
        format!(
            "{:?} {} ({}) at {:.2}, {:.2}, {:.2}, facing {:.0}°",
            kind,
            o.tag,
            o.template.map(|t| t.to_string()).unwrap_or_default(),
            o.position.x,
            o.position.y,
            o.position.z,
            o.facing().to_degrees().rem_euclid(360.0),
        )
    } else if view.selection.len() > 1 {
        format!("{} objects selected", view.selection.len())
    } else {
        "Click to select, drag to move or to select in a box, Shift + right drag to turn, \
         Alt + drag to raise; Ctrl + drag moves the view, Ctrl + right drag turns it."
            .to_string()
    };
    ui.add(egui::Label::new(egui::RichText::new(&status).weak()).truncate()).on_hover_text(status);
}

/// The tiles' outlines (Display Grid), drawn in the scene so that what
/// stands on the ground hides them.
fn grid_lines(view: &AreaView, shown: &AreaModel) -> Vec<mg_render::Line> {
    let color = [1.0, 1.0, 1.0, 40.0 / 255.0];
    let mut out = Vec::new();
    for (i, t) in shown.tiles.iter().enumerate() {
        // At the tile's ground, which some tilesets build above the tile's
        // height.
        let level = view.ground.as_ref().and_then(|g| g.tile_level(i));
        let c = t.position.truncate().extend(level.unwrap_or(t.position.z));
        let h = TILE_SIZE / 2.0;
        let corners = [(-h, -h), (h, -h), (h, h), (-h, h)].map(|(x, y)| c + Vec3::new(x, y, 0.0));
        for k in 0..4 {
            out.push(mg_render::Line { from: corners[k], to: corners[(k + 1) % 4], color });
        }
    }
    out
}

fn viewport(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    view: &mut AreaView,
    start: Option<(Vec3, f32)>,
) {
    view.turn_ring = !app.settings.no_turn_ring;
    if let Some(scene) = view.scene.as_mut() {
        scene.merchant_signs = app.settings.merchant_signs;
        scene.spawn_markers = !app.settings.no_spawn_markers;
    }
    // The blueprint about to be placed, see-through where it would go.
    let ghost = ghost(app, ui, view);
    view.ghost_shown.clone_from(&ghost);
    // What a tile brush's click would make, under the pointer: the tiles in
    // place of those they replace.
    let settings = View {
        time: view.time,
        night: view.night,
        fog: view.fog,
        lit: view.lit,
        show: view.show,
        animate: !app.settings.still_objects,
    };
    let shift = ui.input(|i| i.modifiers.shift);
    let preview = crate::terrain_mode::preview(app, view, shift);
    let (preview_instances, hidden) =
        match (preview, view.scene.as_mut(), app.viewport.as_ref(), app.game.as_deref()) {
            (Some((tiles, hidden)), Some(scene), Some(vp), Some(game)) => {
                let instances =
                    scene.preview_tiles(&vp.gpu, game, &tiles, &settings, PREVIEW_OPACITY);
                view.tile_preview = tiles;
                (instances, hidden)
            }
            _ => {
                view.tile_preview.clear();
                (Vec::new(), Vec::new())
            }
        };
    let (ghost_instances, ghost_box) =
        match (&ghost, view.scene.as_mut(), app.viewport.as_ref(), app.game.as_deref()) {
            (Some(o), Some(scene), Some(vp), Some(game)) => {
                let (instances, bounds) = scene.ghost(&vp.gpu, game, o, view.time, GHOST_OPACITY);
                // Outlined too, to stand out from what is around it.
                (instances, Some((o.transform(), bounds)))
            }
            _ => (Vec::new(), None),
        };
    // What is about to be pasted (copied objects, or a prefab), where the
    // pointer would put it: see-through too, each object in its place
    // around the others.
    let mut ghost_instances = ghost_instances;
    let pasted: Vec<mg_area::AreaObject> = match (&app.object_clip, view.pasting) {
        (Some(clip), true) => {
            // Over the view, not over a window in front of it.
            let over = |p: &Pos2| {
                let layer = ui.ctx().layer_id_at(*p);
                let behind =
                    layer.is_some_and(|l| l != ui.layer_id() && l.order != egui::Order::Tooltip);
                view.rect.contains(*p) && !behind
            };
            let at = ui.ctx().pointer_hover_pos().filter(over);
            let at = at.and_then(|p| view.ground_at(p, 0.0)).map(|at| view.snapped(at));
            at.map_or_else(Vec::new, |at| {
                let places = pasted_positions(view, clip, at);
                let placed = clip.objects.iter().zip(places).map(|((o, _, _), p)| {
                    let mut o = o.clone();
                    let moved = p - o.position;
                    o.outline.iter_mut().for_each(|q| *q += moved);
                    o.position = p;
                    o
                });
                placed.collect()
            })
        }
        _ => Vec::new(),
    };
    if let (Some(scene), Some(vp), Some(game)) =
        (view.scene.as_mut(), app.viewport.as_ref(), app.game.as_deref())
    {
        for o in &pasted {
            let (instances, _) = scene.ghost(&vp.gpu, game, o, view.time, GHOST_OPACITY);
            ghost_instances.extend(instances);
        }
    }
    view.pasted_shown = pasted.len();
    let (Some(model), Some(scene), Some(orbit), Some(vp), Some(game)) =
        (&view.model, &view.scene, view.orbit, app.viewport.as_mut(), app.game.as_deref())
    else {
        return;
    };
    let size = ui.available_size().max(egui::vec2(32.0, 32.0));
    let ppp = ui.ctx().pixels_per_point();
    let (w, h) = ((size.x * ppp).round() as u32, (size.y * ppp).round() as u32);
    let (w, h) = (w.clamp(16, 8192), h.clamp(16, 8192));

    let now = ui.input(|i| i.time);
    view.time += view.last_frame.map_or(0.0, |last| (now - last) as f32).min(0.25);
    view.last_frame = Some(now);

    // The scene, with the drag applied to what it moves.
    let dragged = view.dragged();
    let mut shown = std::borrow::Cow::Borrowed(model);
    for &(i, p, r) in &dragged {
        let o = &mut shown.to_mut().objects[i];
        let delta = p - o.position;
        o.outline.iter_mut().for_each(|q| *q += delta);
        o.position = p;
        o.rotation = r;
    }
    for (i, v) in view.tilted() {
        shown.to_mut().objects[i].visual = Some(v);
    }
    if let Some(Drag::FaceSpawn { object, point, facing, .. }) = view.drag
        && let Some(f) = shown.to_mut().objects[object].spawn_facings.get_mut(point)
    {
        *f = facing;
    }
    let mut frame = scene.scene_hiding(&shown, &settings, &hidden);
    frame.fog = frame.fog.map(|f| view_fog(f, orbit.distance));
    frame.instances.extend(ghost_instances);
    frame.instances.extend(preview_instances);
    if view.grid {
        frame.lines = grid_lines(view, &shown);
    }
    // Options › Area: the background colour, if chosen (gamma space).
    if let Some([r, g, b]) = app.settings.area_background {
        frame.background = [r, g, b].map(|c| f32::from(c) / 255.0);
    }
    if view.targets.as_ref().is_none_or(|(t, _)| t.size != (w, h)) {
        let targets = Targets::new(&vp.gpu, wgpu::TextureFormat::Rgba8Unorm, SAMPLES, w, h);
        let mut egui_renderer = vp.render_state.renderer.write();
        let id = match view.targets.take() {
            Some((_, id)) => {
                egui_renderer.update_egui_texture_from_wgpu_texture(
                    &vp.gpu.device,
                    &targets.color_view,
                    wgpu::FilterMode::Linear,
                    id,
                );
                id
            }
            None => egui_renderer.register_native_texture(
                &vp.gpu.device,
                &targets.color_view,
                wgpu::FilterMode::Linear,
            ),
        };
        view.targets = Some((targets, id));
    }
    let (targets, id) = view.targets.as_ref().expect("made above");
    let camera = orbit.camera();
    vp.renderer.render(
        &vp.gpu,
        &game.resman,
        &frame,
        &camera,
        targets.render_view(),
        targets.resolve_view(),
        &targets.depth,
        (w, h),
    );
    let response = ui.add(
        egui::Image::new(egui::load::SizedTexture::new(*id, size))
            .sense(egui::Sense::click_and_drag()),
    );
    view.rect = response.rect;
    // Placing a door, with a click or by dragging one: the hooks show.
    let dragged = egui::DragAndDrop::payload::<crate::palette_view::Dragged>(ui.ctx());
    let door_brush =
        dragged.map(|d| d.0).or_else(|| brush(app)).is_some_and(|k| k.restype == ResType::UTD);
    let (height, width) = app.settings.spawn_marker_size.unwrap_or(SPAWN_MARKER);
    let marks = Marks {
        spawn_points: (!app.settings.no_spawn_markers)
            .then_some((f32::from(height) / 10.0, f32::from(width) / 10.0)),
        door_arrows: !app.settings.no_door_arrows,
        ghost_box,
    };
    // The start location where it is being dragged to.
    let start = match view.drag {
        Some(Drag::Start { at, facing, .. }) => Some((at, facing)),
        _ => start,
    };
    overlays(ui, view, &shown, start, app.object_clip.as_ref(), door_brush, marks);
    sound_range_overlay(app, ui, view, &shown);
    // Where the pointer is, to the centimeter (Aurora shows whole meters).
    view.pointer = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|p| view.rect.contains(*p))
        .and_then(|p| view.ground_at(p, 0.0));
    if let Some(p) = view.pointer {
        let painter = ui.painter_at(view.rect);
        let text = format!("{:.2}, {:.2}, {:.2}", p.x, p.y, p.z);
        let font = egui::FontId::monospace(12.0);
        let at = view.rect.left_bottom() + egui::vec2(8.0, -8.0);
        let galley = painter.layout_no_wrap(text, font, Color32::WHITE);
        let bg = Rect::from_min_size(
            at - egui::vec2(3.0, galley.size().y + 2.0),
            galley.size() + egui::vec2(6.0, 4.0),
        );
        painter.rect_filled(bg, 3.0, Color32::from_black_alpha(140));
        painter.galley(at - egui::vec2(0.0, galley.size().y), galley, Color32::WHITE);
    }
    walkmesh_overlay(app, ui, view);
    object_walkmesh_overlay(app, ui, view);
    crate::terrain_mode::overlay(app, ui, view);
    crate::tile_select::overlay(ui, view, app.tile_clip.as_ref());
    // Tiles animate: keep drawing while the view is on screen.
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    input(app, ui, view, &response);
}

/// The ground's walkmesh over the view: walkable faces (surfacemat.2da
/// `Walk`) green, the others red.
fn walkmesh_overlay(app: &Moonglow, ui: &egui::Ui, view: &mut AreaView) {
    if !view.walkmesh {
        return;
    }
    if view.walkable.is_none() {
        let walkable = app.game.as_deref().and_then(|g| g.table("surfacemat").ok()).map(|t| {
            (0..t.len()).map(|r| t.get_int(r, "Walk").unwrap_or(0) != 0).collect::<Vec<bool>>()
        });
        view.walkable = Some(walkable.unwrap_or_default());
    }
    let (Some(ground), Some(walkable)) = (view.ground.as_ref(), view.walkable.as_ref()) else {
        return;
    };
    let mut mesh = egui::Mesh::default();
    let (walk, wall) = (
        egui::Color32::from_rgba_unmultiplied(60, 200, 80, 70),
        egui::Color32::from_rgba_unmultiplied(220, 60, 60, 70),
    );
    for (corners, material) in ground.faces() {
        let lifted = corners.map(|c| c + Vec3::Z * 0.03);
        let Some(points) =
            lifted.iter().map(|c| view.screen_pos(*c)).collect::<Option<Vec<Pos2>>>()
        else {
            continue;
        };
        let color =
            if walkable.get(material as usize).copied().unwrap_or(false) { walk } else { wall };
        let first = mesh.vertices.len() as u32;
        for p in points {
            mesh.colored_vertex(p, color);
        }
        mesh.add_triangle(first, first + 1, first + 2);
    }
    ui.painter_at(view.rect).add(egui::Shape::mesh(mesh));
}

/// Where the placed sounds are heard (the Sound Ranges switch): around
/// each positional sound, level at its height, a circle of its
/// `MinDistance` (full volume inside) and one of its `MaxDistance` (silent
/// outside); the selection's brighter. A sound heard everywhere in the
/// area has none.
fn sound_range_overlay(app: &mut Moonglow, ui: &egui::Ui, view: &AreaView, shown: &AreaModel) {
    if !view.sound_ranges || !view.show[ObjectKind::Sound.index()] {
        return;
    }
    let Some(ws) = app.ws.as_mut() else { return };
    let Ok(doc) = ws.doc(&view.git()) else { return };
    let Some(list) = doc.root.list(ObjectKind::Sound.list()) else { return };
    let painter = ui.painter_at(view.rect);
    for (i, o) in shown.objects.iter().enumerate().filter(|(_, o)| o.kind == ObjectKind::Sound) {
        let Some(sound) = list.get(o.index).map(crate::area_audio::PlacedSound::from_git) else {
            continue;
        };
        if !sound.positional {
            continue;
        }
        let alpha = if view.selected(i) { 255 } else { 150 };
        for (radius, width) in [(sound.min_distance, 2.0), (sound.max_distance, 1.0)] {
            if radius <= 0.0 {
                continue;
            }
            let stroke = Stroke::new(width, Color32::from_rgba_unmultiplied(240, 220, 80, alpha));
            let points: Vec<Option<Pos2>> =
                sound_circle(o.position, radius).into_iter().map(|p| view.screen_pos(p)).collect();
            for k in 0..points.len() {
                if let (Some(a), Some(b)) = (points[k], points[(k + 1) % points.len()]) {
                    painter.line_segment([a, b], stroke);
                }
            }
        }
    }
}

/// The points of a level circle around `centre`.
fn sound_circle(centre: Vec3, radius: f32) -> Vec<Vec3> {
    const POINTS: usize = 64;
    (0..POINTS)
        .map(|k| {
            let a = k as f32 / POINTS as f32 * std::f32::consts::TAU;
            centre + Vec3::new(a.cos(), a.sin(), 0.0) * radius
        })
        .collect()
}

/// Placeables' and doors' walkmeshes over the view: placeables orange,
/// doors blue, the selection's brighter.
fn object_walkmesh_overlay(app: &Moonglow, ui: &egui::Ui, view: &mut AreaView) {
    if !view.object_walkmesh {
        return;
    }
    let (Some(game), Some(model)) = (app.game.as_deref(), view.model.as_ref()) else { return };
    if view.object_faces.is_none() {
        view.object_faces = Some(view.object_walks.faces(game, model));
    }
    let Some(faces) = view.object_faces.as_ref() else { return };
    let mut mesh = egui::Mesh::default();
    let mut edges = Vec::new();
    for (corners, object) in faces {
        let Some(o) = model.objects.get(*object) else { continue };
        if !view.show[o.kind.index()] {
            continue;
        }
        let lifted = corners.map(|c| c + Vec3::Z * 0.05);
        let Some(points) =
            lifted.iter().map(|c| view.screen_pos(*c)).collect::<Option<Vec<Pos2>>>()
        else {
            continue;
        };
        let (r, g, b) = if o.kind == ObjectKind::Door { (70, 140, 255) } else { (255, 150, 40) };
        let alpha = if view.selected(*object) { 150 } else { 80 };
        let color = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);
        let first = mesh.vertices.len() as u32;
        for p in &points {
            mesh.colored_vertex(*p, color);
        }
        mesh.add_triangle(first, first + 1, first + 2);
        edges.push((points, egui::Color32::from_rgba_unmultiplied(r, g, b, 200)));
    }
    let painter = ui.painter_at(view.rect);
    painter.add(egui::Shape::mesh(mesh));
    for (p, color) in edges {
        painter.add(egui::Shape::closed_line(p, Stroke::new(1.0, color)));
    }
}

/// Aurora's spawn point markers: Height 12 and Width 4 (tenths of a metre).
pub(crate) const SPAWN_MARKER: (u8, u8) = (12, 4);

/// What Options › Area adds over the view, and the box of a blueprint
/// about to be placed (placed by the transform, in its own space).
#[derive(Clone, Copy)]
struct Marks {
    /// A post over each encounter spawn point: its height and width (m).
    spawn_points: Option<(f32, f32)>,
    /// An arrow along each door's facing.
    door_arrows: bool,
    ghost_box: Option<(glam::Mat4, (Vec3, Vec3))>,
}

/// Markers for objects without models, outlines, the grid, the start
/// location and the selection, drawn over the view.
fn overlays(
    ui: &egui::Ui,
    view: &AreaView,
    shown: &AreaModel,
    start: Option<(Vec3, f32)>,
    clip: Option<&ObjectClip>,
    door_brush: bool,
    marks: Marks,
) {
    let painter = ui.painter_at(view.rect);
    let at = |p: Vec3| view.screen_pos(p);
    let line = |a: Vec3, b: Vec3, stroke: Stroke| {
        if let (Some(a), Some(b)) = (at(a), at(b)) {
            painter.line_segment([a, b], stroke);
        }
    };
    let scene = view.scene.as_ref();
    for (i, o) in shown.objects.iter().enumerate() {
        if !view.show[o.kind.index()] {
            continue;
        }
        let selected = view.selected(i);
        let highlight = Stroke::new(2.0, Color32::YELLOW);
        if o.kind.has_outline() {
            // Encounters orange; triggers green, area transitions blue,
            // traps red.
            let color = match (o.kind, o.trigger_type) {
                (ObjectKind::Encounter, _) => Color32::from_rgb(230, 120, 40),
                (_, 1) => Color32::from_rgb(80, 150, 255),
                (_, 2) => Color32::from_rgb(230, 60, 60),
                _ => Color32::from_rgb(80, 200, 120),
            };
            let stroke = if selected { highlight } else { Stroke::new(1.5, color) };
            let n = o.outline.len();
            for k in 0..n {
                line(o.outline[k], o.outline[(k + 1) % n], stroke);
            }
            if let Some((height, width)) = marks.spawn_points {
                for (k, &p) in o.spawn_points.iter().enumerate() {
                    let (top, half) = (p + Vec3::Z * height, width / 2.0);
                    line(p, top, stroke);
                    line(top - Vec3::X * half, top + Vec3::X * half, stroke);
                    line(top - Vec3::Y * half, top + Vec3::Y * half, stroke);
                    // The way what spawns there faces: an arrow on the
                    // ground.
                    let facing = o.spawn_facings.get(k).copied().unwrap_or(0.0);
                    let ahead = Vec3::new(facing.cos(), facing.sin(), 0.0);
                    let side = Vec3::new(-ahead.y, ahead.x, 0.0) * 0.3;
                    let (base, tip) = (p + Vec3::Z * 0.1, p + Vec3::Z * 0.1 + ahead);
                    line(base, tip, stroke);
                    line(tip, tip - ahead * 0.4 + side, stroke);
                    line(tip, tip - ahead * 0.4 - side, stroke);
                    // Selected, the tip is a handle: led round, it turns
                    // the way what spawns there faces.
                    if let (true, Some(c)) = (selected && !o.locked, at(spawn_arrow_tip(p, facing)))
                    {
                        let held = matches!(view.drag, Some(Drag::FaceSpawn { object, point, .. })
                            if object == i && point == k);
                        let over = view.drag.is_none()
                            && ui
                                .ctx()
                                .pointer_hover_pos()
                                .is_some_and(|q| q.distance(c) <= RING_REACH);
                        let fill = if held || over { Color32::WHITE } else { stroke.color };
                        painter.circle(
                            c,
                            4.5,
                            fill,
                            Stroke::new(1.5, Color32::from_black_alpha(200)),
                        );
                        if held {
                            painter.text(
                                c + egui::vec2(10.0, -10.0),
                                egui::Align2::LEFT_BOTTOM,
                                format!("{:.0}°", facing.to_degrees().rem_euclid(360.0)),
                                egui::FontId::proportional(13.0),
                                Color32::WHITE,
                            );
                        }
                    }
                }
            }
            continue;
        }
        let transform = o.model_transform();
        // Doors: a cyan arrow along their facing (Options › Area).
        if marks.door_arrows && o.kind == ObjectKind::Door {
            let ahead = Vec3::new(o.facing().cos(), o.facing().sin(), 0.0);
            let side = Vec3::new(-ahead.y, ahead.x, 0.0) * 0.4;
            let base = o.position + Vec3::Z * 0.1;
            let tip = base + ahead * 1.5;
            let stroke = Stroke::new(1.5, Color32::from_rgb(80, 220, 230));
            line(base, tip, stroke);
            line(tip, tip - ahead * 0.5 + side, stroke);
            line(tip, tip - ahead * 0.5 - side, stroke);
        }
        // What the scene draws (models, marker models, the arrow) is hidden
        // by what stands in front of it. Without a scene (no GPU), waypoints
        // and merchants are a yellow arrow along their facing, over the
        // view.
        let in_scene = scene.is_some_and(|s| s.draws(shown, i));
        if o.kind.has_arrow() && !selected && !in_scene {
            let ahead = Vec3::new(o.facing().cos(), o.facing().sin(), 0.0);
            let side = Vec3::new(-ahead.y, ahead.x, 0.0) * 0.6;
            let (tip, base) = (o.position + ahead * 1.0, o.position - ahead * 0.6);
            if let (Some(t), Some(l), Some(r)) = (at(tip), at(base + side), at(base - side)) {
                painter.add(egui::Shape::convex_polygon(
                    vec![t, l, r],
                    Color32::from_rgb(240, 210, 40),
                    Stroke::new(1.0, Color32::from_rgb(120, 100, 0)),
                ));
            }
            continue;
        }
        let marker = !in_scene;
        if marker || selected {
            let (min, max) = match scene {
                Some(s) => s.bounds(shown, i),
                None => mg_area::pick::marker_bounds(o.kind),
            };
            let color = match o.kind {
                ObjectKind::Waypoint => Color32::from_rgb(90, 170, 255),
                ObjectKind::Sound => Color32::from_rgb(240, 220, 80),
                ObjectKind::Store => Color32::from_rgb(120, 220, 120),
                _ => Color32::from_rgb(230, 70, 70),
            };
            let stroke = if selected { highlight } else { Stroke::new(1.5, color) };
            let corner = |c: usize| {
                transform.transform_point3(Vec3::new(
                    if c & 1 == 0 { min.x } else { max.x },
                    if c & 2 == 0 { min.y } else { max.y },
                    if c & 4 == 0 { min.z } else { max.z },
                ))
            };
            for (a, b) in [
                (0, 1),
                (1, 3),
                (3, 2),
                (2, 0),
                (4, 5),
                (5, 7),
                (7, 6),
                (6, 4),
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ] {
                line(corner(a), corner(b), stroke);
            }
        }
    }
    if view.pasting
        && let Some(clip) = clip
        && let Some(at) = ui.ctx().pointer_hover_pos().and_then(|p| view.ground_at(p, 0.0))
    {
        let stroke = Stroke::new(1.5, Color32::from_rgb(120, 230, 255));
        for ((o, _, _), p) in
            clip.objects.iter().zip(pasted_positions(view, clip, view.snapped(at)))
        {
            let (min, max) = match (&view.scene, o.kind.has_outline()) {
                (_, true) => {
                    let d = p - o.position;
                    let n = o.outline.len();
                    for k in 0..n {
                        line(o.outline[k] + d, o.outline[(k + 1) % n] + d, stroke);
                    }
                    continue;
                }
                _ => mg_area::pick::marker_bounds(o.kind),
            };
            let t =
                glam::Mat4::from_rotation_translation(glam::Quat::from_rotation_z(o.rotation), p);
            for (a, b) in box_edges(t, min, max) {
                line(a, b, stroke);
            }
        }
    }
    // A blueprint about to be placed: its box (its models', else its
    // marker's).
    if let Some((t, (min, max))) = marks.ghost_box {
        let stroke = Stroke::new(1.5, Color32::from_rgb(120, 230, 255));
        for (a, b) in box_edges(t, min, max) {
            line(a, b, stroke);
        }
    }
    if door_brush {
        // The door hooks: a short line across the doorway, the nearest to
        // the pointer bright.
        let pointer = ui.ctx().pointer_hover_pos().and_then(|p| view.ground_at(p, 0.0));
        let near = pointer.and_then(|p| shown.hook_near(p, DOOR_REACH)).copied();
        for h in &shown.hooks {
            let lit = near.is_some_and(|n| n.position == h.position && n.bearing == h.bearing);
            let color =
                if lit { Color32::from_rgb(120, 255, 140) } else { Color32::from_rgb(60, 140, 80) };
            let across = Vec3::new(h.bearing.cos(), h.bearing.sin(), 0.0);
            line(
                h.position - across,
                h.position + across,
                Stroke::new(if lit { 3.0 } else { 1.5 }, color),
            );
        }
    }
    // The ring for turning the selection: brighter under the pointer and
    // while it is led round, a mark on it the way the object faces.
    // With Shift held, the rings for tilting its model instead: red about
    // X, green about Y.
    let tilting = match view.drag {
        Some(Drag::Tilt { .. }) => true,
        Some(_) => false,
        None => ui.input(|i| i.modifiers.shift && !i.modifiers.command && !i.modifiers.alt),
    };
    let rings = if tilting { view.tilt_rings(shown) } else { Vec::new() };
    if !rings.is_empty() {
        let held = match view.drag {
            Some(Drag::Tilt { ring, .. }) => Some(ring.axis),
            _ => None,
        };
        let over = ui
            .ctx()
            .pointer_hover_pos()
            .filter(|_| view.drag.is_none())
            .and_then(|p| view.tilt_ring_at(p));
        for ring in rings {
            // (Led, every ring of the axis; hovered, the one under the
            // pointer.)
            let lit = held == Some(ring.axis) || over == Some(ring);
            let stroke = if lit {
                Stroke::new(2.5, ring.color())
            } else {
                Stroke::new(1.5, ring.color().gamma_multiply(0.7))
            };
            for k in 0..RING_SEGMENTS {
                let (a, b) = ring.segment(k);
                line(a, b, stroke);
            }
        }
        if let Some(Drag::Tilt { ring, grip, angle, .. }) = view.drag
            && let Some(c) = at(ring.point(grip + angle))
            && let Some((_, visual)) = view.tilted().first()
        {
            painter.circle_filled(c, 3.5, ring.color());
            let degrees = visual.rotate[ring.axis];
            painter.text(
                c + egui::vec2(10.0, -10.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{} {degrees:.0}°", ["X", "Y"][ring.axis]),
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
        }
    }
    // And the arrows for moving it along one axis: east red, north green,
    // up blue.
    let sliding = match view.drag {
        Some(Drag::Slide { axis, .. }) => Some(axis),
        _ => None,
    };
    if sliding.is_some() || (tilting && view.drag.is_none()) {
        let pointer = ui.ctx().pointer_hover_pos().filter(|_| view.drag.is_none());
        let over = pointer.and_then(|p| view.axis_arrow_at(p));
        for arrow in view.axis_arrows(shown) {
            if sliding.is_some_and(|axis| axis != arrow.axis) {
                continue;
            }
            let lit = sliding.is_some() || over == Some(arrow);
            let stroke = if lit {
                Stroke::new(3.0, arrow.color())
            } else {
                Stroke::new(2.0, arrow.color().gamma_multiply(0.75))
            };
            let (neck, tip) = arrow.head();
            line(arrow.pivot, neck, Stroke::new(1.0, stroke.color));
            line(neck, tip, stroke);
            let (Some(n), Some(t)) = (at(neck), at(tip)) else { continue };
            // The head's barbs, on screen.
            let along = (t - n).normalized();
            let side = egui::vec2(-along.y, along.x);
            for s in [-1.0, 1.0] {
                painter.line_segment([t, t - along * 9.0 + side * 5.0 * s], stroke);
            }
            if sliding.is_some() {
                let name = ["X", "Y", "Z"][arrow.axis];
                painter.text(
                    t + egui::vec2(10.0, -4.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{name} {:.2} m", arrow.pivot[arrow.axis]),
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );
            }
        }
    }
    if !tilting
        && !matches!(
            view.drag,
            Some(Drag::Box { .. } | Drag::Move { .. } | Drag::Lift { .. } | Drag::Slide { .. })
        )
    {
        let spinning = matches!(view.drag, Some(Drag::Spin { .. }));
        let pointer = ui.ctx().pointer_hover_pos().filter(|_| view.drag.is_none());
        let color = Color32::from_rgb(255, 170, 60);
        for ring in view.turn_rings(shown) {
            let lit = spinning || pointer.is_some_and(|p| view.on_ring(&ring, p));
            let stroke = if lit {
                Stroke::new(2.5, Color32::from_rgb(255, 225, 150))
            } else {
                Stroke::new(1.5, color.gamma_multiply(0.8))
            };
            for k in 0..RING_SEGMENTS {
                let (a, b) = ring.segment(k);
                line(a, b, stroke);
            }
            let ahead = ring.point(ring.facing);
            line(ring.pivot, ahead, Stroke::new(1.0, stroke.color));
            let Some(c) = at(ahead) else { continue };
            painter.circle_filled(c, 3.5, stroke.color);
            if spinning {
                let degrees = ring.facing.to_degrees().rem_euclid(360.0);
                painter.text(
                    c + egui::vec2(10.0, -10.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{degrees:.0}°"),
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );
            }
        }
    }
    if let Some(Drag::Box { from, to }) = view.drag {
        let r = Rect::from_two_pos(from, to);
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::YELLOW), egui::StrokeKind::Inside);
    }
    if !view.outline.is_empty() {
        let stroke = Stroke::new(2.0, Color32::from_rgb(120, 230, 255));
        for w in view.outline.windows(2) {
            line(w[0], w[1], stroke);
        }
        let pointer = ui.ctx().pointer_hover_pos().and_then(|p| view.ground_at(p, 0.0));
        if let (Some(last), Some(p)) = (view.outline.last(), pointer) {
            line(*last, p, Stroke::new(1.0, stroke.color));
        }
        for p in &view.outline {
            if let Some(c) = at(*p) {
                painter.circle_filled(c, 3.0, stroke.color);
            }
        }
    }
    // The start location, as Aurora marks it: a blue ring on the ground
    // and a red arrow the way the player faces on arriving.
    if let (true, Some((p, facing))) = (view.show_start, start) {
        let m = StartMarker::new(p, facing);
        // Brighter while it is held.
        let held = matches!(view.drag, Some(Drag::Start { .. }));
        let width = if held { 4.0 } else { 3.0 };
        let ring = Stroke::new(width, Color32::from_rgb(70, 130, 255));
        const PIECES: usize = 24;
        let around = |k: usize| m.around(std::f32::consts::TAU * k as f32 / PIECES as f32);
        for k in 0..PIECES {
            line(around(k), around(k + 1), ring);
        }
        let arrow = Stroke::new(width, Color32::from_rgb(235, 40, 40));
        line(m.tail, m.tip, arrow);
        let barb = StartMarker::BARB;
        line(m.tip, m.tip - m.ahead * barb + m.side * barb, arrow);
        line(m.tip, m.tip - m.ahead * barb - m.side * barb, arrow);
        // (And its cross-guard.)
        line(m.guard.0, m.guard.1, arrow);
        // Selected, a box round it as round an object: where a character
        // would stand.
        let selected = view.start_is_selected();
        if selected {
            let t = glam::Mat4::from_translation(p) * glam::Mat4::from_rotation_z(facing);
            let (min, max) = (Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 1.8));
            for (a, b) in box_edges(t, min * START_RING, max * START_RING) {
                line(a, b, Stroke::new(2.0, Color32::YELLOW));
            }
        }
        // Selected, the arrow's tip is a handle: led round, it turns the
        // marker.
        if let (true, Some(c)) = (selected, at(m.tip)) {
            let turning = matches!(view.drag, Some(Drag::Start { turn: true, .. }));
            let pointer = ui.ctx().pointer_hover_pos().filter(|_| view.drag.is_none());
            let lit = turning || pointer.is_some_and(|q| q.distance(c) <= RING_REACH);
            let (radius, fill) = if lit {
                (7.5, Color32::from_rgb(255, 225, 150))
            } else {
                (6.0, Color32::from_rgb(255, 170, 60))
            };
            painter.circle(c, radius, fill, Stroke::new(1.5, Color32::from_black_alpha(220)));
            if turning {
                painter.text(
                    c + egui::vec2(12.0, -10.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{:.0}°", facing.to_degrees().rem_euclid(360.0)),
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );
            }
        }
    }
}

fn input(app: &mut Moonglow, ui: &egui::Ui, view: &mut AreaView, response: &egui::Response) {
    let (shift, command, alt) =
        ui.input(|i| (i.modifiers.shift, i.modifiers.command, i.modifiers.alt));
    // Painting tiles, Shift is the brush's (a drag's rectangle or outline):
    // the camera moves as it does without it.
    let painting = crate::terrain_mode::active(app, view).is_some();
    view.held = Held::read(ui, response, view.held);
    camera_input(ui, view, response, shift && !painting, command, &app.keymap);
    if let Some(dragged) = response.dnd_release_payload::<crate::palette_view::Dragged>() {
        drop_blueprint(app, view, response, dragged.0);
        return;
    }
    // Q and E turn a blueprint about to be placed (dragged, or chosen in
    // the palette), as they turn a selection; doors face as their hooks do.
    let placing = view.ghost_shown.as_ref().filter(|o| o.kind != ObjectKind::Door).is_some();
    if placing && !ui.ctx().memory(|m| m.focused().is_some()) {
        for (cmd, by) in turn_keys(view) {
            if ui.input(|i| app.keymap.pressed(i, cmd)) {
                view.ghost_turn += by;
            }
        }
    }
    if crate::terrain_mode::input(app, ui, view, response) {
        return;
    }
    if crate::tile_select::paste_input(app, ui, view, response) {
        return;
    }
    if crate::tile_select::input(app, ui, view, response) {
        if response.secondary_clicked() {
            view.tile_menu_at = response.interact_pointer_pos();
        }
        if !shift {
            let at = view.tile_menu_at;
            response.context_menu(|ui| crate::tile_select::context_menu(app, view, ui, at));
        }
        return;
    }
    let hovered = response.hovered();

    // Copy, cut and paste (egui's events, or the keys where it sends none).
    let typing = ui.ctx().memory(|m| m.focused().is_some());
    let (copy, cut, pasted) = ui.input(|i| {
        let key = |k: egui::Key| i.modifiers.command && i.key_pressed(k);
        let event = |f: fn(&egui::Event) -> bool| i.events.iter().any(f);
        (
            event(|e| matches!(e, egui::Event::Copy)) || key(egui::Key::C),
            event(|e| matches!(e, egui::Event::Cut)) || key(egui::Key::X),
            event(|e| matches!(e, egui::Event::Paste(_))) || key(egui::Key::V),
        )
    });
    if hovered && !typing {
        if (copy || cut) && !view.selection.is_empty() {
            app.object_clip = copy_selection(app, view);
            if cut {
                delete(app, view);
            }
        }
        if pasted && app.object_clip.is_some() {
            view.pasting = true;
        }
    }
    if view.pasting {
        let cancel = response.secondary_clicked()
            || (hovered && ui.input(|i| i.key_pressed(egui::Key::Escape)));
        if cancel {
            view.pasting = false;
        } else if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(at) = view.ground_at(pos, 0.0)
        {
            paste_at(app, view, at);
            view.pasting = false;
        }
        return;
    }

    // Drawing a trigger's or encounter's outline anew.
    if let Some((kind, index)) = view.redraw {
        let cancel = response.secondary_clicked()
            || (hovered && ui.input(|i| i.key_pressed(egui::Key::Escape)));
        if cancel {
            view.redraw = None;
            view.outline.clear();
        } else if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(at) = view.ground_at(pos, 0.0)
        {
            if view.outline.last().is_none_or(|p| (*p - at).length() > 0.05) {
                view.outline.push(at);
            }
            let closing = response.double_clicked() || response.triple_clicked();
            if closing && view.outline.len() >= 3 {
                let outline = std::mem::take(&mut view.outline);
                redraw_outline(app, view, kind, index, &outline);
                view.redraw = None;
            }
        }
        return;
    }

    // Placing the palette's blueprint.
    if let Some(key) = brush(app) {
        let cancel = response.secondary_clicked()
            || (hovered && ui.input(|i| i.key_pressed(egui::Key::Escape)));
        if cancel {
            app.palette.selected = None;
            view.outline.clear();
            return;
        }
        if response.clicked()
            && !command
            && let Some(pos) = response.interact_pointer_pos()
            && let Some(at) = view.ground_at(pos, 0.0)
        {
            let kind = ObjectKind::from_restype(key.restype).expect("a brush places objects");
            if kind.has_outline() {
                // A double click's second click closes the outline.
                if view.outline.last().is_none_or(|p| (*p - at).length() > 0.05) {
                    view.outline.push(at);
                }
                // (Quick clicks after the last corner count as a triple.)
                let closing = response.double_clicked() || response.triple_clicked();
                if closing && view.outline.len() >= 3 {
                    let outline = std::mem::take(&mut view.outline);
                    place(app, view, key, outline[0], 0.0, &outline);
                    if !shift {
                        app.palette.selected = None;
                    }
                }
            } else if kind == ObjectKind::Door {
                // Doors stand on door hooks (Aurora places none elsewhere).
                match view.model.as_ref().and_then(|m| m.hook_near(at, DOOR_REACH)).copied() {
                    Some(h) => {
                        place(app, view, key, h.position, h.bearing, &[]);
                        if !shift {
                            app.palette.selected = None;
                        }
                    }
                    None => app.log.warn("Doors go on door hooks: click near one"),
                }
            } else {
                place(app, view, key, at, view.ghost_turn, &[]);
                if !shift {
                    app.palette.selected = None;
                }
            }
        }
        return;
    }
    if view.redraw.is_none() {
        view.outline.clear();
    }

    // Selection. (A click on a ring or an arrow with nothing under it
    // leaves the selection be; with an object under it, it picks that.)
    // A click on the start location's marker selects it, alone.
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && view.pick(pos).is_none()
        && start_on_ground(app, view).is_some_and(|s| view.start_grab(s, pos).is_some())
    {
        view.selection.clear();
        view.start_selected = true;
    } else if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && (view.pick(pos).is_some()
            || !(view.spawn_arrow_at(pos).is_some()
                || view.ring_at(pos).is_some_and(|_| !shift)
                || (shift && view.tilt_ring_at(pos).is_some())
                || (shift && view.axis_arrow_at(pos).is_some())))
    {
        let hit = view.pick(pos).and_then(|i| {
            let o = &view.model.as_ref()?.objects[i];
            Some((o.kind, o.index))
        });
        match (hit, command) {
            (Some(h), true) => {
                if let Some(k) = view.selection.iter().position(|s| *s == h) {
                    view.selection.remove(k);
                } else {
                    view.selection.push(h);
                }
            }
            (Some(h), false) => view.selection = vec![h],
            (None, true) => {}
            (None, false) => view.selection.clear(),
        }
        view.start_selected = false;
    }
    if response.secondary_clicked()
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(i) = view.pick(pos)
        && !view.selected(i)
        && let Some(o) = view.model.as_ref().map(|m| &m.objects[i])
    {
        view.selection = vec![(o.kind, o.index)];
    }
    // A double click opens the object's Properties (a quick one after
    // another click counts as a triple).
    if (response.double_clicked() || response.triple_clicked())
        && let Some(pos) = response.interact_pointer_pos()
        && let Some(i) = view.pick(pos)
        && let Some(o) = view.model.as_ref().map(|m| &m.objects[i])
    {
        let (kind, index) = (o.kind, o.index);
        open_properties(app, view, kind, index);
    }
    if response.secondary_clicked() {
        view.menu_at = response.interact_pointer_pos().and_then(|p| view.ground_at(p, 0.0));
    }
    response.context_menu(|ui| context_menu(app, view, ui));

    // Moving, turning, raising; selecting by a box. A drag starts once the
    // pointer has moved: pick where it was pressed. It goes on until its
    // own button is let go, whatever another does meanwhile (the middle
    // one, swinging the camera).
    let held = view.held;
    let origin = ui.input(|i| i.pointer.press_origin());
    if response.drag_started_by(egui::PointerButton::Primary)
        && view.drag.is_none()
        && !command
        && let Some(pos) = origin
    {
        let hit = view.pick(pos);
        let handle = view.ring_at(pos).filter(|_| !alt && !shift);
        let tilt = view.tilt_ring_at(pos).filter(|_| shift && !alt);
        let arrow = view.axis_arrow_at(pos).filter(|_| shift && !alt);
        let spawn =
            view.spawn_arrow_at(pos).filter(|_| !shift && !alt && !app.settings.no_spawn_markers);
        view.drag = if let Some((object, point, pivot)) = spawn {
            let facing =
                view.model.as_ref().and_then(|m| m.objects[object].spawn_facings.get(point));
            Some(Drag::FaceSpawn { object, point, pivot, facing: facing.copied().unwrap_or(0.0) })
        } else if let Some(arrow) = arrow {
            let (grip, screen) = view.slide_grip(&arrow, pos);
            Some(Drag::Slide { axis: arrow.axis, pivot: arrow.pivot, grip, by: 0.0, screen })
        } else if let Some(ring) = tilt {
            view.tilt_grip(&ring, pos).map(|(grip, screen)| Drag::Tilt {
                ring,
                grip,
                angle: 0.0,
                screen,
            })
        } else if let Some(ring) = handle {
            view.angle_about(ring.pivot, pos).map(|grip| Drag::Spin {
                pivot: ring.pivot,
                grip,
                angle: 0.0,
            })
        } else if alt {
            (!view.selection.is_empty()).then_some(Drag::Lift { by: 0.0 })
        } else if let Some(i) = hit {
            let o = view.model.as_ref().map(|m| m.objects[i].clone()).expect("picked");
            if !view.selected(i) {
                view.selection = vec![(o.kind, o.index)];
            }
            view.ground_at(pos, o.position.z).map(|from| Drag::Move { from, offset: Vec2::ZERO })
        } else if let Some((was, turn)) = start_on_ground(app, view)
            .filter(|_| !shift && !alt)
            .and_then(|s| Some((s, view.start_grab(s, pos)?)))
        {
            // The start location, by its marker's lines (what stands on
            // it comes first).
            let grip = view.ground_at(pos, was.0.z).map_or(Vec2::ZERO, |g| (g - was.0).truncate());
            view.selection.clear();
            view.start_selected = true;
            Some(Drag::Start { turn, grip, at: was.0, facing: was.1, was })
        } else {
            Some(Drag::Box { from: pos, to: pos })
        };
    }
    if response.drag_started_by(egui::PointerButton::Secondary)
        && view.drag.is_none()
        && shift
        && !command
        && !view.selection.is_empty()
    {
        view.drag = Some(Drag::Turn { angle: 0.0 });
    }
    let pointer = held.pos;
    let button = match view.drag {
        Some(Drag::Turn { .. }) => egui::PointerButton::Secondary,
        _ => egui::PointerButton::Primary,
    };
    match view.drag {
        Some(Drag::Move { from, .. }) if held.by(button) => {
            if let Some(now) = pointer.and_then(|p| view.ground_at(p, from.z)) {
                view.drag = Some(Drag::Move { from, offset: (now - from).truncate() });
            }
        }
        Some(Drag::Lift { by }) if held.by(button) => {
            let scale = view.orbit.map_or(0.02, |o| o.distance * 0.002);
            view.drag = Some(Drag::Lift { by: by - held.delta.y * scale });
        }
        Some(Drag::Box { from, .. }) if held.by(button) => {
            if let Some(to) = pointer {
                view.drag = Some(Drag::Box { from, to });
            }
        }
        Some(Drag::Spin { pivot, grip, .. }) if held.by(button) => {
            if let Some(now) = pointer.and_then(|p| view.angle_about(pivot, p)) {
                // The short way round from where it was taken.
                let angle = (now - grip + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                view.drag = Some(Drag::Spin { pivot, grip, angle });
            }
        }
        Some(Drag::FaceSpawn { object, point, pivot, .. }) if held.by(button) => {
            if let Some(to) = pointer.and_then(|p| view.angle_about(pivot, p)) {
                let facing = mg_area::arrange::snap_rotation(to, view.snap.1);
                view.drag = Some(Drag::FaceSpawn { object, point, pivot, facing });
            }
        }
        Some(Drag::Start { turn: true, grip, at, was, .. }) if held.by(button) => {
            if let Some(to) = pointer.and_then(|p| view.angle_about(at, p)) {
                let facing = mg_area::arrange::snap_rotation(to, view.snap.1);
                view.drag = Some(Drag::Start { turn: true, grip, at, facing, was });
            }
        }
        Some(Drag::Start { turn: false, grip, facing, was, .. }) if held.by(button) => {
            if let Some(now) = pointer.and_then(|p| view.ground_at(p, was.0.z)) {
                // On the ground there, as the marker is shown.
                let to = view.snapped((now.truncate() - grip).extend(now.z));
                let z = view.ground.as_ref().and_then(|g| g.height(to.truncate(), to.z));
                let at = to.with_z(z.unwrap_or(to.z));
                view.drag = Some(Drag::Start { turn: false, grip, at, facing, was });
            }
        }
        Some(Drag::Slide { axis, pivot, grip, screen, .. }) if held.by(button) => {
            let along = Vec3::AXES[axis];
            let by = match (screen, pointer) {
                (Some((from, per_point)), Some(p)) => Some((p - from).dot(per_point)),
                (None, Some(p)) => view.distance_along(pivot, along, p).map(|now| now - grip),
                _ => None,
            };
            if let Some(by) = by {
                view.drag = Some(Drag::Slide { axis, pivot, grip, by, screen });
            }
        }
        Some(Drag::Tilt { ring, grip, screen, .. }) if held.by(button) => {
            let half = std::f32::consts::PI;
            let angle = match (screen, pointer) {
                (Some((from, per_point)), Some(p)) => Some((p - from).dot(per_point)),
                (None, Some(p)) => view.angle_on(&ring, p).map(|now| now - grip),
                _ => None,
            };
            if let Some(angle) = angle {
                // The short way round from where it was taken.
                let angle = (angle + half).rem_euclid(std::f32::consts::TAU) - half;
                view.drag = Some(Drag::Tilt { ring, grip, angle, screen });
            }
        }
        Some(Drag::Turn { angle }) if held.by(button) => {
            view.drag = Some(Drag::Turn { angle: angle - held.delta.x * 0.01 });
        }
        _ => {}
    }
    if held.cancelled() {
        // Escape: nothing moves.
        view.drag = None;
    }
    if !held.by(button) {
        match view.drag.take() {
            Some(Drag::Box { from, to }) => {
                let inside = boxed(view, Rect::from_two_pos(from, to));
                if !command {
                    view.selection.clear();
                }
                for h in inside {
                    if !view.selection.contains(&h) {
                        view.selection.push(h);
                    }
                }
            }
            Some(Drag::FaceSpawn { object, point, facing, .. }) => {
                if let Some(o) = view.model.as_ref().map(|m| &m.objects[object])
                    && o.spawn_facings.get(point).is_some_and(|was| *was != facing)
                {
                    let path = mg_edit::GffPath::root()
                        .item(o.kind.list(), o.index)
                        .item("SpawnPointList", point);
                    let edit = mg_edit::Edit::SetField {
                        key: view.git(),
                        path,
                        label: "Orientation".into(),
                        value: Some(mg_gff::Value::Float(facing)),
                    };
                    app.actions.push(Action::Apply(Command::new("Turn Spawn Point", vec![edit])));
                }
            }
            Some(Drag::Start { turn, at, facing, was, .. }) => {
                if turn && facing != was.1 {
                    turn_start_location(app, facing);
                } else if !turn && at != was.0 {
                    move_start_location(app, at);
                }
            }
            Some(drag @ Drag::Tilt { .. }) => {
                view.drag = Some(drag);
                let tilted = view.tilted();
                view.drag = None;
                commit_tilts(app, view, &tilted);
            }
            Some(drag) => {
                view.drag = Some(drag);
                let moved = view.dragged();
                let label = match drag {
                    Drag::Turn { .. } | Drag::Spin { .. } => "Rotate",
                    Drag::Lift { .. } | Drag::Slide { axis: 2, .. } => "Raise",
                    _ => "Move",
                };
                view.drag = None;
                commit_moves(app, view, &moved, label);
            }
            None => {}
        }
    }
    // Q and E (Options › Keyboard) turn the selection by the snapping angle
    // (15° when free), Shift + Q and E by 90°; G drops it to the ground.
    // (While placing, they turn what is being placed instead.)
    if hovered && !typing && !placing && !view.selection.is_empty() {
        use crate::keys::Cmd;
        let keys = app.keymap.clone();
        let pressed = |c: Cmd| ui.input(|i| keys.pressed(i, c));
        for (cmd, by) in turn_keys(view) {
            if pressed(cmd) {
                rotate_selection(app, view, by);
            }
        }
        if pressed(Cmd::DropToGround) {
            drop_to_ground(app, view);
        }
    }
    if hovered && !view.selection.is_empty() && ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        delete(app, view);
    }
}

/// The camera: Ctrl + drag moves it, Ctrl + right or middle drag (or a
/// middle drag) turns it, Shift + middle drag moves it, the wheel zooms,
/// Ctrl + Shift + the wheel or a middle drag moves it up and down;
/// the keys of Options › Keyboard (WASD, the arrows and the number keys).
fn camera_input(
    ui: &egui::Ui,
    view: &mut AreaView,
    response: &egui::Response,
    shift: bool,
    command: bool,
    keys: &crate::keys::Keymap,
) {
    let rect = view.rect;
    // The keys drive the area view the pointer was in last, wherever the
    // pointer is now (over the palette, the module tree).
    let last = egui::Id::new("area-view-camera-keys");
    if response.hovered() {
        ui.data_mut(|d| d.insert_temp(last, view.area));
    }
    let active = ui.data(|d| d.get_temp::<ResRef>(last)) == Some(view.area);
    let held = view.held;
    let Some(o) = &mut view.orbit else { return };
    let d = held.delta;
    // A right drag turns it too, as in most 3D views (Shift + right drag
    // turns the selection, as in Aurora; a right click is still the menu).
    let turning = (held.by(egui::PointerButton::Middle) && !shift)
        || (!shift && held.by(egui::PointerButton::Secondary));
    // What a pixel is on the ground at the target.
    let per_pixel = 2.0 * o.distance * (o.camera().fov_y / 2.0).tan() / rect.height().max(1.0);
    if command && shift && held.by(egui::PointerButton::Middle) {
        // Ctrl + Shift + middle drag: up and down, the view following the
        // pointer.
        raise(o, d.y * per_pixel);
    } else if turning {
        o.yaw -= d.x * 0.01;
        o.pitch = (o.pitch + d.y * 0.01).clamp(0.05, MAX_PITCH);
    } else if (command && held.by(egui::PointerButton::Primary))
        || (shift && held.by(egui::PointerButton::Middle))
    {
        // The ground follows the pointer.
        pan(o, Vec2::new(-d.x, d.y) * per_pixel);
    }
    // (egui turns the wheel sideways with Shift held.)
    // (And with Ctrl held it reports the wheel as a zoom, not a scroll.)
    let zoom_speed = ui.ctx().options(|o| o.input_options.scroll_zoom_speed);
    let (scroll, slow) = ui.input(|i| {
        let d = i.smooth_scroll_delta;
        let mut scroll = if i.modifiers.shift && d.y == 0.0 { d.x } else { d.y };
        if scroll == 0.0 && i.modifiers.command && zoom_speed > 0.0 {
            scroll = i.zoom_delta().ln() / zoom_speed;
        }
        (scroll, shift || i.modifiers.command)
    });
    if scroll != 0.0 && response.hovered() && command && shift {
        // Ctrl + Shift + wheel: up and down, finer the nearer the view.
        raise(o, scroll * 0.0006 * o.distance);
    } else if scroll != 0.0 && response.hovered() {
        let rate = if slow { 0.001 } else { 0.002 };
        o.distance = (o.distance * (-scroll * rate).exp()).clamp(1.0, 2000.0);
    }
    if !active {
        return;
    }
    use crate::keys::Cmd;
    // Letters and digits don't move the camera while a field has the
    // keyboard (in a window over the view).
    let typing = ui.memory(|m| m.focused().is_some());
    let held = |c: Cmd| ui.input(|i| keys.held(i, c, typing));
    let axis =
        |plus: Cmd, minus: Cmd| f32::from(u8::from(held(plus))) - f32::from(u8::from(held(minus)));
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let travel = Vec2::new(
        axis(Cmd::CameraRight, Cmd::CameraLeft),
        axis(Cmd::CameraForward, Cmd::CameraBack),
    );
    let turn = Vec2::new(
        axis(Cmd::CameraTurnRight, Cmd::CameraTurnLeft),
        axis(Cmd::CameraTiltUp, Cmd::CameraTiltDown),
    );
    let rise = axis(Cmd::CameraUp, Cmd::CameraDown);
    if travel != Vec2::ZERO {
        pan(o, travel * o.distance * dt);
    }
    if rise != 0.0 {
        raise(o, rise * o.distance * 0.25 * dt);
    }
    if turn != Vec2::ZERO {
        o.yaw -= turn.x * 1.5 * dt;
        o.pitch = (o.pitch + turn.y * 1.0 * dt).clamp(0.05, MAX_PITCH);
    }
    if travel != Vec2::ZERO || turn != Vec2::ZERO || rise != 0.0 {
        ui.ctx().request_repaint();
    }
    // F10, as in Aurora: select tiles or objects.
    if !typing && response.hovered() && ui.input(|i| keys.pressed(i, Cmd::SelectTiles)) {
        view.tile_mode = !view.tile_mode;
        view.selection.clear();
        view.tile_selection.clear();
    }
    if !typing
        && ui.input(|i| keys.pressed(i, Cmd::Overview))
        && let Some(model) = &view.model
    {
        view.orbit = Some(Orbit::overview(model));
    }
}

/// The objects (shown) whose position shows inside `rect`.
fn boxed(view: &AreaView, rect: Rect) -> Vec<(ObjectKind, usize)> {
    let Some(model) = &view.model else { return Vec::new() };
    model
        .objects
        .iter()
        .filter(|o| view.show[o.kind.index()] && !o.locked)
        .filter(|o| {
            let at = if o.kind.has_outline() && !o.outline.is_empty() {
                o.outline.iter().copied().sum::<Vec3>() / o.outline.len() as f32
            } else {
                o.position
            };
            view.screen_pos(at).is_some_and(|p| rect.contains(p))
        })
        .map(|o| (o.kind, o.index))
        .collect()
}

/// A blueprint dragged from the palette and dropped on the view: placed
/// where it was dropped (a door on the nearest door hook); a trigger or
/// encounter begins its outline there, to be drawn on with clicks.
fn drop_blueprint(app: &mut Moonglow, view: &mut AreaView, response: &egui::Response, key: ResKey) {
    let Some(kind) = ObjectKind::from_restype(key.restype) else { return };
    // (A widget being dragged leaves the view unhovered: the pointer's place.)
    let pointer = response.ctx.input(|i| i.pointer.latest_pos());
    let Some(at) = pointer.and_then(|pos| view.ground_at(pos, 0.0)) else { return };
    if kind.has_outline() {
        app.palette.selected = Some(key);
        view.outline = vec![at];
        app.log.info("Click the outline's corners; double-click to close it");
        return;
    } else if kind == ObjectKind::Door {
        match view.model.as_ref().and_then(|m| m.hook_near(at, DOOR_REACH)).copied() {
            Some(h) => place(app, view, key, h.position, h.bearing, &[]),
            None => app.log.warn("Doors go on door hooks: drop it near one"),
        }
    } else {
        place(app, view, key, at, view.ghost_turn, &[]);
    }
    // Placed, as a click places it: the palette's choice is let go (with
    // Shift it stays), so what was dropped can be picked up and moved
    // rather than a click placing another.
    if !response.ctx.input(|i| i.modifiers.shift) {
        app.palette.selected = None;
    }
}

/// How many blueprints the palette's Recent keeps.
const RECENT: usize = 12;

/// The palette's chosen blueprint, if it is one to place.
fn brush(app: &Moonglow) -> Option<ResKey> {
    app.palette.selected.filter(|k| ObjectKind::from_restype(k.restype).is_some())
}

/// Places blueprint `key` at `at` (a trigger or encounter along `outline`,
/// standing at its first point), as Aurora does, and selects it.
fn place(
    app: &mut Moonglow,
    view: &mut AreaView,
    key: ResKey,
    at: Vec3,
    rotation: f32,
    outline: &[Vec3],
) {
    use mg_module::instances::{OUTLINE_LIFT, Placement};
    let Some(kind) = ObjectKind::from_restype(key.restype) else { return };
    // On the grid (doors go on their hooks; outlines are drawn point by point).
    let at = if kind == ObjectKind::Door || kind.has_outline() { at } else { view.snapped(at) };
    let ground = |p: Vec3| view.ground.as_ref().and_then(|g| g.height(p.truncate(), p.z));
    let position = at + Vec3::Z * lift(kind);
    let relative: Vec<[f32; 3]> = outline
        .iter()
        .map(|p| [p.x - at.x, p.y - at.y, ground(*p).unwrap_or(p.z) + OUTLINE_LIFT])
        .collect();
    let placement = Placement { position: position.to_array(), rotation };
    let item = match instance_of(app, key, placement, &relative) {
        Ok(Some(item)) => item,
        Ok(None) => return,
        Err(e) => {
            app.log.error(e);
            return;
        }
    };
    let Some(ws) = app.ws.as_mut() else { return };
    let git = view.git();
    let index = ws.doc(&git).ok().and_then(|g| g.root.list(kind.list())).map_or(0, <[_]>::len);
    let edit = mg_edit::Edit::InsertItem {
        key: git,
        path: mg_edit::GffPath::root(),
        list: kind.list().into(),
        index,
        item,
    };
    app.actions.push(Action::Apply(Command::new(format!("Place {}", key.resref), vec![edit])));
    view.selection = vec![(kind, index)];
    // Remembered in the palette's Recent, the last first.
    let recent = &mut app.settings.palette_recent;
    let r = crate::palette_view::remembered(key);
    recent.retain(|x| *x != r);
    recent.insert(0, r);
    recent.truncate(RECENT);
}

/// The turn keys and how far each turns (radians, anticlockwise): Q and E
/// by the snapping angle (15° when free), Shift + Q and E by 90°.
fn turn_keys(view: &AreaView) -> [(crate::keys::Cmd, f32); 4] {
    use crate::keys::Cmd;
    let step = view.snap.1.unwrap_or(15.0).to_radians();
    [
        (Cmd::TurnLeft, step),
        (Cmd::TurnRight, -step),
        (Cmd::TurnLeft90, std::f32::consts::FRAC_PI_2),
        (Cmd::TurnRight90, -std::f32::consts::FRAC_PI_2),
    ]
}

/// The twelve edges of the box `min`–`max` placed by `t`.
fn box_edges(t: glam::Mat4, min: Vec3, max: Vec3) -> impl Iterator<Item = (Vec3, Vec3)> {
    let c = move |i: usize| {
        t.transform_point3(Vec3::new(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        ))
    };
    [(0, 1), (1, 3), (3, 2), (2, 0), (4, 5), (5, 7), (7, 6), (6, 4), (0, 4), (1, 5), (2, 6), (3, 7)]
        .into_iter()
        .map(move |(a, b)| (c(a), c(b)))
}

/// The farthest the game's camera stands from the player (metres; the
/// classic camera's limit, NWScript's SetCameraFacing): the view's fog is
/// measured from that near its target.
const GAME_CAMERA: f32 = 20.0;

/// The game's fog `f` as a view `distance` from its target shows it. The
/// game measures fog from its camera, which stays near the player (at most
/// [`GAME_CAMERA`] off); the view often looks from much farther, where the
/// game's fog would hide what it looks at. So the fog is the game's as a
/// camera that near the target would see it: what is around the target
/// clear, the distance fading as in the game.
fn view_fog(f: mg_render::Fog, distance: f32) -> mg_render::Fog {
    let beyond = (distance - GAME_CAMERA).max(0.0);
    mg_render::Fog { start: f.start + beyond, end: f.end + beyond, ..f }
}

/// How opaque the tiles a tile brush's click would make are drawn.
const PREVIEW_OPACITY: f32 = 0.85;

/// How opaque a blueprint about to be placed is drawn.
const GHOST_OPACITY: f32 = 0.6;

/// How far above the ground an object of `kind` is placed (sounds, at
/// Aurora's height).
fn lift(kind: ObjectKind) -> f32 {
    if kind == ObjectKind::Sound { mg_area::SOUND_HEIGHT } else { 0.0 }
}

/// Blueprint `key` as an instance at `placement` (`relative`: an outline
/// from there), its blueprint (and items) as their editors have them, not
/// as last saved; an error if the blueprint can't be read.
fn instance_of(
    app: &mut Moonglow,
    key: ResKey,
    placement: mg_module::instances::Placement,
    relative: &[[f32; 3]],
) -> Result<Option<mg_gff::Struct>, String> {
    use mg_module::instances::{Placing, instance};
    let (Some(game), Some(ws)) = (app.game.as_deref(), app.ws.as_mut()) else { return Ok(None) };
    let _ = ws.flush();
    let read = |k: ResKey| -> Option<mg_gff::Struct> {
        let data = ws
            .module
            .get(&k)
            .map(<[u8]>::to_vec)
            .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let blueprint = read(key).ok_or_else(|| format!("{key}: not found or not readable"))?;
    let items = |r: ResRef| read(ResKey::new(r, ResType::UTI));
    let placing = Placing { game, item: &items };
    Ok(instance(&placing, key.restype, &blueprint, placement, relative))
}

/// The blueprint about to be placed, where it would go if the pointer
/// placed it now: one dragged (from the palette or the module tree) over
/// the view, else the palette's choice while the pointer is over it.
/// Snapped to the grid, a door on the nearest hook; not for triggers and
/// encounters (drawn point by point).
fn ghost(app: &mut Moonglow, ui: &egui::Ui, view: &mut AreaView) -> Option<mg_area::AreaObject> {
    let dragged = egui::DragAndDrop::payload::<crate::palette_view::Dragged>(ui.ctx());
    let key = dragged.map(|d| d.0).or_else(|| brush(app));
    let Some(key) = key.filter(|_| !view.pasting && view.outline.is_empty()) else {
        view.ghost = None;
        return None;
    };
    let kind = ObjectKind::from_restype(key.restype).filter(|k| !k.has_outline())?;
    // Over the view, not over a window in front of it.
    let pos = ui.ctx().pointer_hover_pos().filter(|p| view.rect.contains(*p))?;
    if ui
        .ctx()
        .layer_id_at(pos)
        .is_some_and(|l| l != ui.layer_id() && l.order != egui::Order::Tooltip)
    {
        return None;
    }
    let at = view.ground_at(pos, 0.0)?;
    let (at, rotation) = if kind == ObjectKind::Door {
        let hook = view.model.as_ref()?.hook_near(at, DOOR_REACH)?;
        (hook.position, Some(hook.bearing))
    } else {
        (view.snapped(at), None)
    };
    if view.ghost.as_ref().is_none_or(|(k, _)| *k != key) {
        let placement = mg_module::instances::Placement { position: [0.0; 3], rotation: 0.0 };
        let object = instance_of(app, key, placement, &[]).ok().flatten().and_then(|item| {
            let game = app.game.as_deref()?;
            Some(mg_area::AreaObject::read(game, kind, usize::MAX, &item))
        });
        view.ghost = Some((key, object));
        // A blueprint chosen anew starts unturned.
        view.ghost_turn = 0.0;
    }
    let mut o = view.ghost.as_ref()?.1.clone()?;
    o.position = at + Vec3::Z * lift(kind);
    // A door faces as its hook does; the others as Q and E turned them.
    o.rotation = rotation.unwrap_or(o.rotation + view.ghost_turn);
    Some(o)
}

/// Moves the camera's target along the ground: `by.x` to the right of the
/// view, `by.y` ahead.
fn pan(o: &mut Orbit, by: Vec2) {
    let ahead = Vec3::new(-o.yaw.cos(), -o.yaw.sin(), 0.0);
    let right = Vec3::new(ahead.y, -ahead.x, 0.0);
    o.target += right * by.x + ahead * by.y;
}

/// The lowest and highest the camera's target goes, metres: under the
/// deepest pits and over the tallest tiles.
const TARGET_HEIGHTS: (f32, f32) = (-50.0, 200.0);

/// Moves the camera's target (and so the camera) up by `by` metres.
fn raise(o: &mut Orbit, by: f32) {
    o.target.z = (o.target.z + by).clamp(TARGET_HEIGHTS.0, TARGET_HEIGHTS.1);
}

/// One command that puts each object (by its position in the model) where
/// the drag left it.
fn commit_moves(app: &mut Moonglow, view: &AreaView, moved: &[(usize, Vec3, f32)], label: &str) {
    let (Some(model), Some(ws)) = (&view.model, app.ws.as_mut()) else { return };
    let git = view.git();
    let Ok(doc) = ws.doc(&git) else { return };
    let mut edits = Vec::new();
    for &(i, position, rotation) in moved {
        let o = &model.objects[i];
        let Some(s) = doc.root.list(o.kind.list()).and_then(|l| l.get(o.index)) else { continue };
        edits.extend(mg_area::edit::move_edits(git, o, s, position, rotation));
    }
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new(label, edits)));
    }
}

/// One command that gives each object (by its position in the model) the
/// visual transform a tilt left it.
fn commit_tilts(app: &mut Moonglow, view: &AreaView, tilted: &[(usize, mg_area::VisualTransform)]) {
    let (Some(model), Some(ws)) = (&view.model, app.ws.as_mut()) else { return };
    let git = view.git();
    let Ok(doc) = ws.doc(&git) else { return };
    let mut edits = Vec::new();
    for &(i, visual) in tilted {
        let o = &model.objects[i];
        let Some(s) = doc.root.list(o.kind.list()).and_then(|l| l.get(o.index)) else { continue };
        edits.extend(mg_area::edit::visual_transform_edits(git, o, s, visual));
    }
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Tilt", edits)));
    }
}

/// Opens (or shows) the Properties of the placed object `index` of `kind`.
fn open_properties(app: &mut Moonglow, view: &AreaView, kind: ObjectKind, index: usize) {
    let path = mg_edit::GffPath::root().item(kind.list(), index);
    app.actions.push(Action::OpenTab(crate::Tab::Instance { area: view.area, path }));
}

fn delete(app: &mut Moonglow, view: &mut AreaView) {
    let edits = mg_area::edit::delete_edits(view.git(), &view.selection);
    view.selection.clear();
    // Objects after the deleted ones move up their lists: their Properties
    // would show others.
    let area = view.area;
    app.dock.retain_tabs(|t| {
        !matches!(t, crate::Tab::Instance { area: a, .. } | crate::Tab::Instances { area: a, .. } if *a == area)
    });
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Delete", edits)));
    }
}

/// Save as Prefab for the selection of `area`'s view (the palette's
/// Prefabs asks for it): whether there was a selection to save.
pub(crate) fn save_selection_as_prefab(app: &mut Moonglow, area: ResRef) -> bool {
    let Some(view) = app.area_views.remove(&area) else { return false };
    let clip = copy_selection(app, &view);
    app.area_views.insert(area, view);
    let Some(clip) = clip else { return false };
    app.prefab_save = Some((String::new(), clip));
    true
}

/// The selected objects, with their GIT structs and heights above the
/// ground, for the clipboard.
fn copy_selection(app: &mut Moonglow, view: &AreaView) -> Option<ObjectClip> {
    let model = view.model.as_ref()?;
    let doc = app.ws.as_mut()?.doc(&view.git()).ok()?;
    let mut objects = Vec::new();
    for &(kind, index) in &view.selection {
        let (Some(o), Some(s)) =
            (model.object(kind, index), doc.root.list(kind.list()).and_then(|l| l.get(index)))
        else {
            continue;
        };
        let ground =
            view.ground.as_ref().and_then(|g| g.height(o.position.truncate(), o.position.z));
        let lift = match o.kind {
            ObjectKind::Creature => 0.0,
            _ => ground.map_or(0.0, |z| o.position.z - z),
        };
        objects.push((o.clone(), s.clone(), lift));
    }
    let anchor = objects.first()?.0.position;
    Some(ObjectClip { objects, anchor })
}

/// Where each copied object would stand with the first at `at` (the ground
/// under the pointer): the same places around it, the same heights above
/// the ground (outlines at their own).
fn pasted_positions(view: &AreaView, clip: &ObjectClip, at: Vec3) -> Vec<Vec3> {
    clip.objects
        .iter()
        .map(|(o, _, lift)| {
            let xy = at.truncate() + (o.position - clip.anchor).truncate();
            if o.kind.has_outline() {
                return xy.extend(o.position.z);
            }
            let ground = view.ground.as_ref().and_then(|g| g.height(xy, at.z));
            xy.extend(ground.unwrap_or(at.z) + lift)
        })
        .collect()
}

/// Places copies of the clipboard's objects with the first at `at` (one
/// command), and selects them.
fn paste_at(app: &mut Moonglow, view: &mut AreaView, at: Vec3) {
    let Some(clip) = app.object_clip.clone() else { return };
    let at = view.snapped(at);
    let git = view.git();
    let Some(ws) = app.ws.as_mut() else { return };
    let Ok(doc) = ws.doc(&git) else { return };
    let mut counts: std::collections::HashMap<ObjectKind, usize> = ObjectKind::ALL
        .into_iter()
        .map(|k| (k, doc.root.list(k.list()).map_or(0, <[_]>::len)))
        .collect();
    let mut edits = Vec::new();
    let mut selection = Vec::new();
    for ((o, s, _), position) in clip.objects.iter().zip(pasted_positions(view, &clip, at)) {
        let count = counts.get_mut(&o.kind).expect("every kind");
        edits.push(mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: o.kind.list().into(),
            index: *count,
            item: mg_area::edit::moved(o, s, position, o.rotation),
        });
        selection.push((o.kind, *count));
        *count += 1;
    }
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Paste", edits)));
        view.selection = selection;
    }
}

/// Whether an object of this kind turns (outlines and sounds don't).
fn turns(kind: ObjectKind) -> bool {
    !kind.has_outline() && kind != ObjectKind::Sound
}

/// Turns each selected object in place by `by` radians (one command).
fn rotate_selection(app: &mut Moonglow, view: &AreaView, by: f32) {
    let Some(model) = &view.model else { return };
    let moved: Vec<(usize, Vec3, f32)> = view
        .selection
        .iter()
        .filter_map(|&(k, i)| view.object_at(k, i))
        .filter(|&i| turns(model.objects[i].kind))
        .map(|i| {
            let o = &model.objects[i];
            let r = mg_area::arrange::snap_rotation(o.rotation + by, view.snap.1);
            (i, o.position, r)
        })
        .collect();
    commit_moves(app, view, &moved, "Rotate");
}

/// Puts the selected objects on the ground under them (creatures and
/// outlines are there already).
fn drop_to_ground(app: &mut Moonglow, view: &AreaView) {
    let (Some(model), Some(ground)) = (&view.model, &view.ground) else { return };
    let moved: Vec<(usize, Vec3, f32)> = view
        .selection
        .iter()
        .filter_map(|&(k, i)| view.object_at(k, i))
        .filter_map(|i| {
            let o = &model.objects[i];
            if o.kind == ObjectKind::Creature || o.kind.has_outline() {
                return None;
            }
            // The ground below it, else the nearest.
            let xy = o.position.truncate();
            let z = ground
                .height(xy, o.position.z - 0.01)
                .or_else(|| ground.height(xy, o.position.z))?;
            ((z - o.position.z).abs() > 1e-4).then_some((i, xy.extend(z), o.rotation))
        })
        .collect();
    commit_moves(app, view, &moved, "Drop to Ground");
}

/// Arranges the selection (lined up, spaced out, facing alike, mirrored),
/// keeping each object's height above the ground.
fn arrange_selection(
    app: &mut Moonglow,
    view: &AreaView,
    how: mg_area::arrange::Arrange,
    label: &str,
) {
    let Some(model) = &view.model else { return };
    let chosen: Vec<usize> =
        view.selection.iter().filter_map(|&(k, i)| view.object_at(k, i)).collect();
    let before: Vec<(Vec2, f32)> = chosen
        .iter()
        .map(|&i| (model.objects[i].position.truncate(), model.objects[i].rotation))
        .collect();
    let after = mg_area::arrange::arrange(how, &before);
    let moved: Vec<(usize, Vec3, f32)> = chosen
        .iter()
        .zip(after)
        .map(|(&i, (xy, r))| {
            let o = &model.objects[i];
            let position = view.moved(o, xy - o.position.truncate());
            (i, position, if turns(o.kind) { r } else { o.rotation })
        })
        .collect();
    commit_moves(app, view, &moved, label);
}

/// Locks the selected objects (or, with `false`, unlocks them) so clicks
/// and boxes pass them by.
fn set_locked(
    app: &mut Moonglow,
    view: &mut AreaView,
    objects: &[(ObjectKind, usize)],
    lock: bool,
) {
    let git = view.git();
    let edits: Vec<mg_edit::Edit> = objects
        .iter()
        .map(|&(kind, index)| mg_edit::Edit::SetField {
            key: git,
            path: mg_edit::GffPath::root().item(kind.list(), index),
            label: mg_area::LOCKED.into(),
            value: lock.then_some(Value::Byte(1)),
        })
        .collect();
    if lock {
        view.selection.clear();
    }
    if !edits.is_empty() {
        let label = if lock { "Lock" } else { "Unlock" };
        app.actions.push(Action::Apply(Command::new(label, edits)));
    }
}

/// The context menu of the selection: Properties, Adjust Location, what
/// the object's type offers (Aurora's `pmViewerArea`), Variables, Delete.
fn context_menu(app: &mut Moonglow, view: &mut AreaView, ui: &mut egui::Ui) {
    let single = match view.selection.as_slice() {
        [one] => Some(*one),
        _ => None,
    };
    let same_kind =
        view.selection.len() > 1 && view.selection.iter().all(|(k, _)| *k == view.selection[0].0);
    if ui.add_enabled(single.is_some() || same_kind, egui::Button::new("Properties")).clicked() {
        match single {
            Some((kind, index)) => open_properties(app, view, kind, index),
            None => {
                let paths = view
                    .selection
                    .iter()
                    .map(|(k, i)| mg_edit::GffPath::root().item(k.list(), *i))
                    .collect();
                app.actions.push(Action::OpenTab(crate::Tab::Instances { area: view.area, paths }));
            }
        }
        ui.close();
    }
    let any = !view.selection.is_empty();
    if ui.add_enabled(any, egui::Button::new("Adjust Location…")).clicked() {
        app.adjust = crate::area_tools::adjust(view);
        ui.close();
    }
    if ui.add_enabled(any, egui::Button::new("Drop to Ground (G)")).clicked() {
        drop_to_ground(app, view);
        ui.close();
    }
    // Placeables: static (part of the scenery; the game gives those no
    // visual transform, so they don't tilt) or dynamic.
    let placeables: Vec<(usize, bool, bool)> = view.model.as_ref().map_or_else(Vec::new, |m| {
        m.objects
            .iter()
            .filter(|o| o.kind == ObjectKind::Placeable)
            .filter(|o| view.selection.contains(&(o.kind, o.index)))
            .map(|o| (o.index, o.is_static, o.visual.is_some()))
            .collect()
    });
    let path = |i: usize| mg_edit::GffPath::root().item(ObjectKind::Placeable.list(), i);
    let set = |i: usize, label: &str, value: Option<mg_gff::Value>| mg_edit::Edit::SetField {
        key: view.git(),
        path: path(i),
        label: label.into(),
        value,
    };
    if placeables.iter().any(|p| p.1)
        && ui
            .button("Make Dynamic")
            .on_hover_text(
                "Clears Static, so that the placeable's model can be tilted and scaled. \
                 A static placeable is part of the scenery: the game draws it and finds \
                 paths around it more cheaply",
            )
            .clicked()
    {
        let fixed = placeables.iter().filter(|p| p.1);
        let edits = fixed.map(|p| set(p.0, "Static", Some(mg_gff::Value::Byte(0)))).collect();
        app.actions.push(Action::Apply(Command::new("Make Dynamic", edits)));
        ui.close();
    }
    if placeables.iter().any(|p| !p.1)
        && ui
            .button("Make Static")
            .on_hover_text(
                "Sets Static: part of the scenery, which can't be used, tilted or scaled \
                 (its visual transform goes, as in Aurora)",
            )
            .clicked()
    {
        let mut edits = Vec::new();
        for p in placeables.iter().filter(|p| !p.1) {
            edits.push(set(p.0, "Static", Some(mg_gff::Value::Byte(1))));
            if p.2 {
                edits.push(set(p.0, "VisTransformList", None));
                edits.push(set(p.0, "VisualTransform", None));
            }
        }
        app.actions.push(Action::Apply(Command::new("Make Static", edits)));
        ui.close();
    }
    ui.add_enabled_ui(view.selection.len() > 1, |ui| {
        ui.menu_button("Arrange", |ui| {
            use mg_area::arrange::Arrange;
            let items = [
                (
                    Arrange::LineUpWestEast,
                    "Line Up West–East",
                    "On a west–east line through the first selected",
                ),
                (
                    Arrange::LineUpSouthNorth,
                    "Line Up South–North",
                    "On a south–north line through the first selected",
                ),
                (
                    Arrange::Distribute,
                    "Space Evenly",
                    "Evenly spaced between the two farthest apart",
                ),
                (Arrange::SameFacing, "Face Alike", "Facing as the first selected does"),
                (
                    Arrange::MirrorWestEast,
                    "Mirror West–East",
                    "Mirrored about the selection's middle",
                ),
                (
                    Arrange::MirrorSouthNorth,
                    "Mirror South–North",
                    "Mirrored about the selection's middle",
                ),
            ];
            for (how, text, hint) in items {
                if ui.button(text).on_hover_text(hint).clicked() {
                    arrange_selection(app, view, how, text);
                    ui.close();
                }
            }
        });
    });
    if ui
        .add_enabled(any, egui::Button::new("Lock"))
        .on_hover_text(
            "Clicks and boxes pass locked objects by (Unlock All, on the area, frees them)",
        )
        .clicked()
    {
        let chosen = view.selection.clone();
        set_locked(app, view, &chosen, true);
        ui.close();
    }
    let locked: Vec<(ObjectKind, usize)> = view
        .model
        .as_ref()
        .map(|m| m.objects.iter().filter(|o| o.locked).map(|o| (o.kind, o.index)).collect())
        .unwrap_or_default();
    if !locked.is_empty() && ui.button(format!("Unlock All ({})", locked.len())).clicked() {
        set_locked(app, view, &locked, false);
        ui.close();
    }
    if ui.add_enabled(any, egui::Button::new("Save as Prefab…")).clicked() {
        app.prefab_save = copy_selection(app, view).map(|clip| (String::new(), clip));
        ui.close();
    }
    let kinds: Vec<ObjectKind> = view.selection.iter().map(|(k, _)| *k).collect();
    let all = |k: ObjectKind| !kinds.is_empty() && kinds.iter().all(|x| *x == k);
    if all(ObjectKind::Door) {
        if ui.button("Reverse Door").clicked() {
            reverse_doors(app, view);
            ui.close();
        }
        ui.menu_button("Initial State", |ui| {
            for (state, text) in [(1, "Opened Forward"), (2, "Opened Backward"), (0, "Closed")] {
                if ui.button(text).clicked() {
                    set_on_selection(
                        app,
                        view,
                        "Initial state",
                        "AnimationState",
                        Value::Byte(state),
                    );
                    ui.close();
                }
            }
        });
    }
    if all(ObjectKind::Placeable) {
        ui.menu_button("Initial State", |ui| {
            for (state, text) in [
                (0, "Default"),
                (1, "Opened"),
                (2, "Closed"),
                (3, "Destroyed"),
                (4, "Activated"),
                (5, "Deactivated"),
            ] {
                if ui.button(text).clicked() {
                    set_on_selection(
                        app,
                        view,
                        "Initial state",
                        "AnimationState",
                        Value::Byte(state),
                    );
                    ui.close();
                }
            }
        });
    }
    if all(ObjectKind::Sound) {
        if ui.button("Mute").clicked() {
            set_on_selection(app, view, "Mute", "Active", Value::Byte(0));
            ui.close();
        }
        if ui.button("Turn On").clicked() {
            set_on_selection(app, view, "Turn on", "Active", Value::Byte(1));
            ui.close();
        }
    }
    if let Some((kind, index)) = single.filter(|(k, _)| k.has_outline()) {
        if ui.button("Redraw Polygon").clicked() {
            view.redraw = Some((kind, index));
            view.outline.clear();
            ui.close();
        }
        if kind == ObjectKind::Encounter
            && let Some(at) = view.menu_at
            && ui.button("Add Spawn Point").clicked()
        {
            add_spawn_point(app, view, index, at);
            ui.close();
        }
    }
    let picked = single.and_then(|(k, i)| view.model.as_ref()?.object(k, i).cloned());
    if let Some(o) = &picked
        && let Some(dialog) = o.conversation
        && ui.button("Conversation").clicked()
    {
        let key = ResKey::new(dialog, ResType::DLG);
        let local = app.ws.as_ref().is_some_and(|w| w.module.contains(&key));
        app.actions.push(Action::OpenTab(if local {
            crate::Tab::Dialog(key)
        } else {
            crate::Tab::Resource(key)
        }));
        ui.close();
    }
    if let Some((kind, index)) = single.filter(|(k, _)| {
        matches!(k, ObjectKind::Creature | ObjectKind::Placeable | ObjectKind::Store)
    }) && ui.button("Inventory").clicked()
    {
        let path = mg_edit::GffPath::root().item(kind.list(), index);
        app.blueprint_pages.insert((view.git(), path.clone()), "Inventory");
        app.actions.push(Action::OpenTab(crate::Tab::Instance { area: view.area, path }));
        ui.close();
    }
    if let Some((kind, index)) = single
        && ui.button("Add to Palette").clicked()
    {
        add_to_palette(app, view, kind, index);
        ui.close();
    }
    if let Some((ObjectKind::Creature, index)) = single
        && let Some(at) = view.menu_at
        && ui.button("Create Waypoint").clicked()
    {
        create_waypoint(app, view, index, at);
        ui.close();
    }
    if let Some((ObjectKind::Creature, index)) = single
        && ui.button("Levelup Wizard…").clicked()
    {
        let path = mg_edit::GffPath::root().item(ObjectKind::Creature.list(), index);
        crate::levelup_view::open(app, view.git(), path);
        ui.close();
    }
    if let Some((kind @ (ObjectKind::Creature | ObjectKind::Placeable), index)) = single
        && ui.button("Setup Store…").clicked()
    {
        crate::store_wizard::open(app, view.area, kind.list(), index);
        ui.close();
    }
    if let Some((ObjectKind::Placeable, index)) = single
        && ui.button("Add Popup Text…").clicked()
    {
        app.popup_text = Some(crate::store_wizard::PopupText {
            area: view.area,
            placeable: index,
            text: String::new(),
            name: String::new(),
        });
        ui.close();
    }
    if all(ObjectKind::Waypoint) && ui.button("Create Set…").clicked() {
        view.set_name = Some(String::new());
        ui.close();
    }
    if let Some((kind, index)) = single
        && ui.button("Variables…").clicked()
    {
        let path = mg_edit::GffPath::root().item(kind.list(), index);
        let list = app
            .ws
            .as_mut()
            .and_then(|ws| ws.doc(&view.git()).ok())
            .and_then(|g| path.get(&g.root))
            .and_then(|s| s.list("VarTable"))
            .unwrap_or(&[])
            .to_vec();
        let target = crate::widgets::FieldTarget::new(view.git(), path, "VarTable");
        app.var_edit = Some(crate::widgets::VarTableEdit::new(target, &list));
        ui.close();
    }
    ui.separator();
    if ui.add_enabled(any, egui::Button::new("Delete")).clicked() {
        delete(app, view);
        ui.close();
    }
    ui.separator();
    // The module's start location, here, facing the way the camera looks
    // (Aurora places it from the palette).
    if let Some(at) = view.menu_at
        && ui
            .button("Set Start Location Here")
            .on_hover_text(
                "Where a player's character enters the module, facing the way the view looks \
                 (the blue ring with the red arrow)",
            )
            .clicked()
    {
        let facing = view.orbit.as_ref().map_or(0.0, |o| o.yaw + std::f32::consts::PI);
        set_start_location(app, view, at, facing);
        ui.close();
    }
    // Test the module as it is now, starting here, facing the way the
    // camera looks.
    if let Some(at) = view.menu_at
        && ui
            .button("Test From Here")
            .on_hover_text("Start the game here, with the module as it is now (not saved)")
            .clicked()
    {
        let facing = view.orbit.as_ref().map_or(0.0, |o| o.yaw + std::f32::consts::PI);
        app.actions.push(Action::TestFromHere { area: view.area, at: at.to_array(), facing });
        ui.close();
    }
}

/// The start location's ring, meters across its radius.
const START_RING: f32 = 1.0;

/// Makes `at`, in the view's area, the module's start location (the
/// IFO's `Mod_Entry_*`), facing `facing` radians counter-clockwise from
/// east, as one command.
fn set_start_location(app: &mut Moonglow, view: &mut AreaView, at: Vec3, facing: f32) {
    let mut edits = vec![start_field("Mod_Entry_Area", Value::resref(view.area))];
    edits.extend(start_at(at));
    edits.extend(start_facing(facing));
    app.actions.push(Action::Apply(Command::new("Set start location", edits)));
    view.show_start = true;
}

/// Moves the start location to `at`, in the area it is in (a drag of its
/// marker).
fn move_start_location(app: &mut Moonglow, at: Vec3) {
    let edits = start_at(at).to_vec();
    app.actions.push(Action::Apply(Command::new("Move Start Location", edits)));
}

/// Turns the start location to face `facing` (a drag of its arrow's tip).
fn turn_start_location(app: &mut Moonglow, facing: f32) {
    let edits = start_facing(facing).to_vec();
    app.actions.push(Action::Apply(Command::new("Turn Start Location", edits)));
}

/// An edit of one of the IFO's fields.
fn start_field(label: &str, value: Value) -> mg_edit::Edit {
    mg_edit::Edit::SetField {
        key: ResKey::new(ResRef::from_str("module").expect("valid"), ResType::IFO),
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(value),
    }
}

fn start_at(at: Vec3) -> [mg_edit::Edit; 3] {
    [
        start_field("Mod_Entry_X", Value::Float(at.x)),
        start_field("Mod_Entry_Y", Value::Float(at.y)),
        start_field("Mod_Entry_Z", Value::Float(at.z)),
    ]
}

fn start_facing(facing: f32) -> [mg_edit::Edit; 2] {
    [
        start_field("Mod_Entry_Dir_X", Value::Float(facing.cos())),
        start_field("Mod_Entry_Dir_Y", Value::Float(facing.sin())),
    ]
}

/// Sets a field on every selected object (one command).
fn set_on_selection(app: &mut Moonglow, view: &AreaView, what: &str, label: &str, value: Value) {
    let edits = view
        .selection
        .iter()
        .map(|&(kind, index)| mg_edit::Edit::SetField {
            key: view.git(),
            path: mg_edit::GffPath::root().item(kind.list(), index),
            label: label.into(),
            value: Some(value.clone()),
        })
        .collect();
    app.actions.push(Action::Apply(Command::new(what, edits)));
}

/// Turns the selected doors half round (Aurora's Reverse Door).
fn reverse_doors(app: &mut Moonglow, view: &AreaView) {
    let moved: Vec<(usize, Vec3, f32)> = view
        .selection
        .iter()
        .filter_map(|&(k, i)| {
            let o = view.model.as_ref()?.object(k, i)?;
            let at = view.object_at(k, i)?;
            let turned = (o.rotation + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
            Some((at, o.position, turned))
        })
        .collect();
    commit_moves(app, view, &moved, "Reverse door");
}

/// Adds a spawn point to encounter `index` at `at` (Aurora's Add Spawn
/// Point): a SpawnPointList entry (struct 2) in area coordinates.
fn add_spawn_point(app: &mut Moonglow, view: &AreaView, index: usize, at: Vec3) {
    let Some(ws) = app.ws.as_mut() else { return };
    let path = mg_edit::GffPath::root().item(ObjectKind::Encounter.list(), index);
    let count = ws
        .doc(&view.git())
        .ok()
        .and_then(|g| path.get(&g.root))
        .and_then(|s| s.list("SpawnPointList"))
        .map_or(0, <[_]>::len);
    let mut point = mg_gff::Struct::new(2);
    for (label, v) in [("X", at.x), ("Y", at.y), ("Z", at.z), ("Orientation", 0.0)] {
        point.set(label, Value::Float(v));
    }
    let edit = mg_edit::Edit::InsertItem {
        key: view.git(),
        path,
        list: "SpawnPointList".into(),
        index: count,
        item: point,
    };
    app.actions.push(Action::Apply(Command::new("Add spawn point", vec![edit])));
}

/// Replaces a trigger's or encounter's outline with `outline` (relative to
/// where the object stands; each point on the ground plus Aurora's lift).
fn redraw_outline(
    app: &mut Moonglow,
    view: &AreaView,
    kind: ObjectKind,
    index: usize,
    outline: &[Vec3],
) {
    let Some(o) = view.model.as_ref().and_then(|m| m.object(kind, index)) else { return };
    let (id, labels) = match kind {
        ObjectKind::Trigger => (3, ["PointX", "PointY", "PointZ"]),
        _ => (1, ["X", "Y", "Z"]),
    };
    let lift = mg_module::instances::OUTLINE_LIFT;
    let points = outline
        .iter()
        .map(|p| {
            let ground = view.ground.as_ref().and_then(|g| g.height(p.truncate(), p.z));
            let mut s = mg_gff::Struct::new(id);
            let c = [p.x - o.position.x, p.y - o.position.y, ground.unwrap_or(p.z) + lift];
            for (label, v) in labels.iter().zip(c) {
                s.set(label, Value::Float(v));
            }
            s
        })
        .collect();
    let edit = mg_edit::Edit::SetField {
        key: view.git(),
        path: mg_edit::GffPath::root().item(kind.list(), index),
        label: "Geometry".into(),
        value: Some(Value::List(points)),
    };
    app.actions.push(Action::Apply(Command::new("Redraw polygon", vec![edit])));
}

/// Every waypoint tag in the module's areas.
fn waypoint_tags(app: &mut Moonglow) -> Vec<String> {
    let Some(ws) = app.ws.as_mut() else { return Vec::new() };
    let areas: Vec<ResRef> = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
    let mut tags = Vec::new();
    for a in areas {
        let Ok(git) = ws.doc(&ResKey::new(a, ResType::GIT)) else { continue };
        for w in git.root.list(ObjectKind::Waypoint.list()).unwrap_or(&[]) {
            if let Some(t) = w.string("Tag") {
                tags.push(String::from_utf8_lossy(t).into_owned());
            }
        }
    }
    tags
}

/// Aurora's Create Waypoint: one of the creature's walk waypoints
/// (`WP_<its tag>_NN`) where the menu was opened.
fn create_waypoint(app: &mut Moonglow, view: &mut AreaView, creature: usize, at: Vec3) {
    let git = view.git();
    let tags = waypoint_tags(app);
    let Some(ws) = app.ws.as_mut() else { return };
    let Ok(doc) = ws.doc(&git) else { return };
    let tag = doc
        .root
        .list(ObjectKind::Creature.list())
        .and_then(|l| l.get(creature))
        .and_then(|c| c.string("Tag"))
        .map(|t| String::from_utf8_lossy(t).into_owned())
        .unwrap_or_default();
    let index = doc.root.list(ObjectKind::Waypoint.list()).map_or(0, <[_]>::len);
    let tag = mg_module::instances::set_tag(&format!("WP_{tag}"), &tags);
    let item = mg_module::instances::walk_waypoint(&tag, at.to_array());
    let edit = mg_edit::Edit::InsertItem {
        key: git,
        path: mg_edit::GffPath::root(),
        list: ObjectKind::Waypoint.list().into(),
        index,
        item,
    };
    app.actions.push(Action::Apply(Command::new("Create waypoint", vec![edit])));
    view.selection = vec![(ObjectKind::Waypoint, index)];
}

/// Aurora's Create Set: the selected waypoints named `<name>_01`,
/// `<name>_02`, … in the order they were selected.
fn create_set(app: &mut Moonglow, view: &AreaView, name: &str) {
    let mut tags = waypoint_tags(app);
    let mut edits = Vec::new();
    for &(kind, index) in &view.selection {
        if kind != ObjectKind::Waypoint {
            continue;
        }
        let tag = mg_module::instances::set_tag(name, &tags);
        tags.push(tag.clone());
        edits.push(mg_edit::Edit::SetField {
            key: view.git(),
            path: mg_edit::GffPath::root().item(kind.list(), index),
            label: "Tag".into(),
            value: Some(Value::String(tag.into_bytes())),
        });
    }
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Create set", edits)));
    }
}

/// The Create Set window.
fn set_window(app: &mut Moonglow, ui: &egui::Ui, view: &mut AreaView) {
    let Some(mut name) = view.set_name.take() else { return };
    let mut open = true;
    let mut done = false;
    let mut cancel = false;
    egui::Window::new("Create Set").collapsible(false).resizable(false).open(&mut open).show(
        ui.ctx(),
        |ui| {
            ui.horizontal(|ui| {
                ui.label("What is the name of the set?");
                let field = ui.add(egui::TextEdit::singleline(&mut name).hint_text("set name"));
                crate::widgets::autofocus(ui, &field);
            });
            ui.horizontal(|ui| {
                done = ui.add_enabled(!name.trim().is_empty(), egui::Button::new("OK")).clicked()
                    || (!name.trim().is_empty() && crate::widgets::enter(ui));
                cancel = ui.button("Cancel").clicked();
            });
        },
    );
    if done {
        create_set(app, view, name.trim());
    } else if open && !cancel {
        view.set_name = Some(name);
    }
}

/// Aurora's Add to Palette: the object made into a new custom blueprint
/// (with the items it holds), the object naming it; its editor opens.
fn add_to_palette(app: &mut Moonglow, view: &AreaView, kind: ObjectKind, index: usize) {
    let git = view.git();
    let (Some(game), Some(ws)) = (app.game.as_deref(), app.ws.as_mut()) else { return };
    let Some(placed) = ws
        .doc(&git)
        .ok()
        .and_then(|g| g.root.list(kind.list()).and_then(|l| l.get(index)).cloned())
    else {
        return;
    };
    let module = &ws.module;
    let original = |k: ResKey| -> Option<mg_gff::Struct> {
        let data = module
            .get(&k)
            .map(<[u8]>::to_vec)
            .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let taken = |k: &ResKey| module.contains(k) || game.resman.get(k).is_ok();
    let Some(added) =
        mg_module::palette_add::add_to_palette(game, kind.restype(), &placed, &original, &taken)
    else {
        app.log.error("Add to Palette: no blueprint name is free");
        return;
    };
    let mut edits = Vec::new();
    for (key, gff) in &added.blueprints {
        match gff.to_bytes() {
            Ok(data) => edits.push(mg_edit::Edit::SetResource { key: *key, data: Some(data) }),
            Err(e) => {
                app.log.error(format!("{key}: {e}"));
                return;
            }
        }
    }
    let path = mg_edit::GffPath::root();
    edits.push(mg_edit::Edit::RemoveItem {
        key: git,
        path: path.clone(),
        list: kind.list().into(),
        index,
    });
    edits.push(mg_edit::Edit::InsertItem {
        key: git,
        path,
        list: kind.list().into(),
        index,
        item: added.instance,
    });
    let first = added.blueprints[0].0;
    let held = added.blueprints.len() - 1;
    app.actions.push(Action::Apply(Command::new("Add to palette", edits)));
    app.actions.push(Action::OpenTab(crate::Tab::Blueprint(first)));
    app.log.info(if held > 0 {
        format!("Added {first} to the palette, and the {held} items it holds")
    } else {
        format!("Added {first} to the palette")
    });
}

/// Area Statistics (Aurora's Resources Used): what the area's loaded
/// models use.
pub(crate) fn stats_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(area) = app.area_stats else { return };
    let Some(view) = app.area_views.get(&area) else {
        app.area_stats = None;
        return;
    };
    let mut open = true;
    let mut done = false;
    egui::Window::new("Resources Used").open(&mut open).collapsible(false).resizable(false).show(
        ctx,
        |ui| {
            ui.strong(area.to_string());
            egui::Grid::new("usage").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
                if let Some(m) = &view.model {
                    ui.label("Tiles");
                    ui.label(format!("{} ({} by {})", m.tiles.len(), m.width, m.height));
                    ui.end_row();
                    crate::widgets::field_label(ui, "Objects");
                    ui.label(m.objects.len().to_string());
                    ui.end_row();
                }
                if let Some(u) = view.scene.as_ref().map(|s| s.usage()) {
                    for (label, value) in [
                        ("Tile models", u.tile_models.to_string()),
                        ("Object models", u.object_models.to_string()),
                        ("Meshes", u.meshes.to_string()),
                        ("Triangles", u.triangles.to_string()),
                        ("Model memory", format!("{:.1} MB", u.buffer_bytes as f64 / 1048576.0)),
                        ("Textures", u.textures.to_string()),
                    ] {
                        crate::widgets::field_label(ui, label);
                        ui.label(value);
                        ui.end_row();
                    }
                }
            });
            done = ui.button("Done").clicked();
        },
    );
    if !open || done {
        app.area_stats = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_goes_up_and_down_within_its_heights() {
        let mut o =
            Orbit { target: Vec3::new(5.0, 5.0, 0.0), yaw: 0.0, pitch: 1.0, distance: 20.0 };
        raise(&mut o, 2.5);
        assert_eq!(o.target, Vec3::new(5.0, 5.0, 2.5));
        // The camera goes up with its target.
        assert!((o.camera().eye.z - (2.5 + 20.0 * 1f32.sin())).abs() < 1e-3);
        raise(&mut o, -1000.0);
        assert_eq!(o.target.z, TARGET_HEIGHTS.0);
        raise(&mut o, 1e6);
        assert_eq!(o.target.z, TARGET_HEIGHTS.1);
    }

    #[test]
    fn a_sounds_circle_is_level_around_it() {
        let centre = Vec3::new(10.0, 20.0, 1.5);
        let circle = sound_circle(centre, 7.0);
        assert!(circle.len() >= 32);
        assert!(circle.iter().all(|p| p.z == 1.5 && ((*p - centre).length() - 7.0).abs() < 1e-4));
    }

    #[test]
    fn the_fog_is_measured_from_a_game_camera_near_the_target() {
        let city = mg_render::Fog { start: -5.0, end: 45.0, color: Vec3::splat(0.4) };
        // As near as the game's camera: the game's fog.
        assert_eq!(view_fog(city, 15.0), city);
        assert_eq!(view_fog(city, GAME_CAMERA), city);
        // From 60 m: moved back 40 m, so the target (60 m off) is as clear
        // as the game shows a player 20 m from its camera.
        let far = view_fog(city, 60.0);
        assert_eq!((far.start, far.end, far.color), (35.0, 85.0, city.color));
    }
}
