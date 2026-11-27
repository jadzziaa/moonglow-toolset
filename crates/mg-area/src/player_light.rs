//! The light the game's player carries in a dim area, as the client was
//! read to give it (`docs/findings.md`, "The player's own light in a dim
//! area"): the area view's Player's Light gives it to the module's start
//! location, so that a dim area looks as it does in the game.
//!
//! Read with the player of `+TestNewModule` (the first pregenerated
//! character, an elf), who carries `fx_light_clr`'s `Low_Light_Vision`
//! (progfx.2da row 330): ambient only (it lights every face, whatever its
//! facing), color 0.3 0.3 0.4, radius 11 (`lightColor` read in the client
//! as lin 0.0697 0.0697 0.133, `.a` −484: it reaches 22 m), 0.64 m ahead
//! of the player along its facing and 2.65 m up, turning with it. Another
//! race's vision light (darkvision, a human's default) is not read.
//! (`chrome_reflection`'s `MG_CHROME_PAINT=lights`, `light0`, `light0a`
//! and `light0world`, 2026-11-26, client 89.8193.37.)

use glam::Vec3;

use crate::Sky;

/// The light's color (gamma space, as `fx_light_clr`'s model gives it).
pub const COLOR: Vec3 = Vec3::new(0.3, 0.3, 0.4);

/// The light's radius, metres (it reaches twice as far:
/// [`mg_render::CUTOFF_RANGE_MULTIPLIER`]).
pub const RADIUS: f32 = 11.0;

/// How far ahead of the player the light hangs, along its facing, and how
/// far up, metres.
pub const AHEAD: f32 = 0.64;
pub const UP: f32 = 2.65;

/// The gray level (0 to 255) the area's ambient and diffuse add up to from
/// which the light is out.
pub const DIM_BELOW: u32 = 0x80;

/// Whether an area lit by `ambient` and `diffuse` (ARE colors, 0xBBGGRR)
/// is dim enough for the player's light: their levels add up to less
/// than [`DIM_BELOW`].
///
/// Read with gray light only (ambient 0x7C alone and 0x60 with diffuse
/// 0x1C lit, 0x80 alone and 0x60 with 0x20 not, 0x08 with 0x74 lit and
/// with 0x78 not). For colored light the rule is not read: Moonglow takes
/// the mean of the three channels' sums (a deep blue night, dark to the
/// eye, is dim though its blue alone adds up to more than 128). It is the
/// read rule for gray light, whichever order the channels are in.
pub fn dim(ambient: u32, diffuse: u32) -> bool {
    let channels = |c: u32| c.to_le_bytes()[..3].iter().map(|&b| u32::from(b)).sum::<u32>();
    // (The mean below DIM_BELOW: in whole numbers, the sum below three
    // times it.)
    channels(ambient) + channels(diffuse) < 3 * DIM_BELOW
}

/// The player's light for a player at `at` (on the ground) facing
/// `facing` (radians, counter-clockwise from east), whatever the area's
/// light.
pub fn light(at: Vec3, facing: f32) -> mg_render::PointLight {
    let ahead = Vec3::new(facing.cos(), facing.sin(), 0.0);
    // (Priority 3: a placeable's light's, kept past 32 lights before the
    // tiles' least.)
    mg_render::PointLight::new(at + ahead * AHEAD + Vec3::Z * UP, COLOR, RADIUS, true, 3)
}

/// The player's light for a player at `at` facing `facing` (radians) in
/// an area lit by `sky` (its sun's or moon's, as shown): none where the
/// area is not [`dim`].
pub fn of(at: Vec3, facing: f32, sky: &Sky) -> Option<mg_render::PointLight> {
    dim(sky.ambient, sky.diffuse).then(|| light(at, facing))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pairs read in the client (ambient, diffuse, lit), gray.
    const READ: [(u32, u32, bool); 6] = [
        (0x7C7C7C, 0, true),
        (0x808080, 0, false),
        (0x606060, 0x1C1C1C, true),
        (0x606060, 0x202020, false),
        (0x080808, 0x747474, true),
        (0x080808, 0x787878, false),
    ];

    #[test]
    fn the_read_gray_pairs_are_kept() {
        for (ambient, diffuse, lit) in READ {
            assert_eq!(dim(ambient, diffuse), lit, "{ambient:06X} + {diffuse:06X}");
        }
        // (Out from 128 exactly; the dark scene's ambient is lit.)
        assert!(dim(0x7F7F7F, 0));
        assert!(!dim(0x404040, 0x404040));
        assert!(dim(0x080808, 0));
    }

    #[test]
    fn colored_light_goes_by_the_mean_of_its_channels() {
        // A deep blue night: blue alone adds up to 0xA0, the mean to 53.
        assert!(dim(0x600000, 0x400000));
        // Whichever channel holds it.
        assert!(dim(0x000060, 0x000040));
        assert!(dim(0x006000, 0x004000));
        // Bright in one channel, enough for the mean: out.
        assert!(!dim(0xFF0000, 0x00FF00));
        // The mean just under and at 128 (sums 383 and 384).
        assert!(dim(0x7F807F, 0));
        assert!(!dim(0x808080, 0));
        // (The byte above the color is no channel.)
        assert!(dim(0xFF00_0000, 0));
    }

    #[test]
    fn the_light_hangs_ahead_and_turns_with_the_player() {
        let at = Vec3::new(10.0, 20.0, 1.0);
        let east = light(at, 0.0);
        assert!(east.position.abs_diff_eq(Vec3::new(10.64, 20.0, 3.65), 1e-5));
        let north = light(at, std::f32::consts::FRAC_PI_2);
        assert!(north.position.abs_diff_eq(Vec3::new(10.0, 20.64, 3.65), 1e-5));
        // Ambient only, as the client uploads it, reaching 22 m.
        assert!(east.ambient_only);
        assert!(!east.casts_shadows);
        assert!((east.cutoff - 22.0).abs() < 1e-5);
        assert!(east.color.abs_diff_eq(Vec3::new(0.0707, 0.0707, 0.1330), 2e-3));
    }

    #[test]
    fn the_light_is_out_in_a_bright_sky() {
        let at = Vec3::ZERO;
        let bright = Sky { ambient: 0x606060, diffuse: 0xC0C0C0, ..Sky::default() };
        let dark = Sky { ambient: 0x080808, diffuse: 0, ..Sky::default() };
        assert!(of(at, 0.0, &bright).is_none());
        assert!(of(at, 0.0, &dark).is_some());
    }
}
