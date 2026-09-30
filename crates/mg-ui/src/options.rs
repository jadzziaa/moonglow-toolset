//! Tools > Options: where the game and the NWN user folder are.

use std::path::PathBuf;

use egui::Ui;
use mg_resman::GameInstall;

use crate::{Action, Moonglow, Settings};

/// The Options window's fields: folders as typed (empty to detect).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsDraft {
    pub game_root: String,
    pub user_dir: String,
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
        OptionsDraft { game_root: text(&s.game_root), user_dir: text(&s.user_dir) }
    }

    /// The settings with these folders.
    pub fn apply(&self, s: &Settings) -> Settings {
        Settings { game_root: path(&self.game_root), user_dir: path(&self.user_dir), ..s.clone() }
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
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    close = true;
                    if draft.apply(&app.settings) != app.settings {
                        app.actions.push(Action::ApplyOptions(draft.clone()));
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
