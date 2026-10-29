//! Build › Publish to NWSync…: the module's haks and talk table (and, for a
//! single-player module, the module itself) written into an NWSync
//! repository folder for a web server to serve (`mg_module::nwsync`), and
//! the repository's unused data taken away (`nwn_nwsync_prune`'s
//! housekeeping). Both run as jobs (`jobs.rs`): a hak of gigabytes takes a
//! while, the job's window says how far it is and has Cancel, and a panic
//! in the work is said in the log, not a crash.

use std::path::PathBuf;

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
    /// The last one written.
    pub written: Option<mg_module::nwsync::Written>,
    /// What the last Remove Unused Data found and did.
    pub pruned: Option<mg_module::nwsync::Pruned>,
    pub error: Option<String>,
}

/// The job publishing (its window's heading; not the window's own title).
pub const PUBLISHING: &str = "Publishing to NWSync";
/// The job removing unused data.
pub const PRUNING: &str = "Removing unused NWSync data";

/// Opens the window, with the folder last published to.
pub(crate) fn open(app: &mut Moonglow) {
    let info = app.ws.as_mut().and_then(|ws| ws.flush().ok().map(|()| &ws.module));
    let info = info.and_then(|m| mg_module::query::info_in(m, crate::text::game_codepage()).ok());
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

/// `n` of `what`: "1 manifest", "3 manifests".
fn count(n: usize, what: &str) -> String {
    if n == 1 { format!("1 {what}") } else { format!("{n} {what}s") }
}

/// The repository folder chosen, or why there is none (said in the
/// window).
fn folder(p: &mut Publish) -> Option<PathBuf> {
    p.error = None;
    if p.folder.trim().is_empty() {
        p.error = Some("Choose the repository folder.".into());
        return None;
    }
    Some(PathBuf::from(p.folder.trim()))
}

/// Starts publishing, as a job.
fn start(app: &mut Moonglow) {
    let (Some(p), Some(ws), Some(game), Some(install)) =
        (&mut app.publish, &mut app.ws, &app.game, &app.install)
    else {
        return;
    };
    p.written = None;
    p.pruned = None;
    let Some(folder) = folder(p) else { return };
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
    // (Which resources, from where: the haks' indexes, read here.)
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
    app.settings.nwsync_repository = Some(folder.clone());
    app.start_job(
        PUBLISHING,
        move |job| {
            let progress = &job.progress;
            mg_module::nwsync::write_until(
                &folder,
                &contents,
                &o,
                &mut |d, t| progress.step(d, t),
                &|| progress.cancelled(),
            )
        },
        |app, r| {
            let Some(p) = &mut app.publish else { return };
            match r {
                Ok(w) => {
                    app.log.info(format!(
                        "Published to NWSync: manifest {} ({} files)",
                        w.sha1, w.files
                    ));
                    p.written = Some(w);
                }
                Err(e) => p.error = Some(e),
            }
        },
    );
}

/// Starts taking away the repository's unused data, as a job: the data
/// files no manifest names, written more than two weeks ago (as
/// `nwn_nwsync_prune`), the folders they leave and the metadata of
/// manifests taken away.
fn start_pruning(app: &mut Moonglow) {
    let Some(p) = &mut app.publish else { return };
    p.written = None;
    p.pruned = None;
    let Some(folder) = folder(p) else { return };
    app.settings.nwsync_repository = Some(folder.clone());
    app.start_job_anywhere(
        PRUNING,
        move |_| mg_module::nwsync::prune(&folder, mg_module::nwsync::PRUNE_MIN_AGE, false),
        |app, r| {
            let Some(p) = &mut app.publish else { return };
            match r {
                Ok(pruned) => {
                    app.log.info(format!(
                        "NWSync repository: removed {} ({:.1} MB)",
                        count(pruned.removed.len(), "unused data file"),
                        pruned.removed_bytes as f64 / 1048576.0
                    ));
                    for (sha, m) in &pruned.missing {
                        app.log.warn(format!(
                            "NWSync repository: the data file {sha}, which manifest {m} names, \
                             is missing"
                        ));
                    }
                    for problem in &pruned.problems {
                        app.log.warn(format!("NWSync repository: {problem}"));
                    }
                    p.pruned = Some(pruned);
                }
                Err(e) => p.error = Some(e),
            }
        },
    );
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let busy = app.busy();
    let Some(p) = &mut app.publish else { return };
    let mut open = true;
    let mut go = false;
    let mut prune = false;
    let mut browse = false;
    crate::widgets::dialog(ctx, "Publish to NWSync", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(
                "Players' games download the module's haks and talk table from a web server \
                 before they join (nwserver's -nwsyncurl). This writes what the web server serves.",
            );
            ui.add_enabled_ui(!busy, |ui| {
                ui.horizontal(|ui| {
                    crate::widgets::field_label(ui, "Repository folder");
                    ui.add(egui::TextEdit::singleline(&mut p.folder).desired_width(320.0));
                    browse = ui.button("Browse…").clicked();
                });
                ui.checkbox(&mut p.with_module, "With the module itself").on_hover_text(
                    "For a single-player module players download whole; not for a persistent world",
                );
                if p.with_module {
                    ui.horizontal(|ui| {
                        crate::widgets::field_label(ui, "Name");
                        ui.text_edit_singleline(&mut p.name);
                    });
                    crate::widgets::field_label(ui, "Description");
                    let field = egui::TextEdit::multiline(&mut p.description)
                        .desired_rows(3)
                        .desired_width(420.0);
                    crate::widgets::text_area(ui, ui.id().with("nwsync-description"), field);
                }
                ui.horizontal(|ui| {
                    crate::widgets::field_label(ui, "Group ID");
                    ui.add(egui::DragValue::new(&mut p.group_id));
                    ui.weak("for servers sharing a repository");
                });
                ui.checkbox(
                    &mut p.latest,
                    "Make it the latest (what nwserver serves unless told a hash)",
                );
            });
            if let Some(e) = &p.error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            if let Some(w) = &p.written {
                ui.separator();
                ui.horizontal(|ui| {
                    crate::widgets::field_label(ui, "Manifest");
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
            if let Some(r) = &p.pruned {
                ui.separator();
                ui.label(format!(
                    "{}. Removed {} ({:.1} MB); {} kept as written in the last two weeks.",
                    count(r.manifests, "manifest"),
                    count(r.removed.len(), "unused data file"),
                    r.removed_bytes as f64 / 1048576.0,
                    r.kept_young
                ));
                if !r.missing.is_empty() {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!(
                            "{} the manifests name missing (the log names them): publish their \
                             modules again.",
                            count(r.missing.len(), "data file")
                        ),
                    );
                }
                for problem in &r.problems {
                    ui.colored_label(ui.visuals().warn_fg_color, problem);
                }
            }
            ui.horizontal(|ui| {
                go = ui.add_enabled(!busy, egui::Button::new("Publish")).clicked();
                prune = ui
                    .add_enabled(!busy, egui::Button::new("Remove Unused Data"))
                    .on_hover_text(
                        "Take away the data files no manifest in the folder names, written more \
                         than two weeks ago, as nwn_nwsync_prune does. Delete the manifests you \
                         no longer serve (manifests/HASH and HASH.json) first.",
                    )
                    .clicked();
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
    if prune {
        start_pruning(app);
    }
    if !open && !app.busy() {
        app.publish = None;
    }
}
