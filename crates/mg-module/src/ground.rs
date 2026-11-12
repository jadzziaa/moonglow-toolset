//! An area's ground, as the game stands things on it: its tiles'
//! walkmeshes (a tile's `.wok`, else the walkmesh nodes, `aabb`, of its
//! model), each placed as the ARE places its tile. Verify's
//! `transition-hidden` and its fix read it, which sit below the area
//! view's own (`mg_area::walk::Ground`, which reads the faces in the same
//! way, and all of them: Verify takes the walkable ones alone,
//! [`walkable_materials`]); `aurora_trigger_order` in the corpus tests
//! holds the two to one height where both have ground.

use std::collections::HashMap;
use std::sync::Arc;

use mg_core::ResType;
use mg_gff::Struct;
use mg_mdl::walkmesh::{Walkmesh, WalkmeshKind, to_model_space};
use mg_mdl::{MeshExtra, Model, NodeKind};
use mg_resman::ResMan;
use mg_set::Tileset;

/// A tile's side, metres.
const TILE: f32 = 10.0;

/// A tile model's walkmesh: its faces in the model's space, and their box
/// in X and Y.
#[derive(Debug, Clone, PartialEq)]
pub struct TileFaces {
    faces: Vec<[[f32; 3]; 3]>,
    min: [f32; 2],
    max: [f32; 2],
}

impl TileFaces {
    /// Tile model `model`'s walkmesh: its `.wok`, else its model's own
    /// walkmesh nodes; of its faces those `walkable` (by surface material,
    /// [`walkable_materials`]; all without it). `None` without either, or
    /// with no such faces.
    pub fn read(rm: &ResMan, model: &str, walkable: Option<&[bool]>) -> Option<TileFaces> {
        let wok = || -> Option<Vec<([[f32; 3]; 3], u32)>> {
            let data = rm.get_named(model, ResType::WOK).ok()?;
            let w = Walkmesh::read(&data, WalkmeshKind::Tile).ok()?;
            Some(w.object_faces(None))
        };
        let own = || -> Option<Vec<([[f32; 3]; 3], u32)>> {
            let data = rm.get_named(model, ResType::MDL).ok()?;
            Some(walkmesh_nodes(&Model::read(&data).ok()?))
        };
        let faces = wok().filter(|f| !f.is_empty()).or_else(own)?;
        let walks = |m: u32| walkable.is_none_or(|w| w.get(m as usize).copied().unwrap_or(false));
        TileFaces::new(faces.into_iter().filter(|(_, m)| walks(*m)).map(|(f, _)| f).collect())
    }

    /// The faces given (in the model's space); `None` for none.
    pub fn new(faces: Vec<[[f32; 3]; 3]>) -> Option<TileFaces> {
        let mut corners = faces.iter().flatten();
        let first = corners.next()?;
        let (mut min, mut max) = ([first[0], first[1]], [first[0], first[1]]);
        for c in corners {
            for k in 0..2 {
                min[k] = min[k].min(c[k]);
                max[k] = max[k].max(c[k]);
            }
        }
        Some(TileFaces { faces, min, max })
    }

    /// The heights of the faces above or below (`x`, `y`), the model's
    /// space.
    fn heights(&self, x: f32, y: f32) -> impl Iterator<Item = f32> + '_ {
        // (A point on the tile's edge may land a rounding error outside.)
        let e = 1e-3;
        let outside = x < self.min[0] - e
            || y < self.min[1] - e
            || x > self.max[0] + e
            || y > self.max[1] + e;
        self.faces.iter().filter(move |_| !outside).filter_map(move |f| height_on(f, x, y))
    }
}

/// Which surface materials (rows of `surfacemat.2da`, the load order's)
/// are walked on (`Walk` 1): the ground Verify holds a transition to. A
/// tile's non-walkable faces are left out: the game's own modules have
/// transitions at another height over them (Chapter 1's `map_m1q4m`, at
/// 0.025 over a `Nonwalk` plane 3 m up), and what the client does there
/// was not read. (Aurora paints on them too: a corner of
/// `transition-triggers/04-base`'s TT_B3.) `None` without the table.
pub fn walkable_materials(rm: &ResMan) -> Option<Vec<bool>> {
    let data = rm.get_named("surfacemat", ResType::TWODA).ok()?;
    let table = mg_2da::TwoDa::parse(&data, mg_core::Codepage::default()).ok()?;
    Some((0..table.len()).map(|r| table.get(r, "Walk").is_some_and(|w| w.trim() == "1")).collect())
}

/// The faces of a model's walkmesh nodes (`aabb`), placed by its rest pose,
/// with their surface materials.
fn walkmesh_nodes(model: &Model) -> Vec<([[f32; 3]; 3], u32)> {
    let mut out = Vec::new();
    for (i, n) in model.nodes.iter().enumerate() {
        let NodeKind::Mesh(mesh) = &n.kind else { continue };
        if !matches!(mesh.extra, MeshExtra::Aabb(_)) {
            continue;
        }
        for f in &mesh.faces {
            let corner = |k: usize| {
                let v = mesh.vertices.get(f.vertices[k] as usize)?;
                Some(to_model_space(model, i, *v))
            };
            if let (Some(a), Some(b), Some(c)) = (corner(0), corner(1), corner(2)) {
                out.push(([a, b, c], f.material));
            }
        }
    }
    out
}

/// The height of face `f` above (`x`, `y`), if the point is inside it
/// (seen from above).
fn height_on([a, b, c]: &[[f32; 3]; 3], x: f32, y: f32) -> Option<f32> {
    let cross = |p: [f32; 2], q: [f32; 2]| p[0] * q[1] - p[1] * q[0];
    let (ab, ac, ap) =
        ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]], [x - a[0], y - a[1]]);
    let d = cross(ab, ac);
    if d.abs() < 1e-9 {
        return None;
    }
    let u = cross(ap, ac) / d;
    let v = cross(ab, ap) / d;
    let e = -1e-5;
    (u >= e && v >= e && u + v <= 1.0 - e).then_some(a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]))
}

/// Tile models' walkmeshes read, by model name (lower case), for the areas
/// of one run.
pub type Kept = HashMap<String, Option<Arc<TileFaces>>>;

/// A tile's walkmesh, its quarter turns and its floor's middle.
type PlacedTile = (Arc<TileFaces>, u8, [f32; 3]);

/// An area's ground: each tile's walkmesh, its quarter turns and its
/// floor's middle.
#[derive(Debug, Default)]
pub struct AreaGround {
    tiles: Vec<Option<PlacedTile>>,
    width: u32,
}

impl AreaGround {
    /// The ground of the area its ARE's root `are` describes, its tiles
    /// those of `set`; each tile model's walkmesh (its `walkable` faces,
    /// [`TileFaces::read`]) read once into `kept`.
    pub fn new(
        rm: &ResMan,
        are: &Struct,
        set: &Tileset,
        walkable: Option<&[bool]>,
        kept: &mut Kept,
    ) -> AreaGround {
        let int = |s: &Struct, label: &str| s.integer(label).unwrap_or(0);
        let width = u32::try_from(int(are, "Width")).unwrap_or(0).min(1024);
        let height = u32::try_from(int(are, "Height")).unwrap_or(0).min(1024);
        let list = are.list("Tile_List").unwrap_or(&[]);
        let tiles = (list.iter().take((width * height) as usize).enumerate())
            .map(|(i, t)| {
                let tile = usize::try_from(int(t, "Tile_ID")).ok().and_then(|i| set.tiles.get(i));
                let model = tile.filter(|t| !t.is_hole())?.model.to_ascii_lowercase();
                let faces = kept
                    .entry(model.clone())
                    .or_insert_with(|| TileFaces::read(rm, &model, walkable).map(Arc::new))
                    .clone()?;
                let (column, row) = (i as u32 % width.max(1), i as u32 / width.max(1));
                let up = int(t, "Tile_Height").clamp(-1024, 1024) as f32 * set.general.transition;
                let middle = [(column as f32 + 0.5) * TILE, (row as f32 + 0.5) * TILE, up];
                Some((faces, int(t, "Tile_Orientation").rem_euclid(4) as u8, middle))
            })
            .collect();
        AreaGround { tiles, width }
    }

    /// The ground's height at (`x`, `y`) nearest to `near` (bridges and
    /// floors above others have several); `None` off the area or its
    /// walkmeshes.
    pub fn height(&self, x: f32, y: f32, near: f32) -> Option<f32> {
        if !(x >= 0.0 && y >= 0.0) || self.width == 0 {
            return None;
        }
        let (column, row) = ((x / TILE) as u32, (y / TILE) as u32);
        if column >= self.width {
            return None;
        }
        let index = usize::try_from(u64::from(row) * u64::from(self.width) + u64::from(column));
        let (faces, turns, middle) = self.tiles.get(index.ok()?)?.as_ref()?;
        let (dx, dy) = (x - middle[0], y - middle[1]);
        // (Into the tile's model: turned back by its quarter turns.)
        let (lx, ly) = match turns {
            0 => (dx, dy),
            1 => (dy, -dx),
            2 => (-dx, -dy),
            _ => (-dy, dx),
        };
        faces
            .heights(lx, ly)
            .map(|z| z + middle[2])
            .min_by(|a, b| (a - near).abs().total_cmp(&(b - near).abs()))
    }
}

/// What reading areas' grounds keeps from one area to the next: tile
/// models' walkmeshes and the tilesets.
#[derive(Debug, Default)]
pub struct Grounds {
    walkmeshes: Kept,
    sets: HashMap<mg_core::ResRef, Option<Arc<Tileset>>>,
    walkable: Option<Option<Vec<bool>>>,
}

impl Grounds {
    /// The ground of the module's area `area` (its ARE, with its tileset
    /// from `rm`); `None` without either.
    pub fn of(
        &mut self,
        rm: &ResMan,
        module: &crate::Module,
        area: mg_core::ResRef,
    ) -> Option<AreaGround> {
        let are = module.gff(&mg_resman::ResKey::new(area, ResType::ARE))?.ok()?;
        let name = are.root.resref("Tileset")?;
        let set = self
            .sets
            .entry(name)
            .or_insert_with(|| crate::minimap::tileset(rm, &are.root).ok().map(Arc::new))
            .clone()?;
        let walkable = self.walkable.get_or_insert_with(|| walkable_materials(rm)).as_deref();
        Some(AreaGround::new(rm, &are.root, &set, walkable, &mut self.walkmeshes))
    }
}

/// The point of trigger `t`'s outline (its GIT struct) nearest where
/// Aurora puts it ([`crate::instances::OUTLINE_LIFT`] above the ground:
/// the game's height of it, `ZPosition` + `PointZ`, against the ground's
/// height nearest it): its index and how far from there, metres, above
/// (positive) or below; how far the outline as a whole is off the
/// ground, at least. (A corner on a rock or a slope's edge may stand off
/// while the rest is on it: the client was read with whole outlines off.)
/// `None` when no point has ground under it.
pub fn nearest_off(ground: &AreaGround, t: &Struct) -> Option<(usize, f32)> {
    let f = |s: &Struct, label: &str| s.float(label).unwrap_or(0.0);
    let (x, y, z) = (f(t, "XPosition"), f(t, "YPosition"), f(t, "ZPosition"));
    let points = t.list("Geometry").unwrap_or(&[]);
    (points.iter().enumerate())
        .filter_map(|(i, p)| {
            let (px, py, pz) = (x + f(p, "PointX"), y + f(p, "PointY"), z + f(p, "PointZ"));
            let under = ground.height(px, py, pz)?;
            Some((i, pz - under - crate::instances::OUTLINE_LIFT))
        })
        // (Of points as near, the first.)
        .min_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
}

/// Trigger `t`'s outline (its `Geometry`, each point's struct kept) in
/// Aurora's order ([`crate::instances::outline_order`]), and on `ground`
/// as Redraw Polygon draws it, as Aurora's does: the trigger at height 0
/// (its `ZPosition`, the second of what is given back) and each point at
/// the ground under it plus [`crate::instances::OUTLINE_LIFT`] (a point
/// with no walkable ground under it at the height it had, `ZPosition` +
/// `PointZ`). With no `ground`, only in order: `ZPosition` as it was.
pub fn outline_put_right(t: &Struct, ground: Option<&AreaGround>) -> (Vec<Struct>, f32) {
    let f = |s: &Struct, label: &str| s.float(label).unwrap_or(0.0);
    let (x, y, z) = (f(t, "XPosition"), f(t, "YPosition"), f(t, "ZPosition"));
    let points = t.list("Geometry").unwrap_or(&[]);
    let order = crate::instances::outline_order(&crate::instances::trigger_xy(points));
    let to_z = if ground.is_some() { 0.0 } else { z };
    let put = order
        .into_iter()
        .map(|i| {
            let mut p = points[i].clone();
            if let Some(g) = ground {
                let (px, py, pz) = (x + f(&p, "PointX"), y + f(&p, "PointY"), z + f(&p, "PointZ"));
                let height = match g.height(px, py, pz) {
                    Some(under) => crate::instances::outline_point_z(under, 0.0),
                    None => pz,
                };
                if p.float("PointZ") != Some(height) {
                    p.set("PointZ", mg_gff::Value::Float(height));
                }
            }
            p
        })
        .collect();
    (put, to_z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_gff::Value;

    /// A trigger at (`x`, `y`, `z`) with points (relative X and Y, `PointZ`).
    fn trigger(at: [f32; 3], points: &[[f32; 3]]) -> Struct {
        let mut t = Struct::new(1);
        for (l, v) in ["XPosition", "YPosition", "ZPosition"].iter().zip(at) {
            t.set(l, Value::Float(v));
        }
        let list = points
            .iter()
            .map(|c| {
                let mut p = Struct::new(3);
                for (l, v) in ["PointX", "PointY", "PointZ"].iter().zip(c) {
                    p.set(l, Value::Float(*v));
                }
                p
            })
            .collect();
        t.set("Geometry", Value::List(list));
        t
    }

    /// Flat ground 3 m up, one tile.
    fn flat() -> AreaGround {
        let (sw, se, ne, nw) =
            ([-5.0, -5.0, 0.0], [5.0, -5.0, 0.0], [5.0, 5.0, 0.0], [-5.0, 5.0, 0.0]);
        let faces = Arc::new(TileFaces::new(vec![[sw, se, ne], [sw, ne, nw]]).unwrap());
        AreaGround { tiles: vec![Some((faces, 0, [5.0, 5.0, 3.0]))], width: 1 }
    }

    /// How far an outline is off the ground (its point nearest it), and a
    /// clockwise outline off it put in Aurora's order on it against its
    /// trigger's `ZPosition`.
    #[test]
    fn an_outline_off_the_ground_is_found_and_put_back_on_it() {
        let g = flat();
        let lift = crate::instances::OUTLINE_LIFT;
        // At height 1, its points 2.025 up: on the ground 3 m up.
        let on = trigger([2.0, 2.0, 1.0], &[[0.0, 0.0, 2.0 + lift], [4.0, 0.0, 2.0 + lift]]);
        let (_, by) = nearest_off(&g, &on).unwrap();
        assert!(by.abs() < 1e-5, "{by}");
        // Clockwise, at height -1, one point 0.3 above and one 0.2 below:
        // the outline as a whole on the ground.
        let pz = [4.0 + lift, 4.3 + lift, 3.8 + lift];
        let t =
            trigger([2.0, 2.0, -1.0], &[[0.0, 0.0, pz[0]], [0.0, 4.0, pz[1]], [4.0, 0.0, pz[2]]]);
        let (i, by) = nearest_off(&g, &t).unwrap();
        assert!(i == 0 && by.abs() < 1e-4, "{i} {by}");
        // Raised 0.5 whole: off it by 0.5, at least (the one 0.2 below
        // now 0.3 above).
        let mut raised = t.clone();
        raised.set("ZPosition", Value::Float(-0.5));
        let (i, by) = nearest_off(&g, &raised).unwrap();
        assert!(i == 2 && (by - 0.3).abs() < 1e-4, "{i} {by}");
        let (put, z) = outline_put_right(&t, Some(&g));
        assert_eq!(z, 0.0, "at height 0, as Redraw Polygon puts it");
        let xy: Vec<[f32; 2]> =
            put.iter().map(|p| [p.float("PointX").unwrap(), p.float("PointY").unwrap()]).collect();
        assert_eq!(xy, [[0.0, 0.0], [4.0, 0.0], [0.0, 4.0]], "counterclockwise");
        for p in &put {
            let h = p.float("PointZ").unwrap();
            assert!((h - 3.0 - lift).abs() < 1e-5, "{h}");
        }
        // With no ground: only the order, at its height.
        let (put, z) = outline_put_right(&t, None);
        assert_eq!((put[1].float("PointZ"), z), (Some(pz[2]), -1.0));
        // A point off the ground's tile keeps its height, at height 0.
        let wide = trigger([2.0, 2.0, -1.0], &[[0.0, 0.0, 9.0], [30.0, 0.0, 9.0], [0.0, 4.0, 9.0]]);
        let (put, _) = outline_put_right(&wide, Some(&g));
        let heights: Vec<f32> = put.iter().map(|p| p.float("PointZ").unwrap()).collect();
        assert!((heights[0] - 3.0 - lift).abs() < 1e-5 && heights[1] == 8.0, "{heights:?}");
        // Off the ground's tile: no verdict on the height.
        let away = trigger([22.0, 2.0, 0.0], &[[0.0, 0.0, 9.0]]);
        assert!(nearest_off(&g, &away).is_none());
    }

    /// A 10 m tile model's floor, two faces, its middle at the origin,
    /// rising 1 m to the east.
    fn floor() -> Arc<TileFaces> {
        let (sw, se, ne, nw) =
            ([-5.0, -5.0, 0.0], [5.0, -5.0, 1.0], [5.0, 5.0, 1.0], [-5.0, 5.0, 0.0]);
        Arc::new(TileFaces::new(vec![[sw, se, ne], [sw, ne, nw]]).unwrap())
    }

    #[test]
    fn the_height_is_the_tile_s_walkmesh_turned_and_raised_where_it_stands() {
        // Two tiles: the first as made, the second turned a quarter
        // (rising to the north) and raised 2 m.
        let ground = AreaGround {
            tiles: vec![
                Some((floor(), 0, [5.0, 5.0, 0.0])),
                Some((floor(), 1, [15.0, 5.0, 2.0])),
                None,
            ],
            width: 3,
        };
        let at = |x, y| ground.height(x, y, 0.0).map(|z| (z * 1000.0).round() / 1000.0);
        assert_eq!(at(0.0, 5.0), Some(0.0));
        assert_eq!(at(7.5, 5.0), Some(0.75));
        assert_eq!(at(15.0, 0.0), Some(2.0));
        assert_eq!(at(15.0, 9.0), Some(2.9));
        // Off its walkmeshes and off the area: none.
        assert_eq!(at(25.0, 5.0), None);
        assert_eq!(at(-1.0, 5.0), None);
        assert_eq!(at(5.0, 15.0), None);
        assert_eq!(at(35.0, 5.0), None);
        assert_eq!(ground.height(f32::NAN, 5.0, 0.0), None);
        assert!(TileFaces::new(Vec::new()).is_none());
    }
}
