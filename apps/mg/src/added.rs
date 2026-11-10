//! Haks, ERFs, modules and folders given on the command line (`--hak`),
//! read over the game's load order; and `mg cat --to`, many resources
//! read from it in one run.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mg_core::ResType;
use mg_resman::{DirContainer, ErfContainer, LayerClass, ResKey, ResMan, priority};
use serde_json::json;

use crate::{Output, path_text};

/// Where the archives and folders given with `--hak` sit in the load
/// order: over every hak (the game's and the user's, a module's too),
/// under the user folder's `development/` and what the game puts above
/// that (`--no-user-dir` leaves the user folder out).
pub(crate) const PRIORITY: u32 = priority::HAK_USER + 1;

/// `--hak PATH`: archives and folders over the game's resources.
#[derive(clap::Args, Debug, Default, Clone)]
pub(crate) struct Added {
    /// A hak, ERF or module (.hak, .erf, .mod) or a folder of loose files
    /// (its own files, not its folders') to read over the game's
    /// resources, as often as needed: the first given wins over the
    /// next, and all of them over the haks the game and a module have.
    #[arg(long = "hak", value_name = "PATH")]
    pub(crate) haks: Vec<PathBuf>,
}

impl Added {
    /// Puts each archive and folder given into `rm`, the first highest;
    /// an error for a path that is neither, or an archive that can't be
    /// read. Each one's layer is labeled with its kind and path
    /// ([`label`]).
    pub(crate) fn stack_onto(&self, rm: &mut ResMan) -> Result<()> {
        // (Layers of one priority rank in the order they were added.)
        for path in &self.haks {
            if path.is_dir() {
                rm.add(PRIORITY, label(path), LayerClass::Directory, DirContainer::open(path));
            } else if path.is_file() {
                let erf = ErfContainer::open(path).with_context(|| path_text(path))?;
                rm.add(PRIORITY, label(path), LayerClass::Erf, erf);
            } else {
                bail!("{}: no such file or folder (--hak)", path.display());
            }
        }
        Ok(())
    }
}

/// The label of a layer `--hak` adds: `folder:PATH`, or the archive's
/// extension and its path (`hak:PATH`, `erf:PATH`, `mod:PATH`), as given.
pub(crate) fn label(path: &Path) -> String {
    let kind = match path.is_dir() {
        true => "folder".to_string(),
        false => crate::extension(path),
    };
    let kind = if kind.is_empty() { "erf" } else { &kind };
    format!("{kind}:{}", path.display())
}

/// The names `mg cat` is given: those on the command line, then a file's
/// (`-`: standard input), a line each, blank lines and lines that begin
/// with `#` passed over; each once, in the order first given.
pub(crate) fn names(given: &[String], list: Option<&Path>) -> Result<Vec<String>> {
    let mut names: Vec<String> = given.to_vec();
    if let Some(list) = list {
        let text = match list == Path::new("-") {
            true => std::io::read_to_string(std::io::stdin()).context("standard input")?,
            false => std::fs::read_to_string(list).with_context(|| path_text(list))?,
        };
        let lines = text.lines().map(str::trim);
        names.extend(lines.filter(|l| !l.is_empty() && !l.starts_with('#')).map(str::to_string));
    }
    let mut seen = std::collections::HashSet::new();
    names.retain(|n| seen.insert(n.to_ascii_lowercase()));
    Ok(names)
}

/// What `mg cat --to` is asked.
pub(crate) struct Many<'a> {
    pub(crate) names: &'a [String],
    pub(crate) dir: &'a Path,
    pub(crate) text: bool,
    pub(crate) force: bool,
    pub(crate) dry_run: bool,
}

/// `mg cat NAME… --to FOLDER`: each resource read from `rm` (its copy the
/// game reads) and written into the folder by its name, with where it
/// came from, its size and its SHA-256; one found nowhere (or no
/// resource's name) is said, and fails the command once the rest are
/// written. Files of those names in the folder are written over only
/// with `force`; `dry_run` writes nothing and reports the same.
pub(crate) fn many(rm: &ResMan, asked: &Many<'_>) -> Result<Output> {
    let dir = asked.dir;
    // Each name: its key, and the layer it is read from (None: nowhere).
    let found: Vec<(&String, Option<ResKey>, Option<usize>)> = (asked.names.iter())
        .map(|n| {
            let key = ResKey::from_filename(n);
            (n, key, key.and_then(|k| rm.find(&k)))
        })
        .collect();
    let existing: Vec<String> = (found.iter())
        .filter_map(|(_, key, at)| at.and(*key))
        .filter_map(|k| k.path_in(dir).ok().filter(|p| p.exists()).map(|_| k.to_string()))
        .collect();
    if !existing.is_empty() && !asked.force && !asked.dry_run {
        bail!(
            "{} has {} of these files already ({}): --force writes over them",
            dir.display(),
            existing.len(),
            existing.join(", ")
        );
    }
    if !asked.dry_run {
        std::fs::create_dir_all(dir).with_context(|| path_text(dir))?;
    }
    // (A body part bound to its skeleton, as text: its bones named from it.)
    let skeleton = |n: &str| rm.get_named(n, ResType::MDL).ok().map(|d| d.into_owned());
    let mut out = Output::default();
    let (mut entries, mut missing) = (Vec::new(), Vec::new());
    for (name, key, at) in found {
        let (Some(key), Some(at)) = (key, at) else {
            let why = match key {
                None => "not a resource's name, like classes.2da",
                Some(_) => "found nowhere",
            };
            out.line(format!("{name}\t{why}"));
            entries.push(json!({ "resource": name, "found": false, "why": why }));
            missing.push(name.clone());
            continue;
        };
        let layer = rm.layers()[at].label.as_str();
        let data = rm.get_in(at, &key).with_context(|| format!("{key} in {layer}"))?;
        let data = match asked.text && key.restype == ResType::MDL {
            true => match mg_mdl::decompile_with(&data, &skeleton) {
                Ok(text) => text.into_bytes().into(),
                Err(mg_mdl::DecompileError::NotBinary) => data,
                Err(e) => bail!("{key}: {e}"),
            },
            false => data,
        };
        let path = key.path_in(dir)?;
        if !asked.dry_run {
            crate::write_one_of_many(&path, &data)?;
        }
        let sha256 = mg_core::sha1::hex(&mg_core::sha256::sha256(&data));
        out.line(format!("{key}\t{layer}\t{}\t{sha256}", data.len()));
        entries.push(json!({
            "resource": key.to_string(),
            "found": true,
            "layer": layer,
            "file": path_text(&path),
            "size": data.len(),
            "sha256": sha256,
        }));
    }
    let done = if asked.dry_run { "would be written" } else { "written" };
    out.note(format!("{} of {} resources {done}", entries.len() - missing.len(), entries.len()));
    if !missing.is_empty() {
        out.note(format!("not found: {}", missing.join(", ")));
        out.failed = true;
    }
    out.json = json!({
        "folder": path_text(dir),
        "written": !asked.dry_run,
        "resources": entries,
        "missing": missing,
    });
    Ok(out)
}
