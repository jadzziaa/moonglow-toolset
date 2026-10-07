//! Terrain editing: the tileset's palette (`<tileset>palstd.itp`), the
//! brushes in it, and painted tiles as workspace edits of the ARE.
//!
//! The palette has a Features and a Groups branch (tile groups, named by
//! their first tile's model) and a Terrain branch: the tileset's terrains
//! and crossers by lower-case name, the Eraser (chooses one tile again)
//! and Raise/Lower (only with height transitions).
//! Aurora shows each branch's entries sorted by name.

use mg_core::{ResRef, ResType, StrRef};
use mg_gff::{Gff, Struct};
use mg_rules::GameData;
use mg_set::Tileset;
use mg_tiles::{Crosser, Terrain, TileIndex};

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
    /// Steps the tile under the pointer through the tiles that fit there
    /// (the Eraser with Shift), whatever it holds; paints nothing.
    /// Moonglow's own: no tileset's palette has it.
    Refine,
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

pub use mg_edit::terrain::{door_edits, grid, tile_edits, typed_hooks};
