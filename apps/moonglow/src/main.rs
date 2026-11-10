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

    fn save_file_of(
        &mut self,
        title: &str,
        extension: &str,
        suggested: Option<&Path>,
    ) -> Option<PathBuf> {
        let title = if title.is_empty() { FileKind::Any.title(true) } else { title };
        let mut d = rfd::FileDialog::new().set_title(title);
        if !extension.is_empty() {
            d = d.add_filter("What the plugin writes", &[extension]);
        }
        if let Some(c) = suggested {
            if let Some(dir) = c.parent().filter(|d| !d.as_os_str().is_empty()) {
                d = d.set_directory(dir);
            }
            if let Some(name) = c.file_name() {
                d = d.set_file_name(name.to_string_lossy());
            }
        }
        d.save_file()
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
    /// The settings' file, and the settings as this window last read or
    /// saved them with the text it wrote: what another Moonglow saved
    /// since is kept, but for the settings changed here ([`App::save`]).
    settings_file: Option<std::path::PathBuf>,
    saved: (Settings, String),
    /// What is left for `main` once the window has closed (Restart with
    /// …, Restart Now; an update to install).
    again: std::sync::Arc<std::sync::Mutex<Closing>>,
}

/// What is done once the window has closed.
#[derive(Default)]
struct Closing {
    /// Moonglow started again (Restart with …, an update's Restart Now).
    relaunch: Option<mg_ui::renderer_check::Relaunch>,
    /// An update ready, installed as Moonglow closes, and the updates'
    /// cache folder (the installer's log, the install's report).
    update: Option<(mg_update::Staged, std::path::PathBuf)>,
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
            self.moonglow.actions.push(Action::CloseWindow);
        }
        // (The first frame is drawn hidden; the window shows after it.)
        if ui.ctx().cumulative_frame_nr() >= 1
            && MAXIMIZE.swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
        // (A panic keeps the unsaved work in a recovery copy first.)
        self.moonglow.recovering(|m| m.ui(ui));
        // The device lost: the window draws nothing more, so the builder
        // is told by a dialog of the system's (once; the log has it too).
        if let Some(said) = self.moonglow.renderer_check.device_lost.take() {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Moonglow Toolset")
                .set_description(said)
                .show();
        }
        let title = self.moonglow.title();
        if title != self.title {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if self.moonglow.quit_requested {
            // (Each only when there is one: a frame drawn after Close is
            // asked for finds none, and leaves what the first one left.)
            if let Ok(mut left) = self.again.lock() {
                if let Some(again) = self.moonglow.relaunch.take() {
                    left.relaunch = Some(again);
                }
                // (An update is only ever downloaded into the cache folder.)
                if let Some(cache) = self.moonglow.updates.cache.clone()
                    && let Some(staged) = self.moonglow.updates.at_close()
                {
                    left.update = Some((staged, cache));
                }
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Saves the settings. The file is this window's and any other
    /// Moonglow's that runs beside it, each writing it whole: where
    /// another has written it since this one did, the settings changed
    /// here since then go over what is there, and the rest stay as that
    /// window saved them (`Settings::saved_over`), where this one would
    /// have put its own older ones back.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let settings = &self.moonglow.settings;
        let there = (self.settings_file.as_deref())
            .and_then(|f| Settings::text_in_eframe_file(f, SETTINGS_KEY));
        let text = match there {
            Some(there) if there != self.saved.1 => settings.saved_over(&self.saved.0, &there),
            _ => settings.saved_text(),
        };
        storage.set_string(SETTINGS_KEY, text.clone());
        self.saved = (settings.clone(), text);
    }
}

/// A panic in eframe's own code (outside the frame `recovering` runs)
/// unwinds through the event loop and drops the application: the unsaved
/// work is kept then.
impl Drop for App {
    fn drop(&mut self) {
        self.moonglow.keep_while_unwinding();
    }
}

/// On a crash, a report in Moonglow's data folder (`crash-<time>.txt`):
/// the message, where, and the backtrace; the unsaved work is in the
/// recovery copy, written as the frame stopped (`Moonglow::recovering`).
fn crash_reports() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        default(info);
        let Some(dir) = mg_ui::recovery::data_dir() else { return };
        let time = mg_core::time::unix_now();
        let plugins = match mg_ui::plugins::enabled_now().as_slice() {
            [] => "none".to_string(),
            enabled => enabled.join(", "),
        };
        // A model that failed is left out and Moonglow carries on: the
        // report says so, and is named apart from a crash's. So a
        // graphics check that panicked (a backend opening its device):
        // its finding, which the check tells.
        let checking = mg_ui::model_guard::checking();
        let carried_on = checking || mg_ui::model_guard::guarding();
        // (A panic in a thread of its own leaves the window going: the
        // next frame keeps the unsaved work in a recovery copy.)
        if !carried_on {
            mg_ui::recovery::note_panic();
        }
        let what = if checking {
            "found a graphics check failing and carried on"
        } else if carried_on {
            "could not show a model, left it out and carried on"
        } else {
            "crashed"
        };
        let report = format!(
            "Moonglow Toolset {} {what}.\n\n{info}\n\nPlugins enabled: {plugins}\n\nBacktrace:\n{}\n",
            env!("CARGO_PKG_VERSION"),
            std::backtrace::Backtrace::force_capture()
        );
        let name = match (checking, carried_on) {
            (true, _) => "graphics-check",
            (false, true) => "model-failure",
            (false, false) => "crash",
        };
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

/// What `moonglow` is started with: `[--no-plugins] [--user-dir DIR]
/// [MODULE]`.
#[derive(Debug, Default, PartialEq)]
struct CommandLine {
    /// The module to open at start.
    module: Option<PathBuf>,
    /// The NWN user folder for this run (`--user-dir DIR` or
    /// `--user-dir=DIR`), as `mg --user-dir` takes it.
    user_dir: Option<PathBuf>,
    no_plugins: bool,
}

impl CommandLine {
    /// The arguments after the program's name. Others starting with `--`
    /// are passed over.
    fn parse(args: impl IntoIterator<Item = std::ffi::OsString>) -> CommandLine {
        let mut line = CommandLine::default();
        let mut args = args.into_iter();
        while let Some(a) = args.next() {
            let text = a.to_string_lossy();
            if text == "--no-plugins" {
                line.no_plugins = true;
            } else if text == "--user-dir" {
                line.user_dir = args.next().map(PathBuf::from);
            } else if let Some(dir) = text.strip_prefix("--user-dir=") {
                line.user_dir = Some(PathBuf::from(dir));
            } else if !text.starts_with("--") && line.module.is_none() {
                line.module = Some(PathBuf::from(a));
            }
        }
        line
    }
}

/// What the window's adapter was taken by: why, and of which adapters
/// (the debug log's `graphics:` line), the backend chosen in Options that
/// had none, and what the startup check found.
#[derive(Debug, Default)]
struct Taken {
    why: String,
    missing: Option<wgpu::Backend>,
    /// The adapters that failed the startup check, what they drew wrong
    /// (to be said), and whether that is the backend's doing (to be
    /// recorded: `Probe::blames_backend`).
    failed: Vec<(wgpu::AdapterInfo, String, bool)>,
    /// The backends Automatic passed over as recorded.
    passed_over: Vec<wgpu::Backend>,
    /// The adapter taken is OpenGL's or a software one.
    fallback: Option<mg_ui::graphics::Fallback>,
    /// What to say of `WGPU_BACKEND`'s names that are no backend.
    environment_unknown: Option<String>,
    /// The adapter taken failed it (kept: asked for, or no other drew).
    failing: bool,
    /// The pick failed and the same GPU's other backend was taken.
    switched: bool,
    asked_for: bool,
    from_environment: bool,
    /// The same GPU's other backend, not recorded as failing.
    other: Option<wgpu::Backend>,
}

/// The window's adapter, of those wgpu found (`adapters`, of the backends
/// `WGPU_BACKEND` names if set, else of all): one that can draw to the
/// window, chosen by `mg_render::backend::start` for Options › General ›
/// Graphics backend (`wanted`), passing over the backends recorded as
/// failing here (`failed`), and checked by the startup check
/// (`mg_render::probe`: where it draws nothing on Automatic, the same
/// GPU's other backend is taken); kept in `taken`. (Given this, egui-wgpu
/// no longer sorts the adapters by power preference or checks them
/// against the window: the pick does both.)
fn pick_adapter(
    adapters: &[wgpu::Adapter],
    surface: Option<&wgpu::Surface<'_>>,
    wanted: Option<wgpu::Backend>,
    failed: &[mg_ui::graphics::Failed],
    taken: &std::sync::Mutex<Taken>,
) -> Result<wgpu::Adapter, String> {
    use mg_ui::graphics;
    let usable: Vec<&wgpu::Adapter> =
        adapters.iter().filter(|a| surface.is_none_or(|s| a.is_surface_supported(s))).collect();
    let infos: Vec<wgpu::AdapterInfo> = usable.iter().map(|a| a.get_info()).collect();
    let power = wgpu::PowerPreference::from_env().unwrap_or(wgpu::PowerPreference::HighPerformance);
    // (Set but naming no backend: as not set, and said.)
    let (named, unknown) = graphics::backends_from_env();
    let from_environment = named.is_some();
    let platform = graphics::Platform::current();
    let mut probes = Vec::new();
    let check = |i: usize| {
        // (A marker while the check runs: a backend that ends the process
        // in it is known at the next start, `died_checking`.)
        let marker = check_marker();
        if let Some(marker) = &marker {
            let record = mg_ui::settings::GraphicsFailure::of(&infos[i], DIED_CHECKING);
            let _ = marker.parent().map(std::fs::create_dir_all);
            let _ = std::fs::write(marker, record.to_line());
        }
        let p = mg_ui::renderer_check::probe(usable[i]);
        if let Some(marker) = &marker {
            let _ = std::fs::remove_file(marker);
        }
        let outcome = p.outcome.clone();
        probes.push(p);
        outcome
    };
    let started = graphics::start(&infos, wanted, from_environment, power, platform, failed, check)
        .ok_or_else(|| {
            format!(
                "no graphics adapter can draw to the window (of {})",
                graphics::describe_adapters(
                    &adapters.iter().map(|a| a.get_info()).collect::<Vec<_>>()
                )
            )
        })?;
    let mut why = started.to_string();
    if from_environment {
        why += &format!(" (WGPU_BACKEND={})", std::env::var("WGPU_BACKEND").unwrap_or_default());
    }
    for p in &probes {
        why += &format!("; checked {p}");
    }
    why += &format!("; of: {}", graphics::describe_adapters(&infos));
    if platform == graphics::Platform::Windows {
        why += &format!("; {}", mg_ui::dx12::compiler());
    }
    let mut known = failed.to_vec();
    known.extend(started.failures().map(|(i, _)| graphics::Failed::of(&infos[i])));
    // (The checks and the probes are in one order: each check ran one.)
    let blamed = |i: usize| {
        let at = started.checks.iter().position(|(checked, _)| *checked == i);
        at.and_then(|k| probes.get(k)).is_some_and(|p| p.blames_backend)
    };
    if let Ok(mut t) = taken.lock() {
        *t = Taken {
            why,
            missing: started.pick.missing,
            failed: started
                .failures()
                .map(|(i, w)| (infos[i].clone(), w.to_string(), blamed(i)))
                .collect(),
            passed_over: started.pick.passed_over.clone(),
            fallback: graphics::Fallback::of(&infos[started.index], platform)
                .filter(|_| !from_environment),
            environment_unknown: graphics::unknown_backends_said(&unknown, from_environment),
            failing: started.failing(),
            switched: started.switched,
            asked_for: started.pick.asked_for(),
            from_environment,
            other: graphics::other_backend(&infos, started.index, platform, &known)
                .map(|i| infos[i].backend),
        };
    }
    Ok(usable[started.index].clone())
}

/// What a record says found it where Moonglow ended during the startup
/// check of a backend.
const DIED_CHECKING: &str = "the startup check: Moonglow ended during it";

/// The startup check's marker: there while an adapter is checked, with
/// the record of that adapter failing ([`died_checking`]).
fn check_marker() -> Option<PathBuf> {
    mg_ui::recovery::data_dir().map(|d| d.join("graphics-check.txt"))
}

/// The backend the last start ended in the check of (the process ended by
/// the driver, or killed while it hung): its record, the marker removed.
/// Nothing was written of it otherwise, and Automatic took the same
/// backend at every start.
fn died_checking(marker: Option<&std::path::Path>) -> Option<mg_ui::settings::GraphicsFailure> {
    let marker = marker?;
    let line = std::fs::read_to_string(marker).ok()?;
    let _ = std::fs::remove_file(marker);
    mg_ui::settings::GraphicsFailure::from_line(line.trim())
}

/// What the startup check found, into the app: the failures recorded
/// (those that are the backend's doing: Automatic passes over them from
/// the next start), said in the log, and the notice where the adapter
/// taken draws nothing or is a fallback.
fn startup_check(moonglow: &mut Moonglow, taken: &Taken) {
    let check = &mut moonglow.renderer_check;
    check.asked_for = taken.asked_for;
    check.from_environment = taken.from_environment;
    check.other = taken.other;
    check.passes_over = taken.passed_over.clone();
    let name = |b| mg_ui::graphics::backend_name(b);
    if let Some(said) = &taken.environment_unknown {
        moonglow.log.warn(said.clone());
    }
    for (info, why, blamed) in &taken.failed {
        // (No device, a panic, a GPU that didn't finish, a device lost:
        // not this backend's doing for good. It is checked again.)
        if !blamed {
            continue;
        }
        let what = format!("the startup check: {why}");
        moonglow
            .settings
            .record_graphics_failure(mg_ui::settings::GraphicsFailure::of(info, &what));
        if taken.switched && !moonglow.renderer_check.passes_over.contains(&info.backend) {
            moonglow.renderer_check.passes_over.push(info.backend);
        }
    }
    let drawing = moonglow.renderer_check.adapter.clone();
    if taken.switched
        && let (Some((first, why, blamed)), Some(now)) = (taken.failed.first(), &drawing)
    {
        let from_now = if *blamed {
            format!(
                ", and passes over {} on this driver from now on (Options › General › Graphics \
                 backend)",
                name(first.backend)
            )
        } else {
            " in this run".to_string()
        };
        moonglow.log.warn(format!(
            "Graphics: {} on {} failed Moonglow's startup check ({why}); Moonglow draws through \
             {} instead{from_now}",
            name(first.backend),
            first.name,
            name(now.backend),
        ));
    }
    if taken.failing
        && let Some(now) = &drawing
    {
        let mine = taken.failed.iter().find(|(i, ..)| i.backend == now.backend);
        let others: Vec<(wgpu::Backend, String)> = taken
            .failed
            .iter()
            .filter(|(i, ..)| i.backend != now.backend)
            .map(|(i, w, _)| (i.backend, w.clone()))
            .collect();
        let why = mine.map_or("", |(_, w, _)| w.as_str());
        moonglow.renderer_check.startup_failed(why, &others);
        let said = moonglow.renderer_check.trouble.as_ref().map(|t| t.said.clone());
        moonglow.log.warn(format!("Graphics: {}", said.unwrap_or_default()));
    }
    if let Some(fallback) = taken.fallback {
        mg_ui::renderer_check::fallback_taken(moonglow, fallback);
    }
}

/// Starts Moonglow again (Restart with …, in the renderer's notice), now
/// that this run has ended and its settings are saved: the module that
/// was open, this run's `--user-dir` and `--no-plugins`, and the backend
/// (as `WGPU_BACKEND` where that named this run's; else the settings
/// saved say it).
fn relaunch(again: &mg_ui::renderer_check::Relaunch, line: &CommandLine) {
    use mg_update::Restart;
    let mut args: Vec<std::ffi::OsString> = Vec::new();
    if line.no_plugins {
        args.push("--no-plugins".into());
    }
    if let Some(dir) = &line.user_dir {
        args.push("--user-dir".into());
        args.push(dir.into());
    }
    if let Some(module) = &again.module {
        args.push(module.into());
    }
    let mut command = match &again.start {
        // (The installer starts it when it is done.)
        Some(Restart::ByInstaller) => return,
        Some(Restart::Open(app)) => {
            let mut c = std::process::Command::new("open");
            c.arg("-n").arg(app);
            if !args.is_empty() {
                c.arg("--args");
            }
            c
        }
        Some(Restart::Exe(exe)) => std::process::Command::new(exe),
        // This program: the AppImage itself where it is one (the program
        // inside its mount goes when this run ends), which `$APPIMAGE`
        // alone does not say.
        None => match mg_update::Facts::here().image() {
            Some(image) => std::process::Command::new(image),
            None => {
                let Ok(exe) = std::env::current_exe() else { return };
                std::process::Command::new(exe)
            }
        },
    };
    command.args(&args);
    if again.environment {
        let name = match again.backend {
            wgpu::Backend::Dx12 => "dx12",
            wgpu::Backend::Metal => "metal",
            wgpu::Backend::Gl => "gl",
            _ => "vulkan",
        };
        command.env("WGPU_BACKEND", name);
    }
    if let Err(e) = command.spawn() {
        eprintln!("Moonglow could not start again: {e}");
    }
}

/// The update ready as Moonglow closed, installed (its outcome kept for
/// the next start's log); whether it was.
fn install_at_close(staged: &mg_update::Staged, cache: &std::path::Path, restart: bool) -> bool {
    let what = staged.kind.what();
    match mg_update::install::install(staged, restart, cache) {
        // (The AppImage replaced already: its run's log said so.)
        Ok(_) if staged.installed() => true,
        Ok(_) => {
            let said = match staged.kind {
                mg_update::Kind::WindowsInstaller { .. } => format!(
                    "Moonglow {}'s installer was started as Moonglow closed (its log: {}).",
                    staged.version,
                    cache.join("install.log").display()
                ),
                _ => format!("Moonglow {} was installed in place of {what}.", staged.version),
            };
            mg_update::install::write_report(cache, &said);
            true
        }
        Err(why) => {
            let said = format!("Moonglow {} isn't installed: {why}.", staged.version);
            eprintln!("{said}");
            mg_update::install::write_report(cache, &said);
            false
        }
    }
}

fn main() -> eframe::Result<()> {
    // (What the libraries write of the release reads it from there.)
    mg_core::release::set(env!("CARGO_PKG_VERSION"));
    crash_reports();
    let line = CommandLine::parse(std::env::args_os().skip(1));
    mg_ui::trace::by_default();
    // The settings are eframe's, given to the app once the window has its
    // device (`run_native`'s closure, below): the graphics backend chosen
    // is read from the same file before, so that it is one setting among
    // the others in Options (its own file would keep it apart from them).
    // Without the file, or where it can't be read: Automatic. They are
    // read here once, for the app too: where they do not all read, what
    // does is kept, the file as it was is put aside before eframe saves
    // over it, and the log says so (`Settings::read_eframe`).
    let settings_file = eframe::storage_dir(APP_NAME).map(|d| d.join("app.ron"));
    let read = settings_file
        .as_deref()
        .map(|f| Settings::read_eframe(f, SETTINGS_KEY))
        .unwrap_or_default();
    let saved = &read.settings;
    // (As they are in the file: what `App::save` tells another window's
    // save from.)
    let as_read = (saved.clone(), saved.saved_text());
    let app_file = settings_file.clone();
    let wanted = saved.graphics_backend.backend();
    // (The backends recorded as drawing nothing here: Automatic passes
    // over them. And the one the last start ended in the check of.)
    let mut failed = saved.failed_backends();
    let died = died_checking(check_marker().as_deref());
    failed.extend(died.as_ref().and_then(|d| d.failed()));
    let taken = std::sync::Arc::new(std::sync::Mutex::new(Taken::default()));
    let picked = taken.clone();
    // Restart with … (the renderer's notice), Restart Now (an update's)
    // and an update to install: once the window has closed.
    let again = std::sync::Arc::new(std::sync::Mutex::new(Closing::default()));
    let asked_again = again.clone();
    let line_again =
        CommandLine { module: None, user_dir: line.user_dir.clone(), no_plugins: line.no_plugins };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Moonglow Toolset")
            .with_app_id(APP_ID)
            .with_icon(window_icon()),
        wgpu_options: egui_wgpu::WgpuConfiguration {
            // (Direct3D 12 compiles shaders with the DXC beside the
            // program, else FXC: `mg_render::dx12`.)
            wgpu_setup: mg_ui::graphics_setup(egui_wgpu::WgpuSetup::CreateNew(
                egui_wgpu::WgpuSetupCreateNew {
                    // (BC textures where the GPU reads them, the GPU's
                    // memory in small blocks: `device_descriptor`.)
                    device_descriptor: std::sync::Arc::new(|adapter| {
                        device_descriptor(adapter.features(), adapter.limits())
                    }),
                    // The instance keeps every backend (or `WGPU_BACKEND`'s),
                    // so that a backend chosen with no adapter can fall back.
                    native_adapter_selector: Some(std::sync::Arc::new(move |adapters, surface| {
                        pick_adapter(adapters, surface, wanted, &failed, &picked)
                    })),
                    ..egui_wgpu::WgpuSetupCreateNew::without_display_handle()
                },
            )),
            ..Default::default()
        },
        // The settings stay where they were before the window had an app
        // ID (eframe would otherwise name their folder after it).
        persistence_path: settings_file,
        window_builder: Some(Box::new(|window| maximize_later(window, cfg!(windows)))),
        ..Default::default()
    };
    let result = eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            // (As read above, from the file eframe's storage is.)
            let unread = read.problem();
            let mut settings: Settings = read.settings;
            // (A backend another release of Moonglow recorded as failing
            // is tried again: what failed may be mended.)
            let forgotten = settings.forget_other_releases_graphics_failures();
            if let Some(died) = &died {
                settings.record_graphics_failure(died.clone());
            }
            // `--user-dir DIR`: this run's user folder, over Options' (not
            // saved).
            let not_a_folder = line.user_dir.clone().filter(|d| !d.is_dir());
            settings.user_dir_this_run = line.user_dir.clone().filter(|d| d.is_dir());
            let mut moonglow = Moonglow::with_settings(settings, Box::new(NativeDialogs));
            if let Some(unread) = unread {
                moonglow.log.error(unread);
            }
            if let Some(dir) = not_a_folder {
                moonglow.log.error(format!(
                    "--user-dir {}: not a folder; the user folder of Options › Folders is used",
                    dir.display()
                ));
            } else if let Some(dir) = &line.user_dir {
                moonglow.log.info(format!("User folder for this run: {}", dir.display()));
            }
            if let Some(rs) = &cc.wgpu_render_state {
                // (What the GPU refuses is said in the log and left out,
                // where wgpu would end the program.)
                mg_ui::model_guard::note_gpu_errors(&rs.device);
                moonglow.set_render_state(rs.clone());
                let taken = std::mem::take(&mut *taken.lock().unwrap_or_else(|e| e.into_inner()));
                if let Some(missing) = taken.missing {
                    moonglow.log.warn(format!(
                        "Options › General › Graphics backend: no {} adapter here; Moonglow \
                         draws with {} (Automatic)",
                        mg_ui::graphics::backend_name(missing),
                        mg_ui::graphics::backend_name(rs.adapter.get_info().backend),
                    ));
                }
                if forgotten > 0 {
                    moonglow.log.info(
                        "Graphics: the backends an earlier version of Moonglow found drawing \
                         wrong here are tried again",
                    );
                }
                if let Some(died) = &died {
                    moonglow.log.warn(format!(
                        "Graphics: Moonglow ended during its startup check of {} on {} the last \
                         time it started; Automatic passes over it on this driver from now on \
                         (Options › General › Graphics backend)",
                        died.backend, died.adapter
                    ));
                }
                startup_check(&mut moonglow, &taken);
                moonglow.graphics_choice = Some(taken.why);
            }
            // Recovery copies of unsaved work go to Moonglow's data folder;
            // those a crashed session left are offered back.
            moonglow.recovery_dir = mg_ui::recovery::data_dir().map(|d| d.join("recovery"));
            moonglow.compiled_dir = mg_ui::recovery::data_dir().map(|d| d.join("compiled"));
            // A nasher project's locked objects (kept out of its files).
            moonglow.locks_dir = mg_ui::recovery::data_dir().map(|d| d.join("locks"));
            // Each module's open tabs, to open again with it.
            moonglow.open_tabs_dir = mg_ui::recovery::data_dir().map(|d| d.join("open-tabs"));
            // The game's shaders, compiled, in the user's cache folder.
            moonglow.shader_cache_dir = mg_ui::recovery::shader_cache_dir();
            // And each module's pipelines of them, made again as it opens.
            moonglow.pipelines_dir = mg_ui::recovery::cache_dir().map(|d| d.join("pipelines"));
            // Long work runs while the window keeps drawing.
            moonglow.background_jobs = true;
            // Help › Check for Updates… and the daily check ask GitHub
            // (not where a package manager updates Moonglow: Flatpak,
            // Scoop, Nix; Options › Updates), and an update is installed
            // from here where Moonglow updates itself and the build has
            // the key its checksums are signed with (`build.rs`).
            moonglow.updates.http = Some(std::sync::Arc::new(mg_update::http::Client::new()));
            moonglow.updates.kind = mg_update::Kind::detect(&mg_update::Facts::here());
            moonglow.updates.key = update_key();
            moonglow.updates.cache = mg_ui::recovery::cache_dir().map(|d| d.join("updates"));
            mg_ui::updates::startup(&mut moonglow);
            // The plugins installed in the data folder (`--no-plugins`
            // starts without them).
            moonglow.plugin_dir = mg_ui::recovery::data_dir().map(|d| d.join("plugins"));
            moonglow.plugin_answers = mg_ui::plugins::answers_path();
            moonglow.plugin_values = mg_ui::plugins::values_path();
            moonglow.no_plugins = line.no_plugins;
            moonglow.load_plugins();
            moonglow.prefab_dir = mg_ui::prefabs::dir();
            moonglow.var_set_dir = mg_ui::var_sets::dir();
            moonglow.param_set_dir = mg_ui::param_sets::dir();
            moonglow.area_set_dir = mg_ui::area_sets::dir();
            moonglow.lighting_dir = mg_ui::lighting::dir();
            moonglow.find_recoveries();
            // Sounds play when there is an output device.
            if let Some(s) = speakers::Speakers::open() {
                moonglow.speaker = Box::new(s);
            }
            // `moonglow path/to/module.mod` opens a module at start (or
            // asks first, a folder of its name being beside it).
            if let Some(path) = &line.module {
                moonglow.actions.push(mg_ui::Action::OpenModule(path.clone()));
            }
            Ok(Box::new(App {
                moonglow,
                title: String::new(),
                again: asked_again,
                settings_file: app_file,
                saved: as_read,
            }))
        }),
    );
    // (After the window has closed and the settings are saved: the new
    // run reads what this one recorded.)
    let closing = again.lock().map(|mut a| std::mem::take(&mut *a)).unwrap_or_default();
    let mut relaunched = closing.relaunch;
    if let Some((staged, cache)) = &closing.update
        && !install_at_close(staged, cache, relaunched.is_some())
        && let Some(r) = &mut relaunched
    {
        // (Not installed: this copy starts again, as it was.)
        r.start = None;
    }
    if let Some(again) = relaunched {
        relaunch(&again, &line_again);
    }
    result
}

/// The key the releases' checksums are signed with, built in from
/// `packaging/minisign.pub` (`build.rs`); none until the owner commits it.
fn update_key() -> Option<mg_update::PublicKey> {
    let key = option_env!("MOONGLOW_UPDATE_KEY")?;
    match mg_update::PublicKey::parse(key) {
        Ok(k) => Some(k),
        Err(e) => {
            eprintln!("Moonglow's update key: {e}");
            None
        }
    }
}

/// The window's device, of an adapter with `features` and `limits`: BC
/// textures where it reads them (the game's are mostly BC: uploaded as
/// they are, else as RGBA8 four to eight times their size) and the shadow
/// volumes' depth as exact as the scene's; the GPU's memory taken in
/// blocks of 8 to 64 MB, not wgpu's default 128 to 256 (a block is held
/// while anything is in it, and what the views and textures let go of
/// stayed held: 768 MB held for 270 used after Tyrants of the Moonsea's
/// frame test, 524 with these, no frame slower).
fn device_descriptor(
    features: wgpu::Features,
    limits: wgpu::Limits,
) -> wgpu::DeviceDescriptor<'static> {
    // (One with the startup check's: `mg_render::backend`.)
    mg_ui::graphics::device_descriptor("moonglow", features, limits)
}

#[cfg(test)]
mod tests {
    /// The window's device asks for BC textures where the adapter has
    /// them (without, the frame test's took four times the memory), and
    /// for the GPU's memory in small blocks.
    #[test]
    fn the_device_asks_for_bc_textures_and_small_blocks() {
        let all = wgpu::Features::TEXTURE_COMPRESSION_BC | wgpu::Features::DEPTH32FLOAT_STENCIL8;
        let d = super::device_descriptor(all, wgpu::Limits::default());
        assert!(d.required_features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC));
        assert!(matches!(d.memory_hints, wgpu::MemoryHints::MemoryUsage));
        let none = super::device_descriptor(wgpu::Features::empty(), wgpu::Limits::default());
        assert!(none.required_features.is_empty(), "only what the adapter has");
    }
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

    /// `--user-dir` takes the next argument (or the one after `=`), which
    /// is not the module opened.
    #[test]
    fn the_command_line_names_the_user_folder_and_the_module() {
        use super::CommandLine;
        use std::path::PathBuf;
        let parse = |args: &[&str]| CommandLine::parse(args.iter().map(Into::into));
        assert_eq!(parse(&[]), CommandLine::default());
        assert_eq!(
            parse(&["--user-dir", "/nwn/test user", "my module.mod", "--no-plugins"]),
            CommandLine {
                module: Some(PathBuf::from("my module.mod")),
                user_dir: Some(PathBuf::from("/nwn/test user")),
                no_plugins: true,
            }
        );
        assert_eq!(parse(&["x.mod", "--user-dir=/u"]).user_dir, Some(PathBuf::from("/u")));
        assert_eq!(parse(&["x.mod", "--user-dir=/u"]).module, Some(PathBuf::from("x.mod")));
        // (Another `--` option is passed over; the first module is opened.)
        assert_eq!(parse(&["--other", "a.mod", "b.mod"]).module, Some(PathBuf::from("a.mod")));
        assert_eq!(parse(&["--user-dir"]).user_dir, None);
    }

    #[test]
    fn the_window_icon_decodes() {
        let icon = super::window_icon();
        assert_eq!((icon.width, icon.height, icon.rgba.len()), (256, 256, 256 * 256 * 4));
    }

    /// What the startup check found, into the app: Heed's Direct3D 12
    /// drew nothing and Vulkan was taken (recorded and said in the log, no
    /// notice); a backend chosen in Options that drew nothing is kept (a
    /// notice, offering the other backend).
    #[test]
    fn the_startup_checks_verdict_is_recorded_and_told() {
        use super::{Taken, startup_check};
        let info = |backend| {
            let mut a = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, backend);
            (a.vendor, a.device, a.name) = (0x1002, 0x67df, "Radeon RX 580 Series".into());
            a
        };
        let app = |drawing| {
            let mut m = mg_ui::Moonglow::new(None, Box::new(mg_ui::NoDialogs::default()));
            m.renderer_check.adapter = Some(info(drawing));
            m
        };
        let nothing = "the solid mesh is [0, 0, 0] at (8, 18)".to_string();
        let mut heed = app(wgpu::Backend::Vulkan);
        let switched = Taken {
            failed: vec![(info(wgpu::Backend::Dx12), nothing.clone(), true)],
            switched: true,
            ..Default::default()
        };
        startup_check(&mut heed, &switched);
        assert_eq!(heed.settings.graphics_failures.len(), 1);
        assert_eq!(heed.settings.graphics_failures[0].backend, "Direct3D 12");
        assert!(heed.settings.graphics_failures[0].what.starts_with("the startup check: "));
        let said = &heed.log.entries.last().unwrap().1;
        assert!(said.contains("Moonglow draws through Vulkan instead"), "{said}");
        assert_eq!(heed.renderer_check.trouble, None);
        let mut chosen = app(wgpu::Backend::Dx12);
        let kept = Taken {
            failed: vec![(info(wgpu::Backend::Dx12), nothing, true)],
            failing: true,
            asked_for: true,
            other: Some(wgpu::Backend::Vulkan),
            ..Default::default()
        };
        startup_check(&mut chosen, &kept);
        let trouble = chosen.renderer_check.trouble.clone().expect("the notice");
        assert_eq!(trouble.offer, Some(wgpu::Backend::Vulkan));
        assert!(trouble.said.contains("Direct3D 12 on Radeon RX 580 Series"), "{}", trouble.said);
        assert_eq!(heed.renderer_check.passes_over, [wgpu::Backend::Dx12]);
    }

    /// A check that failed without a picture compared (no device: the
    /// GPU's memory taken by another program; a panic; a GPU that didn't
    /// finish) is not the backend's doing for good: the other backend is
    /// drawn with in this run, and nothing is recorded, so Automatic
    /// checks the backend again at the next start.
    #[test]
    fn a_check_that_compared_no_picture_records_nothing() {
        use super::{Taken, startup_check};
        let info = |backend| {
            let mut a = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, backend);
            (a.vendor, a.device, a.name) = (0x1002, 0x67df, "Radeon RX 580 Series".into());
            a
        };
        for why in ["no device: out of memory", "opening the device panicked"] {
            let mut m = mg_ui::Moonglow::new(None, Box::new(mg_ui::NoDialogs::default()));
            m.renderer_check.adapter = Some(info(wgpu::Backend::Vulkan));
            let switched = Taken {
                failed: vec![(info(wgpu::Backend::Dx12), why.to_string(), false)],
                switched: true,
                ..Default::default()
            };
            startup_check(&mut m, &switched);
            assert!(m.settings.graphics_failures.is_empty(), "{why}");
            assert!(m.renderer_check.passes_over.is_empty());
            let said = &m.log.entries.last().unwrap().1;
            assert!(said.ends_with("Moonglow draws through Vulkan instead in this run"), "{said}");
        }
    }

    /// OpenGL or a software adapter taken (nothing else was there) is
    /// said, in the log and a notice, with what to install; and a name in
    /// `WGPU_BACKEND` that is no backend.
    #[test]
    fn a_fallback_adapter_is_told() {
        use super::{Taken, startup_check};
        let mut m = mg_ui::Moonglow::new(None, Box::new(mg_ui::NoDialogs::default()));
        let mut gl = wgpu::AdapterInfo::new(wgpu::DeviceType::IntegratedGpu, wgpu::Backend::Gl);
        gl.name = "Mesa Intel(R) UHD".into();
        m.renderer_check.adapter = Some(gl);
        let taken = Taken {
            fallback: Some(mg_ui::graphics::Fallback::Backend(wgpu::Backend::Gl)),
            environment_unknown: mg_ui::graphics::unknown_backends_said(&["d3d".into()], false),
            ..Default::default()
        };
        startup_check(&mut m, &taken);
        let trouble = m.renderer_check.trouble.clone().expect("the notice");
        assert!(trouble.said.starts_with("Moonglow draws through OpenGL on Mesa Intel(R) UHD, "));
        assert!(!trouble.record && trouble.offer.is_none());
        let log: Vec<&str> = m.log.entries.iter().map(|e| e.1.as_str()).collect();
        assert!(log.iter().any(|l| l.starts_with("WGPU_BACKEND names no backend")), "{log:?}");
        assert!(log.iter().any(|l| l.starts_with("Graphics: Moonglow draws through OpenGL")));
    }

    /// The marker the startup check leaves while it runs is the record of
    /// the backend checked: found at the next start where the last one
    /// ended in the check, and removed; a marker that is no record is
    /// removed and gives none.
    #[test]
    fn a_start_that_ended_in_the_check_is_known_at_the_next() {
        use super::{DIED_CHECKING, died_checking};
        let dir = std::env::temp_dir().join(format!("moonglow-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let marker = dir.join("graphics-check.txt");
        assert_eq!(died_checking(Some(&marker)), None, "no marker");
        assert_eq!(died_checking(None), None);
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, wgpu::Backend::Dx12);
        (info.vendor, info.device, info.name) = (0x1002, 0x67df, "Radeon RX 580 Series".into());
        let record = mg_ui::settings::GraphicsFailure::of(&info, DIED_CHECKING);
        std::fs::write(&marker, record.to_line()).unwrap();
        let died = died_checking(Some(&marker)).expect("the record");
        assert_eq!(died.failed(), Some(mg_ui::graphics::Failed::of(&info)));
        assert!(!marker.exists(), "removed once read");
        std::fs::write(&marker, "not a record").unwrap();
        assert_eq!(died_checking(Some(&marker)), None);
        assert!(!marker.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
