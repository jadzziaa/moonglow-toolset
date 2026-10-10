//! Moonglow Toolset, the desktop application.

// No console window beside the app on Windows (release builds).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

    fn open_file_of(&mut self, title: &str, extensions: &[String]) -> Option<PathBuf> {
        let title = if title.is_empty() { FileKind::Any.title(false) } else { title };
        let mut d = rfd::FileDialog::new().set_title(title);
        if !extensions.is_empty() {
            d = d.add_filter("What the plugin reads", extensions).add_filter("All files", &["*"]);
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

    fn open_files(&mut self, kind: FileKind, start: Option<&Path>) -> Vec<PathBuf> {
        let mut d = rfd::FileDialog::new().set_title(kind.title(false));
        if let Some((name, exts)) = kind.filter(false) {
            d = d.add_filter(name, exts);
        }
        if let Some(s) = start {
            d = d.set_directory(s);
        }
        d.pick_files().unwrap_or_default()
    }
}

const APP_NAME: &str = "Moonglow Toolset";

/// The desktop entry's name (Linux: Wayland's app ID and X11's class, which
/// match the window to `packaging/linux/<APP_ID>.desktop` and its icon).
const APP_ID: &str = "io.github.moonglow_toolset.Moonglow";

/// Where eframe keeps the settings.
const SETTINGS_KEY: &str = "moonglow-settings";

struct App {
    moonglow: Moonglow,
    title: String,
}

/// Windows: the window was last left maximized and is to be maximized
/// once it has drawn (see [`maximize_later`]).
static MAXIMIZE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// On Windows, a window created hidden and maximized is shown at once all
/// the same, blank, then hidden, then shown again when its first frame is
/// drawn: it looks like Moonglow starting twice. So a window left
/// maximized is created as it was before that, and maximized after its
/// first frame ([`MAXIMIZE`]). Elsewhere the window is created as it was
/// left.
fn maximize_later(window: egui::ViewportBuilder, windows: bool) -> egui::ViewportBuilder {
    if windows && window.maximized == Some(true) {
        MAXIMIZE.store(true, std::sync::atomic::Ordering::Relaxed);
        window.with_maximized(false)
    } else {
        window
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Closing the window asks about unsaved work like File > Exit.
        if ui.ctx().input(|i| i.viewport().close_requested()) && !self.moonglow.quit_requested {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.moonglow.actions.push(Action::Quit);
        }
        // (The first frame is drawn hidden; the window shows after it.)
        if ui.ctx().cumulative_frame_nr() >= 1
            && MAXIMIZE.swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(true));
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
        let plugins = match mg_ui::plugins::enabled_now().as_slice() {
            [] => "none".to_string(),
            enabled => enabled.join(", "),
        };
        // A model that failed is left out and Moonglow carries on: the
        // report says so, and is named apart from a crash's.
        let carried_on = mg_ui::model_guard::guarding();
        let what = if carried_on {
            "could not show a model, left it out and carried on"
        } else {
            "crashed"
        };
        let report = format!(
            "Moonglow Toolset {} {what}.\n\n{info}\n\nPlugins enabled: {plugins}\n\nBacktrace:\n{}\n",
            env!("CARGO_PKG_VERSION"),
            std::backtrace::Backtrace::force_capture()
        );
        let name = if carried_on { "model-failure" } else { "crash" };
        let path = dir.join(format!("{name}-{time}.txt"));
        // (In the debug log too, if one is written: where it stopped.)
        mg_ui::trace::note(format!("{what}: {info} (report: {})", path.display()));
        if std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, report)).is_ok() {
            eprintln!("A crash report was written to {}", path.display());
        }
    }));
}

/// The window's icon (`packaging/icons`).
fn window_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../../../packaging/icons/moonglow-256.png"))
        .unwrap_or_default()
}

fn main() -> eframe::Result<()> {
    crash_reports();
    mg_ui::trace::by_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Moonglow Toolset")
            .with_app_id(APP_ID)
            .with_icon(window_icon()),
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
        // The settings stay where they were before the window had an app
        // ID (eframe would otherwise name their folder after it).
        persistence_path: eframe::storage_dir(APP_NAME).map(|d| d.join("app.ron")),
        window_builder: Some(Box::new(|window| maximize_later(window, cfg!(windows)))),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
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
            moonglow.compiled_dir = mg_ui::recovery::data_dir().map(|d| d.join("compiled"));
            // Long work runs while the window keeps drawing.
            moonglow.background_jobs = true;
            // The plugins installed in the data folder (`--no-plugins`
            // starts without them).
            moonglow.plugin_dir = mg_ui::recovery::data_dir().map(|d| d.join("plugins"));
            moonglow.no_plugins = std::env::args().any(|a| a == "--no-plugins");
            moonglow.load_plugins();
            moonglow.prefab_dir = mg_ui::prefabs::dir();
            moonglow.var_set_dir = mg_ui::var_sets::dir();
            moonglow.find_recoveries();
            // Sounds play when there is an output device.
            if let Some(s) = speakers::Speakers::open() {
                moonglow.speaker = Box::new(s);
            }
            // `moonglow path/to/module.mod` opens a module at start.
            let module =
                std::env::args_os().skip(1).find(|a| !a.to_string_lossy().starts_with("--"));
            if let Some(path) = module {
                moonglow.open_module(Path::new(&path));
            }
            Ok(Box::new(App { moonglow, title: String::new() }))
        }),
    )
}

#[cfg(test)]
mod tests {
    /// On Windows a window left maximized is created unmaximized, to be
    /// maximized once drawn; elsewhere, and a window not maximized, as it
    /// was left.
    #[test]
    fn a_maximized_window_is_maximized_after_its_first_frame_on_windows() {
        use std::sync::atomic::Ordering;
        let left = |maximized| egui::ViewportBuilder::default().with_maximized(maximized);
        assert_eq!(super::maximize_later(left(true), false).maximized, Some(true));
        assert_eq!(super::maximize_later(left(false), true).maximized, Some(false));
        assert!(!super::MAXIMIZE.load(Ordering::Relaxed));
        assert_eq!(super::maximize_later(left(true), true).maximized, Some(false));
        assert!(super::MAXIMIZE.swap(false, Ordering::Relaxed));
    }

    #[test]
    fn the_window_icon_decodes() {
        let icon = super::window_icon();
        assert_eq!((icon.width, icon.height, icon.rgba.len()), (256, 256, 256 * 256 * 4));
    }
}
