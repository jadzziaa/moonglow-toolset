//! Moonglow's user interface (egui). [`Moonglow`] is the whole application
//! state; [`Moonglow::ui`] draws it into any `egui::Ui`, so the same code runs
//! in the desktop window and in `egui_kittest` tests. Widgets never change
//! the module directly: they queue [`Action`]s, which run after the frame and
//! turn edits into undoable [`mg_edit::Command`]s.

mod area_audio;
mod area_props;
pub mod area_reshape;
pub mod area_tools;
pub mod area_view;
pub mod audio;
pub mod blueprint;
pub mod blueprint_wizard;
mod browser;
pub mod build_view;
pub mod creature_wizard;
pub mod dialog_view;
pub mod dialogs;
pub mod faction_view;
mod gff_view;
mod images;
pub mod journal_view;
pub mod levelup_view;
pub mod model_view;
pub mod module_props;
mod options;
pub mod palette_view;
pub mod script_tools;
mod script_view;
pub mod script_wizard;
pub mod settings;
pub mod store_wizard;
mod tabs;
pub mod terrain_mode;
pub mod test_module;
mod text;
pub mod tile_select;
mod transfer;
mod tree;
pub mod widgets;
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
pub use text::set_edit_language;
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
    /// Reports the module's haks' conflicts.
    HakReport,
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
    /// Renames a blueprint (and points its editor at the new name).
    RenameBlueprint {
        from: ResKey,
        to: mg_core::ResRef,
    },
    CompileScripts,
    Verify,
    /// Saves, then starts the game on the module (F9).
    TestModule,
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
    /// Where sounds play: [`audio::Silence`] until the desktop app gives
    /// its speakers.
    pub speaker: Box<dyn audio::Speaker>,
    /// Game images decoded for the UI, by lowercase name.
    pictures: images::Pictures,
    /// The area's sounds (Options › Sounds), and the area view heard this
    /// frame.
    area_audio: area_audio::AreaAudio,
    pub(crate) heard: Option<area_audio::Heard>,
    /// Open script editors' text, by script.
    pub(crate) scripts: HashMap<ResKey, script_view::ScriptBuffer>,
    /// Script editors' laid-out text, reused between frames.
    pub(crate) laid_out: HashMap<ResKey, Option<script_view::LaidOut>>,
    /// Pending text of single-line fields being edited, by widget id.
    pub(crate) buffers: HashMap<egui::Id, String>,
    pub wizard: Option<Wizard>,
    /// An open blueprint wizard.
    pub blueprint_wizard: Option<blueprint_wizard::BlueprintWizard>,
    pub settings: Settings,
    /// The Options window's pending values, while it is open.
    pub options: Option<OptionsDraft>,
    pub export: Option<ExportDraft>,
    pub browser: Browser,
    pub loc_edit: Option<widgets::LocStringEdit>,
    pub module_page: module_props::Page,
    /// Each open blueprint editor's page.
    pub blueprint_pages: blueprint::Pages,
    pub faction_view: faction_view::FactionView,
    pub journal_view: journal_view::JournalView,
    pub dialog_views: HashMap<ResKey, dialog_view::DialogView>,
    pub dialog_clip: Option<dialog_view::DialogClip>,
    /// The New Conversation window's name.
    pub new_dialog: Option<String>,
    pub script_tools: script_tools::ScriptTools,
    /// The Save Script As window: the script and the new name.
    pub(crate) script_save_as: Option<(ResKey, String)>,
    /// The New Script window's name.
    pub new_script: Option<String>,
    pub script_wizard: Option<script_wizard::ScriptWizard>,
    /// The GPU for 3D views (from the window), if there is one.
    pub viewport: Option<model_view::Viewport3d>,
    pub model_views: HashMap<ResKey, model_view::ModelView>,
    /// Open area viewers, by area.
    pub area_views: HashMap<mg_core::ResRef, area_view::AreaView>,
    /// The Adjust Location window.
    pub adjust: Option<area_tools::AdjustLocation>,
    /// The Find Instance window.
    pub find_instance: Option<area_tools::FindInstance>,
    /// The Preview window is shown.
    pub preview_window: bool,
    /// The Tile Properties window, while it is open.
    pub tile_props: Option<tile_select::TileProps>,
    /// The Build Module window, while it is open.
    pub build: Option<build_view::BuildWindow>,
    /// Setup Store (a creature's context menu).
    pub store_wizard: Option<store_wizard::StoreWizard>,
    /// Add Popup Text (a placeable's context menu).
    pub popup_text: Option<store_wizard::PopupText>,
    /// The Creature Levelup Wizard.
    pub levelup: Option<levelup_view::LevelupWizard>,
    /// Wizards › Creature Wizard.
    pub creature_wizard: Option<creature_wizard::CreatureWizard>,
    /// The area whose Area Statistics window is open.
    pub area_stats: Option<mg_core::ResRef>,
    /// The Resize Area window, while it is open.
    pub resize_area: Option<area_reshape::ResizeDraft>,
    /// The Rotate Area window, while it is open.
    pub rotate_area: Option<area_reshape::RotateDraft>,
    /// What to open after the Area Wizard's area is made: the viewer, its
    /// properties.
    pub after_new_area: (bool, bool),
    /// Tiles copied in an area viewer.
    pub tile_clip: Option<tile_select::TileClip>,
    /// Objects copied in an area viewer.
    pub object_clip: Option<area_view::ObjectClip>,
    /// An object to show and select when its area's view is next drawn.
    pub area_focus: Option<(mg_core::ResRef, mg_area::ObjectKind, usize)>,
    /// The blueprint palettes pane.
    pub palette: palette_view::PaletteView,
    /// The hak conflict report being shown.
    pub hak_report: Option<String>,
    /// The custom talk table loaded into the game data, by name.
    custom_tlk: Option<String>,
    pub var_edit: Option<widgets::VarTableEdit>,
    pub picker: Option<widgets::Picker>,
    /// Resources chosen in the picker, by the field that asked.
    pub(crate) picked: HashMap<egui::Id, mg_core::ResRef>,
    /// Resources open in read-only viewers, parsed once.
    pub(crate) viewed: HashMap<ResKey, std::sync::Arc<browser::Viewed>>,
    pub import: Option<ImportDraft>,
    /// An action waiting for the answer to "save changes?".
    pub confirm_discard: Option<Action>,
    pub quit_requested: bool,
    /// Test Module asked to minimize the window (Options › General).
    pub minimize_requested: bool,
    /// Where conversation backups go (Options › Conversation Editor), a
    /// folder per module: the temporary folder's `moonglow-backups`.
    pub conversation_backups: std::path::PathBuf,
    /// Where Print writes the pages it opens in the browser.
    pub print_dir: std::path::PathBuf,
    /// When conversations were last backed up, and what was written.
    dialog_backup_at: Option<std::time::Instant>,
    dialog_backed_up: HashMap<ResKey, Vec<u8>>,
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
            speaker: Box::new(audio::Silence::default()),
            pictures: HashMap::new(),
            area_audio: Default::default(),
            heard: None,
            scripts: HashMap::new(),
            laid_out: HashMap::new(),
            buffers: HashMap::new(),
            wizard: None,
            blueprint_wizard: None,
            settings: Settings::default(),
            options: None,
            export: None,
            browser: Browser::new(),
            loc_edit: None,
            module_page: module_props::Page::default(),
            blueprint_pages: Default::default(),
            faction_view: faction_view::FactionView::default(),
            journal_view: journal_view::JournalView::default(),
            dialog_views: HashMap::new(),
            dialog_clip: None,
            new_dialog: None,
            script_tools: script_tools::ScriptTools::default(),
            script_save_as: None,
            new_script: None,
            script_wizard: None,
            viewport: None,
            model_views: HashMap::new(),
            area_views: HashMap::new(),
            adjust: None,
            find_instance: None,
            area_focus: None,
            object_clip: None,
            tile_clip: None,
            after_new_area: (false, false),
            preview_window: false,
            tile_props: None,
            area_stats: None,
            build: None,
            store_wizard: None,
            popup_text: None,
            levelup: None,
            creature_wizard: None,
            resize_area: None,
            rotate_area: None,
            palette: Default::default(),
            hak_report: None,
            custom_tlk: None,
            var_edit: None,
            picker: None,
            picked: HashMap::new(),
            viewed: HashMap::new(),
            import: None,
            confirm_discard: None,
            quit_requested: false,
            minimize_requested: false,
            conversation_backups: std::env::temp_dir().join("moonglow-backups"),
            print_dir: std::env::temp_dir().join("moonglow-print"),
            dialog_backup_at: None,
            dialog_backed_up: HashMap::new(),
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
    /// Gives the app the window's GPU, for 3D views.
    pub fn set_render_state(&mut self, render_state: egui_wgpu::RenderState) {
        self.viewport = Some(model_view::Viewport3d::new(render_state));
    }

    pub fn with_settings(settings: Settings, dialogs: Box<dyn Dialogs>) -> Moonglow {
        let mut app = Moonglow::new(settings.install(), dialogs);
        set_edit_language(mg_core::Language(settings.edit_language.unwrap_or(0)));
        app.settings = settings;
        app
    }

    /// Draws the whole application.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if std::mem::take(&mut self.minimize_requested) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        self.shortcuts(ui);
        egui::Panel::top("menu").show(ui, |ui| {
            self.menu(ui);
            self.toolbar(ui);
        });
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
        self.heard = None;
        egui::CentralPanel::default().show(ui, |ui| {
            let mut dock = std::mem::replace(&mut self.dock, DockState::new(Vec::new()));
            {
                let mut viewer = tabs::Viewer { app: self };
                DockArea::new(&mut dock).show_close_buttons(true).show_inside(ui, &mut viewer);
            }
            self.dock = dock;
        });
        self.backup_timer(ui);
        // The area view's sounds go on between frames.
        let heard = self.heard.take();
        if area_audio::update(self, heard, ui.input(|i| i.time)) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
        wizards::ui(self, ui);
        blueprint_wizard::ui(self, ui);
        options::ui(self, ui);
        transfer::ui(self, ui);
        widgets::ui(self, ui);
        script_view::windows(self, ui);
        dialog_view::windows(self, ui);
        dialog_view::test_window(self, ui);
        script_wizard::window(self, ui);
        area_tools::windows(self, ui);
        area_tools::preview_window(self, ui);
        tile_select::window(self, ui.ctx());
        area_reshape::windows(self, ui.ctx());
        area_view::stats_window(self, ui.ctx());
        build_view::window(self, ui.ctx());
        store_wizard::window(self, ui.ctx());
        store_wizard::popup_window(self, ui.ctx());
        levelup_view::window(self, ui.ctx());
        creature_wizard::window(self, ui.ctx());
        if let Some(report) = &self.hak_report {
            let mut open = true;
            egui::Window::new("Hak Pak Conflict Analysis")
                .open(&mut open)
                .default_size([600.0, 400.0])
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::both().show(ui, |ui| ui.monospace(report));
                });
            if !open {
                self.hak_report = None;
            }
        }
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
        if pressed(ui, Modifiers::COMMAND | Modifiers::ALT, Key::F) && self.ws.is_some() {
            self.actions.push(Action::OpenTab(Tab::Factions));
        }
        if pressed(ui, Modifiers::COMMAND | Modifiers::ALT, Key::J) && self.ws.is_some() {
            self.actions.push(Action::OpenTab(Tab::Journal));
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
        if pressed(ui, Modifiers::NONE, Key::F9) && self.ws.is_some() {
            self.actions.push(Action::SaveThen(Box::new(Action::TestModule)));
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
                ui.separator();
                let area = self.palette.area.filter(|a| self.area_views.contains_key(a));
                if ui.add_enabled(area.is_some(), egui::Button::new("Resize Area…")).clicked()
                    && let Some(a) = area
                {
                    area_reshape::open_resize(self, a);
                }
                if ui.add_enabled(area.is_some(), egui::Button::new("Rotate Area…")).clicked()
                    && let Some(a) = area
                {
                    self.rotate_area = Some(area_reshape::RotateDraft { area: a, turns: 1 });
                }
                ui.separator();
                if ui.add_enabled(open, egui::Button::new("Find Instance…")).clicked() {
                    self.find_instance.get_or_insert_with(Default::default);
                }
            });
            ui.menu_button("Wizards", |ui| {
                if ui.add_enabled(open, egui::Button::new("Area Wizard…")).clicked() {
                    self.actions.push(Action::AreaWizard);
                }
                ui.separator();
                if ui.add_enabled(open, egui::Button::new("Creature Wizard…")).clicked() {
                    self.creature_wizard = Some(Default::default());
                }
                for kind in blueprint_wizard::KINDS {
                    let text = format!("{} Wizard…", kind.label().trim_end_matches('s'));
                    if ui.add_enabled(open, egui::Button::new(text)).clicked() {
                        self.blueprint_wizard = Some(blueprint_wizard::BlueprintWizard::new(kind));
                    }
                }
            });
            ui.menu_button("Tools", |ui| {
                if ui.add_enabled(open, egui::Button::new("New Conversation…")).clicked() {
                    self.new_dialog = Some(String::new());
                }
                if ui.add_enabled(open, egui::Button::new("Faction Editor")).clicked() {
                    self.actions.push(Action::OpenTab(Tab::Factions));
                }
                if ui.add_enabled(open, egui::Button::new("Journal Editor")).clicked() {
                    self.actions.push(Action::OpenTab(Tab::Journal));
                }
                if ui.add_enabled(open, egui::Button::new("New Script…")).clicked() {
                    self.new_script = Some(String::new());
                }
                ui.separator();
                if ui.button("Palettes").clicked() {
                    self.actions.push(Action::OpenTab(Tab::Palette));
                }
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
                if ui.add_enabled(open, egui::Button::new("Build Module…")).clicked() {
                    self.build.get_or_insert_with(Default::default);
                }
                if ui.add_enabled(open, egui::Button::new("Verify Module")).clicked() {
                    self.actions.push(Action::Verify);
                }
                if ui.add_enabled(open, egui::Button::new("Test Module (F9)")).clicked() {
                    self.actions.push(Action::SaveThen(Box::new(Action::TestModule)));
                }
                ui.separator();
                let area = self.palette.area.filter(|a| self.area_views.contains_key(a));
                if ui.add_enabled(area.is_some(), egui::Button::new("Area Statistics")).clicked() {
                    self.area_stats = area;
                }
            });
        });
    }

    /// Buttons for the common commands, below the menu as in Aurora.
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let open = self.ws.is_some();
        let (undo, redo) = self
            .ws
            .as_ref()
            .map_or((false, false), |ws| (ws.can_undo().is_some(), ws.can_redo().is_some()));
        ui.horizontal(|ui| {
            let mut button =
                |ui: &mut egui::Ui, enabled: bool, label: &str, tip: &str, action: Action| {
                    if ui
                        .add_enabled(enabled, egui::Button::new(label).small())
                        .on_hover_text(tip)
                        .clicked()
                    {
                        self.actions.push(action);
                    }
                };
            button(ui, true, "🗋 New", "New module (Ctrl+N)", Action::NewModuleDialog);
            button(ui, true, "🗁 Open", "Open module (Ctrl+O)", Action::OpenModuleDialog);
            button(ui, open, "💾 Save", "Save module (Ctrl+S)", Action::Save);
            ui.separator();
            button(ui, undo, "⟲ Undo", "Undo (Ctrl+Z)", Action::Undo);
            button(ui, redo, "⟳ Redo", "Redo (Ctrl+Y)", Action::Redo);
            ui.separator();
            button(
                ui,
                open,
                "ℹ Properties",
                "Module properties",
                Action::OpenTab(Tab::ModuleProperties),
            );
            button(ui, open, "🗺 New Area", "Area Wizard (Ctrl+Alt+A)", Action::AreaWizard);
            button(ui, true, "🔍 Resources", "Resource browser", Action::OpenTab(Tab::Resources));
            button(ui, true, "📦 Palettes", "Blueprint palettes", Action::OpenTab(Tab::Palette));
            ui.toggle_value(&mut self.preview_window, "👁 Preview")
                .on_hover_text("Show Preview Window: the blueprint chosen in the palette");
            ui.separator();
            button(ui, open, "⚙ Compile", "Compile all scripts (F7)", Action::CompileScripts);
            button(ui, open, "✔ Verify", "Verify the module", Action::Verify);
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let module = match (self.module_path(), &self.ws) {
            (Some(p), _) => p.display().to_string(),
            (None, Some(_)) => "New module, not saved yet".to_string(),
            (None, None) => "No module".to_string(),
        };
        let modified = self.ws.as_ref().is_some_and(Workspace::is_modified);
        let install = match &self.install {
            Some(gi) => gi.root.display().to_string(),
            None => "No game install".to_string(),
        };
        // The game's folder on the right, the module's on the left; each
        // shortened (with the whole path on hover) rather than overlapping.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.scope(|ui| {
                ui.set_max_width(ui.available_width() * 0.4);
                ui.add(egui::Label::new(&install).truncate()).on_hover_text(&install);
            });
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if modified {
                    ui.label("(modified)");
                }
                ui.add(egui::Label::new(&module).truncate()).on_hover_text(&module);
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
        self.custom_tlk = None;
        self.load_custom_tlk();
        self.load_order_changed();
    }

    /// The game data's layers changed: the browser and the read-only
    /// viewers show them anew.
    fn load_order_changed(&mut self) {
        self.browser.stale = true;
        self.viewed.clear();
    }

    /// Loads the module's custom talk table (from its haks or the user's
    /// `tlk/` folder) when the module names another one than is loaded.
    fn load_custom_tlk(&mut self) {
        let (Some(ws), Some(game)) = (&self.ws, &mut self.game) else { return };
        let name = ws.module.custom_tlk().ok().flatten().filter(|n| !n.trim().is_empty());
        if name == self.custom_tlk {
            return;
        }
        self.custom_tlk = name.clone();
        let Some(name) = name else {
            game.set_custom_tlk(None);
            return;
        };
        let data =
            game.resman.get_named(&name, ResType::TLK).map(|d| d.into_owned()).ok().or_else(|| {
                let dirs = self.install.as_ref().map(|i| i.tlk_dirs()).unwrap_or_default();
                dirs.iter().find_map(|d| std::fs::read(d.join(format!("{name}.tlk"))).ok())
            });
        match data.map(|d| mg_tlk::Tlk::read(&d)) {
            Some(Ok(tlk)) => {
                game.set_custom_tlk(Some(tlk));
                self.log.info(format!("Custom talk table {name} loaded"));
            }
            Some(Err(e)) => {
                game.set_custom_tlk(None);
                self.log.error(format!("Custom talk table {name}: {e}"));
            }
            None => {
                game.set_custom_tlk(None);
                self.log.warn(format!("Custom talk table {name} not found"));
            }
        }
        game.invalidate();
    }

    fn hak_report(&mut self) {
        let (Some(ws), Some(install)) = (&self.ws, &self.install) else { return };
        let haks = ws.module.haks().unwrap_or_default();
        match mg_module::haks::hak_report(install, &haks) {
            Ok(r) => {
                self.log.info(format!("{} conflicting hak resources", r.conflicts().count()));
                self.hak_report = Some(r.to_text());
            }
            Err(e) => self.log.error(format!("Hak report failed: {e}")),
        }
    }

    /// Whether closing the module now would lose work: unsaved edits, or a
    /// new module never saved.
    pub fn has_unsaved_work(&self) -> bool {
        self.ws.as_ref().is_some_and(|ws| ws.is_modified() || ws.module.location.is_none())
            || self.scripts.values().any(|b| b.is_dirty())
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
            Ok(()) => {
                self.log.info(format!(
                    "Created area {area} ({}, {}×{})",
                    spec.tileset, spec.width, spec.height
                ));
                // The wizard's last two choices.
                let (viewer, properties) = std::mem::take(&mut self.after_new_area);
                if viewer {
                    self.actions.push(Action::OpenTab(Tab::Area(area)));
                }
                if properties {
                    self.actions.push(Action::OpenTab(Tab::AreaProperties(area)));
                }
            }
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
            Action::HakReport => self.hak_report(),
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
                self.warn_shadowing(&cmd);
                let Some(ws) = &mut self.ws else { return };
                let custom_tlk = cmd.edits.iter().any(|e| matches!(e, mg_edit::Edit::SetField { label, .. } if label == "Mod_CustomTlk"));
                let edits = cmd.clone();
                match ws.apply(cmd) {
                    Ok(()) => blueprint::after_apply(self, &edits),
                    Err(e) => self.log.error(e.to_string()),
                }
                let Some(ws) = &mut self.ws else { return };
                if custom_tlk && ws.flush().is_ok() {
                    self.load_custom_tlk();
                }
            }
            Action::OpenTab(tab) => {
                if self.dock.find_tab(&tab).is_none() {
                    if tab == Tab::Palette {
                        // Its own pane on the right, as in Aurora.
                        self.dock.main_surface_mut().split_right(
                            egui_dock::NodeIndex::root(),
                            0.72,
                            vec![tab.clone()],
                        );
                    } else {
                        // Not into the palette's pane.
                        let palette = self.dock.find_tab(&Tab::Palette);
                        let in_palette = self
                            .dock
                            .focused_leaf()
                            .zip(palette)
                            .is_some_and(|(f, p)| f.surface == p.surface && f.node == p.node);
                        if in_palette {
                            self.dock.push_to_first_leaf(tab.clone());
                        } else {
                            self.dock.push_to_focused_leaf(tab.clone());
                        }
                    }
                }
                // Bring it to the front, new or not.
                if let Some(path) = self.dock.find_tab(&tab) {
                    let _ = self.dock.set_active_tab(path);
                }
            }
            Action::RenameBlueprint { from, to } => blueprint::rename(self, from, to),
            Action::CompileScripts => self.compile_scripts(),
            Action::Verify => self.verify(),
            Action::TestModule => self.test_module(),
            Action::Quit => self.quit_requested = true,
        }
    }

    /// Options › General's warnings for a resource a command adds to the
    /// module (scripts, conversations, blueprints): a blueprint in
    /// BioWare's reserved names, a hak has it too (the game uses the
    /// hak's), or it replaces one of the game's own.
    fn warn_shadowing(&mut self, cmd: &Command) {
        let (Some(ws), Some(game)) = (&self.ws, &self.game) else { return };
        let mut warnings = Vec::new();
        for e in &cmd.edits {
            let mg_edit::Edit::SetResource { key, data: Some(_) } = e else { continue };
            let named = matches!(
                key.restype,
                ResType::NSS
                    | ResType::DLG
                    | ResType::UTC
                    | ResType::UTD
                    | ResType::UTE
                    | ResType::UTI
                    | ResType::UTM
                    | ResType::UTP
                    | ResType::UTS
                    | ResType::UTT
                    | ResType::UTW
            );
            if !named || ws.module.contains(key) {
                continue;
            }
            // BioWare's own blueprint names (nw_, x0_ to x3_).
            let name = key.resref.to_lowercase().to_string();
            let reserved = ["nw_", "x0_", "x1_", "x2_", "x3_"].iter().any(|p| name.starts_with(p));
            if !self.settings.no_namespace_warning
                && key.restype != ResType::NSS
                && key.restype != ResType::DLG
                && reserved
            {
                warnings.push(format!(
                    "{key}: blueprint names starting nw_, x0_, x1_, x2_ or x3_ are BioWare's"
                ));
            }
            let in_layers = |hak: bool| {
                game.resman.layers().iter().any(|l| {
                    let is_hak = l.priority == mg_resman::priority::HAK
                        || l.priority == mg_resman::priority::HAK_USER;
                    let below = l.priority < mg_resman::priority::MODULE;
                    (if hak { is_hak } else { below }) && l.container.contains(key)
                })
            };
            if !self.settings.no_hak_warning && in_layers(true) {
                warnings.push(format!("{key}: a hak has it too, and the game uses the hak's"));
            } else if !self.settings.no_standard_warning && in_layers(false) {
                warnings.push(format!("{key} replaces the game's own"));
            }
        }
        for w in warnings {
            self.log.warn(w);
        }
    }

    fn save(&mut self, to: Option<PathBuf>) {
        // Options > General: Build module on save.
        if self.settings.build_on_save && self.ws.is_some() {
            build_view::build_on_save(self);
        }
        let Some(ws) = &mut self.ws else { return };
        // Script editors' unsaved text goes into the module first (one
        // undoable command), as saving everything means.
        let mut edits = Vec::new();
        for (key, buf) in self.scripts.iter_mut().filter(|(_, b)| b.is_dirty()) {
            buf.saved = buf.text.clone();
            edits.push(mg_edit::Edit::SetResource {
                key: *key,
                data: Some(text::encode(&buf.text)),
            });
        }
        if !edits.is_empty() {
            let n = edits.len();
            if let Err(e) = ws.apply(Command::new(format!("Save {n} script(s)"), edits)) {
                self.log.error(e.to_string());
            }
        }
        // The custom palettes list the module's blueprints, as Aurora keeps
        // them.
        if let Some(game) = &self.game {
            let rebuilt = ws
                .flush()
                .map_err(|e| e.to_string())
                .and_then(|()| mg_module::palette::rebuild_custom_palettes(&mut ws.module, game));
            if let Err(e) = rebuilt {
                self.log.error(format!("Custom palettes: {e}"));
            }
        }
        // Options › General: the module as it was, kept as
        // `<name>.BackupMod` (Aurora's name) before it is overwritten.
        let target = match &to {
            Some(p) if p.extension().is_some() => Some(p.clone()),
            Some(_) => None,
            None => match &ws.module.location {
                Some(ModuleLocation::Archive(p)) => Some(p.clone()),
                _ => None,
            },
        };
        if !self.settings.no_backups
            && let Some(p) = target.filter(|p| p.is_file())
        {
            let backup = p.with_extension("BackupMod");
            if let Err(e) = std::fs::copy(&p, &backup) {
                self.log.warn(format!("{}: {e}", backup.display()));
            }
        }
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

    /// Starts the game on the saved module (Test Module).
    fn test_module(&mut self) {
        let Some(install) = self.install.clone() else {
            self.log.error("Test Module needs the game");
            return;
        };
        let Some(client) = test_module::client_binary(install.root.as_path()) else {
            self.log.error(format!(
                "Test Module: no game client in {}",
                install.root.as_path().display()
            ));
            return;
        };
        let (Some(path), Some(user)) = (self.module_path(), install.user_dir.as_deref()) else {
            self.log.error("Test Module: save the module in the game's modules folder first");
            return;
        };
        let Some(name) = test_module::module_name(user, &path) else {
            self.log.error(format!(
                "Test Module: the game loads modules from {}; save the module there",
                user.join("modules").display()
            ));
            return;
        };
        match test_module::command(&client, user, &name).spawn() {
            Ok(_) => {
                self.log.info(format!("Testing {name}"));
                // Options > General: Minimize Toolset on test module.
                self.minimize_requested = self.settings.minimize_on_test;
            }
            Err(e) => self.log.error(format!("Test Module: {}: {e}", client.display())),
        }
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
    /// Options › Conversation Editor: backs the open conversations up
    /// every so many minutes (Aurora's default: 5).
    fn backup_timer(&mut self, ui: &egui::Ui) {
        if self.settings.dialog_no_backup {
            return;
        }
        let minutes = u64::from(self.settings.dialog_backup_minutes.unwrap_or(5).max(1));
        let every = std::time::Duration::from_secs(60 * minutes);
        let now = std::time::Instant::now();
        let since = *self.dialog_backup_at.get_or_insert(now);
        if now.duration_since(since) >= every {
            self.backup_conversations();
            self.dialog_backup_at = Some(now);
        }
        ui.ctx().request_repaint_after(every);
    }

    /// Writes each conversation open in an editor, while the module has
    /// unsaved changes, as `<name>.bak` (Aurora's name; its backups go in
    /// the module's working folder) in [`Self::conversation_backups`]'
    /// folder for the module, unless it is as last backed up. The files
    /// written.
    pub fn backup_conversations(&mut self) -> Vec<std::path::PathBuf> {
        let keys: Vec<ResKey> = self
            .dock
            .iter_all_tabs()
            .filter_map(|(_, t)| match t {
                Tab::Dialog(k) => Some(*k),
                _ => None,
            })
            .collect();
        let Some(ws) = self.ws.as_mut() else { return Vec::new() };
        if keys.is_empty() || !ws.is_modified() {
            return Vec::new();
        }
        let module = ws
            .module
            .location
            .as_ref()
            .and_then(|l| l.path().file_stem())
            .map_or_else(|| "untitled".to_string(), |s| s.to_string_lossy().into_owned());
        let docs: Vec<(ResKey, Vec<u8>)> =
            keys.into_iter().filter_map(|k| Some((k, ws.doc(&k).ok()?.to_bytes().ok()?))).collect();
        let dir = self.conversation_backups.join(module);
        let mut written = Vec::new();
        for (key, bytes) in docs {
            if self.dialog_backed_up.get(&key) == Some(&bytes) {
                continue;
            }
            let path = dir.join(format!("{}.bak", key.resref));
            match std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, &bytes)) {
                Ok(()) => {
                    self.dialog_backed_up.insert(key, bytes);
                    written.push(path);
                }
                Err(e) => self.log.error(format!("Backup of {}: {e}", key.resref)),
            }
        }
        written
    }

    fn apply_options(&mut self, draft: OptionsDraft) {
        let settings = draft.apply(&self.settings);
        self.close();
        set_edit_language(mg_core::Language(settings.edit_language.unwrap_or(0)));
        self.settings = settings;
        self.install = self.settings.install();
        self.game = load_game(self.install.as_ref(), &mut self.log);
        self.load_order_changed();
    }

    /// A script editor's current text.
    pub fn script_text(&self, key: ResKey) -> Option<String> {
        self.scripts.get(&key).map(|b| b.text.clone())
    }

    /// A script editor's bookmarked lines (0-based).
    pub fn script_bookmarks(&self, key: ResKey) -> Vec<usize> {
        self.scripts.get(&key).map(|b| b.bookmarks.iter().copied().collect()).unwrap_or_default()
    }

    /// A script editor's numbered bookmarks: number and line (0-based).
    pub fn script_numbered_bookmarks(&self, key: ResKey) -> Vec<(usize, usize)> {
        self.scripts
            .get(&key)
            .map(|b| b.numbered.iter().enumerate().filter_map(|(n, l)| Some((n, (*l)?))).collect())
            .unwrap_or_default()
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
