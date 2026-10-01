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
//!   the size, tumbling at `particlerot` turns a second: [`Particles::chunks`].
//! - Particles live `lifeexp` seconds, fall with `mass` (× 9.8 m/s²; negative
//!   rises), spin `particlerot` turns a second, and go from `colorstart`,
//!   `alphastart` and `sizestart` (metres) to the end values. Textures are
//!   `xgrid` × `ygrid` flip-books played from `framestart` to `frameend` at
//!   `fps` (from a random frame with the `random` flag).
//! - `inherit`/`inherit_local` particles move with the emitter; others stay
//!   where they were born.
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
//! of aligned particles show). The drag and Bezier details and the
//! lightning shape are guesses from the wiki's descriptions; wind
//! (`affectedByWind`), `splat` and `deadspace` are not simulated.

use glam::{Mat4, Quat, Vec2, Vec3};
use mg_mdl::{Animation, Model, NodeKind};

use crate::anim::sample;

/// How a batch of particles blends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleBlend {
    /// Alpha blending, drawn back to front.
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
}

impl ParticleVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<ParticleVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4],
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
}

#[derive(Debug, Clone)]
struct Particle {
    /// World space, or the emitter's space for inheriting emitters.
    pos: Vec3,
    /// Where it was born (same space), for Bezier paths.
    origin: Vec3,
    vel: Vec3,
    age: f32,
    life: f32,
    rot: f32,
    frame0: f32,
}

#[derive(Debug, Clone)]
struct EmitterSim {
    node: usize,
    /// The reference child that point-to-point and lightning emitters aim at.
    target: Option<usize>,
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
fn param(
    model: &Model,
    node: usize,
    anim: Option<(&Animation, f32)>,
    name: &str,
    default: f32,
) -> f32 {
    param_v(model, node, anim, name).and_then(|v| v.first().copied()).unwrap_or(default)
}

fn param_v(
    model: &Model,
    node: usize,
    anim: Option<(&Animation, f32)>,
    name: &str,
) -> Option<Vec<f32>> {
    let n = &model.nodes[node];
    if let Some((a, t)) = anim
        && let Some(an) = a.nodes.iter().find(|x| x.name.eq_ignore_ascii_case(&n.name))
        && let Some(c) = an.controllers.iter().find(|c| c.name == name)
    {
        return Some(sample(c, t));
    }
    n.value(name).map(<[f32]>::to_vec)
}

fn color3(v: Option<Vec<f32>>, default: Vec3) -> Vec3 {
    match v.as_deref() {
        Some([r, g, b, ..]) => Vec3::new(*r, *g, *b),
        _ => default,
    }
}

/// The most particles one emitter keeps.
const MAX_PARTICLES: usize = 2000;

impl Particles {
    pub fn new(model: &Model) -> Particles {
        let emitters = model
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| matches!(n.kind, NodeKind::Emitter(_)))
            .map(|(node, n)| EmitterSim {
                node,
                target: n
                    .children
                    .iter()
                    .copied()
                    .find(|&c| matches!(model.nodes[c].kind, NodeKind::Reference(_))),
                particles: Vec::new(),
                owed: 0.0,
                last_time: None,
                last_pos: None,
                bolt: Vec::new(),
                bolt_age: 0.0,
            })
            .collect();
        Particles { emitters, rng: Rng(0x9E37_79B9_7F4A_7C15), ground: 0.0 }
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
            let p = |name, default| param(model, e.node, at, name, default);
            let world = transform * pose.get(e.node).copied().unwrap_or(Mat4::IDENTITY);
            let inherit = em.flags & (0x40 | 0x100) != 0;
            let bounce = (em.flags & 0x10 != 0 && !inherit).then(|| p("bounce_co", 0.0));
            let life = p("lifeexp", 1.0);
            let gravity = Vec3::new(0.0, 0.0, -9.8 * p("mass", 0.0));
            let update = em.update.to_ascii_lowercase();
            let target_world = e.target.map(|t| {
                transform.transform_point3(pose.get(t).map_or(Vec3::ZERO, |m| m.w_axis.truncate()))
            });
            let here = world.w_axis.truncate();

            // Lightning: a new bolt now and then.
            if update == "lightning" {
                e.bolt_age += dt;
                let delay = p("lightningdelay", 0.0).max(0.0);
                if let Some(target) = target_world
                    && (e.bolt.is_empty() || e.bolt_age >= delay)
                {
                    e.bolt_age = 0.0;
                    let rng = &mut self.rng;
                    let radius = p("lightningradius", 0.0).max(0.0);
                    let end = target
                        + Vec3::new(
                            rng.range(-1.0, 1.0),
                            rng.range(-1.0, 1.0),
                            rng.range(-1.0, 1.0),
                        ) * radius;
                    let subdiv = param_v(model, e.node, at, "lightningsubdiv")
                        .and_then(|v| v.first().copied())
                        .unwrap_or_else(|| p("birthrate", 0.0))
                        .clamp(0.0, 9.0) as u32;
                    let scale = p("lightningscale", 0.0).max(0.0);
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
            let (src, trg) = (p("p2p_bezier2", 0.0), p("p2p_bezier3", 0.0));
            let travel = if p("combinetime", 0.0) > 0.0 { p("combinetime", 0.0) } else { life };
            let (grav, drag, threshold) = (p("grav", 0.0), p("drag", 1.0), p("threshold", 0.0));

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
            let count = match update.as_str() {
                "single" => usize::from(e.particles.is_empty()),
                "explosion" => {
                    // A detonate key between the last time and now.
                    let keys = anim
                        .and_then(|a| {
                            a.nodes.iter().find(|x| x.name.eq_ignore_ascii_case(&node.name))
                        })
                        .and_then(|an| an.controllers.iter().find(|c| c.name == "detonate"));
                    let fired = match (keys, e.last_time) {
                        (Some(c), Some(last)) => c.times.iter().any(|&k| {
                            if last <= t { last < k && k <= t } else { k > last || k <= t }
                        }),
                        _ => false,
                    };
                    if fired { p("birthrate", 0.0).max(0.0) as usize } else { 0 }
                }
                "fountain" => {
                    e.owed += p("birthrate", 0.0).max(0.0) * if trail { moved } else { dt };
                    let n = e.owed.floor();
                    e.owed -= n;
                    n as usize
                }
                _ => 0,
            };
            e.last_time = Some(t);
            let (xs, ys) = (p("xsize", 0.0) / 100.0, p("ysize", 0.0) / 100.0);
            let spread = p("spread", 0.0);
            let (v0, rv) = (p("velocity", 0.0), p("randvel", 0.0));
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
                    vel: if update == "single" { Vec3::ZERO } else { vel },
                    age: 0.0,
                    life: if update == "single" && em.looping { -1.0 } else { life },
                    rot: rng.range(0.0, std::f32::consts::TAU),
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
        let blend_of = |em: &mg_mdl::Emitter| match em.blend.to_ascii_lowercase().as_str() {
            "lighten" => ParticleBlend::Lighten,
            "punch-through" | "punchthrough" => ParticleBlend::PunchThrough,
            _ => ParticleBlend::Normal,
        };
        for e in &self.emitters {
            let node = &model.nodes[e.node];
            let NodeKind::Emitter(em) = &node.kind else { continue };
            if e.bolt.len() >= 2 {
                let p = |name, default| param(model, e.node, at, name, default);
                let width = if p("sizestart", 0.0) > 0.0 { p("sizestart", 0.0) } else { 0.1 };
                let c = color3(param_v(model, e.node, at, "colorstart"), Vec3::ONE);
                let color = [c.x, c.y, c.z, p("alphastart", 1.0)];
                let points: Vec<(Vec3, f32, [f32; 4])> =
                    e.bolt.iter().map(|&q| (q, width, color)).collect();
                out.push(ParticleBatch {
                    texture: em.texture.clone(),
                    blend: blend_of(em),
                    render_order: em.render_order,
                    vertices: strip(&points, eye),
                    tint: None,
                });
                continue;
            }
            if e.particles.is_empty() || em.chunk.is_some() {
                continue;
            }
            let p = |name, default| param(model, e.node, at, name, default);
            let world = transform * pose.get(e.node).copied().unwrap_or(Mat4::IDENTITY);
            let tint = (em.flags & 0x8 != 0).then(|| world.w_axis.truncate());
            let inherit = em.flags & (0x40 | 0x100) != 0;
            let (c0, c1) = (
                color3(param_v(model, e.node, at, "colorstart"), Vec3::ONE),
                color3(param_v(model, e.node, at, "colorend"), Vec3::ONE),
            );
            let (a0, a1) = (p("alphastart", 1.0), p("alphaend", 1.0));
            let (s0, s1) = (p("sizestart", 1.0), p("sizeend", 1.0));
            let (sy0, sy1) = (p("sizestart_y", 0.0), p("sizeend_y", 0.0));
            let rot_speed = p("particlerot", 0.0) * std::f32::consts::TAU;
            let (fps, f0, f1) = (p("fps", 0.0), p("framestart", 0.0), p("frameend", 0.0));
            let (gx, gy) = (em.xgrid.max(1), em.ygrid.max(1));
            let frames = (f1 - f0 + 1.0).max(1.0);
            let render = em.render.to_ascii_lowercase();
            if render == "linked" {
                // One strip through the particles, oldest first.
                let mut order: Vec<&Particle> = e.particles.iter().collect();
                order.sort_by(|a, b| b.age.total_cmp(&a.age));
                let points: Vec<(Vec3, f32, [f32; 4])> = order
                    .iter()
                    .map(|q| {
                        let k = if q.life > 0.0 { (q.age / q.life).clamp(0.0, 1.0) } else { 0.0 };
                        let pos = if inherit { world.transform_point3(q.pos) } else { q.pos };
                        let c = c0.lerp(c1, k);
                        (pos, s0 + (s1 - s0) * k, [c.x, c.y, c.z, a0 + (a1 - a0) * k])
                    })
                    .collect();
                out.push(ParticleBatch {
                    texture: em.texture.clone(),
                    blend: blend_of(em),
                    render_order: em.render_order,
                    vertices: strip(&points, eye),
                    tint,
                });
                continue;
            }
            let mut quads: Vec<(f32, [ParticleVertex; 6])> = Vec::new();
            for q in &e.particles {
                let k = if q.life > 0.0 { (q.age / q.life).clamp(0.0, 1.0) } else { 0.0 };
                let pos = if inherit { world.transform_point3(q.pos) } else { q.pos };
                let color = c0.lerp(c1, k);
                let alpha = a0 + (a1 - a0) * k;
                let sx = s0 + (s1 - s0) * k;
                let sy = if sy0 > 0.0 || sy1 > 0.0 { sy0 + (sy1 - sy0) * k } else { sx };
                let frame = (f0 + (q.frame0 + q.age * fps).floor() % frames).min(f1.max(f0)) as u32;
                let (fx, fy) = (frame % gx, frame / gx % gy);
                let uv0 = Vec2::new(fx as f32 / gx as f32, fy as f32 / gy as f32);
                let uv1 = uv0 + Vec2::new(1.0 / gx as f32, 1.0 / gy as f32);
                let angle = q.rot + rot_speed * q.age;
                let (right, up) = match render.as_str() {
                    "billboard_to_world_z" => {
                        let to_eye = (eye - pos) * Vec3::new(1.0, 1.0, 0.0);
                        let r = Vec3::Z.cross(to_eye).normalize_or(Vec3::X);
                        (r, Vec3::Z)
                    }
                    "billboard_to_local_z" => {
                        let z = world.transform_vector3(Vec3::Z).normalize_or(Vec3::Z);
                        let r = z.cross(eye - pos).normalize_or(Vec3::X);
                        (r, z)
                    }
                    "aligned_to_world_z" => (Vec3::X, Vec3::Y),
                    "motion_blur" | "aligned_to_particle_dir" => {
                        let world_vel =
                            if inherit { world.transform_vector3(q.vel) } else { q.vel };
                        let d = world_vel.normalize_or(Vec3::Z);
                        let r = d.cross(eye - pos).normalize_or(Vec3::X);
                        (r, d)
                    }
                    _ => {
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
                };
                let (bl, br, tr, tl) = (pos - hx - hy, pos + hx - hy, pos + hx + hy, pos - hx + hy);
                let quad = [
                    v(bl, Vec2::new(uv0.x, uv0.y)),
                    v(br, Vec2::new(uv1.x, uv0.y)),
                    v(tr, Vec2::new(uv1.x, uv1.y)),
                    v(bl, Vec2::new(uv0.x, uv0.y)),
                    v(tr, Vec2::new(uv1.x, uv1.y)),
                    v(tl, Vec2::new(uv0.x, uv1.y)),
                ];
                quads.push((pos.distance_squared(eye), quad));
            }
            let blend = blend_of(em);
            if blend == ParticleBlend::Normal {
                quads.sort_by(|a, b| b.0.total_cmp(&a.0));
            }
            out.push(ParticleBatch {
                texture: em.texture.clone(),
                blend,
                render_order: em.render_order,
                vertices: quads.into_iter().flat_map(|(_, q)| q).collect(),
                tint,
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
            let Some(chunk) = &em.chunk else { continue };
            let p = |name, default| param(model, e.node, at, name, default);
            let world = transform * pose.get(e.node).copied().unwrap_or(Mat4::IDENTITY);
            let inherit = em.flags & (0x40 | 0x100) != 0;
            let (s0, s1) = (p("sizestart", 1.0), p("sizeend", 1.0));
            let spin = p("particlerot", 0.0) * std::f32::consts::TAU;
            for q in &e.particles {
                let k = if q.life > 0.0 { (q.age / q.life).clamp(0.0, 1.0) } else { 0.0 };
                let pos = if inherit { world.transform_point3(q.pos) } else { q.pos };
                // A tumbling axis fixed per particle (from its random angle).
                let axis = Vec3::new(q.rot.cos(), q.rot.sin(), 0.5).normalize();
                let rot = Quat::from_axis_angle(axis, q.rot + spin * q.age);
                out.push(Chunk {
                    model: chunk.to_ascii_lowercase(),
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

/// Camera-facing quads joining points (position, width, colour), the
/// texture's v running along the strip.
fn strip(points: &[(Vec3, f32, [f32; 4])], eye: Vec3) -> Vec<ParticleVertex> {
    let mut out = Vec::new();
    let n = points.len().saturating_sub(1).max(1) as f32;
    for (i, w) in points.windows(2).enumerate() {
        let ((a, wa, ca), (b, wb, cb)) = (w[0], w[1]);
        let side =
            |p: Vec3, width: f32| (b - a).cross(eye - p).normalize_or(Vec3::X) * (width * 0.5);
        let (sa, sb) = (side(a, wa), side(b, wb));
        let (va, vb) = (i as f32 / n, (i + 1) as f32 / n);
        let v = |p: Vec3, u: f32, v: f32, c: [f32; 4]| ParticleVertex {
            pos: p.to_array(),
            uv: [u, v],
            color: c,
        };
        out.extend([
            v(a - sa, 0.0, va, ca),
            v(a + sa, 1.0, va, ca),
            v(b + sb, 1.0, vb, cb),
            v(a - sa, 0.0, va, ca),
            v(b + sb, 1.0, vb, cb),
            v(b - sb, 0.0, vb, cb),
        ]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_mdl::{Controller, Emitter, Node};

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
}
