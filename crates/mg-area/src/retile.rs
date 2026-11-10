//! Change Tileset: an area's tiles laid anew from another tileset, the
//! rest of the area as it was (its objects, sounds, lighting, variables,
//! scripts and size).
//!
//! The old tiles' corners and edges are read as a lattice; each terrain
//! and crosser is mapped to the new tileset's (by name where both have it,
//! else as the builder chooses), and the result is painted into an area
//! of the new tileset's `Default` terrain as a builder would paint it, the
//! new tileset's placement rules fitting the tiles around: terrain first,
//! then heights a step at a time (where the new tileset has height
//! transitions), then crossers, laid as far as tiles fit them. What can't
//! be laid is counted and said; a group's tiles (a building, a special
//! piece) are terrain like the rest. The doors on the old tiles' typed
//! hooks go and the new tiles bring their own, as painting does.

use mg_core::ResRef;
use mg_edit::{Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_set::Tileset;
use mg_tiles::paint::{Grid, Quarter, Rules};
use mg_tiles::{Corner, Crosser, Lattice, Placement, Terrain, TileIndex};

/// A tileset and its index.
#[derive(Debug, Clone, Copy)]
pub struct Set<'a> {
    pub resref: ResRef,
    pub set: &'a Tileset,
    pub index: &'a TileIndex,
}

/// Each of the old tileset's terrains and crossers the area has, by name
/// (lower case), with the new tileset's it becomes (`None`: a crosser left
/// out).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mapping {
    pub terrains: Vec<(String, String)>,
    pub crossers: Vec<(String, Option<String>)>,
}

impl Mapping {
    /// What the area holds of the old tileset, each mapped to the new
    /// tileset's of the same name (any case), else a terrain to the new
    /// tileset's `Default` and a crosser to none.
    pub fn of(old: &Set<'_>, new: &Set<'_>, are: &Struct) -> Option<Mapping> {
        let grid = crate::terrain::grid(are, old.index)?;
        let l = &grid.lattice;
        let mut terrains: Vec<Terrain> = Vec::new();
        let mut crossers: Vec<Crosser> = Vec::new();
        for y in 0..=l.height() {
            for x in 0..=l.width() {
                let t = l.corner(x, y).terrain;
                if !terrains.contains(&t) {
                    terrains.push(t);
                }
            }
        }
        for y in 0..l.height() {
            for x in 0..l.width() {
                for c in l.cell(x, y).edges.into_iter().flatten() {
                    if !crossers.contains(&c) {
                        crossers.push(c);
                    }
                }
            }
        }
        let default = new.set.general.default.to_ascii_lowercase();
        let same = |name: &str, names: Vec<String>| {
            names.into_iter().find(|n| n.eq_ignore_ascii_case(name))
        };
        let new_terrains =
            || new.index.terrains().map(|t| new.index.terrain_name(t).to_ascii_lowercase());
        let new_crossers =
            || new.index.crossers().map(|c| new.index.crosser_name(c).to_ascii_lowercase());
        let mut out = Mapping::default();
        for t in terrains {
            let name = old.index.terrain_name(t).to_ascii_lowercase();
            let to = same(&name, new_terrains().collect()).unwrap_or_else(|| default.clone());
            out.terrains.push((name, to));
        }
        for c in crossers {
            let name = old.index.crosser_name(c).to_ascii_lowercase();
            let to = same(&name, new_crossers().collect());
            out.crossers.push((name, to));
        }
        out.terrains.sort();
        out.crossers.sort();
        Some(out)
    }
}

/// What changing the tileset does: the edits (one command), and what of
/// the old layout the new tileset couldn't lay.
#[derive(Debug, Clone, PartialEq)]
pub struct Retiled {
    pub edits: Vec<Edit>,
    /// Corners whose terrain or height the new tiles don't have.
    pub corners_missed: usize,
    /// Edges whose crosser (mapped to one) the new tiles don't have.
    pub crossers_missed: usize,
    /// The old tileset's groups placed in the area (laid as terrain).
    pub groups: usize,
    /// Doors taken away with the old tiles' hooks, and brought by the new.
    pub doors_gone: usize,
    pub doors_new: usize,
}

/// The area (its ARE `are`, GIT `git`) laid in the tileset `new` as its
/// layout in `old` maps (`mapping`); `lights` gives each new tile's lights
/// where the old tile had none to keep; `blueprint` reads a door blueprint.
/// `None` when the area's tiles are not the old tileset's, or the new
/// tileset has no `Default` terrain tile.
#[allow(clippy::too_many_arguments)]
pub fn retile(
    game: &GameData,
    are_key: ResKey,
    git_key: ResKey,
    are: &Struct,
    git: &Struct,
    old: &Set<'_>,
    new: &Set<'_>,
    mapping: &Mapping,
    blueprint: &dyn Fn(ResRef) -> Option<Struct>,
    rng: &mut fastrand::Rng,
) -> Option<Retiled> {
    let before = crate::terrain::grid(are, old.index)?;
    let (w, h) = (before.lattice.width(), before.lattice.height());
    // The layout, in the new tileset's terrains and crossers.
    let default = new.index.terrain(&new.set.general.default)?;
    let terrain = |t: Terrain| {
        let name = old.index.terrain_name(t);
        let to = mapping.terrains.iter().find(|(o, _)| o.eq_ignore_ascii_case(name));
        to.and_then(|(_, n)| new.index.terrain(n)).unwrap_or(default)
    };
    let crosser = |c: Crosser| {
        let name = old.index.crosser_name(c);
        let to = mapping.crossers.iter().find(|(o, _)| o.eq_ignore_ascii_case(name));
        to.and_then(|(_, n)| n.as_deref()).and_then(|n| new.index.crosser(n))
    };
    let heights = new.set.general.has_height_transition;
    let mut target = Lattice::new(w, h, Corner { terrain: default, height: 0 });
    for y in 0..=h {
        for x in 0..=w {
            let c = before.lattice.corner(x, y);
            let height = if heights { c.height.max(0) } else { 0 };
            target.set_corner(x, y, Corner { terrain: terrain(c.terrain), height });
        }
    }
    // An area of the Default terrain, painted as the layout has it.
    let plain = Lattice::new(w, h, Corner { terrain: default, height: 0 });
    let tiles = mg_tiles::fill(new.index, &plain, rng).ok()?;
    let mut grid = Grid::new(new.index, w, h, tiles)?;
    let rules = Rules::new(new.index, new.set);
    // Terrain: twice over, as a rule painting one corner can change a
    // neighbor painted before it.
    for _ in 0..2 {
        for y in 0..=h {
            for x in 0..=w {
                let want = target.corner(x, y).terrain;
                if grid.lattice.corner(x, y).terrain == want {
                    continue;
                }
                if let Some(s) = grid.paint(new.index, &rules, x, y, want) {
                    grid.apply(new.index, s, rng);
                }
            }
        }
    }
    // Heights, a step at a time, as Raise does.
    let top = (0..=h).flat_map(|y| (0..=w).map(move |x| (x, y)));
    let highest = top.map(|(x, y)| target.corner(x, y).height).max().unwrap_or(0);
    for level in 1..=highest {
        for y in 0..=h {
            for x in 0..=w {
                let low = grid.lattice.corner(x, y).height < level;
                if target.corner(x, y).height >= level
                    && low
                    && let Some(s) = grid.raise(new.index, &rules, x, y, true)
                {
                    grid.apply(new.index, s, rng);
                }
            }
        }
    }
    // Crossers: each one's edges laid together, as far as tiles fit them.
    let mut edges: Vec<(Crosser, Quarter)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let cell = before.lattice.cell(x, y);
            let mut sides = vec![mg_tiles::SOUTH, mg_tiles::WEST];
            if y + 1 == h {
                sides.push(mg_tiles::NORTH);
            }
            if x + 1 == w {
                sides.push(mg_tiles::EAST);
            }
            for side in sides {
                if let Some(c) = cell.edges[side].and_then(crosser) {
                    edges.push((c, ((x, y), side)));
                }
            }
        }
    }
    let mut crossers: Vec<Crosser> = edges.iter().map(|(c, _)| *c).collect();
    crossers.sort();
    crossers.dedup();
    let mut crossers_missed = 0;
    for c in crossers {
        let mine: Vec<Quarter> = edges.iter().filter(|(e, _)| *e == c).map(|(_, at)| *at).collect();
        let (stroke, held) = grid.lay_crosser(new.index, &mine, &[], c);
        crossers_missed += match stroke {
            Some(s) => {
                grid.apply(new.index, s, rng);
                held.len()
            }
            None => mine.len(),
        };
    }
    let mut corners_missed = 0;
    for y in 0..=h {
        for x in 0..=w {
            let (got, want) = (grid.lattice.corner(x, y), target.corner(x, y));
            if got.terrain != want.terrain || got.height != want.height {
                corners_missed += 1;
            }
        }
    }
    // The tile list: the new tiles, each with the old one's lights.
    let old_list = are.list("Tile_List").unwrap_or(&[]);
    let list: Vec<Struct> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let p = grid.tile(x, y);
            let old_tile = old_list.get(y as usize * w as usize + x as usize);
            let byte =
                |l: &str| old_tile.and_then(|t| t.integer(l)).unwrap_or(0).clamp(0, 255) as u8;
            let lights = ["Tile_MainLight1", "Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2"]
                .map(byte);
            crate::terrain::tile_struct(new.set, p, lights)
        })
        .collect();
    let set_field = |key: ResKey, label: &str, value: Value| Edit::SetField {
        key,
        path: GffPath::root(),
        label: label.into(),
        value: Some(value),
    };
    let mut edits = vec![
        set_field(are_key, "Tileset", Value::resref(new.resref)),
        set_field(are_key, "Tile_List", Value::List(list)),
    ];
    // Doors: those on the old tiles' typed hooks go; the new tiles bring
    // their own.
    let old_step = old.set.general.transition;
    let doors = git.list("Door List").unwrap_or(&[]);
    let on_hooks: Vec<glam::Vec3> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|c| crate::terrain::typed_hooks(old.set, c, before.tile(c.0, c.1), old_step))
        .map(|(_, at, _)| at)
        .collect();
    let kept: Vec<Struct> = doors
        .iter()
        .filter(|d| {
            let (dx, dy) = (d.float("X").unwrap_or(0.0), d.float("Y").unwrap_or(0.0));
            !on_hooks.iter().any(|at| (dx - at.x).abs() < 0.05 && (dy - at.y).abs() < 0.05)
        })
        .cloned()
        .collect();
    let doors_gone = doors.len() - kept.len();
    if doors_gone > 0 {
        edits.push(set_field(git_key, "Door List", Value::List(kept.clone())));
    }
    let mut after = git.clone();
    after.set("Door List", Value::List(kept));
    let none = |_: (u32, u32)| Placement { tile: u32::MAX, orientation: 0, height: 0 };
    let changes: Vec<((u32, u32), Placement)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|c| (c, grid.tile(c.0, c.1)))
        .collect();
    let new_doors = crate::terrain::door_edits(
        game,
        git_key,
        &after,
        new.set,
        new.set.general.transition,
        &none,
        &changes,
        blueprint,
    );
    let doors_new = new_doors.len();
    edits.extend(new_doors);
    let groups = crate::reshape::groups_in(old.index, old.set, &before).len();
    Some(Retiled { edits, corners_missed, crossers_missed, groups, doors_gone, doors_new })
}
