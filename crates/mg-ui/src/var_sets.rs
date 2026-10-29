//! Variable sets: local variables saved under a name (Save Set in the
//! Variables window) and added to any object or blueprint's variables, in
//! any module (Add Set). They live in Moonglow's data folder as small JSON
//! files, easy to share or write by hand:
//!
//! ```json
//! { "variables": [ { "name": "MG_LOOT", "type": "int", "value": 3 } ] }
//! ```
//!
//! Types are `int`, `float` and `string`, as the toolset's variables. A
//! set can also be saved to a file anywhere and added from one (Save to
//! File…, Add from File…), in the same format.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::saved::Shelf;
use crate::widgets::VarRow;

const SHELF: Shelf = Shelf { folder: "variable-sets", suffix: ".vars.json", what: "variable set" };

/// Where variable sets are kept.
pub fn dir() -> Option<PathBuf> {
    SHELF.dir()
}

/// The sets saved in `dir`, by name.
pub fn list(dir: Option<&Path>) -> Vec<String> {
    SHELF.list(dir)
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
    SHELF.write(Some(dir), name, &to_text(rows))
}

/// Reads the set named `name`.
pub fn load(dir: &Path, name: &str) -> Result<Vec<VarRow>, String> {
    let text = SHELF.read(Some(dir), name)?;
    from_text(&text).map_err(|e| format!("{name}: {e}"))
}

/// Deletes the set named `name`: its text, to put it back with [`restore`].
pub fn delete(dir: &Path, name: &str) -> Result<String, String> {
    SHELF.delete(Some(dir), name)
}

/// Puts a deleted set back, unless one of its name was saved since.
pub fn restore(dir: &Path, name: &str, text: &str) -> Result<(), String> {
    SHELF.restore(Some(dir), name, text)
}

/// Gives a set another name: a name another set has, or one that names
/// no file, is refused.
pub fn rename(dir: &Path, from: &str, to: &str) -> Result<(), String> {
    SHELF.rename(Some(dir), from, to)
}

/// The sets saved in `dir`, by name, each read: its variables, or why its
/// file isn't a set (bad JSON, an unknown type), which the menu and the
/// Variable Sets window say beside its name. A file is read again when
/// its size or modified time changes (`kept`, from the last call: a
/// window lists them every frame).
pub fn read_all(dir: Option<&Path>, kept: &mut Read) -> Vec<(String, Result<Vec<VarRow>, String>)> {
    let names = list(dir);
    let mut out = Vec::with_capacity(names.len());
    let mut now = Read::default();
    for name in names {
        let Ok(path) = SHELF.file(dir, &name) else { continue };
        let stamp = std::fs::metadata(&path).ok().map(|m| (m.len(), m.modified().ok()));
        let read = match kept.0.remove(&path) {
            Some((was, read)) if was == stamp => read,
            _ => load(dir.expect("listed"), &name),
        };
        now.0.insert(path, (stamp, read.clone()));
        out.push((name, read));
    }
    *kept = now;
    out
}

/// The sets as [`read_all`] last read them, by file.
#[derive(Debug, Clone, Default)]
#[allow(clippy::type_complexity)]
pub struct Read(
    std::collections::HashMap<
        PathBuf,
        (Option<(u64, Option<std::time::SystemTime>)>, Result<Vec<VarRow>, String>),
    >,
);

/// The most variables a set's summary names ([`describe`]).
const DESCRIBED: usize = 12;

/// What a set holds, as its tip says it: a variable a line, `NAME (type)
/// = value`, the first [`DESCRIBED`] and how many more.
pub fn describe(rows: &[VarRow]) -> String {
    if rows.is_empty() {
        return "No variables".to_string();
    }
    let kind = |k: u32| match k {
        1 => "int",
        2 => "float",
        _ => "string",
    };
    let mut lines: Vec<String> = rows
        .iter()
        .take(DESCRIBED)
        .map(|r| match r.kind {
            3 => format!("{} ({}) = \"{}\"", r.name, kind(r.kind), r.value),
            k => format!("{} ({}) = {}", r.name, kind(k), r.value),
        })
        .collect();
    if rows.len() > DESCRIBED {
        lines.push(format!("and {} more", rows.len() - DESCRIBED));
    }
    lines.join("\n")
}

/// What a set's file is called: `NAME.vars.json`.
pub fn file_name(name: &str) -> String {
    format!("{}{}", name.trim(), SHELF.suffix)
}

/// Saves variables as a set to the file `path` (anywhere), named with the
/// sets' ending, `.vars.json`, whatever the dialog gave (Windows' adds
/// `.json` to a name typed bare, others add nothing): so a copy put in
/// the sets' folder is listed.
pub fn save_file(path: &Path, rows: &[VarRow]) -> Result<PathBuf, String> {
    let path = match path.file_name() {
        Some(n) => {
            let n = n.to_string_lossy();
            let lower = n.to_lowercase();
            let cut = [SHELF.suffix, ".json", ".vars"]
                .iter()
                .find(|s| lower.ends_with(*s))
                .map_or(n.len(), |s| n.len() - s.len());
            path.with_file_name(file_name(&n[..cut]))
        }
        None => path.to_path_buf(),
    };
    std::fs::write(&path, to_text(rows)).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Reads the set in the file `path` (anywhere).
pub fn load_file(path: &Path) -> Result<Vec<VarRow>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    from_text(&text).map_err(|e| format!("{}: {e}", path.display()))
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

    #[test]
    fn sets_are_renamed_deleted_and_kept_in_files_anywhere() {
        let scratch = mg_testkit::scratch_dir("ui-var-sets-files");
        let dir = scratch.join("variable-sets");
        let rows = vec![VarRow::new("MG_LOOT", 1, "3"), VarRow::new("MG_SAY", 3, "Hail")];
        save(&dir, "Loot", &rows).unwrap();
        assert!(dir.join("Loot.vars.json").is_file());
        // Renamed, refused over another's name or one that isn't a file's.
        save(&dir, "Doors", &rows[..1]).unwrap();
        rename(&dir, "Loot", "Treasure").unwrap();
        assert_eq!(list(Some(&dir)), ["Doors", "Treasure"]);
        assert!(rename(&dir, "Treasure", "Doors").unwrap_err().contains("variable set"));
        assert!(rename(&dir, "Treasure", "../Doors").is_err());
        assert_eq!(load(&dir, "Treasure").unwrap().len(), 2);
        // Deleted and put back.
        let text = delete(&dir, "Treasure").unwrap();
        assert_eq!(list(Some(&dir)), ["Doors"]);
        restore(&dir, "Treasure", &text).unwrap();
        assert_eq!(load(&dir, "Treasure").unwrap().len(), 2);
        // A file of one's own choosing, read back as the set.
        let file = scratch.join("shared").join("loot.vars.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        assert_eq!(save_file(&file, &rows).unwrap(), file);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), to_text(&rows));
        let back = load_file(&file).unwrap();
        assert_eq!(back.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["MG_LOOT", "MG_SAY"]);
        // A name given without the whole ending is given it.
        let bare = save_file(&scratch.join("bare"), &rows).unwrap();
        assert_eq!(bare, scratch.join("bare.vars.json"));
        let half = save_file(&scratch.join("half.vars"), &rows).unwrap();
        assert_eq!(half, scratch.join("half.vars.json"));
        let plain = save_file(&scratch.join("plain.json"), &rows).unwrap();
        assert_eq!(plain, scratch.join("plain.vars.json"), "Windows' dialog adds .json");
        let upper = save_file(&scratch.join("Upper.VARS.JSON"), &rows).unwrap();
        assert_eq!(upper, scratch.join("Upper.vars.json"));
        // A file that isn't a set says which file.
        std::fs::write(scratch.join("bad.vars.json"), "{}").unwrap();
        let e = load_file(&scratch.join("bad.vars.json")).unwrap_err();
        assert!(e.contains("bad.vars.json") && e.contains("no \"variables\""), "{e}");
    }

    /// Every set read, a file that isn't one said so by its name; read
    /// again when it changes, and summed up for its tip.
    #[test]
    fn sets_are_read_with_those_that_are_not_sets_said() {
        let dir = mg_testkit::scratch_dir("ui-var-sets-read").join("variable-sets");
        let rows: Vec<VarRow> =
            (0..14).map(|i| VarRow::new(&format!("MG_V{i:02}"), 1, &i.to_string())).collect();
        save(&dir, "Many", &rows).unwrap();
        save(&dir, "Say", &[VarRow::new("MG_SAY", 3, "Hail"), VarRow::new("MG_F", 2, "0.5")])
            .unwrap();
        std::fs::write(dir.join("Broken.vars.json"), "{ \"variables\": [").unwrap();
        let mut kept = Read::default();
        let read = read_all(Some(&dir), &mut kept);
        let names: Vec<&str> = read.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Broken", "Many", "Say"]);
        assert!(read[0].1.as_ref().unwrap_err().contains("Broken"), "{:?}", read[0].1);
        let say = describe(read[2].1.as_ref().unwrap());
        assert_eq!(say, "MG_SAY (string) = \"Hail\"\nMG_F (float) = 0.5");
        let many = describe(read[1].1.as_ref().unwrap());
        assert_eq!(many.lines().count(), 13);
        assert!(many.starts_with("MG_V00 (int) = 0\n") && many.ends_with("and 2 more"), "{many}");
        assert_eq!(describe(&[]), "No variables");
        // Mended by hand: read again.
        std::fs::write(dir.join("Broken.vars.json"), "{ \"variables\": [] }").unwrap();
        let read = read_all(Some(&dir), &mut kept);
        assert_eq!(read[0].1.as_ref().map(Vec::len), Ok(0));
        // Gone: forgotten.
        std::fs::remove_file(dir.join("Many.vars.json")).unwrap();
        assert_eq!(read_all(Some(&dir), &mut kept).len(), 2);
        assert_eq!(kept.0.len(), 2);
        assert!(read_all(None, &mut kept).is_empty());
    }
}
