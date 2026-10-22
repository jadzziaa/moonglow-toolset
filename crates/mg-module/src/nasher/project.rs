//! A module opened from a nasher project: its resources read from the
//! target's source files, and saved back into them in place.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_resman::ResKey;
use mg_schema::{ExoString, StructExt, ifo};

use super::{Conversion, Package, Settings, Target, is_json_type};
use crate::ModuleError;

/// A resource's source file, as last read or written.
#[derive(Debug, Clone)]
struct Source {
    path: PathBuf,
    /// Whether the file is JSON (a GFF kept as text) rather than the
    /// resource itself.
    json: bool,
    /// The file's bytes.
    file: Arc<[u8]>,
    /// The resource's bytes they stand for.
    resource: Arc<[u8]>,
}

/// An open nasher project.
#[derive(Debug, Clone)]
pub struct Project {
    pub package: Package,
    /// The target edited (its name).
    pub target: String,
    pub settings: Settings,
    sources: HashMap<ResKey, Source>,
    /// Each source file's time and size when it was last found as it was
    /// read or written: one that still has them isn't read again
    /// ([`Project::outside`]).
    stamps: HashMap<PathBuf, Stamp>,
    /// Things worth telling the user about the tree, taken by the caller.
    pub warnings: Vec<String>,
    /// Files in the source folders that the target's include leaves out
    /// and Moonglow would read ([`Project::left_out`]), as last looked.
    pub left_out: Vec<PathBuf>,
}

/// A file's modification time and size.
type Stamp = (std::time::SystemTime, u64);

fn stamp(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// A source file changed, added or deleted outside Moonglow since it was
/// read or written.
#[derive(Debug, Clone, PartialEq)]
pub struct Outside {
    pub key: ResKey,
    pub path: PathBuf,
    /// The resource the file now stands for; `None`: the file is gone.
    pub resource: Option<Arc<[u8]>>,
    /// The file's bytes now.
    file: Option<Arc<[u8]>>,
    json: bool,
}

impl Outside {
    /// A change to a file that is its resource as it is (a module
    /// folder's).
    pub(crate) fn plain(key: ResKey, path: PathBuf, resource: Option<Arc<[u8]>>) -> Outside {
        Outside { key, path, file: resource.clone(), resource, json: false }
    }
}

/// What a save wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveReport {
    pub written: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
}

/// A module's resources, by name.
pub type Resources = IndexMap<ResKey, Arc<[u8]>>;

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> ModuleError + '_ {
    move |source| ModuleError::Io { path: path.into(), source }
}

/// The name of a resource's source file (lowercase, as nasher's unpack
/// names files).
fn source_name(key: &ResKey, json: bool) -> String {
    let name = key.to_string().to_ascii_lowercase();
    if json { format!("{name}.json") } else { name }
}

/// The resource a source file name stands for, and whether the file is
/// JSON; `None` if it isn't a resource.
/// The folder an include looks in, relative to the project (the part
/// before its first wildcard): `src/**/*.nss` looks in `src`. `None` for one
/// that looks outside the project's folder, or in all of it.
fn source_base(pattern: &str) -> Option<String> {
    let pattern = pattern.replace('\\', "/");
    let mut parts = Vec::new();
    for part in pattern.split('/') {
        if super::glob::has_magic(part) {
            break;
        }
        parts.push(part);
    }
    // (The last part without a wildcard is a file's name, unless more follow.)
    if parts.len() == pattern.split('/').count() {
        parts.pop();
    }
    let base = parts.join("/");
    let outside = base.is_empty()
        || base.starts_with('/')
        || base.split('/').any(|p| p == "..")
        || base.contains(':');
    (!outside).then_some(base)
}

/// The kinds (extensions, lower case) of `files`.
fn left_out_kinds(files: &[PathBuf]) -> Vec<String> {
    let mut kinds: Vec<String> = files
        .iter()
        .filter_map(|p| p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()))
        .collect();
    kinds.sort();
    kinds.dedup();
    kinds
}

fn resource_of(file_name: &str) -> Option<(ResKey, bool)> {
    let (name, json) = match file_name.strip_suffix(".json") {
        Some(inner) => (inner, true),
        None => (file_name, false),
    };
    crate::folder_key(name).map(|k| (k, json))
}

impl Project {
    /// Opens the project at `root`, editing `target` (by default the target
    /// that packs a module). Returns its resources.
    pub fn open(root: &Path, target: Option<&str>) -> Result<(Project, Resources), ModuleError> {
        let package = Package::read(root)?;
        let settings = Settings::read(root);
        let cfg = Package::config_path(root).display().to_string();
        let bad = |message: String| ModuleError::Source { name: cfg.clone(), message };
        if settings.gff_format != "json" {
            return Err(bad(format!(
                "the project keeps GFF files as {}; Moonglow reads JSON trees only",
                settings.gff_format
            )));
        }
        let t = match target {
            Some(name) => package.target(name).ok_or_else(|| bad(format!("no target {name:?}")))?,
            None => {
                package.module_target().ok_or_else(|| bad("no target packs a module".into()))?
            }
        }
        .clone();
        let mut project = Project {
            package,
            target: t.name.clone(),
            settings,
            sources: HashMap::new(),
            stamps: HashMap::new(),
            warnings: Vec::new(),
            left_out: Vec::new(),
        };
        let conv = project.conversion();
        let mut resources = IndexMap::new();
        for path in t.source_files(root).map_err(bad)? {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if file_name.ends_with(".nwnt") {
                return Err(bad(format!(
                    "{file_name}: NWNT files aren't supported yet; use gffFormat json"
                )));
            }
            let Some((key, json)) = resource_of(&file_name) else {
                project.warnings.push(format!("{}: not a resource; left alone", path.display()));
                continue;
            };
            if json && !is_json_type(key.restype) {
                project.warnings.push(format!(
                    "{}: only GFF files are read as JSON; left alone",
                    path.display()
                ));
                continue;
            }
            if resources.contains_key(&key) {
                project.warnings.push(format!(
                    "{}: {key} is already read from {}; this copy is left alone",
                    path.display(),
                    project.sources[&key].path.display()
                ));
                continue;
            }
            let file: Arc<[u8]> = Arc::from(std::fs::read(&path).map_err(io(&path))?);
            let resource: Arc<[u8]> = if json {
                let gff = conv.from_source(&path.display().to_string(), &file)?;
                Arc::from(gff.to_bytes().map_err(|e| ModuleError::Source {
                    name: path.display().to_string(),
                    message: e.to_string(),
                })?)
            } else {
                file.clone()
            };
            resources.insert(key, resource.clone());
            project.sources.insert(key, Source { path, json, file, resource });
        }
        project.settle_area_list(&mut resources);
        project.left_out = project.left_out();
        if !project.left_out.is_empty() {
            let warning = project.left_out_warning();
            project.warnings.push(warning);
        }
        Ok((project, resources))
    }

    /// The files in the source folders (where the target's includes look,
    /// in the project's folder) that no include takes in and no exclude
    /// leaves out on purpose, of a kind Moonglow reads: nasher won't pack
    /// them, and the project doesn't read them. A `.txt` counts only beside
    /// a NUI window of its name (its settings); other notes are left be. A
    /// builder's NUI window, moved into a folder the include didn't take
    /// in, was not found.
    pub fn left_out(&self) -> Vec<PathBuf> {
        let root = self.root().to_path_buf();
        let target = self.target();
        let mut bases: Vec<String> =
            target.includes.iter().filter_map(|p| source_base(p)).collect();
        bases.sort();
        bases.dedup();
        let mut out = Vec::new();
        for base in bases {
            for path in super::glob::walk(&root, &format!("{base}/**/*")).unwrap_or_default() {
                if out.contains(&path)
                    || target.covers(&root, &path)
                    || target.excludes.iter().any(|p| super::glob::covers(&root, p, &path))
                {
                    continue;
                }
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let Some((key, json)) = resource_of(&name) else { continue };
                if json && !is_json_type(key.restype) {
                    continue;
                }
                if key.restype == ResType::TXT && !path.with_extension("jui").is_file() {
                    continue;
                }
                out.push(path);
            }
        }
        out.sort();
        out
    }

    /// What [`Project::left_out`] found, said.
    fn left_out_warning(&self) -> String {
        let root = self.root();
        let shown: Vec<String> = (self.left_out.iter())
            .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string().replace('\\', "/"))
            .collect();
        format!(
            "{}: in the sources, but nasher.cfg's include ({}) leaves {} out: nasher won't pack \
             {}, and Moonglow doesn't read {}. Build › Add Left-Out Files to nasher.cfg takes \
             in {}.",
            shown.join(", "),
            self.target().includes.join(", "),
            if shown.len() == 1 { "it" } else { "them" },
            if shown.len() == 1 { "it" } else { "them" },
            if shown.len() == 1 { "it" } else { "them" },
            left_out_kinds(&self.left_out).join(", ")
        )
    }

    /// Takes in the files the include leaves out ([`Project::left_out`]):
    /// a line `include = "<folder>/**/*.{<their kinds>}"` is added to
    /// `nasher.cfg` beside the include of the target's they lie under, and
    /// the package is read again. They are read as files added to the
    /// tree ([`Project::outside`]). Returns the line added.
    pub fn take_in_left_out(&mut self) -> Result<Option<String>, ModuleError> {
        let left = self.left_out();
        if left.is_empty() {
            self.left_out = left;
            return Ok(None);
        }
        let root = self.root().to_path_buf();
        let cfg = Package::config_path(&root);
        let bad =
            |message: String| ModuleError::Source { name: cfg.display().to_string(), message };
        let text = std::fs::read_to_string(&cfg).map_err(io(&cfg))?;
        // The include of the target's whose folder holds the first file.
        let (pattern, base) = (self.target().includes.iter())
            .filter_map(|p| Some((p.clone(), source_base(p)?)))
            .find(|(_, base)| left[0].starts_with(root.join(base)))
            .ok_or_else(|| bad("no include of the target's looks in the files' folder".into()))?;
        let kinds = left_out_kinds(&left);
        let pick =
            if kinds.len() == 1 { kinds[0].clone() } else { format!("{{{}}}", kinds.join(",")) };
        let added = format!("include = \"{base}/**/*.{pick}\"");
        // Beside that include's line, as indented.
        let lines: Vec<&str> = text.lines().collect();
        let at = lines
            .iter()
            .position(|l| {
                let t = l.trim_start();
                t.starts_with("include") && l.contains(&format!("\"{pattern}\""))
            })
            .ok_or_else(|| bad(format!("include = \"{pattern}\" is not a line of it")))?;
        let indent: String = lines[at].chars().take_while(|c| c.is_whitespace()).collect();
        let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        out.insert(at + 1, format!("{indent}{added}"));
        let mut written = out.join("\n");
        if text.ends_with('\n') {
            written.push('\n');
        }
        std::fs::write(&cfg, written).map_err(io(&cfg))?;
        self.package = Package::read(&root)?;
        self.left_out = self.left_out();
        Ok(Some(added))
    }

    /// Makes a project at `root` for a module that isn't in one: a
    /// `nasher.cfg` like `nasher init --default` writes, packing `file`
    /// (kept if there is one already), and no sources yet. Its sources take
    /// in scripts and GFF files kept as JSON, as nasher's do, and the files
    /// of the other kinds of `extensions` (a module's 2DAs, pictures) and
    /// NUI windows (`jui`, `txt`): with nasher's alone, a window saved into
    /// the project was neither read back nor packed (a builder's report).
    pub fn create(root: &Path, file: &str, extensions: &[String]) -> Result<Project, ModuleError> {
        let cfg = Package::config_path(root);
        if !cfg.exists() {
            std::fs::create_dir_all(root).map_err(io(root))?;
            let mut kinds: Vec<String> = ["nss", "json", "jui", "txt"].map(String::from).to_vec();
            for e in extensions {
                let e = e.to_ascii_lowercase();
                if !kinds.contains(&e) {
                    kinds.push(e);
                }
            }
            let kinds = kinds.join(",");
            let text = format!(
                "[package]\n  [package.sources]\n  include = \"src/**/*.{{{kinds}}}\"\n\n  \
                 [package.rules]\n  \"*\" = \"src\"\n\n[target]\nname = \"default\"\nfile = \"{file}\"\n\
                 description = \"\"\n"
            );
            std::fs::write(&cfg, text).map_err(io(&cfg))?;
        }
        let (mut project, resources) = Project::open(root, None)?;
        if !resources.is_empty() {
            return Err(ModuleError::Source {
                name: root.display().to_string(),
                message: "the project already has a module; open it instead".into(),
            });
        }
        project.warnings.clear();
        Ok(project)
    }

    /// The target edited.
    pub fn target(&self) -> &Target {
        self.package.target(&self.target).expect("the project's target")
    }

    /// The package root.
    pub fn root(&self) -> &Path {
        &self.package.root
    }

    /// The target's packed file (`root/file`).
    pub fn target_path(&self) -> PathBuf {
        self.root().join(&self.target().file)
    }

    /// How GFF files convert in this project.
    pub fn conversion(&self) -> Conversion {
        Conversion { codepage: self.settings.codepage, float_places: self.settings.truncate_floats }
    }

    /// The source file a resource is kept in, if it has one yet.
    pub fn source_path(&self, key: &ResKey) -> Option<&Path> {
        self.sources.get(key).map(|s| s.path.as_path())
    }

    /// The source files changed on disk since they were read or written.
    pub fn changed_on_disk(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = self
            .sources
            .values()
            .filter(|s| std::fs::read(&s.path).ok().as_deref() != Some(&s.file[..]))
            .map(|s| s.path.clone())
            .collect();
        v.sort();
        v
    }

    /// The resource a source file's bytes stand for.
    fn resource_from(
        &self,
        path: &Path,
        json: bool,
        file: &Arc<[u8]>,
    ) -> Result<Arc<[u8]>, ModuleError> {
        if !json {
            return Ok(file.clone());
        }
        let name = path.display().to_string();
        let gff = self.conversion().from_source(&name, file)?;
        let bytes =
            gff.to_bytes().map_err(|e| ModuleError::Source { name, message: e.to_string() })?;
        Ok(Arc::from(bytes))
    }

    /// The source files changed, added or deleted outside Moonglow since
    /// they were read or written (by an editor, `git checkout`, `git
    /// pull`), with what could not be read of them. Each is named again
    /// every time until it is taken ([`Project::took`]) or is as it was
    /// again. Files whose time and size are as they were are not read.
    pub fn outside(&mut self) -> (Vec<Outside>, Vec<String>) {
        let (mut changes, mut problems) = (Vec::new(), Vec::new());
        let mut settled = Vec::new();
        for (key, src) in &self.sources {
            let now = stamp(&src.path);
            if now.is_some() && self.stamps.get(&src.path) == now.as_ref() {
                continue;
            }
            let gone = || Outside {
                key: *key,
                path: src.path.clone(),
                resource: None,
                file: None,
                json: src.json,
            };
            let Some(file) = std::fs::read(&src.path).ok().map(Arc::<[u8]>::from) else {
                changes.push(gone());
                continue;
            };
            if file[..] == src.file[..] {
                // (Touched, or first looked at: as it was read.)
                settled.extend(now.map(|s| (src.path.clone(), s)));
                continue;
            }
            match self.resource_from(&src.path, src.json, &file) {
                Ok(resource) => {
                    changes.push(Outside { resource: Some(resource), file: Some(file), ..gone() })
                }
                Err(e) => problems.push(e.to_string()),
            }
        }
        self.stamps.extend(settled);
        // Files that are new to the tree.
        let known: std::collections::HashSet<&Path> =
            self.sources.values().map(|s| s.path.as_path()).collect();
        let root = self.root().to_path_buf();
        let mut added: Vec<Outside> = Vec::new();
        for path in self.target().source_files(&root).unwrap_or_default() {
            if known.contains(path.as_path()) {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let Some((key, json)) = resource_of(&name) else { continue };
            // A resource whose file is gone and is found elsewhere in the
            // tree was moved there (in the project's folder, by hand): it
            // is read from there now, rather than taken for deleted (a
            // builder's NUI window, moved, was not found).
            let moved = self.sources.get(&key).is_some_and(|src| !src.path.exists());
            if (json && !is_json_type(key.restype))
                || (self.sources.contains_key(&key) && !moved)
                || added.iter().any(|a| a.key == key)
            {
                continue;
            }
            if moved {
                changes.retain(|c| c.key != key);
            }
            let Some(file) = std::fs::read(&path).ok().map(Arc::<[u8]>::from) else { continue };
            match self.resource_from(&path, json, &file) {
                Ok(resource) => added.push(Outside {
                    key,
                    path,
                    resource: Some(resource),
                    file: Some(file),
                    json,
                }),
                Err(e) => problems.push(e.to_string()),
            }
        }
        changes.extend(added);
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        problems.sort();
        // A file gone from where it was read: if it is now where the
        // include doesn't look, it was moved there, and is said
        // ([`Project::left_out`]) rather than only taken for deleted.
        let gone: Vec<ResKey> =
            changes.iter().filter(|c| c.resource.is_none()).map(|c| c.key).collect();
        if !gone.is_empty() {
            let left = self.left_out();
            let moved = left.iter().any(|p| {
                let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                resource_of(&name).is_some_and(|(k, _)| gone.contains(&k))
            });
            let new = left != self.left_out;
            self.left_out = left;
            if moved && new {
                let warning = self.left_out_warning();
                self.warnings.push(warning);
            }
        }
        (changes, problems)
    }

    /// The resource a key's source file stood for when it was last read or
    /// written (`None`: it has no file).
    pub fn as_read(&self, key: &ResKey) -> Option<&[u8]> {
        self.sources.get(key).map(|s| &s.resource[..])
    }

    /// Counts a file changed outside as read: what a save compares with
    /// from now on. (Whether the module takes its resource is the
    /// caller's: one that keeps its own will write it over the file.)
    pub fn took(&mut self, change: &Outside) {
        match (&change.file, &change.resource) {
            (Some(file), Some(resource)) => {
                self.stamps.extend(stamp(&change.path).map(|s| (change.path.clone(), s)));
                self.sources.insert(
                    change.key,
                    Source {
                        path: change.path.clone(),
                        json: change.json,
                        file: file.clone(),
                        resource: resource.clone(),
                    },
                );
            }
            _ => {
                self.stamps.remove(&change.path);
                self.sources.remove(&change.key);
            }
        }
    }

    /// Lists the areas in the tree that `module.ifo`'s area list misses
    /// (after a merge, say), by name after those listed, as nasher's pack
    /// does unless `removeUnusedAreas` is off. Unlike nasher, it keeps listed
    /// areas the tree doesn't have: they may be in a hak (Kingmaker's are).
    /// The change counts as read, so it is written only with other changes
    /// to `module.ifo`.
    fn settle_area_list(&mut self, resources: &mut Resources) {
        if !self.settings.remove_unused_areas {
            return;
        }
        let info_key = crate::Module::info_key();
        let Some(Ok(mut info)) = resources.get(&info_key).map(|b| Gff::read(b)) else { return };
        let listed: Vec<ResRef> = info
            .root
            .items(&ifo::MOD_AREA_LIST)
            .iter()
            .filter_map(|a| a.try_read(&ifo::mod_area_list::AREA_NAME))
            .collect();
        let mut added: Vec<ResRef> = resources
            .keys()
            .filter(|k| k.restype == ResType::ARE && !listed.contains(&k.resref))
            .map(|k| k.resref)
            .collect();
        if added.is_empty() {
            return;
        }
        added.sort();
        let list = info.root.items_mut(&ifo::MOD_AREA_LIST);
        for a in &added {
            let mut s = ifo::MOD_AREA_LIST.new_item();
            s.write(&ifo::mod_area_list::AREA_NAME, *a);
            list.push(s);
        }
        self.warnings.push(format!(
            "module.ifo: areas added to the area list: {}",
            added.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ")
        ));
        let Ok(bytes) = info.to_bytes() else { return };
        let bytes: Arc<[u8]> = Arc::from(bytes);
        resources.insert(info_key, bytes.clone());
        if let Some(s) = self.sources.get_mut(&info_key) {
            s.resource = bytes;
        }
    }

    /// A resource's source file contents. An area's objects' lock byte
    /// is left out ([`crate::locks`]), but where `file` (the source file
    /// as it stands) has it on the object already.
    fn source_bytes(
        &self,
        key: &ResKey,
        json: bool,
        resource: &[u8],
        file: Option<&[u8]>,
    ) -> Result<Vec<u8>, ModuleError> {
        if !json {
            return Ok(resource.to_vec());
        }
        let mut gff = Gff::read(resource)
            .map_err(|e| ModuleError::Source { name: key.to_string(), message: e.to_string() })?;
        if key.restype == ResType::GIT && crate::locks::has_locks(&gff) {
            let name = key.to_string();
            let before = file.and_then(|f| self.conversion().from_source(&name, f).ok());
            crate::locks::strip(&mut gff, before.as_ref());
        }
        Ok(self.conversion().to_source(key.restype, &gff)?.into_bytes())
    }

    /// Counts `resource` as what a source file stands for: the file's own
    /// resource with what is kept apart from it (the objects' locks, as
    /// [`crate::Module::restore_locks`] puts them back).
    pub(crate) fn stands_for(&mut self, key: &ResKey, resource: Arc<[u8]>) {
        if let Some(s) = self.sources.get_mut(key) {
            s.resource = resource;
        }
    }

    /// Writes the resources that changed into their source files, new ones
    /// where nasher's rules put them, and deletes the files of resources no
    /// longer in the module. New compiled scripts (`.ncs`) aren't written:
    /// they are build output.
    ///
    /// Nothing is written if a file to replace or delete changed on disk
    /// since it was read: the error lists those files.
    pub fn save(&mut self, resources: &Resources) -> Result<SaveReport, ModuleError> {
        enum Step {
            Write(ResKey, PathBuf, bool, Vec<u8>),
            Remove(ResKey, PathBuf),
            Adopt(ResKey, PathBuf, bool, Vec<u8>),
        }
        let root = self.root().to_path_buf();
        let disk = |p: &Path| std::fs::read(p).ok();
        let mut steps = Vec::new();
        let mut conflicts = Vec::new();
        // New files the target's sources don't take in: nasher won't pack
        // them, and the project won't read them back.
        let mut uncovered: Vec<PathBuf> = Vec::new();
        for (key, bytes) in resources {
            match self.sources.get(key) {
                Some(src) if src.resource[..] == bytes[..] => {}
                Some(src) => {
                    let text = self.source_bytes(key, src.json, bytes, Some(&src.file))?;
                    let current = disk(&src.path);
                    if text[..] == src.file[..] || current.as_deref() == Some(&text[..]) {
                        steps.push(Step::Adopt(*key, src.path.clone(), src.json, text));
                    } else if current.as_deref() != Some(&src.file[..]) {
                        conflicts.push(src.path.clone());
                    } else {
                        steps.push(Step::Write(*key, src.path.clone(), src.json, text));
                    }
                }
                None if key.restype == ResType::NCS => {}
                None => {
                    let json = is_json_type(key.restype);
                    let folder = self.target().rule_folder(&key.to_string().to_ascii_lowercase());
                    let folder = match folder.as_deref() {
                        Some("/dev/null") => continue,
                        Some(f) => f.to_string(),
                        None => "unknown".to_string(),
                    };
                    let path = root.join(folder).join(source_name(key, json));
                    if !self.target().covers(&root, &path) {
                        uncovered.push(path.clone());
                    }
                    let text = self.source_bytes(key, json, bytes, None)?;
                    match disk(&path) {
                        Some(existing) if existing == text => {
                            steps.push(Step::Adopt(*key, path, json, text))
                        }
                        Some(_) => conflicts.push(path),
                        None => steps.push(Step::Write(*key, path, json, text)),
                    }
                }
            }
        }
        let mut gone: Vec<(&ResKey, &Source)> =
            self.sources.iter().filter(|(k, _)| !resources.contains_key(*k)).collect();
        gone.sort_by(|a, b| a.1.path.cmp(&b.1.path));
        for (key, src) in gone {
            match disk(&src.path) {
                None => steps.push(Step::Remove(*key, src.path.clone())),
                Some(d) if d[..] == src.file[..] => {
                    steps.push(Step::Remove(*key, src.path.clone()))
                }
                Some(_) => conflicts.push(src.path.clone()),
            }
        }
        if !conflicts.is_empty() {
            conflicts.sort();
            return Err(ModuleError::ChangedOnDisk(conflicts));
        }
        if !uncovered.is_empty() {
            uncovered.sort();
            let shown: Vec<String> = uncovered
                .iter()
                // (With `/`, as nasher.cfg's patterns are written.)
                .map(|p| {
                    p.strip_prefix(&root).unwrap_or(p).display().to_string().replace('\\', "/")
                })
                .collect();
            let mut kinds: Vec<String> = uncovered
                .iter()
                .filter_map(|p| p.extension().map(|e| e.to_string_lossy().to_string()))
                .collect();
            kinds.sort();
            kinds.dedup();
            self.warnings.push(format!(
                "{}: written, but nasher.cfg's sources ({}) don't take them in: nasher won't \
                 pack them, and the project won't read them back when opened again. Add {} \
                 to its include.",
                shown.join(", "),
                self.target().includes.join(", "),
                kinds.join(", ")
            ));
        }
        let mut report = SaveReport::default();
        for step in steps {
            match step {
                Step::Write(key, path, json, text) => {
                    if let Some(dir) = path.parent() {
                        std::fs::create_dir_all(dir).map_err(io(dir))?;
                    }
                    let tmp = crate::sibling(&path, ".moonglow-tmp");
                    std::fs::write(&tmp, &text).map_err(io(&tmp))?;
                    std::fs::rename(&tmp, &path).map_err(io(&path))?;
                    let resource = resources[&key].clone();
                    self.sources.insert(
                        key,
                        Source { path: path.clone(), json, file: Arc::from(text), resource },
                    );
                    report.written.push(path);
                }
                Step::Adopt(key, path, json, text) => {
                    let resource = resources[&key].clone();
                    self.sources
                        .insert(key, Source { path, json, file: Arc::from(text), resource });
                }
                Step::Remove(key, path) => {
                    if path.exists() {
                        std::fs::remove_file(&path).map_err(io(&path))?;
                    }
                    self.sources.remove(&key);
                    report.removed.push(path);
                }
            }
        }
        Ok(report)
    }

    /// The resources the target's file holds: those its filters don't leave
    /// out, with its `modName`, `modDescription` and `modMinGameVersion` in
    /// `module.ifo`, as nasher packs them.
    pub fn packed(&self, resources: &Resources) -> Result<Resources, ModuleError> {
        let t = self.target();
        let mut out = Resources::new();
        for (key, bytes) in resources {
            if t.filtered(&key.to_string()) {
                continue;
            }
            let mut bytes = bytes.clone();
            if key.restype == ResType::IFO
                && (!t.mod_name.is_empty()
                    || !t.mod_description.is_empty()
                    || !t.mod_min_game_version.is_empty())
            {
                let mut info =
                    Gff::read(&bytes).map_err(|e| ModuleError::BadInfo(e.to_string()))?;
                let cp = self.settings.codepage;
                let english = |ls: &mut mg_core::LocString, text: &str| {
                    let bytes = cp
                        .encode(text)
                        .map_or_else(|| text.as_bytes().to_vec(), |b| b.into_owned());
                    ls.set(mg_core::Language::ENGLISH, mg_core::Gender::Male, bytes);
                };
                if !t.mod_name.is_empty() {
                    let mut name = info.root.read(&ifo::MOD_NAME);
                    english(&mut name, &t.mod_name);
                    info.root.write(&ifo::MOD_NAME, name);
                }
                if !t.mod_description.is_empty() {
                    let mut d = info.root.read(&ifo::MOD_DESCRIPTION);
                    english(&mut d, &t.mod_description);
                    info.root.write(&ifo::MOD_DESCRIPTION, d);
                }
                if !t.mod_min_game_version.is_empty() {
                    info.root.write(
                        &ifo::MOD_MIN_GAME_VER,
                        ExoString::from(t.mod_min_game_version.as_str()),
                    );
                }
                bytes =
                    Arc::from(info.to_bytes().map_err(|e| ModuleError::BadInfo(e.to_string()))?);
            }
            out.insert(*key, bytes);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use mg_gff::{Struct, Value};

    use super::*;
    use crate::{Module, ModuleLocation};

    fn key(name: &str, t: ResType) -> ResKey {
        ResKey::new(ResRef::from_str(name).unwrap(), t)
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mg-nasher-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A small module: module.ifo listing area `a1`, area `a1` and a script.
    fn module() -> Module {
        let mut m = Module::new();
        let mut info = Gff::new(*b"IFO ");
        let mut item = ifo::MOD_AREA_LIST.new_item();
        item.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str("a1").unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(item);
        info.root.write(&ifo::MOD_ENTRY_AREA, ResRef::from_str("a1").unwrap());
        info.root.set("Mod_ID", Value::Void(vec![1, 2, 3]));
        m.set_info(&info).unwrap();
        let mut are = Gff::new(*b"ARE ");
        are.root.set("Version", Value::Dword(7));
        are.root.set("Tag", Value::String(b"A1".to_vec()));
        m.set_gff(key("a1", ResType::ARE), &are).unwrap();
        let mut git = Gff::new(*b"GIT ");
        git.root.set("Creature List", Value::List(vec![Struct::new(4)]));
        m.set_gff(key("a1", ResType::GIT), &git).unwrap();
        m.set(key("on_load", ResType::NSS), b"void main() {}\n".to_vec());
        m.set(key("on_load", ResType::NCS), b"NCS V1.0".to_vec());
        m
    }

    fn project_location(root: &Path) -> ModuleLocation {
        ModuleLocation::Project { root: root.into(), target: "default".into() }
    }

    #[test]
    fn a_module_saved_as_a_project_reopens_the_same() {
        let root = scratch("create");
        let mut m = module();
        m.save_as(&project_location(&root)).unwrap();
        let cfg = std::fs::read_to_string(root.join("nasher.cfg")).unwrap();
        assert!(cfg.contains("file = \"mg-nasher-create-"), "{cfg}");
        let src = root.join("src");
        let mut files: Vec<String> = std::fs::read_dir(&src)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        files.sort();
        // Compiled scripts are build output.
        assert_eq!(files, ["a1.are.json", "a1.git.json", "module.ifo.json", "on_load.nss"]);
        let ifo_text = std::fs::read_to_string(src.join("module.ifo.json")).unwrap();
        assert!(!ifo_text.contains("Mod_ID"));
        assert!(!std::fs::read_to_string(src.join("a1.are.json")).unwrap().contains("Version"));

        let back = Module::open(&root).unwrap();
        assert!(back.project.as_ref().unwrap().warnings.is_empty());
        assert_eq!(back.areas().unwrap(), [ResRef::from_str("a1").unwrap()]);
        assert_eq!(back.get(&key("on_load", ResType::NSS)), m.get(&key("on_load", ResType::NSS)));
        assert!(!back.contains(&key("on_load", ResType::NCS)));
        // Saving a project into a folder that already has one is refused.
        let mut other = module();
        assert!(other.save_as(&project_location(&root)).is_err());
    }

    #[test]
    fn files_changed_added_and_deleted_outside_are_found() {
        let root = scratch("outside");
        module().save_as(&project_location(&root)).unwrap();
        let mut m = Module::open(&root).unwrap();
        let project = m.project.as_mut().unwrap();
        let nss = root.join("src/on_load.nss");
        // As it was read: nothing, however often it is asked.
        assert_eq!(project.outside(), (vec![], vec![]));
        assert_eq!(project.outside(), (vec![], vec![]));

        // Changed, added and deleted by another program.
        std::fs::write(&nss, "void main() { int outside; }\n").unwrap();
        std::fs::write(root.join("src/fresh.nss"), "void main() {}\n").unwrap();
        std::fs::remove_file(root.join("src/a1.git.json")).unwrap();
        // (Not a resource: left alone.)
        std::fs::write(root.join("src/notes.txt.bak"), "x").unwrap();
        let (changes, problems) = project.outside();
        assert!(problems.is_empty(), "{problems:?}");
        let found: Vec<(String, Option<usize>)> = changes
            .iter()
            .map(|c| (c.key.to_string(), c.resource.as_ref().map(|r| r.len())))
            .collect();
        assert_eq!(
            found,
            [
                ("a1.git".to_string(), None),
                ("fresh.nss".into(), Some(15)),
                ("on_load.nss".into(), Some(29))
            ]
        );
        // Named again until taken.
        assert_eq!(project.outside().0, changes);
        for c in &changes {
            project.took(c);
        }
        assert_eq!(project.outside(), (vec![], vec![]));
        assert_eq!(
            project.as_read(&key("on_load", ResType::NSS)),
            Some(&b"void main() { int outside; }\n"[..])
        );
        assert_eq!(project.as_read(&key("a1", ResType::GIT)), None);

        // A file put back as it was is no change; one that can't be read
        // is a problem, not a change.
        let are = root.join("src/a1.are.json");
        let was = std::fs::read(&are).unwrap();
        std::fs::write(&are, "{ not json").unwrap();
        let (changes, problems) = project.outside();
        assert!(changes.is_empty() && problems.len() == 1, "{changes:?} {problems:?}");
        std::fs::write(&are, &was).unwrap();
        assert_eq!(project.outside(), (vec![], vec![]));

        // A save after taking them writes nothing and deletes nothing.
        m.set(key("fresh", ResType::NSS), b"void main() {}\n".to_vec());
        m.set(key("on_load", ResType::NSS), b"void main() { int outside; }\n".to_vec());
        m.remove(&key("a1", ResType::GIT));
        let report = m.project.as_mut().unwrap().save(&m.resources).unwrap();
        assert_eq!(report, SaveReport::default());
    }

    /// A source file moved in the project's folder while it is open is
    /// read from where it is now, not taken for deleted (a builder's NUI
    /// window, moved, was not found).
    #[test]
    fn a_file_moved_in_the_tree_is_found_there() {
        let root = scratch("moved");
        module().save_as(&project_location(&root)).unwrap();
        let mut m = Module::open(&root).unwrap();
        let project = m.project.as_mut().unwrap();
        std::fs::create_dir_all(root.join("src/scripts")).unwrap();
        std::fs::rename(root.join("src/on_load.nss"), root.join("src/scripts/on_load.nss"))
            .unwrap();
        let (changes, problems) = project.outside();
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert_eq!(changes[0].path, root.join("src/scripts/on_load.nss"));
        assert!(changes[0].resource.is_some(), "read there, not deleted");
        project.took(&changes[0]);
        assert_eq!(project.outside(), (vec![], vec![]));
        // And it is written there.
        m.set(key("on_load", ResType::NSS), b"void main() { int moved; }\n".to_vec());
        let report = m.project.as_mut().unwrap().save(&m.resources).unwrap();
        assert_eq!(report.written, [root.join("src/scripts/on_load.nss")]);
    }

    /// A new file a save writes where the project's sources don't take it
    /// in is said: nasher wouldn't pack it, nor the project read it back.
    /// A project Moonglow makes takes in the kinds of file it is made with
    /// and NUI windows.
    #[test]
    fn a_file_the_sources_leave_out_is_said() {
        let root = scratch("uncovered");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("nasher.cfg"),
            "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
             [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
        )
        .unwrap();
        let mut m = module();
        m.set(key("win", ResType::JUI), b"{}".to_vec());
        m.save_as(&project_location(&root)).unwrap();
        let warnings = m.project.as_ref().unwrap().warnings.join("\n");
        assert!(warnings.contains("src/win.jui") && warnings.contains("Add jui"), "{warnings}");
        // Made by Moonglow: its include takes them in.
        let made = scratch("made-with-kinds");
        let mut m = module();
        m.set(key("win", ResType::JUI), b"{}".to_vec());
        m.set(key("tbl", ResType::TWODA), b"2DA V2.0\n\n   A\n0  1\n".to_vec());
        m.save_as(&project_location(&made)).unwrap();
        assert!(m.project.as_ref().unwrap().warnings.iter().all(|w| !w.contains("Add ")));
        let back = Module::open(&made).unwrap();
        assert!(
            back.contains(&key("win", ResType::JUI)) && back.contains(&key("tbl", ResType::TWODA))
        );
    }

    /// A NUI window and its settings moved into a folder the include
    /// leaves out (a builder's report: not found, after a restart too) is
    /// said on opening, and taken in by an include line added beside the
    /// project's own, then read as a file added to the tree. A note of
    /// the builder's and a file an exclude leaves out are left be.
    #[test]
    fn files_the_include_leaves_out_are_said_and_taken_in() {
        let root = scratch("left-out");
        let dir = root.join("src/scripts/chat-commands");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            root.join("nasher.cfg"),
            "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
             exclude = \"src/old/**\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
        )
        .unwrap();
        let mut m = module();
        m.save_as(&project_location(&root)).unwrap();
        std::fs::write(dir.join("test.jui"), b"{}").unwrap();
        std::fs::write(dir.join("test.txt"), b"settings").unwrap();
        std::fs::write(dir.join("readme.txt"), b"notes").unwrap();
        std::fs::create_dir_all(root.join("src/old")).unwrap();
        std::fs::write(root.join("src/old/gone.jui"), b"{}").unwrap();
        let opened = Module::open(&root).unwrap();
        let mut p = opened.project.clone().unwrap();
        let names = |files: &[PathBuf]| -> Vec<String> {
            files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect()
        };
        assert_eq!(names(&p.left_out), ["test.jui", "test.txt"]);
        assert!(!opened.contains(&key("test", ResType::JUI)));
        let warning = p.warnings.join("\n");
        assert!(
            warning.contains("src/scripts/chat-commands/test.jui") && warning.contains("jui, txt"),
            "{warning}"
        );
        let added = p.take_in_left_out().unwrap();
        assert_eq!(added.as_deref(), Some("include = \"src/**/*.{jui,txt}\""));
        let cfg = std::fs::read_to_string(root.join("nasher.cfg")).unwrap();
        assert!(
            cfg.contains(
                "  include = \"src/**/*.{nss,json}\"\n  include = \"src/**/*.{jui,txt}\"\n"
            ),
            "{cfg}"
        );
        assert!(p.left_out.is_empty());
        let (changes, problems) = p.outside();
        assert!(problems.is_empty(), "{problems:?}");
        let keys: Vec<ResKey> = changes.iter().map(|c| c.key).collect();
        assert!(
            keys.contains(&key("test", ResType::JUI)) && keys.contains(&key("test", ResType::TXT))
        );
        // And opened again: read.
        let again = Module::open(&root).unwrap();
        assert!(again.contains(&key("test", ResType::JUI)));
        assert!(again.project.unwrap().left_out.is_empty());

        // Moved, while open, to where the include doesn't look: said, not
        // only taken for deleted.
        let moved = scratch("left-out-moved");
        std::fs::create_dir_all(moved.join("src/sub")).unwrap();
        std::fs::write(
            moved.join("nasher.cfg"),
            "[package]\n  [package.sources]\n  include = \"src/*.{nss,json,jui}\"\n  \
             [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
        )
        .unwrap();
        let mut m = module();
        m.set(key("win", ResType::JUI), b"{}".to_vec());
        m.save_as(&project_location(&moved)).unwrap();
        let mut p = Module::open(&moved).unwrap().project.unwrap();
        assert!(p.left_out.is_empty());
        std::fs::rename(moved.join("src/win.jui"), moved.join("src/sub/win.jui")).unwrap();
        let _ = p.outside();
        assert_eq!(names(&p.left_out), ["win.jui"]);
        assert!(p.warnings.iter().any(|w| w.contains("src/sub/win.jui")), "{:?}", p.warnings);
    }

    #[test]
    fn saves_write_only_changes_and_refuse_to_overwrite_others() {
        let root = scratch("save");
        module().save_as(&project_location(&root)).unwrap();
        let mut m = Module::open(&root).unwrap();
        let nss = root.join("src/on_load.nss");
        let are = root.join("src/a1.are.json");

        // Untouched: nothing written.
        let report = m.project.as_mut().unwrap().save(&m.resources).unwrap();
        assert_eq!(report, SaveReport::default());

        // A change writes its file alone; a new resource goes where the rules say.
        m.set(key("on_load", ResType::NSS), b"void main() { int x; }\n".to_vec());
        m.set(key("new_one", ResType::NSS), b"void main() {}\n".to_vec());
        let report = m.project.as_mut().unwrap().save(&m.resources).unwrap();
        assert_eq!(report.written, [nss.clone(), root.join("src/new_one.nss")]);

        // Changed on disk meanwhile: nothing is written, the file is named.
        std::fs::write(&nss, "// edited elsewhere\n").unwrap();
        m.set(key("on_load", ResType::NSS), b"void main() { int y; }\n".to_vec());
        let mut are_gff = m.gff(&key("a1", ResType::ARE)).unwrap().unwrap();
        are_gff.root.set("Tag", Value::String(b"A1X".to_vec()));
        m.set_gff(key("a1", ResType::ARE), &are_gff).unwrap();
        let before_are = std::fs::read(&are).unwrap();
        match m.save() {
            Err(crate::ModuleError::ChangedOnDisk(files)) => {
                assert_eq!(files, std::slice::from_ref(&nss))
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(std::fs::read(&are).unwrap(), before_are);
        assert_eq!(m.project.as_ref().unwrap().changed_on_disk(), std::slice::from_ref(&nss));

        // A removed resource's file goes; one changed on disk stays.
        m.set(key("on_load", ResType::NSS), b"// edited elsewhere\n".to_vec());
        m.remove(&key("new_one", ResType::NSS));
        m.save().unwrap();
        assert!(!root.join("src/new_one.nss").exists());
        assert!(std::fs::read_to_string(&are).unwrap().contains("A1X"));
    }

    #[test]
    fn rules_place_new_files_and_areas_join_the_list() {
        let root = scratch("rules");
        std::fs::create_dir_all(root.join("src/areas")).unwrap();
        std::fs::write(
            root.join("nasher.cfg"),
            "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  filter = \"*.nss\"\n\
             \n  [package.rules]\n  \"*.{are,git,gic}\" = \"src/areas\"\n  \"*.ndb\" = \"/dev/null\"\n  \
             \"*.nss\" = \"src/$ext\"\n  \"*\" = \"src\"\n\n[target]\nname = \"mod\"\nfile = \"out/x.mod\"\nmodName = \"Packed Name\"\n",
        )
        .unwrap();
        let mut m = module();
        // Saving into a project without a module adds it there.
        m.save_as(&ModuleLocation::Project { root: root.clone(), target: "mod".into() }).unwrap();
        assert!(root.join("src/areas/a1.are.json").exists());
        assert!(root.join("src/nss/on_load.nss").exists());
        assert!(root.join("src/module.ifo.json").exists());

        // An area someone else added (as after a merge) joins the area list.
        let conv = m.project.as_ref().unwrap().conversion();
        let mut a2 = Gff::new(*b"ARE ");
        a2.root.set("Tag", Value::String(b"A2".to_vec()));
        std::fs::write(
            root.join("src/areas/a2.are.json"),
            conv.to_source(ResType::ARE, &a2).unwrap(),
        )
        .unwrap();
        let back = Module::open(&root).unwrap();
        let p = back.project.as_ref().unwrap();
        assert_eq!(p.warnings, ["module.ifo: areas added to the area list: a2"]);
        assert_eq!(back.areas().unwrap().len(), 2);
        // ... without a write until module.ifo changes for another reason.
        let mut back = back;
        back.save().unwrap();
        assert!(!std::fs::read_to_string(root.join("src/module.ifo.json")).unwrap().contains("a2"));

        // The packed module: scripts filtered out, the target's module name.
        let (path, bytes) = back.target_archive().unwrap().unwrap();
        assert_eq!(path, root.join("out/x.mod"));
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        assert!(erf.entries.iter().all(|e| e.restype != ResType::NSS));
        let ifo_entry = erf.entries.iter().find(|e| e.restype == ResType::IFO).unwrap();
        let info = Gff::read(&erf.data(ifo_entry).unwrap()).unwrap();
        let name = info.root.read(&ifo::MOD_NAME);
        assert_eq!(
            name.get(mg_core::Language::ENGLISH, mg_core::Gender::Male),
            Some(&b"Packed Name"[..])
        );
    }

    #[test]
    fn nwnt_trees_are_refused() {
        let root = scratch("nwnt");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("nasher.cfg"),
            "[package]\n  [package.sources]\n  include = \"src/*\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
        )
        .unwrap();
        std::fs::write(root.join("src/module.ifo.nwnt"), "x").unwrap();
        let err = Module::open(&root).unwrap_err().to_string();
        assert!(err.contains("NWNT"), "{err}");
    }
}
