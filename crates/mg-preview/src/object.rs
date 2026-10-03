//! Placeables and doors.

use glam::Vec3;
use mg_gff::Struct;
use mg_rules::GameData;

use crate::{Lookup, Part, Preview, PreviewError, PreviewLight, cell, cell_f32, cell_int, env_map};

/// A placeable blueprint (UTP fields): placeables.2da `ModelName`, its
/// `Reflection` environment map, its light (lightcolor.2da row at the light
/// offset, as `fx_placeable01`: radius 10) and the animation of its
/// `AnimationState`.
pub fn placeable(game: &GameData, utp: &Struct) -> Result<Preview, PreviewError> {
    let lk = Lookup { game };
    let table = game.table("placeables")?;
    let row = utp.integer("Appearance").unwrap_or(0);
    let r = usize::try_from(row).map_err(|_| PreviewError::NoRow { table: "placeables", row })?;
    let model = cell(&table, r, "ModelName")
        .map(str::to_ascii_lowercase)
        .ok_or(PreviewError::NoRow { table: "placeables", row })?;
    if !lk.has_model(&model) {
        return Err(PreviewError::NoModel(model));
    }
    let mut base = Part::new(model);
    base.env_map = env_map(cell(&table, r, "Reflection"));
    let mut lights = Vec::new();
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
            radius: 10.0,
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
/// `RESREF` (blue, red, green, yellow).
pub fn waypoint(game: &GameData, utw: &Struct) -> Result<Preview, PreviewError> {
    let lk = Lookup { game };
    let table = game.table("waypoint")?;
    let row = utw.integer("Appearance").unwrap_or(0);
    let model = usize::try_from(row)
        .ok()
        .and_then(|r| cell(&table, r, "RESREF"))
        .map(str::to_ascii_lowercase)
        .ok_or(PreviewError::NoRow { table: "waypoint", row })?;
    if !lk.has_model(&model) {
        return Err(PreviewError::NoModel(model));
    }
    Ok(Preview::model(&model))
}

/// A door blueprint (UTD fields): doortypes.2da `Model` for a tileset door
/// (`Appearance` ≠ 0), else genericdoors.2da `ModelName` of its
/// `GenericType_New` (or the older `GenericType`).
pub fn door(game: &GameData, utd: &Struct) -> Result<Preview, PreviewError> {
    let lk = Lookup { game };
    let appearance = utd.integer("Appearance").unwrap_or(0);
    let model = if appearance > 0 {
        let t = game.table("doortypes")?;
        cell(&t, appearance as usize, "Model")
            .ok_or(PreviewError::NoRow { table: "doortypes", row: appearance })?
            .to_ascii_lowercase()
    } else {
        let t = game.table("genericdoors")?;
        let row =
            ["GenericType_New", "GenericType"].iter().find_map(|f| utd.integer(f)).unwrap_or(0);
        cell(&t, row.max(0) as usize, "ModelName")
            .ok_or(PreviewError::NoRow { table: "genericdoors", row })?
            .to_ascii_lowercase()
    };
    if !lk.has_model(&model) {
        return Err(PreviewError::NoModel(model));
    }
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
