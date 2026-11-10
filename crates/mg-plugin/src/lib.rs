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
//! - [`project`]: a project's own plugins, which come with its checkout
//!   and run only once the builder allows them; [`settings`]: the values
//!   a plugin keeps.

mod files;
mod kept;
mod kinds;
mod manifest;
mod package;
pub mod project;
mod runtime;
pub mod settings;
mod tables;
mod terrain;
mod testing;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

pub use manifest::{
    API, APIS, CheckDecl, CommandDecl, EventDecl, EventKind, ExporterDecl, ImporterDecl, Manifest,
    PanelDecl, RESOURCE_TARGETS, Scope, SettingDecl, SettingKind, SettingValue, Target, WizardDecl,
    is_color, path_title,
};
use mg_core::{ResRef, ResType};
pub use mg_edit::fix::{Fix, Found};
use mg_edit::{Edit, GffPath};
use mg_module::Module;
use mg_module::doctor::Finding;
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_rules::ObjectKind;
pub use package::{
    Existing, MARKER, MAX_ALL, MAX_ARCHIVE, MAX_BYTES, MAX_FILES, MAX_PLUGINS, Package,
    from_archive, pack, remove,
};
pub use runtime::EVENT_NAMES;
pub use settings::{SettingWrite, Settings};
pub use testing::{GAME_FOLDER, GameFor, TestResult, run_tests, test_faults, test_files};
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
    /// A project's plugin is read whole when found, and runs from that
    /// copy: what the builder allowed (by its digest) is what runs,
    /// whatever its folder holds later. None: it runs from its folder.
    pub frozen: Option<Frozen>,
}

/// A plugin's files as they were read, with the digest that names them
/// (`sha256:` and the hexadecimal SHA-256 of each file's path, length and
/// bytes, by path).
#[derive(Clone, PartialEq, Eq)]
pub struct Frozen {
    pub digest: String,
    pub(crate) files: Arc<BTreeMap<String, Vec<u8>>>,
}

impl std::fmt::Debug for Frozen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Frozen({}, {} files)", self.digest, self.files.len())
    }
}

impl Frozen {
    /// The files (by their paths in the folder, with `/`) and their digest.
    pub fn new(files: Vec<(String, Vec<u8>)>) -> Frozen {
        let files: BTreeMap<String, Vec<u8>> = files.into_iter().collect();
        let mut h = mg_core::sha256::Sha256::new();
        for (name, bytes) in &files {
            h.update(name.as_bytes());
            h.update(&[0]);
            h.update(&(bytes.len() as u64).to_le_bytes());
            h.update(bytes);
        }
        let digest = format!("sha256:{}", mg_core::sha1::hex(&h.finish()));
        Frozen { digest, files: Arc::new(files) }
    }
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
        Ok(Plugin { dir: dir.to_path_buf(), manifest, frozen: None })
    }

    /// Reads a project's plugin in folder `dir` whole (its files as an
    /// archive of it would hold them: not hidden ones, nor its `tests/`),
    /// to run from that copy; none of its code runs.
    pub fn load_project(dir: &Path) -> Result<Plugin, PluginError> {
        let shown = dir.join("plugin.cfg").display().to_string();
        let files = package::files_of(dir)?;
        let frozen = Frozen::new(files);
        let fault = |e: String| PluginError::Manifest(shown.clone(), e);
        let text = frozen.files.get("plugin.cfg").ok_or_else(|| fault("not there".into()))?;
        let text = std::str::from_utf8(text).map_err(|_| fault("it is not text (UTF-8)".into()))?;
        let manifest = Manifest::parse(text).map_err(fault)?;
        if !frozen.files.contains_key(&manifest.entry) {
            return Err(fault(format!("its entry {} is not in the folder", manifest.entry)));
        }
        Ok(Plugin { dir: dir.to_path_buf(), manifest, frozen: Some(frozen) })
    }

    /// A project's plugin's digest, by which it is allowed.
    pub fn digest(&self) -> Option<&str> {
        self.frozen.as_ref().map(|f| f.digest.as_str())
    }

    /// Whether it is a project's (read whole, [`Plugin::load_project`]).
    pub fn is_project(&self) -> bool {
        self.frozen.is_some()
    }

    /// A setting it declares.
    pub fn setting(&self, id: &str) -> Option<&SettingDecl> {
        self.manifest.settings.iter().find(|s| s.id == id)
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
        choices: Vec<Choice>,
        default: usize,
    },
    /// A color (API 0.8), its value `"#rrggbb"`. `variable`: a string
    /// variable the objects the job was given are drawn with, set to the
    /// color, while it is being picked (nothing is set by it).
    Color {
        default: [u8; 3],
        variable: Option<String>,
    },
}

impl FieldKind {
    /// What a field of this kind starts with, as its value comes back.
    pub fn default_value(&self) -> serde_json::Value {
        match self {
            FieldKind::Text { default } => serde_json::json!(default),
            FieldKind::Number { default, .. } => serde_json::json!(default),
            FieldKind::Check { default } => serde_json::json!(default),
            FieldKind::Choice { choices, default } => serde_json::json!(choices[*default].text),
            FieldKind::Color { default, .. } => serde_json::json!(color_text(*default)),
        }
    }
}

/// One of a choice field's options: its text (what the field's value is
/// when it is chosen) and the color of the swatch drawn before it (API
/// 0.9), if it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub text: String,
    pub color: Option<[u8; 3]>,
}

impl Choice {
    /// An option of text alone.
    pub fn text(text: impl Into<String>) -> Choice {
        Choice { text: text.into(), color: None }
    }
}

impl From<&str> for Choice {
    fn from(text: &str) -> Choice {
        Choice::text(text)
    }
}

/// A color as a color field gives it: `"#rrggbb"`, in lower case.
pub fn color_text([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// A color from `"#rrggbb"` (the `#` may be left out, any case), as a
/// color field takes one; `None` for anything else.
pub fn color_of(text: &str) -> Option<[u8; 3]> {
    let t = text.trim();
    let hex = t.strip_prefix('#').unwrap_or(t);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
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
    /// A page of a wizard (API 0.6), answered by [`Answer::Page`].
    Page(Page),
}

/// A page of a plugin's wizard, as the window shows it: its fields (their
/// defaults what was typed before, going back or kept on the page), and
/// where it is among the wizard's pages.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// The wizard's title, and the page's.
    pub wizard: String,
    pub title: String,
    /// What the page says above its fields.
    pub text: String,
    pub fields: Vec<FormField>,
    /// Its place, from 1, among `steps` pages.
    pub step: usize,
    pub steps: usize,
    /// There is a page to go back to.
    pub back: bool,
    /// It is the last: Finish, not Next.
    pub last: bool,
    /// What the page's check said is wrong with what was given, asking
    /// again.
    pub error: Option<String>,
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
    /// A wizard's page answered: its values, by field id, and whether the
    /// user went back (else on: Next, or Finish on the last).
    Page {
        values: BTreeMap<String, serde_json::Value>,
        back: bool,
    },
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

    /// Whether a person answers the questions (a window), who can change
    /// what a wizard's page's check objects to. Without one (`mg`, a
    /// test), that check ends the wizard with what it said.
    fn interactive(&self) -> bool {
        false
    }

    /// Asks the user; `None` when there is nobody to ask, or they closed
    /// the question unanswered.
    fn ask(&self, question: &Question) -> Option<Answer>;

    /// A resource the plugin read from the game's data (`ctx.game`) that
    /// is not there (`parts_chest.2da`), or there is no game data at all:
    /// a plugin's test says which it lacked (a note, when it passes).
    fn not_in_game(&self, _resource: &str) {}

    /// The plugin listed the game's resources (`ctx.game:resources()`):
    /// a plugin's test with no game says that the list is only of the
    /// module and the test's game folder (or that there is none).
    fn game_listed(&self) {}
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
    /// The plugin's settings (`ctx.settings`), as
    /// [`settings::resolve`] makes them: none given, their defaults.
    pub settings: Option<Settings>,
    /// How long the job may run: past it, it fails ("out of time"). None:
    /// as long as it takes (until it is canceled). An event that runs at
    /// once, within what the user is doing, has [`EVENT_BUDGET`].
    pub budget: Option<std::time::Duration>,
    /// The values of the panel's fields a command is run from (its button,
    /// or a field set; API 0.9), by field id, as a form's come back
    /// (`ctx.values`): empty for a command run from anywhere else.
    pub values: BTreeMap<String, serde_json::Value>,
}

impl Input {
    /// A job's input with nothing selected or open, and the plugin's
    /// settings at their defaults.
    pub fn new(module: Module, game: Option<Arc<GameData>>) -> Input {
        Input {
            module,
            game,
            selection: Selection::default(),
            settings: None,
            budget: None,
            values: BTreeMap::new(),
        }
    }
}

/// How long the handlers of one plugin's event may run when the event runs
/// at once, within what the user is doing ([`EventKind::at_once`]: a save
/// waits for it; a paste, a wizard's Finish): all the resources of a save
/// together.
pub const EVENT_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);

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
    /// A copy of a resource as one layer of the game's load order has it,
    /// by the layer's label (`hak:castle`, `key:nwn_base`): the resource
    /// browser's overridden copies, open read-only (only as what is open).
    Copy { key: ResKey, layer: String },
    /// A tile of an area, item `index` of its ARE's `Tile_List` (rows
    /// from the south-west, as the list has them; API 0.5).
    Tile { area: ResRef, index: usize },
}

impl Selected {
    /// What it is, as a command's `for` names it (`placeable`, `area`); a
    /// resource that is neither, and a layer's copy, by its type (`nss`,
    /// `dlg`, `utc`).
    pub fn kind(&self) -> String {
        match self {
            Selected::Object { kind, .. } => kind.name().to_string(),
            Selected::Blueprint(key) => match ObjectKind::from_restype(key.restype) {
                Some(kind) => kind.name().to_string(),
                None => key.restype.extension().unwrap_or_default().to_string(),
            },
            Selected::Area(_) => "area".to_string(),
            Selected::Tile { .. } => "tile".to_string(),
            Selected::Resource(key) | Selected::Copy { key, .. } => {
                key.restype.extension().unwrap_or_default().to_string()
            }
        }
    }

    /// What it is, as a command's `for` takes it ([`Target::takes`]): a
    /// placed object `Placed`, a blueprint `Blueprint`.
    pub fn target(&self) -> Option<Target> {
        match self {
            Selected::Object { kind, .. } => Some(Target::Placed(*kind)),
            Selected::Blueprint(key) => {
                ObjectKind::from_restype(key.restype).map(Target::Blueprint)
            }
            Selected::Area(_) => Some(Target::Area),
            Selected::Tile { .. } => Some(Target::Tile),
            Selected::Resource(key) => {
                RESOURCE_TARGETS.contains(&key.restype).then_some(Target::Resource(key.restype))
            }
            Selected::Copy { .. } => None,
        }
    }

    /// Whether a command for `targets` is for it.
    pub fn fits(&self, targets: &[Target]) -> bool {
        self.target().is_some_and(|t| targets.iter().any(|c| c.takes(t)))
    }

    /// The resource it is in, and where in it (the root: `""`), as
    /// `ctx.edit` takes them (a layer's copy: the resource it is a copy
    /// of, which an edit would make the module's own).
    pub fn place(&self) -> (ResKey, GffPath) {
        match self {
            Selected::Object { area, kind, index } => {
                (ResKey::new(*area, ResType::GIT), GffPath::root().item(kind.list(), *index))
            }
            Selected::Blueprint(key) | Selected::Resource(key) | Selected::Copy { key, .. } => {
                (*key, GffPath::root())
            }
            Selected::Area(area) => (ResKey::new(*area, ResType::ARE), GffPath::root()),
            Selected::Tile { area, index } => {
                (ResKey::new(*area, ResType::ARE), GffPath::root().item("Tile_List", *index))
            }
        }
    }

    /// The layer whose copy it is (by its label), for a layer's copy.
    pub fn layer(&self) -> Option<&str> {
        match self {
            Selected::Copy { layer, .. } => Some(layer),
            _ => None,
        }
    }

    /// What `mg plugin run --select` takes: a resource's name, and for an
    /// object placed in an area the GIT and the path to it
    /// (`tavern.git/Placeable List[3]`); a layer's copy of a resource, the
    /// name and the layer's label (`nw_chest.utp@key:nwn_base`).
    pub fn parse(text: &str) -> Result<Selected, String> {
        if let Some((name, layer)) = text.split_once('@') {
            let key = ResKey::from_filename(name)
                .ok_or_else(|| format!("{name:?} is not a resource's name (name.ext)"))?;
            if layer.is_empty() {
                return Err(format!("{text:?}: a copy names its layer after the @"));
            }
            return Ok(Selected::Copy { key, layer: layer.to_string() });
        }
        let (name, path) = match text.find('/') {
            Some(at) => (&text[..at], &text[at..]),
            None => (text, ""),
        };
        let key = ResKey::from_filename(name)
            .ok_or_else(|| format!("{name:?} is not a resource's name (name.ext)"))?;
        if key.restype == ResType::ARE && !path.is_empty() {
            let path: GffPath = path.parse().map_err(|e| format!("{text:?}: {e}"))?;
            return match path.0.as_slice() {
                [mg_edit::Step::Item(list, index)] if list == "Tile_List" => {
                    Ok(Selected::Tile { area: key.resref, index: *index })
                }
                _ => Err(format!(
                    "{text:?}: a tile is its area's ARE and its place in the tile list \
                     (\"{name}/Tile_List[0]\")"
                )),
            };
        }
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
    /// What is open in front edited together, where it is several
    /// (`ctx.open.together`; `open` is the first): several blueprints,
    /// placed objects or areas in one Properties window. Empty for one.
    pub together: Vec<Selected>,
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
    /// The settings it set (`ctx.settings:set`), for the host to keep.
    pub settings: Vec<SettingWrite>,
    /// What a `resource-saving` handler said of the resources it was
    /// called for: warnings, and refusals.
    pub said: Vec<Said>,
}

/// What a `resource-saving` handler said of a resource
/// (`ctx.event:warn`, `ctx.event:refuse`): a warning, which the user is
/// asked about before the save goes on, or a refusal, which stops it; with
/// a fix (`ctx.fix`) the user may make, if it gave one.
#[derive(Debug, Clone, PartialEq)]
pub struct Said {
    pub resource: ResKey,
    pub text: String,
    pub refused: bool,
    pub fix: Option<Fix>,
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
    /// refused, and then nothing is written. Each file is written whole or
    /// not at all, in the place of the one of its name.
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
            // (Whole or not at all, and in the place of a link the file,
            // not what the link leads to.)
            mg_module::write_replacing(&root.join(name), bytes)
                .map_err(|e| format!("{name}: {e}"))?;
        }
        Ok((self.files.len() - replaced, replaced))
    }
}

/// What happened, for a plugin's event handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    /// For a module saved: the resources that changed. For a resource
    /// saving or saved, or made: the resources, each of which the handler
    /// is called for (those its manifest's `types` and `for` take).
    pub resources: Vec<ResKey>,
    /// For a resource saving or saved: each as it was before (its file's
    /// bytes, as last read or saved); a resource new since has none.
    pub before: BTreeMap<ResKey, Arc<[u8]>>,
    /// For objects placed or pasted: where each is (those its manifest's
    /// `for` takes are given to it).
    pub objects: Vec<Selected>,
    /// What made the objects or the resource: `"wizard"`, `"palette"`,
    /// `"paste"`… (`ctx.event.by`).
    pub by: String,
}

impl Event {
    /// An event with its resources, and nothing else.
    pub fn new(kind: EventKind, resources: Vec<ResKey>) -> Event {
        Event { kind, resources, before: BTreeMap::new(), objects: Vec::new(), by: String::new() }
    }

    /// The resources a save writes, and each as it was: those of `module`
    /// that `was` (each resource's bytes before; none: it had none) does
    /// not give as they are, and those it gives that the module no longer
    /// has.
    pub fn saving<'a>(
        kind: EventKind,
        module: &Module,
        was: impl Fn(&ResKey) -> Option<&'a [u8]>,
        gone: impl IntoIterator<Item = ResKey>,
    ) -> Event {
        let mut event = Event::new(kind, Vec::new());
        let mut keys: Vec<ResKey> = module.keys().copied().collect();
        keys.sort();
        for key in keys {
            let now = module.get(&key);
            let before = was(&key);
            let same = match (before, now) {
                (Some(a), Some(b)) => std::ptr::eq(a, b) || a == b,
                _ => false,
            };
            if same {
                continue;
            }
            if let Some(b) = before {
                event.before.insert(key, Arc::from(b));
            }
            event.resources.push(key);
        }
        for key in gone {
            if module.contains(&key) || event.resources.contains(&key) {
                continue;
            }
            if let Some(b) = was(&key) {
                event.before.insert(key, Arc::from(b));
                event.resources.push(key);
            }
        }
        event
    }
}

/// Resources a command wants in a hak of the user's hak folder: added to
/// it, or in place of those of the same name there. Not part of the
/// command's undoable step: a file outside the module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HakWrite {
    /// The hak's name, without `.hak`.
    pub name: String,
    /// Each resource's bytes, or the file of the user's it is a copy of
    /// (or a scratch file of the job's, past what it holds in memory:
    /// [`HELD_IN_MEMORY`]).
    pub files: Vec<(ResKey, HakData)>,
    /// The job's scratch files, kept while this is.
    pub held: Option<Arc<Held>>,
}

pub use kept::{HELD_IN_MEMORY, HakWritten, Held, UNDO_FOLDER};
pub use mg_module::hak_edit::Source as HakData;

impl HakWrite {
    /// Writes the resources into the hak in folder `dir`
    /// ([`mg_module::hak_edit::write_into`]): how many were added and how
    /// many replaced.
    pub fn write(&self, dir: &Path) -> Result<(usize, usize), String> {
        let done = self.write_kept(dir)?;
        Ok((done.added, done.replaced))
    }

    /// Writes the resources into the hak in folder `dir`, keeping the hak
    /// as it was before and as it is written (linked, in the folder's
    /// hidden [`UNDO_FOLDER`]), so that it can be taken back (the
    /// command's Undo, or its edits failing) and written again (Redo).
    pub fn write_kept(&self, dir: &Path) -> Result<HakWritten, String> {
        HakWritten::write(self, dir)
    }
}

/// The largest file of the user's a plugin reads, in bytes.
pub const MAX_FILE: u64 = 64 << 20;

/// The most memory a plugin's code may hold, in bytes.
pub const MEMORY_LIMIT: usize = 256 << 20;

/// The most a job may make Moonglow hold for it besides, in bytes: the
/// tables and talk tables it makes, the pictures it has decoded, and the
/// edits and files that wait to be written.
pub const HELD_LIMIT: usize = 512 << 20;

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
/// check's id ([`Plugin::check_id`]), and the fix the check gave it if it
/// gave one (`ctx.fix`: edits worked out on the module as it was read,
/// none of them made).
pub fn run_check(
    plugin: &Plugin,
    check: &str,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Vec<Found>, PluginError> {
    let decl = plugin.manifest.checks.iter().find(|c| c.id == check).ok_or_else(|| {
        PluginError::Unknown {
            plugin: plugin.manifest.id.clone(),
            kind: "check",
            id: check.to_string(),
        }
    })?;
    runtime::run_check(plugin, decl, input, host)
}

/// Runs an exporter of a plugin (`exporter`: its id) on a module, writing
/// to `to`, the file or the folder the user chose: the files it wrote, as
/// the outcome's `folders` (to write when it is done), and no edits.
pub fn run_export(
    plugin: &Plugin,
    exporter: &str,
    to: &Path,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let decl = plugin.manifest.exporters.iter().find(|c| c.id == exporter).ok_or_else(|| {
        PluginError::Unknown {
            plugin: plugin.manifest.id.clone(),
            kind: "exporter",
            id: exporter.to_string(),
        }
    })?;
    kinds::run_export(plugin, decl, to, input, host)
}

/// Runs an importer of a plugin on a module, reading `from`, the file or
/// the folder the user chose: its edits, to apply as one command.
pub fn run_import(
    plugin: &Plugin,
    importer: &str,
    from: &Path,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let decl = plugin.manifest.importers.iter().find(|c| c.id == importer).ok_or_else(|| {
        PluginError::Unknown {
            plugin: plugin.manifest.id.clone(),
            kind: "importer",
            id: importer.to_string(),
        }
    })?;
    kinds::run_import(plugin, decl, from, input, host)
}

/// Runs a wizard of a plugin: its pages asked of the host one at a time
/// ([`Question::Page`]), then its Finish: the edits, to apply as one
/// command. Canceled on a page: [`PluginError::Canceled`], nothing made.
pub fn run_wizard(
    plugin: &Plugin,
    wizard: &str,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let decl = plugin.manifest.wizards.iter().find(|c| c.id == wizard).ok_or_else(|| {
        PluginError::Unknown {
            plugin: plugin.manifest.id.clone(),
            kind: "wizard",
            id: wizard.to_string(),
        }
    })?;
    kinds::run_wizard(plugin, decl, input, host)
}

/// Runs a panel's handler: what the panel shows. It only reads; past
/// [`PANEL_BUDGET`] (unless the input says another) it is stopped.
pub fn run_panel(
    plugin: &Plugin,
    panel: &str,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Shown, PluginError> {
    let decl = plugin.manifest.panels.iter().find(|c| c.id == panel).ok_or_else(|| {
        PluginError::Unknown {
            plugin: plugin.manifest.id.clone(),
            kind: "panel",
            id: panel.to_string(),
        }
    })?;
    kinds::run_panel(plugin, decl, input, host)
}

pub use kinds::{Element, Item, MAX_ELEMENTS, MAX_TEXT, PANEL_BUDGET, Shown, Tone, value_text};

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
    runtime::inspect(runtime::source_of(plugin), &plugin.manifest, host)
}
