//! Moonglow Toolset, the desktop application.

use std::path::{Path, PathBuf};

use mg_ui::{Action, Dialogs, FileKind, Moonglow, Settings};

mod speakers;

/// Native file dialogs.
struct NativeDialogs;

impl Dialogs for NativeDialogs {
    fn open_file(&mut self, kind: FileKind, start: Option<&Path>) -> Option<PathBuf> {
        let mut d = rfd::FileDialog::new().set_title(kind.title(false));
        if let Some((name, exts)) = kind.filter(false) {
            d = d.add_filter(name, exts);
        }
        if let Some(s) = start {
            d = d.set_directory(s);
        }
        d.pick_file()
    }

    fn save_file(&mut self, kind: FileKind, suggested: Option<&Path>) -> Option<PathBuf> {
        let mut d = rfd::FileDialog::new().set_title(kind.title(true));
        if let Some((name, exts)) = kind.filter(true) {
            d = d.add_filter(name, exts);
        }
        if let Some(c) = suggested {
            if let Some(dir) = c.parent() {
                d = d.set_directory(dir);
            }
            if let Some(name) = c.file_name() {
                d = d.set_file_name(name.to_string_lossy());
            }
        }
        d.save_file()
    }

    fn pick_folder(&mut self, title: &str, start: Option<&Path>) -> Option<PathBuf> {
        let mut d = rfd::FileDialog::new().set_title(title);
        if let Some(s) = start {
            d = d.set_directory(s);
        }
        d.pick_folder()
    }
}

/// Where eframe keeps the settings.
const SETTINGS_KEY: &str = "moonglow-settings";

struct App {
    moonglow: Moonglow,
    title: String,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Closing the window asks about unsaved work like File > Exit.
        if ui.ctx().input(|i| i.viewport().close_requested()) && !self.moonglow.quit_requested {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.moonglow.actions.push(Action::Quit);
        }
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

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, SETTINGS_KEY, &self.moonglow.settings);
    }
}

/// On a crash, a report in Moonglow's data folder (`crash-<time>.txt`):
/// the message, where, and the backtrace; the unsaved work is in the
/// recovery copy.
fn crash_reports() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        let Some(dir) = mg_ui::recovery::data_dir() else { return };
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let report = format!(
            "Moonglow Toolset {} crashed.\n\n{info}\n\nBacktrace:\n{}\n",
            env!("CARGO_PKG_VERSION"),
            std::backtrace::Backtrace::force_capture()
        );
        let path = dir.join(format!("crash-{time}.txt"));
        if std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, report)).is_ok() {
            eprintln!("A crash report was written to {}", path.display());
        }
    }));
}

fn main() -> eframe::Result<()> {
    crash_reports();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Moonglow Toolset"),
        wgpu_options: egui_wgpu::WgpuConfiguration {
            wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(egui_wgpu::WgpuSetupCreateNew {
                // Game textures are mostly BC-compressed: upload them as
                // they are where the GPU can read them.
                device_descriptor: std::sync::Arc::new(|adapter| wgpu::DeviceDescriptor {
                    label: Some("moonglow"),
                    required_features: adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC,
                    required_limits: adapter.limits(),
                    ..Default::default()
                }),
                ..egui_wgpu::WgpuSetupCreateNew::without_display_handle()
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "Moonglow Toolset",
        options,
        Box::new(|cc| {
            let settings: Settings =
                cc.storage.and_then(|s| eframe::get_value(s, SETTINGS_KEY)).unwrap_or_default();
            let mut moonglow = Moonglow::with_settings(settings, Box::new(NativeDialogs));
            if let Some(rs) = &cc.wgpu_render_state {
                moonglow.set_render_state(rs.clone());
            }
            // Recovery copies of unsaved work go to Moonglow's data folder;
            // those a crashed session left are offered back.
            moonglow.recovery_dir = mg_ui::recovery::data_dir().map(|d| d.join("recovery"));
            moonglow.find_recoveries();
            // Sounds play when there is an output device.
            if let Some(s) = speakers::Speakers::open() {
                moonglow.speaker = Box::new(s);
            }
            // `moonglow path/to/module.mod` opens a module at start.
            if let Some(path) = std::env::args_os().nth(1) {
                moonglow.open_module(Path::new(&path));
            }
            Ok(Box::new(App { moonglow, title: String::new() }))
        }),
    )
}
