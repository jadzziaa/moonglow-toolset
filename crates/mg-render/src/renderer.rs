//! Drawing a scene: opaque meshes first, then blended ones back to front
//! (additive ones last), each with up to 32 lights chosen by priority and
//! distance, as the game trims its light list.

use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use mg_image::Rgba;
use mg_image::txi::Blending;

use crate::Gpu;
use crate::assets::Assets;
use crate::batch::{Batcher, Bounds};
use crate::model::{SkinVertex, Vertex};
use crate::particles::{ParticleBlend, ParticleVertex};
use crate::reach::{Frustum, LightGrid};
use crate::scene::{Camera, Instance, Scene};
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
    time: [f32; 4],
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
    world: [[f32; 4]; 4],
    normal_matrix: [[f32; 4]; 4],
    diffuse: [f32; 4],
    ambient: [f32; 4],
    emissive: [f32; 4],
    params: [f32; 4],
    material: [f32; 4],
    maps: [f32; 4],
    maps2: [f32; 4],
    spec_color: [f32; 4],
    extra: [f32; 4],
    water: [f32; 4],
    effect: [f32; 4],
    light_count: [u32; 4],
    light_index: [[u32; 4]; 8],
}

/// How a draw blends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Pass {
    /// The skybox and its horizon fade: first, both sides of each face, at
    /// the far plane.
    Sky,
    SkyFade,
    Opaque,
    /// A see-through mesh's solid parts (what is nearly opaque of it),
    /// which hide what is behind them.
    Blend,
    /// The rest of a see-through mesh, over everything solid, back to
    /// front, hiding nothing: drawn with it, an edge or a pane that came
    /// before what is behind it kept that from showing (a plant's soft
    /// edge before the ground: the water under the ground showed).
    Fringe,
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
    /// A cube map (six faces).
    cube: bool,
    /// It ripples (the game's water): how far, and how fast.
    ripple: Option<(f32, f32)>,
    /// It is water (`bumpmaptexture shinywater`): waves cross it.
    water: bool,
}

/// Texture slots: diffuse, normal, specular, roughness, height,
/// self-illumination; then the two a known custom shader reads
/// ([`Effect`]): an alpha mask or lightmap, and a flow map.
const SLOTS: usize = 8;
/// The slots the stock shaders read (the rest are a custom shader's).
const STOCK_SLOTS: usize = 6;

/// What a custom shader Moonglow knows does, drawn by Moonglow's own
/// shader (it runs none of the game's or a tileset's): the
/// `vertexalpha_vs`/`vertexalpha_fs` pair community tilesets ship. It
/// multiplies the model's vertex colors in (baked shading), takes the
/// alpha from the vertex color or from a mask on the second texture
/// coordinates (`texture6`: layers blended into each other), or multiplies
/// that texture in as a lightmap (`fLightmapMode`), and slides the texture
/// along a flow map (`texture7`, `fSlideSpeed`: lava, water).
///
/// And the `mzlm_vs`/`mzlm_fs` pair NWN Mapper's exports name: a baked
/// lightmap (`texture7`, on the second texture coordinates) multiplied in
/// where `LightmapON` is set. Its source is not at hand: this is what the
/// exporter's materials and its own description say of it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Effect {
    /// The vertex colors are baked shading (and the layer's alpha).
    colors: bool,
    slide_speed: f32,
    shadow_reduction: f32,
    shadow_brightening: f32,
    lightmap: bool,
    /// The MTR's `transparency`: drawn blended.
    blend: bool,
}

/// The custom fragment shaders drawn as [`Effect`]s, by the start of their
/// names.
const EFFECT_SHADERS: [&str; 2] = ["vertexalpha", "mzlm"];

/// A material bind group's key: the slots' textures, the environment map,
/// clamping.
type MaterialKey = ([Option<String>; SLOTS], String, (bool, bool));

/// What a mesh's textures and MTR resolve to.
#[derive(Debug, Default)]
struct Slots {
    /// Per slot, the texture name to load.
    names: [Option<String>; SLOTS],
    /// MTR parameters (0: none).
    specularity: f32,
    roughness: f32,
    metallicness: f32,
    displacement_offset: f32,
    displacement_multiplier: f32,
    /// MTR `CustomSpecularColor`, linear.
    specular_color: Option<glam::Vec3>,
    /// The game's normal-mapped shader variant (`_nm`): a render hint, or
    /// any map beyond the diffuse texture.
    normal_variant: bool,
    /// A known custom shader's doings.
    effect: Option<Effect>,
    /// The fragment shader the MTR names has the environment map switched
    /// off (`#define ENVIRONMENT_MAP 0`, as the game's `fslit_nm` has): the
    /// object's environment map is not reflected, and the texture's alpha
    /// is left to the MTR's `transparency`.
    no_env: bool,
    /// The MTR's `transparency`: the mesh is blended.
    transparency: bool,
}

/// Whether a fragment shader's source switches the environment map off.
fn env_switched_off(source: &str) -> bool {
    source.lines().any(|l| {
        let mut words = l.split_whitespace();
        words.next() == Some("#define")
            && words.next() == Some("ENVIRONMENT_MAP")
            && words.next() == Some("0")
    })
}

/// The key of a mesh's slots: MDL bitmap and texture1–3, MTR name, render
/// hint.
type SlotKey = (Option<String>, [Option<String>; 3], Option<String>, bool);

/// What a mesh is drawn with, for an instance of its model: its textures
/// as the instance's renames, colours and environment map leave them, and
/// what they say of how to draw it. Worked out when a model is first seen
/// so dressed and kept ([`Renderer::surfaces`]): looked up by name for
/// every mesh of every frame, it was a fifth of drawing a large area.
#[derive(Debug)]
struct Surface {
    slots: Arc<Slots>,
    /// The slots whose texture there is.
    bound: [bool; SLOTS],
    /// Whether there is a diffuse texture, and its own: how it blends,
    /// whether it is a decal, its ripples and whether it is water.
    textured: bool,
    blending: Blending,
    decal: bool,
    ripple: Option<(f32, f32)>,
    water: bool,
    /// The texture's alpha makes the mesh see-through (not when it is an
    /// environment map's reflectivity).
    has_alpha: bool,
    /// It reflects an environment map, and that is a cube map.
    reflects: bool,
    env_cube: bool,
    /// Its bind group, in [`Renderer::groups`].
    material: u32,
}

/// What a model's [`Surface`]s are kept by: the model, and of the
/// instance its texture renames, its environment map and its PLT
/// colours (the first two by number: [`Renderer::names`],
/// [`Renderer::renames`]), and the scene's environment map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SurfaceKey {
    model: u64,
    renames: u32,
    env: u32,
    scene_env: u32,
    plt: Option<[u8; 10]>,
}

/// How many models' surfaces are kept before they are worked out anew.
const SURFACES_KEPT: usize = 16_384;

/// One draw, ready to sort.
#[derive(Clone, Copy)]
struct Draw<'a> {
    pass: Pass,
    depth: f32,
    hint: u32,
    /// Its uniforms' place among the frame's.
    uniform: u32,
    /// Its material's bind group, in [`Renderer::groups`].
    material: u32,
    mesh: &'a crate::model::GpuMesh,
    /// Replaced vertices: their bytes in the frame's dynamic buffer, from
    /// and to.
    dynamic: Option<(u64, u64)>,
    /// Its box in the scene (`None` for a mesh that leaves its box: a
    /// skinned or animated one, and the sky), and which of the frame's
    /// instances it is of: for putting draws of one mesh together
    /// ([`crate::batch`]).
    bounds: Option<Bounds>,
    instance: u32,
}

/// What the renderer draws: the lit scene, or a value for comparing with
/// the game's (through a debug shader in the game).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugView {
    #[default]
    Lit,
    /// Each lit mesh's material diffuse colour, linear, as uploaded.
    MaterialDiffuse,
}

#[derive(Debug)]
pub struct Renderer {
    pub debug: DebugView,
    /// Draws of one mesh are drawn as instances of one draw (on; off,
    /// each is drawn by itself, in the order they came in: to compare).
    pub instancing: bool,
    color_format: wgpu::TextureFormat,
    sample_count: u32,
    frame_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    pipelines: HashMap<(Pass, bool), wgpu::RenderPipeline>,
    /// The frame's uniforms as the particles' and the lines' pipelines bind
    /// them (the one buffer for good: made once).
    particle_frame: wgpu::BindGroup,
    particle_pipelines: HashMap<ParticleBlend, wgpu::RenderPipeline>,
    particle_buffer: wgpu::Buffer,
    line_pipeline: wgpu::RenderPipeline,
    line_buffer: wgpu::Buffer,
    /// Animated and dangly meshes' vertices, rewritten every frame.
    dynamic_buffer: wgpu::Buffer,
    frame_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    bone_buffer: wgpu::Buffer,
    /// Every draw's uniforms, one after another, and the draws in the
    /// order they are drawn in: an instance's uniforms are those its
    /// number names there.
    draw_buffer: wgpu::Buffer,
    order_buffer: wgpu::Buffer,
    batcher: Batcher,
    white: Arc<GpuTexture>,
    white_cube: Arc<GpuTexture>,
    textures: HashMap<String, Option<Arc<TextureEntry>>>,
    slots: HashMap<SlotKey, Arc<Slots>>,
    /// The materials' bind groups, and each material's place among them.
    materials: HashMap<MaterialKey, u32>,
    groups: Vec<wgpu::BindGroup>,
    /// Each model's meshes' surfaces, as its instances dress it.
    surfaces: HashMap<SurfaceKey, Arc<[Surface]>>,
    /// Environment maps' names, numbered from 1 (0: none).
    names: HashMap<String, u32>,
    /// Instances' texture renames, numbered from 1 (0: none), by a sum of
    /// their pairs' hashes.
    #[allow(clippy::type_complexity)]
    renames: HashMap<u64, Vec<(Vec<(String, String)>, u32)>>,
    renames_made: u32,
    samplers: HashMap<(bool, bool), wgpu::Sampler>,
    env_sampler: wgpu::Sampler,
    /// A frame's uniforms, bones and replaced vertices: kept from frame
    /// to frame, for their room.
    uniforms: Vec<DrawUniform>,
    bones: Vec<[[f32; 4]; 4]>,
    dynamic: Vec<Vertex>,
    /// What the last frame drew.
    pub drawn: Drawn,
}

/// What a frame drew ([`Renderer::drawn`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Drawn {
    /// The meshes drawn: one each, two for a see-through one.
    pub meshes: usize,
    /// The draws they were drawn with: meshes of one model in several
    /// places are instances of one draw.
    pub batches: usize,
    /// The meshes left out as out of the camera's sight.
    pub hidden: usize,
    /// How many times a draw's material was another than the draw's
    /// before it.
    pub materials: usize,
}

/// A line's end ([`crate::Line`]), in world space.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct LineVertex {
    pos: [f32; 3],
    /// Gamma-space colour and alpha.
    color: [f32; 4],
}

impl LineVertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<LineVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
    };
}

/// A gamma-space colour in linear space.
fn lin(c: glam::Vec3) -> glam::Vec3 {
    c.max(glam::Vec3::ZERO).powf(2.2)
}

/// A material's `CustomSpecularColor` in linear space, where it has one
/// that is not black.
fn custom_specular(mtr: &mg_image::mtr::Mtr) -> Option<glam::Vec3> {
    mtr.float("CustomSpecularColor")
        .filter(|v| v.len() >= 3 && v[..3].iter().any(|&c| c > 0.0))
        .map(|v| lin(glam::Vec3::new(v[0], v[1], v[2])))
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
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
            entries: &[
                texture_entry(0),
                texture_entry(1),
                sampler_entry(2),
                sampler_entry(3),
                texture_entry(4),
                texture_entry(5),
                texture_entry(6),
                texture_entry(7),
                texture_entry(8),
                texture_entry(10),
                texture_entry(11),
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lit"),
            bind_group_layouts: &[Some(&frame_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let mut pipelines = HashMap::new();
        let passes =
            [Pass::Sky, Pass::SkyFade, Pass::Opaque, Pass::Blend, Pass::Fringe, Pass::Additive];
        for (pass, skinned) in passes.into_iter().flat_map(|p| [(p, false), (p, true)]) {
            let sky = matches!(pass, Pass::Sky | Pass::SkyFade);
            let blend = match pass {
                Pass::Sky | Pass::Opaque => None,
                Pass::SkyFade | Pass::Blend | Pass::Fringe => {
                    Some(wgpu::BlendState::ALPHA_BLENDING)
                }
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
                    cull_mode: if sky { None } else { Some(wgpu::Face::Back) },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(
                        !sky && !matches!(pass, Pass::Additive | Pass::Fringe),
                    ),
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
                    format: DEPTH_FORMAT,
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
        // Lines.
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lines"),
            source: wgpu::ShaderSource::Wgsl(include_str!("line.wgsl").into()),
        });
        let line_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lines"),
            bind_group_layouts: &[Some(&particle_frame_layout)],
            immediate_size: 0,
        });
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lines"),
            layout: Some(&line_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(LineVertex::LAYOUT)],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: sample_count, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let draw_size = std::mem::size_of::<DrawUniform>() as u64;
        let frame_buffer = buffer(
            "frame",
            std::mem::size_of::<FrameUniform>() as u64,
            wgpu::BufferUsages::UNIFORM,
        );
        let particle_frame = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle frame"),
            layout: &particle_frame_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_buffer.as_entire_binding(),
            }],
        });
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
            debug: DebugView::Lit,
            instancing: true,
            color_format,
            sample_count,
            frame_layout,
            material_layout,
            pipelines,
            particle_frame,
            particle_pipelines,
            particle_buffer: buffer("particles", 1 << 16, wgpu::BufferUsages::VERTEX),
            line_pipeline,
            line_buffer: buffer("lines", 1 << 16, wgpu::BufferUsages::VERTEX),
            dynamic_buffer: buffer("dynamic vertices", 1 << 16, wgpu::BufferUsages::VERTEX),
            frame_buffer,
            light_buffer: buffer("lights", 32 * 256, wgpu::BufferUsages::STORAGE),
            bone_buffer: buffer("bones", 64 * 64, wgpu::BufferUsages::STORAGE),
            draw_buffer: buffer("draws", draw_size * 64, wgpu::BufferUsages::STORAGE),
            order_buffer: buffer("draw order", 4 * 64, wgpu::BufferUsages::STORAGE),
            batcher: Batcher::default(),
            white: Arc::new(GpuTexture::solid(gpu, "white", [255; 4])),
            white_cube: Arc::new(GpuTexture::upload_cube(gpu, "white cube", &{
                let face =
                    mg_image::Rgba { width: 1, height: 1, data: vec![255; 4] }.into_texture(false);
                std::array::from_fn(|_| face.clone())
            })),
            textures: HashMap::new(),
            slots: HashMap::new(),
            materials: HashMap::new(),
            groups: Vec::new(),
            surfaces: HashMap::new(),
            names: HashMap::new(),
            renames: HashMap::new(),
            renames_made: 0,
            samplers: HashMap::new(),
            env_sampler,
            uniforms: Vec::new(),
            bones: Vec::new(),
            dynamic: Vec::new(),
            drawn: Drawn::default(),
        }
    }

    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.color_format
    }

    /// Forgets loaded textures (after the resources change).
    pub fn clear_textures(&mut self) {
        self.textures.clear();
        self.slots.clear();
        self.materials.clear();
        self.groups.clear();
        self.surfaces.clear();
    }

    /// A mesh's texture slots and MTR parameters: an MTR named by the model,
    /// else one named like its bitmap, overrides the model's textures slot
    /// by slot.
    fn slots(&mut self, assets: &dyn Assets, mat: &crate::model::Material) -> Arc<Slots> {
        let key: SlotKey =
            (mat.texture.clone(), mat.maps.clone(), mat.mtr.clone(), mat.normal_mapped);
        if let Some(s) = self.slots.get(&key) {
            return s.clone();
        }
        let mtr = mat
            .mtr
            .as_deref()
            .and_then(|n| assets.material(n))
            .or_else(|| mat.texture.as_deref().and_then(|n| assets.material(n)));
        let mut out = Slots::default();
        out.names[0] = mat.texture.clone();
        for (i, m) in mat.maps.iter().enumerate() {
            out.names[i + 1] = m.clone();
        }
        if let Some(mtr) = &mtr {
            // Slot 0 keeps the bitmap: its texture lookup follows an MTR of
            // the same name itself (and a PLT's MTR names no texture0).
            if mat.mtr.is_some() && mtr.textures[0].is_some() {
                out.names[0] = mtr.textures[0].clone();
            }
            for i in 1..SLOTS {
                if mtr.textures[i].is_some() {
                    out.names[i] = mtr.textures[i].clone();
                }
            }
            let f = |p: &str| mtr.float(p).and_then(|v| v.first().copied()).unwrap_or(0.0);
            out.specularity = f("Specularity");
            out.roughness = f("Roughness");
            out.metallicness = f("Metallicness");
            out.displacement_offset = f("DisplacementOffset");
            out.displacement_multiplier = f("DisplacementMultiplier");
            out.specular_color = custom_specular(mtr);
            let fs = mtr.shader_fs.as_deref().unwrap_or_default().to_ascii_lowercase();
            let known = EFFECT_SHADERS.iter().any(|known| fs.starts_with(known));
            out.transparency = mtr.transparency;
            out.no_env = !fs.is_empty() && assets.shader(&fs).is_some_and(|s| env_switched_off(&s));
            let mapper = fs.starts_with("mzlm");
            out.effect = known.then(|| Effect {
                colors: !mapper,
                slide_speed: if mapper { 0.0 } else { f("fSlideSpeed") },
                shadow_reduction: f("fShadowReduction"),
                shadow_brightening: f("fShadowBrightening"),
                lightmap: if mapper { f("LightmapON") > 0.5 } else { f("fLightmapMode") > 0.5 },
                blend: mtr.transparency,
            });
            if mapper {
                // Its lightmap is where the other pair has its flow map.
                out.names[6] = out.names[7].take();
            }
        }
        if out.effect.is_none() {
            // (A custom shader's own slots mean nothing to the stock ones.)
            for name in &mut out.names[STOCK_SLOTS..] {
                *name = None;
            }
        }
        out.normal_variant = mat.normal_mapped
            || mtr.as_ref().is_some_and(|m| m.renderhint != mg_image::mtr::RenderHint::None)
            || out.names[1..STOCK_SLOTS].iter().any(Option::is_some);
        let out = Arc::new(out);
        self.slots.insert(key, out.clone());
        out
    }

    /// The bind group for a material key (its place in
    /// [`Renderer::groups`]), made on first use.
    fn material_group(&mut self, gpu: &Gpu, assets: &dyn Assets, key: &MaterialKey) -> u32 {
        if let Some(group) = self.materials.get(key) {
            return *group;
        }
        let textures: Vec<Option<Arc<TextureEntry>>> =
            key.0.iter().map(|n| n.as_deref().and_then(|t| self.texture(gpu, assets, t))).collect();
        let env = self.texture(gpu, assets, &key.1);
        let sampler = self.sampler(gpu, key.2);
        let (white, white_cube) = (self.white.clone(), self.white_cube.clone());
        let view = |i: usize| textures[i].as_ref().map_or(&white.view, |t| &t.gpu.view);
        let (env_view, cube_view) = match &env {
            Some(t) if t.cube => (&white.view, &t.gpu.view),
            Some(t) => (&t.gpu.view, &white_cube.view),
            None => (&white.view, &white_cube.view),
        };
        let tex = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material"),
            layout: &self.material_layout,
            entries: &[
                tex(0, view(0)),
                tex(1, env_view),
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.env_sampler),
                },
                tex(4, view(1)),
                tex(5, view(2)),
                tex(6, view(3)),
                tex(7, view(4)),
                tex(8, view(5)),
                tex(9, cube_view),
                tex(10, view(6)),
                tex(11, view(7)),
            ],
        });
        self.groups.push(group);
        let at = self.groups.len() as u32 - 1;
        self.materials.insert(key.clone(), at);
        at
    }

    /// An environment map's number (0: none).
    fn name_id(&mut self, name: Option<&str>) -> u32 {
        let Some(name) = name else { return 0 };
        if let Some(id) = self.names.get(name) {
            return *id;
        }
        let id = self.names.len() as u32 + 1;
        self.names.insert(name.to_string(), id);
        id
    }

    /// An instance's texture renames' number (0: none): the same for the
    /// same renames, however they are held.
    fn renames_id(&mut self, renames: &HashMap<String, String>) -> u32 {
        use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
        if renames.is_empty() {
            return 0;
        }
        // (A sum: whatever order the map gives its pairs in.)
        let hasher = BuildHasherDefault::<DefaultHasher>::default();
        let sum = renames.iter().fold(0u64, |sum, pair| sum.wrapping_add(hasher.hash_one(pair)));
        let known = self.renames.entry(sum).or_default();
        let same = |pairs: &[(String, String)]| {
            pairs.len() == renames.len() && pairs.iter().all(|(k, v)| renames.get(k) == Some(v))
        };
        if let Some((_, id)) = known.iter().find(|(pairs, _)| same(pairs)) {
            return *id;
        }
        self.renames_made += 1;
        let pairs = renames.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        known.push((pairs, self.renames_made));
        self.renames_made
    }

    /// The surfaces of the meshes of `inst`'s model, as the instance
    /// dresses it (in a scene whose environment map is `scene_env`,
    /// numbered `scene_env_id`).
    fn surfaces(
        &mut self,
        gpu: &Gpu,
        assets: &dyn Assets,
        inst: &Instance,
        scene_env: Option<&str>,
        scene_env_id: u32,
    ) -> Arc<[Surface]> {
        let key = SurfaceKey {
            model: inst.model.id,
            renames: inst.textures.as_deref().map_or(0, |t| self.renames_id(t)),
            env: self.name_id(inst.env_map.as_deref()),
            scene_env: scene_env_id,
            plt: inst.plt_colors,
        };
        if let Some(surfaces) = self.surfaces.get(&key) {
            return surfaces.clone();
        }
        let surfaces: Arc<[Surface]> = inst
            .model
            .meshes
            .iter()
            .map(|mesh| self.surface(gpu, assets, &mesh.material, inst, scene_env))
            .collect();
        if self.surfaces.len() >= SURFACES_KEPT {
            self.surfaces.clear();
        }
        self.surfaces.insert(key, surfaces.clone());
        surfaces
    }

    /// A mesh's surface, for an instance of its model.
    fn surface(
        &mut self,
        gpu: &Gpu,
        assets: &dyn Assets,
        mat: &crate::model::Material,
        inst: &Instance,
        scene_env: Option<&str>,
    ) -> Surface {
        let slots = self.slots(assets, mat);
        // The instance's texture renames, and its PLT colours on the
        // diffuse texture.
        // (A mesh that names none takes the instance's texture
        // for those, under the empty name: a body part's own.)
        let unnamed = || inst.textures.as_ref().and_then(|t| t.get("")).cloned();
        let names: [Option<String>; SLOTS] = std::array::from_fn(|i| {
            let named = slots.names[i].clone().or_else(|| unnamed().filter(|_| i == 0));
            named.as_ref().map(|n| {
                let renamed = inst.textures.as_ref().and_then(|t| t.get(n)).cloned();
                let n = renamed.unwrap_or_else(|| n.clone());
                match (i, inst.plt_colors) {
                    (0, Some(c)) => crate::assets::colored_name(&n, c),
                    _ => n,
                }
            })
        });
        let tex = names[0].as_deref().and_then(|t| self.texture(gpu, assets, t));
        let bound: [bool; SLOTS] = std::array::from_fn(|i| {
            names[i].as_deref().is_some_and(|t| self.texture(gpu, assets, t).is_some())
        });
        let (blending, env, decal, clamp, has_alpha) = match &tex {
            Some(t) => (t.blending, t.env.clone(), t.decal, t.clamp, t.gpu.has_alpha),
            None => (Blending::Default, None, false, (false, false), false),
        };
        // The environment map: the texture's (TXI), `default` or
        // none there meaning the object's, then the area's.
        let fallback = || inst.env_map.clone().or_else(|| scene_env.map(str::to_string));
        let env = match env {
            Some(e) if e.eq_ignore_ascii_case("default") => {
                Some(fallback().unwrap_or_else(|| "chrome1".into()))
            }
            Some(e) => Some(e),
            None => inst.env_map.clone(),
        };
        let env_mapped = env.is_some();
        // A shader with the environment map switched off reflects
        // nothing; the engine still takes the texture's alpha for
        // the map's, so without the MTR's `transparency` the mesh
        // is drawn solid, alpha and all (seen in the client:
        // `creatures_look` in `client_render.rs`).
        let see_through = mat.transparency_hint > 0 || slots.transparency;
        let (reflects, has_alpha) = (
            env_mapped && !slots.no_env,
            has_alpha && !(env_mapped && slots.no_env && !see_through),
        );
        let env_cube = env
            .as_deref()
            .and_then(|e| self.texture(gpu, assets, &e.to_ascii_lowercase()))
            .is_some_and(|t| t.cube);
        let key: MaterialKey = (
            std::array::from_fn(|i| if bound[i] { names[i].clone() } else { None }),
            env.map_or_else(|| "chrome1".into(), |e| e.to_ascii_lowercase()),
            clamp,
        );
        Surface {
            material: self.material_group(gpu, assets, &key),
            slots,
            bound,
            textured: tex.is_some(),
            blending,
            decal,
            ripple: tex.as_ref().and_then(|t| t.ripple),
            water: tex.as_ref().is_some_and(|t| t.water),
            has_alpha,
            reflects,
            env_cube,
        }
    }

    fn texture(&mut self, gpu: &Gpu, assets: &dyn Assets, name: &str) -> Option<Arc<TextureEntry>> {
        if let Some(t) = self.textures.get(name) {
            return t.clone();
        }
        // A cube map: a TXI with `cube 1` and six faces `name0`…`name5`.
        if let Some(txi) = assets.txi(name).filter(|t| t.cube()) {
            let faces: Option<Vec<mg_image::Texture>> =
                (0..6).map(|i| assets.texture(&format!("{name}{i}")).map(|t| t.texture)).collect();
            if let Some(faces) = faces.and_then(|f| <[mg_image::Texture; 6]>::try_from(f).ok()) {
                let entry = Some(Arc::new(TextureEntry {
                    gpu: GpuTexture::upload_cube(gpu, name, &faces),
                    blending: txi.blending(),
                    env: None,
                    decal: false,
                    clamp: (true, true),
                    cube: true,
                    ripple: None,
                    water: false,
                }));
                self.textures.insert(name.to_string(), entry.clone());
                return entry;
            }
        }
        let entry = assets.texture(name).map(|t| {
            Arc::new(TextureEntry {
                cube: false,
                gpu: GpuTexture::upload(gpu, name, &t.texture, t.txi.mipmap()),
                blending: t.txi.blending(),
                env: t.txi.envmap().map(str::to_ascii_lowercase),
                decal: t.txi.decal(),
                clamp: t.txi.clamp(),
                ripple: t.txi.ripple(),
                water: t.txi.water(),
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
    /// and sample count, [`DEPTH_FORMAT`]). The depth buffer keeps the
    /// scene's depths (as `camera`'s view and projection put them), so a
    /// caller can draw over the scene tested against it (overlays: a
    /// walkmesh, a selection) in a pass that loads both.
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
                [max_inv, falloff, scene.lights.len() as f32, size.1 as f32]
            },
            scene_color: [
                0.0,
                0.0,
                0.0,
                if self.debug == DebugView::MaterialDiffuse { 1.0 } else { 0.0 },
            ],
            time: [scene.time, 0.0, 0.0, 0.0],
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
        // The sky first, around the camera.
        // Around the camera, at ground level (the game's horizon fade then
        // covers what the camera sees of the horizon).
        let around = |s: &crate::Instance| crate::Instance {
            transform: Mat4::from_translation(camera.eye.with_z(0.0)) * s.transform,
            ..s.clone()
        };
        let sky = scene.sky.as_ref().map(around);
        let fade = scene.sky_fade.as_ref().map(|(s, c)| (around(s), *c));
        let mut draws: Vec<Draw<'_>> = Vec::new();
        let mut uniforms = std::mem::take(&mut self.uniforms);
        let mut bones = std::mem::take(&mut self.bones);
        let mut dynamic = std::mem::take(&mut self.dynamic);
        uniforms.clear();
        bones.clear();
        dynamic.clear();
        let frustum = Frustum::new(proj * view);
        let mut reach = LightGrid::new(&scene.lights);
        let mut chosen = Vec::new();
        let scene_env = scene.env_map.as_deref();
        let scene_env_id = self.name_id(scene_env);
        let mut hidden = 0;
        // (instance, sky, sky fade colour)
        let all = sky
            .iter()
            .map(|s| (s, true, None))
            .chain(fade.iter().map(|(s, c)| (s, true, Some(*c))))
            .chain(scene.instances.iter().map(|i| (i, false, None)));
        for (instance, (inst, is_sky, fade_color)) in all.enumerate() {
            let rest = &inst.model.rest;
            let pose: &Vec<Mat4> = inst.pose.as_deref().unwrap_or(rest);
            let surfaces = self.surfaces(gpu, assets, inst, scene_env, scene_env_id);
            for (j, (mesh, surface)) in inst.model.meshes.iter().zip(surfaces.iter()).enumerate() {
                let replaced = inst.state.as_ref().and_then(|s| s.meshes.get(j));
                let alpha =
                    replaced.and_then(|r| r.alpha).unwrap_or(mesh.material.alpha) * inst.opacity;
                // (Wholly see-through: nothing to draw.)
                if alpha <= 0.0 {
                    continue;
                }
                let world = inst.transform * pose.get(mesh.node).copied().unwrap_or(Mat4::IDENTITY);
                let half = (mesh.max - mesh.min) * 0.5;
                let centre = world.transform_point3((mesh.min + mesh.max) * 0.5);
                // Out of the camera's sight: not drawn. (Of the meshes
                // that stay in their box: not one a skin moves with its
                // bones or whose vertices an animation replaces.)
                let moved = replaced.and_then(|r| r.vertices.as_ref());
                let mut bounds = None;
                if !is_sky && mesh.skin.is_none() && moved.is_none() {
                    // (The box as it lies in the scene, turned and scaled,
                    // and a ball around it.)
                    let turned = glam::Mat3::from_mat4(world);
                    let around = turned.x_axis.abs() * half.x
                        + turned.y_axis.abs() * half.y
                        + turned.z_axis.abs() * half.z;
                    if frustum.hides(centre, around.length()) {
                        hidden += 1;
                        continue;
                    }
                    bounds = Some(Bounds { min: centre - around, max: centre + around });
                }
                let emissive = replaced.and_then(|r| r.selfillum).unwrap_or(mesh.material.emissive);
                let dynamic_range = moved.map(|v| {
                    let size = std::mem::size_of::<Vertex>() as u64;
                    let start = dynamic.len() as u64 * size;
                    dynamic.extend_from_slice(v);
                    (start, dynamic.len() as u64 * size)
                });
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
                let model_view = view * world;
                let radius = world.transform_vector3(half).length();
                let mat = &mesh.material;
                let slots = &surface.slots;
                let bound = surface.bound;
                let flag = |b: bool| if b { 1.0 } else { 0.0 };
                let pass = if fade_color.is_some() {
                    Pass::SkyFade
                } else if is_sky {
                    Pass::Sky
                } else if surface.blending == Blending::Additive {
                    Pass::Additive
                } else if alpha < 1.0
                    || (surface.has_alpha && !surface.reflects)
                    || mat.transparency_hint > 0
                    || slots.effect.is_some_and(|e| e.blend)
                {
                    Pass::Blend
                } else {
                    Pass::Opaque
                };
                let discard =
                    if surface.blending == Blending::Punchthrough { 0.5 } else { ALPHA_DISCARD };
                // The 32 most important lights that reach the mesh.
                reach.reaching(centre, radius, &mut chosen);
                let mut light_index = [[0u32; 4]; 8];
                for (j, (_, _, i)) in chosen.iter().enumerate() {
                    light_index[j / 4][j % 4] = *i;
                }
                let normal = model_view.inverse().transpose();
                let uniform = DrawUniform {
                    world: cols(world),
                    normal_matrix: cols(normal),
                    // MDL colours are gamma space; the game linearises
                    // them (read back from its uniforms).
                    diffuse: match fade_color {
                        // The fade: its colour, gamma space.
                        Some(c) => c.extend(1.0).to_array(),
                        None => lin(mat.diffuse).extend(alpha).to_array(),
                    },
                    ambient: lin(mat.ambient).extend(1.0).to_array(),
                    emissive: lin(emissive).extend(1.0).to_array(),
                    params: [
                        discard,
                        flag(surface.reflects),
                        flag(surface.textured),
                        // Unlit: 2 a marker (in its material's
                        // colour), 1 a decal and the sky.
                        if inst.unlit {
                            2.0
                        } else if surface.decal || is_sky {
                            1.0
                        } else {
                            0.0
                        },
                    ],
                    material: [
                        slots.specularity,
                        slots.roughness,
                        slots.metallicness,
                        flag(surface.has_alpha),
                    ],
                    maps: [flag(bound[1]), flag(bound[2]), flag(bound[3]), flag(bound[4])],
                    maps2: [
                        flag(bound[5]),
                        slots.displacement_offset,
                        slots.displacement_multiplier,
                        flag(slots.normal_variant),
                    ],
                    spec_color: slots.specular_color.map_or([0.0; 4], |c| c.extend(1.0).to_array()),
                    // (w: which part of a see-through mesh, below.)
                    extra: [flag(surface.env_cube), flag(is_sky), flag(fade_color.is_some()), 0.0],
                    water: {
                        let (far, fast) = surface.ripple.unwrap_or((0.0, 0.0));
                        [far, fast, flag(surface.water), 0.0]
                    },
                    effect: match slots.effect {
                        Some(e) => {
                            // 1 vertex colors, 2 alpha mask, 4 lightmap,
                            // 8 flow.
                            let mask = bound[6] && !e.lightmap;
                            let flow = bound[7] && e.slide_speed != 0.0;
                            let bits = u8::from(e.colors)
                                + 2 * u8::from(mask)
                                + 4 * u8::from(bound[6] && e.lightmap)
                                + 8 * u8::from(flow);
                            [
                                f32::from(bits),
                                e.slide_speed,
                                e.shadow_reduction,
                                e.shadow_brightening,
                            ]
                        }
                        None => [0.0; 4],
                    },
                    light_count: [
                        chosen.len() as u32,
                        u32::from(mesh.skin.is_some()),
                        bone_base,
                        0,
                    ],
                    light_index,
                };
                let draw = Draw {
                    pass,
                    depth: -view.transform_point3(centre).z,
                    hint: mat.transparency_hint,
                    uniform: uniforms.len() as u32,
                    material: surface.material,
                    mesh,
                    dynamic: dynamic_range,
                    bounds,
                    instance: instance as u32,
                };
                // A see-through mesh is drawn in two parts: what is solid
                // of it, then the rest.
                if pass == Pass::Blend {
                    let part = |which: f32| {
                        let [cube, sky, fade, _] = uniform.extra;
                        DrawUniform { extra: [cube, sky, fade, which], ..uniform }
                    };
                    uniforms.push(part(1.0));
                    draws.push(draw);
                    uniforms.push(part(2.0));
                    draws.push(Draw { pass: Pass::Fringe, uniform: draw.uniform + 1, ..draw });
                } else {
                    uniforms.push(uniform);
                    draws.push(draw);
                }
            }
        }
        self.drawn = Drawn { meshes: draws.len(), hidden, ..Drawn::default() };
        // Opaque (any order), then blended back to front by hint then depth,
        // then additive.
        draws.sort_by(|a, b| {
            let rank = |p: Pass| match p {
                Pass::Sky => 0,
                Pass::SkyFade => 1,
                Pass::Opaque => 2,
                Pass::Blend => 3,
                Pass::Fringe => 4,
                Pass::Additive => 5,
            };
            rank(a.pass).cmp(&rank(b.pass)).then_with(|| {
                if matches!(a.pass, Pass::Sky | Pass::SkyFade | Pass::Opaque) {
                    std::cmp::Ordering::Equal
                } else {
                    a.hint.cmp(&b.hint).then(b.depth.total_cmp(&a.depth))
                }
            })
        });

        // Draws of one mesh become one draw of as many instances, where
        // that takes no draw past one it meets (`batch.rs`): solid meshes
        // meet where their boxes do in the scene, see-through ones where
        // they do on the screen.
        let mut batcher = std::mem::take(&mut self.batcher);
        batcher.begin();
        let on_screen = proj * view;
        let mut batch_of = Vec::with_capacity(draws.len());
        let (mut pass, mut last) = (None, (u32::MAX, None));
        for d in &draws {
            let solid = matches!(d.pass, Pass::Sky | Pass::SkyFade | Pass::Opaque);
            if pass != Some(d.pass) {
                pass = Some(d.pass);
                last = (u32::MAX, None);
                // (Squares of a tile's size, and boxes a millimeter into
                // each other at most taken to touch; on the screen, a
                // sixteenth of it and a hundredth of a pixel.)
                if solid { batcher.pass(10.0, 1e-3) } else { batcher.pass(0.125, 1e-5) }
            }
            let key = (std::ptr::from_ref(d.mesh) as usize, d.material, d.dynamic);
            let batch = if !self.instancing {
                batcher.place(key, None, None)
            } else if solid {
                // (A model's own meshes in the order they came in.)
                let after = last.1.filter(|_| last.0 == d.instance);
                let batch = batcher.place(key, d.bounds, after);
                last = (d.instance, Some(batch));
                batch
            } else {
                batcher.place(key, d.bounds.map(|b| screen_box(on_screen, &b)), None)
            };
            batch_of.push(batch);
        }
        let (starts, order) = crate::batch::in_order(&batch_of, batcher.batches());
        self.batcher = batcher;
        self.drawn.batches = starts.len() - 1;
        // (Each instance's uniforms, by its number.)
        let named: Vec<u32> = order.iter().map(|&d| draws[d as usize].uniform).collect();
        let order_bytes = std::mem::size_of_val(named.as_slice()) as u64;
        if self.order_buffer.size() < order_bytes {
            self.order_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("draw order"),
                size: order_bytes.next_power_of_two(),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !named.is_empty() {
            gpu.queue.write_buffer(&self.order_buffer, 0, bytemuck::cast_slice(&named));
        }

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

        // Replaced vertices.
        let dynamic_bytes = std::mem::size_of_val(dynamic.as_slice());
        if (self.dynamic_buffer.size() as usize) < dynamic_bytes {
            self.dynamic_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dynamic vertices"),
                size: dynamic_bytes.next_power_of_two() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if dynamic_bytes > 0 {
            gpu.queue.write_buffer(&self.dynamic_buffer, 0, bytemuck::cast_slice(&dynamic));
        }

        // Upload per-draw data.
        let needed = std::mem::size_of_val(uniforms.as_slice()) as u64;
        if self.draw_buffer.size() < needed {
            self.draw_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("draws"),
                size: needed.next_power_of_two(),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !uniforms.is_empty() {
            gpu.queue.write_buffer(&self.draw_buffer, 0, bytemuck::cast_slice(&uniforms));
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
                wgpu::BindGroupEntry { binding: 3, resource: self.draw_buffer.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.order_buffer.as_entire_binding(),
                },
            ],
        });

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
            let mut names: [Option<String>; SLOTS] = Default::default();
            names[0] = b.texture.clone();
            let key: MaterialKey = (names, "chrome1".into(), (false, false));
            let group = self.material_group(gpu, assets, &key);
            let start = particle_vertices.len() as u32;
            match b.tint {
                Some(at) => {
                    let light = tint_light(scene, at);
                    particle_vertices.extend(b.vertices.iter().map(|v| {
                        let mut v = *v;
                        for (c, l) in v.color.iter_mut().zip(light.to_array()) {
                            *c *= l;
                        }
                        v
                    }));
                }
                None => particle_vertices.extend_from_slice(&b.vertices),
            }
            particle_draws.push((b.blend, group, start..particle_vertices.len() as u32));
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
        let line_vertices: Vec<LineVertex> = scene
            .lines
            .iter()
            .flat_map(|l| [l.from, l.to].map(|p| LineVertex { pos: p.to_array(), color: l.color }))
            .collect();
        let line_bytes = std::mem::size_of_val(line_vertices.as_slice());
        if (self.line_buffer.size() as usize) < line_bytes {
            self.line_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("lines"),
                size: line_bytes.next_power_of_two() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if line_bytes > 0 {
            gpu.queue.write_buffer(&self.line_buffer, 0, bytemuck::cast_slice(&line_vertices));
        }
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
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, &frame_group, &[]);
            // (A pipeline or a material that is the last draw's stays as
            // it is, and a draw's own values are found by its instance
            // number: binding a material and the draw's values anew for
            // every draw was a quarter of drawing a large area.)
            let (mut pipeline, mut material) = (None, None);
            let mut materials = 0;
            for batch in starts.windows(2) {
                // (A batch's draws are of one mesh and one material.)
                let d = &draws[order[batch[0] as usize] as usize];
                let key = (d.pass, d.mesh.skin.is_some());
                if pipeline != Some(key) {
                    pass.set_pipeline(&self.pipelines[&key]);
                    pipeline = Some(key);
                }
                if let Some(skin) = &d.mesh.skin {
                    pass.set_vertex_buffer(1, skin.vertices.slice(..));
                }
                if material != Some(d.material) {
                    pass.set_bind_group(1, &self.groups[d.material as usize], &[]);
                    material = Some(d.material);
                    materials += 1;
                }
                match d.dynamic {
                    Some((from, to)) => {
                        pass.set_vertex_buffer(0, self.dynamic_buffer.slice(from..to))
                    }
                    None => pass.set_vertex_buffer(0, d.mesh.vertices.slice(..)),
                }
                pass.set_index_buffer(d.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..d.mesh.index_count, 0, batch[0]..batch[1]);
            }
            self.drawn.materials = materials;
            if !particle_draws.is_empty() {
                pass.set_bind_group(0, &self.particle_frame, &[]);
                pass.set_vertex_buffer(0, self.particle_buffer.slice(..));
                for (blend, group, range) in &particle_draws {
                    pass.set_pipeline(&self.particle_pipelines[blend]);
                    pass.set_bind_group(1, &self.groups[*group as usize], &[]);
                    pass.draw(range.clone(), 0..1);
                }
            }
            if !line_vertices.is_empty() {
                pass.set_pipeline(&self.line_pipeline);
                pass.set_bind_group(0, &self.particle_frame, &[]);
                pass.set_vertex_buffer(0, self.line_buffer.slice(..));
                pass.draw(0..line_vertices.len() as u32, 0..1);
            }
        }
        gpu.queue.submit([encoder.finish()]);
        drop(draws);
        (self.uniforms, self.bones, self.dynamic) = (uniforms, bones, dynamic);
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

/// A box of the scene as the camera sees it (`on_screen`: its projection
/// times its view): the rectangle around it on the screen, from -1 to 1
/// each way; all of the screen for one that reaches behind the camera.
fn screen_box(on_screen: Mat4, bounds: &Bounds) -> Bounds {
    let (mut min, mut max) = (glam::Vec2::splat(f32::MAX), glam::Vec2::splat(f32::MIN));
    for corner in 0..8 {
        let at = Vec3::new(
            if corner & 1 == 0 { bounds.min.x } else { bounds.max.x },
            if corner & 2 == 0 { bounds.min.y } else { bounds.max.y },
            if corner & 4 == 0 { bounds.min.z } else { bounds.max.z },
        );
        let clip = on_screen * at.extend(1.0);
        if clip.w.is_nan() || clip.w <= 1e-4 {
            (min, max) = (glam::Vec2::splat(-1.0), glam::Vec2::splat(1.0));
            break;
        }
        let at = clip.truncate().truncate() / clip.w;
        (min, max) = (min.min(at), max.max(at));
    }
    // (What is off the screen meets nothing there.)
    Bounds {
        min: min.max(glam::Vec2::splat(-1.0)).extend(0.0),
        max: max.min(glam::Vec2::splat(1.0)).extend(0.0),
    }
}

/// The light tinted particles (`m_isTinted`) take at `at`, gamma space: the
/// area's ambient and diffuse colours added as they are (in the client, a
/// 0x40 ambient and 0x80 diffuse give 0xC0, whatever the particle's
/// facing), and the point lights there, at most 1 a channel.
pub fn tint_light(scene: &Scene, at: Vec3) -> Vec3 {
    let gamma = |c: Vec3| c.max(Vec3::ZERO).powf(1.0 / 2.2);
    let (max_inv, falloff) = attenuation_params();
    let mut points = Vec3::ZERO;
    for l in &scene.lights {
        let (d2, r2) = (l.position.distance_squared(at), l.cutoff * l.cutoff);
        if d2 < r2 {
            let f = d2 / r2;
            points += l.color * (1.0 - f) / (max_inv + falloff * f);
        }
    }
    (gamma(scene.area.ambient) + gamma(scene.area.diffuse) + gamma(points)).min(Vec3::ONE)
}

/// The depth buffer's format.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

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
        let depth = make("depth", DEPTH_FORMAT, samples, wgpu::TextureUsages::RENDER_ATTACHMENT)
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
    /// A component below zero is no light, not a number that is none (a
    /// power of a negative number), which the shader would spread.
    #[test]
    fn a_specular_color_with_a_negative_component_stays_a_color() {
        let mtr = |line: &str| mg_image::mtr::Mtr::parse(line.as_bytes());
        let c = super::custom_specular(&mtr("parameter float CustomSpecularColor 1.0 -0.5 0.5"));
        let c = c.expect("not black");
        assert!(c.is_finite(), "{c:?}");
        assert_eq!((c.x, c.y), (1.0, 0.0));
        assert!((c.z - 0.5f32.powf(2.2)).abs() < 1e-6);
        // Black, or none: the model's own.
        assert_eq!(super::custom_specular(&mtr("parameter float CustomSpecularColor 0 0 0")), None);
        assert_eq!(super::custom_specular(&mtr("renderhint NormalAndSpecMapped")), None);
    }

    /// What the client draws for tinted white particles (`client_render.rs`
    /// `particles_look`): ambient 0x40 grey and diffuse 0x80 grey give 0xC0;
    /// a red diffuse 0xC0 and green ambient 0x40 give (0xC0, 0x40, 0).
    #[test]
    fn tinted_particles_take_the_area_light_as_the_client() {
        use crate::{AreaLight, Scene};
        use glam::Vec3;
        let scene = |ambient, diffuse| Scene {
            area: AreaLight::from_are(ambient, diffuse, Vec3::Z),
            ..Default::default()
        };
        let tint = super::tint_light(&scene(0x404040, 0x808080), Vec3::ZERO) * 255.0;
        assert!((tint - Vec3::splat(192.0)).abs().max_element() < 0.5, "{tint}");
        let tint = super::tint_light(&scene(0x004000, 0x0000C0), Vec3::ZERO) * 255.0;
        assert!((tint - Vec3::new(192.0, 64.0, 0.0)).abs().max_element() < 0.5, "{tint}");
        assert_eq!(super::tint_light(&scene(0xFFFFFF, 0xFFFFFF), Vec3::ZERO), Vec3::ONE);
    }

    /// The client's `lightMaxIntensityInv` and `lightFalloffFactor` with the
    /// default settings.
    #[test]
    fn attenuation_matches_the_game() {
        let (max_inv, falloff) = super::attenuation_params();
        assert!((max_inv - 0.4098).abs() < 1e-3, "{max_inv}");
        assert!((falloff - 136.31).abs() < 0.1, "{falloff}");
    }

    /// A fragment shader stops where it discards. On Metal a discarded
    /// fragment that goes on running writes its depth: a see-through mesh's
    /// solid part hid what was behind the whole of it there (the tests of
    /// `tests/render.rs` see that on macOS alone; this one sees its cause
    /// anywhere).
    #[test]
    fn a_shader_returns_where_it_discards() {
        for (name, source) in [
            ("shader.wgsl", include_str!("shader.wgsl")),
            ("particle.wgsl", include_str!("particle.wgsl")),
            ("line.wgsl", include_str!("line.wgsl")),
        ] {
            let lines: Vec<&str> = source.lines().map(str::trim).collect();
            for (i, line) in lines.iter().enumerate() {
                if line.starts_with("//") || !line.contains("discard;") {
                    continue;
                }
                let next = lines.get(i + 1).copied().unwrap_or_default();
                assert!(
                    *line == "discard;" && next.starts_with("return "),
                    "{name}:{}: a discard without a return on the line after it",
                    i + 1
                );
            }
        }
    }
}
