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
//! string, else its name. Features and groups are in a folder a terrain,
//! as hand-made palettes keep them (the game's `tcm02`, `trm02`, `trs02`:
//! Grass, Water, Castle...): the terrain most of a group's tiles' corners
//! have, labelled by its string or name; one whose corners name no terrain
//! of the set goes in Special (StrRef 6687). A category whose leaves are of
//! one terrain stays flat. (The game's are arranged by hand, and some in
//! folders of their own, City, Cliff; `tests/tileset_authoring.rs` compares
//! what they hold.)

use mg_core::StrRef;
use mg_gff::{Gff, Struct, Value};
use mg_set::Tileset;

/// The talk-table strings of the categories and special terrain leaves.
pub const FEATURES: u32 = 63261;
pub const GROUPS: u32 = 63262;
pub const TERRAIN: u32 = 8282;
pub const ERASER: u32 = 63291;
pub const RAISE_LOWER: u32 = 63292;
/// The folder of features and groups whose tiles' corners name no terrain
/// of the set ("Special", as hand-made palettes have one).
pub const SPECIAL: u32 = 6687;

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

/// A folder of leaves, labelled by a talk-table string or a name.
fn folder(strref: StrRef, name: &str, leaves: Vec<Struct>) -> Struct {
    let mut s = Struct::new(0);
    if strref.is_none() {
        s.set("NAME", Value::String(name.as_bytes().to_vec()));
    } else {
        s.set("STRREF", Value::Dword(strref.0));
    }
    s.set("LIST", Value::List(leaves));
    s
}

/// The terrain (its index in the set's) most of a group's tiles' corners
/// have; the first of the set's terrains among those as many; `None` for
/// one whose corners name none of them.
fn group_terrain(set: &Tileset, tiles: &[Option<u32>]) -> Option<usize> {
    let mut counts = vec![0usize; set.terrains.len()];
    for t in tiles.iter().flatten().filter_map(|t| set.tiles.get(*t as usize)) {
        for (corner, _) in &t.corners {
            if let Some(i) = set.terrains.iter().position(|k| k.name.eq_ignore_ascii_case(corner)) {
                counts[i] += 1;
            }
        }
    }
    let most = *counts.iter().max()?;
    (most > 0).then(|| counts.iter().position(|&c| c == most)).flatten()
}

/// Leaves by terrain ([`group_terrain`]) in folders, in the set's terrains'
/// order, Special last; the leaves as they are where they are of one.
fn by_terrain(set: &Tileset, leaves: Vec<(Option<usize>, Struct)>) -> Vec<Struct> {
    let mut kinds: Vec<Option<usize>> = leaves.iter().map(|(k, _)| *k).collect();
    kinds.sort_by_key(|k| k.unwrap_or(usize::MAX));
    kinds.dedup();
    if kinds.len() < 2 {
        return leaves.into_iter().map(|(_, l)| l).collect();
    }
    let mut out = Vec::new();
    for kind in kinds {
        let these: Vec<Struct> =
            leaves.iter().filter(|(k, _)| *k == kind).map(|(_, l)| l.clone()).collect();
        out.push(match kind.and_then(|i| set.terrains.get(i)) {
            Some(t) => folder(t.strref, t.unlocalized_name.as_deref().unwrap_or(&t.name), these),
            None => folder(StrRef(SPECIAL), "Special", these),
        });
    }
    out
}

/// How many leaves (brushes) a palette's category holds, in its folders
/// too.
pub fn leaves(category: &Struct) -> usize {
    match category.get("LIST") {
        Some(Value::List(items)) => items
            .iter()
            .map(|s| {
                if s.get("LIST").is_some() {
                    leaves(s)
                } else {
                    usize::from(s.get("RESREF").is_some())
                }
            })
            .sum(),
        _ => 0,
    }
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
        let l = (group_terrain(set, &g.tiles), leaf(&model, g.strref, &g.name));
        if g.rows * g.columns == 1 { features.push(l) } else { groups.push(l) }
    }
    let (features, groups) = (by_terrain(set, features), by_terrain(set, groups));
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

/// A palette following a terrain or crosser renamed in its `.set`
/// (`mg_set::edit::rename_type`): in its Terrain category (ID 2, its
/// subfolders too), each leaf naming `from` (its `RESREF`, without case)
/// names `to`, and its label too where it is the name itself (`NAME`, no
/// talk-table string). The rest of the palette, hand-made as it may be,
/// stays as it is. Returns how many leaves changed.
pub fn rename_in_palette(palette: &mut Gff, from: &str, to: &str) -> usize {
    fn walk(items: &mut [Struct], from: &str, to: &str) -> usize {
        let mut changed = 0;
        for s in items {
            if let Some(list) = s.list_mut("LIST") {
                changed += walk(list, from, to);
                continue;
            }
            let named =
                s.resref("RESREF").is_some_and(|r| r.to_string().eq_ignore_ascii_case(from));
            if !named {
                continue;
            }
            s.set("RESREF", Value::ResRef(to.to_ascii_lowercase().into_bytes()));
            if s.string("NAME").is_some_and(|n| n.eq_ignore_ascii_case(from.as_bytes())) {
                s.set("NAME", Value::String(to.as_bytes().to_vec()));
            }
            changed += 1;
        }
        changed
    }
    let Some(main) = palette.root.list_mut("MAIN") else { return 0 };
    main.iter_mut()
        .filter(|c| c.integer("ID") == Some(2))
        .filter_map(|c| c.list_mut("LIST"))
        .map(|l| walk(l, from, to))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_palette_follows_a_terrain_renamed() {
        let set = "[GENERAL]\nName=ZZZ01\n[TERRAIN TYPES]\nCount=2\n[TERRAIN0]\nName=Grass\n\
                   [TERRAIN1]\nName=Water\nStrRef=63000\n[TILES]\nCount=1\n[TILE0]\nModel=grass\n\
                   [GROUPS]\nCount=1\n[GROUP0]\nName=Grass\nRows=1\nColumns=1\nTile0=0\n";
        let set = Tileset::parse(set.as_bytes(), mg_core::Codepage::default()).unwrap();
        let (mut gff, _) = palette(&set);
        assert_eq!(rename_in_palette(&mut gff, "GRASS", "Meadow"), 1);
        let Some(Value::List(main)) = gff.root.get("MAIN") else { panic!() };
        let leaves = |i: usize| -> Vec<(String, Option<Vec<u8>>)> {
            let Some(Value::List(l)) = main[i].get("LIST") else { panic!() };
            l.iter()
                .map(|s| {
                    (s.resref("RESREF").unwrap().to_string(), s.string("NAME").map(<[u8]>::to_vec))
                })
                .collect()
        };
        assert_eq!(leaves(2)[0], ("meadow".to_string(), Some(b"Meadow".to_vec())), "renamed");
        assert_eq!(leaves(2)[1].0, "water", "the other terrain as it was");
        assert_eq!(leaves(0)[0].0, "grass", "a feature's model is not a terrain");
        assert_eq!(rename_in_palette(&mut gff, "Nothing", "x"), 0);
    }

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

    /// Features and groups of more than one terrain are in a folder a
    /// terrain (the one most of their tiles' corners have), as hand-made
    /// palettes keep them; one of no terrain of the set in Special.
    #[test]
    fn features_and_groups_in_a_folder_a_terrain() {
        let corners = |tl: &str, tr: &str, bl: &str, br: &str| {
            format!("TopLeft={tl}\nTopRight={tr}\nBottomLeft={bl}\nBottomRight={br}\n")
        };
        let set = format!(
            "[GENERAL]\nName=ZZZ01\n[TERRAIN TYPES]\nCount=2\n[TERRAIN0]\nName=Grass\nStrRef=63635\n\
             [TERRAIN1]\nName=Water\n[TILES]\nCount=4\n[TILE0]\nModel=zzz01_a01_01\n{}\
             [TILE1]\nModel=zzz01_b01_01\n{}[TILE2]\nModel=zzz01_c01_01\n{}\
             [TILE3]\nModel=zzz01_d01_01\n{}\
             [GROUPS]\nCount=4\n[GROUP0]\nName=Tree\nRows=1\nColumns=1\nTile0=0\n\
             [GROUP1]\nName=Pond\nRows=1\nColumns=1\nTile0=1\n\
             [GROUP2]\nName=Odd\nRows=1\nColumns=1\nTile0=3\n\
             [GROUP3]\nName=Shore\nRows=1\nColumns=2\nTile0=1\nTile1=2\n",
            corners("Grass", "Grass", "Grass", "Water"),
            corners("Water", "Water", "Water", "Grass"),
            corners("Water", "Grass", "Water", "Grass"),
            corners("Lava", "", "", "")
        );
        let set = Tileset::parse(set.as_bytes(), mg_core::Codepage::default()).unwrap();
        let (gff, warnings) = palette(&set);
        assert!(warnings.is_empty(), "{warnings:?}");
        let Some(Value::List(main)) = gff.root.get("MAIN") else { panic!() };
        let list = |s: &Struct| -> Vec<Struct> {
            let Some(Value::List(l)) = s.get("LIST") else { panic!("no LIST") };
            l.clone()
        };
        let named = |s: &Struct| match (s.get("STRREF"), s.get("NAME")) {
            (Some(Value::Dword(r)), _) => r.to_string(),
            (_, Some(Value::String(n))) => String::from_utf8_lossy(n).into_owned(),
            _ => panic!("unnamed"),
        };
        let resref = |s: &Struct| {
            String::from_utf8_lossy(s.resref("RESREF").unwrap().as_bytes()).into_owned()
        };
        // Features: Grass (its StrRef), Water (its name), Special.
        let features = list(&main[0]);
        let folders: Vec<String> = features.iter().map(named).collect();
        assert_eq!(folders, ["63635", "Water", "6687"]);
        assert!(features.iter().all(|f| f.get("RESREF").is_none()), "folders, not leaves");
        assert_eq!(list(&features[0]).iter().map(resref).collect::<Vec<_>>(), ["zzz01_a01_01"]);
        assert_eq!(list(&features[1]).iter().map(resref).collect::<Vec<_>>(), ["zzz01_b01_01"]);
        assert_eq!(list(&features[2]).iter().map(resref).collect::<Vec<_>>(), ["zzz01_d01_01"]);
        // Groups: one, of one terrain (its tiles' corners mostly Water):
        // flat.
        let groups = list(&main[1]);
        assert_eq!(groups.iter().map(resref).collect::<Vec<_>>(), ["zzz01_b01_01"]);
        assert_eq!((leaves(&main[0]), leaves(&main[1]), leaves(&main[2])), (3, 1, 3));
        assert!(Gff::read(&gff.to_bytes().unwrap()).is_ok());
    }
}
