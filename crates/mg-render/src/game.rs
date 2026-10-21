//! The game's own shaders drawing Moonglow's meshes (`docs/shaders-proposal.md`,
//! M1): a stock program pair (`vslit`/`fslit` and their `_nm`, `_sm`,
//! `_sm_nm` variants) compiled from the install by `mg-shader` and drawn
//! beside Moonglow's WGSL shader, chosen per draw.
//!
//! - **Vertex inputs** are bound by name to [`Vertex`]'s and [`SkinVertex`]'s
//!   fields (notes §B.1): the vertex buffer layout is built from the
//!   program's reflected locations, which glslang numbers in its own order.
//!   The bone indices, floats in the game's shaders, are read from
//!   Moonglow's `u32`s ([`Program::read_input_as_uint`]); inputs Moonglow
//!   has no data for (`vColor`, the engine's baked static light, with
//!   `staticLighting` 0) read zeros from a buffer of their own.
//! - **Varyings** are matched by name ([`Program::link_inputs`]).
//! - **Bind groups**: the vertex program's resources in group 0, the
//!   fragment program's in group 1 (each binds its own uniform block at
//!   binding 0), each block at a dynamic offset into one buffer of the
//!   frame's blocks.
//! - **The uniform contract**: every member of a block is filled by name
//!   from what the renderer has ([`key`]); names it doesn't know are an
//!   MTR `parameter` of that name, else zero.
//! - **Clip space**: the matrices are OpenGL's, as the engine uploads them
//!   (`m_proj` with depth from -1 to 1); `mg-shader` takes the position to
//!   wgpu's depth range as it leaves the vertex program.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_image::mtr::Param as MtrParam;
use mg_shader::{
    AlphaKeep, Member, Preamble, Program, Reflection, Scalar, Sources, Stage, Texture,
    UniformBlock, ValueType,
};

use crate::Gpu;
use crate::model::{SkinVertex, Vertex};

/// Where a vertex input of the game's comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Attr {
    /// [`Vertex`], at an offset, in a format.
    Vertex(u64, wgpu::VertexFormat),
    /// [`SkinVertex`].
    Skin(u64, wgpu::VertexFormat),
    /// Zeros: data Moonglow doesn't have.
    Zero,
}

/// A vertex input of the game's shaders, by the name the engine binds it
/// to (notes §B.1), from Moonglow's vertex.
pub(crate) fn attribute(name: &str, skinned: bool) -> Attr {
    use std::mem::offset_of;

    use wgpu::VertexFormat as F;
    let v = |offset: usize, format| Attr::Vertex(offset as u64, format);
    match name {
        "vPos" => v(offset_of!(Vertex, pos), F::Float32x3),
        "vNormal" => v(offset_of!(Vertex, normal), F::Float32x3),
        "vTcIn" => v(offset_of!(Vertex, uv), F::Float32x2),
        "vTangent" => v(offset_of!(Vertex, tangent), F::Float32x3),
        // (The bitangent's sign is the tangent's fourth component.)
        "fHandedness" => v(offset_of!(Vertex, tangent) + 12, F::Float32),
        // The model's `colors`, which the engine hands to custom shaders.
        "vCustomColor" => v(offset_of!(Vertex, color), F::Unorm8x4),
        "vTcIn1" => v(offset_of!(Vertex, uv1), F::Float32x2),
        "vIndex" if skinned => Attr::Skin(offset_of!(SkinVertex, bones) as u64, F::Uint32x4),
        "vWeight" if skinned => Attr::Skin(offset_of!(SkinVertex, weights) as u64, F::Float32x4),
        // vColor (the engine's baked per-vertex light), vStaticLightDir,
        // vTcIn2, vTcIn3, fProjectionFrontal; the bones of a mesh without.
        _ => Attr::Zero,
    }
}

/// The format a zero input is read in, for the shader's type.
fn zero_format(ty: ValueType) -> Option<wgpu::VertexFormat> {
    use wgpu::VertexFormat as F;
    Some(match ty {
        ValueType::Float => F::Float32,
        ValueType::Int => F::Sint32,
        ValueType::Uint => F::Uint32,
        ValueType::Vector(2, Scalar::Float) => F::Float32x2,
        ValueType::Vector(3, Scalar::Float) => F::Float32x3,
        ValueType::Vector(4, Scalar::Float) => F::Float32x4,
        ValueType::Vector(2, Scalar::Int) => F::Sint32x2,
        ValueType::Vector(3, Scalar::Int) => F::Sint32x3,
        ValueType::Vector(4, Scalar::Int) => F::Sint32x4,
        ValueType::Vector(2, Scalar::Uint) => F::Uint32x2,
        ValueType::Vector(3, Scalar::Uint) => F::Uint32x3,
        ValueType::Vector(4, Scalar::Uint) => F::Uint32x4,
        _ => return None,
    })
}

/// What fills a member of a uniform block.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Key {
    // Transforms (OpenGL's clip space).
    Mv,
    Mvp,
    M,
    Normal,
    Proj,
    ProjInv,
    View,
    ViewInv,
    Vp,
    VpInv,
    /// `m_texture`: the identity (only `mat2(m_texture)` is read).
    TextureMatrix,
    Bones,
    Skinmesh,
    ScreenParams,
    PixelParams,
    ClipParams,
    BboxMin,
    BboxMax,
    // Material.
    MaterialAmbient,
    MaterialDiffuse,
    MaterialEmissive,
    /// `globalColor`: white.
    GlobalColor,
    TextureBound(u8),
    AlphaDiscard,
    EnvMapCube,
    TexEnvBound,
    TexEnvCubeBound,
    // Lights.
    NumLights,
    LightPosition,
    LightColor,
    LightAreaAmbient,
    LightAreaDiffuse,
    LightAreaDirection,
    LightMaxIntensityInv,
    LightFalloffFactor,
    // Fog.
    FogParams,
    FogColor,
    // Time.
    SceneCurrentFrame,
    WorldTimer,
    ModuleHour,
    ModuleMinutesPerHour,
    ModuleDawnHour,
    ModuleDuskHour,
    // The camera, and the player at its focus.
    CameraPosition,
    CameraFocus,
    CameraPitch,
    CameraYaw,
    CameraViewAngle,
    PlayerPosition,
    PlayerCameraDist,
    // The area (notes §B.4).
    AreaFlags,
    AreaGlobalWind,
    AreaWeatherType,
    AreaWeatherDensity,
    /// The material's MTR `parameter` of this name, else zero: the
    /// material values (`Specularity`, `Roughness`, `Metallicness`,
    /// `CustomSpecularColor`, `DisplacementOffset`…) and a custom
    /// shader's own (`fSwaySpeed`); and every name the contract doesn't
    /// know.
    Param(String),
}

/// The contract (`docs/shaders-proposal.md`, "The uniform contract"):
/// what fills a uniform of this name.
pub(crate) fn key(name: &str) -> Key {
    use Key::*;
    match name {
        "m_mv" => Mv,
        "m_mvp" => Mvp,
        "m_m" => M,
        "m_normal" => Normal,
        "m_proj" => Proj,
        "m_proj_inv" => ProjInv,
        "m_view" => View,
        "m_view_inv" => ViewInv,
        "m_vp" => Vp,
        "m_vp_inv" => VpInv,
        "m_texture" => TextureMatrix,
        "m_bones" => Bones,
        "skinmesh" => Skinmesh,
        "screenParams" => ScreenParams,
        "pixelParams" => PixelParams,
        "clipParams" => ClipParams,
        "bboxMin" => BboxMin,
        "bboxMax" => BboxMax,
        "materialFrontAmbient" => MaterialAmbient,
        "materialFrontDiffuse" => MaterialDiffuse,
        "materialFrontEmissive" => MaterialEmissive,
        "globalColor" => GlobalColor,
        "fAlphaDiscardValue" => AlphaDiscard,
        "envMapCube" => EnvMapCube,
        "texEnvBound" => TexEnvBound,
        "texEnvCubeBound" => TexEnvCubeBound,
        "numLights" => NumLights,
        "lightPosition" => LightPosition,
        "lightColor" => LightColor,
        "lightAreaAmbient" => LightAreaAmbient,
        "lightAreaDiffuse" => LightAreaDiffuse,
        "lightAreaDiffuseDirection" => LightAreaDirection,
        "lightMaxIntensityInv" => LightMaxIntensityInv,
        "lightFalloffFactor" => LightFalloffFactor,
        "fogParams" => FogParams,
        "fogColor" => FogColor,
        "sceneCurrentFrame" => SceneCurrentFrame,
        "worldtimerTimeOfDay" => WorldTimer,
        "moduleHour" => ModuleHour,
        "moduleMinutesPerHour" => ModuleMinutesPerHour,
        "moduleDawnHour" => ModuleDawnHour,
        "moduleDuskHour" => ModuleDuskHour,
        "cameraPosition" => CameraPosition,
        "cameraFocus" => CameraFocus,
        "cameraPitch" => CameraPitch,
        "cameraYaw" => CameraYaw,
        "cameraViewAngle" => CameraViewAngle,
        "playerPosition" => PlayerPosition,
        "playerCameraDist" => PlayerCameraDist,
        "areaFlags" => AreaFlags,
        "areaGlobalWind" => AreaGlobalWind,
        "areaWeatherType" => AreaWeatherType,
        "areaWeatherDensity" => AreaWeatherDensity,
        _ => match name.strip_prefix("texture").and_then(|n| n.strip_suffix("Bound")) {
            Some(n) if n.parse::<u8>().is_ok_and(|n| n <= 15) => {
                TextureBound(n.parse().unwrap_or_default())
            }
            _ => Param(name.to_string()),
        },
    }
}

/// A block's members and what fills each.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    pub(crate) size: u32,
    members: Vec<(Member, Key)>,
}

impl Plan {
    pub(crate) fn new(block: &UniformBlock) -> Plan {
        Plan {
            size: block.size,
            members: block.members.iter().map(|m| (m.clone(), key(&m.name))).collect(),
        }
    }
}

/// What a frame's draws share.
#[derive(Debug, Clone, Default)]
pub(crate) struct FrameValues {
    /// OpenGL's projection (depth from -1 to 1), as the engine's.
    pub(crate) proj: Mat4,
    pub(crate) view: Mat4,
    /// Target size, pixels.
    pub(crate) size: (f32, f32),
    pub(crate) near: f32,
    pub(crate) far: f32,
    /// Linear.
    pub(crate) area_ambient: Vec3,
    pub(crate) area_diffuse: Vec3,
    /// View space, towards the light.
    pub(crate) area_direction: Vec3,
    pub(crate) max_intensity_inv: f32,
    pub(crate) falloff: f32,
    /// (enabled, start, end, 1 / (end - start)).
    pub(crate) fog: [f32; 4],
    /// Gamma space.
    pub(crate) fog_color: [f32; 4],
    /// The view's clock in seconds (the scene's time).
    pub(crate) seconds: f32,
    pub(crate) eye: Vec3,
    pub(crate) target: Vec3,
    pub(crate) fov_y: f32,
    /// The area's flags, wind and weather ([`crate::scene::Environment`]).
    pub(crate) environment: crate::scene::Environment,
}

/// One draw's own values.
#[derive(Debug, Clone, Default)]
pub(crate) struct DrawValues<'a> {
    /// Model to world.
    pub(crate) world: Mat4,
    /// Bind pose to pose, in the skin node's space; empty: not skinned.
    pub(crate) bones: &'a [[[f32; 4]; 4]],
    /// Linear; diffuse's alpha the mesh's.
    pub(crate) ambient: [f32; 4],
    pub(crate) diffuse: [f32; 4],
    pub(crate) emissive: [f32; 4],
    /// Which texture slots have a texture.
    pub(crate) bound: [bool; 16],
    pub(crate) env_cube: bool,
    pub(crate) alpha_discard: f32,
    /// View-space positions (w 1), and colours with ±cutoff² as alpha.
    pub(crate) light_positions: &'a [[f32; 4]],
    pub(crate) light_colors: &'a [[f32; 4]],
    pub(crate) bbox: (Vec3, Vec3),
    /// The MTR's parameters.
    pub(crate) params: &'a [(String, MtrParam)],
}

/// A value to write.
#[derive(Debug, Clone, Copy)]
enum Val<'a> {
    F(f32),
    I(i32),
    V([f32; 4]),
    M(Mat4),
    Vs(&'a [[f32; 4]]),
    Ms(&'a [[[f32; 4]; 4]]),
    Zero,
}

fn v3(v: Vec3, w: f32) -> [f32; 4] {
    v.extend(w).to_array()
}

/// The camera's pitch from straight down and its yaw anticlockwise from
/// east, degrees (as `SetCameraFacing` takes them [INF]).
fn pitch_yaw(eye: Vec3, target: Vec3) -> (f32, f32) {
    let d = (target - eye).normalize_or_zero();
    let pitch = d.dot(-Vec3::Z).clamp(-1.0, 1.0).acos().to_degrees();
    let yaw = d.y.atan2(d.x).to_degrees().rem_euclid(360.0);
    (pitch, yaw)
}

fn value<'a>(key: &Key, f: &FrameValues, d: &DrawValues<'a>) -> Val<'a> {
    use Key::*;
    let mv = || f.view * d.world;
    match key {
        Mv => Val::M(mv()),
        Mvp => Val::M(f.proj * mv()),
        M => Val::M(d.world),
        Normal => Val::M(mv().inverse().transpose()),
        Proj => Val::M(f.proj),
        ProjInv => Val::M(f.proj.inverse()),
        View => Val::M(f.view),
        ViewInv => Val::M(f.view.inverse()),
        Vp => Val::M(f.proj * f.view),
        VpInv => Val::M((f.proj * f.view).inverse()),
        TextureMatrix => Val::M(Mat4::IDENTITY),
        Bones => Val::Ms(d.bones),
        Skinmesh => Val::I(i32::from(!d.bones.is_empty())),
        ScreenParams => Val::V([f.size.0, f.size.1, 0.0, 0.0]),
        PixelParams => Val::V([1.0 / f.size.0, 1.0 / f.size.1, f.size.0 / f.size.1, 0.0]),
        ClipParams => Val::V([f.near, f.far, 1.0 - f.near / f.far, 0.0]),
        BboxMin => Val::V(v3(d.bbox.0, 0.0)),
        BboxMax => Val::V(v3(d.bbox.1, 0.0)),
        MaterialAmbient => Val::V(d.ambient),
        MaterialDiffuse => Val::V(d.diffuse),
        MaterialEmissive => Val::V(d.emissive),
        GlobalColor => Val::V([1.0; 4]),
        TextureBound(n) => {
            Val::I(i32::from(d.bound.get(usize::from(*n)).copied().unwrap_or(false)))
        }
        AlphaDiscard => Val::F(d.alpha_discard),
        EnvMapCube | TexEnvCubeBound => Val::I(i32::from(d.env_cube)),
        TexEnvBound => Val::I(i32::from(!d.env_cube)),
        NumLights => Val::I(d.light_positions.len() as i32),
        LightPosition => Val::Vs(d.light_positions),
        LightColor => Val::Vs(d.light_colors),
        LightAreaAmbient => Val::V(v3(f.area_ambient, 0.0)),
        LightAreaDiffuse => Val::V(v3(f.area_diffuse, 0.0)),
        LightAreaDirection => Val::V(v3(f.area_direction, 0.0)),
        LightMaxIntensityInv => Val::F(f.max_intensity_inv),
        LightFalloffFactor => Val::F(f.falloff),
        FogParams => Val::V(f.fog),
        FogColor => Val::V(f.fog_color),
        // The client's units (measured, `docs/research/notes_shaders.md`
        // §B.4): see `time_values`.
        SceneCurrentFrame => Val::I(time_values(f.seconds).0),
        WorldTimer => Val::I(time_values(f.seconds).1),
        // The editor keeps no module clock: the game's defaults, at noon.
        ModuleHour => Val::I(12),
        ModuleMinutesPerHour => Val::I(2),
        ModuleDawnHour => Val::I(6),
        ModuleDuskHour => Val::I(18),
        CameraPosition => Val::V(v3(f.eye, 0.0)),
        CameraFocus => Val::V(v3(f.target, 0.0)),
        CameraPitch => Val::F(pitch_yaw(f.eye, f.target).0),
        CameraYaw => Val::F(pitch_yaw(f.eye, f.target).1),
        CameraViewAngle => Val::F(f.fov_y.to_degrees()),
        PlayerPosition => Val::V(v3(f.target, 0.0)),
        PlayerCameraDist => Val::F(f.eye.distance(f.target)),
        AreaFlags => Val::I(f.environment.flags as i32),
        AreaGlobalWind => Val::V(v3(f.environment.wind, 0.0)),
        AreaWeatherType => Val::I(i32::from(f.environment.weather)),
        AreaWeatherDensity => Val::F(f.environment.weather_density),
        Param(name) => match d.params.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
            Some((_, MtrParam::Int(i))) => Val::I(*i),
            Some((_, MtrParam::Float(v))) if v.len() == 1 => Val::F(v[0]),
            Some((_, MtrParam::Float(v))) => {
                Val::V(std::array::from_fn(|i| v.get(i).copied().unwrap_or(0.0)))
            }
            None => Val::Zero,
        },
    }
}

/// `sceneCurrentFrame` and `worldtimerTimeOfDay` for a clock of `seconds`.
pub(crate) fn time_values(seconds: f32) -> (i32, i32) {
    let s = f64::from(seconds.max(0.0));
    ((s * TIME_FRAMES_PER_SECOND) as i32, (s * TIME_TIMER_PER_SECOND) as i32)
}

/// `sceneCurrentFrame` counts the client's frames (54-55 a second on the
/// off-screen display, measured: `time_uniforms_in_the_client` in
/// `client_render.rs`); Moonglow draws when something changes, so its
/// clock counts 60 a second.
pub(crate) const TIME_FRAMES_PER_SECOND: f64 = 60.0;
/// `worldtimerTimeOfDay` counts real milliseconds (measured likewise;
/// whatever the module's minutes an hour). Its origin in the client
/// follows neither the module's hour nor its day and is not known: the
/// stock and TDoD shaders use it as a phase or as seconds.
pub(crate) const TIME_TIMER_PER_SECOND: f64 = 1000.0;

fn put(out: &mut [u8], at: u32, bytes: [u8; 4]) {
    let at = at as usize;
    if let Some(slot) = out.get_mut(at..at + 4) {
        slot.copy_from_slice(&bytes);
    }
}

/// A scalar of the member's kind.
fn put_scalar(out: &mut [u8], at: u32, kind: Scalar, x: f32) {
    let bytes = match kind {
        Scalar::Float => x.to_le_bytes(),
        Scalar::Int => (x.round() as i32).to_le_bytes(),
        Scalar::Uint | Scalar::Bool => (x.round().max(0.0) as u32).to_le_bytes(),
    };
    put(out, at, bytes);
}

/// One element of type `ty` (an array's, or the member) at `at`.
fn put_one(out: &mut [u8], at: u32, ty: ValueType, v: &Val<'_>, index: usize) {
    let mut scalar = |x: f32, kind| put_scalar(out, at, kind, x);
    match (ty, v) {
        (ValueType::Float, Val::F(x)) => scalar(*x, Scalar::Float),
        (ValueType::Float, Val::I(i)) => scalar(*i as f32, Scalar::Float),
        (ValueType::Int, Val::I(i)) => put(out, at, i.to_le_bytes()),
        (ValueType::Int, Val::F(x)) => scalar(*x, Scalar::Int),
        (ValueType::Uint | ValueType::Bool, Val::I(i)) => scalar(*i as f32, Scalar::Uint),
        (ValueType::Uint | ValueType::Bool, Val::F(x)) => scalar(*x, Scalar::Uint),
        (ValueType::Vector(n, kind), Val::V(c)) => {
            for (i, x) in c.iter().take(usize::from(n)).enumerate() {
                put_scalar(out, at + 4 * i as u32, kind, *x);
            }
        }
        (ValueType::Vector(n, kind), Val::Vs(list)) => {
            if let Some(c) = list.get(index) {
                for (i, x) in c.iter().take(usize::from(n)).enumerate() {
                    put_scalar(out, at + 4 * i as u32, kind, *x);
                }
            }
        }
        (ValueType::Matrix(columns, rows), Val::M(m)) => {
            put_matrix(out, at, columns, rows, &m.to_cols_array_2d())
        }
        (ValueType::Matrix(columns, rows), Val::Ms(list)) => {
            if let Some(m) = list.get(index) {
                put_matrix(out, at, columns, rows, m);
            }
        }
        _ => {}
    }
}

/// A matrix's columns at naga's stride for them (a column of two floats
/// takes 8 bytes, of three or four 16).
fn put_matrix(out: &mut [u8], at: u32, columns: u8, rows: u8, m: &[[f32; 4]; 4]) {
    let stride = if rows == 2 { 8 } else { 16 };
    for (c, column) in m.iter().take(usize::from(columns)).enumerate() {
        for (r, x) in column.iter().take(usize::from(rows)).enumerate() {
            put(out, at + (c as u32) * stride + 4 * r as u32, x.to_le_bytes());
        }
    }
}

/// Fills a block (`out`, zeroed, `plan.size` long) for a draw.
pub(crate) fn fill(plan: &Plan, f: &FrameValues, d: &DrawValues<'_>, out: &mut [u8]) {
    for (m, key) in &plan.members {
        let v = value(key, f, d);
        if matches!(v, Val::Zero) {
            continue;
        }
        match m.count {
            None => put_one(out, m.offset, m.ty, &v, 0),
            Some(n) => {
                let len = match v {
                    Val::Vs(l) => l.len(),
                    Val::Ms(l) => l.len(),
                    _ => 1,
                };
                for i in 0..(n as usize).min(len) {
                    put_one(out, m.offset + i as u32 * m.stride, m.ty, &v, i);
                }
            }
        }
    }
}

/// Which of the renderer's textures a sampler of the game's reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unit {
    /// A texture slot of the material (`texUnit0`…).
    Slot(usize),
    Env,
    EnvCube,
    Noise,
    /// None that Moonglow has (`texFBColor`, a custom shader's slot past
    /// what it keeps): white.
    White,
}

/// A sampler's unit, by the name the engine binds it to (notes §C).
pub(crate) fn unit(name: &str) -> Unit {
    match name {
        "texUnitEnv" => Unit::Env,
        "texUnitEnvCube" => Unit::EnvCube,
        "texUnitNoise" => Unit::Noise,
        _ => match name.strip_prefix("texUnit").and_then(|n| n.parse::<usize>().ok()) {
            Some(n) if n < crate::renderer::SLOTS => Unit::Slot(n),
            _ => Unit::White,
        },
    }
}

/// A program's stage, as a pipeline binds it.
#[derive(Debug, Clone)]
struct StagePlan {
    /// Its uniform block, if it has one.
    block: Option<(u32, Plan)>,
    textures: Vec<(Texture, Unit)>,
}

impl StagePlan {
    fn new(r: &Reflection) -> Result<StagePlan, String> {
        if r.uniforms.len() > 1 {
            return Err(format!("{} uniform blocks (one is drawn)", r.uniforms.len()));
        }
        let mut textures = Vec::new();
        for t in &r.textures {
            if !matches!(t.dim, naga::ImageDimension::D2 | naga::ImageDimension::Cube) {
                return Err(format!("{} is a {:?} texture", t.name, t.dim));
            }
            textures.push((t.clone(), unit(&t.name)));
        }
        Ok(StagePlan { block: r.uniforms.first().map(|b| (b.binding, Plan::new(b))), textures })
    }
}

/// Which of the renderer's passes a game draw is in, and so how it blends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GamePass {
    /// Solid (a cut-out among them): not blended.
    Opaque,
    /// A see-through mesh's nearly opaque part: blended, its depth
    /// written, the fragments below [`crate::renderer::SOLID_ALPHA`]
    /// discarded.
    Solid,
    /// The rest of it: blended over everything solid, hiding nothing.
    Fringe,
}

/// A pipeline of a program pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PipelineKey {
    pub(crate) vs: String,
    pub(crate) fs: String,
    pub(crate) skinned: bool,
    pub(crate) twosided: bool,
    pub(crate) pass: GamePass,
}

/// A program pair ready to draw.
#[derive(Debug)]
pub(crate) struct GamePipeline {
    id: u32,
    pipeline: wgpu::RenderPipeline,
    /// Group 0 the vertex program's, 1 the fragment program's.
    layouts: [wgpu::BindGroupLayout; 2],
    stages: [StagePlan; 2],
    /// The vertex buffer slot of the skin's data, and of the zeros.
    skin_slot: Option<u32>,
    zero_slot: Option<u32>,
    /// A program reads the clock (`worldtimerTimeOfDay`,
    /// `sceneCurrentFrame`): what it draws moves by itself.
    pub(crate) timed: bool,
}

/// One draw through a program pair.
#[derive(Debug)]
pub(crate) struct GameDraw<'a> {
    pub(crate) pipeline: Arc<GamePipeline>,
    /// The material's place among the renderer's.
    pub(crate) material: u32,
    pub(crate) mesh: &'a crate::model::GpuMesh,
    /// Replaced vertices, in the frame's dynamic buffer: from, to.
    pub(crate) dynamic: Option<(u64, u64)>,
    /// Each stage's block's offset in the frame's buffer.
    offsets: [Option<u32>; 2],
}

impl<'a> GameDraw<'a> {
    /// The same draw through another pipeline of the same programs (a
    /// see-through mesh's other part): the same blocks.
    pub(crate) fn with_pipeline(&self, pipeline: Arc<GamePipeline>) -> GameDraw<'a> {
        GameDraw { pipeline, offsets: self.offsets, ..*self }
    }
}

/// The textures a material binds, for the game's samplers.
#[derive(Debug)]
pub(crate) struct MaterialViews<'a> {
    pub(crate) slots: [Option<&'a wgpu::TextureView>; crate::renderer::SLOTS],
    pub(crate) env: &'a wgpu::TextureView,
    pub(crate) env_cube: &'a wgpu::TextureView,
    pub(crate) noise: &'a wgpu::TextureView,
    pub(crate) white: &'a wgpu::TextureView,
    pub(crate) white_cube: &'a wgpu::TextureView,
    pub(crate) sampler: &'a wgpu::Sampler,
    pub(crate) env_sampler: &'a wgpu::Sampler,
}

/// The game's programs, their pipelines and the frame's blocks.
#[derive(Debug)]
pub(crate) struct GameShaders {
    preamble: Preamble,
    programs: HashMap<(String, Stage), Result<Arc<Program>, String>>,
    pipelines: HashMap<PipelineKey, Option<Arc<GamePipeline>>>,
    /// What couldn't be drawn, and why: each reported once.
    pub(crate) failures: Vec<(String, String)>,
    zero: wgpu::Buffer,
    /// The frame's blocks, each at an offset aligned for binding.
    blocks: Vec<u8>,
    buffer: wgpu::Buffer,
    /// Bumped when `buffer` is made anew (the groups bind it).
    generation: u64,
    groups: HashMap<(u32, u32), [wgpu::BindGroup; 2]>,
    groups_generation: u64,
    align: u32,
    next_id: u32,
}

/// The zero buffer's size: 16 inputs of 16 bytes.
const ZERO_SIZE: u64 = 256;

/// Programs from the renderer's assets.
struct AssetSources<'a>(&'a dyn crate::Assets);

impl Sources for AssetSources<'_> {
    fn shader(&self, name: &str) -> Option<(std::borrow::Cow<'_, [u8]>, String)> {
        let text = self.0.shader(&name.to_ascii_lowercase())?;
        Some((std::borrow::Cow::Owned(text.into_bytes()), "assets".into()))
    }
}

/// Runs `f` and returns its value, or the validation error wgpu raised.
fn checked<T>(gpu: &Gpu, f: impl FnOnce() -> T) -> Result<T, String> {
    let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let value = f();
    match pollster::block_on(scope.pop()) {
        Some(e) => Err(e.to_string()),
        None => Ok(value),
    }
}

impl GameShaders {
    pub(crate) fn new(gpu: &Gpu) -> GameShaders {
        let buffer = |label, size, usage| {
            gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let zero = buffer("game zeros", ZERO_SIZE, wgpu::BufferUsages::VERTEX);
        gpu.queue.write_buffer(&zero, 0, &[0; ZERO_SIZE as usize]);
        GameShaders {
            // The game at its highest settings; no keyholing (the editor
            // has no player to cut a view to) and materials that may
            // discard (alpha-tested), as Moonglow's own shader draws them.
            preamble: Preamble { keyholing: false, no_discard: false, ..Preamble::default() },
            programs: HashMap::new(),
            pipelines: HashMap::new(),
            failures: Vec::new(),
            zero,
            blocks: Vec::new(),
            buffer: buffer("game blocks", 1 << 16, wgpu::BufferUsages::UNIFORM),
            generation: 0,
            groups: HashMap::new(),
            groups_generation: 0,
            align: gpu.device.limits().min_uniform_buffer_offset_alignment.max(16),
            next_id: 0,
        }
    }

    /// Forgets the programs (the resources changed: a hak's include).
    pub(crate) fn clear(&mut self) {
        self.programs.clear();
        self.pipelines.clear();
        self.groups.clear();
        self.failures.clear();
    }

    pub(crate) fn fail(&mut self, what: &str, why: String) {
        if !self.failures.iter().any(|(w, _)| w == what) {
            eprintln!(
                "moonglow: the game's shader {what} is drawn with Moonglow's own instead: {why}"
            );
            self.failures.push((what.to_string(), why));
        }
    }

    fn program(
        &mut self,
        assets: &dyn crate::Assets,
        name: &str,
        stage: Stage,
    ) -> Result<Arc<Program>, String> {
        let preamble = &self.preamble;
        self.programs
            .entry((name.to_string(), stage))
            .or_insert_with(|| {
                // (A material may name a program for the other stage: an
                // MTR's `customshaderVS fslit_nm`. The game would draw it
                // magenta; it is drawn with Moonglow's own shader.)
                let declared = assets
                    .shader(&name.to_ascii_lowercase())
                    .and_then(|text| mg_shader::Stage::declared(&text));
                if let Some(other) = declared.filter(|d| *d != stage) {
                    return Err(format!(
                        "it is a {} program, named for the {} stage",
                        other.id(),
                        stage.id()
                    ));
                }
                mg_shader::compile(name, stage, preamble, &AssetSources(assets))
                    .map(Arc::new)
                    .map_err(|e| e.to_string())
            })
            .clone()
    }

    /// The pipeline for a program pair, built on first use; `None` (and
    /// reported once) where a program doesn't compile or the pair can't
    /// be drawn: the draw is Moonglow's own then.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn pipeline(
        &mut self,
        gpu: &Gpu,
        assets: &dyn crate::Assets,
        key: &PipelineKey,
        color_format: wgpu::TextureFormat,
        samples: u32,
    ) -> Option<Arc<GamePipeline>> {
        if let Some(p) = self.pipelines.get(key) {
            return p.clone();
        }
        let built = self.build(gpu, assets, key, color_format, samples);
        let p = match built {
            Ok(p) => Some(Arc::new(p)),
            Err(why) => {
                self.fail(&format!("{}/{}", key.vs, key.fs), why);
                None
            }
        };
        self.pipelines.insert(key.clone(), p.clone());
        p
    }

    fn build(
        &mut self,
        gpu: &Gpu,
        assets: &dyn crate::Assets,
        key: &PipelineKey,
        color_format: wgpu::TextureFormat,
        samples: u32,
    ) -> Result<GamePipeline, String> {
        let at = |name: &str, e: String| format!("{name}: {e}");
        let mut vs =
            (*self.program(assets, &key.vs, Stage::Vertex).map_err(|e| at(&key.vs, e))?).clone();
        let mut fs =
            (*self.program(assets, &key.fs, Stage::Fragment).map_err(|e| at(&key.fs, e))?).clone();
        if key.skinned {
            vs.read_input_as_uint("vIndex").map_err(|e| at(&key.vs, e.to_string()))?;
        }
        fs.link_inputs(&vs.reflection.outputs).map_err(|e| at(&key.fs, e.to_string()))?;
        let keep = match key.pass {
            GamePass::Opaque => None,
            GamePass::Solid => Some(AlphaKeep::AtLeast(crate::renderer::SOLID_ALPHA)),
            GamePass::Fringe => Some(AlphaKeep::Below(crate::renderer::SOLID_ALPHA)),
        };
        if let Some(keep) = keep {
            fs.keep_alpha(keep).map_err(|e| at(&key.fs, e.to_string()))?;
        }
        fs.set_group(1).map_err(|e| at(&key.fs, e.to_string()))?;
        let timed = [&vs, &fs].iter().any(|p| {
            p.reflection.uniforms.iter().any(|b| {
                b.members
                    .iter()
                    .any(|m| matches!(self::key(&m.name), Key::WorldTimer | Key::SceneCurrentFrame))
            })
        });
        let stages = [
            StagePlan::new(&vs.reflection).map_err(|e| at(&key.vs, e))?,
            StagePlan::new(&fs.reflection).map_err(|e| at(&key.fs, e))?,
        ];
        let layouts = [
            layout(gpu, &stages[0], wgpu::ShaderStages::VERTEX),
            layout(gpu, &stages[1], wgpu::ShaderStages::FRAGMENT),
        ];

        // Vertex buffers: the vertex, the skin's, the zeros.
        let (mut own, mut skin, mut zero) = (Vec::new(), Vec::new(), Vec::new());
        for input in &vs.reflection.inputs {
            let location = input.location;
            match attribute(&input.name, key.skinned) {
                Attr::Vertex(offset, format) => {
                    own.push(wgpu::VertexAttribute { format, offset, shader_location: location })
                }
                Attr::Skin(offset, format) => {
                    skin.push(wgpu::VertexAttribute { format, offset, shader_location: location })
                }
                Attr::Zero => {
                    let format = zero_format(input.ty).ok_or_else(|| {
                        format!("{}: input {} of {:?}", key.vs, input.name, input.ty)
                    })?;
                    let offset = 16 * zero.len() as u64;
                    if offset + 16 > ZERO_SIZE {
                        return Err(format!("{}: too many inputs", key.vs));
                    }
                    zero.push(wgpu::VertexAttribute { format, offset, shader_location: location });
                }
            }
        }
        let mut buffers = vec![Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &own,
        })];
        let skin_slot = key.skinned.then(|| {
            buffers.push(Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SkinVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &skin,
            }));
            buffers.len() as u32 - 1
        });
        let zero_slot = (!zero.is_empty()).then(|| {
            buffers.push(Some(wgpu::VertexBufferLayout {
                array_stride: ZERO_SIZE,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &zero,
            }));
            buffers.len() as u32 - 1
        });

        let device = &gpu.device;
        let modules = checked(gpu, || {
            let make = |label: &str, p: &Program| {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(p.module.clone())),
                })
            };
            (make(&key.vs, &vs), make(&key.fs, &fs))
        })?;
        let entry = |p: &Program| p.module.entry_points.first().map(|e| e.name.clone());
        let (vs_entry, fs_entry) = (entry(&vs), entry(&fs));
        let pipeline = checked(gpu, || {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("game"),
                bind_group_layouts: &[Some(&layouts[0]), Some(&layouts[1])],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("{}/{}", key.vs, key.fs)),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &modules.0,
                    entry_point: vs_entry.as_deref(),
                    compilation_options: Default::default(),
                    buffers: &buffers,
                },
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: if key.twosided { None } else { Some(wgpu::Face::Back) },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: crate::DEPTH_FORMAT,
                    depth_write_enabled: Some(key.pass != GamePass::Fringe),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
                fragment: Some(wgpu::FragmentState {
                    module: &modules.1,
                    entry_point: fs_entry.as_deref(),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: (key.pass != GamePass::Opaque)
                            .then_some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })?;
        self.next_id += 1;
        Ok(GamePipeline {
            id: self.next_id,
            pipeline,
            layouts,
            stages,
            skin_slot,
            zero_slot,
            timed,
        })
    }

    /// Starts a frame's blocks.
    pub(crate) fn begin(&mut self) {
        self.blocks.clear();
    }

    /// Writes a draw's blocks; the draw to make of it.
    pub(crate) fn push<'a>(
        &mut self,
        pipeline: Arc<GamePipeline>,
        material: u32,
        mesh: &'a crate::model::GpuMesh,
        dynamic: Option<(u64, u64)>,
        f: &FrameValues,
        d: &DrawValues<'_>,
    ) -> GameDraw<'a> {
        let mut offsets = [None; 2];
        for (stage, offset) in pipeline.stages.iter().zip(&mut offsets) {
            let Some((_, plan)) = &stage.block else { continue };
            let align = self.align as usize;
            let at = self.blocks.len().div_ceil(align) * align;
            self.blocks.resize(at + plan.size as usize, 0);
            fill(plan, f, d, &mut self.blocks[at..]);
            *offset = Some(at as u32);
        }
        GameDraw { pipeline, material, mesh, dynamic, offsets }
    }

    /// Uploads the frame's blocks.
    pub(crate) fn upload(&mut self, gpu: &Gpu) {
        let bytes = self.blocks.len() as u64;
        if self.buffer.size() < bytes {
            self.buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("game blocks"),
                size: bytes.next_power_of_two(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.generation += 1;
        }
        if !self.blocks.is_empty() {
            gpu.queue.write_buffer(&self.buffer, 0, &self.blocks);
        }
    }

    /// Makes the bind groups a draw needs, where they aren't made yet.
    pub(crate) fn prepare(&mut self, gpu: &Gpu, draw: &GameDraw<'_>, views: &MaterialViews<'_>) {
        if self.groups_generation != self.generation {
            self.groups.clear();
            self.groups_generation = self.generation;
        }
        let key = (draw.pipeline.id, draw.material);
        if self.groups.contains_key(&key) {
            return;
        }
        let p = &draw.pipeline;
        let group = |i: usize| {
            let stage = &p.stages[i];
            let mut entries = Vec::new();
            if let Some((binding, plan)) = &stage.block {
                entries.push(wgpu::BindGroupEntry {
                    binding: *binding,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(u64::from(plan.size)),
                    }),
                });
            }
            for (t, unit) in &stage.textures {
                let cube = t.dim == naga::ImageDimension::Cube;
                let (view, sampler) = match unit {
                    _ if cube => (
                        if *unit == Unit::EnvCube { views.env_cube } else { views.white_cube },
                        views.env_sampler,
                    ),
                    Unit::Slot(n) => (views.slots[*n].unwrap_or(views.white), views.sampler),
                    Unit::Env => (views.env, views.env_sampler),
                    Unit::Noise => (views.noise, views.sampler),
                    Unit::EnvCube | Unit::White => (views.white, views.sampler),
                };
                entries.push(wgpu::BindGroupEntry {
                    binding: t.binding,
                    resource: wgpu::BindingResource::TextureView(view),
                });
                if let Some(s) = t.sampler {
                    entries.push(wgpu::BindGroupEntry {
                        binding: s,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    });
                }
            }
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("game"),
                layout: &p.layouts[i],
                entries: &entries,
            })
        };
        let groups = [group(0), group(1)];
        self.groups.insert(key, groups);
    }

    /// Draws `draws` into `pass` (their groups prepared).
    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        draws: &[GameDraw<'_>],
        dynamic: &wgpu::Buffer,
    ) {
        let mut pipeline = None;
        for d in draws {
            let Some(groups) = self.groups.get(&(d.pipeline.id, d.material)) else { continue };
            if pipeline != Some(d.pipeline.id) {
                pass.set_pipeline(&d.pipeline.pipeline);
                pipeline = Some(d.pipeline.id);
            }
            for (i, group) in groups.iter().enumerate() {
                let offsets: Vec<u32> = d.offsets[i].into_iter().collect();
                pass.set_bind_group(i as u32, group, &offsets);
            }
            match d.dynamic {
                Some((from, to)) => pass.set_vertex_buffer(0, dynamic.slice(from..to)),
                None => pass.set_vertex_buffer(0, d.mesh.vertices.slice(..)),
            }
            if let (Some(slot), Some(skin)) = (d.pipeline.skin_slot, &d.mesh.skin) {
                pass.set_vertex_buffer(slot, skin.vertices.slice(..));
            }
            if let Some(slot) = d.pipeline.zero_slot {
                pass.set_vertex_buffer(slot, self.zero.slice(..));
            }
            pass.set_index_buffer(d.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..d.mesh.index_count, 0, 0..1);
        }
    }
}

/// A stage's bind group layout: its block at a dynamic offset, then its
/// textures and samplers.
fn layout(gpu: &Gpu, stage: &StagePlan, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayout {
    let mut entries = Vec::new();
    if let Some((binding, plan)) = &stage.block {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: *binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(u64::from(plan.size)),
            },
            count: None,
        });
    }
    for (t, _) in &stage.textures {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: t.binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: if t.dim == naga::ImageDimension::Cube {
                    wgpu::TextureViewDimension::Cube
                } else {
                    wgpu::TextureViewDimension::D2
                },
                multisampled: false,
            },
            count: None,
        });
        if let Some(s) = t.sampler {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: s,
                visibility,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }
    }
    gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("game"),
        entries: &entries,
    })
}

/// The stock lit pair the engine draws a mesh with (notes §A.3): `_nm`
/// with normal and other maps, `_sm` reflecting an environment map.
pub(crate) fn lit_pair(normal_mapped: bool, reflects: bool) -> (&'static str, &'static str) {
    match (reflects, normal_mapped) {
        (false, false) => ("vslit", "fslit"),
        (false, true) => ("vslit_nm", "fslit_nm"),
        (true, false) => ("vslit_sm", "fslit_sm"),
        (true, true) => ("vslit_sm_nm", "fslit_sm_nm"),
    }
}

/// The program pair a material names (notes §A.4): its `customshaderVS`
/// and `customshaderFS`, where it names both. One named alone is no pair:
/// the engine draws the mesh with the stock pair it would choose then (the
/// wiki has both needed; and of The Domains of Dread's materials, 250 name
/// `fslit_nm`, a fragment program, alone as `customshaderVS`, which the
/// game would draw magenta if it took one alone, and doesn't).
pub(crate) fn custom_pair(vs: Option<&str>, fs: Option<&str>) -> Option<(String, String)> {
    let name = |n: Option<&str>| {
        n.map(|n| n.trim().to_ascii_lowercase()).filter(|n| !n.is_empty() && n != "null")
    };
    Some((name(vs)?, name(fs)?))
}

/// The pair a mesh is drawn with: its material's ([`custom_pair`]), else
/// the stock lit pair ([`lit_pair`]); and whether it is the material's.
pub(crate) fn program_pair(
    custom: Option<&(String, String)>,
    normal_mapped: bool,
    reflects: bool,
) -> (String, String, bool) {
    match custom {
        Some((vs, fs)) => (vs.clone(), fs.clone(), true),
        None => {
            let (vs, fs) = lit_pair(normal_mapped, reflects);
            (vs.into(), fs.into(), false)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    #[test]
    fn a_material_names_a_pair_only_with_both_programs() {
        let pair = |vs, fs| custom_pair(vs, fs);
        assert_eq!(
            pair(Some("shd_windgrass_vs"), Some(" SHD_WindGrass_FS ")),
            Some(("shd_windgrass_vs".into(), "shd_windgrass_fs".into()))
        );
        // One alone (The Domains of Dread's `customshadervs fslit_nm`), or
        // an empty name: the stock pair.
        assert_eq!(pair(Some("fslit_nm"), None), None);
        assert_eq!(pair(None, Some("fs_plt_hair")), None);
        assert_eq!(pair(Some(""), Some("fslit")), None);
        assert_eq!(pair(Some("null"), Some("fslit")), None);
        let custom = ("vs_a".to_string(), "fs_a".to_string());
        assert_eq!(program_pair(Some(&custom), false, true), ("vs_a".into(), "fs_a".into(), true));
        assert_eq!(program_pair(None, true, false), ("vslit_nm".into(), "fslit_nm".into(), false));
    }

    use super::*;

    #[test]
    fn vertex_inputs_are_moonglow_s_fields_by_the_engine_s_names() {
        use wgpu::VertexFormat as F;
        assert_eq!(attribute("vPos", false), Attr::Vertex(0, F::Float32x3));
        assert_eq!(attribute("vNormal", false), Attr::Vertex(12, F::Float32x3));
        assert_eq!(attribute("vTcIn", false), Attr::Vertex(24, F::Float32x2));
        assert_eq!(attribute("vTangent", false), Attr::Vertex(32, F::Float32x3));
        assert_eq!(attribute("fHandedness", false), Attr::Vertex(44, F::Float32));
        assert_eq!(attribute("vCustomColor", false), Attr::Vertex(48, F::Unorm8x4));
        assert_eq!(attribute("vTcIn1", false), Attr::Vertex(52, F::Float32x2));
        assert_eq!(attribute("vIndex", true), Attr::Skin(0, F::Uint32x4));
        assert_eq!(attribute("vWeight", true), Attr::Skin(16, F::Float32x4));
        // A mesh without a skin, the engine's baked light, a third UV set.
        for name in ["vIndex", "vWeight", "vColor", "vStaticLightDir", "vTcIn2"] {
            assert_eq!(attribute(name, false), Attr::Zero, "{name}");
        }
        assert_eq!(zero_format(ValueType::Vector(4, Scalar::Float)), Some(F::Float32x4));
        assert_eq!(zero_format(ValueType::Matrix(4, 4)), None);
    }

    #[test]
    fn uniforms_and_samplers_are_known_by_name() {
        assert_eq!(key("m_mv"), Key::Mv);
        assert_eq!(key("lightColor"), Key::LightColor);
        assert_eq!(key("texture3Bound"), Key::TextureBound(3));
        assert_eq!(key("texture16Bound"), Key::Param("texture16Bound".into()));
        assert_eq!(key("fSwaySpeed"), Key::Param("fSwaySpeed".into()));
        assert_eq!(unit("texUnit0"), Unit::Slot(0));
        assert_eq!(unit("texUnit5"), Unit::Slot(5));
        assert_eq!(unit("texUnit9"), Unit::White);
        assert_eq!(unit("texUnitEnv"), Unit::Env);
        assert_eq!(unit("texUnitEnvCube"), Unit::EnvCube);
        assert_eq!(unit("texUnitNoise"), Unit::Noise);
        assert_eq!(unit("texFBColor"), Unit::White);
        assert_eq!(lit_pair(false, false), ("vslit", "fslit"));
        assert_eq!(lit_pair(true, true), ("vslit_sm_nm", "fslit_sm_nm"));
    }

    /// An authored program with uniforms of every kind the contract
    /// fills: matrices, arrays of vectors, ints, a vec3, an MTR's
    /// parameters (a float and a vec4), a name nothing knows.
    fn block() -> UniformBlock {
        let fs = r#"
#define SHADER_TYPE 2
uniform mat4 m_mv;
uniform mat4 m_proj;
uniform mat4 m_texture;
uniform vec4 lightPosition[4];
uniform vec4 lightColor[4];
uniform int numLights;
uniform vec3 lightAreaAmbient;
uniform int texture0Bound;
uniform int texture1Bound;
uniform float fAlphaDiscardValue;
uniform vec4 fogParams;
uniform float fSwaySpeed;
uniform vec4 vOverrideColor;
uniform float fNobodyKnows;
uniform int worldtimerTimeOfDay;
uniform int areaFlags;
uniform vec3 areaGlobalWind;
uniform float fSwayStrength;
uniform int iRoofInterior;
void main ()
{
    vec4 c = m_mv * m_proj * m_texture * lightPosition[numLights] + lightColor[1];
    c.rgb += lightAreaAmbient * float(texture0Bound + texture1Bound + worldtimerTimeOfDay);
    c += fogParams * fAlphaDiscardValue * fSwaySpeed * fNobodyKnows + vOverrideColor;
    c.xyz += areaGlobalWind * float(areaFlags + iRoofInterior) * fSwayStrength;
    gl_FragColor = c;
}
"#;
        let src: BTreeMap<String, String> = [("fs_block".to_string(), fs.to_string())].into();
        let p =
            mg_shader::compile("fs_block", Stage::Fragment, &Preamble::default(), &src).unwrap();
        p.reflection.uniforms[0].clone()
    }

    fn f32_at(b: &[u8], at: u32) -> f32 {
        f32::from_le_bytes(b[at as usize..at as usize + 4].try_into().unwrap())
    }

    fn i32_at(b: &[u8], at: u32) -> i32 {
        i32::from_le_bytes(b[at as usize..at as usize + 4].try_into().unwrap())
    }

    #[test]
    fn a_block_is_filled_by_name() {
        let block = block();
        let plan = Plan::new(&block);
        let view = Mat4::from_translation(Vec3::new(0.0, 0.0, -5.0));
        let f = FrameValues {
            proj: Mat4::perspective_rh_gl(0.8, 1.5, 0.1, 100.0),
            view,
            size: (300.0, 200.0),
            near: 0.1,
            far: 100.0,
            area_ambient: Vec3::new(0.1, 0.2, 0.3),
            fog: [1.0, 10.0, 40.0, 1.0 / 30.0],
            seconds: 2.5,
            environment: crate::scene::Environment::from_are(5, 2),
            ..Default::default()
        };
        let positions = [[1.0, 2.0, 3.0, 1.0], [4.0, 5.0, 6.0, 1.0]];
        let colors = [[0.5, 0.25, 0.125, 400.0], [1.0, 1.0, 1.0, -9.0]];
        let params = vec![
            ("fswayspeed".to_string(), MtrParam::Float(vec![0.75])),
            ("vOverrideColor".to_string(), MtrParam::Float(vec![0.1, 0.2, 0.3, 0.4])),
            ("FSWAYSTRENGTH".to_string(), MtrParam::Float(vec![3.0])),
            ("iRoofInterior".to_string(), MtrParam::Int(1)),
        ];
        let mut bound = [false; 16];
        bound[0] = true;
        let world = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let d = DrawValues {
            world,
            bound,
            alpha_discard: 0.2,
            light_positions: &positions,
            light_colors: &colors,
            params: &params,
            ..Default::default()
        };
        let mut out = vec![0u8; plan.size as usize];
        fill(&plan, &f, &d, &mut out);
        let m = |name: &str| block.member(name).unwrap_or_else(|| panic!("{name}"));
        // Matrices, column by column.
        let mv = (view * world).to_cols_array();
        let at = m("m_mv").offset;
        let got: Vec<f32> = (0..16).map(|i| f32_at(&out, at + 4 * i)).collect();
        assert_eq!(got, mv);
        // The identity, not zeros (only mat2(m_texture) is read).
        let at = m("m_texture").offset;
        assert_eq!(
            (f32_at(&out, at), f32_at(&out, at + 20), f32_at(&out, at + 4)),
            (1.0, 1.0, 0.0)
        );
        assert_eq!(f32_at(&out, m("m_proj").offset + 56), f.proj.w_axis.z);
        // Arrays at their stride; past the draw's lights, zeros.
        let (lp, lc) = (m("lightPosition"), m("lightColor"));
        assert_eq!(f32_at(&out, lp.offset + lp.stride + 4), 5.0);
        assert_eq!(f32_at(&out, lc.offset + 12), 400.0);
        assert_eq!(f32_at(&out, lc.offset + lc.stride + 12), -9.0);
        assert_eq!(f32_at(&out, lp.offset + 2 * lp.stride), 0.0);
        // Ints, a vec3, floats.
        assert_eq!(i32_at(&out, m("numLights").offset), 2);
        assert_eq!(i32_at(&out, m("texture0Bound").offset), 1);
        assert_eq!(i32_at(&out, m("texture1Bound").offset), 0);
        assert_eq!(f32_at(&out, m("lightAreaAmbient").offset + 8), 0.3);
        assert_eq!(f32_at(&out, m("fAlphaDiscardValue").offset), 0.2);
        assert_eq!(f32_at(&out, m("fogParams").offset + 8), 40.0);
        assert_eq!(i32_at(&out, m("worldtimerTimeOfDay").offset), time_values(2.5).1);
        // MTR parameters by name (any case), a vector one too; a name
        // nobody knows is zero.
        assert_eq!(f32_at(&out, m("fSwaySpeed").offset), 0.75);
        assert_eq!(f32_at(&out, m("vOverrideColor").offset + 12), 0.4);
        assert_eq!(f32_at(&out, m("fNobodyKnows").offset), 0.0);
        assert_eq!(f32_at(&out, m("fSwayStrength").offset), 3.0);
        assert_eq!(i32_at(&out, m("iRoofInterior").offset), 1);
        // The area's flags and wind (a strong one).
        assert_eq!(i32_at(&out, m("areaFlags").offset), 5);
        let wind = m("areaGlobalWind").offset;
        assert!((f32_at(&out, wind) - std::f32::consts::SQRT_2).abs() < 1e-5);
        assert_eq!(f32_at(&out, wind + 8), 0.0);
    }
}
