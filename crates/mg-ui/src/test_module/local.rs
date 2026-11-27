//! Test Module on a local server: a module whose project uses NWNX
//! (`[nwnx]`, on Linux) can't be tested by the game alone (`+TestNewModule`
//! starts the game's own server, which NWNX never loads into). The module
//! is served instead by a private `nwserver` with NWNX, in a folder of
//! Moonglow's, on a thread of its own ([`LocalRun::start`]), and the
//! player's game joins it once the module is loaded (`+connect`), to the
//! character list or with a test character put in the server's vault
//! ([`Join::QuickStart`]).
//!
//! Only the server and the game this run started are ever stopped. On
//! Linux both die with the run's thread, however Moonglow ends (a crash,
//! a kill): they are started from it with the parent-death signal
//! ([`mg_serve::die_with_parent`]), and it lives as long as they run.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// A character for the quick start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Character {
    /// A character file (`.bic`), from the player's local vault say: only
    /// read.
    Bic(PathBuf),
}

/// How the player joins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Join {
    /// The game's character list, the player's own characters on it (the
    /// server allows local characters).
    CharacterList,
    /// This character, put alone in the server's vault for the player's CD
    /// key (`-servervault 1`). The player still confirms it once on the
    /// list: no argument of the game's skips that.
    QuickStart(Character),
}

/// How far a local run is, as [`LocalRun::poll`] gives it.
#[derive(Debug, Clone, PartialEq)]
pub enum Progress {
    /// The server's folder being made ready (the module, haks, talk table).
    Preparing,
    /// The server started on this port.
    Starting { port: u16 },
    /// The module loaded, this long after the server started.
    Loaded { seconds: f32 },
    /// The game started, to join.
    Joining,
    /// A line for the Log (`warning: `, `error: ` before what is one).
    Log(String),
    /// The run is over: how, and whether that is a failure.
    Ended { how: String, failed: bool },
}

/// What makes the server's folder ready, on the run's thread: given a
/// way to say things (lines for the Log), the module's name in the
/// folder's `modules`.
pub type Prepare = Box<dyn FnOnce(&mut dyn FnMut(String)) -> Result<String, String> + Send>;

/// What a local run is started with ([`LocalPlan::new`] picks its port and
/// password, for the game's command to name).
pub struct LocalPlan {
    /// The server's program (`nwserver`).
    pub server: PathBuf,
    /// The server's user directory: a folder of Moonglow's, never the
    /// player's.
    pub dir: PathBuf,
    pub prepare: Prepare,
    /// NWNX, resolved for the module ([`mg_module::nwnx::resolve`]).
    pub nwnx: Option<mg_module::nwnx::Nwnx>,
    pub join: Join,
    /// The player's `cdkey.ini`, only read: their public key names their
    /// folder in the vault (for [`Join::QuickStart`]).
    pub cdkey: Option<PathBuf>,
    pub port: u16,
    /// The player password, which the game is given.
    pub password: String,
    /// How long the module may take to load.
    pub within: Duration,
}

impl std::fmt::Debug for LocalPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalPlan")
            .field("server", &self.server)
            .field("dir", &self.dir)
            .field("join", &self.join)
            .field("port", &self.port)
            .finish()
    }
}

impl LocalPlan {
    /// A plan on a free port, with a password nobody else is told.
    pub fn new(server: PathBuf, dir: PathBuf, prepare: Prepare, join: Join) -> LocalPlan {
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .and_then(|s| s.local_addr())
            .map_or(5121, |a| a.port());
        let password = format!("mg{:016x}", fastrand::u64(..));
        LocalPlan {
            server,
            dir,
            prepare,
            nwnx: None,
            join,
            cdkey: None,
            port,
            password,
            within: Duration::from_secs(300),
        }
    }

    /// Where the game connects: `127.0.0.1:PORT`.
    pub fn address(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }
}

/// The real [`Prepare`]: the module (a `.mod`, a folder or a project)
/// made ready in `dir` ([`mg_module::server_dir::prepare`]: its haks, talk
/// table and NWSync content) and the player's `development` folder linked
/// into it.
pub fn prepare_module(
    install: mg_resman::GameInstall,
    module: PathBuf,
    project: Option<PathBuf>,
    dir: PathBuf,
) -> Prepare {
    Box::new(move |say: &mut dyn FnMut(String)| {
        let (name, notes) = mg_module::server_dir::prepare(
            &install,
            &module,
            &dir,
            project.as_deref(),
            &|_| {},
            false,
        )
        .map_err(|e| format!("{e:#}"))?;
        let development = mg_module::server_dir::link_development(
            install.user_dir.as_deref(),
            &dir,
            mg_module::server_dir::Development::Players,
            true,
        )
        .map_err(|e| format!("{e:#}"))?;
        for note in notes.into_iter().chain(development) {
            say(note);
        }
        Ok(name)
    })
}

/// How long a stopped server has to end after `exit` before it is killed.
pub const STOP_PATIENCE: Duration = Duration::from_secs(5);

/// How long a stop waits for the run's thread at most ([`LocalRun::stop_and_wait`]
/// when dropped): the server's patience, and a moment for the game.
pub const STOP_WAIT: Duration = Duration::from_secs(7);

/// A local run under way: its server (and the game, once it joins) on a
/// thread of its own.
#[derive(Debug)]
pub struct LocalRun {
    said: Receiver<Progress>,
    stop: Arc<AtomicBool>,
    done: bool,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl LocalRun {
    /// Starts the run: the server's folder made ready, the private server
    /// started with NWNX, and, once its module is loaded, `client` (the
    /// game's command to join: [`super::join_command`]).
    pub fn start(plan: LocalPlan, client: Command) -> LocalRun {
        let (send, said) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let halt = stop.clone();
        // (The server and the game are started from this thread, and die
        // with it: it lives until both are stopped.)
        let thread = std::thread::spawn(move || {
            let (how, failed) = run(plan, client, &send, &halt);
            let _ = send.send(Progress::Ended { how, failed });
        });
        LocalRun { said, stop, done: false, thread: Some(thread) }
    }

    /// What happened since the last look.
    pub fn poll(&mut self) -> Vec<Progress> {
        let out: Vec<Progress> = self.said.try_iter().collect();
        self.done |= out.iter().any(|p| matches!(p, Progress::Ended { .. }));
        out
    }

    /// Whether the run is over (its `Ended` seen).
    pub fn ended(&self) -> bool {
        self.done
    }

    /// Stops the run: the server is told `exit` on its console, then
    /// killed if it has not ended; the game this run started is stopped.
    /// Never another program.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// Stops the run ([`LocalRun::stop`]) and waits for it to be over, for
    /// at most `within`: whether it is (its server and game stopped).
    /// What it said meanwhile stays to be read with [`LocalRun::poll`].
    pub fn stop_and_wait(&mut self, within: Duration) -> bool {
        self.stop();
        let waited = Instant::now();
        while self.thread.as_ref().is_some_and(|t| !t.is_finished()) && waited.elapsed() < within {
            std::thread::sleep(Duration::from_millis(20));
        }
        match self.thread.take_if(|t| t.is_finished()) {
            Some(t) => {
                let _ = t.join();
                true
            }
            None => self.thread.is_none(),
        }
    }
}

/// Dropped (Moonglow closing), the run is stopped and waited for, as long
/// as [`STOP_WAIT`] at most.
impl Drop for LocalRun {
    fn drop(&mut self) {
        self.stop_and_wait(STOP_WAIT);
    }
}

/// The game this run started: stopped with it, and known as a test's game
/// meanwhile ([`super::game_running`]).
struct Game(Child);

impl Game {
    fn start(mut client: Command) -> std::io::Result<Game> {
        mg_serve::die_with_parent(&mut client);
        let child = client.spawn()?;
        super::GAMES.lock().unwrap_or_else(|e| e.into_inner()).push(child.id());
        Ok(Game(child))
    }

    fn ended(&mut self) -> bool {
        self.0.try_wait().ok().flatten().is_some()
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        if !self.ended() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
        let pid = self.0.id();
        super::GAMES.lock().unwrap_or_else(|e| e.into_inner()).retain(|p| *p != pid);
    }
}

/// What the server said, for the Log: NWNX's notes, the loader's errors,
/// whether one is an error (the run stops: the server runs without NWNX).
fn hear(
    lines: Vec<mg_serve::Line>,
    nwnx: bool,
    builds: &mut mg_module::nwnx::Builds,
    send: &Sender<Progress>,
) -> bool {
    let mut failed = false;
    for line in lines {
        let notes = (builds.line(&line.text).into_iter())
            .chain(mg_module::nwnx::console_note(&line.text, nwnx));
        for n in notes {
            failed |= n.level == mg_module::nwnx::Level::Error;
            let _ = send.send(Progress::Log(n.line()));
        }
    }
    failed
}

/// The run, on its thread: how it ended, and whether that is a failure.
fn run(
    plan: LocalPlan,
    client: Command,
    send: &Sender<Progress>,
    stop: &AtomicBool,
) -> (String, bool) {
    let _ = send.send(Progress::Preparing);
    if let Err(e) = std::fs::create_dir_all(&plan.dir) {
        return (format!("{}: {e}", plan.dir.display()), true);
    }
    let mut say = |line: String| {
        let _ = send.send(Progress::Log(line));
    };
    let name = match (plan.prepare)(&mut say) {
        Ok(name) => name,
        Err(e) => return (format!("the server's folder: {e}"), true),
    };
    let mut args: Vec<std::ffi::OsString> = Vec::new();
    if let Join::QuickStart(Character::Bic(bic)) = &plan.join {
        let key = match &plan.cdkey {
            Some(ini) => mg_serve::cdkey::public_key_of_ini(ini),
            None => Err("no cdkey.ini in the game's user folder".into()),
        };
        let placed = key.and_then(|key| {
            mg_serve::place_in_vault(&plan.dir, &key, bic).map_err(|e| e.to_string())
        });
        if let Err(e) = placed {
            return (format!("the test character could not be put in the vault: {e}"), true);
        }
        args.extend(["-servervault".into(), "1".into()]);
    }
    if stop.load(Ordering::SeqCst) {
        return ("stopped".into(), false);
    }
    let nwnx = plan.nwnx.is_some();
    let mut server = match mg_serve::Server::start(&mg_serve::Options {
        server: plan.server.clone(),
        user_dir: plan.dir.clone(),
        module: name,
        port: Some(plan.port),
        name: "Moonglow test server".into(),
        player_password: Some(plan.password.clone()),
        console: true,
        env: plan.nwnx.as_ref().map(mg_module::nwnx::Nwnx::server_env).unwrap_or_default(),
        // (Nothing of Moonglow's own environment: what NWNX is to have is
        // in its settings, and a shell's LD_PRELOAD would load it unchecked.)
        clean_env: true,
        args,
        die_with_parent: true,
        ..Default::default()
    }) {
        Ok(s) => s,
        Err(e) => return (format!("the server could not start: {e}"), true),
    };
    let _ = send.send(Progress::Starting { port: server.port });
    let mut builds = mg_module::nwnx::Builds::default();
    let mut log = mg_serve::Tail::new(server.log_path());
    let started = Instant::now();
    let patience = STOP_PATIENCE;
    // Waits for the module, a moment at a time, saying what is heard.
    loop {
        let slice = Duration::from_millis(250);
        let waited = server.wait_ready(slice, stop);
        let failed = hear(server.lines(), nwnx, &mut builds, send);
        for line in log.more() {
            let _ = send.send(Progress::Log(line));
        }
        if failed {
            server.stop(patience);
            return ("NWNX did not run in the server (see above)".into(), true);
        }
        match waited {
            Ok(took) => {
                if nwnx && let Some(n) = builds.none_seen() {
                    let _ = send.send(Progress::Log(n.line()));
                    server.stop(patience);
                    return ("NWNX did not run in the server (see above)".into(), true);
                }
                let _ = send.send(Progress::Loaded { seconds: took.as_secs_f32() });
                break;
            }
            Err(mg_serve::NotReady::TimedOut) if started.elapsed() < plan.within => continue,
            Err(mg_serve::NotReady::TimedOut) => {
                server.stop(patience);
                let s = plan.within.as_secs();
                return (format!("the module had not loaded after {s} seconds"), true);
            }
            Err(mg_serve::NotReady::Cancelled) => {
                server.stop(patience);
                return ("stopped".into(), false);
            }
            Err(mg_serve::NotReady::Exited { last_lines }) => {
                return (
                    format!(
                        "the server ended before the module loaded; it said last:\n{}",
                        last_lines.join("\n")
                    ),
                    true,
                );
            }
        }
    }
    let _ = send.send(Progress::Joining);
    let mut game = match Game::start(client) {
        Ok(g) => g,
        Err(e) => {
            server.stop(patience);
            return (format!("the game could not start: {e}"), true);
        }
    };
    // Serves until the game ends, the server does, or the run is stopped.
    loop {
        let failed = hear(server.lines(), nwnx, &mut builds, send);
        for line in log.more() {
            let _ = send.send(Progress::Log(line));
        }
        if failed {
            server.stop(patience);
            return ("NWNX did not run in the server (see above)".into(), true);
        }
        if stop.load(Ordering::SeqCst) {
            server.stop(patience);
            drop(game);
            return ("stopped".into(), false);
        }
        if game.ended() {
            server.stop(patience);
            return ("the game ended: the server was stopped".into(), false);
        }
        if let Some(status) = server.exited() {
            drop(game);
            return (format!("the server ended ({status})"), true);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The server's folder of Test Module's local runs, for a module: in
/// Moonglow's cache, never the player's folder.
pub fn server_dir(module: &str) -> Option<PathBuf> {
    let name: String = (module.chars())
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    mg_module::build::cache_dir().map(|d| d.join("test-server").join(name))
}

/// A path's text for a test's fake program.
#[cfg(all(test, unix))]
fn shell_quoted(p: &std::path::Path) -> String {
    format!("'{}'", p.display().to_string().replace('\'', "'\\''"))
}

#[cfg(all(test, unix))]
mod tests;
