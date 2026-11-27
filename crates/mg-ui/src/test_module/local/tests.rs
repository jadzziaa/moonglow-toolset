use super::*;
use std::path::Path;

/// A fake server: a shell script that writes its arguments, prints
/// `said`, then reads its console until `exit` (written down when typed),
/// or ends at once with `end`.
fn fake_server(dir: &Path, said: &str, end: bool) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let script = dir.join("nwserver-fake");
    let args = shell_quoted(&dir.join("args.txt"));
    let exit = shell_quoted(&dir.join("exit.txt"));
    let rest = match end {
        true => "exit 3".to_string(),
        false => format!(
            "while read line; do case \"$line\" in exit) echo exit > {exit}; exit 0;; esac; done"
        ),
    };
    let pid = shell_quoted(&dir.join("server.pid"));
    let text =
        format!("#!/bin/sh\necho $$ > {pid}\necho \"$@\" > {args}\nprintf '{said}'\n{rest}\n");
    std::fs::write(&script, text).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

/// A prepare that makes the module's file and says a line.
fn prepared() -> Prepare {
    Box::new(|say: &mut dyn FnMut(String)| {
        say("made ready".into());
        Ok("fake".into())
    })
}

/// A fake game: a shell that stays `seconds`, writing its pid.
fn fake_game(dir: &Path, seconds: u32) -> Command {
    let mut c = Command::new("sh");
    let pid = shell_quoted(&dir.join("game.pid"));
    c.args(["-c", &format!("echo $$ > {pid}; sleep {seconds}")]);
    c
}

/// Polls the run until it ends (or a minute passes): all it said.
fn until_ended(run: &mut LocalRun) -> Vec<Progress> {
    let mut all = Vec::new();
    let start = Instant::now();
    while !run.ended() && start.elapsed() < Duration::from_secs(60) {
        all.extend(run.poll());
        std::thread::sleep(Duration::from_millis(50));
    }
    all
}

fn ended(all: &[Progress]) -> (String, bool) {
    match all.last() {
        Some(Progress::Ended { how, failed }) => (how.clone(), *failed),
        other => panic!("not ended: {other:?} of {all:?}"),
    }
}

const LOADS: &str = "Server: Loading module\\nServer: Module loaded\\n";

/// The character list: the server started on the plan's port, the module
/// loaded, the game started; the game ends, and the server is told to
/// exit. No vault (local characters).
#[test]
fn a_local_run_serves_until_the_game_ends() {
    let dir = mg_testkit::scratch_dir("ui-local-run-list");
    let mut plan = LocalPlan::new(
        fake_server(&dir, LOADS, false),
        dir.join("server"),
        prepared(),
        Join::CharacterList,
    );
    plan.within = Duration::from_secs(30);
    let port = plan.port;
    let mut run = LocalRun::start(plan, fake_game(&dir, 1));
    let all = until_ended(&mut run);
    assert_eq!(all[0], Progress::Preparing);
    assert!(all.contains(&Progress::Log("made ready".into())), "{all:?}");
    assert!(all.contains(&Progress::Starting { port }), "{all:?}");
    assert!(all.iter().any(|p| matches!(p, Progress::Loaded { .. })), "{all:?}");
    assert!(all.contains(&Progress::Joining), "{all:?}");
    assert_eq!(ended(&all), ("the game ended: the server was stopped".into(), false));
    assert!(dir.join("exit.txt").is_file(), "the server was told to exit");
    let args = std::fs::read_to_string(dir.join("args.txt")).unwrap();
    assert!(args.contains(&format!("-port {port}")) && !args.contains("-servervault"), "{args}");
    assert!(args.contains("-publicserver 0"), "{args}");
}

/// The quick start: the character alone in the vault folder of the
/// player's public key, and the server told to take it from there.
#[test]
fn a_quick_start_puts_the_character_in_the_vault() {
    let dir = mg_testkit::scratch_dir("ui-local-run-quick");
    std::fs::create_dir_all(&dir).unwrap();
    let bic = dir.join("hero.bic");
    std::fs::write(&bic, b"BIC V3.2").unwrap();
    // (A key nwn-message made up: its public part UANPXTFW.)
    let ini = dir.join("cdkey.ini");
    std::fs::write(&ini, "[NWN1]\nYourKey=HUHAQ-NMPXX-HTDFJ-WFELW-YL7NG-6W4UV-DTV7P\n").unwrap();
    let join = Join::QuickStart(Character::Bic(bic.clone()));
    let mut plan =
        LocalPlan::new(fake_server(&dir, LOADS, false), dir.join("server"), prepared(), join);
    plan.cdkey = Some(ini.clone());
    let mut run = LocalRun::start(plan, fake_game(&dir, 1));
    let all = until_ended(&mut run);
    assert!(!ended(&all).1, "{all:?}");
    let placed = dir.join("server/servervault/UANPXTFW/hero.bic");
    assert_eq!(std::fs::read(&placed).unwrap(), b"BIC V3.2");
    let args = std::fs::read_to_string(dir.join("args.txt")).unwrap();
    assert!(args.contains("-servervault 1"), "{args}");
    // No word of the key's private part in what the run said.
    let said = format!("{all:?}");
    assert!(!said.contains("HUHAQ"), "{said}");
    // Without the player's key: an error, and no server.
    let mut plan = LocalPlan::new(
        fake_server(&dir.join("b"), LOADS, false),
        dir.join("server-b"),
        prepared(),
        Join::QuickStart(Character::Bic(bic)),
    );
    plan.cdkey = Some(dir.join("no-such.ini"));
    let mut run = LocalRun::start(plan, fake_game(&dir, 1));
    let (how, failed) = ended(&until_ended(&mut run));
    assert!(failed && how.contains("vault"), "{how}");
    assert!(!dir.join("b/args.txt").exists());
}

/// Stopped: the server told to exit and the game this run started
/// stopped, nothing else.
#[test]
fn a_stopped_run_stops_its_server_and_its_game() {
    let dir = mg_testkit::scratch_dir("ui-local-run-stop");
    let plan = LocalPlan::new(
        fake_server(&dir, LOADS, false),
        dir.join("server"),
        prepared(),
        Join::CharacterList,
    );
    let mut run = LocalRun::start(plan, fake_game(&dir, 60));
    let start = Instant::now();
    let mut all = Vec::new();
    while !all.contains(&Progress::Joining) && start.elapsed() < Duration::from_secs(30) {
        all.extend(run.poll());
        std::thread::sleep(Duration::from_millis(50));
    }
    // (The game's shell has written its pid.)
    let pid_file = dir.join("game.pid");
    while !pid_file.is_file() && start.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(super::super::game_running(), "known as a test's game");
    run.stop();
    let all = until_ended(&mut run);
    assert_eq!(ended(&all), ("stopped".into(), false));
    assert!(dir.join("exit.txt").is_file());
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    let proc = PathBuf::from(format!("/proc/{}", pid.trim()));
    let gone = (0..100).any(|_| {
        std::thread::sleep(Duration::from_millis(20));
        !proc.exists()
    });
    assert!(gone, "the game was stopped");
}

/// A server that ends before its module loads: a failure with its last
/// lines, and no game started.
#[test]
fn a_server_that_ends_first_fails_the_run_with_its_last_lines() {
    let dir = mg_testkit::scratch_dir("ui-local-run-ended");
    let plan = LocalPlan::new(
        fake_server(&dir, "Server: Loading module\\nUnable to load module\\n", true),
        dir.join("server"),
        prepared(),
        Join::CharacterList,
    );
    let mut run = LocalRun::start(plan, fake_game(&dir, 1));
    let all = until_ended(&mut run);
    let (how, failed) = ended(&all);
    assert!(failed, "{how}");
    assert!(
        how.contains("ended before the module loaded") && how.contains("Unable to load module"),
        "{how}"
    );
    assert!(!all.contains(&Progress::Joining));
    assert!(!dir.join("game.pid").exists());
}

/// With NWNX whose core the loader can't preload (an empty file), the run
/// fails on the loader's own message, before the game joins.
#[test]
fn a_core_the_loader_refuses_fails_the_run() {
    let dir = mg_testkit::scratch_dir("ui-local-run-preload");
    let bin = dir.join("nwnx/Binaries");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("NWNX_Core.so"), b"").unwrap();
    let mut plan = LocalPlan::new(
        fake_server(&dir, LOADS, false),
        dir.join("server"),
        prepared(),
        Join::CharacterList,
    );
    plan.nwnx = Some(mg_module::nwnx::Nwnx {
        home: dir.join("nwnx"),
        binaries: bin.clone(),
        core: bin.join("NWNX_Core.so"),
        env_file: None,
        env: vec![],
        plugins: ["Core".to_string()].into(),
    });
    let mut run = LocalRun::start(plan, fake_game(&dir, 1));
    let all = until_ended(&mut run);
    let (how, failed) = ended(&all);
    assert!(failed && how.contains("NWNX did not run"), "{how}: {all:?}");
    let logged = all.iter().any(|p| matches!(p, Progress::Log(l) if l.contains("LD_PRELOAD")));
    assert!(logged, "{all:?}");
    assert!(!all.contains(&Progress::Joining));
}

/// Dropped (Moonglow closing) while it serves, the run is stopped and
/// waited for: by the time the drop returns, the server was told to exit
/// and the game this run started is gone.
#[test]
fn a_dropped_run_is_stopped_before_the_drop_returns() {
    let dir = mg_testkit::scratch_dir("ui-local-run-drop");
    let plan = LocalPlan::new(
        fake_server(&dir, LOADS, false),
        dir.join("server"),
        prepared(),
        Join::CharacterList,
    );
    let mut run = LocalRun::start(plan, fake_game(&dir, 60));
    let start = Instant::now();
    let mut all = Vec::new();
    let pid_file = dir.join("game.pid");
    while !(all.contains(&Progress::Joining) && pid_file.is_file())
        && start.elapsed() < Duration::from_secs(30)
    {
        all.extend(run.poll());
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(pid_file.is_file(), "{all:?}");
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    drop(run);
    assert!(dir.join("exit.txt").is_file(), "the server was told to exit");
    assert!(!PathBuf::from(format!("/proc/{}", pid.trim())).exists(), "the game was stopped");
}

/// What \`a_killed_moonglow_takes_its_run_with_it\` runs in a process of its
/// own: a local run, served and joined, then waiting to be killed.
#[test]
#[ignore]
fn local_run_helper() {
    let Some(dir) = std::env::var_os("MG_LOCAL_RUN_DIR").map(PathBuf::from) else { return };
    let plan = LocalPlan::new(
        fake_server(&dir, LOADS, false),
        dir.join("server"),
        prepared(),
        Join::CharacterList,
    );
    let mut run = LocalRun::start(plan, fake_game(&dir, 60));
    let start = Instant::now();
    while !(dir.join("game.pid").is_file() && dir.join("server.pid").is_file())
        && start.elapsed() < Duration::from_secs(30)
    {
        run.poll();
        std::thread::sleep(Duration::from_millis(50));
    }
    std::fs::write(dir.join("ready"), b"").unwrap();
    std::thread::sleep(Duration::from_secs(60));
}

/// Moonglow killed outright (\`kill -9\`, a crash) while a local run
/// serves: its server and its game die with it (the parent-death signal
/// of the run's thread), nothing left running.
#[test]
fn a_killed_moonglow_takes_its_run_with_it() {
    let dir = mg_testkit::scratch_dir("ui-local-run-killed");
    std::fs::create_dir_all(&dir).unwrap();
    let mut helper = Command::new(std::env::current_exe().unwrap());
    helper.args(["--exact", "test_module::local::tests::local_run_helper", "--ignored"]);
    helper.env("MG_LOCAL_RUN_DIR", &dir);
    helper.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    let mut helper = helper.spawn().unwrap();
    let start = Instant::now();
    while !dir.join("ready").is_file() && start.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(dir.join("ready").is_file(), "the helper's run never joined");
    let pid = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap().trim().to_string();
    let (server, game) = (pid("server.pid"), pid("game.pid"));
    helper.kill().unwrap();
    helper.wait().unwrap();
    // (Gone, or ended and not yet reaped by whoever took it.)
    let gone = |p: &str| match std::fs::read_to_string(format!("/proc/{p}/stat")) {
        Err(_) => true,
        Ok(stat) => stat.rsplit_once(") ").is_some_and(|(_, rest)| rest.starts_with('Z')),
    };
    let both = (0..250).any(|_| {
        std::thread::sleep(Duration::from_millis(20));
        gone(&server) && gone(&game)
    });
    // (Left running, they are stopped here: they are this test's.)
    for p in [&server, &game] {
        if !gone(p) {
            let _ = Command::new("kill").args(["-9", p.as_str()]).status();
        }
    }
    assert!(both, "the server ({}) or the game ({}) outlived Moonglow", gone(&server), gone(&game));
}
