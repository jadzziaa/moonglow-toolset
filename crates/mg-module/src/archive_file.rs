//! A module kept in one file (a `.mod`) changed outside Moonglow while it
//! is open (Aurora saved it, a build wrote it again): the file's time and
//! size are noted when it is read or written, and when they change it is
//! read again ([`crate::outside`], on another thread), each resource
//! compared with what was read, so that what changed is found as a
//! folder's files are, and taken or asked about the same way.
//!
//! A change found is of one file for all its resources: its time is
//! counted as read once every change found at it was taken (or kept as
//! Moonglow has it), so that one waiting for an answer is found again.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mg_erf::Erf;
use mg_resman::ResKey;

use crate::nasher::Outside;
use crate::outside::{Known, OutsideFound, OutsideScan, Stamp, stamp};

/// A module archive's resources as last read or written.
#[derive(Debug, Clone)]
pub struct ArchiveFile {
    path: PathBuf,
    /// Its time and size then.
    stamp: Option<Stamp>,
    read: HashMap<ResKey, Arc<[u8]>>,
    /// The changes found at the file's time now, not yet taken: once they
    /// all are, the time is counted as read.
    waiting: Option<(Stamp, HashSet<ResKey>)>,
}

/// Where a resource of the archive is said to be: `castle.mod/area1.are`.
fn place(path: &Path, key: &ResKey) -> PathBuf {
    path.join(key.to_string())
}

impl ArchiveFile {
    /// The archive at `path`, as just read or written with `resources`.
    pub(crate) fn new<'a>(
        path: &Path,
        resources: impl Iterator<Item = (&'a ResKey, &'a Arc<[u8]>)>,
    ) -> ArchiveFile {
        ArchiveFile {
            path: path.to_path_buf(),
            stamp: stamp(path),
            read: resources.map(|(k, v)| (*k, v.clone())).collect(),
            waiting: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether changes found in the file (another program's) wait to be
    /// taken or kept as Moonglow has them: a save would write over them
    /// unasked. (One not looked for, with Options' reloading off or by
    /// `mg`, is written over, as before.)
    pub fn changed_outside(&self) -> bool {
        self.waiting.is_some()
    }

    /// What a resource was when the archive was last read or written.
    pub fn as_read(&self, key: &ResKey) -> Option<&[u8]> {
        self.read.get(key).map(|b| &b[..])
    }

    /// What is known of the archive, to look at it anywhere.
    pub(crate) fn outside_scan(&self) -> OutsideScan {
        let files = (self.read.iter())
            .map(|(key, file)| Known {
                key: *key,
                path: place(&self.path, key).into(),
                json: false,
                file: file.clone(),
                stamp: self.stamp,
            })
            .collect();
        let kind = crate::outside::Kind::Archive { path: self.path.clone(), stamp: self.stamp };
        OutsideScan { files, kind, only: None }
    }

    /// Takes what [`OutsideScan::run`] found: the changes that still stand
    /// (not those to resources taken or saved since).
    pub(crate) fn outside_found(&mut self, found: OutsideFound) -> (Vec<Outside>, Vec<String>) {
        let Some(now) = found.archive else { return (Vec::new(), found.problems) };
        let mut changes = found.changes;
        changes.retain(|c| crate::outside::stands(self.read.get(&c.key), c));
        if changes.is_empty() {
            self.stamp = Some(now);
            self.waiting = None;
        } else {
            self.waiting = Some((now, changes.iter().map(|c| c.key).collect()));
        }
        (changes, found.problems)
    }

    /// Counts a change found as read (taken, or kept as Moonglow has it):
    /// what a save compares with from now on.
    pub fn took(&mut self, change: &Outside) {
        match &change.resource {
            Some(bytes) => self.read.insert(change.key, bytes.clone()),
            None => self.read.remove(&change.key),
        };
        if let Some((now, keys)) = &mut self.waiting {
            keys.remove(&change.key);
            if keys.is_empty() {
                self.stamp = Some(*now);
                self.waiting = None;
            }
        }
    }
}

/// [`OutsideScan::run`] for an archive: nothing while its time and size
/// are as they were; else it is read whole and each resource compared.
pub(crate) fn scan(files: &[Known], path: &Path, was: Option<Stamp>) -> OutsideFound {
    let mut found = OutsideFound::default();
    let Some(now) = stamp(path) else {
        // (Gone, or being written: looked at again when it is back.)
        return found;
    };
    if was == Some(now) {
        return found;
    }
    let read = std::fs::read(path).map_err(|e| e.to_string()).and_then(|data| {
        let erf = Erf::read(&data).map_err(|e| e.to_string())?;
        let mut now: HashMap<ResKey, Arc<[u8]>> = HashMap::new();
        for e in &erf.entries {
            let key = ResKey::new(e.resref, e.restype);
            // (Listed twice: the first, as the module is read.)
            if now.contains_key(&key) {
                continue;
            }
            let bytes = erf.data(e).map_err(|e| e.to_string())?;
            now.insert(key, Arc::from(bytes.as_ref()));
        }
        Ok(now)
    });
    let mut now_has = match read {
        Ok(r) => r,
        Err(e) => {
            found.problems.push(format!("{}: {e}", path.display()));
            return found;
        }
    };
    found.archive = Some(now);
    for k in files {
        match now_has.remove(&k.key) {
            Some(bytes) if bytes[..] == k.file[..] => {}
            resource => found.changes.push(Outside::plain(
                k.key,
                k.path.to_path_buf(),
                resource,
                Some(k.file.clone()),
                Some(now),
            )),
        }
    }
    let mut added: Vec<(ResKey, Arc<[u8]>)> = now_has.into_iter().collect();
    added.sort_by_key(|(k, _)| *k);
    for (key, bytes) in added {
        found.changes.push(Outside::plain(key, place(path, &key), Some(bytes), None, Some(now)));
    }
    found.changes.sort_by(|a, b| a.path.cmp(&b.path));
    found
}
