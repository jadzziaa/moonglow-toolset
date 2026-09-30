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
    /// The area as the toolset first shows it.
    pub fn of(area: &AreaModel) -> View {
        View { time: 0.0, night: area.lighting.night_by_default(), fog: true, show: [true; 9] }
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
    sky: Option<Arc<GpuModel>>,
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
        scene.sky = area
            .lighting
            .sky_model(game, false)
            .and_then(|m| models.load(&m))
            .map(|m| Arc::new(GpuModel::new(gpu, m)));
        scene
    }

    /// Follows `area` after an edit: loads the models of tiles and objects
    /// not seen before.
    pub fn update(&mut self, gpu: &Gpu, game: &GameData, area: &AreaModel) {
        let models = Models { game, cache: RefCell::new(HashMap::new()) };
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
            let transform = o.transform();
            instances.extend(c.instances(c.idle.as_deref(), view.time, transform));
            lights.extend(c.point_lights(transform));
        }
        let l = &area.lighting;
        let sky = l.sky(view.night);
        let fog = if view.fog { fog(l, view.night) } else { None };
        let background = fog.map_or([0.0; 3], |f| f.color.to_array());
        Scene {
            instances,
            lights,
            area: AreaLight::from_are(sky.ambient, sky.diffuse, sun_direction()),
            fog,
            background,
            particles: Vec::new(),
            env_map: l.env_map.clone(),
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
        instances.push(Instance {
            pose: (!layers.is_empty()).then(|| Arc::new(pose)),
            state,
            ..Instance::new(loaded.gpu.clone(), transform)
        });
    }

    /// Whether the area has a skybox model loaded.
    pub fn has_sky(&self) -> bool {
        self.sky.is_some()
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

/// The area's fog: it ends at `FogClipDist` and starts nearer the more fog
/// there is. How the engine turns `FogAmount` (0–15) into the start is not
/// measured yet; this takes a linear scale (0 none, 15 at the camera).
fn fog(l: &Lighting, night: bool) -> Option<Fog> {
    let sky = l.sky(night);
    if sky.fog_amount == 0 {
        return None;
    }
    let end = l.fog_clip.max(1.0);
    let start = end * (1.0 - f32::from(sky.fog_amount.min(15)) / 15.0);
    let c = sky.fog_color;
    let ch = |shift: u32| ((c >> shift) & 0xFF) as f32 / 255.0;
    Some(Fog { start, end, color: Vec3::new(ch(0), ch(8), ch(16)) })
}

/// Where a camera sees the whole area from above, for a first view.
pub fn overview(area: &AreaModel) -> mg_render::Camera {
    let (w, h) = area.size();
    let centre = Vec3::new(w / 2.0, h / 2.0, 0.0);
    let span = w.max(h).max(TILE_SIZE);
    mg_render::Camera::orbit(centre, span * 1.2, -std::f32::consts::FRAC_PI_2, 55f32.to_radians())
}
