//! The quiet hold: a run that needs the machine to itself (a timing run, a
//! soak) against the ordinary heavy runs (test binaries, the oracle
//! scripts, an orchestrator's suites), `docs/locks.md`.
//!
//! Two lock files in [`folder`], `quiet.lock` and `quiet.gate`, both
//! `flock(2)` (`std::fs::File::lock`, the same family; `flock(1)` in
//! `tools/locks.sh` takes the same files the same way):
//!
//! * An ordinary run takes `quiet.lock` **shared** ([`share`]): ordinary
//!   runs never exclude each other, and all of them exclude a quiet holder.
//!   It passes through the gate first: `quiet.gate` shared while it takes
//!   `quiet.lock`, given back at once.
//! * A quiet run takes `quiet.gate` **exclusive**, then `quiet.lock`
//!   exclusive ([`hold`]), and keeps both. Held, the gate stops new ordinary
//!   runs while the quiet run waits for those already running to end;
//!   without it, test binaries starting one after another could keep a
//!   quiet run waiting for ever (a lock waiting to be exclusive does not
//!   stop new shared takers).
//!
//! The order of the machine's locks is the quiet lock, then the display,
//! then a GPU slot. Nobody upgrades: a process that shares the quiet lock
//! never takes it exclusive ([`hold`] panics, saying so). A test binary
//! shares it for the rest of its run from its first GPU or display hold
//! (and `mg_testkit::nwn_root`, the corpus tests' first call), before
//! anything else is taken.
//!
//! A holder of either kind tells what it starts (`MOONGLOW_LOCKS_HELD` with
//! `quiet` in it, [`held_by_us`]): a child that shared the lock again would
//! wait at the gate behind a quiet run that waits for the child's parent.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a run waits by default (`MOONGLOW_LOCK_WAIT` sets it).
pub const WAIT: Duration = Duration::from_secs(30 * 60);
/// How often a waiting run repeats that it waits.
pub const SAY_EVERY: Duration = Duration::from_secs(5 * 60);
const POLL: Duration = Duration::from_millis(100);

/// The folder of the machine's quiet lock, beside the GPU slots':
/// `$XDG_STATE_HOME/moonglow/locks`, else `~/.local/state/moonglow/locks`
/// (on Windows, under `%LOCALAPPDATA%` without a home). Not moved by
/// `MOONGLOW_ORACLE`: the machine is the machine.
pub fn folder() -> Option<PathBuf> {
    crate::slots::folder().and_then(|gpu| gpu.parent().map(Path::to_path_buf))
}

/// How long to wait: `MOONGLOW_LOCK_WAIT` seconds (0 refuses at once), else
/// [`WAIT`].
pub fn wait_limit() -> Duration {
    std::env::var("MOONGLOW_LOCK_WAIT")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map_or(WAIT, Duration::from_secs)
}

/// Whether `MOONGLOW_LOCKS_HELD` (a comma list) names `quiet`: an ancestor
/// of this process holds the quiet lock, shared or alone.
pub fn held_by_ancestor() -> bool {
    std::env::var("MOONGLOW_LOCKS_HELD").is_ok_and(|v| v.split(',').any(|l| l.trim() == "quiet"))
}

/// Tells a program about to be started that the quiet lock is held (by
/// this process or its ancestor): `MOONGLOW_LOCKS_HELD` with `quiet` in it.
pub fn held_by_us(command: &mut Command) -> &mut Command {
    let mut held: Vec<String> = std::env::var("MOONGLOW_LOCKS_HELD")
        .map(|v| v.split(',').map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    if !held.iter().any(|l| l == "quiet") {
        held.push("quiet".into());
    }
    command.env("MOONGLOW_LOCKS_HELD", held.join(","))
}

fn minutes(d: Duration) -> String {
    if d.is_zero() || d >= Duration::from_secs(60) {
        format!("{} min", d.as_secs() / 60)
    } else {
        format!("{} s", d.as_secs())
    }
}

/// The refusal an ordinary run gives when a quiet run keeps the machine
/// past its wait.
pub fn refusal_shared(wait: Duration) -> String {
    format!("the machine is still kept quiet after {}: nothing was run", minutes(wait))
}

/// The refusal a quiet run gives when other runs keep the machine busy past
/// its wait.
pub fn refusal_alone(wait: Duration) -> String {
    format!("the machine is still busy after {}: nothing was run", minutes(wait))
}

fn open(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))
}

/// The quiet holder's line in `quiet.lock`, or "someone".
fn holder(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("quiet.lock"))
        .ok()
        .and_then(|t| t.lines().next().map(String::from))
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| "someone".into())
}

/// Tries `lock` until it takes or `wait` has passed since `start`; while it
/// waits, `say` is given `line()` at once and every `every`.
fn wait_for(
    start: Instant,
    wait: Duration,
    every: Duration,
    next: &mut Duration,
    say: &mut dyn FnMut(&str),
    line: &dyn Fn() -> String,
    lock: &mut dyn FnMut() -> Result<bool, String>,
) -> Result<bool, String> {
    loop {
        if lock()? {
            return Ok(true);
        }
        if start.elapsed() >= wait {
            return Ok(false);
        }
        if start.elapsed() >= *next {
            *next = start.elapsed() + every;
            say(&line());
        }
        std::thread::sleep(POLL);
    }
}

fn tried(r: Result<(), TryLockError>, path: &Path) -> Result<bool, String> {
    match r {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) => Ok(false),
        Err(TryLockError::Error(e)) => Err(format!("cannot lock {}: {e}", path.display())),
    }
}

/// Shares the quiet lock in `dir`, waiting up to `wait` while a quiet run
/// holds or waits for it; while it waits, `say` is given the line that says
/// so, at once and every `every`. The file is the share: kept open, it
/// lasts. `Err` is the refusal, or why the files can't be used.
pub fn share_in(
    dir: &Path,
    wait: Duration,
    every: Duration,
    say: &mut dyn FnMut(&str),
) -> Result<File, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let (gate_path, lock_path) = (dir.join("quiet.gate"), dir.join("quiet.lock"));
    let (gate, lock) = (open(&gate_path)?, open(&lock_path)?);
    let line = || {
        format!("waiting for a quiet run to end: held by {}; up to {}", holder(dir), minutes(wait))
    };
    let (start, mut next) = (Instant::now(), Duration::ZERO);
    if wait.is_zero() {
        next = Duration::MAX;
    }
    let mut both = || {
        if !tried(gate.try_lock_shared(), &gate_path)? {
            return Ok(false);
        }
        // (Past the gate, no quiet run holds the lock: one would hold the
        // gate too. Shared at once, then the gate given back.)
        let took = tried(lock.try_lock_shared(), &lock_path);
        let _ = gate.unlock();
        took
    };
    match wait_for(start, wait, every, &mut next, say, &line, &mut both)? {
        true => Ok(lock),
        false => Err(refusal_shared(wait)),
    }
}

/// A quiet hold: the gate and the lock, both exclusive, given back when
/// dropped (or at the process's end).
#[derive(Debug)]
pub struct Quiet {
    gate: File,
    lock: File,
}

impl Drop for Quiet {
    fn drop(&mut self) {
        // (Unlocked before the files close: a child spawned on another
        // thread can hold a copy of a descriptor until its exec.)
        let _ = self.lock.unlock();
        let _ = self.gate.unlock();
    }
}

/// Takes the machine alone in `dir`: the gate, then the lock, waiting up to
/// `wait` in all for another quiet run and for the ordinary runs to end;
/// while it waits, `say` is given the line that says so, at once and every
/// `every`. The holder's line goes into `quiet.lock`.
pub fn hold_in(
    dir: &Path,
    wait: Duration,
    every: Duration,
    say: &mut dyn FnMut(&str),
) -> Result<Quiet, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    let (gate_path, lock_path) = (dir.join("quiet.gate"), dir.join("quiet.lock"));
    let (gate, mut lock) = (open(&gate_path)?, open(&lock_path)?);
    let (start, mut next) = (Instant::now(), Duration::ZERO);
    if wait.is_zero() {
        next = Duration::MAX;
    }
    let other = || {
        format!("waiting for a quiet run to end: held by {}; up to {}", holder(dir), minutes(wait))
    };
    let took = wait_for(start, wait, every, &mut next, say, &other, &mut || {
        tried(gate.try_lock(), &gate_path)
    })?;
    if !took {
        return Err(refusal_alone(wait));
    }
    let busy = || {
        let pids = crate::slots::lock_holders(std::slice::from_ref(&lock_path));
        let runs = match pids.len() {
            0 => "other runs hold it".to_string(),
            1 => format!("1 run holds it: PID {}", pids[0]),
            n => {
                let pids: Vec<String> = pids.iter().map(u32::to_string).collect();
                format!("{n} runs hold it: PIDs {}", pids.join(", "))
            }
        };
        format!("waiting for the machine to be quiet ({runs}); up to {}", minutes(wait))
    };
    let took = wait_for(start, wait, every, &mut next, say, &busy, &mut || {
        tried(lock.try_lock(), &lock_path)
    })?;
    if !took {
        let _ = gate.unlock();
        return Err(refusal_alone(wait));
    }
    let _ = lock.set_len(0);
    let _ = io::Seek::rewind(&mut lock);
    let _ = writeln!(lock, "{}", holder_line());
    Ok(Quiet { gate, lock })
}

/// The holder's line: `PID 1234, started 09:12:05, <command>, in <cwd>`.
fn holder_line() -> String {
    let command: Vec<String> = std::env::args().collect();
    let cwd = std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default();
    let started = Command::new("date")
        .arg("+%T")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
                % 86_400;
            format!("{:02}:{:02}:{:02} UTC", secs / 3600, secs / 60 % 60, secs % 60)
        });
    format!("PID {}, started {started}, {}, in {cwd}", std::process::id(), command.join(" "))
}

/// What this process holds of the quiet lock, kept to its end.
enum Held {
    /// Taken by an ancestor, or not to be had (no folder): nothing.
    Nothing,
    Shared(#[allow(dead_code)] File),
    Alone(#[allow(dead_code)] Quiet),
    /// Refused once: the later calls give the same refusal at once, not a
    /// wait each.
    Refused(String),
}

static HELD: Mutex<Option<Held>> = Mutex::new(None);

fn say(line: &str) {
    let _ = io::stderr().write_all(format!("{line}\n").as_bytes());
}

/// Shares the quiet lock for the rest of this process (an ordinary heavy
/// run), waiting up to `MOONGLOW_LOCK_WAIT` seconds (default 30 minutes)
/// while a quiet run holds it, with a line on standard error saying whose
/// it is. Taken once: later calls return at once. Takes nothing where an
/// ancestor holds it, or this process holds it alone. Where the folder
/// can't be had, it says so once and goes on: a test never fails for want
/// of the lock.
///
/// Panics with the refusal when the wait runs out, and at once at each
/// later call (as the display hold does).
pub fn share() {
    let mut held = HELD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(Held::Refused(why)) = held.as_ref() {
        let why = why.clone();
        drop(held);
        panic!("{why}");
    }
    if held.is_some() {
        return;
    }
    *held = Some(if held_by_ancestor() {
        Held::Nothing
    } else if let Some(dir) = folder() {
        match share_in(&dir, wait_limit(), SAY_EVERY, &mut |l| say(l)) {
            Ok(file) => Held::Shared(file),
            Err(why) if why.starts_with("the machine is") => {
                *held = Some(Held::Refused(why.clone()));
                drop(held);
                panic!("{why}");
            }
            Err(why) => {
                say(&format!("the quiet lock is not taken: {why}"));
                Held::Nothing
            }
        }
    } else {
        say("the quiet lock is not taken: no home folder for it (XDG_STATE_HOME, HOME)");
        Held::Nothing
    });
}

/// Takes the machine alone for the rest of this process (a timing run),
/// waiting up to `MOONGLOW_LOCK_WAIT` seconds for another quiet run and for
/// the ordinary runs to end, with a line on standard error saying what it
/// waits for. Its own GPU and display holds then share nothing. Taken once.
///
/// Panics where this process already shares the quiet lock (nobody
/// upgrades: two processes doing so would wait for each other), and with
/// the refusal when the wait runs out.
pub fn hold() {
    let mut held = HELD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    match held.as_ref() {
        Some(Held::Alone(_)) => return,
        Some(Held::Shared(_)) => {
            drop(held);
            panic!(
                "the quiet hold is taken first: call mg_testkit::quiet::hold() before anything \
                 that shares the quiet lock (a GPU or display hold, mg_testkit::nwn_root): \
                 nobody upgrades a lock"
            );
        }
        Some(Held::Nothing | Held::Refused(_)) | None => {}
    }
    if held_by_ancestor() {
        *held = Some(Held::Nothing);
        return;
    }
    let Some(dir) = folder() else {
        drop(held);
        panic!("no home folder for the quiet lock (XDG_STATE_HOME, HOME)");
    };
    match hold_in(&dir, wait_limit(), SAY_EVERY, &mut |l| say(l)) {
        Ok(q) => *held = Some(Held::Alone(q)),
        Err(why) => {
            drop(held);
            panic!("{why}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::Duration;

    use super::{hold_in, refusal_alone, refusal_shared, share_in};

    const CHILD: &str = "MG_QUIET_TEST_CHILD";

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mg-quiet-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn quick() -> (Duration, Duration) {
        (Duration::from_millis(600), Duration::from_millis(200))
    }

    /// Ordinary runs share it; a quiet run waits for them, saying who.
    #[test]
    fn ordinary_runs_share_and_a_quiet_run_waits_for_them() {
        let dir = scratch("share");
        let a = share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
        let b = share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
        let mut said = Vec::new();
        let (wait, every) = quick();
        let refused = hold_in(&dir, wait, every, &mut |l| said.push(l.to_string()));
        assert_eq!(refused.unwrap_err(), refusal_alone(wait));
        assert!(said.len() >= 2, "said at first and again: {said:?}");
        assert!(said[0].starts_with("waiting for the machine to be quiet ("), "{said:?}");
        #[cfg(target_os = "linux")]
        assert!(said[0].contains(&format!("PID {}", std::process::id())), "{said:?}");
        assert!(said[0].ends_with("; up to 0 s"), "{said:?}");
        // (Refused, it gave the gate back: an ordinary run passes.)
        drop(share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap());
        // With no wait, refused at once and said nothing.
        let mut said = Vec::new();
        let refused = hold_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |l| {
            said.push(l.to_string())
        });
        assert_eq!(refused.unwrap_err(), "the machine is still busy after 0 min: nothing was run");
        assert!(said.is_empty(), "{said:?}");
        drop((a, b));
        let q = hold_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
        // (On Windows a lock is mandatory: the line can't be read while held.)
        #[cfg(unix)]
        {
            let line = std::fs::read_to_string(dir.join("quiet.lock")).unwrap();
            assert!(line.starts_with(&format!("PID {}, started ", std::process::id())), "{line:?}");
            assert_eq!(line.lines().count(), 1, "{line:?}");
        }
        drop(q);
    }

    /// A quiet run excludes ordinary runs and another quiet run, each told
    /// whose it is, and they take it once it ends.
    #[test]
    fn a_quiet_run_keeps_the_others_waiting() {
        let dir = scratch("alone");
        let q = hold_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
        let (wait, every) = quick();
        let mut said = Vec::new();
        let refused = share_in(&dir, wait, every, &mut |l| said.push(l.to_string()));
        assert_eq!(refused.unwrap_err(), refusal_shared(wait));
        assert!(said[0].starts_with("waiting for a quiet run to end: "), "{said:?}");
        #[cfg(unix)]
        assert!(said[0].starts_with("waiting for a quiet run to end: held by PID "), "{said:?}");
        assert!(said[0].ends_with("; up to 0 s"), "{said:?}");
        let refused = hold_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {});
        assert_eq!(refused.unwrap_err(), refusal_alone(Duration::ZERO));
        let waiter = {
            let dir = dir.clone();
            std::thread::spawn(move || {
                share_in(&dir, Duration::from_secs(30), Duration::from_secs(30), &mut |_| {})
            })
        };
        std::thread::sleep(Duration::from_millis(300));
        drop(q);
        assert!(waiter.join().unwrap().is_ok());
    }

    /// A quiet run waiting for an ordinary one stops new ordinary runs at
    /// the gate, so a stream of them can't keep it waiting.
    #[test]
    fn a_waiting_quiet_run_comes_before_new_ordinary_runs() {
        let dir = scratch("gate");
        let running = share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap();
        let waiter = {
            let dir = dir.clone();
            std::thread::spawn(move || {
                hold_in(&dir, Duration::from_secs(30), Duration::from_secs(30), &mut |_| {})
            })
        };
        // (Until the waiter holds the gate: a new ordinary run is refused.)
        let start = std::time::Instant::now();
        loop {
            match share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}) {
                Ok(f) => drop(f),
                Err(why) => {
                    assert_eq!(why, refusal_shared(Duration::ZERO));
                    break;
                }
            }
            assert!(start.elapsed().as_secs() < 10, "the waiter never held the gate");
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(running);
        let q = waiter.join().unwrap().expect("the quiet run, once the ordinary one ended");
        drop(q);
        drop(share_in(&dir, Duration::ZERO, Duration::from_secs(1), &mut |_| {}).unwrap());
    }

    /// The child's side of the next test: shares, then asks to hold alone.
    #[test]
    fn the_child_shares_then_upgrades() {
        if std::env::var_os(CHILD).is_none() {
            return;
        }
        super::share();
        super::hold();
    }

    /// Nobody upgrades: a process that shares the lock and asks to hold it
    /// alone panics, naming the rule. An ancestor's hold is not taken again.
    #[test]
    fn a_share_is_never_upgraded_and_an_ancestors_hold_is_not_taken_again() {
        let state = scratch("upgrade");
        let run = |held: Option<&str>| {
            let mut c = Command::new(std::env::current_exe().unwrap());
            c.args(["--exact", "quiet::tests::the_child_shares_then_upgrades", "--nocapture"])
                .env(CHILD, "1")
                .env("XDG_STATE_HOME", &state)
                .env("MOONGLOW_LOCK_WAIT", "0")
                .env_remove("MOONGLOW_LOCKS_HELD");
            if let Some(h) = held {
                c.env("MOONGLOW_LOCKS_HELD", h);
            }
            c.output().unwrap()
        };
        let out = run(None);
        assert!(!out.status.success(), "{out:?}");
        let text = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(text.contains("nobody upgrades a lock"), "{text}");
        // Under an ancestor's hold (shared or alone), both take nothing,
        // even with a quiet run holding the lock.
        let q = hold_in(
            &state.join("moonglow/locks"),
            Duration::ZERO,
            Duration::from_secs(1),
            &mut |_| {},
        )
        .unwrap();
        let out = run(Some("display,quiet"));
        assert!(out.status.success(), "{out:?}");
        drop(q);
    }

    #[test]
    fn a_started_program_is_told_the_quiet_lock_is_held() {
        let mut c = Command::new("true");
        super::held_by_us(&mut c);
        let env: Vec<_> = c.get_envs().collect();
        let held = env.iter().find(|(k, _)| *k == "MOONGLOW_LOCKS_HELD").and_then(|(_, v)| *v);
        let held = held.unwrap().to_string_lossy().to_string();
        assert!(held.split(',').any(|l| l == "quiet"), "{held}");
    }
}
