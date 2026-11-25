//! The machine's lock on the off-screen display (`docs/locks.md`).
//!
//! The game client, Aurora and the display's own restarts each need the
//! off-screen display to themselves: input sent with `xdrive.py` goes to
//! whatever window has focus, and a client's window is found by its title.
//! So one exclusive lock, `flock(2)` on `locks/display.lock` in the oracle's
//! folder (`MOONGLOW_ORACLE`, else `~/.local/share/moonglow-oracle`), covers
//! them all; `tools/locks.sh` takes the same file for the scripts.
//!
//! [`hold`] takes it for the rest of the test binary's run: a batch of client
//! tests reads `DISPLAY` once, and a lock given back between two runs would
//! let another run restart the display under the batch. It is taken
//! before the client's readiness clock starts, and the scripts it starts are
//! told it is held (`MOONGLOW_LOCKS_HELD`, [`held_by_us`]), or they would
//! wait for their own parent.
//!
//! The order of the machine's locks is the display, then a GPU slot
//! (`mg_testkit::gpu`): a test that holds a place on the GPU and waits for
//! the display could wait for a run that waits for its slot. So [`hold`]
//! panics, saying so, when the calling test already holds the GPU: the test
//! takes the display first.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a run waits for the display by default.
pub const WAIT: Duration = Duration::from_secs(30 * 60);
/// How often a waiting run repeats that it waits.
pub const SAY_EVERY: Duration = Duration::from_secs(5 * 60);

/// The oracle's folder: `MOONGLOW_ORACLE`, else
/// `~/.local/share/moonglow-oracle`.
fn oracle() -> Option<PathBuf> {
    let set = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    set("MOONGLOW_ORACLE").or_else(|| set("HOME").map(|h| h.join(".local/share/moonglow-oracle")))
}

/// The lock's folder, `locks` in the oracle's.
pub fn folder() -> Option<PathBuf> {
    oracle().map(|o| o.join("locks"))
}

/// How long to wait: `MOONGLOW_LOCK_WAIT` seconds (0 refuses at once), else
/// [`WAIT`].
pub fn wait_limit() -> Duration {
    std::env::var("MOONGLOW_LOCK_WAIT")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map_or(WAIT, Duration::from_secs)
}

/// Whether `MOONGLOW_LOCKS_HELD` (a comma list) names the display: an
/// ancestor of this process holds it.
pub fn held_by_ancestor() -> bool {
    std::env::var("MOONGLOW_LOCKS_HELD").is_ok_and(|v| v.split(',').any(|l| l.trim() == "display"))
}

/// Tells a script about to be started that the display is held (by this
/// process, or by its ancestor): `MOONGLOW_LOCKS_HELD` with `display` in it.
pub fn held_by_us(command: &mut Command) -> &mut Command {
    let mut held: Vec<String> = std::env::var("MOONGLOW_LOCKS_HELD")
        .map(|v| v.split(',').map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    if !held.iter().any(|l| l == "display") {
        held.push("display".into());
    }
    command.env("MOONGLOW_LOCKS_HELD", held.join(","))
}

/// The holder's line in the file: `PID 1234, started 09:12:05, <command>, in <cwd>`.
fn holder_line() -> String {
    let command: Vec<String> = std::env::args().collect();
    let cwd = std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default();
    let started = Command::new("date")
        .arg("+%T")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(utc_time);
    format!("PID {}, started {started}, {}, in {cwd}", std::process::id(), command.join(" "))
}

/// The time of day in UTC, where `date` can't say it in local time.
fn utc_time() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
        % 86_400;
    format!("{:02}:{:02}:{:02} UTC", secs / 3600, secs / 60 % 60, secs % 60)
}

/// Writes a line to standard error the way libtest lets through: its
/// output capture takes only the print macros.
fn say(line: &str) {
    let _ = io::stderr().write_all(format!("{line}\n").as_bytes());
}

fn minutes(d: Duration) -> String {
    if d.is_zero() || d >= Duration::from_secs(60) {
        format!("{} min", d.as_secs() / 60)
    } else {
        format!("{} s", d.as_secs())
    }
}

/// The refusal [`take_in`] gives when the lock stays held past the wait.
pub fn refusal(wait: Duration) -> String {
    format!("the off-screen display is still held after {}: nothing was run", minutes(wait))
}

/// Takes the lock in `dir` (the folder holding `display.lock`), waiting up
/// to `wait`; while it waits, `say` is given the line that says so, at once
/// and every `every`. The holder's line goes into the file. `Err` is the
/// refusal, or why the file can't be used.
pub fn take_in(
    dir: &Path,
    wait: Duration,
    every: Duration,
    say: &mut dyn FnMut(&str),
) -> Result<File, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let path = dir.join("display.lock");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let start = Instant::now();
    let mut next = Duration::ZERO;
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(e)) => {
                return Err(format!("cannot lock {}: {e}", path.display()));
            }
        }
        if start.elapsed() >= wait {
            return Err(refusal(wait));
        }
        if start.elapsed() >= next && !wait.is_zero() {
            next = start.elapsed() + every;
            let holder = std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| t.lines().next().map(String::from))
                .filter(|l| !l.is_empty())
                .unwrap_or_else(|| "someone".into());
            say(&format!(
                "waiting for the off-screen display: held by {holder}; up to {}",
                minutes(wait)
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    // (Written through this descriptor from its start: the holder's line is
    // the file's only line.)
    let _ = file.set_len(0);
    let _ = io::Seek::rewind(&mut file);
    let _ = writeln!(file, "{}", holder_line());
    Ok(file)
}

/// What the process's own hold came to: the file (kept open to the end of
/// the process), or why there is none.
static HELD: Mutex<Option<Result<Option<File>, String>>> = Mutex::new(None);

/// Takes the display for the rest of this process, waiting up to
/// `MOONGLOW_LOCK_WAIT` seconds (default 30 minutes) while another run holds
/// it, with a line on standard error saying whose it is. Taken once: the
/// calls after it return at once, or give the same refusal. Takes nothing
/// where `MOONGLOW_LOCKS_HELD` names the display (an ancestor holds it).
///
/// Panics, naming the rule, when the first call is made by a test that
/// holds a GPU place: the display comes before the GPU (see the module).
pub fn try_hold() -> Result<(), String> {
    let mut held = HELD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    // (Once this process has the display, asking again waits for nothing:
    // a test may hold a GPU place then. Only the first call, which could
    // wait, is held to the order.)
    if held.is_none() {
        assert!(
            !mg_testkit_gpu::holding(),
            "the display is taken before the GPU: call mg_testkit::display::hold() before \
             mg_testkit::gpu::hold() (a test that holds a GPU place and waits for the display \
             can wait for a run that waits for a GPU slot)"
        );
        *held = Some(if held_by_ancestor() {
            Ok(None)
        } else {
            match folder() {
                None => Err("no home folder for the display lock (MOONGLOW_ORACLE, HOME)".into()),
                Some(dir) => take_in(&dir, wait_limit(), SAY_EVERY, &mut |l| say(l)).map(Some),
            }
        });
    }
    match held.as_ref() {
        Some(Err(why)) => Err(why.clone()),
        _ => Ok(()),
    }
}

/// [`try_hold`], panicking with the refusal: for a test that needs the
/// display and has no better answer.
pub fn hold() {
    if let Err(why) = try_hold() {
        panic!("{why}");
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    #[cfg(unix)]
    use std::io::{BufRead, BufReader};
    use std::path::Path;
    #[cfg(unix)]
    use std::path::PathBuf;
    use std::process::Command;
    #[cfg(unix)]
    use std::process::{Child, Stdio};
    use std::time::Duration;

    #[cfg(unix)]
    use super::refusal;
    use super::{held_by_us, take_in};

    const CHILD: &str = "MG_DISPLAY_TEST_CHILD";

    // (The helpers serve the tests that run on Unix alone.)
    #[cfg(unix)]
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mg-display-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// This test binary run again as a separate process that takes the
    /// lock in `dir`, says `READY`, and holds it until its stdin closes.
    #[cfg(unix)]
    fn child(dir: &Path) -> Child {
        let mut c = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "display::tests::the_child_holds_the_display", "--nocapture"])
            .env(CHILD, dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut out = BufReader::new(c.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert!(out.read_line(&mut line).unwrap() > 0, "the child ended before it held");
            if line.trim() == "READY" {
                break;
            }
        }
        std::thread::spawn(move || for _ in out.lines() {});
        c
    }

    #[cfg(unix)]
    fn end(mut c: Child) {
        drop(c.stdin.take());
        assert!(c.wait().unwrap().success());
    }

    /// The child's side: not a test of its own where the variable is unset.
    #[test]
    fn the_child_holds_the_display() {
        let Some(dir) = std::env::var_os(CHILD) else { return };
        let _lock =
            take_in(Path::new(&dir), Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
        println!("READY");
        let _ = std::io::stdout().flush();
        let mut rest = String::new();
        let _ = std::io::stdin().read_line(&mut rest);
    }

    // (Unix only: on Windows a lock is mandatory, so the holder's line can't
    // be read while the file is held; the display is Linux's alone anyway.)
    #[cfg(unix)]
    #[test]
    fn the_display_is_exclusive_across_processes_and_says_who_holds_it() {
        let dir = scratch("exclusive");
        let held = child(&dir);
        let pid = held.id();
        // The holder's line: its PID, when it started, its command, its folder.
        let line = std::fs::read_to_string(dir.join("display.lock")).unwrap();
        assert_eq!(line.lines().count(), 1, "{line:?}");
        assert!(line.starts_with(&format!("PID {pid}, started ")), "{line:?}");
        assert!(line.contains("the_child_holds_the_display"), "{line:?}");
        assert!(line.contains(", in /"), "{line:?}");
        // Refused at once with no wait, and after a short one, with the message.
        let mut said = Vec::new();
        let refused = take_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |l| {
            said.push(l.to_string())
        });
        assert_eq!(
            refused.unwrap_err(),
            "the off-screen display is still held after 0 min: nothing was run"
        );
        assert!(said.is_empty(), "a refusal at once is not a wait: {said:?}");
        let wait = Duration::from_secs(1);
        let refused =
            take_in(&dir, wait, Duration::from_millis(300), &mut |l| said.push(l.to_string()));
        assert_eq!(refused.unwrap_err(), refusal(wait));
        assert!(said.len() >= 2, "said at first and again: {said:?}");
        assert!(
            said[0].starts_with("waiting for the off-screen display: held by PID "),
            "{said:?}"
        );
        assert!(said[0].contains(&format!("PID {pid},")), "{said:?}");
        assert!(said[0].ends_with("; up to 1 s"), "{said:?}");
        // A waiter gets it when the holder ends.
        let waiter = {
            let dir = dir.clone();
            std::thread::spawn(move || {
                take_in(&dir, Duration::from_secs(30), Duration::from_secs(1), &mut |_| {})
            })
        };
        std::thread::sleep(Duration::from_millis(400));
        end(held);
        let lock = waiter.join().unwrap().expect("taken once the holder ended");
        let line = std::fs::read_to_string(dir.join("display.lock")).unwrap();
        assert!(line.starts_with(&format!("PID {},", std::process::id())), "{line:?}");
        drop(lock);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A test that holds a GPU place and then asks for the display is the
    /// order that could deadlock two runs: it panics, naming the rule.
    #[test]
    fn the_display_is_never_taken_after_the_gpu() {
        mg_testkit_gpu::hold();
        let why = std::panic::catch_unwind(super::try_hold).unwrap_err();
        mg_testkit_gpu::release();
        let why = why
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| why.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        assert!(why.contains("the display is taken before the GPU"), "{why}");
    }

    #[test]
    fn a_script_started_under_the_hold_is_told_so() {
        let mut c = Command::new("true");
        held_by_us(&mut c);
        let held = c
            .get_envs()
            .find(|(k, _)| *k == "MOONGLOW_LOCKS_HELD")
            .and_then(|(_, v)| v.map(|v| v.to_string_lossy().into_owned()))
            .unwrap();
        assert!(held.split(',').any(|l| l == "display"), "{held}");
    }

    /// `tools/locks.sh` and this module take the one file: a hold by the
    /// script refuses this side, and the other way round.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_script_and_the_module_share_one_lock() {
        let have = |tool: &str| {
            Command::new(tool).arg("--version").output().is_ok_and(|o| o.status.success())
        };
        if !have("bash") || !have("flock") {
            eprintln!("skipped: no bash or flock(1)");
            return;
        }
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/locks.sh");
        let oracle = scratch("script");
        let run = |wait: &str, held: Option<&str>, argv: &[&str]| {
            let mut c = Command::new(&script);
            c.args(["hold", "display", "--"])
                .args(argv)
                .env("MOONGLOW_ORACLE", &oracle)
                .env("MOONGLOW_LOCK_WAIT", wait)
                .env_remove("MOONGLOW_LOCKS_HELD");
            if let Some(held) = held {
                c.env("MOONGLOW_LOCKS_HELD", held);
            }
            c
        };
        // The script holds (sleep): another script, with no wait, is refused
        // with the message and status 2, this module is too, and a script told
        // an ancestor holds it runs.
        let mut holder = run("30", None, &["sleep", "30"]).stderr(Stdio::null()).spawn().unwrap();
        let file = oracle.join("locks/display.lock");
        let started = std::time::Instant::now();
        while std::fs::read_to_string(&file).unwrap_or_default().is_empty() {
            assert!(started.elapsed() < Duration::from_secs(20), "the script never took the lock");
            std::thread::sleep(Duration::from_millis(50));
        }
        let line = std::fs::read_to_string(&file).unwrap();
        assert!(line.starts_with(&format!("PID {},", holder.id())), "{line:?}");
        assert!(line.contains(", sleep 30, in /"), "{line:?}");
        let refused = run("0", None, &["echo", "ran"]).output().unwrap();
        assert_eq!(refused.status.code(), Some(2));
        assert!(refused.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&refused.stderr).trim(),
            "the off-screen display is still held after 0 min: nothing was run"
        );
        let waited = run("1", None, &["echo", "ran"]).output().unwrap();
        assert_eq!(waited.status.code(), Some(2));
        let err = String::from_utf8_lossy(&waited.stderr).into_owned();
        assert!(
            err.starts_with(&format!(
                "waiting for the off-screen display: held by PID {},",
                holder.id()
            )),
            "{err}"
        );
        assert!(err.contains("; up to 1 s\n"), "{err}");
        assert!(err.trim_end().ends_with("after 1 s: nothing was run"), "{err}");
        let skipped = run("0", Some("display"), &["echo", "ran"]).output().unwrap();
        assert!(skipped.status.success());
        assert_eq!(String::from_utf8_lossy(&skipped.stdout).trim(), "ran");
        assert_eq!(
            take_in(&oracle.join("locks"), Duration::ZERO, Duration::from_secs(1), &mut |_| {})
                .unwrap_err(),
            "the off-screen display is still held after 0 min: nothing was run"
        );
        // And the child of the holder sees `display` held.
        let _ = holder.kill();
        let _ = holder.wait();
        let seen = run("5", None, &["bash", "-c", "echo $MOONGLOW_LOCKS_HELD"]).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&seen.stdout).trim(), "display");
        // The module held it: the script is refused.
        let lock =
            take_in(&oracle.join("locks"), Duration::ZERO, Duration::from_secs(1), &mut |_| {})
                .unwrap();
        let refused = run("0", None, &["echo", "ran"]).output().unwrap();
        assert_eq!(refused.status.code(), Some(2));
        drop(lock);
        let _ = std::fs::remove_dir_all(&oracle);
    }

    /// The display's history: one line an event, saying whether the lock
    /// was this run's or an ancestor's, the last 1,000 kept.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_display_history_says_who_held_the_lock() {
        if !Command::new("flock").arg("--version").output().is_ok_and(|o| o.status.success()) {
            eprintln!("skipped: no flock(1)");
            return;
        }
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/locks.sh");
        let oracle = scratch("history");
        let history = oracle.join("locks/display.history");
        let run = |body: &str, held: Option<&str>| {
            let mut c = Command::new("bash");
            c.arg("-c")
                .arg(format!(". '{}'; {body}", script.display()))
                .env("MOONGLOW_ORACLE", &oracle)
                .env("MOONGLOW_LOCK_WAIT", "5")
                .env_remove("MOONGLOW_LOCKS_HELD");
            if let Some(held) = held {
                c.env("MOONGLOW_LOCKS_HELD", held);
            }
            assert!(c.status().unwrap().success());
        };
        run("mg_lock_display 'a test'; mg_display_history 'headless.sh start: display :9'", None);
        run("mg_display_history 'run-client.sh end: status 0'", Some("display"));
        run("mg_display_history 'headless.sh stop: no session running'", None);
        let text = std::fs::read_to_string(&history).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "{text}");
        // (2026-11-25T18:00:00Z PID 1234: ...)
        assert!(lines[0].len() > 21 && lines[0].as_bytes()[19] == b'Z', "{}", lines[0]);
        assert!(
            lines[0]
                .contains(": headless.sh start: display :9; lock held by this run; called by PID "),
            "{}",
            lines[0]
        );
        assert!(lines[1].contains("; lock held by an ancestor (PID "), "{}", lines[1]);
        assert!(lines[1].contains(", a test, in /"), "{}", lines[1]);
        assert!(lines[2].contains("; lock not held; "), "{}", lines[2]);
        assert!(lines[2].contains("; in /"), "{}", lines[2]);
        // Past 1,200 lines the last 1,000 are kept, the newest last.
        let old: String = (0..1300).map(|n| format!("old {n}\n")).collect();
        std::fs::write(&history, old).unwrap();
        run("mg_display_history 'newest'", None);
        let text = std::fs::read_to_string(&history).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 1000);
        assert_eq!(lines[0], "old 301");
        assert!(lines[999].contains(": newest; lock not held"), "{}", lines[999]);
        let _ = std::fs::remove_dir_all(&oracle);
    }

    /// What starts the headless session (headless.sh) closes the lock's
    /// descriptor first, or the lock would be held for the session's life.
    #[cfg(target_os = "linux")]
    #[test]
    fn descriptors_above_two_are_closed_for_what_outlives_the_hold() {
        if !Command::new("bash").arg("--version").output().is_ok_and(|o| o.status.success()) {
            eprintln!("skipped: no bash");
            return;
        }
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/locks.sh");
        // A descriptor open in the shell (fd 7, as the lock's is) is open in
        // an ordinary child and not in the one `mg_close_extra_fds` ran in.
        let probe = format!(
            ". '{}'; exec 7>/dev/null; exec 12>/dev/null; \
             echo \"plain: $(ls /proc/self/fd | tr '\\n' ' ')\"; \
             ( mg_close_extra_fds; echo \"closed: $(ls /proc/self/fd | tr '\\n' ' ')\" )",
            script.display()
        );
        let out = Command::new("bash").arg("-c").arg(probe).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let fds = |tag: &str| -> Vec<u32> {
            let line = text.lines().find(|l| l.starts_with(tag)).unwrap();
            line[tag.len()..].split_whitespace().filter_map(|n| n.parse().ok()).collect()
        };
        let plain = fds("plain: ");
        assert!(plain.contains(&7) && plain.contains(&12), "{text}");
        let closed = fds("closed: ");
        assert!(!closed.contains(&7) && !closed.contains(&12), "{text}");
    }

    /// `headless.sh start` takes the lock while it launches a session and
    /// gives it back: the session it started (a stand-in for KWin here) has
    /// not inherited the descriptor. A session already running is taken
    /// nothing for, and `stop` takes it.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_started_session_does_not_keep_the_lock() {
        use std::os::unix::fs::PermissionsExt;
        let have = |tool: &str| {
            Command::new(tool).arg("--version").output().is_ok_and(|o| o.status.success())
        };
        if !have("bash") || !have("flock") || !have("setsid") {
            eprintln!("skipped: no bash, flock(1) or setsid");
            return;
        }
        let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
        let oracle = scratch("session");
        let bin = oracle.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        // (What KWin's launcher does: runs the program the session is for.)
        let fake = bin.join("dbus-run-session");
        std::fs::write(&fake, "#!/bin/sh\nfor last; do :; done\nDISPLAY=:77 exec \"$last\"\n")
            .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
        let headless = |what: &str, wait: &str| {
            let mut c = Command::new(tools.join("aurora/headless.sh"));
            c.arg(what)
                .env("MOONGLOW_ORACLE", &oracle)
                .env("MOONGLOW_LOCK_WAIT", wait)
                .env("PATH", &path)
                .env_remove("MOONGLOW_LOCKS_HELD");
            c
        };
        struct Stop<'a>(&'a dyn Fn(&str, &str) -> Command);
        impl Drop for Stop<'_> {
            fn drop(&mut self) {
                let _ = (self.0)("stop", "30").output();
            }
        }
        let _stop = Stop(&headless);
        let out = headless("start", "30").output().unwrap();
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), ":77", "{out:?}");
        assert!(out.stderr.is_empty(), "{out:?}");
        let free = Command::new(tools.join("locks.sh"))
            .args(["hold", "display", "--", "true"])
            .env("MOONGLOW_ORACLE", &oracle)
            .env("MOONGLOW_LOCK_WAIT", "0")
            .env_remove("MOONGLOW_LOCKS_HELD")
            .output()
            .unwrap();
        assert!(free.status.success(), "the session kept the lock: {free:?}");
        // With the display held by someone else, a start of the running
        // session still prints it and takes nothing; a stop is refused.
        let lock =
            take_in(&oracle.join("locks"), Duration::ZERO, Duration::from_secs(1), &mut |_| {})
                .unwrap();
        let again = headless("start", "0").output().unwrap();
        assert_eq!(String::from_utf8_lossy(&again.stdout).trim(), ":77", "{again:?}");
        let stop = headless("stop", "0").output().unwrap();
        assert_eq!(stop.status.code(), Some(2), "{stop:?}");
        drop(lock);
    }
}
