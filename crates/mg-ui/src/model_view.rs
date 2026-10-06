//! The model viewer: any model from the resources, or what a creature, item,
//! placeable or door blueprint looks like (`mg_preview`), drawn with the
//! game's lighting, turned by dragging (left: orbit, right or middle: pan,
//! wheel: zoom), with its animations (and its supermodels') to play.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_core::ResRef;
use mg_core::ResType;
use mg_edit::{GffPath, Workspace};
use mg_gff::Gff;
use mg_mdl::Model;
use mg_preview::Preview;
use mg_preview::compose::Composed;
use mg_render::anim;
use mg_render::dangly::Dangly;
use mg_render::particles::Particles;
use mg_render::{AreaLight, Camera, Gpu, GpuModel, Instance, MeshState, Renderer, Scene, Targets};
use mg_resman::ResKey;

use crate::Moonglow;

/// Multisampling for 3D views.
const SAMPLES: u32 = 4;

/// The GPU and renderer the 3D views share, from the window's wgpu state.
pub struct Viewport3d {
    pub render_state: egui_wgpu::RenderState,
    pub gpu: Gpu,
    pub renderer: Renderer,
}

impl std::fmt::Debug for Viewport3d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Viewport3d").finish_non_exhaustive()
    }
}

impl Viewport3d {
    pub fn new(render_state: egui_wgpu::RenderState) -> Viewport3d {
        let gpu = Gpu::new(render_state.device.clone(), render_state.queue.clone());
        let renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, SAMPLES);
        Viewport3d { render_state, gpu, renderer }
    }
}

/// An open model's view.
#[derive(Debug)]
pub struct ModelView {
    model: Result<Arc<GpuModel>, String>,
    /// Supermodels loaded so far, by name.
    supermodels: RefCell<HashMap<String, Option<Arc<Model>>>>,
    pub animation: Option<String>,
    pub playing: bool,
    /// Light the model with its own light nodes.
    pub model_lights: bool,
    pub time: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: Vec3,
    targets: Option<(Targets, egui::TextureId)>,
    last_frame: Option<f64>,
    particles: Option<Particles>,
    dangly: Option<Dangly>,
    /// Chunk emitters' models, by name.
    chunk_models: HashMap<String, Option<Arc<GpuModel>>>,
    /// The base model with its attached parts (a blueprint's body parts,
    /// equipment, wings and tails).
    composed: Option<Composed>,
    /// What the blueprint looked like when built, and the workspace revision
    /// it was read at: an edit in its editor rebuilds the view.
    preview: Option<Preview>,
    revision: Option<u64>,
}

/// What the viewer shows: a model or blueprint (from the module or the
/// game), or an object placed in an area (its entry in the area's GIT).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    Resource(ResKey),
    Instance { area: ResRef, path: GffPath },
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Resource(k) => write!(f, "{k}"),
            Source::Instance { area, .. } => write!(f, "an object placed in {area}"),
        }
    }
}

/// What a source looks like: a model, a blueprint's preview, or a placed
/// object's (its GIT entry has the blueprint's fields, as edited there).
fn preview_of(app: &Moonglow, source: &Source) -> Result<Preview, String> {
    let game = app.game.as_deref().ok_or("No game data.")?;
    let data = |k: &ResKey| {
        app.ws
            .as_ref()
            .and_then(|w| w.module.get(k).map(<[u8]>::to_vec))
            .or_else(|| game.resman.get(k).ok().map(|d| d.into_owned()))
    };
    let read = |k: &ResKey| data(k).and_then(|d| Gff::read(&d).ok());
    let (restype, object) = match source {
        Source::Resource(key) if key.restype == ResType::MDL => {
            return Ok(Preview::model(&key.resref.to_string()));
        }
        Source::Resource(key) => {
            let gff = read(key).ok_or_else(|| format!("{key}: not found or not readable"))?;
            (key.restype, gff.root)
        }
        Source::Instance { area, path } => {
            let git = read(&ResKey::new(*area, ResType::GIT))
                .ok_or_else(|| format!("{area}.git: not found or not readable"))?;
            // (A placed object, or an item one holds.)
            let restype = crate::blueprint::instance_type(path)
                .ok_or_else(|| format!("{source}: not an object's entry"))?;
            let object = path.get(&git.root).cloned().ok_or_else(|| format!("{source}: gone"))?;
            (restype, object)
        }
    };
    let items = |r: ResRef| read(&ResKey::new(r, ResType::UTI));
    let preview = match restype {
        ResType::UTC => mg_preview::creature(game, &object, &items),
        ResType::UTI => mg_preview::item_on(game, &object, app.armor_on_woman),
        ResType::UTP => mg_preview::placeable(game, &object),
        ResType::UTD => mg_preview::door(game, &object),
        // The flag the area view draws. (A merchant's and a sound's markers
        // are the same for every one: no picture to tell them by.)
        ResType::UTW => mg_preview::waypoint(game, &object),
        t => return Err(format!("{source}: no preview for {t:?}")),
    };
    preview.map(|p| mg_preview::replaced(p, &object)).map_err(|e| format!("{source}: {e}"))
}

/// Has every open viewer read its object again (what it is shown on has
/// changed: an armor's wearer).
pub(crate) fn read_again(app: &mut Moonglow) {
    for view in app.model_views.values_mut() {
        view.revision = Some(u64::MAX);
    }
}

/// Whether the viewer can show a resource type.
pub(crate) fn previewable(t: ResType) -> bool {
    matches!(t, ResType::MDL | ResType::UTC | ResType::UTI | ResType::UTP | ResType::UTD)
}

/// A preview's models on the GPU, from the module first, then the game.
fn compose(app: &Moonglow, source: &Source, preview: &Preview) -> Result<Composed, String> {
    let vp = app.viewport.as_ref().ok_or("No GPU: the model viewer needs one.")?;
    let game = app.game.as_deref().ok_or("No game data.")?;
    let load = |name: &str| -> Option<Arc<Model>> {
        let k = ResKey::parse(name, ResType::MDL)?;
        let data = app
            .ws
            .as_ref()
            .and_then(|w| w.module.get(&k).map(<[u8]>::to_vec))
            .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
        Model::read(&data).ok().map(Arc::new)
    };
    Composed::new(&vp.gpu, preview, &load)
        .ok_or_else(|| format!("{source}: model {} not found or not readable", preview.base.model))
}

impl ModelView {
    /// Builds it again if the blueprint now looks different (its editor
    /// changed it), keeping the camera, and the animation where the new
    /// model has it.
    fn refresh(&mut self, app: &mut Moonglow, source: &Source) {
        let revision = app.ws.as_ref().map(Workspace::revision);
        let model = matches!(source, Source::Resource(k) if k.restype == ResType::MDL);
        if model || revision == self.revision {
            return;
        }
        self.revision = revision;
        // The editors' working copies, which the preview reads.
        if let Some(ws) = app.ws.as_mut() {
            let _ = ws.flush();
        }
        let Ok(preview) = preview_of(app, source) else { return };
        if self.preview.as_ref() == Some(&preview) {
            return;
        }
        let composed = compose(app, source, &preview);
        self.model = composed.as_ref().map(|c| c.base().clone()).map_err(Clone::clone);
        let composed = composed.ok();
        let animations = composed.as_ref().map(Composed::animations).unwrap_or_default();
        if self.animation.as_ref().is_some_and(|a| !animations.contains(a)) {
            self.animation = composed.as_ref().and_then(|c| c.idle.clone());
        }
        self.composed = composed;
        self.preview = Some(preview);
        self.supermodels = RefCell::new(HashMap::new());
        (self.particles, self.dangly) = (None, None);
        self.chunk_models.clear();
    }

    fn open(app: &Moonglow, source: &Source) -> ModelView {
        let preview = preview_of(app, source);
        let composed = preview.clone().and_then(|p| compose(app, source, &p));
        let model = composed.as_ref().map(|c| c.base().clone()).map_err(Clone::clone);
        let animation = composed
            .as_ref()
            .ok()
            .and_then(|c| c.idle.clone().filter(|i| c.animations().contains(i)));
        let composed = composed.ok();
        let mut view = ModelView {
            model,
            supermodels: RefCell::new(HashMap::new()),
            animation,
            playing: true,
            model_lights: true,
            time: 0.0,
            // Models face +Y: from the front, a little to the side.
            yaw: 60f32.to_radians(),
            pitch: 20f32.to_radians(),
            distance: 5.0,
            target: Vec3::ZERO,
            targets: None,
            last_frame: None,
            particles: None,
            dangly: None,
            chunk_models: HashMap::new(),
            composed,
            preview: preview.ok(),
            revision: app.ws.as_ref().map(Workspace::revision),
        };
        view.frame();
        view
    }

    /// What it shows of a blueprint (none for a model).
    pub fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

    /// Fits the camera to the model's rest pose.
    pub fn frame(&mut self) {
        if let Some(c) = &self.composed {
            let (min, max) = c.bounds();
            self.target = (min + max) * 0.5;
            let radius = ((max - min).length() * 0.5).max(0.1);
            self.distance = radius / 20f32.to_radians().sin() * 1.1;
            return;
        }
        let Ok(m) = &self.model else { return };
        let pose = mg_render::rest_pose(&m.model);
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for mesh in &m.meshes {
            for c in 0..8 {
                let p = Vec3::new(
                    if c & 1 == 0 { mesh.min.x } else { mesh.max.x },
                    if c & 2 == 0 { mesh.min.y } else { mesh.max.y },
                    if c & 4 == 0 { mesh.min.z } else { mesh.max.z },
                );
                let w = pose[mesh.node].transform_point3(p);
                min = min.min(w);
                max = max.max(w);
            }
        }
        if min.x > max.x {
            (min, max) = (Vec3::splat(-1.0), Vec3::splat(1.0));
        }
        self.target = (min + max) * 0.5;
        let radius = ((max - min).length() * 0.5).max(0.1);
        self.distance = radius / 20f32.to_radians().sin() * 1.1;
    }

    fn camera(&self) -> Camera {
        Camera::orbit(self.target, self.distance, self.yaw, self.pitch)
    }
}

fn load_super(
    app: &Moonglow,
    cache: &RefCell<HashMap<String, Option<Arc<Model>>>>,
    name: &str,
) -> Option<Arc<Model>> {
    let name = name.to_ascii_lowercase();
    if let Some(m) = cache.borrow().get(&name) {
        return m.clone();
    }
    let m = app
        .game
        .as_ref()
        .and_then(|g| g.resman.get_named(&name, ResType::MDL).ok())
        .and_then(|d| Model::read(&d).ok())
        .map(Arc::new);
    cache.borrow_mut().insert(name, m.clone());
    m
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui, source: Source) {
    show(app, ui, source, false);
}

/// The viewer in an editor's page (`embedded`): its toolbar without the
/// model's statistics (the page is narrower than a window), and with Pop
/// Out among its buttons. Whether Pop Out was clicked.
pub(crate) fn embedded(app: &mut Moonglow, ui: &mut egui::Ui, source: Source) -> bool {
    show(app, ui, source, true)
}

fn show(app: &mut Moonglow, ui: &mut egui::Ui, source: Source, embedded: bool) -> bool {
    let key = source.clone();
    if !app.model_views.contains_key(&key) {
        // The editors' working copies, which the preview reads.
        if let Some(ws) = app.ws.as_mut() {
            let _ = ws.flush();
        }
        let view = ModelView::open(app, &source);
        app.model_views.insert(key.clone(), view);
    }
    let mut view = app.model_views.remove(&key).expect("just inserted");
    view.refresh(app, &source);
    let model = match &view.model {
        Ok(m) => m.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
            app.model_views.insert(key, view);
            return false;
        }
    };

    // Animations: the model's own, then its supermodels'.
    let anims = anim::animations(&model.model, &|n| load_super(app, &view.supermodels, n));
    // Wrapping in a narrow window, so none of it is cut off.
    let mut pop_out = false;
    ui.horizontal_wrapped(|ui| {
        if !embedded {
            ui.label(format!(
                "{}: {} nodes, {} meshes",
                model.model.name,
                model.model.nodes.len(),
                model.meshes.len()
            ));
        }
        if let Some(c) = view.composed.as_ref().filter(|c| c.part_count() > 0 && !embedded) {
            ui.weak(format!("+ {} parts", c.part_count()))
                .on_hover_text("Body parts, equipment, wings and tails from the blueprint");
            if !c.missing.is_empty() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!("{} missing", c.missing.len()),
                )
                .on_hover_text(c.missing.join("\n"));
            }
        }
        if let Some(s) = model.model.supermodel.as_ref().filter(|_| !embedded) {
            ui.weak(format!("supermodel {s}"));
        }
        if !embedded {
            ui.separator();
        }
        let current = view.animation.clone().unwrap_or_else(|| "(rest pose)".into());
        egui::ComboBox::new(("anim", &key), "Animation").selected_text(current).show_ui(ui, |ui| {
            if ui.selectable_label(view.animation.is_none(), "(rest pose)").clicked() {
                view.animation = None;
            }
            for (name, _) in &anims {
                if ui.selectable_label(view.animation.as_deref() == Some(name), name).clicked() {
                    view.animation = Some(name.clone());
                    view.time = 0.0;
                }
            }
        });
        if ui.button(if view.playing { "Pause" } else { "Play" }).clicked() {
            view.playing = !view.playing;
        }
        if ui.button("Frame").on_hover_text("Fit the model in the view").clicked() {
            view.frame();
        }
        ui.checkbox(&mut view.model_lights, "Lights")
            .on_hover_text("Light the model with its own light nodes");
        if embedded {
            pop_out =
                ui.button("Pop Out").on_hover_text("Show it in a window of its own").clicked();
        }
    });

    let Some(vp) = app.viewport.as_mut() else {
        ui.label("No GPU.");
        app.model_views.insert(key, view);
        return false;
    };
    let size = ui.available_size().max(egui::vec2(32.0, 32.0));
    let ppp = ui.ctx().pixels_per_point();
    let (w, h) = ((size.x * ppp).round() as u32, (size.y * ppp).round() as u32);
    let (w, h) = (w.clamp(16, 8192), h.clamp(16, 8192));

    // Animation time.
    let now = ui.input(|i| i.time);
    let dt = match (view.last_frame, view.playing) {
        (Some(last), true) => (now - last) as f32,
        _ => 0.0,
    };
    view.time += dt;
    view.last_frame = Some(now);
    let playing: Option<&mg_mdl::Animation> = view.animation.as_ref().and_then(|name| {
        let (_, owner) = anims.iter().find(|(n, _)| n == name)?;
        owner.animation(name)
    });
    let pose = playing.map(|a| Arc::new(anim::pose(&model.model, a, view.time)));
    let lights = if view.model_lights {
        let p = pose.as_deref().unwrap_or(&model.rest);
        anim::lights(&model.model, playing, view.time, p, Mat4::IDENTITY, &|_| None)
    } else {
        Vec::new()
    };
    // Particles run whenever the view plays.
    let camera = view.camera();
    let particles = view.particles.get_or_insert_with(|| Particles::new(&model.model));
    let current_pose: &[Mat4] = pose.as_deref().map_or(&model.rest, Vec::as_slice);
    particles.update(&model.model, playing, view.time, dt, current_pose, Mat4::IDENTITY);
    let batches = particles.batches(
        &model.model,
        playing,
        view.time,
        current_pose,
        Mat4::IDENTITY,
        camera.view(),
    );
    // Animated alpha, self-illumination and vertices; dangly meshes.
    let mut state =
        playing.map_or_else(|| MeshState::new(&model), |a| anim::mesh_state(&model, a, view.time));
    let dangly = view.dangly.get_or_insert_with(|| Dangly::new(&model));
    dangly.update(dt, current_pose, Mat4::IDENTITY, Vec3::ZERO);
    dangly.apply(&mut state, current_pose, Mat4::IDENTITY);
    // Chunk emitters' models.
    let chunks = particles.chunks(&model.model, playing, view.time, current_pose, Mat4::IDENTITY);
    let live = particles.live();
    let mut chunk_instances = Vec::new();
    for c in chunks {
        let gm = view
            .chunk_models
            .entry(c.model.clone())
            .or_insert_with(|| {
                let m = app
                    .game
                    .as_ref()
                    .and_then(|g| g.resman.get_named(&c.model, ResType::MDL).ok())
                    .and_then(|d| Model::read(&d).ok())?;
                // (One that breaks what builds it is left out.)
                let name = c.model.clone();
                let gpu = &vp.gpu;
                mg_render::guard::guarded(&name, || Arc::new(GpuModel::new(gpu, Arc::new(m))))
            })
            .clone();
        if let Some(gm) = gm {
            chunk_instances.push(Instance::new(gm, c.transform));
        }
    }
    // Keep drawing while something moves: an animation, live particles or
    // dangly meshes.
    if (pose.is_some() || live || dangly.moving()) && view.playing {
        ui.ctx().request_repaint();
    }

    // Render into our own texture, shown as an image.
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
    // The base (with the viewer's animated meshes and dangly state), then
    // what hangs from it.
    let mut instances = match &view.composed {
        Some(c) => c.instances(view.animation.as_deref(), view.time, Mat4::IDENTITY),
        None => vec![Instance::new(model.clone(), Mat4::IDENTITY)],
    };
    if let Some(base) = instances.first_mut() {
        base.pose = pose;
        base.state = Some(Arc::new(state));
    }
    instances.extend(chunk_instances);
    let mut lights = lights;
    if let Some(c) = &view.composed {
        lights.extend(c.point_lights(Mat4::IDENTITY));
    }
    let scene = Scene {
        instances,
        lights,
        particles: batches,
        area: AreaLight::default(),
        background: [0.16, 0.18, 0.21],
        ..Default::default()
    };
    let resman = app.game.as_deref().map(|g| &g.resman);
    let assets: &dyn mg_render::Assets = match resman {
        Some(rm) => rm,
        None => &mg_render::NoAssets,
    };
    vp.renderer.render(
        &vp.gpu,
        assets,
        &scene,
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

    // Orbit, pan, zoom.
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        view.yaw -= d.x * 0.01;
        view.pitch = (view.pitch + d.y * 0.01).clamp(-1.45, 1.45);
    }
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let d = response.drag_delta();
        let cam = view.camera();
        let fwd = (cam.target - cam.eye).normalize();
        let right = fwd.cross(Vec3::Z).normalize_or_zero();
        let up = right.cross(fwd);
        let scale = view.distance * 0.0015;
        view.target += (-right * d.x + up * d.y) * scale;
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            view.distance = (view.distance * (-scroll * 0.002).exp()).clamp(0.05, 5000.0);
        }
    }
    app.model_views.insert(key, view);
    pop_out
}

/// Pictures of blueprints as they look (the palette's hover preview),
/// rendered once each and kept until the module changes.
#[derive(Default)]
pub struct Thumbnails {
    revision: Option<u64>,
    made: HashMap<Pictured, Option<(Targets, egui::TextureId)>>,
}

/// What a thumbnail is of: a blueprint or a model, or a creature's look
/// (an appearance with a plain body, for choosing among appearances).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Pictured {
    Resource(ResKey),
    Look(mg_preview::CreatureLook),
}

impl Thumbnails {
    /// The thumbnails made, by blueprint.
    pub(crate) fn all(&self) -> HashMap<ResKey, Option<egui::TextureId>> {
        self.made
            .iter()
            .filter_map(|(k, made)| match k {
                Pictured::Resource(k) => Some((*k, made.as_ref().map(|(_, id)| *id))),
                Pictured::Look(_) => None,
            })
            .collect()
    }

    /// Every thumbnail made.
    pub(crate) fn every(&self) -> HashMap<Pictured, Option<egui::TextureId>> {
        self.made.iter().map(|(k, made)| (*k, made.as_ref().map(|(_, id)| *id))).collect()
    }

    /// The thumbnails made of creatures' looks.
    pub(crate) fn looks(&self) -> HashMap<mg_preview::CreatureLook, Option<egui::TextureId>> {
        self.made
            .iter()
            .filter_map(|(k, made)| match k {
                Pictured::Look(l) => Some((*l, made.as_ref().map(|(_, id)| *id))),
                Pictured::Resource(_) => None,
            })
            .collect()
    }
}

impl std::fmt::Debug for Thumbnails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Thumbnails").field("made", &self.made.len()).finish()
    }
}

/// How large thumbnails are drawn, in pixels.
pub(crate) const THUMBNAIL: u32 = 224;

/// How long a model that emits has run when its thumbnail is drawn: this
/// many steps of this many seconds.
const THUMBNAIL_STEPS: usize = 40;
const THUMBNAIL_STEP: f32 = 0.1;

/// At most this many kept; past it, the oldest are let go.
const THUMBNAILS_KEPT: usize = 192;

/// A blueprint's thumbnail (`None`: no GPU, or nothing to draw).
pub(crate) fn thumbnail(app: &mut Moonglow, key: ResKey) -> Option<egui::TextureId> {
    thumbnail_of(app, Pictured::Resource(key))
}

/// A creature look's thumbnail.
pub(crate) fn look_thumbnail(
    app: &mut Moonglow,
    look: mg_preview::CreatureLook,
) -> Option<egui::TextureId> {
    thumbnail_of(app, Pictured::Look(look))
}

pub(crate) fn thumbnail_of(app: &mut Moonglow, key: Pictured) -> Option<egui::TextureId> {
    let revision = app.ws.as_ref().map(Workspace::revision);
    if app.thumbnails.revision != revision || app.thumbnails.made.len() > THUMBNAILS_KEPT {
        let old = std::mem::take(&mut app.thumbnails.made);
        if let Some(vp) = &app.viewport {
            let mut r = vp.render_state.renderer.write();
            for (_, id) in old.into_values().flatten() {
                r.free_texture(&id);
            }
        }
        app.thumbnails.revision = revision;
    }
    if let Some(made) = app.thumbnails.made.get(&key) {
        return made.as_ref().map(|(_, id)| *id);
    }
    let made = render_thumbnail(app, key);
    let id = made.as_ref().map(|(_, id)| *id);
    app.thumbnails.made.insert(key, made);
    id
}

fn render_thumbnail(app: &mut Moonglow, key: Pictured) -> Option<(Targets, egui::TextureId)> {
    let (source, preview) = match key {
        Pictured::Resource(key) => {
            // (A blueprint's, or a model's own: a gallery of appearances.)
            if !matches!(
                key.restype,
                ResType::UTC
                    | ResType::UTD
                    | ResType::UTI
                    | ResType::UTP
                    | ResType::UTW
                    | ResType::MDL
            ) {
                return None;
            }
            if let Some(ws) = app.ws.as_mut() {
                let _ = ws.flush();
            }
            let source = Source::Resource(key);
            let preview = preview_of(app, &source).ok()?;
            (source, preview)
        }
        Pictured::Look(look) => {
            let preview = mg_preview::creature_look(app.game.as_deref()?, &look).ok()?;
            // (Named, for a message, by the table its row is of.)
            (Source::Resource(ResKey::parse("appearance", ResType::TWODA)?), preview)
        }
    };
    let composed = compose(app, &source, &preview).ok()?;
    let vp = app.viewport.as_mut()?;
    // As the viewer frames it: from the front, a little to the side.
    // A model that emits (flames, sparks, a shaft of light: some are
    // nothing else) is drawn a few seconds in, its particles in the frame.
    let idle = composed.idle.as_deref();
    let (mut min, mut max) = composed.bounds_in(idle, 0.0);
    let pitch = 20f32.to_radians();
    // From the side that shows the most of it: a thing made for a wall has
    // a face and no back, and from behind there is nothing to see.
    let towards =
        |yaw: f32| Vec3::new(yaw.cos() * pitch.cos(), yaw.sin() * pitch.cos(), pitch.sin());
    let usual = 60f32.to_radians();
    let yaw = [usual + std::f32::consts::PI, usual + 1.57, usual - 1.57]
        .into_iter()
        .fold((usual, composed.facing(towards(usual))), |best, yaw| {
            // (Only for a side that shows half as much again.)
            let shows = composed.facing(towards(yaw));
            if shows > best.1 * 1.5 + 1e-3 { (yaw, shows) } else { best }
        })
        .0;
    let mut particles = Vec::new();
    if let Some((model, playing, pose)) = composed.emitters(idle, 0.0) {
        let mut sim = Particles::new(model);
        for step in 0..THUMBNAIL_STEPS {
            let t = step as f32 * THUMBNAIL_STEP;
            sim.update(model, playing, t, THUMBNAIL_STEP, &pose, Mat4::IDENTITY);
        }
        // (Drawn for a camera on that side; where exactly comes after.)
        let rough = Camera::orbit(Vec3::ZERO, 10.0, yaw, pitch);
        let seconds = THUMBNAIL_STEPS as f32 * THUMBNAIL_STEP;
        particles = sim.batches(model, playing, seconds, &pose, Mat4::IDENTITY, rough.view());
        let mut any = false;
        for v in particles.iter().flat_map(|b| &b.vertices) {
            let p = Vec3::from(v.pos);
            (min, max) = if any { (min.min(p), max.max(p)) } else { (p, p) };
            any = true;
        }
        if any {
            let (low, high) = composed.bounds();
            if (high - low).length() > 0.2 {
                (min, max) = (min.min(low), max.max(high));
            }
        }
    }
    let target = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(0.1);
    // (Nearer than the viewer stands: a picture is small, and what is in it
    // should fill it.)
    // … but far enough that a tall thing (a banner) keeps its top and foot.
    let tall = (max.z - min.z) * 0.5 / 20f32.to_radians().tan() * 1.08
        + (max - min).truncate().length() * 0.25;
    // (A creature stands in its animation, which its bounds at rest don't
    // quite hold: it is given the viewer's room.)
    let near = if idle.is_some() { 1.05 } else { 0.85 };
    let distance = (radius / 20f32.to_radians().sin() * near).max(tall);
    let camera = Camera::orbit(target, distance, yaw, pitch);
    let scene = Scene {
        instances: composed.instances(idle, 0.0, Mat4::IDENTITY),
        lights: composed.point_lights(Mat4::IDENTITY),
        area: AreaLight::default(),
        background: [0.16, 0.18, 0.21],
        particles,
        ..Default::default()
    };
    let size = THUMBNAIL;
    let targets = Targets::new(&vp.gpu, wgpu::TextureFormat::Rgba8Unorm, SAMPLES, size, size);
    let resman = app.game.as_deref().map(|g| &g.resman);
    let assets: &dyn mg_render::Assets = match resman {
        Some(rm) => rm,
        None => &mg_render::NoAssets,
    };
    vp.renderer.render(
        &vp.gpu,
        assets,
        &scene,
        &camera,
        targets.render_view(),
        targets.resolve_view(),
        &targets.depth,
        (size, size),
    );
    let id = vp.render_state.renderer.write().register_native_texture(
        &vp.gpu.device,
        &targets.color_view,
        wgpu::FilterMode::Linear,
    );
    Some((targets, id))
}

/// A tile's model seen from straight above, its 10 m square filling
/// `size` pixels: a minimap picture. Rendered four times larger and
/// averaged down; rows bottom first. `None` without a GPU or the model.
pub(crate) fn render_tile_from_above(
    app: &mut Moonglow,
    model: &str,
    size: u32,
) -> Option<mg_image::Rgba> {
    let key = ResKey::parse(model, ResType::MDL)?;
    let source = Source::Resource(key);
    let preview = preview_of(app, &source).ok()?;
    let composed = compose(app, &source, &preview).ok()?;
    let vp = app.viewport.as_mut()?;
    let (min, max) = composed.bounds();
    // Nearly orthographic: a 2° view from far above, its square the tile's
    // (tiles' models are centred on their origin). A hair off vertical,
    // from the south, so north is up.
    let fov = 2f32.to_radians();
    let distance = (mg_area::TILE_SIZE / 2.0) / (fov / 2.0).tan();
    let centre = Vec3::new(0.0, 0.0, (min.z + max.z) * 0.5);
    let mut camera =
        Camera::orbit(centre, distance, -std::f32::consts::FRAC_PI_2, 89.9f32.to_radians());
    camera.fov_y = fov;
    let scene = Scene {
        instances: composed.instances(None, 0.0, Mat4::IDENTITY),
        lights: composed.point_lights(Mat4::IDENTITY),
        area: AreaLight::default(),
        background: [0.0, 0.0, 0.0],
        ..Default::default()
    };
    let assets: &dyn mg_render::Assets = match app.game.as_deref() {
        Some(g) => &g.resman,
        None => &mg_render::NoAssets,
    };
    let big = size * 4;
    let image = vp.renderer.render_image(&vp.gpu, assets, &scene, &camera, big, big);
    let mut out = mg_image::Rgba::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0u32; 3];
            for dy in 0..4 {
                for dx in 0..4 {
                    let p = image.pixel(x * 4 + dx, y * 4 + dy);
                    for c in 0..3 {
                        sum[c] += u32::from(p[c]);
                    }
                }
            }
            let i = ((y * size + x) * 4) as usize;
            out.data[i..i + 4].copy_from_slice(&[
                (sum[0] / 16) as u8,
                (sum[1] / 16) as u8,
                (sum[2] / 16) as u8,
                255,
            ]);
        }
    }
    // Rendered rows come top first; pictures keep them bottom first.
    Some(out.top_down())
}
