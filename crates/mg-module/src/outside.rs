//! A module folder's or a nasher project's files changed outside Moonglow
//! (an editor, `git pull`), looked for in three steps so that the looking
//! can be done off the window's thread: [`crate::Module::outside_scan`]
//! takes what is known of the files (cheap: their bytes are shared);
//! [`OutsideScan::run`] looks at every file's time and size and reads and
//! converts those changed, anywhere; [`crate::Module::outside_found`]
//! takes what it found back into the module's notes. What Moonglow
//! itself changed meanwhile (a save, a change taken) is left out of what
//! is handed back: the next look finds it again if it still differs.
//! [`crate::Module::outside`] does all three at once. Where the system
//! said which files changed (a watch of the folder:
//! [`crate::Module::outside_scan_of`]) only they are looked at, and no
//! folder is walked.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use mg_resman::ResKey;

use crate::nasher::{Conversion, Outside, Target};

/// A file's modification time and size.
pub(crate) type Stamp = (SystemTime, u64);

pub(crate) fn stamp(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// A resource's file as last read or written.
#[derive(Debug, Clone)]
pub(crate) struct Known {
    pub(crate) key: ResKey,
    pub(crate) path: Arc<Path>,
    /// A GFF kept as JSON (a project's), not the resource itself.
    pub(crate) json: bool,
    /// The file's bytes then.
    pub(crate) file: Arc<[u8]>,
    /// Its time and size when last found as it was: one that still has
    /// them is not read.
    pub(crate) stamp: Option<Stamp>,
}

#[derive(Debug, Clone)]
pub(crate) enum Kind {
    Project {
        root: PathBuf,
        target: Box<Target>,
        conversion: Conversion,
    },
    Folder {
        dir: PathBuf,
    },
    /// A module kept in one file, its time and size when last read.
    Archive {
        path: PathBuf,
        stamp: Option<Stamp>,
    },
}

/// What is known of a module's files, to look for those changed outside
/// ([`OutsideScan::run`]), on any thread.
#[derive(Debug, Clone)]
pub struct OutsideScan {
    pub(crate) files: Vec<Known>,
    pub(crate) kind: Kind,
    /// The files to look at, where the system said which changed
    /// ([`crate::Module::outside_scan_of`]); `None`: all of them, and the
    /// folders walked for new ones.
    pub(crate) only: Option<std::collections::HashSet<PathBuf>>,
}

/// What [`OutsideScan::run`] found, for [`crate::Module::outside_found`].
#[derive(Debug, Clone, Default)]
pub struct OutsideFound {
    pub(crate) changes: Vec<Outside>,
    pub(crate) problems: Vec<String>,
    /// Files looked at and found as they were known: their times now.
    pub(crate) settled: Vec<(ResKey, Arc<[u8]>, Stamp)>,
    /// A project's files that its include leaves out, looked at because a
    /// file is gone, and whether a gone one is among them (it was moved
    /// there).
    pub(crate) left_out: Option<(Vec<PathBuf>, bool)>,
    pub(crate) project: bool,
    /// An archive's look, that read it: its time and size then.
    pub(crate) archive: Option<Stamp>,
}

impl OutsideScan {
    /// The files known.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Looks at the files: those whose time or size changed are read, and
    /// those whose bytes changed (or that are gone, or new) are what is
    /// found.
    pub fn run(self) -> OutsideFound {
        match &self.kind {
            Kind::Project { root, target, conversion } => {
                crate::nasher::scan(&self.files, root, target, *conversion, self.only.as_ref())
            }
            Kind::Folder { dir } => crate::folder::scan(&self.files, dir, self.only.as_ref()),
            Kind::Archive { path, stamp } => crate::archive_file::scan(&self.files, path, *stamp),
        }
    }
}

impl OutsideFound {
    /// The files found changed, added or deleted.
    pub fn changes(&self) -> &[Outside] {
        &self.changes
    }
}

/// Whether a change found still stands, `known` being what is known of
/// its file now: not if a save or a change taken since replaced what was
/// known when it was found, nor if a file found gone is back (a checkout
/// that deletes and writes again). (A file written again since it was
/// read is taken as it was read: it was taken with the time it had then,
/// so the next look finds the write. Looking at every changed file's
/// time again here took two milliseconds of the window's frame for a
/// thousand.)
pub(crate) fn stands(known: Option<&Arc<[u8]>>, change: &Outside) -> bool {
    let same = match (known, change.was()) {
        (Some(now), Some(was)) => Arc::ptr_eq(now, was),
        (None, None) => true,
        _ => false,
    };
    same && (change.resource.is_some() || !change.path.exists())
}
