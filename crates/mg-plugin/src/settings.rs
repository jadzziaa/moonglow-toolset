//! A plugin's settings (API 0.4): what its manifest declares
//! (`[setting]`), where each one's value is kept by its scope, and the
//! values a job reads as `ctx.settings`.
//!
//! - `user`: the user's, for every module. Kept by the host: in the
//!   window, Moonglow's settings; `mg` keeps none (a run takes the
//!   defaults, and `--setting`).
//! - `project`: the team's, in the project's `moonglow.toml`
//!   (`[plugins."<id>"]`), committed with it.
//! - `user-project`: the user's own for the project, in its
//!   `.moonglow/user.toml` (`[plugins."<id>"]`), never committed.
//!
//! A value kept that is not of its setting's kind is passed over for the
//! default, and said; one for a setting the plugin doesn't declare is left
//! where it is, and said. Nothing here rewrites what it did not set.

use std::collections::BTreeMap;
use std::path::Path;

use mg_module::project_file::{FILE_NAME, ProjectFile, USER_DIR, USER_FILE};
use mg_module::project_plugins::Values;

use crate::{Manifest, Scope, SettingDecl, SettingKind, SettingValue};

/// What a job reads of its plugin's settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    /// Each declared setting's value: the one kept, where it is good,
    /// else its default.
    pub values: BTreeMap<String, SettingValue>,
    /// Whether a project is open, whose files keep the values of the
    /// settings of scope `project` and `user-project`: without one,
    /// those read their defaults and can't be set.
    pub project: bool,
}

/// What `ctx.settings:set` asked to keep, in order: a setting's new value,
/// or none (`set(name, nil)`): back to its default, the value kept taken
/// out. The host keeps them when the job is done, if it did not fail.
pub type SettingWrite = (String, Option<SettingValue>);

/// Where a scope's values are kept, as the user is told.
pub fn scope_words(scope: Scope) -> &'static str {
    match scope {
        Scope::User => "yours, in Moonglow's settings",
        Scope::Project => "the team's, in the project's moonglow.toml",
        Scope::UserProject => "yours, for this project (.moonglow/user.toml)",
    }
}

/// The values of a plugin's settings for a job: those kept (in the
/// user's store, `user`, and in the project's files, `project`) where
/// they are good, else the defaults; and what is wrong with those kept.
pub fn resolve(
    manifest: &Manifest,
    user: &BTreeMap<String, SettingValue>,
    project: Option<&ProjectFile>,
) -> (Settings, Vec<String>) {
    let mut s = Settings { values: BTreeMap::new(), project: project.is_some() };
    let mut said = Vec::new();
    let id = &manifest.id;
    let team = project.and_then(|p| p.plugins.team.get(id));
    let own = project.and_then(|p| p.plugins.own.get(id));
    for d in &manifest.settings {
        let kept = match d.scope {
            Scope::User => user.get(&d.id).map(|v| (Ok(v.clone()), "Moonglow's settings".into())),
            Scope::Project => {
                team.and_then(|t| t.values.get(&d.id)).map(|v| (v.value.clone(), v.at.clone()))
            }
            Scope::UserProject => {
                own.and_then(|t| t.values.get(&d.id)).map(|v| (v.value.clone(), v.at.clone()))
            }
        };
        let value = match kept {
            None => d.default.clone(),
            Some((value, at)) => match value.and_then(|v| d.check(&v).map(|()| v)) {
                Ok(v) => v,
                Err(why) => {
                    said.push(format!(
                        "{at}: {id}'s setting {}: {why} (it has its default, {}, until it \
                             is set)",
                        d.id, d.default
                    ));
                    d.default.clone()
                }
            },
        };
        s.values.insert(d.id.clone(), value);
    }
    // What is kept and not declared, or kept in the other file.
    let elsewhere = |table: Option<&Values>, here: Scope, said: &mut Vec<String>| {
        let Some(t) = table else { return };
        for (name, v) in &t.values {
            match manifest.settings.iter().find(|d| d.id == *name) {
                None => said.push(format!(
                    "{}: {id} has no setting {name:?} (the value is left there, and not used)",
                    v.at
                )),
                Some(d) if d.scope != here => {
                    let goes = match d.scope {
                        Scope::User => "in Moonglow's settings (Plugins › Settings…)".to_string(),
                        Scope::Project => format!("in {FILE_NAME}"),
                        Scope::UserProject => format!("in {USER_DIR}/{USER_FILE}"),
                    };
                    said.push(format!(
                        "{}: {id}'s setting {name} is of scope {}: it is kept {goes}, not here \
                         (not used)",
                        v.at,
                        d.scope.name()
                    ));
                }
                Some(_) => {}
            }
        }
    };
    elsewhere(team, Scope::Project, &mut said);
    elsewhere(own, Scope::UserProject, &mut said);
    (s, said)
}

/// The tables of the project's files for plugins that are none of
/// `known` (ids): said, and left as they are.
pub fn unclaimed(project: &ProjectFile, known: &[&str]) -> Vec<String> {
    let p = &project.plugins;
    (p.team.iter().chain(&p.own))
        .filter(|(id, _)| !known.contains(&id.as_str()))
        .map(|(id, t)| {
            format!(
                "{}: [plugins.\"{id}\"]: there is no plugin {id} (installed, or the project's): \
                 its values are left there",
                t.at
            )
        })
        .collect()
}

/// A value as Moonglow's settings keep it (JSON).
pub fn to_json(v: &SettingValue) -> serde_json::Value {
    match v {
        SettingValue::Text(t) => serde_json::json!(t),
        SettingValue::Int(n) => serde_json::json!(n),
        SettingValue::Bool(b) => serde_json::json!(b),
    }
}

/// A value Moonglow's settings kept: none for what is no setting's.
pub fn from_json(v: &serde_json::Value) -> Option<SettingValue> {
    match v {
        serde_json::Value::String(t) => Some(SettingValue::Text(t.clone())),
        serde_json::Value::Bool(b) => Some(SettingValue::Bool(*b)),
        serde_json::Value::Number(n) => n.as_i64().map(SettingValue::Int),
        _ => None,
    }
}

/// A value given as text (`mg plugin run --setting NAME=VALUE`, a form's
/// field): read as its setting's kind, and held to it.
pub fn parse_for(decl: &SettingDecl, text: &str) -> Result<SettingValue, String> {
    let v = match &decl.kind {
        SettingKind::Int { .. } => match text.trim().parse() {
            Ok(n) => SettingValue::Int(n),
            Err(_) => return Err(format!("{text:?} is not a whole number")),
        },
        SettingKind::Bool => match text.trim() {
            "true" | "yes" | "1" => SettingValue::Bool(true),
            "false" | "no" | "0" => SettingValue::Bool(false),
            other => return Err(format!("{other:?} is not true or false")),
        },
        _ => SettingValue::Text(text.to_string()),
    };
    decl.check(&v)?;
    Ok(v)
}

/// Keeps a setting's value (none: taken out, back to its default) in the
/// project whose root is `root`: in the team's file for scope `project`,
/// the user's own for `user-project`. A setting of scope `user` is the
/// host's to keep.
pub fn keep_in_project(
    root: &Path,
    plugin: &str,
    decl: &SettingDecl,
    value: Option<&SettingValue>,
) -> Result<(), String> {
    let own = match decl.scope {
        Scope::User => return Err(format!("{} is kept in Moonglow's settings", decl.id)),
        Scope::Project => false,
        Scope::UserProject => true,
    };
    mg_module::project_plugins::set_setting(root, own, plugin, &decl.id, value)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest::parse(
            r#"
[plugin]
id = "t.rules"
name = "Rules"
version = "1.0.0"
api = "0.4"
license = "MIT"

[setting]
id = "prefix"
scope = "project"
default = "TDOD_"

[setting]
id = "code"
scope = "user-project"

[setting]
id = "strict"
type = "bool"

[setting]
id = "most"
type = "int"
max = 32
default = 32
scope = "project"
"#,
        )
        .unwrap()
    }

    fn project(team: &str, own: &str) -> ProjectFile {
        let mut p = ProjectFile::default();
        p.apply(FILE_NAME, team, false);
        p.apply(".moonglow/user.toml", own, true);
        assert_eq!(p.warnings, Vec::<String>::new());
        p
    }

    #[test]
    fn each_value_comes_from_its_scope_s_place_or_the_default() {
        let m = manifest();
        let user = BTreeMap::from([("strict".to_string(), SettingValue::Bool(true))]);
        let p = project(
            "[plugins.\"t.rules\"]\nprefix = \"KRZ_\"\n",
            "[plugins.\"t.rules\"]\ncode = \"ab\"\n",
        );
        let (s, said) = resolve(&m, &user, Some(&p));
        assert_eq!(said, Vec::<String>::new());
        assert!(s.project);
        let text = |t: &str| SettingValue::Text(t.into());
        assert_eq!(s.values["prefix"], text("KRZ_"));
        assert_eq!(s.values["code"], text("ab"));
        assert_eq!(s.values["strict"], SettingValue::Bool(true));
        assert_eq!(s.values["most"], SettingValue::Int(32), "the default");
        // No project: those of the project's scopes are their defaults.
        let (s, _) = resolve(&m, &BTreeMap::new(), None);
        assert!(!s.project);
        assert_eq!((&s.values["prefix"], &s.values["code"]), (&text("TDOD_"), &text("")));
    }

    #[test]
    fn what_is_kept_wrong_is_said_and_left_where_it_is() {
        let m = manifest();
        let user = BTreeMap::from([("strict".to_string(), SettingValue::Int(3))]);
        let p = project(
            "[plugins.\"t.rules\"]\nmost = 40\nprefix = 3\ncode = \"ab\"\nclaimz = \"x\"\n\n\
             [plugins.\"gone.plugin\"]\nx = 1\n",
            "[plugins.\"t.rules\"]\nprefix = \"Z_\"\n",
        );
        let (s, said) = resolve(&m, &user, Some(&p));
        assert_eq!(
            said,
            [
                "moonglow.toml:3: t.rules's setting prefix: 3 is not text (it has its default, \
                 \"TDOD_\", until it is set)",
                "Moonglow's settings: t.rules's setting strict: 3 is not true or false (it has \
                 its default, false, until it is set)",
                "moonglow.toml:2: t.rules's setting most: 40 is more than 32, the most it takes \
                 (it has its default, 32, until it is set)",
                "moonglow.toml:5: t.rules has no setting \"claimz\" (the value is left there, \
                 and not used)",
                "moonglow.toml:4: t.rules's setting code is of scope user-project: it is kept \
                 in .moonglow/user.toml, not here (not used)",
                ".moonglow/user.toml:2: t.rules's setting prefix is of scope project: it is \
                 kept in moonglow.toml, not here (not used)",
            ]
        );
        assert_eq!(s.values["code"], SettingValue::Text(String::new()), "not from the team's");
        assert_eq!(
            unclaimed(&p, &["t.rules"]),
            ["moonglow.toml:7: [plugins.\"gone.plugin\"]: there is no plugin gone.plugin \
                 (installed, or the project's): its values are left there"]
        );
    }

    #[test]
    fn values_go_in_and_out_as_text_and_json() {
        let m = manifest();
        let most = &m.settings[3];
        assert_eq!(parse_for(most, " 12"), Ok(SettingValue::Int(12)));
        assert!(parse_for(most, "40").unwrap_err().contains("most it takes"));
        assert!(parse_for(&m.settings[2], "maybe").is_err());
        assert_eq!(parse_for(&m.settings[2], "yes"), Ok(SettingValue::Bool(true)));
        for v in [SettingValue::Text("a".into()), SettingValue::Int(-3), SettingValue::Bool(true)] {
            assert_eq!(from_json(&to_json(&v)), Some(v));
        }
        assert_eq!(from_json(&serde_json::json!(0.5)), None);
    }

    #[test]
    fn a_project_s_values_are_kept_in_its_files() {
        let m = manifest();
        let dir = mg_testkit::scratch_dir("plugin-settings-keep");
        keep_in_project(&dir, "t.rules", &m.settings[0], Some(&SettingValue::Text("Q_".into())))
            .unwrap();
        keep_in_project(&dir, "t.rules", &m.settings[1], Some(&SettingValue::Text("cd".into())))
            .unwrap();
        assert!(keep_in_project(&dir, "t.rules", &m.settings[2], None).is_err(), "the host's");
        let p = ProjectFile::read(&dir);
        let (s, said) = resolve(&m, &BTreeMap::new(), Some(&p));
        assert_eq!(said, Vec::<String>::new());
        assert_eq!(s.values["prefix"], SettingValue::Text("Q_".into()));
        assert_eq!(s.values["code"], SettingValue::Text("cd".into()));
        keep_in_project(&dir, "t.rules", &m.settings[0], None).unwrap();
        let p = ProjectFile::read(&dir);
        assert_eq!(
            resolve(&m, &BTreeMap::new(), Some(&p)).0.values["prefix"],
            m.settings[0].default
        );
    }
}
