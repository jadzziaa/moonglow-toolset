//! A plugin's settings (API 0.4): what its manifest declares
//! (`[setting]`), where each one's value is kept by its scope, and the
//! values a job reads as `ctx.settings`.
//!
//! - `user`: the user's, for every module: in Moonglow's own folder
//!   ([`USER_VALUES_FILE`], `plugin-settings.json`), which the window and `mg`
//!   share (since Moonglow 3.3.0; before, the window's settings alone).
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

/// The file of the values of plugins' settings of scope `user`, in
/// Moonglow's data folder beside its plugins.
pub const USER_VALUES_FILE: &str = "plugin-settings.json";

/// The values of settings of scope `user`, by plugin id and setting, as
/// [`USER_FILE`] keeps them (JSON values: [`to_json`]).
pub type UserValues = BTreeMap<String, BTreeMap<String, serde_json::Value>>;

/// Where the values of settings of scope `user` are kept:
/// `plugin-settings.json` in Moonglow's data folder (`$XDG_DATA_HOME/moonglow`
/// on Linux).
pub fn user_path() -> Option<std::path::PathBuf> {
    mg_module::build::data_dir().map(|d| d.join(USER_VALUES_FILE))
}

/// The values kept in `path` (none there: none).
pub fn read_user(path: &Path) -> Result<UserValues, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(UserValues::new()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Writes them to `path` (its folder made), whole or not at all
/// (`mg_module::write_replacing`).
pub fn write_user(path: &Path, values: &UserValues) -> Result<(), String> {
    let text = serde_json::to_string_pretty(values).map_err(|e| e.to_string())? + "\n";
    mg_module::write_replacing(path, text.as_bytes())
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Keeps one value of a setting of scope `user` (none: taken out, back to
/// its default) in `path`, read again first (the window, or another `mg`,
/// may have kept one since): the values now kept.
pub fn keep_user(
    path: &Path,
    plugin: &str,
    setting: &str,
    value: Option<&SettingValue>,
) -> Result<UserValues, String> {
    let mut all = read_user(path)?;
    let mine = all.entry(plugin.to_string()).or_default();
    match value {
        Some(v) => mine.insert(setting.to_string(), to_json(v)),
        None => mine.remove(setting),
    };
    if mine.is_empty() {
        all.remove(plugin);
    }
    write_user(path, &all)?;
    Ok(all)
}

/// A plugin's values of scope `user`, of those kept.
pub fn user_values(all: &UserValues, plugin: &str) -> BTreeMap<String, SettingValue> {
    (all.get(plugin).into_iter().flatten())
        .filter_map(|(k, v)| Some((k.clone(), from_json(v)?)))
        .collect()
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
        Scope::UserProject => "yours, for this project, in its .moonglow/user.toml",
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
            Some((value, at)) => {
                match value.map(|v| d.coerce(v)).and_then(|v| d.check(&v).map(|()| v)) {
                    Ok(v) => v,
                    Err(why) => {
                        said.push(format!(
                            "{at}: {id}'s setting {}: {why} (it has its default, {}, until it \
                             is set)",
                            d.id, d.default
                        ));
                        d.default.clone()
                    }
                }
            }
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
        SettingValue::Float(x) => serde_json::json!(x),
        SettingValue::List(items) => serde_json::json!(items),
    }
}

/// A value Moonglow's settings kept: none for what is no setting's.
pub fn from_json(v: &serde_json::Value) -> Option<SettingValue> {
    match v {
        serde_json::Value::String(t) => Some(SettingValue::Text(t.clone())),
        serde_json::Value::Bool(b) => Some(SettingValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            n.as_i64().map(SettingValue::Int).or_else(|| n.as_f64().map(SettingValue::Float))
        }
        serde_json::Value::Array(items) => (items.iter())
            .map(|i| i.as_str().map(str::to_string))
            .collect::<Option<Vec<String>>>()
            .map(SettingValue::List),
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
        SettingKind::Float { .. } => match text.trim().parse() {
            Ok(x) => SettingValue::Float(x),
            Err(_) => return Err(format!("{text:?} is not a number")),
        },
        // (Its texts, separated by commas.)
        SettingKind::List(_) => SettingValue::List(
            (text.split(',')).map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
        ),
        SettingKind::Color => SettingValue::Text(text.trim().to_ascii_lowercase()),
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

/// Tidies what is kept of a plugin's settings, as [`resolve`] finds it
/// wrong: a value not of its setting's kind is taken out (its default is
/// used, as it was); one of a setting the plugin doesn't declare is taken
/// out; one kept in the other place than its scope's is moved there (or,
/// where that place has one, or it is not of its kind, taken out). `user`
/// is the user's values (of scope `user`), changed in place; the project's
/// files, of the project at `root` that `file` was read from, are written
/// (with `dry_run`, nothing is). What was done, a line each; the tables
/// of plugins none of whose manifests is given are left alone (another
/// builder's plugin, not installed here).
pub fn tidy(
    manifest: &Manifest,
    user: &mut UserValues,
    project: Option<(&Path, &ProjectFile)>,
    dry_run: bool,
) -> Result<Vec<String>, String> {
    let id = &manifest.id;
    let mut done = Vec::new();
    let mut said = |text: String| done.push(text);
    // The user's own, of every module.
    if let Some(mine) = user.get_mut(id) {
        let names: Vec<String> = mine.keys().cloned().collect();
        for name in names {
            let why = match manifest.settings.iter().find(|d| d.id == name) {
                None => Some(format!("{id} has no setting {name:?}")),
                Some(d) if d.scope != Scope::User => {
                    Some(format!("{name} is of scope {}, not kept here", d.scope.name()))
                }
                Some(d) => match from_json(&mine[&name]).map(|v| d.coerce(v)) {
                    Some(v) => d.check(&v).err(),
                    None => Some(format!("{name}: not a setting's value")),
                },
            };
            if let Some(why) = why {
                mine.remove(&name);
                said(format!("Moonglow's settings: {why}: taken out"));
            }
        }
        if mine.is_empty() {
            user.remove(id);
        }
    }
    let Some((root, file)) = project else { return Ok(done) };
    let take_out = |own: bool, name: &str| -> Result<(), String> {
        if dry_run {
            return Ok(());
        }
        mg_module::project_plugins::set_setting(root, own, id, name, None)
            .map_err(|e| e.to_string())
    };
    for (own, table) in [(false, file.plugins.team.get(id)), (true, file.plugins.own.get(id))] {
        let Some(t) = table else { continue };
        let here = if own { Scope::UserProject } else { Scope::Project };
        for (name, stored) in &t.values {
            let at = &stored.at;
            match manifest.settings.iter().find(|d| d.id == *name) {
                None => {
                    take_out(own, name)?;
                    said(format!("{at}: {id} has no setting {name:?}: taken out"));
                }
                Some(d) if d.scope == here => {
                    let good = stored.value.clone().map(|v| d.coerce(v)).and_then(|v| d.check(&v));
                    if let Err(why) = good {
                        take_out(own, name)?;
                        said(format!("{at}: {name}: {why}: taken out (its default is used)"));
                    }
                }
                Some(d) => {
                    // (Moved where its scope keeps it, if it is good and
                    // nothing is kept there.)
                    let value = stored.value.clone().map(|v| d.coerce(v));
                    let value = value.and_then(|v| d.check(&v).map(|()| v));
                    let there = match d.scope {
                        Scope::User => user.get(id).is_some_and(|m| m.contains_key(name)),
                        Scope::Project => {
                            file.plugins.team.get(id).is_some_and(|t| t.values.contains_key(name))
                        }
                        Scope::UserProject => {
                            file.plugins.own.get(id).is_some_and(|t| t.values.contains_key(name))
                        }
                    };
                    take_out(own, name)?;
                    match value {
                        Ok(v) if !there => {
                            match d.scope {
                                Scope::User => {
                                    user.entry(id.clone())
                                        .or_default()
                                        .insert(name.clone(), to_json(&v));
                                }
                                _ if dry_run => {}
                                _ => keep_in_project(root, id, d, Some(&v))?,
                            }
                            said(format!(
                                "{at}: {name} is of scope {}: moved where it is kept",
                                d.scope.name()
                            ));
                        }
                        _ => said(format!(
                            "{at}: {name} is of scope {}: taken out (not kept here)",
                            d.scope.name()
                        )),
                    }
                }
            }
        }
    }
    Ok(done)
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

    /// The user's values are written whole, in a folder made for them:
    /// what was there stays under a second name it had (the new file is
    /// moved into its place, not written over it), and nothing is left
    /// beside it.
    #[test]
    fn the_user_s_values_are_written_whole() {
        let dir = mg_testkit::scratch_dir("plugin-settings-whole").join("kept");
        let path = dir.join("plugin-settings.json");
        let values = UserValues::from([(
            "t.rules".to_string(),
            BTreeMap::from([("most".to_string(), serde_json::json!(12))]),
        )]);
        write_user(&path, &values).unwrap();
        std::fs::hard_link(&path, dir.join("second")).unwrap();
        let before = std::fs::read(&path).unwrap();
        write_user(&path, &UserValues::new()).unwrap();
        assert_eq!(std::fs::read(dir.join("second")).unwrap(), before);
        assert_eq!(read_user(&path).unwrap(), UserValues::new());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
    }

    /// What is kept wrong is tidied: a value not of its kind and one of no
    /// setting taken out, one in the other file moved where its scope
    /// keeps it (or taken out where that place has one); another plugin's
    /// table left alone; the user's own taken out where wrong.
    #[test]
    fn what_is_kept_wrong_is_tidied() {
        let m = manifest();
        let dir = mg_testkit::scratch_dir("plugin-settings-tidy");
        let team = "# the team's\n[plugins.\"t.rules\"]\nmost = 40\nprefix = 3\ncode = \"ab\"\n\
                    claimz = \"x\"\n\n[plugins.\"gone.plugin\"]\nx = 1\n";
        std::fs::write(dir.join(FILE_NAME), team).unwrap();
        mg_module::project_file::ProjectFile::write_user(
            &dir,
            "[plugins.\"t.rules\"]\nprefix = \"Z_\"\n",
        )
        .unwrap();
        let mut user = UserValues::from([(
            "t.rules".to_string(),
            BTreeMap::from([
                ("strict".to_string(), serde_json::json!(3)),
                ("most".to_string(), serde_json::json!(12)),
            ]),
        )]);
        let file = ProjectFile::read(&dir);
        // With dry_run: said, nothing written.
        let mut copy = user.clone();
        let said = tidy(&m, &mut copy, Some((&dir, &file)), true).unwrap();
        assert_eq!(said.len(), 7, "{said:#?}");
        assert_eq!(std::fs::read_to_string(dir.join(FILE_NAME)).unwrap(), team);
        let said = tidy(&m, &mut user, Some((&dir, &file)), false).unwrap();
        assert!(
            said.iter()
                .any(|s| s == "moonglow.toml:6: t.rules has no setting \"claimz\": taken out")
        );
        assert!(
            said.iter()
                .any(|s| s.starts_with("moonglow.toml:5: code is of scope user-project: moved"))
        );
        assert!(said.iter().any(|s| {
            s.starts_with(".moonglow/user.toml:2: prefix is of scope project: taken out")
        }));
        assert!(user.is_empty(), "the user's wrong ones out: {user:?}");
        let file = ProjectFile::read(&dir);
        let (s, warnings) = resolve(&m, &BTreeMap::new(), Some(&file));
        assert!(warnings.is_empty(), "{warnings:#?}");
        assert_eq!(s.values["code"], SettingValue::Text("ab".into()), "moved where it is kept");
        let now = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert!(
            now.starts_with("# the team's\n") && now.contains("[plugins.\"gone.plugin\"]\nx = 1"),
            "{now}"
        );
        assert!(tidy(&m, &mut user, Some((&dir, &file)), false).unwrap().is_empty(), "tidy now");
    }

    #[test]
    fn values_go_in_and_out_as_text_and_json() {
        let m = manifest();
        let most = &m.settings[3];
        assert_eq!(parse_for(most, " 12"), Ok(SettingValue::Int(12)));
        assert!(parse_for(most, "40").unwrap_err().contains("most it takes"));
        assert!(parse_for(&m.settings[2], "maybe").is_err());
        assert_eq!(parse_for(&m.settings[2], "yes"), Ok(SettingValue::Bool(true)));
        let list = SettingValue::List(vec!["a".into(), "b c".into()]);
        for v in [
            SettingValue::Text("a".into()),
            SettingValue::Int(-3),
            SettingValue::Bool(true),
            SettingValue::Float(0.5),
            SettingValue::Float(2.0),
            list.clone(),
        ] {
            assert_eq!(from_json(&to_json(&v)), Some(v));
        }
        assert_eq!(from_json(&serde_json::json!({ "a": 1 })), None);
        assert_eq!(from_json(&serde_json::json!([1, 2])), None, "a list is of texts");
        assert_eq!(list.to_string(), "[\"a\", \"b c\"]");
        assert_eq!(SettingValue::Float(2.0).to_string(), "2.0");
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
