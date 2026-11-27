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

fn same(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The server's name for a module: what `-module` takes, and the file in
/// `modules`. (Lower case, the letters a resource name may have.)
pub(crate) fn module_name(module: &Path) -> String {
    let is_nasher = module.file_name().is_some_and(|n| n == "nasher.cfg");
    let dir_or_file = if is_nasher {
        module.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."))
    } else {
        module
    };
    let resolved = if dir_or_file.file_name().is_none() {
        dir_or_file.canonicalize().ok().or_else(|| {
            let cur = std::env::current_dir().ok()?;
            if dir_or_file.as_os_str().is_empty() || dir_or_file == Path::new(".") {
                Some(cur)
            } else {
                cur.join(dir_or_file).canonicalize().ok()
            }
        })
    } else {
        None
    };
    let path = resolved.as_deref().unwrap_or(dir_or_file);
    let stem = if path.is_dir() || is_nasher { path.file_name() } else { path.file_stem() };
    let name: String = stem
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    if name.is_empty() { "module".into() } else { name }
}

/// Whether two paths are one file: the same file reached by two paths,
/// or two hard links of it (which canonical paths can't tell).
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if let (Ok(a), Ok(b)) = (std::fs::metadata(a), std::fs::metadata(b)) {
            return a.dev() == b.dev() && a.ino() == b.ino();
        }
        false
    }
    #[cfg(not(unix))]
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Where a file is placed for the server: whether the folder is the
/// player's own NWN folder (`--real-user-dir`), and whether the file is
/// one of Moonglow's NWSync cache.
#[derive(Debug, Clone, Copy, Default)]
struct Placing {
    players: bool,
    from_cache: bool,
}

/// What stops a file of the player's own folder being replaced.
fn players_file(to: &Path) -> anyhow::Error {
    anyhow::anyhow!(
        "{} is a file of your own NWN folder, which Moonglow doesn't replace: move it away, or          serve from a folder of Moonglow's (leave out --dir and --real-user-dir)",
        to.display()
    )
}

/// Whether there is a file (not a link) at `to`.
fn is_regular_file(to: &Path) -> bool {
    std::fs::symlink_metadata(to).is_ok_and(|m| m.file_type().is_file())
}

/// A file beside `to` that a link or a copy is made as before it is moved
/// into place: `to` is replaced whole, never written through (a link
/// there of an earlier run's, to the player's file or the cache's).
fn part_of(to: &Path) -> PathBuf {
    let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    to.with_file_name(format!(".{name}.{}.mg-part", std::process::id()))
}

/// A way of putting a file at a path (a link, a copy).
type Way<'a> = &'a dyn Fn(&Path, &Path) -> std::io::Result<()>;

/// Puts a file where the server reads it without writing the original.
/// A file of the cache is hard linked (it stays while the server runs,
/// whatever a prune of the cache does), a file elsewhere linked, else
/// either is copied; in the player's own folder, a link only where the
/// system allows one (a hard link would be a file of theirs to the next
/// run), and an existing file there (not a link) is never replaced.
fn place(from: &Path, to: &Path, how: Placing) -> Result<()> {
    if same_file(from, to) {
        return Ok(());
    }
    if how.players && is_regular_file(to) {
        return Err(players_file(to));
    }
    let part = part_of(to);
    let _ = std::fs::remove_file(&part);
    let symlink = |from: &Path, to: &Path| -> std::io::Result<()> {
        #[cfg(unix)]
        return std::os::unix::fs::symlink(from, to);
        #[cfg(not(unix))]
        {
            let _ = (from, to);
            Err(std::io::Error::other("no links"))
        }
    };
    let hard = |from: &Path, to: &Path| std::fs::hard_link(from, to);
    let copy = |from: &Path, to: &Path| std::fs::copy(from, to).map(|_| ());
    let ways: [Way; 3] = match how {
        Placing { players: true, .. } => [&symlink, &copy, &copy],
        Placing { from_cache: true, .. } => [&hard, &symlink, &copy],
        _ => [&symlink, &hard, &copy],
    };
    let mut last = None;
    for way in ways {
        match way(from, &part) {
            Ok(()) => {
                last = None;
                break;
            }
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                last = Some(e);
            }
        }
    }
    if let Some(e) = last {
        return Err(anyhow::Error::new(e).context(to.display().to_string()));
    }
    std::fs::rename(&part, to).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        anyhow::Error::new(e).context(to.display().to_string())
    })
}

/// Writes `bytes` as the file `to` for the server, through a file beside
/// it moved into place (never through a link at `to`); in the player's
/// own folder (`players`), an existing file there is never replaced.
fn place_bytes(bytes: &[u8], to: &Path, players: bool) -> Result<()> {
    if std::fs::read(to).is_ok_and(|had| had == bytes) {
        return Ok(());
    }
    if players && is_regular_file(to) {
        return Err(players_file(to));
    }
    let part = part_of(to);
    std::fs::write(&part, bytes).and_then(|()| std::fs::rename(&part, to)).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        anyhow::Error::new(e).context(to.display().to_string())
    })
}

/// Whether `dir` is the player's own NWN folder (`gi`'s user folder, or
/// where the game keeps it).
pub(crate) fn is_players(gi: &GameInstall, dir: &Path) -> bool {
    players_dir(gi.user_dir.as_deref(), dir).is_some()
}

/// The player's own NWN folder `dir` is, where it is one: `user` (the
/// user folder `mg` was given or found) or where the game keeps it.
pub(crate) fn players_dir(user: Option<&Path>, dir: &Path) -> Option<PathBuf> {
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    (user.map(Path::to_path_buf).into_iter())
        .chain(mg_resman::default_user_dir())
        .find(|p| same(p, &dir))
}

/// What the server's `development` folder is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Development {
    /// A link to the player's own, as Test Module's game has it.
    #[default]
    Players,
    /// The folder `mg watch --serve` writes into: the server's own.
    Watched,
    /// None of the player's (`--no-development`): a link an earlier run
    /// left is taken away.
    Off,
}

impl Development {
    /// The player's, linked, unless `off` (`--no-development`).
    pub(crate) fn players_unless(off: bool) -> Development {
        if off { Development::Off } else { Development::Players }
    }
}

/// Gives the server's folder `dir` the player's `development` folder
/// (in `user`, the user folder `mg` was given or found), as Test Module
/// gives its game: a link to it (a junction on Windows without the right
/// to make a symbolic link), made again at every start so that it
/// follows `--user-dir`. Nothing is written into the player's folder.
/// With none to give (`--no-development`, no user folder), a link an
/// earlier run made is taken away. Nothing is done where the player has
/// none, where `dir` is the
/// player's own (`--real-user-dir`), or for `mg watch`'s folder. A
/// folder (or file) of `dir`'s own there is kept, and said in the note
/// (an empty folder, as the server makes at a start without the link,
/// gives way)
/// returned; in Moonglow's own `scratch` folder a folder there was left
/// by `mg watch --serve`, and gives way.
pub(crate) fn link_development(
    user: Option<&Path>,
    dir: &Path,
    development: Development,
    scratch: bool,
) -> Result<Option<String>> {
    // (None given, `--no-development` or no user folder: a link an earlier
    // run made, into a player's folder, is taken away too.)
    let Some(user) = user.filter(|_| development == Development::Players) else {
        unlink_development(dir)?;
        return Ok(None);
    };
    if players_dir(Some(user), dir).is_some() {
        return Ok(None);
    }
    let to = dir.join("development");
    let from = user.join("development");
    let from = std::path::absolute(&from).unwrap_or(from);
    let context = || to.display().to_string();
    if mg_module::links::is_link(&to) {
        // (The link an earlier run made: made again, to where the player's
        // folder is now.)
        mg_module::links::unlink(&to).with_context(context)?;
    } else if let Ok(meta) = std::fs::symlink_metadata(&to) {
        let empty = meta.is_dir() && std::fs::read_dir(&to).is_ok_and(|mut d| d.next().is_none());
        if empty {
            // (The server's own, made at a start without the link: nothing
            // of anybody's in it.)
            std::fs::remove_dir(&to).with_context(context)?;
        } else if scratch && meta.is_dir() {
            std::fs::remove_dir_all(&to).with_context(context)?;
        } else {
            return Ok(from.is_dir().then(|| {
                format!(
                    "the server's folder has a development folder of its own ({}): kept, so \
                     yours ({}) is not given to the server",
                    to.display(),
                    from.display()
                )
            }));
        }
    }
    if from.is_dir() {
        mg_module::links::link_dir(&from, &to).with_context(context)?;
    }
    Ok(None)
}

/// Takes away a link to the player's `development` folder that `mg
/// serve` left in the server's folder `dir`, before `mg watch --serve`
/// makes the folder it writes into there: never written through.
pub(crate) fn unlink_development(dir: &Path) -> Result<()> {
    let to = dir.join("development");
    if mg_module::links::is_link(&to) {
        mg_module::links::unlink(&to).with_context(|| to.display().to_string())?;
    }
    Ok(())
}

/// Saves a module's copy for a server as an archive at `to`, keeping no
/// backup of the copy it takes the place of. (A `.bak` there that this
/// save did not make is somebody's, and stays.)
pub(crate) fn save_copy(m: &mut mg_module::Module, to: &Path) -> Result<()> {
    let replaced = to.is_file();
    m.save_as(&ModuleLocation::Archive(to.to_path_buf()))
        .with_context(|| to.display().to_string())?;
    if replaced {
        let _ = std::fs::remove_file(to.with_extension("mod.bak"));
    }
    Ok(())
}

/// The user directory made ready: the module as an archive in `modules`,
/// its haks in `hak` and its talk table in `tlk` (from the player's
/// folders and the game's, which are only read). Returns the module's
/// name there and what to say.
///
/// A module whose project names an NWSync server (`[nwsync] source`;
/// not with `no_nwsync`) has the haks that server hands out written from
/// its content into Moonglow's cache
/// ([`mg_module::nwsync_content::write_haks`]: once, and again only for a
/// hak a newer manifest changed) and placed here, over any of the same
/// name on disk, as the game takes them; and its talk table (and its
/// feminine table), where the manifest has it. By Moonglow's rule a hak
/// or talk table on disk newer than the manifest is taken from disk
/// instead, and said ([`mg_module::nwsync_content`]). `progress`: say how
/// far a write is on standard error.
pub(crate) fn prepare(
    gi: &GameInstall,
    module: &Path,
    dir: &Path,
    project: Option<&Path>,
    progress: bool,
    no_nwsync: bool,
) -> Result<(String, Vec<String>)> {
    let mut notes = Vec::new();
    let name = module_name(module);
    // (`--real-user-dir`: none of the player's own files is replaced.)
    let players = is_players(gi, dir);
    for sub in ["modules", "hak", "tlk"] {
        std::fs::create_dir_all(dir.join(sub))
            .with_context(|| dir.join(sub).display().to_string())?;
    }
    let target = dir.join("modules").join(format!("{name}.mod"));
    let mut m = crate::open_module(module).with_context(|| module.display().to_string())?;
    if no_nwsync {
        m.project_file.nwsync_source = None;
    }
    let (haks, tlk) = (m.haks()?, m.custom_tlk()?);
    // (Read before the copy is saved: the copy is a module of its own.)
    let nwsync = m.project_file.nwsync_settings();
    // A talk table the project keeps as JSON, made into the table as
    // nasher packs it (the game's language's, where it keeps one a
    // language): the server finds no other.
    let language = mg_core::Language::from_short_code(&gi.language);
    // (Or the project's an archive was built from, `mg watch`'s.)
    let project_tlk = |table_name: &str| -> Result<_> {
        Ok(match (m.project.as_deref(), project) {
            (Some(p), _) => mg_module::nasher::in_language(p.talk_tables(table_name)?, language),
            (None, Some(root)) if mg_module::nasher::Package::is_package(root) => {
                let package = mg_module::nasher::Package::read(root)?;
                let settings = mg_module::nasher::Settings::read(root);
                let tables = mg_module::nasher::talk_tables(&package, &settings, table_name)?;
                mg_module::nasher::in_language(tables, language)
            }
            _ => None,
        })
    };
    let own_tlk = tlk.as_deref().map(&project_tlk).transpose()?.flatten();
    let own_fem_tlk = match tlk.as_deref().filter(|t| mg_resman::named_file(t, "tlk").is_some()) {
        Some(t) => project_tlk(&mg_module::talk::feminine(t))?,
        None => None,
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
        let (_, wrong) = m.stack_onto(&mut rm, gi);
        if let Some(e) = wrong {
            return Err(e.into());
        }
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
        save_copy(&mut m, &target)?;
    }
    // What is on disk, which the rule weighs against the manifest: the
    // haks as the game finds them, the project's own talk tables (else
    // the `tlk` folders').
    let own_paths = (own_tlk.iter().chain(&own_fem_tlk)).map(|t| t.path.clone()).collect();
    // (After the compile: its scripts compiled now are the module's own.)
    let module_keys: std::collections::HashSet<ResKey> = m.keys().copied().collect();
    let on_disk = mg_module::nwsync_content::OnDisk::of(
        gi,
        &haks,
        tlk.as_deref(),
        own_paths,
        Some(module_keys),
    );
    let synced = match no_nwsync {
        true => None,
        false => nwsync_haks(gi, nwsync, project, &haks, tlk.as_deref(), &on_disk, progress)?,
    };
    if let Some(s) = &synced {
        notes.push(s.summary());
        notes.extend(s.notes());
        if !s.not_listed.is_empty() {
            notes.push(format!(
                "the origins file names haks the module doesn't list, not written: {}",
                s.not_listed.join(", ")
            ));
        }
    }
    let cached = Placing { players, from_cache: true };
    let elsewhere = Placing { players, from_cache: false };
    for hak in &haks {
        // (A name with a folder in it is placed nowhere: `../x` would be
        // a file outside the server's hak folder.)
        let Some(file) = mg_resman::named_file(hak, "hak") else {
            notes.push(format!("warning: {}", gi.hak_not_found(hak)));
            continue;
        };
        let to = dir.join("hak").join(file);
        if let Some(written) = synced.as_ref().and_then(|s| s.hak(hak)) {
            place(&written.path, &to, cached)?;
            continue;
        }
        match gi.find_hak(hak) {
            Some(from) => place(&from, &to, elsewhere)?,
            None if to.is_file() => {}
            None => notes.push(format!("warning: {}", gi.hak_not_found(hak))),
        }
    }
    if let Some(tlk) = tlk {
        let mut place_tlk = |table_name: &str,
                             synced: Option<&Path>,
                             own: Option<mg_module::nasher::TalkTable>,
                             required: bool|
         -> Result<()> {
            let Some(file) = mg_resman::named_file(table_name, "tlk") else {
                if required {
                    notes.push(format!(
                        "warning: talk table {table_name} not found: {}",
                        mg_resman::NO_FILE_NAME
                    ));
                }
                return Ok(());
            };
            let to = dir.join("tlk").join(&file);
            // (The manifest's, as the game data has it, where the one on
            // disk is not newer.)
            if let Some(path) = synced {
                return place(path, &to, cached);
            }
            if let Some(own) = own {
                let bytes = own.tlk.to_bytes_as_nwn_tlk()?;
                place_bytes(&bytes, &to, players)?;
                notes.push(format!("talk table {table_name} made from {}", own.path.display()));
                return Ok(());
            }
            match gi.tlk_dirs().iter().map(|d| d.join(&file)).find(|p| p.is_file()) {
                Some(from) => place(&from, &to, elsewhere)?,
                None if to.is_file() => {}
                None if required => {
                    notes.push(format!("warning: talk table {table_name} not found"));
                }
                None => {}
            }
            Ok(())
        };
        // (The feminine table from where the table comes from: the
        // manifest's where it has one, else none.)
        let from_manifest = synced.as_ref().and_then(|s| s.tlk.as_ref()).is_some();
        let synced_tlk = synced.as_ref().and_then(|s| s.tlk.as_ref()).map(|(_, p)| p.as_path());
        let synced_fem = synced.as_ref().and_then(|s| s.tlk_f.as_ref()).map(|(_, p)| p.as_path());
        place_tlk(&tlk, synced_tlk, own_tlk, true)?;
        if mg_resman::named_file(&tlk, "tlk").is_some() {
            let fem = mg_module::talk::feminine(&tlk);
            match (from_manifest, synced_fem) {
                (true, Some(path)) => place_tlk(&fem, Some(path), None, false)?,
                // (None from where the table comes: one an earlier run put
                // there goes, never a file of the player's.)
                (true, None) => {
                    if let Some(file) = mg_resman::named_file(&fem, "tlk") {
                        let to = dir.join("tlk").join(file);
                        if !(players && is_regular_file(&to)) {
                            let _ = std::fs::remove_file(&to);
                        }
                    }
                }
                (false, _) => place_tlk(&fem, None, own_fem_tlk, false)?,
            }
        }
    }
    Ok((name, notes))
}

/// The module's haks from its NWSync content, written into Moonglow's
/// cache by the rule with what is `on_disk`, where its project's
/// `settings` (or those of the project it was built from) name a server:
/// `None` where none does.
fn nwsync_haks(
    gi: &GameInstall,
    settings: Option<mg_nwsync::Settings>,
    project: Option<&Path>,
    haks: &[String],
    tlk: Option<&str>,
    on_disk: &mg_module::nwsync_content::OnDisk,
    progress: bool,
) -> Result<Option<mg_module::nwsync_content::HakFiles>> {
    let settings = settings.or_else(|| {
        // (A project's folder, or its nasher.cfg.)
        let root = project.map(|p| if p.is_file() { p.parent().unwrap_or(p) } else { p })?;
        mg_module::project_file::ProjectFile::read(root).nwsync_settings()
    });
    let Some(settings) = settings else { return Ok(None) };
    let game = mg_module::nwsync_content::game_storage(gi).map_err(anyhow::Error::msg)?;
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
    let o = mg_module::nwsync_content::WriteOptions { progress: &said, ..Default::default() };
    let files =
        mg_module::nwsync_content::write_haks(&settings, game.as_ref(), haks, tlk, on_disk, &o)
            .map_err(|e| anyhow::anyhow!("the module's NWSync content ({}): {e}", settings.url))?;
    Ok(Some(files))
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

/// What a line the server's console printed says: that the module loaded
/// (`Ready`, `seconds` since `started`), that it could not (`failed`), and
/// the line itself.
fn console_events(line: String, started: Instant, failed: &mut bool) -> Vec<Event> {
    let mut events = Vec::new();
    if line.starts_with("Server: Module loaded") {
        events.push(Event::Ready { seconds: started.elapsed().as_secs_f32() });
    }
    *failed |= line.contains("Unable to load module");
    events.push(Event::Console(line));
    events
}

/// The last of the console's lines, after the loop: reported as the loop
/// does, so a load that came late is still a `Ready`.
fn drain_console(
    lines: impl Iterator<Item = String>,
    started: Instant,
    failed: &mut bool,
    each: &mut dyn FnMut(&Event) -> Flow,
) {
    for line in lines {
        for e in console_events(line, started, failed) {
            each(&e);
        }
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
    let mut passwords = c.passwords.clone();
    loop {
        let (name, notes) =
            prepare(gi, &c.module, &dir, c.project.as_deref(), c.progress, c.no_nwsync)?;
        let development =
            link_development(gi.user_dir.as_deref(), &dir, c.development, scratch.is_some())?;
        for note in notes.into_iter().chain(development) {
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
            player_password: passwords.player_password.clone(),
            dm_password: passwords.dm_password.clone(),
            admin_password: passwords.admin_password.clone(),
            console: true,
            env: env.clone(),
            args: c.args.iter().map(Into::into).collect(),
            own_group: caught,
        })?;
        port = Some(server.port);
        passwords = Passwords {
            player_password: Some(server.player_password.clone()),
            dm_password: Some(server.dm_password.clone()),
            admin_password: Some(server.admin_password.clone()),
        };
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
                events.extend(console_events(line, started, &mut failed));
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
                drain_console(console.try_iter(), started, &mut failed, each);
                for line in engine.more().into_iter().filter(grave) {
                    each(&Event::Engine(line));
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
        drain_console(console.try_iter(), started, &mut failed, each);
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

    /// The server's last lines, read after the loop has ended (at the
    /// timeout, or at its exit), still tell that the module loaded.
    #[test]
    fn a_load_said_after_the_loop_is_still_ready() {
        let lines = ["Server: Loading module", "Server: Module loaded", "Server: Exiting..."];
        let mut seen = Vec::new();
        let mut failed = false;
        drain_console(lines.iter().map(|l| l.to_string()), Instant::now(), &mut failed, &mut |e| {
            seen.push(json(e).unwrap()["event"].as_str().unwrap().to_string());
            Flow::Continue
        });
        assert_eq!(seen, ["console", "ready", "console", "console"]);
        assert!(!failed);
        let mut e = console_events("Unable to load module x".into(), Instant::now(), &mut failed);
        assert!(failed && matches!(e.pop(), Some(Event::Console(_))) && e.is_empty());
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
        assert!(is_players(&gi, &user));
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

    /// `--no-development`: no link, and one an earlier run made is taken
    /// away; nothing in the player's folder is touched.
    #[test]
    fn no_development_takes_the_link_away() {
        let dir = mg_testkit::scratch_dir("mg-serve-development-off");
        let user = players_folder(&dir, "user", "// mine\n");
        let server = dir.join("server");
        std::fs::create_dir_all(&server).unwrap();
        let before = entries(&user);
        link_development(Some(&user), &server, Development::Players, false).unwrap();
        assert!(mg_module::links::is_link(&server.join("development")));
        let note = link_development(Some(&user), &server, Development::Off, false).unwrap();
        assert_eq!(note, None);
        assert!(std::fs::symlink_metadata(server.join("development")).is_err());
        assert_eq!(entries(&user), before);
        // No user folder (`--no-user-dir`): the same.
        link_development(Some(&user), &server, Development::Players, false).unwrap();
        assert_eq!(link_development(None, &server, Development::Players, false).unwrap(), None);
        assert!(std::fs::symlink_metadata(server.join("development")).is_err());
        // A folder of the server's own there is kept.
        std::fs::create_dir_all(server.join("development")).unwrap();
        link_development(Some(&user), &server, Development::Off, false).unwrap();
        assert!(server.join("development").is_dir());
        assert_eq!(Development::players_unless(true), Development::Off);
        assert_eq!(Development::players_unless(false), Development::Players);
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

        // An empty one, as the server makes at a start without the link,
        // gives way outside the scratch folder too, without a note.
        let other = dir.join("other");
        std::fs::create_dir_all(other.join("development")).unwrap();
        let note = link_development(Some(&user), &other, Development::Players, false).unwrap();
        assert_eq!(note, None);
        assert!(mg_module::links::is_link(&other.join("development")));
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
}
