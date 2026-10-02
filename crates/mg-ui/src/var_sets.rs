//! Variable sets: local variables saved under a name (Save Set in the
//! Variables window) and added to any object or blueprint's variables, in
//! any module (Add Set). They live in Moonglow's data folder as small JSON
//! files, easy to share or write by hand:
//!
//! ```json
//! { "variables": [ { "name": "MG_LOOT", "type": "int", "value": 3 } ] }
//! ```
//!
//! Types are `int`, `float` and `string`, as the toolset's variables.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::recovery;
use crate::widgets::VarRow;

const SUFFIX: &str = ".vars.json";

/// Where variable sets are kept.
pub fn dir() -> Option<PathBuf> {
    recovery::data_dir().map(|d| d.join("variable-sets"))
}

/// The sets saved in `dir`, by name.
pub fn list(dir: Option<&Path>) -> Vec<String> {
    let Some(dir) = dir else { return Vec::new() };
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_suffix(SUFFIX).map(str::to_string))
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// Variables as a set's text (only int, float and string variables).
pub fn to_text(rows: &[VarRow]) -> String {
    let vars: Vec<Value> = rows
        .iter()
        .filter_map(|r| {
            let (kind, value) = match r.kind {
                1 => ("int", json!(r.value.trim().parse::<i32>().ok()?)),
                // As typed (an f32 would print 0.1 as 0.10000000149…).
                2 => ("float", json!(r.value.trim().parse::<f64>().ok()?)),
                3 => ("string", json!(r.value)),
                _ => return None,
            };
            Some(json!({ "name": r.name, "type": kind, "value": value }))
        })
        .collect();
    serde_json::to_string_pretty(&json!({ "variables": vars })).expect("JSON") + "\n"
}

/// A set's text as variables.
pub fn from_text(text: &str) -> Result<Vec<VarRow>, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let vars = v.get("variables").and_then(Value::as_array).ok_or("no \"variables\" list")?;
    vars.iter()
        .map(|var| {
            let name = var.get("name").and_then(Value::as_str).ok_or("a variable has no name")?;
            let value = var.get("value").ok_or_else(|| format!("{name} has no value"))?;
            let (kind, value) = match var.get("type").and_then(Value::as_str) {
                Some("int") => {
                    (1, value.as_i64().ok_or(format!("{name}: not an int"))?.to_string())
                }
                Some("float") => {
                    (2, value.as_f64().ok_or(format!("{name}: not a float"))?.to_string())
                }
                Some("string") => (3, value.as_str().unwrap_or_default().to_string()),
                other => return Err(format!("{name}: type {other:?} isn't int, float or string")),
            };
            Ok(VarRow::new(name, kind, &value))
        })
        .collect()
}

/// Adds a set's variables to `rows`: one of the same name takes the set's
/// type and value; the rest are added.
pub fn merge(rows: &mut Vec<VarRow>, set: Vec<VarRow>) {
    for v in set {
        match rows.iter_mut().find(|r| r.name == v.name) {
            Some(r) => {
                r.kind = v.kind;
                r.value = v.value;
            }
            None => rows.push(v),
        }
    }
}

/// Saves variables as a set named `name`.
pub fn save(dir: &Path, name: &str, rows: &[VarRow]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}{SUFFIX}", name.trim()));
    std::fs::write(&path, to_text(rows)).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Reads the set named `name`.
pub fn load(dir: &Path, name: &str) -> Result<Vec<VarRow>, String> {
    let path = dir.join(format!("{name}{SUFFIX}"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    from_text(&text).map_err(|e| format!("{name}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_read_back_and_merge_by_name() {
        let rows = vec![
            VarRow::new("MG_LOOT", 1, "3"),
            VarRow::new("MG_SPEED", 2, "0.1"),
            VarRow::new("MG_SAY", 3, "Hello \"there\""),
        ];
        let text = to_text(&rows);
        assert!(text.contains("\"type\": \"int\""), "{text}");
        let back = from_text(&text).unwrap();
        assert_eq!(
            back.iter().map(|r| (r.name.as_str(), r.kind, r.value.as_str())).collect::<Vec<_>>(),
            [("MG_LOOT", 1, "3"), ("MG_SPEED", 2, "0.1"), ("MG_SAY", 3, "Hello \"there\"")]
        );
        let mut mine = vec![VarRow::new("MG_LOOT", 3, "none"), VarRow::new("MG_KEEP", 1, "7")];
        merge(&mut mine, back);
        assert_eq!(
            mine.iter().map(|r| (r.name.as_str(), r.kind, r.value.as_str())).collect::<Vec<_>>(),
            [
                ("MG_LOOT", 1, "3"),
                ("MG_KEEP", 1, "7"),
                ("MG_SPEED", 2, "0.1"),
                ("MG_SAY", 3, "Hello \"there\"")
            ]
        );
        assert!(
            from_text("{\"variables\": [{\"name\": \"X\", \"type\": \"object\", \"value\": 1}]}")
                .unwrap_err()
                .contains("isn't int")
        );
    }
}
