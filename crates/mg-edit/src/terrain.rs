//! Painted terrain as edits: an area's tile grid read from its ARE, the
//! edits that put new tiles into it, and the doors tiles bring and take.

use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_set::Tileset;
use mg_tiles::paint::Grid;
use mg_tiles::{Placement, TileIndex};

use crate::{Edit, GffPath};

/// A tile's side, in metres.
const TILE_SIZE: f32 = 10.0;

/// An area's tile grid read from its ARE: `None` when the ARE has no
/// usable size or names a tile the tileset does not have. A tile's height
/// is held to [`mg_tiles::MAX_HEIGHT`] steps up or down, as the area view
/// holds it.
pub fn grid(are: &Struct, index: &TileIndex) -> Option<Grid> {
    let width = u32::try_from(are.integer("Width")?).ok()?;
    let height = u32::try_from(are.integer("Height")?).ok()?;
    let top = i64::from(mg_tiles::MAX_HEIGHT);
    let tiles: Vec<Placement> = are
        .list("Tile_List")?
        .iter()
        .map(|t| Placement {
            // (A number no tileset has a tile of stays one it has none of.)
            tile: u32::try_from(t.integer("Tile_ID").unwrap_or(0).max(0)).unwrap_or(u32::MAX),
            orientation: (t.integer("Tile_Orientation").unwrap_or(0) & 3) as u8,
            height: t.integer("Tile_Height").unwrap_or(0).clamp(-top, top) as i32,
        })
        .collect();
    Grid::new(index, width, height, tiles)
}

/// The edits that put `changes` (cells and their new tiles) into the ARE
/// `are` (its root `root`): each tile's ID, orientation and height, lights
/// from `lights` (main 1, main 2, source; Aurora picks them at random from
/// the lighting scheme for every tile it paints) and the animation loops
/// the tileset's tile has.
pub fn tile_edits(
    are: ResKey,
    root: &Struct,
    set: &Tileset,
    changes: &[((u32, u32), Placement)],
    lights: &mut dyn FnMut() -> [u8; 3],
) -> Vec<Edit> {
    let width = root.integer("Width").unwrap_or(0).max(0) as usize;
    let mut edits = Vec::new();
    for &((x, y), p) in changes {
        let i = y as usize * width + x as usize;
        let path = GffPath::root().item("Tile_List", i);
        let current = root.list("Tile_List").and_then(|l| l.get(i));
        let [main1, main2, source] = lights();
        for (label, value) in tile_fields(set, p, [main1, main2, source, source]) {
            if current.and_then(|s| s.get(label)) == Some(&value) {
                continue;
            }
            edits.push(Edit::SetField {
                key: are,
                path: path.clone(),
                label: label.into(),
                value: Some(value),
            });
        }
    }
    edits
}

/// A tile's fields as Moonglow writes one into an ARE's `Tile_List`, in
/// their order: tile `p` of `set`, its `lights` (main 1, main 2, source 1,
/// source 2) and the animation loops the tileset's tile has. The one
/// place a tile's struct is spelled: painting sets these fields, and
/// Resize and Change Tileset write the struct of them ([`tile_struct`]).
pub fn tile_fields(set: &Tileset, p: Placement, lights: [u8; 4]) -> [(&'static str, Value); 10] {
    let loops = set.tiles.get(p.tile as usize).map_or([false; 3], |t| t.anim_loops);
    [
        ("Tile_ID", Value::Int(p.tile as i32)),
        ("Tile_Orientation", Value::Int(i32::from(p.orientation))),
        ("Tile_Height", Value::Int(p.height)),
        ("Tile_MainLight1", Value::Byte(lights[0])),
        ("Tile_MainLight2", Value::Byte(lights[1])),
        ("Tile_SrcLight1", Value::Byte(lights[2])),
        ("Tile_SrcLight2", Value::Byte(lights[3])),
        ("Tile_AnimLoop1", Value::Byte(u8::from(loops[0]))),
        ("Tile_AnimLoop2", Value::Byte(u8::from(loops[1]))),
        ("Tile_AnimLoop3", Value::Byte(u8::from(loops[2]))),
    ]
}

/// A new tile's struct, as the area wizard writes one ([`tile_fields`]).
pub fn tile_struct(set: &Tileset, p: Placement, lights: [u8; 4]) -> Struct {
    let mut s = Struct::new(1);
    for (label, value) in tile_fields(set, p, lights) {
        s.set(label, value);
    }
    s
}

/// The door hooks of a tileset's tile `tile` lying with its middle at
/// `centre`, turned `orientation` quarters: each hook's doortypes.2da row
/// (0: one that brings no door), where a door on it stands and its
/// `Bearing`. The one place a tile's hooks are put into an area: the
/// area's view, painting, Resize and Change Tileset read them here.
pub fn hooks(
    tile: &mg_set::Tile,
    centre: glam::Vec3,
    orientation: u8,
) -> impl Iterator<Item = (i32, glam::Vec3, f32)> + '_ {
    let turn = f32::from(orientation) * 90.0;
    let rotation = glam::Quat::from_rotation_z(turn.to_radians());
    tile.doors.iter().map(move |d| {
        let degrees = (d.orientation + turn).rem_euclid(360.0);
        let degrees = if degrees > 180.0 { degrees - 360.0 } else { degrees };
        (d.door_type, centre + rotation * glam::Vec3::from_array(d.position), degrees.to_radians())
    })
}

/// A tile's door hooks that bring a door (type not 0): doortypes.2da row,
/// where the door stands and its `Bearing`, for tile `p` in cell (x, y)
/// with height steps of `step` metres.
pub fn typed_hooks(
    set: &Tileset,
    (x, y): (u32, u32),
    p: Placement,
    step: f32,
) -> Vec<(i32, glam::Vec3, f32)> {
    let Some(tile) = set.tiles.get(p.tile as usize) else { return Vec::new() };
    let centre = glam::Vec3::new(
        (x as f32 + 0.5) * TILE_SIZE,
        (y as f32 + 0.5) * TILE_SIZE,
        p.height as f32 * step,
    );
    hooks(tile, centre, p.orientation).filter(|(door_type, ..)| *door_type != 0).collect()
}

/// The doors tiles bring and take, as Aurora handles them: a tile it puts
/// in gets a door of each typed hook's type (doortypes.2da
/// `TemplateResRef`, placed as a door from the palette is, its
/// `Appearance` the type), and a tile it
/// replaces takes the doors standing on its typed hooks away. `before`
/// gives each changed cell's tile before; `blueprint` reads a door
/// blueprint.
#[allow(clippy::too_many_arguments)]
pub fn door_edits(
    game: &GameData,
    git_key: ResKey,
    git: &Struct,
    set: &Tileset,
    step: f32,
    before: &dyn Fn((u32, u32)) -> Placement,
    changes: &[((u32, u32), Placement)],
    blueprint: &dyn Fn(ResRef) -> Option<Struct>,
) -> Vec<Edit> {
    use mg_module::instances::{Placement as At, Placing, instance};
    let changes: Vec<_> = changes.iter().filter(|(c, p)| before(*c) != *p).collect();
    let doors = git.list("Door List").unwrap_or(&[]);
    let mut gone: Vec<usize> = Vec::new();
    for &&(cell, _) in &changes {
        for (_, at, _) in typed_hooks(set, cell, before(cell), step) {
            for (i, d) in doors.iter().enumerate() {
                let (dx, dy) = (d.float("X").unwrap_or(0.0), d.float("Y").unwrap_or(0.0));
                if (dx - at.x).abs() < 0.05 && (dy - at.y).abs() < 0.05 && !gone.contains(&i) {
                    gone.push(i);
                }
            }
        }
    }
    gone.sort_unstable();
    let mut edits: Vec<Edit> = gone
        .iter()
        .rev()
        .map(|&index| Edit::RemoveItem {
            key: git_key,
            path: GffPath::root(),
            list: "Door List".into(),
            index,
        })
        .collect();
    let table = game.table("doortypes").ok();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let mut next = doors.len() - gone.len();
    for &&(cell, p) in &changes {
        for (door_type, at, bearing) in typed_hooks(set, cell, p, step) {
            let resref = table
                .as_ref()
                .and_then(|t| t.get(door_type.max(0) as usize, "TemplateResRef"))
                .and_then(|r| ResRef::from_str(r.trim()).ok());
            let Some(bp) = resref.and_then(blueprint) else { continue };
            let placed = At { position: at.to_array(), rotation: bearing };
            let Some(mut item) = instance(&placing, ResType::UTD, &bp, placed, &[]) else {
                continue;
            };
            // The door looks as its hook's type says (captured).
            item.set("Appearance", Value::Dword(door_type.max(0) as u32));
            edits.push(Edit::insert_item(git_key, GffPath::root(), "Door List", next, item));
            next += 1;
        }
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: &str = "[GENERAL]\nName=TST01\nInterior=0\nTransition=5\nDisplayName=-1\n\
Border=A\nDefault=A\nFloor=A\n[TERRAIN TYPES]\nCount=1\n[TERRAIN0]\nName=A\n\
[CROSSER TYPES]\nCount=0\n[PRIMARY RULES]\nCount=0\n[SECONDARY RULES]\nCount=0\n\
[TILES]\nCount=1\n[TILE0]\nModel=t0\nTopLeft=A\nTopRight=A\nBottomLeft=A\nBottomRight=A\n\
[GROUPS]\nCount=0\n";

    fn are(width: i32, height: i32, tiles: &[(i64, i64)]) -> Struct {
        let mut are = Struct::new(0);
        are.set("Width", Value::Int(width));
        are.set("Height", Value::Int(height));
        let tiles = tiles.iter().map(|&(id, up)| {
            let mut t = Struct::new(1);
            t.set("Tile_ID", Value::Int64(id));
            t.set("Tile_Height", Value::Int64(up));
            t
        });
        are.set("Tile_List", Value::List(tiles.collect()));
        are
    }

    /// A tile's struct and a tile's edits are the same ten fields, and a
    /// tile's hooks stand where its middle and its turn put them, those
    /// that bring a door (a type that is not 0) being the typed ones.
    #[test]
    fn a_tile_is_written_and_its_hooks_placed_in_one_way() {
        let text = SET.replace(
            "[TILE0]\nModel=t0\n",
            "[TILE0]\nModel=t0\nAnimLoop2=1\nDoors=2\n[TILE0DOOR0]\nType=3\nX=5\nY=0\nZ=0\n\
             Orientation=90\n[TILE0DOOR1]\nType=0\nX=0\nY=5\nZ=1\nOrientation=0\n[TILE0X]\n",
        );
        let set = Tileset::parse(text.as_bytes(), mg_core::Codepage::WINDOWS_1252).unwrap();
        assert_eq!(set.tiles[0].doors.len(), 2, "{:?}", set.tiles[0]);
        let p = Placement { tile: 0, orientation: 1, height: 2 };
        let s = tile_struct(&set, p, [1, 2, 3, 4]);
        let fields = tile_fields(&set, p, [1, 2, 3, 4]);
        let written: Vec<(String, Value)> =
            s.fields.iter().map(|f| (f.label.to_string(), f.value.clone())).collect();
        let listed: Vec<(String, Value)> =
            fields.iter().map(|(l, v)| (l.to_string(), v.clone())).collect();
        assert_eq!(written, listed);
        assert_eq!(s.id, 1);
        assert_eq!((s.integer("Tile_SrcLight2"), s.integer("Tile_AnimLoop2")), (Some(4), Some(1)));
        // Painting the same tile into an area that has it there: no edit.
        let mut are = are(1, 1, &[]);
        are.set("Tile_List", Value::List(vec![s]));
        let key = ResKey::parse("a", ResType::ARE).unwrap();
        assert!(tile_edits(key, &are, &set, &[((0, 0), p)], &mut || [1, 2, 3]).len() == 1);
        // Cell (2, 1), a quarter turned, two steps of 5 m up.
        let all: Vec<_> = hooks(&set.tiles[0], glam::Vec3::new(25.0, 15.0, 10.0), 1).collect();
        assert_eq!(all.len(), 2);
        let typed = typed_hooks(&set, (2, 1), p, 5.0);
        assert_eq!(typed, [all[0]]);
        let (door_type, at, bearing) = typed[0];
        assert_eq!(door_type, 3);
        assert!((at - glam::Vec3::new(25.0, 20.0, 10.0)).length() < 1e-4, "{at}");
        assert!((bearing - std::f32::consts::PI).abs() < 1e-5, "{bearing}");
    }

    /// An ARE's numbers are whatever the file says: a tile two billion
    /// steps up is read as far up as the area view shows one (Change
    /// Tileset raised its corners a level at a time), a tile number past
    /// a `u32` is no tile of the tileset's and not one of them, and a size
    /// its tile list does not fill is no grid.
    #[test]
    fn an_are_s_numbers_are_held_to_what_a_grid_can_be() {
        let set = Tileset::parse(SET.as_bytes(), mg_core::Codepage::WINDOWS_1252).unwrap();
        let index = TileIndex::new(&set);
        let top = mg_tiles::MAX_HEIGHT;
        let g = grid(&are(2, 1, &[(0, i64::from(i32::MAX)), (0, i64::MIN)]), &index).unwrap();
        assert_eq!(g.tiles.iter().map(|p| p.height).collect::<Vec<_>>(), [top, -top]);
        assert_eq!(g.lattice.corner(0, 0).height, top);
        assert!(grid(&are(1, 1, &[(1 << 32, 0)]), &index).is_none(), "tile 2^32 is not tile 0");
        assert!(grid(&are(i32::MAX, i32::MAX, &[(0, 0)]), &index).is_none());
        assert!(grid(&are(100_000, 100_000, &[(0, 0)]), &index).is_none());
    }
}
