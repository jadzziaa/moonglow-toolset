//! Build › Publish to NWSync…: the module's haks and talk table (and, for a
//! single-player module, the module itself) written into an NWSync
//! repository folder for a web server to serve (`mg_module::nwsync`). It
//! runs in the background: a hak of gigabytes takes a while.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::Moonglow;

/// The Publish to NWSync window.
#[derive(Debug, Default)]
pub struct Publish {
    pub folder: String,
    pub with_module: bool,
    pub name: String,
    pub description: String,
    pub group_id: u32,
    pub latest: bool,
    /// While it runs: resources done and of how many.
    progress: Option<(Arc<AtomicUsize>, Arc<AtomicUsize>)>,
    result: Arc<Mutex<Option<Result<mg_module::nwsync::Written, String>>>>,
    /// The last one written.
    pub written: Option<mg_module::nwsync::Written>,
    pub error: Option<String>,
}

impl Publish {
    pub fn running(&self) -> bool {
        self.progress.is_some()
    }
}

/// Opens the window, with the folder last published to.
pub(crate) fn open(app: &mut Moonglow) {
    let info = app.ws.as_mut().and_then(|ws| ws.flush().ok().map(|()| &ws.module));
    let info = info.and_then(|m| mg_module::query::info(m).ok());
    app.publish = Some(Publish {
        folder: app
            .settings
            .nwsync_repository
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        name: info.as_ref().map(|i| i.name.clone()).unwrap_or_default(),
        description: info.map(|i| i.description).unwrap_or_default(),
        latest: true,
        ..Default::default()
    });
}

/// Starts publishing in the background.
fn start(app: &mut Moonglow) {
    let (Some(p), Some(ws), Some(game), Some(install)) =
        (&mut app.publish, &mut app.ws, &app.game, &app.install)
    else {
        return;
    };
    p.error = None;
    p.written = None;
    let folder = PathBuf::from(p.folder.trim());
    if p.folder.trim().is_empty() {
        p.error = Some("Choose the repository folder.".into());
        return;
    }
    if let Err(e) = ws.flush() {
        p.error = Some(e.to_string());
        return;
    }
    let m = &ws.module;
    let uuid = p.with_module.then(|| {
        m.info()
            .ok()
            .and_then(|i| {
                i.root.string("Mod_UUID").map(|u| String::from_utf8_lossy(u).into_owned())
            })
            .filter(|u| !u.is_empty())
            .unwrap_or_else(mg_module::nwsync::new_uuid)
    });
    let contents = match mg_module::nwsync::module_contents(
        m,
        install,
        &game.resman,
        p.with_module,
        uuid.as_deref(),
    ) {
        Ok((c, missing)) if missing.is_empty() => c,
        Ok((_, missing)) => {
            p.error = Some(format!("Haks not found: {}", missing.join(", ")));
            return;
        }
        Err(e) => {
            p.error = Some(e);
            return;
        }
    };
    let o = mg_module::nwsync::Options {
        with_module: p.with_module,
        name: if p.with_module { p.name.clone() } else { String::new() },
        description: if p.with_module { p.description.clone() } else { String::new() },
        uuid,
        group_id: p.group_id,
        latest: p.latest,
        limit: Some(mg_module::nwsync::FILE_LIMIT),
        force: false,
        dry_run: false,
    };
    let (done, total) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(contents.len())));
    p.progress = Some((done.clone(), total.clone()));
    let result = p.result.clone();
    app.settings.nwsync_repository = Some(folder.clone());
    std::thread::spawn(move || {
        let r = mg_module::nwsync::write(&folder, &contents, &o, &mut |d, t| {
            done.store(d, Ordering::Relaxed);
            total.store(t, Ordering::Relaxed);
        });
        *result.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(r);
    });
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(p) = &mut app.publish else { return };
    // Finished in the background?
    if p.progress.is_some() {
        let finished = p.result.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
        match finished {
            Some(Ok(w)) => {
                p.progress = None;
                app.log
                    .info(format!("Published to NWSync: manifest {} ({} files)", w.sha1, w.files));
                p.written = Some(w);
            }
            Some(Err(e)) => {
                p.progress = None;
                p.error = Some(e);
            }
            None => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
        }
    }
    let mut open = true;
    let mut go = false;
    let mut browse = false;
    egui::Window::new("Publish to NWSync")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                "Players' games download the module's haks and talk table from a web server \
                 before they join (nwserver's -nwsyncurl). This writes what the web server serves.",
            );
            ui.add_enabled_ui(!p.running(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Repository folder");
                    ui.add(egui::TextEdit::singleline(&mut p.folder).desired_width(320.0));
                    browse = ui.button("Browse…").clicked();
                });
                ui.checkbox(&mut p.with_module, "With the module itself").on_hover_text(
                    "For a single-player module players download whole; not for a persistent world",
                );
                if p.with_module {
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut p.name);
                    });
                    ui.label("Description");
                    ui.add(
                        egui::TextEdit::multiline(&mut p.description)
                            .desired_rows(3)
                            .desired_width(420.0),
                    );
                }
                ui.horizontal(|ui| {
                    ui.label("Group ID");
                    ui.add(egui::DragValue::new(&mut p.group_id));
                    ui.weak("for servers sharing a repository");
                });
                ui.checkbox(
                    &mut p.latest,
                    "Make it the latest (what nwserver serves unless told a hash)",
                );
            });
            if let Some((done, total)) = &p.progress {
                let (d, t) = (done.load(Ordering::Relaxed), total.load(Ordering::Relaxed).max(1));
                ui.add(
                    egui::ProgressBar::new(d as f32 / t as f32).text(format!("{d} of {t} files")),
                );
            }
            if let Some(e) = &p.error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            if let Some(w) = &p.written {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Manifest");
                    ui.monospace(&w.sha1);
                    if ui.small_button("Copy").clicked() {
                        ui.ctx().copy_text(w.sha1.clone());
                    }
                });
                ui.weak(format!(
                    "{} files, {:.1} MB ({:.1} MB compressed; {} written now). Serve the folder \
                     with a web server and start nwserver with -nwsyncurl and its address \
                     (-nwsynchash {} to pin this manifest).",
                    w.files,
                    w.bytes as f64 / 1048576.0,
                    w.on_disk as f64 / 1048576.0,
                    w.new_files,
                    w.sha1
                ));
            }
            ui.horizontal(|ui| {
                go = ui.add_enabled(!p.running(), egui::Button::new("Publish")).clicked();
            });
        });
    if browse {
        let start = PathBuf::from(p.folder.trim());
        let start = (!p.folder.trim().is_empty()).then_some(start);
        if let Some(dir) = app.dialogs.pick_folder("NWSync Repository", start.as_deref())
            && let Some(p) = &mut app.publish
        {
            p.folder = dir.display().to_string();
        }
    }
    if go {
        start(app);
    }
    if !open && app.publish.as_ref().is_some_and(|p| !p.running()) {
        app.publish = None;
    }
}
