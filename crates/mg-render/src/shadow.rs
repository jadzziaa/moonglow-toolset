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

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::atomic::{AtomicU64, Ordering};

use glam::{Mat4, Vec3};

use crate::model::GpuMesh;
use crate::scene::{Camera, Instance, PointLight, Scene, SunShadows};

/// Monotonic id assigned to each created [`SunMap`] so cached darken bind
/// groups can detect if a view's sun map is ever replaced.
static NEXT_SUN_ID: AtomicU64 = AtomicU64::new(1);

/// The shadow map's side, texels: over an area seen whole (eight tiles
/// across) a texel is 4 cm, finer than the game's volumes are made.
pub(crate) const MAP_SIZE: u32 = 2048;

/// The lights' maps' side, texels, where they are few: each covers one
/// creature seen from its light, a few metres across. They are tiles of
/// one atlas ([`LightMaps`]), [`LIGHT_ATLAS_SIZE`] at most across: past
/// 64 maps they are made smaller, a half each time the count is four
/// times as many (256 at 256 texels, 1,024 at 128).
pub(crate) const LIGHT_MAP_SIZE: u32 = 512;

/// The lights' maps' atlas's greatest side, texels: 64 MiB of depth.
pub(crate) const LIGHT_ATLAS_SIZE: u32 = 4096;

/// The lights' maps' least side, texels: at that many creatures (4,096) a
/// creature's shadow from a lamp a few metres off is still some 6 cm a
/// texel on the ground beyond it.
pub(crate) const LEAST_LIGHT_MAP: u32 = 64;

/// How many creatures cast the lights' shadows at once: every creature in
/// the reach of a light that casts, as the client's (24 round one lamp
/// post: `batch6_light_shadow_ring`; the client casts from one light, the
/// nearest the player, which waits on the owner's ruling: the deferred
/// list's "Shadows"), as many as the atlas holds at its least maps. Past
/// that, the nearest the camera looks at (a creature made of parts is one:
/// [`Instance::object`]).
pub(crate) const MAX_LIGHT_SHADOWS: usize =
    ((LIGHT_ATLAS_SIZE / LEAST_LIGHT_MAP) * (LIGHT_ATLAS_SIZE / LEAST_LIGHT_MAP)) as usize;

/// The side of the screen's squares the darkening pass looks up which
/// lights' maps reach a pixel in, pixels ([`bins`]).
const BIN: u32 = 32;

/// The depth bias in a light's map (perspective, in its clip space's
/// depth): what casts there (a creature) receives none, so a little
/// suffices.
const LIGHT_BIAS: f32 = 1e-4;

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
    inst.casts_shadows && mesh_casts(inst, mesh, alpha)
}

/// [`casts`] but for the instance's own switch: what a mesh of an instance
/// that casts the lights' shadows ([`Instance::casts_light_shadows`],
/// which the scene's maker sets alone) casts from a light.
fn mesh_casts(inst: &Instance, mesh: &GpuMesh, alpha: f32) -> bool {
    !inst.unlit
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
    /// The model it is of (its faces, for its shadow volume:
    /// [`crate::volume`]).
    pub(crate) model: &'a crate::GpuModel,
    /// From the mesh's node to the world.
    pub(crate) world: Mat4,
    /// A skinned mesh's first bone among the frame's casting bones
    /// ([`Plan::bones`]).
    pub(crate) bones: Option<u32>,
    /// A tile's: the bottom of its bounding box, where the game ends its
    /// sun's shadow volumes (EE 8193.31's "tile-bbox lower clip").
    pub(crate) bottom: Option<f32>,
}

impl Caster<'_> {
    /// What the map holds of it: its mesh (its model's number, which no
    /// two models share, and its node), its place, its bones' first. Not
    /// where the mesh lies in memory: a model dropped and another built
    /// where it was, at the same place in the world, was taken as drawn
    /// already, and the map kept the first one's shadow.
    fn key(&self) -> ((u64, usize), Mat4, Option<u32>) {
        ((self.model.id, self.mesh.node), self.world, self.bones)
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
fn posed_box((min, max): (Vec3, Vec3), skinned: bool) -> (Vec3, Vec3) {
    if skinned {
        let grow = (max - min).max_element() * 0.5;
        (min - Vec3::splat(grow), max + Vec3::splat(grow))
    } else {
        (min, max)
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
    let mut points = Vec::with_capacity(15);
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
        points.extend(ends.into_iter().map(|t| near + ray * t));
    }
    let (min, max) = crate::reach::box_of(points)?;
    // (A millimeter's give: the rays' crossings of a flat receiver, a
    // ground of one height, lie at its height but for the last bits, and
    // one a bit above it left no box.)
    let (min, max) = (min.max(receivers.0), max.min(receivers.1));
    let min = Vec3::select(min.cmpgt(max) & (min - max).cmplt(Vec3::splat(1e-3)), max, min);
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
    /// Towards the area light.
    pub(crate) direction: Vec3,
    /// The lowest of what receives shadows: where a sun's volume cast by
    /// what is not a tile ends.
    pub(crate) floor: f32,
}

/// A caster met, before the cut to the map's region: its box, and a
/// skinned one's pose.
type Met<'a> = (Caster<'a>, (Vec3, Vec3), Option<&'a [Mat4]>);

/// The shadow map a scene needs, seen by `camera` in a picture of
/// `aspect`: `None` where it has no shadows (none asked for, none to cast
/// or none to receive one in sight). `volumes`: it is the stencil
/// volumes' (else the map's), which cast no skinned mesh, as the game's
/// cast none, and end a tile's volumes at its box's bottom
/// ([`Caster::bottom`], which the map reads nothing of).
pub(crate) fn plan<'a>(
    scene: &'a Scene,
    camera: &Camera,
    aspect: f32,
    volumes: bool,
) -> Option<Plan<'a>> {
    let skins = !volumes;
    let shadows = scene.shadows.filter(|s| s.opacity > 0.0)?;
    // (Each with its box, and a skinned one's pose: its bones are worked
    // out once it is known to cast into the map's region.)
    let mut casters: Vec<Met<'a>> = Vec::new();
    let mut bones = Vec::new();
    // (The box around what receives them: of boxes known to be somewhere,
    // so taken as it goes, as [`crate::reach::box_of`] each mesh cost a
    // large area's frame a fifth of a millisecond; no box where none
    // receives.)
    let mut receivers = crate::reach::NO_BOX;
    let somewhere = |(min, max): (Vec3, Vec3)| min.is_finite() && max.is_finite();
    for inst in &scene.instances {
        let pose: &[Mat4] = inst.pose.as_deref().unwrap_or(&inst.model.rest);
        let node = |n: usize| inst.transform * pose.get(n).copied().unwrap_or(Mat4::IDENTITY);
        let model: &'a crate::GpuModel = &inst.model;
        let track_bottom = inst.tile && volumes;
        let caster_start = casters.len();
        let mut tile_bottom = f32::MAX;
        for (j, mesh) in inst.model.meshes.iter().enumerate() {
            let replaced = inst.state.as_ref().and_then(|s| s.meshes.get(j));
            let alpha = replaced.and_then(|r| r.alpha).unwrap_or(mesh.material.alpha);
            let world = node(mesh.node);
            if alpha <= 0.0 {
                if track_bottom {
                    tile_bottom = tile_bottom.min(world_box(world, mesh).0.z);
                }
                continue;
            }
            let bounds = world_box(world, mesh);
            if track_bottom {
                tile_bottom = tile_bottom.min(bounds.0.z);
            }
            // (A mesh that is nowhere, its instance's place no number: it
            // neither receives nor casts. Its box took every receiver's
            // before it with it, and the area's shadows where it came
            // last.)
            if !somewhere(bounds) {
                continue;
            }
            // (One left out, Fade Geometry's, casts but isn't drawn to
            // receive.)
            let hidden = replaced.is_some_and(|r| r.hidden);
            if inst.receives_shadows && inst.opacity >= 1.0 && !hidden {
                receivers = (receivers.0.min(bounds.0), receivers.1.max(bounds.1));
            }
            let skinned = mesh.skin.is_some();
            if casts(inst, mesh, alpha) && (skins || !skinned) {
                let caster = Caster { mesh, model, world, bones: None, bottom: None };
                casters.push((caster, posed_box(bounds, skinned), skinned.then_some(pose)));
            }
        }
        for mesh in &inst.model.shadow_only {
            let skinned = mesh.skin.is_some();
            let world = node(mesh.node);
            if casts(inst, mesh, 1.0) && (skins || !skinned) {
                let raw = world_box(world, mesh);
                if track_bottom {
                    tile_bottom = tile_bottom.min(raw.0.z);
                }
                let bounds = posed_box(raw, skinned);
                if !somewhere(bounds) {
                    continue;
                }
                let caster = Caster { mesh, model, world, bones: None, bottom: None };
                casters.push((caster, bounds, skinned.then_some(pose)));
            } else if track_bottom {
                tile_bottom = tile_bottom.min(world_box(world, mesh).0.z);
            }
        }
        if track_bottom {
            for (caster, ..) in &mut casters[caster_start..] {
                caster.bottom = Some(tile_bottom);
            }
        }
    }
    let receivers =
        Some(receivers).filter(|(min, max)| !casters.is_empty() && min.cmple(*max).all())?;
    let region = seen(camera, aspect, receivers)?;
    let view = light_view(scene.area.direction);
    let to_light = |(min, max): (Vec3, Vec3)| {
        crate::reach::box_of(crate::box_corners(min, max).map(|c| view.transform_point3(c)))
    };
    let (lo, mut hi) = to_light(region)?;
    // The casters whose shadows can fall on what is seen: across the
    // light, over the region; the box reaching back to the farthest
    // towards the light.
    let mut kept = Vec::with_capacity(casters.len());
    for (mut caster, bounds, pose) in casters {
        let Some((clo, chi)) = to_light(bounds) else { continue };
        if chi.x < lo.x || clo.x > hi.x || chi.y < lo.y || clo.y > hi.y || chi.z < lo.z {
            continue;
        }
        hi.z = hi.z.max(chi.z);
        // (The bones of those kept alone: the map is drawn again when
        // they change, and a creature animating out of its region changed
        // them every frame.)
        caster.bones = pose.and_then(|pose| skin_bones(caster.mesh, pose, &mut bones));
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
    Some(Plan {
        light: projection * view,
        casters: kept,
        bones,
        depth: far - near,
        shadows,
        direction: scene.area.direction,
        floor: receivers.0.z,
    })
}

/// The lights' shadows of a frame: per creature casting one, the light's
/// projection (from the world to its map's clip space) and its casters
/// (in [`LightPlan::casters`], from and to).
pub(crate) struct LightPlan<'a> {
    pub(crate) lights: Vec<Mat4>,
    /// Per creature, where its shadow is cast from (its light, lifted) and
    /// how far the light reaches from there (its shadow volume's radius).
    pub(crate) sources: Vec<(Vec3, f32)>,
    pub(crate) ranges: Vec<std::ops::Range<u32>>,
    /// Per creature, its bones in [`LightPlan::bones`].
    pub(crate) bone_ranges: Vec<std::ops::Range<u32>>,
    pub(crate) casters: Vec<Caster<'a>>,
    pub(crate) bones: Vec<Mat4>,
    pub(crate) shadows: SunShadows,
    /// The light at the receivers a shadow's darkness is the casting
    /// light's share of ([`Share`]).
    pub(crate) share: Share,
    /// Per creature, its light's place in the share's lights (`u32::MAX`
    /// where it is not one of them).
    pub(crate) share_index: Vec<u32>,
    /// The height of the lowest of what receives shadows: no shadow falls
    /// below it ([`bins`]).
    pub(crate) floor: f32,
}

impl LightPlan<'_> {
    /// Whether a map's light is not one of the share's (more lights cast
    /// than [`MAX_SHARE_LIGHTS`]): its shadow is as dark as the opacity.
    pub(crate) fn overflows(&self) -> bool {
        self.share_index.contains(&u32::MAX)
    }
}

/// How many of a scene's lights a light's shadow's share of the light at
/// a receiver is worked out with (the casting ones, then the nearest the
/// camera's target).
pub(crate) const MAX_SHARE_LIGHTS: usize = 32;

/// What a light's shadow's darkness is worked out from: the client darkens
/// a receiver by the casting light's share of all the light falling there
/// (its brightness with the light less without, over with), no more than
/// the plane's opacity (`reading_light_shadows`: 0.50, 0.52, 0.53, 0.47,
/// 0.38, 0.29, 0.21, 0.13, 0.11 and 0.08 at 1.5, 2, 3 … 10 m from a lamp
/// of radius 10, its shares 0.58 to 0.07; the findings, "A light's shadow
/// takes the light's share"). The area's light and the lights about (in
/// the world, linear, as Moonglow's shader lights with them), and which of
/// them each map's shadow is cast from.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Share {
    /// The area's ambient and diffuse colors, and the way to its light.
    area: [[f32; 4]; 3],
    /// 1 over the lights' greatest intensity and their falloff (the
    /// shader's attenuation), how many of `lights` there are, and how many
    /// of them (the first) the maps' shadows are cast from.
    falloff: [f32; 4],
    /// Each its place (w: its reach squared, less than 0 for an ambient
    /// one) and color.
    lights: [[[f32; 4]; 2]; MAX_SHARE_LIGHTS],
}

impl Share {
    /// Of `scene`, the lights `sources` (each map's, by its place in the
    /// scene's), and the others nearest `target`; and each map's light's
    /// place in them (`u32::MAX` past [`MAX_SHARE_LIGHTS`] lights casting).
    fn of(scene: &Scene, sources: &[usize], target: Vec3) -> (Share, Vec<u32>) {
        let mut chosen: Vec<usize> = Vec::with_capacity(MAX_SHARE_LIGHTS);
        for &s in sources {
            if !chosen.contains(&s) {
                chosen.push(s);
                if chosen.len() == MAX_SHARE_LIGHTS {
                    break;
                }
            }
        }
        let casting = chosen.len();
        let need = MAX_SHARE_LIGHTS.saturating_sub(chosen.len());
        if need > 0 {
            let mut others: Vec<(f32, usize)> = (0..scene.lights.len())
                .filter(|i| !chosen.contains(i))
                .map(|i| (scene.lights[i].position.distance_squared(target), i))
                .collect();
            let cmp = |a: &(f32, usize), b: &(f32, usize)| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1));
            if others.len() > need {
                others.select_nth_unstable_by(need, cmp);
                others.truncate(need);
            }
            others.sort_unstable_by(cmp);
            chosen.extend(others.into_iter().map(|(_, i)| i));
        }
        let mut share = <Share as bytemuck::Zeroable>::zeroed();
        let a = &scene.area;
        share.area = [
            a.ambient.extend(1.0).to_array(),
            a.diffuse.extend(1.0).to_array(),
            a.direction.normalize_or_zero().extend(0.0).to_array(),
        ];
        let (max_inv, falloff) = crate::renderer::attenuation_params();
        share.falloff = [max_inv, falloff, chosen.len() as f32, casting as f32];
        for (to, &i) in share.lights.iter_mut().zip(&chosen) {
            let l: &PointLight = &scene.lights[i];
            let r2 = l.cutoff * l.cutoff;
            *to = [
                l.position.extend(if l.ambient_only { -r2 } else { r2 }).to_array(),
                l.color.extend(1.0).to_array(),
            ];
        }
        let places = sources
            .iter()
            .map(|s| chosen[..casting].iter().position(|c| c == s).map_or(u32::MAX, |p| p as u32))
            .collect();
        (share, places)
    }
}

/// The light a creature at `at` (its box's middle), `radius` round,
/// standing at `feet` (its place), casts its shadow from: of `lights` that
/// cast shadows, reach it and are outside it (a torch it holds casts
/// none), the nearest to its place, as the client picks (`shadows_look`:
/// of two lamps, the nearer; `reading_light_shadows`: of two lamp posts
/// 2.5 m either side, the one whose light, set off 1 cm by its row's
/// `LightOffsetX` and `Y`, is nearer the ogre's feet, whichever way it
/// faces, where its box's middle turns with it).
pub(crate) fn casting_light(
    lights: &[PointLight],
    at: Vec3,
    radius: f32,
    feet: Vec3,
) -> Option<usize> {
    lights
        .iter()
        .enumerate()
        .filter(|(_, l)| l.casts_shadows && !l.ambient_only)
        .filter(|(_, l)| {
            let d = l.position.distance(at);
            d > radius && d <= l.cutoff
        })
        .map(|(i, l)| (i, l.position.distance(feet)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// A perspective view from `light` that holds a ball at `at`, `radius`
/// round, and what lies behind it out to `reach` metres from the light:
/// from the world to the clip space of the light's map.
pub(crate) fn light_projection(light: Vec3, at: Vec3, radius: f32, reach: f32) -> Mat4 {
    let along = at - light;
    let d = along.length().max(1e-3);
    let up =
        if along.normalize().cross(Vec3::Z).length_squared() < 1e-6 { Vec3::Y } else { Vec3::Z };
    let view = Mat4::look_at_rh(light, at, up);
    let half = (radius / d).clamp(0.0, 0.95).asin() * 1.1;
    let near = (d - radius).max(0.05);
    let far = reach.max(d + radius + 1.0);
    Mat4::perspective_rh(2.0 * half, 1.0, near, far) * view
}

/// The lights' shadows a scene needs, seen by `camera`: `None` where it
/// has none (none asked for, or no creature with a light that casts).
/// `volumes`: it is the stencil volumes', which cast no skinned mesh
/// ([`plan`]).
pub(crate) fn light_plan<'a>(
    scene: &'a Scene,
    camera: &Camera,
    volumes: bool,
) -> Option<LightPlan<'a>> {
    let skins = !volumes;
    let shadows = scene.light_shadows.filter(|s| s.opacity > 0.0)?;
    // A creature's instances (its base first, then its parts: one made of
    // body parts is an instance a part), the nearest the camera's target
    // first.
    let mut creatures: Vec<(f32, Vec<&'a Instance>)> = Vec::new();
    let mut of_object: HashMap<u32, usize> = HashMap::new();
    for inst in scene.instances.iter().filter(|i| i.casts_light_shadows) {
        let known = inst.object.and_then(|o| match of_object.entry(o) {
            Entry::Occupied(e) => Some(*e.get()),
            Entry::Vacant(e) => {
                e.insert(creatures.len());
                None
            }
        });
        match known {
            Some(at) => creatures[at].1.push(inst),
            None => {
                let from_target = inst.transform.w_axis.truncate().distance(camera.target);
                creatures.push((from_target, vec![inst]));
            }
        }
    }
    // (In the scene's order, which keeps each creature's map where it was
    // from one frame to the next, and drawn again only when it changes:
    // the nearest the camera's target first only where there are more
    // than the maps.)
    if creatures.len() > MAX_LIGHT_SHADOWS {
        creatures.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    let mut plan = LightPlan {
        lights: Vec::new(),
        sources: Vec::new(),
        ranges: Vec::new(),
        bone_ranges: Vec::new(),
        casters: Vec::new(),
        bones: Vec::new(),
        shadows,
        share: bytemuck::Zeroable::zeroed(),
        share_index: Vec::new(),
        floor: f32::NEG_INFINITY,
    };
    // (Each map's light, by its place in the scene's.)
    let mut cast_from = Vec::new();
    for (_, parts) in creatures {
        if plan.lights.len() == MAX_LIGHT_SHADOWS {
            break;
        }
        let (first, first_bone) = (plan.casters.len(), plan.bones.len());
        // (The box around all of it, of parts known to be somewhere.)
        let (mut lo, mut hi) = crate::reach::NO_BOX;
        for inst in &parts {
            let pose: &[Mat4] = inst.pose.as_deref().unwrap_or(&inst.model.rest);
            let meshes = inst.model.meshes.iter().enumerate().map(|(j, m)| (Some(j), m));
            for (j, mesh) in meshes.chain(inst.model.shadow_only.iter().map(|m| (None, m))) {
                let replaced = j.and_then(|j| inst.state.as_ref().and_then(|s| s.meshes.get(j)));
                let alpha = replaced.and_then(|r| r.alpha).unwrap_or(mesh.material.alpha);
                // (Not `casts`: whether a creature casts the sun's shadows is
                // another switch, Creature Shadows, which the scene's maker
                // reads for both: `mg_area::View::light_shadows_cast`.)
                if !mesh_casts(inst, mesh, if j.is_some() { alpha } else { 1.0 })
                    || (mesh.skin.is_some() && !skins)
                {
                    continue;
                }
                let world = inst.transform * pose.get(mesh.node).copied().unwrap_or(Mat4::IDENTITY);
                // (A part that is nowhere casts nothing.)
                let (a, b) = posed_box(world_box(world, mesh), mesh.skin.is_some());
                if !(a.is_finite() && b.is_finite()) {
                    continue;
                }
                (lo, hi) = (lo.min(a), hi.max(b));
                let skinned = skin_bones(mesh, pose, &mut plan.bones);
                let model = &inst.model;
                plan.casters.push(Caster { mesh, model, world, bones: skinned, bottom: None });
            }
        }
        if plan.casters.len() == first {
            continue;
        }
        // (One light for the whole of it, from where its base stands, and
        // one map: its box is all its parts'.)
        let (at, radius) = ((lo + hi) * 0.5, (hi - lo).length() * 0.5);
        let feet = parts[0].transform.w_axis.truncate();
        let Some(light) = casting_light(&scene.lights, at, radius, feet) else {
            // (With its bones: nothing draws them.)
            plan.casters.truncate(first);
            plan.bones.truncate(first_bone);
            continue;
        };
        cast_from.push(light);
        let light = &scene.lights[light];
        let from = light.position + Vec3::Z * light.shadow_lift;
        let reach = light.cutoff + light.shadow_lift;
        plan.lights.push(light_projection(from, at, radius, reach));
        plan.sources.push((from, reach));
        in_drawing_order(&mut plan.casters[first..]);
        plan.ranges.push(first as u32..plan.casters.len() as u32);
        plan.bone_ranges.push(first_bone as u32..plan.bones.len() as u32);
    }
    if plan.lights.is_empty() {
        return None;
    }
    (plan.share, plan.share_index) = Share::of(scene, &cast_from, camera.target);
    plan.floor = receivers_floor(scene);
    Some(plan)
}

/// The height of the lowest mesh that receives shadows in `scene` (as it
/// is posed), minus infinity where none does.
fn receivers_floor(scene: &Scene) -> f32 {
    let mut floor = f32::INFINITY;
    for inst in scene.instances.iter().filter(|i| i.receives_shadows) {
        let pose: &[Mat4] = inst.pose.as_deref().unwrap_or(&inst.model.rest);
        for mesh in &inst.model.meshes {
            let world = inst.transform * pose.get(mesh.node).copied().unwrap_or(Mat4::IDENTITY);
            let low = world_box(world, mesh).0.z;
            if low.is_finite() {
                floor = floor.min(low);
            }
        }
    }
    if floor.is_finite() { floor } else { f32::NEG_INFINITY }
}

/// The frame's values the darkening pass reads (`darken_source`).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DarkenUniform {
    /// From the screen (x, y from -1 to 1, the depth buffer's depth) to
    /// the world.
    unproject: [[f32; 4]; 4],
    /// The view, for a pixel's fog (its depth along the camera).
    view: [[f32; 4]; 4],
    /// The picture's width and height, the fade point, the opacity.
    screen: [f32; 4],
    /// 1 with fog, its start and end and 1 over its length; else zeros.
    fog: [f32; 4],
    /// The bias in the maps' depth.
    bias: [f32; 4],
    /// How many maps (the lights'; the sun's is one), how many squares
    /// across and down the screen is binned in ([`bins`]), and 1 where a
    /// map's light is not one of the share's ([`MAX_SHARE_LIGHTS`]).
    count: [u32; 4],
    /// From the world to the sun's map's clip space (the lights' are in
    /// [`MapData`]).
    sun: [[f32; 4]; 4],
    /// The lights' shadows' share of the light ([`Share`]; zeros for the
    /// sun's).
    share: Share,
}

/// The casting pass's shader: positions alone (and a skinned mesh's bones
/// and weights), each instance its world transform and its bones' first.
pub(crate) const CAST: &str = "
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

/// The darkening uniform's size, bytes.
pub(crate) const DARKEN_UNIFORM_SIZE: u64 = std::mem::size_of::<DarkenUniform>() as u64;

/// The darkening's values for the stencil volumes' shadows
/// ([`crate::volume`]): no maps, the look as the maps' have it; the
/// lights' with their plan's share of the light and how many cast.
pub(crate) fn stencil_uniform(
    shadows: SunShadows,
    lights: Option<&LightPlan>,
    camera: &Camera,
    size: (u32, u32),
    fog: Option<crate::scene::Fog>,
) -> Vec<u8> {
    let count = lights.map_or(0, |l| l.lights.len() as u32);
    let look = (shadows, 0.0, [count, 0, 0, u32::from(lights.is_some_and(LightPlan::overflows))]);
    let share = lights.map(|l| l.share);
    bytemuck::bytes_of(&uniform(look, [[0.0; 4]; 4], share, camera, size, fog)).to_vec()
}

/// The darkening pass's shader for `samples` samples a pixel: from the
/// sun's map (orthographic), or from the lights' (perspective, a tile of
/// the atlas each, the darkest taken of those whose light's view takes in
/// the pixel's square of the screen: [`bins`]); or, where `stencil`, over
/// every pixel the pass's stencil test lets through (the shadow volumes'
/// counts), the sun's or the `lights`' way.
///
/// The sun's: no more opaque than the plane's fade up the screen nor the
/// opacity. A light's: its share of the light at the receiver ([`Share`],
/// worked out as Moonglow's shader lights it, the receiver's normal from
/// its depth's slope), no more than the opacity times the fade (as
/// measured in the client at 30 and 100: 0.14 and 0.49 where the fade is
/// 0.5, where the sun's are 0.30 and 0.45). Where the stencil volumes
/// count a light's shadow, which light's isn't known: the largest share of
/// the lights that cast one.
pub(crate) fn darken_source(samples: u32, lights: bool, stencil: bool) -> String {
    let (depth, sample, load) = if samples > 1 {
        (
            crate::framebuffer::depth_read_type(samples),
            ", @builtin(sample_index) s: u32",
            "textureLoad(depth, at, i32(s)).x",
        )
    } else {
        (crate::framebuffer::depth_read_type(1), "", "textureLoad(depth, at, 0).x")
    };
    let (map, dark) = match (stencil, lights) {
        (true, false) => ("", "let a = clamp(min(fade, p.screen.w * (1.0 - fogged)), 0.0, 1.0);"),
        // (Which light cast what the volumes count isn't known: the
        // greatest share of the lights casting, each once.)
        (true, true) => (
            "",
            "var a = 0.0;
    if (p.count.w != 0u) { a = cap; }
    let casting = u32(p.share_falloff.w);
    for (var i = 0u; i < casting; i = i + 1u) {
        a = max(a, min(share_of(i, world, n), cap));
    }",
        ),
        (false, true) => (
            "texture_depth_2d",
            "var a = 0.0;
    // (With one light casting, every map darkens by its share alone: the
    // first that shades the pixel whole is the darkest.)
    let single = u32(p.share_falloff.w) == 1u && p.count.w == 0u;
    let cell = min(vec2<u32>(frag.xy) / BIN, p.count.yz - vec2<u32>(1u));
    let b = cell.y * p.count.y + cell.x;
    let list = p.count.y * p.count.z + 1u;
    for (var j = bins[b]; j < bins[b + 1u]; j = j + 1u) {
        let k = bins[list + j];
        let c = maps[k].m * vec4<f32>(world, 1.0);
        if (c.w <= 0.0) { continue; }
        let m = c.xyz / c.w;
        if (any(abs(m.xy) > vec2<f32>(1.0)) || m.z < 0.0 || m.z > 1.0) { continue; }
        // (Within its tile, half a texel in from the edge: the compare
        // takes the texels about it, not its neighbour's.)
        let t = maps[k].tile;
        let inner = clamp(vec2<f32>(m.x * 0.5 + 0.5, 0.5 - m.y * 0.5), vec2<f32>(t.w), vec2<f32>(1.0 - t.w));
        let uv = t.xy + inner * t.z;
        let shade = 1.0 - textureSampleCompareLevel(map, cmp, uv, m.z - p.bias.x);
        if (shade > 0.0) {
            a = max(a, shade * min(share_of(maps[k].source.x, world, n), cap));
            if (a >= cap || (single && shade >= 1.0)) { break; }
        }
    }",
        ),
        (false, false) => (
            "texture_depth_2d",
            "let l = p.sun * vec4<f32>(world, 1.0);
    let uv = vec2<f32>(l.x * 0.5 + 0.5, 0.5 - l.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || l.z > 1.0) {
        discard;
        return Out(vec4<f32>(0.0), vec4<f32>(0.0));
    }
    let shade = 1.0 - textureSampleCompareLevel(map, cmp, uv, l.z - p.bias.x);
    let a = clamp(min(fade, p.screen.w * (1.0 - fogged)), 0.0, 1.0) * shade;",
        ),
    };
    let mut maps = if stencil {
        String::new()
    } else {
        format!(
            "@group(0) @binding(2) var map: {map};
@group(0) @binding(3) var cmp: sampler_comparison;"
        )
    };
    if lights && !stencil {
        maps.push_str(&format!(
            "
// Each light's map: from the world to its clip space; its tile (its
// corner and side in the atlas, and half a texel of it); its light's
// place in the share's.
struct Map {{
    m: mat4x4<f32>,
    tile: vec4<f32>,
    source: vec4<u32>,
}}
@group(0) @binding(4) var<storage, read> maps: array<Map>;
// Per square of the screen, where its maps start in the list after the
// squares' starts (and one more, the end), then the list.
@group(0) @binding(5) var<storage, read> bins: array<u32>;
const BIN: u32 = {BIN}u;"
        ));
    }
    format!(
        "// The picture's darkening, and the same in the shadows' mask (what
// the picture was multiplied by, read by what is drawn after it).
struct Out {{
    @location(0) color: vec4<f32>,
    @location(1) mask: vec4<f32>,
}}
struct Params {{
    unproject: mat4x4<f32>,
    view: mat4x4<f32>,
    screen: vec4<f32>,
    fog: vec4<f32>,
    bias: vec4<f32>,
    count: vec4<u32>,
    sun: mat4x4<f32>,
    share_area: array<vec4<f32>, 3>,
    share_falloff: vec4<f32>,
    share_lights: array<vec4<f32>, {share_lights}>,
}}
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var depth: {depth};
{maps}
fn brightness(c: vec3<f32>) -> f32 {{
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}}
// How a light of `bright` lights a receiver facing `n` from the way `l`, as
// Moonglow's shader lights it (softened past the terminator).
fn lit(bright: f32, n: vec3<f32>, l: vec3<f32>) -> f32 {{
    let n_dot_l = dot(n, l);
    if (n_dot_l <= -0.25) {{ return 0.0; }}
    return bright * 2.0 * smoothstep(0.0, 2.0, n_dot_l * 0.8 + 0.2);
}}
// The light the share's light `i` gives a receiver at `world` facing `n`.
fn light_at(i: u32, world: vec3<f32>, n: vec3<f32>) -> f32 {{
    let at = p.share_lights[2u * i];
    let to = at.xyz - world;
    let d2 = dot(to, to);
    let r2 = abs(at.w);
    if (d2 > r2) {{ return 0.0; }}
    let f = d2 / r2;
    let att = (1.0 - f) / (p.share_falloff.x + p.share_falloff.y * f);
    let bright = brightness(p.share_lights[2u * i + 1u].rgb) * att;
    if (at.w < 0.0) {{ return bright; }}
    return lit(bright, n, normalize(to));
}}
// The share's light `source`'s share of the brightness at `world` (facing
// `n`): what would be lost without it, in the picture's (gamma) terms. 1
// where its light isn't known.
fn share_of(source: u32, world: vec3<f32>, n: vec3<f32>) -> f32 {{
    let count = u32(p.share_falloff.z);
    if (source >= count) {{ return 1.0; }}
    var total = brightness(p.share_area[0].rgb)
        + lit(brightness(p.share_area[1].rgb), n, p.share_area[2].xyz);
    var mine = 0.0;
    for (var i = 0u; i < count; i = i + 1u) {{
        let v = light_at(i, world, n);
        total = total + v;
        if (i == source) {{ mine = v; }}
    }}
    if (total <= 0.0) {{ return 1.0; }}
    return 1.0 - pow(clamp(1.0 - mine / total, 0.0, 1.0), 1.0 / 2.2);
}}
{vs}@fragment fn fs(@builtin(position) frag: vec4<f32>{sample}) -> Out {{
    let at = vec2<i32>(frag.xy);
    let d = {load};
    let ndc = vec2<f32>(frag.x / p.screen.x * 2.0 - 1.0, 1.0 - frag.y / p.screen.y * 2.0);
    let w = p.unproject * vec4<f32>(ndc, d, 1.0);
    let world = w.xyz / w.w;
    // The receiver's normal, from the slope of its place (taken before
    // anything is discarded), turned to face the camera.
    var n = cross(dpdx(world), dpdy(world));
    n = select(vec3<f32>(0.0, 0.0, 1.0), normalize(n), dot(n, n) > 1e-20);
    if ((p.view * vec4<f32>(n, 0.0)).z < 0.0) {{ n = -n; }}
    // (Nothing drawn there, or the sky at the far plane.) Every discard
    // returns at once (Metal runs a discarded fragment on).
    if (d >= 0.99999) {{
        discard;
        return Out(vec4<f32>(0.0), vec4<f32>(0.0));
    }}
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
    let cap = clamp(clamp(fade, 0.0, 1.0) * p.screen.w * (1.0 - fogged), 0.0, 1.0);
    {dark}
    if (a <= 0.0) {{
        discard;
        return Out(vec4<f32>(0.0), vec4<f32>(0.0));
    }}
    return Out(vec4<f32>(0.0, 0.0, 0.0, a), vec4<f32>(0.0, 0.0, 0.0, a));
}}
",
        share_lights = 2 * MAX_SHARE_LIGHTS,
        vs = crate::framebuffer::FULLSCREEN_VS,
    )
}

/// How the darkening is laid (the map's and the stencil volumes'): black
/// over what is there, as opaque as the shadow: its colour times one less
/// the alpha, its alpha kept.
pub(crate) const DARKEN_BLEND: wgpu::BlendState = wgpu::BlendState {
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

/// The shadows' mask's format: one channel, what the picture was
/// multiplied by.
pub(crate) const MASK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

/// The darkening passes' second target: the shadows' mask, darkened as
/// the picture is.
pub(crate) const MASK_TARGET: wgpu::ColorTargetState = wgpu::ColorTargetState {
    format: MASK_FORMAT,
    blend: Some(DARKEN_BLEND),
    write_mask: wgpu::ColorWrites::RED,
};

/// Numbers that tell one mask from another (0: none, white).
static NEXT_MASK_ID: AtomicU64 = AtomicU64::new(1);

/// A view's mask of the shadows laid over its picture: white, darkened
/// by every darkening pass as the picture is, so that it holds what the
/// picture was multiplied by. What is drawn after the shadows and
/// receives them (a see-through mesh's soft part, which hides nothing
/// and so can't be under them) multiplies itself by it. Multisampled as
/// the picture is, resolved into `view` at every pass.
#[derive(Debug)]
pub(crate) struct Mask {
    pub(crate) id: u64,
    pub(crate) size: (u32, u32),
    pub(crate) samples: u32,
    msaa: Option<wgpu::TextureView>,
    /// What is read: the mask, resolved.
    pub(crate) view: wgpu::TextureView,
}

impl Mask {
    pub(crate) fn new(device: &wgpu::Device, size: (u32, u32), samples: u32) -> Mask {
        let make = |samples, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("shadows' mask"),
                    size: wgpu::Extent3d {
                        width: size.0.max(1),
                        height: size.1.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format: MASK_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let msaa = (samples > 1).then(|| make(samples, wgpu::TextureUsages::empty()));
        let view = make(1, wgpu::TextureUsages::TEXTURE_BINDING);
        let id = NEXT_MASK_ID.fetch_add(1, Ordering::Relaxed);
        Mask { id, size, samples, msaa, view }
    }

    /// The mask as a darkening pass's second target, laid over what it
    /// holds; or cleared to white, where `clear`.
    pub(crate) fn attachment(&self, clear: bool) -> wgpu::RenderPassColorAttachment<'_> {
        let (view, resolve_target) = match &self.msaa {
            Some(msaa) => (msaa, Some(&self.view)),
            None => (&self.view, None),
        };
        let load = if clear { wgpu::LoadOp::Clear(wgpu::Color::WHITE) } else { wgpu::LoadOp::Load };
        wgpu::RenderPassColorAttachment {
            view,
            resolve_target,
            ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
            depth_slice: None,
        }
    }

    /// Clears the mask to white: nothing darkened yet.
    pub(crate) fn clear(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadows' mask"),
            color_attachments: &[Some(self.attachment(true))],
            ..Default::default()
        });
    }
}

/// The darkening pass for a target's format and samples.
#[derive(Debug)]
struct Darkener {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

/// What draws the shadow maps and reads them, made at the first frame
/// that has shadows. The sun's map itself is a view's ([`SunMap`]).
#[derive(Debug)]
pub(crate) struct ShadowMaps {
    sampler: wgpu::Sampler,
    casting: Casting,
    params: wgpu::Buffer,
    darkeners: std::collections::HashMap<(wgpu::TextureFormat, u32, bool), Darkener>,
    /// The lights' maps, made at the first frame with lights' shadows.
    lights: Option<LightMaps>,
}

/// A view's map of the sun's shadows, kept from one of its frames to the
/// next ([`crate::views`]): made at the view's first frame with them.
#[derive(Debug)]
pub(crate) struct SunMap {
    id: u64,
    map: wgpu::TextureView,
    /// What the map holds: the light's projection, the casters' meshes,
    /// transforms and bones (drawn again only when they change).
    holds: Option<Held>,
}

impl SunMap {
    fn new(device: &wgpu::Device) -> SunMap {
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
        SunMap { id: NEXT_SUN_ID.fetch_add(1, Ordering::Relaxed), map, holds: None }
    }
}

/// What a shadow map holds: its projection, its casters (mesh, transform,
/// first bone) and their bones.
type Held = (Mat4, Vec<((u64, usize), Mat4, Option<u32>)>, Vec<Mat4>);

/// What draws casters into a shadow map: a pipeline for rigid meshes and
/// one for skinned, the frame's lights (dynamic-offset uniform), casters,
/// and bones, plus their cached bind group and CPU staging scratch.
#[derive(Debug)]
pub(crate) struct Casting {
    rigid: wgpu::RenderPipeline,
    skinned: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    light_stride: u32,
    lights_buf: wgpu::Buffer,
    lights_staging: Vec<u8>,
    caster_scratch: Vec<CasterData>,
    casters: wgpu::Buffer,
    bones: wgpu::Buffer,
    group: Option<wgpu::BindGroup>,
}

impl ShadowMaps {
    pub(crate) fn new(device: &wgpu::Device) -> ShadowMaps {
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
            sampler,
            casting: Casting::new(device, Some(wgpu::Face::Back)),
            params: buffer(
                "shadow darkening",
                std::mem::size_of::<DarkenUniform>() as u64,
                wgpu::BufferUsages::UNIFORM,
            ),
            darkeners: Default::default(),
            lights: None,
        }
    }

    /// Draws `plan`'s casters into the view's `map` (made where it has
    /// none), unless it holds them already: whether it drew them.
    pub(crate) fn cast(
        &mut self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        plan: &Plan,
        map: &mut Option<SunMap>,
    ) -> bool {
        let map = map.get_or_insert_with(|| SunMap::new(&gpu.device));
        if map.holds.as_ref().is_some_and(|(l, h, b)| {
            *l == plan.light
                && h.len() == plan.casters.len()
                && h.iter().zip(&plan.casters).all(|(k, c)| *k == c.key())
                && *b == plan.bones
        }) {
            return false;
        }
        self.casting.upload(gpu, &plan.casters, &plan.bones, std::slice::from_ref(&plan.light));
        self.casting.draw(encoder, &map.map, 0, &plan.casters, 0, true);
        if let Some((l, h, b)) = &mut map.holds {
            *l = plan.light;
            h.clear();
            h.extend(plan.casters.iter().map(Caster::key));
            b.clone_from(&plan.bones);
        } else {
            map.holds = Some((
                plan.light,
                plan.casters.iter().map(Caster::key).collect(),
                plan.bones.clone(),
            ));
        }
        true
    }

    /// Lays the sun's shadows, as the view's `map` has them, over what is
    /// drawn in `target` (multisampled by `samples`, of `format`), as deep
    /// as `depth` says, for `camera` seeing a picture of `size`; `fog`:
    /// the scene's.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn darken(
        &mut self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&wgpu::TextureView, &wgpu::TextureView),
        mask: &Mask,
        format: (wgpu::TextureFormat, u32),
        (plan, map): (&Plan, &SunMap),
        camera: &Camera,
        size: (u32, u32),
        fog: Option<crate::scene::Fog>,
        cache: Option<&mut Option<(u64, wgpu::BindGroup)>>,
    ) {
        let look = (plan.shadows, BIAS / plan.depth.max(1e-3), [1, 0, 0, 0]);
        let params = uniform(look, plan.light.to_cols_array_2d(), None, camera, size, fog);
        gpu.queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let darkener = self
            .darkeners
            .entry((format.0, format.1, false))
            .or_insert_with(|| Darkener::new(&gpu.device, format.0, format.1, false));
        let mut scratch = None;
        let slot = cache.unwrap_or(&mut scratch);
        if slot.as_ref().is_none_or(|(id, _)| *id != map.id) {
            *slot = Some((
                map.id,
                darkener.make_group(&gpu.device, targets.1, &self.params, &map.map, &self.sampler),
            ));
        }
        if let Some((_, group)) = slot.as_ref() {
            darkener.lay_with(encoder, (targets.0, mask), group);
        }
    }

    /// Draws the lights' maps of `plan` that their tiles don't hold as
    /// they are already (each creature's in the tile of its place in the
    /// plan, kept from frame to frame): how many it drew.
    pub(crate) fn cast_lights(
        &mut self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        plan: &LightPlan,
    ) -> usize {
        let maps = self.lights.get_or_insert_with(|| LightMaps::new(&gpu.device));
        maps.hold(&gpu.device, plan.lights.len());
        let stale: Vec<usize> = (0..plan.lights.len()).filter(|&k| !maps.holds(k, plan)).collect();
        if stale.is_empty() {
            return 0;
        }
        maps.casting.upload(gpu, &plan.casters, &plan.bones, &plan.lights);
        // (Cleared whole where every tile is drawn again; else each tile
        // drawn is cleared alone, the others kept.)
        let whole = stale.len() == plan.lights.len();
        if whole {
            // (Every tile cleared: those past this plan's too.)
            maps.held.iter_mut().for_each(|h| *h = None);
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("light shadow maps"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &maps.atlas,
                depth_ops: Some(wgpu::Operations {
                    load: if whole { wgpu::LoadOp::Clear(1.0) } else { wgpu::LoadOp::Load },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        for &k in &stale {
            let (x, y, side) = maps.tile(k);
            pass.set_viewport(x as f32, y as f32, side as f32, side as f32, 0.0, 1.0);
            pass.set_scissor_rect(x, y, side, side);
            if !whole {
                pass.set_pipeline(&maps.clear);
                pass.draw(0..3, 0..1);
            }
            let range = &plan.ranges[k];
            let casters = &plan.casters[range.start as usize..range.end as usize];
            maps.casting.draw_in(&mut pass, k as u32, casters, range.start);
            maps.held[k] = Some(HeldTile::of(plan, k));
        }
        stale.len()
    }

    /// Lays the lights' shadows of `plan` over what is drawn, as
    /// [`ShadowMaps::darken`] the sun's.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn darken_lights(
        &mut self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&wgpu::TextureView, &wgpu::TextureView),
        mask: &Mask,
        format: (wgpu::TextureFormat, u32),
        plan: &LightPlan,
        camera: &Camera,
        size: (u32, u32),
        fog: Option<crate::scene::Fog>,
        cache: Option<&mut Option<(u64, wgpu::BindGroup)>>,
    ) {
        let Some(maps) = &mut self.lights else { return };
        let (across, down) = (size.0.max(1).div_ceil(BIN), size.1.max(1).div_ceil(BIN));
        let count = [plan.lights.len() as u32, across, down, u32::from(plan.overflows())];
        let look = (plan.shadows, LIGHT_BIAS, count);
        let params = uniform(look, [[0.0; 4]; 4], Some(plan.share), camera, size, fog);
        gpu.queue.write_buffer(&maps.params, 0, bytemuck::bytes_of(&params));
        maps.upload_maps(gpu, plan, camera, size);
        let darkener = self
            .darkeners
            .entry((format.0, format.1, true))
            .or_insert_with(|| Darkener::new(&gpu.device, format.0, format.1, true));
        let mut scratch = None;
        let slot = cache.unwrap_or(&mut scratch);
        if slot.as_ref().is_none_or(|(id, _)| *id != maps.id) {
            let group = darkener.make_lights_group(&gpu.device, targets.1, maps, &self.sampler);
            *slot = Some((maps.id, group));
        }
        if let Some((_, group)) = slot.as_ref() {
            darkener.lay_with(encoder, (targets.0, mask), group);
        }
    }
}

/// The darkening pass's values: the shadows' look, the depth bias, how
/// many maps and their projections; the lights' shadows' share of the
/// light; the camera, the picture's size, the fog.
fn uniform(
    (shadows, bias, count): (SunShadows, f32, [u32; 4]),
    sun: [[f32; 4]; 4],
    share: Option<Share>,
    camera: &Camera,
    size: (u32, u32),
    fog: Option<crate::scene::Fog>,
) -> DarkenUniform {
    let aspect = size.0 as f32 / size.1.max(1) as f32;
    let view = camera.view();
    DarkenUniform {
        unproject: (camera.projection(aspect) * view).inverse().to_cols_array_2d(),
        view: view.to_cols_array_2d(),
        screen: [
            size.0.max(1) as f32,
            size.1.max(1) as f32,
            camera_fade_point(camera, shadows.fade_limit),
            shadows.opacity.clamp(0.0, 1.0),
        ],
        fog: fog.map_or([0.0; 4], |f| [1.0, f.start, f.end, 1.0 / (f.end - f.start).max(1e-3)]),
        bias: [bias, 0.0, 0.0, 0.0],
        count,
        sun,
        share: share.unwrap_or_else(bytemuck::Zeroable::zeroed),
    }
}

/// Numbers that tell one set of the lights' maps' resources from another
/// (the darkening's bind group is made again when they change).
static NEXT_LIGHTS_ID: AtomicU64 = AtomicU64::new(1);

/// The lights' shadow maps: one atlas, a square tile a creature, grown
/// as more creatures cast (never shrunk); what each tile holds (drawn
/// again only when that changes); what draws into them; the darkening
/// pass's values, the maps' projections and tiles, and which of them reach
/// each square of the screen.
#[derive(Debug)]
struct LightMaps {
    /// Changed whenever the atlas or a buffer the darkening reads is made
    /// again.
    id: u64,
    atlas: wgpu::TextureView,
    /// Tiles across (and down) the atlas, and a tile's side, texels.
    across: u32,
    side: u32,
    held: Vec<Option<HeldTile>>,
    casting: Casting,
    /// Clears a tile (its depth to the farthest) before it is drawn again.
    clear: wgpu::RenderPipeline,
    params: wgpu::Buffer,
    maps: wgpu::Buffer,
    bins: wgpu::Buffer,
    map_scratch: Vec<MapData>,
    bin_scratch: Vec<u32>,
}

/// What a light's map holds: the light's projection, its casters (mesh,
/// transform, first bone counted from the creature's first) and their
/// bones.
#[derive(Debug, PartialEq)]
struct HeldTile {
    light: Mat4,
    casters: Vec<((u64, usize), Mat4, Option<u32>)>,
    bones: Vec<Mat4>,
}

impl HeldTile {
    /// What creature `k` of `plan` casts into its map.
    fn of(plan: &LightPlan, k: usize) -> HeldTile {
        let (range, bones) = (&plan.ranges[k], &plan.bone_ranges[k]);
        let casters = &plan.casters[range.start as usize..range.end as usize];
        HeldTile {
            light: plan.lights[k],
            casters: casters.iter().map(|c| HeldTile::key(c, bones.start)).collect(),
            bones: plan.bones[bones.start as usize..bones.end as usize].to_vec(),
        }
    }

    /// A caster's key, its bones counted from `first`.
    fn key(c: &Caster, first: u32) -> ((u64, usize), Mat4, Option<u32>) {
        let (mesh, world, bones) = c.key();
        (mesh, world, bones.map(|b| b - first))
    }

    /// Whether it holds what creature `k` of `plan` casts.
    fn holds(&self, plan: &LightPlan, k: usize) -> bool {
        let (range, bones) = (&plan.ranges[k], &plan.bone_ranges[k]);
        let casters = &plan.casters[range.start as usize..range.end as usize];
        self.light == plan.lights[k]
            && self.casters.len() == casters.len()
            && self.casters.iter().zip(casters).all(|(h, c)| *h == HeldTile::key(c, bones.start))
            && self.bones[..] == plan.bones[bones.start as usize..bones.end as usize]
    }
}

/// A light's map as the darkening pass reads it ([`darken_source`]).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MapData {
    /// From the world to the map's clip space.
    m: [[f32; 4]; 4],
    /// Its tile's corner and side in the atlas (0 to 1), and half a texel.
    tile: [f32; 4],
    /// Its light's place in the share's lights.
    source: [u32; 4],
}

/// How many tiles across the atlas holds `n` maps in, and their side
/// (texels): at least 4 across (16 maps of [`LIGHT_MAP_SIZE`]), as many
/// across as a power of two holds them, the tiles smaller where the atlas
/// would pass [`LIGHT_ATLAS_SIZE`].
fn atlas_layout(n: usize) -> (u32, u32) {
    let most = LIGHT_ATLAS_SIZE / LEAST_LIGHT_MAP;
    let mut across = 4u32;
    while (across * across) < n as u32 && across < most {
        across *= 2;
    }
    (across, LIGHT_MAP_SIZE.min(LIGHT_ATLAS_SIZE / across))
}

/// Which of the lights' maps (each from the world to its clip space) can
/// darken each square of the screen ([`BIN`] pixels a side) of a picture of
/// `size` seen by `camera`: its light's view's box, cut off below `floor`
/// (nothing that receives lies there: most of a lamp's view of a creature
/// runs into the ground), seen on the screen; every square where part of
/// it is behind the eye. Written to `out` as the darkening pass reads it:
/// per square, where its maps start in the list (and one more, the end),
/// then the list.
pub(crate) fn bins(
    lights: &[Mat4],
    floor: f32,
    camera: &Camera,
    size: (u32, u32),
    out: &mut Vec<u32>,
) {
    let (w, h) = (size.0.max(1), size.1.max(1));
    let (across, down) = (w.div_ceil(BIN), h.div_ceil(BIN));
    let n = (across * down) as usize;
    let to_screen = camera.projection(w as f32 / h as f32) * camera.view();
    let all = Some([0, 0, across - 1, down - 1]);
    let rects: Vec<Option<[u32; 4]>> = lights
        .iter()
        .map(|light| {
            let to_world = light.inverse();
            let mut corners = [Vec3::ZERO; 8];
            for (corner, at) in corners.iter_mut().enumerate() {
                let x = if corner & 1 == 0 { -1.0 } else { 1.0 };
                let y = if corner & 2 == 0 { -1.0 } else { 1.0 };
                let z = if corner & 4 == 0 { 0.0 } else { 1.0 };
                let p = to_world * glam::Vec4::new(x, y, z, 1.0);
                if p.w.abs() < 1e-9 || !p.is_finite() {
                    return all;
                }
                *at = p.truncate() / p.w;
            }
            // (The box cut at the floor: its corners above it, and where
            // its edges cross it.)
            let mut kept = Vec::with_capacity(12);
            kept.extend(corners.iter().copied().filter(|p| p.z >= floor));
            for a in 0..8 {
                for bit in [1, 2, 4] {
                    let b = a | bit;
                    let (p, q) = (corners[a], corners[b]);
                    if b != a && (p.z < floor) != (q.z < floor) {
                        kept.push(p.lerp(q, (floor - p.z) / (q.z - p.z)));
                    }
                }
            }
            if kept.is_empty() {
                return None;
            }
            let (mut lo, mut hi) = (glam::Vec2::splat(f32::MAX), glam::Vec2::splat(f32::MIN));
            for p in kept {
                let c = to_screen * p.extend(1.0);
                if c.w <= 1e-4 || !c.is_finite() {
                    return all;
                }
                let ndc = glam::Vec2::new(c.x, c.y) / c.w;
                (lo, hi) = (lo.min(ndc), hi.max(ndc));
            }
            if hi.x < -1.0 || lo.x > 1.0 || hi.y < -1.0 || lo.y > 1.0 {
                return None;
            }
            // (In pixels from the top left, then squares.)
            let px = |x: f32| ((x + 1.0) * 0.5 * w as f32).clamp(0.0, w as f32 - 1.0) as u32;
            let py = |y: f32| ((1.0 - y) * 0.5 * h as f32).clamp(0.0, h as f32 - 1.0) as u32;
            Some([px(lo.x) / BIN, py(hi.y) / BIN, px(hi.x) / BIN, py(lo.y) / BIN])
        })
        .collect();
    out.clear();
    out.resize(n + 1, 0);
    for r in rects.iter().flatten() {
        for y in r[1]..=r[3] {
            for x in r[0]..=r[2] {
                out[(y * across + x) as usize + 1] += 1;
            }
        }
    }
    for b in 1..=n {
        out[b] += out[b - 1];
    }
    let mut cursor: Vec<u32> = out[..n].to_vec();
    out.resize(n + 1 + out[n] as usize, 0);
    for (k, r) in rects.iter().enumerate() {
        let Some(r) = r else { continue };
        for y in r[1]..=r[3] {
            for x in r[0]..=r[2] {
                let b = (y * across + x) as usize;
                out[n + 1 + cursor[b] as usize] = k as u32;
                cursor[b] += 1;
            }
        }
    }
}

/// The pass that clears a light's map's tile: a triangle over it at the
/// farthest depth.
const CLEAR_TILE: &str = "
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 1.0, 1.0);
}
";

impl LightMaps {
    fn new(device: &wgpu::Device) -> LightMaps {
        let (across, side) = atlas_layout(0);
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("light shadow map tile"),
            source: wgpu::ShaderSource::Wgsl(CLEAR_TILE.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("light shadow map tile"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let clear = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("light shadow map tile"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: MAP_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let storage = wgpu::BufferUsages::STORAGE;
        LightMaps {
            id: NEXT_LIGHTS_ID.fetch_add(1, Ordering::Relaxed),
            atlas: LightMaps::atlas(device, across * side),
            across,
            side,
            held: Vec::new(),
            casting: Casting::new(device, None),
            clear,
            params: buffer(
                "light shadow darkening",
                std::mem::size_of::<DarkenUniform>() as u64,
                wgpu::BufferUsages::UNIFORM,
            ),
            maps: buffer("light shadow maps' views", 96 * 16, storage),
            bins: buffer("light shadow maps' squares", 4 * 4096, storage),
            map_scratch: Vec::new(),
            bin_scratch: Vec::new(),
        }
    }

    /// The atlas, `size` texels a side.
    fn atlas(device: &wgpu::Device, size: u32) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("light shadow maps"),
                size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: MAP_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    /// Room for `n` maps: the atlas made again, larger (what it held
    /// lost), where it has less.
    fn hold(&mut self, device: &wgpu::Device, n: usize) {
        let (across, side) = atlas_layout(n);
        if across > self.across {
            (self.across, self.side) = (across, side);
            self.atlas = LightMaps::atlas(device, across * side);
            self.held.clear();
            self.id = NEXT_LIGHTS_ID.fetch_add(1, Ordering::Relaxed);
        }
        let tiles = (self.across * self.across) as usize;
        self.held.resize_with(tiles, || None);
    }

    /// Whether tile `k` holds what creature `k` of `plan` casts.
    fn holds(&self, k: usize, plan: &LightPlan) -> bool {
        self.held.get(k).and_then(Option::as_ref).is_some_and(|h| h.holds(plan, k))
    }

    /// Tile `k`'s corner and side in the atlas, texels.
    fn tile(&self, k: usize) -> (u32, u32, u32) {
        let k = k as u32;
        ((k % self.across) * self.side, (k / self.across) * self.side, self.side)
    }

    /// Uploads `plan`'s maps' views, tiles and lights, and which reach each
    /// square of the screen ([`bins`]) of a picture of `size` seen by
    /// `camera`.
    fn upload_maps(
        &mut self,
        gpu: &crate::Gpu,
        plan: &LightPlan,
        camera: &Camera,
        size: (u32, u32),
    ) {
        let atlas = (self.across * self.side) as f32;
        self.map_scratch.clear();
        for (k, light) in plan.lights.iter().enumerate() {
            let (x, y, side) = self.tile(k);
            self.map_scratch.push(MapData {
                m: light.to_cols_array_2d(),
                tile: [x as f32 / atlas, y as f32 / atlas, side as f32 / atlas, 0.5 / side as f32],
                source: [plan.share_index[k], 0, 0, 0],
            });
        }
        bins(&plan.lights, plan.floor, camera, size, &mut self.bin_scratch);
        let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let least = std::mem::size_of::<MapData>();
        let maps = &self.map_scratch;
        let mut grown = crate::renderer::upload(
            gpu,
            &mut self.maps,
            "light shadow maps' views",
            usage,
            maps,
            least,
        );
        grown |= crate::renderer::upload(
            gpu,
            &mut self.bins,
            "light shadow maps' squares",
            usage,
            &self.bin_scratch,
            4,
        );
        if grown {
            self.id = NEXT_LIGHTS_ID.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Casting {
    /// The casters' pipelines, culling the faces `cull` names as the
    /// light sees them: the sun's and moon's map culls those turned from
    /// the light, as the client casts from a face's front only
    /// (`docs/findings.md`, "A face casts the sun's shadow from its front
    /// only": a single wall with its back to a low sun casts nothing); the
    /// lights' maps cull none (not read).
    pub(crate) fn new(device: &wgpu::Device, cull: Option<wgpu::Face>) -> Casting {
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
        let min_light_size = std::num::NonZeroU64::new(std::mem::size_of::<Mat4>() as u64);
        let light_stride = device
            .limits()
            .min_uniform_buffer_offset_alignment
            .max(std::mem::size_of::<Mat4>() as u32);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow casters"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: min_light_size,
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
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: cull,
                    ..Default::default()
                },
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
        let lights_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow lights"),
            size: (MAX_LIGHT_SHADOWS.max(1) as u64) * light_stride as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Casting {
            rigid,
            skinned,
            layout,
            light_stride,
            lights_buf,
            lights_staging: Vec::new(),
            caster_scratch: Vec::new(),
            casters: buffer("shadow casters", 80 * 256),
            bones: buffer("shadow bones", 64 * 64),
            group: None,
        }
    }

    /// Uploads `casters`, their `bones`, and the pass `lights` (aligned to
    /// `light_stride` for dynamic offsets), rebuilding the bind group only
    /// when a buffer grows.
    pub(crate) fn upload(
        &mut self,
        gpu: &crate::Gpu,
        casters: &[Caster<'_>],
        bones: &[Mat4],
        lights: &[Mat4],
    ) {
        self.caster_scratch.clear();
        self.caster_scratch.extend(casters.iter().map(|c| CasterData {
            world: c.world.to_cols_array_2d(),
            bones: [c.bones.unwrap_or(0), 0, 0, 0],
        }));
        let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let mut dirty = self.group.is_none();
        dirty |= crate::renderer::upload(
            gpu,
            &mut self.casters,
            "shadow casters",
            usage,
            &self.caster_scratch,
            0,
        );
        dirty |= crate::renderer::upload(gpu, &mut self.bones, "shadow bones", usage, bones, 0);

        let stride = self.light_stride as usize;
        let bytes = lights.len().max(1) * stride;
        self.lights_staging.resize(bytes, 0);
        for (k, light) in lights.iter().enumerate() {
            self.lights_staging[k * stride..k * stride + std::mem::size_of::<Mat4>()]
                .copy_from_slice(bytemuck::bytes_of(light));
        }
        dirty |= crate::renderer::upload(
            gpu,
            &mut self.lights_buf,
            "shadow lights",
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            &self.lights_staging[..bytes],
            bytes,
        );
        if dirty {
            self.group = Some(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow casters"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.lights_buf,
                            offset: 0,
                            size: std::num::NonZeroU64::new(std::mem::size_of::<Mat4>() as u64),
                        }),
                    },
                    wgpu::BindGroupEntry { binding: 1, resource: self.casters.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: self.bones.as_entire_binding() },
                ],
            }));
        }
    }

    /// Draws `casters` (in [`in_drawing_order`]; the uploaded ones from
    /// `first`) into `depth`, cleared first where `clear`, using light `light_index`
    /// from the uploaded `lights_buf`.
    pub(crate) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        light_index: u32,
        casters: &[Caster<'_>],
        first: u32,
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
        self.draw_in(&mut pass, light_index, casters, first);
    }

    /// [`Casting::draw`] in a pass begun: into its viewport.
    pub(crate) fn draw_in(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        light_index: u32,
        casters: &[Caster<'_>],
        first: u32,
    ) {
        let group = self.group.as_ref().expect("casters uploaded before draw");
        pass.set_bind_group(0, group, &[light_index * self.light_stride]);
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
            let index = c.mesh.first_index;
            let instances = first + i as u32..first + (i + n) as u32;
            pass.draw_indexed(index..index + c.mesh.index_count, c.mesh.base_vertex, instances);
            i += n;
        }
    }
}

impl Darkener {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        samples: u32,
        lights: bool,
    ) -> Darkener {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow darkening"),
            source: wgpu::ShaderSource::Wgsl(darken_source(samples, lights, false).into()),
        });
        let storage = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let extra: &[wgpu::BindGroupLayoutEntry] =
            if lights { &[storage(4), storage(5)] } else { &[] };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow darkening"),
            entries: &[
                &[
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
                            sample_type: crate::framebuffer::DEPTH_READ,
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
                extra,
            ]
            .concat(),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow darkening"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let blend = DARKEN_BLEND;
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
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(MASK_TARGET),
                ],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: samples, ..Default::default() },
            multiview_mask: None,
            cache: None,
        });
        Darkener { pipeline, layout }
    }

    /// Builds the darkening bind group sampling `depth`, `params`, and `maps`.
    fn make_group(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::TextureView,
        params: &wgpu::Buffer,
        maps: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow darkening"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(maps),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// The lights' darkening's bind group: `depth`, and `maps`' values,
    /// atlas, views and squares.
    fn make_lights_group(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::TextureView,
        maps: &LightMaps,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("light shadow darkening"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: maps.params.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&maps.atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry { binding: 4, resource: maps.maps.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: maps.bins.as_entire_binding() },
            ],
        })
    }

    /// Lays the shadows over what is drawn in `target` using `group`.
    fn lay_with(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        (target, mask): (&wgpu::TextureView, &Mask),
        group: &wgpu::BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow darkening"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    depth_slice: None,
                }),
                Some(mask.attachment(false)),
            ],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plan for the shadow map, and one for the stencil volumes.
    const MAP: bool = false;
    const VOLUMES: bool = true;

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
        // (A flat receiver, a ground of one height, seen at a slant: the
        // rays cross its height but for the last bits.)
        let flat = (Vec3::new(-3.0, -3.0, -0.01), Vec3::new(3.0, 3.0, -0.01));
        let slant = Camera {
            eye: Vec3::new(2.13, 4.71, 1.83),
            target: Vec3::new(0.0, 0.0, 0.84),
            fov_y: 0.7,
            near: 0.05,
            far: 200.0,
        };
        let (min, max) = seen(&slant, 1.4, flat).unwrap();
        assert!(min.z == -0.01 && max.z == -0.01 && max.x > min.x + 1.0, "{min} {max}");
    }

    /// A creature casts its lights' shadow from the light nearest its
    /// place that casts shadows, reaches it and stands outside it, as the
    /// client (of two lamps, the nearer); a tile's flame (radius 7) casts
    /// none.
    #[test]
    fn a_creature_casts_from_the_nearest_light_that_casts() {
        let lamp = |x: f32, radius: f32| {
            PointLight::new(Vec3::new(x, 3.0, 4.0), Vec3::ONE, radius, false, 4)
                .casting(true, radius, 3.0)
        };
        let at = Vec3::new(0.0, 0.0, 1.0);
        let feet = Vec3::ZERO;
        let lights = [lamp(-2.6, 10.0), lamp(3.0, 10.0), lamp(1.0, 7.0)];
        assert!(!lights[2].casts_shadows);
        assert_eq!(casting_light(&lights, at, 1.0, feet), Some(0));
        // (Too far for its light, or inside the creature: none.)
        let far = Vec3::new(40.0, 0.0, 0.0);
        assert_eq!(casting_light(&lights[1..2], far, 1.0, far), None);
        assert_eq!(casting_light(&lights[1..2], lights[1].position, 1.0, feet), None);
        // An ambient light casts none.
        let ambient = PointLight { ambient_only: true, ..lights[0] };
        assert_eq!(casting_light(&[ambient], at, 1.0, feet), None);
        // Nearly as near: the nearer to its place, not to its box's
        // middle (which a creature facing aside carries off its feet).
        let pair = [lamp(-2.504, 10.0), lamp(2.496, 10.0)];
        let aside = Vec3::new(-0.2, 0.0, 1.0);
        assert_eq!(casting_light(&pair, aside, 1.0, feet), Some(1));
    }

    /// A light's view holds the creature and what lies behind it: the
    /// ball's middle and its edge are in sight, the light's side of it is
    /// not cut off, and a point behind it, on the ground, is in the map.
    #[test]
    fn a_light_s_view_holds_the_creature_and_its_shadow() {
        let (light, at, radius) = (Vec3::new(0.0, 3.0, 6.9), Vec3::new(0.0, 0.0, 1.0), 1.2);
        let m = light_projection(light, at, radius, 20.0);
        let inside = |p: Vec3| {
            let c = m * p.extend(1.0);
            let n = c.truncate() / c.w;
            c.w > 0.0 && n.x.abs() <= 1.0 && n.y.abs() <= 1.0 && (0.0..=1.0).contains(&n.z)
        };
        assert!(inside(at) && inside(at + Vec3::X * radius) && inside(at - Vec3::Z * radius));
        // (Its shadow on the ground, beyond it from the light.)
        let ground = at + (at - light) * (1.0 / (light.z - at.z));
        assert!(inside(ground.with_z(0.0)), "{ground}");
        // Nothing beside it.
        assert!(!inside(at + Vec3::X * 5.0));
    }

    /// The plans carry the bones of the skinned meshes that are drawn into
    /// a map: not those of a creature out of the sun's map's region (its
    /// animation changed them every frame, and the map was drawn again
    /// each frame with a still camera), nor of one no light casts for, nor
    /// any where skins cast nothing (the stencil volumes).
    #[test]
    fn the_plans_carry_the_bones_of_what_is_drawn_alone() {
        let Some(gpu) = mg_testkit::gpu::open(crate::Gpu::headless) else { return };
        let model = |m| std::sync::Arc::new(crate::GpuModel::new(&gpu, std::sync::Arc::new(m)));
        let cast = |m: mg_mdl::Model| {
            let mut m = m;
            for n in &mut m.nodes {
                if let mg_mdl::NodeKind::Mesh(mesh) = &mut n.kind {
                    mesh.shadow = true;
                }
            }
            m
        };
        let ground = model(crate::probe::quad("ground", 20.0, 0.0, [1.0; 3], false));
        let skin = model(cast(crate::probe::quad("skinned", 1.0, 1.0, [1.0; 3], true)));
        let rigid = model(cast(crate::probe::quad("rigid", 1.0, 1.0, [1.0; 3], false)));
        assert!(skin.meshes[0].skin.is_some() && skin.meshes[0].casts);
        let at = |x: f32| Mat4::from_translation(Vec3::new(x, 0.0, 0.0));
        let creature = |m: &std::sync::Arc<crate::GpuModel>, x: f32| Instance {
            casts_light_shadows: true,
            ..Instance::new(m.clone(), at(x))
        };
        let scene = |x: f32| Scene {
            instances: vec![
                Instance { receives_shadows: true, ..Instance::new(ground.clone(), at(0.0)) },
                creature(&rigid, 0.0),
                creature(&skin, x),
            ],
            area: crate::AreaLight {
                ambient: Vec3::splat(0.2),
                diffuse: Vec3::splat(0.6),
                direction: Vec3::Z,
            },
            shadows: Some(SunShadows { opacity: 0.5, fade_limit: 45.0 }),
            light_shadows: Some(SunShadows { opacity: 0.5, fade_limit: 45.0 }),
            lights: vec![
                PointLight::new(Vec3::new(3.0, 0.0, 3.0), Vec3::ONE, 10.0, false, 4)
                    .casting(true, 10.0, 0.0),
            ],
            ..Default::default()
        };
        // (Looking down on the ground's middle: 5 m of it seen.)
        let camera = Camera {
            eye: Vec3::new(0.0, -0.01, 6.0),
            target: Vec3::ZERO,
            fov_y: 0.8,
            near: 0.1,
            far: 100.0,
        };
        // In the region: both cast, the skin with its bone.
        let near = scene(1.0);
        let sun = plan(&near, &camera, 1.0, MAP).expect("a plan");
        assert_eq!((sun.casters.len(), sun.bones.len()), (2, 1));
        assert_eq!(sun.casters.iter().filter(|c| c.bones.is_some()).count(), 1);
        // The skin out of the region: the rigid one alone, and no bones.
        let far = scene(60.0);
        let sun = plan(&far, &camera, 1.0, MAP).expect("a plan");
        assert_eq!((sun.casters.len(), sun.bones.len()), (1, 0));
        // Skins casting nothing (the stencil volumes): none planned.
        let sun = plan(&near, &camera, 1.0, VOLUMES).expect("a plan");
        assert_eq!((sun.casters.len(), sun.bones.len()), (1, 0));
        // The lights': the skin within the lamp's reach casts, with its
        // bone; out of it, nothing of it is left; nor where skins cast
        // nothing.
        let lit = light_plan(&near, &camera, MAP).expect("a plan");
        assert_eq!((lit.lights.len(), lit.casters.len(), lit.bones.len()), (2, 2, 1));
        let lit = light_plan(&far, &camera, MAP).expect("a plan");
        assert_eq!((lit.lights.len(), lit.casters.len(), lit.bones.len()), (1, 1, 0));
        let lit = light_plan(&near, &camera, VOLUMES).expect("a plan");
        assert_eq!((lit.lights.len(), lit.casters.len(), lit.bones.len()), (1, 1, 0));
    }

    /// A tile or a static placeable whose place is no number (a corrupt
    /// float in an area's GIT) neither receives nor casts: the map is the
    /// same as without it, wherever it is among the instances (its box
    /// took every receiver's before it with it; last, it left the area no
    /// shadows at all).
    #[test]
    fn an_instance_that_is_nowhere_changes_no_shadow() {
        let Some(gpu) = mg_testkit::gpu::open(crate::Gpu::headless) else { return };
        let model = |m| std::sync::Arc::new(crate::GpuModel::new(&gpu, std::sync::Arc::new(m)));
        let mut caster = crate::probe::quad("rigid", 1.0, 1.0, [1.0; 3], false);
        for n in &mut caster.nodes {
            if let mg_mdl::NodeKind::Mesh(mesh) = &mut n.kind {
                mesh.shadow = true;
            }
        }
        let (ground, caster) =
            (model(crate::probe::quad("ground", 4.0, 0.0, [1.0; 3], false)), model(caster));
        let at = |x: f32| Mat4::from_translation(Vec3::new(x, 0.0, 0.0));
        let tile =
            |x: f32| Instance { receives_shadows: true, ..Instance::new(ground.clone(), at(x)) };
        let nowhere = Instance {
            receives_shadows: true,
            casts_light_shadows: true,
            ..Instance::new(caster.clone(), at(f32::NAN))
        };
        let scene = |instances: Vec<Instance>| Scene {
            instances,
            area: crate::AreaLight {
                ambient: Vec3::splat(0.2),
                diffuse: Vec3::splat(0.6),
                direction: Vec3::new(0.3, 0.2, 1.0),
            },
            shadows: Some(SunShadows { opacity: 0.5, fade_limit: 45.0 }),
            light_shadows: Some(SunShadows { opacity: 0.5, fade_limit: 45.0 }),
            lights: vec![
                PointLight::new(Vec3::new(3.0, 0.0, 3.0), Vec3::ONE, 10.0, false, 4)
                    .casting(true, 10.0, 0.0),
            ],
            ..Default::default()
        };
        let camera = Camera {
            eye: Vec3::new(0.0, -8.0, 10.0),
            target: Vec3::ZERO,
            fov_y: 0.8,
            near: 0.1,
            far: 100.0,
        };
        let good =
            || vec![tile(-4.0), tile(0.0), tile(4.0), Instance::new(caster.clone(), at(0.5))];
        let all_good = scene(good());
        let whole = plan(&all_good, &camera, 1.5, MAP).expect("a plan");
        for place in [0, 1, 2, 4] {
            let mut instances = good();
            instances.insert(place, nowhere.clone());
            let scene = scene(instances);
            let with = plan(&scene, &camera, 1.5, MAP).expect("a plan");
            assert_eq!(with.light, whole.light, "at {place}");
            assert_eq!((with.casters.len(), with.floor), (whole.casters.len(), whole.floor));
            // (Nor a light's shadow of it.)
            assert!(light_plan(&scene, &camera, MAP).is_none(), "at {place}");
        }
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

    #[test]
    fn share_of_keeps_the_nearest_32_lights() {
        let lights = (0..80)
            .map(|i| {
                // Deliberately reversed distances so selection is non-trivial.
                let x = (80 - i) as f32;
                PointLight::new(Vec3::new(x, 0.0, 0.0), Vec3::ONE, 100.0, false, 4)
            })
            .collect();
        let scene = Scene { lights, ..Default::default() };
        let (share, places) = Share::of(&scene, &[2, 2], Vec3::ZERO);
        assert_eq!(share.falloff[2] as usize, MAX_SHARE_LIGHTS);
        // (One light casting, both maps': the first.)
        assert_eq!((share.falloff[3], places), (1.0, vec![0, 0]));
        assert_eq!(share.lights[0][0][0], 78.0, "explicit source comes first");
        // Closest remaining lights are x = 1.0 .. 31.0 (31 lights).
        for (k, light) in share.lights[..MAX_SHARE_LIGHTS].iter().enumerate().skip(1) {
            assert_eq!(light[0][0], k as f32, "slot {k}");
        }
        // More lights casting than the share holds: those past it unknown.
        let sources: Vec<usize> = (0..40).collect();
        let (share, places) = Share::of(&scene, &sources, Vec3::ZERO);
        assert_eq!(share.falloff[3] as usize, MAX_SHARE_LIGHTS);
        assert_eq!(places[31], 31);
        assert!(places[32..].iter().all(|&p| p == u32::MAX));
    }

    /// The atlas holds 16 maps of 512 texels at least, grows to 64 of them,
    /// then makes them smaller, never past its greatest side; its least
    /// maps hold [`MAX_LIGHT_SHADOWS`].
    #[test]
    fn the_atlas_grows_then_its_maps_shrink() {
        assert_eq!(atlas_layout(0), (4, 512));
        assert_eq!(atlas_layout(16), (4, 512));
        assert_eq!(atlas_layout(17), (8, 512));
        assert_eq!(atlas_layout(64), (8, 512));
        assert_eq!(atlas_layout(65), (16, 256));
        assert_eq!(atlas_layout(512), (32, 128));
        assert_eq!(atlas_layout(MAX_LIGHT_SHADOWS), (64, LEAST_LIGHT_MAP));
        assert_eq!(atlas_layout(MAX_LIGHT_SHADOWS + 1), (64, LEAST_LIGHT_MAP));
        for n in [1, 17, 100, 512, 4096] {
            let (across, side) = atlas_layout(n);
            assert!(across * side <= LIGHT_ATLAS_SIZE && (across * across) as usize >= n.min(4096));
        }
    }

    /// Each square of the screen lists the maps whose light's view it sees
    /// part of, and only those: a map's own pixels all fall in squares
    /// that list it; a map out of sight is in none; one behind the eye in
    /// every one. Cut at the floor, a map's view is in fewer squares, and
    /// still in those of its pixels on the floor.
    #[test]
    fn the_squares_list_the_maps_that_reach_them() {
        let camera = Camera {
            eye: Vec3::new(0.0, -10.0, 12.0),
            target: Vec3::ZERO,
            fov_y: 0.8,
            near: 0.1,
            far: 200.0,
        };
        let size: (u32, u32) = (640, 360);
        let lamp = |x: f32, y: f32| {
            let light = Vec3::new(x, y, 5.0);
            light_projection(light, Vec3::new(x + 1.0, y, 1.0), 1.0, 13.0)
        };
        // (Two in sight, one far out of it, one about the eye.)
        let lights = [
            lamp(-3.0, 0.0),
            lamp(3.0, 1.0),
            lamp(500.0, 0.0),
            light_projection(camera.eye + Vec3::Z, camera.eye, 1.0, 13.0),
        ];
        let (across, down) = (size.0.div_ceil(BIN), size.1.div_ceil(BIN));
        let n = (across * down) as usize;
        let mut uncut = Vec::new();
        bins(&lights, f32::NEG_INFINITY, &camera, size, &mut uncut);
        let mut out = Vec::new();
        bins(&lights, 0.0, &camera, size, &mut out);
        let listed = |b: usize| out[n + 1 + out[b] as usize..n + 1 + out[b + 1] as usize].to_vec();
        assert!(out.len() < uncut.len(), "{} {}", out.len(), uncut.len());
        assert!((0..n).all(|b| listed(b).contains(&3)), "behind the eye: everywhere");
        assert!((0..n).all(|b| !listed(b).contains(&2)), "out of sight: nowhere");
        let to_screen = camera.projection(size.0 as f32 / size.1 as f32) * camera.view();
        for (k, light) in lights.iter().enumerate().take(2) {
            // (Points of the floor in the map's view: in its squares.)
            let mut seen = 0;
            for i in 0..80 {
                for j in 0..80 {
                    let p = Vec3::new(-10.0 + i as f32 * 0.25, -10.0 + j as f32 * 0.25, 0.0);
                    let c = *light * p.extend(1.0);
                    let m = c.truncate() / c.w;
                    if c.w <= 0.0
                        || m.x.abs() > 1.0
                        || m.y.abs() > 1.0
                        || !(0.0..=1.0).contains(&m.z)
                    {
                        continue;
                    }
                    let s = to_screen * p.extend(1.0);
                    let (x, y) = ((s.x / s.w + 1.0) * 0.5, (1.0 - s.y / s.w) * 0.5);
                    if !(0.0..1.0).contains(&x) || !(0.0..1.0).contains(&y) {
                        continue;
                    }
                    let (bx, by) =
                        ((x * size.0 as f32) as u32 / BIN, (y * size.1 as f32) as u32 / BIN);
                    assert!(
                        listed((by * across + bx) as usize).contains(&(k as u32)),
                        "map {k} at {p}"
                    );
                    seen += 1;
                }
            }
            assert!(seen > 10, "map {k}: {seen}");
            // (And not every square: it is a few metres across.)
            assert!((0..n).filter(|&b| listed(b).contains(&(k as u32))).count() < n / 2);
        }
    }
}
