//! A module's custom talk table: where the game finds it. The game looks in
//! the module's haks, then the module, then the user's `tlk/` folder
//! (`tests/engine_tlk.rs`; the wiki says haks don't count, but they do), and
//! looks up the feminine table, `<name>f`, the same way on its own. A file in
//! the folder must match the name's case on Linux. A name the game can't
//! find stops the module from loading.
//!
//! [`Table`] is a talk table open in the editor, with undo.

use std::path::{Path, PathBuf};

use mg_core::{Language, ResType, StrRef};
use mg_resman::{ResKey, ResMan};
use mg_tlk::{FLAG_SOUND, FLAG_SOUND_LENGTH, FLAG_TEXT, Tlk, TlkEntry};

/// Where a talk table was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A layer of the resource manager: a hak or the module, by its label.
    Layer(String),
    /// A file in a `tlk/` folder.
    File(PathBuf),
}

impl Source {
    /// The file, when the table is one (and so can be edited in place).
    pub fn file(&self) -> Option<&Path> {
        match self {
            Source::File(p) => Some(p),
            Source::Layer(_) => None,
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Layer(l) => f.write_str(l),
            Source::File(p) => write!(f, "{}", p.display()),
        }
    }
}

/// A talk table found where the game looks, still unparsed.
#[derive(Debug, Clone)]
pub struct Found {
    pub source: Source,
    pub data: Vec<u8>,
}

/// The feminine table's name for a table.
pub fn feminine(name: &str) -> String {
    format!("{name}f")
}

/// Finds the talk table `name` (as `Mod_CustomTlk` names it, without
/// `.tlk`) as the game does: in `rm` (which has the module's haks above the
/// module), then in `tlk_dirs` (the user's `tlk/` first).
pub fn find(rm: &ResMan, tlk_dirs: &[PathBuf], name: &str) -> Option<Found> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    if let Some(key) = ResKey::parse(name, ResType::TLK)
        && let Ok(data) = rm.get(&key)
    {
        let label = rm.origin(&key).unwrap_or("?").to_string();
        return Some(Found { source: Source::Layer(label), data: data.into_owned() });
    }
    tlk_dirs.iter().find_map(|d| {
        let path = d.join(format!("{name}.tlk"));
        let data = std::fs::read(&path).ok()?;
        Some(Found { source: Source::File(path), data })
    })
}

/// Why the game can't find a table of this name, beyond its not being
/// there: a hint for the content doctor.
pub fn name_hint(name: &str) -> Option<&'static str> {
    if name.to_ascii_lowercase().ends_with(".tlk") {
        Some("name it without .tlk")
    } else if name.len() > 16 {
        Some("a name has at most 16 characters")
    } else if name != name.to_ascii_lowercase() {
        Some("on Linux the file name must match its case exactly; use lower case")
    } else {
        None
    }
}

/// One line of a talk table as the editor shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    /// The feminine table's text, when there is one.
    pub feminine: Option<String>,
    /// The sound's resref, or empty.
    pub sound: String,
    /// The sound's length in seconds, or 0.
    pub sound_length: f32,
}

/// The entries of a line, before or after a change: the table's and the
/// feminine table's (`None` where the line isn't there).
type Entries = (Option<TlkEntry>, Option<TlkEntry>);

#[derive(Debug, Clone)]
struct Change {
    row: usize,
    before: Entries,
    after: Entries,
    /// Typing into a field of a line (the caller's number for it): the
    /// next such change to the same field merges into this one.
    typing: Option<u8>,
}

/// A talk table open for editing: its file (and the feminine table's, if
/// any), with undo. Tables found in a hak or the module are read-only.
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub source: Source,
    pub tlk: Tlk,
    /// The feminine table (`<name>f`) and where it is.
    pub feminine: Option<(Source, Tlk)>,
    undo: Vec<Change>,
    redo: Vec<Change>,
    /// The undo depth when last saved; `None` when that state is gone.
    saved: Option<usize>,
}

/// Talk tables are numbered from here in the game's StrRefs.
pub const CUSTOM: u32 = StrRef::CUSTOM_FLAG;

impl Table {
    /// Opens a table found where the game looks.
    pub fn open(name: &str, found: Found, feminine: Option<Found>) -> Result<Table, String> {
        let read = |f: &Found| Tlk::read(&f.data).map_err(|e| format!("{}: {e}", f.source));
        let tlk = read(&found)?;
        let feminine = match feminine {
            Some(f) => Some((f.source.clone(), read(&f)?)),
            None => None,
        };
        Ok(Table {
            name: name.to_string(),
            source: found.source,
            tlk,
            feminine,
            undo: Vec::new(),
            redo: Vec::new(),
            saved: Some(0),
        })
    }

    /// A new, empty table in `dir` (the user's `tlk/` folder), not yet
    /// saved.
    pub fn create(name: &str, dir: &Path, language: Language, feminine: bool) -> Table {
        let file = |n: &str| Source::File(dir.join(format!("{n}.tlk")));
        Table {
            name: name.to_string(),
            source: file(name),
            tlk: Tlk::new(language),
            feminine: feminine.then(|| (file(&self::feminine(name)), Tlk::new(language))),
            undo: Vec::new(),
            redo: Vec::new(),
            saved: None,
        }
    }

    /// Whether it can be saved: a file, not part of a hak or the module.
    pub fn editable(&self) -> bool {
        self.source.file().is_some() && self.feminine.as_ref().is_none_or(|f| f.0.file().is_some())
    }

    pub fn len(&self) -> usize {
        self.tlk.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tlk.entries.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.undo.len())
    }

    /// The StrRef the game knows a line by.
    pub fn strref(row: usize) -> StrRef {
        StrRef(CUSTOM + row as u32)
    }

    fn decode(&self, e: Option<&TlkEntry>) -> String {
        e.filter(|e| e.flags & FLAG_TEXT != 0)
            .map(|e| self.tlk.language.codepage().decode(&e.text).into_owned())
            .unwrap_or_default()
    }

    /// A line as text.
    pub fn line(&self, row: usize) -> Line {
        let e = self.tlk.entries.get(row);
        Line {
            text: self.decode(e),
            feminine: self.feminine.as_ref().map(|(_, f)| self.decode(f.entries.get(row))),
            sound: e
                .filter(|e| e.flags & FLAG_SOUND != 0)
                .map(|e| String::from_utf8_lossy(&e.sound).trim_end_matches('\0').to_string())
                .unwrap_or_default(),
            sound_length: e
                .filter(|e| e.flags & FLAG_SOUND_LENGTH != 0)
                .map_or(0.0, |e| e.sound_length),
        }
    }

    /// The entries a line becomes; an error names what can't be written.
    fn entries(&self, line: &Line, row: usize) -> Result<Entries, String> {
        let codepage = self.tlk.language.codepage();
        let encode = |t: &str| {
            codepage.encode(t).map(|b| b.into_owned()).ok_or_else(|| {
                format!("the text has characters the table's language can't write ({codepage:?})")
            })
        };
        let sound = line.sound.trim().to_ascii_lowercase();
        if sound.len() > 16 || !sound.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err("a sound's name has at most 16 letters, digits and _".into());
        }
        let entry = |text: &str, old: Option<&TlkEntry>| -> Result<TlkEntry, String> {
            let mut e = old.cloned().unwrap_or_default();
            e.text = encode(text)?;
            e.sound = sound.as_bytes().to_vec();
            e.sound_length = line.sound_length.max(0.0);
            let flag = |on: bool, f: u32, flags: u32| if on { flags | f } else { flags & !f };
            e.flags = flag(!text.is_empty(), FLAG_TEXT, e.flags);
            e.flags = flag(!sound.is_empty(), FLAG_SOUND, e.flags);
            e.flags = flag(e.sound_length > 0.0, FLAG_SOUND_LENGTH, e.flags);
            Ok(e)
        };
        let masculine = entry(&line.text, self.tlk.entries.get(row))?;
        let feminine = match &self.feminine {
            Some((_, f)) => {
                let text = line.feminine.as_deref().unwrap_or(&line.text);
                Some(entry(text, f.entries.get(row))?)
            }
            None => None,
        };
        Ok((Some(masculine), feminine))
    }

    fn current(&self, row: usize) -> Entries {
        (
            self.tlk.entries.get(row).cloned(),
            self.feminine.as_ref().and_then(|(_, f)| f.entries.get(row).cloned()),
        )
    }

    /// Puts entries at a row: `None` removes the row (the last one), and
    /// the feminine table is padded with copies of the table's lines to
    /// reach it.
    fn put(&mut self, row: usize, (m, f): Entries) {
        match m {
            Some(m) if row < self.tlk.entries.len() => self.tlk.entries[row] = m,
            Some(m) => self.tlk.entries.push(m),
            None => self.tlk.entries.truncate(row),
        }
        if let Some((_, fem)) = &mut self.feminine {
            match f {
                Some(f) => {
                    while fem.entries.len() < row {
                        fem.entries.push(self.tlk.entries[fem.entries.len()].clone());
                    }
                    if row < fem.entries.len() {
                        fem.entries[row] = f;
                    } else {
                        fem.entries.push(f);
                    }
                }
                None => fem.entries.truncate(row),
            }
        }
    }

    fn apply(&mut self, change: Change) {
        self.put(change.row, change.after.clone());
        if self.saved.is_some_and(|s| s > self.undo.len()) {
            self.saved = None;
        }
        self.undo.push(change);
        self.redo.clear();
    }

    /// Changes a line. `typing` (a field's number) merges it with the
    /// previous change to the same field of the line made by typing, so one
    /// undo takes back a burst of typing.
    pub fn set_line(&mut self, row: usize, line: &Line, typing: Option<u8>) -> Result<(), String> {
        if row >= self.len() {
            return Err(format!("there is no line {row}"));
        }
        let after = self.entries(line, row)?;
        if after == self.current(row) {
            return Ok(());
        }
        let depth = self.undo.len();
        if let Some(last) = self.undo.last_mut()
            && typing.is_some()
            && last.typing == typing
            && last.row == row
            && self.saved != Some(depth)
        {
            last.after = after.clone();
            self.redo.clear();
            self.put(row, after);
            return Ok(());
        }
        let before = self.current(row);
        self.apply(Change { row, before, after, typing });
        Ok(())
    }

    /// Adds a line at the end; returns its row.
    pub fn add_line(&mut self, line: &Line) -> Result<usize, String> {
        let row = self.len();
        let after = self.entries(line, row)?;
        self.apply(Change { row, before: (None, None), after, typing: None });
        Ok(row)
    }

    /// Removes the last line (only the last: removing another would
    /// renumber the lines after it).
    pub fn remove_last(&mut self) {
        let Some(row) = self.len().checked_sub(1) else { return };
        let before = self.current(row);
        self.apply(Change { row, before, after: (None, None), typing: None });
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(c) = self.undo.pop() {
            self.put(c.row, c.before.clone());
            self.redo.push(c);
        }
    }

    pub fn redo(&mut self) {
        if let Some(c) = self.redo.pop() {
            self.put(c.row, c.after.clone());
            self.undo.push(c);
        }
    }

    /// The lines whose text (either table's) has every word, ignoring case,
    /// or whose StrRef or row number is the query.
    pub fn find(&self, query: &str) -> Vec<usize> {
        let query = query.trim();
        let number: Option<u64> = query.parse().ok();
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        (0..self.len())
            .filter(|&row| {
                if let Some(n) = number
                    && (n == row as u64 || n == u64::from(Self::strref(row).0))
                {
                    return true;
                }
                let l = self.line(row);
                let text = format!("{}\n{}", l.text, l.feminine.unwrap_or_default()).to_lowercase();
                words.iter().all(|w| text.contains(w.as_str()))
            })
            .collect()
    }

    /// Writes the table (and the feminine table) to their files.
    pub fn save(&mut self) -> Result<(), String> {
        let write = |source: &Source, tlk: &Tlk| -> Result<(), String> {
            let path = source.file().ok_or_else(|| format!("{source} is read-only here"))?;
            let data = tlk.to_bytes().map_err(|e| e.to_string())?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            // Written beside it, then moved over it: never half a table.
            let tmp = path.with_extension("tlk.part");
            std::fs::write(&tmp, data)
                .and_then(|()| std::fs::rename(&tmp, path))
                .map_err(|e| format!("{}: {e}", path.display()))
        };
        if let Some((source, fem)) = &mut self.feminine {
            while fem.entries.len() < self.tlk.entries.len() {
                fem.entries.push(self.tlk.entries[fem.entries.len()].clone());
            }
            write(source, fem)?;
        }
        write(&self.source, &self.tlk)?;
        self.saved = Some(self.undo.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_resman::{LayerClass, MemContainer, priority};

    #[test]
    fn haks_then_the_module_then_the_folder() {
        let dir = std::env::temp_dir().join(format!("mg-talk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mine.tlk"), b"folder").unwrap();
        std::fs::write(dir.join("minef.tlk"), b"folder f").unwrap();
        let key = ResKey::parse("mine", ResType::TLK).unwrap();
        let mut rm = ResMan::new();
        let dirs = [dir.clone()];
        let got = |rm: &ResMan, name: &str| find(rm, &dirs, name).map(|f| (f.source, f.data));
        assert_eq!(
            got(&rm, "mine"),
            Some((Source::File(dir.join("mine.tlk")), b"folder".to_vec()))
        );
        let mut module = MemContainer::new();
        module.insert(key, b"module".to_vec());
        rm.add(priority::MODULE, "module", LayerClass::Erf, module);
        assert_eq!(got(&rm, "mine"), Some((Source::Layer("module".into()), b"module".to_vec())));
        let mut hak = MemContainer::new();
        hak.insert(key, b"hak".to_vec());
        rm.add(priority::HAK_USER, "my.hak", LayerClass::Erf, hak);
        assert_eq!(got(&rm, "mine"), Some((Source::Layer("my.hak".into()), b"hak".to_vec())));
        // The feminine table is looked up on its own.
        assert_eq!(got(&rm, &feminine("mine")).unwrap().1, b"folder f");
        // Names the game doesn't find.
        assert_eq!(got(&rm, "mine.tlk"), None);
        assert_eq!(got(&rm, ""), None);
        assert_eq!(name_hint("mine.tlk"), Some("name it without .tlk"));
        assert!(name_hint("Mine").is_some());
        assert_eq!(name_hint("mine"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn line(text: &str) -> Line {
        Line { text: text.into(), feminine: None, sound: String::new(), sound_length: 0.0 }
    }

    #[test]
    fn a_new_table_edited_undone_and_saved() {
        let dir = std::env::temp_dir().join(format!("mg-talk-edit-{}", std::process::id()));
        let mut t = Table::create("mine", &dir, Language::ENGLISH, true);
        assert!(t.editable() && t.is_dirty());
        assert_eq!(t.add_line(&line("Hello")).unwrap(), 0);
        let mut l = line("Good day");
        l.feminine = Some("Good day, lady".into());
        l.sound = "VS_Hello".into();
        l.sound_length = 1.5;
        assert_eq!(t.add_line(&l).unwrap(), 1);
        assert_eq!(Table::strref(1).0, 16_777_217);
        // Typing merges into one change.
        for text in ["H", "Hi", "Hi!"] {
            t.set_line(0, &line(text), Some(0)).unwrap();
        }
        assert_eq!(t.line(0).text, "Hi!");
        // The feminine line follows the masculine one when not given.
        assert_eq!(t.line(0).feminine.as_deref(), Some("Hi!"));
        t.undo();
        assert_eq!(t.line(0).text, "Hello");
        t.redo();
        assert_eq!(t.find("lady"), [1]);
        assert_eq!(t.find("16777216"), [0]);
        assert!(t.set_line(0, &line("Ω"), None).is_err());
        assert!(t.set_line(0, &Line { sound: "a sound".into(), ..line("x") }, None).is_err());
        t.save().unwrap();
        assert!(!t.is_dirty());

        let rm = ResMan::new();
        let dirs = [dir.clone()];
        let found = find(&rm, &dirs, "mine").unwrap();
        let back = Table::open("mine", found, find(&rm, &dirs, &feminine("mine"))).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back.line(1), Line { sound: "vs_hello".into(), ..l });
        // The game reads text only with its flag set (tests/engine_tlk.rs).
        assert_eq!(back.tlk.entries[1].flags, FLAG_TEXT | FLAG_SOUND | FLAG_SOUND_LENGTH);
        let mut back = back;
        back.set_line(0, &line(""), None).unwrap();
        assert_eq!(back.tlk.entries[0].flags, 0);
        back.remove_last();
        assert_eq!((back.len(), back.feminine.as_ref().unwrap().1.entries.len()), (1, 1));
        back.undo();
        back.undo();
        assert!(!back.is_dirty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
