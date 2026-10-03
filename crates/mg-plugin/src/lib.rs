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

mod manifest;
mod runtime;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

pub use manifest::{API, CheckDecl, CommandDecl, Manifest};
use mg_edit::Edit;
use mg_module::Module;
use mg_module::doctor::Finding;
use mg_rules::GameData;
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
}

/// The plugins in the folders of `dir` (each folder with a `plugin.cfg`),
/// by folder name: each one read, or why it could not be.
pub fn discover(dir: &Path) -> Vec<Result<Plugin, PluginError>> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut folders: Vec<PathBuf> =
        entries.flatten().map(|e| e.path()).filter(|p| p.join("plugin.cfg").is_file()).collect();
    folders.sort();
    folders.iter().map(|d| Plugin::load(d)).collect()
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
}

/// What a command made: the edits to apply as one command, and its name.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub label: String,
    pub edits: Vec<Edit>,
}

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

/// Loads a plugin's code and compares what it registers with what its
/// manifest declares: the faults, as text (none: the two agree).
pub fn inspect(plugin: &Plugin, host: Rc<dyn Host>) -> Result<Vec<String>, PluginError> {
    runtime::inspect(plugin, host)
}
