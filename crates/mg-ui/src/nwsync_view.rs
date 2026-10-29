//! Build › Publish to NWSync…: the module's haks and talk table (and, for a
//! single-player module, the module itself) written into an NWSync
//! repository folder for a web server to serve (`mg_module::nwsync`), and
//! the repository's unused data taken away (`nwn_nwsync_prune`'s
//! housekeeping), and the folder sent to the web server by the builder's
//! own command (Upload), once for each part in an order that never leaves
//! the server serving a manifest whose data isn't there. All run as jobs
//! (`jobs.rs`): a hak of gigabytes takes a while, the job's window says
//! how far it is (an upload: its output, as a project's command's) and has
//! Cancel, and a panic in the work is said in the log, not a crash.

use std::path::{Path, PathBuf};

use mg_module::nwsync::{AFTER_PRUNING, AFTER_PUBLISHING, Part};
use mg_module::project_file::ProjectFile;

use crate::Moonglow;
use crate::project_commands::run_steps;

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
/// The job sending the repository to the web server.
pub const UPLOADING: &str = "Uploading to NWSync";

/// The upload command in force, and where it came from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UploadCommand {
    pub text: String,
    pub input: bool,
    /// The project's root, where its files name it (`[nwsync] upload`),
    /// and whether the builder's own file does; none: Options'.
    pub project: Option<(PathBuf, bool)>,
}

impl UploadCommand {
    /// Where it was set, as the window says it.
    fn from(&self) -> &'static str {
        match self.project {
            Some((_, true)) => "the project's .moonglow/user.toml",
            Some((_, false)) => "the project's moonglow.toml",
            None => "Options › Folders",
        }
    }
}

/// The upload command in force: the project's (`[nwsync] upload`, the
/// builder's own file's over the team's), else the one set in Options.
pub(crate) fn upload_command(app: &Moonglow) -> Option<UploadCommand> {
    let project = app.ws.as_ref().and_then(|ws| {
        let root = ws.module.location.as_ref().and_then(ProjectFile::root_of)?;
        let c = ws.module.project_file.nwsync_upload.as_ref()?;
        let text = c.run.clone()?;
        Some(UploadCommand { text, input: c.input, project: Some((root.to_path_buf(), c.own)) })
    });
    project.or_else(|| {
        let text = app.settings.nwsync_upload.clone().filter(|t| !t.trim().is_empty())?;
        Some(UploadCommand { text, input: app.settings.nwsync_upload_input, project: None })
    })
}

/// Sends the repository `folder` to the web server: the upload command
/// run as a job once for each of `parts` the folder has, in turn, each
/// only after the one before succeeded. A project's command is asked
/// about first, as its `[commands]` are, until allowed as it reads.
pub(crate) fn upload(app: &mut Moonglow, folder: &Path, parts: &[Part]) {
    let say = |app: &mut Moonglow, e: String| {
        app.log.error(format!("NWSync upload: {e}"));
        if let Some(p) = &mut app.publish {
            p.error = Some(e);
        }
    };
    let Some(c) = upload_command(app) else {
        say(
            app,
            "no upload command: set one in Options › Folders, or the project's [nwsync] upload"
                .into(),
        );
        return;
    };
    if let Some((root, own)) = &c.project
        && !crate::project_commands::upload_allowed(app, root, &c.text)
    {
        app.project_commands.ask = Some(crate::project_commands::Ask {
            name: crate::project_commands::UPLOAD_NAME.into(),
            label: "its NWSync upload".into(),
            run: c.text.clone(),
            root: root.clone(),
            writes: false,
            input: c.input,
            upload: Some(crate::project_commands::UploadAsk {
                folder: folder.to_path_buf(),
                parts: parts.to_vec(),
                own: *own,
            }),
        });
        return;
    }
    let parts = match mg_module::nwsync::upload_parts(folder, parts) {
        Ok(parts) if parts.is_empty() => {
            say(app, format!("{} has nothing to send", folder.display()));
            return;
        }
        Ok(parts) => parts,
        Err(e) => {
            say(app, e);
            return;
        }
    };
    let steps = parts
        .iter()
        .map(|&part| {
            let text = mg_module::nwsync::upload_command(&c.text, folder, part);
            let mut command = crate::project_commands::shell(&text, folder, c.input);
            command.env("MOONGLOW_NWSYNC_FOLDER", folder).env("MOONGLOW_NWSYNC_PART", part.name());
            if let Some((root, _)) = &c.project {
                command.env("MOONGLOW_PROJECT", root);
            }
            let note = Some(format!("Sending {}: {text}", part.name()));
            crate::project_commands::Step { note, command }
        })
        .collect();
    app.log.info(format!("{UPLOADING}: {} (in {})", c.text, folder.display()));
    let run = crate::project_commands::Run {
        label: UPLOADING.into(),
        steps,
        input: c.input,
        output_at: "Publish to NWSync's Show Output",
        let_go: None,
    };
    let names: Vec<&'static str> = parts.iter().map(|p| p.name()).collect();
    run_steps(app, run, move |app, ended, step| {
        if ended.succeeded() {
            app.log.info(format!("Uploaded to NWSync: {} sent", names.join(", ")));
        } else if let Some(stopped) = names.get(step) {
            let sent = &names[..step];
            app.log.warn(format!(
                "NWSync upload: stopped sending {stopped}; {}: the web server serves what it did \
                 before",
                if sent.is_empty() {
                    "nothing was sent".to_string()
                } else {
                    format!("{} sent before it, nothing after", sent.join(", "))
                }
            ));
        }
    });
}

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
    let at = folder.clone();
    let latest = o.latest;
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
        move |app, r| {
            let Some(p) = &mut app.publish else { return };
            match r {
                Ok(w) => {
                    app.log.info(format!(
                        "Published to NWSync: manifest {} ({} files)",
                        w.sha1, w.files
                    ));
                    p.written = Some(w);
                    // (`latest` left as it was: not sent again.)
                    if app.settings.nwsync_upload_after {
                        let parts =
                            if latest { &AFTER_PUBLISHING[..] } else { &AFTER_PUBLISHING[..2] };
                        upload(app, &at, parts);
                    }
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
    let at = folder.clone();
    app.start_job_anywhere(
        PRUNING,
        move |_| mg_module::nwsync::prune(&folder, mg_module::nwsync::PRUNE_MIN_AGE, false),
        move |app, r| {
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
                    if app.settings.nwsync_upload_after {
                        upload(app, &at, &AFTER_PRUNING);
                    }
                }
                Err(e) => p.error = Some(e),
            }
        },
    );
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let busy = app.busy();
    let command = upload_command(app);
    let uploaded = crate::project_commands::last_of(app, UPLOADING);
    let settings = &mut app.settings;
    let Some(p) = &mut app.publish else { return };
    let mut open = true;
    let mut go = false;
    let mut prune = false;
    let mut send = false;
    let mut show_output = false;
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
                     with a web server (Upload sends it) and start nwserver with -nwsyncurl and \
                     its address (-nwsynchash {} to pin this manifest).",
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
                let tip = "Run the upload command once for each part of the folder, in turn: the \
                           data, then the manifests, then latest (after Remove Unused Data: the \
                           manifests, then the data), so that the web server never serves a \
                           manifest whose data isn't there yet";
                send = ui
                    .add_enabled(!busy && command.is_some(), egui::Button::new("Upload"))
                    .on_hover_text(tip)
                    .on_disabled_hover_text(
                        "Set an upload command in Options › Folders (or the project's \
                         [nwsync] upload)",
                    )
                    .clicked();
            });
            ui.separator();
            match &command {
                Some(c) => {
                    ui.horizontal(|ui| {
                        crate::widgets::field_label(ui, "Upload command");
                        ui.add(egui::Label::new(egui::RichText::new(&c.text).monospace()).wrap());
                    });
                    ui.weak(format!("Set in {}.", c.from()));
                    ui.add_enabled_ui(!busy, |ui| {
                        ui.checkbox(
                            &mut settings.nwsync_upload_after,
                            "Upload after publishing and after removing unused data",
                        );
                    });
                }
                None => {
                    ui.weak(
                        "To send the folder to your web server from here, set an upload command \
                         in Options › Folders (or the project's [nwsync] upload in \
                         moonglow.toml).",
                    );
                }
            }
            if let Some(ended) = &uploaded {
                ui.horizontal(|ui| {
                    crate::widgets::field_label(ui, "Last upload");
                    match ended {
                        Some(said) => ui.label(said),
                        None => ui.weak("Running…"),
                    };
                    show_output = ui.small_button("Show Output").clicked();
                });
            }
        });
    if show_output {
        app.project_commands.show = true;
    }
    if send {
        let folder = PathBuf::from(p.folder.trim());
        if p.folder.trim().is_empty() {
            p.error = Some("Choose the repository folder.".into());
        } else {
            p.error = None;
            app.settings.nwsync_repository = Some(folder.clone());
            upload(app, &folder, &AFTER_PUBLISHING);
        }
    }
    let Some(p) = &mut app.publish else { return };
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
