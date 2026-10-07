//! `ctx.terrain`: areas made and their terrain painted as the area
//! editor's brushes do it (`mg_tiles::paint`), the tiles chosen by
//! Moonglow: a corner's terrain, its height, and the tileset's groups.
//!
//! Cells and corners count from the area's south-west, from 0: cell
//! (x, y) has corners (x, y) to (x + 1, y + 1), so an area `width` cells
//! across has corners 0 to `width`.

use std::rc::Rc;

use mg_core::{ResRef, ResType};
use mg_edit::terrain::{door_edits, grid, tile_edits, typed_hooks};
use mg_gff::{Gff, Struct};
use mg_module::new::{AreaSpec, Scheme};
use mg_resman::ResKey;
use mg_set::Tileset;
use mg_tiles::paint::{Grid, Rules, Stroke};
use mg_tiles::{Placement, TileIndex};
use mlua::{Lua, Table};

use crate::runtime::{CODEPAGE, Shared, This, fail};

/// The names of what an area's terrain has, as the reference writes them.
pub(crate) const NAMES: [&str; 17] = [
    "area.resref",
    "area.tileset",
    "area.width",
    "area.height",
    "area.step",
    "area.terrains",
    "area.groups",
    "area.crossers",
    "area:corner",
    "area:tile",
    "area:paint",
    "area:raise",
    "area:set_height",
    "area:place_group",
    "area:set_tile",
    "area:cross",
    "area:erase",
];

/// The most steps one call raises or lowers a corner.
const MAX_STEPS: i32 = 64;

/// An area open for painting.
struct Area {
    are: ResKey,
    git: ResKey,
    set: Tileset,
    index: TileIndex,
    rules: Rules,
    /// The lights its new tiles get, from its lighting scheme.
    scheme: Option<Scheme>,
}

fn resref(name: &str) -> mlua::Result<ResRef> {
    ResRef::from_str(name).or_else(|e| fail(format!("{name:?} is not a resource's name: {e}")))
}

/// A tileset, from a hak the job is writing or the game's data.
fn tileset(sh: &Shared, name: ResRef) -> mlua::Result<Tileset> {
    let bytes = sh.bytes(&ResKey::new(name, ResType::SET), true)?;
    let codepage = sh.game.as_ref().map_or(CODEPAGE, |g| g.language.codepage());
    Tileset::parse(&bytes, codepage).or_else(|e| fail(format!("{name}.set: {e}")))
}

impl Area {
    fn open(sh: &Shared, name: &str) -> mlua::Result<Area> {
        let name = resref(name)?;
        let are = ResKey::new(name, ResType::ARE);
        let root = sh.ws.borrow_mut().doc(&are).or_else(fail)?.root.clone();
        let Some(tiles) = root.get("Tileset").and_then(|v| v.as_resref()) else {
            return fail(format!("{are} names no tileset"));
        };
        let set = tileset(sh, tiles)?;
        let index = TileIndex::new(&set);
        let rules = Rules::new(&index, &set);
        let scheme = sh.game.as_ref().and_then(|game| {
            let row = root.integer("LightingScheme")?.max(0) as usize;
            Scheme::read(game, row).ok()
        });
        Ok(Area { are, git: ResKey::new(name, ResType::GIT), set, index, rules, scheme })
    }

    /// The area's tiles as they are now, the job's edits included.
    fn grid(&self, sh: &Shared) -> mlua::Result<Grid> {
        let mut ws = sh.ws.borrow_mut();
        let doc = ws.doc(&self.are).or_else(fail)?;
        match grid(&doc.root, &self.index) {
            Some(grid) => Ok(grid),
            None => fail(format!(
                "{}: its tiles are not all in the tileset, or its size is missing",
                self.are
            )),
        }
    }

    fn corner(&self, grid: &Grid, x: f64, y: f64) -> mlua::Result<(u32, u32)> {
        let (w, h) = (grid.lattice.width(), grid.lattice.height());
        let whole = x.fract() == 0.0 && y.fract() == 0.0 && x >= 0.0 && y >= 0.0;
        if !whole || x > f64::from(w) || y > f64::from(h) {
            return fail(format!(
                "{} has no corner ({x}, {y}): corners are 0 to {w} across and 0 to {h} up",
                self.are
            ));
        }
        Ok((x as u32, y as u32))
    }

    fn cell(&self, grid: &Grid, x: f64, y: f64) -> mlua::Result<(u32, u32)> {
        let (w, h) = (grid.lattice.width(), grid.lattice.height());
        let whole = x.fract() == 0.0 && y.fract() == 0.0 && x >= 0.0 && y >= 0.0;
        if !whole || x >= f64::from(w) || y >= f64::from(h) {
            return fail(format!(
                "{} has no tile ({x}, {y}): tiles are 0 to {} across and 0 to {} up",
                self.are,
                w.saturating_sub(1),
                h.saturating_sub(1)
            ));
        }
        Ok((x as u32, y as u32))
    }

    /// A stroke made: the tiles chosen and put into the area.
    fn stroke(&self, sh: &Shared, before: &Grid, stroke: Stroke) -> mlua::Result<()> {
        let mut after = before.clone();
        let changes = after.apply(&self.index, stroke, &mut sh.rng.borrow_mut());
        self.put(sh, before, &changes)
    }

    /// Tiles put into the area: the ARE's, with lights as the area editor
    /// gives them, and the doors the tiles bring and take.
    fn put(
        &self,
        sh: &Shared,
        before: &Grid,
        changes: &[((u32, u32), Placement)],
    ) -> mlua::Result<()> {
        let root = sh.ws.borrow_mut().doc(&self.are).or_else(fail)?.root.clone();
        let mut lights =
            || self.scheme.as_ref().map_or([0; 3], |s| s.tile_lights(&mut sh.rng.borrow_mut()));
        let mut edits = tile_edits(self.are, &root, &self.set, changes, &mut lights);
        // (The area's objects are only read where a tile has a door.)
        let step = self.set.general.transition;
        let doors = changes.iter().any(|&(cell, p)| {
            let old = before.tile(cell.0, cell.1);
            !typed_hooks(&self.set, cell, p, step).is_empty()
                || !typed_hooks(&self.set, cell, old, step).is_empty()
        });
        if let (true, Some(game)) = (doors, &sh.game) {
            let git = sh.ws.borrow_mut().doc(&self.git).map(|g| g.root.clone());
            if let Ok(git) = git {
                let read = |r: ResRef| -> Option<Struct> {
                    let key = ResKey::new(r, ResType::UTD);
                    let data = sh.bytes(&key, false).or_else(|_| sh.bytes(&key, true)).ok()?;
                    Gff::read(&data).ok().map(|g| g.root)
                };
                let old = |(x, y): (u32, u32)| before.tile(x, y);
                edits.extend(door_edits(
                    game, self.git, &git, &self.set, step, &old, changes, &read,
                ));
            }
        }
        sh.apply_all(edits)
    }

    /// Raises (or lowers) a corner a step: whether it could be.
    fn step(&self, sh: &Shared, x: u32, y: u32, up: bool) -> mlua::Result<bool> {
        let before = self.grid(sh)?;
        match before.raise(&self.index, &self.rules, x, y, up) {
            Some(stroke) => self.stroke(sh, &before, stroke).map(|()| true),
            None => Ok(false),
        }
    }
}

/// What `ctx.terrain:open` hands out.
fn handle(lua: &Lua, sh: &Rc<Shared>, area: Area) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    let size = area.grid(sh)?;
    t.set("resref", area.are.resref.to_string())?;
    t.set("tileset", area.set_name(sh)?)?;
    t.set("width", size.lattice.width())?;
    t.set("height", size.lattice.height())?;
    t.set("step", area.set.general.transition)?;
    let terrains: Vec<String> =
        area.index.terrains().map(|t| area.index.terrain_name(t).to_string()).collect();
    t.set("terrains", terrains)?;
    let crossers: Vec<String> =
        area.index.crossers().map(|c| area.index.crosser_name(c).to_string()).collect();
    t.set("crossers", crossers)?;
    let groups = lua.create_table()?;
    for g in &area.set.groups {
        let group = lua.create_table()?;
        group.set("name", g.name.clone())?;
        group.set("rows", g.rows)?;
        group.set("columns", g.columns)?;
        groups.push(group)?;
    }
    t.set("groups", groups)?;

    let area = Rc::new(area);
    let (a, s) = (area.clone(), sh.clone());
    t.set(
        "corner",
        lua.create_function(move |_, (_, x, y): (This, f64, f64)| {
            let grid = a.grid(&s)?;
            let (x, y) = a.corner(&grid, x, y)?;
            let c = grid.lattice.corner(x, y);
            Ok((a.index.terrain_name(c.terrain).to_string(), c.height))
        })?,
    )?;
    let (a, s) = (area.clone(), sh.clone());
    t.set(
        "tile",
        lua.create_function(move |lua, (_, x, y): (This, f64, f64)| {
            let grid = a.grid(&s)?;
            let (x, y) = a.cell(&grid, x, y)?;
            let p = grid.tile(x, y);
            let tile = lua.create_table()?;
            tile.set("id", p.tile)?;
            tile.set("orientation", p.orientation)?;
            tile.set("height", p.height)?;
            Ok(tile)
        })?,
    )?;
    let (a, s) = (area.clone(), sh.clone());
    t.set(
        "paint",
        lua.create_function(move |_, (_, x, y, terrain): (This, f64, f64, String)| {
            let before = a.grid(&s)?;
            let (x, y) = a.corner(&before, x, y)?;
            let Some(terrain) = a.index.terrain(&terrain) else {
                let names: Vec<&str> =
                    a.index.terrains().map(|t| a.index.terrain_name(t)).collect();
                return fail(format!(
                    "the tileset has no terrain {terrain:?} (it has {})",
                    names.join(", ")
                ));
            };
            match before.paint(&a.index, &a.rules, x, y, terrain) {
                Some(stroke) => a.stroke(&s, &before, stroke).map(|()| true),
                None => Ok(false),
            }
        })?,
    )?;
    let (a, s) = (area.clone(), sh.clone());
    t.set(
        "raise",
        lua.create_function(move |_, (_, x, y, steps): (This, f64, f64, Option<i32>)| {
            let (x, y) = a.corner(&a.grid(&s)?, x, y)?;
            let steps = steps.unwrap_or(1).clamp(-MAX_STEPS, MAX_STEPS);
            for _ in 0..steps.abs() {
                if !a.step(&s, x, y, steps > 0)? {
                    return Ok(false);
                }
            }
            Ok(true)
        })?,
    )?;
    let (a, s) = (area.clone(), sh.clone());
    t.set(
        "set_height",
        lua.create_function(move |_, (_, x, y, height): (This, f64, f64, i32)| {
            let (x, y) = a.corner(&a.grid(&s)?, x, y)?;
            if height < 0 {
                return fail("a corner's height is 0 or more");
            }
            for _ in 0..MAX_STEPS {
                let now = a.grid(&s)?.lattice.corner(x, y).height;
                if now == height {
                    return Ok(true);
                }
                if !a.step(&s, x, y, now < height)? {
                    return Ok(false);
                }
            }
            Ok(false)
        })?,
    )?;
    let (a, s) = (area.clone(), sh.clone());
    t.set(
        "place_group",
        lua.create_function(
            move |_, (_, group, x, y, turns): (This, String, f64, f64, Option<u8>)| {
                let before = a.grid(&s)?;
                let (x, y) = a.cell(&before, x, y)?;
                let Some(found) = a.set.groups.iter().find(|g| g.name.eq_ignore_ascii_case(&group))
                else {
                    return fail(format!("the tileset has no group {group:?}"));
                };
                match before.place_group(&a.index, found, x, y, turns.unwrap_or(0) % 4) {
                    Some(stroke) => a.stroke(&s, &before, stroke).map(|()| true),
                    None => Ok(false),
                }
            },
        )?,
    )?;
    let (a, s) = (area.clone(), sh.clone());
    t.set(
        "cross",
        lua.create_function(move |_, (_, crosser, path): (This, String, Table)| {
            let before = a.grid(&s)?;
            let Some(found) = a.index.crosser(&crosser) else {
                let names: Vec<&str> =
                    a.index.crossers().map(|c| a.index.crosser_name(c)).collect();
                return fail(format!(
                    "the tileset has no crosser {crosser:?} (it has {})",
                    names.join(", ")
                ));
            };
            // The cells it runs through, each beside the one before.
            let mut cells = Vec::new();
            for step in path.sequence_values::<Table>() {
                let step = step?;
                let (x, y): (f64, f64) = (step.get(1)?, step.get(2)?);
                cells.push(a.cell(&before, x, y)?);
            }
            if cells.len() < 2 {
                return fail("a crosser runs through two cells or more: { {0, 0}, {1, 0} }");
            }
            let Some(edges) = mg_tiles::paint::path_edges(&cells) else {
                return fail("each cell of a crosser's path is beside the one before it");
            };
            match before.draw_crosser(&a.index, &edges, &[], found) {
                Some(stroke) => a.stroke(&s, &before, stroke).map(|()| true),
                None => Ok(false),
            }
        })?,
    )?;
    let (a, s) = (area.clone(), sh.clone());
    t.set(
        "erase",
        lua.create_function(move |_, (_, x, y): (This, f64, f64)| {
            let before = a.grid(&s)?;
            let (x, y) = a.cell(&before, x, y)?;
            match before.erase(&a.index, x, y) {
                Some(stroke) => a.stroke(&s, &before, stroke).map(|()| true),
                None => Ok(false),
            }
        })?,
    )?;
    let (a, s) = (area, sh.clone());
    t.set(
        "set_tile",
        lua.create_function(
            move |_,
                  (_, x, y, id, orientation, height): (
                This,
                f64,
                f64,
                u32,
                Option<u8>,
                Option<i32>,
            )| {
                let before = a.grid(&s)?;
                let cell = a.cell(&before, x, y)?;
                if id as usize >= a.set.tiles.len() {
                    return fail(format!(
                        "the tileset has no tile {id} (it has {})",
                        a.set.tiles.len()
                    ));
                }
                let p = Placement {
                    tile: id,
                    orientation: orientation.unwrap_or(0) % 4,
                    height: height.unwrap_or(0),
                };
                a.put(&s, &before, &[(cell, p)])
            },
        )?,
    )?;
    Ok(t)
}

impl Area {
    /// The tileset's name, as the ARE has it.
    fn set_name(&self, sh: &Shared) -> mlua::Result<String> {
        let mut ws = sh.ws.borrow_mut();
        let doc = ws.doc(&self.are).or_else(fail)?;
        Ok(doc.root.get("Tileset").and_then(|v| v.as_resref()).unwrap_or_default().to_string())
    }
}

/// `ctx.terrain`.
pub(crate) fn terrain(lua: &Lua, sh: &Rc<Shared>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    let s = sh.clone();
    t.set(
        "tilesets",
        lua.create_function(move |lua, _: This| {
            let Some(game) = &s.game else { return fail("there is no game data") };
            let out = lua.create_table()?;
            for choice in mg_module::new::tilesets(game) {
                let entry = lua.create_table()?;
                entry.set("resref", choice.resref.to_string())?;
                entry.set("name", choice.name)?;
                out.push(entry)?;
            }
            Ok(out)
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "new_area",
        lua.create_function(move |_, (_, spec): (This, Table)| {
            let Some(game) = &s.game else { return fail("there is no game data") };
            let name: String = spec.get("name")?;
            let spec = AreaSpec {
                name,
                tileset: resref(&spec.get::<String>("tileset")?)?,
                width: spec.get("width")?,
                height: spec.get("height")?,
            };
            let set = tileset(&s, spec.tileset)?;
            let (area, edits) = {
                let mut ws = s.ws.borrow_mut();
                ws.flush().or_else(fail)?;
                let mut staged = ws.module.clone();
                let area = mg_module::new::add_area_of(
                    &mut staged,
                    game,
                    &spec,
                    &set,
                    &mut s.rng.borrow_mut(),
                )
                .or_else(fail)?;
                (area, ws.edits_to(&staged))
            };
            s.apply_all(edits)?;
            Ok(area.to_string())
        })?,
    )?;
    let s = sh.clone();
    t.set(
        "open",
        lua.create_function(move |lua, (_, area): (This, String)| {
            handle(lua, &s, Area::open(&s, &area)?)
        })?,
    )?;
    Ok(t)
}
