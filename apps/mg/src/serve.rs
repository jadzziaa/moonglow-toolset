//! `mg serve`: a module in the game's headless server, on a user
//! directory that is not the player's (the player's `development`
//! folder linked into it), its log followed and its console passed
//! through. `mg test` runs its tests through the same.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use mg_resman::GameInstall;

/// What to serve, and how.
#[derive(Debug, Clone, Default)]
pub(crate) struct Config {
    /// A `.mod`, a module folder or a nasher project.
    pub module: PathBuf,
    /// The server's user directory; `None`: a scratch one of Moonglow's.
    pub dir: Option<PathBuf>,
    /// `dir` may be the player's own NWN folder.
    pub real_user_dir: bool,
    pub port: Option<u16>,
    pub public: bool,
    pub name: String,
    pub passwords: Passwords,
    /// Environment files (`NAME=value` lines).
    pub env: Vec<PathBuf>,
    /// A library loaded into the server (`LD_PRELOAD`).
    pub preload: Option<PathBuf>,
    /// Start again when the module's file (or folder) changes.
    pub restart_on_change: bool,
    /// Lines typed on standard input go to the server's console
    /// (`/restart` and `/quit` are Moonglow's).
    pub stdin: bool,
    /// More arguments for the server.
    pub args: Vec<String>,
    /// The nasher project `module` was built from (`mg watch`): its own
    /// talk table is made for the server, as for the project served.
    pub project: Option<PathBuf>,
    /// Say how far haks being written from NWSync are, on standard error.
    pub progress: bool,
    /// The haks and talk table from disk, as though the project named no
    /// NWSync server (`--no-nwsync`; `source = ""` in the builder's own
    /// file does the same).
    pub no_nwsync: bool,
    /// What the server's `development` folder is: the player's, linked,
    /// or `mg watch`'s own.
    pub development: Development,
    /// Whose NWNX the server runs with: the project's `[nwnx]`, a
    /// home's (`--nwnx`) or none (`--no-nwnx`).
    pub nwnx: mg_module::nwnx::Choice,
    /// Scripts are replaced while the server runs (`mg watch --serve`).
    pub live: bool,
}

/// The server's passwords, as `mg serve` and `mg watch --serve` take
/// them: one not given is a random one, kept across the server's
/// restarts within a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, clap::Args)]
pub(crate) struct Passwords {
    /// The player password (default: a random one, printed).
    #[arg(long)]
    pub player_password: Option<String>,
    /// The DM password (default: a random one, printed).
    #[arg(long)]
    pub dm_password: Option<String>,
    /// The admin password (default: a random one).
    #[arg(long)]
    pub admin_password: Option<String>,
}

/// `--nwnx DIR` and `--no-nwnx`, as `mg serve`, `mg test` and `mg watch
/// --serve` take them: whose NWNX the server runs with.
#[derive(Debug, Clone, Default, PartialEq, Eq, clap::Args)]
pub(crate) struct NwnxArgs {
    /// Run the server with the NWNX in this folder (its Binaries/), whatever
    /// the project's [nwnx] says; Linux.
    #[arg(long, value_name = "DIR", conflicts_with = "no_nwnx")]
    pub nwnx: Option<PathBuf>,
    /// Run the server without NWNX, though the project's [nwnx] uses it.
    #[arg(long)]
    pub no_nwnx: bool,
}

impl NwnxArgs {
    pub(crate) fn choice(&self) -> mg_module::nwnx::Choice {
        use mg_module::nwnx::Choice;
        match (&self.nwnx, self.no_nwnx) {
            (_, true) => Choice::Off,
            (Some(dir), false) => Choice::Home(dir.clone()),
            (None, false) => Choice::Project,
        }
    }
}

/// What happens while a module is served.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Event {
    /// The server was started (again, after the first).
    Started { port: u16, pid: u32, user_dir: PathBuf, log: PathBuf, engine_log: PathBuf },
    /// What players and DMs need to get in.
    Passwords { player: String, dm: String, admin: String },
    /// The module is loaded and running.
    Ready { seconds: f32 },
    /// A line of the server's log (scripts' `WriteTimestampedLogEntry`).
    Log(String),
    /// An error or a warning in the engine's own log (a script that
    /// failed, a resource it could not read).
    Engine(String),
    /// A line the server printed on its console.
    Console(String),
    /// Something of Moonglow's own to say.
    Note(String),
    /// The module's file changed: the server is started again.
    Changed,
    /// Nothing happened in this look (a moment to check the time).
    Idle,
}

/// What a caller wants after an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    /// Stop the server: the run ends well.
    Stop,
    Restart,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    /// The caller stopped it, or `/quit` or `exit` was typed.
    Stopped,
    /// The server could not load the module.
    NotLoaded,
    /// The server ended on its own.
    Exited(Option<i32>),
    /// NWNX was asked for and failed (not preloaded, another game build):
    /// the server was stopped, as it ran without it.
    Nwnx,
}

/// A scratch folder of a run's ([`run_dir`]), held while the run
/// lasts.
#[derive(Debug)]
pub(crate) struct Scratch {
    pub dir: PathBuf,
    /// Locked while held: another run of the module takes another folder.
    _held: Option<std::fs::File>,
}

/// The scratch folder of a run of `kind` (`serve`, `test`) on a module:
/// one for a module, used again by its next run (so its logs stay to be
/// read, and nothing piles up). While a run holds it (a lock on a file in
/// it, let go when the run ends however it ends), another run of the same
/// module at once takes the next one (`-2`, `-3`…): two servers never
/// share a user directory.
pub(crate) fn run_dir(kind: &str, module: &Path) -> Scratch {
    use std::hash::{Hash, Hasher};
    let module = std::fs::canonicalize(module).unwrap_or_else(|_| module.to_path_buf());
    let mut h = std::collections::hash_map::DefaultHasher::new();
    module.hash(&mut h);
    let stem = module.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let name = format!("{stem}-{:08x}", h.finish() as u32);
    let parent = std::env::temp_dir().join(format!("moonglow-{kind}"));
    for n in 1..100 {
        let dir = match n {
            1 => parent.join(&name),
            n => parent.join(format!("{name}-{n}")),
        };
        let lock = std::fs::create_dir_all(&dir).and_then(|()| {
            // (Written to, not appended to: Windows locks only a file
            // opened for writing.)
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(dir.join(".mg-lock"))
        });
        // (A folder that can't be locked is used unlocked, as before.)
        let Ok(file) = lock else { return Scratch { dir, _held: None } };
        match file.try_lock() {
            Ok(()) => return Scratch { dir, _held: Some(file) },
            Err(std::fs::TryLockError::WouldBlock) => continue,
            Err(std::fs::TryLockError::Error(_)) => return Scratch { dir, _held: None },
        }
    }
    Scratch { dir: parent.join(name), _held: None }
}

/// A server is running (Ctrl+C stops it; otherwise Ctrl+C ends `mg`).
static SERVING: AtomicBool = AtomicBool::new(false);
/// Ctrl+C was pressed while a server ran.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Catches Ctrl+C (and the terminal closing, and a TERM), once for the
/// process: while a server runs, it is stopped as `/quit` stops it, with
/// `exit` on its console (it closes its database and logs in order);
/// otherwise `mg` ends at once, as it would. Whether it is caught (the
/// server then gets a process group of its own, which the terminal's
/// Ctrl+C does not reach).
fn catch_interrupt() -> bool {
    static CAUGHT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CAUGHT.get_or_init(|| {
        ctrlc::set_handler(|| {
            if SERVING.load(Ordering::SeqCst) {
                INTERRUPTED.store(true, Ordering::SeqCst);
            } else {
                std::process::exit(130);
            }
        })
        .is_ok()
    })
}

/// [`SERVING`] for as long as it lives.
struct Serving;

impl Serving {
    fn start() -> Serving {
        INTERRUPTED.store(false, Ordering::SeqCst);
        SERVING.store(true, Ordering::SeqCst);
        Serving
    }
}

impl Drop for Serving {
    fn drop(&mut self) {
        SERVING.store(false, Ordering::SeqCst);
    }
}

pub(crate) use mg_module::server_dir::{
    Development, link_development, module_name, players_dir, save_copy, unlink_development,
};

/// The user directory made ready ([`mg_module::server_dir::prepare`]),
/// with how far a write of haks from NWSync is said on standard error
/// where `progress` asks (every few seconds).
pub(crate) fn prepare(
    gi: &GameInstall,
    module: &Path,
    dir: &Path,
    project: Option<&Path>,
    progress: bool,
    no_nwsync: bool,
) -> Result<(String, Vec<String>)> {
    let last = std::sync::Mutex::new(Instant::now() - Duration::from_secs(60));
    let said = |p: mg_nwsync::haks::Progress| {
        if !progress || p.of_bytes == 0 {
            return;
        }
        let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
        if last.elapsed() >= Duration::from_secs(3) {
            *last = Instant::now();
            eprintln!("{}", crate::nwsync_read::hak_progress(&p));
        }
    };
    let (name, notes) = mg_module::server_dir::prepare(gi, module, dir, project, &said, no_nwsync)?;
    // (The note of a module folder beside the .mod is the module's, as
    // `open_module` says it: once with the output, not at every restart
    // of `--restart-on-change` or `mg watch --serve`.)
    let (beside, notes): (Vec<String>, Vec<String>) =
        notes.into_iter().partition(|n| n.contains("has its module folder beside it"));
    beside.into_iter().for_each(crate::said_of_opened);
    Ok((name, notes))
}

/// What a module's file or folder looks like, to see it change.
pub(crate) fn signature(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    fn walk(path: &Path, depth: usize, h: &mut impl Hasher) {
        let Ok(meta) = std::fs::metadata(path) else { return };
        (path, meta.len(), meta.modified().ok()).hash(h);
        if meta.is_dir() && depth < 6 {
            let mut inside: Vec<PathBuf> =
                std::fs::read_dir(path).into_iter().flatten().flatten().map(|e| e.path()).collect();
            inside.sort();
            for p in inside.iter().filter(|p| !p.ends_with(".git") && !p.ends_with(".nasher")) {
                walk(p, depth + 1, h);
            }
        }
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    walk(path, 0, &mut h);
    h.finish()
}

/// Lines of a reader, as they come, on a thread of their own.
fn lines(reader: impl std::io::Read + Send + 'static) -> Receiver<String> {
    let (send, receive) = channel();
    std::thread::spawn(move || {
        for line in BufReader::new(reader).split(b'\n').map_while(Result::ok) {
            let line = String::from_utf8_lossy(&line).trim_end_matches('\r').to_string();
            if send.send(line).is_err() {
                break;
            }
        }
    });
    receive
}

/// How long after a command the server has its answer ready to print.
const NUDGE: Duration = Duration::from_millis(400);

/// Refuses a server's user directory that is the player's own NWN folder
/// (unless `real_user_dir` allows it) or lies in the game's: asked before
/// anything is made or written under it, by every command that takes
/// `--dir`.
///
/// `real_user_dir`: what `--real-user-dir` said, `None` for a command that
/// has no such option.
pub(crate) fn refuse_dir(gi: &GameInstall, dir: &Path, real_user_dir: Option<bool>) -> Result<()> {
    refuse_players_dir(gi.user_dir.as_deref(), dir, real_user_dir)?;
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    // (A folder not made yet is told by the nearest one above it that is.)
    let mut there = Some(dir.as_path());
    let real = loop {
        let Some(at) = there else { break None };
        match std::fs::canonicalize(at) {
            Ok(real) => break Some(real),
            Err(_) => there = at.parent(),
        }
    };
    let root = std::fs::canonicalize(&gi.root).unwrap_or_else(|_| gi.root.clone());
    if real.is_some_and(|d| d.starts_with(&root) || d.starts_with(&gi.root)) {
        bail!("{} is in the game's own folder", dir.display());
    }
    Ok(())
}

/// Refuses `dir` where it is the player's own NWN folder (`user`, the
/// user folder `mg` was given, or where the game keeps it), unless
/// `real_user_dir` allows it: [`refuse_dir`] without an install.
pub(crate) fn refuse_players_dir(
    user: Option<&Path>,
    dir: &Path,
    real_user_dir: Option<bool>,
) -> Result<()> {
    if let Some(theirs) = players_dir(user, dir)
        && real_user_dir != Some(true)
    {
        let anyway = match real_user_dir {
            Some(_) => " (pass --real-user-dir to serve from it anyway)",
            None => "",
        };
        bail!(
            "{} is your own NWN folder: a server there writes its vault, database and settings \
             into it{anyway}",
            theirs.display()
        );
    }
    Ok(())
}

/// What the server's lines have told so far.
#[derive(Debug)]
struct Heard {
    started: Instant,
    /// It could not load the module.
    failed: bool,
    /// It was started with NWNX.
    nwnx: bool,
    builds: mg_module::nwnx::Builds,
    ready: bool,
    /// What it said of NWNX was an error: the run stops.
    nwnx_failed: bool,
}

impl Heard {
    fn new(nwnx: bool) -> Heard {
        let builds = Default::default();
        Heard {
            started: Instant::now(),
            failed: false,
            nwnx,
            builds,
            ready: false,
            nwnx_failed: false,
        }
    }
}

/// What a line the server printed (on its console, or the loader's on
/// its standard error) says: that the module loaded (`Ready`, `seconds`
/// since it started: [`mg_serve::loaded`]), that it could not (`failed`),
/// the line itself, and what it says of NWNX ([`mg_module::nwnx`]: the
/// builds, a plugin or the preload that failed, and, once loaded, NWNX
/// asked for but never heard of).
fn console_events(line: mg_serve::Line, h: &mut Heard) -> Vec<Event> {
    let mut events = Vec::new();
    let mut notes = Vec::new();
    if mg_serve::loaded(&line.text) {
        events.push(Event::Ready { seconds: h.started.elapsed().as_secs_f32() });
        if h.nwnx && !std::mem::replace(&mut h.ready, true) {
            notes.extend(h.builds.none_seen());
        }
    }
    h.failed |= mg_serve::not_loaded(&line.text);
    notes.extend(h.builds.line(&line.text));
    notes.extend(mg_module::nwnx::console_note(&line.text, h.nwnx));
    events.push(Event::Console(line.text));
    h.nwnx_failed |= notes.iter().any(|n| n.level == mg_module::nwnx::Level::Error);
    events.extend(notes.into_iter().map(|n| Event::Note(n.line())));
    events
}

/// The last of the server's lines, after the loop: reported as the loop
/// does, so a load that came late is still a `Ready`.
fn drain_console(
    lines: impl Iterator<Item = mg_serve::Line>,
    h: &mut Heard,
    each: &mut dyn FnMut(&Event) -> Flow,
) {
    for line in lines {
        for e in console_events(line, h) {
            each(&e);
        }
    }
}

/// The `--env` files' lines, in their order.
fn env_lines(files: &[PathBuf]) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(file).with_context(|| file.display().to_string())?;
        let read =
            mg_serve::env_file(&text).map_err(|e| anyhow::anyhow!("{}: {e}", file.display()))?;
        out.extend(read);
    }
    Ok(out)
}

/// The server's environment as a run starts it ([`server_env`]).
#[derive(Debug, Default, PartialEq)]
struct ServerEnv {
    vars: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    /// NWNX's core is preloaded (the project's, or one of `--preload` or
    /// an `--env` line's `LD_PRELOAD`): what the server says of NWNX is
    /// checked ([`Heard`]).
    nwnx: bool,
}

/// The server's environment: NWNX's (`nwnx`: its core preloaded, its
/// plugins' folder, the project's environment file and `more`, the
/// `--env` files' lines, which [`mg_module::nwnx::resolve_with`] checked
/// with it), or `more` alone without NWNX; then `--preload`'s library:
/// preloaded beside NWNX's core, unless it is that core; another NWNX's
/// core is refused. One `LD_PRELOAD`, the last given.
fn server_env(
    nwnx: Option<&mg_module::nwnx::Nwnx>,
    more: &[(String, String)],
    preload: Option<&Path>,
    linux: bool,
) -> Result<ServerEnv> {
    use mg_module::nwnx::CORE_FILE;
    let mut env = match nwnx {
        Some(n) => n.server_env(),
        None => more.iter().map(|(k, v)| (k.into(), v.into())).collect(),
    };
    if let Some(preload) = preload {
        if !linux {
            bail!("--preload is the Linux loader's LD_PRELOAD");
        }
        let preload =
            std::fs::canonicalize(preload).with_context(|| preload.display().to_string())?;
        let core = nwnx.map(|n| std::fs::canonicalize(&n.core).unwrap_or(n.core.clone()));
        match core {
            Some(core) if core == preload => {}
            Some(core) if preload.file_name().is_some_and(|n| n == CORE_FILE) => bail!(
                "--preload {} is another NWNX than the project's ({}): pass --no-nwnx to \
                 preload it alone, or --nwnx with its folder",
                preload.display(),
                core.display()
            ),
            Some(core) => {
                // (The loader takes a list, separated by colons.)
                let mut both = core.into_os_string();
                both.push(":");
                both.push(&preload);
                env.retain(|(k, _)| k != "LD_PRELOAD");
                env.insert(0, ("LD_PRELOAD".into(), both));
            }
            None => env.push(("LD_PRELOAD".into(), preload.into())),
        }
    }
    // (The last LD_PRELOAD is the one the server gets: kept alone, in the
    // first one's place.)
    if let Some(last) = env.iter().rposition(|(k, _)| k == "LD_PRELOAD") {
        let value = env[last].1.clone();
        env.retain(|(k, _)| k != "LD_PRELOAD");
        env.insert(0, ("LD_PRELOAD".into(), value));
    }
    let nwnx = env.iter().filter(|(k, _)| k == "LD_PRELOAD").any(|(_, v)| {
        let v = v.to_string_lossy();
        v.split([':', ' ']).any(|l| Path::new(l).file_name().is_some_and(|n| n == CORE_FILE))
    });
    Ok(ServerEnv { vars: env, nwnx })
}

/// The project file of the module a run serves (`original`: the
/// `.mod`, folder or nasher project the builder named).
fn project_file_of(original: &Path) -> mg_module::project_file::ProjectFile {
    use mg_module::project_file::{Place, ProjectFile};
    let root = match original.file_name() {
        Some(n) if n == "nasher.cfg" => original.parent().map(Path::to_path_buf),
        _ if original.is_dir() => Some(original.to_path_buf()),
        _ => None,
    };
    match root {
        Some(root) => ProjectFile::read(&root),
        None => Place::beside(original)
            .filter(|p| p.team().is_file())
            .map(|p| ProjectFile::read_at(&p))
            .unwrap_or_default(),
    }
}

/// Serves the module until `each` says to stop or the server ends.
pub(crate) fn serve(
    gi: &GameInstall,
    c: &Config,
    each: &mut dyn FnMut(&Event) -> Flow,
) -> Result<End> {
    let server =
        mg_serve::server_binary(&gi.root).with_context(|| mg_serve::no_server(&gi.root))?;
    // (The server runs in its own folder: a path from here means nothing
    // to it.)
    let scratch = c.dir.is_none().then(|| run_dir("serve", &c.module));
    let dir = match (&c.dir, &scratch) {
        (Some(dir), _) => dir.clone(),
        (None, Some(s)) => s.dir.clone(),
        (None, None) => unreachable!("one or the other"),
    };
    let dir = std::path::absolute(&dir).unwrap_or(dir);
    let caught = catch_interrupt();
    refuse_dir(gi, &dir, Some(c.real_user_dir))?;
    // (The project's `[nwnx]`: the module's the builder named, not the
    // copy a run serves.)
    let project_file = project_file_of(c.project.as_deref().unwrap_or(&c.module));
    let mut said_nwnx: Vec<String> = Vec::new();
    let typed = c.stdin.then(|| lines(std::io::stdin()));
    let mut typing = typed.is_some();
    let mut flow = Flow::Continue;
    // (Started again, the server keeps its port and its passwords.)
    let mut port = c.port;
    let mut passwords = c.passwords.clone();
    loop {
        let (name, notes) =
            prepare(gi, &c.module, &dir, c.project.as_deref(), c.progress, c.no_nwsync)?;
        let development =
            link_development(gi.user_dir.as_deref(), &dir, c.development, scratch.is_some())?;
        for note in notes.into_iter().chain(development) {
            each(&Event::Note(note));
        }
        // NWNX, by the project and the options, checked against the copy's
        // scripts; its notes said again only where they changed.
        let served = mg_module::Module::open(&dir.join("modules").join(format!("{name}.mod")));
        // (The --env files' lines checked with the project's.)
        let more = env_lines(&c.env)?;
        let resolved = mg_module::nwnx::resolve_with(
            &project_file,
            &mg_module::nwnx::Options {
                choice: c.nwnx.clone(),
                module: served.as_ref().ok(),
                live: c.live,
                ..Default::default()
            },
            &more,
        );
        let lines: Vec<String> = resolved.notes.iter().map(|n| n.line()).collect();
        if lines != said_nwnx {
            for n in resolved.notes.iter().filter(|n| n.level != mg_module::nwnx::Level::Error) {
                each(&Event::Note(n.line()));
            }
            said_nwnx = lines;
        }
        if resolved.failed() {
            bail!("NWNX: {}", resolved.errors().join("\n"));
        }
        let nwnx = resolved.nwnx;
        let env =
            server_env(nwnx.as_ref(), &more, c.preload.as_deref(), cfg!(target_os = "linux"))?;
        let seen = signature(&c.module);
        let serving = Serving::start();
        let mut server = mg_serve::Server::start(&mg_serve::Options {
            server: server.clone(),
            user_dir: dir.clone(),
            module: name,
            port,
            public: c.public,
            name: c.name.clone(),
            player_password: passwords.player_password.clone(),
            dm_password: passwords.dm_password.clone(),
            admin_password: passwords.admin_password.clone(),
            console: true,
            env: env.vars,
            // (Nothing of the builder's own environment, NWNX or not: a
            // shell's LD_PRELOAD of NWNX would load it unchecked, and a
            // webhook's address of theirs would post from a test. What
            // the server is to have is given: the project's, --env's.)
            clean_env: true,
            args: c.args.iter().map(Into::into).collect(),
            own_group: caught,
            die_with_parent: false,
        })?;
        port = Some(server.port);
        passwords = Passwords {
            player_password: Some(server.player_password.clone()),
            dm_password: Some(server.dm_password.clone()),
            admin_password: Some(server.admin_password.clone()),
        };
        let mut heard = Heard::new(env.nwnx);
        let mut log = mg_serve::Tail::new(server.log_path());
        let mut engine = mg_serve::Tail::new(server.engine_log_path());
        // (Not the two the engine says at every start.)
        let grave = |l: &String| {
            (l.starts_with("E ") || l.starts_with("W "))
                && !l.contains("Filename passed contains an undefined alias")
                && !l.contains("Resetting in-use connection")
        };
        each(&Event::Started {
            port: server.port,
            pid: server.id(),
            user_dir: dir.clone(),
            log: server.log_path(),
            engine_log: server.engine_log_path(),
        });
        each(&Event::Passwords {
            player: server.player_password.clone(),
            dm: server.dm_password.clone(),
            admin: server.admin_password.clone(),
        });
        let (mut looked, mut quit) = (Instant::now(), false);
        // The server prints a command's answer when it next reads its
        // console: an empty line a moment after a command brings it out.
        let mut nudge: Option<Instant> = None;
        let end = loop {
            let mut events = Vec::new();
            for line in server.lines() {
                events.extend(console_events(line, &mut heard));
            }
            events.extend(engine.more().into_iter().filter(grave).map(Event::Engine));
            events.extend(log.more().into_iter().map(Event::Log));
            if let Some(typed) = typed.as_ref().filter(|_| typing) {
                loop {
                    match typed.try_recv() {
                        Ok(line) => match line.trim() {
                            "/restart" => flow = Flow::Restart,
                            "/quit" => flow = Flow::Stop,
                            said => {
                                quit |= said == "exit" || said == "quit";
                                let _ = server.say(said);
                                nudge = Some(Instant::now() + NUDGE);
                            }
                        },
                        // (Nothing more will be typed; the server stays.)
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            typing = false;
                            break;
                        }
                        Err(_) => break,
                    }
                }
            }
            if nudge.is_some_and(|at| Instant::now() >= at) {
                nudge = None;
                let _ = server.say("");
            }
            if c.restart_on_change && looked.elapsed() > Duration::from_millis(500) {
                looked = Instant::now();
                if signature(&c.module) != seen {
                    events.push(Event::Changed);
                    flow = Flow::Restart;
                }
            }
            if heard.nwnx_failed && flow == Flow::Continue {
                events.push(Event::Note("stopping the server: it runs without NWNX".into()));
                flow = Flow::Stop;
            }
            if INTERRUPTED.swap(false, Ordering::SeqCst) {
                events.push(Event::Note("Ctrl+C: stopping the server".into()));
                flow = Flow::Stop;
            }
            if events.is_empty() {
                events.push(Event::Idle);
            }
            for e in &events {
                match each(e) {
                    Flow::Continue => {}
                    other if flow != Flow::Stop => flow = other,
                    _ => {}
                }
            }
            if flow != Flow::Continue {
                break None;
            }
            if let Some(status) = server.exited() {
                // (What it said last.)
                std::thread::sleep(Duration::from_millis(100));
                drain_console(server.lines().into_iter(), &mut heard, each);
                for line in engine.more().into_iter().filter(grave) {
                    each(&Event::Engine(line));
                }
                for line in log.more() {
                    each(&Event::Log(line));
                }
                break Some(match (heard.failed, quit) {
                    (true, _) => End::NotLoaded,
                    (_, true) => End::Stopped,
                    _ => End::Exited(status.code()),
                });
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        if let Some(end) = end {
            return Ok(end);
        }
        server.stop(Duration::from_secs(5));
        // (What it said last, once its console has closed.)
        std::thread::sleep(Duration::from_millis(100));
        drain_console(server.lines().into_iter(), &mut heard, each);
        for line in engine.more().into_iter().filter(grave) {
            each(&Event::Engine(line));
        }
        for line in log.more() {
            each(&Event::Log(line));
        }
        drop(serving);
        if heard.nwnx_failed {
            return Ok(End::Nwnx);
        }
        match std::mem::replace(&mut flow, Flow::Continue) {
            Flow::Restart => continue,
            _ => return Ok(End::Stopped),
        }
    }
}

/// An event as a line of JSON.
pub(crate) fn json(e: &Event) -> Option<serde_json::Value> {
    use serde_json::json;
    let text = |p: &Path| p.display().to_string();
    Some(match e {
        Event::Started { port, pid, user_dir, log, engine_log } => json!({
            "event": "started", "port": port, "pid": pid, "user_dir": text(user_dir),
            "log": text(log), "engine_log": text(engine_log),
        }),
        Event::Passwords { player, dm, admin } => json!({
            "event": "passwords", "player": player, "dm": dm, "admin": admin,
        }),
        Event::Ready { seconds } => json!({ "event": "ready", "seconds": seconds }),
        Event::Log(line) => json!({ "event": "log", "line": line }),
        Event::Console(line) => json!({ "event": "console", "line": line }),
        Event::Engine(line) => json!({ "event": "engine", "line": line }),
        Event::Note(note) => json!({ "event": "note", "note": note }),
        Event::Changed => json!({ "event": "changed" }),
        Event::Idle => return None,
    })
}

/// Writes an event of a run that prints as it goes (`mg serve`, `mg
/// watch`): a line of JSON with `--json`, else its lines of text.
pub(crate) fn say(json: bool, v: &serde_json::Value, lines: &[String]) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = match json {
        true => writeln!(out, "{v}"),
        false => lines.iter().try_for_each(|l| writeln!(out, "{l}")),
    };
    let _ = out.flush();
}

/// How a run ended, in words.
pub(crate) fn how(end: End) -> String {
    match end {
        End::Stopped => "stopped".into(),
        End::NotLoaded => "the server could not load the module".into(),
        End::Exited(code) => format!("the server ended ({code:?})"),
        End::Nwnx => "NWNX did not run in the server (see above)".into(),
    }
}

/// The last event of a run that prints as it goes (`mg serve`, `mg
/// watch --serve`): how it ended, and whether that is a failure.
pub(crate) fn say_ended(json: bool, how: &str, failed: bool) {
    let v = serde_json::json!({ "event": "ended", "how": how, "failed": failed });
    say(json, &v, &[format!("mg: {how}")]);
}

/// An event as a line for a terminal.
pub(crate) fn text(e: &Event) -> Option<String> {
    Some(match e {
        Event::Started { port, pid, user_dir, log, engine_log } => format!(
            "mg: server started on port {port} (process {pid})\nmg: user directory {}\nmg: log \
             {}\nmg: engine log {}",
            user_dir.display(),
            log.display(),
            engine_log.display()
        ),
        Event::Passwords { player, dm, .. } => {
            format!("mg: player password {player}, DM password {dm}")
        }
        Event::Ready { seconds } => format!("mg: module loaded in {seconds:.1} s"),
        Event::Log(line) | Event::Console(line) | Event::Engine(line) => line.clone(),
        Event::Note(note) => format!("mg: {note}"),
        Event::Changed => "mg: the module changed: starting the server again".into(),
        Event::Idle => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::ResType;
    use mg_module::ModuleLocation;
    use mg_resman::ResKey;

    fn same(a: &Path, b: &Path) -> bool {
        std::fs::canonicalize(a)
            .ok()
            .zip(std::fs::canonicalize(b).ok())
            .is_some_and(|(a, b)| a == b)
    }

    /// The server's last lines, read after the loop has ended (at the
    /// timeout, or at its exit), still tell that the module loaded.
    #[test]
    fn a_load_said_after_the_loop_is_still_ready() {
        let lines = ["Server: Loading module", "Server: Module loaded", "Server: Exiting..."];
        let mut seen = Vec::new();
        let mut heard = Heard::new(false);
        let line = |l: &str| mg_serve::Line { text: l.to_string(), stderr: false };
        drain_console(lines.iter().map(|l| line(l)), &mut heard, &mut |e| {
            seen.push(json(e).unwrap()["event"].as_str().unwrap().to_string());
            Flow::Continue
        });
        assert_eq!(seen, ["console", "ready", "console", "console"]);
        assert!(!heard.failed);
        let mut e = console_events(line("Unable to load module x"), &mut heard);
        assert!(heard.failed && matches!(e.pop(), Some(Event::Console(_))) && e.is_empty());
    }

    #[test]
    fn a_module_s_name_for_the_server() {
        assert_eq!(module_name(Path::new("/x/Neverwinter Chess.mod")), "neverwinter_chess");
        assert_eq!(module_name(Path::new("tdod_library.mod")), "tdod_library");
    }

    /// A project that keeps its talk table as JSON: the server's tlk
    /// folder gets the table made of it (it finds no other), not a link to
    /// the player's own copy.
    #[test]
    fn a_projects_talk_table_is_made_for_the_server() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let dir = mg_testkit::scratch_dir("mg-serve-tlk");
        let user = dir.join("user");
        std::fs::create_dir_all(user.join("tlk")).unwrap();
        std::fs::write(user.join("tlk/mytlk.tlk"), b"the player's own").unwrap();
        let gi = GameInstall::new(&root, Some(user.clone()), "en");
        let mut m = mg_module::Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mytlk".to_vec()));
        m.set_info(&info).unwrap();
        let project = dir.join("project");
        m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
            .unwrap();
        let mut cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
        cfg.push_str("\n[target]\nname = \"tlk\"\nfile = \"mytlk.tlk\"\n  [target.sources]\n  include = \"src/tlk/*.json\"\n");
        std::fs::write(project.join("nasher.cfg"), cfg).unwrap();
        std::fs::create_dir_all(project.join("src/tlk")).unwrap();
        let json = r#"{"language":0,"entries":[{"id":0,"text":"Ours"}]}"#;
        std::fs::write(project.join("src/tlk/mytlk.tlk.json"), json).unwrap();
        let server = dir.join("server");
        let (_, notes) = prepare(&gi, &project, &server, None, false, false).unwrap();
        assert!(notes.iter().any(|n| n.contains("made from")), "{notes:?}");
        let made = std::fs::read(server.join("tlk/mytlk.tlk")).unwrap();
        let tlk = mg_tlk::Tlk::read(&made).unwrap();
        assert_eq!(tlk.entries[0].text, b"Ours");
        assert_eq!(std::fs::read(user.join("tlk/mytlk.tlk")).unwrap(), b"the player's own");
        // An archive built from the project (`mg watch`'s, `mg test`'s
        // copy): the project's own table, made again as it changes.
        let built = dir.join("built.mod");
        mg_module::Module::open(&project)
            .unwrap()
            .save_as(&ModuleLocation::Archive(built.clone()))
            .unwrap();
        let json = r#"{"language":0,"entries":[{"id":0,"text":"Changed"}]}"#;
        std::fs::write(project.join("src/tlk/mytlk.tlk.json"), json).unwrap();
        let (_, notes) = prepare(&gi, &built, &server, Some(&project), false, false).unwrap();
        assert!(notes.iter().any(|n| n.contains("made from")), "{notes:?}");
        let made =
            mg_tlk::Tlk::read(&std::fs::read(server.join("tlk/mytlk.tlk")).unwrap()).unwrap();
        assert_eq!(made.entries[0].text, b"Changed");
    }

    /// A module whose hak and talk table are named with a folder in
    /// them (`../x`): nothing is placed outside the server's `hak` and
    /// `tlk` folders, and each is said not to be found.
    #[test]
    fn a_hak_named_with_a_folder_is_placed_nowhere() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let dir = mg_testkit::scratch_dir("mg-serve-hak-outside");
        let user = dir.join("user");
        for sub in ["hak", "tlk"] {
            std::fs::create_dir_all(user.join(sub)).unwrap();
        }
        // (Where `<user>/hak/../x.hak` and `<user>/tlk/../y.tlk` are.)
        std::fs::write(user.join("x.hak"), b"not the module's").unwrap();
        std::fs::write(user.join("y.tlk"), b"not the module's").unwrap();
        let gi = GameInstall::new(&root, Some(user.clone()), "en");
        assert_eq!(gi.find_hak("../x"), None);
        let mut m = mg_module::Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        let mut hak = mg_gff::Struct::new(8);
        hak.set("Mod_Hak", mg_gff::Value::String(b"../x".to_vec()));
        info.root.set("Mod_HakList", mg_gff::Value::List(vec![hak]));
        info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"../y".to_vec()));
        m.set_info(&info).unwrap();
        let module = dir.join("outside.mod");
        m.save_as(&ModuleLocation::Archive(module.clone())).unwrap();
        let server = dir.join("server");
        let (_, notes) = prepare(&gi, &module, &server, None, false, false).unwrap();
        assert!(!server.join("x.hak").exists() && !server.join("y.tlk").exists());
        let warned =
            |what: &str| notes.iter().any(|n| n.contains(what) && n.contains("folder separator"));
        assert!(warned("hak ../x not found") && warned("talk table ../y not found"), "{notes:?}");
        assert_eq!(std::fs::read(user.join("x.hak")).unwrap(), b"not the module's");
    }

    /// A module whose project names an NWSync server: the haks the
    /// origins file names are written from the game's NWSync storage into
    /// Moonglow's cache and linked for the server, as is the talk table;
    /// a hak it doesn't name comes from disk; data not downloaded is an
    /// error that says how to download it. The player's folders are left
    /// as they were.
    #[test]
    fn haks_from_nwsync_are_written_for_the_server() {
        use mg_core::sha1::sha1;
        let Some(root) = mg_testkit::nwn_root() else { return };
        let dir = mg_testkit::scratch_dir("mg-serve-nwsync-haks");
        let user = dir.join("user");
        for sub in ["hak", "tlk"] {
            std::fs::create_dir_all(user.join(sub)).unwrap();
        }
        let mut local = mg_erf::ErfWriter::new(*b"HAK ");
        local
            .add(mg_core::ResRef::from_str("mine").unwrap(), ResType::TXT, b"local".to_vec())
            .unwrap();
        std::fs::write(user.join("hak/local.hak"), local.to_bytes().unwrap()).unwrap();
        let table = b"2DA V2.0\n\n A\n0 synced\n".to_vec();
        let tlk = b"TLK V3.0 a talk table of the server's".to_vec();
        let (k_table, k_tlk) = (
            ResKey::parse("synced", ResType::TWODA).unwrap(),
            ResKey::parse("mytlk", ResType::TLK).unwrap(),
        );
        let entries =
            [(k_table, sha1(&table), table.len() as u32), (k_tlk, sha1(&tlk), tlk.len() as u32)];
        let manifest = mg_nwsync::manifest::write(&entries);
        let url = "https://nwsync.example.org";
        let storage = |data: Vec<(mg_nwsync::Sha1, Vec<u8>)>| {
            let _ = std::fs::remove_dir_all(user.join("nwsync"));
            mg_nwsync::fixtures::write_storage(
                &user.join("nwsync"),
                &[mg_nwsync::fixtures::Registered {
                    sha1: sha1(&manifest),
                    origin_url: url.into(),
                    created_at: 1,
                    last_used_at: 1,
                    resources: entries.iter().map(|(k, s, _)| (*k, *s)).collect(),
                    cached: Some(manifest.clone()),
                }],
                &[data],
            );
        };
        storage(vec![(sha1(&tlk), tlk.clone())]);
        let gi = GameInstall::new(&root, Some(user.clone()), "en");
        let folder = dir.join("module");
        let mut m = mg_module::Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        let haks = ["synced_hak", "local"].map(|h| {
            let mut s = mg_gff::Struct::new(8);
            s.set("Mod_Hak", mg_gff::Value::String(h.as_bytes().to_vec()));
            s
        });
        info.root.set("Mod_HakList", mg_gff::Value::List(haks.to_vec()));
        info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mytlk".to_vec()));
        m.set_info(&info).unwrap();
        m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
        std::fs::write(folder.join("moonglow.toml"), format!("[nwsync]\nsource = \"{url}\"\n"))
            .unwrap();
        std::fs::create_dir_all(folder.join(".moonglow")).unwrap();
        let cache = dir.join("cache");
        let own = format!("[nwsync]\ncache = '{}'\n", cache.display());
        std::fs::write(folder.join(".moonglow/user.toml"), own).unwrap();
        let origins =
            "Erf:/srv/hak/synced_hak.hak\n\tsynced.2da\nResFile:/srv/tlk/mytlk.tlk\n\tmytlk.tlk\n";
        mg_nwsync::Cache::new(&cache)
            .put_manifest(&sha1(&manifest), "origin", origins.as_bytes())
            .unwrap();
        let server = dir.join("server");

        // The 2DA's data isn't here yet.
        let e = prepare(&gi, &folder, &server, None, false, false).unwrap_err();
        assert!(format!("{e:#}").contains("mg nwsync-pull"), "{e:#}");
        assert!(!server.join("hak/synced_hak.hak").exists());

        storage(vec![
            (sha1(&tlk), tlk.clone()),
            (sha1(&table), mg_erf::compressedbuf::compress(&table, *b"NSYC").unwrap()),
        ]);
        let (_, notes) = prepare(&gi, &folder, &server, None, false, false).unwrap();
        assert!(notes.iter().any(|n| n.starts_with("1 hak from NWSync manifest")), "{notes:?}");
        let hak = std::fs::read(server.join("hak/synced_hak.hak")).unwrap();
        // (Hard linked from the cache: no link to it that a prune breaks.)
        let placed = std::fs::symlink_metadata(server.join("hak/synced_hak.hak")).unwrap();
        assert!(!placed.file_type().is_symlink());
        let erf = mg_erf::Erf::read(&hak).unwrap();
        assert_eq!(&*erf.get(&k_table.resref, k_table.restype).unwrap().unwrap(), &table[..]);
        assert_eq!(
            std::fs::read(server.join("hak/local.hak")).unwrap(),
            std::fs::read(user.join("hak/local.hak")).unwrap()
        );
        assert_eq!(std::fs::read(server.join("tlk/mytlk.tlk")).unwrap(), tlk);
        // (Again: as written before.)
        let (_, notes) = prepare(&gi, &folder, &server, None, false, false).unwrap();
        assert!(notes.iter().any(|n| n.ends_with("all written before")), "{notes:?}");
        let mut names: Vec<_> =
            std::fs::read_dir(user.join("hak")).unwrap().flatten().map(|e| e.file_name()).collect();
        names.sort();
        assert_eq!(names, ["local.hak"]);
        assert_eq!(std::fs::read_dir(user.join("tlk")).unwrap().count(), 0);
    }

    /// A module folder whose project names a server (`moonglow.toml`)
    /// whose manifest (registered in the game's storage of `user` at
    /// `came`, Unix seconds) has `synced.2da` (`table`) in hak
    /// `synced_hak` by its origins file and the talk tables `mytlk` and
    /// `mytlkf`; the builder's own file (`own`) sets a cache in `dir`.
    fn served_from_nwsync(dir: &Path, user: &Path, table: &[u8], came: i64, own: &str) -> PathBuf {
        use mg_core::sha1::sha1;
        let (tlk, tlkf) = (b"TLK male".to_vec(), b"TLK female".to_vec());
        let k = |n: &str, t| ResKey::parse(n, t).unwrap();
        let entries = [
            (k("synced", ResType::TWODA), sha1(table), table.len() as u32),
            (k("mytlk", ResType::TLK), sha1(&tlk), tlk.len() as u32),
            (k("mytlkf", ResType::TLK), sha1(&tlkf), tlkf.len() as u32),
        ];
        let manifest = mg_nwsync::manifest::write(&entries);
        let url = "https://nwsync.example.org";
        let _ = std::fs::remove_dir_all(user.join("nwsync"));
        mg_nwsync::fixtures::write_storage(
            &user.join("nwsync"),
            &[mg_nwsync::fixtures::Registered {
                sha1: sha1(&manifest),
                origin_url: url.into(),
                created_at: came,
                last_used_at: came,
                resources: entries.iter().map(|(k, s, _)| (*k, *s)).collect(),
                cached: Some(manifest.clone()),
            }],
            &[vec![(sha1(table), table.to_vec()), (sha1(&tlk), tlk), (sha1(&tlkf), tlkf)]],
        );
        let folder = dir.join("module");
        if !folder.exists() {
            let mut m = mg_module::Module::new();
            let mut info = mg_gff::Gff::new(*b"IFO ");
            let mut hak = mg_gff::Struct::new(8);
            hak.set("Mod_Hak", mg_gff::Value::String(b"synced_hak".to_vec()));
            info.root.set("Mod_HakList", mg_gff::Value::List(vec![hak]));
            info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mytlk".to_vec()));
            m.set_info(&info).unwrap();
            m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
            std::fs::write(folder.join("moonglow.toml"), format!("[nwsync]\nsource = \"{url}\"\n"))
                .unwrap();
            std::fs::create_dir_all(folder.join(".moonglow")).unwrap();
        }
        let cache = dir.join("cache");
        let own = format!("[nwsync]\ncache = '{}'\n{own}", cache.display());
        std::fs::write(folder.join(".moonglow/user.toml"), own).unwrap();
        let origins = "Erf:/srv/hak/synced_hak.hak\n\tsynced.2da\n";
        mg_nwsync::Cache::new(&cache)
            .put_manifest(&sha1(&manifest), "origin", origins.as_bytes())
            .unwrap();
        folder
    }

    /// A file's time set to `secs` (Unix seconds).
    fn set_time(path: &Path, secs: i64) {
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(std::time::UNIX_EPOCH + Duration::from_secs(secs as u64)).unwrap();
    }

    const CAME: i64 = 1_700_000_000;

    /// The rule: a hak and a talk table on disk newer than the manifest
    /// are placed from disk, and said; older, the manifest's are (the
    /// feminine table with the table, from where it comes). `--no-nwsync`
    /// and `source = ""` in the builder's own file take everything from
    /// disk, the manifest's data not needed at all.
    #[test]
    fn newer_files_on_disk_and_nwsync_turned_off() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let dir = mg_testkit::scratch_dir("mg-serve-nwsync-rule");
        let user = dir.join("user");
        for sub in ["hak", "tlk"] {
            std::fs::create_dir_all(user.join(sub)).unwrap();
        }
        let gi = GameInstall::new(&root, Some(user.clone()), "en");
        let table = b"2DA V2.0\n\n A\n0 synced\n";
        let folder = served_from_nwsync(&dir, &user, table, CAME, "");
        let mut local = mg_erf::ErfWriter::new(*b"HAK ");
        local.add(mg_core::ResRef::from_str("mine").unwrap(), ResType::TXT, b"x".to_vec()).unwrap();
        let disk_hak = user.join("hak/synced_hak.hak");
        let (male, female) = (user.join("tlk/mytlk.tlk"), user.join("tlk/mytlkf.tlk"));
        std::fs::write(&disk_hak, local.to_bytes().unwrap()).unwrap();
        std::fs::write(user.join("tlk/mytlk.tlk"), b"disk male").unwrap();
        std::fs::write(user.join("tlk/mytlkf.tlk"), b"disk female").unwrap();
        let server = dir.join("server");
        let read = |p: &str| std::fs::read(server.join(p)).unwrap();

        // Older than the manifest: the manifest's.
        for f in [&disk_hak, &male, &female] {
            set_time(f, CAME - 60);
        }
        let (_, notes) = prepare(&gi, &folder, &server, None, false, false).unwrap();
        assert!(!notes.iter().any(|n| n.contains("newer")), "{notes:?}");
        let hak = read("hak/synced_hak.hak");
        let erf = mg_erf::Erf::read(&hak).unwrap();
        let synced = mg_core::ResRef::from_str("synced").unwrap();
        assert_eq!(&*erf.get(&synced, ResType::TWODA).unwrap().unwrap(), &table[..]);
        assert_eq!(
            (read("tlk/mytlk.tlk"), read("tlk/mytlkf.tlk")),
            (b"TLK male".to_vec(), b"TLK female".to_vec())
        );

        // Newer: from disk, said.
        for f in [&disk_hak, &male] {
            set_time(f, CAME + 60);
        }
        let (_, notes) = prepare(&gi, &folder, &server, None, false, false).unwrap();
        for what in ["hak synced_hak", "talk table mytlk"] {
            let note = format!("{what} on disk is newer than the NWSync manifest: used over it");
            assert!(notes.contains(&note), "{notes:?}");
        }
        assert_eq!(read("hak/synced_hak.hak"), std::fs::read(&disk_hak).unwrap());
        assert_eq!(
            (read("tlk/mytlk.tlk"), read("tlk/mytlkf.tlk")),
            (b"disk male".to_vec(), b"disk female".to_vec())
        );

        // Turned off: the manifest's data needn't be here.
        let _ = std::fs::remove_dir_all(user.join("nwsync"));
        set_time(&disk_hak, CAME - 60);
        let (_, notes) = prepare(&gi, &folder, &server, None, false, true).unwrap();
        assert!(!notes.iter().any(|n| n.contains("NWSync")), "{notes:?}");
        assert_eq!(read("hak/synced_hak.hak"), std::fs::read(&disk_hak).unwrap());
        assert!(prepare(&gi, &folder, &server, None, false, false).is_err());
        served_from_nwsync(&dir, &user, table, CAME, "source = \"\"\n");
        let _ = std::fs::remove_dir_all(user.join("nwsync"));
        let (_, notes) = prepare(&gi, &folder, &server, None, false, false).unwrap();
        assert!(!notes.iter().any(|n| n.contains("NWSync")), "{notes:?}");
        assert_eq!(read("hak/synced_hak.hak"), std::fs::read(&disk_hak).unwrap());
    }

    /// `--real-user-dir`: the server's folder is the player's own. A hak
    /// or talk table of theirs there that the manifest's would replace is
    /// refused, the file left as it was; one newer than the manifest is
    /// theirs to use. What Moonglow put there (links) is replaced, a
    /// second run with another manifest too.
    #[test]
    fn the_players_own_files_are_never_replaced() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let dir = mg_testkit::scratch_dir("mg-serve-nwsync-real-dir");
        let user = dir.join("user");
        for sub in ["hak", "tlk"] {
            std::fs::create_dir_all(user.join(sub)).unwrap();
        }
        let gi = GameInstall::new(&root, Some(user.clone()), "en");
        assert!(mg_module::server_dir::is_players(&gi, &user));
        let folder = served_from_nwsync(&dir, &user, b"2DA V2.0\n\n A\n0 one\n", CAME, "");
        // Nothing of theirs there: links to the cache, and again for
        // another manifest.
        prepare(&gi, &folder, &user, None, false, false).unwrap();
        let link = user.join("hak/synced_hak.hak");
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        let first = std::fs::read(&link).unwrap();
        served_from_nwsync(&dir, &user, b"2DA V2.0\n\n A\n0 two\n", CAME + 10, "");
        let (_, notes) = prepare(&gi, &folder, &user, None, false, false).unwrap();
        // (Its own link there is not taken for a file of the player's.)
        assert!(!notes.iter().any(|n| n.contains("newer")), "{notes:?}");
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_ne!(std::fs::read(&link).unwrap(), first);
        // A talk table of their own, older than the manifest: refused.
        let theirs = user.join("tlk/mytlk.tlk");
        std::fs::remove_file(&theirs).unwrap();
        std::fs::write(&theirs, b"the player's").unwrap();
        set_time(&theirs, CAME - 60);
        let e = prepare(&gi, &folder, &user, None, false, false).unwrap_err();
        assert!(format!("{e:#}").contains("a file of your own NWN folder"), "{e:#}");
        assert_eq!(std::fs::read(&theirs).unwrap(), b"the player's");
        // A hak of their own, newer: theirs, used as it is.
        set_time(&theirs, CAME + 60);
        std::fs::remove_file(&link).unwrap();
        let their_hak = mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap();
        std::fs::write(&link, &their_hak).unwrap();
        set_time(&link, CAME + 60);
        let (_, notes) = prepare(&gi, &folder, &user, None, false, false).unwrap();
        assert!(
            notes.iter().any(|n| n.starts_with("hak synced_hak on disk is newer")),
            "{notes:?}"
        );
        assert_eq!(std::fs::read(&link).unwrap(), their_hak);
        assert_eq!(std::fs::read(&theirs).unwrap(), b"the player's");
    }

    /// The player's own folder is refused without an install too (`mg
    /// nwsync-haks --out` with no game found): the user folder `mg` was
    /// given.
    #[test]
    fn the_players_folder_is_refused_without_an_install() {
        let dir = mg_testkit::scratch_dir("mg-refuse-players-dir");
        let user = dir.join("user");
        std::fs::create_dir_all(&user).unwrap();
        let e = refuse_players_dir(Some(&user), &user, None).unwrap_err();
        assert!(e.to_string().contains("is your own NWN folder"), "{e}");
        assert!(refuse_players_dir(Some(&user), &user, Some(true)).is_ok());
        assert!(refuse_players_dir(Some(&user), &dir.join("other"), None).is_ok());
    }

    /// Every entry under `dir` (links not followed) with its contents: a
    /// player's made-up folder, compared before and after.
    fn entries(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut all = Vec::new();
        let mut todo = vec![dir.to_path_buf()];
        while let Some(d) = todo.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let path = e.path();
                let rel = path.strip_prefix(dir).unwrap().to_path_buf();
                let kind = e.file_type().unwrap();
                if kind.is_dir() {
                    todo.push(path);
                    all.push((rel, Vec::new()));
                } else {
                    all.push((rel, std::fs::read(&path).unwrap_or_default()));
                }
            }
        }
        all.sort();
        all
    }

    /// A made-up player's folder with a script in `development`.
    fn players_folder(dir: &Path, name: &str, script: &str) -> PathBuf {
        let user = dir.join(name);
        std::fs::create_dir_all(user.join("development")).unwrap();
        std::fs::create_dir_all(user.join("hak")).unwrap();
        std::fs::write(user.join("development/mg_dev.nss"), script).unwrap();
        std::fs::write(user.join("nwn.ini"), "[Alias]\n").unwrap();
        user
    }

    /// The server's folder gets a link to the player's `development`
    /// folder: a file there is read through it, and nothing is made or
    /// changed in the player's folder. Made again, it is still one link.
    #[test]
    fn the_players_development_folder_is_linked_for_the_server() {
        let dir = mg_testkit::scratch_dir("mg-serve-development");
        let user = players_folder(&dir, "user", "// mine\n");
        let server = dir.join("server");
        std::fs::create_dir_all(&server).unwrap();
        let before = entries(&user);
        for _ in 0..2 {
            let note = link_development(Some(&user), &server, Development::Players, false);
            assert_eq!(note.unwrap(), None);
            let link = server.join("development");
            assert!(mg_module::links::is_link(&link));
            assert!(same(&link, &user.join("development")));
            assert_eq!(std::fs::read_to_string(link.join("mg_dev.nss")).unwrap(), "// mine\n");
        }
        assert_eq!(entries(&user), before);
    }

    /// No `development` folder of the player's: no link, no note, and
    /// none made in the player's folder; none for `mg watch`'s own folder
    /// either, nor with no user folder at all.
    #[test]
    fn no_development_folder_means_no_link() {
        let dir = mg_testkit::scratch_dir("mg-serve-no-development");
        let user = dir.join("user");
        std::fs::create_dir_all(user.join("hak")).unwrap();
        let server = dir.join("server");
        std::fs::create_dir_all(&server).unwrap();
        let before = entries(&user);
        let note = link_development(Some(&user), &server, Development::Players, false);
        assert_eq!(note.unwrap(), None);
        assert!(std::fs::symlink_metadata(server.join("development")).is_err());
        assert_eq!(entries(&user), before);

        let user = players_folder(&dir, "user2", "");
        assert_eq!(
            link_development(Some(&user), &server, Development::Watched, false).unwrap(),
            None
        );
        assert_eq!(link_development(None, &server, Development::Players, false).unwrap(), None);
        assert!(std::fs::symlink_metadata(server.join("development")).is_err());
    }

    /// A `--dir` with a development folder of its own keeps it, with its
    /// files, and says so; in Moonglow's own scratch folder one (`mg watch
    /// --serve`'s) gives way to the link.
    #[test]
    fn a_servers_own_development_folder_is_kept() {
        let dir = mg_testkit::scratch_dir("mg-serve-own-development");
        let user = players_folder(&dir, "user", "// mine\n");
        let server = dir.join("server");
        std::fs::create_dir_all(server.join("development")).unwrap();
        std::fs::write(server.join("development/builder.nss"), "// builder's\n").unwrap();
        let note = link_development(Some(&user), &server, Development::Players, false).unwrap();
        let note = note.expect("a note");
        assert!(note.contains("of its own"), "{note}");
        assert!(!mg_module::links::is_link(&server.join("development")));
        let kept = std::fs::read_to_string(server.join("development/builder.nss")).unwrap();
        assert_eq!(kept, "// builder's\n");

        let note = link_development(Some(&user), &server, Development::Players, true).unwrap();
        assert_eq!(note, None);
        assert!(mg_module::links::is_link(&server.join("development")));
        assert!(user.join("development/mg_dev.nss").is_file());
        assert!(!user.join("development/builder.nss").exists());
    }

    /// The link leads into the `development` folder of the user folder
    /// the run was given (`--user-dir`), made again when it changes; a
    /// relative one is made absolute.
    #[test]
    fn the_development_link_follows_the_user_folder() {
        let dir = mg_testkit::scratch_dir("mg-serve-development-user-dir");
        let a = players_folder(&dir, "a", "// a\n");
        let b = players_folder(&dir, "b", "// b\n");
        let server = dir.join("server");
        std::fs::create_dir_all(&server).unwrap();
        let script = || std::fs::read_to_string(server.join("development/mg_dev.nss")).unwrap();
        link_development(Some(&a), &server, Development::Players, false).unwrap();
        assert_eq!(script(), "// a\n");
        link_development(Some(&b), &server, Development::Players, false).unwrap();
        assert_eq!(script(), "// b\n");
        assert!(same(&server.join("development"), &b.join("development")));
        // (A user folder that no longer has one: the old link goes.)
        std::fs::remove_dir_all(b.join("development")).unwrap();
        link_development(Some(&b), &server, Development::Players, false).unwrap();
        assert!(std::fs::symlink_metadata(server.join("development")).is_err());
        // (A relative user folder: the link still leads to it from the
        // server's folder.)
        let cwd = std::env::current_dir().unwrap();
        if let Some(rel) = pathdiff(&a, &cwd) {
            link_development(Some(&rel), &server, Development::Players, false).unwrap();
            assert_eq!(script(), "// a\n");
        }
    }

    /// `a` relative to `base` (`..` where it goes up), where both are on
    /// one drive.
    fn pathdiff(a: &Path, base: &Path) -> Option<PathBuf> {
        let (a, base): (Vec<_>, Vec<_>) = (a.components().collect(), base.components().collect());
        if a.first() != base.first() {
            return None;
        }
        let common = a.iter().zip(&base).take_while(|(x, y)| x == y).count();
        let mut rel: PathBuf = base[common..].iter().map(|_| "..").collect();
        rel.extend(a[common..].iter());
        Some(rel)
    }

    /// With `--real-user-dir` the player's own folder is the server's: its
    /// `development` folder is already there, and is left as it is.
    #[test]
    fn the_players_own_folder_gets_no_link() {
        let dir = mg_testkit::scratch_dir("mg-serve-development-real");
        let user = players_folder(&dir, "user", "// mine\n");
        let before = entries(&user);
        let note = link_development(Some(&user), &user, Development::Players, false).unwrap();
        assert_eq!(note, None);
        assert!(!mg_module::links::is_link(&user.join("development")));
        assert_eq!(entries(&user), before);
    }

    /// `mg watch --serve` writes into the server's own development folder:
    /// a link `mg serve` left there goes first, and the player's folder is
    /// not written through it.
    #[test]
    fn mg_watch_takes_the_link_away_first() {
        let dir = mg_testkit::scratch_dir("mg-serve-development-watch");
        let user = players_folder(&dir, "user", "// mine\n");
        let server = dir.join("server");
        std::fs::create_dir_all(&server).unwrap();
        link_development(Some(&user), &server, Development::Players, false).unwrap();
        assert!(mg_module::links::is_link(&server.join("development")));
        let before = entries(&user);
        unlink_development(&server).unwrap();
        assert!(std::fs::symlink_metadata(server.join("development")).is_err());
        std::fs::create_dir_all(server.join("development")).unwrap();
        std::fs::write(server.join("development/watched.ncs"), b"x").unwrap();
        assert_eq!(entries(&user), before);
        // (A folder of its own is left.)
        unlink_development(&server).unwrap();
        assert!(server.join("development/watched.ncs").is_file());
    }

    #[test]
    fn a_scratch_directory_is_the_module_s_own() {
        let module = |p: &str| Path::new(p).join(format!("a{}.mod", std::process::id()));
        let (a, b) = (run_dir("serve", &module("/x")), run_dir("serve", &module("/y")));
        assert_ne!(a.dir, b.dir);
        assert!(a.dir.starts_with(std::env::temp_dir().join("moonglow-serve")));
        assert!(
            run_dir("test", &module("/x"))
                .dir
                .starts_with(std::env::temp_dir().join("moonglow-test"))
        );
        // Held: another run of the module at once takes another, and the
        // next run after it the first again.
        let again = run_dir("serve", &module("/x"));
        assert_eq!(
            again.dir.file_name().unwrap().to_string_lossy(),
            format!("{}-2", a.dir.file_name().unwrap().to_string_lossy())
        );
        let first = a.dir.clone();
        drop((a, again));
        assert_eq!(run_dir("serve", &module("/x")).dir, first);
    }

    #[test]
    fn module_name_handles_dot_and_bare_nasher_cfg() {
        let cwd = std::env::current_dir().unwrap();
        let expected: String = cwd
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        assert_eq!(module_name(Path::new(".")), expected);
        assert_eq!(module_name(Path::new("nasher.cfg")), expected);
    }

    #[test]
    fn prepare_places_optional_feminine_tlk_and_propagates_corrupt_hak() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let dir = mg_testkit::scratch_dir("mg-serve-fem-and-hak");
        let _ = std::fs::remove_dir_all(&dir);
        let user = dir.join("user");
        for sub in ["tlk", "hak"] {
            std::fs::create_dir_all(user.join(sub)).unwrap();
        }
        let gi = GameInstall::new(&root, Some(user.clone()), "en");

        // Project JSON primary + feminine TLK.
        let mut m = mg_module::Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mytlk".to_vec()));
        m.set_info(&info).unwrap();
        let project = dir.join("project");
        m.save_as(&ModuleLocation::Project { root: project.clone(), target: "default".into() })
            .unwrap();
        let mut cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
        cfg.push_str(
            "\n[target]\nname = \"tlk\"\nfile = \"mytlk.tlk\"\n  [target.sources]\n  include = \"src/tlk/*.json\"\n\
             \n[target]\nname = \"tlkf\"\nfile = \"mytlkf.tlk\"\n  [target.sources]\n  include = \"src/tlk/*.json\"\n",
        );
        std::fs::write(project.join("nasher.cfg"), cfg).unwrap();
        std::fs::create_dir_all(project.join("src/tlk")).unwrap();
        std::fs::write(
            project.join("src/tlk/mytlk.tlk.json"),
            r#"{"language":0,"entries":[{"id":0,"text":"Ours"}]}"#,
        )
        .unwrap();
        std::fs::write(
            project.join("src/tlk/mytlkf.tlk.json"),
            r#"{"language":0,"entries":[{"id":0,"text":"Fem"}]}"#,
        )
        .unwrap();
        let server = dir.join("server_proj");
        prepare(&gi, &project, &server, None, false, false).unwrap();
        let fem =
            mg_tlk::Tlk::read(&std::fs::read(server.join("tlk/mytlkf.tlk")).unwrap()).unwrap();
        assert_eq!(fem.entries[0].text, b"Fem");

        // Loose TLK files: copies <tlk>f.tlk when present, no warning when absent.
        std::fs::write(user.join("tlk/loose.tlk"), b"male").unwrap();
        std::fs::write(user.join("tlk/loosef.tlk"), b"female").unwrap();
        let loose_mod = dir.join("loose.mod");
        let mut lm = mg_module::Module::new();
        let mut linfo = mg_gff::Gff::new(*b"IFO ");
        linfo.root.set("Mod_CustomTlk", mg_gff::Value::String(b"loose".to_vec()));
        lm.set_info(&linfo).unwrap();
        lm.save_as(&ModuleLocation::Archive(loose_mod.clone())).unwrap();
        let server2 = dir.join("server_loose");
        let (_, notes) = prepare(&gi, &loose_mod, &server2, None, false, false).unwrap();
        assert_eq!(std::fs::read(server2.join("tlk/loosef.tlk")).unwrap(), b"female");
        assert!(!notes.iter().any(|n| n.contains("not found")), "{notes:?}");

        std::fs::write(user.join("tlk/onlymale.tlk"), b"male").unwrap();
        linfo.root.set("Mod_CustomTlk", mg_gff::Value::String(b"onlymale".to_vec()));
        lm.set_info(&linfo).unwrap();
        let male_mod = dir.join("male.mod");
        lm.save_as(&ModuleLocation::Archive(male_mod.clone())).unwrap();
        let (_, notes) =
            prepare(&gi, &male_mod, &dir.join("server_male"), None, false, false).unwrap();
        assert!(!notes.iter().any(|n| n.contains("not found")), "{notes:?}");

        // Module folder referencing a corrupt hak surfaces ResError.
        std::fs::write(user.join("hak/corrupt.hak"), b"not an erf").unwrap();
        let bad_dir = dir.join("bad_folder");
        let mut bm = mg_module::Module::new();
        let mut binfo = mg_gff::Gff::new(*b"IFO ");
        let mut hak = mg_gff::Struct::new(8);
        hak.set("Mod_Hak", mg_gff::Value::String(b"corrupt".to_vec()));
        binfo.root.set("Mod_HakList", mg_gff::Value::List(vec![hak]));
        bm.set_info(&binfo).unwrap();
        bm.save_as(&ModuleLocation::Folder(bad_dir.clone())).unwrap();
        assert!(prepare(&gi, &bad_dir, &dir.join("server_bad"), None, false, false).is_err());
    }

    /// A library the loader can preload into a shell: a copy of the C
    /// library's `libdl` (a stub in glibc 2.34 and later), named `name`.
    #[cfg(target_os = "linux")]
    fn a_real_library(dir: &Path, name: &str) -> Option<PathBuf> {
        let found =
            ["/usr/lib/libdl.so.2", "/lib/x86_64-linux-gnu/libdl.so.2", "/usr/lib64/libdl.so.2"]
                .into_iter()
                .map(PathBuf::from)
                .find(|p| p.is_file())?;
        std::fs::create_dir_all(dir).unwrap();
        let to = dir.join(name);
        std::fs::copy(found, &to).unwrap();
        Some(to)
    }

    /// A fake NWNX home: `Binaries/` with `NWNX_Core.so` and `plugins`
    /// (empty files), and a project folder using it with `env` as its
    /// environment file; resolved with `more` (`--env`'s lines).
    fn fake_nwnx(
        dir: &Path,
        plugins: &[&str],
        env: &str,
        more: &[(String, String)],
    ) -> mg_module::nwnx::Resolved {
        let bin = dir.join("nwnx/Binaries");
        std::fs::create_dir_all(&bin).unwrap();
        for p in std::iter::once("Core").chain(plugins.iter().copied()) {
            std::fs::write(bin.join(format!("NWNX_{p}.so")), b"").unwrap();
        }
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let home = dir.join("nwnx").display().to_string().replace('\\', "/");
        let team = format!("[nwnx]\nuse = true\nhome = \"{home}\"\nenv = \"nwnx.env\"\n");
        std::fs::write(root.join("moonglow.toml"), team).unwrap();
        std::fs::write(root.join("nwnx.env"), env).unwrap();
        let project = mg_module::project_file::ProjectFile::read(&root);
        let o = mg_module::nwnx::Options { linux: true, ..Default::default() };
        mg_module::nwnx::resolve_with(&project, &o, more)
    }

    fn lines(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn texts(env: &ServerEnv) -> Vec<(String, String)> {
        (env.vars.iter())
            .map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned()))
            .collect()
    }

    /// The server's environment with NWNX: its core and plugins' folder,
    /// the project's file's lines, then --env's (which win: last); and
    /// --env's alone without NWNX.
    #[test]
    fn nwnx_s_environment_then_the_env_files() {
        let dir = mg_testkit::scratch_dir("mg-serve-env-order");
        let more = lines(&[("NWNX_CORE_LOG_LEVEL", "7"), ("MY_SETTING", "x")]);
        let r = fake_nwnx(&dir, &["Util"], "NWNX_CORE_LOG_LEVEL=6\nNWNX_UTIL_SKIP=n\n", &more);
        let n = r.nwnx.unwrap_or_else(|| panic!("{:?}", r.notes));
        let env = server_env(Some(&n), &more, None, true).unwrap();
        let bin = dir.join("nwnx/Binaries");
        let want = lines(&[
            ("LD_PRELOAD", &bin.join("NWNX_Core.so").display().to_string()),
            ("NWNX_CORE_LOAD_PATH", &bin.display().to_string()),
            ("NWNX_CORE_LOG_LEVEL", "6"),
            ("NWNX_UTIL_SKIP", "n"),
            ("NWNX_CORE_LOG_LEVEL", "7"),
            ("MY_SETTING", "x"),
        ]);
        assert_eq!(texts(&env), want);
        assert!(env.nwnx);
        let env = server_env(None, &more, None, true).unwrap();
        assert_eq!(texts(&env), more);
        assert!(!env.nwnx);
    }

    /// --preload: another library alone without NWNX (and no NWNX check);
    /// an NWNX_Core.so of --preload's, or of an --env LD_PRELOAD, is NWNX;
    /// with the project's NWNX, its own core once, another library beside
    /// it, another NWNX's core refused; not off Linux.
    #[test]
    fn preloads_and_what_is_nwnx() {
        let dir = mg_testkit::scratch_dir("mg-serve-preload");
        let r = fake_nwnx(&dir, &[], "", &[]);
        let n = r.nwnx.unwrap_or_else(|| panic!("{:?}", r.notes));
        let lib = dir.join("lib/libjemalloc.so");
        let other = dir.join("other/Binaries/NWNX_Core.so");
        for f in [&lib, &other] {
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, b"").unwrap();
        }
        let (lib, other) = (lib.canonicalize().unwrap(), other.canonicalize().unwrap());
        let core = n.core.canonicalize().unwrap();
        let preloaded = |e: &ServerEnv| -> Vec<String> {
            texts(e).into_iter().filter(|(k, _)| k == "LD_PRELOAD").map(|(_, v)| v).collect()
        };
        // Without NWNX (--no-nwnx, or none): the library alone, no NWNX.
        let e = server_env(None, &[], Some(&lib), true).unwrap();
        assert_eq!(preloaded(&e), [lib.display().to_string()]);
        assert!(!e.nwnx, "another library is not NWNX");
        // An NWNX core given so: NWNX, checked.
        assert!(server_env(None, &[], Some(&other), true).unwrap().nwnx);
        let given = lines(&[("LD_PRELOAD", &other.display().to_string())]);
        assert!(server_env(None, &given, None, true).unwrap().nwnx);
        // With the project's: its core once; another library beside it.
        let e = server_env(Some(&n), &[], Some(&n.core), true).unwrap();
        assert_eq!(preloaded(&e), [n.core.display().to_string()]);
        assert!(e.nwnx);
        let e = server_env(Some(&n), &[], Some(&lib), true).unwrap();
        assert_eq!(preloaded(&e), [format!("{}:{}", core.display(), lib.display())]);
        assert!(e.nwnx);
        let e = server_env(Some(&n), &[], Some(&other), true).unwrap_err().to_string();
        assert!(e.contains("is another NWNX than the project's"), "{e}");
        assert!(server_env(None, &[], Some(&lib), false).is_err(), "LD_PRELOAD is Linux's");
    }

    /// What the server says of NWNX is held to it only with NWNX: a module
    /// loaded without "Starting NWNX" stops a run with NWNX, not one
    /// without; another library the loader refused is a warning.
    #[test]
    fn nwnx_s_checks_only_with_nwnx() {
        let line = |l: &str| mg_serve::Line { text: l.to_string(), stderr: false };
        let refused = "ERROR: ld.so: object '/usr/lib/libjemalloc.so' from LD_PRELOAD cannot be \
                       preloaded (wrong ELF class): ignored.";
        let mut without = Heard::new(false);
        let mut said = Vec::new();
        for l in [refused, "Server: Module loaded"] {
            said.extend(console_events(line(l), &mut without));
        }
        assert!(!without.nwnx_failed, "{said:?}");
        let warned = said.iter().any(|e| matches!(e, Event::Note(n) if n.starts_with("warning: ")));
        assert!(warned, "{said:?}");
        let mut with = Heard::new(true);
        console_events(line("Server: Module loaded"), &mut with);
        assert!(with.nwnx_failed, "NWNX asked for and never heard of");
    }

    /// A served module, end to end with a stand-in server (a shell script
    /// that writes down its environment): nothing of the caller's own
    /// environment reaches it but the kept variables, NWNX or not; --env's
    /// lines do; `--no-nwnx --preload` of another library loads it and the
    /// run goes on (no "ran without NWNX").
    #[cfg(target_os = "linux")]
    #[test]
    fn a_served_module_gets_what_it_is_given_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = mg_testkit::scratch_dir("mg-serve-env-given-alone");
        let Some(lib) = a_real_library(&dir.join("lib"), "libextra.so") else { return };
        let root = dir.join("root");
        let server = root.join("bin/linux-x86/nwserver-linux");
        std::fs::create_dir_all(server.parent().unwrap()).unwrap();
        let env_out = dir.join("env.txt");
        let script = format!(
            "#!/bin/sh\nenv > '{}'\necho 'Server: Module loaded'\n\
             while read line; do [ \"$line\" = exit ] && exit 0; done\n",
            env_out.display()
        );
        std::fs::write(&server, script).unwrap();
        std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o755)).unwrap();
        let user = dir.join("user");
        std::fs::create_dir_all(&user).unwrap();
        let gi = GameInstall::new(&root, Some(user), "en");
        let module = dir.join("m.mod");
        let mut m = mg_module::Module::new();
        m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        m.save_as(&ModuleLocation::Archive(module.clone())).unwrap();
        let env_file = dir.join("given.env");
        std::fs::write(&env_file, "MG_GIVEN=yes\n").unwrap();
        // (A variable of this process's that is not kept, if it has one:
        // cargo gives a test its CARGO_* ones.)
        let own = std::env::vars().map(|(k, _)| k).find(|k| {
            let kept = mg_serve::KEPT_VARIABLES.iter().any(|v| v.eq_ignore_ascii_case(k));
            !kept
                && !k.starts_with("LC_")
                && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
        let run = |preload: Option<PathBuf>| {
            let c = Config {
                module: module.clone(),
                dir: Some(dir.join("server")),
                env: vec![env_file.clone()],
                preload,
                nwnx: mg_module::nwnx::Choice::Off,
                ..Default::default()
            };
            let mut said = Vec::new();
            let end = serve(&gi, &c, &mut |e| {
                said.push(e.clone());
                match e {
                    Event::Ready { .. } => Flow::Stop,
                    _ => Flow::Continue,
                }
            })
            .unwrap();
            (end, said, std::fs::read_to_string(&env_out).unwrap())
        };
        // Without NWNX or a preload: clean all the same.
        let (end, said, got) = run(None);
        assert_eq!(end, End::Stopped, "{said:?}");
        assert!(got.lines().any(|l| l == "MG_GIVEN=yes"), "{got}");
        if let Some(own) = &own {
            assert!(!got.lines().any(|l| l.starts_with(&format!("{own}="))), "{own} passed: {got}");
        }
        // --no-nwnx --preload of another library: loaded, no NWNX check.
        let (end, said, got) = run(Some(lib.clone()));
        assert_eq!(end, End::Stopped, "{said:?}");
        let preload = format!("LD_PRELOAD={}", lib.canonicalize().unwrap().display());
        assert!(got.lines().any(|l| l == preload), "{got}");
        assert!(
            !said.iter().any(|e| matches!(e, Event::Note(n) if n.contains("NWNX"))),
            "{said:?}"
        );
    }

    /// The note of a module folder beside the .mod: once in a process,
    /// with the output (as `open_module` says it), not at each start.
    #[test]
    fn the_folder_beside_the_mod_is_said_once() {
        let dir = mg_testkit::scratch_dir("mg-serve-beside-once");
        // (No game install needed: the module names no hak or talk table.)
        let gi = GameInstall::new(dir.join("root"), Some(dir.join("user")), "en");
        let module = dir.join("beside.mod");
        let mut m = mg_module::Module::new();
        m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        m.save_as(&ModuleLocation::Archive(module.clone())).unwrap();
        m.save_as(&ModuleLocation::Folder(dir.join("beside"))).unwrap();
        let server = dir.join("server");
        for _ in 0..2 {
            let (_, notes) = prepare(&gi, &module, &server, None, false, false).unwrap();
            assert!(!notes.iter().any(|n| n.contains("beside it")), "{notes:?}");
        }
        let once = crate::opened_notes()
            .into_iter()
            .filter(|n| n.contains(&*module.display().to_string()));
        assert_eq!(once.count(), 1);
    }
}
