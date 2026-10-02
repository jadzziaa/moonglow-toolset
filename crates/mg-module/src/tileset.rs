//! Making a tileset's files beside its `.set`: the palette Aurora and
//! Moonglow paint from (`<tileset>palstd.itp`), generated from the set's
//! groups and terrains.
//!
//! The palette (GFF `ITP `) has three categories under `MAIN`: Features
//! (ID 0, StrRef 63261), the one-tile groups; Groups (ID 1, StrRef 63262),
//! the others; and Terrain (ID 2, StrRef 8282), the terrains and crossers,
//! the eraser and, with height transitions, raise and lower. A group's leaf
//! names the model of its first tile (the painter finds the group by it), a
//! terrain's its name in lower case; each is labelled by its talk-table
//! string, else its name. The game's palettes are arranged by hand (some
//! in subfolders); `tests/tileset_authoring.rs` compares what they hold.

use mg_core::StrRef;
use mg_gff::{Gff, Struct, Value};
use mg_set::Tileset;

/// The talk-table strings of the categories and special terrain leaves.
pub const FEATURES: u32 = 63261;
pub const GROUPS: u32 = 63262;
pub const TERRAIN: u32 = 8282;
pub const ERASER: u32 = 63291;
pub const RAISE_LOWER: u32 = 63292;

fn leaf(resref: &str, strref: StrRef, name: &str) -> Struct {
    let mut s = Struct::new(0);
    if strref.is_none() {
        s.set("NAME", Value::String(name.as_bytes().to_vec()));
    } else {
        s.set("STRREF", Value::Dword(strref.0));
    }
    s.set("RESREF", Value::ResRef(resref.to_ascii_lowercase().into_bytes()));
    s
}

fn category(id: u8, strref: u32, leaves: Vec<Struct>) -> Struct {
    let mut s = Struct::new(0);
    s.set("STRREF", Value::Dword(strref));
    s.set("ID", Value::Byte(id));
    s.set("LIST", Value::List(leaves));
    s
}

/// The palette for a tileset, and what couldn't go in it (a group whose
/// first tile is empty or isn't a tile).
pub fn palette(set: &Tileset) -> (Gff, Vec<String>) {
    let mut warnings = Vec::new();
    let (mut features, mut groups) = (Vec::new(), Vec::new());
    for (i, g) in set.groups.iter().enumerate() {
        let first = g.tiles.first().copied().flatten();
        let Some(model) = first.and_then(|t| set.tiles.get(t as usize)).map(|t| t.model.clone())
        else {
            warnings.push(format!(
                "group {i} ({}): its first tile is empty or no tile, so the palette can't name it",
                g.name
            ));
            continue;
        };
        let l = leaf(&model, g.strref, &g.name);
        if g.rows * g.columns == 1 { features.push(l) } else { groups.push(l) }
    }
    let mut terrain: Vec<Struct> = set
        .terrains
        .iter()
        .chain(&set.crossers)
        .map(|t| leaf(&t.name, t.strref, t.unlocalized_name.as_deref().unwrap_or(&t.name)))
        .collect();
    terrain.push(leaf("eraser", StrRef(ERASER), "Eraser"));
    if set.general.has_height_transition {
        terrain.push(leaf("raiselower", StrRef(RAISE_LOWER), "Raise/Lower"));
    }
    let mut gff = Gff::new(*b"ITP ");
    gff.root.set(
        "MAIN",
        Value::List(vec![
            category(0, FEATURES, features),
            category(1, GROUPS, groups),
            category(2, TERRAIN, terrain),
        ]),
    );
    (gff, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_features_and_terrain() {
        let set = "[GENERAL]\nName=ZZZ01\nHasHeightTransition=1\n[TERRAIN TYPES]\nCount=1\n[TERRAIN0]\nName=Grass\nStrRef=63000\n\
                   [CROSSER TYPES]\nCount=1\n[CROSSER0]\nName=Road\n[TILES]\nCount=2\n[TILE0]\nModel=ZZZ01_A01_01\n[TILE1]\nModel=zzz01_b01_01\n\
                   [GROUPS]\nCount=3\n[GROUP0]\nName=Tree\nRows=1\nColumns=1\nTile0=0\n[GROUP1]\nName=House\nStrRef=64000\nRows=1\nColumns=2\nTile0=1\nTile1=0\n\
                   [GROUP2]\nName=Broken\nRows=1\nColumns=1\nTile0=-1\n";
        let set = Tileset::parse(set.as_bytes(), mg_core::Codepage::default()).unwrap();
        let (gff, warnings) = palette(&set);
        assert_eq!(warnings.len(), 1);
        let Some(Value::List(main)) = gff.root.get("MAIN") else { panic!() };
        let leaves = |i: usize| -> Vec<String> {
            let Some(Value::List(l)) = main[i].get("LIST") else { panic!() };
            l.iter()
                .map(|s| {
                    String::from_utf8_lossy(s.resref("RESREF").unwrap().as_bytes()).into_owned()
                })
                .collect()
        };
        assert_eq!(leaves(0), ["zzz01_a01_01"]);
        assert_eq!(leaves(1), ["zzz01_b01_01"]);
        assert_eq!(leaves(2), ["grass", "road", "eraser", "raiselower"]);
        let Some(Value::List(f)) = main[0].get("LIST") else { panic!() };
        assert_eq!(f[0].get("NAME"), Some(&Value::String(b"Tree".to_vec())));
        let Some(Value::List(g)) = main[1].get("LIST") else { panic!() };
        assert_eq!(g[0].get("STRREF"), Some(&Value::Dword(64000)));
        assert!(Gff::read(&gff.to_bytes().unwrap()).is_ok());
    }
}
