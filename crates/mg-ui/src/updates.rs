//! Help › Check for Updates… and the daily check for a new release
//! (GitHub issue 28, its small part): GitHub is asked for the latest
//! release, and a newer one is said with a link to its page. Nothing is
//! downloaded or installed (`docs/deferred.md`).
//!
//! The request itself is the application's: [`Updates::fetch`] is `None`
//! unless the desktop app gives one (`apps/moonglow`), so the command-line
//! tool and the tests never reach the network.

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::Moonglow;

/// GitHub's latest release of Moonglow (REST API): drafts and pre-releases
/// are never it.
pub const LATEST_API: &str =
    "https://api.github.com/repos/jadzziaa/moonglow-toolset/releases/latest";

/// The releases' page, where a release's own page isn't known.
pub const RELEASES_PAGE: &str = "https://github.com/jadzziaa/moonglow-toolset/releases/latest";

/// How long the automatic check waits between two requests, seconds.
pub const DAY: u64 = 24 * 60 * 60;

/// Asks GitHub for [`LATEST_API`]'s body (the release, as JSON), or says
/// why it couldn't.
pub type Fetch = Arc<dyn Fn() -> Result<String, String> + Send + Sync>;

/// Where a request on its thread sends its answer.
type Answer = Receiver<Result<String, String>>;

/// A release's version: `major.minor.patch`, compared as numbers (2.4.0 is
/// before 2.10.0), a pre-release (`2.5.0-rc.1`) before its release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// Has a pre-release part (`-rc.1`): never offered.
    pub pre: bool,
}

impl Version {
    /// `2.4.0`, `v2.4.0`, `2.5.0-rc.1`, `2.4.0+build` (the build part is
    /// passed over, as semver has it).
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let text = text.strip_prefix(['v', 'V']).unwrap_or(text);
        let text = text.split_once('+').map_or(text, |(v, _)| v);
        let (numbers, pre) = match text.split_once('-') {
            Some((n, p)) if !p.is_empty() => (n, true),
            Some(_) => return None,
            None => (text, false),
        };
        let mut parts = numbers.split('.');
        let mut number = || {
            let p = parts.next()?;
            (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())).then(|| p.parse().ok())?
        };
        let (major, minor, patch) = (number()?, number()?, number()?);
        parts.next().is_none().then_some(Version { major, minor, patch, pre })
    }

    /// This program's version.
    pub fn current() -> Version {
        Version::parse(mg_core::release::version()).expect("the release is semver")
    }

    fn key(&self) -> (u64, u64, u64, bool) {
        (self.major, self.minor, self.patch, !self.pre)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if self.pre {
            f.write_str(" (pre-release)")?;
        }
        Ok(())
    }
}

/// A release GitHub lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// Its page on GitHub (opened in the browser).
    pub page: String,
}

/// The release in GitHub's answer for the latest one; `None` when it is a
/// draft or a pre-release (which never count), or its tag isn't a version.
pub fn parse_latest(json: &str) -> Result<Option<Release>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("GitHub's answer isn't JSON: {e}"))?;
    let tag = value
        .get("tag_name")
        .and_then(serde_json::Value::as_str)
        .ok_or("GitHub's answer names no release")?;
    let flag = |name| value.get(name).and_then(serde_json::Value::as_bool).unwrap_or(false);
    if flag("draft") || flag("prerelease") {
        return Ok(None);
    }
    let Some(version) = Version::parse(tag).filter(|v| !v.pre) else { return Ok(None) };
    // (Only a page of Moonglow's own releases is opened.)
    let page = value
        .get("html_url")
        .and_then(serde_json::Value::as_str)
        .filter(|u| u.starts_with("https://github.com/jadzziaa/moonglow-toolset/releases/"))
        .unwrap_or(RELEASES_PAGE)
        .to_string();
    Ok(Some(Release { version, page }))
}

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
    /// What a fetch's answer says, against `current`.
    pub fn of(answer: Result<String, String>, current: Version) -> Outcome {
        match answer.and_then(|json| parse_latest(&json)) {
            Ok(Some(r)) if r.version > current => Outcome::Newer(r),
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

/// A package manager that installed this copy, and updates it: Moonglow
/// doesn't check by itself there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installer {
    Flatpak,
    Scoop,
    Nix,
}

impl Installer {
    pub fn name(self) -> &'static str {
        match self {
            Installer::Flatpak => "Flatpak",
            Installer::Scoop => "Scoop",
            Installer::Nix => "Nix",
        }
    }

    /// How to update through it.
    pub fn how(self) -> &'static str {
        match self {
            Installer::Flatpak => "flatpak update io.github.moonglow_toolset.Moonglow",
            Installer::Scoop => "scoop update moonglow",
            Installer::Nix => "update the flake or channel it comes from",
        }
    }

    /// The installer of a program at `exe`, with `FLATPAK_ID` set to
    /// `flatpak_id` and `/.flatpak-info` there or not.
    pub fn detect(flatpak_id: Option<&str>, flatpak_info: bool, exe: &Path) -> Option<Installer> {
        if flatpak_id.is_some_and(|id| !id.is_empty()) || flatpak_info {
            return Some(Installer::Flatpak);
        }
        // (Either separator, any case: Scoop is on Windows, the tests not.)
        let path = exe.to_string_lossy().replace('\\', "/").to_lowercase();
        if path.contains("/scoop/apps/") {
            Some(Installer::Scoop)
        } else if path.starts_with("/nix/store/") {
            Some(Installer::Nix)
        } else {
            None
        }
    }

    /// This program's installer.
    pub fn here() -> Option<Installer> {
        let exe = std::env::current_exe().unwrap_or_default();
        let id = std::env::var("FLATPAK_ID").ok();
        Installer::detect(id.as_deref(), Path::new("/.flatpak-info").exists(), &exe)
    }
}

/// The checks' state.
#[derive(Default)]
pub struct Updates {
    /// Asks GitHub (the desktop app's); `None`: never asked.
    pub fetch: Option<Fetch>,
    /// The package manager this copy was installed by, if one was.
    pub installer: Option<Installer>,
    /// The Check for Updates window is open.
    pub window: bool,
    /// What its check found (`None` while asking).
    pub report: Option<Outcome>,
    /// A newer release the automatic check found: the notice under the
    /// toolbar, until dismissed.
    pub notice: Option<Release>,
    /// When the automatic check last looked whether it was due.
    looked: Option<std::time::Instant>,
    /// The request under way (on its thread): whether the automatic
    /// check and the window wait for it.
    pending: Option<(Answer, bool, bool)>,
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates")
            .field("fetch", &self.fetch.is_some())
            .field("installer", &self.installer)
            .field("report", &self.report)
            .field("notice", &self.notice)
            .finish_non_exhaustive()
    }
}

impl Updates {
    /// A check is under way.
    pub fn checking(&self) -> bool {
        self.pending.is_some()
    }

    /// The automatic check looks whether it is due at the next frame, not
    /// within the hour (Options were applied: it may have been switched
    /// on).
    pub(crate) fn look_again(&mut self) {
        self.looked = None;
    }
}

/// How often the automatic check looks whether a day has gone by.
const LOOK_EVERY: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// Help › Check for Updates…: the window, and a request.
pub fn check_now(app: &mut Moonglow) {
    app.updates.window = true;
    app.updates.report = None;
    // (The Flatpak's sandbox has no network, and Flatpak updates it: the
    // window says how, and nothing is asked.)
    if app.updates.installer == Some(Installer::Flatpak) {
        return;
    }
    start(app, false, true);
}

/// Starts a request (or joins the one under way).
fn start(app: &mut Moonglow, auto: bool, manual: bool) {
    if let Some((_, a, m)) = &mut app.updates.pending {
        *a |= auto;
        *m |= manual;
        return;
    }
    let Some(fetch) = app.updates.fetch.clone() else {
        if manual {
            app.updates.report =
                Some(Outcome::Failed("This copy of Moonglow doesn't check for updates".into()));
        }
        return;
    };
    if !app.background_jobs {
        // (Tests: at once.)
        finish(app, fetch(), auto, manual);
        return;
    }
    let (send, receive) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new().name("update-check".into()).spawn(move || {
        let _ = send.send(fetch());
    });
    match spawned {
        Ok(_) => app.updates.pending = Some((receive, auto, manual)),
        Err(e) => finish(app, Err(format!("no thread to ask on: {e}")), auto, manual),
    }
}

/// What a request found, for the window and the notice.
fn finish(app: &mut Moonglow, answer: Result<String, String>, auto: bool, manual: bool) {
    let outcome = Outcome::of(answer, Version::current());
    crate::trace::note(format!("update check: {outcome:?}"));
    if auto
        && let Outcome::Newer(r) = &outcome
        && reminded(r, app.settings.update_skipped.as_deref())
    {
        app.updates.notice = Some(r.clone());
    }
    if manual {
        app.updates.report = Some(outcome);
    }
}

/// Each frame: the automatic check when it is due, and the answer to a
/// request when it has come.
pub(crate) fn frame(app: &mut Moonglow, ctx: &egui::Context) {
    let looked = app.updates.looked.is_some_and(|t| t.elapsed() < LOOK_EVERY);
    if !looked && app.updates.fetch.is_some() {
        app.updates.looked = Some(std::time::Instant::now());
        let now = now();
        if app.updates.installer.is_none()
            && !app.settings.no_update_check
            && due(app.settings.update_checked, now)
        {
            // (Noted as it starts: a day goes by between two requests,
            // whatever they found.)
            app.settings.update_checked = Some(now);
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

/// The notice of a newer release, under the toolbar, while there is one.
pub(crate) fn notice_bar(app: &mut Moonglow, ui: &mut egui::Ui) {
    let Some(release) = app.updates.notice.clone() else { return };
    let mut close = false;
    egui::Panel::top("update-notice").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("ℹ").color(ui.visuals().hyperlink_color));
            ui.label(format!(
                "Moonglow {} is out (you have {}).",
                release.version,
                Version::current()
            ));
            ui.hyperlink_to("Release Page", &release.page)
                .on_hover_text("What is new in it, and its downloads (opens the browser)");
            if ui
                .button("Don't remind me about this version")
                .on_hover_text("No notice for this version; a later one is told again")
                .clicked()
            {
                app.settings.update_skipped = Some(release.version.to_string());
                close = true;
            }
            if ui.small_button("×").on_hover_text("Close (a later check tells again)").clicked() {
                close = true;
            }
        });
    });
    if close {
        app.updates.notice = None;
    }
}

/// What the window says of a check that found `outcome`.
fn said(ui: &mut egui::Ui, outcome: &Outcome) {
    match outcome {
        Outcome::Newest => {
            ui.label(format!("Moonglow {} is the newest.", Version::current()));
        }
        Outcome::Newer(r) => {
            ui.label(format!("Moonglow {} is out (you have {}).", r.version, Version::current()));
            ui.hyperlink_to("Open the release page", &r.page)
                .on_hover_text("What is new in it, and its downloads (opens the browser)");
        }
        Outcome::Failed(why) => {
            ui.colored_label(ui.visuals().error_fg_color, format!("Couldn't ask GitHub: {why}."));
        }
    }
}

/// Help › Check for Updates…, while open.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    if !app.updates.window {
        return;
    }
    let title = "Check for Updates";
    let mut open = true;
    let (mut again, mut close) = (false, false);
    egui::Window::new(title)
        .open(crate::widgets::open_unless_escape(ctx, title, &mut open))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            let flatpak = app.updates.installer == Some(Installer::Flatpak);
            match &app.updates.report {
                None if flatpak => {}
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Asking GitHub for the latest release…");
                    });
                }
                Some(outcome) => said(ui, outcome),
            }
            if let Some(i) = app.updates.installer {
                ui.add_space(4.0);
                ui.label(format!("This copy is updated by {}: {}.", i.name(), i.how()));
            }
            if !flatpak {
                ui.add_space(6.0);
                ui.weak(
                    "Sent: one request to GitHub for the latest release's version and page; \
                     nothing about you, your modules or this computer.",
                );
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let idle = !app.updates.checking();
                if !flatpak && ui.add_enabled(idle, egui::Button::new("Check Again")).clicked() {
                    again = true;
                }
                if ui.button("Close").clicked() {
                    close = true;
                }
            });
        });
    if again {
        check_now(app);
    }
    if !open || close {
        app.updates.window = false;
    }
}

/// Options › Updates.
pub(crate) fn options_page(
    ui: &mut egui::Ui,
    check: &mut bool,
    skipped: &mut Option<String>,
    installer: Option<Installer>,
    checked: Option<u64>,
) {
    let managed = installer.is_some();
    let mut shown = *check && !managed;
    let response = ui
        .add_enabled(!managed, egui::Checkbox::new(&mut shown, "Check for updates automatically"))
        .on_hover_text(
            "Once a day at most, in the background, ask GitHub whether a newer Moonglow is out, \
             and say so under the toolbar",
        );
    if response.changed() {
        *check = shown;
    }
    ui.weak(
        "Sent: one request to GitHub (api.github.com) a day at most, for the latest release's \
         version and page: nothing about you, your modules or this computer. Offline, or when \
         GitHub doesn't answer, nothing is said. Help › Check for Updates… asks at once.",
    );
    if let Some(i) = installer {
        ui.add_space(4.0);
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "This copy is updated by {}: {}. Moonglow doesn't check by itself; Help › \
                 Check for Updates… still asks.",
                i.name(),
                i.how()
            ),
        );
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Last checked");
        ui.label(checked.map_or_else(|| "Not yet".to_string(), local_time));
    });
    if let Some(version) = skipped.clone() {
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Not reminded about");
            ui.label(format!("Moonglow {version}"));
            if ui
                .button("Remind Me Again")
                .on_hover_text("The notice tells of this version again")
                .clicked()
            {
                *skipped = None;
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

    #[test]
    fn versions_compare_as_numbers() {
        assert!(v("2.4.0") < v("2.10.0"));
        assert!(v("2.9.9") < v("3.0.0"));
        assert!(v("v2.4.1") > v("2.4.0"));
        assert_eq!(v("v2.4.0"), v("2.4.0"));
        assert_eq!(v("2.4.0+linux"), v("2.4.0"));
        // A pre-release comes before its release.
        assert!(v("2.5.0-rc.1") < v("2.5.0"));
        assert!(v("2.5.0-rc.1") > v("2.4.0"));
        assert!(v("2.5.0-rc.1").pre && !v("2.5.0").pre);
        for bad in ["", "2.4", "2.4.0.1", "two.4.0", "2..0", "2.4.0-", "+2.4.0", "2.-4.0"] {
            assert_eq!(Version::parse(bad), None, "{bad:?}");
        }
        assert_eq!(Version::current(), v(mg_core::release::version()));
        assert_eq!(v("2.10.3").to_string(), "2.10.3");
    }

    fn answer(tag: &str, draft: bool, pre: bool) -> String {
        serde_json::json!({
            "tag_name": tag,
            "name": format!("Moonglow Toolset {tag}"),
            "html_url": format!("https://github.com/jadzziaa/moonglow-toolset/releases/tag/{tag}"),
            "draft": draft,
            "prerelease": pre,
            "assets": [],
        })
        .to_string()
    }

    #[test]
    fn the_latest_release_is_read_and_drafts_and_pre_releases_never_count() {
        let latest = parse_latest(&answer("v2.10.0", false, false)).unwrap().unwrap();
        assert_eq!(latest.version, v("2.10.0"));
        assert_eq!(
            latest.page,
            "https://github.com/jadzziaa/moonglow-toolset/releases/tag/v2.10.0"
        );
        assert_eq!(parse_latest(&answer("v2.5.0", true, false)).unwrap(), None);
        assert_eq!(parse_latest(&answer("v2.5.0", false, true)).unwrap(), None);
        // (A pre-release's tag, though not marked one.)
        assert_eq!(parse_latest(&answer("v2.5.0-beta.2", false, false)).unwrap(), None);
        assert_eq!(parse_latest(&answer("nightly", false, false)).unwrap(), None);
        // A page elsewhere isn't opened: the releases' page is.
        let elsewhere = r#"{"tag_name":"v3.0.0","html_url":"https://example.com/x"}"#;
        assert_eq!(parse_latest(elsewhere).unwrap().unwrap().page, RELEASES_PAGE);
        assert!(parse_latest("<html>").is_err());
        assert!(parse_latest(r#"{"message":"Not Found"}"#).is_err());
    }

    #[test]
    fn a_check_says_newer_newest_or_why_it_failed() {
        let now = v("2.4.0");
        let newer = Outcome::of(Ok(answer("v2.10.0", false, false)), now);
        assert!(matches!(newer, Outcome::Newer(r) if r.version == v("2.10.0")));
        assert_eq!(Outcome::of(Ok(answer("v2.4.0", false, false)), now), Outcome::Newest);
        assert_eq!(Outcome::of(Ok(answer("v2.3.1", false, false)), now), Outcome::Newest);
        assert_eq!(Outcome::of(Ok(answer("v2.5.0", false, true)), now), Outcome::Newest);
        assert_eq!(Outcome::of(Err("offline".into()), now), Outcome::Failed("offline".into()));
        // This copy a pre-release of the version out: the release is newer.
        let pre = v("2.5.0-rc.1");
        assert!(matches!(Outcome::of(Ok(answer("v2.5.0", false, false)), pre), Outcome::Newer(_)));
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
        let release = |t: &str| Release { version: v(t), page: RELEASES_PAGE.into() };
        assert!(reminded(&release("2.5.0"), None));
        assert!(!reminded(&release("2.5.0"), Some("2.5.0")));
        assert!(!reminded(&release("2.4.9"), Some("2.5.0")));
        assert!(reminded(&release("2.5.1"), Some("2.5.0")));
        // (A skipped version that isn't one: told.)
        assert!(reminded(&release("2.5.0"), Some("?")));
    }

    #[test]
    fn package_managed_installs_are_known_by_path_and_environment() {
        let detect = Installer::detect;
        let plain = Path::new("/home/me/Applications/Moonglow.AppImage");
        assert_eq!(detect(None, false, plain), None);
        assert_eq!(
            detect(Some("io.github.moonglow_toolset.Moonglow"), false, plain),
            Some(Installer::Flatpak)
        );
        assert_eq!(detect(None, true, Path::new("/app/bin/moonglow")), Some(Installer::Flatpak));
        assert_eq!(detect(Some(""), false, plain), None);
        let scoop = Path::new(r"C:\Users\me\scoop\apps\moonglow\2.4.0\moonglow.exe");
        assert_eq!(detect(None, false, scoop), Some(Installer::Scoop));
        let global = Path::new(r"C:\ProgramData\Scoop\Apps\moonglow\current\moonglow.exe");
        assert_eq!(detect(None, false, global), Some(Installer::Scoop));
        let nix = Path::new("/nix/store/abc123-moonglow-2.4.0/bin/moonglow");
        assert_eq!(detect(None, false, nix), Some(Installer::Nix));
        // The installer and the zip, the macOS app: none.
        let windows = Path::new(r"C:\Users\me\AppData\Local\Programs\Moonglow\moonglow.exe");
        assert_eq!(detect(None, false, windows), None);
        let mac = Path::new("/Applications/Moonglow Toolset.app/Contents/MacOS/moonglow");
        assert_eq!(detect(None, false, mac), None);
        assert_eq!(detect(None, false, Path::new("/home/me/nix/store/moonglow")), None);
    }
}
