//! Walkmesh files: tiles' `.wok`, placeables' `.pwk` and doors' `.dwk`.
//!
//! They are ASCII models without a model header, whose nodes name the
//! object's root as their parent without defining it: the reader adds that
//! root, at the object's origin, so the nodes sit where the object's model
//! puts them. A tile's walkmesh is an `aabb` node; a placeable's a mesh
//! (`…_wg`) and the points a creature uses it from (`…_use01`, `…_use02`);
//! a door's a mesh per state (`…_wg_closed`, `…_wg_open1`, `…_wg_open2`),
//! the points creatures pass it at in each (`…_dp_open1_01`, …) and its use
//! points.

use crate::{IDENTITY, MdlError, Model, NodeKind, Quat, Vec3};

/// What a walkmesh belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkmeshKind {
    Tile,
    Placeable,
    Door,
}

impl WalkmeshKind {
    /// By file extension (`wok`, `pwk`, `dwk`, any case).
    pub fn from_extension(ext: &str) -> Option<WalkmeshKind> {
        match ext.to_ascii_lowercase().as_str() {
            "wok" => Some(WalkmeshKind::Tile),
            "pwk" => Some(WalkmeshKind::Placeable),
            "dwk" => Some(WalkmeshKind::Door),
            _ => None,
        }
    }
}

/// A door's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoorState {
    Closed,
    /// Opened one way.
    Open1,
    /// Opened the other way.
    Open2,
}

impl DoorState {
    fn find(name: &str) -> Option<(DoorState, usize)> {
        [("closed", DoorState::Closed), ("open1", DoorState::Open1), ("open2", DoorState::Open2)]
            .into_iter()
            .find_map(|(word, state)| name.find(word).map(|at| (state, at + word.len())))
    }
}

/// A walkable surface: a mesh node of [`Walkmesh::model`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surface {
    pub node: usize,
    /// The door state it is for (doors' meshes).
    pub door: Option<DoorState>,
}

/// What a point is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointKind {
    /// Where a creature stands to use the object (`use01`, `use02`).
    Use(u32),
    /// Where creatures pass a door in a state (`dp_open1_01`, …).
    Door(DoorState, u32),
    /// Another dummy.
    Other,
}

/// A point the walkmesh marks (a dummy node).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub node: usize,
    pub kind: PointKind,
    /// In the object's space.
    pub position: Vec3,
}

/// A walkmesh file.
#[derive(Debug, Clone, PartialEq)]
pub struct Walkmesh {
    pub kind: WalkmeshKind,
    /// The file's nodes under the object's root (`nodes[0]`).
    pub model: Model,
    pub surfaces: Vec<Surface>,
    pub points: Vec<Point>,
}

/// The number that ends a name (`use01` → 1).
fn trailing_number(name: &str) -> u32 {
    let digits = name.len() - name.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    name[name.len() - digits..].parse().unwrap_or(0)
}

impl Walkmesh {
    /// Reads a walkmesh (ASCII, or compiled like a model). A file without
    /// nodes is a walkmesh without surfaces (hundreds of the game's are).
    pub fn read(data: &[u8], kind: WalkmeshKind) -> Result<Walkmesh, MdlError> {
        let model = if crate::is_binary(data) {
            crate::binary::read(data)?
        } else {
            match crate::ascii::read_with(data, true) {
                Ok((m, _)) => m,
                Err(MdlError::Empty) => {
                    let text = String::from_utf8_lossy(data);
                    let name = text
                        .lines()
                        .map(|l| l.split_whitespace().collect::<Vec<_>>())
                        .find(|w| {
                            w.first().is_some_and(|k| k.eq_ignore_ascii_case("beginwalkmeshgeom"))
                        })
                        .and_then(|w| w.get(1).map(|n| n.to_string()))
                        .unwrap_or_default();
                    Model {
                        name: name.clone(),
                        animation_scale: 1.0,
                        nodes: vec![crate::Node::new(&name, NodeKind::Dummy)],
                        ..Default::default()
                    }
                }
                Err(e) => return Err(e),
            }
        };
        let mut surfaces = Vec::new();
        let mut points = Vec::new();
        for (i, n) in model.nodes.iter().enumerate().skip(1) {
            let name = n.name.to_ascii_lowercase();
            match &n.kind {
                NodeKind::Mesh(m) if !m.faces.is_empty() => {
                    let door = match kind {
                        WalkmeshKind::Door => DoorState::find(&name).map(|(s, _)| s),
                        _ => None,
                    };
                    surfaces.push(Surface { node: i, door });
                }
                NodeKind::Dummy => {
                    let point = if let Some(at) = name.find("_dp_") {
                        DoorState::find(&name[at..])
                            .map(|(s, _)| PointKind::Door(s, trailing_number(&name)))
                    } else if name.trim_end_matches(|c: char| c.is_ascii_digit()).ends_with("_use")
                    {
                        Some(PointKind::Use(trailing_number(&name)))
                    } else {
                        None
                    };
                    let kind = point.unwrap_or(PointKind::Other);
                    points.push(Point { node: i, kind, position: object_position(&model, i) });
                }
                _ => {}
            }
        }
        Ok(Walkmesh { kind, model, surfaces, points })
    }

    /// The surfaces creatures walk on in a door state (all of them for tiles
    /// and placeables, and with `None`).
    pub fn surfaces_for(&self, state: Option<DoorState>) -> impl Iterator<Item = &Surface> + '_ {
        self.surfaces.iter().filter(move |s| state.is_none() || s.door.is_none() || s.door == state)
    }

    /// The use points, in order.
    pub fn use_points(&self) -> Vec<Vec3> {
        let mut uses: Vec<(u32, Vec3)> = self
            .points
            .iter()
            .filter_map(|p| match p.kind {
                PointKind::Use(n) => Some((n, p.position)),
                _ => None,
            })
            .collect();
        uses.sort_by_key(|(n, _)| *n);
        uses.into_iter().map(|(_, p)| p).collect()
    }
}

fn rotate(q: Quat, v: Vec3) -> Vec3 {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    let q = if len > 1e-6 { q.map(|c| c / len) } else { IDENTITY };
    let (u, w) = ([q[0], q[1], q[2]], q[3]);
    let cross = |a: Vec3, b: Vec3| {
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    };
    // v + 2w (u × v) + 2 u × (u × v)
    let t = cross(u, v);
    let tt = cross(u, t);
    [0, 1, 2].map(|k| v[k] + 2.0 * (w * t[k] + tt[k]))
}

/// A node's origin in the model's space (rest pose).
pub(crate) fn object_position(model: &Model, node: usize) -> Vec3 {
    let mut p = [0.0f32; 3];
    let mut at = Some(node);
    while let Some(i) = at {
        let n = &model.nodes[i];
        let r = rotate(n.orientation, p.map(|c| c * n.scale));
        p = [r[0] + n.position[0], r[1] + n.position[1], r[2] + n.position[2]];
        at = n.parent.filter(|&parent| parent < i);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    const PWK: &str = "\
#MAXDOOR ASCII
node trimesh BOX_wg
  parent BOX_pwk
  position 0.0 -0.5 0.0
  orientation 0.0 0.0 0.0 0.0
  verts 3
    0 0 0
    1 0 0
    0 1 0
  faces 1
    0 1 2  1  0 0 0  7
endnode
node dummy BOX_pwk_use01
  parent BOX_pwk
  position 0.0 -1.0 0.0
  orientation 0.0 0.0 0.0 0.0
endnode
node dummy BOX_pwk_use02
  parent BOX_pwk
  position 0.0 1.0 0.0
endnode
";

    #[test]
    fn placeable_nodes_hang_from_the_objects_root() {
        let w = Walkmesh::read(PWK.as_bytes(), WalkmeshKind::Placeable).unwrap();
        assert_eq!(w.model.nodes[0].name, "BOX_pwk");
        assert!(w.model.nodes[1..].iter().all(|n| n.parent == Some(0)));
        assert_eq!(w.surfaces, [Surface { node: 1, door: None }]);
        // Not offset by the walk mesh's position, as they would be under it.
        assert_eq!(w.use_points(), [[0.0, -1.0, 0.0], [0.0, 1.0, 0.0]]);
        assert_eq!(w.model.nodes[1].mesh().unwrap().faces[0].material, 7);
    }

    #[test]
    fn door_states() {
        let dwk = "\
node trimesh 09_DWK_wg_closed
  parent T_Door09_DWK
  verts 3
    0 0 0
    1 0 0
    0 1 0
  faces 1
    0 1 2 1 0 0 0 1
endnode
node trimesh 09_DWK_wg_open1
  parent T_Door09_DWK
  position -1 0 0
  orientation 0 0 1 1.5707964
  verts 3
    0 0 0
    1 0 0
    0 1 0
  faces 1
    0 1 2 1 0 0 0 1
endnode
node dummy 09_DWK_dp_open1_02
  parent 09_DWK_wg_open1
  position 1 0 0
endnode
node dummy T_Door09_DWK_use01
  parent T_Door09_DWK
endnode
";
        let w = Walkmesh::read(dwk.as_bytes(), WalkmeshKind::Door).unwrap();
        let states: Vec<Option<DoorState>> = w.surfaces.iter().map(|s| s.door).collect();
        assert_eq!(states, [Some(DoorState::Closed), Some(DoorState::Open1)]);
        assert_eq!(w.surfaces_for(Some(DoorState::Open1)).count(), 1);
        let dp = w.points.iter().find(|p| matches!(p.kind, PointKind::Door(..))).unwrap();
        assert_eq!(dp.kind, PointKind::Door(DoorState::Open1, 2));
        // Turned a quarter about Z under its parent at (-1, 0, 0).
        let close = |a: Vec3, b: Vec3| (0..3).all(|k| (a[k] - b[k]).abs() < 1e-5);
        assert!(close(dp.position, [-1.0, 1.0, 0.0]), "{:?}", dp.position);
        assert_eq!(w.use_points().len(), 1);
    }

    #[test]
    fn tiles() {
        let wok = "\
#MAXWALKMESH  ASCII
beginwalkmeshgeom TCN01_A01_01
node aabb colmesh40
  parent TCN01_A01_01
  position 0.0 0.0 3.0
  verts 3
    0 0 0
    1 0 0
    0 1 0
  faces 1
    0 1 2 1 0 0 0 4
aabb 0 0 0 1 1 0 0
endnode
endwalkmeshgeom TCN01_A01_01
";
        let w = Walkmesh::read(wok.as_bytes(), WalkmeshKind::Tile).unwrap();
        assert_eq!(w.model.name, "TCN01_A01_01");
        assert_eq!(w.surfaces.len(), 1);
        assert_eq!(object_position(&w.model, w.surfaces[0].node), [0.0, 0.0, 3.0]);
        assert_eq!(WalkmeshKind::from_extension("DWK"), Some(WalkmeshKind::Door));
        let empty = "beginwalkmeshgeom tcm02_a40_02\nendwalkmeshgeom tcm02_a40_02\n";
        let w = Walkmesh::read(empty.as_bytes(), WalkmeshKind::Tile).unwrap();
        assert_eq!((w.model.name.as_str(), w.surfaces.len()), ("tcm02_a40_02", 0));
    }
}
