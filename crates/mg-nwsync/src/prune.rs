//! Moonglow's cache pruned: what no manifest still kept needs goes (data
//! files, written haks and talk tables, manifests and their side files).
//! The game's NWSync storage is never touched.
//!
//! Kept: the manifests named ([`Keep::manifests`]: the module's own, the
//! one pinned), the newest each server named ([`Cache::latest`]), and the
//! last used ([`Keep::recent`]; `manifests/`, `written/` and the marks
//! [`Cache::touch`] leaves in `used/`). What was changed within
//! [`Keep::min_age`] stays whatever it is, for a download or a write may
//! be under way. Where a named manifest's resources can't be read (the
//! cache hasn't its file, the game's storage hasn't it either, or wasn't
//! looked in), no data file goes: which it needs is not known. One of the
//! newest or last used that can't be read is passed over with a note
//! ([`Pruned::notes`]): a server's `latest/` naming a manifest never
//! downloaded would otherwise stop every prune after it. (A manifest used
//! from the game's storage alone is read only where the prune is given
//! that storage.)
//!
//! This is the builder's cache, not a server's repository: publishing's
//! `mg nwsync-prune` prunes that.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::{Cache, GameStorage, Sha1, manifest, parse_sha1, sha1_hex};

/// How many manifests last used are kept, unless told.
pub const RECENT: usize = 3;
/// How long what was just changed is left alone, unless told.
pub const MIN_AGE: Duration = Duration::from_secs(60 * 60);

/// What a prune keeps.
#[derive(Debug, Clone)]
pub struct Keep {
    /// Manifests kept whatever else (the module's current one).
    pub manifests: Vec<Sha1>,
    /// How many manifests last used are kept besides.
    pub recent: usize,
    /// Files changed more recently than this are left alone.
    pub min_age: Duration,
}

impl Default for Keep {
    fn default() -> Keep {
        Keep { manifests: Vec::new(), recent: RECENT, min_age: MIN_AGE }
    }
}

/// What a prune took away (or would, in a dry run).
#[derive(Debug, Clone, Default)]
pub struct Pruned {
    /// The manifests kept, and those removed.
    pub kept: Vec<Sha1>,
    pub removed: Vec<Sha1>,
    /// Data files removed, and their bytes.
    pub data_files: usize,
    pub data_bytes: u64,
    /// Written haks and talk tables removed, and their bytes.
    pub haks: usize,
    pub hak_bytes: u64,
    /// Other files removed (manifests, their side files and indexes,
    /// parts of writes cut off), and their bytes.
    pub other: usize,
    pub other_bytes: u64,
    /// Why no data file was removed, where that is so.
    pub data_left: Option<String>,
    /// Kept manifests passed over (one in `latest/` or among the last
    /// used that can't be read here), each said once.
    pub notes: Vec<String>,
    pub dry_run: bool,
}

impl Pruned {
    /// Every byte removed.
    pub fn bytes(&self) -> u64 {
        self.data_bytes + self.hak_bytes + self.other_bytes
    }
}

/// A file's last change, and its size.
fn stat(path: &Path) -> Option<(SystemTime, u64)> {
    let m = std::fs::symlink_metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// The files of a folder.
fn files(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<PathBuf> = read
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

/// The SHA-1 a file of the cache is named by (`<sha1>`, `<sha1>.ext`).
fn named(path: &Path) -> Option<Sha1> {
    let name = path.file_name()?.to_str()?;
    parse_sha1(name.split('.').next()?)
}

fn is_part(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.') && n.ends_with(".part"))
}

/// Prunes the cache (in a dry run, says what would go).
pub fn prune(
    cache: &Cache,
    game: Option<&GameStorage>,
    keep: &Keep,
    dry_run: bool,
) -> Result<Pruned, String> {
    let root = cache.root();
    let now = SystemTime::now();
    let old = |at: SystemTime| now.duration_since(at).unwrap_or_default() >= keep.min_age;
    // The manifests here, the last used first.
    let mut here: Vec<(Sha1, SystemTime)> = Vec::new();
    let marked = || {
        let [m, w, u] = ["manifests", "written", "used"].map(|d| files(&root.join(d)));
        m.into_iter().chain(w).chain(u)
    };
    for path in marked() {
        let (Some(sha), Some((at, _))) = (named(&path), stat(&path)) else { continue };
        match here.iter_mut().find(|(s, _)| *s == sha) {
            Some((_, t)) => *t = (*t).max(at),
            None => here.push((sha, at)),
        }
    }
    here.sort_by_key(|(_, at)| std::cmp::Reverse(*at));
    let mut kept: Vec<Sha1> = keep.manifests.clone();
    for path in files(&root.join("latest")) {
        let sha = std::fs::read_to_string(&path).ok().and_then(|t| parse_sha1(t.trim()));
        kept.extend(sha);
    }
    kept.extend(here.iter().take(keep.recent).map(|(s, _)| *s));
    let mut seen = HashSet::new();
    kept.retain(|s| seen.insert(*s));
    let kept_set: HashSet<Sha1> = kept.iter().copied().collect();

    // What the kept manifests need.
    let mut data: HashSet<Sha1> = HashSet::new();
    let mut data_left = None;
    let mut notes = Vec::new();
    let mut written: HashSet<Sha1> = HashSet::new();
    for sha in &kept {
        let index = cache.written(sha);
        written.extend(index.haks.iter().chain(&index.files).map(|(_, s)| *s));
        let bytes = cache.manifest(sha, "").filter(|b| mg_core::sha1::sha1(b) == *sha);
        let entries = match bytes.map(|b| manifest::read(&b)) {
            Some(Ok(e)) => Some(e),
            _ => match game {
                Some(g) => g.manifest_entries(sha)?,
                None => None,
            },
        };
        match entries {
            Some(e) => data.extend(e.iter().map(|(_, s, _)| *s)),
            None if keep.manifests.contains(sha) => {
                data_left = Some(format!(
                    "manifest {} is kept and its resources can't be read here (neither the cache \
                     nor the game's NWSync storage has it), so no data file is removed",
                    &sha1_hex(sha)[..8]
                ));
            }
            None => notes.push(format!(
                "manifest {} (a server's latest, or used lately) can't be read here (neither the \
                 cache nor the game's NWSync storage has it): passed over",
                &sha1_hex(sha)[..8]
            )),
        }
    }

    let mut out = Pruned { kept, data_left, notes, dry_run, ..Default::default() };
    let remove = |path: &Path| -> Result<(), String> {
        if dry_run {
            return Ok(());
        }
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    };

    // Data files.
    let data_root = root.join("data/sha1");
    for a in std::fs::read_dir(&data_root).into_iter().flatten().flatten() {
        for b in std::fs::read_dir(a.path()).into_iter().flatten().flatten() {
            for path in files(&b.path()) {
                let Some((at, len)) = stat(&path) else { continue };
                if !old(at) {
                    continue;
                }
                if is_part(&path) {
                    remove(&path)?;
                    out.other += 1;
                    out.other_bytes += len;
                    continue;
                }
                let Some(sha) = named(&path) else { continue };
                if out.data_left.is_some() || data.contains(&sha) {
                    continue;
                }
                remove(&path)?;
                out.data_files += 1;
                out.data_bytes += len;
            }
            if !dry_run {
                let _ = std::fs::remove_dir(b.path());
            }
        }
        if !dry_run {
            let _ = std::fs::remove_dir(a.path());
        }
    }

    // Written haks and talk tables.
    for path in files(&root.join("haks")) {
        let Some((at, len)) = stat(&path) else { continue };
        if !old(at) {
            continue;
        }
        if is_part(&path) {
            remove(&path)?;
            out.other += 1;
            out.other_bytes += len;
            continue;
        }
        if named(&path).is_some_and(|s| !written.contains(&s)) {
            remove(&path)?;
            out.haks += 1;
            out.hak_bytes += len;
        }
    }

    // Manifests not kept, their side files, indexes and marks.
    let mut removed = HashSet::new();
    for path in marked() {
        let Some((at, len)) = stat(&path) else { continue };
        if !old(at) {
            continue;
        }
        let gone = is_part(&path) || named(&path).is_some_and(|s| !kept_set.contains(&s));
        if gone {
            remove(&path)?;
            out.other += 1;
            out.other_bytes += len;
            removed.extend(named(&path));
        }
    }
    out.removed = here.iter().map(|(s, _)| *s).filter(|s| removed.contains(s)).collect();
    Ok(out)
}
