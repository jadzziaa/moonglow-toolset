//! Areas as the area editor shows them: the tile grid with its lights and
//! animation loops, the objects placed in it (the GIT's lists) with their
//! models and outlines, and the area's sun, moon, fog and sky.
//!
//! [`AreaModel`] reads an area (its ARE and GIT) without the GPU;
//! [`AreaScene`] loads the models and builds the renderer's scene from it at
//! a moment of the day.
//!
//! Conventions (checked in the engine, `tests/engine_facings.rs`): tile
//! (x, y) is `Tile_List[y * Width + x]`, 10 m square, its centre at
//! (10x + 5, 10y + 5) and its floor at `Tile_Height` height steps; a tile
//! turns `Tile_Orientation` quarter turns counter-clockwise. The game turns
//! an object's model by its facing less 90°: a door's or placeable's
//! `Bearing` is that turn itself; the other objects' orientation vector
//! points along their facing.

use std::f32::consts::FRAC_PI_2;

use glam::{Mat4, Quat, Vec3};
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct};
use mg_preview::Preview;
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_set::Tileset;

mod scene;

pub use scene::{AreaScene, View, overview};

/// A tile's edge, metres.
pub const TILE_SIZE: f32 = 10.0;

/// Why an area's tileset could not be read.
#[derive(Debug, thiserror::Error)]
pub enum AreaError {
    #[error("tileset {0}: not found")]
    NoTileset(ResRef),
    #[error("tileset {0}: {1}")]
    Tileset(ResRef, mg_set::SetError),
}

/// Reads the tileset `resref` (`.set`) through the game's resources.
pub fn tileset(game: &GameData, resref: ResRef) -> Result<Tileset, AreaError> {
    let data = game
        .resman
        .get(&ResKey::new(resref, ResType::SET))
        .map_err(|_| AreaError::NoTileset(resref))?;
    Tileset::parse(&data, game.language.codepage()).map_err(|e| AreaError::Tileset(resref, e))
}

/// A tile of the grid, as the ARE places it.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaTile {
    /// Its index in `Tile_List`.
    pub index: usize,
    pub column: u32,
    pub row: u32,
    /// `Tile_ID`: the tileset's tile.
    pub id: i64,
    /// The tile's model (lower case); `None` when the tileset has no such
    /// tile or was not found.
    pub model: Option<String>,
    /// In height steps.
    pub height: i32,
    /// Quarter turns counter-clockwise.
    pub orientation: u8,
    /// lightcolor.2da rows of main lights 1 and 2 (0: black, none).
    pub main_lights: [u8; 2],
    /// Source lights 1 and 2: the flame animation (`fx_flame01`) played at
    /// the tile's `sl1`/`sl2` node, 0 for none.
    pub source_lights: [u8; 2],
    /// Whether animation loops `animloop01`..`03` play.
    pub anim_loops: [bool; 3],
    /// The centre of the tile's floor.
    pub position: Vec3,
}

impl AreaTile {
    /// Places the tile's model.
    pub fn transform(&self) -> Mat4 {
        Mat4::from_rotation_translation(
            Quat::from_rotation_z(f32::from(self.orientation) * FRAC_PI_2),
            self.position,
        )
    }
}

/// What an object in an area is: the GIT list it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectKind {
    Creature,
    Door,
    Encounter,
    Item,
    Placeable,
    Sound,
    Store,
    Trigger,
    Waypoint,
}

impl ObjectKind {
    pub const ALL: [ObjectKind; 9] = [
        ObjectKind::Creature,
        ObjectKind::Door,
        ObjectKind::Encounter,
        ObjectKind::Item,
        ObjectKind::Placeable,
        ObjectKind::Sound,
        ObjectKind::Store,
        ObjectKind::Trigger,
        ObjectKind::Waypoint,
    ];

    /// Its blueprint type.
    pub fn restype(self) -> ResType {
        match self {
            ObjectKind::Creature => ResType::UTC,
            ObjectKind::Door => ResType::UTD,
            ObjectKind::Encounter => ResType::UTE,
            ObjectKind::Item => ResType::UTI,
            ObjectKind::Placeable => ResType::UTP,
            ObjectKind::Sound => ResType::UTS,
            ObjectKind::Store => ResType::UTM,
            ObjectKind::Trigger => ResType::UTT,
            ObjectKind::Waypoint => ResType::UTW,
        }
    }

    /// The GIT list its instances are in.
    pub fn list(self) -> &'static str {
        mg_module::instances::git_list(self.restype()).map_or("", |(list, _)| list)
    }

    /// Whether it is placed by `X`, `Y`, `Z` and `Bearing` (doors and
    /// placeables) rather than by position and orientation fields.
    pub fn has_bearing(self) -> bool {
        matches!(self, ObjectKind::Door | ObjectKind::Placeable)
    }

    /// Whether it has an outline (`Geometry`) around its position.
    pub fn has_outline(self) -> bool {
        matches!(self, ObjectKind::Trigger | ObjectKind::Encounter)
    }
}

/// An object placed in the area.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaObject {
    pub kind: ObjectKind,
    /// Its index in its GIT list.
    pub index: usize,
    pub position: Vec3,
    /// The model's turn around Z (its facing less 90°), radians.
    pub rotation: f32,
    pub tag: String,
    /// The blueprint it was made from (`TemplateResRef`; a store's
    /// `ResRef`).
    pub template: Option<ResRef>,
    /// Its models: creatures, doors, items and placeables. `None` for the
    /// others (drawn as markers) and when its appearance cannot be shown
    /// (see `problem`).
    pub preview: Option<Preview>,
    /// Why a creature, door, item or placeable has no preview.
    pub problem: Option<String>,
    /// A trigger's or encounter's outline, world space.
    pub outline: Vec<Vec3>,
}

impl AreaObject {
    /// Reads the object `index` of the GIT list of `kind`.
    pub fn read(game: &GameData, kind: ObjectKind, index: usize, s: &Struct) -> AreaObject {
        let float = |label: &str| s.float(label).unwrap_or(0.0);
        let (position, rotation) = if kind.has_bearing() {
            (Vec3::new(float("X"), float("Y"), float("Z")), float("Bearing"))
        } else {
            let position = Vec3::new(float("XPosition"), float("YPosition"), float("ZPosition"));
            let (x, y) = (float("XOrientation"), float("YOrientation"));
            let rotation =
                if kind.has_outline() || kind == ObjectKind::Sound || (x == 0.0 && y == 0.0) {
                    0.0
                } else {
                    y.atan2(x) - FRAC_PI_2
                };
            (position, rotation)
        };
        let outline = match kind {
            ObjectKind::Trigger => outline(s, ["PointX", "PointY", "PointZ"], position),
            ObjectKind::Encounter => outline(s, ["X", "Y", "Z"], position),
            _ => Vec::new(),
        };
        let items = |r: ResRef| -> Option<Gff> {
            let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
            Gff::read(&data).ok()
        };
        let preview = match kind {
            ObjectKind::Creature => Some(mg_preview::creature(game, s, &items)),
            ObjectKind::Door => Some(mg_preview::door(game, s)),
            ObjectKind::Item => Some(mg_preview::item(game, s)),
            ObjectKind::Placeable => Some(mg_preview::placeable(game, s)),
            _ => None,
        };
        let (preview, problem) = match preview {
            Some(Ok(p)) => (Some(p), None),
            Some(Err(e)) => (None, Some(e.to_string())),
            None => (None, None),
        };
        let template_field = if kind == ObjectKind::Store { "ResRef" } else { "TemplateResRef" };
        AreaObject {
            kind,
            index,
            position,
            rotation,
            tag: s
                .string("Tag")
                .map(|t| String::from_utf8_lossy(t).into_owned())
                .unwrap_or_default(),
            template: s.resref(template_field).filter(|r| !r.is_empty()),
            preview,
            problem,
            outline,
        }
    }

    /// Where its model stands and how it turns.
    pub fn transform(&self) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_z(self.rotation), self.position)
    }

    /// Its facing (what `GetFacing` reports), radians counter-clockwise
    /// from east.
    pub fn facing(&self) -> f32 {
        self.rotation + FRAC_PI_2
    }
}

/// An outline's points (`Geometry`, relative to the position) in world
/// space.
fn outline(s: &Struct, labels: [&str; 3], at: Vec3) -> Vec<Vec3> {
    s.list("Geometry")
        .unwrap_or(&[])
        .iter()
        .map(|p| {
            let c = |i: usize| p.float(labels[i]).unwrap_or(0.0);
            at + Vec3::new(c(0), c(1), c(2))
        })
        .collect()
}

/// Sun or moon settings of an area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sky {
    /// 0x00BBGGRR.
    pub ambient: u32,
    pub diffuse: u32,
    pub fog_color: u32,
    /// 0 (none) to 15.
    pub fog_amount: u8,
    pub shadows: bool,
}

impl Sky {
    fn read(are: &Struct, prefix: &str) -> Sky {
        let int = |label: &str| are.integer(&format!("{prefix}{label}")).unwrap_or(0);
        Sky {
            ambient: int("AmbientColor") as u32,
            diffuse: int("DiffuseColor") as u32,
            fog_color: int("FogColor") as u32,
            fog_amount: int("FogAmount").clamp(0, 255) as u8,
            shadows: int("Shadows") != 0,
        }
    }
}

/// An area's lighting.
#[derive(Debug, Clone, PartialEq)]
pub struct Lighting {
    pub sun: Sky,
    pub moon: Sky,
    /// Always night (`IsNight`), unless the day cycles.
    pub is_night: bool,
    pub day_night_cycle: bool,
    /// Where fog ends and tiles stop being drawn (`FogClipDist`, 45 when
    /// missing).
    pub fog_clip: f32,
    /// skyboxes.2da row, 0 for none.
    pub sky_box: u32,
    /// The tileset's environment map (`default` textures reflect it).
    pub env_map: Option<String>,
    pub shadow_opacity: u8,
}

impl Lighting {
    pub fn read(are: &Struct, tileset: Option<&Tileset>) -> Lighting {
        Lighting {
            sun: Sky::read(are, "Sun"),
            moon: Sky::read(are, "Moon"),
            is_night: are.integer("IsNight").unwrap_or(0) != 0,
            day_night_cycle: are.integer("DayNightCycle").unwrap_or(0) != 0,
            fog_clip: are.float("FogClipDist").unwrap_or(45.0),
            sky_box: are.integer("SkyBox").unwrap_or(0).clamp(0, i64::from(u32::MAX)) as u32,
            env_map: tileset
                .and_then(|t| t.general.env_map.clone())
                .map(|e| e.to_ascii_lowercase())
                .filter(|e| !e.is_empty()),
            shadow_opacity: are.integer("ShadowOpacity").unwrap_or(0).clamp(0, 100) as u8,
        }
    }

    /// Whether the area is shown at night by default: at night always
    /// (`IsNight`) unless the day cycles, when the toolset shows the day.
    pub fn night_by_default(&self) -> bool {
        self.is_night && !self.day_night_cycle
    }

    /// The sun's settings, or the moon's at night.
    pub fn sky(&self, night: bool) -> Sky {
        if night { self.moon } else { self.sun }
    }

    /// The skybox model for the time of day (skyboxes.2da `DAY` or
    /// `NIGHT`), lower case.
    pub fn sky_model(&self, game: &GameData, night: bool) -> Option<String> {
        if self.sky_box == 0 {
            return None;
        }
        let t = game.table("skyboxes").ok()?;
        let v = t.get(self.sky_box as usize, if night { "NIGHT" } else { "DAY" })?.trim();
        (!v.is_empty() && v != "****").then(|| v.to_ascii_lowercase())
    }
}

/// An area as the editor shows it.
#[derive(Debug, Clone)]
pub struct AreaModel {
    pub tileset: Option<ResRef>,
    /// Tiles along X (east) and Y (north).
    pub width: u32,
    pub height: u32,
    /// One step of tile height, metres (the tileset's `Transition`).
    pub height_step: f32,
    pub tiles: Vec<AreaTile>,
    pub objects: Vec<AreaObject>,
    pub lighting: Lighting,
    /// What could not be read: a missing tileset, unknown tiles.
    pub problems: Vec<String>,
}

impl AreaModel {
    /// Reads an area from its ARE and GIT (their root structs) with its
    /// tileset (`None` when it is missing: the tiles have no models).
    pub fn read(game: &GameData, are: &Struct, git: &Struct, tileset: Option<&Tileset>) -> Self {
        let mut problems = Vec::new();
        let width = are.integer("Width").unwrap_or(0).clamp(0, 1024) as u32;
        let height = are.integer("Height").unwrap_or(0).clamp(0, 1024) as u32;
        let step = tileset.map_or(0.0, |t| t.general.transition);
        let list = are.list("Tile_List").unwrap_or(&[]);
        if list.len() != (width * height) as usize {
            problems
                .push(format!("Tile_List has {} tiles for a {width} by {height} area", list.len()));
        }
        let mut unknown = 0;
        let tiles = list
            .iter()
            .enumerate()
            .take((width * height) as usize)
            .map(|(index, t)| {
                let int = |label: &str| t.integer(label).unwrap_or(0);
                let byte = |label: &str| int(label).clamp(0, 255) as u8;
                let id = int("Tile_ID");
                let model = tileset.and_then(|s| {
                    let tile = usize::try_from(id).ok().and_then(|i| s.tiles.get(i));
                    if tile.is_none() {
                        unknown += 1;
                    }
                    tile.map(|t| t.model.to_ascii_lowercase())
                });
                let (column, row) = (index as u32 % width.max(1), index as u32 / width.max(1));
                let tile_height = int("Tile_Height").clamp(-1024, 1024) as i32;
                AreaTile {
                    index,
                    column,
                    row,
                    id,
                    model,
                    height: tile_height,
                    orientation: int("Tile_Orientation").rem_euclid(4) as u8,
                    main_lights: [byte("Tile_MainLight1"), byte("Tile_MainLight2")],
                    source_lights: [byte("Tile_SrcLight1"), byte("Tile_SrcLight2")],
                    anim_loops: [1, 2, 3].map(|n| int(&format!("Tile_AnimLoop{n}")) != 0),
                    position: Vec3::new(
                        (column as f32 + 0.5) * TILE_SIZE,
                        (row as f32 + 0.5) * TILE_SIZE,
                        tile_height as f32 * step,
                    ),
                }
            })
            .collect();
        if unknown > 0 {
            problems.push(format!("{unknown} tiles are not in the tileset"));
        }
        let tileset_ref = are.resref("Tileset").filter(|r| !r.is_empty());
        if tileset.is_none() {
            problems.push(match tileset_ref {
                Some(r) => format!("tileset {r} not found"),
                None => "no tileset".into(),
            });
        }
        let mut objects = Vec::new();
        for kind in ObjectKind::ALL {
            for (i, s) in git.list(kind.list()).unwrap_or(&[]).iter().enumerate() {
                objects.push(AreaObject::read(game, kind, i, s));
            }
        }
        AreaModel {
            tileset: tileset_ref,
            width,
            height,
            height_step: step,
            tiles,
            objects,
            lighting: Lighting::read(are, tileset),
            problems,
        }
    }

    /// The object `index` of the GIT list of `kind`.
    pub fn object(&self, kind: ObjectKind, index: usize) -> Option<&AreaObject> {
        self.objects.iter().find(|o| o.kind == kind && o.index == index)
    }

    /// The area's extent: from the origin to its far corner.
    pub fn size(&self) -> (f32, f32) {
        (self.width as f32 * TILE_SIZE, self.height as f32 * TILE_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_gff::Value;

    fn object(fields: &[(&str, f32)]) -> Struct {
        let mut s = Struct::new(0);
        for (label, v) in fields {
            s.set(label, Value::Float(*v));
        }
        s
    }

    fn game() -> GameData {
        GameData::new(mg_resman::ResMan::new(), mg_tlk::Tlk::new(mg_core::Language::ENGLISH))
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn objects_turn_as_the_game_turns_them() {
        let game = game();
        // A placeable's bearing is the model's turn; its facing is 90° more.
        let p = object(&[("X", 1.0), ("Y", 2.0), ("Bearing", 0.5)]);
        let o = AreaObject::read(&game, ObjectKind::Placeable, 3, &p);
        assert_eq!((o.position, o.rotation, o.index), (Vec3::new(1.0, 2.0, 0.0), 0.5, 3));
        assert!(close(o.facing(), 0.5 + FRAC_PI_2));
        // A waypoint facing north (orientation (0, 1)) is not turned.
        let w = object(&[("XPosition", 4.0), ("YOrientation", 1.0), ("XOrientation", 0.0)]);
        let o = AreaObject::read(&game, ObjectKind::Waypoint, 0, &w);
        assert!(close(o.rotation, 0.0) && close(o.facing(), FRAC_PI_2));
        // Facing east: turned a quarter clockwise.
        let c = object(&[("XOrientation", 1.0), ("YOrientation", 0.0)]);
        let o = AreaObject::read(&game, ObjectKind::Creature, 0, &c);
        assert!(close(o.rotation, -FRAC_PI_2));
        assert!(o.preview.is_none() && o.problem.is_some(), "no game data: no appearance");
    }

    #[test]
    fn outlines_are_around_the_position() {
        let game = game();
        let mut t = object(&[("XPosition", 10.0), ("YPosition", 20.0)]);
        let point = |x: f32, y: f32| {
            let mut p = Struct::new(3);
            p.set("PointX", Value::Float(x));
            p.set("PointY", Value::Float(y));
            p.set("PointZ", Value::Float(0.5));
            p
        };
        t.set("Geometry", Value::List(vec![point(0.0, 0.0), point(2.0, 0.0), point(2.0, 3.0)]));
        let o = AreaObject::read(&game, ObjectKind::Trigger, 0, &t);
        assert_eq!(o.outline[2], Vec3::new(12.0, 23.0, 0.5));
        assert_eq!(o.rotation, 0.0);
    }

    #[test]
    fn tiles_stand_on_the_grid() {
        let game = game();
        let mut are = Struct::new(0);
        are.set("Width", Value::Int(2));
        are.set("Height", Value::Int(2));
        let tiles = (0..4)
            .map(|i| {
                let mut t = Struct::new(1);
                t.set("Tile_ID", Value::Int(i));
                t.set("Tile_Orientation", Value::Int(i));
                t.set("Tile_Height", Value::Int(i % 2));
                t.set("Tile_AnimLoop2", Value::Byte(1));
                t
            })
            .collect();
        are.set("Tile_List", Value::List(tiles));
        let area = AreaModel::read(&game, &are, &Struct::new(0xFFFF_FFFF), None);
        let t = &area.tiles[3];
        assert_eq!((t.column, t.row, t.orientation, t.height), (1, 1, 3, 1));
        assert_eq!(t.position, Vec3::new(15.0, 15.0, 0.0), "no tileset: no height step");
        assert_eq!(t.anim_loops, [false, true, false]);
        assert!(t.model.is_none());
        assert_eq!(area.problems, ["no tileset"]);
        assert_eq!(area.lighting.fog_clip, 45.0);
        // Quarter turns counter-clockwise: +X goes to +Y after one.
        let one = &area.tiles[1];
        let x = one.transform().transform_vector3(Vec3::X);
        assert!(close(x.x, 0.0) && close(x.y, 1.0));
    }
}
