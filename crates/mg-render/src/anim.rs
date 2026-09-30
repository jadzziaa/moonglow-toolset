//! Poses: node transforms from an animation at a moment. Animations bind to
//! the model's nodes by name (case-insensitive); a model plays its own
//! animations and, for names it lacks, its supermodels' (nearest first).
//! Keys interpolate linearly (orientations by slerp); a node the animation
//! does not key keeps its rest transform.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};
use mg_mdl::{AnimMeshSets, Animation, Controller, Mesh, Model};

use crate::model::{GpuModel, Vertex, local};
use crate::scene::MeshState;

/// A controller's value at `t` (clamped to its keys).
pub fn sample(c: &Controller, t: f32) -> Vec<f32> {
    let n = c.times.len();
    if n == 0 || c.columns == 0 {
        return Vec::new();
    }
    let i = c.times.partition_point(|&k| k <= t);
    if i == 0 {
        return c.row(0).to_vec();
    }
    if i >= n {
        return c.row(n - 1).to_vec();
    }
    let (t0, t1) = (c.times[i - 1], c.times[i]);
    let f = if t1 > t0 { (t - t0) / (t1 - t0) } else { 0.0 };
    let (a, b) = (c.row(i - 1), c.row(i));
    if c.name == "orientation" && a.len() == 4 && b.len() == 4 {
        let qa = Quat::from_slice(a).normalize();
        let qb = Quat::from_slice(b).normalize();
        return qa.slerp(qb, f).to_array().to_vec();
    }
    a.iter().zip(b).map(|(x, y)| x + (y - x) * f).collect()
}

/// The animations a model can play: its own and its supermodels' (loaded
/// by `load`), each name once, nearest model first.
pub fn animations(
    model: &Arc<Model>,
    load: &dyn Fn(&str) -> Option<Arc<Model>>,
) -> Vec<(String, Arc<Model>)> {
    let mut out: Vec<(String, Arc<Model>)> = Vec::new();
    let mut current = Some(model.clone());
    let mut depth = 0;
    while let Some(m) = current {
        for a in &m.animations {
            if !out.iter().any(|(n, _)| n.eq_ignore_ascii_case(&a.name)) {
                out.push((a.name.clone(), m.clone()));
            }
        }
        depth += 1;
        current = if depth > 16 { None } else { m.supermodel.as_deref().and_then(load) };
    }
    out
}

/// The model-space transform of every node of `model` playing `anim` at
/// time `t` (seconds, wrapped to the animation's length).
pub fn pose(model: &Model, anim: &Animation, t: f32) -> Vec<Mat4> {
    let t = if anim.length > 0.0 { t.rem_euclid(anim.length) } else { 0.0 };
    let by_name: HashMap<String, usize> =
        anim.nodes.iter().enumerate().map(|(i, n)| (n.name.to_ascii_lowercase(), i)).collect();
    let mut out: Vec<Mat4> = Vec::with_capacity(model.nodes.len());
    for n in &model.nodes {
        let (mut pos, mut orient, mut scale) = (n.position, n.orientation, n.scale);
        if let Some(&i) = by_name.get(&n.name.to_ascii_lowercase()) {
            for c in &anim.nodes[i].controllers {
                let v = sample(c, t);
                match (c.name.as_str(), v.len()) {
                    ("position", 3) => pos = [v[0], v[1], v[2]],
                    ("orientation", 4) => orient = [v[0], v[1], v[2], v[3]],
                    ("scale", 1) => scale = v[0],
                    _ => {}
                }
            }
        }
        let l = local(pos, orient, scale);
        out.push(match n.parent {
            Some(p) => out[p] * l,
            None => l,
        });
    }
    out
}

/// The lights of a model instance: its light nodes at their posed
/// positions, with colour, radius and multiplier from the animation where it
/// keys them, else the rest values. Tile main lights (`…ml1`, `…ml2`) take
/// their colour from `main_light` (the area's tile settings) when given, and
/// then the engine's radius: 10 for main light 1, 5 for main light 2.
pub fn lights(
    model: &Model,
    anim: Option<&Animation>,
    t: f32,
    pose: &[Mat4],
    transform: Mat4,
    main_light: &dyn Fn(usize) -> Option<Vec3>,
) -> Vec<crate::scene::PointLight> {
    let t = anim.map_or(0.0, |a| if a.length > 0.0 { t.rem_euclid(a.length) } else { 0.0 });
    let by_name: HashMap<String, usize> = anim
        .map(|a| {
            a.nodes.iter().enumerate().map(|(i, n)| (n.name.to_ascii_lowercase(), i)).collect()
        })
        .unwrap_or_default();
    let mut out = Vec::new();
    for (i, n) in model.nodes.iter().enumerate() {
        let mg_mdl::NodeKind::Light(l) = &n.kind else { continue };
        let keyed = |name: &str| -> Option<Vec<f32>> {
            let a = anim?;
            let an = &a.nodes[*by_name.get(&n.name.to_ascii_lowercase())?];
            let c = an.controllers.iter().find(|c| c.name == name)?;
            Some(sample(c, t))
        };
        let value = |name: &str| keyed(name).or_else(|| n.value(name).map(<[f32]>::to_vec));
        let lower = n.name.to_ascii_lowercase();
        let main = if lower.ends_with("ml1") {
            Some(0)
        } else if lower.ends_with("ml2") {
            Some(1)
        } else {
            None
        };
        let tile = main.and_then(|slot| Some((slot, main_light(slot)?)));
        let color = match tile {
            Some((_, c)) => c,
            None => match value("color").as_deref() {
                Some([r, g, b, ..]) => Vec3::new(*r, *g, *b),
                _ => Vec3::ONE,
            },
        };
        let multiplier = value("multiplier").and_then(|v| v.first().copied()).unwrap_or(1.0);
        let radius = match tile {
            Some((0, _)) => 10.0,
            Some(_) => 5.0,
            None => value("radius").and_then(|v| v.first().copied()).unwrap_or(5.0),
        };
        if radius <= 0.0 || color == Vec3::ZERO {
            continue;
        }
        out.push(crate::scene::PointLight::new(
            transform.transform_point3(node_position(pose, i)),
            color * multiplier,
            radius,
            l.ambient_only,
            l.priority.clamp(1, 5),
        ));
    }
    out
}

/// What `anim` at `t` changes in `model`'s meshes: animated alpha and
/// self-illumination, and animated meshes' vertices. The vertex sets are
/// samples `sample_period` apart from the animation's start (the last at
/// its end), interpolated; normals are recomputed from the moved faces.
pub fn mesh_state(model: &GpuModel, anim: &Animation, t: f32) -> MeshState {
    let t = if anim.length > 0.0 { t.rem_euclid(anim.length) } else { 0.0 };
    let by_name: HashMap<String, usize> =
        anim.nodes.iter().enumerate().map(|(i, n)| (n.name.to_ascii_lowercase(), i)).collect();
    let mut state = MeshState::new(model);
    for (i, mesh) in model.meshes.iter().enumerate() {
        let name = model.model.nodes[mesh.node].name.to_ascii_lowercase();
        let Some(an) = by_name.get(&name).map(|&k| &anim.nodes[k]) else { continue };
        let out = &mut state.meshes[i];
        for c in &an.controllers {
            let v = sample(c, t);
            match (c.name.as_str(), v.as_slice()) {
                ("alpha", [a, ..]) => out.alpha = Some(*a),
                ("selfillumcolor", [r, g, b, ..]) => out.selfillum = Some(Vec3::new(*r, *g, *b)),
                _ => {}
            }
        }
        if let (Some(sets), Some(data)) = (&an.anim_mesh, model.mesh_data(i)) {
            out.vertices = animated_vertices(data, sets, t);
        }
    }
    state
}

/// An animated mesh's vertices at `t`.
fn animated_vertices(m: &Mesh, sets: &AnimMeshSets, t: f32) -> Option<Vec<Vertex>> {
    if sets.vertex_sets.is_empty() && sets.uv_sets.is_empty() {
        return None;
    }
    // Set `k` and the fraction towards `k + 1` at `t`.
    let at = |count: usize| -> (usize, usize, f32) {
        let f = if sets.sample_period > 0.0 { t / sets.sample_period } else { 0.0 };
        let last = count.saturating_sub(1);
        let k = (f.floor().max(0.0) as usize).min(last);
        (k, (k + 1).min(last), (f - k as f32).clamp(0.0, 1.0))
    };
    let mut out = crate::model::mesh_vertices(m);
    if !sets.vertex_sets.is_empty() {
        let (a, b, f) = at(sets.vertex_sets.len());
        let (a, b) = (&sets.vertex_sets[a], &sets.vertex_sets[b]);
        for (v, &s) in out.iter_mut().zip(&m.source) {
            let s = s as usize;
            if let (Some(p), Some(q)) = (a.get(s), b.get(s)) {
                v.pos = Vec3::from(*p).lerp(Vec3::from(*q), f).to_array();
            }
        }
        smooth_normals(m, &mut out);
    }
    if !sets.uv_sets.is_empty() {
        let (a, b, f) = at(sets.uv_sets.len());
        let (a, b) = (&sets.uv_sets[a], &sets.uv_sets[b]);
        let source = if m.source_uv.len() == out.len() { &m.source_uv } else { &m.source };
        for (v, &s) in out.iter_mut().zip(source) {
            let s = s as usize;
            if let (Some(p), Some(q)) = (a.get(s), b.get(s)) {
                v.uv = glam::Vec2::from(*p).lerp(glam::Vec2::from(*q), f).to_array();
            }
        }
    }
    Some(out)
}

/// Normals from the faces around each vertex. Vertices that had the same
/// position and normal at rest (split only by UV seams) share one, so hard
/// edges stay hard.
fn smooth_normals(m: &Mesh, out: &mut [Vertex]) {
    let mut class: HashMap<([u32; 3], [u32; 3]), usize> = HashMap::new();
    let classes: Vec<usize> = (0..out.len())
        .map(|i| {
            let p = m.vertices[i].map(f32::to_bits);
            let n = m.normals.get(i).copied().unwrap_or_default().map(f32::to_bits);
            let next = class.len();
            *class.entry((p, n)).or_insert(next)
        })
        .collect();
    let mut sum = vec![Vec3::ZERO; class.len()];
    for f in &m.faces {
        let [a, b, c] = f.vertices.map(|v| v as usize);
        if a.max(b).max(c) >= out.len() {
            continue;
        }
        let (pa, pb, pc) = (Vec3::from(out[a].pos), Vec3::from(out[b].pos), Vec3::from(out[c].pos));
        let n = (pb - pa).cross(pc - pa);
        for v in [a, b, c] {
            sum[classes[v]] += n;
        }
    }
    for (v, &k) in out.iter_mut().zip(&classes) {
        if let Some(n) = sum[k].try_normalize() {
            v.normal = n.to_array();
        }
    }
}

/// A node's position in a pose (for framing and attachments).
pub fn node_position(pose: &[Mat4], node: usize) -> Vec3 {
    pose.get(node).map_or(Vec3::ZERO, |m| m.w_axis.truncate())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_interpolate_and_clamp() {
        let c = Controller {
            name: "position".into(),
            times: vec![0.0, 1.0],
            values: vec![0.0, 0.0, 0.0, 2.0, 4.0, 6.0],
            columns: 3,
        };
        assert_eq!(sample(&c, 0.5), [1.0, 2.0, 3.0]);
        assert_eq!(sample(&c, -1.0), [0.0, 0.0, 0.0]);
        assert_eq!(sample(&c, 9.0), [2.0, 4.0, 6.0]);
        let q = Controller {
            name: "orientation".into(),
            times: vec![0.0, 1.0],
            values: [Quat::IDENTITY.to_array(), Quat::from_rotation_z(1.0).to_array()].concat(),
            columns: 4,
        };
        let half = Quat::from_slice(&sample(&q, 0.5));
        assert!(half.angle_between(Quat::from_rotation_z(0.5)) < 1e-4);
    }
}
