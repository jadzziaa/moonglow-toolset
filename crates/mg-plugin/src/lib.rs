//! Plugins: folders with a manifest (`plugin.cfg`) and Luau scripts that
//! add commands and checks to Moonglow (`docs/plugin-proposal.md`).
//!
//! A plugin never changes a module itself. Its code runs in a sandbox (no
//! files, no network, no other programs) on a snapshot of the module, and
//! what it hands back is data: the edits it made, for the host to apply as
//! one undoable command, or a check's findings. The host is the window or
//! `mg`; both use this crate the same way.
//!
//! - [`Plugin::load`] and [`discover`] read manifests, running nothing.
//! - [`run_command`] and [`run_check`] run a plugin's code for one job.
//! - [`Host`] is what the plugin's code reaches of whoever runs it: the
//!   log, progress, and questions for the user.
//! - [`Package`] is a plugin as an archive to hand around, read and
//!   installed into a plugins folder.

mod files;
mod manifest;
mod package;
mod runtime;
mod tables;
mod terrain;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

pub use manifest::{API, APIS, CheckDecl, CommandDecl, EventDecl, EventKind, Manifest, Target};
use mg_core::{ResRef, ResType};
use mg_edit::{Edit, GffPath};
use mg_module::Module;
use mg_module::doctor::Finding;
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_rules::ObjectKind;
pub use package::{
    Existing, MARKER, MAX_ARCHIVE, MAX_BYTES, MAX_FILES, Package, from_archive, pack, remove,
};
use thiserror::Error;

/// What can go wrong with a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PluginError {
    #[error("{0}: {1}")]
    Manifest(String, String),
    #[error("{plugin} has no {kind} {id:?}")]
    Unknown { plugin: String, kind: &'static str, id: String },
    /// Its code failed: what it said, and where.
    #[error("{plugin}: {message}")]
    Script { plugin: String, message: String },
    #[error("{0}: canceled")]
    Canceled(String),
    /// An archive that is no plugin's, or one that could not be
    /// installed: why.
    #[error("{0}")]
    Package(String),
}

/// A plugin as installed: its folder and what its manifest says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    pub dir: PathBuf,
    pub manifest: Manifest,
}

impl Plugin {
    /// Reads the plugin in folder `dir` (its `plugin.cfg`); none of its
    /// code runs.
    pub fn load(dir: &Path) -> Result<Plugin, PluginError> {
        let file = dir.join("plugin.cfg");
        let shown = file.display().to_string();
        let text = std::fs::read_to_string(&file)
            .map_err(|e| PluginError::Manifest(shown.clone(), e.to_string()))?;
        let manifest =
            Manifest::parse(&text).map_err(|e| PluginError::Manifest(shown.clone(), e))?;
        if !dir.join(&manifest.entry).is_file() {
            return Err(PluginError::Manifest(
                shown,
                format!("its entry {} is not in the folder", manifest.entry),
            ));
        }
        Ok(Plugin { dir: dir.to_path_buf(), manifest })
    }

    /// The id of one of its checks as findings carry it: the plugin's id,
    /// a slash, the check's.
    pub fn check_id(&self, check: &str) -> String {
        format!("{}/{check}", self.manifest.id)
    }

    /// The finding for one of its checks that failed to run: an error
    /// under the check's id, about the module.
    pub fn check_failed(&self, check: &str, why: &PluginError) -> Finding {
        Finding {
            severity: mg_module::doctor::Severity::Error,
            check: self.check_id(check).into(),
            source: "plugin".into(),
            resource: ResKey::parse("module", mg_core::ResType::IFO).expect("a resource name"),
            at: String::new(),
            message: format!("the check failed: {why}"),
        }
    }
}

/// The plugins' folders in `dir`: each folder with a `plugin.cfg`, sorted
/// by name. Hidden ones are not plugins (an install under way is one).
pub fn folders(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .filter(|p| p.join("plugin.cfg").is_file())
        .collect();
    folders.sort();
    folders
}

/// The plugins in the folders of `dir` (each folder with a `plugin.cfg`),
/// by folder name: each one read, or why it could not be.
pub fn discover(dir: &Path) -> Vec<Result<Plugin, PluginError>> {
    folders(dir).iter().map(|d| Plugin::load(d)).collect()
}

/// How much a log line matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warning,
    Error,
}

/// A field of a form a plugin asks the user to fill.
#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    /// The key its value comes back under.
    pub id: String,
    pub label: String,
    pub kind: FieldKind,
}

/// What a form's field takes, and what it starts with.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text {
        default: String,
    },
    Number {
        default: f64,
        min: Option<f64>,
        max: Option<f64>,
    },
    Check {
        default: bool,
    },
    /// One of these; the default is its place in them.
    Choice {
        choices: Vec<String>,
        default: usize,
    },
}

/// Something a plugin asks of the user.
#[derive(Debug, Clone, PartialEq)]
pub enum Question {
    /// Something to read.
    Message(String),
    /// Yes or no.
    Confirm(String),
    /// A form to fill.
    Form { title: String, fields: Vec<FormField> },
    /// A file of the user's to read: what it is for, and the extensions
    /// it may have (none: any).
    File { title: String, extensions: Vec<String> },
    /// A folder of the user's to read the files of.
    Folder { title: String },
}

/// The user's answer.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// A message read, or a question answered yes.
    Yes,
    No,
    /// A form's values, by field id (text, a number, true or false; a
    /// choice as its text).
    Values(BTreeMap<String, serde_json::Value>),
    /// The file or the folder chosen.
    Path(PathBuf),
}

/// What a plugin's code reaches of whoever runs it.
pub trait Host {
    fn log(&self, level: Level, text: &str);

    /// `done` of `total` steps (0: not known), and what is being done.
    fn progress(&self, _done: usize, _total: usize, _note: &str) {}

    /// Whether the job was called off: the plugin's code stops at once.
    fn cancelled(&self) -> bool {
        false
    }

    /// Asks the user; `None` when there is nobody to ask, or they closed
    /// the question unanswered.
    fn ask(&self, question: &Question) -> Option<Answer>;
}

/// What a job reads.
#[derive(Debug, Clone)]
pub struct Input {
    /// The module as it is, unsaved changes included.
    pub module: Module,
    /// The game data, with the module's haks (and the module) layered in.
    pub game: Option<Arc<GameData>>,
    /// What is selected and open in the window (`ctx.selection`,
    /// `ctx.open`): nothing for a check, or where there is no window.
    pub selection: Selection,
}

impl Input {
    /// A job's input with nothing selected or open.
    pub fn new(module: Module, game: Option<Arc<GameData>>) -> Input {
        Input { module, game, selection: Selection::default() }
    }
}

/// Something selected, or open, in the window: what the plugin reads as
/// an entry of `ctx.selection` (its tag and resref read from the job's
/// copy of the module).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selected {
    /// An object placed in an area: item `index` of its kind's list in the
    /// area's GIT.
    Object { area: ResRef, kind: ObjectKind, index: usize },
    /// A blueprint: the module's own, or the game's (a standard palette's).
    Blueprint(ResKey),
    /// An area (its ARE).
    Area(ResRef),
    /// Another resource, in its editor (only as what is open: a script, a
    /// conversation).
    Resource(ResKey),
}

impl Selected {
    /// What it is, as a command's `for` names it (`placeable`, `area`); a
    /// resource that is neither, by its type (`nss`, `dlg`).
    pub fn kind(&self) -> String {
        match self {
            Selected::Object { kind, .. } => kind.name().to_string(),
            Selected::Blueprint(key) => match ObjectKind::from_restype(key.restype) {
                Some(kind) => kind.name().to_string(),
                None => key.restype.extension().unwrap_or_default().to_string(),
            },
            Selected::Area(_) => "area".to_string(),
            Selected::Resource(key) => key.restype.extension().unwrap_or_default().to_string(),
        }
    }

    /// What a command must be for to be run on it.
    pub fn target(&self) -> Option<Target> {
        match self {
            Selected::Object { kind, .. } => Some(Target::Object(*kind)),
            Selected::Blueprint(key) => ObjectKind::from_restype(key.restype).map(Target::Object),
            Selected::Area(_) => Some(Target::Area),
            Selected::Resource(_) => None,
        }
    }

    /// The resource it is in, and where in it (the root: `""`), as
    /// `ctx.edit` takes them.
    pub fn place(&self) -> (ResKey, GffPath) {
        match self {
            Selected::Object { area, kind, index } => {
                (ResKey::new(*area, ResType::GIT), GffPath::root().item(kind.list(), *index))
            }
            Selected::Blueprint(key) | Selected::Resource(key) => (*key, GffPath::root()),
            Selected::Area(area) => (ResKey::new(*area, ResType::ARE), GffPath::root()),
        }
    }

    /// What `mg plugin run --select` takes: a resource's name, and for an
    /// object placed in an area the GIT and the path to it
    /// (`tavern.git/Placeable List[3]`).
    pub fn parse(text: &str) -> Result<Selected, String> {
        let (name, path) = match text.find('/') {
            Some(at) => (&text[..at], &text[at..]),
            None => (text, ""),
        };
        let key = ResKey::from_filename(name)
            .ok_or_else(|| format!("{name:?} is not a resource's name (name.ext)"))?;
        if key.restype == ResType::GIT {
            let path: GffPath = match path {
                "" => GffPath::root(),
                path => path.parse().map_err(|e| format!("{text:?}: {e}"))?,
            };
            return match path.0.as_slice() {
                [mg_edit::Step::Item(list, index)] => match ObjectKind::from_list(list) {
                    Some(kind) => Ok(Selected::Object { area: key.resref, kind, index: *index }),
                    None => Err(format!("{list:?} is not a list of an area's objects")),
                },
                _ => Err(format!(
                    "{text:?}: an object placed in an area is its GIT and its place in a list \
                     (\"{name}/Placeable List[0]\")"
                )),
            };
        }
        if !path.is_empty() {
            return Err(format!(
                "{text:?}: only an object placed in an area (in a GIT) has a path"
            ));
        }
        Ok(match key.restype {
            ResType::ARE => Selected::Area(key.resref),
            t if ObjectKind::from_restype(t).is_some() => Selected::Blueprint(key),
            _ => Selected::Resource(key),
        })
    }
}

/// What is selected in the window (the objects in the area view, or the
/// blueprints in a palette or the module tree; for a command chosen from
/// a right-click menu, what was clicked), and what is open in front.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    pub selected: Vec<Selected>,
    pub open: Option<Selected>,
}

/// What a command made: the edits to apply as one command, and its name;
/// and what it wants written into haks (which the user agreed to).
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub label: String,
    pub edits: Vec<Edit>,
    pub haks: Vec<HakWrite>,
    /// Files it wants written into folders the user chose (and agreed
    /// to have written into).
    pub folders: Vec<FolderWrite>,
}

/// Files a command wants in a folder the user chose: new ones, or in
/// place of those of the same name there. Not part of the command's
/// undoable step: files outside the module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderWrite {
    pub dir: PathBuf,
    /// Each file's name (a name, not a path) and bytes.
    pub files: Vec<(String, Vec<u8>)>,
}

/// Whether `name` is a file's name a plugin may write in a folder: a name
/// alone, nothing that leads elsewhere or hides.
pub fn is_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && !name.ends_with(['.', ' '])
        && !name.chars().any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
}

impl FolderWrite {
    /// Writes the files into the folder: how many were new and how many
    /// replaced. A name that is a link out of the folder, or a folder, is
    /// refused, and then nothing is written.
    pub fn write(&self) -> Result<(usize, usize), String> {
        let root = self.dir.canonicalize().map_err(|e| format!("{}: {e}", self.dir.display()))?;
        let mut replaced = 0;
        for (name, _) in &self.files {
            if !is_file_name(name) {
                return Err(format!("{name:?} is not a file's name"));
            }
            let path = root.join(name);
            if path.symlink_metadata().is_ok() {
                let real = path.canonicalize().map_err(|e| format!("{name}: {e}"))?;
                if real.parent() != Some(&root) || !real.is_file() {
                    return Err(format!("{name} is not a file of the folder"));
                }
                replaced += 1;
            }
        }
        for (name, bytes) in &self.files {
            std::fs::write(root.join(name), bytes).map_err(|e| format!("{name}: {e}"))?;
        }
        Ok((self.files.len() - replaced, replaced))
    }
}

/// What happened, for a plugin's event handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    /// For a module saved: the resources that changed.
    pub resources: Vec<ResKey>,
}

/// Resources a command wants in a hak of the user's hak folder: added to
/// it, or in place of those of the same name there. Not part of the
/// command's undoable step: a file outside the module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HakWrite {
    /// The hak's name, without `.hak`.
    pub name: String,
    /// Each resource's bytes, or the file of the user's it is a copy of.
    pub files: Vec<(ResKey, HakData)>,
}

pub use mg_module::hak_edit::Source as HakData;

impl HakWrite {
    /// Writes the resources into the hak in folder `dir`
    /// ([`mg_module::hak_edit::write_into`]): how many were added and how
    /// many replaced.
    pub fn write(&self, dir: &Path) -> Result<(usize, usize), String> {
        let path = dir.join(format!("{}.hak", self.name));
        let done = mg_module::hak_edit::write_into(&path, self.files.clone())?;
        Ok((done.added, done.replaced))
    }
}

/// The largest file of the user's a plugin reads, in bytes.
pub const MAX_FILE: u64 = 64 << 20;

/// The most memory a plugin's code may hold, in bytes.
pub const MEMORY_LIMIT: usize = 256 << 20;

/// Runs a command of a plugin on a module: the edits it made, to apply as
/// one command. Nothing is changed here.
pub fn run_command(
    plugin: &Plugin,
    command: &str,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let decl = plugin.manifest.commands.iter().find(|c| c.id == command).ok_or_else(|| {
        PluginError::Unknown {
            plugin: plugin.manifest.id.clone(),
            kind: "command",
            id: command.to_string(),
        }
    })?;
    runtime::run_command(plugin, decl, input, host)
}

/// Runs what a plugin does on an event (`event`: the id its manifest
/// declares it under): as a command, its edits to apply as one.
pub fn run_event(
    plugin: &Plugin,
    event: &str,
    happened: &Event,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let decl = plugin.manifest.events.iter().find(|e| e.id == event).ok_or_else(|| {
        PluginError::Unknown {
            plugin: plugin.manifest.id.clone(),
            kind: "event",
            id: event.to_string(),
        }
    })?;
    runtime::run_event(plugin, decl, happened, input, host)
}

/// Runs a check of a plugin on a module: its findings, each carrying the
/// check's id ([`Plugin::check_id`]).
pub fn run_check(
    plugin: &Plugin,
    check: &str,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Vec<Finding>, PluginError> {
    let decl = plugin.manifest.checks.iter().find(|c| c.id == check).ok_or_else(|| {
        PluginError::Unknown {
            plugin: plugin.manifest.id.clone(),
            kind: "check",
            id: check.to_string(),
        }
    })?;
    runtime::run_check(plugin, decl, input, host)
}

/// Runs code typed into the plugin console on a module, with `mg` (the
/// API) and `ctx` (as a command's handler gets it) at hand: what it
/// returns goes to the log, its edits come back like a command's. The
/// code is sandboxed as a plugin's is, and can `require` no files.
pub fn run_console(code: &str, input: Input, host: Rc<dyn Host>) -> Result<Outcome, PluginError> {
    runtime::run_console(code, input, host)
}

/// Every name a plugin's code can use, as the reference writes them
/// (`mg.command`, `ctx.module:gff`, `ctx.plugin.id`).
pub fn api_names() -> Vec<String> {
    runtime::api_names()
}

/// Loads a plugin's code and compares what it registers with what its
/// manifest declares: the faults, as text (none: the two agree).
pub fn inspect(plugin: &Plugin, host: Rc<dyn Host>) -> Result<Vec<String>, PluginError> {
    runtime::inspect(runtime::Source::Folder(plugin.dir.clone()), &plugin.manifest, host)
}
