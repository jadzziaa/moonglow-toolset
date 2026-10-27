//! A hak (or any ERF archive) open for editing: its resources, from the
//! archive itself or from files added to it, with undo. Saving streams the
//! archive one resource at a time, so a hak of gigabytes isn't held in
//! memory; it's written beside the old one and moved over it, so a failed
//! save leaves the old one whole.
//!
//! File names that aren't resource names are refused with the reason
//! (nwhak, BioWare's hak editor, cuts names past 16 characters short
//! without a word).

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mg_core::{Language, ResRef, ResType};
use mg_erf::Header;
use mg_resman::{Container, ErfContainer, ResKey};

/// Where a resource's bytes are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// In the archive as it was opened (or last saved).
    Archive,
    /// A file added to it.
    File(PathBuf),
    /// Bytes given to it.
    Bytes(Arc<[u8]>),
}

/// One resource of the hak.
#[derive(Debug, Clone)]
pub struct Item {
    pub key: ResKey,
    pub size: u64,
    pub source: Source,
}

/// The files of a folder and of the folders in it, as
/// [`Hak::add_folder`] adds them.
pub fn folder_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(d) = dirs.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if ignored(&name) {
                continue;
            }
            match e.file_type() {
                Ok(t) if t.is_dir() => dirs.push(e.path()),
                Ok(t) if t.is_file() => files.push(e.path()),
                _ => {}
            }
        }
    }
    files.sort();
    files
}

/// What adding files did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Added {
    pub added: usize,
    /// Resources a file of the same name replaced.
    pub replaced: usize,
    /// Files left out, and why.
    pub skipped: Vec<(PathBuf, String)>,
}

/// One step's change, enough to take it back and to do it again (not a
/// copy of every item: a hak of thousands of resources kept one per
/// step).
#[derive(Debug, Clone, Default)]
struct Change {
    /// Items replaced in place: their place, before and after.
    replaced: Vec<(usize, Item, Item)>,
    /// Items taken out: their places (ascending) before, and what they were.
    removed: Vec<(usize, Item)>,
    /// Items added at the end.
    appended: Vec<Item>,
    /// The description before and after.
    description: Option<(String, String)>,
}

/// Renamed resources' bytes, kept in scratch files until the hak is saved;
/// the files go when the last copy of the hak does.
#[derive(Debug, Default)]
struct Stash(std::sync::Mutex<Vec<PathBuf>>);

impl Drop for Stash {
    fn drop(&mut self) {
        let files = std::mem::take(self.0.get_mut().unwrap_or_else(|e| e.into_inner()));
        for f in &files {
            let _ = std::fs::remove_file(f);
        }
        // (The folder is the process's: gone once no hak keeps a file in it.)
        if let Some(dir) = files.first().and_then(|f| f.parent()) {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

/// A hak open for editing.
#[derive(Debug, Clone)]
pub struct Hak {
    /// Where it's saved; `None` for a new one.
    pub path: Option<PathBuf>,
    pub header: Header,
    items: Vec<Item>,
    /// Each item's place, by its key.
    places: HashMap<ResKey, usize>,
    /// The English description (the header's other languages are kept).
    pub description: String,
    archive: Option<Arc<ErfContainer>>,
    undo: Vec<(String, Change)>,
    redo: Vec<(String, Change)>,
    saved: Option<usize>,
    stash: Arc<Stash>,
}

/// Junk that file managers leave in folders.
fn ignored(name: &str) -> bool {
    name.starts_with('.')
        || name.eq_ignore_ascii_case("thumbs.db")
        || name.eq_ignore_ascii_case("desktop.ini")
}

/// The resource a file becomes, or why it can't be one.
pub fn key_for(name: &str) -> Result<ResKey, String> {
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return Err("no extension to give its type".into());
    };
    let Some(restype) = ResType::from_extension(ext) else {
        return Err(format!(".{ext} isn't a type the game reads"));
    };
    if stem.is_empty() {
        return Err("no name".into());
    }
    if stem.len() > 16 {
        return Err(format!("the name has {} characters; the game reads at most 16", stem.len()));
    }
    if !stem.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
        return Err("names are letters, digits, _ and -".into());
    }
    let resref = ResRef::from_str(&stem.to_ascii_lowercase()).map_err(|e| e.to_string())?;
    Ok(ResKey::new(resref, restype))
}

fn english(header: &Header) -> String {
    header
        .description
        .strings
        .iter()
        .find(|(l, _)| *l == Language::ENGLISH.0)
        .map(|(_, b)| Language::ENGLISH.codepage().decode(b).into_owned())
        .unwrap_or_default()
}

impl Hak {
    /// A new, empty hak.
    pub fn new() -> Hak {
        Hak {
            path: None,
            header: Header { file_type: *b"HAK ", ..Default::default() },
            items: Vec::new(),
            places: HashMap::new(),
            description: String::new(),
            archive: None,
            undo: Vec::new(),
            redo: Vec::new(),
            saved: None,
            stash: Arc::default(),
        }
    }

    /// Opens an archive.
    pub fn open(path: &Path) -> Result<Hak, String> {
        let archive = ErfContainer::open(path).map_err(|e| e.to_string())?;
        let items = archive
            .index()
            .iter()
            .map(|&(key, size)| Item { key, size, source: Source::Archive })
            .collect();
        let header = archive.header.clone();
        let mut hak = Hak {
            path: Some(path.to_path_buf()),
            description: english(&header),
            header,
            items,
            archive: Some(Arc::new(archive)),
            saved: Some(0),
            ..Hak::new()
        };
        hak.index();
        Ok(hak)
    }

    /// Finds each item's place again.
    fn index(&mut self) {
        self.places = self.items.iter().enumerate().map(|(i, item)| (item.key, i)).collect();
    }

    /// The item of a key.
    fn item(&self, key: &ResKey) -> Option<&Item> {
        self.places.get(key).map(|&i| &self.items[i])
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// The total size of its resources.
    pub fn size(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }

    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.undo.len())
    }

    /// Makes a change, as one step to undo.
    fn commit(&mut self, what: &str, change: Change) {
        if self.saved.is_some_and(|s| s > self.undo.len()) {
            self.saved = None;
        }
        self.apply(&change);
        self.undo.push((what.to_string(), change));
        self.redo.clear();
    }

    /// Does a change: what it replaces, then what it removes, then what it
    /// adds at the end.
    fn apply(&mut self, c: &Change) {
        for (at, _, after) in &c.replaced {
            self.places.remove(&self.items[*at].key);
            self.items[*at] = after.clone();
            self.places.insert(after.key, *at);
        }
        if !c.removed.is_empty() {
            for (at, _) in c.removed.iter().rev() {
                self.items.remove(*at);
            }
            self.index();
        }
        for item in &c.appended {
            self.places.insert(item.key, self.items.len());
            self.items.push(item.clone());
        }
        if let Some((_, after)) = &c.description {
            self.description = after.clone();
        }
    }

    /// Takes a change back, in the reverse order.
    fn revert(&mut self, c: &Change) {
        for item in &c.appended {
            self.places.remove(&item.key);
        }
        self.items.truncate(self.items.len() - c.appended.len());
        if !c.removed.is_empty() {
            for (at, item) in &c.removed {
                self.items.insert(*at, item.clone());
            }
            self.index();
        }
        for (at, before, _) in &c.replaced {
            self.places.remove(&self.items[*at].key);
            self.items[*at] = before.clone();
            self.places.insert(before.key, *at);
        }
        if let Some((before, _)) = &c.description {
            self.description = before.clone();
        }
    }

    /// What Undo would take back.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|(w, _)| w.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|(w, _)| w.as_str())
    }

    pub fn undo(&mut self) {
        if let Some((what, change)) = self.undo.pop() {
            self.revert(&change);
            self.redo.push((what, change));
        }
    }

    pub fn redo(&mut self) {
        if let Some((what, change)) = self.redo.pop() {
            self.apply(&change);
            self.undo.push((what, change));
        }
    }

    /// Sets the English description.
    pub fn set_description(&mut self, text: &str) {
        if text != self.description {
            let description = Some((self.description.clone(), text.to_string()));
            self.commit("Description", Change { description, ..Default::default() });
        }
    }

    /// Puts items in, replacing those of the same name (a later of one
    /// name replacing an earlier): one step to undo.
    fn put(&mut self, what: &str, items: Vec<Item>, out: &mut Added) {
        let mut change = Change::default();
        // Where each key went in the change.
        let mut put: HashMap<ResKey, (bool, usize)> = HashMap::new();
        for item in items {
            match (put.get(&item.key), self.places.get(&item.key)) {
                (Some(&(true, i)), _) => {
                    change.replaced[i].2 = item;
                    out.replaced += 1;
                }
                (Some(&(false, i)), _) => {
                    change.appended[i] = item;
                    out.replaced += 1;
                }
                (None, Some(&at)) => {
                    put.insert(item.key, (true, change.replaced.len()));
                    change.replaced.push((at, self.items[at].clone(), item));
                    out.replaced += 1;
                }
                (None, None) => {
                    put.insert(item.key, (false, change.appended.len()));
                    change.appended.push(item);
                    out.added += 1;
                }
            }
        }
        self.commit(what, change);
    }

    /// Adds files, replacing resources of the same name; one undo step.
    pub fn add_files(&mut self, paths: &[PathBuf]) -> Added {
        let mut out = Added::default();
        let mut found = Vec::new();
        for p in paths {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if ignored(&name) {
                continue;
            }
            match (key_for(&name), std::fs::metadata(p)) {
                (Ok(key), Ok(meta)) if meta.is_file() => found.push((key, meta.len(), p.clone())),
                (Ok(_), Ok(_)) => {}
                (Err(why), _) => out.skipped.push((p.clone(), why)),
                (_, Err(e)) => out.skipped.push((p.clone(), e.to_string())),
            }
        }
        // Two files with one name (in different folders): the first stays.
        found.sort_by(|a, b| a.2.cmp(&b.2));
        let mut seen = std::collections::HashSet::new();
        let found: Vec<_> = found
            .into_iter()
            .filter(|(key, _, p)| {
                let first = seen.insert(*key);
                if !first {
                    out.skipped.push((p.clone(), format!("another {key} is added already")));
                }
                first
            })
            .collect();
        if found.is_empty() {
            return out;
        }
        let items = found
            .into_iter()
            .map(|(key, size, path)| Item { key, size, source: Source::File(path) })
            .collect();
        self.put("Add files", items, &mut out);
        out
    }

    /// Adds resources by where their bytes are (given, or a file's),
    /// replacing those of the same name; one undo step.
    pub fn add_sources(&mut self, resources: Vec<(ResKey, Source)>) -> Result<Added, String> {
        self.add_sources_as("Add resources", resources)
    }

    /// [`add_sources`](Self::add_sources), the step named `what` in Undo.
    pub fn add_sources_as(
        &mut self,
        what: &str,
        resources: Vec<(ResKey, Source)>,
    ) -> Result<Added, String> {
        let mut out = Added::default();
        if resources.is_empty() {
            return Ok(out);
        }
        let mut items = Vec::new();
        for (key, source) in resources {
            let size = match &source {
                Source::Bytes(b) => b.len() as u64,
                Source::File(p) => {
                    std::fs::metadata(p).map_err(|e| format!("{}: {e}", p.display()))?.len()
                }
                Source::Archive => return Err(format!("{key}: nothing to add")),
            };
            items.push(Item { key, size, source });
        }
        self.put(what, items, &mut out);
        Ok(out)
    }

    /// Adds the files of a folder and the folders in it.
    pub fn add_folder(&mut self, dir: &Path) -> Added {
        self.add_files(&folder_files(dir))
    }

    /// Of `paths`, the files that would replace resources the hak has:
    /// each with the resource (for asking first, as Aurora's hak editor
    /// does).
    pub fn replaced_by(&self, paths: &[PathBuf]) -> Vec<(PathBuf, ResKey)> {
        paths
            .iter()
            .filter_map(|p| {
                let name = p.file_name()?.to_string_lossy().into_owned();
                let key = key_for(&name).ok().filter(|_| !ignored(&name))?;
                self.places.contains_key(&key).then(|| (p.clone(), key))
            })
            .collect()
    }

    /// Removes resources; one undo step.
    pub fn remove(&mut self, keys: &[ResKey]) {
        let mut places: Vec<usize> =
            keys.iter().filter_map(|k| self.places.get(k)).copied().collect();
        if places.is_empty() {
            return;
        }
        places.sort_unstable();
        places.dedup();
        let removed = places.into_iter().map(|at| (at, self.items[at].clone())).collect();
        self.commit("Remove", Change { removed, ..Default::default() });
    }

    /// Renames a resource (its type stays).
    pub fn rename(&mut self, key: ResKey, name: &str) -> Result<ResKey, String> {
        let to = key_for(&format!("{name}.{}", key.restype))?;
        if to == key {
            return Ok(key);
        }
        if self.places.contains_key(&to) {
            return Err(format!("{to} is in the hak already"));
        }
        let at = *self.places.get(&key).ok_or("not in the hak")?;
        let before = self.items[at].clone();
        let mut after = Item { key: to, ..before.clone() };
        // An archive entry keeps its bytes under the new name.
        if before.source == Source::Archive {
            after.source = Source::File(self.stash(to, &self.data(key)?)?);
        }
        self.commit("Rename", Change { replaced: vec![(at, before, after)], ..Default::default() });
        Ok(to)
    }

    /// Keeps renamed bytes in a scratch file until the hak is saved (and
    /// removed when the hak is closed).
    fn stash(&self, key: ResKey, bytes: &[u8]) -> Result<PathBuf, String> {
        let dir = std::env::temp_dir().join(format!("moonglow-hak-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = dir.join(format!("{n}-{key}"));
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        self.stash.0.lock().unwrap_or_else(|e| e.into_inner()).push(path.clone());
        Ok(path)
    }

    /// A resource's bytes.
    pub fn data(&self, key: ResKey) -> Result<Vec<u8>, String> {
        let item = self.item(&key).ok_or("not in the hak")?;
        self.read(item)
    }

    fn read(&self, item: &Item) -> Result<Vec<u8>, String> {
        match &item.source {
            Source::Archive => {
                let a = self.archive.as_ref().ok_or("no archive")?;
                a.read(&item.key).map(Cow::into_owned).map_err(|e| e.to_string())
            }
            Source::File(p) => std::fs::read(p).map_err(|e| format!("{}: {e}", p.display())),
            Source::Bytes(b) => Ok(b.to_vec()),
        }
    }

    /// Writes resources as files into a folder; returns how many.
    pub fn extract(&self, keys: &[ResKey], dir: &Path) -> Result<usize, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut n = 0;
        let mut places: Vec<usize> =
            keys.iter().filter_map(|k| self.places.get(k)).copied().collect();
        places.sort_unstable();
        places.dedup();
        for item in places.into_iter().map(|at| &self.items[at]) {
            let path = item.key.path_in(dir).map_err(|e| e.to_string())?;
            std::fs::write(&path, self.read(item)?)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            n += 1;
        }
        Ok(n)
    }

    /// The resources that would start past 2 GiB, which the game can't read.
    pub fn past_read_limit(&self) -> Vec<ResKey> {
        let entries: Vec<_> =
            self.items.iter().map(|i| (i.key.resref, i.key.restype, i.size)).collect();
        mg_erf::past_read_limit(&self.header, &entries)
            .into_iter()
            .map(|(r, t)| ResKey::new(r, t))
            .collect()
    }

    /// Saves to `path` (its own when `None`), then reads the archive back.
    pub fn save(&mut self, path: Option<&Path>) -> Result<(), String> {
        let path = path.or(self.path.as_deref()).ok_or("no file to save to")?.to_path_buf();
        // Sizes now: an added file may have changed since.
        for item in &mut self.items {
            if let Source::File(p) = &item.source {
                item.size =
                    std::fs::metadata(p).map_err(|e| format!("{}: {e}", p.display()))?.len();
            }
        }
        let mut header = self.header.clone();
        let codepage = Language::ENGLISH.codepage();
        let text = codepage
            .encode(&self.description)
            .ok_or("the description has characters the game can't write")?
            .into_owned();
        header.description.strings.retain(|(l, _)| *l != Language::ENGLISH.0);
        if !text.is_empty() {
            header.description.strings.insert(0, (Language::ENGLISH.0, text));
        }
        let (year, day) = crate::build_date();
        header.build_year = year;
        header.build_day = day;
        let entries: Vec<_> =
            self.items.iter().map(|i| (i.key.resref, i.key.restype, i.size)).collect();
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let part = path.with_extension("part");
        let write = || -> Result<(), String> {
            let file =
                std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
            let mut w = std::io::BufWriter::new(file);
            mg_erf::write_streamed(&mut w, &header, &entries, |i| {
                self.read(&self.items[i]).map(Cow::Owned).map_err(std::io::Error::other)
            })
            .map_err(|e| e.to_string())?;
            w.flush().map_err(|e| e.to_string())
        };
        if let Err(e) = write() {
            let _ = std::fs::remove_file(&part);
            return Err(e);
        }
        std::fs::rename(&part, &path).map_err(|e| format!("{}: {e}", path.display()))?;
        let archive = ErfContainer::open(&path).map_err(|e| e.to_string())?;
        for item in &mut self.items {
            item.source = Source::Archive;
        }
        self.header = archive.header.clone();
        self.archive = Some(Arc::new(archive));
        self.path = Some(path);
        // Undo now would point at sources that may be gone: start afresh.
        self.undo.clear();
        self.redo.clear();
        self.saved = Some(0);
        Ok(())
    }
}

/// Puts `resources` into the hak at `path`, replacing those of the same
/// name there: the hak is made if there is none, and one that is there is
/// first copied beside itself as `.bak`.
pub fn write_into(path: &Path, resources: Vec<(ResKey, Source)>) -> Result<Added, String> {
    let mut hak = if path.exists() {
        let hak = Hak::open(path)?;
        let bak = path.with_extension("hak.bak");
        std::fs::copy(path, &bak).map_err(|e| format!("{}: {e}", bak.display()))?;
        hak
    } else {
        Hak::new()
    };
    let added = hak.add_sources(resources)?;
    hak.save(Some(path))?;
    Ok(added)
}

impl Default for Hak {
    fn default() -> Self {
        Hak::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_go_into_a_new_hak_and_into_one_that_is_there() {
        let dir = std::env::temp_dir().join(format!("mg-hak-bytes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("made.hak");
        let key = |n: &str| ResKey::parse(n, ResType::TXT).unwrap();
        let bytes = |b: &[u8]| Source::Bytes(b.into());
        let first = vec![(key("one"), bytes(b"1")), (key("two"), bytes(b"2"))];
        let added = write_into(&path, first).unwrap();
        assert_eq!((added.added, added.replaced), (2, 0));
        // (One of them a file's.)
        let file = dir.join("three.txt");
        std::fs::write(&file, "3").unwrap();
        let second = vec![(key("two"), bytes(b"two")), (key("three"), Source::File(file))];
        let added = write_into(&path, second).unwrap();
        assert_eq!((added.added, added.replaced), (1, 1));
        let hak = Hak::open(&path).unwrap();
        assert_eq!(hak.items().len(), 3);
        assert_eq!(hak.data(key("one")).unwrap(), b"1");
        assert_eq!(hak.data(key("two")).unwrap(), b"two");
        assert_eq!(hak.data(key("three")).unwrap(), b"3");
        // What was there before is kept beside it.
        let bak = Hak::open(&dir.join("made.hak.bak")).unwrap();
        assert_eq!(bak.data(key("two")).unwrap(), b"2");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn write(dir: &Path, name: &str, data: &[u8]) -> PathBuf {
        let p = dir.join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, data).unwrap();
        p
    }

    #[test]
    fn built_from_a_folder_edited_and_saved() {
        let dir = std::env::temp_dir().join(format!("mg-hak-edit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let src = dir.join("content");
        write(&src, "plc_chair.mdl", b"model");
        write(&src, "textures/Plc_Chair.TGA", b"texture");
        write(&src, "readme", b"no type");
        write(&src, "a_name_much_too_long.mdl", b"x");
        write(&src, ".hidden.2da", b"x");
        write(&src, "textures/Thumbs.db", b"x");
        let mut hak = Hak::new();
        let added = hak.add_folder(&src);
        assert_eq!((added.added, added.replaced), (2, 0));
        let mut why: Vec<String> = added.skipped.iter().map(|(_, w)| w.clone()).collect();
        why.sort();
        assert_eq!(
            why,
            [
                "no extension to give its type",
                "the name has 20 characters; the game reads at most 16"
            ]
        );
        assert!(hak.is_dirty());
        let keys: Vec<String> = hak.items().iter().map(|i| i.key.to_string()).collect();
        assert_eq!(keys, ["plc_chair.mdl", "plc_chair.tga"]);

        let path = dir.join("mine.hak");
        hak.set_description("Chairs");
        hak.save(Some(&path)).unwrap();
        assert!(!hak.is_dirty());
        let back = Hak::open(&path).unwrap();
        assert_eq!(back.description, "Chairs");
        assert_eq!(
            back.data(ResKey::parse("plc_chair", ResType::TGA).unwrap()).unwrap(),
            b"texture"
        );

        // Replace, rename, remove, undo; then save over itself.
        let mut hak = back;
        let mdl = ResKey::parse("plc_chair", ResType::MDL).unwrap();
        let more = hak.add_files(&[write(&dir, "plc_chair.mdl", b"model 2")]);
        assert_eq!((more.added, more.replaced), (0, 1));
        let renamed =
            hak.rename(ResKey::parse("plc_chair", ResType::TGA).unwrap(), "plc_seat").unwrap();
        assert!(hak.rename(renamed, "plc_chair").is_ok(), "the name is free again");
        hak.undo();
        hak.remove(&[mdl]);
        assert_eq!(hak.items().len(), 1);
        hak.undo();
        assert_eq!(hak.data(mdl).unwrap(), b"model 2");
        hak.save(None).unwrap();
        let back = Hak::open(&path).unwrap();
        let keys: Vec<String> = back.items().iter().map(|i| i.key.to_string()).collect();
        assert_eq!(keys, ["plc_chair.mdl", "plc_seat.tga"]);
        assert_eq!(
            back.data(ResKey::parse("plc_seat", ResType::TGA).unwrap()).unwrap(),
            b"texture"
        );
        let out = dir.join("out");
        assert_eq!(back.extract(&[mdl], &out).unwrap(), 1);
        assert_eq!(std::fs::read(out.join("plc_chair.mdl")).unwrap(), b"model 2");
        assert!(!dir.join("mine.part").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Many resources added at once are found by name (not a scan of the
    /// hak per resource); one of a name twice in a step is the later; undo
    /// and redo go back and forth over adds, removes, renames and the
    /// description, each step keeping only what it changed.
    #[test]
    fn steps_are_undone_and_redone_by_what_they_changed() {
        let key = |n: String| ResKey::parse(&n, ResType::TXT).unwrap();
        let bytes = |b: &[u8]| Source::Bytes(b.into());
        let mut hak = Hak::new();
        let many: Vec<(ResKey, Source)> =
            (0..20_000).map(|i| (key(format!("r{i}")), bytes(b"x"))).collect();
        let start = std::time::Instant::now();
        assert_eq!(hak.add_sources(many.clone()).unwrap().added, 20_000);
        let again = hak.add_sources(many).unwrap();
        assert_eq!((again.added, again.replaced), (0, 20_000));
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        let twice = vec![(key("new".into()), bytes(b"1")), (key("new".into()), bytes(b"2"))];
        let added = hak.add_sources(twice).unwrap();
        assert_eq!((added.added, added.replaced), (1, 1));
        assert_eq!(hak.data(key("new".into())).unwrap(), b"2");
        assert_eq!(hak.items().len(), 20_001);
        let names = |h: &Hak| -> Vec<String> {
            h.items().iter().map(|i| i.key.resref.to_string()).collect()
        };
        let all = names(&hak);
        hak.remove(&[key("r5".into()), key("r1".into()), key("nothing".into())]);
        hak.rename(key("r2".into()), "two").unwrap();
        hak.set_description("Text");
        let changed = names(&hak);
        assert_eq!(changed[..3], ["r0", "two", "r3"]);
        for _ in 0..3 {
            hak.undo();
        }
        assert_eq!(names(&hak), all);
        assert_eq!((hak.description.as_str(), hak.data(key("r1".into())).is_ok()), ("", true));
        for _ in 0..3 {
            hak.redo();
        }
        assert_eq!(names(&hak), changed);
        assert_eq!(hak.description, "Text");
        assert!(hak.data(key("r2".into())).is_err() && hak.data(key("two".into())).is_ok());
        for _ in 0..6 {
            hak.undo();
        }
        assert!(hak.items().is_empty(), "back to an empty hak");
    }

    /// A renamed archive entry's bytes wait in a scratch file, removed
    /// with the hak (its last copy).
    #[test]
    fn renamed_bytes_are_kept_until_the_hak_is_closed() {
        let dir = mg_testkit::scratch_dir("mg-hak-stash");
        let path = dir.join("s.hak");
        let key = |n: &str| ResKey::parse(n, ResType::TXT).unwrap();
        write_into(&path, vec![(key("one"), Source::Bytes(b"1"[..].into()))]).unwrap();
        let mut hak = Hak::open(&path).unwrap();
        let renamed = hak.rename(key("one"), "two").unwrap();
        let Source::File(stashed) = hak.items()[0].source.clone() else { panic!("a file") };
        assert!(stashed.is_file());
        let copy = hak.clone();
        drop(hak);
        assert!(stashed.is_file(), "a copy still has it");
        assert_eq!(copy.data(renamed).unwrap(), b"1");
        drop(copy);
        assert!(!stashed.exists());
    }

    #[test]
    fn build_dates() {
        let (year, day) = crate::build_date();
        assert!(year >= 125 && day < 366);
    }
}
