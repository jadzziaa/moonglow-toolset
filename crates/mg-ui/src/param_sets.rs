//! Parameter sets: a conversation script's parameters saved under a name
//! (Save Set beside the parameters of Text Appears When… and Actions
//! Taken) and added to any line's or link's, in any conversation of any
//! module (Add Set), with the script they were saved with. They live in
//! Moonglow's data folder as small JSON files, easy to share or write by
//! hand:
//!
//! ```json
//! { "script": "ttc_c_skillcheck",
//!   "parameters": [ { "name": "CHECK", "value": "12" }, { "name": "DC", "value": "1" } ] }
//! ```
//!
//! The values are text, as the game's parameters are (`GetScriptParam`);
//! a number written by hand is read as its text. `script` may be left
//! out or empty: a set of parameters alone. A set can also be saved to a
//! file anywhere and added from one (Save to File…, Add from File…), in
//! the same format.

use std::path::{Path, PathBuf};

use mg_core::ResRef;
use serde_json::{Value, json};

use crate::saved::Shelf;

pub(crate) const SHELF: Shelf =
    Shelf { folder: "parameter-sets", suffix: ".params.json", what: "parameter set" };

/// A parameter set: the script it was saved with, if any, and its
/// parameters (name and value) in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParamSet {
    pub script: Option<ResRef>,
    pub parameters: Vec<(String, String)>,
}

/// Where parameter sets are kept.
pub fn dir() -> Option<PathBuf> {
    SHELF.dir()
}

/// The sets saved in `dir`, by name.
pub fn list(dir: Option<&Path>) -> Vec<String> {
    SHELF.list(dir)
}

/// A set's text. A parameter with no name is left out (the game can't
/// ask for it).
pub fn to_text(set: &ParamSet) -> String {
    let params: Vec<Value> = (set.parameters.iter())
        .filter(|(name, _)| !name.trim().is_empty())
        .map(|(name, value)| json!({ "name": name, "value": value }))
        .collect();
    let mut v = serde_json::Map::new();
    if let Some(script) = set.script.filter(|s| !s.is_empty()) {
        v.insert("script".into(), json!(script.to_string_lossy()));
    }
    v.insert("parameters".into(), Value::Array(params));
    serde_json::to_string_pretty(&Value::Object(v)).expect("JSON") + "\n"
}

/// A set's text as a set.
pub fn from_text(text: &str) -> Result<ParamSet, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let script = match v.get("script") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.trim().is_empty() => None,
        Some(Value::String(s)) => Some(
            ResRef::from_str(s.trim())
                .map_err(|_| format!("\"{s}\" isn't a script's name (16 letters at most)"))?,
        ),
        Some(_) => return Err("\"script\" isn't a script's name".into()),
    };
    let params = v.get("parameters").and_then(Value::as_array).ok_or("no \"parameters\" list")?;
    let parameters = params
        .iter()
        .map(|p| {
            let name = p.get("name").and_then(Value::as_str).ok_or("a parameter has no name")?;
            let value = match p.get("value") {
                Some(Value::String(s)) => s.clone(),
                // (Written by hand as a number or TRUE: its text.)
                Some(n @ (Value::Number(_) | Value::Bool(_))) => n.to_string(),
                Some(_) => return Err(format!("{name}: the value isn't text")),
                None => return Err(format!("{name} has no value")),
            };
            Ok((name.to_string(), value))
        })
        .collect::<Result<_, String>>()?;
    Ok(ParamSet { script, parameters })
}

/// Adds a set's parameters to `pairs`: one of the same name takes the
/// set's value; the rest are added after.
pub fn merge(pairs: &mut Vec<(String, String)>, set: &[(String, String)]) {
    for (name, value) in set {
        match pairs.iter_mut().find(|(n, _)| n == name) {
            Some(p) => p.1 = value.clone(),
            None => pairs.push((name.clone(), value.clone())),
        }
    }
}

/// Saves a set named `name`.
pub fn save(dir: &Path, name: &str, set: &ParamSet) -> Result<PathBuf, String> {
    SHELF.write(Some(dir), name, &to_text(set))
}

/// Reads the set named `name`.
pub fn load(dir: &Path, name: &str) -> Result<ParamSet, String> {
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

/// The sets saved in `dir`, by name, each read: the set, or why its file
/// isn't one, which the menu and the Parameter Sets window say beside
/// its name (read again only when a file changes: `kept`).
pub fn read_all(dir: Option<&Path>, kept: &mut Read) -> Vec<(String, Result<ParamSet, String>)> {
    SHELF.read_all(dir, kept, load)
}

/// The sets as [`read_all`] last read them, by file.
pub type Read = crate::saved::Read<ParamSet>;

/// The most parameters a set's summary names ([`describe`]).
const DESCRIBED: usize = 12;

/// What a set holds, as its tip says it: its script, then a parameter a
/// line, `NAME = value`, the first [`DESCRIBED`] and how many more.
pub fn describe(set: &ParamSet) -> String {
    let mut lines = Vec::new();
    if let Some(script) = set.script {
        lines.push(format!("Script: {script}"));
    }
    if set.parameters.is_empty() {
        lines.push("No parameters".to_string());
    }
    lines.extend(set.parameters.iter().take(DESCRIBED).map(|(n, v)| format!("{n} = {v}")));
    if set.parameters.len() > DESCRIBED {
        lines.push(format!("and {} more", set.parameters.len() - DESCRIBED));
    }
    lines.join("\n")
}

/// What a set's file is called: `NAME.params.json`.
pub fn file_name(name: &str) -> String {
    format!("{}{}", name.trim(), SHELF.suffix)
}

/// Saves a set to the file `path` (anywhere), named with the sets'
/// ending, `.params.json`, whatever the dialog gave (Windows' adds
/// `.json` to a name typed bare, others add nothing): so a copy put in
/// the sets' folder is listed.
pub fn save_file(path: &Path, set: &ParamSet) -> Result<PathBuf, String> {
    let path = match path.file_name() {
        Some(n) => {
            let n = n.to_string_lossy();
            let lower = n.to_lowercase();
            let cut = [SHELF.suffix, ".json", ".params"]
                .iter()
                .find(|s| lower.ends_with(*s))
                .map_or(n.len(), |s| n.len() - s.len());
            path.with_file_name(file_name(&n[..cut]))
        }
        None => path.to_path_buf(),
    };
    crate::saved::write_whole(&path, &to_text(set))?;
    Ok(path)
}

/// Reads the set in the file `path` (anywhere).
pub fn load_file(path: &Path) -> Result<ParamSet, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    from_text(&text).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter().map(|(n, v)| (n.to_string(), v.to_string())).collect()
    }

    fn skillcheck() -> ParamSet {
        ParamSet {
            script: Some(ResRef::from_str("ttc_c_skillcheck").unwrap()),
            parameters: pairs(&[("CHECK", "12"), ("DC", "1"), ("FEEDBACK", "1"), ("SHOW_DC", "1")]),
        }
    }

    #[test]
    fn sets_read_back_and_merge_by_name() {
        let set = skillcheck();
        let text = to_text(&set);
        assert!(text.contains("\"script\": \"ttc_c_skillcheck\""), "{text}");
        assert!(text.contains("\"name\": \"CHECK\"") && text.contains("\"value\": \"12\""));
        assert_eq!(from_text(&text).unwrap(), set);
        // Values with quotes and non-ASCII letters, kept as they are.
        let odd =
            ParamSet { script: None, parameters: pairs(&[("say", "\"Hail\", café"), ("x", "")]) };
        assert_eq!(from_text(&to_text(&odd)).unwrap(), odd);
        // Merged: one of the same name takes the set's value, in its place;
        // the rest after.
        let mut mine = pairs(&[("DC", "20"), ("MINE", "7")]);
        merge(&mut mine, &set.parameters);
        assert_eq!(
            mine,
            pairs(&[
                ("DC", "1"),
                ("MINE", "7"),
                ("CHECK", "12"),
                ("FEEDBACK", "1"),
                ("SHOW_DC", "1")
            ])
        );
        // A parameter with no name isn't kept.
        let unnamed = ParamSet { script: None, parameters: pairs(&[("", "x"), ("A", "1")]) };
        assert_eq!(from_text(&to_text(&unnamed)).unwrap().parameters, pairs(&[("A", "1")]));
    }

    /// No script, or an empty one: a set of parameters alone. Written by
    /// hand: a number's value read as its text.
    #[test]
    fn a_set_s_script_may_be_left_out() {
        let none = ParamSet { script: None, parameters: pairs(&[("DC", "1")]) };
        let text = to_text(&none);
        assert!(!text.contains("script"), "{text}");
        assert_eq!(from_text(&text).unwrap(), none);
        let empty = from_text("{ \"script\": \"\", \"parameters\": [] }").unwrap();
        assert_eq!(empty, ParamSet::default());
        let null = from_text("{ \"script\": null, \"parameters\": [] }").unwrap();
        assert_eq!(null.script, None);
        let to_text_empty = ParamSet { script: Some(ResRef::EMPTY), parameters: Vec::new() };
        assert!(!to_text(&to_text_empty).contains("script"));
        let by_hand = from_text(
            "{ \"parameters\": [ { \"name\": \"DC\", \"value\": 12 }, \
             { \"name\": \"ON\", \"value\": true } ] }",
        )
        .unwrap();
        assert_eq!(by_hand.parameters, pairs(&[("DC", "12"), ("ON", "true")]));
    }

    /// What isn't a set says what is wrong: cut short, another file's
    /// JSON, a parameter with no name or no value, a script's name too long.
    #[test]
    fn a_file_that_isn_t_a_set_says_why() {
        let text = to_text(&skillcheck());
        for cut in [1, text.len() / 2, text.len() - 3] {
            assert!(from_text(&text[..cut]).is_err(), "cut at {cut}");
        }
        let err = |t: &str| from_text(t).unwrap_err();
        assert!(err("{}").contains("no \"parameters\""));
        assert!(err("{ \"variables\": [] }").contains("no \"parameters\""));
        assert!(err("[]").contains("no \"parameters\""));
        assert!(err("{ \"parameters\": [ { \"value\": \"1\" } ] }").contains("has no name"));
        assert_eq!(err("{ \"parameters\": [ { \"name\": \"DC\" } ] }"), "DC has no value");
        assert!(err("{ \"parameters\": [ { \"name\": \"DC\", \"value\": [1] } ] }").contains("DC"));
        let long = "{ \"script\": \"a_name_far_too_long\", \"parameters\": [] }";
        assert!(err(long).contains("a_name_far_too_long"), "{}", err(long));
        assert!(err("{ \"script\": 5, \"parameters\": [] }").contains("script"));
    }

    #[test]
    fn sets_are_renamed_deleted_and_kept_in_files_anywhere() {
        let scratch = mg_testkit::scratch_dir("ui-param-sets-files");
        let dir = scratch.join("parameter-sets");
        save(&dir, "Skill check", &skillcheck()).unwrap();
        assert!(dir.join("Skill check.params.json").is_file());
        save(&dir, "Gold", &ParamSet { script: None, parameters: pairs(&[("GOLD", "50")]) })
            .unwrap();
        rename(&dir, "Skill check", "Check").unwrap();
        assert_eq!(list(Some(&dir)), ["Check", "Gold"]);
        assert!(rename(&dir, "Check", "gold").unwrap_err().contains("parameter set"));
        assert!(rename(&dir, "Check", "../Gold").is_err());
        assert_eq!(load(&dir, "Check").unwrap(), skillcheck());
        let text = delete(&dir, "Check").unwrap();
        assert_eq!(list(Some(&dir)), ["Gold"]);
        restore(&dir, "Check", &text).unwrap();
        assert_eq!(load(&dir, "Check").unwrap(), skillcheck());
        // A file of one's own choosing, read back as the set.
        let file = scratch.join("shared").join("check.params.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        assert_eq!(save_file(&file, &skillcheck()).unwrap(), file);
        assert_eq!(load_file(&file).unwrap(), skillcheck());
        // A name given without the whole ending is given it.
        for (given, made) in [
            ("bare", "bare.params.json"),
            ("half.params", "half.params.json"),
            ("plain.json", "plain.params.json"),
            ("Upper.PARAMS.JSON", "Upper.params.json"),
        ] {
            assert_eq!(save_file(&scratch.join(given), &skillcheck()).unwrap(), scratch.join(made));
        }
        // A file that isn't a set says which file.
        std::fs::write(scratch.join("bad.params.json"), "{}").unwrap();
        let e = load_file(&scratch.join("bad.params.json")).unwrap_err();
        assert!(e.contains("bad.params.json") && e.contains("no \"parameters\""), "{e}");
        assert!(load_file(&scratch.join("none.params.json")).is_err(), "not there");
    }

    /// Every set read, a file that isn't one said so by its name; read
    /// again when it changes, and summed up for its tip.
    #[test]
    fn sets_are_read_with_those_that_are_not_sets_said() {
        let dir = mg_testkit::scratch_dir("ui-param-sets-read").join("parameter-sets");
        let many: Vec<(String, String)> =
            (0..14).map(|i| (format!("P{i:02}"), i.to_string())).collect();
        save(&dir, "Many", &ParamSet { script: None, parameters: many }).unwrap();
        save(&dir, "Check", &skillcheck()).unwrap();
        std::fs::write(dir.join("Broken.params.json"), "{ \"parameters\": [").unwrap();
        let mut kept = Read::default();
        let read = read_all(Some(&dir), &mut kept);
        let names: Vec<&str> = read.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Broken", "Check", "Many"]);
        assert!(read[0].1.as_ref().unwrap_err().contains("Broken"), "{:?}", read[0].1);
        assert_eq!(
            describe(read[1].1.as_ref().unwrap()),
            "Script: ttc_c_skillcheck\nCHECK = 12\nDC = 1\nFEEDBACK = 1\nSHOW_DC = 1"
        );
        let many = describe(read[2].1.as_ref().unwrap());
        assert_eq!(many.lines().count(), 13);
        assert!(many.starts_with("P00 = 0\n") && many.ends_with("and 2 more"), "{many}");
        assert_eq!(describe(&ParamSet::default()), "No parameters");
        let script_alone = ParamSet { script: skillcheck().script, parameters: Vec::new() };
        assert_eq!(describe(&script_alone), "Script: ttc_c_skillcheck\nNo parameters");
        // Mended by hand: read again.
        std::fs::write(dir.join("Broken.params.json"), "{ \"parameters\": [] }").unwrap();
        let read = read_all(Some(&dir), &mut kept);
        assert_eq!(read[0].1.as_ref().map(|s| s.parameters.len()), Ok(0));
        // Gone: forgotten.
        std::fs::remove_file(dir.join("Many.params.json")).unwrap();
        assert_eq!(read_all(Some(&dir), &mut kept).len(), 2);
        assert_eq!(kept.0.len(), 2);
        assert!(read_all(None, &mut kept).is_empty());
    }
}
