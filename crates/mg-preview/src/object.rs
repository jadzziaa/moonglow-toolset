//! Placeables and doors.

use glam::Vec3;
use mg_gff::Struct;
use mg_rules::{GameData, ObjectKind};

use crate::{
    Lookup, Part, Preview, PreviewError, PreviewLight, appearance, cell, cell_f32, cell_int,
    env_map,
};

/// The map a placeable whose appearance's `Reflection` is `default`
/// reflects: the game's own, whatever the area's ([`placeable`]).
pub const PLACEABLE_DEFAULT_ENV_MAP: &str = "chrome1";

/// A placeable blueprint (UTP fields): placeables.2da `ModelName`, its
/// `Reflection` environment map unless it is static, its light
/// (lightcolor.2da row at the light offset, as `fx_placeable01`: radius
/// 10) unless **Illuminated** is off (`LightState` 0; absent is on) and
/// the animation of its `AnimationState`. (Illuminated off leaves the
/// model's own light nodes on, as the client does: `model_light_uniforms`
/// in `client_render.rs`.)
///
/// A static placeable's meshes don't reflect its appearance's map in the
/// client, `default` or named: its textures' alpha is their opacity
/// there, as for any placeable without one; a texture's own map (TXI
/// `envmaptexture`) it keeps (`area_env_map_look` in `client_render.rs`).
///
/// An appearance's `Reflection` of `default` is the game's own map,
/// `chrome1`, wherever the placeable stands: not the area's (its
/// tileset's `EnvMap`), which a texture's `envmaptexture default` and a
/// creature's appearance.2da `ENVMAP` `default` reflect. Read from the
/// client: in a `tin01` interior, whose map is a copper brown, a sphere
/// of each drawn with the map bound to it (`area_env_map_look` with
/// `MG_ENV_PAINT`, `MG_ENV_SHAPE=sphere` and `MG_ENV_CREATURE`).
pub fn placeable(game: &GameData, utp: &Struct) -> Result<Preview, PreviewError> {
    let table = game.table("placeables")?;
    let (r, model) = appearance(game, ObjectKind::Placeable, utp)?;
    let mut base = Part::new(model.unwrap_or_default());
    let is_static = utp.integer("Static").is_some_and(|v| v != 0);
    if !is_static {
        base.env_map = env_map(cell(&table, r, "Reflection"))
            .map(|e| if e == "default" { PLACEABLE_DEFAULT_ENV_MAP.to_string() } else { e });
    }
    let mut lights = Vec::new();
    let illuminated = utp.integer("LightState").is_none_or(|v| v != 0);
    if let Some(c) = cell_int(&table, r, "LightColor").filter(|&c| c > 0) {
        let colors = game.table("lightcolor")?;
        let ch = |col| cell_f32(&colors, c as usize, col).unwrap_or(0.0);
        let offset = |col| cell_f32(&table, r, col).unwrap_or(0.0);
        lights.push(PreviewLight {
            offset: Vec3::new(
                offset("LightOffsetX"),
                offset("LightOffsetY"),
                offset("LightOffsetZ"),
            ),
            color: Vec3::new(ch("RED"), ch("GREEN"), ch("BLUE")),
            // (Measured in the client: `placeable_light_uniforms`.)
            radius: 10.0,
            on: illuminated,
        });
    }
    let idle = match utp.integer("AnimationState").unwrap_or(0) {
        1 => "open",
        2 => "close",
        3 => "dead",
        4 => "on",
        5 => "off",
        _ => "default",
    };
    Ok(Preview { base, parts: Vec::new(), idle: Some(idle.into()), lights })
}

/// A waypoint (UTW fields): the flag of its `Appearance`, waypoint.2da
/// `RESREF` (blue, red, green, yellow), by [`mg_rules::appearance`] as
/// Verify judges it.
pub fn waypoint(game: &GameData, utw: &Struct) -> Result<Preview, PreviewError> {
    let (_, model) = crate::appearance(game, mg_rules::ObjectKind::Waypoint, utw)?;
    let model = model.ok_or_else(|| PreviewError::NoModel("waypoint".into()))?;
    Ok(Preview::model(&model))
}

/// A store's marker, as Aurora shows one: the game's `gi_store` model
/// (a store has no appearance).
pub fn store(game: &GameData) -> Result<Preview, PreviewError> {
    const MARKER: &str = "gi_store";
    if !(Lookup { game }).has_model(MARKER) {
        return Err(PreviewError::NoModel(MARKER.into()));
    }
    Ok(Preview::model(MARKER))
}

/// A sound's marker, as Aurora shows one (UTS fields): the game's
/// `gi_sound_area` for a sound heard everywhere in the area, `gi_sound_rndm`
/// for one played from a random position, else `gi_sound_pos`.
pub fn sound(game: &GameData, uts: &Struct) -> Result<Preview, PreviewError> {
    let on = |label: &str| uts.integer(label).unwrap_or(0) != 0;
    let marker = match (on("Positional"), on("RandomPosition")) {
        (false, _) => "gi_sound_area",
        (true, true) => "gi_sound_rndm",
        (true, false) => "gi_sound_pos",
    };
    if !(Lookup { game }).has_model(marker) {
        return Err(PreviewError::NoModel(marker.into()));
    }
    Ok(Preview::model(marker))
}

/// A door blueprint (UTD fields): doortypes.2da `Model` for a tileset door
/// (`Appearance` ≠ 0), else genericdoors.2da `ModelName` of its
/// `GenericType_New` (or the older `GenericType`).
pub fn door(game: &GameData, utd: &Struct) -> Result<Preview, PreviewError> {
    let (_, model) = appearance(game, ObjectKind::Door, utd)?;
    let model = model.unwrap_or_default();
    // A placed door's initial state (`AnimationState`).
    let idle = match utd.integer("AnimationState").unwrap_or(0) {
        1 => "opened1",
        2 => "opened2",
        _ => "closed",
    };
    Ok(Preview {
        base: Part::new(model),
        parts: Vec::new(),
        idle: Some(idle.into()),
        lights: Vec::new(),
    })
}
