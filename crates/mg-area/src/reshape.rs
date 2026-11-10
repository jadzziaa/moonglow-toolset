//! Resize Area and Rotate Area, as Aurora does them (captured:
//! `aurora_terrain.rs`, `resize_and_rotate_match_aurora`).
//!
//! - Resizing keeps the south-west corner: rows and columns come and go at
//!   the north and east. New tiles continue the terrain at the old edge
//!   (each new corner is the nearest old one, each new edge the nearest old
//!   edge across), chosen at random among the tiles that fit. Objects
//!   outside the new area go, and so do groups the new edge cuts: their
//!   remaining tiles become terrain tiles, and the doors on their hooks go.
//! - Rotating turns every tile and object about the area, a quarter turn
//!   counter-clockwise at a time: tile (x, y) of an area `h` tiles high
//!   goes to (h − 1 − y, x), turned once more; points turn about the
//!   origin and move east by the new width; facings turn with them.

use std::f32::consts::{FRAC_PI_2, PI};

use mg_edit::{Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_set::Tileset;
use mg_tiles::paint::Grid;
use mg_tiles::{Lattice, TileIndex};

use crate::{ObjectKind, TILE_SIZE};

/// What resizing or rotating does: the edits, and how many objects it
/// deleted.
#[derive(Debug, Clone, PartialEq)]
pub struct Reshaped {
    pub edits: Vec<Edit>,
    pub deleted: usize,
}

/// The placed groups in a grid: the group (index in the tileset `set`) and
/// the cells of its tiles, by the painting grid's own rule for which group
/// stands in a cell ([`Grid::groups`], `TileIndex::group_at`: from any of
/// its tiles, turned and raised as one).
pub fn groups_in(index: &TileIndex, _set: &Tileset, grid: &Grid) -> Vec<(usize, Vec<(u32, u32)>)> {
    grid.groups(index)
}

/// Where an object stands (its position fields).
fn position(kind: ObjectKind, s: &Struct) -> (f32, f32) {
    let [x, y, _] = mg_module::placement::position_fields(kind);
    (s.float(x).unwrap_or(0.0), s.float(y).unwrap_or(0.0))
}

/// Resizing the area (its ARE `are`, GIT `git`) to `width` by `height`
/// tiles; `lights` gives each new tile's lights. `None` when the size is
/// outside Aurora's 2 to 32, the area's tiles are not the tileset's, or it
/// has none (a width or height of 0).
#[allow(clippy::too_many_arguments)]
pub fn resize(
    are_key: ResKey,
    git_key: ResKey,
    are: &Struct,
    git: &Struct,
    set: &Tileset,
    index: &TileIndex,
    width: u32,
    height: u32,
    rng: &mut fastrand::Rng,
    lights: &mut dyn FnMut() -> [u8; 3],
) -> Option<Reshaped> {
    if !(2..=32).contains(&width) || !(2..=32).contains(&height) {
        return None;
    }
    let old = crate::terrain::grid(are, index)?;
    let (ow, oh) = (old.lattice.width(), old.lattice.height());
    // (An area of no tiles has no edge to continue.)
    if ow == 0 || oh == 0 {
        return None;
    }
    // The terrain, continued past the old edges.
    let fill = old.lattice.corner(0, 0);
    let mut lattice = Lattice::new(width, height, fill);
    for y in 0..=height {
        for x in 0..=width {
            lattice.set_corner(x, y, old.lattice.corner(x.min(ow), y.min(oh)));
        }
    }
    // Edges along x (row `ey`, from column `ex`) and along y (column `ex`,
    // from row `ey`) of the old lattice.
    let x_edge = |ex: u32, ey: u32| {
        if ey < oh {
            old.lattice.cell(ex, ey).edges[mg_tiles::SOUTH]
        } else {
            old.lattice.cell(ex, oh - 1).edges[mg_tiles::NORTH]
        }
    };
    let y_edge = |ex: u32, ey: u32| {
        if ex < ow {
            old.lattice.cell(ex, ey).edges[mg_tiles::WEST]
        } else {
            old.lattice.cell(ow - 1, ey).edges[mg_tiles::EAST]
        }
    };
    for y in 0..height {
        for x in 0..width {
            let (cx, cy) = (x.min(ow - 1), y.min(oh - 1));
            lattice.set_edge(x, y, mg_tiles::SOUTH, x_edge(cx, y.min(oh)));
            lattice.set_edge(x, y, mg_tiles::NORTH, x_edge(cx, (y + 1).min(oh)));
            lattice.set_edge(x, y, mg_tiles::WEST, y_edge(x.min(ow), cy));
            lattice.set_edge(x, y, mg_tiles::EAST, y_edge((x + 1).min(ow), cy));
        }
    }
    // Groups the new edge cuts.
    let cut: Vec<(u32, u32)> = groups_in(index, set, &old)
        .into_iter()
        .filter(|(_, cells)| cells.iter().any(|&(x, y)| x >= width || y >= height))
        .flat_map(|(_, cells)| cells)
        .filter(|&(x, y)| x < width && y < height)
        .collect();
    let old_list = are.list("Tile_List").unwrap_or(&[]);
    let mut list = Vec::with_capacity((width * height) as usize);
    let default = index.terrain(&set.general.default);
    for y in 0..height {
        for x in 0..width {
            let kept = x < ow && y < oh && !cut.contains(&(x, y));
            if kept && let Some(s) = old_list.get((y * ow + x) as usize) {
                list.push(s.clone());
                continue;
            }
            let fits = index.fits(&lattice.cell(x, y));
            let p = match fits.len() {
                0 => {
                    // Nothing continues the edge here: the Default terrain.
                    let d = default.map(|t| mg_tiles::Corner { terrain: t, height: 0 });
                    let plain = d.map(|c| mg_tiles::Cell { corners: [c; 4], edges: [None; 4] });
                    *plain.and_then(|c| index.fits(&c).first().copied()).as_ref()?
                }
                n => fits[rng.usize(..n)],
            };
            let [main1, main2, source] = lights();
            list.push(crate::terrain::tile_struct(set, p, [main1, main2, source, source]));
        }
    }
    let mut edits = vec![
        set_field(are_key, "Width", Value::Int(width as i32)),
        set_field(are_key, "Height", Value::Int(height as i32)),
        set_field(are_key, "Tile_List", Value::List(list)),
    ];
    // Objects outside go, and doors on the hooks of the cut groups' tiles.
    let hooks: Vec<(f32, f32)> = cut
        .iter()
        .flat_map(|&(x, y)| crate::terrain::typed_hooks(set, (x, y), old.tile(x, y), 0.0))
        .map(|(_, at, _)| (at.x, at.y))
        .collect();
    let (wm, hm) = (width as f32 * TILE_SIZE, height as f32 * TILE_SIZE);
    let mut deleted = 0;
    for kind in ObjectKind::ALL {
        let Some(objects) = git.list(kind.list()) else { continue };
        let kept: Vec<Struct> = objects
            .iter()
            .filter(|s| {
                let (x, y) = position(kind, s);
                let outside = x < 0.0 || y < 0.0 || x >= wm || y >= hm;
                let on_cut = kind == ObjectKind::Door
                    && hooks.iter().any(|&(hx, hy)| (hx - x).abs() < 0.05 && (hy - y).abs() < 0.05);
                !(outside || on_cut)
            })
            .cloned()
            .collect();
        if kept.len() != objects.len() {
            deleted += objects.len() - kept.len();
            edits.push(set_field(git_key, kind.list(), Value::List(kept)));
        }
    }
    Some(Reshaped { edits, deleted })
}

/// [`resize`] at other edges than the north and east ones: `turns` says
/// which two the rows and columns come and go at (0 north and east, 1
/// east and south, 2 south and west, 3 west and north). The area is
/// turned that many quarters, resized and turned back, so what stays
/// keeps its place against the two edges that stay.
#[allow(clippy::too_many_arguments)]
pub fn resize_at(
    are_key: ResKey,
    git_key: ResKey,
    are: &Struct,
    git: &Struct,
    set: &Tileset,
    index: &TileIndex,
    width: u32,
    height: u32,
    turns: u8,
    rng: &mut fastrand::Rng,
    lights: &mut dyn FnMut() -> [u8; 3],
) -> Option<Reshaped> {
    let turns = turns % 4;
    if turns == 0 {
        return resize(are_key, git_key, are, git, set, index, width, height, rng, lights);
    }
    let (mut a, mut g) = (are.clone(), git.clone());
    for _ in 0..turns {
        rotate_once(&mut a, &mut g);
    }
    let (w, h) = if turns % 2 == 1 { (height, width) } else { (width, height) };
    let r = resize(are_key, git_key, &a, &g, set, index, w, h, rng, lights)?;
    for e in r.edits {
        if let Edit::SetField { key, label, value: Some(v), .. } = e {
            let of = if key == are_key { &mut a } else { &mut g };
            of.set(&label, v);
        }
    }
    for _ in 0..4 - turns {
        rotate_once(&mut a, &mut g);
    }
    let mut edits = Vec::new();
    for label in ["Width", "Height", "Tile_List"] {
        if let Some(v) = a.get(label) {
            edits.push(set_field(are_key, label, v.clone()));
        }
    }
    for kind in ObjectKind::ALL {
        if let Some(v) = g.get(kind.list()) {
            edits.push(set_field(git_key, kind.list(), v.clone()));
        }
    }
    Some(Reshaped { edits, deleted: r.deleted })
}

fn set_field(key: ResKey, label: &str, value: Value) -> Edit {
    Edit::SetField { key, path: GffPath::root(), label: label.into(), value: Some(value) }
}

/// An angle turned a quarter counter-clockwise, in [−π, π) (Aurora turns a
/// door facing west to −π).
fn turned(a: f32) -> f32 {
    let mut b = a + FRAC_PI_2;
    if b >= PI - 1e-5 {
        b -= 2.0 * PI;
    }
    b
}

/// Rotating the area `quarter_turns` quarter turns counter-clockwise.
pub fn rotate(
    are_key: ResKey,
    git_key: ResKey,
    are: &Struct,
    git: &Struct,
    quarter_turns: u8,
) -> Vec<Edit> {
    let mut are = are.clone();
    let mut git = git.clone();
    for _ in 0..quarter_turns % 4 {
        rotate_once(&mut are, &mut git);
    }
    let mut edits = Vec::new();
    for label in ["Width", "Height", "Tile_List"] {
        if let Some(v) = are.get(label) {
            edits.push(set_field(are_key, label, v.clone()));
        }
    }
    for kind in ObjectKind::ALL {
        if let Some(v) = git.get(kind.list()) {
            edits.push(set_field(git_key, kind.list(), v.clone()));
        }
    }
    edits
}

fn rotate_once(are: &mut Struct, git: &mut Struct) {
    let w = are.integer("Width").unwrap_or(0).max(0) as u32;
    let h = are.integer("Height").unwrap_or(0).max(0) as u32;
    let old = are.list("Tile_List").unwrap_or(&[]).to_vec();
    let mut list = old.clone();
    for y in 0..h {
        for x in 0..w {
            let Some(mut s) = old.get((y * w + x) as usize).cloned() else { continue };
            let o = s.integer("Tile_Orientation").unwrap_or(0);
            s.set("Tile_Orientation", Value::Int(((o + 1) & 3) as i32));
            let (nx, ny) = (h - 1 - y, x);
            if let Some(slot) = list.get_mut((ny * h + nx) as usize) {
                *slot = s;
            }
        }
    }
    are.set("Width", Value::Int(h as i32));
    are.set("Height", Value::Int(w as i32));
    are.set("Tile_List", Value::List(list));
    // Points turn about the origin and move east by the new width.
    let shift = h as f32 * TILE_SIZE;
    let point = |x: f32, y: f32| (shift - y, x);
    let float = |s: &Struct, l: &str| s.float(l).unwrap_or(0.0);
    for kind in ObjectKind::ALL {
        let Some(objects) = git.list_mut(kind.list()) else { continue };
        for s in objects {
            let [xl, yl, _] = mg_module::placement::position_fields(kind);
            let (x, y) = point(float(s, xl), float(s, yl));
            s.set(xl, Value::Float(x));
            s.set(yl, Value::Float(y));
            if s.contains("Bearing") {
                s.set("Bearing", Value::Float(turned(float(s, "Bearing"))));
            }
            if s.contains("XOrientation") && s.contains("YOrientation") {
                let (ox, oy) = (float(s, "XOrientation"), float(s, "YOrientation"));
                s.set("XOrientation", Value::Float(-oy));
                s.set("YOrientation", Value::Float(ox));
            }
            // A trigger's points are `PointX`/`PointY`, an encounter's `X`/`Y`.
            if let Some([pxl, pyl, _]) = mg_module::placement::point_fields(kind)
                && let Some(points) = s.list_mut("Geometry")
            {
                for p in points {
                    let (px, py) = (float(p, pxl), float(p, pyl));
                    p.set(pxl, Value::Float(-py));
                    p.set(pyl, Value::Float(px));
                }
            }
            if let Some(spawns) = s.list_mut("SpawnPointList") {
                for p in spawns {
                    let (x, y) = point(float(p, "X"), float(p, "Y"));
                    p.set("X", Value::Float(x));
                    p.set("Y", Value::Float(y));
                    if p.contains("Orientation") {
                        p.set("Orientation", Value::Float(turned(float(p, "Orientation"))));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One terrain; tile 0 is open ground, tiles 1 to 3 a group of 2 by 2
    /// whose first slot (the south-west one) is empty.
    const SET: &str = "[GENERAL]\nName=TST01\nInterior=0\nTransition=5\nDisplayName=-1\n\
Border=A\nDefault=A\nFloor=A\n[TERRAIN TYPES]\nCount=1\n[TERRAIN0]\nName=A\n\
[CROSSER TYPES]\nCount=0\n[PRIMARY RULES]\nCount=0\n[SECONDARY RULES]\nCount=0\n\
[TILES]\nCount=4\n\
[TILE0]\nModel=t0\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\n\
[TILE1]\nModel=t1\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\n\
[TILE2]\nModel=t2\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\n\
[TILE3]\nModel=t3\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\n\
[GROUPS]\nCount=1\n[GROUP0]\nName=L\nRows=2\nColumns=2\nTile0=-1\nTile1=1\nTile2=2\nTile3=3\n";

    /// An area `width` by `height` of the tiles `tile` gives each cell.
    fn area(width: u32, height: u32, tile: impl Fn(u32, u32) -> i32) -> Struct {
        let mut are = Struct::new(0);
        are.set("Width", Value::Int(width as i32));
        are.set("Height", Value::Int(height as i32));
        let cells = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
        let tiles = cells.map(|(x, y)| {
            let mut t = Struct::new(1);
            t.set("Tile_ID", Value::Int(tile(x, y)));
            t.set("Tile_Orientation", Value::Int(0));
            t.set("Tile_Height", Value::Int(0));
            t
        });
        are.set("Tile_List", Value::List(tiles.collect()));
        are
    }

    fn resized(are: &Struct, width: u32, height: u32) -> Option<Reshaped> {
        let set = Tileset::parse(SET.as_bytes(), mg_core::Codepage::WINDOWS_1252).unwrap();
        let index = TileIndex::new(&set);
        let key = |t| ResKey::parse("a", t).unwrap();
        let (are_key, git_key) = (key(mg_core::ResType::ARE), key(mg_core::ResType::GIT));
        let mut rng = fastrand::Rng::with_seed(1);
        let git = Struct::new(0);
        let lights = &mut || [0; 3];
        resize(are_key, git_key, are, &git, &set, &index, width, height, &mut rng, lights)
    }

    /// A group whose first slot is empty is a group all the same: one the
    /// new edge cuts leaves terrain tiles, not a part of itself (the copy
    /// of "which group stands here" this file had looked for a group's
    /// first tile alone).
    #[test]
    fn a_group_with_an_empty_first_slot_is_cut_whole() {
        // The group's tiles at (3, 1), (2, 2) and (3, 2) of a 4 by 4 area.
        let group = |x, y| match (x, y) {
            (3, 1) => 1,
            (2, 2) => 2,
            (3, 2) => 3,
            _ => 0,
        };
        let are = area(4, 4, group);
        let set = Tileset::parse(SET.as_bytes(), mg_core::Codepage::WINDOWS_1252).unwrap();
        let index = TileIndex::new(&set);
        let grid = crate::terrain::grid(&are, &index).unwrap();
        assert_eq!(groups_in(&index, &set, &grid), [(0, vec![(3, 1), (2, 2), (3, 2)])]);
        // Three wide: the column with two of its tiles goes, and the one
        // that stays, at (2, 2), is open ground.
        let r = resized(&are, 3, 4).unwrap();
        let Some(Edit::SetField { value: Some(Value::List(tiles)), .. }) = r
            .edits
            .iter()
            .find(|e| matches!(e, Edit::SetField { label, .. } if label == "Tile_List"))
        else {
            panic!("no tile list: {:?}", r.edits)
        };
        let ids: Vec<_> = tiles.iter().map(|t| t.integer("Tile_ID").unwrap()).collect();
        assert_eq!(ids, [0; 12]);
        // Left whole, it stays.
        let r = resized(&are, 5, 4).unwrap();
        let Some(Edit::SetField { value: Some(Value::List(tiles)), .. }) = r.edits.get(2) else {
            panic!("no tile list: {:?}", r.edits)
        };
        assert_eq!(tiles[2 * 5 + 2].integer("Tile_ID"), Some(2));
    }

    /// An ARE a tool wrote with no tiles has nothing to continue: no
    /// resize, and no panic (`0 - 1` in the old edge's place).
    #[test]
    fn an_area_of_no_tiles_is_not_resized() {
        assert_eq!(resized(&area(0, 0, |_, _| 0), 4, 4), None);
        assert_eq!(resized(&area(0, 3, |_, _| 0), 4, 4), None);
        assert_eq!(resized(&area(3, 0, |_, _| 0), 4, 4), None);
    }

    #[test]
    fn rotating_four_times_is_the_identity() {
        let mut are = Struct::new(0);
        are.set("Width", Value::Int(3));
        are.set("Height", Value::Int(2));
        let tiles: Vec<Struct> = (0..6)
            .map(|i| {
                let mut t = Struct::new(1);
                t.set("Tile_ID", Value::Int(i));
                t.set("Tile_Orientation", Value::Int(0));
                t
            })
            .collect();
        are.set("Tile_List", Value::List(tiles));
        let mut git = Struct::new(0);
        let mut door = Struct::new(8);
        door.set("X", Value::Float(25.0));
        door.set("Y", Value::Float(5.0));
        door.set("Bearing", Value::Float(0.0));
        git.set("Door List", Value::List(vec![door]));
        // An encounter 10 by 4, lying west to east: its points are `X`/`Y`.
        let mut encounter = Struct::new(7);
        encounter.set("XPosition", Value::Float(10.0));
        encounter.set("YPosition", Value::Float(5.0));
        let outline = [(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (0.0, 4.0)].map(|(x, y)| {
            let mut p = Struct::new(1);
            p.set("X", Value::Float(x));
            p.set("Y", Value::Float(y));
            p.set("Z", Value::Float(0.0));
            p
        });
        encounter.set("Geometry", Value::List(outline.to_vec()));
        git.set("Encounter List", Value::List(vec![encounter]));
        let (mut a, mut g) = (are.clone(), git.clone());
        rotate_once(&mut a, &mut g);
        // The encounter's outline turns with it, in its own fields.
        let e = &g.list("Encounter List").unwrap()[0];
        let points = e.list("Geometry").unwrap();
        assert!(points.iter().all(|p| !p.contains("PointX") && !p.contains("PointY")));
        assert_eq!((points[1].float("X"), points[1].float("Y")), (Some(0.0), Some(10.0)));
        assert_eq!((points[2].float("X"), points[2].float("Y")), (Some(-4.0), Some(10.0)));
        // 3 by 2 becomes 2 by 3; tile 0 at (0, 0) goes to (1, 0), turned.
        assert_eq!((a.integer("Width"), a.integer("Height")), (Some(2), Some(3)));
        let t = &a.list("Tile_List").unwrap()[1];
        assert_eq!((t.integer("Tile_ID"), t.integer("Tile_Orientation")), (Some(0), Some(1)));
        let d = &g.list("Door List").unwrap()[0];
        assert_eq!((d.float("X"), d.float("Y")), (Some(15.0), Some(25.0)));
        assert!((d.float("Bearing").unwrap() - FRAC_PI_2).abs() < 1e-6);
        for _ in 0..3 {
            rotate_once(&mut a, &mut g);
        }
        assert_eq!(a.list("Tile_List"), are.list("Tile_List"));
        let d = &g.list("Door List").unwrap()[0];
        assert!(
            (d.float("X").unwrap() - 25.0).abs() < 1e-4
                && (d.float("Y").unwrap() - 5.0).abs() < 1e-4
        );
        let e = &g.list("Encounter List").unwrap()[0];
        for (now, was) in e.list("Geometry").unwrap().iter().zip(&outline) {
            assert!(!now.contains("PointX"), "no field is added to a point");
            for l in ["X", "Y"] {
                assert!((now.float(l).unwrap() - was.float(l).unwrap()).abs() < 1e-4);
            }
        }
    }
}
