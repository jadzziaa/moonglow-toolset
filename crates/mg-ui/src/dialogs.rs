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
    /// A variable set: `.vars.json`.
    VariableSet,
    /// A class spell list: `.ini`.
    SpellList,
    /// A conversation as text, CSV, Twine (Twee) or Ink.
    Conversation(mg_module::dialog_io::Format),
    /// A story to read as a conversation: Twine (Twee) or Ink.
    Story,
    /// A hak (or other ERF archive) for the hak editor.
    Hak,
    /// Haks and a talk table to attach to the module.
    Content,
    /// Files to add to a hak.
    HakFiles,
    /// A picture: `.png`.
    Png,
    /// Pictures to add to a module or a hak: the types the game reads.
    Images,
    /// A tileset: `.set`.
    Tileset,
    /// A plugin's archive: `.zip`.
    Plugin,
    /// The log, as text: `.txt`.
    Log,
    /// A 2DA table: `.2da`.
    Table,
    /// A talk table: `.tlk`.
    TalkTable,
    /// A talk table's lines, as CSV or (`json`) as `nwn_tlk`'s JSON.
    TalkLines { json: bool },
}

impl FileKind {
    pub fn title(self, save: bool) -> &'static str {
        match (self, save) {
            // (The file dialog picks no folder: said where it is seen.)
            (FileKind::Module, false) => "Open Module (a module folder: File › Open Folder…)",
            (FileKind::Module, true) => "Save Module As",
            (FileKind::Erf, false) => "Import",
            (FileKind::Erf, true) => "Export",
            (FileKind::Any, false) => "Open",
            (FileKind::Any, true) => "Save As",
            (FileKind::ScriptSet, false) => "Load Script Set",
            (FileKind::ScriptSet, true) => "Save Script Set",
            (FileKind::VariableSet, false) => "Add Variable Set from File",
            (FileKind::VariableSet, true) => "Save Variable Set to File",
            (FileKind::SpellList, false) => "Load Class Spell List",
            (FileKind::SpellList, true) => "Save Class Spell List",
            (FileKind::Conversation(_), false) => "Import Lines",
            (FileKind::Conversation(_), true) => "Export Conversation",
            (FileKind::Story, _) => "Import Conversation",
            (FileKind::Hak, false) => "Open Hak",
            (FileKind::Hak, true) => "Save Hak As",
            (FileKind::Content, _) => "Add Haks and Talk Table",
            (FileKind::HakFiles, _) => "Add Files to the Hak",
            (FileKind::Png, _) => "Save Picture",
            (FileKind::Images, _) => "Add Images",
            (FileKind::Tileset, false) => "Open Tileset",
            (FileKind::Tileset, true) => "New Tileset",
            (FileKind::Plugin, _) => "Install Plugin from File",
            (FileKind::Log, _) => "Save Log",
            (FileKind::Table, false) => "Open 2DA",
            (FileKind::Table, true) => "Save 2DA As",
            (FileKind::TalkTable, false) => "Open Talk Table",
            (FileKind::TalkTable, true) => "New Talk Table",
            (FileKind::TalkLines { .. }, false) => "Import Lines",
            (FileKind::TalkLines { .. }, true) => "Export Lines",
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
            // (A filter matches the last extension: .vars.json ends in json.)
            (FileKind::VariableSet, _) => Some(("Variable sets", &["json"])),
            (FileKind::SpellList, _) => Some(("Spell lists", &["ini"])),
            (FileKind::Conversation(f), _) => Some((f.name(), f.extensions())),
            (FileKind::Story, _) => Some(("Twine or Ink stories", &["twee", "tw", "ink"])),
            (FileKind::Hak, false) => Some(("Haks and archives", &["hak", "erf"])),
            (FileKind::Hak, true) => Some(("Hak", &["hak"])),
            (FileKind::Content, _) => {
                Some(("Haks, talk tables and zips of them", &["hak", "tlk", "zip"]))
            }
            (FileKind::HakFiles, _) => None,
            (FileKind::Png, _) => Some(("PNG picture", &["png"])),
            (FileKind::Images, _) => Some(("Pictures", &["png", "tga", "dds", "jpg", "bmp"])),
            (FileKind::Tileset, _) => Some(("Tileset", &["set"])),
            (FileKind::Plugin, _) => Some(("Plugin archives", &["zip"])),
            (FileKind::Log, _) => Some(("Text", &["txt", "log"])),
            (FileKind::Table, _) => Some(("2DA tables", &["2da"])),
            (FileKind::TalkTable, _) => Some(("Talk tables", &["tlk"])),
            (FileKind::TalkLines { json: false }, _) => Some(("CSV", &["csv"])),
            (FileKind::TalkLines { json: true }, _) => Some(("JSON", &["json"])),
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
    /// Asks for a file of any of `extensions` (none: any file), under
    /// `title`: what a plugin asks for.
    fn open_file_of(&mut self, _title: &str, _extensions: &[String]) -> Option<PathBuf> {
        self.open_file(FileKind::Any, None)
    }
    /// Asks for files to open (none: cancelled).
    fn open_files(&mut self, kind: FileKind, start: Option<&Path>) -> Vec<PathBuf> {
        self.open_file(kind, start).into_iter().collect()
    }
}

/// Dialogs that answer from a script (for tests), or cancel: each call takes
/// the last path of its list.
#[derive(Debug, Default)]
pub struct NoDialogs {
    pub open: Vec<PathBuf>,
    pub save: Vec<PathBuf>,
    pub folders: Vec<PathBuf>,
    /// Answers to [`Dialogs::open_files`], the last first.
    pub open_many: Vec<Vec<PathBuf>>,
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
    fn open_files(&mut self, _kind: FileKind, _start: Option<&Path>) -> Vec<PathBuf> {
        self.open_many.pop().unwrap_or_default()
    }
}
