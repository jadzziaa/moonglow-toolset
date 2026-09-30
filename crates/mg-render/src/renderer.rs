//! Drawing a scene: opaque meshes first, then blended ones back to front
//! (additive ones last), each with up to 32 lights chosen by priority and
//! distance, as the game trims its light list.

use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use mg_image::Rgba;
use mg_image::txi::Blending;

use crate::Gpu;
use crate::assets::Assets;
use crate::model::{SkinVertex, Vertex};
use crate::particles::{ParticleBlend, ParticleVertex};
use crate::scene::{Camera, Scene};
use crate::texture::GpuTexture;

/// Lights per draw, as the game's default `max-lights`.
pub const MAX_LIGHTS: usize = 32;
/// The game's `max-intensity` and `intensity-at-range` settings. It
/// linearises them and uploads 1 / max and a falloff factor that makes the
/// attenuation's denominator 1 / at-range where d = cutoff / 2 (both read
/// back from its uniforms).
const MAX_INTENSITY: f32 = 1.5;
const INTENSITY_AT_RANGE: f32 = 0.2;

/// The attenuation uniforms: 1 / max intensity and the falloff factor, as
/// the game computes them from its default settings.
pub fn attenuation_params() -> (f32, f32) {
    let max_inv = MAX_INTENSITY.powf(-2.2);
    let m2 = crate::scene::CUTOFF_RANGE_MULTIPLIER.powi(2);
    (max_inv, m2 * (INTENSITY_AT_RANGE.powf(-2.2) - max_inv))
}
/// The default alpha test.
const ALPHA_DISCARD: f32 = 0.2;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct FrameUniform {
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    area_ambient: [f32; 4],
    area_diffuse: [f32; 4],
    area_dir: [f32; 4],
    fog: [f32; 4],
    fog_color: [f32; 4],
    light_params: [f32; 4],
    scene_color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct LightUniform {
    pos: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct DrawUniform {
    model_view: [[f32; 4]; 4],
    normal_matrix: [[f32; 4]; 4],
    diffuse: [f32; 4],
    ambient: [f32; 4],
    emissive: [f32; 4],
    params: [f32; 4],
    material: [f32; 4],
    light_count: [u32; 4],
    light_index: [[u32; 4]; 8],
}

/// How a draw blends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Pass {
    Opaque,
    Blend,
    Additive,
}

/// A texture as the renderer uses it.
#[derive(Debug)]
struct TextureEntry {
    gpu: GpuTexture,
    blending: Blending,
    env: Option<String>,
    decal: bool,
    clamp: (bool, bool),
    spec: [f32; 3],
}

/// A material bind group's key: base texture, environment map, clamping.
type MaterialKey = (Option<String>, String, (bool, bool));

/// One draw, ready to sort.
struct Draw {
    pass: Pass,
    depth: f32,
    hint: u32,
    uniform: DrawUniform,
    material: MaterialKey,
    vertices: wgpu::Buffer,
    skin: Option<wgpu::Buffer>,
    indices: wgpu::Buffer,
    count: u32,
}

#[derive(Debug)]
pub struct Renderer {
    color_format: wgpu::TextureFormat,
    sample_count: u32,
    frame_layout: wgpu::BindGroupLayout,
    draw_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    pipelines: HashMap<(Pass, bool), wgpu::RenderPipeline>,
    particle_frame_layout: wgpu::BindGroupLayout,
    particle_pipelines: HashMap<ParticleBlend, wgpu::RenderPipeline>,
    particle_buffer: wgpu::Buffer,
    frame_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    bone_buffer: wgpu::Buffer,
    draw_buffer: wgpu::Buffer,
    draw_stride: u64,
    white: Arc<GpuTexture>,
    textures: HashMap<String, Option<Arc<TextureEntry>>>,
    materials: HashMap<MaterialKey, wgpu::BindGroup>,
    samplers: HashMap<(bool, bool), wgpu::Sampler>,
    env_sampler: wgpu::Sampler,
}

fn cols(m: Mat4) -> [[f32; 4]; 4] {
    m.to_cols_array_2d()
}

impl Renderer {
    /// A renderer drawing into `color_format` targets with `sample_count`
    /// samples (1 = no MSAA).
    pub fn new(gpu: &Gpu, color_format: wgpu::TextureFormat, sample_count: u32) -> Renderer {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lit"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let uniform = |binding, dynamic| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: None,
            },
            count: None,
        };
        let frame_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame"),
            entries: &[
                uniform(0, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let draw_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("draw"),
            entries: &[uniform(0, true)],
        });
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &[texture_entry(0), texture_entry(1), sampler_entry(2), sampler_entry(3)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lit"),
            bind_group_layouts: &[Some(&frame_layout), Some(&draw_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let mut pipelines = HashMap::new();
        for (pass, skinned) in [Pass::Opaque, Pass::Blend, Pass::Additive]
            .into_iter()
            .flat_map(|p| [(p, false), (p, true)])
        {
            let blend = match pass {
                Pass::Opaque => None,
                Pass::Blend => Some(wgpu::BlendState::ALPHA_BLENDING),
                Pass::Additive => Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                }),
            };
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("lit"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(if skinned { "vs_skinned" } else { "vs_main" }),
                    compilation_options: Default::default(),
                    buffers: if skinned {
                        &[Some(Vertex::LAYOUT), Some(SkinVertex::LAYOUT)]
                    } else {
                        &[Some(Vertex::LAYOUT)]
                    },
                },
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(pass != Pass::Additive),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: sample_count, ..Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            pipelines.insert((pass, skinned), pipeline);
        }
        // Particles.
        let particle_frame_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("particle frame"),
                entries: &[uniform(0, false)],
            });
        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particles"),
            source: wgpu::ShaderSource::Wgsl(include_str!("particle.wgsl").into()),
        });
        let particle_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particles"),
            bind_group_layouts: &[Some(&particle_frame_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let mut particle_pipelines = HashMap::new();
        for blend in [ParticleBlend::Normal, ParticleBlend::Lighten, ParticleBlend::PunchThrough] {
            let (entry, state, depth_write) = match blend {
                ParticleBlend::Normal => {
                    ("fs_blend", Some(wgpu::BlendState::ALPHA_BLENDING), false)
                }
                ParticleBlend::PunchThrough => ("fs_punch", None, true),
                ParticleBlend::Lighten => (
                    "fs_add",
                    Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    false,
                ),
            };
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("particles"),
                layout: Some(&particle_layout),
                vertex: wgpu::VertexState {
                    module: &particle_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(ParticleVertex::LAYOUT)],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(depth_write),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: sample_count, ..Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &particle_shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: state,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            particle_pipelines.insert(blend, pipeline);
        }
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let align = u64::from(device.limits().min_uniform_buffer_offset_alignment);
        let draw_stride = (std::mem::size_of::<DrawUniform>() as u64).div_ceil(align) * align;
        let env_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("env"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        Renderer {
            color_format,
            sample_count,
            frame_layout,
            draw_layout,
            material_layout,
            pipelines,
            particle_frame_layout,
            particle_pipelines,
            particle_buffer: buffer("particles", 1 << 16, wgpu::BufferUsages::VERTEX),
            frame_buffer: buffer(
                "frame",
                std::mem::size_of::<FrameUniform>() as u64,
                wgpu::BufferUsages::UNIFORM,
            ),
            light_buffer: buffer("lights", 32 * 256, wgpu::BufferUsages::STORAGE),
            bone_buffer: buffer("bones", 64 * 64, wgpu::BufferUsages::STORAGE),
            draw_buffer: buffer("draws", draw_stride * 64, wgpu::BufferUsages::UNIFORM),
            draw_stride,
            white: Arc::new(GpuTexture::solid(gpu, "white", [255; 4])),
            textures: HashMap::new(),
            materials: HashMap::new(),
            samplers: HashMap::new(),
            env_sampler,
        }
    }

    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.color_format
    }

    /// Forgets loaded textures (after the resources change).
    pub fn clear_textures(&mut self) {
        self.textures.clear();
        self.materials.clear();
    }

    fn texture(&mut self, gpu: &Gpu, assets: &dyn Assets, name: &str) -> Option<Arc<TextureEntry>> {
        if let Some(t) = self.textures.get(name) {
            return t.clone();
        }
        let entry = assets.texture(name).map(|t| {
            let spec = t.mtr.as_ref().map_or([0.0; 3], |m| {
                let f = |p: &str| m.float(p).and_then(|v| v.first().copied()).unwrap_or(0.0);
                [f("Specularity"), f("Roughness"), f("Metallicness")]
            });
            Arc::new(TextureEntry {
                gpu: GpuTexture::upload(gpu, name, &t.texture, t.txi.mipmap()),
                blending: t.txi.blending(),
                env: t.txi.envmap().map(str::to_ascii_lowercase),
                decal: t.txi.decal(),
                clamp: t.txi.clamp(),
                spec,
            })
        });
        self.textures.insert(name.to_string(), entry.clone());
        entry
    }

    fn sampler(&mut self, gpu: &Gpu, clamp: (bool, bool)) -> wgpu::Sampler {
        self.samplers
            .entry(clamp)
            .or_insert_with(|| {
                let mode =
                    |c| if c { wgpu::AddressMode::ClampToEdge } else { wgpu::AddressMode::Repeat };
                gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("texture"),
                    address_mode_u: mode(clamp.0),
                    address_mode_v: mode(clamp.1),
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::MipmapFilterMode::Linear,
                    anisotropy_clamp: 8,
                    ..Default::default()
                })
            })
            .clone()
    }

    /// Draws a scene into `target` (with a depth buffer of the same size
    /// and sample count).
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        gpu: &Gpu,
        assets: &dyn Assets,
        scene: &Scene,
        camera: &Camera,
        target: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        depth: &wgpu::TextureView,
        size: (u32, u32),
    ) {
        let view = camera.view();
        let proj = camera.projection(size.0 as f32 / size.1.max(1) as f32);
        let area_dir = view.transform_vector3(scene.area.direction).normalize_or_zero();
        let fog = scene
            .fog
            .map_or([0.0; 4], |f| [1.0, f.start, f.end, 1.0 / (f.end - f.start).max(1e-3)]);
        let frame = FrameUniform {
            view: cols(view),
            proj: cols(proj),
            area_ambient: scene.area.ambient.extend(1.0).to_array(),
            area_diffuse: scene.area.diffuse.extend(1.0).to_array(),
            area_dir: area_dir.extend(0.0).to_array(),
            fog,
            fog_color: scene.fog.map_or([0.0; 4], |f| f.color.extend(1.0).to_array()),
            light_params: {
                let (max_inv, falloff) = attenuation_params();
                [max_inv, falloff, scene.lights.len() as f32, 0.0]
            },
            scene_color: [0.0; 4],
        };
        gpu.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&frame));

        // Lights in view space.
        let lights: Vec<LightUniform> = scene
            .lights
            .iter()
            .map(|l| {
                let r2 = l.cutoff * l.cutoff;
                LightUniform {
                    pos: view.transform_point3(l.position).extend(1.0).to_array(),
                    color: l.color.extend(if l.ambient_only { -r2 } else { r2 }).to_array(),
                }
            })
            .collect();
        let light_bytes = std::mem::size_of::<LightUniform>() * lights.len().max(1);
        if (self.light_buffer.size() as usize) < light_bytes {
            self.light_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("lights"),
                size: light_bytes.next_power_of_two() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !lights.is_empty() {
            gpu.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&lights));
        }

        // Draws.
        let mut draws: Vec<Draw> = Vec::new();
        let mut bones: Vec<[[f32; 4]; 4]> = Vec::new();
        for inst in &scene.instances {
            let rest = &inst.model.rest;
            let pose: &Vec<Mat4> = inst.pose.as_deref().unwrap_or(rest);
            for mesh in &inst.model.meshes {
                // Bones: bind pose to current pose, in the skin node's space.
                let bone_base = bones.len() as u32;
                if let Some(skin) = &mesh.skin {
                    let skin_now = pose.get(mesh.node).copied().unwrap_or(Mat4::IDENTITY);
                    let to_skin = skin_now.inverse();
                    for (&b, inverse_bind) in skin.bones.iter().zip(&skin.inverse_bind) {
                        let now = pose.get(b).copied().unwrap_or(Mat4::IDENTITY);
                        bones.push(cols(to_skin * now * *inverse_bind));
                    }
                }
                let world = inst.transform * pose.get(mesh.node).copied().unwrap_or(Mat4::IDENTITY);
                let model_view = view * world;
                let centre = world.transform_point3((mesh.min + mesh.max) * 0.5);
                let radius = world.transform_vector3((mesh.max - mesh.min) * 0.5).length();
                let tex =
                    mesh.material.texture.as_deref().and_then(|t| self.texture(gpu, assets, t));
                let mat = &mesh.material;
                let (blending, env, decal, clamp, spec, has_alpha) = match &tex {
                    Some(t) => {
                        (t.blending, t.env.clone(), t.decal, t.clamp, t.spec, t.gpu.has_alpha)
                    }
                    None => (Blending::Default, None, false, (false, false), [0.0; 3], false),
                };
                let env_mapped = env.is_some();
                let pass = if blending == Blending::Additive {
                    Pass::Additive
                } else if mat.alpha < 1.0 || (has_alpha && !env_mapped) || mat.transparency_hint > 0
                {
                    Pass::Blend
                } else {
                    Pass::Opaque
                };
                let discard = if blending == Blending::Punchthrough { 0.5 } else { ALPHA_DISCARD };
                // The 32 most important lights that reach the mesh.
                let mut chosen: Vec<(u32, f32, u32)> = scene
                    .lights
                    .iter()
                    .enumerate()
                    .filter_map(|(i, l)| {
                        let d = l.position.distance(centre) - radius;
                        (d <= l.cutoff).then_some((l.priority, d, i as u32))
                    })
                    .collect();
                chosen.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
                chosen.truncate(MAX_LIGHTS);
                let mut light_index = [[0u32; 4]; 8];
                for (j, (_, _, i)) in chosen.iter().enumerate() {
                    light_index[j / 4][j % 4] = *i;
                }
                let normal = model_view.inverse().transpose();
                draws.push(Draw {
                    pass,
                    depth: -view.transform_point3(centre).z,
                    hint: mat.transparency_hint,
                    uniform: DrawUniform {
                        model_view: cols(model_view),
                        normal_matrix: cols(normal),
                        diffuse: mat.diffuse.extend(mat.alpha).to_array(),
                        ambient: mat.ambient.extend(1.0).to_array(),
                        emissive: mat.emissive.extend(1.0).to_array(),
                        params: [
                            discard,
                            if env_mapped { 1.0 } else { 0.0 },
                            if tex.is_some() { 1.0 } else { 0.0 },
                            if decal { 1.0 } else { 0.0 },
                        ],
                        material: [spec[0], spec[1], spec[2], if has_alpha { 1.0 } else { 0.0 }],
                        light_count: [
                            chosen.len() as u32,
                            u32::from(mesh.skin.is_some()),
                            bone_base,
                            0,
                        ],
                        light_index,
                    },
                    material: (
                        tex.as_ref().map(|_| mat.texture.clone().unwrap_or_default()),
                        env.unwrap_or_else(|| "chrome1".into()),
                        clamp,
                    ),
                    vertices: mesh.vertices.clone(),
                    skin: mesh.skin.as_ref().map(|s| s.vertices.clone()),
                    indices: mesh.indices.clone(),
                    count: mesh.index_count,
                });
            }
        }
        // Opaque (any order), then blended back to front by hint then depth,
        // then additive.
        draws.sort_by(|a, b| {
            let rank = |p: Pass| match p {
                Pass::Opaque => 0,
                Pass::Blend => 1,
                Pass::Additive => 2,
            };
            rank(a.pass).cmp(&rank(b.pass)).then_with(|| {
                if a.pass == Pass::Opaque {
                    std::cmp::Ordering::Equal
                } else {
                    a.hint.cmp(&b.hint).then(b.depth.total_cmp(&a.depth))
                }
            })
        });

        // Bones.
        let bone_bytes = 64 * bones.len().max(1);
        if (self.bone_buffer.size() as usize) < bone_bytes {
            self.bone_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bones"),
                size: bone_bytes.next_power_of_two() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bones.is_empty() {
            gpu.queue.write_buffer(&self.bone_buffer, 0, bytemuck::cast_slice(&bones));
        }

        // Upload per-draw data.
        let needed = self.draw_stride * draws.len().max(1) as u64;
        if self.draw_buffer.size() < needed {
            self.draw_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("draws"),
                size: needed.next_power_of_two(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        let mut bytes = vec![0u8; (self.draw_stride as usize) * draws.len()];
        for (i, d) in draws.iter().enumerate() {
            let at = i * self.draw_stride as usize;
            bytes[at..at + std::mem::size_of::<DrawUniform>()]
                .copy_from_slice(bytemuck::bytes_of(&d.uniform));
        }
        if !bytes.is_empty() {
            gpu.queue.write_buffer(&self.draw_buffer, 0, &bytes);
        }

        // Bind groups.
        let frame_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame"),
            layout: &self.frame_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.frame_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.light_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry { binding: 2, resource: self.bone_buffer.as_entire_binding() },
            ],
        });
        let draw_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("draw"),
            layout: &self.draw_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.draw_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(std::mem::size_of::<DrawUniform>() as u64),
                }),
            }],
        });
        for d in &draws {
            if self.materials.contains_key(&d.material) {
                continue;
            }
            let tex = d.material.0.as_deref().and_then(|t| self.texture(gpu, assets, t));
            let env = self.texture(gpu, assets, &d.material.1);
            let sampler = self.sampler(gpu, d.material.2);
            let white = self.white.clone();
            let tex_view = tex.as_ref().map_or(&white.view, |t| &t.gpu.view);
            let env_view = env.as_ref().map_or(&white.view, |t| &t.gpu.view);
            let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("material"),
                layout: &self.material_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(tex_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(env_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.env_sampler),
                    },
                ],
            });
            self.materials.insert(d.material.clone(), group);
        }

        // Particles: punch-through, then blended by render order, then
        // additive; their textures bind like meshes'.
        let mut batches: Vec<&crate::particles::ParticleBatch> =
            scene.particles.iter().filter(|b| !b.vertices.is_empty()).collect();
        batches.sort_by_key(|b| {
            let rank = match b.blend {
                ParticleBlend::PunchThrough => 0,
                ParticleBlend::Normal => 1,
                ParticleBlend::Lighten => 2,
            };
            (rank, b.render_order)
        });
        let mut particle_vertices: Vec<ParticleVertex> = Vec::new();
        let mut particle_draws = Vec::new();
        for b in &batches {
            let key: MaterialKey = (b.texture.clone(), "chrome1".into(), (false, false));
            if !self.materials.contains_key(&key) {
                let tex = b.texture.as_deref().and_then(|t| self.texture(gpu, assets, t));
                let sampler = self.sampler(gpu, (false, false));
                let white = self.white.clone();
                let view = tex.as_ref().map_or(&white.view, |t| &t.gpu.view);
                let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("particle material"),
                    layout: &self.material_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&white.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::Sampler(&self.env_sampler),
                        },
                    ],
                });
                self.materials.insert(key.clone(), group);
            }
            let start = particle_vertices.len() as u32;
            particle_vertices.extend_from_slice(&b.vertices);
            particle_draws.push((b.blend, key, start..particle_vertices.len() as u32));
        }
        let particle_bytes = std::mem::size_of_val(particle_vertices.as_slice());
        if (self.particle_buffer.size() as usize) < particle_bytes {
            self.particle_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("particles"),
                size: particle_bytes.next_power_of_two() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if particle_bytes > 0 {
            gpu.queue.write_buffer(
                &self.particle_buffer,
                0,
                bytemuck::cast_slice(&particle_vertices),
            );
        }
        let particle_frame = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle frame"),
            layout: &self.particle_frame_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.frame_buffer.as_entire_binding(),
            }],
        });

        let bg = scene.background;
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: resolve,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(bg[0]),
                            g: f64::from(bg[1]),
                            b: f64::from(bg[2]),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &frame_group, &[]);
            let mut current = None;
            for (i, d) in draws.iter().enumerate() {
                let key = (d.pass, d.skin.is_some());
                if current != Some(key) {
                    pass.set_pipeline(&self.pipelines[&key]);
                    current = Some(key);
                }
                if let Some(skin) = &d.skin {
                    pass.set_vertex_buffer(1, skin.slice(..));
                }
                pass.set_bind_group(1, &draw_group, &[(i as u64 * self.draw_stride) as u32]);
                pass.set_bind_group(2, &self.materials[&d.material], &[]);
                pass.set_vertex_buffer(0, d.vertices.slice(..));
                pass.set_index_buffer(d.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..d.count, 0, 0..1);
            }
            if !particle_draws.is_empty() {
                pass.set_bind_group(0, &particle_frame, &[]);
                pass.set_vertex_buffer(0, self.particle_buffer.slice(..));
                for (blend, key, range) in &particle_draws {
                    pass.set_pipeline(&self.particle_pipelines[blend]);
                    pass.set_bind_group(1, &self.materials[key], &[]);
                    pass.draw(range.clone(), 0..1);
                }
            }
        }
        gpu.queue.submit([encoder.finish()]);
    }

    /// Draws a scene offscreen and reads it back (rows top first).
    pub fn render_image(
        &mut self,
        gpu: &Gpu,
        assets: &dyn Assets,
        scene: &Scene,
        camera: &Camera,
        width: u32,
        height: u32,
    ) -> Rgba {
        let targets = Targets::new(gpu, self.color_format, self.sample_count, width, height);
        self.render(
            gpu,
            assets,
            scene,
            camera,
            targets.render_view(),
            targets.resolve_view(),
            &targets.depth,
            (width, height),
        );
        gpu.read_rgba(&targets.color)
    }
}

/// Offscreen colour (and, with MSAA, multisampled) and depth targets.
#[derive(Debug)]
pub struct Targets {
    pub color: wgpu::Texture,
    pub color_view: wgpu::TextureView,
    pub msaa_view: Option<wgpu::TextureView>,
    pub depth: wgpu::TextureView,
    pub size: (u32, u32),
}

impl Targets {
    pub fn new(
        gpu: &Gpu,
        format: wgpu::TextureFormat,
        samples: u32,
        width: u32,
        height: u32,
    ) -> Targets {
        let (width, height) = (width.max(1), height.max(1));
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let make = |label, format, samples, usage| {
            gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = make(
            "color",
            format,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let msaa_view = (samples > 1).then(|| {
            make("msaa", format, samples, wgpu::TextureUsages::RENDER_ATTACHMENT)
                .create_view(&Default::default())
        });
        let depth = make(
            "depth",
            wgpu::TextureFormat::Depth32Float,
            samples,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&Default::default());
        let color_view = color.create_view(&Default::default());
        Targets { color, color_view, msaa_view, depth, size: (width, height) }
    }

    /// Where to draw: the multisampled view, else the colour view.
    pub fn render_view(&self) -> &wgpu::TextureView {
        self.msaa_view.as_ref().unwrap_or(&self.color_view)
    }

    /// The resolve target with MSAA.
    pub fn resolve_view(&self) -> Option<&wgpu::TextureView> {
        self.msaa_view.as_ref().map(|_| &self.color_view)
    }
}

#[cfg(test)]
mod tests {
    /// The client's `lightMaxIntensityInv` and `lightFalloffFactor` with the
    /// default settings.
    #[test]
    fn attenuation_matches_the_game() {
        let (max_inv, falloff) = super::attenuation_params();
        assert!((max_inv - 0.4098).abs() < 1e-3, "{max_inv}");
        assert!((falloff - 136.31).abs() < 0.1, "{falloff}");
    }
}
