//! `mg plugin`: plugins from the command line, and in a build pipeline.
//! They run as in the window: sandboxed, on the module as it is on disk,
//! their edits applied as one command. Questions a plugin asks are
//! answered from the arguments.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use mg_edit::{Command, Workspace};
use mg_module::Module;
use mg_module::doctor::Severity;
use mg_plugin::{Answer, FieldKind, Host, Input, Level, Plugin, PluginError, Question};
use mg_resman::GameInstall;
use mg_rules::GameData;
use serde_json::json;

use super::Output;

/// The plugins a folder names: the one it is, or those in its folders.
pub(crate) fn plugins_in(dir: &Path) -> Vec<Result<Plugin, PluginError>> {
    if dir.join("plugin.cfg").is_file() {
        vec![Plugin::load(dir)]
    } else {
        mg_plugin::discover(dir)
    }
}

/// The game's data as a plugin reads it: the module's haks and the module
/// layered in, its talk table. `None` (with a note) without a game.
pub(crate) fn game_for(
    gi: Option<&GameInstall>,
    m: &Module,
    out: &mut Output,
) -> Option<Arc<GameData>> {
    let Some(gi) = gi else {
        out.note("no game install found: a plugin's game data is empty (--root, or NWN_ROOT)");
        return None;
    };
    let mut game = match GameData::open(gi) {
        Ok(g) => g,
        Err(e) => {
            out.note(format!("the game data could not be read: {e}"));
            return None;
        }
    };
    let (missing, wrong) = m.stack_onto(&mut game.resman, gi);
    missing.iter().for_each(|h| out.note(format!("warning: hak {h} not found")));
    if let Some(e) = wrong {
        out.note(format!("warning: the module's haks: {e}"));
    }
    if let Some(name) = m.custom_tlk().ok().flatten()
        && let Some(found) = mg_module::talk::find(&game.resman, &gi.tlk_dirs(), &name)
        && let Ok(tlk) = mg_tlk::Tlk::read(&found.data)
    {
        game.set_custom_tlk(Some(tlk));
    }
    Some(Arc::new(game))
}

/// The command line as a plugin's host: the log kept for the output, a
/// form answered with its defaults and `--answer`s, a question with
/// `--yes` or no.
#[derive(Default)]
pub(crate) struct CliHost {
    pub(crate) log: RefCell<Vec<(Level, String)>>,
    answers: BTreeMap<String, String>,
    /// The files and folders it gets where it asks for one, in order.
    files: RefCell<std::collections::VecDeque<PathBuf>>,
    yes: bool,
    /// Answers given that no form asked for (a misspelled id, likely).
    used: RefCell<Vec<String>>,
    /// What went wrong answering (reported after the run).
    faults: RefCell<Vec<String>>,
}

impl Host for CliHost {
    fn log(&self, level: Level, text: &str) {
        self.log.borrow_mut().push((level, text.to_string()));
    }

    fn ask(&self, question: &Question) -> Option<Answer> {
        match question {
            Question::Message(text) => {
                self.log(Level::Info, text);
                Some(Answer::Yes)
            }
            Question::Confirm(text) => {
                let said = if self.yes { "yes" } else { "no (pass --yes to agree)" };
                self.log(Level::Info, &format!("{text} {said}"));
                Some(if self.yes { Answer::Yes } else { Answer::No })
            }
            Question::Form { fields, .. } => {
                let mut values = BTreeMap::new();
                for f in fields {
                    let given = self.answers.get(&f.id);
                    if given.is_some() {
                        self.used.borrow_mut().push(f.id.clone());
                    }
                    let fault = |why: String| {
                        self.faults.borrow_mut().push(format!("--answer {}: {why}", f.id));
                    };
                    let value = match (&f.kind, given) {
                        (FieldKind::Text { default }, g) => json!(g.unwrap_or(default)),
                        (FieldKind::Number { default, min, max }, g) => {
                            let n = match g.map(|g| g.trim().parse::<f64>()) {
                                None => *default,
                                Some(Ok(n)) => n,
                                Some(Err(_)) => {
                                    fault(format!("{:?} is not a number", g.unwrap()));
                                    *default
                                }
                            };
                            if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) {
                                fault(format!("{n} is outside what the field takes"));
                            }
                            json!(n)
                        }
                        (FieldKind::Check { default }, g) => match g.map(|g| g.trim()) {
                            None => json!(default),
                            Some("true" | "yes" | "1") => json!(true),
                            Some("false" | "no" | "0") => json!(false),
                            Some(other) => {
                                fault(format!("{other:?} is not true or false"));
                                json!(default)
                            }
                        },
                        (FieldKind::Choice { choices, default }, g) => match g {
                            None => json!(choices[*default]),
                            Some(g) if choices.contains(g) => json!(g),
                            Some(g) => {
                                fault(format!("{g:?} is not one of {}", choices.join(", ")));
                                json!(choices[*default])
                            }
                        },
                    };
                    values.insert(f.id.clone(), value);
                }
                Some(Answer::Values(values))
            }
            Question::File { title, .. } | Question::Folder { title } => {
                let given = self.files.borrow_mut().pop_front();
                match &given {
                    Some(path) => self.log(Level::Info, &format!("{title} {}", path.display())),
                    None => self.log(Level::Info, &format!("{title} none given (--file PATH)")),
                }
                given.map(Answer::Path)
            }
        }
    }
}

impl CliHost {
    pub(crate) fn new(answers: &[String], files: &[PathBuf], yes: bool) -> Result<CliHost> {
        let answers = answers
            .iter()
            .map(|a| {
                a.split_once('=')
                    .map(|(k, v)| (k.trim().to_string(), v.to_string()))
                    .with_context(|| format!("--answer {a:?}: give it as ID=VALUE"))
            })
            .collect::<Result<_>>()?;
        let files = RefCell::new(files.iter().cloned().collect());
        Ok(CliHost { answers, files, yes, ..Default::default() })
    }

    /// What was wrong with the answers given, if anything: nothing is
    /// saved then.
    fn answered(&self) -> Result<()> {
        let used = self.used.borrow();
        if let Some(unasked) = self.answers.keys().find(|k| !used.contains(k)) {
            bail!("--answer {unasked}: no form asked for it");
        }
        if let Some(fault) = self.faults.borrow().first() {
            bail!("{fault}");
        }
        if let Some(unasked) = self.files.borrow().front() {
            bail!("--file {}: the plugin asked for no more files", unasked.display());
        }
        Ok(())
    }

    /// The log as the output's notes.
    fn finish(&self, out: &mut Output) {
        let mut log = Vec::new();
        for (level, text) in self.log.borrow().iter() {
            let level = match level {
                Level::Info => "info",
                Level::Warning => "warning",
                Level::Error => "error",
            };
            out.note(format!("{level}: {text}"));
            log.push(json!({ "level": level, "text": text }));
        }
        if let serde_json::Value::Object(o) = &mut out.json {
            o.insert("log".into(), log.into());
        }
    }
}

/// A project's root, from what names it: a module folder, a nasher
/// project's folder or its `nasher.cfg`.
pub(crate) fn project_root(path: &Path) -> Result<PathBuf> {
    let root = match path.file_name() {
        Some(n) if n == "nasher.cfg" => path.parent().unwrap_or(Path::new(".")),
        _ => path,
    };
    if !root.is_dir() {
        bail!("{}: not a project's folder", root.display());
    }
    Ok(root.to_path_buf())
}

/// A setting as `mg plugin list` says it: its id, kind, scope and value.
fn setting_json(d: &mg_plugin::SettingDecl, value: &mg_plugin::SettingValue) -> serde_json::Value {
    let choices = match &d.kind {
        mg_plugin::SettingKind::Choice(c) => json!(c),
        _ => serde_json::Value::Null,
    };
    let (min, max) = match &d.kind {
        mg_plugin::SettingKind::Int { min, max } => (json!(min), json!(max)),
        _ => (serde_json::Value::Null, serde_json::Value::Null),
    };
    json!({
        "id": d.id, "type": d.kind.name(), "scope": d.scope.name(), "label": d.label,
        "help": d.help, "default": mg_plugin::settings::to_json(&d.default),
        "value": mg_plugin::settings::to_json(value), "choices": choices, "min": min, "max": max,
    })
}

/// `mg plugin list`: what each plugin is and adds; with `project`, also
/// the project's own plugins, as the builder allowed them.
pub(crate) fn list(dirs: &[PathBuf], project: Option<&Path>) -> Result<Output> {
    let mut out = Output::default();
    let mut all = Vec::new();
    let mut found_all: Vec<(Result<Plugin, PluginError>, Option<mg_plugin::project::Trust>)> =
        dirs.iter().flat_map(|d| plugins_in(d)).map(|p| (p, None)).collect();
    let file = match project {
        Some(path) => {
            let root = project_root(path)?;
            let file = mg_module::project_file::ProjectFile::read(&root);
            for line in file.errors.iter().chain(&file.warnings) {
                out.note(line.clone());
            }
            let found = mg_plugin::project::discover(&root, &file);
            if found.is_empty() {
                out.note(format!(
                    "{}: the project comes with no plugins (none in {})",
                    root.display(),
                    mg_plugin::project::folder(&root, &file).display()
                ));
            }
            for f in found {
                match f.plugin {
                    Ok(p) => {
                        let t = mg_plugin::project::trust(&file, &p);
                        found_all.push((Ok(p), Some(t)));
                    }
                    Err(e) => out.note(format!("warning: {e}")),
                }
            }
            Some(file)
        }
        None => None,
    };
    {
        for (found, trust) in found_all {
            match found {
                Ok(p) => {
                    let m = &p.manifest;
                    let state = match &trust {
                        Some(t) => format!("\tthe project's: {}", t.name()),
                        None => String::new(),
                    };
                    out.line(format!(
                        "{}\t{} {}\t{}{state}",
                        m.id,
                        m.name,
                        m.version,
                        p.dir.display()
                    ));
                    for c in &m.commands {
                        let on: Vec<&str> = c.targets.iter().map(|t| t.name()).collect();
                        let on = if on.is_empty() {
                            String::new()
                        } else {
                            format!("\tfor {}", on.join(", "))
                        };
                        out.line(format!("\tcommand\t{}\t{}{on}", c.id, c.title));
                    }
                    for c in &m.checks {
                        out.line(format!("\tcheck\t{}\t{}", c.id, c.title));
                    }
                    for e in &m.events {
                        out.line(format!("\tevent\t{}\t{}", e.id, e.on.name()));
                    }
                    // Its settings, with their values in the project.
                    let in_project = file.as_ref().filter(|_| trust.is_some());
                    let (values, said) =
                        mg_plugin::settings::resolve(m, &BTreeMap::new(), in_project);
                    said.into_iter().for_each(|s| out.note(format!("warning: {s}")));
                    let mut settings = Vec::new();
                    for d in &m.settings {
                        let v = values.values.get(&d.id).unwrap_or(&d.default);
                        out.line(format!(
                            "\tsetting\t{}\t{}\t{}\t{v}",
                            d.id,
                            d.kind.name(),
                            d.scope.name()
                        ));
                        settings.push(setting_json(d, v));
                    }
                    let project = trust.as_ref().map(
                        |t| json!({ "trust": t.name(), "runs": t.runs(), "digest": p.digest() }),
                    );
                    all.push(json!({
                        "id": m.id, "name": m.name, "version": m.version, "api": m.api,
                        "license": m.license, "description": m.description,
                        "authors": m.authors, "folder": p.dir.display().to_string(),
                        "commands": m.commands.iter().map(|c| json!({
                            "id": c.id, "title": c.title, "hint": c.hint, "key": c.key,
                            "for": c.targets.iter().map(|t| t.name()).collect::<Vec<_>>(),
                        })).collect::<Vec<_>>(),
                        "checks": m.checks.iter().map(|c| json!({
                            "id": c.id, "title": c.title,
                            "severity": severity(c.severity),
                        })).collect::<Vec<_>>(),
                        "events": m.events.iter().map(|e| json!({
                            "id": e.id, "on": e.on.name(),
                        })).collect::<Vec<_>>(),
                        "settings": settings,
                        "project": project,
                    }));
                }
                Err(e) => out.note(format!("warning: {e}")),
            }
        }
    }
    out.json = json!({ "plugins": all });
    Ok(out)
}

fn severity(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// `mg plugin check`: each plugin's manifest reads, and its code registers
/// what the manifest declares, no more and no less.
pub(crate) fn check(dirs: &[PathBuf]) -> Result<Output> {
    let mut out = Output::default();
    let mut all = Vec::new();
    for dir in dirs {
        // A file is a plugin's archive: what the installer checks of it,
        // then its code, as it is in the archive.
        if dir.is_file() {
            let (name, faults) = match archive(dir) {
                Ok(package) => {
                    let host = Rc::new(CliHost::default());
                    let faults = match package.inspect(host) {
                        Ok(faults) => faults,
                        Err(e) => vec![e.to_string()],
                    };
                    (package.manifest.id.clone(), faults)
                }
                Err(e) => (dir.display().to_string(), vec![e.to_string()]),
            };
            match faults.as_slice() {
                [] => out.line(format!("{name}\tok")),
                faults => faults.iter().for_each(|f| out.line(format!("{name}\t{f}"))),
            }
            out.failed |= !faults.is_empty();
            all.push(json!({ "plugin": name, "faults": faults }));
            continue;
        }
        let found = plugins_in(dir);
        if found.is_empty() {
            bail!("{}: no plugin there (no plugin.cfg)", dir.display());
        }
        for found in found {
            let (name, faults) = match found {
                Ok(p) => {
                    let host = Rc::new(CliHost::default());
                    let faults = match mg_plugin::inspect(&p, host) {
                        Ok(faults) => faults,
                        Err(e) => vec![e.to_string()],
                    };
                    (p.manifest.id.clone(), faults)
                }
                Err(e) => (dir.display().to_string(), vec![e.to_string()]),
            };
            match faults.as_slice() {
                [] => out.line(format!("{name}\tok")),
                faults => faults.iter().for_each(|f| out.line(format!("{name}\t{f}"))),
            }
            out.failed |= !faults.is_empty();
            all.push(json!({ "plugin": name, "faults": faults }));
        }
    }
    out.json = json!({ "plugins": all });
    Ok(out)
}

/// `mg plugin test`: a plugin's own tests (`tests/*.toml`), each on its
/// fixture module (or `module`), with the game's data where there is a
/// game. A line a test, a summary; failing if a test fails or there are
/// none.
pub(crate) fn test(gi: Option<&GameInstall>, dir: &Path, module: Option<&Path>) -> Result<Output> {
    let plugin = Plugin::load(dir).map_err(|e| anyhow!("{e}"))?;
    let mut out = Output::default();
    let notes = RefCell::new(Output::default());
    let game = |m: &Module| {
        let mut said = notes.borrow_mut();
        // (Said once: without a game, a plugin's ctx.game is empty.)
        match gi {
            None if said.notes.is_empty() => {
                game_for(None, m, &mut said);
                None
            }
            None => None,
            Some(gi) => game_for(Some(gi), m, &mut said),
        }
    };
    let results = mg_plugin::run_tests(&plugin, module, &game);
    out.notes.extend(notes.into_inner().notes);
    let id = &plugin.manifest.id;
    for r in &results {
        match r.problems.as_slice() {
            [] => out.line(format!("ok\t{} › {}", r.file, r.name)),
            problems => {
                for p in problems {
                    out.line(format!("FAIL\t{} › {}: {p}", r.file, r.name));
                }
            }
        }
    }
    let failed = results.iter().filter(|r| !r.passed()).count();
    let passed = results.len() - failed;
    if results.is_empty() {
        out.note(format!("mg: {id} has no tests (tests/*.toml in {})", plugin.dir.display()));
        out.failed = true;
    } else {
        out.note(format!("{id}: {} tests, {passed} passed, {failed} failed", results.len()));
        out.failed = failed > 0;
    }
    out.json = json!({
        "plugin": id,
        "passed": passed,
        "failed": failed,
        "tests": results.iter().map(|r| json!({
            "file": r.file, "name": r.name, "passed": r.passed(), "problems": r.problems,
        })).collect::<Vec<_>>(),
    });
    Ok(out)
}

/// `mg plugin pack`: a plugin's folder as an archive to hand around, which
/// Install Plugin from File and `mg plugin install` take. Checked first,
/// as `mg plugin check` does: an archive of a plugin that does not load is
/// not written.
pub(crate) fn pack(dir: &Path, output: Option<&Path>) -> Result<Output> {
    let plugin = Plugin::load(dir).map_err(|e| anyhow!("{e}"))?;
    let faults = match mg_plugin::inspect(&plugin, Rc::new(CliHost::default())) {
        Ok(faults) => faults,
        Err(e) => vec![e.to_string()],
    };
    if let Some(fault) = faults.first() {
        bail!("{}: {fault}", plugin.manifest.id);
    }
    let bytes = mg_plugin::pack(&plugin).map_err(|e| anyhow!("{e}"))?;
    let m = &plugin.manifest;
    let path = match output {
        Some(path) => path.to_path_buf(),
        None => PathBuf::from(format!("{}-{}.zip", m.id, m.version)),
    };
    std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;
    let package = mg_plugin::Package::read(&bytes).map_err(|e| anyhow!("{e}"))?;
    let files: Vec<&str> = package.files().collect();
    let mut out = Output::new(json!({
        "id": m.id, "version": m.version, "file": path.display().to_string(),
        "bytes": bytes.len(), "files": files,
    }));
    out.line(format!("{}\t{} {}\t{} files", path.display(), m.name, m.version, files.len()));
    Ok(out)
}

/// A plugin's archive read from a file, as the installer reads it.
fn archive(file: &Path) -> Result<mg_plugin::Package> {
    mg_plugin::Package::open(file).map_err(|e| anyhow!("{e}"))
}

/// `mg plugin remove`: a plugin installed from an archive taken out of a
/// folder of plugins, by its id. One put there by hand is left alone.
pub(crate) fn remove(id: &str, plugins: &Path) -> Result<Output> {
    let found = mg_plugin::discover(plugins)
        .into_iter()
        .flatten()
        .find(|p| p.manifest.id == id)
        .map(|p| (p.dir, p.manifest.version))
        // (One whose manifest does not read is still where it was put.)
        .or_else(|| Some((plugins.join(id), String::new())).filter(|(dir, _)| dir.is_dir()));
    let Some((dir, version)) = found else {
        bail!("{}: no plugin {id} there", plugins.display());
    };
    mg_plugin::remove(&dir).map_err(|e| anyhow!("{e}"))?;
    let mut out = Output::new(json!({
        "id": id, "version": version, "folder": dir.display().to_string(),
    }));
    out.line(format!("{id}\t{version}\tremoved from {}", plugins.display()));
    Ok(out)
}

/// `mg plugin install`: a plugin's archive unpacked into a plugins folder
/// (as Install Plugin from File does: checked, none of it run), in a
/// folder named by its id.
pub(crate) fn install(file: &Path, plugins: &Path, replace: bool) -> Result<Output> {
    let package = archive(file).map_err(|e| anyhow!("{}: {e}", file.display()))?;
    let replaced = match package.existing(plugins) {
        mg_plugin::Existing::Installed { version } if !replace => bail!(
            "{} {version} is installed already (--replace installs over it)",
            package.manifest.name
        ),
        mg_plugin::Existing::Installed { version } => Some(version),
        _ => None,
    };
    let plugin = package.install(plugins, replace).map_err(|e| anyhow!("{e}"))?;
    let m = &plugin.manifest;
    let mut out = Output::new(json!({
        "id": m.id, "version": m.version, "folder": plugin.dir.display().to_string(),
        "replaced": replaced,
    }));
    out.line(format!("{}\t{} {}\t{}", m.id, m.name, m.version, plugin.dir.display()));
    Ok(out)
}

/// `mg plugin run`: a plugin's command on a module, its edits applied as
/// one command and saved (or, with `dry_run`, printed).
/// What of a plugin to run: a command, or (`changed`) what it does by
/// itself on an event, with the resources a save changed (none named:
/// all of the module's).
pub(crate) struct What<'a> {
    pub id: &'a str,
    pub changed: Option<Vec<mg_resman::ResKey>>,
    /// What a command is run on (`--select`), and what is open
    /// (`--open`).
    pub selection: mg_plugin::Selection,
    /// Its settings for the run (`--setting ID=VALUE`).
    pub settings: &'a [String],
}

/// The settings of a plugin for a run on a module: what the module's
/// project's files hold, else the defaults (mg keeps none of the user's),
/// with `given` (`ID=VALUE`) over them; what is wrong with those kept, as
/// notes.
fn settings_for(
    plugin: &Plugin,
    m: &Module,
    given: &[String],
    out: &mut Output,
) -> Result<mg_plugin::Settings> {
    let project = project_of(m).map(|_| &m.project_file);
    let (mut s, said) = mg_plugin::settings::resolve(&plugin.manifest, &BTreeMap::new(), project);
    said.into_iter().for_each(|w| out.note(format!("warning: {w}")));
    for g in given {
        let (id, value) =
            g.split_once('=').with_context(|| format!("--setting {g:?}: give it as ID=VALUE"))?;
        let id = id.trim();
        let Some(d) = plugin.setting(id) else {
            let ids: Vec<&str> = plugin.manifest.settings.iter().map(|d| d.id.as_str()).collect();
            bail!(
                "--setting {id}: {} has no such setting ({})",
                plugin.manifest.id,
                ids.join(", ")
            );
        };
        let v =
            mg_plugin::settings::parse_for(d, value).map_err(|e| anyhow!("--setting {id}: {e}"))?;
        s.values.insert(id.to_string(), v);
    }
    Ok(s)
}

/// The root of a module's project: none for a module kept in one file.
fn project_of(m: &Module) -> Option<&Path> {
    m.location.as_ref().and_then(mg_module::project_file::ProjectFile::root_of)
}

/// Keeps what a plugin set (`ctx.settings:set`) in the project's files,
/// as the window does; one of scope "user" is not kept (mg keeps none).
/// With `dry_run`, says what it would keep.
fn keep_settings(
    plugin: &Plugin,
    m: &Module,
    set: &[mg_plugin::SettingWrite],
    dry_run: bool,
    out: &mut Output,
) -> Result<Vec<serde_json::Value>> {
    let mut kept = Vec::new();
    // (The last value of each, in the order they were first set.)
    let mut last: Vec<(&str, Option<&mg_plugin::SettingValue>)> = Vec::new();
    for (id, v) in set {
        match last.iter_mut().find(|(i, _)| *i == id.as_str()) {
            Some(slot) => slot.1 = v.as_ref(),
            None => last.push((id, v.as_ref())),
        }
    }
    for (id, v) in last {
        let Some(d) = plugin.setting(id) else { continue };
        let shown = v.map_or_else(|| format!("its default ({})", d.default), |v| v.to_string());
        let place = mg_plugin::settings::scope_words(d.scope);
        let done = match (d.scope.in_project(), project_of(m), dry_run) {
            (false, _, _) => {
                out.note(format!(
                    "{}: {id} set to {shown} is not kept: mg keeps no settings of scope \
                     \"user\" (Moonglow's Plugins window does)",
                    plugin.manifest.name
                ));
                false
            }
            (true, None, _) => bail!("{id}: no project to keep it in"),
            (true, Some(_), true) => {
                out.note(format!("would set {id} to {shown} ({place})"));
                false
            }
            (true, Some(root), false) => {
                mg_plugin::settings::keep_in_project(root, &plugin.manifest.id, d, v)
                    .map_err(|e| anyhow!("{id}: {e}"))?;
                out.note(format!("{}: {id} set to {shown} ({place})", plugin.manifest.name));
                true
            }
        };
        kept.push(json!({
            "id": id, "scope": d.scope.name(),
            "value": v.map(mg_plugin::settings::to_json), "kept": done,
        }));
    }
    Ok(kept)
}

pub(crate) fn run(
    gi: Option<&GameInstall>,
    module: &Path,
    dir: &Path,
    what: &What<'_>,
    host: CliHost,
    hak_dir: Option<&Path>,
    dry_run: bool,
) -> Result<Output> {
    let plugin = Plugin::load(dir).map_err(|e| anyhow!("{e}"))?;
    let m = Module::open(module)?;
    let mut notes = Output::default();
    let game = game_for(gi, &m, &mut notes);
    // (The edits are shown in the module's own letters, as the plugin
    // wrote them.)
    let codepage = game.as_ref().map_or(mg_core::Codepage::WINDOWS_1252, |game| {
        game.codepage().for_language(mg_core::Language::ENGLISH)
    });
    let host = Rc::new(host);
    // What it is run on is the module's (a blueprint may be the game's).
    let named = what.selection.selected.iter().chain(&what.selection.open);
    for s in named {
        let (key, _) = s.place();
        let game_has = game.as_ref().is_some_and(|g| g.resman.get(&key).is_ok());
        if !m.contains(&key) && !(matches!(s, mg_plugin::Selected::Blueprint(_)) && game_has) {
            bail!("{key} is not in the module");
        }
    }
    if let (Some(decl), false) =
        (plugin.manifest.commands.iter().find(|c| c.id == what.id), what.changed.is_some())
    {
        // (A command run on what it is not for: as the window would not
        // offer it. It is still run: it may say so itself.)
        let unfit: Vec<String> = (what.selection.selected.iter())
            .filter(|s| !s.target().is_some_and(|t| decl.targets.contains(&t)))
            .map(|s| s.place().0.to_string())
            .collect();
        if !decl.targets.is_empty() && !unfit.is_empty() {
            notes.note(format!(
                "warning: {} is not for {} (it is for {})",
                decl.id,
                unfit.join(", "),
                decl.targets.iter().map(|t| t.name()).collect::<Vec<_>>().join(", ")
            ));
        }
    }
    let settings = settings_for(&plugin, &m, what.settings, &mut notes)?;
    let input = Input {
        selection: what.selection.clone(),
        settings: Some(settings),
        ..Input::new(m.clone(), game)
    };
    let outcome = match &what.changed {
        None => mg_plugin::run_command(&plugin, what.id, input, host.clone()),
        Some(changed) => {
            let on = plugin.manifest.events.iter().find(|e| e.id == what.id).map(|e| e.on);
            let mut resources = changed.clone();
            if resources.is_empty() {
                resources = m.keys().copied().collect();
                resources.sort();
            }
            // (An event that is not the plugin's is refused below.)
            let kind = on.unwrap_or(mg_plugin::EventKind::ModuleSaved);
            if kind == mg_plugin::EventKind::ModuleOpened {
                resources.clear();
            }
            let event = mg_plugin::Event { kind, resources };
            mg_plugin::run_event(&plugin, what.id, &event, input, host.clone())
        }
    };
    let outcome = match outcome {
        Ok(o) => o,
        Err(e) => {
            // What it logged before it failed belongs with the failure.
            let log: Vec<String> =
                host.log.borrow().iter().map(|(_, text)| format!("\n  {text}")).collect();
            bail!("{e}{}", log.concat());
        }
    };
    host.answered()?;
    // Its haks first: the module's edits may name them.
    let hak_dir = hak_dir
        .map(Path::to_path_buf)
        .or_else(|| gi.and_then(|gi| gi.user_dir.as_ref().map(|u| u.join("hak"))));
    for hak in &outcome.haks {
        let Some(dir) = &hak_dir else {
            bail!("the plugin writes the hak {}: say where haks go (--hak-dir DIR)", hak.name);
        };
        let file = dir.join(format!("{}.hak", hak.name));
        if dry_run {
            notes.note(format!("would put {} resources into {}", hak.files.len(), file.display()));
            continue;
        }
        let (added, replaced) = hak.write(dir).map_err(|e| anyhow!("{e}"))?;
        notes.note(format!("{}: {added} resources added, {replaced} replaced", file.display()));
    }
    for folder in &outcome.folders {
        if dry_run {
            let n = folder.files.len();
            notes.note(format!("would write {n} files into {}", folder.dir.display()));
            continue;
        }
        let (new, replaced) =
            folder.write().map_err(|e| anyhow!("{}: {e}", folder.dir.display()))?;
        notes.note(format!("{}: {new} files written, {replaced} replaced", folder.dir.display()));
    }
    let cmd = Command::new(outcome.label, outcome.edits);
    let module = m.clone();
    let mut out = super::edits::run(Workspace::new(m), cmd, dry_run, None, dry_run, codepage)?;
    let kept = keep_settings(&plugin, &module, &outcome.settings, dry_run, &mut notes)?;
    if let serde_json::Value::Object(o) = &mut out.json
        && !kept.is_empty()
    {
        o.insert("settings".into(), kept.into());
    }
    out.notes.splice(0..0, notes.notes);
    host.finish(&mut out);
    Ok(out)
}

/// The plugins whose checks `mg verify` runs: those named, and the
/// project's own (those allowed, or with `project` all of them).
pub(crate) struct Chosen<'a> {
    pub dirs: &'a [PathBuf],
    pub project: bool,
}

/// The project's plugins that run: as the builder allowed them, or every
/// one (`all`); each that does not is said.
fn project_plugins(m: &Module, all: bool, named: &[Plugin], out: &mut Output) -> Vec<Plugin> {
    let Some(root) = project_of(m) else { return Vec::new() };
    let file = &m.project_file;
    let mut run = Vec::new();
    for f in mg_plugin::project::discover(root, file) {
        let p = match f.plugin {
            Ok(p) => p,
            Err(e) => {
                out.note(format!("warning: the project's plugin {}: {e}", f.dir.display()));
                continue;
            }
        };
        let id = &p.manifest.id;
        if named.iter().any(|n| n.manifest.id == *id) {
            out.note(format!("{id}: the one --plugins names runs, not the project's"));
            continue;
        }
        let t = mg_plugin::project::trust(file, &p);
        if all || t.runs() {
            run.push(p);
            continue;
        }
        let why = match t {
            mg_plugin::project::Trust::Denied => "you said no to it".to_string(),
            mg_plugin::project::Trust::Changed { was } => {
                format!("it changed since you allowed it (version {was})")
            }
            _ => "you have not allowed it".to_string(),
        };
        out.note(format!(
            "the project's plugin {id} {} did not run: {why} (Moonglow asks when the project \
             is opened; --allow-project-plugins runs every one, in a build pipeline)",
            p.manifest.version
        ));
    }
    run
}

/// The findings of every check of the plugins chosen, for `mg verify`,
/// each with its fix if the check gave it one.
/// A check that fails is itself a finding, an error.
pub(crate) fn findings(
    gi: &GameInstall,
    m: &Module,
    chosen: &Chosen<'_>,
    out: &mut Output,
) -> Result<Vec<mg_plugin::Found>> {
    let mut all = Vec::new();
    let mut plugins = Vec::new();
    for dir in chosen.dirs {
        let found = plugins_in(dir);
        if found.is_empty() {
            bail!("{}: no plugin there (no plugin.cfg)", dir.display());
        }
        for plugin in found {
            plugins.push(plugin.map_err(|e| anyhow!("{e}"))?);
        }
    }
    let theirs = project_plugins(m, chosen.project, &plugins, out);
    plugins.extend(theirs);
    if plugins.is_empty() {
        return Ok(all);
    }
    let game = game_for(Some(gi), m, &mut Output::default());
    {
        for plugin in plugins {
            let settings = settings_for(&plugin, m, &[], out)?;
            for check in &plugin.manifest.checks {
                let host = Rc::new(CliHost::default());
                let input = Input {
                    settings: Some(settings.clone()),
                    ..Input::new(m.clone(), game.clone())
                };
                match mg_plugin::run_check(&plugin, &check.id, input, host.clone()) {
                    Ok(found) => all.extend(found),
                    Err(e) => all.push(plugin.check_failed(&check.id, &e).into()),
                }
                for (_, text) in host.log.borrow().iter() {
                    out.note(format!("{}: {text}", plugin.manifest.name));
                }
            }
        }
    }
    Ok(all)
}
