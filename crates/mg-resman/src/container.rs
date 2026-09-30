//! Resource containers: the layers a [`crate::ResMan`] stacks.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use memmap2::Mmap;
use mg_core::{ResRef, ResType};
use mg_erf::{Entry, Erf};
use mg_key::KeySet;

use crate::{ResError, ResKey};

/// A source of resources.
pub trait Container: Send + Sync + fmt::Debug {
    /// Whether the container has the resource.
    fn contains(&self, key: &ResKey) -> bool;
    /// The resource's bytes.
    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError>;
    /// Every resource in the container (any order).
    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[allow(unsafe_code)]
pub(crate) fn map_file(path: &Path) -> Result<Mmap, ResError> {
    let file = File::open(path).map_err(|e| ResError::io(path, e))?;
    // SAFETY: game archives are treated as read-only while mapped; a file
    // truncated underneath us can fault reads, the accepted risk of
    // memory-mapping (the game and every Aurora tool do the same).
    unsafe { Mmap::map(&file) }.map_err(|e| ResError::io(path, e))
}

/// The base game's KEY/BIF archives.
#[derive(Debug)]
pub struct KeyContainer(pub KeySet);

impl KeyContainer {
    pub fn open(key: &Path, install_root: &Path) -> Result<KeyContainer, ResError> {
        KeySet::open(key, install_root)
            .map(KeyContainer)
            .map_err(|e| ResError::Container { path: key.into(), message: e.to_string() })
    }

    /// Opens a key whose BIFs may live in a language directory: each BIF is
    /// looked up in `lang_data` by file name first, then under `install_root`
    /// (as the game does for `nwn_base_loc.key`).
    pub fn open_localized(
        key: &Path,
        install_root: &Path,
        lang_data: &Path,
    ) -> Result<KeyContainer, ResError> {
        // BIF paths in a localized key are relative to the language root.
        let lang_root = lang_data.parent().unwrap_or(lang_data);
        match KeySet::open(key, lang_root) {
            Ok(k) => Ok(KeyContainer(k)),
            Err(_) => Self::open(key, install_root),
        }
    }
}

impl Container for KeyContainer {
    fn contains(&self, key: &ResKey) -> bool {
        self.0.find(&key.resref, key.restype).is_some()
    }

    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        let e = self.0.find(&key.resref, key.restype).ok_or(ResError::NotFound(*key))?;
        self.0
            .data(e)
            .map(Cow::Borrowed)
            .map_err(|e| ResError::Container { path: self.0.path.clone(), message: e.to_string() })
    }

    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        Box::new(self.0.table.entries.iter().map(|e| ResKey::new(e.resref, e.restype)))
    }

    fn len(&self) -> usize {
        self.0.table.entries.len()
    }
}

/// An ERF archive (hak, module, texture pack) on disk, memory-mapped.
pub struct ErfContainer {
    pub path: PathBuf,
    map: Mmap,
    entries: HashMap<ResKey, Entry>,
}

impl fmt::Debug for ErfContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErfContainer")
            .field("path", &self.path)
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl ErfContainer {
    pub fn open(path: &Path) -> Result<ErfContainer, ResError> {
        let map = map_file(path)?;
        let erf = Erf::read(&map)
            .map_err(|e| ResError::Container { path: path.into(), message: e.to_string() })?;
        let mut entries = HashMap::with_capacity(erf.entries.len());
        for e in erf.entries {
            // The first of duplicate names wins, as in `Erf::find`.
            entries.entry(ResKey::new(e.resref, e.restype)).or_insert(e);
        }
        Ok(ErfContainer { path: path.into(), map, entries })
    }
}

impl Container for ErfContainer {
    fn contains(&self, key: &ResKey) -> bool {
        self.entries.contains_key(key)
    }

    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        let e = self.entries.get(key).ok_or(ResError::NotFound(*key))?;
        mg_erf::entry_data(&self.map, e).map_err(|err| ResError::Container {
            path: self.path.clone(),
            message: err.to_string(),
        })
    }

    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        Box::new(self.entries.keys().copied())
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// A directory of loose files named `resref.ext` (override, development,
/// portraits, a module folder). Files with unknown extensions or names that
/// are not valid resrefs are ignored. The listing is taken when opened; call
/// [`DirContainer::rescan`] to pick up changes.
#[derive(Debug)]
pub struct DirContainer {
    pub path: PathBuf,
    files: HashMap<ResKey, PathBuf>,
}

impl DirContainer {
    /// Opens a directory; a missing directory is an empty container.
    pub fn open(path: &Path) -> DirContainer {
        let mut d = DirContainer { path: path.into(), files: HashMap::new() };
        d.rescan();
        d
    }

    pub fn rescan(&mut self) {
        self.files.clear();
        let Ok(rd) = std::fs::read_dir(&self.path) else { return };
        let mut entries: Vec<PathBuf> = rd
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.path())
            .collect();
        // Deterministic choice if two names differ only in case.
        entries.sort();
        for p in entries {
            if let Some(key) =
                p.file_name().and_then(|n| n.to_str()).and_then(ResKey::from_filename)
            {
                self.files.entry(key).or_insert(p);
            }
        }
    }
}

impl Container for DirContainer {
    fn contains(&self, key: &ResKey) -> bool {
        self.files.contains_key(key)
    }

    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        let p = self.files.get(key).ok_or(ResError::NotFound(*key))?;
        std::fs::read(p).map(Cow::Owned).map_err(|e| ResError::io(p, e))
    }

    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        Box::new(self.files.keys().copied())
    }

    fn len(&self) -> usize {
        self.files.len()
    }
}

/// Resources held in memory (e.g. an open module's edited, unsaved files).
#[derive(Debug, Default, Clone)]
pub struct MemContainer {
    files: HashMap<ResKey, Arc<[u8]>>,
}

impl MemContainer {
    pub fn new() -> MemContainer {
        MemContainer::default()
    }

    pub fn insert(&mut self, key: ResKey, data: impl Into<Arc<[u8]>>) {
        self.files.insert(key, data.into());
    }

    pub fn remove(&mut self, key: &ResKey) -> bool {
        self.files.remove(key).is_some()
    }
}

impl Container for MemContainer {
    fn contains(&self, key: &ResKey) -> bool {
        self.files.contains_key(key)
    }

    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        self.files.get(key).map(|d| Cow::Borrowed(&d[..])).ok_or(ResError::NotFound(*key))
    }

    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        Box::new(self.files.keys().copied())
    }

    fn len(&self) -> usize {
        self.files.len()
    }
}

impl ResKey {
    /// Parses `name.ext` (the last dot separates the extension).
    pub fn from_filename(name: &str) -> Option<ResKey> {
        let (stem, ext) = name.rsplit_once('.')?;
        let restype = ResType::from_extension(ext)?;
        let resref = ResRef::from_str(stem).ok().filter(|r| !r.is_empty())?;
        Some(ResKey::new(resref, restype))
    }
}
