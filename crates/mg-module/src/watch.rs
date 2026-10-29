//! A watch of a module's files (a module folder's, a nasher project's)
//! or of the haks and folders the game data reads: told by the system
//! (inotify, FSEvents, ReadDirectoryChangesW: the `notify` crate) which
//! files changed, were added or removed, and said once a burst of changes
//! (a pull's) has settled, so that they are read only then, and only they
//! ([`crate::Module::outside_scan_of`]). The window's look at files
//! changed outside and `mg watch` both follow a module with it.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What a watch calls when it hears a change (the window asks for a
/// frame).
pub type OnChange = Arc<dyn Fn() + Send + Sync>;

/// What a [`Watch`] heard since it was last asked.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Heard {
    /// Something changed: to look.
    pub changed: bool,
    /// All of them to be looked at: a walk saw it (the system doesn't
    /// tell), a folder was made, moved or removed, the system lost count.
    pub all: bool,
    /// Else the files the system said changed, were made or removed.
    pub paths: std::collections::BTreeSet<PathBuf>,
}

impl Heard {
    /// The files to look at; `None`: all of them.
    pub fn files(self) -> Option<Vec<PathBuf>> {
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
/// window's thread (or `mg watch`) reads them only then, and only them. Where the system
/// can't be asked (a path it can't watch, no watches left), it walks the
/// files every few seconds, as before: their names, times and sizes.
/// (Looked at on the window's thread, a large module's thousands of files
/// stalled the window for a moment every few seconds: a builder saw it.)
/// Told by the system, it still walks them now and then
/// ([`Watch::CHECK`]), for a change the system didn't tell (a network
/// share's, made on another machine).
#[derive(Debug)]
pub struct Watch {
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
    /// `on_change` is called when something changed (the window asks for
    /// a frame).
    pub fn start(
        paths: Vec<PathBuf>,
        deep: bool,
        every: Duration,
        walk: bool,
        on_change: Option<OnChange>,
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
            if let Some(on_change) = &on_change {
                on_change();
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
    pub fn take(&self) -> Option<Heard> {
        let mut heard = self.heard.lock().unwrap_or_else(|e| e.into_inner());
        heard.changed.then(|| std::mem::take(&mut *heard))
    }

    /// Whether it heard a change, not yet taken.
    pub fn pending(&self) -> bool {
        self.heard.lock().unwrap_or_else(|e| e.into_inner()).changed
    }

    /// The folders and files it looks at.
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Whether the system tells of changes (else they are walked for).
    pub fn told(&self) -> bool {
        self.told
    }

    /// Whether it has no thread (none to be had): looked at each time.
    pub fn dead(&self) -> bool {
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn the_files_signature_changes_with_them_not_with_hidden_folders() {
        let dir = mg_testkit::scratch_dir("watch-signature");
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
        let dir = mg_testkit::scratch_dir("watch-watch");
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
        assert!(
            names.iter().all(|n| ["src/a.nss", "src/c.nss", "src/d.nss"].contains(&n.as_str()))
        );
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
        let dir = mg_testkit::scratch_dir("watch-content");
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
