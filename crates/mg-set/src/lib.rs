//! Tileset definitions (`.set`): terrains, crossers, painting rules, tiles
//! with their corner terrains, heights, lights and doors, and tile groups.
//!
//! [`Tileset::parse`] reads the typed model; the underlying [`Ini`] is kept
//! for anything the model does not cover. The toolset never writes tilesets.

pub mod ini;

use mg_core::{Codepage, StrRef};
use thiserror::Error;

pub use ini::{Ini, Section};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SetError {
    #[error("missing section [{0}]")]
    MissingSection(String),
    #[error("[{section}] is missing {key}")]
    MissingKey { section: String, key: String },
}

/// `[GENERAL]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct General {
    pub name: String,
    pub interior: bool,
    pub has_height_transition: bool,
    pub env_map: Option<String>,
    /// Height (in metres) of one height step.
    pub transition: f32,
    pub display_name: StrRef,
    pub unlocalized_name: Option<String>,
    /// Terrain along the area border.
    pub border: String,
    /// Terrain new areas start with.
    pub default: String,
    pub floor: String,
    pub selector_height: Option<f32>,
}

/// `[GRASS]`: grass drawn on tiles' grass faces.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Grass {
    pub enabled: bool,
    pub density: f32,
    pub height: f32,
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
}

/// A terrain or crosser type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedType {
    pub name: String,
    pub strref: StrRef,
    pub unlocalized_name: Option<String>,
}

/// A painting rule: placing `placed` next to `adjacent` turns the adjacent
/// corner into `changed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub placed: String,
    pub placed_height: i32,
    pub adjacent: String,
    pub adjacent_height: i32,
    pub changed: String,
    pub changed_height: i32,
}

/// A door hook on a tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileDoor {
    /// Row of `doortypes.2da`.
    pub door_type: i32,
    pub position: [f32; 3],
    /// Degrees.
    pub orientation: f32,
}

/// Corners in the order the file lists them.
pub const CORNERS: [&str; 4] = ["TopLeft", "TopRight", "BottomLeft", "BottomRight"];
/// Edges in the order the file lists them.
pub const EDGES: [&str; 4] = ["Top", "Right", "Bottom", "Left"];

/// A tile.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    pub model: String,
    pub walkmesh: Option<String>,
    /// Terrain and height at each corner, in [`CORNERS`] order.
    pub corners: [(String, i32); 4],
    /// Crosser on each edge (empty for none), in [`EDGES`] order.
    pub edges: [String; 4],
    pub main_lights: [bool; 2],
    pub source_lights: [bool; 2],
    pub anim_loops: [bool; 3],
    pub doors: Vec<TileDoor>,
    pub sounds: i32,
    pub path_node: Option<String>,
    pub orientation: i32,
    pub visibility_node: Option<String>,
    pub visibility_orientation: i32,
    pub door_visibility_node: Option<String>,
    pub door_visibility_orientation: i32,
    pub image_map_2d: Option<String>,
}

/// A group: a rectangle of tiles placed together (`None` cells are left as
/// they are).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub strref: StrRef,
    pub rows: u32,
    pub columns: u32,
    /// Row-major tile indices, `rows * columns` long.
    pub tiles: Vec<Option<u32>>,
}

/// A tileset.
#[derive(Debug, Clone, PartialEq)]
pub struct Tileset {
    pub general: General,
    pub grass: Grass,
    pub terrains: Vec<NamedType>,
    pub crossers: Vec<NamedType>,
    pub primary_rules: Vec<Rule>,
    pub secondary_rules: Vec<Rule>,
    pub tiles: Vec<Tile>,
    pub groups: Vec<Group>,
    pub ini: Ini,
    /// Inconsistencies found while parsing, which shipped tilesets have
    /// (tiles using undeclared terrains or crossers, missing door sections).
    pub warnings: Vec<String>,
}

fn section<'a>(ini: &'a Ini, name: &str) -> Result<&'a Section, SetError> {
    ini.section(name).ok_or_else(|| SetError::MissingSection(name.to_string()))
}

fn count(ini: &Ini, name: &str) -> usize {
    ini.section(name).and_then(|s| s.int("Count")).unwrap_or(0).max(0) as usize
}

fn strref(s: &Section, key: &str) -> StrRef {
    s.int(key).map_or(StrRef::NONE, |v| StrRef(v as u32))
}

fn owned(v: Option<&str>) -> Option<String> {
    v.map(str::to_string)
}

fn flag(s: &Section, key: &str) -> bool {
    s.int(key).unwrap_or(0) != 0
}

impl Tileset {
    /// Parses a tileset; text is decoded with `codepage`.
    pub fn parse(data: &[u8], codepage: Codepage) -> Result<Tileset, SetError> {
        let ini = Ini::parse(&codepage.decode(data));
        let g = section(&ini, "GENERAL")?;
        let general = General {
            name: g.get("Name").unwrap_or_default().to_string(),
            interior: flag(g, "Interior"),
            has_height_transition: flag(g, "HasHeightTransition"),
            env_map: owned(g.text("EnvMap")),
            transition: g.float("Transition").unwrap_or(0.0),
            display_name: strref(g, "DisplayName"),
            unlocalized_name: owned(g.text("UnlocalizedName")),
            border: g.get("Border").unwrap_or_default().to_string(),
            default: g.get("Default").unwrap_or_default().to_string(),
            floor: g.get("Floor").unwrap_or_default().to_string(),
            selector_height: g.float("SelectorHeight"),
        };
        let grass = ini.section("GRASS").map_or(Grass::default(), |s| {
            let rgb = |p: &str| {
                ["Red", "Green", "Blue"].map(|c| s.float(&format!("{p}{c}")).unwrap_or(0.0))
            };
            Grass {
                enabled: flag(s, "Grass"),
                density: s.float("Density").unwrap_or(0.0),
                height: s.float("Height").unwrap_or(0.0),
                ambient: rgb("Ambient"),
                diffuse: rgb("Diffuse"),
            }
        });
        let named = |list: &str, item: &str| -> Result<Vec<NamedType>, SetError> {
            (0..count(&ini, list))
                .map(|i| {
                    let s = section(&ini, &format!("{item}{i}"))?;
                    Ok(NamedType {
                        name: s.get("Name").unwrap_or_default().to_string(),
                        strref: strref(s, "StrRef"),
                        unlocalized_name: owned(s.text("UnlocalizedName")),
                    })
                })
                .collect()
        };
        let rules = |list: &str, item: &str| -> Result<Vec<Rule>, SetError> {
            (0..count(&ini, list))
                .map(|i| {
                    let s = section(&ini, &format!("{item}{i}"))?;
                    let t = |k: &str| s.get(k).unwrap_or_default().to_string();
                    let h = |k: &str| s.int(k).unwrap_or(0);
                    Ok(Rule {
                        placed: t("Placed"),
                        placed_height: h("PlacedHeight"),
                        adjacent: t("Adjacent"),
                        adjacent_height: h("AdjacentHeight"),
                        changed: t("Changed"),
                        changed_height: h("ChangedHeight"),
                    })
                })
                .collect()
        };
        let mut warnings = Vec::new();
        let tiles = (0..count(&ini, "TILES"))
            .map(|i| {
                let name = format!("TILE{i}");
                let s = section(&ini, &name)?;
                let model = s.text("Model").ok_or_else(|| SetError::MissingKey {
                    section: name.clone(),
                    key: "Model".into(),
                })?;
                let mut doors = Vec::new();
                for d in 0..s.int("Doors").unwrap_or(0).max(0) {
                    // Some shipped tiles declare doors without a section.
                    let Some(ds) = ini.section(&format!("TILE{i}DOOR{d}")) else {
                        warnings
                            .push(format!("tile {i}: door {d} has no [TILE{i}DOOR{d}] section"));
                        continue;
                    };
                    let f = |k: &str| ds.float(k).unwrap_or(0.0);
                    doors.push(TileDoor {
                        door_type: ds.int("Type").unwrap_or(0),
                        position: [f("X"), f("Y"), f("Z")],
                        orientation: f("Orientation"),
                    });
                }
                Ok(Tile {
                    model: model.to_string(),
                    walkmesh: owned(s.text("WalkMesh")),
                    corners: CORNERS.map(|c| {
                        (
                            s.get(c).unwrap_or_default().to_string(),
                            s.int(&format!("{c}Height")).unwrap_or(0),
                        )
                    }),
                    edges: EDGES.map(|e| s.get(e).unwrap_or_default().to_string()),
                    main_lights: [flag(s, "MainLight1"), flag(s, "MainLight2")],
                    source_lights: [flag(s, "SourceLight1"), flag(s, "SourceLight2")],
                    anim_loops: [flag(s, "AnimLoop1"), flag(s, "AnimLoop2"), flag(s, "AnimLoop3")],
                    doors,
                    sounds: s.int("Sounds").unwrap_or(0),
                    path_node: owned(s.text("PathNode")),
                    orientation: s.int("Orientation").unwrap_or(0),
                    visibility_node: owned(s.text("VisibilityNode")),
                    visibility_orientation: s.int("VisibilityOrientation").unwrap_or(0),
                    door_visibility_node: owned(s.text("DoorVisibilityNode")),
                    door_visibility_orientation: s.int("DoorVisibilityOrientation").unwrap_or(0),
                    image_map_2d: owned(s.text("ImageMap2D")),
                })
            })
            .collect::<Result<Vec<_>, SetError>>()?;
        let groups = (0..count(&ini, "GROUPS"))
            .map(|i| {
                let s = section(&ini, &format!("GROUP{i}"))?;
                let rows = s.int("Rows").unwrap_or(0).max(0) as u32;
                let columns = s.int("Columns").unwrap_or(0).max(0) as u32;
                let tiles = (0..rows * columns)
                    .map(|t| s.int(&format!("Tile{t}")).and_then(|v| u32::try_from(v).ok()))
                    .collect();
                Ok(Group {
                    name: s.get("Name").unwrap_or_default().to_string(),
                    strref: strref(s, "StrRef"),
                    rows,
                    columns,
                    tiles,
                })
            })
            .collect::<Result<Vec<_>, SetError>>()?;
        let mut t = Tileset {
            general,
            grass,
            terrains: named("TERRAIN TYPES", "TERRAIN")?,
            crossers: named("CROSSER TYPES", "CROSSER")?,
            primary_rules: rules("PRIMARY RULES", "PRIMARY RULE")?,
            secondary_rules: rules("SECONDARY RULES", "SECONDARY RULE")?,
            tiles,
            groups,
            ini,
            warnings,
        };
        t.check();
        Ok(t)
    }

    /// Records references to undeclared terrains, crossers and tiles.
    fn check(&mut self) {
        let mut w = Vec::new();
        for (i, tile) in self.tiles.iter().enumerate() {
            for (terrain, _) in &tile.corners {
                if self.terrain(terrain).is_none() {
                    w.push(format!("tile {i}: corner terrain {terrain:?} is not declared"));
                }
            }
            for crosser in tile.edges.iter().filter(|e| !e.is_empty()) {
                if self.crosser(crosser).is_none() {
                    w.push(format!("tile {i}: edge crosser {crosser:?} is not declared"));
                }
            }
        }
        for g in &self.groups {
            if let Some(bad) = g.tiles.iter().flatten().find(|&&i| i as usize >= self.tiles.len()) {
                w.push(format!("group {:?}: tile {bad} does not exist", g.name));
            }
        }
        for r in self.primary_rules.iter().chain(&self.secondary_rules) {
            for n in [&r.placed, &r.adjacent, &r.changed] {
                if self.terrain(n).is_none() {
                    w.push(format!("rule: terrain {n:?} is not declared"));
                }
            }
        }
        self.warnings.extend(w);
    }

    /// The index of a terrain by name (case-insensitive).
    pub fn terrain(&self, name: &str) -> Option<usize> {
        self.terrains.iter().position(|t| t.name.eq_ignore_ascii_case(name))
    }

    /// The index of a crosser by name (case-insensitive).
    pub fn crosser(&self, name: &str) -> Option<usize> {
        self.crossers.iter().position(|t| t.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[GENERAL]\nName=TST01\nInterior=0\nHasHeightTransition=1\nTransition=5\nDisplayName=1606\nBorder=Grass\nDefault=Grass\nFloor=Grass\n\
[GRASS]\nGrass=1\nDensity=5.0\nAmbientRed=0.5\n\
[TERRAIN TYPES]\nCount=2\n[TERRAIN0]\nName=Grass\nStrRef=1\n[TERRAIN1]\nName=Water\n\
[CROSSER TYPES]\nCount=1\n[CROSSER0]\nName=Road\n\
[PRIMARY RULES]\nCount=1\n[PRIMARY RULE0]\nPlaced=Water\nPlacedHeight=0\nAdjacent=Grass\nAdjacentHeight=1\nChanged=Grass\nChangedHeight=0\n\
[SECONDARY RULES]\nCount=0\n\
[TILES]\nCount=1\n[TILE0]\nModel=tst01_a01_01\nWalkMesh=msb01\nTopLeft=Grass\nTopLeftHeight=1\nTopRight=Water\nBottomLeft=Grass\nBottomRight=Grass\nTop=Road\nRight=\nMainLight1=1\nDoors=1\nOrientation=90\n[TILE0DOOR0]\nType=6\nX=5.0\nY=0\nOrientation=180\n\
[GROUPS]\nCount=1\n[GROUP0]\nName=Big\nRows=1\nColumns=2\nTile0=0\nTile1=-1\n";

    #[test]
    fn parses_a_tileset() {
        let t = Tileset::parse(SAMPLE.as_bytes(), Codepage::default()).unwrap();
        assert_eq!(t.general.name, "TST01");
        assert!(t.general.has_height_transition);
        assert_eq!(t.general.transition, 5.0);
        assert_eq!(t.general.display_name, StrRef(1606));
        assert!(t.grass.enabled);
        assert_eq!(t.grass.ambient, [0.5, 0.0, 0.0]);
        assert_eq!(t.terrain("water"), Some(1));
        assert_eq!(t.terrains[1].strref, StrRef::NONE);
        assert_eq!(t.primary_rules[0].adjacent_height, 1);
        let tile = &t.tiles[0];
        assert_eq!(tile.corners[0], ("Grass".to_string(), 1));
        assert_eq!(tile.corners[1], ("Water".to_string(), 0));
        assert_eq!(tile.edges, ["Road".to_string(), String::new(), String::new(), String::new()]);
        assert_eq!(tile.main_lights, [true, false]);
        assert_eq!(tile.doors[0].position, [5.0, 0.0, 0.0]);
        assert_eq!(tile.doors[0].orientation, 180.0);
        assert_eq!(tile.orientation, 90);
        assert_eq!(t.groups[0].tiles, vec![Some(0), None]);
    }

    #[test]
    fn missing_sections_are_errors() {
        assert_eq!(
            Tileset::parse(b"[TILES]\nCount=0\n", Codepage::default()).unwrap_err(),
            SetError::MissingSection("GENERAL".into())
        );
        let no_tile = "[GENERAL]\nName=x\n[TILES]\nCount=1\n";
        let no_door =
            "[GENERAL]\nName=x\n[TILES]\nCount=1\n[TILE0]\nModel=m\nTopLeft=Nope\nDoors=1\n";
        let t = Tileset::parse(no_door.as_bytes(), Codepage::default()).unwrap();
        assert!(t.tiles[0].doors.is_empty());
        assert_eq!(t.warnings.len(), 5, "{:?}", t.warnings);
        assert_eq!(
            Tileset::parse(no_tile.as_bytes(), Codepage::default()).unwrap_err(),
            SetError::MissingSection("TILE0".into())
        );
    }
}
