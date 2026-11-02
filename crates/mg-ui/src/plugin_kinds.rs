//! What the enabled plugins add besides commands and checks (API 0.6):
//! exporters (File › Export), importers (File › Import), wizards (the
//! Wizards menu) and panels (panes of their own, Plugins › Panels).
//!
//! An exporter or an importer asks the user where first, with the file
//! dialog, and runs as a job with that in hand; a wizard runs as a job
//! whose pages are its questions, in the job's window. A panel's code runs
//! beside the window, never in a frame and not as a job: on a thread of
//! its own, one render at a time, within [`mg_plugin::PANEL_BUDGET`]; the
//! frame draws what it last returned, data.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use mg_plugin::{
    Answer, Element, Host, Input, Plugin, PluginError, Question, Selected, Shown, Tone,
};

use crate::{Action, Moonglow, Tab};

/// What a plugin adds besides commands and checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Added {
    Exporter,
    Importer,
    Wizard,
    Panel,
}

/// An exporter, importer, wizard or panel of an enabled plugin, as the
/// menus and the Command Palette list it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginEntry {
    pub kind: Added,
    pub plugin: String,
    pub plugin_name: String,
    pub id: String,
    pub title: String,
    pub hint: String,
    /// An exporter's `for`: what is selected that it exports.
    pub targets: Vec<mg_plugin::Target>,
}

/// A render under way: where its result comes, and its switch to stop it.
type Render = (Receiver<Result<Shown, PluginError>>, Arc<AtomicBool>);

/// A panel as the window has it: what it showed last, and the render
/// under way.
#[derive(Debug)]
pub struct PanelState {
    /// What its code returned last, or why it could not.
    pub shown: Option<Result<Shown, String>>,
    /// To be drawn again (once the render under way is done).
    pub stale: bool,
    /// A render under way: where its result comes, and its switch to
    /// stop it.
    running: Option<Render>,
    /// What it was drawn for: the module's revision and what was
    /// selected.
    revision: Option<u64>,
    selection: Vec<Selected>,
    /// When the render under way began, and how long the last took.
    began: Option<Instant>,
    pub took: Option<Duration>,
}

impl Default for PanelState {
    fn default() -> PanelState {
        PanelState {
            shown: None,
            stale: true,
            running: None,
            revision: None,
            selection: Vec::new(),
            began: None,
            took: None,
        }
    }
}

impl PanelState {
    /// Whether a render is under way.
    pub fn drawing(&self) -> bool {
        self.running.is_some()
    }
}

/// A panel's render as a plugin's host: nobody is asked anything, and
/// what it logs goes nowhere (it shows what it has to say).
struct PanelHost {
    stop: Arc<AtomicBool>,
}

impl Host for PanelHost {
    fn log(&self, _: mg_plugin::Level, _: &str) {}

    fn cancelled(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    fn ask(&self, _: &Question) -> Option<Answer> {
        None
    }
}

impl crate::plugins::Plugins {
    /// Stops the panels' renders under way (the game's data is to change:
    /// they hold it), each to be drawn again after.
    pub(crate) fn stop_panels(&mut self) {
        for state in self.panels.values_mut() {
            if let Some((_, stop)) = &state.running {
                stop.store(true, Ordering::Relaxed);
                state.stale = true;
            }
        }
    }

    /// Marks a plugin's panels to be drawn again: one of its jobs ended.
    pub(crate) fn stale_panels(&mut self, plugin: &str) {
        for ((p, _), state) in self.panels.iter_mut() {
            if p == plugin {
                state.stale = true;
            }
        }
    }
}

impl Moonglow {
    /// What the enabled plugins add of a kind, in their plugins' order.
    pub fn plugin_entries(&self, kind: Added) -> Vec<PluginEntry> {
        let mut out = Vec::new();
        for p in self.enabled_plugins() {
            let m = &p.manifest;
            let entry =
                |id: &str, title: &str, hint: &str, targets: &[mg_plugin::Target]| PluginEntry {
                    kind,
                    plugin: m.id.clone(),
                    plugin_name: m.name.clone(),
                    id: id.to_string(),
                    title: title.to_string(),
                    hint: hint.to_string(),
                    targets: targets.to_vec(),
                };
            match kind {
                Added::Exporter => out.extend(
                    m.exporters.iter().map(|e| entry(&e.id, &e.title, &e.hint, &e.targets)),
                ),
                Added::Importer => {
                    out.extend(m.importers.iter().map(|e| entry(&e.id, &e.title, &e.hint, &[])))
                }
                Added::Wizard => {
                    out.extend(m.wizards.iter().map(|e| entry(&e.id, &e.title, &e.hint, &[])))
                }
                Added::Panel => {
                    out.extend(m.panels.iter().map(|e| entry(&e.id, &e.title, &e.hint, &[])))
                }
            }
        }
        out
    }

    /// Whether an entry can be chosen now: with a module open; an exporter
    /// of what is selected, when what is selected is all of its kinds.
    pub fn plugin_entry_enabled(&self, e: &PluginEntry) -> bool {
        if self.ws.is_none() {
            return false;
        }
        if e.kind == Added::Exporter && !e.targets.is_empty() {
            let selected = self.plugin_selection();
            return !selected.is_empty() && selected.iter().all(|s| s.fits(&e.targets));
        }
        true
    }

    /// Whether a plugin's panel is open (in a pane or a window).
    pub fn plugin_panel_open(&self, plugin: &str, panel: &str) -> bool {
        let tab = Tab::PluginPanel { plugin: plugin.to_string(), panel: panel.to_string() };
        self.dock.find_tab(&tab).is_some()
    }

    /// Runs what an entry is: an export or an import (asking where
    /// first), a wizard, or a panel shown (or, shown, closed).
    pub fn run_plugin_entry(&mut self, kind: Added, plugin: &str, id: &str) {
        match kind {
            Added::Exporter => self.run_plugin_export(plugin, id),
            Added::Importer => self.run_plugin_import(plugin, id),
            Added::Wizard => self.run_plugin_wizard(plugin, id),
            Added::Panel => {
                let tab = Tab::PluginPanel { plugin: plugin.to_string(), panel: id.to_string() };
                match self.plugin_panel_open(plugin, id) {
                    true => self.actions.push(Action::CloseTab(tab)),
                    false => self.actions.push(Action::OpenTab(tab)),
                }
            }
        }
    }

    /// The enabled plugin of this id, or an error said.
    fn plugin_of(&mut self, plugin: &str) -> Option<Plugin> {
        let found = self.enabled_plugins().into_iter().find(|p| p.manifest.id == plugin).cloned();
        if found.is_none() {
            self.log.error(format!("The plugin {plugin} is not enabled"));
        }
        found
    }

    /// What a job of a plugin's runs on: what is selected and open.
    fn plugin_job_selection(&self, selected: Vec<Selected>) -> mg_plugin::Selection {
        mg_plugin::Selection {
            selected,
            open: self.plugin_open(),
            together: self.plugin_together(),
        }
    }

    /// File › Export › an exporter: where it writes is asked (a file, or a
    /// folder), then it runs as a job on what is selected and open.
    pub fn run_plugin_export(&mut self, plugin: &str, id: &str) {
        let Some(p) = self.plugin_of(plugin) else { return };
        let Some(decl) = p.manifest.exporters.iter().find(|e| e.id == id).cloned() else {
            self.log.error(format!("{} has no exporter {id}", p.manifest.name));
            return;
        };
        let selected = self.plugin_selection();
        let title = decl.title.trim_end_matches('…').to_string();
        if !decl.targets.is_empty()
            && !(!selected.is_empty() && selected.iter().all(|s| s.fits(&decl.targets)))
        {
            let kinds: Vec<String> = decl.targets.iter().map(|t| t.name()).collect();
            self.log.warn(format!(
                "{}: {title} exports what is selected: select {} first",
                p.manifest.name,
                kinds.join(", ")
            ));
            return;
        }
        let to = if decl.folder {
            self.dialogs.pick_folder(&title, None)
        } else {
            // (Named after the module, beside it.)
            let stem = (self.module_path())
                .and_then(|m| m.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "export".into());
            let name = match decl.extension.as_str() {
                "" => stem,
                ext => format!("{stem}.{ext}"),
            };
            let at = self.module_path().and_then(|m| m.parent().map(|d| d.join(&name)));
            let suggested = at.unwrap_or_else(|| PathBuf::from(&name));
            self.dialogs.save_file_of(&title, &decl.extension, Some(&suggested))
        };
        let Some(to) = to else { return };
        let selection = self.plugin_job_selection(selected);
        let settings = Some(self.plugin_settings(&p));
        let (name, keeper, id) = (p.manifest.name.clone(), p.clone(), id.to_string());
        let run = move |input: Input, host: Rc<dyn Host>| {
            let input = Input { selection, settings, ..input };
            mg_plugin::run_export(&p, &id, &to, input, host)
        };
        self.run_plugin(name, decl.title.clone(), false, None, Some(keeper), run);
    }

    /// File › Import › an importer: what it reads is asked (a file of its
    /// kinds, or a folder), then it runs as a job; its edits go in as one
    /// step of Undo.
    pub fn run_plugin_import(&mut self, plugin: &str, id: &str) {
        let Some(p) = self.plugin_of(plugin) else { return };
        let Some(decl) = p.manifest.importers.iter().find(|e| e.id == id).cloned() else {
            self.log.error(format!("{} has no importer {id}", p.manifest.name));
            return;
        };
        let title = decl.title.trim_end_matches('…').to_string();
        let from = match decl.folder {
            true => self.dialogs.pick_folder(&title, None),
            false => self.dialogs.open_file_of(&title, &decl.extensions),
        };
        let Some(from) = from else { return };
        let selection = self.plugin_job_selection(self.plugin_selection());
        let settings = Some(self.plugin_settings(&p));
        let (name, keeper, id) = (p.manifest.name.clone(), p.clone(), id.to_string());
        let run = move |input: Input, host: Rc<dyn Host>| {
            let input = Input { selection, settings, ..input };
            mg_plugin::run_import(&p, &id, &from, input, host)
        };
        self.run_plugin(name, decl.title.clone(), false, None, Some(keeper), run);
    }

    /// Wizards › a plugin's wizard: a job whose pages are asked in its
    /// window, its Finish's edits one step of Undo.
    pub fn run_plugin_wizard(&mut self, plugin: &str, id: &str) {
        let Some(p) = self.plugin_of(plugin) else { return };
        let Some(decl) = p.manifest.wizards.iter().find(|e| e.id == id).cloned() else {
            self.log.error(format!("{} has no wizard {id}", p.manifest.name));
            return;
        };
        let selection = self.plugin_job_selection(self.plugin_selection());
        let settings = Some(self.plugin_settings(&p));
        let (name, keeper, id) = (p.manifest.name.clone(), p.clone(), id.to_string());
        let run = move |input: Input, host: Rc<dyn Host>| {
            let input = Input { selection, settings, ..input };
            mg_plugin::run_wizard(&p, &id, input, host)
        };
        self.run_plugin(name, decl.title.clone(), false, None, Some(keeper), run);
    }

    /// Draws the open panels again where they are stale: by their
    /// Refresh, a job of their plugin's, or (as their manifest says) the
    /// module changed or what is selected. Their renders' results are
    /// taken in. Called once a frame; runs no plugin code in it, unless
    /// jobs run to their end at once (tests).
    pub(crate) fn panels_frame(&mut self, ctx: &egui::Context) {
        let open: Vec<(String, String)> = (self.dock.iter_all_tabs())
            .filter_map(|(_, t)| match t {
                Tab::PluginPanel { plugin, panel } => Some((plugin.clone(), panel.clone())),
                _ => None,
            })
            .collect();
        // (Those closed are forgotten, a render under way stopped.)
        self.plugins.panels.retain(|key, state| {
            let keep = open.contains(key);
            if !keep && let Some((_, stop)) = &state.running {
                stop.store(true, Ordering::Relaxed);
            }
            keep
        });
        if self.ws.is_none() {
            return;
        }
        let revision = self.ws.as_ref().map(mg_edit::Workspace::revision);
        for key in open {
            let Some(plugin) =
                self.enabled_plugins().into_iter().find(|p| p.manifest.id == key.0).cloned()
            else {
                continue;
            };
            let Some(decl) = plugin.manifest.panels.iter().find(|d| d.id == key.1).cloned() else {
                continue;
            };
            let selection = decl.on_selection.then(|| self.plugin_selection());
            let state = self.plugins.panels.entry(key.clone()).or_default();
            // Its render's result, when it has come.
            if let Some((rx, _)) = &state.running {
                match rx.try_recv() {
                    Ok(result) => {
                        state.running = None;
                        state.took = state.began.map(|b| b.elapsed());
                        match result {
                            // (Stopped for the game's data: drawn again.)
                            Err(PluginError::Canceled(_)) => state.stale = true,
                            r => state.shown = Some(r.map_err(|e| e.to_string())),
                        }
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => {
                        state.running = None;
                        state.shown = Some(Err("its render stopped".into()));
                    }
                }
            }
            if decl.on_edits && state.revision.is_some() && state.revision != revision {
                state.stale = true;
            }
            if let Some(s) = &selection
                && *s != state.selection
            {
                state.stale = true;
            }
            if !state.stale || state.running.is_some() || self.job.is_some() {
                continue;
            }
            state.stale = false;
            state.revision = revision;
            state.selection = selection.clone().unwrap_or_default();
            state.began = Some(Instant::now());
            self.start_panel(plugin, &decl.id, key, ctx);
        }
    }

    /// A panel's render begun: on a thread of its own, or (jobs run to
    /// their end at once) here.
    fn start_panel(
        &mut self,
        plugin: Plugin,
        panel: &str,
        key: (String, String),
        ctx: &egui::Context,
    ) {
        let module = match self.ws.as_mut().map(|ws| ws.snapshot()) {
            Some(Ok(m)) => m,
            Some(Err(e)) => {
                if let Some(state) = self.plugins.panels.get_mut(&key) {
                    state.shown = Some(Err(e.to_string()));
                }
                return;
            }
            None => return,
        };
        let selection = self.plugin_job_selection(self.plugin_selection());
        let input = Input {
            selection,
            settings: Some(self.plugin_settings(&plugin)),
            ..Input::new(module, self.game.clone())
        };
        let stop = Arc::new(AtomicBool::new(false));
        let host = PanelHost { stop: stop.clone() };
        let panel = panel.to_string();
        if !self.background_jobs {
            let shown = mg_plugin::run_panel(&plugin, &panel, input, Rc::new(host));
            if let Some(state) = self.plugins.panels.get_mut(&key) {
                state.took = state.began.map(|b| b.elapsed());
                state.shown = Some(shown.map_err(|e| e.to_string()));
            }
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let repaint = ctx.clone();
        let started = std::thread::Builder::new().name("moonglow panel".into()).spawn(move || {
            let shown = mg_plugin::run_panel(&plugin, &panel, input, Rc::new(host));
            let _ = tx.send(shown);
            repaint.request_repaint();
        });
        if let Some(state) = self.plugins.panels.get_mut(&key) {
            match started {
                Ok(_) => state.running = Some((rx, stop)),
                Err(e) => state.shown = Some(Err(format!("could not start: {e}"))),
            }
        }
    }

    /// Takes the panels of plugins no longer enabled out of the window.
    pub(crate) fn close_stray_panels(&mut self) {
        let enabled: Vec<(String, Vec<String>)> = (self.enabled_plugins().into_iter())
            .map(|p| {
                (
                    p.manifest.id.clone(),
                    p.manifest.ids("panel").iter().map(|s| s.to_string()).collect(),
                )
            })
            .collect();
        crate::tabs::close_tabs(&mut self.dock, |t| match t {
            Tab::PluginPanel { plugin, panel } => {
                enabled.iter().any(|(p, panels)| p == plugin && panels.contains(panel))
            }
            _ => true,
        });
    }

    /// Opens what a panel's item names: an object in its area, an area,
    /// a blueprint or another resource in its editor.
    fn open_selected(&mut self, s: &Selected) {
        match s {
            Selected::Object { area, kind, index } => {
                self.area_focus = Some((*area, *kind, *index));
                self.actions.push(Action::OpenTab(Tab::Area(*area)));
            }
            Selected::Area(area) | Selected::Tile { area, .. } => {
                self.actions.push(Action::OpenTab(Tab::Area(*area)));
            }
            Selected::Blueprint(key) | Selected::Resource(key) => {
                self.actions.push(Action::OpenResource(*key));
            }
            Selected::Copy { key, layer } => {
                let tab = Tab::ResourceCopy { key: *key, layer: layer.clone() };
                self.actions.push(Action::OpenTab(tab));
            }
        }
    }
}

/// A panel's tab's title: its manifest's, while its plugin is enabled.
pub(crate) fn panel_title(app: &Moonglow, plugin: &str, panel: &str) -> String {
    (app.enabled_plugins().into_iter())
        .find(|p| p.manifest.id == plugin)
        .and_then(|p| p.manifest.panels.iter().find(|d| d.id == panel))
        .map_or_else(|| panel.to_string(), |d| d.title.clone())
}

/// A panel: its Refresh, and what its code returned, drawn.
pub(crate) fn panel_ui(app: &mut Moonglow, ui: &mut egui::Ui, plugin: &str, panel: &str) {
    let key = (plugin.to_string(), panel.to_string());
    let found = (app.enabled_plugins().into_iter()).find(|p| p.manifest.id == plugin).map(|p| {
        (p.manifest.name.clone(), p.manifest.panels.iter().find(|d| d.id == panel).cloned())
    });
    let Some((name, Some(decl))) = found else {
        ui.weak("Its plugin is not enabled (Plugins › Manage Plugins…).");
        return;
    };
    ui.horizontal(|ui| {
        let state = app.plugins.panels.entry(key.clone()).or_default();
        if ui.button("⟳ Refresh").on_hover_text("Draw the panel again").clicked() {
            state.stale = true;
        }
        if state.drawing() {
            ui.spinner();
        }
        let took =
            state.took.map(|t| format!(", drawn in {} ms", t.as_millis())).unwrap_or_default();
        ui.weak(format!("{name}{took}")).on_hover_text(&decl.hint);
    });
    ui.separator();
    let shown = app.plugins.panels.get(&key).and_then(|s| s.shown.clone());
    let mut chosen: Option<(String, Vec<Selected>)> = None;
    let mut open: Option<Selected> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match &shown {
        None => {
            ui.weak("Drawing…");
        }
        Some(Err(e)) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        Some(Ok(shown)) => {
            for (i, e) in shown.elements.iter().enumerate() {
                match e {
                    Element::Heading(text) => {
                        crate::widgets::section_heading(ui, text);
                    }
                    Element::Text { text, tone } => {
                        match tone {
                            Tone::Plain => ui.label(text),
                            Tone::Weak => ui.weak(text),
                            Tone::Warning => ui.colored_label(ui.visuals().warn_fg_color, text),
                            Tone::Error => ui.colored_label(ui.visuals().error_fg_color, text),
                        };
                    }
                    Element::Value { label, text } => {
                        ui.horizontal(|ui| {
                            crate::widgets::field_label(ui, label);
                            ui.label(text);
                        });
                    }
                    Element::Separator => {
                        ui.separator();
                    }
                    Element::Button { text, hint, command, on } => {
                        let mut b = ui.button(text);
                        if !hint.is_empty() {
                            b = b.on_hover_text(hint);
                        }
                        if b.clicked() {
                            chosen = Some((command.clone(), on.clone()));
                        }
                    }
                    Element::List(items) => {
                        ui.push_id(("panel-list", i), |ui| {
                            for item in items {
                                let r = match (&item.command, &item.open) {
                                    (None, None) => ui.label(&item.text),
                                    _ => ui.link(&item.text),
                                };
                                let r = if item.hint.is_empty() {
                                    r
                                } else {
                                    r.on_hover_text(&item.hint)
                                };
                                if r.clicked() {
                                    match (&item.command, &item.open) {
                                        (Some(c), _) => chosen = Some((c.clone(), item.on.clone())),
                                        (None, Some(s)) => open = Some(s.clone()),
                                        (None, None) => {}
                                    }
                                }
                            }
                        });
                    }
                }
            }
            if shown.cut > 0 {
                ui.weak(format!(
                    "{} more not shown (a panel shows at most {})",
                    shown.cut,
                    mg_plugin::MAX_ELEMENTS
                ));
            }
            if shown.shortened {
                ui.weak(format!("Texts of over {} characters are cut short.", mg_plugin::MAX_TEXT));
            }
        }
    });
    // A button's command, on what it names, else on what is selected.
    if let Some((command, on)) = chosen {
        let selected = if on.is_empty() { app.plugin_selection() } else { on };
        let (plugin, command) = (plugin.to_string(), command);
        app.actions.push(Action::RunPluginCommand { plugin, command, selected });
    }
    if let Some(s) = open {
        app.open_selected(&s);
    }
}
