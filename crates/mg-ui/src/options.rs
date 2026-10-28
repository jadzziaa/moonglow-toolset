//! Tools > Options (Aurora's `TdlgOptions`), as pages: Folders (where the
//! game and the NWN user folder are), General (Build module on save,
//! Minimize Toolset on test module) and Script Editor (compile on save,
//! debug information, the code templates folder, font size and colours),
//! and Moonglow's own (Updates: the daily check for a new release).

use std::path::PathBuf;

use egui::Ui;
use mg_resman::GameInstall;

use crate::script_view::Palette;
use crate::settings::{ModuleFolders, SCRIPT_ELEMENTS, ScriptStyle};
use crate::{Action, Moonglow, Settings};

/// The Options window's pages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OptionsPage {
    #[default]
    Folders,
    Area,
    General,
    ScriptEditor,
    ConversationEditor,
    Sounds,
    Verify,
    Language,
    Keyboard,
    Updates,
}

/// Options › General › Show duplicate tags warning, on hover.
const DUPLICATE_TAGS_TIP: &str = "Warn when a paste or an area's Copy… puts a tag you set on a \
     second object (a script finds only one object by a tag). Off as in Aurora, which has no such \
     warning. A project whose moonglow.toml sets [tags] paste_warning follows that whatever this \
     says, and tags the project wants unique are warned of either way";

/// The Options window's pages and their names.
pub(crate) const PAGES: [(OptionsPage, &str); 10] = [
    (OptionsPage::Folders, "Folders"),
    (OptionsPage::Area, "Area"),
    (OptionsPage::General, "General"),
    (OptionsPage::ScriptEditor, "Script Editor"),
    (OptionsPage::ConversationEditor, "Conversation Editor"),
    (OptionsPage::Sounds, "Sounds"),
    (OptionsPage::Verify, "Verify"),
    (OptionsPage::Language, "Language"),
    (OptionsPage::Keyboard, "Keyboard"),
    (OptionsPage::Updates, "Updates"),
];

/// The Options window's fields: folders as typed (empty to detect), and
/// the other options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsDraft {
    pub page: OptionsPage,
    pub game_root: String,
    pub user_dir: String,
    pub script_style: ScriptStyle,
    pub build_on_save: bool,
    pub mod_beside_folder: bool,
    pub module_folders: ModuleFolders,
    pub area_names: bool,
    pub categories_by_name: bool,
    pub drag_windows_anywhere: bool,
    pub last_area: bool,
    pub reopen_tabs: bool,
    pub tabs_by_recent: bool,
    pub ask_before_closing: bool,
    pub debug_log: bool,
    pub name_resrefs: bool,
    pub palette_cr: bool,
    pub armor_on_woman: bool,
    pub light_theme: bool,
    pub graphics_backend: crate::settings::GraphicsBackend,
    /// Per cent.
    pub ui_scale: u16,
    pub minimize_on_test: bool,
    pub test_with_steam: bool,
    pub auto_reload: bool,
    pub backups: bool,
    pub autosave: bool,
    pub autosave_minutes: u32,
    pub hak_warning: bool,
    pub namespace_warning: bool,
    pub spell_warning: bool,
    pub inventory_warning: bool,
    pub duplicate_tags_warning: bool,
    pub standard_warning: bool,
    pub auto_compile: bool,
    pub live_check: bool,
    pub debug_info: bool,
    pub scripts_external: bool,
    pub scripts_external_only: bool,
    pub scripts_together: bool,
    pub properties_together: bool,
    pub tree_opens_area: bool,
    pub tree_follows_tabs: bool,
    pub dialogs_together: bool,
    pub dialog_font_size: Option<u8>,
    pub script_templates: String,
    pub scratch_dir: String,
    pub external_editor: String,
    pub external_compiler: String,
    pub workshop: bool,
    pub userpatch: bool,
    pub external_compiler_args: String,
    pub dialog_names: bool,
    pub edit_language: Option<u32>,
    pub area_background: Option<[u8; 3]>,
    pub spawn_markers: bool,
    /// Height and Width, tenths of a metre.
    pub spawn_marker_size: (u8, u8),
    pub door_arrows: bool,
    /// Verify: the groups left out of its report, by id.
    pub verify_skip: Vec<String>,
    pub merchant_signs: bool,
    pub edit_group_tiles: bool,
    pub turn_ring: bool,
    pub dialog_npc_color: Option<[u8; 3]>,
    pub dialog_pc_color: Option<[u8; 3]>,
    pub dialog_text_popup: bool,
    pub dialog_paste_source_to_dest: bool,
    pub dialog_drag_source_to_dest: bool,
    pub dialog_backup: bool,
    pub dialog_backup_minutes: u32,
    pub placed_sounds: bool,
    pub ambient_sound: bool,
    pub ambient_music: bool,
    pub music_volume: u8,
    /// Updates: Check for updates automatically, and the version not to be
    /// reminded about.
    pub update_check: bool,
    pub update_skipped: Option<String>,
    pub keymap: crate::keys::Keymap,
    /// The enabled plugins' commands, listed after the window's: the id
    /// each one's keys are kept under, and its name.
    pub plugin_commands: Vec<(String, String)>,
    /// The command (by the id its keys are kept under) taking the next key.
    pub recording: Option<String>,
    /// The game install found on this machine when Options opened, for the
    /// Folders page's hints under an empty field.
    pub detected: Option<GameInstall>,
}

fn text(p: &Option<PathBuf>) -> String {
    p.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
}

fn path(s: &str) -> Option<PathBuf> {
    let s = s.trim();
    (!s.is_empty()).then(|| PathBuf::from(s))
}

impl OptionsDraft {
    pub fn from_settings(s: &Settings) -> OptionsDraft {
        OptionsDraft {
            page: OptionsPage::default(),
            keymap: crate::keys::Keymap::new(&s.key_bindings),
            plugin_commands: Vec::new(),
            recording: None,
            detected: None,
            game_root: text(&s.game_root),
            user_dir: text(&s.user_dir),
            script_style: s.script_style.clone(),
            build_on_save: s.build_on_save,
            mod_beside_folder: !s.no_mod_beside_folder,
            module_folders: s.module_folders,
            area_names: s.area_names,
            categories_by_name: s.categories_by_name,
            drag_windows_anywhere: s.drag_windows_anywhere,
            last_area: !s.no_last_area,
            reopen_tabs: !s.no_reopen_tabs,
            tabs_by_recent: s.tabs_by_recent,
            ask_before_closing: !s.close_without_asking,
            debug_log: !s.no_debug_log,
            name_resrefs: s.name_resrefs,
            palette_cr: !s.palette_no_cr,
            armor_on_woman: s.armor_on_woman,
            light_theme: s.light_theme,
            graphics_backend: s.graphics_backend,
            ui_scale: s.ui_scale.unwrap_or(100),
            minimize_on_test: s.minimize_on_test,
            test_with_steam: !s.test_without_steam,
            auto_reload: !s.no_auto_reload,
            backups: !s.no_backups,
            autosave: !s.no_autosave,
            autosave_minutes: s.autosave_minutes.unwrap_or(5),
            hak_warning: !s.no_hak_warning,
            namespace_warning: !s.no_namespace_warning,
            spell_warning: !s.no_spell_warning,
            inventory_warning: !s.no_inventory_warning,
            duplicate_tags_warning: s.warn_duplicate_tags,
            standard_warning: !s.no_standard_warning,
            auto_compile: s.auto_compile,
            live_check: !s.no_live_check,
            debug_info: s.debug_info,
            scripts_external: s.scripts_external,
            scripts_external_only: s.scripts_external_only,
            scripts_together: !s.scripts_own_windows,
            properties_together: !s.properties_own_windows,
            tree_opens_area: !s.tree_keeps_areas_closed,
            tree_follows_tabs: !s.tree_stays,
            dialogs_together: !s.dialogs_own_windows,
            dialog_font_size: s.dialog_font_size,
            script_templates: text(&s.script_templates),
            scratch_dir: text(&s.scratch_dir),
            external_editor: text(&s.external_editor),
            external_compiler: text(&s.external_compiler),
            workshop: s.read_workshop,
            userpatch: s.read_userpatch,
            external_compiler_args: s.external_compiler_args.clone(),
            dialog_names: !s.dialog_hide_names,
            edit_language: s.edit_language,
            area_background: s.area_background,
            spawn_markers: !s.no_spawn_markers,
            spawn_marker_size: s.spawn_marker_size.unwrap_or(crate::area_view::SPAWN_MARKER),
            door_arrows: !s.no_door_arrows,
            verify_skip: s.verify_skip.clone(),
            merchant_signs: s.merchant_signs,
            edit_group_tiles: s.edit_group_tiles,
            turn_ring: !s.no_turn_ring,
            dialog_npc_color: s.dialog_npc_color,
            dialog_pc_color: s.dialog_pc_color,
            dialog_text_popup: !s.dialog_no_text_popup,
            dialog_paste_source_to_dest: s.dialog_paste_source_to_dest,
            dialog_drag_source_to_dest: !s.dialog_drag_dest_to_source,
            dialog_backup: !s.dialog_no_backup,
            dialog_backup_minutes: s.dialog_backup_minutes.unwrap_or(5),
            placed_sounds: !s.no_placed_sounds,
            ambient_sound: s.ambient_sound,
            ambient_music: s.ambient_music,
            music_volume: s.music_volume.unwrap_or(crate::area_audio::MUSIC_VOLUME),
            update_check: !s.no_update_check,
            update_skipped: s.update_skipped.clone(),
        }
    }

    /// The settings with these options.
    pub fn apply(&self, s: &Settings) -> Settings {
        Settings {
            game_root: path(&self.game_root),
            user_dir: path(&self.user_dir),
            script_style: self.script_style.clone(),
            build_on_save: self.build_on_save,
            no_mod_beside_folder: !self.mod_beside_folder,
            module_folders: self.module_folders,
            area_names: self.area_names,
            categories_by_name: self.categories_by_name,
            drag_windows_anywhere: self.drag_windows_anywhere,
            no_last_area: !self.last_area,
            no_reopen_tabs: !self.reopen_tabs,
            tabs_by_recent: self.tabs_by_recent,
            close_without_asking: !self.ask_before_closing,
            no_debug_log: !self.debug_log,
            name_resrefs: self.name_resrefs,
            palette_no_cr: !self.palette_cr,
            armor_on_woman: self.armor_on_woman,
            light_theme: self.light_theme,
            graphics_backend: self.graphics_backend,
            ui_scale: (self.ui_scale != 100).then_some(self.ui_scale.clamp(UI_SCALES[0], 300)),
            minimize_on_test: self.minimize_on_test,
            test_without_steam: !self.test_with_steam,
            no_auto_reload: !self.auto_reload,
            no_backups: !self.backups,
            no_autosave: !self.autosave,
            autosave_minutes: (self.autosave_minutes != 5)
                .then_some(self.autosave_minutes.clamp(1, 120)),
            no_hak_warning: !self.hak_warning,
            no_namespace_warning: !self.namespace_warning,
            no_spell_warning: !self.spell_warning,
            no_inventory_warning: !self.inventory_warning,
            warn_duplicate_tags: self.duplicate_tags_warning,
            no_standard_warning: !self.standard_warning,
            auto_compile: self.auto_compile,
            no_live_check: !self.live_check,
            debug_info: self.debug_info,
            scripts_external: self.scripts_external,
            scripts_external_only: self.scripts_external_only,
            scripts_own_windows: !self.scripts_together,
            properties_own_windows: !self.properties_together,
            tree_keeps_areas_closed: !self.tree_opens_area,
            tree_stays: !self.tree_follows_tabs,
            dialogs_own_windows: !self.dialogs_together,
            dialog_font_size: self.dialog_font_size,
            script_templates: path(&self.script_templates),
            scratch_dir: path(&self.scratch_dir),
            external_editor: path(&self.external_editor),
            external_compiler: path(&self.external_compiler),
            read_workshop: self.workshop,
            read_userpatch: self.userpatch,
            external_compiler_args: self.external_compiler_args.trim().to_string(),
            dialog_hide_names: !self.dialog_names,
            edit_language: self.edit_language,
            area_background: self.area_background,
            no_spawn_markers: !self.spawn_markers,
            spawn_marker_size: (self.spawn_marker_size != crate::area_view::SPAWN_MARKER)
                .then_some(self.spawn_marker_size),
            no_door_arrows: !self.door_arrows,
            verify_skip: self.verify_skip.clone(),
            merchant_signs: self.merchant_signs,
            edit_group_tiles: self.edit_group_tiles,
            no_turn_ring: !self.turn_ring,
            dialog_npc_color: self.dialog_npc_color,
            dialog_pc_color: self.dialog_pc_color,
            dialog_no_text_popup: !self.dialog_text_popup,
            dialog_paste_source_to_dest: self.dialog_paste_source_to_dest,
            dialog_drag_dest_to_source: !self.dialog_drag_source_to_dest,
            dialog_no_backup: !self.dialog_backup,
            dialog_backup_minutes: (self.dialog_backup_minutes != 5)
                .then_some(self.dialog_backup_minutes.clamp(1, 180)),
            no_placed_sounds: !self.placed_sounds,
            ambient_sound: self.ambient_sound,
            ambient_music: self.ambient_music,
            music_volume: (self.music_volume != crate::area_audio::MUSIC_VOLUME)
                .then_some(self.music_volume.min(127)),
            key_bindings: self.keymap.chosen(),
            no_update_check: !self.update_check,
            update_skipped: self.update_skipped.clone(),
            ..s.clone()
        }
    }

    /// Whether these options change where the game is (which reloads it).
    pub fn moves_game(&self, s: &Settings) -> bool {
        path(&self.game_root) != s.game_root
            || path(&self.user_dir) != s.user_dir
            || self.workshop != s.read_workshop
            || self.userpatch != s.read_userpatch
    }
}

/// Options › General › Interface size: the sizes offered, per cent.
pub(crate) const UI_SCALES: [u16; 7] = [90, 100, 110, 125, 150, 175, 200];

/// The Options window's size when first opened.
pub(crate) const WINDOW_SIZE: [f32; 2] = [780.0, 540.0];

enum Browse {
    Game,
    User,
    Templates,
    Editor,
    Compiler,
    Scratch,
}

/// A row for a folder or a program: its path to type, and Browse… to
/// choose it (`which`, noted in `browse` for after the page is drawn).
fn path_row(ui: &mut Ui, text: &mut String, browse: &mut Option<Browse>, which: Browse) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(text).desired_width(300.0));
        if ui.button("Browse…").clicked() {
            *browse = Some(which);
        }
    });
}

/// The Options tab: the buttons along the bottom, the pages listed beside,
/// and the page scrolling in what is left. What is set is a draft until OK.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let Some(draft) = &mut app.options else {
        // (A tab left from before, with nothing to show.)
        app.actions.push(Action::CloseTab(crate::Tab::Options));
        return;
    };
    let detected = draft.detected.clone();
    let this_run = app.settings.user_dir_this_run.clone();
    let drawing_with = app.drawing_with.clone();
    let (installer, checked) = (app.updates.installer, app.settings.update_checked);
    let mut browse = None;
    let mut close = false;
    // Enter is OK while the pointer is over the tab (it is one window among
    // others now).
    let over = ui.rect_contains_pointer(ui.max_rect());
    {
        {
            egui::Panel::bottom("options-buttons").frame(egui::Frame::NONE.inner_margin(6.0)).show(
                ui,
                |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() || (over && crate::widgets::enter(ui)) {
                            close = true;
                            // (Updates switched on: the check looks at once.)
                            app.updates.look_again();
                            if draft.moves_game(&app.settings)
                                || draft.edit_language != app.settings.edit_language
                            {
                                // (Another language: text is read again in
                                // it, and the game's data where the install
                                // has that language's.)
                                app.actions.push(Action::ApplyOptions(Box::new(draft.clone())));
                            } else {
                                // Only looks: no reload.
                                app.settings = draft.apply(&app.settings);
                            }
                        }
                        // (Escape closes it too, as the window in front:
                        // `tabs.rs`.)
                        if ui.button("Cancel").on_hover_text("Discard changes").clicked() {
                            close = true;
                        }
                    });
                },
            );
            egui::Panel::left("options-pages")
                .resizable(false)
                .exact_size(160.0)
                .frame(egui::Frame::NONE.inner_margin(6.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        for (page, name) in PAGES {
                            ui.selectable_value(&mut draft.page, page, name);
                        }
                    });
                });
            egui::CentralPanel::default().frame(egui::Frame::NONE.inner_margin(6.0)).show(
                ui,
                |ui| {
                    egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| match draft.page {
                        OptionsPage::Keyboard => keyboard(ui, draft),
                        OptionsPage::Folders => {
                            crate::widgets::field_label(ui, "Neverwinter Nights installation");
                            path_row(ui, &mut draft.game_root, &mut browse, Browse::Game);
                            match path(&draft.game_root) {
                                Some(p) if GameInstall::is_install(&p) => {
                                    ui.weak("A game installation.")
                                }
                                Some(_) => ui.colored_label(
                                    ui.visuals().error_fg_color,
                                    "Not a game installation (no data/nwn_base.key).",
                                ),
                                None => match &detected {
                                    Some(d) => {
                                        ui.weak(format!("Empty: found at {}", d.root.display()))
                                    }
                                    None => ui.colored_label(
                                        ui.visuals().warn_fg_color,
                                        "Empty, and none found.",
                                    ),
                                },
                            };
                            ui.add_space(8.0);
                            crate::widgets::field_label(
                                ui,
                                "NWN user folder (haks, override, modules)",
                            );
                            path_row(ui, &mut draft.user_dir, &mut browse, Browse::User);
                            if let Some(run) = &this_run {
                                ui.colored_label(
                                    ui.visuals().warn_fg_color,
                                    format!(
                                        "This run uses {} (--user-dir), not this folder.",
                                        run.display()
                                    ),
                                );
                            } else if path(&draft.user_dir).is_none() {
                                match detected.as_ref().and_then(|d| d.user_dir.as_ref()) {
                                    Some(u) => ui.weak(format!("Empty: {}", u.display())),
                                    None => ui.weak("Empty: none"),
                                };
                            }
                            ui.add_space(6.0);
                            crate::widgets::field_label(ui, "Scratch folder (To Scratch)");
                            path_row(ui, &mut draft.scratch_dir, &mut browse, Browse::Scratch);
                            ui.weak(
                                "Where To Scratch copies a script (with its compiled script) or \
                                 an area as loose files: a development folder, a server's, or \
                                 anywhere. Empty: asked for the first time.",
                            );
                        }
                        OptionsPage::General => {
                            // Aurora's order and groups.
                            ui.checkbox(&mut draft.backups, "Create backups of modules")
                                .on_hover_text(
                                    "Keep the module as it was as <name>.BackupMod at each save",
                                );
                            ui.checkbox(&mut draft.build_on_save, "Build module on save")
                                .on_hover_text(
                                    "Run Build Module (with its defaults) before saving",
                                );
                            ui.checkbox(
                                &mut draft.mod_beside_folder,
                                "Write a .mod beside a module folder on save",
                            )
                            .on_hover_text(
                                "A module opened as a folder is saved into the folder, and as \
                                 <folder>.mod next to it as well, as Aurora saves a module \
                                 directory: the game loads the .mod. (A nasher project has \
                                 Build › Pack for its module file.)",
                            );
                            ui.horizontal(|ui| {
                                crate::widgets::field_label(ui, "A .mod with its folder beside it");
                                let name = |c: &ModuleFolders| match c {
                                    ModuleFolders::Ask => "Ask which to open",
                                    ModuleFolders::Folder => "Open the folder",
                                    ModuleFolders::Module => "Open the .mod",
                                };
                                let all = [
                                    ModuleFolders::Ask,
                                    ModuleFolders::Folder,
                                    ModuleFolders::Module,
                                ];
                                crate::widgets::dropdown("module-folders")
                                    .selected_text(name(&draft.module_folders))
                                    .show(ui, &mut draft.module_folders, &all, name)
                                    .on_hover_text(
                                        "A module opened as name.mod, with a folder named name \
                                         beside it (a module directory, as Aurora keeps one): \
                                         asked, as Aurora asks, or always the folder (Aurora's \
                                         Always open module directories), or always the .mod",
                                    );
                            });
                            ui.checkbox(
                                &mut draft.minimize_on_test,
                                "Minimize Toolset on test module",
                            );
                            ui.checkbox(
                                &mut draft.test_with_steam,
                                "Start a Steam copy of the game through Steam for a test",
                            )
                            .on_hover_text(
                                "On: the game reaches Steam (its overlay, your Workshop content), \
                                 as when started from Steam. Off: the game's program is started \
                                 on its own, without Steam",
                            );
                            ui.horizontal(|ui| {
                                crate::widgets::field_label(ui, "Interface size");
                                crate::widgets::dropdown("ui-scale")
                                    .selected_text(format!("{}%", draft.ui_scale))
                                    .show(ui, &mut draft.ui_scale, &UI_SCALES, |s| format!("{s}%"))
                                    .on_hover_text(
                                        "The whole interface, text and all, larger or smaller \
                                         (set when you choose OK)",
                                    );
                            });
                            ui.checkbox(&mut draft.light_theme, "Light theme");
                            graphics_backend(ui, draft, drawing_with.as_deref());
                            ui.checkbox(
                                &mut draft.last_area,
                                "Open a module on the area opened last",
                            )
                            .on_hover_text(
                                "The first time, on the first area the module tree lists",
                            );
                            ui.checkbox(
                                &mut draft.reopen_tabs,
                                "Open a module with the tabs it had when it was closed",
                            )
                            .on_hover_text(
                                "Its areas, scripts, blueprints, conversations and tools, in \
                                 their order and their places, the one in front in front \
                                 (kept in Moonglow's data folder, not in the module)",
                            );
                            ui.checkbox(
                                &mut draft.ask_before_closing,
                                "Ask before the window is closed",
                            )
                            .on_hover_text(
                                "The window's close button asks first, where there are no \
                                 unsaved changes to ask about (File › Exit does not)",
                            );
                            ui.checkbox(
                                &mut draft.tree_opens_area,
                                "Show the object selected in an area in the module tree",
                            )
                            .on_hover_text(
                                "An object selected in an area's view opens its area out in the \
                                 module tree and goes to it there, as Aurora does; off, the tree \
                                 goes to it only where its area is opened out already",
                            );
                            ui.checkbox(
                                &mut draft.tree_follows_tabs,
                                "Show the tab chosen in the module tree",
                            )
                            .on_hover_text(
                                "A tab chosen (clicked, reached by Ctrl+Tab, or opened) marks its \
                                 script, conversation, blueprint or area in the module tree, its \
                                 group opened and the row brought into view, the keys left with \
                                 the tab; off, the tree stays where it is and marks the area in \
                                 front only, as Aurora does",
                            );
                            ui.checkbox(
                                &mut draft.properties_together,
                                "Open Properties as tabs of one window",
                            )
                            .on_hover_text(
                                "A blueprint's or a placed object's Properties open as a tab of \
                                 the window Properties are open in already; off, each in a \
                                 window of its own. (A tab dragged out is a window of its own \
                                 again)",
                            );
                            ui.checkbox(
                                &mut draft.drag_windows_anywhere,
                                "Move windows by dragging anywhere in them",
                            )
                            .on_hover_text(
                                "A window of tabs (Properties, conversations, scripts…) moves \
                                 when dragged by any part that is not a control; off, only by \
                                 its tab bar, so that a click that misses a control doesn't \
                                 move it",
                            );
                            ui.checkbox(
                                &mut draft.tabs_by_recent,
                                "Ctrl+Tab goes to the tab used last",
                            )
                            .on_hover_text(
                                "Next Tab and Previous Tab go through a pane's tabs in the \
                                 order they were last used, for as long as Ctrl is held; off, \
                                 to the next tab as they lie",
                            );
                            let log = crate::trace::path()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default();
                            ui.checkbox(&mut draft.debug_log, "Write a debug log").on_hover_text(
                                format!(
                                    "What Moonglow does, step by step, to send with a report of \
                                     something that fails without a word, each line with its \
                                     time: {log} (a new one each time Moonglow starts, the last \
                                     five kept, a few megabytes each at most). On unless you \
                                     switch it off: after a crash, what led to it is there to send"
                                ),
                            );
                            ui.checkbox(&mut draft.area_names, "List areas and blueprints by name")
                                .on_hover_text(
                                    "In the module tree, by their names rather than their ResRefs \
                                 (in the names' order; the ResRef shows on hover)",
                                );
                            ui.checkbox(
                                &mut draft.categories_by_name,
                                "List a blueprint's categories alphabetically",
                            )
                            .on_hover_text(
                                "A blueprint's Category list in alphabetical order; off, in the \
                                 palette's (Palette › Categories… sets it)",
                            );
                            ui.checkbox(&mut draft.name_resrefs, "Show ResRefs beside names")
                                .on_hover_text(
                                    "Name (resref) in the module tree's lists by name and in \
                                     the palettes: for telling apart blueprints of one name",
                                );
                            ui.checkbox(
                                &mut draft.palette_cr,
                                "Show challenge ratings in the creature palette",
                            );
                            ui.checkbox(
                                &mut draft.armor_on_woman,
                                "Show armor and cloaks on a woman",
                            )
                            .on_hover_text(
                                "An armor's or a cloak's model on a woman's body, and an \
                                 armor's inventory icon as a woman's inventory shows it, in \
                                 inventories, the palette and the item's Properties (whose \
                                 Shown on changes it too); off, a man's",
                            );
                            ui.checkbox(&mut draft.workshop, "Read Steam Workshop content")
                                .on_hover_text(
                                    "The Steam Workshop items you are subscribed to (overrides, \
                                     haks, talk tables), as the game started through Steam \
                                     reads them. Off, as in Aurora: an item subscribed to for \
                                     play is not the module's. Changing this reads the game's \
                                     data again",
                                );
                            ui.checkbox(&mut draft.userpatch, "Read userpatch.ini's haks")
                                .on_hover_text(
                                    "The haks your userpatch.ini names (the HD Pack and the \
                                     like), from the patch folder, under the module and its haks \
                                     and over override, as the game reads them. Off, as in \
                                     Aurora. Verify warns of what only they provide: players \
                                     without them won't have it. Changing this reads the game's \
                                     data again",
                                );
                            ui.checkbox(
                                &mut draft.auto_reload,
                                "Reload haks, override and development when they change",
                            )
                            .on_hover_text(
                                "Changed haks, 2DAs, models and textures show without a restart; \
                                 a nasher project's files changed by another program are read \
                                 again",
                            );
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut draft.autosave,
                                    "Keep a recovery copy of unsaved work every",
                                )
                                .on_hover_text(
                                    "In Moonglow's data folder; offered back after a crash \
                                     (Recover Unsaved Work)",
                                );
                                ui.add_enabled(
                                    draft.autosave,
                                    egui::DragValue::new(&mut draft.autosave_minutes)
                                        .range(1..=120),
                                );
                                ui.label("minutes");
                            });
                            ui.add_space(8.0);
                            ui.checkbox(
                                &mut draft.namespace_warning,
                                "Show reserved Blueprint ResRef namespace warning",
                            )
                            .on_hover_text(
                                "Warn about a blueprint named like the game's (nw_, x0_ to x3_)",
                            );
                            ui.checkbox(
                                &mut draft.standard_warning,
                                "Show standard resource overwrite warning",
                            )
                            .on_hover_text("Warn when the module adds a resource the game has");
                            ui.checkbox(&mut draft.hak_warning, "Show resource in Hak Pak warning")
                            .on_hover_text(
                                "Warn when a hak has a resource the module adds (the hak's wins)",
                            );
                            ui.checkbox(
                                &mut draft.duplicate_tags_warning,
                                "Show duplicate tags warning after a paste or a copy",
                            )
                            .on_hover_text(DUPLICATE_TAGS_TIP);
                            ui.add_space(8.0);
                            ui.checkbox(
                                &mut draft.spell_warning,
                                "Show invalid creature spell assignment warning",
                            )
                            .on_hover_text(
                                "On a creature's Spells page: spells too high for its level or \
                             ability, or more than it may have",
                            );
                            ui.checkbox(
                                &mut draft.inventory_warning,
                                "Show creature inventory warning",
                            )
                            .on_hover_text(
                                "On a creature's Inventory page: the game may unequip what its \
                                 feats or level do not allow",
                            );
                        }
                        OptionsPage::Area => {
                            ui.horizontal(|ui| {
                                crate::widgets::field_label(ui, "Background Color");
                                let mut custom = draft.area_background.is_some();
                                if ui.checkbox(&mut custom, "Custom").changed() {
                                    draft.area_background = custom.then_some([192, 192, 192]);
                                }
                                if let Some(rgb) = &mut draft.area_background {
                                    crate::color_button::edit_srgb(ui, rgb);
                                } else {
                                    ui.weak("the area's fog");
                                }
                            });
                            ui.checkbox(
                                &mut draft.spawn_markers,
                                "Show Encounter Spawnpoint Markers",
                            );
                            ui.add_enabled_ui(draft.spawn_markers, |ui| {
                                ui.horizontal(|ui| {
                                    crate::widgets::field_label(ui, "Height");
                                    ui.add(
                                        egui::DragValue::new(&mut draft.spawn_marker_size.0)
                                            .range(0..=100),
                                    );
                                    crate::widgets::field_label(ui, "Width");
                                    ui.add(
                                        egui::DragValue::new(&mut draft.spawn_marker_size.1)
                                            .range(0..=100),
                                    );
                                    ui.weak("(tenths of a meter)");
                                });
                            });
                            ui.checkbox(&mut draft.door_arrows, "Show Door Orientation Arrows");
                            ui.checkbox(
                                &mut draft.turn_ring,
                                "Show the turning and tilt rings around selected objects",
                            )
                            .on_hover_text(
                                "A ring around the selection: drag it round to turn the objects. \
                                 With Shift held, two rings that tilt their models \
                                 and arrows that move them along one axis",
                            );
                            ui.checkbox(&mut draft.merchant_signs, "Show stores as $ signs")
                                .on_hover_text(
                                    "The game's marker for a store, as Aurora shows them, \
                                 rather than an arrow along the store's facing",
                                );
                            ui.checkbox(
                                &mut draft.edit_group_tiles,
                                "Allow editing tiles of groups and features",
                            )
                            .on_hover_text(
                                "Off, a group or feature is one thing: its tiles don't turn, \
                                 change appearance or take terrain, no group goes over another, \
                                 and the Eraser or Delete takes it away whole. On, each tile of \
                                 one can be changed alone, which leaves the rest of it standing: \
                                 hard to see and to put right later (Undo does)",
                            );
                            if draft.edit_group_tiles {
                                ui.label(
                                    egui::RichText::new(
                                        "⚠ Tiles of groups and features can be changed one by \
                                         one: a group half changed stays so.",
                                    )
                                    .color(ui.visuals().warn_fg_color),
                                );
                            }
                        }
                        OptionsPage::Verify => verify(ui, draft),
                        OptionsPage::Updates => crate::updates::options_page(
                            ui,
                            &mut draft.update_check,
                            &mut draft.update_skipped,
                            installer,
                            checked,
                        ),
                        OptionsPage::Language => {
                            ui.label("The language text is shown and edited in:");
                            let mut default = draft.edit_language.is_none();
                            if ui
                                .radio_value(&mut default, true, "Use Default Language: English")
                                .clicked()
                            {
                                draft.edit_language = None;
                            }
                            if ui.radio_value(&mut default, false, "Specified Language:").clicked()
                                && draft.edit_language.is_none()
                            {
                                draft.edit_language = Some(0);
                            }
                            ui.add_enabled_ui(!default, |ui| {
                                for l in mg_core::Language::EE {
                                    let name = l.name().unwrap_or("?");
                                    if ui
                                        .selectable_label(draft.edit_language == Some(l.0), name)
                                        .clicked()
                                    {
                                        draft.edit_language = Some(l.0);
                                    }
                                }
                            });
                        }
                        OptionsPage::ConversationEditor => {
                            ui.checkbox(
                                &mut draft.dialog_text_popup,
                                "Show popup when creating a new text entry",
                            );
                            ui.checkbox(&mut draft.dialog_names, "Show speaker name before text");
                            ui.checkbox(
                                &mut draft.dialogs_together,
                                "Open conversations as tabs of one window",
                            )
                            .on_hover_text(
                                "A conversation opens as a tab of the window one is open in \
                                 already; off, each in a window of its own",
                            );
                            ui.horizontal(|ui| {
                                crate::widgets::field_label(ui, "Font size");
                                let usual = egui::TextStyle::Body.resolve(ui.style()).size;
                                let mut size =
                                    draft.dialog_font_size.unwrap_or(usual.round() as u8);
                                let field = egui::DragValue::new(&mut size).range(6..=48);
                                if ui.add(field).changed() {
                                    draft.dialog_font_size = Some(size);
                                }
                                if draft.dialog_font_size.is_some()
                                    && ui.small_button("Default").clicked()
                                {
                                    draft.dialog_font_size = None;
                                }
                            })
                            .response
                            .on_hover_text(
                                "The size of the conversation editor's text: its lines and the \
                                 fields under them",
                            );
                            for (label, color, default) in [
                                ("NPC Text Color", &mut draft.dialog_npc_color, [210, 70, 70]),
                                ("Player Text Color", &mut draft.dialog_pc_color, [90, 140, 230]),
                            ] {
                                ui.horizontal(|ui| {
                                    crate::widgets::field_label(ui, label);
                                    let mut rgb = color.unwrap_or(default);
                                    if crate::color_button::edit_srgb(ui, &mut rgb).changed() {
                                        *color = Some(rgb);
                                    }
                                    if color.is_some() && ui.small_button("Default").clicked() {
                                        *color = None;
                                    }
                                });
                            }
                            ui.add_space(6.0);
                            ui.horizontal_top(|ui| {
                                for (title, source_to_dest) in [
                                    ("Paste Link Options", &mut draft.dialog_paste_source_to_dest),
                                    ("Drag Link Options", &mut draft.dialog_drag_source_to_dest),
                                ] {
                                    ui.group(|ui| {
                                        ui.vertical(|ui| {
                                            crate::widgets::section_heading(ui, title);
                                            ui.radio_value(
                                                source_to_dest,
                                                true,
                                                "Link Source To Destination",
                                            );
                                            ui.radio_value(
                                                source_to_dest,
                                                false,
                                                "Link Destination To Source",
                                            );
                                        });
                                    });
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut draft.dialog_backup,
                                    "Automatically backup the conversation files",
                                )
                                .on_hover_text(
                                    "Every few minutes, a copy of each open conversation with \
                                 unsaved changes, in the temporary folder's moonglow-backups",
                                );
                                ui.add_enabled(
                                    draft.dialog_backup,
                                    egui::DragValue::new(&mut draft.dialog_backup_minutes)
                                        .range(1..=180),
                                );
                                ui.label("minutes");
                            });
                        }
                        OptionsPage::Sounds => {
                            ui.checkbox(
                                &mut draft.placed_sounds,
                                "Play placed sound objects in area",
                            );
                            ui.checkbox(&mut draft.ambient_sound, "Play ambient sound in area");
                            ui.checkbox(&mut draft.ambient_music, "Play ambient music in area");
                            ui.horizontal(|ui| {
                                crate::widgets::field_label(ui, "Ambient music volume");
                                ui.add(egui::Slider::new(&mut draft.music_volume, 0..=127));
                            });
                            ui.weak(
                                "Placed sounds are heard from where the area view looks, at full \
                             volume within their Max Volume Distance and fading out at their \
                             Cutoff Distance.",
                            );
                        }
                        OptionsPage::ScriptEditor => {
                            crate::widgets::field_label(ui, "Code Templates Directory");
                            path_row(
                                ui,
                                &mut draft.script_templates,
                                &mut browse,
                                Browse::Templates,
                            );
                            ui.weak("Listed with the game's (data/scr) and scripttemplates.");
                            ui.checkbox(
                                &mut draft.auto_compile,
                                "Automatically Compile Scripts on Save",
                            );
                            ui.checkbox(
                                &mut draft.debug_info,
                                "Generate Debug Information When Compiling Scripts",
                            )
                            .on_hover_text("Store a .ndb with each compiled script, for debuggers");
                            ui.checkbox(&mut draft.live_check, "Check for errors as you type")
                                .on_hover_text(
                                    "Compile the script a moment after typing stops and show \
                                     its first error under the toolbar, its line underlined",
                                );
                            ui.horizontal(|ui| {
                                crate::widgets::field_label(ui, "Indent with");
                                let style = &mut draft.script_style;
                                ui.radio_value(&mut style.indent_tabs, false, "Spaces");
                                ui.radio_value(&mut style.indent_tabs, true, "Tabs");
                                ui.add_space(12.0);
                                crate::widgets::field_label(ui, "Indent width");
                                ui.add(egui::DragValue::new(&mut style.indent_width).range(1..=16))
                                    .on_hover_text(
                                        "The spaces Tab puts in (to the next multiple of it), \
                                         and Shift+Tab takes off",
                                    );
                            });
                            ui.checkbox(
                                &mut draft.scripts_together,
                                "Open scripts as tabs of one window",
                            )
                            .on_hover_text(
                                "A script opens as a tab of the window scripts are open in \
                                 already; off, each in a window of its own. (A tab dragged out \
                                 is a window of its own again, and scripts docked in the \
                                 middle open there either way)",
                            );
                            crate::widgets::field_label(ui, "External Script Editor");
                            path_row(ui, &mut draft.external_editor, &mut browse, Browse::Editor);
                            ui.add_enabled(
                                !draft.external_editor.trim().is_empty(),
                                egui::Checkbox::new(
                                    &mut draft.scripts_external,
                                    "Open scripts in the external editor",
                                ),
                            )
                            .on_hover_text(
                                "A script opened from the module tree opens in the external \
                                 editor too; what it saves comes back into Moonglow's editor",
                            )
                            .on_disabled_hover_text("Choose an external script editor first");
                            ui.add_enabled(
                                draft.scripts_external && !draft.external_editor.trim().is_empty(),
                                egui::Checkbox::new(
                                    &mut draft.scripts_external_only,
                                    "…and not in Moonglow's editor",
                                ),
                            )
                            .on_hover_text(
                                "Only the external editor opens, for a script kept as a file \
                                 of its own (a nasher project's, a module folder's), whose \
                                 saves Moonglow reads as it does files changed outside. A \
                                 script in a .mod file still opens in Moonglow's editor too: \
                                 that is how what the external editor saves comes back",
                            )
                            .on_disabled_hover_text("With Open scripts in the external editor");
                            crate::widgets::field_label(ui, "External Script Compiler");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut draft.external_compiler)
                                        .hint_text("the built-in compiler")
                                        .desired_width(300.0),
                                )
                                .on_hover_text(
                                    "A compiler program (nwn_script_comp, nwnsc) for Compile, \
                                     Compile All Scripts and the build, in place of the one \
                                     built in (the game's and Aurora's own). Empty: the \
                                     built-in one. Errors as you type are always the built-in \
                                     compiler's",
                                );
                                if ui.button("Browse…").clicked() {
                                    browse = Some(Browse::Compiler);
                                }
                            });
                            let external = draft.external_compiler.trim();
                            if !external.is_empty() {
                                use mg_module::external_compiler::{ExternalCompiler, PLACES};
                                let usual = ExternalCompiler::default_arguments(
                                    std::path::Path::new(external),
                                    draft.debug_info,
                                );
                                crate::widgets::field_label(ui, "Its arguments");
                                ui.add(
                                    egui::TextEdit::singleline(&mut draft.external_compiler_args)
                                        .hint_text(usual.clone())
                                        .desired_width(460.0),
                                )
                                .on_hover_ui(|ui| {
                                    ui.label(format!(
                                        "Empty, the usual for this program:\n{usual}"
                                    ));
                                    ui.add_space(4.0);
                                    for (place, what) in PLACES {
                                        ui.label(format!("{place}  {what}"));
                                    }
                                });
                            }
                            ui.add_space(6.0);
                            script_style(ui, &mut draft.script_style);
                        }
                    });
                },
            );
        }
    }
    if let Some(which @ (Browse::Editor | Browse::Compiler)) = &browse {
        if let Some(p) = app.dialogs.open_file(crate::dialogs::FileKind::Any, None) {
            let field = match which {
                Browse::Compiler => &mut draft.external_compiler,
                _ => &mut draft.external_editor,
            };
            *field = p.display().to_string();
        }
    } else if let Some(which) = browse {
        let (title, field) = match which {
            Browse::Game => ("Neverwinter Nights installation", &mut draft.game_root),
            Browse::User => ("NWN user folder", &mut draft.user_dir),
            Browse::Templates => ("Code Templates Directory", &mut draft.script_templates),
            Browse::Scratch => ("Scratch folder", &mut draft.scratch_dir),
            Browse::Editor | Browse::Compiler => unreachable!("handled above"),
        };
        if let Some(p) = app.dialogs.pick_folder(title, path(field).as_deref()) {
            *field = p.display().to_string();
        }
    }
    let keys_file: Option<bool> = ui.data_mut(|d| d.remove_temp(keys_file_id()));
    match keys_file {
        Some(true) => {
            let suggested = std::path::Path::new("moonglow-keys.json");
            if let Some(p) = app.dialogs.save_file(crate::dialogs::FileKind::Any, Some(suggested)) {
                match std::fs::write(&p, draft.keymap.to_file()) {
                    Ok(()) => app.log.info(format!("Keys written to {}", p.display())),
                    Err(e) => app.log.error(format!("{}: {e}", p.display())),
                }
            }
        }
        Some(false) => {
            if let Some(p) = app.dialogs.open_file(crate::dialogs::FileKind::Any, None) {
                let taken = std::fs::read_to_string(&p)
                    .map_err(|e| e.to_string())
                    .and_then(|text| draft.keymap.take_file(&text));
                match taken {
                    Ok(n) => app.log.info(format!(
                        "Keys of {n} commands taken from {} (OK keeps them)",
                        p.display()
                    )),
                    Err(e) => app.log.error(format!("{}: {e}", p.display())),
                }
            }
        }
        None => {}
    }
    if close {
        app.options = None;
        app.actions.push(Action::CloseTab(crate::Tab::Options));
    }
}

/// The script editor's font size and a colour per syntax element (the
/// theme's unless "Custom" is ticked), with a preview.
/// Options › General › Graphics backend: Automatic and the backends of
/// this platform, from the next start; what this run draws with
/// (`drawing_with`), and whether `WGPU_BACKEND` decides instead.
fn graphics_backend(ui: &mut Ui, draft: &mut OptionsDraft, drawing_with: Option<&str>) {
    use crate::settings::GraphicsBackend;
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Graphics backend");
        let offered = GraphicsBackend::offered(mg_render::backend::Platform::current());
        crate::widgets::dropdown("graphics-backend")
            .selected_text(draft.graphics_backend.name())
            .show(ui, &mut draft.graphics_backend, &offered, |b| b.name())
            .on_hover_text(
                "What the area view, the previews and the window draw with. Automatic: the \
                 graphics library's choice, except that an AMD GPU on Windows draws through \
                 Direct3D 12 rather than Vulkan (AMD's Vulkan driver there drew areas' tiles \
                 wrong). A backend with no adapter here falls back to Automatic",
            );
    });
    let mut note = "Takes effect the next time Moonglow starts.".to_string();
    if let Some(now) = drawing_with {
        note.push_str(&format!(" This run draws with {now}."));
    }
    if std::env::var_os("WGPU_BACKEND").is_some_and(|v| !v.is_empty()) {
        note.push_str(" WGPU_BACKEND is set, and decides over this.");
    }
    ui.weak(note);
}

fn script_style(ui: &mut Ui, style: &mut ScriptStyle) {
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Font size");
        ui.add(egui::DragValue::new(&mut style.font_size).range(6..=48));
    });
    let defaults = Palette::defaults(ui.visuals().dark_mode, ui.visuals().text_color());
    egui::Grid::new("script-colors").num_columns(3).show(ui, |ui| {
        for (i, name) in SCRIPT_ELEMENTS.iter().enumerate() {
            crate::widgets::field_label(ui, *name);
            let mut custom = style.colors[i].is_some();
            if ui.checkbox(&mut custom, "Custom").changed() {
                style.colors[i] = custom.then(|| {
                    let c = defaults[i];
                    [c.r(), c.g(), c.b()]
                });
            }
            if let Some(rgb) = &mut style.colors[i] {
                crate::color_button::edit_srgb(ui, rgb);
            } else {
                ui.weak("theme");
            }
            ui.end_row();
        }
        // The editor's ground.
        let ground = ui.visuals().text_edit_bg_color();
        crate::widgets::field_label(ui, "Background");
        let mut custom = style.background.is_some();
        if ui.checkbox(&mut custom, "Custom").changed() {
            style.background = custom.then(|| [ground.r(), ground.g(), ground.b()]);
        }
        if let Some(rgb) = &mut style.background {
            crate::color_button::edit_srgb(ui, rgb);
        } else {
            ui.weak("theme");
        }
        ui.end_row();
    });
    if ui.button("Reset to defaults").clicked() {
        // (The colors and the size: the indentation is not a look.)
        let (indent_tabs, indent_width) = (style.indent_tabs, style.indent_width);
        *style = ScriptStyle { indent_tabs, indent_width, ..ScriptStyle::default() };
    }
    let preview = "#include \"nw_i0_generic\"\n// A comment\nvoid main()\n{\n    int n = 12;\n    string s = \"text\";\n    object o = OBJECT_SELF; @\n}";
    let palette = Palette::for_ui(style, ui);
    let fill = palette.background.unwrap_or_else(|| ui.visuals().text_edit_bg_color());
    egui::Frame::group(ui.style()).fill(fill).show(ui, |ui| {
        ui.label(crate::script_view::highlight(preview, &palette));
    });
}

/// Options › Verify: what Build › Verify Module reports, by group (a
/// builder's tileset errors nobody is going to fix hid the missing
/// references among them).
fn verify(ui: &mut Ui, draft: &mut OptionsDraft) {
    use mg_module::doctor::{Check, Group};
    ui.label("What Verify Module reports:");
    for group in Group::ALL {
        let mut on = !draft.verify_skip.iter().any(|id| id == group.id());
        let checks: Vec<&str> =
            Check::ALL.iter().filter(|c| c.group() == group).map(|c| c.about()).collect();
        let tip = match group {
            Group::References => {
                "Scripts, conversations, blueprints, areas and other resources the module names \
                 that neither it, its haks nor the game has"
                    .to_string()
            }
            Group::Plugins => "The checks of the enabled plugins".to_string(),
            _ => format!("That {}", checks.join("; that ")),
        };
        if ui.checkbox(&mut on, group.title()).on_hover_text(tip).changed() {
            draft.verify_skip.retain(|id| id != group.id());
            if !on {
                draft.verify_skip.push(group.id().to_string());
            }
        }
    }
    ui.weak("What is left out isn't checked, which is quicker; the log's summary line names it.");
}

/// Options › Keyboard: each command's keys, to add to (the next key
/// pressed), take away or reset; keys two commands share are named.
fn keyboard(ui: &mut Ui, draft: &mut OptionsDraft) {
    use crate::keys::{Cmd, Group, shown};
    let mac = ui.ctx().os() == egui::os::OperatingSystem::Mac;
    // Taking a key: the next press with a key that isn't only a modifier.
    if let Some(id) = draft.recording.clone() {
        let taken = ui.input_mut(|i| {
            let mut got = None;
            i.events.retain(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } if got.is_none() => {
                    got = Some(egui::KeyboardShortcut::new(*modifiers, *key));
                    false
                }
                _ => true,
            });
            got
        });
        if let Some(k) = taken {
            draft.recording = None;
            if k.logical_key != egui::Key::Escape {
                let mut keys = draft.keymap.keys_of(&id);
                let k = egui::KeyboardShortcut::new(
                    egui::Modifiers {
                        command: k.modifiers.command || k.modifiers.ctrl || k.modifiers.mac_cmd,
                        ctrl: false,
                        mac_cmd: false,
                        ..k.modifiers
                    },
                    k.logical_key,
                );
                if !keys.contains(&k) {
                    keys.push(k);
                }
                draft.keymap.set_of(&id, keys);
            }
        }
    }
    ui.label("Click + and press the keys to add them; × takes a key away.");
    let conflicts = draft.keymap.conflicts();
    for (k, a, b) in &conflicts {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "{} is the key of both {} and {}",
                shown(k, mac),
                crate::commands::name_of(a),
                crate::commands::name_of(b)
            ),
        );
    }
    // The window's commands (every one of the menus'), then the keys of
    // the area view and the editors.
    let general =
        crate::commands::Id::all().into_iter().map(|c| (Group::General, c.id(), c.name()));
    let others = Cmd::ALL
        .into_iter()
        .filter(|c| c.group() != Group::General)
        .map(|c| (c.group(), c.id().to_string(), c.name().to_string()));
    let plugins =
        draft.plugin_commands.iter().map(|(id, name)| (Group::Plugins, id.clone(), name.clone()));
    let mut rows: Vec<(Group, String, String)> = general.chain(plugins).chain(others).collect();
    // The list is long: narrow it by a command's name, or its group's.
    let filter_id = egui::Id::new("options-keys-filter");
    let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
    ui.add(
        egui::TextEdit::singleline(&mut filter).hint_text("Find a command").desired_width(240.0),
    );
    ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
    let needle = mg_core::text::Needle::new(&filter);
    rows.retain(|(group, _, name)| {
        needle.finds_in(&[&mg_core::text::fold(name), &mg_core::text::fold(group.name())])
    });
    egui::ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
        let mut group = None;
        egui::Grid::new("keys").num_columns(3).striped(true).show(ui, |ui| {
            for (in_group, id, name) in &rows {
                if group != Some(*in_group) {
                    group = Some(*in_group);
                    crate::widgets::table_heading(ui, in_group.name());
                    ui.end_row();
                }
                ui.label(name);
                ui.horizontal(|ui| {
                    let mut keys = draft.keymap.keys_of(id);
                    let mut remove = None;
                    for (i, k) in keys.iter().enumerate() {
                        if ui
                            .small_button(format!("{} ×", shown(k, mac)))
                            .on_hover_text("Take this key away")
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    }
                    if let Some(i) = remove {
                        keys.remove(i);
                        draft.keymap.set_of(id, keys);
                    }
                    let recording = draft.recording.as_ref() == Some(id);
                    let label = if recording { "press a key…" } else { "+" };
                    let add = ui.small_button(label).on_hover_text("Add a key (Escape: none)");
                    add.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("Add a key for {name}"),
                        )
                    });
                    if add.clicked() {
                        draft.recording = if recording { None } else { Some(id.clone()) };
                    }
                });
                let own = draft.keymap.defaults_of(id);
                let default = draft.keymap.keys_of(id) == own;
                if ui.add_enabled(!default, egui::Button::new("Reset").small()).clicked() {
                    draft.keymap.set_of(id, own);
                }
                ui.end_row();
            }
        });
    });
    ui.horizontal(|ui| {
        if ui.button("Reset All").clicked() {
            draft.keymap.reset_all();
        }
        // (Asked of the file dialogs once the page is drawn.)
        let ask = |ui: &Ui, export: bool| ui.data_mut(|d| d.insert_temp(keys_file_id(), export));
        if ui
            .button("Export…")
            .on_hover_text("The keys you chose, as a file to keep or hand on")
            .clicked()
        {
            ask(ui, true);
        }
        if ui
            .button("Import…")
            .on_hover_text("Take the keys of such a file, in place of these")
            .clicked()
        {
            ask(ui, false);
        }
    });
}

/// Where the Keyboard page leaves its wish for a file dialog (true: to
/// export).
fn keys_file_id() -> egui::Id {
    egui::Id::new("options-keys-file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_alone_do_not_reload_the_game() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        d.script_style.colors[3] = Some([255, 0, 0]);
        d.dialog_font_size = Some(18);
        assert!(!d.moves_game(&s));
        assert_eq!(d.apply(&s).script_style.colors[3], Some([255, 0, 0]));
        assert_eq!(d.apply(&s).dialog_font_size, Some(18));
        d.game_root = "/games/nwn".into();
        assert!(d.moves_game(&s));
    }

    /// Options › Language alone, OK: applied as a language is (text fields
    /// are read again in it), not as a look.
    #[test]
    fn another_language_alone_is_applied_as_one() {
        use egui_kittest::Harness;
        use egui_kittest::kittest::Queryable;
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        // (No game here, whatever this machine has: nothing to read again.)
        app.settings.game_root = Some(mg_testkit::scratch_dir("ui-options-language"));
        let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run();
        h.state_mut().actions.push(Action::OptionsDialog);
        h.run();
        let field = egui::Id::new("a field's text, in English");
        h.state_mut().buffers.insert(field, "English".into());
        h.get_by_label("Language").click();
        h.run();
        h.get_by_label("Specified Language:").click();
        h.run();
        h.get_by_label("German").click();
        h.run();
        h.get_by_label("OK").click();
        h.run();
        crate::text::set_edit_language(mg_core::Language::ENGLISH);
        assert_eq!(h.state().settings.edit_language, Some(mg_core::Language::GERMAN.0));
        assert!(!h.state().buffers.contains_key(&field), "its text is the other language's");
    }

    #[test]
    fn general_and_script_editor_options_apply() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        (d.build_on_save, d.minimize_on_test, d.auto_compile, d.debug_info) =
            (true, true, true, true);
        (d.area_names, d.categories_by_name, d.drag_windows_anywhere) = (true, true, true);
        d.ui_scale = 125;
        (d.name_resrefs, d.palette_cr, d.light_theme) = (true, false, true);
        d.armor_on_woman = true;
        d.script_templates = " /tmp/templates ".into();
        d.scratch_dir = "/tmp/scratch".into();
        assert!(d.live_check, "checked as you type unless asked");
        d.live_check = false;
        let t = d.apply(&s);
        assert!(t.no_live_check);
        assert!(t.build_on_save && t.minimize_on_test && t.auto_compile && t.debug_info);
        assert!(t.area_names && !s.area_names, "areas by ResRef unless asked");
        assert!(t.categories_by_name && !s.categories_by_name, "the palette's order unless asked");
        assert!(t.drag_windows_anywhere && !s.drag_windows_anywhere, "by the tab bar unless asked");
        assert_eq!((t.ui_scale, s.ui_scale), (Some(125), None), "the usual size unless asked");
        assert!(t.name_resrefs && t.palette_no_cr && t.light_theme);
        assert!(t.armor_on_woman && !s.armor_on_woman, "on a man unless asked");
        assert!(OptionsDraft::from_settings(&t).armor_on_woman, "and kept");
        use crate::settings::GraphicsBackend;
        assert_eq!(s.graphics_backend, GraphicsBackend::Automatic, "Automatic unless asked");
        d.graphics_backend = GraphicsBackend::Direct3d12;
        let g = d.apply(&s);
        assert_eq!(g.graphics_backend, GraphicsBackend::Direct3d12);
        assert_eq!(OptionsDraft::from_settings(&g).graphics_backend, GraphicsBackend::Direct3d12);
        // Unless asked: names alone, challenge ratings shown, the dark theme.
        let plain = OptionsDraft::from_settings(&s);
        assert!(!plain.name_resrefs && plain.palette_cr && !plain.light_theme);
        assert_eq!(OptionsDraft::from_settings(&s).apply(&s).ui_scale, None);
        assert_eq!(t.script_templates, Some(PathBuf::from("/tmp/templates")));
        assert_eq!(t.scratch_dir, Some(PathBuf::from("/tmp/scratch")));
        assert_eq!(s.scratch_dir, None, "asked for the first time");
        assert!(!d.moves_game(&s), "no reload for these");
        assert_eq!(OptionsDraft::from_settings(&t).script_templates, "/tmp/templates");
    }

    #[test]
    fn area_options_apply() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        assert!(d.spawn_markers && d.door_arrows, "on by default, as in Aurora");
        assert!(!d.merchant_signs, "stores are arrows unless asked");
        assert!(!d.edit_group_tiles, "groups are one thing unless asked");
        assert!(d.turn_ring, "the turning ring shows unless switched off");
        assert_eq!(d.area_background, None, "the fog color by default");
        (d.spawn_markers, d.area_background) = (false, Some([192, 192, 192]));
        d.edit_group_tiles = true;
        let t = d.apply(&s);
        assert!(t.no_spawn_markers && !t.no_door_arrows);
        assert!(t.edit_group_tiles && OptionsDraft::from_settings(&t).edit_group_tiles);
        assert_eq!(d.spawn_marker_size, (12, 4), "Aurora's Height and Width");
        let mut d = OptionsDraft::from_settings(&t);
        d.spawn_marker_size = (20, 6);
        assert_eq!(d.apply(&t).spawn_marker_size, Some((20, 6)));
        assert_eq!(t.area_background, Some([192, 192, 192]));
    }

    #[test]
    fn conversation_editor_options_apply() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        assert!(d.dialog_names, "names are shown by default, as in Aurora");
        d.dialog_names = false;
        d.dialog_pc_color = Some([1, 2, 3]);
        let t = d.apply(&s);
        assert!(t.dialog_hide_names);
        assert_eq!((t.dialog_pc_color, t.dialog_npc_color), (Some([1, 2, 3]), None));
    }

    #[test]
    fn autosave_is_on_every_five_minutes_by_default() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        assert!(d.autosave);
        assert_eq!(d.autosave_minutes, 5);
        (d.autosave, d.autosave_minutes) = (false, 10);
        let t = d.apply(&s);
        assert!(t.no_autosave);
        assert_eq!(t.autosave_minutes, Some(10));
    }

    #[test]
    fn warnings_are_on_by_default_and_apply() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        assert!(d.namespace_warning && d.spell_warning && d.hak_warning && d.standard_warning);
        assert!(d.inventory_warning);
        (d.spell_warning, d.hak_warning) = (false, false);
        let t = d.apply(&s);
        assert!(t.no_spell_warning && t.no_hak_warning);
        assert!(!t.no_namespace_warning && !t.no_standard_warning);
    }

    /// Duplicate tags after a paste are not warned of unless asked for, as
    /// in Aurora (Troubletcat's report: a spawn system's waypoints share
    /// tags on purpose).
    #[test]
    fn the_duplicate_tags_warning_is_off_until_asked_for() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        assert!(!d.duplicate_tags_warning);
        d.duplicate_tags_warning = true;
        assert!(d.apply(&s).warn_duplicate_tags);
    }

    #[test]
    fn the_editing_language_applies() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        assert_eq!(d.edit_language, None, "English by default");
        d.edit_language = Some(mg_core::Language::FRENCH.0);
        let t = d.apply(&s);
        assert_eq!(t.edit_language, Some(1));
        assert!(!d.moves_game(&s));
        assert_eq!(OptionsDraft::from_settings(&t).edit_language, Some(1));
    }

    #[test]
    fn conversation_link_and_backup_options_apply() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        // Aurora's defaults.
        assert!(d.dialog_text_popup && d.dialog_backup);
        assert!(!d.dialog_paste_source_to_dest && d.dialog_drag_source_to_dest);
        assert_eq!(d.dialog_backup_minutes, 5);
        assert_eq!(d.apply(&s), s, "the defaults change nothing");
        (d.dialog_text_popup, d.dialog_paste_source_to_dest, d.dialog_backup_minutes) =
            (false, true, 12);
        let t = d.apply(&s);
        assert!(t.dialog_no_text_popup && t.dialog_paste_source_to_dest);
        assert_eq!(t.dialog_backup_minutes, Some(12));
    }

    #[test]
    fn sound_options_apply() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        // Aurora's defaults.
        assert!(d.placed_sounds && !d.ambient_sound && !d.ambient_music);
        assert_eq!(d.music_volume, 92);
        (d.placed_sounds, d.ambient_music, d.music_volume) = (false, true, 40);
        let t = d.apply(&s);
        assert!(t.no_placed_sounds && t.ambient_music && !t.ambient_sound);
        assert_eq!(t.music_volume, Some(40));
    }

    #[test]
    fn custom_colours_override_the_theme() {
        let mut style = ScriptStyle::default();
        style.colors[3] = Some([1, 2, 3]);
        style.font_size = 16;
        let p = Palette::new(&style, true, egui::Color32::WHITE);
        assert_eq!(p.colors[3], egui::Color32::from_rgb(1, 2, 3));
        assert_eq!(p.colors[0], egui::Color32::WHITE);
        assert_eq!(p.size, 16.0);
        let job = crate::script_view::highlight("int x;", &p);
        assert_eq!(job.sections[0].format.color, egui::Color32::from_rgb(1, 2, 3));
    }
}
