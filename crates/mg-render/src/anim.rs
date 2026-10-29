//! Poses: node transforms from an animation at a moment. Animations bind to
//! the model's nodes as the game binds them: by part number where the
//! model and the animation are compiled and so have them (a supermodel's
//! animation moves the node of the same number, whatever it is called:
//! measured in the client, where a skeleton numbered afresh under the same
//! names no longer poses its creatures), else by name (case-insensitive:
//! models read from text, whose compiler gives the numbers by the names).
//! A model plays its own animations and, for names it lacks, its
//! supermodels' (nearest first).
//! Keys interpolate linearly (orientations by slerp); a node the animation
//! does not key keeps its rest transform.
//!
//! A time within an animation (0 to its length, inclusive) is taken as it
//! is and later times wrap ([`time_in`]): an animation played once holds its
//! last frame while its time stays at its length. [`blend`] makes the
//! transition from one pose to another, as an animation starting from
//! another over its `transtime`.

use std::collections::HashMap;

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

// (Which animations a model plays: mg-mdl's, as blueprint previews read
// them without a renderer.)
pub use mg_mdl::animations;

/// The model-space transform of every node of `model` playing `anim` at
/// time `t` (seconds; see [`time_in`]).
pub fn pose(model: &Model, anim: &Animation, t: f32) -> Vec<Mat4> {
    pose_layers(model, &[anim], t)
}

/// A node of an animation layer: the node, the layer's time, and how its
/// position keys are scaled.
type Keyed<'a> = (&'a AnimNode, f32, f32);

/// An animation bound to a model's nodes, once: which of the model's
/// nodes each of the animation's moves (by part number or by name, as the
/// game binds them: see the module's head), and the scale of its
/// positions: the model's `setanimationscale` for an animation that is
/// another model's (a supermodel's, made for a body of another size), 1
/// for its own. What is posed with it ([`Playing`]) looks up no names: a
/// model kept loaded binds its animations when it is loaded.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Binding {
    /// (the animation's node, the model's node it moves), in the
    /// animation's order.
    moves: Vec<(usize, usize)>,
    scale: f32,
}

impl Binding {
    /// `anim`, played by `model`.
    pub fn new(model: &Model, anim: &Animation) -> Binding {
        Nodes::of(model, None).bind(model, anim)
    }

    /// Each of `anims`, played by `model` (its nodes looked up once).
    pub fn all<'a>(model: &Model, anims: impl IntoIterator<Item = &'a Animation>) -> Vec<Binding> {
        let nodes = Nodes::of(model, None);
        anims.into_iter().map(|a| nodes.bind(model, a)).collect()
    }
}

/// A model's nodes as an animation's are bound to them: by number, if
/// every node has one (and the model plays its own body's animations),
/// and by name in lower case.
struct Nodes {
    numbered: bool,
    by_part: HashMap<i32, usize>,
    by_name: HashMap<String, Vec<usize>>,
    /// The scale of another model's animations given (`worn`:
    /// [`pose_worn`]), else the model's own.
    worn: Option<f32>,
}

impl Nodes {
    /// `worn`: a model worn by another, its nodes found by name (its
    /// numbers are its own, not those of the body whose animation it
    /// plays), its positions scaled as the wearer's.
    fn of(model: &Model, worn: Option<f32>) -> Nodes {
        let numbered = worn.is_none() && model.nodes.iter().all(|n| n.part.is_some());
        let mut by_part: HashMap<i32, usize> = HashMap::new();
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, n) in model.nodes.iter().enumerate() {
            if let Some(part) = n.part.filter(|p| *p >= 0) {
                by_part.entry(part).or_insert(i);
            }
            by_name.entry(n.name.to_ascii_lowercase()).or_default().push(i);
        }
        Nodes { numbered, by_part, by_name, worn }
    }

    fn bind(&self, model: &Model, anim: &Animation) -> Binding {
        let own = model.animations.iter().any(|own| std::ptr::eq(own, anim));
        let scale = match self.worn.unwrap_or(model.animation_scale) {
            s if !own && s.is_finite() && s > 0.0 => s,
            _ => 1.0,
        };
        let mut moves = Vec::new();
        for (k, n) in anim.nodes.iter().enumerate() {
            if n.controllers.is_empty() && n.anim_mesh.is_none() {
                continue;
            }
            match n.part.filter(|_| self.numbered) {
                // (−1: a node of no part of the model.)
                Some(part) => moves.extend(self.by_part.get(&part).map(|&i| (k, i))),
                None => {
                    let found = self.by_name.get(&n.name.to_ascii_lowercase());
                    moves.extend(found.into_iter().flatten().map(|&i| (k, i)));
                }
            }
        }
        Binding { moves, scale }
    }
}

/// Animations playing together on a model at a moment, by the model's
/// node each of their nodes moves: what its pose ([`Playing::pose`]), its
/// lights ([`Playing::lamps`]) and its meshes ([`Playing::mesh_state`])
/// are made from, the animations bound once ([`Binding`]) and their keys
/// gathered once for the three.
#[derive(Debug, Clone)]
pub struct Playing<'a> {
    /// For each node of the model, the animation nodes that move it, in
    /// layer order, each with its layer's time and the scale of its
    /// positions.
    by_node: Vec<Vec<Keyed<'a>>>,
}

impl<'a> Playing<'a> {
    /// `layers` (each animation with its binding to `model`) at time `t`
    /// (see [`time_in`]): a controller follows the last layer that keys it.
    pub fn new(model: &Model, layers: &[(&'a Animation, &Binding)], t: f32) -> Playing<'a> {
        let mut by_node: Vec<Vec<Keyed<'a>>> = vec![Vec::new(); model.nodes.len()];
        for (a, binding) in layers {
            let at = time_in(a, t);
            for &(k, i) in &binding.moves {
                if let (Some(n), Some(node)) = (a.nodes.get(k), by_node.get_mut(i)) {
                    node.push((n, at, binding.scale));
                }
            }
        }
        Playing { by_node }
    }

    /// [`Playing::new`] of animations not bound before (bound here, by
    /// name where they go by name). `worn`: see [`pose_worn`].
    fn unbound(model: &Model, layers: &[&'a Animation], t: f32, worn: Option<f32>) -> Playing<'a> {
        let nodes = Nodes::of(model, worn);
        let bound: Vec<Binding> = layers.iter().map(|a| nodes.bind(model, a)).collect();
        let layers: Vec<(&'a Animation, &Binding)> = layers.iter().copied().zip(&bound).collect();
        Playing::new(model, &layers, t)
    }

    /// The animation nodes that move node `i` of the model.
    fn of(&self, i: usize) -> Option<&[Keyed<'a>]> {
        self.by_node.get(i).map(Vec::as_slice).filter(|n| !n.is_empty())
    }

    /// The model-space transform of each of the model's nodes.
    pub fn pose(&self, model: &Model) -> Vec<Mat4> {
        compose(model, &self.locals(model))
    }

    /// Each node's transform relative to its parent.
    pub fn locals(&self, model: &Model) -> Vec<Local> {
        model
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let (mut pos, mut orient, mut scale) = (n.position, n.orientation, n.scale);
                if let Some(nodes) = self.of(i) {
                    for (c, t, by) in controllers(nodes) {
                        let v = sample(c, t);
                        match (c.name.as_str(), v.len()) {
                            ("position", 3) => pos = [v[0] * by, v[1] * by, v[2] * by],
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
}

/// A node's controllers across layers: a later layer's replaces an earlier
/// one's of the same name.
fn controllers<'a, 'b>(
    nodes: &'b [Keyed<'a>],
) -> impl Iterator<Item = (&'a Controller, f32, f32)> + use<'a, 'b> {
    nodes.iter().enumerate().flat_map(move |(i, (n, t, scale))| {
        n.controllers
            .iter()
            .filter(move |c| {
                !nodes[i + 1..].iter().any(|(m, ..)| m.controllers.iter().any(|d| d.name == c.name))
            })
            .map(move |c| (c, *t, *scale))
    })
}

/// A pose of several animations played together (a tile's `tiledefault`,
/// `day` and animation loops): each controller follows the last layer that
/// keys it.
pub fn pose_layers(model: &Model, layers: &[&Animation], t: f32) -> Vec<Mat4> {
    Playing::unbound(model, layers, t, None).pose(model)
}

/// Every node's transform relative to its parent playing `anim` at `t`.
pub fn locals(model: &Model, anim: &Animation, t: f32) -> Vec<Local> {
    locals_layers(model, &[anim], t)
}

/// [`locals`] for several animations played together (see
/// [`pose_layers`]).
pub fn locals_layers(model: &Model, layers: &[&Animation], t: f32) -> Vec<Local> {
    Playing::unbound(model, layers, t, None).locals(model)
}

/// [`pose`] for a model worn by another (a cloak, a robe), playing the
/// wearer's animation: its positions scaled as the wearer's are (`scale`,
/// the wearer's `setanimationscale`), whatever the worn model's own says.
/// The game moves a cloak with the body's bones: an elf's cloak, whose
/// model has no scale of its own, sits on an elf's shoulders, not at a
/// human's height.
pub fn pose_worn(model: &Model, anim: &Animation, t: f32, scale: f32) -> Vec<Mat4> {
    Playing::unbound(model, &[anim], t, Some(scale)).pose(model)
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
    match f {
        f if f >= 1.0 => to.to_vec(),
        f if f <= 0.0 => from.to_vec(),
        f => from.iter().zip(to).map(|(a, b)| a.lerp(b, f)).collect(),
    }
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
    let lamps = lamps_layers(model, layers, t, pose);
    lamps.iter().filter_map(|l| l.placed(transform, main_light)).collect()
}

/// A light node of a model at a moment, where the model's pose has it:
/// what [`lights_layers`] makes a scene's light of, once the model is
/// placed. The same for every instance of the model playing the same
/// animations (an area's tiles), which then differ by where they stand and
/// by their main lights' colours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lamp {
    /// Where it is, in the model's space.
    pub at: Vec3,
    /// A tile's main light 1 (0) or 2 (1), by its node's name.
    pub main: Option<usize>,
    /// Its own colour, radius and multiplier: the animations' where they
    /// key them, else the node's.
    pub color: Vec3,
    pub radius: f32,
    pub multiplier: f32,
    pub ambient_only: bool,
    pub priority: u32,
    /// Its node's `shadow` flag, and its `verticaldisplacement`.
    pub shadow: bool,
    pub lift: f32,
}

impl Lamp {
    /// The light of a model placed by `transform`; none for one that is
    /// black or reaches nowhere. A main light takes `main_light`'s colour
    /// when it gives one, and then the engine's radius. The node's
    /// `multiplier` lengthens the light's reach, not its colour: a white
    /// light of radius 5 and multiplier 2 ends 20 m out, as bright as with
    /// 1; a gray one stays as gray (read from the client:
    /// `model_light_uniforms` in `client_render.rs`).
    pub fn placed(
        &self,
        transform: Mat4,
        main_light: &dyn Fn(usize) -> Option<Vec3>,
    ) -> Option<crate::scene::PointLight> {
        let tile = self.main.and_then(|slot| Some((slot, main_light(slot)?)));
        let color = tile.map_or(self.color, |(_, c)| c);
        let radius = match tile {
            Some((0, _)) => 10.0,
            Some(_) => 5.0,
            None => self.radius,
        };
        let radius = radius * self.multiplier;
        if radius <= 0.0 || color == Vec3::ZERO {
            return None;
        }
        let light = crate::scene::PointLight::new(
            transform.transform_point3(self.at),
            color,
            radius,
            self.ambient_only,
            self.priority,
        );
        Some(light.casting(self.shadow, radius, self.lift))
    }
}

/// A model's light nodes at `t` of the animations `layers`, in `pose`.
pub fn lamps_layers(model: &Model, layers: &[&Animation], t: f32, pose: &[Mat4]) -> Vec<Lamp> {
    // (Without a light, nothing to look up.)
    if !model.nodes.iter().any(|n| matches!(n.kind, mg_mdl::NodeKind::Light(_))) {
        return Vec::new();
    }
    Playing::unbound(model, layers, t, None).lamps(model, pose)
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
    Playing::unbound(&model.model, layers, t, None).mesh_state(model)
}

impl Playing<'_> {
    /// [`lamps_layers`]: the model's light nodes, in `pose`.
    pub fn lamps(&self, model: &Model, pose: &[Mat4]) -> Vec<Lamp> {
        let mut out = Vec::new();
        for (i, n) in model.nodes.iter().enumerate() {
            let mg_mdl::NodeKind::Light(l) = &n.kind else { continue };
            out.push(self.lamp(i, n, l, pose));
        }
        out
    }

    fn lamp(&self, i: usize, n: &mg_mdl::Node, l: &mg_mdl::Light, pose: &[Mat4]) -> Lamp {
        let keyed = |name: &str| -> Option<Vec<f32>> {
            let nodes = self.of(i)?;
            let (c, t, _) = controllers(nodes).filter(|(c, ..)| c.name == name).last()?;
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
        Lamp {
            at: node_position(pose, i),
            main,
            color: match value("color").as_deref() {
                Some([r, g, b, ..]) => Vec3::new(*r, *g, *b),
                _ => Vec3::ONE,
            },
            radius: value("radius").and_then(|v| v.first().copied()).unwrap_or(5.0),
            multiplier: value("multiplier").and_then(|v| v.first().copied()).unwrap_or(1.0),
            ambient_only: l.ambient_only,
            priority: l.priority.clamp(1, 5),
            shadow: l.shadow,
            lift: value("verticaldisplacement").and_then(|v| v.first().copied()).unwrap_or(0.0),
        }
    }

    /// [`mesh_state_layers`]: what the animations change in `model`'s
    /// meshes (the model they were bound to).
    pub fn mesh_state(&self, model: &GpuModel) -> MeshState {
        let mut state = MeshState::new(model);
        for (i, mesh) in model.meshes.iter().enumerate() {
            let Some(nodes) = self.of(mesh.node) else { continue };
            let out = &mut state.meshes[i];
            for (c, t, _) in controllers(nodes) {
                let v = sample(c, t);
                match (c.name.as_str(), v.as_slice()) {
                    ("alpha", [a, ..]) => out.alpha = Some(*a),
                    ("selfillumcolor", [r, g, b, ..]) => {
                        out.selfillum = Some(Vec3::new(*r, *g, *b))
                    }
                    _ => {}
                }
            }
            let sets = nodes.iter().rev().find_map(|(n, t, _)| Some((n.anim_mesh.as_ref()?, *t)));
            if let (Some((sets, t)), Some(data)) = (sets, model.mesh_data(i)) {
                out.vertices = animated_vertices(data, sets, t);
            }
        }
        state
    }
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
    for t in m.triangles() {
        let [a, b, c] = t.map(|v| v as usize);
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

    fn lamp(color: Vec3, radius: f32, multiplier: f32) -> Lamp {
        Lamp {
            at: Vec3::new(1.0, 2.0, 3.0),
            main: None,
            color,
            radius,
            multiplier,
            ambient_only: false,
            priority: 2,
            shadow: false,
            lift: 0.0,
        }
    }

    /// A light node's multiplier lengthens its reach and leaves its colour,
    /// as the client's uniforms have it (`model_light_uniforms` in
    /// `client_render.rs`): white radius 5 ends 10 m out, 20 m with
    /// multiplier 2 and 5 m with 0.5; gray 0.3 with multiplier 2 stays
    /// lin(0.3) and ends 20 m out; (2, 1, 1) with 2 is (1, lin(0.5),
    /// lin(0.5)) ending 40 m out.
    #[test]
    fn a_light_node_s_multiplier_lengthens_its_reach() {
        let none = |_: usize| None;
        let at = |l: Lamp| l.placed(Mat4::from_translation(Vec3::X), &none).unwrap();
        let white = at(lamp(Vec3::ONE, 5.0, 1.0));
        assert_eq!((white.color, white.cutoff), (Vec3::ONE, 10.0));
        assert_eq!(white.position, Vec3::new(2.0, 2.0, 3.0));
        assert_eq!(at(lamp(Vec3::ONE, 5.0, 2.0)).cutoff, 20.0);
        assert_eq!(at(lamp(Vec3::ONE, 5.0, 0.5)).cutoff, 5.0);
        let gray = at(lamp(Vec3::splat(0.3), 5.0, 2.0));
        assert!((gray.color.x - 0.3f32.powf(2.2)).abs() < 1e-6, "{gray:?}");
        assert_eq!(gray.cutoff, 20.0);
        let bright = at(lamp(Vec3::new(2.0, 1.0, 1.0), 5.0, 2.0));
        assert!((bright.color - Vec3::new(1.0, 0.2176, 0.2176)).abs().max_element() < 1e-3);
        assert_eq!(bright.cutoff, 40.0);
        // A light flagged `shadow` casts creatures' shadows from 10 m of
        // radius (a placeable's light; a tile's flame, 7, none), from its
        // `verticaldisplacement` above it.
        let casting =
            |radius: f32| at(Lamp { shadow: true, lift: 3.0, ..lamp(Vec3::ONE, radius, 1.0) });
        assert!(casting(10.0).casts_shadows && casting(10.0).shadow_lift == 3.0);
        assert!(!casting(7.0).casts_shadows && !at(lamp(Vec3::ONE, 10.0, 1.0)).casts_shadows);
        // A main light keeps the engine's radius, times its multiplier (the
        // game's tiles have 1).
        let main = Lamp { main: Some(0), ..lamp(Vec3::ONE, 14.0, 1.0) };
        let red = |_: usize| Some(Vec3::X);
        let placed = main.placed(Mat4::IDENTITY, &red).unwrap();
        assert_eq!((placed.color, placed.cutoff), (Vec3::X, 20.0));
        assert!(lamp(Vec3::ZERO, 5.0, 1.0).placed(Mat4::IDENTITY, &none).is_none(), "black");
        assert!(lamp(Vec3::ONE, 5.0, 0.0).placed(Mat4::IDENTITY, &none).is_none(), "no reach");
    }

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
    fn a_supermodel_s_positions_are_scaled_to_the_model() {
        // A model three quarters the size of the one whose animation it
        // plays (`setanimationscale 0.75`).
        let key = |values: Vec<f32>| Controller {
            name: "position".into(),
            columns: 3,
            times: vec![0.0],
            values,
            ..Default::default()
        };
        let anim = |name: &str| Animation {
            name: name.into(),
            length: 1.0,
            nodes: vec![AnimNode {
                name: "a".into(),
                controllers: vec![key(vec![2.0, 0.0, 4.0])],
                ..Default::default()
            }],
            ..Default::default()
        };
        let model = Model {
            nodes: vec![mg_mdl::Node::new("a", mg_mdl::NodeKind::Dummy)],
            animation_scale: 0.75,
            animations: vec![anim("own")],
            ..Default::default()
        };
        let borrowed = anim("borrowed");
        let at = |a: &Animation| pose(&model, a, 0.0)[0].to_scale_rotation_translation().2;
        assert_eq!(at(&borrowed), Vec3::new(1.5, 0.0, 3.0), "another model's: scaled");
        assert_eq!(at(&model.animations[0]), Vec3::new(2.0, 0.0, 4.0), "its own: as keyed");
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
        assert_eq!(blend(&rest, &turned, 7.0), turned, "past the end: the new pose");
        assert_eq!(blend(&rest, &turned, -1.0), rest);
        let quarter = compose(&model, &blend(&rest, &turned, 0.25))[1];
        assert!(quarter.abs_diff_eq(
            compose(&model, &[rest[0].lerp(&turned[0], 0.25), rest[1].lerp(&turned[1], 0.25)])[1],
            1e-5
        ));
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

    /// Compiled models bind by part number, as the game does: an
    /// animation's node moves the node of its number, whatever either is
    /// called; one numbered −1 moves nothing. Models from text (no
    /// numbers) bind by name; so does a worn model, whose numbers are its
    /// own and not the body's.
    #[test]
    fn compiled_models_bind_by_part_number() {
        let moved = |x: f32, name: &str, part: Option<i32>| AnimNode {
            name: name.into(),
            part,
            controllers: vec![Controller {
                name: "position".into(),
                columns: 3,
                times: vec![0.0],
                values: vec![x, 0.0, 0.0],
                ..Default::default()
            }],
            ..Default::default()
        };
        let model = |parts: [Option<i32>; 3]| {
            let mut m = Model::default();
            for (i, name) in ["root", "arm", "leg"].into_iter().enumerate() {
                let mut n = mg_mdl::Node::new(name, mg_mdl::NodeKind::Dummy);
                n.parent = (i > 0).then_some(0);
                n.part = parts[i];
                m.nodes.push(n);
            }
            m
        };
        let x = |pose: &[Mat4]| [pose[0].w_axis.x, pose[1].w_axis.x, pose[2].w_axis.x];
        // A supermodel's animation: its root (another name, number 0), a
        // node called "leg" numbered as the model's arm, one numbered −1.
        let anim = |parts: [Option<i32>; 3]| Animation {
            nodes: vec![
                moved(1.0, "super", parts[0]),
                moved(2.0, "leg", parts[1]),
                moved(4.0, "arm", parts[2]),
            ],
            ..Default::default()
        };
        let compiled = model([Some(0), Some(1), Some(2)]);
        let by_number = anim([Some(0), Some(1), Some(-1)]);
        // The root moves (by 1), the arm by the node called "leg"; the leg
        // stays (carried by the root alone).
        assert_eq!(x(&pose(&compiled, &by_number, 0.0)), [1.0, 3.0, 1.0]);
        // From text: by name.
        let text = model([None; 3]);
        assert_eq!(x(&pose(&text, &anim([None; 3]), 0.0)), [0.0, 4.0, 2.0]);
        // A compiled model under an animation from text, and a worn one:
        // by name as well.
        assert_eq!(x(&pose(&compiled, &anim([None; 3]), 0.0)), [0.0, 4.0, 2.0]);
        assert_eq!(x(&pose_worn(&compiled, &by_number, 0.0, 1.0)), [0.0, 4.0, 2.0]);
    }

    /// Animations bound once, when a model is loaded, pose it and light it
    /// as those bound where they are played: by part number, by name in
    /// another case, a node of no part (−1) moving nothing, a supermodel's
    /// animation scaled, a light's color keyed, layers in their order.
    #[test]
    fn animations_bound_once_pose_and_light_as_bound_each_time() {
        let key = |name: &str, values: Vec<f32>| Controller {
            name: name.into(),
            columns: values.len(),
            times: vec![0.0, 1.0],
            values: [values.clone(), values.iter().map(|v| v * 2.0).collect()].concat(),
            ..Default::default()
        };
        let node = |name: &str, part, controllers| AnimNode {
            name: name.into(),
            part,
            controllers,
            ..Default::default()
        };
        let model = |parts: [Option<i32>; 3]| {
            let mut m = Model { animation_scale: 0.5, ..Default::default() };
            let light = mg_mdl::NodeKind::Light(mg_mdl::Light::default());
            for (i, (name, kind)) in [
                ("Root", mg_mdl::NodeKind::Dummy),
                ("Arm", mg_mdl::NodeKind::Dummy),
                ("lamp", light),
            ]
            .into_iter()
            .enumerate()
            {
                let mut n = mg_mdl::Node::new(name, kind);
                n.parent = (i > 0).then_some(0);
                n.part = parts[i];
                m.nodes.push(n);
            }
            m.animations.push(Animation {
                name: "own".into(),
                length: 1.0,
                nodes: vec![
                    node("ROOT", Some(0), vec![key("position", vec![1.0, 0.0, 0.0])]),
                    node("lamp", Some(2), vec![key("color", vec![0.25, 0.5, 0.75])]),
                ],
                ..Default::default()
            });
            m
        };
        let borrowed = Animation {
            name: "borrowed".into(),
            length: 1.0,
            nodes: vec![
                node("arm", Some(1), vec![key("position", vec![0.0, 2.0, 0.0])]),
                node("nothing", Some(-1), vec![key("position", vec![9.0, 9.0, 9.0])]),
                node("ARM", Some(7), vec![key("scale", vec![1.5])]),
            ],
            ..Default::default()
        };
        for model in [model([Some(0), Some(1), Some(2)]), model([None; 3])] {
            let layers = [&model.animations[0], &borrowed];
            let bound = Binding::all(&model, layers);
            let pairs: Vec<(&Animation, &Binding)> = layers.into_iter().zip(&bound).collect();
            for t in [0.0, 0.25, 1.0, 1.5] {
                let playing = Playing::new(&model, &pairs, t);
                let posed = pose_layers(&model, &layers, t);
                assert_eq!(playing.pose(&model), posed, "at {t}");
                assert_eq!(playing.lamps(&model, &posed), lamps_layers(&model, &layers, t, &posed));
            }
            let played = Playing::new(&model, &pairs, 0.0).pose(&model);
            // (The borrowed arm's position is the supermodel's, scaled.)
            assert_eq!(played[1].w_axis.truncate(), Vec3::new(1.0, 1.0, 0.0));
            let lamp = Playing::new(&model, &pairs, 0.0).lamps(&model, &played);
            assert_eq!(lamp[0].color, Vec3::new(0.25, 0.5, 0.75));
        }
    }
}
