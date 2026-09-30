//! Moonglow's user interface (egui). [`Moonglow`] is the whole application
//! state; [`Moonglow::ui`] draws it into any `egui::Ui`, so the same code runs
//! in the desktop window and in `egui_kittest` tests. Widgets never change
//! the module directly: they queue [`Action`]s, which run after the frame and
//! turn edits into undoable [`mg_edit::Command`]s.

pub mod dialogs;
mod gff_view;
mod module_props;
mod script_view;
mod tabs;
mod text;
mod tree;

use std::collections::HashMap;
use std::path::PathBuf;

use egui_dock::{DockArea, DockState};
use mg_core::ResType;
use mg_edit::{Command, Workspace};
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_rules::GameData;

pub use dialogs::{Dialogs, NoDialogs};
pub use tabs::Tab;

/// Something the user asked for, run after the frame is drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    OpenModuleDialog,
    OpenModule(PathBuf),
    Save,
    SaveAsDialog,
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
    /// Pending text of single-line fields being edited, by widget id.
    pub(crate) buffers: HashMap<egui::Id, String>,
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
        let game = match &install {
            Some(gi) => match GameData::open(gi) {
                Ok(g) => {
                    log.info(format!("Game data loaded from {}", gi.root.display()));
                    Some(g)
                }
                Err(e) => {
                    log.error(format!("Could not load the game data: {e}"));
                    None
                }
            },
            None => {
                log.warn("No Neverwinter Nights installation found; set NWN_ROOT.");
                None
            }
        };
        Moonglow {
            install,
            game,
            ws: None,
            dock: DockState::new(vec![Tab::Welcome]),
            log,
            actions: Vec::new(),
            dialogs,
            scripts: HashMap::new(),
            buffers: HashMap::new(),
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
                if ui.button("Open Module…").clicked() {
                    self.actions.push(Action::OpenModuleDialog);
                }
                if ui.add_enabled(open, egui::Button::new("Save")).clicked() {
                    self.actions.push(Action::Save);
                }
                if ui.add_enabled(open, egui::Button::new("Save As…")).clicked() {
                    self.actions.push(Action::SaveAsDialog);
                }
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
            match self.module_path() {
                Some(p) => ui.label(p.display().to_string()),
                None => ui.label("No module"),
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
                self.close();
                if let (Some(game), Some(gi)) = (&mut self.game, &self.install) {
                    let haks = m.haks().unwrap_or_default();
                    match game
                        .resman
                        .add_haks(gi, &haks.iter().map(String::as_str).collect::<Vec<_>>())
                    {
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
                self.log.info(format!("Opened {} ({} resources)", path.display(), m.len()));
                self.ws = Some(Workspace::new(m));
                self.dock = DockState::new(vec![Tab::ModuleProperties]);
            }
            Err(e) => self.log.error(format!("Could not open {}: {e}", path.display())),
        }
    }

    /// Closes the module (discarding unsaved changes; the caller asks first).
    pub fn close(&mut self) {
        self.ws = None;
        self.scripts.clear();
        self.buffers.clear();
        self.dock = DockState::new(vec![Tab::Welcome]);
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
    fn refresh_module_layer(&mut self) {
        let (Some(game), Some(ws)) = (&mut self.game, &mut self.ws) else { return };
        if ws.flush().is_err() {
            return;
        }
        game.resman.remove("module");
        game.resman.add(priority::MODULE, "module", LayerClass::Erf, ws.module.container());
        game.invalidate();
    }

    fn run_actions(&mut self) {
        let actions = std::mem::take(&mut self.actions);
        for a in actions {
            self.run(a);
        }
    }

    /// Runs one action now.
    pub fn run(&mut self, action: Action) {
        match action {
            Action::OpenModuleDialog => {
                let start = self
                    .install
                    .as_ref()
                    .and_then(|gi| gi.user_dir.as_ref())
                    .map(|u| u.join("modules"));
                if let Some(p) = self.dialogs.open_module(start.as_deref()) {
                    self.open_module(&p);
                }
            }
            Action::OpenModule(p) => self.open_module(&p),
            Action::Save => self.save(None),
            Action::SaveAsDialog => {
                if let Some(p) = self.dialogs.save_module(self.module_path().as_deref()) {
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
                self.actions.push(Action::SaveAsDialog);
                return;
            }
            None => ws.save().map_err(|e| e.to_string()),
        };
        match result {
            Ok(()) => {
                self.log
                    .info(format!("Saved {}", self.module_path().unwrap_or_default().display()));
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

    /// A resman view for things that need one without a game install (tests).
    pub fn resman(&self) -> Option<&ResMan> {
        self.game.as_ref().map(|g| &g.resman)
    }
}
