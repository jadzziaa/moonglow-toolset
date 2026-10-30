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
//!
//! A test reads the game's data (`ctx.game`) from the machine's game, or
//! the one at `root` (`root = false`: none), with the fixture's haks and
//! the fixture in it (with no game, the fixture alone), and over it the
//! files of the folder `game` names (by default `game/` beside the test
//! file, where there is one), as the game's development folder is read:
//! so a test that reads only those runs without a game. That folder may
//! bring a `dialog.tlk`, the fixture's custom talk table and, in its
//! `hak/`, the fixture's haks. A test may also run Moonglow's own checks
//! (`builtin = "tag-unique"`, a list of them, or `"all"`) on its fixture,
//! with the fixture's project file.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, PoisonError, RwLock};

use mg_core::{Codepage, Gender, Language};
use mg_edit::fix::{Found, Guard};
use mg_edit::wire::field_from_str;
use mg_edit::{Command, GffPath, Step, Workspace};
use mg_gff::Value;
use mg_module::Module;
use mg_module::doctor::Severity;
use mg_resman::{Container, MemContainer, ResError, ResKey};
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
    /// A folder of the game's resources for the tests (2DAs, blueprints),
    /// from the test file's folder, over the game's: [`GAME_FOLDER`] where
    /// there is one, when this is not given.
    game: Option<String>,
    /// The game's root (its install's folder) the tests read the game
    /// from, in place of the machine's: absolute, from the test file's
    /// folder, or from the home folder (`~/`); `false`, no game (`true`,
    /// the machine's).
    root: Option<RootSetting>,
    #[serde(default, rename = "test")]
    tests: Vec<Test>,
}

/// A test's `root` as written: a game's folder, or whether to read one.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RootSetting {
    At(String),
    Game(bool),
}

/// A test's `builtin` as written: one of Moonglow's checks by its id,
/// several, or `"all"`.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum BuiltinSetting {
    One(String),
    Many(Vec<String>),
}

/// Moonglow's own checks a test runs, and whether it named them (rather
/// than `"all"`).
#[derive(Debug, Clone)]
struct Builtins {
    checks: Vec<mg_module::doctor::Check>,
    named: bool,
}

/// The game a test reads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Root {
    /// The one `mg` finds (the machine's), if any.
    Machine,
    /// The one at this folder.
    At(PathBuf),
    /// None (`root = false`): only the test's game folder and the module.
    No,
}

/// The folder of the game's resources beside a test file that the tests
/// read over the game's, where the file names none (`game = "…"`).
pub const GAME_FOLDER: &str = "game";

/// How [`run_tests`] reads a fixture's game data: the game at a root
/// (`Some`, a test's `root`) or the machine's (`None`), with the module's
/// haks and the module layered in. `Ok(None)`: no game; an error, a root
/// that is no game.
pub type GameFor<'a> = dyn Fn(&Module, Option<&Path>) -> Result<Option<GameData>, String> + 'a;

/// One test: what to run, on what, and what is to come of it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Test {
    name: String,
    /// What is run: one of these, by its id in the manifest, or one of
    /// Moonglow's own checks (Verify's, by its id: `tag-unique`).
    check: Option<String>,
    command: Option<String>,
    event: Option<String>,
    builtin: Option<BuiltinSetting>,
    /// The fixture, if not the file's.
    module: Option<String>,
    /// The game's resources and the game's root, if not the file's.
    game: Option<String>,
    root: Option<RootSetting>,
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
    /// An event's resources: changed (a module saved), saving or saved
    /// (none: those that differ from `was`, else all), or made.
    #[serde(default)]
    changed: Vec<String>,
    /// For a resource saving or saved: the module as it was before (a
    /// fixture, from the file's folder), whose resources are the old ones.
    was: Option<String>,
    /// What made the objects or resources (`ctx.event.by`).
    by: Option<String>,
    /// What a resource saving's handler says: its warnings and its
    /// refusals, all of them in order, each part of what one says.
    warned: Option<Vec<String>>,
    refused: Option<Vec<String>>,
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
    /// Its path (`Tag`, `/Placeable List[0]/Tag`), or a list's item's
    /// (`Equip_ItemList[2]`), which is there or not and has no value.
    field: String,
    value: Option<toml::Value>,
    /// The field (or the item) is not there.
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
    /// What a test that passed is told all the same: what it read of the
    /// game and the game's data lacked (it made do without), what of the
    /// fixture's project file was not understood.
    pub notes: Vec<String>,
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
/// fixture is read with (none: the plugin's `ctx.game` is the fixture
/// and the test's own game folder alone). A file that does not read is a
/// test that failed.
pub fn run_tests(plugin: &Plugin, module: Option<&Path>, game: &GameFor) -> Vec<TestResult> {
    let mut out = Vec::new();
    for path in test_files(plugin) {
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let fail = |name: &str, why: String| TestResult {
            file: file.clone(),
            name: name.to_string(),
            problems: vec![why],
            notes: Vec::new(),
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
        // (Each fixture read once for the file, with each game it is read
        // with.)
        let mut opened: BTreeMap<GameKey, Result<Fixture, String>> = BTreeMap::new();
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
            let root = match test.root.as_ref().or(tests.root.as_ref()) {
                None | Some(RootSetting::Game(true)) => Root::Machine,
                Some(RootSetting::Game(false)) => Root::No,
                Some(RootSetting::At(r)) => Root::At(from(dir, r)),
            };
            let folder = match test.game.as_ref().or(tests.game.as_ref()) {
                Some(named) => {
                    let folder = dir.join(named);
                    if !folder.is_dir() {
                        let why = format!("game: {} is not a folder", folder.display());
                        out.push(fail(&test.name, why));
                        continue;
                    }
                    Some(folder)
                }
                None => Some(dir.join(GAME_FOLDER)).filter(|f| f.is_dir()),
            };
            let at = (fixture.clone(), root.clone(), folder.clone());
            let read = opened.entry(at).or_insert_with(|| {
                let m =
                    Module::open(&fixture).map_err(|e| format!("{}: {e}", fixture.display()))?;
                let g = match &root {
                    Root::Machine => game(&m, None)?,
                    Root::At(r) => {
                        game(&m, Some(r)).map_err(|e| format!("root {}: {e}", r.display()))?
                    }
                    Root::No => None,
                };
                Ok(Fixture::new(m, g, folder.as_deref()))
            });
            let (problems, notes) = match read {
                Ok(f) => {
                    let from =
                        GameFrom { game: f.game, root: &root, folder: folder.as_deref(), dir };
                    run_one(plugin, test, dir, f, &from)
                }
                Err(e) => (vec![e.clone()], Vec::new()),
            };
            let name = test.name.clone();
            out.push(TestResult { file: file.clone(), name, problems, notes });
        }
    }
    out
}

/// A path a test file names: absolute, from the home folder (`~/`), or
/// from the test file's folder.
fn from(dir: &Path, named: &str) -> PathBuf {
    if let Some(rest) = named.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
    {
        return PathBuf::from(home).join(rest);
    }
    dir.join(named)
}

/// The module's layer in a test's game data: the fixture as it is, or,
/// for what is `left` after the fixes, the module as fixed, so that what
/// reads the module through the game's data (its 2DAs) reads the fixes.
#[derive(Debug, Clone)]
struct ModuleLayer(Arc<RwLock<MemContainer>>);

impl ModuleLayer {
    fn of(m: &Module) -> ModuleLayer {
        ModuleLayer(Arc::new(RwLock::new(m.container())))
    }

    /// Puts this module's resources in the layer (the game data is then
    /// to forget what it read: [`GameData::invalidate`]).
    fn set(&self, m: &Module) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = m.container();
    }

    fn now(&self) -> std::sync::RwLockReadGuard<'_, MemContainer> {
        self.0.read().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Container for ModuleLayer {
    fn contains(&self, key: &ResKey) -> bool {
        self.now().contains(key)
    }

    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        Ok(Cow::Owned(self.now().read(key)?.into_owned()))
    }

    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        Box::new(self.now().keys().collect::<Vec<_>>().into_iter())
    }

    fn len(&self) -> usize {
        self.now().len()
    }
}

/// A fixture module read, and the game data it is read with.
struct Fixture {
    module: Module,
    /// The game's (or none's) with the module's haks and the module, and
    /// the test's own folder of resources over it.
    data: Arc<GameData>,
    /// The module's layer in it.
    layer: ModuleLayer,
    /// There was a game (the machine's, or at `root`) under the test's
    /// folder.
    game: bool,
    /// Its talk tables are counted: a game's, or a `dialog.tlk` in the
    /// test's folder.
    talk: bool,
    /// What it could not read, for each test's notes.
    notes: Vec<String>,
    /// The haks it lists that its game data has not, for the notes of a
    /// test of Moonglow's checks (a plugin's reads say what they lacked).
    haks: Option<String>,
}

impl Fixture {
    /// The game's data (none: the module alone) with the test's own folder
    /// of resources over it, as the game's development folder is read
    /// (over the install, the haks and the module). The folder's
    /// `dialog.tlk` is read in place of the game's, its `hak/` has the
    /// module's haks (before those the game would find), and the module's
    /// custom talk table is the one the game data has (the module's haks,
    /// the module, its nasher project or the folder) over the one of the
    /// machine's `tlk/`.
    fn new(module: Module, g: Option<GameData>, folder: Option<&Path>) -> Fixture {
        use mg_resman::{DirContainer, ErfContainer, LayerClass, ResMan, priority};
        let game = g.is_some();
        let mut g =
            g.unwrap_or_else(|| GameData::new(ResMan::new(), mg_tlk::Tlk::new(Language::ENGLISH)));
        let mut notes = Vec::new();
        let layer = ModuleLayer::of(&module);
        if g.resman.replace("module", layer.clone()).is_none() {
            g.resman.add(priority::MODULE, "module", LayerClass::Erf, layer.clone());
        }
        let mut talk = game;
        let haks = module.haks().unwrap_or_default();
        if let Some(folder) = folder {
            for name in &haks {
                let path = folder.join("hak").join(format!("{name}.hak"));
                if !path.is_file() {
                    continue;
                }
                let label = format!("hak:{name}");
                match ErfContainer::open(&path) {
                    // (In the place of the game's, among the module's.)
                    Ok(c) if g.resman.layer_mut(&label).is_some() => {
                        g.resman.replace(&label, c);
                    }
                    Ok(c) => g.resman.add(priority::HAK_USER, label, LayerClass::Erf, c),
                    Err(e) => notes.push(format!("the test's hak {}: {e}", path.display())),
                }
            }
            let dialog = folder.join("dialog.tlk");
            if dialog.is_file() {
                let read = std::fs::read(&dialog)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| mg_tlk::Tlk::read(&bytes).map_err(|e| e.to_string()));
                match read {
                    Ok(tlk) => {
                        g.set_tlk(tlk);
                        talk = true;
                    }
                    Err(e) => {
                        notes.push(format!("the test's {} can't be read: {e}", dialog.display()))
                    }
                }
            }
            g.resman.add(
                priority::DEVELOPMENT,
                "test:game",
                LayerClass::Directory,
                DirContainer::open(folder),
            );
        }
        // (No user's tlk folder: a fixture's table is its own.)
        let found = mg_module::talk::of_module(&module, &g.resman, &[], Some(g.language));
        match found.read() {
            (_, Some(tlk)) => g.set_custom_tlk(Some(tlk)),
            (mg_module::talk::Custom::Unreadable { error, .. }, None) => {
                notes.push(format!("the module's talk table {error}"))
            }
            _ => {}
        }
        // (With a game, `mg` says which it did not find.)
        let lacked: Vec<&str> = (haks.iter())
            .filter(|h| g.resman.layer_mut(&format!("hak:{h}")).is_none())
            .map(String::as_str)
            .collect();
        let mut lacks = None;
        if !game && !lacked.is_empty() {
            let them = if lacked.len() == 1 { "it" } else { "them" };
            lacks = Some(format!(
                "the fixture's haks {} are not in the test's game data, which has no game: put \
                 {them} in its game folder's hak/ ({GAME_FOLDER}/hak/ beside the test file), or \
                 name a game with root = \"…\"",
                lacked.join(", ")
            ));
        }
        // (Tables looked up before, found or not, may be the folder's now.)
        g.invalidate();
        Fixture { module, data: Arc::new(g), layer, game, talk, notes, haks: lacks }
    }
}

/// A fixture read with a game: its path, the game's root and the test's
/// folder of resources.
type GameKey = (PathBuf, Root, Option<PathBuf>);

/// Where a test's game data came from, for what it says it lacked.
struct GameFrom<'a> {
    /// There was a game (the machine's, or at `root`).
    game: bool,
    root: &'a Root,
    folder: Option<&'a Path>,
    dir: &'a Path,
}

/// The test host: the log kept, a form filled from the answers, a question
/// answered as the test says.
struct TestHost {
    log: RefCell<Vec<String>>,
    /// The game's resources the plugin read and the game data lacked.
    not_in_game: RefCell<Vec<String>>,
    /// It listed the game's resources.
    listed: std::cell::Cell<bool>,
    /// What the test is told all the same.
    notes: RefCell<Vec<String>>,
    answers: BTreeMap<String, toml::Value>,
    files: RefCell<std::collections::VecDeque<PathBuf>>,
    yes: bool,
}

impl Host for TestHost {
    fn log(&self, _: Level, text: &str) {
        self.log.borrow_mut().push(text.to_string());
    }

    fn not_in_game(&self, resource: &str) {
        let mut lacked = self.not_in_game.borrow_mut();
        if !lacked.iter().any(|r| r == resource) {
            lacked.push(resource.to_string());
        }
    }

    fn game_listed(&self) {
        self.listed.set(true);
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

/// What the test runs: a check, a command or an event, or one of
/// Moonglow's own checks.
enum Run<'a> {
    Check(&'a str),
    Command(&'a str),
    Event(&'a str),
    Builtin(Builtins),
}

/// Runs a test: what was not as it wants, and what it is told all the
/// same.
fn run_one(
    plugin: &Plugin,
    t: &Test,
    dir: &Path,
    fixture: &Fixture,
    from: &GameFrom,
) -> (Vec<String>, Vec<String>) {
    let host = Rc::new(TestHost {
        log: RefCell::default(),
        not_in_game: RefCell::default(),
        listed: Default::default(),
        notes: RefCell::default(),
        answers: t.answers.clone(),
        files: RefCell::new(t.files.iter().map(|f| dir.join(f)).collect()),
        yes: t.yes,
    });
    let mut problems = held(plugin, t, dir, fixture, from, &host);
    let mut notes = fixture.notes.clone();
    notes.extend(host.notes.take());
    // What it read of the game and did not find: where it failed, that may
    // be why; where it passed, it made do without (in a `pcall`), and so
    // did not test what it would do with it.
    let game = match (from.game, from.root) {
        (_, Root::At(root)) => format!("the game at {} has none", root.display()),
        (true, _) => "this machine's game has none".to_string(),
        (false, Root::No) => "the test reads no game, root = false".to_string(),
        (false, _) => "there is no game install".to_string(),
    };
    let folder = from.folder.map(|f| f.strip_prefix(from.dir).unwrap_or(f).display().to_string());
    let lacked = host.not_in_game.borrow();
    if !lacked.is_empty() {
        let them = if lacked.len() == 1 { "it" } else { "them" };
        let (folder, put) = match &folder {
            Some(f) => (format!("nor the test's game folder, {f}"), format!("put {them} in {f}")),
            None => (
                format!("and the test has no game folder ({GAME_FOLDER}/ beside its file)"),
                format!("put {them} in a folder {GAME_FOLDER}/ beside the test file"),
            ),
        };
        let lacked = lacked.join(", ");
        if problems.is_empty() {
            notes.push(format!(
                "it passed without {lacked}, which it read and the game's data has not ({game}, \
                 {folder}): to test it with {them}, {put}, or name a game with root = \"…\""
            ));
        } else {
            problems.push(format!(
                "the game's data has no {lacked}, which it read ({game}, {folder}): {put}, or \
                 name a game with root = \"…\""
            ));
        }
    }
    // A list of the game's resources with no game is of what the test
    // brings alone.
    if host.listed.get() && !from.game {
        let game = game.trim_end_matches(" has none");
        let listed = match &folder {
            Some(f) => format!(
                "it listed the game's resources (ctx.game:resources()), and {game}: the list is \
                 only the module's and those of the test's game folder, {f}"
            ),
            None => format!(
                "it listed the game's resources (ctx.game:resources()), and {game}, nor a game \
                 folder: the list is only the module's (put those it looks for in a folder \
                 {GAME_FOLDER}/ beside the test file, or name a game with root = \"…\")"
            ),
        };
        if problems.is_empty() { notes.push(listed) } else { problems.push(listed) }
    }
    // A built-in check follows the fixture's project file: what of it
    // was not understood is left out of the rules it holds the module to.
    if t.builtin.is_some() {
        notes.extend(fixture.haks.clone());
        for w in &fixture.module.project_file.warnings {
            notes.push(format!("the fixture's project file: {w}"));
        }
    }
    (problems, notes)
}

/// What was not as the test wants.
fn held(
    plugin: &Plugin,
    t: &Test,
    dir: &Path,
    fixture: &Fixture,
    from: &GameFrom,
    host: &Rc<TestHost>,
) -> Vec<String> {
    let module = &fixture.module;
    let game = Some(fixture.data.clone());
    let mut problems = Vec::new();
    if (t.was.is_some() || t.by.is_some() || t.warned.is_some() || t.refused.is_some())
        && t.event.is_none()
    {
        return vec!["was, by, warned and refused are for an event".into()];
    }
    let run = match (&t.check, &t.command, &t.event, &t.builtin) {
        (Some(c), None, None, None) => Run::Check(c),
        (None, Some(c), None, None) => Run::Command(c),
        (None, None, Some(e), None) => Run::Event(e),
        (None, None, None, Some(b)) => match builtin_checks(b) {
            Ok(c) => Run::Builtin(c),
            Err(e) => return vec![e],
        },
        _ => {
            return vec![
                "a test runs one thing: a check, a command, an event or a builtin check".into(),
            ];
        }
    };
    if matches!(run, Run::Builtin(_)) {
        let theirs = [
            ("select", !t.select.is_empty()),
            ("open", t.open.is_some()),
            ("answers", !t.answers.is_empty()),
            ("yes", t.yes),
            ("files", !t.files.is_empty()),
            ("settings", !t.settings.is_empty()),
            ("log", !t.log.is_empty()),
            ("error", t.error.is_some()),
        ];
        let given: Vec<&str> = theirs.iter().filter(|(_, g)| *g).map(|(k, _)| *k).collect();
        if !given.is_empty() {
            return vec![format!(
                "{} {} for a plugin's own check, command or event, not a builtin check",
                given.join(", "),
                if given.len() == 1 { "is" } else { "are" }
            )];
        }
    }
    let mut selection = Selection::default();
    for s in &t.select {
        match Selected::parse(s) {
            Ok(Selected::Copy { .. }) => {
                return vec![format!("select: {s}: a layer's copy is only `open`")];
            }
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
    let settings = match settings_for(plugin, t, module) {
        Ok(s) => s,
        Err(e) => return vec![e],
    };
    let input =
        Input { selection, settings: Some(settings), ..Input::new(module.clone(), game.clone()) };
    let codepage = fixture.data.codepage().for_language(Language::ENGLISH);
    // Moonglow's checks that count talk-table strings, with none to count.
    if let Run::Builtin(b) = &run
        && let Some((said, wrong)) = uncounted(b, fixture, from)
    {
        if wrong {
            problems.push(said);
        } else {
            host.notes.borrow_mut().push(said);
        }
    }
    // What it made: findings, or edits.
    let made: Result<Result<Vec<Found>, Outcome>, PluginError> = match &run {
        Run::Check(id) => crate::run_check(plugin, id, input, host.clone()).map(Ok),
        Run::Builtin(b) => Ok(Ok(run_builtin(b, module, fixture))),
        Run::Command(id) => crate::run_command(plugin, id, input, host.clone()).map(Err),
        Run::Event(id) => {
            let kind = plugin.manifest.events.iter().find(|e| e.id == *id).map(|e| e.on);
            let kind = kind.unwrap_or(crate::EventKind::ModuleSaved);
            let event = match event_of(t, kind, dir, module, input.selection.selected.clone()) {
                Ok(event) => event,
                Err(e) => return vec![e],
            };
            let input = Input { selection: Selection::default(), ..input };
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
            said_as_wanted(t, &outcome.said, &mut problems);
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
    if let Some(n) = t.left
        && (t.check.is_some() || t.builtin.is_some())
    {
        if let Err(e) = ws.flush() {
            problems.push(e.to_string());
            return problems;
        }
        // (The check reads the module as fixed, through the game's data
        // too: its 2DAs, its scripts.)
        fixture.layer.set(&ws.module);
        fixture.data.invalidate();
        let after = match (&t.check, &run) {
            (Some(id), _) => {
                let input = Input::new(ws.module.clone(), game);
                crate::run_check(plugin, id, input, host.clone())
            }
            (None, Run::Builtin(b)) => Ok(run_builtin(b, &ws.module, fixture)),
            _ => Ok(Vec::new()),
        };
        fixture.layer.set(module);
        fixture.data.invalidate();
        match after {
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

/// One of Moonglow's own checks, by its id, as Verify and `mg verify
/// --json` name it.
fn builtin_check(id: &str) -> Result<mg_module::doctor::Check, String> {
    use mg_module::doctor::Check;
    Check::ALL.into_iter().find(|c| c.id() == id).ok_or_else(|| {
        let ids: Vec<&str> = Check::ALL.iter().map(|c| c.id()).collect();
        format!(
            "builtin: Moonglow has no check {id:?} (its checks: {}; \"all\" for every one)",
            ids.join(", ")
        )
    })
}

/// The checks a test's `builtin` names: one, several (each once, in the
/// order given), or `"all"`.
fn builtin_checks(b: &BuiltinSetting) -> Result<Builtins, String> {
    use mg_module::doctor::Check;
    let ids: Vec<&str> = match b {
        BuiltinSetting::One(all) if all == "all" => {
            return Ok(Builtins { checks: Check::ALL.to_vec(), named: false });
        }
        BuiltinSetting::One(id) => vec![id],
        BuiltinSetting::Many(ids) => ids.iter().map(String::as_str).collect(),
    };
    if ids.is_empty() {
        return Err("builtin: name a check, several in a list, or \"all\"".into());
    }
    let mut checks = Vec::new();
    for id in ids {
        let check = builtin_check(id)?;
        if !checks.contains(&check) {
            checks.push(check);
        }
    }
    Ok(Builtins { checks, named: true })
}

/// What a test's builtin checks that count talk-table strings say of a
/// test whose game data has no talk tables to count them by (no game, no
/// `dialog.tlk` in its game folder), and whether it is a problem: a
/// check so named by the test can't test what it is for (`2da-strref`;
/// `custom-tlk` where the module's own table isn't in the game data
/// either); those run as part of more are told so in a note.
fn uncounted(b: &Builtins, fixture: &Fixture, from: &GameFrom) -> Option<(String, bool)> {
    use mg_module::doctor::Check;
    if fixture.talk {
        return None;
    }
    let custom = (fixture.module.custom_tlk().ok().flatten())
        .filter(|n| !n.trim().is_empty() && fixture.data.custom_tlk().is_none());
    let blind = |c: &Check| match c {
        Check::TwoDaStrRef | Check::TwoDaDescribed => true,
        Check::CustomTlk => custom.is_some(),
        _ => false,
    };
    let checks: Vec<&str> = b.checks.iter().filter(|c| blind(c)).map(|c| c.id()).collect();
    if checks.is_empty() {
        return None;
    }
    let wrong = b.named
        && (b.checks.iter())
            .any(|c| matches!(c, Check::TwoDaStrRef) || matches!(c, Check::CustomTlk) && blind(c));
    let folder = from.folder.map(|f| f.strip_prefix(from.dir).unwrap_or(f).display().to_string());
    let (tables, them) = match &custom {
        Some(name) => (format!("neither dialog.tlk nor the module's {name}.tlk"), "them"),
        None => ("no dialog.tlk".to_string(), "it"),
    };
    let (has, put) = match &folder {
        Some(f) => (format!("its game folder, {f}, has {tables}"), f.clone()),
        None => (
            format!("it has no game folder (and so {tables})"),
            format!("a folder {GAME_FOLDER}/ beside the test file"),
        ),
    };
    let game = match from.root {
        Root::No => "the test reads no game, root = false",
        _ => "there is no game install",
    };
    let what = if checks.len() == 1 { "counts" } else { "count" };
    Some((
        format!(
            "{} {what} no talk-table strings: {game}, and {has}: put {them} in {put}, or name a \
             game with root = \"…\"",
            checks.join(", ")
        ),
        wrong,
    ))
}

/// Moonglow's own checks (Verify's) run on the module, with the game's
/// data (the module layered in) as Verify runs them: by the module's
/// project file (`moonglow.toml`, `variables.toml`), their findings with
/// the fixes Verify offers. A talk table the test's game data has not is
/// not counted (none of its strings is said to be missing).
fn run_builtin(b: &Builtins, module: &Module, fixture: &Fixture) -> Vec<Found> {
    use mg_module::doctor::{Check, Group, Skip, TalkTables, Timings, examine_groups};
    let g = &fixture.data;
    let custom = match g.custom_tlk() {
        Some(t) => Some(t.entries.len()),
        None if !fixture.talk && module.custom_tlk().ok().flatten().is_some() => {
            Some(TalkTables::UNCOUNTED)
        }
        None => None,
    };
    let tlk = TalkTables { base: if fixture.talk { g.tlk().entries.len() } else { 0 }, custom };
    // (Only these checks run: the others are most of a Verify's time.)
    let others = (Check::ALL.into_iter()).filter(|c| !b.checks.contains(c)).map(Check::id);
    let skip = Skip::parse(&others.chain([Group::Plugins.id()]).collect::<Vec<_>>())
        .expect("the checks' ids");
    let findings: Vec<mg_module::doctor::Finding> =
        examine_groups(module, &g.resman, tlk, &skip, &mut Timings::default(), None)
            .into_iter()
            .filter(|f| b.checks.iter().any(|c| f.check == c.id()))
            .collect();
    mg_edit::fix::attach(module, findings, &[])
}

/// What happened, for a test of an event: the resources it names (or
/// those of a save: that differ from `was`, else all), the objects it
/// selects (placed or pasted), what made them.
fn event_of(
    t: &Test,
    kind: crate::EventKind,
    dir: &Path,
    module: &Module,
    selected: Vec<Selected>,
) -> Result<Event, String> {
    use crate::EventKind as K;
    let mut named = Vec::new();
    for name in &t.changed {
        match ResKey::from_filename(name) {
            Some(k) => named.push(k),
            None => return Err(format!("changed: {name:?} is not a resource's name")),
        }
    }
    let mut event = match kind {
        K::ResourceSaving | K::ResourceSaved => {
            let was = match &t.was {
                Some(w) => {
                    let path = dir.join(w);
                    Some(Module::open(&path).map_err(|e| format!("was: {}: {e}", path.display()))?)
                }
                None => None,
            };
            let before = |k: &ResKey| match &was {
                Some(w) => w.get(k),
                // (No module before: each as it is.)
                None => module.get(k),
            };
            let gone: Vec<ResKey> = was.iter().flat_map(|w| w.keys().copied()).collect();
            let mut e = Event::saving(kind, module, before, gone);
            if was.is_none() {
                e.resources = module.keys().copied().collect();
                e.resources.sort();
                for k in &e.resources {
                    if let Some(b) = module.get(k) {
                        e.before.insert(*k, Arc::from(b));
                    }
                }
            }
            if !named.is_empty() {
                e.resources.retain(|k| named.contains(k));
                if let Some(k) = named.iter().find(|k| !e.resources.contains(k)) {
                    return Err(format!("changed: {k} is not written by the save"));
                }
            }
            e
        }
        K::ResourceCreated if named.is_empty() => {
            return Err("changed: name the resources made".into());
        }
        K::ObjectPlaced | K::ObjectPasted => {
            if selected.is_empty() || selected.iter().any(|s| !matches!(s, Selected::Object { .. }))
            {
                return Err(
                    "select: the objects placed or pasted (tavern.git/Placeable List[0])".into()
                );
            }
            let mut e = Event::new(kind, Vec::new());
            e.objects = selected;
            e
        }
        K::ModuleSaved if named.is_empty() => {
            let mut all: Vec<ResKey> = module.keys().copied().collect();
            all.sort();
            Event::new(kind, all)
        }
        _ => Event::new(kind, named),
    };
    event.by = t.by.clone().unwrap_or_default();
    Ok(event)
}

/// What a resource saving's handler said against `warned` and `refused`.
fn said_as_wanted(t: &Test, said: &[crate::Said], problems: &mut Vec<String>) {
    for (wanted, refused, what) in [(&t.warned, false, "warnings"), (&t.refused, true, "refusals")]
    {
        let Some(wanted) = wanted else { continue };
        let got: Vec<String> = (said.iter())
            .filter(|s| s.refused == refused)
            .map(|s| format!("{}: {}", s.resource, s.text))
            .collect();
        let fits = got.len() == wanted.len()
            && got.iter().zip(wanted).all(|(g, w)| g.contains(w.as_str()));
        if !fits {
            problems.push(format!("the {what} are {got:?}, not {wanted:?}"));
        }
    }
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
            guard.check(ws, fix).and_then(|now| ws.apply(now.command()).map_err(|e| e.to_string()));
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
    // A list's item (`Equip_ItemList[2]`, `/VarTable[0]`): there or not.
    let full = if f.field.starts_with('/') { f.field.clone() } else { format!("/{}", f.field) };
    if let Ok(path) = full.parse::<GffPath>()
        && let Some(Step::Item(..)) = path.0.last()
    {
        if f.value.is_some() {
            return Err(format!("{at}: a list's item has no value: name a field of it"));
        }
        let doc = ws.doc(&key).map_err(|e| format!("{at}: {e}"))?;
        return match (path.get(&doc.root).is_some(), f.absent) {
            (false, true) | (true, false) => Ok(()),
            (true, true) => Err(format!("{at} is there, and was to be gone")),
            (false, false) => Err(format!("{at} is not there")),
        };
    }
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
