//! Models: the game's MDL files, binary (compiled) and ASCII, read into one
//! data model for the renderer.
//!
//! - Nodes are kept in pre-order (`nodes[0]` is the root), with parent and
//!   child indices; a node's type is its [`NodeKind`].
//! - Meshes are stored per corner, as the binary format stores them: every
//!   vertex stream shares one index, which faces use. The ASCII loader
//!   de-indexes its separate vertex and texture-vertex lists, splitting
//!   vertices where smoothing groups give different normals, and keeps each
//!   render vertex's source vertex so per-vertex data (skin weights, dangly
//!   constraints, animated vertices) still applies.
//! - Controllers are keyed by lower-case name (`position`, `orientation`,
//!   `alpha`, `birthrate`, …); orientations are always quaternions
//!   (x, y, z, w), as binary files store them (ASCII's axis-angle is
//!   converted). A model's own controllers are its rest pose; animations
//!   carry keyed ones.

pub mod ascii;
pub mod binary;
pub mod ctrl;

use thiserror::Error;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
/// A rotation as x, y, z, w.
pub type Quat = [f32; 4];

pub const IDENTITY: Quat = [0.0, 0.0, 0.0, 1.0];

#[derive(Debug, Clone, PartialEq, Error)]
pub enum MdlError {
    #[error("truncated or corrupt binary model: {0}")]
    Binary(String),
    #[error("line {line}: {message}")]
    Ascii { line: usize, message: String },
    #[error("empty model")]
    Empty,
}

/// What a model is for (the engine's render order and behaviour).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Classification {
    #[default]
    Other,
    Effect,
    Tile,
    Character,
    Door,
}

impl Classification {
    pub fn from_byte(b: u8) -> Classification {
        match b {
            1 => Classification::Effect,
            2 => Classification::Tile,
            4 => Classification::Character,
            8 => Classification::Door,
            _ => Classification::Other,
        }
    }

    pub fn from_name(s: &str) -> Classification {
        match s.to_ascii_lowercase().as_str() {
            "effect" | "effects" => Classification::Effect,
            "tile" => Classification::Tile,
            "character" => Classification::Character,
            "door" => Classification::Door,
            _ => Classification::Other,
        }
    }
}

/// A model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Model {
    pub name: String,
    /// The model animations and missing nodes come from (`NULL`: none).
    pub supermodel: Option<String>,
    pub classification: Classification,
    pub ignore_fog: bool,
    pub animation_scale: f32,
    /// In pre-order; `nodes[0]` is the root.
    pub nodes: Vec<Node>,
    pub animations: Vec<Animation>,
}

impl Model {
    /// Reads either format: binary files start with four zero bytes.
    pub fn read(data: &[u8]) -> Result<Model, MdlError> {
        if is_binary(data) { binary::read(data) } else { ascii::read(data) }
    }

    /// A node by name (case-insensitive).
    pub fn node(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name.eq_ignore_ascii_case(name))
    }

    /// An animation by name (case-insensitive).
    pub fn animation(&self, name: &str) -> Option<&Animation> {
        self.animations.iter().find(|a| a.name.eq_ignore_ascii_case(name))
    }
}

/// Whether the data is a compiled (binary) model.
pub fn is_binary(data: &[u8]) -> bool {
    data.len() >= 12 && data[..4] == [0, 0, 0, 0]
}

/// A controller: key times and, per key, `columns` values.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Controller {
    /// Lower case, without `key` (e.g. `position`, `colorstart`).
    pub name: String,
    pub times: Vec<f32>,
    /// Row-major, `times.len() * columns` values.
    pub values: Vec<f32>,
    pub columns: usize,
}

impl Controller {
    /// A single value (a rest-pose setting).
    pub fn constant(name: &str, values: &[f32]) -> Controller {
        Controller {
            name: name.to_string(),
            times: vec![0.0],
            values: values.to_vec(),
            columns: values.len(),
        }
    }

    /// The values of key `i`.
    pub fn row(&self, i: usize) -> &[f32] {
        self.values.get(i * self.columns..(i + 1) * self.columns).unwrap_or(&[])
    }
}

/// A node.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    /// Relative to the parent.
    pub position: Vec3,
    pub orientation: Quat,
    pub scale: f32,
    pub inherit_color: bool,
    /// The node's other rest-pose controllers (alpha, self-illumination,
    /// light colour and radius, emitter parameters, …).
    pub controllers: Vec<Controller>,
    pub kind: NodeKind,
}

impl Node {
    pub fn new(name: &str, kind: NodeKind) -> Node {
        Node {
            name: name.to_string(),
            parent: None,
            children: Vec::new(),
            position: [0.0; 3],
            orientation: IDENTITY,
            scale: 1.0,
            inherit_color: false,
            controllers: Vec::new(),
            kind,
        }
    }

    /// A rest-pose controller's values.
    pub fn value(&self, name: &str) -> Option<&[f32]> {
        self.controllers.iter().find(|c| c.name == name).map(|c| c.row(0))
    }

    pub fn mesh(&self) -> Option<&Mesh> {
        match &self.kind {
            NodeKind::Mesh(m) => Some(m),
            _ => None,
        }
    }
}

/// What a node is.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Dummy,
    Light(Light),
    Emitter(Emitter),
    Reference(Reference),
    Camera,
    Mesh(Box<Mesh>),
}

impl NodeKind {
    /// The ASCII node type.
    pub fn type_name(&self) -> &'static str {
        match self {
            NodeKind::Dummy => "dummy",
            NodeKind::Light(_) => "light",
            NodeKind::Emitter(_) => "emitter",
            NodeKind::Reference(_) => "reference",
            NodeKind::Camera => "camera",
            NodeKind::Mesh(m) => match m.extra {
                MeshExtra::None => "trimesh",
                MeshExtra::Skin(_) => "skin",
                MeshExtra::Dangly(_) => "danglymesh",
                MeshExtra::Anim(_) => "animmesh",
                MeshExtra::Aabb(_) => "aabb",
            },
        }
    }
}

/// A light.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Light {
    pub flare_radius: f32,
    pub flare_sizes: Vec<f32>,
    pub flare_positions: Vec<f32>,
    pub flare_color_shifts: Vec<Vec3>,
    pub flare_textures: Vec<String>,
    /// 1 (highest) to 5.
    pub priority: u32,
    pub ambient_only: bool,
    /// Tile lights: 0 main lights, 1 source lights.
    pub dynamic_type: u32,
    pub affect_dynamic: bool,
    pub shadow: bool,
    pub generate_flare: bool,
    pub fading: bool,
}

/// A particle emitter's fixed settings (its particle parameters are
/// controllers).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Emitter {
    pub deadspace: f32,
    pub blast_radius: f32,
    pub blast_length: f32,
    pub xgrid: u32,
    pub ygrid: u32,
    pub spawntype: u32,
    /// Fountain, Single, Explosion, Lightning.
    pub update: String,
    /// Normal, Linked, Billboard_to_Local_Z, …
    pub render: String,
    /// Normal, Punch-Through, Lighten.
    pub blend: String,
    pub texture: Option<String>,
    pub chunk: Option<String>,
    pub two_sided: bool,
    pub looping: bool,
    pub render_order: u32,
    /// p2p 0x1, p2p_sel 0x2, affectedByWind 0x4, m_isTinted 0x8, bounce
    /// 0x10, random 0x20, inherit 0x40, inheritvel 0x80, inherit_local
    /// 0x100, splat 0x200, inherit_part 0x400.
    pub flags: u32,
}

/// Emitter flag names and bits, in [`Emitter::flags`].
pub const EMITTER_FLAGS: [(&str, u32); 11] = [
    ("p2p", 0x1),
    ("p2p_sel", 0x2),
    ("affectedbywind", 0x4),
    ("m_istinted", 0x8),
    ("bounce", 0x10),
    ("random", 0x20),
    ("inherit", 0x40),
    ("inheritvel", 0x80),
    ("inherit_local", 0x100),
    ("splat", 0x200),
    ("inherit_part", 0x400),
];

/// A reference (target for emitters that shoot between points).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reference {
    pub model: Option<String>,
    pub reattachable: bool,
}

/// A triangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Face {
    pub vertices: [u32; 3],
    /// Surface material (`surfacemat.2da`, walkmeshes).
    pub material: u32,
}

/// A mesh, with per-corner vertex streams.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    pub diffuse: Vec3,
    pub ambient: Vec3,
    pub specular: Vec3,
    pub shininess: f32,
    pub render: bool,
    pub shadow: bool,
    pub beaming: bool,
    pub rotate_texture: bool,
    pub tilefade: u32,
    pub transparency_hint: u32,
    /// bitmap/texture0 … texture3 (lower case; `NULL`: none).
    pub textures: [Option<String>; 4],
    /// EE: an MTR.
    pub material: Option<String>,
    /// EE: `renderhint` (lower case).
    pub renderhint: Option<String>,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    /// UV sets 0–3 (empty when absent).
    pub uvs: [Vec<Vec2>; 4],
    /// Vertex colours (RGBA), when the model has them.
    pub colors: Vec<[u8; 4]>,
    pub faces: Vec<Face>,
    /// Each render vertex's vertex in the source list (ASCII) or itself
    /// (binary); per-vertex data such as weights is indexed by it.
    pub source: Vec<u32>,
    /// Each render vertex's texture vertex in the source `tverts` list
    /// (ASCII) or itself (binary); animated UVs are indexed by it.
    pub source_uv: Vec<u32>,
    pub extra: MeshExtra,
}

/// What a mesh adds.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MeshExtra {
    #[default]
    None,
    Skin(Skin),
    Dangly(Dangly),
    Anim(AnimMesh),
    Aabb(Vec<AabbEntry>),
}

/// A skinned mesh's bones and weights.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Skin {
    /// Bone nodes (indices into [`Model::nodes`]).
    pub bones: Vec<usize>,
    /// Per render vertex, up to four (bone, weight) pairs; bone indexes
    /// [`Skin::bones`].
    pub weights: Vec<[(u16, f32); 4]>,
    /// The inverse bind pose per node, as stored in binary models
    /// (rotation, translation); empty for ASCII.
    pub inverse_bind: Vec<(Quat, Vec3)>,
}

/// A mesh that sways (cloth, hair, plants).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dangly {
    /// Per render vertex, 0 (fixed) to 255.
    pub constraints: Vec<f32>,
    pub displacement: f32,
    pub tightness: f32,
    pub period: f32,
}

/// A mesh with vertex animation (the sets are in the animations).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimMesh {
    pub sample_period: f32,
}

/// An entry of a walkmesh's bounding-box tree, in pre-order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AabbEntry {
    pub min: Vec3,
    pub max: Vec3,
    /// The face of a leaf, or −1.
    pub face: i32,
}

/// An animation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Animation {
    pub name: String,
    pub length: f32,
    pub transtime: f32,
    pub animroot: String,
    pub events: Vec<(f32, String)>,
    /// In pre-order; bound to the model's nodes by name.
    pub nodes: Vec<AnimNode>,
}

/// A node of an animation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimNode {
    pub name: String,
    pub parent: Option<usize>,
    pub controllers: Vec<Controller>,
    /// Vertex animation: sample period, vertex sets and UV sets, each set a
    /// full list over the mesh's source vertices and source texture
    /// vertices (ASCII) or its vertices (binary).
    pub anim_mesh: Option<AnimMeshSets>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimMeshSets {
    pub sample_period: f32,
    pub vertex_sets: Vec<Vec<Vec3>>,
    pub uv_sets: Vec<Vec<Vec2>>,
}

/// A rotation of `angle` radians about `axis` (ASCII orientations).
pub fn axis_angle(axis: Vec3, angle: f32) -> Quat {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if len < 1e-12 || angle == 0.0 {
        return IDENTITY;
    }
    let (s, c) = (angle / 2.0).sin_cos();
    [axis[0] / len * s, axis[1] / len * s, axis[2] / len * s, c]
}

/// Face normals from positions, area-weighted (the cross product's length).
pub(crate) fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
}

pub(crate) fn normalize(v: Vec3) -> Vec3 {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-20 { [0.0, 0.0, 1.0] } else { [v[0] / len, v[1] / len, v[2] / len] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_angle_quaternions() {
        let q = axis_angle([0.0, 0.0, 1.0], std::f32::consts::PI);
        assert!((q[2] - 1.0).abs() < 1e-6 && q[3].abs() < 1e-6);
        assert_eq!(axis_angle([0.0, 0.0, 0.0], 1.0), IDENTITY);
    }
}
