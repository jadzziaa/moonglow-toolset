//! Numbers for a custom talk table's lines that stay: a name is given a
//! line once and keeps it, whatever is added or taken away after.
//!
//! A project keeps its strings by name (a key and a text). The map says
//! which line of the table each key has; a new key gets the next line
//! that was never anyone's; a key taken away keeps its line reserved, so
//! that a number saved in a character or an item never comes to mean
//! another text.

use std::collections::BTreeMap;

use mg_core::StrRef;

/// A string by name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Name {
    pub key: String,
    pub text: String,
    /// The feminine table's text, where it is not `text`.
    pub feminine: Option<String>,
    pub sound: String,
    pub sound_length: f32,
}

/// A TOML file as JSON, its keys in the order the file has them (the
/// `toml` crate's tables are sorted: a names file's new keys would take
/// their lines in the order of their names). A date is its text.
pub fn toml_in_order(text: &str) -> Result<serde_json::Value, String> {
    use serde_json::Value as J;
    use toml_edit::{Item, Value as T};
    fn value(v: &T) -> J {
        match v {
            T::String(s) => J::String(s.value().clone()),
            T::Integer(i) => J::from(*i.value()),
            T::Float(f) => serde_json::Number::from_f64(*f.value()).map_or(J::Null, J::Number),
            T::Boolean(b) => J::Bool(*b.value()),
            T::Datetime(d) => J::String(d.value().to_string()),
            T::Array(a) => J::Array(a.iter().map(value).collect()),
            T::InlineTable(t) => {
                J::Object(t.iter().map(|(k, v)| (k.to_string(), value(v))).collect())
            }
        }
    }
    fn table(t: &toml_edit::Table) -> J {
        J::Object(t.iter().map(|(k, i)| (k.to_string(), item(i))).collect())
    }
    fn item(i: &Item) -> J {
        match i {
            Item::None => J::Null,
            Item::Value(v) => value(v),
            Item::Table(t) => table(t),
            Item::ArrayOfTables(a) => J::Array(a.iter().map(table).collect()),
        }
    }
    let doc: toml_edit::DocumentMut = text.parse().map_err(|e| format!("{e}"))?;
    Ok(table(doc.as_table()))
}

/// Which line of the table each key has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Map {
    /// The keys in use.
    pub lines: BTreeMap<String, u32>,
    /// Keys no longer in use, whose lines stay theirs.
    pub retired: BTreeMap<String, u32>,
}

impl Map {
    pub fn read(text: &str) -> Result<Map, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if v.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
            return Err("not a map of talk-table lines (\"version\": 1)".into());
        }
        let part = |name: &str| -> Result<BTreeMap<String, u32>, String> {
            let Some(o) = v.get(name) else { return Ok(BTreeMap::new()) };
            let o = o.as_object().ok_or_else(|| format!("\"{name}\" is not an object"))?;
            o.iter()
                .map(|(k, n)| {
                    let n = n.as_u64().and_then(|n| u32::try_from(n).ok());
                    let n = n.filter(|n| *n < StrRef::CUSTOM_FLAG);
                    Ok((k.clone(), n.ok_or_else(|| format!("{k}: not a line's number"))?))
                })
                .collect()
        };
        let map = Map { lines: part("lines")?, retired: part("retired")? };
        let mut seen = BTreeMap::new();
        for (key, line) in map.lines.iter().chain(&map.retired) {
            if let Some(other) = seen.insert(*line, key) {
                return Err(format!("line {line} is both {other}'s and {key}'s"));
            }
        }
        Ok(map)
    }

    /// The map as a file to keep with the project: one key a line, in
    /// order, so that a change reads well in a diff.
    pub fn to_json(&self) -> String {
        let v = serde_json::json!({
            "version": 1,
            "first_strref": StrRef::CUSTOM_FLAG,
            "lines": self.lines,
            "retired": self.retired,
        });
        serde_json::to_string_pretty(&v).expect("JSON") + "\n"
    }

    /// The game's number for a key's line (16,777,216 and up).
    pub fn strref(&self, key: &str) -> Option<u32> {
        self.lines.get(key).map(|line| line | StrRef::CUSTOM_FLAG)
    }

    /// The keys that have another line here than in `older`.
    pub fn moved(&self, older: &Map) -> Vec<(String, u32, u32)> {
        let all = |m: &Map| -> BTreeMap<String, u32> {
            m.lines.iter().chain(&m.retired).map(|(k, n)| (k.clone(), *n)).collect()
        };
        let (was, now) = (all(older), all(self));
        was.iter()
            .filter_map(|(k, a)| now.get(k).filter(|b| *b != a).map(|b| (k.clone(), *a, *b)))
            .collect()
    }
}

/// What allocating did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub map: Map,
    /// Keys given a line now.
    pub added: Vec<(String, u32)>,
    /// Keys no longer named, whose lines are kept for them.
    pub retired: Vec<String>,
    /// Keys named again, back on the lines they had.
    pub restored: Vec<String>,
}

/// Whether a key can be one: letters, digits, `_`, `.` and `-`.
pub fn is_key(key: &str) -> bool {
    !key.is_empty()
        && key.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Gives every name a line: the one the map has for it, or the next from
/// `start` that is nobody's (not in the map, in use or retired, and not
/// `taken`: a line of the table with a text of someone else's).
pub fn allocate(
    names: &[Name],
    map: &Map,
    start: u32,
    taken: impl Fn(u32) -> bool,
) -> Result<Plan, String> {
    let mut plan = Plan {
        map: Map { retired: map.retired.clone(), ..Default::default() },
        ..Default::default()
    };
    let mut seen = std::collections::BTreeSet::new();
    for n in names {
        if !is_key(&n.key) {
            return Err(format!("{:?} is not a key (letters, digits, _ . -)", n.key));
        }
        if !seen.insert(n.key.as_str()) {
            return Err(format!("{} is there twice", n.key));
        }
    }
    let mut used: std::collections::BTreeSet<u32> =
        map.lines.values().chain(map.retired.values()).copied().collect();
    let mut next = start;
    for n in names {
        let line = if let Some(line) = map.lines.get(&n.key) {
            *line
        } else if let Some(line) = plan.map.retired.remove(&n.key) {
            plan.restored.push(n.key.clone());
            line
        } else {
            while used.contains(&next) || taken(next) {
                next += 1;
            }
            if next >= StrRef::CUSTOM_FLAG {
                return Err("the table is full".into());
            }
            used.insert(next);
            plan.added.push((n.key.clone(), next));
            next
        };
        plan.map.lines.insert(n.key.clone(), line);
    }
    for (key, line) in &map.lines {
        if !plan.map.lines.contains_key(key) {
            plan.map.retired.insert(key.clone(), *line);
            plan.retired.push(key.clone());
        }
    }
    Ok(plan)
}

/// A key as a constant's name: upper case, what isn't a letter or a digit
/// an underscore, after `prefix`.
pub fn constant(key: &str, prefix: &str) -> String {
    let name: String = (key.chars())
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect();
    format!("{prefix}{name}")
}

/// The keys in use with their constants' names and the game's numbers, by
/// number. Two keys that come to one name are an error.
pub fn constants(map: &Map, prefix: &str) -> Result<Vec<(String, u32)>, String> {
    let mut out: Vec<(String, u32)> = Vec::new();
    let mut by_name: BTreeMap<String, &String> = BTreeMap::new();
    for (key, line) in &map.lines {
        let name = constant(key, prefix);
        if let Some(other) = by_name.insert(name.clone(), key) {
            return Err(format!("{other} and {key} are both the constant {name}"));
        }
        out.push((name, line | StrRef::CUSTOM_FLAG));
    }
    out.sort_by_key(|(_, n)| *n);
    Ok(out)
}

/// The constants as an NWScript include.
pub fn constants_nss(map: &Map, prefix: &str) -> Result<String, String> {
    let mut s = String::from(
        "// Written by `mg tlk-alloc`: the custom talk table's lines by name.\n\
         // Do not edit: change the names file and run it again.\n\n",
    );
    for (name, strref) in constants(map, prefix)? {
        s += &format!("const int {name} = {strref};\n");
    }
    Ok(s)
}

/// The constants as JSON: a name, its number.
pub fn constants_json(map: &Map, prefix: &str) -> Result<String, String> {
    let o: serde_json::Map<String, serde_json::Value> =
        constants(map, prefix)?.into_iter().map(|(k, n)| (k, n.into())).collect();
    Ok(serde_json::to_string_pretty(&o).expect("JSON") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(keys: &[&str]) -> Vec<Name> {
        keys.iter()
            .map(|k| Name { key: k.to_string(), text: k.to_uppercase(), ..Default::default() })
            .collect()
    }

    #[test]
    fn a_key_keeps_its_line_whatever_comes_and_goes() {
        let first = allocate(
            &names(&["feat.cleave", "feat.dodge", "skill.hide"]),
            &Map::default(),
            0,
            |_| false,
        )
        .unwrap();
        assert_eq!(first.added.len(), 3);
        assert_eq!(first.map.strref("feat.dodge"), Some(16_777_217));
        // One taken away, one put before the others, one new.
        let second =
            allocate(&names(&["aaa.first", "skill.hide", "feat.cleave"]), &first.map, 0, |_| false)
                .unwrap();
        assert_eq!(second.map.lines["feat.cleave"], 0);
        assert_eq!(second.map.lines["skill.hide"], 2);
        assert_eq!(second.added, [("aaa.first".to_string(), 3)], "not the line that was dodge's");
        assert_eq!(second.retired, ["feat.dodge"]);
        assert_eq!(second.map.retired["feat.dodge"], 1);
        assert!(second.map.moved(&first.map).is_empty());
        // Named again: back on its line.
        let third =
            allocate(&names(&["feat.dodge", "aaa.first"]), &second.map, 0, |_| false).unwrap();
        assert_eq!((third.map.lines["feat.dodge"], third.restored.len()), (1, 1));
        assert!(third.added.is_empty());
        assert_eq!(Map::read(&third.map.to_json()).unwrap(), third.map);
    }

    /// A names file's keys in its own order, groups and all, whatever
    /// their names' order: new keys take lines in it.
    #[test]
    fn toml_keeps_the_file_s_order() {
        let v = toml_in_order(
            "zed = \"Z\"\n[feat.cleave]\nname = \"N\"\ndesc = { text = \"D\", sound_length = \
             1.5 }\n[aaa]\nb = 2\na = true\n",
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&v).unwrap(),
            r#"{"zed":"Z","feat":{"cleave":{"name":"N","desc":{"text":"D","sound_length":1.5}}},"aaa":{"b":2,"a":true}}"#
        );
        assert!(toml_in_order("[a\n").is_err());
    }

    #[test]
    fn lines_of_others_are_passed_over() {
        let plan =
            allocate(&names(&["a", "b"]), &Map::default(), 10, |line| line == 10 || line == 12)
                .unwrap();
        assert_eq!(plan.added, [("a".to_string(), 11), ("b".to_string(), 13)]);
    }

    #[test]
    fn keys_are_checked() {
        assert!(
            allocate(&names(&["a", "a"]), &Map::default(), 0, |_| false)
                .unwrap_err()
                .contains("twice")
        );
        assert!(allocate(&names(&["a b"]), &Map::default(), 0, |_| false).is_err());
        let bad = r#"{"version":1,"lines":{"a":1},"retired":{"b":1}}"#;
        assert!(Map::read(bad).unwrap_err().contains("line 1"));
        assert!(Map::read("{}").is_err());
    }

    #[test]
    fn a_map_moved_is_seen() {
        let old = Map::read(r#"{"version":1,"lines":{"a":0,"b":1}}"#).unwrap();
        let new = Map::read(r#"{"version":1,"lines":{"a":0,"c":1},"retired":{"b":2}}"#).unwrap();
        assert_eq!(new.moved(&old), [("b".to_string(), 1, 2)]);
    }

    #[test]
    fn constants_are_names_and_numbers() {
        let plan =
            allocate(&names(&["feat.cleave-name", "skill.hide"]), &Map::default(), 0, |_| false)
                .unwrap();
        let nss = constants_nss(&plan.map, "STR_").unwrap();
        assert!(nss.contains(
            "const int STR_FEAT_CLEAVE_NAME = 16777216;\nconst int STR_SKILL_HIDE = 16777217;\n"
        ));
        let json: serde_json::Value =
            serde_json::from_str(&constants_json(&plan.map, "").unwrap()).unwrap();
        assert_eq!(json["SKILL_HIDE"], 16_777_217);
        let clash = allocate(&names(&["a.b", "a_b"]), &Map::default(), 0, |_| false).unwrap();
        assert!(constants(&clash.map, "").unwrap_err().contains("A_B"));
    }
}
