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

    /// Deletes the one named `name`: its text, to put it back with
    /// [`Shelf::restore`].
    pub fn delete(&self, dir: Option<&Path>, name: &str) -> Result<String, String> {
        let path = self.file(dir, name)?;
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(text)
    }

    /// Puts a deleted one back, unless one of its name was kept since.
    pub fn restore(&self, dir: Option<&Path>, name: &str, text: &str) -> Result<(), String> {
        let path = self.file(dir, name)?;
        if path.exists() {
            return Err(format!("there is a {} named '{}' again", self.what, name.trim()));
        }
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Gives the one named `from` another name. A name another has is
    /// refused, as is one that names no file.
    pub fn rename(&self, dir: Option<&Path>, from: &str, to: &str) -> Result<(), String> {
        let (old, new) = (self.file(dir, from)?, self.file(dir, to)?);
        if from.trim() == to.trim() {
            return Ok(());
        }
        // Another's name in any case: the same file where names ignore case
        // (Windows, macOS). A change of case alone is the same file there.
        let (from, to) = (from.trim(), to.trim());
        let taken = self.list(dir).iter().any(|n| n != from && n.eq_ignore_ascii_case(to));
        if taken || (new.exists() && !from.eq_ignore_ascii_case(to)) {
            return Err(format!("there is a {} named '{to}' already", self.what));
        }
        std::fs::rename(&old, &new).map_err(|e| format!("{}: {e}", old.display()))
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

    #[test]
    fn what_is_kept_is_renamed_and_deleted_and_put_back() {
        let dir = mg_testkit::scratch_dir("ui-saved-rename");
        let shelf = Shelf { folder: "things", suffix: ".thing.json", what: "thing" };
        let dir = Some(dir.as_path());
        shelf.write(dir, "Loot", "1").unwrap();
        shelf.write(dir, "Doors", "2").unwrap();
        // Renamed: the file moves, its text with it.
        shelf.rename(dir, "Loot", "Treasure").unwrap();
        assert_eq!(shelf.list(dir), ["Doors", "Treasure"]);
        assert_eq!(shelf.read(dir, "Treasure").unwrap(), "1");
        // Refused: another's name (in any case), names that aren't a file's,
        // and one that isn't there.
        let taken = shelf.rename(dir, "Treasure", "doors").unwrap_err();
        assert!(taken.contains("there is a thing named 'doors' already"), "{taken}");
        for bad in ["", "  ", "../out", ".hidden", "a/b", "a\\b", "x:y", &"n".repeat(65)] {
            assert!(shelf.rename(dir, "Treasure", bad).is_err(), "{bad:?}");
        }
        assert!(shelf.rename(dir, "Nothing", "Something").is_err(), "not there");
        assert_eq!(shelf.list(dir), ["Doors", "Treasure"], "nothing moved");
        // A change of case alone, and the same name, are allowed.
        shelf.rename(dir, "Treasure", "treasure").unwrap();
        assert_eq!(shelf.list(dir), ["Doors", "treasure"]);
        shelf.rename(dir, "Doors", " Doors ").unwrap();
        // Deleted, and put back from its text.
        let text = shelf.delete(dir, "Doors").unwrap();
        assert_eq!(text, "2");
        assert_eq!(shelf.list(dir), ["treasure"]);
        assert!(shelf.delete(dir, "Doors").is_err(), "gone already");
        shelf.restore(dir, "Doors", &text).unwrap();
        assert_eq!(shelf.read(dir, "Doors").unwrap(), "2");
        // Not over one kept under its name since.
        shelf.delete(dir, "Doors").unwrap();
        shelf.write(dir, "Doors", "3").unwrap();
        assert!(shelf.restore(dir, "Doors", &text).unwrap_err().contains("again"));
        assert_eq!(shelf.read(dir, "Doors").unwrap(), "3");
        assert!(shelf.delete(None, "Doors").is_err(), "no data folder");
    }
}
