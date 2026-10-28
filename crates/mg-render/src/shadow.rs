//! The sun's and moon's shadows (`docs/research/notes_shaders.md` §G).
//!
//! The game casts them as stencil volumes extruded from the meshes flagged
//! `shadow` away from the area light, and lays a black plane over what the
//! volumes mark, as opaque as the area's Shadow Opacity, less towards the
//! top of the screen (`fs_shadowplane`'s fade) and in fog. What it marks
//! is what is drawn before the plane: the area's tiles and its static
//! placeables (read from the client's pictures: `shadows_look` in
//! `client_render.rs`), not dynamic placeables, doors or creatures.
//!
//! Moonglow stands a shadow map in for the volumes: the casting meshes'
//! depth seen from the area light, in a box fitted to what the camera sees
//! of the receiving meshes ([`plan`]). The renderer draws the receiving
//! meshes first, then a full-screen pass ([`ShadowMaps::darken`]) finds
//! each pixel's place in the scene from its depth, looks it up in the map,
//! and lays black over it as the game's plane does (one sample at a time
//! where the picture is multisampled), then the rest of the scene. So the
//! game's own shaders and Moonglow's are darkened alike, after either has
//! lit the pixel.

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::model::GpuMesh;
use crate::scene::{Camera, Instance, Scene, SunShadows};

/// The shadow map's side, texels: over an area seen whole (eight tiles
/// across) a texel is 4 cm, finer than the game's volumes are made.
pub(crate) const MAP_SIZE: u32 = 2048;

/// The shadow map's format.
const MAP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The farthest the game's camera stands from what it looks at
/// (`SetCameraFacing`'s distance, 1 to 25 m): its height over its focus
/// is what fades the shadows up the screen, and an editor's camera far
/// off is taken as the game's at its farthest at the same pitch.
pub(crate) const GAME_CAMERA_REACH: f32 = 25.0;

/// How far past the camera's far plane nothing is taken to receive
/// shadows, metres: the box of the map ends there (a camera looking at
/// the horizon).
const FARTHEST: f32 = 300.0;

/// The depth, metres, by which a surface lies nearer the light than the
/// map says before it is in its own shadow (with the map's slope bias):
/// a floor that also casts would be dotted with its own shadow
/// ("acne") without it; more, and a shadow comes loose from its caster.
const BIAS: f32 = 0.03;

/// Where, as a fraction of the screen's height from its bottom, the
/// game's shadow plane fades out to nothing (`fs_shadowplane`'s
/// `fShadowFadePoint`, worked out in `vs_shadowplane`), for a camera
/// `pitch` degrees from straight down whose eye is `height` metres over
/// its focus, with the fog ending (or the far clip) at `limit` metres.
pub(crate) fn fade_point(pitch: f32, height: f32, limit: f32) -> f32 {
    let limit = limit.max(1.0);
    let max_y = limit * (89.99 - pitch).to_radians().tan() - height * pitch.to_radians().sin();
    (max_y / limit + 0.5).max(0.01)
}

/// The game's shadow plane's opacity at `up` (a fraction of the screen's
/// height from its bottom) for a fade point `point` ([`fade_point`]):
/// whole at the bottom, less higher up, as `fs_shadowplane` has it. The
/// plane is no more opaque than this nor than the area's opacity. (The
/// darkening pass's shader works it out per pixel.)
#[cfg(test)]
pub(crate) fn fade(point: f32, up: f32) -> f32 {
    (1.1 * (point - up) / point - 0.1).clamp(0.0, 1.0)
}

/// The fade point for `camera` ([`fade_point`]): its pitch, and its
/// height over its focus taken no greater than the game's camera at its
/// farthest has it at that pitch ([`GAME_CAMERA_REACH`]).
pub(crate) fn camera_fade_point(camera: &Camera, limit: f32) -> f32 {
    let along = camera.target - camera.eye;
    let distance = along.length();
    let d = along.normalize_or(-Vec3::Z);
    let pitch = d.dot(-Vec3::Z).clamp(-1.0, 1.0).acos();
    let height = distance.min(GAME_CAMERA_REACH) * pitch.cos();
    fade_point(pitch.to_degrees(), height, limit)
}

/// Whether a mesh of an instance casts a shadow: flagged `shadow`, solid
/// (as the game's volumes are of solid models: a see-through mesh casts
/// none), of an instance that casts at all (not a marker, a see-through
/// ghost or grass); `alpha` is its alpha as it is drawn now. A skinned
/// mesh casts as its bones pose it ([`Caster::bones`]).
pub(crate) fn casts(inst: &Instance, mesh: &GpuMesh, alpha: f32) -> bool {
    inst.casts_shadows
        && !inst.unlit
        && !inst.grass
        && inst.opacity >= 1.0
        && mesh.casts
        && alpha >= 1.0
        && mesh.material.transparency_hint == 0
}

/// A mesh drawn into a shadow map: where it is, and a skinned one's
/// bones.
#[derive(Clone, Copy)]
pub(crate) struct Caster<'a> {
    pub(crate) mesh: &'a GpuMesh,
    /// From the mesh's node to the world.
    pub(crate) world: Mat4,
    /// A skinned mesh's first bone among the frame's casting bones
    /// ([`Plan::bones`]).
    pub(crate) bones: Option<u32>,
}

impl Caster<'_> {
    /// What the map holds of it: its mesh, its place, its bones' first.
    fn key(&self) -> (usize, Mat4, Option<u32>) {
        (std::ptr::from_ref(self.mesh) as usize, self.world, self.bones)
    }
}

/// A skinned mesh's bones as `pose` has them, each from the bind pose to
/// the pose, in the space of the skin's node (as the renderer's draws take
/// them), pushed onto `bones`: their first.
fn skin_bones(mesh: &GpuMesh, pose: &[Mat4], bones: &mut Vec<Mat4>) -> Option<u32> {
    let skin = mesh.skin.as_ref()?;
    let first = bones.len() as u32;
    let to_skin = pose.get(mesh.node).copied().unwrap_or(Mat4::IDENTITY).inverse();
    for (&b, inverse_bind) in skin.bones.iter().zip(&skin.inverse_bind) {
        let now = pose.get(b).copied().unwrap_or(Mat4::IDENTITY);
        bones.push(to_skin * now * *inverse_bind);
    }
    Some(first)
}

/// A caster's box: a skinned mesh's grown by half its size each way (its
/// bones move it out of the box it was bound in).
fn posed_box((min, max): (Vec3, Vec3), skinned: Option<u32>) -> (Vec3, Vec3) {
    match skinned {
        Some(_) => {
            let grow = (max - min).max_element() * 0.5;
            (min - Vec3::splat(grow), max + Vec3::splat(grow))
        }
        None => (min, max),
    }
}

/// Casters in the order they are drawn: rigid then skinned (a pipeline
/// each), a mesh's together, those of a chunk in a row.
pub(crate) fn in_drawing_order(casters: &mut [Caster<'_>]) {
    casters
        .sort_by_key(|c| (c.bones.is_some(), c.mesh.chunk(), std::ptr::from_ref(c.mesh) as usize));
}

/// A mesh's box as it lies in the scene, transformed by `world`.
fn world_box(world: Mat4, mesh: &GpuMesh) -> (Vec3, Vec3) {
    let centre = world.transform_point3((mesh.min + mesh.max) * 0.5);
    let half = (mesh.max - mesh.min) * 0.5;
    let m = glam::Mat3::from_mat4(world);
    let around = m.x_axis.abs() * half.x + m.y_axis.abs() * half.y + m.z_axis.abs() * half.z;
    (centre - around, centre + around)
}

/// The eight corners of a box.
fn corners((min, max): (Vec3, Vec3)) -> [Vec3; 8] {
    std::array::from_fn(|i| {
        Vec3::new(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        )
    })
}

/// A view from the area light: looking along it (towards where it
/// shines), Z up where the light is not straight above.
pub(crate) fn light_view(direction: Vec3) -> Mat4 {
    let towards = direction.normalize_or(Vec3::Z);
    let up = if towards.cross(Vec3::Z).length_squared() < 1e-6 { Vec3::Y } else { Vec3::Z };
    Mat4::look_at_rh(Vec3::ZERO, -towards, up)
}

/// What the camera can see of the scene's receiving boxes: where the rays
/// through the corners of its picture (of `aspect`) cross the heights
/// between which the receivers lie (or end, [`FARTHEST`] past its far
/// plane at most), boxed and cut to the receivers' box.
fn seen(camera: &Camera, aspect: f32, receivers: (Vec3, Vec3)) -> Option<(Vec3, Vec3)> {
    let inverse = (camera.projection(aspect) * camera.view()).inverse();
    let far = camera.far.max(camera.near) + FARTHEST;
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (0.0, 0.0)] {
        let near = inverse.project_point3(Vec3::new(x, y, 0.0));
        let further = inverse.project_point3(Vec3::new(x, y, 0.5));
        let ray = (further - near).normalize_or(Vec3::NEG_Z);
        let mut ends = vec![0.0f32];
        for z in [receivers.0.z, receivers.1.z] {
            if ray.z.abs() > 1e-6 {
                let t = (z - near.z) / ray.z;
                if t > 0.0 {
                    ends.push(t.min(far));
                }
            }
        }
        // (Between the heights: as far as the ray goes, at most.)
        if (receivers.0.z..=receivers.1.z).contains(&near.z) {
            ends.push(far);
        }
        if ends.len() == 1 {
            ends.push(far);
        }
        for t in ends {
            let p = near + ray * t;
            min = min.min(p);
            max = max.max(p);
        }
    }
    let (min, max) = (min.max(receivers.0), max.min(receivers.1));
    (min.cmple(max).all()).then_some((min, max))
}

/// What the shadow map of a frame is made of: the light's projection and
/// the casting meshes (with their world transforms) in the order they
/// are drawn into it, those of a mesh in a row.
pub(crate) struct Plan<'a> {
    /// From the world to the map (its texture coordinates from -1 to 1,
    /// its depth from 0 at the light to 1).
    pub(crate) light: Mat4,
    pub(crate) casters: Vec<Caster<'a>>,
    /// The skinned casters' bones.
    pub(crate) bones: Vec<Mat4>,
    /// The map's depth range, metres.
    pub(crate) depth: f32,
    pub(crate) shadows: SunShadows,
}

/// The shadow map a scene needs, seen by `camera` in a picture of
/// `aspect`: `None` where it has no shadows (none asked for, none to cast
/// or none to receive one in sight).
pub(crate) fn plan<'a>(scene: &'a Scene, camera: &Camera, aspect: f32) -> Option<Plan<'a>> {
    let shadows = scene.shadows.filter(|s| s.opacity > 0.0)?;
    let mut casters: Vec<(Caster<'a>, (Vec3, Vec3))> = Vec::new();
    let mut bones = Vec::new();
    let (mut rmin, mut rmax) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for inst in &scene.instances {
        let pose: &[Mat4] = inst.pose.as_deref().unwrap_or(&inst.model.rest);
        let node = |n: usize| inst.transform * pose.get(n).copied().unwrap_or(Mat4::IDENTITY);
        for (j, mesh) in inst.model.meshes.iter().enumerate() {
            let replaced = inst.state.as_ref().and_then(|s| s.meshes.get(j));
            let alpha = replaced.and_then(|r| r.alpha).unwrap_or(mesh.material.alpha);
            if alpha <= 0.0 {
                continue;
            }
            let world = node(mesh.node);
            let bounds = world_box(world, mesh);
            if inst.receives_shadows && inst.opacity >= 1.0 {
                (rmin, rmax) = (rmin.min(bounds.0), rmax.max(bounds.1));
            }
            if casts(inst, mesh, alpha) {
                let skinned = skin_bones(mesh, pose, &mut bones);
                casters.push((Caster { mesh, world, bones: skinned }, posed_box(bounds, skinned)));
            }
        }
        for mesh in &inst.model.shadow_only {
            if casts(inst, mesh, 1.0) {
                let world = node(mesh.node);
                let skinned = skin_bones(mesh, pose, &mut bones);
                let bounds = posed_box(world_box(world, mesh), skinned);
                casters.push((Caster { mesh, world, bones: skinned }, bounds));
            }
        }
    }
    if casters.is_empty() || rmin.cmpgt(rmax).any() {
        return None;
    }
    let region = seen(camera, aspect, (rmin, rmax))?;
    let view = light_view(scene.area.direction);
    let to_light = |(min, max): (Vec3, Vec3)| {
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for c in corners((min, max)) {
            let p = view.transform_point3(c);
            (lo, hi) = (lo.min(p), hi.max(p));
        }
        (lo, hi)
    };
    let (lo, mut hi) = to_light(region);
    // The casters whose shadows can fall on what is seen: across the
    // light, over the region; the box reaching back to the farthest
    // towards the light.
    let mut kept = Vec::with_capacity(casters.len());
    for (caster, bounds) in casters {
        let (clo, chi) = to_light(bounds);
        if chi.x < lo.x || clo.x > hi.x || chi.y < lo.y || clo.y > hi.y || chi.z < lo.z {
            continue;
        }
        hi.z = hi.z.max(chi.z);
        kept.push(caster);
    }
    if kept.is_empty() {
        return None;
    }
    // (Square texels, the box's corner on a texel's: the map stays still
    // under a camera turning or moving across it by less than its size.)
    let side = (hi.x - lo.x).max(hi.y - lo.y).max(1e-3);
    let texel = side / MAP_SIZE as f32;
    let (left, bottom) = ((lo.x / texel).floor() * texel, (lo.y / texel).floor() * texel);
    let side = side + 2.0 * texel;
    // (Looking along -Z: the near plane is the highest Z, towards the
    // light, a little further back.)
    let (near, far) = (-hi.z - 1.0, -lo.z + 1.0);
    let projection = Mat4::orthographic_rh(left, left + side, bottom, bottom + side, near, far);
    in_drawing_order(&mut kept);
    Some(Plan { light: projection * view, casters: kept, bones, depth: far - near, shadows })
}

/// The frame's values the darkening pass reads (`DARKEN` below).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DarkenUniform {
    /// From the screen (x, y from -1 to 1, the depth buffer's depth) to
    /// the world.
    unproject: [[f32; 4]; 4],
    light: [[f32; 4]; 4],
    /// The view, for a pixel's fog (its depth along the camera).
    view: [[f32; 4]; 4],
    /// The picture's width and height, the fade point, the opacity.
    screen: [f32; 4],
    /// 1 with fog, its start and end and 1 over its length; else zeros.
    fog: [f32; 4],
    /// The bias in the map's depth, and a texel's size in its coordinates.
    bias: [f32; 4],
}

/// The casting pass's shader: positions alone (and a skinned mesh's bones
/// and weights), each instance its world transform and its bones' first.
const CAST: &str = "
struct Light { m: mat4x4<f32> }
struct Caster { world: mat4x4<f32>, bones: vec4<u32> }
@group(0) @binding(0) var<uniform> light: Light;
@group(0) @binding(1) var<storage, read> casters: array<Caster>;
@group(0) @binding(2) var<storage, read> bones: array<mat4x4<f32>>;
@vertex fn vs(@location(0) pos: vec3<f32>, @builtin(instance_index) i: u32) -> @builtin(position) vec4<f32> {
    return light.m * casters[i].world * vec4<f32>(pos, 1.0);
}
// (As the renderer's skinned draws: up to four bones, by their weights.)
@vertex fn vs_skinned(@location(0) pos: vec3<f32>, @location(1) b: vec4<u32>, @location(2) w: vec4<f32>, @builtin(instance_index) i: u32) -> @builtin(position) vec4<f32> {
    var p = vec3<f32>(0.0);
    var total = 0.0;
    for (var k = 0u; k < 4u; k = k + 1u) {
        if (w[k] > 0.0) {
            p = p + w[k] * (bones[casters[i].bones.x + b[k]] * vec4<f32>(pos, 1.0)).xyz;
            total = total + w[k];
        }
    }
    if (total > 0.0) { p = p / total; } else { p = pos; }
    return light.m * casters[i].world * vec4<f32>(p, 1.0);
}
";

/// A caster as the casting shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CasterData {
    world: [[f32; 4]; 4],
    bones: [u32; 4],
}

/// The darkening pass's shader for `samples` samples a pixel.
fn darken_source(samples: u32) -> String {
    let (depth, sample, load) = if samples > 1 {
        (
            "texture_depth_multisampled_2d",
            ", @builtin(sample_index) s: u32",
            "textureLoad(depth, at, i32(s))",
        )
    } else {
        ("texture_depth_2d", "", "textureLoad(depth, at, 0)")
    };
    format!(
        "struct Params {{
    unproject: mat4x4<f32>,
    light: mat4x4<f32>,
    view: mat4x4<f32>,
    screen: vec4<f32>,
    fog: vec4<f32>,
    bias: vec4<f32>,
}}
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var depth: {depth};
@group(0) @binding(2) var map: texture_depth_2d;
@group(0) @binding(3) var cmp: sampler_comparison;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}}
@fragment fn fs(@builtin(position) frag: vec4<f32>{sample}) -> @location(0) vec4<f32> {{
    let at = vec2<i32>(frag.xy);
    let d = {load};
    // (Nothing drawn there, or the sky at the far plane.)
    if (d >= 0.99999) {{ discard; }}
    let ndc = vec2<f32>(frag.x / p.screen.x * 2.0 - 1.0, 1.0 - frag.y / p.screen.y * 2.0);
    let w = p.unproject * vec4<f32>(ndc, d, 1.0);
    let world = w.xyz / w.w;
    let l = p.light * vec4<f32>(world, 1.0);
    let uv = vec2<f32>(l.x * 0.5 + 0.5, 0.5 - l.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || l.z > 1.0) {{ discard; }}
    let lit = textureSampleCompareLevel(map, cmp, uv, l.z - p.bias.x);
    let shade = 1.0 - lit;
    // The game's plane: no more opaque than its fade up the screen (from
    // the bottom, as OpenGL counts) nor than the area's opacity in what
    // fog leaves.
    let up = 1.0 - frag.y / p.screen.y;
    let fade = 1.1 * (p.screen.z - up) / p.screen.z - 0.1;
    var fogged = 0.0;
    if (p.fog.x > 0.0) {{
        let along = -(p.view * vec4<f32>(world, 1.0)).z;
        fogged = clamp((along - p.fog.y) * p.fog.w, 0.0, 1.0);
    }}
    let a = clamp(min(fade, p.screen.w * (1.0 - fogged)), 0.0, 1.0) * shade;
    if (a <= 0.0) {{ discard; }}
    return vec4<f32>(0.0, 0.0, 0.0, a);
}}
"
    )
}

/// The darkening pass for a target's format and samples.
#[derive(Debug)]
struct Darkener {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

/// The shadow map and what draws it and reads it, made at the first frame
/// that has shadows.
#[derive(Debug)]
pub(crate) struct ShadowMaps {
    map: wgpu::TextureView,
    sampler: wgpu::Sampler,
    casting: Casting,
    params: wgpu::Buffer,
    darkeners: std::collections::HashMap<(wgpu::TextureFormat, u32), Darkener>,
    /// What the map holds: the light's projection, the casters' meshes,
    /// transforms and bones (drawn again only when they change).
    holds: Option<Held>,
}

/// What a shadow map holds: its projection, its casters (mesh, transform,
/// first bone) and their bones.
type Held = (Mat4, Vec<(usize, Mat4, Option<u32>)>, Vec<Mat4>);

/// What draws casters into a shadow map: a pipeline for rigid meshes and
/// one for skinned, the frame's casters and bones.
#[derive(Debug)]
pub(crate) struct Casting {
    rigid: wgpu::RenderPipeline,
    skinned: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    casters: wgpu::Buffer,
    bones: wgpu::Buffer,
}

impl ShadowMaps {
    pub(crate) fn new(device: &wgpu::Device) -> ShadowMaps {
        let map = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow map"),
                size: wgpu::Extent3d {
                    width: MAP_SIZE,
                    height: MAP_SIZE,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: MAP_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        // (Four texels compared and their results blended: an edge a texel
        // wide, as hard as the game's.)
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow map"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        ShadowMaps {
            map,
            sampler,
            casting: Casting::new(device),
            params: buffer(
                "shadow darkening",
                std::mem::size_of::<DarkenUniform>() as u64,
                wgpu::BufferUsages::UNIFORM,
            ),
            darkeners: Default::default(),
            holds: None,
        }
    }

    /// Draws `plan`'s casters into the map (unless it holds them already).
    pub(crate) fn cast(
        &mut self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        plan: &Plan,
    ) {
        let holds: Vec<_> = plan.casters.iter().map(Caster::key).collect();
        if self
            .holds
            .as_ref()
            .is_some_and(|(l, h, b)| *l == plan.light && *h == holds && *b == plan.bones)
        {
            return;
        }
        self.casting.upload(gpu, &plan.casters, &plan.bones);
        let group = self.casting.group(gpu, plan.light);
        self.casting.draw(encoder, &self.map, &group, &plan.casters, true);
        self.holds = Some((plan.light, holds, plan.bones.clone()));
    }

    /// Lays the shadows over what is drawn in `target` (multisampled by
    /// `samples`, of `format`), as deep as `depth` says, for `camera`
    /// seeing a picture of `size`; `fog`: the scene's.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn darken(
        &mut self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        (target, depth): (&wgpu::TextureView, &wgpu::TextureView),
        (format, samples): (wgpu::TextureFormat, u32),
        plan: &Plan,
        camera: &Camera,
        size: (u32, u32),
        fog: Option<crate::scene::Fog>,
    ) {
        let darkener = self
            .darkeners
            .entry((format, samples))
            .or_insert_with(|| Darkener::new(&gpu.device, format, samples));
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        let view = camera.view();
        let texel = 2.0 / MAP_SIZE as f32;
        let params = DarkenUniform {
            unproject: (camera.projection(aspect) * view).inverse().to_cols_array_2d(),
            light: plan.light.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            screen: [
                size.0.max(1) as f32,
                size.1.max(1) as f32,
                camera_fade_point(camera, plan.shadows.fade_limit),
                plan.shadows.opacity.clamp(0.0, 1.0),
            ],
            fog: fog.map_or([0.0; 4], |f| [1.0, f.start, f.end, 1.0 / (f.end - f.start).max(1e-3)]),
            bias: [BIAS / plan.depth.max(1e-3), texel, 0.0, 0.0],
        };
        gpu.queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow darkening"),
            layout: &darkener.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.params.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.map),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow darkening"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                depth_slice: None,
            })],
            ..Default::default()
        });
        pass.set_pipeline(&darkener.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Casting {
    pub(crate) fn new(device: &wgpu::Device) -> Casting {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow casters"),
            source: wgpu::ShaderSource::Wgsl(CAST.into()),
        });
        let storage = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow casters"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(1),
                storage(2),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow casters"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let position = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<crate::model::Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3],
        };
        let skin = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<crate::model::SkinVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![1 => Uint32x4, 2 => Float32x4],
        };
        let pipeline = |entry: &str, buffers: &[Option<wgpu::VertexBufferLayout<'_>>]| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shadow casters"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    buffers,
                },
                // (Both sides: the game's meshes are not all closed.)
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: MAP_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState { constant: 0, slope_scale: 1.5, clamp: 0.0 },
                }),
                multisample: Default::default(),
                fragment: None,
                multiview_mask: None,
                cache: None,
            })
        };
        let rigid = pipeline("vs", &[Some(position.clone())]);
        let skinned = pipeline("vs_skinned", &[Some(position), Some(skin)]);
        let buffer = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Casting {
            rigid,
            skinned,
            layout,
            casters: buffer("shadow casters", 80 * 256),
            bones: buffer("shadow bones", 64 * 64),
        }
    }

    /// Uploads `casters` and their `bones`.
    pub(crate) fn upload(&mut self, gpu: &crate::Gpu, casters: &[Caster<'_>], bones: &[Mat4]) {
        let data: Vec<CasterData> = casters
            .iter()
            .map(|c| CasterData {
                world: c.world.to_cols_array_2d(),
                bones: [c.bones.unwrap_or(0), 0, 0, 0],
            })
            .collect();
        let grow = |buffer: &mut wgpu::Buffer, bytes: u64, label| {
            if buffer.size() < bytes {
                *buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: bytes.next_power_of_two(),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
        };
        grow(&mut self.casters, std::mem::size_of_val(&data[..]) as u64, "shadow casters");
        grow(&mut self.bones, std::mem::size_of_val(bones) as u64, "shadow bones");
        gpu.queue.write_buffer(&self.casters, 0, bytemuck::cast_slice(&data));
        if !bones.is_empty() {
            gpu.queue.write_buffer(&self.bones, 0, bytemuck::cast_slice(bones));
        }
    }

    /// The bind group for drawing the uploaded casters seen through
    /// `light` (from the world to the map's clip space).
    pub(crate) fn group(&self, gpu: &crate::Gpu, light: Mat4) -> wgpu::BindGroup {
        let light = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow light"),
            contents: bytemuck::cast_slice(&[light]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow casters"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: light.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: self.casters.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: self.bones.as_entire_binding() },
            ],
        })
    }

    /// Draws `casters` (in [`in_drawing_order`]) into `depth`, cleared
    /// first where `clear`, with `group` (a light's, made by
    /// [`Casting::upload`]).
    pub(crate) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        group: &wgpu::BindGroup,
        casters: &[Caster<'_>],
        clear: bool,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow casters"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: if clear { wgpu::LoadOp::Clear(1.0) } else { wgpu::LoadOp::Load },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_bind_group(0, group, &[]);
        let (mut chunk, mut skinned) = (None, None);
        let mut i = 0;
        while i < casters.len() {
            let c = casters[i];
            let mut n = 1;
            while i + n < casters.len() && std::ptr::eq(casters[i + n].mesh, c.mesh) {
                n += 1;
            }
            let skin = c.mesh.skin.as_ref().filter(|_| c.bones.is_some());
            if skinned != Some(skin.is_some()) {
                pass.set_pipeline(if skin.is_some() { &self.skinned } else { &self.rigid });
                skinned = Some(skin.is_some());
                chunk = None;
            }
            if chunk != Some(c.mesh.chunk()) {
                pass.set_vertex_buffer(0, c.mesh.vertices.slice(..));
                pass.set_index_buffer(c.mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                if let Some(skin) = skin {
                    pass.set_vertex_buffer(1, skin.vertices.slice(..));
                }
                chunk = Some(c.mesh.chunk());
            }
            let first = c.mesh.first_index;
            pass.draw_indexed(
                first..first + c.mesh.index_count,
                c.mesh.base_vertex,
                i as u32..(i + n) as u32,
            );
            i += n;
        }
    }
}

impl Darkener {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Darkener {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow darkening"),
            source: wgpu::ShaderSource::Wgsl(darken_source(samples).into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow darkening"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: samples > 1,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow darkening"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        // Black over what is there, as opaque as the shadow: its colour
        // times one less the alpha, its alpha kept.
        let blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow darkening"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            multiview_mask: None,
            cache: None,
        });
        Darkener { pipeline, layout }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The client's camera of the reference scenes (pitch 50, its eye 19 m
    /// from a focus 12 m below it) with the fog ending at 45 m: its
    /// shadows half as opaque as they can be a little over halfway up the
    /// screen, as measured in its pictures (`shadows_look`: 0.45 at 0.54
    /// up, at an opacity of 60 and of 100 alike).
    #[test]
    fn the_shadows_fade_up_the_screen_as_the_client_s() {
        let point = fade_point(50.0, 19.0 * 50f32.to_radians().cos(), 45.0);
        let at = fade(point, 0.5417);
        assert!((at - 0.46).abs() < 0.03, "{at}");
        // Whole at the bottom, none past the top.
        assert!((fade(point, 0.0) - 1.0).abs() < 1e-6);
        assert!(fade(point, 1.0) < 0.05);
        // From straight above, whole all over.
        let above = fade_point(0.0, 30.0, 45.0);
        assert!(fade(above, 1.0) > 0.99, "{}", fade(above, 1.0));
    }

    /// A camera farther than the game's reaches fades the shadows as the
    /// game's at its farthest, at the same pitch, would.
    #[test]
    fn a_far_camera_fades_them_as_the_game_s_farthest() {
        let target = Vec3::new(50.0, 50.0, 0.0);
        let pitch = 50f32.to_radians();
        let at = |distance: f32| {
            let dir = Vec3::new(0.0, -pitch.sin(), pitch.cos());
            Camera { eye: target + dir * distance, target, fov_y: 0.8, near: 0.1, far: 500.0 }
        };
        let farthest = camera_fade_point(&at(GAME_CAMERA_REACH), 45.0);
        assert!((camera_fade_point(&at(200.0), 45.0) - farthest).abs() < 1e-4);
        assert!(camera_fade_point(&at(10.0), 45.0) > farthest);
        // (Some shadow is left in the picture's lower half.)
        assert!(fade(farthest, 0.25) > 0.3, "{}", fade(farthest, 0.25));
    }

    /// The map covers what the camera sees of the receivers: from above,
    /// the patch under it (not the whole area); looking at the horizon,
    /// as far as the receivers go; nothing for a camera looking away.
    #[test]
    fn the_map_covers_what_the_camera_sees_of_the_receivers() {
        let area = (Vec3::ZERO, Vec3::new(320.0, 320.0, 3.0));
        let above = Camera {
            eye: Vec3::new(100.0, 99.99, 30.0),
            target: Vec3::new(100.0, 100.0, 0.0),
            fov_y: 0.8,
            near: 0.1,
            far: 500.0,
        };
        let (min, max) = seen(&above, 1.5, area).unwrap();
        // (30 m up, 0.8 rad across: about 25 m high and 37 m wide.)
        assert!(min.x > 70.0 && max.x < 130.0 && min.y > 75.0 && max.y < 125.0, "{min} {max}");
        assert!(min.x < 85.0 && max.x > 115.0, "{min} {max}");
        let horizon =
            Camera { eye: Vec3::new(10.0, 10.0, 2.0), target: Vec3::new(20.0, 20.0, 2.0), ..above };
        let (min, max) = seen(&horizon, 1.5, area).unwrap();
        assert!(min.cmpge(area.0).all() && max.cmple(area.1).all() && max.x > 300.0, "{min} {max}");
        let away = Camera {
            eye: Vec3::new(-10.0, -10.0, 2.0),
            target: Vec3::new(-20.0, -20.0, 2.0),
            ..above
        };
        assert_eq!(seen(&away, 1.5, area), None);
    }

    /// The light's view looks down along the light: a point towards the
    /// light is nearer it (higher Z in its view), whatever the direction.
    #[test]
    fn the_light_looks_along_its_direction() {
        for dir in [Vec3::new(4000.0, 4500.0, 7000.0).normalize(), Vec3::Z, Vec3::NEG_Z] {
            let v = light_view(dir);
            let (a, b) = (v.transform_point3(Vec3::ZERO), v.transform_point3(dir * 10.0));
            assert!((b.z - a.z - 10.0).abs() < 1e-3, "{dir}: {a} {b}");
            assert!(v.is_finite());
        }
    }
}
