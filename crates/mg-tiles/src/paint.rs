//! Painting an area's terrain as Aurora's brushes do (captured under Wine;
//! `aurora_terrain.rs` replays the captures):
//!
//! - A terrain brush sets one lattice corner's terrain, keeping its height;
//!   the tileset's primary rules then rewrite its eight neighbours (Storage
//!   painted next to Rich turns the Rich corner into Wall).
//! - Raise and Lower move one corner a height step; its neighbours follow so
//!   that no two neighbouring corners are more than a step apart (raising a
//!   corner twice raises the eight around it once), and the rules apply as
//!   for painting its terrain at the new height.
//! - A crosser brush dragged from cell to cell puts the crosser on the edge
//!   each step crosses.
//! - Every cell with a changed corner or edge, and the four cells around a
//!   painted corner, get a new tile, chosen at random among those that fit;
//!   the others keep theirs. When one of those cells has no tile that fits,
//!   Aurora changes nothing: the stroke is refused.
//!
//! Rules are written in relative heights (0 and 1): a rule for `Placed@1`
//! next to `Adjacent@0` applies where the painted corner is a step above
//! its neighbour, and `Changed@1` puts the neighbour a step above the lower
//! of the two.

use std::collections::{BTreeSet, HashMap, VecDeque};

use mg_set::Tileset;

use crate::{Cell, Corner, Crosser, Lattice, Placement, Terrain, TileIndex};

/// A tileset's primary rules, by the terrains and relative heights they
/// apply to.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    rules: HashMap<(Terrain, i32, Terrain, i32), (Terrain, i32)>,
}

impl Rules {
    /// The primary rules of `set`; rules naming terrains no tile has are
    /// left out, and of two rules for the same pair the first counts.
    pub fn new(index: &TileIndex, set: &Tileset) -> Rules {
        let mut rules = HashMap::new();
        for r in &set.primary_rules {
            let (Some(placed), Some(adjacent), Some(changed)) =
                (index.terrain(&r.placed), index.terrain(&r.adjacent), index.terrain(&r.changed))
            else {
                continue;
            };
            rules
                .entry((placed, r.placed_height, adjacent, r.adjacent_height))
                .or_insert((changed, r.changed_height));
        }
        Rules { rules }
    }

    /// What a rule makes of corner `adjacent` next to a corner painted
    /// `placed`; `None` when no rule applies.
    pub fn apply(&self, placed: Corner, adjacent: Corner) -> Option<Corner> {
        let base = placed.height.min(adjacent.height);
        let (p, a) = (placed.height - base, adjacent.height - base);
        if p > 1 || a > 1 {
            return None;
        }
        let (terrain, height) = self.rules.get(&(placed.terrain, p, adjacent.terrain, a))?;
        Some(Corner { terrain: *terrain, height: base + height })
    }
}

/// An area's tiles and the lattice they make.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub lattice: Lattice,
    /// Row-major from the south-west, like `Tile_List`.
    pub tiles: Vec<Placement>,
}

/// A change a brush makes: the lattice after it and the cells that get new
/// tiles.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub lattice: Lattice,
    pub cells: Vec<(u32, u32)>,
    /// Cells whose tile is given (a group's).
    pub fixed: Vec<((u32, u32), Placement)>,
}

impl Grid {
    /// The grid of a `width` × `height` area's tiles; `None` when a tile is
    /// not in the tileset or the list is short.
    pub fn new(index: &TileIndex, width: u32, height: u32, tiles: Vec<Placement>) -> Option<Grid> {
        let (lattice, _) = Lattice::from_tiles(index, width, height, &tiles)?;
        Some(Grid { lattice, tiles })
    }

    pub fn tile(&self, x: u32, y: u32) -> Placement {
        self.tiles[(y * self.lattice.width() + x) as usize]
    }

    /// Painting corner (x, y) with `terrain`; `None` when refused.
    pub fn paint(
        &self,
        index: &TileIndex,
        rules: &Rules,
        x: u32,
        y: u32,
        terrain: Terrain,
    ) -> Option<Stroke> {
        let mut lattice = self.lattice.clone();
        let mut changed = BTreeSet::new();
        let mut c = lattice.corner(x, y);
        c.terrain = terrain;
        lattice.set_corner(x, y, c);
        changed.insert((x, y));
        apply_rules(rules, &mut lattice, x, y, &mut changed);
        self.stroke(index, lattice, changed, around(&self.lattice, x, y))
    }

    /// Raising (`up`) or lowering corner (x, y) a step; `None` when refused
    /// (a cell nothing fits). A corner at height 0 stays there: lowering it
    /// chooses the tiles around it again.
    pub fn raise(
        &self,
        index: &TileIndex,
        rules: &Rules,
        x: u32,
        y: u32,
        up: bool,
    ) -> Option<Stroke> {
        let mut lattice = self.lattice.clone();
        let mut c = lattice.corner(x, y);
        c.height += if up { 1 } else { -1 };
        if c.height < 0 {
            return self.repick(index, &around(&self.lattice, x, y));
        }
        lattice.set_corner(x, y, c);
        let mut changed = BTreeSet::from([(x, y)]);
        settle(&mut lattice, vec![(x, y)], &mut changed);
        apply_rules(rules, &mut lattice, x, y, &mut changed);
        self.stroke(index, lattice, changed, around(&self.lattice, x, y))
    }

    /// Dragging `crosser` over `edges` (cell and edge, from [`SOUTH`]
    /// counter-clockwise): Aurora's crosser brush puts the crosser on the
    /// edge of every quarter of a cell the pointer passes through with the
    /// button down (the quarter nearest that edge), from the one it was in
    /// when pressed ([`path_edges`] for a path through cell centres). The
    /// cells on both sides of each edge choose their tiles again; with no
    /// edge, the `cells` given (a click). `None` when refused.
    ///
    /// [`SOUTH`]: crate::SOUTH
    pub fn draw_crosser(
        &self,
        index: &TileIndex,
        edges: &[((u32, u32), usize)],
        click: &[(u32, u32)],
        crosser: Crosser,
    ) -> Option<Stroke> {
        let mut lattice = self.lattice.clone();
        let mut cells: Vec<(u32, u32)> = click.to_vec();
        for &((x, y), edge) in edges {
            lattice.set_edge(x, y, edge, Some(crosser));
            for c in std::iter::once((x, y)).chain(beside(&lattice, x, y, edge)) {
                if !cells.contains(&c) {
                    cells.push(c);
                }
            }
        }
        self.stroke(index, lattice, BTreeSet::new(), cells)
    }

    /// The Eraser on cell (x, y): the crossers on its edges go and its tile
    /// is chosen again, as are those across the cleared edges. A tile that
    /// then has nothing to fit loses its other kinds of crosser (a road
    /// erased beside a bridge takes the stream with it), or else all of
    /// them, and so on outward. `None` when refused.
    pub fn erase(&self, index: &TileIndex, x: u32, y: u32) -> Option<Stroke> {
        let mut lattice = self.lattice.clone();
        let mut cells = vec![(x, y)];
        // Cells to fit again, with the crosser they lost.
        let mut queue: VecDeque<((u32, u32), Crosser)> = VecDeque::new();
        let clear = |lattice: &mut Lattice,
                     (x, y): (u32, u32),
                     keep: Option<Crosser>,
                     queue: &mut VecDeque<((u32, u32), Crosser)>| {
            let cell = lattice.cell(x, y);
            for edge in 0..4 {
                if let Some(c) = cell.edges[edge].filter(|c| Some(*c) != keep) {
                    lattice.set_edge(x, y, edge, None);
                    if let Some(n) = beside(lattice, x, y, edge) {
                        queue.push_back((n, c));
                    }
                }
            }
        };
        clear(&mut lattice, (x, y), None, &mut queue);
        while let Some((n, lost)) = queue.pop_front() {
            if !cells.contains(&n) {
                cells.push(n);
            }
            if !index.fits(&lattice.cell(n.0, n.1)).is_empty() {
                continue;
            }
            clear(&mut lattice, n, Some(lost), &mut queue);
            if index.fits(&lattice.cell(n.0, n.1)).is_empty() {
                clear(&mut lattice, n, None, &mut queue);
            }
        }
        self.stroke(index, lattice, BTreeSet::new(), cells)
    }

    /// Choosing the tiles of `cells` again, the terrain as it is (the
    /// Eraser, on the tile under the pointer). `None` when one has no tile
    /// that fits.
    pub fn repick(&self, index: &TileIndex, cells: &[(u32, u32)]) -> Option<Stroke> {
        self.stroke(index, self.lattice.clone(), BTreeSet::new(), cells.to_vec())
    }

    /// Placing tile group `group` of the tileset with its first tile (the
    /// south-west one) in cell (x, y), turned `turns` quarter turns
    /// counter-clockwise about that cell, as Aurora does: the group's tiles
    /// go in at the height of the ground under its first tile, their corners
    /// and edges replace the terrain there (neighbouring corners follow
    /// within a step), and the tiles around choose again; a cell the group
    /// leaves empty (`-1`) is matched like terrain. `None` when the group
    /// does not fit inside the area or a tile around it has nothing to fit.
    pub fn place_group(
        &self,
        index: &TileIndex,
        group: &mg_set::Group,
        x: u32,
        y: u32,
        turns: u8,
    ) -> Option<Stroke> {
        let (w, h) = (self.lattice.width() as i64, self.lattice.height() as i64);
        let base = self.lattice.cell(x, y).corners.iter().map(|c| c.height).min().unwrap_or(0);
        let mut lattice = self.lattice.clone();
        let mut fixed = Vec::new();
        let mut empty = Vec::new();
        let columns = group.columns.max(1);
        for (k, tile) in group.tiles.iter().enumerate() {
            let (mut c, mut r) = ((k as u32 % columns) as i64, (k as u32 / columns) as i64);
            for _ in 0..turns % 4 {
                (c, r) = (-r, c);
            }
            let (cx, cy) = (x as i64 + c, y as i64 + r);
            if cx < 0 || cy < 0 || cx >= w || cy >= h {
                return None;
            }
            let cell = (cx as u32, cy as u32);
            match tile {
                Some(t) => {
                    let p = Placement { tile: *t, orientation: turns % 4, height: base };
                    lattice.set_cell(cell.0, cell.1, &index.cell(p)?);
                    fixed.push((cell, p));
                }
                None => empty.push(cell),
            }
        }
        let placed: Vec<(u32, u32)> = fixed.iter().map(|(c, _)| *c).collect();
        let in_group = |(u, v): (u32, u32)| {
            placed.iter().any(|&(cx, cy)| (u == cx || u == cx + 1) && (v == cy || v == cy + 1))
        };
        let mut changed = BTreeSet::new();
        for v in 0..=self.lattice.height() {
            for u in 0..=self.lattice.width() {
                if self.lattice.corner(u, v) != lattice.corner(u, v) {
                    changed.insert((u, v));
                }
            }
        }
        settle_around(&mut lattice, changed.iter().copied().collect(), &mut changed, &in_group);
        // The cells around the group whose corners or edges changed.
        let mut cells = empty;
        for cy in 0..self.lattice.height() {
            for cx in 0..self.lattice.width() {
                let c = (cx, cy);
                if !placed.contains(&c)
                    && !cells.contains(&c)
                    && self.lattice.cell(cx, cy) != lattice.cell(cx, cy)
                {
                    cells.push(c);
                }
            }
        }
        let mut stroke = self.stroke(index, lattice, BTreeSet::new(), cells)?;
        stroke.fixed = fixed;
        Some(stroke)
    }

    /// The stroke that takes the grid to `lattice`: the cells touching a
    /// changed corner, plus `also`. Refused when one of them has no tile
    /// that fits, or holds a group's tile the change would not keep.
    fn stroke(
        &self,
        index: &TileIndex,
        lattice: Lattice,
        changed: BTreeSet<(u32, u32)>,
        also: Vec<(u32, u32)>,
    ) -> Option<Stroke> {
        let mut cells: Vec<(u32, u32)> = also;
        for &(x, y) in &changed {
            if self.lattice.corner(x, y) == lattice.corner(x, y) && !cells.is_empty() {
                // Unchanged after all (rules can put a corner back).
                continue;
            }
            for c in around(&lattice, x, y) {
                if !cells.contains(&c) {
                    cells.push(c);
                }
            }
        }
        for &(x, y) in &cells {
            let cell = lattice.cell(x, y);
            let tile = self.tile(x, y);
            if index.is_grouped(tile.tile) {
                if index.cell(tile) != Some(cell) {
                    return None;
                }
            } else if index.fits(&cell).is_empty() {
                return None;
            }
        }
        // Group tiles that still fit stay.
        cells.retain(|&(x, y)| !index.is_grouped(self.tile(x, y).tile));
        Some(Stroke { lattice, cells, fixed: Vec::new() })
    }

    /// Applies a stroke, choosing each of its cells' tiles at random among
    /// those that fit. Returns the cells that changed and their new tiles.
    pub fn apply(
        &mut self,
        index: &TileIndex,
        stroke: Stroke,
        rng: &mut fastrand::Rng,
    ) -> Vec<((u32, u32), Placement)> {
        let mut out = Vec::new();
        for &((x, y), p) in &stroke.fixed {
            self.tiles[(y * stroke.lattice.width() + x) as usize] = p;
            out.push(((x, y), p));
        }
        for &(x, y) in &stroke.cells {
            let fits = index.fits(&stroke.lattice.cell(x, y));
            if fits.is_empty() {
                continue;
            }
            let p = fits[rng.usize(..fits.len())];
            self.tiles[(y * stroke.lattice.width() + x) as usize] = p;
            out.push(((x, y), p));
        }
        self.lattice = stroke.lattice;
        out
    }
}

/// The cells around corner (x, y) (up to four).
fn around(l: &Lattice, x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::with_capacity(4);
    for (dx, dy) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
        let (cx, cy) = (x as i64 + dx, y as i64 + dy);
        if cx >= 0 && cy >= 0 && cx < i64::from(l.width()) && cy < i64::from(l.height()) {
            out.push((cx as u32, cy as u32));
        }
    }
    out
}

/// The corners around corner (x, y) (up to eight).
fn neighbours(l: &Lattice, x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::with_capacity(8);
    for dy in -1..=1i64 {
        for dx in -1..=1i64 {
            let (u, v) = (x as i64 + dx, y as i64 + dy);
            if (dx, dy) != (0, 0)
                && u >= 0
                && v >= 0
                && u <= i64::from(l.width())
                && v <= i64::from(l.height())
            {
                out.push((u as u32, v as u32));
            }
        }
    }
    out
}

/// The cell across edge `edge` of cell (x, y), if inside the grid.
fn beside(l: &Lattice, x: u32, y: u32, edge: usize) -> Option<(u32, u32)> {
    let (dx, dy) = [(0, -1), (1, 0), (0, 1), (-1, 0)][edge % 4];
    let (u, v) = (x as i64 + dx, y as i64 + dy);
    (u >= 0 && v >= 0 && u < i64::from(l.width()) && v < i64::from(l.height()))
        .then_some((u as u32, v as u32))
}

/// The edge of cell `a` it shares with cell `b`, if they are side by side.
fn shared_edge(a: (u32, u32), b: (u32, u32)) -> Option<usize> {
    let (dx, dy) = (b.0 as i64 - a.0 as i64, b.1 as i64 - a.1 as i64);
    match (dx, dy) {
        (0, -1) => Some(crate::SOUTH),
        (1, 0) => Some(crate::EAST),
        (0, 1) => Some(crate::NORTH),
        (-1, 0) => Some(crate::WEST),
        _ => None,
    }
}

/// The edges a drag through the centres of `path`'s cells (each beside the
/// one before) passes: each step leaves a cell by the edge it shares with
/// the next, and the press is in the quarter facing the first step.
pub fn path_edges(path: &[(u32, u32)]) -> Option<Vec<((u32, u32), usize)>> {
    path.windows(2).map(|pair| Some((pair[0], shared_edge(pair[0], pair[1])?))).collect()
}

/// The rules for corner (x, y), freshly painted, on its neighbours; then
/// the heights settle around those the rules changed.
fn apply_rules(
    rules: &Rules,
    lattice: &mut Lattice,
    x: u32,
    y: u32,
    changed: &mut BTreeSet<(u32, u32)>,
) {
    let placed = lattice.corner(x, y);
    let mut moved = Vec::new();
    for (u, v) in neighbours(lattice, x, y) {
        let adjacent = lattice.corner(u, v);
        if let Some(c) = rules.apply(placed, adjacent).filter(|c| *c != adjacent) {
            lattice.set_corner(u, v, c);
            changed.insert((u, v));
            if c.height != adjacent.height {
                moved.push((u, v));
            }
        }
    }
    settle(lattice, moved, changed);
}

/// Moves corners until no two neighbours are more than a height step apart,
/// starting from the corners `from` that moved: a neighbour follows a
/// moved corner to within a step of it.
fn settle(lattice: &mut Lattice, from: Vec<(u32, u32)>, changed: &mut BTreeSet<(u32, u32)>) {
    settle_around(lattice, from, changed, &|_| false);
}

/// [`settle`], leaving the corners `locked` says where they are.
fn settle_around(
    lattice: &mut Lattice,
    from: Vec<(u32, u32)>,
    changed: &mut BTreeSet<(u32, u32)>,
    locked: &dyn Fn((u32, u32)) -> bool,
) {
    let mut queue: VecDeque<(u32, u32)> = from.into();
    while let Some((x, y)) = queue.pop_front() {
        let h = lattice.corner(x, y).height;
        for (u, v) in neighbours(lattice, x, y) {
            if locked((u, v)) {
                continue;
            }
            let mut c = lattice.corner(u, v);
            let to = c.height.clamp(h - 1, h + 1);
            if to != c.height {
                c.height = to;
                lattice.set_corner(u, v, c);
                changed.insert((u, v));
                queue.push_back((u, v));
            }
        }
    }
}

/// The cell's tiles in Aurora's cycling order (Shift + click): by tile,
/// then orientation; the one after `current`, or the first.
pub fn next_fit(index: &TileIndex, cell: &Cell, current: Placement) -> Option<Placement> {
    let mut fits = index.fits(cell);
    fits.sort_by_key(|p| (p.tile, p.orientation, p.height));
    let at = fits.iter().position(|p| *p == current);
    match at {
        Some(i) => fits.get((i + 1) % fits.len()).copied(),
        None => fits.first().copied(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::Codepage;

    /// Terrains A (default), B and W; heights 0 and 1 for A. Tiles cover
    /// every A/B corner mix at height 0, A slopes, and a road (R) straight,
    /// end and crossing a stream (S). Rule: B next to W makes W into A.
    fn set() -> Tileset {
        let mut s = String::from(
            "[GENERAL]\nName=TST01\nInterior=0\nHasHeightTransition=1\nTransition=5\nDisplayName=-1\n\
Border=A\nDefault=A\nFloor=A\n\
[TERRAIN TYPES]\nCount=3\n[TERRAIN0]\nName=A\n[TERRAIN1]\nName=B\n[TERRAIN2]\nName=W\n\
[CROSSER TYPES]\nCount=2\n[CROSSER0]\nName=R\n[CROSSER1]\nName=S\n\
[PRIMARY RULES]\nCount=2\n\
[PRIMARY RULE0]\nPlaced=B\nPlacedHeight=0\nAdjacent=W\nAdjacentHeight=0\nChanged=A\nChangedHeight=0\n\
[PRIMARY RULE1]\nPlaced=B\nPlacedHeight=1\nAdjacent=A\nAdjacentHeight=0\nChanged=A\nChangedHeight=1\n\
[SECONDARY RULES]\nCount=0\n",
        );
        let mut tiles: Vec<String> = Vec::new();
        let tile = |c: [(&str, i32); 4], e: [&str; 4]| {
            format!(
                "Model=m\nTopLeft={}\nTopLeftHeight={}\nTopRight={}\nTopRightHeight={}\n\
BottomLeft={}\nBottomLeftHeight={}\nBottomRight={}\nBottomRightHeight={}\n\
Top={}\nRight={}\nBottom={}\nLeft={}\n",
                c[0].0,
                c[0].1,
                c[1].0,
                c[1].1,
                c[2].0,
                c[2].1,
                c[3].0,
                c[3].1,
                e[0],
                e[1],
                e[2],
                e[3]
            )
        };
        // Every A/B and A/W mix at height 0 (by bit pattern; turned copies
        // are matched by orientation).
        for other in ["B", "W"] {
            for bits in 0..16u8 {
                let t = |b: u8| if bits & (1 << b) != 0 { other } else { "A" };
                tiles.push(tile([(t(0), 0), (t(1), 0), (t(2), 0), (t(3), 0)], ["", "", "", ""]));
            }
        }
        // A slopes: one, two and three corners raised; and raised B.
        for bits in 1..16u8 {
            let h = |b: u8| i32::from(bits & (1 << b) != 0);
            tiles
                .push(tile([("A", h(0)), ("A", h(1)), ("A", h(2)), ("A", h(3))], ["", "", "", ""]));
        }
        tiles.push(tile([("B", 0), ("A", 0), ("A", 0), ("A", 0)], ["", "", "", ""]));
        // Roads: straight, end, and crossing a stream.
        tiles.push(tile([("A", 0); 4], ["R", "", "R", ""]));
        tiles.push(tile([("A", 0); 4], ["R", "", "", ""]));
        tiles.push(tile([("A", 0); 4], ["R", "S", "R", "S"]));
        tiles.push(tile([("A", 0); 4], ["", "S", "", ""]));
        s += &format!("[TILES]\nCount={}\n", tiles.len());
        for (i, t) in tiles.iter().enumerate() {
            s += &format!("[TILE{i}]\n{t}");
        }
        s += "[GROUPS]\nCount=0\n";
        Tileset::parse(s.as_bytes(), Codepage::WINDOWS_1252).unwrap()
    }

    fn grid(index: &TileIndex, n: u32) -> Grid {
        let a = Corner { terrain: index.terrain("A").unwrap(), height: 0 };
        let lattice = Lattice::new(n, n, a);
        let tiles = crate::fill(index, &lattice, &mut fastrand::Rng::with_seed(1)).unwrap();
        Grid::new(index, n, n, tiles).unwrap()
    }

    fn apply(g: &mut Grid, index: &TileIndex, brush: impl FnOnce(&Grid) -> Option<Stroke>) {
        let s = brush(g).expect("refused");
        g.apply(index, s, &mut fastrand::Rng::with_seed(2));
        let (back, bad) =
            Lattice::from_tiles(index, g.lattice.width(), g.lattice.height(), &g.tiles).unwrap();
        assert!(bad.is_empty());
        assert_eq!(back, g.lattice, "the tiles make the lattice");
    }

    fn heights(l: &Lattice) -> Vec<Vec<i32>> {
        (0..=l.height())
            .rev()
            .map(|y| (0..=l.width()).map(|x| l.corner(x, y).height).collect())
            .collect()
    }

    #[test]
    fn painting_changes_one_corner_and_its_four_cells() {
        let set = set();
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let mut g = grid(&index, 4);
        let b = index.terrain("B").unwrap();
        let before = g.tiles.clone();
        let s = g.paint(&index, &rules, 2, 2, b).unwrap();
        assert_eq!(s.cells.len(), 4);
        apply(&mut g, &index, |_| Some(s));
        assert_eq!(g.lattice.corner(2, 2).terrain, b);
        let changed = (0..16).filter(|&i| g.tiles[i] != before[i]).count();
        assert!(changed <= 4);
    }

    #[test]
    fn rules_rewrite_the_eight_neighbours() {
        let set = set();
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let mut g = grid(&index, 4);
        let (a, b, w) =
            (index.terrain("A").unwrap(), index.terrain("B").unwrap(), index.terrain("W").unwrap());
        apply(&mut g, &index, |g| g.paint(&index, &rules, 1, 1, w));
        // B diagonally beside the W corner: W turns into A.
        apply(&mut g, &index, |g| g.paint(&index, &rules, 2, 2, b));
        assert_eq!(g.lattice.corner(1, 1).terrain, a);
    }

    #[test]
    fn raising_twice_raises_the_neighbours_once() {
        let set = set();
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let mut g = grid(&index, 4);
        apply(&mut g, &index, |g| g.raise(&index, &rules, 2, 2, true));
        assert_eq!(g.lattice.corner(2, 2).height, 1);
        assert_eq!(g.lattice.corner(1, 1).height, 0);
        apply(&mut g, &index, |g| g.raise(&index, &rules, 2, 2, true));
        assert_eq!(
            heights(&g.lattice),
            [[0, 0, 0, 0, 0], [0, 1, 1, 1, 0], [0, 1, 2, 1, 0], [0, 1, 1, 1, 0], [0, 0, 0, 0, 0]]
        );
        // A corner at 0 stays there; its tiles are chosen again.
        let s = g.raise(&index, &rules, 0, 0, false).unwrap();
        assert_eq!((s.lattice == g.lattice, s.cells.as_slice()), (true, &[(0, 0)][..]));
        // Lowered back: the neighbours stay up, a step below.
        apply(&mut g, &index, |g| g.raise(&index, &rules, 2, 2, false));
        apply(&mut g, &index, |g| g.raise(&index, &rules, 2, 2, false));
        assert_eq!(
            heights(&g.lattice),
            [[0, 0, 0, 0, 0], [0, 1, 1, 1, 0], [0, 1, 0, 1, 0], [0, 1, 1, 1, 0], [0, 0, 0, 0, 0]]
        );
    }

    #[test]
    fn a_stroke_no_tile_fits_is_refused() {
        let set = set();
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let mut g = grid(&index, 4);
        let (b, w) = (index.terrain("B").unwrap(), index.terrain("W").unwrap());
        apply(&mut g, &index, |g| g.paint(&index, &rules, 1, 1, b));
        // W diagonally beside B (no rule for W): no tile has both.
        let before = g.clone();
        assert!(g.paint(&index, &rules, 2, 2, w).is_none());
        assert_eq!(g, before);
    }

    #[test]
    fn heights_settle_within_a_step() {
        let set = set();
        let index = TileIndex::new(&set);
        let a = Corner { terrain: index.terrain("A").unwrap(), height: 0 };
        let mut l = Lattice::new(4, 4, a);
        l.set_corner(2, 2, Corner { height: 3, ..a });
        let mut changed = BTreeSet::new();
        settle(&mut l, vec![(2, 2)], &mut changed);
        assert_eq!(
            heights(&l),
            [[1, 1, 1, 1, 1], [1, 2, 2, 2, 1], [1, 2, 3, 2, 1], [1, 2, 2, 2, 1], [1, 1, 1, 1, 1]]
        );
    }

    #[test]
    fn rule_heights_are_relative() {
        let set = set();
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let (a, b) = (index.terrain("A").unwrap(), index.terrain("B").unwrap());
        // B a step above A: the A corner comes up to B's height.
        let placed = Corner { terrain: b, height: 4 };
        let next = Corner { terrain: a, height: 3 };
        assert_eq!(rules.apply(placed, next), Some(Corner { terrain: a, height: 4 }));
        // Two steps apart: no rule.
        assert_eq!(rules.apply(placed, Corner { terrain: a, height: 2 }), None);
    }

    #[test]
    fn crossers_go_on_the_edges_a_drag_crosses() {
        let set = set();
        let index = TileIndex::new(&set);
        let mut g = grid(&index, 5);
        let (r, s) = (index.crosser("R").unwrap(), index.crosser("S").unwrap());
        let road = |path: &[(u32, u32)]| path_edges(path).unwrap();
        apply(&mut g, &index, |g| g.draw_crosser(&index, &road(&[(1, 2), (2, 2), (3, 2)]), &[], r));
        let row: Vec<Option<Crosser>> =
            (0..5).map(|x| g.lattice.cell(x, 2).edges[crate::WEST]).collect();
        assert_eq!(row, [None, None, Some(r), Some(r), None]);
        // A stream across the road's middle: the crossing tile.
        apply(&mut g, &index, |g| g.draw_crosser(&index, &road(&[(2, 1), (2, 2), (2, 3)]), &[], s));
        assert_eq!(g.lattice.cell(2, 2).edges, [Some(s), Some(r), Some(s), Some(r)]);
        // A road turning a corner: no tile, refused.
        assert!(g.draw_crosser(&index, &road(&[(3, 2), (3, 3)]), &[], r).is_none());
        // Erasing the road's west end: the crossing loses its road on that
        // side, has nothing to fit, and drops the stream (another kind)
        // rather than the road, whose east end is left.
        apply(&mut g, &index, |g| g.erase(&index, 1, 2));
        assert_eq!(g.lattice.cell(2, 2).edges, [None, Some(r), None, None]);
        assert_eq!(g.lattice.cell(2, 1).edges, [None; 4]);
        assert_eq!(g.lattice.cell(3, 2).edges, [None, None, None, Some(r)]);
    }
}
