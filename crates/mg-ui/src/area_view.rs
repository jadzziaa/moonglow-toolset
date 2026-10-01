//! The area viewer: an area's tiles and objects drawn with its lighting,
//! a camera to move around it, objects to select, move, turn, raise and
//! delete, and blueprints from the palette to place, each change one
//! undoable command on the area's GIT.
//!
//! Aurora's bindings: a click selects, a drag on the ground selects what is
//! in the box, a drag moves the selection over the ground, Shift + right
//! drag turns it and Alt + drag raises or lowers it; Ctrl + drag moves the
//! camera, Ctrl + right or middle drag turns it, the wheel zooms (Shift or
//! Ctrl: slowly), numpad 4, 6, 8, 2 move it, 7, 9, 1, 3 turn it and 5
//! looks straight down. With a blueprint chosen in the palette, a click
//! places it (Shift + click keeps it chosen; right click or Escape lets it
//! go); triggers and encounters are drawn point by point, a double click
//! closing the outline. Also: a middle drag turns the camera (Shift:
//! moves it), the arrow keys move it, Ctrl + click adds to the selection
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

/// What a drag does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    /// Moves the selection by the travel of the ground point under the
    /// pointer since `from` (where it was pressed).
    Move { from: Vec3, offset: Vec2 },
    /// Turns each selected object about itself.
    Turn { angle: f32 },
    /// Raises or lowers the selection (not creatures: they stand on the
    /// ground).
    Lift { by: f32 },
    /// Selects what is inside the box.
    Box { from: Pos2, to: Pos2 },
}

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
    /// The walkmesh is drawn over the view (Aurora's Render AABB Nodes).
    pub walkmesh: bool,
    /// surfacemat.2da's `Walk` by row, read once.
    walkable: Option<Vec<bool>>,
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
    pub grid: bool,
    /// Which kinds of object are shown, by [`ObjectKind::index`].
    pub show: [bool; 9],
    pub show_start: bool,
    drag: Option<Drag>,
    /// The outline being drawn for a trigger or encounter.
    pub outline: Vec<Vec3>,
    /// The copied objects follow the pointer, to be placed with a click.
    pub pasting: bool,
    /// Where the context menu was opened (on the ground).
    menu_at: Option<Vec3>,
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
            walkmesh: false,
            walkable: None,
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
            grid: true,
            show: [true; 9],
            show_start: true,
            drag: None,
            outline: Vec::new(),
            pasting: false,
            menu_at: None,
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
        pick(model, &ray, &|i| scene.bounds(model, i), &|k| self.show[k.index()])
    }

    /// The ground point under the pointer: on the walkmesh, else on the
    /// plane at height `z`.
    pub(crate) fn ground_at(&self, pos: Pos2, z: f32) -> Option<Vec3> {
        let ray = self.ray(pos)?;
        self.ground.as_ref().and_then(|g| g.hit(&ray)).or_else(|| ray.at_height(z))
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

    /// Where each selected object stands and turns with the drag applied.
    fn dragged(&self) -> Vec<(usize, Vec3, f32)> {
        let Some(model) = &self.model else { return Vec::new() };
        let Some(drag) = self.drag.filter(|d| !matches!(d, Drag::Box { .. })) else {
            return Vec::new();
        };
        self.selection
            .iter()
            .filter_map(|&(k, i)| self.object_at(k, i))
            .map(|i| {
                let o = &model.objects[i];
                match drag {
                    Drag::Move { offset, .. } => (i, self.moved(o, offset), o.rotation),
                    Drag::Turn { angle } => {
                        let turns = !o.kind.has_outline() && o.kind != ObjectKind::Sound;
                        (i, o.position, if turns { o.rotation + angle } else { o.rotation })
                    }
                    Drag::Lift { by } => {
                        let lifts = o.kind != ObjectKind::Creature && !o.kind.has_outline();
                        let by = if lifts { by } else { 0.0 };
                        (i, o.position + Vec3::Z * by, o.rotation)
                    }
                    Drag::Box { .. } => (i, o.position, o.rotation),
                }
            })
            .collect()
    }
}

/// Reads the area again when the workspace changed.
fn refresh(app: &mut Moonglow, view: &mut AreaView) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_ref()) else { return };
    if view.revision == Some(ws.revision()) {
        return;
    }
    view.revision = Some(ws.revision());
    let are_key = ResKey::new(view.area, ResType::ARE);
    let are = match ws.doc(&are_key) {
        Ok(g) => g.clone(),
        Err(e) => {
            view.error = Some(e.to_string());
            return;
        }
    };
    // An area without a GIT has no objects yet.
    let git = ws.doc(&view.git()).cloned().unwrap_or_else(|_| Gff::new(*b"GIT "));
    let tileset_ref = are.root.resref("Tileset").unwrap_or(ResRef::EMPTY);
    if view.tileset.as_ref().is_none_or(|(r, _)| *r != tileset_ref) {
        let set = mg_area::tileset(game, tileset_ref).ok();
        view.terrain = set
            .as_ref()
            .map(|t| crate::terrain_mode::Tools::new(tileset_ref, std::sync::Arc::new(t.clone())));
        view.tileset = Some((tileset_ref, set));
    }
    let tileset = view.tileset.as_ref().and_then(|(_, t)| t.as_ref());
    let model = AreaModel::read(game, &are.root, &git.root, tileset);
    match &mut view.ground {
        Some(g) => g.update(game, &model),
        None => view.ground = Some(Ground::new(game, &model)),
    }
    if view.orbit.is_none() {
        view.orbit = Some(Orbit::overview(&model));
        view.night = model.lighting.night_by_default();
    }
    if let Some(vp) = &app.viewport {
        match &mut view.scene {
            Some(s) => s.update(&vp.gpu, game, &model),
            None => view.scene = Some(AreaScene::new(&vp.gpu, game, &model)),
        }
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

pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui, area: ResRef) {
    let mut view = app.area_views.remove(&area).unwrap_or_else(|| AreaView::new(area));
    refresh(app, &mut view);
    app.palette.area = Some(area);
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
    let start = start_location(app, area);
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
        for kind in ObjectKind::ALL {
            ui.toggle_value(&mut view.show[kind.index()], kind.plural())
                .on_hover_text(format!("Show {}", kind.plural()));
        }
        ui.toggle_value(&mut view.show_start, "Start").on_hover_text("Show Start Location");
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
        ui.toggle_value(&mut view.night, "Night").on_hover_text("Show the area at night");
        ui.toggle_value(&mut view.fog, "Fog");
        ui.toggle_value(&mut view.grid, "Grid").on_hover_text("Display Grid");
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
        ui.toggle_value(&mut app.settings.ambient_sound, "Ambient")
            .on_hover_text("Play ambient sound in area");
        ui.toggle_value(&mut app.settings.ambient_music, "Music")
            .on_hover_text("Play ambient music in area");
        ui.toggle_value(&mut view.walkmesh, "Walkmesh")
            .on_hover_text("Render AABB Nodes: the ground's walkmesh, walkable faces green");
        if ui
            .toggle_value(&mut view.tile_mode, "Select Tiles")
            .on_hover_text("Select tiles rather than objects (Aurora's Select Terrain)")
            .changed()
        {
            view.selection.clear();
            view.tile_selection.clear();
        }
        if ui.button("Area Properties").clicked() {
            app.actions.push(Action::OpenTab(crate::Tab::AreaProperties(view.area)));
        }
        if ui.button("Reorient Camera").clicked()
            && let Some(o) = &mut view.orbit
        {
            o.yaw = -FRAC_PI_2;
        }
        if ui.button("Go to Start Location").clicked()
            && let (Some((p, _)), Some(o)) = (start_location(app, view.area), &mut view.orbit)
        {
            o.target = p;
            o.distance = o.distance.min(30.0);
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
            mg_area::terrain::Brush::Group(_) => {
                "click to place (Shift + click: place more), right click to turn"
            }
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

fn viewport(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    view: &mut AreaView,
    start: Option<(Vec3, f32)>,
) {
    let (Some(model), Some(scene), Some(orbit), Some(vp), Some(game)) =
        (&view.model, &view.scene, view.orbit, app.viewport.as_mut(), app.game.as_ref())
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
    let settings = View { time: view.time, night: view.night, fog: view.fog, show: view.show };
    let mut frame = scene.scene(&shown, &settings);
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
    let door_brush = brush(app).is_some_and(|k| k.restype == ResType::UTD);
    let (height, width) = app.settings.spawn_marker_size.unwrap_or(SPAWN_MARKER);
    let marks = Marks {
        spawn_points: (!app.settings.no_spawn_markers)
            .then_some((f32::from(height) / 10.0, f32::from(width) / 10.0)),
        door_arrows: !app.settings.no_door_arrows,
    };
    overlays(ui, view, &shown, start, app.object_clip.as_ref(), door_brush, marks);
    walkmesh_overlay(app, ui, view);
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
        let walkable = app.game.as_ref().and_then(|g| g.table("surfacemat").ok()).map(|t| {
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

/// Aurora's spawn point markers: Height 12 and Width 4 (tenths of a metre).
pub(crate) const SPAWN_MARKER: (u8, u8) = (12, 4);

/// What Options › Area adds over the view.
#[derive(Clone, Copy)]
struct Marks {
    /// A post over each encounter spawn point: its height and width (m).
    spawn_points: Option<(f32, f32)>,
    /// An arrow along each door's facing.
    door_arrows: bool,
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
    if view.grid {
        let stroke = Stroke::new(1.0, Color32::from_white_alpha(40));
        for t in &shown.tiles {
            let c = t.position;
            let h = TILE_SIZE / 2.0;
            let corners =
                [(-h, -h), (h, -h), (h, h), (-h, h)].map(|(x, y)| c + Vec3::new(x, y, 0.0));
            for k in 0..4 {
                line(corners[k], corners[(k + 1) % 4], stroke);
            }
        }
    }
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
                for &p in &o.spawn_points {
                    let (top, half) = (p + Vec3::Z * height, width / 2.0);
                    line(p, top, stroke);
                    line(top - Vec3::X * half, top + Vec3::X * half, stroke);
                    line(top - Vec3::Y * half, top + Vec3::Y * half, stroke);
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
        // Waypoints and merchants: a yellow arrow along their facing, as
        // Aurora draws them.
        if matches!(o.kind, ObjectKind::Waypoint | ObjectKind::Store) && !selected {
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
        let marker = o.preview.is_none() || scene.is_none();
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
        for ((o, _, _), p) in clip.objects.iter().zip(pasted_positions(view, clip, at)) {
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
            let c = |i: usize| {
                t.transform_point3(Vec3::new(
                    if i & 1 == 0 { min.x } else { max.x },
                    if i & 2 == 0 { min.y } else { max.y },
                    if i & 4 == 0 { min.z } else { max.z },
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
                line(c(a), c(b), stroke);
            }
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
    if let (true, Some((p, facing))) = (view.show_start, start) {
        let stroke = Stroke::new(2.0, Color32::from_rgb(230, 90, 230));
        let ahead = p + Vec3::new(facing.cos(), facing.sin(), 0.0) * 1.5;
        line(p, ahead, stroke);
        line(p, p + Vec3::Z * 2.0, stroke);
        if let Some(c) = at(p) {
            painter.circle_stroke(c, 5.0, stroke);
        }
    }
}

fn input(app: &mut Moonglow, ui: &egui::Ui, view: &mut AreaView, response: &egui::Response) {
    let (shift, command, alt) =
        ui.input(|i| (i.modifiers.shift, i.modifiers.command, i.modifiers.alt));
    camera_input(ui, view, response, shift, command);
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
                place(app, view, key, at, 0.0, &[]);
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

    // Selection.
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
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
    // pointer has moved: pick where it was pressed.
    let origin = ui.input(|i| i.pointer.press_origin());
    if response.drag_started_by(egui::PointerButton::Primary)
        && !command
        && let Some(pos) = origin
    {
        let hit = view.pick(pos);
        view.drag = if alt {
            (!view.selection.is_empty()).then_some(Drag::Lift { by: 0.0 })
        } else if let Some(i) = hit {
            let o = view.model.as_ref().map(|m| m.objects[i].clone()).expect("picked");
            if !view.selected(i) {
                view.selection = vec![(o.kind, o.index)];
            }
            view.ground_at(pos, o.position.z).map(|from| Drag::Move { from, offset: Vec2::ZERO })
        } else {
            Some(Drag::Box { from: pos, to: pos })
        };
    }
    if response.drag_started_by(egui::PointerButton::Secondary)
        && shift
        && !command
        && !view.selection.is_empty()
    {
        view.drag = Some(Drag::Turn { angle: 0.0 });
    }
    let pointer = response.interact_pointer_pos();
    match view.drag {
        Some(Drag::Move { from, .. }) if response.dragged_by(egui::PointerButton::Primary) => {
            if let Some(now) = pointer.and_then(|p| view.ground_at(p, from.z)) {
                view.drag = Some(Drag::Move { from, offset: (now - from).truncate() });
            }
        }
        Some(Drag::Lift { by }) if response.dragged_by(egui::PointerButton::Primary) => {
            let scale = view.orbit.map_or(0.02, |o| o.distance * 0.002);
            view.drag = Some(Drag::Lift { by: by - response.drag_delta().y * scale });
        }
        Some(Drag::Box { from, .. }) if response.dragged_by(egui::PointerButton::Primary) => {
            if let Some(to) = pointer {
                view.drag = Some(Drag::Box { from, to });
            }
        }
        Some(Drag::Turn { angle }) if response.dragged_by(egui::PointerButton::Secondary) => {
            view.drag = Some(Drag::Turn { angle: angle - response.drag_delta().x * 0.01 });
        }
        _ => {}
    }
    if response.drag_stopped() {
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
            Some(drag) => {
                view.drag = Some(drag);
                let moved = view.dragged();
                let label = match drag {
                    Drag::Turn { .. } => "Rotate",
                    Drag::Lift { .. } => "Raise",
                    _ => "Move",
                };
                view.drag = None;
                commit_moves(app, view, &moved, label);
            }
            None => {}
        }
    }
    if hovered && !view.selection.is_empty() && ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        delete(app, view);
    }
}

/// The camera: Ctrl + drag moves it, Ctrl + right or middle drag (or a
/// middle drag) turns it, Shift + middle drag moves it, the wheel zooms;
/// numpad and arrow keys.
fn camera_input(
    ui: &egui::Ui,
    view: &mut AreaView,
    response: &egui::Response,
    shift: bool,
    command: bool,
) {
    let rect = view.rect;
    let Some(o) = &mut view.orbit else { return };
    let d = response.drag_delta();
    let turning = (response.dragged_by(egui::PointerButton::Middle) && !shift)
        || (command && response.dragged_by(egui::PointerButton::Secondary));
    if turning {
        o.yaw -= d.x * 0.01;
        o.pitch = (o.pitch + d.y * 0.01).clamp(0.05, MAX_PITCH);
    } else if (command && response.dragged_by(egui::PointerButton::Primary))
        || (shift && response.dragged_by(egui::PointerButton::Middle))
    {
        // The ground follows the pointer.
        let per_pixel = 2.0 * o.distance * (o.camera().fov_y / 2.0).tan() / rect.height().max(1.0);
        pan(o, Vec2::new(-d.x, d.y) * per_pixel);
    }
    if !response.hovered() {
        return;
    }
    let (scroll, slow) =
        ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.shift || i.modifiers.command));
    if scroll != 0.0 {
        let rate = if slow { 0.001 } else { 0.002 };
        o.distance = (o.distance * (-scroll * rate).exp()).clamp(1.0, 2000.0);
    }
    use egui::Key;
    let down = |k: Key| ui.input(|i| i.key_down(k));
    let axis = |plus: &[Key], minus: &[Key]| {
        f32::from(u8::from(plus.iter().any(|&k| down(k))))
            - f32::from(u8::from(minus.iter().any(|&k| down(k))))
    };
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    let travel = Vec2::new(
        axis(&[Key::ArrowRight, Key::Num6], &[Key::ArrowLeft, Key::Num4]),
        axis(&[Key::ArrowUp, Key::Num8], &[Key::ArrowDown, Key::Num2]),
    );
    let turn = Vec2::new(axis(&[Key::Num9], &[Key::Num7]), axis(&[Key::Num1], &[Key::Num3]));
    if travel != Vec2::ZERO {
        pan(o, travel * o.distance * dt);
    }
    if turn != Vec2::ZERO {
        o.yaw -= turn.x * 1.5 * dt;
        o.pitch = (o.pitch + turn.y * 1.0 * dt).clamp(0.05, MAX_PITCH);
    }
    if travel != Vec2::ZERO || turn != Vec2::ZERO {
        ui.ctx().request_repaint();
    }
    if ui.input(|i| i.key_pressed(Key::Num5))
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
        .filter(|o| view.show[o.kind.index()])
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
    use mg_module::instances::{OUTLINE_LIFT, Placement, Placing, instance};
    let (Some(game), Some(ws)) = (app.game.as_ref(), app.ws.as_mut()) else { return };
    let Some(kind) = ObjectKind::from_restype(key.restype) else { return };
    let read = |k: ResKey| -> Option<mg_gff::Struct> {
        let data = ws
            .module
            .get(&k)
            .map(<[u8]>::to_vec)
            .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let Some(blueprint) = read(key) else {
        app.log.error(format!("{key}: not found or not readable"));
        return;
    };
    let items = |r: ResRef| read(ResKey::new(r, ResType::UTI));
    let placing = Placing { game, item: &items };
    let ground = |p: Vec3| view.ground.as_ref().and_then(|g| g.height(p.truncate(), p.z));
    let lift = if kind == ObjectKind::Sound { mg_area::SOUND_HEIGHT } else { 0.0 };
    let position = at + Vec3::Z * lift;
    let relative: Vec<[f32; 3]> = outline
        .iter()
        .map(|p| [p.x - at.x, p.y - at.y, ground(*p).unwrap_or(p.z) + OUTLINE_LIFT])
        .collect();
    let placement = Placement { position: position.to_array(), rotation };
    let Some(item) = instance(&placing, key.restype, &blueprint, placement, &relative) else {
        return;
    };
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
}

/// Moves the camera's target along the ground: `by.x` to the right of the
/// view, `by.y` ahead.
fn pan(o: &mut Orbit, by: Vec2) {
    let ahead = Vec3::new(-o.yaw.cos(), -o.yaw.sin(), 0.0);
    let right = Vec3::new(ahead.y, -ahead.x, 0.0);
    o.target += right * by.x + ahead * by.y;
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
                ui.add(egui::TextEdit::singleline(&mut name).hint_text("set name"));
            });
            ui.horizontal(|ui| {
                done = ui.add_enabled(!name.trim().is_empty(), egui::Button::new("OK")).clicked();
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
    let (Some(game), Some(ws)) = (app.game.as_ref(), app.ws.as_mut()) else { return };
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
                    ui.label("Objects");
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
                        ui.label(label);
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
