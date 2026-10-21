//! Engine tests: run a module in the game's headless server (`nwserver`)
//! with a throwaway user directory and read what its scripts log.
//!
//! Test scripts report with `WriteTimestampedLogEntry("...")`, which lands in
//! `<userdir>/logs.0/nwserverLog1.txt`; the runner stops the server once a
//! line containing the done marker appears, or at the timeout.
//!
//! The user directory is always one the test created, never the player's
//! real NWN folder.

use std::io;
use std::path::Path;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

pub use mg_serve::server_binary;

/// What a server run logged.
#[derive(Debug, Clone)]
pub struct ServerRun {
    /// The server log (`nwserverLog1.txt`).
    pub log: String,
    /// Whether the done marker appeared before the timeout.
    pub finished: bool,
}

impl ServerRun {
    /// Every logged line containing `tag`, with the part after the tag.
    pub fn values(&self, tag: &str) -> Vec<String> {
        self.log
            .lines()
            .filter_map(|l| l.find(tag).map(|i| l[i + tag.len()..].trim().to_string()))
            .collect()
    }
}

/// The most servers a test binary runs at once. The engine tests share a
/// binary and libtest runs them on every core; each server loads a module
/// on a core of its own, and the tests wait on them by the clock.
const SERVERS_AT_ONCE: usize = 8;

static RUNNING: Mutex<usize> = Mutex::new(0);
static A_SERVER_ENDED: Condvar = Condvar::new();

/// One of the [`SERVERS_AT_ONCE`] places, held while a server runs.
struct Turn;

impl Turn {
    fn take() -> Turn {
        let mut running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner);
        while *running >= SERVERS_AT_ONCE {
            running = A_SERVER_ENDED.wait(running).unwrap_or_else(PoisonError::into_inner);
        }
        *running += 1;
        Turn
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        *RUNNING.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
        A_SERVER_ENDED.notify_one();
    }
}

/// For a test that has another program start the server (`mg serve`, `mg
/// test`) and waits on it by the clock: holds the others of its binary
/// back until the returned guard is dropped.
pub fn one_at_a_time() -> MutexGuard<'static, ()> {
    static ONE: Mutex<()> = Mutex::new(());
    ONE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs `<user_dir>/modules/<module>.mod`
/// in the headless server until a log line contains `done_marker`, the
/// server shuts down or `timeout` passes.
pub fn run_server(
    root: &Path,
    user_dir: &Path,
    module: &str,
    done_marker: &str,
    timeout: Duration,
) -> io::Result<ServerRun> {
    let server = server_binary(root)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, mg_serve::no_server(root)))?;
    // (Before the clock starts: a test waiting its turn is not timing out.)
    let _turn = Turn::take();
    // Never listed publicly, and locked against anyone who finds the port
    // anyway: what `mg_serve` does unless asked otherwise.
    let server = mg_serve::Server::start(&mg_serve::Options {
        server,
        user_dir: user_dir.to_path_buf(),
        module: module.to_string(),
        name: "moonglow-test".into(),
        ..Default::default()
    })?;

    let start = Instant::now();
    loop {
        let log = server.log();
        if log.contains(done_marker) {
            return Ok(ServerRun { log, finished: true });
        }
        // It quit (a module it can't load): nothing more will come.
        if start.elapsed() > timeout || log.contains("Server shutting down") {
            return Ok(ServerRun { log, finished: false });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Runs `<user_dir>/modules/<module>.mod` in the headless server until a
/// log line contains `ready_marker`, then saves the game from the server's
/// console (`save <slot> <name>`): what the engine holds, read back where
/// no script can ask for it. The saved game's `.sav` (an ERF holding the
/// areas' ARE and GIT as the engine writes them); an error with what the
/// server printed and logged if it did not get to the marker or the save
/// did not complete in `timeout`.
pub fn save_game(
    root: &Path,
    user_dir: &Path,
    module: &str,
    ready_marker: &str,
    timeout: Duration,
) -> io::Result<std::path::PathBuf> {
    let server = server_binary(root)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, mg_serve::no_server(root)))?;
    let _turn = Turn::take();
    let saves = user_dir.join("saves");
    let _ = std::fs::remove_dir_all(&saves);
    let mut server = mg_serve::Server::start(&mg_serve::Options {
        server,
        user_dir: user_dir.to_path_buf(),
        module: module.to_string(),
        name: "moonglow-test".into(),
        console: true,
        ..Default::default()
    })?;
    // What it prints, line by line (and read, or the server blocks once
    // its pipe is full).
    let (said, lines) = std::sync::mpsc::channel();
    if let Some(out) = server.console() {
        std::thread::spawn(move || {
            for line in io::BufRead::lines(io::BufReader::new(out)).map_while(Result::ok) {
                let _ = said.send(line);
            }
        });
    }
    let mut printed = Vec::new();
    let failed = |printed: &[String], log: String, what: &str| {
        let printed = printed.join("\n");
        io::Error::other(format!("{what}; the server printed:\n{printed}\nand logged:\n{log}"))
    };
    let start = Instant::now();
    while !server.log().contains(ready_marker) {
        printed.extend(lines.try_iter());
        if start.elapsed() > timeout {
            return Err(failed(&printed, server.log(), "the module did not get ready"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    // (A command typed while the module is still being set up is not
    // taken: typed again until the server says it is saving.)
    let mut complete = false;
    let mut saving = false;
    let mut asked = None::<Instant>;
    while start.elapsed() < timeout && !complete {
        if !saving && asked.is_none_or(|at| at.elapsed() > Duration::from_secs(3)) {
            server.say("save 2 mgsave")?;
            asked = Some(Instant::now());
        }
        let Ok(line) = lines.recv_timeout(Duration::from_millis(200)) else { continue };
        saving |= line.contains("Saving");
        complete = line.contains("Save complete");
        printed.push(line);
    }
    let log = server.log();
    server.stop(Duration::from_secs(10));
    if !complete {
        return Err(failed(&printed, log, "the save did not complete"));
    }
    Ok(saves.join("000002 - mgsave").join(format!("{module}.sav")))
}
