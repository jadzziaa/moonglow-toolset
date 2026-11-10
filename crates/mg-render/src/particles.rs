//! Particle emitters, simulated on the CPU and drawn as billboards. What
//! the wiki's emitter page and the game's models say:
//!
//! - Fountain emitters spawn `birthrate` particles a second from a
//!   `xsize` × `ysize` cm rectangle, along the emitter's +Z within a cone of
//!   `spread` radians, at `velocity` plus up to `randvel` m/s (a `trail`
//!   spawntype: `birthrate` a metre the emitter moves); Single ones keep one
//!   particle; Explosion ones spawn `birthrate` at once when their
//!   `detonate` key passes.
//! - Point-to-point emitters (`p2p`) send particles to their reference
//!   child: Bezier ones (`p2p_sel` 1) along a curve with `p2p_bezier2`/`3`
//!   tangents on Z, taking `combinetime` seconds (else their life); Gravity
//!   ones (`p2p_sel` 0) pulled at `grav` m/s², keeping `drag` of their speed
//!   a second (1: all, so they overshoot and swing back), gone within
//!   `threshold` of the target.
//! - Lightning emitters draw a jagged bolt to their reference child, struck
//!   anew every `lightningdelay` seconds: `lightningsubdiv` (else
//!   `birthrate`) halvings, each bend up to `lightningscale` × a fifth of its
//!   segment, the end within `lightningradius` of the target.
//! - `Linked` particles are drawn as one strip through them in birth order.
//! - Chunk emitters throw models (`chunkName`) instead of quads, scaled by
//!   the size, each born facing as its emitter does and turning about the
//!   emitter's z at `particlerot` radians a second, anticlockwise seen from
//!   above (read in the client, `reading_chunk_spin`: a chunk a second
//!   at 0.25 rose as a stair turning 14° a step, a still one turned 14° a
//!   second; [`Particles::chunks`]). An emitter turned off the upright was
//!   not read.
//! - Particles live `lifeexp` seconds, fall with `mass` (× 9.8 m/s²; negative
//!   rises), spin `particlerot` radians a second (read in the client:
//!   `reading_particle_spin_and_order`), and go from `colorstart`,
//!   `alphastart` and `sizestart` (metres) to the end values. Textures are
//!   `xgrid` × `ygrid` flip-books played from `framestart` to `frameend` at
//!   `fps` (from a random frame with the `random` flag).
//! - `inherit`/`inherit_local` particles move with the emitter; others stay
//!   where they were born, but a looping Single emitter's one particle,
//!   which never dies and stays at its emitter.
//! - `bounce` particles bounce off the ground ([`Particles::ground`]),
//!   keeping 0.8 of their speed along it and 0.8 × `bounce_co` of their
//!   speed off it, until they come to rest.
//! - `m_isTinted` particles take the light at their emitter
//!   ([`ParticleBatch::tint`]).
//!
//! Measured in the client (`client_render.rs`, `particles_look`): the
//! gravity, bouncing, tinting, and that the three-stop values (`colorMid`,
//! `alphaMid`, `sizeMid`, `percentStart`/`Mid`/`End`) change nothing the
//! client draws, so they are not used; nor does `twosidedtex` (both sides
//! of aligned particles show), but on a `Billboard_to_Local_Z` emitter,
//! whose particles show only their front without it
//! (`local_z_particles_lie_as_the_client_lays_them`). The drag and Bezier details and the
//! lightning shape are guesses from the wiki's descriptions; wind
//! (`affectedByWind`), `splat`, `deadspace`, `inheritvel` and
//! `inherit_part` are not simulated.

use glam::{Mat4, Quat, Vec2, Vec3};
use mg_mdl::{AnimNode, Animation, Model, NodeKind};

use crate::anim::sample;

/// How a batch of particles blends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleBlend {
    /// Alpha blending, an emitter's particles in the order they were
    /// born (as the client draws them).
    Normal,
    /// Added to what is behind ("Lighten"): black is transparent.
    Lighten,
    /// Cut out at half alpha.
    PunchThrough,
}

/// A particle vertex, in world space.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleVertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    /// Gamma-space colour and alpha.
    pub color: [f32; 4],
    /// Its draw's softness, where Moonglow's own shader fades it where it
    /// meets what is behind (the batch's [`ParticleBatch::softness`], set
    /// by the renderer as it draws; 0: not faded).
    pub softness: f32,
}

impl ParticleVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ParticleVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32
        ],
    };
}

/// Quads (six vertices each) of one emitter, ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleBatch {
    pub texture: Option<String>,
    pub blend: ParticleBlend,
    pub render_order: u32,
    pub vertices: Vec<ParticleVertex>,
    /// For tinted emitters (`m_isTinted`), where the emitter is (world
    /// space): the renderer multiplies the colours by the scene's light
    /// there, the same for every particle.
    pub tint: Option<Vec3>,
    /// How sharply its particles fade where they meet what lies behind
    /// them (the game's `particleSoftness`, a metre's inverse): read from
    /// the client, 10 over the larger of the emitter's `sizeStart` and
    /// `sizeEnd` (a particle fades over its last tenth), at either shader
    /// quality; 0 for the emitters the client lays along an axis
    /// (`Billboard_to_Local_Z`, `Billboard_to_World_Z`) and, not read,
    /// lightning ([`softness`]).
    pub softness: f32,
}

/// [`ParticleBatch::softness`] for an emitter of these sizes and render
/// mode (`soft_particles_in_the_client`: 40 at 0.25 m, 20 at 0.5, 10 at
/// 1, 5 at 2; a growing or shrinking emitter's by its larger size).
pub fn softness(size_start: f32, size_end: f32, render: &str) -> f32 {
    soft(size_start, size_end, Render::of(render))
}

/// [`softness`], of a render mode read already.
fn soft(size_start: f32, size_end: f32, render: Render) -> f32 {
    let size = size_start.max(size_end);
    let laid = matches!(render, Render::BillboardToLocalZ | Render::BillboardToWorldZ);
    if laid || size <= 0.0 { 0.0 } else { 10.0 / size }
}

/// How an emitter makes its particles (`update`, any case).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Update {
    Fountain,
    Single,
    Explosion,
    Lightning,
    /// A word that is none of them: no particles.
    Other,
}

impl Update {
    fn of(word: &str) -> Update {
        match word.to_ascii_lowercase().as_str() {
            "fountain" => Update::Fountain,
            "single" => Update::Single,
            "explosion" => Update::Explosion,
            "lightning" => Update::Lightning,
            _ => Update::Other,
        }
    }
}

/// How an emitter's particles are drawn (`render`, any case).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Render {
    /// Facing the camera (`Normal`, and any word that is none of the
    /// others).
    Normal,
    Linked,
    MotionBlur,
    AlignedToWorldZ,
    BillboardToLocalZ,
    BillboardToWorldZ,
    AlignedToParticleDir,
}

impl Render {
    fn of(word: &str) -> Render {
        match word.to_ascii_lowercase().as_str() {
            "linked" => Render::Linked,
            "motion_blur" => Render::MotionBlur,
            "aligned_to_world_z" => Render::AlignedToWorldZ,
            "billboard_to_local_z" => Render::BillboardToLocalZ,
            "billboard_to_world_z" => Render::BillboardToWorldZ,
            "aligned_to_particle_dir" => Render::AlignedToParticleDir,
            _ => Render::Normal,
        }
    }
}

/// An emitter's blend (`blend`, any case).
fn blend_of(word: &str) -> ParticleBlend {
    match word.to_ascii_lowercase().as_str() {
        "lighten" => ParticleBlend::Lighten,
        "punch-through" | "punchthrough" => ParticleBlend::PunchThrough,
        _ => ParticleBlend::Normal,
    }
}

/// An emitter's parameters of one number, by their place in [`NAMES`]:
/// each one's rest value is found once, as the simulation is made
/// ([`EmitterSim::rest`]), not by its name thirty times an emitter a
/// frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum P {
    BounceCo,
    LifeExp,
    Mass,
    LightningDelay,
    LightningRadius,
    LightningSubdiv,
    Birthrate,
    LightningScale,
    Bezier2,
    Bezier3,
    CombineTime,
    Grav,
    Drag,
    Threshold,
    XSize,
    YSize,
    Spread,
    Velocity,
    RandVel,
    SizeStart,
    SizeEnd,
    AlphaStart,
    AlphaEnd,
    SizeStartY,
    SizeEndY,
    ParticleRot,
    BlurLength,
    Fps,
    FrameStart,
    FrameEnd,
}

/// The parameters' names, as a model and its animations have them.
const NAMES: [&str; 30] = [
    "bounce_co",
    "lifeexp",
    "mass",
    "lightningdelay",
    "lightningradius",
    "lightningsubdiv",
    "birthrate",
    "lightningscale",
    "p2p_bezier2",
    "p2p_bezier3",
    "combinetime",
    "grav",
    "drag",
    "threshold",
    "xsize",
    "ysize",
    "spread",
    "velocity",
    "randvel",
    "sizestart",
    "sizeend",
    "alphastart",
    "alphaend",
    "sizestart_y",
    "sizeend_y",
    "particlerot",
    "blurlength",
    "fps",
    "framestart",
    "frameend",
];

#[derive(Debug, Clone)]
struct Particle {
    /// World space, or the emitter's space for inheriting emitters.
    pos: Vec3,
    /// Where it was born (same space), for Bezier paths.
    origin: Vec3,
    vel: Vec3,
    age: f32,
    life: f32,
    frame0: f32,
}

#[derive(Debug, Clone)]
struct EmitterSim {
    node: usize,
    /// The reference child that point-to-point and lightning emitters aim at.
    target: Option<usize>,
    /// What the emitter's words say, read once.
    update: Update,
    render: Render,
    blend: ParticleBlend,
    /// A chunk emitter's model, lower case.
    chunk: Option<String>,
    /// Each parameter's value at rest (its first; `None`: the model has
    /// none), by its place in [`NAMES`]; and the start and end colors.
    rest: [Option<f32>; NAMES.len()],
    colors: [Option<Vec3>; 2],
    /// Where the emitter was at the last update (a looping Single
    /// emitter's particle goes with it).
    last_world: Option<Mat4>,
    particles: Vec<Particle>,
    /// Fractional particles owed.
    owed: f32,
    last_time: Option<f32>,
    /// World position at the last update (trail emitters).
    last_pos: Option<Vec3>,
    /// Lightning: the current bolt (world space) and its age.
    bolt: Vec<Vec3>,
    bolt_age: f32,
}

/// A chunk particle: a model to draw where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// The model (`chunkName`), lower case.
    pub model: String,
    pub transform: Mat4,
}

/// A point on a cubic Bezier curve.
fn bezier(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, u: f32) -> Vec3 {
    let v = 1.0 - u;
    p0 * (v * v * v) + p1 * (3.0 * v * v * u) + p2 * (3.0 * v * u * u) + p3 * (u * u * u)
}

/// A small, deterministic random source.
#[derive(Debug, Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

/// What a bounce keeps of a particle's speed (along the ground; off it,
/// times `bounce_co` too), measured in the client.
const BOUNCE_KEEP: f32 = 0.8;

/// The particles of one model instance.
#[derive(Debug, Clone)]
pub struct Particles {
    emitters: Vec<EmitterSim>,
    rng: Rng,
    /// The height `bounce` particles bounce at (world space): the ground
    /// under the object; 0 unless set.
    pub ground: f32,
}

/// An emitter's parameter now: keyed by the animation, else its rest value.
/// The animation's node for an emitter, and the time in it: looked up
/// once an emitter, not again for each of its parameters.
fn anim_node<'a>(
    model: &Model,
    node: usize,
    anim: Option<(&'a Animation, f32)>,
) -> Option<(&'a AnimNode, f32)> {
    let (a, t) = anim?;
    let name = &model.nodes[node].name;
    Some((a.nodes.iter().find(|x| x.name.eq_ignore_ascii_case(name))?, t))
}

/// A parameter now: keyed by the animation's node (`anim`, and the time
/// in it), else its rest value, else `default`.
fn param(e: &EmitterSim, anim: Option<(&AnimNode, f32)>, which: P, default: f32) -> f32 {
    param_opt(e, anim, which).unwrap_or(default)
}

/// [`param`], `None` where neither the animation nor the model has it (or
/// the animation's key has no value).
fn param_opt(e: &EmitterSim, anim: Option<(&AnimNode, f32)>, which: P) -> Option<f32> {
    if let Some((an, t)) = anim
        && let Some(c) = an.controllers.iter().find(|c| c.name == NAMES[which as usize])
    {
        return sample(c, t).first().copied();
    }
    e.rest[which as usize]
}

/// The emitter's start (`end` false) or end color now, else `default`.
fn color(e: &EmitterSim, anim: Option<(&AnimNode, f32)>, end: bool, default: Vec3) -> Vec3 {
    let name = if end { "colorend" } else { "colorstart" };
    if let Some((an, t)) = anim
        && let Some(c) = an.controllers.iter().find(|c| c.name == name)
    {
        return color3(Some(&sample(c, t))).unwrap_or(default);
    }
    e.colors[usize::from(end)].unwrap_or(default)
}

fn color3(v: Option<&[f32]>) -> Option<Vec3> {
    match v {
        Some([r, g, b, ..]) => Some(Vec3::new(*r, *g, *b)),
        _ => None,
    }
}

/// The most particles one emitter keeps.
const MAX_PARTICLES: usize = 2000;

/// The seed every simulation made with [`Particles::new`] starts from.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

impl Particles {
    /// The particles of `model` (the model every later call is given),
    /// none yet. Every simulation made so draws the same random numbers:
    /// for a picture that is the same each time. Emitters of one model
    /// that stand together are made with [`Particles::new_seeded`].
    pub fn new(model: &Model) -> Particles {
        Particles::with_rng(model, Rng(SEED))
    }

    /// [`Particles::new`], drawing its own random numbers, by `seed`:
    /// where many of one model run side by side (an area's torches, a
    /// row of fountains), each made with its own seed (its object's or
    /// its tile's number) moves by itself. With one seed, each flame of a
    /// row was a copy of the others, moved over. The same seed gives the
    /// same particles each time.
    pub fn new_seeded(model: &Model, seed: u64) -> Particles {
        // (SplitMix64 of the seed: seeds one apart share no bits. Never
        // zero, where the generator would stay.)
        let mut z = seed.wrapping_add(SEED);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Particles::with_rng(model, Rng((z ^ (z >> 31)) | 1))
    }

    fn with_rng(model: &Model, rng: Rng) -> Particles {
        fn emitter(n: &mg_mdl::Node) -> Option<&mg_mdl::Emitter> {
            match &n.kind {
                NodeKind::Emitter(em) => Some(em),
                _ => None,
            }
        }
        let emitters = model
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(node, n)| Some((node, n, emitter(n)?)))
            .map(|(node, n, em)| EmitterSim {
                node,
                target: n
                    .children
                    .iter()
                    .copied()
                    .find(|&c| matches!(model.nodes[c].kind, NodeKind::Reference(_))),
                update: Update::of(&em.update),
                render: Render::of(&em.render),
                blend: blend_of(&em.blend),
                chunk: em.chunk.as_ref().map(|c| c.to_ascii_lowercase()),
                rest: NAMES.map(|name| n.value(name).and_then(|v| v.first().copied())),
                colors: ["colorstart", "colorend"].map(|name| color3(n.value(name))),
                last_world: None,
                particles: Vec::new(),
                owed: 0.0,
                last_time: None,
                last_pos: None,
                bolt: Vec::new(),
                bolt_age: 0.0,
            })
            .collect();
        Particles { emitters, rng, ground: 0.0 }
    }

    pub fn is_empty(&self) -> bool {
        self.emitters.is_empty()
    }

    /// Whether anything is showing (particles or a bolt).
    pub fn live(&self) -> bool {
        self.emitters.iter().any(|e| !e.particles.is_empty() || !e.bolt.is_empty())
    }

    /// Advances by `dt` seconds; `time` is the animation's time (for keyed
    /// parameters and detonations), `pose` the model-space node transforms
    /// and `transform` the instance's.
    pub fn update(
        &mut self,
        model: &Model,
        anim: Option<&Animation>,
        time: f32,
        dt: f32,
        pose: &[Mat4],
        transform: Mat4,
    ) {
        let dt = dt.clamp(0.0, 0.25);
        let t = anim.map_or(time, |a| crate::anim::time_in(a, time));
        let at = anim.map(|a| (a, t));
        let ground = self.ground;
        for e in &mut self.emitters {
            let node = &model.nodes[e.node];
            let NodeKind::Emitter(em) = &node.kind else { continue };
            let at = anim_node(model, e.node, at);
            let (rest, update) = (e.rest, e.update);
            let p = |which: P, default: f32| -> f32 {
                if let Some((an, t)) = at
                    && let Some(c) = an.controllers.iter().find(|c| c.name == NAMES[which as usize])
                {
                    return sample(c, t).first().copied().unwrap_or(default);
                }
                rest[which as usize].unwrap_or(default)
            };
            let world = transform * pose.get(e.node).copied().unwrap_or(Mat4::IDENTITY);
            let inherit = em.flags & (0x40 | 0x100) != 0;
            // (A looping Single emitter's particle never dies: it goes
            // with its emitter where that moved, an object dragged or
            // turned in the area; it stayed where it was born until the
            // area was read again.)
            if let Some(last) = e.last_world.filter(|last| !inherit && *last != world) {
                let moved = world * last.inverse();
                for q in e.particles.iter_mut().filter(|q| q.life < 0.0) {
                    q.pos = moved.transform_point3(q.pos);
                    q.origin = moved.transform_point3(q.origin);
                    q.vel = moved.transform_vector3(q.vel);
                }
            }
            e.last_world = Some(world);
            let bounce = (em.flags & 0x10 != 0 && !inherit).then(|| p(P::BounceCo, 0.0));
            let life = p(P::LifeExp, 1.0);
            // (Down is the world's: particles kept in the emitter's own
            // space fall its way, not along the emitter's axis. A waterfall's
            // emitter points down the fall, and its water rose.)
            let down = Vec3::new(0.0, 0.0, -9.8 * p(P::Mass, 0.0));
            let gravity = if inherit { world.inverse().transform_vector3(down) } else { down };
            let target_world = e.target.map(|t| {
                transform.transform_point3(pose.get(t).map_or(Vec3::ZERO, |m| m.w_axis.truncate()))
            });
            let here = world.w_axis.truncate();

            // Lightning: a new bolt now and then.
            if update == Update::Lightning {
                e.bolt_age += dt;
                let delay = p(P::LightningDelay, 0.0).max(0.0);
                if let Some(target) = target_world
                    && (e.bolt.is_empty() || e.bolt_age >= delay)
                {
                    e.bolt_age = 0.0;
                    let rng = &mut self.rng;
                    let radius = p(P::LightningRadius, 0.0).max(0.0);
                    let end = target
                        + Vec3::new(
                            rng.range(-1.0, 1.0),
                            rng.range(-1.0, 1.0),
                            rng.range(-1.0, 1.0),
                        ) * radius;
                    let subdiv = param_opt(e, at, P::LightningSubdiv)
                        .unwrap_or_else(|| p(P::Birthrate, 0.0))
                        .clamp(0.0, 9.0) as u32;
                    let scale = p(P::LightningScale, 0.0).max(0.0);
                    let mut bolt = vec![here, end];
                    for _ in 0..subdiv.max(1) {
                        let mut next = Vec::with_capacity(bolt.len() * 2);
                        for w in bolt.windows(2) {
                            let (a, b) = (w[0], w[1]);
                            let len = a.distance(b);
                            let jitter = Vec3::new(
                                rng.range(-1.0, 1.0),
                                rng.range(-1.0, 1.0),
                                rng.range(-1.0, 1.0),
                            ) * (scale * len * 0.2);
                            next.push(a);
                            next.push((a + b) * 0.5 + jitter);
                        }
                        next.push(*bolt.last().expect("two points"));
                        bolt = next;
                    }
                    e.bolt = bolt;
                }
                e.last_time = Some(t);
                continue;
            }

            // Point to point: aim at the reference child.
            let p2p = em.flags & 0x1 != 0 && target_world.is_some();
            let bezier_path = em.flags & 0x2 != 0;
            let to_space = |w: Vec3| if inherit { world.inverse().transform_point3(w) } else { w };
            let target = target_world.map(to_space);
            let z_axis = if inherit {
                Vec3::Z
            } else {
                world.transform_vector3(Vec3::Z).normalize_or(Vec3::Z)
            };
            let (src, trg) = (p(P::Bezier2, 0.0), p(P::Bezier3, 0.0));
            let combine = p(P::CombineTime, 0.0);
            let travel = if combine > 0.0 { combine } else { life };
            let (grav, drag, threshold) = (p(P::Grav, 0.0), p(P::Drag, 1.0), p(P::Threshold, 0.0));

            // Age and move.
            e.particles.retain_mut(|q| {
                q.age += dt;
                match target {
                    Some(tg) if p2p && bezier_path => {
                        let u = (q.age / travel.max(1e-3)).clamp(0.0, 1.0);
                        q.pos = bezier(q.origin, q.origin + z_axis * src, tg + z_axis * trg, tg, u);
                    }
                    Some(tg) if p2p => {
                        let to = tg - q.pos;
                        let dist = to.length();
                        if threshold > 0.0 && dist < threshold {
                            return false;
                        }
                        q.vel = q.vel * drag.clamp(0.0, 1.0).powf(dt)
                            + to.normalize_or_zero() * grav * dt;
                        q.pos += q.vel * dt;
                    }
                    _ => {
                        q.vel += gravity * dt;
                        q.pos += q.vel * dt;
                        if let Some(co) = bounce
                            && q.pos.z < ground
                            && q.vel.z < 0.0
                        {
                            q.pos.z = ground;
                            q.vel = Vec3::new(q.vel.x, q.vel.y, -q.vel.z * co) * BOUNCE_KEEP;
                        }
                    }
                }
                q.life < 0.0 || q.age < q.life
            });
            // Spawn.
            let moved = e.last_pos.map_or(0.0, |last| last.distance(here));
            e.last_pos = Some(here);
            let trail = em.spawntype == 1;
            let count = match update {
                Update::Single => usize::from(e.particles.is_empty()),
                Update::Explosion => {
                    // A detonate key between the last time and now.
                    let keys =
                        at.and_then(|(an, _)| an.controllers.iter().find(|c| c.name == "detonate"));
                    let fired = match (keys, e.last_time) {
                        (Some(c), Some(last)) => c.times.iter().any(|&k| {
                            if last <= t { last < k && k <= t } else { k > last || k <= t }
                        }),
                        _ => false,
                    };
                    if fired { p(P::Birthrate, 0.0).max(0.0) as usize } else { 0 }
                }
                Update::Fountain => {
                    // (A birthrate under 1 a second makes none: the client
                    // drew no particle of 0.5 or 0.667 in ten seconds,
                    // ASCII or compiled, and 1.5 a second of 1.5;
                    // `reading_particle_turn_and_birth`. None of the
                    // game's 9,530 emitters has such a birthrate.)
                    let rate = p(P::Birthrate, 0.0);
                    let rate = if !trail && rate < 1.0 { 0.0 } else { rate.max(0.0) };
                    e.owed += rate * if trail { moved } else { dt };
                    let n = e.owed.floor();
                    e.owed -= n;
                    n as usize
                }
                _ => 0,
            };
            e.last_time = Some(t);
            let (xs, ys) = (p(P::XSize, 0.0) / 100.0, p(P::YSize, 0.0) / 100.0);
            let spread = p(P::Spread, 0.0);
            let (v0, rv) = (p(P::Velocity, 0.0), p(P::RandVel, 0.0));
            let rng = &mut self.rng;
            for _ in 0..count.min(MAX_PARTICLES.saturating_sub(e.particles.len())) {
                let local = Vec3::new(rng.range(-0.5, 0.5) * xs, rng.range(-0.5, 0.5) * ys, 0.0);
                // A direction within the cone around +Z.
                let theta = rng.range(0.0, spread * 0.5);
                let phi = rng.range(0.0, std::f32::consts::TAU);
                let dir = Vec3::new(theta.sin() * phi.cos(), theta.sin() * phi.sin(), theta.cos());
                let speed = v0 + rng.range(0.0, rv);
                let (pos, vel) = if inherit {
                    (local, dir * speed)
                } else {
                    (
                        world.transform_point3(local),
                        world.transform_vector3(dir).normalize_or_zero() * speed,
                    )
                };
                let frame0 = if em.flags & 0x20 != 0 { rng.range(0.0, 64.0) } else { 0.0 };
                e.particles.push(Particle {
                    pos,
                    origin: pos,
                    vel: if update == Update::Single { Vec3::ZERO } else { vel },
                    age: 0.0,
                    life: if update == Update::Single && em.looping { -1.0 } else { life },
                    frame0,
                });
            }
        }
    }

    /// The particles as camera-facing (or aligned) quads, for a camera with
    /// this view matrix.
    pub fn batches(
        &self,
        model: &Model,
        anim: Option<&Animation>,
        time: f32,
        pose: &[Mat4],
        transform: Mat4,
        view: Mat4,
    ) -> Vec<ParticleBatch> {
        let t = anim.map_or(time, |a| crate::anim::time_in(a, time));
        let at = anim.map(|a| (a, t));
        let inv_view = view.inverse();
        let eye = inv_view.w_axis.truncate();
        let (cam_right, cam_up) = (inv_view.x_axis.truncate(), inv_view.y_axis.truncate());
        let mut out = Vec::new();
        for e in &self.emitters {
            let node = &model.nodes[e.node];
            let NodeKind::Emitter(em) = &node.kind else { continue };
            if e.bolt.len() >= 2 {
                let at = anim_node(model, e.node, at);
                let p = |which, default| param(e, at, which, default);
                let size = p(P::SizeStart, 0.0);
                let width = if size > 0.0 { size } else { 0.1 };
                let c = color(e, at, false, Vec3::ONE);
                let color = [c.x, c.y, c.z, p(P::AlphaStart, 1.0)];
                let points: Vec<(Vec3, f32, [f32; 4])> =
                    e.bolt.iter().map(|&q| (q, width, color)).collect();
                out.push(ParticleBatch {
                    texture: em.texture.clone(),
                    blend: e.blend,
                    render_order: em.render_order,
                    vertices: strip(&points, None, eye),
                    tint: None,
                    // (Soft as other particles, by its size: the client
                    // gave a bolt of size 0.5 a softness of 20;
                    // `reading_soft_particles_of_lightning_and_linked`.)
                    softness: soft(size, p(P::SizeEnd, 0.0), e.render),
                });
                continue;
            }
            if e.particles.is_empty() || em.chunk.is_some() {
                continue;
            }
            let at = anim_node(model, e.node, at);
            let p = |which, default| param(e, at, which, default);
            let (s0, s1) = (p(P::SizeStart, 1.0), p(P::SizeEnd, 1.0));
            let softness = soft(s0, s1, e.render);
            let world = transform * pose.get(e.node).copied().unwrap_or(Mat4::IDENTITY);
            let tint = (em.flags & 0x8 != 0).then(|| world.w_axis.truncate());
            let inherit = em.flags & (0x40 | 0x100) != 0;
            let (c0, c1) = (color(e, at, false, Vec3::ONE), color(e, at, true, Vec3::ONE));
            let (a0, a1) = (p(P::AlphaStart, 1.0), p(P::AlphaEnd, 1.0));
            let (sy0, sy1) = (p(P::SizeStartY, 0.0), p(P::SizeEndY, 0.0));
            // (Radians a second: the client turned a 0.25 particle a
            // quarter turn in about 7 s; `reading_particle_spin_and_order`.)
            let rot_speed = p(P::ParticleRot, 0.0);
            // (Absent, 10, as compilers write it; every one of the game's
            // 332 motion-blurred emitters has its own. Not read in the
            // client: a hand-written ASCII model without one.)
            let blur = p(P::BlurLength, 10.0);
            let (fps, f0, f1) = (p(P::Fps, 0.0), p(P::FrameStart, 0.0), p(P::FrameEnd, 0.0));
            let (gx, gy) = (em.xgrid.max(1), em.ygrid.max(1));
            let frames = (f1 - f0 + 1.0).max(1.0);
            let render = e.render;
            if render == Render::Linked {
                // One strip through the particles, oldest first.
                let mut order: Vec<&Particle> = e.particles.iter().collect();
                order.sort_by(|a, b| b.age.total_cmp(&a.age));
                let points: Vec<(Vec3, f32, [f32; 4])> = order
                    .iter()
                    .map(|q| {
                        let k = if q.life > 0.0 { (q.age / q.life).clamp(0.0, 1.0) } else { 0.0 };
                        let pos = if inherit { world.transform_point3(q.pos) } else { q.pos };
                        let c = c0.lerp(c1, k);
                        // (A ribbon is twice its particles' size across:
                        // their size to either side. Seen in the client.)
                        (pos, 2.0 * (s0 + (s1 - s0) * k), [c.x, c.y, c.z, a0 + (a1 - a0) * k])
                    })
                    .collect();
                // Each particle's frame of the flip-book, as a quad's.
                let cells: Vec<(Vec2, Vec2)> = order
                    .iter()
                    .map(|q| {
                        let frame =
                            (f0 + (q.frame0 + q.age * fps).floor() % frames).min(f1.max(f0)) as u32;
                        cell(frame, gx, gy)
                    })
                    .collect();
                out.push(ParticleBatch {
                    texture: em.texture.clone(),
                    blend: e.blend,
                    render_order: em.render_order,
                    vertices: strip(&points, Some(&cells), eye),
                    tint,
                    softness,
                });
                continue;
            }
            // (Six vertices a particle, in the order they were born, the
            // newest over the older, as the client draws them, whichever
            // is nearer the eye: a rising and a falling column alike;
            // `reading_particle_spin_and_order`.)
            let mut vertices: Vec<ParticleVertex> = Vec::with_capacity(e.particles.len() * 6);
            for q in &e.particles {
                let k = if q.life > 0.0 { (q.age / q.life).clamp(0.0, 1.0) } else { 0.0 };
                let pos = if inherit { world.transform_point3(q.pos) } else { q.pos };
                let color = c0.lerp(c1, k);
                let alpha = a0 + (a1 - a0) * k;
                let sx = s0 + (s1 - s0) * k;
                let sy = if sy0 > 0.0 || sy1 > 0.0 { sy0 + (sy1 - sy0) * k } else { sx };
                let frame = (f0 + (q.frame0 + q.age * fps).floor() % frames).min(f1.max(f0)) as u32;
                let (uv0, uv1) = cell(frame, gx, gy);
                // (Upright where it doesn't spin: the client draws a still
                // emitter's square particles square, each of them, not each
                // turned by a random angle; `surfaces_in_the_client`. A
                // spinning one starts upright too, turning anticlockwise as
                // seen: `reading_particle_spin_and_order`.)
                let angle = rot_speed * q.age;
                if render == Render::MotionBlur {
                    let quad = motion_blur(
                        q,
                        inherit,
                        world,
                        eye,
                        (sx, sy),
                        blur,
                        (uv0, uv1),
                        color,
                        alpha,
                    );
                    vertices.extend(quad.into_iter().flatten());
                    continue;
                }
                let (right, up) = match render {
                    // Upright, turned about the world's Z to the eye (a
                    // shaft of light); `billboard_to_world_z` lies flat,
                    // facing up (the glow at the shaft's foot). Seen in
                    // the client: `particles_look`, `MG_PARTICLES=worldz`;
                    // turned to the eye from every side, however the
                    // emitter is turned: `upright_particles_look`.
                    Render::AlignedToWorldZ => {
                        let to_eye = (eye - pos) * Vec3::new(1.0, 1.0, 0.0);
                        let r = Vec3::Z.cross(to_eye).normalize_or(Vec3::X);
                        (r, Vec3::Z)
                    }
                    // Lie flat in the emitter's local XY plane, facing
                    // along the emitter's Z axis. This matches the game;
                    // seen from behind, the client draws nothing of it
                    // unless the emitter is two-sided (`twosidedtex`):
                    // `local_z_particles_lie_as_the_client_lays_them`.
                    Render::BillboardToLocalZ => {
                        let r = world.transform_vector3(Vec3::X).normalize_or(Vec3::X);
                        let u = world.transform_vector3(Vec3::Y).normalize_or(Vec3::Y);
                        if !em.two_sided && r.cross(u).dot(eye - pos) <= 0.0 {
                            continue;
                        }
                        (r, u)
                    }
                    Render::BillboardToWorldZ => (Vec3::X, Vec3::Y),
                    Render::AlignedToParticleDir => {
                        let world_vel =
                            if inherit { world.transform_vector3(q.vel) } else { q.vel };
                        let d = world_vel.normalize_or(Vec3::Z);
                        let r = d.cross(eye - pos).normalize_or(Vec3::X);
                        (r, d)
                    }
                    Render::Normal | Render::Linked | Render::MotionBlur => {
                        let rot = Quat::from_axis_angle((eye - pos).normalize_or(Vec3::Z), angle);
                        (rot * cam_right, rot * cam_up)
                    }
                };
                let (hx, hy) = (right * sx * 0.5, up * sy * 0.5);
                let c = [color.x, color.y, color.z, alpha];
                let v = |p: Vec3, uv: Vec2| ParticleVertex {
                    pos: p.to_array(),
                    uv: uv.to_array(),
                    color: c,
                    softness: 0.0,
                };
                let (bl, br, tr, tl) = (pos - hx - hy, pos + hx - hy, pos + hx + hy, pos - hx + hy);
                vertices.extend([
                    v(bl, Vec2::new(uv0.x, uv0.y)),
                    v(br, Vec2::new(uv1.x, uv0.y)),
                    v(tr, Vec2::new(uv1.x, uv1.y)),
                    v(bl, Vec2::new(uv0.x, uv0.y)),
                    v(tr, Vec2::new(uv1.x, uv1.y)),
                    v(tl, Vec2::new(uv0.x, uv1.y)),
                ]);
            }
            out.push(ParticleBatch {
                texture: em.texture.clone(),
                blend: e.blend,
                render_order: em.render_order,
                vertices,
                tint,
                softness,
            });
        }
        out
    }

    /// Chunk emitters' particles: their models where they are, tumbling.
    pub fn chunks(
        &self,
        model: &Model,
        anim: Option<&Animation>,
        time: f32,
        pose: &[Mat4],
        transform: Mat4,
    ) -> Vec<Chunk> {
        let t = anim.map_or(time, |a| crate::anim::time_in(a, time));
        let at = anim.map(|a| (a, t));
        let mut out = Vec::new();
        for e in &self.emitters {
            let NodeKind::Emitter(em) = &model.nodes[e.node].kind else { continue };
            let Some(chunk) = &e.chunk else { continue };
            let at = anim_node(model, e.node, at);
            let p = |which, default| param(e, at, which, default);
            let world = transform * pose.get(e.node).copied().unwrap_or(Mat4::IDENTITY);
            let inherit = em.flags & (0x40 | 0x100) != 0;
            let (s0, s1) = (p(P::SizeStart, 1.0), p(P::SizeEnd, 1.0));
            // (Radians a second about the emitter's z, from the way the
            // emitter faces at birth, anticlockwise seen from above: as the
            // quads' spin, read in the client, `reading_chunk_spin`. It
            // was taken as turns a second about an axis of each chunk's
            // own, from a random angle.)
            let spin = p(P::ParticleRot, 0.0);
            let (_, facing, _) = world.to_scale_rotation_translation();
            for q in &e.particles {
                let k = if q.life > 0.0 { (q.age / q.life).clamp(0.0, 1.0) } else { 0.0 };
                let pos = if inherit { world.transform_point3(q.pos) } else { q.pos };
                let rot = facing * Quat::from_rotation_z(spin * q.age);
                out.push(Chunk {
                    model: chunk.clone(),
                    transform: Mat4::from_scale_rotation_translation(
                        Vec3::splat(s0 + (s1 - s0) * k),
                        rot,
                        pos,
                    ),
                });
            }
        }
        out
    }
}

/// A `Motion_Blur` particle's quad: twice its size across and from its
/// head back to where its tail has got to, faded out there, as the client
/// draws one (`client_render.rs`'s `reading_motion_blur_*`, 2026-10-30):
/// the tail follows the particle `0.76 × blurlength` seconds behind, so
/// that a streak is `speed × τ × (1 − e^(−age/τ))` longer than the square
/// (a 2 m/s particle's 1.3 m at its 0.9th second with `blurlength` 1, 0.56
/// m with 0.25). None with `blurlength` 0, as the client draws none.
#[allow(clippy::too_many_arguments)]
fn motion_blur(
    q: &Particle,
    inherit: bool,
    world: Mat4,
    eye: Vec3,
    (sx, sy): (f32, f32),
    blur: f32,
    (uv0, uv1): (Vec2, Vec2),
    color: Vec3,
    alpha: f32,
) -> Option<[ParticleVertex; 6]> {
    if blur <= 0.0 {
        return None;
    }
    let pos = if inherit { world.transform_point3(q.pos) } else { q.pos };
    let vel = if inherit { world.transform_vector3(q.vel) } else { q.vel };
    let d = vel.normalize_or(Vec3::Z);
    let r = d.cross(eye - pos).normalize_or(Vec3::X) * sx;
    let tau = BLUR_SECONDS * blur;
    let trail = vel.length() * tau * (1.0 - (-q.age / tau).exp());
    let (head, tail) = (pos + d * sy, pos - d * (sy + trail));
    let v = |p: Vec3, uv: Vec2, a: f32| ParticleVertex {
        pos: p.to_array(),
        uv: uv.to_array(),
        color: [color.x, color.y, color.z, a],
        softness: 0.0,
    };
    let (bl, br) = (v(tail - r, uv0, 0.0), v(tail + r, Vec2::new(uv1.x, uv0.y), 0.0));
    let (tr, tl) = (v(head + r, uv1, alpha), v(head - r, Vec2::new(uv0.x, uv1.y), alpha));
    Some([bl, br, tr, bl, tr, tl])
}

/// How far behind a `Motion_Blur` particle's tail follows it, seconds a
/// unit of `blurlength` (fitted to the client's streaks: [`motion_blur`]).
const BLUR_SECONDS: f32 = 0.76;

/// A flip-book's frame as the corners of its cell in the texture: the
/// first frame is the picture's top left, and they go along each row and
/// down the rows, as the game plays them. (v is 0 at the picture's bottom:
/// the top row is the last. A fog whose frames go from nothing at the top
/// to thick at the bottom fades in; begun at the bottom, each puff was
/// born at its thickest. Seen in the client: `particles_look`.)
fn cell(frame: u32, gx: u32, gy: u32) -> (Vec2, Vec2) {
    let (fx, fy) = (frame % gx, gy - 1 - frame / gx % gy);
    let uv0 = Vec2::new(fx as f32 / gx as f32, fy as f32 / gy as f32);
    (uv0, uv0 + Vec2::new(1.0 / gx as f32, 1.0 / gy as f32))
}

/// How far along its cell a linked particle's piece is at the next
/// particle, `gap` metres on, for a ribbon `width` across (twice the
/// particle's size): `gap / (gap + size / 2)`, fitted to the client's
/// (`batch6_ribbon_sizes`: 0.93 to 0.63 at a metre's gap for sizes 0.15 to
/// 1.2 m). The cell runs on past the next particle, where the next piece
/// starts its own.
fn reach(gap: f32, width: f32) -> f32 {
    let run = gap + width / 4.0;
    if run > 0.0 { gap / run } else { 1.0 }
}

/// Camera-facing quads joining points (position, width, colour). Without
/// `cells`, the texture's v runs along the whole strip (a bolt of
/// lightning). With them (one a point: the corners of its frame in the
/// texture), each piece is its first point's particle stretched to reach
/// the next, showing that particle's frame: linked particles. (Drawn with
/// the whole texture, a flip-book's frames showed side by side along the
/// strip: a lattice over a ground mist.)
fn strip(
    points: &[(Vec3, f32, [f32; 4])],
    cells: Option<&[(Vec2, Vec2)]>,
    eye: Vec3,
) -> Vec<ParticleVertex> {
    let mut out = Vec::new();
    let n = points.len().saturating_sub(1).max(1) as f32;
    for (i, w) in points.windows(2).enumerate() {
        let ((a, wa, ca), (b, wb, cb)) = (w[0], w[1]);
        let side =
            |p: Vec3, width: f32| (b - a).cross(eye - p).normalize_or(Vec3::X) * (width * 0.5);
        let (sa, sb) = (side(a, wa), side(b, wb));
        let (uv0, uv1) = match cells.and_then(|c| c.get(i)) {
            // (The piece's cell reaches the next particle at
            // `gap / (gap + size / 2)` of it, running on half the older
            // particle's size past it, as the client draws it
            // (`batch6_ribbon_sizes`); the width is twice the size.)
            Some(&(c0, c1)) => {
                (c0, Vec2::new(c1.x, c0.y + (c1.y - c0.y) * reach(a.distance(b), wa)))
            }
            None => (Vec2::new(0.0, i as f32 / n), Vec2::new(1.0, (i + 1) as f32 / n)),
        };
        let v = |p: Vec3, u: f32, v: f32, c: [f32; 4]| ParticleVertex {
            pos: p.to_array(),
            uv: [u, v],
            color: c,
            softness: 0.0,
        };
        out.extend([
            v(a - sa, uv0.x, uv0.y, ca),
            v(a + sa, uv1.x, uv0.y, ca),
            v(b + sb, uv1.x, uv1.y, cb),
            v(a - sa, uv0.x, uv0.y, ca),
            v(b + sb, uv1.x, uv1.y, cb),
            v(b - sb, uv0.x, uv1.y, cb),
        ]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_mdl::{Controller, Emitter, Node};

    /// The client's `particleSoftness` (`soft_particles_in_the_client`):
    /// 10 over the larger size, none for the emitters laid along an axis.
    #[test]
    fn softness_is_the_client_s() {
        for (start, end, render, want) in [
            (0.25, 0.25, "Normal", 40.0),
            (0.5, 0.5, "normal", 20.0),
            (1.0, 1.0, "Normal", 10.0),
            (2.0, 2.0, "Normal", 5.0),
            (1.2, 1.2, "Normal", 10.0 / 1.2),
            (0.25, 1.0, "Normal", 10.0),
            (1.0, 0.25, "Normal", 10.0),
            (0.5, 0.5, "Aligned_to_World_Z", 20.0),
            (0.5, 0.5, "Motion_Blur", 20.0),
            (0.5, 0.5, "Billboard_to_Local_Z", 0.0),
            (0.5, 0.5, "Billboard_to_World_Z", 0.0),
            (0.0, 0.0, "Normal", 0.0),
        ] {
            let got = softness(start, end, render);
            assert!((got - want).abs() < 1e-4, "{start} {end} {render}: {got}");
        }
    }

    fn fountain() -> Model {
        let mut root = Node::new("m", NodeKind::Dummy);
        root.children = vec![1];
        let mut e = Node::new(
            "fire",
            NodeKind::Emitter(Emitter {
                update: "Fountain".into(),
                render: "Normal".into(),
                blend: "Lighten".into(),
                xgrid: 2,
                ygrid: 2,
                ..Default::default()
            }),
        );
        e.parent = Some(0);
        for (name, v) in
            [("birthrate", 10.0), ("lifeexp", 1.0), ("velocity", 1.0), ("sizestart", 0.5)]
        {
            e.controllers.push(Controller::constant(name, &[v]));
        }
        Model { name: "m".into(), nodes: vec![root, e], ..Default::default() }
    }

    /// A waterfall of Medieval Rural: a heavy fountain turned to point
    /// down, its particles kept in its own space. They fall.
    #[test]
    fn particles_kept_with_a_turned_emitter_fall_the_world_s_way() {
        for (flags, turned) in [(0x40, true), (0x40, false), (0, true), (0, false)] {
            let mut m = fountain();
            let NodeKind::Emitter(em) = &mut m.nodes[1].kind else { unreachable!() };
            em.flags = flags;
            m.nodes[1].controllers.push(Controller::constant("mass", &[1.0]));
            m.nodes[1].controllers.retain(|c| c.name != "velocity");
            m.nodes[1].controllers.push(Controller::constant("velocity", &[0.0]));
            if turned {
                // Upside down, about X.
                m.nodes[1].orientation = [1.0, 0.0, 0.0, 0.0];
            }
            let pose = crate::rest_pose(&m);
            let mut p = Particles::new(&m);
            for _ in 0..10 {
                p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
            }
            let eye = Mat4::look_at_rh(Vec3::new(0.0, -5.0, 1.0), Vec3::ZERO, Vec3::Z);
            let b = p.batches(&m, None, 0.0, &pose, Mat4::IDENTITY, eye);
            let lowest = b[0].vertices.iter().map(|v| v.pos[2]).fold(f32::MAX, f32::min);
            let highest = b[0].vertices.iter().map(|v| v.pos[2]).fold(f32::MIN, f32::max);
            assert!(lowest < -1.0 && highest < 1.0, "{flags:#x} {turned}: {lowest} to {highest}");
        }
    }

    /// `fountain` with `params` set (replacing its own).
    fn fountain_with(render: &str, params: &[(&str, f32)]) -> Model {
        let mut m = fountain();
        if let NodeKind::Emitter(e) = &mut m.nodes[1].kind {
            e.render = render.into();
            e.blend = "Normal".into();
        }
        for (name, v) in params {
            m.nodes[1].controllers.retain(|c| c.name != *name);
            m.nodes[1].controllers.push(Controller::constant(name, &[*v]));
        }
        m
    }

    /// A Fountain emitter's birthrate under 1 a second makes no particle,
    /// as in the client (`client_render.rs`'s
    /// `reading_particle_turn_and_birth`: 0.5 and 0.667 none, ASCII or
    /// compiled, 1.5 one and a half a second).
    #[test]
    fn a_birthrate_under_one_makes_none() {
        for (rate, want) in [(0.5f32, 0..=0), (0.99, 0..=0), (1.5, 14..=15)] {
            let m = fountain_with("Normal", &[("birthrate", rate), ("lifeexp", 20.0)]);
            let pose = crate::rest_pose(&m);
            let mut p = Particles::new(&m);
            for _ in 0..100 {
                p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
            }
            let n = p.emitters[0].particles.len();
            assert!(want.contains(&n), "birthrate {rate}: {n} in 10 s");
        }
    }

    /// A `Motion_Blur` particle is twice its size across and streaks
    /// behind it as the client's (`reading_motion_blur_single`): its tail
    /// follows `0.76 × blurlength` s behind, faded out; with `blurlength`
    /// 0 it isn't drawn.
    #[test]
    fn a_motion_blurred_particle_streaks_behind_it() {
        let view = Mat4::look_at_rh(Vec3::new(0.0, -5.0, 1.0), Vec3::ZERO, Vec3::Z);
        let streak = |blur: f32| -> Option<(f32, f32, [f32; 2])> {
            let m = fountain_with(
                "Motion_Blur",
                &[
                    ("birthrate", 1.0),
                    ("lifeexp", 5.0),
                    ("velocity", 2.0),
                    ("sizestart", 0.12),
                    ("sizeend", 0.12),
                    ("blurlength", blur),
                ],
            );
            let pose = crate::rest_pose(&m);
            let mut p = Particles::new(&m);
            // (One born in the first second, 0.9 s old.)
            for _ in 0..19 {
                p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
            }
            assert_eq!(p.emitters[0].particles.len(), 1);
            let b = p.batches(&m, None, 0.0, &pose, Mat4::IDENTITY, view);
            let v = b.first().map(|b| b.vertices.clone()).unwrap_or_default();
            if v.is_empty() {
                return None;
            }
            let z: Vec<f32> = v.iter().map(|v| v.pos[2]).collect();
            let x: Vec<f32> = v.iter().map(|v| v.pos[0]).collect();
            let span = |s: &[f32]| {
                s.iter().copied().fold(f32::MIN, f32::max)
                    - s.iter().copied().fold(f32::MAX, f32::min)
            };
            Some((span(&z), span(&x), [v[0].color[3], v[2].color[3]]))
        };
        // (Born as the first second ended: 0.9 s old.)
        let age = 0.9;
        for (blur, want) in [(1.0f32, 1.3f32), (0.25, 0.56)] {
            let (long, wide, [tail, head]) = streak(blur).unwrap();
            let tau = 0.76 * blur;
            let trail = 2.0 * tau * (1.0 - (-age / tau).exp());
            assert!((long - (0.24 + trail)).abs() < 1e-3, "blurlength {blur}: {long}");
            assert!(
                (long - want).abs() < 0.1,
                "blurlength {blur}: {long} against the client's {want}"
            );
            assert!((wide - 0.24).abs() < 1e-3, "twice its size across: {wide}");
            assert_eq!((tail, head), (0.0, 1.0), "faded out at its tail");
        }
        assert!(streak(0.0).is_none(), "none with blurlength 0");
    }

    /// A spinning particle starts upright and turns by `particleRot`
    /// radians a second, anticlockwise as seen; a Normal emitter's
    /// particles are drawn in the order they were born, the newest last
    /// (`reading_particle_spin_and_order`).
    #[test]
    fn spinning_particles_start_upright_and_the_newest_is_drawn_last() {
        let view = Mat4::look_at_rh(Vec3::new(0.0, -5.0, 1.0), Vec3::ZERO, Vec3::Z);
        let m = fountain_with(
            "Normal",
            &[("birthrate", 1.0), ("lifeexp", 10.0), ("velocity", 0.25), ("particlerot", 0.25)],
        );
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        for _ in 0..41 {
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        }
        let ages: Vec<f32> = p.emitters[0].particles.iter().map(|q| q.age).collect();
        assert!(ages.len() >= 4 && ages.windows(2).all(|w| w[0] > w[1]), "{ages:?}");
        let b = p.batches(&m, None, 0.0, &pose, Mat4::IDENTITY, view);
        for (q, v) in p.emitters[0].particles.iter().zip(b[0].vertices.as_chunks::<6>().0) {
            let right = Vec3::from(v[1].pos) - Vec3::from(v[0].pos);
            let angle = right.z.atan2(right.x);
            assert!((angle - 0.25 * q.age).abs() < 0.02, "age {}: turned {angle}", q.age);
        }
    }

    /// A flip-book's first frame is the picture's top left (the last row
    /// of v), and its frames go along a row, then down.
    #[test]
    fn a_flip_book_begins_at_the_picture_s_top_left() {
        let at = |frame| cell(frame, 4, 2).0;
        assert_eq!(at(0), Vec2::new(0.0, 0.5));
        assert_eq!(at(3), Vec2::new(0.75, 0.5));
        assert_eq!(at(4), Vec2::new(0.0, 0.0));
        assert_eq!(cell(7, 4, 2), (Vec2::new(0.75, 0.0), Vec2::new(1.0, 0.5)));
        // (One cell: the whole picture.)
        assert_eq!(cell(0, 1, 1), (Vec2::ZERO, Vec2::ONE));
    }

    #[test]
    fn a_fountain_spawns_rises_and_expires() {
        let m = fountain();
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        for _ in 0..10 {
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        }
        let n = p.emitters[0].particles.len();
        assert!((8..=10).contains(&n), "{n} particles after a second");
        // Newborn particles are still at the emitter; older ones have risen.
        assert!(p.emitters[0].particles.iter().all(|q| q.pos.z >= 0.0 && q.age <= 1.0));
        assert!(p.emitters[0].particles.iter().any(|q| q.pos.z > 0.5));
        let b = p.batches(
            &m,
            None,
            0.0,
            &pose,
            Mat4::IDENTITY,
            Mat4::look_at_rh(Vec3::new(0.0, -5.0, 1.0), Vec3::ZERO, Vec3::Z),
        );
        assert_eq!(b[0].vertices.len(), n * 6);
        assert_eq!(b[0].blend, ParticleBlend::Lighten);
        for _ in 0..20 {
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        }
        assert!(p.emitters[0].particles.iter().all(|q| q.age <= 1.0));
    }

    /// `Aligned_to_World_Z` particles stand (a shaft of light's beam);
    /// `Billboard_to_World_Z` ones lie flat (the glow at its foot), as the
    /// client draws them.
    #[test]
    fn world_z_particles_stand_or_lie() {
        let heights = |render: &str| -> f32 {
            let mut m = fountain();
            if let NodeKind::Emitter(e) = &mut m.nodes[1].kind {
                e.render = render.into();
            }
            let pose = crate::rest_pose(&m);
            let mut p = Particles::new(&m);
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
            let view = Mat4::look_at_rh(Vec3::new(0.0, -5.0, 1.0), Vec3::ZERO, Vec3::Z);
            let b = p.batches(&m, None, 0.0, &pose, Mat4::IDENTITY, view);
            let z: Vec<f32> = b[0].vertices[..6].iter().map(|v| v.pos[2]).collect();
            z.iter().copied().fold(f32::MIN, f32::max) - z.iter().copied().fold(f32::MAX, f32::min)
        };
        assert!(heights("Aligned_to_World_Z") > 0.2, "upright");
        assert!(heights("Billboard_to_World_Z") < 1e-4, "flat");
        assert!(heights("Billboard_to_Local_Z") < 1e-4, "flat when the emitter is not rotated");
    }

    /// A `Billboard_to_Local_Z` particle lies in its emitter's local XY
    /// plane, facing along the emitter's Z axis. This holds however the
    /// emitter is rotated and wherever the camera is.
    #[test]
    fn a_local_z_particle_lies_in_its_emitter_s_plane() {
        let mut m = fountain();
        if let NodeKind::Emitter(e) = &mut m.nodes[1].kind {
            e.render = "Billboard_to_Local_Z".into();
        }
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        // Not rotated, and rotated 90 degrees about X (so its Z points along -Y).
        let tipped = Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
        for (turn, normal) in [(Mat4::IDENTITY, Vec3::Z), (tipped, Vec3::NEG_Y)] {
            // (Both in front of it: seen from behind or edge-on it isn't drawn.)
            for eye in [Vec3::new(0.0, -5.0, 1.0), Vec3::new(5.0, -3.0, 4.0)] {
                let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Z);
                let b = p.batches(&m, None, 0.0, &pose, turn, view);
                let v = &b[0].vertices[..6];
                let (bl, br, tr) =
                    (Vec3::from(v[0].pos), Vec3::from(v[1].pos), Vec3::from(v[2].pos));
                let facing = (br - bl).cross(tr - br).normalize();
                assert!(facing.dot(normal).abs() > 0.999, "{normal} from {eye}: {facing}");
            }
        }
        // Seen from behind, nothing; unless it is two-sided.
        let below = Mat4::look_at_rh(Vec3::new(0.0, -1.0, -5.0), Vec3::ZERO, Vec3::Z);
        let drawn = |m: &Model| {
            p.batches(m, None, 0.0, &pose, Mat4::IDENTITY, below)
                .iter()
                .any(|b| !b.vertices.is_empty())
        };
        assert!(!drawn(&m), "its back");
        if let NodeKind::Emitter(e) = &mut m.nodes[1].kind {
            e.two_sided = true;
        }
        assert!(drawn(&m), "two-sided");
    }

    /// An upright (`Aligned_to_World_Z`) particle is turned about Z to the
    /// eye, whichever way the emitter is turned: the client shows it face
    /// on and the same way round from every side (`client_render.rs`'s
    /// `upright_particles_look`).
    #[test]
    fn an_upright_particle_turns_to_the_eye() {
        let mut m = fountain();
        if let NodeKind::Emitter(e) = &mut m.nodes[1].kind {
            e.render = "Aligned_to_World_Z".into();
        }
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        // The emitter turned a quarter about Z, seen from the south and
        // from the east.
        let turned = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        for eye in [Vec3::new(0.0, -5.0, 1.0), Vec3::new(5.0, 0.0, 1.0)] {
            let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Z);
            let b = p.batches(&m, None, 0.0, &pose, turned, view);
            let v = &b[0].vertices[..6];
            let (bl, br) = (Vec3::from(v[0].pos), Vec3::from(v[1].pos));
            let right = (br - bl).normalize();
            let to_eye = (eye * Vec3::new(1.0, 1.0, 0.0)).normalize();
            assert!(right.dot(to_eye).abs() < 1e-3, "face on from {eye}: {right}");
            // Its first column on the left as seen.
            let seen_right = Vec3::Z.cross(to_eye);
            assert!(right.dot(seen_right) > 0.99, "the right way round from {eye}");
        }
    }

    /// Thrown down from 1.5 m (the emitter turned upside down), as the
    /// client was measured: a bounce keeps 0.8 of the speed along the ground
    /// and 0.8 × `bounce_co` off it.
    #[test]
    fn bouncing_particles() {
        let bouncy =
            Emitter { update: "Single".into(), flags: 0x10, looping: true, ..Default::default() };
        let mut m = emitter(bouncy, &[("mass", 1.0), ("bounce_co", 0.5)]);
        m.nodes[1].position = [0.0, 0.0, 1.5];
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        p.update(&m, None, 0.0, 0.0, &pose, Mat4::IDENTITY);
        p.emitters[0].particles[0].vel = Vec3::new(1.0, 0.0, 0.0);
        // Falling 1.5 m takes 0.553 s; the first bounce comes at 5.42 m/s.
        let mut top_after = 0.0f32;
        let mut bounced = false;
        for _ in 0..2000 {
            p.update(&m, None, 0.0, 0.001, &pose, Mat4::IDENTITY);
            let q = &p.emitters[0].particles[0];
            assert!(q.pos.z >= 0.0, "never under the ground");
            bounced |= q.vel.z > 0.0;
            if bounced {
                top_after = top_after.max(q.pos.z);
            }
        }
        // Up at 0.4 × 5.42 m/s: (0.4)² × 1.5 = 0.24 m.
        assert!((top_after - 0.24).abs() < 0.02, "{top_after}");
        let q = &p.emitters[0].particles[0];
        assert!(q.pos.x > 0.8 && q.pos.x < 1.1, "came to rest about 1 m out: {}", q.pos.x);
        // Without the flag, it falls through.
        let mut m2 = m.clone();
        if let NodeKind::Emitter(e) = &mut m2.nodes[1].kind {
            e.flags = 0;
        }
        let mut p = Particles::new(&m2);
        for _ in 0..100 {
            p.update(&m2, None, 0.0, 0.01, &pose, Mat4::IDENTITY);
        }
        assert!(p.emitters[0].particles[0].pos.z < 0.0);
    }

    #[test]
    fn tinted_emitters_say_where_they_are() {
        let mut m = fountain();
        m.nodes[1].position = [1.0, 2.0, 3.0];
        let pose = crate::rest_pose(&m);
        let view = Mat4::look_at_rh(Vec3::new(0.0, -5.0, 1.0), Vec3::ZERO, Vec3::Z);
        let mut p = Particles::new(&m);
        p.update(&m, None, 0.0, 0.5, &pose, Mat4::IDENTITY);
        assert_eq!(p.batches(&m, None, 0.0, &pose, Mat4::IDENTITY, view)[0].tint, None);
        if let NodeKind::Emitter(e) = &mut m.nodes[1].kind {
            e.flags |= 0x8;
        }
        let at = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0));
        let b = p.batches(&m, None, 0.0, &pose, at, view);
        assert_eq!(b[0].tint, Some(Vec3::new(11.0, 2.0, 3.0)));
    }

    /// An emitter (root dummy, emitter, reference child 2 m below).
    fn emitter(e: Emitter, params: &[(&str, f32)]) -> Model {
        let mut root = Node::new("m", NodeKind::Dummy);
        root.children = vec![1];
        let mut n = Node::new("em", NodeKind::Emitter(e));
        n.parent = Some(0);
        n.children = vec![2];
        for (name, v) in params {
            n.controllers.push(Controller::constant(name, &[*v]));
        }
        let mut r = Node::new("target", NodeKind::Reference(Default::default()));
        r.parent = Some(1);
        r.position = [0.0, 0.0, -2.0];
        Model { name: "m".into(), nodes: vec![root, n, r], ..Default::default() }
    }

    #[test]
    fn lightning_strikes_its_target() {
        let e =
            Emitter { update: "Lightning".into(), render: "Linked".into(), ..Default::default() };
        let m = emitter(
            e,
            &[
                ("birthrate", 3.0),
                ("lightningradius", 0.1),
                ("lightningscale", 1.0),
                ("sizestart", 0.2),
            ],
        );
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        let bolt = &p.emitters[0].bolt;
        assert_eq!(bolt.len(), 9, "three halvings: eight segments");
        assert_eq!(bolt[0], Vec3::ZERO);
        assert!(bolt[8].distance(Vec3::new(0.0, 0.0, -2.0)) <= 0.1 * 3f32.sqrt() + 1e-5);
        assert!(bolt.iter().any(|q| q.x != 0.0 || q.y != 0.0), "jagged");
        assert!(p.live());
        let view = Mat4::look_at_rh(Vec3::new(0.0, -5.0, -1.0), Vec3::new(0.0, 0.0, -1.0), Vec3::Z);
        let b = p.batches(&m, None, 0.0, &pose, Mat4::IDENTITY, view);
        assert_eq!(b[0].vertices.len(), 8 * 6);
        // Soft by its size, as the client's bolts are
        // (`reading_soft_particles_of_lightning_and_linked`: 20 at 0.5).
        assert!((b[0].softness - 10.0 / 0.2).abs() < 1e-3, "{}", b[0].softness);
    }

    #[test]
    fn point_to_point_particles_reach_the_target() {
        let target = Vec3::new(0.0, 0.0, -2.0);
        // Gravity: pulled in, overshooting and swinging back (drag 1).
        let gravity = Emitter {
            update: "Fountain".into(),
            render: "Normal".into(),
            flags: 0x1,
            ..Default::default()
        };
        let m = emitter(
            gravity,
            &[("birthrate", 20.0), ("lifeexp", 3.0), ("grav", 4.0), ("drag", 0.5)],
        );
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        for _ in 0..30 {
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        }
        let old: Vec<&Particle> = p.emitters[0].particles.iter().filter(|q| q.age > 1.5).collect();
        assert!(!old.is_empty());
        assert!(old.iter().all(|q| q.pos.distance(target) < 1.5), "pulled towards the target");
        // Bezier: at the target after combinetime.
        let bezier_em = Emitter {
            update: "Fountain".into(),
            render: "Normal".into(),
            flags: 0x1 | 0x2,
            ..Default::default()
        };
        let m = emitter(
            bezier_em,
            &[("birthrate", 10.0), ("lifeexp", 2.0), ("combinetime", 1.0), ("p2p_bezier2", 1.0)],
        );
        let mut p = Particles::new(&m);
        for _ in 0..15 {
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        }
        for q in &p.emitters[0].particles {
            if q.age >= 1.0 {
                assert!(q.pos.distance(target) < 1e-4, "arrived: {:?}", q.pos);
            } else if q.age > 0.0 {
                assert!(q.pos.z > -2.0 && q.pos.distance(target) > 1e-4, "on the way");
            }
        }
    }

    /// The parameters found once as the simulation is made are the model's
    /// by their names (as they were looked up, by name, each frame), an
    /// animation's key of one decides over them, and the emitter's words
    /// are read whatever their case.
    #[test]
    fn parameters_found_once_are_the_model_s_by_name() {
        let e = Emitter {
            update: "FOUNTAIN".into(),
            render: "Billboard_To_Local_Z".into(),
            blend: "Punch-Through".into(),
            chunk: Some("Plc_Chunk_W01".into()),
            ..Default::default()
        };
        // Every parameter its own value (its place among the names, and a
        // half), but two the model has none of.
        let absent = [P::LightningSubdiv as usize, P::Drag as usize];
        let values: Vec<(&str, f32)> = (NAMES.iter().enumerate())
            .filter(|(i, _)| !absent.contains(i))
            .map(|(i, name)| (*name, i as f32 + 0.5))
            .collect();
        let mut m = emitter(e, &values);
        m.nodes[1].controllers.push(Controller::constant("colorstart", &[0.1, 0.2, 0.3]));
        m.nodes[1].controllers.push(Controller::constant("colorend", &[0.4, 0.5]));
        let p = Particles::new(&m);
        let sim = &p.emitters[0];
        assert_eq!(
            (sim.update, sim.render, sim.blend),
            (Update::Fountain, Render::BillboardToLocalZ, ParticleBlend::PunchThrough)
        );
        assert_eq!(sim.chunk.as_deref(), Some("plc_chunk_w01"));
        for (i, name) in NAMES.iter().enumerate() {
            let by_name = m.nodes[1].value(name).and_then(|v| v.first().copied());
            assert_eq!(sim.rest[i], by_name, "{name}");
            assert_eq!(by_name.is_none(), absent.contains(&i), "{name}");
        }
        assert_eq!(param(sim, None, P::Velocity, 9.0), P::Velocity as usize as f32 + 0.5);
        assert_eq!(param(sim, None, P::Drag, 1.0), 1.0, "the default where the model has none");
        assert_eq!(param_opt(sim, None, P::LightningSubdiv), None);
        assert_eq!(color(sim, None, false, Vec3::ONE), Vec3::new(0.1, 0.2, 0.3));
        assert_eq!(color(sim, None, true, Vec3::ONE), Vec3::ONE, "two values are no color");
        // An animation's key decides, at its time.
        let key = Controller {
            name: "velocity".into(),
            times: vec![0.0, 1.0],
            values: vec![2.0, 4.0],
            columns: 1,
            ..Default::default()
        };
        let node = AnimNode { name: "EM".into(), controllers: vec![key], ..Default::default() };
        assert_eq!(param(sim, Some((&node, 0.5)), P::Velocity, 9.0), 3.0);
        assert_eq!(param(sim, Some((&node, 0.5)), P::Mass, 9.0), P::Mass as usize as f32 + 0.5);
        // (Words that are none of them: no particles; facing the camera.)
        assert_eq!((Update::of("Fountains"), Render::of("")), (Update::Other, Render::Normal));
        assert_eq!(blend_of("LIGHTEN"), ParticleBlend::Lighten);
    }

    /// A looping Single emitter's one particle never dies: it goes with
    /// its emitter when the object is moved or turned (it stayed where it
    /// was born until the area was read again). A fountain's particles
    /// stay where they were born, as before.
    #[test]
    fn a_looping_single_particle_goes_with_its_emitter() {
        let single = Emitter {
            update: "Single".into(),
            render: "Normal".into(),
            looping: true,
            ..Default::default()
        };
        let m = emitter(single, &[("sizestart", 0.5), ("xsize", 200.0)]);
        let pose = crate::rest_pose(&m);
        let view = Mat4::look_at_rh(Vec3::new(0.0, -20.0, 1.0), Vec3::ZERO, Vec3::Z);
        let middle = |p: &Particles, at: Mat4| {
            let b = p.batches(&m, None, 0.0, &pose, at, view);
            assert_eq!(b[0].vertices.len(), 6, "one particle");
            // (A quad's two triangles: its corners 0, 1, 2 and 5.)
            let v = |i: usize| Vec3::from(b[0].vertices[i].pos);
            (v(0) + v(1) + v(2) + v(5)) / 4.0
        };
        let here = Mat4::from_translation(Vec3::new(1.0, 2.0, 0.0));
        let mut p = Particles::new(&m);
        p.update(&m, None, 0.0, 0.1, &pose, here);
        let born = middle(&p, here);
        // (Somewhere along its emitter's two meters of X.)
        let local = born - Vec3::new(1.0, 2.0, 0.0);
        assert!(local.x.abs() <= 1.0 && local.y.abs() < 1e-4 && local.z.abs() < 1e-4, "{local}");
        // Dragged 5 m and turned a quarter: the particle where its emitter
        // has it now.
        let there = Mat4::from_translation(Vec3::new(6.0, 2.0, 0.0))
            * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        p.update(&m, None, 0.0, 0.1, &pose, there);
        let moved = middle(&p, there);
        let want = there.transform_point3(local);
        assert!(moved.abs_diff_eq(want, 1e-4), "{moved} for {want}");
        // And still, where the emitter stays.
        p.update(&m, None, 0.0, 0.1, &pose, there);
        assert!(middle(&p, there).abs_diff_eq(want, 1e-4));
        // A fountain's stay where they were born.
        let f = fountain_with("Normal", &[("velocity", 0.0)]);
        let pose = crate::rest_pose(&f);
        let mut p = Particles::new(&f);
        p.update(&f, None, 0.0, 0.2, &pose, here);
        let before: Vec<Vec3> = p.emitters[0].particles.iter().map(|q| q.pos).collect();
        assert!(!before.is_empty());
        p.update(&f, None, 0.0, 0.0, &pose, there);
        let after: Vec<Vec3> = p.emitters[0].particles.iter().map(|q| q.pos).collect();
        assert_eq!(after[..before.len()], before[..]);
    }

    /// Simulations of one model made with different seeds move each by
    /// itself (with the one seed, an area's torches were copies of one
    /// another, moved over); the same seed repeats, and a simulation made
    /// without one is the same each time, as the pictures' tests need.
    #[test]
    fn a_seed_gives_a_simulation_its_own_particles() {
        let m = fountain_with("Normal", &[("spread", 3.0), ("randvel", 2.0), ("xsize", 50.0)]);
        let pose = crate::rest_pose(&m);
        let run = |mut p: Particles| -> Vec<Vec3> {
            for _ in 0..10 {
                p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
            }
            p.emitters[0].particles.iter().map(|q| q.pos).collect()
        };
        let (a, b) = (run(Particles::new_seeded(&m, 1)), run(Particles::new_seeded(&m, 2)));
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(p, q)| !p.abs_diff_eq(*q, 1e-3)), "each its own");
        assert_eq!(run(Particles::new_seeded(&m, 1)), a, "the same seed repeats");
        assert_eq!(run(Particles::new(&m)), run(Particles::new(&m)));
        assert_ne!(run(Particles::new_seeded(&m, 0)), a);
        // (Seeds one apart, as objects' numbers are: no two alike.)
        let firsts: Vec<Vec3> =
            (0..64).map(|seed| run(Particles::new_seeded(&m, seed))[0]).collect();
        for (i, p) in firsts.iter().enumerate() {
            assert!(firsts[..i].iter().all(|q| !q.abs_diff_eq(*p, 1e-4)), "seed {i}");
        }
    }

    #[test]
    fn chunks_and_trails() {
        let chunky = Emitter {
            update: "Fountain".into(),
            render: "Normal".into(),
            chunk: Some("Plc_Chunk_W01".into()),
            ..Default::default()
        };
        let m = emitter(
            chunky,
            &[("birthrate", 10.0), ("lifeexp", 5.0), ("sizestart", 0.1), ("sizeend", 0.1)],
        );
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        for _ in 0..10 {
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        }
        let chunks = p.chunks(&m, None, 0.0, &pose, Mat4::IDENTITY);
        assert!(!chunks.is_empty() && chunks.iter().all(|c| c.model == "plc_chunk_w01"));
        let (scale, _, _) = chunks[0].transform.to_scale_rotation_translation();
        assert!((scale - Vec3::splat(0.1)).abs().max_element() < 1e-4);
        // Chunks draw as models, not quads.
        let view = Mat4::look_at_rh(Vec3::new(0.0, -5.0, 1.0), Vec3::ZERO, Vec3::Z);
        assert!(p.batches(&m, None, 0.0, &pose, Mat4::IDENTITY, view).is_empty());

        // A trail: birthrate per metre moved, none standing still.
        let trail = Emitter {
            update: "Fountain".into(),
            render: "Normal".into(),
            spawntype: 1,
            ..Default::default()
        };
        let m = emitter(trail, &[("birthrate", 5.0), ("lifeexp", 10.0)]);
        let mut p = Particles::new(&m);
        for i in 0..=10 {
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
            let _ = i;
        }
        assert!(p.emitters[0].particles.is_empty());
        for i in 1..=10 {
            let at = Mat4::from_translation(Vec3::new(i as f32 * 0.2, 0.0, 0.0));
            p.update(&m, None, 0.0, 0.1, &pose, at);
        }
        let n = p.emitters[0].particles.len();
        assert!((9..=10).contains(&n), "{n} particles over 2 m");
    }

    /// A chunk is born facing as its emitter does and turns about the
    /// emitter's z by `particlerot` radians a second, anticlockwise seen
    /// from above, as the client turns one (`reading_chunk_spin`: at 0.25,
    /// a chunk a second rising made a stair whose steps turn 14°); it was
    /// taken as turns a second, about an axis and from an angle of each
    /// chunk's own.
    #[test]
    fn a_chunk_turns_particlerot_radians_a_second_about_its_emitter_s_z() {
        let chunky = Emitter {
            update: "Fountain".into(),
            render: "Normal".into(),
            chunk: Some("mgchunk".into()),
            ..Default::default()
        };
        let m = emitter(
            chunky,
            &[("birthrate", 1.0), ("lifeexp", 10.0), ("velocity", 0.25), ("particlerot", 0.25)],
        );
        let pose = crate::rest_pose(&m);
        let mut p = Particles::new(&m);
        for _ in 0..40 {
            p.update(&m, None, 0.0, 0.1, &pose, Mat4::IDENTITY);
        }
        // Placed facing east (a quarter turn): born facing east too.
        let placed = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        for (transform, facing) in [(Mat4::IDENTITY, 0.0), (placed, std::f32::consts::FRAC_PI_2)] {
            let chunks = p.chunks(&m, None, 0.0, &pose, transform);
            let ages: Vec<f32> = p.emitters[0].particles.iter().map(|q| q.age).collect();
            assert!(ages.len() >= 3 && ages.iter().any(|&a| a > 2.5), "{ages:?}");
            for (c, age) in chunks.iter().zip(ages) {
                let (_, rot, _) = c.transform.to_scale_rotation_translation();
                let want = Quat::from_rotation_z(facing + 0.25 * age);
                assert!(rot.angle_between(want) < 1e-3, "at {age} s: {rot:?}, not {want:?}");
            }
        }
    }

    /// A linked particle's piece runs its cell past the next particle, as
    /// the client draws it (`batch6_ribbon_sizes`, at a metre's gap: 0.93,
    /// 0.87, 0.82, 0.77, 0.69 and 0.63 at sizes 0.15 to 1.2 m), where the
    /// next piece starts its own.
    #[test]
    fn a_ribbon_s_cell_runs_half_a_particle_past_the_next() {
        for (size, want) in
            [(0.15, 0.93), (0.3, 0.87), (0.45, 0.82), (0.6, 0.77), (0.9, 0.69), (1.2, 0.63)]
        {
            let got = reach(1.0, 2.0 * size);
            assert!((got - want).abs() < 0.01, "{size}: {got}");
        }
        assert_eq!(reach(0.0, 0.0), 1.0);
        let points = [
            (Vec3::ZERO, 1.2, [1.0; 4]),
            (Vec3::new(1.0, 0.0, 0.0), 1.2, [1.0; 4]),
            (Vec3::new(2.0, 0.0, 0.0), 1.2, [1.0; 4]),
        ];
        let cells = [(Vec2::new(0.0, 0.5), Vec2::new(0.5, 1.0)); 3];
        let v = strip(&points, Some(&cells), Vec3::new(0.0, 0.0, 10.0));
        assert_eq!(v.len(), 12);
        // (Each piece: its older particle at the cell's start, the next
        // 0.77 of the way along it.)
        for piece in v.chunks(6) {
            assert_eq!(piece[0].uv, [0.0, 0.5]);
            assert!((piece[2].uv[1] - (0.5 + 0.5 * 0.769)).abs() < 1e-3, "{:?}", piece[2].uv);
        }
    }
}
