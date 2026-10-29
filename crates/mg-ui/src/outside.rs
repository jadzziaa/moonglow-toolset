//! A nasher project's or a module folder's files changed outside Moonglow
//! (an editor, `git checkout`, `git pull`) while it is open: every few seconds, with
//! Options › General's reloading on, those the module has no unsaved
//! change to are read again; where it has one, Moonglow's is kept and the
//! Changed Outside Moonglow window asks which to keep.
//!
//! None of the looking is done on the window's thread: a [`Watch`] sees
//! that something changed (and waits for a burst of changes, a pull's, to
//! end), then a thread of its own reads what changed
//! ([`mg_module::outside`]), and the window takes what it found when it
//! is done. (On the window's thread, a large project's pull held the
//! window for a second and a half: a builder saw it freeze.)

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mg_core::ResType;
use mg_module::locks::Lock;
use mg_module::nasher::Outside;
use mg_module::outside::OutsideFound;
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;

use crate::{Moonglow, Tab};

/// What is known of the files changed outside.
#[derive(Debug, Default)]
pub(crate) struct OutsideState {
    /// Changed outside and in Moonglow both: waiting for an answer.
    pub(crate) conflicts: Vec<Outside>,
    /// What was said already (a problem, a conflict), not to say it again
    /// every few seconds.
    said: std::collections::HashSet<u64>,
    /// The conflicts "Later" was chosen for.
    later: Vec<PathBuf>,
    /// Looks at the module's files off the UI's thread.
    watch: Option<Watch>,
    /// And at the haks and folders the game data reads.
    content: Option<Watch>,
    /// The files being read on a thread of their own, after the watch saw
    /// a change.
    looking: Option<Looking>,
    /// Blueprints read again, of the module whose place this is, whose
    /// custom palettes are still to be looked at
    /// ([`Moonglow::look_at_palettes`]).
    palettes: Option<(PathBuf, Vec<ResKey>)>,
    /// What a watch asks for a frame with, when it hears a change.
    pub(crate) repaint: Option<egui::Context>,
    /// Scripts read again being compiled, a few each frame.
    compiling: Option<Compiling>,
    /// The watches walk the files rather than be told by the system (a
    /// test of the walk).
    pub(crate) walk: bool,
}

/// A look at the module's files under way ([`Moonglow::look_outside`]).
#[derive(Debug)]
struct Looking {
    /// The module's folder or project looked at.
    root: PathBuf,
    started: Instant,
    done: std::sync::mpsc::Receiver<Looked>,
}

/// What a look found.
#[derive(Debug)]
struct Looked {
    found: OutsideFound,
    /// The locks on the objects of each area whose `.git` changed, as the
    /// module had the area when the look began: put back as the area is
    /// read again ([`Moonglow::take_outside`]).
    locks: HashMap<ResKey, (Arc<[u8]>, Vec<Lock>)>,
}

/// What a [`Watch`] heard since it was last asked.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Heard {
    /// Something changed: to look.
    changed: bool,
    /// All of them to be looked at: a walk saw it (the system doesn't
    /// tell), a folder was made, moved or removed, the system lost count.
    all: bool,
    /// Else the files the system said changed, were made or removed.
    paths: std::collections::BTreeSet<PathBuf>,
}

impl Heard {
    /// The files to look at; `None`: all of them.
    fn files(self) -> Option<Vec<PathBuf>> {
        (!self.all).then(|| self.paths.into_iter().collect())
    }

    fn take(&mut self, other: Heard) {
        self.changed |= other.changed;
        self.all |= other.all;
        if self.all {
            self.paths.clear();
        } else {
            self.paths.extend(other.paths);
        }
    }
}

/// A thread that is told by the system (inotify, FSEvents,
/// ReadDirectoryChangesW: the `notify` crate) when a module folder's (or a
/// nasher project's) files change, are added or removed, and which, and
/// says so once a burst of changes (a pull's) has settled, so that the
/// window's thread reads them only then, and only them. Where the system
/// can't be asked (a path it can't watch, no watches left), it walks the
/// files every few seconds, as before: their names, times and sizes.
/// (Looked at on the window's thread, a large module's thousands of files
/// stalled the window for a moment every few seconds: a builder saw it.)
/// Told by the system, it still walks them now and then
/// ([`Watch::CHECK`]), for a change the system didn't tell (a network
/// share's, made on another machine).
#[derive(Debug)]
struct Watch {
    /// The folders (with what is under them, where `deep`) and files
    /// looked at.
    paths: Vec<PathBuf>,
    heard: Arc<std::sync::Mutex<Heard>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    /// Whether the system tells of changes (else they are walked for).
    told: bool,
    /// The system's watch, ended with this one.
    _system: Option<notify::RecommendedWatcher>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The files `paths` name, and those in the folders they name, as one
/// number: their names, times and sizes. With `deep`, the folders under
/// a folder too (not what a version control or a build keeps there:
/// `.git`, `.nasher` and other hidden folders).
fn signature(paths: &[PathBuf], deep: bool) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut dirs = Vec::new();
    for path in paths {
        match std::fs::metadata(path) {
            Ok(meta) if meta.is_dir() => dirs.push(path.clone()),
            Ok(meta) => (path, meta.modified().ok(), meta.len()).hash(&mut h),
            Err(_) => path.hash(&mut h),
        }
    }
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut seen: Vec<(std::ffi::OsString, Option<std::time::SystemTime>, u64)> = Vec::new();
        for e in entries.flatten() {
            let name = e.file_name();
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                if deep && !name.to_string_lossy().starts_with('.') {
                    dirs.push(e.path());
                }
            } else {
                seen.push((name, meta.modified().ok(), meta.len()));
            }
        }
        seen.sort();
        (dir, seen).hash(&mut h);
    }
    h.finish()
}

/// What an event of the system's says to look at: nothing (a file read,
/// or in a hidden folder: `.git`, `.nasher`), its files, or all (a folder
/// made, moved or removed whole; the system's count of changes lost).
fn heard_of(
    event: notify::Result<notify::Event>,
    paths: &[PathBuf],
    real: &[(PathBuf, PathBuf)],
    deep: bool,
) -> Heard {
    use notify::EventKind;
    use notify::event::{AccessKind, AccessMode, CreateKind, RemoveKind};
    let Ok(event) = event else { return Heard { changed: true, all: true, ..Heard::default() } };
    if event.need_rescan() {
        return Heard { changed: true, all: true, ..Heard::default() };
    }
    match event.kind {
        // (Moonglow's own reads are heard too: only a write's end counts.)
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => {}
        EventKind::Access(_) => return Heard::default(),
        EventKind::Create(CreateKind::Folder) | EventKind::Remove(RemoveKind::Folder) => {
            return Heard { changed: true, all: true, ..Heard::default() };
        }
        _ => {}
    }
    let mut heard = Heard::default();
    for path in event.paths.into_iter().map(|p| as_watched(p, real)) {
        // A file named as watched (the builder's own project file), or one
        // under a folder watched, not in a hidden folder there.
        let named = paths.contains(&path);
        let under =
            paths.iter().filter(|p| path.starts_with(p)).max_by_key(|p| p.as_os_str().len());
        let hidden = under.is_some_and(|root| {
            let inside = path.strip_prefix(root).unwrap_or(&path);
            let mut parts: Vec<_> = inside.components().collect();
            parts.pop();
            parts.iter().any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        });
        if !named && (under.is_none() || hidden) {
            continue;
        }
        // (Not deep: what is in the folder's own folders isn't its.)
        if !deep && !named && path.parent().is_some_and(|p| !paths.iter().any(|w| w == p)) {
            continue;
        }
        heard.changed = true;
        // A folder (made, moved there) or what was one (a name with no
        // extension, gone): all of it is looked at.
        let folder = match std::fs::metadata(&path) {
            Ok(meta) => meta.is_dir(),
            Err(_) => path.extension().is_none(),
        };
        if folder {
            heard.all = true;
        } else {
            heard.paths.insert(path);
        }
    }
    if heard.all {
        heard.paths.clear();
    }
    heard
}

/// The paths watched that are another where the system says it (a link
/// followed: macOS says `/private/var/…` of `/var/…`): each, and that.
fn real_paths(paths: &[PathBuf]) -> Vec<(PathBuf, PathBuf)> {
    let real = |p: &Path| {
        std::fs::canonicalize(p).ok().or_else(|| {
            let parent = std::fs::canonicalize(p.parent()?).ok()?;
            Some(parent.join(p.file_name()?))
        })
    };
    (paths.iter()).filter_map(|p| real(p).filter(|r| r != p).map(|r| (p.clone(), r))).collect()
}

/// A path the system said, as under the paths watched.
fn as_watched(path: PathBuf, real: &[(PathBuf, PathBuf)]) -> PathBuf {
    for (watched, r) in real {
        if let Ok(rest) = path.strip_prefix(r) {
            return if rest.as_os_str().is_empty() { watched.clone() } else { watched.join(rest) };
        }
    }
    path
}

impl Watch {
    /// How often a watch the system tells walks the files all the same.
    const CHECK: Duration = Duration::from_secs(30);
    /// How long a burst of changes the system tells of must be quiet
    /// before it is said.
    const QUIET: Duration = Duration::from_millis(300);

    /// A watch of `paths`, told by the system unless `walk` (or the
    /// system can't watch them), walking them every `every` otherwise.
    /// `repaint` is asked for a frame when something changed.
    fn start(
        paths: Vec<PathBuf>,
        deep: bool,
        every: Duration,
        walk: bool,
        repaint: Option<egui::Context>,
    ) -> Watch {
        use std::sync::atomic::{AtomicBool, Ordering};
        let stop = Arc::new(AtomicBool::new(false));
        // (Looked at once at the start, and only now: what changed before
        // this first look is in it, and would never be said.)
        let first = Heard { changed: true, all: true, ..Heard::default() };
        let heard = Arc::new(std::sync::Mutex::new(first));
        let (send, told) = std::sync::mpsc::channel();
        let system = if walk { None } else { system_watch(&paths, deep, send) };
        let is_told = system.is_some();
        let (dirs, said, done) = (paths.clone(), heard.clone(), stop.clone());
        let say = move |h: Heard| {
            said.lock().unwrap_or_else(|e| e.into_inner()).take(h);
            if let Some(ctx) = &repaint {
                ctx.request_repaint();
            }
        };
        let work = move || {
            let real = real_paths(&dirs);
            let mut last = signature(&dirs, deep);
            let mut walked = Instant::now();
            while !done.load(Ordering::Relaxed) {
                if is_told {
                    // Told: what the system says, once it is quiet a moment
                    // (at most ten seconds of a burst).
                    let Ok(event) = told.recv_timeout(Duration::from_millis(100)) else {
                        if walked.elapsed() >= Watch::CHECK {
                            let now = signature(&dirs, deep);
                            walked = Instant::now();
                            if now != last {
                                last = now;
                                say(Heard { changed: true, all: true, ..Heard::default() });
                            }
                        }
                        continue;
                    };
                    let mut heard = heard_of(event, &dirs, &real, deep);
                    let began = Instant::now();
                    while began.elapsed() < Duration::from_secs(10) {
                        match told.recv_timeout(Watch::QUIET) {
                            Ok(event) => heard.take(heard_of(event, &dirs, &real, deep)),
                            Err(_) => break,
                        }
                    }
                    if heard.changed {
                        // (What it walks next is as it is now.)
                        last = signature(&dirs, deep);
                        walked = Instant::now();
                        say(heard);
                    }
                    continue;
                }
                // (In short naps, to end soon after the module closes.)
                let until = Instant::now() + every;
                while Instant::now() < until && !done.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(100));
                }
                let mut now = signature(&dirs, deep);
                if now == last {
                    continue;
                }
                // A burst of changes (a pull, a checkout) is said once it
                // is over: when a look a moment later finds the same, or
                // after a while of changes.
                let settle = (every / 6).min(Duration::from_millis(500));
                for _ in 0..20 {
                    std::thread::sleep(settle);
                    let again = signature(&dirs, deep);
                    if again == now || done.load(Ordering::Relaxed) {
                        break;
                    }
                    now = again;
                }
                last = now;
                say(Heard { changed: true, all: true, ..Heard::default() });
            }
        };
        if std::thread::Builder::new().name("moonglow files".into()).spawn(work).is_err() {
            // (No thread: looked at every time, as before.)
            stop.store(true, Ordering::Relaxed);
        }
        Watch { paths, heard, stop, told: is_told, _system: system }
    }

    /// What it heard since it was last asked, if anything changed.
    fn take(&self) -> Option<Heard> {
        let mut heard = self.heard.lock().unwrap_or_else(|e| e.into_inner());
        heard.changed.then(|| std::mem::take(&mut *heard))
    }

    /// Whether it heard a change, not yet taken.
    fn pending(&self) -> bool {
        self.heard.lock().unwrap_or_else(|e| e.into_inner()).changed
    }

    /// Whether it has no thread (none to be had): looked at each time.
    fn dead(&self) -> bool {
        self.stop.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// The system's watch of `paths` (a folder with what is under it where
/// `deep`; a file, or a file not there yet by its folder), its events
/// sent to `send`; `None` where one of them can't be watched.
fn system_watch(
    paths: &[PathBuf],
    deep: bool,
    send: std::sync::mpsc::Sender<notify::Result<notify::Event>>,
) -> Option<notify::RecommendedWatcher> {
    use notify::{RecursiveMode, Watcher};
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = send.send(event);
    })
    .ok()?;
    let mode = if deep { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
    let mut watched: Vec<(&Path, RecursiveMode)> = Vec::new();
    for path in paths {
        // (A file under a folder watched whole is told of with it.)
        if deep && paths.iter().any(|p| p != path && path.starts_with(p)) {
            continue;
        }
        // A file by its folder: one written whole and moved into place (a
        // module's save, a hak's build) is another file, which a watch of
        // the first doesn't hear of.
        let (what, mode) = match path.is_dir() {
            true => (path.as_path(), mode),
            false => (path.parent()?, RecursiveMode::NonRecursive),
        };
        if !watched.iter().any(|(w, _)| *w == what) {
            watched.push((what, mode));
        }
    }
    for (what, mode) in watched {
        watcher.watch(what, mode).ok()?;
    }
    Some(watcher)
}

/// Whether two resources are the same bytes: at once where they are the
/// same shared bytes (a module's resource and its file's, as read), which
/// compared byte by byte took milliseconds for a pull's areas.
fn same_bytes(a: Option<&[u8]>, b: Option<&[u8]>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::eq(a, b) || a == b,
        (None, None) => true,
        _ => false,
    }
}

fn hash(parts: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

/// File names, a few of them by name.
fn named(changes: &[Outside]) -> String {
    let names: Vec<String> = changes
        .iter()
        .take(6)
        .map(|c| c.path.file_name().unwrap_or_default().to_string_lossy().into_owned())
        .collect();
    match changes.len() - names.len() {
        0 => names.join(", "),
        more => format!("{} and {more} more", names.join(", ")),
    }
}

impl Moonglow {
    /// The module's folder or nasher project, or the file it is kept in
    /// (a `.mod`), if it has one.
    fn module_root(&self) -> Option<PathBuf> {
        self.ws.as_ref().and_then(|ws| {
            let m = &ws.module;
            (m.project.as_ref().map(|p| p.root().to_path_buf()))
                .or_else(|| m.folder.as_ref().map(|f| f.dir().to_path_buf()))
                .or_else(|| m.archive.as_ref().map(|a| a.path().to_path_buf()))
        })
    }

    /// Every few seconds: the module's files are read again, on a thread
    /// of their own ([`Moonglow::look_outside`]), if the watch saw any
    /// change among them. The watch is the module's place's: begun with
    /// it, ended when the module is closed or kept elsewhere.
    pub fn reload_changed_files(&mut self) {
        let Some(root) = self.module_root() else {
            self.outside.watch = None;
            self.outside.looking = None;
            return;
        };
        // A module kept in one file: the file (and its own project files
        // beside it, `krezk.moonglow.toml`, looked at as they are, a few
        // small files).
        let archive = self.ws.as_ref().is_some_and(|ws| ws.module.archive.is_some());
        if archive {
            self.reread_project_file();
        }
        // (And the builder's own project file, in a hidden folder the
        // walk passes over.)
        let paths = match archive {
            true => vec![root.clone()],
            false => vec![root.clone(), mg_module::project_file::ProjectFile::user_path(&root)],
        };
        if self.outside.watch.as_ref().is_none_or(|w| w.paths != paths) {
            let (every, walk, repaint) =
                (Duration::from_secs(3), self.outside.walk, self.outside.repaint.clone());
            self.outside.watch = Some(Watch::start(paths, !archive, every, walk, repaint));
        }
        let watch = self.outside.watch.as_ref().expect("just made");
        // (A watch without its thread says nothing: looked at each time,
        // on the window's thread, as there are no threads to be had.)
        if watch.dead() {
            self.reload_project_files();
            return;
        }
        // (One look at a time: a change seen during one is looked at
        // after it.)
        if self.outside.looking.is_none()
            && let Some(heard) = watch.take()
        {
            self.look_outside_at(heard.files());
        }
    }

    /// Whether a watch heard a change not yet looked at: looked at in
    /// this frame, not when the few seconds' look is due.
    pub(crate) fn heard_outside(&self) -> bool {
        let module = self.outside.looking.is_none()
            && self.outside.watch.as_ref().is_some_and(Watch::pending);
        module || self.outside.content.as_ref().is_some_and(Watch::pending)
    }

    /// Whether the module's files are told of by the system (not walked
    /// for every few seconds).
    pub fn outside_told(&self) -> bool {
        self.outside.watch.as_ref().is_some_and(|w| w.told)
    }

    /// Begins a look at the module's files on a thread of its own: what is
    /// known of them is taken now (their bytes are shared, not copied), and
    /// [`Moonglow::take_looked`] takes what the thread found when it is
    /// done. Without a thread, looked at here and now.
    pub fn look_outside(&mut self) {
        self.look_outside_at(None);
    }

    /// [`Moonglow::look_outside`] at these files alone (those a watch
    /// heard of: their snapshot and their look are theirs alone, not the
    /// module's thousands), or all (`None`).
    fn look_outside_at(&mut self, files: Option<Vec<PathBuf>>) {
        let Some(root) = self.module_root() else { return };
        // The project file (`moonglow.toml`) made, changed or deleted.
        self.reread_project_file();
        let Some(ws) = self.ws.as_ref() else { return };
        let scan = match &files {
            Some(files) => ws.module.outside_scan_of(files),
            None => ws.module.outside_scan(),
        };
        let Some(scan) = scan else { return };
        // (The areas as they are now, for their locks: only the project's
        // files leave them out. Those of the files named, where they are:
        // all of them were 1,400 a look.)
        let areas: HashMap<ResKey, Arc<[u8]>> = if ws.module.project.is_some() {
            let gits: Vec<ResKey> = match &files {
                Some(_) => scan.keys().filter(|k| k.restype == ResType::GIT).collect(),
                None => ws.module.keys_of(ResType::GIT).copied().collect(),
            };
            (gits.into_iter()).filter_map(|k| Some((k, ws.module.shared(&k)?.clone()))).collect()
        } else {
            HashMap::new()
        };
        let (send, done) = std::sync::mpsc::channel();
        let work = move || {
            let found = scan.run();
            let mut locks = HashMap::new();
            for c in found.changes() {
                let Some(bytes) = areas.get(&c.key) else { continue };
                let Ok(git) = mg_gff::Gff::read(bytes) else { continue };
                locks
                    .insert(c.key, (bytes.clone(), mg_module::locks::locks_in(c.key.resref, &git)));
            }
            let _ = send.send(Looked { found, locks });
        };
        match std::thread::Builder::new().name("moonglow outside".into()).spawn(work) {
            Ok(_) => {
                self.outside.looking = Some(Looking { root, started: Instant::now(), done });
            }
            Err(_) => self.reload_project_files(),
        }
    }

    /// Stops looking at the module's files (a look under way is dropped):
    /// a project's command is about to change them. The watch begins
    /// again with the next look.
    pub(crate) fn pause_outside(&mut self) {
        self.outside.watch = None;
        self.outside.looking = None;
    }

    /// Whether the module's files are being read (changed outside).
    pub fn looking_outside(&self) -> bool {
        self.outside.looking.is_some()
    }

    /// What the status bar says of a look under way that takes a while,
    /// or of scripts read again being compiled.
    pub(crate) fn looking_note(&self) -> Option<&'static str> {
        if self.outside.compiling.as_ref().is_some_and(|c| c.queue.len() > 1) {
            return Some("Compiling scripts changed outside Moonglow…");
        }
        let looking = self.outside.looking.as_ref()?;
        (looking.started.elapsed() > Duration::from_millis(400))
            .then_some("Reading files changed outside Moonglow…")
    }

    /// Takes what a look at the module's files found, once it is done
    /// (each frame while one is under way; not under a job, which holds
    /// the game data the module's layer is in): the changes are taken as
    /// [`Moonglow::reload_project_files`] takes them. Returns whether one
    /// is still under way.
    pub fn take_looked(&mut self) -> bool {
        let Some(looking) = self.outside.looking.as_ref() else { return false };
        if self.busy() {
            return true;
        }
        let looked = match looking.done.try_recv() {
            Ok(looked) => looked,
            Err(std::sync::mpsc::TryRecvError::Empty) => return true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.outside.looking = None;
                self.log.error("Could not look at the module's files (the look stopped)");
                return false;
            }
        };
        let looking = self.outside.looking.take().expect("under way");
        // (Another module open since: what was found is another's.)
        if self.module_root().as_ref() != Some(&looking.root) {
            return false;
        }
        let Some(ws) = self.ws.as_mut() else { return false };
        let (changes, problems) = ws.module.outside_found(looked.found);
        crate::trace::changed("outside", || {
            format!("{} changed, looked at in {:.0?}", changes.len(), looking.started.elapsed())
        });
        self.take_found(changes, problems, &looked.locks);
        false
    }

    /// Whether the blueprints read again change their custom palettes (as
    /// shown, and as the next save writes them): a palette changed so is
    /// unsaved work, so that closing asks. One type a frame (a pull's 400
    /// blueprints took 4.6 ms in one, with the rest of the frame over the
    /// budget). With unsaved work already, nothing is looked at; else the
    /// module is as saved but for the blueprints read again, and only they
    /// are looked at (`update_custom_palette_for`). Returns whether any is
    /// still to be looked at.
    pub fn look_at_palettes(&mut self) -> bool {
        let Some((root, keys)) = self.outside.palettes.as_mut() else { return false };
        let (Some(ws), Some(game)) = (self.ws.as_mut(), self.game.as_deref()) else {
            self.outside.palettes = None;
            return false;
        };
        let here = (ws.module.project.as_ref().map(|p| p.root()))
            .or_else(|| ws.module.folder.as_ref().map(|f| f.dir()))
            .or_else(|| ws.module.archive.as_ref().map(|a| a.path()));
        if ws.is_modified() || here != Some(root.as_path()) {
            self.outside.palettes = None;
            return false;
        }
        let Some(kind) = keys.first().and_then(|k| BlueprintKind::from_restype(k.restype)) else {
            self.outside.palettes = None;
            return false;
        };
        let (these, rest): (Vec<ResKey>, Vec<ResKey>) =
            std::mem::take(keys).into_iter().partition(|k| k.restype == kind.restype());
        *keys = rest;
        let update = mg_module::palette::update_custom_palette_for;
        if matches!(update(&ws.module, game, kind, &these), Ok(Some(_))) {
            ws.mark_modified();
            self.outside.palettes = None;
        } else if keys.is_empty() {
            self.outside.palettes = None;
        }
        self.outside.palettes.is_some()
    }

    /// Every few seconds: the haks and folders the game data reads (and the
    /// custom talk table) are looked at again
    /// ([`Moonglow::reload_resources`]) if a watch of theirs saw a change:
    /// a large `override` looked at on the window's thread every time is
    /// what a module's files were.
    pub fn reload_changed_content(&mut self) {
        let Some(game) = self.game.as_ref() else {
            self.outside.content = None;
            return;
        };
        let mut paths: Vec<PathBuf> = (game.resman.layers().iter())
            .filter(|l| crate::user_content(l))
            .filter_map(|l| l.container.watched().map(PathBuf::from))
            .collect();
        paths.extend(self.tlk_stamp.as_ref().map(|(path, _)| path.clone()));
        if self.outside.content.as_ref().is_none_or(|w| w.paths != paths) {
            let (every, walk, repaint) =
                (Duration::from_secs(3), self.outside.walk, self.outside.repaint.clone());
            self.outside.content = Some(Watch::start(paths, false, every, walk, repaint));
        }
        // (Held by something else, a job: looked at once it is free. Not
        // let go of by the 3D views here: only when something changed.)
        if self.game_data_held() {
            crate::trace::changed("content", || "the game data is held: not looked at".into());
            return;
        }
        crate::trace::changed("content", || "the game data is free".into());
        let watch = self.outside.content.as_ref().expect("just made");
        if watch.take().is_some() || watch.dead() {
            self.reload_resources(false);
        }
    }

    /// Reads the project file (`moonglow.toml`) again if it was made,
    /// changed or deleted, and says so in the log.
    pub(crate) fn reread_project_file(&mut self) {
        let Some(ws) = self.ws.as_mut() else { return };
        let had = ws.module.project_file.found;
        if ws.module.reread_project_file() {
            self.log_project_file(true, had);
        }
    }

    /// Looks at the project's or the module folder's files (see the
    /// module's notes) here and now, on the window's thread: what Build ›
    /// Add Left-Out Files to nasher.cfg does after it, and tests. (The
    /// watch's looks are done on a thread of their own:
    /// [`Moonglow::look_outside`].)
    pub fn reload_project_files(&mut self) {
        // The project file (`moonglow.toml`) made, changed or deleted.
        self.reread_project_file();
        let Some(ws) = self.ws.as_mut() else { return };
        let Some((changes, problems)) = ws.module.outside() else { return };
        self.take_found(changes, problems, &HashMap::new());
    }

    /// Takes the files found changed outside: those the module has no
    /// unsaved change to are read into it, the others wait for an answer.
    /// `locks`: the locks of areas, found off the window's thread, as the
    /// module had them then.
    fn take_found(
        &mut self,
        changes: Vec<Outside>,
        problems: Vec<String>,
        locks: &HashMap<ResKey, (Arc<[u8]>, Vec<Lock>)>,
    ) {
        // (A file moved where the project's include doesn't look is said.)
        self.take_project_warnings();
        let Some(ws) = self.ws.as_mut() else { return };
        for p in problems {
            if self.outside.said.insert(hash(&p)) {
                self.log.warn(format!("Changed outside Moonglow, and not read: {p}"));
            }
        }
        if changes.is_empty() {
            self.outside.conflicts.clear();
            return;
        }
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let (mut take, mut same, mut conflicts) = (Vec::new(), Vec::new(), Vec::new());
        for c in changes {
            let ours = ws.module.get(&c.key);
            // (A script being typed isn't in the module yet.)
            let typing = self.scripts.get(&c.key).is_some_and(|b| b.is_dirty());
            if typing {
                conflicts.push(c);
            } else if same_bytes(ours, c.resource.as_deref()) {
                same.push(c);
            } else if same_bytes(ours, ws.module.as_read(&c.key)) {
                take.push(c);
            } else {
                conflicts.push(c);
            }
        }
        for c in &same {
            ws.module.took(c);
        }
        for c in &conflicts {
            let id = hash((&c.path, c.resource.as_deref()));
            if self.outside.said.insert(id) {
                self.log.warn(format!(
                    "{} changed outside Moonglow and has unsaved changes here: Moonglow's is \
                     kept (Changed Outside Moonglow asks which to keep)",
                    c.path.display()
                ));
            }
        }
        self.outside.conflicts = conflicts;
        if !take.is_empty() {
            self.take_outside(&take, locks);
        }
    }

    /// The files changed outside that wait for an answer (which to keep).
    pub fn outside_conflicts(&self) -> Vec<PathBuf> {
        self.outside.conflicts.iter().map(|c| c.path.clone()).collect()
    }

    /// Reads `changes` into the module (their files' versions), and counts
    /// the files as read. `found`: areas' locks found already, as the
    /// module had the areas then.
    fn take_outside(
        &mut self,
        changes: &[Outside],
        found: &HashMap<ResKey, (Arc<[u8]>, Vec<Lock>)>,
    ) {
        let Some(ws) = self.ws.as_mut() else { return };
        // An area's locks, which a project's files don't carry, are put
        // back on its objects as they are read again (only a project's:
        // a folder's files have them).
        let mut locks: Vec<Lock> = Vec::new();
        if ws.module.project.is_some() {
            for c in changes.iter().filter(|c| c.key.restype == ResType::GIT) {
                let now = ws.module.shared(&c.key);
                match found.get(&c.key) {
                    // (As they were found: the area is as it was then.)
                    Some((was, these)) if now.is_some_and(|n| Arc::ptr_eq(n, was)) => {
                        locks.extend(these.iter().cloned())
                    }
                    _ => {
                        if let Some(Ok(g)) = ws.module.gff(&c.key) {
                            locks.extend(mg_module::locks::locks_in(c.key.resref, &g));
                        }
                    }
                }
            }
        }
        let list: Vec<_> = changes.iter().map(|c| (c.key, c.resource.clone())).collect();
        let dropped = match ws.adopt(&list) {
            Ok(d) => d,
            Err(e) => {
                self.log.error(e.to_string());
                return;
            }
        };
        for c in changes {
            ws.module.took(c);
        }
        // A blueprint read again has its custom palette entry made right
        // (`Workspace::adopt`: shown at once, written at the next save, as
        // after Aurora's Refresh Palette); whether that changes the palette
        // is looked at in the frames after ([`Moonglow::look_at_palettes`]).
        let keys = changes.iter().map(|c| c.key);
        let blueprints: Vec<ResKey> =
            keys.filter(|k| BlueprintKind::from_restype(k.restype).is_some()).collect();
        if let Some(root) = self.module_root().filter(|_| !blueprints.is_empty()) {
            match &mut self.outside.palettes {
                Some((was, keys)) if *was == root => keys.extend(blueprints),
                palettes => *palettes = Some((root, blueprints)),
            }
        }
        let Some(ws) = self.ws.as_mut() else { return };
        if !locks.is_empty() {
            ws.module.restore_locks(&locks);
        }
        for c in changes {
            // A script's editor reads its text again.
            self.scripts.remove(&c.key);
        }
        // What is gone has no editor any more.
        let gone: Vec<_> = changes.iter().filter(|c| c.resource.is_none()).map(|c| c.key).collect();
        if !gone.is_empty() {
            crate::tabs::close_tabs(&mut self.dock, |t| match t {
                Tab::Script(k) | Tab::Gff(k) | Tab::Dialog(k) | Tab::Blueprint(k) => {
                    !gone.contains(k)
                }
                Tab::Nui(k) => !gone.contains(k),
                Tab::Area(r) | Tab::AreaProperties(r) => {
                    !gone.iter().any(|k| k.restype == mg_core::ResType::ARE && k.resref == *r)
                }
                _ => true,
            });
        }
        // Names shown from the module are read again.
        self.area_names = Default::default();
        self.area_contents.clear();
        let keys: Vec<ResKey> = changes.iter().map(|c| c.key).collect();
        self.update_module_layer(&keys);
        // (A module.ifo read again may list other haks: a pull's.)
        if keys.contains(&mg_module::Module::info_key()) {
            self.sync_haks();
        }
        self.log.info(format!(
            "Read again from the module's files (changed outside Moonglow): {}{}",
            named(changes),
            if dropped { "; the undo history, which had changes to them, was cleared" } else { "" }
        ));
        self.compile_outside(changes);
    }

    /// Options › Script Editor › Automatically Compile Scripts on Save: a
    /// script another program saved is compiled as one saved here is, so
    /// that its compiled script is not the older one, and so are the
    /// scripts that include it: a few each frame
    /// ([`Moonglow::compile_outside_step`]), however many a checkout
    /// changed, the window taking input meanwhile. (A job, which holds the
    /// window, compiles them only for an external compiler, which is run
    /// once for them all.)
    fn compile_outside(&mut self, changes: &[Outside]) {
        if !self.settings.auto_compile {
            return;
        }
        let scripts: Vec<_> = changes
            .iter()
            .filter(|c| c.resource.is_some() && c.key.restype == mg_core::ResType::NSS)
            .map(|c| c.key)
            .collect();
        if scripts.is_empty() {
            return;
        }
        if self.game.is_none() {
            self.log.info(format!(
                "{} scripts read again were not compiled (no game data)",
                scripts.len()
            ));
            return;
        }
        let Some(root) = self.module_root() else { return };
        let Some(ws) = self.ws.as_mut() else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        // (And the scripts that include them.)
        let mut all = scripts.clone();
        all.extend(mg_module::refs::includers(&ws.module, &scripts));
        if self.settings.external_compiler.is_some() && all.len() > 1 {
            self.log.info(format!(
                "{} scripts (those changed and those that include them) are compiled in the \
                 background",
                all.len()
            ));
            self.compile_job(Some(all));
            return;
        }
        let compiling = match self.outside.compiling.take() {
            Some(c) if c.root == root => c,
            _ => Compiling { root, ..Compiling::default() },
        };
        let mut compiling = compiling;
        for key in all {
            if !compiling.queue.contains(&key) {
                compiling.queue.push_back(key);
            }
        }
        self.outside.compiling = Some(compiling);
        self.compile_outside_step();
    }

    /// Compiles some of the scripts read again from outside that are still
    /// to be compiled: for a few milliseconds of the frame (one at least);
    /// once none is left, puts what they made in the module (as made from
    /// it where the module has no unsaved changes, a folder's compiled
    /// scripts written beside them; else as one step to undo) and says
    /// so. A script changed meanwhile (typed, read again) is left out of
    /// what is put in: it is compiled again when it is saved. Returns
    /// whether any is still to be compiled.
    pub(crate) fn compile_outside_step(&mut self) -> bool {
        let Some(mut c) = self.outside.compiling.take() else { return false };
        if self.module_root().as_ref() != Some(&c.root) || self.game.is_none() {
            return false;
        }
        let started = Instant::now();
        while let Some(key) = c.queue.pop_front() {
            let source = self.ws.as_ref().and_then(|ws| ws.module.shared(&key).cloned());
            crate::script_view::compile_edits(self, &[key], &mut c.made);
            c.sources.push((key, source));
            if started.elapsed() >= Compiling::BUDGET {
                break;
            }
        }
        if !c.queue.is_empty() {
            self.outside.compiling = Some(c);
            return true;
        }
        let Compiling { sources, mut made, .. } = c;
        let Some(ws) = self.ws.as_mut() else { return false };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return false;
        }
        let changed: Vec<ResKey> = (sources.iter())
            .filter(|(k, was)| match (ws.module.shared(k), was) {
                (Some(now), Some(was)) => !Arc::ptr_eq(now, was),
                (None, None) => false,
                _ => true,
            })
            .map(|(k, _)| *k)
            .collect();
        made.drop_scripts(&changed);
        // A module with no unsaved changes has none after: what is
        // compiled is made from the files as they are, and a folder's
        // compiled scripts are written beside them at once (a project
        // keeps none: they are build output).
        let clean = !ws.is_modified();
        let (compiled, _) = crate::script_view::put_compiled(self, made, clean);
        if clean && !compiled.is_empty() {
            let made: Vec<ResKey> = (compiled.iter())
                .filter_map(|n| mg_core::ResRef::from_str(n).ok())
                .flat_map(|r| [ResType::NCS, ResType::NDB].map(|t| ResKey::new(r, t)))
                .collect();
            if let Some(Err(e)) = self.ws.as_mut().map(|ws| ws.module.write_made(&made)) {
                self.log.error(e.to_string());
            }
        }
        if !compiled.is_empty() {
            self.log.info(format!(
                "Compiled (changed outside Moonglow): {}",
                crate::transfer::listed(&compiled)
            ));
        }
        let failed: Vec<String> = (sources.iter())
            .map(|(k, _)| *k)
            .filter(|k| !changed.contains(k) && !compiled.contains(&k.resref.to_string()))
            .filter(|k| self.script_fails(*k))
            .map(|k| k.resref.to_string())
            .collect();
        if !failed.is_empty() {
            self.log.error(format!(
                "Did not compile (Compile in the script's editor says why): {}",
                crate::transfer::listed(&failed)
            ));
        }
        false
    }

    /// Whether scripts read again from outside are being compiled.
    pub fn compiling_outside(&self) -> bool {
        self.outside.compiling.is_some()
    }
}

/// Scripts read again from outside being compiled, a few each frame
/// ([`Moonglow::compile_outside_step`]).
#[derive(Debug, Default)]
pub(crate) struct Compiling {
    /// The module's folder or project they are of.
    root: PathBuf,
    queue: std::collections::VecDeque<ResKey>,
    /// Each compiled, with its source as it was compiled.
    sources: Vec<(ResKey, Option<Arc<[u8]>>)>,
    made: crate::script_view::Compiled,
}

impl Compiling {
    /// How long a frame compiles for.
    const BUDGET: Duration = Duration::from_millis(4);
}

/// Changed Outside Moonglow: the files changed outside that Moonglow has
/// unsaved changes to, and which to keep.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let paths: Vec<PathBuf> = app.outside.conflicts.iter().map(|c| c.path.clone()).collect();
    if paths.is_empty() || paths == app.outside.later {
        return;
    }
    enum Answer {
        Ours,
        Theirs,
        Later,
    }
    let mut answer = None;
    egui::Window::new("Changed Outside Moonglow")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                "These files of the project were changed by another program, and Moonglow has \
                 unsaved changes to them:",
            );
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for c in &app.outside.conflicts {
                    let gone = if c.resource.is_none() { " (deleted)" } else { "" };
                    ui.monospace(format!("{}{gone}", c.path.display()));
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .button("Keep Moonglow's")
                    .on_hover_text("Your changes stay; the next Save writes them over the files")
                    .clicked()
                {
                    answer = Some(Answer::Ours);
                }
                if ui
                    .button("Take the Files'")
                    .on_hover_text(
                        "The files are read again; your unsaved changes to them are lost",
                    )
                    .clicked()
                {
                    answer = Some(Answer::Theirs);
                }
                if ui
                    .button("Later")
                    .on_hover_text("Moonglow's stay for now, and Save won't write over these files")
                    .clicked()
                    || crate::widgets::escape_closes(ui.ctx(), "Changed Outside Moonglow")
                {
                    answer = Some(Answer::Later);
                }
            });
        });
    let conflicts = std::mem::take(&mut app.outside.conflicts);
    match answer {
        Some(Answer::Ours) => {
            if let Some(ws) = app.ws.as_mut() {
                for c in &conflicts {
                    ws.module.took(c);
                }
            }
            // (No longer as saved: the files have something else.)
            if let Some(ws) = app.ws.as_mut() {
                ws.mark_modified();
            }
            app.log.info(format!("Kept Moonglow's: {} (Save writes them)", named(&conflicts)));
        }
        Some(Answer::Theirs) => app.take_outside(&conflicts, &HashMap::new()),
        Some(Answer::Later) => {
            app.outside.later = paths;
            app.outside.conflicts = conflicts;
        }
        None => app.outside.conflicts = conflicts,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn the_files_signature_changes_with_them_not_with_hidden_folders() {
        let dir = mg_testkit::scratch_dir("ui-outside-signature");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("src/a.nss"), "void main() {}").unwrap();
        // The files as one number: other when one is longer, added or
        // removed; the same for what a hidden folder holds.
        let first = signature(std::slice::from_ref(&dir), true);
        assert_eq!(signature(std::slice::from_ref(&dir), true), first);
        std::fs::write(dir.join(".git/index"), "x").unwrap();
        assert_eq!(signature(std::slice::from_ref(&dir), true), first);
        std::fs::write(dir.join("src/a.nss"), "void main() { int a; }").unwrap();
        let longer = signature(std::slice::from_ref(&dir), true);
        assert_ne!(longer, first);
        std::fs::write(dir.join("src/b.nss"), "").unwrap();
        let added = signature(std::slice::from_ref(&dir), true);
        assert_ne!(added, longer);
        std::fs::remove_file(dir.join("src/b.nss")).unwrap();
        assert_eq!(signature(std::slice::from_ref(&dir), true), longer);
    }

    /// What the watch hears, waited for.
    fn heard(watch: &Watch) -> Option<Heard> {
        for _ in 0..150 {
            if let Some(h) = watch.take() {
                return Some(h);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    }

    /// The watch, told by the system: all at the start, then the files
    /// changed, added and removed by name, once a burst is quiet; nothing
    /// for a file only read or one in a hidden folder; all for a folder
    /// made. Walking (where the system can't tell): all, after a change.
    #[test]
    fn the_watch_says_which_files_changed_and_only_then() {
        let dir = mg_testkit::scratch_dir("ui-outside-watch");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("src/a.nss"), "void main() {}").unwrap();
        let user = dir.join(".moonglow/user.toml");
        let paths = vec![dir.clone(), user.clone()];
        let watch = Watch::start(paths.clone(), true, Duration::from_millis(40), false, None);
        assert!(watch.told, "inotify, FSEvents or ReadDirectoryChangesW watches the folder");
        let first = heard(&watch).expect("looked at once, at the start");
        assert!(first.all);
        std::thread::sleep(Duration::from_millis(400));
        assert!(watch.take().is_none(), "nothing changed: nothing said");
        // Read: nothing. In a hidden folder: nothing.
        let _ = std::fs::read(dir.join("src/a.nss")).unwrap();
        std::fs::write(dir.join(".git/index"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(600));
        assert!(watch.take().is_none(), "a read and a hidden folder's file: nothing said");
        // A burst: said once, by name.
        std::fs::write(dir.join("src/a.nss"), "void main() { int a; }").unwrap();
        std::fs::write(dir.join("src/c.nss"), "void main() {}").unwrap();
        std::fs::remove_file(dir.join("src/c.nss")).unwrap();
        std::fs::write(dir.join("src/d.nss"), "void main() {}").unwrap();
        let burst = heard(&watch).expect("a burst of changes is said");
        assert!(!burst.all, "{burst:?}");
        let names: Vec<String> = (burst.paths.iter())
            .map(|p| p.strip_prefix(&dir).unwrap().to_string_lossy().replace('\\', "/"))
            .collect();
        // (The file made and removed within the burst may be folded away
        // by the system: macOS's FSEvents does.)
        assert!(names.contains(&"src/a.nss".to_string()), "{names:?}");
        assert!(names.contains(&"src/d.nss".to_string()), "{names:?}");
        assert!(names.iter().all(|n| ["src/a.nss", "src/c.nss", "src/d.nss"].contains(&n.as_str())));
        std::thread::sleep(Duration::from_millis(400));
        assert!(watch.take().is_none(), "and nothing said after");
        // The builder's own project file, in a hidden folder: by name.
        std::fs::create_dir_all(dir.join(".moonglow")).unwrap();
        std::fs::write(&user, "[builder]\ncode = \"ab\"\n").unwrap();
        let own = heard(&watch).expect("the builder's own file is said");
        assert!(own.all || own.paths.contains(&user), "{own:?}");
        // A folder made with files: all.
        std::fs::create_dir_all(dir.join("src/new")).unwrap();
        std::fs::write(dir.join("src/new/e.nss"), "void main() {}").unwrap();
        let folder = heard(&watch).expect("a folder made is said");
        assert!(folder.all, "{folder:?}");
        drop(watch);

        // Walking: all, after a change, and only then.
        let watch = Watch::start(paths, true, Duration::from_millis(40), true, None);
        assert!(!watch.told);
        assert!(heard(&watch).expect("at the start").all);
        std::thread::sleep(Duration::from_millis(300));
        assert!(watch.take().is_none(), "nothing changed: nothing said");
        std::fs::write(dir.join("src/f.nss"), "void main() {}").unwrap();
        assert!(heard(&watch).expect("a file added is seen").all);
        // (The watch may have seen the file made and then written: twice.)
        std::thread::sleep(Duration::from_millis(300));
        let _ = watch.take();
        std::thread::sleep(Duration::from_millis(300));
        assert!(watch.take().is_none(), "and nothing said after");
    }

    /// What the system says under a folder's real place (macOS says
    /// `/private/var/…` of `/var/…`) is taken as under the folder watched.
    #[test]
    fn a_path_said_under_a_real_place_is_the_watched_one() {
        let real = vec![(PathBuf::from("/var/m"), PathBuf::from("/private/var/m"))];
        let watched = |p: &str| as_watched(PathBuf::from(p), &real);
        assert_eq!(watched("/private/var/m/src/a.nss"), Path::new("/var/m/src/a.nss"));
        assert_eq!(watched("/private/var/m"), Path::new("/var/m"));
        assert_eq!(watched("/elsewhere/a.nss"), Path::new("/elsewhere/a.nss"));
    }

    /// A watch of haks and folders (not deep): a hak written, a file in a
    /// folder; not a file in a folder's folder.
    #[test]
    fn the_content_watch_hears_its_files_and_folders() {
        let dir = mg_testkit::scratch_dir("ui-outside-content");
        std::fs::create_dir_all(dir.join("override/below")).unwrap();
        std::fs::write(dir.join("my.hak"), "ERF").unwrap();
        let paths = vec![dir.join("override"), dir.join("my.hak")];
        let watch = Watch::start(paths, false, Duration::from_millis(40), false, None);
        assert!(watch.told);
        assert!(heard(&watch).is_some());
        std::fs::write(dir.join("override/below/x.2da"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(600));
        assert!(watch.take().is_none(), "a folder's folder isn't the game's");
        std::fs::write(dir.join("my.hak"), "ERF again").unwrap();
        let hak = heard(&watch).expect("the hak written is said");
        assert_eq!(hak.paths.iter().collect::<Vec<_>>(), [&dir.join("my.hak")]);
        // Written whole and moved into place, twice: heard each time.
        for n in 0..2 {
            std::fs::write(dir.join("my.hak.tmp"), format!("ERF {n}")).unwrap();
            std::fs::rename(dir.join("my.hak.tmp"), dir.join("my.hak")).unwrap();
            let moved = heard(&watch).expect("the hak moved into place is said");
            assert!(moved.paths.contains(&dir.join("my.hak")), "{moved:?}");
        }
        std::fs::write(dir.join("override/y.2da"), "y").unwrap();
        assert!(
            heard(&watch).expect("a file in override").paths.contains(&dir.join("override/y.2da"))
        );
    }
}
