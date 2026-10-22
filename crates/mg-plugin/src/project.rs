//! A project's own plugins: those in a folder of the checkout (`plugins/`
//! beside `nasher.cfg`, or the folder its `moonglow.toml` names), which
//! come with it for every builder. None of them runs until the builder
//! allows it, and what they allow is the plugin as it is then: its
//! content, by a digest of its files ([`Plugin::load_project`]). A plugin
//! changed since (a pull brought another version of it, or the same
//! version changed) is asked about again; one the builder said no to
//! stays off whatever it becomes, until they enable it. The answers are
//! the builder's own (`.moonglow/user.toml`, never committed).

use std::path::{Path, PathBuf};

use mg_module::project_file::ProjectFile;
use mg_module::project_plugins::Decision;

use crate::Plugin;

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

/// What the builder answered for a project's plugin, as it is now.
pub fn trust(file: &ProjectFile, plugin: &Plugin) -> Trust {
    let Some(d) = file.plugins.decided.get(&plugin.manifest.id) else { return Trust::Unasked };
    match (d.allow, plugin.digest()) {
        (false, _) => Trust::Denied,
        (true, Some(digest)) if digest == d.digest => Trust::Allowed,
        (true, _) => Trust::Changed { was: d.version.clone() },
    }
}

/// The answer that allows a plugin as it is (`allow`), or keeps it off.
pub fn decision(plugin: &Plugin, allow: bool) -> Decision {
    Decision {
        allow,
        version: plugin.manifest.version.clone(),
        digest: plugin.digest().unwrap_or_default().to_string(),
    }
}

/// Keeps the builder's answer for a project's plugin in their own file.
pub fn decide(root: &Path, plugin: &Plugin, allow: bool) -> Result<(), String> {
    let d = decision(plugin, allow);
    mg_module::project_plugins::set_decision(root, &plugin.manifest.id, Some(&d))
        .map_err(|e| e.to_string())
}
