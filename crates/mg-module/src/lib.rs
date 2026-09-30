//! The module workspace: a module's resources in memory, opened from and
//! saved to a `.mod`/`.nwm` archive or a module folder.
//!
//! Resources are raw bytes in their original order (new ones are appended),
//! so saving an untouched module reproduces its contents, and parsed views
//! (GFF trees, typed fields) are built on demand. Saving is atomic: the new
//! file is written next to the old one and renamed over it, after the old
//! one is kept as a single backup.

pub mod build;
pub mod dialog;
pub mod factions;
pub mod haks;
pub mod journal;
pub mod new;
pub mod refs;
pub mod script_wizard;
pub mod transfer;
pub mod verify;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use indexmap::IndexMap;
use mg_core::{ResRef, ResType};
use mg_erf::{Description, Erf, ErfWriter};
use mg_gff::Gff;
use mg_resman::{MemContainer, ResKey};
use mg_schema::{ExoString, StructExt, ifo};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{path}: {message}")]
    Archive { path: PathBuf, message: String },
    #[error("the module has no module.ifo")]
    NoInfo,
    #[error("module.ifo: {0}")]
    BadInfo(String),
    #[error("{0} is not a valid resource file name")]
    BadName(String),
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> ModuleError + '_ {
    move |source| ModuleError::Io { path: path.into(), source }
}

/// Where a module lives on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleLocation {
    /// A `.mod` (or `.nwm`) archive.
    Archive(PathBuf),
    /// A folder of loose resource files (EE's module directories).
    Folder(PathBuf),
}

impl ModuleLocation {
    pub fn path(&self) -> &Path {
        match self {
            ModuleLocation::Archive(p) | ModuleLocation::Folder(p) => p,
        }
    }
}

/// A module's resources.
#[derive(Debug, Clone)]
pub struct Module {
    pub location: Option<ModuleLocation>,
    /// Archive header data kept for archives (type tag and description).
    pub file_type: [u8; 4],
    pub description: Description,
    resources: IndexMap<ResKey, Arc<[u8]>>,
    dirty: bool,
}

impl Default for Module {
    fn default() -> Self {
        Module::new()
    }
}

impl Module {
    /// An empty, unsaved module.
    pub fn new() -> Module {
        Module {
            location: None,
            file_type: *b"MOD ",
            description: Description::default(),
            resources: IndexMap::new(),
            dirty: false,
        }
    }

    /// Opens a module archive or folder.
    pub fn open(path: &Path) -> Result<Module, ModuleError> {
        if path.is_dir() {
            return Self::open_folder(path);
        }
        let data = std::fs::read(path).map_err(io(path))?;
        let erf = Erf::read(&data)
            .map_err(|e| ModuleError::Archive { path: path.into(), message: e.to_string() })?;
        let mut m = Module::new();
        m.file_type = erf.file_type;
        m.description = erf.description.clone();
        for e in &erf.entries {
            let key = ResKey::new(e.resref, e.restype);
            // Some shipped archives list a name twice; lookups use the first.
            if m.resources.contains_key(&key) {
                continue;
            }
            let bytes = erf.data(e).map_err(|err| ModuleError::Archive {
                path: path.into(),
                message: err.to_string(),
            })?;
            m.resources.insert(key, Arc::from(bytes.as_ref()));
        }
        m.location = Some(ModuleLocation::Archive(path.into()));
        Ok(m)
    }

    fn open_folder(path: &Path) -> Result<Module, ModuleError> {
        let mut m = Module::new();
        let mut files: Vec<PathBuf> =
            std::fs::read_dir(path).map_err(io(path))?.flatten().map(|e| e.path()).collect();
        files.sort();
        for f in files.iter().filter(|f| f.is_file()) {
            let Some(key) = f.file_name().and_then(|n| n.to_str()).and_then(folder_key) else {
                continue; // not a resource (notes, editor files, ...)
            };
            if !m.resources.contains_key(&key) {
                m.resources.insert(key, Arc::from(std::fs::read(f).map_err(io(f))?));
            }
        }
        m.location = Some(ModuleLocation::Folder(path.into()));
        Ok(m)
    }

    /// Whether anything changed since opening or the last save.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn len(&self) -> usize {
        self.resources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Resource names in module order.
    pub fn keys(&self) -> impl Iterator<Item = &ResKey> {
        self.resources.keys()
    }

    /// Resource names of one type, in module order.
    pub fn keys_of(&self, restype: ResType) -> impl Iterator<Item = &ResKey> {
        self.resources.keys().filter(move |k| k.restype == restype)
    }

    pub fn get(&self, key: &ResKey) -> Option<&[u8]> {
        self.resources.get(key).map(|d| &d[..])
    }

    pub fn contains(&self, key: &ResKey) -> bool {
        self.resources.contains_key(key)
    }

    /// Adds or replaces a resource (a replaced resource keeps its position).
    pub fn set(&mut self, key: ResKey, data: impl Into<Arc<[u8]>>) {
        self.resources.insert(key, data.into());
        self.dirty = true;
    }

    pub fn remove(&mut self, key: &ResKey) -> Option<Arc<[u8]>> {
        let removed = self.resources.shift_remove(key);
        self.dirty |= removed.is_some();
        removed
    }

    /// A GFF resource, parsed.
    pub fn gff(&self, key: &ResKey) -> Option<Result<Gff, mg_gff::ReadError>> {
        self.get(key).map(Gff::read)
    }

    /// Stores a GFF resource.
    pub fn set_gff(&mut self, key: ResKey, gff: &Gff) -> Result<(), mg_gff::WriteError> {
        self.set(key, gff.to_bytes()?);
        Ok(())
    }

    fn info_key() -> ResKey {
        ResKey::new(ResRef::from_str("module").expect("valid"), ResType::IFO)
    }

    /// `module.ifo`, parsed.
    pub fn info(&self) -> Result<Gff, ModuleError> {
        self.gff(&Self::info_key())
            .ok_or(ModuleError::NoInfo)?
            .map_err(|e| ModuleError::BadInfo(e.to_string()))
    }

    pub fn set_info(&mut self, info: &Gff) -> Result<(), ModuleError> {
        self.set_gff(Self::info_key(), info).map_err(|e| ModuleError::BadInfo(e.to_string()))
    }

    /// The module's areas, in `Mod_Area_list` order.
    pub fn areas(&self) -> Result<Vec<ResRef>, ModuleError> {
        let info = self.info()?;
        Ok(info
            .root
            .items(&ifo::MOD_AREA_LIST)
            .iter()
            .filter_map(|a| a.try_read(&ifo::mod_area_list::AREA_NAME))
            .collect())
    }

    /// The module's haks, highest priority first: `Mod_HakList`, or the
    /// single pre-1.69 `Mod_Hak` field when there is no list.
    pub fn haks(&self) -> Result<Vec<String>, ModuleError> {
        let info = self.info()?;
        let text = |s: ExoString| String::from_utf8_lossy(s.as_bytes()).into_owned();
        let list: Vec<String> = info
            .root
            .items(&ifo::MOD_HAK_LIST)
            .iter()
            .map(|h| text(h.read(&ifo::mod_hak_list::MOD_HAK)))
            .filter(|h| !h.is_empty())
            .collect();
        if !list.is_empty() || info.root.contains(ifo::MOD_HAK_LIST.label) {
            return Ok(list);
        }
        Ok(info
            .root
            .try_read(&ifo::MOD_HAK)
            .map(text)
            .filter(|h| !h.is_empty())
            .into_iter()
            .collect())
    }

    /// The custom talk table's name (without `.tlk`), if any.
    pub fn custom_tlk(&self) -> Result<Option<String>, ModuleError> {
        let info = self.info()?;
        let name =
            String::from_utf8_lossy(info.root.read(&ifo::MOD_CUSTOM_TLK).as_bytes()).into_owned();
        Ok(Some(name).filter(|n| !n.is_empty()))
    }

    /// A snapshot of the resources as a resman container (cheap: the data
    /// is shared).
    pub fn container(&self) -> MemContainer {
        let mut c = MemContainer::new();
        for (k, v) in &self.resources {
            c.insert(*k, v.clone());
        }
        c
    }

    /// Saves to where the module was opened from.
    pub fn save(&mut self) -> Result<(), ModuleError> {
        let location = self.location.clone().ok_or(ModuleError::Io {
            path: PathBuf::new(),
            source: std::io::Error::other("the module has never been saved; use save_as"),
        })?;
        self.save_as(&location)
    }

    /// Saves to an archive or folder and makes it the module's location.
    pub fn save_as(&mut self, location: &ModuleLocation) -> Result<(), ModuleError> {
        match location {
            ModuleLocation::Archive(path) => self.write_archive(path)?,
            ModuleLocation::Folder(path) => self.write_folder(path)?,
        }
        self.location = Some(location.clone());
        self.dirty = false;
        Ok(())
    }

    /// The module as archive bytes. The build date is today's (or
    /// `SOURCE_DATE_EPOCH`'s, for reproducible builds).
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, ModuleError> {
        let mut w = ErfWriter::new(self.file_type);
        w.description = self.description.clone();
        let (year, day) = build_date();
        w.build_year = year;
        w.build_day = day;
        for (k, v) in &self.resources {
            w.add(k.resref, k.restype, &v[..]).map_err(|e| ModuleError::Archive {
                path: PathBuf::new(),
                message: e.to_string(),
            })?;
        }
        w.to_bytes()
            .map_err(|e| ModuleError::Archive { path: PathBuf::new(), message: e.to_string() })
    }

    fn write_archive(&self, path: &Path) -> Result<(), ModuleError> {
        let bytes = self.to_archive_bytes()?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        let tmp = sibling(path, ".moonglow-tmp");
        std::fs::write(&tmp, &bytes).map_err(io(&tmp))?;
        std::fs::File::open(&tmp).and_then(|f| f.sync_all()).map_err(io(&tmp))?;
        if path.exists() {
            let backup = sibling(path, ".bak");
            std::fs::copy(path, &backup).map_err(io(&backup))?;
        }
        std::fs::rename(&tmp, path).map_err(io(path))
    }

    fn write_folder(&self, dir: &Path) -> Result<(), ModuleError> {
        std::fs::create_dir_all(dir).map_err(io(dir))?;
        // Remove resource files that are no longer in the module; leave
        // anything that is not a resource alone.
        for entry in std::fs::read_dir(dir).map_err(io(dir))?.flatten() {
            let p = entry.path();
            if let Some(key) = p.file_name().and_then(|n| n.to_str()).and_then(folder_key)
                && p.is_file()
                && !self.resources.contains_key(&key)
            {
                std::fs::remove_file(&p).map_err(io(&p))?;
            }
        }
        for (k, v) in &self.resources {
            let p = dir.join(k.to_string());
            if std::fs::read(&p).ok().as_deref() == Some(&v[..]) {
                continue;
            }
            let tmp = sibling(&p, ".moonglow-tmp");
            std::fs::write(&tmp, &v[..]).map_err(io(&tmp))?;
            std::fs::rename(&tmp, &p).map_err(io(&p))?;
        }
        Ok(())
    }
}

/// The resource a module-folder file holds. Like [`ResKey::from_filename`],
/// but also accepts an empty name (`.res`): shipped modules contain such
/// entries, and module folders must keep them.
fn folder_key(name: &str) -> Option<ResKey> {
    if let Some(ext) = name.strip_prefix('.') {
        return ResType::from_extension(ext).map(|t| ResKey::new(ResRef::EMPTY, t));
    }
    ResKey::from_filename(name)
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// (years since 1900, day of the year) for the ERF header.
fn build_date() -> (u32, u32) {
    let secs =
        std::env::var("SOURCE_DATE_EPOCH").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or_else(
            || SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs()),
        );
    year_day(secs)
}

/// Unix seconds to (years since 1900, 0-based day of the year), UTC.
fn year_day(secs: u64) -> (u32, u32) {
    let mut days = secs / 86_400;
    let mut year = 1970u32;
    loop {
        let len =
            if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
                366
            } else {
                365
            };
        if days < len {
            break;
        }
        days -= len;
        year += 1;
    }
    (year - 1900, days as u32)
}

#[cfg(test)]
mod tests {
    use mg_gff::{Struct, Value};

    use super::*;

    fn key(n: &str, t: ResType) -> ResKey {
        ResKey::parse(n, t).unwrap()
    }

    fn sample() -> Module {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.write(&ifo::MOD_CUSTOM_TLK, ExoString::from("mytlk"));
        let area = |n: &str| {
            let mut s = ifo::MOD_AREA_LIST.new_item();
            s.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str(n).unwrap());
            s
        };
        ifo.root.items_mut(&ifo::MOD_AREA_LIST).extend([area("area001"), area("area002")]);
        let mut hak = Struct::new(8);
        hak.set("Mod_Hak", Value::String(b"top".to_vec()));
        ifo.root.items_mut(&ifo::MOD_HAK_LIST).push(hak);
        m.set_info(&ifo).unwrap();
        m.set(key("area001", ResType::ARE), &b"are"[..]);
        m.set(key("script", ResType::NSS), &b"void main() {}"[..]);
        m
    }

    #[test]
    fn module_info_accessors() {
        let m = sample();
        let areas: Vec<String> = m.areas().unwrap().iter().map(|a| a.to_string()).collect();
        assert_eq!(areas, ["area001", "area002"]);
        assert_eq!(m.haks().unwrap(), ["top"]);
        assert_eq!(m.custom_tlk().unwrap().as_deref(), Some("mytlk"));
        assert!(Module::new().info().is_err());
    }

    #[test]
    fn legacy_single_hak_field() {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.set("Mod_Hak", Value::String(b"old".to_vec()));
        m.set_info(&ifo).unwrap();
        assert_eq!(m.haks().unwrap(), ["old"]);
    }

    #[test]
    fn archive_round_trip_keeps_order_and_backs_up() {
        let dir = mg_testkit::scratch_dir("mg-module-archive");
        let path = dir.join("test.mod");
        let mut m = sample();
        m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        assert!(!m.is_dirty());
        let back = Module::open(&path).unwrap();
        assert_eq!(back.keys().collect::<Vec<_>>(), m.keys().collect::<Vec<_>>());
        assert_eq!(back.get(&key("script", ResType::NSS)), Some(&b"void main() {}"[..]));
        m.set(key("script", ResType::NSS), &b"changed"[..]);
        m.save().unwrap();
        assert!(dir.join("test.mod.bak").is_file(), "the previous version is kept");
        assert!(!dir.join("test.mod.moonglow-tmp").exists());
        assert_eq!(
            Module::open(&path).unwrap().get(&key("script", ResType::NSS)),
            Some(&b"changed"[..])
        );
    }

    #[test]
    fn folder_round_trip_removes_only_resources() {
        let dir = mg_testkit::scratch_dir("mg-module-folder");
        let mut m = sample();
        let loc = ModuleLocation::Folder(dir.join("mymod"));
        m.save_as(&loc).unwrap();
        std::fs::write(dir.join("mymod/notes.txt~"), b"keep me").unwrap();
        m.remove(&key("area001", ResType::ARE));
        m.save().unwrap();
        assert!(!dir.join("mymod/area001.are").exists());
        assert!(dir.join("mymod/notes.txt~").exists());
        let back = Module::open(&dir.join("mymod")).unwrap();
        assert_eq!(back.len(), m.len());
        assert_eq!(back.areas().unwrap().len(), 2);
    }

    #[test]
    fn folders_keep_empty_names() {
        let dir = mg_testkit::scratch_dir("mg-module-empty-name");
        let mut m = sample();
        m.set(ResKey::new(ResRef::EMPTY, ResType::RES), &b"junk"[..]);
        m.save_as(&ModuleLocation::Folder(dir.join("m"))).unwrap();
        let back = Module::open(&dir.join("m")).unwrap();
        assert_eq!(back.get(&ResKey::new(ResRef::EMPTY, ResType::RES)), Some(&b"junk"[..]));
        assert_eq!(folder_key(".hidden"), None);
    }

    #[test]
    fn build_dates() {
        assert_eq!(year_day(0), (70, 0));
        // 2026-09-30T00:00:00Z is day 272 (0-based) of 2026.
        assert_eq!(year_day(1_790_726_400), (126, 272));
        // 2024-12-31 is day 365 of a leap year.
        assert_eq!(year_day(1_735_603_200), (124, 365));
    }
}
