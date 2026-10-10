//! Compiled (binary) models written from `mg-mdl`'s model of one: the
//! layout the game loads, a 32-bit memory dump (`mg-mdl`'s `binary.rs` is
//! its reader; the layout is in the toolset's `docs/research/
//! notes_models.md` §B, from nwnmdlcomp's headers and every compiled model
//! in the game).
//!
//! What a binary holds and the model does not is derived here: each face's
//! plane and the faces beside it, a mesh's bounds, a walkmesh's tree from
//! its entries, part numbers. What the game ignores (function pointers,
//! back pointers, padding: its own compiler leaves garbage there) is zero.
//!
//! This is the first stage of the native compiler (the plan's Phase 9): a
//! model read from a binary writes back to one that reads the same. What a
//! compiler derives from ASCII (vertices split by corner, normals from
//! smoothing groups, a skin's bone tables) comes with the next stages.

use std::collections::HashMap;

use crate::ctrl::flags;
use crate::{
    AabbEntry, AnimNode, Animation, Classification, Controller, Emitter, Light, Mesh, MeshExtra,
    Model, Node, NodeKind, Skin, Vec3,
};
use thiserror::Error;

/// Why a model cannot be written as a binary.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CompileError {
    #[error("the model has no nodes")]
    Empty,
    #[error("{node}: {count} vertices (a mesh holds at most 65,535)")]
    TooManyVertices { node: String, count: usize },
    #[error("{node}: {count} faces (a mesh holds at most 32,767)")]
    TooManyFaces { node: String, count: usize },
    #[error("{node}: the name is longer than {max} characters")]
    NameTooLong { node: String, max: usize },
    #[error("{node}: no controller named {name} for this kind of node")]
    UnknownController { node: String, name: String },
    #[error("{node}: {what}")]
    Invalid { node: String, what: String },
}

/// The model data section: structures placed at offsets, four bytes
/// aligned, pointers being offsets from its start.
#[derive(Default)]
struct Data(Vec<u8>);

impl Data {
    /// `size` zero bytes, at a multiple of four.
    fn alloc(&mut self, size: usize) -> usize {
        while !self.0.len().is_multiple_of(4) {
            self.0.push(0);
        }
        let at = self.0.len();
        self.0.resize(at + size, 0);
        at
    }
    fn put(&mut self, at: usize, bytes: &[u8]) {
        self.0[at..at + bytes.len()].copy_from_slice(bytes);
    }
    fn u8(&mut self, at: usize, v: u8) {
        self.put(at, &[v]);
    }
    fn u16(&mut self, at: usize, v: u16) {
        self.put(at, &v.to_le_bytes());
    }
    fn u32(&mut self, at: usize, v: u32) {
        self.put(at, &v.to_le_bytes());
    }
    fn f32(&mut self, at: usize, v: f32) {
        self.u32(at, v.to_bits());
    }
    fn vec3(&mut self, at: usize, v: Vec3) {
        for (i, x) in v.iter().enumerate() {
            self.f32(at + 4 * i, *x);
        }
    }
    fn floats(&mut self, at: usize, v: &[f32]) {
        for (i, x) in v.iter().enumerate() {
            self.f32(at + 4 * i, *x);
        }
    }
    /// A name in a fixed field, NUL-terminated (cut to fit).
    fn name(&mut self, at: usize, size: usize, s: &str) {
        let bytes: Vec<u8> = s.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).collect();
        let n = bytes.len().min(size - 1);
        self.put(at, &bytes[..n]);
    }
    /// An array header: offset, count and allocated count.
    fn array(&mut self, at: usize, offset: usize, count: usize) {
        self.u32(at, if count == 0 { 0 } else { offset as u32 });
        self.u32(at + 4, count as u32);
        self.u32(at + 8, count as u32);
    }
    /// An array of floats placed after what is there, and its header.
    fn float_array(&mut self, at: usize, v: &[f32]) {
        let offset = self.alloc(v.len() * 4);
        self.floats(offset, v);
        self.array(at, offset, v.len());
    }
}

/// The raw data section: vertex streams, appended.
#[derive(Default)]
struct Raw(Vec<u8>);

/// The raw pointer that points nowhere.
const NO_RAW: u32 = u32::MAX;

impl Raw {
    fn floats(&mut self, v: impl IntoIterator<Item = f32>) -> u32 {
        let at = self.0.len() as u32;
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
        at
    }
    fn bytes(&mut self, v: &[u8]) -> u32 {
        let at = self.0.len() as u32;
        self.0.extend_from_slice(v);
        at
    }
}

pub(crate) fn node_flags(kind: &NodeKind) -> u32 {
    flags::HEADER
        | match kind {
            NodeKind::Dummy => 0,
            NodeKind::Light(_) => flags::LIGHT,
            NodeKind::Emitter(_) => flags::EMITTER,
            NodeKind::Camera => flags::CAMERA,
            NodeKind::Reference(_) => flags::REFERENCE,
            NodeKind::Mesh(m) => {
                flags::MESH
                    | match m.extra {
                        MeshExtra::None => 0,
                        MeshExtra::Skin(_) => flags::SKIN,
                        MeshExtra::Dangly(_) => flags::DANGLY,
                        MeshExtra::Anim(_) => flags::ANIM,
                        MeshExtra::Aabb(_) => flags::AABB,
                    }
            }
        }
}

/// How many bones the skin structure of the 1.69 game holds (17 and, in
/// what it calls padding, an eighteenth).
const OLD_SKIN_BONES: usize = 18;
/// How many EE's skin structure holds.
const EE_SKIN_BONES: usize = 64;

/// A node structure's size for its type (`bones`: a skin's).
fn node_size(node_flags: u32, bones: usize) -> usize {
    if node_flags & flags::SKIN != 0 {
        if bones > OLD_SKIN_BONES { 0x3B0 } else { 0x2D4 }
    } else if node_flags & flags::DANGLY != 0 {
        0x288
    } else if node_flags & flags::ANIM != 0 {
        0x2A8
    } else if node_flags & flags::AABB != 0 {
        0x274
    } else if node_flags & flags::MESH != 0 {
        0x270
    } else if node_flags & flags::LIGHT != 0 {
        0xCC
    } else if node_flags & flags::EMITTER != 0 {
        0x148
    } else if node_flags & flags::REFERENCE != 0 {
        0xB4
    } else {
        0x70
    }
}

/// A controller's binary ID for a node type.
pub(crate) fn controller_id(node_flags: u32, name: &str) -> Option<u32> {
    let name = if name == "setfillumcolor" { "selfillumcolor" } else { name };
    if let Some(id) = name.strip_prefix("ctrl").and_then(|n| n.parse().ok()) {
        return Some(id);
    }
    (0..=512).find(|&id| crate::ctrl::name(node_flags, id) == name)
}

/// The node type whose controllers these are, for an animation's node the
/// model has no node for: the first type that knows every name.
pub(crate) fn flags_for_controllers(controllers: &[Controller]) -> u32 {
    [flags::HEADER, flags::HEADER | flags::MESH, flags::HEADER | flags::LIGHT]
        .into_iter()
        .chain([flags::HEADER | flags::EMITTER])
        .find(|&f| controllers.iter().all(|c| controller_id(f, &c.name).is_some()))
        .unwrap_or(flags::HEADER)
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(u: Vec3, v: Vec3) -> Vec3 {
    [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
}
fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn length(v: Vec3) -> f32 {
    dot(v, v).sqrt()
}

/// What a mesh's faces hold besides their corners: per face its plane (a
/// unit normal and the distance `d` of `n·p + d = 0`) and the face across
/// each edge (−1: none), and the mesh's area.
pub(crate) struct FaceData {
    pub planes: Vec<(Vec3, f32)>,
    pub adjacent: Vec<[i16; 3]>,
    pub area: f32,
}

/// Faces meet where they share an edge's two positions (vertices are per
/// corner, so those of one place differ across a seam or a hard edge); an
/// edge is the corners `i` and `i + 1`.
pub(crate) fn face_data(mesh: &Mesh) -> FaceData {
    let corner =
        |v: u32| mesh.vertices.get(v as usize).copied().unwrap_or([0.0; 3]).map(f32::to_bits);
    let mut planes = Vec::with_capacity(mesh.faces.len());
    let mut area = 0.0;
    let mut edges: HashMap<([u32; 3], [u32; 3]), Vec<usize>> = HashMap::new();
    for (i, f) in mesh.faces.iter().enumerate() {
        let p = f.vertices.map(|v| mesh.vertices.get(v as usize).copied().unwrap_or([0.0; 3]));
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let len = length(n);
        area += len * 0.5;
        let n = if len > 0.0 { n.map(|x| x / len) } else { [0.0; 3] };
        planes.push((n, -dot(n, p[0])));
        for e in 0..3 {
            let (a, b) = (corner(f.vertices[e]), corner(f.vertices[(e + 1) % 3]));
            edges.entry(if a <= b { (a, b) } else { (b, a) }).or_default().push(i);
        }
    }
    let adjacent = mesh
        .faces
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut out = [-1i16; 3];
            for (e, o) in out.iter_mut().enumerate() {
                let (a, b) = (corner(f.vertices[e]), corner(f.vertices[(e + 1) % 3]));
                let key = if a <= b { (a, b) } else { (b, a) };
                if let Some(other) = edges[&key].iter().find(|&&j| j != i) {
                    *o = *other as i16;
                }
            }
            out
        })
        .collect();
    FaceData { planes, adjacent, area }
}

/// A mesh's box, the mean of its places and how far the farthest is from
/// that, as BioWare's compiler has them: the box holds the mesh's origin
/// too, and a place counts once however many corners (vertices) meet
/// there.
pub(crate) fn bounds(vertices: &[Vec3]) -> (Vec3, Vec3, Vec3, f32) {
    if vertices.is_empty() {
        return ([0.0; 3], [0.0; 3], [0.0; 3], 0.0);
    }
    let (mut min, mut max, mut sum) = ([0.0f32; 3], [0.0f32; 3], [0.0f64; 3]);
    let mut places = std::collections::HashSet::new();
    for v in vertices {
        for k in 0..3 {
            min[k] = min[k].min(v[k]);
            max[k] = max[k].max(v[k]);
        }
        if places.insert(v.map(f32::to_bits)) {
            for k in 0..3 {
                sum[k] += f64::from(v[k]);
            }
        }
    }
    let average = sum.map(|s| (s / places.len() as f64) as f32);
    let radius = vertices.iter().map(|v| length(sub(*v, average))).fold(0.0, f32::max);
    (min, max, average, radius)
}

/// The numbers the game knows a model's nodes by, and how many it counts.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartNumbers {
    /// Per node of the model, in its order.
    pub numbers: Vec<i32>,
    /// The model's node count: past its supermodel's.
    pub count: u32,
}

impl PartNumbers {
    /// The numbers a compiled model holds (its nodes in pre-order, as
    /// `mg-mdl` reads them), or nothing for anything else. A supermodel's
    /// are best taken from its file: BioWare's compiler numbered nodes in
    /// their file's order, which a compiled model's tree does not keep.
    pub fn read(data: &[u8]) -> Option<PartNumbers> {
        if !crate::is_binary(data) {
            return None;
        }
        let m = data.get(12..)?;
        let u32_at = |at: usize| -> Option<u32> {
            let b = m.get(at..at.checked_add(4)?)?;
            Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let mut numbers = Vec::new();
        let mut stack = vec![u32_at(0x48)? as usize];
        while let Some(at) = stack.pop() {
            if numbers.len() > 100_000 {
                return None;
            }
            numbers.push(u32_at(at + 0x1C)? as i32);
            let (list, count) = (u32_at(at + 0x48)? as usize, u32_at(at + 0x4C)? as usize);
            for i in (0..count.min(100_000)).rev() {
                stack.push(u32_at(list + 4 * i)? as usize);
            }
        }
        Some(PartNumbers { numbers, count: u32_at(0x4C)? })
    }
}

/// A model's part numbers, as the compilers give them: its nodes in order,
/// after its supermodel's. A node the supermodel has at the same place by
/// name (walked from the roots, child by name) takes the supermodel's
/// number, so the supermodel's animations find it; a node it lacks there
/// has none (−1), and what is under such a node keeps its own.
pub fn part_numbers(model: &Model, supermodel: Option<(&Model, &PartNumbers)>) -> PartNumbers {
    let order: Vec<usize> = (0..model.nodes.len()).collect();
    number_nodes(model, &order, supermodel, None)
}

/// Pairs of nodes two models have at the same place by name, walked from
/// the roots child by name (case-insensitive); and the first model's nodes
/// whose parent is paired while they are not.
fn same_places(model: &Model, other: &Model) -> (Vec<(usize, usize)>, Vec<usize>) {
    let (mut same, mut lacking) = (Vec::new(), Vec::new());
    if model.nodes.is_empty() || other.nodes.is_empty() {
        return (same, lacking);
    }
    let mut pairs = vec![(0usize, 0usize)];
    while let Some((own, theirs)) = pairs.pop() {
        same.push((own, theirs));
        for &c in &model.nodes[own].children {
            let found = other.nodes[theirs]
                .children
                .iter()
                .find(|&&s| other.nodes[s].name.eq_ignore_ascii_case(&model.nodes[c].name));
            match found {
                Some(&s) => pairs.push((c, s)),
                None => lacking.push(c),
            }
        }
    }
    (same, lacking)
}

/// A model's part numbers when it is compiled. Its nodes are numbered in
/// `order` (their order in its text: indices of `model.nodes`), after its
/// supermodel's, and take the supermodel's numbers where it has them
/// ([`part_numbers`]). Then, where the model was compiled before
/// (`existing`: the compiled model of its name and its numbers), a node it
/// had at the same place by name keeps the number it had, and new nodes
/// are numbered past the old count: the game finds a supermodel's
/// animations by these numbers, so other models compiled against the old
/// ones go on working.
pub fn number_nodes(
    model: &Model,
    order: &[usize],
    supermodel: Option<(&Model, &PartNumbers)>,
    existing: Option<(&Model, &PartNumbers)>,
) -> PartNumbers {
    let count = model.nodes.len();
    // What the model had before, by node.
    let mut kept: Vec<Option<i32>> = vec![None; count];
    if let Some((old, theirs)) = existing {
        for (own, other) in same_places(model, old).0 {
            kept[own] = theirs.numbers.get(other).copied();
        }
    }
    let first =
        supermodel.map_or(0, |(_, p)| p.count + 1).max(existing.map_or(0, |(_, p)| p.count));
    // The others, in the text's order (then any the order left out).
    let mut numbers = vec![-1i32; count];
    let mut next = first as i32;
    let rest = (0..count).filter(|i| !order.contains(i));
    for i in order.iter().copied().filter(|&i| i < count).chain(rest) {
        if kept[i].is_none() && numbers[i] < 0 {
            numbers[i] = next;
            next += 1;
        }
    }
    if let Some((sup, theirs)) = supermodel {
        let (same, lacking) = same_places(model, sup);
        for (own, other) in same {
            if let Some(n) = theirs.numbers.get(other) {
                numbers[own] = *n;
            }
        }
        for own in lacking {
            numbers[own] = -1;
        }
    }
    for (number, old) in numbers.iter_mut().zip(&kept) {
        if let Some(n) = old {
            *number = *n;
        }
    }
    // The count it had, unless nodes were added (a model compiled against
    // a smaller supermodel than today's keeps its smaller count).
    let total = match existing {
        Some((_, theirs)) if next as u32 == first => theirs.count,
        Some((_, theirs)) => theirs.count.max(next as u32),
        None => first + count as u32,
    };
    PartNumbers { numbers, count: total }
}

struct Writer<'a> {
    model: &'a Model,
    parts: &'a PartNumbers,
    data: Data,
    raw: Raw,
}

impl Writer<'_> {
    fn check_name(node: &str, max: usize) -> Result<(), CompileError> {
        if node.len() > max {
            return Err(CompileError::NameTooLong { node: node.to_string(), max });
        }
        Ok(())
    }

    /// A node's controller keys and their data: one row at time 0 for a
    /// model's node, the keys for an animation's.
    fn controllers(
        &mut self,
        at: usize,
        node: &str,
        node_flags: u32,
        list: &[&Controller],
    ) -> Result<(), CompileError> {
        if list.is_empty() {
            return Ok(());
        }
        let mut data: Vec<f32> = Vec::new();
        let keys = self.data.alloc(list.len() * 12);
        for (i, c) in list.iter().enumerate() {
            let id = controller_id(node_flags, &c.name).ok_or_else(|| {
                CompileError::UnknownController { node: node.to_string(), name: c.name.clone() }
            })?;
            let bezier = !c.handles.is_empty();
            let rows = c.times.len();
            let invalid = |what: &str| CompileError::Invalid {
                node: node.to_string(),
                what: format!("{}: {what}", c.name),
            };
            if c.values.len() != rows * c.columns {
                return Err(invalid("its values are not its keys times its columns"));
            }
            if bezier && c.handles.len() != rows * c.columns * 2 {
                return Err(invalid("its handles are not two per value"));
            }
            let time_at = data.len();
            data.extend_from_slice(&c.times);
            let value_at = data.len();
            for row in 0..rows {
                data.extend_from_slice(&c.values[row * c.columns..(row + 1) * c.columns]);
                if bezier {
                    data.extend_from_slice(
                        &c.handles[row * c.columns * 2..(row + 1) * c.columns * 2],
                    );
                }
            }
            if rows > i16::MAX as usize || data.len() > i16::MAX as usize {
                return Err(invalid("too many keys for one node"));
            }
            let k = keys + i * 12;
            self.data.u32(k, id);
            self.data.u16(k + 4, rows as u16);
            self.data.u16(k + 6, time_at as u16);
            self.data.u16(k + 8, value_at as u16);
            // No values at all: −1 (a `detonatekey`'s times alone).
            let columns = match c.columns {
                0 => -1i8,
                n => (n as i8) | if bezier { 0x10 } else { 0 },
            };
            self.data.u8(k + 10, columns as u8);
        }
        self.data.array(at + 0x54, keys, list.len());
        self.data.float_array(at + 0x60, &data);
        Ok(())
    }

    fn mesh(&mut self, at: usize, name: &str, m: &Mesh) -> Result<(), CompileError> {
        let count = m.vertices.len();
        if count > usize::from(u16::MAX) {
            return Err(CompileError::TooManyVertices { node: name.to_string(), count });
        }
        if m.faces.len() > i16::MAX as usize {
            return Err(CompileError::TooManyFaces {
                node: name.to_string(),
                count: m.faces.len(),
            });
        }
        let invalid =
            |what: &str| CompileError::Invalid { node: name.to_string(), what: what.to_string() };
        let triangles = if m.drawn.is_empty() {
            m.faces.iter().map(|f| f.vertices).collect()
        } else {
            m.drawn.clone()
        };
        if triangles.iter().flatten().any(|&v| v as usize >= count) {
            return Err(invalid("a face names a vertex the mesh does not have"));
        }
        let d = &mut self.data;
        // Faces, with what the game derives nothing of itself.
        let derived = face_data(m);
        let faces = d.alloc(m.faces.len() * 32);
        for (i, f) in m.faces.iter().enumerate() {
            let o = faces + i * 32;
            d.vec3(o, derived.planes[i].0);
            d.f32(o + 0x0C, derived.planes[i].1);
            d.u32(o + 0x10, f.material);
            for e in 0..3 {
                d.u16(o + 0x14 + 2 * e, derived.adjacent[i][e] as u16);
                d.u16(o + 0x1A + 2 * e, f.vertices[e] as u16);
            }
        }
        d.array(at + 0x78, faces, m.faces.len());
        let (min, max, average, radius) = bounds(&m.vertices);
        d.vec3(at + 0x84, min);
        d.vec3(at + 0x90, max);
        d.f32(at + 0x9C, radius);
        d.vec3(at + 0xA0, average);
        d.vec3(at + 0xAC, m.diffuse);
        d.vec3(at + 0xB8, m.ambient);
        d.vec3(at + 0xC4, m.specular);
        d.f32(at + 0xD0, m.shininess);
        d.u32(at + 0xD4, u32::from(m.shadow));
        d.u32(at + 0xD8, u32::from(m.beaming));
        d.u32(at + 0xDC, u32::from(m.render));
        d.u32(at + 0xE0, m.transparency_hint);
        d.u32(
            at + 0xE4,
            match m.renderhint.as_deref().map(str::to_ascii_lowercase).as_deref() {
                Some("none") => 1,
                Some("normalandspecmapped") => 2,
                Some("normaltangents") => 3,
                _ => 0,
            },
        );
        // The fourth slot is the `materialname`'s, as the game's compiler
        // has it.
        for (i, t) in m.textures.iter().take(3).enumerate() {
            d.name(at + 0xE8 + 64 * i, 64, t.as_deref().unwrap_or(""));
        }
        d.name(at + 0xE8 + 64 * 3, 64, m.material.as_deref().unwrap_or(""));
        d.u32(at + 0x1E8, m.tilefade);
        // What is drawn: one list of indices, and its length.
        let indices = triangles.len() * 3;
        if indices > 0 {
            let counts = d.alloc(4);
            d.u32(counts, indices as u32);
            d.array(at + 0x204, counts, 1);
            let lists = d.alloc(4);
            let bytes: Vec<u8> =
                triangles.iter().flatten().flat_map(|&v| (v as u16).to_le_bytes()).collect();
            let list = self.raw.bytes(&bytes);
            d.u32(lists, list);
            d.array(at + 0x210, lists, 1);
        }
        d.u32(at + 0x21C, NO_RAW);
        d.u8(at + 0x224, if indices > 0 { 3 } else { 0 });
        let raw = &mut self.raw;
        let stream = |raw: &mut Raw, present: bool, v: &mut dyn Iterator<Item = f32>| {
            if present { raw.floats(v) } else { NO_RAW }
        };
        d.u32(at + 0x22C, stream(raw, count > 0, &mut m.vertices.iter().flatten().copied()));
        d.u16(at + 0x230, count as u16);
        let sets = m.uvs.iter().filter(|u| !u.is_empty()).count();
        d.u16(at + 0x232, sets as u16);
        for (i, uv) in m.uvs.iter().enumerate() {
            if !uv.is_empty() && uv.len() != count {
                return Err(invalid("texture coordinates are not one per vertex"));
            }
            let has = count > 0 && !uv.is_empty();
            d.u32(at + 0x234 + 4 * i, stream(raw, has, &mut uv.iter().flatten().copied()));
        }
        if !m.normals.is_empty() && m.normals.len() != count {
            return Err(invalid("normals are not one per vertex"));
        }
        let has = count > 0 && !m.normals.is_empty();
        d.u32(at + 0x244, stream(raw, has, &mut m.normals.iter().flatten().copied()));
        if !m.colors.is_empty() && m.colors.len() != count {
            return Err(invalid("colors are not one per vertex"));
        }
        d.u32(
            at + 0x248,
            if count > 0 && !m.colors.is_empty() { raw.bytes(&m.colors.concat()) } else { NO_RAW },
        );
        // Tangents and the bitangents' signs, where the game's compiler
        // puts them for a render hint.
        for slot in 0..5 {
            d.u32(at + 0x24C + 4 * slot, NO_RAW);
        }
        d.u32(at + 0x260, NO_RAW);
        if count > 0 && !m.tangents.is_empty() {
            if m.tangents.len() != count {
                return Err(invalid("tangents are not one per vertex"));
            }
            let t = raw.floats(m.tangents.iter().flat_map(|t| [t[0], t[1], t[2]]));
            d.u32(at + 0x258, t);
            let signs = raw.floats(m.tangents.iter().map(|t| t[3]));
            d.u32(at + 0x260, signs);
        }
        d.u8(at + 0x265, u8::from(m.rotate_texture));
        d.f32(at + 0x268, derived.area);
        Ok(())
    }

    fn skin(&mut self, at: usize, name: &str, count: usize, s: &Skin) -> Result<(), CompileError> {
        let invalid = |what: String| CompileError::Invalid { node: name.to_string(), what };
        if s.bones.len() > EE_SKIN_BONES {
            return Err(invalid(format!(
                "{} bones (a skin holds at most {EE_SKIN_BONES})",
                s.bones.len()
            )));
        }
        if !s.weights.is_empty() && s.weights.len() != count {
            return Err(invalid("weights are not one set per vertex".into()));
        }
        let nodes = self.model.nodes.len();
        // Which bone each node is: a bone's node, the bones the weights use
        // first (a slot no node was given reads back as node 0's).
        let mut used = vec![false; s.bones.len()];
        for w in s.weights.iter().flatten() {
            if w.1 != 0.0 && usize::from(w.0) < used.len() {
                used[usize::from(w.0)] = true;
            }
        }
        let mut map = vec![-1i16; nodes];
        for pass in [true, false] {
            for (bone, &node) in s.bones.iter().enumerate() {
                if used[bone] == pass && node < nodes && map[node] < 0 {
                    map[node] = bone as i16;
                }
            }
        }
        let d = &mut self.data;
        if !s.weights.is_empty() {
            let weights = self.raw.floats(s.weights.iter().flatten().map(|w| w.1));
            d.u32(at + 0x27C, weights);
            let refs: Vec<u8> = s
                .weights
                .iter()
                .flatten()
                .flat_map(|w| {
                    // An unused slot: no bone.
                    let bone = if w.0 == 0 && w.1 == 0.0 { -1i16 } else { w.0 as i16 };
                    bone.to_le_bytes()
                })
                .collect();
            let refs = self.raw.bytes(&refs);
            d.u32(at + 0x280, refs);
        } else {
            d.u32(at + 0x27C, NO_RAW);
            d.u32(at + 0x280, NO_RAW);
        }
        let map_at = d.alloc(nodes * 2);
        for (i, b) in map.iter().enumerate() {
            d.u16(map_at + 2 * i, *b as u16);
        }
        d.u32(at + 0x284, map_at as u32);
        d.u32(at + 0x288, nodes as u32);
        // Each node's inverse bind: rotations stored w, x, y, z.
        let q = d.alloc(s.inverse_bind.len() * 16);
        let t = d.alloc(s.inverse_bind.len() * 12);
        for (i, (rot, pos)) in s.inverse_bind.iter().enumerate() {
            d.floats(q + i * 16, &[rot[3], rot[0], rot[1], rot[2]]);
            d.vec3(t + i * 12, *pos);
        }
        d.array(at + 0x28C, q, s.inverse_bind.len());
        d.array(at + 0x298, t, s.inverse_bind.len());
        let constants = d.alloc(s.inverse_bind.len() * 4);
        d.array(at + 0x2A4, constants, s.inverse_bind.len());
        // Each bone's node, in the structure itself.
        let slots = if s.bones.len() > OLD_SKIN_BONES { EE_SKIN_BONES } else { OLD_SKIN_BONES };
        for slot in 0..slots {
            let node = s.bones.get(slot).map_or(-1i16, |&n| n as i16);
            d.u16(at + 0x2B0 + 2 * slot, node as u16);
        }
        Ok(())
    }

    /// A walkmesh's tree from its entries in pre-order (a leaf holds a
    /// face; another entry has the two that follow it): the root's offset.
    fn aabb(&mut self, entries: &[AabbEntry]) -> usize {
        fn place(d: &mut Data, entries: &[AabbEntry], next: &mut usize) -> usize {
            let Some(e) = entries.get(*next) else { return 0 };
            *next += 1;
            let at = d.alloc(40);
            d.vec3(at, e.min);
            d.vec3(at + 12, e.max);
            d.u32(at + 0x20, e.face as u32);
            if e.face < 0 {
                let left = place(d, entries, next);
                d.u32(at + 0x18, left as u32);
                let right = place(d, entries, next);
                d.u32(at + 0x1C, right as u32);
                // The plane that parts its children: the box's longest
                // side.
                let size = sub(e.max, e.min);
                let axis = (0..3).fold(0, |a, k| if size[k] > size[a] { k } else { a });
                d.u32(at + 0x24, 1 << axis);
            }
            at
        }
        let mut next = 0;
        place(&mut self.data, entries, &mut next)
    }

    fn light(&mut self, at: usize, l: &Light) {
        let d = &mut self.data;
        d.f32(at + 0x70, l.flare_radius);
        d.float_array(at + 0x80, &l.flare_sizes);
        d.float_array(at + 0x8C, &l.flare_positions);
        let shifts: Vec<f32> = l.flare_color_shifts.iter().flatten().copied().collect();
        let offset = d.alloc(shifts.len() * 4);
        d.floats(offset, &shifts);
        d.array(at + 0x98, offset, l.flare_color_shifts.len());
        let names = d.alloc(l.flare_textures.len() * 4);
        for (i, t) in l.flare_textures.iter().enumerate() {
            let s = d.alloc(t.len() + 1);
            d.name(s, t.len() + 1, t);
            d.u32(names + 4 * i, s as u32);
        }
        d.array(at + 0xA4, names, l.flare_textures.len());
        d.u32(at + 0xB0, l.priority);
        d.u32(at + 0xB4, u32::from(l.ambient_only));
        d.u32(at + 0xB8, l.dynamic_type);
        d.u32(at + 0xBC, u32::from(l.affect_dynamic));
        d.u32(at + 0xC0, u32::from(l.shadow));
        d.u32(at + 0xC4, u32::from(l.generate_flare));
        d.u32(at + 0xC8, u32::from(l.fading));
    }

    fn emitter(&mut self, at: usize, e: &Emitter) {
        let d = &mut self.data;
        d.f32(at + 0x70, e.deadspace);
        d.f32(at + 0x74, e.blast_radius);
        d.f32(at + 0x78, e.blast_length);
        d.u32(at + 0x7C, e.xgrid);
        d.u32(at + 0x80, e.ygrid);
        d.u32(at + 0x84, e.spawntype);
        d.name(at + 0x88, 32, &e.update);
        d.name(at + 0xA8, 32, &e.render);
        d.name(at + 0xC8, 32, &e.blend);
        d.name(at + 0xE8, 64, e.texture.as_deref().unwrap_or(""));
        d.name(at + 0x128, 16, e.chunk.as_deref().unwrap_or(""));
        d.u32(at + 0x138, u32::from(e.two_sided));
        d.u32(at + 0x13C, u32::from(e.looping));
        d.u16(at + 0x140, e.render_order as u16);
        d.u32(at + 0x144, e.flags);
    }

    /// The node header and what its type adds; the children's place is
    /// left for the caller.
    #[allow(clippy::too_many_arguments)]
    fn node_header(
        &mut self,
        name: &str,
        node_flags: u32,
        bones: usize,
        part: i32,
        inherit_color: bool,
        children: usize,
    ) -> Result<(usize, usize), CompileError> {
        Self::check_name(name, 31)?;
        let at = self.data.alloc(node_size(node_flags, bones));
        let d = &mut self.data;
        d.u32(at + 0x18, u32::from(inherit_color));
        d.u32(at + 0x1C, part as u32);
        d.name(at + 0x20, 32, name);
        d.u32(at + 0x6C, node_flags);
        let list = d.alloc(children * 4);
        d.array(at + 0x48, list, children);
        Ok((at, list))
    }

    /// A node of the model and, after it, its children.
    fn model_node(&mut self, index: usize) -> Result<usize, CompileError> {
        let model = self.model;
        let node: &Node = &model.nodes[index];
        let node_flags = node_flags(&node.kind);
        let bones = match &node.kind {
            NodeKind::Mesh(m) => match &m.extra {
                MeshExtra::Skin(s) => s.bones.len(),
                _ => 0,
            },
            _ => 0,
        };
        let part = self.parts.numbers.get(index).copied().unwrap_or(index as i32);
        let (at, list) = self.node_header(
            &node.name,
            node_flags,
            bones,
            part,
            node.inherit_color,
            node.children.len(),
        )?;
        // Where it is at rest, then what else it sets.
        let position = Controller::constant("position", &node.position);
        let orientation = Controller::constant("orientation", &node.orientation);
        let scale = Controller::constant("scale", &[node.scale]);
        let mut controllers = vec![&position, &orientation];
        if node.scale != 1.0 {
            controllers.push(&scale);
        }
        controllers.extend(&node.controllers);
        self.controllers(at, &node.name, node_flags, &controllers)?;
        match &node.kind {
            NodeKind::Dummy | NodeKind::Camera => {}
            NodeKind::Light(l) => self.light(at, l),
            NodeKind::Emitter(e) => self.emitter(at, e),
            NodeKind::Reference(r) => {
                self.data.name(at + 0x70, 64, r.model.as_deref().unwrap_or(""));
                self.data.u32(at + 0xB0, u32::from(r.reattachable));
            }
            NodeKind::Mesh(m) => {
                self.mesh(at, &node.name, m)?;
                match &m.extra {
                    MeshExtra::None => {}
                    MeshExtra::Skin(s) => self.skin(at, &node.name, m.vertices.len(), s)?,
                    MeshExtra::Dangly(dangly) => {
                        self.data.float_array(at + 0x270, &dangly.constraints);
                        self.data.f32(at + 0x27C, dangly.displacement);
                        self.data.f32(at + 0x280, dangly.tightness);
                        self.data.f32(at + 0x284, dangly.period);
                    }
                    MeshExtra::Anim(a) => self.data.f32(at + 0x270, a.sample_period),
                    MeshExtra::Aabb(entries) => {
                        let root = self.aabb(entries);
                        self.data.u32(at + 0x270, root as u32);
                    }
                }
            }
        }
        for (i, &child) in node.children.iter().enumerate() {
            let c = self.model_node(child)?;
            self.data.u32(list + 4 * i, c as u32);
        }
        Ok(at)
    }

    /// A node of an animation and, after it, its children: of the type of
    /// the model's node of its name, which says what its controllers are.
    fn anim_node(
        &mut self,
        anim: &Animation,
        children: &[Vec<usize>],
        index: usize,
    ) -> Result<usize, CompileError> {
        let node: &AnimNode = &anim.nodes[index];
        let own = self.model.node(&node.name);
        let mut node_flags = match own {
            Some(i) => node_flags(&self.model.nodes[i].kind),
            None => flags_for_controllers(&node.controllers),
        };
        // An animation's node carries no skin or walkmesh of its own.
        node_flags &= !(flags::SKIN | flags::AABB);
        if node.anim_mesh.is_some() {
            node_flags = flags::HEADER | flags::MESH | flags::ANIM;
        }
        let part = own.and_then(|i| self.parts.numbers.get(i).copied()).unwrap_or(-1);
        let (at, list) =
            self.node_header(&node.name, node_flags, 0, part, false, children[index].len())?;
        let controllers: Vec<&Controller> = node.controllers.iter().collect();
        self.controllers(at, &node.name, node_flags, &controllers)?;
        if node_flags & flags::MESH != 0 {
            // An empty mesh: no stream points anywhere.
            let d = &mut self.data;
            d.u32(at + 0x21C, NO_RAW);
            for offset in (0x22C..=0x260).step_by(4).filter(|&o| o != 0x230) {
                d.u32(at + offset, NO_RAW);
            }
            if node_flags & flags::SKIN == 0 && node_flags & flags::DANGLY == 0 {
                // (Nothing of a plain mesh's is past its header.)
            }
        }
        if let Some(sets) = &node.anim_mesh {
            let d = &mut self.data;
            let count = sets
                .vertex_sets
                .first()
                .map(Vec::len)
                .or_else(|| sets.uv_sets.first().map(Vec::len))
                .unwrap_or(0);
            let invalid = |what: &str| CompileError::Invalid {
                node: node.name.clone(),
                what: what.to_string(),
            };
            if sets.vertex_sets.iter().any(|s| s.len() != count)
                || sets.uv_sets.iter().any(|s| s.len() != count)
            {
                return Err(invalid("its animated sets are not of one size"));
            }
            if count > usize::from(u16::MAX) {
                return Err(CompileError::TooManyVertices { node: node.name.clone(), count });
            }
            // The model's mesh of this name says how many vertices a set
            // has; an animation's own count is for one it does not.
            let models =
                own.and_then(|i| self.model.nodes[i].mesh()).map_or(0, |m| m.vertices.len());
            if models != count {
                d.u16(at + 0x230, count as u16);
            }
            d.f32(at + 0x270, sets.sample_period);
            // Vertex-major: a vertex's value in each set, then the next's.
            let (vsets, tsets) = (sets.vertex_sets.len(), sets.uv_sets.len());
            let verts = d.alloc(count * vsets * 12);
            for v in 0..count {
                for (s, set) in sets.vertex_sets.iter().enumerate() {
                    d.vec3(verts + (v * vsets + s) * 12, set[v]);
                }
            }
            let tverts = d.alloc(count * tsets * 8);
            for v in 0..count {
                for (s, set) in sets.uv_sets.iter().enumerate() {
                    d.floats(tverts + (v * tsets + s) * 8, &set[v]);
                }
            }
            d.u32(at + 0x298, verts as u32);
            d.u32(at + 0x29C, tverts as u32);
            d.u32(at + 0x2A0, vsets as u32);
            d.u32(at + 0x2A4, tsets as u32);
        }
        for (i, &child) in children[index].iter().enumerate() {
            let c = self.anim_node(anim, children, child)?;
            self.data.u32(list + 4 * i, c as u32);
        }
        Ok(at)
    }

    fn animation(&mut self, anim: &Animation) -> Result<usize, CompileError> {
        Self::check_name(&anim.name, 63)?;
        Self::check_name(&anim.animroot, 63)?;
        let at = self.data.alloc(0xC4);
        let d = &mut self.data;
        d.name(at + 0x08, 64, &anim.name);
        d.u32(at + 0x4C, anim.nodes.len() as u32);
        d.u8(at + 0x6C, 5);
        d.f32(at + 0x70, anim.length);
        d.f32(at + 0x74, anim.transtime);
        d.name(at + 0x78, 64, &anim.animroot);
        let events = d.alloc(anim.events.len() * 36);
        for (i, (time, name)) in anim.events.iter().enumerate() {
            d.f32(events + i * 36, *time);
            d.name(events + i * 36 + 4, 32, name);
        }
        d.array(at + 0xB8, events, anim.events.len());
        if !anim.nodes.is_empty() {
            let mut children = vec![Vec::new(); anim.nodes.len()];
            for (i, n) in anim.nodes.iter().enumerate() {
                if let Some(p) = n.parent.filter(|&p| p < i) {
                    children[p].push(i);
                }
            }
            let root = self.anim_node(anim, &children, 0)?;
            self.data.u32(at + 0x48, root as u32);
        }
        Ok(at)
    }
}

/// Writes a model as a compiled (binary) one. `parts`: its nodes' numbers
/// ([`part_numbers`], from its supermodel's).
pub fn write(model: &Model, parts: &PartNumbers) -> Result<Vec<u8>, CompileError> {
    if model.nodes.is_empty() {
        return Err(CompileError::Empty);
    }
    Writer::check_name(&model.name, 63)?;
    let mut w = Writer { model, parts, data: Data::default(), raw: Raw::default() };
    let header = w.data.alloc(0xE8);
    let d = &mut w.data;
    d.name(0x08, 64, &model.name);
    d.u32(0x4C, parts.count.max(model.nodes.len() as u32));
    d.u8(0x6C, 2);
    d.u8(
        0x72,
        match model.classification {
            Classification::Other => 0,
            Classification::Effect => 1,
            Classification::Tile => 2,
            Classification::Character => 4,
            Classification::Door => 8,
        },
    );
    d.u8(0x73, u8::from(!model.ignore_fog));
    // The box and the radius every compiler gives a model, wider with an
    // emitter (its particles go far).
    d.vec3(0x88, [-5.0, -5.0, -1.0]);
    d.vec3(0x94, [5.0, 5.0, 10.0]);
    let emits = model.nodes.iter().any(|n| matches!(n.kind, NodeKind::Emitter(_)));
    d.f32(0xA0, if emits { 40.0 } else { 7.0 });
    d.f32(0xA4, model.animation_scale);
    d.name(0xA8, 64, model.supermodel.as_deref().unwrap_or("NULL"));
    debug_assert_eq!(header, 0);
    let animations = w.data.alloc(model.animations.len() * 4);
    w.data.array(0x78, animations, model.animations.len());
    let root = w.model_node(0)?;
    w.data.u32(0x48, root as u32);
    for (i, anim) in model.animations.iter().enumerate() {
        let at = w.animation(anim)?;
        w.data.u32(animations + 4 * i, at as u32);
    }
    let mut out = Vec::with_capacity(12 + w.data.0.len() + w.raw.0.len());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(w.data.0.len() as u32).to_le_bytes());
    out.extend_from_slice(&(w.raw.0.len() as u32).to_le_bytes());
    out.extend_from_slice(&w.data.0);
    out.extend_from_slice(&w.raw.0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = "newmodel m\nsetsupermodel m NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom m\n\
node dummy m\n  parent NULL\nendnode\n\
node trimesh box\n  parent m\n  position 0 0 1\n  orientation 0 0 1 1.5\n  bitmap wood\n  \
verts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  \
tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n  \
faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n\
node light glow\n  parent m\n  color 1 0.5 0\n  radius 4\nendnode\n\
node emitter smoke\n  parent box\n  birthrate 5\n  update Fountain\nendnode\n\
endmodelgeom m\n\
newanim wave m\n  length 1\n  transtime 0.25\n  animroot m\n  event 0.5 hit\n\
  node dummy m\n    parent NULL\n  endnode\n\
  node trimesh box\n    parent m\n    positionkey\n      0 0 0 1\n      1 0 0 2\n    endlist\n  endnode\n\
doneanim wave m\ndonemodel m\n";

    #[test]
    fn a_model_written_reads_back_the_same() {
        let model = Model::read(MODEL.as_bytes()).unwrap();
        let bytes = write(&model, &part_numbers(&model, None)).unwrap();
        assert!(crate::is_binary(&bytes));
        let back = Model::read(&bytes).unwrap();
        // (The ASCII reader keeps where vertices came from; a binary's are
        // its own.)
        let bare = |m: &Model| {
            let mut m = m.clone();
            // (Nor does a text hold part numbers: a binary's are read.)
            for n in &mut m.nodes {
                n.part = None;
                if let NodeKind::Mesh(mesh) = &mut n.kind {
                    mesh.source.clear();
                    mesh.source_uv.clear();
                }
            }
            for n in m.animations.iter_mut().flat_map(|a| &mut a.nodes) {
                n.part = None;
            }
            m
        };
        assert_eq!(bare(&back), bare(&model));
        // And once more, byte for byte.
        let again = write(&back, &part_numbers(&back, None)).unwrap();
        assert_eq!(again, bytes);
    }

    #[test]
    fn faces_know_their_planes_and_neighbours() {
        let model = Model::read(MODEL.as_bytes()).unwrap();
        let mesh = model.nodes[model.node("box").unwrap()].mesh().unwrap();
        let d = face_data(mesh);
        assert_eq!(d.planes[0], ([0.0, 0.0, 1.0], 0.0));
        // The two triangles of the square share the diagonal.
        assert!(d.adjacent[0].contains(&1) && d.adjacent[1].contains(&0));
        assert_eq!(d.adjacent[0].iter().filter(|&&a| a < 0).count(), 2);
        assert!((d.area - 1.0).abs() < 1e-6);
        let (min, max, average, radius) = bounds(&mesh.vertices);
        assert_eq!((min, max), ([0.0, 0.0, 0.0], [1.0, 1.0, 0.0]));
        assert_eq!(average, [0.5, 0.5, 0.0]);
        // Away from the origin, the box still reaches it.
        let (min, max, ..) = bounds(&[[1.0, 2.0, 3.0], [2.0, 3.0, 4.0], [1.0, 2.0, 3.0]]);
        assert_eq!((min, max), ([0.0; 3], [2.0, 3.0, 4.0]));
        assert!((radius - 0.5f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn part_numbers_follow_the_supermodel() {
        let sup = Model::read(MODEL.as_bytes()).unwrap();
        let sup_parts = part_numbers(&sup, None);
        assert_eq!(sup_parts.numbers, [0, 1, 2, 3]);
        assert_eq!(sup_parts.count, 4);
        // Its own root and `box` are the supermodel's; `extra` is new.
        let text = MODEL
            .replace("node light glow", "node dummy extra\n  parent m\nendnode\nnode light glow")
            .replace("node emitter smoke\n  parent box", "node emitter smoke\n  parent extra");
        let model = Model::read(text.as_bytes()).unwrap();
        let parts = part_numbers(&model, Some((&sup, &sup_parts)));
        let of = |name: &str| parts.numbers[model.node(name).unwrap()];
        assert_eq!((of("m"), of("box"), of("glow")), (0, 1, 3));
        // New under a node the supermodel has: no number; under that, its own.
        assert_eq!(of("extra"), -1);
        assert!(of("smoke") > 4, "{:?}", parts.numbers);
        // A compiled model's numbers are read back from it.
        let bytes = write(&model, &parts).unwrap();
        assert_eq!(PartNumbers::read(&bytes), Some(parts.clone()));
        assert_eq!(PartNumbers::read(MODEL.as_bytes()), None);
        // Compiled again with its nodes in another order and one more: the
        // old ones keep their numbers, the new one is past the old count.
        let again = text.replace(
            "node light glow",
            "node dummy more\n  parent extra\nendnode\nnode light glow",
        );
        let again = Model::read(again.as_bytes()).unwrap();
        let reversed: Vec<usize> = (0..again.nodes.len()).rev().collect();
        let kept =
            number_nodes(&again, &reversed, Some((&sup, &sup_parts)), Some((&model, &parts)));
        for (i, n) in again.nodes.iter().enumerate().filter(|(_, n)| n.name != "more") {
            assert_eq!(kept.numbers[i], of(&n.name), "{}", n.name);
        }
        let more = kept.numbers[again.node("more").unwrap()];
        assert!(more >= parts.count as i32 && kept.count > more as u32, "{kept:?}");
        // New altogether, numbered in the order of its text.
        let fresh = number_nodes(&sup, &[0, 3, 2, 1], None, None);
        assert_eq!(fresh.numbers, [0, 3, 2, 1]);
        assert_eq!(parts.count, 5 + model.nodes.len() as u32);
    }

    #[test]
    fn what_cannot_be_held_is_refused() {
        let mut model = Model::read(MODEL.as_bytes()).unwrap();
        let parts = part_numbers(&model, None);
        model.nodes[1].name = "a".repeat(40);
        assert!(matches!(write(&model, &parts), Err(CompileError::NameTooLong { .. })));
        let mut model = Model::read(MODEL.as_bytes()).unwrap();
        model.nodes[1].controllers.push(Controller::constant("birthrate", &[1.0]));
        assert!(matches!(write(&model, &parts), Err(CompileError::UnknownController { .. })));
        assert_eq!(write(&Model::default(), &parts), Err(CompileError::Empty));
    }
}
