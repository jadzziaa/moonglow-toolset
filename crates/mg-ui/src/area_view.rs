//! The area viewer: an area's tiles and objects drawn with its lighting,
//! a camera that orbits, pans and zooms, and objects to select, move, turn
//! and delete, each change one undoable command on the area's GIT.
//!
//! Mouse: a left click selects (Ctrl adds or removes), a left drag moves
//! the selection over the ground and Shift + left drag turns it; a middle
//! drag orbits, Shift + middle drag pans and the wheel zooms. Over the
//! view, the arrow keys pan and Delete deletes the selection.

use std::f32::consts::FRAC_PI_2;

use egui::{Color32, Pos2, Rect, Stroke};
use glam::{Vec2, Vec3};
use mg_area::pick::{Ray, pick, project};
use mg_area::{AreaModel, AreaScene, ObjectKind, TILE_SIZE, View};
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_gff::Gff;
use mg_render::{Camera, Targets};
use mg_resman::ResKey;
use mg_schema::{StructExt, ifo};
use mg_set::Tileset;

use crate::{Action, Moonglow};

/// Multisampling for the area view.
const SAMPLES: u32 = 4;

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
    /// Looking north at the area's centre from where all of it shows.
    pub fn overview(area: &AreaModel) -> Orbit {
        let (w, h) = area.size();
        let span = w.max(h).max(TILE_SIZE);
        Orbit {
            target: Vec3::new(w / 2.0, h / 2.0, 0.0),
            yaw: -FRAC_PI_2,
            pitch: 55f32.to_radians(),
            distance: span * 1.2,
        }
    }

    pub fn camera(&self) -> Camera {
        let mut c = Camera::orbit(self.target, self.distance, self.yaw, self.pitch);
        c.far = c.far.max(self.distance * 4.0 + 400.0);
        c
    }
}

/// What a left drag does.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    /// Moves the selection by the ground point's travel since `from`, on
    /// the plane at height `z`.
    Move { from: Vec3, z: f32, offset: Vec3 },
    /// Turns each selected object about itself.
    Turn { angle: f32 },
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
            error: None,
            orbit: None,
            selection: Vec::new(),
            night: false,
            fog: true,
            grid: true,
            show: [true; 9],
            show_start: true,
            drag: None,
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

    /// Where each selected object stands and turns with the drag applied.
    fn dragged(&self) -> Vec<(usize, Vec3, f32)> {
        let Some(model) = &self.model else { return Vec::new() };
        let Some(drag) = self.drag else { return Vec::new() };
        self.selection
            .iter()
            .filter_map(|&(k, i)| self.object_at(k, i))
            .map(|i| {
                let o = &model.objects[i];
                match drag {
                    Drag::Move { offset, .. } => (i, o.position + offset, o.rotation),
                    Drag::Turn { angle } => {
                        let turns = !o.kind.has_outline() && o.kind != ObjectKind::Sound;
                        (i, o.position, if turns { o.rotation + angle } else { o.rotation })
                    }
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
        view.tileset = Some((tileset_ref, mg_area::tileset(game, tileset_ref).ok()));
    }
    let tileset = view.tileset.as_ref().and_then(|(_, t)| t.as_ref());
    let model = AreaModel::read(game, &are.root, &git.root, tileset);
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
    if let ([(kind, index)], Some(m)) = (view.selection.as_slice(), &view.model)
        && let Some(o) = m.object(*kind, *index)
    {
        ui.weak(format!(
            "{:?} {} ({}) at {:.2}, {:.2}, {:.2}, facing {:.0}°",
            kind,
            o.tag,
            o.template.map(|t| t.to_string()).unwrap_or_default(),
            o.position.x,
            o.position.y,
            o.position.z,
            o.facing().to_degrees().rem_euclid(360.0),
        ));
    } else if view.selection.len() > 1 {
        ui.weak(format!("{} objects selected", view.selection.len()));
    } else {
        // Always a line, so that the view does not move when selecting.
        ui.weak(
            "Click to select (Ctrl: add), drag to move, Shift + drag to turn; middle drag \
             orbits (Shift: pans), the wheel zooms.",
        );
    }
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
    let frame = scene.scene(&shown, &settings);
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
    overlays(ui, view, &shown, start);
    // Tiles animate: keep drawing while the view is on screen.
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    input(app, ui, view, &response);
}

/// Markers for objects without models, outlines, the grid, the start
/// location and the selection, drawn over the view.
fn overlays(ui: &egui::Ui, view: &AreaView, shown: &AreaModel, start: Option<(Vec3, f32)>) {
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
            let color = match o.kind {
                ObjectKind::Encounter => Color32::from_rgb(230, 120, 40),
                _ => Color32::from_rgb(80, 200, 120),
            };
            let stroke = if selected { highlight } else { Stroke::new(1.5, color) };
            let n = o.outline.len();
            for k in 0..n {
                line(o.outline[k], o.outline[(k + 1) % n], stroke);
            }
            continue;
        }
        let transform = o.transform();
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
    let (shift, command) = ui.input(|i| (i.modifiers.shift, i.modifiers.command));
    // Camera: middle drag orbits (Shift: pans), the wheel zooms.
    if response.dragged_by(egui::PointerButton::Middle)
        && let Some(o) = &mut view.orbit
    {
        let d = response.drag_delta();
        if shift {
            pan(o, Vec2::new(-d.x, d.y) * o.distance * 0.0015);
        } else {
            o.yaw -= d.x * 0.01;
            o.pitch = (o.pitch + d.y * 0.01).clamp(0.05, 1.55);
        }
    }
    if response.hovered()
        && let Some(o) = &mut view.orbit
    {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            o.distance = (o.distance * (-scroll * 0.002).exp()).clamp(1.0, 2000.0);
        }
        let step = o.distance * 0.02;
        let keys = ui.input(|i| {
            [egui::Key::ArrowLeft, egui::Key::ArrowRight, egui::Key::ArrowUp, egui::Key::ArrowDown]
                .map(|k| i.key_down(k))
        });
        let dir = Vec2::new(
            f32::from(u8::from(keys[1])) - f32::from(u8::from(keys[0])),
            f32::from(u8::from(keys[2])) - f32::from(u8::from(keys[3])),
        );
        if dir != Vec2::ZERO {
            pan(o, dir * step);
            ui.ctx().request_repaint();
        }
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
    response.context_menu(|ui| {
        if ui.add_enabled(!view.selection.is_empty(), egui::Button::new("Delete")).clicked() {
            delete(app, view);
            ui.close();
        }
    });

    // Moving and turning.
    // A drag starts once the pointer has moved: pick where it was pressed.
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = ui.input(|i| i.pointer.press_origin())
        && let Some(i) = view.pick(pos)
        && let Some(o) = view.model.as_ref().map(|m| m.objects[i].clone())
    {
        if !view.selected(i) {
            view.selection = vec![(o.kind, o.index)];
        }
        view.drag = if shift {
            Some(Drag::Turn { angle: 0.0 })
        } else {
            view.ray(pos).and_then(|r| r.at_height(o.position.z)).map(|from| Drag::Move {
                from,
                z: o.position.z,
                offset: Vec3::ZERO,
            })
        };
    }
    if response.dragged_by(egui::PointerButton::Primary) {
        let pointer = response.interact_pointer_pos();
        let ground = |z: f32| pointer.and_then(|p| view.ray(p)).and_then(|r| r.at_height(z));
        match view.drag {
            Some(Drag::Move { from, z, .. }) => {
                if let Some(now) = ground(z) {
                    view.drag = Some(Drag::Move { from, z, offset: now - from });
                }
            }
            Some(Drag::Turn { angle }) => {
                view.drag = Some(Drag::Turn { angle: angle - response.drag_delta().x * 0.01 });
            }
            None => {}
        }
    }
    if response.drag_stopped_by(egui::PointerButton::Primary) {
        let moved = view.dragged();
        let label = match view.drag {
            Some(Drag::Turn { .. }) => "Rotate",
            _ => "Move",
        };
        view.drag = None;
        commit_moves(app, view, &moved, label);
    }
    if response.hovered()
        && !view.selection.is_empty()
        && ui.input(|i| i.key_pressed(egui::Key::Delete))
    {
        delete(app, view);
    }
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

fn delete(app: &mut Moonglow, view: &mut AreaView) {
    let edits = mg_area::edit::delete_edits(view.git(), &view.selection);
    view.selection.clear();
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Delete", edits)));
    }
}
