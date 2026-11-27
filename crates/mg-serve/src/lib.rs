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

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

pub mod cdkey;

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
    /// read with [`Server::lines`], and [`Server::say`] types into it.
    /// Otherwise it runs with `-quiet`.
    pub console: bool,
    /// More of the environment (NWNX's settings, say).
    pub env: Vec<(OsString, OsString)>,
    /// The server gets `env` and a few of the caller's own variables
    /// alone ([`KEPT_VARIABLES`]): none of the rest (a builder's own
    /// settings, an NWNX setting or a webhook's address of their shell's).
    pub clean_env: bool,
    /// More arguments for the server, after Moonglow's.
    pub args: Vec<OsString>,
    /// In a process group of its own (a console group, on Windows): the
    /// terminal's Ctrl+C reaches the caller alone, which stops the server
    /// itself (with `exit`, [`Server::stop`]). Only for a caller that
    /// catches Ctrl+C: one that doesn't would leave the server running.
    pub own_group: bool,
    /// Killed when the thread that starts it ends, on Linux
    /// ([`die_with_parent`]): for a caller that starts it from a thread
    /// living as long as the server should (Test Module's local run), so
    /// that it dies with the caller however the caller ends.
    pub die_with_parent: bool,
}

/// What a server started with [`Options::clean_env`] keeps of the
/// caller's environment: where programs and the home are, the user, the
/// language and time zone, the temporary folder (and Windows' own).
pub const KEPT_VARIABLES: [&str; 16] = [
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "LANG",
    "LANGUAGE",
    "TZ",
    "TMPDIR",
    "SYSTEMROOT",
    "WINDIR",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "COMSPEC",
];

/// Whether a variable of the caller's is kept ([`KEPT_VARIABLES`], and
/// the locale's `LC_*`).
fn kept(name: &std::ffi::OsStr) -> bool {
    let name = name.to_string_lossy();
    KEPT_VARIABLES.iter().any(|k| k.eq_ignore_ascii_case(&name)) || name.starts_with("LC_")
}

/// Has the program `command` starts killed (`SIGKILL`) when the thread
/// that started it ends, however it ends (a crash, a kill of the whole
/// process): Linux's parent-death signal (`PR_SET_PDEATHSIG`). It is the
/// starting *thread*'s death that counts, not the process's: the program
/// is to be started from a thread that lives as long as the program
/// should. Nothing on another system.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
pub fn die_with_parent(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    let parent = rustix::process::getpid();
    // SAFETY: the closure runs in the child between fork and exec, where
    // only async-signal-safe calls are allowed: it makes two system calls
    // (prctl, getppid) and allocates nothing.
    unsafe {
        command.pre_exec(move || {
            let kill = Some(rustix::process::Signal::KILL);
            rustix::process::set_parent_process_death_signal(kill)?;
            // (The parent gone before the signal was asked for: the child
            // was given to another, and no signal will come.)
            if rustix::process::getppid() != Some(parent) {
                return Err(io::Error::other("the program's parent ended as it started"));
            }
            Ok(())
        });
    }
}

/// [`die_with_parent`] off Linux: nothing.
#[cfg(not(target_os = "linux"))]
pub fn die_with_parent(_command: &mut Command) {}

/// Whether a line the server printed says its module is loaded: the one
/// reading of it that `mg serve`, Test Module and [`Server::wait_ready`]
/// share.
pub fn loaded(line: &str) -> bool {
    line.starts_with("Server: Module loaded")
}

/// Whether a line the server printed says it could not load its module.
pub fn not_loaded(line: &str) -> bool {
    line.contains("Unable to load module")
}

/// A line the server printed, on its console or (`stderr`) as an error:
/// the loader's own, where a preloaded library could not be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub stderr: bool,
}

/// Why [`Server::wait_ready`] gave up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotReady {
    /// The server ended before its module loaded; what it printed last.
    Exited { last_lines: Vec<String> },
    /// The module had not loaded in the time given.
    TimedOut,
    /// The caller asked to stop waiting.
    Cancelled,
}

/// How many of the server's last lines [`NotReady::Exited`] keeps.
const LAST_LINES: usize = 20;

/// Reads a pipe's lines into `send` on a thread of its own, until it
/// closes (so the server never blocks on a full pipe).
fn read_lines(pipe: impl Read + Send + 'static, stderr: bool, send: Sender<Line>) {
    std::thread::spawn(move || {
        for line in io::BufReader::new(pipe).split(b'\n').map_while(Result::ok) {
            let text = String::from_utf8_lossy(&line).trim_end_matches('\r').to_string();
            if send.send(Line { text, stderr }).is_err() {
                break;
            }
        }
    });
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
    /// What it prints (its console and its errors), where it has a
    /// console.
    lines: Option<Receiver<Line>>,
    /// Lines read while waiting for it, not yet taken by [`Server::lines`].
    kept: VecDeque<Line>,
    started: Instant,
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
        if o.clean_env {
            let kept: Vec<_> = std::env::vars_os().filter(|(k, _)| kept(k)).collect();
            command.env_clear().envs(kept);
        }
        command.envs(o.env.iter().map(|(k, v)| (k, v)));
        let pipe = |on: bool| if on { Stdio::piped() } else { Stdio::null() };
        // (With a console, its errors too: the loader says there why a
        // preloaded library was not loaded.)
        command.stdin(pipe(o.console)).stdout(pipe(o.console)).stderr(pipe(o.console));
        if o.die_with_parent {
            die_with_parent(&mut command);
        }
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
        let lines = o.console.then(|| {
            let (send, receive) = channel();
            if let Some(out) = child.stdout.take() {
                read_lines(out, false, send.clone());
            }
            if let Some(err) = child.stderr.take() {
                read_lines(err, true, send);
            }
            receive
        });
        Ok(Server {
            stdin: child.stdin.take(),
            lines,
            kept: VecDeque::new(),
            started: Instant::now(),
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

    /// What the server printed since the last look (its console and its
    /// errors, in the order read; none without a console), those
    /// [`Server::wait_ready`] read first.
    pub fn lines(&mut self) -> Vec<Line> {
        let mut out: Vec<Line> = self.kept.drain(..).collect();
        if let Some(lines) = &self.lines {
            out.extend(lines.try_iter());
        }
        out
    }

    /// Waits until the server says its module is loaded ([`loaded`]),
    /// for at most `within`, or until `cancel` is set: how long after its
    /// start it loaded. The lines read meanwhile stay to be read with
    /// [`Server::lines`]. Needs the console ([`Options::console`]).
    pub fn wait_ready(
        &mut self,
        within: Duration,
        cancel: &AtomicBool,
    ) -> Result<Duration, NotReady> {
        let waited = Instant::now();
        let mut last: VecDeque<String> = VecDeque::new();
        let mut seen = 0;
        loop {
            let ended = self.exited().is_some();
            if let Some(lines) = &self.lines {
                self.kept.extend(lines.try_iter());
            }
            for line in self.kept.iter().skip(seen) {
                if loaded(&line.text) {
                    return Ok(self.started.elapsed());
                }
                last.push_back(line.text.clone());
                if last.len() > LAST_LINES {
                    last.pop_front();
                }
            }
            seen = self.kept.len();
            if ended {
                // (What it printed as it ended, read once its pipes close.)
                std::thread::sleep(Duration::from_millis(100));
                if let Some(lines) = &self.lines {
                    for line in lines.try_iter() {
                        if loaded(&line.text) {
                            self.kept.push_back(line);
                            return Ok(self.started.elapsed());
                        }
                        last.push_back(line.text.clone());
                        self.kept.push_back(line);
                    }
                }
                let skip = last.len().saturating_sub(LAST_LINES);
                return Err(NotReady::Exited { last_lines: last.into_iter().skip(skip).collect() });
            }
            if cancel.load(Ordering::SeqCst) {
                return Err(NotReady::Cancelled);
            }
            if waited.elapsed() >= within {
                return Err(NotReady::TimedOut);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
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
    /// (What it printed meanwhile stays to be read with [`Server::lines`].)
    pub fn stop(&mut self, patience: Duration) -> Option<ExitStatus> {
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

/// Puts a player's character, the file `bic`, as the only one in their
/// folder of the server vault of `user_dir` (`servervault/<public_key>/`,
/// [`cdkey::public_key`]), where a server started with `-servervault 1`
/// gives it to them: the folder's other files are taken away. For a
/// server's folder of Moonglow's own: never the player's. The character's
/// path there.
pub fn place_in_vault(user_dir: &Path, public_key: &str, bic: &Path) -> io::Result<PathBuf> {
    let plain = |c: char| c.is_ascii_alphanumeric();
    if public_key.len() != 8 || !public_key.chars().all(plain) {
        return Err(io::Error::other("not a public CD key"));
    }
    let Some(name) = bic.file_name() else {
        return Err(io::Error::other(format!("{}: not a character's file", bic.display())));
    };
    let dir = user_dir.join("servervault").join(public_key);
    std::fs::create_dir_all(&dir)?;
    for e in std::fs::read_dir(&dir)?.flatten() {
        if e.file_type().is_ok_and(|t| !t.is_dir()) {
            std::fs::remove_file(e.path())?;
        }
    }
    let to = dir.join(name);
    std::fs::copy(bic, &to)?;
    Ok(to)
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

/// The settings of an environment file ([`mg_core::env_file::parse`]).
pub use mg_core::env_file::parse as env_file;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tail_gives_whole_lines_as_they_come() {
        let dir = mg_testkit::scratch_dir("mg-serve-tail");
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

    /// A character in the vault: the one file of the player's folder.
    #[test]
    fn a_character_is_placed_alone_in_the_vault() {
        let dir = mg_testkit::scratch_dir("mg-serve-vault");
        std::fs::create_dir_all(&dir).unwrap();
        let bic = dir.join("hero.bic");
        std::fs::write(&bic, b"BIC V3.2").unwrap();
        let user = dir.join("server");
        let old = user.join("servervault/UANPXTFW/old.bic");
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        std::fs::write(&old, b"old").unwrap();
        let to = place_in_vault(&user, "UANPXTFW", &bic).unwrap();
        assert_eq!(to, user.join("servervault/UANPXTFW/hero.bic"));
        assert_eq!(std::fs::read(&to).unwrap(), b"BIC V3.2");
        let names: Vec<_> = std::fs::read_dir(to.parent().unwrap()).unwrap().flatten().collect();
        assert_eq!(names.len(), 1);
        assert!(bic.is_file(), "the player's file is only read");
        assert!(place_in_vault(&user, "../x", &bic).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A fake server: a shell script that prints `said` (its console)
    /// and `errors` (its standard error), then waits `stay` seconds.
    #[cfg(unix)]
    fn fake(dir: &Path, said: &str, errors: &str, stay: u32) -> Options {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(dir).unwrap();
        let script = dir.join("nwserver-fake");
        let text = format!(
            "#!/bin/sh\nenv > \"{}\"\nprintf '{said}'\nprintf '{errors}' >&2\nsleep {stay}\n",
            dir.join("env.txt").display()
        );
        std::fs::write(&script, text).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        Options {
            server: script,
            user_dir: dir.join("user"),
            module: "m".into(),
            console: true,
            ..Default::default()
        }
    }

    /// `wait_ready` hears the module load (the lines before it kept for
    /// the caller, the loader's error among them), gives the last lines of
    /// a server that ended first, times out and is cancelled.
    #[cfg(unix)]
    #[test]
    fn a_server_is_waited_for_until_its_module_loads() {
        let dir = mg_testkit::scratch_dir("mg-serve-wait-ready");
        let never = AtomicBool::new(false);
        let said =
            "Starting NWNX 8193.37-17 [x]\\nServer: Loading module\\nServer: Module loaded\\n";
        let mut s = Server::start(&fake(&dir.join("a"), said, "ERROR: ld.so: no\\n", 5)).unwrap();
        let took = s.wait_ready(Duration::from_secs(20), &never).unwrap();
        assert!(took < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(200));
        let lines = s.lines();
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert!(texts.contains(&"Starting NWNX 8193.37-17 [x]"), "{texts:?}");
        assert!(texts.contains(&"Server: Module loaded"), "{texts:?}");
        assert!(lines.iter().any(|l| l.stderr && l.text == "ERROR: ld.so: no"), "{lines:?}");
        drop(s);
        // Ended before it loaded.
        let mut s = Server::start(&fake(&dir.join("b"), "one\\ntwo\\n", "", 0)).unwrap();
        match s.wait_ready(Duration::from_secs(20), &never) {
            Err(NotReady::Exited { last_lines }) => assert_eq!(last_lines, ["one", "two"]),
            other => panic!("{other:?}"),
        }
        // Still loading at the time given; then cancelled.
        let mut s = Server::start(&fake(&dir.join("c"), "loading\\n", "", 30)).unwrap();
        assert_eq!(s.wait_ready(Duration::from_millis(300), &never), Err(NotReady::TimedOut));
        let cancel = AtomicBool::new(true);
        assert_eq!(s.wait_ready(Duration::from_secs(20), &cancel), Err(NotReady::Cancelled));
        assert_eq!(s.lines().iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["loading"]);
        drop(s);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// With a clean environment the server gets what it is given and the
    /// few variables kept, none of the caller's own (cargo's, here; a
    /// builder's webhook address, at home).
    #[cfg(unix)]
    #[test]
    fn a_clean_environment_passes_nothing_of_the_caller_s() {
        let dir = mg_testkit::scratch_dir("mg-serve-clean-env");
        assert!(std::env::var_os("CARGO_MANIFEST_DIR").is_some(), "cargo's, in a test");
        let mut o = fake(&dir, "Server: Module loaded\\n", "", 0);
        o.clean_env = true;
        o.env = vec![("NWNX_CORE_SKIP_ALL".into(), "y".into())];
        let mut s = Server::start(&o).unwrap();
        let _ = s.wait_ready(Duration::from_secs(20), &AtomicBool::new(false));
        let env = std::fs::read_to_string(dir.join("env.txt")).unwrap();
        assert!(env.lines().any(|l| l == "NWNX_CORE_SKIP_ALL=y"), "{env}");
        assert!(env.lines().any(|l| l.starts_with("PATH=")), "{env}");
        assert!(!env.contains("CARGO_MANIFEST_DIR"), "{env}");
        // Not asked: the caller's passed on, as before.
        drop(s);
        o.clean_env = false;
        let mut s = Server::start(&o).unwrap();
        let _ = s.wait_ready(Duration::from_secs(20), &AtomicBool::new(false));
        let env = std::fs::read_to_string(dir.join("env.txt")).unwrap();
        assert!(env.contains("CARGO_MANIFEST_DIR"), "{env}");
        drop(s);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn passwords_not_given_are_made_up_and_differ() {
        assert_ne!(secret(5121, 1), secret(5121, 1));
        assert!(secret(5121, 1).len() > 12);
    }

    /// Whether a process is gone (or ended, not yet reaped).
    #[cfg(target_os = "linux")]
    fn gone(pid: u32) -> bool {
        match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Err(_) => true,
            Ok(stat) => stat.rsplit_once(") ").is_some_and(|(_, rest)| rest.starts_with('Z')),
        }
    }

    #[cfg(target_os = "linux")]
    fn within(seconds: u64, done: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < deadline {
            if done() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        done()
    }

    /// Started with [`die_with_parent`] from a thread that then ends, a
    /// program is killed: the parent-death signal is the thread's (so a
    /// local run starts its server from the thread that lives as long).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_program_dies_with_the_thread_that_started_it() {
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let mut c = Command::new("sleep");
            c.arg("60");
            die_with_parent(&mut c);
            send.send(c.spawn().unwrap()).unwrap();
        })
        .join()
        .unwrap();
        let mut child = receive.recv().unwrap();
        let ended = within(10, || gone(child.id()));
        let _ = child.kill();
        let status = child.wait().unwrap();
        assert!(ended, "the program outlived the thread that started it");
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(9), "{status:?}");
    }

    /// What `the_program_dies_with_its_process` runs in a process of its
    /// own: a `sleep` started (with [`die_with_parent`], unless
    /// `MG_DIE_PLAIN` is set), its pid written to `MG_DIE_OUT`, then
    /// waiting to be killed.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore]
    fn die_with_parent_helper() {
        let Some(out) = std::env::var_os("MG_DIE_OUT") else { return };
        let mut c = Command::new("sleep");
        c.arg("60");
        if std::env::var_os("MG_DIE_PLAIN").is_none() {
            die_with_parent(&mut c);
        }
        let mut child = c.spawn().unwrap();
        std::fs::write(&out, child.id().to_string()).unwrap();
        // (Killed before it ends: this process is.)
        let _ = child.wait();
    }

    /// The process that started a program killed outright (`kill -9`, as a
    /// crash or a session's end would): the program is killed with it.
    /// Without [`die_with_parent`] it would live on (the check shows the
    /// test can fail).
    #[cfg(target_os = "linux")]
    #[test]
    fn a_program_dies_with_its_process() {
        let dir = mg_testkit::scratch_dir("serve-die-with-parent");
        std::fs::create_dir_all(&dir).unwrap();
        let run = |plain: bool| -> u32 {
            let out = dir.join(if plain { "plain.pid" } else { "dies.pid" });
            let _ = std::fs::remove_file(&out);
            let mut helper = Command::new(std::env::current_exe().unwrap());
            helper.args(["--exact", "tests::die_with_parent_helper", "--ignored", "--nocapture"]);
            helper.env("MG_DIE_OUT", &out).stdout(Stdio::null()).stderr(Stdio::null());
            if plain {
                helper.env("MG_DIE_PLAIN", "1");
            }
            let mut helper = helper.spawn().unwrap();
            assert!(within(30, || std::fs::read_to_string(&out).is_ok_and(|t| !t.is_empty())));
            let pid: u32 = std::fs::read_to_string(&out).unwrap().trim().parse().unwrap();
            helper.kill().unwrap();
            helper.wait().unwrap();
            pid
        };
        let pid = run(false);
        assert!(within(10, || gone(pid)), "the program outlived its process");
        // Without it: alive still, and then stopped here (the pid is the
        // one the helper started, checked by its command line).
        let pid = run(true);
        std::thread::sleep(Duration::from_millis(500));
        let alive = !gone(pid);
        let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        if cmdline.starts_with(b"sleep\0") {
            let target = rustix::process::Pid::from_raw(pid as i32).unwrap();
            let _ = rustix::process::kill_process(target, rustix::process::Signal::KILL);
        }
        assert!(alive, "without the signal the program lives on");
    }
}
