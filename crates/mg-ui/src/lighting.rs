//! Lighting kept under a name ([`mg_module::lighting`], which says what a
//! file holds): listed, kept, given, renamed and deleted from an area's
//! Visual page, in Moonglow's data folder (`lighting`).

use std::path::{Path, PathBuf};

use mg_gff::Struct;
use mg_module::lighting::{FOLDER, SUFFIX, Saved};

use crate::saved::Shelf;

const SHELF: Shelf = Shelf { folder: FOLDER, suffix: SUFFIX, what: "lighting" };

/// Where lighting is kept.
pub fn dir() -> Option<PathBuf> {
    SHELF.dir()
}

/// The lighting kept in `dir`, by name.
pub fn list(dir: Option<&Path>) -> Vec<String> {
    SHELF.list(dir)
}

/// Keeps an area's lighting (its root, an ARE's) as `name`.
pub fn save(dir: Option<&Path>, name: &str, are: &Struct) -> Result<PathBuf, String> {
    SHELF.write(dir, name, &Saved::of_area(are).to_text())
}

/// The lighting named `name`.
pub fn read(dir: Option<&Path>, name: &str) -> Result<Saved, String> {
    Saved::parse(&SHELF.read(dir, name)?).map_err(|e| format!("{name}: {e}"))
}

/// Gives the one named `from` another name ([`Shelf::rename`]).
pub fn rename(dir: Option<&Path>, from: &str, to: &str) -> Result<(), String> {
    SHELF.rename(dir, from, to)
}

/// Deletes the one named `name`: its text, to put it back with
/// [`restore`].
pub fn delete(dir: Option<&Path>, name: &str) -> Result<String, String> {
    SHELF.delete(dir, name)
}

/// Puts a deleted one back, unless one of its name was kept since.
pub fn restore(dir: Option<&Path>, name: &str, text: &str) -> Result<(), String> {
    SHELF.restore(dir, name, text)
}
