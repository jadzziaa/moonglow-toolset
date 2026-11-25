//! The game's own kind of shadows: stencil shadow volumes
//! (`docs/research/notes_shaders.md` §G), the area view's Shadows quality
//! High ([`crate::Renderer::shadow_volumes`]); Fast is the shadow map
//! ([`crate::shadow`]).
//!
//! The game extrudes each casting mesh (a trimesh flagged `shadow`, not a
//! skin: the wiki's Model Shadows) away from the light in its
//! `vs_shadowvol`: the vertices facing the light move 0.1 m along it, the
//! others to the tile's bounding box's bottom (the sun) or to the light's
//! radius (a point light), never above the camera; counts the volumes in
//! the stencil buffer; and lays its shadow plane over the pixels they
//! mark. Moonglow does the same with volumes that are closed whatever the
//! mesh: each face that faces the light is a prism, from the face moved
//! 0.1 m to the face extruded, whose sides are drawn only where the
//! neighbouring face doesn't face the light too ([`volume_mesh`]). The
//! sun's and moon's volumes come of the faces whose front faces the light
//! alone, as the client's (`docs/findings.md`, "A face casts the sun's
//! shadow from its front only": a single wall with its back to the sun
//! casts nothing); a face turned from it is no prism, and an edge of many
//! faces casts no spike (the game's "shadow tearing": its volumes are
//! torn there), each face its own prism. A light's volumes of a mesh that
//! isn't closed (an open sheet, an edge of three faces or more, faces
//! wound against each other) come of both sides of its faces (not read
//! in the client), so a lone plank casts whichever way it faces. The
//! counts are z-fail ("Carmack's reverse"): right where the camera stands
//! inside a volume.
//!
//! What is counted against is the depth of what receives shadows (the
//! renderer draws it first): a copy of the scene's depth into a depth and
//! stencil target of the view's samples ([`ShadowVolumes`]), so that the
//! scene's own depth buffer, and every other pass, stay as they are.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, Weak};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};

use crate::scene::Camera;
use crate::shadow::{LightPlan, Plan};

/// A face's copy at the volume's near end: kept (moved 0.1 m along the
/// light) where its face faces the light, else gone.
const FRONT_CAP: u32 = 0;
/// A face's copy at the volume's far end (wound the other way): extruded
/// where its face faces the light, else gone.
const BACK_CAP: u32 = 1;
/// An edge's side, at one end: kept where the face it carries faces the
/// light, else extruded. Its quad is the edge's side where the two faces
/// it carries disagree, and nothing where they agree.
const SIDE: u32 = 2;
/// An edge's side's far end where only one face has the edge: always
/// extruded.
const FAR: u32 = 3;

/// How far the light's side of a volume is moved along the light, metres:
/// the game's `fMinOffset` ("inherited from old NWN"), which keeps a
/// caster's lit faces out of its own shadow.
const MIN_OFFSET: f32 = 0.1;

/// How far below the tile's bounding box (or the lowest receiver) a sun's
/// volume ends, metres: where it ended on the ground, the ground and the
/// volume's far end would be at one depth.
const BELOW: f32 = 0.1;

/// A vertex of a shadow volume as the volume shader reads it: its place
/// (in its mesh's node's space), its part of the volume, the plane of the
/// face it belongs to (`n·x + d`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct VolumeVertex {
    pos: [f32; 3],
    role: u32,
    plane: [f32; 4],
}

/// A mesh's shadow volume, before it is on the GPU: its vertices and
/// triangles, and what its edges are.
#[derive(Debug, Default)]
pub(crate) struct VolumeMesh {
    vertices: Vec<VolumeVertex>,
    indices: Vec<u32>,
    /// Every edge has two faces, wound against each other.
    pub(crate) closed: bool,
    /// Edges with one face.
    pub(crate) open_edges: usize,
    /// Edges with three faces or more, or two wound alike.
    pub(crate) torn_edges: usize,
}

/// The shadow volume of the mesh whose vertices are at `positions` and
/// whose faces are `triangles` (a face naming a vertex it hasn't, or of
/// no area, left out). Vertices at one place are one (the game's meshes
/// split them where their texture coordinates part), so that the faces
/// meet at their edges. `both_sides`: a mesh that isn't closed casts from
/// both sides of its faces (a light's volumes), else from their fronts
/// alone (the sun's).
pub(crate) fn volume_mesh(
    positions: &[[f32; 3]],
    triangles: impl IntoIterator<Item = [u32; 3]>,
    both_sides: bool,
) -> VolumeMesh {
    use std::collections::HashMap;
    // (The same place: the same bits, but for -0.)
    let key = |p: [f32; 3]| p.map(|c| if c == 0.0 { 0u32 } else { c.to_bits() });
    let mut welded: HashMap<[u32; 3], u32> = HashMap::new();
    let mut places: Vec<Vec3> = Vec::new();
    let mut one = Vec::with_capacity(positions.len());
    for &p in positions {
        let next = places.len() as u32;
        let at = *welded.entry(key(p)).or_insert(next);
        if at == next {
            places.push(Vec3::from(p));
        }
        one.push(at);
    }
    let mut faces: Vec<([u32; 3], Vec4)> = Vec::new();
    for t in triangles {
        if t.iter().any(|&i| i as usize >= positions.len()) {
            continue;
        }
        let [a, b, c] = t.map(|i| one[i as usize]);
        if a == b || b == c || a == c {
            continue;
        }
        let (pa, pb, pc) = (places[a as usize], places[b as usize], places[c as usize]);
        let n = (pb - pa).cross(pc - pa);
        if !n.is_finite() || n.length_squared() < 1e-14 {
            continue;
        }
        let n = n.normalize();
        faces.push(([a, b, c], n.extend(-n.dot(pa))));
    }
    // The faces' edges, each way.
    let edges_of = |faces: &[([u32; 3], Vec4)]| {
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for ([a, b, c], _) in faces {
            for e in [(*a, *b), (*b, *c), (*c, *a)] {
                *edges.entry(e).or_default() += 1;
            }
        }
        edges
    };
    let edges = edges_of(&faces);
    let shared = |edges: &HashMap<(u32, u32), u32>, (a, b): (u32, u32)| {
        edges.get(&(a, b)) == Some(&1) && edges.get(&(b, a)) == Some(&1)
    };
    let mut mesh = VolumeMesh::default();
    let mut undirected: HashMap<(u32, u32), (u32, u32)> = HashMap::new();
    for (&(a, b), &n) in &edges {
        let e = undirected.entry((a.min(b), a.max(b))).or_default();
        if a < b { e.0 += n } else { e.1 += n }
    }
    for &(there, back) in undirected.values() {
        match (there, back) {
            (1, 1) => {}
            (1, 0) | (0, 1) => mesh.open_edges += 1,
            _ => mesh.torn_edges += 1,
        }
    }
    mesh.closed = mesh.open_edges == 0 && mesh.torn_edges == 0;
    // (Not closed, cast from both sides: each face cast from both its
    // sides, so that every edge of one face is shared with the face turned
    // over, and the volume is closed whichever way the face is turned.
    // From the fronts alone, a face's open edges are sides extruded at
    // both ends: each face lit is a closed prism, one turned away
    // nothing.)
    let edges = if mesh.closed || !both_sides {
        edges
    } else {
        let turned: Vec<_> = faces.iter().map(|&([a, b, c], p)| ([a, c, b], -p)).collect();
        faces.extend(turned);
        edges_of(&faces)
    };
    let push = |mesh: &mut VolumeMesh, at: u32, plane: Vec4, role: u32| {
        mesh.vertices.push(VolumeVertex {
            pos: places[at as usize].to_array(),
            role,
            plane: plane.to_array(),
        });
        mesh.vertices.len() as u32 - 1
    };
    // Where each face lies, by its edges, for the shared sides.
    let mut by_edge: HashMap<(u32, u32), Vec4> = HashMap::new();
    for &([a, b, c], plane) in &faces {
        for e in [(a, b), (b, c), (c, a)] {
            by_edge.insert(e, plane);
        }
    }
    for &([a, b, c], plane) in &faces {
        let f = [a, b, c].map(|v| push(&mut mesh, v, plane, FRONT_CAP));
        mesh.indices.extend([f[0], f[1], f[2]]);
        let k = [a, b, c].map(|v| push(&mut mesh, v, plane, BACK_CAP));
        mesh.indices.extend([k[0], k[2], k[1]]);
        for (u, v) in [(a, b), (b, c), (c, a)] {
            // (A side between two faces is that of the one whose edge
            // runs from the lower vertex: once.)
            let (near, far) = if shared(&edges, (u, v)) {
                if u > v {
                    continue;
                }
                (SIDE, by_edge[&(v, u)])
            } else {
                (FAR, Vec4::ZERO)
            };
            let a0 = push(&mut mesh, u, plane, SIDE);
            let a1 = push(&mut mesh, v, plane, SIDE);
            let b1 = push(&mut mesh, v, far, near);
            let b0 = push(&mut mesh, u, far, near);
            mesh.indices.extend([a1, a0, b0, a1, b0, b1]);
        }
    }
    mesh
}

/// A mesh's shadow volume on the GPU, made the first time it casts one.
#[derive(Debug)]
pub(crate) struct GpuVolume {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    /// The mesh is closed: cast from both sides of its faces, the volume
    /// is the same.
    closed: bool,
}

/// Which mesh a volume is of: its model's [`crate::GpuModel::id`], its
/// node, and whether it is cast from both sides of its faces, in one
/// number ([`key_of`]).
type Key = u64;

/// The [`Key`] of node `node` of model `model`: the model's id above
/// 24 bits, the node's number (below 2²³) and `both_sides`.
fn key_of(model: u64, node: usize, both_sides: bool) -> Key {
    (model << 24) | ((node as u64 & 0x7f_ffff) << 1) | u64::from(both_sides)
}

/// The model's id of a [`Key`].
fn model_of(key: Key) -> u64 {
    key >> 24
}

/// A mesh's volume as the renderer keeps it ([`VolumeMeshes`]).
#[derive(Debug, Clone)]
enum Kept {
    /// Being made apart.
    Asked,
    /// Made: `None` for a mesh with no faces to cast with.
    Made(Option<Arc<GpuVolume>>),
    /// Cast from both sides, its volume from its faces' fronts: the mesh
    /// is closed.
    Front,
}

/// How long a model's volumes are kept while none of them is cast,
/// seconds of the renderer's frames (as long as a texture not drawn): the
/// area view drawn with Fast, or not drawn.
const STALE_AFTER: u32 = 30;

/// How many threads make volumes apart.
const MAKERS: usize = 2;

/// The shadow volume of node `node` of `model`, cast from its faces'
/// fronts alone or (`both_sides`) from both sides where it isn't closed
/// (where it is, [`Kept::Front`]): on `device`.
fn make(device: &wgpu::Device, model: &mg_mdl::Model, node: usize, both_sides: bool) -> Kept {
    use wgpu::util::DeviceExt;
    let Some(mg_mdl::NodeKind::Mesh(m)) = model.nodes.get(node).map(|n| &n.kind) else {
        return Kept::Made(None);
    };
    let v = volume_mesh(&m.vertices, m.triangles(), both_sides);
    if both_sides && v.closed {
        return Kept::Front;
    }
    if v.indices.is_empty() {
        return Kept::Made(None);
    }
    let buffer = |contents, usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow volume"),
            contents,
            usage,
        })
    };
    Kept::Made(Some(Arc::new(GpuVolume {
        vertices: buffer(bytemuck::cast_slice(&v.vertices), wgpu::BufferUsages::VERTEX),
        indices: buffer(bytemuck::cast_slice(&v.indices), wgpu::BufferUsages::INDEX),
        count: v.indices.len() as u32,
        closed: v.closed,
    })))
}

/// A volume to make apart: its key, its model's faces, its node and
/// whether it is cast from both sides.
struct Job {
    key: Key,
    model: Arc<mg_mdl::Model>,
    node: usize,
    both_sides: bool,
}

/// The threads that make volumes apart from the frames (as textures are
/// read apart): they take jobs and give back what they made, uploaded.
/// Joined when it is dropped, as the shader worker is (a thread holding
/// the GPU as the process ends crashed in Vulkan's validation layer).
struct Maker {
    ask: Option<Sender<Job>>,
    made: Receiver<(Key, Kept)>,
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Maker {
    fn new(
        gpu: &crate::Gpu,
        waker: &Arc<Mutex<Option<crate::game::Waker>>>,
        held: &Arc<AtomicBool>,
    ) -> Option<Maker> {
        let (ask, jobs) = std::sync::mpsc::channel::<Job>();
        let (give, made) = std::sync::mpsc::channel();
        let jobs = Arc::new(Mutex::new(jobs));
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::new();
        for n in 0..MAKERS {
            let (gpu, jobs, give) = (gpu.clone(), jobs.clone(), give.clone());
            let (waker, held, stop) = (waker.clone(), held.clone(), stop.clone());
            let spawned = std::thread::Builder::new()
                .name(format!("moonglow shadow volumes {n}"))
                .spawn(move || {
                    loop {
                        let job = match jobs.lock() {
                            Ok(rx) => rx.recv(),
                            Err(_) => return,
                        };
                        let Ok(Job { key, model, node, both_sides }) = job else { return };
                        // (Held, in a test: nothing made until it is let go.)
                        while held.load(Ordering::SeqCst) && !stop.load(Ordering::SeqCst) {
                            std::thread::sleep(std::time::Duration::from_millis(2));
                        }
                        if stop.load(Ordering::SeqCst) {
                            return;
                        }
                        let kept = crate::guard::guarded(&model.name, || {
                            make(&gpu.device, &model, node, both_sides)
                        })
                        .unwrap_or(Kept::Made(None));
                        if give.send((key, kept)).is_err() {
                            return;
                        }
                        let wake = waker.lock().ok().and_then(|w| w.clone());
                        if let Some(w) = wake {
                            (w.0)();
                        }
                    }
                });
            match spawned {
                Ok(t) => threads.push(t),
                Err(_) => break,
            }
        }
        // (No thread to make them on: made in the frame.)
        (!threads.is_empty()).then_some(Maker { ask: Some(ask), made, stop, threads })
    }
}

impl Drop for Maker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.ask = None;
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl std::fmt::Debug for Maker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Maker")
    }
}

/// The kept volumes, each in a place of its own: found by its key, or at
/// once at the place its mesh remembers ([`crate::model::GpuMesh::volume_at`],
/// checked against the key: a place another renderer's, or let go of
/// and taken again, is looked up by the key). A key looked up in a map
/// for each caster of each frame cost 0.1 ms more than the meshes' own
/// slots had in area084's 4,800 casters.
#[derive(Debug, Default)]
struct Slab {
    /// Each place's key (0: a free place, which no key is: a model's id
    /// is 1 or more) and its volume.
    places: Vec<(Key, Kept)>,
    by_key: crate::game::Fast<Key, u32>,
    free: Vec<u32>,
}

impl Slab {
    fn find(&self, key: Key, hint: &AtomicU32) -> Option<u32> {
        let at = hint.load(Ordering::Relaxed);
        if self.places.get(at as usize).is_some_and(|p| p.0 == key) {
            return Some(at);
        }
        let at = *self.by_key.get(&key)?;
        hint.store(at, Ordering::Relaxed);
        Some(at)
    }

    #[cfg(test)]
    fn get(&self, key: Key, hint: &AtomicU32) -> Option<&Kept> {
        self.find(key, hint).map(|at| &self.places[at as usize].1)
    }

    fn get_mut(&mut self, key: Key) -> Option<&mut Kept> {
        let at = *self.by_key.get(&key)?;
        Some(&mut self.places[at as usize].1)
    }

    /// Keeps `kept` as `key`'s, its place remembered in `hint`: says
    /// where.
    fn insert(&mut self, key: Key, kept: Kept, hint: &AtomicU32) -> u32 {
        let at = match self.by_key.get(&key) {
            Some(&at) => at,
            None => {
                let at = self.free.pop().unwrap_or_else(|| {
                    self.places.push((0, Kept::Made(None)));
                    self.places.len() as u32 - 1
                });
                self.by_key.insert(key, at);
                at
            }
        };
        self.places[at as usize] = (key, kept);
        hint.store(at, Ordering::Relaxed);
        at
    }

    /// The volume made at place `at`, if one is.
    fn volume(&self, at: u32) -> Option<&GpuVolume> {
        match self.places.get(at as usize) {
            Some((_, Kept::Made(Some(v)))) => Some(v),
            _ => None,
        }
    }

    /// Lets go of the volumes whose key `keep` says no to.
    fn retain(&mut self, keep: impl Fn(Key) -> bool) {
        for (i, place) in self.places.iter_mut().enumerate() {
            if place.0 != 0 && !keep(place.0) {
                self.by_key.remove(&place.0);
                *place = (0, Kept::Made(None));
                self.free.push(i as u32);
            }
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.by_key.len()
    }

    #[cfg(test)]
    fn keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.by_key.keys().copied()
    }

    fn values(&self) -> impl Iterator<Item = &Kept> {
        self.places.iter().filter(|p| p.0 != 0).map(|p| &p.1)
    }
}

/// The meshes' shadow volumes, kept by the renderer (not by the meshes:
/// so that they can be let go of): each made the first time its mesh
/// casts with High, in the frame or apart from it ([`VolumeMeshes::apart`]),
/// let go of with its model, or when none of its model's has been cast
/// for [`STALE_AFTER`].
#[derive(Debug)]
pub(crate) struct VolumeMeshes {
    kept: Slab,
    /// The models whose meshes have volumes kept: whether each is still
    /// there ([`crate::GpuModel::life`]), and when one of them was last
    /// cast.
    models: crate::game::Fast<u64, (Weak<()>, u32)>,
    /// The model last stamped in this frame.
    stamped: Option<u64>,
    /// This frame's time ([`crate::Renderer`]'s kept clock, seconds).
    now: u32,
    /// Volumes not made yet are made apart, a caster casting nothing
    /// until its volume is: the frame never waits for one. Off, the frame
    /// makes them (a picture drawn once).
    pub(crate) apart: bool,
    maker: Option<Maker>,
    /// Called as each volume made apart is ready.
    waker: Arc<Mutex<Option<crate::game::Waker>>>,
    /// Nothing is made apart while it is on (a test's).
    held: Arc<AtomicBool>,
    /// How many volumes made apart have been taken.
    pub(crate) taken: u64,
}

impl VolumeMeshes {
    fn new(waker: Arc<Mutex<Option<crate::game::Waker>>>, held: Arc<AtomicBool>) -> VolumeMeshes {
        VolumeMeshes {
            kept: Default::default(),
            models: Default::default(),
            stamped: None,
            now: 0,
            apart: false,
            maker: None,
            waker,
            held,
            taken: 0,
        }
    }

    /// At a frame's beginning (at `now`): takes the volumes made apart,
    /// and lets go of those of models that are gone or were not cast
    /// lately.
    pub(crate) fn begin(&mut self, now: u32) {
        self.now = now;
        self.stamped = None;
        if let Some(maker) = &self.maker {
            while let Ok((key, kept)) = maker.made.try_recv() {
                self.taken += 1;
                // (Not one let go of meanwhile, or made in a frame.)
                if let Some(slot @ Kept::Asked) = self.kept.get_mut(key) {
                    *slot = kept;
                }
            }
        }
        let before = self.models.len();
        self.models.retain(|_, (life, used)| {
            life.strong_count() > 0 && now.saturating_sub(*used) < STALE_AFTER
        });
        if self.models.len() != before {
            let models = &self.models;
            self.kept.retain(|k| models.contains_key(&model_of(k)));
        }
    }

    /// How many volumes are kept (made, with faces).
    pub(crate) fn count(&self) -> usize {
        self.kept.values().filter(|k| matches!(k, Kept::Made(Some(_)))).count()
    }

    /// The volume of `mesh` (of `model`), from its faces' fronts or
    /// (`both_sides`) from both sides: its place among those kept;
    /// `Ok(None)` for a mesh with no faces to cast with; `Err` while it is
    /// made apart (asked for).
    fn volume(
        &mut self,
        gpu: &crate::Gpu,
        model: &crate::GpuModel,
        mesh: &crate::model::GpuMesh,
        both_sides: bool,
    ) -> Result<Option<u32>, Waiting> {
        let (node, hint) = (mesh.node, &mesh.volume_at[usize::from(both_sides)]);
        if both_sides {
            match self.volume(gpu, model, mesh, false) {
                Ok(Some(at)) if self.kept.volume(at).is_some_and(|v| v.closed) => {
                    return Ok(Some(at));
                }
                Ok(None) => return Ok(None),
                // (Its front made, or being made: both sides asked for beside it.)
                _ => {}
            }
        }
        let key = key_of(model.id, node, both_sides);
        if let Some(at) = self.kept.find(key, hint) {
            match &self.kept.places[at as usize].1 {
                Kept::Made(v) => return Ok(v.as_ref().map(|_| at)),
                // (The front's, being made: the mesh is closed.)
                Kept::Front => return Err(Waiting),
                Kept::Asked if self.apart => return Err(Waiting),
                Kept::Asked => {}
            }
        }
        let now = self.now;
        (self.models.entry(model.id)).or_insert_with(|| (Arc::downgrade(&model.life), now));
        if self.apart {
            if self.maker.is_none() {
                self.maker = Maker::new(gpu, &self.waker, &self.held);
            }
            let ask = self.maker.as_ref().and_then(|m| m.ask.as_ref());
            if let Some(ask) = ask
                && ask.send(Job { key, model: model.model.clone(), node, both_sides }).is_ok()
            {
                self.kept.insert(key, Kept::Asked, hint);
                return Err(Waiting);
            }
        }
        let made = crate::guard::guarded(&model.model.name, || {
            make(&gpu.device, &model.model, node, both_sides)
        })
        .unwrap_or(Kept::Made(None));
        let (front, some) = (matches!(made, Kept::Front), matches!(made, Kept::Made(Some(_))));
        let at = self.kept.insert(key, made, hint);
        if front {
            // (Its front, made above: closed.)
            return self.volume(gpu, model, mesh, false);
        }
        Ok(some.then_some(at))
    }
}

/// A volume is being made apart ([`VolumeMeshes::apart`]).
#[derive(Debug, Clone, Copy)]
struct Waiting;

/// A caster as the volume shader reads it: from its node to the world,
/// its light in its node's space (for which way its faces face) and in
/// the world (the direction the sun's light goes, w −1; a point light's
/// place and radius), where the sun's volume ends (z).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct VolumeCaster {
    world: [[f32; 4]; 4],
    light_local: [f32; 4],
    light_world: [f32; 4],
    clip: [f32; 4],
}

/// The view as the volume shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct VolumeFrame {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    /// The higher of the camera's and its focus's heights, its near and
    /// far planes.
    camera: [f32; 4],
}

/// The volume shader (`vs_shadowvol` as the game has it, for Moonglow's
/// volumes) and the depth copy's.
pub(crate) const VOLUME: &str = "
struct Frame { view: mat4x4<f32>, projection: mat4x4<f32>, camera: vec4<f32> }
struct Caster { world: mat4x4<f32>, light_local: vec4<f32>, light_world: vec4<f32>, clip: vec4<f32> }
@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> casters: array<Caster>;
@vertex fn vs(@location(0) pos: vec3<f32>, @location(1) role: u32, @location(2) plane: vec4<f32>, @builtin(instance_index) i: u32) -> @builtin(position) vec4<f32> {
    let c = casters[i];
    let lit = dot(plane, c.light_local) > 0.0;
    // (A cap of a face turned from the light: nothing, all three corners
    // at one point.)
    if (role <= 1u && !lit) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    let extruded = role == 3u || (role == 1u) || (role == 2u && !lit);
    let world = (c.world * vec4<f32>(pos, 1.0)).xyz;
    var along: vec3<f32>;
    var offset = 0.1;
    if (c.light_world.w < 0.0) {
        along = c.light_world.xyz;
        let down = min(along.z, -0.001);
        if (extruded) { offset = max(0.1, (c.clip.x - world.z) / down); }
        // (Shadows never above the camera or what it looks at.)
        offset = max(offset, (frame.camera.x - world.z) / down);
        offset = min(offset, 10000.0);
    } else {
        let v = world - c.light_world.xyz;
        let length = max(length(v), 1e-4);
        along = v / length;
        if (extruded) { offset = max(0.1, c.light_world.w - length); }
    }
    var p = (frame.view * vec4<f32>(world, 1.0)).xyz;
    let d = (frame.view * vec4<f32>(along, 0.0)).xyz;
    // (Not past the far plane: moved back along the light to it, as the
    // game's.)
    let far = -frame.camera.z + 0.001;
    var z = p.z + d.z * offset;
    if (d.z < -0.001 && z < far) {
        offset = -(p.z - far) / d.z;
        z = far;
    }
    p = vec3<f32>(p.xy + offset * d.xy, z);
    return frame.projection * vec4<f32>(p, 1.0);
}
";

/// The depth copy's shader, for `samples` samples a pixel: each sample's
/// depth as the scene's.
pub(crate) fn copy_source(samples: u32) -> String {
    let (depth, sample, load) = if samples > 1 {
        (
            crate::framebuffer::depth_read_type(samples),
            ", @builtin(sample_index) s: u32",
            "textureLoad(depth, vec2<i32>(frag.xy), i32(s)).x",
        )
    } else {
        (crate::framebuffer::depth_read_type(1), "", "textureLoad(depth, vec2<i32>(frag.xy), 0).x")
    };
    let vs = crate::framebuffer::FULLSCREEN_VS;
    format!(
        "@group(0) @binding(0) var depth: {depth};
{vs}@fragment fn fs(@builtin(position) frag: vec4<f32>{sample}) -> @builtin(frag_depth) f32 {{
    return {load};
}}
"
    )
}

/// The depth and stencil format the volumes are counted in: 32-bit float
/// depth where the device has it (the scene's depth as it is), else 24
/// bits (every backend has it).
pub(crate) fn stencil_format(device: &wgpu::Device) -> wgpu::TextureFormat {
    if device.features().contains(wgpu::Features::DEPTH32FLOAT_STENCIL8) {
        wgpu::TextureFormat::Depth32FloatStencil8
    } else {
        wgpu::TextureFormat::Depth24PlusStencil8
    }
}

/// The casters of one count (the sun's, or the lights'), and their
/// buffers (apart: a buffer written twice in a frame holds the last
/// write for both).
#[derive(Debug)]
struct Set {
    frame: wgpu::Buffer,
    casters: wgpu::Buffer,
    params: wgpu::Buffer,
    /// Bind group on `(frame, casters)`; invalidated when `casters` grows.
    volume_group: Option<wgpu::BindGroup>,
}

impl Set {
    fn new(device: &wgpu::Device) -> Set {
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Set {
            frame: buffer(
                "shadow volume view",
                std::mem::size_of::<VolumeFrame>() as u64,
                wgpu::BufferUsages::UNIFORM,
            ),
            casters: buffer("shadow volume casters", 112 * 256, wgpu::BufferUsages::STORAGE),
            params: buffer(
                "shadow volume darkening",
                crate::shadow::DARKEN_UNIFORM_SIZE,
                wgpu::BufferUsages::UNIFORM,
            ),
            volume_group: None,
        }
    }
}

/// The pipelines of a target's samples (and colour format, for the
/// darkening).
#[derive(Debug)]
struct Pipelines {
    copy: wgpu::RenderPipeline,
    copy_layout: wgpu::BindGroupLayout,
    volumes: wgpu::RenderPipeline,
    volume_layout: wgpu::BindGroupLayout,
    /// The sun's darkening and the lights'.
    darken: [wgpu::RenderPipeline; 2],
    darken_layout: wgpu::BindGroupLayout,
}

/// A view's depth and stencil target for the volumes, its size and
/// samples: kept from one of the view's frames to the next
/// ([`crate::views`]) while it is drawn with High shadows.
pub(crate) type StencilTarget = (wgpu::TextureView, (u32, u32), u32);

/// What draws into a view's stencil target and reads it, made at the
/// first frame with High shadows.
#[derive(Debug)]
pub(crate) struct ShadowVolumes {
    format: wgpu::TextureFormat,
    pipelines: std::collections::HashMap<(wgpu::TextureFormat, u32), Pipelines>,
    sun: Set,
    lights: Set,
    /// The meshes' volumes.
    pub(crate) meshes: VolumeMeshes,
}

/// What [`ShadowVolumes::lay`] counts: a set's casters, each mesh's in a
/// row.
/// (A caster whose volume is still being made is left out:
/// `pending` counts them.)
struct Counted {
    /// Each mesh's volume's place among those kept, and its casters.
    meshes: Vec<(u32, std::ops::Range<u32>)>,
    casters: Vec<VolumeCaster>,
    pending: usize,
}

impl Counted {
    fn new() -> Self {
        Counted { meshes: Vec::new(), casters: Vec::new(), pending: 0 }
    }
}

/// The darkening's look of a set: its shadows, whether they are the
/// lights'.
type Look = (crate::scene::SunShadows, bool);

impl VolumeMeshes {
    /// The sun's casters of `plan` as the volume shader takes them: the
    /// rigid ones (the game's skins cast none), each mesh's in a row.
    fn sun_casters(&mut self, gpu: &crate::Gpu, plan: &Plan) -> Counted {
        let to_light = plan.direction.normalize_or(Vec3::Z);
        let light_world = (-to_light).extend(-1.0);
        let mut counted = Counted::new();
        for c in plan.casters.iter().filter(|c| c.bones.is_none()) {
            self.stamp(c.model.id);
            match self.volume(gpu, c.model, c.mesh, false) {
                Ok(Some(at)) => {
                    let clip = c.bottom.unwrap_or(plan.floor) - BELOW;
                    let local = c.world.inverse() * to_light.extend(0.0);
                    push(&mut counted, at, c.world, local, light_world, clip);
                }
                Ok(None) => {}
                Err(Waiting) => counted.pending += 1,
            }
        }
        counted
    }

    /// `model`'s volumes cast in this frame: kept.
    fn stamp(&mut self, model: u64) {
        if self.stamped != Some(model) {
            self.stamped = Some(model);
            if let Some(m) = self.models.get_mut(&model) {
                m.1 = self.now;
            }
        }
    }

    /// The lights' casters of `plan`: each creature's rigid meshes, from
    /// its light.
    fn light_casters(&mut self, gpu: &crate::Gpu, plan: &LightPlan) -> Counted {
        let mut counted = Counted::new();
        for (range, &(at, radius)) in plan.ranges.iter().zip(&plan.sources) {
            let light_world = at.extend(radius.max(MIN_OFFSET));
            let casters = &plan.casters[range.start as usize..range.end as usize];
            for c in casters.iter().filter(|c| c.bones.is_none()) {
                self.stamp(c.model.id);
                let volume = match self.volume(gpu, c.model, c.mesh, true) {
                    Ok(Some(v)) => v,
                    Ok(None) => continue,
                    Err(Waiting) => {
                        counted.pending += 1;
                        continue;
                    }
                };
                let local = c.world.inverse() * at.extend(1.0);
                push(&mut counted, volume, c.world, local, light_world, 0.0);
            }
        }
        counted
    }
}

impl ShadowVolumes {
    /// `waker`: called as each volume made apart is ready; `held`:
    /// nothing is made apart while it is on.
    pub(crate) fn new(
        device: &wgpu::Device,
        waker: Arc<Mutex<Option<crate::game::Waker>>>,
        held: Arc<AtomicBool>,
    ) -> ShadowVolumes {
        ShadowVolumes {
            format: stencil_format(device),
            pipelines: Default::default(),
            sun: Set::new(device),
            lights: Set::new(device),
            meshes: VolumeMeshes::new(waker, held),
        }
    }

    /// Lays the shadows of `sun` and of `lights` over what is drawn in
    /// `target` (of `format`, multisampled by its samples), as deep as
    /// `depth` (the scene's) says: each set's volumes counted in the
    /// view's stencil target (`made`: made where the view has none of the
    /// picture's size and samples), then black laid over what they mark
    /// as the game's shadow plane lays it, for `camera` seeing a picture
    /// of `size`; `fog`: the scene's. Says how many casters cast nothing
    /// for their volumes still being made apart.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn lay(
        &mut self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        (target_view, depth): (&wgpu::TextureView, &wgpu::TextureView),
        mask: &crate::shadow::Mask,
        format: (wgpu::TextureFormat, u32),
        (sun, lights): (Option<&Plan>, Option<&LightPlan>),
        camera: &Camera,
        size: (u32, u32),
        fog: Option<crate::scene::Fog>,
        made: &mut Option<StencilTarget>,
        mut copy_cache: Option<&mut Option<wgpu::BindGroup>>,
        mut darken_cache: Option<&mut [Option<wgpu::BindGroup>; 2]>,
    ) -> usize {
        let samples = format.1;
        if made.as_ref().is_none_or(|(_, s, n)| *s != size || *n != samples) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow volumes"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            *made = Some((texture.create_view(&Default::default()), size, samples));
        }
        let Self { format: stencil_format, pipelines, sun: sun_set, lights: light_set, meshes } =
            self;
        let pipelines = pipelines
            .entry(format)
            .or_insert_with(|| Pipelines::new(&gpu.device, format.0, samples, *stencil_format));
        let Some((stencil, ..)) = made.as_ref() else { return 0 };
        let mut copied = false;
        let sets: [(Option<Counted>, &mut Set, Option<Look>); 2] = [
            (sun.map(|p| meshes.sun_casters(gpu, p)), sun_set, sun.map(|p| (p.shadows, false))),
            (
                lights.map(|p| meshes.light_casters(gpu, p)),
                light_set,
                lights.map(|p| (p.shadows, true)),
            ),
        ];
        let mut pending = 0;
        for (counted, set, look) in sets {
            let (Some(counted), Some((shadows, lit))) = (counted, look) else { continue };
            pending += counted.pending;
            if counted.casters.is_empty() {
                continue;
            }
            upload(gpu, set, &counted.casters, camera, size);
            let plan = lights.filter(|_| lit);
            let params = crate::shadow::stencil_uniform(shadows, plan, camera, size, fog);
            gpu.queue.write_buffer(&set.params, 0, &params);
            pipelines.count(
                gpu,
                encoder,
                (stencil, depth),
                (set, &counted, &meshes.kept),
                !copied,
                copy_cache.as_deref_mut(),
            );
            copied = true;
            pipelines.darken(
                gpu,
                encoder,
                (target_view, mask),
                (stencil, depth),
                (set, lit),
                darken_cache.as_deref_mut().map(|slots| &mut slots[usize::from(lit)]),
            );
        }
        pending
    }
}

/// Adds a caster of `volume` to `counted` (beside its mesh's others where
/// it follows them).
fn push(
    counted: &mut Counted,
    volume: u32,
    world: Mat4,
    light_local: Vec4,
    light_world: Vec4,
    clip: f32,
) {
    let i = counted.casters.len() as u32;
    counted.casters.push(VolumeCaster {
        world: world.to_cols_array_2d(),
        light_local: light_local.to_array(),
        light_world: light_world.to_array(),
        clip: [clip, 0.0, 0.0, 0.0],
    });
    match counted.meshes.last_mut() {
        Some((v, range)) if *v == volume && range.end == i => range.end += 1,
        _ => counted.meshes.push((volume, i..i + 1)),
    }
}

/// Uploads a set's casters and the view.
fn upload(
    gpu: &crate::Gpu,
    set: &mut Set,
    casters: &[VolumeCaster],
    camera: &Camera,
    size: (u32, u32),
) {
    let aspect = size.0 as f32 / size.1.max(1) as f32;
    let frame = VolumeFrame {
        view: camera.view().to_cols_array_2d(),
        projection: camera.projection(aspect).to_cols_array_2d(),
        camera: [camera.eye.z.max(camera.target.z), camera.near, camera.far, 0.0],
    };
    gpu.queue.write_buffer(&set.frame, 0, bytemuck::bytes_of(&frame));
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    if crate::renderer::upload(gpu, &mut set.casters, "shadow volume casters", usage, casters, 0) {
        set.volume_group = None;
    }
}

impl Pipelines {
    fn new(
        device: &wgpu::Device,
        color: wgpu::TextureFormat,
        samples: u32,
        stencil_format: wgpu::TextureFormat,
    ) -> Pipelines {
        let multisample = wgpu::MultisampleState { count: samples, ..Default::default() };
        let depth_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: crate::framebuffer::DEPTH_READ,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: samples > 1,
            },
            count: None,
        };
        let uniform = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout_of = |label, entries: &[wgpu::BindGroupLayoutEntry]| {
            let group = device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label, entries });
            let pipeline = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[Some(&group)],
                immediate_size: 0,
            });
            (group, pipeline)
        };
        // The depth copy: every sample's depth written as the scene's.
        let (copy_layout, copy_pipeline_layout) =
            layout_of(Some("shadow volume depth"), &[depth_entry(0)]);
        let copy_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow volume depth"),
            source: wgpu::ShaderSource::Wgsl(copy_source(samples).into()),
        });
        let copy = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow volume depth"),
            layout: Some(&copy_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &copy_module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &copy_module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: stencil_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample,
            multiview_mask: None,
            cache: None,
        });
        // The volumes: counted where they lie behind what receives (z-fail):
        // a back face up, a front face down.
        let storage = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let (volume_layout, volume_pipeline_layout) =
            layout_of(Some("shadow volumes"), &[uniform(0, wgpu::ShaderStages::VERTEX), storage]);
        let volume_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow volumes"),
            source: wgpu::ShaderSource::Wgsl(VOLUME.into()),
        });
        let face = |depth_fail_op| wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Always,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op,
            pass_op: wgpu::StencilOperation::Keep,
        };
        let volumes = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow volumes"),
            layout: Some(&volume_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &volume_module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<VolumeVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Uint32, 2 => Float32x4
                    ],
                })],
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: stencil_format,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState {
                    front: face(wgpu::StencilOperation::DecrementWrap),
                    back: face(wgpu::StencilOperation::IncrementWrap),
                    read_mask: 0xFF,
                    write_mask: 0xFF,
                },
                bias: Default::default(),
            }),
            multisample,
            multiview_mask: None,
            cache: None,
        });
        // The darkening: the game's plane over what the counts mark.
        let (darken_layout, darken_pipeline_layout) = layout_of(
            Some("shadow volume darkening"),
            &[uniform(0, wgpu::ShaderStages::FRAGMENT), depth_entry(1)],
        );
        let blend = crate::shadow::DARKEN_BLEND;
        let marked = wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::NotEqual,
            fail_op: wgpu::StencilOperation::Keep,
            depth_fail_op: wgpu::StencilOperation::Keep,
            pass_op: wgpu::StencilOperation::Keep,
        };
        let darken = [false, true].map(|lights| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shadow volume darkening"),
                source: wgpu::ShaderSource::Wgsl(
                    crate::shadow::darken_source(samples, lights, true).into(),
                ),
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shadow volume darkening"),
                layout: Some(&darken_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format: color,
                            blend: Some(blend),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        Some(crate::shadow::MASK_TARGET),
                    ],
                }),
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: stencil_format,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: wgpu::StencilState {
                        front: marked,
                        back: marked,
                        read_mask: 0xFF,
                        write_mask: 0,
                    },
                    bias: Default::default(),
                }),
                multisample,
                multiview_mask: None,
                cache: None,
            })
        });
        Pipelines { copy, copy_layout, volumes, volume_layout, darken, darken_layout }
    }

    /// Counts `counted`'s volumes in `stencil` (cleared), against the
    /// scene's `depth` copied into it first where `copy`.
    fn count(
        &self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        (stencil, depth): (&wgpu::TextureView, &wgpu::TextureView),
        (set, counted, kept): (&mut Set, &Counted, &Slab),
        copy: bool,
        copy_cache: Option<&mut Option<wgpu::BindGroup>>,
    ) {
        let mut scratch_copy = None;
        let copy_group = if copy {
            let slot = match copy_cache {
                Some(slot) => slot,
                None => &mut scratch_copy,
            };
            Some(slot.get_or_insert_with(|| {
                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("shadow volume depth"),
                    layout: &self.copy_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(depth),
                    }],
                })
            }))
        } else {
            None
        };
        let group = set.volume_group.get_or_insert_with(|| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow volumes"),
                layout: &self.volume_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: set.frame.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: set.casters.as_entire_binding() },
                ],
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow volumes"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: stencil,
                depth_ops: Some(wgpu::Operations {
                    load: if copy { wgpu::LoadOp::Clear(1.0) } else { wgpu::LoadOp::Load },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            }),
            ..Default::default()
        });
        if let Some(copy_group) = copy_group {
            pass.set_pipeline(&self.copy);
            pass.set_bind_group(0, &*copy_group, &[]);
            pass.draw(0..3, 0..1);
        }
        pass.set_pipeline(&self.volumes);
        pass.set_bind_group(0, &*group, &[]);
        for (at, instances) in &counted.meshes {
            let Some(volume) = kept.volume(*at) else { continue };
            pass.set_vertex_buffer(0, volume.vertices.slice(..));
            pass.set_index_buffer(volume.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..volume.count, 0, instances.clone());
        }
    }

    /// Lays black over what `stencil` marks in `target` (the sun's way,
    /// or the `lights`' way), its fade and fog from the scene's `depth`.
    fn darken(
        &self,
        gpu: &crate::Gpu,
        encoder: &mut wgpu::CommandEncoder,
        (target, mask): (&wgpu::TextureView, &crate::shadow::Mask),
        (stencil, depth): (&wgpu::TextureView, &wgpu::TextureView),
        (set, lights): (&Set, bool),
        darken_cache: Option<&mut Option<wgpu::BindGroup>>,
    ) {
        let mut scratch = None;
        let slot = match darken_cache {
            Some(slot) => slot,
            None => &mut scratch,
        };
        let group = slot.get_or_insert_with(|| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow volume darkening"),
                layout: &self.darken_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: set.params.as_entire_binding() },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(depth),
                    },
                ],
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow volume darkening"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    depth_slice: None,
                }),
                Some(mask.attachment(false)),
            ],
            // (Read only: the counts tested, nothing written.)
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: stencil,
                depth_ops: None,
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.darken[usize::from(lights)]);
        pass.set_bind_group(0, &*group, &[]);
        pass.set_stencil_reference(0);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where the volume shader puts a vertex for a light (`to_light`, the
    /// direction towards the sun, w 0; or a point light's place, w 1, and
    /// its `radius`), its node in the world: the sun's volumes ending at
    /// `clip`. `None`: a cap of a face turned from the light, which is
    /// nothing. (The shader's arithmetic, but the camera's limits.)
    fn placed(v: &VolumeVertex, light: Vec4, radius: f32, clip: f32) -> Option<Vec3> {
        let lit = Vec4::from(v.plane).dot(light) > 0.0;
        if v.role <= BACK_CAP && !lit {
            return None;
        }
        let extruded = v.role == FAR || v.role == BACK_CAP || (v.role == SIDE && !lit);
        let p = Vec3::from(v.pos);
        let (along, offset) = if light.w == 0.0 {
            let along = -light.truncate().normalize();
            let offset =
                if extruded { ((clip - p.z) / along.z).max(MIN_OFFSET) } else { MIN_OFFSET };
            (along, offset)
        } else {
            let v = p - light.truncate();
            let offset = if extruded { (radius - v.length()).max(MIN_OFFSET) } else { MIN_OFFSET };
            (v.normalize(), offset)
        };
        Some(p + along * offset)
    }

    /// The triangles a volume is drawn as for a light: those with an area.
    fn drawn(mesh: &VolumeMesh, light: Vec4, radius: f32, clip: f32) -> Vec<[Vec3; 3]> {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter_map(|t| {
                let [a, b, c] = t.map(|i| placed(&mesh.vertices[i as usize], light, radius, clip));
                let t = [a?, b?, c?];
                ((t[1] - t[0]).cross(t[2] - t[0]).length() > 1e-6).then_some(t)
            })
            .collect()
    }

    /// Whether triangles close a volume: each edge met once each way.
    fn closed(triangles: &[[Vec3; 3]]) -> bool {
        use std::collections::HashMap;
        let key = |p: Vec3| (p * 1e4).round().as_ivec3().to_array();
        let mut edges: HashMap<([i32; 3], [i32; 3]), i32> = HashMap::new();
        for t in triangles {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *edges.entry((key(a), key(b))).or_default() += 1;
                *edges.entry((key(b), key(a))).or_default() -= 1;
            }
        }
        edges.values().all(|&n| n == 0)
    }

    /// Whether a point is inside a closed volume: how many of its faces a
    /// ray from it crosses, counted by which way they face (the stencil's
    /// count, from far off).
    fn inside(triangles: &[[Vec3; 3]], p: Vec3) -> bool {
        // (A ray of no special direction, so that it meets no edge.)
        let ray = Vec3::new(0.123, 0.456, 0.789).normalize();
        let mut count = 0;
        for &[a, b, c] in triangles {
            let (e1, e2) = (b - a, c - a);
            let h = ray.cross(e2);
            let det = e1.dot(h);
            if det.abs() < 1e-9 {
                continue;
            }
            let s = p - a;
            let u = s.dot(h) / det;
            let q = s.cross(e1);
            let v = ray.dot(q) / det;
            let t = e2.dot(q) / det;
            if u >= 0.0 && v >= 0.0 && u + v <= 1.0 && t > 0.0 {
                count += if det > 0.0 { 1 } else { -1 };
            }
        }
        count != 0
    }

    /// A unit cube from 0 to 1, its faces wound outwards, its vertices
    /// split by face (as a model's are where its texture coordinates part).
    fn cube() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        let mut positions = Vec::new();
        let mut triangles = Vec::new();
        // (Each face: its corners anticlockwise seen from outside.)
        let faces: [[[f32; 3]; 4]; 6] = [
            [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
            [[0., 0., 0.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]],
            [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
            [[1., 1., 0.], [0., 1., 0.], [0., 1., 1.], [1., 1., 1.]],
            [[0., 1., 0.], [0., 0., 0.], [0., 0., 1.], [0., 1., 1.]],
            [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]],
        ];
        for f in faces {
            let n = positions.len() as u32;
            positions.extend(f);
            triangles.extend([[n, n + 1, n + 2], [n, n + 2, n + 3]]);
        }
        (positions, triangles)
    }

    /// The edges of a drawn volume's sides: the silhouette, as pairs of
    /// the caster's corners whose side quads have an area.
    fn sides(mesh: &VolumeMesh, light: Vec4, clip: f32) -> usize {
        // (A side's two triangles: one with two corners at the caster.)
        drawn(mesh, light, 0.0, clip)
            .iter()
            .filter(|t| {
                let near = t.iter().filter(|p| p.z > clip + 0.01).count();
                near == 2
            })
            .count()
    }

    /// A closed cube casts a closed volume: the sun straight above, its
    /// top and four sides (a silhouette of 4 edges, each a quad of two
    /// triangles); from a corner, a silhouette of 6 edges. A point above
    /// the cube's shadow on the ground is in it; one beside it is not; the
    /// cube's top, lit, is not (it lies 0.1 m before the volume).
    #[test]
    fn a_cube_casts_a_closed_volume_with_its_silhouette_for_sides() {
        let (positions, triangles) = cube();
        let mesh = volume_mesh(&positions, triangles, false);
        assert!(mesh.closed && mesh.open_edges == 0 && mesh.torn_edges == 0);
        let above = Vec4::new(0.0, 0.0, 1.0, 0.0);
        let clip = -3.0;
        let t = drawn(&mesh, above, 0.0, clip);
        assert!(closed(&t), "closed");
        // (The top's two triangles, twice: the near and far caps; four
        // sides of two triangles.)
        assert_eq!(t.len(), 4 + 8, "{t:?}");
        assert_eq!(sides(&mesh, above, clip), 4, "4 silhouette edges");
        assert!(inside(&t, Vec3::new(0.5, 0.5, -2.0)), "under it");
        assert!(!inside(&t, Vec3::new(1.5, 0.5, -2.0)), "beside it");
        assert!(!inside(&t, Vec3::new(0.5, 0.5, 1.0)), "its lit top");
        assert!(!inside(&t, Vec3::new(0.5, 0.5, -3.5)), "below where it ends");
        let corner = Vec4::new(1.0, 1.2, 1.4, 0.0);
        let t = drawn(&mesh, corner, 0.0, clip);
        assert!(closed(&t), "closed from a corner");
        assert_eq!(sides(&mesh, corner, clip), 6, "6 silhouette edges from a corner");
        // (Its shadow on the ground, away from the light.)
        let ground = Vec3::new(0.5, 0.5, 0.5) - corner.truncate().normalize() * 2.0;
        assert!(inside(&t, ground), "{ground}");
    }

    /// An open sheet (a plank's one face, its front up) casts a light's
    /// shadow whichever way it faces, a volume closed both ways; its edges
    /// are open. The sun's it casts from its front alone, as the client
    /// (a single wall with its back to the sun casts nothing): lit from
    /// above, a closed volume under it; from below, nothing.
    #[test]
    fn an_open_sheet_casts_the_sun_s_shadow_from_its_front_only() {
        let positions = [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]];
        let mesh = volume_mesh(&positions, [[0, 1, 2], [0, 2, 3]], true);
        assert!(!mesh.closed && mesh.open_edges == 4 && mesh.torn_edges == 0);
        let front = volume_mesh(&positions, [[0, 1, 2], [0, 2, 3]], false);
        assert!(!front.closed && front.open_edges == 4);
        for light in [Vec4::new(0.1, 0.2, 1.0, 0.0), Vec4::new(0.1, 0.2, -1.0, 0.0)] {
            let clip = if light.z > 0.0 { -3.0 } else { 5.0 };
            let t = drawn(&mesh, light, 0.0, clip);
            assert!(closed(&t), "{light}");
            let shadow = Vec3::new(0.5, 0.5, 1.0) - light.truncate().normalize() * 0.5;
            assert!(inside(&t, shadow), "{light}: {shadow}");
            assert!(!inside(&t, Vec3::new(3.0, 3.0, 1.0)), "{light}");
            let t = drawn(&front, light, 0.0, clip);
            if light.z > 0.0 {
                assert!(closed(&t) && inside(&t, shadow), "{light}: its front lit");
                assert!(!inside(&t, Vec3::new(3.0, 3.0, 1.0)), "{light}");
            } else {
                assert!(t.is_empty(), "{light}: its back to the light: {t:?}");
            }
        }
    }

    /// The wiki's "finhead": three triangles on one edge (an edge of
    /// three faces, which tears the game's volumes) cast a closed volume,
    /// the shadow of the three and nothing past it.
    #[test]
    fn three_faces_on_one_edge_cast_no_spike() {
        let positions = [[0., 0., 1.], [0., 0., 2.], [1., 0., 1.5], [-1., 0.2, 1.5], [0., 1., 1.5]];
        let faces = [[0, 1, 2], [1, 0, 3], [0, 1, 4]];
        let both = volume_mesh(&positions, faces, true);
        let front = volume_mesh(&positions, faces, false);
        assert!(!both.closed && both.torn_edges == 1, "{both:?}");
        let lights = [Vec4::new(0.3, 0.4, 1.0, 0.0), Vec4::new(-0.5, 0.1, 0.8, 0.0)];
        for (mesh, light) in [&both, &front].into_iter().flat_map(|m| lights.map(|l| (m, l))) {
            let t = drawn(mesh, light, 0.0, -2.0);
            assert!(closed(&t), "{light}");
            // (Nothing far off on the ground.)
            for x in -10..=10 {
                for y in -10..=10 {
                    let p = Vec3::new(x as f32 * 3.0, y as f32 * 3.0, -1.0);
                    let dark = inside(&t, p);
                    // (The shadow lies within 4 m of the fins' foot.)
                    assert!(!dark || p.truncate().length() < 6.0, "{light}: a spike at {p}");
                }
            }
        }
    }

    /// A point light's volume reaches its radius, away from the light, and
    /// is closed.
    #[test]
    fn a_point_light_s_volume_reaches_its_radius() {
        let (positions, triangles) = cube();
        let mesh = volume_mesh(&positions, triangles, false);
        let light = Vec4::new(-3.0, 0.5, 3.0, 1.0);
        let t = drawn(&mesh, light, 10.0, 0.0);
        assert!(closed(&t));
        let from = light.truncate();
        let away = (Vec3::splat(0.5) - from).normalize();
        assert!(inside(&t, from + away * 9.0), "within the radius");
        assert!(!inside(&t, from + away * 11.0), "past it");
        assert!(!inside(&t, from + away * 1.0), "between the light and the cube");
    }

    /// What the renderer keeps of a model's meshes' volumes is let go of
    /// at a frame's beginning when the model is gone, or when none of its
    /// meshes has cast for [`STALE_AFTER`] (High turned to Fast); a model
    /// cast lately keeps its own, and a volume made apart for one let go
    /// of meanwhile is dropped as it comes.
    #[test]
    fn volumes_are_let_go_of_with_their_model_or_when_not_cast() {
        let mut v = VolumeMeshes::new(Default::default(), Default::default());
        let (gone, cast, idle) = (Arc::new(()), Arc::new(()), Arc::new(()));
        for (id, life) in [(1, &gone), (2, &cast), (3, &idle)] {
            v.models.insert(id, (Arc::downgrade(life), 0));
            let hint = AtomicU32::default();
            v.kept.insert(key_of(id, 0, false), Kept::Made(None), &hint);
            v.kept.insert(key_of(id, 1, true), Kept::Asked, &hint);
        }
        drop(gone);
        v.begin(10);
        assert!(!v.models.contains_key(&1) && !v.kept.keys().any(|k| model_of(k) == 1));
        assert_eq!(v.kept.len(), 4);
        v.models.get_mut(&2).unwrap().1 = 20;
        v.begin(STALE_AFTER + 5);
        assert_eq!(v.models.keys().copied().collect::<Vec<_>>(), [2], "the idle one let go of");
        assert!(v.kept.keys().all(|k| model_of(k) == 2) && v.kept.len() == 2);
        drop(idle);
        drop(cast);
        v.begin(STALE_AFTER + 6);
        assert!(v.models.is_empty() && v.kept.len() == 0);
        // (The places let go of are taken again; a mesh that remembers one
        // taken by another finds its own by its key.)
        let (hint, other) = (AtomicU32::default(), AtomicU32::default());
        v.kept.insert(key_of(7, 0, false), Kept::Asked, &hint);
        v.kept.insert(key_of(8, 0, false), Kept::Made(None), &other);
        assert_eq!(v.kept.places.len(), 6, "free places taken");
        other.store(hint.load(Ordering::Relaxed), Ordering::Relaxed);
        assert!(matches!(v.kept.get(key_of(8, 0, false), &other), Some(Kept::Made(None))));
        assert!(matches!(v.kept.get(key_of(7, 0, false), &hint), Some(Kept::Asked)));
        assert!(v.kept.get(key_of(9, 0, false), &hint).is_none());
    }

    /// Faces naming vertices the mesh hasn't, or of no area, are left out
    /// (nothing cast, no panic).
    #[test]
    fn bad_faces_are_left_out() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]];
        let mesh = volume_mesh(&positions, [[0, 1, 9], [0, 1, 2], [1, 1, 2]], false);
        assert!(mesh.vertices.is_empty() && mesh.indices.is_empty());
        let mesh = volume_mesh(&[], [], false);
        assert!(mesh.indices.is_empty() && mesh.closed);
    }
}
