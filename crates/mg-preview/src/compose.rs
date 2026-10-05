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

    /// Instances at time `t` of `animation` (none: the rest pose), placed
    /// by `transform`.
    ///
    /// A model that breaks what poses it gives nothing, from then on: the
    /// failure is noted (`mg_render::guard`) and the toolset carries on.
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
                own.or(base_anim).map(|a| Arc::new(anim::pose(&p.gpu.model, a, t)))
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

    /// Bounds of everything in the rest pose (for framing a camera).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let rest = &self.base.gpu.rest;
        let mut add = |gm: &GpuModel, to: Mat4| {
            for m in &gm.meshes {
                let node = gm.rest.get(m.node).copied().unwrap_or(Mat4::IDENTITY);
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
        add(&self.base.gpu, Mat4::IDENTITY);
        for p in &self.parts {
            let at = p.attach.and_then(|i| rest.get(i).copied()).unwrap_or(Mat4::IDENTITY);
            add(&p.gpu, at * Mat4::from_scale(Vec3::splat(p.scale)));
        }
        if min.x > max.x { (Vec3::splat(-1.0), Vec3::splat(1.0)) } else { (min, max) }
    }
}
