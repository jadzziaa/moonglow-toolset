//! A placeable's illumination: `LightState`, a byte the game keeps for
//! `GetPlaceableIllumination` and `SetPlaceableIllumination`, which Aurora
//! neither shows nor writes (Jasperre's report). Settled in the engine
//! (`engine_light_state.rs`): the server reads it from a placed
//! placeable's GIT entry and from a blueprint created by script; 0 is off
//! and any other value on; without the field the light is on. A placeable
//! whose appearance has no light (placeables.2da `LightColor`) is never
//! lit, whatever the field says.

use mg_gff::{Struct, Value};

use crate::GameData;

/// The field.
pub const LIGHT_STATE: &str = "LightState";

/// Whether the placeable's light is on when the game loads it (if its
/// appearance has one: [`has_light`]).
pub fn illuminated(utp: &Struct) -> bool {
    utp.integer(LIGHT_STATE).is_none_or(|v| v != 0)
}

/// Turns the light on or off: the field, as a byte (as the engine writes
/// it).
pub fn set_illuminated(utp: &mut Struct, on: bool) {
    utp.set(LIGHT_STATE, Value::Byte(u8::from(on)));
}

/// Whether placeables.2da row `appearance` has a light (a `LightColor`
/// row; the area view and the preview draw it).
pub fn has_light(game: &GameData, appearance: i64) -> bool {
    let Ok(row) = usize::try_from(appearance) else { return false };
    let Ok(table) = game.table("placeables") else { return false };
    let cell = table.get(row, "LightColor").map(str::trim).unwrap_or("");
    cell.parse::<f32>().is_ok_and(|c| c > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placeable_without_the_field_is_lit_and_keeps_none() {
        let mut s = Struct::new(0);
        s.set("Tag", Value::String(b"brazier".to_vec()));
        let before = s.clone();
        assert!(illuminated(&s));
        assert_eq!(s, before, "reading it adds nothing");
        set_illuminated(&mut s, false);
        assert_eq!(s.get(LIGHT_STATE), Some(&Value::Byte(0)));
        assert!(!illuminated(&s));
        set_illuminated(&mut s, true);
        assert_eq!(s.get(LIGHT_STATE), Some(&Value::Byte(1)));
        // Any value but 0 is on.
        s.set(LIGHT_STATE, Value::Byte(2));
        assert!(illuminated(&s));
    }
}
