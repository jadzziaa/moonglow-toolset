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
//! - A group (a building of several tiles, a feature of one) is one
//!   thing: no stroke changes a tile of one, no group goes over another,
//!   and the Eraser and Delete take it away whole, unless
//!   [`Grid::free_groups`] (Options › Area, "Allow editing tiles of groups
//!   and features") lets its tiles be edited one by one.
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
#[derive(Debug, Clone)]
pub struct Grid {
    pub lattice: Lattice,
    /// Row-major from the south-west, like `Tile_List`.
    pub tiles: Vec<Placement>,
    /// The cells that refused the stroke asked for last ([`Grid::blocked`]).
    blocked: std::cell::RefCell<Vec<(u32, u32)>>,
    /// Whether a group's tiles may be edited one by one: a stroke that
    /// changes one chooses a terrain tile there, a group placed over part
    /// of another takes that one away whole, and Delete takes a group's
    /// tile alone (as Aurora's does). Off (the default), a group is one
    /// thing: a stroke that would change one of its tiles is refused, so
    /// is a group placed over another (as Aurora refuses it, unless
    /// `replace_groups`), and Delete takes the whole group, as the Eraser
    /// does.
    pub free_groups: bool,
    /// Whether a group placed over part of another takes that one away
    /// whole (as with `free_groups`), the rest of a group still one thing:
    /// the plugin API's `place_group`, documented so.
    pub replace_groups: bool,
}

impl PartialEq for Grid {
    fn eq(&self, other: &Grid) -> bool {
        self.lattice == other.lattice && self.tiles == other.tiles
    }
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
        let blocked = Default::default();
        Some(Grid { lattice, tiles, blocked, free_groups: false, replace_groups: false })
    }

    /// The tile in cell (x, y), which must be the grid's: [`Grid::get_tile`]
    /// where it may not be (a cell from outside, or kept from before the
    /// area changed size).
    pub fn tile(&self, x: u32, y: u32) -> Placement {
        self.get_tile(x, y).unwrap_or_else(|| panic!("cell ({x}, {y}) outside the grid"))
    }

    /// The tile in cell (x, y); `None` outside the grid.
    pub fn get_tile(&self, x: u32, y: u32) -> Option<Placement> {
        let i = self.slot(x, y)?;
        self.tiles.get(i).copied()
    }

    /// Where cell (x, y)'s tile is in the list; `None` outside the grid.
    fn slot(&self, x: u32, y: u32) -> Option<usize> {
        self.lattice.has_cell(x, y).then(|| (y * self.lattice.width() + x) as usize)
    }

    /// Whether the tile in cell (x, y) is a group's, which changes only
    /// with its group (never, with [`Grid::free_groups`] on). A cell
    /// outside the grid is not.
    pub fn locked(&self, index: &TileIndex, x: u32, y: u32) -> bool {
        !self.free_groups && self.get_tile(x, y).is_some_and(|t| index.is_grouped(t.tile))
    }

    /// The groups that `changes` (cells and their new tiles) would leave
    /// part of: each as the cells it has, where it is whole now. A group
    /// they take away whole, or don't touch, is not one of them.
    pub fn groups_broken(
        &self,
        index: &TileIndex,
        changes: &[((u32, u32), Placement)],
    ) -> Vec<Vec<(u32, u32)>> {
        let changed = |c: (u32, u32)| {
            changes.iter().any(|&(at, p)| at == c && Some(p) != self.get_tile(c.0, c.1))
        };
        let mut out: Vec<Vec<(u32, u32)>> = Vec::new();
        for &((x, y), _) in changes {
            if !changed((x, y)) || out.iter().any(|g| g.contains(&(x, y))) {
                continue;
            }
            if let Some(cells) = self.group_cells(index, x, y)
                && !cells.iter().all(|&c| changed(c))
            {
                out.push(cells);
            }
        }
        out
    }

    /// Painting corner (x, y) with `terrain`; `None` when refused, or
    /// outside the lattice.
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
        let mut c = lattice.get_corner(x, y)?;
        c.terrain = terrain;
        lattice.set_corner(x, y, c);
        changed.insert((x, y));
        apply_rules(rules, &mut lattice, x, y, &mut changed);
        self.stroke(index, lattice, changed, around(&self.lattice, x, y))
    }

    /// Raising (`up`) or lowering corner (x, y) a step; `None` when refused
    /// (a cell nothing fits), or outside the lattice. A corner at height 0
    /// stays there: lowering it chooses the tiles around it again.
    pub fn raise(
        &self,
        index: &TileIndex,
        rules: &Rules,
        x: u32,
        y: u32,
        up: bool,
    ) -> Option<Stroke> {
        let mut lattice = self.lattice.clone();
        let mut c = lattice.get_corner(x, y)?;
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
    /// edge, the `cells` given (a click). `None` when refused, or for a
    /// cell outside the grid.
    ///
    /// [`SOUTH`]: crate::SOUTH
    pub fn draw_crosser(
        &self,
        index: &TileIndex,
        edges: &[((u32, u32), usize)],
        click: &[(u32, u32)],
        crosser: Crosser,
    ) -> Option<Stroke> {
        if !self.all_inside(edges.iter().map(|(c, _)| c).chain(click)) {
            return None;
        }
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
    ///
    /// On a tile of a group (a building of several tiles, a feature of
    /// one) it takes the group away whole: each of its tiles is replaced
    /// by one that fits the ground there. What is left of a group painted
    /// over goes a tile at a time. `None` outside the grid, too.
    pub fn erase(&self, index: &TileIndex, x: u32, y: u32) -> Option<Stroke> {
        if index.is_grouped(self.get_tile(x, y)?.tile) {
            let cells = self.group_cells(index, x, y).unwrap_or_else(|| vec![(x, y)]);
            return self.delete(index, &cells);
        }
        self.erase_only(index, x, y, None)
    }

    /// The cells of the group that has a tile at (x, y), where it is
    /// whole ([`TileIndex::group_at`]).
    pub fn group_cells(&self, index: &TileIndex, x: u32, y: u32) -> Option<Vec<(u32, u32)>> {
        let (w, h) = (i64::from(self.lattice.width()), i64::from(self.lattice.height()));
        let tile = |cx: i64, cy: i64| {
            (cx >= 0 && cy >= 0 && cx < w && cy < h).then(|| self.tile(cx as u32, cy as u32))
        };
        index.group_at(&tile, x, y)
    }

    /// [`Grid::erase`] taking from cell (x, y) only crosser `only` (a road,
    /// and not the stream it crosses), if one is given.
    pub fn erase_only(
        &self,
        index: &TileIndex,
        x: u32,
        y: u32,
        only: Option<Crosser>,
    ) -> Option<Stroke> {
        if !self.lattice.has_cell(x, y) {
            return None;
        }
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
        match only {
            None => clear(&mut lattice, (x, y), None, &mut queue),
            Some(only) => {
                let cell = lattice.cell(x, y);
                for edge in (0..4).filter(|&e| cell.edges[e] == Some(only)) {
                    lattice.set_edge(x, y, edge, None);
                    if let Some(n) = beside(&lattice, x, y, edge) {
                        queue.push_back((n, only));
                    }
                }
            }
        }
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
    /// that fits, or is outside the grid.
    pub fn repick(&self, index: &TileIndex, cells: &[(u32, u32)]) -> Option<Stroke> {
        if !self.all_inside(cells) {
            return None;
        }
        self.stroke(index, self.lattice.clone(), BTreeSet::new(), cells.to_vec())
    }

    /// Whether every one of `cells` is the grid's.
    fn all_inside<'a>(&self, cells: impl IntoIterator<Item = &'a (u32, u32)>) -> bool {
        cells.into_iter().all(|&(x, y)| self.lattice.has_cell(x, y))
    }

    /// Delete on selected tiles: each loses its crossers as the Eraser takes
    /// them, and a group's tile is replaced by a tile that fits its place,
    /// the rest of its group too (as the Eraser takes a group), or with
    /// [`Grid::free_groups`] on, alone (the rest of the group stays, as
    /// Aurora's Delete leaves it). `None` when one has nothing to fit, or
    /// is outside the grid.
    pub fn delete(&self, index: &TileIndex, cells: &[(u32, u32)]) -> Option<Stroke> {
        if !self.all_inside(cells) {
            return None;
        }
        let mut grid = self.clone();
        let mut chosen: Vec<(u32, u32)> = Vec::new();
        let mut unlocked = Vec::new();
        // (A group's tile: the whole group, where it is whole.)
        let mut all: Vec<(u32, u32)> = Vec::new();
        let mut seen = BTreeSet::new();
        for &(x, y) in cells {
            let group = if self.free_groups { None } else { self.group_cells(index, x, y) };
            all.extend(
                group.unwrap_or_else(|| vec![(x, y)]).into_iter().filter(|c| seen.insert(*c)),
            );
        }
        // (Refused on the copy: what refused it is this grid's to show.)
        let refused = |at: Vec<(u32, u32)>| {
            *self.blocked.borrow_mut() = at;
            None
        };
        let mut seen = BTreeSet::new();
        for &(x, y) in &all {
            let stroke = if index.is_grouped(grid.tile(x, y).tile) {
                unlocked.push((x, y));
                let mut lattice = grid.lattice.clone();
                let mut also = vec![(x, y)];
                for edge in 0..4 {
                    if lattice.cell(x, y).edges[edge].is_some() {
                        lattice.set_edge(x, y, edge, None);
                        also.extend(beside(&lattice, x, y, edge));
                    }
                }
                grid.stroke_unlocking(index, lattice, BTreeSet::new(), also, &unlocked)
            } else {
                grid.erase(index, x, y)
            };
            let Some(stroke) = stroke else { return refused(grid.blocked()) };
            // Stand-in tiles, so that the next tile sees what this one leaves.
            for &(cx, cy) in &stroke.cells {
                let Some(&fit) = index.fits(&stroke.lattice.cell(cx, cy)).first() else {
                    return refused(vec![(cx, cy)]);
                };
                grid.tiles[(cy * grid.lattice.width() + cx) as usize] = fit;
                if seen.insert((cx, cy)) {
                    chosen.push((cx, cy));
                }
            }
            grid.lattice = stroke.lattice;
        }
        self.blocked.borrow_mut().clear();
        Some(Stroke { lattice: grid.lattice, cells: chosen, fixed: Vec::new() })
    }

    /// Placing tile group `group` of the tileset with its first tile (the
    /// south-west one) in cell (x, y), turned `turns` quarter turns
    /// counter-clockwise about that cell, as Aurora does: the group's tiles
    /// go in at the height of the ground under its first tile, their corners
    /// and edges replace the terrain there (neighbouring corners follow
    /// within a step), and the tiles around choose again; a cell the group
    /// leaves empty (`-1`) is matched like terrain. `None` when the group
    /// does not fit inside the area or a tile around it has nothing to fit.
    /// Over a tile of another group it is refused, as Aurora refuses it
    /// (that group is then [`Grid::blocked`]); with [`Grid::free_groups`]
    /// on, it takes the other away whole.
    pub fn place_group(
        &self,
        index: &TileIndex,
        group: &mg_set::Group,
        x: u32,
        y: u32,
        turns: u8,
    ) -> Option<Stroke> {
        let columns = group.columns.max(1);
        let tiles: Vec<((i64, i64), Option<Placement>)> = group
            .tiles
            .iter()
            .enumerate()
            .map(|(k, tile)| {
                let (c, r) = crate::group_slot(k, columns, turns);
                let p = tile.map(|t| Placement { tile: t, orientation: turns % 4, height: 0 });
                ((c, r), p)
            })
            .collect();
        self.place_tiles(index, &tiles, x, y)
    }

    /// Placing a block of tiles as a group goes in (pasted tiles too):
    /// each at its offset from cell (x, y), raised by the height of the
    /// ground under that cell; `None` offsets are matched like terrain.
    /// `None` when refused, or for a cell outside the grid.
    pub fn place_tiles(
        &self,
        index: &TileIndex,
        tiles: &[((i64, i64), Option<Placement>)],
        x: u32,
        y: u32,
    ) -> Option<Stroke> {
        let (w, h) = (self.lattice.width() as i64, self.lattice.height() as i64);
        let base = self.lattice.get_cell(x, y)?.corners.iter().map(|c| c.height).min().unwrap_or(0);
        let mut lattice = self.lattice.clone();
        let mut fixed = Vec::new();
        let mut empty = Vec::new();
        for &((c, r), tile) in tiles {
            let (cx, cy) = (x as i64 + c, y as i64 + r);
            if cx < 0 || cy < 0 || cx >= w || cy >= h {
                return None;
            }
            let cell = (cx as u32, cy as u32);
            match tile {
                Some(t) => {
                    let p = Placement { height: t.height + base, ..t };
                    lattice.set_cell(cell.0, cell.1, &index.cell(p)?);
                    fixed.push((cell, p));
                }
                None => empty.push(cell),
            }
        }
        let placed: Vec<(u32, u32)> = fixed.iter().map(|(c, _)| *c).collect();
        // Over a group's tiles: refused, those groups in the way.
        if !self.free_groups && !self.replace_groups {
            let mut covered: Vec<(u32, u32)> = Vec::new();
            for &(cx, cy) in placed.iter().chain(&empty) {
                if index.is_grouped(self.tile(cx, cy).tile) {
                    let group = self.group_cells(index, cx, cy).unwrap_or_else(|| vec![(cx, cy)]);
                    for c in group {
                        if !covered.contains(&c) {
                            covered.push(c);
                        }
                    }
                }
            }
            if !covered.is_empty() {
                *self.blocked.borrow_mut() = covered;
                return None;
            }
        }
        // (Allowed:) a group under what is placed goes whole: its tiles
        // outside the new ones' cells are chosen again to fit the ground
        // (half a building isn't left standing beside the new one).
        let mut evicted: Vec<(u32, u32)> = Vec::new();
        for &(cx, cy) in placed.iter().chain(&empty) {
            for c in self.group_cells(index, cx, cy).unwrap_or_default() {
                if !placed.contains(&c) && !evicted.contains(&c) {
                    evicted.push(c);
                }
            }
        }
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
        let empty_cells = empty.clone();
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
        for c in &evicted {
            if !cells.contains(c) {
                cells.push(*c);
            }
        }
        // (The cells the new group leaves empty are free to change, too.)
        evicted.extend(empty_cells);
        let mut stroke = self.stroke_unlocking(index, lattice, BTreeSet::new(), cells, &evicted)?;
        stroke.fixed = fixed;
        Some(stroke)
    }

    /// Putting tile `p` in cell (x, y) exactly as it is given, whether it
    /// fits its neighbours or not (the tiles palette's Exact tile): no
    /// other tile, corner terrain, height or crosser around it changes,
    /// and its own corners and edges are what the tile has. `None` on a
    /// tile of a group or feature, which changes only with it unless
    /// [`Grid::free_groups`] is on (that group is then
    /// [`Grid::blocked`]), or for a tile the tileset hasn't, or a cell
    /// outside the grid.
    pub fn set_exact(&self, index: &TileIndex, x: u32, y: u32, p: Placement) -> Option<Stroke> {
        if !self.lattice.has_cell(x, y) {
            return None;
        }
        if self.locked(index, x, y) {
            *self.blocked.borrow_mut() =
                self.group_cells(index, x, y).unwrap_or_else(|| vec![(x, y)]);
            return None;
        }
        self.blocked.borrow_mut().clear();
        let cell = index.cell(p)?;
        let mut lattice = self.lattice.clone();
        lattice.set_cell(x, y, &cell);
        Some(Stroke { lattice, cells: Vec::new(), fixed: vec![((x, y), p)] })
    }

    /// The cells that refused the stroke asked of this grid last (a brush
    /// that gave `None`): each has no tile that fits what the stroke would
    /// make of it, or holds a group's tile the stroke would not keep.
    /// Empty when the last stroke was not refused there.
    pub fn blocked(&self) -> Vec<(u32, u32)> {
        self.blocked.borrow().clone()
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
        self.stroke_unlocking(index, lattice, changed, also, &[])
    }

    /// [`Grid::stroke`], choosing again the group tiles of `unlocked` too
    /// (and with [`Grid::free_groups`] on, those the change doesn't keep).
    fn stroke_unlocking(
        &self,
        index: &TileIndex,
        lattice: Lattice,
        changed: BTreeSet<(u32, u32)>,
        also: Vec<(u32, u32)>,
        unlocked: &[(u32, u32)],
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
        let mut blocked = Vec::new();
        for &(x, y) in &cells {
            let cell = lattice.cell(x, y);
            let tile = self.tile(x, y);
            let grouped = index.is_grouped(tile.tile) && !unlocked.contains(&(x, y));
            let refuses = if grouped && index.cell(tile) == Some(cell) {
                false
            } else if grouped && !self.free_groups {
                true
            } else {
                index.fits(&cell).is_empty()
            };
            if refuses {
                blocked.push((x, y));
            }
        }
        let refused = !blocked.is_empty();
        *self.blocked.borrow_mut() = blocked;
        if refused {
            return None;
        }
        // Group tiles that still fit stay.
        cells.retain(|&(x, y)| {
            let tile = self.tile(x, y);
            !index.is_grouped(tile.tile)
                || unlocked.contains(&(x, y))
                || index.cell(tile) != Some(lattice.cell(x, y))
        });
        Some(Stroke { lattice, cells, fixed: Vec::new() })
    }

    /// Applies a stroke, choosing each of its cells' tiles at random among
    /// those that fit. Returns the cells that changed and their new tiles.
    /// A stroke worked out on a grid of another size changes nothing.
    pub fn apply(
        &mut self,
        index: &TileIndex,
        stroke: Stroke,
        rng: &mut fastrand::Rng,
    ) -> Vec<((u32, u32), Placement)> {
        let size = |l: &Lattice| (l.width(), l.height());
        if size(&stroke.lattice) != size(&self.lattice) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for &((x, y), p) in &stroke.fixed {
            let Some(i) = self.slot(x, y) else { continue };
            self.tiles[i] = p;
            out.push(((x, y), p));
        }
        for &(x, y) in &stroke.cells {
            let (Some(i), Some(cell)) = (self.slot(x, y), stroke.lattice.get_cell(x, y)) else {
                continue;
            };
            let fits = index.fits(&cell);
            if fits.is_empty() {
                continue;
            }
            let p = fits[rng.usize(..fits.len())];
            self.tiles[i] = p;
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
    step_fit(index, cell, current, false)
}

/// The tiles that fit a cell, in the order [`next_fit`] steps through
/// them.
pub fn fits_in_order(index: &TileIndex, cell: &Cell) -> Vec<Placement> {
    let mut fits = index.fits(cell);
    fits.sort_by_key(|p| (p.tile, p.orientation, p.height));
    fits
}

/// [`next_fit`], or with `back` the one before `current` (the last from
/// the first): a step back without going all the way round.
pub fn step_fit(
    index: &TileIndex,
    cell: &Cell,
    current: Placement,
    back: bool,
) -> Option<Placement> {
    let fits = fits_in_order(index, cell);
    let n = fits.len();
    let at = fits.iter().position(|p| *p == current);
    match (at, back) {
        (Some(i), false) => fits.get((i + 1) % n).copied(),
        (Some(i), true) => fits.get((i + n - 1) % n).copied(),
        (None, false) => fits.first().copied(),
        (None, true) => fits.last().copied(),
    }
}

/// The cell's next appearance: the next other tile (by its index in the
/// tileset) among those that fit, turned and raised as `current` is where
/// it fits so, else as it first fits; with `back`, the one before. `None`
/// when no other tile fits the cell. (Its turns are [`turn_fit`]'s.)
pub fn step_appearance(
    index: &TileIndex,
    cell: &Cell,
    current: Placement,
    back: bool,
) -> Option<Placement> {
    let fits = fits_in_order(index, cell);
    let mut tiles: Vec<u32> = fits.iter().map(|p| p.tile).collect();
    tiles.dedup();
    let n = tiles.len();
    let next = match tiles.iter().position(|t| *t == current.tile) {
        Some(_) if n < 2 => return None,
        Some(i) if back => tiles[(i + n - 1) % n],
        Some(i) => tiles[(i + 1) % n],
        None if back => *tiles.last()?,
        None => *tiles.first()?,
    };
    let of = |keep: &dyn Fn(&Placement) -> bool| fits.iter().find(|p| p.tile == next && keep(p));
    of(&|p| p.orientation == current.orientation && p.height == current.height)
        .or_else(|| of(&|p| p.orientation == current.orientation))
        .or_else(|| of(&|_| true))
        .copied()
}

/// `current` turned a quarter counter-clockwise (with `back`, clockwise)
/// where it still fits the cell, else the turn after that does (a half
/// turn, for a tile two of whose sides match); the same tile, never
/// another. `None` when it fits there only as it lies: turned, its
/// corners and edges would no longer match its neighbours'. (A group's
/// tile, which fits no cell as terrain, turns where it still matches its
/// cell, at its height.)
pub fn turn_fit(
    index: &TileIndex,
    cell: &Cell,
    current: Placement,
    back: bool,
) -> Option<Placement> {
    if index.is_grouped(current.tile) {
        return (1..4u8).find_map(|k| {
            let turns = if back { 4 - k } else { k };
            let p = Placement { orientation: (current.orientation % 4 + turns) % 4, ..current };
            (index.cell(p).as_ref() == Some(cell)).then_some(p)
        });
    }
    let fits = index.fits(cell);
    (1..4u8).find_map(|k| {
        let turns = if back { 4 - k } else { k };
        let orientation = (current.orientation % 4 + turns) % 4;
        let of = |keep: &dyn Fn(&Placement) -> bool| {
            fits.iter().find(|p| p.tile == current.tile && p.orientation == orientation && keep(p))
        };
        of(&|p| p.height == current.height).or_else(|| of(&|_| true)).copied()
    })
}

/// Cells of open ground around a group placed by [`group_placed`]: room
/// for the tiles around it that its placement changes.
pub const GROUP_MARGIN: u32 = 3;

/// A group as it stands once placed, unturned, in open ground (the
/// tileset's `Default` terrain): its tiles and the tiles around it that
/// its placement changes (the castle around a castle's gate, whose
/// corners are castle), each cell with its tile, the group's first tile
/// in cell ([`GROUP_MARGIN`], [`GROUP_MARGIN`]). `None` where it cannot
/// be placed there.
pub fn group_placed(
    index: &TileIndex,
    set: &Tileset,
    group: usize,
) -> Option<Vec<((u32, u32), Placement)>> {
    let g = set.groups.get(group)?;
    let (w, h) = (g.columns.max(1) + 2 * GROUP_MARGIN, g.rows.max(1) + 2 * GROUP_MARGIN);
    let ground = Corner { terrain: index.terrain(&set.general.default)?, height: 0 };
    let lattice = Lattice::new(w, h, ground);
    let tiles = crate::fill(index, &lattice, &mut fastrand::Rng::with_seed(0)).ok()?;
    let mut grid = Grid::new(index, w, h, tiles)?;
    let stroke = grid.place_group(index, g, GROUP_MARGIN, GROUP_MARGIN, 0)?;
    // (The cells around whose terrain changes; the rest stay open ground.)
    let changed: BTreeSet<(u32, u32)> = (stroke.fixed.iter().map(|&(cell, _)| cell))
        .chain(
            stroke
                .cells
                .iter()
                .copied()
                .filter(|&(x, y)| stroke.lattice.cell(x, y) != grid.lattice.cell(x, y)),
        )
        .collect();
    grid.apply(index, stroke, &mut fastrand::Rng::with_seed(0));
    Some(changed.into_iter().map(|(x, y)| ((x, y), grid.tile(x, y))).collect())
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

    /// A group whose corners are another terrain than open ground's is
    /// pictured with the tiles around it that its placement changes: a
    /// castle's gate with the castle around it (a builder's report: the
    /// Gallery showed the gate alone).
    #[test]
    fn a_group_is_placed_with_the_tiles_it_changes_around_it() {
        let mut set = set();
        // (Tile 15: B at every corner.)
        let gate = mg_set::Group {
            name: "Gate".into(),
            strref: Default::default(),
            rows: 1,
            columns: 1,
            tiles: vec![Some(15)],
        };
        set.groups.push(gate);
        let index = TileIndex::new(&set);
        let placed = group_placed(&index, &set, 0).expect("placed");
        let m = GROUP_MARGIN;
        assert!(placed.contains(&((m, m), Placement { tile: 15, orientation: 0, height: 0 })));
        let cells: Vec<(u32, u32)> = placed.iter().map(|&(c, _)| c).collect();
        let around: Vec<(u32, u32)> =
            (m - 1..=m + 1).flat_map(|y| (m - 1..=m + 1).map(move |x| (x, y))).collect();
        assert_eq!(
            cells,
            around.iter().copied().collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>()
        );
        // A group of open ground's own terrain changes nothing around it.
        set.groups[0].tiles = vec![Some(0)];
        let placed = group_placed(&index, &set, 0).expect("placed");
        assert_eq!(placed.iter().map(|&(c, _)| c).collect::<Vec<_>>(), [(m, m)]);
        assert!(group_placed(&index, &set, 5).is_none());
    }

    /// A group is one thing: nothing changes a tile of it, no group goes
    /// over it, Delete takes it whole; with `free_groups`, each of these
    /// may break it, and `groups_broken` says which.
    #[test]
    fn groups_are_one_thing_unless_freed() {
        let mut set = set();
        // (A pond two tiles long, of open ground's terrain, A: tiles of
        // its own, so that open ground has others when it goes.)
        let n = set.tiles.len() as u32;
        let mut pond = set.tiles[0].clone();
        pond.model = "pond".into();
        set.tiles.extend([pond.clone(), pond]);
        set.groups.push(mg_set::Group {
            name: "Pond".into(),
            strref: Default::default(),
            rows: 1,
            columns: 2,
            tiles: vec![Some(n), Some(n + 1)],
        });
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let pond = set.groups[0].clone();
        let mut g = grid(&index, 6);
        apply(&mut g, &index, |g| g.place_group(&index, &pond, 1, 1, 0));
        assert_eq!(g.group_cells(&index, 2, 1), Some(vec![(1, 1), (2, 1)]));
        assert!(g.locked(&index, 1, 1) && !g.locked(&index, 3, 1));
        // Another over its east half: refused, the pond in the way.
        assert!(g.place_group(&index, &pond, 2, 1, 0).is_none());
        assert_eq!(g.blocked(), [(1, 1), (2, 1)]);
        // B painted at a corner of it would make its W into A: refused.
        let b = index.terrain("B").unwrap();
        assert!(g.paint(&index, &rules, 2, 1, b).is_none());
        assert!(g.blocked().contains(&(1, 1)), "{:?}", g.blocked());
        // Painted beside it, where its tiles still fit: they stay.
        let beside = g.paint(&index, &rules, 4, 4, b).expect("beside the pond");
        assert!(!beside.cells.contains(&(1, 1)) && !beside.cells.contains(&(2, 1)));
        // Delete on one of its tiles: all of it.
        let deleted = g.delete(&index, &[(2, 1)]).unwrap();
        assert!(deleted.cells.contains(&(1, 1)) && deleted.cells.contains(&(2, 1)));
        // A pond tile turns where it still matches its cell (all W: any way).
        let p = g.tile(1, 1);
        let turned = turn_fit(&index, &g.lattice.cell(1, 1), p, false);
        assert_eq!(turned, Some(Placement { orientation: (p.orientation + 1) % 4, ..p }));

        // Freed: a tile of it deleted alone, and the change breaks it.
        let mut free = g.clone();
        free.free_groups = true;
        assert!(!free.locked(&index, 1, 1));
        let deleted = free.delete(&index, &[(2, 1)]).unwrap();
        assert!(!deleted.cells.contains(&(1, 1)) && deleted.cells.contains(&(2, 1)));
        let changes = vec![((2, 1), Placement { tile: 0, orientation: 0, height: 0 })];
        assert_eq!(free.groups_broken(&index, &changes), [vec![(1, 1), (2, 1)]]);
        // Both its tiles changed (the Eraser's way): not broken, gone.
        let both = vec![changes[0], ((1, 1), Placement { tile: 0, orientation: 0, height: 0 })];
        assert!(free.groups_broken(&index, &both).is_empty());
        // Painting at its corner chooses terrain tiles in its place.
        assert!(free.paint(&index, &rules, 2, 1, b).is_some());
        // A pond over its east half takes the first away whole.
        apply(&mut free, &index, |g| g.place_group(&index, &pond, 2, 1, 0));
        assert_eq!(free.group_cells(&index, 3, 1), Some(vec![(2, 1), (3, 1)]));
        assert!(!index.is_grouped(free.tile(1, 1).tile));
    }

    /// An exact tile goes in as it is given: its cell alone changes, a tile
    /// that doesn't fit its neighbours included, raised or turned as asked;
    /// not over a group's tile unless they are freed.
    #[test]
    fn an_exact_tile_changes_its_cell_alone() {
        let mut set = set();
        let n = set.tiles.len() as u32;
        let mut pond = set.tiles[0].clone();
        pond.model = "pond".into();
        set.tiles.push(pond);
        set.groups.push(mg_set::Group {
            name: "Pond".into(),
            strref: Default::default(),
            rows: 1,
            columns: 1,
            tiles: vec![Some(n)],
        });
        let index = TileIndex::new(&set);
        let mut g = grid(&index, 5);
        let pond = set.groups[0].clone();
        apply(&mut g, &index, |g| g.place_group(&index, &pond, 4, 4, 0));
        let before = g.clone();
        // Tile 15, B at every corner, in open ground of A: it fits none of
        // its neighbours. Turned and two steps up.
        let p = Placement { tile: 15, orientation: 3, height: 2 };
        let s = g.set_exact(&index, 2, 1, p).expect("placed");
        assert!(s.cells.is_empty() && s.fixed == [((2, 1), p)]);
        let changes = g.apply(&index, s, &mut fastrand::Rng::with_seed(3));
        assert_eq!(changes, [((2, 1), p)]);
        for (i, (now, was)) in g.tiles.iter().zip(&before.tiles).enumerate() {
            if i != 5 + 2 {
                assert_eq!(now, was, "tile {i} is as it was");
            }
        }
        assert_eq!(g.tile(2, 1), p);
        // The area read back: the tile as put, its neighbours too.
        let back = Grid::new(&index, 5, 5, g.tiles.clone()).unwrap();
        assert_eq!(back.tiles, g.tiles);
        // Not over the pond's tile, which flashes; freed, it goes.
        assert!(g.set_exact(&index, 4, 4, p).is_none());
        assert_eq!(g.blocked(), [(4, 4)]);
        g.free_groups = true;
        assert!(g.set_exact(&index, 4, 4, p).is_some());
        // A tile the tileset hasn't: refused.
        assert!(g.set_exact(&index, 0, 0, Placement { tile: 999, ..p }).is_none());
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

    /// A corner or cell past the grid's edge (one kept from a larger area,
    /// GitHub issue 31) is refused by every brush, not a panic.
    #[test]
    fn brushes_outside_the_grid_are_refused() {
        let set = set();
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let g = grid(&index, 4);
        let (b, r) = (index.terrain("B").unwrap(), index.crosser("R").unwrap());
        // Corners run to 4, cells to 3: (2, 5) and (4, 4) are past them.
        assert!(g.paint(&index, &rules, 2, 5, b).is_none());
        assert!(g.paint(&index, &rules, 5, 0, b).is_none());
        assert!(g.paint(&index, &rules, 4, 4, b).is_some(), "the far corner is the lattice's");
        assert!(g.raise(&index, &rules, 2, 5, true).is_none());
        assert!(g.raise(&index, &rules, 2, 5, false).is_none());
        assert!(g.erase(&index, 4, 4).is_none());
        assert!(g.erase_only(&index, 1, 4, Some(r)).is_none());
        assert!(g.repick(&index, &[(1, 1), (1, 6)]).is_none());
        assert!(g.delete(&index, &[(1, 6)]).is_none());
        assert!(g.draw_crosser(&index, &[], &[(4, 0)], r).is_none());
        assert!(
            g.draw_crosser(&index, &[((3, 3), crate::NORTH), ((3, 4), crate::SOUTH)], &[], r)
                .is_none()
        );
        let p = g.tile(1, 1);
        assert!(g.place_tiles(&index, &[((0, 0), Some(p))], 4, 1).is_none());
        assert!(g.set_exact(&index, 1, 4, p).is_none());
        assert!(g.group_cells(&index, 0, 9).is_none());
        // (A cell past the east edge is not the next row's first.)
        assert_eq!((g.get_tile(4, 0), g.get_tile(0, 4)), (None, None));
        assert_eq!(g.get_tile(3, 3), Some(g.tiles[15]));
        assert!(!g.locked(&index, 9, 9));
        // A stroke worked out on another grid's size changes nothing.
        let mut small = grid(&index, 2);
        let s = g.paint(&index, &rules, 3, 3, b).unwrap();
        assert!(small.apply(&index, s, &mut fastrand::Rng::with_seed(4)).is_empty());
        assert_eq!(small, grid(&index, 2));
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
        // What refused it: the cell that would have B and W both.
        assert_eq!(g.blocked(), [(1, 1)]);
        // A stroke that is taken leaves nothing refused.
        assert!(g.paint(&index, &rules, 3, 3, b).is_some());
        assert!(g.blocked().is_empty());
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
        // Delete on a tile without a group is the Eraser.
        let deleted = g.delete(&index, &[(1, 2)]).unwrap();
        assert_eq!(deleted.lattice, g.erase(&index, 1, 2).unwrap().lattice);
        // Erasing the road's west end: the crossing loses its road on that
        // side, has nothing to fit, and drops the stream (another kind)
        // rather than the road, whose east end is left.
        apply(&mut g, &index, |g| g.erase(&index, 1, 2));
        assert_eq!(g.lattice.cell(2, 2).edges, [None, Some(r), None, None]);
        assert_eq!(g.lattice.cell(2, 1).edges, [None; 4]);
        assert_eq!(g.lattice.cell(3, 2).edges, [None, None, None, Some(r)]);
    }

    #[test]
    fn appearances_step_through_other_tiles_and_turns_keep_the_tile() {
        let set = set();
        let index = TileIndex::new(&set);
        let g = grid(&index, 3);
        // Open ground: two tiles fit (all-A twice over in the set), each
        // at every turn.
        let open = g.lattice.cell(1, 1);
        let now = Placement { tile: 0, orientation: 1, height: 0 };
        let next = step_appearance(&index, &open, now, false).unwrap();
        assert_eq!(next, Placement { tile: 16, ..now }, "another tile, turned as this one");
        assert_eq!(step_appearance(&index, &open, next, false), Some(now), "and round again");
        assert_eq!(step_appearance(&index, &open, next, true), Some(now));
        let turned = turn_fit(&index, &open, now, false).unwrap();
        assert_eq!(turned, Placement { orientation: 2, ..now });
        assert_eq!(turn_fit(&index, &open, now, true), Some(Placement { orientation: 0, ..now }));
        let round = (0..4).fold(now, |p, _| turn_fit(&index, &open, p, false).unwrap());
        assert_eq!(round, now, "four quarter turns");
        // One B corner: other tiles fit, each only as it lies; none turns.
        let b = index.terrain("B").unwrap();
        let mut l = g.lattice.clone();
        l.set_corner(1, 1, Corner { terrain: b, height: 0 });
        let corner = l.cell(1, 1);
        let fits = index.fits(&corner);
        assert!(fits.len() > 1, "{fits:?}");
        for p in &fits {
            assert_eq!(turn_fit(&index, &corner, *p, false), None, "{p:?} fits only as it lies");
            let other = step_appearance(&index, &corner, *p, false).unwrap();
            assert_ne!(other.tile, p.tile);
            assert!(fits.contains(&other));
        }
        // A tile alone in its cell has no other appearance.
        let r = index.crosser("R").unwrap();
        let mut l = g.lattice.clone();
        l.set_edge(1, 1, crate::NORTH, Some(r));
        let end = l.cell(1, 1);
        let only = index.fits(&end);
        assert_eq!(only.len(), 1, "{only:?}");
        assert_eq!(step_appearance(&index, &end, only[0], false), None);
        assert_eq!(turn_fit(&index, &end, only[0], false), None);
    }
}
