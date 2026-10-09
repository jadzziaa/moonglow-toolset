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
pub const MENUS: [(&str, &[Item]); 7] = [
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
    ("Wizards", &[Do(Id::AreaWizard), Separator, Do(Id::CreatureWizard), Item::Wizards]),
    (
        "Tools",
        &[
            Do(Id::NewConversation),
            Do(Id::Factions),
            Do(Id::Journal),
            Do(Id::TalkTable),
            Do(Id::NewScript),
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
        };
        (id.into(), label.into(), hint)
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
            | Id::FullScreen => true,
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
            | Id::CompileAll
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
    let keymap = app.keymap.clone();
    // (A letter given to a command stays a letter in a text field.)
    let typing = ui.memory(|m| m.focused().is_some());
    for id in Id::all() {
        let pressed = ui.input_mut(|i| keymap.consume_outside_text(i, &id.id(), typing));
        if pressed && id.enabled(app) {
            id.run(app, ui.ctx());
        }
    }
    // The plugins' commands, after the window's own: a key both have is
    // the window's.
    for c in app.plugin_commands() {
        let pressed = ui.input_mut(|i| keymap.consume_outside_text(i, &c.key_id, typing));
        if pressed && app.ws.is_some() {
            app.run_plugin_command(&c.plugin, &c.command);
        }
    }
}

/// The menu bar, from [`MENUS`].
pub(crate) fn menu_bar(app: &mut Moonglow, ui: &mut Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        for (name, items) in MENUS {
            ui.menu_button(name, |ui| menu(app, ui, items));
        }
    });
}

fn menu(app: &mut Moonglow, ui: &mut Ui, items: &[Item]) {
    for item in items {
        match *item {
            Do(id) => button(app, ui, id),
            Separator => {
                ui.separator();
            }
            Sub(name, items) => {
                ui.menu_button(name, |ui| menu(app, ui, items));
            }
            Item::Wizards => {
                for kind in crate::blueprint_wizard::KINDS {
                    button(app, ui, Id::Wizard(kind));
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
                    let mut r = ui.add_enabled(app.ws.is_some(), entry);
                    if !c.hint.is_empty() {
                        r = r.on_hover_text(&c.hint);
                    }
                    if r.clicked() {
                        app.run_plugin_command(&c.plugin, &c.command);
                    }
                }
                if !commands.is_empty() {
                    ui.separator();
                }
            }
            Item::Recent => {
                let recent = app.settings.recent.clone();
                ui.add_enabled_ui(!recent.is_empty(), |ui| {
                    ui.menu_button("Recent Modules", |ui| {
                        for p in recent {
                            if ui.button(p.display().to_string()).clicked() {
                                app.actions.push(Action::OpenModule(p));
                            }
                        }
                    });
                });
            }
            Item::Prefabs => {
                let names = crate::prefabs::list(app.prefab_dir.as_deref());
                ui.add_enabled_ui(app.ws.is_some() && !names.is_empty(), |ui| {
                    ui.menu_button("Prefabs", |ui| {
                        for n in names {
                            if ui.button(&n).on_hover_text("Place it in the area shown").clicked() {
                                app.actions.push(Action::PlacePrefab(n));
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_disabled_hover_text("Save objects as a prefab from an area's menu first");
                });
            }
        }
    }
}

/// A command's menu entry: its name as things are, its first key, its tip.
fn button(app: &mut Moonglow, ui: &mut Ui, id: Id) {
    if !id.shown(app) {
        return;
    }
    let mut entry = egui::Button::new(id.label(app));
    let key = app.keymap.label_of(&id.id(), ui.ctx());
    if !key.is_empty() {
        entry = entry.shortcut_text(key);
    }
    let mut r = ui.add_enabled(id.enabled(app), entry);
    let hint = id.text().2;
    if !hint.is_empty() {
        r = r.on_hover_text(hint);
    }
    if r.clicked() {
        id.run(app, ui.ctx());
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
