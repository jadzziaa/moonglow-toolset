//! Moonglow's user interface (egui). [`Moonglow`] is the whole application
//! state; [`Moonglow::ui`] draws it into any `egui::Ui`, so the same code runs
//! in the desktop window and in `egui_kittest` tests. Widgets never change
//! the module directly: they queue [`Action`]s, which run after the frame and
//! turn edits into undoable [`mg_edit::Command`]s.

mod appearance_gallery;
mod area_audio;
mod area_grass;
mod area_props;
pub mod area_reshape;
pub mod area_sets;
pub mod area_tools;
pub mod area_view;
pub mod audio;
pub mod blueprint;
pub mod blueprint_wizard;
mod browser;
pub mod build_view;
pub mod builder_code;
pub mod bulk;
mod claims;
pub mod color_button;
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
pub mod lighting;
mod manual;
mod minimap;
mod model_textures;
pub mod model_view;
pub mod module_folder;
pub mod module_props;
mod nui_view;
pub mod open_tabs;
/// NUI windows drawn off the screen: `mg nui render` and `mg nui layout`.
pub mod nui_render {
    pub use crate::nui_view::headless::{
        Area, Element, Error, Layout, Look, Options, Picture, Window, layout, render,
    };
}
pub mod nwsync_view;
mod options;
mod outside;
pub mod paint_mode;
pub mod palette_categories;
pub mod palette_view;
pub mod plugin_events;
pub mod plugins;
pub mod prefabs;
mod project_commands;
pub mod recovery;
pub mod references;
pub mod renderer_check;
pub mod saved;
pub mod script_nav;
pub mod script_tools;
mod script_view;
pub mod script_wizard;
pub mod search;
pub mod settings;
mod side_doc;
pub mod store_wizard;
pub mod table_view;
mod tabs;
pub mod tag_clash;
pub mod talk_view;
pub mod terrain_mode;
pub mod test_module;
mod text;
pub mod tile_select;
mod tile_view;
pub mod tileset_view;
pub mod trace;
mod transfer;
mod tree;
pub mod updates;
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
pub use mg_render::backend as graphics;
/// Direct3D 12's shader compiler (DXC beside the program, else FXC).
pub use mg_render::dx12;
/// Carrying on when one model fails (for the application's panic hook).
pub use mg_render::guard as model_guard;
pub use options::{OptionsDraft, OptionsPage};
pub use settings::{GraphicsBackend, Settings};
pub use tabs::Tab;
pub use text::set_edit_language;
pub use transfer::{ExportDraft, ImportDraft};
pub use wizards::{AreaWizard, Wizard};

/// `setup` (egui-wgpu's: the window's, or a test's) with the backend
/// options every wgpu instance Moonglow makes takes: Direct3D 12 compiles
/// with the compiler `mg_render::dx12` chose (the DXC beside the program,
/// else FXC).
pub fn graphics_setup(mut setup: egui_wgpu::WgpuSetup) -> egui_wgpu::WgpuSetup {
    if let egui_wgpu::WgpuSetup::CreateNew(new) = &mut setup {
        new.instance_descriptor.backend_options = mg_render::dx12::backend_options();
    }
    setup
}

/// Where a tool window is, or opens: a window of its own over the area
/// (as each does at first), a tab of the main pane beside the areas, or a
/// tab of the palettes' pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Window,
    Middle,
    Palettes,
    /// A pane of its own split off a side of the middle (GitHub issue 22),
    /// this many thousandths of it.
    Split(Side, u16),
}

/// A side of the middle a pane is split off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Left,
    Right,
    Above,
    Below,
}

impl Side {
    fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
            Side::Above => "above",
            Side::Below => "below",
        }
    }
}

/// Something the user asked for, run after the frame is drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Folds the palettes' pane away (as their fold button and View do).
    FoldPalettes,
    NewModuleDialog,
    /// Creates a module with this name (then opens the Area Wizard).
    NewModule(String),
    AreaWizard,
    NewArea(AreaSpec),
    OpenModuleDialog,
    /// Opens a module the user chose: a `.mod` with a folder of its name
    /// beside it asks which first (Options › General).
    OpenModule(PathBuf),
    /// Opens this module as it is (the answer to that question).
    OpenModuleAsIs(PathBuf),
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
    /// Export Model Files… of an area: what the game draws it with, into a
    /// folder asked for (`model_view::export_area_files`).
    ExportAreaFiles(mg_core::ResRef),
    /// Export as Blueprint…: a placed object written into an archive as a
    /// blueprint of its own (`transfer::run_export_object`).
    ExportObject {
        area: mg_core::ResRef,
        kind: mg_area::ObjectKind,
        index: usize,
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
    /// Build › Pack *file*: a nasher project's module file, as nasher packs it.
    PackTarget,
    /// Build › Add Left-Out Files to nasher.cfg: an include line for the
    /// files the project's include leaves out, and they are read.
    TakeInLeftOut,
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
    /// Runs a plugin's command on what a right-click menu was opened on
    /// (its `ctx.selection`).
    RunPluginCommand {
        plugin: String,
        command: String,
        selected: Vec<mg_plugin::Selected>,
    },
    /// Makes the fix offered beside this line of the log (a finding of
    /// Verify Module's), as one undoable command.
    ApplyFix(usize),
    /// Opens a resource's editor, as a double click in the module tree
    /// does: a script in the external editor too, if Options say so.
    OpenResource(ResKey),
    /// The talk table's lines to a CSV file, or (true) read from one.
    TalkCsv(bool),
    /// Find References for a talk-table line, by its StrRef.
    FindStrRef(u32),
    /// The resource browser's Save As: a resource of the load order to a
    /// file (the copy the game reads, or the one a layer has, by its
    /// label).
    SaveResource(ResKey, Option<String>),
    /// A model of the load order saved as text (decompiled).
    SaveModelText(ResKey, Option<String>),
    /// The resource browser's Export as Files: those listed, into a folder
    /// (an overridden copy with its layer's label, as that layer has it).
    SaveResources(Vec<(ResKey, Option<String>)>),
    /// The same, as JSON (`nwn_tlk`'s).
    TalkJson(bool),
    /// The talk table editor opens a `.tlk` file anywhere, or (true) makes
    /// one.
    TalkFile(bool),
    /// Makes every static placeable of these areas dynamic.
    DynamicPlaceables(Vec<mg_core::ResRef>),
    Apply(Command),
    /// Applies a command that makes something (a wizard's, a paste's),
    /// with the edits the plugins' handlers of what it makes add to it
    /// (`plugin_events`): one command.
    ApplyMade(Command, plugin_events::Made),
    /// Creates a NUI window with this name and opens it in the NUI Creator.
    NewNui(String),
    /// Builds a NUI window's opener include and event script (Build &
    /// compile), then, when `export`, offers what it is made of for export.
    GenerateNui {
        name: String,
        export: bool,
    },
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
    /// `area` at `at`, facing `facing` (radians from east); with
    /// `choose`, by the game's character selection, as Test Module,
    /// Choose Character.
    TestFromHere {
        area: mg_core::ResRef,
        at: [f32; 3],
        facing: f32,
        choose: bool,
    },
    /// Tools › Reload Resources: haks and folders that changed on disk.
    ReloadResources,
    /// The scripts that include this one are compiled again (it was
    /// saved, with Automatically Compile Scripts on Save).
    CompileIncluders(ResKey),
    /// Scripts compiled one by one (a script's Compile, a save), each
    /// with the compiled script made: noted as compiled where the module
    /// has it now, for Compile All to pass over.
    NoteCompiled(Vec<(ResKey, Vec<u8>)>),
    /// A script editor's Save (its toolbar's, or the Save key while its
    /// text has the keyboard): the text into the module, not the module
    /// to its file.
    SaveScript(ResKey),
    /// A script editor's To Scratch, once the script is saved (and
    /// compiled, if `compiled`): its source copied into the scratch
    /// folder, and its compiled script only if it compiled just now.
    ScriptToScratch {
        key: ResKey,
        compiled: bool,
        /// And made a debug file, which goes with it.
        debug: bool,
    },
    /// Edit › Prefabs › a prefab: placed with a click in the area shown.
    PlacePrefab(String),
    /// Edit › Find References: where a resource is used.
    FindReferences(ResKey),
    /// The Rename window for a resource.
    RenameDialog(ResKey),
    /// Opens the Placeable Gallery.
    PlaceableGallery,
    /// The palette's More › Refresh Palette: the module's custom palette
    /// of a type made anew from the blueprints, as Aurora's is.
    RefreshPalette(mg_module::palette::BlueprintKind),
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
    /// Asks whether to delete the resources chosen in the module tree (its
    /// Delete… of several).
    DeleteChosenDialog(Vec<ResKey>),
    /// Claim This Area: the area claimed by the builder's code in the
    /// project's claims table.
    Claim(mg_core::ResRef),
    /// Release Claim: the builder's claim of the area let go.
    ReleaseClaim(mg_core::ResRef),
    /// File › Project › a command the project names (`[commands]`): the
    /// module saved and its files let go, the command run in the project's
    /// folder (allowed first), the module's files read again.
    ProjectCommand(String),
    /// Deletes a resource of the module (an area with its objects, a
    /// script with its compiled one).
    DeleteResource(ResKey),
    /// Deletes resources of the module, as one step.
    DeleteResources(Vec<ResKey>),
    /// Renames a blueprint (and points its editor at the new name).
    RenameBlueprint {
        from: ResKey,
        to: mg_core::ResRef,
    },
    CompileScripts,
    /// Build › Compile Models: the module's models kept as text.
    CompileModels,
    Verify,
    /// Build › Round-Trip Check: the module saved to a copy and read back,
    /// its GFFs and 2DAs written anew (`mg roundtrip`).
    RoundTrip,
    /// Saves, then starts the game on the module (F9).
    TestModule,
    Quit,
    /// The window's own close button: Quit, asked about first where there
    /// is no unsaved work to ask about (Options › General).
    CloseWindow,
    /// The renderer's notice: Quit (unsaved work asked about first), and
    /// start again with the module, through this backend
    /// ([`renderer_check::restart`]).
    RestartGraphics(wgpu::Backend),
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
    /// When each entry was said, the local time of day (`14:03:27`), as
    /// the Compiler tab's messages have it (Jasperre's report).
    pub times: Vec<String>,
    /// The fixes the last Verify's findings offer, each beside its line.
    pub fixes: Vec<OfferedFix>,
    /// What those fixes were worked out on.
    pub(crate) fix_guard: mg_edit::fix::Guard,
    /// Each line's height as the panel last drew it (0: not yet): the
    /// lines out of sight take their room alone.
    rows: Vec<f32>,
}

/// A fix offered beside a line of the log (a finding of Verify Module's):
/// made only when its button is clicked, as one undoable command.
#[derive(Debug, Clone)]
pub struct OfferedFix {
    /// The line's place in the log, and its text (a line that is not the
    /// one it was offered beside has no button).
    pub line: usize,
    pub text: String,
    pub fix: mg_edit::fix::Fix,
    /// Made already.
    pub made: bool,
}

impl Log {
    /// Empties the log, and the fixes beside its lines go with them.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.times.clear();
        self.fixes.clear();
        self.rows.clear();
    }

    /// The fix offered beside line `line`, if one is.
    pub fn fix_at(&self, line: usize) -> Option<&OfferedFix> {
        let text = &self.entries.get(line)?.1;
        self.fixes.iter().find(|f| f.line == line && f.text == *text)
    }

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
        self.times.push(script_tools::clock());
    }

    /// Line `line` as the log shows it: the time it was said, then the
    /// message.
    pub fn shown(&self, line: usize) -> String {
        let msg = self.entries.get(line).map_or("", |e| e.1.as_str());
        match self.times.get(line) {
            Some(at) => format!("{at} {msg}"),
            None => msg.to_string(),
        }
    }
    pub fn error(&mut self, s: impl Into<String>) {
        self.said(Level::Error, s.into());
    }

    /// The whole log as text, a line each as the log shows it (its time
    /// first), for Copy Log and Save Log….
    pub fn text(&self) -> String {
        (0..self.entries.len()).map(|i| self.shown(i) + "\n").collect()
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
    /// The game's shader programs the log was told couldn't be drawn
    /// (the renderer's [`mg_render::Renderer::game_shader_failures`]): told
    /// once each.
    shader_failures_told: Vec<String>,
    pub actions: Vec<Action>,
    pub dialogs: Box<dyn Dialogs>,
    /// Where sounds play: [`audio::Silence`] until the desktop app gives
    /// its speakers.
    pub speaker: Box<dyn audio::Speaker>,
    /// Game images decoded for the UI, by lowercase name.
    pictures: images::Pictures,
    /// The game's NUI skin, its fonts and the pictures the open windows
    /// use, as the NUI Creator's preview draws them.
    pub(crate) nui_assets: nui_view::skin::Assets,
    /// What each NUI Creator tab shows of its window: the selection, the
    /// preview's settings, the checks.
    pub(crate) nui_views: HashMap<ResKey, nui_view::State>,
    /// The New NUI Window window's name.
    pub(crate) new_nui: Option<String>,
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
    /// The text field of a Properties window that has the keyboard: when
    /// it was drawn last, and what it would set if left now (`None`:
    /// nothing typed). A field that is gone before the keyboard left it
    /// (another page chosen, its window closed) is set from this, so that
    /// what was typed is not lost.
    pub(crate) held_text: Option<(egui::Id, u64, Option<Command>)>,
    /// The tabs by when each was last in front, the latest first.
    tabs_recent: Vec<Tab>,
    /// Next Tab going through the tabs by when they were last used: the
    /// order it began with, and where it is in it.
    tab_cycle: Option<(Vec<Tab>, usize)>,
    pub wizard: Option<Wizard>,
    /// An open blueprint wizard.
    pub blueprint_wizard: Option<blueprint_wizard::BlueprintWizard>,
    pub settings: Settings,
    /// The Options window's pending values, while it is open.
    pub options: Option<OptionsDraft>,
    pub export: Option<ExportDraft>,
    /// An Export to an archive that is there, asking whether to append.
    pub export_ask: Option<transfer::ExportAsk>,
    pub browser: Browser,
    pub loc_edit: Option<widgets::LocStringEdit>,
    pub module_page: module_props::Page,
    /// Each open blueprint editor's page.
    pub blueprint_pages: blueprint::Pages,
    /// The ids of placed objects' Properties windows (and of the items
    /// they hold), by where their objects are: a window keeps its id as
    /// it follows its object, and with it its widgets' state (its scroll,
    /// a field being typed in). Given as a window is first drawn.
    pub(crate) window_ids: HashMap<(mg_core::ResRef, mg_edit::GffPath), egui::Id>,
    /// What each open Properties window has changed, for its Cancel.
    pub(crate) properties_changed: blueprint::Changed,
    pub faction_view: faction_view::FactionView,
    pub journal_view: journal_view::JournalView,
    pub dialog_views: HashMap<ResKey, dialog_view::DialogView>,
    /// A conversation opened or brought to the front: its tree takes the
    /// keys at its next frame, as Aurora's does (not the module tree it was
    /// opened from, whose Delete would delete the whole conversation).
    pub(crate) dialog_keys: Option<ResKey>,
    pub dialog_clip: Option<dialog_view::DialogClip>,
    /// The New Conversation window's name.
    pub new_dialog: Option<String>,
    pub script_tools: script_tools::ScriptTools,
    /// The NUI window whose last Build failed, and why: shown by its Build button.
    pub(crate) nui_build_error: Option<(String, String)>,
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
    /// The open project's commands' names and own keys the keymap was
    /// made with.
    project_keys_from: Vec<(String, Option<String>)>,
    /// Haks open in the hak editor.
    pub haks: Vec<hak_view::HakDoc>,
    /// The 2DAs open in the table editor.
    pub tables: Vec<table_view::TableDoc>,
    pub(crate) next_table: u32,
    /// The Open 2DA window.
    pub open_table: Option<table_view::OpenTable>,
    pub(crate) next_hak: u32,
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
    /// The side editors' tabs (a hak's, a 2DA's, a tileset's) closed with
    /// unsaved changes, waiting in turn for the question to be answered
    /// (`side_doc::closing_window`).
    pub(crate) side_closing: std::collections::VecDeque<Tab>,
    /// The side editor (or the talk table's) under the pointer, as drawn
    /// this frame: Undo and Redo are its (`Moonglow::local_undo`).
    pub(crate) undo_under_pointer: Option<Tab>,
    /// The 2DA grid that has the keys, as drawn this frame: Undo and Redo
    /// are its, the pointer over no side editor.
    pub(crate) undo_keys: Option<Tab>,
    /// The side editor the pointer was over before it went onto the menu
    /// bar or the toolbar (or a menu opened): Edit › Undo and the
    /// toolbar's are its while the pointer is there (`pointer_on_bars`).
    pub(crate) undo_before_bars: Option<Tab>,
    pub(crate) pointer_on_bars: bool,
    /// The script whose editor has the keys, or had them when the menu
    /// still open was opened (the click on it takes them): Edit › Undo
    /// and Redo and the toolbar's are that editor's, as its keys are
    /// (`script_view::undo_target`).
    pub(crate) undo_script: Option<ResKey>,
    /// The module Test Module looked for running games on before its save
    /// ([`Moonglow::test_may_start`]): not looked for, and warned of, again
    /// once saved.
    test_checked: Option<String>,
    pub model_views: HashMap<model_view::Source, model_view::ModelView>,
    /// The Palette Categories window, when open.
    pub palette_categories: Option<palette_categories::PaletteCategories>,
    /// The areas' names as last read (Options › General: Show areas by
    /// name).
    pub(crate) area_names: tree::AreaNames,
    /// What is placed in the areas opened out in the module tree (in
    /// every area, while its Filter has something typed), each with what
    /// it was read from (`tree::Read`).
    pub(crate) area_contents: HashMap<mg_core::ResRef, (tree::Read, tree::AreaContents)>,
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
    /// The area views' placeable brush is on (their toolbar's Paint): the
    /// placeables chosen in the palette are painted, not placed one a click.
    pub paint: bool,
    /// Objects copied in an area viewer.
    pub object_clip: Option<area_view::ObjectClip>,
    /// The items copied in an inventory (their blueprints), to paste into
    /// another's.
    pub item_clip: Vec<mg_core::ResRef>,
    /// An object to show and select when its area's view is next drawn.
    pub area_focus: Option<(mg_core::ResRef, mg_area::ObjectKind, usize)>,
    /// The area whose view goes to the start location when next drawn.
    pub(crate) start_focus: Option<mg_core::ResRef>,
    /// The References tab.
    pub references: references::References,
    /// What each script was compiled from when Compile All last compiled
    /// it (its stamp), and what those stamps were made with: a script
    /// whose stamp is the same is not compiled again.
    /// The text field typed in since it got the keyboard: Undo and Redo
    /// are its own there (`commands::keys_pressed`).
    pub(crate) text_typed: Option<egui::Id>,
    /// A command asked for by its key while a text field was typed in:
    /// run in the next frame, after the field has set its text.
    pub(crate) command_after_text: Option<commands::AfterText>,
    pub(crate) script_stamps: mg_module::build::Remembered,
    /// Where what Compile All remembers is kept from one session to the
    /// next (a file a module, in Moonglow's own folder); `None`: not kept.
    pub compiled_dir: Option<PathBuf>,
    /// Where a nasher project's locked objects are kept (a file a
    /// project, in Moonglow's own folder: the lock is kept out of the
    /// project's files); `None`: locks in a project last the session.
    pub locks_dir: Option<PathBuf>,
    /// Where each module's open tabs are kept, to open again with it (a
    /// file a module, in Moonglow's own folder: `open-tabs`); `None`: not
    /// kept.
    pub open_tabs_dir: Option<PathBuf>,
    open_tabs: open_tabs::OpenTabs,
    /// Where the game's shaders, compiled, are kept from one session to the
    /// next (in the user's cache folder); `None`: compiled each session.
    pub shader_cache_dir: Option<PathBuf>,
    /// The Rename window, while open.
    pub rename: Option<references::RenameDraft>,
    /// The Placeable Gallery, while it is open.
    pub placeable_gallery: Option<appearance_gallery::Gallery>,
    /// The Copy window (Edit Copy, the module tree's Copy…).
    pub copy_as: Option<copy_as::CopyDraft>,
    /// The Duplicate Tags window: tags a paste or a copy put on more than
    /// one object, and Retag.
    pub tag_clash: Option<tag_clash::TagClash>,
    /// The module's tags, area by area, for the paste's and Setup Store's
    /// checks ([`tag_clash::index`]).
    pub(crate) tag_cache: mg_module::tags::TagCache,
    /// The Builder Code window (Edit › Builder Code…): the code typed.
    pub builder_code: Option<builder_code::Draft>,
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
    /// The Advanced Search tab's state.
    pub advanced_search: search::AdvancedSearch,
    /// What its last question found in each resource, for the next time
    /// (named with the game data: forgotten with it).
    pub(crate) search_kept: mg_module::search::Kept,
    /// Where prefabs are kept (the app sets Moonglow's data folder's
    /// `prefabs`; none: prefabs can't be saved).
    pub prefab_dir: Option<PathBuf>,
    /// The prefab deleted last, its name and text: the palette's Undo
    /// Delete puts it back.
    pub prefab_deleted: Option<(String, String)>,
    /// Where variable sets are kept (`var_sets`).
    pub var_set_dir: Option<PathBuf>,
    /// Where Edit Areas Together's sets are kept (`area_sets`).
    pub area_set_dir: Option<PathBuf>,
    /// What each area holds, for Edit Areas Together's Holding (read
    /// again once an area's bytes change).
    pub(crate) area_holds: mg_module::areas::HoldsCache,
    /// Where lighting is kept (`lighting`: an area's Visual settings under
    /// a name).
    pub lighting_dir: Option<PathBuf>,
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
    pub(crate) viewed: HashMap<(ResKey, Option<String>), std::sync::Arc<browser::Viewed>>,
    /// The pass in which an area view last had something in hand (a
    /// brush, a paste, a drag, a terrain tool): Escape is its then.
    pub(crate) area_tool_at: u64,
    /// The Properties window Escape is Cancel for this frame, as Aurora's
    /// Escape is (`blueprint::window_drawn_but` presses it).
    pub(crate) escape_cancel: Option<Tab>,
    /// A resource for the module tree to bring into view, its group
    /// opened (and, an area, if asked, opened out to what is placed in
    /// it): that of the tab just chosen, or of its menu's Show in Module
    /// Tree (`tree::reveal_tab`).
    pub tree_reveal: Option<(ResKey, bool)>,
    /// The area the module tree last brought into view.
    pub tree_area: Option<mg_core::ResRef>,
    /// The row the module tree marks as the one in hand: the resource of
    /// the tab last chosen (or the area of the one in front).
    pub tree_marked: Option<ResKey>,
    /// The tab in front when the module tree last looked
    /// (`tree::follow_front_tab`).
    pub(crate) tree_front: (Option<Tab>, Vec<Tab>),
    /// The one object selected in the area in front, as the module tree
    /// shows it (its row marked): its area, kind and place in its list;
    /// and whether the tree has still to go to it.
    pub tree_object: Option<(mg_core::ResRef, mg_area::ObjectKind, usize)>,
    pub tree_object_pending: bool,
    /// The resources chosen together in the module tree (Ctrl and Shift
    /// with a click, or the resources of a tab of several), and the row
    /// a Shift+click goes from. Marked, and what a menu acts on, when
    /// there are several.
    pub tree_selected: std::collections::BTreeSet<ResKey>,
    pub(crate) tree_anchor: Option<ResKey>,
    /// The rows of the tab chosen while the module tree's Filter had this
    /// text typed: listed under it, which would hide them, until it is
    /// changed.
    pub(crate) tree_kept: Option<(Vec<ResKey>, String)>,
    /// The placed objects chosen together in the module tree, those of
    /// one area: as the area view's selection, or by Ctrl and Shift with a
    /// click there; and the row a Shift+click goes from.
    pub tree_objects: Option<(mg_core::ResRef, std::collections::BTreeSet<tree::Placed>)>,
    pub(crate) tree_objects_anchor: Option<tree::Placed>,
    /// The area view's selection as the module tree last saw it.
    pub(crate) tree_seen_selection: Option<(mg_core::ResRef, Vec<tree::Placed>)>,
    /// Objects to select in an area's view once it is open (the module
    /// tree's Go To on several).
    pub(crate) area_select: Option<(mg_core::ResRef, Vec<tree::Placed>)>,
    /// The module tree's row the arrow keys are at (a click puts them
    /// there), whether it is to be brought into view, and a group to open
    /// or close at the next frame.
    pub tree_cursor: Option<tree::TreeAt>,
    pub(crate) tree_cursor_moved: bool,
    pub(crate) tree_fold: Option<(tree::TreeAt, bool)>,
    /// The keys come from the Filter by Down: to the first row it matches
    /// once the rows are laid out.
    pub(crate) tree_to_match: bool,
    pub import: Option<ImportDraft>,
    /// An action waiting for the answer to "save changes?".
    pub confirm_discard: Option<Action>,
    /// A `.mod` waiting for the answer to "open the folder beside it?".
    pub module_folder: Option<PathBuf>,
    /// What is asked of claimed and protected areas.
    pub(crate) claims: claims::ClaimsState,
    /// The project's commands (File › Project): the one asked about, its
    /// output.
    pub(crate) project_commands: project_commands::CommandsState,
    /// The window's close button was pressed with nothing unsaved: asked
    /// whether to close.
    pub closing: bool,
    /// The graphics adapter and its limits, for the debug log.
    render_info: Option<String>,
    /// Why the window draws with its adapter (the desktop app's choice by
    /// Options › General › Graphics backend, `WGPU_BACKEND` or AMD on
    /// Windows), and of which: for the debug log.
    pub graphics_choice: Option<String>,
    /// The backend and the adapter the window draws with, for Options
    /// (`Direct3D 12, AMD Radeon PRO …`).
    pub(crate) drawing_with: Option<String>,
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
    /// Whether the pointer was over an area's view when one was last
    /// drawn: F10 is Select Tiles there, not the menus'.
    pub(crate) area_hovered: bool,
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
    /// The resources waiting for the answer to "delete it?" (one, or those
    /// chosen in the module tree).
    pub confirm_delete: Option<Vec<ResKey>>,
    pub quit_requested: bool,
    /// What the startup check and the area view's watch found of the
    /// renderer here, for the notice under the toolbar.
    pub renderer_check: renderer_check::RendererCheck,
    /// The lit check, once the game's data is read
    /// ([`renderer_check::lit_check`]).
    pub lit_check: renderer_check::LitCheck,
    /// Moonglow is to be started again when it has quit (Restart with …,
    /// in the renderer's notice): the desktop app does it.
    pub relaunch: Option<renderer_check::Relaunch>,
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
    /// Help › Check for Updates… and the daily check (`updates`).
    pub updates: updates::Updates,
    /// The window's content area at the last frame (where new windows go).
    screen: Option<egui::Rect>,
    /// The widths of the module tree and of the panes beside it at the last
    /// frame (the Palettes pane opens as wide as the tree).
    tree_width: Option<f32>,
    dock_width: Option<f32>,
    /// The palettes' pane's share of the middle's width, as last drawn.
    palette_share: Option<f32>,
    /// The side of the middle the palettes' pane was on at the last frame,
    /// with the tabs in it: they leave room along its inner edge for the
    /// button that folds it away (`palettes_edge`).
    palette_edge: Option<(Side, Vec<Tab>)>,
    /// Where the palettes' pane's covered arrow (egui_dock's) was drawn:
    /// the keyboard is moved on past it ([`Moonglow::pass_covered_arrow`]).
    palette_arrow: Option<egui::Rect>,
    /// The last Tab pressed was Shift+Tab: the way the keyboard goes.
    tab_back: bool,
    /// A module was opened: the Palettes pane opens once the module tree
    /// shows (and has a width to match). Tests of other windows turn it
    /// off.
    pub open_palette: bool,
    /// The plugins installed, and what they are doing (`plugins`).
    pub plugins: plugins::Plugins,
    /// Where plugins are installed (the application gives Moonglow's data
    /// folder's `plugins`).
    pub plugin_dir: Option<PathBuf>,
    /// Where the answers about projects' own plugins are kept (which may
    /// run): a file of Moonglow's own folder, never a project's
    /// (`mg_plugin::project::answers_path`). None: kept for this run only.
    pub plugin_answers: Option<PathBuf>,
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
            shader_failures_told: Vec::new(),
            actions: Vec::new(),
            dialogs,
            speaker: Box::new(audio::Silence::default()),
            pictures: HashMap::new(),
            nui_assets: Default::default(),
            nui_views: HashMap::new(),
            new_nui: None,
            palettes: HashMap::new(),
            area_audio: Default::default(),
            heard: None,
            scripts: HashMap::new(),
            laid_out: HashMap::new(),
            buffers: HashMap::new(),
            held_text: None,
            tabs_recent: Vec::new(),
            tab_cycle: None,
            wizard: None,
            blueprint_wizard: None,
            settings: Settings::default(),
            options: None,
            export: None,
            export_ask: None,
            browser: Browser::new(),
            loc_edit: None,
            module_page: module_props::Page::default(),
            blueprint_pages: Default::default(),
            window_ids: Default::default(),
            properties_changed: Default::default(),
            faction_view: faction_view::FactionView::default(),
            journal_view: journal_view::JournalView::default(),
            dialog_views: HashMap::new(),
            dialog_keys: None,
            dialog_clip: None,
            new_dialog: None,
            script_tools: script_tools::ScriptTools::default(),
            nui_build_error: None,
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
            project_keys_from: Vec::new(),
            haks: Vec::new(),
            next_hak: 0,
            tables: Vec::new(),
            next_table: 0,
            open_table: None,
            hak_list: Default::default(),
            suggested_hak: None,
            attach: None,
            publish: None,
            tilesets: Vec::new(),
            next_tileset: 0,
            side_closing: Default::default(),
            undo_under_pointer: None,
            undo_keys: None,
            undo_before_bars: None,
            pointer_on_bars: false,
            undo_script: None,
            test_checked: None,
            model_views: HashMap::new(),
            palette_categories: None,
            area_names: Default::default(),
            area_contents: Default::default(),
            area_views: HashMap::new(),
            adjust: None,
            find_instance: None,
            area_focus: None,
            start_focus: None,
            references: Default::default(),
            text_typed: None,
            command_after_text: None,
            script_stamps: Default::default(),
            compiled_dir: None,
            locks_dir: None,
            open_tabs_dir: None,
            open_tabs: Default::default(),
            shader_cache_dir: None,
            rename: None,
            copy_as: None,
            tag_clash: None,
            tag_cache: Default::default(),
            builder_code: None,
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
            advanced_search: Default::default(),
            search_kept: Default::default(),
            prefab_dir: None,
            prefab_deleted: None,
            var_set_dir: None,
            area_set_dir: None,
            area_holds: Default::default(),
            lighting_dir: None,
            reload_checked: None,
            tlk_stamp: None,
            paint: false,
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
            escape_cancel: None,
            tree_reveal: None,
            tree_area: None,
            tree_marked: None,
            tree_front: Default::default(),
            tree_object: None,
            tree_object_pending: false,
            tree_selected: Default::default(),
            tree_anchor: None,
            tree_kept: None,
            tree_objects: None,
            tree_objects_anchor: None,
            tree_seen_selection: None,
            area_select: None,
            tree_cursor: None,
            tree_cursor_moved: false,
            tree_fold: None,
            tree_to_match: false,
            import: None,
            confirm_discard: None,
            module_folder: None,
            claims: Default::default(),
            project_commands: Default::default(),
            closing: false,
            render_info: None,
            graphics_choice: None,
            drawing_with: None,
            outside: Default::default(),
            maximized: HashMap::new(),
            tab_buttons: Vec::new(),
            menu_keys: Default::default(),
            menu_row: [0, 0],
            menu_open: None,
            menu_alt_spoiled: false,
            area_hovered: false,
            tab_places: HashMap::new(),
            panes_tabs: Vec::new(),
            closed_tabs: Vec::new(),
            windows: HashMap::new(),
            dock_rect: None,
            confirm_delete: None,
            quit_requested: false,
            renderer_check: Default::default(),
            lit_check: Default::default(),
            relaunch: None,
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
            updates: Default::default(),
            screen: None,
            tree_width: None,
            dock_width: None,
            palette_share: None,
            palette_edge: None,
            palette_arrow: None,
            tab_back: false,
            open_palette: false,
            plugins: Default::default(),
            plugin_dir: None,
            plugin_answers: None,
            no_plugins: false,
            keymap_stale: false,
            command_palette: None,
            job: None,
            background_jobs: false,
        }
    }

    /// Whether the 3D views draw with the game's shaders: the Shaders
    /// switch, unless the lit check found that they draw nothing here
    /// ([`renderer_check::RendererCheck::game_shaders_off`]).
    pub fn game_shaders(&self) -> bool {
        self.settings.game_shaders() && self.renderer_check.game_shaders_off.is_none()
    }

    /// The Shaders switch turned to `on`: on, the game's shaders again,
    /// whatever the lit check found (the builder asked for them).
    pub fn set_game_shaders(&mut self, on: bool) {
        self.settings.game_shaders = Some(on);
        if on {
            self.renderer_check.game_shaders_off = None;
        }
        let shaders = self.game_shaders();
        if let Some(vp) = self.viewport.as_mut() {
            vp.renderer.game_shaders = shaders;
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
        let adapter = render_state.adapter.get_info();
        self.renderer_check.adapter = Some(adapter.clone());
        self.drawing_with = Some(format!(
            "{}, {}",
            mg_render::backend::backend_name(adapter.backend),
            adapter.name
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
        // (Pressed by the window it was for, in the frame it was asked.)
        self.escape_cancel = None;
        let female = self.settings.armor_on_woman;
        self.thumbnails.begin_frame(self.ws.as_ref().map(|ws| ws.revision()), female);
        widgets::install_fonts(ui.ctx());
        // (Ctrl+Insert, Shift+Delete and Shift+Insert as Copy, Cut and
        // Paste: first, before anything reads the keys.)
        widgets::clipboard_keys(ui.ctx());
        widgets::plain_pastes(ui.ctx());
        widgets::dialogs_begin(ui.ctx());
        widgets::text_begin(ui.ctx());
        // Models that broke what builds or poses them, last frame: left out
        // of what is drawn, and said so. (Only a window that draws makes
        // them, and the list is the process's: a test's window with no GPU
        // must not take another's.)
        let failures =
            if self.viewport.is_some() { mg_render::guard::take_failures() } else { Vec::new() };
        for failure in failures {
            self.log.error(format!(
                "A model could not be shown and is left out ({failure}). The module is unharmed; \
                 a report was written for the report of the problem (Help › User Manual, \
                 Troubleshooting)"
            ));
        }
        // The Shaders switch, for every 3D view (they share one renderer),
        // and the game's programs that couldn't be drawn, said once each.
        renderer_check::lit_check(self);
        let shaders = self.game_shaders();
        if let Some(vp) = self.viewport.as_mut() {
            vp.renderer.game_shaders = shaders;
            // (Once: the programs' cache folder, and a frame asked for when
            // the worker has made a pipeline.)
            if !vp.configured {
                vp.configured = true;
                vp.renderer.set_shader_cache(self.shader_cache_dir.clone());
                let ctx = ui.ctx().clone();
                vp.renderer.set_game_waker(move || ctx.request_repaint());
            }
            // (Forgotten with the programs, when the resources are read
            // again: told again if they still fail.)
            if vp.renderer.game_shader_failures().is_empty() {
                self.shader_failures_told.clear();
            }
            for (what, why) in vp.renderer.game_shader_failures() {
                if !self.shader_failures_told.contains(what) {
                    self.shader_failures_told.push(what.clone());
                    self.log.warn(format!(
                        "The game's shader {what} is drawn with Moonglow's own shader instead: \
                         {why}"
                    ));
                }
            }
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
        updates::frame(self, ui.ctx());
        self.next_plugin_event();
        // (Before the menus and the toolbar, which are drawn before the
        // editor: they see the keys it had.)
        script_view::undo_target(self, ui.ctx());
        // (A running job has the window to itself.)
        if !self.busy() {
            self.shortcuts(ui);
        }
        // (A menu open as the frame began: one of its entries may be
        // chosen in it, and close it.)
        let menu_was_open = egui::Popup::is_any_open(ui.ctx());
        let bars = egui::Panel::top("menu").show(ui, |ui| {
            commands::menu_bar(self, ui);
            // Buttons for the common commands, below the menu as in Aurora.
            commands::toolbar(self, ui);
        });
        // (On the way from a side editor to Edit › Undo or the toolbar's,
        // or with a menu open, Undo stays the editor's the pointer left.)
        self.pointer_on_bars = ui.rect_contains_pointer(bars.response.rect)
            || menu_was_open
            || egui::Popup::is_any_open(ui.ctx());
        if !self.pointer_on_bars {
            self.undo_before_bars = self.undo_under_pointer.clone();
        }
        // A newer release, told under the toolbar until dismissed.
        updates::notice_bar(self, ui);
        // A renderer that draws nothing (the startup check, the area
        // view's watch).
        renderer_check::notice_bar(self, ui);
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        // The log shrinks to a single line if the user wants it that small.
        let line = ui.text_style_height(&egui::TextStyle::Monospace);
        // The side panes leave the middle its room, however they were left
        // (their sizes are remembered between runs) and whatever is in
        // them: the log at most half the window's height, the module tree
        // at most two fifths of its width.
        let window = ui.ctx().content_rect().size();
        // Each pane beside the middle has a strip along the edge it faces
        // the middle by, with its button to fold it away (GitHub issue
        // 10); folded away (the button, View), the strip is all it leaves,
        // with the same button, in the same place, to bring it back.
        if !self.settings.hide_log {
            egui::Panel::bottom("log")
                .resizable(true)
                .default_size(120.0)
                .size_range((line + 6.0)..=(window.y * 0.5).max(line + 6.0))
                .show(ui, |ui| self.log_ui(ui));
        }
        self.pane_edge(ui, commands::Id::ViewLog, Side::Below);
        if self.ws.is_some() {
            if !self.settings.hide_tree {
                let tree = egui::Panel::left("tree")
                    .resizable(true)
                    .default_size(240.0)
                    .size_range(150.0..=(window.x * 0.4).max(150.0))
                    .show(ui, |ui| tree::module_tree(self, ui));
                self.tree_width = Some(tree.response.rect.width());
            }
            self.pane_edge(ui, commands::Id::ViewTree, Side::Left);
            // The palette beside a newly opened module, as in Aurora (not
            // when it was folded away).
            if std::mem::take(&mut self.open_palette)
                && self.game.is_some()
                && !self.settings.hide_palettes
            {
                self.actions.push(Action::OpenTab(Tab::Palette));
            }
            // The palettes' strip, folded away (shown, their pane is the
            // dock's, and the button is drawn on its edge after it:
            // `palettes_edge`): on the side they come back on.
            if self.settings.hide_palettes
                && self.game.is_some()
                && self.dock.find_tab(&Tab::Palette).is_none()
            {
                let side = match self.place_for(&Tab::Palette) {
                    Place::Split(Side::Left, _) => Side::Left,
                    _ => Side::Right,
                };
                self.pane_edge(ui, commands::Id::ViewPalettes, side);
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
            // (Set again by the side editors and the talk table's, if they
            // are drawn.)
            self.undo_under_pointer = None;
            self.undo_keys = None;
            self.open_models = self
                .dock
                .iter_all_tabs()
                .map(|(_, t)| t)
                .filter(|t| {
                    matches!(t, Tab::Model(_) | Tab::InstanceModel { .. } | Tab::ModelCopy { .. })
                })
                .cloned()
                .collect();
            self.panes_tabs = self.dock.iter_leaves().map(|(_, l)| l.tabs.clone()).collect();
            let mut dock = std::mem::replace(&mut self.dock, DockState::new(Vec::new()));
            // A window's frame without a margin: egui_dock makes a collapsed
            // window as tall as its tab bar, margin included, which left
            // half the bar (and half the arrow that opens it again).
            ui.spacing_mut().window_margin = egui::Margin::ZERO;
            self.pass_covered_arrow(ui.ctx());
            {
                let mut viewer = tabs::Viewer { app: self };
                DockArea::new(&mut dock).show_close_buttons(true).show_inside(ui, &mut viewer);
            }
            self.dock = dock;
            self.window_bars(ui);
            self.palettes_edge(ui);
        });
        trace::changed("panes", || self.panes());
        self.remember_window_sizes();
        self.remember_window_places();
        self.note_recent_tabs(ui.ctx());
        tree::follow_front_tab(self);
        self.backup_timer(ui);
        self.autosave_timer(ui);
        self.reload_timer(ui);
        recovery::window(self, ui);
        manual::about_window(self, ui.ctx());
        updates::window(self, ui.ctx());
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
        nui_view::windows(self, ui);
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
        tag_clash::window(self, ui.ctx());
        builder_code::window(self, ui.ctx());
        claims::windows(self, ui.ctx());
        plugin_events::window(self, ui.ctx());
        project_commands::window(self, ui.ctx());
        outside::window(self, ui.ctx());
        script_nav::rename_window(self, ui.ctx());
        prefabs::save_window(self, ui.ctx());
        palette_categories::window(self, ui.ctx());
        bulk::update_window(self, ui.ctx());
        area_props::chooser_window(self, ui.ctx());
        side_doc::closing_window(self, ui.ctx());
        tile_select::closing_window(self, ui.ctx());
        table_view::open_table_ui(self, ui.ctx());
        hak_view::adding_window(self, ui.ctx());
        model_view::export_window(self, ui.ctx());
        module_props::attach_window(self, ui.ctx());
        nwsync_view::window(self, ui.ctx());
        bulk::text_window(self, ui.ctx());
        levelup_view::window(self, ui.ctx());
        creature_wizard::window(self, ui.ctx());
        if let Some(report) = &self.hak_report {
            let mut open = true;
            crate::widgets::dialog(ui.ctx(), "Hak Pak Conflict Analysis", &mut open)
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
        plugins::project_window(self, ui.ctx());
        plugins::settings_window(self, ui.ctx());
        plugins::remove_window(self, ui.ctx());
        commands::palette_window(self, ui.ctx());
        jobs::window(self, ui.ctx());
        // The pictures the frame's views asked for and lack: a few now.
        model_view::make_thumbnails(self, ui.ctx());
        // (What a Properties window changed is kept while it is open.)
        let dock = &self.dock;
        self.properties_changed.retain(|_, changes| dock.find_tab(&changes.tab).is_some());
        // A Properties text field that was typed in and is not drawn any
        // more: set now, as it would have been when left.
        if let Some((id, drawn, typed)) = &self.held_text
            && *drawn != ui.ctx().cumulative_pass_nr()
        {
            if let Some(command) = typed.clone() {
                self.actions.push(Action::Apply(command));
            }
            self.buffers.remove(id);
            self.held_text = None;
        }
        if !self.actions.is_empty() {
            self.run_actions();
            // Show the result now, not at the next input event.
            ui.ctx().request_repaint();
        }
        self.open_tabs_frame(ui.ctx());
        widgets::dialogs_end(ui.ctx());
    }

    /// The debug log's start of a frame: once, the settings and the
    /// graphics adapter; then the window and what is under way, as they
    /// change.
    fn trace_frame(&mut self, ui: &egui::Ui) {
        trace::set(trace::wanted(!self.settings.no_debug_log));
        if !trace::on() {
            return;
        }
        trace::changed("graphics", || {
            let info = self.render_info.as_deref().unwrap_or("none");
            match &self.graphics_choice {
                Some(why) => format!("{info}; taken by {why}"),
                None => info.to_string(),
            }
        });
        trace::changed("settings", || {
            let s = &self.settings;
            format!(
                "game {:?}, user folder {:?} (this run {:?}), last area {}, areas by name {}, \
                 scale {:?}, still objects {}, unlit {}, plugins {:?}; install {:?}",
                s.game_root,
                s.user_dir,
                s.user_dir_this_run,
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
            _ => self.split_of(path.node).unwrap_or(Place::Middle),
        })
    }

    /// Splits the middle (the pane of the areas) at `side` for `tab`, the
    /// new pane `share` thousandths of the two, and gives its place; `None`
    /// with no middle to split.
    fn split_middle(&mut self, side: Side, share: u16, tab: &Tab) -> Option<egui_dock::NodePath> {
        let middle = |t: &Tab| matches!(t, Tab::Area(_) | Tab::NoArea);
        let at = (self.dock.iter_leaves())
            .find(|(p, leaf)| p.surface.is_main() && leaf.tabs.iter().any(middle))
            .map(|(p, _)| p.node)?;
        // (The fraction is the left or upper pane's, whichever is new.)
        let share = f32::from(share) / 1000.0;
        let main = self.dock.main_surface_mut();
        let [_, new] = match side {
            Side::Left => main.split_left(at, share, vec![tab.clone()]),
            Side::Right => main.split_right(at, 1.0 - share, vec![tab.clone()]),
            Side::Above => main.split_above(at, share, vec![tab.clone()]),
            Side::Below => main.split_below(at, 1.0 - share, vec![tab.clone()]),
        };
        Some(egui_dock::NodePath::new(egui_dock::SurfaceIndex::main(), new))
    }

    /// A pane of the main surface split off a side of the middle (the
    /// panes with the areas): which side, and how much of the two it is.
    fn split_of(&self, node: egui_dock::NodeIndex) -> Option<Place> {
        let parent = node.parent()?;
        let sibling = if node.is_left() { parent.right() } else { parent.left() };
        // (The other side holds the middle: a pane of the areas, or the
        // one that says none is open.)
        let middle = |t: &Tab| matches!(t, Tab::Area(_) | Tab::NoArea);
        let within = |mut n: egui_dock::NodeIndex| loop {
            if n == sibling {
                return true;
            }
            match n.parent() {
                Some(p) => n = p,
                None => return false,
            }
        };
        let holds = (self.dock.iter_leaves())
            .filter(|(p, _)| p.surface.is_main())
            .any(|(p, leaf)| leaf.tabs.iter().any(middle) && within(p.node));
        if !holds {
            return None;
        }
        let (side, share) = match &self.dock.main_surface()[parent] {
            egui_dock::Node::Horizontal(split) if node.is_left() => (Side::Left, split.fraction),
            egui_dock::Node::Horizontal(split) => (Side::Right, 1.0 - split.fraction),
            egui_dock::Node::Vertical(split) if node.is_left() => (Side::Above, split.fraction),
            egui_dock::Node::Vertical(split) => (Side::Below, 1.0 - split.fraction),
            _ => return None,
        };
        Some(Place::Split(side, (share.clamp(0.05, 0.95) * 1000.0).round() as u16))
    }

    /// Where the next window of a kind opens: where one of its kind was
    /// last dragged to (GitHub issue 14), a window of its own otherwise.
    fn place_for(&self, tab: &Tab) -> Place {
        let kept = kept_by_kind(&self.settings.window_places, tab);
        match kept.map(|(_, p)| p.as_str()) {
            Some("middle") => Place::Middle,
            Some("palettes") => Place::Palettes,
            Some(split) => {
                // "split left 300": the side, and thousandths of the two.
                let mut words = split.split_whitespace();
                let side = match (words.next(), words.next()) {
                    (Some("split"), Some("left")) => Side::Left,
                    (Some("split"), Some("right")) => Side::Right,
                    (Some("split"), Some("above")) => Side::Above,
                    (Some("split"), Some("below")) => Side::Below,
                    _ => return Place::Window,
                };
                match words.next().and_then(|w| w.parse::<u16>().ok()) {
                    Some(share) => Place::Split(side, share.clamp(50, 950)),
                    None => Place::Window,
                }
            }
            None => Place::Window,
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
        // The palettes' pane split off another side of the middle than its
        // right: kept, for when it opens again (on the right, as at first,
        // its width is kept as it was).
        if let Some(path) = self.dock.find_tab(&Tab::Palette).filter(|p| p.surface.is_main())
            && self.maximized.is_empty()
        {
            let side = match self.split_of(path.node) {
                Some(Place::Split(side, share)) if side != Side::Right => {
                    Some(format!("split {} {share}", side.name()))
                }
                _ => None,
            };
            let kind = Tab::Palette.kind();
            let kept = self.settings.window_places.iter().position(|(k, _)| k == kind);
            match (side, kept) {
                (Some(side), Some(i)) if self.settings.window_places[i].1 != side => {
                    self.settings.window_places[i].1 = side;
                }
                (Some(side), None) => self.settings.window_places.push((kind.to_string(), side)),
                (None, Some(i)) => _ = self.settings.window_places.remove(i),
                _ => {}
            }
        }
        // (Not while one is dragged, or made to fill the pane.)
        for tab in open {
            let Some(now) = self.place_of(&tab) else { continue };
            let before = self.tab_places.insert(tab.clone(), now);
            if before.is_some_and(|b| b != now) && !self.maximized.contains_key(&tab) {
                let kind = tab.kind();
                self.settings.window_places.retain(|(k, _)| k != kind);
                let name = match now {
                    Place::Middle => "middle".to_string(),
                    Place::Palettes => "palettes".to_string(),
                    Place::Split(side, share) => format!("split {} {share}", side.name()),
                    Place::Window => continue,
                };
                self.settings.window_places.push((kind.to_string(), name));
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

    /// The strip along the inner edge of the pane `panel` (View's command
    /// for it), a panel at the window's `side`: with the pane shown,
    /// between it and the middle; folded away, all that is left of it.
    /// Its button folds the pane away and brings it back.
    fn pane_edge(&mut self, ui: &mut egui::Ui, panel: commands::Id, side: Side) {
        let id = egui::Id::new(("pane-edge", panel.id()));
        let strip = match side {
            Side::Left => egui::Panel::left(id),
            Side::Right => egui::Panel::right(id),
            Side::Above => egui::Panel::top(id),
            Side::Below => egui::Panel::bottom(id),
        };
        let frame = egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::ZERO);
        // (With room for the line along its edge, which egui counts in.)
        let line = ui.visuals().widgets.noninteractive.bg_stroke.width.round();
        let size = widgets::PANE_EDGE + line;
        strip.exact_size(size).resizable(false).frame(frame).show(ui, |ui| {
            let rect = ui.max_rect();
            self.pane_fold(ui, panel, rect, side);
        });
    }

    /// The button that folds the pane `panel` away, or brings it back, in
    /// the strip `strip` along its inner edge, for a pane on the `side` of
    /// the middle: centered along the strip, and pointing the way the pane
    /// goes when it is clicked.
    fn pane_fold(&mut self, ui: &mut egui::Ui, panel: commands::Id, strip: egui::Rect, side: Side) {
        use widgets::Toward;
        let shown = self.panel_shown(panel);
        let toward = match (side, shown) {
            (Side::Left, true) | (Side::Right, false) => Toward::Left,
            (Side::Left, false) | (Side::Right, true) => Toward::Right,
            (Side::Above, true) | (Side::Below, false) => Toward::Up,
            (Side::Above, false) | (Side::Below, true) => Toward::Down,
        };
        let (name, what) = match panel {
            commands::Id::ViewTree => ("Module Tree", "the module tree"),
            commands::Id::ViewLog => ("Log", "the log"),
            _ => ("Palettes", "the palettes"),
        };
        let key = self.keymap.label_of(&panel.id(), ui.ctx());
        let key = if key.is_empty() { String::new() } else { format!(" ({key})") };
        let (label, tip) = if shown {
            (format!("Fold {name}"), format!("Fold {what} away{key}"))
        } else {
            (format!("Show {name}"), format!("Show {what}{key}"))
        };
        let rect = widgets::pane_fold_rect(strip, matches!(side, Side::Left | Side::Right));
        let id = egui::Id::new(("pane-fold", name));
        if widgets::pane_fold_button(ui, rect, id, toward, &label, &tip) {
            self.show_panel(panel, !shown);
        }
    }

    /// The keyboard on the palettes' covered arrow (reached by Tab or
    /// Shift+Tab, where Enter pressed it): moved on past it, the way it was
    /// going, before the dock draws its widgets again (egui gives the
    /// focus to the next widget as they ask for it).
    fn pass_covered_arrow(&mut self, ctx: &egui::Context) {
        let tab = ctx.input(|i| {
            i.events.iter().rev().find_map(|e| match e {
                egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } => {
                    Some(modifiers.shift)
                }
                _ => None,
            })
        });
        if let Some(back) = tab {
            self.tab_back = back;
        }
        let Some(arrow) = self.palette_arrow else { return };
        let on_arrow = ctx.memory(|m| m.focused()).is_some_and(|id| {
            ctx.read_response(id).is_some_and(|r| arrow.contains(r.rect.center()))
        });
        if on_arrow {
            let way = if self.tab_back {
                egui::FocusDirection::Previous
            } else {
                egui::FocusDirection::Next
            };
            ctx.memory_mut(|m| m.move_focus(way));
            ctx.request_repaint();
        }
    }

    /// The palettes' pane's fold button, on the edge it faces the middle
    /// by, once the dock has drawn it (a pane of the dock's, not a panel:
    /// its tabs leave the room for it, `palette_edge`). Only where it is a
    /// pane at a side of the main window's middle: joined to the middle's
    /// tabs, in a window of its own, above or below it, View folds it away.
    fn palettes_edge(&mut self, ui: &mut egui::Ui) {
        self.palette_edge = None;
        self.palette_arrow = None;
        let Some(dock) = self.dock_rect else { return };
        let leaf = self.dock.iter_leaves().find(|(p, leaf)| {
            p.surface.is_main()
                && leaf.tabs.contains(&Tab::Palette)
                && !leaf.collapsed
                && !leaf.tabs.iter().any(|t| matches!(t, Tab::Area(_)))
        });
        let Some((_, leaf)) = leaf else { return };
        let (rect, bar) = (leaf.rect, leaf.viewport.top());
        let side = if rect.left() > dock.left() + 1.0 && rect.right() >= dock.right() - 1.0 {
            Side::Right
        } else if rect.left() <= dock.left() + 1.0 && rect.right() < dock.right() - 1.0 {
            Side::Left
        } else {
            return;
        };
        self.palette_edge = Some((side, leaf.tabs.clone()));
        // egui_dock's arrow at the left of the pane's tab bar, which folds
        // the pane to its bar, was a second fold button beside the pane's
        // own (the reporter of issue 10): covered, and its clicks taken.
        // (egui_dock has it on every pane or none; windows keep theirs, to
        // fold to their bar.)
        let arrow = egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.left() + 24.0, bar));
        if arrow.is_positive() {
            let px = ui.ctx().pixels_per_point().recip();
            let visuals = ui.visuals();
            let corner = egui::CornerRadius {
                nw: visuals.widgets.inactive.corner_radius.nw.saturating_add(2),
                ..Default::default()
            };
            let fill = visuals.extreme_bg_color;
            let cover = arrow.with_max_y(arrow.bottom() - px).expand2(egui::vec2(px, 0.0));
            ui.painter().rect_filled(cover, corner, fill);
            ui.interact(arrow, egui::Id::new("palettes-bar-arrow"), egui::Sense::CLICK);
            self.palette_arrow = Some(arrow);
        }
        let x = match side {
            Side::Right => rect.left(),
            _ => rect.right() - widgets::PANE_EDGE,
        };
        let strip = egui::Rect::from_x_y_ranges(x..=x + widgets::PANE_EDGE, rect.y_range());
        self.pane_fold(ui, commands::Id::ViewPalettes, strip, side);
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

    /// Whether a tab may close now, letting go of what it held: a side
    /// editor's (a hak, a 2DA, a tileset) with unsaved changes asks first,
    /// and stays for the answer.
    pub(crate) fn may_close(&mut self, tab: &Tab) -> bool {
        // (Closed, its draft goes: Cancel.)
        if *tab == Tab::Options {
            self.options = None;
        }
        // (Tile Properties with changes not applied asks first, and stays
        // for the answer: `tile_select::closing_window`.)
        if *tab == Tab::TileProperties {
            match self.tile_props.as_mut() {
                Some(p) if p.changed() => {
                    p.closing = true;
                    return false;
                }
                _ => self.tile_props = None,
            }
        }
        self.may_close_side(tab)
    }

    /// Notes a tab closed, for Reopen Closed Tab: those that show a
    /// resource or a tool of the module, not a side editor's (a hak's, a
    /// 2DA's, a tileset's: closed, it has let its file go).
    pub(crate) fn closed(&mut self, tab: &Tab) {
        if side_doc::is_side(tab) || matches!(tab, Tab::Options | Tab::TileProperties) {
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
    pub fn front_tab(&self) -> Option<Tab> {
        let pane = self.dock.focused_leaf().or_else(|| self.tab_pane())?;
        let leaf = self.dock.leaf(pane).ok()?;
        leaf.tabs.get(leaf.active.0).filter(|t| t.closeable()).cloned()
    }

    /// The tab in front ([`Moonglow::front_tab`]) where it is in a window
    /// of its own: what Maximize Window maximizes.
    pub(crate) fn front_window_tab(&self) -> Option<Tab> {
        let tab = self.front_tab()?;
        let path = self.dock.find_tab(&tab)?;
        (!path.surface.is_main()).then_some(tab)
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
        if !self.settings.tabs_by_recent {
            let next = (leaf.active.0 as isize + by).rem_euclid(n) as usize;
            self.show_tab(leaf.tabs[next].clone());
            return;
        }
        // By when they were last used (Options › General): the order as
        // it was when the keys began, gone through for as long as Ctrl is
        // held ([`Moonglow::note_recent_tabs`] ends it).
        let (order, at) = self.tab_cycle.take().unwrap_or_else(|| {
            let mut order: Vec<Tab> =
                self.tabs_recent.iter().filter(|t| leaf.tabs.contains(t)).cloned().collect();
            order.extend(leaf.tabs.iter().filter(|t| !self.tabs_recent.contains(t)).cloned());
            let at = leaf.tabs.get(leaf.active.0).and_then(|t| order.iter().position(|o| o == t));
            (order, at.unwrap_or(0))
        });
        let at = (at as isize + by).rem_euclid(order.len() as isize) as usize;
        let tab = order[at].clone();
        self.tab_cycle = Some((order, at));
        self.show_tab(tab);
    }

    /// Next Pane and Previous Pane (F6): the arrow keys given to the next
    /// of the panes shown, round from the last: the module tree (at the
    /// row they were at, else its first), the middle (an area's view has
    /// them, as after a click in it), the palettes.
    pub(crate) fn step_pane(&mut self, ctx: &egui::Context, by: isize) {
        use palette_view::Keys;
        // Each pane's keys are what its tab in front has: the middle's (the
        // tab used last outside the palettes' pane, a window's too) a talk
        // table's lines, a table's cells or the resource browser's list,
        // else an area's; the palettes' pane's the palette, or the
        // resource browser docked there.
        let palettes = self.dock.find_tab(&Tab::Palette).map(|p| (p.surface, p.node));
        let in_palettes = |tab: &Tab| {
            let at = self.dock.find_tab(tab).map(|p| (p.surface, p.node));
            at.is_some() && at == palettes
        };
        let front = |in_pane: bool| {
            if in_pane {
                let (s, n) = palettes?;
                let leaf = self.dock.iter_leaves().find(|(p, _)| p.surface == s && p.node == n)?.1;
                leaf.tabs.get(leaf.active.0).cloned()
            } else {
                self.tabs_recent.iter().find(|t| !in_palettes(t)).cloned()
            }
        };
        let keys_of = |tab: Option<Tab>| match tab {
            Some(Tab::TalkTable) => (Keys::Talk, None),
            Some(Tab::Table(id)) => (Keys::Table, Some(id)),
            Some(Tab::Resources) => (Keys::Browser, None),
            _ => (Keys::Area, None),
        };
        let mut panes = Vec::new();
        if self.ws.is_some() && !self.settings.hide_tree {
            panes.push((Keys::Tree, None));
        }
        panes.push(keys_of(front(false)));
        if palettes.is_some() {
            panes.push(match front(true) {
                Some(Tab::Resources) => (Keys::Browser, None),
                _ => (Keys::Palette, None),
            });
        }
        let now = panes.iter().position(|p| palette_view::keys_are(ctx, p.0));
        // (The keys anywhere else, a conversation's lines: as the middle's.)
        let middle = usize::from(panes[0].0 == Keys::Tree);
        let now = now.unwrap_or(middle.min(panes.len() - 1));
        let (next, table) = panes[(now as isize + by).rem_euclid(panes.len() as isize) as usize];
        palette_view::give_keys(ctx, next);
        if let Some(id) = table {
            table_view::take_keys(ctx, id);
            // (At its first cell, where none is selected: the arrows go
            // from a cell.)
            if let Some(doc) = self.tables.iter_mut().find(|d| d.id == id)
                && doc.selected.is_none()
                && !doc.edit.table().is_empty()
            {
                doc.selected = Some((0, 0));
            }
        }
        if next == Keys::Tree && self.tree_cursor.is_none() {
            self.tree_cursor = Some(tree::first_row());
            self.tree_cursor_moved = true;
        }
        if next == Keys::Palette {
            self.palette.to_first = true;
        }
    }

    /// Notes the tab in front of each pane (the middle's, a window's) as
    /// the pane's one used last, each frame; not while Next Tab is going
    /// through the tabs with Ctrl held, which ends when it is let go (the
    /// tab arrived at is then the one used last). One list for every
    /// pane: a pane's order is its tabs' in it ([`Moonglow::step_tab`]).
    fn note_recent_tabs(&mut self, ctx: &egui::Context) {
        if self.tab_cycle.is_some() {
            if ctx.input(|i| i.modifiers.command || i.modifiers.ctrl) {
                return;
            }
            self.tab_cycle = None;
        }
        let fronts: Vec<(Tab, Vec<Tab>)> = (self.dock.iter_leaves())
            .filter_map(|(_, leaf)| {
                Some((leaf.tabs.get(leaf.active.0)?.clone(), leaf.tabs.clone()))
            })
            .collect();
        for (front, tabs) in fronts {
            // (Its pane's first in the list already: nothing changes, and
            // the panes' fronts don't push each other down, frame by
            // frame.)
            if self.tabs_recent.iter().find(|t| tabs.contains(t)) == Some(&front) {
                continue;
            }
            self.tabs_recent.retain(|t| *t != front);
            self.tabs_recent.insert(0, front);
            // (Closed ones go as the list is looked through; not kept
            // without end.)
            self.tabs_recent.truncate(64);
        }
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
        // (The palettes dropped into the middle, among its areas, make it
        // no side pane: taken for one, the middle was found empty and a
        // pane that says so was set beside it, the areas on the right.
        // GitHub issue 20.)
        let side = (self.dock.find_tab(&Tab::Palette))
            .filter(|p| p.surface.is_main())
            .filter(|p| {
                let middle = |t: &Tab| matches!(t, Tab::Area(_) | Tab::NoArea);
                self.dock
                    .iter_leaves()
                    .find(|(q, _)| q.surface == p.surface && q.node == p.node)
                    .is_none_or(|(_, leaf)| !leaf.tabs.iter().any(middle))
            })
            .map(|p| (p.surface, p.node));
        // (Without the palettes, a pane of the resource browser alone is
        // the side pane all the same: the palettes closed, it took the
        // whole window, and nothing could be docked beside it. GitHub
        // issue 23.)
        let side = side.or_else(|| {
            let tools = |t: &Tab| matches!(t, Tab::Resources);
            (self.dock.iter_leaves())
                .filter(|(p, _)| p.surface.is_main())
                .find(|(_, leaf)| !leaf.tabs.is_empty() && leaf.tabs.iter().all(tools))
                .map(|(p, _)| (p.surface, p.node))
        });
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
            match side {
                Some((_, node)) => {
                    let left = match self.palette_share {
                        Some(share) if share < 0.95 => (1.0 - share).clamp(0.12, 0.88),
                        _ => self.palette_split(),
                    };
                    self.dock.main_surface_mut().split_left(node, left, vec![Tab::NoArea]);
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
        let left = kept_by_kind(&self.settings.window_sizes, &tab);
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
    /// offers that; a button at its right end before the close button
    /// does it too, and shows which the window is (🗖 Maximize, 🗗
    /// Restore). (egui_dock gives the bar nothing; a builder looked for
    /// them there, as on any window's title.) The window is still dragged
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
            let maximized = self.windows_maximized(&leaf.tabs);
            let label = if maximized { "Restore" } else { "Maximize" };
            // (Registered after the bar: it takes its clicks.)
            let glyph = if maximized { icons::RESTORE } else { icons::MAXIMIZE };
            let side = (bar.height() - 4.0).clamp(12.0, 20.0);
            let at = egui::Rect::from_center_size(
                egui::pos2(bar.right() - side / 2.0 - 2.0, bar.center().y),
                egui::vec2(side, side),
            );
            let button = egui::Button::new(glyph).frame(false).small();
            let pressed = ui
                .scope_builder(egui::UiBuilder::new().layer_id(layer).max_rect(at), |ui| {
                    let r = ui.put(at, button);
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label)
                    });
                    r.on_hover_text(label).clicked()
                })
                .inner;
            if pressed {
                toggle = Some(tab.clone());
            }
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

    /// Whether the window of these tabs (one of its panes') is maximized:
    /// it is known by its first tab.
    fn windows_maximized(&self, tabs: &[Tab]) -> bool {
        let Some(path) = tabs.first().and_then(|t| self.dock.find_tab(t)) else { return false };
        (self.dock.iter_leaves())
            .find(|(p, _)| p.surface == path.surface)
            .and_then(|(_, leaf)| leaf.tabs.first())
            .is_some_and(|first| self.maximized.contains_key(first))
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
        // A maximized window stays so as Moonglow's window is resized:
        // put over the room again where it changed.
        if let Some(room) = self.maximize_room() {
            let moved = |put: egui::Rect| {
                (put.min - room.min).length() > 0.5 || (put.max - room.max).length() > 0.5
            };
            for tab in self.maximized.keys() {
                if self.windows.get(tab).is_some_and(|w| moved(w.put)) {
                    again.push((tab.clone(), room));
                }
            }
        }
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
        // (And the open project's commands' own keys, as they read: a
        // project opened, or its file changed, takes the keys anew.)
        let project = project_commands::commands(self).map(|(_, c)| c).unwrap_or_default();
        let project_keys: Vec<(String, Option<String>)> =
            project.iter().map(|c| (c.name.clone(), c.key.clone())).collect();
        if std::mem::take(&mut self.keymap_stale)
            || self.keymap_from != self.settings.key_bindings
            || self.project_keys_from != project_keys
        {
            self.keymap = keys::Keymap::new(&self.settings.key_bindings);
            let commands = self.plugin_commands();
            plugins::register_keys(&mut self.keymap, &commands);
            project_commands::register_keys(&mut self.keymap, &project);
            self.keymap_from = self.settings.key_bindings.clone();
            self.project_keys_from = project_keys;
        }
        // (Find's keys, for the search fields.)
        self.keymap.share_find(ui.ctx());
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
                if let Some(note) = self.looking_note() {
                    ui.label(note);
                }
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
                let mut clicked = None;
                // Only the lines in sight are laid out (a Verify of a large
                // module says thousands): the others take the room they
                // took when last drawn, or a line's if never, together.
                // (The heights stay when the width changes: those in sight
                // are measured again as they are drawn, and starting over
                // made the scroll bar come and go, each frame the width.)
                let width = ui.available_width();
                let mut heights = std::mem::take(&mut self.log.rows);
                heights.resize(self.log.entries.len(), 0.0);
                // (A line never drawn: as tall as the last one drawn, which
                // a line of one row is; its room is then right when it is
                // drawn, and the list doesn't move.)
                let line =
                    (heights.iter().rev().find(|h| **h > 0.0).copied()).unwrap_or_else(|| {
                        ui.text_style_height(&egui::TextStyle::Monospace)
                            + ui.spacing().item_spacing.y
                    });
                let mut skipped = 0.0;
                for (i, (level, _)) in self.log.entries.iter().enumerate() {
                    let known = if heights[i] > 0.0 { heights[i] } else { line };
                    let top = ui.cursor().top() + skipped;
                    let room = egui::Rect::from_min_size(
                        egui::pos2(ui.cursor().left(), top),
                        egui::vec2(width, known),
                    );
                    if !ui.is_rect_visible(room) {
                        skipped += known;
                        continue;
                    }
                    if skipped > 0.0 {
                        ui.add_space(std::mem::take(&mut skipped));
                    }
                    let before = ui.cursor().top();
                    // (When it was said, before it: the Compiler tab's way.)
                    let text = self.log.shown(i);
                    let text = egui::RichText::new(text).color(level.color(ui)).monospace();
                    // A finding's fix: a button beside it, made when
                    // clicked (Verify Module).
                    let Some(offer) = self.log.fix_at(i) else {
                        ui.label(text);
                        heights[i] = ui.cursor().top() - before;
                        continue;
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(text);
                        if offer.made {
                            ui.add_enabled(false, egui::Button::new("✔ Fixed").small())
                                .on_disabled_hover_text(&offer.fix.label);
                            return;
                        }
                        let button = ui.add(egui::Button::new(&offer.fix.label).small());
                        let resources: Vec<String> =
                            offer.fix.resources().iter().map(ToString::to_string).collect();
                        let hint = format!("Changes {} (one step to undo)", resources.join(", "));
                        if button.on_hover_text(hint).clicked() {
                            clicked = Some(i);
                        }
                    });
                    heights[i] = ui.cursor().top() - before;
                }
                if skipped > 0.0 {
                    ui.add_space(skipped);
                }
                self.log.rows = heights;
                if let Some(line) = clicked {
                    self.actions.push(Action::ApplyFix(line));
                }
            });
        // Buttons in its corner: the whole log copied or saved (a line
        // selected is copied alone: Jasperre's report), and emptied, so
        // that a second Verify's report isn't mistaken for the first's.
        // (The one that folds it away is on its edge: `pane_edge`.)
        let mut right = ui.max_rect().right() - 4.0;
        let top = ui.max_rect().top() + 2.0;
        let buttons = [
            ("Clear", "Clear Log", "Empty the log (View › Clear Log)", 44.0),
            ("Save…", "Save Log", "The whole log into a text file (View › Save Log…)", 48.0),
            ("Copy", "Copy Log", "The whole log to the clipboard (View › Copy Log)", 44.0),
        ];
        for (text, name, tip, width) in buttons {
            let at =
                egui::Rect::from_min_size(egui::pos2(right - width, top), egui::vec2(width, 18.0));
            right -= width + 4.0;
            let enabled = !self.log.entries.is_empty();
            let b = ui.put(at, egui::Button::new(text).small());
            b.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name));
            if b.on_hover_text(tip).clicked() && enabled {
                match name {
                    "Clear Log" => self.log.clear(),
                    "Save Log" => self.save_log(),
                    _ => self.copy_log(ui.ctx()),
                }
            }
        }
    }

    /// Copy Log: the whole log to the clipboard, each line with its time.
    pub(crate) fn copy_log(&mut self, ctx: &egui::Context) {
        ctx.copy_text(self.log.text());
    }

    /// Save Log…: the whole log into a text file, where the builder says.
    pub(crate) fn save_log(&mut self) {
        let dir = self.module_path().and_then(|p| p.parent().map(|d| d.to_path_buf()));
        let suggested = dir.map(|d| d.join("moonglow-log.txt"));
        let Some(path) = self.dialogs.save_file(FileKind::Log, suggested.as_deref()) else {
            return;
        };
        match std::fs::write(&path, self.log.text()) {
            Ok(()) => self.log.info(format!("Saved the log to {}", path.display())),
            Err(e) => self.log.error(format!("Could not write {}: {e}", path.display())),
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
                self.log_project_file(false, false);
                let path = remembered.as_deref().unwrap_or(path);
                self.settings.remember(path);
                // The area opened last in it, if it still has it; else the
                // first the module tree lists (remembered from then on).
                let last =
                    self.settings.last_area(path).and_then(|a| a.parse::<mg_core::ResRef>().ok());
                let by_name = self.settings.area_names;
                // (Not when the tabs it had open are opened again.)
                let reopened = self.read_open_tabs();
                if let Some(ws) =
                    self.ws.as_ref().filter(|_| !self.settings.no_last_area && !reopened)
                {
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
        self.release_game_data();
        if let (Some(game), Some(gi)) = (exclusive(&mut self.game), &self.install) {
            let (missing, wrong) = m.stack_onto(&mut game.resman, gi);
            for h in missing {
                self.log.warn(format!("Hak {h} not found"));
            }
            if let Some(e) = wrong {
                self.log.error(format!("Could not open the module's haks: {e}"));
            }
            game.invalidate();
            text::set_game_codepage(Some(game.codepage()));
            self.haks_layered = m.haks().unwrap_or_default();
        } else if self.game.is_some() && self.install.is_some() {
            // (Something still reads the game data: said, rather than a
            // module opened without its haks and nothing to show why.)
            self.log.error(
                "The module's haks could not be added to the game data (it is in use): its \
                 custom content will be missing. Close and open the module again.",
            );
        }
        let mut m = m;
        self.restore_locks(&mut m);
        self.ws = Some(Workspace::new(m));
        // (The last module's tags, and the bytes they were read from, let go.)
        self.tag_cache = Default::default();
        // (The project's plugins first: those allowed act on its opening.)
        self.load_project_plugins();
        self.plugins_module_opened();
        self.outside = Default::default();
        self.area_names = Default::default();
        self.area_contents.clear();
        self.dock = DockState::new(vec![Tab::ModuleProperties]);
        self.open_palette = true;
        // On the area's tiles, what a new module needs first; or on the
        // palette shown last, where one was chosen.
        if self.settings.palette_type.is_none() {
            self.palette.tiles = true;
        }
        self.custom_tlk = None;
        self.load_custom_tlk();
        self.load_order_changed();
    }

    /// The game data's layers changed: the browser and the read-only
    /// viewers show them anew.
    fn load_order_changed(&mut self) {
        self.nui_assets.invalidate();
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
        self.release_game_data();
        let (Some(ws), Some(game)) = (&self.ws, exclusive(&mut self.game)) else { return };
        let name = ws.module.custom_tlk().ok().flatten().filter(|n| !n.trim().is_empty());
        if name == self.custom_tlk {
            return;
        }
        self.custom_tlk = name.clone();
        // (Blueprints' names kept by the editors, and what Advanced Search
        // found, may come from it.)
        self.palette.kept = Default::default();
        self.search_kept = Default::default();
        let Some(name) = name else {
            game.set_custom_tlk(None);
            return;
        };
        let dirs = self.install.as_ref().map(|i| i.tlk_dirs()).unwrap_or_default();
        // (A nasher project's own, kept as JSON, before the tlk folder's.)
        let found =
            mg_module::talk::find_for(&ws.module, &game.resman, &dirs, &name, Some(game.language));
        let file = found.as_ref().ok().and_then(Option::as_ref).and_then(|f| f.source.file());
        let stamp = file.and_then(|path| {
            let t = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
            Some((path.to_path_buf(), t))
        });
        self.tlk_stamp = stamp;
        match found.map(|f| f.map(|f| f.tlk())).unwrap_or_else(|e| Some(Err(e))) {
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
            || self.side_unsaved()
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
        // (With what the plugins do on a new area, in the same command.)
        let made = plugin_events::Made::resources("wizard", vec![ResKey::new(area, ResType::ARE)]);
        let cmd = plugin_events::with_plugins(
            self,
            Command::new(format!("New area {area}"), edits),
            &made,
        );
        match self.apply(cmd) {
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
        // Its tabs, for the next time it is opened.
        self.keep_open_tabs();
        self.open_tabs = Default::default();
        // Closed (saved, or its changes discarded): no recovery copy.
        self.forget_recovery();
        self.ws = None;
        self.tag_cache = Default::default();
        // (The project's plugins were its own.)
        self.plugins.settings_form = None;
        self.load_project_plugins();
        // (Another module's blueprints of these names look different.)
        self.thumbnails.forget(self.viewport.as_ref());
        self.scripts.clear();
        // (The module's own 2DAs too: kept, the next module's Save would
        // write them into it, and it would look changed from the start.)
        self.tables.retain(|d| d.home != table_view::Home::Module);
        // (`nwscript.nss` too: Options' reload closes the module, and
        // another install has its own.)
        self.script_tools.spec = None;
        self.script_tools.symbols.clear();
        self.script_tools.seen.clear();
        self.buffers.clear();
        self.properties_changed.clear();
        self.talk = None;
        self.talk_view = Default::default();
        // (Its views, kept by name: another module's area, conversation or
        // window of the same name is its own; Jasperre's report.)
        self.area_views.clear();
        self.area_contents.clear();
        self.dialog_views.clear();
        self.nui_views.clear();
        self.nui_build_error = None;
        self.new_nui = None;
        self.laid_out.clear();
        self.tile_props = None;
        self.area_stats = None;
        self.resize_area = None;
        self.rotate_area = None;
        self.adjust = None;
        self.find_instance = None;
        self.area_focus = None;
        self.start_focus = None;
        self.heard = None;
        self.tree_area = None;
        self.tree_marked = None;
        self.tree_object = None;
        self.tree_selected.clear();
        self.tree_anchor = None;
        self.tree_kept = None;
        self.tree_objects = None;
        self.tree_objects_anchor = None;
        self.tree_seen_selection = None;
        self.area_select = None;
        self.dock = DockState::new(vec![Tab::Welcome]);
        self.load_order_changed();
        // (Another module's haks may have other tilesets of these names.)
        self.palette.forget_game_data();
        self.search_kept = Default::default();
        self.release_game_data();
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
        self.release_game_data();
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

    /// The game data's module layer with `keys` as the module has them
    /// now, changed in place: built again whole, a large module's layer
    /// took milliseconds of a frame for a few files read again
    /// ([`crate::outside`]). The game data's tables are forgotten only
    /// where a 2DA is among them.
    pub(crate) fn update_module_layer(&mut self, keys: &[ResKey]) {
        self.release_game_data();
        let (Some(game), Some(ws)) = (exclusive(&mut self.game), &mut self.ws) else { return };
        if ws.flush().is_err() {
            return;
        }
        let layer = game.resman.layer_mut("module").and_then(|l| l.container.as_mem_mut());
        let Some(layer) = layer else {
            self.refresh_module_layer();
            return;
        };
        for k in keys {
            match ws.module.shared(k) {
                Some(data) => layer.insert(*k, data.clone()),
                None => {
                    layer.remove(k);
                }
            }
        }
        if keys.iter().any(|k| k.restype == ResType::TWODA) {
            game.invalidate();
            text::set_game_codepage(Some(game.codepage()));
        }
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
        // A protected area's ResRef about to change: asked first.
        if claims::asks_first(self, &action) {
            return;
        }
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
                    | Action::RestartGraphics(_)
                    | Action::ProjectCommand(_)
            ),
        };
        // The window's X: with unsaved work, the question Exit asks; with
        // none, one of its own, so that a slip of the pointer does not end
        // a session of open areas and scripts.
        let action = match action {
            Action::CloseWindow if self.has_unsaved_work() => {
                self.confirm_discard = Some(Action::Quit);
                return;
            }
            Action::CloseWindow if self.settings.close_without_asking => Action::Quit,
            other => other,
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
                // (Areas not this builder's: asked first.)
                if claims::hold_save(self, Action::SaveThen(next.clone())) {
                    return;
                }
                // (And what the plugins say of what it writes.)
                if plugin_events::hold_save(self, Action::SaveThen(next.clone())) {
                    return;
                }
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
                    module_folder::open(self, &p);
                }
            }
            Action::OpenModule(p) => module_folder::open(self, &p),
            Action::OpenModuleAsIs(p) => self.open_module(&p),
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
            Action::FoldPalettes => self.show_panel(commands::Id::ViewPalettes, false),
            Action::PackTarget => _ = self.pack_target(),
            Action::TakeInLeftOut => self.take_in_left_out(),
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
            Action::ExportAreaFiles(area) => model_view::export_area_files(self, area),
            Action::ExportObject { area, kind, index } => self.run_export_object(area, kind, index),
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
                // The enabled plugins' checks, for Options › Verify.
                draft.plugin_checks = (self.enabled_plugins().into_iter())
                    .flat_map(|p| {
                        (p.manifest.checks.iter())
                            .map(|c| {
                                (p.check_id(&c.id), format!("{}: {}", p.manifest.name, c.title))
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();
                // And the project's.
                let project = project_commands::commands(self).map(|(_, c)| c);
                project_commands::register_keys(&mut draft.keymap, &project.unwrap_or_default());
                draft.project_commands = project_commands::key_rows(self);
                self.options = Some(draft);
            }
            Action::ApplyOptions(draft) => self.apply_options(*draft),
            Action::Save => {
                if !claims::hold_save(self, Action::Save)
                    && !plugin_events::hold_save(self, Action::Save)
                {
                    self.save(None)
                }
            }
            Action::Claim(area) => claims::claim(self, area, false),
            Action::ReleaseClaim(area) => claims::claim(self, area, true),
            Action::ProjectCommand(name) => project_commands::run(self, &name),
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
                // (Over a hak's, a 2DA's, a tileset's or the talk table's
                // editor, its own.)
                if self.local_undo(action == Action::Redo) {
                    return;
                }
                // (A Ctrl + wheel scaling under way is what Undo takes
                // back: it is not a command yet.)
                if action == Action::Undo && area_view::drop_wheel_scale(self) {
                    return;
                }
                let Some(ws) = &mut self.ws else { return };
                let next = if action == Action::Undo { ws.peek_undo() } else { ws.peek_redo() };
                // (Where what the open Properties windows show goes.)
                let edits =
                    next.map(|c| blueprint::follow::list_edits(&c.edits)).unwrap_or_default();
                let followed = blueprint::follow::followed(self, &edits);
                let Some(ws) = &mut self.ws else { return };
                let r = if action == Action::Undo { ws.undo() } else { ws.redo() };
                match r {
                    Ok(Some(label)) => {
                        blueprint::follow::retarget(self, followed);
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
                // (One held for an answer says so in its window.)
                if let Err(e) = self.apply(cmd)
                    && !std::mem::take(&mut self.claims.held)
                {
                    self.log.error(e.to_string());
                }
            }
            Action::ApplyMade(cmd, made) => plugin_events::apply_made(self, cmd, &made),
            Action::ApplyFix(line) => self.apply_fix(line),
            Action::NewNui(name) => nui_view::create(self, &name),
            Action::GenerateNui { name, export } => nui_view::generate(self, &name, export),
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
                if let Tab::Dialog(key) = &tab {
                    self.dialog_keys = Some(*key);
                }
                // The area opened last is opened again with the module.
                if let (Tab::Area(area), Some(module)) = (&tab, self.module_path()) {
                    self.settings.remember_area(&module, &area.to_string());
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
                        // (Where it was split off another side of the
                        // middle: there again. GitHub issue 22.)
                        let split = match self.place_for(&tab) {
                            Place::Split(side, share) => self.split_middle(side, share, &tab),
                            _ => None,
                        };
                        let main = self.dock.main_surface_mut();
                        if split.is_none() && main.root_node().is_none_or(|n| n.is_empty()) {
                            *main = egui_dock::Tree::new(vec![tab.clone()]);
                        } else if split.is_none() {
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
                            // A pane of its own off that side of the middle,
                            // as large as it was (GitHub issue 22).
                            Place::Split(side, share) => self.split_middle(side, share, &tab),
                        };
                        // A script, with a script's window open: a tab of
                        // that window, not one more window (Options ›
                        // Script Editor).
                        // (And so a Properties window and a
                        // conversation, where their options say so.)
                        let kind = tab.gathers().filter(|kind| match kind {
                            0 => !self.settings.scripts_own_windows,
                            1 => !self.settings.properties_own_windows,
                            2 => !self.settings.dialogs_own_windows,
                            // (NUI windows always.)
                            _ => true,
                        });
                        let beside = kind
                            .and_then(|kind| {
                                self.dock.iter_leaves().find(|(p, leaf)| {
                                    !p.surface.is_main()
                                        && leaf.tabs.iter().any(|t| t.gathers() == Some(kind))
                                })
                            })
                            .map(|(p, _)| p);
                        match (pane, beside) {
                            // (Split off for it: there already.)
                            (Some(_), _) if self.dock.find_tab(&tab).is_some() => {
                                self.tab_places.remove(&tab);
                            }
                            (Some(pane), _) => {
                                self.dock.set_focused_node_and_surface(pane);
                                self.dock.push_to_focused_leaf(tab.clone());
                                self.tab_places.remove(&tab);
                            }
                            (None, Some(window)) => {
                                self.dock.set_focused_node_and_surface(window);
                                self.dock.push_to_focused_leaf(tab.clone());
                            }
                            (None, None) => self.open_window(tab.clone()),
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
            Action::RefreshPalette(kind) => palette_view::refresh(self, kind),
            Action::EditScript { name, condition } => self.edit_script(name, condition),
            Action::DeleteDialog(k) => self.confirm_delete = Some(vec![k]),
            Action::DeleteChosenDialog(keys) => self.confirm_delete = Some(keys),
            Action::DeleteResource(k) => self.delete_resources(&[k]),
            Action::DeleteResources(keys) => self.delete_resources(&keys),
            Action::CompileScripts => self.compile_scripts(),
            Action::CompileModels => self.compile_models(),
            Action::Verify => self.verify(),
            Action::RoundTrip => self.round_trip(),
            Action::TestModule => self.test_module(false),
            Action::TestModuleChoose => self.test_module(true),
            Action::TestFromHere { area, at, facing, choose } => {
                self.test_from_here(area, at, facing, choose)
            }
            Action::ReloadResources => self.reload_resources(true),
            Action::CompileIncluders(key) => self.compile_includers(key),
            Action::NoteCompiled(made) => self.note_compiled(&made),
            Action::SaveScript(key) => {
                script_view::save_script(self, key, script_view::ScriptSave::Save);
            }
            Action::ScriptToScratch { key, compiled, debug } => {
                self.script_to_scratch(key, compiled, debug)
            }
            Action::SaveTalkTable => {
                talk_view::save(self);
            }
            Action::ExportMinimap(area) => self.export_minimap(area),
            Action::StaticPlaceables(areas) => self.static_placeables(&areas),
            Action::RunPluginCommand { plugin, command, selected } => {
                self.run_plugin_command_on(&plugin, &command, selected);
            }
            Action::DynamicPlaceables(areas) => self.dynamic_placeables(&areas),
            Action::TalkCsv(import) => talk_view::transfer(self, import, false),
            Action::TalkJson(import) => talk_view::transfer(self, import, true),
            Action::TalkFile(new) => talk_view::file(self, new),
            Action::SaveResource(key, layer) => {
                browser::save_as(self, key, layer.as_deref(), false)
            }
            Action::SaveModelText(key, layer) => {
                browser::save_as(self, key, layer.as_deref(), true)
            }
            Action::SaveResources(keys) => browser::export(self, &keys),
            Action::OpenResource(key) => {
                let Some(tab) = Tab::for_resource(key) else { return };
                // Options › Script Editor: scripts open in the external
                // editor (once Moonglow's editor has the script).
                if key.restype == ResType::NSS
                    && self.settings.scripts_external
                    && self.settings.external_editor.is_some()
                {
                    // (Alone, where the script's own file can be edited in
                    // place.)
                    if self.settings.scripts_external_only
                        && script_view::open_only_external(self, key)
                    {
                        return;
                    }
                    self.script_tools.open_externally.insert(key);
                }
                self.run_now(Action::OpenTab(tab));
            }
            Action::PlacePrefab(name) => self.place_prefab(&name),
            Action::CloseWindow => self.closing = true,
            Action::RestartGraphics(backend) => {
                renderer_check::restart(self, backend);
                self.run_now(Action::Quit);
            }
            Action::Quit => {
                // Saved or discarded by now: no recovery copy.
                self.forget_recovery();
                self.keep_open_tabs();
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
        // Claimed areas kept read-only, protected names asked about.
        claims::gate(self, &cmd)?;
        // (Where what the open Properties windows show goes.)
        let followed = blueprint::follow::followed(self, &cmd.edits);
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
        let tagged = (tag_clash::made_by(ws, &cmd.edits), format!("“{}”", cmd.label));
        ws.apply(cmd)?;
        blueprint::follow::retarget(self, followed);
        blueprint::after_apply(self, derived);
        tag_clash::check(self, &tagged.1, tagged.0, false);
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
            edits.push(mg_edit::Edit::SetResource { key: *key, data: Some(buf.mark_saved()) });
            if let Some(w) = buf.unwritable(*key) {
                self.log.warn(w);
            }
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
        // (Compiling on save off: the Compiler tab says they weren't.)
        if !self.settings.auto_compile {
            stored.iter().for_each(|k| self.script_tools.saved_not_compiled(*k));
        }
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
        // (Tables first: one of a hak's is saved into the hak, which is
        // then written; one of the module's is a step saved below.)
        self.save_side_docs();
        let Some(ws) = &mut self.ws else { return };
        // The custom palettes, as Aurora keeps them: the entries of the
        // blueprints added, changed or deleted since the module was opened
        // or saved are brought up to date, and nothing else is touched
        // (Build's Palettes makes them anew).
        if let Some(game) = &self.game {
            let saved = ws.saved().clone();
            let rebuilt = ws
                .derive(|module| mg_module::palette::update_custom_palettes(module, &saved, game))
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
                self.keep_locks();
                self.write_mod_beside_folder();
                self.plugins_module_saved();
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

    /// The file that keeps the locks of the open project, if it is one.
    fn locks_file(&self, m: &Module) -> Option<PathBuf> {
        let root = m.project.as_ref()?.root().to_path_buf();
        Some(mg_module::locks::file(self.locks_dir.as_deref()?, &root))
    }

    /// Puts a project's locked objects back on them: the project's files
    /// don't carry the lock ([`mg_module::locks`]).
    fn restore_locks(&mut self, m: &mut Module) {
        let Some(path) = self.locks_file(m) else { return };
        // (One that can't be read is kept beside it as `.unreadable` and
        // said: its objects come back unlocked.)
        let locks = mg_module::locks::try_read(&path).unwrap_or_else(|e| {
            self.log.warn(format!(
                "The project's locked objects could not be read ({e}): none is locked; the \
                 file is kept as .unreadable"
            ));
            mg_module::locks::read(&path)
        });
        let found = m.restore_locks(&locks);
        if found < locks.len() {
            trace::note(format!("locks: {found} of {} found again", locks.len()));
        }
    }

    /// Keeps the open project's locked objects in Moonglow's folder, after
    /// it was saved (its files don't carry them).
    fn keep_locks(&mut self) {
        let Some(ws) = self.ws.as_ref() else { return };
        let Some(path) = self.locks_file(&ws.module) else { return };
        let root = ws.module.project.as_ref().map(|p| p.root().to_path_buf()).unwrap_or_default();
        let locks = mg_module::locks::module_locks(&ws.module);
        if let Err(e) = mg_module::locks::write(&path, &root, &locks) {
            self.log.warn(format!("The locked objects could not be kept: {}: {e}", path.display()));
        }
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
        // (The .mod of its name there, in any case: `Castle/` beside
        // `castle.mod` is written as `castle.mod`.)
        let path = mg_module::folder::mod_beside(dir);
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else { return };
        // (As saving a .mod: the one there kept as its backup, and the new
        // one written beside it, then moved into its place.)
        if !self.settings.no_backups && path.is_file() {
            let backup = path.with_extension("BackupMod");
            if let Err(e) = std::fs::copy(&path, &backup) {
                self.log.warn(format!("{}: {e}", backup.display()));
            }
        }
        let part = dir.with_file_name(format!("{name}.moonglow-tmp"));
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

    /// Says in the log that the project file (`moonglow.toml`) was read
    /// (`again`: when it or the files beside it changed; `had`: whether
    /// it was there before), and what in them was not understood.
    pub(crate) fn log_project_file(&mut self, again: bool, had: bool) {
        let Some(ws) = &self.ws else { return };
        let p = &ws.module.project_file;
        // (Beside a .mod, the module's own: `krezk.moonglow.toml`.)
        let name = (p.place().map(|place| place.team_name()))
            .unwrap_or_else(|| mg_module::project_file::FILE_NAME.into());
        match (p.found, again && had) {
            (true, false) => self.log.info(format!("The project's rules are read from {name}")),
            (true, true) => self.log.info(format!("{name} changed: its rules are read again")),
            (false, true) => self.log.info(format!("{name} is gone: no project rules now")),
            (false, false) => {}
        }
        let described = p.variables.described.len();
        let file = p.variables.file();
        if p.variables_found {
            self.log.info(format!(
                "{described} variable{} described in {file}",
                if described == 1 { "" } else { "s" }
            ));
        }
        for e in &p.errors {
            self.log.error(format!("{e} (its rules are left out)"));
        }
        for w in &p.warnings {
            self.log.warn(w.clone());
        }
    }

    /// Logs (and clears) what the open nasher project has to say.
    fn take_project_warnings(&mut self) {
        let Some(ws) = &mut self.ws else { return };
        // (And a module folder's: a resource there under two spellings.)
        if let Some(folder) = ws.module.folder.as_mut() {
            for w in std::mem::take(&mut folder.warnings) {
                self.log.warn(w);
            }
        }
        let Some(project) = ws.module.project.as_mut() else { return };
        for w in std::mem::take(&mut project.warnings) {
            self.log.warn(w);
        }
    }

    /// Build › Add Left-Out Files to nasher.cfg (see
    /// [`mg_module::nasher::Project::take_in_left_out`]); the files are then
    /// read as files added to the tree.
    fn take_in_left_out(&mut self) {
        let Some(project) = self.ws.as_mut().and_then(|ws| ws.module.project.as_mut()) else {
            return;
        };
        match project.take_in_left_out() {
            Ok(Some(line)) => {
                self.log.info(format!("nasher.cfg: added {line}"));
                self.reload_project_files();
            }
            Ok(None) => self.log.info("The project's include leaves no file out"),
            Err(e) => self.log.error(e.to_string()),
        }
    }

    /// Build › Pack *file*: writes a nasher project's module file (its
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
                self.log.error("Pack: the module isn't in a nasher project");
                None
            }
            Err(e) => {
                self.log.error(format!("Pack: {e}"));
                None
            }
        }
    }

    /// Compiles the scripts that have no compiled version (a project's tree
    /// holds none).
    fn compile_uncompiled(&mut self) {
        let external = self.external_compiler();
        let debug = self.settings.debug_info;
        // (Noted as compiled, as Compile All notes them: it passes over
        // them next.)
        let file = self.remembered_scripts(external.as_ref());
        let known = self.script_stamps.clone();
        let (Some(ws), Some(game)) = (&mut self.ws, &self.game) else { return };
        let Ok(pass) = ws.derive(|module| {
            let uncompiled: Vec<ResKey> = (module.keys_of(ResType::NSS))
                .filter(|k| !module.contains(&ResKey::new(k.resref, ResType::NCS)))
                .filter(|k| {
                    let target = module.project.as_ref().map(|p| p.target());
                    !target.is_some_and(|t| t.skips_compiling(&k.to_string()))
                })
                .copied()
                .collect();
            mg_module::build::compile_changed(
                module,
                &game.resman,
                external.as_ref(),
                &known,
                Some(uncompiled),
                debug,
            )
        }) else {
            return;
        };
        self.keep_compiled(pass.now, file.as_deref());
        let results = pass.results;
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
        let haks = haks.iter().filter_map(|h| install?.find_hak(h)).collect();
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
                // (Its materials too: a mesh with no render hint takes its
                // material's.)
                let files: Vec<(ResKey, &[u8])> = (job.module.keys_of(ResType::MDL))
                    .chain(job.module.keys_of(ResType::MTR))
                    .filter_map(|k| Some((*k, job.module.get(k)?)))
                    .collect();
                let lookup = |name: &str, t: ResType| {
                    game.resman.get_named(name, t).ok().map(|d| d.into_owned())
                };
                let each = |done: usize, total: usize| {
                    job.progress.step(done, total);
                    !job.progress.cancelled()
                };
                Some(mg_module::models::compile_models_each(&files, &lookup, &each))
            },
            |app, made| {
                let Some(results) = made else { return };
                let sorted = model_view::CompiledModels::sort(&mut app.log, "the module", results);
                let Some(sorted) = sorted else { return };
                let edits: Vec<_> = (sorted.made.iter())
                    .map(|(key, b)| mg_edit::Edit::SetResource { key: *key, data: Some(b.clone()) })
                    .collect();
                if !edits.is_empty()
                    && let Err(e) = app.apply(Command::new("Compile Models", edits))
                {
                    app.log.error(e.to_string());
                    return;
                }
                sorted.say(&mut app.log);
            },
        );
    }

    /// Forgets what Compile All remembers in this session, as a new
    /// session begins: what is kept in [`compiled_dir`](Self::compiled_dir)
    /// is read at the next compile.
    pub fn forget_compiled(&mut self) {
        self.script_stamps = Default::default();
    }

    /// Makes `now` what is remembered of the compiled scripts, and keeps
    /// it in `file` (what [`remembered_scripts`](Self::remembered_scripts)
    /// gave) for the next session and `mg compile`.
    fn keep_compiled(&mut self, now: mg_module::build::Remembered, file: Option<&std::path::Path>) {
        self.script_stamps = now;
        if let Some(file) = file
            && let Err(e) = self.script_stamps.write(file)
        {
            crate::trace::note(format!("compiled scripts not noted in {file:?}: {e}"));
        }
    }

    /// [`Action::NoteCompiled`]: scripts compiled one by one, noted as
    /// compiled (as Compile All notes those it compiles) where the module
    /// has the compiled script that was made of each, so that the next
    /// Compile All, here or `mg compile`, passes over them.
    pub(crate) fn note_compiled(&mut self, made: &[(ResKey, Vec<u8>)]) {
        let external = self.external_compiler();
        let file = self.remembered_scripts(external.as_ref());
        let Some(ws) = &self.ws else { return };
        let module = &ws.module;
        let stored: Vec<ResKey> = (made.iter())
            .filter(|(k, ncs)| module.get(&ResKey::new(k.resref, ResType::NCS)) == Some(&ncs[..]))
            .map(|(k, _)| *k)
            .collect();
        if stored.is_empty() {
            return;
        }
        let stamps = mg_module::build::script_stamps_of(module, &stored);
        let mut now = self.script_stamps.clone();
        for k in stored {
            now.note(module, &stamps, k);
        }
        self.keep_compiled(now, file.as_deref());
    }

    /// Makes `script_stamps` what is remembered of the module's compiled
    /// scripts for a compile with `external` (the built-in compiler
    /// without): what this session remembers if it was made with the same,
    /// else what is kept from the last session, or by `mg compile`, if
    /// that was, else nothing. Returns the file it is kept in, to write
    /// what the compile adds; None where nothing is kept (or without the
    /// game data, which nothing compiles without).
    pub(crate) fn remembered_scripts(
        &mut self,
        external: Option<&mg_module::external_compiler::ExternalCompiler>,
    ) -> Option<std::path::PathBuf> {
        let game = self.game.as_deref()?;
        // What the stamps were made with: another module, compiler,
        // setting, or game data (the haks and folders as they were read),
        // and every script is compiled again.
        let made_with = mg_module::build::made_with(
            self.module_path().as_deref(),
            external,
            self.install.as_ref().map(|i| i.root.as_path()),
            &game.resman,
        );
        if self.script_stamps.made_with != made_with {
            self.script_stamps = mg_module::build::Remembered::kept(
                self.compiled_dir.as_deref(),
                self.module_path().as_deref(),
                made_with,
            );
        }
        (self.compiled_dir.as_ref().zip(self.module_path()))
            .map(|(dir, module)| mg_module::build::Remembered::file(dir, &module))
    }

    /// Compiles scripts as a job: all of them (`None`: those changed
    /// since they were last compiled here), or those named.
    pub(crate) fn compile_job(&mut self, only: Option<Vec<ResKey>>) {
        if self.ws.is_none() || self.game.is_none() {
            self.log.error("Compiling needs an open module and the game data");
            return;
        }
        let external = self.external_compiler();
        let file = self.remembered_scripts(external.as_ref());
        let known = self.script_stamps.clone();
        let debug = self.settings.debug_info;
        let all = only.is_none();
        let title = if all { "Compile All Scripts" } else { "Compile Scripts" };
        self.start_job(
            title,
            move |job| {
                let game = job.game.as_deref()?;
                let mut staged = job.module.clone();
                let pass = mg_module::build::compile_changed(
                    &mut staged,
                    &game.resman,
                    external.as_ref(),
                    &known,
                    only,
                    debug,
                );
                let edits: Vec<mg_edit::Edit> = staged
                    .keys()
                    .filter(|k| matches!(k.restype, ResType::NCS | ResType::NDB))
                    .filter(|k| staged.get(k) != job.module.get(k))
                    .map(|k| mg_edit::Edit::SetResource {
                        key: *k,
                        data: staged.get(k).map(<[u8]>::to_vec),
                    })
                    .collect();
                let mg_module::build::Pass { results, now, scripts } = pass;
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
                // (Each once: an include's is said by every includer.)
                for (w, n) in mg_module::build::distinct_warnings(&results) {
                    app.log.warn(match n {
                        1 => w.message.clone(),
                        n => format!("{} (in {n} scripts)", w.message),
                    });
                }
                app.keep_compiled(now, file.as_deref());
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

    /// Whether a test may write the module `name` and start the game on
    /// it, as to games running that Moonglow did not start (GitHub issue
    /// 6): not under one started on that module (closed first, as one
    /// Moonglow started); under any other, with a warning (one started
    /// from the game's own menu may have it open). Read only: a game is
    /// never stopped.
    fn other_game_allows(&mut self, name: &str) -> bool {
        let others = test_module::other_games();
        if let Some(game) = others.iter().find(|g| g.plays(name) == Some(true)) {
            self.log.warn(format!(
                "A game is running on {name} (process {}), not started by Moonglow: close it, then                  test again (the test would write the module under it)",
                game.pid
            ));
            return false;
        }
        if !others.is_empty() {
            let pids: Vec<String> = others.iter().map(|g| g.pid.to_string()).collect();
            self.log.warn(format!(
                "A game is running that Moonglow did not start (process {}): if it has {name}                  open, close it and test again, for the module is written under it",
                pids.join(", ")
            ));
        }
        true
    }

    /// Before Test Module's save, which writes the module where the game
    /// reads it: whether a test may go on, as to the games running (one
    /// Moonglow started for a test, one started otherwise on this module).
    /// A nasher project's packed module is looked at once packed, before
    /// it is copied there.
    pub(crate) fn test_may_start(&mut self) -> bool {
        if test_module::game_running() {
            self.log.warn(
                "The game started for the last test is still running: close it, then test \
                 again (a second test would write the module under it)",
            );
            return false;
        }
        if self.ws.as_ref().is_some_and(|ws| ws.module.project.is_some()) {
            return true;
        }
        let user = self.install.as_ref().and_then(|gi| gi.user_dir.clone());
        let name = user.zip(self.module_path()).and_then(|(u, p)| test_module::module_name(&u, &p));
        let Some(name) = name else { return true };
        let allowed = self.other_game_allows(&name);
        self.test_checked = allowed.then_some(name);
        allowed
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
        // NUI windows changed since their last build are built first, so the
        // game opens what the Creator shows, and saved with the module (which
        // Test Module saved before this); one that can't be built is named.
        let stale =
            self.ws.as_ref().map(|ws| nui_view::stale_windows(&ws.module)).unwrap_or_default();
        for name in &stale {
            nui_view::generate(self, name, false);
        }
        if !stale.is_empty() {
            self.save(None);
        }
        if let Some(ws) = &self.ws {
            let left = nui_view::stale_windows(&ws.module);
            if !left.is_empty() {
                self.log.warn(format!(
                    "NUI windows not built (fix their problems in the NUI Creator): {}; the \
                     game runs what was last built",
                    left.join(", ")
                ));
            }
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
            let name = packed.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            if !self.other_game_allows(&name) {
                return;
            }
            let dest = user.join("modules").join(file);
            let copied = std::fs::create_dir_all(user.join("modules"))
                .and_then(|()| std::fs::copy(&packed, &dest).map(|_| ()));
            if let Err(e) = copied {
                self.log.error(format!("Test Module: {}: {e}", dest.display()));
                return;
            }
            self.log.info(format!("Installed {}", dest.display()));
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
        // (Looked at before Test Module's save; else now.)
        if self.test_checked.take().as_deref() != Some(name.as_str())
            && !self.other_game_allows(&name)
        {
            return;
        }
        self.launch_test(&client, user, &name, choose);
    }

    /// Test From Here: the module as it is now, not saved, written to the
    /// modules folder as [`test_module::FROM_HERE`] with its start moved,
    /// and the game started on it.
    fn test_from_here(&mut self, area: mg_core::ResRef, at: [f32; 3], facing: f32, choose: bool) {
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
        if !self.other_game_allows(test_module::FROM_HERE) {
            return;
        }
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
                self.launch_test(&client, &user, test_module::FROM_HERE, choose);
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
        self.release_game_data();
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
        // (And the talk table editor's, from its own files.)
        talk_view::read_again(self);
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
        self.release_game_data();
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
        self.search_kept = Default::default();
        self.model_views.clear();
        for view in self.area_views.values_mut() {
            view.reload();
        }
        self.load_order_changed();
    }

    /// Makes the placeables of `areas` static, as one command, where
    /// nothing is lost by it (`mg_module::areas::static_plan`: not the
    /// Useable ones, those a visual transform changes, those with scripts
    /// and the like, nor those with variables or the project's rules
    /// name), and says in the log what was left, and why.
    fn static_placeables(&mut self, areas: &[mg_core::ResRef]) {
        let Some(ws) = self.ws.as_mut() else { return };
        let rules = ws.module.project_file.make_static.clone();
        let mut edits = Vec::new();
        let (mut made, mut within) = (0, 0);
        let mut left = mg_module::areas::StaticPlan::default();
        // (The first few kept by a variable or the project, by name.)
        let mut named = Vec::new();
        for &area in areas {
            let key = ResKey::new(area, ResType::GIT);
            let Ok(git) = ws.doc(&key) else { continue };
            let plan = mg_module::areas::static_plan(&git.root, &rules);
            let list = git.root.list("Placeable List").unwrap_or(&[]);
            for &i in &plan.convert {
                let path = mg_edit::GffPath::root().item("Placeable List", i);
                for (label, value) in mg_module::areas::static_fields(&list[i]) {
                    edits.push(mg_edit::Edit::SetField {
                        key,
                        path: path.clone(),
                        label: label.into(),
                        value,
                    });
                }
            }
            for (i, why) in &plan.kept {
                use mg_module::areas::Kept;
                if matches!(why, Kept::Variable(_) | Kept::Tag(_) | Kept::ResRef(_)) {
                    let tag = list[*i].string("Tag").map(text::decode).unwrap_or_default();
                    named.push(format!("{tag} in {area}: {}", why.describe()));
                }
            }
            made += plan.convert.len();
            within += usize::from(!plan.convert.is_empty());
            left.add(&plan);
        }
        let kept = match left.left() {
            l if l.is_empty() => String::new(),
            l => format!(" (left dynamic: {l})"),
        };
        if edits.is_empty() {
            self.log.info(format!("No placeables to make static{kept}"));
        } else {
            match self.apply(Command::new("Make Placeables Static", edits)) {
                Ok(()) => {
                    let s = if made == 1 { "" } else { "s" };
                    let a = if within == 1 { "" } else { "s" };
                    self.log
                        .info(format!("Made {made} placeable{s} static in {within} area{a}{kept}"));
                }
                Err(e) => {
                    self.log.error(e.to_string());
                    return;
                }
            }
        }
        const SHOWN: usize = 8;
        for line in named.iter().take(SHOWN) {
            self.log.info(format!("Left dynamic: {line}"));
        }
        if named.len() > SHOWN {
            self.log.info(format!(
                "Left dynamic: {} more by variables or the project",
                named.len() - SHOWN
            ));
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
        self.ws.as_mut().map(listed_haks).unwrap_or_default()
    }

    /// Layers the module's haks as Module Properties lists them, when the
    /// list changed (an edit, an undo): no reopening needed.
    /// Closes the module's haks, so that another program may write over
    /// them (a project's command): whether any were closed.
    /// [`Moonglow::sync_haks`] opens them again.
    pub(crate) fn release_haks(&mut self) -> bool {
        if self.haks_layered.is_empty() {
            return false;
        }
        self.release_game_data();
        let Some(game) = exclusive(&mut self.game) else { return false };
        let haks: Vec<String> = (game.resman.layers().iter())
            .filter(|l| l.label.starts_with("hak:"))
            .map(|l| l.label.clone())
            .collect();
        for l in haks {
            game.resman.remove(&l);
        }
        self.haks_layered.clear();
        self.game_data_changed();
        true
    }

    pub(crate) fn sync_haks(&mut self) {
        let listed = self.listed_haks();
        if listed == self.haks_layered {
            return;
        }
        self.release_game_data();
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
        match game.resman.add_haks(gi, &listed) {
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
    /// and folders that changed. With it off, the project file
    /// (`moonglow.toml`) alone, as frames are drawn: a few small files,
    /// and the rules the module is worked by.
    fn reload_timer(&mut self, ui: &egui::Ui) {
        // A look at the module's files under way: taken once it is done.
        if self.take_looked(ui.ctx()) {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
        // (Whether blueprints read again changed their palettes.)
        if self.look_at_palettes() {
            ui.ctx().request_repaint();
        }
        // (Not under a job, which reads the game data and the module.)
        if self.busy() {
            return;
        }
        // (Scripts read again from outside, compiled a few a frame.)
        if self.compile_outside_step() {
            ui.ctx().request_repaint();
        }
        let every = std::time::Duration::from_secs(3);
        let now = std::time::Instant::now();
        let off = self.settings.no_auto_reload || self.game.is_none();
        // (A watch asks for a frame when it hears a change, which is
        // looked at then, not when the few seconds' look is due.)
        self.outside.repaint = Some(ui.ctx().clone());
        let heard = !off && self.heard_outside();
        if self.reload_checked.is_some_and(|t| now.duration_since(t) < every) && !heard {
            if !off {
                ui.ctx().request_repaint_after(every);
            }
            return;
        }
        if !heard {
            self.reload_checked = Some(now);
        }
        if off {
            self.reread_project_file();
            return;
        }
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

    /// Makes the fix offered beside line `line` of the log, as one undoable
    /// command: only while what it changes is as Verify found it
    /// ([`mg_edit::fix::Guard`]); else the log says to verify again.
    fn apply_fix(&mut self, line: usize) {
        let Some(offer) = self.log.fix_at(line).filter(|o| !o.made).cloned() else { return };
        let Some(ws) = self.ws.as_mut() else { return };
        // (Its paths following what other fixes took out.)
        let now = match self.log.fix_guard.check(ws, &offer.fix) {
            Ok(now) => now,
            Err(why) => {
                self.log.error(format!("{}: {why}", offer.fix.label));
                return;
            }
        };
        if let Err(e) = self.apply(now.command()) {
            self.log.error(format!("{}: {e}", offer.fix.label));
            return;
        }
        if let Some(ws) = self.ws.as_mut() {
            self.log.fix_guard.made(ws, &offer.fix);
        }
        if let Some(o) = self.log.fixes.iter_mut().find(|o| o.line == line) {
            o.made = true;
        }
        let resources: Vec<String> = offer.fix.resources().iter().map(|k| k.to_string()).collect();
        self.log.info(format!("Fixed: {} ({})", offer.fix.label, resources.join(", ")));
    }

    /// Verify Module, as a job: what is missing, and what is wrong with the
    /// custom content, into the log.
    /// Build › Round-Trip Check, as a job: the module as it is (unsaved
    /// changes too) saved to a copy of its own and read back, and its GFFs
    /// and 2DAs written anew; what doesn't come back as it was is said in
    /// the log. The module and its file are not touched.
    fn round_trip(&mut self) {
        if self.ws.is_none() {
            return;
        }
        let copy =
            std::env::temp_dir().join(format!("moonglow-roundtrip-{}.mod", std::process::id()));
        self.start_job(
            "Round-Trip Check",
            move |job| {
                job.progress.say("Saving a copy and reading it back");
                let progress = |done, total| {
                    job.progress.step(done, total);
                    !job.progress.cancelled()
                };
                mg_module::roundtrip::check(&job.module, &copy, false, &progress)
            },
            |app, report| match report {
                Ok(r) => {
                    for (k, why) in &r.unread {
                        app.log.warn(format!(
                            "Round trip: {k} cannot be read ({why}): kept as it is, never written \
                             anew"
                        ));
                    }
                    for (k, what) in &r.differences {
                        app.log.error(format!("Round trip: {k}: {what}"));
                    }
                    match r.differences.is_empty() {
                        true => app.log.info(format!("Round trip: {}", r.summary())),
                        false => app.log.error(format!("Round trip: {}", r.summary())),
                    }
                }
                // (Called off: nothing to say, as Verify.)
                Err(e) if e == "stopped" => {}
                Err(e) => app.log.error(format!("Round trip: {e}")),
            },
        );
    }

    fn verify(&mut self) {
        if self.ws.is_none() || self.game.is_none() {
            return;
        }
        // The enabled plugins' checks run with the doctor's, each with its
        // settings.
        let plugins: Vec<(mg_plugin::Plugin, mg_plugin::Settings)> = self
            .enabled_plugins()
            .into_iter()
            .filter(|p| !p.manifest.checks.is_empty())
            .map(|p| (p.clone(), self.plugin_settings(p)))
            .collect();
        // What is left out (Options › Verify).
        let skip = mg_module::doctor::Skip::read(&self.settings.verify_skip);
        // (The shader programs that compiled kept where the views keep
        // theirs, and what was found of the haks' models beside them.)
        let cache = (self.shader_cache_dir.as_deref())
            .and_then(std::path::Path::parent)
            .map(mg_module::doctor::Cache::new);
        self.start_job(
            "Verify Module",
            move |job| {
                use mg_module::doctor::Group;
                let game = job.game.as_deref()?;
                // (A group left out isn't checked.)
                let missing = if skip.group(Group::References) {
                    Vec::new()
                } else {
                    job.progress.say("Looking for what is missing");
                    mg_module::verify::missing(&job.module, &game.resman)
                };
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
                let mut times = mg_module::doctor::Timings::default();
                let mut findings = mg_module::doctor::examine_groups(
                    &job.module,
                    &game.resman,
                    tlk,
                    &skip,
                    &mut times,
                    cache.as_ref(),
                );
                // (What the plugins would say of themselves goes with them.)
                let (theirs, said) = if skip.group(Group::Plugins) {
                    (Vec::new(), Vec::new())
                } else {
                    plugins::check_findings(&plugins, &skip, job)
                };
                findings.extend(theirs.iter().map(|f| f.finding.clone()));
                let mut missing = missing;
                skip.leave_out(&mut missing, &mut findings);
                let left = skip.named();
                // Each finding with its fix, if it has one; and what the
                // fixes were worked out on.
                let found = mg_edit::fix::attach(&job.module, findings, &theirs);
                let fixes = found.iter().filter_map(|f| f.fix.as_ref());
                let guard = mg_edit::fix::Guard::new(&job.module, fixes);
                Some((missing, found, said, left, guard))
            },
            |app, made| {
                let Some((missing, found, said, left, guard)) = made else { return };
                for (level, text) in said {
                    app.log.said(level, text);
                }
                // (The last Verify's fixes alone are offered.)
                app.log.fixes.clear();
                app.log.fix_guard = guard;
                for (level, text, fix) in verify_report_fixes(&missing, &found) {
                    if let Some(fix) = fix {
                        let line = app.log.entries.len();
                        let offer = OfferedFix { line, text: text.clone(), fix, made: false };
                        app.log.fixes.push(offer);
                    }
                    match level {
                        LogLevel::Error => app.log.error(text),
                        LogLevel::Warn => app.log.warn(text),
                        LogLevel::Info => app.log.info(text),
                    }
                }
                let findings: Vec<&mg_module::doctor::Finding> =
                    found.iter().map(|f| &f.finding).collect();
                let errors = missing.iter().filter(|m| m.is_error()).count()
                    + findings
                        .iter()
                        .filter(|f| f.severity == mg_module::doctor::Severity::Error)
                        .count();
                let gone = missing.iter().filter(|m| m.placed_from_gone_blueprint()).count();
                let warnings = missing.len() + findings.len() - errors - gone;
                let gone = match gone {
                    0 => String::new(),
                    n => format!("; {n} object(s) placed from blueprints the module no longer has"),
                };
                app.log.info(format!(
                    "Verify: {errors} error(s), {warnings} warning(s) ({} missing reference(s), {} content problem(s){gone}){}",
                    missing.len(),
                    findings.len(),
                    left_out(&left),
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
        let minutes = u64::from(self.settings.dialog_backup_every().max(1));
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
            // (Not while `--user-dir` chooses this run's.)
            || (settings.user_dir != self.settings.user_dir
                && settings.user_dir_this_run.is_none())
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
                self.search_kept = Default::default();
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
    /// The scripts the compile memory holds: compiled here (Compile All,
    /// a Build, a rename), passed over while they are as compiled.
    pub fn compiled_scripts(&self) -> Vec<ResKey> {
        self.script_stamps.scripts.keys().copied().collect()
    }

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

pub(crate) use mg_module::build::user_content;

/// How long the window waits for what reads the game data on a thread of
/// its own to let go of it, before its layers are changed
/// ([`Moonglow::release_game_data`]).
const RELEASE_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

impl Moonglow {
    /// An open area view's terrain tools on `tileset`: the tileset as the
    /// view read it, its tiles' index and rules.
    pub(crate) fn open_tileset(&self, tileset: mg_core::ResRef) -> Option<&terrain_mode::Tools> {
        (self.area_views.values().filter_map(|v| v.terrain.as_ref())).find(|t| t.tileset == tileset)
    }

    /// Tileset `tileset`: as an open area's view has it, else read from
    /// the game (`None`: there is none, or it can't be read).
    pub fn tileset(&self, tileset: mg_core::ResRef) -> Option<Arc<mg_set::Tileset>> {
        if let Some(open) = self.open_tileset(tileset) {
            return Some(open.set.clone());
        }
        let game = self.game.as_deref()?;
        mg_area::tileset(game, tileset).ok().map(Arc::new)
    }

    /// Has what reads the game data apart from the window let go of it, so
    /// that it can be changed ([`exclusive`], called after this): the 3D
    /// views' texture readers (taken again when an area view next draws);
    /// and, ten seconds at most, what still reads it on a thread (an area
    /// of a module just closed, read on). Not a job: the window waits for
    /// none while one runs (its game data is changed once it is done).
    pub(crate) fn release_game_data(&mut self) {
        let Some(game) = &self.game else { return };
        if Arc::strong_count(game) == 1 {
            return;
        }
        if let Some(vp) = &mut self.viewport {
            vp.release_game_data();
        }
        if self.busy() {
            return;
        }
        let started = std::time::Instant::now();
        while Arc::strong_count(game) > 1 && started.elapsed() < RELEASE_WAIT {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        // (Said where it was waited for: the views let go at every edit.)
        let (others, waited) = (Arc::strong_count(game) - 1, started.elapsed());
        if others > 0 || waited.as_millis() >= 10 {
            let still =
                if others > 0 { format!(", still shared by {others} other(s)") } else { "".into() };
            trace::note(format!("game data released in {waited:.0?}{still}"));
        }
    }

    /// Whether something other than the window reads the game data (a job,
    /// an area read on a thread): not the 3D views, which let go of it to
    /// have it changed ([`Moonglow::release_game_data`]).
    pub(crate) fn game_data_held(&self) -> bool {
        let Some(game) = &self.game else { return false };
        let views = self.viewport.as_ref().is_some_and(|vp| vp.holds(game));
        Arc::strong_count(game) - 1 > usize::from(views)
    }
}

/// The module's haks as Module Properties lists them, unsaved changes
/// included ([`mg_module::haks_of`], as the module's saved haks are read).
pub(crate) fn listed_haks(ws: &mut mg_edit::Workspace) -> Vec<String> {
    ws.doc(&module_props::info_key()).map(|info| mg_module::haks_of(&info.root)).unwrap_or_default()
}

/// What is kept for windows of `tab`'s kind (a size, a place), or for its
/// kind as it was called before ([`Tab::kind_before`]).
fn kept_by_kind<'a, T>(kept: &'a [(String, T)], tab: &Tab) -> Option<&'a (String, T)> {
    let kind = tab.kind();
    kept.iter()
        .find(|(k, _)| k == kind)
        .or_else(|| tab.kind_before().and_then(|b| kept.iter().find(|(k, _)| k == b)))
}

/// The game data to change (its layers, its talk table): only while
/// nothing else reads it, which is whenever the window takes input once
/// what reads it apart has let go of it ([`Moonglow::release_game_data`],
/// called first).
pub(crate) fn exclusive(game: &mut Option<Arc<GameData>>) -> Option<&mut GameData> {
    game.as_mut().and_then(Arc::get_mut)
}

/// What Verify's summary says it left out (a group by its title, the rest
/// by its word: a check's id, a hak, a resource), and where to have them
/// back.
fn left_out(left: &[String]) -> String {
    if left.is_empty() {
        return String::new();
    }
    format!("; left out, not checked (Options › Verify): {}", left.join(", "))
}

/// How a line of Verify's report is said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
}

/// Verify Module's report, as the log shows it: errors first, then
/// warnings; and objects placed from a blueprint that is gone, one line a
/// blueprint, last, said for what they are (each object was a line of its
/// own, among the first, a builder's report: some 200 of them, read as
/// errors).
pub fn verify_report(
    missing: &[mg_module::verify::Missing],
    findings: &[mg_module::doctor::Finding],
) -> Vec<(LogLevel, String)> {
    let found: Vec<mg_edit::fix::Found> = findings.iter().cloned().map(Into::into).collect();
    verify_report_fixes(missing, &found).into_iter().map(|(l, t, _)| (l, t)).collect()
}

/// [`verify_report`], each finding's line with the fix it offers.
pub fn verify_report_fixes(
    missing: &[mg_module::verify::Missing],
    found: &[mg_edit::fix::Found],
) -> Vec<(LogLevel, String, Option<mg_edit::fix::Fix>)> {
    use mg_module::doctor::Severity;
    let reference = |m: &mg_module::verify::Missing| {
        let what = format!("is {}", m.what());
        format!(
            "{:?}: {}{} → {} {} {what}",
            m.category,
            m.reference.from,
            m.reference.path,
            m.reference.kind.name(),
            m.reference.target
        )
    };
    let finding = |x: &mg_edit::fix::Found| {
        let f = &x.finding;
        let at = if f.at.is_empty() { String::new() } else { format!(" › {}", f.at) };
        format!("{} › {}{at}: {}", f.source, f.resource, f.message)
    };
    let (gone, missing): (Vec<_>, Vec<_>) =
        missing.iter().partition(|m| m.placed_from_gone_blueprint());
    let mut out = Vec::new();
    let of = |severity: Severity, level: LogLevel| {
        (found.iter().filter(move |x| x.finding.severity == severity))
            .map(move |x| (level, finding(x), x.fix.clone()))
    };
    out.extend(
        missing.iter().filter(|m| m.is_error()).map(|m| (LogLevel::Error, reference(m), None)),
    );
    out.extend(of(Severity::Error, LogLevel::Error));
    out.extend(
        missing.iter().filter(|m| !m.is_error()).map(|m| (LogLevel::Warn, reference(m), None)),
    );
    out.extend(of(Severity::Warning, LogLevel::Warn));
    // By blueprint: how many are placed from it, and in which areas.
    let mut by_blueprint: std::collections::BTreeMap<String, (usize, Vec<String>)> =
        Default::default();
    for m in &gone {
        let r = &m.reference;
        let name = match r.kind {
            mg_module::refs::RefKind::Blueprint(t) => {
                format!("{}.{}", r.target, t.extension().unwrap_or("?"))
            }
            _ => r.target.to_string(),
        };
        let entry = by_blueprint.entry(name).or_default();
        entry.0 += 1;
        let area = r.from.resref.to_string();
        if !entry.1.contains(&area) {
            entry.1.push(area);
        }
    }
    if !by_blueprint.is_empty() {
        out.push((
            LogLevel::Info,
            format!(
                "{} object(s) are placed from {} blueprint(s) the module no longer has: they \
                 work as they are (each holds all it needs, as in many of BioWare's modules); \
                 only placing more of them and Update Instances need the blueprint",
                gone.len(),
                by_blueprint.len()
            ),
            None,
        ));
        for (blueprint, (count, areas)) in by_blueprint {
            out.push((
                LogLevel::Info,
                format!("  {blueprint}: {count} placed, in {}", areas.join(", ")),
                None,
            ));
        }
    }
    out
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
