//! Build › NWSync Content…: the module's haks' content read from the
//! NWSync server its project names (`[nwsync] source`,
//! `mg_module::nwsync_content`): which manifest is in the game data, how
//! much of its data is here (the game's own NWSync storage, read only, and
//! Moonglow's cache), and **Sync**, which asks the server for its latest
//! manifest and downloads what is missing into the cache.
//!
//! The look and the download run on a thread of their own, not as a job:
//! a first download of a persistent world's content is gigabytes, and the
//! module stays open to work on meanwhile. The Status Bar shows a Sync
//! under way as it shows a job ([`NwSyncContent::shown`], read by
//! [`Moonglow::job_shown`] while no job runs), and the window says how
//! far it is and has Cancel; what was downloaded stays, so a Sync canceled
//! or cut off goes on from there the next time. Once it is done the
//! NWSync layer is put in again (the newer manifest, or the one there was
//! none of).
//!
//! **Clean Up Cache…** prunes Moonglow's cache ([`mg_nwsync::prune`]): it
//! says first what would go (a dry run), and removes it when asked. Kept:
//! the module's manifest, the newest each server named, and the last used
//! (`[nwsync] keep`, 3). The Status Bar shows a clean-up too, as it shows
//! a Sync.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use mg_nwsync::fetch::{self, Fetch, Pulled, Step};
use mg_nwsync::prune::{Keep, Pruned};
use mg_nwsync::{Settings, Survey};
use mg_resman::GameInstall;

use crate::Moonglow;

/// What the Status Bar calls a Sync under way.
pub const SYNC_TITLE: &str = "Sync NWSync Content";
/// What the Status Bar calls a clean-up of the cache under way (its dry
/// run too: it reads the whole cache).
pub const CLEAN_TITLE: &str = "Clean Up NWSync Cache";

/// What a thread of the window is doing, and what it made.
#[derive(Default)]
struct Run {
    syncing: bool,
    /// A clean-up of the cache: whether it is a dry run.
    cleaning: Option<bool>,
    cancel: AtomicBool,
    step: Mutex<Option<Step>>,
    done: Mutex<Option<Made>>,
}

enum Made {
    Looked(Result<Survey, String>),
    Synced(Result<Pulled, String>),
    Pruned(Result<Pruned, String>),
}

/// The NWSync Content window, and the module's NWSync content as it was
/// put into the game data.
#[derive(Default)]
pub struct NwSyncContent {
    pub open: bool,
    /// What became of the content when the game data was put together.
    pub mounted: Option<mg_module::nwsync_content::Mounted>,
    /// How much of the manifest's data is here, as last looked.
    pub survey: Option<Survey>,
    /// What the last Sync did, in a line.
    pub synced: Option<String>,
    /// What a clean-up of the cache would remove (its dry run), until it
    /// is done or called off.
    pub prune: Option<Pruned>,
    /// What the last clean-up removed, in a line.
    pub pruned: Option<String>,
    pub error: Option<String>,
    run: Option<Arc<Run>>,
    /// The requests (the tests' own; none: HTTP, `fetch::Client`).
    pub fetch: Option<Arc<dyn Fetch>>,
}

impl std::fmt::Debug for NwSyncContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NwSyncContent")
            .field("open", &self.open)
            .field("mounted", &self.mounted)
            .field("survey", &self.survey.as_ref().map(|s| s.missing.len()))
            .field("running", &self.running())
            .finish_non_exhaustive()
    }
}

impl NwSyncContent {
    /// Whether a look or a download is under way.
    pub fn running(&self) -> bool {
        self.run.is_some()
    }

    /// Whether a download is under way.
    pub fn syncing(&self) -> bool {
        self.run.as_ref().is_some_and(|r| r.syncing)
    }

    /// A Sync or a clean-up under way as the Status Bar shows a job: its
    /// title; a Sync's data files downloaded of those missing (once it
    /// knows them), and what it is doing (where its Cancel is, in the tip).
    pub(crate) fn shown(&self) -> Option<crate::jobs::JobShown> {
        let run = self.run.as_ref()?;
        if let Some(dry_run) = run.cleaning {
            let note = if dry_run {
                "Finding what no kept manifest needs"
            } else {
                "Removing what no kept manifest needs"
            };
            let title = CLEAN_TITLE.to_string();
            return Some(crate::jobs::JobShown { title, steps: None, note: note.to_string() });
        }
        if !run.syncing {
            return None;
        }
        let step = *run.step.lock().unwrap_or_else(|e| e.into_inner());
        let (steps, doing) = match step {
            None | Some(Step::Asking) => (None, "Asking the server for its manifest".to_string()),
            Some(Step::Looking) => {
                (None, "Looking for its data in the game's storage and the cache".to_string())
            }
            Some(Step::Downloading(p)) => (
                (p.of_files > 0).then_some((p.files.min(p.of_files), p.of_files)),
                format!("{} of {}", megabytes(p.bytes), megabytes(p.of_bytes)),
            ),
        };
        let doing =
            if run.cancel.load(Ordering::Relaxed) { "Canceling".to_string() } else { doing };
        let note = format!("{doing}\nBuild › NWSync Content… has its Cancel");
        Some(crate::jobs::JobShown { title: SYNC_TITLE.to_string(), steps, note })
    }

    /// The module closed: its content forgotten, a download under way
    /// called off (what it downloaded stays in the cache).
    pub(crate) fn module_closed(&mut self) {
        if let Some(r) = &self.run {
            r.cancel.store(true, Ordering::Relaxed);
        }
        let fetch = self.fetch.take();
        *self = NwSyncContent { fetch, ..Default::default() };
    }
}

/// The module's NWSync settings, where its project names a server.
fn settings(app: &Moonglow) -> Option<Settings> {
    app.ws.as_ref()?.module.project_file.nwsync_settings()
}

/// Starts `work` on a thread of its own; its result is taken at a later
/// frame ([`window`]).
fn start(
    app: &mut Moonglow,
    run: Run,
    work: impl FnOnce(&Run) -> Made + Send + 'static,
    ctx: Option<&egui::Context>,
) {
    let run = Arc::new(run);
    let theirs = run.clone();
    let ctx = ctx.cloned();
    let spawned = std::thread::Builder::new().name("nwsync-content".into()).spawn(move || {
        let made = work(&theirs);
        *theirs.done.lock().unwrap_or_else(|e| e.into_inner()) = Some(made);
        if let Some(ctx) = ctx {
            ctx.request_repaint();
        }
    });
    match spawned {
        Ok(_) => app.nwsync_content.run = Some(run),
        Err(e) => app.nwsync_content.error = Some(format!("Could not start: {e}")),
    }
}

/// Looks at how much of the manifest's data is here, on a thread.
fn look(app: &mut Moonglow, ctx: Option<&egui::Context>) {
    let (Some(settings), Some(install)) = (settings(app), app.install.clone()) else { return };
    if app.nwsync_content.running() {
        return;
    }
    start(app, Run::default(), move |_| Made::Looked(survey_of(&settings, &install)), ctx);
}

fn survey_of(settings: &Settings, install: &GameInstall) -> Result<Survey, String> {
    let game = mg_module::nwsync_content::game_storage(install)?;
    let cache = mg_module::nwsync_content::cache_of(settings);
    let found = mg_nwsync::find(&settings.url, settings.manifest, cache.as_ref(), game.as_ref())?;
    mg_nwsync::survey(&found.entries, game.as_ref(), cache.as_ref())
}

/// Starts a Sync: the server's manifest, and the data missing.
fn sync(app: &mut Moonglow, ctx: &egui::Context) {
    let (Some(settings), Some(install)) = (settings(app), app.install.clone()) else { return };
    if app.nwsync_content.running() {
        return;
    }
    let Some(cache) = mg_module::nwsync_content::cache_of(&settings) else {
        app.nwsync_content.error =
            Some("There is no cache folder to download into: set [nwsync] cache.".into());
        return;
    };
    let fetch: Arc<dyn Fetch> =
        app.nwsync_content.fetch.clone().unwrap_or_else(|| Arc::new(fetch::Client::new()));
    app.nwsync_content.error = None;
    app.nwsync_content.synced = None;
    let ctx_thread = ctx.clone();
    start(
        app,
        Run { syncing: true, ..Default::default() },
        move |run| {
            let pulled = (|| {
                let game = mg_module::nwsync_content::game_storage(&install)?;
                let step = |s: Step| {
                    *run.step.lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
                    ctx_thread.request_repaint();
                };
                fetch::pull(
                    &*fetch,
                    &settings,
                    &cache,
                    game.as_ref(),
                    fetch::Options::default(),
                    &step,
                    &|| run.cancel.load(Ordering::Relaxed),
                )
            })();
            Made::Synced(pulled)
        },
        Some(ctx),
    );
}

/// What a clean-up keeps: the manifest in the game data and the one
/// pinned, and as many last used as the project says.
fn keep_of(app: &Moonglow, settings: &Settings) -> Keep {
    let mut keep = Keep::default();
    if let Some(n) = app.ws.as_ref().and_then(|ws| ws.module.project_file.nwsync_keep) {
        keep.recent = n;
    }
    if let mg_nwsync::Wanted::Pinned(sha) = settings.manifest {
        keep.manifests.push(sha);
    }
    if let Some(Ok(l)) = app.nwsync_content.mounted.as_ref().map(|m| &m.layer) {
        keep.manifests.push(l.sha1);
    }
    keep
}

/// Cleans up the cache, on a thread: a dry run, or the clean-up itself.
pub(crate) fn prune(app: &mut Moonglow, dry_run: bool, ctx: Option<&egui::Context>) {
    let (Some(settings), Some(install)) = (settings(app), app.install.clone()) else { return };
    if app.nwsync_content.running() {
        return;
    }
    let Some(cache) = mg_module::nwsync_content::cache_of(&settings) else { return };
    let keep = keep_of(app, &settings);
    app.nwsync_content.error = None;
    app.nwsync_content.pruned = None;
    if !dry_run {
        app.nwsync_content.prune = None;
    }
    start(
        app,
        Run { cleaning: Some(dry_run), ..Default::default() },
        move |_| {
            Made::Pruned((|| {
                let game = mg_module::nwsync_content::game_storage(&install)?;
                mg_nwsync::prune::prune(&cache, game.as_ref(), &keep, dry_run)
            })())
        },
        ctx,
    );
}

/// What a clean-up removed, or would.
fn pruned_line(p: &Pruned) -> String {
    let manifests = match p.removed.len() {
        0 => String::new(),
        n => format!(", {}", count(n, "manifest")),
    };
    format!(
        "{} ({}, {}{manifests}; {} kept)",
        megabytes(p.bytes()),
        count(p.data_files, "data file"),
        count(p.haks, "written hak"),
        count(p.kept.len(), "manifest")
    )
}

/// Opens the window, and looks at what is here.
pub(crate) fn open(app: &mut Moonglow) {
    app.nwsync_content.open = true;
    app.nwsync_content.error = None;
    look(app, None);
}

/// Takes what a thread made, once it is done.
pub(crate) fn poll(app: &mut Moonglow) {
    let Some(run) = &app.nwsync_content.run else { return };
    let Some(made) = run.done.lock().unwrap_or_else(|e| e.into_inner()).take() else { return };
    app.nwsync_content.run = None;
    match made {
        Made::Looked(Ok(s)) => app.nwsync_content.survey = Some(s),
        Made::Looked(Err(e)) => {
            app.nwsync_content.survey = None;
            app.nwsync_content.error = Some(e);
        }
        Made::Synced(Err(e)) => {
            app.log.error(format!("NWSync content: {e}"));
            app.nwsync_content.error = Some(e);
        }
        Made::Synced(Ok(p)) => synced(app, p),
        Made::Pruned(Err(e)) => {
            app.nwsync_content.prune = None;
            app.nwsync_content.error = Some(e);
        }
        Made::Pruned(Ok(p)) if p.dry_run => {
            if let Some(why) = &p.data_left {
                app.nwsync_content.error = Some(why.clone());
            }
            app.nwsync_content.prune = Some(p);
        }
        Made::Pruned(Ok(p)) => {
            let line = format!("Removed {}", pruned_line(&p));
            app.log.info(format!("NWSync content: cache cleaned up: {}", pruned_line(&p)));
            app.nwsync_content.pruned = Some(line);
            app.nwsync_content.prune = None;
            look(app, None);
        }
    }
}

/// A Sync done: said, and the layer put in again.
fn synced(app: &mut Moonglow, p: Pulled) {
    for w in &p.resolved.warnings {
        app.log.warn(format!("NWSync content: {w}"));
    }
    let d = &p.downloaded;
    let left = p.before.missing.len() - d.files;
    let line = format!(
        "{}{} of {} downloaded ({:.1} MB){}",
        if d.stopped { "Canceled: " } else { "" },
        count(d.files, "data file"),
        p.before.missing.len(),
        d.received as f64 / 1_048_576.0,
        if left == 0 { String::new() } else { format!("; {left} still to download") }
    );
    let mut said = line.chars();
    let said: String =
        said.next().map(|c| c.to_lowercase().chain(said).collect()).unwrap_or_default();
    app.log.info(format!("NWSync content, manifest {}: {said}", p.resolved.found.short()));
    for (sha, why) in d.failed.iter().take(10) {
        app.log.warn(format!(
            "NWSync content: data file {} not downloaded: {why}",
            mg_nwsync::sha1_hex(sha)
        ));
    }
    app.nwsync_content.synced = Some(line);
    remount(app);
    look(app, None);
}

/// Puts the module's NWSync layer into the game data again, from what is
/// here now.
pub(crate) fn remount(app: &mut Moonglow) {
    let Some(settings) = settings(app) else { return };
    app.release_game_data();
    let haks = app.ws.as_ref().and_then(|ws| ws.module.haks().ok()).unwrap_or_default();
    // (What is on disk, for the rule: a hak or talk table newer than the
    // manifest over it.)
    let on_disk = match (app.ws.as_ref(), app.install.as_ref()) {
        (Some(ws), Some(gi)) => ws.module.nwsync_on_disk(gi),
        _ => Default::default(),
    };
    let (Some(game), Some(gi)) = (crate::exclusive(&mut app.game), &app.install) else { return };
    // (The haks not on disk, as the module's opening found them: those
    // still missing were said then.)
    let mut missing: Vec<String> = haks.into_iter().filter(|h| gi.find_hak(h).is_none()).collect();
    let m =
        mg_module::nwsync_content::mount(&mut game.resman, gi, &settings, &on_disk, &mut missing);
    for w in m.warnings() {
        app.log.warn(format!("NWSync content: {w}"));
    }
    app.nwsync_content.mounted = Some(m);
    app.game_data_changed();
    app.load_order_changed();
    app.reload_custom_tlk();
}

/// `n` of `what`: "1 data file", "3 data files".
fn count(n: usize, what: &str) -> String {
    if n == 1 { format!("1 {what}") } else { format!("{n} {what}s") }
}

/// A number with its thousands apart: 218,936.
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn megabytes(n: u64) -> String {
    format!("{:.1} MB", n as f64 / 1_048_576.0)
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    poll(app);
    if !app.nwsync_content.open {
        return;
    }
    let settings = settings(app);
    let c = &mut app.nwsync_content;
    let mut open = true;
    let (mut go, mut cancel) = (false, false);
    let (mut look_prune, mut remove, mut unprune) = (false, false, false);
    let step = c.run.as_ref().and_then(|r| *r.step.lock().unwrap_or_else(|e| e.into_inner()));
    let footed = crate::widgets::dialog(ctx, "NWSync Content", &mut open)
        .collapsible(false)
        .resizable(false)
        .show_foot(ctx, |ui| {
            ui.label(
                "The module's haks and talk table, read from the NWSync server players download \
                 them from, without the hak files.",
            );
            let Some(settings) = &settings else {
                ui.weak(
                    "The module's project names no NWSync server. Add its address to the \
                     project's moonglow.toml: [nwsync] source = \"https://…\".",
                );
                return ((), crate::widgets::Foot::new().leave("Close"));
            };
            egui::Grid::new("nwsync-content").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "Server");
                ui.monospace(&settings.url);
                ui.end_row();
                crate::widgets::field_label(ui, "Manifest");
                match c.mounted.as_ref().map(|m| &m.layer) {
                    Some(Ok(l)) => {
                        ui.horizontal(|ui| {
                            let hex = mg_nwsync::sha1_hex(&l.sha1);
                            ui.monospace(&hex[..12])
                                .on_hover_text(format!("{hex}, from {}", l.from.said()));
                            if ui.small_button("Copy").clicked() {
                                ui.ctx().copy_text(hex);
                            }
                        });
                    }
                    Some(Err(_)) | None => {
                        ui.weak("None here yet");
                    }
                }
                ui.end_row();
                if let Some(Ok(l)) = c.mounted.as_ref().map(|m| &m.layer) {
                    crate::widgets::field_label(ui, "Resources");
                    ui.label(thousands(l.resources));
                    ui.end_row();
                    crate::widgets::field_label(ui, "Haks");
                    ui.label(match (l.haks.len(), l.by_origins) {
                        (0, _) => "All on disk".to_string(),
                        (n, true) => format!("{} from NWSync, as its origins file names them", n),
                        (n, false) => format!("{} not on disk, taken to be in it", n),
                    });
                    ui.end_row();
                }
                if let Some(s) = &c.survey {
                    crate::widgets::field_label(ui, "In the game's storage");
                    ui.label(thousands(s.in_game));
                    ui.end_row();
                    crate::widgets::field_label(ui, "In Moonglow's cache");
                    ui.label(thousands(s.in_cache));
                    ui.end_row();
                    crate::widgets::field_label(ui, "Not downloaded");
                    if s.complete() {
                        ui.label("None");
                    } else {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            format!(
                                "{} ({})",
                                count(s.missing.len(), "data file"),
                                megabytes(s.missing_bytes)
                            ),
                        );
                    }
                    ui.end_row();
                }
            });
            match step {
                Some(Step::Asking) => {
                    ui.weak("Asking the server for its manifest…");
                }
                Some(Step::Looking) => {
                    ui.weak("Looking for its data in the game's storage and the cache…");
                }
                Some(Step::Downloading(p)) => {
                    let share =
                        if p.of_files == 0 { 0.0 } else { p.files as f32 / p.of_files as f32 };
                    ui.add(egui::ProgressBar::new(share).text(format!(
                        "{} of {} data files, {} of {}",
                        thousands(p.files),
                        thousands(p.of_files),
                        megabytes(p.bytes),
                        megabytes(p.of_bytes)
                    )));
                }
                None if c.running() => {
                    ui.weak("Looking at what is here…");
                }
                None => {}
            }
            if let Some(line) = &c.synced {
                ui.label(line);
            }
            if let Some(p) = &c.prune {
                ui.separator();
                if p.bytes() == 0 {
                    ui.label("Nothing to clean up: the cache holds only what is kept.");
                    if ui.button("OK").clicked() {
                        unprune = true;
                    }
                } else {
                    ui.label(format!("Clean up would remove {}.", pruned_line(p)));
                    ui.horizontal(|ui| {
                        remove = ui
                            .add_enabled(!c.running(), egui::Button::new("Remove"))
                            .on_hover_text(
                                "Remove from Moonglow's cache what no kept manifest needs",
                            )
                            .clicked();
                        unprune = ui.button("Keep Everything").clicked();
                    });
                }
            }
            if let Some(line) = &c.pruned {
                ui.label(line);
            }
            if let Some(e) = &c.error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            ui.weak(
                "Downloads and haks written for testing go to Moonglow's own cache; the game's \
                 NWSync storage is read, never changed.",
            );
            let mut foot = crate::widgets::Foot::new()
                .main("Sync", !c.running())
                .tip("Download the server's latest manifest and the data not here");
            if c.syncing() {
                foot = foot.button("Cancel", true);
            }
            let foot = foot.button("Clean Up Cache…", !c.running()).tip(
                "Say what of Moonglow's cache no kept manifest needs (data files, written haks, \
                 older manifests), to remove it",
            );
            ((), foot.leave("Close"))
        });
    if let Some(f) = footed {
        go = f.pressed.main();
        cancel = f.pressed.clicked("Cancel");
        look_prune = f.pressed.clicked("Clean Up Cache…");
        if f.pressed.cancelled() {
            open = false;
        }
    }
    if c.running() {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
    if cancel && let Some(r) = &c.run {
        r.cancel.store(true, Ordering::Relaxed);
    }
    if !open {
        c.open = false;
    }
    if go {
        sync(app, ctx);
    }
    if unprune {
        app.nwsync_content.prune = None;
    }
    if look_prune {
        prune(app, true, Some(ctx));
    }
    if remove {
        prune(app, false, Some(ctx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Sync under way shows in the Status Bar as a job does: under way
    /// while it asks and looks, then its data files done of those missing,
    /// the megabytes and where Cancel is in the tip; a look alone (the
    /// window's, as it opens) shows nothing there.
    #[test]
    fn a_sync_under_way_is_shown_as_a_job() {
        let at = |syncing: bool, step: Option<Step>| {
            let run = Run { syncing, step: Mutex::new(step), ..Default::default() };
            let c = NwSyncContent { run: Some(Arc::new(run)), ..Default::default() };
            c.shown().map(|s| (s.title, s.steps, s.note))
        };
        assert!(at(false, None).is_none(), "a look is no Sync");
        assert!(NwSyncContent::default().shown().is_none());
        let (title, steps, note) = at(true, Some(Step::Asking)).unwrap();
        assert_eq!((title.as_str(), steps), (SYNC_TITLE, None));
        assert!(note.starts_with("Asking the server"), "{note}");
        assert!(note.ends_with("Build › NWSync Content… has its Cancel"), "{note}");
        let p = fetch::Progress {
            files: 3,
            of_files: 10,
            bytes: 3 << 20,
            of_bytes: 10 << 20,
            received: 1 << 20,
        };
        let (_, steps, note) = at(true, Some(Step::Downloading(p))).unwrap();
        assert_eq!(steps, Some((3, 10)));
        assert!(note.starts_with("3.0 MB of 10.0 MB\n"), "{note}");
        // (Nothing missing yet counted: under way, not 0 of 0.)
        let none = fetch::Progress::default();
        assert_eq!(at(true, Some(Step::Downloading(none))).unwrap().1, None);
    }

    /// A clean-up of the cache under way shows in the Status Bar too, its
    /// dry run and the removal each saying which it is.
    #[test]
    fn a_clean_up_under_way_is_shown_as_a_job() {
        for dry_run in [true, false] {
            let run = Run { cleaning: Some(dry_run), ..Default::default() };
            let c = NwSyncContent { run: Some(Arc::new(run)), ..Default::default() };
            let shown = c.shown().unwrap();
            assert_eq!((shown.title.as_str(), shown.steps), (CLEAN_TITLE, None));
            let word = if dry_run { "Finding" } else { "Removing" };
            assert!(shown.note.starts_with(word), "{}", shown.note);
        }
    }
}
