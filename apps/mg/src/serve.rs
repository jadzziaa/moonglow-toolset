//! `mg serve`: a module in the game's headless server, on a user
//! directory that is not the player's, its log followed and its console
//! passed through. `mg test` runs its tests through the same.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use mg_core::ResType;
use mg_module::ModuleLocation;
use mg_resman::{GameInstall, ResKey};

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
    pub player_password: Option<String>,
    pub dm_password: Option<String>,
    pub admin_password: Option<String>,
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
            std::fs::OpenOptions::new().create(true).append(true).open(dir.join(".mg-lock"))
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

fn same(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The server's name for a module: what `-module` takes, and the file in
/// `modules`. (Lower case, the letters a resource name may have.)
pub(crate) fn module_name(module: &Path) -> String {
    let path = match module.file_name() {
        Some(n) if n == "nasher.cfg" => module.parent().unwrap_or(module),
        _ => module,
    };
    let stem = match path.is_dir() {
        true => path.file_name(),
        false => path.file_stem(),
    };
    let name: String = stem
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    if name.is_empty() { "module".into() } else { name }
}

/// Puts a file where the server reads it without writing the original: a
/// link where the system allows one, else a copy.
fn place(from: &Path, to: &Path) -> std::io::Result<()> {
    let _ = std::fs::remove_file(to);
    #[cfg(unix)]
    if std::os::unix::fs::symlink(from, to).is_ok() {
        return Ok(());
    }
    std::fs::hard_link(from, to).or_else(|_| std::fs::copy(from, to).map(|_| ()))
}

/// The user directory made ready: the module as an archive in `modules`,
/// its haks in `hak` and its talk table in `tlk` (from the player's
/// folders and the game's, which are only read). Returns the module's
/// name there and what to say.
pub(crate) fn prepare(
    gi: &GameInstall,
    module: &Path,
    dir: &Path,
) -> Result<(String, Vec<String>)> {
    let mut notes = Vec::new();
    let name = module_name(module);
    for sub in ["modules", "hak", "tlk"] {
        std::fs::create_dir_all(dir.join(sub))
            .with_context(|| dir.join(sub).display().to_string())?;
    }
    let target = dir.join("modules").join(format!("{name}.mod"));
    let mut m = crate::open_module(module).with_context(|| module.display().to_string())?;
    let (haks, tlk) = (m.haks()?, m.custom_tlk()?);
    // A talk table the project keeps as JSON, made into the table as
    // nasher packs it (the game's language's, where it keeps one a
    // language): the server finds no other.
    let language = mg_core::Language::from_short_code(&gi.language);
    let own_tlk = match (&tlk, m.project.as_deref()) {
        (Some(name), Some(p)) => mg_module::nasher::in_language(p.talk_tables(name)?, language),
        _ => None,
    };
    if matches!(m.location, Some(ModuleLocation::Archive(_))) {
        if !same(module, &target) {
            std::fs::copy(module, &target).with_context(|| target.display().to_string())?;
        }
    } else {
        // A project or a folder: its scripts that have no compiled script,
        // or are not as they were when last compiled (by `mg compile` or
        // the window: what they remember), are compiled, as `mg build`
        // would (the server runs compiled scripts only, and a module
        // packed without them loads and does nothing). The folder or
        // project itself is not written.
        let mut rm = mg_resman::ResMan::for_game(gi)?;
        let _ = m.stack_onto(&mut rm, gi);
        let made_with = mg_module::build::made_with(Some(module), None, Some(&gi.root), &rm);
        let changed = mg_module::build::changed(&m, &crate::remembered(module, made_with), false);
        if !changed.is_empty() {
            let results = mg_module::build::compile_named_scripts(&mut m, &rm, &changed, None);
            let compiled = (results.iter())
                .filter(|r| r.result.is_ok())
                .filter(|r| m.contains(&ResKey::new(r.script.resref, ResType::NCS)));
            notes.push(format!(
                "compiled {} of the module's scripts that had no compiled script or changed \
                 since they were compiled",
                compiled.count()
            ));
            for r in &results {
                if let Err(e) = &r.result {
                    notes.push(format!("warning: {} did not compile: {}", r.script, e.message));
                }
            }
        }
        // (The server loads an archive only.)
        m.save_as(&ModuleLocation::Archive(target.clone()))?;
        let _ = std::fs::remove_file(target.with_extension("mod.bak"));
    }
    for hak in &haks {
        let file = format!("{hak}.hak");
        let to = dir.join("hak").join(&file);
        match gi.hak_dirs().iter().map(|d| d.join(&file)).find(|p| p.is_file()) {
            Some(from) if !same(&from, &to) => place(&from, &to)?,
            Some(_) => {}
            None if to.is_file() => {}
            None => notes.push(format!("warning: hak {hak} not found")),
        }
    }
    if let Some(tlk) = tlk {
        let file = format!("{tlk}.tlk");
        let to = dir.join("tlk").join(&file);
        if let Some(own) = own_tlk {
            let bytes = own.tlk.to_bytes_as_nwn_tlk()?;
            if !std::fs::read(&to).is_ok_and(|had| had == bytes) {
                // (Not through a link to the player's own folder.)
                let _ = std::fs::remove_file(&to);
                std::fs::write(&to, bytes).with_context(|| to.display().to_string())?;
            }
            notes.push(format!("talk table {tlk} made from {}", own.path.display()));
            return Ok((name, notes));
        }
        match gi.tlk_dirs().iter().map(|d| d.join(&file)).find(|p| p.is_file()) {
            Some(from) if !same(&from, &to) => place(&from, &to)?,
            Some(_) => {}
            None if to.is_file() => {}
            None => notes.push(format!("warning: talk table {tlk} not found")),
        }
    }
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
    let players =
        (gi.user_dir.iter().cloned()).chain(mg_resman::default_user_dir()).find(|p| same(p, &dir));
    if let Some(theirs) = players
        && !c.real_user_dir
    {
        bail!(
            "{} is your own NWN folder: a server there writes its vault, database and settings \
             into it (pass --real-user-dir to serve from it anyway)",
            theirs.display()
        );
    }
    if std::fs::canonicalize(&dir).is_ok_and(|d| d.starts_with(&gi.root)) {
        bail!("{} is in the game's own folder", dir.display());
    }
    let mut env: Vec<(std::ffi::OsString, std::ffi::OsString)> = Vec::new();
    for file in &c.env {
        let text = std::fs::read_to_string(file).with_context(|| file.display().to_string())?;
        let read =
            mg_serve::env_file(&text).map_err(|e| anyhow::anyhow!("{}: {e}", file.display()))?;
        env.extend(read.into_iter().map(|(k, v)| (k.into(), v.into())));
    }
    if let Some(preload) = &c.preload {
        if !cfg!(target_os = "linux") {
            bail!("--preload is the Linux loader's LD_PRELOAD");
        }
        let preload =
            std::fs::canonicalize(preload).with_context(|| preload.display().to_string())?;
        env.push(("LD_PRELOAD".into(), preload.into()));
    }
    let typed = c.stdin.then(|| lines(std::io::stdin()));
    let mut typing = typed.is_some();
    let mut flow = Flow::Continue;
    // (Started again, the server keeps its port and its passwords.)
    let mut port = c.port;
    let mut passwords =
        (c.player_password.clone(), c.dm_password.clone(), c.admin_password.clone());
    loop {
        let (name, notes) = prepare(gi, &c.module, &dir)?;
        for note in notes {
            each(&Event::Note(note));
        }
        let seen = signature(&c.module);
        let serving = Serving::start();
        let mut server = mg_serve::Server::start(&mg_serve::Options {
            server: server.clone(),
            user_dir: dir.clone(),
            module: name,
            port,
            public: c.public,
            name: c.name.clone(),
            player_password: passwords.0.clone(),
            dm_password: passwords.1.clone(),
            admin_password: passwords.2.clone(),
            console: true,
            env: env.clone(),
            args: c.args.iter().map(Into::into).collect(),
            own_group: caught,
        })?;
        port = Some(server.port);
        passwords = (
            Some(server.player_password.clone()),
            Some(server.dm_password.clone()),
            Some(server.admin_password.clone()),
        );
        let started = Instant::now();
        let console = lines(server.console().expect("started with a console"));
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
        let (mut looked, mut failed, mut quit) = (Instant::now(), false, false);
        // The server prints a command's answer when it next reads its
        // console: an empty line a moment after a command brings it out.
        let mut nudge: Option<Instant> = None;
        let end = loop {
            let mut events = Vec::new();
            for line in console.try_iter() {
                if line.starts_with("Server: Module loaded") {
                    events.push(Event::Ready { seconds: started.elapsed().as_secs_f32() });
                }
                failed |= line.contains("Unable to load module");
                events.push(Event::Console(line));
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
                for line in console.try_iter() {
                    failed |= line.contains("Unable to load module");
                    each(&Event::Console(line));
                }
                for line in log.more() {
                    each(&Event::Log(line));
                }
                break Some(match (failed, quit) {
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
        for line in console.try_iter() {
            each(&Event::Console(line));
        }
        for line in engine.more().into_iter().filter(grave) {
            each(&Event::Engine(line));
        }
        for line in log.more() {
            each(&Event::Log(line));
        }
        drop(serving);
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
        let (_, notes) = prepare(&gi, &project, &server).unwrap();
        assert!(notes.iter().any(|n| n.contains("made from")), "{notes:?}");
        let made = std::fs::read(server.join("tlk/mytlk.tlk")).unwrap();
        let tlk = mg_tlk::Tlk::read(&made).unwrap();
        assert_eq!(tlk.entries[0].text, b"Ours");
        assert_eq!(std::fs::read(user.join("tlk/mytlk.tlk")).unwrap(), b"the player's own");
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
}
