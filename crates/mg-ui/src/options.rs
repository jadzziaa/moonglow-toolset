//! Tools > Options (Aurora's `TdlgOptions`), as pages: Folders (where the
//! game and the NWN user folder are), General (Build module on save,
//! Minimize Toolset on test module) and Script Editor (compile on save,
//! debug information, the code templates folder, font size and colours).

use std::path::PathBuf;

use egui::Ui;
use mg_resman::GameInstall;

use crate::script_view::Palette;
use crate::settings::{SCRIPT_ELEMENTS, ScriptStyle};
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
    Language,
}

/// The Options window's fields: folders as typed (empty to detect), and
/// the other options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsDraft {
    pub page: OptionsPage,
    pub game_root: String,
    pub user_dir: String,
    pub script_style: ScriptStyle,
    pub build_on_save: bool,
    pub minimize_on_test: bool,
    pub backups: bool,
    pub autosave: bool,
    pub autosave_minutes: u32,
    pub hak_warning: bool,
    pub namespace_warning: bool,
    pub spell_warning: bool,
    pub inventory_warning: bool,
    pub standard_warning: bool,
    pub auto_compile: bool,
    pub debug_info: bool,
    pub script_templates: String,
    pub external_editor: String,
    pub dialog_names: bool,
    pub edit_language: Option<u32>,
    pub area_background: Option<[u8; 3]>,
    pub spawn_markers: bool,
    /// Height and Width, tenths of a metre.
    pub spawn_marker_size: (u8, u8),
    pub door_arrows: bool,
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
            game_root: text(&s.game_root),
            user_dir: text(&s.user_dir),
            script_style: s.script_style.clone(),
            build_on_save: s.build_on_save,
            minimize_on_test: s.minimize_on_test,
            backups: !s.no_backups,
            autosave: !s.no_autosave,
            autosave_minutes: s.autosave_minutes.unwrap_or(5),
            hak_warning: !s.no_hak_warning,
            namespace_warning: !s.no_namespace_warning,
            spell_warning: !s.no_spell_warning,
            inventory_warning: !s.no_inventory_warning,
            standard_warning: !s.no_standard_warning,
            auto_compile: s.auto_compile,
            debug_info: s.debug_info,
            script_templates: text(&s.script_templates),
            external_editor: text(&s.external_editor),
            dialog_names: !s.dialog_hide_names,
            edit_language: s.edit_language,
            area_background: s.area_background,
            spawn_markers: !s.no_spawn_markers,
            spawn_marker_size: s.spawn_marker_size.unwrap_or(crate::area_view::SPAWN_MARKER),
            door_arrows: !s.no_door_arrows,
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
        }
    }

    /// The settings with these options.
    pub fn apply(&self, s: &Settings) -> Settings {
        Settings {
            game_root: path(&self.game_root),
            user_dir: path(&self.user_dir),
            script_style: self.script_style.clone(),
            build_on_save: self.build_on_save,
            minimize_on_test: self.minimize_on_test,
            no_backups: !self.backups,
            no_autosave: !self.autosave,
            autosave_minutes: (self.autosave_minutes != 5)
                .then_some(self.autosave_minutes.clamp(1, 120)),
            no_hak_warning: !self.hak_warning,
            no_namespace_warning: !self.namespace_warning,
            no_spell_warning: !self.spell_warning,
            no_inventory_warning: !self.inventory_warning,
            no_standard_warning: !self.standard_warning,
            auto_compile: self.auto_compile,
            debug_info: self.debug_info,
            script_templates: path(&self.script_templates),
            external_editor: path(&self.external_editor),
            dialog_hide_names: !self.dialog_names,
            edit_language: self.edit_language,
            area_background: self.area_background,
            no_spawn_markers: !self.spawn_markers,
            spawn_marker_size: (self.spawn_marker_size != crate::area_view::SPAWN_MARKER)
                .then_some(self.spawn_marker_size),
            no_door_arrows: !self.door_arrows,
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
            ..s.clone()
        }
    }

    /// Whether these options change where the game is (which reloads it).
    pub fn moves_game(&self, s: &Settings) -> bool {
        path(&self.game_root) != s.game_root || path(&self.user_dir) != s.user_dir
    }
}

enum Browse {
    Game,
    User,
    Templates,
    Editor,
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let Some(draft) = &mut app.options else { return };
    let ctx = ui.ctx().clone();
    let detected = GameInstall::detect();
    let mut browse = None;
    let mut close = false;
    egui::Window::new("Options")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(&ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(110.0);
                    for (page, name) in [
                        (OptionsPage::Folders, "Folders"),
                        (OptionsPage::Area, "Area"),
                        (OptionsPage::General, "General"),
                        (OptionsPage::ScriptEditor, "Script Editor"),
                        (OptionsPage::ConversationEditor, "Conversation Editor"),
                        (OptionsPage::Sounds, "Sounds"),
                        (OptionsPage::Language, "Language"),
                    ] {
                        ui.selectable_value(&mut draft.page, page, name);
                    }
                });
                ui.add_space(12.0);
                ui.vertical(|ui| match draft.page {
                    OptionsPage::Folders => {
                        ui.label("Neverwinter Nights installation");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut draft.game_root)
                                    .desired_width(300.0),
                            );
                            if ui.button("Browse…").clicked() {
                                browse = Some(Browse::Game);
                            }
                        });
                        match path(&draft.game_root) {
                            Some(p) if GameInstall::is_install(&p) => {
                                ui.weak("A game installation.")
                            }
                            Some(_) => ui.colored_label(
                                ui.visuals().error_fg_color,
                                "Not a game installation (no data/nwn_base.key).",
                            ),
                            None => match &detected {
                                Some(d) => ui.weak(format!("Empty: found at {}", d.root.display())),
                                None => ui.colored_label(
                                    ui.visuals().warn_fg_color,
                                    "Empty, and none found.",
                                ),
                            },
                        };
                        ui.add_space(8.0);
                        ui.label("NWN user folder (haks, override, modules)");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut draft.user_dir)
                                    .desired_width(300.0),
                            );
                            if ui.button("Browse…").clicked() {
                                browse = Some(Browse::User);
                            }
                        });
                        if path(&draft.user_dir).is_none() {
                            match detected.as_ref().and_then(|d| d.user_dir.as_ref()) {
                                Some(u) => ui.weak(format!("Empty: {}", u.display())),
                                None => ui.weak("Empty: none"),
                            };
                        }
                    }
                    OptionsPage::General => {
                        // Aurora's order and groups.
                        ui.checkbox(&mut draft.backups, "Create backups of modules").on_hover_text(
                            "Keep the module as it was as <name>.BackupMod at each save",
                        );
                        ui.checkbox(&mut draft.build_on_save, "Build module on save")
                            .on_hover_text("Run Build Module (with its defaults) before saving");
                        ui.checkbox(&mut draft.minimize_on_test, "Minimize Toolset on test module");
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
                                egui::DragValue::new(&mut draft.autosave_minutes).range(1..=120),
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
                            ui.label("Background Color");
                            let mut custom = draft.area_background.is_some();
                            if ui.checkbox(&mut custom, "Custom").changed() {
                                draft.area_background = custom.then_some([192, 192, 192]);
                            }
                            if let Some(rgb) = &mut draft.area_background {
                                ui.color_edit_button_srgb(rgb);
                            } else {
                                ui.weak("the area's fog");
                            }
                        });
                        ui.checkbox(&mut draft.spawn_markers, "Show Encounter Spawnpoint Markers");
                        ui.add_enabled_ui(draft.spawn_markers, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Height");
                                ui.add(
                                    egui::DragValue::new(&mut draft.spawn_marker_size.0)
                                        .range(0..=100),
                                );
                                ui.label("Width");
                                ui.add(
                                    egui::DragValue::new(&mut draft.spawn_marker_size.1)
                                        .range(0..=100),
                                );
                                ui.weak("(tenths of a meter)");
                            });
                        });
                        ui.checkbox(&mut draft.door_arrows, "Show Door Orientation Arrows");
                    }
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
                        for (label, color, default) in [
                            ("NPC Text Color", &mut draft.dialog_npc_color, [210, 70, 70]),
                            ("Player Text Color", &mut draft.dialog_pc_color, [90, 140, 230]),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(label);
                                let mut rgb = color.unwrap_or(default);
                                if ui.color_edit_button_srgb(&mut rgb).changed() {
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
                                        ui.strong(title);
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
                        ui.checkbox(&mut draft.placed_sounds, "Play placed sound objects in area");
                        ui.checkbox(&mut draft.ambient_sound, "Play ambient sound in area");
                        ui.checkbox(&mut draft.ambient_music, "Play ambient music in area");
                        ui.horizontal(|ui| {
                            ui.label("Ambient music volume");
                            ui.add(egui::Slider::new(&mut draft.music_volume, 0..=127));
                        });
                        ui.weak(
                            "Placed sounds are heard from where the area view looks, at full \
                             volume within their Max Volume Distance and fading out at their \
                             Cutoff Distance.",
                        );
                    }
                    OptionsPage::ScriptEditor => {
                        ui.label("Code Templates Directory");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut draft.script_templates)
                                    .desired_width(300.0),
                            );
                            if ui.button("Browse…").clicked() {
                                browse = Some(Browse::Templates);
                            }
                        });
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
                        ui.label("External Script Editor");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut draft.external_editor)
                                    .desired_width(300.0),
                            );
                            if ui.button("Browse…").clicked() {
                                browse = Some(Browse::Editor);
                            }
                        });
                        ui.add_space(6.0);
                        script_style(ui, &mut draft.script_style);
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    close = true;
                    if draft.moves_game(&app.settings) {
                        app.actions.push(Action::ApplyOptions(draft.clone()));
                    } else {
                        // Only looks: no reload.
                        app.settings = draft.apply(&app.settings);
                        crate::text::set_edit_language(mg_core::Language(
                            app.settings.edit_language.unwrap_or(0),
                        ));
                    }
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    if let Some(Browse::Editor) = browse {
        if let Some(p) = app.dialogs.open_file(crate::dialogs::FileKind::Any, None) {
            draft.external_editor = p.display().to_string();
        }
    } else if let Some(which) = browse {
        let (title, field) = match which {
            Browse::Game => ("Neverwinter Nights installation", &mut draft.game_root),
            Browse::User => ("NWN user folder", &mut draft.user_dir),
            Browse::Templates => ("Code Templates Directory", &mut draft.script_templates),
            Browse::Editor => unreachable!("handled above"),
        };
        if let Some(p) = app.dialogs.pick_folder(title, path(field).as_deref()) {
            *field = p.display().to_string();
        }
    }
    if close {
        app.options = None;
    }
}

/// The script editor's font size and a colour per syntax element (the
/// theme's unless "Custom" is ticked), with a preview.
fn script_style(ui: &mut Ui, style: &mut ScriptStyle) {
    ui.horizontal(|ui| {
        ui.label("Font size");
        ui.add(egui::DragValue::new(&mut style.font_size).range(6..=48));
    });
    let defaults = Palette::defaults(ui.visuals().dark_mode, ui.visuals().text_color());
    egui::Grid::new("script-colors").num_columns(3).show(ui, |ui| {
        for (i, name) in SCRIPT_ELEMENTS.iter().enumerate() {
            ui.label(*name);
            let mut custom = style.colors[i].is_some();
            if ui.checkbox(&mut custom, "Custom").changed() {
                style.colors[i] = custom.then(|| {
                    let c = defaults[i];
                    [c.r(), c.g(), c.b()]
                });
            }
            if let Some(rgb) = &mut style.colors[i] {
                ui.color_edit_button_srgb(rgb);
            } else {
                ui.weak("theme");
            }
            ui.end_row();
        }
    });
    if ui.button("Reset to defaults").clicked() {
        *style = ScriptStyle::default();
    }
    let preview = "#include \"nw_i0_generic\"\n// A comment\nvoid main()\n{\n    int n = 12;\n    string s = \"text\";\n    object o = OBJECT_SELF; @\n}";
    let palette = Palette::for_ui(style, ui);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(crate::script_view::highlight(preview, &palette));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_alone_do_not_reload_the_game() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        d.script_style.colors[3] = Some([255, 0, 0]);
        assert!(!d.moves_game(&s));
        assert_eq!(d.apply(&s).script_style.colors[3], Some([255, 0, 0]));
        d.game_root = "/games/nwn".into();
        assert!(d.moves_game(&s));
    }

    #[test]
    fn general_and_script_editor_options_apply() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        (d.build_on_save, d.minimize_on_test, d.auto_compile, d.debug_info) =
            (true, true, true, true);
        d.script_templates = " /tmp/templates ".into();
        let t = d.apply(&s);
        assert!(t.build_on_save && t.minimize_on_test && t.auto_compile && t.debug_info);
        assert_eq!(t.script_templates, Some(PathBuf::from("/tmp/templates")));
        assert!(!d.moves_game(&s), "no reload for these");
        assert_eq!(OptionsDraft::from_settings(&t).script_templates, "/tmp/templates");
    }

    #[test]
    fn area_options_apply() {
        let s = Settings::default();
        let mut d = OptionsDraft::from_settings(&s);
        assert!(d.spawn_markers && d.door_arrows, "on by default, as in Aurora");
        assert_eq!(d.area_background, None, "the fog color by default");
        (d.spawn_markers, d.area_background) = (false, Some([192, 192, 192]));
        let t = d.apply(&s);
        assert!(t.no_spawn_markers && !t.no_door_arrows);
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
