//! Help › Check for Updates…, the daily check for a new release, and its
//! install (GitHub issue 28; the design in `docs/update-install-proposal.md`):
//! GitHub is asked for the latest release, a newer one is told with its
//! notes, and installed from here where Moonglow updates itself (the
//! AppImage, the Windows installer and zip, the macOS app), its package
//! checked against the release's signed `SHA256SUMS` first (`mg_update`).
//!
//! The requests are the application's: [`Updates::http`] is `None` unless
//! the desktop app gives one (`apps/moonglow`), so the command-line tool
//! and the tests never reach the network (the tests give a server of
//! their own on this computer, or answers made up).

use mg_uikit::Controls as _;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

pub use mg_update::{
    Api, Http, Installer, Kind, LATEST_API, PublicKey, RELEASES_PAGE, Release, Staged, Version,
};

use crate::{Action, Moonglow};

/// How long the automatic check waits between two requests, seconds.
pub const DAY: u64 = 24 * 60 * 60;

/// Where a request on its thread sends its answer.
type Answer = Receiver<Result<String, String>>;

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing newer than this copy is out.
    Newest,
    Newer(Release),
    /// GitHub couldn't be asked, or its answer read: why.
    Failed(String),
}

impl Outcome {
    /// What an answer says, against `current`: GitHub's latest release, or
    /// (`pre`, Include pre-releases) its list.
    pub fn of(answer: Result<String, String>, current: &Version, api: &Api, pre: bool) -> Outcome {
        let read = |json: String| {
            if pre {
                mg_update::release::parse_list(&json, api)
            } else {
                mg_update::release::parse_latest(&json, api)
            }
        };
        match answer.and_then(read) {
            Ok(Some(r)) if r.version > *current => Outcome::Newer(r),
            Ok(_) => Outcome::Newest,
            Err(e) => Outcome::Failed(e),
        }
    }
}

/// Whether the automatic check asks again: a day after it last did
/// (`last`, UNIX seconds), or at once if it never did, or if the clock is
/// now before that (it was set back).
pub fn due(last: Option<u64>, now: u64) -> bool {
    last.is_none_or(|last| now < last || now - last >= DAY)
}

/// Whether the notice is shown for `release`: not for the version the
/// builder said not to be reminded about (a later one is shown).
pub fn reminded(release: &Release, skipped: Option<&str>) -> bool {
    skipped.and_then(Version::parse).is_none_or(|s| release.version > s)
}

/// The time now, UNIX seconds.
pub use mg_core::time::unix_now as now;

/// An install of a release under way, or done.
#[derive(Debug)]
pub struct Getting {
    pub release: Release,
    /// Install Now (Restart Now is asked when it is ready); otherwise
    /// Later: it is installed as Moonglow closes.
    pub now: bool,
    pub state: GetState,
    /// How far the download has got (bytes, of how many).
    progress: Arc<Mutex<(u64, Option<u64>)>>,
    answer: Option<Receiver<Result<Staged, String>>>,
}

/// Where an install is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetState {
    /// Downloading and checking the package.
    Downloading,
    /// Checked and ready: installed as Moonglow closes (or, the AppImage
    /// with Install Now, installed already).
    Ready(Staged),
    /// Refused or failed: why (the release page is offered).
    Failed(String),
}

impl Getting {
    /// Bytes downloaded so far, of how many.
    pub fn progress(&self) -> (u64, Option<u64>) {
        *self.progress.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// An install of `release` in `state`, `got` of `of` bytes downloaded
    /// so far, with nothing under way: the pictures' and the tests' way to
    /// show each state of one (`get` starts a real one).
    #[doc(hidden)]
    pub fn shown(release: Release, now: bool, state: GetState, got: u64, of: Option<u64>) -> Self {
        let progress = Arc::new(Mutex::new((got, of)));
        Getting { release, now, state, progress, answer: None }
    }

    pub fn ready(&self) -> Option<&Staged> {
        match &self.state {
            GetState::Ready(s) => Some(s),
            _ => None,
        }
    }
}

/// The checks' and the installs' state.
pub struct Updates {
    /// Makes the requests (the desktop app's); `None`: never asked.
    pub http: Option<Arc<dyn Http>>,
    /// Where the releases are (GitHub's; a test's server).
    pub api: Api,
    /// The key the releases' checksums are signed with, built into the app
    /// (`packaging/minisign.pub`); `None`: nothing is installed from here.
    pub key: Option<PublicKey>,
    /// How this copy was installed, and so how it is updated.
    pub kind: Kind,
    /// Moonglow's cache folder for updates (the installer and the disk
    /// image downloaded, the last install's report).
    pub cache: Option<PathBuf>,
    /// The Check for Updates window is open.
    pub window: bool,
    /// What its check found (`None` while asking).
    pub report: Option<Outcome>,
    /// A newer release the automatic check found: the notice under the
    /// Top Bar, until dismissed.
    pub notice: Option<Release>,
    /// The install of a release, under way or ready.
    pub getting: Option<Getting>,
    /// Restart Now is asked in the window (Install Now's package is ready).
    pub ask_restart: bool,
    /// The notes' Markdown, drawn.
    notes: egui_commonmark::CommonMarkCache,
    /// When the automatic check last looked whether it was due.
    looked: Option<std::time::Instant>,
    /// The automatic check has asked since this program started: it asks
    /// once at each start (or as it is switched on), then once a day.
    asked: bool,
    /// The release the automatic check opened the window on, in this run:
    /// once for each (the daily check doesn't open it again mid-work).
    opened: Option<Version>,
    /// The request under way (on its thread): whether the automatic
    /// check and the window wait for it.
    pending: Option<(Answer, bool, bool)>,
}

impl Default for Updates {
    fn default() -> Updates {
        Updates {
            http: None,
            api: Api::default(),
            key: None,
            kind: Kind::Other,
            cache: None,
            window: false,
            report: None,
            notice: None,
            getting: None,
            ask_restart: false,
            notes: Default::default(),
            looked: None,
            asked: false,
            opened: None,
            pending: None,
        }
    }
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates")
            .field("http", &self.http.is_some())
            .field("key", &self.key.is_some())
            .field("kind", &self.kind)
            .field("report", &self.report)
            .field("notice", &self.notice)
            .field("getting", &self.getting)
            .finish_non_exhaustive()
    }
}

impl Updates {
    /// A check is under way.
    pub fn checking(&self) -> bool {
        self.pending.is_some()
    }

    /// A check under way that never answers: the pictures' and the tests'
    /// way to show the state of one (no request is made).
    #[doc(hidden)]
    pub fn check_never_answered(&mut self) {
        let (send, receive) = std::sync::mpsc::channel();
        // (Kept, so that the answer stays to come rather than the check
        // stopping.)
        std::mem::forget(send);
        self.pending = Some((receive, true, false));
    }

    /// The automatic check looks whether it is due at the next frame, not
    /// within the hour (Options were applied: it may have been switched
    /// on).
    pub fn look_again(&mut self) {
        self.looked = None;
    }

    /// The package manager that updates this copy, if one does.
    pub fn installer(&self) -> Option<Installer> {
        match self.kind {
            Kind::Managed(i) => Some(i),
            _ => None,
        }
    }

    /// Why a release can't be installed from here, if it can't.
    pub fn why_not(&self, release: &Release) -> Option<String> {
        if let Kind::Managed(i) = self.kind {
            return Some(format!("This copy is updated by {}: {}.", i.name(), i.how()));
        }
        let Some(package) = self.kind.package(&release.version) else {
            return Some(
                "Moonglow doesn't update this copy itself (a build of your own, or a \
                 distribution's package): install the new release as you installed this one."
                    .into(),
            );
        };
        if self.key.is_none() {
            return Some(
                "This build of Moonglow has no key to check a download against, so it doesn't \
                 install updates: download the new release from its page."
                    .into(),
            );
        }
        if self.http.is_none() {
            return Some("This copy of Moonglow doesn't download updates.".into());
        }
        // (Never the system's temporary folder in its place: a download
        // and its lock under names anyone can guess, in a folder every
        // user of the machine writes to.)
        if self.cache.is_none() {
            return Some(
                "Moonglow found no cache folder of yours to download an update into: download \
                 the new release from its page."
                    .into(),
            );
        }
        if release.asset(&package).is_none() {
            return Some(format!("The release has no {package} for this copy."));
        }
        None
    }

    /// The update to install as Moonglow closes (the desktop app installs
    /// it after the window has gone); `None` if there is none.
    pub fn at_close(&mut self) -> Option<Staged> {
        match self.getting.take() {
            Some(Getting { state: GetState::Ready(staged), .. }) => Some(staged),
            other => {
                self.getting = other;
                None
            }
        }
    }
}

/// How often the automatic check looks whether a day has gone by.
const LOOK_EVERY: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// The address the check asks.
fn check_url(app: &Moonglow) -> String {
    if app.settings.update_prereleases {
        app.updates.api.list.clone()
    } else {
        app.updates.api.latest.clone()
    }
}

/// Help › Check for Updates…: the window, and a request.
pub fn check_now(app: &mut Moonglow) {
    app.updates.window = true;
    app.updates.report = None;
    // (The Flatpak's sandbox has no network, and Flatpak updates it: the
    // window says how, and nothing is asked.)
    if app.updates.installer() == Some(Installer::Flatpak) {
        return;
    }
    start(app, false, true);
}

/// The notice's What's New…: the window, with the release found.
pub fn whats_new(app: &mut Moonglow, release: Release) {
    app.updates.window = true;
    app.updates.report = Some(Outcome::Newer(release));
}

/// Starts a request (or joins the one under way).
fn start(app: &mut Moonglow, auto: bool, manual: bool) {
    if let Some((_, a, m)) = &mut app.updates.pending {
        *a |= auto;
        *m |= manual;
        return;
    }
    let Some(http) = app.updates.http.clone() else {
        if manual {
            app.updates.report =
                Some(Outcome::Failed("This copy of Moonglow doesn't check for updates".into()));
        }
        return;
    };
    let url = check_url(app);
    if !app.background_jobs {
        // (Tests: at once.)
        finish(app, http.ask(&url), auto, manual);
        return;
    }
    let (send, receive) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new().name("update-check".into()).spawn(move || {
        let _ = send.send(http.ask(&url));
    });
    match spawned {
        Ok(_) => app.updates.pending = Some((receive, auto, manual)),
        Err(e) => finish(app, Err(format!("no thread to ask on: {e}")), auto, manual),
    }
}

/// What a request found, for the window and the notice.
fn finish(app: &mut Moonglow, answer: Result<String, String>, auto: bool, manual: bool) {
    let pre = app.settings.update_prereleases;
    let outcome = Outcome::of(answer, &Version::current(), &app.updates.api, pre);
    crate::trace::note(format!("update check: {outcome:?}"));
    if auto
        && let Outcome::Newer(r) = &outcome
        && reminded(r, app.settings.update_skipped.as_deref())
    {
        app.updates.notice = Some(r.clone());
        // Install updates automatically: downloaded now, installed as
        // Moonglow closes.
        let getting = app.updates.getting.as_ref().map(|g| &g.release.version);
        let auto_install = app.settings.update_auto_install && app.updates.why_not(r).is_none();
        if auto_install && getting != Some(&r.version) {
            get(app, r.clone(), false);
        }
        // Otherwise the window opens on it, asking (a builder's ask: the
        // line under the toolbar alone went unseen): once a run for each
        // release, not while it is open, nor for one being installed.
        else if !auto_install
            && getting != Some(&r.version)
            && !app.updates.window
            && app.updates.opened.as_ref() != Some(&r.version)
        {
            app.updates.opened = Some(r.version.clone());
            whats_new(app, r.clone());
        }
    }
    if manual {
        app.updates.report = Some(outcome);
    }
}

/// Install Now (`now`) or Later: `release`'s package downloaded and
/// checked, on a thread (with jobs in the background), and made ready;
/// with Install Now an AppImage is replaced at once, and Restart Now
/// asked when it is ready.
pub fn get(app: &mut Moonglow, release: Release, now: bool) {
    // (One at a time: the same release asked for again keeps its download,
    // now with Install Now's question if it was asked so.)
    if let Some(g) = &mut app.updates.getting
        && g.release.version == release.version
        && !matches!(g.state, GetState::Failed(_))
    {
        if now && !g.now {
            g.now = true;
            app.updates.ask_restart = g.ready().is_some();
            if let Some(staged) = g.ready().cloned() {
                install_image_now(app, staged);
            }
        }
        return;
    }
    let progress = Arc::new(Mutex::new((0, None)));
    let mut getting = Getting {
        release: release.clone(),
        now,
        state: GetState::Downloading,
        progress: progress.clone(),
        answer: None,
    };
    if let Some(why) = app.updates.why_not(&release) {
        getting.state = GetState::Failed(why);
        app.updates.getting = Some(getting);
        return;
    }
    let u = &app.updates;
    let (http, key) = (u.http.clone().expect("why_not"), u.key.clone().expect("why_not"));
    let (kind, cache) = (u.kind.clone(), u.cache.clone().expect("why_not"));
    crate::trace::note(format!("update: getting {} ({})", release.version, kind.what()));
    let work = move || {
        let mut said = |got, of| *progress.lock().unwrap_or_else(|e| e.into_inner()) = (got, of);
        mg_update::stage::fetch(&*http, &key, &release, &kind, &cache, &mut said)
    };
    if !app.background_jobs {
        app.updates.getting = Some(getting);
        got(app, work());
        return;
    }
    let (send, receive) = std::sync::mpsc::channel();
    match std::thread::Builder::new().name("update-download".into()).spawn(move || {
        let _ = send.send(work());
    }) {
        Ok(_) => {
            getting.answer = Some(receive);
            app.updates.getting = Some(getting);
        }
        Err(e) => {
            getting.state = GetState::Failed(format!("no thread to download on: {e}"));
            app.updates.getting = Some(getting);
        }
    }
}

/// The package downloaded and checked, or why not.
fn got(app: &mut Moonglow, outcome: Result<Staged, String>) {
    let Some(g) = &mut app.updates.getting else { return };
    let version = g.release.version.clone();
    let now = g.now;
    match outcome {
        Ok(staged) => {
            app.log.info(format!(
                "Moonglow {version} is downloaded, and its checksum and signature verified."
            ));
            g.state = GetState::Ready(staged.clone());
            if now {
                // (Before the install, which takes the question back
                // where the image could not be replaced.)
                app.updates.ask_restart = true;
                install_image_now(app, staged);
            }
        }
        Err(why) => {
            app.log.warn(format!("Moonglow {version} isn't installed: {why}."));
            g.state = GetState::Failed(why);
        }
    }
}

/// Install Now of an AppImage: the image replaced at once (the running
/// copy goes on reading the file it opened).
fn install_image_now(app: &mut Moonglow, staged: Staged) {
    if !matches!(staged.ready, mg_update::Ready::Image { .. }) {
        return;
    }
    // (What is staged was downloaded into the cache folder: there is one.)
    let Some(cache) = app.updates.cache.clone() else { return };
    let outcome = mg_update::install::install(&staged, false, &cache);
    let Some(g) = &mut app.updates.getting else { return };
    match outcome {
        Ok(_) => {
            app.log.info(format!(
                "Moonglow {} is installed in place of {}: it runs from the next start.",
                staged.version,
                staged.kind.what()
            ));
            g.state = GetState::Ready(Staged { ready: mg_update::Ready::Installed, ..staged });
        }
        Err(why) => {
            app.log.warn(format!("Moonglow {} isn't installed: {why}.", staged.version));
            g.state = GetState::Failed(why);
            app.updates.ask_restart = false;
        }
    }
}

/// **Restart Now**, once unsaved work is saved or discarded: Moonglow
/// closes, installs what is ready, and starts again with the module.
pub(crate) fn restart(app: &mut Moonglow) {
    let Some(staged) = app.updates.getting.as_ref().and_then(Getting::ready) else { return };
    let Some(cache) = app.updates.cache.clone() else { return };
    let start = mg_update::install::plan(staged, true, &cache).ok().map(|(_, again)| again);
    crate::trace::note(format!("update: restart to install {}", staged.version));
    let backend = app.renderer_check.adapter.as_ref().map_or(wgpu::Backend::Vulkan, |a| a.backend);
    app.relaunch = Some(crate::renderer_check::Relaunch {
        module: app.module_path(),
        backend,
        environment: false,
        start,
    });
}

/// The answers to the requests under way, and the automatic check when
/// it is due.
pub(crate) fn frame(app: &mut Moonglow, ctx: &egui::Context) {
    if let Some(g) = &mut app.updates.getting
        && let Some(answer) = &g.answer
    {
        match answer.try_recv() {
            Ok(outcome) => {
                g.answer = None;
                got(app, outcome);
                ctx.request_repaint();
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            Err(TryRecvError::Disconnected) => {
                g.answer = None;
                got(app, Err("the download stopped".into()));
            }
        }
    }
    let looked = app.updates.looked.is_some_and(|t| t.elapsed() < LOOK_EVERY);
    if !looked && app.updates.http.is_some() {
        app.updates.looked = Some(std::time::Instant::now());
        let now = now();
        // (At each start too, not only a day after the last: a release
        // out since that day's check was told only the next day, though a
        // builder had started Moonglow again, Carcerian's report on 3.1.0,
        // and the notice isn't kept from one run to the next.)
        if app.updates.installer().is_none()
            && !app.settings.no_update_check
            && (!app.updates.asked || due(app.settings.update_checked, now))
        {
            // (Noted as it starts: a day goes by between two requests of a
            // run, whatever they found.)
            app.settings.update_checked = Some(now);
            app.updates.asked = true;
            start(app, true, false);
        }
    }
    let Some((receive, auto, manual)) = &app.updates.pending else { return };
    let (auto, manual) = (*auto, *manual);
    match receive.try_recv() {
        Ok(answer) => {
            app.updates.pending = None;
            finish(app, answer, auto, manual);
            ctx.request_repaint();
        }
        Err(TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        Err(TryRecvError::Disconnected) => {
            app.updates.pending = None;
            finish(app, Err("the check stopped".into()), auto, manual);
        }
    }
}

/// At start: what a failed or interrupted update left removed, and the
/// outcome of the install as Moonglow last closed put in the log.
pub fn startup(app: &mut Moonglow) {
    let Some(cache) = app.updates.cache.clone() else { return };
    mg_update::stage::clean(&app.updates.kind, &cache);
    if let Some(report) = mg_update::install::take_report(&cache) {
        let report = report.trim().to_string();
        if report.contains("isn't installed") {
            app.log.warn(report);
        } else {
            app.log.info(report);
        }
    }
}

/// What a release is called: `Moonglow 3.3.0`, `Moonglow 3.3.0-rc.1
/// (a pre-release)`.
fn named(release: &Release) -> String {
    if release.version.is_pre() {
        format!("Moonglow {} (a pre-release)", release.version)
    } else {
        format!("Moonglow {}", release.version)
    }
}

/// `n` bytes in megabytes.
fn megabytes(n: u64) -> String {
    format!("{:.1} MB", n as f64 / 1_000_000.0)
}

/// What was clicked of an install's buttons.
#[derive(PartialEq, Eq)]
enum Clicked {
    Nothing,
    RestartNow,
    Later,
    TryAgain,
}

/// Where the install of `getting` is, with its buttons: in the notice
/// (`notice`) and in the window, which (`ask`) asks to restart now; a
/// failure offers Try Again (`retry`: unless nothing here installs it).
fn install_state(
    ui: &mut egui::Ui,
    getting: &Getting,
    notice: bool,
    ask: bool,
    retry: bool,
) -> Clicked {
    let mut clicked = Clicked::Nothing;
    let version = &getting.release.version;
    match &getting.state {
        GetState::Downloading => {
            let (got, of) = getting.progress();
            ui.spinner();
            match of.filter(|&t| t > 0) {
                Some(total) => {
                    ui.label(format!(
                        "Downloading Moonglow {version}… {} of {}",
                        megabytes(got),
                        megabytes(total)
                    ));
                    if !notice {
                        let bar = egui::ProgressBar::new(got as f32 / total as f32)
                            .desired_width(160.0)
                            .show_percentage();
                        ui.add(bar);
                    }
                }
                None => {
                    ui.label(format!("Downloading Moonglow {version}… {}", megabytes(got)));
                }
            }
        }
        GetState::Ready(staged) => {
            let said = if staged.installed() {
                format!("Moonglow {version} is installed: it runs from the next start.")
            } else {
                format!(
                    "Moonglow {version} is downloaded and verified: it is installed when \
                     Moonglow closes."
                )
            };
            ui.label(said);
            if ask {
                ui.label("Restart Moonglow now?");
            }
            if ui
                .button("Restart Now")
                .on_hover_text("Install the update and restart with the module that is open")
                .clicked()
            {
                clicked = Clicked::RestartNow;
            }
            if ask
                && ui
                    .button("Later")
                    .on_hover_text(
                        "Install it when Moonglow closes. The next start is the new version.",
                    )
                    .clicked()
            {
                clicked = Clicked::Later;
            }
        }
        GetState::Failed(why) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Moonglow {version} isn't installed: {why}."),
            );
            if retry
                && ui
                    .button("Try Again")
                    .on_hover_text("Downloads it again from the start, and verifies it again")
                    .clicked()
            {
                clicked = Clicked::TryAgain;
            }
            let (words, tip) = Offer::ReleasePage.text();
            ui.hyperlink_to(words, &getting.release.page).on_hover_text(tip);
        }
    }
    clicked
}

/// A button of the update's line on the Welcome page ([`line`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Offer {
    WhatsNew,
    InstallNow,
    Later,
    ReleasePage,
    RestartNow,
    TryAgain,
}

impl Offer {
    /// Its name and tip, the notice's.
    pub(crate) fn text(self) -> (&'static str, &'static str) {
        match self {
            Offer::WhatsNew => ("What's New…", "The release's notes, and how to install it"),
            Offer::InstallNow => {
                ("Install Now", "Download, verify and install it, then ask to restart")
            }
            Offer::Later => {
                ("Later", "Download and verify it now, and install it when Moonglow closes")
            }
            Offer::ReleasePage => {
                ("Release Page", "What is new in it, and its downloads (opens the browser)")
            }
            Offer::RestartNow => {
                ("Restart Now", "Install the update and restart with the module that is open")
            }
            Offer::TryAgain => {
                ("Try Again", "Downloads it again from the start, and verifies it again")
            }
        }
    }
}

/// What the Welcome page's Status says of updates: the notice's states
/// (a newer release, its download, ready, failed), and a check under way.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
    /// What it says.
    pub(crate) words: String,
    /// Said as an error (a failed install).
    pub(crate) error: bool,
    /// Under way (a check, a download): a spinner in place of its icon.
    pub(crate) busy: bool,
    /// Its buttons, in order.
    pub(crate) offers: Vec<Offer>,
    /// The release it is of (none for a check), and whether it was to be
    /// installed now (Try Again's).
    release: Option<(Release, bool)>,
}

/// Whether this copy installs `release` ([`Updates::why_not`]), checked
/// once for each release and way this copy is set up, not each frame.
fn why_not(u: &Updates, release: &Release) -> Option<String> {
    type Key = (String, usize, String, bool, bool, Option<std::path::PathBuf>);
    thread_local! {
        static KEPT: std::cell::RefCell<Option<(Key, Option<String>)>> =
            const { std::cell::RefCell::new(None) };
    }
    let key: Key = (
        release.version.to_string(),
        release.assets.len(),
        format!("{:?}", u.kind),
        u.key.is_some(),
        u.http.is_some(),
        u.cache.clone(),
    );
    KEPT.with(|kept| {
        let mut kept = kept.borrow_mut();
        if let Some((k, why)) = kept.as_ref()
            && *k == key
        {
            return why.clone();
        }
        crate::welcome::count(crate::welcome::Counted::InstallCheck);
        let why = u.why_not(release);
        *kept = Some((key, why.clone()));
        why
    })
}

/// The update's line, if there is one to tell: the words and buttons of
/// the strip under the Top Bar (`notice_bar`, which adds Don't Remind Me
/// and its Close) and of the Welcome page's Status, which also says
/// "Checking for updates…" while a check is under way. Nothing while the
/// Check for Updates window tells of the same release (once is enough).
pub(crate) fn line(app: &Moonglow) -> Option<Line> {
    let u = &app.updates;
    let release = u.notice.as_ref().or(u.getting.as_ref().map(|g| &g.release));
    let told = |r: &Release| matches!(&u.report, Some(Outcome::Newer(t)) if t.version == r.version);
    if u.window && release.is_some_and(told) {
        return None;
    }
    let Some(release) = release else {
        let checking = Line {
            words: "Checking for updates…".into(),
            error: false,
            busy: true,
            offers: Vec::new(),
            release: None,
        };
        return u.checking().then_some(checking);
    };
    let installable = why_not(u, release).is_none();
    let version = &release.version;
    let getting = u.getting.as_ref().filter(|g| g.release.version == *version);
    let (words, error, busy, offers) = match getting.map(|g| (g, &g.state)) {
        Some((g, GetState::Downloading)) => {
            let (got, of) = g.progress();
            let words = match of.filter(|&t| t > 0) {
                Some(total) => format!(
                    "Downloading Moonglow {version}… {} of {}",
                    megabytes(got),
                    megabytes(total)
                ),
                None => format!("Downloading Moonglow {version}… {}", megabytes(got)),
            };
            (words, false, true, vec![])
        }
        Some((_, GetState::Ready(staged))) => {
            let words = if staged.installed() {
                format!("Moonglow {version} is installed: it runs from the next start.")
            } else {
                format!(
                    "Moonglow {version} is downloaded and verified: it is installed when \
                     Moonglow closes."
                )
            };
            (words, false, false, vec![Offer::RestartNow])
        }
        Some((_, GetState::Failed(why))) => {
            let words = format!("Moonglow {version} isn't installed: {why}.");
            let offers = match installable {
                true => vec![Offer::TryAgain, Offer::ReleasePage],
                false => vec![Offer::ReleasePage],
            };
            (words, true, false, offers)
        }
        None => {
            let words = format!("{} is out (you have {}).", named(release), Version::current());
            let offers = match installable {
                true => vec![Offer::WhatsNew, Offer::InstallNow, Offer::Later],
                false => vec![Offer::WhatsNew, Offer::ReleasePage],
            };
            (words, false, false, offers)
        }
    };
    let now = getting.is_some_and(|g| g.now);
    Some(Line { words, error, busy, offers, release: Some((release.clone(), now)) })
}

/// Does what `offer` of `line` says.
pub(crate) fn act(app: &mut Moonglow, ctx: &egui::Context, line: &Line, offer: Offer) {
    let Some((release, now)) = line.release.clone() else { return };
    match offer {
        Offer::WhatsNew => whats_new(app, release),
        Offer::InstallNow => get(app, release, true),
        Offer::Later => get(app, release, false),
        Offer::TryAgain => get(app, release, now),
        Offer::ReleasePage => ctx.open_url(egui::OpenUrl::new_tab(&release.page)),
        Offer::RestartNow => app.actions.push(Action::RestartToUpdate),
    }
}

/// The notice of a newer release, under the Top Bar, while there is one:
/// [`line`]'s words and buttons, with Don't Remind Me and its Close; not
/// while the Welcome page tells it in sight (M4's U24: once is enough), nor
/// while the Check for Updates window does.
pub(crate) fn notice_bar(app: &mut Moonglow, ui: &mut egui::Ui) {
    let Some(release) = app.updates.notice.clone() else { return };
    if crate::welcome::in_sight(app, ui.ctx()) {
        return;
    }
    let Some(line) = line(app) else { return };
    let mut close = false;
    let mut offered = None;
    egui::Panel::top("update-notice").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let side = egui::Vec2::splat(mg_uikit::metrics::ICON);
            let (mark, _) = ui.allocate_exact_size(side, egui::Sense::hover());
            mg_uikit::Icon::Info.paint(ui.painter(), mark, ui.visuals().hyperlink_color);
            if line.busy {
                ui.spinner();
            }
            match line.error {
                true => ui.colored_label(ui.visuals().error_fg_color, &line.words),
                false => ui.label(&line.words),
            };
            for &offer in &line.offers {
                let (words, tip) = offer.text();
                if offer == Offer::ReleasePage {
                    ui.hyperlink_to(words, &release.page).on_hover_text(tip);
                } else if ui.button(words).on_hover_text(tip).clicked() {
                    offered = Some(offer);
                }
            }
            // (Of a release not yet being got: What's New… leads.)
            if line.offers.first() == Some(&Offer::WhatsNew)
                && ui
                    .button("Don't Remind Me About This Version")
                    .on_hover_text("No notice for this version. A later one is told again.")
                    .clicked()
            {
                app.settings.update_skipped = Some(release.version.to_string());
                close = true;
            }
            // (Named for what it closes: the renderer's notice can show with it.)
            let tip = "Close the notice (a later check tells again)";
            let close_icon = mg_uikit::Icon::Close;
            if crate::widgets::icon_button(ui, close_icon, "Close Update Notice", tip).clicked() {
                close = true;
            }
        });
    });
    if let Some(offer) = offered {
        act(app, ui.ctx(), &line, offer);
    } else if close {
        app.updates.notice = None;
    }
}

/// What the window says of a check that found `outcome`, and the newer
/// release's notes and install.
fn said(app: &mut Moonglow, ui: &mut egui::Ui, outcome: &Outcome) -> Option<Action> {
    let r = match outcome {
        Outcome::Newest => {
            ui.label(format!("Moonglow {} is the newest.", Version::current()));
            return None;
        }
        Outcome::Failed(why) => {
            ui.colored_label(ui.visuals().error_fg_color, format!("Couldn't ask GitHub: {why}."));
            return None;
        }
        Outcome::Newer(r) => r,
    };
    ui.label(format!("{} is out (you have {}).", named(r), Version::current()));
    let notes = r.notes.trim();
    if !notes.is_empty() {
        ui.add_space(4.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(260.0).auto_shrink([false, true]).show(
                ui,
                |ui| {
                    egui_commonmark::CommonMarkViewer::new().show(
                        ui,
                        &mut app.updates.notes,
                        notes,
                    );
                },
            );
        });
    }
    ui.hyperlink_to("Open the release page", &r.page)
        .on_hover_text("What is new in it, and its downloads (opens the browser)");
    ui.add_space(4.0);
    let mut action = None;
    let getting = app.updates.getting.as_ref().filter(|g| g.release.version == r.version);
    match (getting, app.updates.why_not(r)) {
        (Some(g), _) => {
            let ask = app.updates.ask_restart && g.ready().is_some();
            let retry = app.updates.why_not(r).is_none();
            let now = g.now;
            match ui.horizontal_wrapped(|ui| install_state(ui, g, false, ask, retry)).inner {
                Clicked::RestartNow => action = Some(Action::RestartToUpdate),
                Clicked::Later => {
                    app.updates.ask_restart = false;
                    app.updates.window = false;
                }
                Clicked::TryAgain => get(app, r.clone(), now),
                Clicked::Nothing => {}
            }
        }
        (None, Some(why)) => {
            ui.weak(why);
            // (The notice's, which the Welcome page's Status leaves out:
            // here for a copy that does not install too.)
            if ui
                .button("Don't Remind Me About This Version")
                .on_hover_text("No notice for this version. A later one is told again.")
                .clicked()
            {
                app.settings.update_skipped = Some(r.version.to_string());
                app.updates.notice = None;
            }
        }
        (None, None) => {
            ui.horizontal(|ui| {
                if ui
                    .button("Install Now")
                    .on_hover_text("Download, verify and install it, then ask to restart")
                    .clicked()
                {
                    get(app, r.clone(), true);
                }
                if ui
                    .button("Later")
                    .on_hover_text(
                        "Download and verify it now, and install it when Moonglow closes",
                    )
                    .clicked()
                {
                    get(app, r.clone(), false);
                }
                if ui
                    .button("Don't Remind Me About This Version")
                    .on_hover_text("No notice for this version. A later one is told again.")
                    .clicked()
                {
                    app.settings.update_skipped = Some(r.version.to_string());
                    app.updates.notice = None;
                }
            });
            ui.weak(format!("Installs it in place of {}.", app.updates.kind.what()));
            let always = ui
                .check_box(&mut app.settings.update_auto_install, "Always install updates")
                .on_hover_text(
                    "Install this and later releases when Moonglow closes, without asking",
                );
            // (Ticked: this one too, as Later gets it.)
            if always.changed() && app.settings.update_auto_install {
                get(app, r.clone(), false);
            }
        }
    }
    action
}

/// Help › Check for Updates…, while open.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    if !app.updates.window {
        return;
    }
    let title = "Check for Updates";
    let mut open = true;
    let (mut again, mut close, mut action) = (false, false, None);
    let footed = crate::widgets::dialog(ctx, title, &mut open)
        .collapsible(false)
        .resizable(false)
        .show_foot(ctx, |ui| {
            crate::widgets::fit_width(ui, 520.0);
            let flatpak = app.updates.installer() == Some(Installer::Flatpak);
            match app.updates.report.clone() {
                None if flatpak => {}
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Asking GitHub for the latest release…");
                    });
                }
                Some(outcome) => action = said(app, ui, &outcome),
            }
            let newer = matches!(app.updates.report, Some(Outcome::Newer(_)));
            if let Some(i) = app.updates.installer()
                && !newer
            {
                ui.add_space(4.0);
                ui.label(format!("This copy is updated by {}: {}.", i.name(), i.how()));
            }
            if !flatpak {
                ui.add_space(6.0);
                ui.weak(
                    "Sent: one request to GitHub for the latest release's version, notes and \
                     page; installing downloads the release's package and its signed \
                     checksums from GitHub. Nothing about you, your modules or this computer.",
                );
            }
            let mut foot = crate::widgets::Foot::new();
            if !flatpak {
                foot = foot.main("Check Again", !app.updates.checking());
            }
            ((), foot.leave("Close"))
        });
    if let Some(f) = footed {
        again = f.pressed.main();
        close = f.pressed.cancelled();
    }
    if let Some(a) = action {
        app.actions.push(a);
    }
    if again {
        check_now(app);
    }
    if !open || close {
        app.updates.window = false;
        app.updates.ask_restart = false;
    }
}

/// Options › Updates' switches.
pub(crate) struct UpdateOptions<'a> {
    pub check: &'a mut bool,
    pub auto_install: &'a mut bool,
    pub prereleases: &'a mut bool,
    pub skipped: &'a mut Option<String>,
}

/// What Options › Updates knows of this copy: the package manager that
/// updates it, and why it doesn't install updates itself (if it doesn't).
pub(crate) struct UpdatesHere {
    pub installer: Option<Installer>,
    pub cannot: Option<String>,
    pub checked: Option<u64>,
}

impl Updates {
    /// Why no release is installed from here, whatever it holds.
    pub(crate) fn here(&self, checked: Option<u64>) -> UpdatesHere {
        let probe = Release {
            version: Version::current(),
            tag: String::new(),
            page: String::new(),
            notes: String::new(),
            assets: Vec::new(),
        };
        let cannot = match self.why_not(&probe) {
            // (The probe has no package: that is no reason here.)
            Some(why) if !why.starts_with("The release has no") => Some(why),
            _ => None,
        };
        UpdatesHere { installer: self.installer(), cannot, checked }
    }
}

/// Options › Updates.
pub(crate) fn options_page(ui: &mut egui::Ui, o: UpdateOptions, here: &UpdatesHere) {
    let (installer, cannot, checked) = (here.installer, here.cannot.clone(), here.checked);
    let managed = installer.is_some();
    let mut shown = *o.check && !managed;
    let response = ui
        .add_enabled(
            !managed,
            mg_uikit::Checkbox::new(&mut shown, "Check for updates automatically"),
        )
        .on_hover_text("Ask GitHub at each start and daily whether a newer Moonglow is out");
    if response.changed() {
        *o.check = shown;
    }
    let mut auto = *o.auto_install && cannot.is_none();
    let response = ui
        .add_enabled(
            cannot.is_none(),
            mg_uikit::Checkbox::new(&mut auto, "Install updates automatically"),
        )
        .on_hover_text(
            "Download newer releases in the background and install them when Moonglow closes",
        );
    if response.changed() {
        *o.auto_install = auto;
    }
    ui.check_box(o.prereleases, "Include pre-releases").on_hover_text(
        "Offer newer pre-releases too, to try before they are out. Off: releases only.",
    );
    // (The Flatpak's sandbox has no network: nothing is sent from it, as
    // Help › Check for Updates… there says. Scoop's and Nix's copies have
    // no automatic check: only what Check for Updates… sends.)
    let flatpak = installer == Some(Installer::Flatpak);
    if installer.is_some() && !flatpak {
        ui.weak(
            "Sent, when Help › Check for Updates… asks: one request to GitHub \
             (api.github.com) for the latest release's version, notes and page (with Include \
             pre-releases, the twenty newest releases'): nothing about you, your modules or \
             this computer.",
        );
    } else if !flatpak {
        ui.weak(
            "Sent: one request to GitHub (api.github.com) at each start and once a day while \
             Moonglow runs, for the latest release's version, notes and page (with Include \
             pre-releases, the twenty newest releases'): nothing about you, your modules or \
             this computer. Offline, or when GitHub doesn't answer, nothing is said. Help › \
             Check for Updates… asks at once. Installing downloads the release's package for \
             this copy and its checksums (SHA256SUMS, signed) from github.com, and installs it \
             only if they verify.",
        );
    }
    if let Some(i) = installer {
        ui.add_space(4.0);
        // (Scoop's and Nix's copies still ask when asked to: only the
        // automatic check is off. The Flatpak asks nothing.)
        let asks = if flatpak {
            "Moonglow asks GitHub nothing from its sandbox. Help › Check for Updates… says how \
             to update."
        } else {
            "Moonglow doesn't check by itself; Help › Check for Updates… still asks."
        };
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!("This copy is updated by {}: {}. {asks}", i.name(), i.how()),
        );
    } else if let Some(why) = cannot {
        ui.add_space(4.0);
        ui.weak(why);
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Last checked");
        ui.label(checked.map_or_else(|| "Not yet".to_string(), local_time));
    });
    if let Some(version) = o.skipped.clone() {
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Not reminded about");
            ui.label(format!("Moonglow {version}"));
            if ui
                .button("Remind Me Again")
                .on_hover_text("The notice tells of this version again")
                .clicked()
            {
                *o.skipped = None;
            }
        });
    }
}

/// A time (UNIX seconds) on this computer's clock.
fn local_time(seconds: u64) -> String {
    let Ok(t) = jiff::Timestamp::from_second(i64::try_from(seconds).unwrap_or(i64::MAX)) else {
        return "?".into();
    };
    t.to_zoned(jiff::tz::TimeZone::system()).strftime("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn answer(tag: &str, pre: bool) -> String {
        serde_json::json!({
            "tag_name": tag,
            "html_url": format!("https://github.com/jadzziaa/moonglow-toolset/releases/tag/{tag}"),
            "draft": false,
            "prerelease": pre,
        })
        .to_string()
    }

    #[test]
    fn a_check_says_newer_newest_or_why_it_failed() {
        let now = v("2.4.0");
        let api = Api::default();
        let of = |a, now: &Version| Outcome::of(a, now, &api, false);
        let newer = of(Ok(answer("v2.10.0", false)), &now);
        assert!(matches!(newer, Outcome::Newer(r) if r.version == v("2.10.0")));
        assert_eq!(of(Ok(answer("v2.4.0", false)), &now), Outcome::Newest);
        assert_eq!(of(Ok(answer("v2.3.1", false)), &now), Outcome::Newest);
        assert_eq!(of(Ok(answer("v2.5.0", true)), &now), Outcome::Newest);
        assert_eq!(of(Err("offline".into()), &now), Outcome::Failed("offline".into()));
        // This copy a pre-release of the version out: the release is newer.
        assert!(matches!(of(Ok(answer("v2.5.0", false)), &v("2.5.0-rc.1")), Outcome::Newer(_)));
        // With Include pre-releases, the list's newest.
        let list = format!("[{},{}]", answer("v2.5.0-rc.2", true), answer("v2.4.0", false));
        let pre = Outcome::of(Ok(list.clone()), &now, &api, true);
        assert!(matches!(pre, Outcome::Newer(r) if r.version == v("2.5.0-rc.2")));
        assert_eq!(Outcome::of(Ok(list), &v("2.5.0-rc.2"), &api, true), Outcome::Newest);
    }

    #[test]
    fn the_automatic_check_asks_once_a_day() {
        assert!(due(None, 1_000));
        assert!(!due(Some(1_000), 1_000));
        assert!(!due(Some(1_000), 1_000 + DAY - 1));
        assert!(due(Some(1_000), 1_000 + DAY));
        // The clock set back: not a day's wait from a time to come.
        assert!(due(Some(1_000 + DAY * 10), 1_000));
    }

    #[test]
    fn a_skipped_version_is_not_told_again_but_a_later_one_is() {
        let release = |t: &str| Release {
            version: v(t),
            tag: format!("v{t}"),
            page: RELEASES_PAGE.into(),
            notes: String::new(),
            assets: Vec::new(),
        };
        assert!(reminded(&release("2.5.0"), None));
        assert!(!reminded(&release("2.5.0"), Some("2.5.0")));
        assert!(!reminded(&release("2.4.9"), Some("2.5.0")));
        assert!(reminded(&release("2.5.1"), Some("2.5.0")));
        // (A skipped version that isn't one: told.)
        assert!(reminded(&release("2.5.0"), Some("?")));
        // A pre-release skipped, then its release told.
        assert!(!reminded(&release("2.5.0-rc.1"), Some("2.5.0-rc.1")));
        assert!(reminded(&release("2.5.0"), Some("2.5.0-rc.1")));
    }
}
