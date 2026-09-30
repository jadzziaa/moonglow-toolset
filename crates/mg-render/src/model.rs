//! A model on the GPU: a vertex and index buffer per drawn mesh, its
//! material, and the node it hangs from.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use mg_mdl::{MeshExtra, Model, NodeKind};
use wgpu::util::DeviceExt;

use crate::Gpu;

/// A vertex as the shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
    };
}

/// A mesh's material, from the model.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// The texture name (bitmap), lower case.
    pub texture: Option<String>,
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
}

/// A model's meshes on the GPU.
#[derive(Debug)]
pub struct GpuModel {
    pub model: Arc<Model>,
    pub meshes: Vec<GpuMesh>,
}

impl GpuModel {
    /// Uploads the meshes that render (walkmeshes and `render 0` shadow
    /// meshes are skipped).
    pub fn new(gpu: &Gpu, model: Arc<Model>) -> GpuModel {
        let mut meshes = Vec::new();
        for (i, node) in model.nodes.iter().enumerate() {
            let NodeKind::Mesh(m) = &node.kind else { continue };
            if !m.render || m.faces.is_empty() || matches!(m.extra, MeshExtra::Aabb(_)) {
                continue;
            }
            let vertices: Vec<Vertex> = (0..m.vertices.len())
                .map(|v| Vertex {
                    pos: m.vertices[v],
                    normal: m.normals.get(v).copied().unwrap_or([0.0, 0.0, 1.0]),
                    uv: m.uvs[0].get(v).copied().unwrap_or([0.0, 0.0]),
                })
                .collect();
            let indices: Vec<u32> = m.faces.iter().flat_map(|f| f.vertices).collect();
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
            meshes.push(GpuMesh {
                node: i,
                vertices: vb,
                indices: ib,
                index_count: indices.len() as u32,
                material: Material {
                    texture: m.textures[0].clone(),
                    diffuse: Vec3::from(m.diffuse),
                    ambient: Vec3::from(m.ambient),
                    emissive,
                    alpha: node.value("alpha").and_then(|v| v.first().copied()).unwrap_or(1.0),
                    transparency_hint: m.transparency_hint,
                },
                min,
                max,
            });
        }
        GpuModel { model, meshes }
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
