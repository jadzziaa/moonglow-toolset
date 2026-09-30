//! File dialogs behind a trait, so tests can script them.

use std::path::{Path, PathBuf};

/// The dialogs the application opens.
pub trait Dialogs {
    /// Asks for a module to open (a `.mod` file or a module folder).
    fn open_module(&mut self, start: Option<&Path>) -> Option<PathBuf>;
    /// Asks where to save the module.
    fn save_module(&mut self, current: Option<&Path>) -> Option<PathBuf>;
}

/// Dialogs that answer from a script (for tests), or cancel.
#[derive(Debug, Default)]
pub struct NoDialogs {
    pub open: Vec<PathBuf>,
    pub save: Vec<PathBuf>,
}

impl Dialogs for NoDialogs {
    fn open_module(&mut self, _start: Option<&Path>) -> Option<PathBuf> {
        self.open.pop()
    }
    fn save_module(&mut self, _current: Option<&Path>) -> Option<PathBuf> {
        self.save.pop()
    }
}
