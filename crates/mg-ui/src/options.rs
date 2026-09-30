//! Tools > Options: where the game and the NWN user folder are, and the
//! script editor's font size and colours.

use std::path::PathBuf;

use egui::Ui;
use mg_resman::GameInstall;

use crate::script_view::Palette;
use crate::settings::{SCRIPT_ELEMENTS, ScriptStyle};
use crate::{Action, Moonglow, Settings};

/// The Options window's fields: folders as typed (empty to detect), and
/// the script editor's style.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsDraft {
    pub game_root: String,
    pub user_dir: String,
    pub script_style: ScriptStyle,
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
            game_root: text(&s.game_root),
            user_dir: text(&s.user_dir),
            script_style: s.script_style.clone(),
        }
    }

    /// The settings with these options.
    pub fn apply(&self, s: &Settings) -> Settings {
        Settings {
            game_root: path(&self.game_root),
            user_dir: path(&self.user_dir),
            script_style: self.script_style.clone(),
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
            ui.label("Neverwinter Nights installation");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut draft.game_root).desired_width(360.0));
                if ui.button("Browse…").clicked() {
                    browse = Some(Browse::Game);
                }
            });
            match path(&draft.game_root) {
                Some(p) if GameInstall::is_install(&p) => ui.weak("A game installation."),
                Some(_) => ui.colored_label(
                    ui.visuals().error_fg_color,
                    "Not a game installation (no data/nwn_base.key).",
                ),
                None => match &detected {
                    Some(d) => ui.weak(format!("Empty: found at {}", d.root.display())),
                    None => ui.colored_label(ui.visuals().warn_fg_color, "Empty, and none found."),
                },
            };
            ui.add_space(8.0);
            ui.label("NWN user folder (haks, override, modules)");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut draft.user_dir).desired_width(360.0));
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
            ui.add_space(8.0);
            egui::CollapsingHeader::new("Script Editor").show(ui, |ui| {
                script_style(ui, &mut draft.script_style);
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    close = true;
                    if draft.moves_game(&app.settings) {
                        app.actions.push(Action::ApplyOptions(draft.clone()));
                    } else {
                        // Only looks: no reload.
                        app.settings = draft.apply(&app.settings);
                    }
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    if let Some(which) = browse {
        let (title, field) = match which {
            Browse::Game => ("Neverwinter Nights installation", &mut draft.game_root),
            Browse::User => ("NWN user folder", &mut draft.user_dir),
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
