//! An area's grass as the area view draws it: wherever its camera sees
//! grass ground, out to the end of the view, with fewer tufts the further
//! off it is.
//!
//! The game draws grass within 30 meters of its camera by default (the
//! wiki's Game Options: `Grass Far Render Distance`, 900, the distance
//! squared, up to 30,000, 173 m), fading it in at the edge; a builder
//! looking over a field from high up sees none of that edge.
//!
//! Each tile model's grass of each variant ([`crate::grass::tufts`]) is
//! uploaded once, and drawn at each of the levels of
//! [`crate::grass::level`] by the first of its tufts (they are in no
//! order of place): its levels share its buffers. Near
//! the eye each tile's grass is drawn by itself; where a whole chunk of
//! [`CHUNK`] by [`CHUNK`] tiles is at the last level, its tiles' tufts are
//! put into one mesh, made the first time the chunk is seen (on a thread
//! of its own, [`crate::AreaScene::grow_grass`]) and kept: a large area seen
//! from high up is some sixty draws of grass rather than a thousand. Until
//! a chunk's mesh is made, its tiles are drawn by themselves, with the
//! same tufts: nothing in the picture changes when it is.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, mpsc};

use glam::{Mat4, Vec3};
use mg_mdl::{Mesh, Model, Node, NodeKind};
use mg_render::{Gpu, GpuModel};

use crate::TILE_SIZE;
use crate::grass::{STEPS, TUFT_FACES, TUFT_VERTICES, kept, level, share};

/// The camera an area's grass is drawn for: where its eye is (the grass
/// is thinned with the distance from it) and what it sees (what is out of
/// its sight is left out).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sight {
    pub eye: Vec3,
    pub frustum: mg_render::Frustum,
}

impl Sight {
    /// What `camera` sees in a picture `aspect` times as wide as high.
    pub fn of(camera: &mg_render::Camera, aspect: f32) -> Sight {
        let frustum = mg_render::Frustum::new(camera.projection(aspect) * camera.view());
        Sight { eye: camera.eye, frustum }
    }

    /// How far a ball is from the eye: to the nearest of it, 0 around it.
    fn distance(&self, centre: Vec3, radius: f32) -> f32 {
        (centre.distance(self.eye) - radius).max(0.0)
    }
}

/// How many tiles each way the grass far off is put together in.
pub const CHUNK: i32 = 4;

/// The last of the levels ([`crate::grass::level`]): grass drawn at it is
/// put together by chunks.
const FAR: usize = STEPS - 1;

/// A tile whose grass may be drawn: its index, its position (which tells
/// its chunk), and a ball around its grass (in the area).
pub type Grown = (usize, Vec3, Vec3, f32);

/// What is drawn of an area's grass.
#[derive(Debug, Clone, PartialEq)]
pub enum GrassDraw {
    /// A tile's grass (by the tile's index), at a level of
    /// [`crate::grass::level`]'s.
    Tile(usize, usize),
    /// A chunk's (by its column and row), all of it at the last level:
    /// its tiles, by index.
    Chunk((i32, i32), Vec<usize>),
}

/// The chunk a tile at `position` is in.
pub fn chunk_of(position: Vec3) -> (i32, i32) {
    let side = TILE_SIZE * CHUNK as f32;
    ((position.x / side).floor() as i32, (position.y / side).floor() as i32)
}

/// Of the tiles' grass `grown`, what `sight` sees, at the level its
/// distance from the eye takes ([`level`]): wherever it is in sight, near
/// or far. A chunk all of whose grass is at the last level is drawn whole;
/// the grass of the others, tile by tile.
pub fn grass_chosen(grown: &[Grown], sight: &Sight) -> Vec<GrassDraw> {
    let mut chunks: BTreeMap<(i32, i32), Vec<&Grown>> = BTreeMap::new();
    for g in grown {
        chunks.entry(chunk_of(g.1)).or_default().push(g);
    }
    let mut out = Vec::new();
    for (key, tiles) in chunks {
        let (centre, radius) = around(tiles.iter().map(|g| (g.2, g.3)));
        if sight.frustum.hides(centre, radius) {
            continue;
        }
        if level(sight.distance(centre, radius)) == FAR {
            out.push(GrassDraw::Chunk(key, tiles.iter().map(|g| g.0).collect()));
            continue;
        }
        for &&(i, _, centre, radius) in &tiles {
            if !sight.frustum.hides(centre, radius) {
                out.push(GrassDraw::Tile(i, level(sight.distance(centre, radius))));
            }
        }
    }
    out
}

/// A ball around balls (around their box).
fn around(balls: impl Iterator<Item = (Vec3, f32)>) -> (Vec3, f32) {
    let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (c, r) in balls {
        (min, max) = (min.min(c - r), max.max(c + r));
    }
    ((min + max) / 2.0, (max - min).length() / 2.0)
}

/// A tile model's grass: the tufts, uploaded, at each of the [`STEPS`]
/// levels, and
/// a ball around them (in the model's space).
#[derive(Debug)]
pub(crate) struct TileGrass {
    pub(crate) levels: Vec<Arc<GpuModel>>,
    /// Each mesh's tufts to a square meter ([`crate::grass::densities`]).
    densities: Vec<f32>,
    pub(crate) centre: Vec3,
    pub(crate) radius: f32,
}

impl TileGrass {
    /// `grass` ([`crate::grass::tufts`]), its meshes `densities` dense.
    pub(crate) fn new(gpu: &Gpu, grass: Model, densities: Vec<f32>) -> TileGrass {
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for mesh in grass.nodes.iter().filter_map(Node::mesh) {
            for p in &mesh.vertices {
                (min, max) = (min.min(Vec3::from(*p)), max.max(Vec3::from(*p)));
            }
        }
        let all = Arc::new(GpuModel::new(gpu, Arc::new(grass)));
        // (A tuft's indices: three to each of its faces. Each of the
        // model's meshes is uploaded: a grown mesh has a tuft at least.)
        let per_tuft = (TUFT_FACES * 3) as u32;
        let levels = (0..STEPS)
            .map(|level| {
                let mut shares = densities.iter().map(|&d| share(level, d));
                if shares.all(|s| s >= 1.0) {
                    return all.clone();
                }
                let mut mesh = 0;
                let count = |m: &mg_render::model::GpuMesh| {
                    let density = densities.get(mesh).copied().unwrap_or(f32::INFINITY);
                    mesh += 1;
                    let tufts = (m.index_count / per_tuft) as usize;
                    kept(tufts, share(level, density)) as u32 * per_tuft
                };
                Arc::new(all.first_indices(count))
            })
            .collect();
        let (centre, radius) = ((min + max) / 2.0, (max - min).length() / 2.0);
        TileGrass { levels, densities, centre, radius }
    }

    /// The tufts, as they were grown.
    fn model(&self) -> &Model {
        &self.levels[0].model
    }
}

/// What a chunk's grass is made of: each tile's index, its grass and
/// where it stands.
pub(crate) type MadeOf = Vec<(usize, Arc<TileGrass>, Mat4)>;

/// A chunk's grass at the last level, in one mesh for each kind of grass
/// (in the area, less the chunk's corner), and what it was made of (kept,
/// so that what it is compared with is never another's in its place).
#[derive(Debug)]
pub(crate) struct ChunkGrass {
    made_of: MadeOf,
    pub(crate) model: Arc<GpuModel>,
    pub(crate) at: Mat4,
}

impl ChunkGrass {
    /// Whether it is made of what `now` has (each tile's index, its grass
    /// and where it stands).
    pub(crate) fn is_of<'a>(
        &self,
        now: impl ExactSizeIterator<Item = (usize, Option<&'a Arc<TileGrass>>, Mat4)>,
    ) -> bool {
        now.len() == self.made_of.len()
            && now.zip(&self.made_of).all(|((i, grass, at), (was, made, then))| {
                i == *was && grass.is_some_and(|g| Arc::ptr_eq(g, made)) && at == *then
            })
    }

    /// Puts together the grass of `tiles` in chunk `key`: the tufts the
    /// last level draws of each.
    pub(crate) fn new(gpu: &Gpu, key: (i32, i32), tiles: MadeOf) -> ChunkGrass {
        let side = TILE_SIZE * CHUNK as f32;
        let corner = Vec3::new(key.0 as f32 * side, key.1 as f32 * side, 0.0);
        // One mesh for each kind (its texture and colors), in the order
        // they are first met.
        let mut meshes: Vec<Mesh> = Vec::new();
        for (_, grass, transform) in &tiles {
            let to = Mat4::from_translation(-corner) * *transform;
            let meshes_of = grass.model().nodes.iter().filter_map(Node::mesh);
            for (mesh, &density) in meshes_of.zip(&grass.densities) {
                let same = |m: &&mut Mesh| {
                    m.textures[0] == mesh.textures[0]
                        && m.diffuse == mesh.diffuse
                        && m.ambient == mesh.ambient
                };
                let into = match meshes.iter_mut().position(|m| same(&m)) {
                    Some(i) => &mut meshes[i],
                    None => {
                        meshes.push(Mesh {
                            diffuse: mesh.diffuse,
                            ambient: mesh.ambient,
                            render: true,
                            textures: mesh.textures.clone(),
                            ..Mesh::default()
                        });
                        meshes.last_mut().expect("just pushed")
                    }
                };
                let tufts = mesh.vertices.len() / TUFT_VERTICES;
                let n = kept(tufts, share(FAR, density)) * TUFT_VERTICES;
                let first = into.vertices.len() as u32;
                into.vertices.extend(
                    mesh.vertices[..n]
                        .iter()
                        .map(|p| to.transform_point3(Vec3::from(*p)).to_array()),
                );
                into.normals.extend(&mesh.normals[..n]);
                into.uvs[0].extend(&mesh.uvs[0][..n]);
                let faces = n / TUFT_VERTICES * TUFT_FACES;
                into.faces.extend(
                    mesh.faces[..faces]
                        .iter()
                        .map(|f| mg_mdl::Face { vertices: f.vertices.map(|v| v + first), ..*f }),
                );
            }
        }
        let mut nodes = vec![Node::new("chunk_grass", NodeKind::Dummy)];
        for (k, mut mesh) in meshes.into_iter().enumerate() {
            mesh.source = (0..mesh.vertices.len() as u32).collect();
            mesh.source_uv = mesh.source.clone();
            let mut node = Node::new(&format!("chunk_grass{k}"), NodeKind::Mesh(Box::new(mesh)));
            node.parent = Some(0);
            nodes.push(node);
        }
        nodes[0].children = (1..nodes.len()).collect();
        let name = format!("chunk_grass_{}_{}", key.0, key.1);
        let model = Model { name, nodes, animation_scale: 1.0, ..Model::default() };
        ChunkGrass {
            made_of: tiles,
            model: Arc::new(GpuModel::new(gpu, Arc::new(model))),
            at: Mat4::from_translation(corner),
        }
    }
}

/// A chunk to put together: the GPU, the chunk, and what of.
type Job = (Gpu, (i32, i32), MadeOf);

/// Puts chunks together on a thread of its own, so that no frame waits on
/// one: a frame hands them over and takes those that are ready.
#[derive(Debug)]
pub(crate) struct ChunkWorker {
    jobs: mpsc::Sender<Job>,
    done: mpsc::Receiver<((i32, i32), Option<ChunkGrass>)>,
    /// The chunks handed over and not yet taken back.
    pending: HashSet<(i32, i32)>,
    /// Those that failed (drawn tile by tile from then on).
    failed: HashSet<(i32, i32)>,
}

impl ChunkWorker {
    /// Its thread started (`None` if it can't be).
    pub(crate) fn start() -> Option<ChunkWorker> {
        let (jobs, todo) = mpsc::channel::<Job>();
        let (made, done) = mpsc::channel();
        std::thread::Builder::new()
            .name("grass chunks".into())
            .spawn(move || {
                for (gpu, key, tiles) in todo {
                    let chunk =
                        mg_render::guard::guarded("grass", || ChunkGrass::new(&gpu, key, tiles));
                    if made.send((key, chunk)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        let (pending, failed) = (HashSet::new(), HashSet::new());
        Some(ChunkWorker { jobs, done, pending, failed })
    }

    /// Hands chunk `key` over, unless it is being made already (or failed).
    pub(crate) fn make(&mut self, gpu: &Gpu, key: (i32, i32), tiles: MadeOf) {
        if !self.failed.contains(&key) && self.pending.insert(key) {
            let _ = self.jobs.send((gpu.clone(), key, tiles));
        }
    }

    /// The chunks made since the last call.
    pub(crate) fn made(&mut self) -> Vec<((i32, i32), ChunkGrass)> {
        let mut out = Vec::new();
        while let Ok((key, chunk)) = self.done.try_recv() {
            self.pending.remove(&key);
            match chunk {
                Some(c) => out.push((key, c)),
                None => {
                    self.failed.insert(key);
                }
            }
        }
        out
    }

    /// Whether chunks are being made.
    pub(crate) fn busy(&self) -> bool {
        !self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 32 by 32 field's tiles, each with its grass in a ball around its
    /// middle (a tile's grass, a meter high).
    fn field() -> Vec<Grown> {
        let mut out = Vec::new();
        for row in 0..32 {
            for column in 0..32 {
                let middle = Vec3::new(column as f32 * 10.0 + 5.0, row as f32 * 10.0 + 5.0, 0.0);
                out.push((row * 32 + column, middle, middle + Vec3::Z * 0.5, 7.1));
            }
        }
        out
    }

    /// A sequence of numbers from 0 to 1 that looks random.
    struct Numbers(u64);

    impl Numbers {
        fn next(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (self.0 >> 40) as f32 / (1u64 << 24) as f32
        }
    }

    /// The level each tile is drawn at, by what is drawn (none: not drawn).
    fn levels(drawn: &[GrassDraw]) -> Vec<Option<usize>> {
        let mut out = vec![None; 32 * 32];
        for d in drawn {
            match d {
                GrassDraw::Tile(i, level) => out[*i] = Some(*level),
                GrassDraw::Chunk(_, tiles) => tiles.iter().for_each(|i| out[*i] = Some(FAR)),
            }
        }
        out
    }

    /// The grass drawn covers all the grass ground a camera sees, out to
    /// the end of its view: a point of the ground in the picture is on a
    /// tile whose grass is drawn, at the level its distance takes (all of
    /// it near; at the last level, put together by chunks). The grass
    /// once drawn within 30 meters of the view's focus only.
    #[test]
    fn the_grass_drawn_covers_the_ground_in_sight() {
        let field = field();
        let middle = Vec3::new(160.0, 160.0, 0.0);
        let mut n = Numbers(5);
        // From high up (all of the field in sight), from low down across
        // it, close to the ground, and from outside it looking in.
        let cameras = [
            mg_render::Camera::orbit(middle, 420.0, -1.0, 0.9),
            mg_render::Camera::orbit(middle, 50.0, -1.0, 0.3),
            mg_render::Camera::orbit(Vec3::new(20.0, 20.0, 0.0), 9.0, -1.0, 0.6),
            mg_render::Camera::orbit(Vec3::new(10.0, 160.0, 0.0), 120.0, 3.0, 0.4),
        ];
        for camera in cameras {
            let mut camera = camera;
            camera.far = camera.far.max(camera.eye.distance(camera.target) * 4.0 + 400.0);
            let aspect = 2.4;
            let sight = Sight::of(&camera, aspect);
            let drawn = grass_chosen(&field, &sight);
            let levels = levels(&drawn);
            let view_proj = camera.projection(aspect) * camera.view();
            let (mut seen, mut far) = (0, 0.0f32);
            for _ in 0..20_000 {
                let p = Vec3::new(n.next() * 320.0, n.next() * 320.0, 0.0);
                let clip = view_proj * p.extend(1.0);
                let in_sight = clip.w > 0.0
                    && clip.x.abs() <= clip.w
                    && clip.y.abs() <= clip.w
                    && (0.0..=clip.w).contains(&clip.z);
                if !in_sight {
                    continue;
                }
                seen += 1;
                let tile = (p.y / 10.0) as usize * 32 + (p.x / 10.0) as usize;
                let distance = p.distance(camera.eye);
                far = far.max(p.distance(camera.eye));
                let at = levels[tile].unwrap_or_else(|| panic!("no grass at {p} in sight"));
                assert!(at <= level(distance), "{p}: level {at}, {distance:.0} m off");
            }
            assert!(seen > 20, "{seen} points in sight");
            // Near the eye, all of it.
            let near = levels.iter().zip(&field).filter(|(_, g)| g.2.distance(camera.eye) < 40.0);
            assert!(near.into_iter().all(|(l, _)| l.is_none_or(|l| l == 0)));
            // (Points seen far past the old reach.)
            if camera.eye.distance(camera.target) > 40.0 {
                assert!(far > 150.0, "{far}");
            }
        }
    }

    /// A chunk is drawn whole only where all of its grass is at the last
    /// level: from high up, every chunk; close to the ground, none near.
    #[test]
    fn chunks_far_off_are_drawn_whole() {
        let field = field();
        let middle = Vec3::new(160.0, 160.0, 0.0);
        let high = mg_render::Camera::orbit(middle, 420.0, -1.0, 0.9);
        let drawn = grass_chosen(&field, &Sight::of(&high, 2.4));
        assert!(drawn.iter().all(|d| matches!(d, GrassDraw::Chunk(..))), "{drawn:?}");
        assert!(drawn.len() <= 64);
        let close = mg_render::Camera::orbit(middle, 25.0, -1.0, 0.7);
        let drawn = grass_chosen(&field, &Sight::of(&close, 2.4));
        for d in &drawn {
            if let GrassDraw::Chunk(_, tiles) = d {
                for &i in tiles {
                    let d = field[i].2.distance(close.eye) - field[i].3;
                    assert!(level(d) == FAR, "tile {i} {d:.0} m off, in a chunk");
                }
            }
        }
        assert!(drawn.iter().any(|d| matches!(d, GrassDraw::Tile(_, 0))));
    }

    /// No distance draws no grass: the levels run from all of it to fewer,
    /// never to none.
    #[test]
    fn grass_is_thinned_with_distance_never_to_none() {
        assert_eq!(level(0.0), 0);
        assert_eq!(level(30.0), 0, "near: as the game's");
        let mut last = 0;
        for d in 0..100_000 {
            let l = level(d as f32 / 10.0);
            assert!(l >= last && l < STEPS);
            assert!(crate::grass::share(l, 1000.0) > 0.0);
            last = l;
        }
        assert_eq!(last, FAR);
        let levels = crate::grass::LEVELS;
        assert!(levels.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 > w[1].1));
    }
}
