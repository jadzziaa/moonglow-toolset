//! The ground: tiles' walkmeshes, where objects stand.
//!
//! A tile's walkmesh is its `.wok` (what the engine uses), else the walkmesh
//! node (`aabb`) its model carries. Faces keep their surface material
//! (surfacemat.2da).

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3};
use mg_core::ResType;
use mg_mdl::walkmesh::{DoorState, WalkmeshKind};
use mg_mdl::{MeshExtra, Model, NodeKind};
use mg_rules::GameData;

use crate::AreaModel;
use crate::pick::Ray;

/// A walkmesh triangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkFace {
    pub corners: [Vec3; 3],
    /// surfacemat.2da row.
    pub material: u32,
}

/// A model's walkmesh, in the model's space.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Walkmesh {
    pub faces: Vec<WalkFace>,
    pub min: Vec3,
    pub max: Vec3,
}

impl Walkmesh {
    /// The walkmesh nodes (`aabb`) of a model, placed by its rest pose.
    pub fn from_model(model: &Model) -> Option<Walkmesh> {
        Walkmesh::from_nodes(model, |_, mesh| matches!(mesh.extra, MeshExtra::Aabb(_)))
    }

    /// A placeable's walkmesh (`<model>.pwk`) or a door's (`<model>.dwk`,
    /// the surfaces of its state), in the object's space: where it keeps
    /// creatures out.
    pub fn of_object(game: &GameData, model: &str, door: Option<DoorState>) -> Option<Walkmesh> {
        let (restype, kind) = match door {
            Some(_) => (ResType::DWK, WalkmeshKind::Door),
            None => (ResType::PWK, WalkmeshKind::Placeable),
        };
        let data = game.resman.get_named(model, restype).ok()?;
        let w = mg_mdl::walkmesh::Walkmesh::read(&data, kind).ok()?;
        let nodes: Vec<usize> = w.surfaces_for(door).map(|s| s.node).collect();
        Walkmesh::from_nodes(&w.model, |i, _| nodes.contains(&i))
    }

    /// The faces of a model's meshes that `keep` keeps, placed by its rest
    /// pose.
    fn from_nodes(model: &Model, keep: impl Fn(usize, &mg_mdl::Mesh) -> bool) -> Option<Walkmesh> {
        let pose = mg_render::rest_pose(model);
        let mut faces = Vec::new();
        for (i, n) in model.nodes.iter().enumerate() {
            let NodeKind::Mesh(mesh) = &n.kind else { continue };
            if !keep(i, mesh) {
                continue;
            }
            let to = pose[i];
            for f in &mesh.faces {
                let corner = |k: usize| -> Option<Vec3> {
                    let v = mesh.vertices.get(f.vertices[k] as usize)?;
                    Some(to.transform_point3(Vec3::from_array(*v)))
                };
                let (Some(a), Some(b), Some(c)) = (corner(0), corner(1), corner(2)) else {
                    continue;
                };
                faces.push(WalkFace { corners: [a, b, c], material: f.material });
            }
        }
        if faces.is_empty() {
            return None;
        }
        let (min, max) = mg_render::box_of(faces.iter().flat_map(|f| f.corners))?;
        Some(Walkmesh { faces, min, max })
    }

    /// A tile's walkmesh: `<model>.wok` (its nodes under the tile's root),
    /// else its model's own.
    pub fn of_tile(game: &GameData, model: &str) -> Option<Walkmesh> {
        let wok = || -> Option<Walkmesh> {
            let data = game.resman.get_named(model, ResType::WOK).ok()?;
            let w = mg_mdl::walkmesh::Walkmesh::read(&data, mg_mdl::walkmesh::WalkmeshKind::Tile);
            Walkmesh::from_model(&w.ok()?.model)
        };
        let own = || -> Option<Walkmesh> {
            let data = game.resman.get_named(model, ResType::MDL).ok()?;
            Walkmesh::from_model(&Model::read(&data).ok()?)
        };
        wok().or_else(own)
    }

    /// The heights of the faces above or below `p` (the model's space).
    pub fn heights(&self, p: Vec2) -> impl Iterator<Item = f32> + '_ {
        self.surfaces(p).map(|(z, _)| z)
    }

    /// The faces above or below `p` (the model's space): each one's height
    /// there and its surface material.
    pub fn surfaces(&self, p: Vec2) -> impl Iterator<Item = (f32, u32)> + '_ {
        // (A point on the tile's edge, as a start location at a tile corner
        // is, may land a rounding error outside.)
        let e = 1e-3;
        let outside = p.x < self.min.x - e
            || p.y < self.min.y - e
            || p.x > self.max.x + e
            || p.y > self.max.y + e;
        self.faces
            .iter()
            .filter(move |_| !outside)
            .filter_map(move |f| height_on(f, p).map(|z| (z, f.material)))
    }

    /// The ground's level (the model's space): the median, over a 5 × 5
    /// grid of points across it, of the lowest face at each (rocks, trees
    /// and walls stand above the ground; the median ignores the odd one
    /// that covers a point). Some tilesets build their ground above the
    /// tile's own height.
    pub fn level(&self) -> Option<f32> {
        let mut lows: Vec<f32> = (0..25)
            .filter_map(|i| {
                let t = Vec2::new((i % 5) as f32 + 0.5, (i / 5) as f32 + 0.5) / 5.0;
                let p = self.min.truncate() + (self.max - self.min).truncate() * t;
                self.heights(p).min_by(f32::total_cmp)
            })
            .collect();
        lows.sort_by(f32::total_cmp);
        lows.get(lows.len() / 2).copied()
    }

    /// The distance along `ray` (in the model's space) to the nearest face.
    pub fn hit(&self, ray: &Ray) -> Option<f32> {
        ray.hits_box(self.min, self.max, Mat4::IDENTITY)?;
        self.faces.iter().filter_map(|f| hit_triangle(ray, f.corners)).min_by(f32::total_cmp)
    }
}

/// The height of face `f` above `p`, if `p` is inside it (seen from above).
/// How nearly upright a face's normal must be to count as flat, a surface
/// to set things on: its height part at least this (about 45° of slope).
pub const FLAT: f32 = 0.7;

/// Whether a triangle (its corners) is flat within [`FLAT`], facing up or
/// down (a walkmesh's faces are wound either way).
fn is_flat([a, b, c]: [Vec3; 3]) -> bool {
    let n = (b - a).cross(c - a).normalize_or_zero();
    n.z.abs() >= FLAT
}

fn height_on(f: &WalkFace, p: Vec2) -> Option<f32> {
    let [a, b, c] = f.corners;
    let (a2, b2, c2) = (a.truncate(), b.truncate(), c.truncate());
    let d = (b2 - a2).perp_dot(c2 - a2);
    if d.abs() < 1e-9 {
        return None;
    }
    let u = (p - a2).perp_dot(c2 - a2) / d;
    let v = (b2 - a2).perp_dot(p - a2) / d;
    let e = -1e-5;
    (u >= e && v >= e && u + v <= 1.0 - e).then_some(a.z + u * (b.z - a.z) + v * (c.z - a.z))
}

/// The distance along `ray` to a triangle (the previews' Möller–Trumbore).
fn hit_triangle(ray: &Ray, [a, b, c]: [Vec3; 3]) -> Option<f32> {
    mg_preview::compose::ray_triangle(ray.origin, ray.dir, a, b, c)
}

/// Per-tile cached world inverse and world-space AABB for [`Ground`].
#[derive(Debug, Clone)]
struct GroundTile {
    walk: Arc<Walkmesh>,
    to: Mat4,
    inv: Mat4,
    min: Vec3,
    max: Vec3,
}

/// An area's ground: its tiles' walkmeshes, each read once per model.
#[derive(Debug, Default)]
pub struct Ground {
    /// By tile index, with precomputed inverse transform and world AABB.
    tiles: Vec<Option<GroundTile>>,
    cache: HashMap<String, Option<Arc<Walkmesh>>>,
    /// By tile index, its ground level ([`Ground::tile_level`]).
    levels: Vec<Option<f32>>,
    /// Each model's level, measured once.
    level_cache: HashMap<String, Option<f32>>,
    width: u32,
    area_bounds: Option<(Vec3, Vec3)>,
    /// Tiles whose world AABB extends beyond their grid cell (or when `width == 0`).
    overhangs: Vec<usize>,
}

impl Ground {
    pub fn new(game: &GameData, area: &AreaModel) -> Ground {
        let mut g = Ground::default();
        g.update(game, area);
        g
    }

    /// Builds a [`GroundTile`] and folds its bounds/overhang status into the area summary.
    fn place_tile(
        i: usize,
        width: u32,
        walk: Arc<Walkmesh>,
        to: Mat4,
        area_bounds: &mut Option<(Vec3, Vec3)>,
        overhangs: &mut Vec<usize>,
    ) -> GroundTile {
        let inv = to.inverse();
        let (min, max) = world_aabb(walk.min, walk.max, to);
        *area_bounds = Some(match *area_bounds {
            Some((amin, amax)) => (amin.min(min), amax.max(max)),
            None => (min, max),
        });
        let in_cell = width > 0 && {
            let col = (i as u32 % width) as f32 * crate::TILE_SIZE;
            let row = (i as u32 / width) as f32 * crate::TILE_SIZE;
            let eps = 1e-3;
            min.x >= col - eps
                && max.x <= col + crate::TILE_SIZE + eps
                && min.y >= row - eps
                && max.y <= row + crate::TILE_SIZE + eps
        };
        if !in_cell {
            overhangs.push(i);
        }
        GroundTile { walk, to, inv, min, max }
    }

    /// Follows `area` after an edit (reading walkmeshes not seen before).
    pub fn update(&mut self, game: &GameData, area: &AreaModel) {
        let cache = &mut self.cache;
        let mut area_bounds: Option<(Vec3, Vec3)> = None;
        let mut overhangs = Vec::new();
        self.width = area.width;
        self.tiles = area
            .tiles
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let name = t.model.as_ref()?;
                let walk = cache
                    .entry(name.clone())
                    .or_insert_with(|| Walkmesh::of_tile(game, name).map(Arc::new))
                    .clone()?;
                Some(Self::place_tile(
                    i,
                    area.width,
                    walk,
                    t.transform(),
                    &mut area_bounds,
                    &mut overhangs,
                ))
            })
            .collect();
        self.area_bounds = area_bounds;
        self.overhangs = overhangs;
        let levels = &mut self.level_cache;
        self.levels = area
            .tiles
            .iter()
            .zip(&self.tiles)
            .map(|(t, w)| {
                let tile = w.as_ref()?;
                let name = t.model.as_ref()?;
                let level = *levels.entry(name.clone()).or_insert_with(|| tile.walk.level());
                Some(level? + tile.to.transform_point3(Vec3::ZERO).z)
            })
            .collect();
        let active: std::collections::HashSet<&str> =
            area.tiles.iter().filter_map(|t| t.model.as_deref()).collect();
        self.cache.retain(|k, _| active.contains(k.as_str()));
        self.level_cache.retain(|k, _| active.contains(k.as_str()));
    }

    /// The tile index at a point, if it is inside the area.
    fn tile_at(&self, p: Vec2) -> Option<usize> {
        crate::tile_at(self.width, self.tiles.len(), p.x, p.y)
    }

    /// Tile `index`'s ground level ([`Walkmesh::level`], in the area's
    /// space), where it has a walkmesh.
    pub fn tile_level(&self, index: usize) -> Option<f32> {
        self.levels.get(index).copied().flatten()
    }

    /// The ground level of the tile `p` is over ([`Ground::tile_level`]),
    /// where `p` is in the area and its tile has a walkmesh.
    pub fn level_at(&self, p: Vec2) -> Option<f32> {
        self.tile_level(self.tile_at(p)?)
    }

    /// The ground's height at `p` nearest to `near` (bridges and floors
    /// above others have several).
    pub fn height(&self, p: Vec2, near: f32) -> Option<f32> {
        let tile = self.tiles.get(self.tile_at(p)?)?.as_ref()?;
        let local = tile.inv.transform_point3(p.extend(0.0));
        let base = tile.to.transform_point3(Vec3::ZERO).z;
        tile.walk
            .heights(local.truncate())
            .map(|z| z + base)
            .min_by(|a, b| (a - near).abs().total_cmp(&(b - near).abs()))
    }

    /// The ground's flat levels at `p`, lowest first: the heights of its
    /// walkmesh faces there that face up within [`FLAT`] (a bridge over
    /// the ground, a floor over another: several; a cliff's face, none).
    pub fn flat_heights(&self, p: Vec2) -> Vec<f32> {
        let Some(tile) = self.tile_at(p).and_then(|i| self.tiles.get(i)?.as_ref()) else {
            return Vec::new();
        };
        let local = tile.inv.transform_point3(p.extend(0.0)).truncate();
        let base = tile.to.transform_point3(Vec3::ZERO).z;
        let mut out: Vec<f32> = (tile.walk.faces.iter())
            .filter(|f| is_flat(f.corners))
            .filter_map(|f| height_on(f, local))
            .map(|z| z + base)
            .collect();
        out.sort_by(f32::total_cmp);
        out
    }

    /// The ground's height at `p` nearest to `near`, as
    /// [`height`](Self::height), on the faces whose surface material
    /// (surfacemat.2da row) `keep` keeps.
    pub fn height_on(&self, p: Vec2, near: f32, keep: impl Fn(u32) -> bool) -> Option<f32> {
        let tile = self.tiles.get(self.tile_at(p)?)?.as_ref()?;
        let local = tile.inv.transform_point3(p.extend(0.0));
        let base = tile.to.transform_point3(Vec3::ZERO).z;
        tile.walk
            .surfaces(local.truncate())
            .filter(|(_, m)| keep(*m))
            .map(|(z, _)| z + base)
            .min_by(|a, b| (a - near).abs().total_cmp(&(b - near).abs()))
    }

    /// Every walkmesh face of the ground, in the area's space, with its
    /// surface material (surfacemat.2da row).
    pub fn faces(&self) -> impl Iterator<Item = ([Vec3; 3], u32)> + '_ {
        self.tiles().flat_map(|(w, to)| {
            w.faces.iter().map(move |f| (f.corners.map(|c| to.transform_point3(c)), f.material))
        })
    }

    /// Each tile's walkmesh and where it is placed (its faces' corners
    /// in the area's space by `transform_point3`), in the order of
    /// [`Ground::faces`].
    pub fn tiles(&self) -> impl Iterator<Item = (&Walkmesh, Mat4)> + '_ {
        self.tiles.iter().flatten().map(|tile| (&*tile.walk, tile.to))
    }

    /// Where `ray` first meets the ground.
    pub fn hit(&self, ray: &Ray) -> Option<Vec3> {
        let (area_min, area_max) = self.area_bounds?;
        let (t_enter, t_exit) = ray_box_range(ray.origin, ray.dir, area_min, area_max)?;
        let mut best: Option<f32> = None;
        let try_tile = |tile: &GroundTile, best: &mut Option<f32>| {
            if let Some(t_box) =
                mg_preview::compose::ray_box(ray.origin, ray.dir, tile.min, tile.max)
                && best.is_none_or(|b| t_box <= b)
            {
                let local = Ray {
                    origin: tile.inv.transform_point3(ray.origin),
                    dir: tile.inv.transform_vector3(ray.dir),
                };
                if let Some(t) = tile.walk.hit(&local)
                    && best.is_none_or(|b| t < b)
                {
                    *best = Some(t);
                }
            }
        };
        for &idx in &self.overhangs {
            if let Some(tile) = self.tiles.get(idx).and_then(Option::as_ref) {
                try_tile(tile, &mut best);
            }
        }
        if self.width == 0 || self.tiles.is_empty() {
            return best.map(|t| ray.at(t));
        }
        let w = self.width as i32;
        let h = self.tiles.len().div_ceil(self.width as usize) as i32;
        let side = crate::TILE_SIZE;
        let eps = 1e-3;
        let t_end = best.map_or(t_exit, |b| t_exit.min(b));
        if t_end < t_enter {
            return best.map(|t| ray.at(t));
        }
        let p0 = ray.at(t_enter);
        let p1 = ray.at(t_end);
        let (min_x, max_x) = (p0.x.min(p1.x), p0.x.max(p1.x));
        let (min_y, max_y) = (p0.y.min(p1.y), p0.y.max(p1.y));
        if max_x + eps < 0.0
            || min_x - eps > w as f32 * side
            || max_y + eps < 0.0
            || min_y - eps > h as f32 * side
        {
            return best.map(|t| ray.at(t));
        }
        let c0 = ((min_x - eps) / side).floor().clamp(0.0, (w - 1) as f32) as i32;
        let c1 = ((max_x + eps) / side).floor().clamp(0.0, (w - 1) as f32) as i32;
        let r0 = ((min_y - eps) / side).floor().clamp(0.0, (h - 1) as f32) as i32;
        let r1 = ((max_y + eps) / side).floor().clamp(0.0, (h - 1) as f32) as i32;

        let visit = |col: i32, row: i32, best: &mut Option<f32>| {
            if col >= 0 && col < w && row >= 0 && row < h {
                let idx = (row as usize) * self.width as usize + (col as usize);
                if let Some(Some(tile)) = self.tiles.get(idx) {
                    try_tile(tile, best);
                }
            }
        };

        if c1 - c0 <= 1 || r1 - r0 <= 1 {
            let dr = if ray.dir.y >= 0.0 { 1 } else { -1 };
            let dc = if ray.dir.x >= 0.0 { 1 } else { -1 };
            let mut r = if dr > 0 { r0 } else { r1 };
            loop {
                let mut c = if dc > 0 { c0 } else { c1 };
                loop {
                    visit(c, r, &mut best);
                    if (dc > 0 && c >= c1) || (dc < 0 && c <= c0) {
                        break;
                    }
                    c += dc;
                }
                if (dr > 0 && r >= r1) || (dr < 0 && r <= r0) {
                    break;
                }
                r += dr;
            }
            return best.map(|t| ray.at(t));
        }

        // 2D Amanatides-Woo DDA across grid cells intersecting `[t_enter, t_end]`,
        // visiting immediate `1e-3`-margin neighbors around each cell so grid-tolerant
        // tiles near cell boundaries cannot be skipped.
        let grid_min = Vec3::new(-eps, -eps, area_min.z);
        let grid_max = Vec3::new(w as f32 * side + eps, h as f32 * side + eps, area_max.z);
        let Some((g_enter, g_exit)) = ray_box_range(ray.origin, ray.dir, grid_min, grid_max) else {
            return best.map(|t| ray.at(t));
        };
        let mut t = t_enter.max(g_enter);
        let mut t_limit = best.map_or(g_exit.min(t_end), |b| g_exit.min(t_end).min(b));
        if t > t_limit {
            return best.map(|t| ray.at(t));
        }
        let start = ray.at(t);
        let step_x: i32 = if ray.dir.x >= 0.0 { 1 } else { -1 };
        let step_y: i32 = if ray.dir.y >= 0.0 { 1 } else { -1 };
        let mut col = ((start.x / side).floor() as i32).clamp(0, w - 1);
        let mut row = ((start.y / side).floor() as i32).clamp(0, h - 1);
        let next_bx = ((col + if step_x > 0 { 1 } else { 0 }) as f32) * side;
        let next_by = ((row + if step_y > 0 { 1 } else { 0 }) as f32) * side;
        let mut t_max_x = (next_bx - ray.origin.x) / ray.dir.x;
        let mut t_max_y = (next_by - ray.origin.y) / ray.dir.y;
        if t_max_x < t {
            t_max_x = t;
        }
        if t_max_y < t {
            t_max_y = t;
        }
        let t_delta_x = side / ray.dir.x.abs();
        let t_delta_y = side / ray.dir.y.abs();
        let margin_x = eps / ray.dir.x.abs();
        let margin_y = eps / ray.dir.y.abs();
        let stop_exit = g_exit.min(t_end);

        loop {
            visit(col, row, &mut best);
            let p = ray.at(t);
            let near_lo_x = (p.x - (col as f32) * side).abs() <= eps;
            let near_hi_x = (((col + 1) as f32) * side - p.x).abs() <= eps;
            let near_lo_y = (p.y - (row as f32) * side).abs() <= eps;
            let near_hi_y = (((row + 1) as f32) * side - p.y).abs() <= eps;
            if near_lo_x {
                visit(col - 1, row, &mut best);
                if near_lo_y {
                    visit(col - 1, row - 1, &mut best);
                }
                if near_hi_y {
                    visit(col - 1, row + 1, &mut best);
                }
            }
            if near_hi_x {
                visit(col + 1, row, &mut best);
                if near_lo_y {
                    visit(col + 1, row - 1, &mut best);
                }
                if near_hi_y {
                    visit(col + 1, row + 1, &mut best);
                }
            }
            if near_lo_y {
                visit(col, row - 1, &mut best);
            }
            if near_hi_y {
                visit(col, row + 1, &mut best);
            }
            if let Some(b) = best {
                t_limit = t_limit.min(b);
            }
            if t_max_x < t_max_y {
                t = t_max_x;
                if t > stop_exit
                    || (best.is_some()
                        && t_max_x > t_limit + margin_x
                        && t_max_y > t_limit + margin_y)
                {
                    break;
                }
                col += step_x;
                t_max_x += t_delta_x;
            } else {
                t = t_max_y;
                if t > stop_exit
                    || (best.is_some()
                        && t_max_x > t_limit + margin_x
                        && t_max_y > t_limit + margin_y)
                {
                    break;
                }
                row += step_y;
                t_max_y += t_delta_y;
            }
            if col < -1 || col > w || row < -1 || row > h {
                break;
            }
        }
        best.map(|t| ray.at(t))
    }
}

fn world_aabb(min: Vec3, max: Vec3, to: Mat4) -> (Vec3, Vec3) {
    let mut wmin = Vec3::splat(f32::INFINITY);
    let mut wmax = Vec3::splat(f32::NEG_INFINITY);
    for sx in [min.x, max.x] {
        for sy in [min.y, max.y] {
            for sz in [min.z, max.z] {
                let p = to.transform_point3(Vec3::new(sx, sy, sz));
                wmin = wmin.min(p);
                wmax = wmax.max(p);
            }
        }
    }
    (wmin, wmax)
}

fn ray_box_range(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<(f32, f32)> {
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
    (near <= far && far >= 0.0).then_some((near.max(0.0), far))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A baked walkmesh (`mg_module::bake`, by `VisualTransform::matrix`),
    /// placed as Object Walkmeshes place every walkmesh (the object's
    /// position and Bearing), is where the original's faces are when they
    /// are drawn with the model ([`AreaObject::model_transform`]): it
    /// blocks as the model is drawn, tilted, turned, scaled and moved.
    /// (The bake is the module workspace's: with the feature `edit`.)
    #[cfg(feature = "edit")]
    #[test]
    fn a_baked_walkmesh_lies_where_the_model_is_drawn() {
        let pwk = "node trimesh box_wg\n  parent box_pwk\n  position 0.5 1 0\n  verts 3\n    \
                   0 0 0\n    2 0 0.5\n    0 1 0\n  faces 1\n    0 1 2  1  0 0 0  7\nendnode\n";
        let w = mg_mdl::walkmesh::Walkmesh::read(
            pwk.as_bytes(),
            mg_mdl::walkmesh::WalkmeshKind::Placeable,
        )
        .unwrap();
        let mut o =
            crate::AreaObject::placed(crate::ObjectKind::Placeable, 0, &mg_gff::Struct::new(9));
        o.position = Vec3::new(20.0, 30.0, 1.0);
        o.rotation = 0.7;
        let v = crate::VisualTransform {
            scale: Vec3::splat(0.4),
            rotate: Vec3::new(35.0, 20.0, -10.0),
            translate: Vec3::new(1.0, -0.5, 0.25),
        };
        o.visual = Some(v);
        let faces = w.object_faces(None);
        let baked = mg_module::bake::moved(&faces, v.matrix().to_cols_array_2d());
        for (b, (f, _)) in baked.iter().zip(&faces) {
            for (got, of) in b.iter().zip(f) {
                let got = o.transform().transform_point3(Vec3::from(*got));
                let drawn = o.model_transform().transform_point3(Vec3::from(*of));
                assert!((got - drawn).length() < 1e-4, "{got} {drawn}");
            }
        }
        // And the walkmesh file written so reads back as those faces.
        let text = w.moved_text("mgbk_0001", v.matrix().to_cols_array_2d(), "");
        let back = mg_mdl::walkmesh::Walkmesh::read(
            text.as_bytes(),
            mg_mdl::walkmesh::WalkmeshKind::Placeable,
        )
        .unwrap();
        let read =
            mg_module::bake::moved(&back.object_faces(None), Mat4::IDENTITY.to_cols_array_2d());
        assert!(mg_module::bake::same_bake(&read, &baked));
    }

    /// A 10 m square tile: a ramp from 0 (west) to 5 (east).
    fn ramp() -> Walkmesh {
        let (a, b, c, d) = (
            Vec3::new(-5.0, -5.0, 0.0),
            Vec3::new(5.0, -5.0, 5.0),
            Vec3::new(5.0, 5.0, 5.0),
            Vec3::new(-5.0, 5.0, 0.0),
        );
        Walkmesh {
            faces: vec![
                WalkFace { corners: [a, b, c], material: 3 },
                WalkFace { corners: [a, c, d], material: 3 },
            ],
            min: Vec3::new(-5.0, -5.0, 0.0),
            max: Vec3::new(5.0, 5.0, 5.0),
        }
    }

    #[test]
    fn a_tile_s_ground_level_is_its_ground_not_what_stands_on_it() {
        // Ground 2 m up (as some tilesets build it), with a 3 m block on
        // one corner.
        let flat = |z: f32, x0: f32, y0: f32, x1: f32, y1: f32| {
            let (a, b, c, d) = (
                Vec3::new(x0, y0, z),
                Vec3::new(x1, y0, z),
                Vec3::new(x1, y1, z),
                Vec3::new(x0, y1, z),
            );
            [
                WalkFace { corners: [a, b, c], material: 3 },
                WalkFace { corners: [a, c, d], material: 2 },
            ]
        };
        let mut faces = flat(2.0, -5.0, -5.0, 5.0, 5.0).to_vec();
        faces.extend(flat(5.0, -5.0, -5.0, -2.0, -2.0));
        let w = Walkmesh { faces, min: Vec3::new(-5.0, -5.0, 2.0), max: Vec3::new(5.0, 5.0, 5.0) };
        assert_eq!(w.level(), Some(2.0));
        assert_eq!(ramp().level(), Some(2.5), "a ramp's middle");
    }

    #[test]
    fn heights_and_hits_follow_the_faces() {
        let w = ramp();
        let h: Vec<f32> = w.heights(Vec2::new(0.0, 2.0)).collect();
        assert_eq!(h, [2.5]);
        assert_eq!(w.heights(Vec2::new(6.0, 0.0)).count(), 0);
        let down = Ray { origin: Vec3::new(2.5, 0.0, 10.0), dir: Vec3::NEG_Z };
        assert!((w.hit(&down).unwrap() - 6.25).abs() < 1e-4);
    }

    /// The flat levels at a point: a floor and a bridge over it (wound
    /// either way), lowest first; not the ramp beside them (steeper than
    /// [`FLAT`]) nor faces that don't reach the point.
    #[test]
    fn the_flat_levels_at_a_point_are_every_floor_there() {
        let square = |z: f32, flip: bool| {
            let (a, b, c, d) = (
                Vec3::new(-5.0, -5.0, z),
                Vec3::new(5.0, -5.0, z),
                Vec3::new(5.0, 5.0, z),
                Vec3::new(-5.0, 5.0, z),
            );
            let tri = |p: [Vec3; 3]| WalkFace {
                corners: if flip { [p[0], p[2], p[1]] } else { p },
                material: 3,
            };
            vec![tri([a, b, c]), tri([a, c, d])]
        };
        // A steep wall across the middle, 10 m high over 1 m.
        let wall = WalkFace {
            corners: [
                Vec3::new(-5.0, -0.5, 0.0),
                Vec3::new(5.0, -0.5, 0.0),
                Vec3::new(5.0, 0.5, 10.0),
            ],
            material: 3,
        };
        let mut faces = square(0.0, false);
        faces.extend(square(3.0, true));
        faces.push(wall);
        let w = Walkmesh { faces, min: Vec3::splat(-5.0), max: Vec3::new(5.0, 5.0, 10.0) };
        let g = ground_from(
            1,
            vec![Some((Arc::new(w), Mat4::from_translation(Vec3::new(5.0, 5.0, 1.0))))],
        );
        assert_eq!(g.flat_heights(Vec2::new(5.5, 5.2)), [1.0, 4.0]);
        assert!(g.flat_heights(Vec2::new(25.0, 5.0)).is_empty(), "outside the area");
        // (The ramp, 15 m up over 10: steeper than flat.)
        let mut r = ramp();
        r.faces.iter_mut().for_each(|f| f.corners.iter_mut().for_each(|c| c.z *= 3.0));
        let g = ground_from(
            1,
            vec![Some((Arc::new(r), Mat4::from_translation(Vec3::new(5.0, 5.0, 0.0))))],
        );
        assert!(g.flat_heights(Vec2::new(5.0, 5.0)).is_empty(), "a slope of 56°");
    }

    fn ground_from(width: u32, raw: Vec<Option<(Arc<Walkmesh>, Mat4)>>) -> Ground {
        let mut area_bounds: Option<(Vec3, Vec3)> = None;
        let mut overhangs = Vec::new();
        let tiles = raw
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let (walk, to) = entry?;
                Some(Ground::place_tile(i, width, walk, to, &mut area_bounds, &mut overhangs))
            })
            .collect();
        Ground {
            tiles,
            cache: HashMap::new(),
            levels: Vec::new(),
            level_cache: HashMap::new(),
            width,
            area_bounds,
            overhangs,
        }
    }

    #[test]
    fn the_ground_is_placed_and_turned_with_its_tiles() {
        let w = Arc::new(ramp());
        // Two tiles: the second turned a half turn (its ramp rises west)
        // and a height step of 2 up.
        let at = |x: f32, turns: f32, z: f32| {
            Mat4::from_rotation_translation(
                glam::Quat::from_rotation_z(turns * std::f32::consts::FRAC_PI_2),
                Vec3::new(x, 5.0, z),
            )
        };
        let g = ground_from(
            2,
            vec![Some((w.clone(), at(5.0, 0.0, 0.0))), Some((w, at(15.0, 2.0, 2.0)))],
        );
        assert!((g.height(Vec2::new(7.5, 5.0), 0.0).unwrap() - 3.75).abs() < 1e-4);
        assert!((g.height(Vec2::new(17.5, 5.0), 0.0).unwrap() - 3.25).abs() < 1e-4);
        assert!(g.height(Vec2::new(25.0, 5.0), 0.0).is_none(), "outside the area");
        let down = Ray { origin: Vec3::new(12.5, 5.0, 50.0), dir: Vec3::NEG_Z };
        let hit = g.hit(&down).unwrap();
        assert!((hit - Vec3::new(12.5, 5.0, 5.75)).length() < 1e-4, "{hit}");
    }

    #[test]
    fn ground_update_evicts_removed_tile_caches() {
        let empty =
            GameData::new(mg_resman::ResMan::new(), mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut g = Ground::default();
        g.cache.insert("keep".into(), None);
        g.cache.insert("old".into(), None);
        g.level_cache.insert("keep".into(), Some(1.0));
        g.level_cache.insert("old".into(), Some(2.0));
        let area = AreaModel {
            tileset: None,
            width: 1,
            height: 1,
            height_step: 5.0,
            tiles: vec![crate::AreaTile {
                index: 0,
                column: 0,
                row: 0,
                id: 0,
                model: Some("keep".into()),
                height: 0,
                orientation: 0,
                main_lights: [0; 2],
                source_lights: [0; 2],
                anim_loops: [false; 3],
                replace_texture: None,
                position: glam::Vec3::ZERO,
            }],
            objects: Vec::new(),
            hooks: Vec::new(),
            lighting: crate::Lighting {
                sun: crate::Sky::default(),
                moon: crate::Sky::default(),
                is_night: false,
                day_night_cycle: false,
                fog_clip: 45.0,
                sky_box: 0,
                env_map: None,
                shadow_opacity: 0,
                flags: 0,
                wind_power: 0,
                sun_direction: None,
                moon_direction: None,
                wallpaper: None,
            },
            grass: Vec::new(),
            border: Vec::new(),
            problems: Vec::new(),
        };
        g.update(&empty, &area);
        assert!(g.cache.contains_key("keep") && !g.cache.contains_key("old"));
        assert!(g.level_cache.contains_key("keep") && !g.level_cache.contains_key("old"));
    }

    #[test]
    fn ground_hit_dda_matches_brute_force() {
        let face = |corners: [Vec3; 3]| WalkFace { corners, material: 1 };
        let grid_tile = |z: f32| {
            let faces = vec![
                face([Vec3::new(0.0, 0.0, z), Vec3::new(10.0, 0.0, z), Vec3::new(10.0, 10.0, z)]),
                face([Vec3::new(0.0, 0.0, z), Vec3::new(10.0, 10.0, z), Vec3::new(0.0, 10.0, z)]),
            ];
            let (min, max) = mg_render::box_of(faces.iter().flat_map(|f| f.corners)).unwrap();
            Arc::new(Walkmesh { faces, min, max })
        };
        let overhang_tile = || {
            // Protrudes 3 m beyond its 10x10 tile cell.
            let faces = vec![face([
                Vec3::new(-3.0, 2.0, 2.0),
                Vec3::new(13.0, 2.0, 2.0),
                Vec3::new(5.0, 13.0, 2.0),
            ])];
            let (min, max) = mg_render::box_of(faces.iter().flat_map(|f| f.corners)).unwrap();
            Arc::new(Walkmesh { faces, min, max })
        };

        let mut g = Ground { width: 4, ..Default::default() };
        for row in 0..4 {
            for col in 0..4 {
                let to = Mat4::from_translation(Vec3::new(
                    col as f32 * crate::TILE_SIZE,
                    row as f32 * crate::TILE_SIZE,
                    0.0,
                ));
                let walk = if col == 2 && row == 1 {
                    overhang_tile()
                } else {
                    grid_tile((col + row) as f32 * 0.25)
                };
                let (min, max) = world_aabb(walk.min, walk.max, to);
                let in_cell = min.x >= col as f32 * crate::TILE_SIZE - 1e-3
                    && max.x <= (col + 1) as f32 * crate::TILE_SIZE + 1e-3
                    && min.y >= row as f32 * crate::TILE_SIZE - 1e-3
                    && max.y <= (row + 1) as f32 * crate::TILE_SIZE + 1e-3;
                if !in_cell {
                    g.overhangs.push(row * 4 + col);
                }
                g.area_bounds = Some(match g.area_bounds {
                    Some((amin, amax)) => (amin.min(min), amax.max(max)),
                    None => (min, max),
                });
                g.tiles.push(Some(GroundTile { walk, to, inv: to.inverse(), min, max }));
            }
        }
        assert_eq!(g.overhangs, vec![6]);

        let brute = |ray: &Ray| -> Option<Vec3> {
            let mut best: Option<f32> = None;
            for (w, to) in g.tiles() {
                let inv = to.inverse();
                let local = Ray {
                    origin: inv.transform_point3(ray.origin),
                    dir: inv.transform_vector3(ray.dir),
                };
                if let Some(t) = w.hit(&local)
                    && best.is_none_or(|b| t < b)
                {
                    best = Some(t);
                }
            }
            best.map(|t| ray.at(t))
        };

        let mut rays = vec![
            // Vertical straight down & up.
            Ray { origin: Vec3::new(15.0, 15.0, 20.0), dir: Vec3::NEG_Z },
            Ray { origin: Vec3::new(15.0, 15.0, -5.0), dir: Vec3::Z },
            // Seam along tile boundary x = 10.0 and y = 20.0.
            Ray { origin: Vec3::new(10.0, 5.0, 10.0), dir: Vec3::new(0.0, 1.0, -0.5).normalize() },
            Ray { origin: Vec3::new(5.0, 20.0, 10.0), dir: Vec3::new(1.0, 0.0, -0.4).normalize() },
            // Corner crossing through (10, 10) -> (20, 20) -> (30, 30).
            Ray { origin: Vec3::new(0.0, 0.0, 8.0), dir: Vec3::new(1.0, 1.0, -0.35).normalize() },
            // Outside start missing and pointing away.
            Ray {
                origin: Vec3::new(-10.0, -10.0, 10.0),
                dir: Vec3::new(-1.0, -1.0, 0.0).normalize(),
            },
            Ray { origin: Vec3::new(15.0, 15.0, 20.0), dir: Vec3::Z },
        ];
        let mut seed = 0x1234_5678_9ABC_DEF0u64;
        let mut next_f = || -> f32 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            ((seed >> 40) as f32) / ((1u64 << 24) as f32)
        };
        for _ in 0..300 {
            let origin =
                Vec3::new(next_f() * 60.0 - 10.0, next_f() * 60.0 - 10.0, next_f() * 15.0 - 2.0);
            let target =
                Vec3::new(next_f() * 45.0 - 2.0, next_f() * 45.0 - 2.0, next_f() * 3.0 - 0.5);
            let dir = (target - origin).normalize_or(Vec3::NEG_Z);
            rays.push(Ray { origin, dir });
        }
        for (idx, ray) in rays.iter().enumerate() {
            match (g.hit(ray), brute(ray)) {
                (Some(a), Some(b)) => assert!(a.distance(b) < 1e-4, "ray {idx}: {a:?} vs {b:?}"),
                (None, None) => {}
                (a, b) => panic!("ray {idx} mismatch: dda={a:?} vs brute={b:?} ({ray:?})"),
            }
        }
    }
}

/// The walkmeshes of an area's placeables and doors (`.pwk`, `.dwk`), each
/// read once per model and door state.
#[derive(Debug, Default)]
pub struct ObjectWalkmeshes {
    cache: HashMap<(String, Option<DoorState>), Option<Arc<Walkmesh>>>,
}

impl ObjectWalkmeshes {
    /// Every face of the placed placeables' and doors' walkmeshes, in the
    /// area's space, with the object's index in `area.objects`. Doors use
    /// their state's surfaces. Placed by the object's position and Bearing
    /// alone ([`AreaObject::transform`](crate::AreaObject::transform)):
    /// visual transforms don't move walkmeshes, in the engine no part of
    /// one (scale, turn, tilt or offset; a static placeable's, a door's and
    /// a script's alike), as creatures' paths showed it
    /// (`walkmeshes_by_visual_transforms` in
    /// `mg-corpus-tests/tests/engine_placeable_walkmesh.rs`).
    pub fn faces(&mut self, game: &GameData, area: &AreaModel) -> Vec<([Vec3; 3], usize)> {
        let mut out = Vec::new();
        for (i, o) in area.objects.iter().enumerate() {
            let door = match o.kind {
                crate::ObjectKind::Door => Some(match o.state {
                    1 => DoorState::Open1,
                    2 => DoorState::Open2,
                    _ => DoorState::Closed,
                }),
                crate::ObjectKind::Placeable => None,
                _ => continue,
            };
            let Some(model) = o.preview.as_ref().map(|p| p.base.model.clone()) else { continue };
            let w = self
                .cache
                .entry((model.clone(), door))
                .or_insert_with(|| Walkmesh::of_object(game, &model, door).map(Arc::new))
                .clone();
            let Some(w) = w else { continue };
            let to = o.transform();
            out.extend(w.faces.iter().map(|f| (f.corners.map(|c| to.transform_point3(c)), i)));
        }
        out
    }
}
