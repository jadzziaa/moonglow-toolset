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
use mg_tiles::{Lattice, Placement, TileIndex};

use crate::{ObjectKind, TILE_SIZE};

/// What resizing or rotating does: the edits, and how many objects it
/// deleted.
#[derive(Debug, Clone, PartialEq)]
pub struct Reshaped {
    pub edits: Vec<Edit>,
    pub deleted: usize,
}

/// The placed groups in a grid: the group (index in the tileset) and the
/// cells of its tiles.
pub fn groups_in(index: &TileIndex, set: &Tileset, grid: &Grid) -> Vec<(usize, Vec<(u32, u32)>)> {
    let (w, h) = (grid.lattice.width() as i64, grid.lattice.height() as i64);
    let mut out: Vec<(usize, Vec<(u32, u32)>)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let p = grid.tile(x as u32, y as u32);
            if !index.is_grouped(p.tile) {
                continue;
            }
            'groups: for (g, group) in set.groups.iter().enumerate() {
                if group.tiles.first().copied().flatten() != Some(p.tile) {
                    continue;
                }
                let columns = group.columns.max(1) as i64;
                let mut cells = Vec::new();
                for (k, tile) in group.tiles.iter().enumerate() {
                    let (mut c, mut r) = (k as i64 % columns, k as i64 / columns);
                    for _ in 0..p.orientation {
                        (c, r) = (-r, c);
                    }
                    let (cx, cy) = (x + c, y + r);
                    let Some(t) = tile else { continue };
                    if cx < 0 || cy < 0 || cx >= w || cy >= h {
                        continue 'groups;
                    }
                    let here = grid.tile(cx as u32, cy as u32);
                    if here.tile != *t || here.orientation != p.orientation {
                        continue 'groups;
                    }
                    cells.push((cx as u32, cy as u32));
                }
                if !out.iter().any(|(_, c)| *c == cells) {
                    out.push((g, cells));
                }
            }
        }
    }
    out
}

/// A new tile's struct, as the area wizard writes one.
fn tile_struct(set: &Tileset, p: Placement, lights: [u8; 3]) -> Struct {
    let loops = set.tiles.get(p.tile as usize).map_or([false; 3], |t| t.anim_loops);
    let mut s = Struct::new(1);
    s.set("Tile_ID", Value::Int(p.tile as i32));
    s.set("Tile_Orientation", Value::Int(i32::from(p.orientation)));
    s.set("Tile_Height", Value::Int(p.height));
    s.set("Tile_MainLight1", Value::Byte(lights[0]));
    s.set("Tile_MainLight2", Value::Byte(lights[1]));
    s.set("Tile_SrcLight1", Value::Byte(lights[2]));
    s.set("Tile_SrcLight2", Value::Byte(lights[2]));
    for (k, on) in loops.into_iter().enumerate() {
        s.set(&format!("Tile_AnimLoop{}", k + 1), Value::Byte(u8::from(on)));
    }
    s
}

/// Where an object stands (its position fields).
fn position(kind: ObjectKind, s: &Struct) -> (f32, f32) {
    if kind.has_bearing() {
        (s.float("X").unwrap_or(0.0), s.float("Y").unwrap_or(0.0))
    } else {
        (s.float("XPosition").unwrap_or(0.0), s.float("YPosition").unwrap_or(0.0))
    }
}

/// Resizing the area (its ARE `are`, GIT `git`) to `width` by `height`
/// tiles; `lights` gives each new tile's lights. `None` when the size is
/// outside Aurora's 2 to 32 or the area's tiles are not the tileset's.
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
            list.push(tile_struct(set, p, lights()));
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
        .flat_map(|&(x, y)| {
            let p = old.tile(x, y);
            let tile = set.tiles.get(p.tile as usize);
            let centre =
                glam::Vec3::new((x as f32 + 0.5) * TILE_SIZE, (y as f32 + 0.5) * TILE_SIZE, 0.0);
            let turn = glam::Quat::from_rotation_z(f32::from(p.orientation) * FRAC_PI_2);
            tile.map(|t| t.doors.clone())
                .unwrap_or_default()
                .into_iter()
                .filter(|d| d.door_type != 0)
                .map(move |d| {
                    let at = centre + turn * glam::Vec3::from_array(d.position);
                    (at.x, at.y)
                })
        })
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
            let (xl, yl) = if kind.has_bearing() { ("X", "Y") } else { ("XPosition", "YPosition") };
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
            if let Some(points) = s.list_mut("Geometry") {
                for p in points {
                    let (px, py) = (float(p, "PointX"), float(p, "PointY"));
                    p.set("PointX", Value::Float(-py));
                    p.set("PointY", Value::Float(px));
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
        let (mut a, mut g) = (are.clone(), git.clone());
        rotate_once(&mut a, &mut g);
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
    }
}
