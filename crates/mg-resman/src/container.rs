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
    /// Reads again what the container holds on disk: a folder lists its
    /// files again, an archive is opened again (on an error it keeps what
    /// it had). Others have nothing to read.
    fn rescan(&mut self) -> Result<(), ResError> {
        Ok(())
    }
    /// A number that changes when what the container reads changes on
    /// disk (an archive written again; files added, removed or changed in
    /// a folder); 0 for containers that don't change.
    fn fingerprint(&self) -> u64 {
        0
    }
    /// The file or folder on disk the container reads, where it is one
    /// that may change ([`Container::fingerprint`] looks at it).
    fn watched(&self) -> Option<&Path> {
        None
    }
    /// Resources the container lists but the game can't read (an
    /// archive's past [`ERF_READ_LIMIT`]); reading them fails.
    fn unreadable(&self) -> Vec<ResKey> {
        Vec::new()
    }
}

/// The game reads no resource that starts this far or further into an
/// archive (a signed 32-bit offset): it finds the resource, fails to read
/// it, and doesn't look in the archives below
/// (`mg-corpus-tests/tests/engine_big_hak.rs`). One that starts before
/// the mark is read whole.
pub const ERF_READ_LIMIT: u64 = 1 << 31;

/// Hashes a file's size and modification time into `h`.
fn hash_file(h: &mut impl std::hash::Hasher, meta: &std::fs::Metadata) {
    use std::hash::Hash;
    meta.len().hash(h);
    if let Ok(t) = meta.modified() {
        t.hash(h);
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

/// An ERF archive (hak, module, texture pack) on disk. Its table is read
/// when opened (through a short-lived mapping); entries are read from the
/// file at their offsets. It isn't kept mapped: Windows refuses to rewrite
/// a mapped file, and a hak tool must be able to save a hak Moonglow has
/// open (Reload Resources then picks it up).
pub struct ErfContainer {
    pub path: PathBuf,
    file: std::fs::File,
    entries: HashMap<ResKey, Entry>,
    /// Each name once, in archive order, with its size.
    order: Vec<(ResKey, u64)>,
    /// The header, but for the entries.
    pub header: mg_erf::Header,
}

/// Reads `buf.len()` bytes at `offset` without moving a shared cursor.
fn read_exact_at(file: &std::fs::File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_exact_at(buf, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done = 0;
        while done < buf.len() {
            let n = file.seek_read(&mut buf[done..], offset + done as u64)?;
            if n == 0 {
                return Err(std::io::ErrorKind::UnexpectedEof.into());
            }
            done += n;
        }
        Ok(())
    }
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
        let file = std::fs::File::open(path).map_err(|e| ResError::io(path, e))?;
        let map = map_file(path)?;
        let erf = Erf::read(&map)
            .map_err(|e| ResError::Container { path: path.into(), message: e.to_string() })?;
        let header = mg_erf::Header {
            file_type: erf.file_type,
            build_year: erf.build_year,
            build_day: erf.build_day,
            description: erf.description.clone(),
        };
        let mut entries = HashMap::with_capacity(erf.entries.len());
        let mut order = Vec::with_capacity(erf.entries.len());
        for e in erf.entries {
            // The first of duplicate names wins, as in `Erf::find`.
            let key = ResKey::new(e.resref, e.restype);
            if let std::collections::hash_map::Entry::Vacant(v) = entries.entry(key) {
                order.push((key, u64::from(e.size)));
                v.insert(e);
            }
        }
        drop(map);
        Ok(ErfContainer { path: path.into(), file, entries, order, header })
    }

    /// Each resource once, in archive order, with its size.
    pub fn index(&self) -> &[(ResKey, u64)] {
        &self.order
    }
}

impl Container for ErfContainer {
    fn contains(&self, key: &ResKey) -> bool {
        self.entries.contains_key(key)
    }

    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        let e = self.entries.get(key).ok_or(ResError::NotFound(*key))?;
        if u64::from(e.offset) >= ERF_READ_LIMIT {
            return Err(ResError::Container {
                path: self.path.clone(),
                message: format!("{key} starts past 2 GiB, where the game stops reading"),
            });
        }
        let mut raw = vec![0; e.disk_size as usize];
        read_exact_at(&self.file, &mut raw, u64::from(e.offset))
            .map_err(|err| ResError::io(&self.path, err))?;
        // The stored bytes alone, decompressed if they are.
        let stored = Entry { offset: 0, ..e.clone() };
        mg_erf::entry_data(&raw, &stored).map(|d| Cow::Owned(d.into_owned())).map_err(|err| {
            ResError::Container { path: self.path.clone(), message: err.to_string() }
        })
    }

    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        Box::new(self.entries.keys().copied())
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn rescan(&mut self) -> Result<(), ResError> {
        *self = ErfContainer::open(&self.path)?;
        Ok(())
    }

    fn unreadable(&self) -> Vec<ResKey> {
        let mut keys: Vec<ResKey> = self
            .entries
            .iter()
            .filter(|(_, e)| u64::from(e.offset) >= ERF_READ_LIMIT)
            .map(|(k, _)| *k)
            .collect();
        keys.sort();
        keys
    }

    fn watched(&self) -> Option<&Path> {
        Some(&self.path)
    }

    fn fingerprint(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        if let Ok(meta) = std::fs::metadata(&self.path) {
            hash_file(&mut h, &meta);
        }
        std::hash::Hasher::finish(&h).max(1)
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

    fn rescan(&mut self) -> Result<(), ResError> {
        DirContainer::rescan(self);
        Ok(())
    }

    fn watched(&self) -> Option<&Path> {
        Some(&self.path)
    }

    fn fingerprint(&self) -> u64 {
        use std::hash::Hash;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        if let Ok(rd) = std::fs::read_dir(&self.path) {
            let mut files: Vec<(std::ffi::OsString, std::fs::Metadata)> =
                rd.flatten().filter_map(|e| Some((e.file_name(), e.metadata().ok()?))).collect();
            files.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, meta) in files {
                name.hash(&mut h);
                hash_file(&mut h, &meta);
            }
        }
        std::hash::Hasher::finish(&h).max(1)
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
