//! Lighting kept under a name (dansen9191's ask): what an area's Visual
//! page sets (its lighting scheme, the sun's and the moon's colors, fog
//! and shadows, day and night, the sky and the weather) and the colors
//! its tiles' lights are picked from, saved from one area and given to
//! any other, in any module. Kept in Moonglow's data folder (`lighting`)
//! as JSON, field by field:
//!
//! ```json
//! { "lighting": { "SunAmbientColor": 4210752, "SunFogAmount": 10, "FogClipDist": 45.0 },
//!   "tile_lights": { "main1": [27, 27, 4, 0], "main2": [0, 0, 0, 0], "source": [2, 2, 2, 0] } }
//! ```
//!
//! The tiles' lights are kept as a lighting scheme (an environment.2da
//! row) has them: four colors (lightcolor.2da rows) for each of a tile's
//! main lights and its source lights, and an area given the lighting has
//! each tile's lights picked from them at random, as choosing a scheme
//! picks them. The four are the area's own, the most used first, each as
//! often as the area's tiles have it (near enough, in quarters). A file
//! saved before they were kept has none, and leaves an area's tiles as
//! they are.
//!
//! Kept lighting becomes a lighting scheme of the module's own as an
//! environment.2da row ([`Saved::environment_row`]), which the Visual
//! page lists and the game's `SetAreaLightingScheme` takes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mg_core::{Codepage, ResType};
use mg_gff::{FieldType, Struct, Value};
use serde_json::{Map, json};

/// The Visual page's fields: the lighting scheme, the sun's and the
/// moon's colors, fog and shadows, day and night, the sky and the weather.
/// What Give Every Area This Page's Settings, Copy From Area and saved
/// lighting give an area.
pub const VISUAL_FIELDS: &[&str] = &[
    "ChanceLightning",
    "ChanceRain",
    "ChanceSnow",
    "DayNightCycle",
    "IsNight",
    "LightingScheme",
    "MoonAmbientColor",
    "MoonDiffuseColor",
    "MoonFogAmount",
    "MoonFogColor",
    "MoonShadows",
    "SunAmbientColor",
    "SunDiffuseColor",
    "SunFogAmount",
    "SunFogColor",
    "SunShadows",
    "ShadowOpacity",
    "SkyBox",
    "WindPower",
    "FogClipDist",
];

/// The folder of Moonglow's data folder lighting is kept in.
pub const FOLDER: &str = "lighting";
/// A kept lighting's file is its name and this.
pub const SUFFIX: &str = ".lighting.json";

/// Where lighting is kept: Moonglow's data folder's `lighting`.
pub fn dir() -> Option<PathBuf> {
    crate::build::data_dir().map(|d| d.join(FOLDER))
}

/// The file the lighting named `name` is kept in, in `dir`.
pub fn file(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}{SUFFIX}", name.trim()))
}

/// The colors a tile's lights are picked from: four lightcolor.2da rows
/// each for main light 1, main light 2 and the source lights, as an
/// environment.2da row's `MAIN1_COLOR1`… `SECONDARY_COLOR4` have them.
pub type TileColors = [[u8; 4]; 3];

/// The names [`TileColors`] are kept under, in its order.
const TILE_KEYS: [&str; 3] = ["main1", "main2", "source"];

/// A tile's light fields, as a scheme sets them.
pub const TILE_LIGHT_FIELDS: [&str; 4] =
    ["Tile_MainLight1", "Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2"];

/// The colors an area's tiles' lights are picked from, as a scheme would
/// pick them to give those tiles theirs: for each light, the four colors
/// its tiles have most, each in as many of the four places as its share
/// of the tiles (a color of half the tiles in two), every one of the four
/// in one at least. `None` for an area without tiles.
pub fn tile_colors(tiles: &[Struct]) -> Option<TileColors> {
    if tiles.is_empty() {
        return None;
    }
    let of = |label: &str| {
        let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
        for t in tiles {
            let c = t.integer(label).unwrap_or(0).clamp(0, 255) as u8;
            *counts.entry(c).or_default() += 1;
        }
        four_of(counts)
    };
    Some([of(TILE_LIGHT_FIELDS[0]), of(TILE_LIGHT_FIELDS[1]), of(TILE_LIGHT_FIELDS[2])])
}

/// Four places shared out among colors by how often each is used: the
/// four used most, the most used first (the lower color of two used as
/// often), each in one place at least and in more by its share.
fn four_of(counts: BTreeMap<u8, usize>) -> [u8; 4] {
    let mut by_use: Vec<(u8, usize)> = counts.into_iter().collect();
    by_use.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    by_use.truncate(4);
    let total: usize = by_use.iter().map(|c| c.1).sum();
    let mut places: Vec<usize> = by_use.iter().map(|c| (4 * c.1 / total).max(1)).collect();
    // Too many (each took one at least): the one most over its share
    // gives one back; too few: the one most under it takes one.
    let share =
        |i: usize, places: &[usize]| by_use[i].1 as f64 / total as f64 * 4.0 - places[i] as f64;
    while places.iter().sum::<usize>() > 4 {
        let i = (0..places.len())
            .filter(|&i| places[i] > 1)
            .min_by(|&a, &b| share(a, &places).total_cmp(&share(b, &places)))
            .expect("one with more than one place");
        places[i] -= 1;
    }
    while places.iter().sum::<usize>() < 4 {
        let i = (0..places.len())
            .max_by(|&a, &b| share(a, &places).total_cmp(&share(b, &places)).then(b.cmp(&a)))
            .expect("a color");
        places[i] += 1;
    }
    let mut out = Vec::with_capacity(4);
    for (i, (color, _)) in by_use.iter().enumerate() {
        out.extend(std::iter::repeat_n(*color, places[i]));
    }
    out.try_into().expect("four places")
}

/// What a tile's lights become, picked from `colors` at random: its
/// [`TILE_LIGHT_FIELDS`], main lights 1 and 2 and the source light (both
/// source fields alike, as the Area Wizard writes them).
pub fn pick_tile_fields(
    colors: &TileColors,
    rng: &mut fastrand::Rng,
) -> [(&'static str, Value); 4] {
    let [main1, main2, source] = colors.map(|four| four[rng.u32(1..=4) as usize - 1]);
    let [a, b, c, d] = TILE_LIGHT_FIELDS;
    [(a, main1), (b, main2), (c, source), (d, source)].map(|(l, v)| (l, Value::Byte(v)))
}

/// Lighting kept under a name: the Visual page's fields an area had, each
/// a number, and the colors its tiles' lights were picked from.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    fields: Map<String, serde_json::Value>,
    tiles: Option<TileColors>,
}

impl Saved {
    /// An area's lighting (its root, an ARE's): the Visual page's fields
    /// it has, and its tiles' colors ([`tile_colors`]).
    pub fn of_area(are: &Struct) -> Saved {
        let mut fields = Map::new();
        for label in VISUAL_FIELDS {
            let n = match are.get(label) {
                Some(Value::Float(v)) => json!(v),
                Some(Value::Double(v)) => json!(v),
                Some(v) => match v.as_integer() {
                    Some(n) => json!(n),
                    None => continue,
                },
                None => continue,
            };
            fields.insert((*label).to_string(), n);
        }
        Saved { fields, tiles: tile_colors(are.list("Tile_List").unwrap_or(&[])) }
    }

    /// As a file's text.
    pub fn to_text(&self) -> String {
        let mut out = Map::new();
        out.insert("lighting".into(), serde_json::Value::Object(self.fields.clone()));
        if let Some(tiles) = &self.tiles {
            let lights: Map<String, serde_json::Value> = TILE_KEYS
                .iter()
                .zip(tiles)
                .map(|(k, four)| ((*k).to_string(), json!(four)))
                .collect();
            out.insert("tile_lights".into(), serde_json::Value::Object(lights));
        }
        serde_json::to_string_pretty(&serde_json::Value::Object(out)).expect("JSON") + "\n"
    }

    /// A file's text: its fields (the Visual page's; others are left out)
    /// and its tiles' colors, which a file saved before they were kept
    /// hasn't.
    pub fn parse(text: &str) -> Result<Saved, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let all = v.get("lighting").and_then(|l| l.as_object()).ok_or("no \"lighting\" fields")?;
        let mut fields = Map::new();
        for label in VISUAL_FIELDS {
            if let Some(n) = all.get(*label) {
                if !n.is_number() {
                    return Err(format!("{label}: {n} is not a number"));
                }
                fields.insert((*label).to_string(), n.clone());
            }
        }
        let tiles = match v.get("tile_lights") {
            None => None,
            Some(t) => {
                let four = |key: &str| -> Result<[u8; 4], String> {
                    let list = t.get(key).and_then(|l| l.as_array());
                    let colors: Option<Vec<u8>> = list.map(|l| {
                        l.iter()
                            .filter_map(|c| c.as_u64().and_then(|c| u8::try_from(c).ok()))
                            .collect()
                    });
                    match colors.filter(|c| c.len() == 4 && list.is_some_and(|l| l.len() == 4)) {
                        Some(c) => Ok([c[0], c[1], c[2], c[3]]),
                        None => Err(format!("tile_lights: {key} is not four colors (0 to 255)")),
                    }
                };
                Some([four(TILE_KEYS[0])?, four(TILE_KEYS[1])?, four(TILE_KEYS[2])?])
            }
        };
        if fields.is_empty() && tiles.is_none() {
            return Err("no lighting fields".into());
        }
        Ok(Saved { fields, tiles })
    }

    /// The colors its tiles' lights are picked from, where it has them.
    pub fn tiles(&self) -> Option<&TileColors> {
        self.tiles.as_ref()
    }

    /// Its fields as those of an area whose root is `are`: each of the
    /// type it has there (the game's, for one it lacks).
    pub fn fields_for(&self, are: &Struct) -> Result<Vec<(&'static str, Value)>, String> {
        let mut out = Vec::new();
        for label in VISUAL_FIELDS {
            let Some(n) = self.fields.get(*label) else { continue };
            let ty = are
                .get(label)
                .map(Value::field_type)
                .or_else(|| mg_schema::root_field_type(ResType::ARE, label))
                .unwrap_or(FieldType::Int);
            let value = Value::from_text(ty, &n.to_string(), None, Codepage::default())
                .map_err(|e| format!("{label}: {e}"))?;
            out.push((*label, value));
        }
        Ok(out)
    }

    /// Gives it to an area (its root, an ARE's): its fields, and with
    /// `relight` each tile's lights picked anew from its colors (where it
    /// has them; without, the tiles' lights are left as they are).
    /// Whether the fields changed, and how many tiles' lights did.
    pub fn give(
        &self,
        are: &mut Struct,
        rng: &mut fastrand::Rng,
        relight: bool,
    ) -> Result<(bool, usize), String> {
        let mut fields_changed = false;
        for (label, value) in self.fields_for(are)? {
            if are.get(label) != Some(&value) {
                are.set(label, value);
                fields_changed = true;
            }
        }
        let mut lit = 0;
        if let Some(colors) = self.tiles.as_ref().filter(|_| relight) {
            for tile in are.list_mut("Tile_List").into_iter().flatten() {
                let mut changed = false;
                for (label, value) in pick_tile_fields(colors, rng) {
                    changed |= tile.get(label) != Some(&value);
                    tile.set(label, value);
                }
                lit += usize::from(changed);
            }
        }
        Ok((fields_changed, lit))
    }

    fn int(&self, label: &str) -> i64 {
        let n = self.fields.get(label);
        n.and_then(|n| n.as_i64().or_else(|| n.as_f64().map(|f| f as i64))).unwrap_or(0)
    }

    /// As an environment.2da row, a lighting scheme of the module's own
    /// (`LABEL` `label`, no talk-table name: the Visual page lists it by
    /// its label): every column [`crate::new::Scheme::read`] reads, so the
    /// scheme gives an area what the lighting does, but its sky box and its
    /// fog's distance (a scheme has neither: choosing one sets them back
    /// to none and 45 meters). Tiles' colors it lacks are 0 (no light).
    pub fn environment_row(&self, label: &str) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        let mut put = |column: &str, value: String| out.push((column.to_string(), value));
        put("LABEL", label.to_string());
        put("STRREF", "****".into());
        let (cycle, night) = (self.int("DayNightCycle") != 0, self.int("IsNight") != 0);
        let day_night = match (cycle, night) {
            (true, _) => "cycle",
            (false, true) => "night",
            (false, false) => "day",
        };
        put("DAYNIGHT", day_night.into());
        for (prefix, sun) in [("LIGHT", "Sun"), ("DARK", "Moon")] {
            for (part, field) in [("AMB", "AmbientColor"), ("DIFF", "DiffuseColor")] {
                let c = self.int(&format!("{sun}{field}")) as u32;
                for (i, color) in ["RED", "GREEN", "BLUE"].iter().enumerate() {
                    put(&format!("{prefix}_{part}_{color}"), ((c >> (8 * i)) & 0xff).to_string());
                }
            }
            put(&format!("{prefix}_SHADOWS"), self.int(&format!("{sun}Shadows")).to_string());
            let c = self.int(&format!("{sun}FogColor")) as u32;
            for (i, color) in ["RED", "GREEN", "BLUE"].iter().enumerate() {
                put(&format!("{prefix}_FOG_{color}"), ((c >> (8 * i)) & 0xff).to_string());
            }
            put(&format!("{prefix}_FOG"), self.int(&format!("{sun}FogAmount")).to_string());
        }
        let tiles = self.tiles.unwrap_or_default();
        for (prefix, four) in ["MAIN1_COLOR", "MAIN2_COLOR", "SECONDARY_COLOR"].iter().zip(tiles) {
            for (n, c) in four.iter().enumerate() {
                put(&format!("{prefix}{}", n + 1), c.to_string());
            }
        }
        put("WIND", self.int("WindPower").to_string());
        put("SNOW", self.int("ChanceSnow").to_string());
        put("RAIN", self.int("ChanceRain").to_string());
        put("LIGHTNING", self.int("ChanceLightning").to_string());
        let alpha = self.int("ShadowOpacity") as f64 / 100.0;
        put("SHADOW_ALPHA", format!("{alpha:.2}"));
        out
    }
}

/// A label for an environment.2da row from a kept lighting's name: its
/// letters, digits and underscores (a space an underscore), as the game's
/// own labels are written (`ExteriorClear`).
pub fn row_label(name: &str) -> String {
    let label: String = (name.trim().chars())
        .filter_map(|c| match c {
            c if c.is_ascii_alphanumeric() || c == '_' => Some(c),
            ' ' | '-' | '.' => Some('_'),
            _ => None,
        })
        .collect();
    if label.is_empty() { "Lighting".to_string() } else { label }
}

/// The lighting kept in `dir` as `name`, or in the file `name` names (a
/// path, ending in `.json`).
pub fn read(dir: Option<&Path>, name: &str) -> Result<Saved, String> {
    let path = if name.ends_with(".json") || name.contains(['/', '\\']) {
        PathBuf::from(name)
    } else {
        let dir = dir.ok_or("Moonglow has no data folder")?;
        file(dir, name)
    };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Saved::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(main1: u8, main2: u8, source: u8) -> Struct {
        let mut t = Struct::new(1);
        t.set("Tile_MainLight1", Value::Byte(main1));
        t.set("Tile_MainLight2", Value::Byte(main2));
        t.set("Tile_SrcLight1", Value::Byte(source));
        t.set("Tile_SrcLight2", Value::Byte(source));
        t
    }

    /// Saved from one area and read as another's fields: each of the type
    /// the area has it as (an older area's byte, a dword), the page's
    /// fields only, and the tiles' colors with them.
    #[test]
    fn lighting_reads_back_as_an_area_s_fields() {
        let mut from = Struct::new(0);
        from.set("SunAmbientColor", Value::Dword(0x40_4040));
        from.set("SunFogAmount", Value::Byte(10));
        from.set("FogClipDist", Value::Float(45.5));
        from.set("Tag", Value::String(b"CAVE".to_vec()));
        from.set("Tile_List", Value::List(vec![tile(27, 0, 2), tile(27, 0, 2), tile(4, 0, 0)]));
        let text = Saved::of_area(&from).to_text();
        assert!(!text.contains("Tag"), "{text}");
        let saved = Saved::parse(&text).unwrap();
        assert_eq!(saved.tiles(), Some(&[[27, 27, 27, 4], [0; 4], [2, 2, 2, 0]]));
        let mut to = Struct::new(0);
        to.set("SunFogAmount", Value::Byte(3));
        let fields = saved.fields_for(&to).unwrap();
        assert!(fields.contains(&("SunAmbientColor", Value::Dword(0x40_4040))), "{fields:?}");
        assert!(fields.contains(&("SunFogAmount", Value::Byte(10))));
        assert!(fields.contains(&("FogClipDist", Value::Float(45.5))));
        assert!(Saved::parse("{\"lighting\": {}}").is_err());
        let byte = Saved::parse("{\"lighting\": {\"SunFogAmount\": 300}}").unwrap();
        assert!(byte.fields_for(&to).is_err(), "a byte");
        // A file kept before the tiles were: its fields, the tiles left be.
        let old = Saved::parse("{\"lighting\": {\"SunFogAmount\": 4}}").unwrap();
        assert_eq!(old.tiles(), None);
        let mut area = Struct::new(0);
        area.set("Tile_List", Value::List(vec![tile(9, 9, 9)]));
        assert_eq!(old.give(&mut area, &mut fastrand::Rng::with_seed(1), true), Ok((true, 0)));
        assert_eq!(area.list("Tile_List").unwrap()[0], tile(9, 9, 9));
        for bad in [
            "{\"lighting\": {\"SunFogAmount\": \"x\"}}",
            "{\"lighting\": {}, \"tile_lights\": {\"main1\": [1, 2, 3]}}",
            "{\"lighting\": {}, \"tile_lights\": {\"main1\": [1, 2, 3, 256], \"main2\": [0, 0, 0, 0], \"source\": [0, 0, 0, 0]}}",
            "[]",
            "",
        ] {
            assert!(Saved::parse(bad).is_err(), "{bad}");
        }
    }

    /// The four colors a light is picked from are an area's own, each as
    /// often as its tiles have it, in quarters: one color in all four; a
    /// color of three tiles in four in three; five colors, the four used
    /// most, once each.
    #[test]
    fn tile_colors_are_shared_by_how_much_each_is_used() {
        let counts = |pairs: &[(u8, usize)]| pairs.iter().copied().collect::<BTreeMap<_, _>>();
        assert_eq!(four_of(counts(&[(7, 12)])), [7; 4]);
        assert_eq!(four_of(counts(&[(7, 9), (2, 3)])), [7, 7, 7, 2]);
        assert_eq!(four_of(counts(&[(7, 1), (2, 1)])), [2, 2, 7, 7], "as often: the lower first");
        assert_eq!(four_of(counts(&[(1, 5), (2, 4), (3, 3), (4, 2), (5, 1)])), [1, 2, 3, 4]);
        assert_eq!(four_of(counts(&[(1, 97), (2, 1), (3, 1), (4, 1)])), [1, 2, 3, 4]);
        assert_eq!(four_of(counts(&[(1, 60), (2, 40)])), [1, 1, 2, 2]);
        assert_eq!(four_of(counts(&[(1, 70), (2, 20), (3, 10)])), [1, 1, 2, 3]);
        assert_eq!(tile_colors(&[]), None);
    }

    /// Given to an area, its tiles' lights are picked from the colors kept
    /// (both source fields alike), and the same seed picks the same.
    #[test]
    fn kept_lighting_picks_each_tile_s_lights_from_its_colors() {
        let mut from = Struct::new(0);
        from.set("SunFogAmount", Value::Byte(10));
        let tiles: Vec<Struct> = (0..8).map(|i| tile(if i < 6 { 5 } else { 6 }, 1, 3)).collect();
        from.set("Tile_List", Value::List(tiles));
        let saved = Saved::of_area(&from);
        let mut to = Struct::new(0);
        to.set("Tile_List", Value::List(vec![tile(0, 0, 0); 40]));
        // Not asked for: the tiles' lights left as they are.
        let mut left = to.clone();
        assert_eq!(saved.give(&mut left, &mut fastrand::Rng::with_seed(7), false), Ok((true, 0)));
        assert_eq!(left.list("Tile_List"), to.list("Tile_List"));
        let (fields, lit) = saved.give(&mut to, &mut fastrand::Rng::with_seed(7), true).unwrap();
        assert!(fields);
        assert_eq!(lit, 40);
        for t in to.list("Tile_List").unwrap() {
            assert!(matches!(t.integer("Tile_MainLight1"), Some(5 | 6)), "{t:?}");
            assert_eq!(t.integer("Tile_MainLight2"), Some(1));
            assert_eq!(t.integer("Tile_SrcLight1"), Some(3));
            assert_eq!(t.integer("Tile_SrcLight2"), Some(3));
        }
        let mut again = Struct::new(0);
        again.set("Tile_List", Value::List(vec![tile(0, 0, 0); 40]));
        saved.give(&mut again, &mut fastrand::Rng::with_seed(7), true).unwrap();
        assert_eq!(again.list("Tile_List"), to.list("Tile_List"));
    }

    /// As an environment.2da row: the inverse of what a scheme gives an
    /// area (colors red in the low byte), its day and night, its tiles'
    /// colors, and a label made of its name.
    #[test]
    fn kept_lighting_is_an_environment_row() {
        let mut from = Struct::new(0);
        for (label, v) in [
            ("SunAmbientColor", Value::Dword(0x03_0201)),
            ("MoonFogColor", Value::Dword(0x0c_0b0a)),
            ("SunFogAmount", Value::Byte(12)),
            ("MoonShadows", Value::Byte(1)),
            ("DayNightCycle", Value::Byte(0)),
            ("IsNight", Value::Byte(1)),
            ("ShadowOpacity", Value::Byte(35)),
            ("WindPower", Value::Int(2)),
            ("ChanceRain", Value::Int(40)),
        ] {
            from.set(label, v);
        }
        from.set("Tile_List", Value::List(vec![tile(27, 0, 2)]));
        let row = Saved::of_area(&from).environment_row(&row_label("Crypt, damp - 2"));
        let get = |c: &str| row.iter().find(|(k, _)| k == c).map(|(_, v)| v.as_str());
        assert_eq!(get("LABEL"), Some("Crypt_damp___2"));
        assert_eq!(get("STRREF"), Some("****"));
        assert_eq!(get("DAYNIGHT"), Some("night"));
        assert_eq!((get("LIGHT_AMB_RED"), get("LIGHT_AMB_GREEN")), (Some("1"), Some("2")));
        assert_eq!(get("LIGHT_AMB_BLUE"), Some("3"));
        assert_eq!(get("DARK_FOG_BLUE"), Some("12"));
        assert_eq!(get("LIGHT_FOG"), Some("12"));
        assert_eq!(get("DARK_SHADOWS"), Some("1"));
        assert_eq!(get("MAIN1_COLOR4"), Some("27"));
        assert_eq!(get("SECONDARY_COLOR1"), Some("2"));
        assert_eq!(get("RAIN"), Some("40"));
        assert_eq!(get("SHADOW_ALPHA"), Some("0.35"));
        assert_eq!(row_label("  "), "Lighting");
    }

    /// The row added to a table of the game's columns (as the 2DA editor
    /// adds it) is read back as a scheme that gives an area what the
    /// lighting gives it: its fields (but the sky box and the fog's
    /// distance, which a scheme sets back) and its tiles' colors.
    #[test]
    fn a_kept_lighting_s_row_is_read_back_as_a_scheme() {
        use mg_resman::{LayerClass, MemContainer, ResKey, ResMan, priority};
        let mut columns = vec!["LABEL", "STRREF", "DAYNIGHT"];
        let mut owned = Vec::new();
        for p in ["LIGHT", "DARK"] {
            for part in ["AMB", "DIFF"] {
                for c in ["RED", "GREEN", "BLUE"] {
                    owned.push(format!("{p}_{part}_{c}"));
                }
            }
            owned.push(format!("{p}_SHADOWS"));
        }
        for p in ["LIGHT", "DARK"] {
            for c in ["RED", "GREEN", "BLUE"] {
                owned.push(format!("{p}_FOG_{c}"));
            }
        }
        owned.extend(["LIGHT_FOG".into(), "DARK_FOG".into()]);
        for p in ["MAIN1_COLOR", "MAIN2_COLOR", "SECONDARY_COLOR"] {
            owned.extend((1..=4).map(|n| format!("{p}{n}")));
        }
        owned.extend(["WIND", "SNOW", "RAIN", "LIGHTNING", "SHADOW_ALPHA"].map(String::from));
        columns.extend(owned.iter().map(String::as_str));
        let empty = vec!["****"; columns.len()].join(" ");
        let text = format!("2DA V2.0\n\n {}\n0 {empty}\n", columns.join(" "));
        let table = mg_2da::TwoDa::parse(text.as_bytes(), Codepage::default()).unwrap();

        let mut from = Struct::new(0);
        for (label, v) in [
            ("SunAmbientColor", Value::Dword(0x30_2010)),
            ("SunDiffuseColor", Value::Dword(0xff_8040)),
            ("MoonAmbientColor", Value::Dword(0x01_0203)),
            ("MoonDiffuseColor", Value::Dword(0x0a_0b0c)),
            ("SunFogColor", Value::Dword(0x11_2233)),
            ("MoonFogColor", Value::Dword(0x44_5566)),
            ("SunFogAmount", Value::Byte(12)),
            ("MoonFogAmount", Value::Byte(30)),
            ("SunShadows", Value::Byte(1)),
            ("MoonShadows", Value::Byte(0)),
            ("DayNightCycle", Value::Byte(0)),
            ("IsNight", Value::Byte(1)),
            ("ShadowOpacity", Value::Byte(35)),
            ("WindPower", Value::Int(2)),
            ("ChanceRain", Value::Int(40)),
            ("ChanceSnow", Value::Int(5)),
            ("ChanceLightning", Value::Int(1)),
            ("SkyBox", Value::Byte(3)),
            ("FogClipDist", Value::Float(80.0)),
        ] {
            from.set(label, v);
        }
        from.set("Tile_List", Value::List(vec![tile(27, 3, 2), tile(4, 3, 2)]));
        let saved = Saved::of_area(&from);
        let mut edit = crate::table_edit::Editing::new(table);
        let row = edit.add_row_of("Add", &saved.environment_row("Crypt"));
        let bytes = edit.table().to_bytes(Codepage::default()).unwrap();
        let mut mem = MemContainer::new();
        let key = ResKey::new(mg_core::ResRef::from_str("environment").unwrap(), ResType::TWODA);
        mem.insert(key, bytes);
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:own", LayerClass::Erf, mem);
        let game = mg_rules::GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        assert_eq!(crate::new::Scheme::find(&game, "Crypt"), Ok(row));
        // Listed by its label where the Visual page lists the schemes (it
        // has no talk-table name).
        let columns = mg_rules::ChoiceColumns { name: Some("STRREF"), label: Some("LABEL") };
        let listed = game.choices("environment", columns).unwrap();
        assert!(listed.iter().any(|c| c.row == row && c.text == "Crypt"), "{listed:?}");
        let scheme = crate::new::Scheme::read(&game, row).unwrap();
        let given = saved.fields_for(&from).unwrap();
        for (label, value) in scheme.fields() {
            if label == "LightingScheme" {
                assert_eq!(value, Value::Byte(row as u8));
                continue;
            }
            let kept = given.iter().find(|(l, _)| *l == label).map(|(_, v)| v.as_integer());
            assert_eq!(kept, Some(value.as_integer()), "{label}");
        }
        assert_eq!(scheme.tile_colors(), *saved.tiles().unwrap());
    }
}
