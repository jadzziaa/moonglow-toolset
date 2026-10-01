//! Terrain editing: the tileset's palette (`<tileset>palstd.itp`), the
//! brushes in it, and painted tiles as workspace edits of the ARE.
//!
//! The palette has a Features and a Groups branch (tile groups, named by
//! their first tile's model) and a Terrain branch: the tileset's terrains
//! and crossers by lower-case name, the Eraser (chooses one tile again)
//! and Raise/Lower (only with height transitions).
//! Aurora shows each branch's entries sorted by name.

use mg_core::{ResRef, ResType, StrRef};
use mg_edit::{Edit, GffPath};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_set::Tileset;
use mg_tiles::paint::Grid;
use mg_tiles::{Crosser, Placement, Terrain, TileIndex};

/// What a palette entry paints with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Brush {
    Terrain(Terrain),
    Crosser(Crosser),
    /// Chooses the tile under the pointer again, its terrain as it is
    /// (captured: not the tileset's `Default` terrain, as the ITP
    /// documentation says).
    Eraser,
    /// Raises a corner a height step (the right button lowers it).
    RaiseLower,
    /// The tileset's group of this index.
    Group(usize),
}

/// An entry of the tileset palette: a brush, or a folder of entries.
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteItem {
    Brush { label: String, brush: Brush },
    Folder { label: String, items: Vec<PaletteItem> },
}

impl PaletteItem {
    pub fn label(&self) -> &str {
        match self {
            PaletteItem::Brush { label, .. } | PaletteItem::Folder { label, .. } => label,
        }
    }
}

/// A tileset's palette: its top-level branches (Features, Groups, Terrain),
/// each a folder.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TilesetPalette {
    pub branches: Vec<PaletteItem>,
}

impl TilesetPalette {
    /// The palette of `tileset` (`<tileset>palstd.itp`); empty when the game
    /// has none.
    pub fn read(game: &GameData, tileset: ResRef, set: &Tileset, index: &TileIndex) -> Self {
        let name = format!("{tileset}palstd");
        let Ok(data) = game.resman.get_named(&name, ResType::ITP) else {
            return TilesetPalette::default();
        };
        let Ok(gff) = Gff::read(&data) else {
            return TilesetPalette::default();
        };
        Self::from_itp(&gff.root, &|s| game.string(s).unwrap_or_default(), set, index)
    }

    /// The palette an ITP's root describes; `string` looks talk-table
    /// strings up.
    pub fn from_itp(
        root: &Struct,
        string: &dyn Fn(StrRef) -> String,
        set: &Tileset,
        index: &TileIndex,
    ) -> Self {
        let mut branches: Vec<PaletteItem> = root
            .list("MAIN")
            .unwrap_or(&[])
            .iter()
            .map(|branch| {
                let terrain = branch.integer("ID") == Some(2);
                let items = items(branch, terrain, string, set, index);
                PaletteItem::Folder { label: label(branch, string), items }
            })
            .collect();
        sort(&mut branches);
        TilesetPalette { branches }
    }

    /// Every brush, depth first, with its label.
    pub fn brushes(&self) -> Vec<(&str, Brush)> {
        fn walk<'a>(items: &'a [PaletteItem], out: &mut Vec<(&'a str, Brush)>) {
            for item in items {
                match item {
                    PaletteItem::Brush { label, brush } => out.push((label, *brush)),
                    PaletteItem::Folder { items, .. } => walk(items, out),
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.branches, &mut out);
        out
    }

    /// The brush labelled `label` (as the palette shows it).
    pub fn brush(&self, label: &str) -> Option<Brush> {
        self.brushes().into_iter().find(|(l, _)| l.eq_ignore_ascii_case(label)).map(|(_, b)| b)
    }
}

fn label(s: &Struct, string: &dyn Fn(StrRef) -> String) -> String {
    match s.integer("STRREF") {
        Some(r) if r >= 0 && r != i64::from(u32::MAX) => string(StrRef(r as u32)),
        _ => String::from_utf8_lossy(s.string("NAME").unwrap_or_default()).into_owned(),
    }
}

fn sort(items: &mut [PaletteItem]) {
    items.sort_by_key(|i| i.label().to_lowercase());
}

fn items(
    folder: &Struct,
    terrain: bool,
    string: &dyn Fn(StrRef) -> String,
    set: &Tileset,
    index: &TileIndex,
) -> Vec<PaletteItem> {
    let mut out = Vec::new();
    for s in folder.list("LIST").unwrap_or(&[]) {
        let name = label(s, string);
        if s.contains("LIST") {
            let items = items(s, terrain, string, set, index);
            out.push(PaletteItem::Folder { label: name, items });
            continue;
        }
        let Some(resref) = s.resref("RESREF").map(|r| r.to_string()) else { continue };
        let brush = if terrain {
            match resref.as_str() {
                "eraser" => Some(Brush::Eraser),
                "raiselower" => set.general.has_height_transition.then_some(Brush::RaiseLower),
                r => index
                    .terrain(r)
                    .map(Brush::Terrain)
                    .or_else(|| index.crosser(r).map(Brush::Crosser)),
            }
        } else {
            group_of(set, &resref).map(Brush::Group)
        };
        if let Some(brush) = brush {
            out.push(PaletteItem::Brush { label: name, brush });
        }
    }
    sort(&mut out);
    out
}

/// The group whose first tile has model `model`.
fn group_of(set: &Tileset, model: &str) -> Option<usize> {
    set.groups.iter().position(|g| {
        g.tiles
            .first()
            .copied()
            .flatten()
            .and_then(|t| set.tiles.get(t as usize))
            .is_some_and(|t| t.model.eq_ignore_ascii_case(model))
    })
}

/// An area's tile grid read from its ARE: `None` when the ARE has no
/// usable size or names a tile the tileset does not have.
pub fn grid(are: &Struct, index: &TileIndex) -> Option<Grid> {
    let width = u32::try_from(are.integer("Width")?).ok()?;
    let height = u32::try_from(are.integer("Height")?).ok()?;
    let tiles: Vec<Placement> = are
        .list("Tile_List")?
        .iter()
        .map(|t| Placement {
            tile: t.integer("Tile_ID").unwrap_or(0).max(0) as u32,
            orientation: (t.integer("Tile_Orientation").unwrap_or(0) & 3) as u8,
            height: t.integer("Tile_Height").unwrap_or(0) as i32,
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
    let width = root.integer("Width").unwrap_or(0).max(0) as u32;
    let mut edits = Vec::new();
    for &((x, y), p) in changes {
        let i = (y * width + x) as usize;
        let path = GffPath::root().item("Tile_List", i);
        let current = root.list("Tile_List").and_then(|l| l.get(i));
        let loops = set.tiles.get(p.tile as usize).map_or([false; 3], |t| t.anim_loops);
        let [main1, main2, source] = lights();
        let fields: [(&str, Value); 10] = [
            ("Tile_ID", Value::Int(p.tile as i32)),
            ("Tile_Orientation", Value::Int(i32::from(p.orientation))),
            ("Tile_Height", Value::Int(p.height)),
            ("Tile_MainLight1", Value::Byte(main1)),
            ("Tile_MainLight2", Value::Byte(main2)),
            ("Tile_SrcLight1", Value::Byte(source)),
            ("Tile_SrcLight2", Value::Byte(source)),
            ("Tile_AnimLoop1", Value::Byte(u8::from(loops[0]))),
            ("Tile_AnimLoop2", Value::Byte(u8::from(loops[1]))),
            ("Tile_AnimLoop3", Value::Byte(u8::from(loops[2]))),
        ];
        for (label, value) in fields {
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
        (x as f32 + 0.5) * crate::TILE_SIZE,
        (y as f32 + 0.5) * crate::TILE_SIZE,
        p.height as f32 * step,
    );
    let turn = f32::from(p.orientation) * 90.0;
    let rotation = glam::Quat::from_rotation_z(turn.to_radians());
    tile.doors
        .iter()
        .filter(|d| d.door_type != 0)
        .map(|d| {
            let degrees = (d.orientation + turn).rem_euclid(360.0);
            let degrees = if degrees > 180.0 { degrees - 360.0 } else { degrees };
            (
                d.door_type,
                centre + rotation * glam::Vec3::from_array(d.position),
                degrees.to_radians(),
            )
        })
        .collect()
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
            edits.push(Edit::InsertItem {
                key: git_key,
                path: GffPath::root(),
                list: "Door List".into(),
                index: next,
                item,
            });
            next += 1;
        }
    }
    edits
}
