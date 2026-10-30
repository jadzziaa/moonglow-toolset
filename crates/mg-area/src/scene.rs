//! An area's models on the GPU and the renderer's scene at a moment.
//!
//! Tiles play `tiledefault`, then `day` or `night`, then each animation
//! loop the ARE enables, together (a later one's keys win); their main
//! lights take lightcolor.2da colours, and their source lights are
//! `fx_flame01` playing the animation named by the value, at the tile's
//! `sl1`/`sl2` node. Objects stand in their previews' idle animation.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_core::ResType;
use mg_mdl::{Animation, Model};
use mg_preview::compose::Composed;
use mg_render::anim;
use mg_render::scene::{AreaLight, Fog, Instance, PointLight, Scene};
use mg_render::{Gpu, GpuModel};
use mg_rules::GameData;

use crate::grass::GrassLook;
pub use crate::grass_view::Sight;
use crate::grass_view::{ChunkGrass, GrassDraw, Grown, MadeOf, TileGrass};
use crate::walk::Walkmesh;
use crate::{AreaModel, AreaObject, AreaTile, Lighting, ObjectKind, TILE_SIZE};

/// How many objects' particles are simulated in a frame at most (an area
/// of hundreds of torches still draws at once): those nearest the view.
const MAX_PARTICLE_OBJECTS: usize = 256;

/// A flat model no larger than this across (metres) has nothing to see or
/// to click (the game's `dag_invisible`, a square of 0.2 m lying flat):
/// its object is drawn and picked by a marker's box.
const BARE: f32 = 0.5;

/// How thin a model is that lies flat, metres.
const FLAT: f32 = 0.01;

/// Whether a model's box is that of nothing to see: flat and [`BARE`]
/// across at most, or no box at all (a model without a vertex). A small
/// model with some height is something to see: a potion, a gem, a
/// healer's kit is picked by its own box (it was by a marker's, a metre
/// wide, a builder's report).
fn bare((min, max): (Vec3, Vec3)) -> bool {
    let across = max - min;
    !across.is_finite() || across.min_element() < 0.0 || (across.length() < BARE && across.z < FLAT)
}

/// An emitter new to the view is run ahead this far, in steps this long,
/// so that it shows as it does once going: a wide mist lets out a few
/// slow particles a second that live a quarter of a minute, and showed
/// next to nothing for as long after every change to the area.
const PARTICLES_AHEAD: f32 = 20.0;
const PARTICLES_AHEAD_STEP: f32 = 0.25;

/// And how many tiles' (the nearest the view).
const MAX_PARTICLE_TILES: usize = 128;

/// Of `places` (each an index and where it is), the indices of the `most`
/// nearest `eye`, nearest first: the emitters given particles.
fn nearest(places: impl Iterator<Item = (usize, Vec3)>, eye: Vec3, most: usize) -> Vec<usize> {
    let mut by: Vec<(f32, usize)> = places.map(|(i, p)| (p.distance_squared(eye), i)).collect();
    // (Ties by index: the same ones whatever order they came in.)
    by.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    by.truncate(most);
    by.into_iter().map(|(_, i)| i).collect()
}

/// How to show the area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Seconds, for animations.
    pub time: f32,
    pub night: bool,
    pub fog: bool,
    /// Lit by the area's lighting and its lights; else by an even working
    /// light, in which a dark area shows (Aurora's lighting switched off).
    pub lit: bool,
    /// Which kinds of object are drawn, by [`ObjectKind::index`].
    pub show: [bool; 9],
    /// Placed objects play their animations (a creature's idle loop);
    /// else they hold still, as at their start. Tiles animate either way.
    pub animate: bool,
    /// Tiles are drawn without the parts that fade in the game to show a
    /// character behind them (roofs, upper walls: meshes with `tilefade`
    /// 1, and 4, which fade with a neighbouring tile's; those with 2, the
    /// black caps under them, stay), as Aurora's Environment › Fade
    /// Geometry leaves them out. (The game's tiles have 0, 1, 2 and 4.)
    pub fade: bool,
    /// Grass is drawn wherever this camera sees it ([`Sight`]); `None`:
    /// none.
    pub grass: Option<Sight>,
    /// The sun's or moon's shadows are drawn where the area has them
    /// ([`View::shadows_of`]): cast by tiles, placeables and doors.
    pub shadows: bool,
    /// Creatures cast the sun's or moon's shadows too.
    pub creature_shadows: bool,
    /// Point lights flagged `shadow` cast shadows (`mg_render::LightShadows`).
    pub light_shadows: bool,
}

impl View {
    /// The area as the toolset first shows it (without fog, as Aurora's
    /// Scene › Fog starts unchecked).
    pub fn of(area: &AreaModel) -> View {
        View {
            time: 0.0,
            night: area.lighting.night_by_default(),
            fog: false,
            lit: true,
            show: [true; 9],
            animate: true,
            fade: false,
            grass: None,
            shadows: true,
            creature_shadows: true,
            light_shadows: true,
        }
    }

    /// The sun's or moon's shadows an area lit as `l` shows in this view (cast
    /// by what [`View::shadows`] and [`View::creature_shadows`] say): the sun's
    /// by day, if its Sun Shadows are on, the moon's at night, if its Moon
    /// Shadows are (the client draws none for the other, read from its
    /// pictures: `shadows_look` in `client_render.rs`), as opaque as its
    /// Shadow Opacity; none with the lighting off, or with the view's
    /// shadows switched off.
    pub fn shadows_of(&self, l: &Lighting) -> Option<mg_render::SunShadows> {
        let casting = self.shadows || self.creature_shadows;
        (casting && self.lit && l.sky(self.night).shadows && l.shadow_opacity > 0).then(|| {
            mg_render::SunShadows {
                opacity: f32::from(l.shadow_opacity) / 100.0,
                fade_limit: l.fog_clip,
            }
        })
    }

    /// The lights' shadows an area lit as `l` shows in this view: where
    /// [`View::light_shadows`] asks for them and the area has no sun or moon
    /// shadows at the time (as the client: each creature then casts one
    /// from the nearest light that casts them, `mg_render::PointLight::casts_shadows`;
    /// with the sun's or moon's, it casts that alone), as opaque as its
    /// Shadow Opacity (measured in `shadows_look`).
    pub fn light_shadows_of(&self, l: &Lighting) -> Option<mg_render::SunShadows> {
        let sky = l.sky(self.night).shadows;
        (self.light_shadows && self.lit && !sky && l.shadow_opacity > 0).then(|| {
            mg_render::SunShadows {
                opacity: f32::from(l.shadow_opacity) / 100.0,
                fade_limit: l.fog_clip,
            }
        })
    }

    pub fn shows(&self, kind: ObjectKind) -> bool {
        self.show[kind.index()]
    }
}

/// The animations a tile plays, by name: [`TILE_ANIMATIONS`]'s indices.
const TILE_ANIMATIONS: [&str; 6] =
    ["tiledefault", "day", "night", "animloop01", "animloop02", "animloop03"];

/// A model with the animations it can play (its own and its supermodels'),
/// bound to its nodes when it is loaded: posing it looks up no names.
#[derive(Debug)]
struct Loaded {
    gpu: Arc<GpuModel>,
    anims: Vec<(String, Arc<Model>)>,
    /// Each of `anims`: where it is in its owner's animations, and its
    /// binding to this model's nodes.
    bound: Vec<Option<(usize, anim::Binding)>>,
    /// The animations of [`TILE_ANIMATIONS`], by their places in `anims`.
    tile: [Option<usize>; 6],
    /// Its source lights' nodes (`…sl1`, `…sl2`).
    sources: [Option<usize>; 2],
}

impl Loaded {
    fn new(gpu: Arc<GpuModel>, anims: Vec<(String, Arc<Model>)>) -> Loaded {
        let model = &gpu.model;
        let at: Vec<Option<usize>> = (anims.iter())
            .map(|(name, owner)| {
                owner.animations.iter().position(|a| a.name.eq_ignore_ascii_case(name))
            })
            .collect();
        let found = anims.iter().zip(&at).filter_map(|((_, owner), i)| owner.animations.get((*i)?));
        let mut bindings = anim::Binding::all(model, found).into_iter();
        let bound = at.iter().map(|i| Some(((*i)?, bindings.next()?))).collect();
        let tile =
            TILE_ANIMATIONS.map(|t| anims.iter().position(|(n, _)| n.eq_ignore_ascii_case(t)));
        let sources = [1, 2].map(|slot| {
            let suffix = format!("sl{slot}");
            model.nodes.iter().position(|n| n.name.to_ascii_lowercase().ends_with(&suffix))
        });
        Loaded { gpu, anims, bound, tile, sources }
    }

    /// The animation at `k` in `anims`, with its binding.
    fn bound_at(&self, k: usize) -> Option<(&Animation, &anim::Binding)> {
        let (i, binding) = self.bound.get(k)?.as_ref()?;
        Some((self.anims[k].1.animations.get(*i)?, binding))
    }

    fn animation(&self, name: &str) -> Option<&Animation> {
        let k = self.anims.iter().position(|(n, _)| n.eq_ignore_ascii_case(name))?;
        self.bound_at(k).map(|(a, _)| a)
    }
}

/// What the tiles of one model playing the same animations share at a
/// moment: its pose (`None`: at rest), what the animations change in its
/// meshes (and Fade Geometry leaves out), its lights before they are
/// placed, and its source lights' nodes (`sl1`, `sl2`).
#[derive(Debug)]
struct PosedTile {
    pose: Option<Arc<Vec<Mat4>>>,
    state: Option<Arc<mg_render::scene::MeshState>>,
    lamps: Vec<anim::Lamp>,
    sources: [Option<usize>; 2],
}

/// A source light's flame at a moment: its pose and its lights.
#[derive(Debug)]
struct PosedFlame {
    pose: Arc<Vec<Mat4>>,
    lamps: Vec<anim::Lamp>,
}

/// What a scene's tiles share (a view at a moment): an area's tiles are
/// of few models, and each tile posed for itself was most of what making
/// a large area's scene cost. Tiles by their model and the animation
/// loops they play; flames by their kind (`None`: there is no such flame).
#[derive(Debug, Default)]
struct Shared {
    tiles: HashMap<(usize, [bool; 3]), Arc<PosedTile>>,
    flames: HashMap<u8, Option<Arc<PosedFlame>>>,
}

/// An object's models and their bounds (as it stands: at the start of
/// the animation it is shown in, its rest pose without one; the object's
/// own space).
#[derive(Debug)]
struct Shown {
    composed: Composed,
    bounds: (Vec3, Vec3),
    /// How it stands in the area (its pause, at its start): what its box
    /// is around, and what a click is tried against.
    stance: mg_preview::compose::Stance,
}

/// What an area's loaded models use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub tile_models: usize,
    pub object_models: usize,
    pub meshes: usize,
    pub triangles: usize,
    pub buffer_bytes: u64,
    pub textures: usize,
}

/// An area's models, loaded. Models are kept by name (tiles) and by
/// preview (objects), so that updating the scene after an edit loads only
/// what is new.
#[derive(Debug, Default)]
pub struct AreaScene {
    /// By tile index.
    tiles: Vec<Option<Arc<Loaded>>>,
    /// By object (as in [`AreaModel::objects`]).
    objects: Vec<Option<Arc<Shown>>>,
    tile_cache: HashMap<String, Option<Arc<Loaded>>>,
    object_cache: HashMap<String, Option<Arc<Shown>>>,
    flame: Option<Option<Loaded>>,
    /// The skybox's day and night models, and its skyboxes.2da row.
    sky: [Option<Arc<GpuModel>>; 2],
    sky_box: Option<u32>,
    /// The game's `skyfade1`, with a sky.
    sky_fade: Option<Arc<GpuModel>>,
    /// lightcolor.2da colours by row.
    colors: Vec<Vec3>,
    /// Models named by tiles or previews that could not be loaded.
    pub missing: Vec<String>,
    /// Of those, the tiles' models that were read and then broke what
    /// builds them (a panic, caught): a renderer that fails every model
    /// fails them all, and they are what the area view's watch expects
    /// drawn all the same ([`AreaScene::failed_tiles`]).
    broken: std::collections::HashSet<String>,
    /// The loaded tiles and objects that failed while posed (by where they
    /// are kept): left out from then on.
    failed: RefCell<std::collections::HashSet<usize>>,
    /// Moonglow's arrow ([`crate::marker`]).
    arrow: Option<Arc<GpuModel>>,
    /// Stores as the game's marker for one (a $) rather than as arrows.
    pub merchant_signs: bool,
    /// The game's marker for an encounter's spawn point (`spawnpoint`).
    spawn_point: Option<Arc<GpuModel>>,
    /// Encounters' spawn points are marked (Options › Area).
    pub spawn_markers: bool,
    /// By tile index, the grass it grows ([`crate::grass::tufts`]).
    tile_grass: Vec<Option<Arc<TileGrass>>>,
    /// What grows (by material) that the grass was made for, and the
    /// grass of each tile model, by name.
    grass_for: Vec<(u32, GrassLook)>,
    grass_cache: HashMap<(String, u32), Option<Arc<TileGrass>>>,
    /// The grass of chunks far off, each in one mesh, by chunk
    /// ([`crate::grass_view`]).
    chunk_grass: HashMap<(i32, i32), ChunkGrass>,
    /// What puts them together.
    chunk_worker: Option<crate::grass_view::ChunkWorker>,
    /// The tile models' walkmeshes, by name.
    walkmesh_cache: HashMap<String, Option<Arc<Walkmesh>>>,
}

/// Loads models (and their supermodels) by name, each once.
struct Models<'a> {
    game: &'a GameData,
    cache: RefCell<HashMap<String, Option<Arc<Model>>>>,
}

impl Models<'_> {
    fn load(&self, name: &str) -> Option<Arc<Model>> {
        let name = name.to_ascii_lowercase();
        if let Some(m) = self.cache.borrow().get(&name) {
            return m.clone();
        }
        let m = self
            .game
            .resman
            .get_named(&name, ResType::MDL)
            .ok()
            .and_then(|d| Model::read(&d).ok())
            .map(Arc::new);
        self.cache.borrow_mut().insert(name, m.clone());
        m
    }
}

impl AreaScene {
    /// Loads the models of `area`'s tiles and objects.
    pub fn new(gpu: &Gpu, game: &GameData, area: &AreaModel) -> AreaScene {
        let mut scene = AreaScene { colors: light_colors(game), ..Default::default() };
        scene.update(gpu, game, area);
        let models = Models { game, cache: RefCell::new(HashMap::new()) };
        scene.load_sky(gpu, &models, game, area);
        scene
    }

    /// Loads the skybox's day and night models (skyboxes.2da), unless it
    /// is the one loaded.
    fn load_sky(&mut self, gpu: &Gpu, models: &Models<'_>, game: &GameData, area: &AreaModel) {
        if self.sky_box == Some(area.lighting.sky_box) {
            return;
        }
        self.sky_box = Some(area.lighting.sky_box);
        self.sky = [false, true].map(|night| {
            area.lighting
                .sky_model(game, night)
                .and_then(|m| models.load(&m))
                .map(|m| Arc::new(GpuModel::new(gpu, m)))
        });
        if self.sky_fade.is_none() && self.sky.iter().any(Option::is_some) {
            self.sky_fade = models.load("skyfade1").map(|m| Arc::new(GpuModel::new(gpu, m)));
        }
    }

    /// Whether `object` (its models `shown`, if any) is drawn as the
    /// arrow: a waypoint or store without a marker model, and stores
    /// unless their signs are asked for.
    fn as_arrow(&self, object: &AreaObject, shown: Option<&Arc<Shown>>) -> bool {
        object.kind.has_arrow()
            && self.arrow.is_some()
            && (shown.is_none() || (object.kind == ObjectKind::Store && !self.merchant_signs))
    }

    /// The arrow standing for an object at `transform`.
    fn arrow_instance(&self, transform: Mat4) -> Option<Instance> {
        let arrow = self.arrow.clone()?;
        Some(Instance { unlit: true, ..Instance::new(arrow, transform) })
    }

    /// Whether object `i` has something in the scene (its models, a marker
    /// model or the arrow), rather than a marker drawn over the view.
    pub fn draws(&self, area: &AreaModel, i: usize) -> bool {
        match (area.objects.get(i), self.objects.get(i)) {
            (Some(o), Some(shown)) => shown.is_some() || self.as_arrow(o, shown.as_ref()),
            _ => false,
        }
    }

    /// Follows `area` after an edit: loads the models of tiles and objects
    /// not seen before.
    pub fn update(&mut self, gpu: &Gpu, game: &GameData, area: &AreaModel) {
        if self.arrow.is_none() {
            self.arrow = Some(Arc::new(GpuModel::new(gpu, Arc::new(crate::marker::arrow()))));
            self.spawn_point = Models { game, cache: RefCell::new(HashMap::new()) }
                .load("spawnpoint")
                .map(|m| Arc::new(GpuModel::new(gpu, m)));
        }
        let models = Models { game, cache: RefCell::new(HashMap::new()) };
        // A skybox chosen anew (Area Properties).
        if self.sky_box.is_some() {
            self.load_sky(gpu, &models, game, area);
        }
        let load = |name: &str| models.load(name);
        // (A model that is read and then breaks what builds it is left out,
        // as a missing one is: the toolset carries on.)
        // (`None`: it broke; `Some(None)`: not found, or not readable.)
        let built = |name: &str| -> Option<Option<Loaded>> {
            mg_render::guard::guarded(name, || {
                let model = load(name)?;
                let anims = anim::animations(&model, &load);
                Some(Loaded::new(Arc::new(GpuModel::new(gpu, model)), anims))
            })
        };
        let loaded = |name: &str| built(name).flatten();
        let mut missing = Vec::new();
        let tile_cache = &mut self.tile_cache;
        let broken = &mut self.broken;
        self.tiles = area
            .tiles
            .iter()
            .map(|t| {
                let name = t.model.as_ref()?;
                tile_cache
                    .entry(name.clone())
                    .or_insert_with(|| {
                        let b = built(name);
                        if b.is_none() {
                            broken.insert(name.clone());
                        }
                        let l = b.flatten().map(Arc::new);
                        if l.is_none() {
                            missing.push(name.clone());
                        }
                        l
                    })
                    .clone()
            })
            .collect();
        // Objects that look the same share their models.
        let object_cache = &mut self.object_cache;
        self.objects =
            area.objects.iter().map(|o| shown(gpu, object_cache, o, &load, &mut missing)).collect();
        let needs_flame = area.tiles.iter().any(|t| t.source_lights.iter().any(|&s| s > 0));
        if needs_flame && self.flame.is_none() {
            self.flame = Some(loaded("fx_flame01"));
        }
        self.missing.extend(missing);
        self.missing.sort();
        self.missing.dedup();
        self.update_grass(gpu, game, area, &area.grass);
    }

    /// Grows `growing` (what grows on each surface material: the area's
    /// own, [`AreaModel::grass`], or settings being tried out) on
    /// `area`'s tiles: each tile model's grass is made once, until what
    /// grows changes.
    pub fn update_grass(
        &mut self,
        gpu: &Gpu,
        game: &GameData,
        area: &AreaModel,
        growing: &[(u32, GrassLook)],
    ) {
        if self.grass_for != growing {
            self.grass_for = growing.to_vec();
            self.grass_cache.clear();
            self.chunk_grass.clear();
        }
        let (grass, walkmeshes) = (&mut self.grass_cache, &mut self.walkmesh_cache);
        self.tile_grass = area
            .tiles
            .iter()
            .map(|t| {
                let name = t.model.as_ref().filter(|_| !growing.is_empty())?;
                let variant = crate::grass::variant(t.column, t.row);
                grass
                    .entry((name.clone(), variant))
                    .or_insert_with(|| {
                        let walk = walkmeshes
                            .entry(name.clone())
                            .or_insert_with(|| Walkmesh::of_tile(game, name).map(Arc::new));
                        let faces = &walk.as_ref()?.faces;
                        let model = crate::grass::tufts(name, variant, faces, growing)?;
                        let densities = crate::grass::densities(&model, faces, growing);
                        Some(Arc::new(TileGrass::new(gpu, model, densities)))
                    })
                    .clone()
            })
            .collect();
    }

    /// What grows that the grass shown was made for.
    pub fn grass_grown(&self) -> &[(u32, GrassLook)] {
        &self.grass_for
    }

    /// What `sight` sees of the grass of `area` ([`crate::grass_chosen`]).
    fn grass_in_sight(&self, area: &AreaModel, sight: &Sight) -> Vec<GrassDraw> {
        let grown: Vec<Grown> = (area.tiles.iter().zip(&self.tile_grass).enumerate())
            .filter_map(|(i, (t, g))| {
                let g = g.as_ref()?;
                Some((i, t.position, t.transform().transform_point3(g.centre), g.radius))
            })
            .collect();
        crate::grass_chosen(&grown, sight)
    }

    /// What the grass of chunk `tiles` (by index) is made of now: each
    /// tile's index, its grass and where it stands.
    fn made_of<'a>(
        &'a self,
        area: &'a AreaModel,
        tiles: &'a [usize],
    ) -> impl ExactSizeIterator<Item = (usize, Option<&'a Arc<TileGrass>>, Mat4)> + 'a {
        tiles.iter().map(|&i| (i, self.tile_grass[i].as_ref(), area.tiles[i].transform()))
    }

    /// The chunk's grass in one mesh, if it is made and of what the chunk
    /// has now.
    fn chunk_made(
        &self,
        area: &AreaModel,
        key: (i32, i32),
        tiles: &[usize],
    ) -> Option<&ChunkGrass> {
        self.chunk_grass.get(&key).filter(|c| c.is_of(self.made_of(area, tiles)))
    }

    /// Has the grass of the chunks far off that `sight` sees put together,
    /// those not yet (or that have changed), nearest first, on a thread of
    /// its own, and takes those made since the last call: until they are,
    /// their tiles are drawn one by one (the same tufts). Whether some are
    /// being made.
    pub fn grow_grass(&mut self, gpu: &Gpu, area: &AreaModel, sight: &Sight) -> bool {
        if self.chunk_worker.is_none() {
            self.chunk_worker = crate::grass_view::ChunkWorker::start();
        }
        let Some(worker) = self.chunk_worker.as_mut() else { return false };
        self.chunk_grass.extend(worker.made());
        let mut wanted: Vec<(f32, (i32, i32), MadeOf)> = self
            .grass_in_sight(area, sight)
            .into_iter()
            .filter_map(|d| match d {
                GrassDraw::Chunk(key, tiles) if self.chunk_made(area, key, &tiles).is_none() => {
                    let at = area.tiles[tiles[0]].position;
                    let made_of = (self.made_of(area, &tiles))
                        .filter_map(|(i, grass, transform)| Some((i, grass?.clone(), transform)))
                        .collect();
                    Some((at.distance(sight.eye), key, made_of))
                }
                _ => None,
            })
            .collect();
        wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
        let Some(worker) = self.chunk_worker.as_mut() else { return false };
        for (_, key, made_of) in wanted {
            worker.make(gpu, key, made_of);
        }
        worker.busy()
    }

    /// `object` (not in the area: a blueprint about to be placed) as
    /// see-through models, `opacity` opaque, and its box in its own space;
    /// no models and a marker's box for an object without models (a
    /// sound).
    pub fn ghost(
        &mut self,
        gpu: &Gpu,
        game: &GameData,
        object: &AreaObject,
        time: f32,
        opacity: f32,
    ) -> (Vec<Instance>, (Vec3, Vec3)) {
        let models = Models { game, cache: RefCell::new(HashMap::new()) };
        let load = |name: &str| models.load(name);
        let mut missing = Vec::new();
        let shown = shown(gpu, &mut self.object_cache, object, &load, &mut missing);
        if self.as_arrow(object, shown.as_ref()) {
            let arrow = self.arrow_instance(object.model_transform());
            let out = arrow.into_iter().map(|i| Instance { opacity, ..i }).collect();
            return (out, crate::marker::arrow_bounds());
        }
        let Some(shown) = shown else {
            return (Vec::new(), crate::pick::marker_bounds(object.kind));
        };
        let c = &shown.composed;
        let mut out = c.instances(c.idle.as_deref(), time, object.model_transform());
        for i in &mut out {
            i.opacity = opacity;
            i.unlit = object.kind.is_marker();
        }
        (out, shown.bounds)
    }

    /// `tiles` (not the area's: what a brush would make of them) as
    /// `view` shows them, `opacity` opaque, their models loaded (and kept)
    /// as the area's are.
    pub fn preview_tiles(
        &mut self,
        gpu: &Gpu,
        game: &GameData,
        tiles: &[AreaTile],
        view: &View,
        opacity: f32,
    ) -> Vec<Instance> {
        let models = Models { game, cache: RefCell::new(HashMap::new()) };
        let load = |name: &str| models.load(name);
        let (mut out, mut lights) = (Vec::new(), Vec::new());
        let mut shared = Shared::default();
        for t in tiles {
            let Some(name) = t.model.as_ref() else { continue };
            let loaded = self
                .tile_cache
                .entry(name.clone())
                .or_insert_with(|| {
                    mg_render::guard::guarded(name, || {
                        let model = load(name)?;
                        let anims = anim::animations(&model, &load);
                        Some(Arc::new(Loaded::new(Arc::new(GpuModel::new(gpu, model)), anims)))
                    })
                    .flatten()
                })
                .clone();
            if let Some(l) = loaded {
                self.tile(t, &l, view, &mut shared, &mut out, &mut lights);
            }
        }
        for i in &mut out {
            i.opacity = opacity;
        }
        out
    }

    /// Object `i`'s box in its own space: its models' (as it stands, in
    /// its pause), the arrow's, or a marker's.
    pub fn bounds(&self, area: &AreaModel, i: usize) -> (Vec3, Vec3) {
        let shown = self.objects.get(i).and_then(Option::as_ref);
        match shown {
            _ if self.as_arrow(&area.objects[i], shown) => crate::marker::arrow_bounds(),
            // (Nothing to see or to click: a light of an invisible model.)
            Some(s) if bare(s.bounds) => crate::pick::marker_bounds(area.objects[i].kind),
            Some(s) => s.bounds,
            None => crate::pick::marker_bounds(area.objects[i].kind),
        }
    }

    /// Whether object `i` is only a light: a placeable with a model too
    /// small to see whose appearance has a light (placeables.2da's
    /// `LightColor`), as the game's "Light, White" has, or whose model has
    /// the light node of a light Moonglow made (`mg_module::lights`). Its
    /// light, if so.
    pub fn bare_light(&self, area: &AreaModel, i: usize) -> Option<BareLight> {
        let shown = self.objects.get(i).and_then(Option::as_ref)?;
        let o = area.objects.get(i)?;
        if o.kind != ObjectKind::Placeable || !bare(shown.bounds) {
            return None;
        }
        if let Some(light) = o.preview.as_ref()?.lights.first() {
            let reach = mg_render::PointLight::new(Vec3::ZERO, light.color, light.radius, false, 4);
            return Some(BareLight {
                color: light.color,
                on: light.on,
                reach: reach.cutoff,
                stock: true,
            });
        }
        // (A builder's own small lamp model is drawn as it is: a light
        // Moonglow made is nothing else.)
        if !mg_module::lights::is_generated(&o.preview.as_ref()?.base.model) {
            return None;
        }
        let lamp = shown.composed.point_lights(glam::Mat4::IDENTITY).into_iter().next()?;
        Some(BareLight { color: lamp.color, on: true, reach: lamp.cutoff, stock: false })
    }

    /// How far along `ray` it meets object `i`'s model itself (its
    /// triangles, as it stands: in its pause, where its box is), for a click that passes a wide box to reach
    /// what is behind. `None`: the object has no model to meet (a marker,
    /// an arrow, an emitter alone) and is picked by its box;
    /// `Some(None)`: the ray passes its model.
    pub fn ray_hit(
        &self,
        area: &AreaModel,
        i: usize,
        ray: &crate::pick::Ray,
    ) -> Option<Option<f32>> {
        let shown = self.objects.get(i).and_then(Option::as_ref)?;
        let o = area.objects.get(i)?;
        if self.as_arrow(o, Some(shown)) || !shown.composed.has_meshes() {
            return None;
        }
        let inverse = o.model_transform().inverse();
        let (origin, dir) =
            (inverse.transform_point3(ray.origin), inverse.transform_vector3(ray.dir));
        Some(shown.composed.ray_hit_in(&shown.stance, origin, dir))
    }

    /// The particles of the placed objects with emitters (a campfire's
    /// flames, a portal, sparks: placeables that are nothing else show
    /// nothing without them), moved on by `dt` seconds and drawn for a
    /// camera with the view matrix `camera`. `sims` keeps each object's
    /// particles from frame to frame, by its place in the area's list and
    /// its model; those of objects no longer shown are dropped. The
    /// tiles' emitters follow.
    pub fn particles(
        &self,
        area: &AreaModel,
        view: &View,
        sims: &mut HashMap<(usize, usize), mg_render::particles::Particles>,
        dt: f32,
        camera: Mat4,
    ) -> Vec<mg_render::particles::ParticleBatch> {
        let mut out = Vec::new();
        let mut seen = Vec::new();
        // The objects with emitters, those nearest the view first: past
        // [`MAX_PARTICLE_OBJECTS`] the far ones go without.
        let eye = camera.inverse().w_axis.truncate();
        let emits = |n: &mg_mdl::Node| matches!(n.kind, mg_mdl::NodeKind::Emitter(_));
        let emitting = (area.objects.iter().zip(&self.objects).enumerate())
            .filter(|(_, (o, shown))| {
                view.shows(o.kind)
                    && shown
                        .as_ref()
                        .is_some_and(|s| s.composed.base().model.nodes.iter().any(emits))
            })
            .map(|(i, (o, _))| (i, o.position));
        for i in nearest(emitting, eye, MAX_PARTICLE_OBJECTS) {
            let (o, Some(shown)) = (&area.objects[i], &self.objects[i]) else { continue };
            let key = (i, Arc::as_ptr(shown) as usize);
            if self.failed.borrow().contains(&key.1) {
                continue;
            }
            let c = &shown.composed;
            let name = o.preview.as_ref().map_or("an object", |p| p.base.model.as_str());
            let transform = o.model_transform();
            let drawn = mg_render::guard::guarded(name, || {
                let (model, playing, pose) = c.emitters(c.idle.as_deref(), view.time)?;
                let new = !sims.contains_key(&key);
                let sim =
                    sims.entry(key).or_insert_with(|| mg_render::particles::Particles::new(model));
                sim.ground = o.position.z;
                if new {
                    let mut ahead = 0.0;
                    while ahead < PARTICLES_AHEAD {
                        let step = PARTICLES_AHEAD_STEP;
                        sim.update(model, playing, view.time, step, &pose, transform);
                        ahead += step;
                    }
                }
                sim.update(model, playing, view.time, dt, &pose, transform);
                Some(sim.batches(model, playing, view.time, &pose, transform, camera))
            });
            match drawn {
                Some(Some(batches)) => {
                    seen.push(key);
                    out.extend(batches);
                }
                Some(None) => {}
                None => {
                    self.failed.borrow_mut().insert(key.1);
                }
            }
        }
        // The tiles' own: a fountain's water, a forge's sparks. They emit
        // as the animations playing on the tile have them (its animation
        // loops switched on in Tile Properties, day or night). Those
        // nearest the view: past [`MAX_PARTICLE_TILES`] the far ones go
        // without (it was the first listed, wherever the camera was).
        let emitting = (area.tiles.iter().zip(&self.tiles).enumerate())
            .filter(|(_, (_, l))| l.as_ref().is_some_and(|l| l.gpu.model.nodes.iter().any(emits)))
            .map(|(i, (t, _))| (i, t.transform().w_axis.truncate()));
        for i in nearest(emitting, eye, MAX_PARTICLE_TILES) {
            let (tile, Some(loaded)) = (&area.tiles[i], &self.tiles[i]) else { continue };
            let model = &*loaded.gpu.model;
            // (Told apart from the objects' by counting down from the top.)
            let key = (usize::MAX - i, Arc::as_ptr(loaded) as usize);
            if self.failed.borrow().contains(&key.1) {
                continue;
            }
            let name = tile.model.as_deref().unwrap_or("a tile");
            let transform = tile.transform();
            let drawn = mg_render::guard::guarded(name, || {
                let layers = self.tile_layers(tile, loaded, view);
                let pose = if layers.is_empty() {
                    loaded.gpu.rest.clone()
                } else {
                    anim::Playing::new(model, &layers, view.time).pose(model)
                };
                // One animation of the emitters' keys, the later layer's
                // first (the one that has its say).
                let keys = (!layers.is_empty()).then(|| Animation {
                    length: layers.iter().map(|(a, _)| a.length).fold(0.0, f32::max),
                    nodes: layers
                        .iter()
                        .rev()
                        .flat_map(|(a, _)| a.nodes.iter())
                        .filter(|n| {
                            model
                                .nodes
                                .iter()
                                .any(|m| emits(m) && m.name.eq_ignore_ascii_case(&n.name))
                        })
                        .cloned()
                        .collect(),
                    ..Animation::default()
                });
                let new = !sims.contains_key(&key);
                let sim =
                    sims.entry(key).or_insert_with(|| mg_render::particles::Particles::new(model));
                sim.ground = transform.w_axis.z;
                // (Run ahead as an object's: a forge's fire was dark for
                // its first seconds.)
                if new {
                    let mut ahead = 0.0;
                    while ahead < PARTICLES_AHEAD {
                        let step = PARTICLES_AHEAD_STEP;
                        sim.update(model, keys.as_ref(), view.time, step, &pose, transform);
                        ahead += step;
                    }
                }
                sim.update(model, keys.as_ref(), view.time, dt, &pose, transform);
                sim.batches(model, keys.as_ref(), view.time, &pose, transform, camera)
            });
            match drawn {
                Some(batches) => {
                    seen.push(key);
                    out.extend(batches);
                }
                None => {
                    self.failed.borrow_mut().insert(key.1);
                }
            }
        }
        // The tiles' source lights: a torch's flame is `fx_flame01`, which
        // is nothing but emitters (and a light): without them a torch on
        // a wall lit it and showed no flame. Those nearest the view first.
        if let Some(Some(flame)) = &self.flame {
            let model = &*flame.gpu.model;
            let mut burning: Vec<(f32, usize, usize, u8, Mat4)> = Vec::new();
            for (i, (tile, loaded)) in area.tiles.iter().zip(&self.tiles).enumerate() {
                let Some(loaded) = loaded else { continue };
                if tile.source_lights.iter().all(|&v| v == 0) {
                    continue;
                }
                let layers = self.tile_layers(tile, loaded, view);
                let posed;
                let pose: &[Mat4] = if layers.is_empty() {
                    &loaded.gpu.rest
                } else {
                    let model = &loaded.gpu.model;
                    posed = anim::Playing::new(model, &layers, view.time).pose(model);
                    &posed
                };
                let transform = tile.transform();
                for (slot, &value) in tile.source_lights.iter().enumerate() {
                    let node = loaded.sources.get(slot).copied().flatten();
                    let Some(node) = node.filter(|_| value > 0) else { continue };
                    let at = transform * Mat4::from_translation(anim::node_position(pose, node));
                    let d = at.w_axis.truncate().distance_squared(eye);
                    burning.push((d, i, slot, value, at));
                }
            }
            burning.sort_by(|a, b| a.0.total_cmp(&b.0));
            burning.truncate(MAX_PARTICLE_OBJECTS);
            for (_, i, slot, value, at) in burning {
                // (Told apart from the tiles' by counting down from the
                // middle: two a tile.)
                let key = (usize::MAX / 2 - 2 * i - slot, Arc::as_ptr(&flame.gpu) as usize);
                if self.failed.borrow().contains(&key.1) {
                    continue;
                }
                let Some(playing) = flame.animation(&value.to_string()) else { continue };
                let drawn = mg_render::guard::guarded("fx_flame01", || {
                    let pose = anim::pose(model, playing, view.time);
                    let new = !sims.contains_key(&key);
                    let sim = sims
                        .entry(key)
                        .or_insert_with(|| mg_render::particles::Particles::new(model));
                    sim.ground = at.w_axis.z - 2.0;
                    if new {
                        let mut ahead = 0.0;
                        while ahead < 2.0 {
                            sim.update(model, Some(playing), view.time, 0.1, &pose, at);
                            ahead += 0.1;
                        }
                    }
                    sim.update(model, Some(playing), view.time, dt, &pose, at);
                    sim.batches(model, Some(playing), view.time, &pose, at, camera)
                });
                match drawn {
                    Some(batches) => {
                        seen.push(key);
                        out.extend(batches);
                    }
                    None => {
                        self.failed.borrow_mut().insert(key.1);
                    }
                }
            }
        }
        sims.retain(|k, _| seen.contains(k));
        out
    }

    /// The animations playing on a tile, later ones over earlier: its
    /// default, day or night, and the animation loops switched on.
    /// Each with its binding to the tile's model, as it was loaded.
    fn tile_layers<'a>(
        &self,
        tile: &AreaTile,
        loaded: &'a Loaded,
        view: &View,
    ) -> Vec<(&'a Animation, &'a anim::Binding)> {
        // (`TILE_ANIMATIONS`' places.)
        let mut names = vec![0, if view.night { 2 } else { 1 }];
        for (on, name) in tile.anim_loops.iter().zip([3, 4, 5]) {
            if *on {
                names.push(name);
            }
        }
        names.iter().filter_map(|&n| loaded.bound_at(loaded.tile[n]?)).collect()
    }

    /// The scene of `area` (the model these models were loaded for) as
    /// `view` shows it.
    pub fn scene(&self, area: &AreaModel, view: &View) -> Scene {
        self.scene_hiding(area, view, &[])
    }

    /// [`scene`](Self::scene) without the tiles of index `hidden` (a
    /// preview shows others there).
    /// Where the tiles are (their middles, as their instances are placed)
    /// that would be drawn but failed: their models read and broken while
    /// built, or while posed. Those `hidden` (a brush's preview over them)
    /// are not.
    pub fn failed_tiles(&self, area: &AreaModel, hidden: &[usize]) -> Vec<Vec3> {
        let failed = self.failed.borrow();
        area.tiles
            .iter()
            .zip(&self.tiles)
            .enumerate()
            .filter(|(i, _)| !hidden.contains(i))
            .filter(|(_, (tile, loaded))| match loaded {
                Some(l) => failed.contains(&(Arc::as_ptr(l) as usize)),
                None => tile.model.as_ref().is_some_and(|m| self.broken.contains(m)),
            })
            .map(|(_, (tile, _))| tile.transform().w_axis.truncate())
            .collect()
    }

    pub fn scene_hiding(&self, area: &AreaModel, view: &View, hidden: &[usize]) -> Scene {
        let mut instances = Vec::new();
        let mut lights = Vec::new();
        let mut shared = Shared::default();
        // What fails while it is posed is drawn no more (once noted), so
        // that it doesn't fail again every frame.
        let failed = |key: usize| self.failed.borrow().contains(&key);
        for (i, (tile, loaded)) in area.tiles.iter().zip(&self.tiles).enumerate() {
            let Some(loaded) = loaded.as_ref().filter(|_| !hidden.contains(&i)) else { continue };
            let key = Arc::as_ptr(loaded) as usize;
            if failed(key) {
                continue;
            }
            // (Into the scene's own lists, not a pair of new ones for each
            // tile each frame: what a tile added before it failed goes.)
            let (drawn, lit) = (instances.len(), lights.len());
            let name = tile.model.as_deref().unwrap_or("a tile");
            let posed = mg_render::guard::guarded(name, || {
                self.tile(tile, loaded, view, &mut shared, &mut instances, &mut lights);
            });
            if posed.is_none() {
                instances.truncate(drawn);
                lights.truncate(lit);
                self.failed.borrow_mut().insert(key);
            }
        }
        if let Some(sight) = &view.grass {
            let tile = |i: usize, level: usize| {
                let grass = self.tile_grass[i].as_ref().filter(|_| !hidden.contains(&i))?;
                let model = grass.levels[level.min(grass.levels.len() - 1)].clone();
                Some(Instance { grass: true, ..Instance::new(model, area.tiles[i].transform()) })
            };
            for draw in self.grass_in_sight(area, sight) {
                match draw {
                    GrassDraw::Tile(i, level) => instances.extend(tile(i, level)),
                    // (Whole where it is made and none of it is hidden;
                    // else its tiles at the same level, the same tufts.)
                    GrassDraw::Chunk(key, tiles) => {
                        let whole = self.chunk_made(area, key, &tiles);
                        match whole.filter(|_| !tiles.iter().any(|i| hidden.contains(i))) {
                            Some(c) => instances.push(Instance {
                                grass: true,
                                ..Instance::new(c.model.clone(), c.at)
                            }),
                            None => {
                                let last = crate::grass::STEPS - 1;
                                instances.extend(tiles.iter().filter_map(|&i| tile(i, last)));
                            }
                        }
                    }
                }
            }
        }
        let lit_shadows = view.light_shadows_of(&area.lighting).is_some();
        for (o, shown) in area.objects.iter().zip(&self.objects) {
            if !view.shows(o.kind) {
                continue;
            }
            // An encounter's spawn points: the game's marker at each,
            // turned the way what spawns there faces.
            if let (true, Some(marker)) = (self.spawn_markers, &self.spawn_point) {
                for (p, facing) in o.spawn_points.iter().zip(&o.spawn_facings) {
                    let turn = Mat4::from_rotation_z(facing - std::f32::consts::FRAC_PI_2);
                    let at = Mat4::from_translation(*p) * turn;
                    // (See-through: a prism as tall as a creature, which
                    // would hide what stands behind it.)
                    instances.push(Instance {
                        unlit: true,
                        opacity: SPAWN_POINT_OPACITY,
                        ..Instance::new(marker.clone(), at)
                    });
                }
            }
            let transform = o.model_transform();
            if self.as_arrow(o, shown.as_ref()) {
                instances.extend(self.arrow_instance(transform));
                continue;
            }
            let Some(shown) = shown else { continue };
            let c = &shown.composed;
            // A waypoint's flag, a store's sign and a sound's speaker
            // are markers: in their own colours, unlit.
            let unlit = o.kind.is_marker();
            let time = if view.animate { view.time } else { 0.0 };
            let key = Arc::as_ptr(shown) as usize;
            if failed(key) {
                continue;
            }
            let name = o.preview.as_ref().map_or("an object", |p| p.base.model.as_str());
            let posed = mg_render::guard::guarded(name, || {
                (c.instances(c.idle.as_deref(), time, transform), c.point_lights(transform))
            });
            let Some((drawn, lit)) = posed else {
                self.failed.borrow_mut().insert(key);
                continue;
            };
            // The sun's shadows: cast by static placeables with the tiles;
            // by what moves (creatures, dynamic placeables, doors) where
            // creature shadows are asked for, as the client's Creature
            // Shadow Detail governs them (Best: they cast; Fast and Off:
            // none); falling on static placeables.
            let casts = match o.kind {
                ObjectKind::Placeable if o.is_static => view.shadows,
                ObjectKind::Placeable | ObjectKind::Door | ObjectKind::Creature => {
                    view.creature_shadows
                }
                _ => false,
            };
            let from_lights = lit_shadows && o.kind == ObjectKind::Creature;
            instances.extend(drawn.into_iter().map(|i| Instance {
                unlit,
                casts_shadows: casts && i.casts_shadows,
                casts_light_shadows: from_lights && i.casts_shadows,
                receives_shadows: o.is_static,
                ..i
            }));
            lights.extend(lit);
        }
        if !view.lit {
            lights.clear();
        }
        let l = &area.lighting;
        let sky = l.sky(view.night);
        let fog = view.fog.then(|| fog(l, view.night));
        let background = fog.map_or([0.0; 3], |f| f.color.to_array());
        Scene {
            instances,
            lights,
            area: area_light(sky, l.direction(view.night), view.lit),
            fog,
            background,
            particles: Vec::new(),
            env_map: l.env_map.clone(),
            sky: self.sky[usize::from(view.night)]
                .clone()
                .map(|m| Instance::new(m, glam::Mat4::IDENTITY)),
            // Over a sky, the fade in the fog's colour (whether or not the
            // fog is shown).
            sky_fade: self.sky[usize::from(view.night)]
                .as_ref()
                .and(self.sky_fade.clone())
                .map(|m| (Instance::new(m, glam::Mat4::IDENTITY), self::fog(l, view.night).color)),
            lines: Vec::new(),
            // (Water ripples whether or not the objects' animations play,
            // as the tiles' animations go on.)
            time: view.time,
            environment: mg_render::Environment::from_are(l.flags, l.wind_power),
            shadows: view.shadows_of(l),
            light_shadows: view.light_shadows_of(l),
        }
    }

    /// A tile's model as `view` shows it playing the tile's animations:
    /// what every tile of the model playing the same shares.
    fn posed_tile(&self, tile: &AreaTile, loaded: &Loaded, view: &View) -> PosedTile {
        let model = &loaded.gpu.model;
        let layers = self.tile_layers(tile, loaded, view);
        // (The animations' keys gathered once, for the pose, the lights
        // and the meshes.)
        let playing = anim::Playing::new(model, &layers, view.time);
        let pose = (!layers.is_empty()).then(|| Arc::new(playing.pose(model)));
        let lamps = {
            let pose: &[Mat4] = pose.as_deref().map_or(&loaded.gpu.rest, |p| p);
            playing.lamps(model, pose)
        };
        // Source lights: the flame at the tile's `sl1`/`sl2` node.
        let sources = loaded.sources;
        let mut state = (!layers.is_empty()).then(|| playing.mesh_state(&loaded.gpu));
        // Fade Geometry: the fading meshes aren't drawn.
        if view.fade {
            let fades = |m: &mg_render::model::GpuMesh| {
                matches!(&model.nodes[m.node].kind,
                    mg_mdl::NodeKind::Mesh(mesh) if matches!(mesh.tilefade, 1 | 4))
            };
            if loaded.gpu.meshes.iter().any(fades) {
                let s = state.get_or_insert_with(|| mg_render::scene::MeshState::new(&loaded.gpu));
                for (o, m) in s.meshes.iter_mut().zip(&loaded.gpu.meshes) {
                    if fades(m) {
                        o.alpha = Some(0.0);
                    }
                }
            }
        }
        PosedTile { pose, state: state.map(Arc::new), lamps, sources }
    }

    fn tile(
        &self,
        tile: &AreaTile,
        loaded: &Arc<Loaded>,
        view: &View,
        shared: &mut Shared,
        instances: &mut Vec<Instance>,
        lights: &mut Vec<PointLight>,
    ) {
        let transform = tile.transform();
        let key = (Arc::as_ptr(loaded) as usize, tile.anim_loops);
        let posed = match shared.tiles.get(&key) {
            Some(posed) => posed.clone(),
            None => {
                let posed = Arc::new(self.posed_tile(tile, loaded, view));
                shared.tiles.insert(key, posed.clone());
                posed
            }
        };
        let pose: &[Mat4] = posed.pose.as_deref().map_or(&loaded.gpu.rest, |p| p);
        let color = |row: u8| self.colors.get(usize::from(row)).copied();
        let main_light = |slot: usize| color(tile.main_lights[slot.min(1)]);
        lights.extend(posed.lamps.iter().filter_map(|l| l.placed(transform, &main_light)));
        // Source lights: the flame at the tile's `sl1`/`sl2` node.
        if let Some(Some(flame)) = &self.flame {
            for (slot, &value) in tile.source_lights.iter().enumerate() {
                if value == 0 {
                    continue;
                }
                let Some(node) = posed.sources[slot] else { continue };
                let at = transform * Mat4::from_translation(anim::node_position(pose, node));
                let burning = shared.flames.entry(value).or_insert_with(|| {
                    let a = flame.animation(&value.to_string())?;
                    let pose = anim::pose(&flame.gpu.model, a, view.time);
                    let lamps = anim::lamps_layers(&flame.gpu.model, &[a], view.time, &pose);
                    Some(Arc::new(PosedFlame { pose: Arc::new(pose), lamps }))
                });
                let Some(burning) = burning else { continue };
                lights.extend(burning.lamps.iter().filter_map(|l| l.placed(at, &|_| None)));
                // (An effect: the game casts no shadow of it.)
                instances.push(Instance {
                    pose: Some(burning.pose.clone()),
                    casts_shadows: false,
                    ..Instance::new(flame.gpu.clone(), at)
                });
            }
        }
        // The model's `replace_tex`, drawn with the tile's replacement
        // (where the model has one: every tile without the field takes
        // row 0, and the game's models but one have none).
        let replaced = loaded.gpu.model.draws_texture(crate::REPLACE_TEX);
        let textures = tile.replace_texture.as_ref().filter(|_| replaced).map(|t| {
            Arc::new(std::collections::HashMap::from([(crate::REPLACE_TEX.to_string(), t.clone())]))
        });
        instances.push(Instance {
            pose: posed.pose.clone(),
            state: posed.state.clone(),
            textures,
            tile: true,
            receives_shadows: true,
            casts_shadows: view.shadows,
            ..Instance::new(loaded.gpu.clone(), transform)
        });
    }

    /// What the loaded models use (Area Statistics): distinct tile and
    /// object models, their meshes and triangles, the GPU memory of their
    /// vertex and index buffers, and the distinct textures they name.
    pub fn usage(&self) -> Usage {
        let mut models: Vec<&Arc<GpuModel>> = Vec::new();
        let tiles: Vec<&Arc<Loaded>> = {
            let mut t: Vec<&Arc<Loaded>> = self.tiles.iter().flatten().collect();
            t.sort_by_key(|l| Arc::as_ptr(l));
            t.dedup_by_key(|l| Arc::as_ptr(l));
            t
        };
        models.extend(tiles.iter().map(|l| &l.gpu));
        let mut objects: Vec<&Arc<Shown>> = self.objects.iter().flatten().collect();
        objects.sort_by_key(|s| Arc::as_ptr(s));
        objects.dedup_by_key(|s| Arc::as_ptr(s));
        for o in &objects {
            models.extend(o.composed.models());
        }
        let mut u =
            Usage { tile_models: tiles.len(), object_models: objects.len(), ..Usage::default() };
        let mut textures: Vec<&str> = Vec::new();
        for m in models {
            for mesh in &m.meshes {
                u.meshes += 1;
                u.triangles += mesh.index_count as usize / 3;
                // (Its own share of the buffers it lies in, with others.)
                let vertex = std::mem::size_of::<mg_render::model::Vertex>() as u64;
                u.buffer_bytes +=
                    u64::from(mesh.vertex_count) * vertex + u64::from(mesh.index_count) * 4;
                if let Some(t) = &mesh.material.texture {
                    textures.push(t);
                }
            }
        }
        textures.sort_unstable();
        textures.dedup();
        u.textures = textures.len();
        u
    }
}

/// A placed light with nothing to see ([`AreaScene::bare_light`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BareLight {
    /// Its color: lightcolor.2da's values, or the model's light's.
    pub color: Vec3,
    /// It shines (**Illuminated**: a model's light always does).
    pub on: bool,
    /// Where its light ends, meters.
    pub reach: f32,
    /// Its light is its appearance's (`LightColor`: the game's lights,
    /// whose color Light Color changes), not its model's.
    pub stock: bool,
}

/// An object's models, from `cache` if an object that looks the same
/// loaded them (what could not be loaded named in `missing`).
fn shown(
    gpu: &Gpu,
    cache: &mut HashMap<String, Option<Arc<Shown>>>,
    o: &AreaObject,
    load: &dyn Fn(&str) -> Option<Arc<Model>>,
    missing: &mut Vec<String>,
) -> Option<Arc<Shown>> {
    let p = o.preview.as_ref()?;
    cache
        .entry(format!("{p:?}"))
        .or_insert_with(|| {
            let c = Composed::new(gpu, p, load);
            match &c {
                Some(c) => missing.extend(c.missing.iter().cloned()),
                None => missing.push(p.base.model.clone()),
            }
            c.map(|composed| {
                // As it stands in the area: in its pause (a dragon at
                // rest lies stretched out, wings spread).
                let stance = composed.stance(composed.idle.as_deref(), 0.0);
                let bounds = composed.bounds_of(&stance);
                // (A model of nothing drawn is the size of what it holds
                // undrawn, where that is something: the invisible
                // object's box, as Aurora outlines it.)
                let hidden =
                    composed.hidden_bounds().filter(|_| !composed.has_meshes() || bare(bounds));
                let bounds =
                    hidden.filter(|h| h.0.is_finite() && h.1.is_finite()).unwrap_or(bounds);
                Arc::new(Shown { composed, bounds, stance })
            })
        })
        .clone()
}

/// lightcolor.2da's colours (RED, GREEN, BLUE) by row.
fn light_colors(game: &GameData) -> Vec<Vec3> {
    let Ok(t) = game.table("lightcolor") else { return Vec::new() };
    (0..t.len())
        .map(|row| {
            let c = |col| t.get(row, col).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0);
            Vec3::new(c("RED"), c("GREEN"), c("BLUE"))
        })
        .collect()
}

/// The sun's or moon's light (towards `direction`, else the game's own
/// direction), or with the lighting off the working light.
fn area_light(sky: crate::Sky, direction: Option<Vec3>, lit: bool) -> AreaLight {
    if lit {
        AreaLight::from_are(sky.ambient, sky.diffuse, direction.unwrap_or_else(sun_direction))
    } else {
        working_light()
    }
}

/// How opaque an encounter's spawn point markers are.
const SPAWN_POINT_OPACITY: f32 = 0.35;

/// The light of a view with the area's lighting off: bright and white
/// from everywhere, with enough from the sun's side to show shapes.
fn working_light() -> AreaLight {
    AreaLight {
        ambient: Vec3::splat(0.75_f32.powf(2.2)),
        diffuse: Vec3::splat(0.5_f32.powf(2.2)),
        direction: sun_direction(),
    }
}

/// The direction towards the sun and moon: the game's default (towards
/// 4000, 4500, 7000).
fn sun_direction() -> Vec3 {
    Vec3::new(4000.0, 4500.0, 7000.0).normalize()
}

/// The area's fog, as the client sets it (its fog uniforms read back in
/// `client_render.rs`): always on, ending at `FogClipDist` and starting
/// `FogAmount` metres nearer than 30 m (so past 30 it starts behind the
/// camera), at most 1 m before its end; its colour as stored (gamma space).
pub fn fog(l: &Lighting, night: bool) -> Fog {
    let sky = l.sky(night);
    let end = l.fog_clip;
    let start = (30.0 - f32::from(sky.fog_amount)).min(end - 1.0);
    let c = sky.fog_color;
    let ch = |shift: u32| ((c >> shift) & 0xFF) as f32 / 255.0;
    Fog { start, end, color: Vec3::new(ch(0), ch(8), ch(16)) }
}

/// Where a camera sees the whole area from above, for a first view.
pub fn overview(area: &AreaModel) -> mg_render::Camera {
    let (w, h) = area.size();
    let centre = Vec3::new(w / 2.0, h / 2.0, 0.0);
    let span = w.max(h).max(TILE_SIZE);
    mg_render::Camera::orbit(centre, span * 1.2, -std::f32::consts::FRAC_PI_2, 55f32.to_radians())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small model lying flat is nothing to see (the game's
    /// `dag_invisible`, a square of 0.2 m, a light's), and is picked by a
    /// marker's box; a small one with height (a potion, 0.15 by 0.27 by
    /// 0.15 m) is something, and by its own.
    #[test]
    fn only_a_small_flat_model_is_bare() {
        let square = (Vec3::new(-0.1, -0.1, 0.0), Vec3::new(0.1, 0.1, 0.0));
        let potion = (Vec3::new(-0.076, 0.007, -0.077), Vec3::new(0.077, 0.274, 0.076));
        let floor = (Vec3::new(-2.0, -2.0, 0.0), Vec3::new(2.0, 2.0, 0.0));
        assert!(bare(square));
        assert!(!bare(potion));
        assert!(!bare(floor), "flat, but large");
        assert!(bare((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN))), "no vertex");
    }

    /// The emitters given particles are the nearest the camera, objects'
    /// and tiles' alike: a large area's tiles with emitters were the
    /// first 128 listed, the south-west's, wherever the camera was.
    #[test]
    fn the_emitters_nearest_the_camera_get_particles() {
        // A row of 300 tiles' places, west to east, the eye at the east end.
        let places = (0..300).map(|i| (i, Vec3::new(i as f32 * 10.0 + 5.0, 5.0, 0.0)));
        let eye = Vec3::new(3000.0, 5.0, 20.0);
        let chosen = nearest(places, eye, MAX_PARTICLE_TILES);
        assert_eq!(chosen.len(), MAX_PARTICLE_TILES);
        assert_eq!(chosen[0], 299, "the nearest first");
        assert!(chosen.iter().all(|&i| i >= 300 - MAX_PARTICLE_TILES), "{chosen:?}");
        // Fewer than the most: all of them; ties in the order of the list.
        let two = [(5, Vec3::X), (2, -Vec3::X)].into_iter();
        assert_eq!(nearest(two, Vec3::ZERO, 8), vec![2, 5]);
    }

    #[test]
    fn with_the_lighting_off_a_dark_area_is_evenly_lit() {
        // A black night.
        let night = crate::Sky::default();
        let lit = area_light(night, None, true);
        assert_eq!((lit.ambient, lit.diffuse), (Vec3::ZERO, Vec3::ZERO));
        // Off: bright and white, whatever the area's colours.
        let red = crate::Sky { ambient: 0x0000_00FF, diffuse: 0x0000_0040, ..night };
        let off = area_light(night, None, false);
        assert_eq!(off, area_light(red, Some(Vec3::X), false));
        assert!(off.ambient.min_element() > 0.5 && off.ambient.x == off.ambient.z);
        assert!(off.diffuse.min_element() > 0.0);
    }

    /// The sun's shadows by day where Sun Shadows are on, the moon's at
    /// night where Moon Shadows are, as opaque as Shadow Opacity, ending
    /// their fade at the fog clip; none for the other, at no opacity, with
    /// the lighting or the view's shadows off.
    #[test]
    fn shadows_follow_the_time_of_day_s_flag_and_the_opacity() {
        let l = Lighting {
            sun: crate::Sky { shadows: true, ..Default::default() },
            moon: crate::Sky { shadows: false, ..Default::default() },
            is_night: false,
            day_night_cycle: false,
            fog_clip: 60.0,
            sky_box: 0,
            env_map: None,
            shadow_opacity: 30,
            flags: 0,
            wind_power: 0,
            sun_direction: None,
            moon_direction: None,
        };
        let day = View {
            time: 0.0,
            night: false,
            fog: false,
            lit: true,
            show: [true; 9],
            animate: false,
            fade: false,
            grass: None,
            shadows: true,
            creature_shadows: true,
            light_shadows: false,
        };
        let s = day.shadows_of(&l).unwrap();
        assert_eq!((s.opacity, s.fade_limit), (0.3, 60.0));
        let night = View { night: true, ..day };
        assert_eq!(night.shadows_of(&l), None);
        let moonlit = Lighting {
            sun: crate::Sky { shadows: false, ..l.sun },
            moon: crate::Sky { shadows: true, ..l.moon },
            ..l.clone()
        };
        assert!(night.shadows_of(&moonlit).is_some() && day.shadows_of(&moonlit).is_none());
        assert_eq!(day.shadows_of(&Lighting { shadow_opacity: 0, ..l.clone() }), None);
        assert_eq!(View { lit: false, ..day }.shadows_of(&l), None);
        // (Creatures alone cast them too.)
        assert!(View { shadows: false, ..day }.shadows_of(&l).is_some());
        let none = View { shadows: false, creature_shadows: false, ..day };
        assert_eq!(none.shadows_of(&l), None);
        // The lights' only where asked, and where the area has no sun's or
        // moon's at the time.
        let lamps = View { light_shadows: true, ..day };
        assert_eq!(day.light_shadows_of(&l), None);
        assert_eq!(lamps.light_shadows_of(&l), None);
        let s = View { night: true, ..lamps }.light_shadows_of(&l).unwrap();
        assert_eq!((s.opacity, s.fade_limit), (0.3, 60.0));
        assert!(lamps.light_shadows_of(&moonlit).is_some());
        assert_eq!(View { lit: false, ..lamps }.light_shadows_of(&moonlit), None);
    }
}
