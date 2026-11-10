//! Area tile grids.
//!
//! An area is a grid of tiles over a lattice of corners. Every corner has a
//! terrain and a height; every cell edge may carry a crosser (a road, a
//! stream). A tile fits a cell when its corners and edges, turned to its
//! orientation, equal the cell's.
//!
//! Conventions follow ARE files: tile (x, y) is `Tile_List[y * width + x]`
//! with y = 0 the south row, and orientation `n` turns the tile `n` quarter
//! turns counter-clockwise.

use std::collections::HashMap;

use mg_set::Tileset;
use thiserror::Error;

pub mod paint;
mod seams;

pub use seams::{Disagreement, Seam, seams};

/// A terrain of a tileset (see [`TileIndex::terrain`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Terrain(pub u16);

/// A crosser of a tileset (see [`TileIndex::crosser`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Crosser(pub u16);

/// A lattice corner: terrain and height (in the tileset's height steps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Corner {
    pub terrain: Terrain,
    pub height: i32,
}

/// Corner indices of a [`Cell`], counter-clockwise from the south-west.
pub const SW: usize = 0;
pub const SE: usize = 1;
pub const NE: usize = 2;
pub const NW: usize = 3;
/// Edge indices of a [`Cell`], counter-clockwise from the south.
pub const SOUTH: usize = 0;
pub const EAST: usize = 1;
pub const NORTH: usize = 2;
pub const WEST: usize = 3;

/// What a cell holds: its corners (from [`SW`]) and edges (from [`SOUTH`]),
/// counter-clockwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub corners: [Corner; 4],
    pub edges: [Option<Crosser>; 4],
}

/// Where slot `k` of a group `columns` wide lies from the group's first
/// cell, with the group turned `quarter_turns` (columns, rows).
pub fn group_slot(k: usize, columns: u32, quarter_turns: u8) -> (i64, i64) {
    let columns = columns.max(1);
    let (mut c, mut r) = ((k as u32 % columns) as i64, (k as u32 / columns) as i64);
    for _ in 0..quarter_turns % 4 {
        (c, r) = (-r, c);
    }
    (c, r)
}

impl Cell {
    /// The cell turned `quarter_turns` counter-clockwise.
    pub fn turned(self, quarter_turns: u8) -> Cell {
        let q = usize::from(quarter_turns % 4);
        let mut out = self;
        for i in 0..4 {
            out.corners[(i + q) % 4] = self.corners[i];
            out.edges[(i + q) % 4] = self.edges[i];
        }
        out
    }
}

/// A tile at an orientation, raised by `height` steps (`Tile_Height`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Placement {
    pub tile: u32,
    pub orientation: u8,
    pub height: i32,
}

/// A tileset's tiles, indexed by the cells they fit.
#[derive(Debug, Clone)]
pub struct TileIndex {
    terrains: Vec<String>,
    crossers: Vec<String>,
    cells: Vec<Cell>,
    grouped: Vec<bool>,
    /// Tiles with no section or no model (`mg_set::Tile::is_hole`).
    holes: Vec<bool>,
    /// The tileset's groups: columns, and row-major tiles.
    groups: Vec<(u32, Vec<Option<u32>>)>,
    fits: HashMap<Cell, Vec<Placement>>,
    /// The highest corner height any tile has.
    max_height: i32,
}

/// The most height steps a corner is read as being up or down: a tileset's
/// corner heights and an area's `Tile_Height` are whatever their files
/// say, and work that goes a step at a time (a tile fitted at each height,
/// Change Tileset raising a corner level by level) is bounded by it. (The
/// area view holds `Tile_Height` to as much.)
pub const MAX_HEIGHT: i32 = 1024;

/// Tile corners are listed TopLeft, TopRight, BottomLeft, BottomRight
/// (`mg_set::CORNERS`); these are their positions counter-clockwise from SW.
const SET_CORNERS: [usize; 4] = [2, 3, 1, 0];
/// Tile edges are listed Top, Right, Bottom, Left (`mg_set::EDGES`).
const SET_EDGES: [usize; 4] = [2, 1, 0, 3];

/// A tileset's tile as it lies turned `quarter_turns`, not raised: its
/// corners (terrain and height) counter-clockwise from the south-west
/// ([`SW`]) and its edges' crossers ("": none) counter-clockwise from the
/// south ([`SOUTH`]), as the tileset names them. The one place a `.set`'s
/// order of corners and edges becomes the grid's: the index is made of
/// it, and so is the tile border.
pub fn tile_sides(tile: &mg_set::Tile, quarter_turns: u8) -> ([(&str, i32); 4], [&str; 4]) {
    let q = usize::from(quarter_turns % 4);
    let (mut corners, mut edges) = ([("", 0); 4], [""; 4]);
    for i in 0..4 {
        let (name, height) = &tile.corners[SET_CORNERS[i]];
        corners[(i + q) % 4] = (name.as_str(), *height);
        edges[(i + q) % 4] = tile.edges[SET_EDGES[i]].as_str();
    }
    (corners, edges)
}

fn intern(names: &mut Vec<String>, name: &str) -> u16 {
    match names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
        Some(i) => i as u16,
        None => {
            names.push(name.to_string());
            (names.len() - 1) as u16
        }
    }
}

impl TileIndex {
    /// Indexes a tileset's tiles. Terrain and crosser names match
    /// case-insensitively; names that tiles use without declaring them (as
    /// some shipped tilesets do) are indexed too.
    pub fn new(set: &Tileset) -> TileIndex {
        let mut terrains = Vec::new();
        let mut crossers = Vec::new();
        for t in &set.terrains {
            intern(&mut terrains, &t.name);
        }
        for c in &set.crossers {
            intern(&mut crossers, &c.name);
        }
        let cells: Vec<Cell> = set
            .tiles
            .iter()
            .map(|t| {
                let (corners, edges) = tile_sides(t, 0);
                let corners = corners.map(|(name, height)| {
                    let height = height.clamp(-MAX_HEIGHT, MAX_HEIGHT);
                    Corner { terrain: Terrain(intern(&mut terrains, name)), height }
                });
                let edges = edges
                    .map(|name| (!name.is_empty()).then(|| Crosser(intern(&mut crossers, name))));
                Cell { corners, edges }
            })
            .collect();
        let mut grouped = vec![false; cells.len()];
        for g in &set.groups {
            for t in g.tiles.iter().flatten() {
                if let Some(slot) = grouped.get_mut(*t as usize) {
                    *slot = true;
                }
            }
        }
        let mut fits: HashMap<Cell, Vec<Placement>> = HashMap::new();
        for (tile, cell) in cells.iter().enumerate() {
            // (A hole in the tileset, a tile with no model, is never one
            // to paint.)
            if grouped[tile] || set.tiles[tile].is_hole() {
                continue;
            }
            for orientation in 0..4 {
                fits.entry(cell.turned(orientation)).or_default().push(Placement {
                    tile: tile as u32,
                    orientation,
                    height: 0,
                });
            }
        }
        let holes = set.tiles.iter().map(mg_set::Tile::is_hole).collect();
        let max_height = cells.iter().flat_map(|c| c.corners).map(|c| c.height).max().unwrap_or(0);
        let groups = set.groups.iter().map(|g| (g.columns.max(1), g.tiles.clone())).collect();
        TileIndex { terrains, crossers, cells, grouped, holes, groups, fits, max_height }
    }

    /// A terrain by name (case-insensitive).
    pub fn terrain(&self, name: &str) -> Option<Terrain> {
        self.terrains.iter().position(|n| n.eq_ignore_ascii_case(name)).map(|i| Terrain(i as u16))
    }

    /// A crosser by name (case-insensitive).
    pub fn crosser(&self, name: &str) -> Option<Crosser> {
        self.crossers.iter().position(|n| n.eq_ignore_ascii_case(name)).map(|i| Crosser(i as u16))
    }

    /// The terrains, declared ones first, then those only tiles name (not
    /// the nameless corners of a hole in the tileset).
    pub fn terrains(&self) -> impl Iterator<Item = Terrain> + '_ {
        (0..self.terrains.len())
            .filter(|&i| !self.terrains[i].is_empty())
            .map(|i| Terrain(i as u16))
    }

    /// The crossers, declared ones first, then those only tiles name.
    pub fn crossers(&self) -> impl Iterator<Item = Crosser> + '_ {
        (0..self.crossers.len()).map(|i| Crosser(i as u16))
    }

    pub fn terrain_name(&self, t: Terrain) -> &str {
        &self.terrains[usize::from(t.0)]
    }

    pub fn crosser_name(&self, c: Crosser) -> &str {
        &self.crossers[usize::from(c.0)]
    }

    /// What a placed tile puts in its cell; `None` for a tile the tileset
    /// does not have.
    pub fn cell(&self, p: Placement) -> Option<Cell> {
        let mut cell = self.cells.get(p.tile as usize)?.turned(p.orientation);
        for c in &mut cell.corners {
            c.height = c.height.saturating_add(p.height);
        }
        Some(cell)
    }

    /// The tiles (outside groups) whose cell, unturned, `keep` keeps, in
    /// the tileset's order: the tiles with a terrain at a corner, say.
    pub fn tiles_where(&self, keep: impl Fn(&Cell) -> bool) -> Vec<u32> {
        (self.cells.iter().enumerate())
            .filter(|(i, cell)| !self.grouped[*i] && keep(cell))
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Whether a tile belongs to a group. Group tiles are placed only with
    /// their group, never to fit terrain.
    pub fn is_grouped(&self, tile: u32) -> bool {
        self.grouped.get(tile as usize).copied().unwrap_or(false)
    }

    /// Whether a tile is a hole in the tileset: its section or its model
    /// is missing (the game draws nothing there).
    pub fn is_hole(&self, tile: u32) -> bool {
        self.holes.get(tile as usize).copied().unwrap_or(false)
    }

    /// Whether a tile outside groups (and no hole) has crosser `c` on an
    /// edge: one painting can lay, and that can meet a group's.
    pub fn is_free_crosser(&self, c: Crosser) -> bool {
        (self.cells.iter().enumerate())
            .any(|(i, cell)| !self.grouped[i] && !self.holes[i] && cell.edges.contains(&Some(c)))
    }

    /// The cells of the group placed with a tile of its own at (x, y):
    /// every cell the group put a tile in, where all of them are still as
    /// it placed them (`tile` gives a cell's tile, `None` outside the
    /// area). `None` for a tile of no group, and for what is left of a
    /// group that something else was painted over.
    pub fn group_at(
        &self,
        tile: &dyn Fn(i64, i64) -> Option<Placement>,
        x: u32,
        y: u32,
    ) -> Option<Vec<(u32, u32)>> {
        self.group_found_at(tile, x, y).map(|(_, cells)| cells)
    }

    /// [`TileIndex::group_at`], with which of the tileset's groups it is
    /// (its index): the one rule for "which group stands here".
    pub fn group_found_at(
        &self,
        tile: &dyn Fn(i64, i64) -> Option<Placement>,
        x: u32,
        y: u32,
    ) -> Option<(usize, Vec<(u32, u32)>)> {
        let here = tile(i64::from(x), i64::from(y))?;
        if !self.is_grouped(here.tile) {
            return None;
        }
        let turns = here.orientation % 4;
        // Where slot `k` of a group of `columns` goes from its first cell,
        // turned as the group was (as `Grid::place_group` turns it).
        let offset = |k: usize, columns: u32| group_slot(k, columns, turns);
        for (group, (columns, tiles)) in self.groups.iter().enumerate() {
            for (k, _) in tiles.iter().enumerate().filter(|(_, t)| **t == Some(here.tile)) {
                let (c, r) = offset(k, *columns);
                let origin = (i64::from(x) - c, i64::from(y) - r);
                let mut cells = Vec::new();
                let whole = tiles.iter().enumerate().all(|(j, t)| {
                    let Some(t) = t else { return true };
                    let (c, r) = offset(j, *columns);
                    let at = (origin.0 + c, origin.1 + r);
                    let same = tile(at.0, at.1).is_some_and(|p| {
                        p.tile == *t && p.orientation % 4 == turns && p.height == here.height
                    });
                    if same {
                        cells.push((at.0 as u32, at.1 as u32));
                    }
                    same
                });
                if whole {
                    return Some((group, cells));
                }
            }
        }
        None
    }

    /// The tiles (outside groups) that fit a cell, at each orientation and
    /// height that fits.
    pub fn fits(&self, cell: &Cell) -> Vec<Placement> {
        let low = cell.corners.iter().map(|c| c.height).min().unwrap_or(0);
        let mut out = Vec::new();
        // (A tile raised: from where its highest corner reaches the
        // cell's lowest, up to the cell's lowest; as many tries as the
        // tileset's tiles are high, at most.)
        for raise in low.saturating_sub(self.max_height).max(0)..=low.max(0) {
            let mut lowered = *cell;
            for c in &mut lowered.corners {
                c.height = c.height.saturating_sub(raise);
            }
            if let Some(fits) = self.fits.get(&lowered) {
                out.extend(fits.iter().map(|p| Placement { height: raise, ..*p }));
            }
        }
        out
    }
}

/// The corners and edges of a `width` × `height` grid of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lattice {
    width: u32,
    height: u32,
    /// `(width + 1) * (height + 1)`, row-major from the south-west.
    corners: Vec<Corner>,
    /// Edges along x: `width * (height + 1)`; edge (x, y) runs from corner
    /// (x, y) to (x + 1, y).
    x_edges: Vec<Option<Crosser>>,
    /// Edges along y: `(width + 1) * height`; edge (x, y) runs from corner
    /// (x, y) to (x, y + 1).
    y_edges: Vec<Option<Crosser>>,
}

impl Lattice {
    /// A lattice with every corner `fill` and no crossers.
    pub fn new(width: u32, height: u32, fill: Corner) -> Lattice {
        let (w, h) = (width as usize, height as usize);
        Lattice {
            width,
            height,
            corners: vec![fill; (w + 1) * (h + 1)],
            x_edges: vec![None; w * (h + 1)],
            y_edges: vec![None; (w + 1) * h],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Whether corner (x, y) is one of the lattice's: x up to its width,
    /// y up to its height.
    pub fn has_corner(&self, x: u32, y: u32) -> bool {
        x <= self.width && y <= self.height
    }

    /// Whether cell (x, y) is one of the grid's.
    pub fn has_cell(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    fn corner_index(&self, x: u32, y: u32) -> usize {
        assert!(self.has_corner(x, y), "corner ({x}, {y}) outside the lattice");
        y as usize * (self.width as usize + 1) + x as usize
    }

    /// Corner (x, y), which must be the lattice's: [`Lattice::get_corner`]
    /// where it may not be (a point from outside, or kept from before the
    /// area changed size).
    pub fn corner(&self, x: u32, y: u32) -> Corner {
        self.corners[self.corner_index(x, y)]
    }

    /// Corner (x, y); `None` outside the lattice.
    pub fn get_corner(&self, x: u32, y: u32) -> Option<Corner> {
        self.has_corner(x, y).then(|| self.corner(x, y))
    }

    pub fn set_corner(&mut self, x: u32, y: u32, c: Corner) {
        let i = self.corner_index(x, y);
        self.corners[i] = c;
    }

    fn edge_slots(&self, x: u32, y: u32) -> [(bool, usize); 4] {
        assert!(self.has_cell(x, y), "cell ({x}, {y}) outside the grid");
        let (x, y, w) = (x as usize, y as usize, self.width as usize);
        [
            (true, y * w + x),
            (false, y * (w + 1) + x + 1),
            (true, (y + 1) * w + x),
            (false, y * (w + 1) + x),
        ]
    }

    /// The cell at (x, y), which must be the grid's: [`Lattice::get_cell`]
    /// where it may not be.
    pub fn cell(&self, x: u32, y: u32) -> Cell {
        assert!(self.has_cell(x, y), "cell ({x}, {y}) outside the grid");
        let corners = [
            self.corner(x, y),
            self.corner(x + 1, y),
            self.corner(x + 1, y + 1),
            self.corner(x, y + 1),
        ];
        let edges = self
            .edge_slots(x, y)
            .map(|(along_x, i)| if along_x { self.x_edges[i] } else { self.y_edges[i] });
        Cell { corners, edges }
    }

    /// The cell at (x, y); `None` outside the grid.
    pub fn get_cell(&self, x: u32, y: u32) -> Option<Cell> {
        self.has_cell(x, y).then(|| self.cell(x, y))
    }

    /// Sets edge `edge` (from [`SOUTH`], counter-clockwise) of cell (x, y),
    /// which it shares with the neighbour on that side.
    pub fn set_edge(&mut self, x: u32, y: u32, edge: usize, crosser: Option<Crosser>) {
        let (along_x, slot) = self.edge_slots(x, y)[edge % 4];
        if along_x {
            self.x_edges[slot] = crosser;
        } else {
            self.y_edges[slot] = crosser;
        }
    }

    /// Writes a cell's corners and edges (shared with its neighbours).
    pub fn set_cell(&mut self, x: u32, y: u32, cell: &Cell) {
        for (i, (cx, cy)) in
            [(x, y), (x + 1, y), (x + 1, y + 1), (x, y + 1)].into_iter().enumerate()
        {
            self.set_corner(cx, cy, cell.corners[i]);
        }
        for (i, (along_x, slot)) in self.edge_slots(x, y).into_iter().enumerate() {
            if along_x {
                self.x_edges[slot] = cell.edges[i];
            } else {
                self.y_edges[slot] = cell.edges[i];
            }
        }
    }

    /// The lattice a grid of placed tiles describes, and the cells whose
    /// tile disagrees with an earlier neighbour about a shared corner or
    /// edge (their own values win). `None` if a tile is not in the tileset,
    /// or if there are fewer tiles than the size asks for: the size is a
    /// number a file gives, and nothing is made that large before the
    /// tiles are counted (so an area a side of which is 0 has no lattice
    /// either, unless both are).
    pub fn from_tiles(
        index: &TileIndex,
        width: u32,
        height: u32,
        tiles: &[Placement],
    ) -> Option<(Lattice, Vec<(u32, u32)>)> {
        let cells = (width as usize).checked_mul(height as usize)?;
        if tiles.len() < cells || (cells == 0 && (width != 0 || height != 0)) {
            return None;
        }
        let fill = Corner { terrain: Terrain(0), height: 0 };
        let mut lattice = Lattice::new(width, height, fill);
        let mut set_corners = vec![false; lattice.corners.len()];
        let mut set_x = vec![false; lattice.x_edges.len()];
        let mut set_y = vec![false; lattice.y_edges.len()];
        let mut mismatches = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let cell = index.cell(*tiles.get(y as usize * width as usize + x as usize)?)?;
                let mut bad = false;
                for (i, (cx, cy)) in
                    [(x, y), (x + 1, y), (x + 1, y + 1), (x, y + 1)].into_iter().enumerate()
                {
                    let k = lattice.corner_index(cx, cy);
                    bad |= set_corners[k] && lattice.corners[k] != cell.corners[i];
                    set_corners[k] = true;
                }
                for (i, (along_x, slot)) in lattice.edge_slots(x, y).into_iter().enumerate() {
                    let (seen, value) = if along_x {
                        (&mut set_x[slot], &lattice.x_edges[slot])
                    } else {
                        (&mut set_y[slot], &lattice.y_edges[slot])
                    };
                    bad |= *seen && *value != cell.edges[i];
                    *seen = true;
                }
                if bad {
                    mismatches.push((x, y));
                }
                lattice.set_cell(x, y, &cell);
            }
        }
        Some((lattice, mismatches))
    }
}

/// The terrain Aurora's area wizard gives a new area: the tileset's
/// `Default` terrain, its `Border` terrain on the outer corners, and its
/// `Floor` terrain on the corners nearest the centre (one to four of them),
/// all at height 0.
pub fn new_area(
    index: &TileIndex,
    set: &Tileset,
    width: u32,
    height: u32,
) -> Result<Lattice, TilesError> {
    let terrain = |name: &str| {
        index.terrain(name).ok_or_else(|| TilesError::UnknownTerrain(name.to_string()))
    };
    let corner = |t| Corner { terrain: t, height: 0 };
    let (default, border, floor) = (
        terrain(&set.general.default)?,
        terrain(&set.general.border)?,
        terrain(&set.general.floor)?,
    );
    let mut lattice = Lattice::new(width, height, corner(default));
    for y in 0..=height {
        for x in 0..=width {
            if x == 0 || y == 0 || x == width || y == height {
                lattice.set_corner(x, y, corner(border));
            }
        }
    }
    // Corners within half a cell of the centre.
    let near = |v: u32, n: u32| (2 * i64::from(v) - i64::from(n)).abs() <= 1;
    for y in 1..height {
        for x in 1..width {
            if near(x, width) && near(y, height) {
                lattice.set_corner(x, y, corner(floor));
            }
        }
    }
    Ok(lattice)
}

/// Chooses a tile for every cell of a lattice, at random among the tiles
/// (outside groups) and orientations that fit. Row-major from the
/// south-west, like `Tile_List`.
pub fn fill(
    index: &TileIndex,
    lattice: &Lattice,
    rng: &mut fastrand::Rng,
) -> Result<Vec<Placement>, TilesError> {
    let mut out = Vec::with_capacity((lattice.width * lattice.height) as usize);
    for y in 0..lattice.height {
        for x in 0..lattice.width {
            let fits = index.fits(&lattice.cell(x, y));
            if fits.is_empty() {
                return Err(TilesError::NoTile { x, y });
            }
            out.push(fits[rng.usize(..fits.len())]);
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TilesError {
    #[error("the tileset has no terrain {0:?}")]
    UnknownTerrain(String),
    #[error("no tile fits cell ({x}, {y})")]
    NoTile { x: u32, y: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::Codepage;

    /// Terrains A and B; tiles: all A, one B corner (bottom left), two B
    /// corners (bottom), all B, plus an all-A tile that only a group uses.
    const SET: &str = "[GENERAL]\nName=TST01\nInterior=1\nTransition=3\nDisplayName=-1\nBorder=A\nDefault=A\nFloor=B\n\
[TERRAIN TYPES]\nCount=2\n[TERRAIN0]\nName=A\n[TERRAIN1]\nName=B\n[CROSSER TYPES]\nCount=0\n\
[PRIMARY RULES]\nCount=0\n[SECONDARY RULES]\nCount=0\n\
[TILES]\nCount=5\n\
[TILE0]\nModel=t0\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\n\
[TILE1]\nModel=t1\nTopLeft=A\nTopRight=A\nBottomLeft=B\nBottomRight=A\n\
[TILE2]\nModel=t2\nTopLeft=A\nTopRight=A\nBottomLeft=B\nBottomRight=B\n\
[TILE3]\nModel=t3\nTopLeft=B\nTopRight=B\nBottomLeft=B\nBottomRight=B\n\
[TILE4]\nModel=t4\nTopLeft=a\nTopRight=a\nBottomLeft=a\nBottomRight=a\n\
[GROUPS]\nCount=1\n[GROUP0]\nName=G\nRows=1\nColumns=1\nTile0=4\n";

    fn set() -> Tileset {
        Tileset::parse(SET.as_bytes(), Codepage::WINDOWS_1252).unwrap()
    }

    #[test]
    fn turning_moves_corners_counter_clockwise() {
        let index = TileIndex::new(&set());
        let (a, b) = (index.terrain("a").unwrap(), index.terrain("B").unwrap());
        // Tile 1 has B at its south-west corner; three quarter turns bring it
        // to the north-west.
        let cell = index.cell(Placement { tile: 1, orientation: 3, height: 0 }).unwrap();
        let terrains = cell.corners.map(|c| c.terrain);
        assert_eq!(terrains[NW], b);
        assert_eq!(terrains.iter().filter(|t| **t == a).count(), 3);
        assert_eq!(cell.turned(1).turned(3), cell);
    }

    /// A tile's corners and edges as the grid has them, from the `.set`'s
    /// own order (TopLeft, TopRight, BottomLeft, BottomRight; Top, Right,
    /// Bottom, Left), turned as the tile lies: what the index holds.
    #[test]
    fn a_tile_s_sides_are_the_index_s() {
        let text = SET.replace(
            "[TILE1]\nModel=t1\nTopLeft=A\nTopRight=A\nBottomLeft=B\nBottomRight=A\n",
            "[TILE1]\nModel=t1\nTopLeft=A\nTopRight=A\nBottomLeft=B\nBottomRight=A\n\
             BottomLeftHeight=1\nTop=Road\nLeft=Stream\n",
        );
        assert_ne!(text, SET);
        let set = Tileset::parse(text.as_bytes(), Codepage::WINDOWS_1252).unwrap();
        let (corners, edges) = tile_sides(&set.tiles[1], 0);
        assert_eq!(corners, [("B", 1), ("A", 0), ("A", 0), ("A", 0)], "from the south-west");
        assert_eq!(edges, ["", "", "Road", "Stream"], "from the south");
        let (corners, edges) = tile_sides(&set.tiles[1], 1);
        assert_eq!(corners[SE], ("B", 1));
        assert_eq!((edges[WEST], edges[SOUTH]), ("Road", "Stream"));
        let index = TileIndex::new(&set);
        for turns in 0..4 {
            let (corners, edges) = tile_sides(&set.tiles[1], turns);
            let cell = index.cell(Placement { tile: 1, orientation: turns, height: 0 }).unwrap();
            for i in 0..4 {
                let c = cell.corners[i];
                assert_eq!((index.terrain_name(c.terrain), c.height), corners[i]);
                assert_eq!(cell.edges[i].map_or("", |e| index.crosser_name(e)), edges[i]);
            }
        }
    }

    #[test]
    fn group_tiles_do_not_fit_terrain() {
        let index = TileIndex::new(&set());
        assert!(index.is_grouped(4));
        let a = Corner { terrain: index.terrain("A").unwrap(), height: 0 };
        let fits = index.fits(&Cell { corners: [a; 4], edges: [None; 4] });
        assert_eq!(fits.len(), 4, "tile 0 at each orientation: {fits:?}");
        assert!(fits.iter().all(|p| p.tile == 0));
    }

    #[test]
    fn new_area_puts_floor_near_the_centre() {
        let set = set();
        let index = TileIndex::new(&set);
        let b = index.terrain("B").unwrap();
        let floor = |l: &Lattice| {
            let mut v = Vec::new();
            for y in 0..=l.height() {
                for x in 0..=l.width() {
                    if l.corner(x, y).terrain == b {
                        v.push((x, y));
                    }
                }
            }
            v
        };
        // As Aurora makes them (captured under Wine).
        assert_eq!(floor(&new_area(&index, &set, 4, 4).unwrap()), [(2, 2)]);
        assert_eq!(floor(&new_area(&index, &set, 3, 3).unwrap()), [(1, 1), (2, 1), (1, 2), (2, 2)]);
        assert_eq!(floor(&new_area(&index, &set, 5, 2).unwrap()), [(2, 1), (3, 1)]);
        assert_eq!(floor(&new_area(&index, &set, 2, 2).unwrap()), [(1, 1)]);
    }

    #[test]
    fn filled_tiles_reproduce_the_lattice() {
        let set = set();
        let index = TileIndex::new(&set);
        let mut rng = fastrand::Rng::with_seed(7);
        for (w, h) in [(2, 2), (3, 3), (4, 4), (5, 2), (8, 8)] {
            let lattice = new_area(&index, &set, w, h).unwrap();
            let tiles = fill(&index, &lattice, &mut rng).unwrap();
            let (back, mismatches) = Lattice::from_tiles(&index, w, h, &tiles).unwrap();
            assert!(mismatches.is_empty());
            assert_eq!(back, lattice);
        }
    }

    #[test]
    fn a_cell_nothing_fits_is_reported() {
        let set = set();
        let index = TileIndex::new(&set);
        let (a, b) = (index.terrain("A").unwrap(), index.terrain("B").unwrap());
        let mut lattice = Lattice::new(2, 1, Corner { terrain: a, height: 0 });
        // B on opposite corners of cell (0, 0): no tile has that.
        lattice.set_corner(0, 0, Corner { terrain: b, height: 0 });
        lattice.set_corner(1, 1, Corner { terrain: b, height: 0 });
        let err = fill(&index, &lattice, &mut fastrand::Rng::with_seed(1)).unwrap_err();
        assert_eq!(err, TilesError::NoTile { x: 0, y: 0 });
    }

    /// A size no list of tiles stands behind (an ARE's `Width` and
    /// `Height` are whatever the file says) makes no lattice, and is
    /// refused before anything of that size is allocated.
    #[test]
    fn a_size_without_its_tiles_makes_no_lattice() {
        let index = TileIndex::new(&set());
        let one = Placement { tile: 0, orientation: 0, height: 0 };
        assert!(Lattice::from_tiles(&index, u32::MAX, u32::MAX, &[]).is_none());
        assert!(Lattice::from_tiles(&index, 100_000, 100_000, &[one]).is_none());
        assert!(Lattice::from_tiles(&index, i32::MAX as u32, i32::MAX as u32, &[one]).is_none());
        assert!(Lattice::from_tiles(&index, u32::MAX, 0, &[]).is_none());
        assert!(Lattice::from_tiles(&index, 0, 100_000, &[one]).is_none());
        assert!(paint::Grid::new(&index, u32::MAX, u32::MAX, Vec::new()).is_none());
        assert!(paint::Grid::new(&index, 100_000, 100_000, vec![one]).is_none());
        // An area of no tiles is still one, and a list may run on.
        assert!(Lattice::from_tiles(&index, 0, 0, &[]).is_some());
        assert!(Lattice::from_tiles(&index, 1, 1, &[one, one]).is_some());
    }

    /// Heights are numbers files give: a tileset's corner 2 billion up is
    /// read as [`MAX_HEIGHT`], a cell that high is fitted in as many tries
    /// as the tileset is high, and a tile raised past what an `i32` holds
    /// neither wraps nor panics.
    #[test]
    fn heights_from_files_are_bounded() {
        let high = SET.replace(
            "[TILE1]\nModel=t1\nTopLeft=A",
            "[TILE1]\nModel=t1\nTopLeftHeight=2000000000\nBottomRightHeight=-2000000000\nTopLeft=A",
        );
        assert_ne!(high, SET);
        let set = Tileset::parse(high.as_bytes(), Codepage::WINDOWS_1252).unwrap();
        assert_eq!(set.tiles[1].corners[0].1, 2_000_000_000);
        let index = TileIndex::new(&set);
        let cell = index.cell(Placement { tile: 1, orientation: 0, height: 0 }).unwrap();
        let heights = cell.corners.map(|c| c.height);
        assert_eq!(heights.iter().max(), Some(&MAX_HEIGHT));
        assert_eq!(heights.iter().min(), Some(&-MAX_HEIGHT));
        let up = index.cell(Placement { tile: 1, orientation: 0, height: i32::MAX }).unwrap();
        assert_eq!(up.corners.map(|c| c.height).iter().max(), Some(&i32::MAX));
        // A cell every corner of which is as high as an `i32` goes, and
        // one as low: looked up in a moment, not once a step.
        let a = index.terrain("A").unwrap();
        let started = std::time::Instant::now();
        for height in [i32::MAX, i32::MIN, 2_000_000_000] {
            let cell = Cell { corners: [Corner { terrain: a, height }; 4], edges: [None; 4] };
            let fits = index.fits(&cell);
            assert_eq!(fits.len(), if height > 0 { 4 } else { 0 }, "{height}: {fits:?}");
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
    }

    /// Corners run to the width and height, cells to one less: past them
    /// a lookup is `None` (GitHub issue 31's corner (5, 9) of a 16 by 8
    /// area).
    #[test]
    fn lookups_outside_the_lattice_are_none() {
        let set = set();
        let index = TileIndex::new(&set);
        let a = Corner { terrain: index.terrain("A").unwrap(), height: 0 };
        let lattice = Lattice::new(3, 2, a);
        assert_eq!(lattice.get_corner(3, 2), Some(a));
        assert_eq!((lattice.get_corner(4, 0), lattice.get_corner(0, 3)), (None, None));
        assert!(lattice.get_cell(2, 1).is_some());
        assert_eq!((lattice.get_cell(3, 0), lattice.get_cell(0, 2)), (None, None));
        assert!(lattice.has_corner(3, 2) && !lattice.has_cell(3, 2));
        let empty = Lattice::new(0, 0, a);
        assert!(empty.get_corner(0, 0).is_some() && empty.get_cell(0, 0).is_none());
    }
}
