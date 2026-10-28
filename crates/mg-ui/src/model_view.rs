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

/// How often a model the game's shaders move by the clock (wind-swayed
/// grass, flowing water) is drawn again, seconds: as the area view's
/// animations step.
const CLOCK_STEP: f32 = 0.04;

/// The GPU and renderer the 3D views share, from the window's wgpu state.
pub struct Viewport3d {
    pub render_state: egui_wgpu::RenderState,
    pub gpu: Gpu,
    pub renderer: Renderer,
    /// The renderer has its shader cache and waker
    /// ([`crate::Moonglow::shader_cache_dir`]).
    pub(crate) configured: bool,
    /// The game data the area view's textures are read apart from, as the
    /// renderer takes it ([`mg_render::Renderer::read_textures_apart`]).
    textures_from: Option<(Arc<mg_rules::GameData>, Arc<dyn mg_render::Assets + Send + Sync>)>,
}

/// The game data as a source of textures read apart.
struct GameAssets(Arc<mg_rules::GameData>);

impl mg_render::Assets for GameAssets {
    fn texture(&self, name: &str) -> Option<mg_render::LoadedTexture> {
        mg_render::Assets::texture(&self.0.resman, name)
    }

    fn material(&self, name: &str) -> Option<mg_image::mtr::Mtr> {
        mg_render::Assets::material(&self.0.resman, name)
    }

    fn txi(&self, name: &str) -> Option<mg_image::txi::Txi> {
        mg_render::Assets::txi(&self.0.resman, name)
    }

    fn shader(&self, name: &str) -> Option<String> {
        mg_render::Assets::shader(&self.0.resman, name)
    }
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
        Viewport3d { render_state, gpu, renderer, configured: false, textures_from: None }
    }

    /// `game` as the renderer reads textures apart from it: the same while
    /// the game data is.
    pub(crate) fn textures_from(
        &mut self,
        game: &Arc<mg_rules::GameData>,
    ) -> Arc<dyn mg_render::Assets + Send + Sync> {
        match &self.textures_from {
            Some((g, from)) if Arc::ptr_eq(g, game) => from.clone(),
            _ => {
                let from: Arc<dyn mg_render::Assets + Send + Sync> =
                    Arc::new(GameAssets(game.clone()));
                self.textures_from = Some((game.clone(), from.clone()));
                from
            }
        }
    }

    /// Makes the targets a view of `w` by `h` pixels renders into, and the
    /// egui texture that shows them, when it has none or its size changed
    /// (the texture keeps its id).
    pub(crate) fn size_targets(
        &self,
        held: &mut Option<(Targets, egui::TextureId)>,
        w: u32,
        h: u32,
    ) {
        if held.as_ref().is_some_and(|(t, _)| t.size == (w, h)) {
            return;
        }
        let targets = Targets::new(&self.gpu, wgpu::TextureFormat::Rgba8Unorm, SAMPLES, w, h);
        let mut egui_renderer = self.render_state.renderer.write();
        let id = match held.take() {
            Some((_, id)) => {
                egui_renderer.update_egui_texture_from_wgpu_texture(
                    &self.gpu.device,
                    &targets.color_view,
                    wgpu::FilterMode::Linear,
                    id,
                );
                id
            }
            None => egui_renderer.register_native_texture(
                &self.gpu.device,
                &targets.color_view,
                wgpu::FilterMode::Linear,
            ),
        };
        *held = Some((targets, id));
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
    /// How what is shown is stood for the viewer: an item is modelled
    /// lying down (its length along the ground) and is shown stood on its
    /// end, as its icon shows it; all else as it is.
    stand: Mat4,
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
    /// Whether armor was shown on a woman when it was read (Options ›
    /// General): another choice reads it again.
    female: bool,
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
/// Armor and cloaks on a woman where Options say so.
fn preview_of(app: &Moonglow, source: &Source) -> Result<Preview, String> {
    preview_on(app, source, app.settings.armor_on_woman)
}

/// [`preview_of`], armor and cloaks on a woman where `female`.
fn preview_on(app: &Moonglow, source: &Source, female: bool) -> Result<Preview, String> {
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
        ResType::UTI => mg_preview::item_on(game, &object, female),
        ResType::UTP => mg_preview::placeable(game, &object),
        ResType::UTD => mg_preview::door(game, &object),
        // The flag the area view draws. (A store's and a sound's markers
        // are the same for every one: no picture to tell them by.)
        ResType::UTW => mg_preview::waypoint(game, &object),
        t => return Err(format!("{source}: no preview for {t:?}")),
    };
    preview.map(|p| mg_preview::replaced(p, &object)).map_err(|e| format!("{source}: {e}"))
}

/// An Export Files waiting for an answer: the folder has files of some of
/// its names already.
#[derive(Debug, Clone)]
pub struct ModelExport {
    /// What is exported, for the log.
    what: String,
    dir: std::path::PathBuf,
    /// The files to write, each with where it comes from.
    files: Vec<(ResKey, String)>,
    /// Those the folder has already.
    pub existing: Vec<ResKey>,
    /// The game's own files left out.
    left_out: usize,
}

/// Export Files: what the game loads to draw `source` (its models, their
/// walkmeshes, textures and materials), written as `name.ext` into a
/// folder the user chooses. The game's own files are left out, unless
/// every one of them is the game's. Where the folder has files of those
/// names already, it asks first ([`export_window`]).
pub fn export_files(app: &mut Moonglow, source: &Source) {
    let preview = match preview_of(app, source) {
        Ok(p) => p,
        Err(e) => return app.log.error(e),
    };
    let Some(game) = app.game.clone() else { return };
    let files = mg_preview::files::files(&game, &preview);
    if files.is_empty() {
        return app.log.warn(format!("{source}: no model found to export"));
    }
    let stock = files.iter().filter(|f| f.stock).count();
    let all = stock == files.len();
    let Some(dir) = app.dialogs.pick_folder("Export its files into", app.export_dir.as_deref())
    else {
        return;
    };
    let files: Vec<(ResKey, String)> =
        files.into_iter().filter(|f| all || !f.stock).map(|f| (f.key, f.origin)).collect();
    let existing: Vec<ResKey> =
        files.iter().map(|(k, _)| *k).filter(|k| dir.join(k.to_string()).exists()).collect();
    let export = ModelExport {
        what: source.to_string(),
        dir,
        files,
        existing,
        left_out: if all { 0 } else { stock },
    };
    if export.existing.is_empty() {
        write_export(app, &export, true);
    } else {
        app.model_export = Some(export);
    }
}

/// Writes an export's files: all of them, or (`replace` off) those the
/// folder did not have.
fn write_export(app: &mut Moonglow, export: &ModelExport, replace: bool) {
    let Some(game) = app.game.clone() else { return };
    let dir = &export.dir;
    let mut written = Vec::new();
    for (key, origin) in &export.files {
        if !replace && export.existing.contains(key) {
            continue;
        }
        let Ok(data) = game.resman.get(key) else { continue };
        if let Err(e) = key.path_in(dir).and_then(|path| std::fs::write(path, &data)) {
            return app.log.error(format!("Could not write to {}: {e}", dir.display()));
        }
        written.push(format!("{key} ({origin})"));
    }
    let mut notes = String::new();
    if export.left_out > 0 {
        notes.push_str(&format!("; {} of the game's own left out", export.left_out));
    }
    if !replace && !export.existing.is_empty() {
        notes.push_str(&format!("; {} already there kept", export.existing.len()));
    }
    app.log.info(format!(
        "Exported {} files of {} to {}{notes}: {}",
        written.len(),
        export.what,
        dir.display(),
        written.join(", ")
    ));
    app.export_dir = Some(dir.clone());
}

/// Asks what to do with the files an Export Files would write over.
pub(crate) fn export_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(export) = app.model_export.clone() else { return };
    // Replace all (true), skip those (false).
    let mut answer = None;
    let mut cancel = false;
    egui::Window::new("Files Already in the Folder")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let (n, all) = (export.existing.len(), export.files.len());
            ui.label(format!("{} already has {n} of the {all} files:", export.dir.display()));
            egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                for key in &export.existing {
                    ui.label(egui::RichText::new(key.to_string()).monospace());
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .button(format!("Replace {n}"))
                    .on_hover_text("Write all the files: these take the place of the folder's")
                    .clicked()
                {
                    answer = Some(true);
                }
                if ui
                    .button("Skip Those")
                    .on_hover_text("Write the other files and leave the folder's as they are")
                    .clicked()
                {
                    answer = Some(false);
                }
                cancel = crate::widgets::cancel(ui);
            });
        });
    if cancel {
        app.model_export = None;
    }
    let Some(replace) = answer else { return };
    app.model_export = None;
    write_export(app, &export, replace);
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
        let female = app.settings.armor_on_woman;
        if model || (revision == self.revision && female == self.female) {
            return;
        }
        (self.revision, self.female) = (revision, female);
        // The editors' working copies, which the preview reads.
        if let Some(ws) = app.ws.as_mut() {
            let _ = ws.flush();
        }
        // Nothing to show now (an appearance with no model): why, in place
        // of the picture, rather than what it showed before.
        let preview = match preview_of(app, source) {
            Ok(p) => p,
            Err(e) => {
                self.model = Err(e);
                (self.composed, self.preview) = (None, None);
                (self.particles, self.dangly) = (None, None);
                return;
            }
        };
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
        // (An item of its own model: not armor or a cloak, shown worn.)
        let item = match source {
            Source::Resource(k) => k.restype == ResType::UTI,
            Source::Instance { path, .. } => {
                crate::blueprint::instance_type(path) == Some(ResType::UTI)
            }
        };
        let lying = item
            && preview.as_ref().is_ok_and(|p| {
                p.idle.is_none()
                    && !p.base.model.contains("cloak")
                    && !worn_as_modeled(app, source, p)
            });
        // (A shield from its front, the side away from its bearer, as its
        // thumbnail shows it.)
        let shield = preview.as_ref().is_ok_and(|p| is_shield(app, source, p));
        let yaw = if shield { 240f32 } else { 60f32 }.to_radians();
        let stand =
            if lying { Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2) } else { Mat4::IDENTITY };
        let mut view = ModelView {
            model,
            stand,
            supermodels: RefCell::new(HashMap::new()),
            animation,
            playing: true,
            model_lights: true,
            time: 0.0,
            // Models face +Y: from the front, a little to the side.
            yaw,
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
            female: app.settings.armor_on_woman,
        };
        view.frame();
        view
    }

    /// Whether what it shows is stood up for the viewer (an item modeled
    /// lying), not shown as it is.
    pub fn stood(&self) -> bool {
        self.stand != Mat4::IDENTITY
    }

    /// What it shows of a blueprint (none for a model).
    pub fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

    /// Fits the camera to the model's rest pose.
    pub fn frame(&mut self) {
        if let Some(c) = &self.composed {
            let (low, high) = c.bounds();
            // (The box around it as it is stood.)
            let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for corner in 0..8 {
                let p = Vec3::new(
                    if corner & 1 == 0 { low.x } else { high.x },
                    if corner & 2 == 0 { low.y } else { high.y },
                    if corner & 4 == 0 { low.z } else { high.z },
                );
                let w = self.stand.transform_point3(p);
                (min, max) = (min.min(w), max.max(w));
            }
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
    let mut export = false;
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
        let options: Vec<Option<String>> =
            std::iter::once(None).chain(anims.iter().map(|(name, _)| Some(name.clone()))).collect();
        let was = view.animation.clone();
        crate::widgets::dropdown_labelled(("anim", &key), "Animation").selected_text(current).show(
            ui,
            &mut view.animation,
            &options,
            |a| a.clone().unwrap_or_else(|| "(rest pose)".into()),
        );
        if view.animation != was {
            view.time = 0.0;
        }
        if ui.button(if view.playing { "Pause" } else { "Play" }).clicked() {
            view.playing = !view.playing;
        }
        if ui.button("Frame").on_hover_text("Fit the model in the view").clicked() {
            view.frame();
        }
        ui.checkbox(&mut view.model_lights, "Lights")
            .on_hover_text("Light the model with its own light nodes");
        let mut shaders = app.settings.game_shaders();
        if ui
            .checkbox(&mut shaders, "Shaders")
            .on_hover_text(
                "Draw with the game's own shaders, compiled from the game and the module's haks: \
                 a material's custom shaders (swaying grass and leaves, flowing water) and the \
                 game's lit shaders; off, with Moonglow's own (in every 3D view)",
            )
            .changed()
        {
            app.settings.game_shaders = Some(shaders);
            if let Some(vp) = app.viewport.as_mut() {
                vp.renderer.game_shaders = shaders;
            }
        }
        if embedded {
            pop_out =
                ui.button("Pop Out").on_hover_text("Show it in a window of its own").clicked();
        }
        export = ui
            .button("Export Files…")
            .on_hover_text(
                "Write the files the game draws this with into a folder: its models, their \
                 walkmeshes, textures and materials, from the haks and wherever else they are \
                 (the game's own are left out, unless all of it is the game's)",
            )
            .clicked();
    });
    if export {
        export_files(app, &source);
    }

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
    vp.size_targets(&mut view.targets, w, h);
    let (targets, id) = view.targets.as_ref().expect("made above");
    // The base (with the viewer's animated meshes and dangly state), then
    // what hangs from it.
    let mut instances = match &view.composed {
        Some(c) => c.instances(view.animation.as_deref(), view.time, view.stand),
        None => vec![Instance::new(model.clone(), Mat4::IDENTITY)],
    };
    if let Some(base) = instances.first_mut() {
        base.pose = pose;
        base.state = Some(Arc::new(state));
    }
    instances.extend(chunk_instances);
    let mut lights = lights;
    if let Some(c) = &view.composed {
        // (The base's light nodes are the viewer's own above, as Lights
        // has them.)
        lights.extend(c.row_lights(Mat4::IDENTITY));
    }
    let scene = Scene {
        instances,
        lights,
        particles: batches,
        area: AreaLight::default(),
        background: [0.16, 0.18, 0.21],
        // (The game's shaders' clock: wind sways while the view plays.)
        time: view.time,
        ..Default::default()
    };
    let resman = app.game.as_deref().map(|g| &g.resman);
    let assets: &dyn mg_render::Assets = match resman {
        Some(rm) => rm,
        None => &mg_render::NoAssets,
    };
    // (Drawn again and again: the game's programs are made while it goes
    // on, Moonglow's own shader drawing until they are.)
    vp.renderer.game_background = true;
    vp.renderer.render_targets(&vp.gpu, assets, &scene, &camera, targets);
    vp.renderer.game_background = false;
    // A program of the game's that reads the clock moves what it draws:
    // drawn again as the area view's animations step, while the view plays;
    // and, while its pipelines are made, until they are.
    if (vp.renderer.drawn.timed > 0 && view.playing) || vp.renderer.drawn.game_pending > 0 {
        ui.ctx().request_repaint_after(std::time::Duration::from_secs_f32(CLOCK_STEP));
    }
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

/// Pictures of blueprints, models, creatures' looks and tiles as they look
/// (the galleries, a hovered row's preview, a tile's variants). A view
/// asks for those it shows, every frame it shows them ([`Thumbnails::get`],
/// [`Thumbnails::show`]); those not made yet are made after the frame, a
/// few at a time ([`make_thumbnails`]), and those not asked for any more
/// are let go when there are many.
#[derive(Default)]
pub struct Thumbnails {
    made: HashMap<Kept, Made>,
    /// Asked for this frame and not made (or made before the module last
    /// changed), in the order asked.
    wanted: Vec<Kept>,
    /// The frames, counted, and the module's revision in this one.
    frame: u64,
    revision: Option<u64>,
    /// Armor is shown on a woman in this frame (Options › General): an
    /// item blueprint's picture is hers.
    female: bool,
    /// The items pictured by their looks ([`Pictured::Item`]), and whether
    /// on a woman: those asked for and kept.
    items: HashMap<u64, (mg_gff::Struct, bool)>,
    /// The revision at which the editors' working copies were last written
    /// into the module for the blueprints' looks (once a revision: it
    /// writes every document the workspace has read).
    flushed: Option<u64>,
    /// What every picture is drawn into, before it is resolved into a
    /// texture of its own.
    targets: Option<Targets>,
    /// How many pictures have been drawn.
    pub drawn: u64,
    /// How many pictures out of sight are kept, if not [`THUMBNAILS_KEPT`].
    pub keep: Option<usize>,
}

/// What a picture is kept by: what it is of, and whether as a woman wears
/// it (an item blueprint's, while armor is shown on a woman): a man's
/// picture and a woman's are kept apart, so choosing again shows those
/// made before at once.
type Kept = (Pictured, bool);

/// A picture made.
struct Made {
    /// Its texture, and egui's name for it; `None`: nothing to draw.
    picture: Option<(wgpu::Texture, egui::TextureId)>,
    /// A blueprint's look when its picture was drawn, and the module's
    /// revision it was last found to look so at: after an edit, the
    /// picture is kept if the blueprint still looks the same.
    look: Option<Preview>,
    revision: Option<u64>,
    /// The frame it was last asked for in.
    used: u64,
}

/// What a thumbnail is of: a blueprint or a model, or a creature's look
/// (an appearance with a plain body, for choosing among appearances).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Pictured {
    Resource(ResKey),
    Look(mg_preview::CreatureLook),
    /// A tile's model lying at this many quarter turns, as it would in
    /// an area (a tile's variants, which differ by their turn too).
    Tile(ResKey, u8),
    /// A tileset's group (its index): its tiles' models, each in its place,
    /// as the group is placed (a feature, a building).
    Group(mg_core::ResRef, u32),
    /// An item as its fields make it look (with another model or part, for
    /// choosing among them), by those fields: [`Thumbnails::item`].
    Item(u64),
}

impl Pictured {
    /// Whether an edit of the module may change the picture: a
    /// blueprint's (a model's is its own, whatever the module's
    /// blueprints say).
    fn follows_the_module(self) -> bool {
        matches!(self, Pictured::Resource(k) if k.restype != ResType::MDL)
    }
}

impl Thumbnails {
    /// What `key`'s picture is kept by in this frame: a woman's of an item
    /// blueprint while armor is shown on one.
    fn kept_as(&self, key: Pictured) -> Kept {
        let item = matches!(key, Pictured::Resource(k) if k.restype == ResType::UTI);
        (key, self.female && item)
    }

    /// The pictures made that this frame shows, by what they are of (of
    /// item blueprints, those of the body armor is shown on).
    fn current(&self) -> impl Iterator<Item = (Pictured, &Made)> {
        self.made.iter().filter(|(k, _)| self.kept_as(k.0) == **k).map(|(k, made)| (k.0, made))
    }

    /// The thumbnails made, by blueprint.
    pub(crate) fn all(&self) -> HashMap<ResKey, Option<egui::TextureId>> {
        self.current()
            .filter_map(|(k, made)| match k {
                Pictured::Resource(k) => Some((k, made.id())),
                Pictured::Look(_)
                | Pictured::Tile(..)
                | Pictured::Group(..)
                | Pictured::Item(_) => None,
            })
            .collect()
    }

    /// Every thumbnail made.
    pub(crate) fn every(&self) -> HashMap<Pictured, Option<egui::TextureId>> {
        self.current().map(|(k, made)| (k, made.id())).collect()
    }

    /// The thumbnails made of creatures' looks.
    pub(crate) fn looks(&self) -> HashMap<mg_preview::CreatureLook, Option<egui::TextureId>> {
        self.current()
            .filter_map(|(k, made)| match k {
                Pictured::Look(l) => Some((l, made.id())),
                Pictured::Resource(_)
                | Pictured::Tile(..)
                | Pictured::Group(..)
                | Pictured::Item(_) => None,
            })
            .collect()
    }

    /// A new frame, the module at `revision`, armor shown on a woman where
    /// `female`.
    pub(crate) fn begin_frame(&mut self, revision: Option<u64>, female: bool) {
        self.frame += 1;
        self.revision = revision;
        self.female = female;
        self.wanted.clear();
    }

    /// What pictures `item` (an item's fields: as it would be with another
    /// model or part), on a woman where `female` (armor and cloaks).
    pub(crate) fn item(&mut self, item: &mg_gff::Struct, female: bool) -> Pictured {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        // (Its fields as written out: a struct has no hash of its own.)
        format!("{item:?}").hash(&mut h);
        female.hash(&mut h);
        let id = h.finish();
        self.items.entry(id).or_insert_with(|| (item.clone(), female));
        Pictured::Item(id)
    }

    /// The pictures a view shows this frame: kept while they are shown,
    /// and made if they aren't yet (or looked at again, if the module has
    /// changed since).
    pub(crate) fn show(&mut self, keys: impl IntoIterator<Item = Pictured>) {
        for key in keys {
            let kept = self.kept_as(key);
            match self.made.get_mut(&kept) {
                Some(made) => {
                    made.used = self.frame;
                    if key.follows_the_module() && made.revision != self.revision {
                        self.wanted.push(kept);
                    }
                }
                None => self.wanted.push(kept),
            }
        }
    }

    /// A picture, if it is made (`Some(None)`: there is nothing to draw of
    /// it); it is asked for either way, as [`Thumbnails::show`] asks.
    pub(crate) fn get(&mut self, key: Pictured) -> Option<Option<egui::TextureId>> {
        self.show([key]);
        self.made.get(&self.kept_as(key)).map(Made::id)
    }

    /// Lets go of every picture (another module, or the game's data read
    /// anew: the same names may now look different).
    pub(crate) fn forget(&mut self, viewport: Option<&Viewport3d>) {
        let old = std::mem::take(&mut self.made);
        if let Some(vp) = viewport {
            let mut r = vp.render_state.renderer.write();
            for (_, id) in old.into_values().filter_map(|made| made.picture) {
                r.free_texture(&id);
            }
        }
        self.wanted.clear();
        self.items.clear();
        self.flushed = None;
    }

    /// How many pictures are kept.
    pub fn kept(&self) -> usize {
        self.made.len()
    }

    /// How many pictures asked for this frame are still to make.
    pub fn waiting(&self) -> usize {
        self.wanted.len()
    }
}

impl Made {
    fn id(&self) -> Option<egui::TextureId> {
        self.picture.as_ref().map(|(_, id)| *id)
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

/// The pictures kept at most, those not shown in this frame or the last:
/// past it, the ones longest unseen are let go. (What a view shows is
/// kept, however much: a gallery over a wide window shows hundreds.)
const THUMBNAILS_KEPT: usize = 512;

/// How long pictures are made for in a frame: once it has passed, the
/// rest wait for the next frame (one is made in every frame, however
/// long it takes). A picture takes several milliseconds (its models and
/// textures read and uploaded, then drawn), and a gallery wants hundreds:
/// three a frame, as they were made, was 25 ms a frame while it filled.
const THUMBNAIL_TIME: std::time::Duration = std::time::Duration::from_millis(2);

/// After a frame: makes the pictures its views asked for and lack, for
/// [`THUMBNAIL_TIME`], and asks for another frame if there are more (or
/// to show those made).
pub(crate) fn make_thumbnails(app: &mut Moonglow, ctx: &egui::Context) {
    let wanted = std::mem::take(&mut app.thumbnails.wanted);
    let started = std::time::Instant::now();
    let revision = app.thumbnails.revision;
    // The editors' working copies, which a blueprint's look is read from.
    if app.thumbnails.flushed != revision
        && wanted.iter().any(|(k, _)| k.follows_the_module())
        && let Some(ws) = app.ws.as_mut()
    {
        let _ = ws.flush();
        app.thumbnails.flushed = revision;
    }
    let (mut made, mut waiting) = (0, Vec::new());
    for key in wanted {
        let current = app
            .thumbnails
            .made
            .get(&key)
            .is_some_and(|m| !key.0.follows_the_module() || m.revision == revision);
        // (Asked for twice in the frame, and made the first time.)
        if current {
            continue;
        }
        if made > 0 && started.elapsed() >= THUMBNAIL_TIME {
            waiting.push(key);
            continue;
        }
        make_thumbnail(app, key);
        made += 1;
    }
    if made > 0 || !waiting.is_empty() {
        ctx.request_repaint();
    }
    app.thumbnails.wanted = waiting;
    // Those not shown any more go, the longest unseen first, when there
    // are many.
    let thumbnails = &mut app.thumbnails;
    let keep = thumbnails.keep.unwrap_or(THUMBNAILS_KEPT);
    if thumbnails.made.len() > keep {
        let frame = thumbnails.frame;
        let mut unseen: Vec<(u64, Kept)> = thumbnails
            .made
            .iter()
            .filter(|(_, m)| m.used + 1 < frame)
            .map(|(k, m)| (m.used, *k))
            .collect();
        unseen.sort_unstable_by_key(|(used, _)| *used);
        let over = thumbnails.made.len() - keep;
        let gone: Vec<Made> =
            unseen.iter().take(over).filter_map(|(_, k)| thumbnails.made.remove(k)).collect();
        if let Some(vp) = &app.viewport {
            let mut r = vp.render_state.renderer.write();
            for (_, id) in gone.into_iter().filter_map(|made| made.picture) {
                r.free_texture(&id);
            }
        }
    }
    // The items pictured by their looks that are neither kept nor still
    // to make (a row the pointer has left).
    let thumbnails = &mut app.thumbnails;
    if !thumbnails.items.is_empty() {
        let (made, wanted) = (&thumbnails.made, &thumbnails.wanted);
        thumbnails.items.retain(|id, _| {
            let key = (Pictured::Item(*id), false);
            made.contains_key(&key) || wanted.contains(&key)
        });
    }
}

/// Makes a picture, or finds the one made before the module changed to be
/// what the blueprint still looks like.
fn make_thumbnail(app: &mut Moonglow, kept: Kept) {
    let (frame, revision) = (app.thumbnails.frame, app.thumbnails.revision);
    let key = kept.0;
    if let Pictured::Group(tileset, group) = key {
        let picture = draw_group_thumbnail(app, tileset, group);
        app.thumbnails.drawn += 1;
        let made = Made { picture, look: None, revision, used: frame };
        if let Some(old) = app.thumbnails.made.insert(kept, made)
            && let (Some((_, id)), Some(vp)) = (old.picture, &app.viewport)
        {
            vp.render_state.renderer.write().free_texture(&id);
        }
        return;
    }
    let look = pictured_look(app, kept);
    let same = key.follows_the_module().then_some(look.as_ref().map(|(_, preview)| preview));
    if let (Some(made), Some(now)) = (app.thumbnails.made.get_mut(&kept), same)
        && made.look.as_ref() == now
    {
        made.revision = revision;
        return;
    }
    let picture =
        look.as_ref().and_then(|(source, preview)| draw_thumbnail(app, key, source, preview));
    let look = look.map(|(_, preview)| preview).filter(|_| key.follows_the_module());
    let made = Made { picture, look, revision, used: frame };
    app.thumbnails.drawn += 1;
    if let Some(old) = app.thumbnails.made.insert(kept, made)
        && let (Some((_, id)), Some(vp)) = (old.picture, &app.viewport)
    {
        vp.render_state.renderer.write().free_texture(&id);
    }
}

/// What a picture is drawn from: `None` for what has no picture.
fn pictured_look(app: &Moonglow, (key, female): Kept) -> Option<(Source, Preview)> {
    match key {
        Pictured::Resource(key) | Pictured::Tile(key, _) => {
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
            let source = Source::Resource(key);
            let preview = preview_on(app, &source, female).ok()?;
            Some((source, preview))
        }
        Pictured::Item(id) => {
            let (item, female) = app.thumbnails.items.get(&id)?;
            let game = app.game.as_deref()?;
            let preview = mg_preview::item_on(game, item, *female).ok()?;
            // (Named as an item's, which tells a shield's from the others'.)
            let source = Source::Resource(ResKey::parse("item", ResType::UTI)?);
            Some((source, mg_preview::replaced(preview, item)))
        }
        Pictured::Look(look) => {
            let preview = mg_preview::creature_look(app.game.as_deref()?, &look).ok()?;
            // (Named, for a message, by the table its row is of.)
            Some((Source::Resource(ResKey::parse("appearance", ResType::TWODA)?), preview))
        }
        // (Several models: drawn by `draw_group_thumbnail`.)
        Pictured::Group(..) => None,
    }
}

/// Whether a preview is a shield's: an item of a base item that goes in
/// the left hand alone (`EquipableSlots`: 0x20 and not the right hand's
/// 0x10, which a torch has too), by its model's name (`ItemClass` and
/// what follows). A shield's model faces its bearer: its front, which
/// tells one from another, is the side away from where models face.
fn is_shield(app: &Moonglow, source: &Source, preview: &Preview) -> bool {
    worn_slots(app, source, preview).is_some_and(|s| s & 0x30 == 0x20)
}

/// Whether a preview is an item modeled as it is worn, a helmet (on the
/// head, `EquipableSlots` 0x1) or a shield ([`is_shield`]): shown so, not
/// stood up as an item modeled lying is (a helmet lay on its side, a
/// shield on its back, a builder's report).
fn worn_as_modeled(app: &Moonglow, source: &Source, preview: &Preview) -> bool {
    worn_slots(app, source, preview).is_some_and(|s| s & 0x30 == 0x20 || s & 0x1 != 0)
}

/// The slots an item's preview is worn in (baseitems.2da's
/// `EquipableSlots`, by its model's name: `ItemClass` and what follows);
/// `None` if it is no item's.
fn worn_slots(app: &Moonglow, source: &Source, preview: &Preview) -> Option<i32> {
    let item = match source {
        Source::Resource(k) => k.restype == ResType::UTI,
        Source::Instance { path, .. } => {
            crate::blueprint::instance_type(path) == Some(ResType::UTI)
        }
    };
    let t = app.game.as_deref().filter(|_| item)?.table("baseitems").ok()?;
    let class = preview.base.model.split('_').next().unwrap_or_default();
    (0..t.len())
        .filter(|row| t.get(*row, "ItemClass").is_some_and(|c| c.eq_ignore_ascii_case(class)))
        .find_map(|row| t.get_int(row, "EquipableSlots"))
}

fn draw_thumbnail(
    app: &mut Moonglow,
    key: Pictured,
    source: &Source,
    preview: &Preview,
) -> Option<(wgpu::Texture, egui::TextureId)> {
    // (A tile is seen from one side always, turned as it lies.)
    let turned = match key {
        Pictured::Tile(_, quarters) => Some(f32::from(quarters % 4) * std::f32::consts::FRAC_PI_2),
        _ => None,
    };
    let shield = is_shield(app, source, preview);
    let composed = compose(app, source, preview).ok()?;
    // (Drawn with the GPU: none, no picture.)
    app.viewport.as_ref()?;
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
    // (Of `first` and the others, the side that shows half as much again.)
    let best = |first: f32, others: [f32; 3]| {
        others
            .into_iter()
            .fold((first, composed.facing(towards(first))), |best, yaw| {
                let shows = composed.facing(towards(yaw));
                if shows > best.1 * 1.5 + 1e-3 { (yaw, shows) } else { best }
            })
            .0
    };
    let quarter = std::f32::consts::FRAC_PI_2;
    let yaw = match turned {
        // From the south-west, as an area opens (the camera goes round the
        // other way from the tile's turn), or from another corner where
        // that shows much more: a chasm's wall faces into the chasm, and
        // from behind it showed only its lip and the floor far below.
        Some(by) => {
            let sw = -120f32.to_radians() - by;
            best(sw, [sw + quarter, sw - quarter, sw + 2.0 * quarter])
        }
        // (A shield from its front.)
        None if shield => usual + std::f32::consts::PI,
        None => best(usual, [usual + std::f32::consts::PI, usual + 1.57, usual - 1.57]),
    };
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
    // The sun goes round with the view: a thing seen from another side
    // than the usual (a shield, a banner) is lit on the side seen.
    let mut area = AreaLight::default();
    if turned.is_none() {
        area.direction = glam::Quat::from_rotation_z(yaw - usual) * area.direction;
    }
    let scene = Scene {
        instances: composed.instances(idle, 0.0, Mat4::IDENTITY),
        lights: composed.point_lights(Mat4::IDENTITY),
        area,
        background: [0.16, 0.18, 0.21],
        particles,
        ..Default::default()
    };
    render_picture(app, &scene, &camera)
}

/// A tileset's group as a picture: as it stands placed, unturned, in open
/// ground, with the tiles around it that its placement changes (a castle's
/// gate, a single tile, in the castle its corners call for), seen from the
/// south-west and a little above, as a tile's picture is. `None` where
/// none of its tiles can be drawn.
fn draw_group_thumbnail(
    app: &mut Moonglow,
    tileset: mg_core::ResRef,
    group: u32,
) -> Option<(wgpu::Texture, egui::TextureId)> {
    let tools = (app.area_views.values().filter_map(|v| v.terrain.as_ref()))
        .find(|t| t.tileset == tileset)
        .map(|t| (t.set.clone(), Some(t.index.clone())));
    let (set, index) = tools.or_else(|| {
        let game = app.game.as_deref()?;
        mg_area::tileset(game, tileset).ok().map(|set| (Arc::new(set), None))
    })?;
    let g = set.groups.get(group as usize)?;
    let index = index.unwrap_or_else(|| mg_tiles::TileIndex::new(&set));
    // The tiles and their places: as placed (the group's first tile at
    // the origin), else the group's own tiles alone.
    let placed: Vec<((i64, i64), mg_tiles::Placement)> =
        match mg_tiles::paint::group_placed(&index, &set, group as usize) {
            Some(placed) => {
                let m = i64::from(mg_tiles::paint::GROUP_MARGIN);
                let at = |(x, y): (u32, u32)| (i64::from(x) - m, i64::from(y) - m);
                placed.into_iter().map(|(cell, p)| (at(cell), p)).collect()
            }
            None => (g.tiles.iter().enumerate())
                .filter_map(|(k, t)| {
                    let p = mg_tiles::Placement { tile: (*t)?, orientation: 0, height: 0 };
                    Some((mg_tiles::group_slot(k, g.columns, 0), p))
                })
                .collect(),
        };
    // (Each tile's models kept until the picture is drawn.)
    let mut parts = Vec::new();
    for ((c, r), p) in placed {
        let Some(t) = set.tiles.get(p.tile as usize) else { continue };
        let Some(key) = ResKey::parse(&t.model.to_ascii_lowercase(), ResType::MDL) else {
            continue;
        };
        let source = Source::Resource(key);
        let Ok(preview) = preview_of(app, &source) else { continue };
        let Ok(composed) = compose(app, &source, &preview) else { continue };
        let at = Vec3::new(
            c as f32 * mg_area::TILE_SIZE,
            r as f32 * mg_area::TILE_SIZE,
            p.height as f32 * set.general.transition,
        );
        let turn =
            glam::Quat::from_rotation_z(f32::from(p.orientation % 4) * std::f32::consts::FRAC_PI_2);
        parts.push((composed, Mat4::from_rotation_translation(turn, at)));
    }
    if parts.is_empty() {
        return None;
    }
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let (mut instances, mut lights) = (Vec::new(), Vec::new());
    for (composed, at) in &parts {
        let (low, high) = composed.bounds();
        (min, max) = (min.min(at.transform_point3(low)), max.max(at.transform_point3(high)));
        instances.extend(composed.instances(None, 0.0, *at));
        // (A group's tiles: their light nodes are main lights, which the
        // area's tiles colour; the picture is drawn without them.)
        lights.extend(composed.row_lights(*at));
    }
    let pitch = 30f32.to_radians();
    let yaw = -120f32.to_radians();
    let target = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(0.1);
    let distance = radius / 20f32.to_radians().sin() * 0.9;
    let camera = Camera::orbit(target, distance, yaw, pitch);
    let scene = Scene {
        instances,
        lights,
        area: AreaLight::default(),
        background: [0.16, 0.18, 0.21],
        ..Default::default()
    };
    render_picture(app, &scene, &camera)
}

/// A picture of `scene` from `camera`, in a texture of its own, and egui's
/// name for it: a thumbnail.
fn render_picture(
    app: &mut Moonglow,
    scene: &Scene,
    camera: &Camera,
) -> Option<(wgpu::Texture, egui::TextureId)> {
    let vp = app.viewport.as_mut()?;
    // Drawn into the targets every picture is drawn into, and resolved
    // into a texture of its own: what is kept of it. (Each kept its own
    // multisampled and depth targets, nine times its picture's memory.)
    let size = THUMBNAIL;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let targets = app
        .thumbnails
        .targets
        .get_or_insert_with(|| Targets::new(&vp.gpu, format, SAMPLES, size, size));
    let picture = vp.gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("thumbnail"),
        size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = picture.create_view(&Default::default());
    let (target, resolve) = match &targets.msaa_view {
        Some(multisampled) => (multisampled, Some(&view)),
        None => (&view, None),
    };
    let resman = app.game.as_deref().map(|g| &g.resman);
    let assets: &dyn mg_render::Assets = match resman {
        Some(rm) => rm,
        None => &mg_render::NoAssets,
    };
    vp.renderer.render(
        &vp.gpu,
        assets,
        scene,
        camera,
        target,
        resolve,
        &targets.depth,
        (size, size),
    );
    let id = vp.render_state.renderer.write().register_native_texture(
        &vp.gpu.device,
        &view,
        wgpu::FilterMode::Linear,
    );
    Some((picture, id))
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
        // (A tile's light nodes are main lights, which the area's tiles
        // colour: the picture is drawn without them.)
        lights: composed.row_lights(Mat4::IDENTITY),
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
