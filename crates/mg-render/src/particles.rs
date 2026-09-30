//! Particle emitters, simulated on the CPU and drawn as billboards. What
//! the wiki's emitter page and the game's models say:
//!
//! - Fountain emitters spawn `birthrate` particles a second from a
//!   `xsize` × `ysize` cm rectangle, along the emitter's +Z within a cone of
//!   `spread` radians, at `velocity` plus up to `randvel` m/s; Single ones
//!   keep one particle; Explosion ones spawn `birthrate` at once when their
//!   `detonate` key passes. Lightning and point-to-point emitters, and
//!   chunk (model) particles, are not drawn yet.
//! - Particles live `lifeexp` seconds, fall with `mass` (× 9.8 m/s²; negative
//!   rises), spin `particlerot` turns a second, and go from `colorstart`,
//!   `alphastart` and `sizestart` (metres) to the end values. Textures are
//!   `xgrid` × `ygrid` flip-books played from `framestart` to `frameend` at
//!   `fps` (from a random frame with the `random` flag).
//! - `inherit`/`inherit_local` particles move with the emitter; others stay
//!   where they were born.
//!
//! The gravity scale is a guess: the wiki only says positive mass falls.

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
}

#[derive(Debug, Clone)]
struct Particle {
    /// World space, or the emitter's space for inheriting emitters.
    pos: Vec3,
    vel: Vec3,
    age: f32,
    life: f32,
    rot: f32,
    frame0: f32,
}

#[derive(Debug, Clone)]
struct EmitterSim {
    node: usize,
    particles: Vec<Particle>,
    /// Fractional particles owed.
    owed: f32,
    last_time: Option<f32>,
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

/// The particles of one model instance.
#[derive(Debug, Clone)]
pub struct Particles {
    emitters: Vec<EmitterSim>,
    rng: Rng,
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
            .map(|(node, _)| EmitterSim { node, particles: Vec::new(), owed: 0.0, last_time: None })
            .collect();
        Particles { emitters, rng: Rng(0x9E37_79B9_7F4A_7C15) }
    }

    pub fn is_empty(&self) -> bool {
        self.emitters.is_empty()
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
        let t = anim.map_or(time, |a| if a.length > 0.0 { time.rem_euclid(a.length) } else { 0.0 });
        let at = anim.map(|a| (a, t));
        for e in &mut self.emitters {
            let node = &model.nodes[e.node];
            let NodeKind::Emitter(em) = &node.kind else { continue };
            let p = |name, default| param(model, e.node, at, name, default);
            let world = transform * pose.get(e.node).copied().unwrap_or(Mat4::IDENTITY);
            let inherit = em.flags & (0x40 | 0x100) != 0;
            let life = p("lifeexp", 1.0);
            let gravity = Vec3::new(0.0, 0.0, -9.8 * p("mass", 0.0));
            // Age and move.
            e.particles.retain_mut(|q| {
                q.age += dt;
                q.vel += gravity * dt;
                q.pos += q.vel * dt;
                q.life < 0.0 || q.age < q.life
            });
            // Spawn.
            let update = em.update.to_ascii_lowercase();
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
                    e.owed += p("birthrate", 0.0).max(0.0) * dt;
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
        let t = anim.map_or(time, |a| if a.length > 0.0 { time.rem_euclid(a.length) } else { 0.0 });
        let at = anim.map(|a| (a, t));
        let inv_view = view.inverse();
        let eye = inv_view.w_axis.truncate();
        let (cam_right, cam_up) = (inv_view.x_axis.truncate(), inv_view.y_axis.truncate());
        let mut out = Vec::new();
        for e in &self.emitters {
            let node = &model.nodes[e.node];
            let NodeKind::Emitter(em) = &node.kind else { continue };
            if e.particles.is_empty() || em.chunk.is_some() {
                continue;
            }
            let p = |name, default| param(model, e.node, at, name, default);
            let world = transform * pose.get(e.node).copied().unwrap_or(Mat4::IDENTITY);
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
            let blend = match em.blend.to_ascii_lowercase().as_str() {
                "lighten" => ParticleBlend::Lighten,
                "punch-through" | "punchthrough" => ParticleBlend::PunchThrough,
                _ => ParticleBlend::Normal,
            };
            if blend == ParticleBlend::Normal {
                quads.sort_by(|a, b| b.0.total_cmp(&a.0));
            }
            out.push(ParticleBatch {
                texture: em.texture.clone(),
                blend,
                render_order: em.render_order,
                vertices: quads.into_iter().flat_map(|(_, q)| q).collect(),
            });
        }
        out
    }
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
}
