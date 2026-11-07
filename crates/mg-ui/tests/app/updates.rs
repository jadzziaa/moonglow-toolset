//! Help › Check for Updates…, the daily check and its notice, Options ›
//! Updates, and installing an update from the app: the releases from a
//! server of the test's own on 127.0.0.1 (`mg_update::serve`) through the
//! app's client, or answers made up; never GitHub.

use mg_update::{Installer, Kind};

use crate::common::*;

/// GitHub's answer for the latest release (as `/releases/latest` gives
/// it), tagged `tag`.
fn release_answer(tag: &str) -> String {
    serde_json::json!({
        "tag_name": tag,
        "html_url": format!("https://github.com/jadzziaa/moonglow-toolset/releases/tag/{tag}"),
        "draft": false,
        "prerelease": false,
    })
    .to_string()
}

/// Answers to every request: `answer` (never the network).
fn answering(answer: Result<String, String>) -> std::sync::Arc<dyn mg_update::Http> {
    mg_update::http::from_fn(move |_| answer.clone().map(String::into_bytes))
}

/// An app whose checks for updates get `answer` (never the network), and
/// how many times it was asked.
fn app_with_releases(
    answer: Result<String, String>,
) -> (Moonglow, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let asked = std::sync::Arc::new(AtomicUsize::new(0));
    let mut app = app_with(Vec::new());
    let counted = asked.clone();
    app.updates.http = Some(mg_update::http::from_fn(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
        answer.clone().map(String::into_bytes)
    }));
    (app, asked)
}

/// Help › Check for Updates… says a newer release is out, with its page,
/// or that this one is the newest, or why GitHub couldn't be asked; a
/// copy a package manager updates says how.
#[test]
fn check_for_updates_says_what_is_newest() {
    use std::sync::atomic::Ordering;
    let current = mg_core::release::version();
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    // (No automatic check: switched off.)
    app.settings.no_update_check = true;
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 0, "not asked by itself");
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label_contains("Check for Updates").click();
    h.run();
    h.get_by_label(&format!("Moonglow 99.0.0 is out (you have {current})."));
    h.get_by_label("Open the release page");
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    assert!(h.state().updates.notice.is_none(), "the notice is the automatic check's");
    // The newest.
    h.state_mut().updates.http = Some(answering(Ok(release_answer(&format!("v{current}")))));
    h.get_by_label("Check Again").click();
    h.run();
    h.get_by_label(&format!("Moonglow {current} is the newest."));
    // Offline.
    h.state_mut().updates.http = Some(answering(Err("not connected to the Internet".into())));
    h.get_by_label("Check Again").click();
    h.run();
    h.get_by_label("Couldn't ask GitHub: not connected to the Internet.");
    // Installed by Scoop: still asked, and said how it is updated.
    h.state_mut().updates.kind = Kind::Managed(Installer::Scoop);
    h.state_mut().updates.http = Some(answering(Ok(release_answer("v99.0.0"))));
    // (Drawn so first: the window is taller, centered again a frame
    // later, its button elsewhere.)
    h.run_steps(2);
    h.get_by_label("Check Again").click();
    h.run();
    h.get_by_label(&format!("Moonglow 99.0.0 is out (you have {current})."));
    h.get_by_label("This copy is updated by Scoop: scoop update moonglow.");
    h.get_by_label("Close").click();
    h.run();
    assert!(h.query_by_label("Check Again").is_none());
    // And from Help › About.
    h.state_mut().about = true;
    h.run();
    h.get_by_label("Check for Updates…").click();
    h.run();
    h.get_by_label("Check Again");
}

/// Closes the Check for Updates window the automatic check opened on a
/// newer release: the notice under the toolbar stays.
fn close_window(h: &mut Harness<'_, Moonglow>) {
    assert!(h.state().updates.window, "the window opened on the release");
    h.get_by_label("Close").click();
    h.run();
}

/// A newer release the automatic check finds at start opens the Check for
/// Updates window on it, asking (Carcerian's ask on Discord, 2026-11-06:
/// the line under the toolbar went unseen), with Always install updates,
/// which is Options › Updates › Install updates automatically; with that
/// on, it is downloaded without the window. A skipped version opens
/// nothing.
#[test]
fn a_newer_release_found_at_start_opens_the_window_asking() {
    let current = mg_core::release::version();
    let told = format!("Moonglow 99.0.0 is out (you have {current}).");
    let dir = mg_testkit::scratch_dir("ui-update-window");
    let key = mg_testkit::signing::TestKey::new();
    let server = serve_release(&key, "v99.0.0", false, b"Moonglow 99.0.0's AppImage");
    let (app, _) = app_updating(&dir, &server, Some(key.public()));
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert!(h.state().updates.window, "the window, open");
    assert_eq!(h.get_all_by_label(&told).count(), 1, "told once: the notice waits");
    h.get_by_label("Install Now");
    h.get_by_label("Later");
    assert!(!h.state().settings.update_auto_install);
    // Closed, the daily check while Moonglow runs doesn't open it again.
    close_window(&mut h);
    let day_ago = mg_ui::updates::now() - mg_ui::updates::DAY - 60;
    h.state_mut().settings.update_checked = Some(day_ago);
    h.state_mut().updates.look_again();
    h.run_steps(3);
    assert!(h.state().settings.update_checked > Some(day_ago), "asked again");
    assert!(!h.state().updates.window, "not opened again in this run");
    h.get_by_label(&told);
    // What's New… opens it; Always install updates gets this release too.
    h.get_by_label("What's New…").click();
    h.run_steps(2);
    assert!(h.state().updates.getting.is_none());
    h.get_by_label("Always install updates").click();
    h.run_steps(2);
    assert!(h.state().settings.update_auto_install, "Options › Updates' switch");
    let getting = h.state().updates.getting.as_ref().expect("this release, downloading");
    assert!(!getting.now, "installed as Moonglow closes, as Later");
    h.get_by_label("Close").click();
    h.run();
    assert!(h.query_by_label("Always install updates").is_none(), "the window's alone");

    // Always: the next start downloads it, without the window.
    let settings = h.state().settings.clone();
    let (mut app, _) = app_updating(&dir, &server, Some(key.public()));
    app.settings = settings;
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(3);
    assert!(!h.state().updates.window, "not asked");
    assert!(h.state().updates.getting.is_some(), "downloading");

    // Skipped: nothing opens.
    let (mut app, _) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings.update_skipped = Some("99.0.0".into());
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert!(!h.state().updates.window);
}

/// The automatic check, due, tells of a newer release under the toolbar
/// at start; Don't remind me about this version keeps it from telling of
/// that one again (a later one is told), and × closes it for now.
#[test]
fn the_update_notice_appears_and_is_dismissed() {
    use std::sync::atomic::Ordering;
    let current = mg_core::release::version();
    let told = format!("Moonglow 99.0.0 is out (you have {current}).");
    let (app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    assert_eq!(app.settings.update_checked, None);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    // (The window opens on it; closed, the notice under the toolbar.)
    h.get_by_label(&told);
    close_window(&mut h);
    h.get_by_label(&told);
    h.get_by_label("Release Page");
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    let checked = h.state().settings.update_checked.expect("the check's time is kept");
    assert!(checked.abs_diff(mg_ui::updates::now()) < 60);
    // (Once: frames go on without asking again.)
    h.run_steps(5);
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    h.get_by_label("Don't remind me about this version").click();
    h.run();
    assert!(h.query_by_label(&told).is_none());
    assert_eq!(h.state().settings.update_skipped.as_deref(), Some("99.0.0"));

    // The next day: the skipped version isn't told; a later one is.
    let mut settings = h.state().settings.clone();
    settings.update_checked = Some(checked - mg_ui::updates::DAY);
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings = settings.clone();
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 1, "asked, a day having gone by");
    assert!(h.query_by_label(&told).is_none(), "the skipped version isn't told again");
    let (mut app, _) = app_with_releases(Ok(release_answer("v99.1.0")));
    app.settings = settings;
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    close_window(&mut h);
    h.get_by_label(&format!("Moonglow 99.1.0 is out (you have {current})."));
    h.get_by_label("×").click();
    h.run();
    assert!(h.state().updates.notice.is_none());
    assert_eq!(h.state().settings.update_skipped.as_deref(), Some("99.0.0"));

    // Offline: nothing said, in the log or elsewhere.
    let (app, asked) = app_with_releases(Err("not connected to the Internet".into()));
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    assert!(h.state().updates.notice.is_none());
    assert!(!h.state().log.entries.iter().any(|(_, m)| m.contains("GitHub")));
}

/// The automatic check asks at each start, not only a day after the last
/// one: a release out since that day's check is told when Moonglow starts
/// again (Carcerian's report on 3.1.0: the notice never came, Check for
/// Updates… found the release). While it runs, a day between two asks.
#[test]
fn the_update_check_asks_at_each_start() {
    use std::sync::atomic::Ordering;
    let current = mg_core::release::version();
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    // (Checked an hour ago, when nothing newer was out.)
    app.settings.update_checked = Some(mg_ui::updates::now() - 60 * 60);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 1, "asked at start, a day not gone by");
    h.get_by_label(&format!("Moonglow 99.0.0 is out (you have {current})."));
    // (Looked again within the run, before a day is out: not asked.)
    h.state_mut().updates.look_again();
    h.run_steps(3);
    assert_eq!(asked.load(Ordering::SeqCst), 1, "once a day while it runs");
}

/// Options › Updates › Check for updates automatically, off: no request
/// at start; and none where a package manager updates Moonglow, whose
/// page says so.
#[test]
fn the_updates_option_turns_automatic_checks_off() {
    use std::sync::atomic::Ordering;
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings.update_checked = Some(mg_ui::updates::now());
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 1, "asked at start, on");
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Updates").click();
    h.run();
    h.get_by_label("Check for updates automatically").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().settings.no_update_check);
    assert_eq!(asked.load(Ordering::SeqCst), 1, "not again, switched off");

    // At the next start, a day later: not asked.
    let mut settings = h.state().settings.clone();
    settings.update_checked = None;
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings = settings.clone();
    let mut h = window_harness(app);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 0, "switched off");
    assert!(h.state().updates.notice.is_none());

    // On, but installed by Flatpak: not asked, and the page says why.
    settings.no_update_check = false;
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings = settings;
    app.updates.kind = Kind::Managed(Installer::Flatpak);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 0, "a package manager updates it");
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Updates").click();
    h.run();
    h.get_by_label_contains("This copy is updated by Flatpak");
    assert!(h.get_by_label("Check for updates automatically").accesskit_node().is_disabled());
    // Help › Check for Updates… in the Flatpak, whose sandbox has no
    // network: nothing asked, only how Flatpak updates it.
    h.get_by_label("Cancel").click();
    h.run();
    mg_ui::updates::check_now(h.state_mut());
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 0, "the Flatpak asks nothing");
    h.get_by_label_contains("This copy is updated by Flatpak: flatpak update");
    assert!(h.query_by_label("Check Again").is_none());
    assert!(h.query_by_label_contains("Couldn't ask GitHub").is_none());
}

/// As the desktop app runs it (jobs in the background): the request goes
/// on its own thread, the window asks while it waits, and the answer is
/// shown when it comes; turned on in Options, the automatic check asks
/// at once.
#[test]
fn the_update_check_asks_on_its_own_thread() {
    use std::sync::atomic::Ordering;
    let current = mg_core::release::version();
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.background_jobs = true;
    app.settings.no_update_check = true;
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(2);
    mg_ui::updates::check_now(h.state_mut());
    h.run_steps(1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while h.state().updates.report.is_none() {
        assert!(std::time::Instant::now() < deadline, "no answer");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.run_steps(1);
    }
    h.run_steps(2);
    h.get_by_label(&format!("Moonglow 99.0.0 is out (you have {current})."));
    assert!(h.query_by_label("Asking GitHub for the latest release…").is_none());
    assert!(!h.state().updates.checking());
    assert!(h.state().updates.notice.is_none(), "the window's check, not the automatic one");
    assert_eq!(asked.load(Ordering::SeqCst), 1);

    // Switched on in Options: asked at the next frames, not in an hour.
    h.state_mut().updates.window = false;
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run_steps(2);
    h.get_by_label("Updates").click();
    h.run_steps(2);
    h.get_by_label("Check for updates automatically").click();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(1);
    assert!(!h.state().settings.no_update_check);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while h.state().updates.notice.is_none() {
        assert!(std::time::Instant::now() < deadline, "no notice");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.run_steps(1);
    }
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

/// A release on a server of the test's own (127.0.0.1), signed by `signer`
/// (as `packaging/sign-release.sh` signs one): its notes, this system's
/// AppImage and the checksums. `/latest` is it; `/releases` lists it.
fn serve_release(
    signer: &mg_testkit::signing::TestKey,
    tag: &str,
    prerelease: bool,
    image: &[u8],
) -> mg_update::serve::Server {
    use mg_update::serve::{Published, Reply, Server, signed};
    let server = Server::start();
    let v = tag.trim_start_matches('v');
    let name = format!("Moonglow-{v}-{}.AppImage", std::env::consts::ARCH);
    let published = Published {
        tag: tag.into(),
        prerelease,
        notes: "## What's new\n\n- Updates install from the app, verified first.".into(),
        files: signed(signer, tag, &[(name.as_str(), image)]),
    };
    published.put_files(&server);
    let json = published.json(&server);
    if !prerelease {
        server.put("/latest", Reply::Body(json.to_string().into_bytes()));
    }
    server.put("/releases", Reply::Body(serde_json::json!([json]).to_string().into_bytes()));
    server
}

/// An app that updates the scratch AppImage in `dir` (never the owner's
/// install) from `server`, its downloads checked against `key`: through
/// the app's own client, plain HTTP allowed for 127.0.0.1.
fn app_updating(
    dir: &std::path::Path,
    server: &mg_update::serve::Server,
    key: Option<mg_update::PublicKey>,
) -> (Moonglow, std::path::PathBuf) {
    let image = dir.join("Moonglow-3.2.0-x86_64.AppImage");
    std::fs::write(&image, b"the AppImage running").unwrap();
    let mut app = app_with(Vec::new());
    app.updates.http = Some(std::sync::Arc::new(mg_update::http::Client::plain_http_for_tests()));
    app.updates.api = mg_update::Api::at(&server.base);
    app.updates.key = key;
    app.updates.kind = Kind::AppImage { image: image.clone(), arch: std::env::consts::ARCH.into() };
    app.updates.cache = Some(dir.join("cache"));
    (app, image)
}

/// The files of `dir` whose names an update gives them.
fn update_files(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with(mg_update::stage::PREFIX))
        .collect()
}

/// The notice of a newer release → its notes in the window → Install Now
/// → downloaded and verified → the AppImage replaced (a scratch copy) →
/// Restart Now asked, and Moonglow restarts as the new image.
#[test]
fn an_update_is_installed_from_the_notice_and_a_restart_asked() {
    let dir = mg_testkit::scratch_dir("ui-update-install");
    let key = mg_testkit::signing::TestKey::new();
    let server = serve_release(&key, "v99.0.0", false, b"Moonglow 99.0.0's AppImage");
    let (app, image) = app_updating(&dir, &server, Some(key.public()));
    let current = mg_core::release::version();
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    close_window(&mut h);
    h.get_by_label(&format!("Moonglow 99.0.0 is out (you have {current})."));
    h.get_by_label("What's New…").click();
    h.run_steps(2);
    // The notes, in the window.
    h.get_by_label_contains("Updates install from the app, verified first.");
    h.get_by_label_contains("Installs it in place of the AppImage");
    assert_eq!(std::fs::read(&image).unwrap(), b"the AppImage running", "not yet");
    h.get_all_by_label("Install Now").last().unwrap().click();
    h.run_steps(2);
    // Verified and replaced, in place: the image's path kept, nothing left.
    assert_eq!(std::fs::read(&image).unwrap(), b"Moonglow 99.0.0's AppImage");
    assert!(update_files(&dir).is_empty(), "{:?}", update_files(&dir));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&image).unwrap().permissions().mode();
        assert_eq!(mode & 0o700, 0o700, "runnable: {mode:o}");
    }
    let asked = server.asked();
    for path in ["/latest", "/download/v99.0.0/SHA256SUMS", "/download/v99.0.0/SHA256SUMS.minisig"]
    {
        assert!(asked.iter().any(|p| p == path), "{path} in {asked:?}");
    }
    assert!(
        h.state().log.entries.iter().any(|(_, m)| m.contains("checksum and signature verified"))
    );
    h.get_all_by_label("Moonglow 99.0.0 is installed: it runs from the next start.")
        .next()
        .unwrap();
    h.get_by_label("Restart Moonglow now?");
    // (Drawn so first: the window is taller, centered again a frame
    // later, its button elsewhere.)
    h.run_steps(2);
    h.get_all_by_label("Restart Now").last().unwrap().click();
    h.run_steps(2);
    assert!(h.state().quit_requested);
    let again = h.state().relaunch.clone().expect("to start again");
    assert_eq!(again.start, Some(mg_update::Restart::Exe(image.clone())), "the new image");
    // (Nothing is left to install as it closes: it is in place.)
    let staged = h.state_mut().updates.at_close().expect("the update");
    let (steps, _) = mg_update::install::plan(&staged, true, &dir.join("cache")).unwrap();
    assert!(steps.is_empty());
}

/// Restart Now with unsaved work asks about it first, as Exit does.
#[test]
fn restart_now_asks_about_unsaved_work_first() {
    let dir = mg_testkit::scratch_dir("ui-update-unsaved");
    let key = mg_testkit::signing::TestKey::new();
    let server = serve_release(&key, "v99.0.0", false, b"new");
    let (mut app, _) = app_updating(&dir, &server, Some(key.public()));
    let path = sample_module(&dir);
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    close_window(&mut h);
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let edit = mg_edit::Edit::SetField {
        key: ifo,
        path: mg_edit::GffPath::root(),
        label: "Mod_Tag".into(),
        value: Some(mg_gff::Value::String(b"CHANGED".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
    h.run();
    h.get_by_label("Install Now").click();
    h.run_steps(2);
    h.get_by_label("Restart Now").click();
    h.run_steps(2);
    assert!(!h.state().quit_requested, "asked first");
    assert!(matches!(h.state().confirm_discard, Some(mg_ui::Action::RestartToUpdate)));
}

/// A release whose checksums are signed by another key than the app's is
/// refused: the reason said, with the release page, the image untouched
/// and nothing of the download left.
#[test]
fn an_update_with_a_bad_signature_is_refused() {
    let dir = mg_testkit::scratch_dir("ui-update-refused");
    let signer = mg_testkit::signing::TestKey::new();
    let server = serve_release(&signer, "v99.0.0", false, b"not ours");
    let ours = mg_testkit::signing::TestKey::new();
    let (app, image) = app_updating(&dir, &server, Some(ours.public()));
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    close_window(&mut h);
    h.get_by_label("Install Now").click();
    h.run_steps(2);
    h.get_by_label(
        "Moonglow 99.0.0 isn't installed: the release is signed with another key than this copy \
         of Moonglow knows.",
    );
    h.get_by_label("Release Page");
    assert_eq!(std::fs::read(&image).unwrap(), b"the AppImage running");
    assert!(update_files(&dir).is_empty(), "{:?}", update_files(&dir));
    assert!(h.query_by_label("Restart Now").is_none());
    assert!(h.state_mut().updates.at_close().is_none(), "nothing to install as it closes");
    // Signed again with the right key: Try Again downloads it from the
    // start, and installs it.
    let name = format!("Moonglow-99.0.0-{}.AppImage", std::env::consts::ARCH);
    let fixed = mg_update::serve::Published {
        tag: "v99.0.0".into(),
        prerelease: false,
        notes: String::new(),
        files: mg_update::serve::signed(&ours, "v99.0.0", &[(name.as_str(), b"ours")]),
    };
    fixed.put_files(&server);
    h.get_by_label("Try Again").click();
    h.run_steps(2);
    assert_eq!(std::fs::read(&image).unwrap(), b"ours");
    h.get_by_label("Moonglow 99.0.0 is installed: it runs from the next start.");

    // A build without the key: the page link alone, and why.
    let (app, _) = app_updating(&dir, &server, None);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    close_window(&mut h);
    assert!(h.query_by_label("Install Now").is_none());
    h.get_by_label("Release Page");
    h.get_by_label("What's New…").click();
    h.run_steps(2);
    h.get_by_label_contains("has no key to check a download against");
}

/// Later: downloaded and verified now, the image left as it is until
/// Moonglow closes, then replaced; with Install updates automatically,
/// the check does it without being asked.
#[test]
fn later_installs_the_update_as_moonglow_closes() {
    let dir = mg_testkit::scratch_dir("ui-update-later");
    let key = mg_testkit::signing::TestKey::new();
    let server = serve_release(&key, "v99.0.0", false, b"Moonglow 99.0.0's AppImage");
    let (app, image) = app_updating(&dir, &server, Some(key.public()));
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    h.get_by_label("Later").click();
    h.run_steps(2);
    h.get_by_label(
        "Moonglow 99.0.0 is downloaded and verified: it is installed when Moonglow closes.",
    );
    assert_eq!(std::fs::read(&image).unwrap(), b"the AppImage running", "not yet");
    assert_eq!(update_files(&dir), [".moonglow-update-99.0.0.AppImage"]);
    // Closed: what the desktop app does once the window has gone.
    h.state_mut().actions.push(mg_ui::Action::Quit);
    h.run();
    assert!(h.state().relaunch.is_none());
    let staged = h.state_mut().updates.at_close().expect("to install as it closes");
    mg_update::install::install(&staged, false, &dir.join("cache")).unwrap();
    assert_eq!(std::fs::read(&image).unwrap(), b"Moonglow 99.0.0's AppImage");
    assert!(update_files(&dir).is_empty());

    // Install updates automatically: at start, unasked.
    let dir = mg_testkit::scratch_dir("ui-update-automatic");
    let (mut app, image) = app_updating(&dir, &server, Some(key.public()));
    app.settings.update_auto_install = true;
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(2);
    h.get_by_label(
        "Moonglow 99.0.0 is downloaded and verified: it is installed when Moonglow closes.",
    );
    assert_eq!(std::fs::read(&image).unwrap(), b"the AppImage running");
    assert!(h.state_mut().updates.at_close().is_some());
    // What was left by a run that never closed is removed at the next start.
    let (mut app, _) = app_updating(&dir, &server, Some(key.public()));
    mg_ui::updates::startup(&mut app);
    assert!(update_files(&dir).is_empty());
}

/// As the desktop app runs it: the download on its own thread, its
/// progress shown, the answer taken when it has come.
#[test]
fn the_download_runs_on_its_own_thread() {
    let dir = mg_testkit::scratch_dir("ui-update-thread");
    let key = mg_testkit::signing::TestKey::new();
    let body: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
    let server = serve_release(&key, "v99.0.0", false, &body);
    let (mut app, image) = app_updating(&dir, &server, Some(key.public()));
    app.background_jobs = true;
    let mut h = harness(app, 1280.0, 800.0);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while h.state().updates.notice.is_none() {
        assert!(std::time::Instant::now() < deadline, "no notice");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.run_steps(1);
    }
    h.run_steps(2);
    h.get_by_label("Install Now").click();
    h.run_steps(1);
    while h.state().updates.getting.as_ref().is_some_and(|g| g.ready().is_none()) {
        assert!(std::time::Instant::now() < deadline, "not downloaded");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.run_steps(1);
    }
    h.run_steps(2);
    h.get_by_label("Moonglow 99.0.0 is installed: it runs from the next start.");
    assert_eq!(std::fs::read(&image).unwrap(), body);
}

/// A newer pre-release is offered only with Options › Updates › Include
/// pre-releases, which asks for the releases' list instead of the latest.
#[test]
fn a_pre_release_is_offered_only_with_the_option() {
    let dir = mg_testkit::scratch_dir("ui-update-pre");
    let key = mg_testkit::signing::TestKey::new();
    let server = serve_release(&key, "v99.0.0-rc.1", true, b"rc");
    // (The latest release is this one.)
    let current = mg_core::release::version();
    let latest = release_answer(&format!("v{current}"));
    server.put("/latest", mg_update::serve::Reply::Body(latest.into_bytes()));
    let (app, _) = app_updating(&dir, &server, Some(key.public()));
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert!(h.state().updates.notice.is_none(), "releases only");
    assert_eq!(server.asked(), ["/latest"]);
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Updates").click();
    h.run();
    h.get_by_label("Include pre-releases").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().settings.update_prereleases);
    mg_ui::updates::check_now(h.state_mut());
    h.run_steps(2);
    assert_eq!(server.asked().last().map(String::as_str), Some("/releases"));
    h.get_by_label(&format!("Moonglow 99.0.0-rc.1 (a pre-release) is out (you have {current})."));
    h.get_all_by_label("Install Now").last().unwrap().click();
    h.run_steps(2);
    h.get_all_by_label("Moonglow 99.0.0-rc.1 is installed: it runs from the next start.")
        .next()
        .unwrap();
}
