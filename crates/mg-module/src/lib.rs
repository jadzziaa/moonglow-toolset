//! The module workspace: a module's resources in memory, opened from and
//! saved to a `.mod`/`.nwm` archive or a module folder.
//!
//! Resources are raw bytes in their original order (new ones are appended),
//! so saving an untouched module reproduces its contents, and parsed views
//! (GFF trees, typed fields) are built on demand. Saving is atomic: the new
//! file is written next to the old one and renamed over it, after the old
//! one is kept as a single backup.

pub mod areas;
pub mod attach;
pub mod blueprints;
pub mod build;
pub mod claims;
pub mod dialog;
pub mod dialog_io;
pub mod doctor;
pub mod external_compiler;
pub mod factions;
pub mod folder;
pub mod grass;
pub mod hak_edit;
pub mod haks;
pub mod instances;
pub mod journal;
pub mod locks;
pub mod minimap;
pub mod models;
pub mod nasher;
pub mod new;
pub mod nwsync;
pub mod outside;
pub mod palette;
pub mod palette_add;
mod process;
pub mod project_file;
pub mod project_plugins;
pub mod query;
pub mod refs;
pub mod rename;
pub mod script_set;
pub mod script_wizard;
pub mod search;
pub mod stamp;
pub mod store_setup;
pub mod table_desc;
pub mod table_edit;
pub mod table_layers;
pub mod tags;
pub mod talk;
pub mod testing;
pub mod text;
pub mod tileset;
pub mod tlk_alloc;
pub mod transfer;
pub mod var_desc;
pub mod vars;
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
    #[error("{name}: {message}")]
    Source { name: String, message: String },
    #[error("changed on disk since Moonglow read them: {}", .0.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", "))]
    ChangedOnDisk(Vec<PathBuf>),
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
    /// A nasher project (its root folder) and the target edited.
    Project { root: PathBuf, target: String },
}

impl ModuleLocation {
    pub fn path(&self) -> &Path {
        match self {
            ModuleLocation::Archive(p) | ModuleLocation::Folder(p) => p,
            ModuleLocation::Project { root, .. } => root,
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
    /// The nasher project the module is kept in, when it is.
    pub project: Option<Box<nasher::Project>>,
    /// A module folder's files as last read or written, when it is one.
    pub folder: Option<Box<folder::FolderFiles>>,
    /// The project's rules (`moonglow.toml`): none for a module kept in
    /// one file, or a folder or project without the file.
    pub project_file: project_file::ProjectFile,
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
            project: None,
            folder: None,
            project_file: project_file::ProjectFile::default(),
        }
    }

    /// Reads the project file again if it changed (made, edited or
    /// deleted) since it was read: whether it did.
    pub fn reread_project_file(&mut self) -> bool {
        let Some(root) = self.location.as_ref().and_then(project_file::ProjectFile::root_of) else {
            let had = self.project_file.path.is_some();
            self.project_file = project_file::ProjectFile::default();
            return had;
        };
        let here = self.project_file.path.as_deref() == Some(&root.join(project_file::FILE_NAME));
        if here && !self.project_file.changed() {
            return false;
        }
        self.project_file = project_file::ProjectFile::read(root);
        true
    }

    /// Opens a module archive, folder, or nasher project (its folder or its
    /// `nasher.cfg`).
    pub fn open(path: &Path) -> Result<Module, ModuleError> {
        if path.file_name().is_some_and(|n| n == "nasher.cfg")
            && let Some(root) = path.parent()
        {
            return Self::open_project(root, None);
        }
        if path.is_dir() && nasher::Package::is_package(path) {
            return Self::open_project(path, None);
        }
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

    /// Opens a nasher project, editing `target` (by default the target that
    /// packs a module). The project's warnings are in
    /// `module.project.warnings`.
    pub fn open_project(root: &Path, target: Option<&str>) -> Result<Module, ModuleError> {
        let (project, resources) = nasher::Project::open(root, target)?;
        let mut m = Module::new();
        m.resources = resources;
        m.location =
            Some(ModuleLocation::Project { root: root.into(), target: project.target.clone() });
        m.project = Some(Box::new(project));
        m.reread_project_file();
        Ok(m)
    }

    fn open_folder(path: &Path) -> Result<Module, ModuleError> {
        let mut m = Module::new();
        let mut files: Vec<PathBuf> =
            std::fs::read_dir(path).map_err(io(path))?.flatten().map(|e| e.path()).collect();
        files.sort();
        // Two files of one resource (`x.UTI` as Aurora names a new one,
        // `x.uti` as Moonglow does): the one changed last is the resource.
        let mut changed = std::collections::HashMap::<ResKey, (SystemTime, String)>::new();
        let mut read = folder::FolderFiles::new(path);
        let mut twice: Vec<(ResKey, Vec<String>)> = Vec::new();
        for f in files.iter().filter(|f| f.is_file()) {
            let Some(name) = f.file_name().and_then(|n| n.to_str()) else { continue };
            let Some(key) = folder_key(name) else {
                continue; // not a resource (notes, editor files, ...)
            };
            let at = f.metadata().and_then(|d| d.modified()).unwrap_or(std::time::UNIX_EPOCH);
            if let Some((_, other)) = changed.get(&key) {
                match twice.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, names)) => names.push(name.to_string()),
                    None => twice.push((key, vec![other.clone(), name.to_string()])),
                }
            }
            if changed.get(&key).is_none_or(|(had, _)| at > *had) {
                let bytes: Arc<[u8]> = Arc::from(std::fs::read(f).map_err(io(f))?);
                read.note(key, f.clone(), bytes.clone());
                m.resources.insert(key, bytes);
                changed.insert(key, (at, name.to_string()));
            }
        }
        for (key, names) in twice {
            let taken = changed.get(&key).map(|(_, n)| n.clone()).unwrap_or_default();
            read.warnings.push(format!(
                "{key} is in the folder as {}: {taken}, changed last, is the one read, and at \
                 the next save it alone is left, as {key}",
                names.join(" and "),
            ));
        }
        m.folder = Some(Box::new(read));
        m.location = Some(ModuleLocation::Folder(path.into()));
        m.reread_project_file();
        Ok(m)
    }

    /// Takes another module's resources (a recovered copy's, say), keeping
    /// this one's location and project.
    pub fn adopt_resources(&mut self, other: Module) {
        self.resources = other.resources;
        self.dirty = true;
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

    /// A resource's bytes as the module holds them: another `Arc` once
    /// the resource is set again, so a cache can tell what changed.
    pub fn shared(&self, key: &ResKey) -> Option<&Arc<[u8]>> {
        self.resources.get(key)
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

    /// `module.ifo`'s key.
    pub fn info_key() -> ResKey {
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

    /// Moves where players start: an area, a point in it and the way they
    /// face (radians, counter-clockwise from east), as Test From Here uses.
    pub fn set_start(
        &mut self,
        area: ResRef,
        at: [f32; 3],
        facing: f32,
    ) -> Result<(), ModuleError> {
        let mut info = self.info()?;
        info.root.write(&ifo::MOD_ENTRY_AREA, area);
        info.root.write(&ifo::MOD_ENTRY_X, at[0]);
        info.root.write(&ifo::MOD_ENTRY_Y, at[1]);
        info.root.write(&ifo::MOD_ENTRY_Z, at[2]);
        info.root.write(&ifo::MOD_ENTRY_DIR_X, facing.cos());
        info.root.write(&ifo::MOD_ENTRY_DIR_Y, facing.sin());
        self.set_info(&info)
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

    /// Puts the module over a game's resources as the game has them: its
    /// haks (the user's, else the install's), then the module itself.
    /// The haks found nowhere, and what went wrong opening the others, if
    /// anything did: the module is put over what there is either way.
    pub fn stack_onto(
        &self,
        resman: &mut mg_resman::ResMan,
        install: &mg_resman::GameInstall,
    ) -> (Vec<String>, Option<mg_resman::ResError>) {
        let haks = self.haks().unwrap_or_default();
        let (missing, wrong) = match resman.add_haks(install, &haks) {
            Ok(missing) => (missing, None),
            Err(e) => (Vec::new(), Some(e)),
        };
        resman.add(
            mg_resman::priority::MODULE,
            "module",
            mg_resman::LayerClass::Erf,
            self.container(),
        );
        (missing, wrong)
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

    /// Saves to an archive, folder or nasher project and makes it the
    /// module's location. Saving into a project the module isn't in makes
    /// one there (or adds the module to a project without one).
    pub fn save_as(&mut self, location: &ModuleLocation) -> Result<(), ModuleError> {
        let mut folder = None;
        match location {
            ModuleLocation::Archive(path) => self.write_archive(path)?,
            ModuleLocation::Folder(path) => folder = Some(Box::new(self.write_folder(path)?)),
            ModuleLocation::Project { root, target } => {
                let same = self
                    .project
                    .as_ref()
                    .is_some_and(|p| p.root() == root.as_path() && &p.target == target);
                if !same {
                    let file = format!("{}.mod", self.project_file_stem(root));
                    // (The kinds of file it keeps other than scripts and
                    // GFF, for its sources to take them in.)
                    let mut kinds: Vec<String> = self
                        .keys()
                        .filter(|k| {
                            !nasher::is_json_type(k.restype)
                                && !matches!(k.restype, ResType::NSS | ResType::NCS | ResType::NDB)
                        })
                        .filter_map(|k| k.restype.extension().map(str::to_string))
                        .collect();
                    kinds.sort();
                    kinds.dedup();
                    let mut project = nasher::Project::create(root, &file, &kinds)?;
                    // A project keeps scripts as source; compiled scripts
                    // without one are lost, as with nasher.
                    let orphans: Vec<String> = self
                        .keys_of(ResType::NCS)
                        .filter(|k| !self.contains(&ResKey::new(k.resref, ResType::NSS)))
                        .map(|k| k.to_string())
                        .collect();
                    if !orphans.is_empty() {
                        project.warnings.push(format!(
                            "compiled scripts without source aren't kept in the project: {}",
                            orphans.join(", ")
                        ));
                    }
                    self.project = Some(Box::new(project));
                }
                let project = self.project.as_mut().expect("just made");
                project.save(&self.resources)?;
                let location =
                    ModuleLocation::Project { root: root.clone(), target: project.target.clone() };
                self.location = Some(location);
                self.dirty = false;
                self.reread_project_file();
                return Ok(());
            }
        }
        self.project = None;
        self.folder = folder;
        self.location = Some(location.clone());
        self.dirty = false;
        self.reread_project_file();
        Ok(())
    }

    /// The name a new project's module file takes: the module's archive or
    /// folder name, else the project folder's.
    fn project_file_stem(&self, root: &Path) -> String {
        self.location
            .as_ref()
            .and_then(|l| l.path().file_stem())
            .or_else(|| root.file_name())
            .map_or_else(|| "module".into(), |s| s.to_string_lossy().to_string())
    }

    /// For a module in a nasher project, its target's file and contents as
    /// nasher packs them (filters and module name applied).
    pub fn target_archive(&self) -> Result<Option<(PathBuf, Vec<u8>)>, ModuleError> {
        let Some(project) = &self.project else { return Ok(None) };
        let mut w = ErfWriter::new(self.file_type);
        w.description = self.description.clone();
        let (year, day) = build_date();
        w.build_year = year;
        w.build_day = day;
        let packed = project.packed(&self.resources)?;
        for (k, v) in &packed {
            w.add(k.resref, k.restype, &v[..]).map_err(|e| ModuleError::Archive {
                path: project.target_path(),
                message: e.to_string(),
            })?;
        }
        let bytes = w.to_bytes().map_err(|e| ModuleError::Archive {
            path: project.target_path(),
            message: e.to_string(),
        })?;
        Ok(Some((project.target_path(), bytes)))
    }

    /// The module as archive bytes. The build date is today's (or
    /// `SOURCE_DATE_EPOCH`'s, for reproducible builds).
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, ModuleError> {
        self.archive_writer()?
            .to_bytes()
            .map_err(|e| ModuleError::Archive { path: PathBuf::new(), message: e.to_string() })
    }

    /// The module as an archive to write, its resources borrowed.
    fn archive_writer(&self) -> Result<ErfWriter<'_>, ModuleError> {
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
        Ok(w)
    }

    /// Writes the archive beside `path`, flushed to the disk, keeps what
    /// was at `path` as `<name>.bak`, then puts the new one in its place:
    /// at no moment is there no module at `path`. The archive is written
    /// as it is made (not built whole in memory first), and the backup is
    /// a second name for the old file where the file system has them (not
    /// a copy of it: a large module's save copied a gigabyte).
    fn write_archive(&self, path: &Path) -> Result<(), ModuleError> {
        let writer = self.archive_writer()?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(io(parent))?;
        }
        let tmp = sibling(path, ".moonglow-tmp");
        // Written and flushed through one handle: flushing needs write
        // access on Windows.
        let written = std::fs::File::create(&tmp).map_err(io(&tmp)).and_then(|f| {
            let mut out = std::io::BufWriter::new(f);
            writer
                .write_to(&mut out)
                .map_err(|e| ModuleError::Archive { path: tmp.clone(), message: e.to_string() })?;
            let f = out.into_inner().map_err(|e| e.into_error()).map_err(io(&tmp))?;
            f.sync_all().map_err(io(&tmp))
        });
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        if path.exists() {
            let backup = sibling(path, ".bak");
            let _ = std::fs::remove_file(&backup);
            if std::fs::hard_link(path, &backup).is_err() {
                std::fs::copy(path, &backup).map_err(io(&backup))?;
            }
        }
        std::fs::rename(&tmp, path).map_err(io(path))
    }

    /// Writes the module into a folder, a file a resource: those that
    /// differ are written, and the files of resources no longer in the
    /// module removed (anything that is not a resource is left alone).
    ///
    /// In the folder the module was read from, what changed there since
    /// (a script saved by another editor, a file copied in) is not
    /// written over: a file changed outside whose resource is as it was
    /// read is left for [`Module::outside`] to take, and a new file is
    /// left. Nothing is written if a file to replace or delete changed
    /// outside and the module changed its resource too: the error lists
    /// those files. Returns the files as they now are.
    fn write_folder(&self, dir: &Path) -> Result<folder::FolderFiles, ModuleError> {
        std::fs::create_dir_all(dir).map_err(io(dir))?;
        let known = self.folder.as_deref().filter(|f| f.dir() == dir);
        let disk = |p: &Path| std::fs::read(p).ok();
        let listed = || -> Result<Vec<(String, ResKey)>, ModuleError> {
            Ok((std::fs::read_dir(dir).map_err(io(dir))?.flatten())
                .filter(|e| e.path().is_file())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter_map(|name| folder_key(&name).map(|key| (name, key)))
                .collect())
        };
        // What can't be done without losing something, and what is left
        // as it is on disk.
        let mut conflicts: Vec<PathBuf> = Vec::new();
        let mut left: std::collections::HashSet<ResKey> = std::collections::HashSet::new();
        // Whether a resource's file (under its own name) is already the
        // module's, where it was read here: not read again to be written.
        let mut on_disk: std::collections::HashMap<ResKey, bool> = Default::default();
        if let Some(known) = known {
            for (k, v) in &self.resources {
                match (known.as_read(k), known.path(k)) {
                    (Some(read), Some(path)) => {
                        let now = disk(path);
                        if k.path_in(dir).is_ok_and(|own| own == path) {
                            on_disk.insert(*k, now.as_deref() == Some(&v[..]));
                        }
                        if now.as_deref() == Some(read) || now.as_deref() == Some(&v[..]) {
                        } else if v[..] == read[..] {
                            left.insert(*k);
                        } else {
                            conflicts.push(path.to_path_buf());
                        }
                    }
                    _ => {
                        // (New in the module: a file of its name put there
                        // meanwhile is not written over.)
                        let own = k.path_in(dir).map_err(io(dir))?;
                        let now = disk(&own);
                        on_disk.insert(*k, now.as_deref() == Some(&v[..]));
                        if now.is_some_and(|now| now[..] != v[..]) {
                            conflicts.push(own);
                        }
                    }
                }
            }
        }
        // Resource files no longer in the module: removed, but for one
        // that is new to the folder (left), or changed since it was read.
        let mut remove = Vec::new();
        // (In a folder that isn't the module's, such files are another's:
        // not deleted, and the module is not saved in among them.)
        let mut foreign = Vec::new();
        for (name, key) in listed()? {
            if self.resources.contains_key(&key) {
                continue;
            }
            let path = dir.join(&name);
            match known {
                None => foreign.push(name),
                Some(known) if !known.knows(&key) => {}
                Some(known) if disk(&path).as_deref() == known.as_read(&key) => remove.push(path),
                Some(_) => conflicts.push(path),
            }
        }
        if !foreign.is_empty() {
            foreign.sort();
            let message = format!(
                "the folder holds resources that are not the module's ({}): save into an empty \
                 folder, or one of the module's own",
                foreign.join(", ")
            );
            return Err(ModuleError::Archive { path: dir.to_path_buf(), message });
        }
        if !conflicts.is_empty() {
            conflicts.sort();
            conflicts.dedup();
            return Err(ModuleError::ChangedOnDisk(conflicts));
        }
        for path in remove {
            std::fs::remove_file(&path).map_err(io(&path))?;
        }
        // A resource's file under another spelling (`x.UTI`, as Aurora
        // names a new one) takes the module's own (`x.uti`), so that a
        // resource is one file; where both are there (a folder that sets
        // capitals apart), the other spelling goes.
        let named = listed()?;
        let names: std::collections::HashSet<&str> =
            named.iter().map(|(n, _)| n.as_str()).collect();
        for (name, key) in &named {
            let own = key.to_string();
            if *name == own || left.contains(key) || !self.resources.contains_key(key) {
                continue;
            }
            let (from, to) = (dir.join(name), dir.join(&own));
            if names.contains(own.as_str()) {
                std::fs::remove_file(&from).map_err(io(&from))?;
            } else {
                std::fs::rename(&from, &to).map_err(io(&from))?;
            }
        }
        let mut files = folder::FolderFiles::new(dir);
        for (k, v) in &self.resources {
            if left.contains(k) {
                // (As it was read: still to be taken.)
                if let Some((known, path)) = known.and_then(|f| Some((f.as_read(k)?, f.path(k)?))) {
                    files.note_as(*k, path.to_path_buf(), Arc::from(known));
                }
                continue;
            }
            let p = k.path_in(dir).map_err(io(dir))?;
            // (A file found the module's above is so still: only another
            // name's file was renamed or removed since.)
            let same = match on_disk.get(k) {
                Some(&same) => same,
                None => disk(&p).as_deref() == Some(&v[..]),
            };
            if !same {
                let tmp = sibling(&p, ".moonglow-tmp");
                std::fs::write(&tmp, &v[..]).map_err(io(&tmp))?;
                std::fs::rename(&tmp, &p).map_err(io(&p))?;
            }
            files.note(*k, p, v.clone());
        }
        Ok(files)
    }

    /// The files of the module's folder or nasher project changed, added
    /// or deleted outside Moonglow since they were read or written, and
    /// what could not be read of them. `None`: the module is an archive
    /// (or was never saved), which nothing else edits in place.
    pub fn outside(&mut self) -> Option<(Vec<nasher::Outside>, Vec<String>)> {
        let found = self.outside_scan()?.run();
        Some(self.outside_found(found))
    }

    /// What is known of the module's files, for [`Module::outside`]'s
    /// looking to be done on another thread ([`outside::OutsideScan::run`],
    /// then [`Module::outside_found`]). `None`: an archive.
    pub fn outside_scan(&self) -> Option<outside::OutsideScan> {
        match (&self.project, &self.folder) {
            (Some(project), _) => Some(project.outside_scan()),
            (None, Some(folder)) => Some(folder.outside_scan()),
            (None, None) => None,
        }
    }

    /// Takes what [`outside::OutsideScan::run`] found: the files changed
    /// outside that still are (not those Moonglow saved or took since it
    /// looked: the next look finds them again if they still differ), and
    /// what could not be read. (Nothing, from a look at another kind of
    /// module.)
    pub fn outside_found(
        &mut self,
        found: outside::OutsideFound,
    ) -> (Vec<nasher::Outside>, Vec<String>) {
        match (&mut self.project, &mut self.folder) {
            (Some(project), _) => project.outside_found(found),
            (None, Some(folder)) => folder.outside_found(found),
            (None, None) => (Vec::new(), Vec::new()),
        }
    }

    /// Puts the locks kept apart from a nasher project's files
    /// ([`locks`]) back on the objects they find, as if the files had them:
    /// the module doesn't count as changed, and a file changed outside
    /// Moonglow is still told from Moonglow's changes. Returns how many
    /// objects were locked. Only for a project (other modules keep the
    /// lock byte in their files).
    pub fn restore_locks(&mut self, locks: &[locks::Lock]) -> usize {
        let Some(project) = self.project.as_mut() else { return 0 };
        let mut areas: Vec<ResRef> = locks.iter().map(|l| l.area).collect();
        areas.sort();
        areas.dedup();
        let mut n = 0;
        for area in areas {
            let key = ResKey::new(area, ResType::GIT);
            let Some(Ok(mut git)) = self.resources.get(&key).map(|b| Gff::read(b)) else {
                continue;
            };
            let these: Vec<&locks::Lock> = locks.iter().filter(|l| l.area == area).collect();
            let locked = locks::apply(&mut git, &these);
            if locked == 0 {
                continue;
            }
            let Ok(bytes) = git.to_bytes() else { continue };
            let bytes: Arc<[u8]> = Arc::from(bytes);
            project.stands_for(&key, bytes.clone());
            self.resources.insert(key, bytes);
            n += locked;
        }
        n
    }

    /// What a resource's file held when it was last read or written.
    pub fn as_read(&self, key: &ResKey) -> Option<&[u8]> {
        match (&self.project, &self.folder) {
            (Some(project), _) => project.as_read(key),
            (None, Some(folder)) => folder.as_read(key),
            (None, None) => None,
        }
    }

    /// Counts a file changed outside as read (see [`nasher::Project::took`]).
    pub fn took(&mut self, change: &nasher::Outside) {
        match (&mut self.project, &mut self.folder) {
            (Some(project), _) => project.took(change),
            (None, Some(folder)) => folder.took(change),
            (None, None) => {}
        }
    }

    /// The file a resource is kept in, where each has one of its own (a
    /// nasher project's source, a module folder's file).
    pub fn file_of(&self, key: &ResKey) -> Option<&Path> {
        match (&self.project, &self.folder) {
            (Some(project), _) => project.source_path(key),
            (None, Some(folder)) => folder.path(key),
            (None, None) => None,
        }
    }
}

/// The resource a module-folder file holds. Like [`ResKey::from_filename`],
/// but also accepts an empty name (`.res`): shipped modules contain such
/// entries, and module folders must keep them.
pub(crate) fn folder_key(name: &str) -> Option<ResKey> {
    if let Some(ext) = name.strip_prefix('.') {
        return ResType::from_extension(ext).map(|t| ResKey::new(ResRef::EMPTY, t));
    }
    ResKey::from_filename(name)
}

pub(crate) fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// Writes `data` to `path` whole or not at all: into a file beside it,
/// flushed to the disk, then put in its place. A crash or a full disk
/// leaves the file as it was, never cut short.
pub(crate) fn write_replacing(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = sibling(path, ".moonglow-tmp");
    // Written and flushed through one handle: flushing needs write access
    // on Windows.
    let written = std::fs::File::create(&tmp).and_then(|mut f| {
        std::io::Write::write_all(&mut f, data)?;
        f.sync_all()
    });
    match written.and_then(|()| std::fs::rename(&tmp, path)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// A GFF's root from `module` as it is now, else from the game: how a
/// blueprint is read wherever one is needed.
pub fn gff_root(
    module: Option<&Module>,
    game: &mg_rules::GameData,
    key: &ResKey,
) -> Option<mg_gff::Struct> {
    let root = |data: &[u8]| Gff::read(data).ok().map(|g| g.root);
    match module.and_then(|m| m.get(key)) {
        Some(data) => root(data),
        None => root(&game.resman.get(key).ok()?),
    }
}

/// (years since 1900, day of the year) for the ERF header: today, or
/// `SOURCE_DATE_EPOCH` for a reproducible build.
pub(crate) fn build_date() -> (u32, u32) {
    let set = std::env::var("SOURCE_DATE_EPOCH").ok().and_then(|s| s.parse::<u64>().ok());
    mg_core::time::year_day(set.unwrap_or_else(mg_core::time::unix_now))
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
        let script = |p: &Path| {
            Module::open(p).unwrap().get(&key("script", ResType::NSS)).map(<[u8]>::to_vec)
        };
        assert_eq!(script(&path).as_deref(), Some(&b"changed"[..]));
        // (The backup is the old file under a second name, where the file
        // system has them: the new one put in place leaves it as it was.)
        assert_eq!(script(&dir.join("test.mod.bak")).as_deref(), Some(&b"void main() {}"[..]));
        m.set(key("script", ResType::NSS), &b"again"[..]);
        m.save().unwrap();
        assert_eq!(script(&path).as_deref(), Some(&b"again"[..]));
        assert_eq!(script(&dir.join("test.mod.bak")).as_deref(), Some(&b"changed"[..]));
    }

    /// Saving into the module's own folder writes only the files that
    /// changed, each read once (not once to look for changes made outside
    /// and again to write it).
    #[test]
    fn a_folder_save_writes_what_changed() {
        let dir = mg_testkit::scratch_dir("mg-module-folder-changed");
        let mut m = sample();
        m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
        let file = |n: &str| dir.join(n);
        let modified = |n: &str| std::fs::metadata(file(n)).unwrap().modified().unwrap();
        let before = modified("script.nss");
        // A file of the module's under another spelling takes its own.
        let others: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n != "script.nss")
            .collect();
        let other = others.first().unwrap().clone();
        let shouted = other.to_uppercase();
        std::fs::rename(file(&other), file(&shouted)).unwrap();
        let mut m = Module::open(&dir).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        m.set(key("script", ResType::NSS), &b"changed"[..]);
        m.save().unwrap();
        assert_eq!(std::fs::read(file("script.nss")).unwrap(), b"changed");
        assert_ne!(modified("script.nss"), before);
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&other), "{names:?}");
        assert!(!names.iter().any(|n| n.ends_with(".moonglow-tmp")));
    }

    /// Save As into a folder of other resources (another module's, a
    /// builder's own files) deletes none of them, and says so.
    #[test]
    fn saving_into_another_s_folder_deletes_nothing() {
        let dir = mg_testkit::scratch_dir("mg-module-folder-foreign");
        let other = dir.join("other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("theirs.uti"), b"theirs").unwrap();
        let mut m = sample();
        let said = m.save_as(&ModuleLocation::Folder(other.clone())).unwrap_err().to_string();
        assert!(said.contains("theirs.uti"), "{said}");
        assert_eq!(std::fs::read(other.join("theirs.uti")).unwrap(), b"theirs");
        assert!(!other.join("module.ifo").exists(), "nothing written");
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

    /// A builder's folder: Aurora named new items `x.UTI`, Moonglow
    /// wrote them again as `x.uti`, and both were there.
    #[test]
    fn a_folder_s_resource_under_two_spellings_becomes_one_file() {
        let dir = mg_testkit::scratch_dir("mg-module-folder-spellings");
        let folder = dir.join("mymod");
        let mut m = sample();
        m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
        let names = |folder: &Path| {
            let mut names: Vec<String> = (std::fs::read_dir(folder).unwrap().flatten())
                .map(|e| e.file_name().into_string().unwrap())
                .filter(|n| n.to_ascii_lowercase().ends_with(".uti"))
                .collect();
            names.sort();
            names
        };
        // Alone under Aurora's spelling: read, and renamed at the save.
        std::fs::write(folder.join("Bread.UTI"), b"aurora").unwrap();
        let mut m = Module::open(&folder).unwrap();
        assert_eq!(m.get(&key("bread", ResType::UTI)), Some(&b"aurora"[..]));
        m.set(key("other", ResType::NSS), &b"x"[..]);
        m.save().unwrap();
        assert_eq!(names(&folder), ["bread.uti"]);
        assert_eq!(std::fs::read(folder.join("bread.uti")).unwrap(), b"aurora");
        // Under both (where the folder sets capitals apart): the one
        // changed last is the item, and the save leaves one file of it.
        std::fs::write(folder.join("bread.UTI"), b"newer").unwrap();
        if names(&folder).len() == 2 {
            let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
            let f = std::fs::File::options().write(true).open(folder.join("bread.UTI")).unwrap();
            f.set_modified(later).unwrap();
            let mut m = Module::open(&folder).unwrap();
            assert_eq!(m.get(&key("bread", ResType::UTI)), Some(&b"newer"[..]));
            let said = &m.folder.as_ref().unwrap().warnings;
            assert_eq!(said.len(), 1, "{said:?}");
            assert!(said[0].contains("bread.UTI, changed last, is the one read"), "{said:?}");
            m.set(key("other", ResType::NSS), &b"y"[..]);
            m.save().unwrap();
            assert_eq!(names(&folder), ["bread.uti"]);
            assert_eq!(std::fs::read(folder.join("bread.uti")).unwrap(), b"newer");
            // The older of the two is not the item, whichever its name.
            std::fs::write(folder.join("BREAD.uti"), b"older").unwrap();
            let before = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
            let f = std::fs::File::options().write(true).open(folder.join("BREAD.uti")).unwrap();
            f.set_modified(before).unwrap();
            let m = Module::open(&folder).unwrap();
            assert_eq!(m.get(&key("bread", ResType::UTI)), Some(&b"newer"[..]));
        }
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
}
