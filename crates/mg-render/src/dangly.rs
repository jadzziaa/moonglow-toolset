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

use crate::model::{GpuModel, Vertex, mesh_vertices};
use crate::scene::MeshState;

/// The longest simulation step, seconds.
const STEP: f32 = 1.0 / 120.0;
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
            if d.period >= 60.0 || d.period <= 0.0 || d.displacement <= 0.0 {
                continue;
            }
            let limit: Vec<f32> = (0..m.vertices.len())
                .map(|v| {
                    let c = m.source.get(v).and_then(|&s| d.constraints.get(s as usize));
                    d.displacement * 0.5 * c.copied().unwrap_or(0.0) / 255.0
                })
                .collect();
            if limit.iter().all(|&l| l <= 0.0) {
                continue;
            }
            meshes.push(DanglyMesh {
                mesh: i,
                node: model.meshes[i].node,
                rest: mesh_vertices(m),
                limit,
                omega: d.period,
                zeta: (d.tightness / 10.0).clamp(0.05, 1.0),
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
            let steps = (dt / STEP).ceil().clamp(1.0, 240.0) as usize;
            let h = dt / steps as f32;
            let (k, c) = (d.omega * d.omega, 2.0 * d.zeta * d.omega);
            for ((p, v), (&a, &limit)) in
                d.pos.iter_mut().zip(&mut d.vel).zip(anchors.iter().zip(&d.limit))
            {
                if limit <= 0.0 {
                    (*p, *v) = (a, Vec3::ZERO);
                    continue;
                }
                for _ in 0..steps {
                    let offset = *p - a;
                    *v += (wind - k * offset - c * *v) * h;
                    *p += *v * h;
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
}
