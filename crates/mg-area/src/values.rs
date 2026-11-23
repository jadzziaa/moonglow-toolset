//! An area as plain values: what [`AreaModel::from_values`] builds an
//! area's model from, and what [`AreaModel::read`] makes of its files
//! first ([`Area::read`]: the ARE's and GIT's fields, each read here and
//! nowhere else). A game client, which has no files but a server's
//! messages, says the same with these: tiles by their tileset's number,
//! turn, height, lights and loops; the sun, moon and fog
//! ([`Lighting`]); the objects placed by kind, where they stand and how
//! they look ([`Look`]).
//!
//! What is missing is decided here, by the reader of the files (a field
//! left out is a value: its default); what is out of range is settled by
//! [`AreaModel::from_values`], for values from anywhere: a size past 1024
//! tiles, a turn past three quarters, a height past 1024 steps, a
//! direction of no length. Nothing given here makes it panic.
//!
//! ```
//! # use mg_area::values::{Area, Look, Object, Tile};
//! # use mg_area::{AreaModel, ObjectKind};
//! # use glam::Vec3;
//! # let game = mg_rules::GameData::new(mg_resman::ResMan::new(), mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
//! let area = Area {
//!     width: 2,
//!     height: 1,
//!     tiles: vec![Tile::new(0), Tile { orientation: 1, ..Tile::new(4) }],
//!     objects: vec![Object::facing(
//!         ObjectKind::Placeable,
//!         Vec3::new(5.0, 5.0, 0.0),
//!         90f32.to_radians(),
//!         Look::Model(mg_preview::Preview::model("plc_chest1")),
//!     )],
//!     ..Area::default()
//! };
//! let model = AreaModel::from_values(&game, area, None);
//! assert_eq!(model.tiles[1].orientation, 1);
//! assert_eq!(model.objects[0].rotation, 0.0, "facing north: the model unturned");
//! ```

use std::f32::consts::FRAC_PI_2;
use std::num::NonZeroU8;

use glam::Vec3;
use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_preview::{DressedLook, Preview};
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_rules::grass::AreaGrass;
use mg_rules::placement::Scoped;

#[allow(unused_imports)] // (For the documentation's links.)
use crate::AreaModel;
use crate::{LOCKED, Lighting, ObjectKind, Sky, VisualTransform};

/// An area: its size, tiles, lighting, grass and objects.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Area {
    /// The tileset's resource name (`Tileset`); the tileset itself is
    /// given to [`AreaModel::from_values`] beside these.
    pub tileset: Option<ResRef>,
    /// Tiles along X (east) and Y (north): `Width` and `Height`. More than
    /// 1024 either way is taken as 1024.
    pub width: u32,
    pub height: u32,
    /// The tiles row by row from the south-west: tile (x, y) is
    /// `tiles[y * width + x]` (`Tile_List`). Past `width * height`, left
    /// out; fewer, a problem of the model's and the rest of the grid empty.
    pub tiles: Vec<Tile>,
    /// The sun, moon, fog and sky; its `env_map` is the tileset's
    /// ([`AreaModel::from_values`] sets it from the tileset given, over
    /// what is here).
    pub lighting: Lighting,
    /// What grass grows (the GIT's `AreaProperties`: the tileset's grass
    /// turned off, and the overrides by surface material).
    pub grass: AreaGrass,
    /// The placed objects. Each one's place in its own GIT list
    /// ([`crate::AreaObject::index`]) is its place among the objects of its
    /// kind here, in order.
    pub objects: Vec<Object>,
}

/// A tile of the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    /// `Tile_ID`: the tileset's tile, its `[TILEn]` section. One the
    /// tileset has not is drawn as nothing and counted among the model's
    /// problems.
    pub id: i64,
    /// `Tile_Orientation`: quarter turns counter-clockwise (taken modulo 4).
    pub orientation: u8,
    /// `Tile_Height`: in the tileset's height steps (its `Transition`,
    /// metres), from -1024 to 1024.
    pub height: i32,
    /// `Tile_MainLight1` and `2`: lightcolor.2da rows, the colors of the
    /// light nodes `<model>ml1` and `ml2` (row 0 is black: no light).
    pub main_lights: [u8; 2],
    /// `Tile_SrcLight1` and `2`: the flames at the nodes `<model>sl1` and
    /// `sl2`. `None` is no flame (the file's 0); `n` the model
    /// `fx_flame01` playing its animation named `n` (the game's has `1` to
    /// `15`, the colors of Tile Properties' source lights: one it lacks
    /// draws nothing). These are the file's values: Moonglow Client found
    /// its server's area message sending one less (a file's 0 as 255;
    /// seen once, not checked here).
    pub source_lights: [Option<NonZeroU8>; 2],
    /// `Tile_AnimLoop1` to `3`: whether the model's animations `animloop01`
    /// to `03` play.
    pub anim_loops: [bool; 3],
    /// The replacetexture.2da row whose `TEXTURENAME` the model's meshes
    /// textured `replace_tex` are drawn with; `None` for none. (The file's
    /// `Tile_ReplaceTex` is one past the row, 0 for none and a tile without
    /// the field takes row 0: [`mg_rules::rows::replacement_row`].) A row
    /// past the table draws none.
    pub replace_texture: Option<u32>,
}

impl Tile {
    /// Tile `id` unturned, at height 0, without lights or loops: what a
    /// file's tile of no other field reads as, but for its replacement
    /// texture (none here; such a file's tile takes row 0).
    pub fn new(id: i64) -> Tile {
        Tile {
            id,
            orientation: 0,
            height: 0,
            main_lights: [0; 2],
            source_lights: [None; 2],
            anim_loops: [false; 3],
            replace_texture: None,
        }
    }

    /// A tile of an ARE's `Tile_List`.
    pub fn read(t: &Struct) -> Tile {
        let int = |label: &str| t.integer(label).unwrap_or(0);
        let byte = |label: &str| int(label).clamp(0, 255) as u8;
        Tile {
            id: int("Tile_ID"),
            orientation: int("Tile_Orientation").rem_euclid(4) as u8,
            height: int("Tile_Height").clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            main_lights: [byte("Tile_MainLight1"), byte("Tile_MainLight2")],
            source_lights: [byte("Tile_SrcLight1"), byte("Tile_SrcLight2")].map(NonZeroU8::new),
            anim_loops: [1, 2, 3].map(|n| int(&format!("Tile_AnimLoop{n}")) != 0),
            replace_texture: mg_rules::rows::replacement_row(
                t.integer("Tile_ReplaceTex"),
                usize::MAX,
            )
            .map(|r| u32::try_from(r).unwrap_or(u32::MAX)),
        }
    }
}

/// What an object looks like: its models, or why it has none.
// (Not boxed: each look is made once and moved into the model, which holds
// the preview unboxed.)
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Look {
    /// No models: a trigger or encounter (drawn as its outline), a
    /// waypoint without a flag, a store or sound without the game's marker
    /// model (drawn as markers).
    None,
    /// Its models, put together ([`mg_preview`]: the base, its parts, the
    /// animation it stands in, its lights).
    Model(Preview),
    /// Why a creature, door, item or placeable has no models (its
    /// appearance cannot be shown): drawn as a marker, the reason in
    /// [`crate::AreaObject::problem`].
    Problem(String),
}

impl Look {
    /// The look of the placed object `s` (its GIT struct, or a blueprint's)
    /// of `kind`, the items a creature wears read from its own list or
    /// through the game's resources: [`mg_preview::of_object`].
    pub fn read(game: &GameData, kind: ObjectKind, s: &Struct) -> Look {
        let items = |r: ResRef| -> Option<Gff> {
            let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
            Gff::read(&data).ok()
        };
        Self::look_with(
            mg_preview::of_object(game, kind, s, &items, mg_preview::Shown::Placed),
            kind,
        )
    }

    /// [`Look::read`], sharing `remembered`'s part-model texture caches across
    /// an area's objects.
    fn read_remembered(
        game: &GameData,
        kind: ObjectKind,
        s: &Struct,
        remembered: &mg_preview::Remembered,
    ) -> Look {
        let items = |r: ResRef| -> Option<Gff> {
            let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
            Gff::read(&data).ok()
        };
        Self::look_with(
            mg_preview::of_object_remembered(
                game,
                kind,
                s,
                &items,
                mg_preview::Shown::Placed,
                remembered,
            ),
            kind,
        )
    }

    fn look_with(
        preview: Option<Result<mg_preview::Preview, mg_preview::PreviewError>>,
        kind: ObjectKind,
    ) -> Look {
        // A waypoint without a flag (no appearance) is drawn as a marker:
        // nothing is wrong with it; a store or a sound without the game's
        // marker model, drawn as one.
        let preview = match kind {
            ObjectKind::Waypoint | ObjectKind::Store | ObjectKind::Sound => {
                preview.and_then(Result::ok).map(Ok)
            }
            _ => preview,
        };
        match preview {
            Some(Ok(p)) => Look::Model(p),
            Some(Err(e)) => Look::Problem(e.to_string()),
            None => Look::None,
        }
    }

    /// A creature as values: its body and what it wears and holds
    /// ([`DressedLook`], or a bare body's [`mg_preview::CreatureLook`],
    /// given or borrowed), drawn as [`mg_preview::dressed_look`] draws it:
    /// everything given is worn. The game takes off a creature what it may
    /// not wear as it loads it ([`mg_rules::GameData::wear_problem`]), so a
    /// client gives what its server says the creature wears, and a reader
    /// of files reads with [`Look::read`], which leaves those items out.
    /// No texture or animation replacement and no shader parameters: those
    /// are an object's fields, which [`Look::read`] reads.
    pub fn creature(game: &GameData, look: impl Into<DressedLook>) -> Look {
        Look::of(mg_preview::dressed_look(game, &look.into()))
    }

    /// A placeable of placeables.2da row `appearance` in animation state
    /// `state` (`AnimationState`: 0 default, 1 open, 2 closed, 3 dead, 4
    /// on, 5 off), `is_static` or not (a static one reflects no
    /// environment map and takes no tint).
    pub fn placeable(game: &GameData, appearance: u32, state: u8, is_static: bool) -> Look {
        let mut s = Struct::new(ObjectKind::Placeable.struct_id());
        s.set("Appearance", Value::Dword(appearance));
        s.set("AnimationState", Value::Byte(state));
        s.set("Static", Value::Byte(u8::from(is_static)));
        Look::read(game, ObjectKind::Placeable, &s)
    }

    fn of(made: Result<Preview, mg_preview::PreviewError>) -> Look {
        match made {
            Ok(p) => Look::Model(p),
            Err(e) => Look::Problem(e.to_string()),
        }
    }

    /// Its models and its problem, as [`crate::AreaObject`] holds them.
    pub(crate) fn split(self) -> (Option<Preview>, Option<String>) {
        match self {
            Look::None => (None, None),
            Look::Model(p) => (Some(p), None),
            Look::Problem(e) => (None, Some(e)),
        }
    }
}

/// A placed object.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub kind: ObjectKind,
    /// Where it stands, metres (an item's, a placeable's or a door's `X`,
    /// `Y`, `Z`; the others' `XPosition`…).
    pub position: Vec3,
    /// Its model's turn about Z, radians counter-clockwise: its facing
    /// less a quarter turn, so 0 faces north. A door's or placeable's
    /// `Bearing` is this turn itself; the others' `XOrientation` and
    /// `YOrientation` point along the facing. ([`Object::facing`] makes
    /// one from a facing.) Sounds, triggers and encounters don't turn.
    pub rotation: f32,
    pub look: Look,
    /// How an item lies on the ground (baseitems.2da `RotateOnGround`: 0
    /// as it is, 1 on its side about Y, 2 about X); 0 for other kinds.
    pub laid: u8,
    pub tag: String,
    /// The blueprint it was made from (`TemplateResRef`; a store's
    /// `ResRef`).
    pub template: Option<ResRef>,
    /// A trigger's or encounter's outline (`Geometry`), relative to its
    /// position.
    pub outline: Vec<Vec3>,
    /// Its model's visual transform (scope 0 of `VisTransformList`, or
    /// `VisualTransform`).
    pub visual: Option<VisualTransform>,
    /// Every entry of its `VisTransformList`.
    pub scopes: Vec<Scoped>,
    /// A placeable's `Static`.
    pub is_static: bool,
    /// A trigger's `Type` (0 generic, 1 area transition, 2 trap).
    pub trigger_type: i64,
    pub conversation: Option<ResRef>,
    /// An encounter's spawn points (`SpawnPointList`), where each is in
    /// the area (not relative to the encounter) and its facing, radians
    /// counter-clockwise from east.
    pub spawn_points: Vec<(Vec3, f32)>,
    /// Locked in the area editor ([`LOCKED`]).
    pub locked: bool,
    /// Its outline's own color, red, green and blue (an encounter's:
    /// [`mg_rules::placement::OUTLINE_COLOR`]); `None`, its kind's.
    pub outline_color: Option<[u8; 3]>,
    /// A door's or placeable's `AnimationState` (a door: 0 closed, 1 and 2
    /// open one way or the other).
    pub state: u8,
}

impl Object {
    /// An object of `kind` at `position` turned `rotation` (its model's
    /// turn: [`Object::rotation`]) that looks `look`, of no other field.
    pub fn new(kind: ObjectKind, position: Vec3, rotation: f32, look: Look) -> Object {
        Object {
            kind,
            position,
            rotation,
            look,
            laid: 0,
            tag: String::new(),
            template: None,
            outline: Vec::new(),
            visual: None,
            scopes: Vec::new(),
            is_static: false,
            trigger_type: 0,
            conversation: None,
            spawn_points: Vec::new(),
            locked: false,
            outline_color: None,
            state: 0,
        }
    }

    /// [`Object::new`] facing `facing`: radians counter-clockwise from east
    /// (what `GetFacing` reports, in degrees). Its model turns a quarter
    /// turn less.
    pub fn facing(kind: ObjectKind, position: Vec3, facing: f32, look: Look) -> Object {
        Object::new(kind, position, facing - FRAC_PI_2, look)
    }

    /// The placed object `s` of a GIT's list of `kind`, with its look.
    pub fn read(game: &GameData, kind: ObjectKind, s: &Struct) -> Object {
        let laid = if kind == ObjectKind::Item { mg_preview::ground_turn(game, s) } else { 0 };
        Object { look: Look::read(game, kind, s), laid, ..Object::placed(kind, s) }
    }

    /// [`Object::read`], sharing `remembered`'s part-model texture caches across
    /// an area's objects.
    fn read_remembered(
        game: &GameData,
        kind: ObjectKind,
        s: &Struct,
        remembered: &mg_preview::Remembered,
    ) -> Object {
        let laid = if kind == ObjectKind::Item { mg_preview::ground_turn(game, s) } else { 0 };
        Object {
            look: Look::read_remembered(game, kind, s, remembered),
            laid,
            ..Object::placed(kind, s)
        }
    }

    /// [`Object::read`] without the game: where it stands, how it turns,
    /// its outline and fields; no look, and an item not turned as it lies.
    pub fn placed(kind: ObjectKind, s: &Struct) -> Object {
        let at = mg_rules::placement::Placement::read(kind, s);
        let outline = mg_rules::placement::outline(kind, s);
        let points = if kind == ObjectKind::Encounter {
            let points = s.list("SpawnPointList").unwrap_or(&[]);
            let point = |p: &Struct| {
                let f = |l: &str| p.float(l).unwrap_or(0.0);
                (Vec3::new(f("X"), f("Y"), f("Z")), f("Orientation"))
            };
            points.iter().map(point).collect()
        } else {
            Vec::new()
        };
        Object {
            kind,
            position: Vec3::from(at.position),
            rotation: at.rotation,
            look: Look::None,
            laid: 0,
            tag: s
                .string("Tag")
                .map(|t| String::from_utf8_lossy(t).into_owned())
                .unwrap_or_default(),
            template: s.resref(kind.resref_field()).filter(|r| !r.is_empty()),
            outline: outline.into_iter().map(Vec3::from).collect(),
            visual: VisualTransform::read(s),
            scopes: mg_rules::placement::visual_scopes(s),
            is_static: kind == ObjectKind::Placeable && s.integer("Static").unwrap_or(0) != 0,
            trigger_type: if kind == ObjectKind::Trigger {
                s.integer("Type").unwrap_or(0)
            } else {
                0
            },
            conversation: s.resref("Conversation").filter(|r| !r.is_empty()),
            spawn_points: points,
            locked: s.integer(LOCKED).is_some_and(|v| v != 0),
            outline_color: mg_rules::placement::outline_color(s),
            state: s.integer("AnimationState").and_then(|v| u8::try_from(v).ok()).unwrap_or(0),
        }
    }
}

impl Area {
    /// An area's ARE and GIT (their root structs) as values.
    pub fn read(game: &GameData, are: &Struct, git: &Struct) -> Area {
        let size = |label: &str| are.integer(label).unwrap_or(0).clamp(0, i64::from(u32::MAX));
        let remembered = mg_preview::Remembered::default();
        let mut objects = Vec::new();
        for kind in ObjectKind::ALL {
            for s in git.list(kind.list()).unwrap_or(&[]) {
                objects.push(Object::read_remembered(game, kind, s, &remembered));
            }
        }
        Area {
            tileset: are.resref("Tileset").filter(|r| !r.is_empty()),
            width: size("Width") as u32,
            height: size("Height") as u32,
            tiles: are.list("Tile_List").unwrap_or(&[]).iter().map(Tile::read).collect(),
            lighting: Lighting::fields(are, git),
            grass: AreaGrass::read(git),
            objects,
        }
    }
}

impl Default for Lighting {
    /// What an ARE of no lighting field reads as: black sun and moon
    /// without fog or shadows, by day, fog ending at 45 m, no skybox.
    fn default() -> Lighting {
        Lighting {
            sun: Sky::default(),
            moon: Sky::default(),
            is_night: false,
            day_night_cycle: false,
            fog_clip: 45.0,
            sky_box: 0,
            env_map: None,
            shadow_opacity: 0,
            flags: 0,
            wind_power: 0,
            sun_direction: None,
            moon_direction: None,
            wallpaper: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AreaModel, Sky};

    fn game() -> GameData {
        GameData::new(mg_resman::ResMan::new(), mg_tlk::Tlk::new(mg_core::Language::ENGLISH))
    }

    /// Game data of a placeables.2da whose row 5 is a chest, and its model.
    fn chest_game() -> GameData {
        let mut c = mg_resman::MemContainer::new();
        let mut rows = String::from("2DA V2.0\n\n   Label ModelName\n");
        for r in 0..5 {
            rows += &format!("{r} ****  ****\n");
        }
        rows += "5 Chest PLC_Chest1\n";
        let key = |name: &str, t: ResType| ResKey::parse(name, t).unwrap();
        c.insert(key("placeables", ResType::TWODA), rows.into_bytes());
        let model = "newmodel plc_chest1\nsetsupermodel plc_chest1 NULL\nclassification \
                     character\nbeginmodelgeom plc_chest1\nnode dummy plc_chest1\n  parent \
                     NULL\nendnode\nendmodelgeom plc_chest1\ndonemodel plc_chest1\n";
        c.insert(key("plc_chest1", ResType::MDL), model.as_bytes().to_vec());
        let mut rm = mg_resman::ResMan::new();
        rm.add(mg_resman::priority::KEY, "game", mg_resman::LayerClass::Key, c);
        GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH))
    }

    /// An area's files as an editor writes them, and the same area as a
    /// game client says it in values (as Moonglow Client's `mc-scene`
    /// did, through fields): one model.
    #[test]
    fn an_area_s_values_are_its_fields() {
        let game = chest_game();
        let mut are = Struct::new(0);
        are.set("Width", Value::Int(2));
        are.set("Height", Value::Int(1));
        are.set("Tileset", Value::ResRef(b"tcn01".to_vec()));
        are.set("SunAmbientColor", Value::Dword(0x0030_2010));
        are.set("SunFogAmount", Value::Byte(4));
        are.set("MoonDiffuseColor", Value::Dword(0x0000_00ff));
        are.set("MoonShadows", Value::Byte(1));
        are.set("IsNight", Value::Byte(1));
        are.set("FogClipDist", Value::Float(22.0));
        are.set("SkyBox", Value::Byte(3));
        are.set("ShadowOpacity", Value::Byte(60));
        are.set("Flags", Value::Dword(4));
        are.set("WindPower", Value::Int(2));
        let mut first = Struct::new(1);
        first.set("Tile_ID", Value::Int(7));
        first.set("Tile_Orientation", Value::Int(3));
        first.set("Tile_Height", Value::Int(1));
        first.set("Tile_MainLight1", Value::Byte(12));
        first.set("Tile_SrcLight2", Value::Byte(5));
        first.set("Tile_AnimLoop3", Value::Byte(1));
        first.set("Tile_ReplaceTex", Value::Byte(2));
        // (A tile of no other field: row 0 of replacetexture.2da.)
        let mut second = Struct::new(1);
        second.set("Tile_ID", Value::Int(0));
        are.set("Tile_List", Value::List(vec![first, second]));

        let mut git = Struct::new(0xFFFF_FFFF);
        let mut props = Struct::new(100);
        for (axis, v) in [("X", 0.0f32), ("Y", 0.0), ("Z", 2.0)] {
            props.set(&format!("MoonDirection{axis}"), Value::Float(v));
        }
        git.set("AreaProperties", Value::Struct(props));
        let mut chest = Struct::new(ObjectKind::Placeable.struct_id());
        for (l, v) in [("X", 12.0f32), ("Y", 4.0), ("Z", 0.5), ("Bearing", 0.25)] {
            chest.set(l, Value::Float(v));
        }
        chest.set("Appearance", Value::Dword(5));
        chest.set("Static", Value::Byte(1));
        chest.set("Tag", Value::String(b"CHEST".to_vec()));
        let mut guard = Struct::new(ObjectKind::Creature.struct_id());
        for (l, v) in [("XPosition", 3.0f32), ("YPosition", 4.0), ("XOrientation", 1.0)] {
            guard.set(l, Value::Float(v));
        }
        guard.set("Appearance_Type", Value::Word(6));
        git.set(ObjectKind::Placeable.list(), Value::List(vec![chest]));
        git.set(ObjectKind::Creature.list(), Value::List(vec![guard.clone()]));
        let read = AreaModel::read(&game, &are, &git, None);

        let lighting = Lighting {
            sun: Sky { ambient: 0x0030_2010, fog_amount: 4, ..Sky::default() },
            moon: Sky { diffuse: 0x0000_00ff, shadows: true, ..Sky::default() },
            is_night: true,
            fog_clip: 22.0,
            sky_box: 3,
            shadow_opacity: 60,
            flags: 4,
            wind_power: 2,
            moon_direction: Some(Vec3::new(0.0, 0.0, 2.0)),
            ..Lighting::default()
        };
        let tiles = vec![
            Tile {
                orientation: 3,
                height: 1,
                main_lights: [12, 0],
                source_lights: [None, NonZeroU8::new(5)],
                anim_loops: [false, false, true],
                replace_texture: Some(1),
                ..Tile::new(7)
            },
            Tile { replace_texture: Some(0), ..Tile::new(0) },
        ];
        // (The kinds in the GIT's order: creatures before placeables.)
        let objects = vec![
            Object::facing(
                ObjectKind::Creature,
                Vec3::new(3.0, 4.0, 0.0),
                0.0,
                Look::read(&game, ObjectKind::Creature, &guard),
            ),
            Object {
                tag: "CHEST".into(),
                is_static: true,
                ..Object::new(
                    ObjectKind::Placeable,
                    Vec3::new(12.0, 4.0, 0.5),
                    0.25,
                    Look::placeable(&game, 5, 0, true),
                )
            },
        ];
        assert!(
            matches!(&objects[1].look, Look::Model(p) if p.base.model == "plc_chest1"),
            "{:?}",
            objects[1].look
        );
        let tileset = ResRef::from_str("tcn01").ok();
        let values =
            Area { tileset, width: 2, height: 1, tiles, lighting, objects, ..Area::default() };
        assert_eq!(Area::read(&game, &are, &git), values);
        let built = AreaModel::from_values(&game, values, None);
        assert_eq!(built, read);
        // (What the model makes of them: the moon's direction of unit
        // length, the flame of animation 5 at the tile's second source
        // light, each object the first of its kind.)
        assert_eq!(built.lighting.moon_direction, Some(Vec3::Z));
        assert_eq!(built.tiles[0].source_lights, [0, 5]);
        assert_eq!(built.objects[0].facing(), 0.0);
        assert_eq!((built.objects[0].index, built.objects[1].index), (0, 0));
    }

    /// Values from anywhere (a client's message, garbled) make a model,
    /// settled as the files' would be, and never a panic.
    #[test]
    fn values_out_of_range_are_settled() {
        let game = game();
        let area = Area {
            width: u32::MAX,
            height: 3,
            tiles: vec![
                Tile {
                    orientation: 255,
                    height: i32::MIN,
                    replace_texture: Some(u32::MAX),
                    source_lights: [NonZeroU8::new(255), None],
                    ..Tile::new(-4)
                },
                Tile { height: i32::MAX, ..Tile::new(i64::MAX) },
            ],
            lighting: Lighting {
                sun_direction: Some(Vec3::ZERO),
                moon_direction: Some(Vec3::new(f32::NAN, 0.0, 1.0)),
                shadow_opacity: 255,
                fog_clip: f32::NAN,
                ..Lighting::default()
            },
            objects: vec![
                Object::new(ObjectKind::Trigger, Vec3::NAN, f32::INFINITY, Look::None),
                Object {
                    outline: vec![Vec3::X, Vec3::Y],
                    spawn_points: vec![(Vec3::ONE, 1.0)],
                    ..Object::new(ObjectKind::Encounter, Vec3::splat(2.0), 0.0, Look::None)
                },
            ],
            ..Area::default()
        };
        let m = AreaModel::from_values(&game, area, None);
        assert_eq!((m.width, m.height), (1024, 3));
        assert_eq!(m.problems[0], "Tile_List has 2 tiles for a 1024 by 3 area");
        let t = &m.tiles[0];
        assert_eq!((t.orientation, t.height, t.id), (3, -1024, -4));
        assert_eq!(t.replace_texture, None);
        assert_eq!(m.tiles[1].height, 1024);
        assert_eq!((m.lighting.sun_direction, m.lighting.moon_direction), (None, None));
        assert_eq!(m.lighting.shadow_opacity, 100);
        let encounter = &m.objects[1];
        assert_eq!(encounter.outline, [Vec3::new(3.0, 2.0, 2.0), Vec3::new(2.0, 3.0, 2.0)]);
        assert_eq!(encounter.spawn_points, [Vec3::ONE]);
        assert_eq!(encounter.spawn_facings, [1.0]);
        // An object at no number: under no tile.
        let x = m.objects[0].position.x;
        assert_eq!(crate::tile_at(m.width, m.tiles.len(), x, 0.0), None);
        // An area of nothing.
        let empty = AreaModel::from_values(&game, Area::default(), None);
        assert!(empty.tiles.is_empty() && empty.objects.is_empty());
        assert_eq!(empty.problems, ["no tileset"]);
    }

    /// An object moved where a client says it walked is the object read
    /// from its fields there: its outline goes with it.
    #[test]
    fn an_object_moved_is_the_object_read_where_it_went() {
        let point = |x: f32, y: f32| {
            let mut p = Struct::new(3);
            for (l, v) in [("PointX", x), ("PointY", y), ("PointZ", 0.0)] {
                p.set(l, Value::Float(v));
            }
            p
        };
        let at = |x: f32, y: f32| {
            let mut t = Struct::new(ObjectKind::Trigger.struct_id());
            t.set("XPosition", Value::Float(x));
            t.set("YPosition", Value::Float(y));
            t.set("Geometry", Value::List(vec![point(0.0, 0.0), point(2.0, 1.0)]));
            t
        };
        let mut moved = crate::AreaObject::placed(ObjectKind::Trigger, 0, &at(1.0, 1.0));
        moved.move_to(Vec3::new(5.0, 6.0, 0.0), 0.0);
        assert_eq!(moved, crate::AreaObject::placed(ObjectKind::Trigger, 0, &at(5.0, 6.0)));
    }
}
