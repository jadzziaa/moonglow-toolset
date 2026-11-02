//! Plugins in the window: those installed in Moonglow's plugins folder,
//! which of them are enabled, their commands (run as jobs, their edits one
//! undoable command) and their checks (run with Verify Module).
//!
//! A plugin is off until enabled here. Its code runs sandboxed
//! (`mg_plugin`): what it can do is read the open module and the game
//! data, hand back edits, write to the log and ask the user something,
//! which the job's window shows.
//!
//! A project's own plugins (`plugins/` of its checkout) are listed beside
//! those installed, and run only once the user allows them: when the
//! project is opened, Moonglow asks about each it has no answer for, or
//! that changed since it was allowed. A plugin's settings are edited in a
//! form of their own, and kept where each one's scope says.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use mg_edit::Command;
use mg_module::project_file::ProjectFile;
use mg_plugin::Found;
use mg_plugin::{
    Answer, Existing, FieldKind, Host, Input, Outcome, Package, Plugin, PluginError, Question,
};

use crate::jobs::{Context, Progress};
use crate::{Action, Level, Moonglow};

/// A plugin found in the plugins folder: read, or why it could not be.
#[derive(Debug, Clone)]
pub struct Installed {
    pub dir: PathBuf,
    pub plugin: Result<Plugin, String>,
    /// Installed from an archive (Install from File): Moonglow's to
    /// replace and remove. Else it was put there by hand, and is not.
    pub from_archive: bool,
    /// One of the open project's own plugins (its `plugins/`): it runs
    /// only as the user allowed it, and is not Moonglow's to remove.
    pub project: bool,
}

/// A plugin's settings in their form: whose, and the values in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsForm {
    pub plugin: String,
    pub values: std::collections::BTreeMap<String, mg_plugin::SettingValue>,
}

/// The plugins as the window has them.
#[derive(Debug, Default)]
pub struct Plugins {
    pub installed: Vec<Installed>,
    /// The Plugins window is open.
    pub window: bool,
    /// The console's code, and what its last run said.
    pub console: String,
    pub console_output: Vec<(Level, String)>,
    /// Answers for what the next plugin asks, in order, where there is no
    /// window to ask in (`None`: closed unanswered).
    pub answers: VecDeque<Option<Answer>>,
    /// Where a running plugin's question waits for its answer.
    ask: Option<Arc<Ask>>,
    /// The form being filled: the question it belongs to, and its values.
    form: Option<(Question, Vec<serde_json::Value>)>,
    /// An archive chosen to install whose plugin is installed already:
    /// asked before it is replaced; and, of an archive of several, those
    /// to ask about after it.
    pub replace: Option<Replace>,
    pub replace_next: VecDeque<Replace>,
    /// What the last install or removal came to, shown in the window.
    pub install_note: Option<(Level, String)>,
    /// A plugin chosen to remove: asked before its folder is deleted.
    pub remove: Option<Remove>,
    /// What happened that enabled plugins act on, each run as a job when
    /// no other is running: the plugin's id, its event's id, the event.
    events: VecDeque<(String, String, mg_plugin::Event)>,
    /// The module as it was last saved or opened, to tell what a save
    /// changed.
    pub(crate) saved: Option<mg_module::Module>,
    /// A save held for what the plugins said of it (`resource-saving`).
    pub save: crate::plugin_events::SaveState,
    /// What each plugin's events were allowed (a folder chosen, a yes to
    /// writing into it), by plugin id: an event that comes again does not
    /// ask again. Kept between runs in the settings (`plugin_grants`).
    granted: std::collections::HashMap<String, Granted>,
    /// The project's plugins the user is being asked about (their ids),
    /// in the question that comes when the project is opened. Emptied
    /// when each is answered, or all are put off (Decide Later).
    pub project_asking: Vec<String>,
    /// A plugin's settings being edited.
    pub settings_form: Option<SettingsForm>,
    /// What the job that just ended set, until it is kept.
    last_set: Option<Vec<mg_plugin::SettingWrite>>,
    /// What the user answered about projects' own plugins, as kept in
    /// Moonglow's own folder ([`Moonglow::plugin_answers`]).
    pub trust_answers: mg_plugin::project::Answers,
    /// The project's root and that root canonical, as its plugins were
    /// read (the trust answers are kept by the canonical path).
    project_root: Option<(std::path::PathBuf, std::path::PathBuf)>,
    /// The haks plugins' commands wrote, by the step of Undo of the
    /// command's edits: Undo takes them back, Redo writes them again.
    pub(crate) hak_steps: Vec<(u64, Vec<mg_plugin::HakWritten>)>,
    /// The project's plugins' folder as it was when its plugins were read
    /// (each file's path, size and time), to see that it changed.
    project_stamp: Option<u64>,
    /// The pane last clicked in, whose selection a command from the
    /// Plugins menu, the Command Palette or a key runs on.
    pub pane: Option<Pane>,
    /// The open panels (API 0.6), by plugin and panel id: what each
    /// showed last, and its render under way.
    pub panels: std::collections::HashMap<(String, String), crate::plugin_kinds::PanelState>,
}

/// A pane whose selection a plugin's command may run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Tree,
    Palette,
    Area(mg_core::ResRef),
}

/// The most seconds the handlers of an event that runs at once may be
/// given.
const MOST_EVENT_SECONDS: u32 = 120;

/// What a folder of plugins holds, as a number that changes when a file
/// in it is added, removed, written or renamed (none: no such folder).
fn folder_stamp(dir: &Path) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut todo = vec![(dir.to_path_buf(), 0)];
    let mut seen = 0usize;
    std::fs::metadata(dir).ok()?;
    while let Some((d, depth)) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            seen += 1;
            // (A plugin is a few files: a folder of thousands is no plugins'.)
            if seen > 20_000 {
                return Some(h.finish());
            }
            let Ok(m) = e.metadata() else { continue };
            e.path().hash(&mut h);
            m.len().hash(&mut h);
            m.modified().ok().hash(&mut h);
            if m.is_dir() && depth < 8 {
                todo.push((e.path(), depth + 1));
            }
        }
    }
    Some(h.finish())
}

/// Answers remembered, by question.
type Granted = Arc<Mutex<std::collections::HashMap<String, Answer>>>;

/// A plugin as its grants are kept under: another version asks again (a
/// project's plugin, other content).
fn grants_name(p: &Plugin) -> String {
    match p.digest() {
        // (And by where it is: another project's of the same content is
        // another plugin.)
        Some(digest) => {
            let dir = p.dir.canonicalize().unwrap_or_else(|_| p.dir.clone());
            format!("{} {digest} {}", p.manifest.id, dir.display())
        }
        None => format!("{} {}", p.manifest.id, p.manifest.version),
    }
}

/// What the settings keep of what `plugin` ([`grants_name`]) was allowed.
fn kept_grants(
    kept: &[(String, String, String)],
    plugin: &str,
) -> std::collections::HashMap<String, Answer> {
    (kept.iter().filter(|(p, _, _)| p == plugin))
        .filter_map(|(_, question, answer)| {
            let answer = match answer.strip_prefix("path:") {
                Some(path) => Answer::Path(path.into()),
                None if answer == "yes" => Answer::Yes,
                None => return None,
            };
            Some((question.clone(), answer))
        })
        .collect()
}

/// Puts what `plugin` was allowed into the settings' list, in place of
/// what was there of it; whether the list changed.
fn keep_grants(
    kept: &mut Vec<(String, String, String)>,
    plugin: &str,
    granted: &std::collections::HashMap<String, Answer>,
) -> bool {
    let mut now: Vec<(String, String, String)> = granted
        .iter()
        .filter_map(|(question, answer)| {
            let answer = match answer {
                Answer::Yes => "yes".to_string(),
                Answer::Path(p) => format!("path:{}", p.display()),
                _ => return None,
            };
            Some((plugin.to_string(), question.clone(), answer))
        })
        .collect();
    now.sort();
    let mut before: Vec<_> = kept.iter().filter(|(p, _, _)| p == plugin).cloned().collect();
    before.sort();
    if before == now {
        return false;
    }
    kept.retain(|(p, _, _)| p != plugin);
    kept.extend(now);
    true
}

/// An installed plugin waiting for a yes to be removed.
#[derive(Debug, Clone)]
pub struct Remove {
    pub dir: PathBuf,
    /// Its name and version (its folder's name, if its manifest does not
    /// read).
    pub name: String,
    pub id: Option<String>,
}

/// A plugin's archive waiting for a yes to replace the one installed.
#[derive(Debug, Clone)]
pub struct Replace {
    pub package: Package,
    /// The archive's file name.
    pub file: String,
    /// The version installed (empty if its manifest does not read).
    pub installed: String,
}

/// A command a plugin adds, as the menus, the keys and the Command Palette
/// show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCommand {
    pub plugin: String,
    pub plugin_name: String,
    pub command: String,
    pub title: String,
    pub hint: String,
    /// The id its keys are kept under: the plugin's id, a slash, its own.
    pub key_id: String,
    /// The key its manifest asks for.
    pub key: Option<String>,
    /// What it is for: the right-click menus it is in (none: the Plugins
    /// menu alone).
    pub targets: Vec<mg_plugin::Target>,
}

impl PluginCommand {
    /// Whether it is for every one of `kinds` (and there are some): a
    /// right-click menu on them has it. A kind is what the menu is on (a
    /// placed object `Placed`, a blueprint `Blueprint`), which a command
    /// for both takes ([`mg_plugin::Target::takes`]).
    pub fn fits(&self, kinds: &[mg_plugin::Target]) -> bool {
        !kinds.is_empty() && kinds.iter().all(|k| self.targets.iter().any(|t| t.takes(*k)))
    }
}

/// The kinds of what is selected, as a menu on them has commands for
/// ([`PluginCommand::fits`]); none where one of them is of no kind.
pub(crate) fn kinds_of(selected: &[mg_plugin::Selected]) -> Vec<mg_plugin::Target> {
    let mut kinds: Vec<mg_plugin::Target> =
        selected.iter().map(|s| s.target()).collect::<Option<_>>().unwrap_or_default();
    kinds.sort();
    kinds.dedup();
    kinds
}

/// What a place in a resource is, as a plugin's selection: an object
/// placed in an area (a GIT's list item), an area, a blueprint or another
/// resource of an editor of its own (Find References' and Advanced
/// Search's rows).
pub(crate) fn selected_at(key: mg_resman::ResKey, path: &mg_edit::GffPath) -> mg_plugin::Selected {
    use mg_core::ResType;
    use mg_plugin::Selected;
    match (key.restype, path.0.first()) {
        (ResType::GIT, Some(mg_edit::Step::Item(list, index))) => {
            match mg_area::ObjectKind::from_list(list) {
                Some(kind) => Selected::Object { area: key.resref, kind, index: *index },
                None => Selected::Area(key.resref),
            }
        }
        (ResType::ARE | ResType::GIT | ResType::GIC, _) => Selected::Area(key.resref),
        (t, _) if mg_area::ObjectKind::from_restype(t).is_some() => Selected::Blueprint(key),
        _ => Selected::Resource(key),
    }
}

/// What a tab edits, as a plugin's selection: the object, blueprint or
/// area of a Properties window (each of several edited together), or the
/// resource of another editor; none for the rest.
pub(crate) fn tab_selection(tab: &crate::Tab) -> Vec<mg_plugin::Selected> {
    use crate::Tab;
    use mg_plugin::Selected;
    let root = mg_edit::GffPath::root();
    let object = |area: mg_core::ResRef, path: &mg_edit::GffPath| {
        Some(crate::plugins::selected_at(mg_resman::ResKey::new(area, mg_core::ResType::GIT), path))
            .filter(|s| matches!(s, Selected::Object { .. }))
    };
    match tab {
        Tab::Blueprint(key) => vec![Selected::Blueprint(*key)],
        Tab::Blueprints(keys) => keys.iter().map(|k| Selected::Blueprint(*k)).collect(),
        Tab::Instance { area, path } => object(*area, path).into_iter().collect(),
        Tab::Instances { area, paths } => paths.iter().filter_map(|p| object(*area, p)).collect(),
        Tab::Area(area) | Tab::AreaProperties(area) => vec![Selected::Area(*area)],
        Tab::AreasProperties(areas) => areas.iter().map(|a| Selected::Area(*a)).collect(),
        Tab::Script(key) | Tab::Dialog(key) | Tab::Nui(key) | Tab::Gff(key) => {
            vec![selected_at(*key, &root)]
        }
        Tab::ModuleProperties => vec![Selected::Resource(
            mg_core::ResRef::from_str("module")
                .map(|r| mg_resman::ResKey::new(r, mg_core::ResType::IFO))
                .unwrap_or_else(|_| unreachable!()),
        )],
        _ => Vec::new(),
    }
}

/// The plugins' commands for `selected` in a right-click menu (as
/// [`menu_commands`]), the one chosen run on it.
pub(crate) fn menu_on(
    ui: &mut egui::Ui,
    commands: &[PluginCommand],
    selected: &[mg_plugin::Selected],
    actions: &mut Vec<Action>,
) {
    if let Some(i) = menu_commands(ui, commands, &kinds_of(selected)) {
        actions.push(run_on(&commands[i], selected.to_vec()));
    }
}

/// The plugins' commands for what a right-click menu is on (`kinds`),
/// after a separator, as the menu's last group before Variables and
/// Properties: the one chosen, by its place in `commands`.
pub(crate) fn menu_commands(
    ui: &mut egui::Ui,
    commands: &[PluginCommand],
    kinds: &[mg_plugin::Target],
) -> Option<usize> {
    let mut chosen = None;
    let mut first = true;
    for (i, c) in commands.iter().enumerate().filter(|(_, c)| c.fits(kinds)) {
        if std::mem::take(&mut first) {
            ui.separator();
        }
        let tip = match c.hint.as_str() {
            "" => format!("A command of the plugin {}", c.plugin_name),
            hint => format!("{hint}\nA command of the plugin {}", c.plugin_name),
        };
        if ui.button(&c.title).on_hover_text(tip).clicked() {
            chosen = Some(i);
            ui.close();
        }
    }
    chosen
}

/// The action that runs `command` on `selected` (what its menu was opened
/// on).
pub(crate) fn run_on(command: &PluginCommand, selected: Vec<mg_plugin::Selected>) -> Action {
    Action::RunPluginCommand {
        plugin: command.plugin.clone(),
        command: command.command.clone(),
        selected,
    }
}

/// Where the answers about projects' own plugins are kept: a file of
/// Moonglow's own data folder (`mg` reads the same), never a project's.
pub fn answers_path() -> Option<PathBuf> {
    mg_plugin::project::answers_path()
}

/// Where the values of plugins' settings of scope `user` are kept, which
/// `mg` reads and keeps too: `plugin-settings.json` beside the plugins.
pub fn values_path() -> Option<PathBuf> {
    mg_plugin::settings::user_path()
}

/// The plugins enabled now, for a crash report (the panic hook reads it).
static ENABLED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The ids of the plugins enabled when this is called.
pub fn enabled_now() -> Vec<String> {
    ENABLED.lock().map(|e| e.clone()).unwrap_or_default()
}

/// A question a plugin's job waits on, and its answer.
#[derive(Debug, Default)]
struct Ask {
    /// The question, and once it is given the answer (`None`: closed).
    state: Mutex<(Option<Question>, Option<Option<Answer>>)>,
    changed: Condvar,
}

impl Ask {
    /// Asks, on the job's thread, and waits for the window to answer (or
    /// for the job to be called off).
    fn ask(&self, question: Question, progress: &Progress) -> Option<Answer> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        *state = (Some(question), None);
        loop {
            if let Some(answer) = state.1.take() {
                state.0 = None;
                return answer;
            }
            if progress.cancelled() {
                state.0 = None;
                return None;
            }
            let wait = self.changed.wait_timeout(state, Duration::from_millis(100));
            state = wait.unwrap_or_else(|e| e.into_inner()).0;
        }
    }

    fn pending(&self) -> Option<Question> {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).0.clone()
    }

    fn answer(&self, answer: Option<Answer>) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).1 = Some(answer);
        self.changed.notify_all();
    }
}

/// What a plugin logged, to put into the window's log when its job ends.
type Logged = Arc<Mutex<Vec<(mg_plugin::Level, String)>>>;

/// A job as a plugin's host.
struct JobHost {
    progress: Arc<Progress>,
    log: Logged,
    ask: Option<Arc<Ask>>,
    answers: RefCell<VecDeque<Option<Answer>>>,
    /// An event's job: what was allowed before is not asked again.
    granted: Option<Granted>,
}

impl Host for JobHost {
    fn log(&self, level: mg_plugin::Level, text: &str) {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).push((level, text.to_string()));
    }

    fn progress(&self, done: usize, total: usize, note: &str) {
        self.progress.step(done, total);
        if !note.is_empty() {
            self.progress.say(note);
        }
    }

    fn cancelled(&self) -> bool {
        self.progress.cancelled()
    }

    /// The job's window asks (a wizard's page asked again, saying what its
    /// check objects to).
    fn interactive(&self) -> bool {
        self.ask.is_some() && self.answers.borrow().is_empty()
    }

    fn ask(&self, question: &Question) -> Option<Answer> {
        // (A folder chosen, a yes: not what a form was filled with.)
        let lasting = matches!(question, Question::Folder { .. } | Question::Confirm(_));
        let key = format!("{question:?}");
        if let Some(granted) = self.granted.as_ref().filter(|_| lasting)
            && let Some(answer) = granted.lock().unwrap_or_else(|e| e.into_inner()).get(&key)
        {
            return Some(answer.clone());
        }
        let answer = match self.answers.borrow_mut().pop_front() {
            Some(answer) => answer,
            None => self.ask.as_ref()?.ask(question.clone(), &self.progress),
        };
        if let Some(granted) = self.granted.as_ref().filter(|_| lasting)
            && let Some(answer @ (Answer::Yes | Answer::Path(_))) = &answer
        {
            granted.lock().unwrap_or_else(|e| e.into_inner()).insert(key, answer.clone());
        }
        answer
    }
}

/// A plugin's log level as the log's.
pub(crate) fn level(l: mg_plugin::Level) -> Level {
    match l {
        mg_plugin::Level::Info => Level::Info,
        mg_plugin::Level::Warning => Level::Warning,
        mg_plugin::Level::Error => Level::Error,
    }
}

/// The findings of the enabled plugins' checks (each with its fix, if it
/// has one), on a job's snapshot, with each plugin's settings, and what the
/// checks logged. A check that fails is itself a finding.
pub(crate) fn check_findings(
    plugins: &[(Plugin, mg_plugin::Settings)],
    skip: &mg_module::doctor::Skip,
    job: &Context,
) -> (Vec<Found>, Vec<(Level, String)>) {
    let (mut findings, mut said) = (Vec::new(), Vec::new());
    for (plugin, settings) in plugins {
        for check in &plugin.manifest.checks {
            // (One Options › Verify leaves out isn't run.)
            if skip.plugin_check(&plugin.check_id(&check.id)) {
                continue;
            }
            if job.progress.cancelled() {
                return (findings, said);
            }
            job.progress.say(format!("{}: {}", plugin.manifest.name, check.title));
            let log = Logged::default();
            let host = Rc::new(JobHost {
                progress: job.progress.clone(),
                log: log.clone(),
                ask: None,
                answers: RefCell::default(),
                granted: None,
            });
            let input = Input {
                settings: Some(settings.clone()),
                ..Input::new(job.module.clone(), job.game.clone())
            };
            match mg_plugin::run_check(plugin, &check.id, input, host) {
                Ok(found) => findings.extend(found),
                Err(PluginError::Canceled(_)) => return (findings, said),
                Err(e) => findings.push(plugin.check_failed(&check.id, &e).into()),
            }
            let logged = std::mem::take(&mut *log.lock().unwrap_or_else(|e| e.into_inner()));
            let name = &plugin.manifest.name;
            said.extend(logged.into_iter().map(|(l, text)| (level(l), format!("{name}: {text}"))));
        }
    }
    (findings, said)
}

impl Moonglow {
    /// Reads the plugins folder again: who is installed (none of their
    /// code runs).
    pub fn load_plugins(&mut self) {
        self.read_plugin_values();
        self.plugins.installed.clear();
        if !self.no_plugins
            && let Some(dir) = &self.plugin_dir
        {
            for dir in mg_plugin::folders(dir) {
                let plugin = Plugin::load(&dir).map_err(|e| e.to_string());
                let from_archive = mg_plugin::from_archive(&dir);
                self.plugins.installed.push(Installed {
                    dir,
                    plugin,
                    from_archive,
                    project: false,
                });
            }
        }
        self.read_project_plugins();
        self.plugins_changed();
    }

    /// The open project's root (a module folder, or a nasher project's):
    /// none for a module kept in one file.
    pub fn project_root(&self) -> Option<PathBuf> {
        let ws = self.ws.as_ref()?;
        let root = ws.module.location.as_ref().and_then(ProjectFile::root_of)?;
        Some(root.to_path_buf())
    }

    /// Reads the open project's plugins again (none of their code runs),
    /// and asks about those it has no answer for.
    pub fn load_project_plugins(&mut self) {
        self.read_project_plugins();
        self.plugins_changed();
    }

    fn read_project_plugins(&mut self) {
        self.plugins.installed.retain(|i| !i.project);
        self.plugins.project_asking.clear();
        self.plugins.project_root = None;
        self.plugins.project_stamp = None;
        let Some(root) = self.project_root().filter(|_| !self.no_plugins) else { return };
        self.plugins.project_stamp =
            self.project_plugins_folder().as_deref().and_then(folder_stamp);
        self.plugins.project_root = Some((root.clone(), mg_plugin::project::canonical(&root)));
        // What the user answered, read again (another Moonglow, or `mg`,
        // may have answered since); one that can't be read allows nothing.
        if let Some(path) = &self.plugin_answers {
            match mg_plugin::project::Answers::read(path) {
                Ok(a) => self.plugins.trust_answers = a,
                Err(e) => {
                    self.plugins.trust_answers = Default::default();
                    self.log.error(format!("{e}: none of a project's plugins runs until it reads"));
                }
            }
        }
        let Some(ws) = &self.ws else { return };
        let found = mg_plugin::project::discover(&root, &ws.module.project_file);
        for f in found {
            if let Err(e) = &f.plugin {
                self.log.warn(format!("The project's plugin in {}: {e}", f.dir.display()));
            }
            let plugin = f.plugin;
            self.plugins.installed.push(Installed {
                dir: f.dir,
                plugin,
                from_archive: false,
                project: true,
            });
        }
        let asking: Vec<String> = (self.plugins.installed.iter())
            .filter(|i| i.project)
            .filter_map(|i| i.plugin.as_ref().ok())
            .filter(|p| self.project_trust(p).is_some_and(|t| t.to_ask()))
            .map(|p| p.manifest.id.clone())
            .collect();
        self.plugins.project_asking = asking;
        // What the project's files hold of plugins' settings that is not
        // as their manifests say.
        let Some(ws) = &self.ws else { return };
        let file = &ws.module.project_file;
        let plugins: Vec<&Plugin> =
            self.plugins.installed.iter().filter_map(|i| i.plugin.as_ref().ok()).collect();
        let mut said = Vec::new();
        for p in &plugins {
            let user = self.user_settings(&p.manifest.id);
            said.extend(mg_plugin::settings::resolve(&p.manifest, &user, Some(file)).1);
        }
        let known: Vec<&str> = plugins.iter().map(|p| p.manifest.id.as_str()).collect();
        said.extend(mg_plugin::settings::unclaimed(file, &known));
        said.dedup();
        for s in said {
            self.log.warn(s);
        }
    }

    /// How long the handlers of an event that runs at once may take
    /// (Manage Plugins; the default [`mg_plugin::EVENT_BUDGET`]).
    pub fn plugin_event_budget(&self) -> std::time::Duration {
        match self.settings.plugin_event_seconds {
            0 => mg_plugin::EVENT_BUDGET,
            s => std::time::Duration::from_secs(u64::from(s.clamp(1, MOST_EVENT_SECONDS))),
        }
    }

    /// The open project's plugins' folder.
    fn project_plugins_folder(&self) -> Option<PathBuf> {
        let root = self.project_root()?;
        let ws = self.ws.as_ref()?;
        Some(mg_plugin::project::folder(&root, &ws.module.project_file))
    }

    /// Whether the open project's plugins' folder changed on disk since its
    /// plugins were read (a pull, a plugin added or edited): then they are
    /// read again, and those not as they were allowed are asked about (and
    /// off until then). Looked at every few seconds while the project is
    /// open, between jobs.
    pub fn project_plugins_changed(&mut self) -> bool {
        if self.no_plugins || self.busy() {
            return false;
        }
        let Some(dir) = self.project_plugins_folder() else { return false };
        let now = folder_stamp(&dir);
        if now == self.plugins.project_stamp {
            return false;
        }
        let before: Vec<(String, Option<String>)> = (self.project_plugins().into_iter())
            .map(|p| (p.manifest.id.clone(), p.digest().map(str::to_string)))
            .collect();
        self.load_project_plugins();
        let after: Vec<(String, Option<String>)> = (self.project_plugins().into_iter())
            .map(|p| (p.manifest.id.clone(), p.digest().map(str::to_string)))
            .collect();
        if before != after {
            self.log.info(format!(
                "The project's plugins changed in {}: read again{}",
                dir.display(),
                if self.plugins.project_asking.is_empty() {
                    String::new()
                } else {
                    " (those not as you allowed them are off until you answer)".to_string()
                }
            ));
        }
        true
    }

    /// What the user answered for a project's plugin, as it is now (none:
    /// it is no project's plugin, or no project is open).
    pub fn project_trust(&self, p: &Plugin) -> Option<mg_plugin::project::Trust> {
        let root = self.project_root().filter(|_| p.is_project())?;
        let answers = &self.plugins.trust_answers;
        Some(match &self.plugins.project_root {
            Some((r, canonical)) if *r == root => answers.trust_in(canonical, p),
            _ => answers.trust(&root, p),
        })
    }

    /// The project's plugins.
    pub fn project_plugins(&self) -> Vec<&Plugin> {
        (self.plugins.installed.iter())
            .filter(|i| i.project)
            .filter_map(|i| i.plugin.as_ref().ok())
            .collect()
    }

    /// Allows one of the project's plugins as it is now, or keeps it off
    /// (`allow` false): the answer kept in the user's own part of the
    /// project, and asked again only if it was a yes and the plugin
    /// changes.
    pub fn allow_project_plugin(&mut self, id: &str, allow: bool) {
        let Some(root) = self.project_root() else { return };
        let found = self.project_plugins().into_iter().find(|p| p.manifest.id == id).cloned();
        let Some(p) = found else { return };
        let m = &p.manifest;
        // (Kept in Moonglow's own folder, never in the project.)
        match &self.plugin_answers {
            Some(path) => match mg_plugin::project::decide(path, &root, &p, allow) {
                Ok(answers) => self.plugins.trust_answers = answers,
                Err(e) => {
                    self.log.error(format!("{}: your answer was not kept: {e}", m.name));
                    return;
                }
            },
            None => self.plugins.trust_answers.decide(&root, &p, allow),
        }
        self.plugins.project_asking.retain(|a| a != id);
        self.plugins_changed();
        if !allow {
            self.log.info(format!(
                "The project's plugin {} {} stays off (Plugins › Manage Plugins… enables it)",
                m.name, m.version
            ));
            return;
        }
        self.log.info(format!(
            "The project's plugin {} {} is allowed: it runs as it is now (if it changes, you \
             are asked again)",
            m.name, m.version
        ));
        // What it does when a module is opened, as if it had been on then.
        let event = mg_plugin::Event::new(mg_plugin::EventKind::ModuleOpened, vec![]);
        for e in m.events.iter().filter(|e| e.on == event.kind) {
            self.plugins.events.push_back((m.id.clone(), e.id.clone(), event.clone()));
        }
    }

    /// Reads the values of plugins' settings of scope `user` from the file
    /// `mg` shares ([`Moonglow::plugin_values`]), which Moonglow's
    /// settings then hold; where there is none yet, it is written from
    /// what Moonglow's settings kept (before 3.3.0, there alone).
    fn read_plugin_values(&mut self) {
        let Some(path) = &self.plugin_values else { return };
        match mg_plugin::settings::read_user(path) {
            Ok(values) if path.exists() => self.settings.plugin_settings = values,
            Ok(_) if self.settings.plugin_settings.is_empty() => {}
            Ok(_) => {
                if let Err(e) =
                    mg_plugin::settings::write_user(path, &self.settings.plugin_settings)
                {
                    self.log.error(format!("Plugins' settings: {e}"));
                }
            }
            Err(e) => self.log.error(format!("Plugins' settings: {e}")),
        }
    }

    /// The values a plugin's settings of scope `user` have: as the file
    /// shared with `mg` has them now (it may have kept one since), else
    /// as Moonglow's settings hold them.
    fn user_settings(
        &self,
        id: &str,
    ) -> std::collections::BTreeMap<String, mg_plugin::SettingValue> {
        let read = self
            .plugin_values
            .as_deref()
            .and_then(|p| p.exists().then(|| mg_plugin::settings::read_user(p).ok()).flatten());
        let all = read.as_ref().unwrap_or(&self.settings.plugin_settings);
        mg_plugin::settings::user_values(all, id)
    }

    /// A plugin's settings for a job: those kept (Moonglow's settings, the
    /// project's files) where they are good, else the defaults.
    pub fn plugin_settings(&self, p: &Plugin) -> mg_plugin::Settings {
        let user = self.user_settings(&p.manifest.id);
        let project = self.project_root().and(self.ws.as_ref()).map(|ws| &ws.module.project_file);
        mg_plugin::settings::resolve(&p.manifest, &user, project).0
    }

    /// Keeps a plugin's settings where each one's scope says: Moonglow's
    /// settings, or the project's files (`None`: back to its default, the
    /// value kept taken out). What was kept goes to the log.
    pub fn keep_plugin_settings(&mut self, p: &Plugin, set: &[mg_plugin::SettingWrite]) {
        let root = self.project_root();
        let mut wrote = false;
        for (id, value) in set {
            let Some(d) = p.setting(id) else { continue };
            let shown = value
                .as_ref()
                .map_or_else(|| format!("its default ({})", d.default), |v| v.to_string());
            let kept = match (d.scope, &root) {
                // (In the file shared with `mg`, where there is one.)
                (mg_plugin::Scope::User, _) if self.plugin_values.is_some() => {
                    let path = self.plugin_values.clone().unwrap_or_default();
                    mg_plugin::settings::keep_user(&path, &p.manifest.id, id, value.as_ref())
                        .map(|all| self.settings.plugin_settings = all)
                }
                (mg_plugin::Scope::User, _) => {
                    let all =
                        self.settings.plugin_settings.entry(p.manifest.id.clone()).or_default();
                    match value {
                        Some(v) => all.insert(id.clone(), mg_plugin::settings::to_json(v)),
                        None => all.remove(id),
                    };
                    if all.is_empty() {
                        self.settings.plugin_settings.remove(&p.manifest.id);
                    }
                    Ok(())
                }
                (_, Some(root)) => {
                    wrote = true;
                    mg_plugin::settings::keep_in_project(root, &p.manifest.id, d, value.as_ref())
                }
                (_, None) => Err("no project is open".to_string()),
            };
            let place = mg_plugin::settings::scope_words(d.scope);
            match kept {
                Ok(()) => self.log.info(format!(
                    "{}: {} is {shown} ({place})",
                    p.manifest.name,
                    d.shown()
                )),
                Err(e) => {
                    self.log.error(format!("{}: {} was not kept: {e}", p.manifest.name, d.shown()))
                }
            }
        }
        if wrote && let (Some(root), Some(ws)) = (&root, &mut self.ws) {
            ws.module.project_file = ProjectFile::read(root);
        }
    }

    /// Runs a plugin's own tests (`tests/*.toml`) as a job, as `mg plugin
    /// test` does: each on its fixture, with the game's data of the
    /// install (or none); the log says what each came to.
    pub fn run_plugin_tests(&mut self, plugin: Plugin) {
        let install = self.install.clone();
        let name = plugin.manifest.name.clone();
        self.start_job_anywhere(
            format!("{name}: Run Tests"),
            move |_| {
                let game = |m: &mg_module::Module, root: Option<&Path>| {
                    let at = match (root, &install) {
                        (Some(root), _) => mg_resman::GameInstall::new(root, None, "en"),
                        (None, Some(gi)) => gi.clone(),
                        (None, None) => return Ok(None),
                    };
                    m.game_data(&at).map(|f| Some(f.game)).map_err(|e| e.to_string())
                };
                mg_plugin::run_tests(&plugin, None, &game)
            },
            move |app, results| {
                let failed = results.iter().filter(|r| !r.passed()).count();
                for r in &results {
                    for p in &r.problems {
                        app.log.error(format!("{name}: FAIL {} › {}: {p}", r.file, r.name));
                    }
                    if r.passed() {
                        app.log.info(format!("{name}: ok {} › {}", r.file, r.name));
                    }
                }
                let n = results.len();
                let said = format!("{name}: {n} tests, {} passed, {failed} failed", n - failed);
                if failed > 0 { app.log.warn(said) } else { app.log.info(said) }
            },
        );
    }

    /// What is kept of a plugin's settings that is not as its manifest
    /// declares (a value not of its kind, of a setting it doesn't declare,
    /// in the other file than its scope's), as the log says it when the
    /// project is opened.
    pub fn plugin_settings_faults(&self, p: &Plugin) -> Vec<String> {
        let user = self.user_settings(&p.manifest.id);
        let project = self.project_root().and(self.ws.as_ref()).map(|ws| &ws.module.project_file);
        let mut faults = mg_plugin::settings::resolve(&p.manifest, &user, project).1;
        // (Of the user's own, what no setting of scope user is.)
        let raw = match self.plugin_values.as_deref().filter(|p| p.exists()) {
            Some(path) => mg_plugin::settings::read_user(path).unwrap_or_default(),
            None => self.settings.plugin_settings.clone(),
        };
        for name in raw.get(&p.manifest.id).into_iter().flatten().map(|(k, _)| k) {
            if !p
                .manifest
                .settings
                .iter()
                .any(|d| d.id == *name && d.scope == mg_plugin::Scope::User)
            {
                faults.push(format!(
                    "Moonglow's settings: {} keeps {name:?}, which is not its",
                    p.manifest.id
                ));
            }
        }
        faults
    }

    /// Tidies what is kept of a plugin's settings
    /// ([`mg_plugin::settings::tidy`]): what was wrong taken out, or moved
    /// where its scope keeps it; what was done goes to the log.
    pub fn tidy_plugin_settings(&mut self, id: &str) {
        let found = (self.plugins.installed.iter())
            .filter_map(|i| i.plugin.as_ref().ok())
            .find(|p| p.manifest.id == id)
            .cloned();
        let Some(p) = found else { return };
        let mut user = match self.plugin_values.as_deref().filter(|p| p.exists()) {
            Some(path) => match mg_plugin::settings::read_user(path) {
                Ok(u) => u,
                Err(e) => {
                    self.log.error(format!("{}: not tidied: {e}", p.manifest.name));
                    return;
                }
            },
            None => self.settings.plugin_settings.clone(),
        };
        let root = self.project_root();
        let file = self.ws.as_ref().map(|ws| ws.module.project_file.clone());
        let project = root.as_deref().zip(file.as_ref());
        let done = match mg_plugin::settings::tidy(&p.manifest, &mut user, project, false) {
            Ok(done) => done,
            Err(e) => {
                self.log.error(format!("{}: not tidied: {e}", p.manifest.name));
                return;
            }
        };
        if let Some(path) = &self.plugin_values
            && let Err(e) = mg_plugin::settings::write_user(path, &user)
        {
            self.log.error(format!("{}: {e}", p.manifest.name));
        }
        self.settings.plugin_settings = user;
        if let (Some(root), Some(ws)) = (&root, &mut self.ws) {
            ws.module.project_file = ProjectFile::read(root);
        }
        for line in &done {
            self.log.info(format!("{}: tidied: {line}", p.manifest.name));
        }
        // (The form, if open, with the values as they now are.)
        if self.plugins.settings_form.as_ref().is_some_and(|f| f.plugin == id) {
            let values = self.plugin_settings(&p).values;
            self.plugins.settings_form = Some(SettingsForm { plugin: id.to_string(), values });
        }
    }

    /// Opens a plugin's settings in their form, with the values they have.
    pub fn open_plugin_settings(&mut self, id: &str) {
        let found = (self.plugins.installed.iter())
            .filter_map(|i| i.plugin.as_ref().ok())
            .find(|p| p.manifest.id == id)
            .cloned();
        let Some(p) = found else { return };
        let values = self.plugin_settings(&p).values;
        self.plugins.settings_form = Some(SettingsForm { plugin: id.to_string(), values });
    }

    /// Keeps what the settings' form holds: each value that is not what
    /// the setting has now (one back at its default is taken out).
    pub fn save_plugin_settings(&mut self) {
        let Some(form) = self.plugins.settings_form.take() else { return };
        let found = (self.plugins.installed.iter())
            .filter_map(|i| i.plugin.as_ref().ok())
            .find(|p| p.manifest.id == form.plugin)
            .cloned();
        let Some(p) = found else { return };
        let now = self.plugin_settings(&p).values;
        let set: Vec<mg_plugin::SettingWrite> = (p.manifest.settings.iter())
            .filter_map(|d| {
                let v = &tidy(form.values.get(&d.id)?);
                if now.get(&d.id) == Some(v) || d.check(v).is_err() {
                    return None;
                }
                Some((d.id.clone(), (*v != d.default).then(|| v.clone())))
            })
            .collect();
        if set.is_empty() {
            return;
        }
        self.keep_plugin_settings(&p, &set);
    }

    /// After the installed or enabled plugins changed: their keys, and
    /// what a crash report would name.
    pub(crate) fn plugins_changed(&mut self) {
        self.keymap_stale = true;
        let enabled = self.enabled_plugins().iter().map(|p| p.manifest.id.clone()).collect();
        if let Ok(mut now) = ENABLED.lock() {
            *now = enabled;
        }
        self.close_stray_panels();
    }

    pub fn plugin_enabled(&self, id: &str) -> bool {
        self.settings.plugins_enabled.iter().any(|e| e == id)
    }

    /// Enables or disables an installed plugin.
    pub fn enable_plugin(&mut self, id: &str, on: bool) {
        self.forget_plugin_grants(id);
        self.settings.plugins_enabled.retain(|e| e != id);
        if on {
            self.settings.plugins_enabled.push(id.to_string());
        }
        self.plugins_changed();
    }

    /// Forgets what a plugin's events were allowed (it asks again): when
    /// it is enabled or disabled, removed, or installed anew.
    pub fn forget_plugin_grants(&mut self, id: &str) {
        self.plugins.granted.remove(id);
        let of = |p: &str| p.split_once(' ').is_some_and(|(i, _)| i == id);
        self.settings.plugin_grants.retain(|(p, _, _)| !of(p));
    }

    /// Forgets what this run of Moonglow holds of what plugins' events
    /// were allowed, as a new start has it: what the settings keep is
    /// read again.
    pub fn forget_plugin_session(&mut self) {
        self.next_plugin_event();
        self.plugins.granted.clear();
    }

    /// How many things a plugin's events were allowed and are not asked
    /// again.
    pub fn plugin_grants(&self, id: &str) -> usize {
        let of = |p: &str| p.split_once(' ').is_some_and(|(i, _)| i == id);
        self.settings.plugin_grants.iter().filter(|(p, _, _)| of(p)).count()
    }

    /// The plugins that run: those installed that are enabled, and the
    /// project's that the user allowed (over one installed of the same id).
    pub fn enabled_plugins(&self) -> Vec<&Plugin> {
        let project: Vec<&Plugin> = (self.project_plugins().into_iter())
            .filter(|p| self.project_trust(p).is_some_and(|t| t.runs()))
            .collect();
        let installed = (self.plugins.installed.iter())
            .filter(|i| !i.project)
            .filter_map(|i| i.plugin.as_ref().ok())
            .filter(|p| self.plugin_enabled(&p.manifest.id))
            .filter(|p| !project.iter().any(|q| q.manifest.id == p.manifest.id));
        installed.chain(project.iter().copied()).collect()
    }

    /// The commands of the enabled plugins, in their plugins' order.
    pub fn plugin_commands(&self) -> Vec<PluginCommand> {
        self.enabled_plugins()
            .into_iter()
            .flat_map(|p| {
                p.manifest.commands.iter().map(|c| PluginCommand {
                    plugin: p.manifest.id.clone(),
                    plugin_name: p.manifest.name.clone(),
                    command: c.id.clone(),
                    title: c.title.clone(),
                    hint: c.hint.clone(),
                    key_id: format!("{}/{}", p.manifest.id, c.id),
                    key: c.key.clone(),
                    targets: c.targets.clone(),
                })
            })
            .collect()
    }

    /// The enabled plugins' commands that are in right-click menus.
    pub(crate) fn plugin_menu_commands(&self) -> Vec<PluginCommand> {
        let mut all = self.plugin_commands();
        all.retain(|c| !c.targets.is_empty());
        all
    }

    /// What is selected in the window, for a command chosen from the
    /// Plugins menu, the Command Palette or a key: the objects selected in
    /// the area view shown last, else the blueprints selected in the
    /// palette.
    pub fn plugin_selection(&self) -> Vec<mg_plugin::Selected> {
        // The pane last clicked in, where something is selected there.
        let from = match self.plugins.pane {
            Some(Pane::Tree) => self.tree_selection(),
            Some(Pane::Palette) => self.palette_selection(),
            Some(Pane::Area(area)) => self.area_selection(area),
            None => Vec::new(),
        };
        if !from.is_empty() {
            return from;
        }
        // Else the area view shown last, else the palette's.
        let area = self.palette.area.map(|a| self.area_selection(a)).unwrap_or_default();
        if !area.is_empty() {
            return area;
        }
        self.palette_selection()
    }

    /// What is selected in an area's view: its objects, or in tile mode
    /// its tiles.
    fn area_selection(&self, area: mg_core::ResRef) -> Vec<mg_plugin::Selected> {
        let Some(view) = self.area_views.get(&area) else { return Vec::new() };
        if !view.selection.is_empty() {
            let object = |&(kind, index): &(mg_area::ObjectKind, usize)| {
                mg_plugin::Selected::Object { area, kind, index }
            };
            return view.selection.iter().map(object).collect();
        }
        if view.tile_mode && !view.tile_selection.is_empty() {
            return self.tiles_of(area, &view.tile_selection);
        }
        Vec::new()
    }

    /// An area's tiles (column, row) as a plugin's selection.
    pub(crate) fn tiles_of(
        &self,
        area: mg_core::ResRef,
        cells: &[(u32, u32)],
    ) -> Vec<mg_plugin::Selected> {
        let key = mg_resman::ResKey::new(area, mg_core::ResType::ARE);
        let width = (self.ws.as_ref())
            .and_then(|ws| ws.loaded(&key).and_then(|g| g.root.integer("Width")))
            .or_else(|| {
                let bytes = self.ws.as_ref()?.module.get(&key)?;
                mg_gff::Gff::read(bytes).ok()?.root.integer("Width")
            })
            .unwrap_or(0)
            .max(0) as usize;
        (cells.iter())
            .map(|&(x, y)| mg_plugin::Selected::Tile {
                area,
                index: y as usize * width + x as usize,
            })
            .collect()
    }

    /// The blueprints chosen in the palette.
    fn palette_selection(&self) -> Vec<mg_plugin::Selected> {
        let p = &self.palette;
        let mut keys: Vec<mg_resman::ResKey> = p.selected.into_iter().collect();
        keys.extend(p.chosen.iter().filter(|k| p.selected != Some(**k)));
        keys.into_iter().map(mg_plugin::Selected::Blueprint).collect()
    }

    /// What is chosen in the module tree: objects of an area's list, or
    /// its rows (areas, blueprints, other resources).
    fn tree_selection(&self) -> Vec<mg_plugin::Selected> {
        if let Some((area, set)) = &self.tree_objects
            && !set.is_empty()
        {
            let area = *area;
            return (set.iter())
                .map(|&(kind, index)| mg_plugin::Selected::Object { area, kind, index })
                .collect();
        }
        let path = mg_edit::GffPath::root();
        self.tree_selected.iter().map(|k| selected_at(*k, &path)).collect()
    }

    /// What is open in front edited together, where it is several (a
    /// Properties window of several blueprints, objects or areas): each,
    /// the first being [`Moonglow::plugin_open`]'s.
    pub fn plugin_together(&self) -> Vec<mg_plugin::Selected> {
        use crate::Tab;
        use mg_plugin::Selected;
        match self.front_tab() {
            Some(Tab::Blueprints(keys)) => keys.into_iter().map(Selected::Blueprint).collect(),
            Some(Tab::AreasProperties(areas)) => areas.into_iter().map(Selected::Area).collect(),
            Some(Tab::Instances { area, paths }) => (paths.iter())
                .filter_map(|p| match p.0.as_slice() {
                    [mg_edit::Step::Item(list, index)] => Some(Selected::Object {
                        area,
                        kind: mg_area::ObjectKind::from_list(list)?,
                        index: *index,
                    }),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The area or editor in front, for `ctx.open`.
    pub fn plugin_open(&self) -> Option<mg_plugin::Selected> {
        use crate::Tab;
        use mg_plugin::Selected;
        if let Some(first) = self.plugin_together().into_iter().next() {
            return Some(first);
        }
        Some(match self.front_tab()? {
            Tab::Area(area) | Tab::AreaProperties(area) => Selected::Area(area),
            Tab::Instance { area, path } => match path.0.as_slice() {
                [mg_edit::Step::Item(list, index)] => Selected::Object {
                    area,
                    kind: mg_area::ObjectKind::from_list(list)?,
                    index: *index,
                },
                _ => return None,
            },
            Tab::Blueprint(key) => Selected::Blueprint(key),
            Tab::Script(key)
            | Tab::Nui(key)
            | Tab::Gff(key)
            | Tab::Dialog(key)
            | Tab::Resource(key) => Selected::Resource(key),
            // (The resource browser's overridden copies, as their layer
            // has them.)
            Tab::ResourceCopy { key, layer } | Tab::ModelCopy { key, layer } => {
                Selected::Copy { key, layer }
            }
            _ => return None,
        })
    }

    /// Runs a plugin's command as a job, on what is selected in the
    /// window ([`Moonglow::plugin_selection`]): its edits go in as one
    /// undoable command when it is done.
    pub fn run_plugin_command(&mut self, plugin: &str, command: &str) {
        let selected = self.plugin_selection();
        self.run_plugin_command_on(plugin, command, selected);
    }

    /// Runs a plugin's command as a job on `selected` (what a right-click
    /// menu was opened on), with what is open in front: its edits go in
    /// as one undoable command when it is done.
    pub fn run_plugin_command_on(
        &mut self,
        plugin: &str,
        command: &str,
        selected: Vec<mg_plugin::Selected>,
    ) {
        let selection = mg_plugin::Selection {
            selected,
            open: self.plugin_open(),
            together: self.plugin_together(),
        };
        let found = self.enabled_plugins().into_iter().find(|p| p.manifest.id == plugin).cloned();
        let Some(plugin) = found else {
            self.log.error(format!("The plugin {plugin} is not enabled"));
            return;
        };
        let Some(decl) = plugin.manifest.commands.iter().find(|c| c.id == command) else {
            self.log.error(format!("{} has no command {command}", plugin.manifest.name));
            return;
        };
        let (name, title) = (plugin.manifest.name.clone(), decl.title.clone());
        let command = command.to_string();
        let settings = Some(self.plugin_settings(&plugin));
        let keeper = plugin.clone();
        let run = move |input: Input, host: Rc<dyn Host>| {
            let input = Input { selection, settings, ..input };
            mg_plugin::run_command(&plugin, &command, input, host)
        };
        self.run_plugin(name, title, false, None, Some(keeper), run);
    }

    /// Notes that a module was opened, for the plugins that act on it.
    pub(crate) fn plugins_module_opened(&mut self) {
        self.plugins.saved = self.ws.as_ref().map(|ws| ws.module.clone());
        self.plugins.events.clear();
        let event = mg_plugin::Event::new(mg_plugin::EventKind::ModuleOpened, vec![]);
        self.queue_plugin_event(&event);
    }

    /// Notes that the module was saved, for the plugins that act on it:
    /// with the resources that are not as they were at the last save.
    pub(crate) fn plugins_module_saved(&mut self) {
        let Some(module) = self.ws.as_ref().map(|ws| ws.module.clone()) else { return };
        let was = self.plugins.saved.replace(module.clone());
        let resources: Vec<mg_resman::ResKey> = (module.keys())
            .filter(|k| match (was.as_ref().and_then(|w| w.shared(k)), module.shared(k)) {
                (Some(a), Some(b)) => !Arc::ptr_eq(a, b) && a != b,
                _ => true,
            })
            .copied()
            .collect();
        let event = mg_plugin::Event::new(mg_plugin::EventKind::ModuleSaved, resources);
        self.queue_plugin_event(&event);
        // And each resource written, for those that act on one.
        crate::plugin_events::queue_saved(self, was.as_ref());
    }

    /// Queues one plugin's event's handler, to run as a job when no other
    /// is running.
    pub(crate) fn queue_event_of(&mut self, plugin: &str, id: &str, event: mg_plugin::Event) {
        self.plugins.events.push_back((plugin.to_string(), id.to_string(), event));
    }

    fn queue_plugin_event(&mut self, event: &mg_plugin::Event) {
        let wanted: Vec<(String, String)> = (self.enabled_plugins().into_iter())
            .flat_map(|p| {
                let of = p.manifest.events.iter().filter(|e| e.on == event.kind);
                of.map(|e| (p.manifest.id.clone(), e.id.clone())).collect::<Vec<_>>()
            })
            .collect();
        for (plugin, id) in wanted {
            self.plugins.events.push_back((plugin, id, event.clone()));
        }
    }

    /// Runs the next thing a plugin does on an event, if no job is
    /// running: as a job, like a command.
    pub(crate) fn next_plugin_event(&mut self) {
        if self.busy() || self.ws.is_none() {
            return;
        }
        // What the last event's job was allowed is kept for the next run.
        let names: Vec<(String, String)> = (self.enabled_plugins().into_iter())
            .map(|p| (p.manifest.id.clone(), grants_name(p)))
            .collect();
        for (id, name) in &names {
            if let Some(granted) = self.plugins.granted.get(id) {
                let granted = granted.lock().unwrap_or_else(|e| e.into_inner());
                keep_grants(&mut self.settings.plugin_grants, name, &granted);
            }
        }
        let Some((id, event_id, event)) = self.plugins.events.pop_front() else { return };
        let found = self.enabled_plugins().into_iter().find(|p| p.manifest.id == id).cloned();
        let Some(plugin) = found else { return };
        let (name, title) = (plugin.manifest.name.clone(), event.kind.name().to_string());
        let kept = &self.settings.plugin_grants;
        let granted = (self.plugins.granted.entry(id))
            .or_insert_with(|| Arc::new(Mutex::new(kept_grants(kept, &grants_name(&plugin)))))
            .clone();
        let settings = Some(self.plugin_settings(&plugin));
        let keeper = plugin.clone();
        let run = move |input: Input, host: Rc<dyn Host>| {
            mg_plugin::run_event(&plugin, &event_id, &event, Input { settings, ..input }, host)
        };
        self.run_plugin(name, title, false, Some(granted), Some(keeper), run);
    }

    /// Plugins › Install Plugin from File…: asks for a plugin's archive
    /// and installs it.
    pub fn install_plugin(&mut self) {
        if self.plugin_dir.is_none() {
            return;
        }
        let Some(path) = self.dialogs.open_file(crate::FileKind::Plugin, None) else { return };
        self.install_plugin_from(&path);
    }

    /// Installs a plugin from its archive into the plugins folder (each
    /// plugin of an archive of several). None of its code runs, and
    /// nothing is enabled. One installed from an archive before is
    /// replaced only after a yes ([`Plugins::replace`]), asked of each.
    pub fn install_plugin_from(&mut self, path: &Path) {
        let Some(dir) = self.plugin_dir.clone() else { return };
        self.plugins.window = true;
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let read = || Package::open_all(path).map_err(|e| e.to_string());
        let packages = match read() {
            Err(e) => {
                self.install_said(Level::Error, format!("{file} was not installed: {e}"));
                return;
            }
            Ok(packages) => packages,
        };
        let mut said = Vec::new();
        for package in packages {
            match package.existing(&dir) {
                Existing::Installed { version } => {
                    let r = Replace { package, file: file.clone(), installed: version };
                    match self.plugins.replace {
                        None => self.plugins.replace = Some(r),
                        Some(_) => self.plugins.replace_next.push_back(r),
                    }
                }
                _ => {
                    self.install_package(&package, &file, false);
                    said.extend(self.plugins.install_note.clone());
                }
            }
        }
        // (Of several, what became of each.)
        if said.len() > 1 {
            let worst =
                said.iter().map(|(l, _)| *l).max_by_key(|l| *l as u8).unwrap_or(Level::Info);
            let text: Vec<String> = said.into_iter().map(|(_, t)| t).collect();
            self.plugins.install_note = Some((worst, text.join("\n")));
        }
    }

    /// Installs a package read from `file`, over the one installed if
    /// `replace`.
    pub fn install_package(&mut self, package: &Package, file: &str, replace: bool) {
        let Some(dir) = self.plugin_dir.clone() else { return };
        let m = &package.manifest;
        match package.install(&dir, replace) {
            Ok(_) => {
                // Installing enables nothing: a plugin new here is off,
                // whatever was enabled under its id once; and what it
                // was allowed, another's code now, is asked again.
                self.forget_plugin_grants(&m.id);
                if !replace {
                    self.settings.plugins_enabled.retain(|e| e != &m.id);
                }
                self.load_plugins();
                let state = if self.plugin_enabled(&m.id) {
                    "it stays enabled"
                } else {
                    "it is off until you enable it"
                };
                self.install_said(
                    Level::Info,
                    format!("Installed {} {} from {file}: {state}", m.name, m.version),
                );
            }
            Err(e) => self.install_said(Level::Error, format!("{file} was not installed: {e}")),
        }
    }

    /// Removes a plugin that was installed from an archive: its folder is
    /// deleted, and it is enabled no longer. One put there by hand is
    /// refused.
    pub fn remove_plugin(&mut self, remove: &Remove) {
        match mg_plugin::remove(&remove.dir) {
            Ok(()) => {
                if let Some(id) = &remove.id {
                    self.settings.plugins_enabled.retain(|e| e != id);
                    self.forget_plugin_grants(id);
                }
                self.load_plugins();
                self.install_said(Level::Info, format!("Removed {}", remove.name));
            }
            Err(e) => {
                self.install_said(Level::Error, format!("{} was not removed: {e}", remove.name));
            }
        }
    }

    fn install_said(&mut self, level: Level, text: String) {
        self.log.said(level, text.clone());
        self.plugins.install_note = Some((level, text));
    }

    /// Runs what is typed in the plugin console, as a plugin's command
    /// runs; what it logs and returns shows under the console.
    pub fn run_console(&mut self) {
        let code = self.plugins.console.clone();
        // (With what is selected and open, as a command from the Plugins
        // menu has them: to try `ctx.selection` on.)
        let selection = mg_plugin::Selection {
            selected: self.plugin_selection(),
            open: self.plugin_open(),
            together: self.plugin_together(),
        };
        let run = move |input: Input, host: Rc<dyn Host>| {
            mg_plugin::run_console(&code, Input { selection, ..input }, host)
        };
        self.plugins.console_output.clear();
        self.run_plugin("Console".into(), "Console".into(), true, None, None, run);
    }

    /// A job running a plugin's code (`run`), with the window as its host.
    /// `console`: the plugin console's, whose output shows under it too;
    /// `plugin`: whose settings what it sets are.
    pub(crate) fn run_plugin(
        &mut self,
        name: String,
        title: String,
        console: bool,
        granted: Option<Granted>,
        plugin: Option<Plugin>,
        run: impl FnOnce(Input, Rc<dyn Host>) -> Result<Outcome, PluginError> + Send + 'static,
    ) {
        // (An event's job that changes nothing says nothing.)
        let quiet = granted.is_some();
        // Its questions are answered in the job's window; where there is
        // none (jobs run to their end at once), from the answers given.
        let ask = self.background_jobs.then(|| Arc::new(Ask::default()));
        let answers = std::mem::take(&mut self.plugins.answers);
        let log = Logged::default();
        let (job_ask, job_log) = (ask.clone(), log.clone());
        let job_name = name.clone();
        self.plugins.form = None;
        self.plugins.ask = ask;
        self.start_job(
            format!("{name}: {}", title.trim_end_matches('…')),
            move |job| {
                let host = Rc::new(JobHost {
                    progress: job.progress.clone(),
                    log: job_log,
                    ask: job_ask,
                    answers: RefCell::new(answers),
                    granted,
                });
                let input = Input::new(job.module.clone(), job.game.clone());
                run(input, host)
            },
            move |app, result| {
                app.plugin_finished(&job_name, &title, console, quiet, result, &log);
                // (Its panels show what it did.)
                if let Some(p) = &plugin {
                    app.plugins.stale_panels(&p.manifest.id);
                }
                // What it set (`ctx.settings:set`), kept once it is done.
                if let (Some(p), Some(set)) = (&plugin, app.plugins.last_set.take()) {
                    app.keep_plugin_settings(p, &set);
                }
            },
        );
        if !self.busy() {
            self.plugins.ask = None;
        }
    }

    /// A plugin's job is over: what it logged goes to the log, its edits
    /// in as one command.
    fn plugin_finished(
        &mut self,
        name: &str,
        title: &str,
        console: bool,
        quiet: bool,
        result: Result<Outcome, PluginError>,
        log: &Logged,
    ) {
        self.plugins.ask = None;
        self.plugins.form = None;
        let logged = std::mem::take(&mut *log.lock().unwrap_or_else(|e| e.into_inner()));
        for (l, text) in logged {
            if console {
                self.plugins.console_output.push((level(l), text.clone()));
            }
            self.log.said(level(l), format!("{name}: {text}"));
        }
        let say = |app: &mut Moonglow, l: Level, text: String| {
            if console {
                app.plugins.console_output.push((l, text.clone()));
            }
            app.log.said(l, text);
        };
        let title = title.trim_end_matches('…');
        // Its haks first (the user agreed to each while it ran): the
        // module's edits may name them. Each is kept as it was, so that
        // the edits failing, or their Undo, takes it back.
        let (mut wrote, mut wrote_files) = (false, false);
        let mut written: Vec<mg_plugin::HakWritten> = Vec::new();
        let result = result.and_then(|outcome| {
            let dir = self.install.as_ref().and_then(mg_resman::GameInstall::user_hak_dir);
            for hak in &outcome.haks {
                let Some(dir) = dir.clone() else {
                    return Err(PluginError::Package(format!(
                        "{name}: {title}: there is no user folder to write the hak {} into",
                        hak.name
                    )));
                };
                match hak.write_kept(&dir) {
                    Ok(done) => {
                        wrote = true;
                        let undone = if outcome.edits.is_empty() {
                            "with no edits, Undo doesn't take a hak back"
                        } else {
                            "Edit › Undo takes it back with the edits"
                        };
                        say(
                            self,
                            Level::Info,
                            format!(
                                "{name}: {title} put {} resources into {} ({} replaced; {undone})",
                                done.added + done.replaced,
                                done.path.display(),
                                done.replaced,
                            ),
                        );
                        written.push(done);
                    }
                    Err(e) => {
                        return Err(PluginError::Package(format!(
                            "{name}: {title}: the hak {} was not written: {e}",
                            hak.name
                        )));
                    }
                }
            }
            // Then the files for folders the user chose.
            for folder in &outcome.folders {
                match folder.write() {
                    Ok((new, replaced)) => {
                        wrote_files = true;
                        say(
                            self,
                            Level::Info,
                            format!(
                                "{name}: {title} wrote {} files into {} ({replaced} replaced; Undo doesn't take them back)",
                                new + replaced,
                                folder.dir.display()
                            ),
                        );
                    }
                    Err(e) => {
                        return Err(PluginError::Package(format!(
                            "{name}: {title}: nothing was written into {}: {e}",
                            folder.dir.display()
                        )));
                    }
                }
            }
            Ok(outcome)
        });
        // (Nothing of it goes in: what it wrote into haks is taken back.)
        if result.is_err() {
            wrote |= self.take_haks_back(&mut written, name, title, console);
        }
        if wrote {
            // (A hak the module has already is read again.)
            self.reload_resources(false);
        }
        // What it set, for the caller to keep: of a job that did not fail,
        // and whose edits went in (taken back below where they did not).
        self.plugins.last_set =
            result.as_ref().ok().map(|o| o.settings.clone()).filter(|s| !s.is_empty());
        match result {
            Ok(outcome) if outcome.edits.is_empty() => {
                if !console && !wrote && !wrote_files && !quiet {
                    say(self, Level::Info, format!("{name}: {title} changed nothing"));
                }
            }
            Ok(outcome) => {
                let mut resources = Vec::new();
                for e in &outcome.edits {
                    if !resources.contains(e.key()) {
                        resources.push(*e.key());
                    }
                }
                let n = resources.len();
                match self.apply(Command::new(outcome.label, outcome.edits)) {
                    Ok(()) => {
                        // (Its haks go with its step of Undo.)
                        if !written.is_empty()
                            && let Some(step) = self.ws.as_ref().and_then(|w| w.last_step())
                        {
                            self.plugins.hak_steps.push((step, std::mem::take(&mut written)));
                        }
                        say(
                            self,
                            Level::Info,
                            format!(
                                "{name}: {title} changed {n} resource{} (Edit › Undo takes it back)",
                                if n == 1 { "" } else { "s" }
                            ),
                        )
                    }
                    // (Held for the builder's answer, it may still go in:
                    // its haks stay.)
                    Err(e) if self.claims.held => {
                        say(self, Level::Error, format!("{name}: {title}: {e}"))
                    }
                    Err(e) => {
                        say(self, Level::Error, format!("{name}: {title}: {e}"));
                        if self.take_haks_back(&mut written, name, title, console) {
                            self.reload_resources(false);
                        }
                        if self.plugins.last_set.take().is_some() {
                            say(
                                self,
                                Level::Info,
                                format!("{name}: {title}: what it set is not kept"),
                            );
                        }
                    }
                }
            }
            Err(PluginError::Canceled(_)) => {
                say(self, Level::Info, format!("{name}: {title} canceled"))
            }
            Err(e) => say(self, Level::Error, e.to_string()),
        }
    }
}

impl Moonglow {
    /// Takes back what a plugin's command wrote into haks, whose edits did
    /// not go in: whether a hak changed.
    fn take_haks_back(
        &mut self,
        written: &mut Vec<mg_plugin::HakWritten>,
        name: &str,
        title: &str,
        console: bool,
    ) -> bool {
        let mut changed = false;
        for mut done in written.drain(..).rev() {
            let (level, text) = match done.undo() {
                Ok(()) => {
                    changed = true;
                    let how = if done.was_there() { "as it was" } else { "not made" };
                    (Level::Info, format!("{name}: {title}: {} {how}", done.path.display()))
                }
                Err(e) => (Level::Warning, format!("{name}: {title}: {e}")),
            };
            if console {
                self.plugins.console_output.push((level, text.clone()));
            }
            self.log.said(level, text);
        }
        changed
    }

    /// After Undo (`redo`: Redo) of step `step`: takes back (writes again)
    /// the haks a plugin's command wrote with it.
    pub(crate) fn plugin_haks_follow(&mut self, step: Option<u64>, redo: bool) {
        // (Those of steps gone, dropped from Redo or of another module, go.)
        match &self.ws {
            Some(ws) => self.plugins.hak_steps.retain(|(s, _)| ws.has_step(*s)),
            None => self.plugins.hak_steps.clear(),
        }
        let Some(step) = step else { return };
        let Some((_, haks)) = self.plugins.hak_steps.iter_mut().find(|(s, _)| *s == step) else {
            return;
        };
        let mut said = Vec::new();
        let mut changed = false;
        let order: Vec<usize> =
            if redo { (0..haks.len()).collect() } else { (0..haks.len()).rev().collect() };
        for i in order {
            let done = &mut haks[i];
            match if redo { done.redo() } else { done.undo() } {
                Ok(()) => {
                    changed = true;
                    let what = match (redo, done.was_there()) {
                        (true, _) => "written again",
                        (false, true) => "as it was",
                        (false, false) => "taken away (the plugin made it)",
                    };
                    said.push((Level::Info, format!("{}: {what}", done.path.display())));
                }
                Err(e) => said.push((Level::Warning, e)),
            }
        }
        for (l, text) in said {
            self.log.said(l, text);
        }
        if changed {
            self.reload_resources(false);
        }
    }
}

/// The keys the enabled plugins' commands ask for, as their own until the
/// user chooses others.
pub(crate) fn register_keys(keymap: &mut crate::keys::Keymap, commands: &[PluginCommand]) {
    for c in commands {
        let keys = c.key.as_deref().and_then(crate::keys::from_text).into_iter().collect();
        keymap.set_default(&c.key_id, keys);
    }
}

/// A running plugin's question, in the job's window: something to read, a
/// yes or no, or a form. `false` when it asks nothing now.
pub(crate) fn question_ui(app: &mut Moonglow, ui: &mut egui::Ui) -> bool {
    let Some(ask) = app.plugins.ask.clone() else { return false };
    let Some(question) = ask.pending() else {
        app.plugins.form = None;
        return false;
    };
    match &question {
        Question::Message(text) => {
            ui.label(text);
            ui.add_space(6.0);
            if ui.button("OK").clicked() {
                ask.answer(Some(Answer::Yes));
            }
        }
        Question::Confirm(text) => {
            ui.label(text);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Yes").clicked() {
                    ask.answer(Some(Answer::Yes));
                }
                if ui.button("No").clicked() {
                    ask.answer(Some(Answer::No));
                }
            });
        }
        Question::Form { title, fields } => {
            let values = form_values(app, &question, fields);
            if !title.is_empty() {
                crate::widgets::section_heading(ui, title);
            }
            form_grid(ui, fields, values);
            ui.add_space(6.0);
            let mut done = None;
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    done = Some(true);
                }
                if crate::widgets::cancel(ui) {
                    done = Some(false);
                }
            });
            match done {
                Some(true) => {
                    let values = fields
                        .iter()
                        .zip(values.iter())
                        .map(|(f, v)| (f.id.clone(), v.clone()))
                        .collect();
                    ask.answer(Some(Answer::Values(values)));
                }
                Some(false) => ask.answer(None),
                None => {}
            }
        }
        Question::File { title, extensions } => {
            let what = if title.is_empty() { "The plugin asks for a file." } else { title };
            ui.label(what);
            if !extensions.is_empty() {
                let kinds: Vec<String> = extensions.iter().map(|e| format!(".{e}")).collect();
                ui.weak(format!("It reads {}.", kinds.join(", ")));
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Choose File…").clicked()
                    && let Some(path) = app.dialogs.open_file_of(title, extensions)
                {
                    ask.answer(Some(Answer::Path(path)));
                }
                if crate::widgets::cancel(ui) {
                    ask.answer(None);
                }
            });
        }
        Question::Page(page) => {
            let values = form_values(app, &question, &page.fields);
            let heading = if page.title.is_empty() { &page.wizard } else { &page.title };
            crate::widgets::section_heading(ui, heading);
            ui.weak(format!("Page {} of {}", page.step, page.steps));
            if !page.text.is_empty() {
                ui.add_space(4.0);
                ui.label(&page.text);
            }
            ui.add_space(6.0);
            form_grid(ui, &page.fields, values);
            if let Some(error) = &page.error {
                ui.add_space(4.0);
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.add_space(6.0);
            let mut go = None;
            ui.horizontal(|ui| {
                if ui.add_enabled(page.back, egui::Button::new("< Back")).clicked() {
                    go = Some(Some(true));
                }
                let next = if page.last { "Finish" } else { "Next >" };
                if ui.button(next).clicked() {
                    go = Some(Some(false));
                }
                if crate::widgets::cancel(ui) {
                    go = Some(None);
                }
            });
            match go {
                Some(Some(back)) => {
                    let values = (page.fields.iter())
                        .zip(values.iter())
                        .map(|(f, v)| (f.id.clone(), v.clone()))
                        .collect();
                    ask.answer(Some(Answer::Page { values, back }));
                }
                Some(None) => ask.answer(None),
                None => {}
            }
        }
        Question::Folder { title } => {
            let what = if title.is_empty() { "The plugin asks for a folder." } else { title };
            ui.label(what);
            ui.weak("It can read the files in the folder you choose, and nothing outside it.");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Choose Folder…").clicked()
                    && let Some(path) = app.dialogs.pick_folder(what, None)
                {
                    ask.answer(Some(Answer::Path(path)));
                }
                if crate::widgets::cancel(ui) {
                    ask.answer(None);
                }
            });
        }
    }
    true
}

/// The values of a form (or a wizard's page) being filled: from its
/// defaults when it is first shown.
fn form_values<'a>(
    app: &'a mut Moonglow,
    question: &Question,
    fields: &[mg_plugin::FormField],
) -> &'a mut Vec<serde_json::Value> {
    let fresh = !matches!(&app.plugins.form, Some((q, _)) if q == question);
    if fresh {
        let defaults = fields.iter().map(|f| f.kind.default_value()).collect();
        app.plugins.form = Some((question.clone(), defaults));
    }
    &mut app.plugins.form.get_or_insert_with(|| (question.clone(), Vec::new())).1
}

/// A form's fields, each with its label, and what was typed into them.
fn form_grid(ui: &mut egui::Ui, fields: &[mg_plugin::FormField], values: &mut [serde_json::Value]) {
    egui::Grid::new("plugin-form").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for (f, value) in fields.iter().zip(values.iter_mut()) {
            crate::widgets::field_label(ui, &f.label);
            match &f.kind {
                FieldKind::Text { .. } => {
                    let mut text = value.as_str().unwrap_or_default().to_string();
                    ui.add(egui::TextEdit::singleline(&mut text).desired_width(220.0));
                    *value = serde_json::json!(text);
                }
                FieldKind::Number { min, max, .. } => {
                    let mut n = value.as_f64().unwrap_or_default();
                    let range = min.unwrap_or(f64::MIN)..=max.unwrap_or(f64::MAX);
                    ui.add(egui::DragValue::new(&mut n).range(range));
                    *value = serde_json::json!(n);
                }
                FieldKind::Check { .. } => {
                    let mut on = value.as_bool().unwrap_or_default();
                    ui.checkbox(&mut on, "");
                    *value = serde_json::json!(on);
                }
                FieldKind::Choice { choices, .. } => {
                    let mut chosen = value.as_str().unwrap_or_default().to_string();
                    crate::widgets::dropdown(("plugin-form", &f.id))
                        .selected_text(chosen.clone())
                        .show(ui, &mut chosen, choices, Clone::clone);
                    *value = serde_json::json!(chosen);
                }
            }
            ui.end_row();
        }
    });
}

/// Tools › Plugins: the plugins installed, each to enable; where they are
/// kept; and the console.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    if !app.plugins.window {
        return;
    }
    let mut open = true;
    let (mut reload, mut toggle, mut run_console, mut install) = (false, None, false, false);
    let mut remove = None;
    let mut forgotten = None;
    let (mut toggle_project, mut open_settings, mut run_tests) = (None, None, None);
    let allowed: std::collections::HashMap<String, usize> = (app.plugins.installed.iter())
        .filter_map(|i| i.plugin.as_ref().ok())
        .map(|p| (p.manifest.id.clone(), app.plugin_grants(&p.manifest.id)))
        .collect();
    // The project's plugins: what the user answered for each, and the
    // installed ones they stand in for.
    let trusts: std::collections::HashMap<String, mg_plugin::project::Trust> =
        (app.project_plugins().into_iter())
            .filter_map(|p| Some((p.manifest.id.clone(), app.project_trust(p)?)))
            .collect();
    let shadowed: Vec<String> =
        (trusts.iter()).filter(|(_, t)| t.runs()).map(|(id, _)| id.clone()).collect();
    crate::widgets::dialog(ctx, "Plugins", &mut open).default_size([560.0, 480.0]).show(
        ctx,
        |ui| {
            ui.label(
                "A plugin adds commands and checks, and may add exporters, importers, wizards \
                 and panels. It reads the open module and the game data \
             and hands back changes that Undo takes back; it cannot reach your files, the \
             network or other programs, but for a file or a folder you choose for it and \
             what you allow it to write there. Each is off until you enable it.",
            );
            ui.weak(format!(
                "Plugins are experimental: the plugin API ({}) is new, and a later release may \
                 change it.",
                mg_plugin::API
            ));
            // How long what plugins do within a save, a paste or a
            // wizard's Finish may take.
            ui.horizontal(|ui| {
                let mut seconds = app.plugin_event_budget().as_secs() as u32;
                crate::widgets::field_label(ui, "Time within a save or a paste").on_hover_text(
                    "How long each plugin's handlers of a save, a paste or a wizard's Finish \
                         may take, which wait for them; past it, they are stopped (and a save \
                         goes on)",
                );
                let r = ui.add(
                    egui::DragValue::new(&mut seconds).range(1..=MOST_EVENT_SECONDS).suffix(" s"),
                );
                if r.changed() {
                    app.settings.plugin_event_seconds = seconds;
                }
            });
            ui.add_space(4.0);
            // The buttons first, the folder under them: its path may be
            // longer than the window is wide.
            ui.horizontal(|ui| {
                install = ui
                    .add_enabled(app.plugin_dir.is_some(), egui::Button::new("Install from File…"))
                    .on_hover_text(
                        "Install a plugin from its archive (a zip); it is off until you enable it",
                    )
                    .clicked();
                reload = ui
                    .button("Reload")
                    .on_hover_text("Read the folder again: a plugin copied in, or changed")
                    .clicked();
                if let Some(dir) = app.plugin_dir.clone()
                    && ui
                        .button("Open Folder")
                        .on_hover_text("Open the plugins folder (it is made if it is not there)")
                        .clicked()
                {
                    crate::widgets::open_folder(ui.ctx(), &dir, &mut app.log);
                }
            });
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Folder");
                match &app.plugin_dir {
                    Some(dir) => {
                        let path = dir.display().to_string();
                        ui.add(egui::Label::new(&path).truncate()).on_hover_text(&path);
                    }
                    None => {
                        ui.weak("none");
                    }
                }
            });
            if let Some((l, note)) = &app.plugins.install_note {
                ui.colored_label(l.color(ui), note);
            }
            if app.no_plugins {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Started with --no-plugins: none are loaded.",
                );
            }
            ui.separator();
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                // (Started without plugins, none are listed: said above.)
                if !app.plugins.installed.iter().any(|i| !i.project) && !app.no_plugins {
                    ui.weak(
                        "No plugins are installed. Install one from its archive (Install from \
                         File…), or copy a plugin's folder into the folder above.",
                    );
                }
                // Remove, for what Install from File put there.
                let mut remove_button = |ui: &mut egui::Ui, installed: &Installed, name: String| {
                    let id = installed.plugin.as_ref().ok().map(|p| p.manifest.id.clone());
                    let tip = if installed.from_archive {
                        "Delete this plugin's folder from the plugins folder"
                    } else {
                        "Not installed from a file: to remove it, delete its folder (Open Folder)"
                    };
                    let button = egui::Button::new("Remove…").small();
                    if ui
                        .add_enabled(installed.from_archive, button)
                        .on_hover_text(tip)
                        .on_disabled_hover_text(tip)
                        .clicked()
                    {
                        remove = Some(Remove { dir: installed.dir.clone(), name, id });
                    }
                };
                // Those installed, then the open project's own.
                let ours = app.project_root();
                let mut project_heading = false;
                for installed in &app.plugins.installed {
                    if installed.project && !std::mem::replace(&mut project_heading, true) {
                        ui.separator();
                        crate::widgets::section_heading(ui, "This project's plugins");
                        let folder = ours
                            .as_ref()
                            .zip(app.ws.as_ref())
                            .map(|(r, ws)| mg_plugin::project::folder(r, &ws.module.project_file));
                        if let Some(folder) = folder {
                            let path = folder.display().to_string();
                            ui.add(egui::Label::new(egui::RichText::new(&path).weak()).truncate())
                                .on_hover_text(&path);
                        }
                        ui.label(
                            "They come with the project's checkout. Each runs only once you \
                             allow it, as it is then: one that changes is off until you allow it \
                             again. Your answers are yours alone, kept in Moonglow's own \
                             settings, never in the project.",
                        );
                        ui.add_space(4.0);
                    }
                    match &installed.plugin {
                        Ok(p) => {
                            let m = &p.manifest;
                            let trust = trusts.get(&m.id).filter(|_| installed.project);
                            let mut on = match trust {
                                Some(t) => t.runs(),
                                None => app.plugin_enabled(&m.id),
                            };
                            ui.horizontal(|ui| {
                                let label = format!("{} {}", m.name, m.version);
                                if ui
                                    .checkbox(&mut on, egui::RichText::new(&label).strong())
                                    .changed()
                                {
                                    match trust {
                                        Some(_) => toggle_project = Some((m.id.clone(), on)),
                                        None => toggle = Some((m.id.clone(), on)),
                                    }
                                }
                                ui.weak(&m.id);
                                if installed.project {
                                    ui.weak("· the project's").on_hover_text(
                                        "It comes with the project's checkout (its plugins \
                                         folder): to remove it, the project removes it",
                                    );
                                } else {
                                    remove_button(ui, installed, label);
                                }
                                if !m.settings.is_empty()
                                    && ui
                                        .small_button("Settings…")
                                        .on_hover_text("The values this plugin keeps")
                                        .clicked()
                                {
                                    open_settings = Some(m.id.clone());
                                }
                                // (Its own tests, as `mg plugin test` runs them.)
                                if let Ok(p) = &installed.plugin
                                    && !mg_plugin::test_files(p).is_empty()
                                    && ui
                                        .small_button("Run Tests")
                                        .on_hover_text(
                                            "Its own tests (tests/*.toml), on their fixtures: \
                                             the log says what each came to",
                                        )
                                        .clicked()
                                {
                                    run_tests = Some(p.clone());
                                }
                            });
                            ui.indent(("plugin", &m.id, installed.project), |ui| {
                                if let Some(t) = trust {
                                    trust_line(ui, t);
                                } else if shadowed.contains(&m.id) {
                                    ui.weak(
                                        "The project has a plugin of this id, which runs in \
                                         its stead while the project is open.",
                                    );
                                }
                                if !m.description.is_empty() {
                                    ui.label(&m.description);
                                }
                                let by = match m.authors.as_slice() {
                                    [] => String::new(),
                                    authors => format!("By {}. ", authors.join(", ")),
                                };
                                ui.weak(format!("{by}License: {}.", m.license));
                                adds(ui, m);
                                let allowed = allowed.get(&m.id).copied().unwrap_or(0);
                                if allowed > 0 {
                                    ui.horizontal(|ui| {
                                        ui.weak(format!(
                                            "Allowed before, and not asked again: {allowed}"
                                        ));
                                        let forget = ui.small_button("Forget").on_hover_text(
                                            "It asks again for the folder it writes into, \
                                             and for your leave",
                                        );
                                        if forget.clicked() {
                                            forgotten = Some(m.id.clone());
                                        }
                                    });
                                }
                            });
                        }
                        Err(e) => {
                            ui.horizontal_wrapped(|ui| {
                                ui.colored_label(ui.visuals().error_fg_color, e);
                                if !installed.project {
                                    let folder = installed.dir.file_name().unwrap_or_default();
                                    let name = folder.to_string_lossy().into_owned();
                                    remove_button(ui, installed, name);
                                }
                            });
                        }
                    }
                    ui.add_space(4.0);
                }
            });
            ui.separator();
            egui::CollapsingHeader::new("Console").show(ui, |ui| {
                ui.label(
                    "Try the plugin API on the open module: mg and ctx are at hand; what the code \
                 returns is shown, and its edits go in as one step.",
                );
                let field = egui::TextEdit::multiline(&mut app.plugins.console)
                    .code_editor()
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .hint_text("return ctx.module:resources(\"utc\")");
                crate::widgets::text_area(ui, egui::Id::new("plugin-console"), field);
                run_console = ui.add_enabled(app.ws.is_some(), egui::Button::new("Run")).clicked();
                for (l, line) in &app.plugins.console_output {
                    ui.label(egui::RichText::new(line).monospace().color(l.color(ui)));
                }
            });
        },
    );
    if let Some((id, on)) = toggle {
        app.enable_plugin(&id, on);
    }
    if let Some((id, on)) = toggle_project {
        app.allow_project_plugin(&id, on);
    }
    if let Some(id) = open_settings {
        app.open_plugin_settings(&id);
    }
    if let Some(p) = run_tests {
        app.run_plugin_tests(p);
    }
    if let Some(id) = forgotten {
        app.forget_plugin_grants(&id);
    }
    if reload {
        app.plugins.install_note = None;
        app.load_plugins();
    }
    if install {
        app.install_plugin();
    }
    if remove.is_some() {
        app.plugins.remove = remove;
    }
    if run_console {
        app.run_console();
    }
    if !open {
        app.plugins.window = false;
        app.plugins.install_note = None;
    }
}

/// What a plugin adds, as the Plugins window and the project's question
/// list it: its commands (and the menus they are in), checks, what it does
/// by itself, and its settings (and where each is kept).
fn adds(ui: &mut egui::Ui, m: &mg_plugin::Manifest) {
    for c in &m.commands {
        // (And the right-click menus it is in.)
        let menus = match c.targets.as_slice() {
            [] => String::new(),
            t => {
                let names: Vec<String> = t.iter().map(|t| t.name()).collect();
                format!(" (right-click: {})", names.join(", "))
            }
        };
        let title = c.title.trim_end_matches('…');
        ui.label(format!("Command: {title}{menus}")).on_hover_text(&c.hint);
    }
    for c in &m.checks {
        ui.label(format!("Check: {}", c.title));
    }
    for e in &m.events {
        ui.label(format!("By itself, {}: {}", e.said(), e.id));
    }
    for e in &m.exporters {
        let of = if e.targets.is_empty() { "" } else { " (of what is selected)" };
        ui.label(format!("Export: {}{of}", e.title.trim_end_matches('…'))).on_hover_text(&e.hint);
    }
    for e in &m.importers {
        ui.label(format!("Import: {}", e.title.trim_end_matches('…'))).on_hover_text(&e.hint);
    }
    for e in &m.wizards {
        ui.label(format!("Wizard: {}", e.title.trim_end_matches('…'))).on_hover_text(&e.hint);
    }
    for e in &m.panels {
        ui.label(format!("Panel: {}", e.title)).on_hover_text(&e.hint);
    }
    // Its settings, by where they are kept (the team's are written into
    // the project's moonglow.toml).
    for (scope, words) in [
        (mg_plugin::Scope::Project, "Settings the team shares (the project's moonglow.toml)"),
        (mg_plugin::Scope::UserProject, "Settings of yours for this project"),
        (mg_plugin::Scope::User, "Settings of yours"),
    ] {
        let each: Vec<&str> =
            m.settings.iter().filter(|d| d.scope == scope).map(|d| d.shown()).collect();
        if !each.is_empty() {
            ui.label(format!("{words}: {}", each.join(", ")));
        }
    }
}

/// Where a setting is kept, in a word or two.
fn scope_short(scope: mg_plugin::Scope) -> &'static str {
    match scope {
        mg_plugin::Scope::User => "yours",
        mg_plugin::Scope::Project => "the team's",
        mg_plugin::Scope::UserProject => "yours, for this project",
    }
}

/// What the user answered for a project's plugin, under its name.
fn trust_line(ui: &mut egui::Ui, t: &mg_plugin::project::Trust) {
    use mg_plugin::project::Trust;
    match t {
        Trust::Allowed => ui.label("Allowed, as it is now: it runs."),
        Trust::Unasked => ui.weak("Not allowed yet: it is off until you allow it."),
        Trust::Changed { was } => ui.colored_label(
            ui.visuals().warn_fg_color,
            format!("Changed since you allowed {was}: it is off until you allow it again."),
        ),
        Trust::Denied => ui.weak("You said no: it is off."),
    };
}

/// The question when a project is opened whose plugins the user has not
/// answered for (or that changed since they allowed them): each plugin with
/// what it adds and keeps, to allow or not. Put off, they stay off, and
/// are asked about at the next opening.
pub(crate) fn project_window(app: &mut Moonglow, ctx: &egui::Context) {
    if app.plugins.project_asking.is_empty() || app.ws.is_none() {
        return;
    }
    let asked: Vec<Plugin> = (app.project_plugins().into_iter())
        .filter(|p| app.plugins.project_asking.contains(&p.manifest.id))
        .cloned()
        .collect();
    if asked.is_empty() {
        app.plugins.project_asking.clear();
        return;
    }
    let folder = app.project_root().zip(app.ws.as_ref()).map(|(r, ws)| {
        let f = mg_plugin::project::folder(&r, &ws.module.project_file);
        f.strip_prefix(&r).map(|p| p.display().to_string()).unwrap_or_default()
    });
    let mut answers: Vec<(String, bool)> = Vec::new();
    let mut later = false;
    let modal = egui::Modal::new(egui::Id::new("plugin-project")).show(ctx, |ui| {
        ui.set_width(520.0);
        ui.heading("This project comes with plugins");
        ui.add_space(4.0);
        ui.label(format!(
            "Its folder {}/ holds plugins the team shares. None of their code runs until you \
             allow it, and what you allow is each plugin as it is now: if it changes (a pull \
             brings another version), you are asked again. Your answers are yours alone, kept \
             in Moonglow's own settings, never in the project.",
            folder.unwrap_or_else(|| "plugins".into())
        ));
        ui.add_space(6.0);
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for p in &asked {
                let m = &p.manifest;
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{} {}", m.name, m.version)).strong());
                    ui.weak(&m.id);
                });
                if let Some(t @ mg_plugin::project::Trust::Changed { .. }) = app.project_trust(p) {
                    trust_line(ui, &t);
                }
                if !m.description.is_empty() {
                    ui.label(&m.description);
                }
                let by = match m.authors.as_slice() {
                    [] => String::new(),
                    authors => format!("By {}. ", authors.join(", ")),
                };
                ui.weak(format!("{by}License: {}.", m.license));
                adds(ui, m);
                ui.horizontal(|ui| {
                    if ui.button("Allow").on_hover_text("Its code runs: as it is now").clicked() {
                        answers.push((m.id.clone(), true));
                    }
                    if ui
                        .button("Don't Allow")
                        .on_hover_text("It stays off; Manage Plugins enables it later")
                        .clicked()
                    {
                        answers.push((m.id.clone(), false));
                    }
                });
            }
        });
        ui.separator();
        ui.weak(
            "Like any plugin, an allowed one runs sandboxed: it reads the module and the game's \
             data and hands back changes that Undo takes back, and it reads or writes a file of \
             yours only when you choose it or allow it.",
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if asked.len() > 1
                && ui
                    .button("Allow All")
                    .on_hover_text("Each one's code runs: as it is now")
                    .clicked()
            {
                answers.extend(asked.iter().map(|p| (p.manifest.id.clone(), true)));
            }
            later = ui
                .button("Decide Later")
                .on_hover_text(
                    "They stay off now, and you are asked when the project is next opened",
                )
                .clicked();
        });
    });
    for (id, allow) in answers {
        app.allow_project_plugin(&id, allow);
    }
    if later || modal.should_close() {
        app.plugins.project_asking.clear();
    }
}

/// A plugin's settings, to edit: each by its kind, where it is kept said
/// beside it. Saved, each value is kept where its scope says.
pub(crate) fn settings_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(form) = &app.plugins.settings_form else { return };
    let found = (app.plugins.installed.iter())
        .filter_map(|i| i.plugin.as_ref().ok())
        .find(|p| p.manifest.id == form.plugin)
        .cloned();
    let Some(p) = found else {
        app.plugins.settings_form = None;
        return;
    };
    let project = app.project_root();
    let faults = app.plugin_settings_faults(&p);
    let mut form = form.clone();
    let (mut save, mut cancel, mut defaults, mut tidy) = (false, false, false, false);
    let mut open = true;
    let title = format!("Settings: {}", p.manifest.name);
    crate::widgets::dialog(ctx, &title, &mut open)
        .id(egui::Id::new("plugin-settings"))
        .default_width(460.0)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.weak(
                "Each is kept where it says: yours (Moonglow's settings, for every module), the \
                 team's (the project's moonglow.toml, shared with the checkout), or yours for \
                 this project (its .moonglow/user.toml, never shared).",
            );
            ui.add_space(6.0);
            egui::Grid::new("plugin-settings-grid").num_columns(3).spacing([12.0, 6.0]).show(
                ui,
                |ui| {
                    for d in &p.manifest.settings {
                        let usable = !d.scope.in_project() || project.is_some();
                        let label = crate::widgets::field_label(ui, d.shown());
                        if !d.help.is_empty() {
                            label.on_hover_text(&d.help);
                        }
                        let value = form.values.entry(d.id.clone()).or_insert(d.default.clone());
                        // (Each setting's widgets under its own id: a click
                        // reaches its button whatever the rows above do.)
                        ui.push_id(&d.id, |ui| {
                            ui.add_enabled_ui(usable, |ui| {
                                setting_field(app, ui, d, value, &project)
                            })
                        });
                        let note = ui.weak(scope_short(d.scope));
                        if usable {
                            note.on_hover_text(mg_plugin::settings::scope_words(d.scope));
                        } else {
                            note.on_hover_text(
                                "No project is open (a module kept in one file has none): this \
                                 is kept in a project's files",
                            );
                        }
                        ui.end_row();
                    }
                },
            );
            // What is kept wrong, to tidy.
            if !faults.is_empty() {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    let n = faults.len();
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!(
                            "{n} value{} kept {} not as the plugin declares {}.",
                            if n == 1 { "" } else { "s" },
                            if n == 1 { "is" } else { "are" },
                            if n == 1 { "it" } else { "them" },
                        ),
                    )
                    .on_hover_text(faults.join("\n"));
                    tidy = ui
                        .button("Tidy Up")
                        .on_hover_text(
                            "Take out what is not of its kind or of no setting (its default is \
                             used, as now), and move what is in the other file where its scope \
                             keeps it",
                        )
                        .clicked();
                });
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                save = ui.button("Save").on_hover_text("Keep these values").clicked();
                cancel = crate::widgets::cancel(ui);
                defaults = ui
                    .button("Defaults")
                    .on_hover_text("Put each back to the plugin's own value (Save keeps it)")
                    .clicked();
            });
        });
    if defaults {
        for d in &p.manifest.settings {
            if !d.scope.in_project() || project.is_some() {
                form.values.insert(d.id.clone(), d.default.clone());
            }
        }
    }
    app.plugins.settings_form = Some(form);
    if tidy {
        app.tidy_plugin_settings(&p.manifest.id);
    } else if save {
        app.save_plugin_settings();
    } else if cancel || !open {
        app.plugins.settings_form = None;
    }
}

/// A value as the form keeps it: a list's lines that are empty left out
/// (they are kept while it is edited, for the next to be typed).
fn tidy(v: &mg_plugin::SettingValue) -> mg_plugin::SettingValue {
    match v {
        mg_plugin::SettingValue::List(items) => mg_plugin::SettingValue::List(
            items.iter().map(|i| i.trim().to_string()).filter(|i| !i.is_empty()).collect(),
        ),
        v => v.clone(),
    }
}

/// A color as `#rrggbb` reads, and back.
fn rgb_of(text: &str) -> Option<[u8; 3]> {
    if !mg_plugin::is_color(text) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&text[i..i + 2], 16).ok();
    Some([byte(1)?, byte(3)?, byte(5)?])
}

/// One setting's field in the settings' form.
fn setting_field(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    d: &mg_plugin::SettingDecl,
    value: &mut mg_plugin::SettingValue,
    project: &Option<PathBuf>,
) {
    use mg_plugin::{SettingKind, SettingValue};
    match (&d.kind, value) {
        (SettingKind::Bool, SettingValue::Bool(on)) => {
            ui.checkbox(on, "");
        }
        (SettingKind::Int { min, max }, SettingValue::Int(n)) => {
            let range = min.unwrap_or(i64::MIN)..=max.unwrap_or(i64::MAX);
            ui.add(egui::DragValue::new(n).range(range));
        }
        (SettingKind::Float { min, max }, SettingValue::Float(x)) => {
            let range = min.unwrap_or(f64::MIN)..=max.unwrap_or(f64::MAX);
            ui.add(egui::DragValue::new(x).range(range).speed(0.05).max_decimals(4));
        }
        (SettingKind::List(choices), SettingValue::List(items)) if !choices.is_empty() => {
            // (Each choice ticked or not, in the choices' order, one a line.)
            ui.vertical(|ui| {
                for c in choices {
                    let mut on = items.contains(c);
                    if ui.checkbox(&mut on, c).changed() {
                        let mut now: Vec<String> =
                            choices.iter().filter(|x| items.contains(x)).cloned().collect();
                        match on {
                            true if !now.contains(c) => {
                                now = (choices.iter())
                                    .filter(|x| now.contains(x) || *x == c)
                                    .cloned()
                                    .collect();
                            }
                            false => now.retain(|x| x != c),
                            _ => {}
                        }
                        *items = now;
                    }
                }
            });
        }
        (SettingKind::List(_), SettingValue::List(items)) => {
            let mut text = items.join("\n");
            let edit = egui::TextEdit::multiline(&mut text).desired_rows(3).desired_width(220.0);
            if ui.add(edit).on_hover_text("One a line").changed() {
                *items = text.split('\n').map(str::to_string).collect();
            }
        }
        (SettingKind::Color, SettingValue::Text(hex)) => {
            ui.horizontal(|ui| {
                let mut rgb = rgb_of(hex).unwrap_or_default();
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    *hex = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                }
                ui.add(egui::TextEdit::singleline(hex).desired_width(70.0));
            });
        }
        (SettingKind::Pattern(_), SettingValue::Text(text)) => {
            ui.vertical(|ui| {
                ui.add(egui::TextEdit::singleline(text).desired_width(220.0));
                // (What it is held to, said as it is typed.)
                if let Err(why) = d.check(&SettingValue::Text(text.clone())) {
                    ui.colored_label(ui.visuals().warn_fg_color, why);
                }
            });
        }
        (SettingKind::Choice(choices), SettingValue::Text(chosen)) => {
            crate::widgets::dropdown(("plugin-setting", &d.id)).selected_text(chosen.clone()).show(
                ui,
                chosen,
                choices,
                Clone::clone,
            );
        }
        (SettingKind::Path { folder }, SettingValue::Text(path)) => {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(path).desired_width(200.0));
                let start = std::path::Path::new(path.as_str());
                let start = match project.as_ref().filter(|_| d.scope.in_project()) {
                    Some(root) if start.is_relative() => root.join(start),
                    _ => start.to_path_buf(),
                };
                let start = start.is_dir().then_some(start);
                let tip = if *folder { "Choose a folder" } else { "Choose a file" };
                let chose = ui.small_button("Choose…").on_hover_text(tip).clicked();
                let chosen = match (chose, *folder) {
                    (false, _) => None,
                    (true, true) => app.dialogs.pick_folder(d.shown(), start.as_deref()),
                    (true, false) => app.dialogs.open_file_of(d.shown(), &[]),
                };
                if let Some(chosen) = chosen {
                    // (Of the project, from its root: the same on every
                    // builder's machine.)
                    let inside = project
                        .as_ref()
                        .filter(|_| d.scope.in_project())
                        .and_then(|r| chosen.strip_prefix(r).ok());
                    *path = inside.unwrap_or(&chosen).to_string_lossy().replace('\\', "/");
                }
            });
        }
        (_, SettingValue::Text(text)) => {
            ui.add(egui::TextEdit::singleline(text).desired_width(220.0));
        }
        // (A value not of its kind: back to the default.)
        (_, v) => *v = d.default.clone(),
    }
}

/// The question before a plugin installed from an archive is replaced by
/// another archive's.
pub(crate) fn replace_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(r) = &app.plugins.replace else { return };
    let m = &r.package.manifest;
    let (mut yes, mut no) = (false, false);
    let modal = egui::Modal::new(egui::Id::new("plugin-replace")).show(ctx, |ui| {
        ui.set_width(400.0);
        ui.heading(format!("Replace {}?", m.name));
        ui.add_space(4.0);
        let installed = match r.installed.as_str() {
            "" => format!("{} is installed.", m.name),
            version => format!("{} {version} is installed.", m.name),
        };
        ui.label(format!("{installed} Replace it with {} from {}?", m.version, r.file));
        ui.label(if app.plugin_enabled(&m.id) {
            "Its folder is replaced whole. It is enabled, and stays enabled."
        } else {
            "Its folder is replaced whole. It stays off until you enable it."
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            yes = ui.button("Replace").clicked();
            no = crate::widgets::cancel(ui);
        });
    });
    if yes {
        if let Some(r) = app.plugins.replace.take() {
            app.install_package(&r.package, &r.file, true);
        }
    } else if no || modal.should_close() {
        app.plugins.replace = None;
    }
    // (Of an archive of several, the next one installed already.)
    if app.plugins.replace.is_none() {
        app.plugins.replace = app.plugins.replace_next.pop_front();
    }
}

/// The question before an installed plugin's folder is deleted.
pub(crate) fn remove_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(r) = &app.plugins.remove else { return };
    let (mut yes, mut no) = (false, false);
    let modal = egui::Modal::new(egui::Id::new("plugin-remove")).show(ctx, |ui| {
        ui.set_width(400.0);
        ui.heading(format!("Remove {}?", r.name));
        ui.add_space(4.0);
        ui.label(
            "Its folder is deleted from the plugins folder. Your modules are not touched: what \
             its commands changed in them stays.",
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            yes = ui.button("Remove").clicked();
            no = crate::widgets::cancel(ui);
        });
    });
    if yes {
        if let Some(r) = app.plugins.remove.take() {
            app.remove_plugin(&r);
        }
    } else if no || modal.should_close() {
        app.plugins.remove = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_an_event_was_allowed_is_kept_by_plugin_and_version() {
        let mut kept = vec![("other 1.0.0".to_string(), "q".to_string(), "yes".to_string())];
        let mut granted = std::collections::HashMap::new();
        granted.insert("Folder { title: \"Sources\" }".to_string(), Answer::Path("/src".into()));
        granted.insert("Confirm(\"Write 3 files?\")".to_string(), Answer::Yes);
        // (A form's values are not for keeping.)
        granted.insert("Form".to_string(), Answer::Values(Default::default()));
        assert!(keep_grants(&mut kept, "mine 1.2.0", &granted));
        assert_eq!(kept.len(), 3);
        assert!(!keep_grants(&mut kept, "mine 1.2.0", &granted), "as it was: nothing to save");
        let back = kept_grants(&kept, "mine 1.2.0");
        assert_eq!(back.len(), 2);
        assert_eq!(back["Folder { title: \"Sources\" }"], Answer::Path("/src".into()));
        assert_eq!(back["Confirm(\"Write 3 files?\")"], Answer::Yes);
        // Another version of the plugin was allowed nothing; nor is an
        // answer that cannot be read taken for a yes.
        assert!(kept_grants(&kept, "mine 1.3.0").is_empty());
        kept.push(("mine 1.2.0".into(), "odd".into(), "maybe".into()));
        assert_eq!(kept_grants(&kept, "mine 1.2.0").len(), 2);
        // One taken back is gone from what is kept.
        granted.remove("Confirm(\"Write 3 files?\")");
        assert!(keep_grants(&mut kept, "mine 1.2.0", &granted));
        assert_eq!(kept_grants(&kept, "mine 1.2.0").len(), 1);
        assert_eq!(kept_grants(&kept, "other 1.0.0").len(), 1);
    }
}
