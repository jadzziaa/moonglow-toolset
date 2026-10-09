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
    /// Its own animations and its supermodels' (animated parts).
    anims: Vec<(String, Arc<Model>)>,
}

/// A preview ready to draw.
#[derive(Debug)]
pub struct Composed {
    base: Placed,
    parts: Vec<Placed>,
    /// The animation the preview stands in.
    pub idle: Option<String>,
    pub lights: Vec<PreviewLight>,
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
    let model = load(&part.model)?;
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
        anims,
    })
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
        Some(Composed {
            base,
            parts,
            idle: preview.idle.clone(),
            lights: preview.lights.clone(),
            missing,
            failed: Default::default(),
        })
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
        let playing = animation.and_then(|name| {
            let owner = self.base.anims.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
            owner.1.animation(name)
        });
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
        fn find<'a>(anims: &'a [(String, Arc<Model>)], name: &str) -> Option<&'a Animation> {
            anims
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .and_then(|(_, owner)| owner.animation(name))
        }
        // The animation asked for, else the other standing one: a model
        // whose appearance says full-body (`pause1`) may have a creature's
        // animations (`cpause1`), and stands in its rest pose without.
        let base_anim = animation.and_then(|a| {
            find(&self.base.anims, a)
                .or_else(|| ["pause1", "cpause1"].iter().find_map(|a| find(&self.base.anims, a)))
        });
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
            let placed = transform * at * Mat4::from_scale(Vec3::splat(p.scale));
            // Animated parts play the animation on their own nodes: their
            // own (wings, tails), else the base's (robe and cloak bones).
            let pose = if p.animated {
                let own = animation.and_then(|a| find(&p.anims, a)).or_else(|| {
                    ["creadyl", "cpause1", "pause1"].iter().find_map(|a| find(&p.anims, a))
                });
                // (A skeleton's animation, the base's or its supermodel's,
                // moves the part as it moves the base: at the base's
                // scale. The part's very own keep theirs.)
                let worn = self.base.gpu.model.animation_scale;
                own.or(base_anim).map(|a| Arc::new(anim::pose_worn(&p.gpu.model, a, t, worn)))
            } else {
                None
            };
            out.push(instance(p, placed, pose));
        }
        out
    }

    /// The preview's lights, placed by `transform`.
    pub fn point_lights(&self, transform: Mat4) -> Vec<PointLight> {
        self.lights
            .iter()
            .map(|l| {
                PointLight::new(transform.transform_point3(l.offset), l.color, l.radius, false, 4)
            })
            .collect()
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

    /// [`Composed::bounds`] as it stands in `animation` at `t`: a creature
    /// whose skeleton plays another body's animations at a scale stands
    /// taller or shorter than at rest, and a picture framed by its rest
    /// cut its head off.
    pub fn bounds_in(&self, animation: Option<&str>, t: f32) -> (Vec3, Vec3) {
        let find = |name: &str| {
            let owner = self.base.anims.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
            owner.1.animation(name)
        };
        let playing = animation
            .and_then(|a| find(a).or_else(|| ["pause1", "cpause1"].iter().find_map(|a| find(a))));
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
                let node = nodes.get(m.node).copied().unwrap_or(Mat4::IDENTITY);
                // The ray in the mesh's space (an affine map keeps its
                // parameter).
                let inverse = (to * node).inverse();
                let (o, d) = (inverse.transform_point3(origin), inverse.transform_vector3(dir));
                // (Past its box, none of its triangles.)
                if !ray_meets_box(o, d, m.min, m.max) {
                    continue;
                }
                let corner = |i: u32| mesh.vertices.get(i as usize).copied().map(Vec3::from);
                let faces = mesh.faces.iter().map(|f| f.vertices);
                for [a, b, c] in faces.chain(mesh.drawn.iter().copied()) {
                    let (Some(a), Some(b), Some(c)) = (corner(a), corner(b), corner(c)) else {
                        continue;
                    };
                    if let Some(t) = ray_triangle(o, d, a, b, c)
                        && best.is_none_or(|b| t < b)
                    {
                        best = Some(t);
                    }
                }
            }
        };
        let rest = &self.base.gpu.rest;
        meet(&self.base.gpu, rest, Mat4::IDENTITY);
        for p in &self.parts {
            let at = p.attach.and_then(|i| rest.get(i).copied()).unwrap_or(Mat4::IDENTITY);
            meet(&p.gpu, &p.gpu.rest, at * Mat4::from_scale(Vec3::splat(p.scale)));
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
                if let Some((skin, weights, vertices)) = skinned(gm, m) {
                    let bones: Vec<Mat4> = skin
                        .bones
                        .iter()
                        .zip(&skin.inverse_bind)
                        // (A bone that is no node of the model holds the
                        // mesh where it is, as the renderer has it.)
                        .map(|(&b, bind)| {
                            to * nodes.get(b).copied().unwrap_or(Mat4::IDENTITY) * *bind
                        })
                        .collect();
                    for (v, weights) in vertices.iter().zip(weights) {
                        let v = Vec3::from(*v);
                        let (mut at, mut total) = (Vec3::ZERO, 0.0);
                        for &(bone, weight) in weights {
                            if let Some(m) = bones.get(usize::from(bone)).filter(|_| weight > 0.0) {
                                at += m.transform_point3(v) * weight;
                                total += weight;
                            }
                        }
                        // (A vertex no bone holds stays with the mesh's node.)
                        let w =
                            if total > 0.0 { at / total } else { (to * node).transform_point3(v) };
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
                let find = |name: &str| {
                    let owner = p.anims.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
                    owner.1.animation(name)
                };
                let own = find(asked)
                    .or_else(|| ["creadyl", "cpause1", "pause1"].iter().find_map(|a| find(a)));
                let worn = self.base.gpu.model.animation_scale;
                Some(anim::pose_worn(&p.gpu.model, own.or(Some(base))?, t, worn))
            });
            let nodes = posed.as_deref().unwrap_or(&p.gpu.rest);
            add(&p.gpu, nodes, at * Mat4::from_scale(Vec3::splat(p.scale)));
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

/// Whether a ray meets the box `min`..`max`.
fn ray_meets_box(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> bool {
    let (mut near, mut far) = (f32::MIN, f32::MAX);
    for axis in 0..3 {
        if dir[axis].abs() < 1e-9 {
            if origin[axis] < min[axis] || origin[axis] > max[axis] {
                return false;
            }
            continue;
        }
        let a = (min[axis] - origin[axis]) / dir[axis];
        let b = (max[axis] - origin[axis]) / dir[axis];
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    near <= far && far >= 0.0
}

/// How far along a ray it meets a triangle (either face), ahead of its
/// origin (Möller–Trumbore).
fn ray_triangle(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
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
        assert!(ray_meets_box(Vec3::new(0.5, 0.5, 5.0), down, Vec3::ZERO, Vec3::ONE));
        assert!(!ray_meets_box(Vec3::new(2.0, 0.5, 5.0), down, Vec3::ZERO, Vec3::ONE));
    }
}
