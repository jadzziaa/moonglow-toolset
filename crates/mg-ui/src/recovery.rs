//! Crash safety: while the open module has unsaved changes, a recovery copy
//! of it (edits and unsaved script text included) is written every few
//! minutes (Options › General; 5 by default) to Moonglow's own data folder,
//! never next to the module or into the game's folders. Saving the module,
//! or closing it without saving, removes the copy; a copy still there at
//! the next start (after a crash) is offered back by Recover Unsaved Work.
//!
//! A copy is the module as an archive (`<name>-<id>.mod`) and a note
//! (`.json`): where the module lives, its name, when the copy was made.
//! Each is written whole or not at all, the note first; an archive whose
//! note is not there or does not read is offered all the same, named from
//! its file, as a module never saved.
//!
//! The window that writes a copy holds a lock on a file beside it
//! (`<name>-<id>.lock`) from its first writing until the copy is forgotten
//! or the program ends, however it ends. A copy so held is a running
//! Moonglow's, not a crashed session's: another window does not offer it,
//! and one with the same module open keeps a copy of its own
//! (`<name>-<id>-2`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use mg_core::time::unix_now as now;
use mg_module::{Module, ModuleLocation};
use serde::{Deserialize, Serialize};

use crate::Moonglow;

/// Moonglow's data folder ([`mg_module::build::data_dir`]).
pub fn data_dir() -> Option<PathBuf> {
    mg_module::build::data_dir()
}

/// Moonglow's cache folder ([`mg_module::build::cache_dir`]).
pub fn cache_dir() -> Option<PathBuf> {
    mg_module::build::cache_dir()
}

/// Where the game's shaders, compiled, are kept
/// ([`mg_module::build::shader_cache_dir`]).
pub fn shader_cache_dir() -> Option<PathBuf> {
    mg_module::build::shader_cache_dir()
}

/// A recovery copy's note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// Where the module lives (`None`: never saved).
    pub module: Option<PathBuf>,
    /// Whether it lives in a folder (else an archive).
    pub folder: bool,
    /// The target, when the module lives in a nasher project (`module` is
    /// the project's folder).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_target: Option<String>,
    pub name: String,
    /// When the copy was made (seconds since 1970).
    pub time: u64,
}

/// A recovery copy found in the folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Copy {
    pub archive: PathBuf,
    pub note: Note,
}

impl Copy {
    fn note_path(&self) -> PathBuf {
        self.archive.with_extension("json")
    }

    /// Removes the copy, unless a running Moonglow holds it (its window's
    /// own, which it removes itself): whether it was removed.
    pub fn remove(&self) -> bool {
        match hold(&self.archive) {
            Hold::Others => false,
            Hold::Ours(lock) => {
                self.remove_files();
                lock.forgotten.store(true, Ordering::Release);
                true
            }
            Hold::Unlocked => {
                self.remove_files();
                true
            }
        }
    }

    /// Removes the archive and its note (by the window that holds them).
    fn remove_files(&self) {
        let _ = std::fs::remove_file(&self.archive);
        let _ = std::fs::remove_file(self.note_path());
    }
}

/// A recovery copy's lock: its file (`<name>-<id>.lock` beside the
/// archive), locked for as long as this is kept. The system lets go of it
/// when the program ends, a crash included: a copy left then is no
/// longer held.
#[derive(Debug)]
struct Lock {
    path: PathBuf,
    /// (Kept open: the lock is the open file's.)
    _file: std::fs::File,
    /// The copy was removed: the lock's file goes too, when the last use
    /// of the lock does (while it is still held, so that no other window
    /// takes a file about to go).
    forgotten: AtomicBool,
}

impl Drop for Lock {
    fn drop(&mut self) {
        if self.forgotten.load(Ordering::Acquire) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// What came of taking a copy's lock.
enum Hold {
    /// Held by this call's window from now on.
    Ours(Lock),
    /// A running Moonglow holds it (another window, or this one).
    Others,
    /// The folder takes no lock (read-only, a file system without them):
    /// the copy is used unlocked, as every copy was.
    Unlocked,
}

/// Takes the lock of the copy at `archive`, if no running Moonglow has it.
fn hold(archive: &Path) -> Hold {
    let path = archive.with_extension("lock");
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // (Written to, not appended to: Windows locks only a file opened for
    // writing.)
    let opened = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(&path);
    let Ok(file) = opened else { return Hold::Unlocked };
    match file.try_lock() {
        // (The file its holder removed as it let go, opened before it
        // went: the lock is of a file no longer there, which holds
        // nothing.)
        Ok(()) if !same_file(&file, &path) => Hold::Others,
        Ok(()) => Hold::Ours(Lock { path, _file: file, forgotten: AtomicBool::new(false) }),
        Err(std::fs::TryLockError::WouldBlock) => Hold::Others,
        Err(std::fs::TryLockError::Error(_)) => Hold::Unlocked,
    }
}

/// Whether the open `file` is still the one at `path` (not one removed
/// since it was opened, another made in its place).
fn same_file(file: &std::fs::File, path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (file.metadata(), std::fs::metadata(path)) {
            (Ok(a), Ok(b)) => (a.dev(), a.ino()) == (b.dev(), b.ino()),
            _ => false,
        }
    }
    // (Elsewhere a file removed while open keeps its name from any new
    // file until it is closed: the lock would not have been made.)
    #[cfg(not(unix))]
    {
        let _ = file;
        path.exists()
    }
}

/// Whether a running Moonglow holds the copy at `archive` (its lock is
/// taken): one without a lock file is nobody's.
fn held(archive: &Path) -> bool {
    let path = archive.with_extension("lock");
    let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) else { return false };
    matches!(file.try_lock(), Err(std::fs::TryLockError::WouldBlock))
}

/// The copy the window holds for the open module: the module's own place
/// among the copies (`base`, [`Moonglow::recovery_archive`]), the archive
/// held for it (`base`, or `-2`… beside another window's) and its lock
/// (none where the folder takes none).
#[derive(Debug)]
struct Held {
    base: PathBuf,
    archive: PathBuf,
    lock: Option<Arc<Lock>>,
}

/// The `n`th archive for a module whose copies are at `base`: `base`
/// itself, then `<stem>-2.mod`, `<stem>-3.mod`…
fn nth_archive(base: &Path, n: usize) -> PathBuf {
    if n <= 1 {
        return base.to_path_buf();
    }
    let stem = base.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    base.with_file_name(format!("{stem}-{n}.mod"))
}

/// How long ago, in words.
pub fn age(time: u64) -> String {
    let s = now().saturating_sub(time);
    match s {
        0..60 => "less than a minute ago".into(),
        60..7200 => format!("{} minutes ago", s / 60),
        7200..172_800 => format!("{} hours ago", s / 3600),
        _ => format!("{} days ago", s / 86_400),
    }
}

/// A copy's file name for a module: its name and a hash of where it lives,
/// so that two modules of the same name keep separate copies.
fn file_stem(module: Option<&Path>, name: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let place = module.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    for b in place.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
    }
    let clean: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(40)
        .collect();
    format!("{clean}-{:08x}", h as u32)
}

/// The note of an archive that has none that reads (the note was written
/// in place after the archive: power lost between the two, or in the
/// middle of the second, left unsaved work that was never offered, and
/// never removed): the module's name from the file's (without the hash of
/// its place), where it lives not known (recovered, it is saved where the
/// builder says), made when the file was written.
fn note_from_file(archive: &Path) -> Note {
    let stem = archive.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    // (`<name>-<id>`, or a second window's `<name>-<id>-2`.)
    let id = |s: &str| s.len() == 8 && s.bytes().all(|b| b.is_ascii_hexdigit());
    let nth = |s: &str| !s.is_empty() && s.len() < 3 && s.bytes().all(|b| b.is_ascii_digit());
    let whole = match stem.rsplit_once('-') {
        Some((before, n)) if nth(n) && before.rsplit_once('-').is_some_and(|(_, h)| id(h)) => {
            before
        }
        _ => stem.as_str(),
    };
    let name = match whole.rsplit_once('-') {
        Some((name, hash)) if !name.is_empty() && id(hash) => name.to_string(),
        _ => stem.clone(),
    };
    let time = (std::fs::metadata(archive).and_then(|m| m.modified()).ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    Note { module: None, folder: false, project_target: None, name, time }
}

/// The copies in a folder that no running Moonglow holds, newest first:
/// those a session that did not end left. (One held is another window's
/// work, still open there: offered as a crashed session's, it was opened
/// a second time, and saved over what the first window went on to do.)
pub fn find(dir: &Path) -> Vec<Copy> {
    find_all(dir).into_iter().filter(|c| !held(&c.archive)).collect()
}

/// Every copy in a folder, held or not, newest first: each archive, with
/// its note (one without a note that reads, named from its file:
/// [`note_from_file`]).
pub fn find_all(dir: &Path) -> Vec<Copy> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Copy> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "mod") && p.is_file())
        .map(|archive| {
            let note = std::fs::read(archive.with_extension("json"))
                .ok()
                .and_then(|text| serde_json::from_slice::<Note>(&text).ok())
                .unwrap_or_else(|| note_from_file(&archive));
            Copy { archive, note }
        })
        .collect();
    out.sort_by_key(|c| std::cmp::Reverse(c.note.time));
    out
}

/// A copy to write: the module and where it goes.
struct Job {
    module: Module,
    archive: PathBuf,
    note: Note,
    /// The copy's lock, held until it is written (a copy forgotten
    /// meanwhile stays the window's until its files are gone).
    lock: Option<Arc<Lock>>,
}

impl Job {
    /// Writes the note, then the archive, each whole or not at all
    /// ([`mg_module::write_replacing`]). The note first: an archive is
    /// never there without the note of where its module lives (a note
    /// alone is no copy, and goes with the next one or when the copy is
    /// forgotten).
    fn run(self) -> Result<PathBuf, String> {
        let bytes = self.module.to_archive_bytes().map_err(|e| e.to_string())?;
        let note = serde_json::to_vec_pretty(&self.note).map_err(|e| e.to_string())?;
        let path = self.archive.with_extension("json");
        mg_module::write_replacing(&path, &note).map_err(|e| format!("{}: {e}", path.display()))?;
        mg_module::write_replacing(&self.archive, &bytes)
            .map_err(|e| format!("{}: {e}", self.archive.display()))?;
        Ok(self.archive)
    }
}

/// The autosave's state: when the last copy was made, of which revision,
/// and whether one is being written.
#[derive(Debug)]
pub(crate) struct Autosave {
    last: Option<(std::time::Instant, u64)>,
    busy: Arc<AtomicBool>,
    /// Counted up as the copy is forgotten (saved, closed): a copy being
    /// written meanwhile removes itself when done, or the next start
    /// offers work that was saved.
    forgotten: Arc<std::sync::atomic::AtomicU64>,
    /// The panics elsewhere ([`note_panic`]) a copy was made after.
    panics_seen: u64,
    /// A copy was written as a frame panicked: the unwinding needs none
    /// again ([`Moonglow::keep_while_unwinding`]).
    kept_at_panic: bool,
    /// The copy held for the open module, from its first writing.
    held: Option<Held>,
}

impl Default for Autosave {
    fn default() -> Autosave {
        Autosave {
            last: None,
            busy: Arc::default(),
            forgotten: Arc::default(),
            panics_seen: PANICS.load(Ordering::Acquire),
            kept_at_panic: false,
            held: None,
        }
    }
}

/// The panics in the program, wherever they were (`note_panic`).
static PANICS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Notes a panic anywhere in the program: the panic hook calls it (the
/// `moonglow` binary's). One in a frame writes the recovery copy as the
/// frame stops ([`Moonglow::recovering`]); one in a thread of its own (a
/// job, an autosave, a plugin's run, an area's load) leaves the window
/// going, and the next frame writes the copy then, in the background: the
/// work is kept before whatever that thread left half done shows.
pub fn note_panic() {
    PANICS.fetch_add(1, Ordering::AcqRel);
}

/// Sets the autosave's "being written" back when dropped, however the
/// writing ended (a panic in it included: else no copy was ever made
/// again, and a crash waited for it).
struct Done(Arc<AtomicBool>);

impl Drop for Done {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Where a copy's module lives, by its note.
fn location_of(note: &Note, place: &Path) -> ModuleLocation {
    if note.folder {
        ModuleLocation::Folder(place.to_path_buf())
    } else {
        ModuleLocation::Archive(place.to_path_buf())
    }
}

impl Moonglow {
    /// Where the open module's copy is written, held by this window from
    /// here on ([`Held`]): the module's own place among the copies, or
    /// beside it (`-2`, `-3`…) where another window with the same module
    /// open holds that.
    fn held_archive(&mut self) -> Option<(PathBuf, Note, Option<Arc<Lock>>)> {
        let (base, note) = self.recovery_archive()?;
        if let Some(h) = self.autosave.held.as_ref().filter(|h| h.base == base) {
            return Some((h.archive.clone(), note, h.lock.clone()));
        }
        let mut held = Held { base: base.clone(), archive: base.clone(), lock: None };
        for n in 1..100 {
            let archive = nth_archive(&base, n);
            match hold(&archive) {
                Hold::Others => continue,
                Hold::Ours(lock) => held = Held { base, archive, lock: Some(Arc::new(lock)) },
                Hold::Unlocked => {}
            }
            break;
        }
        let made = (held.archive.clone(), note, held.lock.clone());
        self.autosave.held = Some(held);
        Some(made)
    }

    /// The open module's own place among the copies (with no folder: no
    /// copies). The copy this window writes is there, or beside it
    /// ([`Moonglow::held_archive`]).
    pub(crate) fn recovery_archive(&self) -> Option<(PathBuf, Note)> {
        let dir = self.recovery_dir.as_ref()?;
        let ws = self.ws.as_ref()?;
        let module = ws.module.location.as_ref().map(|l| l.path().to_path_buf());
        let folder = matches!(ws.module.location, Some(ModuleLocation::Folder(_)));
        let project_target = match &ws.module.location {
            Some(ModuleLocation::Project { target, .. }) => Some(target.clone()),
            _ => None,
        };
        let name = module
            .as_deref()
            .and_then(|p| p.file_stem())
            .map_or_else(|| "untitled".to_string(), |s| s.to_string_lossy().into_owned());
        let stem = file_stem(module.as_deref(), &name);
        Some((
            dir.join(format!("{stem}.mod")),
            Note { module, folder, project_target, name, time: now() },
        ))
    }

    /// The copy of the module as it is now: edits, and script editors'
    /// unsaved text.
    fn recovery_job(&mut self) -> Option<Job> {
        let (archive, note, lock) = self.held_archive()?;
        let mut module = self.ws.as_mut()?.snapshot().ok()?;
        for (key, buf) in self.scripts.iter().filter(|(_, b)| b.is_dirty()) {
            module.set(*key, buf.bytes());
        }
        Some(Job { module, archive, note, lock })
    }

    /// Writes the open module's recovery copy now; its archive.
    pub fn write_recovery(&mut self) -> Result<PathBuf, String> {
        self.recovery_job().ok_or_else(|| "no module or no recovery folder".to_string())?.run()
    }

    /// Runs `frame` (a frame of the window); if it panics, writes the
    /// recovery copy of the unsaved work first (unless Options › General
    /// keeps none), then lets the panic go on. The copy is the work as it
    /// was when the frame stopped: a crash costs none of it, where the copy
    /// made every few minutes could be minutes old.
    ///
    /// A panic elsewhere since the frame before (a thread's, noted by
    /// [`note_panic`]) has the copy written after the frame, in the
    /// background.
    pub fn recovering(&mut self, frame: impl FnOnce(&mut Moonglow)) {
        use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
        let Err(panic) = catch_unwind(AssertUnwindSafe(|| frame(self))) else {
            self.keep_after_panic_elsewhere();
            return;
        };
        self.keep_now();
        self.autosave.kept_at_panic = true;
        resume_unwind(panic)
    }

    /// Writes the recovery copy while the program unwinds from a panic
    /// outside a frame (in eframe's own code, which [`recovering`]
    /// doesn't see): the `moonglow` binary's application calls it as it is
    /// dropped. Nothing when no panic is under way, or the frame's panic
    /// wrote the copy already.
    ///
    /// [`recovering`]: Moonglow::recovering
    pub fn keep_while_unwinding(&mut self) {
        if std::thread::panicking() && !self.autosave.kept_at_panic {
            self.keep_now();
            self.autosave.kept_at_panic = true;
        }
    }

    /// The recovery copy written now, on this thread (a panic is under
    /// way), unless Options › General keeps none or nothing is unsaved.
    fn keep_now(&mut self) {
        use std::panic::{AssertUnwindSafe, catch_unwind};
        if self.settings.no_autosave || !self.has_unsaved_work() {
            return;
        }
        // (After a copy being written in the background, if one is: the
        // two would write one temporary file. A few seconds at most.)
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while self.autosave.busy.load(Ordering::Acquire) && std::time::Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        // (A copy that fails, or panics too, leaves the crash as it was.)
        match catch_unwind(AssertUnwindSafe(|| self.write_recovery())) {
            Ok(Ok(path)) => {
                crate::trace::note(format!("recovery copy written: {}", path.display()));
                eprintln!("The unsaved work was kept in {}", path.display());
            }
            Ok(Err(e)) => eprintln!("recovery copy: {e}"),
            Err(_) => eprintln!("recovery copy: failed"),
        }
    }

    /// After a frame: a panic noted elsewhere since the last look has the
    /// recovery copy written now, in the background (the next frame
    /// again, while one is being written).
    fn keep_after_panic_elsewhere(&mut self) {
        let panics = PANICS.load(Ordering::Acquire);
        if panics == self.autosave.panics_seen || self.autosave.busy.load(Ordering::Acquire) {
            return;
        }
        self.autosave.panics_seen = panics;
        if self.settings.no_autosave || self.recovery_dir.is_none() || !self.has_unsaved_work() {
            return;
        }
        let Some(job) = self.recovery_job() else { return };
        crate::trace::note("a panic elsewhere: the recovery copy written");
        self.log.warn(
            "Something failed in the background (Moonglow goes on): the unsaved work is kept in \
             a recovery copy, offered at the next start if Moonglow stops before you save",
        );
        self.write_in_background(job);
    }

    /// Removes the open module's recovery copy (saved, or closed without
    /// saving).
    pub(crate) fn forget_recovery(&mut self) {
        self.autosave.forgotten.fetch_add(1, Ordering::AcqRel);
        // The copy this window wrote, and its lock.
        let own = self.autosave.held.take();
        if let Some(held) = &own {
            let _ = std::fs::remove_file(&held.archive);
            let _ = std::fs::remove_file(held.archive.with_extension("json"));
            match &held.lock {
                Some(lock) => lock.forgotten.store(true, Ordering::Release),
                None => {
                    let _ = std::fs::remove_file(held.archive.with_extension("lock"));
                }
            }
        }
        drop(own);
        // One a session that did not end left for this module (not
        // recovered, and the module since saved or closed): not another
        // window's.
        if let Some((archive, note)) = self.recovery_archive() {
            Copy { archive, note }.remove();
        }
        self.autosave.last = None;
    }

    /// Every few minutes, while there are unsaved changes, a copy in the
    /// background (Options › General).
    pub(crate) fn autosave_timer(&mut self, ui: &egui::Ui) {
        if let Some(next) = self.autosave_tick(std::time::Instant::now()) {
            ui.ctx().request_repaint_after(next);
        }
    }

    /// A mark of the unsaved work as it is: the module's revision and the
    /// text typed in script editors and not saved. Another mark, other
    /// work (never `u64::MAX`).
    pub(crate) fn unsaved_mark(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.ws.as_ref().map(mg_edit::Workspace::revision).hash(&mut h);
        let mut typed: Vec<_> = (self.scripts.iter())
            .filter(|(_, b)| b.is_dirty())
            .map(|(k, b)| (*k, crate::script_tools::hash(&b.text)))
            .collect();
        typed.sort();
        typed.hash(&mut h);
        h.finish() & (u64::MAX >> 1)
    }

    /// The autosave at `now`: a copy started in the background when one is
    /// due; when to look again.
    fn autosave_tick(&mut self, now: std::time::Instant) -> Option<Duration> {
        if self.settings.no_autosave || self.recovery_dir.is_none() {
            return None;
        }
        let ws = self.ws.as_ref()?;
        // (Text typed in a script's editor is unsaved work too: it is in
        // no command until the script is saved.)
        if !ws.is_modified() && !self.scripts.values().any(|b| b.is_dirty()) {
            return None;
        }
        let every = Duration::from_secs(60 * u64::from(self.settings.autosave_every().max(1)));
        // The first change starts the clock.
        let (since, saved) = *self.autosave.last.get_or_insert((now, u64::MAX));
        if now.saturating_duration_since(since) < every
            || self.autosave.busy.load(Ordering::Acquire)
        {
            return Some(every);
        }
        // (What a copy would hold, told apart. Looked at only when a copy
        // is due.)
        let state = self.unsaved_mark();
        if saved == state {
            return Some(every);
        }
        let job = self.recovery_job()?;
        self.autosave.last = Some((now, state));
        self.write_in_background(job);
        Some(every)
    }

    /// Writes a copy on a thread of its own, the autosave busy until it is
    /// done.
    fn write_in_background(&mut self, job: Job) {
        let busy = self.autosave.busy.clone();
        busy.store(true, Ordering::Release);
        let (forgotten, started) =
            (self.autosave.forgotten.clone(), self.autosave.forgotten.load(Ordering::Acquire));
        std::thread::spawn(move || {
            let _done = Done(busy);
            let copy = Copy { archive: job.archive.clone(), note: job.note.clone() };
            // (Its lock is held until the files are gone.)
            let lock = job.lock.clone();
            if let Err(e) = job.run() {
                eprintln!("recovery copy: {e}");
            }
            // (Written whole before this look: a forgetting after it
            // removes the files itself.)
            if forgotten.load(Ordering::Acquire) != started {
                copy.remove_files();
            }
            drop(lock);
        });
    }

    /// Looks for copies left by a session that did not end (shown by
    /// Recover Unsaved Work).
    pub fn find_recoveries(&mut self) {
        self.recoveries = self.recovery_dir.as_deref().map(find).unwrap_or_default();
    }

    /// Opens a recovery copy as the module it came from: saving writes it
    /// there.
    pub fn recover(&mut self, copy: &Copy) {
        // The copy is this window's from here on: written again as the
        // work goes on, removed when it is saved or closed. (One this
        // window holds already is its own, listed before it was.)
        let own = self.autosave.held.take_if(|h| h.archive == copy.archive);
        let lock = match own {
            Some(held) => held.lock,
            None => match hold(&copy.archive) {
                Hold::Ours(lock) => Some(Arc::new(lock)),
                Hold::Unlocked => None,
                Hold::Others => {
                    self.log.warn(format!(
                        "The unsaved work on {} is open in another Moonglow window: not opened \
                         here",
                        copy.note.name
                    ));
                    return;
                }
            },
        };
        let mut m = match Module::open(&copy.archive) {
            Ok(m) => m,
            Err(e) => {
                self.log.error(format!("Could not open the recovery copy: {e}"));
                return;
            }
        };
        match (&copy.note.module, &copy.note.project_target) {
            // The project as it is on disk now, with the recovered resources:
            // saving writes what differs.
            (Some(root), Some(target)) => match Module::open_project(root, Some(target)) {
                Ok(mut project) => {
                    project.adopt_resources(m);
                    m = project;
                }
                Err(e) => {
                    self.log.error(format!(
                        "Could not reopen the project {}: {e}; the recovered module is unsaved",
                        root.display()
                    ));
                    m.location = None;
                }
            },
            // The module where it lives, with the recovered resources: it
            // knows its own files (a folder's, which a save compares with
            // and deletes from; an archive's, which the look outside
            // watches). The copy's archive took their place: a folder then
            // refused the save for the files the copy no longer has, and
            // the copy itself, rewritten by the autosave, was watched as
            // the module's file.
            (Some(place), None) if place.exists() => match Module::open(place) {
                Ok(mut home) => {
                    home.adopt_resources(m);
                    m = home;
                }
                // (No longer a module that reads: saved over, from the copy.)
                Err(_) => m.location = Some(location_of(&copy.note, place)),
            },
            _ => m.location = copy.note.module.as_deref().map(|p| location_of(&copy.note, p)),
        }
        self.use_module(m);
        if let Some(ws) = &mut self.ws {
            ws.mark_modified();
        }
        if let Some((base, _)) = self.recovery_archive() {
            self.autosave.held = Some(Held { base, archive: copy.archive.clone(), lock });
        }
        // With the tabs it had, as opened from its file.
        self.read_open_tabs();
        self.log.info(format!(
            "Recovered the unsaved work on {} from {}: save to keep it",
            copy.note.name,
            age(copy.note.time)
        ));
    }
}

/// Recover Unsaved Work: the copies a session that did not end left.
pub(crate) fn window(app: &mut Moonglow, ui: &egui::Ui) {
    if app.recoveries.is_empty() {
        return;
    }
    let mut open = true;
    let (mut recover, mut discard, mut later) = (None, None, false);
    crate::widgets::dialog(ui.ctx(), "Recover Unsaved Work", &mut open).collapsible(false).show(
        ui.ctx(),
        |ui| {
            ui.label("Moonglow closed without saving these modules. Their unsaved work was kept:");
            egui::Grid::new("recoveries").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
                for (i, c) in app.recoveries.iter().enumerate() {
                    let place = c
                        .note
                        .module
                        .as_ref()
                        .map_or_else(|| "never saved".to_string(), |p| p.display().to_string());
                    ui.strong(&c.note.name).on_hover_text(place);
                    ui.label(age(c.note.time));
                    if ui.button("Recover").on_hover_text("Open it, and save to keep it").clicked()
                    {
                        recover = Some(i);
                    }
                    if ui.button("Discard").on_hover_text("Throw the unsaved work away").clicked() {
                        discard = Some(i);
                    }
                    ui.end_row();
                }
            });
            later = ui.button("Later").on_hover_text("Ask again at the next start").clicked();
        },
    );
    if let Some(i) = recover {
        // (As an action: the open module's unsaved work is asked about
        // first, and the copy stays listed until it is opened.)
        app.actions.push(crate::Action::Recover(app.recoveries[i].clone()));
    }
    if let Some(i) = discard {
        let copy = app.recoveries.remove(i);
        if !copy.remove() {
            app.log.warn(format!(
                "The unsaved work on {} is open in another Moonglow window: left as it is",
                copy.note.name
            ));
        }
    }
    if later || !open {
        app.recoveries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_is_made_when_one_is_due() {
        use std::time::Instant;
        let dir = mg_testkit::scratch_dir("ui-autosave");
        let mut m = Module::new();
        let key = mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        m.set(key, b"void main() {}".to_vec());
        m.save_as(&ModuleLocation::Archive(dir.join("m.mod"))).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.recovery_dir = Some(dir.join("recovery"));
        app.open_module(&dir.join("m.mod"));
        let t0 = Instant::now();
        assert_eq!(app.autosave_tick(t0), None, "nothing unsaved");
        let edit = mg_edit::Edit::SetResource { key, data: Some(b"// changed".to_vec()) };
        app.ws.as_mut().unwrap().apply(mg_edit::Command::new("edit", vec![edit])).unwrap();
        // The clock starts; five minutes later a copy is written.
        assert_eq!(app.autosave_tick(t0), Some(Duration::from_secs(300)));
        assert!(find_all(&dir.join("recovery")).is_empty());
        app.autosave_tick(t0 + Duration::from_secs(301));
        while app.autosave.busy.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(10));
        }
        let copies = find_all(&dir.join("recovery"));
        assert_eq!(copies.len(), 1);
        let copy = Module::open(&copies[0].archive).unwrap();
        assert_eq!(copy.get(&key), Some(&b"// changed"[..]));
        // Not again for the same revision.
        app.autosave_tick(t0 + Duration::from_secs(700));
        assert!(!app.autosave.busy.load(Ordering::Acquire));
        // Text typed in a script's editor and nothing else: a copy too, and
        // another when more is typed (it was in no copy until the script
        // was saved: the review of November 2026, S4).
        app.ws.as_mut().unwrap().save().unwrap();
        app.forget_recovery();
        assert_eq!(app.autosave_tick(t0), None, "saved");
        let source = (0, 0, 0);
        let mut buf = crate::script_view::ScriptBuffer::read(b"// changed", source);
        buf.text.push_str(" typed");
        app.scripts.insert(key, buf);
        let t1 = t0 + Duration::from_secs(2000);
        assert_eq!(app.autosave_tick(t1), Some(Duration::from_secs(300)));
        let wait = |app: &Moonglow| {
            while app.autosave.busy.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        let held = |dir: &Path| {
            let copies = find_all(&dir.join("recovery"));
            assert_eq!(copies.len(), 1);
            Module::open(&copies[0].archive).unwrap().get(&key).map(<[u8]>::to_vec)
        };
        app.autosave_tick(t1 + Duration::from_secs(301));
        wait(&app);
        assert_eq!(held(&dir).as_deref(), Some(&b"// changed typed"[..]));
        app.scripts.get_mut(&key).unwrap().text.push_str(" more");
        app.autosave_tick(t1 + Duration::from_secs(700));
        wait(&app);
        assert_eq!(held(&dir).as_deref(), Some(&b"// changed typed more"[..]));
        app.scripts.clear();
        app.forget_recovery();
        // Saved while a copy is being written: no copy is left, whichever
        // finishes first (one was offered at the next start, of work
        // saved since).
        let edit = mg_edit::Edit::SetResource { key, data: Some(b"// again".to_vec()) };
        app.ws.as_mut().unwrap().apply(mg_edit::Command::new("edit", vec![edit])).unwrap();
        app.autosave_tick(t0 + Duration::from_secs(1100));
        app.forget_recovery();
        while app.autosave.busy.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(find_all(&dir.join("recovery")).is_empty());
    }

    /// An application with a module open, a change unsaved, its copies in
    /// `dir/recovery`.
    fn changed(dir: &Path) -> Moonglow {
        let mut m = Module::new();
        let key = mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        m.set(key, b"void main() {}".to_vec());
        m.save_as(&ModuleLocation::Archive(dir.join("m.mod"))).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.recovery_dir = Some(dir.join("recovery"));
        app.open_module(&dir.join("m.mod"));
        let edit = mg_edit::Edit::SetResource { key, data: Some(b"// changed".to_vec()) };
        app.ws.as_mut().unwrap().apply(mg_edit::Command::new("edit", vec![edit])).unwrap();
        app
    }

    fn kept(dir: &Path) -> Option<Vec<u8>> {
        let key = mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        let copy = find_all(&dir.join("recovery")).into_iter().next()?;
        Module::open(&copy.archive).ok()?.get(&key).map(<[u8]>::to_vec)
    }

    /// A recovered module took the copy for its file: a module folder
    /// then refused the save ("the folder holds resources that are not the
    /// module's") where the recovered work had deleted or renamed one
    /// (the review of November 2026, S7).
    #[test]
    fn a_recovered_folder_module_saves_into_its_folder() {
        let dir = mg_testkit::scratch_dir("ui-recover-folder");
        let folder = dir.join("m");
        let (hello, other) = (
            mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap(),
            mg_resman::ResKey::parse("other", mg_core::ResType::NSS).unwrap(),
        );
        let mut m = Module::new();
        m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
        m.set(hello, b"void main() {}".to_vec());
        m.set(other, b"void main() {}".to_vec());
        m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
        assert!(folder.join("other.nss").is_file());
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.recovery_dir = Some(dir.join("recovery"));
        app.open_module(&folder);
        // A resource deleted, another changed; then the crash.
        let edits = vec![
            mg_edit::Edit::SetResource { key: other, data: None },
            mg_edit::Edit::SetResource { key: hello, data: Some(b"// kept".to_vec()) },
        ];
        app.ws.as_mut().unwrap().apply(mg_edit::Command::new("edit", edits)).unwrap();
        app.write_recovery().unwrap();
        // (Gone with the crash, and its hold on the copy with it.)
        drop(app);
        let copy = find(&dir.join("recovery")).remove(0);
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.recovery_dir = Some(dir.join("recovery"));
        app.recover(&copy);
        assert!(app.ws.as_ref().unwrap().is_modified());
        app.save(None);
        assert!(!app.has_unsaved_work(), "{:?}", app.log.entries);
        assert!(!folder.join("other.nss").exists(), "deleted with the save");
        assert_eq!(std::fs::read(folder.join("hello.nss")).unwrap(), b"// kept");
    }

    /// Save As (and a new module's first save) left the copy made under
    /// the old place: offered as unsaved work at the next start, and
    /// Recover put it over the old module (the review of November 2026,
    /// S6).
    #[test]
    fn a_save_elsewhere_takes_the_old_place_s_copy_with_it() {
        let dir = mg_testkit::scratch_dir("ui-recovery-save-as");
        let mut app = changed(&dir);
        app.write_recovery().unwrap();
        assert_eq!(find_all(&dir.join("recovery")).len(), 1);
        app.save(Some(ModuleLocation::Archive(dir.join("other.mod"))));
        assert!(dir.join("other.mod").is_file());
        assert!(find_all(&dir.join("recovery")).is_empty(), "the work is saved");
    }

    /// A panic in a thread of its own (a job's, an autosave's) leaves the
    /// window going: the next frame writes the copy, once a panic.
    #[test]
    fn a_panic_elsewhere_has_the_next_frame_keep_the_work() {
        let dir = mg_testkit::scratch_dir("ui-recovery-elsewhere");
        let mut app = changed(&dir);
        app.recovering(|_| {});
        assert_eq!(kept(&dir), None, "no panic, no copy");
        note_panic();
        app.recovering(|_| {});
        while app.autosave.busy.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(kept(&dir).as_deref(), Some(&b"// changed"[..]));
        assert!(app.log.entries.iter().any(|(_, m)| m.contains("kept in a recovery copy")));
        // Not again until another panic.
        find_all(&dir.join("recovery"))[0].remove_files();
        app.recovering(|_| {});
        assert!(!app.autosave.busy.load(Ordering::Acquire) && kept(&dir).is_none());
    }

    /// A panic outside the frame (eframe's own code) unwinds through the
    /// application: as it is dropped, the copy is written.
    #[test]
    fn a_panic_unwinding_through_the_application_keeps_the_work() {
        struct Window(Moonglow);
        impl Drop for Window {
            fn drop(&mut self) {
                self.0.keep_while_unwinding();
            }
        }
        let dir = mg_testkit::scratch_dir("ui-recovery-unwinding");
        let window = Window(changed(&dir));
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _window = window;
            panic!("in eframe");
        }));
        assert!(r.is_err());
        assert_eq!(kept(&dir).as_deref(), Some(&b"// changed"[..]));
        // Dropped with no panic under way: no copy.
        let dir = mg_testkit::scratch_dir("ui-recovery-closed");
        drop(Window(changed(&dir)));
        assert_eq!(kept(&dir), None);
    }

    /// An autosave whose writing panics is over all the same: the next
    /// one is made, and a crash doesn't wait for it.
    #[test]
    fn an_autosave_that_panics_is_done() {
        let busy = Arc::new(AtomicBool::new(true));
        let held = busy.clone();
        let _ = std::thread::spawn(move || {
            let _done = Done(held);
            panic!("while writing");
        })
        .join();
        assert!(!busy.load(Ordering::Acquire));
    }

    /// An archive whose note is not there, is empty or is cut short is
    /// still offered (the note was written in place, after the archive:
    /// the review of November 2026, S23), named from its file, and
    /// recovered as a module never saved; removing it takes both files.
    /// A note without an archive is no copy.
    #[test]
    fn an_archive_without_a_note_that_reads_is_still_found() {
        let dir = mg_testkit::scratch_dir("ui-recovery-no-note");
        let mut app = changed(&dir);
        let archive = app.write_recovery().unwrap();
        let note = archive.with_extension("json");
        let found = find_all(&dir.join("recovery"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].note.module.as_deref(), Some(dir.join("m.mod").as_path()));
        let whole = std::fs::read(&note).unwrap();
        for cut in [Some(&whole[..whole.len() / 2]), Some(&b""[..]), None] {
            match cut {
                Some(text) => std::fs::write(&note, text).unwrap(),
                None => std::fs::remove_file(&note).unwrap(),
            }
            let found = find_all(&dir.join("recovery"));
            assert_eq!(found.len(), 1, "{cut:?}");
            assert_eq!(found[0].archive, archive);
            let made = Note { time: found[0].note.time, ..note_from_file(&archive) };
            assert_eq!(found[0].note, made);
            assert_eq!((made.name.as_str(), made.module), ("m", None));
            assert!(now().abs_diff(made.time) < 600, "made when the file was: {}", made.time);
        }
        assert_eq!(kept(&dir).as_deref(), Some(&b"// changed"[..]));
        // (The window gone with the crash, and its hold on the copy.)
        drop(app);
        let copy = find(&dir.join("recovery")).remove(0);
        let mut again = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        again.recover(&copy);
        let ws = again.ws.as_ref().unwrap();
        assert!(ws.is_modified() && ws.module.location.is_none(), "never saved");
        drop(again);
        assert!(copy.remove());
        assert!(!archive.exists() && find_all(&dir.join("recovery")).is_empty());
        // A note alone (the copy's first writing stopped after it).
        std::fs::write(&note, &whole).unwrap();
        assert!(find_all(&dir.join("recovery")).is_empty());
        // Nothing is left beside a copy written but its lock.
        let mut app = changed(&dir);
        app.write_recovery().unwrap();
        let mut files: Vec<String> = (std::fs::read_dir(dir.join("recovery")).unwrap())
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        let stem = archive.file_stem().unwrap().to_string_lossy().into_owned();
        assert_eq!(files, [format!("{stem}.json"), format!("{stem}.lock"), format!("{stem}.mod")]);
    }

    /// A copy a running Moonglow holds is its own work, still open: a
    /// second window does not offer it as a crashed session's, keeps a
    /// copy of its own beside it (`-2`) for the same module, and neither
    /// its save nor its Discard touches the first one's. Once the first
    /// window is gone without having saved, its copy is offered; the
    /// window that recovers it holds it, and removes it with its lock
    /// when the work is saved (the plugins and command line page's C27).
    #[test]
    fn a_copy_a_running_window_holds_is_its_own() {
        let dir = mg_testkit::scratch_dir("ui-recovery-held");
        let recovery = dir.join("recovery");
        let key = mg_resman::ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        let text = |archive: &Path| Module::open(archive).unwrap().get(&key).map(<[u8]>::to_vec);
        let mut first = changed(&dir);
        let one = first.write_recovery().unwrap();
        assert!(one.with_extension("lock").is_file());
        assert!(find(&recovery).is_empty(), "held by the window that wrote it");
        assert_eq!(find_all(&recovery).len(), 1);
        // A second window, the same module open and changed otherwise.
        let mut second = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        second.recovery_dir = Some(recovery.clone());
        second.find_recoveries();
        assert!(second.recoveries.is_empty(), "not a crashed session's");
        second.open_module(&dir.join("m.mod"));
        let edit = mg_edit::Edit::SetResource { key, data: Some(b"// second".to_vec()) };
        second.ws.as_mut().unwrap().apply(mg_edit::Command::new("edit", vec![edit])).unwrap();
        let two = second.write_recovery().unwrap();
        let stem = one.file_stem().unwrap().to_string_lossy().into_owned();
        assert_eq!(two, recovery.join(format!("{stem}-2.mod")));
        assert_eq!(second.write_recovery().unwrap(), two, "the same one again");
        assert_eq!(text(&one).as_deref(), Some(&b"// changed"[..]), "the first one's, as it was");
        assert_eq!(text(&two).as_deref(), Some(&b"// second"[..]));
        // (Named from its file, were its note lost.)
        assert_eq!(note_from_file(&two).name, "m");
        // Its Discard of the first one's (listed by a start before the
        // lock was held) leaves it; its save removes its own alone.
        let theirs = find_all(&recovery).into_iter().find(|c| c.archive == one).unwrap();
        assert!(!theirs.remove() && one.is_file());
        second.save(None);
        assert!(!two.exists() && !two.with_extension("lock").exists(), "saved: forgotten");
        assert!(one.is_file() && one.with_extension("json").is_file());
        assert!(find(&recovery).is_empty());
        // The first window goes without saving: its copy is offered, and
        // the window that recovers it holds it.
        drop(first);
        let mut third = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        third.recovery_dir = Some(recovery.clone());
        third.find_recoveries();
        assert_eq!(third.recoveries.len(), 1);
        let copy = third.recoveries[0].clone();
        assert_eq!(copy.archive, one);
        third.recover(&copy);
        assert!(third.has_unsaved_work());
        assert!(one.is_file(), "kept until the work is saved");
        assert!(find(&recovery).is_empty(), "the recovering window's now");
        // (Another window that listed it too can't open it as well.)
        let mut fourth = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        fourth.recovery_dir = Some(recovery.clone());
        fourth.recover(&copy);
        assert!(fourth.ws.is_none(), "{:?}", fourth.log.entries);
        // Written again as the work goes on, in its place; gone when saved.
        assert_eq!(third.write_recovery().unwrap(), one);
        third.save(None);
        assert!(!third.has_unsaved_work(), "{:?}", third.log.entries);
        let left: Vec<_> =
            std::fs::read_dir(&recovery).unwrap().map(|e| e.unwrap().path()).collect();
        assert!(left.is_empty(), "{left:?}");
        assert_eq!(text(&dir.join("m.mod")).as_deref(), Some(&b"// changed"[..]));
    }

    /// A lock is taken once, let go with its file when its copy is
    /// forgotten, and a folder that takes no lock file leaves the copy
    /// used unlocked, as before.
    #[test]
    fn a_lock_is_held_once_and_goes_with_its_copy() {
        let dir = mg_testkit::scratch_dir("ui-recovery-lock");
        let archive = dir.join("m-00000000.mod");
        let Hold::Ours(lock) = hold(&archive) else { panic!("nobody holds it") };
        assert!(held(&archive));
        assert!(matches!(hold(&archive), Hold::Others));
        lock.forgotten.store(true, Ordering::Release);
        drop(lock);
        assert!(!archive.with_extension("lock").exists());
        assert!(!held(&archive));
        assert_eq!(nth_archive(&archive, 1), archive);
        assert_eq!(nth_archive(&archive, 3), dir.join("m-00000000-3.mod"));
        // (A file where the folder would be: no lock can be made.)
        std::fs::write(dir.join("file"), b"").unwrap();
        assert!(matches!(hold(&dir.join("file").join("m.mod")), Hold::Unlocked));
    }

    #[test]
    fn copies_are_named_by_module_and_place() {
        let a = file_stem(Some(Path::new("/m/a/Keep.mod")), "Keep");
        let b = file_stem(Some(Path::new("/m/b/Keep.mod")), "Keep");
        assert!(a.starts_with("Keep-") && a != b);
        assert_eq!(file_stem(None, "a b/c"), file_stem(None, "a b/c"));
        assert!(file_stem(None, "a b/c").starts_with("a_b_c-"));
        assert_eq!(age(now()), "less than a minute ago");
        assert_eq!(age(now() - 600), "10 minutes ago");
    }
}
