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
        Ok((project, resources))
    }

    /// Makes a project at `root` for a module that isn't in one: a
    /// `nasher.cfg` like `nasher init --default` writes, packing `file`
    /// (kept if there is one already), and no sources yet.
    pub fn create(root: &Path, file: &str) -> Result<Project, ModuleError> {
        let cfg = Package::config_path(root);
        if !cfg.exists() {
            std::fs::create_dir_all(root).map_err(io(root))?;
            let text = format!(
                "[package]\n  [package.sources]\n  include = \"src/**/*.{{nss,json}}\"\n\n  \
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
            if (json && !is_json_type(key.restype))
                || self.sources.contains_key(&key)
                || added.iter().any(|a| a.key == key)
            {
                continue;
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
        let info_key = ResKey::new(ResRef::from_str("module").expect("valid"), ResType::IFO);
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

    /// A resource's source file contents.
    fn source_bytes(
        &self,
        key: &ResKey,
        json: bool,
        resource: &[u8],
    ) -> Result<Vec<u8>, ModuleError> {
        if !json {
            return Ok(resource.to_vec());
        }
        let gff = Gff::read(resource)
            .map_err(|e| ModuleError::Source { name: key.to_string(), message: e.to_string() })?;
        Ok(self.conversion().to_source(key.restype, &gff)?.into_bytes())
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
        for (key, bytes) in resources {
            match self.sources.get(key) {
                Some(src) if src.resource[..] == bytes[..] => {}
                Some(src) => {
                    let text = self.source_bytes(key, src.json, bytes)?;
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
                    let text = self.source_bytes(key, json, bytes)?;
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
