//! Previews of blueprints: the models the game shows for a creature, an
//! item, a placeable or a door, assembled from the blueprint and the rules
//! 2DAs, with their PLT colours and environment maps
//! (`docs/research/notes_models.md` part D).
//!
//! A [`Preview`] names models and where they hang; [`compose::Composed`]
//! loads them and turns them into renderer instances at a moment of an
//! animation.

use std::collections::BTreeMap;
use std::sync::Arc;

use glam::Vec3;
use mg_2da::TwoDa;
use mg_core::{ResRef, ResType};
use mg_gff::Struct;
use mg_mdl::{Model, NodeKind};
use mg_rules::GameData;

pub mod compose;
mod creature;
mod item;
mod object;

pub use creature::creature;
pub use item::item;
pub use object::{door, placeable};

/// Why a preview could not be made.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("{0}")]
    Rules(#[from] mg_rules::RulesError),
    #[error("{table}.2da has no row {row}")]
    NoRow { table: &'static str, row: i64 },
    #[error("no model {0}")]
    NoModel(String),
}

/// A model in a preview.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// The model's resource name, lower case.
    pub model: String,
    /// The base model's node it hangs from (lower case); `None`: the base's
    /// origin.
    pub attach: Option<String>,
    pub scale: f32,
    /// Textures to use instead of the model's (lower case, from → to).
    pub textures: BTreeMap<String, String>,
    /// PLT colours by layer: skin, hair, metal 1 and 2, cloth 1 and 2,
    /// leather 1 and 2, tattoo 1 and 2.
    pub colors: Option<[u8; 10]>,
    /// The object's environment map (texture alpha is reflectivity); `None`:
    /// only what its textures ask for.
    pub env_map: Option<String>,
    /// Plays the animation on its own nodes: skinned robes and cloaks (their
    /// bones copy the skeleton's), wings and tails (with their own
    /// animations).
    pub animated: bool,
}

impl Part {
    fn new(model: impl Into<String>) -> Part {
        Part {
            model: model.into().to_ascii_lowercase(),
            attach: None,
            scale: 1.0,
            textures: BTreeMap::new(),
            colors: None,
            env_map: None,
            animated: false,
        }
    }
}

/// A light a preview carries (a placeable's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewLight {
    /// From the base's origin, metres.
    pub offset: Vec3,
    /// lightcolor.2da values.
    pub color: Vec3,
    pub radius: f32,
}

/// What to draw for a blueprint.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// The base model: a creature's skeleton or body, the item, placeable or
    /// door.
    pub base: Part,
    pub parts: Vec<Part>,
    /// The animation it stands in (`pause1`, `cpause1`, a placeable's
    /// state).
    pub idle: Option<String>,
    pub lights: Vec<PreviewLight>,
}

impl Preview {
    /// A model on its own.
    pub fn model(name: &str) -> Preview {
        Preview { base: Part::new(name), parts: Vec::new(), idle: None, lights: Vec::new() }
    }
}

/// PLT layers.
const SKIN: usize = 0;
const HAIR: usize = 1;
const TATTOO1: usize = 8;
const TATTOO2: usize = 9;

/// An item's colours in PLT layer order (metal, cloth, leather), over
/// `colors`.
fn item_colors(item: &Struct, mut colors: [u8; 10]) -> [u8; 10] {
    for (layer, field) in [
        (2, "Metal1Color"),
        (3, "Metal2Color"),
        (4, "Cloth1Color"),
        (5, "Cloth2Color"),
        (6, "Leather1Color"),
        (7, "Leather2Color"),
    ] {
        if let Some(v) = item.integer(field) {
            colors[layer] = v.clamp(0, 255) as u8;
        }
    }
    colors
}

/// A 2DA cell, trimmed; `None` for `****` or blank.
fn cell<'a>(t: &'a TwoDa, row: usize, column: &str) -> Option<&'a str> {
    t.get(row, column).map(str::trim).filter(|v| !v.is_empty() && *v != "****")
}

fn cell_f32(t: &TwoDa, row: usize, column: &str) -> Option<f32> {
    cell(t, row, column)?.parse().ok()
}

fn cell_int(t: &TwoDa, row: usize, column: &str) -> Option<i64> {
    let v = cell(t, row, column)?;
    v.parse::<i64>().ok().or_else(|| v.parse::<f32>().ok().map(|f| f as i64))
}

/// An environment map column: `default` is the toolset's `chrome1`; blank
/// or `****`: none.
fn env_map(value: Option<&str>) -> Option<String> {
    match value {
        Some(v) if v.eq_ignore_ascii_case("default") => Some("chrome1".into()),
        Some(v) => Some(v.to_ascii_lowercase()),
        None => None,
    }
}

/// Resources the builders look up.
struct Lookup<'a> {
    game: &'a GameData,
}

impl Lookup<'_> {
    fn has_model(&self, name: &str) -> bool {
        ResRef::from_str(name).is_ok() && self.game.resman.get_named(name, ResType::MDL).is_ok()
    }

    fn model(&self, name: &str) -> Option<Arc<Model>> {
        let data = self.game.resman.get_named(name, ResType::MDL).ok()?;
        Model::read(&data).ok().map(Arc::new)
    }

    /// Whether a texture name resolves to anything (MTR, DDS, TGA, PLT).
    fn has_texture(&self, name: &str) -> bool {
        let Ok(r) = ResRef::from_str(name) else { return false };
        let rm = &self.game.resman;
        rm.texture(r).is_some()
            || [ResType::PLT, ResType::MTR]
                .iter()
                .any(|&t| rm.get(&mg_resman::ResKey::new(r, t)).is_ok())
    }

    /// The meshes' textures (lower case).
    fn bitmaps(&self, model: &str) -> Vec<String> {
        let Some(m) = self.model(model) else { return Vec::new() };
        let mut out: Vec<String> = m
            .nodes
            .iter()
            .filter_map(|n| match &n.kind {
                NodeKind::Mesh(mesh) if mesh.render => mesh.textures[0].clone(),
                _ => None,
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }
}
