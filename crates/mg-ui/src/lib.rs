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
pub mod bulk;
pub mod commands;
pub mod creature_wizard;
pub mod dialog_view;
pub mod dialogs;
pub mod faction_view;
mod gff_view;
pub mod hak_view;
mod icons;
mod images;
pub mod jobs;
pub mod journal_view;
pub mod keys;
pub mod levelup_view;
mod manual;
pub mod model_view;
pub mod module_props;
pub mod nwsync_view;
mod options;
pub mod palette_view;
pub mod prefabs;
pub mod recovery;
pub mod references;
pub mod script_nav;
pub mod script_tools;
mod script_view;
pub mod script_wizard;
pub mod settings;
pub mod store_wizard;
mod tabs;
pub mod talk_view;
pub mod terrain_mode;
pub mod test_module;
mod text;
pub mod tile_select;
pub mod tileset_view;
mod transfer;
mod tree;
pub mod var_sets;
pub mod widgets;
pub mod wizards;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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
    /// File › Save As nasher Project…: the module into a nasher project
    /// (made in the folder chosen if it has none).
    SaveAsProjectDialog,
    /// File › Open Folder…: a module folder or a nasher project.
    OpenFolderDialog,
    /// Build › Pack Target: a nasher project's module file, as nasher packs it.
    PackTarget,
    /// Saves, then runs the action if the module was saved.
    SaveThen(Box<Action>),
    /// Runs an action that would discard unsaved changes, without asking.
    Proceed(Box<Action>),
    Close,
    Undo,
    Redo,
    /// Saves the module's talk table (Talk Table › Save).
    SaveTalkTable,
    /// An area's minimap, saved as a PNG.
    ExportMinimap(mg_core::ResRef),
    Apply(Command),
    OpenTab(Tab),
    /// Renames a blueprint (and points its editor at the new name).
    /// Build › Test Module, Choose Character: the game's character
    /// selection for the module.
    TestModuleChoose,
    /// Test From Here: the module as it is now (saved or not), starting in
    /// `area` at `at`, facing `facing` (radians from east).
    TestFromHere {
        area: mg_core::ResRef,
        at: [f32; 3],
        facing: f32,
    },
    /// Tools › Reload Resources: haks and folders that changed on disk.
    ReloadResources,
    /// Edit › Prefabs › a prefab: placed with a click in the area shown.
    PlacePrefab(String),
    /// Edit › Find References: where a resource is used.
    FindReferences(ResKey),
    /// Where a tag is used.
    FindTag(String),
    /// The Rename window for a resource.
    RenameDialog(ResKey),
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
    /// The game's data, shared with the jobs that read it while they run
    /// (changed only between them: [`exclusive`]).
    pub game: Option<Arc<GameData>>,
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
    palettes: images::Palettes,
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
    /// The palette's hover previews.
    pub(crate) thumbnails: model_view::Thumbnails,
    /// The module's talk table, open for editing (Tools › Talk Table).
    pub talk: Option<mg_module::talk::Table>,
    pub talk_view: talk_view::TalkView,
    /// The haks layered into the game data, as the module listed them.
    haks_layered: Vec<String>,
    /// The keys of commands (Options › Keyboard), and the settings they
    /// were read from.
    pub keymap: keys::Keymap,
    keymap_from: std::collections::BTreeMap<String, Vec<String>>,
    /// Haks open in the hak editor.
    pub haks: Vec<hak_view::HakDoc>,
    pub(crate) next_hak: u32,
    /// A hak whose tab is being closed with unsaved changes.
    pub(crate) hak_closing: Option<u32>,
    /// Where Build Hak from Folder suggests saving.
    pub(crate) suggested_hak: Option<std::path::PathBuf>,
    /// Add Haks and Talk Table: what goes where, before it's done.
    pub attach: Option<module_props::AttachDraft>,
    /// Build › Publish to NWSync.
    pub publish: Option<nwsync_view::Publish>,
    /// Tilesets open in the tileset editor.
    pub tilesets: Vec<tileset_view::TilesetDoc>,
    pub(crate) next_tileset: u32,
    /// A tileset whose tab is being closed with unsaved changes.
    pub(crate) tileset_closing: Option<u32>,
    pub model_views: HashMap<model_view::Source, model_view::ModelView>,
    /// The areas' names as last read (Options › General: Show areas by
    /// name).
    pub(crate) area_names: tree::AreaNames,
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
    /// The References tab.
    pub references: references::References,
    /// The Rename window, while open.
    pub rename: Option<references::RenameDraft>,
    /// The script editor's code navigation and errors as you type.
    pub script_nav: script_nav::Nav,
    /// The Save as Prefab window: the name being typed and the objects.
    pub prefab_save: Option<(String, area_view::ObjectClip)>,
    /// The Update Instances window.
    pub update_draft: Option<bulk::UpdateDraft>,
    /// The Find and Replace Text window.
    pub text_replace: Option<bulk::TextReplace>,
    /// Where prefabs are kept (the app sets Moonglow's data folder's
    /// `prefabs`; none: prefabs can't be saved).
    pub prefab_dir: Option<PathBuf>,
    /// Where variable sets are kept (`var_sets`).
    pub var_set_dir: Option<PathBuf>,
    /// When changed haks and folders were last looked for.
    reload_checked: Option<std::time::Instant>,
    /// The custom talk table's file and its time when loaded.
    tlk_stamp: Option<(PathBuf, std::time::SystemTime)>,
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
    /// Where recovery copies of unsaved work go (Options › General); none
    /// until the desktop app gives its data folder.
    pub recovery_dir: Option<std::path::PathBuf>,
    autosave: recovery::Autosave,
    /// Recovery copies a session that did not end left (Recover Unsaved
    /// Work).
    pub recoveries: Vec<recovery::Copy>,
    /// Where Print writes the pages it opens in the browser.
    pub print_dir: std::path::PathBuf,
    /// When conversations were last backed up, and what was written.
    dialog_backup_at: Option<std::time::Instant>,
    dialog_backed_up: HashMap<ResKey, Vec<u8>>,
    /// Help › User Manual's chapter and rendering cache.
    manual: manual::Manual,
    /// Help › About is open.
    pub about: bool,
    /// The window's content area at the last frame (where new windows go).
    screen: Option<egui::Rect>,
    /// The widths of the module tree and of the panes beside it at the last
    /// frame (the Palettes pane opens as wide as the tree).
    tree_width: Option<f32>,
    dock_width: Option<f32>,
    /// A module was opened: the Palettes pane opens once the module tree
    /// shows (and has a width to match). Tests of other windows turn it
    /// off.
    pub open_palette: bool,
    /// The job under way, if one is (`jobs`).
    pub(crate) job: Option<jobs::Job>,
    /// Jobs run while the window keeps drawing (the application turns
    /// this on); off, starting a job waits for it, as tests want.
    pub background_jobs: bool,
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
            palettes: HashMap::new(),
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
            thumbnails: Default::default(),
            talk: None,
            talk_view: Default::default(),
            haks_layered: Vec::new(),
            keymap: Default::default(),
            keymap_from: Default::default(),
            haks: Vec::new(),
            next_hak: 0,
            hak_closing: None,
            suggested_hak: None,
            attach: None,
            publish: None,
            tilesets: Vec::new(),
            next_tileset: 0,
            tileset_closing: None,
            model_views: HashMap::new(),
            area_names: Default::default(),
            area_views: HashMap::new(),
            adjust: None,
            find_instance: None,
            area_focus: None,
            references: Default::default(),
            rename: None,
            script_nav: Default::default(),
            prefab_save: None,
            update_draft: None,
            text_replace: None,
            prefab_dir: None,
            var_set_dir: None,
            reload_checked: None,
            tlk_stamp: None,
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
            recovery_dir: None,
            autosave: Default::default(),
            recoveries: Vec::new(),
            print_dir: std::env::temp_dir().join("moonglow-print"),
            dialog_backup_at: None,
            dialog_backed_up: HashMap::new(),
            manual: Default::default(),
            about: false,
            screen: None,
            tree_width: None,
            dock_width: None,
            open_palette: false,
            job: None,
            background_jobs: false,
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
        widgets::install_fonts(ui.ctx());
        self.screen = Some(ui.ctx().content_rect());
        if std::mem::take(&mut self.minimize_requested) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        self.poll_job(ui.ctx());
        // (A running job has the window to itself.)
        if !self.busy() {
            self.shortcuts(ui);
        }
        egui::Panel::top("menu").show(ui, |ui| {
            self.menu(ui);
            self.toolbar(ui);
        });
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        // The log shrinks to a single line if the user wants it that small.
        let line = ui.text_style_height(&egui::TextStyle::Monospace);
        egui::Panel::bottom("log")
            .resizable(true)
            .default_size(120.0)
            .min_size(line + 6.0)
            .show(ui, |ui| self.log_ui(ui));
        if self.ws.is_some() {
            let tree =
                egui::Panel::left("tree").resizable(true).default_size(240.0).show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| tree::module_tree(self, ui));
                });
            self.tree_width = Some(tree.response.rect.width());
            // The palette beside a newly opened module, as in Aurora.
            if std::mem::take(&mut self.open_palette) && self.game.is_some() {
                self.actions.push(Action::OpenTab(Tab::Palette));
            }
        }
        self.heard = None;
        egui::CentralPanel::default().show(ui, |ui| {
            self.dock_width = Some(ui.available_width());
            let mut dock = std::mem::replace(&mut self.dock, DockState::new(Vec::new()));
            {
                let mut viewer = tabs::Viewer { app: self };
                DockArea::new(&mut dock).show_close_buttons(true).show_inside(ui, &mut viewer);
            }
            self.dock = dock;
        });
        self.backup_timer(ui);
        self.autosave_timer(ui);
        self.reload_timer(ui);
        recovery::window(self, ui);
        manual::about_window(self, ui.ctx());
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
        references::rename_window(self, ui.ctx());
        script_nav::rename_window(self, ui.ctx());
        prefabs::save_window(self, ui.ctx());
        bulk::update_window(self, ui.ctx());
        hak_view::closing_window(self, ui.ctx());
        module_props::attach_window(self, ui.ctx());
        nwsync_view::window(self, ui.ctx());
        tileset_view::closing_window(self, ui.ctx());
        bulk::text_window(self, ui.ctx());
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
        jobs::window(self, ui.ctx());
        if !self.actions.is_empty() {
            self.run_actions();
            // Show the result now, not at the next input event.
            ui.ctx().request_repaint();
        }
    }

    /// Where the main panes split for the Palettes pane on the right: the
    /// left side's share, leaving the palette as wide as the module tree.
    /// (egui_dock's split fraction is always the left side's.)
    fn palette_split(&self) -> f32 {
        match (self.tree_width, self.dock_width) {
            (Some(tree), Some(dock)) if dock > 0.0 => (1.0 - tree / dock).clamp(0.5, 0.9),
            _ => 0.72,
        }
    }

    /// Opens a tab in a window of its own, sized for it and fitted to the
    /// screen, centred over the main pane (clear of the palette's), each a
    /// little below and right of the last.
    fn open_window(&mut self, tab: Tab) {
        let screen = self
            .screen
            .unwrap_or(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0)));
        let palette = self.dock.find_tab(&Tab::Palette).map(|p| (p.surface, p.node));
        let main = self
            .dock
            .iter_leaves()
            .find(|(p, _)| p.surface.is_main() && Some((p.surface, p.node)) != palette)
            .map(|(_, leaf)| leaf.rect)
            .filter(|r| r.is_positive())
            .unwrap_or(screen);
        // Below the menu and toolbar (the main pane's top), never over them.
        let room = egui::Rect::from_min_max(egui::pos2(screen.left(), main.top()), screen.max);
        let want = tab.window_size();
        // Some of the main pane (the area view) shows beside it.
        let wide = (main.width() * 0.85).max(420.0).min(room.width());
        let size = egui::vec2(want.x.min(wide), want.y.min(room.height() * 0.9));
        let windows = self
            .dock
            .iter_surfaces()
            .filter(|s| matches!(s, egui_dock::Surface::Window(..)))
            .count();
        let step = 28.0 * (windows % 8) as f32;
        let mut at = main.center() - size / 2.0 + egui::vec2(step, step);
        at.x = at.x.min(room.right() - size.x).max(room.left());
        at.y = at.y.min(room.bottom() - size.y).max(room.top());
        let surface = self.dock.add_window(vec![tab]);
        if let Some(state) = self.dock.get_window_state_mut(surface) {
            state.set_position(at).set_size(size);
        }
    }

    fn shortcuts(&mut self, ui: &mut egui::Ui) {
        // The keys as Options › Keyboard last set them.
        if self.keymap_from != self.settings.key_bindings {
            self.keymap = keys::Keymap::new(&self.settings.key_bindings);
            self.keymap_from = self.settings.key_bindings.clone();
        }
        // Options › Keyboard takes the next key itself.
        if self.options.as_ref().is_some_and(|o| o.recording.is_some()) {
            return;
        }
        commands::keys_pressed(self, ui);
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        commands::menu_bar(self, ui);
    }

    /// Buttons for the common commands, below the menu as in Aurora.
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        commands::toolbar(self, ui);
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
        // No minimum of its own (egui's is 64 points), so the panel decides.
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .show(ui, |ui| {
                for (level, msg) in &self.log.entries {
                    let color = match level {
                        Level::Info => ui.visuals().text_color(),
                        Level::Warning => ui.visuals().warn_fg_color,
                        Level::Error => ui.visuals().error_fg_color,
                    };
                    ui.label(egui::RichText::new(msg).color(color).monospace());
                }
            });
    }

    /// Opens a module: the workspace, and the game data with the module's
    /// haks and the module layered in.
    pub fn open_module(&mut self, path: &std::path::Path) {
        match Module::open(path) {
            Ok(m) => {
                match &m.project {
                    Some(p) => self.log.info(format!(
                        "Opened nasher project {}, target {} ({} resources)",
                        p.root().display(),
                        p.target,
                        m.len()
                    )),
                    None => {
                        self.log.info(format!("Opened {} ({} resources)", path.display(), m.len()))
                    }
                }
                let remembered = m.location.as_ref().map(|l| l.path().to_path_buf());
                self.use_module(m);
                self.take_project_warnings();
                self.settings.remember(remembered.as_deref().unwrap_or(path));
            }
            Err(e) => self.log.error(format!("Could not open {}: {e}", path.display())),
        }
    }

    /// Makes a module the open one, with its haks and itself layered into
    /// the game data.
    fn use_module(&mut self, m: Module) {
        self.close();
        if let (Some(game), Some(gi)) = (exclusive(&mut self.game), &self.install) {
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
            self.haks_layered = haks;
        }
        self.ws = Some(Workspace::new(m));
        self.area_names = Default::default();
        self.dock = DockState::new(vec![Tab::ModuleProperties]);
        self.open_palette = true;
        // On the area's tiles: what a new module needs first.
        self.palette.tiles = true;
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

    /// Loads the module's custom talk table (from its haks, the module or
    /// the user's `tlk/` folder, as the game looks) when the module names
    /// another one than is loaded.
    /// Reads the module's custom talk table again (after it was saved).
    pub(crate) fn reload_custom_tlk(&mut self) {
        self.custom_tlk = None;
        self.load_custom_tlk();
    }

    fn load_custom_tlk(&mut self) {
        let (Some(ws), Some(game)) = (&self.ws, exclusive(&mut self.game)) else { return };
        let name = ws.module.custom_tlk().ok().flatten().filter(|n| !n.trim().is_empty());
        if name == self.custom_tlk {
            return;
        }
        self.custom_tlk = name.clone();
        let Some(name) = name else {
            game.set_custom_tlk(None);
            return;
        };
        let dirs = self.install.as_ref().map(|i| i.tlk_dirs()).unwrap_or_default();
        let found = mg_module::talk::find(&game.resman, &dirs, &name);
        let stamp = found.as_ref().and_then(|f| f.source.file()).and_then(|path| {
            let t = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
            Some((path.to_path_buf(), t))
        });
        let data = found.map(|f| f.data);
        self.tlk_stamp = stamp;
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
            || self.talk.as_ref().is_some_and(|t| t.is_dirty() && t.editable())
            || hak_view::unsaved(self)
            || tileset_view::unsaved(self)
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
        let edits = ws.edits_to(&staged);
        match self.apply(Command::new(format!("New area {area}"), edits)) {
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
        // Closed (saved, or its changes discarded): no recovery copy.
        self.forget_recovery();
        self.ws = None;
        self.scripts.clear();
        self.buffers.clear();
        self.talk = None;
        self.talk_view = Default::default();
        self.dock = DockState::new(vec![Tab::Welcome]);
        self.load_order_changed();
        // (Another module's haks may have other tilesets of these names.)
        self.palette.forget_game_data();
        if let Some(game) = exclusive(&mut self.game) {
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
        let (Some(game), Some(ws)) = (exclusive(&mut self.game), &mut self.ws) else { return };
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
                | Action::OpenFolderDialog
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
            Action::OpenFolderDialog => {
                if let Some(p) =
                    self.dialogs.pick_folder("Open Module Folder or nasher Project", None)
                {
                    self.open_module(&p);
                }
            }
            Action::SaveAsProjectDialog => {
                if let Some(root) = self
                    .dialogs
                    .pick_folder("Save As nasher Project", self.module_path().as_deref())
                {
                    let target = match &self.ws {
                        Some(ws) => match &ws.module.location {
                            Some(ModuleLocation::Project { root: r, target }) if *r == root => {
                                target.clone()
                            }
                            _ => "default".into(),
                        },
                        None => return,
                    };
                    self.save(Some(ModuleLocation::Project { root, target }));
                }
            }
            Action::PackTarget => _ = self.pack_target(),
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
                    let loc = if p.extension().is_some() {
                        ModuleLocation::Archive(p)
                    } else {
                        ModuleLocation::Folder(p)
                    };
                    self.save(Some(loc));
                }
            }
            Action::Close => self.close(),
            Action::Undo | Action::Redo => {
                let Some(ws) = &mut self.ws else { return };
                let r = if action == Action::Undo { ws.undo() } else { ws.redo() };
                match r {
                    Ok(Some(label)) => {
                        self.scripts.clear();
                        self.sync_haks();
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
                if let Err(e) = self.apply(cmd) {
                    self.log.error(e.to_string());
                }
            }
            Action::OpenTab(tab) => {
                // A tab docked in an area's pane (Module Properties, docked
                // when the module opened) moves to a window of its own
                // rather than come to the front over the area.
                if !tab.docks()
                    && tab != Tab::Palette
                    && let Some(path) = self.dock.find_tab(&tab)
                    && let Ok(leaf) = self
                        .dock
                        .leaf(egui_dock::NodePath { surface: path.surface, node: path.node })
                    && leaf.tabs.iter().any(|t| matches!(t, Tab::Area(_)))
                {
                    self.dock.remove_tab(path);
                }
                if self.dock.find_tab(&tab).is_none() {
                    if tab == Tab::Palette {
                        // Its own pane on the right, as in Aurora (the only
                        // one when every other tab was closed).
                        let fraction = self.palette_split();
                        let main = self.dock.main_surface_mut();
                        if main.root_node().is_none_or(|n| n.is_empty()) {
                            *main = egui_dock::Tree::new(vec![tab.clone()]);
                        } else {
                            main.split_right(
                                egui_dock::NodeIndex::root(),
                                fraction,
                                vec![tab.clone()],
                            );
                        }
                    } else if !tab.docks() {
                        self.open_window(tab.clone());
                    } else {
                        // Into the main window, not the palette's pane: the
                        // focused pane, else the first other one. (Right after
                        // the palette's pane is made, egui_dock reports no
                        // focused pane but would push into the palette's; an
                        // editor window that has the focus isn't a place for
                        // an area.)
                        let palette =
                            self.dock.find_tab(&Tab::Palette).map(|p| (p.surface, p.node));
                        let other = |p: &egui_dock::NodePath| {
                            p.surface.is_main() && Some((p.surface, p.node)) != palette
                        };
                        let target = self
                            .dock
                            .focused_leaf()
                            .filter(other)
                            .or_else(|| self.dock.iter_leaves().map(|(p, _)| p).find(other));
                        match (target, palette) {
                            (Some(pane), _) => {
                                self.dock.set_focused_node_and_surface(pane);
                                self.dock.push_to_focused_leaf(tab.clone());
                            }
                            // The palette's is the only pane: a new one on its left.
                            (None, Some((_, node))) => {
                                let fraction = self.palette_split();
                                self.dock.main_surface_mut().split_left(
                                    node,
                                    fraction,
                                    vec![tab.clone()],
                                );
                            }
                            (None, None) => self.dock.push_to_first_leaf(tab.clone()),
                        }
                    }
                }
                // Bring it to the front, new or not.
                if let Some(path) = self.dock.find_tab(&tab) {
                    let _ = self.dock.set_active_tab(path);
                }
            }
            Action::RenameBlueprint { from, to } => blueprint::rename(self, from, to),
            Action::FindReferences(k) => self.find_references(references::Query::Resource(k)),
            Action::FindTag(t) => self.find_references(references::Query::Tag(t)),
            Action::RenameDialog(k) => self.rename_dialog(k),
            Action::CompileScripts => self.compile_scripts(),
            Action::Verify => self.verify(),
            Action::TestModule => self.test_module(false),
            Action::TestModuleChoose => self.test_module(true),
            Action::TestFromHere { area, at, facing } => self.test_from_here(area, at, facing),
            Action::ReloadResources => self.reload_resources(true),
            Action::SaveTalkTable => {
                talk_view::save(self);
            }
            Action::ExportMinimap(area) => self.export_minimap(area),
            Action::PlacePrefab(name) => self.place_prefab(&name),
            Action::Quit => {
                // Saved or discarded by now: no recovery copy.
                self.forget_recovery();
                self.quit_requested = true;
            }
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
        // (An import can bring hundreds: the first few say what to look for.)
        const MOST: usize = 8;
        let more = warnings.len().saturating_sub(MOST);
        for w in warnings.into_iter().take(MOST) {
            self.log.warn(w);
        }
        if more > 0 {
            self.log.warn(format!(
                "… and {more} more like these (Module Properties › Custom Content › \
                 Check for Conflicts lists what haks override)"
            ));
        }
    }

    /// Applies a command to the open module: the one way a change gets
    /// into it, whoever makes it (an editor, a wizard, an import, a build).
    /// It warns of new resources that shadow others, brings the values
    /// derived from what changed up to date as part of the command (an
    /// item's cost, a creature's hit points), and follows a changed hak list
    /// or talk table. On failure nothing is changed.
    pub(crate) fn apply(&mut self, cmd: Command) -> Result<(), mg_edit::EditError> {
        self.warn_shadowing(&cmd);
        let Some(ws) = &mut self.ws else { return Ok(()) };
        let custom_tlk = cmd.edits.iter().any(
            |e| matches!(e, mg_edit::Edit::SetField { label, .. } if label == "Mod_CustomTlk"),
        );
        let derived = blueprint::derived_of(&cmd);
        ws.apply(cmd)?;
        blueprint::after_apply(self, derived);
        if custom_tlk && self.ws.as_mut().is_some_and(|ws| ws.flush().is_ok()) {
            self.load_custom_tlk();
        }
        self.sync_haks();
        Ok(())
    }

    /// Puts the script editors' unsaved text into the module (one undoable
    /// command).
    pub(crate) fn store_script_text(&mut self) {
        if self.ws.is_none() {
            return;
        }
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
            if let Err(e) = self.apply(Command::new(format!("Save {n} script(s)"), edits)) {
                self.log.error(e.to_string());
            }
        }
    }

    fn save(&mut self, to: Option<ModuleLocation>) {
        // Options > General: Build module on save.
        if self.settings.build_on_save && self.ws.is_some() {
            build_view::build_on_save(self);
        }
        // Script editors' unsaved text goes into the module first, as saving
        // everything means; so does the talk table, a file of its own.
        self.store_script_text();
        talk_view::save(self);
        hak_view::save_all(self);
        tileset_view::save_all(self);
        let Some(ws) = &mut self.ws else { return };
        // The custom palettes list the module's blueprints, as Aurora keeps
        // them.
        if let Some(game) = &self.game {
            let rebuilt = ws
                .derive(|module| mg_module::palette::rebuild_custom_palettes(module, game))
                .map_err(|e| e.to_string())
                .and_then(|made| made);
            if let Err(e) = rebuilt {
                self.log.error(format!("Custom palettes: {e}"));
            }
        }
        // Options › General: the module as it was, kept as
        // `<name>.BackupMod` (Aurora's name) before it is overwritten.
        let target = match &to {
            Some(ModuleLocation::Archive(p)) => Some(p.clone()),
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
            Some(loc) => ws.save_as(&loc).map_err(|e| e.to_string()),
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
                self.forget_recovery();
            }
            Err(e) => {
                self.log.error(format!("Save failed: {e}"));
                if e.contains("changed on disk") {
                    self.log.warn(
                        "Nothing was written. Reopen the project to load those files, \
                         or save into another folder",
                    );
                }
            }
        }
        self.take_project_warnings();
        self.refresh_module_layer();
    }

    /// Logs (and clears) what the open nasher project has to say.
    fn take_project_warnings(&mut self) {
        let Some(ws) = &mut self.ws else { return };
        let Some(project) = ws.module.project.as_mut() else { return };
        for w in std::mem::take(&mut project.warnings) {
            self.log.warn(w);
        }
    }

    /// Build › Pack Target: writes a nasher project's module file (its
    /// target's `file`), scripts compiled, as `nasher pack` would. Returns
    /// where it went.
    fn pack_target(&mut self) -> Option<PathBuf> {
        self.compile_uncompiled();
        let ws = self.ws.as_mut()?;
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return None;
        }
        match ws.module.target_archive() {
            Ok(Some((path, bytes))) => {
                let written = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .and_then(|()| std::fs::write(&path, bytes));
                match written {
                    Ok(()) => {
                        self.log.info(format!("Packed {}", path.display()));
                        Some(path)
                    }
                    Err(e) => {
                        self.log.error(format!("{}: {e}", path.display()));
                        None
                    }
                }
            }
            Ok(None) => {
                self.log.error("Pack Target: the module isn't in a nasher project");
                None
            }
            Err(e) => {
                self.log.error(format!("Pack Target: {e}"));
                None
            }
        }
    }

    /// Compiles the scripts that have no compiled version (a project's tree
    /// holds none).
    fn compile_uncompiled(&mut self) {
        let (Some(ws), Some(game)) = (&mut self.ws, &self.game) else { return };
        let Ok(results) = ws.derive(|module| {
            mg_module::build::compile_scripts(
                module,
                &game.resman,
                mg_module::build::ScriptSelection::Uncompiled,
            )
        }) else {
            return;
        };
        let failed = results.iter().filter(|r| r.result.is_err()).count();
        if failed > 0 {
            self.log.warn(format!("{failed} script(s) didn't compile; see Build › Compile"));
        }
    }

    /// Compile All Scripts, as a job: the bytecode that changed is stored
    /// through one undoable command when it is done.
    fn compile_scripts(&mut self) {
        if self.ws.is_none() || self.game.is_none() {
            self.log.error("Compiling needs an open module and the game data");
            return;
        }
        self.start_job(
            "Compile All Scripts",
            |job| {
                let game = job.game.as_deref()?;
                let mut staged = job.module.clone();
                let results = mg_module::build::compile_scripts(
                    &mut staged,
                    &game.resman,
                    mg_module::build::ScriptSelection::All,
                );
                let edits: Vec<mg_edit::Edit> = staged
                    .keys_of(ResType::NCS)
                    .filter(|k| staged.get(k) != job.module.get(k))
                    .map(|k| mg_edit::Edit::SetResource {
                        key: *k,
                        data: staged.get(k).map(<[u8]>::to_vec),
                    })
                    .collect();
                Some((results, edits))
            },
            |app, made| {
                let Some((results, edits)) = made else { return };
                let failed: Vec<String> = results
                    .iter()
                    .filter_map(|r| r.result.as_ref().err().map(|e| e.message.clone()))
                    .collect();
                for f in &failed {
                    app.log.error(f.clone());
                }
                let changed = edits.len();
                if !edits.is_empty()
                    && let Err(e) = app.apply(Command::new("Compile scripts", edits))
                {
                    app.log.error(e.to_string());
                }
                app.log.info(format!(
                    "Compiled {} scripts: {} failed, {changed} changed",
                    results.len(),
                    failed.len()
                ));
            },
        );
    }

    /// Starts the game on the saved module (Test Module).
    fn test_module(&mut self, choose: bool) {
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
        let Some(user) = install.user_dir.clone() else {
            self.log.error("Test Module needs the game's user folder (Options › Folders)");
            return;
        };
        let user = user.as_path();
        // A nasher project is packed into the modules folder, as nasher's
        // install does.
        if self.ws.as_ref().is_some_and(|ws| ws.module.project.is_some()) {
            let Some(packed) = self.pack_target() else { return };
            let Some(file) = packed.file_name() else { return };
            let dest = user.join("modules").join(file);
            let copied = std::fs::create_dir_all(user.join("modules"))
                .and_then(|()| std::fs::copy(&packed, &dest).map(|_| ()));
            if let Err(e) = copied {
                self.log.error(format!("Test Module: {}: {e}", dest.display()));
                return;
            }
            self.log.info(format!("Installed {}", dest.display()));
            let name = packed.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            self.launch_test(&client, user, &name, choose);
            return;
        }
        let Some(path) = self.module_path() else {
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
        self.launch_test(&client, user, &name, choose);
    }

    /// Test From Here: the module as it is now, not saved, written to the
    /// modules folder as [`test_module::FROM_HERE`] with its start moved,
    /// and the game started on it.
    fn test_from_here(&mut self, area: mg_core::ResRef, at: [f32; 3], facing: f32) {
        let Some(install) = self.install.clone() else {
            self.log.error("Test From Here needs the game");
            return;
        };
        let (Some(client), Some(user)) =
            (test_module::client_binary(install.root.as_path()), install.user_dir.clone())
        else {
            self.log.error("Test From Here: no game client, or no user folder (Options › Folders)");
            return;
        };
        let bytes = self.test_from_here_archive(area, at, facing);
        let dest = user.join("modules").join(format!("{}.mod", test_module::FROM_HERE));
        let written = bytes.and_then(|b| {
            std::fs::create_dir_all(user.join("modules"))
                .and_then(|()| std::fs::write(&dest, b))
                .map_err(|e| format!("{}: {e}", dest.display()))
        });
        match written {
            Ok(()) => {
                self.log.info(format!(
                    "Testing from {area} ({:.1}, {:.1}): {} has the module as it is now",
                    at[0],
                    at[1],
                    dest.display()
                ));
                self.launch_test(&client, &user, test_module::FROM_HERE, false);
            }
            Err(e) => self.log.error(format!("Test From Here: {e}")),
        }
    }

    /// The module Test From Here plays: as it is now (script editors' text
    /// and uncompiled scripts compiled, nothing saved), its start moved; a
    /// nasher project's packed as its target.
    pub fn test_from_here_archive(
        &mut self,
        area: mg_core::ResRef,
        at: [f32; 3],
        facing: f32,
    ) -> Result<Vec<u8>, String> {
        self.store_script_text();
        self.compile_uncompiled();
        let ws = self.ws.as_mut().ok_or("no module")?;
        ws.flush().map_err(|e| e.to_string())?;
        let mut m = ws.module.clone();
        m.set_start(area, at, facing).map_err(|e| e.to_string())?;
        match m.target_archive().map_err(|e| e.to_string())? {
            Some((_, bytes)) => Ok(bytes),
            None => m.to_archive_bytes().map_err(|e| e.to_string()),
        }
    }

    /// Reads again the haks and the override, development and portrait
    /// folders that changed on disk (and the custom talk table), and
    /// refreshes what was shown from them. Asked for (Tools › Reload
    /// Resources), it says when nothing changed.
    pub fn reload_resources(&mut self, asked: bool) {
        use mg_resman::priority as p;
        let Some(game) = exclusive(&mut self.game) else { return };
        // The user's content; the install's own folders don't change.
        let user = |l: &mg_resman::Layer| {
            matches!(
                l.priority,
                p::HAK
                    | p::HAK_USER
                    | p::OVERRIDE
                    | p::DEVELOPMENT
                    | p::DEVELOPMENT_USER
                    | p::PORTRAITS_USER
            )
        };
        let mut changed = match game.resman.reload_changed(user) {
            Ok(c) => c,
            Err(e) => {
                self.log.error(format!("Reload Resources: {e}"));
                return;
            }
        };
        let tlk_now = self.tlk_stamp.as_ref().and_then(|(path, _)| {
            Some((path.clone(), std::fs::metadata(path).ok()?.modified().ok()?))
        });
        if tlk_now.is_some() && tlk_now != self.tlk_stamp {
            changed.push("custom talk table".into());
            self.custom_tlk = None;
            self.load_custom_tlk();
        }
        if changed.is_empty() {
            if asked {
                self.log.info("Reload Resources: nothing changed");
            }
            return;
        }
        self.game_data_changed();
        self.log.info(format!("Reloaded {}", changed.join(", ")));
    }

    /// What was shown from the game data is shown anew: its layers changed.
    fn game_data_changed(&mut self) {
        if let Some(game) = exclusive(&mut self.game) {
            game.invalidate();
        }
        self.pictures = Default::default();
        self.palettes = Default::default();
        self.palette.forget_game_data();
        self.model_views.clear();
        for view in self.area_views.values_mut() {
            view.reload();
        }
        self.load_order_changed();
    }

    /// Writes an area's minimap (as it is now, unsaved tiles included) to a
    /// PNG the user chooses.
    fn export_minimap(&mut self, area: mg_core::ResRef) {
        let (Some(ws), Some(game)) = (&mut self.ws, &self.game) else { return };
        let Ok(are) = ws.doc(&ResKey::new(area, ResType::ARE)).map(|g| g.root.clone()) else {
            return;
        };
        let image = mg_module::minimap::tileset(&game.resman, &are)
            .and_then(|set| mg_module::minimap::minimap(&game.resman, &are, &set, None))
            .and_then(|i| mg_module::minimap::png(&i));
        let data = match image {
            Ok(d) => d,
            Err(e) => {
                self.log.error(format!("Minimap of {area}: {e}"));
                return;
            }
        };
        let suggested = std::path::PathBuf::from(format!("{area}.png"));
        let Some(path) = self.dialogs.save_file(dialogs::FileKind::Png, Some(&suggested)) else {
            return;
        };
        match std::fs::write(&path, data) {
            Ok(()) => self.log.info(format!("Saved the minimap of {area} to {}", path.display())),
            Err(e) => self.log.error(format!("{}: {e}", path.display())),
        }
    }

    /// The haks Module Properties lists now, unsaved changes included.
    fn listed_haks(&mut self) -> Vec<String> {
        use mg_schema::StructExt;
        let Some(ws) = &mut self.ws else { return Vec::new() };
        let key = ResKey::parse("module", ResType::IFO).expect("valid");
        let Ok(info) = ws.doc(&key) else { return Vec::new() };
        let text = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
        let mut haks: Vec<String> = info
            .root
            .items(&mg_schema::ifo::MOD_HAK_LIST)
            .iter()
            .map(|h| text(h.read(&mg_schema::ifo::mod_hak_list::MOD_HAK).as_bytes()))
            .filter(|h| !h.is_empty())
            .collect();
        if haks.is_empty() {
            haks.extend(
                Some(text(info.root.read(&mg_schema::ifo::MOD_HAK).as_bytes()))
                    .filter(|h| !h.is_empty()),
            );
        }
        haks
    }

    /// Layers the module's haks as Module Properties lists them, when the
    /// list changed (an edit, an undo): no reopening needed.
    pub(crate) fn sync_haks(&mut self) {
        let listed = self.listed_haks();
        if listed == self.haks_layered {
            return;
        }
        let (Some(game), Some(gi)) = (exclusive(&mut self.game), &self.install) else { return };
        let old: Vec<String> = game
            .resman
            .layers()
            .iter()
            .filter(|l| l.label.starts_with("hak:"))
            .map(|l| l.label.clone())
            .collect();
        for l in old {
            game.resman.remove(&l);
        }
        match game.resman.add_haks(gi, &listed.iter().map(String::as_str).collect::<Vec<_>>()) {
            Ok(missing) => {
                for h in missing {
                    self.log.warn(format!("Hak {h} not found"));
                }
            }
            Err(e) => self.log.error(format!("Could not open the module's haks: {e}")),
        }
        self.haks_layered = listed;
        self.game_data_changed();
        self.reload_custom_tlk();
        self.log.info("The haks changed: the game data now has them as listed");
    }

    /// Every few seconds, with Options › General's reloading on: the haks
    /// and folders that changed.
    fn reload_timer(&mut self, ui: &egui::Ui) {
        // (Not under a job, which reads the game data.)
        if self.settings.no_auto_reload || self.game.is_none() || self.busy() {
            return;
        }
        let every = std::time::Duration::from_secs(3);
        let now = std::time::Instant::now();
        if self.reload_checked.is_some_and(|t| now.duration_since(t) < every) {
            ui.ctx().request_repaint_after(every);
            return;
        }
        self.reload_checked = Some(now);
        self.reload_resources(false);
        ui.ctx().request_repaint_after(every);
    }

    fn launch_test(
        &mut self,
        client: &std::path::Path,
        user: &std::path::Path,
        name: &str,
        choose: bool,
    ) {
        match test_module::command(client, user, name, choose).spawn() {
            Ok(_) => {
                self.log.info(format!("Testing {name}"));
                // Options > General: Minimize Toolset on test module.
                self.minimize_requested = self.settings.minimize_on_test;
            }
            Err(e) => self.log.error(format!("Test Module: {}: {e}", client.display())),
        }
    }

    /// Verify Module, as a job: what is missing, and what is wrong with the
    /// custom content, into the log.
    fn verify(&mut self) {
        if self.ws.is_none() || self.game.is_none() {
            return;
        }
        self.start_job(
            "Verify Module",
            |job| {
                let game = job.game.as_deref()?;
                job.progress.say("Looking for what is missing");
                let missing = mg_module::verify::missing(&job.module, &game.resman);
                if job.progress.cancelled() {
                    return None;
                }
                // The custom content: tilesets, 2DAs, materials, objects
                // naming rows that don't exist.
                job.progress.say("Checking the custom content");
                let tlk = mg_module::doctor::TalkTables {
                    base: game.tlk().entries.len(),
                    custom: game.custom_tlk().map(|t| t.entries.len()),
                };
                let findings = mg_module::doctor::examine(&job.module, &game.resman, tlk);
                Some((missing, findings))
            },
            |app, made| {
                let Some((missing, findings)) = made else { return };
                for m in &missing {
                    let what = if m.uncompiled { "is not compiled" } else { "is missing" };
                    let text = format!(
                        "{:?}: {}{} → {} {} {what}",
                        m.category,
                        m.reference.from,
                        m.reference.path,
                        m.reference.kind.name(),
                        m.reference.target
                    );
                    if m.is_error() { app.log.error(text) } else { app.log.warn(text) }
                }
                for f in &findings {
                    let at = if f.at.is_empty() { String::new() } else { format!(" › {}", f.at) };
                    let text = format!("{} › {}{at}: {}", f.source, f.resource, f.message);
                    match f.severity {
                        mg_module::doctor::Severity::Error => app.log.error(text),
                        mg_module::doctor::Severity::Warning => app.log.warn(text),
                    }
                }
                let errors = missing.iter().filter(|m| m.is_error()).count()
                    + findings
                        .iter()
                        .filter(|f| f.severity == mg_module::doctor::Severity::Error)
                        .count();
                let warnings = missing.len() + findings.len() - errors;
                app.log.info(format!(
                    "Verify: {errors} error(s), {warnings} warning(s) ({} missing reference(s), {} content problem(s))",
                    missing.len(),
                    findings.len()
                ));
            },
        );
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
        self.game.as_deref().map(|g| &g.resman)
    }
}

/// Loads the game data of an install, logging the outcome.
/// The game data to change (its layers, its talk table): only while no job
/// is reading it, which is whenever the window takes input.
pub(crate) fn exclusive(game: &mut Option<Arc<GameData>>) -> Option<&mut GameData> {
    game.as_mut().and_then(Arc::get_mut)
}

fn load_game(install: Option<&GameInstall>, log: &mut Log) -> Option<Arc<GameData>> {
    match install {
        Some(gi) => match GameData::open(gi) {
            Ok(g) => {
                log.info(format!("Game data loaded from {}", gi.root.display()));
                Some(Arc::new(g))
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
