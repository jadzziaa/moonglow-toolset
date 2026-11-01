//! The tile border: what the game client draws past an area's edge
//! (Options' "Enable Tile Border", on by default; a module turns it off
//! with `SetAreaTileBorderDisabled`), read from the client in
//! `docs/research/notes_tile_border.md`.
//!
//! Each of the area's edge tiles is continued outward, [`DEPTH`] tiles
//! deep, by one of the tileset's edge tiles (`<tileset>_edge.2da`): the
//! row whose `Corner1`, `Edge` and `Corner2` are the tile's two corners on
//! that side (left and right, looking out; a raised corner's terrain with
//! a `+`) and its crosser there, the same model in each of the five cells,
//! turned as the side is. Past the area's corners, a square of five by
//! five holds the edge tile of the corner's terrain alone (`Corner2` and
//! `Edge` empty). The client draws them lit by the area's light, their
//! material's ambient and diffuse half the sun's diffuse color (gamma
//! space): [`material`].

use glam::{Mat4, Quat, Vec3};
use mg_set::Tileset;
use mg_tiles::{NE, NW, SE, SW};

use crate::{AreaTile, TILE_SIZE};

/// How many tiles deep the border is.
pub const DEPTH: u32 = 5;

/// One of the border's tiles: its model (lower case) and where it lies.
#[derive(Debug, Clone, PartialEq)]
pub struct BorderTile {
    pub model: String,
    pub transform: Mat4,
}

/// A row of a tileset's edge table (`<tileset>_edge.2da`): terrains and
/// crosser lower case, `None` for `****`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRow {
    pub corner1: Option<String>,
    pub edge: Option<String>,
    pub corner2: Option<String>,
    /// In the tileset's height steps.
    pub height: i32,
    pub model: String,
}

/// The rows of an edge table that name a model.
pub fn edge_rows(table: &mg_2da::TwoDa) -> Vec<EdgeRow> {
    let cell = |row: usize, column: &str| table.get(row, column).map(str::to_ascii_lowercase);
    (0..table.len())
        .filter_map(|row| {
            Some(EdgeRow {
                corner1: cell(row, "Corner1"),
                edge: cell(row, "Edge"),
                corner2: cell(row, "Corner2"),
                height: table.get_int(row, "Height").unwrap_or(0),
                model: cell(row, "Model")?,
            })
        })
        .collect()
}

/// The material color the client gives the border's meshes, as their
/// ambient and diffuse (linear): half the area's sun's (or moon's)
/// diffuse color in gamma space, `diffuse` linear (read back from the
/// client's `materialFrontAmbient` and `materialFrontDiffuse`: 55, 30, 12
/// and 3 of 255 for a diffuse of FF, C0, 80 and 40).
pub fn material(diffuse: Vec3) -> Vec3 {
    diffuse * 0.5f32.powf(2.2)
}

/// A side's corner terrain, named as the edge table names it: `+` for a
/// raised corner (a level of 1 or more).
fn terrain(name: &str, level: i32) -> String {
    let name = name.to_ascii_lowercase();
    if level > 0 { format!("{name}+") } else { name }
}

/// The edge table's row for a side whose corners are `left` and `right`
/// (looking out of the area) with crosser `edge`, and whether it is
/// listed the other way round (its model then turned half a turn): as
/// listed, else the other way round; then without the crosser.
fn lookup<'a>(
    rows: &'a [EdgeRow],
    left: &str,
    edge: Option<&str>,
    right: &str,
) -> Option<(&'a EdgeRow, bool)> {
    let is = |c: &Option<String>, name: &str| c.as_deref() == Some(name);
    let with = |edge: Option<&str>| {
        let crosses = |r: &EdgeRow| r.edge.as_deref() == edge;
        (rows.iter().find(|r| is(&r.corner1, left) && crosses(r) && is(&r.corner2, right)))
            .map(|r| (r, false))
            .or_else(|| {
                rows.iter()
                    .find(|r| is(&r.corner1, right) && crosses(r) && is(&r.corner2, left))
                    .map(|r| (r, true))
            })
    };
    with(edge).or_else(|| edge.and_then(|_| with(None)))
}

/// The edge table's row for the corner square of terrain `corner`.
fn corner_row<'a>(rows: &'a [EdgeRow], corner: &str) -> Option<&'a EdgeRow> {
    rows.iter()
        .find(|r| r.corner1.as_deref() == Some(corner) && r.edge.is_none() && r.corner2.is_none())
}

/// A tile's corners, each its terrain and level, counter-clockwise from
/// the south-west.
type Corners = [(String, i32); 4];
/// A tile's edges' crossers (lower case), counter-clockwise from the south.
type Crossers = [Option<String>; 4];
/// A side of the area: its index (counter-clockwise from the south), its
/// edge tiles, the step outward and the turn of its edge tiles.
type Side = (usize, Vec<(u32, u32)>, Step, u32);
/// A step from a cell to the next, along X and Y.
type Step = (i64, i64);

/// A tile's corners (terrain and level: the tile's height and the
/// corner's) and edges' crossers, turned as it lies, counter-clockwise
/// from the south-west and the south ([`mg_tiles::SW`], [`mg_tiles::SOUTH`]).
fn sides(set: &Tileset, tile: &AreaTile) -> Option<(Corners, Crossers)> {
    let t = set.tiles.get(usize::try_from(tile.id).ok()?)?;
    // (The set lists corners TopLeft, TopRight, BottomLeft, BottomRight
    // and edges Top, Right, Bottom, Left.)
    const CORNERS: [usize; 4] = [NW, NE, SW, SE];
    const EDGES: [usize; 4] = [2, 1, 0, 3];
    let mut corners: Corners = Default::default();
    let mut edges: Crossers = Default::default();
    let q = usize::from(tile.orientation % 4);
    for (k, (name, height)) in t.corners.iter().enumerate() {
        corners[(CORNERS[k] + q) % 4] = (name.clone(), tile.height + height);
    }
    for (k, crosser) in t.edges.iter().enumerate() {
        let crosser = (!crosser.is_empty()).then(|| crosser.to_ascii_lowercase());
        edges[(EDGES[k] + q) % 4] = crosser;
    }
    Some((corners, edges))
}

/// The border of an area `width` by `height` tiles of `set`, whose tiles
/// are `tiles` (row by row from the south-west), from its edge table's
/// `rows`; `step` the tileset's height step, metres.
pub fn border(
    set: &Tileset,
    rows: &[EdgeRow],
    tiles: &[AreaTile],
    (width, height): (u32, u32),
    step: f32,
) -> Vec<BorderTile> {
    let mut out = Vec::new();
    if width == 0 || height == 0 || tiles.len() < (width * height) as usize {
        return out;
    }
    let tile = |x: u32, y: u32| &tiles[(y * width + x) as usize];
    let place = |model: &str, (cx, cy): (i64, i64), level: i32, turns: u32| BorderTile {
        model: model.to_string(),
        transform: Mat4::from_rotation_translation(
            Quat::from_rotation_z(turns as f32 * std::f32::consts::FRAC_PI_2),
            Vec3::new(
                (cx as f32 + 0.5) * TILE_SIZE,
                (cy as f32 + 0.5) * TILE_SIZE,
                level as f32 * step,
            ),
        ),
    };
    // The sides, south, east, north and west: each tile along it, the
    // cell beyond it, the way out, and the side's turn (the edge tiles
    // are made for the east side: `Corner1` north, `Corner2` south;
    // read from the client's pictures, `reading_tile_border`).
    let sides_of: [Side; 4] = [
        (0, (0..width).map(|x| (x, 0)).collect(), (0, -1), 3),
        (1, (0..height).map(|y| (width - 1, y)).collect(), (1, 0), 0),
        (2, (0..width).map(|x| (x, height - 1)).collect(), (0, 1), 1),
        (3, (0..height).map(|y| (0, y)).collect(), (-1, 0), 2),
    ];
    for (side, cells, (dx, dy), turns) in sides_of {
        for (x, y) in cells {
            let Some((corners, edges)) = sides(set, tile(x, y)) else { continue };
            // (Counter-clockwise, a side's corners are its right and then
            // its left, looking out.)
            let (right, left) = (&corners[side], &corners[(side + 1) % 4]);
            let names = (terrain(&left.0, left.1), terrain(&right.0, right.1));
            let Some((row, reversed)) = lookup(rows, &names.0, edges[side].as_deref(), &names.1)
            else {
                continue;
            };
            let level = row.height.max(left.1.min(right.1));
            let turns = if reversed { turns + 2 } else { turns } % 4;
            for d in 1..=i64::from(DEPTH) {
                let at = (i64::from(x) + dx * d, i64::from(y) + dy * d);
                out.push(place(&row.model, at, level, turns));
            }
        }
    }
    // The corners' squares: the south-west, south-east, north-east and
    // north-west, by the corner tile's corner there.
    let squares = [
        ((0, 0), SW, (-1, -1), 3),
        ((width - 1, 0), SE, (1, -1), 0),
        ((width - 1, height - 1), NE, (1, 1), 1),
        ((0, height - 1), NW, (-1, 1), 2),
    ];
    for ((x, y), corner, (dx, dy), turns) in squares {
        let Some((corners, _)) = sides(set, tile(x, y)) else { continue };
        let (name, level) = &corners[corner];
        let Some(row) = corner_row(rows, &terrain(name, *level)) else { continue };
        let level = row.height.max(*level);
        for i in 1..=i64::from(DEPTH) {
            for j in 1..=i64::from(DEPTH) {
                let at = (i64::from(x) + dx * i, i64::from(y) + dy * j);
                out.push(place(&row.model, at, level, turns));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::Codepage;

    /// Grass and water, a road and a stream; tiles: all grass, water on
    /// the east side, a road out of the top, grass raised along the top, a
    /// stream out of the top.
    const SET: &str = "[GENERAL]\nName=TST01\nInterior=0\nTransition=5\nDisplayName=-1\nBorder=Grass\nDefault=Grass\nFloor=Grass\n\
[TERRAIN TYPES]\nCount=2\n[TERRAIN0]\nName=Grass\n[TERRAIN1]\nName=Water\n\
[CROSSER TYPES]\nCount=2\n[CROSSER0]\nName=Road\n[CROSSER1]\nName=Stream\n\
[PRIMARY RULES]\nCount=0\n[SECONDARY RULES]\nCount=0\n\
[TILES]\nCount=5\n\
[TILE0]\nModel=t0\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\n\
[TILE1]\nModel=t1\nTopLeft=Grass\nTopRight=Water\nBottomLeft=Grass\nBottomRight=Water\n\
[TILE2]\nModel=t2\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\nTop=Road\n\
[TILE3]\nModel=t3\nTopLeft=Grass\nTopLeftHeight=1\nTopRight=Grass\nTopRightHeight=1\nBottomLeft=Grass\nBottomRight=Grass\n\
[TILE4]\nModel=t4\nTopLeft=Grass\nTopRight=Grass\nBottomLeft=Grass\nBottomRight=Grass\nTop=Stream\n";

    /// As the game's edge tables are: one row for a pair of terrains.
    const EDGE: &str = "2DA V2.0\n\n   Corner1 Edge Corner2 Height Model\n\
0  Grass  **** Grass  0 EG\n\
1  Grass  **** Water  0 EGW\n\
2  Grass  Road Grass  0 ER\n\
3  Grass+ **** Grass+ 1 EH\n\
4  Grass  **** ****   0 CG\n\
5  Water  **** ****   0 CW\n";

    fn rows() -> Vec<EdgeRow> {
        edge_rows(&mg_2da::TwoDa::parse(EDGE.as_bytes(), Codepage::WINDOWS_1252).unwrap())
    }

    /// A 2 by 2 area of tile 0 but for `tiles` (column, row, tile, turns).
    fn area(tiles: &[(u32, u32, i64, u8)]) -> Vec<AreaTile> {
        (0..4)
            .map(|index| {
                let (column, row) = (index as u32 % 2, index as u32 / 2);
                let (id, orientation) = (tiles.iter())
                    .find(|t| (t.0, t.1) == (column, row))
                    .map_or((0, 0), |t| (t.2, t.3));
                AreaTile {
                    index,
                    column,
                    row,
                    id,
                    model: None,
                    height: 0,
                    orientation,
                    main_lights: [0; 2],
                    source_lights: [0; 2],
                    anim_loops: [false; 3],
                    replace_texture: None,
                    position: Vec3::ZERO,
                }
            })
            .collect()
    }

    fn border_of(tiles: &[(u32, u32, i64, u8)]) -> Vec<BorderTile> {
        let set = Tileset::parse(SET.as_bytes(), Codepage::WINDOWS_1252).unwrap();
        border(&set, &rows(), &area(tiles), (2, 2), 5.0)
    }

    /// The border tile whose middle is in cell (`x`, `y`): its model, its
    /// quarter turns and its height.
    fn at(border: &[BorderTile], x: i32, y: i32) -> Option<(String, u32, f32)> {
        let middle = Vec3::new((x as f32 + 0.5) * TILE_SIZE, (y as f32 + 0.5) * TILE_SIZE, 0.0);
        let here = |b: &&BorderTile| (b.transform.w_axis.truncate() - middle).truncate().length();
        border.iter().find(|b| here(b) < 0.01).map(|b| {
            let x = b.transform.x_axis;
            let turns = (x.y.atan2(x.x) / std::f32::consts::FRAC_PI_2).round() as i32;
            (b.model.clone(), turns.rem_euclid(4) as u32, b.transform.w_axis.z)
        })
    }

    #[test]
    fn each_side_goes_out_five_deep_and_each_corner_five_by_five() {
        let b = border_of(&[]);
        assert_eq!(b.len(), 4 * 2 * 5 + 4 * 25);
        // The north side turned a quarter (the edge tiles are made for the
        // east side), the east side as made, the south and the west.
        assert_eq!(at(&b, 0, 2), Some(("eg".into(), 1, 0.0)));
        assert_eq!(at(&b, 1, 6), Some(("eg".into(), 1, 0.0)));
        assert_eq!(at(&b, 1, 7), None, "five deep");
        assert_eq!(at(&b, 2, 0), Some(("eg".into(), 0, 0.0)));
        assert_eq!(at(&b, 0, -1), Some(("eg".into(), 3, 0.0)));
        assert_eq!(at(&b, -5, 1), Some(("eg".into(), 2, 0.0)));
        // The corners' squares, of the corner's terrain.
        assert_eq!(at(&b, 2, 2).map(|t| t.0), Some("cg".into()));
        assert_eq!(at(&b, -5, -5).map(|t| t.0), Some("cg".into()));
        assert_eq!(at(&b, 6, 6).map(|t| t.0), Some("cg".into()));
        assert_eq!(at(&b, 7, 0), None, "five deep");
    }

    #[test]
    fn a_side_listed_the_other_way_round_is_turned_half_a_turn() {
        // Water east on the north-east tile: grass left, water right,
        // looking out north, as listed.
        let b = border_of(&[(1, 1, 1, 0)]);
        assert_eq!(at(&b, 1, 3), Some(("egw".into(), 1, 0.0)));
        // Its east side is all water, which has no row: nothing.
        assert_eq!(at(&b, 2, 1), None);
        // The north-east corner's square: water.
        assert_eq!(at(&b, 3, 3).map(|t| t.0), Some("cw".into()));
        // Turned half a turn on the north-west tile: water left, grass
        // right, the row the other way round.
        let b = border_of(&[(0, 1, 1, 2)]);
        assert_eq!(at(&b, 0, 4), Some(("egw".into(), 3, 0.0)));
        assert_eq!(at(&b, -1, 1), None, "its west side, all water");
        assert_eq!(at(&b, -1, 2).map(|t| t.0), Some("cw".into()), "its corner's square");
    }

    #[test]
    fn a_crosser_and_a_raised_side_take_their_rows() {
        // A road out of the north side: its row; a stream, which has
        // none, the row without a crosser.
        let b = border_of(&[(0, 1, 2, 0), (1, 1, 4, 0)]);
        assert_eq!(at(&b, 0, 2).map(|t| t.0), Some("er".into()));
        assert_eq!(at(&b, 1, 2).map(|t| t.0), Some("eg".into()));
        // A road turned to the east, on the north-east tile.
        let b = border_of(&[(1, 1, 2, 3)]);
        assert_eq!(at(&b, 2, 1).map(|t| t.0), Some("er".into()));
        assert_eq!(at(&b, 1, 2).map(|t| t.0), Some("eg".into()));
        // Raised corners: `Grass+`, at its height (a step of 5 m).
        let b = border_of(&[(0, 1, 3, 0)]);
        assert_eq!(at(&b, 0, 4), Some(("eh".into(), 1, 5.0)));
        // Its corner's square: the raised corner's terrain has no row.
        assert_eq!(at(&b, -1, 2), None);
    }

    #[test]
    fn the_border_s_material_is_half_the_sun_s_diffuse() {
        // (Read from the client: 55 of 255 for a diffuse of FF, 30 for C0.)
        let lin = |c: f32| (c / 255.0).powf(2.2);
        assert!((material(Vec3::splat(1.0)).x * 255.0 - 55.0).abs() < 1.0);
        assert!((material(Vec3::splat(lin(192.0))).x * 255.0 - 30.0).abs() < 1.0);
    }
}
