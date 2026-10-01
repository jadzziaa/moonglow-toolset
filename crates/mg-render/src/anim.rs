//! Poses: node transforms from an animation at a moment. Animations bind to
//! the model's nodes by name (case-insensitive); a model plays its own
//! animations and, for names it lacks, its supermodels' (nearest first).
//! Keys interpolate linearly (orientations by slerp); a node the animation
//! does not key keeps its rest transform.
//!
//! A time within an animation (0 to its length, inclusive) is taken as it
//! is and later times wrap ([`time_in`]): an animation played once holds its
//! last frame while its time stays at its length. [`blend`] makes the
//! transition from one pose to another, as an animation starting from
//! another over its `transtime`.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};
use mg_mdl::{AnimMeshSets, AnimNode, Animation, Controller, Mesh, Model};

use crate::model::{GpuModel, Vertex};
use crate::scene::MeshState;

/// An animation's time at `t`: `t` within the animation (its length
/// included), else wrapped (looping).
pub fn time_in(anim: &Animation, t: f32) -> f32 {
    if anim.length <= 0.0 {
        0.0
    } else if (0.0..=anim.length).contains(&t) {
        t
    } else {
        t.rem_euclid(anim.length)
    }
}

/// A node's transform relative to its parent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Local {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: f32,
}

impl Local {
    /// From a model's position, orientation (x, y, z, w) and scale.
    pub fn new(position: [f32; 3], orientation: [f32; 4], scale: f32) -> Local {
        let q = Quat::from_array(orientation);
        let rotation = if q.length_squared() > 1e-12 { q.normalize() } else { Quat::IDENTITY };
        Local { position: Vec3::from(position), rotation, scale }
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(Vec3::splat(self.scale), self.rotation, self.position)
    }

    /// `f` of the way to `other`: position and scale linearly, rotation by
    /// slerp.
    pub fn lerp(&self, other: &Local, f: f32) -> Local {
        Local {
            position: self.position.lerp(other.position, f),
            rotation: self.rotation.slerp(other.rotation, f),
            scale: self.scale + (other.scale - self.scale) * f,
        }
    }
}

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
/// time `t` (seconds; see [`time_in`]).
pub fn pose(model: &Model, anim: &Animation, t: f32) -> Vec<Mat4> {
    pose_layers(model, &[anim], t)
}

/// The animation nodes of `layers` by node name (lower case), in layer
/// order, each with its layer's time at `t`.
fn keyed<'a>(layers: &[&'a Animation], t: f32) -> HashMap<String, Vec<(&'a AnimNode, f32)>> {
    let mut out: HashMap<String, Vec<_>> = HashMap::new();
    for a in layers {
        let at = time_in(a, t);
        for n in &a.nodes {
            if !n.controllers.is_empty() || n.anim_mesh.is_some() {
                out.entry(n.name.to_ascii_lowercase()).or_default().push((n, at));
            }
        }
    }
    out
}

/// A node's controllers across layers: a later layer's replaces an earlier
/// one's of the same name.
fn controllers<'a, 'b>(
    nodes: &'b [(&'a AnimNode, f32)],
) -> impl Iterator<Item = (&'a Controller, f32)> + use<'a, 'b> {
    nodes.iter().enumerate().flat_map(move |(i, (n, t))| {
        n.controllers
            .iter()
            .filter(move |c| {
                !nodes[i + 1..].iter().any(|(m, _)| m.controllers.iter().any(|d| d.name == c.name))
            })
            .map(move |c| (c, *t))
    })
}

/// A pose of several animations played together (a tile's `tiledefault`,
/// `day` and animation loops): each controller follows the last layer that
/// keys it.
pub fn pose_layers(model: &Model, layers: &[&Animation], t: f32) -> Vec<Mat4> {
    compose(model, &locals_layers(model, layers, t))
}

/// Every node's transform relative to its parent playing `anim` at `t`.
pub fn locals(model: &Model, anim: &Animation, t: f32) -> Vec<Local> {
    locals_layers(model, &[anim], t)
}

/// [`locals`] for several animations played together (see
/// [`pose_layers`]).
pub fn locals_layers(model: &Model, layers: &[&Animation], t: f32) -> Vec<Local> {
    let by_name = keyed(layers, t);
    model
        .nodes
        .iter()
        .map(|n| {
            let (mut pos, mut orient, mut scale) = (n.position, n.orientation, n.scale);
            if let Some(nodes) = by_name.get(&n.name.to_ascii_lowercase()) {
                for (c, t) in controllers(nodes) {
                    let v = sample(c, t);
                    match (c.name.as_str(), v.len()) {
                        ("position", 3) => pos = [v[0], v[1], v[2]],
                        ("orientation", 4) => orient = [v[0], v[1], v[2], v[3]],
                        ("scale", 1) => scale = v[0],
                        _ => {}
                    }
                }
            }
            Local::new(pos, orient, scale)
        })
        .collect()
}

/// The rest pose's transforms relative to their parents.
pub fn rest_locals(model: &Model) -> Vec<Local> {
    model.nodes.iter().map(|n| Local::new(n.position, n.orientation, n.scale)).collect()
}

/// Model-space transforms from transforms relative to the parents.
pub fn compose(model: &Model, locals: &[Local]) -> Vec<Mat4> {
    let mut out: Vec<Mat4> = Vec::with_capacity(model.nodes.len());
    for (i, n) in model.nodes.iter().enumerate() {
        let l = locals.get(i).map_or(Mat4::IDENTITY, Local::matrix);
        out.push(match n.parent.filter(|&p| p < i) {
            Some(p) => out[p] * l,
            None => l,
        });
    }
    out
}

/// A transition: each node `f` (0 to 1) of the way from one pose to
/// another, relative to its parent (as when an animation starts from
/// another over its `transtime`; the engine's exact blend is not measured).
pub fn blend(from: &[Local], to: &[Local], f: f32) -> Vec<Local> {
    let f = f.clamp(0.0, 1.0);
    from.iter().zip(to).map(|(a, b)| a.lerp(b, f)).collect()
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
    lights_layers(model, anim.as_slice(), t, pose, transform, main_light)
}

/// [`lights`] for several animations played together (see
/// [`pose_layers`]).
pub fn lights_layers(
    model: &Model,
    layers: &[&Animation],
    t: f32,
    pose: &[Mat4],
    transform: Mat4,
    main_light: &dyn Fn(usize) -> Option<Vec3>,
) -> Vec<crate::scene::PointLight> {
    let by_name = keyed(layers, t);
    let mut out = Vec::new();
    for (i, n) in model.nodes.iter().enumerate() {
        let mg_mdl::NodeKind::Light(l) = &n.kind else { continue };
        let keyed = |name: &str| -> Option<Vec<f32>> {
            let nodes = by_name.get(&n.name.to_ascii_lowercase())?;
            let (c, t) = controllers(nodes).filter(|(c, _)| c.name == name).last()?;
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
    mesh_state_layers(model, &[anim], t)
}

/// [`mesh_state`] for several animations played together (see
/// [`pose_layers`]).
pub fn mesh_state_layers(model: &GpuModel, layers: &[&Animation], t: f32) -> MeshState {
    let by_name = keyed(layers, t);
    let mut state = MeshState::new(model);
    for (i, mesh) in model.meshes.iter().enumerate() {
        let name = model.model.nodes[mesh.node].name.to_ascii_lowercase();
        let Some(nodes) = by_name.get(&name) else { continue };
        let out = &mut state.meshes[i];
        for (c, t) in controllers(nodes) {
            let v = sample(c, t);
            match (c.name.as_str(), v.as_slice()) {
                ("alpha", [a, ..]) => out.alpha = Some(*a),
                ("selfillumcolor", [r, g, b, ..]) => out.selfillum = Some(Vec3::new(*r, *g, *b)),
                _ => {}
            }
        }
        let sets = nodes.iter().rev().find_map(|(n, t)| Some((n.anim_mesh.as_ref()?, *t)));
        if let (Some((sets, t)), Some(data)) = (sets, model.mesh_data(i)) {
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
            ..Default::default()
        };
        assert_eq!(sample(&c, 0.5), [1.0, 2.0, 3.0]);
        assert_eq!(sample(&c, -1.0), [0.0, 0.0, 0.0]);
        assert_eq!(sample(&c, 9.0), [2.0, 4.0, 6.0]);
        let q = Controller {
            name: "orientation".into(),
            times: vec![0.0, 1.0],
            values: [Quat::IDENTITY.to_array(), Quat::from_rotation_z(1.0).to_array()].concat(),
            columns: 4,
            ..Default::default()
        };
        let half = Quat::from_slice(&sample(&q, 0.5));
        assert!(half.angle_between(Quat::from_rotation_z(0.5)) < 1e-4);
    }

    #[test]
    fn times_within_hold_and_past_wrap() {
        let a = Animation { length: 2.0, ..Default::default() };
        assert_eq!(time_in(&a, 2.0), 2.0, "the last frame, for playing once");
        assert_eq!(time_in(&a, 2.5), 0.5);
        assert_eq!(time_in(&a, -0.5), 1.5);
        assert_eq!(time_in(&Animation::default(), 3.0), 0.0);
    }

    #[test]
    fn transitions_blend_each_node_from_its_parent() {
        let mut model = Model::default();
        model.nodes.push(mg_mdl::Node::new("root", mg_mdl::NodeKind::Dummy));
        let mut child = mg_mdl::Node::new("arm", mg_mdl::NodeKind::Dummy);
        child.parent = Some(0);
        child.position = [1.0, 0.0, 0.0];
        model.nodes.push(child);
        let rest = rest_locals(&model);
        let mut turned = rest.clone();
        turned[0].rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        turned[1].scale = 3.0;
        let half = blend(&rest, &turned, 0.5);
        assert!(
            half[0].rotation.angle_between(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4))
                < 1e-5
        );
        assert_eq!(half[1].scale, 2.0);
        // The arm swings with the root, around it (not across).
        let pose = compose(&model, &half);
        let arm = pose[1].w_axis.truncate();
        assert!((arm - Vec3::new(1.0, 1.0, 0.0).normalize()).length() < 1e-5, "{arm}");
        assert_eq!(compose(&model, &blend(&rest, &turned, 7.0))[1], compose(&model, &turned)[1]);
    }

    #[test]
    fn layers_replace_controllers_of_the_same_name() {
        let key = |name: &str, values: Vec<f32>| Controller {
            name: name.into(),
            columns: values.len(),
            times: vec![0.0],
            values,
            ..Default::default()
        };
        let node = |name: &str, controllers| AnimNode {
            name: name.into(),
            controllers,
            ..Default::default()
        };
        let mut model = Model::default();
        model.nodes.push(mg_mdl::Node::new("root", mg_mdl::NodeKind::Dummy));
        let mut child = mg_mdl::Node::new("A", mg_mdl::NodeKind::Dummy);
        child.parent = Some(0);
        model.nodes.push(child);
        let turn = Quat::from_rotation_z(1.0).to_array().to_vec();
        let first = Animation {
            nodes: vec![node(
                "a",
                vec![key("position", vec![1.0, 0.0, 0.0]), key("orientation", turn)],
            )],
            ..Default::default()
        };
        let second = Animation {
            nodes: vec![node("a", vec![key("position", vec![2.0, 0.0, 0.0])])],
            ..Default::default()
        };
        let pose = pose_layers(&model, &[&first, &second], 0.0);
        let (_, rotation, translation) = pose[1].to_scale_rotation_translation();
        assert_eq!(translation, Vec3::new(2.0, 0.0, 0.0), "the later layer's position");
        assert!(rotation.angle_between(Quat::from_rotation_z(1.0)) < 1e-5, "the earlier's turn");
        assert_eq!(pose_layers(&model, &[&second, &first], 0.0)[1].w_axis.x, 1.0);
    }
}
