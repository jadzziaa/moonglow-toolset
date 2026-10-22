//! A plugin's manifest, `plugin.cfg`: who it is and what it adds, read
//! without running any of its code. It is written as `nasher.cfg` is
//! (sections, `key = "value"`), a `[command]` or `[check]` section for
//! each thing added.

use mg_module::doctor::Severity;
use mg_module::nasher::{Section, sections};
pub use mg_module::project_plugins::SettingValue;
use mg_rules::ObjectKind;

/// The plugin API this Moonglow serves (a manifest's `api`).
pub const API: &str = "0.4";
/// The APIs a plugin may be written for: each later one has all of the
/// ones before.
pub const APIS: [&str; 4] = ["0.1", "0.2", "0.3", "0.4"];

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
    /// What it does by itself, when something happens.
    pub events: Vec<EventDecl>,
    /// The values it keeps (API 0.4), which the user can set in the
    /// Plugins window and it reads as `ctx.settings`.
    pub settings: Vec<SettingDecl>,
}

/// A setting a plugin keeps: a value of one kind, kept where its scope
/// says, read by the plugin as `ctx.settings.<id>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingDecl {
    /// Its name, as the plugin reads it (`ctx.settings.code`): lower-case
    /// letters, digits and `_`, from a letter.
    pub id: String,
    pub kind: SettingKind,
    /// Its value until one is set.
    pub default: SettingValue,
    /// What the settings' form calls it (else its id).
    pub label: String,
    /// What the form's tip says of it.
    pub help: String,
    pub scope: Scope,
}

/// What a setting holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingKind {
    /// Text.
    Text,
    /// A whole number, within its limits if it has them.
    Int { min: Option<i64>, max: Option<i64> },
    /// True or false.
    Bool,
    /// One of these texts.
    Choice(Vec<String>),
    /// A file's or a folder's path, as text: the form has a button to
    /// choose it. The plugin reads no file by it (the sandbox).
    Path,
}

impl SettingKind {
    /// Its name in a manifest (`type = "int"`).
    pub fn name(&self) -> &'static str {
        match self {
            SettingKind::Text => "string",
            SettingKind::Int { .. } => "int",
            SettingKind::Bool => "bool",
            SettingKind::Choice(_) => "choice",
            SettingKind::Path => "path",
        }
    }
}

/// Where a setting's value is kept, and who shares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    /// The user's, for every module: in Moonglow's settings.
    User,
    /// The team's, for the project: in its `moonglow.toml`
    /// (`[plugins."<id>"]`), which is committed with it.
    Project,
    /// The user's own, for the project: in its `.moonglow/user.toml`
    /// (`[plugins."<id>"]`), which is never committed.
    UserProject,
}

impl Scope {
    pub const ALL: [Scope; 3] = [Scope::User, Scope::Project, Scope::UserProject];

    /// Its name in a manifest (`scope = "user-project"`).
    pub fn name(self) -> &'static str {
        match self {
            Scope::User => "user",
            Scope::Project => "project",
            Scope::UserProject => "user-project",
        }
    }

    pub fn named(name: &str) -> Option<Scope> {
        Scope::ALL.into_iter().find(|s| s.name() == name)
    }

    /// Whether its value is kept in the project's files.
    pub fn in_project(self) -> bool {
        self != Scope::User
    }
}

impl SettingDecl {
    /// What its form calls it.
    pub fn shown(&self) -> &str {
        if self.label.is_empty() { &self.id } else { &self.label }
    }

    /// A value held to its kind: why it is not one, if it is not.
    pub fn check(&self, value: &SettingValue) -> Result<(), String> {
        match (&self.kind, value) {
            (SettingKind::Text | SettingKind::Path, SettingValue::Text(_)) => Ok(()),
            (SettingKind::Bool, SettingValue::Bool(_)) => Ok(()),
            (SettingKind::Int { min, max }, SettingValue::Int(n)) => {
                if let Some(m) = min.filter(|m| n < m) {
                    return Err(format!("{n} is less than {m}, the least it takes"));
                }
                if let Some(m) = max.filter(|m| n > m) {
                    return Err(format!("{n} is more than {m}, the most it takes"));
                }
                Ok(())
            }
            (SettingKind::Choice(choices), SettingValue::Text(t)) => {
                if choices.contains(t) {
                    Ok(())
                } else {
                    Err(format!("{t:?} is not one of {}", choices.join(", ")))
                }
            }
            (kind, v) => Err(format!("{v} is not {}", kind_words(kind))),
        }
    }
}

/// A kind as a value of it is said to be (`a whole number`).
fn kind_words(kind: &SettingKind) -> &'static str {
    match kind {
        SettingKind::Text => "text",
        SettingKind::Path => "a path (text)",
        SettingKind::Int { .. } => "a whole number",
        SettingKind::Bool => "true or false",
        SettingKind::Choice(_) => "one of its choices (text)",
    }
}

/// Whether text is a setting's id: what `ctx.settings.<id>` can name.
fn is_setting_id(text: &str) -> bool {
    (1..=64).contains(&text.len())
        && text.starts_with(|c: char| c.is_ascii_lowercase())
        && text.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A `[setting]` section read.
fn setting_of(s: &Section) -> Result<SettingDecl, String> {
    let get =
        keys(s, &["id", "type", "default", "label", "help", "scope", "choices", "min", "max"])?;
    let id = required(s, "id", get("id"))?;
    if !is_setting_id(id) {
        return Err(format!(
            "line {}: a setting's id {id:?} is lower-case letters, digits and '_', from a letter \
             (the plugin reads it as ctx.settings.{id})",
            s.line
        ));
    }
    if id == "set" {
        return Err(format!(
            "line {}: a setting can't be called \"set\" (ctx.settings:set sets one)",
            s.line
        ));
    }
    let int = |key: &str| -> Result<Option<i64>, String> {
        match get(key) {
            "" => Ok(None),
            n => n.parse().map(Some).map_err(|_| {
                format!("line {}: the setting {id:?}: {key} is a whole number, not {n:?}", s.line)
            }),
        }
    };
    let kind = match get("type") {
        "" | "string" => SettingKind::Text,
        "int" => SettingKind::Int { min: int("min")?, max: int("max")? },
        "bool" => SettingKind::Bool,
        "path" => SettingKind::Path,
        "choice" => {
            let choices: Vec<String> = (get("choices").split(','))
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .collect();
            if choices.is_empty() {
                return Err(format!(
                    "line {}: the setting {id:?} is a choice: it needs choices = \"a, b\"",
                    s.line
                ));
            }
            SettingKind::Choice(choices)
        }
        other => {
            return Err(format!(
                "line {}: the setting {id:?}: type is string, int, bool, choice or path, not \
                 {other:?}",
                s.line
            ));
        }
    };
    for key in ["choices", "min", "max"] {
        let fits = match key {
            "choices" => matches!(kind, SettingKind::Choice(_)),
            _ => matches!(kind, SettingKind::Int { .. }),
        };
        if !get(key).is_empty() && !fits {
            return Err(format!(
                "line {}: the setting {id:?} is {}: it has no {key}",
                s.line,
                kind.name()
            ));
        }
    }
    let given = get("default");
    let default = match (&kind, given) {
        (SettingKind::Text | SettingKind::Path, g) => SettingValue::Text(g.to_string()),
        (SettingKind::Bool, "" | "false") => SettingValue::Bool(false),
        (SettingKind::Bool, "true") => SettingValue::Bool(true),
        (SettingKind::Bool, other) => {
            return Err(format!(
                "line {}: the setting {id:?}: its default is true or false, not {other:?}",
                s.line
            ));
        }
        (SettingKind::Int { min, .. }, "") => SettingValue::Int(min.unwrap_or(0).max(0)),
        (SettingKind::Int { .. }, n) => SettingValue::Int(n.parse().map_err(|_| {
            format!("line {}: the setting {id:?}: its default is a whole number, not {n:?}", s.line)
        })?),
        (SettingKind::Choice(c), "") => SettingValue::Text(c[0].clone()),
        (SettingKind::Choice(_), g) => SettingValue::Text(g.to_string()),
    };
    let scope = match get("scope") {
        "" => Scope::User,
        name => Scope::named(name).ok_or_else(|| {
            format!(
                "line {}: the setting {id:?}: scope is \"user\", \"project\" or \
                 \"user-project\", not {name:?}",
                s.line
            )
        })?,
    };
    let decl = SettingDecl {
        id: id.to_string(),
        kind,
        default,
        label: get("label").to_string(),
        help: get("help").to_string(),
        scope,
    };
    if let Err(why) = decl.check(&decl.default) {
        return Err(format!("line {}: the setting {id:?}: its default: {why}", s.line));
    }
    Ok(decl)
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
    /// What it is for (`for = "placeable, creature"`): it is in the
    /// right-click menus of each, and gets what was clicked as its
    /// selection. None: the Plugins menu alone (API 0.4).
    pub targets: Vec<Target>,
}

/// What a command can be for: a kind of object (placed in an area, or its
/// blueprint) or an area. Their right-click menus have the command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Target {
    /// Objects of this kind placed in an area, and their blueprints.
    Object(ObjectKind),
    Area,
}

impl Target {
    /// Every one, in the order the manual lists them.
    pub fn all() -> impl Iterator<Item = Target> {
        ObjectKind::ALL.into_iter().map(Target::Object).chain([Target::Area])
    }

    /// Its name in a manifest: `creature`, `placeable`, `area`.
    pub fn name(self) -> &'static str {
        match self {
            Target::Object(kind) => kind.name(),
            Target::Area => "area",
        }
    }

    pub fn named(name: &str) -> Option<Target> {
        Target::all().find(|t| t.name() == name)
    }
}

/// Whether `api` is a version before `than` (both among [`APIS`]).
fn api_before(api: &str, than: &str) -> bool {
    let at = |v: &str| APIS.iter().position(|a| *a == v);
    at(api) < at(than)
}

/// A command's `for`: kinds of object, or `area`, each named once, given
/// as a list (`"placeable, creature"`) or a key to each.
fn targets_of(s: &Section) -> Result<Vec<Target>, String> {
    let mut targets = Vec::new();
    let given = s.pairs.iter().filter(|(k, _)| k == "for");
    for name in given.flat_map(|(_, v)| v.split(',')).map(str::trim) {
        let Some(t) = Target::named(name) else {
            let names: Vec<&str> = Target::all().map(Target::name).collect();
            return Err(format!(
                "line {}: a command is for {}, not {name:?}",
                s.line,
                names.join(", ")
            ));
        };
        if targets.contains(&t) {
            return Err(format!("line {}: the command is for {name} twice", s.line));
        }
        targets.push(t);
    }
    Ok(targets)
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

/// What a plugin can act on by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A module was opened.
    ModuleOpened,
    /// The module was saved: with the resources that are not as they
    /// were when it was last saved, or opened.
    ModuleSaved,
}

impl EventKind {
    pub fn name(self) -> &'static str {
        match self {
            EventKind::ModuleOpened => "module-opened",
            EventKind::ModuleSaved => "module-saved",
        }
    }

    pub fn named(name: &str) -> Option<EventKind> {
        [EventKind::ModuleOpened, EventKind::ModuleSaved].into_iter().find(|k| k.name() == name)
    }
}

/// Something a plugin does when an event happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDecl {
    pub id: String,
    pub on: EventKind,
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
        let mut events = Vec::<EventDecl>::new();
        let mut settings = Vec::<SettingDecl>::new();
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
                    if !APIS.contains(&api) {
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
                        events: Vec::new(),
                        settings: Vec::new(),
                    });
                }
                "command" => {
                    let get = keys(s, &["id", "title", "hint", "key", "for"])?;
                    let id = id_of(s, get("id"))?;
                    if commands.iter().any(|c| c.id == id) {
                        return Err(format!("line {}: a second command {id:?}", s.line));
                    }
                    let targets = targets_of(s)?;
                    commands.push(CommandDecl {
                        id,
                        title: required(s, "title", get("title"))?.to_string(),
                        hint: get("hint").to_string(),
                        key: Some(get("key")).filter(|k| !k.is_empty()).map(str::to_string),
                        targets,
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
                "event" => {
                    let get = keys(s, &["id", "on"])?;
                    let id = id_of(s, get("id"))?;
                    if events.iter().any(|e: &EventDecl| e.id == id) {
                        return Err(format!("line {}: a second event {id:?}", s.line));
                    }
                    let Some(on) = EventKind::named(required(s, "on", get("on"))?) else {
                        return Err(format!(
                            "line {}: on is \"module-opened\" or \"module-saved\", not {:?}",
                            s.line,
                            get("on")
                        ));
                    };
                    events.push(EventDecl { id, on });
                }
                "setting" => {
                    let decl = setting_of(s)?;
                    if settings.iter().any(|d| d.id == decl.id) {
                        return Err(format!("line {}: a second setting {:?}", s.line, decl.id));
                    }
                    settings.push(decl);
                }
                other => {
                    return Err(format!(
                        "line {}: no section [{other}] (there are [plugin], [command], [check], \
                         [event] and [setting])",
                        s.line
                    ));
                }
            }
        }
        let mut plugin = plugin.ok_or("it has no [plugin] section")?;
        // (What a later API added, in a manifest that names an earlier
        // one: said plainly, as a Moonglow without it would refuse it.)
        if let Some(c) = commands.iter().find(|c| !c.targets.is_empty())
            && api_before(&plugin.api, "0.4")
        {
            return Err(format!(
                "the command {:?} says what it is for (`for`), which is plugin API 0.4; \
                 the manifest says api = {:?}",
                c.id, plugin.api
            ));
        }
        if let Some(d) = settings.first()
            && api_before(&plugin.api, "0.4")
        {
            return Err(format!(
                "the setting {:?} ([setting]) is plugin API 0.4; the manifest says api = {:?}",
                d.id, plugin.api
            ));
        }
        plugin.commands = commands;
        plugin.checks = checks;
        plugin.events = events;
        plugin.settings = settings;
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

    /// API 0.4: a command says what it is for, as a list or a key to
    /// each; a command without says nothing, as before.
    #[test]
    fn a_command_says_what_it_is_for() {
        let text = SAMPLE.replace("api = \"0.1\"", "api = \"0.4\"").replace(
            "key = \"Ctrl+Alt+T\"",
            "key = \"Ctrl+Alt+T\"\nfor = \"placeable, creature\"\nfor = \"area\"",
        );
        let m = Manifest::parse(&text).unwrap();
        assert_eq!(
            m.commands[0].targets,
            [
                Target::Object(ObjectKind::Placeable),
                Target::Object(ObjectKind::Creature),
                Target::Area
            ]
        );
        assert!(Manifest::parse(SAMPLE).unwrap().commands[0].targets.is_empty());
        let names: Vec<&str> = Target::all().map(Target::name).collect();
        assert_eq!(
            names,
            [
                "creature",
                "door",
                "encounter",
                "item",
                "placeable",
                "sound",
                "store",
                "trigger",
                "waypoint",
                "area"
            ]
        );
        // Faults: a kind there is none of, one named twice, and `for` in
        // a manifest written for an API before it.
        let with = |to: &str| Manifest::parse(&text.replace("for = \"area\"", to)).unwrap_err();
        let e = with("for = \"chest\"");
        assert!(e.contains("is for creature, door,") && e.contains("not \"chest\""), "{e}");
        assert!(with("for = \"creature\"").contains("for creature twice"));
        let e = Manifest::parse(&text.replace("api = \"0.4\"", "api = \"0.3\"")).unwrap_err();
        assert!(e.contains("plugin API 0.4") && e.contains("\"fix-tags\""), "{e}");
    }

    /// API 0.4: settings, each of a kind, with a default and a scope.
    #[test]
    fn a_plugin_declares_its_settings() {
        let text = SAMPLE.replace("api = \"0.1\"", "api = \"0.4\"")
            + r#"
[setting]
id = "prefix"
label = "Team prefix"
scope = "project"
default = "TDOD_"
help = "What tags begin with"

[setting]
id = "max_length"
type = "int"
min = 1
max = 32
default = 32

[setting]
id = "strict"
type = "bool"

[setting]
id = "kinds"
type = "choice"
choices = "both, creatures , placeables"
scope = "user-project"

[setting]
id = "claims"
type = "path"
"#;
        let m = Manifest::parse(&text).unwrap();
        let s = &m.settings;
        assert_eq!(s.len(), 5);
        assert_eq!(
            s[0],
            SettingDecl {
                id: "prefix".into(),
                kind: SettingKind::Text,
                default: SettingValue::Text("TDOD_".into()),
                label: "Team prefix".into(),
                help: "What tags begin with".into(),
                scope: Scope::Project,
            }
        );
        assert_eq!(s[1].kind, SettingKind::Int { min: Some(1), max: Some(32) });
        assert_eq!(
            (&s[1].default, s[1].scope, s[1].shown()),
            (&SettingValue::Int(32), Scope::User, "max_length")
        );
        assert_eq!(s[2].default, SettingValue::Bool(false));
        let choices = vec!["both".to_string(), "creatures".into(), "placeables".into()];
        assert_eq!(s[3].kind, SettingKind::Choice(choices));
        assert_eq!(
            (&s[3].default, s[3].scope),
            (&SettingValue::Text("both".into()), Scope::UserProject)
        );
        assert_eq!((s[4].kind.name(), &s[4].default), ("path", &SettingValue::Text(String::new())));
        // Values held to their kinds.
        assert!(s[1].check(&SettingValue::Int(33)).unwrap_err().contains("most it takes"));
        assert!(s[1].check(&SettingValue::Int(0)).unwrap_err().contains("least it takes"));
        assert_eq!(
            s[1].check(&SettingValue::Text("3".into())).unwrap_err(),
            "\"3\" is not a whole number"
        );
        assert!(
            s[3].check(&SettingValue::Text("doors".into()))
                .unwrap_err()
                .contains("not one of both,")
        );
        assert!(s[2].check(&SettingValue::Bool(true)).is_ok());
        // Faults, each by its line.
        let with = |from: &str, to: &str| Manifest::parse(&text.replace(from, to)).unwrap_err();
        assert!(with("id = \"strict\"", "id = \"Strict\"").contains("lower-case letters"));
        assert!(with("id = \"strict\"", "id = \"set\"").contains("can't be called \"set\""));
        assert!(with("id = \"strict\"", "id = \"prefix\"").contains("a second setting \"prefix\""));
        assert!(with("type = \"bool\"", "type = \"float\"").contains("not \"float\""));
        assert!(with("default = 32", "default = 40").contains("its default: 40 is more"));
        assert!(with("default = 32", "default = many").contains("a whole number, not \"many\""));
        assert!(
            with("type = \"bool\"", "type = \"bool\"\nmin = 1").contains("is bool: it has no min")
        );
        assert!(with("choices = \"both, creatures , placeables\"\n", "").contains("needs choices"));
        assert!(with("scope = \"project\"", "scope = \"team\"").contains("not \"team\""));
        assert!(
            with("type = \"bool\"", "type = \"bool\"\ncolour = 1").contains("no key \"colour\"")
        );
        let e = Manifest::parse(&text.replace("api = \"0.4\"", "api = \"0.3\"")).unwrap_err();
        assert!(e.contains("plugin API 0.4") && e.contains("\"prefix\""), "{e}");
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
