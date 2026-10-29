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
    path: Arc<Path>,
    /// Whether the file is JSON (a GFF kept as text) rather than the
    /// resource itself.
    json: bool,
    /// The file's bytes.
    file: Arc<[u8]>,
    /// The resource's bytes they stand for.
    resource: Arc<[u8]>,
    /// The file's time and size when it was last found as it was read
    /// or written: one that still has them isn't read again
    /// ([`Project::outside`]).
    stamp: Option<Stamp>,
}

/// An open nasher project.
#[derive(Debug, Clone)]
pub struct Project {
    pub package: Package,
    /// The target edited (its name).
    pub target: String,
    pub settings: Settings,
    sources: HashMap<ResKey, Source>,
    /// Things worth telling the user about the tree, taken by the caller.
    pub warnings: Vec<String>,
    /// Files in the source folders that the target's include leaves out
    /// and Moonglow would read ([`Project::left_out`]), as last looked.
    pub left_out: Vec<PathBuf>,
    /// Resources found in one folder under two spellings: the lower-case
    /// file, which the next save makes the resource's own (the other
    /// spelling's, read, written there), as nasher's pack does.
    spellings: HashMap<ResKey, PathBuf>,
}

/// Adds the line `exclude` to the sources of the target named `target` in
/// `nasher.cfg`'s lines: in its `[target.sources]` (made at the end of its
/// block where it has none). A target with no excludes of its own has the
/// package's (`had`, nasher's inheritance, list by list): written in with
/// it, so that it keeps them. `None` where no target has that name.
fn exclude_in_target(
    lines: &mut Vec<String>,
    target: &str,
    exclude: &str,
    had: &[String],
) -> Option<()> {
    let header = |l: &str| l.trim().eq_ignore_ascii_case("[target]");
    let starts: Vec<usize> = (0..lines.len()).filter(|&i| header(&lines[i])).collect();
    let named = |l: &str| {
        let t = l.trim();
        t.strip_prefix("name")
            .map(str::trim_start)
            .and_then(|r| r.strip_prefix('='))
            .is_some_and(|v| v.trim().trim_matches('"') == target)
    };
    let (start, end) = starts.iter().enumerate().find_map(|(n, &s)| {
        let end = starts.get(n + 1).copied().unwrap_or(lines.len());
        lines[s..end].iter().any(|l| named(l)).then_some((s, end))
    })?;
    let own = lines[start..end].iter().any(|l| l.trim_start().starts_with("exclude"));
    let mut add: Vec<String> =
        if own { Vec::new() } else { had.iter().map(|e| format!("exclude = \"{e}\"")).collect() };
    add.push(exclude.to_string());
    let sources = (start..end).find(|&i| lines[i].trim().eq_ignore_ascii_case("[target.sources]"));
    match sources {
        Some(s) => {
            let indent: String = (lines.get(s + 1))
                .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
                .filter(|i: &String| !i.is_empty())
                .unwrap_or_else(|| "  ".into());
            for (n, line) in add.into_iter().enumerate() {
                lines.insert(s + 1 + n, format!("{indent}{line}"));
            }
        }
        None => {
            // (After its last line that isn't blank.)
            let mut at = end;
            while at > start + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            let mut block = vec!["  [target.sources]".to_string()];
            block.extend(add.into_iter().map(|l| format!("  {l}")));
            for (n, line) in block.into_iter().enumerate() {
                lines.insert(at + n, line);
            }
        }
    }
    Some(())
}

/// Whether `key` is a compiled script whose source the module has: build
/// output, which a project doesn't keep (nasher compiles it when it
/// packs). One without its source is kept as it is, as a `.ncs` file
/// (nasher packs it, where the include takes `.ncs` files in).
fn built(key: &ResKey, resources: &Resources) -> bool {
    key.restype == ResType::NCS && resources.contains_key(&ResKey::new(key.resref, ResType::NSS))
}

/// Whether a file's name is in lower case (as nasher's pack renames
/// every source file).
fn is_lower(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    name == name.to_ascii_lowercase()
}

/// The warning for a resource in one folder under two spellings, `taken`
/// read and `lower` the lower-case file.
fn two_spellings(key: &ResKey, taken: &Path, lower: &Path) -> String {
    let name = |p: &Path| p.file_name().unwrap_or_default().to_string_lossy().to_string();
    format!(
        "{key} is in {} as {} and {}: nasher's pack renames {} over {}, so it is the one read, \
         and at the next save it alone is left, as {}",
        taken.parent().unwrap_or(Path::new("")).display(),
        name(taken),
        name(lower),
        name(taken),
        name(lower),
        name(lower)
    )
}

use crate::outside::{Known, OutsideFound, OutsideScan, Stamp, stamp};

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
    /// The file's bytes as they were known when it was looked at (`None`:
    /// a file new to the module): the change stands while they still are
    /// ([`crate::outside::stands`]).
    was: Option<Arc<[u8]>>,
    /// Its time and size when it was read (`None`: gone).
    stamp: Option<Stamp>,
}

impl Outside {
    /// A change to a file that is its resource as it is (a module
    /// folder's), found from `was`, read at `stamp`.
    pub(crate) fn plain(
        key: ResKey,
        path: PathBuf,
        resource: Option<Arc<[u8]>>,
        was: Option<Arc<[u8]>>,
        stamp: Option<Stamp>,
    ) -> Outside {
        Outside { key, path, file: resource.clone(), resource, json: false, was, stamp }
    }

    /// The time and size the file had when it was read.
    pub(crate) fn stamp(&self) -> Option<Stamp> {
        self.stamp
    }

    pub(crate) fn was(&self) -> Option<&Arc<[u8]>> {
        self.was.as_ref()
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
            warnings: Vec::new(),
            left_out: Vec::new(),
            spellings: HashMap::new(),
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
            // (A file half written when Moonglow stopped: not the
            // builder's, and gone once it is old.)
            if crate::left_writing(&path) {
                continue;
            }
            let Some((key, json)) = resource_of(&file_name) else {
                project.warnings.push(format!("{}: not a resource; left alone", path.display()));
                continue;
            };
            // (A talk table's JSON the target takes in: nasher converts it
            // and packs the table into the module, as `packed` does. A
            // target of its own that packs it too has it read with the
            // module's talk table, `nasher::talk_tables`.)
            if json && key.restype == ResType::TLK {
                continue;
            }
            if json && !is_json_type(key.restype) {
                project.warnings.push(format!(
                    "{}: only GFF files are read as JSON; left alone",
                    path.display()
                ));
                continue;
            }
            // A resource found twice. In one folder under two spellings
            // (`x.UTI.json` and `x.uti.json`), nasher's pack renames the
            // one not in lower case over the other, whichever is newer:
            // that one is read, and the next save leaves it alone, under
            // the lower-case name. In two folders nasher packs one of
            // them, in no fixed order: the first here.
            let mut twin = None;
            if let Some(had) = project.sources.get(&key) {
                let (had, here) = (had.path.to_path_buf(), path.clone());
                let one_folder = had.parent() == here.parent();
                if one_folder && is_lower(&had) && !is_lower(&here) {
                    project.warnings.push(two_spellings(&key, &here, &had));
                    twin = Some(had);
                } else if one_folder && !is_lower(&had) && is_lower(&here) {
                    project.warnings.push(two_spellings(&key, &had, &here));
                    project.spellings.insert(key, here);
                    continue;
                } else {
                    project.warnings.push(format!(
                        "{}: {key} is already read from {}; this copy is left alone (nasher \
                         packs one of the two too, in no fixed order: keep one)",
                        here.display(),
                        had.display()
                    ));
                    continue;
                }
            }
            // (Its time and size before it is read: a later look reads
            // only the files changed since.)
            let stamp = stamp(&path);
            let file: Arc<[u8]> = Arc::from(std::fs::read(&path).map_err(io(&path))?);
            // A file that can't be read (JSON broken by a merge, say) is
            // left out, and said: the rest of the project opens.
            let resource = match resource_from(conv, &path, json, &file) {
                Ok(resource) => resource,
                Err(e) => {
                    project.warnings.push(format!(
                        "{}: can't be read, and is left out of the module ({e}). Mend or \
                         delete it: a save doesn't write over it",
                        path.display()
                    ));
                    continue;
                }
            };
            if let Some(twin) = twin {
                project.spellings.insert(key, twin);
            }
            resources.insert(key, resource.clone());
            project.sources.insert(key, Source { path: path.into(), json, file, resource, stamp });
        }
        if super::kept_as(&project.settings).is_none()
            && project.package.targets.iter().any(Target::is_talk_table)
        {
            project.warnings.push(format!(
                "the project keeps its talk tables as {}: Moonglow reads JSON and CSV ones \
                 only, and leaves them alone",
                project.settings.tlk_format
            ));
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
        left_out_of(self.root(), self.target())
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
             in {} (the line {}).",
            shown.join(", "),
            self.target().includes.join(", "),
            if shown.len() == 1 { "it" } else { "them" },
            if shown.len() == 1 { "it" } else { "them" },
            if shown.len() == 1 { "it" } else { "them" },
            left_out_kinds(&self.left_out).join(", "),
            self.include_for(&self.left_out).unwrap_or_default()
        )
    }

    /// The source file a resource the project doesn't have yet is written
    /// to, where nasher's rules put it (`unknown` when none matches);
    /// `None` for one the rules discard. (A compiled script with its
    /// source is build output, never written: [`built`].)
    fn new_file(&self, key: &ResKey) -> Option<PathBuf> {
        // (A name with a folder in it gets no file: it would be written
        // outside the project.)
        if !key.resref.is_file_name() {
            return None;
        }
        let json = is_json_type(key.restype);
        let folder = self.target().rule_folder(&key.to_string().to_ascii_lowercase());
        let folder = match folder.as_deref() {
            Some("/dev/null") => return None,
            Some(f) => f.to_string(),
            None => "unknown".to_string(),
        };
        Some(self.root().join(folder).join(source_name(key, json)))
    }

    /// The files of `keys` that the target's sources would leave out: those
    /// the project doesn't have yet, written where the rules put them and
    /// not taken in by its include (nasher won't pack them, and the project
    /// won't read them back). A NUI window's files (`.jui`, `.txt`) in a
    /// project whose include names only scripts and JSON are the usual.
    pub fn uncovered(&self, keys: &[ResKey]) -> Vec<PathBuf> {
        let root = self.root();
        let mut out: Vec<PathBuf> = keys
            .iter()
            .filter(|k| !self.sources.contains_key(*k) && k.restype != ResType::NCS)
            .filter_map(|k| self.new_file(k))
            .filter(|p| !self.target().covers(root, p))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The `include` line that takes in `files` (paths in the project) with
    /// the kinds of file they are: `include = "src/**/*.{jui,txt}"`, from
    /// the folder an include of the target's looks in that holds the first
    /// of them, else from that file's own folder. `None` for no files, or
    /// files outside the project.
    pub fn include_for(&self, files: &[PathBuf]) -> Option<String> {
        let root = self.root();
        let first = files.first()?;
        let base = (self.target().includes.iter())
            .filter_map(|p| source_base(p))
            .find(|base| first.starts_with(root.join(base)))
            .or_else(|| {
                let folder = first.parent()?.strip_prefix(root).ok()?;
                let text = folder.display().to_string().replace('\\', "/");
                (!text.is_empty()).then_some(text)
            })?;
        let kinds = left_out_kinds(files);
        let pick =
            if kinds.len() == 1 { kinds[0].clone() } else { format!("{{{}}}", kinds.join(",")) };
        Some(format!("include = \"{base}/**/*.{pick}\""))
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
        let pattern = (self.target().includes.iter())
            .find(|p| source_base(p).is_some_and(|base| left[0].starts_with(root.join(base))))
            .cloned()
            .ok_or_else(|| bad("no include of the target's looks in the files' folder".into()))?;
        let added = self.include_for(&left).expect("files under an include's folder");
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
        // (In the file's own line ends: a Windows-written one's diff is
        // the line added, not every line.)
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let mut written = out.join(newline);
        if text.ends_with('\n') {
            written.push_str(newline);
        }
        let tmp = crate::sibling(&cfg, ".moonglow-tmp");
        std::fs::write(&tmp, written).map_err(io(&tmp))?;
        std::fs::rename(&tmp, &cfg).map_err(io(&cfg))?;
        self.package = Package::read(&root)?;
        self.left_out = self.left_out();
        Ok(Some(added))
    }

    /// The file a new talk table `name` is made in ([`Project::add_talk_table`]):
    /// `<name>.tlk.json` (`.csv` where the project keeps its tables so)
    /// where nasher's rules put `<name>.tlk` (`unknown` where none
    /// matches). `None` where the project keeps its tables in a form
    /// Moonglow doesn't read, or its rules discard them.
    pub fn talk_table_file(&self, name: &str) -> Option<PathBuf> {
        let format = super::kept_as(&self.settings)?;
        let ext = match format {
            mg_tlk::csv::Format::Json => "json",
            mg_tlk::csv::Format::Csv => "csv",
        };
        let folder = match self.target().rule_folder(&format!("{name}.tlk")).as_deref() {
            Some("/dev/null") => return None,
            Some(f) => f.to_string(),
            None => "unknown".to_string(),
        };
        Some(self.root().join(folder).join(format!("{name}.tlk.{ext}")))
    }

    /// Makes a new, empty talk table `name` in the project, as nasher keeps
    /// one ([`super::talk_tables`]): its file ([`Project::talk_table_file`])
    /// and a target of its own in `nasher.cfg` that packs it (`file =
    /// "<name>.tlk"`, its sources that file); the feminine table's too
    /// (`<name>f`) when asked. A module target that would take the file in
    /// is given `exclude = "**/*.tlk.json"` (`.csv`), as nasher's own
    /// example has it. Returns the files made.
    pub fn add_talk_table(
        &mut self,
        name: &str,
        language: mg_core::Language,
        feminine: bool,
    ) -> Result<Vec<PathBuf>, ModuleError> {
        let root = self.root().to_path_buf();
        let cfg = Package::config_path(&root);
        let bad =
            |message: String| ModuleError::Source { name: cfg.display().to_string(), message };
        let format = super::kept_as(&self.settings).ok_or_else(|| {
            bad(format!(
                "the project keeps its talk tables as {}: Moonglow makes JSON and CSV ones only",
                self.settings.tlk_format
            ))
        })?;
        let names: Vec<String> = std::iter::once(name.to_string())
            .chain(feminine.then(|| crate::talk::feminine(name)))
            .collect();
        let mut files = Vec::new();
        for n in &names {
            let file = self.talk_table_file(n).ok_or_else(|| {
                bad(format!("the project's rules discard {n}.tlk: no folder to make it in"))
            })?;
            if file.exists() {
                return Err(bad(format!("{} is there already", file.display())));
            }
            if self.package.targets.iter().any(|t| t.talk_table_name().as_deref() == Some(n)) {
                return Err(bad(format!("a target packs {n}.tlk already")));
            }
            files.push(file);
        }
        let text = std::fs::read_to_string(&cfg).map_err(io(&cfg))?;
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        // The module target's own sources leave the tables out, where it
        // would take them in.
        let ext = if format == mg_tlk::csv::Format::Csv { "csv" } else { "json" };
        let exclude = format!("exclude = \"**/*.tlk.{ext}\"");
        if files.iter().any(|f| self.target().covers(&root, f)) {
            let had = self.target().excludes.clone();
            exclude_in_target(&mut lines, &self.target, &exclude, &had)
                .ok_or_else(|| bad(format!("no [target] named {:?} in it", self.target)))?;
        }
        let taken: Vec<String> = self.package.targets.iter().map(|t| t.name.clone()).collect();
        for (n, file) in names.iter().zip(&files) {
            let mut target = n.clone();
            while taken.contains(&target) {
                target.push_str("_tlk");
            }
            let source = file.strip_prefix(&root).unwrap_or(file).display().to_string();
            let source = source.replace('\\', "/");
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.extend([
                "[target]".to_string(),
                format!("name = \"{target}\""),
                format!("file = \"{n}.tlk\""),
                "  [target.sources]".to_string(),
                format!("  include = \"{source}\""),
            ]);
        }
        let mut written = lines.join(newline);
        written.push_str(newline);
        // The tables first: a nasher.cfg naming files that aren't there
        // would be read as a mistake.
        let empty = mg_tlk::Tlk::new(language);
        for file in &files {
            super::write_talk_table(file, &empty, self.settings.tlk_style).map_err(|message| {
                ModuleError::Source { name: file.display().to_string(), message }
            })?;
        }
        let tmp = crate::sibling(&cfg, ".moonglow-tmp");
        std::fs::write(&tmp, written).map_err(io(&tmp))?;
        std::fs::rename(&tmp, &cfg).map_err(io(&cfg))?;
        self.package = Package::read(&root)?;
        // (A package's own excludes, which a target with none takes, could
        // leave a table's file out of its target.)
        for (n, file) in names.iter().zip(&files) {
            let t = self.package.targets.iter().find(|t| t.talk_table_name().as_deref() == Some(n));
            let found = t.and_then(|t| t.talk_table_source(&root, format).ok().flatten());
            if found.as_deref() != Some(file.as_path()) {
                self.warnings.push(format!(
                    "{}: made, but its target's sources leave it out (an exclude of the \
                     package's): mend nasher.cfg",
                    file.display()
                ));
            }
        }
        Ok(files)
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

    /// The talk tables named `name` that the project keeps as JSON, a
    /// language each ([`super::talk_tables`]).
    pub fn talk_tables(&self, name: &str) -> Result<Vec<super::TalkTable>, ModuleError> {
        super::talk_tables(&self.package, &self.settings, name)
    }

    /// How GFF files convert in this project.
    pub fn conversion(&self) -> Conversion {
        Conversion { codepage: self.settings.codepage, float_places: self.settings.truncate_floats }
    }

    /// The source file a resource is kept in, if it has one yet.
    pub fn source_path(&self, key: &ResKey) -> Option<&Path> {
        self.sources.get(key).map(|s| &*s.path)
    }

    /// The source files changed on disk since they were read or written.
    pub fn changed_on_disk(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = self
            .sources
            .values()
            .filter(|s| std::fs::read(&s.path).ok().as_deref() != Some(&s.file[..]))
            .map(|s| s.path.to_path_buf())
            .collect();
        v.sort();
        v
    }

    /// The source files changed, added or deleted outside Moonglow since
    /// they were read or written (by an editor, `git checkout`, `git
    /// pull`), with what could not be read of them. Each is named again
    /// every time until it is taken ([`Project::took`]) or is as it was
    /// again. Files whose time and size are as they were are not read.
    /// (The three steps of [`crate::outside`] at once.)
    pub fn outside(&mut self) -> (Vec<Outside>, Vec<String>) {
        let found = self.outside_scan().run();
        self.outside_found(found)
    }

    /// What is known of the source files, to look at them anywhere
    /// ([`OutsideScan::run`]).
    pub(crate) fn outside_scan(&self) -> OutsideScan {
        let files = (self.sources.iter()).map(|(key, src)| Self::known(*key, src)).collect();
        OutsideScan { files, kind: self.scan_kind(), only: None }
    }

    fn known(key: ResKey, src: &Source) -> Known {
        Known {
            key,
            path: src.path.clone(),
            json: src.json,
            file: src.file.clone(),
            stamp: src.stamp,
        }
    }

    fn scan_kind(&self) -> crate::outside::Kind {
        crate::outside::Kind::Project {
            root: self.root().to_path_buf(),
            target: Box::new(self.target().clone()),
            conversion: self.conversion(),
        }
    }

    /// [`Project::outside_scan`] of these files alone: what is known of
    /// the resources they are, by their names (a file moved is a known
    /// resource at a new path).
    pub(crate) fn outside_scan_of(&self, paths: &[PathBuf]) -> OutsideScan {
        let mut keys: Vec<ResKey> = (paths.iter())
            .filter_map(|p| resource_of(&p.file_name()?.to_string_lossy()).map(|(k, _)| k))
            .collect();
        keys.sort();
        keys.dedup();
        let files = (keys.iter())
            .filter_map(|k| self.sources.get(k).map(|src| Self::known(*k, src)))
            .collect();
        let only = Some(paths.iter().cloned().collect());
        OutsideScan { files, kind: self.scan_kind(), only }
    }

    /// Takes what [`OutsideScan::run`] found: the times of the files found
    /// as they were, the files the include leaves out; and returns the
    /// changes that still stand (not those to files Moonglow saved or
    /// took meanwhile, which the next look finds again if they still
    /// differ) and the problems.
    pub(crate) fn outside_found(&mut self, found: OutsideFound) -> (Vec<Outside>, Vec<String>) {
        if !found.project {
            return (Vec::new(), Vec::new());
        }
        for (key, was, s) in found.settled {
            if let Some(src) = self.sources.get_mut(&key)
                && Arc::ptr_eq(&src.file, &was)
            {
                src.stamp = Some(s);
            }
        }
        let mut changes = found.changes;
        changes.retain(|c| crate::outside::stands(self.sources.get(&c.key).map(|s| &s.file), c));
        // A file gone from where it was read: if it is now where the
        // include doesn't look, it was moved there, and is said
        // ([`Project::left_out`]) rather than only taken for deleted.
        if let Some((left, moved)) = found.left_out {
            let new = left != self.left_out;
            self.left_out = left;
            if moved && new {
                let warning = self.left_out_warning();
                self.warnings.push(warning);
            }
        }
        (changes, found.problems)
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
                self.sources.insert(
                    change.key,
                    Source {
                        path: change.path.as_path().into(),
                        json: change.json,
                        file: file.clone(),
                        resource: resource.clone(),
                        // (Its time when it was read, not now: a write
                        // since then is still to be found.)
                        stamp: change.stamp,
                    },
                );
            }
            _ => {
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
                // (A compiled script kept without its source has one now:
                // its file goes, below.)
                _ if built(key, resources) => {}
                Some(src) if src.resource[..] == bytes[..] => {}
                Some(src) => {
                    let text = self.source_bytes(key, src.json, bytes, Some(&src.file))?;
                    let current = disk(&src.path);
                    if text[..] == src.file[..] || current.as_deref() == Some(&text[..]) {
                        steps.push(Step::Adopt(*key, src.path.to_path_buf(), src.json, text));
                    } else if current.as_deref() != Some(&src.file[..]) {
                        conflicts.push(src.path.to_path_buf());
                    } else {
                        steps.push(Step::Write(*key, src.path.to_path_buf(), src.json, text));
                    }
                }
                None => {
                    let Some(path) = self.new_file(key) else { continue };
                    let json = is_json_type(key.restype);
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
        let mut gone: Vec<(&ResKey, &Source)> = (self.sources.iter())
            .filter(|(k, _)| !resources.contains_key(*k) || built(k, resources))
            .collect();
        gone.sort_by(|a, b| a.1.path.cmp(&b.1.path));
        for (key, src) in gone {
            match disk(&src.path) {
                None => steps.push(Step::Remove(*key, src.path.to_path_buf())),
                Some(d) if d[..] == src.file[..] => {
                    steps.push(Step::Remove(*key, src.path.to_path_buf()))
                }
                Some(_) => conflicts.push(src.path.to_path_buf()),
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
            self.warnings.push(format!(
                "{}: written, but nasher.cfg's sources ({}) don't take them in: nasher won't \
                 pack them, and the project won't read them back when opened again. Add {} \
                 to its include: the line {} (Build › Add Left-Out Files to nasher.cfg adds it).",
                shown.join(", "),
                self.target().includes.join(", "),
                left_out_kinds(&uncovered).join(", "),
                self.include_for(&uncovered).unwrap_or_default()
            ));
        }
        let wrote_uncovered = !uncovered.is_empty();
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
                        Source {
                            path: path.as_path().into(),
                            json,
                            file: Arc::from(text),
                            resource,
                            stamp: None,
                        },
                    );
                    report.written.push(path);
                }
                Step::Adopt(key, path, json, text) => {
                    let resource = resources[&key].clone();
                    self.sources.insert(
                        key,
                        Source {
                            path: path.into(),
                            json,
                            file: Arc::from(text),
                            resource,
                            stamp: None,
                        },
                    );
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
        // A resource read from one of two spellings in a folder: its file
        // is the lower-case one now, as nasher's pack leaves it, and the
        // other goes (both, for a resource the module no longer has).
        let mut spellings: Vec<(ResKey, PathBuf)> = self.spellings.drain().collect();
        spellings.sort_by(|a, b| a.1.cmp(&b.1));
        for (key, lower) in spellings {
            let Some(src) = self.sources.get(&key).filter(|s| *s.path != *lower) else {
                if !self.sources.contains_key(&key) && lower.exists() {
                    std::fs::remove_file(&lower).map_err(io(&lower))?;
                    report.removed.push(lower);
                }
                continue;
            };
            let other = src.path.to_path_buf();
            let tmp = crate::sibling(&lower, ".moonglow-tmp");
            std::fs::write(&tmp, &src.file).map_err(io(&tmp))?;
            std::fs::rename(&tmp, &lower).map_err(io(&lower))?;
            if other.exists() {
                std::fs::remove_file(&other).map_err(io(&other))?;
            }
            let src = self.sources.get_mut(&key).expect("looked up");
            src.path = lower.as_path().into();
            src.stamp = None;
            report.written.push(lower);
            report.removed.push(other);
        }
        // (Now on disk where the include doesn't look: Build › Add Left-Out
        // Files to nasher.cfg can take them in.)
        if wrote_uncovered {
            self.left_out = self.left_out();
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
        // The talk tables the target takes in as JSON (no `exclude =
        // "**/*.tlk.json"`), as nasher packs them: converted as `nwn_tlk`
        // converts them, as they are on disk when packed (the talk table
        // editor saves into them; one added since the project was opened
        // is one).
        let bad = |message: String| ModuleError::Source {
            name: Package::config_path(self.root()).display().to_string(),
            message,
        };
        for path in t.source_files(self.root()).map_err(bad)? {
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
            let Some((key, true)) = resource_of(&name) else { continue };
            if key.restype != ResType::TLK {
                continue;
            }
            if t.filtered(&key.to_string()) {
                continue;
            }
            let (_, tlk) = super::read_talk_table(&path, self.settings.tlk_style)?;
            let bytes = tlk.to_bytes_as_nwn_tlk().map_err(|e| ModuleError::Source {
                name: path.display().to_string(),
                message: e.to_string(),
            })?;
            out.insert(key, Arc::from(bytes));
        }
        Ok(out)
    }
}

/// The resource a source file's bytes stand for.
fn resource_from(
    conversion: Conversion,
    path: &Path,
    json: bool,
    file: &Arc<[u8]>,
) -> Result<Arc<[u8]>, ModuleError> {
    if !json {
        return Ok(file.clone());
    }
    let name = path.display().to_string();
    let gff = conversion.from_source(&name, file)?;
    let bytes = gff.to_bytes().map_err(|e| ModuleError::Source { name, message: e.to_string() })?;
    Ok(Arc::from(bytes))
}

/// [`OutsideScan::run`] for a project: its known source files, and the
/// target's tree for files new to it.
pub(crate) fn scan(
    files: &[Known],
    root: &Path,
    target: &Target,
    conversion: Conversion,
    only: Option<&std::collections::HashSet<PathBuf>>,
) -> OutsideFound {
    let mut found = OutsideFound { project: true, ..Default::default() };
    let mut changes = Vec::new();
    for k in files {
        if only.is_some_and(|o| !o.contains(&*k.path)) {
            continue;
        }
        // (Its time and size before it is read: a write after the read
        // gives it others, and it is read again.)
        let now = stamp(&k.path);
        // (One the system named is read whatever its time: a change within
        // the clock's grain keeps it.)
        if now.is_some() && k.stamp == now && only.is_none() {
            continue;
        }
        let gone = || Outside {
            key: k.key,
            path: k.path.to_path_buf(),
            resource: None,
            file: None,
            json: k.json,
            was: Some(k.file.clone()),
            stamp: None,
        };
        let Some(file) = std::fs::read(&k.path).ok().map(Arc::<[u8]>::from) else {
            changes.push(gone());
            continue;
        };
        if file[..] == k.file[..] {
            // (Touched, or first looked at: as it was read.)
            found.settled.extend(now.map(|s| (k.key, k.file.clone(), s)));
            continue;
        }
        match resource_from(conversion, &k.path, k.json, &file) {
            Ok(resource) => changes.push(Outside {
                resource: Some(resource),
                file: Some(file),
                stamp: now,
                ..gone()
            }),
            Err(e) => found.problems.push(e.to_string()),
        }
    }
    // Files that are new to the tree.
    let known: std::collections::HashSet<&Path> = files.iter().map(|k| &*k.path).collect();
    let by_key: HashMap<ResKey, &Known> = files.iter().map(|k| (k.key, k)).collect();
    let mut added: Vec<Outside> = Vec::new();
    // (Of the files named, those the sources take in: the tree isn't
    // walked.)
    let sources = match only {
        Some(only) => {
            let mut paths: Vec<PathBuf> =
                only.iter().filter(|p| p.is_file() && target.covers(root, p)).cloned().collect();
            paths.sort();
            paths
        }
        None => target.source_files(root).unwrap_or_default(),
    };
    for path in sources {
        if known.contains(path.as_path()) {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let Some((key, json)) = resource_of(&name) else { continue };
        // A resource whose file is gone and is found elsewhere in the
        // tree was moved there (in the project's folder, by hand): it
        // is read from there now, rather than taken for deleted (a
        // builder's NUI window, moved, was not found).
        let was = by_key.get(&key);
        let moved = was.is_some_and(|k| !k.path.exists());
        if (json && !is_json_type(key.restype))
            || (was.is_some() && !moved)
            || added.iter().any(|a| a.key == key)
        {
            continue;
        }
        if moved {
            changes.retain(|c| c.key != key);
        }
        let now = stamp(&path);
        let Some(file) = std::fs::read(&path).ok().map(Arc::<[u8]>::from) else { continue };
        match resource_from(conversion, &path, json, &file) {
            Ok(resource) => added.push(Outside {
                key,
                path,
                resource: Some(resource),
                file: Some(file),
                json,
                was: was.map(|k| k.file.clone()),
                stamp: now,
            }),
            Err(e) => found.problems.push(e.to_string()),
        }
    }
    changes.extend(added);
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    found.problems.sort();
    let gone: Vec<ResKey> =
        changes.iter().filter(|c| c.resource.is_none()).map(|c| c.key).collect();
    if !gone.is_empty() {
        let left = left_out_of(root, target);
        let moved = left.iter().any(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            resource_of(&name).is_some_and(|(k, _)| gone.contains(&k))
        });
        found.left_out = Some((left, moved));
    }
    found.changes = changes;
    found
}

/// [`Project::left_out`] of a project at `root` editing `target`.
fn left_out_of(root: &Path, target: &Target) -> Vec<PathBuf> {
    let mut bases: Vec<String> = target.includes.iter().filter_map(|p| source_base(p)).collect();
    bases.sort();
    bases.dedup();
    let mut out = Vec::new();
    for base in bases {
        for path in super::glob::walk(root, &format!("{base}/**/*")).unwrap_or_default() {
            if out.contains(&path)
                || target.covers(root, &path)
                || target.excludes.iter().any(|p| super::glob::covers(root, p, &path))
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

    /// The looking done apart (on another thread, `crate::outside`): what
    /// Moonglow took while it looked, and a file found deleted that is back
    /// since, are left out of what it hands back; the next look finds what
    /// still differs. A file read and written again before it is taken is
    /// found again.
    #[test]
    fn a_look_apart_hands_back_only_what_still_stands() {
        let root = scratch("outside-apart");
        module().save_as(&project_location(&root)).unwrap();
        let mut m = Module::open(&root).unwrap();
        let (nss, git) = (root.join("src/on_load.nss"), root.join("src/a1.git.json"));
        let (script, fresh) = (key("on_load", ResType::NSS), key("fresh", ResType::NSS));
        // Opened: every file's time is known, so a look reads none.
        let scan = m.outside_scan().unwrap();
        assert!(!scan.is_empty() && scan.files.iter().all(|f| f.stamp.is_some()));
        assert_eq!(m.outside_found(scan.run()), (vec![], vec![]));

        // Changed, deleted and added outside, and looked at apart...
        let area = std::fs::read(&git).unwrap();
        std::fs::write(&nss, "void main() { int one; }\n").unwrap();
        std::fs::remove_file(&git).unwrap();
        std::fs::write(root.join("src/fresh.nss"), "void main() {}\n").unwrap();
        let found = m.outside_scan().unwrap().run();
        assert_eq!(found.changes().len(), 3);
        // ...while the script is taken here and the area's file put back:
        // only the new file is handed back.
        let (now, _) = m.outside().unwrap();
        m.took(now.iter().find(|c| c.key == script).unwrap());
        std::fs::write(&git, &area).unwrap();
        let (changes, _) = m.outside_found(found);
        assert_eq!(changes.iter().map(|c| c.key).collect::<Vec<_>>(), [fresh]);
        m.took(&changes[0]);
        assert_eq!(m.outside(), Some((vec![], vec![])));

        // Read, then written again before it is taken: still to be found.
        std::fs::write(&nss, "void main() { int three; }\n").unwrap();
        let (changes, _) = m.outside().unwrap();
        let read = changes.iter().find(|c| c.key == script).unwrap().clone();
        std::fs::write(&nss, "void main() { int four, more; }\n").unwrap();
        m.took(&read);
        let (changes, _) = m.outside().unwrap();
        let again = changes.iter().find(|c| c.key == script).unwrap();
        assert_eq!(again.resource.as_deref(), Some(&b"void main() { int four, more; }\n"[..]));
    }

    /// Where the system named the files that changed (a watch), only they
    /// are looked at: a file changed and not named isn't found (until a
    /// look at all), one named is, as are one added and one deleted; one
    /// named that the sources don't take in, or moved to where they do,
    /// is as a whole look finds it.
    #[test]
    fn a_look_at_the_files_named_looks_at_them_alone() {
        let root = scratch("outside-named");
        module().save_as(&project_location(&root)).unwrap();
        let mut m = Module::open(&root).unwrap();
        let nss = root.join("src/on_load.nss");
        let are = root.join("src/a1.are.json");
        let look = |m: &mut Module, paths: &[PathBuf]| {
            let found = m.outside_scan_of(paths).unwrap().run();
            let (changes, problems) = m.outside_found(found);
            assert!(problems.is_empty(), "{problems:?}");
            let mut names: Vec<String> = changes.iter().map(|c| c.key.to_string()).collect();
            names.sort();
            names
        };
        std::fs::write(&nss, "void main() { int outside; }\n").unwrap();
        let was = std::fs::read(&are).unwrap();
        std::fs::write(&are, String::from_utf8(was).unwrap().replace('{', "{ ")).unwrap();
        std::fs::write(root.join("src/fresh.nss"), "void main() {}\n").unwrap();
        std::fs::remove_file(root.join("src/a1.git.json")).unwrap();
        std::fs::write(root.join("notes.nss"), "void main() {}\n").unwrap();
        let named = [
            nss.clone(),
            root.join("src/fresh.nss"),
            root.join("src/a1.git.json"),
            root.join("notes.nss"),
        ];
        assert_eq!(look(&mut m, &named), ["a1.git", "fresh.nss", "on_load.nss"]);
        // (The area's file changed too, and wasn't named: a whole look
        // finds it.)
        let (all, _) = m.outside().unwrap();
        let all: Vec<String> = all.iter().map(|c| c.key.to_string()).collect();
        assert!(all.contains(&"a1.are".to_string()), "{all:?}");
        // A file moved where the sources take it in: read from there.
        let mut m = Module::open(&root).unwrap();
        std::fs::create_dir_all(root.join("src/scripts")).unwrap();
        std::fs::rename(&nss, root.join("src/scripts/on_load.nss")).unwrap();
        let named = [nss.clone(), root.join("src/scripts/on_load.nss")];
        let found = m.outside_scan_of(&named).unwrap().run();
        let (changes, _) = m.outside_found(found);
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert!(changes[0].path.ends_with("src/scripts/on_load.nss") && changes[0].was().is_some());
        // A change that keeps the file's time and size (within the clock's
        // grain): a whole look passes over it, one that names it doesn't.
        let mut m = Module::open(&root).unwrap();
        let file = root.join("src/scripts/on_load.nss");
        let when = std::fs::metadata(&file).unwrap().modified().unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        let same_length = text.replace("theirs", "THEIRS").replace("outside", "OUTSIDE");
        assert_eq!(same_length.len(), text.len());
        assert_ne!(same_length, text);
        std::fs::write(&file, &same_length).unwrap();
        std::fs::File::options().write(true).open(&file).unwrap().set_modified(when).unwrap();
        assert!(m.outside().unwrap().0.is_empty(), "the time and size are as they were");
        let found = m.outside_scan_of(std::slice::from_ref(&file)).unwrap().run();
        let (changes, _) = m.outside_found(found);
        assert_eq!(changes.len(), 1, "{changes:?}");
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
        // (Written on Windows: CRLF, which the line added keeps.)
        std::fs::write(
            root.join("nasher.cfg"),
            "[package]\r\n  [package.sources]\r\n  include = \"src/**/*.{nss,json}\"\r\n  \
             exclude = \"src/old/**\"\r\n[target]\r\nname = \"m\"\r\nfile = \"m.mod\"\r\n",
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
                "  include = \"src/**/*.{nss,json}\"\r\n  include = \"src/**/*.{jui,txt}\"\r\n"
            ),
            "{cfg}"
        );
        assert_eq!(cfg.matches('\n').count(), cfg.matches("\r\n").count(), "CRLF kept: {cfg}");
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

    /// A project whose sources are scripts and JSON (`src/**/*.{nss,json}`,
    /// as `nasher init` writes and The Domains of Dread had): a NUI window's
    /// files would be written where the rules put them and left out. Said
    /// before (for `mg nui new` and the Creator) and on saving, with the
    /// line that takes them in; Add Left-Out Files can then add it.
    #[test]
    fn a_window_s_files_the_include_would_leave_out_are_said_with_the_line() {
        let root = scratch("uncovered-window");
        std::fs::write(
            root.join("nasher.cfg"),
            "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
             [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
        )
        .unwrap();
        let mut m = module();
        m.save_as(&project_location(&root)).unwrap();
        let mut m = Module::open(&root).unwrap();
        let window = [key("crier", ResType::JUI), key("crier", ResType::TXT)];
        let p = m.project.as_ref().unwrap();
        // A script is taken in; a compiled one is never written.
        let asked = [window[0], window[1], key("crier_e", ResType::NSS), key("x", ResType::NCS)];
        let left = p.uncovered(&asked);
        assert_eq!(left, [root.join("src/crier.jui"), root.join("src/crier.txt")]);
        assert_eq!(p.include_for(&left).as_deref(), Some("include = \"src/**/*.{jui,txt}\""));
        assert_eq!(p.include_for(&left[..1]).as_deref(), Some("include = \"src/**/*.jui\""));
        assert_eq!(p.include_for(&[]), None);
        // Saved: written, said with the line, and left out for Build ›
        // Add Left-Out Files to take in.
        m.set(window[0], b"{}".to_vec());
        m.set(window[1], b"{}".to_vec());
        m.save().unwrap();
        let p = m.project.as_mut().unwrap();
        let said = p.warnings.join("\n");
        assert!(said.contains("the line include = \"src/**/*.{jui,txt}\""), "{said}");
        assert_eq!(p.left_out, left);
        assert_eq!(
            p.take_in_left_out().unwrap().as_deref(),
            Some("include = \"src/**/*.{jui,txt}\"")
        );
        assert!(p.uncovered(&window).is_empty() && p.left_out.is_empty());
        assert!(Module::open(&root).unwrap().contains(&window[0]));
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

    /// A compiled script without its source (a module's, from before its
    /// builder kept sources) is kept when the module becomes a project, as
    /// it is: the project's include takes `.ncs` files in (nasher packs
    /// such a file into the module, measured with nasher 1.1.3). Once its
    /// source is in the module the compiled script is build output, and
    /// its file goes.
    #[test]
    fn a_compiled_script_without_source_is_kept() {
        let root = scratch("orphan-ncs");
        let mut m = module();
        m.set(key("old_one", ResType::NCS), b"NCS V1.0 old".to_vec());
        m.save_as(&project_location(&root)).unwrap();
        let said = m.project.as_ref().unwrap().warnings.join("\n");
        assert!(said.contains("kept in the project as they are") && said.contains("old_one"));
        let cfg = std::fs::read_to_string(root.join("nasher.cfg")).unwrap();
        assert!(cfg.contains("include = \"src/**/*.{") && cfg.contains(",ncs"), "{cfg}");
        let file = root.join("src/old_one.ncs");
        assert_eq!(std::fs::read(&file).unwrap(), b"NCS V1.0 old");
        assert!(!root.join("src/on_load.ncs").exists(), "one with its source is build output");
        let mut m = Module::open(&root).unwrap();
        assert_eq!(m.get(&key("old_one", ResType::NCS)), Some(&b"NCS V1.0 old"[..]));
        assert!(m.project.as_ref().unwrap().warnings.is_empty());
        assert!(m.project.as_ref().unwrap().left_out.is_empty());
        // Its source written: the compiled script is build output now.
        m.set(key("old_one", ResType::NSS), b"void main() {}\n".to_vec());
        m.save().unwrap();
        assert!(!file.exists() && root.join("src/old_one.nss").exists());
        assert!(m.contains(&key("old_one", ResType::NCS)), "still the module's, until packed");
        m.save().unwrap();
        assert!(!file.exists());
        let m = Module::open(&root).unwrap();
        assert!(!m.contains(&key("old_one", ResType::NCS)));
    }

    /// A module target that takes a talk table's JSON in (no `exclude =
    /// "**/*.tlk.json"`): the module packed holds the table, as nasher
    /// packs it (`nwn_tlk`'s conversion), and the JSON is no warning.
    #[test]
    fn a_talk_table_the_module_target_takes_in_is_packed() {
        let root = scratch("tlk-in-module");
        module().save_as(&project_location(&root)).unwrap();
        let json = r#"{"language":0,"entries":[{"id":0,"text":"Hello"},{"id":2,"text":"","sound":"vs_x"}]}"#;
        std::fs::create_dir_all(root.join("src/tlk")).unwrap();
        std::fs::write(root.join("src/tlk/mytlk.tlk.json"), json).unwrap();
        let m = Module::open(&root).unwrap();
        let said = &m.project.as_ref().unwrap().warnings;
        assert!(said.is_empty(), "{said:?}");
        assert!(!m.contains(&key("mytlk", ResType::TLK)), "the table, not a resource to edit");
        let (_, bytes) = m.target_archive().unwrap().unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        let entry = erf.entries.iter().find(|e| e.restype == ResType::TLK).expect("packed");
        assert_eq!(entry.resref.to_string(), "mytlk");
        let tlk =
            mg_tlk::json::from_json(json.as_bytes(), mg_core::Codepage::WINDOWS_1252).unwrap();
        assert_eq!(&erf.data(entry).unwrap()[..], &tlk.to_bytes_as_nwn_tlk().unwrap()[..]);
        // One added while the project is open: packed too, as it is then.
        std::fs::write(root.join("src/tlk/later.tlk.json"), json).unwrap();
        let (_, bytes) = m.target_archive().unwrap().unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        let mut tables: Vec<String> = (erf.entries.iter())
            .filter(|e| e.restype == ResType::TLK)
            .map(|e| e.resref.to_string())
            .collect();
        tables.sort();
        assert_eq!(tables, ["later", "mytlk"]);
        std::fs::remove_file(root.join("src/tlk/later.tlk.json")).unwrap();
        // Left out by the target's exclude: not packed.
        let cfg = std::fs::read_to_string(root.join("nasher.cfg")).unwrap();
        let cfg = cfg.replacen(
            "  [package.rules]",
            "  exclude = \"**/*.tlk.json\"\n  [package.rules]",
            1,
        );
        std::fs::write(root.join("nasher.cfg"), cfg).unwrap();
        let m = Module::open(&root).unwrap();
        let (_, bytes) = m.target_archive().unwrap().unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        assert!(erf.entries.iter().all(|e| e.restype != ResType::TLK));
    }

    /// A new talk table made in a project goes into it as nasher keeps one:
    /// `<name>.tlk.json` where the rules put `<name>.tlk`, a target of its
    /// own packing it, and the module target's sources leaving it out (the
    /// package's excludes kept with it); the feminine table's too.
    #[test]
    fn a_new_talk_table_goes_into_the_project() {
        let root = scratch("new-tlk");
        module().save_as(&project_location(&root)).unwrap();
        // (A package's exclude, which the module target takes.)
        let cfg = std::fs::read_to_string(root.join("nasher.cfg")).unwrap();
        let cfg = cfg.replacen(
            "  [package.rules]",
            "  exclude = \"**/test_*.nss\"\n  [package.rules]",
            1,
        );
        std::fs::write(root.join("nasher.cfg"), cfg).unwrap();
        let mut m = Module::open(&root).unwrap();
        let p = m.project.as_mut().unwrap();
        assert_eq!(p.talk_table_file("mytlk"), Some(root.join("src/mytlk.tlk.json")));
        let made = p.add_talk_table("mytlk", mg_core::Language::GERMAN, true).unwrap();
        assert_eq!(made, [root.join("src/mytlk.tlk.json"), root.join("src/mytlkf.tlk.json")]);
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        assert_eq!(std::fs::read_to_string(&made[0]).unwrap(), r#"{"language":2,"entries":[]}"#);
        let cfg = std::fs::read_to_string(root.join("nasher.cfg")).unwrap();
        assert!(
            cfg.contains(
                "[target]\nname = \"mytlk\"\nfile = \"mytlk.tlk\"\n  [target.sources]\n  include \
                 = \"src/mytlk.tlk.json\"\n"
            ),
            "{cfg}"
        );
        // The module target leaves the tables out, and keeps the package's
        // exclude; each table is its target's.
        let module_target = p.target().clone();
        assert_eq!(module_target.excludes, ["**/test_*.nss", "**/*.tlk.json"]);
        assert!(!module_target.covers(&root, &made[0]));
        let tables = p.talk_tables("mytlk").unwrap();
        assert_eq!((tables.len(), tables[0].language()), (1, mg_core::Language::GERMAN));
        assert_eq!(p.talk_tables("mytlkf").unwrap()[0].path, made[1]);
        // Opened again: no warning of the JSON; packed by its target.
        let m = Module::open(&root).unwrap();
        assert!(m.project.as_ref().unwrap().warnings.is_empty());
        let package = Package::read(&root).unwrap();
        let (file, bytes) =
            super::super::pack_talk_table(&root, package.target("mytlk").unwrap()).unwrap();
        assert_eq!(file, root.join("mytlk.tlk"));
        assert_eq!(mg_tlk::Tlk::read(&bytes).unwrap().language, mg_core::Language::GERMAN);
        // Once is enough.
        let mut m = m;
        let again =
            m.project.as_mut().unwrap().add_talk_table("mytlk", mg_core::Language::ENGLISH, false);
        assert!(again.unwrap_err().to_string().contains("there already"));
    }

    /// The JSON of an item, tagged `tag`.
    fn item_json(tag: &str) -> String {
        format!(
            "{{\n  \"__data_type\": \"UTI \",\n  \"Tag\": {{\n    \"type\": \"cexostring\",\n    \
             \"value\": \"{tag}\"\n  }}\n}}\n"
        )
    }

    fn tag_of(m: &Module, name: &str) -> String {
        let gff = m.gff(&key(name, ResType::UTI)).unwrap().unwrap();
        String::from_utf8_lossy(gff.root.string("Tag").unwrap()).into_owned()
    }

    /// One item in one folder under two spellings (`x.UTI.json`, as
    /// Aurora's names come, and `x.uti.json`): nasher's pack renames the
    /// one not in lower case over the other, newer or older (measured with
    /// nasher 1.1.3), so the project reads that one, says so, and the next
    /// save leaves it alone, as `x.uti.json`. In two folders nasher packs
    /// one of them, in no fixed order: the first is read, and both stay.
    #[test]
    fn a_resource_under_two_spellings_is_the_one_nasher_packs() {
        let root = scratch("two-spellings");
        module().save_as(&project_location(&root)).unwrap();
        let src = root.join("src");
        std::fs::write(src.join("bread.UTI.json"), item_json("AURORA")).unwrap();
        std::fs::write(src.join("bread.uti.json"), item_json("MOONGLOW")).unwrap();
        // (A file system that doesn't set capitals apart keeps one file,
        // which both names find: listed, the folder holds one.)
        let spellings = (std::fs::read_dir(&src).unwrap().flatten())
            .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("bread.uti.json"))
            .count();
        if spellings < 2 {
            return;
        }
        let mut m = Module::open(&root).unwrap();
        assert_eq!(tag_of(&m, "bread"), "AURORA");
        let said = &m.project.as_ref().unwrap().warnings;
        assert!(
            said.iter().any(|w| w.contains(
                "as bread.UTI.json and bread.uti.json: nasher's pack \
                renames bread.UTI.json over bread.uti.json"
            )),
            "{said:?}"
        );
        m.save().unwrap();
        let items: Vec<String> = (std::fs::read_dir(&src).unwrap().flatten())
            .map(|e| e.file_name().into_string().unwrap())
            .filter(|n| n.to_ascii_lowercase().starts_with("bread."))
            .collect();
        assert_eq!(items, ["bread.uti.json"]);
        assert!(std::fs::read_to_string(src.join("bread.uti.json")).unwrap().contains("AURORA"));
        let mut m = Module::open(&root).unwrap();
        assert_eq!(tag_of(&m, "bread"), "AURORA");
        assert!(m.project.as_ref().unwrap().warnings.is_empty());
        // Changed and saved again: the one file.
        let mut gff = m.gff(&key("bread", ResType::UTI)).unwrap().unwrap();
        gff.root.set("Tag", Value::String(b"EDITED".to_vec()));
        m.set_gff(key("bread", ResType::UTI), &gff).unwrap();
        m.save().unwrap();
        assert!(std::fs::read_to_string(src.join("bread.uti.json")).unwrap().contains("EDITED"));
        // Two spellings of an item the module then deletes: both go.
        std::fs::write(src.join("bread.UTI.json"), item_json("AGAIN")).unwrap();
        let mut m = Module::open(&root).unwrap();
        assert_eq!(tag_of(&m, "bread"), "AGAIN");
        m.remove(&key("bread", ResType::UTI));
        m.save().unwrap();
        assert!(!src.join("bread.UTI.json").exists() && !src.join("bread.uti.json").exists());
        // In two folders: the first read, the other said and left.
        std::fs::create_dir_all(src.join("a")).unwrap();
        std::fs::create_dir_all(src.join("b")).unwrap();
        std::fs::write(src.join("b/cake.uti.json"), item_json("IN_B")).unwrap();
        std::fs::write(src.join("a/cake.uti.json"), item_json("IN_A")).unwrap();
        let mut m = Module::open(&root).unwrap();
        let other = if tag_of(&m, "cake") == "IN_A" { "b" } else { "a" };
        let said = &m.project.as_ref().unwrap().warnings;
        let left = src.join(other).join("cake.uti.json").display().to_string();
        assert!(
            said.iter().any(|w| w.starts_with(&left) && w.contains("in no fixed order")),
            "{said:?}"
        );
        m.save().unwrap();
        assert!(src.join("a/cake.uti.json").exists() && src.join("b/cake.uti.json").exists());
    }

    /// A source file that can't be read (JSON broken by a merge's markers)
    /// is left out of the module and said: the project opens. A resource
    /// of its name made in the module doesn't write over it.
    #[test]
    fn a_file_that_can_t_be_read_is_left_out_and_said() {
        let root = scratch("broken-file");
        module().save_as(&project_location(&root)).unwrap();
        let broken = root.join("src/bread.uti.json");
        let text = "<<<<<<< HEAD\n{\"__data_type\": \"UTI \"}\n=======\n{}\n>>>>>>> theirs\n";
        std::fs::write(&broken, text).unwrap();
        let mut m = Module::open(&root).unwrap();
        assert!(!m.contains(&key("bread", ResType::UTI)));
        assert!(m.contains(&key("a1", ResType::ARE)), "the rest is read");
        let said = &m.project.as_ref().unwrap().warnings;
        assert!(
            said.iter().any(|w| w.contains("bread.uti.json: can't be read, and is left out")),
            "{said:?}"
        );
        // Saved with other changes: the file stays as it is.
        m.set(key("other", ResType::NSS), b"void main() {}\n".to_vec());
        m.save().unwrap();
        assert_eq!(std::fs::read_to_string(&broken).unwrap(), text);
        // A resource of its name made here: not written over it.
        let mut item = Gff::new(*b"UTI ");
        item.root.set("Tag", Value::String(b"NEW".to_vec()));
        m.set_gff(key("bread", ResType::UTI), &item).unwrap();
        let err = m.save().unwrap_err();
        assert!(
            matches!(&err, ModuleError::ChangedOnDisk(p) if p == std::slice::from_ref(&broken)),
            "{err}"
        );
        assert_eq!(std::fs::read_to_string(&broken).unwrap(), text);
        // Mended outside: read as a file added.
        std::fs::write(&broken, item_json("MENDED")).unwrap();
        let (changes, problems) = m.project.as_mut().unwrap().outside();
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].key, key("bread", ResType::UTI));
    }
}
