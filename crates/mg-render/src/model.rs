//! A model on the GPU: each drawn mesh's vertices and indices (in the
//! device's shared buffers, `pool.rs`), its material, and the node it
//! hangs from.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use mg_mdl::{Mesh, MeshExtra, Model, NodeKind};

use crate::Gpu;

/// A vertex as the shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// The tangent and the bitangent's sign: the model's (`tangents`, or
    /// compiled with a render hint), else made from its texture coordinates
    /// ([`GpuMesh::tangents_made`]: the game's shaders read them, as the
    /// engine makes them; Moonglow's own takes the tangent frame from
    /// screen-space derivatives then); zero without texture coordinates.
    pub tangent: [f32; 4],
    /// The model's vertex color (`colors`), white without: the game hands
    /// it to custom shaders alone (`vCustomColor`).
    pub color: [u8; 4],
    /// The other texture coordinate sets (`tverts1`, `tverts2`,
    /// `tverts3`), zero without (as the client feeds a custom shader's
    /// `vTcIn1`…`vTcIn3` for a set the mesh doesn't have).
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub uv3: [f32; 2],
}

impl Vertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 5 => Float32x4,
            6 => Unorm8x4, 7 => Float32x2
        ],
    };
}

/// A skinned vertex's bones (indices into the skin's bone list) and
/// weights.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SkinVertex {
    pub bones: [u32; 4],
    pub weights: [f32; 4],
}

impl SkinVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SkinVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![3 => Uint32x4, 4 => Float32x4],
    };
}

/// A skinned mesh's bones and their weights on the GPU.
#[derive(Debug, Clone)]
pub struct GpuSkin {
    /// Bone nodes.
    pub bones: Vec<usize>,
    /// Per bone: from the skin node's space to the bone's, in the bind
    /// pose. Compiled models store it (and a few bind in a pose other than
    /// the rest pose); for ASCII models it comes from the rest pose.
    pub inverse_bind: Vec<Mat4>,
    /// The bones and weights of the mesh's chunk: the mesh's at its
    /// vertices' places ([`GpuMesh::base_vertex`]).
    pub vertices: wgpu::Buffer,
    /// The least and the most a vertex's weights add up to, of the
    /// vertices weighed to a bone (1 and 1 for a skin as the game's are):
    /// where its bones can carry the mesh, as the game's programs weigh
    /// them (Moonglow's own shader makes them add up to one).
    pub weighed: [f32; 2],
    /// A vertex is weighed to no bone: it stays where it is bound
    /// (Moonglow's own shader), or goes to the skin node's origin (the
    /// game's).
    pub unweighed: bool,
}

/// A mesh's material, from the model.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// The texture name (bitmap), lower case.
    pub texture: Option<String>,
    /// `texture1`–`texture3`: normal, specular and roughness maps.
    pub maps: [Option<String>; 3],
    /// An MTR named by the model (`materialname`).
    pub mtr: Option<String>,
    /// `renderhint NormalAndSpecMapped` (or `NormalTangents`).
    pub normal_mapped: bool,
    pub diffuse: Vec3,
    pub ambient: Vec3,
    pub emissive: Vec3,
    pub alpha: f32,
    pub transparency_hint: u32,
    /// `rotatetexture 1`: on a tile, its texture turns with the tile
    /// ([`crate::Instance::tile`]).
    pub rotate_texture: bool,
    /// It has texture coordinates (`tverts`): the game draws one without
    /// with `vslitnotex` (or `vslitnt_sm`).
    pub uvs: bool,
}

/// A drawn mesh.
#[derive(Debug)]
pub struct GpuMesh {
    /// The node it belongs to.
    pub node: usize,
    /// The vertices and indices of the chunk the mesh lies in, shared with
    /// other meshes (`pool.rs`): the mesh's are `index_count` indices
    /// from `first_index`, of vertices counted from `base_vertex`.
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub index_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    /// How many vertices it has.
    pub vertex_count: u32,
    pub material: Material,
    /// Local bounds.
    pub min: Vec3,
    pub max: Vec3,
    pub skin: Option<GpuSkin>,
    /// The model has no tangents for it: they were made from its texture
    /// coordinates ([`made_tangents`]).
    pub tangents_made: bool,
    /// It casts the sun's and moon's shadows: its node's `shadow` flag (on
    /// unless a model says otherwise).
    pub casts: bool,
    /// Its place in its chunk, given back when the last mesh drawing from
    /// it is dropped.
    slot: Arc<crate::pool::Slot>,
}

impl GpuMesh {
    /// The chunk its vertices and indices lie in: draws of one chunk in a
    /// row bind its buffers once.
    pub(crate) fn chunk(&self) -> u64 {
        self.slot.chunk.id
    }
}

/// A model's meshes on the GPU.
#[derive(Debug)]
pub struct GpuModel {
    /// A number of its own among the models uploaded: what the renderer
    /// keeps of a model, it keeps by this.
    pub id: u64,
    pub model: Arc<Model>,
    pub meshes: Vec<GpuMesh>,
    /// The meshes that only cast shadows (`render 0`, `shadow 1`: a
    /// low-poly stand-in for what is drawn), never drawn.
    pub shadow_only: Vec<GpuMesh>,
    /// The rest pose (model-space node transforms).
    pub rest: Vec<Mat4>,
}

/// The next model's [`GpuModel::id`].
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// A skin's inverse bind pose for each of its bones, in the space of the
/// skin's node `skin_node`: as the model stores it (binary models), else
/// from the rest pose.
///
/// A bone that is no node of the model (custom content whose bone map was
/// left from another model names nodes past the model's last) holds the
/// mesh where it is: the renderer poses such a bone as unmoved.
fn inverse_binds(skin: &mg_mdl::Skin, rest: &[Mat4], skin_node: usize) -> Vec<Mat4> {
    let stored = skin.inverse_bind.len() == rest.len();
    let here = rest.get(skin_node).copied().unwrap_or(Mat4::IDENTITY);
    skin.bones
        .iter()
        .map(|&b| match (rest.get(b), skin.inverse_bind.get(b)) {
            (Some(_), Some(&(q, t))) if stored => {
                let q = Quat::from_array(q);
                let q = if q.length_squared() > 1e-12 { q.normalize() } else { Quat::IDENTITY };
                Mat4::from_rotation_translation(q, Vec3::from(t))
            }
            (Some(bone), _) => bone.inverse() * here,
            (None, _) => here,
        })
        .collect()
}

impl GpuModel {
    /// Uploads the meshes that render, and apart those that only cast
    /// shadows (`render 0`, `shadow 1`); walkmeshes and meshes neither
    /// drawn nor shadowing are skipped.
    pub fn new(gpu: &Gpu, model: Arc<Model>) -> GpuModel {
        crate::guard::fail_if_asked(&model.name);
        let rest = rest_pose(&model);
        let mut meshes = Vec::new();
        let mut shadow_only = Vec::new();
        for (i, node) in model.nodes.iter().enumerate() {
            let NodeKind::Mesh(m) = &node.kind else { continue };
            if !(m.render || m.shadow)
                || m.triangles().next().is_none()
                || matches!(m.extra, MeshExtra::Aabb(_))
            {
                continue;
            }
            let vertices = mesh_vertices(m);
            // (A face naming a vertex the mesh hasn't would draw another
            // mesh's, in the shared buffers: left out.)
            let n = vertices.len() as u32;
            let mut indices: Vec<u32> =
                m.triangles().filter(|t| t.iter().all(|&i| i < n)).flatten().collect();
            let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for p in &m.vertices {
                min = min.min(Vec3::from(*p));
                max = max.max(Vec3::from(*p));
            }
            let emissive = node.value("selfillumcolor").map_or(Vec3::ZERO, |v| {
                Vec3::new(
                    v.first().copied().unwrap_or(0.0),
                    v.get(1).copied().unwrap_or(0.0),
                    v.get(2).copied().unwrap_or(0.0),
                )
            });
            let skin = match &m.extra {
                MeshExtra::Skin(s)
                    if !s.bones.is_empty() && s.weights.len() == m.vertices.len() =>
                {
                    let data: Vec<SkinVertex> = s
                        .weights
                        .iter()
                        .map(|w| SkinVertex {
                            bones: w.map(|(b, _)| u32::from(b)),
                            weights: w.map(|(_, x)| x),
                        })
                        .collect();
                    Some((s, data))
                }
                _ => None,
            };
            // (A skin's face that has a vertex no bone weighs is not
            // drawn: the game's programs put such a vertex at clip space's
            // origin, w 0 (MDL-SKIN-030), and the client shows nothing of
            // its faces, the stock `c_wings2`'s 102 of them
            // (`reading_wings_without_bones` in `client_render.rs`).)
            // (A skin none of whose bones is a node of the model, as
            // `c_wings2`'s right wing, has none of its faces drawn.)
            if matches!(&m.extra, MeshExtra::Skin(s) if s.bones.is_empty()) {
                indices.clear();
            }
            if let Some((_, data)) = &skin {
                let weighed = |i: u32| data[i as usize].weights.iter().any(|w| *w > 0.0);
                indices = indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .filter(|t| t.iter().all(|&i| weighed(i)))
                    .flatten()
                    .copied()
                    .collect();
            }
            let slot = gpu.pool.put(gpu, &vertices, &indices, skin.as_ref().map(|(_, d)| &d[..]));
            let skin = skin.map(|(s, data)| {
                let totals = data.iter().map(|v| v.weights.iter().filter(|w| **w > 0.0).sum());
                let (least, most, unweighed) =
                    totals.fold((f32::MAX, 0.0f32, false), |(l, m, u), t: f32| match t > 0.0 {
                        true => (l.min(t), m.max(t), u),
                        false => (l, m, true),
                    });
                GpuSkin {
                    bones: s.bones.clone(),
                    inverse_bind: inverse_binds(s, &rest, i),
                    vertices: slot.chunk.skin.clone().expect("a skinned chunk"),
                    weighed: if most > 0.0 { [least, most] } else { [1.0, 1.0] },
                    unweighed,
                }
            });
            let mesh = GpuMesh {
                node: i,
                vertices: slot.chunk.vertices.clone(),
                indices: slot.chunk.indices.clone(),
                index_count: slot.index_count,
                first_index: slot.first_index,
                base_vertex: slot.base_vertex as i32,
                vertex_count: slot.vertex_count,
                material: Material {
                    texture: m.textures[0].clone(),
                    maps: [m.textures[1].clone(), m.textures[2].clone(), m.textures[3].clone()],
                    mtr: m.material.clone(),
                    normal_mapped: m.renderhint.as_deref().is_some_and(|h| h != "none"),
                    diffuse: Vec3::from(m.diffuse),
                    ambient: Vec3::from(m.ambient),
                    emissive,
                    alpha: node.value("alpha").and_then(|v| v.first().copied()).unwrap_or(1.0),
                    transparency_hint: m.transparency_hint,
                    rotate_texture: m.rotate_texture,
                    uvs: m.uvs.first().is_some_and(|u| !u.is_empty()),
                },
                min,
                max,
                tangents_made: m.tangents.is_empty(),
                casts: m.shadow,
                skin,
                slot: Arc::new(slot),
            };
            if m.render { meshes.push(mesh) } else { shadow_only.push(mesh) }
        }
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        GpuModel { id, model, meshes, shadow_only, rest }
    }

    /// This model drawing only the first `count(mesh)` of each mesh's
    /// indices (at most all of them), from the same buffers: nothing is
    /// uploaded. (An area's grass far off: the first of its tufts.)
    pub fn first_indices(&self, mut count: impl FnMut(&GpuMesh) -> u32) -> GpuModel {
        let meshes = self
            .meshes
            .iter()
            .map(|m| GpuMesh {
                node: m.node,
                vertices: m.vertices.clone(),
                indices: m.indices.clone(),
                index_count: count(m).min(m.index_count),
                first_index: m.first_index,
                base_vertex: m.base_vertex,
                vertex_count: m.vertex_count,
                material: m.material.clone(),
                min: m.min,
                max: m.max,
                skin: m.skin.clone(),
                tangents_made: m.tangents_made,
                casts: m.casts,
                slot: m.slot.clone(),
            })
            .collect();
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // (Grass casts no shadow.)
        let (model, rest) = (self.model.clone(), self.rest.clone());
        GpuModel { id, model, meshes, shadow_only: Vec::new(), rest }
    }
}

/// Tangents for a mesh that has none (each vertex's, and the bitangent's
/// sign), from its positions and first texture coordinates, as the engine
/// makes them for a mesh whose material asks for normal maps (the wiki's
/// MTR, "Renderhint"): the stock normal-mapped shaders take the tangent as
/// it comes (`SetupTSB` normalizes it), and a zero one is a black mesh.
/// Each face's tangent and bitangent are added to its corners'; each
/// vertex's tangent is made at right angles to its normal. Where the
/// texture coordinates give none (no coordinates, a face without area in
/// them), one at right angles to the normal.
pub fn made_tangents(m: &Mesh) -> Vec<[f32; 4]> {
    let n = m.vertices.len();
    let (mut t, mut b) = (vec![Vec3::ZERO; n], vec![Vec3::ZERO; n]);
    let uv = |v: usize| glam::Vec2::from(m.uvs[0].get(v).copied().unwrap_or([0.0, 0.0]));
    for [i, j, k] in m.triangles() {
        let (i, j, k) = (i as usize, j as usize, k as usize);
        if i >= n || j >= n || k >= n {
            continue;
        }
        let p = |v: usize| Vec3::from(m.vertices[v]);
        let (e1, e2) = (p(j) - p(i), p(k) - p(i));
        let (d1, d2) = (uv(j) - uv(i), uv(k) - uv(i));
        let det = d1.x * d2.y - d2.x * d1.y;
        if det.abs() < 1e-12 {
            continue;
        }
        let r = 1.0 / det;
        let (ft, fb) = ((e1 * d2.y - e2 * d1.y) * r, (e2 * d1.x - e1 * d2.x) * r);
        for v in [i, j, k] {
            t[v] += ft;
            b[v] += fb;
        }
    }
    (0..n)
        .map(|v| {
            let normal = Vec3::from(m.normals.get(v).copied().unwrap_or([0.0, 0.0, 1.0]))
                .normalize_or(Vec3::Z);
            let along = t[v] - normal * normal.dot(t[v]);
            let tangent = if along.length_squared() > 1e-12 {
                along.normalize()
            } else {
                normal.any_orthonormal_vector()
            };
            let sign = if normal.cross(tangent).dot(b[v]) < 0.0 { -1.0 } else { 1.0 };
            tangent.extend(sign).to_array()
        })
        .collect()
}

/// A mesh's vertices as uploaded.
pub fn mesh_vertices(m: &Mesh) -> Vec<Vertex> {
    let made = if m.tangents.is_empty() { made_tangents(m) } else { Vec::new() };
    let tangents = if m.tangents.is_empty() { &made } else { &m.tangents };
    (0..m.vertices.len())
        .map(|v| Vertex {
            pos: m.vertices[v],
            normal: m.normals.get(v).copied().unwrap_or([0.0, 0.0, 1.0]),
            uv: m.uvs[0].get(v).copied().unwrap_or([0.0, 0.0]),
            tangent: tangents.get(v).copied().unwrap_or([0.0; 4]),
            color: m.colors.get(v).copied().unwrap_or([255; 4]),
            uv1: m.uvs[1].get(v).copied().unwrap_or([0.0, 0.0]),
            uv2: m.uvs[2].get(v).copied().unwrap_or([0.0, 0.0]),
            uv3: m.uvs[3].get(v).copied().unwrap_or([0.0, 0.0]),
        })
        .collect()
}

impl GpuModel {
    /// The model data of mesh `index` (of [`GpuModel::meshes`]).
    pub fn mesh_data(&self, index: usize) -> Option<&Mesh> {
        match &self.model.nodes.get(self.meshes.get(index)?.node)?.kind {
            NodeKind::Mesh(m) => Some(m),
            _ => None,
        }
    }
}

/// A node's transform relative to its parent.
pub fn local(position: [f32; 3], orientation: [f32; 4], scale: f32) -> Mat4 {
    crate::anim::Local::new(position, orientation, scale).matrix()
}

/// Each node's transform in model space, in the rest pose. (A node whose
/// parent comes after it, which no model of the game's has, stands as if
/// it had none.)
pub fn rest_pose(model: &Model) -> Vec<Mat4> {
    crate::anim::compose(model, &crate::anim::rest_locals(model))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tangents made for a mesh without: along +u, at right angles to the
    /// normal, the bitangent's sign by which way +v runs; one at right
    /// angles to the normal where the coordinates give none.
    #[test]
    fn tangents_are_made_from_the_texture_coordinates() {
        let quad = |uvs: Vec<[f32; 2]>| Mesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 4],
            uvs: [uvs, vec![], vec![], vec![]],
            faces: vec![
                mg_mdl::Face { vertices: [0, 1, 2], material: 0 },
                mg_mdl::Face { vertices: [0, 2, 3], material: 0 },
            ],
            ..Default::default()
        };
        let up = quad(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        for t in made_tangents(&up) {
            assert!((Vec3::from_slice(&t[..3]) - Vec3::X).length() < 1e-5 && t[3] == 1.0, "{t:?}");
        }
        // Mirrored in v: the bitangent the other way.
        let down = quad(vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);
        assert!(made_tangents(&down).iter().all(|t| t[0] > 0.99 && t[3] == -1.0));
        // No coordinates: at right angles to the normal, not zero.
        let none = quad(vec![]);
        for t in made_tangents(&none) {
            let t = Vec3::from_slice(&t[..3]);
            assert!((t.length() - 1.0).abs() < 1e-5 && t.z.abs() < 1e-5, "{t}");
        }
        // The model's own are kept.
        let mut own = quad(vec![[0.0, 0.0]; 4]);
        own.tangents = vec![[0.0, 1.0, 0.0, -1.0]; 4];
        assert!(mesh_vertices(&own).iter().all(|v| v.tangent == [0.0, 1.0, 0.0, -1.0]));
        assert!(mesh_vertices(&up).iter().all(|v| v.tangent[0] > 0.99));
    }

    /// Every texture coordinate set of a mesh is uploaded (a custom
    /// shader's `vTcIn1`…`vTcIn3`), and a set the mesh doesn't have is
    /// zero, as the client feeds it (`custom_shader_uvs_look` in
    /// `client_render.rs`): the third and fourth were zero for every
    /// mesh up to 2.5.3, which drew a builder's walls of `tverts2` in
    /// one flat color.
    #[test]
    fn every_texture_coordinate_set_is_uploaded() {
        let set = |k: f32| (0..3).map(|v| [k, v as f32]).collect::<Vec<_>>();
        let mesh = |uvs: [Vec<[f32; 2]>; 4]| Mesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs,
            faces: vec![mg_mdl::Face { vertices: [0, 1, 2], material: 0 }],
            ..Default::default()
        };
        let all = mesh_vertices(&mesh([set(0.0), set(1.0), set(2.0), set(3.0)]));
        for (v, vertex) in all.iter().enumerate() {
            let y = v as f32;
            assert_eq!(
                [vertex.uv, vertex.uv1, vertex.uv2, vertex.uv3],
                [[0.0, y], [1.0, y], [2.0, y], [3.0, y]]
            );
        }
        let two = mesh_vertices(&mesh([set(0.0), set(1.0), vec![], vec![]]));
        assert!(two.iter().all(|v| v.uv1[0] == 1.0 && v.uv2 == [0.0; 2] && v.uv3 == [0.0; 2]));
    }

    /// A node that names a parent after itself (no model of the game's
    /// does; a broken custom one might) stands as if it had none: the rest
    /// pose went past the end of what it had worked out.
    #[test]
    fn a_node_whose_parent_comes_later_has_a_rest_pose() {
        use mg_mdl::{Node, NodeKind};
        let mut model = Model::default();
        let mut child = Node::new("child", NodeKind::Dummy);
        child.parent = Some(1);
        child.position = [1.0, 0.0, 0.0];
        let mut parent = Node::new("parent", NodeKind::Dummy);
        parent.position = [0.0, 2.0, 0.0];
        model.nodes = vec![child, parent];
        let pose = rest_pose(&model);
        assert_eq!(pose.len(), 2);
        assert_eq!(pose[0].w_axis.truncate(), Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(pose[1].w_axis.truncate(), Vec3::new(0.0, 2.0, 0.0));
    }

    /// A skin whose bone map names a node the model doesn't have (custom
    /// content with a bone map left from another model: bone 36 of a model
    /// of 3 nodes crashed Moonglow 1.4.0 on loading the module) is given a
    /// bind pose that holds the mesh where it is.
    #[test]
    fn a_skin_s_bone_that_is_no_node_holds_the_mesh_still() {
        let rest = [
            Mat4::IDENTITY,
            Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            Mat4::from_translation(Vec3::new(2.0, 0.0, 1.0)),
        ];
        let skin = mg_mdl::Skin { bones: vec![1, 36], ..Default::default() };
        let binds = inverse_binds(&skin, &rest, 2);
        assert_eq!(binds.len(), 2);
        // A bone the model has: its rest pose undone, in the skin's space.
        assert_eq!(binds[0], rest[1].inverse() * rest[2]);
        // One it doesn't: posed as unmoved, the mesh stays at its rest.
        assert_eq!(rest[2].inverse() * Mat4::IDENTITY * binds[1], Mat4::IDENTITY);

        // The same with the bind poses the model stores (binary models).
        let stored = vec![([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0]); 3];
        let skin = mg_mdl::Skin { bones: vec![1, 36], inverse_bind: stored, ..Default::default() };
        let binds = inverse_binds(&skin, &rest, 2);
        assert_eq!((binds[0], binds[1]), (Mat4::IDENTITY, rest[2]));
        // And a skin node that is itself out of range doesn't panic either.
        assert_eq!(inverse_binds(&skin, &rest, 9).len(), 2);
    }
}
