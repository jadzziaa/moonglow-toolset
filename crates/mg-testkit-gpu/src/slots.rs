//! The GPU slots of the machine: the cap on open devices across test
//! binaries.
//!
//! [`crate::gpu`] caps the devices of one process. Several test binaries (and
//! several worktrees' test runs) at once would add those caps up, so each
//! test also takes one slot of the machine: an advisory lock (`flock(2)`;
//! `std::fs::File::lock`, the same on Windows and macOS) on one of the files
//! `gpu/0` to `gpu/N-1` in [`folder`], N being [`crate::gpu::places`]. A
//! slot belongs to the open file, so it ends with the test, or with the
//! process if that is killed. Any other program may take the same files the
//! same way (`docs/locks.md`).
//!
//! The files are made when first needed and never deleted: deleting a lock
//! file would split it into two locks. A lock cannot wait for "any of N", so
//! a taker tries the slots and polls while they are all held.
//!
//! Where the folder cannot be made or opened (no home, a read-only disk), the
//! hold says so once and goes on with the process's own places alone: a
//! test never fails for want of a slot.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::{Duration, Instant};

/// How long a taker waits before it says so, and how often it repeats it.
pub(crate) const SAY_AFTER: Duration = Duration::from_secs(10);
pub(crate) const SAY_EVERY: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(50);

/// A machine slot, given back when dropped.
#[derive(Debug)]
pub(crate) struct Slot {
    file: File,
}

impl Drop for Slot {
    fn drop(&mut self) {
        // Unlocked before the file is closed: a child spawned on another
        // thread can hold a copy of the descriptor until its exec, and the
        // lock belongs to the open file, not to this descriptor.
        let _ = self.file.unlock();
    }
}

/// The folder of the slot files: `$XDG_STATE_HOME/moonglow/locks/gpu`, else
/// `~/.local/state/moonglow/locks/gpu` (on Windows, under `%LOCALAPPDATA%`
/// without a home). Not moved by `MOONGLOW_ORACLE`: the GPU belongs to the
/// machine. `None` where there is no such place.
pub fn folder() -> Option<PathBuf> {
    let set = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let state = set("XDG_STATE_HOME")
        .or_else(|| set("HOME").map(|h| h.join(".local/state")))
        .or_else(|| set("LOCALAPPDATA"))?;
    Some(state.join("moonglow/locks/gpu"))
}

/// Writes a line to the standard error stream the way libtest lets through:
/// its output capture takes only the print macros, so a test's waiting line
/// would otherwise be hidden until the test ended.
pub(crate) fn say(line: &str) {
    let _ = io::stderr().write_all(format!("{line}\n").as_bytes());
}

/// Takes a free slot of `0..n` in `dir` if there is one.
pub(crate) fn try_take_in(dir: &Path, n: usize) -> io::Result<Option<Slot>> {
    std::fs::create_dir_all(dir)?;
    for i in 0..n {
        let file = open(dir, i)?;
        match file.try_lock() {
            Ok(()) => return Ok(Some(Slot { file })),
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(e)) => return Err(e),
        }
    }
    Ok(None)
}

fn open(dir: &Path, i: usize) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join(i.to_string()))
}

/// Takes a slot of `0..n` in `dir`, waiting while they are all held; after
/// `first` it says so through `say`, and again every `every`.
pub(crate) fn take_in(
    dir: &Path,
    n: usize,
    first: Duration,
    every: Duration,
    say: &mut dyn FnMut(&str),
) -> io::Result<Slot> {
    let start = Instant::now();
    let mut next = first;
    loop {
        if let Some(slot) = try_take_in(dir, n)? {
            return Ok(slot);
        }
        if start.elapsed() >= next {
            next = start.elapsed() + every;
            let pids = holders(dir, n);
            let held = if pids.is_empty() {
                format!("{n} held")
            } else {
                let pids: Vec<String> = pids.iter().map(u32::to_string).collect();
                format!("{n} held: PIDs {}", pids.join(", "))
            };
            say(&format!("waiting for a GPU slot ({held})"));
        }
        std::thread::sleep(POLL);
    }
}

/// The processes holding slots `0..n` of `dir`, from `/proc/locks` on Linux
/// (matched by the file's inode); none elsewhere or where it can't be read.
fn holders(dir: &Path, n: usize) -> Vec<u32> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let inodes: Vec<u64> = (0..n)
            .filter_map(|i| std::fs::metadata(dir.join(i.to_string())).ok())
            .map(|m| m.ino())
            .collect();
        let Ok(locks) = std::fs::read_to_string("/proc/locks") else { return Vec::new() };
        let mut pids: Vec<u32> = Vec::new();
        for line in locks.lines() {
            // `1: FLOCK  ADVISORY  WRITE 1234 08:02:5678 0 EOF` (a waiting
            // taker has `->` before FLOCK: not a holder).
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() < 6 || f[1] != "FLOCK" {
                continue;
            }
            let on_ours = f[5]
                .rsplit(':')
                .next()
                .and_then(|i| i.parse::<u64>().ok())
                .is_some_and(|i| inodes.contains(&i));
            if let (true, Ok(pid)) = (on_ours, f[4].parse::<u32>())
                && !pids.contains(&pid)
            {
                pids.push(pid);
            }
        }
        pids.sort_unstable();
        pids
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (dir, n);
        Vec::new()
    }
}

/// A slot for the calling test: waiting while all `n` are held, with the
/// waiting lines on standard error. `None` (said once) where the folder
/// can't be used.
pub(crate) fn take(n: usize) -> Option<Slot> {
    static SAID: Once = Once::new();
    let unusable = |why: String| {
        SAID.call_once(|| {
            say(&format!(
                "no GPU slots of the machine ({why}): only this process's places are kept"
            ));
        });
        None
    };
    let Some(dir) = folder() else {
        return unusable("no home or state folder".into());
    };
    match take_in(&dir, n, SAY_AFTER, SAY_EVERY, &mut |l| say(l)) {
        Ok(slot) => Some(slot),
        Err(e) => unusable(format!("{}: {e}", dir.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::time::Duration;

    use super::{holders, take_in, try_take_in};

    const CHILD: &str = "MG_SLOTS_TEST_CHILD";

    /// This test binary run again as a separate process that takes `taking`
    /// slots of `n` in `dir`, says `READY`, and holds them until its stdin
    /// closes.
    fn child(dir: &Path, n: usize, taking: usize) -> Child {
        let mut c = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "slots::tests::the_child_holds_slots", "--nocapture"])
            .env(CHILD, format!("{}|{n}|{taking}", dir.display()))
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
        // (Read on to its end: a closed pipe would fail the child's own
        // test report.)
        std::thread::spawn(move || for _ in out.lines() {});
        c
    }

    fn end(mut c: Child) {
        drop(c.stdin.take());
        assert!(c.wait().unwrap().success());
    }

    /// The child's side: not a test of its own where the variable is unset.
    #[test]
    fn the_child_holds_slots() {
        let Ok(spec) = std::env::var(CHILD) else { return };
        let mut p = spec.split('|');
        let dir = PathBuf::from(p.next().unwrap());
        let n: usize = p.next().unwrap().parse().unwrap();
        let taking: usize = p.next().unwrap().parse().unwrap();
        let _slots: Vec<_> =
            (0..taking).map(|_| try_take_in(&dir, n).unwrap().expect("a free slot")).collect();
        println!("READY");
        let _ = std::io::stdout().flush();
        let mut rest = String::new();
        let _ = std::io::stdin().read_line(&mut rest);
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mg-slots-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn slots_cap_across_processes_and_a_waiter_gets_one_when_they_end() {
        let dir = scratch("cap");
        let held = child(&dir, 3, 3);
        assert!(try_take_in(&dir, 3).unwrap().is_none(), "all three are another process's");
        // A process using fewer slots (MG_TEST_GPUS=1) sees its own subset.
        assert!(try_take_in(&dir, 1).unwrap().is_none());
        let pid = held.id();
        let waiter = {
            let dir = dir.clone();
            std::thread::spawn(move || {
                let mut said = Vec::new();
                let slot = take_in(
                    &dir,
                    3,
                    Duration::from_millis(100),
                    Duration::from_millis(100),
                    &mut |l| said.push(l.to_string()),
                )
                .unwrap();
                (slot, said)
            })
        };
        std::thread::sleep(Duration::from_millis(450));
        if cfg!(target_os = "linux") {
            assert_eq!(holders(&dir, 3), vec![pid]);
        }
        end(held);
        let (slot, said) = waiter.join().unwrap();
        assert!(said.len() >= 2, "said at first and again: {said:?}");
        assert!(said[0].starts_with("waiting for a GPU slot (3 held"), "{said:?}");
        if cfg!(target_os = "linux") {
            assert!(said[0].contains(&format!("PIDs {pid}")), "{said:?}");
        }
        drop(slot);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_process_holding_some_leaves_the_rest_and_a_given_back_slot_is_free_again() {
        let dir = scratch("some");
        let held = child(&dir, 3, 2);
        let mine = try_take_in(&dir, 3).unwrap().expect("the third is free");
        assert!(try_take_in(&dir, 3).unwrap().is_none());
        drop(mine);
        let again = try_take_in(&dir, 3).unwrap();
        assert!(again.is_some(), "a slot given back is taken again");
        drop(again);
        end(held);
        assert!(try_take_in(&dir, 3).unwrap().is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_that_cannot_be_made_is_an_error_not_a_panic() {
        let dir = scratch("file");
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = dir.join("gpu");
        std::fs::write(&blocker, b"a file where the folder should be").unwrap();
        assert!(try_take_in(&blocker, 2).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
