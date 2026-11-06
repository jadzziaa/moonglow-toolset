//! Where an area's tiles disagree: neighbors that give a corner or an edge
//! they share different terrain, height or crosser. Aurora's painting
//! leaves none but what a tileset's own group makes; an area written by
//! hand, by a script or by a tool can have any.

use crate::{Corner, Crosser, EAST, NE, NORTH, NW, Placement, SE, SOUTH, SW, TileIndex, WEST};

/// One thing two neighboring tiles disagree about, as each has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disagreement {
    /// Corner `corner` of the cell (from [`SW`] counter-clockwise): the
    /// cell's and its neighbor's.
    Corner { corner: usize, here: Corner, there: Corner },
    /// Edge `edge` of the cell (from [`SOUTH`] counter-clockwise), shared
    /// with its neighbor: its crosser in the cell's tile and in the
    /// neighbor's.
    Edge { edge: usize, here: Option<Crosser>, there: Option<Crosser> },
}

/// A cell and a neighbor to its east or north (or across a corner, where
/// the cells between are not the tileset's) that disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seam {
    pub cell: (u32, u32),
    pub neighbor: (u32, u32),
    pub what: Vec<Disagreement>,
}

/// An area's seams, the area's and its tileset's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seams {
    /// What the area's tiles could have agreed about.
    pub area: Vec<Seam>,
    /// What a placed group of the tileset makes, which no other tile
    /// could close: between two tiles of the group (ttu01's 299 and 300
    /// have rock on one tile's side and floor on the other's), and, on
    /// its outside, a corner the group's own tiles disagree about, where
    /// the neighbor agrees with one of them, and an edge whose crosser on
    /// the group's side is one only groups have (trs02's `path`, on tile
    /// 582 alone).
    pub groups: Vec<Seam>,
}

/// The seams of a `width` × `height` grid of placed tiles (row-major from
/// the south-west, as `Tile_List`), each pair of neighbors once, in that
/// order. A tile the tileset doesn't have, or a hole in it, has no
/// corners to compare: such a cell is passed over (and its neighbors
/// across a corner compared with each other instead), as are cells past
/// the end of a short list.
pub fn seams(index: &TileIndex, width: u32, height: u32, tiles: &[Placement]) -> Seams {
    let placed = |x: i64, y: i64| {
        let inside = x >= 0 && y >= 0 && x < i64::from(width) && y < i64::from(height);
        inside.then(|| tiles.get((y * i64::from(width) + x) as usize).copied()).flatten()
    };
    let cell = |x: u32, y: u32| {
        let p = placed(i64::from(x), i64::from(y))?;
        if index.is_hole(p.tile) {
            return None;
        }
        index.cell(p)
    };
    // A corner of the lattice as the cells of a group that touch it have
    // it: more than one value when they disagree.
    let group_has = |group: &[(u32, u32)], (u, v): (u32, u32)| {
        let mut values: Vec<Corner> = Vec::new();
        for &(cx, cy) in group {
            let k = match (u.wrapping_sub(cx), v.wrapping_sub(cy)) {
                (0, 0) => SW,
                (1, 0) => SE,
                (1, 1) => NE,
                (0, 1) => NW,
                _ => continue,
            };
            if let Some(c) = cell(cx, cy).map(|c| c.corners[k])
                && !values.contains(&c)
            {
                values.push(c);
            }
        }
        values
    };
    let mut out = Seams::default();
    for y in 0..height {
        for x in 0..width {
            let Some(here) = cell(x, y) else { continue };
            let mut compare = |neighbor: (u32, u32), corners: &[(usize, usize)], edge| {
                let Some(there) = cell(neighbor.0, neighbor.1) else { return };
                let mut what = Vec::new();
                for &(mine, theirs) in corners {
                    let (a, b) = (here.corners[mine], there.corners[theirs]);
                    if a != b {
                        what.push(Disagreement::Corner { corner: mine, here: a, there: b });
                    }
                }
                if let Some((mine, theirs)) = edge {
                    let (a, b) = (here.edges[mine], there.edges[theirs]);
                    if a != b {
                        what.push(Disagreement::Edge { edge: mine, here: a, there: b });
                    }
                }
                if what.is_empty() {
                    return;
                }
                let groups = [(x, y), neighbor].map(|(gx, gy)| index.group_at(&placed, gx, gy));
                let (mut area, mut tileset) = (Vec::new(), Vec::new());
                for w in what {
                    let ours = match w {
                        _ if groups[0].as_ref().is_some_and(|g| g.contains(&neighbor)) => false,
                        Disagreement::Corner { corner, here, there } => {
                            let at = [(x, y), (x + 1, y), (x + 1, y + 1), (x, y + 1)][corner];
                            // The group's tiles disagree there, and the
                            // other side agrees with one of them.
                            let made = |g: &Option<Vec<(u32, u32)>>, other: Corner| {
                                g.as_ref().is_some_and(|g| {
                                    let values = group_has(g, at);
                                    values.len() > 1 && values.contains(&other)
                                })
                            };
                            !(made(&groups[0], there) || made(&groups[1], here))
                        }
                        Disagreement::Edge { here, there, .. } => {
                            // The group's crosser is one only groups have.
                            let made = |g: &Option<Vec<(u32, u32)>>, c: Option<Crosser>| {
                                g.is_some() && c.is_some_and(|c| !index.is_free_crosser(c))
                            };
                            !(made(&groups[0], here) || made(&groups[1], there))
                        }
                    };
                    if ours { area.push(w) } else { tileset.push(w) }
                }
                for (list, what) in [(&mut out.area, area), (&mut out.groups, tileset)] {
                    if !what.is_empty() {
                        list.push(Seam { cell: (x, y), neighbor, what });
                    }
                }
            };
            compare((x + 1, y), &[(SE, SW), (NE, NW)], Some((EAST, WEST)));
            compare((x, y + 1), &[(NW, SW), (NE, SE)], Some((NORTH, SOUTH)));
            // Across a corner only where neither cell beside both is
            // compared with them.
            if cell(x + 1, y).is_none() && cell(x, y + 1).is_none() {
                compare((x + 1, y + 1), &[(NE, SW)], None);
            }
            if x > 0 && cell(x - 1, y).is_none() && cell(x, y + 1).is_none() {
                compare((x - 1, y + 1), &[(NW, SE)], None);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lattice, Terrain};
    use mg_core::Codepage;
    use mg_set::Tileset;

    /// Terrains A and B, a crosser Road; tiles: all A, one B corner
    /// (bottom left), all A with a road from top to bottom, tile 3 a hole
    /// (no model), all B, and a group of two tiles side by side that are
    /// both A on the left and B on the right, so disagree with each other
    /// (as ttu01's group of tiles 299 and 300 does), the second with a
    /// crosser Path on its right that no other tile has (as trs02's
    /// `path`), and B along the bottom.
    const SET: &str = "[GENERAL]\nName=TST02\nInterior=0\nTransition=5\nDisplayName=-1\nBorder=A\nDefault=A\nFloor=A\n\
[TERRAIN TYPES]\nCount=2\n[TERRAIN0]\nName=A\n[TERRAIN1]\nName=B\n[CROSSER TYPES]\nCount=1\n[CROSSER0]\nName=Road\n\
[PRIMARY RULES]\nCount=0\n[SECONDARY RULES]\nCount=0\n\
[TILES]\nCount=8\n\
[TILE0]\nModel=t0\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\n\
[TILE1]\nModel=t1\nTopLeft=A\nTopRight=A\nBottomLeft=B\nBottomRight=A\n\
[TILE2]\nModel=t2\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\nTop=Road\nBottom=Road\n\
[TILE3]\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\n\
[TILE4]\nModel=t4\nTopLeft=B\nTopRight=B\nBottomLeft=B\nBottomRight=B\n\
[TILE5]\nModel=g0\nTopLeft=A\nTopRight=B\nBottomLeft=A\nBottomRight=B\n\
[TILE6]\nModel=g1\nTopLeft=A\nTopRight=B\nBottomLeft=A\nBottomRight=B\nRight=Path\n\
[TILE7]\nModel=t7\nTopLeft=A\nTopRight=A\nBottomLeft=B\nBottomRight=B\n\
[GROUPS]\nCount=1\n[GROUP0]\nName=G\nRows=1\nColumns=2\nTile0=5\nTile1=6\n";

    fn index() -> TileIndex {
        TileIndex::new(&Tileset::parse(SET.as_bytes(), Codepage::WINDOWS_1252).unwrap())
    }

    fn at(tile: u32, orientation: u8, height: i32) -> Placement {
        Placement { tile, orientation, height }
    }

    #[test]
    fn tiles_that_fit_have_no_seams() {
        let index = index();
        // Tile 1's B corner turned to meet at corner (1, 1): SW of (1, 1),
        // SE of (0, 1) (one turn), NE of (0, 0) (two), NW of (1, 0) (three).
        let tiles = [at(1, 2, 0), at(1, 3, 0), at(1, 1, 0), at(1, 0, 0)];
        assert!(seams(&index, 2, 2, &tiles).area.is_empty());
        let (_, bad) = Lattice::from_tiles(&index, 2, 2, &tiles).unwrap();
        assert!(bad.is_empty());
        // A road running north through two tiles.
        assert!(seams(&index, 1, 2, &[at(2, 0, 0), at(2, 2, 0)]).area.is_empty());
    }

    #[test]
    fn a_seam_names_the_corner_the_edge_and_the_neighbor() {
        let index = index();
        let (a, b) = (index.terrain("A").unwrap(), index.terrain("B").unwrap());
        let road = index.crosser("Road").unwrap();
        // (0, 0) puts B at its north-east corner; (1, 0) has A all round.
        let found = seams(&index, 2, 1, &[at(1, 2, 0), at(0, 0, 0)]).area;
        let corner = |terrain: Terrain, height| Corner { terrain, height };
        assert_eq!(
            found,
            [Seam {
                cell: (0, 0),
                neighbor: (1, 0),
                what: vec![Disagreement::Corner {
                    corner: NE,
                    here: corner(b, 0),
                    there: corner(a, 0)
                }],
            }]
        );
        // A tile raised a step beside one that isn't: both shared corners.
        let found = seams(&index, 1, 2, &[at(0, 0, 0), at(0, 0, 1)]).area;
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].cell, found[0].neighbor), ((0, 0), (0, 1)));
        assert_eq!(
            found[0].what,
            [
                Disagreement::Corner { corner: NW, here: corner(a, 0), there: corner(a, 1) },
                Disagreement::Corner { corner: NE, here: corner(a, 0), there: corner(a, 1) },
            ]
        );
        // A road that ends at a tile without one.
        let found = seams(&index, 1, 2, &[at(2, 0, 0), at(0, 0, 0)]).area;
        assert_eq!(
            found[0].what,
            [Disagreement::Edge { edge: NORTH, here: Some(road), there: None }]
        );
    }

    /// A tile the tileset lacks, or a hole, is someone else's to report:
    /// its neighbors are compared across it only where they share a corner.
    #[test]
    fn unknown_tiles_and_holes_are_passed_over() {
        let index = index();
        let b_ne = at(1, 2, 0);
        assert!(seams(&index, 2, 1, &[b_ne, at(99, 0, 0)]).area.is_empty());
        assert!(seams(&index, 2, 1, &[b_ne, at(3, 0, 0)]).area.is_empty());
        // (0, 0)'s B corner is (1, 1)'s south-west, which has A; the cells
        // between are a hole and a tile not in the tileset.
        let tiles = [b_ne, at(3, 0, 0), at(99, 0, 0), at(0, 0, 0)];
        let found = seams(&index, 2, 2, &tiles).area;
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].cell, found[0].neighbor), ((0, 0), (1, 1)));
        // And the other diagonal: (1, 0)'s north-west (B) against (0, 1)'s
        // south-east.
        let tiles = [at(99, 0, 0), at(1, 3, 0), at(0, 0, 0), at(3, 0, 0)];
        let found = seams(&index, 2, 2, &tiles).area;
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].cell, found[0].neighbor), ((1, 0), (0, 1)));
        // A short list: the cells past it are not compared.
        assert!(seams(&index, 2, 2, &[b_ne]).area.is_empty());
    }

    /// A group whose own tiles disagree: the seam between them, and the
    /// one its outside can't help having, are the tileset's. Stamped
    /// beside the wrong terrain, its outside is the area's.
    #[test]
    fn a_group_s_own_seams_are_the_tileset_s() {
        let index = index();
        // Row 0: all A, the group (A | B, A | B), all B. Row 1 agrees with
        // the group's first tile at corner (2, 1): B, where its second
        // tile has A. Path, on the group's right, meets nothing.
        let mut tiles = vec![at(0, 0, 0), at(5, 0, 0), at(6, 0, 0), at(4, 0, 0)];
        tiles.extend([at(0, 0, 0), at(1, 1, 0), at(7, 0, 0), at(7, 0, 0)]);
        let found = seams(&index, 4, 2, &tiles);
        assert!(found.area.is_empty(), "{:?}", found.area);
        let pairs: Vec<_> = found.groups.iter().map(|s| (s.cell, s.neighbor)).collect();
        assert_eq!(pairs, [((1, 0), (2, 0)), ((2, 0), (3, 0)), ((2, 0), (2, 1))]);
        let (road, path) = (index.crosser("Road").unwrap(), index.crosser("Path").unwrap());
        assert!(index.is_free_crosser(road) && !index.is_free_crosser(path));
        // A neighbor that agrees with neither (a step up) is the area's.
        let mut odd = tiles.clone();
        odd[6] = at(7, 0, 1);
        let found = seams(&index, 4, 2, &odd);
        let at_2_1 = found.area.iter().find(|s| (s.cell, s.neighbor) == ((2, 0), (2, 1)));
        assert!(
            at_2_1
                .unwrap()
                .what
                .iter()
                .any(|w| matches!(w, Disagreement::Corner { corner: NW, .. })),
            "{:?}",
            found.area
        );
        // The group stamped beside A on its B side.
        tiles[3] = at(0, 0, 0);
        let found = seams(&index, 4, 2, &tiles);
        assert_eq!(found.area[0].cell, (2, 0));
        assert_eq!(found.area[0].neighbor, (3, 0));
        assert_eq!(found.area[0].what.len(), 2, "the corners, not the path");
        // A group's tile without the rest of its group is any tile.
        let lone = [at(0, 0, 0), at(5, 0, 0), at(0, 0, 0)];
        let found = seams(&index, 3, 1, &lone);
        assert_eq!((found.area.len(), found.groups.len()), (1, 0));
    }
}
