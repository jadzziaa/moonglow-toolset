//! What a job puts into haks, outside its memory and on disk: the bytes it
//! handed over past what a job holds in memory, in scratch files until the
//! haks are written; and each hak as it was before a job wrote it and as
//! the job left it, so that the job's step of Undo (or its edits failing)
//! takes the hak back, and Redo writes it again.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use mg_module::hak_edit::{STASH, sweep_left, sweep_stashes};

/// How many bytes of what a job puts into haks it holds in memory, all
/// its haks together; past them each resource is kept in a scratch file
/// until the haks are written.
pub const HELD_IN_MEMORY: usize = 64 << 20;

/// A job's scratch folder of the bytes it put into haks past
/// [`HELD_IN_MEMORY`]: in the system's temporary folder, among the hak
/// editor's (`moonglow-hak-<process>`, swept once the process is gone),
/// and taken away with the last of the job's haks.
#[derive(Debug)]
pub struct Held {
    dir: PathBuf,
}

impl PartialEq for Held {
    fn eq(&self, other: &Held) -> bool {
        self.dir == other.dir
    }
}

impl Eq for Held {}

impl Default for Held {
    fn default() -> Held {
        Held::new()
    }
}

impl Held {
    /// A new scratch folder (made when its first file is written).
    pub(crate) fn new() -> Held {
        let temp = std::env::temp_dir();
        static SWEPT: std::sync::Once = std::sync::Once::new();
        SWEPT.call_once(|| _ = sweep_stashes(&temp));
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = temp.join(format!("{STASH}{}", std::process::id())).join(format!("plugin-{n}"));
        Held { dir }
    }

    /// Keeps `bytes` in a file of its own: its path.
    pub(crate) fn keep(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("{}: {e}", self.dir.display()))?;
        let path = self.dir.join(format!("{n}-{name}"));
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }

    /// Its folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
        // (The process's folder, once nothing else keeps a file there.)
        if let Some(parent) = self.dir.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }
}

/// What the folder of the copies a hak folder keeps for Undo is named,
/// before the process's number: hidden, in the hak folder itself (so a
/// copy is a link to the same file, not the bytes again).
pub const UNDO_FOLDER: &str = ".moonglow-undo-";

/// A file's length and when it was last written: by them a hak is known
/// to be as it was left.
type Stamp = (u64, Option<SystemTime>);

fn stamp(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.len(), m.modified().ok()))
}

/// `from` linked (or, where links can't be made, copied) as `to`.
fn link(from: &Path, to: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(to);
    if std::fs::hard_link(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("{}: {e}", to.display()))
}

/// `from` put in the place of `path`, whole or not at all.
fn put_back(from: &Path, path: &Path) -> Result<(), String> {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".moonglow-tmp");
    let part = path.with_file_name(name);
    link(from, &part)?;
    std::fs::rename(&part, path).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        format!("{}: {e}", path.display())
    })
}

/// A hak a job wrote, with what it was before (a copy kept, or none: it
/// was made) and what the job left (a copy kept, for Redo). The copies go
/// when this does.
#[derive(Debug)]
pub struct HakWritten {
    /// The hak's file.
    pub path: PathBuf,
    /// Resources added to it, and those of the same name replaced.
    pub added: usize,
    pub replaced: usize,
    before: Option<PathBuf>,
    after: PathBuf,
    /// The file as it was before (None: there was none), and as written.
    before_stamp: Option<Stamp>,
    after_stamp: Option<Stamp>,
}

impl HakWritten {
    /// Writes `hak` into the hak folder `dir`, keeping copies of the hak
    /// before and after in the folder's undo copies.
    pub(crate) fn write(hak: &crate::HakWrite, dir: &Path) -> Result<HakWritten, String> {
        let path = dir.join(format!("{}.hak", hak.name));
        let copies = dir.join(format!("{UNDO_FOLDER}{}", std::process::id()));
        // (Once a hak folder: the copies a Moonglow that crashed left.)
        static SWEPT: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());
        {
            let mut swept = SWEPT.lock().unwrap_or_else(|e| e.into_inner());
            if !swept.iter().any(|d| d == dir) {
                sweep_left(dir, UNDO_FOLDER);
                swept.push(dir.to_path_buf());
            }
        }
        std::fs::create_dir_all(&copies).map_err(|e| format!("{}: {e}", copies.display()))?;
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let before_stamp = stamp(&path);
        let before = match before_stamp {
            Some(_) => {
                let copy = copies.join(format!("{n}-before-{}.hak", hak.name));
                link(&path, &copy)?;
                Some(copy)
            }
            None => None,
        };
        let forget = |before: &Option<PathBuf>| {
            if let Some(b) = before {
                let _ = std::fs::remove_file(b);
            }
            let _ = std::fs::remove_dir(&copies);
        };
        let done = match mg_module::hak_edit::write_into(&path, hak.files.clone()) {
            Ok(done) => done,
            Err(e) => {
                forget(&before);
                return Err(e);
            }
        };
        let after = copies.join(format!("{n}-after-{}.hak", hak.name));
        if let Err(e) = link(&path, &after) {
            forget(&before);
            return Err(e);
        }
        Ok(HakWritten {
            added: done.added,
            replaced: done.replaced,
            after_stamp: stamp(&path),
            path,
            before,
            after,
            before_stamp,
        })
    }

    /// Whether the hak was there before the job wrote it.
    pub fn was_there(&self) -> bool {
        self.before.is_some()
    }

    /// Takes the hak back to what it was before the job wrote it (removed,
    /// if the job made it); refused when it was changed since the job
    /// wrote it, which is then left as it is.
    pub fn undo(&mut self) -> Result<(), String> {
        if stamp(&self.path) != self.after_stamp {
            return Err(format!(
                "{} was changed since the plugin wrote it: left as it is",
                self.path.display()
            ));
        }
        match &self.before {
            Some(before) => put_back(before, &self.path)?,
            None => std::fs::remove_file(&self.path)
                .map_err(|e| format!("{}: {e}", self.path.display()))?,
        }
        // (A copy, where links can't be made, is a file of its own time.)
        self.before_stamp = stamp(&self.path);
        Ok(())
    }

    /// Writes the hak again as the job left it (after [`HakWritten::undo`]);
    /// refused when it was changed since it was taken back.
    pub fn redo(&mut self) -> Result<(), String> {
        if stamp(&self.path) != self.before_stamp {
            return Err(format!(
                "{} was changed since the plugin's hak was taken back: left as it is",
                self.path.display()
            ));
        }
        put_back(&self.after, &self.path)?;
        self.after_stamp = stamp(&self.path);
        Ok(())
    }
}

impl Drop for HakWritten {
    fn drop(&mut self) {
        if let Some(b) = &self.before {
            let _ = std::fs::remove_file(b);
        }
        let _ = std::fs::remove_file(&self.after);
        if let Some(dir) = self.after.parent() {
            let _ = std::fs::remove_dir(dir);
        }
    }
}
