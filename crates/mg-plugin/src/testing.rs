//! A plugin's own tests (`mg plugin test`): files `tests/*.toml` in its
//! folder, each naming a fixture module and what to run on it (a check, a
//! command, an event), with what is to come of it (the findings and their
//! fixes, the fields after the edits, the log, an error). Each test runs
//! on the fixture as it is on disk, which nothing writes to.
//!
//! ```toml
//! module = "module"            # the fixture, from this file's folder
//!
//! [[test]]
//! name = "a lower-case tag is found"
//! check = "tag-case"
//! [[test.finding]]
//! resource = "guard.utc"
//! at = "Tag"
//! message = 'tag "gate_guard" is not upper case'
//! fix = "Upper-case this tag"
//!
//! [[test]]
//! name = "the command fixes it"
//! command = "fix-tags"
//! log = ["1 creature tags changed"]
//! [[test.field]]
//! resource = "guard.utc"
//! field = "Tag"
//! value = "GATE_GUARD"
//!
//! [[test]]
//! name = "a builder's code is remembered"
//! command = "remember-code"
//! settings = { prefix = "KRZ_" }   # the plugin's settings for the run
//! answers = { code = "ab" }
//! set = { code = "ab" }            # what the job sets (ctx.settings:set)
//! ```

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use mg_core::{Codepage, Gender, Language};
use mg_edit::fix::{Found, Guard};
use mg_edit::wire::field_from_str;
use mg_edit::{Command, Workspace};
use mg_gff::Value;
use mg_module::Module;
use mg_module::doctor::Severity;
use mg_resman::ResKey;
use mg_rules::GameData;
use serde::Deserialize;

use crate::{
    Answer, Event, FieldKind, Host, Input, Level, Outcome, Plugin, PluginError, Question, Selected,
    Selection,
};

/// A test file, as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    /// The fixture module: a module folder, a `.mod` or a nasher project,
    /// from the test file's folder.
    module: Option<String>,
    #[serde(default, rename = "test")]
    tests: Vec<Test>,
}

/// One test: what to run, on what, and what is to come of it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Test {
    name: String,
    /// What is run: one of these, by its id in the manifest.
    check: Option<String>,
    command: Option<String>,
    event: Option<String>,
    /// The fixture, if not the file's.
    module: Option<String>,
    /// What the command is run on, and what is open (as `mg plugin run
    /// --select` and `--open` take them).
    #[serde(default)]
    select: Vec<String>,
    open: Option<String>,
    /// A form's values, by field id; the other fields take their defaults.
    #[serde(default)]
    answers: BTreeMap<String, toml::Value>,
    /// The answer to a question (`ctx.ui:confirm`): yes or no.
    #[serde(default)]
    yes: bool,
    /// The files and folders the plugin gets where it asks for one, in
    /// order, from the test file's folder.
    #[serde(default)]
    files: Vec<String>,
    /// An event's resources changed (a module saved); none: all.
    #[serde(default)]
    changed: Vec<String>,
    /// The findings, all of them in order (with `[[test.finding]]`), or
    /// how many.
    #[serde(default, rename = "finding")]
    expected: Vec<Expected>,
    findings: Option<usize>,
    /// Make the fixes so labeled, then hold the module to `field`,
    /// `edits` and `resources`, and the check to `left` findings.
    fix: Option<String>,
    left: Option<usize>,
    /// The fields after the edits.
    #[serde(default, rename = "field")]
    fields: Vec<FieldWanted>,
    /// How many edits, and which resources they change.
    edits: Option<usize>,
    resources: Option<Vec<String>>,
    /// Lines the log must have: each is part of one.
    #[serde(default)]
    log: Vec<String>,
    /// The job fails, saying this (part of what it says).
    error: Option<String>,
    /// The plugin's settings for the run, by id, over what the fixture's
    /// project files hold (and the defaults).
    #[serde(default)]
    settings: BTreeMap<String, toml::Value>,
    /// The settings the job is to set (`ctx.settings:set`), by id, with
    /// the value each ends with.
    #[serde(default)]
    set: BTreeMap<String, toml::Value>,
}

/// A setting's value as a test file writes it.
fn setting_of(v: &toml::Value) -> Option<crate::SettingValue> {
    match v {
        toml::Value::String(t) => Some(crate::SettingValue::Text(t.clone())),
        toml::Value::Integer(n) => Some(crate::SettingValue::Int(*n)),
        toml::Value::Boolean(b) => Some(crate::SettingValue::Bool(*b)),
        _ => None,
    }
}

/// The plugin's settings for a test: the fixture's project's values (and
/// the defaults), and the test's own.
fn settings_for(plugin: &Plugin, t: &Test, module: &Module) -> Result<crate::Settings, String> {
    let project = (module.location.as_ref())
        .and_then(mg_module::project_file::ProjectFile::root_of)
        .map(|_| &module.project_file);
    let (mut s, _) = crate::settings::resolve(&plugin.manifest, &BTreeMap::new(), project);
    for (name, v) in &t.settings {
        let Some(d) = plugin.setting(name) else {
            return Err(format!("settings: the plugin declares no setting {name:?}"));
        };
        let Some(v) = setting_of(v) else {
            return Err(format!("settings: {name}: text, a whole number, true or false"));
        };
        d.check(&v).map_err(|why| format!("settings: {name}: {why}"))?;
        s.values.insert(name.clone(), v);
    }
    Ok(s)
}

/// A finding a check is to make.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    resource: String,
    message: Option<String>,
    at: Option<String>,
    severity: Option<String>,
    /// Its fix's label; `""`: it has none.
    fix: Option<String>,
}

/// A field as it is to be after the edits.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldWanted {
    resource: String,
    /// Its path (`Tag`, `/Placeable List[0]/Tag`).
    field: String,
    value: Option<toml::Value>,
    /// The field is not there.
    #[serde(default)]
    absent: bool,
}

/// How one test went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResult {
    /// The test file's name (`tags.toml`) and the test's.
    pub file: String,
    pub name: String,
    /// What was not as expected: nothing, it passed.
    pub problems: Vec<String>,
}

impl TestResult {
    pub fn passed(&self) -> bool {
        self.problems.is_empty()
    }
}

/// The test files of a plugin: `tests/*.toml` in its folder, sorted.
pub fn test_files(plugin: &Plugin) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(plugin.dir.join("tests")) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "toml"))
        .collect();
    files.sort();
    files
}

/// Runs a plugin's tests: each of its test files' (`module`, when given,
/// in place of the fixture each file names). `game` gives the game data a
/// fixture is read with (none: the plugin's `ctx.game` is empty). A file
/// that does not read is a test that failed.
pub fn run_tests(
    plugin: &Plugin,
    module: Option<&Path>,
    game: &dyn Fn(&Module) -> Option<Arc<GameData>>,
) -> Vec<TestResult> {
    let mut out = Vec::new();
    for path in test_files(plugin) {
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let fail = |name: &str, why: String| TestResult {
            file: file.clone(),
            name: name.to_string(),
            problems: vec![why],
        };
        let parsed = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|text| toml::from_str::<File>(&text).map_err(|e| e.to_string()));
        let tests = match parsed {
            Ok(tests) => tests,
            Err(e) => {
                out.push(fail("(the file)", format!("it does not read: {e}")));
                continue;
            }
        };
        if tests.tests.is_empty() {
            out.push(fail("(the file)", "it has no tests ([[test]])".into()));
        }
        let dir = path.parent().unwrap_or(Path::new("."));
        // (Each fixture read once for the file.)
        let mut opened: BTreeMap<PathBuf, Result<Fixture, String>> = BTreeMap::new();
        for test in &tests.tests {
            let fixture = match (module, test.module.as_ref().or(tests.module.as_ref())) {
                (Some(given), _) => given.to_path_buf(),
                (None, Some(named)) => dir.join(named),
                (None, None) => {
                    let why = "no fixture: the file names no module, and none is given".into();
                    out.push(fail(&test.name, why));
                    continue;
                }
            };
            let read = opened.entry(fixture.clone()).or_insert_with(|| {
                let m =
                    Module::open(&fixture).map_err(|e| format!("{}: {e}", fixture.display()))?;
                let g = game(&m);
                Ok((m, g))
            });
            let problems = match read {
                Ok((m, g)) => run_one(plugin, test, dir, m, g.clone()),
                Err(e) => vec![e.clone()],
            };
            out.push(TestResult { file: file.clone(), name: test.name.clone(), problems });
        }
    }
    out
}

/// A fixture module read, and the game data it is read with.
type Fixture = (Module, Option<Arc<GameData>>);

/// The test host: the log kept, a form filled from the answers, a question
/// answered as the test says.
struct TestHost {
    log: RefCell<Vec<String>>,
    answers: BTreeMap<String, toml::Value>,
    files: RefCell<std::collections::VecDeque<PathBuf>>,
    yes: bool,
}

impl Host for TestHost {
    fn log(&self, _: Level, text: &str) {
        self.log.borrow_mut().push(text.to_string());
    }

    fn ask(&self, question: &Question) -> Option<Answer> {
        match question {
            Question::Message(text) => {
                self.log(Level::Info, text);
                Some(Answer::Yes)
            }
            Question::Confirm(_) => Some(if self.yes { Answer::Yes } else { Answer::No }),
            Question::Form { fields, .. } => {
                let mut values = BTreeMap::new();
                for f in fields {
                    let given = self.answers.get(&f.id).and_then(|v| toml_json(v).ok());
                    let value = given.unwrap_or_else(|| match &f.kind {
                        FieldKind::Text { default } => serde_json::json!(default),
                        FieldKind::Number { default, .. } => serde_json::json!(default),
                        FieldKind::Check { default } => serde_json::json!(default),
                        FieldKind::Choice { choices, default } => {
                            serde_json::json!(choices[*default])
                        }
                    });
                    values.insert(f.id.clone(), value);
                }
                Some(Answer::Values(values))
            }
            Question::File { .. } | Question::Folder { .. } => {
                self.files.borrow_mut().pop_front().map(Answer::Path)
            }
        }
    }
}

fn toml_json(v: &toml::Value) -> Result<serde_json::Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

/// What the test runs: a check, a command or an event.
enum Run<'a> {
    Check(&'a str),
    Command(&'a str),
    Event(&'a str),
}

fn run_one(
    plugin: &Plugin,
    t: &Test,
    dir: &Path,
    module: &Module,
    game: Option<Arc<GameData>>,
) -> Vec<String> {
    let mut problems = Vec::new();
    let run = match (&t.check, &t.command, &t.event) {
        (Some(c), None, None) => Run::Check(c),
        (None, Some(c), None) => Run::Command(c),
        (None, None, Some(e)) => Run::Event(e),
        _ => return vec!["a test runs one thing: a check, a command or an event".into()],
    };
    let mut selection = Selection::default();
    for s in &t.select {
        match Selected::parse(s) {
            Ok(s) => selection.selected.push(s),
            Err(e) => return vec![format!("select: {e}")],
        }
    }
    if let Some(open) = &t.open {
        match Selected::parse(open) {
            Ok(s) => selection.open = Some(s),
            Err(e) => return vec![format!("open: {e}")],
        }
    }
    let host = Rc::new(TestHost {
        log: RefCell::default(),
        answers: t.answers.clone(),
        files: RefCell::new(t.files.iter().map(|f| dir.join(f)).collect()),
        yes: t.yes,
    });
    let settings = match settings_for(plugin, t, module) {
        Ok(s) => s,
        Err(e) => return vec![e],
    };
    let input =
        Input { selection, settings: Some(settings), ..Input::new(module.clone(), game.clone()) };
    let codepage = game
        .as_deref()
        .map_or(Codepage::WINDOWS_1252, |g| g.codepage().for_language(Language::ENGLISH));
    // What it made: findings, or edits.
    let made: Result<Result<Vec<Found>, Outcome>, PluginError> = match run {
        Run::Check(id) => crate::run_check(plugin, id, input, host.clone()).map(Ok),
        Run::Command(id) => crate::run_command(plugin, id, input, host.clone()).map(Err),
        Run::Event(id) => {
            let kind = plugin.manifest.events.iter().find(|e| e.id == id).map(|e| e.on);
            let mut resources = Vec::new();
            for name in &t.changed {
                match ResKey::from_filename(name) {
                    Some(k) => resources.push(k),
                    None => return vec![format!("changed: {name:?} is not a resource's name")],
                }
            }
            if resources.is_empty() && kind == Some(crate::EventKind::ModuleSaved) {
                resources = module.keys().copied().collect();
                resources.sort();
            }
            let event = Event { kind: kind.unwrap_or(crate::EventKind::ModuleSaved), resources };
            crate::run_event(plugin, id, &event, input, host.clone()).map(Err)
        }
    };
    let log = host.log.borrow().clone();
    for wanted in &t.log {
        if !log.iter().any(|l| l.contains(wanted.as_str())) {
            problems.push(format!("the log has no line with {wanted:?} (it has {log:?})"));
        }
    }
    let made = match (made, &t.error) {
        (Err(e), Some(wanted)) => {
            if !e.to_string().contains(wanted.as_str()) {
                problems.push(format!("it failed, saying {:?}, not {wanted:?}", e.to_string()));
            }
            return problems;
        }
        (Err(e), None) => {
            problems.push(format!("it failed: {e}"));
            return problems;
        }
        (Ok(_), Some(wanted)) => {
            problems.push(format!("it was to fail, saying {wanted:?}, and did not"));
            return problems;
        }
        (Ok(made), None) => made,
    };
    let mut ws = Workspace::new(module.clone());
    // A check's findings held to the test, and the fixes asked for made; a
    // command's or an event's edits, to make.
    let commands: Vec<Command> = match &made {
        Ok(found) => {
            findings_as_wanted(t, found, &mut problems);
            if let Some(label) = &t.fix {
                fixes_made(module, found, label, &mut ws, &mut problems);
            }
            Vec::new()
        }
        Err(outcome) => {
            settings_as_wanted(t, &outcome.settings, &mut problems);
            vec![Command::new(outcome.label.clone(), outcome.edits.clone())]
        }
    };
    if made.is_ok() && !t.set.is_empty() {
        problems.push("a check sets no settings: set is for a command or an event".into());
    }
    let mut count = 0;
    let mut changed: Vec<String> = Vec::new();
    for cmd in commands {
        count += cmd.edits.len();
        for e in &cmd.edits {
            let k = e.key().to_string();
            if !changed.contains(&k) {
                changed.push(k);
            }
        }
        if let Err(e) = ws.apply(cmd) {
            problems.push(format!("its edits do not apply: {e}"));
            return problems;
        }
    }
    if let Some(fix) = &t.fix
        && let Ok(found) = &made
    {
        for f in found.iter().filter_map(|f| f.fix.as_ref()).filter(|f| f.label == *fix) {
            count += f.edits.len();
            for k in f.resources() {
                if !changed.contains(&k.to_string()) {
                    changed.push(k.to_string());
                }
            }
        }
    }
    if let Some(n) = t.edits
        && n != count
    {
        problems.push(format!("{count} edits, not {n}"));
    }
    if let Some(wanted) = &t.resources {
        let (mut a, mut b) = (changed.clone(), wanted.clone());
        a.sort();
        b.sort();
        if a != b {
            problems.push(format!("the edits change {a:?}, not {b:?}"));
        }
    }
    for f in &t.fields {
        if let Err(why) = field_as_wanted(&mut ws, f, codepage) {
            problems.push(why);
        }
    }
    if let (Some(n), Some(id)) = (t.left, &t.check) {
        if let Err(e) = ws.flush() {
            problems.push(e.to_string());
            return problems;
        }
        let input = Input::new(ws.module.clone(), game);
        match crate::run_check(plugin, id, input, host) {
            Ok(after) if after.len() != n => problems.push(format!(
                "after the fixes the check finds {}, not {n}: {:?}",
                after.len(),
                after.iter().map(|f| &f.finding.message).collect::<Vec<_>>()
            )),
            Ok(_) => {}
            Err(e) => problems.push(format!("after the fixes the check failed: {e}")),
        }
    }
    problems
}

/// What the job set against `set`: each setting named ends with its value.
fn settings_as_wanted(t: &Test, set: &[crate::SettingWrite], problems: &mut Vec<String>) {
    for (name, wanted) in &t.set {
        let last = set.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v);
        match (last, setting_of(wanted)) {
            (None, _) => problems.push(format!("it did not set {name}")),
            (Some(got), w) if got.as_ref() != w.as_ref() => problems.push(format!(
                "it set {name} to {}, not {wanted}",
                got.as_ref().map_or("its default".to_string(), |v| v.to_string())
            )),
            _ => {}
        }
    }
}

/// The findings against `[[test.finding]]` (all of them, in order) or
/// `findings` (how many).
fn findings_as_wanted(t: &Test, found: &[Found], problems: &mut Vec<String>) {
    let said = |f: &Found| {
        let fix = f.fix.as_ref().map_or(String::new(), |x| format!(" [fix: {}]", x.label));
        format!("{} {}: {}{fix}", f.finding.resource, f.finding.at, f.finding.message)
    };
    if let Some(n) = t.findings
        && n != found.len()
    {
        problems.push(format!(
            "{} findings, not {n}: {:?}",
            found.len(),
            found.iter().map(said).collect::<Vec<_>>()
        ));
    }
    if t.expected.is_empty() {
        return;
    }
    if t.expected.len() != found.len() {
        problems.push(format!(
            "{} findings, not {}: {:?}",
            found.len(),
            t.expected.len(),
            found.iter().map(said).collect::<Vec<_>>()
        ));
    }
    for (i, (want, got)) in t.expected.iter().zip(found).enumerate() {
        let f = &got.finding;
        let n = i + 1;
        if f.resource.to_string() != want.resource.to_ascii_lowercase() {
            problems.push(format!("finding {n} is of {}, not {}", f.resource, want.resource));
        }
        if let Some(m) = &want.message
            && *m != f.message
        {
            problems.push(format!("finding {n} says {:?}, not {m:?}", f.message));
        }
        if let Some(at) = &want.at
            && *at != f.at
        {
            problems.push(format!("finding {n} is at {:?}, not {at:?}", f.at));
        }
        if let Some(s) = &want.severity {
            let is = match f.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            if s != is {
                problems.push(format!("finding {n} is a {is}, not a {s}"));
            }
        }
        if let Some(label) = &want.fix {
            let has = got.fix.as_ref().map_or("", |x| x.label.as_str());
            if has != label {
                match has {
                    "" => problems.push(format!("finding {n} has no fix, not {label:?}")),
                    _ if label.is_empty() => {
                        problems.push(format!("finding {n} has the fix {has:?}, not none"))
                    }
                    _ => problems.push(format!("finding {n}'s fix is {has:?}, not {label:?}")),
                }
            }
        }
    }
}

/// Makes the fixes labeled `label`, each as the window would (held to the
/// module as the check read it).
fn fixes_made(
    module: &Module,
    found: &[Found],
    label: &str,
    ws: &mut Workspace,
    problems: &mut Vec<String>,
) {
    let fixes: Vec<&mg_edit::fix::Fix> =
        found.iter().filter_map(|f| f.fix.as_ref()).filter(|f| f.label == label).collect();
    if fixes.is_empty() {
        problems.push(format!("no finding has the fix {label:?}"));
        return;
    }
    let mut guard = Guard::new(module, fixes.iter().copied());
    for fix in fixes {
        let made =
            guard.check(ws, fix).and_then(|()| ws.apply(fix.command()).map_err(|e| e.to_string()));
        match made {
            Ok(()) => guard.made(ws, fix),
            Err(why) => problems.push(format!("the fix {label:?} is not made: {why}")),
        }
    }
}

/// A field after the edits against what the test wants.
fn field_as_wanted(ws: &mut Workspace, f: &FieldWanted, codepage: Codepage) -> Result<(), String> {
    let at = format!("{} {}", f.resource, f.field);
    let key = ResKey::from_filename(&f.resource)
        .ok_or_else(|| format!("{:?} is not a resource's name", f.resource))?;
    let (path, label) = field_from_str(&f.field).map_err(|e| format!("{at}: {e}"))?;
    let doc = ws.doc(&key).map_err(|e| format!("{at}: {e}"))?;
    let value = path.get(&doc.root).and_then(|s| s.get(&label));
    match (value, &f.value, f.absent) {
        (None, _, true) => Ok(()),
        (Some(v), _, true) => {
            Err(format!("{at} is there ({}), and was to be gone", shown(v, codepage)))
        }
        (None, _, false) => Err(format!("{at} is not there")),
        (Some(_), None, false) => Ok(()),
        (Some(v), Some(want), false) => {
            if same(v, want, codepage) {
                Ok(())
            } else {
                Err(format!("{at} is {}, not {want}", shown(v, codepage)))
            }
        }
    }
}

/// A field's value as the test file would write it.
fn shown(v: &Value, codepage: Codepage) -> String {
    match v {
        Value::String(b) => format!("{:?}", codepage.decode(b)),
        Value::ResRef(r) => format!("{:?}", String::from_utf8_lossy(r)),
        Value::LocString(l) => match l.get(Language::ENGLISH, Gender::Male) {
            Some(b) => format!("{:?}", codepage.decode(b)),
            None => format!("strref {}", l.strref.0),
        },
        other => match mg_gff::value_to_json(other, codepage) {
            Ok(j) => j.get("value").cloned().unwrap_or(j).to_string(),
            Err(e) => e.to_string(),
        },
    }
}

/// Whether a field's value is what the test file says: text for text (a
/// localized string's English), a number for a number.
fn same(v: &Value, want: &toml::Value, codepage: Codepage) -> bool {
    let number = |n: f64| match want {
        toml::Value::Integer(i) => *i as f64 == n,
        toml::Value::Float(f) => (*f - n).abs() < 1e-6,
        _ => false,
    };
    match v {
        Value::Byte(n) => number(f64::from(*n)),
        Value::Char(n) => number(f64::from(*n)),
        Value::Word(n) => number(f64::from(*n)),
        Value::Short(n) => number(f64::from(*n)),
        Value::Dword(n) => number(f64::from(*n)),
        Value::Int(n) => number(f64::from(*n)),
        Value::Dword64(n) => number(*n as f64),
        Value::Int64(n) => number(*n as f64),
        Value::Float(n) => number(f64::from(*n)),
        Value::Double(n) => number(*n),
        Value::String(b) => want.as_str() == Some(&codepage.decode(b)),
        Value::ResRef(r) => {
            want.as_str().is_some_and(|w| w.eq_ignore_ascii_case(&String::from_utf8_lossy(r)))
        }
        Value::LocString(l) => match want {
            toml::Value::Integer(i) => i64::from(l.strref.0) == *i,
            toml::Value::String(s) => {
                l.get(Language::ENGLISH, Gender::Male).map(|b| codepage.decode(b)).as_deref()
                    == Some(s.as_str())
            }
            _ => false,
        },
        other => match (mg_gff::value_to_json(other, codepage), toml_json(want)) {
            (Ok(j), Ok(w)) => j.get("value") == Some(&w),
            _ => false,
        },
    }
}
