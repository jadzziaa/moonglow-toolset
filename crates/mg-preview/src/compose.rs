//! A preview's models on the GPU, posed: the base plays an animation (its
//! own or its supermodels'), attached parts follow their nodes, and
//! animated parts (skinned robes and cloaks, wings, tails) play the same
//! animation on their own nodes.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_mdl::{Animation, Model};
use mg_render::{Gpu, GpuModel, Instance, PointLight, anim};

use crate::{Preview, PreviewLight};

#[derive(Debug)]
struct Placed {
    gpu: Arc<GpuModel>,
    /// The base's node it hangs from.
    attach: Option<usize>,
    scale: f32,
    textures: Option<Arc<HashMap<String, String>>>,
    colors: Option<[u8; 10]>,
    env_map: Option<String>,
    animated: bool,
    /// A body part whose skins are bound to the base's skeleton: its
    /// nodes from here on stand for the base's, posed as the base is
    /// ([`on_skeleton`]).
    skeleton: Option<Skeleton>,
    /// Its own animations and its supermodels' (animated parts).
    anims: Vec<(String, Arc<Model>)>,
}

impl Placed {
    /// One of its animations (its own or a supermodel's), by name.
    fn animation(&self, name: &str) -> Option<&Animation> {
        let owner = self.anims.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
        owner.1.animation(name)
    }

    /// A body part bound to the skeleton ([`Placed::skeleton`]), posed:
    /// its own nodes at rest, and its stand-ins where the base's nodes are
    /// at `base_pose`, in the part's space (`hung`: where it hangs, at its
    /// scale). `None` for any other part.
    fn skeleton_pose(&self, base_pose: &[Mat4], hung: Mat4) -> Option<Vec<Mat4>> {
        let skeleton = self.skeleton.as_ref()?;
        let back = hung.inverse();
        let rest = &self.gpu.rest;
        let mut pose = rest[..skeleton.from.min(rest.len())].to_vec();
        let at = |&n: &usize| back * base_pose.get(n).copied().unwrap_or(Mat4::IDENTITY);
        pose.extend(skeleton.nodes.iter().map(at));
        Some(pose)
    }
}

/// The animations a creature stands in.
const STANDING: [&str; 2] = ["pause1", "cpause1"];

/// The animation a model plays when `asked` is asked of it (`find` looks a
/// name up in the model and its supermodels). A standing idle missing
/// falls back to the other: a model whose appearance says full-body
/// (`pause1`) may have a creature's animations (`cpause1`). Any other
/// missing (a placeable's `default`), none: the model shows its rest pose.
/// A corpse placeable of CEP's (`plc_orc2`) has no animations, and its
/// supermodels are a creature's: their `pause1` stood it up in the area,
/// where its rest pose lies dead.
fn playing<'a>(asked: &str, find: impl Fn(&str) -> Option<&'a Animation>) -> Option<&'a Animation> {
    find(asked).or_else(|| {
        let standing = STANDING.iter().any(|s| s.eq_ignore_ascii_case(asked));
        standing.then(|| STANDING.iter().find_map(|s| find(s))).flatten()
    })
}

/// A preview ready to draw.
#[derive(Debug)]
pub struct Composed {
    base: Placed,
    parts: Vec<Placed>,
    /// The animation the preview stands in.
    pub idle: Option<String>,
    /// The light of its appearance (placeables.2da `LightColor`).
    pub lights: Vec<PreviewLight>,
    /// The light nodes of its models (the base's and the parts'), in the
    /// base's space as it stands in its idle animation: a builder's lamp,
    /// a held torch. The client lights an area with them as with any
    /// other light, whatever the placeable's **Illuminated** says (read
    /// from the client: `model_light_uniforms` in `client_render.rs`).
    lamps: Vec<anim::Lamp>,
    /// Models named by the preview that could not be loaded.
    pub missing: Vec<String>,
    /// Posing it failed once: it is drawn no more (rather than fail again
    /// every frame).
    failed: std::sync::atomic::AtomicBool,
}

fn place(
    gpu: &Gpu,
    part: &crate::Part,
    base: Option<&Model>,
    load: &dyn Fn(&str) -> Option<Arc<Model>>,
) -> Option<Placed> {
    // (A model that is read and then breaks what builds it is shown as a
    // missing one: the toolset carries on.)
    mg_render::guard::guarded(&part.model, || place_unguarded(gpu, part, base, load)).flatten()
}

fn place_unguarded(
    gpu: &Gpu,
    part: &crate::Part,
    base: Option<&Model>,
    load: &dyn Fn(&str) -> Option<Arc<Model>>,
) -> Option<Placed> {
    let mut model = load(&part.model)?;
    // A part hung from a node and not animated (a body part, a held
    // item) is drawn rigid by the game, and a mesh in it that is a skin
    // has no skeleton to follow there, save a body part's skin bound to
    // the body's skeleton.
    let mut skeleton = None;
    if let Some((node, base)) = part.attach.as_deref().zip(base).filter(|_| !part.animated) {
        let body_part = node.ends_with("_g").then_some(base);
        model = without_skins(model, body_part);
        if let Some((on, at)) = body_part.and_then(|base| on_skeleton(&model, base)) {
            (model, skeleton) = (on, Some(at));
        }
    }
    let attach = part
        .attach
        .as_deref()
        .and_then(|name| base?.nodes.iter().position(|n| n.name.eq_ignore_ascii_case(name)));
    let anims =
        if part.animated || base.is_none() { anim::animations(&model, load) } else { Vec::new() };
    Some(Placed {
        gpu: Arc::new(GpuModel::new(gpu, model)),
        attach,
        scale: part.scale,
        textures: (!part.textures.is_empty())
            .then(|| Arc::new(part.textures.iter().map(|(a, b)| (a.clone(), b.clone())).collect())),
        colors: part.colors,
        env_map: part.env_map.clone(),
        animated: part.animated,
        skeleton,
        anims,
    })
}

/// A part hung from a node, with the skin meshes the game does not draw
/// there not drawn (the model itself if it has none such). Seen in the
/// client (`docs/findings.md`, "A skin mesh in a body part or a held
/// item"): a body part's skin whose bones are the skeleton's, no nodes of
/// the part, is drawn as a plain mesh where the model is text and, where
/// it is compiled, only if it was compiled against the skeleton
/// ([`bound_to_skeleton`]); a held item's is drawn in neither.
/// (`body_part`: the skeleton, where it hangs from one of its part nodes.)
fn without_skins(model: Arc<Model>, body_part: Option<&Model>) -> Arc<Model> {
    let hidden = |n: &mg_mdl::Node| {
        n.mesh().is_some_and(|m| match &m.extra {
            mg_mdl::MeshExtra::Skin(s) => {
                // (Text: no bind pose is stored.)
                let text = s.bones.is_empty() && s.inverse_bind.is_empty();
                let bound = body_part.is_some_and(|base| bound_to_skeleton(s, &model, base));
                m.render && !(body_part.is_some() && text) && !bound
            }
            _ => false,
        })
    };
    if !model.nodes.iter().any(hidden) {
        return model;
    }
    let mut rigid = (*model).clone();
    for n in rigid.nodes.iter_mut().filter(|n| hidden(n)) {
        if let mg_mdl::NodeKind::Mesh(m) = &mut n.kind {
            m.render = false;
        }
    }
    Arc::new(rigid)
}

/// Whether a compiled body part's skin was compiled against the skeleton
/// it hangs from: its bind pose has one entry per node of the skeleton
/// (not of the part), and its bones are nodes of the skeleton. Beamdog's
/// HD bodies are made so (`pmh0_neck001`: bones 4, 5 and 2 of `pmh0`,
/// `neck_g`, `head_g` and `torso_g`; nwn-mdl-spec MDL-SKIN-012).
fn bound_to_skeleton(skin: &mg_mdl::Skin, part: &Model, skeleton: &Model) -> bool {
    let nodes = skeleton.nodes.len();
    !skin.bones.is_empty()
        && skin.inverse_bind.len() == nodes
        && nodes != part.nodes.len()
        && skin.bones.iter().all(|&b| b < nodes)
}

/// A model's nodes in the order of a depth-first walk of its tree from its
/// first root, children in their order: where the engine binds a skin's
/// bones (nwn-mdl-spec MDL-SKIN-011). A binary model's nodes are read in
/// this order; one read from text keeps the file's. Nodes outside the tree
/// follow, in their order.
fn tree_order(model: &Model) -> Vec<usize> {
    let mut out = Vec::with_capacity(model.nodes.len());
    let mut seen = vec![false; model.nodes.len()];
    let root = model.nodes.iter().position(|n| n.parent.is_none());
    let mut stack: Vec<usize> = root.into_iter().collect();
    while let Some(i) = stack.pop() {
        if std::mem::replace(&mut seen[i], true) {
            continue;
        }
        out.push(i);
        let children = model.nodes[i].children.iter().rev();
        stack.extend(children.filter(|&&c| c < model.nodes.len() && !seen[c]));
    }
    out.extend((0..model.nodes.len()).filter(|&i| !seen[i]));
    out
}

/// Where a body part's stand-ins for the skeleton's nodes start, and the
/// base's node each stands for.
#[derive(Debug)]
struct Skeleton {
    from: usize,
    nodes: Vec<usize>,
}

/// A body part whose skins are bound to the skeleton ([`bound_to_skeleton`])
/// as the renderer can draw it: the skeleton's nodes put after its own (as
/// dummies, which [`Composed::instances`] poses as the base's), and its
/// skins' bones and bind poses moved onto them. `None` for a part with no
/// such skin.
fn on_skeleton(model: &Model, skeleton: &Model) -> Option<(Arc<Model>, Skeleton)> {
    let bound = |n: &mg_mdl::Node| {
        n.mesh().is_some_and(|m| match &m.extra {
            mg_mdl::MeshExtra::Skin(s) => m.render && bound_to_skeleton(s, model, skeleton),
            _ => false,
        })
    };
    if !model.nodes.iter().any(bound) {
        return None;
    }
    let from = model.nodes.len();
    // A compiled skin's bone is the skeleton's node at that place in a
    // depth-first walk of its tree (nwn-mdl-spec MDL-SKIN-011: not its part
    // number, which a skeleton on a supermodel takes from that one's).
    let nodes = tree_order(skeleton);
    let mut on = model.clone();
    for (n, was) in on.nodes.iter_mut().zip(&model.nodes) {
        if !bound(was) {
            continue;
        }
        if let mg_mdl::NodeKind::Mesh(m) = &mut n.kind
            && let mg_mdl::MeshExtra::Skin(s) = &mut m.extra
        {
            s.bones.iter_mut().for_each(|b| *b += from);
            let mut binds = vec![([0.0, 0.0, 0.0, 1.0], [0.0; 3]); from];
            binds.append(&mut s.inverse_bind);
            s.inverse_bind = binds;
        }
    }
    let stand_in = |&i: &usize| mg_mdl::Node::new(&skeleton.nodes[i].name, mg_mdl::NodeKind::Dummy);
    on.nodes.extend(nodes.iter().map(stand_in));
    Some((Arc::new(on), Skeleton { from, nodes }))
}

impl Composed {
    /// The models it draws: the base, then the parts.
    pub fn models(&self) -> impl Iterator<Item = &Arc<GpuModel>> {
        std::iter::once(&self.base.gpu).chain(self.parts.iter().map(|p| &p.gpu))
    }

    /// Loads the preview's models (`load` finds a model and its
    /// supermodels by name); `None` without a base model.
    pub fn new(
        gpu: &Gpu,
        preview: &Preview,
        load: &dyn Fn(&str) -> Option<Arc<Model>>,
    ) -> Option<Composed> {
        let base = place(gpu, &preview.base, None, load)?;
        let base_model = base.gpu.model.clone();
        let mut missing = Vec::new();
        // A part hangs from a node of the base: where the base has no such
        // node (a phenotype with another body: a bariaur's has no pelvis
        // or legs), the game draws no part.
        let hangs = |p: &&crate::Part| {
            p.attach.as_deref().is_none_or(|name| {
                base_model.nodes.iter().any(|n| n.name.eq_ignore_ascii_case(name))
            })
        };
        let parts = preview
            .parts
            .iter()
            .filter(hangs)
            .filter_map(|p| {
                let placed = place(gpu, p, Some(&base_model), load);
                if placed.is_none() {
                    missing.push(p.model.clone());
                }
                placed
            })
            .collect();
        let mut c = Composed {
            base,
            parts,
            idle: preview.idle.clone(),
            lights: preview.lights.clone(),
            lamps: Vec::new(),
            missing,
            failed: Default::default(),
        };
        // (Once: where they are as it stands, not as each frame poses it.)
        let name = c.base.gpu.model.name.clone();
        c.lamps = mg_render::guard::guarded(&name, || c.lamps_standing()).unwrap_or_default();
        Some(c)
    }

    /// The light nodes of its models as it stands at the start of its idle
    /// animation, in the base's space ([`Composed::lamps`]).
    fn lamps_standing(&self) -> Vec<anim::Lamp> {
        let has_light =
            |m: &Model| m.nodes.iter().any(|n| matches!(n.kind, mg_mdl::NodeKind::Light(_)));
        let base_model = &*self.base.gpu.model;
        if !has_light(base_model) && !self.parts.iter().any(|p| has_light(&p.gpu.model)) {
            return Vec::new();
        }
        let idle = self.idle.as_deref().and_then(|a| playing(a, |n| self.base.animation(n)));
        let base_pose = match idle {
            Some(a) => anim::pose(base_model, a, 0.0),
            None => self.base.gpu.rest.clone(),
        };
        let mut lamps = anim::lamps_layers(base_model, idle.as_slice(), 0.0, &base_pose);
        for p in &self.parts {
            if !has_light(&p.gpu.model) {
                continue;
            }
            let at = p.attach.and_then(|i| base_pose.get(i).copied()).unwrap_or(Mat4::IDENTITY);
            let hung = at * Mat4::from_scale(Vec3::splat(p.scale));
            let own = anim::lamps_layers(&p.gpu.model, &[], 0.0, &p.gpu.rest);
            lamps.extend(
                own.into_iter().map(|l| anim::Lamp { at: hung.transform_point3(l.at), ..l }),
            );
        }
        lamps
    }

    /// The base model.
    pub fn base(&self) -> &Arc<GpuModel> {
        &self.base.gpu
    }

    /// How many models hang from the base.
    pub fn part_count(&self) -> usize {
        self.parts.len()
    }

    /// The animations the base can play (its own and its supermodels').
    pub fn animations(&self) -> Vec<String> {
        self.base.anims.iter().map(|(n, _)| n.clone()).collect()
    }

    /// The base model's particle emitters, to simulate: the model, the
    /// animation it stands in and its nodes' places at `t`. `None` for a
    /// model without emitters (most).
    pub fn emitters(
        &self,
        animation: Option<&str>,
        t: f32,
    ) -> Option<(&Model, Option<&Animation>, Vec<Mat4>)> {
        let model = &*self.base.gpu.model;
        if !model.nodes.iter().any(|n| matches!(n.kind, mg_mdl::NodeKind::Emitter(_))) {
            return None;
        }
        let playing = animation.and_then(|name| self.base.animation(name));
        let pose = match playing {
            Some(a) => anim::pose(model, a, t),
            None => self.base.gpu.rest.clone(),
        };
        Some((model, playing, pose))
    }

    /// Instances at time `t` of `animation` (none: the rest pose), placed
    /// by `transform`.
    ///
    /// A model that breaks what poses it gives nothing, from then on: the
    /// failure is noted (`mg_render::guard`) and the toolset carries on.
    pub fn instances(&self, animation: Option<&str>, t: f32, transform: Mat4) -> Vec<Instance> {
        use std::sync::atomic::Ordering;
        if self.failed.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let name = &self.base.gpu.model.name;
        let posed = mg_render::guard::guarded(name, || self.posed(animation, t, transform));
        posed.unwrap_or_else(|| {
            self.failed.store(true, Ordering::Relaxed);
            Vec::new()
        })
    }

    fn posed(&self, animation: Option<&str>, t: f32, transform: Mat4) -> Vec<Instance> {
        let base_anim = animation.and_then(|a| playing(a, |n| self.base.animation(n)));
        let base_pose: Arc<Vec<Mat4>> = Arc::new(match &base_anim {
            Some(a) => anim::pose(&self.base.gpu.model, a, t),
            None => self.base.gpu.rest.clone(),
        });
        let instance = |p: &Placed, transform: Mat4, pose: Option<Arc<Vec<Mat4>>>| Instance {
            pose,
            env_map: p.env_map.clone(),
            plt_colors: p.colors,
            textures: p.textures.clone(),
            ..Instance::new(p.gpu.clone(), transform)
        };
        let mut out = vec![instance(&self.base, transform, Some(base_pose.clone()))];
        for p in &self.parts {
            let at = p.attach.and_then(|i| base_pose.get(i).copied()).unwrap_or(Mat4::IDENTITY);
            let hung = at * Mat4::from_scale(Vec3::splat(p.scale));
            let placed = transform * hung;
            // Animated parts play the animation on their own nodes: their
            // own (wings, tails), else the base's (robe and cloak bones).
            let pose = if p.animated {
                let own = animation.and_then(|a| p.animation(a)).or_else(|| {
                    ["creadyl", "cpause1", "pause1"].iter().find_map(|a| p.animation(a))
                });
                // (A skeleton's animation, the base's or its supermodel's,
                // moves the part as it moves the base: at the base's
                // scale. The part's very own keep theirs.)
                let worn = self.base.gpu.model.animation_scale;
                own.or(base_anim).map(|a| Arc::new(anim::pose_worn(&p.gpu.model, a, t, worn)))
            } else {
                p.skeleton_pose(&base_pose, hung).map(Arc::new)
            };
            out.push(instance(p, placed, pose));
        }
        out
    }

    /// The lights it casts, placed by `transform`: its appearance's
    /// ([`Composed::row_lights`]) and its models' light nodes.
    pub fn point_lights(&self, transform: Mat4) -> Vec<PointLight> {
        let mut lights = self.row_lights(transform);
        lights.extend(self.lamps.iter().filter_map(|l| l.placed(transform, &|_| None)));
        lights
    }

    /// The light of its appearance (placeables.2da `LightColor`), placed by
    /// `transform`; none when it is not illuminated.
    pub fn row_lights(&self, transform: Mat4) -> Vec<PointLight> {
        self.lights
            .iter()
            .filter(|l| l.on)
            .map(|l| {
                PointLight::new(transform.transform_point3(l.offset), l.color, l.radius, false, 4)
            })
            .collect()
    }

    /// How many light nodes its models have (the base's and the parts').
    pub fn lamp_count(&self) -> usize {
        self.lamps.len()
    }

    /// How much of the base model's surface faces a viewer in direction
    /// `towards` (from the model, in its space): its vertices' normals,
    /// summed as far as they point that way. A flat thing made to hang on
    /// a wall (a banner, wall weeds) faces one way only, and seen from
    /// behind shows nothing.
    pub fn facing(&self, towards: Vec3) -> f32 {
        let gm = &self.base.gpu;
        let mut sum = 0.0;
        for (i, node) in gm.model.nodes.iter().enumerate() {
            let mg_mdl::NodeKind::Mesh(mesh) = &node.kind else { continue };
            if !mesh.render {
                continue;
            }
            let at = gm.rest.get(i).copied().unwrap_or(Mat4::IDENTITY);
            for n in &mesh.normals {
                sum +=
                    at.transform_vector3(Vec3::from(*n)).normalize_or_zero().dot(towards).max(0.0);
            }
        }
        sum
    }

    /// Bounds of everything in the rest pose (for framing a camera).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        self.bounds_with(&self.base.gpu.rest, None)
    }

    /// The box around the base model's meshes that are not drawn (`render
    /// 0`), at rest: what a model of nothing to see is still the size of.
    /// The game's invisible object is a box 0.6 m across and 0.18 m tall
    /// that is never drawn, which Aurora outlines where it is selected.
    /// `None` for a model with none.
    pub fn hidden_bounds(&self) -> Option<(Vec3, Vec3)> {
        let gm = &self.base.gpu;
        let mut bounds: Option<(Vec3, Vec3)> = None;
        for (i, node) in gm.model.nodes.iter().enumerate() {
            let mg_mdl::NodeKind::Mesh(mesh) = &node.kind else { continue };
            if mesh.render {
                continue;
            }
            let at = gm.rest.get(i).copied().unwrap_or(Mat4::IDENTITY);
            for v in &mesh.vertices {
                let v = at.transform_point3(Vec3::from(*v));
                let (min, max) = bounds.unwrap_or((v, v));
                bounds = Some((min.min(v), max.max(v)));
            }
        }
        bounds
    }

    /// [`Composed::bounds`] as it stands in `animation` at `t`: a creature
    /// whose skeleton plays another body's animations at a scale stands
    /// taller or shorter than at rest, and a picture framed by its rest
    /// cut its head off.
    pub fn bounds_in(&self, animation: Option<&str>, t: f32) -> (Vec3, Vec3) {
        let playing = animation.and_then(|a| playing(a, |n| self.base.animation(n)));
        match (playing, animation) {
            (Some(a), Some(asked)) => {
                self.bounds_with(&anim::pose(&self.base.gpu.model, a, t), Some((asked, a, t)))
            }
            _ => self.bounds(),
        }
    }

    /// How far along a ray (from `origin` along `dir`, in the preview's own
    /// space; in units of `dir`) it first meets one of the drawn
    /// triangles, at rest: what a click is on, where the box around the
    /// whole is much more than the model. `None` past it all (and for a
    /// preview with no mesh to meet, an emitter alone: see
    /// [`Composed::has_meshes`]).
    pub fn ray_hit(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        let mut best: Option<f32> = None;
        let mut meet = |gm: &GpuModel, nodes: &[Mat4], to: Mat4| {
            for m in &gm.meshes {
                let Some(mesh) = gm.model.nodes.get(m.node).and_then(|n| n.mesh()) else {
                    continue;
                };
                let faces = mesh.faces.iter().map(|f| f.vertices);
                let triangles = faces.chain(mesh.drawn.iter().copied());
                let mut meets = |t: Option<f32>| {
                    if let Some(t) = t.filter(|&t| best.is_none_or(|b| t < b)) {
                        best = Some(t);
                    }
                };
                // A skinned mesh where its bones put it, as it is drawn.
                if let Some(places) = skinned_places(gm, m, nodes, to) {
                    let corner = |i: u32| places.get(i as usize).copied();
                    for [a, b, c] in triangles {
                        if let (Some(a), Some(b), Some(c)) = (corner(a), corner(b), corner(c)) {
                            meets(ray_triangle(origin, dir, a, b, c));
                        }
                    }
                    continue;
                }
                let node = nodes.get(m.node).copied().unwrap_or(Mat4::IDENTITY);
                // The ray in the mesh's space (an affine map keeps its
                // parameter).
                let inverse = (to * node).inverse();
                let (o, d) = (inverse.transform_point3(origin), inverse.transform_vector3(dir));
                // (Past its box, none of its triangles.)
                if ray_box(o, d, m.min, m.max).is_none() {
                    continue;
                }
                let corner = |i: u32| mesh.vertices.get(i as usize).copied().map(Vec3::from);
                for [a, b, c] in triangles {
                    if let (Some(a), Some(b), Some(c)) = (corner(a), corner(b), corner(c)) {
                        meets(ray_triangle(o, d, a, b, c));
                    }
                }
            }
        };
        let rest = &self.base.gpu.rest;
        meet(&self.base.gpu, rest, Mat4::IDENTITY);
        for p in &self.parts {
            let at = p.attach.and_then(|i| rest.get(i).copied()).unwrap_or(Mat4::IDENTITY);
            let hung = at * Mat4::from_scale(Vec3::splat(p.scale));
            let posed = p.skeleton_pose(rest, hung);
            meet(&p.gpu, posed.as_deref().unwrap_or(&p.gpu.rest), hung);
        }
        best
    }

    /// Whether it draws any mesh (not an emitter or a light alone).
    pub fn has_meshes(&self) -> bool {
        self.models().any(|m| !m.meshes.is_empty())
    }

    /// The box around it with the base's nodes at `pose` (model space); in
    /// an animation (`playing`: the one asked for, the base's and the
    /// time), the parts that play it (wings, tails, robes) as they stand in
    /// it too, as [`Composed::instances`] draws them.
    fn bounds_with(&self, pose: &[Mat4], playing: Option<(&str, &Animation, f32)>) -> (Vec3, Vec3) {
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let mut add = |gm: &GpuModel, nodes: &[Mat4], to: Mat4| {
            for m in &gm.meshes {
                let node = nodes.get(m.node).copied().unwrap_or(Mat4::IDENTITY);
                // A skinned mesh is where its bones put each vertex (as
                // the renderer draws it), not where it was bound: a
                // dragon's wings, bound spread out, are folded in its
                // pause, and the box around the spread was several times
                // the dragon.
                if let Some(places) = skinned_places(gm, m, nodes, to) {
                    for w in places {
                        min = min.min(w);
                        max = max.max(w);
                    }
                    continue;
                }
                for c in 0..8 {
                    let p = Vec3::new(
                        if c & 1 == 0 { m.min.x } else { m.max.x },
                        if c & 2 == 0 { m.min.y } else { m.max.y },
                        if c & 4 == 0 { m.min.z } else { m.max.z },
                    );
                    let w = (to * node).transform_point3(p);
                    min = min.min(w);
                    max = max.max(w);
                }
            }
        };
        add(&self.base.gpu, pose, Mat4::IDENTITY);
        for p in &self.parts {
            let at = p.attach.and_then(|i| pose.get(i).copied()).unwrap_or(Mat4::IDENTITY);
            let posed = playing.filter(|_| p.animated).and_then(|(asked, base, t)| {
                let find = |name: &str| p.animation(name);
                let own = find(asked)
                    .or_else(|| ["creadyl", "cpause1", "pause1"].iter().find_map(|a| find(a)));
                let worn = self.base.gpu.model.animation_scale;
                Some(anim::pose_worn(&p.gpu.model, own.or(Some(base))?, t, worn))
            });
            let hung = at * Mat4::from_scale(Vec3::splat(p.scale));
            let posed = posed.or_else(|| p.skeleton_pose(pose, hung));
            add(&p.gpu, posed.as_deref().unwrap_or(&p.gpu.rest), hung);
        }
        if min.x > max.x { (Vec3::splat(-1.0), Vec3::splat(1.0)) } else { (min, max) }
    }
}

/// A drawn mesh's skin, if it is skinned: its bones, and each vertex's
/// weights and place in the skin node's space.
type Skinned<'a> = (&'a mg_render::model::GpuSkin, &'a [[(u16, f32); 4]], &'a [[f32; 3]]);

fn skinned<'a>(gm: &'a GpuModel, m: &'a mg_render::model::GpuMesh) -> Option<Skinned<'a>> {
    let skin = m.skin.as_ref()?;
    let mesh = gm.model.nodes.get(m.node)?.mesh()?;
    match &mesh.extra {
        mg_mdl::MeshExtra::Skin(s) if s.weights.len() == mesh.vertices.len() => {
            Some((skin, &s.weights, &mesh.vertices))
        }
        _ => None,
    }
}

/// Where a skinned mesh's bones put each of its vertices, as the renderer
/// draws them (`nodes`: its model's pose; `to`: from the model's space);
/// `None` for a mesh that is not skinned.
fn skinned_places(
    gm: &GpuModel,
    m: &mg_render::model::GpuMesh,
    nodes: &[Mat4],
    to: Mat4,
) -> Option<Vec<Vec3>> {
    let (skin, weights, vertices) = skinned(gm, m)?;
    let node = nodes.get(m.node).copied().unwrap_or(Mat4::IDENTITY);
    let bones: Vec<Mat4> = skin
        .bones
        .iter()
        .zip(&skin.inverse_bind)
        // (A bone that is no node of the model holds the mesh where it
        // is, as the renderer has it.)
        .map(|(&b, bind)| to * nodes.get(b).copied().unwrap_or(Mat4::IDENTITY) * *bind)
        .collect();
    let place = |(v, weights): (&[f32; 3], &[(u16, f32); 4])| {
        let v = Vec3::from(*v);
        let (mut at, mut total) = (Vec3::ZERO, 0.0);
        for &(bone, weight) in weights {
            if let Some(m) = bones.get(usize::from(bone)).filter(|_| weight > 0.0) {
                at += m.transform_point3(v) * weight;
                total += weight;
            }
        }
        // (A vertex no bone holds stays with the mesh's node.)
        if total > 0.0 { at / total } else { (to * node).transform_point3(v) }
    };
    Some(vertices.iter().zip(weights).map(place).collect())
}

/// How far along a ray (from `origin` along `dir`) it meets the box
/// `min`..`max`, if it does: 0 from inside it (the slab test; the area
/// view's picking takes it too).
pub fn ray_box(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let (mut near, mut far) = (f32::MIN, f32::MAX);
    for axis in 0..3 {
        if dir[axis].abs() < 1e-9 {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return None;
            }
            continue;
        }
        let a = (min[axis] - origin[axis]) / dir[axis];
        let b = (max[axis] - origin[axis]) / dir[axis];
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far && far >= 0.0).then_some(near.max(0.0))
}

/// How far along a ray it meets a triangle (either face), ahead of its
/// origin (Möller–Trumbore).
pub fn ray_triangle(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let (ab, ac) = (b - a, c - a);
    let p = dir.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(ab);
    let v = dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = ac.dot(q) * inv;
    (t >= 0.0).then_some(t)
}

#[cfg(test)]
mod ray_tests {
    use super::*;

    #[test]
    fn a_rigid_part_s_skin_meshes_are_not_drawn() {
        use mg_mdl::{Mesh, MeshExtra, Node, NodeKind};
        let mesh =
            |extra| NodeKind::Mesh(Box::new(Mesh { render: true, extra, ..Default::default() }));
        let mut model = Model::default();
        model.nodes.push(Node::new("plain", mesh(MeshExtra::None)));
        model.nodes.push(Node::new("skin", mesh(MeshExtra::Skin(Default::default()))));
        let rigid = without_skins(Arc::new(model), None);
        let drawn: Vec<bool> = rigid.nodes.iter().map(|n| n.mesh().unwrap().render).collect();
        assert_eq!(drawn, [true, false]);
        // One without skins is the same model, not a copy.
        let mut plain = Model::default();
        plain.nodes.push(Node::new("plain", mesh(MeshExtra::None)));
        let plain = Arc::new(plain);
        assert!(Arc::ptr_eq(&without_skins(plain.clone(), None), &plain));
    }

    /// A body part whose mesh is a skin weighted to a bone of the
    /// skeleton, which is no node of the part.
    const SKINNED_PART: &str = "newmodel part\nsetsupermodel part NULL\n\
        classification character\nbeginmodelgeom part\nnode dummy part\n  parent NULL\n\
        endnode\nnode skin part_g\n  parent part\n  bitmap part\n\
        verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
        tverts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
        faces 1\n    0 1 2 1 0 1 2 0\n\
        weights 3\n    torso_g 1.0\n    torso_g 1.0\n    torso_g 1.0\n\
        endnode\nendmodelgeom part\ndonemodel part\n";

    /// A skeleton for it: the part hangs from `pelvis_g`, half a meter
    /// below `torso_g`.
    fn skeleton() -> Model {
        let text = "newmodel base\nsetsupermodel base NULL\nclassification character\n\
            beginmodelgeom base\nnode dummy base\n  parent NULL\nendnode\n\
            node dummy pelvis_g\n  parent base\n  position 0 0 0.5\nendnode\n\
            node dummy torso_g\n  parent base\n  position 0 0 1\nendnode\n\
            endmodelgeom base\ndonemodel base\n";
        Model::read(text.as_bytes()).unwrap()
    }

    /// The part as compiled against the skeleton (as Beamdog's HD bodies
    /// are): its bone is the skeleton's `torso_g` (node 2), and its bind
    /// pose has one entry per node of the skeleton.
    fn bound_part() -> Model {
        let mut part = Model::read(SKINNED_PART.as_bytes()).unwrap();
        let mg_mdl::NodeKind::Mesh(m) = &mut part.nodes[1].kind else { panic!("a mesh") };
        let count = m.vertices.len();
        let mg_mdl::MeshExtra::Skin(s) = &mut m.extra else { panic!("a skin") };
        s.bones = vec![2];
        s.weights = vec![[(0, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)]; count];
        s.inverse_bind = vec![([0.0, 0.0, 0.0, 1.0], [0.0; 3]); 3];
        part
    }

    #[test]
    fn a_body_part_s_skin_is_drawn_as_the_game_draws_it() {
        // As text, and compiled (on its own, not against the skeleton).
        let skeleton = skeleton();
        let drawn = |model: Model, body_part: bool| {
            let rigid = without_skins(Arc::new(model), body_part.then_some(&skeleton));
            rigid.nodes.iter().find_map(|n| n.mesh()).expect("a mesh").render
        };
        let text = SKINNED_PART;
        let as_text = Model::read(text.as_bytes()).unwrap();
        let compiled = mg_mdl::compile::compile(text.as_bytes(), &Default::default()).unwrap();
        let as_binary = Model::read(&compiled.binary).unwrap();
        assert!(matches!(as_binary.nodes[1].mesh().unwrap().extra, mg_mdl::MeshExtra::Skin(_)));
        assert!(drawn(as_text.clone(), true), "text, a body part: a plain mesh");
        assert!(!drawn(as_binary.clone(), true), "compiled, a body part: not drawn");
        assert!(!drawn(as_text, false), "text, a held item: not drawn");
        assert!(!drawn(as_binary, false), "compiled, a held item: not drawn");
        // Compiled against the skeleton: drawn as a body part, its bone
        // moved onto a stand-in for the skeleton's node.
        assert!(drawn(bound_part(), true), "bound to the skeleton, a body part: drawn");
        assert!(!drawn(bound_part(), false), "bound to the skeleton, a held item: not drawn");
        let (on, at) = on_skeleton(&bound_part(), &skeleton).expect("bound to the skeleton");
        assert_eq!((at.from, at.nodes.as_slice()), (2, [0, 1, 2].as_slice()));
        assert_eq!(on.nodes.len(), 5);
        assert_eq!(on.nodes[4].name, "torso_g");
        let mg_mdl::MeshExtra::Skin(s) = &on.nodes[1].mesh().unwrap().extra else { panic!() };
        assert_eq!((s.bones.as_slice(), s.inverse_bind.len()), ([4].as_slice(), 5));
        assert!(on_skeleton(&Model::read(text.as_bytes()).unwrap(), &skeleton).is_none());
    }

    /// A skin's bones are found by a depth-first walk of the skeleton's
    /// tree, whatever order its nodes are listed in.
    #[test]
    fn a_skeleton_s_tree_is_walked_depth_first() {
        // (Declared s, arm, head, hand under arm: file order is not the
        // tree's.)
        let text = "newmodel s\nsetsupermodel s NULL\nbeginmodelgeom s\n\
            node dummy s\n  parent NULL\nendnode\n\
            node dummy arm\n  parent s\nendnode\n\
            node dummy head\n  parent s\nendnode\n\
            node dummy hand\n  parent arm\nendnode\n\
            endmodelgeom s\ndonemodel s\n";
        let m = Model::read(text.as_bytes()).unwrap();
        let names: Vec<&str> = tree_order(&m).iter().map(|&i| m.nodes[i].name.as_str()).collect();
        assert_eq!(names, ["s", "arm", "hand", "head"]);
    }

    /// Beamdog's HD bodies showed only their head and pelvis (a builder's
    /// report): their other parts are skins compiled against the skeleton,
    /// which follow its bones, not the node they hang from.
    #[test]
    fn a_body_part_bound_to_the_skeleton_follows_its_bones() {
        mg_testkit::gpu::hold();
        let Some(gpu) = Gpu::headless() else {
            eprintln!("skipped: no GPU");
            return;
        };
        let (base, part) = (Arc::new(skeleton()), Arc::new(bound_part()));
        let load = |name: &str| match name {
            "base" => Some(base.clone()),
            "part" => Some(part.clone()),
            _ => None,
        };
        let part = crate::Part { attach: Some("pelvis_g".into()), ..crate::Part::new("part") };
        let preview = Preview { parts: vec![part], ..Preview::model("base") };
        let c = Composed::new(&gpu, &preview, &load).unwrap();
        // At `torso_g`, a meter up, not at `pelvis_g` where it hangs.
        let (min, max) = c.bounds();
        assert!((min.z - 1.0).abs() < 1e-4 && (max.z - 1.0).abs() < 1e-4, "{min} {max}");
        assert!((max.x - 1.0).abs() < 1e-4 && (max.y - 1.0).abs() < 1e-4, "{min} {max}");
        let instances = c.instances(None, 0.0, Mat4::IDENTITY);
        let pose = instances[1].pose.as_ref().expect("posed by the skeleton");
        let torso = instances[1].transform * pose[4];
        assert!(torso.w_axis.truncate().abs_diff_eq(Vec3::Z, 1e-5), "{torso}");
        // Clicked where it is drawn.
        assert!(
            c.ray_hit(Vec3::new(0.2, 0.2, 5.0), Vec3::NEG_Z)
                .is_some_and(|t| (t - 4.0).abs() < 1e-4)
        );
    }

    #[test]
    fn a_ray_meets_a_triangle_ahead_of_it() {
        let (a, b, c) = (Vec3::ZERO, Vec3::X, Vec3::Y);
        let down = Vec3::NEG_Z;
        let t = ray_triangle(Vec3::new(0.2, 0.2, 5.0), down, a, b, c).unwrap();
        assert!((t - 5.0).abs() < 1e-5);
        // From behind too (a model's faces are picked from either side).
        assert!(ray_triangle(Vec3::new(0.2, 0.2, -5.0), Vec3::Z, a, b, c).is_some());
        // Beside it, behind the origin, and along its plane: nothing.
        assert!(ray_triangle(Vec3::new(0.8, 0.8, 5.0), down, a, b, c).is_none());
        assert!(ray_triangle(Vec3::new(0.2, 0.2, -5.0), down, a, b, c).is_none());
        assert!(ray_triangle(Vec3::new(-1.0, 0.2, 0.0), Vec3::X, a, b, c).is_none());
        assert_eq!(ray_box(Vec3::new(0.5, 0.5, 5.0), down, Vec3::ZERO, Vec3::ONE), Some(4.0));
        assert_eq!(ray_box(Vec3::new(0.5, 0.5, 0.5), down, Vec3::ZERO, Vec3::ONE), Some(0.0));
        assert_eq!(ray_box(Vec3::new(2.0, 0.5, 5.0), down, Vec3::ZERO, Vec3::ONE), None);
        assert_eq!(ray_box(Vec3::new(0.5, 0.5, -1.0), down, Vec3::ZERO, Vec3::ONE), None);
    }

    /// A standing idle missing falls back to the other; a placeable's
    /// state missing does not: a corpse placeable on a creature's
    /// supermodel (`pause1`, no `default`) lies in its rest pose.
    #[test]
    fn only_a_standing_idle_falls_back_to_another() {
        use super::playing;
        use mg_mdl::Animation;
        let anims: Vec<Animation> = ["pause1", "deadfnt"]
            .map(|n| Animation { name: n.into(), ..Default::default() })
            .to_vec();
        let find = |n: &str| anims.iter().find(|a| a.name.eq_ignore_ascii_case(n));
        let name = |a: Option<&Animation>| a.map(|a| a.name.clone());
        assert_eq!(name(playing("default", find)), None);
        assert_eq!(name(playing("open", find)), None);
        assert_eq!(name(playing("deadfnt", find)), Some("deadfnt".into()));
        assert_eq!(name(playing("CPAUSE1", find)), Some("pause1".into()));
        assert_eq!(name(playing("pause1", find)), Some("pause1".into()));
    }
}
