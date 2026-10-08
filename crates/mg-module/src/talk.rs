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
    /// Changes made as one (an import's): undone and redone together.
    /// 0: on its own.
    batch: u64,
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
    /// The game's codepage, where the open module has its own table
    /// (`encoding.2da`): the game reads the talk tables by it too.
    pub game: Option<mg_core::Codepage>,
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
            game: None,
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
            game: None,
        }
    }

    /// The codepage of the table's text.
    fn codepage(&self) -> mg_core::Codepage {
        let language = self.tlk.language;
        self.game.map_or(language.codepage(), |g| g.for_language(language))
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
            .map(|e| self.codepage().decode(&e.text).into_owned())
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
        let codepage = self.codepage();
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
        self.apply(Change { row, before, after, typing, batch: 0 });
        Ok(())
    }

    /// Adds a line at the end; returns its row.
    pub fn add_line(&mut self, line: &Line) -> Result<usize, String> {
        let row = self.len();
        let after = self.entries(line, row)?;
        self.apply(Change { row, before: (None, None), after, typing: None, batch: 0 });
        Ok(row)
    }

    /// Removes the last line (only the last: removing another would
    /// renumber the lines after it).
    pub fn remove_last(&mut self) {
        let Some(row) = self.len().checked_sub(1) else { return };
        let before = self.current(row);
        self.apply(Change { row, before, after: (None, None), typing: None, batch: 0 });
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        while let Some(c) = self.undo.pop() {
            self.put(c.row, c.before.clone());
            let batch = c.batch;
            self.redo.push(c);
            // (The rest of an import with it.)
            if batch == 0 || self.undo.last().is_none_or(|next| next.batch != batch) {
                break;
            }
        }
    }

    pub fn redo(&mut self) {
        while let Some(c) = self.redo.pop() {
            self.put(c.row, c.after.clone());
            let batch = c.batch;
            self.undo.push(c);
            if batch == 0 || self.redo.last().is_none_or(|next| next.batch != batch) {
                break;
            }
        }
    }

    /// The table as CSV, for a spreadsheet or a translator: a heading row,
    /// then a row for each line: its StrRef, its text (and the feminine
    /// table's, when there is one), its sound and the sound's length.
    pub fn to_csv(&self) -> String {
        use crate::dialog_io::csv_field;
        let feminine = self.feminine.is_some();
        let mut out = String::from(if feminine {
            "StrRef,Text,Feminine,Sound,SoundLength\n"
        } else {
            "StrRef,Text,Sound,SoundLength\n"
        });
        for row in 0..self.len() {
            let line = self.line(row);
            let mut cells = vec![Table::strref(row).0.to_string(), csv_field(&line.text)];
            if feminine {
                cells.push(csv_field(line.feminine.as_deref().unwrap_or_default()));
            }
            cells.push(csv_field(&line.sound));
            let length = line.sound_length;
            cells.push(if length == 0.0 { String::new() } else { length.to_string() });
            out.push_str(&cells.join(","));
            out.push('\n');
        }
        out
    }

    /// Reads lines from CSV as [`to_csv`](Self::to_csv) writes it (its
    /// columns found by their headings, in any order; a StrRef is the
    /// game's, 16777216 and up, or the line's number in the table). A row
    /// sets the line of its StrRef, which keeps what the row has no column
    /// for; rows past the table's end add lines, empty ones between if
    /// they leave a gap. Nothing changes unless every row can be read, and
    /// one undo takes the import back. Returns how many lines changed and
    /// how many were added.
    pub fn import_csv(&mut self, text: &str) -> Result<(usize, usize), String> {
        let rows = crate::dialog_io::read_csv(text)?;
        let Some((headings, rows)) = rows.split_first() else {
            return Err("the file is empty".into());
        };
        let column = |name: &str| headings.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
        let strref = column("StrRef").ok_or("no StrRef column")?;
        let (text, feminine) = (column("Text"), column("Feminine"));
        let (sound, length) = (column("Sound"), column("SoundLength"));
        if text.is_none() && feminine.is_none() && sound.is_none() && length.is_none() {
            return Err("no Text, Feminine, Sound or SoundLength column".into());
        }
        // Each row as the line it makes of the line that is there (or of
        // an empty one), by its place in the table.
        let empty = Line {
            text: String::new(),
            feminine: self.feminine.as_ref().map(|_| String::new()),
            sound: String::new(),
            sound_length: 0.0,
        };
        let mut lines: std::collections::BTreeMap<usize, Line> = Default::default();
        for (n, cells) in rows.iter().enumerate() {
            if cells.iter().all(|c| c.trim().is_empty()) {
                continue;
            }
            let at = format!("row {}", n + 2);
            let cell = |i: Option<usize>| i.and_then(|i| cells.get(i));
            let number: u32 = cell(Some(strref))
                .and_then(|c| c.trim().parse().ok())
                .ok_or_else(|| format!("{at}: its StrRef is not a number"))?;
            let row = number.checked_sub(CUSTOM).unwrap_or(number) as usize;
            if row >= (CUSTOM as usize) {
                return Err(format!("{at}: StrRef {number} is past what a table holds"));
            }
            let mut line = match lines.remove(&row) {
                Some(line) => line,
                None if row < self.len() => self.line(row),
                None => empty.clone(),
            };
            if let Some(t) = cell(text) {
                line.text.clone_from(t);
            }
            if let (Some(t), Some(f)) = (cell(feminine), line.feminine.as_mut()) {
                f.clone_from(t);
            }
            if let Some(s) = cell(sound) {
                line.sound = s.trim().to_string();
            }
            if let Some(l) = cell(length) {
                line.sound_length = match l.trim() {
                    "" => 0.0,
                    l => l.parse().map_err(|_| format!("{at}: its SoundLength is not a number"))?,
                };
            }
            lines.insert(row, line);
        }
        self.import(lines)
    }

    /// The table as JSON, as neverwinter.nim's `nwn_tlk` writes one (and
    /// nasher keeps one in a repository): its language and the lines that
    /// have text or a sound, each with its `id` (the line's number in the
    /// table, from 0). The feminine table is a file of its own there and
    /// is left out.
    pub fn to_json(&self) -> String {
        let entries: Vec<serde_json::Value> = (0..self.len())
            .map(|row| (row, self.line(row)))
            .filter(|(_, l)| !l.text.is_empty() || !l.sound.is_empty())
            .map(|(row, l)| {
                let mut e = serde_json::Map::new();
                e.insert("id".into(), row.into());
                e.insert("text".into(), l.text.into());
                if !l.sound.is_empty() {
                    e.insert("sound".into(), l.sound.into());
                }
                if l.sound_length != 0.0 {
                    e.insert("soundLength".into(), l.sound_length.into());
                }
                serde_json::Value::Object(e)
            })
            .collect();
        let table = serde_json::json!({ "language": self.tlk.language.0, "entries": entries });
        let mut out = serde_json::to_string_pretty(&table).expect("JSON");
        out.push('\n');
        out
    }

    /// Reads lines from JSON as [`to_json`](Self::to_json) writes it. An
    /// entry sets the text of the line of its `id` (the game's StrRef,
    /// 16777216 and up, is taken too) and its sound, if it names one; the
    /// feminine text stays. Lines the file leaves out stay as they are.
    /// As with [`import_csv`](Self::import_csv): all or nothing, one undo,
    /// and the lines changed and added returned.
    pub fn import_json(&mut self, text: &str) -> Result<(usize, usize), String> {
        let json: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let entries =
            json.get("entries").and_then(|e| e.as_array()).ok_or("no \"entries\" list")?;
        let empty = Line {
            text: String::new(),
            feminine: self.feminine.as_ref().map(|_| String::new()),
            sound: String::new(),
            sound_length: 0.0,
        };
        let mut lines: std::collections::BTreeMap<usize, Line> = Default::default();
        for (n, e) in entries.iter().enumerate() {
            let at = format!("entry {}", n + 1);
            let number = e
                .get("id")
                .and_then(serde_json::Value::as_u64)
                .and_then(|id| u32::try_from(id).ok())
                .ok_or_else(|| format!("{at}: its id is not a number"))?;
            let row = number.checked_sub(CUSTOM).unwrap_or(number) as usize;
            if row >= (CUSTOM as usize) {
                return Err(format!("{at}: id {number} is past what a table holds"));
            }
            let mut line = match lines.remove(&row) {
                Some(line) => line,
                None if row < self.len() => self.line(row),
                None => empty.clone(),
            };
            if let Some(t) = e.get("text") {
                line.text = t.as_str().ok_or_else(|| format!("{at}: its text is not text"))?.into();
            }
            if let Some(s) = e.get("sound").and_then(|s| s.as_str()) {
                line.sound = s.trim().to_string();
            }
            if let Some(l) = e.get("soundLength").and_then(serde_json::Value::as_f64) {
                line.sound_length = l as f32;
            }
            lines.insert(row, line);
        }
        self.import(lines)
    }

    /// Sets the lines given by their places in the table as one undoable
    /// step, adding lines (empty ones between) for those past its end:
    /// all, or none if one is not a line the table can hold. Returns how
    /// many changed and how many were added.
    fn import(
        &mut self,
        lines: std::collections::BTreeMap<usize, Line>,
    ) -> Result<(usize, usize), String> {
        let empty = Line {
            text: String::new(),
            feminine: self.feminine.as_ref().map(|_| String::new()),
            sound: String::new(),
            sound_length: 0.0,
        };
        // Every line must be one the table can hold, before any is set.
        for (row, line) in &lines {
            self.entries(line, *row)
                .map_err(|e| format!("StrRef {}: {e}", CUSTOM as usize + row))?;
        }
        let batch = self.undo.last().map_or(1, |c| c.batch + 1).max(1);
        let from = self.undo.len();
        let (mut changed, mut added) = (0, 0);
        for (row, line) in lines {
            while self.len() < row {
                self.add_line(&empty)?;
            }
            if row < self.len() {
                let before = self.undo.len();
                self.set_line(row, &line, None)?;
                changed += usize::from(self.undo.len() > before);
            } else {
                self.add_line(&line)?;
                added += 1;
            }
        }
        for c in &mut self.undo[from..] {
            c.batch = batch;
        }
        Ok((changed, added))
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
    fn csv_goes_out_and_comes_back_as_one_undoable_import() {
        let dir = std::env::temp_dir();
        let mut t = Table::create("csv", &dir, Language::ENGLISH, true);
        let line = |text: &str, feminine: &str, sound: &str| Line {
            text: text.into(),
            feminine: Some(feminine.into()),
            sound: sound.into(),
            sound_length: 0.0,
        };
        t.add_line(&line("Hello, \"friend\"", "Hello, \"sister\"", "vs_hello")).unwrap();
        t.add_line(&line("Two\nlines", "", "")).unwrap();
        let csv = t.to_csv();
        assert!(csv.starts_with("StrRef,Text,Feminine,Sound,SoundLength\n16777216,"), "{csv}");
        assert!(csv.contains("\"Hello, \"\"friend\"\"\""), "{csv}");
        // Read back as it is: nothing changes.
        assert_eq!(t.import_csv(&csv), Ok((0, 0)));

        // A translator's sheet: the text of line 1 changed, line 3 added
        // (line 2 left out: an empty one between), by StrRef or by number.
        let sheet = "Text,StrRef\nZwei Zeilen,16777217\nNeu,3\n";
        let depth = t.undo.len();
        assert_eq!(t.import_csv(sheet), Ok((1, 1)));
        assert_eq!(t.len(), 4);
        assert_eq!(t.line(1).text, "Zwei Zeilen");
        assert_eq!(t.line(1).feminine.as_deref(), Some(""), "what it has no column for stays");
        assert_eq!((t.line(2).text.as_str(), t.line(3).text.as_str()), ("", "Neu"));
        // One undo takes the whole import back; redo brings it again.
        t.undo();
        assert_eq!((t.len(), t.undo.len()), (2, depth));
        assert_eq!(t.line(1).text, "Two\nlines");
        t.redo();
        assert_eq!((t.len(), t.line(3).text.as_str()), (4, "Neu"));

        // A row that can't be read changes nothing.
        let before = t.to_csv();
        assert!(t.import_csv("StrRef,Text\n1,ok\nx,bad\n").is_err());
        assert!(t.import_csv("Text\nno strref\n").is_err());
        assert_eq!(t.to_csv(), before);
    }

    #[test]
    fn json_goes_out_and_comes_back_as_nwn_tlk_has_it() {
        let dir = mg_testkit::scratch_dir("talk-json");
        let mut t = Table::create("json", &dir, Language::ENGLISH, false);
        let line = |text: &str, sound: &str| Line {
            text: text.into(),
            feminine: None,
            sound: sound.into(),
            sound_length: 0.0,
        };
        t.add_line(&line("Bad Strref", "")).unwrap();
        t.add_line(&line("", "")).unwrap();
        t.add_line(&line("Hello \"there\"", "vs_hello")).unwrap();
        let json = t.to_json();
        let read: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(read["language"], 0);
        let entries = read["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2, "the empty line is left out");
        assert_eq!((&entries[1]["id"], &entries[1]["sound"]), (&2.into(), &"vs_hello".into()));
        assert_eq!(t.import_json(&json), Ok((0, 0)));
        // A line changed, one added past a gap; by the game's StrRef too.
        let other =
            r#"{"language":0,"entries":[{"id":16777217,"text":"Second"},{"id":5,"text":"Sixth"}]}"#;
        assert_eq!(t.import_json(other), Ok((1, 1)));
        assert_eq!((t.len(), t.line(1).text, t.line(5).text), (6, "Second".into(), "Sixth".into()));
        assert_eq!(t.line(2).sound, "vs_hello", "what the file leaves out stays");
        t.undo();
        assert_eq!((t.len(), t.line(1).text), (3, String::new()), "one undo");
        assert!(t.import_json("{}").is_err());
        assert!(t.import_json(r#"{"entries":[{"text":"no id"}]}"#).is_err());
    }

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
