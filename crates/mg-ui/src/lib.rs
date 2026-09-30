//! Moonglow's user interface (egui). [`Moonglow`] is the whole application
//! state; [`Moonglow::ui`] draws it into any `egui::Ui`, so the same code runs
//! in the desktop window and in `egui_kittest` tests. Widgets never change
//! the module directly: they queue [`Action`]s, which run after the frame and
//! turn edits into undoable [`mg_edit::Command`]s.

mod browser;
pub mod dialogs;
mod gff_view;
mod module_props;
mod options;
mod script_view;
pub mod settings;
mod tabs;
mod text;
mod transfer;
mod tree;
pub mod wizards;

use std::collections::HashMap;
use std::path::PathBuf;

use egui_dock::{DockArea, DockState};
use mg_core::ResType;
use mg_edit::{Command, Workspace};
use mg_module::new::AreaSpec;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_rules::GameData;

pub use browser::Browser;
pub use dialogs::{Dialogs, FileKind, NoDialogs};
pub use options::OptionsDraft;
pub use settings::Settings;
pub use tabs::Tab;
pub use transfer::{ExportDraft, ImportDraft};
pub use wizards::{AreaWizard, Wizard};

/// Something the user asked for, run after the frame is drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    NewModuleDialog,
    /// Creates a module with this name (then opens the Area Wizard).
    NewModule(String),
    AreaWizard,
    NewArea(AreaSpec),
    OpenModuleDialog,
    OpenModule(PathBuf),
    /// Opens the Export window with these resources chosen.
    ExportDialog(Vec<ResKey>),
    Export(ExportDraft),
    ImportDialog,
    Import(ImportDraft),
    OptionsDialog,
    /// Uses these folders for the game and the user directory (reloading the
    /// game data; closes the module).
    ApplyOptions(OptionsDraft),
    Save,
    SaveAsDialog,
    /// Saves, then runs the action if the module was saved.
    SaveThen(Box<Action>),
    /// Runs an action that would discard unsaved changes, without asking.
    Proceed(Box<Action>),
    Close,
    Undo,
    Redo,
    Apply(Command),
    OpenTab(Tab),
    CompileScripts,
    Verify,
    Quit,
}

/// How serious a log message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

/// The message log (Aurora's output pane).
#[derive(Debug, Clone, Default)]
pub struct Log {
    pub entries: Vec<(Level, String)>,
}

impl Log {
    pub fn info(&mut self, s: impl Into<String>) {
        self.entries.push((Level::Info, s.into()));
    }
    pub fn warn(&mut self, s: impl Into<String>) {
        self.entries.push((Level::Warning, s.into()));
    }
    pub fn error(&mut self, s: impl Into<String>) {
        self.entries.push((Level::Error, s.into()));
    }
}

/// The application.
pub struct Moonglow {
    pub install: Option<GameInstall>,
    /// The game data, with the open module's haks and module layered in.
    pub game: Option<GameData>,
    pub ws: Option<Workspace>,
    pub dock: DockState<Tab>,
    pub log: Log,
    pub actions: Vec<Action>,
    pub dialogs: Box<dyn Dialogs>,
    /// Open script editors' text, by script.
    pub(crate) scripts: HashMap<ResKey, script_view::ScriptBuffer>,
    /// Script editors' laid-out text, reused between frames.
    pub(crate) laid_out: HashMap<ResKey, Option<script_view::LaidOut>>,
    /// Pending text of single-line fields being edited, by widget id.
    pub(crate) buffers: HashMap<egui::Id, String>,
    pub wizard: Option<Wizard>,
    pub settings: Settings,
    /// The Options window's pending values, while it is open.
    pub options: Option<OptionsDraft>,
    pub export: Option<ExportDraft>,
    pub browser: Browser,
    /// Resources open in read-only viewers, parsed once.
    pub(crate) viewed: HashMap<ResKey, std::sync::Arc<browser::Viewed>>,
    pub import: Option<ImportDraft>,
    /// An action waiting for the answer to "save changes?".
    pub confirm_discard: Option<Action>,
    pub quit_requested: bool,
}

impl std::fmt::Debug for Moonglow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Moonglow")
            .field("install", &self.install)
            .field("module", &self.module_path())
            .finish()
    }
}

impl Moonglow {
    /// The application over a game install (loaded now; failures are logged).
    pub fn new(install: Option<GameInstall>, dialogs: Box<dyn Dialogs>) -> Moonglow {
        let mut log = Log::default();
        let game = load_game(install.as_ref(), &mut log);
        Moonglow {
            install,
            game,
            ws: None,
            dock: DockState::new(vec![Tab::Welcome]),
            log,
            actions: Vec::new(),
            dialogs,
            scripts: HashMap::new(),
            laid_out: HashMap::new(),
            buffers: HashMap::new(),
            wizard: None,
            settings: Settings::default(),
            options: None,
            export: None,
            browser: Browser::new(),
            viewed: HashMap::new(),
            import: None,
            confirm_discard: None,
            quit_requested: false,
        }
    }

    pub fn module_path(&self) -> Option<PathBuf> {
        self.ws.as_ref().and_then(|w| w.module.location.as_ref()).map(|l| l.path().to_path_buf())
    }

    /// The window title: module name and a modified marker.
    pub fn title(&self) -> String {
        match (&self.ws, self.module_path()) {
            (Some(ws), Some(p)) => format!(
                "Moonglow Toolset - {}{}",
                p.file_name().unwrap_or_default().to_string_lossy(),
                if ws.is_modified() { " *" } else { "" }
            ),
            (Some(_), None) => "Moonglow Toolset - (new module)".into(),
            _ => "Moonglow Toolset".into(),
        }
    }

    /// The application with saved settings: their game install (or the
    /// detected one) and recent modules.
    pub fn with_settings(settings: Settings, dialogs: Box<dyn Dialogs>) -> Moonglow {
        let mut app = Moonglow::new(settings.install(), dialogs);
        app.settings = settings;
        app
    }

    /// Draws the whole application.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.shortcuts(ui);
        egui::Panel::top("menu").show(ui, |ui| self.menu(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::bottom("log")
            .resizable(true)
            .default_size(120.0)
            .show(ui, |ui| self.log_ui(ui));
        if self.ws.is_some() {
            egui::Panel::left("tree").resizable(true).default_size(240.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| tree::module_tree(self, ui));
            });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            let mut dock = std::mem::replace(&mut self.dock, DockState::new(Vec::new()));
            {
                let mut viewer = tabs::Viewer { app: self };
                DockArea::new(&mut dock).show_close_buttons(true).show_inside(ui, &mut viewer);
            }
            self.dock = dock;
        });
        wizards::ui(self, ui);
        options::ui(self, ui);
        transfer::ui(self, ui);
        if !self.actions.is_empty() {
            self.run_actions();
            // Show the result now, not at the next input event.
            ui.ctx().request_repaint();
        }
    }

    fn shortcuts(&mut self, ui: &mut egui::Ui) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let pressed = |ui: &mut egui::Ui, m, k| {
            ui.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)))
        };
        if pressed(ui, Modifiers::COMMAND, Key::N) {
            self.actions.push(Action::NewModuleDialog);
        }
        if pressed(ui, Modifiers::COMMAND | Modifiers::ALT, Key::A) {
            self.actions.push(Action::AreaWizard);
        }
        if pressed(ui, Modifiers::COMMAND, Key::O) {
            self.actions.push(Action::OpenModuleDialog);
        }
        if pressed(ui, Modifiers::COMMAND, Key::S) {
            self.actions.push(Action::Save);
        }
        if pressed(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)
            || pressed(ui, Modifiers::COMMAND, Key::Y)
        {
            self.actions.push(Action::Redo);
        }
        if pressed(ui, Modifiers::COMMAND, Key::Z) {
            self.actions.push(Action::Undo);
        }
        if pressed(ui, Modifiers::NONE, Key::F7) {
            self.actions.push(Action::CompileScripts);
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        let open = self.ws.is_some();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Module…").clicked() {
                    self.actions.push(Action::NewModuleDialog);
                }
                if ui.button("Open Module…").clicked() {
                    self.actions.push(Action::OpenModuleDialog);
                }
                let recent = self.settings.recent.clone();
                ui.add_enabled_ui(!recent.is_empty(), |ui| {
                    ui.menu_button("Recent Modules", |ui| {
                        for p in recent {
                            if ui.button(p.display().to_string()).clicked() {
                                self.actions.push(Action::OpenModule(p));
                            }
                        }
                    });
                });
                if ui.add_enabled(open, egui::Button::new("Save")).clicked() {
                    self.actions.push(Action::Save);
                }
                if ui.add_enabled(open, egui::Button::new("Save As…")).clicked() {
                    self.actions.push(Action::SaveAsDialog);
                }
                ui.separator();
                if ui.add_enabled(open, egui::Button::new("Import…")).clicked() {
                    self.actions.push(Action::ImportDialog);
                }
                if ui.add_enabled(open, egui::Button::new("Export…")).clicked() {
                    self.actions.push(Action::ExportDialog(Vec::new()));
                }
                ui.separator();
                if ui.add_enabled(open, egui::Button::new("Close")).clicked() {
                    self.actions.push(Action::Close);
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    self.actions.push(Action::Quit);
                }
            });
            ui.menu_button("Edit", |ui| {
                let (undo, redo) = match &self.ws {
                    Some(ws) => {
                        (ws.can_undo().map(str::to_string), ws.can_redo().map(str::to_string))
                    }
                    None => (None, None),
                };
                let label = |what: &str, cmd: &Option<String>| match cmd {
                    Some(c) => format!("{what} {c}"),
                    None => what.to_string(),
                };
                if ui.add_enabled(undo.is_some(), egui::Button::new(label("Undo", &undo))).clicked()
                {
                    self.actions.push(Action::Undo);
                }
                if ui.add_enabled(redo.is_some(), egui::Button::new(label("Redo", &redo))).clicked()
                {
                    self.actions.push(Action::Redo);
                }
                ui.separator();
                if ui.add_enabled(open, egui::Button::new("Module Properties")).clicked() {
                    self.actions.push(Action::OpenTab(Tab::ModuleProperties));
                }
            });
            ui.menu_button("Wizards", |ui| {
                if ui.add_enabled(open, egui::Button::new("Area Wizard…")).clicked() {
                    self.actions.push(Action::AreaWizard);
                }
            });
            ui.menu_button("Tools", |ui| {
                if ui.button("Resource Browser").clicked() {
                    self.actions.push(Action::OpenTab(Tab::Resources));
                }
                if ui.button("Options…").clicked() {
                    self.actions.push(Action::OptionsDialog);
                }
            });
            ui.menu_button("Build", |ui| {
                if ui.add_enabled(open, egui::Button::new("Compile All Scripts")).clicked() {
                    self.actions.push(Action::CompileScripts);
                }
                if ui.add_enabled(open, egui::Button::new("Verify Module")).clicked() {
                    self.actions.push(Action::Verify);
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match (self.module_path(), &self.ws) {
                (Some(p), _) => ui.label(p.display().to_string()),
                (None, Some(_)) => ui.label("New module, not saved yet"),
                (None, None) => ui.label("No module"),
            };
            if self.ws.as_ref().is_some_and(Workspace::is_modified) {
                ui.label("(modified)");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match &self.install {
                    Some(gi) => ui.label(gi.root.display().to_string()),
                    None => ui.label("No game install"),
                }
            });
        });
    }

    fn log_ui(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(
            ui,
            |ui| {
                for (level, msg) in &self.log.entries {
                    let color = match level {
                        Level::Info => ui.visuals().text_color(),
                        Level::Warning => ui.visuals().warn_fg_color,
                        Level::Error => ui.visuals().error_fg_color,
                    };
                    ui.label(egui::RichText::new(msg).color(color).monospace());
                }
            },
        );
    }

    /// Opens a module: the workspace, and the game data with the module's
    /// haks and the module layered in.
    pub fn open_module(&mut self, path: &std::path::Path) {
        match Module::open(path) {
            Ok(m) => {
                self.log.info(format!("Opened {} ({} resources)", path.display(), m.len()));
                self.use_module(m);
                self.settings.remember(path);
            }
            Err(e) => self.log.error(format!("Could not open {}: {e}", path.display())),
        }
    }

    /// Makes a module the open one, with its haks and itself layered into
    /// the game data.
    fn use_module(&mut self, m: Module) {
        self.close();
        if let (Some(game), Some(gi)) = (&mut self.game, &self.install) {
            let haks = m.haks().unwrap_or_default();
            match game.resman.add_haks(gi, &haks.iter().map(String::as_str).collect::<Vec<_>>()) {
                Ok(missing) => {
                    for h in missing {
                        self.log.warn(format!("Hak {h} not found"));
                    }
                }
                Err(e) => self.log.error(format!("Could not open the module's haks: {e}")),
            }
            game.resman.add(priority::MODULE, "module", LayerClass::Erf, m.container());
            game.invalidate();
        }
        self.ws = Some(Workspace::new(m));
        self.dock = DockState::new(vec![Tab::ModuleProperties]);
        self.load_order_changed();
    }

    /// The game data's layers changed: the browser and the read-only
    /// viewers show them anew.
    fn load_order_changed(&mut self) {
        self.browser.stale = true;
        self.viewed.clear();
    }

    /// Whether closing the module now would lose work: unsaved edits, or a
    /// new module never saved.
    pub fn has_unsaved_work(&self) -> bool {
        self.ws.as_ref().is_some_and(|ws| ws.is_modified() || ws.module.location.is_none())
    }

    fn new_module(&mut self, name: &str) {
        let Some(game) = &self.game else {
            self.log.error("Creating a module needs the game data");
            return;
        };
        match mg_module::new::new_module(game, name, &mut fastrand::Rng::new()) {
            Ok(m) => {
                self.use_module(m);
                self.log.info(format!("Created module {name}"));
                self.run(Action::AreaWizard);
            }
            Err(e) => self.log.error(format!("Could not create the module: {e}")),
        }
    }

    /// Adds an area through one undoable command.
    fn new_area(&mut self, spec: &AreaSpec) {
        let (Some(ws), Some(game)) = (&mut self.ws, &self.game) else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let mut staged = ws.module.clone();
        let area =
            match mg_module::new::add_area(&mut staged, game, spec, &mut fastrand::Rng::new()) {
                Ok(a) => a,
                Err(e) => {
                    self.log.error(format!("Could not create the area: {e}"));
                    return;
                }
            };
        let edits: Vec<mg_edit::Edit> = staged
            .keys()
            .filter(|k| staged.get(k) != ws.module.get(k))
            .map(|k| mg_edit::Edit::SetResource {
                key: *k,
                data: staged.get(k).map(<[u8]>::to_vec),
            })
            .collect();
        match ws.apply(Command::new(format!("New area {area}"), edits)) {
            Ok(()) => self.log.info(format!(
                "Created area {area} ({}, {}×{})",
                spec.tileset, spec.width, spec.height
            )),
            Err(e) => self.log.error(e.to_string()),
        }
        self.refresh_module_layer();
    }

    /// Closes the module (discarding unsaved changes; the caller asks first).
    pub fn close(&mut self) {
        self.ws = None;
        self.scripts.clear();
        self.buffers.clear();
        self.dock = DockState::new(vec![Tab::Welcome]);
        self.load_order_changed();
        if let Some(game) = &mut self.game {
            let module_layers: Vec<String> = game
                .resman
                .layers()
                .iter()
                .filter(|l| l.priority == priority::MODULE || l.label.starts_with("hak:"))
                .map(|l| l.label.clone())
                .collect();
            for l in module_layers {
                game.resman.remove(&l);
            }
            game.invalidate();
        }
    }

    /// Refreshes the module layer of the game data after edits.
    pub(crate) fn refresh_module_layer(&mut self) {
        let (Some(game), Some(ws)) = (&mut self.game, &mut self.ws) else { return };
        if ws.flush().is_err() {
            return;
        }
        game.resman.remove("module");
        game.resman.add(priority::MODULE, "module", LayerClass::Erf, ws.module.container());
        game.invalidate();
        self.load_order_changed();
    }

    fn run_actions(&mut self) {
        let actions = std::mem::take(&mut self.actions);
        for a in actions {
            self.run(a);
        }
    }

    /// Runs one action now. Actions that would discard unsaved work ask
    /// first (see [`Moonglow::confirm_discard`]).
    pub fn run(&mut self, action: Action) {
        let discards = matches!(
            action,
            Action::NewModuleDialog
                | Action::OpenModuleDialog
                | Action::OpenModule(_)
                | Action::ApplyOptions(_)
                | Action::Close
                | Action::Quit
        );
        if discards && self.has_unsaved_work() {
            self.confirm_discard = Some(action);
            return;
        }
        self.run_now(action);
    }

    fn run_now(&mut self, action: Action) {
        match action {
            Action::NewModuleDialog => {
                self.wizard = Some(Wizard::NewModule { name: "module000".into() });
            }
            Action::NewModule(name) => self.new_module(&name),
            Action::AreaWizard => {
                let (Some(ws), Some(game)) = (&self.ws, &self.game) else { return };
                let areas = ws.module.keys_of(ResType::ARE).count();
                self.wizard =
                    Some(Wizard::NewArea(AreaWizard::new(mg_module::new::tilesets(game), areas)));
            }
            Action::NewArea(spec) => self.new_area(&spec),
            Action::SaveThen(next) => {
                self.save(None);
                if !self.has_unsaved_work() {
                    self.run_now(*next);
                }
            }
            Action::Proceed(next) => self.run_now(*next),
            Action::OpenModuleDialog => {
                let start = self
                    .install
                    .as_ref()
                    .and_then(|gi| gi.user_dir.as_ref())
                    .map(|u| u.join("modules"));
                if let Some(p) = self.dialogs.open_file(FileKind::Module, start.as_deref()) {
                    self.open_module(&p);
                }
            }
            Action::OpenModule(p) => self.open_module(&p),
            Action::ExportDialog(keys) => {
                if self.ws.is_some() {
                    self.export = Some(ExportDraft {
                        selected: keys.into_iter().collect(),
                        dependencies: true,
                        ..Default::default()
                    });
                }
            }
            Action::Export(draft) => self.run_export(draft),
            Action::ImportDialog => self.open_import(),
            Action::Import(draft) => self.run_import(draft),
            Action::OptionsDialog => {
                self.options = Some(OptionsDraft::from_settings(&self.settings))
            }
            Action::ApplyOptions(draft) => self.apply_options(draft),
            Action::Save => self.save(None),
            Action::SaveAsDialog => {
                // A new module is offered as <name>.mod in the modules folder.
                let suggested = self.module_path().or_else(|| {
                    let ws = self.ws.as_ref()?;
                    let name = ws
                        .module
                        .info()
                        .ok()?
                        .root
                        .locstring("Mod_Name")?
                        .text(mg_core::Language::ENGLISH, mg_core::Gender::Male)?
                        .into_owned();
                    let dir = self.install.as_ref()?.user_dir.as_ref()?.join("modules");
                    Some(dir.join(format!("{name}.mod")))
                });
                if let Some(p) = self.dialogs.save_file(FileKind::Module, suggested.as_deref()) {
                    self.save(Some(p));
                }
            }
            Action::Close => self.close(),
            Action::Undo | Action::Redo => {
                let Some(ws) = &mut self.ws else { return };
                let r = if action == Action::Undo { ws.undo() } else { ws.redo() };
                match r {
                    Ok(Some(label)) => {
                        self.scripts.clear();
                        self.log.info(format!(
                            "{} {label}",
                            if action == Action::Undo { "Undid" } else { "Redid" }
                        ));
                    }
                    Ok(None) => {}
                    Err(e) => self.log.error(e.to_string()),
                }
            }
            Action::Apply(cmd) => {
                let Some(ws) = &mut self.ws else { return };
                if let Err(e) = ws.apply(cmd) {
                    self.log.error(e.to_string());
                }
            }
            Action::OpenTab(tab) => {
                if self.dock.find_tab(&tab).is_none() {
                    self.dock.push_to_focused_leaf(tab.clone());
                }
                // Bring it to the front, new or not.
                if let Some(path) = self.dock.find_tab(&tab) {
                    let _ = self.dock.set_active_tab(path);
                }
            }
            Action::CompileScripts => self.compile_scripts(),
            Action::Verify => self.verify(),
            Action::Quit => self.quit_requested = true,
        }
    }

    fn save(&mut self, to: Option<PathBuf>) {
        let Some(ws) = &mut self.ws else { return };
        // Script editors' unsaved text is not part of the module until saved
        // there; say so rather than guess.
        let dirty_scripts = self.scripts.values().filter(|b| b.is_dirty()).count();
        let result = match to {
            Some(p) => {
                let loc = if p.extension().is_some() {
                    ModuleLocation::Archive(p)
                } else {
                    ModuleLocation::Folder(p)
                };
                ws.flush()
                    .map_err(|e| e.to_string())
                    .and_then(|_| ws.module.save_as(&loc).map_err(|e| e.to_string()))
            }
            None if ws.module.location.is_none() => {
                self.run_now(Action::SaveAsDialog);
                return;
            }
            None => ws.save().map_err(|e| e.to_string()),
        };
        match result {
            Ok(()) => {
                let path = self.module_path().unwrap_or_default();
                self.log.info(format!("Saved {}", path.display()));
                self.settings.remember(&path);
                if dirty_scripts > 0 {
                    self.log.warn(format!(
                        "{dirty_scripts} script(s) have unsaved edits in their editors"
                    ));
                }
            }
            Err(e) => self.log.error(format!("Save failed: {e}")),
        }
        self.refresh_module_layer();
    }

    fn compile_scripts(&mut self) {
        self.refresh_module_layer();
        let (Some(ws), Some(game)) = (&mut self.ws, &self.game) else {
            self.log.error("Compiling needs an open module and the game data");
            return;
        };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let mut staged = ws.module.clone();
        let results = mg_module::build::compile_scripts(
            &mut staged,
            &game.resman,
            mg_module::build::ScriptSelection::All,
        );
        // Store the new bytecode through one undoable command.
        let edits: Vec<mg_edit::Edit> = staged
            .keys_of(ResType::NCS)
            .filter(|k| staged.get(k) != ws.module.get(k))
            .map(|k| mg_edit::Edit::SetResource {
                key: *k,
                data: staged.get(k).map(<[u8]>::to_vec),
            })
            .collect();
        let failed: Vec<String> = results
            .iter()
            .filter_map(|r| r.result.as_ref().err().map(|e| e.message.clone()))
            .collect();
        for f in &failed {
            self.log.error(f.clone());
        }
        let changed = edits.len();
        if !edits.is_empty()
            && let Err(e) = ws.apply(Command::new("Compile scripts", edits))
        {
            self.log.error(e.to_string());
        }
        self.log.info(format!(
            "Compiled {} scripts: {} failed, {changed} changed",
            results.len(),
            failed.len()
        ));
    }

    fn verify(&mut self) {
        self.refresh_module_layer();
        let (Some(ws), Some(game)) = (&self.ws, &self.game) else { return };
        let missing = mg_module::verify::missing(&ws.module, &game.resman);
        for m in &missing {
            let what = if m.uncompiled { "is not compiled" } else { "is missing" };
            self.log.warn(format!(
                "{:?}: {}{} → {:?} {} {what}",
                m.category,
                m.reference.from,
                m.reference.path,
                m.reference.kind,
                m.reference.target
            ));
        }
        self.log.info(format!("Verify: {} missing reference(s)", missing.len()));
    }

    /// Uses new game and user folders: closes the module and reloads the
    /// game data.
    fn apply_options(&mut self, draft: OptionsDraft) {
        let settings = draft.apply(&self.settings);
        self.close();
        self.settings = settings;
        self.install = self.settings.install();
        self.game = load_game(self.install.as_ref(), &mut self.log);
        self.load_order_changed();
    }

    /// A resman view for things that need one without a game install (tests).
    pub fn resman(&self) -> Option<&ResMan> {
        self.game.as_ref().map(|g| &g.resman)
    }
}

/// Loads the game data of an install, logging the outcome.
fn load_game(install: Option<&GameInstall>, log: &mut Log) -> Option<GameData> {
    match install {
        Some(gi) => match GameData::open(gi) {
            Ok(g) => {
                log.info(format!("Game data loaded from {}", gi.root.display()));
                Some(g)
            }
            Err(e) => {
                log.error(format!("Could not load the game data from {}: {e}", gi.root.display()));
                None
            }
        },
        None => {
            log.warn("No Neverwinter Nights installation found; choose it in Tools > Options.");
            None
        }
    }
}
