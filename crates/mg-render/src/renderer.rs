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
/// The alpha at or below which a texel is cut out, whatever the texture's
/// TXI says (`blending punchthrough` or not, `decal` or not) and with an
/// MTR or without: the client's `fAlphaDiscardValue`, read from it
/// (`cutouts_look` with `MG_DISCARD=1` in `client_render.rs`).
const ALPHA_DISCARD: f32 = 0.2;
/// The alpha from which a see-through mesh's texel is drawn with its solid
/// part (`SOLID_ALPHA` in `shader.wgsl`): [`Pass::Blend`], hiding what is
/// behind it; below, with the rest ([`Pass::Fringe`]).
pub(crate) const SOLID_ALPHA: f32 = 0.95;
/// The depth bias of what is drawn over the solid scene without laying its
/// depth (an additive mesh, a see-through mesh's soft part), towards the
/// eye, by either shader: such a mesh often lies on another (a glow on a
/// body: the Domains of Dread's `dd_ce3_zombieash`, two skins of the same
/// vertices), and where one of the two is drawn by the game's programs and
/// the other by Moonglow's shader their depths differ by the last bits
/// (the projections and the skinning are reckoned apart), which flickered
/// in patches from frame to frame. The client draws both with one program.
/// 8 steps and one of the depth's slope: the least that measured clean
/// from 1.6 to 150 m was 4 and 1.
pub(crate) const OVERLAY_DEPTH_BIAS: wgpu::DepthBiasState =
    wgpu::DepthBiasState { constant: -8, slope_scale: -1.0, clamp: 0.0 };

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
    uv_turn: [f32; 4],
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
    /// Its alpha means something: an alpha channel with texels below
    /// opaque in it (a 32-bit picture all opaque has none, as the game
    /// takes it), or a BioWare DDS's alpha mean below 1.
    alpha: bool,
    /// The game cuts a mesh by it (`NO_DISCARD` 0): it has alpha, or its
    /// TXI's `alphamean` is below 1 (that decides, where given).
    cut: bool,
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

/// Whether the game's shiny water is drawn with its own programs
/// (`vswater`/`fswater`, which the client draws it with) where the game's
/// shaders are on, or with Moonglow's water (`docs/shaders-proposal.md`,
/// M5): Moonglow's. `fswater` sees what is under the water through the
/// framebuffer (`texFBColor`, `texFBDepth`), which Moonglow doesn't give
/// it; without it, the water is drawn opaque and pale, farther from the
/// client than Moonglow's own (TDoD's fountain 16.4 of 255 against 11.7, a
/// rural lake 19.4 against 12.6: `surfaces_in_the_client`).
pub const GAME_WATER: bool = false;

/// Texture slots: diffuse, normal, specular, roughness, height,
/// self-illumination; then the two a known custom shader reads
/// ([`Effect`]): an alpha mask or lightmap, and a flow map.
pub(crate) const SLOTS: usize = 8;
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
    /// The game's normal-mapped shader variant (`_nm`): a render hint, the
    /// model's or the MTR's. Without one the game draws `fslit`, which reads
    /// no map beyond the diffuse texture (read from the client): the maps
    /// are left out.
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
    /// The MTR's `parameter`s, which the game's shaders take by name.
    params: Vec<(String, mg_image::mtr::Param)>,
    /// The program pair the MTR names (`customshaderVS` and `FS`, both):
    /// drawn with it where the game's shaders are on
    /// ([`Renderer::game_shaders`]).
    custom: Option<(String, String)>,
    /// The MTR names a geometry program (`customshaderGS`), which wgpu has
    /// no stage for: drawn with Moonglow's own shader.
    geometry: Option<String>,
    /// The MTR's `twosided`: its faces are drawn from behind too. Nothing
    /// else shows a face's back in the game: not a cut-out texture
    /// (`blending punchthrough`), not `decal` (seen in the client:
    /// `cutouts_look` in `client_render.rs`).
    twosided: bool,
}

/// `m_texture` for a mesh of an instance: a tile's texture turned with it
/// where the mesh asks (`rotatetexture 1`), by the tile's turn; else the
/// identity ([`crate::Instance::tile`]).
fn texture_turn(inst: &Instance, mat: &crate::model::Material) -> Mat4 {
    if !(inst.tile && mat.rotate_texture) {
        return Mat4::IDENTITY;
    }
    let x = inst.transform.x_axis;
    Mat4::from_rotation_z(x.y.atan2(x.x))
}

/// Whether a texture's alpha means something ([`TextureEntry::alpha`]): an
/// alpha channel, with texels below opaque where they can be read (RGBA8
/// pictures: TGA, PLT; a BioWare DDS by its alpha mean).
fn alpha_in(t: &mg_image::Texture) -> bool {
    if !t.has_alpha {
        return false;
    }
    if let Some(mean) = t.alpha_mean {
        return mean < 1.0;
    }
    match (t.format, t.mips.first()) {
        (mg_image::Format::Rgba8, Some(level)) => {
            level.as_chunks::<4>().0.iter().any(|p| p[3] < 255)
        }
        _ => true,
    }
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
    /// The game cuts it by its texture's alpha (`NO_DISCARD` 0, with the
    /// draw's own reasons: [`crate::game::no_discard`]).
    cut: bool,
    /// What chooses the game's stock pair for it.
    traits: crate::game::Traits,
    /// It reflects an environment map, and that is a cube map.
    reflects: bool,
    env_cube: bool,
    /// Its bind group, in [`Renderer::groups`].
    material: u32,
    /// Shiny water's bind group for `fswater`, and whether its map is a
    /// cube: the area's map, as every lit mesh's without one of its own
    /// (read from the client: its `bumpyshinytexture` cube is not bound),
    /// where Moonglow's own water reflects that cube.
    water_material: Option<(u32, bool)>,
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
    /// Drawn from behind too ([`Slots::twosided`]).
    twosided: bool,
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
    /// Drawn with the game's shaders: which of the frame's see-through
    /// game draws it is.
    game: Option<u32>,
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

/// A mesh's pipeline of the game's programs, and a see-through one's
/// fringe's.
type GamePipelines = (Arc<crate::game::GamePipeline>, Option<Arc<crate::game::GamePipeline>>);

/// A mesh the game's shaders drew, and with what
/// ([`Renderer::game_programs`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameProgram {
    /// The model's name and the mesh's node's.
    pub model: String,
    pub node: String,
    /// The program pair, and the preamble's `NO_DISCARD`.
    pub vs: String,
    pub fs: String,
    pub no_discard: bool,
}

/// What [`GamePipelines`] are kept by ([`Renderer::game_pipelines`]).
type GamePipelinesKey = (usize, crate::game::Traits, bool, bool, bool, crate::game::GamePass);

#[derive(Debug)]
pub struct Renderer {
    pub debug: DebugView,
    /// Where `Some`, each frame lists there the program pairs the game's
    /// shaders chose for the meshes they draw (for comparing with the
    /// client's choice: `program_choice_in_the_client`).
    pub game_programs: Option<Vec<GameProgram>>,
    /// Lit meshes are drawn with the game's own shaders (`vslit`/`fslit`
    /// and their variants, compiled from the assets' `.shd`) where they
    /// can be, Moonglow's own shader drawing the rest and any whose
    /// program doesn't compile (`docs/shaders-proposal.md`, M1). Off by
    /// default.
    pub game_shaders: bool,
    /// Shiny water is drawn by the game's `vswater`/`fswater` too, where
    /// the game's shaders are on ([`GAME_WATER`] by default: Moonglow's own
    /// water; on, for comparing).
    pub game_water: bool,
    /// The game's programs are compiled, and their pipelines made, while
    /// the frames go on (a view drawn again and again: the area view, the
    /// model viewer), a mesh drawn with Moonglow's own shader until its
    /// pipeline is ready ([`Drawn::game_pending`]); off, a frame waits for
    /// them (a picture drawn once: a thumbnail, a preview, a test). Off by
    /// default.
    pub game_background: bool,
    game: crate::game::GameShaders,
    /// The pipelines of each mesh's slots ([`Renderer::game_pipelines`]): by
    /// the slots (their place), what chooses its stock pair, its
    /// `NO_DISCARD`, whether it is skinned, and its pass.
    game_pipelines: HashMap<GamePipelinesKey, Option<GamePipelines>>,
    /// Each material's key, by its place among [`Renderer::groups`].
    material_keys: Vec<MaterialKey>,
    /// Draws of one mesh are drawn as instances of one draw (on; off,
    /// each is drawn by itself, in the order they came in: to compare).
    pub instancing: bool,
    color_format: wgpu::TextureFormat,
    sample_count: u32,
    frame_layout: wgpu::BindGroupLayout,
    material_layout: wgpu::BindGroupLayout,
    /// By pass, whether the mesh is skinned and whether it is two-sided.
    pipelines: HashMap<(Pass, bool, bool), wgpu::RenderPipeline>,
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
    /// The meshes drawn with the game's own shaders
    /// ([`Renderer::game_shaders`]), each a draw of its own (two for a
    /// see-through one).
    pub game: usize,
    /// Pipelines of the game's programs being made
    /// ([`Renderer::game_background`]): meshes waiting for them were drawn
    /// with Moonglow's own shader, and a frame drawn again once they are
    /// ready shows them otherwise.
    pub game_pending: usize,
    /// Of those, the meshes whose programs read the clock
    /// (`worldtimerTimeOfDay`, `sceneCurrentFrame`): they move by
    /// themselves, and a view that shows them draws them again as time
    /// passes.
    pub timed: usize,
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

/// Puts `data` into `buffer`, which is made anew, larger, where it is too
/// small for it (or for `least` bytes, where a binding needs something
/// to be there with no data).
fn upload<T: bytemuck::Pod>(
    gpu: &Gpu,
    buffer: &mut wgpu::Buffer,
    label: &'static str,
    usage: wgpu::BufferUsages,
    data: &[T],
    least: usize,
) {
    let bytes = std::mem::size_of_val(data).max(least) as u64;
    if buffer.size() < bytes {
        *buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: bytes.next_power_of_two(),
            usage,
            mapped_at_creation: false,
        });
    }
    if !data.is_empty() {
        gpu.queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
    }
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
        for (pass, skinned, twosided) in passes
            .into_iter()
            .flat_map(|p| [(p, false), (p, true)])
            .flat_map(|(p, s)| [(p, s, false), (p, s, true)])
        {
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
                    cull_mode: if sky || twosided { None } else { Some(wgpu::Face::Back) },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(
                        !sky && !matches!(pass, Pass::Additive | Pass::Fringe),
                    ),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: if matches!(pass, Pass::Additive | Pass::Fringe) {
                        OVERLAY_DEPTH_BIAS
                    } else {
                        Default::default()
                    },
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
            pipelines.insert((pass, skinned, twosided), pipeline);
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
            game_programs: None,
            game_shaders: false,
            game_water: GAME_WATER,
            game_background: false,
            game: crate::game::GameShaders::new(gpu),
            game_pipelines: HashMap::new(),
            material_keys: Vec::new(),
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
        self.material_keys.clear();
        self.surfaces.clear();
        self.game.clear();
        self.game_pipelines.clear();
    }

    /// Whether a mesh is one the game's shaders draw (with them on): one
    /// whose material names a program pair; else one Moonglow's shader
    /// does nothing of its own for (no known custom shader's effect, no
    /// ripples, no water), with the stock pair (a decal's unlit). Shiny
    /// water only with [`Renderer::game_water`], rippling or not (`fswater`
    /// reads its texture's mean colour alone); a texture that only ripples
    /// (`proceduretype arturo`, a procedure the engine runs on the CPU)
    /// stays Moonglow's, which ripples it.
    fn game_path(&mut self, slots: &Slots, surface: &Surface) -> bool {
        if let Some(g) = &slots.geometry {
            self.game.fail(g, "it is a geometry program, which wgpu has no stage for".into());
            return false;
        }
        let own = if surface.water { !self.game_water } else { surface.ripple.is_some() };
        slots.custom.is_some() || (slots.effect.is_none() && !own)
    }

    /// The pipelines a mesh of these slots is drawn with by the game's
    /// shaders (a see-through one's two parts: its fringe's second); kept
    /// by the slots, so that a frame's draws look up no programs by name.
    #[allow(clippy::too_many_arguments)]
    fn game_pipelines(
        &mut self,
        gpu: &Gpu,
        assets: &dyn Assets,
        slots: &Arc<Slots>,
        traits: crate::game::Traits,
        no_discard: bool,
        skinned: bool,
        tangents: bool,
        pass: crate::game::GamePass,
    ) -> Option<GamePipelines> {
        use crate::game::{GamePass, PipelineKey};
        let at = (Arc::as_ptr(slots) as usize, traits, no_discard, skinned, tangents, pass);
        if let Some(p) = self.game_pipelines.get(&at) {
            return p.clone();
        }
        let (vs, fs, _) = crate::game::program_pair(slots.custom.as_ref(), traits);
        let twosided = slots.twosided;
        let key = PipelineKey { vs, fs, no_discard, skinned, tangents, twosided, pass };
        let (format, samples, wait) = (self.color_format, self.sample_count, !self.game_background);
        use crate::game::Ready;
        let p = match self.game.pipeline(gpu, assets, &key, format, samples, wait) {
            Ready::Yes(p) if pass == GamePass::Solid => {
                let fringe = PipelineKey { pass: GamePass::Fringe, ..key };
                match self.game.pipeline(gpu, assets, &fringe, format, samples, wait) {
                    Ready::Yes(q) => Some((p, Some(q))),
                    Ready::No => None,
                    Ready::Pending => return None,
                }
            }
            Ready::Yes(p) => Some((p, None)),
            Ready::No => None,
            // (Drawn with Moonglow's own until it is made; not kept.)
            Ready::Pending => return None,
        };
        self.game_pipelines.insert(at, p.clone());
        p
    }

    /// Keeps the game's compiled programs in `dir` between sessions (the
    /// user's cache folder: what is made of the game's shaders stays on
    /// the user's machine), or nowhere. Read where a program compiled
    /// before is drawn again.
    pub fn set_shader_cache(&mut self, dir: Option<std::path::PathBuf>) {
        self.game.cache = dir.map(mg_shader::ProgramCache::new);
    }

    /// Calls `wake` when a pipeline of the game's programs is ready
    /// ([`Renderer::game_background`]), from the thread that made it: a
    /// window asks for a frame.
    pub fn set_game_waker(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        if let Ok(mut w) = self.game.waker.lock() {
            *w = Some(crate::game::Waker(Arc::new(wake)));
        }
    }

    /// What has been compiled and read from the cache, and how long the
    /// pipelines took to make.
    pub fn game_shader_stats(&self) -> crate::game::ShaderStats {
        self.game.stats
    }

    /// The game's shaders that couldn't be drawn ([`Renderer::game_shaders`]),
    /// as `vs/fs` pairs or programs, and why: drawn with Moonglow's own.
    pub fn game_shader_failures(&self) -> &[(String, String)] {
        &self.game.failures
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
            out.params = mtr.params.clone();
            out.custom =
                crate::game::custom_pair(mtr.shader_vs.as_deref(), mtr.shader_fs.as_deref());
            out.geometry = mtr
                .shader_gs
                .as_deref()
                .map(|g| g.trim().to_ascii_lowercase())
                .filter(|g| !g.is_empty() && g != "null");
            let fs = mtr.shader_fs.as_deref().unwrap_or_default().to_ascii_lowercase();
            let known = EFFECT_SHADERS.iter().any(|known| fs.starts_with(known));
            out.transparency = mtr.transparency;
            out.twosided = mtr.twosided;
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
            if known
                && let Some((vs, fs)) = &out.custom
                && (assets.shader(vs).is_none() || assets.shader(fs).is_none())
            {
                // (Its source isn't there: NWN Mapper ships none of
                // `mzlm`'s. The hand-ported effect draws it, as with the
                // game's shaders off.)
                out.custom = None;
            }
            if mapper {
                // Its lightmap is where the other pair has its flow map.
                out.names[6] = out.names[7].take();
            }
        }
        out.normal_variant = mat.normal_mapped
            || mtr.as_ref().is_some_and(|m| m.renderhint != mg_image::mtr::RenderHint::None);
        if out.effect.is_none() && out.custom.is_none() {
            // (A custom shader's own slots mean nothing to the stock ones;
            // nor do the maps to `fslit`, without a render hint.)
            let from = if out.normal_variant { STOCK_SLOTS } else { 1 };
            for name in &mut out.names[from..] {
                *name = None;
            }
        }
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
        self.material_keys.push(key.clone());
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
        let (blending, env, decal, clamp, has_alpha, cut) = match &tex {
            Some(t) => (t.blending, t.env.clone(), t.decal, t.clamp, t.alpha, t.cut),
            None => (Blending::Default, None, false, (false, false), false, false),
        };
        // The environment map a mesh reflects: its texture's (TXI
        // `envmaptexture`), else its object's (placeables.2da
        // `Reflection`, appearance.2da `ENVMAP`); `default` the area's
        // (the tileset's `EnvMap`), else `chrome1` (a placeable's
        // `Reflection` `default` is `chrome1` whatever the area's, and
        // comes here so: `mg_preview::placeable`). And the one every lit
        // mesh's specular light reflects, bound to `texUnitEnv`: its own,
        // else the area's (read from the client:
        // `program_choice_in_the_client` in `client_render.rs`).
        let area = || scene_env.map_or_else(|| "chrome1".to_string(), str::to_string);
        let resolved = |e: String| if e.eq_ignore_ascii_case("default") { area() } else { e };
        let object = inst.env_map.clone().map(resolved);
        let env = match env {
            Some(e) if e.eq_ignore_ascii_case("default") => Some(object.unwrap_or_else(area)),
            Some(e) => Some(e),
            None => object,
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
        let env = env.unwrap_or_else(area).to_ascii_lowercase();
        let env_cube = self.texture(gpu, assets, &env).is_some_and(|t| t.cube);
        let key: MaterialKey =
            (std::array::from_fn(|i| if bound[i] { names[i].clone() } else { None }), env, clamp);
        let hint = slots.normal_variant;
        let water = tex.as_ref().is_some_and(|t| t.water);
        let water_material = water.then(|| {
            let env = area().to_ascii_lowercase();
            let cube = self.texture(gpu, assets, &env).is_some_and(|t| t.cube);
            (self.material_group(gpu, assets, &(key.0.clone(), env, key.2)), cube)
        });
        Surface {
            water_material,
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
            // (An environment-mapped texture's alpha is its reflectivity.)
            cut: cut && !env_mapped,
            traits: crate::game::Traits {
                uvs: mat.uvs,
                reflects,
                hint,
                decal,
                water,
                grass: inst.grass,
                sky: false,
            },
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
                    alpha: false,
                    cut: false,
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
            let alpha = alpha_in(&t.texture);
            Arc::new(TextureEntry {
                cube: false,
                alpha,
                cut: t.txi.alpha_mean().map_or(alpha, |mean| mean < 1.0),
                gpu: GpuTexture::upload(gpu, name, &t.texture, t.txi.mipmap()),
                blending: t.txi.blending(),
                // (Water reflects its `bumpyshinytexture`, as the game's
                // shiny water does: Moonglow's stands in for it.)
                env: t
                    .txi
                    .envmap()
                    .or_else(|| t.txi.water().then(|| t.txi.bumpy_shiny()).flatten())
                    .map(str::to_ascii_lowercase),
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
        // What the game's shaders share across the frame's draws.
        let game_frame = self.game_shaders.then(|| {
            let (max_intensity_inv, falloff) = attenuation_params();
            crate::game::FrameValues {
                proj: camera.projection_gl(size.0 as f32 / size.1.max(1) as f32),
                view,
                size: (size.0.max(1) as f32, size.1.max(1) as f32),
                near: camera.near,
                far: camera.far,
                area_ambient: scene.area.ambient,
                area_diffuse: scene.area.diffuse,
                area_direction: area_dir,
                max_intensity_inv,
                falloff,
                fog,
                fog_color: frame.fog_color,
                seconds: scene.time,
                eye: camera.eye,
                target: camera.target,
                fov_y: camera.fov_y,
                environment: scene.environment,
                ..Default::default()
            }
            .derive()
        });
        // (What the worker made since the last frame is drawn from now.)
        self.game.poll();
        // The solid meshes drawn with the game's shaders (drawn after
        // Moonglow's own solid ones), and the see-through ones' two parts
        // (each in `draws`, sorted with the rest).
        let mut game_draws: Vec<crate::game::GameDraw<'_>> = Vec::new();
        let mut game_blended: Vec<crate::game::GameDraw<'_>> = Vec::new();
        let mut timed = 0;

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
        upload(
            gpu,
            &mut self.light_buffer,
            "lights",
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &lights,
            std::mem::size_of::<LightUniform>(),
        );
        self.game.begin(bytemuck::cast_slice(&lights));

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
        // (The lights as a game draw's blocks take them.)
        let frame_lights: &[[[f32; 4]; 2]] = bytemuck::cast_slice(&lights);
        // (The sets of lights the game's draws are lit by, each numbered;
        // and the last draw's, which the next often shares.)
        let mut light_sets: crate::game::Fast<Box<[u32]>, u32> = Default::default();
        let mut light_set: Vec<u32> = Vec::new();
        let mut last_set: (Vec<u32>, u32) = (Vec::new(), u32::MAX);
        let scene_env = scene.env_map.as_deref();
        let scene_env_id = self.name_id(scene_env);
        let mut hidden = 0;
        if let Some(list) = &mut self.game_programs {
            list.clear();
        }
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
                // See-through of its own, whatever the instance's opacity.
                // (A cut-out is solid where it is not cut: its rim is not
                // blended, as the client draws it.)
                let see_through = (surface.has_alpha
                    && !surface.reflects
                    && surface.blending != Blending::Punchthrough)
                    || mat.transparency_hint > 0
                    || slots.effect.is_some_and(|e| e.blend);
                let pass = if fade_color.is_some() {
                    Pass::SkyFade
                } else if is_sky {
                    Pass::Sky
                } else if surface.blending == Blending::Additive {
                    Pass::Additive
                } else if alpha < 1.0 || see_through {
                    Pass::Blend
                } else {
                    Pass::Opaque
                };
                // A mesh solid of its own in an instance drawn see-through
                // (a tile or a blueprint about to be placed): its depth is
                // laid first, so that the instance's nearer surfaces hide
                // its farther ones (a tile's water, nearer the eye by its
                // middle, was drawn over its bridge).
                let ghost_solid = pass == Pass::Blend
                    && inst.opacity < 1.0
                    && alpha / inst.opacity >= 1.0
                    && !see_through;
                let discard = ALPHA_DISCARD;
                // The 32 most important lights that reach the mesh.
                reach.reaching(centre, radius, &mut chosen);
                let mut light_index = [[0u32; 4]; 8];
                for (j, (_, _, i)) in chosen.iter().enumerate() {
                    light_index[j / 4][j % 4] = *i;
                }
                // The game's own shaders, where they can draw it: a mesh
                // whose material names a program pair, with that pair; else
                // a lit one Moonglow's shader does nothing of its own for,
                // with the stock pair the engine would choose. Solid, or
                // see-through in two parts as Moonglow draws it.
                let game_pass = match pass {
                    Pass::Opaque => Some(crate::game::GamePass::Opaque),
                    Pass::Blend => Some(crate::game::GamePass::Solid),
                    Pass::Additive => Some(crate::game::GamePass::Additive),
                    Pass::Sky => Some(crate::game::GamePass::Sky),
                    // (The horizon's fade: Moonglow's, in the fog's colour.)
                    Pass::SkyFade | Pass::Fringe => None,
                };
                // (The skybox: `vsvt`/`fst`, read from the client.)
                let traits = crate::game::Traits { sky: is_sky, ..surface.traits };
                if let Some(f) = &game_frame
                    && let Some(game_pass) = game_pass
                    && !inst.unlit
                    // (A blueprint about to be placed, see-through, is
                    // Moonglow's: it lays its depth first.)
                    && inst.opacity >= 1.0
                    && self.debug == DebugView::Lit
                    && mesh.skin.as_ref().is_none_or(|s| s.bones.len() <= 64)
                    && self.game_path(slots, surface)
                {
                    // (`NO_DISCARD`: the material's, and the mesh's alpha
                    // as it is now.)
                    // (The skybox and shiny water: 1 whatever their
                    // textures' alpha, read from the client.)
                    let no_discard = traits.sky
                        || (traits.water && slots.custom.is_none())
                        || crate::game::no_discard(
                            surface.textured,
                            surface.cut,
                            alpha,
                            slots.transparency,
                        );
                    if let Some(list) = &mut self.game_programs {
                        let (vs, fs, _) = crate::game::program_pair(slots.custom.as_ref(), traits);
                        let node = |n: usize| inst.model.model.nodes.get(n).map(|n| n.name.clone());
                        list.push(GameProgram {
                            model: inst.model.model.name.clone(),
                            node: node(mesh.node).unwrap_or_default(),
                            vs,
                            fs,
                            no_discard,
                        });
                    }
                    let pipelines = self.game_pipelines(
                        gpu,
                        assets,
                        slots,
                        traits,
                        no_discard,
                        mesh.skin.is_some(),
                        // (The engine makes a mesh tangents for a render
                        // hint alone, the model's or its MTR's.)
                        !mesh.tangents_made || slots.normal_variant,
                        game_pass,
                    );
                    // (Shiny water: the area's map.)
                    let (material, env_cube) = match surface.water_material {
                        Some(m) if traits.water && slots.custom.is_none() => m,
                        _ => (surface.material, surface.env_cube),
                    };
                    if let Some((p, fringe)) = pipelines {
                        light_set.clear();
                        light_set.extend(chosen.iter().map(|(_, _, i)| *i));
                        let lights_id = if last_set.1 != u32::MAX && last_set.0 == light_set {
                            last_set.1
                        } else {
                            let next = light_sets.len() as u32;
                            let n = match light_sets.get(&light_set[..]) {
                                Some(&n) => n,
                                None => {
                                    light_sets.insert(light_set.clone().into_boxed_slice(), next);
                                    next
                                }
                            };
                            last_set.0.clone_from(&light_set);
                            last_set.1 = n;
                            n
                        };
                        let d = crate::game::DrawValues {
                            world,
                            bones: &bones[bone_base as usize..],
                            ambient: lin(mat.ambient).extend(1.0).to_array(),
                            diffuse: lin(mat.diffuse).extend(alpha).to_array(),
                            emissive: lin(emissive).extend(1.0).to_array(),
                            bound: std::array::from_fn(|i| bound.get(i).copied().unwrap_or(false)),
                            env_cube,
                            alpha_discard: ALPHA_DISCARD,
                            frame_lights,
                            light_index: &light_set,
                            lights: lights_id,
                            bbox: (mesh.min, mesh.max),
                            params: &slots.params,
                            material,
                            texture_matrix: texture_turn(inst, mat),
                            blend_mode: 0,
                        };
                        timed += usize::from(p.timed);
                        let geometry = crate::game::Geometry::Mesh(mesh);
                        let draw = self.game.push(p, material, geometry, dynamic_range, f, &d);
                        match fringe {
                            None if game_pass == crate::game::GamePass::Opaque => {
                                game_draws.push(draw)
                            }
                            // (Additive, the sky: a draw of its own, in its
                            // place among the others.)
                            None => {
                                let at = game_blended.len() as u32;
                                game_blended.push(draw);
                                draws.push(Draw {
                                    pass,
                                    twosided: slots.twosided,
                                    depth: -view.transform_point3(centre).z,
                                    hint: mat.transparency_hint,
                                    uniform: 0,
                                    material: surface.material,
                                    mesh,
                                    dynamic: dynamic_range,
                                    game: Some(at),
                                    bounds,
                                    instance: instance as u32,
                                });
                            }
                            Some(q) => {
                                // Its two parts, among the see-through
                                // meshes (the same blocks for both).
                                let rest = self.game.again(&draw, q, f);
                                let at = game_blended.len() as u32;
                                game_blended.push(draw);
                                game_blended.push(rest);
                                let draw = Draw {
                                    pass: Pass::Blend,
                                    twosided: slots.twosided,
                                    depth: -view.transform_point3(centre).z,
                                    hint: mat.transparency_hint,
                                    uniform: 0,
                                    material: surface.material,
                                    mesh,
                                    dynamic: dynamic_range,
                                    game: Some(at),
                                    bounds,
                                    instance: instance as u32,
                                };
                                draws.push(draw);
                                draws.push(Draw { pass: Pass::Fringe, game: Some(at + 1), ..draw });
                            }
                        }
                        continue;
                    }
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
                    // (w: which part of a see-through mesh, below; 3 a
                    // cut-out, solid where it is not cut.)
                    extra: [
                        flag(surface.env_cube),
                        flag(is_sky),
                        flag(fade_color.is_some()),
                        if surface.blending == Blending::Punchthrough { 3.0 } else { 0.0 },
                    ],
                    water: {
                        let (far, fast) = surface.ripple.unwrap_or((0.0, 0.0));
                        [far, fast, flag(surface.water), 0.0]
                    },
                    uv_turn: {
                        let m = texture_turn(inst, mat);
                        [m.x_axis.x, m.x_axis.y, 0.0, 0.0]
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
                    // (w: the tangents were made, not the model's: the
                    // tangent frame is taken from the derivatives.)
                    light_count: [
                        chosen.len() as u32,
                        u32::from(mesh.skin.is_some()),
                        bone_base,
                        u32::from(mesh.tangents_made),
                    ],
                    light_index,
                };
                let draw = Draw {
                    pass,
                    twosided: slots.twosided,
                    depth: -view.transform_point3(centre).z,
                    hint: mat.transparency_hint,
                    uniform: uniforms.len() as u32,
                    material: surface.material,
                    mesh,
                    dynamic: dynamic_range,
                    game: None,
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
                    // (Its depth alone, before any of the rest is drawn.)
                    if ghost_solid {
                        uniforms.push(part(4.0));
                        draws.push(Draw { depth: f32::INFINITY, ..draw });
                    }
                    let draw = Draw { uniform: uniforms.len() as u32, ..draw };
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
        self.drawn = Drawn {
            meshes: draws.len() + game_draws.len(),
            hidden,
            game: game_draws.len() + game_blended.len(),
            timed,
            game_pending: self.game.busy(),
            ..Drawn::default()
        };
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
            // (A see-through, additive or sky draw of the game's: with
            // others of its pipeline, material and mesh where they don't
            // meet on the screen, as Moonglow's own; a skinned one by
            // itself.)
            let game_key = d.game.map(|i| game_blended[i as usize].batch_key());
            let key = match game_key {
                Some(Some((pipeline, material))) => {
                    (std::ptr::from_ref(d.mesh) as usize, material, pipeline + 1, d.dynamic)
                }
                Some(None) => (usize::MAX - d.game.unwrap_or(0) as usize, u32::MAX, u32::MAX, None),
                None => (std::ptr::from_ref(d.mesh) as usize, d.material, 0, d.dynamic),
            };
            let batch = if !self.instancing || matches!(game_key, Some(None)) {
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
        // The game's draws among them in the order they are drawn, so that
        // a batch's are instances in a row.
        if !game_blended.is_empty() {
            let mut taken: Vec<Option<crate::game::GameDraw<'_>>> =
                game_blended.drain(..).map(Some).collect();
            for &k in &order {
                if let Some(i) = draws[k as usize].game {
                    let at = game_blended.len() as u32;
                    game_blended.extend(taken[i as usize].take());
                    draws[k as usize].game = Some(at);
                }
            }
        }
        // (And the game's solid draws', as they are drawn.)
        self.drawn.batches = starts.len() - 1;
        // (Each instance's uniforms, by its number.)
        let named: Vec<u32> = order.iter().map(|&d| draws[d as usize].uniform).collect();
        upload(
            gpu,
            &mut self.order_buffer,
            "draw order",
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &named,
            0,
        );

        // Bones.
        upload(
            gpu,
            &mut self.bone_buffer,
            "bones",
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &bones,
            64,
        );

        // Replaced vertices.
        upload(
            gpu,
            &mut self.dynamic_buffer,
            "dynamic vertices",
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &dynamic,
            0,
        );

        // Upload per-draw data.
        upload(
            gpu,
            &mut self.draw_buffer,
            "draws",
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            &uniforms,
            0,
        );

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
            particle_draws.push((b.blend, group, start..particle_vertices.len() as u32, None));
        }
        // The particles through `vsparticle`/`fsparticle` (read from the
        // client: `NO_DISCARD` 1 but for punch-through, `blendMode` the
        // blend); where their pipeline isn't ready, Moonglow's own.
        if let Some(f) = &game_frame
            && self.debug == DebugView::Lit
        {
            use crate::game::{GamePass, PipelineKey, Ready};
            let (format, samples, wait) =
                (self.color_format, self.sample_count, !self.game_background);
            for (k, (blend, group, range, game)) in particle_draws.iter_mut().enumerate() {
                let key = PipelineKey {
                    vs: "vsparticle".into(),
                    fs: "fsparticle".into(),
                    no_discard: *blend != ParticleBlend::PunchThrough,
                    skinned: false,
                    tangents: true,
                    twosided: false,
                    pass: GamePass::Particles(*blend),
                };
                let Ready::Yes(p) = self.game.pipeline(gpu, assets, &key, format, samples, wait)
                else {
                    continue;
                };
                let textured = batches[k]
                    .texture
                    .as_deref()
                    .is_some_and(|t| self.texture(gpu, assets, t).is_some());
                let mut bound = [false; 16];
                bound[0] = textured;
                let d = crate::game::DrawValues {
                    world: Mat4::IDENTITY,
                    ambient: [1.0; 4],
                    diffuse: [1.0; 4],
                    emissive: [0.0, 0.0, 0.0, 1.0],
                    bound,
                    alpha_discard: ALPHA_DISCARD,
                    lights: u32::MAX,
                    material: *group,
                    texture_matrix: Mat4::IDENTITY,
                    blend_mode: match blend {
                        ParticleBlend::Normal => 0,
                        ParticleBlend::Lighten => 1,
                        ParticleBlend::PunchThrough => 2,
                    },
                    ..Default::default()
                };
                if let Some(list) = &mut self.game_programs {
                    list.push(GameProgram {
                        model: "(particles)".into(),
                        node: format!("{blend:?}"),
                        vs: key.vs.clone(),
                        fs: key.fs.clone(),
                        no_discard: key.no_discard,
                    });
                }
                let geometry = crate::game::Geometry::Particles(range.start, range.end);
                *game = Some(game_blended.len());
                game_blended.push(self.game.push(p, *group, geometry, None, f, &d));
                self.drawn.game += 1;
            }
            self.drawn.game_pending = self.game.busy();
        }
        upload(
            gpu,
            &mut self.particle_buffer,
            "particles",
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &particle_vertices,
            0,
        );
        let line_vertices: Vec<LineVertex> = scene
            .lines
            .iter()
            .flat_map(|l| [l.from, l.to].map(|p| LineVertex { pos: p.to_array(), color: l.color }))
            .collect();
        upload(
            gpu,
            &mut self.line_buffer,
            "lines",
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &line_vertices,
            0,
        );
        // The game's shaders' blocks and bind groups; the solid draws by
        // pipeline and material, each set once for its draws.
        game_draws.sort_by_key(|d| d.order());
        if !game_draws.is_empty() || !game_blended.is_empty() {
            self.game.upload(gpu, &mut game_draws, &mut game_blended);
            let (white, white_cube) = (self.white.clone(), self.white_cube.clone());
            let noise = self.texture(gpu, assets, "solid_noise");
            let env_sampler = self.env_sampler.clone();
            for d in game_draws.iter().chain(&game_blended) {
                if self.game.prepared(d) {
                    continue;
                }
                let key = self.material_keys[d.material as usize].clone();
                let textures: Vec<Option<Arc<TextureEntry>>> = key
                    .0
                    .iter()
                    .map(|n| n.as_deref().and_then(|t| self.texture(gpu, assets, t)))
                    .collect();
                let env = self.texture(gpu, assets, &key.1);
                let sampler = self.sampler(gpu, key.2);
                let (env_view, cube_view) = match &env {
                    Some(t) if t.cube => (&white.view, &t.gpu.view),
                    Some(t) => (&t.gpu.view, &white_cube.view),
                    None => (&white.view, &white_cube.view),
                };
                let views = crate::game::MaterialViews {
                    slots: std::array::from_fn(|i| textures[i].as_ref().map(|t| &t.gpu.view)),
                    env: env_view,
                    env_cube: cube_view,
                    noise: noise.as_ref().map_or(&white.view, |t| &t.gpu.view),
                    white: &white.view,
                    white_cube: &white_cube.view,
                    sampler: &sampler,
                    env_sampler: &env_sampler,
                };
                self.game.prepare(gpu, d, &views);
            }
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
            // The game's shaders' draws, after the solid ones of Moonglow's.
            let mut game_drawn = game_draws.is_empty();
            // (What the game's draws left bound, and whether Moonglow's own
            // must bind its frame again before it draws.)
            let mut game_bound = crate::game::Bound3::default();
            let mut own_lost = false;
            for batch in starts.windows(2) {
                // (A batch's draws are of one mesh and one material.)
                let d = &draws[order[batch[0] as usize] as usize];
                if !game_drawn && !matches!(d.pass, Pass::Sky | Pass::SkyFade | Pass::Opaque) {
                    let (dynamic, particles) = (&self.dynamic_buffer, &self.particle_buffer);
                    self.drawn.batches +=
                        self.game.draw(&mut pass, &game_draws, dynamic, particles, &mut game_bound);
                    game_drawn = true;
                    own_lost = true;
                }
                if let Some(i) = d.game {
                    // (The batch's draws, in a row.)
                    let n = (batch[1] - batch[0]) as usize;
                    let draws = &game_blended[i as usize..i as usize + n];
                    let (dynamic, particles) = (&self.dynamic_buffer, &self.particle_buffer);
                    let _ = self.game.draw(&mut pass, draws, dynamic, particles, &mut game_bound);
                    own_lost = true;
                    continue;
                }
                if own_lost {
                    pass.set_bind_group(0, &frame_group, &[]);
                    (pipeline, material) = (None, None);
                    game_bound = crate::game::Bound3::default();
                    own_lost = false;
                }
                let key = (d.pass, d.mesh.skin.is_some(), d.twosided);
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
            if !game_drawn {
                let (dynamic, particles) = (&self.dynamic_buffer, &self.particle_buffer);
                self.drawn.batches +=
                    self.game.draw(&mut pass, &game_draws, dynamic, particles, &mut game_bound);
            }
            self.drawn.materials = materials;
            // (Moonglow's particle pipeline's frame and vertices, bound
            // again after a game draw.)
            let mut own_bound = false;
            for (blend, group, range, game) in &particle_draws {
                if let Some(i) = game {
                    let draw = std::slice::from_ref(&game_blended[*i]);
                    let (dynamic, particles) = (&self.dynamic_buffer, &self.particle_buffer);
                    let _ = self.game.draw(&mut pass, draw, dynamic, particles, &mut game_bound);
                    own_bound = false;
                    continue;
                }
                if !own_bound {
                    pass.set_bind_group(0, &self.particle_frame, &[]);
                    pass.set_vertex_buffer(0, self.particle_buffer.slice(..));
                    own_bound = true;
                }
                pass.set_pipeline(&self.particle_pipelines[blend]);
                pass.set_bind_group(1, &self.groups[*group as usize], &[]);
                pass.draw(range.clone(), 0..1);
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

    /// The WGSL shaders' cube maps, by their names.
    fn cube_maps(source: &str) -> Vec<String> {
        source
            .lines()
            .filter(|l| l.contains("texture_cube"))
            .filter_map(|l| l.split("var ").nth(1)?.split(':').next())
            .map(|n| n.trim().to_string())
            .collect()
    }

    /// No cube map is read with gradients or a bias: naga writes
    /// `textureSampleGrad` on a cube for Metal with 2D gradients
    /// (`metal::gradient2d` of `float3`s), which Metal refuses, and the
    /// whole lit pipeline wasn't made there (M4's CI on macOS).
    #[test]
    fn no_cube_map_is_read_with_gradients() {
        for (name, source) in [
            ("shader.wgsl", include_str!("shader.wgsl")),
            ("particle.wgsl", include_str!("particle.wgsl")),
            ("line.wgsl", include_str!("line.wgsl")),
        ] {
            let cubes = cube_maps(source);
            for (i, line) in source.lines().enumerate() {
                for cube in &cubes {
                    for call in ["textureSampleGrad(", "textureSampleBias("] {
                        let used = line.replace(' ', "").contains(&format!("{call}{cube},"));
                        assert!(!used, "{name}:{}: {call} on the cube map {cube}", i + 1);
                    }
                }
            }
        }
        assert_eq!(cube_maps(include_str!("shader.wgsl")), ["tex_env_cube"]);
    }

    /// The shaders as naga writes them for Metal: no cube map sampled with
    /// 2D gradients.
    #[test]
    fn the_shaders_written_for_metal_sample_cube_maps_as_cubes() {
        for (name, source) in [
            ("shader.wgsl", include_str!("shader.wgsl")),
            ("particle.wgsl", include_str!("particle.wgsl")),
            ("line.wgsl", include_str!("line.wgsl")),
        ] {
            let module =
                naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {e}"));
            let info = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let options = naga::back::msl::Options {
                lang_version: (2, 1),
                fake_missing_bindings: true,
                ..Default::default()
            };
            let pipeline = naga::back::msl::PipelineOptions::default();
            let (msl, _) = naga::back::msl::write_string(&module, &info, &options, &pipeline)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let cubes: Vec<&str> = msl
                .lines()
                .filter(|l| l.contains("metal::texturecube<"))
                .filter_map(|l| l.split('>').nth(1)?.split_whitespace().next())
                .collect();
            if name == "shader.wgsl" {
                assert!(!cubes.is_empty(), "no cube map found in the Metal source");
            }
            for line in msl.lines().filter(|l| l.contains("gradient2d")) {
                for cube in &cubes {
                    assert!(
                        !line.contains(&format!("{cube}.sample(")),
                        "{name}: a cube map with 2D gradients: {line}"
                    );
                }
            }
        }
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
