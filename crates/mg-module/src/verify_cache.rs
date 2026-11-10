//! What Verify found of each model of an archive, kept between runs in
//! the user's cache folder (`verify/`, beside the shader programs that
//! compiled), so that a hak that is as it was is not read again: a
//! model's text's findings (`mdl-text`, the linter's) and, of a tile's
//! model, how many faces it has past Aurora's limit (`tile-faces`,
//! `mdl-read`). The Domains of Dread's 145 haks hold 43,000 models kept
//! as text (1.9 GB) and 32,000 tile models (3.8 GB): reading and linting
//! them was most of a Verify's work.
//!
//! An archive is the same while its time and size
//! ([`mg_resman::Layer::fingerprint`]) and what it lists (each entry's
//! name, place and size: [`mg_resman::Container::listing`]) are; its
//! file names it with them, with the folder it is in and with the
//! release, so a changed hak or another Moonglow reads it again, and a
//! hak of the same name in another checkout has a file of its own. (A
//! hak written again with the same time, size and layout, only its data
//! other, is not told apart: that would take reading it.)
//! Folders (`override`, `development`, the module's) are always read.
//! What each run learns is handed back from the workers and saved once
//! at the end, a file an archive, written whole and renamed into place.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use mg_resman::{LayerClass, ResKey, ResMan};
use rayon::prelude::*;
use serde_json::{Value, json};

use crate::doctor::Severity;

/// A finding as a model's facts keep it: its severity, where and what
/// (its source and check are the model's).
pub(crate) type Noted = (Severity, String, String);

/// What a run learned of models, for the next: each one's layer, key and
/// facts.
pub(crate) type Learned = Vec<(usize, ResKey, Facts)>;

/// What is known of one model.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Facts {
    /// Its text's findings; `Some(None)`: it is compiled, with no text.
    pub(crate) text: Option<Option<Vec<Noted>>>,
    /// Its faces past the limit (`Some(n)`), none past it, or why it can't
    /// be read.
    pub(crate) faces: Option<Result<Option<usize>, String>>,
    /// Whether it was read whole (`Some(None)`) or why it couldn't be
    /// (an object's appearance's or part's model: `object-row`).
    pub(crate) read: Option<Option<String>>,
}

impl Facts {
    fn merge(&mut self, other: Facts) {
        if other.text.is_some() {
            self.text = other.text;
        }
        if other.faces.is_some() {
            self.faces = other.faces;
        }
        if other.read.is_some() {
            self.read = other.read;
        }
    }
}

/// What earlier runs found of the archives of a Verify's game data, by
/// layer: `None` for a layer not kept (a folder, the game's own).
#[derive(Debug, Default)]
pub(crate) struct Memo {
    dir: PathBuf,
    layers: Vec<Option<(PathBuf, HashMap<ResKey, Facts>)>>,
}

/// What tells an archive from another of its label, and from itself
/// written again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Print {
    /// The folder it is in (a number of its path; 0: none known).
    folder: u32,
    /// Its time and size, and what it lists.
    version: u64,
}

impl Print {
    fn of(layer: &mg_resman::Layer) -> Print {
        use std::hash::{Hash, Hasher};
        let hashed = |of: &dyn Fn(&mut std::collections::hash_map::DefaultHasher)| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            of(&mut h);
            h.finish()
        };
        let folder = layer.container.watched().and_then(Path::parent).map_or(0, |dir| {
            // (As it is on the disk, where that can be told: one folder
            // reached by two paths is one.)
            let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
            hashed(&|h| dir.hash(h)) as u32
        });
        let listing = layer.container.listing();
        Print { folder, version: hashed(&|h| (layer.fingerprint, listing).hash(h)) }
    }
}

/// The file an archive's facts are kept in: its label, its folder and its
/// version.
fn file_name(label: &str, print: Print) -> String {
    format!("{}{:016x}.json", file_stem(label, print.folder), print.version)
}

/// A label as a file's name has it.
fn label_name(label: &str) -> String {
    (label.chars())
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

/// The words a file's name begins with, for an archive of this label in
/// this folder: its other versions' files begin so too.
fn file_stem(label: &str, folder: u32) -> String {
    format!("{}-{folder:08x}-", label_name(label))
}

/// What follows the stem in a file's name: the version and `.json`.
const VERSION_LEN: usize = "0000000000000000.json".len();

fn severity_word(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

fn read_facts(v: &Value) -> Option<Facts> {
    let text = match v.get("text") {
        None => None,
        Some(Value::String(s)) if s == "compiled" => Some(None),
        Some(Value::Array(found)) => {
            let mut noted = Vec::new();
            for f in found {
                let severity = match f.get(0)?.as_str()? {
                    "error" => Severity::Error,
                    "warning" => Severity::Warning,
                    _ => return None,
                };
                noted.push((severity, f.get(1)?.as_str()?.into(), f.get(2)?.as_str()?.into()));
            }
            Some(Some(noted))
        }
        Some(_) => return None,
    };
    let faces = match v.get("faces") {
        None => None,
        Some(Value::String(s)) if s == "under" => Some(Ok(None)),
        Some(Value::Number(n)) => Some(Ok(Some(usize::try_from(n.as_u64()?).ok()?))),
        Some(Value::Object(o)) => Some(Err(o.get("error")?.as_str()?.to_string())),
        Some(_) => return None,
    };
    let read = match v.get("read") {
        None => None,
        Some(Value::String(s)) if s == "whole" => Some(None),
        Some(Value::Object(o)) => Some(Some(o.get("error")?.as_str()?.to_string())),
        Some(_) => return None,
    };
    Some(Facts { text, faces, read })
}

fn write_facts(f: &Facts) -> Value {
    let mut o = serde_json::Map::new();
    match &f.text {
        None => {}
        Some(None) => {
            o.insert("text".into(), json!("compiled"));
        }
        Some(Some(found)) => {
            let found: Vec<Value> =
                found.iter().map(|(s, at, m)| json!([severity_word(*s), at, m])).collect();
            o.insert("text".into(), Value::Array(found));
        }
    }
    match &f.faces {
        None => {}
        Some(Ok(None)) => {
            o.insert("faces".into(), json!("under"));
        }
        Some(Ok(Some(n))) => {
            o.insert("faces".into(), json!(n));
        }
        Some(Err(e)) => {
            o.insert("faces".into(), json!({ "error": e }));
        }
    }
    match &f.read {
        None => {}
        Some(None) => {
            o.insert("read".into(), json!("whole"));
        }
        Some(Some(e)) => {
            o.insert("read".into(), json!({ "error": e }));
        }
    }
    Value::Object(o)
}

/// An archive's facts as its file has them; nothing if it isn't there, or
/// isn't of this release.
fn read_file(path: &Path) -> HashMap<ResKey, Facts> {
    let Ok(bytes) = std::fs::read(path) else { return HashMap::new() };
    let Ok(v) = serde_json::from_slice::<Value>(&bytes) else { return HashMap::new() };
    if v.get("release").and_then(Value::as_str) != Some(mg_core::release::version()) {
        return HashMap::new();
    }
    let Some(Value::Object(models)) = v.get("models") else { return HashMap::new() };
    (models.iter())
        .filter_map(|(name, f)| Some((ResKey::from_filename(name)?, read_facts(f)?)))
        .collect()
}

impl Memo {
    /// What earlier runs found of `resman`'s archives that are as they
    /// were (each file read in parallel).
    pub(crate) fn load(dir: &Path, resman: &ResMan, custom: impl Fn(usize) -> bool) -> Memo {
        let layers = resman.layers();
        let kept: Vec<Option<PathBuf>> = (layers.iter().enumerate())
            .map(|(i, l)| {
                (custom(i) && l.class == LayerClass::Erf && l.fingerprint != 0)
                    .then(|| dir.join(file_name(&l.label, Print::of(l))))
            })
            .collect();
        let layers =
            (kept.into_par_iter()).map(|path| path.map(|p| (p.clone(), read_file(&p)))).collect();
        Memo { dir: dir.to_path_buf(), layers }
    }

    /// Whether what is learned of a layer's models is kept.
    pub(crate) fn keeps(&self, layer: usize) -> bool {
        self.layers.get(layer).is_some_and(Option::is_some)
    }

    pub(crate) fn get(&self, layer: usize, key: &ResKey) -> Option<&Facts> {
        self.layers.get(layer)?.as_ref()?.1.get(key)
    }

    /// Takes what a run learned, and writes each archive's file that it
    /// changed (taking away those of the archive's earlier versions).
    pub(crate) fn save(mut self, learned: Learned, resman: &ResMan) {
        let mut changed: BTreeMap<usize, ()> = BTreeMap::new();
        for (layer, key, facts) in learned {
            let Some(Some((_, known))) = self.layers.get_mut(layer) else { continue };
            let was = known.get(&key).cloned();
            let now = known.entry(key).or_default();
            now.merge(facts);
            if was.as_ref() != Some(now) {
                changed.insert(layer, ());
            }
        }
        if changed.is_empty() || std::fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        // (Each archive's files' stem: its label's and its folder's, and the
        // one its files had before the folder was in their names.)
        let stems: Vec<[String; 2]> = (resman.layers().iter())
            .map(|l| {
                [file_stem(&l.label, Print::of(l).folder), format!("{}-", label_name(&l.label))]
            })
            .collect();
        // (The folder's files listed once, for the earlier versions'.)
        let there: Vec<String> = (std::fs::read_dir(&self.dir).into_iter().flatten().flatten())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        // (Written in parallel: a first Verify of 145 haks writes 4 MB.)
        let changed: Vec<usize> = changed.into_keys().collect();
        changed.par_iter().for_each(|&layer| {
            let Some(Some((path, known))) = self.layers.get(layer) else { return };
            let mut models: Vec<(String, Value)> =
                known.iter().map(|(k, f)| (k.to_string(), write_facts(f))).collect();
            models.sort_by(|a, b| a.0.cmp(&b.0));
            let file = json!({
                "release": mg_core::release::version(),
                "models": models.into_iter().collect::<serde_json::Map<_, _>>(),
            });
            let Ok(bytes) = serde_json::to_vec(&file) else { return };
            let tmp = path.with_extension(format!("tmp{}", std::process::id()));
            if std::fs::write(&tmp, bytes).is_err() || std::fs::rename(&tmp, path).is_err() {
                let _ = std::fs::remove_file(&tmp);
                return;
            }
            // (The archive's earlier versions' facts are of no use now: its
            // own, not those of an archive of its name in another folder.)
            let Some(stems) = stems.get(layer) else { return };
            let ours = path.file_name().map(|n| n.to_string_lossy().into_owned());
            for name in &there {
                let earlier = |stem: &String| {
                    name.starts_with(stem.as_str()) && name.len() == stem.len() + VERSION_LEN
                };
                if stems.iter().any(earlier) && Some(name) != ours.as_ref() {
                    let _ = std::fs::remove_file(self.dir.join(name));
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facts_read_back_as_written() {
        let all = [
            Facts::default(),
            Facts { text: Some(None), faces: Some(Ok(None)), read: Some(None) },
            Facts {
                text: Some(Some(vec![
                    (Severity::Error, "line 3".into(), "a \"quoted\" word".into()),
                    (Severity::Warning, String::new(), "and 4 more".into()),
                ])),
                faces: Some(Ok(Some(12_345))),
                read: Some(Some("truncated".into())),
            },
            Facts {
                text: Some(Some(Vec::new())),
                faces: Some(Err("can't be read: Empty".into())),
                read: None,
            },
        ];
        for f in all {
            assert_eq!(read_facts(&write_facts(&f)), Some(f));
        }
        assert_eq!(read_facts(&json!({ "text": 3 })), None);
        assert_eq!(read_facts(&json!({ "text": [["odd", "", ""]] })), None);
    }

    #[test]
    fn an_archive_s_file_is_named_by_its_label_folder_and_version() {
        let print = Print { folder: 0xc0de, version: 0xab };
        assert_eq!(
            file_name("hak:tdod_tiles01", print),
            "hak_tdod_tiles01-0000c0de-00000000000000ab.json"
        );
        assert_eq!(file_stem("hak:tdod_tiles01", 0xc0de), "hak_tdod_tiles01-0000c0de-");
    }

    /// A hak of `models` (each a name and its text) at `path`, its time
    /// set to a fixed one, as a reproducible build stamps it.
    fn hak(path: &Path, models: &[(&str, &str)]) {
        let mut w = mg_erf::ErfWriter::new(*b"HAK ");
        for (name, text) in models {
            let name = mg_core::ResRef::from_str(name).unwrap();
            w.add(name, mg_core::ResType::MDL, text.as_bytes().to_vec()).unwrap();
        }
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, w.to_bytes().unwrap()).unwrap();
        let stamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
        std::fs::File::options().write(true).open(path).unwrap().set_modified(stamp).unwrap();
    }

    fn game(path: &Path) -> ResMan {
        let mut rm = ResMan::new();
        let c = mg_resman::ErfContainer::open(path).unwrap();
        rm.add(mg_resman::priority::HAK_USER, "hak:kz", LayerClass::Erf, c);
        rm
    }

    /// Two archives of one name, time and size whose entries differ (one
    /// built again with a fixed time) are two versions, each with its own
    /// file; and two of one name in two folders (two checkouts of a
    /// project) each keep theirs, where each Verify took the other's away
    /// as "the archive's earlier version" (the review's V14).
    #[test]
    fn archives_of_one_name_time_and_size_are_told_apart() {
        let dir = mg_testkit::scratch_dir("verify-cache-names");
        let (a, b) = (dir.join("a/kz.hak"), dir.join("b/kz.hak"));
        hak(&a, &[("one", "12345"), ("two", "123")]);
        hak(&b, &[("one", "1234"), ("two", "1234")]);
        let (rm_a, rm_b) = (game(&a), game(&b));
        let (la, lb) = (&rm_a.layers()[0], &rm_b.layers()[0]);
        assert_eq!(la.fingerprint, lb.fingerprint, "one time and size");
        assert_ne!(Print::of(la).version, Print::of(lb).version, "other entries");
        assert_ne!(Print::of(la).folder, Print::of(lb).folder);
        // The same entries in another folder: the same version, another file.
        let c = dir.join("c/kz.hak");
        hak(&c, &[("one", "12345"), ("two", "123")]);
        let rm_c = game(&c);
        let lc = &rm_c.layers()[0];
        assert_eq!(Print::of(la).version, Print::of(lc).version);
        assert_ne!(file_name(&la.label, Print::of(la)), file_name(&lc.label, Print::of(lc)));

        // Each checkout's Verify keeps its own file, and takes away its
        // archive's earlier version alone (and a file named as before the
        // folder was in the name).
        let cache = dir.join("cache");
        let key = ResKey::parse("one", mg_core::ResType::MDL).unwrap();
        let learn = |rm: &ResMan| {
            let memo = Memo::load(&cache, rm, |_| true);
            let facts = Facts { read: Some(None), ..Facts::default() };
            memo.save(vec![(0, key, facts)], rm);
        };
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("hak_kz-00000000000000ab.json"), "{}").unwrap();
        learn(&rm_a);
        learn(&rm_c);
        let files = || {
            let mut names: Vec<String> = (std::fs::read_dir(&cache).unwrap().flatten())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        };
        let mut both =
            vec![file_name(&la.label, Print::of(la)), file_name(&lc.label, Print::of(lc))];
        both.sort();
        assert_eq!(files(), both);
        assert!(Memo::load(&cache, &rm_a, |_| true).get(0, &key).is_some(), "a's is still there");
        // a's hak built again, its entries other: its earlier file goes,
        // c's stays.
        hak(&a, &[("one", "1234"), ("two", "1234")]);
        let rm_a2 = game(&a);
        assert!(Memo::load(&cache, &rm_a2, |_| true).get(0, &key).is_none(), "read again");
        learn(&rm_a2);
        let la2 = &rm_a2.layers()[0];
        let mut both =
            vec![file_name(&la2.label, Print::of(la2)), file_name(&lc.label, Print::of(lc))];
        both.sort();
        assert_eq!(files(), both);
    }
}
