//! What Moonglow keeps under a name in its data folder, for any module:
//! variable sets, prefabs and lighting. Each kind is a folder of small
//! JSON files, one a name; this is where they are listed, read and
//! written (each kind's own module says what a file holds).

use std::path::{Path, PathBuf};

use crate::recovery;

/// A kind of thing kept under a name: its folder and its files' ending.
#[derive(Debug, Clone, Copy)]
pub struct Shelf {
    pub folder: &'static str,
    pub suffix: &'static str,
    /// What one is called, in messages ("prefab").
    pub what: &'static str,
}

impl Shelf {
    /// Its folder in Moonglow's data folder.
    pub fn dir(&self) -> Option<PathBuf> {
        recovery::data_dir().map(|d| d.join(self.folder))
    }

    /// The names kept in `dir`, in alphabetical order.
    pub fn list(&self, dir: Option<&Path>) -> Vec<String> {
        let Some(dir) = dir else { return Vec::new() };
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().to_str()?.strip_suffix(self.suffix).map(str::to_string))
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names
    }

    /// The file of the one named `name`.
    pub fn file(&self, dir: Option<&Path>, name: &str) -> Result<PathBuf, String> {
        let dir = dir.ok_or("Moonglow has no data folder")?;
        if !valid_name(name) {
            return Err(format!("'{name}' is not a {}'s name", self.what));
        }
        Ok(dir.join(format!("{}{}", name.trim(), self.suffix)))
    }

    /// Keeps `text` as the one named `name` (in place of one of that name).
    pub fn write(&self, dir: Option<&Path>, name: &str, text: &str) -> Result<PathBuf, String> {
        let path = self.file(dir, name)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }

    /// The text of the one named `name`.
    pub fn read(&self, dir: Option<&Path>, name: &str) -> Result<String, String> {
        let path = self.file(dir, name)?;
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// Whether a name can be kept under (it names a file).
pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
        && !name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_kept_is_listed_by_name_and_read_back() {
        let dir = mg_testkit::scratch_dir("ui-saved");
        let shelf = Shelf { folder: "things", suffix: ".thing.json", what: "thing" };
        let dir = Some(dir.as_path());
        assert!(shelf.list(dir).is_empty());
        shelf.write(dir, "beta", "{}").unwrap();
        shelf.write(dir, "Alpha", "[]").unwrap();
        assert_eq!(shelf.list(dir), ["Alpha", "beta"]);
        assert_eq!(shelf.read(dir, "Alpha").unwrap(), "[]");
        assert!(shelf.write(dir, "../out", "x").is_err(), "not a name");
        assert!(shelf.read(None, "beta").is_err(), "no data folder");
    }
}
