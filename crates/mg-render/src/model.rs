//! A model on the GPU: a vertex and index buffer per drawn mesh, its
//! material, and the node it hangs from.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use mg_mdl::{Mesh, MeshExtra, Model, NodeKind};
use wgpu::util::DeviceExt;

use crate::Gpu;

/// A vertex as the shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// The tangent and the bitangent's sign, where the model has them
    /// (`tangents`, or compiled with a render hint); zero otherwise, and
    /// the shader takes the tangent frame from screen-space derivatives.
    pub tangent: [f32; 4],
    /// The model's vertex color (`colors`), white without: the game hands
    /// it to custom shaders alone (`vCustomColor`).
    pub color: [u8; 4],
    /// The second texture coordinates (`tverts1`), zero without.
    pub uv1: [f32; 2],
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
#[derive(Debug)]
pub struct GpuSkin {
    /// Bone nodes.
    pub bones: Vec<usize>,
    /// Per bone: from the skin node's space to the bone's, in the bind
    /// pose. Compiled models store it (and a few bind in a pose other than
    /// the rest pose); for ASCII models it comes from the rest pose.
    pub inverse_bind: Vec<Mat4>,
    pub vertices: wgpu::Buffer,
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
}

/// A drawn mesh.
#[derive(Debug)]
pub struct GpuMesh {
    /// The node it belongs to.
    pub node: usize,
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
    pub index_count: u32,
    pub material: Material,
    /// Local bounds.
    pub min: Vec3,
    pub max: Vec3,
    pub skin: Option<GpuSkin>,
}

/// A model's meshes on the GPU.
#[derive(Debug)]
pub struct GpuModel {
    /// A number of its own among the models uploaded: what the renderer
    /// keeps of a model, it keeps by this.
    pub id: u64,
    pub model: Arc<Model>,
    pub meshes: Vec<GpuMesh>,
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
    /// Uploads the meshes that render (walkmeshes and `render 0` shadow
    /// meshes are skipped).
    pub fn new(gpu: &Gpu, model: Arc<Model>) -> GpuModel {
        crate::guard::fail_if_asked(&model.name);
        let rest = rest_pose(&model);
        let mut meshes = Vec::new();
        for (i, node) in model.nodes.iter().enumerate() {
            let NodeKind::Mesh(m) = &node.kind else { continue };
            if !m.render || m.triangles().next().is_none() || matches!(m.extra, MeshExtra::Aabb(_))
            {
                continue;
            }
            let vertices = mesh_vertices(m);
            let indices: Vec<u32> = m.triangles().flatten().collect();
            let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for p in &m.vertices {
                min = min.min(Vec3::from(*p));
                max = max.max(Vec3::from(*p));
            }
            let label = format!("{}:{}", model.name, node.name);
            let vb = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&label),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ib = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&label),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
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
                    let inverse_bind = inverse_binds(s, &rest, i);
                    Some(GpuSkin {
                        bones: s.bones.clone(),
                        inverse_bind,
                        vertices: gpu.device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: Some(&label),
                                contents: bytemuck::cast_slice(&data),
                                usage: wgpu::BufferUsages::VERTEX,
                            },
                        ),
                    })
                }
                _ => None,
            };
            meshes.push(GpuMesh {
                node: i,
                vertices: vb,
                indices: ib,
                index_count: indices.len() as u32,
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
                },
                min,
                max,
                skin,
            });
        }
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        GpuModel { id, model, meshes, rest }
    }
}

/// A mesh's vertices as uploaded.
pub fn mesh_vertices(m: &Mesh) -> Vec<Vertex> {
    (0..m.vertices.len())
        .map(|v| Vertex {
            pos: m.vertices[v],
            normal: m.normals.get(v).copied().unwrap_or([0.0, 0.0, 1.0]),
            uv: m.uvs[0].get(v).copied().unwrap_or([0.0, 0.0]),
            tangent: m.tangents.get(v).copied().unwrap_or([0.0; 4]),
            color: m.colors.get(v).copied().unwrap_or([255; 4]),
            uv1: m.uvs[1].get(v).copied().unwrap_or([0.0, 0.0]),
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
    let q = Quat::from_array(orientation);
    let q = if q.length_squared() > 1e-12 { q.normalize() } else { Quat::IDENTITY };
    Mat4::from_scale_rotation_translation(Vec3::splat(scale), q, Vec3::from(position))
}

/// Each node's transform in model space, in the rest pose.
pub fn rest_pose(model: &Model) -> Vec<Mat4> {
    let mut out: Vec<Mat4> = Vec::with_capacity(model.nodes.len());
    for n in &model.nodes {
        let l = local(n.position, n.orientation, n.scale);
        let world = match n.parent {
            Some(p) => out[p] * l,
            None => l,
        };
        out.push(world);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
