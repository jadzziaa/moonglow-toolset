//! What Moonglow keeps under a name in its data folder, for any module:
//! variable sets, parameter sets, prefabs and lighting. Each kind is a folder of small
//! JSON files, one a name; this is where they are listed, read and
//! written (each kind's own module says what a file holds).

use std::path::{Path, PathBuf};

use crate::recovery;

/// Writes `text` to `path` whole or not at all (`mg_module::write_replacing`:
/// beside it, then put in its place, its folder made where it is not
/// there): a set, a script set or a spell list written in place was cut
/// short by a full disk or a crash, where a whole one had stood.
pub(crate) fn write_whole(path: &Path, text: &str) -> Result<(), String> {
    mg_module::write_replacing(path, text.as_bytes())
        .map_err(|e| format!("{}: {e}", path.display()))
}

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
        write_whole(&path, text)?;
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
        write_whole(&path, text)
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

    /// The ones kept in `dir`, by name, each read by `load` (from `dir`
    /// and its name): what it holds, or why its file isn't one (bad JSON,
    /// an unknown type), which a menu or window says beside its name. A
    /// file is read again when its size or modified time changes
    /// (`kept`, from the last call: a window lists them every frame).
    pub fn read_all<T: Clone>(
        &self,
        dir: Option<&Path>,
        kept: &mut Read<T>,
        load: impl Fn(&Path, &str) -> Result<T, String>,
    ) -> Vec<(String, Result<T, String>)> {
        let names = self.list(dir);
        let mut out = Vec::with_capacity(names.len());
        let mut now = Read::default();
        for name in names {
            let Ok(path) = self.file(dir, &name) else { continue };
            let stamp = std::fs::metadata(&path).ok().map(|m| (m.len(), m.modified().ok()));
            let read = match kept.0.remove(&path) {
                Some((was, read)) if was == stamp => read,
                _ => load(dir.expect("listed"), &name),
            };
            now.0.insert(path, (stamp, read.clone()));
            out.push((name, read));
        }
        *kept = now;
        out
    }
}

/// A file's size and modified time, as [`Shelf::read_all`] saw them.
type Stamp = Option<(u64, Option<std::time::SystemTime>)>;

/// What [`Shelf::read_all`] last read, by file.
#[derive(Debug, Clone)]
pub struct Read<T>(pub(crate) std::collections::HashMap<PathBuf, (Stamp, Result<T, String>)>);

impl<T> Default for Read<T> {
    fn default() -> Self {
        Read(Default::default())
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

    /// A name written over is written whole beside the file it replaces,
    /// then put in its place (the review's B30: written in place, a full
    /// disk or a crash left it cut short where a whole one had stood). So
    /// another name for the old file still has the old text, and nothing
    /// of the writing is left in the folder.
    #[test]
    fn a_name_written_over_replaces_its_file_and_leaves_no_other() {
        let dir = mg_testkit::scratch_dir("ui-saved-whole");
        let shelf = Shelf { folder: "things", suffix: ".thing.json", what: "thing" };
        let at = Some(dir.as_path());
        let file = shelf.write(at, "Loot", "the first, a long one").unwrap();
        #[cfg(unix)]
        std::fs::hard_link(&file, dir.join("as-it-was")).unwrap();
        assert_eq!(shelf.write(at, "Loot", "the second").unwrap(), file);
        assert_eq!(shelf.read(at, "Loot").unwrap(), "the second");
        #[cfg(unix)]
        {
            let was = std::fs::read_to_string(dir.join("as-it-was")).unwrap();
            assert_eq!(was, "the first, a long one", "not written into");
            std::fs::remove_file(dir.join("as-it-was")).unwrap();
        }
        let left: Vec<_> =
            std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(left, ["Loot.thing.json"], "no temporary file");
        // A file anywhere (Save to File…), its folder made.
        let far = dir.join("made").join("set.json");
        write_whole(&far, "{}").unwrap();
        assert_eq!(std::fs::read_to_string(&far).unwrap(), "{}");
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
