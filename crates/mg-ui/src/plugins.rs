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

use mg_edit::{Command, Edit};
use mg_module::doctor::{Finding, Severity};
use mg_plugin::{
    Answer, Existing, FieldKind, Host, Input, Outcome, Package, Plugin, PluginError, Question,
};

use crate::jobs::{Context, Progress};
use crate::{Level, Moonglow};

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
        if let Some(answer) = self.answers.borrow_mut().pop_front() {
            return answer;
        }
        self.ask.as_ref()?.ask(question.clone(), &self.progress)
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
            });
            let input = Input { module: job.module.clone(), game: job.game.clone() };
            match mg_plugin::run_check(plugin, &check.id, input, host) {
                Ok(found) => findings.extend(found),
                Err(PluginError::Canceled(_)) => return (findings, said),
                Err(e) => findings.push(Finding {
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
        self.settings.plugins_enabled.retain(|e| e != id);
        if on {
            self.settings.plugins_enabled.push(id.to_string());
        }
        self.plugins_changed();
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
                })
            })
            .collect()
    }

    /// Runs a plugin's command as a job: its edits go in as one undoable
    /// command when it is done.
    pub fn run_plugin_command(&mut self, plugin: &str, command: &str) {
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
            mg_plugin::run_command(&plugin, &command, input, host)
        };
        self.run_plugin(name, title, run);
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
        let read = || -> Result<Package, String> {
            let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
            if size > mg_plugin::MAX_ARCHIVE {
                return Err(format!("it is over {} MB", mg_plugin::MAX_ARCHIVE >> 20));
            }
            let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
            Package::read(&bytes).map_err(|e| e.to_string())
        };
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
                // whatever was enabled under its id once.
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
        self.log.entries.push((level, text.clone()));
        self.plugins.install_note = Some((level, text));
    }

    /// Runs what is typed in the plugin console, as a plugin's command
    /// runs; what it logs and returns shows under the console.
    pub fn run_console(&mut self) {
        let code = self.plugins.console.clone();
        let run =
            move |input: Input, host: Rc<dyn Host>| mg_plugin::run_console(&code, input, host);
        self.plugins.console_output.clear();
        self.run_plugin("Console".into(), "Console".into(), run);
    }

    /// A job running a plugin's code (`run`), with the window as its host.
    fn run_plugin(
        &mut self,
        name: String,
        title: String,
        run: impl FnOnce(Input, Rc<dyn Host>) -> Result<Outcome, PluginError> + Send + 'static,
    ) {
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
                });
                let input = Input { module: job.module.clone(), game: job.game.clone() };
                run(input, host)
            },
            move |app, result| app.plugin_finished(&job_name, &title, result, &log),
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
        result: Result<Outcome, PluginError>,
        log: &Logged,
    ) {
        self.plugins.ask = None;
        self.plugins.form = None;
        let console = name == "Console";
        let logged = std::mem::take(&mut *log.lock().unwrap_or_else(|e| e.into_inner()));
        for (l, text) in logged {
            if console {
                self.plugins.console_output.push((level(l), text.clone()));
            }
            self.log.entries.push((level(l), format!("{name}: {text}")));
        }
        let say = |app: &mut Moonglow, l: Level, text: String| {
            if console {
                app.plugins.console_output.push((l, text.clone()));
            }
            app.log.entries.push((l, text));
        };
        let title = title.trim_end_matches('…');
        match result {
            Ok(outcome) if outcome.edits.is_empty() => {
                if !console {
                    say(self, Level::Info, format!("{name}: {title} changed nothing"));
                }
            }
            Ok(outcome) => {
                let mut resources = Vec::new();
                for e in &outcome.edits {
                    let (Edit::SetField { key, .. }
                    | Edit::InsertItem { key, .. }
                    | Edit::RemoveItem { key, .. }
                    | Edit::SetResource { key, .. }) = e;
                    if !resources.contains(key) {
                        resources.push(*key);
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
                if ui.button("Cancel").clicked() {
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
    egui::Window::new("Plugins")
        .open(&mut open)
        .default_pos([180.0, 90.0])
        .default_size([560.0, 480.0])
        .show(ctx, |ui| {
            ui.label(
                "A plugin adds commands and checks. It reads the open module and the game data \
             and hands back changes that Undo takes back; it cannot reach your files, the \
             network or other programs. Each is off until you enable it.",
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
                let color = match l {
                    Level::Error => ui.visuals().error_fg_color,
                    Level::Warning => ui.visuals().warn_fg_color,
                    Level::Info => ui.visuals().text_color(),
                };
                ui.colored_label(color, note);
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
                                    ui.label(format!("Command: {}", c.title.trim_end_matches('…')))
                                        .on_hover_text(&c.hint);
                                }
                                for c in &m.checks {
                                    ui.label(format!("Check: {}", c.title));
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
                    let color = match l {
                        Level::Info => ui.visuals().text_color(),
                        Level::Warning => ui.visuals().warn_fg_color,
                        Level::Error => ui.visuals().error_fg_color,
                    };
                    ui.label(egui::RichText::new(line).monospace().color(color));
                }
            });
        });
    if let Some((id, on)) = toggle {
        app.enable_plugin(&id, on);
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
            no = ui.button("Cancel").clicked();
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
            no = ui.button("Cancel").clicked();
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
