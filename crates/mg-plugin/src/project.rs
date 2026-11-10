//! A project's own plugins: those in a folder of the checkout (`plugins/`
//! beside `nasher.cfg`, or the folder its `moonglow.toml` names), which
//! come with it for every builder. None of them runs until the builder
//! allows it, and what they allow is the plugin as it is then: its
//! content, by a digest of its files ([`Plugin::load_project`]). A plugin
//! changed since (a pull brought another version of it, or the same
//! version changed) is asked about again; one the builder said no to
//! stays off whatever it becomes, until they enable it.
//!
//! The answers are kept in Moonglow's own folder ([`Answers`], a file the
//! window and `mg` share), by the project's canonical path, the plugin's
//! id and its digest; never in the project's files, which come with its
//! checkout and could carry answers of their own. A yes is the plugin's
//! content's wherever it is: a project moved or copied elsewhere on disk
//! (another clone, a worktree) runs the very files allowed without being
//! asked again, as a project anywhere does whose plugin is, byte for byte,
//! one allowed. A no is the project's: kept for that project alone.

use std::path::{Path, PathBuf};

use mg_module::project_file::ProjectFile;
use serde::{Deserialize, Serialize};

use crate::Plugin;

/// The answers' file in Moonglow's data folder.
pub const ANSWERS_FILE: &str = "project-plugins.toml";

/// Where the answers are kept: `project-plugins.toml` in Moonglow's data
/// folder (`$XDG_DATA_HOME/moonglow` on Linux), beside its plugins folder.
pub fn answers_path() -> Option<PathBuf> {
    mg_module::build::data_dir().map(|d| d.join(ANSWERS_FILE))
}

/// A project's plugin, as the builder has answered for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trust {
    /// Not asked yet: it does not run.
    Unasked,
    /// Allowed as it is: it runs.
    Allowed,
    /// Allowed once, and changed since (another version, or other
    /// content): asked again, and off until then.
    Changed {
        /// The version allowed.
        was: String,
    },
    /// The builder said no: it stays off until they enable it.
    Denied,
}

impl Trust {
    /// Whether it runs.
    pub fn runs(&self) -> bool {
        *self == Trust::Allowed
    }

    /// Whether the builder is to be asked about it.
    pub fn to_ask(&self) -> bool {
        matches!(self, Trust::Unasked | Trust::Changed { .. })
    }

    /// What it is, as `mg plugin list` says it.
    pub fn name(&self) -> &'static str {
        match self {
            Trust::Unasked => "not asked",
            Trust::Allowed => "allowed",
            Trust::Changed { .. } => "changed since allowed",
            Trust::Denied => "denied",
        }
    }
}

/// One answer: a project's plugin, as it was when answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    /// The project's root, canonical.
    pub project: PathBuf,
    /// The plugin's id.
    pub plugin: String,
    pub allow: bool,
    /// Its version when answered, to say what was allowed.
    #[serde(default)]
    pub version: String,
    /// Its content when answered (`sha256:…`).
    #[serde(default)]
    pub digest: String,
}

/// The builder's answers about projects' plugins, as Moonglow keeps them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answers {
    #[serde(default, rename = "answer")]
    pub answers: Vec<Answer>,
}

/// A project's root as answers are kept under it.
pub fn canonical(root: &Path) -> PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}

impl Answers {
    /// The answers in `path` (none there: none). A file that can't be
    /// read allows nothing: the error says why.
    pub fn read(path: &Path) -> Result<Answers, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Answers::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    /// Writes them to `path` (its folder made), whole or not at all
    /// (`mg_module::write_replacing`).
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let text = format!(
            "# Moonglow's answers: which of a project's own plugins may run, by the project,\n\
             # the plugin and its content. Yours alone, never in a project.\n{}",
            toml::to_string(self).map_err(|e| e.to_string())?
        );
        mg_module::write_replacing(path, text.as_bytes())
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The answer for a plugin of the project whose root is `root`.
    pub fn get(&self, root: &Path, id: &str) -> Option<&Answer> {
        let project = canonical(root);
        self.answers.iter().find(|a| a.project == project && a.plugin == id)
    }

    /// What the builder answered for a project's plugin, as it is now.
    pub fn trust(&self, root: &Path, plugin: &Plugin) -> Trust {
        self.trust_in(&canonical(root), plugin)
    }

    /// [`Answers::trust`] for a project root already canonical (the path
    /// looked up on disk once, not each time: the window asks every frame).
    /// The project's own no holds; else a yes to the plugin's very content
    /// holds wherever it was given (the project moved or copied, another
    /// checkout of it): what runs is what was allowed.
    pub fn trust_in(&self, project: &Path, plugin: &Plugin) -> Trust {
        let id = &plugin.manifest.id;
        let found = self.answers.iter().find(|a| a.project == project && a.plugin == *id);
        if found.is_some_and(|a| !a.allow) {
            return Trust::Denied;
        }
        let digest = plugin.digest().filter(|d| !d.is_empty());
        let allowed = |a: &&Answer| a.allow && a.plugin == *id && Some(a.digest.as_str()) == digest;
        if self.answers.iter().any(|a| allowed(&a)) {
            return Trust::Allowed;
        }
        match found {
            Some(a) => Trust::Changed { was: a.version.clone() },
            None => Trust::Unasked,
        }
    }

    /// Where a yes to the plugin as it is now was given, if it was given
    /// for another project (moved, or copied): that project's root.
    pub fn allowed_elsewhere(&self, project: &Path, plugin: &Plugin) -> Option<&Path> {
        let id = &plugin.manifest.id;
        let digest = plugin.digest()?;
        if self.answers.iter().any(|a| a.project == project && a.plugin == *id) {
            return None;
        }
        (self.answers.iter())
            .find(|a| a.allow && a.plugin == *id && a.digest == digest)
            .map(|a| a.project.as_path())
    }

    /// Answers for a project's plugin as it is now: allowed (`allow`), or
    /// kept off.
    pub fn decide(&mut self, root: &Path, plugin: &Plugin, allow: bool) {
        let answer = Answer {
            project: canonical(root),
            plugin: plugin.manifest.id.clone(),
            allow,
            version: plugin.manifest.version.clone(),
            digest: plugin.digest().unwrap_or_default().to_string(),
        };
        self.answers.retain(|a| !(a.project == answer.project && a.plugin == answer.plugin));
        self.answers.push(answer);
    }
}

/// Answers for a project's plugin, and keeps the answer in `path` (read
/// again first: another Moonglow, or `mg`, may have answered since).
pub fn decide(path: &Path, root: &Path, plugin: &Plugin, allow: bool) -> Result<Answers, String> {
    let mut answers = Answers::read(path)?;
    answers.decide(root, plugin, allow);
    answers.write(path)?;
    Ok(answers)
}

/// The folder of a project's plugins, for its root and its files.
pub fn folder(root: &Path, file: &ProjectFile) -> PathBuf {
    file.plugins.folder_in(root)
}

/// A project's plugin found: its folder, the plugin (read whole, as it
/// runs) or why it could not be read.
#[derive(Debug, Clone)]
pub struct Found {
    pub dir: PathBuf,
    pub plugin: Result<Plugin, String>,
}

/// The project's plugins, each folder of its plugins' folder with a
/// `plugin.cfg`, read as they run; none of their code runs.
pub fn discover(root: &Path, file: &ProjectFile) -> Vec<Found> {
    crate::folders(&folder(root, file))
        .into_iter()
        .map(|dir| {
            let plugin = Plugin::load_project(&dir).map_err(|e| e.to_string());
            Found { dir, plugin }
        })
        .collect()
}
