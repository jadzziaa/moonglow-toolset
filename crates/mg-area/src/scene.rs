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

use crate::{AreaModel, AreaTile, Lighting, ObjectKind, TILE_SIZE};

/// How to show the area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Seconds, for animations.
    pub time: f32,
    pub night: bool,
    pub fog: bool,
    /// Which kinds of object are drawn, by [`ObjectKind::index`].
    pub show: [bool; 9],
}

impl View {
    /// The area as the toolset first shows it (without fog, as Aurora's
    /// Scene › Fog starts unchecked).
    pub fn of(area: &AreaModel) -> View {
        View { time: 0.0, night: area.lighting.night_by_default(), fog: false, show: [true; 9] }
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

/// An object's models and their bounds (rest pose, the object's own
/// space).
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

    /// Follows `area` after an edit: loads the models of tiles and objects
    /// not seen before.
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

    pub fn update(&mut self, gpu: &Gpu, game: &GameData, area: &AreaModel) {
        let models = Models { game, cache: RefCell::new(HashMap::new()) };
        // A skybox chosen anew (Area Properties).
        if self.sky_box.is_some() {
            self.load_sky(gpu, &models, game, area);
        }
        let load = |name: &str| models.load(name);
        let loaded = |name: &str| -> Option<Loaded> {
            let model = load(name)?;
            let anims = anim::animations(&model, &load);
            Some(Loaded { gpu: Arc::new(GpuModel::new(gpu, model)), anims })
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
        self.objects = area
            .objects
            .iter()
            .map(|o| {
                let p = o.preview.as_ref()?;
                object_cache
                    .entry(format!("{p:?}"))
                    .or_insert_with(|| {
                        let c = Composed::new(gpu, p, &load);
                        match &c {
                            Some(c) => missing.extend(c.missing.iter().cloned()),
                            None => missing.push(p.base.model.clone()),
                        }
                        c.map(|composed| {
                            let bounds = composed.bounds();
                            Arc::new(Shown { composed, bounds })
                        })
                    })
                    .clone()
            })
            .collect();
        let needs_flame = area.tiles.iter().any(|t| t.source_lights.iter().any(|&s| s > 0));
        if needs_flame && self.flame.is_none() {
            self.flame = Some(loaded("fx_flame01"));
        }
        self.missing.extend(missing);
        self.missing.sort();
        self.missing.dedup();
    }

    /// Object `i`'s box in its own space: its models' (rest pose), or a
    /// marker's.
    pub fn bounds(&self, area: &AreaModel, i: usize) -> (Vec3, Vec3) {
        match self.objects.get(i) {
            Some(Some(s)) => s.bounds,
            _ => crate::pick::marker_bounds(area.objects[i].kind),
        }
    }

    /// The scene of `area` (the model these models were loaded for) as
    /// `view` shows it.
    pub fn scene(&self, area: &AreaModel, view: &View) -> Scene {
        let mut instances = Vec::new();
        let mut lights = Vec::new();
        for (tile, loaded) in area.tiles.iter().zip(&self.tiles) {
            let Some(loaded) = loaded else { continue };
            self.tile(tile, loaded, view, &mut instances, &mut lights);
        }
        for (o, shown) in area.objects.iter().zip(&self.objects) {
            let Some(shown) = shown.as_ref().filter(|_| view.shows(o.kind)) else { continue };
            let c = &shown.composed;
            let transform = o.model_transform();
            instances.extend(c.instances(c.idle.as_deref(), view.time, transform));
            lights.extend(c.point_lights(transform));
        }
        let l = &area.lighting;
        let sky = l.sky(view.night);
        let fog = view.fog.then(|| fog(l, view.night));
        let background = fog.map_or([0.0; 3], |f| f.color.to_array());
        Scene {
            instances,
            lights,
            area: AreaLight::from_are(sky.ambient, sky.diffuse, sun_direction()),
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
        }
    }

    fn tile(
        &self,
        tile: &AreaTile,
        loaded: &Loaded,
        view: &View,
        instances: &mut Vec<Instance>,
        lights: &mut Vec<PointLight>,
    ) {
        let model = &loaded.gpu.model;
        let transform = tile.transform();
        let mut names = vec!["tiledefault", if view.night { "night" } else { "day" }];
        for (on, name) in tile.anim_loops.iter().zip(["animloop01", "animloop02", "animloop03"]) {
            if *on {
                names.push(name);
            }
        }
        let layers: Vec<&Animation> = names.iter().filter_map(|n| loaded.animation(n)).collect();
        let pose = if layers.is_empty() {
            loaded.gpu.rest.clone()
        } else {
            anim::pose_layers(model, &layers, view.time)
        };
        let color = |row: u8| self.colors.get(usize::from(row)).copied();
        lights.extend(anim::lights_layers(model, &layers, view.time, &pose, transform, &|slot| {
            color(tile.main_lights[slot.min(1)])
        }));
        // Source lights: the flame at the tile's `sl1`/`sl2` node.
        if let Some(Some(flame)) = &self.flame {
            for (slot, &value) in tile.source_lights.iter().enumerate() {
                if value == 0 {
                    continue;
                }
                let suffix = format!("sl{}", slot + 1);
                let Some(node) =
                    model.nodes.iter().position(|n| n.name.to_ascii_lowercase().ends_with(&suffix))
                else {
                    continue;
                };
                let at = transform * Mat4::from_translation(anim::node_position(&pose, node));
                let Some(a) = flame.animation(&value.to_string()) else { continue };
                let flame_pose = anim::pose(&flame.gpu.model, a, view.time);
                lights.extend(anim::lights(
                    &flame.gpu.model,
                    Some(a),
                    view.time,
                    &flame_pose,
                    at,
                    &|_| None,
                ));
                instances.push(Instance {
                    pose: Some(Arc::new(flame_pose)),
                    ..Instance::new(flame.gpu.clone(), at)
                });
            }
        }
        let state = (!layers.is_empty()).then(|| {
            let s = anim::mesh_state_layers(&loaded.gpu, &layers, view.time);
            Arc::new(s)
        });
        // The model's `replace_tex`, drawn with the tile's replacement.
        let textures = tile.replace_texture.as_ref().map(|t| {
            Arc::new(std::collections::HashMap::from([("replace_tex".to_string(), t.clone())]))
        });
        instances.push(Instance {
            pose: (!layers.is_empty()).then(|| Arc::new(pose)),
            state,
            textures,
            ..Instance::new(loaded.gpu.clone(), transform)
        });
    }

    /// Whether the area has a skybox model loaded.
    pub fn has_sky(&self) -> bool {
        self.sky.iter().any(Option::is_some)
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
