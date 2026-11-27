//! Test Module on a local server, for a project that uses NWNX: chosen by
//! the project's [nwnx], its notes in the Log, a second test refused while
//! it runs, its stop (Build › Stop Test Server), and Moonglow closing
//! stopping it. Stand-ins for the server and the game (shell scripts named
//! as the game's), and NWNX's core a copy of a real library, so that the
//! loader preloads it.
#![cfg(target_os = "linux")]

use crate::common::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A shell script at `path`, executable.
fn script(path: &Path, text: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn quoted(p: &Path) -> String {
    format!("'{}'", p.display().to_string().replace('\'', "'\\''"))
}

/// A library the loader preloads into a shell (the C library's `libdl`).
fn a_real_library() -> Option<PathBuf> {
    ["/usr/lib/libdl.so.2", "/lib/x86_64-linux-gnu/libdl.so.2", "/usr/lib64/libdl.so.2"]
        .into_iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

/// The install's stand-ins in `dir`: a server that says NWNX's and its
/// own build, loads at once and reads its console until `exit` (written
/// down in `exit.txt`); a game that writes its arguments and pid, then
/// stays (a minute), or ends at once on `+TestNewModule`.
fn stand_ins(dir: &Path) -> PathBuf {
    let root = dir.join("root");
    let bin = root.join("bin/linux-x86");
    let exit = quoted(&dir.join("exit.txt"));
    let started = quoted(&dir.join("server.args"));
    script(
        &bin.join("nwserver-linux"),
        &format!(
            "#!/bin/sh\necho \"$@\" > {started}\n\
             printf 'Starting NWNX 8193.37-17 [2d8465a]\\nBuild: 8193.37-17 [26c6e573]\\n'\n\
             printf 'Server: Loading module\\nServer: Module loaded\\n'\n\
             while read line; do case \"$line\" in exit) echo exit > {exit}; exit 0;; esac; done\n"
        ),
    );
    let args = quoted(&dir.join("game.args"));
    let pid = quoted(&dir.join("game.pid"));
    script(
        &bin.join("nwmain-linux"),
        &format!(
            "#!/bin/sh\necho \"$@\" > {args}\n\
             case \"$*\" in *+TestNewModule*) exit 0;; esac\n\
             echo $$ > {pid}\nexec sleep 60\n"
        ),
    );
    root
}

/// Runs frames until `done`, for at most half a minute.
#[track_caller]
fn until(
    h: &mut egui_kittest::Harness<'_, mg_ui::Moonglow>,
    done: impl Fn(&mg_ui::Moonglow) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done(h.state()) {
        assert!(Instant::now() < deadline, "not in time: {:?}", h.state().log.entries);
        h.run_steps(1);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn logged(app: &mg_ui::Moonglow, text: &str) -> bool {
    app.log.entries.iter().any(|(_, m)| m.contains(text))
}

fn gone(pid_file: &Path) -> bool {
    let pid = std::fs::read_to_string(pid_file).unwrap();
    !Path::new(&format!("/proc/{}", pid.trim())).exists()
}

/// A project that uses NWNX is served by a local server with it, and the
/// game joins it (`+connect`): what NWNX says in the Log, the run as the
/// Status Bar's job; a second test refused while it runs; Build › Stop
/// Test Server stops the server and the game; Moonglow closing stops a
/// run under way before it closes. A module without NWNX is tested by
/// the game itself (`+TestNewModule`).
#[test]
fn a_project_with_nwnx_is_tested_on_a_local_server() {
    let _games = TEST_GAMES.lock().unwrap_or_else(|e| e.into_inner());
    let Some(lib) = a_real_library() else { return };
    let dir = mg_testkit::scratch_dir("ui-test-module-local");
    let root = stand_ins(&dir);
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("modules")).unwrap();
    let core = dir.join("nwnx/Binaries/NWNX_Core.so");
    std::fs::create_dir_all(core.parent().unwrap()).unwrap();
    std::fs::copy(&lib, &core).unwrap();
    // (Named for this process: a run of these tests beside it has its own.)
    let name = format!("mglocal{}", std::process::id());
    let file = user.join(format!("modules/{name}.mod"));
    std::fs::copy(sample_module(&dir), &file).unwrap();
    let home = dir.join("nwnx").display().to_string();
    std::fs::write(
        user.join(format!("modules/{name}.moonglow.toml")),
        format!("[nwnx]\nuse = true\nhome = \"{home}\"\n"),
    )
    .unwrap();
    let mut app = game_app_with(&root, Some(user.clone()), NoDialogs::default());
    app.settings.no_last_area = true;
    app.open_module(&file);
    let mut h = window_harness(app);
    h.run_steps(2);
    let start = |h: &mut egui_kittest::Harness<'_, mg_ui::Moonglow>| {
        let _ = std::fs::remove_file(dir.join("exit.txt"));
        let _ = std::fs::remove_file(dir.join("game.pid"));
        h.state_mut().actions.push(mg_ui::Action::TestModule);
        until(h, |a| logged(a, "on a local server with NWNX"));
        until(h, |_| dir.join("game.pid").is_file());
        until(h, |a| logged(a, "Test Module: the game is joining"));
    };
    start(&mut h);
    let app = h.state();
    assert!(
        logged(app, "Test Module: NWNX 8193.37-17 on server build 8193.37-17"),
        "{:?}",
        app.log.entries
    );
    assert!(app.local_run_under_way());
    assert!(!logged(app, &format!("Testing {name}")), "not the game's own test");
    let game = std::fs::read_to_string(dir.join("game.args")).unwrap();
    assert!(game.contains("+connect 127.0.0.1:") && game.contains("+password mg"), "{game}");
    let served = std::fs::read_to_string(dir.join("server.args")).unwrap();
    assert!(served.contains("-publicserver 0"), "{served}");
    // The run as the Status Bar's job.
    h.run_steps(2);
    assert!(h.query_by_label("Test Module, under way").is_some(), "the Status Bar's job");
    // A second test, refused.
    let from = h.state().log.entries.len();
    h.state_mut().actions.push(mg_ui::Action::TestModule);
    h.run_steps(3);
    let said = &h.state().log.entries[from..];
    assert!(
        said.iter().any(|(_, m)| m.contains("still running: close the game (or Build › Stop Test Server)")),
        "{said:?}"
    );
    // Build › Stop Test Server.
    menu_command(&mut h, "Build", "Stop Test Server");
    until(&mut h, |a| !a.local_run_under_way() && logged(a, "Test Module: stopped"));
    assert!(dir.join("exit.txt").is_file(), "the server was told to exit");
    assert!(gone(&dir.join("game.pid")), "the game was stopped");
    // Again, and Moonglow closes: stopped before it has closed.
    start(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Quit);
    h.run_steps(1);
    assert!(h.state().quit_requested);
    assert!(dir.join("exit.txt").is_file(), "the server was told to exit");
    assert!(gone(&dir.join("game.pid")), "the game was stopped");
}

/// Without NWNX (no [nwnx] in the project), Test Module is the game's own:
/// `+TestNewModule`, no local server.
#[test]
fn a_module_without_nwnx_is_tested_by_the_game() {
    let _games = TEST_GAMES.lock().unwrap_or_else(|e| e.into_inner());
    let dir = mg_testkit::scratch_dir("ui-test-module-plain");
    let root = stand_ins(&dir);
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("modules")).unwrap();
    let name = format!("mgplain{}", std::process::id());
    let file = user.join(format!("modules/{name}.mod"));
    std::fs::copy(sample_module(&dir), &file).unwrap();
    let mut app = game_app_with(&root, Some(user), NoDialogs::default());
    app.settings.no_last_area = true;
    app.open_module(&file);
    let mut h = window_harness(app);
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::TestModule);
    until(&mut h, |a| logged(a, &format!("Testing {name}")));
    assert!(!h.state().local_run_under_way());
    assert!(!logged(h.state(), "local server"));
    until(&mut h, |_| dir.join("game.args").is_file());
    let args = std::fs::read_to_string(dir.join("game.args")).unwrap();
    assert!(args.contains(&format!("+TestNewModule {name}")), "{args}");
    assert!(!dir.join("server.args").exists(), "no server");
}
