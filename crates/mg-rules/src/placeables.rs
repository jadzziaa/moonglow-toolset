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
    light_color(&table, row).is_some()
}

/// The lightcolor.2da row of the light placeables.2da row `row` has
/// (`LightColor`, a number above 0, as the game reads a cell): `None` for
/// none. The one reading of it: [`has_light`] and the light the preview
/// draws.
pub fn light_color(placeables: &mg_2da::TwoDa, row: usize) -> Option<usize> {
    let color = placeables.get_int(row, "LightColor").filter(|c| *c > 0)?;
    usize::try_from(color).ok()
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

    /// A placeable's light is its `LightColor` as the game reads a cell,
    /// by one reading: `3x` is row 3 for the Properties and for the
    /// light drawn alike (it was a light for one and none for the other),
    /// and an armor is a `ModelType` of 3 read the same way.
    #[test]
    fn a_light_and_an_armor_are_read_as_the_game_reads_a_cell() {
        let game = crate::test_game(&[
            (
                "placeables",
                "Label LightColor\n0 Chair ****\n1 Lamp 3\n2 Odd 3x\n3 Off 0\n4 Low -1\n",
            ),
            ("baseitems", "label ModelType\n0 sword 2\n1 armor 3\n2 odd 3x\n3 none ****\n"),
        ]);
        let table = game.table("placeables").unwrap();
        let colors: Vec<Option<usize>> = (0..6).map(|row| light_color(&table, row)).collect();
        assert_eq!(colors, [None, Some(3), Some(3), None, None, None]);
        for row in 0..6 {
            assert_eq!(has_light(&game, row), light_color(&table, row as usize).is_some());
        }
        assert!(!has_light(&game, -1));
        let armor: Vec<bool> = (0..5).map(|row| game.is_armor(row)).collect();
        assert_eq!(armor, [false, true, true, false, false]);
        let items = game.table("baseitems").unwrap();
        assert!(crate::items::is_armor_row(&items, 1) && !crate::items::is_armor_row(&items, 0));
    }
}
