//! Moonglow Toolset, the desktop application.

use std::path::{Path, PathBuf};

use mg_resman::GameInstall;
use mg_ui::{Dialogs, Moonglow};

/// Native file dialogs.
struct NativeDialogs;

impl Dialogs for NativeDialogs {
    fn open_module(&mut self, start: Option<&Path>) -> Option<PathBuf> {
        let mut d =
            rfd::FileDialog::new().add_filter("Module", &["mod", "nwm"]).set_title("Open Module");
        if let Some(s) = start {
            d = d.set_directory(s);
        }
        d.pick_file()
    }

    fn save_module(&mut self, current: Option<&Path>) -> Option<PathBuf> {
        let mut d =
            rfd::FileDialog::new().add_filter("Module", &["mod"]).set_title("Save Module As");
        if let Some(c) = current {
            if let Some(dir) = c.parent() {
                d = d.set_directory(dir);
            }
            if let Some(name) = c.file_name() {
                d = d.set_file_name(name.to_string_lossy());
            }
        }
        d.save_file()
    }
}

struct App {
    moonglow: Moonglow,
    title: String,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.moonglow.ui(ui);
        let title = self.moonglow.title();
        if title != self.title {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if self.moonglow.quit_requested {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn main() -> eframe::Result<()> {
    let install = GameInstall::detect();
    let mut moonglow = Moonglow::new(install, Box::new(NativeDialogs));
    // `moonglow path/to/module.mod` opens a module at start.
    if let Some(path) = std::env::args_os().nth(1) {
        moonglow.open_module(Path::new(&path));
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Moonglow Toolset"),
        ..Default::default()
    };
    eframe::run_native(
        "Moonglow Toolset",
        options,
        Box::new(|_cc| Ok(Box::new(App { moonglow, title: String::new() }))),
    )
}
