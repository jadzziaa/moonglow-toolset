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

pub mod arrange;
pub mod edit;
pub mod grass;
mod grass_view;
pub mod marker;
pub mod pick;
pub mod reshape;
mod scene;
pub mod terrain;
pub mod walk;

pub use grass_view::{CHUNK, GrassDraw, Grown, chunk_of, grass_chosen};
pub use mg_rules::rows::{replacement_row, replacement_value};
pub use scene::{AreaScene, BareLight, Sight, Usage, View, fog, overview};

/// The texture a tile model's mesh is drawn with where the tile's
/// `Tile_ReplaceTex` replaces it ([`replacement_row`]).
pub const REPLACE_TEX: &str = "replace_tex";

/// A tile's edge, metres.
pub const TILE_SIZE: f32 = 10.0;

/// How far above the ground sounds are placed, metres (three quarters of
/// the sounds in the shipped areas stand exactly this high).
pub const SOUND_HEIGHT: f32 = 1.5;

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
    /// The texture the model's `replace_tex` is drawn with (lower case):
    /// see [`replacement_row`] (`mg_rules::rows`).
    pub replace_texture: Option<String>,
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

/// Which of a tile's lights and animation loops its model has: what
/// Aurora's Tile Properties enables (the others are greyed out). The
/// tileset's `MainLight1`..`AnimLoop3` flags don't tell (the game's sets
/// have 1 for every tile); the model does: a light node `<model>ml1` or
/// `ml2` for a main light, a node (a dummy) `<model>sl1` or `sl2` where a
/// source light's flame goes, and an animation `animloop01`..`03` for a
/// loop. Its replacement texture (EE's Tile Properties alone: Aurora has
/// none) where a mesh is drawn with a texture named `replace_tex`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TileFeatures {
    pub main: [bool; 2],
    pub source: [bool; 2],
    pub loops: [bool; 3],
    pub replace: bool,
}

impl TileFeatures {
    /// Every light and loop (what a model not found is taken to have).
    pub const ALL: TileFeatures =
        TileFeatures { main: [true; 2], source: [true; 2], loops: [true; 3], replace: true };

    /// What tile model `model`, read from resource `resref`, has. A node
    /// counts named after the model's own name or the resource's (they
    /// are the same in the game's tilesets), case aside.
    pub fn of(model: &mg_mdl::Model, resref: &str) -> TileFeatures {
        // (A main light is a light node; a source light's place any node.)
        let has = |suffix: &str, light: bool| {
            model.nodes.iter().any(|n| {
                let named = [model.name.as_str(), resref].iter().any(|base| {
                    n.name.len() == base.len() + suffix.len()
                        && n.name.get(..base.len()).is_some_and(|b| b.eq_ignore_ascii_case(base))
                        && n.name.get(base.len()..).is_some_and(|s| s.eq_ignore_ascii_case(suffix))
                });
                named && (!light || matches!(n.kind, mg_mdl::NodeKind::Light(_)))
            })
        };
        TileFeatures {
            main: [has("ml1", true), has("ml2", true)],
            source: [has("sl1", false), has("sl2", false)],
            loops: ["animloop01", "animloop02", "animloop03"].map(|a| model.animation(a).is_some()),
            replace: model.draws_texture(REPLACE_TEX),
        }
    }

    /// What any of `all` has (Aurora's Tile Properties of several tiles
    /// enables what any of them has).
    pub fn union(all: impl IntoIterator<Item = TileFeatures>) -> TileFeatures {
        all.into_iter().fold(TileFeatures::default(), |a, b| TileFeatures {
            main: [a.main[0] | b.main[0], a.main[1] | b.main[1]],
            source: [a.source[0] | b.source[0], a.source[1] | b.source[1]],
            loops: [a.loops[0] | b.loops[0], a.loops[1] | b.loops[1], a.loops[2] | b.loops[2]],
            replace: a.replace | b.replace,
        })
    }
}

/// A place a door can stand: a tile's door hook (the tileset's
/// `[TILEnDOORm]`), where Aurora puts a door placed near it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorHook {
    pub tile: usize,
    /// doortypes.2da row (0: generic doors).
    pub door_type: i32,
    pub position: Vec3,
    /// The door's `Bearing` there, radians in (−π, π] (the hook's
    /// orientation turned with the tile).
    pub bearing: f32,
}

/// What an object in an area is: the GIT list it is in.
pub use mg_rules::ObjectKind;

/// An EE visual transform (scale, rotation in degrees and translation of
/// the model, not of the object): `VisTransformList`'s entry for scope 0,
/// or the older `VisualTransform` struct. The fields as the file has them
/// (`ScaleX`… `TranslateZ`); what they do to the model is
/// [`VisualTransform::matrix`], as the game client draws it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualTransform {
    /// `ScaleX`, `ScaleY`, `ScaleZ`. The game scales the model by `ScaleX`
    /// alone, the same every way (the script's one `SCALE`); Aurora and
    /// Adjust Location write the three alike.
    pub scale: Vec3,
    /// `RotateX`, `RotateY`, `RotateZ` (Adjust Location's X, Y and Z
    /// Rotation), degrees. Not about the axes they are named for: X Rotation
    /// turns the model about its upright axis (as Bearing does), Y Rotation
    /// tips it about its own X (east at Bearing 0) and Z Rotation leans it
    /// about its own Y.
    pub rotate: Vec3,
    /// `TranslateX`, `TranslateY`, `TranslateZ`: metres along the object's
    /// own axes (turned by its Bearing, not by the rotation above).
    pub translate: Vec3,
}

impl Default for VisualTransform {
    fn default() -> VisualTransform {
        VisualTransform { scale: Vec3::ONE, rotate: Vec3::ZERO, translate: Vec3::ZERO }
    }
}

impl VisualTransform {
    /// A placed object's, as its model is drawn with it (`None` without
    /// one): [`mg_module::placement::visual_transform`].
    pub fn read(s: &Struct) -> Option<VisualTransform> {
        let t = mg_module::placement::visual_transform(s)?;
        Some(VisualTransform {
            scale: Vec3::from(t.scale),
            rotate: Vec3::from(t.rotate),
            translate: Vec3::from(t.translate),
        })
    }

    pub fn is_identity(&self) -> bool {
        *self == VisualTransform::default()
    }

    /// The model's placement within the object, as the game client draws
    /// it (`visual_transforms_look` in `client_render.rs`): scaled by
    /// `ScaleX` every way; leaned about its Y by `RotateZ`, then tipped
    /// about its X by `RotateY`, then turned about its upright axis by
    /// `RotateX`; then moved by the translation.
    pub fn matrix(&self) -> Mat4 {
        mg_preview::visual_matrix(self.scale.x, self.rotate.into(), self.translate.into())
    }

    /// The rotation alone (of [`VisualTransform::matrix`]).
    pub fn turn(&self) -> Mat4 {
        mg_preview::visual_turn(self.rotate.into())
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
    /// Its models: creatures, doors, items, placeables, and the markers of
    /// waypoints (their flags) and stores. `None` for the others (drawn as markers) and when its
    /// appearance cannot be shown (see `problem`).
    pub preview: Option<Preview>,
    /// Why a creature, door, item or placeable has no preview.
    pub problem: Option<String>,
    /// A trigger's or encounter's outline, world space.
    pub outline: Vec<Vec3>,
    /// Its model's visual transform.
    pub visual: Option<VisualTransform>,
    /// Every entry of its `VisTransformList`: the whole object's (scope 0)
    /// with its animation speed and lerps in progress, and a creature's
    /// head, tail, wings and cloak (drawn on those parts:
    /// `mg_preview::scope_matrix`). What Adjust Location shows of them.
    pub scopes: Vec<mg_module::placement::Scoped>,
    /// A placeable's `Static`: part of the scenery. The game gives a
    /// static placeable no visual transform (Aurora resets it).
    pub is_static: bool,
    /// A trigger's `Type` (0 generic, 1 area transition, 2 trap).
    pub trigger_type: i64,
    /// Its conversation (`Conversation`), if it has one.
    pub conversation: Option<ResRef>,
    /// An encounter's spawn points (`SpawnPointList`), world space.
    pub spawn_points: Vec<Vec3>,
    /// Each spawn point's facing (`Orientation`): radians anticlockwise
    /// from east, the way what spawns there faces.
    pub spawn_facings: Vec<f32>,
    /// Locked in the area editor ([`LOCKED`]): not picked by clicks or boxes.
    pub locked: bool,
    /// A door's or placeable's `AnimationState` (a door: 0 closed, 1 and 2
    /// open one way or the other).
    pub state: u8,
    /// How an item is turned where it lies (`mg_preview::ground_turn`:
    /// baseitems.2da's `RotateOnGround`).
    pub laid: u8,
}

/// The field Moonglow marks a locked object with: a byte the game and
/// Aurora don't read (Aurora drops it when it saves). A nasher project
/// keeps it out of its files ([`mg_module::locks`]).
pub use mg_module::locks::LOCKED;

impl AreaObject {
    /// Reads the object `index` of the GIT list of `kind`.
    pub fn read(game: &GameData, kind: ObjectKind, index: usize, s: &Struct) -> AreaObject {
        let items = |r: ResRef| -> Option<Gff> {
            let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
            Gff::read(&data).ok()
        };
        let preview = mg_preview::of_object(game, kind, s, &items, mg_preview::Shown::Placed);
        // A waypoint without a flag (no appearance) is drawn as a marker:
        // nothing is wrong with it; a store or a sound without the game's
        // marker model, drawn as one.
        let preview = match kind {
            ObjectKind::Waypoint | ObjectKind::Store | ObjectKind::Sound => {
                preview.and_then(Result::ok).map(Ok)
            }
            _ => preview,
        };
        let (preview, problem) = match preview {
            Some(Ok(p)) => (Some(p), None),
            Some(Err(e)) => (None, Some(e.to_string())),
            None => (None, None),
        };
        let laid = if kind == ObjectKind::Item { mg_preview::ground_turn(game, s) } else { 0 };
        AreaObject { preview, problem, laid, ..AreaObject::placed(kind, index, s) }
    }

    /// The object `index` of the GIT list of `kind` as it is placed (where
    /// it stands, how it turns, its outline and fields), without the game:
    /// no preview, and an item not turned as it lies.
    pub fn placed(kind: ObjectKind, index: usize, s: &Struct) -> AreaObject {
        let at = mg_module::placement::Placement::read(kind, s);
        let (position, rotation) = (Vec3::from(at.position), at.rotation);
        let points = mg_module::placement::outline(kind, s);
        let outline = points.into_iter().map(|p| position + Vec3::from(p)).collect();
        let template_field = kind.resref_field();
        AreaObject {
            laid: 0,
            kind,
            index,
            position,
            rotation,
            tag: s
                .string("Tag")
                .map(|t| String::from_utf8_lossy(t).into_owned())
                .unwrap_or_default(),
            template: s.resref(template_field).filter(|r| !r.is_empty()),
            preview: None,
            problem: None,
            outline,
            visual: VisualTransform::read(s),
            scopes: mg_module::placement::visual_scopes(s),
            is_static: kind == ObjectKind::Placeable && s.integer("Static").unwrap_or(0) != 0,
            trigger_type: if kind == ObjectKind::Trigger {
                s.integer("Type").unwrap_or(0)
            } else {
                0
            },
            conversation: s.resref("Conversation").filter(|r| !r.is_empty()),
            locked: s.integer(LOCKED).is_some_and(|v| v != 0),
            state: s.integer("AnimationState").and_then(|v| u8::try_from(v).ok()).unwrap_or(0),
            spawn_points: if kind == ObjectKind::Encounter {
                s.list("SpawnPointList")
                    .unwrap_or(&[])
                    .iter()
                    .map(|p| {
                        let f = |l: &str| p.float(l).unwrap_or(0.0);
                        Vec3::new(f("X"), f("Y"), f("Z"))
                    })
                    .collect()
            } else {
                Vec::new()
            },
            spawn_facings: if kind == ObjectKind::Encounter {
                let points = s.list("SpawnPointList").unwrap_or(&[]);
                points.iter().map(|p| p.float("Orientation").unwrap_or(0.0)).collect()
            } else {
                Vec::new()
            },
        }
    }

    /// Where its model stands and how it turns.
    pub fn transform(&self) -> Mat4 {
        let stands =
            Mat4::from_rotation_translation(Quat::from_rotation_z(self.rotation), self.position);
        // An item lies as its base item says (`RotateOnGround`): a sword
        // and a shield on their flat, a potion stood up.
        match self.laid {
            1 => stands * Mat4::from_rotation_y(FRAC_PI_2),
            2 => stands * Mat4::from_rotation_x(FRAC_PI_2),
            _ => stands,
        }
    }

    /// Where its model is drawn: where it stands, with its visual
    /// transform if it takes one (the game draws a static placeable
    /// upright whatever its file says: `placeables_look`, in the client).
    pub fn model_transform(&self) -> Mat4 {
        match &self.visual {
            Some(v) if self.takes_visual_transform() => self.transform() * v.matrix(),
            _ => self.transform(),
        }
    }

    /// Whether its model takes a visual transform: creatures, items,
    /// doors and placeables that aren't static.
    pub fn takes_visual_transform(&self) -> bool {
        match self.kind {
            ObjectKind::Creature | ObjectKind::Item | ObjectKind::Door => true,
            ObjectKind::Placeable => !self.is_static,
            _ => false,
        }
    }

    /// Its facing (what `GetFacing` reports), radians counter-clockwise
    /// from east.
    pub fn facing(&self) -> f32 {
        self.rotation + FRAC_PI_2
    }
}

/// The door hooks of `tiles` (with the tileset they are from).
fn tiles_hooks(tiles: &[AreaTile], tileset: Option<&Tileset>) -> Vec<DoorHook> {
    let Some(set) = tileset else { return Vec::new() };
    let mut out = Vec::new();
    for t in tiles {
        let Some(tile) = usize::try_from(t.id).ok().and_then(|i| set.tiles.get(i)) else {
            continue;
        };
        let to = t.transform();
        for d in &tile.doors {
            let turn = f32::from(t.orientation) * 90.0;
            let degrees = (d.orientation + turn).rem_euclid(360.0);
            let degrees = if degrees > 180.0 { degrees - 360.0 } else { degrees };
            out.push(DoorHook {
                tile: t.index,
                door_type: d.door_type,
                position: to.transform_point3(Vec3::from_array(d.position)),
                bearing: degrees.to_radians(),
            });
        }
    }
    out
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
    /// The ARE's `Flags` (1 interior, 2 underground, 4 natural) and
    /// `WindPower` (0 to 2): what the game's shaders read of the area
    /// (`areaFlags`, `areaGlobalWind`: wind-swayed grass and leaves).
    pub flags: u32,
    pub wind_power: i64,
    /// The directions towards the sun and the moon its GIT gives
    /// (`AreaProperties` `SunDirectionX/Y/Z`, `MoonDirectionX/Y/Z`), which
    /// the client lights the area and casts its shadows by (seen:
    /// `reading_git_sun_direction` in `client_render.rs`); `None`: the
    /// game's own.
    pub sun_direction: Option<Vec3>,
    pub moon_direction: Option<Vec3>,
}

impl Lighting {
    /// [`Lighting::read`], with the directions of the sun and moon the
    /// GIT whose root is `git` gives.
    pub fn read_with(are: &Struct, git: &Struct, tileset: Option<&Tileset>) -> Lighting {
        let props = match git.get("AreaProperties") {
            Some(mg_gff::Value::Struct(p)) => Some(p),
            _ => None,
        };
        let direction = |prefix: &str| {
            let props = props?;
            let c = |axis: &str| props.float(&format!("{prefix}Direction{axis}"));
            let d = Vec3::new(c("X")?, c("Y")?, c("Z")?);
            (d.is_finite() && d.length_squared() > 1e-12).then(|| d.normalize())
        };
        Lighting {
            sun_direction: direction("Sun"),
            moon_direction: direction("Moon"),
            ..Lighting::read(are, tileset)
        }
    }

    /// The direction towards the sun, or the moon at night: the GIT's,
    /// else the game's own.
    pub fn direction(&self, night: bool) -> Option<Vec3> {
        if night { self.moon_direction } else { self.sun_direction }
    }

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
            flags: are.integer("Flags").unwrap_or(0).clamp(0, i64::from(u32::MAX)) as u32,
            wind_power: are.integer("WindPower").unwrap_or(0),
            sun_direction: None,
            moon_direction: None,
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
    /// The tiles' door hooks.
    pub hooks: Vec<DoorHook>,
    pub lighting: Lighting,
    /// What grass grows on the tiles' walkmesh faces, by surface material
    /// (the tileset's and the area's overrides: [`grass::AreaGrass`]).
    pub grass: Vec<(u32, grass::GrassLook)>,
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
        let mut holes = Vec::new();
        let replacements = game.table("replacetexture").ok();
        let tiles: Vec<AreaTile> = list
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
                    // (A hole in the tileset: nothing, as in the game.)
                    if tile.is_some_and(mg_set::Tile::is_hole) && !holes.contains(&id) {
                        holes.push(id);
                    }
                    tile.filter(|t| !t.is_hole()).map(|t| t.model.to_ascii_lowercase())
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
                    replace_texture: replacements.as_ref().and_then(|table| {
                        let row = replacement_row(t.integer("Tile_ReplaceTex"), table.len())?;
                        Some(table.get(row, "TEXTURENAME")?.to_ascii_lowercase())
                    }),
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
        if !holes.is_empty() {
            holes.sort_unstable();
            let named: Vec<String> = holes.iter().map(|t| format!("tile {t}")).collect();
            problems.push(format!(
                "the tileset names no model for {} (its section or Model= is missing)",
                named.join(", ")
            ));
        }
        let tileset_ref = are.resref("Tileset").filter(|r| !r.is_empty());
        if tileset.is_none() {
            problems.push(match tileset_ref {
                Some(r) => format!("tileset {r} not found"),
                None => "no tileset".into(),
            });
        }
        let hooks = tiles_hooks(&tiles, tileset);
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
            hooks,
            lighting: Lighting::read_with(are, git, tileset),
            grass: grass::AreaGrass::read(git).growing(tileset),
            problems,
        }
    }

    /// The door hook nearest to `p` (in the ground plane) within `reach`
    /// metres; of the tile `p` is on first (a doorway between two tiles
    /// has a hook in each, facing opposite ways: Aurora takes the one of
    /// the tile under the pointer).
    pub fn hook_near(&self, p: Vec3, reach: f32) -> Option<&DoorHook> {
        let tile = (p.x >= 0.0 && p.y >= 0.0).then(|| {
            let (x, y) = ((p.x / TILE_SIZE) as u32, (p.y / TILE_SIZE) as u32);
            (x < self.width).then_some((y * self.width + x) as usize)
        });
        let on_tile = |h: &DoorHook| tile.flatten() == Some(h.tile);
        self.hooks
            .iter()
            .map(|h| (h, (h.position - p).truncate().length()))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| on_tile(b.0).cmp(&on_tile(a.0)).then(a.1.total_cmp(&b.1)))
            .map(|(h, _)| h)
    }

    /// The door hooks at `p` (within 5 cm).
    pub fn hooks_at(&self, p: Vec3) -> impl Iterator<Item = &DoorHook> {
        self.hooks.iter().filter(move |h| (h.position - p).length() < 0.05)
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

    /// A tile model with main light 1, source light 2's place (named in
    /// another case) and animation loop 2; a light named ml2 of another
    /// model's name is not this tile's.
    #[test]
    fn a_tiles_lights_and_loops_are_its_models() {
        let text = "newmodel tst01_a01_01\nsetsupermodel tst01_a01_01 NULL\n\
classification tile\nbeginmodelgeom tst01_a01_01\n\
node dummy tst01_a01_01\n  parent NULL\nendnode\n\
node light tst01_a01_01ml1\n  parent tst01_a01_01\n  radius 10\nendnode\n\
node light other_ml2\n  parent tst01_a01_01\n  radius 5\nendnode\n\
node dummy TST01_A01_01SL2\n  parent tst01_a01_01\nendnode\n\
endmodelgeom tst01_a01_01\n\
newanim animloop02 tst01_a01_01\n  length 1\n  transtime 0\n  animroot tst01_a01_01\n\
  node dummy tst01_a01_01\n    parent NULL\n  endnode\n\
doneanim animloop02 tst01_a01_01\ndonemodel tst01_a01_01\n";
        let model = mg_mdl::Model::read(text.as_bytes()).unwrap();
        let has = TileFeatures::of(&model, "tst01_a01_01");
        assert_eq!(has.main, [true, false]);
        assert_eq!(has.source, [false, true]);
        assert_eq!(has.loops, [false, true, false]);
        // (Named after the resource, where the model's own name differs.)
        assert_eq!(TileFeatures::of(&model, "renamed"), has);
        let none = TileFeatures::default();
        assert_eq!(TileFeatures::union([has, none]), has);
        let other = TileFeatures { loops: [true, false, false], ..none };
        assert_eq!(TileFeatures::union([has, other]).loops, [true, true, false]);
        assert!(!has.replace, "no mesh drawn with replace_tex");
    }

    /// A tile model has a replacement texture where a mesh's bitmap is
    /// `replace_tex` (any case), and not where only another texture is.
    #[test]
    fn a_tile_s_replacement_texture_is_its_model_s() {
        let model = |bitmap: &str| {
            let text = format!(
                "newmodel tst01_a01_01\nsetsupermodel tst01_a01_01 NULL\nclassification tile\n\
beginmodelgeom tst01_a01_01\nnode dummy tst01_a01_01\n  parent NULL\nendnode\n\
node trimesh flag\n  parent tst01_a01_01\n  bitmap {bitmap}\n  verts 3\n    0 0 0\n    1 0 0\n\
    0 1 0\n  faces 1\n    0 1 2 1 0 1 2 1\nendnode\nendmodelgeom tst01_a01_01\n\
donemodel tst01_a01_01\n"
            );
            mg_mdl::Model::read(text.as_bytes()).unwrap()
        };
        assert!(TileFeatures::of(&model("Replace_Tex"), "tst01_a01_01").replace);
        let plain = TileFeatures::of(&model("tst01_grass"), "tst01_a01_01");
        assert!(!plain.replace);
        let with = TileFeatures { replace: true, ..plain };
        assert!(TileFeatures::union([plain, with]).replace, "any of them");
    }

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
    fn an_encounters_spawn_points_have_their_facings() {
        let game = game();
        let point = |x: f32, facing: f32| {
            let mut p = Struct::new(2);
            for (label, v) in [("X", x), ("Y", 2.0), ("Z", 0.5), ("Orientation", facing)] {
                p.set(label, Value::Float(v));
            }
            p
        };
        let mut e = object(&[("XPosition", 10.0), ("YPosition", 20.0)]);
        e.set("SpawnPointList", Value::List(vec![point(1.0, 0.0), point(3.0, 1.5)]));
        let o = AreaObject::read(&game, ObjectKind::Encounter, 0, &e);
        assert_eq!(o.spawn_points, [Vec3::new(1.0, 2.0, 0.5), Vec3::new(3.0, 2.0, 0.5)]);
        assert_eq!(o.spawn_facings, [0.0, 1.5]);
        // Only encounters have them.
        let w = AreaObject::read(&game, ObjectKind::Waypoint, 0, &e);
        assert!(w.spawn_points.is_empty() && w.spawn_facings.is_empty());
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

    /// The sun's and the moon's directions are the GIT's where it gives
    /// them (as the client lights the area), normalized; none where it
    /// doesn't or gives no direction.
    #[test]
    fn the_sun_and_moon_take_the_git_s_directions() {
        let mut props = Struct::new(100);
        for (axis, v) in [("X", -2.0f32), ("Y", 0.0), ("Z", 0.5)] {
            props.set(&format!("SunDirection{axis}"), Value::Float(v));
            props.set(&format!("MoonDirection{axis}"), Value::Float(0.0));
        }
        let mut git = Struct::new(0xFFFF_FFFF);
        git.set("AreaProperties", Value::Struct(props));
        let l = Lighting::read_with(&Struct::new(0), &git, None);
        let sun = l.direction(false).unwrap();
        assert!((sun - Vec3::new(-2.0, 0.0, 0.5).normalize()).length() < 1e-6);
        assert_eq!(l.direction(true), None, "a zero direction is none");
        let l = Lighting::read_with(&Struct::new(0), &Struct::new(0xFFFF_FFFF), None);
        assert_eq!((l.direction(false), l.direction(true)), (None, None));
    }

    #[test]
    fn tiles_take_their_replacement_texture() {
        let mut rm = mg_resman::ResMan::new();
        let mut c = mg_resman::MemContainer::new();
        c.insert(
            mg_resman::ResKey::parse("replacetexture", mg_core::ResType::TWODA).unwrap(),
            b"2DA V2.0\n\n   TEXTURENAME\n0  T_Flag02\n1  T_Flag03\n".to_vec(),
        );
        rm.add(mg_resman::priority::KEY, "game", mg_resman::LayerClass::Key, c);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut are = Struct::new(0);
        are.set("Width", Value::Int(2));
        are.set("Height", Value::Int(1));
        are.set("Width", Value::Int(5));
        // (As the client draws them: the value is one past the row; a tile
        // without the field takes row 0; 0 and a value past the table are
        // no row, which the client crashes on.)
        let tile = |v: Option<u8>| {
            let mut t = Struct::new(1);
            if let Some(v) = v {
                t.set("Tile_ReplaceTex", Value::Byte(v));
            }
            t
        };
        let values = [Some(2), Some(1), None, Some(0), Some(3)];
        are.set("Tile_List", Value::List(values.into_iter().map(tile).collect()));
        let area = AreaModel::read(&game, &are, &Struct::new(0xFFFF_FFFF), None);
        let drawn: Vec<Option<&str>> =
            area.tiles.iter().map(|t| t.replace_texture.as_deref()).collect();
        assert_eq!(drawn, [Some("t_flag03"), Some("t_flag02"), Some("t_flag02"), None, None]);
    }
}
