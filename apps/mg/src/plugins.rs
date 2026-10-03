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
use mg_module::doctor::{Finding, Severity};
use mg_plugin::{Answer, FieldKind, Host, Input, Level, Plugin, PluginError, Question};
use mg_resman::{GameInstall, LayerClass, priority};
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
    let haks = m.haks().unwrap_or_default();
    match game.resman.add_haks(gi, &haks.iter().map(String::as_str).collect::<Vec<_>>()) {
        Ok(missing) => missing.iter().for_each(|h| out.note(format!("warning: hak {h} not found"))),
        Err(e) => out.note(format!("warning: the module's haks: {e}")),
    }
    game.resman.add(priority::MODULE, "module", LayerClass::Erf, m.container());
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
        }
    }
}

impl CliHost {
    pub(crate) fn new(answers: &[String], yes: bool) -> Result<CliHost> {
        let answers = answers
            .iter()
            .map(|a| {
                a.split_once('=')
                    .map(|(k, v)| (k.trim().to_string(), v.to_string()))
                    .with_context(|| format!("--answer {a:?}: give it as ID=VALUE"))
            })
            .collect::<Result<_>>()?;
        Ok(CliHost { answers, yes, ..Default::default() })
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

/// `mg plugin list`: what each plugin is and adds.
pub(crate) fn list(dirs: &[PathBuf]) -> Result<Output> {
    let mut out = Output::default();
    let mut all = Vec::new();
    for dir in dirs {
        for found in plugins_in(dir) {
            match found {
                Ok(p) => {
                    let m = &p.manifest;
                    out.line(format!("{}\t{} {}\t{}", m.id, m.name, m.version, p.dir.display()));
                    for c in &m.commands {
                        out.line(format!("\tcommand\t{}\t{}", c.id, c.title));
                    }
                    for c in &m.checks {
                        out.line(format!("\tcheck\t{}\t{}", c.id, c.title));
                    }
                    all.push(json!({
                        "id": m.id, "name": m.name, "version": m.version, "api": m.api,
                        "license": m.license, "description": m.description,
                        "authors": m.authors, "folder": p.dir.display().to_string(),
                        "commands": m.commands.iter().map(|c| json!({
                            "id": c.id, "title": c.title, "hint": c.hint, "key": c.key,
                        })).collect::<Vec<_>>(),
                        "checks": m.checks.iter().map(|c| json!({
                            "id": c.id, "title": c.title,
                            "severity": severity(c.severity),
                        })).collect::<Vec<_>>(),
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

/// `mg plugin run`: a plugin's command on a module, its edits applied as
/// one command and saved (or, with `dry_run`, printed).
pub(crate) fn run(
    gi: Option<&GameInstall>,
    module: &Path,
    dir: &Path,
    command: &str,
    host: CliHost,
    dry_run: bool,
) -> Result<Output> {
    let plugin = Plugin::load(dir).map_err(|e| anyhow!("{e}"))?;
    let m = Module::open(module)?;
    let mut notes = Output::default();
    let game = game_for(gi, &m, &mut notes);
    let host = Rc::new(host);
    let input = Input { module: m.clone(), game };
    let outcome = mg_plugin::run_command(&plugin, command, input, host.clone());
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
    let cmd = Command::new(outcome.label, outcome.edits);
    let mut out = super::edits::run(Workspace::new(m), cmd, dry_run, None, dry_run)?;
    out.notes.splice(0..0, notes.notes);
    host.finish(&mut out);
    Ok(out)
}

/// The findings of every check of the plugins in `dirs`, for `mg verify`.
/// A check that fails is itself a finding, an error.
pub(crate) fn findings(
    gi: &GameInstall,
    m: &Module,
    dirs: &[PathBuf],
    out: &mut Output,
) -> Result<Vec<Finding>> {
    let mut all = Vec::new();
    if dirs.is_empty() {
        return Ok(all);
    }
    let game = game_for(Some(gi), m, &mut Output::default());
    for dir in dirs {
        let found = plugins_in(dir);
        if found.is_empty() {
            bail!("{}: no plugin there (no plugin.cfg)", dir.display());
        }
        for plugin in found {
            let plugin = plugin.map_err(|e| anyhow!("{e}"))?;
            for check in &plugin.manifest.checks {
                let host = Rc::new(CliHost::default());
                let input = Input { module: m.clone(), game: game.clone() };
                match mg_plugin::run_check(&plugin, &check.id, input, host.clone()) {
                    Ok(found) => all.extend(found),
                    Err(e) => all.push(Finding {
                        severity: Severity::Error,
                        check: plugin.check_id(&check.id).into(),
                        source: "plugin".into(),
                        resource: mg_resman::ResKey::new(
                            mg_core::ResRef::from_str("module").expect("valid"),
                            mg_core::ResType::IFO,
                        ),
                        at: String::new(),
                        message: format!("the check failed: {e}"),
                    }),
                }
                for (_, text) in host.log.borrow().iter() {
                    out.note(format!("{}: {text}", plugin.manifest.name));
                }
            }
        }
    }
    Ok(all)
}
