//! The Domains of Dread's color tint (its `docs/design/colorways.md`,
//! tried in the game client on 2026-11-06): a material is tintable when
//! its MTR names the module's tint shader and declares a `tdodTint`
//! parameter (sRGB red, green and blue, then a strength: 0 is the material
//! as made). The module's scripts set it from locals:
//!
//! - on a placed placeable, `tdod_tint` (`"#rrggbb"`) for every tintable
//!   material of the object, and `tdod_tint_<mtr resref>` for that one
//!   material, which wins over `tdod_tint`
//!   (`SetMaterialShaderUniformVec4`). Only a dynamic placeable takes it:
//!   the client bakes a static one into the tiles as made;
//! - on an area (its GIT, where the game reads an area's locals),
//!   `tdod_wallpaper`, for the wallpaper materials' `scriptableVec1`
//!   (`SetShaderUniformVec` with `SHADER_UNIFORM_1`, per player): the
//!   color and strength 1; without it, strength 0.
//!
//! The one reading of those locals: the area view and the previews draw
//! them, Verify notes a tint on a static placeable, and the properties
//! offer a color picker for them.

use mg_gff::Struct;

/// The local that tints every tintable material of an object.
pub const TINT: &str = "tdod_tint";
/// The start of a local that tints one material: the MTR's resref follows.
pub const TINT_PREFIX: &str = "tdod_tint_";
/// The area's local for its wallpaper.
pub const WALLPAPER: &str = "tdod_wallpaper";
/// The MTR parameter a tintable material declares.
pub const PARAMETER: &str = "tdodTint";
/// The scriptable uniform the wallpaper is set in: `scriptableVec1`
/// (`SHADER_UNIFORM_1` is 0).
pub const WALLPAPER_UNIFORM: usize = 0;

/// An object's tints, as its locals give them: sRGB, 0 to 1.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tints {
    /// `tdod_tint`: every tintable material.
    pub all: Option<[f32; 3]>,
    /// `tdod_tint_<mtr>`: one material each, by MTR resref (lower case).
    pub by_material: Vec<(String, [f32; 3])>,
}

impl Tints {
    pub fn is_empty(&self) -> bool {
        self.all.is_none() && self.by_material.is_empty()
    }

    /// The tint of the material `mtr` (a resref, any case): its own, else
    /// the object's.
    pub fn of(&self, mtr: &str) -> Option<[f32; 3]> {
        self.by_material
            .iter()
            .find(|(m, _)| m.eq_ignore_ascii_case(mtr))
            .map(|(_, c)| *c)
            .or(self.all)
    }
}

/// `"#rrggbb"` as sRGB, 0 to 1; anything else (another length, no `#`,
/// a letter past `f`) is no color, as the module's script reads none.
pub fn parse(text: &str) -> Option<[f32; 3]> {
    parse_bytes(text).map(|c| c.map(|v| f32::from(v) / 255.0))
}

/// `"#rrggbb"` as its three bytes.
pub fn parse_bytes(text: &str) -> Option<[u8; 3]> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// A color as the locals hold it: `"#rrggbb"`, lower case.
pub fn format(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// Whether a local of this name holds a tint (an object's or an area's).
pub fn is_tint_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == TINT
        || name == WALLPAPER
        || name.strip_prefix(TINT_PREFIX).is_some_and(|m| !m.is_empty())
}

/// The string locals of `s`'s `VarTable`, by name.
fn strings(s: &Struct) -> impl Iterator<Item = (String, String)> + '_ {
    s.list("VarTable").unwrap_or(&[]).iter().filter_map(|v| {
        let name = String::from_utf8_lossy(v.string("Name")?).into_owned();
        let value = String::from_utf8_lossy(v.string("Value")?).into_owned();
        Some((name, value))
    })
}

/// The tints an object's locals give (whether or not it takes them: a
/// static placeable doesn't, [`takes_tint`]).
pub fn of_object(object: &Struct) -> Tints {
    let mut out = Tints::default();
    for (name, value) in strings(object) {
        let name = name.to_ascii_lowercase();
        let Some(color) = parse(&value) else { continue };
        if name == TINT {
            out.all = Some(color);
        } else if let Some(mtr) = name.strip_prefix(TINT_PREFIX).filter(|m| !m.is_empty())
            && !out.by_material.iter().any(|(m, _)| m == mtr)
        {
            out.by_material.push((mtr.to_string(), color));
        }
    }
    out
}

/// Whether a placeable takes a tint in the game: a dynamic one (the
/// client bakes a static one into the tiles, as made).
pub fn takes_tint(utp: &Struct) -> bool {
    utp.integer("Static").is_none_or(|v| v == 0)
}

/// An area's wallpaper color, from its GIT's root: `tdod_wallpaper`.
pub fn wallpaper(git: &Struct) -> Option<[f32; 3]> {
    strings(git).find(|(n, _)| n.eq_ignore_ascii_case(WALLPAPER)).and_then(|(_, v)| parse(&v))
}

/// The uniform's value for a tint: the color, strength 1.
pub fn uniform(color: [f32; 3]) -> [f32; 4] {
    [color[0], color[1], color[2], 1.0]
}

#[cfg(test)]
mod tests {
    use mg_gff::Value;

    use super::*;

    fn vars(vars: &[(&str, &str)]) -> Struct {
        let mut s = Struct::default();
        let list = vars
            .iter()
            .map(|(n, v)| {
                let mut e = Struct::default();
                e.set("Name", Value::String(n.as_bytes().to_vec()));
                e.set("Type", Value::Dword(3));
                e.set("Value", Value::String(v.as_bytes().to_vec()));
                e
            })
            .collect();
        s.set("VarTable", Value::List(list));
        s
    }

    #[test]
    fn a_color_is_a_hash_and_six_hex_digits() {
        assert_eq!(parse_bytes("#2a4d8f"), Some([0x2a, 0x4d, 0x8f]));
        assert_eq!(parse_bytes(" #FFFFFF "), Some([255; 3]));
        for bad in ["2a4d8f", "#2a4d8", "#2a4d8f0", "#2g4d8f", "", "#", "#ÿÿÿ"] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
        assert_eq!(format([0x2a, 0x4d, 0x8f]), "#2a4d8f");
        assert_eq!(parse("#ff0000"), Some([1.0, 0.0, 0.0]));
    }

    #[test]
    fn a_material_s_own_tint_wins_over_the_object_s() {
        let t = of_object(&vars(&[
            ("tdod_tint", "#ff0000"),
            ("TDOD_TINT_dd_ctn_bedb", "#00ff00"),
            ("tdod_tint_mats", "dd_ctn_bedb,dd_ctn_bedc"),
            ("tdod_tint_broken", "green"),
            ("other", "#0000ff"),
        ]));
        assert_eq!(t.all, Some([1.0, 0.0, 0.0]));
        assert_eq!(t.by_material, [("dd_ctn_bedb".to_string(), [0.0, 1.0, 0.0])]);
        assert_eq!(t.of("DD_CTN_BEDB"), Some([0.0, 1.0, 0.0]));
        assert_eq!(t.of("dd_ctn_bedc"), Some([1.0, 0.0, 0.0]));
        assert!(of_object(&Struct::default()).is_empty());
    }

    #[test]
    fn a_static_placeable_takes_no_tint() {
        let mut p = vars(&[("tdod_tint", "#ff0000")]);
        assert!(takes_tint(&p));
        p.set("Static", Value::Byte(1));
        assert!(!takes_tint(&p));
    }

    #[test]
    fn an_area_s_wallpaper_is_its_git_local() {
        assert_eq!(wallpaper(&vars(&[("tdod_wallpaper", "#7a2e3a")])), parse("#7a2e3a"));
        assert_eq!(wallpaper(&vars(&[("tdod_wallpaper", "oxblood")])), None);
        assert!(is_tint_name("tdod_wallpaper") && is_tint_name("TDOD_TINT"));
        assert!(is_tint_name("tdod_tint_dd_gca_carpeta") && !is_tint_name("tdod_tint_"));
    }
}
