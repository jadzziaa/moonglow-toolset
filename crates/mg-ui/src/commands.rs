//! The window's commands, in one table: what each is called, when it can
//! be chosen and what it does. The menus, the toolbar, the keys (Options ›
//! Keyboard) and whatever else offers commands read it, so a command is
//! named, enabled and run the same wherever it is chosen, and every one of
//! them can be given a key.
//!
//! [`MENUS`] and [`TOP_BAR`] say where the commands show. Commands that act
//! on one thing (an object's or a resource's context menu) are not here:
//! they belong to what they act on.

use egui::Ui;
use mg_module::palette::BlueprintKind;

use crate::icons::Icon;
use crate::{Action, Moonglow, Tab, keys};

/// A command of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Id {
    NewModule,
    OpenModule,
    OpenFolder,
    Save,
    SaveAs,
    SaveAsProject,
    Import,
    /// File › Import Files…: loose files' resources.
    ImportFiles,
    /// File › Import Folder…: a folder's files' resources.
    ImportFolder,
    Export,
    ImportConversation,
    Close,
    Exit,
    Undo,
    Redo,
    ModuleProperties,
    BuilderCode,
    /// Edit › Claims Table…: whose each area is.
    ClaimsTable,
    ResizeArea,
    RotateArea,
    ChangeTileset,
    FindInstance,
    /// Edit › Edit Areas Together…: the areas to edit as one.
    EditAreasTogether,
    /// Edit › Update All Instances…: Update Instances for every blueprint
    /// of the module.
    UpdateAllInstances,
    ReplaceText,
    FindReferences,
    AdvancedSearch,
    /// Edit › Convert Indentation to Tabs: the script in front's.
    IndentToTabs,
    /// Edit › Convert Indentation to Spaces.
    IndentToSpaces,
    AreaWizard,
    CreatureWizard,
    /// A blueprint wizard (the kinds of `blueprint_wizard::KINDS`).
    Wizard(BlueprintKind),
    NewConversation,
    Factions,
    Journal,
    TalkTable,
    /// Tools › 2DA File…: a table anywhere, in the 2DA editor.
    OpenTable,
    NewScript,
    NewNui,
    Palettes,
    PlaceableGallery,
    Resources,
    NewTileset,
    OpenTileset,
    NewHak,
    OpenHak,
    BuildHak,
    ReloadResources,
    Options,
    CompileAll,
    CompileModels,
    BuildModule,
    PublishNwsync,
    Verify,
    /// The module saved to a copy and read back (`mg roundtrip`).
    RoundTrip,
    /// The area the module starts in, opened at the start location.
    GoToStart,
    TestModule,
    TestChoose,
    PackTarget,
    /// A nasher project's files its include leaves out, taken in.
    TakeInLeftOut,
    AreaStatistics,
    /// The Plugins window.
    Plugins,
    /// Install a plugin from its archive.
    InstallPlugin,
    Manual,
    CommandPalette,
    /// Asks GitHub whether a newer Moonglow is out (`updates`).
    CheckUpdates,
    About,
    /// Tools › Preview: the Right Dock shown on its Preview panel, as
    /// Tools › Palettes shows it on the Palette (the Preview window's
    /// command until M3's T6: its id, and so a key given it, as before).
    PreviewWindow,
    FullScreen,
    /// The tab in front: closed, the next or the one before brought to the
    /// front, the one closed last opened again.
    CloseTab,
    NextTab,
    PreviousTab,
    ReopenTab,
    /// The window of the tab in front made to fill Moonglow's window, or
    /// put back (as its bar's button, a double click on it, its menu).
    MaximizeWindow,
    /// The arrow keys given to the next pane, or the one before.
    NextPane,
    PreviousPane,
    /// View: the panes beside the middle shown or folded away.
    ViewTree,
    ViewPalettes,
    ViewLog,
    /// View › Copy Log and Save Log…: the whole log, each line with its
    /// time, to the clipboard or into a text file (Jasperre's report).
    CopyLog,
    SaveLog,
    /// View › Clear Log: the log emptied.
    ClearLog,
    HidePanels,
    ResetLayout,
}

/// What a menu holds, in order.
#[derive(Debug, Clone, Copy)]
pub enum Item {
    Do(Id),
    Separator,
    Sub(&'static str, &'static [Item]),
    /// The modules opened lately.
    Recent,
    /// The saved prefabs, to place.
    Prefabs,
    /// The blueprint wizards.
    Wizards,
    /// The enabled plugins' commands.
    PluginCommands,
    /// A command, and the enabled plugins' entries of a kind after it in
    /// a submenu of the command's name where there are any (File › Export
    /// › an exporter, API 0.6); else the command alone.
    WithPlugins(Id, crate::plugin_kinds::Added),
    /// The enabled plugins' wizards (API 0.6).
    PluginWizards,
    /// The enabled plugins' panels, in a submenu Panels (API 0.6).
    PluginPanels,
    /// The commands the project names (moonglow.toml's `[commands]`).
    Project,
}

use Item::{Do, Separator, Sub};

/// The menu bar.
pub const MENUS: [(&str, &[Item]); 8] = [
    (
        "File",
        &[
            Do(Id::NewModule),
            Do(Id::OpenModule),
            Do(Id::OpenFolder),
            Item::Recent,
            Do(Id::Save),
            Do(Id::SaveAs),
            Do(Id::SaveAsProject),
            Item::Project,
            Separator,
            Item::WithPlugins(Id::Import, crate::plugin_kinds::Added::Importer),
            Do(Id::ImportFiles),
            Do(Id::ImportFolder),
            Item::WithPlugins(Id::Export, crate::plugin_kinds::Added::Exporter),
            Do(Id::ImportConversation),
            Separator,
            Do(Id::Close),
            Separator,
            Do(Id::Exit),
        ],
    ),
    (
        "Edit",
        &[
            Do(Id::Undo),
            Do(Id::Redo),
            Separator,
            Do(Id::ModuleProperties),
            Do(Id::BuilderCode),
            Do(Id::ClaimsTable),
            Separator,
            Do(Id::ResizeArea),
            Do(Id::RotateArea),
            Do(Id::ChangeTileset),
            Separator,
            Do(Id::FindInstance),
            Item::Prefabs,
            Do(Id::EditAreasTogether),
            Do(Id::UpdateAllInstances),
            Do(Id::ReplaceText),
            Do(Id::FindReferences),
            Do(Id::AdvancedSearch),
            Separator,
            Do(Id::IndentToTabs),
            Do(Id::IndentToSpaces),
        ],
    ),
    (
        "View",
        &[
            Do(Id::ViewTree),
            Do(Id::ViewPalettes),
            Do(Id::ViewLog),
            Do(Id::CopyLog),
            Do(Id::SaveLog),
            Do(Id::ClearLog),
            Separator,
            Do(Id::GoToStart),
            Separator,
            Do(Id::HidePanels),
            Do(Id::ResetLayout),
        ],
    ),
    (
        "Wizards",
        &[
            Do(Id::AreaWizard),
            Separator,
            Do(Id::CreatureWizard),
            Item::Wizards,
            Item::PluginWizards,
        ],
    ),
    (
        "Tools",
        &[
            Do(Id::NewConversation),
            Do(Id::Factions),
            Do(Id::Journal),
            Do(Id::TalkTable),
            Do(Id::OpenTable),
            Do(Id::NewScript),
            Do(Id::NewNui),
            Separator,
            Do(Id::Palettes),
            Do(Id::PreviewWindow),
            Do(Id::PlaceableGallery),
            Do(Id::Resources),
            Sub("Tilesets", &[Do(Id::NewTileset), Do(Id::OpenTileset)]),
            Sub("Haks", &[Do(Id::NewHak), Do(Id::OpenHak), Do(Id::BuildHak)]),
            Do(Id::ReloadResources),
            Do(Id::Options),
        ],
    ),
    (
        "Build",
        &[
            Do(Id::CompileAll),
            Do(Id::CompileModels),
            Do(Id::BuildModule),
            Do(Id::PublishNwsync),
            Do(Id::Verify),
            Do(Id::RoundTrip),
            Do(Id::TestModule),
            Do(Id::TestChoose),
            Do(Id::PackTarget),
            Do(Id::TakeInLeftOut),
            Separator,
            Do(Id::AreaStatistics),
        ],
    ),
    (
        "Plugins",
        &[Item::PluginCommands, Item::PluginPanels, Do(Id::Plugins), Do(Id::InstallPlugin)],
    ),
    (
        "Help",
        &[Do(Id::Manual), Do(Id::CommandPalette), Separator, Do(Id::CheckUpdates), Do(Id::About)],
    ),
];

/// A button of the Top Bar: its command, its icon, its name (its
/// accessible name and the first line of its tip) and its tip.
pub type TopBarButton = (Id, Icon, &'static str, &'static str);

/// The Top Bar's buttons ([`crate::shell::top_bar`]): the quick actions
/// after the menus (`None`: a line between their groups), and the build
/// actions at the right end. Test Module's arrow offers
/// [`TOP_BAR_TEST_MENU`].
#[derive(Debug)]
pub struct TopBar {
    /// After the menus: New, Open, Save; Undo, Redo.
    pub quick: [Option<TopBarButton>; 6],
    /// At the right end: Compile, Verify, Test Module.
    pub build: [TopBarButton; 3],
}

impl TopBar {
    /// Every command the Top Bar offers, its arrow's included.
    pub fn commands(&self) -> impl Iterator<Item = Id> + '_ {
        let quick = self.quick.iter().flatten().map(|b| b.0);
        quick.chain(self.build.iter().map(|b| b.0)).chain(TOP_BAR_TEST_MENU)
    }

    /// Every button's icon.
    pub fn icons(&self) -> impl Iterator<Item = Icon> + '_ {
        self.quick.iter().flatten().chain(self.build.iter()).map(|b| b.1)
    }
}

/// The Top Bar, today's menu bar and toolbar in one row (proposal
/// section 5). Today's other toolbar buttons have their homes elsewhere:
/// Properties and New Area in the Module panel's header, Start in View ›
/// Go to Start Location, Resources and Palettes in the Tools menu (each
/// showing its dock's panel), Preview in the Command Palette (Preview
/// Window).
pub const TOP_BAR: TopBar = TopBar {
    quick: [
        Some((Id::NewModule, Icon::New, "New", "New module")),
        Some((Id::OpenModule, Icon::Open, "Open", "Open module")),
        Some((Id::Save, Icon::Save, "Save", "Save module")),
        None,
        Some((Id::Undo, Icon::Undo, "Undo", "Undo")),
        Some((Id::Redo, Icon::Redo, "Redo", "Redo")),
    ],
    build: [
        (Id::CompileAll, Icon::Compile, "Compile", "Compile all scripts"),
        (Id::Verify, Icon::Verify, "Verify", "Verify the module"),
        (Id::TestModule, Icon::Test, "Test Module", "Test the module in the game"),
    ],
};

/// What Test Module's arrow on the Top Bar offers.
pub const TOP_BAR_TEST_MENU: [Id; 1] = [Id::TestChoose];

impl Id {
    /// The Wizards menu's command that makes a new blueprint of `kind`:
    /// the Creature Wizard for creatures, a blueprint wizard otherwise.
    pub(crate) fn new_blueprint(kind: BlueprintKind) -> Id {
        if kind == BlueprintKind::Creature { Id::CreatureWizard } else { Id::Wizard(kind) }
    }

    /// Every command, in the menus' order, then those no menu has.
    pub fn all() -> Vec<Id> {
        fn walk(items: &[Item], out: &mut Vec<Id>) {
            for item in items {
                match item {
                    Do(id) | Item::WithPlugins(id, _) => out.push(*id),
                    Sub(_, items) => walk(items, out),
                    Item::Wizards => {
                        out.extend(crate::blueprint_wizard::KINDS.into_iter().map(Id::Wizard));
                    }
                    Separator
                    | Item::Recent
                    | Item::Prefabs
                    | Item::PluginCommands
                    | Item::PluginWizards
                    | Item::PluginPanels
                    | Item::Project => {}
                }
            }
        }
        let mut out = Vec::new();
        for (_, items) in MENUS {
            walk(items, &mut out);
        }
        out.push(Id::FullScreen);
        out.extend([Id::CloseTab, Id::NextTab, Id::PreviousTab, Id::ReopenTab]);
        out.push(Id::MaximizeWindow);
        out.extend([Id::NextPane, Id::PreviousPane]);
        out
    }

    /// The id its keys are kept under in the settings (never to change),
    /// its name in a menu (with `…` where a window follows), and what its
    /// tip says, if it has one.
    fn text(
        self,
    ) -> (std::borrow::Cow<'static, str>, std::borrow::Cow<'static, str>, &'static str) {
        let (id, label, hint) = match self {
            Id::NewModule => ("new-module", "New Module…", ""),
            Id::OpenModule => {
                ("open-module", "Open Module…", "A .mod file (a module folder: Open Folder…)")
            }
            Id::OpenFolder => {
                ("open-folder", "Open Folder…", "A module folder, or a nasher project")
            }
            Id::Save => ("save", "Save", ""),
            Id::SaveAs => ("save-as", "Save As…", ""),
            Id::SaveAsProject => (
                "save-as-project",
                "Save As nasher Project…",
                "Keep the module as text files for version control",
            ),
            Id::Import => ("import", "Import…", ""),
            Id::ImportFiles => (
                "import-files",
                "Import Files…",
                "Loose files (name.ext), as Export as Files writes them",
            ),
            Id::ImportFolder => {
                ("import-folder", "Import Folder…", "Every file in a folder and its subfolders")
            }
            Id::Export => ("export", "Export…", ""),
            Id::ImportConversation => (
                "import-conversation",
                "Import Conversation…",
                "A Twine (.twee) or Ink (.ink) story, as a new conversation",
            ),
            Id::Close => ("close", "Close", ""),
            Id::Exit => ("exit", "Exit", ""),
            Id::Undo => ("undo", "Undo", ""),
            Id::Redo => ("redo", "Redo", ""),
            Id::ModuleProperties => ("module-properties", "Module Properties", ""),
            Id::BuilderCode => (
                "builder-code",
                "Builder Code…",
                "Your own code, which new ResRefs begin with (kept out of version control)",
            ),
            Id::ClaimsTable => (
                "claims-table",
                "Claims Table…",
                "Who has claimed each area: claim, release, hand on or take over one",
            ),
            Id::ResizeArea => ("resize-area", "Resize Area…", ""),
            Id::RotateArea => ("rotate-area", "Rotate Area…", ""),
            Id::ChangeTileset => ("change-tileset", "Change Tileset…", ""),
            Id::FindInstance => ("find-instance", "Find Instance…", ""),
            Id::EditAreasTogether => (
                "edit-areas-together",
                "Edit Areas Together…",
                "Set the lighting, fog, weather, music and scripts of several areas at once",
            ),
            Id::UpdateAllInstances => (
                "update-all-instances",
                "Update All Instances…",
                "Remake every placed object from its blueprint, where it stands. Lists them first.",
            ),
            Id::ReplaceText => (
                "replace-text",
                "Find and Replace Text…",
                "In names, descriptions, conversations and the journal",
            ),
            Id::FindReferences => (
                "find-references",
                "Find References…",
                "Where a script, area, conversation, blueprint or tag is used",
            ),
            Id::AdvancedSearch => (
                "advanced-search",
                "Advanced Search…",
                "Find blueprints and placed objects by their variables, properties, classes or any \
                 field",
            ),
            Id::IndentToTabs => (
                "convert-indent-tabs",
                "Convert Indentation to Tabs",
                "Make each line's indentation tabs, in the script in front",
            ),
            Id::IndentToSpaces => (
                "convert-indent-spaces",
                "Convert Indentation to Spaces",
                "Make each line's indentation spaces, in the script in front",
            ),
            Id::AreaWizard => ("area-wizard", "Area Wizard…", ""),
            Id::CreatureWizard => ("creature-wizard", "Creature Wizard…", ""),
            Id::Wizard(kind) => {
                let what = kind.label().trim_end_matches('s');
                let id = format!("{}-wizard", what.to_ascii_lowercase().replace(' ', "-"));
                return (id.into(), format!("{what} Wizard…").into(), "");
            }
            Id::NewConversation => ("new-conversation", "New Conversation…", ""),
            Id::Factions => ("factions", "Faction Editor", ""),
            Id::Journal => ("journal", "Journal Editor", ""),
            Id::TalkTable => (
                "talk-table",
                "Talk Table",
                "The module's own talk table (text named by StrRef), or a .tlk file",
            ),
            Id::OpenTable => (
                "open-2da",
                "2DA File…",
                "Edit one of the module's or project's 2DAs, or a .2da file anywhere",
            ),
            Id::NewScript => ("new-script", "New Script…", ""),
            Id::NewNui => ("new-nui", "New NUI Window…", "Create a game UI window and its scripts"),
            Id::Palettes => {
                ("palettes", "Palettes", "The blueprint and tile palettes, in the Right Dock")
            }
            Id::PlaceableGallery => (
                "placeable-gallery",
                "Appearance Gallery…",
                "Every placeable, creature and door appearance as a picture, to give to the \
                 selection",
            ),
            Id::Resources => (
                "resources",
                "Resource Browser",
                "Every resource of the load order, in the Left Dock",
            ),
            Id::NewTileset => ("new-tileset", "New Tileset…", "A new .set file to fill"),
            Id::OpenTileset => ("open-tileset", "Open Tileset…", ""),
            Id::NewHak => ("new-hak", "New Hak", ""),
            Id::OpenHak => ("open-hak", "Open Hak…", ""),
            Id::BuildHak => (
                "build-hak",
                "Build Hak from Folder…",
                "A new hak with a folder's files, to look over and save",
            ),
            Id::ReloadResources => (
                "reload-resources",
                "Reload Resources",
                "Read again the haks, override and development folders that changed",
            ),
            Id::Options => ("options", "Options…", ""),
            Id::CompileAll => ("compile-all", "Compile All Scripts", ""),
            Id::CompileModels => (
                "compile-models",
                "Compile Models",
                "Compile the module's ASCII models against their supermodels",
            ),
            Id::BuildModule => ("build-module", "Build Module…", ""),
            Id::PublishNwsync => (
                "publish-nwsync",
                "Publish to NWSync…",
                "The module's haks and talk table, for players' games to download",
            ),
            Id::Verify => ("verify", "Verify Module", ""),
            Id::RoundTrip => (
                "round-trip",
                "Round-Trip Check",
                "Write each GFF and 2DA of a copy anew and log what changes. The module is \
                 untouched.",
            ),
            Id::GoToStart => (
                "go-to-start",
                "Go to Start Location",
                "Open the area the module starts in, at the start location",
            ),
            Id::TestModule => ("test-module", "Test Module", ""),
            Id::TestChoose => (
                "test-choose",
                "Test Module, Choose Character",
                "The game asks which character to play",
            ),
            Id::PackTarget => (
                "pack-target",
                "Pack the Project's Module",
                "Write the nasher project's module file, as nasher packs it",
            ),
            Id::TakeInLeftOut => (
                "take-in-left-out",
                "Add Left-Out Files to nasher.cfg",
                "Add include lines for the source files nasher.cfg leaves out, and read them",
            ),
            Id::AreaStatistics => ("area-statistics", "Area Statistics", ""),
            Id::Plugins => (
                "plugins",
                "Manage Plugins…",
                "The plugins installed: enable them, and try the plugin console",
            ),
            Id::InstallPlugin => (
                "install-plugin",
                "Install Plugin from File…",
                "Install a plugin from its zip archive. It stays off until you enable it.",
            ),
            Id::Manual => ("manual", "User Manual", ""),
            Id::CommandPalette => {
                ("command-palette", "Command Palette…", "Find a command by its name and run it")
            }
            Id::CheckUpdates => (
                "check-updates",
                "Check for Updates…",
                "Ask GitHub whether a newer Moonglow is out (nothing is downloaded)",
            ),
            Id::About => ("about", "About Moonglow Toolset", ""),
            // (The Preview window's id, and so a key given it, as before.)
            Id::PreviewWindow => (
                "preview-window",
                "Preview",
                "The blueprint chosen in the palette, in the Right Dock",
            ),
            Id::FullScreen => ("full-screen", "Full Screen", ""),
            Id::CloseTab => ("close-tab", "Close Tab", "The tab in front"),
            Id::MaximizeWindow => (
                "maximize-window",
                "Maximize Window",
                "Fill Moonglow's window with the tab in front, or restore it",
            ),
            Id::NextTab => ("next-tab", "Next Tab", ""),
            Id::PreviousTab => ("previous-tab", "Previous Tab", ""),
            Id::ReopenTab => ("reopen-tab", "Reopen Closed Tab", "The tab closed last"),
            Id::NextPane => (
                "next-pane",
                "Next Pane",
                "Give the arrow keys to the next pane: a dock's panel or the editor area",
            ),
            Id::PreviousPane => {
                ("previous-pane", "Previous Pane", "Give the arrow keys to the pane before")
            }
            // (The Left Dock's, since it holds the Resources panel beside
            // the module tree: its id, and so a key given it, as before.)
            Id::ViewTree => ("view-tree", "Left Dock", "Show the Left Dock, or fold it away"),
            // (The Right Dock's, since the palettes are its Palette panel:
            // its id, and so a key given it, as before.)
            Id::ViewPalettes => {
                ("view-palettes", "Right Dock", "Show the Right Dock, or fold it away")
            }
            Id::ViewLog => ("view-log", "Log", "Show the log, or fold it away"),
            Id::CopyLog => {
                ("copy-log", "Copy Log", "The whole log to the clipboard, each line with its time")
            }
            Id::SaveLog => ("save-log", "Save Log…", "The whole log into a text file"),
            Id::ClearLog => ("clear-log", "Clear Log", "Empty the log"),
            Id::HidePanels => (
                "hide-panels",
                "Hide All Panels",
                "Fold the tree, the palettes and the log away, or bring them back",
            ),
            Id::ResetLayout => (
                "reset-layout",
                "Reset Layout",
                "Put the tree, the palettes and the log back where they started",
            ),
        };
        (id.into(), label.into(), hint)
    }

    /// What its tip says beside its name (empty where it has none).
    pub(crate) fn hint(self) -> &'static str {
        self.text().2
    }

    /// Its name, as Options › Keyboard lists it.
    pub fn name(self) -> String {
        self.text().1.trim_end_matches('…').to_string()
    }

    /// The id its keys are kept under in the settings.
    pub fn id(self) -> String {
        self.text().0.into_owned()
    }

    /// Its name in a menu as things are: Undo names what it would undo, a
    /// project's Pack its file.
    pub(crate) fn label(self, app: &Moonglow) -> String {
        let ws = app.ws.as_ref();
        match self {
            // (A script's step, with the keys in its editor: not the
            // module's to name, nor one it has a name for.)
            Id::Undo | Id::Redo if app.undo_script.is_some() => self.text().1.into_owned(),
            // (A side editor's, with the pointer over it or the keys in its
            // grid: named where the editor names its steps, a hak's and a
            // 2DA's.)
            Id::Undo | Id::Redo if app.local_can_undo(self == Id::Redo).is_some() => {
                match app.local_step_name(self == Id::Redo) {
                    Some(step) => format!("{} {step}", self.text().1),
                    None => self.text().1.into_owned(),
                }
            }
            Id::Undo => match ws.and_then(|ws| ws.can_undo()) {
                Some(what) => format!("Undo {what}"),
                None => "Undo".into(),
            },
            Id::Redo => match ws.and_then(|ws| ws.can_redo()) {
                Some(what) => format!("Redo {what}"),
                None => "Redo".into(),
            },
            Id::PackTarget => match ws.and_then(|ws| ws.module.project.as_ref()) {
                Some(p) => format!("Pack {}", p.target().file),
                None => self.text().1.into_owned(),
            },
            Id::HidePanels if app.panels_hidden() => "Show All Panels".into(),
            _ => self.text().1.into_owned(),
        }
    }

    /// Whether its menu row is ticked (a panel shown), where it is one
    /// that can be.
    fn ticked(self, app: &Moonglow) -> Option<bool> {
        match self {
            // (The Right Dock not drawn, its row grayed: not ticked.)
            Id::ViewPalettes => {
                Some(app.panel_shown(self) && crate::shell::right_shown(app).is_some())
            }
            Id::ViewTree | Id::ViewLog => Some(app.panel_shown(self)),
            _ => None,
        }
    }

    /// Whether it is offered at all (a project's command without a
    /// project is not).
    fn shown(self, app: &Moonglow) -> bool {
        match self {
            Id::PackTarget | Id::TakeInLeftOut => {
                app.ws.as_ref().is_some_and(|ws| ws.module.project.is_some())
            }
            _ => true,
        }
    }

    /// Whether it can be chosen now.
    pub(crate) fn enabled(self, app: &Moonglow) -> bool {
        let open = app.ws.is_some();
        match self {
            Id::NewModule
            | Id::OpenModule
            | Id::OpenFolder
            | Id::Exit
            | Id::Palettes
            | Id::PlaceableGallery
            | Id::Resources
            | Id::NewTileset
            | Id::OpenTileset
            | Id::NewHak
            | Id::OpenHak
            | Id::OpenTable
            | Id::BuildHak
            | Id::TalkTable
            | Id::ReloadResources
            | Id::Options
            | Id::Plugins
            | Id::Manual
            | Id::CommandPalette
            | Id::CheckUpdates
            | Id::About
            | Id::PreviewWindow
            | Id::FullScreen
            | Id::NextTab
            | Id::PreviousTab => true,
            // (The docks' panels too: the Resources panel or the Palette
            // panel without a module.)
            Id::NextPane | Id::PreviousPane => {
                open || crate::shell::left_shown(app).is_some()
                    || crate::shell::right_shown(app).is_some()
            }
            Id::ViewLog | Id::ResetLayout => true,
            Id::CopyLog | Id::SaveLog | Id::ClearLog => !app.log.entries.is_empty(),
            // (Whenever the Left Dock holds a panel: its Resources panel
            // without a module too.)
            Id::ViewTree => crate::shell::left_shown(app).is_some(),
            Id::HidePanels => open,
            // (Whenever the Right Dock holds a panel: the Palette panel
            // without a module too, once asked for.)
            Id::ViewPalettes => crate::shell::right_shown(app).is_some(),
            Id::CloseTab => app.front_tab().is_some(),
            Id::MaximizeWindow => app.front_window_tab().is_some(),
            Id::ReopenTab => !app.closed_tabs.is_empty(),
            Id::InstallPlugin => app.plugin_dir.is_some(),
            // (A script's, with the keys in its editor: as its Ctrl+Z,
            // which does nothing with nothing to undo. Whether its history
            // has a step is not known without a copy of it all.)
            Id::Undo | Id::Redo if app.undo_script.is_some() => true,
            // (A side editor's own or the talk table's, with the pointer
            // over it or the keys in its grid.)
            Id::Undo => app
                .local_can_undo(false)
                .unwrap_or_else(|| app.ws.as_ref().is_some_and(|ws| ws.can_undo().is_some())),
            Id::Redo => app
                .local_can_undo(true)
                .unwrap_or_else(|| app.ws.as_ref().is_some_and(|ws| ws.can_redo().is_some())),
            Id::ResizeArea | Id::RotateArea | Id::ChangeTileset | Id::AreaStatistics => {
                shown_area(app).is_some()
            }
            Id::IndentToTabs | Id::IndentToSpaces => script_in_front(app).is_some(),
            Id::PackTarget => self.shown(app),
            // (A project's or a module folder's: a .mod's folder holds
            // other modules too.)
            Id::BuilderCode => crate::builder_code::root(app).is_some(),
            Id::ClaimsTable => crate::claims::kept(app),
            Id::TakeInLeftOut => (app.ws.as_ref())
                .and_then(|ws| ws.module.project.as_ref())
                .is_some_and(|p| !p.left_out.is_empty()),
            Id::Save
            | Id::SaveAs
            | Id::SaveAsProject
            | Id::Import
            | Id::ImportFiles
            | Id::ImportFolder
            | Id::Export
            | Id::ImportConversation
            | Id::Close
            | Id::ModuleProperties
            | Id::FindInstance
            | Id::EditAreasTogether
            | Id::UpdateAllInstances
            | Id::ReplaceText
            | Id::FindReferences
            | Id::AdvancedSearch
            | Id::AreaWizard
            | Id::CreatureWizard
            | Id::Wizard(_)
            | Id::NewConversation
            | Id::Factions
            | Id::Journal
            | Id::NewScript
            | Id::NewNui
            | Id::CompileAll
            | Id::CompileModels
            | Id::BuildModule
            | Id::PublishNwsync
            | Id::Verify
            | Id::RoundTrip
            | Id::GoToStart
            | Id::TestModule
            | Id::TestChoose => open,
        }
    }

    /// Does it.
    pub(crate) fn run(self, app: &mut Moonglow, ctx: &egui::Context) {
        let action = match self {
            Id::NewModule => Action::NewModuleDialog,
            Id::OpenModule => Action::OpenModuleDialog,
            Id::OpenFolder => Action::OpenFolderDialog,
            Id::Save => Action::Save,
            Id::SaveAs => Action::SaveAsDialog,
            Id::SaveAsProject => Action::SaveAsProjectDialog,
            Id::Import => Action::ImportDialog,
            Id::ImportFiles => Action::ImportFilesDialog,
            Id::ImportFolder => Action::ImportFolderDialog,
            Id::Export => Action::ExportDialog(Vec::new()),
            Id::Close => Action::Close,
            Id::Exit => Action::Quit,
            // (With the keys in a script's editor, or a menu opened from
            // it, its own step: `script_view::undo_target`.)
            Id::Undo | Id::Redo if let Some(key) = app.undo_script => {
                crate::script_view::undo(app, ctx, key, self == Id::Redo);
                return;
            }
            Id::Undo => Action::Undo,
            Id::Redo => Action::Redo,
            Id::ModuleProperties => Action::OpenTab(Tab::ModuleProperties),
            Id::FindReferences => Action::OpenTab(Tab::References),
            Id::AdvancedSearch => Action::OpenTab(Tab::AdvancedSearch),
            Id::AreaWizard => Action::AreaWizard,
            Id::Factions => Action::OpenTab(Tab::Factions),
            Id::Journal => Action::OpenTab(Tab::Journal),
            Id::TalkTable => Action::OpenTab(Tab::TalkTable),
            Id::Palettes => Action::ShowPanel(crate::Panel::Palette),
            Id::PreviewWindow => Action::ShowPanel(crate::Panel::Preview),
            Id::PlaceableGallery => Action::PlaceableGallery,
            Id::Resources => Action::ShowPanel(crate::Panel::Resources),
            Id::ReloadResources => Action::ReloadResources,
            Id::Options => Action::OptionsDialog,
            Id::CompileAll => Action::CompileScripts,
            Id::CompileModels => Action::CompileModels,
            Id::Verify => Action::Verify,
            Id::RoundTrip => Action::RoundTrip,
            // (The games running looked at before the save writes the
            // module under one.)
            Id::TestModule | Id::TestChoose if !app.test_may_start() => return,
            Id::TestModule => Action::SaveThen(Box::new(Action::TestModule)),
            Id::TestChoose => Action::SaveThen(Box::new(Action::TestModuleChoose)),
            Id::PackTarget => Action::PackTarget,
            Id::TakeInLeftOut => Action::TakeInLeftOut,
            Id::Manual => Action::OpenTab(Tab::Manual),
            // The rest open a window of their own, or change what shows.
            Id::ImportConversation => {
                if let Some(path) = app.dialogs.open_file(crate::dialogs::FileKind::Story, None) {
                    app.import_conversation(&path);
                }
                return;
            }
            Id::ResizeArea => {
                if let Some(a) = shown_area(app) {
                    crate::area_reshape::open_resize(app, a);
                }
                return;
            }
            Id::RotateArea => {
                if let Some(a) = shown_area(app) {
                    app.rotate_area = Some(crate::area_reshape::RotateDraft { area: a, turns: 1 });
                }
                return;
            }
            Id::ChangeTileset => {
                if let Some(a) = shown_area(app) {
                    crate::area_reshape::open_change_tileset(app, a);
                }
                return;
            }
            Id::FindInstance => {
                app.find_instance.get_or_insert_with(Default::default);
                return;
            }
            Id::EditAreasTogether => {
                app.area_chooser.get_or_insert_with(Default::default);
                return;
            }
            Id::UpdateAllInstances => return app.update_all_instances(),
            Id::ReplaceText => {
                app.text_replace.get_or_insert_with(Default::default);
                return;
            }
            Id::IndentToTabs | Id::IndentToSpaces => {
                let tabs = self == Id::IndentToTabs;
                // (The lines selected, where the script had the keys as the
                // menu was opened: the selection was there to see.)
                let keys = app.undo_script;
                app.script_tools.convert_indent =
                    script_in_front(app).map(|k| (k, tabs, keys == Some(k)));
                return;
            }
            Id::CreatureWizard => {
                app.creature_wizard = Some(Default::default());
                return;
            }
            Id::BuilderCode => return crate::builder_code::open(app),
            Id::ClaimsTable => return crate::claims::open_table(app),
            Id::Wizard(kind) => {
                app.blueprint_wizard = Some(crate::blueprint_wizard::BlueprintWizard::new(kind));
                return;
            }
            // (Their names start with the builder's code.)
            Id::NewConversation => {
                app.new_dialog = Some(crate::widgets::code_start(app, mg_core::ResType::DLG));
                return;
            }
            Id::NewScript => {
                app.new_script = Some(crate::widgets::code_start(app, mg_core::ResType::NSS));
                return;
            }
            Id::NewNui => {
                app.new_nui = Some(crate::widgets::code_start(app, mg_core::ResType::JUI));
                return;
            }
            Id::NewTileset => return crate::tileset_view::new_tileset(app),
            Id::OpenTileset => return crate::tileset_view::open(app),
            Id::NewHak => return crate::hak_view::new_hak(app),
            Id::OpenHak => return crate::hak_view::open_hak(app),
            Id::OpenTable => return crate::table_view::open_table_window(app),
            Id::BuildHak => return crate::hak_view::build_from_folder(app),
            Id::BuildModule => {
                app.build.get_or_insert_with(Default::default);
                return;
            }
            Id::PublishNwsync => return crate::nwsync_view::open(app),
            Id::AreaStatistics => {
                app.area_stats = shown_area(app);
                return;
            }
            Id::Plugins => {
                app.plugins.window = true;
                return;
            }
            Id::InstallPlugin => return app.install_plugin(),
            Id::CommandPalette => {
                app.command_palette = Some(Finder::default());
                return;
            }
            Id::About => {
                app.about = true;
                return;
            }
            Id::CheckUpdates => return crate::updates::check_now(app),
            Id::GoToStart => {
                crate::area_view::go_to_start(app);
                return;
            }
            Id::FullScreen => {
                let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
                return;
            }
            Id::CloseTab => match app.front_tab() {
                Some(tab) => Action::CloseTabs(vec![tab]),
                None => return,
            },
            Id::MaximizeWindow => match app.front_window_tab() {
                Some(tab) => Action::ToggleMaximize(tab),
                None => return,
            },
            Id::NextTab => return app.step_tab(1),
            Id::PreviousTab => return app.step_tab(-1),
            Id::ReopenTab => return app.reopen_tab(),
            Id::NextPane => return app.step_pane(ctx, 1),
            Id::PreviousPane => return app.step_pane(ctx, -1),
            Id::ViewTree | Id::ViewPalettes | Id::ViewLog => {
                let shown = app.panel_shown(self);
                return app.show_panel(self, !shown);
            }
            Id::HidePanels => {
                let show = app.panels_hidden();
                // (A Right Dock not drawn keeps its fold for the next area.)
                let right = crate::shell::right_shown(app).is_some();
                for panel in [Id::ViewTree, Id::ViewPalettes, Id::ViewLog] {
                    if panel != Id::ViewPalettes || right {
                        app.show_panel(panel, show);
                    }
                }
                return;
            }
            Id::ResetLayout => return app.reset_layout(ctx),
            Id::CopyLog => return app.copy_log(ctx),
            Id::SaveLog => return app.save_log(),
            Id::ClearLog => return app.log.clear(),
        };
        app.actions.push(action);
    }
}

/// The area the commands about "the area" mean: the one shown last, while
/// its view is open.
fn shown_area(app: &Moonglow) -> Option<mg_core::ResRef> {
    app.palette.area.filter(|a| app.area_views.contains_key(a))
}

/// The script the script commands mean: the one whose editor is the tab
/// in front, else (a script in a window of its own) the one whose editor
/// had the keyboard last, else the only one open.
fn script_in_front(app: &Moonglow) -> Option<mg_resman::ResKey> {
    let open = |k: &mg_resman::ResKey| app.scripts.contains_key(k);
    match app.front_tab() {
        Some(Tab::Script(k)) if open(&k) => Some(k),
        _ => app.script_tools.last_typed.filter(open).or_else(|| {
            let mut keys = app.scripts.keys();
            keys.next().filter(|_| keys.next().is_none()).copied()
        }),
    }
}

/// The name of the command with the id `id`, as Options › Keyboard lists
/// it: what a key command that works anywhere is called
/// ([`keys::Cmd::name`]). (The names by id, made once; the id itself for
/// one that is no command.)
pub(crate) fn general_name(id: &'static str) -> &'static str {
    static NAMES: std::sync::OnceLock<std::collections::HashMap<String, String>> =
        std::sync::OnceLock::new();
    let names = NAMES.get_or_init(|| Id::all().into_iter().map(|c| (c.id(), c.name())).collect());
    names.get(id).map_or(id, String::as_str)
}

/// A command's or a key command's name, by the id its keys are kept under.
pub fn name_of(id: &str) -> String {
    Id::all()
        .into_iter()
        .find(|c| c.id() == id)
        .map(Id::name)
        .or_else(|| keys::Cmd::from_id(id).map(|c| c.name().to_string()))
        .unwrap_or_else(|| id.to_string())
}

/// A command asked for by its key while a text field that sets its text
/// when the keyboard leaves it held what was typed: the keyboard leaves
/// it, and the command is run a frame on, with what was typed set.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AfterText {
    /// One of the window's own.
    Command(Id),
    /// A plugin's (it reads the module as the command begins).
    Plugin { plugin: String, command: String },
    /// The project's (`moonglow.toml`'s `[commands]`).
    Project(String),
}

/// Asks for `after` to be run now, or a frame on if a text field typed in
/// holds the keyboard (see [`AfterText`]).
fn run_after_text(app: &mut Moonglow, ui: &Ui, after: AfterText) {
    if crate::widgets::leave_text(ui.ctx()) {
        app.command_after_text = Some(after);
        ui.ctx().request_repaint();
    } else {
        run_after(app, ui, after);
    }
}

fn run_after(app: &mut Moonglow, ui: &Ui, after: AfterText) {
    match after {
        AfterText::Command(id) => {
            if id.enabled(app) {
                id.run(app, ui.ctx());
            }
        }
        // (Begun with the frame's actions, after the fields are drawn:
        // egui says a field lost the keyboard in two frames, and one whose
        // text a plugin's edits changed before it was drawn would set what
        // was typed again, over them.)
        AfterText::Plugin { plugin, command } => {
            if app.ws.is_some() {
                let selected = app.plugin_selection();
                let values = Default::default();
                app.actions.push(Action::RunPluginCommand { plugin, command, selected, values });
            }
        }
        AfterText::Project(name) => app.actions.push(Action::ProjectCommand(name)),
    }
}

/// The commands whose key was pressed, run (a key pressed for a command
/// that cannot be chosen now is still used up).
pub(crate) fn keys_pressed(app: &mut Moonglow, ui: &Ui) {
    // (A letter given to a command stays a letter in a text field.)
    let typing = ui.memory(|m| m.focused().is_some());
    // (And Undo and Redo are the text field's that has been typed in since
    // it got the keyboard: its own typing is taken back, not the module's
    // last change, which may be in an area nobody is looking at. GitHub
    // issue 18. A field only stepped into, or left, has nothing of its
    // own to take back: there they are the module's.)
    let focused = ui.memory(|m| m.focused());
    if app.text_typed.is_some() && app.text_typed != focused {
        app.text_typed = None;
    }
    let edits = ui.ctx().egui_wants_keyboard_input()
        && ui.input(|i| {
            i.events.iter().any(|e| match e {
                egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Cut => true,
                egui::Event::Key { key, pressed: true, .. } => {
                    matches!(key, egui::Key::Backspace | egui::Key::Delete)
                }
                _ => false,
            })
        });
    if edits {
        app.text_typed = focused;
    }
    // (A script's editor keeps them whenever it has the keyboard: Enter's
    // indent, Tab, a completion and a replacement are its steps as much
    // as typing is, and it has them still after the keyboard went to its
    // Find field and came back.)
    let in_text = (app.text_typed.is_some() && ui.ctx().egui_wants_keyboard_input())
        || crate::script_view::has_keys(app, ui.ctx());
    // A command kept from the frame before (see below): run now.
    if let Some(after) = app.command_after_text.take() {
        run_after(app, ui, after);
    }
    // (Undo and Redo's keys of Options › Keyboard in a script's editor:
    // its own history's, as the menu's are. Taken before the editor is
    // drawn, so that its own Ctrl+Z doesn't take a second step.)
    let script = crate::script_view::with_keys(app, ui.ctx());
    for id in Id::all() {
        if let (Some(key), Id::Undo | Id::Redo) = (script, id) {
            if ui.input_mut(|i| app.keymap.consume_outside_text(i, &id.id(), typing)) {
                crate::script_view::undo(app, ui.ctx(), key, id == Id::Redo);
            }
            continue;
        }
        if in_text && matches!(id, Id::Undo | Id::Redo) {
            continue;
        }
        let pressed = ui.input_mut(|i| app.keymap.consume_outside_text(i, &id.id(), typing));
        // The Save key in a script editor that has the keyboard is that
        // script's Save (as Aurora's script editor's Ctrl+S): its text
        // into the module, not the module to its file (El Grillo's
        // request). File › Save is the module's still.
        if pressed
            && id == Id::Save
            && let Some(script) = crate::script_view::with_keys(app, ui.ctx())
        {
            app.actions.push(crate::Action::SaveScript(script));
            continue;
        }
        if pressed && id.enabled(app) {
            // With a text field typed in that sets its text when the
            // keyboard leaves it, the keyboard leaves it first and the
            // command is run a frame on: one that closes the field's tab,
            // goes to another or saves would else be done without what
            // was typed.
            run_after_text(app, ui, AfterText::Command(id));
        }
    }
    // The plugins' commands, after the window's own: a key both have is
    // the window's. (They too wait for a field typed in to set its text:
    // a plugin reads the module as its command begins.)
    for c in app.plugin_commands() {
        let pressed = ui.input_mut(|i| app.keymap.consume_outside_text(i, &c.key_id, typing));
        if pressed && app.ws.is_some() {
            let after = AfterText::Plugin { plugin: c.plugin, command: c.command };
            run_after_text(app, ui, after);
        }
    }
    // And the project's.
    let project = crate::project_commands::commands(app).map(|(_, c)| c).unwrap_or_default();
    for c in project {
        let id = crate::project_commands::key_id(&c.name);
        let pressed = ui.input_mut(|i| app.keymap.consume_outside_text(i, &id, typing));
        if pressed && c.run.is_some() {
            run_after_text(app, ui, AfterText::Project(c.name));
        }
    }
}

/// The menus by the keyboard (GitHub issue 9): which row of the open menu
/// the keys are at, and of its submenu if one is open.
#[derive(Debug, Clone, Default)]
pub struct MenuKeys {
    /// The row the keys are at, in the menu (`[row]`) or in a submenu of
    /// it (`[row of the submenu's entry, row in it]`). Empty: the pointer
    /// has the menus, and no row is marked.
    path: Vec<usize>,
    /// Each level's rows as last drawn: whether each can be chosen.
    rows: Vec<Vec<bool>>,
    /// And the letter each begins with (lower case), for a letter typed.
    initials: Vec<Vec<char>>,
    /// Enter or Space was pressed: the marked row is chosen as it is drawn.
    choose: bool,
    /// Right was pressed: a submenu's entry opens it.
    into: bool,
    /// Alt went down, and nothing else has been pressed since.
    alt_alone: bool,
    /// Left or Escape left a submenu: its entry closes it as it is drawn.
    close_sub: bool,
}

impl MenuKeys {
    /// Whether the keys are at row `row` of `level`.
    fn at(&self, level: usize, row: usize) -> bool {
        self.path.len() == level + 1 && self.path[level] == row
    }

    /// The marked row moved by `by`, round the ends, past rows that can't
    /// be chosen.
    fn step(&mut self, by: isize) {
        let Some(level) = self.path.len().checked_sub(1) else { return };
        let Some(rows) = self.rows.get(level).filter(|r| r.iter().any(|on| *on)) else { return };
        let n = rows.len() as isize;
        let mut at = self.path[level] as isize;
        for _ in 0..n {
            at = (at + by).rem_euclid(n);
            if rows[at as usize] {
                break;
            }
        }
        self.path[level] = at as usize;
    }

    /// The marked row moved to the next that begins with `letter`, round
    /// the ends (from the top where none is marked).
    fn seek(&mut self, letter: char) {
        if self.path.is_empty() {
            self.path = vec![usize::MAX];
        }
        let level = self.path.len() - 1;
        let (Some(rows), Some(initials)) = (self.rows.get(level), self.initials.get(level)) else {
            return;
        };
        let n = rows.len();
        let from = self.path[level];
        let found = (1..=n)
            .map(|by| if from == usize::MAX { by - 1 } else { (from + by) % n })
            .find(|at| rows[*at] && initials.get(*at) == Some(&letter));
        match found {
            Some(at) => self.path[level] = at,
            None if from == usize::MAX => self.path[level] = self.first(level),
            None => {}
        }
    }

    /// The first row of a level that can be chosen.
    fn first(&self, level: usize) -> usize {
        self.rows.get(level).and_then(|r| r.iter().position(|on| *on)).unwrap_or(0)
    }
}

/// The menu bar, from [`MENUS`]. By the mouse as egui has it, and: with a
/// menu open, the pointer over another menu's name opens that one. By the
/// keyboard: Alt (pressed and let go alone) or F10 opens the first menu,
/// and closes the menus (F10 over an area's view is Select Tiles there,
/// while that is its key); Left and Right go from menu to menu, round the
/// ends; Up and Down from row to row, past separators and rows that can't
/// be chosen; Enter or Space chooses; Right opens a submenu, Left closes
/// it; Escape closes the submenu the keys are in, else the menus. Alt and
/// a menu's first letter (underlined) opens that menu; with a menu open, a
/// letter goes to the next row that begins with it.
pub(crate) fn menu_bar(app: &mut Moonglow, ui: &mut Ui) {
    use egui::{Key, Modifiers, Popup};
    let ctx = ui.ctx().clone();
    // A letter pressed: with Alt alone held, a menu's (unless a command
    // was given Alt and that letter in Options › Keyboard: the command's
    // then); with a menu open and nothing held, a row's. Taken from the
    // frame's keys, with the text it typed.
    let keymap = &app.keymap;
    let letter = |alt: bool| {
        ctx.input_mut(|i| {
            let m = i.modifiers;
            if m.alt != alt || m.ctrl || m.shift || m.command {
                return None;
            }
            let found = i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, repeat: false, .. }
                    if !(alt && keymap.bound(e)) =>
                {
                    let name = key.name();
                    let c = name.chars().next().filter(|c| name.len() == 1 && c.is_alphabetic())?;
                    Some((*key, c.to_ascii_lowercase()))
                }
                _ => None,
            })?;
            if alt && !MENUS.iter().any(|(name, _)| initial(name) == found.1) {
                return None;
            }
            i.events.retain(|e| match e {
                egui::Event::Key { key, .. } => *key != found.0,
                egui::Event::Text(t) => !t.eq_ignore_ascii_case(&found.1.to_string()),
                _ => true,
            });
            Some(found.1)
        })
    };
    let mnemonic = letter(true);
    // Alt alone: down, then up with no key or button between.
    let (alt_down, other) = ctx.input(|i| {
        // (Alt+Tab to another program and back is not Alt alone: the
        // window lost the keyboard meanwhile.)
        let other = i.pointer.any_pressed()
            || !i.focused
            || i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key { .. } | egui::Event::Text(_) | egui::Event::WindowFocused(_)
                )
            });
        (i.modifiers.alt && i.focused, other)
    });
    let was = std::mem::replace(&mut app.menu_keys.alt_alone, alt_down);
    let alt_released = was && !alt_down && !other && !app.menu_alt_spoiled;
    if !alt_down {
        app.menu_alt_spoiled = false;
    } else if other || ctx.input(|i| i.modifiers.ctrl || i.modifiers.shift) {
        app.menu_alt_spoiled = true;
    }
    // F10 with the pointer over an area's view is that view's while it is
    // Select Tiles' key there (Aurora's), unless a menu is open to close.
    let areas = std::mem::take(&mut app.area_hovered)
        && app.menu_open.is_none()
        && ctx.input(|i| app.keymap.pressed(i, crate::keys::Cmd::SelectTiles));
    let toggle =
        alt_released || (!areas && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F10)));

    let mut tops: Vec<(egui::Id, egui::Response)> = Vec::new();
    let mut keys = std::mem::take(&mut app.menu_keys);
    let was_open = app.menu_open;
    // (Read before the menus are drawn: a row chosen is chosen this frame.)
    let pressed = |key: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    if was_open.is_some() && !ctx.egui_wants_keyboard_input() {
        if pressed(Key::ArrowDown) {
            if keys.path.is_empty() {
                keys.path = vec![keys.first(0)];
            } else {
                keys.step(1);
            }
        }
        if pressed(Key::ArrowUp) {
            if keys.path.is_empty() {
                keys.path = vec![keys.first(0)];
            }
            keys.step(-1);
        }
        keys.choose = pressed(Key::Enter) || pressed(Key::Space);
        keys.into = false;
        if let Some(c) = letter(false) {
            keys.seek(c);
        }
        // Escape in a submenu closes that one; else egui closes the menus.
        if keys.path.len() > 1 && pressed(Key::Escape) {
            keys.path.pop();
            keys.close_sub = true;
        }
    }
    let mut switch: Option<isize> = None;
    if was_open.is_some() {
        if pressed(Key::ArrowLeft) {
            if keys.path.len() > 1 {
                keys.path.pop();
                keys.close_sub = true;
            } else {
                switch = Some(-1);
            }
        }
        if pressed(Key::ArrowRight) {
            // (A submenu's entry takes it, as it is drawn; else the next menu.)
            keys.into = true;
        }
    }
    app.menu_keys = keys;
    for level in &mut app.menu_keys.rows {
        level.clear();
    }
    for level in &mut app.menu_keys.initials {
        level.clear();
    }

    // (The suite's menus: titles hovered `raised`, open and rows hovered
    // `accent-fill`.)
    let config = egui::containers::menu::MenuConfig::new().style(mg_uikit::menu_style);
    // (egui's menu bar takes the width it is given: given the menus' own,
    // as they took it in the last frame, the Top Bar's buttons follow
    // them. When that changes, the frame is drawn again.)
    let width_id = ui.id().with("menu-bar-width");
    let start = ui.cursor().min.x;
    let width = ui.data(|d| d.get_temp::<f32>(width_id)).unwrap_or(ui.available_width());
    let size = egui::vec2(width.min(ui.available_width()), ui.available_height());
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    ui.allocate_ui_with_layout(size, layout, |ui| {
        egui::MenuBar::new().style(mg_uikit::menu_bar_style).config(config).ui(ui, |ui| {
            for (name, items) in MENUS {
                // (Its first letter underlined: Alt and the letter opens it.)
                let mut job = egui::text::LayoutJob::default();
                let font = egui::TextStyle::Button.resolve(ui.style());
                // (The button's own text color, which its state gives: on the
                // open menu's `accent-fill`, `on-fill`.)
                let color = egui::Color32::PLACEHOLDER;
                let plain = egui::TextFormat { font_id: font, color, ..Default::default() };
                let under =
                    egui::TextFormat { underline: egui::Stroke::new(1.0, color), ..plain.clone() };
                let split = name.chars().next().map_or(0, char::len_utf8);
                job.append(&name[..split], 0.0, under);
                job.append(&name[split..], 0.0, plain);
                let (response, _) = egui::containers::menu::MenuButton::new(job).ui(ui, |ui| {
                    app.menu_row = vec![0];
                    menu(app, ui, items, 0);
                });
                tops.push((Popup::default_response_id(&response), response));
            }
        });
    });
    if let Some((_, last)) = tops.last() {
        let measured = (last.rect.right() - start).ceil();
        if (measured - width).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(width_id, measured));
            ui.ctx().request_discard("the menu bar's width changed");
        }
    }

    let open = tops.iter().position(|(id, _)| Popup::is_id_open(&ctx, *id));
    let n = tops.len() as isize;
    let mut go = None;
    let by_letter = mnemonic.and_then(|c| MENUS.iter().position(|(name, _)| initial(name) == c));
    match open {
        Some(now) => {
            // Right, on a row that is no submenu's: the next menu.
            if std::mem::take(&mut app.menu_keys.into) {
                switch = Some(1);
            }
            if let Some(by) = switch {
                go = Some((now as isize + by).rem_euclid(n) as usize);
            }
            // The pointer over another menu's name opens it.
            let hovered = tops.iter().position(|(_, r)| r.hovered());
            if let Some(h) = hovered.filter(|h| *h != now)
                && ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
            {
                go = Some(h);
                app.menu_keys.path.clear();
            }
            if toggle {
                Popup::close_all(&ctx);
                app.menu_keys.path.clear();
                go = None;
            }
            // (The pointer moved over the menu: it has the rows again.)
            if ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO) && switch.is_none() {
                app.menu_keys.path.clear();
            }
        }
        None => {
            app.menu_keys.path.clear();
            if toggle && !Popup::is_any_open(&ctx) {
                go = Some(0);
            }
        }
    }
    if let Some(to) = by_letter.filter(|to| Some(*to) != open) {
        go = Some(to);
    }
    if let Some(to) = go {
        Popup::open_id(&ctx, tops[to].0);
        // (Opened by a key: its first row marked, once it has been drawn.)
        let keyed = switch.is_some() || toggle || by_letter.is_some();
        app.menu_keys.path = if keyed { vec![usize::MAX] } else { Vec::new() };
        ctx.request_repaint();
    }
    // A menu opened by a key marks its first row that can be chosen.
    if app.menu_keys.path == [usize::MAX] && open.is_some() && go.is_none() {
        app.menu_keys.path = vec![app.menu_keys.first(0)];
        ctx.request_repaint();
    }
    app.menu_keys.choose = false;
    app.menu_keys.close_sub = false;
    app.menu_open = tops.iter().position(|(id, _)| Popup::is_id_open(&ctx, *id));
    // A menu opened from a script's editor and closed with nothing chosen
    // (Escape, Alt or F10, a click on its name) gives the keys back to the
    // editor, so that Ctrl+Z after it is the script's still. (A click
    // elsewhere is a click outside the editor: the keys stay nowhere.)
    let unused = ctx.input(|i| i.key_pressed(Key::Escape))
        || toggle
        || tops.iter().any(|(_, r)| r.clicked());
    if was_open.is_some() && app.menu_open.is_none() && unused {
        crate::script_view::give_keys_back(app, &ctx);
    }
}

/// The letter a menu's or a row's name begins with (past a tick or
/// spaces before it), lower case.
fn initial(name: &str) -> char {
    let first = name.chars().find(|c| c.is_alphanumeric());
    first.map_or(' ', |c| c.to_lowercase().next().unwrap_or(c))
}

/// A menu row's keys (beside its name), laid out by the row and painted
/// by [`add_entry`] in the color its state gives (`mg_uikit::MenuKeys`):
/// `on-fill` where the row is lit, under the pointer or the keys' cursor
/// (the weak text did not read on the menu's `accent-fill`). None where
/// it has no keys.
fn keys_of(ui: &Ui, key: String) -> Option<mg_uikit::MenuKeys> {
    (!key.is_empty()).then(|| mg_uikit::MenuKeys::new(ui, key))
}

/// A row of a menu, for the keys: counted, marked where the keys are at
/// it, and chosen by Enter. Returns the button to draw, and whether the
/// keys chose it.
fn row<'a>(
    app: &mut Moonglow,
    level: usize,
    name: &str,
    entry: egui::Button<'a>,
    enabled: bool,
) -> (egui::Button<'a>, bool) {
    if app.menu_row.len() <= level {
        app.menu_row.resize(level + 1, 0);
    }
    let index = app.menu_row[level];
    app.menu_row[level] += 1;
    if app.menu_keys.rows.len() <= level {
        app.menu_keys.rows.resize(level + 1, Vec::new());
    }
    app.menu_keys.rows[level].push(enabled);
    if app.menu_keys.initials.len() <= level {
        app.menu_keys.initials.resize(level + 1, Vec::new());
    }
    app.menu_keys.initials[level].push(initial(name));
    let at = app.menu_keys.at(level, index);
    let chosen = at && enabled && std::mem::take(&mut app.menu_keys.choose);
    (entry.selected(at), chosen)
}

/// A submenu's entry: a row for the keys, opened by Right or Enter (and
/// kept open while the keys are in it), its own rows a level deeper.
fn submenu(
    app: &mut Moonglow,
    ui: &mut Ui,
    level: usize,
    name: &str,
    enabled: bool,
    contents: impl FnOnce(&mut Moonglow, &mut Ui),
) -> egui::Response {
    use egui::containers::menu::{MenuState, SubMenu};
    let (button, chosen) = row(app, level, name, egui::Button::new(name), enabled);
    let index = app.menu_row[level] - 1;
    let at = app.menu_keys.at(level, index);
    let inside = app.menu_keys.path.len() == level + 2 && app.menu_keys.path[level] == index;
    let enter = at && enabled && (chosen || app.menu_keys.into);
    if enter {
        app.menu_keys.into = false;
        app.menu_keys.path.push(usize::MAX);
    }
    if !enabled {
        return ui.add_enabled(false, button);
    }
    if enter || inside {
        // (Held open for the keys: the pointer is elsewhere. Said before
        // the entry is drawn, and as shown, or the menu forgets it.)
        let id = SubMenu::id_from_widget_id(ui.next_auto_id());
        MenuState::mark_shown(ui.ctx(), id);
        MenuState::from_ui(ui, |state, _| state.open_item = Some(id));
    }
    if at && std::mem::take(&mut app.menu_keys.close_sub) {
        MenuState::from_ui(ui, |state, _| state.open_item = None);
        ui.ctx().request_repaint();
    }
    // (Its words and a chevron: `mg_uikit`'s submenu row.)
    let entry = mg_uikit::IconButton::new(Icon::ChevronRight, name)
        .label(mg_uikit::Label::WordsOnly)
        .selected(at);
    let response = entry
        .menu(ui, |ui| {
            if app.menu_row.len() <= level + 1 {
                app.menu_row.resize(level + 2, 0);
            }
            app.menu_row[level + 1] = 0;
            contents(app, ui);
        })
        .response;
    if inside && app.menu_keys.path.last() == Some(&usize::MAX) {
        let first = app.menu_keys.first(level + 1);
        *app.menu_keys.path.last_mut().expect("just seen") = first;
        ui.ctx().request_repaint();
    }
    if enter {
        ui.ctx().request_repaint();
    }
    response
}

fn menu(app: &mut Moonglow, ui: &mut Ui, items: &[Item], level: usize) {
    for item in items {
        match *item {
            Do(id) => button(app, ui, id, level),
            Separator => {
                mg_uikit::menu_separator(ui);
            }
            Sub(name, items) => {
                submenu(app, ui, level, name, true, |app, ui| menu(app, ui, items, level + 1));
            }
            Item::Wizards => {
                for kind in crate::blueprint_wizard::KINDS {
                    button(app, ui, Id::Wizard(kind), level);
                }
            }
            Item::WithPlugins(id, kind) => {
                let entries = app.plugin_entries(kind);
                if entries.is_empty() {
                    button(app, ui, id, level);
                } else {
                    let name = id.label(app).trim_end_matches('…').to_string();
                    submenu(app, ui, level, &name, true, |app, ui| {
                        button(app, ui, id, level + 1);
                        mg_uikit::menu_separator(ui);
                        plugin_entries(app, ui, &entries, level + 1);
                    });
                }
            }
            Item::PluginWizards => {
                let entries = app.plugin_entries(crate::plugin_kinds::Added::Wizard);
                if !entries.is_empty() {
                    mg_uikit::menu_separator(ui);
                    plugin_entries(app, ui, &entries, level);
                }
            }
            Item::PluginPanels => {
                let entries = app.plugin_entries(crate::plugin_kinds::Added::Panel);
                if !entries.is_empty() {
                    submenu(app, ui, level, "Panels", true, |app, ui| {
                        plugin_entries(app, ui, &entries, level + 1);
                    });
                    mg_uikit::menu_separator(ui);
                }
            }
            Item::PluginCommands => {
                let commands = app.plugin_commands();
                let several = commands.iter().any(|c| c.plugin != commands[0].plugin);
                let mut at = 0;
                while at < commands.len() {
                    // Each plugin's under its name, when there are several.
                    let plugin = &commands[at].plugin;
                    let n = commands[at..].iter().take_while(|c| &c.plugin == plugin).count();
                    if several {
                        mg_uikit::menu_heading(ui, &commands[at].plugin_name);
                    }
                    plugin_command_rows(app, ui, level, &commands[at..at + n], 0);
                    at += n;
                }
                if !commands.is_empty() {
                    mg_uikit::menu_separator(ui);
                }
            }
            Item::Project => {
                let commands =
                    crate::project_commands::commands(app).map(|(_, c)| c).unwrap_or_default();
                let enabled = !commands.is_empty();
                submenu(app, ui, level, "Project", enabled, |app, ui| {
                    for c in commands {
                        let name = format!("{}…", c.label);
                        let can = c.run.is_some();
                        let mut entry = egui::Button::new(&name);
                        let id = crate::project_commands::key_id(&c.name);
                        let keys = keys_of(ui, app.keymap.label_of(&id, ui.ctx()));
                        if let Some(keys) = &keys {
                            entry = entry.shortcut_text(keys.atom());
                        }
                        let (entry, chosen) = row(app, level + 1, &name, entry, can);
                        let tip = crate::project_commands::tip(&c);
                        let at = app.menu_keys.at(level + 1, app.menu_row[level + 1] - 1);
                        let (r, _) = add_entry(ui, can, entry, keys.as_ref(), at);
                        let r = r.on_hover_text(&tip).on_disabled_hover_text(&tip);
                        if r.clicked() || chosen {
                            app.actions.push(Action::ProjectCommand(c.name));
                            egui::Popup::close_all(ui.ctx());
                        }
                    }
                    mg_uikit::menu_separator(ui);
                    let name = "Command Output";
                    let can = crate::project_commands::has_output(app);
                    let (entry, chosen) = row(app, level + 1, name, egui::Button::new(name), can);
                    let r = ui
                        .add_enabled(can, entry)
                        .on_hover_text("The last command's output, all of it, and how it ended");
                    if r.clicked() || chosen {
                        app.project_commands.show = true;
                        egui::Popup::close_all(ui.ctx());
                    }
                })
                .on_disabled_hover_text(
                    "The update and submit commands a project's moonglow.toml names ([commands])",
                );
            }
            Item::Recent => {
                let recent = app.settings.recent.clone();
                let enabled = !recent.is_empty();
                submenu(app, ui, level, "Recent Modules", enabled, |app, ui| {
                    for p in recent {
                        let name = p.display().to_string();
                        let entry = egui::Button::new(&name);
                        let (entry, chosen) = row(app, level + 1, &name, entry, true);
                        if ui.add(entry).clicked() || chosen {
                            app.actions.push(Action::OpenModule(p));
                            egui::Popup::close_all(ui.ctx());
                        }
                    }
                });
            }
            Item::Prefabs => {
                let names = crate::prefabs::list(app.prefab_dir.as_deref());
                let enabled = app.ws.is_some() && !names.is_empty();
                submenu(app, ui, level, "Prefabs", enabled, |app, ui| {
                    for n in names {
                        let (entry, chosen) = row(app, level + 1, &n, egui::Button::new(&n), true);
                        let r = ui.add(entry).on_hover_text("Place it in the area shown");
                        if r.clicked() || chosen {
                            app.actions.push(Action::PlacePrefab(n));
                            egui::Popup::close_all(ui.ctx());
                        }
                    }
                })
                .on_disabled_hover_text("Save objects as a prefab from an area's menu first");
            }
        }
    }
}

/// A command's menu entry: its name as things are, its first key, its tip.
fn button(app: &mut Moonglow, ui: &mut Ui, id: Id, level: usize) {
    if !id.shown(app) {
        return;
    }
    let name = id.label(app);
    let ticked = id.ticked(app);
    let mut entry = match ticked {
        Some(_) => egui::Button::new(tick_atoms(&name)),
        None => egui::Button::new(&name),
    };
    let keys = keys_of(ui, app.keymap.label_of(&id.id(), ui.ctx()));
    if let Some(keys) = &keys {
        entry = entry.shortcut_text(keys.atom());
    }
    let enabled = id.enabled(app);
    let (entry, chosen) = row(app, level, &name, entry, enabled);
    let at = app.menu_keys.at(level, app.menu_row[level] - 1);
    let (mut r, tick) = add_entry(ui, enabled, entry, keys.as_ref(), at);
    if let Some(on) = ticked {
        paint_tick(ui, &r, tick, on, at, &name);
    }
    let hint = id.text().2;
    if !hint.is_empty() {
        r = r.on_hover_text(hint);
    }
    if r.clicked() || chosen {
        id.run(app, ui.ctx());
        if chosen {
            egui::Popup::close_all(ui.ctx());
        }
    }
}

/// A command's row in a menu outside the menu bar (Test Module's arrow on
/// the Top Bar), read from the window before the menu is drawn: named as
/// the menus name it, with its keys and its tip, enabled as it is.
pub(crate) struct CommandRow {
    pub(crate) id: Id,
    label: String,
    keys: String,
    enabled: bool,
}

impl CommandRow {
    pub(crate) fn of(app: &Moonglow, ctx: &egui::Context, id: Id) -> CommandRow {
        let keys = app.keymap.label_of(&id.id(), ctx);
        CommandRow { id, label: id.label(app), keys, enabled: id.enabled(app) }
    }

    /// Whether it can be chosen now.
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    /// Draws it; whether it was chosen (the menu is closed then: the
    /// caller runs it).
    pub(crate) fn show(&self, ui: &mut Ui) -> bool {
        let mut entry = egui::Button::new(&self.label);
        let keys = keys_of(ui, self.keys.clone());
        if let Some(keys) = &keys {
            entry = entry.shortcut_text(keys.atom());
        }
        let (mut r, _) = add_entry(ui, self.enabled, entry, keys.as_ref(), false);
        let hint = self.id.hint();
        if !hint.is_empty() {
            r = r.on_hover_text(hint);
        }
        let chosen = r.clicked();
        if chosen {
            ui.close();
        }
        chosen
    }
}

/// The id of a tick's room in a menu row ([`tick_atoms`]).
fn tick_id() -> egui::Id {
    egui::Id::new("menu-tick")
}

/// A menu row's words after a tick's room (an icon's, 16 px).
fn tick_atoms(name: &str) -> egui::Atoms<'static> {
    let mut atoms = egui::Atoms::default();
    atoms.push_right(egui::Atom::custom(tick_id(), egui::Vec2::splat(16.0)));
    atoms.push_right(name.to_string());
    atoms
}

/// A menu row `entry` added, enabled or not, its `keys` painted where it
/// has them (lit where the row is, or where the keys' cursor is `at` it),
/// and the room it laid out for its tick, where it has one
/// ([`tick_atoms`]).
fn add_entry(
    ui: &mut Ui,
    enabled: bool,
    entry: egui::Button<'_>,
    keys: Option<&mg_uikit::MenuKeys>,
    at: bool,
) -> (egui::Response, Option<egui::Rect>) {
    ui.add_enabled_ui(enabled, |ui| {
        let laid = entry.atom_ui(ui);
        if let Some(keys) = keys {
            keys.paint(ui, &laid, at || mg_uikit::menu_row_lit(&laid.response));
        }
        (laid.response.clone(), laid.rect(tick_id()))
    })
    .inner
}

/// A ticked menu row's tick (`check`, in its words' color) in `room`, the
/// room [`tick_atoms`] left at its start (`lit`: the keys' cursor is at
/// it), and its state for a screen reader: a check box named `name`,
/// ticked or not.
fn paint_tick(
    ui: &Ui,
    r: &egui::Response,
    room: Option<egui::Rect>,
    on: bool,
    lit: bool,
    name: &str,
) {
    let enabled = r.enabled();
    r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, on, name));
    if let Some(room) = room.filter(|_| on && ui.is_rect_visible(r.rect)) {
        let color = ui.style().interact_selectable(r, lit).text_color();
        Icon::Check.paint(ui.painter(), room, color);
    }
}

/// The Command Palette: the commands found by what is typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Finder {
    pub filter: String,
    /// Which of those found Enter runs.
    pub selected: usize,
    /// Its field has taken the keyboard (once, when it opens).
    focused: bool,
}

/// A plugin's commands in the Plugins menu at `depth` of their submenus
/// (API 0.7's `menu`): each without a submenu there a row, each submenu
/// once, where its first command is, holding those that name it.
fn plugin_command_rows(
    app: &mut Moonglow,
    ui: &mut Ui,
    level: usize,
    commands: &[crate::plugins::PluginCommand],
    depth: usize,
) {
    let mut made: Vec<&str> = Vec::new();
    for c in commands {
        let enabled = app.ws.is_some();
        let Some(name) = c.menu.get(depth) else {
            let mut entry = egui::Button::new(&c.title);
            let keys = keys_of(ui, app.keymap.label_of(&c.key_id, ui.ctx()));
            if let Some(keys) = &keys {
                entry = entry.shortcut_text(keys.atom());
            }
            let (entry, chosen) = row(app, level, &c.title, entry, enabled);
            let at = app.menu_keys.at(level, app.menu_row[level] - 1);
            let (mut r, _) = add_entry(ui, enabled, entry, keys.as_ref(), at);
            if !c.hint.is_empty() {
                r = r.on_hover_text(&c.hint);
            }
            if r.clicked() || chosen {
                app.run_plugin_command(&c.plugin, &c.command);
                if chosen {
                    egui::Popup::close_all(ui.ctx());
                }
            }
            continue;
        };
        if made.contains(&name.as_str()) {
            continue;
        }
        made.push(name);
        let inside: Vec<crate::plugins::PluginCommand> =
            commands.iter().filter(|d| d.menu.get(depth) == Some(name)).cloned().collect();
        submenu(app, ui, level, name, enabled, |app, ui| {
            plugin_command_rows(app, ui, level + 1, &inside, depth + 1);
        });
    }
}

/// The enabled plugins' exporters, importers, wizards or panels in a
/// menu: each plugin's under its name, where there are several; a panel's
/// ticked while it is shown.
fn plugin_entries(
    app: &mut Moonglow,
    ui: &mut Ui,
    entries: &[crate::plugin_kinds::PluginEntry],
    level: usize,
) {
    let several = entries.iter().any(|e| e.plugin != entries[0].plugin);
    let mut last = None;
    for e in entries {
        if several && last != Some(&e.plugin) {
            mg_uikit::menu_heading(ui, &e.plugin_name);
        }
        last = Some(&e.plugin);
        let enabled = app.plugin_entry_enabled(e);
        // (A panel ticked while shown, as View's panes are.)
        let ticked = match e.kind {
            crate::plugin_kinds::Added::Panel => Some(app.plugin_panel_open(&e.plugin, &e.id)),
            _ => None,
        };
        let entry = match ticked {
            Some(_) => egui::Button::new(tick_atoms(&e.title)),
            None => egui::Button::new(&e.title),
        };
        let (entry, chosen) = row(app, level, &e.title, entry, enabled);
        let (mut r, tick) = add_entry(ui, enabled, entry, None, false);
        if let Some(on) = ticked {
            let at = app.menu_keys.at(level, app.menu_row[level] - 1);
            paint_tick(ui, &r, tick, on, at, &e.title);
        }
        let tip = match e.hint.as_str() {
            "" => format!("Of the plugin {}", e.plugin_name),
            hint => format!("{hint}\nOf the plugin {}", e.plugin_name),
        };
        r = r.on_hover_text(tip);
        if r.clicked() || chosen {
            app.actions.push(Action::RunPluginEntry {
                kind: e.kind,
                plugin: e.plugin.clone(),
                id: e.id.clone(),
            });
            egui::Popup::close_all(ui.ctx());
        }
    }
}

/// The menu a command is in.
fn menu_of(id: Id) -> Option<&'static str> {
    fn has(items: &[Item], id: Id) -> bool {
        items.iter().any(|item| match item {
            Do(other) | Item::WithPlugins(other, _) => *other == id,
            Sub(_, items) => has(items, id),
            Item::Wizards => matches!(id, Id::Wizard(_)),
            Separator
            | Item::Recent
            | Item::Prefabs
            | Item::PluginCommands
            | Item::PluginWizards
            | Item::PluginPanels
            | Item::Project => false,
        })
    }
    MENUS.iter().find(|(_, items)| has(items, id)).map(|(name, _)| *name)
}

/// What a row of the palette runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Choice {
    Window(Id),
    /// A plugin's command: the plugin's id and the command's.
    Plugin(String, String),
    /// A plugin's exporter, importer, wizard or panel (API 0.6).
    Entry(crate::plugin_kinds::Added, String, String),
    /// A project's command (`[commands]`), by its name.
    Project(String),
}

/// A command as the palette lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    pub(crate) choice: Choice,
    pub(crate) label: String,
    /// Its menu (a plugin's command: the plugin's name).
    pub(crate) menu: String,
    /// The id its keys are kept under.
    pub(crate) key_id: String,
    pub(crate) enabled: bool,
}

/// The commands the palette offers for what is typed: those whose name (as
/// its menu shows it now) holds every word, in the menus' order, then
/// those that need their menu's name for it (`build` finds Build Module,
/// then the rest of the Build menu). The enabled plugins' commands are
/// among them.
pub(crate) fn found(app: &Moonglow, filter: &str) -> Vec<Row> {
    let needle = mg_core::text::Needle::new(filter);
    let has_all = |text: &str| needle.finds(text);
    let window =
        Id::all().into_iter().filter(|id| *id != Id::CommandPalette && id.shown(app)).map(|id| {
            Row {
                choice: Choice::Window(id),
                label: id.label(app),
                menu: menu_of(id).unwrap_or_default().to_string(),
                key_id: id.id(),
                enabled: id.enabled(app),
            }
        });
    let plugins = app.plugin_commands().into_iter().map(|c| Row {
        label: c.path_title(),
        choice: Choice::Plugin(c.plugin, c.command),
        menu: c.plugin_name,
        key_id: c.key_id,
        enabled: app.ws.is_some(),
    });
    // (Their exporters, importers, wizards and panels, by their menus.)
    use crate::plugin_kinds::Added;
    let entries = [
        (Added::Exporter, "Export"),
        (Added::Importer, "Import"),
        (Added::Wizard, "Wizards"),
        (Added::Panel, "Panels"),
    ]
    .into_iter()
    .flat_map(|(kind, menu)| {
        app.plugin_entries(kind).into_iter().map(move |e| (menu, e)).collect::<Vec<_>>()
    })
    .map(|(menu, e)| Row {
        enabled: app.plugin_entry_enabled(&e),
        key_id: format!("{}/{}:{}", e.plugin, menu.to_lowercase(), e.id),
        label: e.title.clone(),
        menu: format!("{menu} {}", e.plugin_name),
        choice: Choice::Entry(e.kind, e.plugin, e.id),
    })
    .collect::<Vec<_>>();
    // (The project's commands, as File › Project has them.)
    let project = crate::project_commands::commands(app).map(|(_, c)| c).unwrap_or_default();
    let project = project.into_iter().map(|c| Row {
        key_id: crate::project_commands::key_id(&c.name),
        label: format!("{}…", c.label),
        menu: "Project".to_string(),
        enabled: c.run.is_some(),
        choice: Choice::Project(c.name),
    });
    let mut found: Vec<(bool, Row)> = window
        .chain(plugins)
        .chain(entries)
        .chain(project)
        .filter_map(|row| {
            let name = row.label.to_lowercase();
            let with_menu = format!("{} {name}", row.menu.to_lowercase());
            let by_name = has_all(&name);
            (by_name || has_all(&with_menu)).then_some((!by_name, row))
        })
        .collect();
    found.sort_by_key(|(by_menu, _)| *by_menu);
    found.into_iter().map(|(_, row)| row).collect()
}

/// The Command Palette's window: type part of a command's name, choose
/// with the arrows, Enter runs it; Escape or a click outside closes it.
pub(crate) fn palette_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut finder) = app.command_palette.take() else { return };
    let matches = found(app, &finder.filter);
    finder.selected = finder.selected.min(matches.len().saturating_sub(1));
    let mut run = None;
    let modal = egui::Modal::new(egui::Id::new("command-palette")).show(ctx, |ui| {
        ui.set_width(440.0);
        // The arrows and Enter choose; the rest is the field's.
        let none = egui::Modifiers::NONE;
        let (up, down, enter) = ui.input_mut(|i| {
            (
                i.consume_key(none, egui::Key::ArrowUp),
                i.consume_key(none, egui::Key::ArrowDown),
                i.consume_key(none, egui::Key::Enter),
            )
        });
        if !matches.is_empty() {
            let n = matches.len();
            finder.selected = (finder.selected + usize::from(down) + (n - usize::from(up))) % n;
        }
        // (Escape clears the name typed, then closes the palette.)
        let before = finder.filter.len();
        let searched = crate::widgets::SearchField::new(
            egui::Id::new("command-palette-find"),
            &mut finder.filter,
        )
        .hint("Type a command's name")
        .show(ui);
        let field = searched.response;
        if !std::mem::replace(&mut finder.focused, true) {
            field.request_focus();
        }
        if field.changed() || finder.filter.len() != before {
            finder.selected = 0;
        }
        if searched.left == Some(crate::widgets::FilterLeft::Escape) {
            ui.close();
        }
        ui.separator();
        if matches.is_empty() {
            ui.weak("No command has that name.");
        }
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                mg_uikit::list_rows(ui);
                for (i, row) in matches.iter().enumerate() {
                    // Its menu and its key, at the right.
                    let key = app.keymap.label_of(&row.key_id, ui.ctx());
                    let beside = match (row.menu.is_empty(), key.is_empty()) {
                        (false, true) => row.menu.clone(),
                        (false, false) => format!("{}   {key}", row.menu),
                        (true, _) => key,
                    };
                    // (Beside the row chosen, on the selection's fill: in the
                    // fill's color, `on-fill`, not the weak text's.)
                    let mut beside = egui::RichText::new(beside);
                    if i == finder.selected {
                        beside = beside.color(mg_uikit::tokens(ui.ctx()).on_fill);
                    }
                    let entry = egui::Button::selectable(i == finder.selected, &row.label)
                        .shortcut_text(beside);
                    let entry = ui.add_enabled(row.enabled, entry);
                    // (Named by the command alone, not with what is beside it.)
                    entry.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Button,
                            row.enabled,
                            i == finder.selected,
                            &row.label,
                        )
                    });
                    if (up || down) && i == finder.selected {
                        entry.scroll_to_me(None);
                    }
                    if entry.clicked() {
                        run = Some(row.choice.clone());
                    }
                }
            });
        });
        if enter && let Some(row) = matches.get(finder.selected).filter(|row| row.enabled) {
            run = Some(row.choice.clone());
        }
    });
    match run {
        Some(Choice::Window(id)) => id.run(app, ctx),
        Some(Choice::Plugin(plugin, command)) => app.run_plugin_command(&plugin, &command),
        Some(Choice::Entry(kind, plugin, id)) => {
            app.actions.push(Action::RunPluginEntry { kind, plugin, id })
        }
        // (Saved first, asked as from File › Project.)
        Some(Choice::Project(name)) => app.actions.push(Action::ProjectCommand(name)),
        None if modal.should_close() => {}
        None => app.command_palette = Some(finder),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_command_has_an_id_of_its_own() {
        let all = Id::all();
        let ids: HashSet<String> = all.iter().map(|c| c.id()).collect();
        assert_eq!(ids.len(), all.len(), "{all:?}");
        assert!(all.iter().all(|c| !c.name().is_empty() && !c.name().contains('…')));
        // Ids are what the settings keep keys under: these must not move.
        assert_eq!(Id::BuildModule.id(), "build-module");
        assert_eq!(Id::Wizard(BlueprintKind::Door).id(), "door-wizard");
        assert_eq!(Id::Wizard(BlueprintKind::Item).id(), "item-wizard");
        assert_eq!(Id::Save.id(), "save");
        assert_eq!(Id::SaveAsProject.id(), "save-as-project");
        assert_eq!(Id::TestChoose.id(), "test-choose");
    }

    #[test]
    fn the_keys_that_work_anywhere_are_all_commands() {
        // Each general key command is a command here (so its key runs it),
        // and no command's id is another group's key command's.
        let ids: HashSet<String> = Id::all().iter().map(|c| c.id()).collect();
        for cmd in keys::Cmd::ALL {
            let general = cmd.group() == keys::Group::General;
            assert_eq!(ids.contains(cmd.id()), general, "{}", cmd.id());
        }
        // (And named as that command is: one name, the menus'.)
        for cmd in keys::Cmd::ALL {
            assert!(!cmd.name().is_empty() && cmd.name() != cmd.id(), "{}", cmd.id());
            if cmd.group() == keys::Group::General {
                let id = Id::all().into_iter().find(|c| c.id() == cmd.id()).unwrap();
                assert_eq!(cmd.name(), id.name());
            }
        }
        assert_eq!(keys::Cmd::Save.name(), "Save");
        assert_eq!(keys::Cmd::TestChoose.name(), "Test Module, Choose Character");
        assert_eq!(name_of("build-module"), "Build Module");
        assert_eq!(name_of("camera-forward"), "Move Camera Forward");
        assert_eq!(name_of("some-plugin.command"), "some-plugin.command");
    }

    #[test]
    fn the_top_bar_and_menus_name_commands_once() {
        let all = Id::all();
        let unique: HashSet<Id> = all.iter().copied().collect();
        assert_eq!(unique.len(), all.len(), "a command is in the menus twice");
        let mut on_bar = HashSet::new();
        for id in TOP_BAR.commands() {
            assert!(all.contains(&id), "{id:?} is a command of the menus");
            assert!(on_bar.insert(id), "{id:?} is on the Top Bar twice");
        }
        assert!(on_bar.contains(&Id::TestModule) && on_bar.contains(&Id::TestChoose));
    }
}
