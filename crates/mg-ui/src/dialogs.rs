//! File dialogs behind a trait, so tests can script them.

use std::path::{Path, PathBuf};

/// What a file dialog is for (its title and file filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A module: `.mod` (or `.nwm` to open).
    Module,
    /// An export archive: `.erf`.
    Erf,
    /// Any file (resource export).
    Any,
    /// A script set: `.ini`.
    ScriptSet,
    /// A class spell list: `.ini`.
    SpellList,
}

impl FileKind {
    pub fn title(self, save: bool) -> &'static str {
        match (self, save) {
            (FileKind::Module, false) => "Open Module",
            (FileKind::Module, true) => "Save Module As",
            (FileKind::Erf, false) => "Import",
            (FileKind::Erf, true) => "Export",
            (FileKind::Any, false) => "Open",
            (FileKind::Any, true) => "Save As",
            (FileKind::ScriptSet, false) => "Load Script Set",
            (FileKind::ScriptSet, true) => "Save Script Set",
            (FileKind::SpellList, false) => "Load Class Spell List",
            (FileKind::SpellList, true) => "Save Class Spell List",
        }
    }

    /// Filter name and extensions.
    pub fn filter(self, save: bool) -> Option<(&'static str, &'static [&'static str])> {
        match (self, save) {
            (FileKind::Module, false) => Some(("Module", &["mod", "nwm"])),
            (FileKind::Module, true) => Some(("Module", &["mod"])),
            (FileKind::Erf, _) => Some(("Exported resources", &["erf"])),
            (FileKind::Any, _) => None,
            (FileKind::ScriptSet, _) => Some(("Script sets", &["ini"])),
            (FileKind::SpellList, _) => Some(("Spell lists", &["ini"])),
        }
    }
}

/// The dialogs the application opens.
pub trait Dialogs {
    /// Asks for a file to open.
    fn open_file(&mut self, kind: FileKind, start: Option<&Path>) -> Option<PathBuf>;
    /// Asks where to save; `suggested` is the proposed file (it may not exist).
    fn save_file(&mut self, kind: FileKind, suggested: Option<&Path>) -> Option<PathBuf>;
    /// Asks for a folder.
    fn pick_folder(&mut self, title: &str, start: Option<&Path>) -> Option<PathBuf>;
}

/// Dialogs that answer from a script (for tests), or cancel: each call takes
/// the last path of its list.
#[derive(Debug, Default)]
pub struct NoDialogs {
    pub open: Vec<PathBuf>,
    pub save: Vec<PathBuf>,
    pub folders: Vec<PathBuf>,
}

impl Dialogs for NoDialogs {
    fn open_file(&mut self, _kind: FileKind, _start: Option<&Path>) -> Option<PathBuf> {
        self.open.pop()
    }
    fn save_file(&mut self, _kind: FileKind, _suggested: Option<&Path>) -> Option<PathBuf> {
        self.save.pop()
    }
    fn pick_folder(&mut self, _title: &str, _start: Option<&Path>) -> Option<PathBuf> {
        self.folders.pop()
    }
}
