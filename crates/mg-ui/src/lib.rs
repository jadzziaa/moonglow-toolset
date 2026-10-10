//! Moonglow's user interface (egui). [`Moonglow`] is the whole application
//! state; [`Moonglow::ui`] draws it into any `egui::Ui`, so the same code runs
//! in the desktop window and in `egui_kittest` tests. Widgets never change
//! the module directly: they queue [`Action`]s, which run after the frame and
//! turn edits into undoable [`mg_edit::Command`]s.

mod appearance_gallery;
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
pub mod copy_as;
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
mod outside;
pub mod palette_categories;
pub mod palette_view;
pub mod plugins;
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
pub mod trace;
mod transfer;
mod tree;
pub use tree::TreeAt;
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
use mg_resman::{GameInstall, LayerClass, ResKey, priority};
use mg_rules::GameData;

pub use browser::Browser;
pub use dialogs::{Dialogs, FileKind, NoDialogs};
/// Carrying on when one model fails (for the application's panic hook).
pub use mg_render::guard as model_guard;
pub use options::OptionsDraft;
pub use settings::Settings;
pub use tabs::Tab;
pub use text::set_edit_language;
pub use transfer::{ExportDraft, ImportDraft};
pub use wizards::{AreaWizard, Wizard};

/// Where a tool window is, or opens: a window of its own over the area
/// (as each does at first), a tab of the main pane beside the areas, or a
/// tab of the palettes' pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Window,
    Middle,
    Palettes,
}

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
    /// Writes resources as loose files (a script with its compiled script,
    /// an area with its `.git` and `.gic`; with `dependencies`, the module
    /// resources they use too): into the scratch folder (Options › Folders;
    /// asked for the first time), or a folder asked for.
    ExportFiles {
        keys: Vec<ResKey>,
        dependencies: bool,
        scratch: bool,
    },
    Export(ExportDraft),
    ImportDialog,
    Import(ImportDraft),
    /// Reports the module's haks' conflicts.
    HakReport,
    OptionsDialog,
    /// Uses these folders for the game and the user directory (reloading the
    /// game data; closes the module).
    ApplyOptions(Box<OptionsDraft>),
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
    /// Makes these areas' placeables static, where nothing is lost by it.
    StaticPlaceables(Vec<mg_core::ResRef>),
    /// Opens a resource's editor, as a double click in the module tree
    /// does: a script in the external editor too, if Options say so.
    OpenResource(ResKey),
    /// The talk table's lines to a CSV file, or (true) read from one.
    TalkCsv(bool),
    /// The resource browser's Save As: a resource of the load order to a
    /// file.
    /// Find References for a talk-table line, by its StrRef.
    FindStrRef(u32),
    SaveResource(ResKey),
    /// A model of the load order saved as text (decompiled).
    SaveModelText(ResKey),
    /// The resource browser's Export as Files: those listed, into a folder.
    SaveResources(Vec<ResKey>),
    /// The same, as JSON (`nwn_tlk`'s).
    TalkJson(bool),
    /// The talk table editor opens a `.tlk` file anywhere, or (true) makes
    /// one.
    TalkFile(bool),
    /// Makes every static placeable of these areas dynamic.
    DynamicPlaceables(Vec<mg_core::ResRef>),
    Apply(Command),
    OpenTab(Tab),
    /// Closes a tab's window (Escape over a model's window).
    CloseTab(Tab),
    /// Closes tabs as their close buttons would: one with unsaved work of
    /// its own (a hak, a tileset) asks first and stays.
    CloseTabs(Vec<Tab>),
    /// A tab's own window made to fill the main pane, or put back as it
    /// was.
    ToggleMaximize(Tab),
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
    /// The scripts that include this one are compiled again (it was
    /// saved, with Automatically Compile Scripts on Save).
    CompileIncluders(ResKey),
    /// Edit › Prefabs › a prefab: placed with a click in the area shown.
    PlacePrefab(String),
    /// Edit › Find References: where a resource is used.
    FindReferences(ResKey),
    /// The Rename window for a resource.
    RenameDialog(ResKey),
    /// Opens the Placeable Gallery.
    PlaceableGallery,
    /// Copy…: asks for the copy's ResRef and Tag.
    CopyDialog(ResKey),
    /// Edit beside a script's name: opens the module's script, else the
    /// game's; a script that is nowhere is made in the module first (as
    /// Aurora does). `condition`: one that answers whether a conversation
    /// line shows.
    EditScript {
        name: mg_core::ResRef,
        condition: bool,
    },
    /// Asks whether to delete a resource of the module (the module tree's
    /// Delete…).
    DeleteDialog(ResKey),
    /// Deletes a resource of the module (an area with its objects, a
    /// script with its compiled one).
    DeleteResource(ResKey),
    /// Renames a blueprint (and points its editor at the new name).
    RenameBlueprint {
        from: ResKey,
        to: mg_core::ResRef,
    },
    CompileScripts,
    /// Build › Compile Models: the module's models kept as text.
    CompileModels,
    Verify,
    /// Saves, then starts the game on the module (F9).
    TestModule,
    Quit,
}

/// Where a window is: where it was last put, where its panes were when
/// first drawn there, and where that makes it now.
#[derive(Debug, Clone, Copy)]
struct WindowTrack {
    put: egui::Rect,
    /// Its panes last frame.
    seen: Option<egui::Rect>,
    first: Option<egui::Rect>,
    now: egui::Rect,
    /// Where its panes' corner should come to be (a window put back).
    aim: Option<egui::Pos2>,
}

/// How serious a log message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    /// The color its messages are written in.
    pub(crate) fn color(self, ui: &egui::Ui) -> egui::Color32 {
        match self {
            Level::Info => ui.visuals().text_color(),
            Level::Warning => ui.visuals().warn_fg_color,
            Level::Error => ui.visuals().error_fg_color,
        }
    }
}

/// The message log (Aurora's output pane).
#[derive(Debug, Clone, Default)]
pub struct Log {
    pub entries: Vec<(Level, String)>,
}

impl Log {
    pub fn info(&mut self, s: impl Into<String>) {
        self.said(Level::Info, s.into());
    }
    pub fn warn(&mut self, s: impl Into<String>) {
        self.said(Level::Warning, s.into());
    }
    /// A message of any level. (The debug log has the messages too.)
    pub(crate) fn said(&mut self, level: Level, s: String) {
        trace::note(format!("log {level:?}: {s}"));
        self.entries.push((level, s));
    }
    pub fn error(&mut self, s: impl Into<String>) {
        self.said(Level::Error, s.into());
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
    pub thumbnails: model_view::Thumbnails,
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
    /// Module Properties › Custom Content: the hak list's rows chosen.
    pub hak_list: module_props::HakList,
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
    /// The Palette Categories window, when open.
    pub palette_categories: Option<palette_categories::PaletteCategories>,
    /// The areas' names as last read (Options › General: Show areas by
    /// name).
    pub(crate) area_names: tree::AreaNames,
    /// What is placed in the areas opened out in the module tree, each as
    /// read at a revision of the workspace.
    pub(crate) area_contents: HashMap<mg_core::ResRef, (u64, tree::AreaContents)>,
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
    /// The items copied in an inventory (their blueprints), to paste into
    /// another's.
    pub item_clip: Vec<mg_core::ResRef>,
    /// An object to show and select when its area's view is next drawn.
    pub area_focus: Option<(mg_core::ResRef, mg_area::ObjectKind, usize)>,
    /// The References tab.
    pub references: references::References,
    /// What each script was compiled from when Compile All last compiled
    /// it (its stamp), and what those stamps were made with: a script
    /// whose stamp is the same is not compiled again.
    /// The text field typed in since it got the keyboard: Undo and Redo
    /// are its own there (`commands::keys_pressed`).
    pub(crate) text_typed: Option<egui::Id>,
    pub(crate) script_stamps: mg_module::build::Remembered,
    /// Where what Compile All remembers is kept from one session to the
    /// next (a file a module, in Moonglow's own folder); `None`: not kept.
    pub compiled_dir: Option<PathBuf>,
    /// The Rename window, while open.
    pub rename: Option<references::RenameDraft>,
    /// The Placeable Gallery, while it is open.
    pub placeable_gallery: Option<appearance_gallery::Gallery>,
    /// The Copy window (Edit Copy, the module tree's Copy…).
    pub copy_as: Option<copy_as::CopyDraft>,
    /// The script editor's code navigation and errors as you type.
    pub script_nav: script_nav::Nav,
    /// The Save as Prefab window: the name being typed and the objects.
    pub prefab_save: Option<(String, area_view::ObjectClip)>,
    /// The interface size was set from the settings (to set it back).
    scaled: bool,
    /// The models' windows open as the frame began.
    pub(crate) open_models: Vec<Tab>,
    /// The folder Export as Files last wrote to (offered again).
    pub(crate) export_dir: Option<PathBuf>,
    /// An Export Files (of a model) asking about files already there.
    pub model_export: Option<model_view::ModelExport>,
    /// The Update Instances window.
    pub update_draft: Option<bulk::UpdateDraft>,
    /// The Edit Areas Together window: the areas being chosen.
    pub(crate) area_chooser: Option<area_props::AreaChooser>,
    /// The Find and Replace Text window.
    pub text_replace: Option<bulk::TextReplace>,
    /// Where prefabs are kept (the app sets Moonglow's data folder's
    /// `prefabs`; none: prefabs can't be saved).
    pub prefab_dir: Option<PathBuf>,
    /// The prefab deleted last, its name and text: the palette's Undo
    /// Delete puts it back.
    pub prefab_deleted: Option<(String, String)>,
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
    /// The pass in which an area view last had something in hand (a
    /// brush, a paste, a drag, a terrain tool): Escape is its then.
    pub(crate) area_tool_at: u64,
    /// An armor's model is shown on a woman (the item's Appearance page
    /// chooses), rather than a man.
    pub armor_on_woman: bool,
    /// An area for the module tree to bring into view (and, if asked, to
    /// open out to what is placed in it): the area of the tab just chosen,
    /// or of its menu's Show in Module Tree.
    pub tree_reveal: Option<(mg_core::ResRef, bool)>,
    /// The area the module tree shows as the one in hand: the last one
    /// brought into view.
    pub tree_area: Option<mg_core::ResRef>,
    /// The one object selected in the area in front, as the module tree
    /// shows it (its row marked): its area, kind and place in its list;
    /// and whether the tree has still to go to it.
    pub tree_object: Option<(mg_core::ResRef, mg_area::ObjectKind, usize)>,
    pub tree_object_pending: bool,
    /// The module tree's row the arrow keys are at (a click puts them
    /// there), whether it is to be brought into view, and a group to open
    /// or close at the next frame.
    pub tree_cursor: Option<tree::TreeAt>,
    pub(crate) tree_cursor_moved: bool,
    pub(crate) tree_fold: Option<(&'static str, bool)>,
    pub import: Option<ImportDraft>,
    /// An action waiting for the answer to "save changes?".
    pub confirm_discard: Option<Action>,
    /// The graphics adapter and its limits, for the debug log.
    render_info: Option<String>,
    /// A nasher project's files changed outside Moonglow.
    pub(crate) outside: outside::OutsideState,
    /// The windows maximized, and where each was before (and its panes'
    /// corner then).
    pub(crate) maximized: HashMap<Tab, (egui::Rect, egui::Pos2)>,
    /// The tabs of windows drawn this frame: each one's button and the
    /// layer its window is on ([`Moonglow::window_bars`]).
    pub(crate) tab_buttons: Vec<(Tab, egui::Rect, egui::LayerId)>,
    /// The menus by the keyboard: where the keys are, the rows counted as
    /// they are drawn (a menu's, its submenu's), the menu open, and
    /// whether Alt was used with something else since it went down.
    pub(crate) menu_keys: commands::MenuKeys,
    pub(crate) menu_row: [usize; 2],
    pub(crate) menu_open: Option<usize>,
    pub(crate) menu_alt_spoiled: bool,
    /// Where each open tool window was seen last (see [`Place`]).
    tab_places: HashMap<Tab, Place>,
    /// Each pane's tabs in their order, as this frame began (the dock is
    /// out of reach while its tabs are drawn).
    pub(crate) panes_tabs: Vec<Vec<Tab>>,
    /// The tabs closed, the last one last: Reopen Closed Tab's.
    pub(crate) closed_tabs: Vec<Tab>,
    /// Where each window (by its first tab) is.
    windows: HashMap<Tab, WindowTrack>,
    /// Where the panes are drawn (under the toolbar, beside the tree).
    dock_rect: Option<egui::Rect>,
    /// A resource waiting for the answer to "delete it?".
    pub confirm_delete: Option<ResKey>,
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
    /// The palettes' pane's share of the middle's width, as last drawn.
    palette_share: Option<f32>,
    /// A module was opened: the Palettes pane opens once the module tree
    /// shows (and has a width to match). Tests of other windows turn it
    /// off.
    pub open_palette: bool,
    /// The plugins installed, and what they are doing (`plugins`).
    pub plugins: plugins::Plugins,
    /// Where plugins are installed (the application gives Moonglow's data
    /// folder's `plugins`).
    pub plugin_dir: Option<PathBuf>,
    /// Started without plugins (`--no-plugins`): none are read.
    pub no_plugins: bool,
    /// The keys are to be worked out again (the plugins changed).
    keymap_stale: bool,
    /// The Command Palette, while it is open.
    pub command_palette: Option<commands::Finder>,
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

/// The smallest window whose size is remembered for the next of its kind.
const MIN_REMEMBERED: egui::Vec2 = egui::vec2(260.0, 160.0);

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
            hak_list: Default::default(),
            suggested_hak: None,
            attach: None,
            publish: None,
            tilesets: Vec::new(),
            next_tileset: 0,
            tileset_closing: None,
            model_views: HashMap::new(),
            palette_categories: None,
            area_names: Default::default(),
            area_contents: Default::default(),
            area_views: HashMap::new(),
            adjust: None,
            find_instance: None,
            area_focus: None,
            references: Default::default(),
            text_typed: None,
            script_stamps: Default::default(),
            compiled_dir: None,
            rename: None,
            copy_as: None,
            placeable_gallery: None,
            script_nav: Default::default(),
            prefab_save: None,
            update_draft: None,
            export_dir: None,
            model_export: None,
            open_models: Vec::new(),
            scaled: false,
            area_chooser: None,
            text_replace: None,
            prefab_dir: None,
            prefab_deleted: None,
            var_set_dir: None,
            reload_checked: None,
            tlk_stamp: None,
            object_clip: None,
            item_clip: Vec::new(),
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
            area_tool_at: 0,
            armor_on_woman: false,
            tree_reveal: None,
            tree_area: None,
            tree_object: None,
            tree_object_pending: false,
            tree_cursor: None,
            tree_cursor_moved: false,
            tree_fold: None,
            import: None,
            confirm_discard: None,
            render_info: None,
            outside: Default::default(),
            maximized: HashMap::new(),
            tab_buttons: Vec::new(),
            menu_keys: Default::default(),
            menu_row: [0, 0],
            menu_open: None,
            menu_alt_spoiled: false,
            tab_places: HashMap::new(),
            panes_tabs: Vec::new(),
            closed_tabs: Vec::new(),
            windows: HashMap::new(),
            dock_rect: None,
            confirm_delete: None,
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
            palette_share: None,
            open_palette: false,
            plugins: Default::default(),
            plugin_dir: None,
            no_plugins: false,
            keymap_stale: false,
            command_palette: None,
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

    /// Gives the app the window's GPU, for 3D views.
    pub fn set_render_state(&mut self, render_state: egui_wgpu::RenderState) {
        self.render_info = Some(format!(
            "{:?}; limits: texture {} px, bind groups {}; BC textures {}",
            render_state.adapter.get_info(),
            render_state.device.limits().max_texture_dimension_2d,
            render_state.device.limits().max_bind_groups,
            render_state.device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        ));
        self.viewport = Some(model_view::Viewport3d::new(render_state));
    }

    /// The application with saved settings: their game install (or the
    /// detected one) and recent modules.
    pub fn with_settings(settings: Settings, dialogs: Box<dyn Dialogs>) -> Moonglow {
        let mut app = Moonglow::new(settings.install(), dialogs);
        set_edit_language(mg_core::Language(settings.edit_language.unwrap_or(0)));
        // (From the start, so that it has a module opened at once.)
        trace::set(trace::wanted(!settings.no_debug_log));
        app.settings = settings;
        app
    }

    /// Draws the whole application.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // (The game's text is read by the open module's table, if its haks
        // have one: `encoding.2da`.)
        text::set_game_codepage(self.game.as_deref().map(GameData::codepage));
        if let Some(talk) = &mut self.talk {
            talk.game = self.game.as_deref().map(GameData::codepage);
        }
        self.trace_frame(ui);
        self.thumbnails.begin_frame(self.ws.as_ref().map(|ws| ws.revision()));
        widgets::install_fonts(ui.ctx());
        widgets::dialogs_begin(ui.ctx());
        // Models that broke what builds or poses them, last frame: left out
        // of what is drawn, and said so.
        for failure in mg_render::guard::take_failures() {
            self.log.error(format!(
                "A model could not be shown and is left out ({failure}). The module is unharmed; \
                 a report was written for the report of the problem (Help › User Manual, \
                 Troubleshooting)"
            ));
        }
        // Options › General › Light theme (dark unless chosen).
        let theme = if self.settings.light_theme { egui::Theme::Light } else { egui::Theme::Dark };
        if ui.ctx().theme() != theme {
            ui.ctx().set_theme(theme);
        }
        // Scroll bars that are always there, with room of their own (egui's
        // float over the content and show only under the pointer, so a
        // list gave no sign that there was more of it).
        let bars = egui::style::ScrollStyle::solid();
        if ui.ctx().global_style().spacing.scroll != bars {
            ui.ctx().all_styles_mut(|s| s.spacing.scroll = bars);
        }
        // Options › General › Interface size (when chosen; else the size is
        // egui's own, which Ctrl with + and - change).
        match self.settings.ui_scale {
            Some(scale) => {
                let zoom = f32::from(scale) / 100.0;
                if (ui.ctx().zoom_factor() - zoom).abs() > 0.001 {
                    ui.ctx().set_zoom_factor(zoom);
                }
                self.scaled = true;
            }
            // Set back to the usual size: once.
            None if std::mem::take(&mut self.scaled) => ui.ctx().set_zoom_factor(1.0),
            None => {}
        }
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
            commands::menu_bar(self, ui);
            // Buttons for the common commands, below the menu as in Aurora.
            commands::toolbar(self, ui);
        });
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        // The log shrinks to a single line if the user wants it that small.
        let line = ui.text_style_height(&egui::TextStyle::Monospace);
        // The side panes leave the middle its room, however they were left
        // (their sizes are remembered between runs) and whatever is in
        // them: the log at most half the window's height, the module tree
        // at most two fifths of its width.
        let window = ui.ctx().content_rect().size();
        // A pane folded away (View) leaves a strip at its edge, to bring
        // it back with a click.
        let key = |app: &Moonglow, id: commands::Id, ctx: &egui::Context| {
            let key = app.keymap.label_of(&id.id(), ctx);
            if key.is_empty() { String::new() } else { format!(" ({key})") }
        };
        if self.settings.hide_log {
            egui::Panel::bottom("log-strip").exact_size(line + 6.0).show(ui, |ui| {
                let tip = format!("Show the log{}", key(self, commands::Id::ViewLog, ui.ctx()));
                if ui.small_button("⏶ Log").on_hover_text(tip).clicked() {
                    self.show_panel(commands::Id::ViewLog, true);
                }
            });
        } else {
            egui::Panel::bottom("log")
                .resizable(true)
                .default_size(120.0)
                .size_range((line + 6.0)..=(window.y * 0.5).max(line + 6.0))
                .show(ui, |ui| self.log_ui(ui));
        }
        if self.ws.is_some() {
            if self.settings.hide_tree {
                egui::Panel::left("tree-strip").exact_size(22.0).show(ui, |ui| {
                    let id = commands::Id::ViewTree;
                    let tip = format!("Show the module tree{}", key(self, id, ui.ctx()));
                    if ui.small_button("⏵").on_hover_text(tip).clicked() {
                        self.show_panel(id, true);
                    }
                });
                // (As wide as it was, for where the palettes' pane goes.)
            } else {
                let tree = egui::Panel::left("tree")
                    .resizable(true)
                    .default_size(240.0)
                    .size_range(150.0..=(window.x * 0.4).max(150.0))
                    .show(ui, |ui| tree::module_tree(self, ui));
                self.tree_width = Some(tree.response.rect.width());
            }
            // The palette beside a newly opened module, as in Aurora (not
            // when it was folded away).
            if std::mem::take(&mut self.open_palette)
                && self.game.is_some()
                && !self.settings.hide_palettes
            {
                self.actions.push(Action::OpenTab(Tab::Palette));
            }
            if self.settings.hide_palettes
                && self.game.is_some()
                && self.dock.find_tab(&Tab::Palette).is_none()
            {
                egui::Panel::right("palettes-strip").exact_size(22.0).show(ui, |ui| {
                    let id = commands::Id::ViewPalettes;
                    let tip = format!("Show the palettes{}", key(self, id, ui.ctx()));
                    if ui.small_button("⏴").on_hover_text(tip).clicked() {
                        self.show_panel(id, true);
                    }
                });
            }
        }
        self.heard = None;
        // The options' draft shows in its tab.
        if self.options.is_some() && self.dock.find_tab(&Tab::Options).is_none() {
            self.run_now(Action::OpenTab(Tab::Options));
        }
        // And Tile Properties in its own; without tiles, the tab goes.
        match (self.tile_props.is_some(), self.dock.find_tab(&Tab::TileProperties).is_some()) {
            (true, false) => self.run_now(Action::OpenTab(Tab::TileProperties)),
            (false, true) => self.run_now(Action::CloseTab(Tab::TileProperties)),
            _ => {}
        }
        self.keep_middle();
        self.tab_number_keys(ui.ctx());
        egui::CentralPanel::default().show(ui, |ui| {
            self.dock_width = Some(ui.available_width());
            self.dock_rect = Some(ui.max_rect());
            // No pane (the palettes', one docked beside another) is
            // dragged to nothing, or over all of its neighbour.
            for (_, node) in self.dock.iter_all_nodes_mut() {
                if let egui_dock::Node::Vertical(split) | egui_dock::Node::Horizontal(split) = node
                {
                    split.fraction = match split.fraction {
                        f if f.is_finite() => f.clamp(0.12, 0.88),
                        _ => 0.5,
                    };
                }
            }
            // (The dock is out of reach while its tabs are drawn: the model
            // windows open, for a page that shows a model unless one is.)
            // (Set again by the talk table's editor, if it is drawn.)
            self.talk_view.hovered = false;
            self.open_models = self
                .dock
                .iter_all_tabs()
                .map(|(_, t)| t)
                .filter(|t| matches!(t, Tab::Model(_) | Tab::InstanceModel { .. }))
                .cloned()
                .collect();
            self.panes_tabs = self.dock.iter_leaves().map(|(_, l)| l.tabs.clone()).collect();
            let mut dock = std::mem::replace(&mut self.dock, DockState::new(Vec::new()));
            // A window's frame without a margin: egui_dock makes a collapsed
            // window as tall as its tab bar, margin included, which left
            // half the bar (and half the arrow that opens it again).
            ui.spacing_mut().window_margin = egui::Margin::ZERO;
            {
                let mut viewer = tabs::Viewer { app: self };
                DockArea::new(&mut dock).show_close_buttons(true).show_inside(ui, &mut viewer);
            }
            self.dock = dock;
            self.window_bars(ui);
        });
        trace::changed("panes", || self.panes());
        self.remember_window_sizes();
        self.remember_window_places();
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
        transfer::ui(self, ui);
        widgets::ui(self, ui);
        script_view::windows(self, ui);
        dialog_view::windows(self, ui);
        dialog_view::test_window(self, ui);
        script_wizard::window(self, ui);
        area_tools::windows(self, ui);
        area_tools::preview_window(self, ui);
        area_reshape::windows(self, ui.ctx());
        area_view::stats_window(self, ui.ctx());
        build_view::window(self, ui.ctx());
        store_wizard::window(self, ui.ctx());
        store_wizard::popup_window(self, ui.ctx());
        references::rename_window(self, ui.ctx());
        references::delete_window(self, ui.ctx());
        copy_as::window(self, ui.ctx());
        outside::window(self, ui.ctx());
        script_nav::rename_window(self, ui.ctx());
        prefabs::save_window(self, ui.ctx());
        palette_categories::window(self, ui.ctx());
        bulk::update_window(self, ui.ctx());
        area_props::chooser_window(self, ui.ctx());
        hak_view::closing_window(self, ui.ctx());
        hak_view::adding_window(self, ui.ctx());
        model_view::export_window(self, ui.ctx());
        module_props::attach_window(self, ui.ctx());
        nwsync_view::window(self, ui.ctx());
        tileset_view::closing_window(self, ui.ctx());
        bulk::text_window(self, ui.ctx());
        levelup_view::window(self, ui.ctx());
        creature_wizard::window(self, ui.ctx());
        if let Some(report) = &self.hak_report {
            let mut open = true;
            egui::Window::new("Hak Pak Conflict Analysis")
                .pivot(egui::Align2::CENTER_CENTER)
                .default_pos(ui.ctx().content_rect().center())
                .open(crate::widgets::open_unless_escape(
                    ui.ctx(),
                    "Hak Pak Conflict Analysis",
                    &mut open,
                ))
                .default_size([600.0, 400.0])
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::both().show(ui, |ui| ui.monospace(report));
                });
            if !open {
                self.hak_report = None;
            }
        }
        plugins::window(self, ui.ctx());
        plugins::replace_window(self, ui.ctx());
        plugins::remove_window(self, ui.ctx());
        commands::palette_window(self, ui.ctx());
        jobs::window(self, ui.ctx());
        // The pictures the frame's views asked for and lack: a few now.
        model_view::make_thumbnails(self, ui.ctx());
        if !self.actions.is_empty() {
            self.run_actions();
            // Show the result now, not at the next input event.
            ui.ctx().request_repaint();
        }
    }

    /// The debug log's start of a frame: once, the settings and the
    /// graphics adapter; then the window and what is under way, as they
    /// change.
    fn trace_frame(&mut self, ui: &egui::Ui) {
        trace::set(trace::wanted(!self.settings.no_debug_log));
        if !trace::on() {
            return;
        }
        trace::changed("graphics", || self.render_info.clone().unwrap_or_else(|| "none".into()));
        trace::changed("settings", || {
            let s = &self.settings;
            format!(
                "game {:?}, user folder {:?}, last area {}, areas by name {}, scale {:?}, \
                 still objects {}, unlit {}, plugins {:?}; install {:?}",
                s.game_root,
                s.user_dir,
                !s.no_last_area,
                s.area_names,
                s.ui_scale,
                s.still_objects,
                s.unlit_areas,
                s.plugins_enabled,
                self.install.as_ref().map(|i| i.root.clone()),
            )
        });
        trace::changed("window", || {
            format!(
                "{:?}, {} points per pixel, zoom {}; tree {:?} wide, panes {:?} wide; job {:?}; \
                 module {:?}; game data {}; 3D viewport {}",
                ui.ctx().content_rect(),
                ui.ctx().pixels_per_point(),
                ui.ctx().zoom_factor(),
                self.tree_width,
                self.dock_width,
                self.job.as_ref().map(|j| j.title.clone()),
                self.module_path(),
                self.game.is_some(),
                self.viewport.is_some(),
            )
        });
    }

    /// The panes and their tabs, for the debug log: each pane's surface
    /// and node, where it is on screen, its tabs and the one shown.
    fn panes(&self) -> String {
        let mut out = Vec::new();
        for (path, leaf) in self.dock.iter_leaves() {
            let tabs: Vec<String> = leaf.tabs.iter().map(|t| format!("{t:?}")).collect();
            out.push(format!(
                "[surface {} node {} at {:?} shows #{}: {}]",
                path.surface.0,
                path.node.0,
                leaf.rect,
                leaf.active.0,
                tabs.join(", ")
            ));
        }
        if out.is_empty() { "none".into() } else { out.join(" ") }
    }

    /// Where the main panes split for the Palettes pane on the right: the
    /// left side's share, leaving the palette as wide as the module tree.
    /// (egui_dock's split fraction is always the left side's.)
    /// Where a tool window is: a window of its own, a tab of the main
    /// pane, or a tab of the palettes' pane.
    fn place_of(&self, tab: &Tab) -> Option<Place> {
        let path = self.dock.find_tab(tab)?;
        if !path.surface.is_main() {
            return Some(Place::Window);
        }
        let leaf = self.dock.leaf(egui_dock::NodePath { surface: path.surface, node: path.node });
        Some(match leaf {
            Ok(leaf) if leaf.tabs.contains(&Tab::Palette) => Place::Palettes,
            _ => Place::Middle,
        })
    }

    /// Where the next window of a kind opens: where one of its kind was
    /// last dragged to (GitHub issue 14), a window of its own otherwise.
    fn place_for(&self, tab: &Tab) -> Place {
        let kept = self.settings.window_places.iter().find(|(k, _)| k == tab.kind());
        match kept.map(|(_, p)| p.as_str()) {
            Some("middle") => Place::Middle,
            Some("palettes") => Place::Palettes,
            _ => Place::Window,
        }
    }

    /// Notes where the user dragged a tool window to (a window seen in
    /// another place than the frame before), for the next of its kind.
    fn remember_window_places(&mut self) {
        let open: Vec<Tab> = (self.dock.iter_all_tabs().map(|(_, t)| t))
            .filter(|t| !t.docks() && **t != Tab::Palette)
            .cloned()
            .collect();
        self.tab_places.retain(|t, _| open.contains(t));
        // (Not while one is dragged, or made to fill the pane.)
        for tab in open {
            let Some(now) = self.place_of(&tab) else { continue };
            let before = self.tab_places.insert(tab.clone(), now);
            if before.is_some_and(|b| b != now) && !self.maximized.contains_key(&tab) {
                let kind = tab.kind();
                self.settings.window_places.retain(|(k, _)| k != kind);
                let name = match now {
                    Place::Middle => "middle",
                    Place::Palettes => "palettes",
                    Place::Window => continue,
                };
                self.settings.window_places.push((kind.to_string(), name.to_string()));
            }
        }
    }

    /// Whether a pane beside the middle is shown (View): the module tree,
    /// the palettes, the log.
    pub(crate) fn panel_shown(&self, panel: commands::Id) -> bool {
        match panel {
            commands::Id::ViewTree => !self.settings.hide_tree,
            commands::Id::ViewLog => !self.settings.hide_log,
            _ => self.dock.find_tab(&Tab::Palette).is_some(),
        }
    }

    /// Whether every one of them is folded away.
    pub(crate) fn panels_hidden(&self) -> bool {
        use commands::Id::{ViewLog, ViewPalettes, ViewTree};
        ![ViewTree, ViewPalettes, ViewLog].into_iter().any(|p| self.panel_shown(p))
    }

    /// Shows a pane beside the middle, or folds it away (it leaves a
    /// strip at its edge, and is as wide when it comes back).
    pub(crate) fn show_panel(&mut self, panel: commands::Id, show: bool) {
        match panel {
            commands::Id::ViewTree => self.settings.hide_tree = !show,
            commands::Id::ViewLog => self.settings.hide_log = !show,
            _ => {
                self.settings.hide_palettes = !show;
                let open = self.dock.find_tab(&Tab::Palette);
                match (show, open) {
                    (true, None) if self.game.is_some() && self.ws.is_some() => {
                        self.actions.push(Action::OpenTab(Tab::Palette));
                    }
                    (false, Some(path)) => {
                        self.dock.remove_tab(path);
                    }
                    _ => {}
                }
            }
        }
    }

    /// View › Reset Layout: the tree, the palettes and the log shown, at
    /// the sizes they have at first; the palettes in their pane on the
    /// right.
    pub(crate) fn reset_layout(&mut self, ctx: &egui::Context) {
        for id in ["tree", "log"] {
            ctx.data_mut(|d| d.remove::<egui::containers::panel::PanelState>(egui::Id::new(id)));
        }
        self.settings.hide_tree = false;
        self.settings.hide_log = false;
        self.settings.window_places.clear();
        self.palette_share = None;
        if let Some(path) = self.dock.find_tab(&Tab::Palette) {
            self.dock.remove_tab(path);
        }
        self.show_panel(commands::Id::ViewPalettes, true);
    }

    /// Whether a tab may close now, letting go of what it held: a hak or a
    /// tileset with unsaved changes asks first, and stays for the answer.
    pub(crate) fn may_close(&mut self, tab: &Tab) -> bool {
        // (Closed, its draft goes: Cancel.)
        if *tab == Tab::Options {
            self.options = None;
        }
        if *tab == Tab::TileProperties {
            self.tile_props = None;
        }
        if let Tab::Hak(id) = *tab {
            if self.haks.iter().any(|d| d.id == id && d.hak.is_dirty()) {
                self.hak_closing = Some(id);
                return false;
            }
            self.haks.retain(|d| d.id != id);
        }
        if let Tab::Tileset(id) = *tab {
            if self.tilesets.iter().any(|d| d.id == id && d.is_dirty()) {
                self.tileset_closing = Some(id);
                return false;
            }
            self.tilesets.retain(|d| d.id != id);
        }
        true
    }

    /// Notes a tab closed, for Reopen Closed Tab: those that show a
    /// resource or a tool of the module, not a hak's or a tileset's own
    /// editor (closed, it has let its file go).
    pub(crate) fn closed(&mut self, tab: &Tab) {
        if matches!(tab, Tab::Hak(_) | Tab::Tileset(_) | Tab::Options | Tab::TileProperties) {
            return;
        }
        // (The palettes' pane closed is the palettes folded away: View
        // and the strip at the edge bring them back.)
        if *tab == Tab::Palette {
            self.settings.hide_palettes = true;
            return;
        }
        self.closed_tabs.retain(|t| t != tab);
        self.closed_tabs.push(tab.clone());
        let extra = self.closed_tabs.len().saturating_sub(20);
        self.closed_tabs.drain(..extra);
    }

    /// Reopen Closed Tab: the tab closed last that can still be shown (its
    /// resource is in the module still).
    pub(crate) fn reopen_tab(&mut self) {
        while let Some(tab) = self.closed_tabs.pop() {
            let there = match tab.renamable() {
                Some(key) => self.ws.as_ref().is_some_and(|ws| ws.module.contains(&key)),
                None => true,
            };
            if there && self.dock.find_tab(&tab).is_none() {
                self.actions.push(Action::OpenTab(tab));
                return;
            }
        }
    }

    /// The pane the tab keys work in: the one with the focus, if it has
    /// tabs to go between, else the middle's (the areas').
    fn tab_pane(&self) -> Option<egui_dock::NodePath> {
        let tabs = |p: &egui_dock::NodePath| self.dock.leaf(*p).map_or(0, |l| l.tabs.len());
        let focused = self.dock.focused_leaf().filter(|p| tabs(p) > 1);
        let palette = self.dock.find_tab(&Tab::Palette).map(|p| (p.surface, p.node));
        focused.or_else(|| {
            (self.dock.iter_leaves().map(|(p, _)| p))
                .find(|p| p.surface.is_main() && Some((p.surface, p.node)) != palette)
        })
    }

    /// The tab Close Tab closes: the one in front of the pane with the
    /// focus (a window's, or the middle's), if it can be closed.
    pub(crate) fn front_tab(&self) -> Option<Tab> {
        let pane = self.dock.focused_leaf().or_else(|| self.tab_pane())?;
        let leaf = self.dock.leaf(pane).ok()?;
        leaf.tabs.get(leaf.active.0).filter(|t| t.closeable()).cloned()
    }

    /// Next Tab and Previous Tab: the pane's next tab brought to the
    /// front, round from the last to the first.
    pub(crate) fn step_tab(&mut self, by: isize) {
        let Some(pane) = self.tab_pane() else { return };
        let Ok(leaf) = self.dock.leaf(pane) else { return };
        let n = leaf.tabs.len() as isize;
        if n < 2 {
            return;
        }
        let next = (leaf.active.0 as isize + by).rem_euclid(n) as usize;
        self.show_tab(leaf.tabs[next].clone());
    }

    /// The middle's tab of a number (from 1; 9: its last), brought to the
    /// front.
    fn number_tab(&mut self, number: usize) {
        let palette = self.dock.find_tab(&Tab::Palette).map(|p| (p.surface, p.node));
        let Some((_, leaf)) = (self.dock.iter_leaves())
            .find(|(p, _)| p.surface.is_main() && Some((p.surface, p.node)) != palette)
        else {
            return;
        };
        let tab = if number == 9 { leaf.tabs.last() } else { leaf.tabs.get(number - 1) };
        if let Some(tab) = tab.cloned() {
            self.show_tab(tab);
        }
    }

    /// Brings a tab that is open to the front of its pane, as a click on
    /// it would.
    fn show_tab(&mut self, tab: Tab) {
        if let Some(path) = self.dock.find_tab(&tab) {
            let _ = self.dock.set_active_tab(path);
            self.dock.set_focused_node_and_surface(egui_dock::NodePath {
                surface: path.surface,
                node: path.node,
            });
            if let Tab::Area(area) = tab {
                self.tree_reveal = Some((area, false));
            }
        }
    }

    /// Ctrl and a digit: the middle's tab of that number (9: the last), as
    /// in a browser. Not while text is typed in (the script editor's
    /// numbered bookmarks are Ctrl and a digit there).
    fn tab_number_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        use egui::Key::{Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9};
        let keys = [Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9];
        let pressed = ctx
            .input_mut(|i| keys.iter().position(|k| i.consume_key(egui::Modifiers::COMMAND, *k)));
        if let Some(i) = pressed {
            self.number_tab(i + 1);
        }
    }

    /// With a module open, the middle of the window keeps its place when
    /// its last tab is closed: a pane that says no area is open takes it,
    /// and goes when something opens there. (Without it the palettes'
    /// pane, the only one left, took the whole middle.)
    fn keep_middle(&mut self) {
        if self.ws.is_none() {
            return;
        }
        // The middle's tabs: those of the main surface that are not in
        // the palettes' pane. (A tab docked beside the palettes there, the
        // resource browser's, is the side pane's, not the middle's: with
        // it open the middle was let go all the same. GitHub issue 11.)
        let side = (self.dock.find_tab(&Tab::Palette))
            .filter(|p| p.surface.is_main())
            .map(|p| (p.surface, p.node));
        let main: Vec<Tab> = (self.dock.iter_all_tabs())
            .filter(|(p, _)| p.surface.is_main() && Some((p.surface, p.node)) != side)
            .map(|(_, t)| t.clone())
            .collect();
        let others = main.iter().any(|t| !matches!(t, Tab::Palette | Tab::NoArea));
        let placeholder = main.contains(&Tab::NoArea);
        // (Where the palettes' pane begins, kept for when the pane beside
        // it is made anew.)
        if let Some(palette) = self.dock.find_tab(&Tab::Palette).filter(|p| p.surface.is_main())
            && let Some(parent) = palette.node.parent()
            && let egui_dock::Node::Horizontal(split) = &self.dock.main_surface()[parent]
            && palette.node == parent.right()
        {
            self.palette_share = Some(1.0 - split.fraction);
        }
        if others && placeholder {
            if let Some(path) = self.dock.find_tab(&Tab::NoArea) {
                self.dock.remove_tab(path);
            }
        } else if !others && !placeholder {
            match self.dock.find_tab(&Tab::Palette).filter(|p| p.surface.is_main()) {
                Some(palette) => {
                    let left = match self.palette_share {
                        Some(share) if share < 0.95 => (1.0 - share).clamp(0.12, 0.88),
                        _ => self.palette_split(),
                    };
                    self.dock.main_surface_mut().split_left(palette.node, left, vec![Tab::NoArea]);
                }
                None => self.dock.main_surface_mut().push_to_first_leaf(Tab::NoArea),
            }
        }
    }

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
            // (The pane that says no area is open is as no pane: windows
            // open where they did with the middle empty.)
            .find(|(p, leaf)| {
                p.surface.is_main()
                    && Some((p.surface, p.node)) != palette
                    && !leaf.tabs.contains(&Tab::NoArea)
            })
            .map(|(_, leaf)| leaf.rect)
            .filter(|r| r.is_positive())
            .unwrap_or(screen);
        // Below the menu and toolbar (the main pane's top), never over them.
        let room = egui::Rect::from_min_max(egui::pos2(screen.left(), main.top()), screen.max);
        // As wide and tall as one of its kind was last left.
        let kind = tab.kind();
        let left = self.settings.window_sizes.iter().find(|(k, _)| k == kind);
        // (One left too small to use, by an older version, is passed over.)
        let left = left
            .filter(|(_, s)| s[0] as f32 >= MIN_REMEMBERED.x && s[1] as f32 >= MIN_REMEMBERED.y);
        let want = left.map_or(tab.window_size(), |(_, s)| egui::vec2(s[0] as f32, s[1] as f32));
        let remembered = left.is_some();
        // Some of the main pane (the area view) shows beside it.
        // (The Options need their width, and aren't worked in beside it.)
        let wide = if tab == Tab::Options {
            room.width()
        } else {
            (main.width() * 0.85).max(420.0).min(room.width())
        };
        let size = if remembered {
            // (As it was left, if the screen still has the room.)
            egui::vec2(want.x.min(room.width()), want.y.min(room.height()))
        } else {
            egui::vec2(want.x.min(wide), want.y.min(room.height() * 0.9))
        };
        let windows = self
            .dock
            .iter_surfaces()
            .filter(|s| matches!(s, egui_dock::Surface::Window(..)))
            .count();
        let step = 28.0 * (windows % 8) as f32;
        let mut at = main.center() - size / 2.0 + egui::vec2(step, step);
        at.x = at.x.min(room.right() - size.x).max(room.left());
        at.y = at.y.min(room.bottom() - size.y).max(room.top());
        self.dock.add_window(vec![tab.clone()]);
        self.put_window(&tab, egui::Rect::from_min_size(at, size));
    }

    /// The bar beside a window's tabs does what its tab does: a double
    /// click maximizes the window and restores it, and a right click
    /// offers that. (egui_dock gives the bar nothing; a builder looked for
    /// both there, as on any window's title.) The window is still dragged
    /// by it.
    fn window_bars(&mut self, ui: &mut egui::Ui) {
        let buttons = std::mem::take(&mut self.tab_buttons);
        let mut toggle = None;
        for (path, leaf) in self.dock.iter_leaves() {
            let Some(tab) = leaf.tabs.get(leaf.active.0).or(leaf.tabs.first()) else { continue };
            if path.surface.is_main() || tab.docks() || *tab == Tab::Palette {
                continue;
            }
            let own: Vec<_> = buttons.iter().filter(|(t, ..)| leaf.tabs.contains(t)).collect();
            let Some(layer) = own.first().map(|b| b.2) else { continue };
            // From the last tab to the window's close button.
            let left = own.iter().map(|b| b.1.right()).fold(leaf.rect.left(), f32::max) + 2.0;
            let bar = egui::Rect::from_min_max(
                egui::pos2(left, leaf.rect.top()),
                egui::pos2(leaf.rect.right() - 28.0, leaf.viewport.top()),
            );
            if !bar.is_positive() || leaf.collapsed {
                continue;
            }
            let id = egui::Id::new(("window-bar", path.surface.0, path.node.0));
            let builder = egui::UiBuilder::new().layer_id(layer).max_rect(bar);
            let response =
                ui.scope_builder(builder, |ui| ui.interact(bar, id, egui::Sense::click()));
            let response = response.inner;
            if response.double_clicked() {
                toggle = Some(tab.clone());
            }
            let label = if self.maximized.contains_key(tab) { "Restore" } else { "Maximize" };
            response.context_menu(|ui| {
                if ui.button(label).clicked() {
                    toggle = Some(tab.clone());
                    ui.close();
                }
            });
        }
        if let Some(tab) = toggle {
            self.actions.push(Action::ToggleMaximize(tab));
        }
    }

    /// Where each window is: its first tab, and the room its panes take.
    /// (egui_dock doesn't say where a window is; its panes do.)
    fn window_panes(&self) -> Vec<(Tab, egui::Rect)> {
        let mut out: Vec<(egui_dock::SurfaceIndex, Tab, egui::Rect)> = Vec::new();
        for (path, leaf) in self.dock.iter_leaves() {
            let Some(tab) = leaf.tabs.first() else { continue };
            if path.surface.is_main() || !leaf.rect.is_positive() || !leaf.rect.is_finite() {
                continue;
            }
            match out.iter_mut().find(|(s, ..)| *s == path.surface) {
                Some((_, _, rect)) => *rect = rect.union(leaf.rect),
                None => out.push((path.surface, tab.clone(), leaf.rect)),
            }
        }
        out.into_iter().map(|(_, tab, rect)| (tab, rect)).collect()
    }

    /// Each frame: where each window is now, from where it was put and
    /// how its panes have moved and grown since; and its size, by the kind
    /// of its first tab, for the next of the kind to open at (not a
    /// maximized one's).
    fn remember_window_sizes(&mut self) {
        let mut again = Vec::new();
        // The windows with a pane folded (the arrow at their bar's left).
        let folded: Vec<Tab> = self
            .dock
            .iter_leaves()
            .filter(|(p, leaf)| !p.surface.is_main() && leaf.collapsed)
            .flat_map(|(_, leaf)| leaf.tabs.iter().cloned())
            .collect();
        for (tab, panes) in self.window_panes() {
            let Some(track) = self.windows.get_mut(&tab) else { continue };
            // (Once it has settled there: a window is a frame or two in
            // getting where it was put.)
            let settled = track.seen.replace(panes) == Some(panes);
            let first = match track.first {
                Some(first) => first,
                None if settled => {
                    // Put back where it was, it may land beside it (held
                    // inside the screen at the size it still had): put
                    // there again, once, now that it is small.
                    if let Some(aim) = track.aim.take()
                        && (aim - panes.min).length() > 0.5
                    {
                        again.push((tab.clone(), track.put));
                        continue;
                    }
                    *track.first.insert(panes)
                }
                None => continue,
            };
            track.now = egui::Rect::from_min_size(
                track.put.min + (panes.min - first.min),
                track.put.size() + (panes.size() - first.size()),
            );
            if self.maximized.contains_key(&tab) || panes == first {
                continue;
            }
            // (Not a window folded to its bar, nor one squeezed to nothing:
            // the next of its kind opened as small, as if it had folded.)
            let small =
                track.now.width() < MIN_REMEMBERED.x || track.now.height() < MIN_REMEMBERED.y;
            if folded.contains(&tab) || small {
                continue;
            }
            let size = [track.now.width().round() as u32, track.now.height().round() as u32];
            let kind = tab.kind();
            match self.settings.window_sizes.iter_mut().find(|(k, _)| k == kind) {
                Some((_, s)) => *s = size,
                None => self.settings.window_sizes.push((kind.to_string(), size)),
            }
        }
        for (tab, rect) in again {
            self.put_window(&tab, rect);
        }
        // (Closed windows are forgotten.)
        let dock = &self.dock;
        self.maximized.retain(|t, _| dock.find_tab(t).is_some());
        self.windows.retain(|t, _| dock.find_tab(t).is_some());
    }

    /// Puts a tab's window at `rect` (position and size), and follows it
    /// from there.
    fn put_window(&mut self, tab: &Tab, rect: egui::Rect) {
        let Some(path) = self.dock.find_tab(tab) else { return };
        let Some(state) = self.dock.get_window_state_mut(path.surface) else { return };
        state.set_position(rect.min).set_size(rect.size());
        trace::note(format!("  window of {tab:?} put at {rect:?}"));
        self.windows.insert(
            tab.clone(),
            WindowTrack { put: rect, seen: None, first: None, now: rect, aim: None },
        );
    }

    /// The room a maximized window takes: the whole of Moonglow's window
    /// under its menu and toolbar, over the module tree, the palettes and
    /// the log as well as the panes (a builder: Maximize "doesn't make it
    /// fill the screen").
    fn maximize_room(&self) -> Option<egui::Rect> {
        match (self.dock_rect, self.screen) {
            (Some(dock), Some(screen)) => {
                Some(egui::Rect::from_min_max(egui::pos2(screen.left(), dock.top()), screen.max))
            }
            (dock, screen) => dock.or(screen),
        }
    }

    /// [`Action::ToggleMaximize`]: the tab's window over the whole of
    /// [`Moonglow::maximize_room`], or back where it was.
    fn toggle_maximize(&mut self, tab: &Tab) {
        // (The window is followed by its first tab.)
        let Some(path) = self.dock.find_tab(tab) else { return };
        if path.surface.is_main() {
            return;
        }
        let first = self
            .dock
            .iter_leaves()
            .find(|(p, _)| p.surface == path.surface)
            .and_then(|(_, leaf)| leaf.tabs.first().cloned());
        let Some(tab) = first else { return };
        if let Some((was, corner)) = self.maximized.remove(&tab) {
            self.put_window(&tab, was);
            if let Some(track) = self.windows.get_mut(&tab) {
                track.aim = Some(corner);
            }
        } else if let (Some(room), Some(track)) = (self.maximize_room(), self.windows.get(&tab)) {
            let corner = track.seen.map_or(track.now.min, |r| r.min);
            self.maximized.insert(tab.clone(), (track.now, corner));
            self.put_window(&tab, room);
        }
    }

    fn shortcuts(&mut self, ui: &mut egui::Ui) {
        // The keys as Options › Keyboard last set them.
        if std::mem::take(&mut self.keymap_stale) || self.keymap_from != self.settings.key_bindings
        {
            self.keymap = keys::Keymap::new(&self.settings.key_bindings);
            let commands = self.plugin_commands();
            plugins::register_keys(&mut self.keymap, &commands);
            self.keymap_from = self.settings.key_bindings.clone();
        }
        // Options › Keyboard takes the next key itself, and the Command
        // Palette what is typed into it.
        if self.options.as_ref().is_some_and(|o| o.recording.is_some())
            || self.command_palette.is_some()
        {
            return;
        }
        commands::keys_pressed(self, ui);
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
                    ui.label(egui::RichText::new(msg).color(level.color(ui)).monospace());
                }
            });
        // Its own button to fold it away, in its corner (GitHub issue 10:
        // each pane has one; the strip it leaves brings it back).
        let corner = ui.max_rect().right_top() + egui::vec2(-36.0, 2.0);
        let at = egui::Rect::from_min_size(corner, egui::vec2(20.0, 18.0));
        if ui.put(at, egui::Button::new("–").small()).on_hover_text("Fold the log away").clicked()
        {
            self.show_panel(commands::Id::ViewLog, false);
        }
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
                let path = remembered.as_deref().unwrap_or(path);
                self.settings.remember(path);
                // The area opened last in it, if it still has it; else the
                // first the module tree lists (remembered from then on).
                let last =
                    self.settings.last_area(path).and_then(|a| a.parse::<mg_core::ResRef>().ok());
                let by_name = self.settings.area_names;
                if let Some(ws) = self.ws.as_ref().filter(|_| !self.settings.no_last_area) {
                    let has = |area: &mg_core::ResRef| {
                        ws.module.contains(&ResKey::new(*area, ResType::ARE))
                    };
                    let first = || {
                        let mut areas =
                            mg_module::areas::list_in(&ws.module, text::game_codepage());
                        if by_name {
                            areas.sort_by_cached_key(|a| {
                                let named = !a.name.trim().is_empty();
                                let shown =
                                    if named { a.name.clone() } else { a.resref.to_string() };
                                (shown.to_lowercase(), a.resref)
                            });
                        }
                        areas.first().map(|a| a.resref)
                    };
                    if let Some(area) = last.filter(has).or_else(first) {
                        self.actions.push(Action::OpenTab(Tab::Area(area)));
                    }
                }
            }
            Err(e) => self.log.error(format!("Could not open {}: {e}", path.display())),
        }
    }

    /// Makes a module the open one, with its haks and itself layered into
    /// the game data.
    fn use_module(&mut self, m: Module) {
        self.close();
        trace::note(format!(
            "use module: {} resources, haks {:?}; game data {}, shared by {} other(s); install {}",
            m.len(),
            m.haks().unwrap_or_default(),
            if self.game.is_some() { "loaded" } else { "none" },
            self.game.as_ref().map_or(0, |g| Arc::strong_count(g) - 1),
            if self.install.is_some() { "known" } else { "none" },
        ));
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
            text::set_game_codepage(Some(game.codepage()));
            self.haks_layered = haks;
        } else if self.game.is_some() && self.install.is_some() {
            // (Something still reads the game data: said, rather than a
            // module opened without its haks and nothing to show why.)
            self.log.error(
                "The module's haks could not be added to the game data (it is in use): its \
                 custom content will be missing. Close and open the module again.",
            );
        }
        self.ws = Some(Workspace::new(m));
        self.outside = Default::default();
        self.area_names = Default::default();
        self.area_contents.clear();
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

    /// Reads the module's custom talk table again (after it was saved).
    pub(crate) fn reload_custom_tlk(&mut self) {
        self.custom_tlk = None;
        self.load_custom_tlk();
    }

    /// Loads the module's custom talk table (from its haks, the module or
    /// the user's `tlk/` folder, as the game looks) when the module names
    /// another one than is loaded.
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
        text::set_game_codepage(Some(game.codepage()));
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
        // (Another module's blueprints of these names look different.)
        self.thumbnails.forget(self.viewport.as_ref());
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
            text::set_game_codepage(Some(game.codepage()));
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
        text::set_game_codepage(Some(game.codepage()));
        self.load_order_changed();
    }

    fn run_actions(&mut self) {
        let actions = std::mem::take(&mut self.actions);
        for a in actions {
            self.run(a);
        }
        // (A tab of the middle closed just now: its place is kept at once.)
        self.keep_middle();
    }

    /// Runs one action now. Actions that would discard unsaved work ask
    /// first (see [`Moonglow::confirm_discard`]).
    pub fn run(&mut self, action: Action) {
        let discards = match &action {
            // Options discard nothing unless they choose other folders: the
            // module is opened again from its file then.
            Action::ApplyOptions(draft) => self.options_reload(draft),
            other => matches!(
                other,
                Action::NewModuleDialog
                    | Action::OpenModuleDialog
                    | Action::OpenFolderDialog
                    | Action::OpenModule(_)
                    | Action::Close
                    | Action::Quit
            ),
        };
        if discards && self.has_unsaved_work() {
            self.confirm_discard = Some(action);
            return;
        }
        self.run_now(action);
    }

    fn run_now(&mut self, action: Action) {
        trace::note(format!("action {action:?}"));
        if let Action::OpenTab(tab) = &action {
            trace::note(format!(
                "  open tab {tab:?}: docks {}, already open {}, focused pane {:?}",
                tab.docks(),
                self.dock.find_tab(tab).is_some(),
                self.dock.focused_leaf().map(|p| (p.surface.0, p.node.0)),
            ));
        }
        let opened = matches!(action, Action::OpenTab(_));
        self.run_matched(action);
        if opened {
            trace::note(format!("  panes after: {}", self.panes()));
        }
    }

    fn run_matched(&mut self, action: Action) {
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
            Action::ExportFiles { keys, dependencies, scratch } => {
                self.run_export_files(keys, dependencies, scratch);
            }
            Action::ImportDialog => self.open_import(),
            Action::Import(draft) => self.run_import(draft),
            Action::HakReport => self.hak_report(),
            Action::OptionsDialog => {
                let mut draft = OptionsDraft::from_settings(&self.settings);
                // (Looked for once: it reads Steam's library files.)
                draft.detected = GameInstall::detect();
                // The plugins' commands take keys like the rest.
                let commands = self.plugin_commands();
                plugins::register_keys(&mut draft.keymap, &commands);
                draft.plugin_commands = commands
                    .into_iter()
                    .map(|c| (c.key_id, format!("{}: {}", c.plugin_name, c.title)))
                    .collect();
                self.options = Some(draft);
            }
            Action::ApplyOptions(draft) => self.apply_options(*draft),
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
                        .text_in(
                            mg_core::Language::ENGLISH,
                            mg_core::Gender::Male,
                            text::game_codepage(),
                        )?
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
                // (Over the talk table's editor, its own.)
                if talk_view::undo(self, action == Action::Redo) {
                    return;
                }
                // (A Ctrl + wheel scaling under way is what Undo takes
                // back: it is not a command yet.)
                if action == Action::Undo && area_view::drop_wheel_scale(self) {
                    return;
                }
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
            Action::CloseTab(tab) => {
                if tab == Tab::Options {
                    self.options = None;
                }
                if tab == Tab::TileProperties {
                    self.tile_props = None;
                }
                if let Some(path) = self.dock.find_tab(&tab) {
                    self.dock.remove_tab(path);
                }
            }
            Action::CloseTabs(tabs) => {
                for tab in tabs {
                    if tab.closeable() && self.may_close(&tab) {
                        self.closed(&tab);
                        if let Some(path) = self.dock.find_tab(&tab) {
                            self.dock.remove_tab(path);
                        }
                    }
                }
            }
            Action::ToggleMaximize(tab) => self.toggle_maximize(&tab),
            Action::OpenTab(tab) => {
                // The area opened last is opened again with the module.
                if let (Tab::Area(area), Some(module)) = (&tab, self.module_path()) {
                    self.settings.remember_area(&module, &area.to_string());
                }
                // The module tree marks the area opened, as when its tab is
                // clicked: the one marked before is not the one in front.
                if let Tab::Area(area) = &tab {
                    self.tree_reveal = Some((*area, false));
                }
                // A tab docked in an area's pane (Module Properties, docked
                // when the module opened) moves to a window of its own
                // rather than come to the front over the area.
                if !tab.docks()
                    && tab != Tab::Palette
                    && self.place_for(&tab) != Place::Middle
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
                        // (As wide as it was when it was folded away.)
                        let fraction = match self.palette_share {
                            Some(share) if share < 0.95 => (1.0 - share).clamp(0.12, 0.88),
                            _ => self.palette_split(),
                        };
                        self.settings.hide_palettes = false;
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
                        // Where one of its kind was last dragged to, if
                        // that pane is there; a window of its own else.
                        let palette =
                            self.dock.find_tab(&Tab::Palette).map(|p| (p.surface, p.node));
                        let pane = match self.place_for(&tab) {
                            Place::Window => None,
                            Place::Palettes => self
                                .dock
                                .find_tab(&Tab::Palette)
                                .map(|p| egui_dock::NodePath { surface: p.surface, node: p.node }),
                            Place::Middle => (self.dock.iter_leaves().map(|(p, _)| p)).find(|p| {
                                p.surface.is_main() && Some((p.surface, p.node)) != palette
                            }),
                        };
                        match pane {
                            Some(pane) => {
                                self.dock.set_focused_node_and_surface(pane);
                                self.dock.push_to_focused_leaf(tab.clone());
                                self.tab_places.remove(&tab);
                            }
                            None => self.open_window(tab.clone()),
                        }
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
            Action::FindStrRef(n) => self.find_references(references::Query::StrRef(n)),
            Action::RenameDialog(k) => self.rename_dialog(k),
            Action::CopyDialog(k) => self.copy_dialog(k, false),
            Action::PlaceableGallery => {
                // (For the selection: not the placeable it was last opened for.)
                self.placeable_gallery = Some(Default::default());
                self.run_now(Action::OpenTab(Tab::PlaceableGallery));
            }
            Action::EditScript { name, condition } => self.edit_script(name, condition),
            Action::DeleteDialog(k) => self.confirm_delete = Some(k),
            Action::DeleteResource(k) => self.delete_resource(k),
            Action::CompileScripts => self.compile_scripts(),
            Action::CompileModels => self.compile_models(),
            Action::Verify => self.verify(),
            Action::TestModule => self.test_module(false),
            Action::TestModuleChoose => self.test_module(true),
            Action::TestFromHere { area, at, facing } => self.test_from_here(area, at, facing),
            Action::ReloadResources => self.reload_resources(true),
            Action::CompileIncluders(key) => self.compile_includers(key),
            Action::SaveTalkTable => {
                talk_view::save(self);
            }
            Action::ExportMinimap(area) => self.export_minimap(area),
            Action::StaticPlaceables(areas) => self.static_placeables(&areas),
            Action::DynamicPlaceables(areas) => self.dynamic_placeables(&areas),
            Action::TalkCsv(import) => talk_view::transfer(self, import, false),
            Action::TalkJson(import) => talk_view::transfer(self, import, true),
            Action::TalkFile(new) => talk_view::file(self, new),
            Action::SaveResource(key) => browser::save_as(self, key, false),
            Action::SaveModelText(key) => browser::save_as(self, key, true),
            Action::SaveResources(keys) => browser::export(self, &keys),
            Action::OpenResource(key) => {
                let Some(tab) = Tab::for_resource(key) else { return };
                // Options › Script Editor: scripts open in the external
                // editor (once Moonglow's editor has the script).
                if key.restype == ResType::NSS
                    && self.settings.scripts_external
                    && self.settings.external_editor.is_some()
                {
                    self.script_tools.open_externally.insert(key);
                }
                self.run_now(Action::OpenTab(tab));
            }
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
        // One of the game's blueprints, open to look at: nothing changes it.
        let viewed = |e: &mg_edit::Edit| match e {
            mg_edit::Edit::SetField { key, .. }
            | mg_edit::Edit::InsertItem { key, .. }
            | mg_edit::Edit::RemoveItem { key, .. } => ws.is_viewed(key),
            mg_edit::Edit::SetResource { .. } => false,
        };
        if cmd.edits.iter().any(viewed) {
            return Ok(());
        }
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
    /// command): the scripts it stored.
    pub(crate) fn store_script_text(&mut self) -> Vec<ResKey> {
        if self.ws.is_none() {
            return Vec::new();
        }
        let mut edits = Vec::new();
        for (key, buf) in self.scripts.iter_mut().filter(|(_, b)| b.is_dirty()) {
            buf.saved = buf.text.clone();
            edits.push(mg_edit::Edit::SetResource {
                key: *key,
                data: Some(text::encode(&buf.text)),
            });
        }
        let stored: Vec<ResKey> = edits
            .iter()
            .filter_map(|e| match e {
                mg_edit::Edit::SetResource { key, .. } => Some(*key),
                _ => None,
            })
            .collect();
        if !edits.is_empty() {
            let n = edits.len();
            if let Err(e) = self.apply(Command::new(format!("Save {n} script(s)"), edits)) {
                self.log.error(e.to_string());
            }
        }
        stored
    }

    fn save(&mut self, to: Option<ModuleLocation>) {
        // A module never saved is asked a place first: Save As comes back
        // here with it, and a save called off there does nothing.
        if to.is_none() && self.ws.as_ref().is_some_and(|ws| ws.module.location.is_none()) {
            self.run_now(Action::SaveAsDialog);
            return;
        }
        // (A Ctrl + wheel scaling still under way is part of what is saved.)
        area_view::commit_wheel_scales(self);
        // Options > General: Build module on save.
        if self.settings.build_on_save && self.ws.is_some() {
            build_view::build_on_save(self);
        }
        // Script editors' unsaved text goes into the module first, as saving
        // everything means; so does the talk table, a file of its own.
        let stored = self.store_script_text();
        // Options › Script Editor › Automatically Compile Scripts on Save:
        // the scripts whose text was just stored are compiled before the
        // module is written (saving the module, not only a script's own
        // Save, is a save).
        let stored = match self.settings.auto_compile && !stored.is_empty() {
            // (With the scripts that include them: theirs are older too.)
            true => self.with_includers(&stored).unwrap_or_default(),
            false => Vec::new(),
        };
        if !stored.is_empty() {
            let (compiled, _) = script_view::compile_stale(self, &stored);
            if !compiled.is_empty() {
                self.log.info(format!("Compiled on save: {}", transfer::listed(&compiled)));
            }
            let failed: Vec<String> = stored
                .iter()
                .filter(|k| !compiled.contains(&k.resref.to_string()))
                .filter(|k| self.script_fails(**k))
                .map(|k| k.resref.to_string())
                .collect();
            if !failed.is_empty() {
                self.log.error(format!(
                    "Did not compile (saved as they are; Compile in the script's editor says why): {}",
                    transfer::listed(&failed)
                ));
            }
        }
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
            None => ws.save().map_err(|e| e.to_string()),
        };
        match result {
            Ok(()) => {
                let path = self.module_path().unwrap_or_default();
                self.log.info(format!("Saved {}", path.display()));
                self.settings.remember(&path);
                self.forget_recovery();
                self.write_mod_beside_folder();
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

    /// A module that is a folder (and no nasher project, which packs its
    /// own target) is written as `<folder>.mod` beside the folder too when
    /// it is saved, as Aurora saves a module directory; unless Options ›
    /// General switches that off.
    fn write_mod_beside_folder(&mut self) {
        let Some(ws) = self.ws.as_ref().filter(|_| !self.settings.no_mod_beside_folder) else {
            return;
        };
        let Some(ModuleLocation::Folder(dir)) = &ws.module.location else { return };
        if ws.module.project.is_some() {
            return;
        }
        let Some(name) = dir.file_name().map(|n| n.to_string_lossy().into_owned()) else { return };
        let path = dir.with_file_name(format!("{name}.mod"));
        // (As saving a .mod: the one there kept as its backup, and the new
        // one written beside it, then moved into its place.)
        if !self.settings.no_backups && path.is_file() {
            let backup = path.with_extension("BackupMod");
            if let Err(e) = std::fs::copy(&path, &backup) {
                self.log.warn(format!("{}: {e}", backup.display()));
            }
        }
        let part = dir.with_file_name(format!("{name}.mod.moonglow-tmp"));
        let written = ws
            .module
            .to_archive_bytes()
            .map_err(|e| e.to_string())
            .and_then(|bytes| std::fs::write(&part, bytes).map_err(|e| e.to_string()))
            .and_then(|()| std::fs::rename(&part, &path).map_err(|e| e.to_string()));
        match written {
            Ok(()) => self.log.info(format!("Wrote {}", path.display())),
            Err(e) => self.log.error(format!("{}: {e}", path.display())),
        }
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
        let external = self.external_compiler();
        let (Some(ws), Some(game)) = (&mut self.ws, &self.game) else { return };
        let Ok(results) = ws.derive(|module| {
            mg_module::build::compile_scripts_with(
                module,
                &game.resman,
                mg_module::build::ScriptSelection::Uncompiled,
                external.as_ref(),
            )
        }) else {
            return;
        };
        let failed = results.iter().filter(|r| r.result.is_err()).count();
        if failed > 0 {
            self.log.warn(format!("{failed} script(s) didn't compile; see Build › Compile"));
        }
    }

    /// The external compiler chosen in Options › Script Editor, told where
    /// the game, the user folder and the module's haks are; `None`: the
    /// built-in compiler.
    pub(crate) fn external_compiler(
        &mut self,
    ) -> Option<mg_module::external_compiler::ExternalCompiler> {
        let program = self.settings.external_compiler.clone()?;
        let haks = self.listed_haks();
        let install = self.install.as_ref();
        let dirs = install.map(mg_resman::GameInstall::hak_dirs).unwrap_or_default();
        let haks = haks
            .iter()
            .filter_map(|h| dirs.iter().map(|d| d.join(format!("{h}.hak"))).find(|p| p.is_file()))
            .collect();
        Some(mg_module::external_compiler::ExternalCompiler {
            program,
            arguments: self.settings.external_compiler_args.clone(),
            game: install.map(|i| i.root.clone()),
            user: install.and_then(|i| i.user_dir.clone()),
            haks,
            debug: self.settings.debug_info,
        })
    }

    /// Compile All Scripts, as a job: the bytecode that changed is stored
    /// through one undoable command when it is done. A script is passed
    /// over where it, and what it includes, is as it was when this window
    /// last compiled it (its stamp: `mg_module::build::script_stamps`).
    fn compile_scripts(&mut self) {
        self.compile_job(None);
    }

    /// Build › Compile Models, as a job: the module's own models kept as
    /// text are compiled (`mg_module::models`), each against its
    /// supermodel from the module, its haks or the game, and stored as
    /// one undoable command. One that can't be stays as text, and the log
    /// says why.
    fn compile_models(&mut self) {
        if self.ws.is_none() || self.game.is_none() {
            self.log.error("Compiling models needs an open module and the game data");
            return;
        }
        self.start_job(
            "Compile Models",
            |job| {
                let game = job.game.as_deref()?;
                let files: Vec<(ResKey, &[u8])> = (job.module.keys_of(ResType::MDL))
                    .filter_map(|k| Some((*k, job.module.get(k)?)))
                    .collect();
                let lookup = |name: &str| {
                    game.resman.get_named(name, ResType::MDL).ok().map(|d| d.into_owned())
                };
                let each = |done: usize, total: usize| {
                    job.progress.step(done, total);
                    !job.progress.cancelled()
                };
                Some(mg_module::models::compile_models_each(&files, &lookup, &each))
            },
            |app, made| {
                let Some(results) = made else { return };
                let (mut edits, mut left, mut notes) = (Vec::new(), Vec::new(), 0);
                for c in results {
                    match c.result {
                        Ok((binary, said)) => {
                            notes += said.len();
                            edits.push(mg_edit::Edit::SetResource {
                                key: c.model,
                                data: Some(binary),
                            });
                        }
                        Err(why) => left.push(format!("{}: {why}", c.model)),
                    }
                }
                let compiled = edits.len();
                if compiled == 0 && left.is_empty() {
                    app.log.info("Compile Models: the module has no models kept as text");
                    return;
                }
                if !edits.is_empty()
                    && let Err(e) = app.apply(Command::new("Compile Models", edits))
                {
                    app.log.error(e.to_string());
                    return;
                }
                app.log.info(format!(
                    "Compile Models: {compiled} compiled ({notes} notes of the compiler's), {} \
                     left as text",
                    left.len()
                ));
                for l in left.iter().take(12) {
                    app.log.warn(format!("Left as text: {l}"));
                }
                if left.len() > 12 {
                    app.log.warn(format!("…and {} more left as text", left.len() - 12));
                }
            },
        );
    }

    /// Forgets what Compile All remembers in this session, as a new
    /// session begins: what is kept in [`compiled_dir`](Self::compiled_dir)
    /// is read at the next compile.
    pub fn forget_compiled(&mut self) {
        self.script_stamps = Default::default();
    }

    /// Compiles scripts as a job: all of them (`None`: those changed
    /// since they were last compiled here), or those named.
    pub(crate) fn compile_job(&mut self, only: Option<Vec<ResKey>>) {
        if self.ws.is_none() || self.game.is_none() {
            self.log.error("Compiling needs an open module and the game data");
            return;
        }
        let external = self.external_compiler();
        // What the stamps were made with: another module, compiler,
        // setting, or game data (the haks and folders as they were read),
        // and every script is compiled again.
        let made_with = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            self.module_path().hash(&mut h);
            (self.settings.debug_info, format!("{external:?}")).hash(&mut h);
            if let (Some(game), Some(install)) = (self.game.as_deref(), self.install.as_ref()) {
                install.root.hash(&mut h);
                for l in game.resman.layers().iter().filter(|l| user_content(l)) {
                    (&l.label, l.fingerprint).hash(&mut h);
                }
            }
            h.finish()
        };
        // (Kept from the last session, if it was made with the same.)
        let file = (self.compiled_dir.as_ref().zip(self.module_path()))
            .map(|(dir, module)| mg_module::build::Remembered::file(dir, &module));
        if self.script_stamps.made_with != made_with {
            let kept = file.as_deref().and_then(mg_module::build::Remembered::read);
            self.script_stamps = match kept {
                Some(kept) if kept.made_with == made_with => kept,
                _ => mg_module::build::Remembered { made_with, ..Default::default() },
            };
        }
        let known = self.script_stamps.clone();
        let all = only.is_none();
        let title = if all { "Compile All Scripts" } else { "Compile Scripts" };
        self.start_job(
            title,
            move |job| {
                let game = job.game.as_deref()?;
                let mut staged = job.module.clone();
                let stamps = mg_module::build::script_stamps(&job.module);
                let skipped = |k: &ResKey| {
                    let target = job.module.project.as_ref().map(|p| p.target());
                    target.is_some_and(|t| t.skips_compiling(&k.to_string()))
                };
                let stale = |k: &ResKey| known.stale(&job.module, &stamps, k);
                let names: Vec<ResKey> = match only {
                    Some(names) => names,
                    None => (job.module.keys_of(ResType::NSS))
                        .filter(|k| !skipped(k) && stale(k))
                        .copied()
                        .collect(),
                };
                let scripts = job.module.keys_of(ResType::NSS).filter(|k| !skipped(k)).count();
                let results = mg_module::build::compile_named_scripts(
                    &mut staged,
                    &game.resman,
                    &names,
                    external.as_ref(),
                );
                let edits: Vec<mg_edit::Edit> = staged
                    .keys_of(ResType::NCS)
                    .filter(|k| staged.get(k) != job.module.get(k))
                    .map(|k| mg_edit::Edit::SetResource {
                        key: *k,
                        data: staged.get(k).map(<[u8]>::to_vec),
                    })
                    .collect();
                // Those compiled are remembered as they are now; one that
                // failed is tried again next time.
                let mut now = known.clone();
                for r in &results {
                    match r.result {
                        Ok(()) => now.note(&staged, &stamps, r.script),
                        Err(_) => _ = now.scripts.remove(&r.script),
                    }
                }
                Some((results, edits, now, scripts))
            },
            move |app, made| {
                let Some((results, edits, now, scripts)) = made else { return };
                let failed: Vec<String> = results
                    .iter()
                    .filter_map(|r| r.result.as_ref().err().map(|e| e.message.clone()))
                    .collect();
                for f in &failed {
                    app.log.error(f.clone());
                }
                app.script_stamps = now;
                if let Some(file) = &file
                    && let Err(e) = app.script_stamps.write(file)
                {
                    crate::trace::note(format!("compiled scripts not noted in {file:?}: {e}"));
                }
                let changed = edits.len();
                if !edits.is_empty()
                    && let Err(e) = app.apply(Command::new("Compile scripts", edits))
                {
                    app.log.error(e.to_string());
                }
                let left = scripts.saturating_sub(results.len());
                let unchanged = match all && left > 0 {
                    true => format!("; {left} as they were when last compiled, left alone"),
                    false => String::new(),
                };
                app.log.info(format!(
                    "Compiled {} scripts: {} failed, {changed} changed{unchanged}",
                    results.len(),
                    failed.len()
                ));
            },
        );
    }

    /// Starts the game on the saved module (Test Module).
    fn test_module(&mut self, choose: bool) {
        if test_module::game_running() {
            self.log.warn(
                "The game started for the last test is still running: close it, then test \
                 again (a second test would write the module under it)",
            );
            return;
        }
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
        let Some(user) = install.user_dir else {
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
        if test_module::game_running() {
            self.log.warn(
                "The game started for the last test is still running: close it, then test \
                 again (a second test would write the module under it)",
            );
            return;
        }
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
        let Some(game) = exclusive(&mut self.game) else { return };
        let mut changed = match game.resman.reload_changed(user_content) {
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
            text::set_game_codepage(Some(game.codepage()));
        }
        // (The game's own includes and nwscript may be others now.)
        self.script_stamps = Default::default();
        self.pictures = Default::default();
        self.thumbnails.forget(self.viewport.as_ref());
        self.palettes = Default::default();
        self.palette.forget_game_data();
        self.model_views.clear();
        for view in self.area_views.values_mut() {
            view.reload();
        }
        self.load_order_changed();
    }

    /// Makes the placeables of `areas` static, as one command, where
    /// nothing is lost by it (`mg_module::areas::static_plan`: not the
    /// Useable ones, those a visual transform changes, nor those with
    /// scripts and the like), and says in the log what was left.
    fn static_placeables(&mut self, areas: &[mg_core::ResRef]) {
        let Some(ws) = self.ws.as_mut() else { return };
        let mut edits = Vec::new();
        let (mut made, mut within) = (0, 0);
        let mut left = [0usize; 3];
        for &area in areas {
            let key = ResKey::new(area, ResType::GIT);
            let Ok(git) = ws.doc(&key) else { continue };
            let plan = mg_module::areas::static_plan(&git.root);
            let list = git.root.list("Placeable List").unwrap_or(&[]);
            for &i in &plan.convert {
                let path = mg_edit::GffPath::root().item("Placeable List", i);
                let set = |label: &str, value: Option<mg_gff::Value>| mg_edit::Edit::SetField {
                    key,
                    path: path.clone(),
                    label: label.into(),
                    value,
                };
                edits.push(set("Static", Some(mg_gff::Value::Byte(1))));
                // (One that changes nothing: static placeables have none.)
                for label in ["VisTransformList", "VisualTransform"] {
                    if list[i].contains(label) {
                        edits.push(set(label, None));
                    }
                }
            }
            made += plan.convert.len();
            within += usize::from(!plan.convert.is_empty());
            for (n, more) in left.iter_mut().zip([plan.useable, plan.transformed, plan.active]) {
                *n += more;
            }
        }
        let kept = format!(
            "left dynamic: {} Useable, {} with a visual transform, {} with a script, \
             conversation, trap, inventory or animation",
            left[0], left[1], left[2]
        );
        if edits.is_empty() {
            self.log.info(format!("No placeables to make static ({kept})"));
            return;
        }
        match self.apply(Command::new("Make Placeables Static", edits)) {
            Ok(()) => {
                let (s, a) = (if made == 1 { "" } else { "s" }, if within == 1 { "" } else { "s" });
                self.log
                    .info(format!("Made {made} placeable{s} static in {within} area{a} ({kept})"));
            }
            Err(e) => self.log.error(e.to_string()),
        }
    }

    /// Makes every static placeable of `areas` dynamic, as one command.
    fn dynamic_placeables(&mut self, areas: &[mg_core::ResRef]) {
        let Some(ws) = self.ws.as_mut() else { return };
        let mut edits = Vec::new();
        let mut within = 0;
        for &area in areas {
            let key = ResKey::new(area, ResType::GIT);
            let Ok(git) = ws.doc(&key) else { continue };
            let before = edits.len();
            for (i, p) in git.root.list("Placeable List").unwrap_or(&[]).iter().enumerate() {
                if p.integer("Static").unwrap_or(0) != 0 {
                    edits.push(mg_edit::Edit::SetField {
                        key,
                        path: mg_edit::GffPath::root().item("Placeable List", i),
                        label: "Static".into(),
                        value: Some(mg_gff::Value::Byte(0)),
                    });
                }
            }
            within += usize::from(edits.len() > before);
        }
        if edits.is_empty() {
            self.log.info("No static placeables to make dynamic");
            return;
        }
        let made = edits.len();
        match self.apply(Command::new("Make Placeables Dynamic", edits)) {
            Ok(()) => {
                let (s, a) = (if made == 1 { "" } else { "s" }, if within == 1 { "" } else { "s" });
                self.log.info(format!("Made {made} placeable{s} dynamic in {within} area{a}"));
            }
            Err(e) => self.log.error(e.to_string()),
        }
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
        let Ok(info) = ws.doc(&module_props::info_key()) else { return Vec::new() };
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
        self.reload_changed_content();
        self.reload_changed_files();
        ui.ctx().request_repaint_after(every);
    }

    fn launch_test(
        &mut self,
        client: &std::path::Path,
        user: &std::path::Path,
        name: &str,
        choose: bool,
    ) {
        let steam = !self.settings.test_without_steam;
        match test_module::command(client, user, name, choose, steam).spawn() {
            Ok(game) => {
                test_module::let_game_run(game);
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
        // The enabled plugins' checks run with the doctor's.
        let plugins: Vec<mg_plugin::Plugin> = self
            .enabled_plugins()
            .into_iter()
            .filter(|p| !p.manifest.checks.is_empty())
            .cloned()
            .collect();
        self.start_job(
            "Verify Module",
            move |job| {
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
                let mut findings = mg_module::doctor::examine(&job.module, &game.resman, tlk);
                let (found, said) = plugins::check_findings(&plugins, job);
                findings.extend(found);
                Some((missing, findings, said))
            },
            |app, made| {
                let Some((missing, findings, said)) = made else { return };
                app.log.entries.extend(said);
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

    /// Whether these options choose another game or user folder: the
    /// game's data is read again then, and the module with it.
    fn options_reload(&self, draft: &OptionsDraft) -> bool {
        let settings = draft.apply(&self.settings);
        // (Another language edited, if the game has it: its talk table.)
        let language = |i: Option<GameInstall>| i.map(|i| i.language);
        settings.game_root != self.settings.game_root
            || settings.user_dir != self.settings.user_dir
            || language(settings.install()) != language(self.settings.install())
    }

    /// Options › OK. The module stays open: only another game or user
    /// folder has the game's data read again, and then the module is opened
    /// again from its file (the caller has asked about unsaved work).
    fn apply_options(&mut self, draft: OptionsDraft) {
        let reload = self.options_reload(&draft);
        let settings = draft.apply(&self.settings);
        let language = settings.edit_language != self.settings.edit_language;
        set_edit_language(mg_core::Language(settings.edit_language.unwrap_or(0)));
        if language {
            // Text fields showing the other language's text are read again.
            self.buffers.clear();
        }
        if !reload {
            self.settings = settings;
            if language {
                // Names are shown in the language chosen: read them again.
                self.area_names = Default::default();
                self.area_contents.clear();
                self.palette.forget_game_data();
            }
            return;
        }
        let module = self.module_path();
        self.close();
        self.settings = settings;
        self.install = self.settings.install();
        self.game = load_game(self.install.as_ref(), &mut self.log);
        self.load_order_changed();
        if let Some(path) = module {
            self.open_module(&path);
        }
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
}

/// The game data to change (its layers, its talk table): only while no job
/// is reading it, which is whenever the window takes input.
/// A layer of the user's content, which may change on disk (the
/// install's own folders don't).
pub(crate) fn user_content(l: &mg_resman::Layer) -> bool {
    use mg_resman::priority as p;
    matches!(
        l.priority,
        p::HAK
            | p::HAK_USER
            | p::OVERRIDE
            | p::DEVELOPMENT
            | p::DEVELOPMENT_USER
            | p::PORTRAITS_USER
    )
}

pub(crate) fn exclusive(game: &mut Option<Arc<GameData>>) -> Option<&mut GameData> {
    game.as_mut().and_then(Arc::get_mut)
}

/// Loads the game data of an install, logging the outcome.
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
