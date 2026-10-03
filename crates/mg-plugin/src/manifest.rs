//! A plugin's manifest, `plugin.cfg`: who it is and what it adds, read
//! without running any of its code. It is written as `nasher.cfg` is
//! (sections, `key = "value"`), a `[command]` or `[check]` section for
//! each thing added.

use mg_module::doctor::Severity;
use mg_module::nasher::{Section, sections};

/// The plugin API this Moonglow serves (a manifest's `api`).
pub const API: &str = "0.1";

/// What a manifest says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Its id, the same wherever it is installed (`author.plugin-name`).
    pub id: String,
    pub name: String,
    pub version: String,
    /// The plugin API it was written for.
    pub api: String,
    /// Its license (an SPDX name).
    pub license: String,
    pub description: String,
    pub authors: Vec<String>,
    /// The script that registers its commands and checks.
    pub entry: String,
    pub commands: Vec<CommandDecl>,
    pub checks: Vec<CheckDecl>,
}

/// A command a plugin adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDecl {
    pub id: String,
    /// Its name in the menu.
    pub title: String,
    /// What its tip says.
    pub hint: String,
    /// The key it has until the user chooses another (`Ctrl+Alt+T`).
    pub key: Option<String>,
}

/// A check a plugin adds to Verify Module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckDecl {
    pub id: String,
    /// What it holds to be true.
    pub title: String,
    /// How bad its findings are, unless one says otherwise.
    pub severity: Severity,
}

/// Whether text is an id: lower-case letters, digits, `.`, `-` and `_`,
/// starting with a letter or digit.
fn is_id(text: &str) -> bool {
    let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_');
    (1..=64).contains(&text.len())
        && text.chars().all(ok)
        && text.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// A section's keys read into named values; a key not expected is an
/// error (a misspelled one, most likely).
fn keys<'a>(s: &'a Section, known: &[&str]) -> Result<impl Fn(&str) -> &'a str, String> {
    if let Some((k, _)) = s.pairs.iter().find(|(k, _)| !known.contains(&k.as_str())) {
        return Err(format!(
            "line {}: [{}] has no key {k:?} (it has {})",
            s.line,
            s.name,
            known.join(", ")
        ));
    }
    Ok(move |key: &str| s.get(key).unwrap_or_default().trim())
}

fn required<'a>(s: &Section, key: &str, value: &'a str) -> Result<&'a str, String> {
    if value.is_empty() {
        return Err(format!("line {}: [{}] needs {key}", s.line, s.name));
    }
    Ok(value)
}

fn id_of(s: &Section, value: &str) -> Result<String, String> {
    let value = required(s, "id", value)?;
    if !is_id(value) {
        return Err(format!(
            "line {}: the id {value:?} must be lower-case letters, digits, '.', '-' or '_'",
            s.line
        ));
    }
    Ok(value.to_string())
}

impl Manifest {
    /// Reads a manifest. Everything it needs must be there, and nothing it
    /// does not know: a fault names the line.
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let all = sections(text)?;
        let mut plugin: Option<Manifest> = None;
        let (mut commands, mut checks) = (Vec::<CommandDecl>::new(), Vec::<CheckDecl>::new());
        for s in &all {
            match s.name.as_str() {
                "plugin" => {
                    if plugin.is_some() {
                        return Err(format!("line {}: a second [plugin] section", s.line));
                    }
                    let get = keys(
                        s,
                        &[
                            "id",
                            "name",
                            "version",
                            "api",
                            "license",
                            "description",
                            "author",
                            "entry",
                        ],
                    )?;
                    let api = required(s, "api", get("api"))?;
                    if api != API {
                        return Err(format!(
                            "it was written for plugin API {api}; this Moonglow has {API}"
                        ));
                    }
                    let entry = match get("entry") {
                        "" => "main.luau",
                        e => e,
                    };
                    let plain = !entry.starts_with('/')
                        && !entry.contains('\\')
                        && !entry.contains(':')
                        && entry.split('/').all(|p| !p.is_empty() && p != ".." && p != ".");
                    if !plain {
                        return Err(format!(
                            "line {}: entry must be a file of the plugin's folder, not {entry:?}",
                            s.line
                        ));
                    }
                    plugin = Some(Manifest {
                        id: id_of(s, get("id"))?,
                        name: required(s, "name", get("name"))?.to_string(),
                        version: required(s, "version", get("version"))?.to_string(),
                        api: api.to_string(),
                        license: required(s, "license", get("license"))?.to_string(),
                        description: get("description").to_string(),
                        authors: s
                            .pairs
                            .iter()
                            .filter(|(k, _)| k == "author")
                            .map(|(_, v)| v.trim().to_string())
                            .collect(),
                        entry: entry.to_string(),
                        commands: Vec::new(),
                        checks: Vec::new(),
                    });
                }
                "command" => {
                    let get = keys(s, &["id", "title", "hint", "key"])?;
                    let id = id_of(s, get("id"))?;
                    if commands.iter().any(|c| c.id == id) {
                        return Err(format!("line {}: a second command {id:?}", s.line));
                    }
                    commands.push(CommandDecl {
                        id,
                        title: required(s, "title", get("title"))?.to_string(),
                        hint: get("hint").to_string(),
                        key: Some(get("key")).filter(|k| !k.is_empty()).map(str::to_string),
                    });
                }
                "check" => {
                    let get = keys(s, &["id", "title", "severity"])?;
                    let id = id_of(s, get("id"))?;
                    if checks.iter().any(|c| c.id == id) {
                        return Err(format!("line {}: a second check {id:?}", s.line));
                    }
                    let severity = match get("severity") {
                        "" | "warning" => Severity::Warning,
                        "error" => Severity::Error,
                        other => {
                            return Err(format!(
                                "line {}: severity is \"warning\" or \"error\", not {other:?}",
                                s.line
                            ));
                        }
                    };
                    checks.push(CheckDecl {
                        id,
                        title: required(s, "title", get("title"))?.to_string(),
                        severity,
                    });
                }
                other => {
                    return Err(format!(
                        "line {}: no section [{other}] (there are [plugin], [command] and [check])",
                        s.line
                    ));
                }
            }
        }
        let mut plugin = plugin.ok_or("it has no [plugin] section")?;
        plugin.commands = commands;
        plugin.checks = checks;
        Ok(plugin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
; A sample.
[plugin]
id = "example.tag-conventions"
name = "Tag conventions"
version = "1.0.0"
api = "0.1"
license = "GPL-3.0-or-later"
description = "Checks tags against the server's rules, and fixes them."
author = "A. Builder"
author = "B. Builder"

[command]
id = "fix-tags"
title = "Fix Creature Tags"
hint = "Upper-case every creature's tag"
key = "Ctrl+Alt+T"

[check]
id = "tag-case"
title = "Creature tags are upper case"

[check]
id = "tag-length"
title = "Tags are at most 32 characters"
severity = "error"
"#;

    #[test]
    fn a_manifest_says_what_a_plugin_adds() {
        let m = Manifest::parse(SAMPLE).unwrap();
        assert_eq!((m.id.as_str(), m.entry.as_str()), ("example.tag-conventions", "main.luau"));
        assert_eq!(m.authors, ["A. Builder", "B. Builder"]);
        assert_eq!(m.commands.len(), 1);
        assert_eq!(m.commands[0].key.as_deref(), Some("Ctrl+Alt+T"));
        assert_eq!(m.commands[0].hint, "Upper-case every creature's tag");
        let severities: Vec<Severity> = m.checks.iter().map(|c| c.severity).collect();
        assert_eq!(severities, [Severity::Warning, Severity::Error]);
    }

    #[test]
    fn faults_name_the_line() {
        let with = |from: &str, to: &str| Manifest::parse(&SAMPLE.replace(from, to)).unwrap_err();
        assert_eq!(Manifest::parse("").unwrap_err(), "it has no [plugin] section");
        assert!(with("api = \"0.1\"", "api = \"7.0\"").contains("written for plugin API 7.0"));
        assert!(with("license = \"GPL-3.0-or-later\"\n", "").contains("[plugin] needs license"));
        assert!(with("example.tag-conventions", "Example Tags").contains("lower-case letters"));
        assert!(with("hint =", "hnit =").contains("[command] has no key \"hnit\""));
        assert!(
            with("[check]\nid = \"tag-length\"", "[check]\nid = \"tag-case\"")
                .contains("a second check \"tag-case\"")
        );
        assert!(with("severity = \"error\"", "severity = \"fatal\"").contains("not \"fatal\""));
        assert!(with("[command]", "[panel]").contains("no section [panel]"));
        for entry in ["../other.luau", "/etc/passwd", "a\\b.luau", "c:x.luau", "./main.luau"] {
            let text =
                SAMPLE.replace("api = \"0.1\"", &format!("api = \"0.1\"\nentry = {entry:?}"));
            assert!(Manifest::parse(&text).unwrap_err().contains("entry must be"), "{entry}");
        }
    }
}
