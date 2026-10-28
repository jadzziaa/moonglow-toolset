//! Runs a module in the game's headless server (`nwserver`): a user
//! directory of the caller's choosing, a free port, the log followed, and
//! the server's console on a pipe when asked.
//!
//! A server started here is private unless the caller says otherwise:
//! `-publicserver 0` (without it the server registers on Beamdog's public
//! list under the user's address) and passwords nobody was told.
//!
//! What a module's scripts say with `WriteTimestampedLogEntry("...")` lands
//! in `<userdir>/logs.0/nwserverLog1.txt`. That log is how a runner knows
//! the module is loaded: the server's port answers queries about a second
//! after it starts, before any module is.

use std::ffi::OsString;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

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

/// Why there is no server to run, in words for the user.
pub fn no_server(root: &Path) -> String {
    if cfg!(target_os = "macos") {
        "the game has no headless server for macOS (nwserver is for Linux and Windows)".into()
    } else {
        format!("no nwserver in the game install at {}", root.display())
    }
}

/// How to start a server.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The server's program ([`server_binary`]).
    pub server: PathBuf,
    /// The user directory it runs on (`-userdirectory`): its `modules`
    /// holds the module, its `logs.0` gets the logs.
    pub user_dir: PathBuf,
    /// The module's name, without `.mod`. (The server loads an archive:
    /// it refuses a module kept as a folder.)
    pub module: String,
    /// The port; `None`: a free one.
    pub port: Option<u16>,
    /// Listed on the public server list. Off unless asked.
    pub public: bool,
    /// The server's name, as players see it.
    pub name: String,
    /// The player, DM and admin passwords; one not given is made up, so
    /// nobody who finds the port gets in.
    pub player_password: Option<String>,
    pub dm_password: Option<String>,
    pub admin_password: Option<String>,
    /// The server's console on pipes (`-interactive`): what it prints is
    /// read with [`Server::console`], and [`Server::say`] types into it.
    /// Otherwise it runs with `-quiet`.
    pub console: bool,
    /// More of the environment (NWNX's settings, say).
    pub env: Vec<(OsString, OsString)>,
    /// More arguments for the server, after Moonglow's.
    pub args: Vec<OsString>,
    /// In a process group of its own (a console group, on Windows): the
    /// terminal's Ctrl+C reaches the caller alone, which stops the server
    /// itself (with `exit`, [`Server::stop`]). Only for a caller that
    /// catches Ctrl+C: one that doesn't would leave the server running.
    pub own_group: bool,
}

/// What the server is started with.
fn arguments(o: &Options, port: u16, passwords: [&str; 3]) -> Vec<OsString> {
    let mut out: Vec<OsString> = vec!["-userdirectory".into(), o.user_dir.clone().into()];
    let [player, dm, admin] = passwords;
    let rest = [
        "-module",
        &o.module,
        "-port",
        &port.to_string(),
        if o.console { "-interactive" } else { "-quiet" },
        "-publicserver",
        if o.public { "1" } else { "0" },
        "-servername",
        &o.name,
        "-playerpassword",
        player,
        "-dmpassword",
        dm,
        "-adminpassword",
        admin,
    ];
    out.extend(rest.iter().map(Into::into));
    out.extend(o.args.iter().cloned());
    out
}

/// A running server; stopped when dropped.
#[derive(Debug)]
pub struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    pub port: u16,
    pub user_dir: PathBuf,
    pub player_password: String,
    pub dm_password: String,
    pub admin_password: String,
}

/// A password nobody was told.
fn secret(port: u16, salt: u32) -> String {
    use std::hash::{BuildHasher, Hasher};
    // (Keyed at random by the standard library, for each process.)
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u32(std::process::id());
    h.write_u16(port);
    h.write_u32(salt);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    h.write_u128(now.map_or(0, |d| d.as_nanos()));
    format!("mg{:016x}", h.finish())
}

impl Server {
    /// Starts the server. Its logs of an earlier run in the same user
    /// directory are removed first, so that what is read is this run's.
    pub fn start(o: &Options) -> io::Result<Server> {
        let port = match o.port {
            Some(port) => port,
            // A free port, so servers can run side by side.
            None => UdpSocket::bind("127.0.0.1:0")?.local_addr()?.port(),
        };
        let logs = o.user_dir.join("logs.0");
        for log in ["nwserverLog1.txt", "nwserverError1.txt", "nwengineLog.txt"] {
            let _ = std::fs::remove_file(logs.join(log));
        }
        let or_secret =
            |given: &Option<String>, salt| given.clone().unwrap_or_else(|| secret(port, salt));
        let (player, dm, admin) = (
            or_secret(&o.player_password, 1),
            or_secret(&o.dm_password, 2),
            or_secret(&o.admin_password, 3),
        );
        let mut command = Command::new(&o.server);
        if let Some(dir) = o.server.parent() {
            command.current_dir(dir);
        }
        command.args(arguments(o, port, [&player, &dm, &admin]));
        command.envs(o.env.iter().map(|(k, v)| (k, v)));
        let pipe = |on: bool| if on { Stdio::piped() } else { Stdio::null() };
        command.stdin(pipe(o.console)).stdout(pipe(o.console)).stderr(Stdio::null());
        if o.own_group {
            #[cfg(unix)]
            std::os::unix::process::CommandExt::process_group(&mut command, 0);
            #[cfg(windows)]
            {
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
                std::os::windows::process::CommandExt::creation_flags(
                    &mut command,
                    CREATE_NEW_PROCESS_GROUP,
                );
            }
        }
        let mut child = command.spawn()?;
        Ok(Server {
            stdin: child.stdin.take(),
            stdout: child.stdout.take(),
            child,
            port,
            user_dir: o.user_dir.clone(),
            player_password: player,
            dm_password: dm,
            admin_password: admin,
        })
    }

    /// The server's log, where scripts' `WriteTimestampedLogEntry` goes.
    pub fn log_path(&self) -> PathBuf {
        self.user_dir.join("logs.0").join("nwserverLog1.txt")
    }

    /// The engine's own log (levels, source lines: why a module did not
    /// load).
    pub fn engine_log_path(&self) -> PathBuf {
        self.user_dir.join("logs.0").join("nwengineLog.txt")
    }

    /// The server's log so far. (It is in the game's codepage, not UTF-8.)
    pub fn log(&self) -> String {
        std::fs::read(self.log_path())
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default()
    }

    /// What the server prints on its console, to be read on a thread of
    /// the caller's (once; `None` without a console).
    pub fn console(&mut self) -> Option<ChildStdout> {
        self.stdout.take()
    }

    /// Types a line into the server's console (`status`, `exit`, NWNX's
    /// `runscript`…).
    pub fn say(&mut self, line: &str) -> io::Result<()> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(io::Error::other("the server was started without a console"));
        };
        stdin.write_all(line.trim_end_matches(['\r', '\n']).as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()
    }

    /// Whether the server has ended, and how.
    pub fn exited(&mut self) -> Option<ExitStatus> {
        self.child.try_wait().ok().flatten()
    }

    /// The server's process id.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Stops the server: asked to on its console where it has one (so it
    /// closes its database and logs), and killed if it has not ended in
    /// `patience`.
    pub fn stop(mut self, patience: Duration) -> Option<ExitStatus> {
        if self.say("exit").is_ok() {
            let asked = Instant::now();
            while asked.elapsed() < patience {
                if let Some(status) = self.exited() {
                    return Some(status);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        let _ = self.child.kill();
        self.child.wait().ok()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Follows a log file as it grows: the whole lines added since the last
/// look.
#[derive(Debug)]
pub struct Tail {
    path: PathBuf,
    read: u64,
    partial: Vec<u8>,
}

impl Tail {
    pub fn new(path: impl Into<PathBuf>) -> Tail {
        Tail { path: path.into(), read: 0, partial: Vec::new() }
    }

    /// The lines added since the last look (none while the file isn't
    /// there). A file that became shorter is another file: read from its
    /// start.
    pub fn more(&mut self) -> Vec<String> {
        let Ok(mut file) = std::fs::File::open(&self.path) else { return Vec::new() };
        let len = file.metadata().map_or(0, |m| m.len());
        if len < self.read {
            (self.read, self.partial) = (0, Vec::new());
        }
        if len == self.read || file.seek(SeekFrom::Start(self.read)).is_err() {
            return Vec::new();
        }
        let mut added = Vec::new();
        let Ok(n) = file.take(len - self.read).read_to_end(&mut added) else { return Vec::new() };
        self.read += n as u64;
        self.partial.extend_from_slice(&added);
        let Some(end) = self.partial.iter().rposition(|b| *b == b'\n') else { return Vec::new() };
        let rest = self.partial.split_off(end + 1);
        let whole = std::mem::replace(&mut self.partial, rest);
        String::from_utf8_lossy(&whole)
            .lines()
            .map(|l| l.trim_end_matches('\r').to_string())
            .collect()
    }
}

/// The settings of an environment file: `NAME=value` lines, `#` comments,
/// blank lines, an optional `export ` before a name, a value in single or
/// double quotes taken without them. Nothing is expanded.
pub fn env_file(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").map_or(line, str::trim_start);
        let Some((name, value)) = line.split_once('=') else {
            return Err(format!("line {}: no NAME=value: {line}", i + 1));
        };
        let (name, value) = (name.trim(), value.trim());
        let plain = |c: char| c.is_ascii_alphanumeric() || c == '_';
        if name.is_empty() || !name.chars().all(plain) {
            return Err(format!("line {}: not a name: {name}", i + 1));
        }
        let quoted = |q: char| value.len() >= 2 && value.starts_with(q) && value.ends_with(q);
        let value = match quoted('"') || quoted('\'') {
            true => &value[1..value.len() - 1],
            false => value,
        };
        out.push((name.to_string(), value.to_string()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tail_gives_whole_lines_as_they_come() {
        let dir = std::env::temp_dir().join(format!("mg-serve-tail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("log.txt");
        let mut tail = Tail::new(&path);
        assert!(tail.more().is_empty(), "no file yet");
        std::fs::write(&path, "one\r\ntwo\nthr").unwrap();
        assert_eq!(tail.more(), ["one", "two"]);
        assert!(tail.more().is_empty());
        std::fs::write(&path, "one\r\ntwo\nthree\nfour\n").unwrap();
        assert_eq!(tail.more(), ["three", "four"]);
        // Written anew (a restarted server): read from its start.
        std::fs::write(&path, "again\n").unwrap();
        assert_eq!(tail.more(), ["again"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_environment_file_is_names_and_values() {
        let text = "# NWNX\nNWNX_CORE_LOG_LEVEL=7\nexport A_B = \"two words\"\n\nC='x=y'\nD=\n";
        let read = env_file(text).unwrap();
        let pairs: Vec<(&str, &str)> = read.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(
            pairs,
            [("NWNX_CORE_LOG_LEVEL", "7"), ("A_B", "two words"), ("C", "x=y"), ("D", "")]
        );
        assert!(env_file("just words").unwrap_err().contains("line 1"));
        assert!(env_file("ok=1\nbad name=2").unwrap_err().contains("line 2"));
    }

    #[test]
    fn a_server_is_private_unless_asked() {
        let text = |o: &Options| {
            let args = arguments(o, 5121, ["p", "d", "a"]);
            args.iter().map(|a| a.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" ")
        };
        let mut o = Options { module: "m".into(), ..Default::default() };
        assert!(text(&o).contains("-publicserver 0"), "{}", text(&o));
        assert!(text(&o).contains("-quiet") && !text(&o).contains("-interactive"));
        assert!(text(&o).contains("-playerpassword p -dmpassword d -adminpassword a"));
        (o.public, o.console) = (true, true);
        o.args = vec!["-maxclients".into(), "2".into()];
        assert!(text(&o).contains("-publicserver 1") && text(&o).contains("-interactive"));
        assert!(text(&o).ends_with("-maxclients 2"));
    }

    #[test]
    fn passwords_not_given_are_made_up_and_differ() {
        assert_ne!(secret(5121, 1), secret(5121, 1));
        assert!(secret(5121, 1).len() > 12);
    }
}
