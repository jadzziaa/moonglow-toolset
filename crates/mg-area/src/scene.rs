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

use crate::{AreaModel, AreaObject, AreaTile, Lighting, ObjectKind, TILE_SIZE};

/// How many objects' particles are simulated in a frame at most (an area
/// of hundreds of torches still draws at once): those nearest the view.
const MAX_PARTICLE_OBJECTS: usize = 256;

/// A model no larger than this across (metres) has nothing to see or to
/// click (the game's `dag_invisible`): its object is drawn and picked by a
/// marker's box.
const BARE: f32 = 0.5;

/// Whether a model's box is that of nothing to see: [`BARE`] across at
/// most, or no box at all (a model without a vertex).
fn bare((min, max): (Vec3, Vec3)) -> bool {
    let across = max - min;
    !across.is_finite() || across.min_element() < 0.0 || across.length() < BARE
}

/// An emitter new to the view is run ahead this far, in steps this long,
/// so that it shows as it does once going: a wide mist lets out a few
/// slow particles a second that live a quarter of a minute, and showed
/// next to nothing for as long after every change to the area.
const PARTICLES_AHEAD: f32 = 20.0;
const PARTICLES_AHEAD_STEP: f32 = 0.25;

/// And how many tiles'.
const MAX_PARTICLE_TILES: usize = 128;

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
        }
    }

    pub fn shows(&self, kind: ObjectKind) -> bool {
        self.show[kind.index()]
    }
}

/// A model with the animations it can play (its own and its supermodels').
#[derive(Debug)]
struct Loaded {
    gpu: Arc<GpuModel>,
    anims: Vec<(String, Arc<Model>)>,
}

impl Loaded {
    fn animation(&self, name: &str) -> Option<&Animation> {
        self.anims
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .and_then(|(_, owner)| owner.animation(name))
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
    /// The loaded tiles and objects that failed while posed (by where they
    /// are kept): left out from then on.
    failed: RefCell<std::collections::HashSet<usize>>,
    /// Moonglow's arrow ([`crate::marker`]).
    arrow: Option<Arc<GpuModel>>,
    /// Merchants as the game's marker for one (a $) rather than as arrows.
    pub merchant_signs: bool,
    /// The game's marker for an encounter's spawn point (`spawnpoint`).
    spawn_point: Option<Arc<GpuModel>>,
    /// Encounters' spawn points are marked (Options › Area).
    pub spawn_markers: bool,
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
    /// arrow: a waypoint or merchant without a marker model, and merchants
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
        let loaded = |name: &str| -> Option<Loaded> {
            mg_render::guard::guarded(name, || {
                let model = load(name)?;
                let anims = anim::animations(&model, &load);
                Some(Loaded { gpu: Arc::new(GpuModel::new(gpu, model)), anims })
            })
            .flatten()
        };
        let mut missing = Vec::new();
        let tile_cache = &mut self.tile_cache;
        self.tiles = area
            .tiles
            .iter()
            .map(|t| {
                let name = t.model.as_ref()?;
                tile_cache
                    .entry(name.clone())
                    .or_insert_with(|| {
                        let l = loaded(name).map(Arc::new);
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
                        Some(Arc::new(Loaded { gpu: Arc::new(GpuModel::new(gpu, model)), anims }))
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

    /// Whether object `i` is only a light: a placeable whose appearance
    /// has a light (placeables.2da's `LightColor`) and a model too small
    /// to see, as the game's "Light, White" has. Its color, if
    /// so (lightcolor.2da's values).
    pub fn bare_light(&self, area: &AreaModel, i: usize) -> Option<Vec3> {
        let shown = self.objects.get(i).and_then(Option::as_ref)?;
        let o = area.objects.get(i)?;
        let light = o.preview.as_ref()?.lights.first()?;
        (o.kind == ObjectKind::Placeable && bare(shown.bounds)).then_some(light.color)
    }

    /// How far along `ray` it meets object `i`'s model itself (its
    /// triangles, at rest), for a click that passes a wide box to reach
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
        Some(shown.composed.ray_hit(origin, dir))
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
        let mut emitting: Vec<(f32, usize)> = area
            .objects
            .iter()
            .zip(&self.objects)
            .enumerate()
            .filter(|(_, (o, shown))| {
                view.shows(o.kind)
                    && shown
                        .as_ref()
                        .is_some_and(|s| s.composed.base().model.nodes.iter().any(emits))
            })
            .map(|(i, (o, _))| (o.position.distance_squared(eye), i))
            .collect();
        emitting.sort_by(|a, b| a.0.total_cmp(&b.0));
        emitting.truncate(MAX_PARTICLE_OBJECTS);
        for (_, i) in emitting {
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
        // loops switched on in Tile Properties, day or night).
        let mut tiles = 0;
        for (i, (tile, loaded)) in area.tiles.iter().zip(&self.tiles).enumerate() {
            let Some(loaded) = loaded else { continue };
            let model = &*loaded.gpu.model;
            let emits = |n: &mg_mdl::Node| matches!(n.kind, mg_mdl::NodeKind::Emitter(_));
            if tiles >= MAX_PARTICLE_TILES || !model.nodes.iter().any(emits) {
                continue;
            }
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
                    anim::pose_layers(model, &layers, view.time)
                };
                // One animation of the emitters' keys, the later layer's
                // first (the one that has its say).
                let keys = (!layers.is_empty()).then(|| Animation {
                    length: layers.iter().map(|a| a.length).fold(0.0, f32::max),
                    nodes: layers
                        .iter()
                        .rev()
                        .flat_map(|a| a.nodes.iter())
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
                let sim =
                    sims.entry(key).or_insert_with(|| mg_render::particles::Particles::new(model));
                sim.ground = transform.w_axis.z;
                sim.update(model, keys.as_ref(), view.time, dt, &pose, transform);
                sim.batches(model, keys.as_ref(), view.time, &pose, transform, camera)
            });
            match drawn {
                Some(batches) => {
                    tiles += 1;
                    seen.push(key);
                    out.extend(batches);
                }
                None => {
                    self.failed.borrow_mut().insert(key.1);
                }
            }
        }
        sims.retain(|k, _| seen.contains(k));
        out
    }

    /// The animations playing on a tile, later ones over earlier: its
    /// default, day or night, and the animation loops switched on.
    fn tile_layers<'a>(
        &self,
        tile: &AreaTile,
        loaded: &'a Loaded,
        view: &View,
    ) -> Vec<&'a Animation> {
        let mut names = vec!["tiledefault", if view.night { "night" } else { "day" }];
        for (on, name) in tile.anim_loops.iter().zip(["animloop01", "animloop02", "animloop03"]) {
            if *on {
                names.push(name);
            }
        }
        names.iter().filter_map(|n| loaded.animation(n)).collect()
    }

    /// The scene of `area` (the model these models were loaded for) as
    /// `view` shows it.
    pub fn scene(&self, area: &AreaModel, view: &View) -> Scene {
        self.scene_hiding(area, view, &[])
    }

    /// [`scene`](Self::scene) without the tiles of index `hidden` (a
    /// preview shows others there).
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
            let (mut drawn, mut lit) = (Vec::new(), Vec::new());
            let name = tile.model.as_deref().unwrap_or("a tile");
            let posed = mg_render::guard::guarded(name, || {
                self.tile(tile, loaded, view, &mut shared, &mut drawn, &mut lit);
            });
            match posed {
                Some(()) => {
                    instances.extend(drawn);
                    lights.extend(lit);
                }
                None => {
                    self.failed.borrow_mut().insert(key);
                }
            }
        }
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
            // A waypoint's flag, a merchant's sign and a sound's speaker
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
            instances.extend(drawn.into_iter().map(|i| Instance { unlit, ..i }));
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
            area: area_light(sky, view.lit),
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
        }
    }

    /// A tile's model as `view` shows it playing the tile's animations:
    /// what every tile of the model playing the same shares.
    fn posed_tile(&self, tile: &AreaTile, loaded: &Loaded, view: &View) -> PosedTile {
        let model = &loaded.gpu.model;
        let layers = self.tile_layers(tile, loaded, view);
        let pose =
            (!layers.is_empty()).then(|| Arc::new(anim::pose_layers(model, &layers, view.time)));
        let lamps = {
            let pose: &[Mat4] = pose.as_deref().map_or(&loaded.gpu.rest, |p| p);
            anim::lamps_layers(model, &layers, view.time, pose)
        };
        // Source lights: the flame at the tile's `sl1`/`sl2` node.
        let sources = [1, 2].map(|slot| {
            let suffix = format!("sl{slot}");
            model.nodes.iter().position(|n| n.name.to_ascii_lowercase().ends_with(&suffix))
        });
        let mut state =
            (!layers.is_empty()).then(|| anim::mesh_state_layers(&loaded.gpu, &layers, view.time));
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
                instances.push(Instance {
                    pose: Some(burning.pose.clone()),
                    ..Instance::new(flame.gpu.clone(), at)
                });
            }
        }
        // The model's `replace_tex`, drawn with the tile's replacement.
        let textures = tile.replace_texture.as_ref().map(|t| {
            Arc::new(std::collections::HashMap::from([("replace_tex".to_string(), t.clone())]))
        });
        instances.push(Instance {
            pose: posed.pose.clone(),
            state: posed.state.clone(),
            textures,
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
                u.buffer_bytes += mesh.vertices.size() + mesh.indices.size();
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

    /// How many distinct looks the objects have (each loaded once).
    pub fn object_models(&self) -> usize {
        let mut seen: Vec<*const Shown> = self.objects.iter().flatten().map(Arc::as_ptr).collect();
        seen.sort();
        seen.dedup();
        seen.len()
    }
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
                let bounds = composed.bounds_in(composed.idle.as_deref(), 0.0);
                Arc::new(Shown { composed, bounds })
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

/// The sun's or moon's light, or with the lighting off the working light.
fn area_light(sky: crate::Sky, lit: bool) -> AreaLight {
    if lit {
        AreaLight::from_are(sky.ambient, sky.diffuse, sun_direction())
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

    #[test]
    fn with_the_lighting_off_a_dark_area_is_evenly_lit() {
        // A black night.
        let night = crate::Sky::default();
        let lit = area_light(night, true);
        assert_eq!((lit.ambient, lit.diffuse), (Vec3::ZERO, Vec3::ZERO));
        // Off: bright and white, whatever the area's colours.
        let red = crate::Sky { ambient: 0x0000_00FF, diffuse: 0x0000_0040, ..night };
        let off = area_light(night, false);
        assert_eq!(off, area_light(red, false));
        assert!(off.ambient.min_element() > 0.5 && off.ambient.x == off.ambient.z);
        assert!(off.diffuse.min_element() > 0.0);
    }
}
