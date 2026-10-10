//! The window's commands, in one table: what each is called, when it can
//! be chosen and what it does. The menus, the toolbar, the keys (Options ›
//! Keyboard) and whatever else offers commands read it, so a command is
//! named, enabled and run the same wherever it is chosen, and every one of
//! them can be given a key.
//!
//! [`MENUS`] and [`TOOLBAR`] say where the commands show. Commands that act
//! on one thing (an object's or a resource's context menu) are not here:
//! they belong to what they act on.

use egui::Ui;
use mg_module::palette::BlueprintKind;

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
    Export,
    ImportConversation,
    Close,
    Exit,
    Undo,
    Redo,
    ModuleProperties,
    ResizeArea,
    RotateArea,
    FindInstance,
    /// Edit › Edit Areas Together…: the areas to edit as one.
    EditAreasTogether,
    ReplaceText,
    FindReferences,
    AreaWizard,
    CreatureWizard,
    /// A blueprint wizard (the kinds of `blueprint_wizard::KINDS`).
    Wizard(BlueprintKind),
    NewConversation,
    Factions,
    Journal,
    TalkTable,
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
    TestModule,
    TestChoose,
    PackTarget,
    AreaStatistics,
    /// The Plugins window.
    Plugins,
    /// Install a plugin from its archive.
    InstallPlugin,
    Manual,
    CommandPalette,
    About,
    /// The toolbar's Preview: the window showing the palette's blueprint.
    PreviewWindow,
    FullScreen,
    /// The tab in front: closed, the next or the one before brought to the
    /// front, the one closed last opened again.
    CloseTab,
    NextTab,
    PreviousTab,
    ReopenTab,
    /// View: the panes beside the middle shown or folded away.
    ViewTree,
    ViewPalettes,
    ViewLog,
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
            Separator,
            Do(Id::Import),
            Do(Id::Export),
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
            Separator,
            Do(Id::ResizeArea),
            Do(Id::RotateArea),
            Separator,
            Do(Id::FindInstance),
            Item::Prefabs,
            Do(Id::EditAreasTogether),
            Do(Id::ReplaceText),
            Do(Id::FindReferences),
        ],
    ),
    (
        "View",
        &[
            Do(Id::ViewTree),
            Do(Id::ViewPalettes),
            Do(Id::ViewLog),
            Separator,
            Do(Id::HidePanels),
            Do(Id::ResetLayout),
        ],
    ),
    ("Wizards", &[Do(Id::AreaWizard), Separator, Do(Id::CreatureWizard), Item::Wizards]),
    (
        "Tools",
        &[
            Do(Id::NewConversation),
            Do(Id::Factions),
            Do(Id::Journal),
            Do(Id::TalkTable),
            Do(Id::NewScript),
            Do(Id::NewNui),
            Separator,
            Do(Id::Palettes),
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
            Do(Id::TestModule),
            Do(Id::TestChoose),
            Do(Id::PackTarget),
            Separator,
            Do(Id::AreaStatistics),
        ],
    ),
    ("Plugins", &[Item::PluginCommands, Do(Id::Plugins), Do(Id::InstallPlugin)]),
    ("Help", &[Do(Id::Manual), Do(Id::CommandPalette), Do(Id::About)]),
];

/// The toolbar: each command with its button's text and its tip (`None`:
/// a gap).
pub const TOOLBAR: [Option<(Id, &str, &str)>; 15] = [
    Some((Id::NewModule, "🗋 New", "New module")),
    Some((Id::OpenModule, "🗁 Open", "Open module")),
    Some((Id::Save, "💾 Save", "Save module")),
    None,
    Some((Id::Undo, "⟲ Undo", "Undo")),
    Some((Id::Redo, "⟳ Redo", "Redo")),
    None,
    Some((Id::ModuleProperties, "ℹ Properties", "Module properties")),
    Some((Id::AreaWizard, "🗺 New Area", "Area Wizard")),
    Some((Id::Resources, "🔍 Resources", "Resource browser")),
    Some((Id::Palettes, "📦 Palettes", "Blueprint palettes")),
    Some((
        Id::PreviewWindow,
        "👁 Preview",
        "Show Preview Window: the blueprint chosen in the palette",
    )),
    None,
    Some((Id::CompileAll, "⚙ Compile", "Compile all scripts")),
    Some((Id::Verify, "✔ Verify", "Verify the module")),
];

impl Id {
    /// Every command, in the menus' order, then those no menu has.
    pub fn all() -> Vec<Id> {
        fn walk(items: &[Item], out: &mut Vec<Id>) {
            for item in items {
                match item {
                    Do(id) => out.push(*id),
                    Sub(_, items) => walk(items, out),
                    Item::Wizards => {
                        out.extend(crate::blueprint_wizard::KINDS.into_iter().map(Id::Wizard));
                    }
                    Separator | Item::Recent | Item::Prefabs | Item::PluginCommands => {}
                }
            }
        }
        let mut out = Vec::new();
        for (_, items) in MENUS {
            walk(items, &mut out);
        }
        out.extend([Id::PreviewWindow, Id::FullScreen]);
        out.extend([Id::CloseTab, Id::NextTab, Id::PreviousTab, Id::ReopenTab]);
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
            Id::OpenModule => ("open-module", "Open Module…", ""),
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
            Id::ResizeArea => ("resize-area", "Resize Area…", ""),
            Id::RotateArea => ("rotate-area", "Rotate Area…", ""),
            Id::FindInstance => ("find-instance", "Find Instance…", ""),
            Id::EditAreasTogether => (
                "edit-areas-together",
                "Edit Areas Together…",
                "Choose several areas (by name, tileset, interior, underground…) and set their \
                 lighting, fog, weather, music, scripts and variables at once",
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
            Id::NewScript => ("new-script", "New Script…", ""),
            Id::NewNui => ("new-nui", "NUI Creator…", "Create a game UI window and its scripts"),
            Id::Palettes => ("palettes", "Palettes", ""),
            Id::PlaceableGallery => (
                "placeable-gallery",
                "Appearance Gallery…",
                "Every placeable's, creature's and door's appearance as a picture; a click gives it to what is selected",
            ),
            Id::Resources => ("resources", "Resource Browser", ""),
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
                "Compile the module's own models kept as text (ASCII), each against its \
                 supermodel from the module, its haks or the game",
            ),
            Id::BuildModule => ("build-module", "Build Module…", ""),
            Id::PublishNwsync => (
                "publish-nwsync",
                "Publish to NWSync…",
                "The module's haks and talk table, for players' games to download",
            ),
            Id::Verify => ("verify", "Verify Module", ""),
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
            Id::AreaStatistics => ("area-statistics", "Area Statistics", ""),
            Id::Plugins => (
                "plugins",
                "Manage Plugins…",
                "The plugins installed: enable them, and try the plugin console",
            ),
            Id::InstallPlugin => (
                "install-plugin",
                "Install Plugin from File…",
                "Install a plugin from its archive (a zip); it is off until you enable it",
            ),
            Id::Manual => ("manual", "User Manual", ""),
            Id::CommandPalette => {
                ("command-palette", "Command Palette…", "Find a command by its name and run it")
            }
            Id::About => ("about", "About Moonglow Toolset", ""),
            Id::PreviewWindow => ("preview-window", "Preview Window", ""),
            Id::FullScreen => ("full-screen", "Full Screen", ""),
            Id::CloseTab => ("close-tab", "Close Tab", "The tab in front"),
            Id::NextTab => ("next-tab", "Next Tab", ""),
            Id::PreviousTab => ("previous-tab", "Previous Tab", ""),
            Id::ReopenTab => ("reopen-tab", "Reopen Closed Tab", "The tab closed last"),
            Id::ViewTree => ("view-tree", "Module Tree", "Show the module tree, or fold it away"),
            Id::ViewPalettes => {
                ("view-palettes", "Palettes Panel", "Show the palettes, or fold them away")
            }
            Id::ViewLog => ("view-log", "Log", "Show the log, or fold it away"),
            Id::HidePanels => (
                "hide-panels",
                "Hide All Panels",
                "Fold the tree, the palettes and the log away, or bring them back",
            ),
            Id::ResetLayout => (
                "reset-layout",
                "Reset Layout",
                "The tree, the palettes and the log shown where they are at first",
            ),
        };
        (id.into(), label.into(), hint)
    }

    /// Its name, as Options › Keyboard lists it.
    pub fn name(self) -> String {
        self.text().1.trim_end_matches('…').to_string()
    }

    /// The key command it shares its keys with, where it had keys before
    /// every command could (its id and its defaults are that one's).
    fn key(self) -> Option<keys::Cmd> {
        use keys::Cmd;
        Some(match self {
            Id::NewModule => Cmd::NewModule,
            Id::OpenModule => Cmd::OpenModule,
            Id::Save => Cmd::Save,
            Id::Undo => Cmd::Undo,
            Id::Redo => Cmd::Redo,
            Id::ReplaceText => Cmd::ReplaceText,
            Id::AreaWizard => Cmd::AreaWizard,
            Id::CreatureWizard => Cmd::CreatureWizard,
            Id::Wizard(BlueprintKind::Item) => Cmd::ItemWizard,
            Id::NewConversation => Cmd::NewConversation,
            Id::Factions => Cmd::Factions,
            Id::Journal => Cmd::Journal,
            Id::NewScript => Cmd::NewScript,
            Id::CompileAll => Cmd::CompileAll,
            Id::TestModule => Cmd::TestModule,
            Id::TestChoose => Cmd::TestChoose,
            Id::Manual => Cmd::Manual,
            Id::CommandPalette => Cmd::CommandPalette,
            Id::FullScreen => Cmd::FullScreen,
            Id::Close => Cmd::Close,
            Id::CloseTab => Cmd::CloseTab,
            Id::NextTab => Cmd::NextTab,
            Id::PreviousTab => Cmd::PreviousTab,
            Id::ReopenTab => Cmd::ReopenTab,
            Id::ViewTree => Cmd::ToggleTree,
            Id::ViewPalettes => Cmd::TogglePalettes,
            Id::ViewLog => Cmd::ToggleLog,
            Id::HidePanels => Cmd::HidePanels,
            _ => return None,
        })
    }

    /// The id its keys are kept under in the settings.
    pub fn id(self) -> String {
        self.text().0.into_owned()
    }

    /// Moonglow's keys for it.
    pub fn defaults(self) -> Vec<egui::KeyboardShortcut> {
        self.key().map(keys::Cmd::defaults).unwrap_or_default()
    }

    /// Its name in a menu as things are: Undo names what it would undo, a
    /// project's Pack its file.
    fn label(self, app: &Moonglow) -> String {
        let ws = app.ws.as_ref();
        match self {
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
            // (Shown: ticked.)
            Id::ViewTree | Id::ViewPalettes | Id::ViewLog => {
                let tick = if app.panel_shown(self) { "✔ " } else { "    " };
                format!("{tick}{}", self.text().1)
            }
            Id::HidePanels if app.panels_hidden() => "Show All Panels".into(),
            _ => self.text().1.into_owned(),
        }
    }

    /// Whether it is offered at all (a project's command without a
    /// project is not).
    fn shown(self, app: &Moonglow) -> bool {
        match self {
            Id::PackTarget => app.ws.as_ref().is_some_and(|ws| ws.module.project.is_some()),
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
            | Id::BuildHak
            | Id::TalkTable
            | Id::ReloadResources
            | Id::Options
            | Id::Plugins
            | Id::Manual
            | Id::CommandPalette
            | Id::About
            | Id::PreviewWindow
            | Id::FullScreen
            | Id::NextTab
            | Id::PreviousTab => true,
            Id::ViewLog | Id::ResetLayout => true,
            Id::ViewTree | Id::HidePanels => open,
            Id::ViewPalettes => open && app.game.is_some(),
            Id::CloseTab => app.front_tab().is_some(),
            Id::ReopenTab => !app.closed_tabs.is_empty(),
            Id::InstallPlugin => app.plugin_dir.is_some(),
            // (The talk table's own, with the pointer over its editor.)
            Id::Undo => {
                app.ws.as_ref().is_some_and(|ws| ws.can_undo().is_some())
                    || (app.talk_view.hovered && app.talk.as_ref().is_some_and(|t| t.can_undo()))
            }
            Id::Redo => {
                app.ws.as_ref().is_some_and(|ws| ws.can_redo().is_some())
                    || (app.talk_view.hovered && app.talk.as_ref().is_some_and(|t| t.can_redo()))
            }
            Id::ResizeArea | Id::RotateArea | Id::AreaStatistics => shown_area(app).is_some(),
            Id::PackTarget => self.shown(app),
            Id::Save
            | Id::SaveAs
            | Id::SaveAsProject
            | Id::Import
            | Id::Export
            | Id::ImportConversation
            | Id::Close
            | Id::ModuleProperties
            | Id::FindInstance
            | Id::EditAreasTogether
            | Id::ReplaceText
            | Id::FindReferences
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
            Id::Export => Action::ExportDialog(Vec::new()),
            Id::Close => Action::Close,
            Id::Exit => Action::Quit,
            Id::Undo => Action::Undo,
            Id::Redo => Action::Redo,
            Id::ModuleProperties => Action::OpenTab(Tab::ModuleProperties),
            Id::FindReferences => Action::OpenTab(Tab::References),
            Id::AreaWizard => Action::AreaWizard,
            Id::Factions => Action::OpenTab(Tab::Factions),
            Id::Journal => Action::OpenTab(Tab::Journal),
            Id::TalkTable => Action::OpenTab(Tab::TalkTable),
            Id::Palettes => Action::OpenTab(Tab::Palette),
            Id::PlaceableGallery => Action::PlaceableGallery,
            Id::Resources => Action::OpenTab(Tab::Resources),
            Id::ReloadResources => Action::ReloadResources,
            Id::Options => Action::OptionsDialog,
            Id::CompileAll => Action::CompileScripts,
            Id::CompileModels => Action::CompileModels,
            Id::Verify => Action::Verify,
            Id::TestModule => Action::SaveThen(Box::new(Action::TestModule)),
            Id::TestChoose => Action::SaveThen(Box::new(Action::TestModuleChoose)),
            Id::PackTarget => Action::PackTarget,
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
            Id::FindInstance => {
                app.find_instance.get_or_insert_with(Default::default);
                return;
            }
            Id::EditAreasTogether => {
                app.area_chooser.get_or_insert_with(Default::default);
                return;
            }
            Id::ReplaceText => {
                app.text_replace.get_or_insert_with(Default::default);
                return;
            }
            Id::CreatureWizard => {
                app.creature_wizard = Some(Default::default());
                return;
            }
            Id::Wizard(kind) => {
                app.blueprint_wizard = Some(crate::blueprint_wizard::BlueprintWizard::new(kind));
                return;
            }
            Id::NewConversation => {
                app.new_dialog = Some(String::new());
                return;
            }
            Id::NewScript => {
                app.new_script = Some(String::new());
                return;
            }
            Id::NewNui => Action::OpenTab(Tab::Nui(None)),
            Id::NewTileset => return crate::tileset_view::new_tileset(app),
            Id::OpenTileset => return crate::tileset_view::open(app),
            Id::NewHak => return crate::hak_view::new_hak(app),
            Id::OpenHak => return crate::hak_view::open_hak(app),
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
            Id::PreviewWindow => {
                app.preview_window = !app.preview_window;
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
            Id::NextTab => return app.step_tab(1),
            Id::PreviousTab => return app.step_tab(-1),
            Id::ReopenTab => return app.reopen_tab(),
            Id::ViewTree | Id::ViewPalettes | Id::ViewLog => {
                let shown = app.panel_shown(self);
                return app.show_panel(self, !shown);
            }
            Id::HidePanels => {
                let show = app.panels_hidden();
                for panel in [Id::ViewTree, Id::ViewPalettes, Id::ViewLog] {
                    app.show_panel(panel, show);
                }
                return;
            }
            Id::ResetLayout => return app.reset_layout(ctx),
        };
        app.actions.push(action);
    }
}

/// The area the commands about "the area" mean: the one shown last, while
/// its view is open.
fn shown_area(app: &Moonglow) -> Option<mg_core::ResRef> {
    app.palette.area.filter(|a| app.area_views.contains_key(a))
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
    let in_text = app.text_typed.is_some() && ui.ctx().egui_wants_keyboard_input();
    for id in Id::all() {
        if in_text && matches!(id, Id::Undo | Id::Redo) {
            continue;
        }
        let pressed = ui.input_mut(|i| app.keymap.consume_outside_text(i, &id.id(), typing));
        if pressed && id.enabled(app) {
            id.run(app, ui.ctx());
        }
    }
    // The plugins' commands, after the window's own: a key both have is
    // the window's.
    for c in app.plugin_commands() {
        let pressed = ui.input_mut(|i| app.keymap.consume_outside_text(i, &c.key_id, typing));
        if pressed && app.ws.is_some() {
            app.run_plugin_command(&c.plugin, &c.command);
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
/// and closes the menus; Left and Right go from menu to menu, round the
/// ends; Up and Down from row to row, past separators and rows that can't
/// be chosen; Enter or Space chooses; Right opens a submenu, Left closes
/// it; Escape closes the submenu the keys are in, else the menus. Alt and
/// a menu's first letter (underlined) opens that menu; with a menu open, a
/// letter goes to the next row that begins with it.
pub(crate) fn menu_bar(app: &mut Moonglow, ui: &mut Ui) {
    use egui::{Key, Modifiers, Popup};
    let ctx = ui.ctx().clone();
    // A letter pressed: with Alt alone held, a menu's; with a menu open
    // and nothing held, a row's. Taken from the frame's keys, with the
    // text it typed.
    let letter = |alt: bool| {
        ctx.input_mut(|i| {
            let m = i.modifiers;
            if m.alt != alt || m.ctrl || m.shift || m.command {
                return None;
            }
            let found = i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, repeat: false, .. } => {
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
    let toggle = alt_released || ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F10));

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

    egui::MenuBar::new().ui(ui, |ui| {
        for (name, items) in MENUS {
            // (Its first letter underlined: Alt and the letter opens it.)
            let mut job = egui::text::LayoutJob::default();
            let font = egui::TextStyle::Button.resolve(ui.style());
            let color = ui.visuals().widgets.inactive.text_color();
            let plain = egui::TextFormat { font_id: font, color, ..Default::default() };
            let under =
                egui::TextFormat { underline: egui::Stroke::new(1.0, color), ..plain.clone() };
            let split = name.chars().next().map_or(0, char::len_utf8);
            job.append(&name[..split], 0.0, under);
            job.append(&name[split..], 0.0, plain);
            let (response, _) = egui::containers::menu::MenuButton::new(job).ui(ui, |ui| {
                app.menu_row = [0, 0];
                menu(app, ui, items, 0);
            });
            tops.push((Popup::default_response_id(&response), response));
        }
    });

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
}

/// The letter a menu's or a row's name begins with (past a tick or
/// spaces before it), lower case.
fn initial(name: &str) -> char {
    let first = name.chars().find(|c| c.is_alphanumeric());
    first.map_or(' ', |c| c.to_lowercase().next().unwrap_or(c))
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
    use egui::containers::menu::{MenuState, SubMenu, SubMenuButton};
    let index = app.menu_row[level];
    let (button, chosen) = row(app, level, name, egui::Button::new(name), enabled);
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
    let (response, _) = SubMenuButton::from_button(button).ui(ui, |ui| {
        if level + 1 < app.menu_row.len() {
            app.menu_row[level + 1] = 0;
        }
        contents(app, ui);
    });
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
                ui.separator();
            }
            Sub(name, items) => {
                submenu(app, ui, level, name, true, |app, ui| menu(app, ui, items, level + 1));
            }
            Item::Wizards => {
                for kind in crate::blueprint_wizard::KINDS {
                    button(app, ui, Id::Wizard(kind), level);
                }
            }
            Item::PluginCommands => {
                let commands = app.plugin_commands();
                let several = commands.iter().any(|c| c.plugin != commands[0].plugin);
                let mut last = None;
                for c in &commands {
                    // Each plugin's under its name, when there are several.
                    if several && last != Some(&c.plugin) {
                        ui.weak(&c.plugin_name);
                    }
                    last = Some(&c.plugin);
                    let mut entry = egui::Button::new(&c.title);
                    let key = app.keymap.label_of(&c.key_id, ui.ctx());
                    if !key.is_empty() {
                        entry = entry.shortcut_text(key);
                    }
                    let enabled = app.ws.is_some();
                    let (entry, chosen) = row(app, level, &c.title, entry, enabled);
                    let mut r = ui.add_enabled(enabled, entry);
                    if !c.hint.is_empty() {
                        r = r.on_hover_text(&c.hint);
                    }
                    if r.clicked() || chosen {
                        app.run_plugin_command(&c.plugin, &c.command);
                        if chosen {
                            egui::Popup::close_all(ui.ctx());
                        }
                    }
                }
                if !commands.is_empty() {
                    ui.separator();
                }
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
    let name = id.label(app).to_string();
    let mut entry = egui::Button::new(&name);
    let key = app.keymap.label_of(&id.id(), ui.ctx());
    if !key.is_empty() {
        entry = entry.shortcut_text(key);
    }
    let enabled = id.enabled(app);
    let (entry, chosen) = row(app, level, &name, entry, enabled);
    let mut r = ui.add_enabled(enabled, entry);
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

/// The Command Palette: the commands found by what is typed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Finder {
    pub filter: String,
    /// Which of those found Enter runs.
    pub selected: usize,
    /// Its field has taken the keyboard (once, when it opens).
    focused: bool,
}

/// The menu a command is in.
fn menu_of(id: Id) -> Option<&'static str> {
    fn has(items: &[Item], id: Id) -> bool {
        items.iter().any(|item| match item {
            Do(other) => *other == id,
            Sub(_, items) => has(items, id),
            Item::Wizards => matches!(id, Id::Wizard(_)),
            Separator | Item::Recent | Item::Prefabs | Item::PluginCommands => false,
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
    let words: Vec<String> = filter.split_whitespace().map(str::to_lowercase).collect();
    let has_all = |text: &str| words.iter().all(|w| text.contains(w));
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
        choice: Choice::Plugin(c.plugin, c.command),
        label: c.title,
        menu: c.plugin_name,
        key_id: c.key_id,
        enabled: app.ws.is_some(),
    });
    let mut found: Vec<(bool, Row)> = window
        .chain(plugins)
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
        let field = ui.add(
            egui::TextEdit::singleline(&mut finder.filter)
                .hint_text("Type a command's name")
                .desired_width(f32::INFINITY),
        );
        if !std::mem::replace(&mut finder.focused, true) {
            field.request_focus();
        }
        if field.changed() {
            finder.selected = 0;
        }
        ui.separator();
        if matches.is_empty() {
            ui.weak("No command has that name.");
        }
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                for (i, row) in matches.iter().enumerate() {
                    // Its menu and its key, at the right.
                    let key = app.keymap.label_of(&row.key_id, ui.ctx());
                    let beside = match (row.menu.is_empty(), key.is_empty()) {
                        (false, true) => row.menu.clone(),
                        (false, false) => format!("{}   {key}", row.menu),
                        (true, _) => key,
                    };
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
        None if modal.should_close() => {}
        None => app.command_palette = Some(finder),
    }
}

/// The toolbar, from [`TOOLBAR`]: each command's button, its key in its
/// tip.
pub(crate) fn toolbar(app: &mut Moonglow, ui: &mut Ui) {
    ui.horizontal(|ui| {
        for tool in TOOLBAR {
            let Some((id, text, tip)) = tool else {
                ui.separator();
                continue;
            };
            let tip = match app.keymap.label_of(&id.id(), ui.ctx()) {
                k if k.is_empty() => tip.to_string(),
                k => format!("{tip} ({k})"),
            };
            let clicked = if id == Id::PreviewWindow {
                // A window that is shown or not: its button stays down.
                let mut on = app.preview_window;
                ui.toggle_value(&mut on, text).on_hover_text(tip);
                on != app.preview_window
            } else {
                ui.add_enabled(id.enabled(app), egui::Button::new(text).small())
                    .on_hover_text(tip)
                    .clicked()
            };
            if clicked {
                id.run(app, ui.ctx());
            }
        }
    });
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
        // A command that had keys before keeps the id they are kept under.
        for c in &all {
            if let Some(cmd) = c.key() {
                assert_eq!(c.id(), cmd.id());
            }
        }
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
        assert_eq!(name_of("build-module"), "Build Module");
        assert_eq!(name_of("camera-forward"), "Move Camera Forward");
        assert_eq!(name_of("some-plugin.command"), "some-plugin.command");
    }

    #[test]
    fn the_toolbar_and_menus_name_commands_once() {
        let all = Id::all();
        let unique: HashSet<Id> = all.iter().copied().collect();
        assert_eq!(unique.len(), all.len(), "a command is in the menus twice");
        for (id, ..) in TOOLBAR.into_iter().flatten() {
            assert!(all.contains(&id));
        }
    }
}
