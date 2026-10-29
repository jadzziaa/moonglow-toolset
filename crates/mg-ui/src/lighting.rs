//! Lighting kept under a name: what an area's Visual page sets (its
//! lighting scheme, the sun's and the moon's colors, fog and shadows, day
//! and night, the sky and the weather), saved from one area and given to
//! any other, in any module (dansen9191's ask). Kept in Moonglow's data
//! folder (`lighting`) as JSON, field by field:
//!
//! ```json
//! { "lighting": { "SunAmbientColor": 4210752, "SunFogAmount": 10, "FogClipDist": 45.0 } }
//! ```
//!
//! The fields are the Visual page's ([`crate::area_props::VISUAL_FIELDS`]),
//! as Give Every Area This Page's Settings and Copy From Area give them.

use std::path::PathBuf;

use mg_core::Codepage;
use mg_gff::{FieldType, Struct, Value};
use serde_json::{Map, json};

use crate::saved::Shelf;

const SHELF: Shelf = Shelf { folder: "lighting", suffix: ".lighting.json", what: "lighting" };

/// Where lighting is kept.
pub fn dir() -> Option<PathBuf> {
    SHELF.dir()
}

/// The lighting kept in `dir`, by name.
pub fn list(dir: Option<&std::path::Path>) -> Vec<String> {
    SHELF.list(dir)
}

/// An area's lighting (its root, an ARE's) as a file's text: the Visual
/// page's fields it has, each a number.
pub fn to_text(are: &Struct) -> String {
    let mut fields = Map::new();
    for label in crate::area_props::VISUAL_FIELDS {
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
    serde_json::to_string_pretty(&json!({ "lighting": fields })).expect("JSON") + "\n"
}

/// A file's lighting as the fields of an area whose root is `are`: each
/// of the type it has there (the game's, for one it lacks). Only the
/// Visual page's fields are taken.
pub fn from_text(text: &str, are: &Struct) -> Result<Vec<(&'static str, Value)>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let fields = v.get("lighting").and_then(|l| l.as_object()).ok_or("no \"lighting\" fields")?;
    let mut out = Vec::new();
    for label in crate::area_props::VISUAL_FIELDS {
        let Some(n) = fields.get(*label) else { continue };
        let ty = are
            .get(label)
            .map(Value::field_type)
            .or_else(|| mg_schema::root_field_type(mg_core::ResType::ARE, label))
            .unwrap_or(FieldType::Int);
        let value = Value::from_text(ty, &n.to_string(), None, Codepage::default())
            .map_err(|e| format!("{label}: {e}"))?;
        out.push((*label, value));
    }
    if out.is_empty() {
        return Err("no lighting fields".into());
    }
    Ok(out)
}

/// Keeps an area's lighting as `name`.
pub fn save(dir: Option<&std::path::Path>, name: &str, are: &Struct) -> Result<PathBuf, String> {
    SHELF.write(dir, name, &to_text(are))
}

/// The lighting named `name`, as the fields of the area whose root is
/// `are`.
pub fn load(
    dir: Option<&std::path::Path>,
    name: &str,
    are: &Struct,
) -> Result<Vec<(&'static str, Value)>, String> {
    from_text(&SHELF.read(dir, name)?, are).map_err(|e| format!("{name}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Saved from one area and read as another's fields: each of the type
    /// the area has it as (an older area's byte, a dword), the page's
    /// fields only.
    #[test]
    fn lighting_reads_back_as_an_area_s_fields() {
        let mut from = Struct::new(0);
        from.set("SunAmbientColor", Value::Dword(0x40_4040));
        from.set("SunFogAmount", Value::Byte(10));
        from.set("FogClipDist", Value::Float(45.5));
        from.set("Tag", Value::String(b"CAVE".to_vec()));
        let text = to_text(&from);
        assert!(!text.contains("Tag"), "{text}");
        let mut to = Struct::new(0);
        to.set("SunFogAmount", Value::Byte(3));
        let fields = from_text(&text, &to).unwrap();
        assert!(fields.contains(&("SunAmbientColor", Value::Dword(0x40_4040))), "{fields:?}");
        assert!(fields.contains(&("SunFogAmount", Value::Byte(10))));
        assert!(fields.contains(&("FogClipDist", Value::Float(45.5))));
        assert!(from_text("{\"lighting\": {}}", &to).is_err());
        assert!(from_text("{\"lighting\": {\"SunFogAmount\": 300}}", &to).is_err(), "a byte");
    }
}
