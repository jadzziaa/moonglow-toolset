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
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

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

/// The headless server binary for this platform in a game install.
pub fn server_binary(root: &Path) -> Option<PathBuf> {
    let rel = if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "bin/linux-arm64/nwserver-linux"
        } else {
            "bin/linux-x86/nwserver-linux"
        }
    } else if cfg!(target_os = "macos") {
        "bin/macos/nwserver-macos"
    } else {
        "bin/win32/nwserver.exe"
    };
    Some(root.join(rel)).filter(|p| p.is_file())
}

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Runs `<user_dir>/modules/<module>.mod` (or a module folder of that name)
/// in the headless server until a log line contains `done_marker`, the
/// server shuts down or `timeout` passes.
pub fn run_server(
    root: &Path,
    user_dir: &Path,
    module: &str,
    done_marker: &str,
    timeout: Duration,
) -> io::Result<ServerRun> {
    let server = server_binary(root).ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "no nwserver in the game install")
    })?;
    // A free port, so tests can run in parallel.
    let port = UdpSocket::bind("127.0.0.1:0")?.local_addr()?.port();
    let log_path = user_dir.join("logs.0").join("nwserverLog1.txt");
    let _ = std::fs::remove_file(&log_path);
    // Never list test servers publicly, and lock them against anyone who
    // finds the port anyway.
    let secret = format!(
        "mg{:x}{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
            ^ u128::from(port)
    );

    let child = Command::new(&server)
        .current_dir(server.parent().unwrap())
        .arg("-userdirectory")
        .arg(user_dir)
        .args(["-module", module, "-port", &port.to_string(), "-quiet"])
        .args(["-publicserver", "0", "-servername", "moonglow-test"])
        .args(["-playerpassword", &secret, "-dmpassword", &secret, "-adminpassword", &secret])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let _guard = KillOnDrop(child);

    let start = Instant::now();
    loop {
        // The log is in the game's codepage, not UTF-8.
        let log = std::fs::read(&log_path)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
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
