//! What Verify found of each model of an archive, kept between runs in
//! the user's cache folder (`verify/`, beside the shader programs that
//! compiled), so that a hak that is as it was is not read again: a
//! model's text's findings (`mdl-text`, the linter's) and, of a tile's
//! model, how many faces it has past Aurora's limit (`tile-faces`,
//! `mdl-read`). The Domains of Dread's 145 haks hold 43,000 models kept
//! as text (1.9 GB) and 32,000 tile models (3.8 GB): reading and linting
//! them was most of a Verify's work.
//!
//! An archive is the same while its time and size are
//! ([`mg_resman::Layer::fingerprint`]); its file names it with them and
//! the release, so a changed hak or another Moonglow reads it again.
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
}

impl Facts {
    fn merge(&mut self, other: Facts) {
        if other.text.is_some() {
            self.text = other.text;
        }
        if other.faces.is_some() {
            self.faces = other.faces;
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

/// The file an archive's facts are kept in: its label and fingerprint.
fn file_name(label: &str, fingerprint: u64) -> String {
    let name: String = (label.chars())
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    format!("{name}-{fingerprint:016x}.json")
}

/// The words a file's name begins with, for an archive of this label.
fn file_stem(label: &str) -> String {
    let name = file_name(label, 0);
    name[..name.len() - "0000000000000000.json".len()].to_string()
}

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
    Some(Facts { text, faces })
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
                    .then(|| dir.join(file_name(&l.label, l.fingerprint)))
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
        let labels: Vec<&str> = resman.layers().iter().map(|l| l.label.as_str()).collect();
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
            // (The archive's earlier versions' facts are of no use now.)
            let stem = labels.get(layer).map(|l| file_stem(l)).unwrap_or_default();
            let ours = path.file_name().map(|n| n.to_string_lossy().into_owned());
            for name in &there {
                if name.starts_with(&stem)
                    && name.len() == stem.len() + "0000000000000000.json".len()
                    && Some(name) != ours.as_ref()
                {
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
            Facts { text: Some(None), faces: Some(Ok(None)) },
            Facts {
                text: Some(Some(vec![
                    (Severity::Error, "line 3".into(), "a \"quoted\" word".into()),
                    (Severity::Warning, String::new(), "and 4 more".into()),
                ])),
                faces: Some(Ok(Some(12_345))),
            },
            Facts { text: Some(Some(Vec::new())), faces: Some(Err("can't be read: Empty".into())) },
        ];
        for f in all {
            assert_eq!(read_facts(&write_facts(&f)), Some(f));
        }
        assert_eq!(read_facts(&json!({ "text": 3 })), None);
        assert_eq!(read_facts(&json!({ "text": [["odd", "", ""]] })), None);
    }

    #[test]
    fn an_archive_s_file_is_named_by_its_label_and_fingerprint() {
        assert_eq!(file_name("hak:tdod_tiles01", 0xab), "hak_tdod_tiles01-00000000000000ab.json");
        assert_eq!(file_stem("hak:tdod_tiles01"), "hak_tdod_tiles01-");
    }
}
