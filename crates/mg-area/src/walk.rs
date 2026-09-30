//! The ground: tiles' walkmeshes, where objects stand.
//!
//! A tile's walkmesh is its `.wok` (what the engine uses), else the walkmesh
//! node (`aabb`) its model carries. Faces keep their surface material
//! (surfacemat.2da).

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3};
use mg_core::ResType;
use mg_mdl::{MeshExtra, Model, NodeKind};
use mg_rules::GameData;

use crate::pick::Ray;
use crate::{AreaModel, TILE_SIZE};

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
        let pose = mg_render::rest_pose(model);
        let mut faces = Vec::new();
        for (i, n) in model.nodes.iter().enumerate() {
            let NodeKind::Mesh(mesh) = &n.kind else { continue };
            if !matches!(mesh.extra, MeshExtra::Aabb(_)) {
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
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in faces.iter().flat_map(|f| f.corners) {
            min = min.min(p);
            max = max.max(p);
        }
        Some(Walkmesh { faces, min, max })
    }

    /// A tile's walkmesh: `<model>.wok`, else its model's own.
    pub fn of_tile(game: &GameData, model: &str) -> Option<Walkmesh> {
        let read = |t: ResType| -> Option<Walkmesh> {
            let data = game.resman.get_named(model, t).ok()?;
            Walkmesh::from_model(&Model::read(&data).ok()?)
        };
        read(ResType::WOK).or_else(|| read(ResType::MDL))
    }

    /// The heights of the faces above or below `p` (the model's space).
    pub fn heights(&self, p: Vec2) -> impl Iterator<Item = f32> + '_ {
        let outside = p.x < self.min.x || p.y < self.min.y || p.x > self.max.x || p.y > self.max.y;
        self.faces.iter().filter(move |_| !outside).filter_map(move |f| height_on(f, p))
    }

    /// The distance along `ray` (in the model's space) to the nearest face.
    pub fn hit(&self, ray: &Ray) -> Option<f32> {
        ray.hits_box(self.min, self.max, Mat4::IDENTITY)?;
        self.faces.iter().filter_map(|f| hit_triangle(ray, f.corners)).min_by(f32::total_cmp)
    }
}

/// The height of face `f` above `p`, if `p` is inside it (seen from above).
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

/// Möller–Trumbore: the distance along `ray` to a triangle.
fn hit_triangle(ray: &Ray, [a, b, c]: [Vec3; 3]) -> Option<f32> {
    let (e1, e2) = (b - a, c - a);
    let p = ray.dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-9 {
        return None;
    }
    let s = ray.origin - a;
    let u = s.dot(p) / det;
    let q = s.cross(e1);
    let v = ray.dir.dot(q) / det;
    if u < 0.0 || v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) / det;
    (t >= 0.0).then_some(t)
}

/// An area's ground: its tiles' walkmeshes, each read once per model.
#[derive(Debug, Default)]
pub struct Ground {
    /// By tile index, with the tile's transform.
    tiles: Vec<Option<(Arc<Walkmesh>, Mat4)>>,
    cache: HashMap<String, Option<Arc<Walkmesh>>>,
    width: u32,
}

impl Ground {
    pub fn new(game: &GameData, area: &AreaModel) -> Ground {
        let mut g = Ground::default();
        g.update(game, area);
        g
    }

    /// Follows `area` after an edit (reading walkmeshes not seen before).
    pub fn update(&mut self, game: &GameData, area: &AreaModel) {
        let cache = &mut self.cache;
        self.tiles = area
            .tiles
            .iter()
            .map(|t| {
                let name = t.model.as_ref()?;
                let w = cache
                    .entry(name.clone())
                    .or_insert_with(|| Walkmesh::of_tile(game, name).map(Arc::new))
                    .clone()?;
                Some((w, t.transform()))
            })
            .collect();
        self.width = area.width;
    }

    /// The tile index at a point, if it is inside the area.
    fn tile_at(&self, p: Vec2) -> Option<usize> {
        if p.x < 0.0 || p.y < 0.0 || self.width == 0 {
            return None;
        }
        let (x, y) = ((p.x / TILE_SIZE) as u32, (p.y / TILE_SIZE) as u32);
        let i = (y * self.width + x) as usize;
        (x < self.width && i < self.tiles.len()).then_some(i)
    }

    /// The ground's height at `p` nearest to `near` (bridges and floors
    /// above others have several).
    pub fn height(&self, p: Vec2, near: f32) -> Option<f32> {
        let (w, to) = self.tiles.get(self.tile_at(p)?)?.as_ref()?;
        let local = to.inverse().transform_point3(p.extend(0.0));
        let base = to.transform_point3(Vec3::ZERO).z;
        w.heights(local.truncate())
            .map(|z| z + base)
            .min_by(|a, b| (a - near).abs().total_cmp(&(b - near).abs()))
    }

    /// Where `ray` first meets the ground.
    pub fn hit(&self, ray: &Ray) -> Option<Vec3> {
        let mut best: Option<f32> = None;
        for (w, to) in self.tiles.iter().flatten() {
            let inverse = to.inverse();
            let local = Ray {
                origin: inverse.transform_point3(ray.origin),
                dir: inverse.transform_vector3(ray.dir),
            };
            if let Some(t) = w.hit(&local)
                && best.is_none_or(|b| t < b)
            {
                best = Some(t);
            }
        }
        best.map(|t| ray.at(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn heights_and_hits_follow_the_faces() {
        let w = ramp();
        let h: Vec<f32> = w.heights(Vec2::new(0.0, 2.0)).collect();
        assert_eq!(h, [2.5]);
        assert_eq!(w.heights(Vec2::new(6.0, 0.0)).count(), 0);
        let down = Ray { origin: Vec3::new(2.5, 0.0, 10.0), dir: Vec3::NEG_Z };
        assert!((w.hit(&down).unwrap() - 6.25).abs() < 1e-4);
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
        let g = Ground {
            tiles: vec![Some((w.clone(), at(5.0, 0.0, 0.0))), Some((w, at(15.0, 2.0, 2.0)))],
            cache: HashMap::new(),
            width: 2,
        };
        assert!((g.height(Vec2::new(7.5, 5.0), 0.0).unwrap() - 3.75).abs() < 1e-4);
        assert!((g.height(Vec2::new(17.5, 5.0), 0.0).unwrap() - 3.25).abs() < 1e-4);
        assert!(g.height(Vec2::new(25.0, 5.0), 0.0).is_none(), "outside the area");
        let down = Ray { origin: Vec3::new(12.5, 5.0, 50.0), dir: Vec3::NEG_Z };
        let hit = g.hit(&down).unwrap();
        assert!((hit - Vec3::new(12.5, 5.0, 5.75)).length() < 1e-4, "{hit}");
    }
}
