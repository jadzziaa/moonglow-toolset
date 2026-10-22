//! Plugins in the window: those installed in Moonglow's plugins folder,
//! which of them are enabled, their commands (run as jobs, their edits one
//! undoable command) and their checks (run with Verify Module).
//!
//! A plugin is off until enabled here. Its code runs sandboxed
//! (`mg_plugin`): what it can do is read the open module and the game
//! data, hand back edits, write to the log and ask the user something,
//! which the job's window shows.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use mg_edit::Command;
use mg_module::doctor::Finding;
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
    /// asked before it is replaced.
    pub replace: Option<Replace>,
    /// What the last install or removal came to, shown in the window.
    pub install_note: Option<(Level, String)>,
    /// A plugin chosen to remove: asked before its folder is deleted.
    pub remove: Option<Remove>,
    /// What happened that enabled plugins act on, each run as a job when
    /// no other is running: the plugin's id, its event's id, the event.
    events: VecDeque<(String, String, mg_plugin::Event)>,
    /// The module as it was last saved or opened, to tell what a save
    /// changed.
    saved: Option<mg_module::Module>,
    /// What each plugin's events were allowed (a folder chosen, a yes to
    /// writing into it), by plugin id: an event that comes again does not
    /// ask again. Kept between runs in the settings (`plugin_grants`).
    granted: std::collections::HashMap<String, Granted>,
}

/// Answers remembered, by question.
type Granted = Arc<Mutex<std::collections::HashMap<String, Answer>>>;

/// A plugin as its grants are kept under: another version asks again.
fn grants_name(m: &mg_plugin::Manifest) -> String {
    format!("{} {}", m.id, m.version)
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
    /// right-click menu on them has it.
    pub fn fits(&self, kinds: &[mg_plugin::Target]) -> bool {
        !kinds.is_empty() && kinds.iter().all(|k| self.targets.contains(k))
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

fn level(l: mg_plugin::Level) -> Level {
    match l {
        mg_plugin::Level::Info => Level::Info,
        mg_plugin::Level::Warning => Level::Warning,
        mg_plugin::Level::Error => Level::Error,
    }
}

/// The findings of the enabled plugins' checks, on a job's snapshot, and
/// what the checks logged. A check that fails is itself a finding.
pub(crate) fn check_findings(
    plugins: &[Plugin],
    job: &Context,
) -> (Vec<Finding>, Vec<(Level, String)>) {
    let (mut findings, mut said) = (Vec::new(), Vec::new());
    for plugin in plugins {
        for check in &plugin.manifest.checks {
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
            let input = Input::new(job.module.clone(), job.game.clone());
            match mg_plugin::run_check(plugin, &check.id, input, host) {
                Ok(found) => findings.extend(found),
                Err(PluginError::Canceled(_)) => return (findings, said),
                Err(e) => findings.push(plugin.check_failed(&check.id, &e)),
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
        self.plugins.installed.clear();
        if !self.no_plugins
            && let Some(dir) = &self.plugin_dir
        {
            for dir in mg_plugin::folders(dir) {
                let plugin = Plugin::load(&dir).map_err(|e| e.to_string());
                let from_archive = mg_plugin::from_archive(&dir);
                self.plugins.installed.push(Installed { dir, plugin, from_archive });
            }
        }
        self.plugins_changed();
    }

    /// After the installed or enabled plugins changed: their keys, and
    /// what a crash report would name.
    pub(crate) fn plugins_changed(&mut self) {
        self.keymap_stale = true;
        let enabled = self.enabled_plugins().iter().map(|p| p.manifest.id.clone()).collect();
        if let Ok(mut now) = ENABLED.lock() {
            *now = enabled;
        }
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

    /// The installed plugins that are enabled.
    pub fn enabled_plugins(&self) -> Vec<&Plugin> {
        self.plugins
            .installed
            .iter()
            .filter_map(|i| i.plugin.as_ref().ok())
            .filter(|p| self.plugin_enabled(&p.manifest.id))
            .collect()
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
        let area = self.palette.area.and_then(|a| Some((a, self.area_views.get(&a)?)));
        if let Some((area, view)) = area.filter(|(_, v)| !v.selection.is_empty()) {
            let object = |&(kind, index): &(mg_area::ObjectKind, usize)| {
                mg_plugin::Selected::Object { area, kind, index }
            };
            return view.selection.iter().map(object).collect();
        }
        let p = &self.palette;
        let mut keys: Vec<mg_resman::ResKey> = p.selected.into_iter().collect();
        keys.extend(p.chosen.iter().filter(|k| p.selected != Some(**k)));
        keys.into_iter().map(mg_plugin::Selected::Blueprint).collect()
    }

    /// The area or editor in front, for `ctx.open`.
    pub fn plugin_open(&self) -> Option<mg_plugin::Selected> {
        use crate::Tab;
        use mg_plugin::Selected;
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
        let selection = mg_plugin::Selection { selected, open: self.plugin_open() };
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
        let run = move |input: Input, host: Rc<dyn Host>| {
            mg_plugin::run_command(&plugin, &command, Input { selection, ..input }, host)
        };
        self.run_plugin(name, title, false, None, run);
    }

    /// Notes that a module was opened, for the plugins that act on it.
    pub(crate) fn plugins_module_opened(&mut self) {
        self.plugins.saved = self.ws.as_ref().map(|ws| ws.module.clone());
        self.plugins.events.clear();
        let event =
            mg_plugin::Event { kind: mg_plugin::EventKind::ModuleOpened, resources: vec![] };
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
        let event = mg_plugin::Event { kind: mg_plugin::EventKind::ModuleSaved, resources };
        self.queue_plugin_event(&event);
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
        let names: Vec<(String, String)> = (self.plugins.installed.iter())
            .filter_map(|i| i.plugin.as_ref().ok())
            .map(|p| (p.manifest.id.clone(), grants_name(&p.manifest)))
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
            .or_insert_with(|| {
                Arc::new(Mutex::new(kept_grants(kept, &grants_name(&plugin.manifest))))
            })
            .clone();
        let run = move |input: Input, host: Rc<dyn Host>| {
            mg_plugin::run_event(&plugin, &event_id, &event, input, host)
        };
        self.run_plugin(name, title, false, Some(granted), run);
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

    /// Installs a plugin from its archive into the plugins folder. None
    /// of its code runs, and nothing is enabled. One installed from an
    /// archive before is replaced only after a yes ([`Plugins::replace`]).
    pub fn install_plugin_from(&mut self, path: &Path) {
        let Some(dir) = self.plugin_dir.clone() else { return };
        self.plugins.window = true;
        let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let read = || Package::open(path).map_err(|e| e.to_string());
        match read() {
            Err(e) => self.install_said(Level::Error, format!("{file} was not installed: {e}")),
            Ok(package) => match package.existing(&dir) {
                Existing::Installed { version } => {
                    self.plugins.replace = Some(Replace { package, file, installed: version });
                }
                _ => self.install_package(&package, &file, false),
            },
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
        let selection =
            mg_plugin::Selection { selected: self.plugin_selection(), open: self.plugin_open() };
        let run = move |input: Input, host: Rc<dyn Host>| {
            mg_plugin::run_console(&code, Input { selection, ..input }, host)
        };
        self.plugins.console_output.clear();
        self.run_plugin("Console".into(), "Console".into(), true, None, run);
    }

    /// A job running a plugin's code (`run`), with the window as its host.
    /// `console`: the plugin console's, whose output shows under it too.
    fn run_plugin(
        &mut self,
        name: String,
        title: String,
        console: bool,
        granted: Option<Granted>,
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
            move |app, result| app.plugin_finished(&job_name, &title, console, quiet, result, &log),
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
        // module's edits may name them.
        let (mut wrote, mut wrote_files) = (false, false);
        let result = result.and_then(|outcome| {
            let dir = self.install.as_ref().and_then(|gi| gi.user_dir.clone());
            for hak in &outcome.haks {
                let Some(dir) = dir.as_ref().map(|u| u.join("hak")) else {
                    return Err(PluginError::Package(format!(
                        "{name}: {title}: there is no user folder to write the hak {} into",
                        hak.name
                    )));
                };
                match hak.write(&dir) {
                    Ok((added, replaced)) => {
                        wrote = true;
                        say(
                            self,
                            Level::Info,
                            format!(
                                "{name}: {title} put {} resources into {} ({replaced} replaced; Undo doesn't take a hak back)",
                                added + replaced,
                                dir.join(format!("{}.hak", hak.name)).display()
                            ),
                        );
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
        if wrote {
            // (A hak the module has already is read again.)
            self.reload_resources(false);
        }
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
                    Ok(()) => say(
                        self,
                        Level::Info,
                        format!(
                            "{name}: {title} changed {n} resource{} (Edit › Undo takes it back)",
                            if n == 1 { "" } else { "s" }
                        ),
                    ),
                    Err(e) => say(self, Level::Error, format!("{name}: {title}: {e}")),
                }
            }
            Err(PluginError::Canceled(_)) => {
                say(self, Level::Info, format!("{name}: {title} canceled"))
            }
            Err(e) => say(self, Level::Error, e.to_string()),
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
            // Its values, from the defaults when it is first shown.
            let fresh = !matches!(&app.plugins.form, Some((q, _)) if *q == question);
            if fresh {
                let defaults = fields
                    .iter()
                    .map(|f| match &f.kind {
                        FieldKind::Text { default } => serde_json::json!(default),
                        FieldKind::Number { default, .. } => serde_json::json!(default),
                        FieldKind::Check { default } => serde_json::json!(default),
                        FieldKind::Choice { choices, default } => {
                            serde_json::json!(choices[*default])
                        }
                    })
                    .collect();
                app.plugins.form = Some((question.clone(), defaults));
            }
            let Some((_, values)) = &mut app.plugins.form else { return true };
            if !title.is_empty() {
                crate::widgets::section_heading(ui, title);
            }
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
                            egui::ComboBox::from_id_salt(("plugin-form", &f.id))
                                .selected_text(chosen.clone())
                                .show_ui(ui, |ui| {
                                    for c in choices {
                                        ui.selectable_value(&mut chosen, c.clone(), c);
                                    }
                                });
                            *value = serde_json::json!(chosen);
                        }
                    }
                    ui.end_row();
                }
            });
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
    let allowed: std::collections::HashMap<String, usize> = (app.plugins.installed.iter())
        .filter_map(|i| i.plugin.as_ref().ok())
        .map(|p| (p.manifest.id.clone(), app.plugin_grants(&p.manifest.id)))
        .collect();
    egui::Window::new("Plugins")
        .open(crate::widgets::open_unless_escape(ctx, "Plugins", &mut open))
        .default_pos([180.0, 90.0])
        .default_size([560.0, 480.0])
        .show(ctx, |ui| {
            ui.label(
                "A plugin adds commands and checks. It reads the open module and the game data \
             and hands back changes that Undo takes back; it cannot reach your files, the \
             network or other programs, but for a file or a folder you choose for it and \
             what you allow it to write there. Each is off until you enable it.",
            );
            ui.weak(format!(
                "Plugins are experimental: the plugin API ({}) is new, and a later release may \
                 change it.",
                mg_plugin::API
            ));
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
                if let Some(dir) = &app.plugin_dir
                    && ui
                        .button("Open Folder")
                        .on_hover_text("Open the plugins folder (it is made if it is not there)")
                        .clicked()
                {
                    let _ = std::fs::create_dir_all(dir);
                    ui.ctx().open_url(egui::OpenUrl::new_tab(format!("file://{}", dir.display())));
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
                if app.plugins.installed.is_empty() && !app.no_plugins {
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
                for installed in &app.plugins.installed {
                    match &installed.plugin {
                        Ok(p) => {
                            let m = &p.manifest;
                            let mut on = app.plugin_enabled(&m.id);
                            ui.horizontal(|ui| {
                                let label = format!("{} {}", m.name, m.version);
                                if ui
                                    .checkbox(&mut on, egui::RichText::new(&label).strong())
                                    .changed()
                                {
                                    toggle = Some((m.id.clone(), on));
                                }
                                ui.weak(&m.id);
                                remove_button(ui, installed, label);
                            });
                            ui.indent(("plugin", &m.id), |ui| {
                                if !m.description.is_empty() {
                                    ui.label(&m.description);
                                }
                                let by = match m.authors.as_slice() {
                                    [] => String::new(),
                                    authors => format!("By {}. ", authors.join(", ")),
                                };
                                ui.weak(format!("{by}License: {}.", m.license));
                                for c in &m.commands {
                                    // (And the right-click menus it is in.)
                                    let menus = match c.targets.as_slice() {
                                        [] => String::new(),
                                        t => {
                                            let names: Vec<&str> =
                                                t.iter().map(|t| t.name()).collect();
                                            format!(" (right-click: {})", names.join(", "))
                                        }
                                    };
                                    let title = c.title.trim_end_matches('…');
                                    ui.label(format!("Command: {title}{menus}"))
                                        .on_hover_text(&c.hint);
                                }
                                for c in &m.checks {
                                    ui.label(format!("Check: {}", c.title));
                                }
                                for e in &m.events {
                                    let when = match e.on {
                                        mg_plugin::EventKind::ModuleOpened => "a module is opened",
                                        mg_plugin::EventKind::ModuleSaved => "the module is saved",
                                    };
                                    ui.label(format!("By itself, when {when}: {}", e.id));
                                }
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
                                let folder = installed.dir.file_name().unwrap_or_default();
                                let name = folder.to_string_lossy().into_owned();
                                remove_button(ui, installed, name);
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
                ui.add(
                    egui::TextEdit::multiline(&mut app.plugins.console)
                        .code_editor()
                        .desired_rows(4)
                        .desired_width(f32::INFINITY)
                        .hint_text("return ctx.module:resources(\"utc\")"),
                );
                run_console = ui.add_enabled(app.ws.is_some(), egui::Button::new("Run")).clicked();
                for (l, line) in &app.plugins.console_output {
                    ui.label(egui::RichText::new(line).monospace().color(l.color(ui)));
                }
            });
        });
    if let Some((id, on)) = toggle {
        app.enable_plugin(&id, on);
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
