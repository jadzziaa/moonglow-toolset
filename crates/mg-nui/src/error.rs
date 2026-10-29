//! What can go wrong reading a window, naming one and building its scripts.

use mg_resman::ResKey;
use thiserror::Error;

use crate::Diagnostic;

/// Why a window could not be read, made or built. The variants tell apart
/// what a caller may want to treat differently: text that is not JSON, a
/// name that cannot be a window's, a resource in the way, a script changed
/// by hand, and a script that does not compile.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NuiError {
    /// The bytes are not JSON, or an object in them has a key twice.
    #[error("{0}")]
    Json(String),
    /// JSON, but not a settings resource this version reads.
    #[error("{0}")]
    Settings(String),
    /// A name that cannot be a window's (see [`crate::check_name`]).
    #[error(
        "{name:?}: use 1–14 lowercase ASCII letters, digits or underscores (room for _o and _e)."
    )]
    Name { name: String },
    /// A new window's resource is in the module already.
    #[error("Resource already exists: {0}")]
    Exists(ResKey),
    /// The module's rename could not move a script of the window: its
    /// error.
    #[error("{0}")]
    Rename(String),
    /// The window has no layout in the module.
    #[error("Missing {0}")]
    Missing(ResKey),
    /// The checks found errors ([`crate::validate`]): these, a line each.
    #[error("{}", .0.iter().map(|d| format!("{}: {}", d.path, d.message)).collect::<Vec<_>>().join("\n"))]
    Invalid(Vec<Diagnostic>),
    /// A script of the window was written or changed by hand where Build
    /// would lose it, or is another author's: the message says which.
    #[error("{0}")]
    EditedByHand(String),
    /// A compiled script is in the module without its source.
    #[error("{0} exists without its source")]
    WithoutSource(ResKey),
    /// A handler shows a swap layout variant the settings do not have.
    #[error("Missing variant: {0}")]
    MissingVariant(String),
    /// Windows could not be brought in from files ([`crate::import`]):
    /// what was wrong, and where.
    #[error("{0}")]
    Import(String),
    /// A script of the window does not compile: the compiler's error.
    #[error("{0}")]
    Compile(#[source] mg_script::CompileError),
}
