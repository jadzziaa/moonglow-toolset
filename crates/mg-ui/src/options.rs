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
    pub hak_warning: bool,
    pub namespace_warning: bool,
    pub standard_warning: bool,
    pub auto_compile: bool,
    pub debug_info: bool,
    pub script_templates: String,
    pub external_editor: String,
    pub dialog_names: bool,
    pub edit_language: Option<u32>,
    pub area_background: Option<[u8; 3]>,
    pub spawn_markers: bool,
    pub door_arrows: bool,
    pub dialog_npc_color: Option<[u8; 3]>,
    pub dialog_pc_color: Option<[u8; 3]>,
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
            hak_warning: !s.no_hak_warning,
            namespace_warning: !s.no_namespace_warning,
            standard_warning: !s.no_standard_warning,
            auto_compile: s.auto_compile,
            debug_info: s.debug_info,
            script_templates: text(&s.script_templates),
            external_editor: text(&s.external_editor),
            dialog_names: !s.dialog_hide_names,
            edit_language: s.edit_language,
            area_background: s.area_background,
            spawn_markers: !s.no_spawn_markers,
            door_arrows: !s.no_door_arrows,
            dialog_npc_color: s.dialog_npc_color,
            dialog_pc_color: s.dialog_pc_color,
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
            no_hak_warning: !self.hak_warning,
            no_namespace_warning: !self.namespace_warning,
            no_standard_warning: !self.standard_warning,
            auto_compile: self.auto_compile,
            debug_info: self.debug_info,
            script_templates: path(&self.script_templates),
            external_editor: path(&self.external_editor),
            dialog_hide_names: !self.dialog_names,
            edit_language: self.edit_language,
            area_background: self.area_background,
            no_spawn_markers: !self.spawn_markers,
            no_door_arrows: !self.door_arrows,
            dialog_npc_color: self.dialog_npc_color,
            dialog_pc_color: self.dialog_pc_color,
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
                        ui.checkbox(&mut draft.build_on_save, "Build module on save")
                            .on_hover_text("Run Build Module (with its defaults) before saving");
                        ui.checkbox(&mut draft.minimize_on_test, "Minimize Toolset on test module");
                        ui.checkbox(&mut draft.backups, "Create backups of modules").on_hover_text(
                            "Keep the module as it was as <name>.BackupMod at each save",
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
        assert_eq!(d.area_background, None, "the fog colour by default");
        (d.spawn_markers, d.area_background) = (false, Some([192, 192, 192]));
        let t = d.apply(&s);
        assert!(t.no_spawn_markers && !t.no_door_arrows);
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
