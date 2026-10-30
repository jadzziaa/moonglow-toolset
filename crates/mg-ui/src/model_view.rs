//! The model viewer: any model from the resources, or what a creature, item,
//! placeable or door blueprint looks like (`mg_preview`), drawn with the
//! game's lighting, turned by dragging (left: orbit, right or middle: pan,
//! wheel: zoom, as the area view's wheel), with its animations (and its
//! supermodels') to play.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_core::ResRef;
use mg_core::ResType;
use mg_edit::{GffPath, Workspace};
use mg_gff::{Gff, Struct};
use mg_mdl::Model;
use mg_preview::Preview;
use mg_preview::compose::Composed;
use mg_render::anim;
use mg_render::dangly::Dangly;
use mg_render::particles::Particles;
use mg_render::{AreaLight, Camera, Gpu, GpuModel, Instance, MeshState, Renderer, Scene, Targets};
use mg_resman::ResKey;

use crate::Moonglow;
use crate::area_view::{Orbit, Wheel};

/// Multisampling for 3D views.
pub(crate) const SAMPLES: u32 = 4;

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
    /// renderer takes it ([`mg_render::Renderer::read_textures_apart`]):
    /// until the game data is to be changed ([`Viewport3d::release_game_data`]).
    textures_from: Option<Arc<GameAssets>>,
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

    fn game_shader(&self, name: &str) -> Option<String> {
        mg_render::Assets::game_shader(&self.0.resman, name)
    }

    fn shader_origin(&self, name: &str) -> Option<String> {
        mg_render::Assets::shader_origin(&self.0.resman, name)
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
    /// the game data is, and is not released.
    pub(crate) fn textures_from(
        &mut self,
        game: &Arc<mg_rules::GameData>,
    ) -> Arc<dyn mg_render::Assets + Send + Sync> {
        match &self.textures_from {
            Some(from) if Arc::ptr_eq(&from.0, game) => from.clone(),
            _ => {
                let from = Arc::new(GameAssets(game.clone()));
                self.textures_from = Some(from.clone());
                from
            }
        }
    }

    /// Lets go of the game data the textures were read apart from (the
    /// renderer's readers with it; those read are kept): to be changed, it
    /// must be the window's alone ([`crate::exclusive`]). Taken again by the
    /// next area view drawn. (Kept, a module's haks could not be layered
    /// into the game data, nor its edits, once an area had been drawn; a
    /// builder's report on 2.13.1.)
    pub(crate) fn release_game_data(&mut self) {
        self.textures_from = None;
        self.renderer.read_textures_apart(&self.gpu, None);
    }

    /// Whether textures are read apart from `game` ([`Self::textures_from`]).
    pub(crate) fn holds(&self, game: &Arc<mg_rules::GameData>) -> bool {
        self.textures_from.as_ref().is_some_and(|from| Arc::ptr_eq(&from.0, game))
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
    /// The camera, turned about what it looks at, as the area view's.
    pub orbit: Orbit,
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
    Instance {
        area: ResRef,
        path: GffPath,
    },
    /// The copy of a resource a layer of the load order has (`layer`, by
    /// its label), which a higher one overrides (the resource browser's).
    Copy {
        key: ResKey,
        layer: String,
    },
}

impl Source {
    /// What the viewer of a tab shows (`None`: the tab is no model's).
    pub(crate) fn of_tab(tab: &crate::Tab) -> Option<Source> {
        Some(match tab {
            crate::Tab::Model(k) => Source::Resource(*k),
            crate::Tab::InstanceModel { area, path } => {
                Source::Instance { area: *area, path: path.clone() }
            }
            crate::Tab::ModelCopy { key, layer } => {
                Source::Copy { key: *key, layer: layer.clone() }
            }
            _ => return None,
        })
    }

    /// The tab whose viewer shows it.
    fn tab(&self) -> crate::Tab {
        match self {
            Source::Resource(k) => crate::Tab::Model(*k),
            Source::Instance { area, path } => {
                crate::Tab::InstanceModel { area: *area, path: path.clone() }
            }
            Source::Copy { key, layer } => {
                crate::Tab::ModelCopy { key: *key, layer: layer.clone() }
            }
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Resource(k) => write!(f, "{k}"),
            Source::Instance { area, .. } => write!(f, "an object placed in {area}"),
            Source::Copy { key, layer } => write!(f, "{key} from {layer}"),
        }
    }
}

impl Source {
    /// The resource it is (none for a placed object).
    fn key(&self) -> Option<ResKey> {
        match self {
            Source::Resource(k) | Source::Copy { key: k, .. } => Some(*k),
            Source::Instance { .. } => None,
        }
    }

    /// Whether it is a model, shown as it is.
    fn is_model(&self) -> bool {
        self.key().is_some_and(|k| k.restype == ResType::MDL)
    }

    /// Whether it is an item (shown lying, or worn).
    fn is_item(&self) -> bool {
        match self {
            Source::Resource(k) | Source::Copy { key: k, .. } => k.restype == ResType::UTI,
            Source::Instance { path, .. } => {
                crate::blueprint::instance_type(path) == Some(ResType::UTI)
            }
        }
    }

    /// `key`'s bytes for it: a layer's copy's own for that copy, else as
    /// [`resource_data`] reads them.
    fn data(&self, app: &Moonglow, key: &ResKey) -> Option<Vec<u8>> {
        match self {
            Source::Copy { key: k, layer } if k == key => {
                let rm = &app.game.as_deref()?.resman;
                let at = rm.layers().iter().position(|l| l.label == *layer)?;
                rm.get_in(at, key).ok().map(|d| d.into_owned())
            }
            _ => resource_data(app, key),
        }
    }
}

/// What a source looks like: a model, a blueprint's preview, or a placed
/// object's (its GIT entry has the blueprint's fields, as edited there).
/// Armor and cloaks on a woman where Options say so.
fn preview_of(app: &Moonglow, source: &Source) -> Result<Preview, String> {
    preview_on(app, source, app.settings.armor_on_woman)
}

/// A resource as the module has it (its editors' copies as last written
/// into it), else as the game's load order does.
fn resource_data(app: &Moonglow, key: &ResKey) -> Option<Vec<u8>> {
    let module = app.ws.as_ref().and_then(|w| w.module.get(key).map(<[u8]>::to_vec));
    module.or_else(|| app.game.as_deref()?.resman.get(key).ok().map(|d| d.into_owned()))
}

/// What a source is of: a blueprint's fields, or a placed object's (its
/// entry in the area's GIT), with the blueprint type it is of; `None` for
/// a model.
fn source_object(app: &Moonglow, source: &Source) -> Result<Option<(ResType, Struct)>, String> {
    let read = |k: &ResKey| source.data(app, k).and_then(|d| Gff::read(&d).ok());
    Ok(Some(match source {
        _ if source.is_model() => return Ok(None),
        Source::Resource(key) | Source::Copy { key, .. } => {
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
    }))
}

/// [`preview_of`], armor and cloaks on a woman where `female`.
fn preview_on(app: &Moonglow, source: &Source, female: bool) -> Result<Preview, String> {
    let game = app.game.as_deref().ok_or("No game data.")?;
    let Some((restype, object)) = source_object(app, source)? else {
        let key = source.key().expect("a model is a resource");
        return Ok(Preview::model(&key.resref.to_string()));
    };
    let read = |k: &ResKey| resource_data(app, k).and_then(|d| Gff::read(&d).ok());
    let items = |r: ResRef| read(&ResKey::new(r, ResType::UTI));
    // (A store's and a sound's markers are the same for every one: no
    // picture to tell them by; a waypoint is the flag the area view draws.)
    let kind = mg_rules::ObjectKind::from_restype(restype)
        .filter(|k| !matches!(k, mg_rules::ObjectKind::Store | mg_rules::ObjectKind::Sound));
    let shown = mg_preview::Shown::Alone { female };
    match kind.and_then(|k| mg_preview::of_object(game, k, &object, &items, shown)) {
        Some(preview) => preview.map_err(|e| format!("{source}: {e}")),
        None => Err(format!("{source}: no preview for {restype:?}")),
    }
}

/// An Export Files waiting for an answer: the folder has files of some of
/// its names already.
#[derive(Debug, Clone)]
pub struct ModelExport {
    /// What is exported, for the log.
    what: String,
    dir: std::path::PathBuf,
    /// The files to write, each with where it comes from.
    files: Vec<ExportFile>,
    /// The names of those the folder has already.
    pub existing: Vec<String>,
    /// The game's own files left out.
    left_out: usize,
    /// The models and textures named and found nowhere.
    pub missing: Vec<String>,
}

/// One file of an Export Files.
#[derive(Debug, Clone)]
struct ExportFile {
    /// Its name in the folder.
    name: String,
    /// Where it comes from (`hak:name`, `the module`…).
    origin: String,
    what: Exported,
}

#[derive(Debug, Clone)]
enum Exported {
    /// A resource, as the module has it, else the game's load order.
    Resource(ResKey),
    /// Made for the export (a table's row).
    Text(String),
}

/// Export Files: what the game loads to draw `source` (its models, their
/// walkmeshes, textures and materials), with a blueprint's file (a placed
/// object's: the blueprint it was placed from) and its row of the table
/// its look is chosen by ([`mg_preview::files::look_row`]), written into
/// a folder the user chooses. The game's own models and textures are left
/// out, unless every one of them is the game's. Where the folder has files
/// of those names already, it asks first ([`export_window`]). What the
/// model names and the load order lacks is said in the log.
pub fn export_files(app: &mut Moonglow, source: &Source) {
    // (The editors' copies, which the files are read from.)
    if let Some(ws) = app.ws.as_mut() {
        let _ = ws.flush();
    }
    let (preview, object) = match preview_of(app, source).and_then(|p| {
        let object = source_object(app, source)?;
        Ok((p, object))
    }) {
        Ok(found) => found,
        Err(e) => return app.log.error(e),
    };
    let Some(game) = app.game.clone() else { return };
    let mut gather = mg_preview::files::Gatherer::new(&game.resman);
    gather.preview(&preview);
    let mut out = Vec::new();
    if let Some((restype, object)) = &object {
        // The blueprint itself (a placed object's: the one it was placed
        // from, where it is found).
        let key = match source {
            Source::Resource(key) | Source::Copy { key, .. } => Some(*key),
            Source::Instance { .. } => {
                object.resref("TemplateResRef").map(|r| ResKey::new(r, *restype))
            }
        };
        let in_module = |k: &ResKey| app.ws.as_ref().is_some_and(|w| w.module.contains(k));
        if let Some(key) = key.filter(|k| in_module(k) || game.resman.contains(k)) {
            let origin = if in_module(&key) {
                "the module".to_string()
            } else {
                game.resman.origin(&key).unwrap_or_default().to_string()
            };
            out.push(ExportFile { name: key.to_string(), origin, what: Exported::Resource(key) });
        }
        let kind = mg_rules::ObjectKind::from_restype(*restype);
        if let Some(row) = kind.and_then(|k| mg_preview::files::look_row(&game, k, object)) {
            let (name, origin) = (row.file_name(), format!("{}.2da of {}", row.table, row.origin));
            out.push(ExportFile { name, origin, what: Exported::Text(row.text) });
        }
    }
    export_gathered(app, source.to_string(), out, gather);
}

/// Export Model Files… of an area (the module tree's menu): what the game
/// draws it with ([`mg_preview::files::Gatherer::area`]: its tileset, its
/// tiles' models, walkmeshes and minimap pictures, its skybox, and what is
/// placed in it), with its own `.are` and `.git`, as [`export_files`]
/// writes a blueprint's.
pub fn export_area_files(app: &mut Moonglow, area: ResRef) {
    if let Some(ws) = app.ws.as_mut() {
        let _ = ws.flush();
    }
    let Some(game) = app.game.clone() else { return };
    let read = |t: ResType| {
        let key = ResKey::new(area, t);
        resource_data(app, &key).and_then(|d| Gff::read(&d).ok()).map(|g| (key, g.root))
    };
    let Some((are_key, are)) = read(ResType::ARE) else {
        return app.log.error(format!("{area}.are: not found or not readable"));
    };
    let git = read(ResType::GIT);
    let mut gather = mg_preview::files::Gatherer::new(&game.resman);
    gather.area(&game, &are, None, git.as_ref().map(|(_, s)| s));
    let own = [Some(are_key), git.map(|(k, _)| k)];
    let out = own
        .into_iter()
        .flatten()
        .map(|key| ExportFile {
            name: key.to_string(),
            origin: "the module".into(),
            what: Exported::Resource(key),
        })
        .collect();
    export_gathered(app, format!("area {area}"), out, gather);
}

/// What an Export Files writes, after its own files (`out`: a blueprint
/// and its row, an area's ARE and GIT): the files `gather` found, the
/// game's own left out unless every one is the game's, into a folder the
/// user chooses; asked about first where the folder has some of them.
fn export_gathered(
    app: &mut Moonglow,
    what: String,
    mut out: Vec<ExportFile>,
    gather: mg_preview::files::Gatherer<'_>,
) {
    let missing = gather.missing().to_vec();
    let files = gather.into_files();
    if files.is_empty() {
        return app.log.warn(format!("{what}: no model found to export"));
    }
    let stock = files.iter().filter(|f| f.stock).count();
    let all = stock == files.len();
    let Some(dir) = app.dialogs.pick_folder("Export its files into", app.export_dir.as_deref())
    else {
        return;
    };
    out.extend(files.into_iter().filter(|f| all || !f.stock).map(|f| ExportFile {
        name: f.key.to_string(),
        origin: f.origin,
        what: Exported::Resource(f.key),
    }));
    let existing: Vec<String> =
        out.iter().map(|f| f.name.clone()).filter(|n| dir.join(n).exists()).collect();
    let export = ModelExport {
        what,
        dir,
        files: out,
        existing,
        left_out: if all { 0 } else { stock },
        missing,
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
    let dir = &export.dir;
    let mut written = Vec::new();
    for f in &export.files {
        if !replace && export.existing.contains(&f.name) {
            continue;
        }
        let (path, data) = match &f.what {
            Exported::Resource(key) => (key.path_in(dir), resource_data(app, key)),
            Exported::Text(text) => (Ok(dir.join(&f.name)), Some(text.clone().into_bytes())),
        };
        let Some(data) = data else { continue };
        if let Err(e) = path.and_then(|path| std::fs::write(path, &data)) {
            return app.log.error(format!("Could not write to {}: {e}", dir.display()));
        }
        written.push(format!("{} ({})", f.name, f.origin));
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
    if !export.missing.is_empty() {
        app.log.warn(format!(
            "{} names what is found nowhere (the game draws it without them): {}",
            export.what,
            export.missing.join(", ")
        ));
    }
    app.export_dir = Some(dir.clone());
}

/// What Compile Models made, of the module's models or a hak's.
pub(crate) struct CompiledModels {
    /// Each model compiled, and its compiled bytes.
    pub made: Vec<(ResKey, Vec<u8>)>,
    /// Each left as text, and why.
    left: Vec<String>,
    /// How many notes the compiler made.
    notes: usize,
}

impl CompiledModels {
    /// Sorts Compile Models' results; `what` says whose they are ("the
    /// module", "the hak") in the log, where the compiler's notes go (three
    /// of each model's). `None`: nothing was kept as text.
    pub(crate) fn sort(
        log: &mut crate::Log,
        what: &str,
        results: Vec<mg_module::models::Compiled>,
    ) -> Option<CompiledModels> {
        let mut sorted = CompiledModels { made: Vec::new(), left: Vec::new(), notes: 0 };
        for c in results {
            match c.result {
                Ok((binary, said)) => {
                    sorted.notes += said.len();
                    for (line, note) in said.iter().take(3) {
                        let at = if *line > 0 { format!(" line {line}") } else { String::new() };
                        log.info(format!("{}{at}: {note}", c.model));
                    }
                    sorted.made.push((c.model, binary));
                }
                Err(why) => sorted.left.push(format!("{}: {why}", c.model)),
            }
        }
        if sorted.made.is_empty() && sorted.left.is_empty() {
            log.info(format!("Compile Models: {what} has no models kept as text"));
            return None;
        }
        Some(sorted)
    }

    /// Says in the log what was compiled and what was left as text (once
    /// the compiled models are in).
    pub(crate) fn say(&self, log: &mut crate::Log) {
        log.info(format!(
            "Compile Models: {} compiled ({} notes of the compiler's), {} left as text",
            self.made.len(),
            self.notes,
            self.left.len()
        ));
        const LISTED: usize = 12;
        for l in self.left.iter().take(LISTED) {
            log.warn(format!("Left as text: {l}"));
        }
        if self.left.len() > LISTED {
            log.warn(format!("…and {} more left as text", self.left.len() - LISTED));
        }
    }
}

/// Asks what to do with the files an Export Files would write over.
pub(crate) fn export_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(export) = app.model_export.clone() else { return };
    let (n, all) = (export.existing.len(), export.files.len());
    let said = format!("{} already has {n} of the {all} files:", export.dir.display());
    let files: Vec<_> = export.existing.iter().map(|k| (k.to_string(), None)).collect();
    let tips = [
        "Write all the files: these take the place of the folder's",
        "Write the other files and leave the folder's as they are",
    ];
    use crate::widgets::Already;
    let Some(answer) = crate::widgets::files_already(ctx, "Folder", &said, &files, tips) else {
        return;
    };
    app.model_export = None;
    match answer {
        Already::Replace => write_export(app, &export, true),
        Already::Skip => write_export(app, &export, false),
        Already::Cancel => {}
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
        // (A layer's copy of a model: its own, the rest as the game has
        // them.)
        let data = match source {
            Source::Copy { key, .. } if *key == k => source.data(app, &k),
            _ => None,
        };
        let data = data
            .or_else(|| app.ws.as_ref().and_then(|w| w.module.get(&k).map(<[u8]>::to_vec)))
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
        let model = source.is_model();
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
        let item = source.is_item();
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
        let pitch = 20f32.to_radians();
        let stand = if lying { stood_up(composed.as_ref(), yaw, pitch) } else { Mat4::IDENTITY };
        let mut view = ModelView {
            model,
            stand,
            supermodels: RefCell::new(HashMap::new()),
            animation,
            playing: true,
            model_lights: true,
            time: 0.0,
            // Models face +Y: from the front, a little to the side.
            orbit: Orbit { target: Vec3::ZERO, yaw, pitch, distance: 5.0 },
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

    /// How what it shows is stood for the viewer ([`ModelView::stood`]):
    /// an item modeled lying, its length up and its broad side to the
    /// viewer.
    pub fn stand(&self) -> Mat4 {
        self.stand
    }

    /// What it shows of a blueprint (none for a model).
    pub fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

    /// Fits the camera to the model as it stands in the animation it
    /// plays (a dragon at rest lies stretched out, wings spread, far out
    /// of the box around its pause).
    pub fn frame(&mut self) {
        if let Some(c) = &self.composed {
            let (low, high) = c.bounds_in(self.animation.as_deref(), self.time);
            // (The box around it as it is stood.)
            let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for p in mg_render::box_corners(low, high) {
                let w = self.stand.transform_point3(p);
                (min, max) = (min.min(w), max.max(w));
            }
            self.orbit.target = (min + max) * 0.5;
            let radius = ((max - min).length() * 0.5).max(0.1);
            self.orbit.distance = radius / 20f32.to_radians().sin() * 1.1;
            return;
        }
        let Ok(m) = &self.model else { return };
        let pose = mg_render::rest_pose(&m.model);
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for mesh in &m.meshes {
            for p in mg_render::box_corners(mesh.min, mesh.max) {
                let w = pose[mesh.node].transform_point3(p);
                min = min.min(w);
                max = max.max(w);
            }
        }
        if min.x > max.x {
            (min, max) = (Vec3::splat(-1.0), Vec3::splat(1.0));
        }
        self.orbit.target = (min + max) * 0.5;
        let radius = ((max - min).length() * 0.5).max(0.1);
        self.orbit.distance = radius / 20f32.to_radians().sin() * 1.1;
    }

    fn camera(&self) -> Camera {
        self.orbit.camera()
    }
}

/// How an item modeled lying (its length along the ground, +Y) is stood
/// on its end for a viewer at `yaw` and `pitch`, as its icon shows it: its
/// length up, and turned about it so that its broad side faces the viewer
/// (a sword its flat, not its edge). Of the quarter turns, the first unless
/// another shows half as much again of the base model's surface
/// ([`Composed::seen_area`]), as a picture's side is chosen.
fn stood_up(composed: Option<&Composed>, yaw: f32, pitch: f32) -> Mat4 {
    let up = Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let Some(c) = composed else { return up };
    let towards = Vec3::new(yaw.cos() * pitch.cos(), yaw.sin() * pitch.cos(), pitch.sin());
    let stood = |turn: f32| Mat4::from_rotation_z(turn) * up;
    let shows = |turn: f32| c.seen_area(stood(turn).inverse().transform_vector3(towards));
    let quarter = std::f32::consts::FRAC_PI_2;
    let turn = [quarter, -quarter, 2.0 * quarter]
        .into_iter()
        .fold((0.0, shows(0.0)), |best, turn| {
            let s = shows(turn);
            if s > best.1 * 1.1 + 1e-6 { (turn, s) } else { best }
        })
        .0;
    stood(turn)
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
        let mut view = ModelView::open(app, &source);
        // (Its camera as a session left it, a tab opened again.)
        if !embedded
            && let Some((orbit, _)) = app.open_tabs.take_view(&source.tab()).and_then(|v| v.orbit())
        {
            view.orbit = orbit;
        }
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
        // (With Icons only, each glyph alone, its name in its tip.)
        let tools = crate::icons::Tools::of(&app.settings);
        use crate::icons::editor as glyph;
        let button = |ui: &mut egui::Ui, g: &str, name: &str, tip: &str| {
            let r = ui.button(tools.word(g, name));
            tools.named(r, name, tip, None)
        };
        let (g, name) = match view.playing {
            true => (glyph::PAUSE, "Pause"),
            false => (glyph::PLAY, "Play"),
        };
        if button(ui, g, name, "").clicked() {
            view.playing = !view.playing;
        }
        if button(ui, glyph::FRAME, "Frame", "Fit the model in the view").clicked() {
            view.frame();
        }
        let tip = "Light the model with its own light nodes";
        tools.checkbox_word(ui, &mut view.model_lights, glyph::LIGHTS, "Lights", tip);
        let mut shaders = app.game_shaders();
        if tools
            .checkbox_word(
                ui,
                &mut shaders,
                crate::icons::SHADERS,
                "Shaders",
                "Draw with the game's own shaders, compiled from the game and the module's haks: \
                 a material's custom shaders (swaying grass and leaves, flowing water) and the \
                 game's lit shaders; off, with Moonglow's own (in every 3D view)",
            )
            .changed()
        {
            app.set_game_shaders(shaders);
        }
        if embedded {
            let tip = "Show it in a window of its own";
            pop_out = button(ui, glyph::POP_OUT, "Pop Out", tip).clicked();
        }
        // (Not of an overridden copy: the files it would gather are those
        // the game reads, not the copy's layer's. Its Save As… is there.)
        let copy = matches!(source, Source::Copy { .. });
        export = !copy
            && button(
                ui,
                glyph::EXPORT,
                "Export Files…",
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
    let o = &mut view.orbit;
    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        o.yaw -= d.x * 0.01;
        o.pitch = (o.pitch + d.y * 0.01).clamp(-1.45, 1.45);
    }
    // (Across the screen and up it, by the pointer's points.)
    let pan = |o: &mut Orbit, d: egui::Vec2| {
        let cam = o.camera();
        let fwd = (cam.target - cam.eye).normalize();
        let right = fwd.cross(Vec3::Z).normalize_or_zero();
        let up = right.cross(fwd);
        o.target += (-right * d.x + up * d.y) * o.distance * 0.0015;
    };
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        pan(o, response.drag_delta());
    }
    // The wheel as the area view's: Shift or Ctrl zooms finer, a pinch
    // zooms, a sideways scroll pans.
    if response.hovered() {
        let wheel = Wheel::read(ui);
        if wheel.zoom != 0.0 {
            o.distance = wheel.zoomed(o.distance, (0.05, 5000.0));
        }
        if wheel.sideways != 0.0 {
            pan(o, egui::vec2(wheel.sideways, 0.0));
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
    /// The items pictured by their looks ([`Pictured::Item`]), whether on
    /// a woman, and for one of an armor's parts alone, its models' names
    /// ([`mg_preview::armor_part`]): those asked for and kept.
    items: HashMap<u64, (Struct, bool, Option<&'static [&'static str]>)>,
    /// The creatures pictured as they are in other appearances
    /// ([`Pictured::Creature`]): those asked for and kept.
    creatures: HashMap<u64, Struct>,
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
    /// What reads pictures' models apart ([`make_thumbnail`]), made with
    /// the first it reads.
    reader: Option<PictureReader>,
    /// The pictures whose models are being read apart, and those read,
    /// waiting to be drawn (for their textures, read apart too).
    loading: HashMap<Kept, Loading>,
    ready: HashMap<Kept, Ready>,
    /// Bumped as pictures are let go of: what was being read before is
    /// dropped as it comes.
    era: u64,
    /// The module's models as they were at a revision, which pictures read
    /// apart are read from.
    #[allow(clippy::type_complexity)]
    module_models: Option<(Option<u64>, Arc<HashMap<ResKey, Arc<[u8]>>>)>,
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
    /// A blueprint with nothing to draw because its appearance has no
    /// model ([`mg_rules::appearance`]): the palette's Gallery marks it.
    no_model: bool,
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
    /// A creature as its fields make it look (its head, colors, what it
    /// wears), by those fields ([`Thumbnails::creature`]), in another
    /// appearance (an appearance.2da row), for choosing among them.
    Creature(u64, u16),
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

    /// `key`'s picture, as this frame shows it (of an item blueprint, the
    /// one of the body armor is shown on): `None` not made yet,
    /// `Some(None)` made with nothing to draw. Looked up, where the
    /// galleries made a map of every picture each frame.
    pub(crate) fn made(&self, key: Pictured) -> Option<Option<egui::TextureId>> {
        self.made.get(&self.kept_as(key)).map(Made::id)
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
    pub(crate) fn item(&mut self, item: &Struct, female: bool) -> Pictured {
        self.item_part(item, female, None)
    }

    /// What pictures one of armor `item`'s parts alone (`pieces`, its
    /// models' names: [`mg_preview::armor_part`]), on a woman where
    /// `female`; with no `pieces`, the whole item ([`Thumbnails::item`]).
    pub(crate) fn item_part(
        &mut self,
        item: &Struct,
        female: bool,
        pieces: Option<&'static [&'static str]>,
    ) -> Pictured {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        // (Its fields as written out: a struct has no hash of its own.)
        format!("{item:?}").hash(&mut h);
        female.hash(&mut h);
        pieces.hash(&mut h);
        let id = h.finish();
        self.items.entry(id).or_insert_with(|| (item.clone(), female, pieces));
        Pictured::Item(id)
    }

    /// What pictures `creature` (a creature's fields) as it is in another
    /// appearance: [`Pictured::Creature`] with this and the row.
    pub(crate) fn creature(&mut self, creature: &Struct) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{creature:?}").hash(&mut h);
        let id = h.finish();
        self.creatures.entry(id).or_insert_with(|| creature.clone());
        id
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

    /// Whether `key`'s picture is made with nothing to draw because the
    /// blueprint's appearance has no model.
    pub(crate) fn no_model(&self, key: Pictured) -> bool {
        self.made.get(&self.kept_as(key)).is_some_and(|m| m.no_model)
    }

    /// [`no_model`](Self::no_model) of blueprint `key`'s picture.
    pub fn blueprint_has_no_model(&self, key: ResKey) -> bool {
        self.no_model(Pictured::Resource(key))
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
        self.let_go();
        self.ready.clear();
        self.module_models = None;
    }

    /// How many pictures are kept.
    pub fn kept(&self) -> usize {
        self.made.len()
    }

    /// How many pictures asked for this frame are still to make.
    pub fn waiting(&self) -> usize {
        self.wanted.len()
    }

    /// How many pictures are being read apart, or wait to be drawn for
    /// their textures ([`make_thumbnail`]).
    pub fn reading(&self) -> usize {
        self.loading.len() + self.ready.len()
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
    app.thumbnails.take_read();
    // (Read apart and no longer asked for: let go of.)
    let asked: std::collections::HashSet<Kept> = wanted.iter().copied().collect();
    app.thumbnails.ready.retain(|k, _| asked.contains(k));
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
    // (Being read apart: looked for again soon.)
    if !app.thumbnails.loading.is_empty() || !app.thumbnails.ready.is_empty() {
        ctx.request_repaint_after(std::time::Duration::from_millis(8));
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
    if !thumbnails.creatures.is_empty() {
        let mut used = std::collections::HashSet::new();
        for (key, _) in thumbnails.made.keys().chain(&thumbnails.wanted) {
            if let Pictured::Creature(id, _) = key {
                used.insert(*id);
            }
        }
        thumbnails.creatures.retain(|id, _| used.contains(id));
    }
}

/// Makes a picture, or finds the one made before the module changed to be
/// what the blueprint still looks like. Where the window keeps going
/// ([`Moonglow::background_jobs`]), a picture's models are read apart
/// ([`PictureReader`]) and its textures too: it is drawn in a later frame,
/// once they are (a model with large textures took a frame of 7 to 9 ms,
/// read on the window's thread). Else it is made at once.
fn make_thumbnail(app: &mut Moonglow, kept: Kept) {
    let (frame, revision) = (app.thumbnails.frame, app.thumbnails.revision);
    let key = kept.0;
    // Its models read apart: drawn once its textures are read too.
    if let Some(ready) = app.thumbnails.ready.remove(&kept) {
        let drawn = match &ready.subject {
            Some((source, preview)) => match ready.composed.first() {
                Some((composed, _)) => draw_composed(app, key, source, preview, composed, true),
                None => Ok(None),
            },
            None => draw_group(app, &ready.composed, true),
        };
        match drawn {
            Err(Waiting) => _ = app.thumbnails.ready.insert(kept, ready),
            Ok(picture) => {
                let made = Made {
                    picture,
                    look: ready.look,
                    revision: ready.revision,
                    used: frame,
                    no_model: false,
                };
                keep_made(app, kept, made);
            }
        }
        return;
    }
    // (Being read.)
    if app.thumbnails.loading.contains_key(&kept) {
        return;
    }
    let apart = app.background_jobs && app.viewport.is_some() && app.game.is_some();
    if let Pictured::Group(tileset, group) = key {
        if apart {
            let tiles = group_tiles(app, tileset, group).unwrap_or_default();
            let parts = tiles.iter().map(|(_, preview, at)| (preview.clone(), *at)).collect();
            let loading = Loading { subject: None, look: None, revision };
            read_apart(app, kept, parts, loading);
            return;
        }
        let picture = draw_group_thumbnail(app, tileset, group);
        keep_made(app, kept, Made { picture, look: None, revision, used: frame, no_model: false });
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
    let kept_look = look.as_ref().map(|(_, p)| p.clone()).filter(|_| key.follows_the_module());
    if apart && let Some((source, preview)) = look {
        let parts = vec![(preview.clone(), Mat4::IDENTITY)];
        let loading = Loading { subject: Some((source, preview)), look: kept_look, revision };
        read_apart(app, kept, parts, loading);
        return;
    }
    let picture =
        look.as_ref().and_then(|(source, preview)| draw_thumbnail(app, key, source, preview));
    // (Nothing to draw: whether for an appearance with no model.)
    let no_model = look.is_none() && has_no_model(app, key);
    keep_made(app, kept, Made { picture, look: kept_look, revision, used: frame, no_model });
}

/// Whether `key` is a blueprint whose appearance has no model to draw
/// (as its Properties and Verify judge it).
fn has_no_model(app: &Moonglow, key: Pictured) -> bool {
    let Pictured::Resource(k) = key else { return false };
    let Some(kind) = mg_rules::ObjectKind::from_restype(k.restype) else { return false };
    let (Some(game), Some(gff)) = (app.game.as_deref(), crate::area_tools::blueprint(app, k))
    else {
        return false;
    };
    // (Not a part's model: the rest of the object is drawn.)
    game.appearance_problem(kind, &gff.root).is_some_and(|p| !p.problem.is_part())
}

/// Keeps a picture made, letting go of the one it takes the place of.
fn keep_made(app: &mut Moonglow, kept: Kept, made: Made) {
    app.thumbnails.drawn += 1;
    if let Some(old) = app.thumbnails.made.insert(kept, made)
        && let (Some((_, id)), Some(vp)) = (old.picture, &app.viewport)
    {
        vp.render_state.renderer.write().free_texture(&id);
    }
}

/// Asks for a picture's models (`parts`: each preview and where it stands)
/// to be read apart, from the module's models as they are now and the
/// game's.
fn read_apart(app: &mut Moonglow, kept: Kept, parts: Vec<(Preview, Mat4)>, loading: Loading) {
    let (Some(vp), Some(game)) = (&app.viewport, &app.game) else { return };
    let thumbnails = &mut app.thumbnails;
    let revision = thumbnails.revision;
    let module = match &thumbnails.module_models {
        Some((at, models)) if *at == revision => models.clone(),
        _ => {
            let models: HashMap<ResKey, Arc<[u8]>> = (app.ws.as_ref())
                .map(|ws| {
                    (ws.module.keys_of(ResType::MDL))
                        .filter_map(|k| Some((*k, ws.module.shared(k)?.clone())))
                        .collect()
                })
                .unwrap_or_default();
            let models = Arc::new(models);
            thumbnails.module_models = Some((revision, models.clone()));
            models
        }
    };
    let reader = thumbnails.reader.get_or_insert_with(|| PictureReader::new(vp.gpu.clone()));
    let era = thumbnails.era;
    reader.ask(PictureJob { kept, era, parts, module, game: game.clone() });
    thumbnails.loading.insert(kept, loading);
}

/// A picture whose models are being read apart ([`Thumbnails::loading`]):
/// what it is of (none for a tileset's group) and what is kept with it.
struct Loading {
    subject: Option<(Source, Preview)>,
    look: Option<Preview>,
    revision: Option<u64>,
}

/// A picture whose models are read, drawn once its textures are
/// ([`Thumbnails::ready`]).
struct Ready {
    subject: Option<(Source, Preview)>,
    composed: Vec<(Composed, Mat4)>,
    look: Option<Preview>,
    revision: Option<u64>,
}

/// What a [`PictureReader`] is asked to read: a picture's previews' models,
/// from the module's (as they were when asked) and the game's.
struct PictureJob {
    kept: Kept,
    era: u64,
    parts: Vec<(Preview, Mat4)>,
    module: Arc<HashMap<ResKey, Arc<[u8]>>>,
    game: Arc<mg_rules::GameData>,
}

/// What it read: each part's models on the GPU (`None`: not found or not
/// readable), where it stands.
struct PictureDone {
    kept: Kept,
    era: u64,
    composed: Vec<(Option<Composed>, Mat4)>,
}

/// How many threads read pictures' models apart.
const PICTURE_READERS: usize = 2;

/// Threads that read pictures' models apart from the window (a gallery's:
/// [`make_thumbnail`]), uploading them, as the renderer reads textures
/// apart. The jobs waiting are let go of with the game data they hold
/// ([`Thumbnails::let_go`]); the threads end with it.
struct PictureReader {
    queue: Arc<(std::sync::Mutex<PictureQueue>, std::sync::Condvar)>,
    done: std::sync::mpsc::Receiver<PictureDone>,
}

#[derive(Default)]
struct PictureQueue {
    jobs: std::collections::VecDeque<PictureJob>,
    stop: bool,
}

impl PictureReader {
    fn new(gpu: Gpu) -> PictureReader {
        let queue: Arc<(std::sync::Mutex<PictureQueue>, std::sync::Condvar)> = Default::default();
        let (give, done) = std::sync::mpsc::channel();
        for n in 0..PICTURE_READERS {
            let (queue, give, gpu) = (queue.clone(), give.clone(), gpu.clone());
            let read = move || {
                loop {
                    let job = {
                        let (lock, ready) = &*queue;
                        let Ok(mut q) = lock.lock() else { return };
                        loop {
                            if q.stop {
                                return;
                            }
                            if let Some(job) = q.jobs.pop_front() {
                                break job;
                            }
                            q = match ready.wait(q) {
                                Ok(q) => q,
                                Err(_) => return,
                            };
                        }
                    };
                    let composed = read_picture(&gpu, &job);
                    let done = PictureDone { kept: job.kept, era: job.era, composed };
                    // (The game data let go of before it is said.)
                    drop(job);
                    if give.send(done).is_err() {
                        return;
                    }
                }
            };
            let name = format!("moonglow pictures {n}");
            // (No thread: the jobs wait; the window reads none apart.)
            let _ = std::thread::Builder::new().name(name).spawn(read);
        }
        PictureReader { queue, done }
    }

    fn ask(&self, job: PictureJob) {
        let (lock, ready) = &*self.queue;
        if let Ok(mut q) = lock.lock() {
            q.jobs.push_back(job);
            ready.notify_one();
        }
    }

    /// Lets go of the jobs waiting (and the game data they hold).
    fn clear(&self) {
        if let Ok(mut q) = self.queue.0.lock() {
            q.jobs.clear();
        }
    }
}

impl Drop for PictureReader {
    fn drop(&mut self) {
        let (lock, ready) = &*self.queue;
        if let Ok(mut q) = lock.lock() {
            q.jobs.clear();
            q.stop = true;
        }
        ready.notify_all();
    }
}

/// A job's models read and uploaded, as [`compose`] reads them.
fn read_picture(gpu: &Gpu, job: &PictureJob) -> Vec<(Option<Composed>, Mat4)> {
    let load = |name: &str| -> Option<Arc<Model>> {
        let k = ResKey::parse(name, ResType::MDL)?;
        let model = match job.module.get(&k) {
            Some(data) => Model::read(data),
            None => Model::read(&job.game.resman.get(&k).ok()?),
        };
        model.ok().map(Arc::new)
    };
    (job.parts.iter()).map(|(preview, at)| (Composed::new(gpu, preview, &load), *at)).collect()
}

impl Thumbnails {
    /// Takes the pictures whose models were read apart since the last
    /// frame: each to be drawn once its textures are.
    fn take_read(&mut self) {
        let Some(reader) = &self.reader else { return };
        while let Ok(done) = reader.done.try_recv() {
            if done.era != self.era {
                continue;
            }
            let Some(loading) = self.loading.remove(&done.kept) else { continue };
            let composed = done.composed.into_iter().filter_map(|(c, at)| Some((c?, at))).collect();
            let Loading { subject, look, revision } = loading;
            self.ready.insert(done.kept, Ready { subject, composed, look, revision });
        }
    }

    /// Lets go of what reads pictures apart holds of the game data, to
    /// have it changed ([`Moonglow::release_game_data`]): the jobs waiting
    /// (asked for again by the views that want them).
    pub(crate) fn let_go(&mut self) {
        if let Some(reader) = &self.reader {
            reader.clear();
        }
        self.era += 1;
        self.loading.clear();
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
            let (item, female, pieces) = app.thumbnails.items.get(&id)?;
            let game = app.game.as_deref()?;
            let preview = match pieces {
                Some(pieces) => mg_preview::armor_part(game, item, *female, pieces).ok()?,
                None => mg_preview::item_on(game, item, *female).ok()?,
            };
            // (Named as an item's, which tells a shield's from the others'.)
            let source = Source::Resource(ResKey::parse("item", ResType::UTI)?);
            Some((source, mg_preview::replaced(preview, item)))
        }
        Pictured::Creature(id, row) => {
            let mut utc = app.thumbnails.creatures.get(&id)?.clone();
            utc.set("Appearance_Type", mg_gff::Value::Word(row));
            let game = app.game.as_deref()?;
            let read = |k: &ResKey| resource_data(app, k).and_then(|d| Gff::read(&d).ok());
            let items = |r: ResRef| read(&ResKey::new(r, ResType::UTI));
            let kind = mg_rules::ObjectKind::Creature;
            let shown = mg_preview::Shown::default();
            let preview = mg_preview::of_object(game, kind, &utc, &items, shown)?.ok()?;
            Some((Source::Resource(ResKey::parse("appearance", ResType::TWODA)?), preview))
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
    let item = source.is_item();
    let t = app.game.as_deref().filter(|_| item)?.table("baseitems").ok()?;
    let class = preview.base.model.split('_').next().unwrap_or_default();
    (0..t.len())
        .filter(|row| t.get(*row, "ItemClass").is_some_and(|c| c.eq_ignore_ascii_case(class)))
        .find_map(|row| t.get_int(row, "EquipableSlots"))
}

/// A picture drawn, or not drawn yet: a texture it needs is being read
/// apart ([`mg_render::Renderer::read_textures_apart`]).
type Drawn = Result<Option<(wgpu::Texture, egui::TextureId)>, Waiting>;

/// A picture waits for its textures.
#[derive(Debug, Clone, Copy)]
struct Waiting;

fn draw_thumbnail(
    app: &mut Moonglow,
    key: Pictured,
    source: &Source,
    preview: &Preview,
) -> Option<(wgpu::Texture, egui::TextureId)> {
    let composed = compose(app, source, preview).ok()?;
    draw_composed(app, key, source, preview, &composed, false).ok().flatten()
}

/// [`draw_thumbnail`] of a preview's models read already; `apart`: its
/// textures met for the first time are read apart, and it is not drawn
/// until they are.
fn draw_composed(
    app: &mut Moonglow,
    key: Pictured,
    source: &Source,
    preview: &Preview,
    composed: &Composed,
    apart: bool,
) -> Drawn {
    // (A tile is seen from one side always, turned as it lies.)
    let turned = match key {
        Pictured::Tile(_, quarters) => Some(f32::from(quarters % 4) * std::f32::consts::FRAC_PI_2),
        _ => None,
    };
    let shield = is_shield(app, source, preview);
    // (Drawn with the GPU: none, no picture.)
    if app.viewport.is_none() {
        return Ok(None);
    }
    // As the viewer frames it: from the front, a little to the side.
    // A model that emits (flames, sparks, a shaft of light: some are
    // nothing else) is drawn a few seconds in, its particles in the frame.
    let idle = composed.idle.as_deref();
    let mut points = composed.points_in(&composed.stance(idle, 0.0));
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
        points.extend(particles.iter().flat_map(|b| &b.vertices).map(|v| Vec3::from(v.pos)));
    }
    // Framed tightly on what it shows, as the picture sees it: a dragon,
    // long and winged, filled a corner of its picture framed by a ball
    // around its box. (A creature stands in its animation, which moves it
    // from where it stands at its start: it is given more room.)
    let room = if idle.is_some() { 1.15 } else { 1.06 };
    let fov = Camera::orbit(Vec3::ZERO, 1.0, yaw, pitch).fov_y;
    let corners = || {
        let (min, max) = composed.bounds_in(idle, 0.0);
        (0..8).map(move |c| {
            let pick = |bit: i32, axis: usize| if c & bit == 0 { min[axis] } else { max[axis] };
            Vec3::new(pick(1, 0), pick(2, 1), pick(4, 2))
        })
    };
    // (Nothing drawn, an emitter that has thrown nothing yet: its box.)
    if points.is_empty() {
        points.extend(corners());
    }
    let (target, distance) =
        mg_preview::compose::framed_on(&points, (yaw, pitch), fov, 1.0).expect("some points");
    let camera = Camera::orbit(target, (distance * room).max(0.1), yaw, pitch);
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
    render_picture(app, &scene, &camera, apart)
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
    let tiles = group_tiles(app, tileset, group)?;
    let parts: Vec<(Composed, Mat4)> = (tiles.iter())
        .filter_map(|(source, preview, at)| Some((compose(app, source, preview).ok()?, *at)))
        .collect();
    draw_group(app, &parts, false).ok().flatten()
}

/// The tiles a group's picture shows: each tile's model, as a source and
/// its preview, and where it lies ([`draw_group_thumbnail`]).
fn group_tiles(
    app: &Moonglow,
    tileset: mg_core::ResRef,
    group: u32,
) -> Option<Vec<(Source, Preview, Mat4)>> {
    let set = app.tileset(tileset)?;
    let g = set.groups.get(group as usize)?;
    let index = (app.open_tileset(tileset).map(|t| t.index.clone()))
        .unwrap_or_else(|| mg_tiles::TileIndex::new(&set));
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
    let mut tiles = Vec::new();
    for ((c, r), p) in placed {
        let Some(t) = set.tiles.get(p.tile as usize) else { continue };
        let Some(key) = ResKey::parse(&t.model.to_ascii_lowercase(), ResType::MDL) else {
            continue;
        };
        let source = Source::Resource(key);
        let Ok(preview) = preview_of(app, &source) else { continue };
        let at = Vec3::new(
            c as f32 * mg_area::TILE_SIZE,
            r as f32 * mg_area::TILE_SIZE,
            p.height as f32 * set.general.transition,
        );
        let turn =
            glam::Quat::from_rotation_z(f32::from(p.orientation % 4) * std::f32::consts::FRAC_PI_2);
        tiles.push((source, preview, Mat4::from_rotation_translation(turn, at)));
    }
    Some(tiles)
}

/// A group's picture of its tiles' models read already, each where it
/// lies; `apart` as [`draw_composed`]'s. `Ok(None)` where there are none.
fn draw_group(app: &mut Moonglow, parts: &[(Composed, Mat4)], apart: bool) -> Drawn {
    if parts.is_empty() {
        return Ok(None);
    }
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let (mut instances, mut lights) = (Vec::new(), Vec::new());
    for (composed, at) in parts {
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
    render_picture(app, &scene, &camera, apart)
}

/// A picture of `scene` from `camera`, in a texture of its own, and egui's
/// name for it: a thumbnail. `apart`: textures met for the first time are
/// read apart from the window ([`mg_render::Renderer::read_textures_apart`])
/// and the picture waits for them (`Err`); else they are read as they are
/// met.
fn render_picture(app: &mut Moonglow, scene: &Scene, camera: &Camera, apart: bool) -> Drawn {
    let game = app.game.clone();
    let Some(vp) = app.viewport.as_mut() else { return Ok(None) };
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
    let resman = game.as_deref().map(|g| &g.resman);
    let assets: &dyn mg_render::Assets = match resman {
        Some(rm) => rm,
        None => &mg_render::NoAssets,
    };
    // (As the area view reads them apart: from the game data, which it
    // lets go of to have it changed, `Viewport3d::release_game_data`.)
    let apart = apart && game.is_some();
    if let Some(game) = game.as_ref().filter(|_| apart) {
        let from = vp.textures_from(game);
        vp.renderer.read_textures_apart(&vp.gpu, Some(from));
    }
    vp.renderer.textures_apart = apart;
    // (And the game's programs a picture is the first to draw with made
    // apart too, the picture waiting for them: made in the frame, one
    // took 13 to 27 ms.)
    vp.renderer.game_background = apart;
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
    vp.renderer.textures_apart = false;
    vp.renderer.game_background = false;
    let drawn = &vp.renderer.drawn;
    if apart && (drawn.textures_pending > 0 || drawn.game_pending > 0) {
        return Err(Waiting);
    }
    let id = vp.render_state.renderer.write().register_native_texture(
        &vp.gpu.device,
        &view,
        wgpu::FilterMode::Linear,
    );
    Ok(Some((picture, id)))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A creature pictured in another appearance (its Appearance list,
    /// the Appearance Gallery) wears what it wears: the bandit's armor
    /// (torso 28) and longsword on a half-elf's body, not the bare body of
    /// a plain look.
    #[test]
    fn a_creature_in_another_appearance_wears_its_armor() {
        let Some(root) = mg_testkit::nwn_root() else {
            eprintln!("skipped: no game install");
            return;
        };
        let install = mg_resman::GameInstall::new(&root, None, "en");
        let mut app = Moonglow::new(Some(install), Box::new(crate::NoDialogs::default()));
        let game = app.game.clone().unwrap();
        let data = game.resman.get(&ResKey::parse("nw_bandit001", ResType::UTC).unwrap()).unwrap();
        let bandit = Gff::read(&data).unwrap().root;
        let id = app.thumbnails.creature(&bandit);
        // (appearance.2da row 4: a half-elf, a body of parts.)
        let (_, dressed) = pictured_look(&app, (Pictured::Creature(id, 4), false)).unwrap();
        let models: Vec<&str> = dressed.parts.iter().map(|p| p.model.as_str()).collect();
        assert!(models.iter().any(|m| m.ends_with("_chest028")), "the armor's torso: {models:?}");
        assert!(models.iter().any(|m| m.starts_with("wswls_")), "the sword: {models:?}");
        let (_, bare) =
            pictured_look(&app, (Pictured::Look(mg_preview::CreatureLook::new(4)), false)).unwrap();
        assert!(bare.parts.iter().any(|p| p.model.ends_with("_chest001")), "a plain body's own");
        assert_ne!(dressed, bare);
    }
}
