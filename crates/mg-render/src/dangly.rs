//! Dangly meshes: vertices that lag behind their mesh when it moves and
//! spring back (hair, cloaks, pennants). The game simulates them on the
//! CPU and its exact dynamics are not documented, so this is an
//! approximation from the MDL fields' descriptions:
//!
//! - each vertex is a damped spring anchored at its place on the moving
//!   mesh, simulated in world space;
//! - `period` is the spring's angular frequency (rad/s): higher is quicker.
//!   From 60 up the game locks the mesh, and so does this;
//! - `tightness` damps the motion (a damping ratio of tightness / 10);
//! - a vertex strays at most `displacement` × 0.5 m × its constraint / 255
//!   from its anchor (constraint 0: fixed).

use glam::{Mat4, Vec3};
use mg_mdl::MeshExtra;

use crate::model::{GpuModel, Vertex, uploaded};
use crate::scene::MeshState;

/// The longest simulation step, seconds.
const STEP: f32 = 1.0 / 120.0;
/// The most steps an update takes: no more than two seconds are simulated
/// at once (past them the steps grew, and near a period of 60 the spring
/// then ran away).
const STEPS: f32 = 240.0;
/// Below this speed (m/s) a vertex is at rest.
const REST: f32 = 1e-4;

/// The dangly meshes of one model instance.
#[derive(Debug, Clone, Default)]
pub struct Dangly {
    meshes: Vec<DanglyMesh>,
}

#[derive(Debug, Clone)]
struct DanglyMesh {
    /// Index in [`GpuModel::meshes`].
    mesh: usize,
    node: usize,
    rest: Vec<Vertex>,
    /// Per vertex: how far it may stray, metres.
    limit: Vec<f32>,
    omega: f32,
    zeta: f32,
    /// World-space positions and velocities; empty until the first update.
    pos: Vec<Vec3>,
    vel: Vec<Vec3>,
}

impl Dangly {
    /// The dangly meshes of `model` (none that are locked or fixed).
    pub fn new(model: &GpuModel) -> Dangly {
        let mut meshes = Vec::new();
        for i in 0..model.meshes.len() {
            let Some(m) = model.mesh_data(i) else { continue };
            let MeshExtra::Dangly(d) = &m.extra else { continue };
            let Some((omega, zeta)) = spring(d) else { continue };
            // (As uploaded: a copy's, on a mirrored seam, its original's.)
            let up = uploaded(m);
            let limit: Vec<f32> = up
                .originals
                .iter()
                .map(|&v| {
                    let c = m.source.get(v as usize).and_then(|&s| d.constraints.get(s as usize));
                    d.displacement * 0.5 * c.copied().unwrap_or(0.0) / 255.0
                })
                .collect();
            // (None that may stray: fixed. A constraint that is no number
            // strays nowhere.)
            if !limit.iter().any(|&l| l > 0.0) {
                continue;
            }
            meshes.push(DanglyMesh {
                mesh: i,
                node: model.meshes[i].node,
                rest: up.vertices,
                limit,
                omega,
                zeta,
                pos: Vec::new(),
                vel: Vec::new(),
            });
        }
        Dangly { meshes }
    }

    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
    }

    /// Advances `dt` seconds with the model at `pose` (model space) and
    /// `transform` (world), pushed by `wind` (m/s², world space).
    pub fn update(&mut self, dt: f32, pose: &[Mat4], transform: Mat4, wind: Vec3) {
        for d in &mut self.meshes {
            let world = transform * pose.get(d.node).copied().unwrap_or(Mat4::IDENTITY);
            let anchors: Vec<Vec3> =
                d.rest.iter().map(|v| world.transform_point3(Vec3::from(v.pos))).collect();
            if d.pos.len() != anchors.len() {
                d.pos = anchors.clone();
                d.vel = vec![Vec3::ZERO; anchors.len()];
                continue;
            }
            // (A time that is no number, or runs back, is none passed.)
            let dt = if dt.is_finite() { dt.clamp(0.0, STEPS * STEP) } else { 0.0 };
            let steps = (dt / STEP).ceil().clamp(1.0, STEPS) as usize;
            let h = dt / steps as f32;
            let (k, c) = (d.omega * d.omega, 2.0 * d.zeta * d.omega);
            for ((p, v), (&a, &limit)) in
                d.pos.iter_mut().zip(&mut d.vel).zip(anchors.iter().zip(&d.limit))
            {
                // (Fixed; so one whose limit is no number.)
                if limit.is_nan() || limit <= 0.0 {
                    (*p, *v) = (a, Vec3::ZERO);
                    continue;
                }
                for _ in 0..steps {
                    let offset = *p - a;
                    *v += (wind - k * offset - c * *v) * h;
                    *p += *v * h;
                }
                // (A vertex that is nowhere, its wind or its mesh having
                // been for a frame, is back at its anchor: it never came
                // back by itself, and the mesh was gone.)
                if !(p.is_finite() && v.is_finite()) {
                    (*p, *v) = (a, Vec3::ZERO);
                    continue;
                }
                // Held at the limit: no speed away from the anchor.
                let offset = *p - a;
                if offset.length() > limit {
                    let dir = offset.normalize();
                    *p = a + dir * limit;
                    *v -= dir * v.dot(dir).max(0.0);
                }
            }
        }
    }

    /// Whether any vertex is still moving.
    pub fn moving(&self) -> bool {
        self.meshes.iter().any(|d| d.vel.iter().any(|v| v.length() > REST))
    }

    /// Writes the meshes' current vertices (in their own space) into
    /// `state`, for the same `pose` and `transform` as the last update.
    pub fn apply(&self, state: &mut MeshState, pose: &[Mat4], transform: Mat4) {
        for d in &self.meshes {
            let Some(out) = state.meshes.get_mut(d.mesh) else { continue };
            if d.pos.len() != d.rest.len() {
                continue;
            }
            let world = transform * pose.get(d.node).copied().unwrap_or(Mat4::IDENTITY);
            let to_local = world.inverse();
            let vertices = d
                .rest
                .iter()
                .zip(&d.pos)
                .map(|(v, p)| Vertex { pos: to_local.transform_point3(*p).to_array(), ..*v })
                .collect();
            out.vertices = Some(vertices);
        }
    }

    /// The largest distance of a vertex from its anchor, metres.
    pub fn max_offset(&self, pose: &[Mat4], transform: Mat4) -> f32 {
        let mut max = 0.0f32;
        for d in &self.meshes {
            let world = transform * pose.get(d.node).copied().unwrap_or(Mat4::IDENTITY);
            for (v, p) in d.rest.iter().zip(&d.pos) {
                max = max.max(world.transform_point3(Vec3::from(v.pos)).distance(*p));
            }
        }
        max
    }
}

/// A dangly mesh's spring (its angular frequency, its damping ratio),
/// where it is simulated: its period above 0 and below 60 (from 60 up the
/// game locks the mesh), with somewhere to stray to, and its values
/// numbers (a `period nan` passed for one below 60, and every vertex was
/// nowhere from the second update on).
fn spring(d: &mg_mdl::Dangly) -> Option<(f32, f32)> {
    let simulated = d.period > 0.0
        && d.period < 60.0
        && d.displacement > 0.0
        && d.displacement.is_finite()
        && d.tightness.is_finite();
    simulated.then(|| (d.period, (d.tightness / 10.0).clamp(0.05, 1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(limit: Vec<f32>) -> DanglyMesh {
        let rest = limit
            .iter()
            .enumerate()
            .map(|(i, _)| Vertex {
                pos: [i as f32, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0; 2],
                tangent: [0.0; 4],
                color: [255; 4],
                uv1: [0.0; 2],
                uv2: [0.0; 2],
                uv3: [0.0; 2],
            })
            .collect();
        DanglyMesh {
            mesh: 0,
            node: 0,
            rest,
            limit,
            omega: 8.0,
            zeta: 0.3,
            pos: Vec::new(),
            vel: Vec::new(),
        }
    }

    #[test]
    fn vertices_lag_stay_within_limits_and_settle() {
        let mut d = Dangly { meshes: vec![mesh(vec![0.0, 0.05, 0.1])] };
        let pose = [Mat4::IDENTITY];
        let at = |x: f32| Mat4::from_translation(Vec3::new(0.0, x, 0.0));
        d.update(0.0, &pose, at(0.0), Vec3::ZERO);
        // A sudden move: the free vertices lag behind, the fixed one follows.
        d.update(1.0 / 60.0, &pose, at(1.0), Vec3::ZERO);
        let m = &d.meshes[0];
        assert_eq!(m.pos[0], Vec3::new(0.0, 1.0, 0.0));
        assert!(m.pos[1].y < 1.0 && m.pos[2].y < 1.0);
        for (p, (v, limit)) in m.pos.iter().zip(m.rest.iter().zip(&m.limit)) {
            let anchor = Vec3::from(v.pos) + Vec3::Y;
            assert!(p.distance(anchor) <= limit + 1e-5);
        }
        assert!(d.moving());
        // Held still, they come back to rest.
        for _ in 0..600 {
            d.update(1.0 / 60.0, &pose, at(1.0), Vec3::ZERO);
        }
        assert!(!d.moving());
        assert!(d.max_offset(&pose, at(1.0)) < 1e-3);
    }

    /// A long step at a period near 60 (a minute away from the window, by
    /// a caller that caps nothing) leaves every vertex somewhere, within
    /// its limit; a vertex made nowhere (a wind that is no number, for a
    /// frame) is back at its anchor at the next update; and a mesh whose
    /// values are no numbers is not simulated.
    #[test]
    fn a_long_step_or_a_value_that_is_no_number_loses_no_vertex() {
        let pose = [Mat4::IDENTITY];
        let at = |x: f32| Mat4::from_translation(Vec3::new(0.0, x, 0.0));
        let mut stiff = mesh(vec![0.0, 0.05, 0.1, f32::NAN]);
        (stiff.omega, stiff.zeta) = (59.0, 0.05);
        let mut d = Dangly { meshes: vec![stiff] };
        d.update(0.0, &pose, at(0.0), Vec3::ZERO);
        for step in [60.0, 8.0, f32::INFINITY, f32::NAN, -1.0, 60.0] {
            d.update(step, &pose, at(1.0), Vec3::new(0.0, 0.0, 3.0));
            let m = &d.meshes[0];
            for (p, (v, limit)) in m.pos.iter().zip(m.rest.iter().zip(&m.limit)) {
                // (A limit that is no number: fixed.)
                let limit = if limit.is_nan() { 0.0 } else { *limit };
                let anchor = Vec3::from(v.pos) + Vec3::Y;
                assert!(p.is_finite() && p.distance(anchor) <= limit + 1e-4, "{step}: {p}");
            }
            assert!(m.vel.iter().all(|v| v.is_finite()), "{step}");
        }
        // A wind that is no number for a frame.
        d.update(1.0 / 60.0, &pose, at(1.0), Vec3::NAN);
        d.update(1.0 / 60.0, &pose, at(1.0), Vec3::ZERO);
        assert!(d.meshes[0].pos.iter().all(|p| p.is_finite()));
        for _ in 0..600 {
            d.update(1.0 / 60.0, &pose, at(1.0), Vec3::ZERO);
        }
        assert!(!d.moving() && d.max_offset(&pose, at(1.0)) < 1e-3);
        // What is simulated: numbers alone.
        let dangly = |period: f32, displacement: f32, tightness: f32| {
            spring(&mg_mdl::Dangly { constraints: vec![255.0], displacement, tightness, period })
        };
        assert_eq!(dangly(8.0, 1.0, 3.0), Some((8.0, 0.3)));
        assert_eq!(dangly(8.0, 1.0, 0.0), Some((8.0, 0.05)));
        for (period, displacement, tightness) in [
            (f32::NAN, 1.0, 3.0),
            (60.0, 1.0, 3.0),
            (0.0, 1.0, 3.0),
            (f32::INFINITY, 1.0, 3.0),
            (8.0, f32::NAN, 3.0),
            (8.0, f32::INFINITY, 3.0),
            (8.0, 0.0, 3.0),
            (8.0, 1.0, f32::NAN),
            (8.0, 1.0, f32::INFINITY),
        ] {
            assert_eq!(dangly(period, displacement, tightness), None, "{period} {displacement}");
        }
    }
}
