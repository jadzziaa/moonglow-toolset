//! A module folder's files as Moonglow last read or wrote them, to tell
//! what changed outside it since (a script kept open in another editor,
//! a file copied in): those are read again rather than written over, as
//! a nasher project's are.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use mg_resman::ResKey;

use crate::nasher::Outside;

type Stamp = (SystemTime, u64);

fn stamp(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// What each resource's file held when it was read or written.
#[derive(Debug, Clone, Default)]
pub struct FolderFiles {
    dir: PathBuf,
    files: HashMap<ResKey, (PathBuf, Arc<[u8]>)>,
    /// Each file's time and size when it was last looked at and found as
    /// it was read: a file with the same is not read again.
    stamps: HashMap<PathBuf, Stamp>,
}

impl FolderFiles {
    pub(crate) fn new(dir: &Path) -> FolderFiles {
        FolderFiles { dir: dir.to_path_buf(), ..Default::default() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Notes a file as read or written with these bytes.
    pub(crate) fn note(&mut self, key: ResKey, path: PathBuf, bytes: Arc<[u8]>) {
        self.stamps.extend(stamp(&path).map(|s| (path.clone(), s)));
        self.files.insert(key, (path, bytes));
    }

    /// Notes a file as last read with these bytes, whatever it holds
    /// now (it is not looked at: its change is still to be found).
    pub(crate) fn note_as(&mut self, key: ResKey, path: PathBuf, bytes: Arc<[u8]>) {
        self.files.insert(key, (path, bytes));
    }

    pub(crate) fn forget(&mut self, key: &ResKey) {
        if let Some((path, _)) = self.files.remove(key) {
            self.stamps.remove(&path);
        }
    }

    pub(crate) fn knows(&self, key: &ResKey) -> bool {
        self.files.contains_key(key)
    }

    /// The file a resource is kept in.
    pub fn path(&self, key: &ResKey) -> Option<&Path> {
        self.files.get(key).map(|(p, _)| p.as_path())
    }

    /// What a resource's file held when it was last read or written.
    pub fn as_read(&self, key: &ResKey) -> Option<&[u8]> {
        self.files.get(key).map(|(_, b)| &b[..])
    }

    /// The files changed, added or deleted outside Moonglow since they
    /// were read or written. Each is named again every time until it is
    /// taken ([`FolderFiles::took`]) or is as it was again. Files whose
    /// time and size are as they were are not read.
    pub fn outside(&mut self) -> (Vec<Outside>, Vec<String>) {
        let (mut changes, mut problems) = (Vec::new(), Vec::new());
        let mut settled = Vec::new();
        for (key, (path, read)) in &self.files {
            let now = stamp(path);
            if now.is_some() && self.stamps.get(path) == now.as_ref() {
                continue;
            }
            match std::fs::read(path) {
                Err(_) => changes.push(Outside::plain(*key, path.clone(), None)),
                Ok(bytes) if bytes[..] == read[..] => {
                    settled.extend(now.map(|s| (path.clone(), s)));
                }
                Ok(bytes) => {
                    changes.push(Outside::plain(*key, path.clone(), Some(Arc::from(bytes))))
                }
            }
        }
        self.stamps.extend(settled);
        // Files that are new to the folder.
        match std::fs::read_dir(&self.dir) {
            Err(e) => problems.push(format!("{}: {e}", self.dir.display())),
            Ok(entries) => {
                let mut added: Vec<Outside> = Vec::new();
                for path in entries.flatten().map(|e| e.path()).filter(|p| p.is_file()) {
                    let name = path.file_name().and_then(|n| n.to_str());
                    let Some(key) = name.and_then(crate::folder_key) else { continue };
                    // (Moonglow's own file being written, and what it knows.)
                    if self.files.contains_key(&key) || added.iter().any(|a| a.key == key) {
                        continue;
                    }
                    if let Ok(bytes) = std::fs::read(&path) {
                        added.push(Outside::plain(key, path, Some(Arc::from(bytes))));
                    }
                }
                changes.extend(added);
            }
        }
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        (changes, problems)
    }

    /// Counts a file changed outside as read: what a save compares with
    /// from now on. (Whether the module takes its resource is the
    /// caller's: one that keeps its own will write it over the file.)
    pub fn took(&mut self, change: &Outside) {
        match &change.resource {
            Some(bytes) => self.note(change.key, change.path.clone(), bytes.clone()),
            None => self.forget(&change.key),
        }
    }
}
