//! Hak files written from a manifest, for what can't read NWSync: the
//! game's server (`mg serve`, `mg test`) and the game itself on a module
//! of its own (Test Module) load a module's haks from `hak/`, and a
//! manifest is no hak.
//!
//! Which resources go into which hak:
//!
//! - with the manifest's origins file ([`Origins`]), each hak gets the
//!   resources it lists for it: the copy of each name that won from that
//!   hak when the server built the manifest, so the haks give what the
//!   manifest does ([`from_origins`]);
//! - without one, the manifest's resources are spread over the module's
//!   haks by their names, each hak kept under the engine's 2 GiB
//!   ([`spread`]). Each resource is in one hak, so which wins is never in
//!   question.
//!
//! A hak is written once for its content: its file in the cache is named
//! by a SHA-1 of what it holds ([`Planned::key`]), and it is written the
//! same way every time (resources in the order of their names, no build
//! date, no description), so a hak that a newer manifest leaves as it was
//! is the file already written, not written again. Which hak of which
//! manifest is which file is kept beside (`written/<manifest>`,
//! [`Index`]).

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use mg_core::ResType;
use mg_resman::ResKey;

use crate::manifest::Entry;
use crate::{Cache, GameStorage, Origins, Sha1, sha1_hex, unpack};

/// What is hashed before a hak's resources, for its key: a hak written
/// another way some day is another file.
const KEY_VERSION: &str = "moonglow nwsync hak 1\n";

/// A hak to write: its name (as the module lists it) and its resources,
/// in the order they are written (by name and type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub name: String,
    pub resources: Vec<Entry>,
}

impl Planned {
    fn new(name: &str, mut resources: Vec<Entry>) -> Planned {
        resources.sort_by_cached_key(|(k, _, _)| (k.resref.to_lowercase(), k.restype));
        Planned { name: name.to_string(), resources }
    }

    /// What names its file in the cache: a SHA-1 of its resources (name,
    /// type, SHA-1 and size of each).
    pub fn key(&self) -> Sha1 {
        let mut h = mg_core::sha1::Sha1::new();
        h.update(KEY_VERSION.as_bytes());
        for (k, sha, size) in &self.resources {
            let hex = sha1_hex(sha);
            let line = format!("{}.{} {hex} {size}\n", k.resref.to_lowercase(), k.restype.0);
            h.update(line.as_bytes());
        }
        h.finish()
    }

    /// Its resources' bytes, as the manifest says them.
    pub fn bytes(&self) -> u64 {
        self.resources.iter().map(|(_, _, n)| u64::from(*n)).sum()
    }

    /// The archive's layout: name, type and size of each entry.
    fn layout(&self) -> Vec<(mg_core::ResRef, ResType, u64)> {
        self.resources.iter().map(|(k, _, n)| (k.resref, k.restype, u64::from(*n))).collect()
    }

    /// The resources the game could not read, which would start 2 GiB or
    /// further into the file.
    pub fn past_read_limit(&self) -> usize {
        mg_erf::past_read_limit(&header(), &self.layout()).len()
    }
}

/// How a plan was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanFrom {
    /// The manifest's origins file named each hak's resources.
    Origins,
    /// The manifest's resources spread over the haks by their names.
    Spread,
}

/// The haks to write for a module.
#[derive(Debug, Clone)]
pub struct Plan {
    pub haks: Vec<Planned>,
    pub from: PlanFrom,
}

impl Plan {
    /// Every resource of every hak.
    pub fn resources(&self) -> usize {
        self.haks.iter().map(|h| h.resources.len()).sum()
    }

    pub fn bytes(&self) -> u64 {
        self.haks.iter().map(Planned::bytes).sum()
    }
}

/// The resource an origins file names (`resref.ext`, or `resref.<type
/// number>` for a type without an extension).
pub fn origin_key(name: &str) -> Option<ResKey> {
    let (resref, ext) = name.rsplit_once('.')?;
    let restype = ResType::from_extension(ext).or_else(|| ext.parse().ok().map(ResType))?;
    ResKey::parse(resref, restype)
}

/// The manifest's resources by name, the first of a name where it has
/// one twice (as [`crate::NwSyncContainer`] reads them).
fn by_name(entries: &[Entry]) -> HashMap<ResKey, (Sha1, u32)> {
    let mut map = HashMap::with_capacity(entries.len());
    for (key, sha, size) in entries {
        map.entry(*key).or_insert((*sha, *size));
    }
    map
}

/// Says the first few of a list, and how many more.
fn some_of(names: &[String]) -> String {
    let mut said = names.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    if names.len() > 5 {
        said.push_str(&format!(" and {} more", names.len() - 5));
    }
    said
}

/// The haks `names` (those the origins file names; the others are not
/// planned) with the resources the origins file lists for each. An
/// error where it lists a resource the manifest hasn't, or a hak would
/// be too big for the game to read: the origins file is then not the
/// manifest's.
pub fn from_origins(
    origins: &Origins,
    entries: &[Entry],
    names: &[String],
) -> Result<Plan, String> {
    let map = by_name(entries);
    let mut haks = Vec::new();
    for name in names {
        let Some(origin) = origins.hak(name) else { continue };
        let mut resources = Vec::with_capacity(origin.resources.len());
        let mut seen = HashSet::with_capacity(origin.resources.len());
        let mut unknown = Vec::new();
        for listed in &origin.resources {
            let found = origin_key(listed).and_then(|k| map.get(&k).map(|(s, n)| (k, *s, *n)));
            match found {
                Some((k, sha, size)) => {
                    if seen.insert(k) {
                        resources.push((k, sha, size));
                    }
                }
                None => unknown.push(listed.clone()),
            }
        }
        if !unknown.is_empty() {
            return Err(format!(
                "the origins file lists {} resources of hak {name} that the manifest has not ({}): \
                 it is not this manifest's",
                unknown.len(),
                some_of(&unknown)
            ));
        }
        haks.push(Planned::new(name, resources));
    }
    let plan = Plan { haks, from: PlanFrom::Origins };
    too_big(&plan)?;
    Ok(plan)
}

/// An error naming the haks the game couldn't read all of.
fn too_big(plan: &Plan) -> Result<(), String> {
    let big: Vec<String> = (plan.haks.iter())
        .filter(|h| h.past_read_limit() > 0)
        .map(|h| format!("{} ({:.2} GiB)", h.name, h.bytes() as f64 / (1u64 << 30) as f64))
        .collect();
    if big.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "haks would be too big for the game to read (an entry can't start 2 GiB or more into \
             a hak): {}",
            some_of(&big)
        ))
    }
}

/// Where a resource goes when spread: a number of its name alone (the
/// first bytes of its SHA-1), so that it stays in its hak from one
/// manifest to the next and the other haks stay as they were.
fn bucket(key: &ResKey, n: usize) -> usize {
    let name = format!("{}.{}", key.resref.to_lowercase(), key.restype.0);
    let h = mg_core::sha1::sha1(name.as_bytes());
    (u64::from_le_bytes(h[..8].try_into().unwrap_or_default()) % n as u64) as usize
}

/// The manifest's resources spread over the haks `names`, but for talk
/// tables (the game reads those from `tlk/`, not from a hak): each by
/// its name where that keeps every hak under 2 GiB, else the biggest
/// first into the hak with the most room. An error where they don't fit
/// in that many haks. Sizes of 0 (not known) are taken as 0: give the
/// sizes ([`sizes`]) first.
pub fn spread(entries: &[Entry], names: &[String]) -> Result<Plan, String> {
    if names.is_empty() {
        return Err("the module lists no haks to put the manifest's resources in".into());
    }
    let map = by_name(entries);
    let mut all: Vec<Entry> = (map.into_iter())
        .filter(|(k, _)| k.restype != ResType::TLK)
        .map(|(k, (s, n))| (k, s, n))
        .collect();
    all.sort_by_cached_key(|(k, _, _)| (k.resref.to_lowercase(), k.restype));
    let n = names.len();
    let mut parts: Vec<Vec<Entry>> = vec![Vec::new(); n];
    for e in &all {
        parts[bucket(&e.0, n)].push(*e);
    }
    let plan = |parts: Vec<Vec<Entry>>| Plan {
        haks: names.iter().zip(parts).map(|(name, r)| Planned::new(name, r)).collect(),
        from: PlanFrom::Spread,
    };
    let by_name = plan(parts);
    if too_big(&by_name).is_ok() {
        return Ok(by_name);
    }
    // (The biggest first, each into the hak with the most room left.)
    let total: u64 = all.iter().map(|e| u64::from(e.2)).sum();
    all.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    let mut parts: Vec<Vec<Entry>> = vec![Vec::new(); n];
    let mut used = vec![0u64; n];
    for e in all {
        let i = (0..n).min_by_key(|&i| (used[i], i)).unwrap_or(0);
        used[i] += u64::from(e.2);
        parts[i].push(e);
    }
    let packed = plan(parts);
    too_big(&packed).map_err(|_| {
        format!(
            "the manifest's {:.2} GiB don't fit in the module's {n} haks, each under the game's \
             2 GiB: add hak names to the module, or have the server publish its origins file",
            total as f64 / (1u64 << 30) as f64
        )
    })?;
    Ok(packed)
}

/// What reads a manifest's data: the game's storage, then the cache.
#[derive(Debug, Clone, Copy)]
pub struct Store<'a> {
    pub game: Option<&'a GameStorage>,
    pub cache: &'a Cache,
}

impl Store<'_> {
    /// A data file as stored, and whether it is the cache's: `None` where
    /// neither has it.
    fn read_from(&self, sha1: &Sha1) -> Result<Option<(Vec<u8>, bool)>, String> {
        if let Some(g) = self.game
            && let Some(stored) = g.read(sha1)?
        {
            return Ok(Some((stored, false)));
        }
        let cached =
            self.cache.read(sha1).map_err(|e| format!("{}: {e}", self.cache.root().display()))?;
        Ok(cached.map(|c| (c, true)))
    }

    /// A data file as stored: `None` where neither has it.
    pub fn read(&self, sha1: &Sha1) -> Result<Option<Vec<u8>>, String> {
        Ok(self.read_from(sha1)?.map(|(stored, _)| stored))
    }

    /// A resource's bytes, checked against its SHA-1 and size. The cache's
    /// copy of one that doesn't unpack to its SHA-1 (a write cut off) is
    /// taken away, for the next pull to download again; one only the
    /// manifest's size disagrees with stays.
    fn bytes(&self, key: &ResKey, sha1: &Sha1, size: u32) -> Result<Vec<u8>, String> {
        let (stored, cached) = self
            .read_from(sha1)?
            .ok_or_else(|| format!("{key}: its data ({}) is not downloaded yet", sha1_hex(sha1)))?;
        let data = unpack(&stored).and_then(|data| {
            let got = mg_core::sha1::sha1(&data);
            if got == *sha1 {
                Ok(data)
            } else {
                Err(format!("its SHA-1 is {}, not {}", sha1_hex(&got), sha1_hex(sha1)))
            }
        });
        let data = match data {
            Ok(data) => data,
            Err(e) if cached => {
                self.cache.discard(sha1);
                return Err(format!(
                    "{key}: its data in the cache was damaged ({e}) and is removed: download it \
                     again (Sync in Build › NWSync Content, or mg nwsync-pull)"
                ));
            }
            Err(e) => return Err(format!("{key}: {e}")),
        };
        if size != 0 && data.len() != size as usize {
            return Err(format!("{key}: {} bytes, where the manifest says {size}", data.len()));
        }
        Ok(data)
    }
}

/// The entries but for the module's own resources (`module`: the names
/// the module being edited has), for a manifest built with them
/// ([`crate::Found::includes_module`]): the server's copies would
/// otherwise sit over the module's, in a layer or in a hak.
pub fn without_module(entries: &[Entry], module: &HashSet<ResKey>) -> Vec<Entry> {
    entries.iter().filter(|(k, _, _)| !module.contains(k)).copied().collect()
}

/// The SHA-1 of no bytes: a resource of 0 bytes has it (its size of 0 is
/// known, not missing: a persistent world's manifest had one).
pub const EMPTY: Sha1 = [
    0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60, 0x18, 0x90,
    0xaf, 0xd8, 0x07, 0x09,
];

/// Whether an entry's size is not known (0, but for an empty resource).
pub fn size_unknown(entry: &Entry) -> bool {
    entry.2 == 0 && entry.1 != EMPTY
}

/// The size of a resource as stored: a compressed buffer's header says
/// it, else it is the stored bytes'.
fn stored_size(stored: &[u8]) -> Result<u32, String> {
    if stored.starts_with(&mg_erf::compressedbuf::MAGIC_NSYC) && stored.len() >= 16 {
        let mut n = [0u8; 4];
        n.copy_from_slice(&stored[12..16]);
        return Ok(u32::from_le_bytes(n));
    }
    // (Raw bytes, or a buffer too short for its header: unpacked to tell.)
    Ok(unpack(stored)?.len() as u32)
}

/// The entries with their sizes where the manifest doesn't say them (one
/// the game kept without its file): read from the data, which must be
/// here.
pub fn sizes(entries: &[Entry], store: Store<'_>) -> Result<Vec<Entry>, String> {
    let mut known: HashMap<Sha1, u32> = HashMap::new();
    let mut out = Vec::with_capacity(entries.len());
    for (key, sha, size) in entries {
        let size = match (*size, known.get(sha)) {
            (0, _) if *sha == EMPTY => 0,
            (0, Some(n)) => *n,
            (0, None) => {
                let stored = store.read(sha)?.ok_or_else(|| {
                    format!("{key}: its data ({}) is not downloaded yet", sha1_hex(sha))
                })?;
                let n = stored_size(&stored).map_err(|e| format!("{key}: {e}"))?;
                known.insert(*sha, n);
                n
            }
            (n, _) => n,
        };
        out.push((*key, *sha, size));
    }
    Ok(out)
}

/// The header every written hak has: no build date, no description.
fn header() -> mg_erf::Header {
    mg_erf::Header { file_type: *b"HAK ", ..Default::default() }
}

/// Writes a file through a part file beside it, moved into place once
/// whole.
fn write_through_part(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<std::fs::File>) -> Result<(), String>,
) -> Result<(), String> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let part = path.with_file_name(format!(".{name}.{}-{n}.part", std::process::id()));
    let made = (|| {
        let file = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        let mut w = BufWriter::with_capacity(1 << 20, file);
        write(&mut w)?;
        let file = w.into_inner().map_err(|e| format!("{}: {}", part.display(), e.error()))?;
        file.sync_all().map_err(|e| format!("{}: {e}", part.display()))?;
        drop(file);
        crate::cache::into_place(&part, path).map_err(|e| format!("{}: {e}", path.display()))
    })();
    if made.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    made
}

/// Writes a planned hak into the cache, unless the cache has it already:
/// its file, and whether it was written now.
pub fn write_hak(
    hak: &Planned,
    store: Store<'_>,
    progress: &(dyn Fn(u64) + Sync),
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<(PathBuf, bool), String> {
    let path = store.cache.hak_path(&hak.key());
    if path.is_file() {
        return Ok((path, false));
    }
    write_through_part(&path, |w| {
        let layout = hak.layout();
        mg_erf::write_streamed(w, &header(), &layout, |i| {
            if stop() {
                return Err(std::io::Error::other("canceled"));
            }
            let (key, sha, size) = &hak.resources[i];
            let bytes = store.bytes(key, sha, *size).map_err(std::io::Error::other)?;
            progress(bytes.len() as u64);
            Ok(Cow::Owned(bytes))
        })
        .map_err(|e| format!("hak {}: {e}", hak.name))
    })?;
    Ok((path, true))
}

/// Writes a resource as a file of its own into the cache (a talk table,
/// which the game reads from `tlk/`), unless the cache has it: its file.
pub fn write_file(
    key: &ResKey,
    sha1: &Sha1,
    size: u32,
    store: Store<'_>,
) -> Result<PathBuf, String> {
    let ext = key.restype.extension().unwrap_or("res");
    let path = store.cache.file_path(sha1, ext);
    if path.is_file() {
        return Ok(path);
    }
    let bytes = store.bytes(key, sha1, size)?;
    write_through_part(&path, |w| w.write_all(&bytes).map_err(|e| e.to_string()))?;
    Ok(path)
}

/// Which file of the cache is which hak (and talk table) of a manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Index {
    /// Hak names and their keys ([`Planned::key`]).
    pub haks: Vec<(String, Sha1)>,
    /// Files of single resources (`world.tlk`) and their SHA-1s.
    pub files: Vec<(String, Sha1)>,
}

impl Index {
    /// As kept: `hak <name> <key>` and `file <name> <sha1>` lines.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (name, key) in &self.haks {
            out.push_str(&format!("hak {name} {}\n", sha1_hex(key)));
        }
        for (name, sha) in &self.files {
            out.push_str(&format!("file {name} {}\n", sha1_hex(sha)));
        }
        out
    }

    /// Read back; lines it can't read are left out. The name is what lies
    /// between the kind and the SHA-1, the last word, spaces and all.
    pub fn parse(text: &str) -> Index {
        let mut out = Index::default();
        for line in text.lines() {
            let Some((kind, rest)) = line.split_once(' ') else { continue };
            let Some((name, sha)) = rest.rsplit_once(' ') else { continue };
            let Some(sha) = crate::parse_sha1(sha) else { continue };
            match kind {
                "hak" => out.haks.push((name.to_string(), sha)),
                "file" => out.files.push((name.to_string(), sha)),
                _ => {}
            }
        }
        out
    }

    /// Merged with what was kept before for the same manifest (another
    /// module of the same server, other haks): this one's over it.
    pub fn merged(mut self, before: &Index) -> Index {
        for (name, key) in &before.haks {
            if !self.haks.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) {
                self.haks.push((name.clone(), *key));
            }
        }
        for (name, sha) in &before.files {
            if !self.files.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) {
                self.files.push((name.clone(), *sha));
            }
        }
        self
    }
}

/// A hak's file, and whether it was written now; or why not.
type HakWritten = Result<(PathBuf, bool), String>;

/// How far a write is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    /// Haks done (written or already there), of how many.
    pub haks: usize,
    pub of_haks: usize,
    /// Bytes of resources written, of how many there are to write (the
    /// haks not already there).
    pub bytes: u64,
    pub of_bytes: u64,
}

/// A hak written, or found written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenHak {
    pub name: String,
    pub path: PathBuf,
    pub resources: usize,
    pub bytes: u64,
    /// Written now (else it was in the cache: written before, for this
    /// manifest or another).
    pub written: bool,
}

/// What [`write_all`] did.
#[derive(Debug, Clone, Default)]
pub struct Written {
    pub haks: Vec<WrittenHak>,
    pub took: Duration,
}

impl Written {
    /// The haks written now, and their bytes.
    pub fn written(&self) -> (usize, u64) {
        let now = self.haks.iter().filter(|h| h.written);
        (now.clone().count(), now.map(|h| h.bytes).sum())
    }
}

/// Writes the plan's haks into the cache, `jobs` at a time, each one
/// the cache hasn't (the others are taken as they are); `progress` hears
/// how far it is, from the threads; `stop` ends it. Data missing is an
/// error before anything is written ([`missing`]).
pub fn write_all(
    plan: &Plan,
    store: Store<'_>,
    jobs: usize,
    progress: &(dyn Fn(Progress) + Sync),
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<Written, String> {
    let started = Instant::now();
    let to_write: Vec<&Planned> =
        plan.haks.iter().filter(|h| !store.cache.hak_path(&h.key()).is_file()).collect();
    let lacking = missing(to_write.iter().flat_map(|h| &h.resources), store)?;
    if !lacking.is_empty() {
        let bytes: u64 = lacking.iter().map(|(_, n)| u64::from(*n)).sum();
        return Err(format!(
            "{} data files ({:.1} MB) of the haks to write are not downloaded yet: Sync in Build › \
             NWSync Content, or run mg nwsync-pull",
            lacking.len(),
            bytes as f64 / 1_048_576.0
        ));
    }
    let of_bytes: u64 = to_write.iter().map(|h| h.bytes()).sum();
    let (done, bytes) = (AtomicUsize::new(0), AtomicU64::new(0));
    let say = |haks: usize| {
        progress(Progress {
            haks,
            of_haks: plan.haks.len(),
            bytes: bytes.load(Ordering::Relaxed),
            of_bytes,
        })
    };
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, HakWritten)>> = Mutex::new(Vec::new());
    let work = || {
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            let Some(hak) = plan.haks.get(i) else { return };
            if stop() {
                results.lock().unwrap_or_else(|e| e.into_inner()).push((i, Err("canceled".into())));
                return;
            }
            let wrote = |n: u64| {
                bytes.fetch_add(n, Ordering::Relaxed);
                say(done.load(Ordering::Relaxed));
            };
            let got = write_hak(hak, store, &wrote, stop);
            let failed = got.is_err();
            results.lock().unwrap_or_else(|e| e.into_inner()).push((i, got));
            say(done.fetch_add(1, Ordering::Relaxed) + 1);
            if failed {
                return;
            }
        }
    };
    std::thread::scope(|s| {
        for _ in 0..jobs.clamp(1, 16).min(plan.haks.len().max(1)) {
            s.spawn(work);
        }
    });
    let mut results = results.into_inner().unwrap_or_else(|e| e.into_inner());
    results.sort_by_key(|(i, _)| *i);
    if stop() {
        return Err("canceled".into());
    }
    let mut haks = Vec::with_capacity(plan.haks.len());
    for (i, got) in results {
        let (path, written) = got?;
        let h = &plan.haks[i];
        haks.push(WrittenHak {
            name: h.name.clone(),
            path,
            resources: h.resources.len(),
            bytes: h.bytes(),
            written,
        });
    }
    if haks.len() != plan.haks.len() {
        return Err("not every hak was written".into());
    }
    Ok(Written { haks, took: started.elapsed() })
}

/// The data files of `resources` that neither the game's storage nor the
/// cache has, with their sizes.
pub fn missing<'e>(
    resources: impl IntoIterator<Item = &'e Entry>,
    store: Store<'_>,
) -> Result<Vec<(Sha1, u32)>, String> {
    let entries: Vec<Entry> = resources.into_iter().copied().collect();
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    Ok(crate::survey(&entries, store.game, Some(store.cache))?.missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> ResKey {
        let (resref, ext) = name.rsplit_once('.').unwrap();
        ResKey::parse(resref, ResType::from_extension(ext).unwrap()).unwrap()
    }

    fn entry(name: &str, size: u32) -> Entry {
        (key(name), mg_core::sha1::sha1(name.as_bytes()), size)
    }

    #[test]
    fn origins_names_are_resources() {
        assert_eq!(origin_key("pfh0_head001.mdl"), Some(key("pfh0_head001.mdl")));
        assert_eq!(origin_key("PFH0_Head001.MDL"), Some(key("pfh0_head001.mdl")));
        assert_eq!(origin_key("thing.9999"), ResKey::parse("thing", ResType(9999)));
        assert_eq!(origin_key("noext"), None);
        assert_eq!(origin_key("x.nosuchtype"), None);
        assert_eq!(origin_key("a_name_far_longer_than_sixteen.mdl"), None);
    }

    /// Each hak named gets what the origins file lists for it, in the
    /// order of names; a hak it doesn't name isn't planned; a resource it
    /// lists that the manifest hasn't is an error.
    #[test]
    fn a_plan_from_origins() {
        let entries =
            [entry("b.mdl", 3), entry("a.mdl", 2), entry("c.2da", 1), entry("mine.tlk", 9)];
        let o = Origins::parse("Erf:/srv/hak/one.hak\n\tb.mdl\n\ta.mdl\nErf:two.hak\n\tc.2da\n");
        let names = vec!["One".to_string(), "three".to_string(), "two".to_string()];
        let plan = from_origins(&o, &entries, &names).unwrap();
        assert_eq!(plan.from, PlanFrom::Origins);
        assert_eq!(plan.haks.len(), 2);
        assert_eq!(plan.haks[0].name, "One");
        assert_eq!(plan.haks[0].resources, [entry("a.mdl", 2), entry("b.mdl", 3)]);
        assert_eq!(plan.haks[1].resources, [entry("c.2da", 1)]);
        assert_eq!((plan.resources(), plan.bytes()), (3, 6));
        let o = Origins::parse("Erf:one.hak\n\tb.mdl\n\tgone.mdl\n");
        let e = from_origins(&o, &entries, &names).unwrap_err();
        assert!(e.contains("1 resources of hak One") && e.contains("gone.mdl"), "{e}");
    }

    /// A hak's key is its content's: the same resources (in any order)
    /// the same key; another SHA-1 or size, another key.
    #[test]
    fn a_hak_s_key_is_its_content() {
        let a = Planned::new("a", vec![entry("x.mdl", 1), entry("y.mdl", 2)]);
        let b = Planned::new("b", vec![entry("y.mdl", 2), entry("x.mdl", 1)]);
        assert_eq!(a.key(), b.key());
        let c = Planned::new("a", vec![entry("x.mdl", 1), entry("y.mdl", 3)]);
        assert_ne!(a.key(), c.key());
        let mut d = a.clone();
        d.resources[0].1 = mg_core::sha1::sha1(b"other");
        assert_ne!(a.key(), d.key());
    }

    /// Spread: every resource in one hak, talk tables in none; a resource
    /// stays in its hak when others come and go; too much for the haks to
    /// hold under 2 GiB is an error.
    #[test]
    fn a_spread_plan() {
        let names: Vec<String> = ["h1", "h2", "h3"].iter().map(|s| s.to_string()).collect();
        let mut entries: Vec<Entry> = (0..300).map(|i| entry(&format!("r{i}.mdl"), 10)).collect();
        entries.push(entry("mine.tlk", 5));
        let plan = spread(&entries, &names).unwrap();
        assert_eq!(plan.from, PlanFrom::Spread);
        assert_eq!(plan.resources(), 300);
        assert!(plan.haks.iter().all(|h| !h.resources.is_empty()));
        // (One more resource: the other haks as they were.)
        entries.push(entry("new.mdl", 10));
        let more = spread(&entries, &names).unwrap();
        let same = (0..3).filter(|&i| plan.haks[i] == more.haks[i]).count();
        assert_eq!(same, 2);
        assert!(spread(&entries, &[]).is_err());

        // Big resources: by name they'd overflow one hak, packed they fit.
        let gib = 1u32 << 30;
        let big = [entry("a.tga", gib - 4096), entry("b.tga", gib - 4096), entry("c.tga", 4096)];
        let two = vec!["x".to_string(), "y".to_string()];
        let plan = spread(&big, &two).unwrap();
        assert!(plan.haks.iter().all(|h| h.past_read_limit() == 0));
        // Five of a GiB in two haks can't be: one would have three, the
        // third starting past 2 GiB.
        let five: Vec<Entry> = (0..5).map(|i| entry(&format!("g{i}.tga"), gib)).collect();
        let e = spread(&five, &two).unwrap_err();
        assert!(e.contains("don't fit in the module's 2 haks"), "{e}");
    }

    #[test]
    fn an_index_is_kept_and_read_back() {
        let sha = mg_core::sha1::sha1(b"x");
        let other = mg_core::sha1::sha1(b"y");
        let i = Index {
            haks: vec![("pw_parts".into(), sha), ("PW_VFX".into(), other)],
            files: vec![("world.tlk".into(), other)],
        };
        assert_eq!(Index::parse(&i.to_text()), i);
        assert_eq!(Index::parse("nonsense\nhak a\nhak a b c\n"), Index::default());
        let before =
            Index { haks: vec![("pw_vfx".into(), sha), ("old".into(), sha)], files: vec![] };
        let m = i.clone().merged(&before);
        assert_eq!(m.haks.len(), 3);
        assert_eq!(m.haks[1], ("PW_VFX".into(), other));
        assert_eq!(m.haks[2], ("old".into(), sha));
    }

    /// A hak named with a space is kept in the index and read back; an
    /// index written before reads as it did.
    #[test]
    fn a_hak_named_with_a_space_is_kept() {
        let sha = mg_core::sha1::sha1(b"x");
        let i = Index {
            haks: vec![("my parts".into(), sha), (" odd  name ".into(), sha)],
            files: vec![("my world.tlk".into(), sha)],
        };
        assert_eq!(Index::parse(&i.to_text()), i);
        let old = format!("hak pw_parts {}\nfile world.tlk {}\n", sha1_hex(&sha), sha1_hex(&sha));
        let read = Index::parse(&old);
        assert_eq!(read.haks, [("pw_parts".to_string(), sha)]);
        assert_eq!(read.files, [("world.tlk".to_string(), sha)]);
    }

    /// A data file of the cache that doesn't unpack to what it is named
    /// (a write cut off by a crash) is taken away when it is read, so
    /// that the next pull downloads it again.
    #[test]
    fn a_bad_data_file_of_the_cache_is_taken_away() {
        let cache = Cache::new(mg_testkit::scratch_dir("nwsync-haks-bad-data"));
        let data = b"a talk table, a talk table, a talk table".to_vec();
        let sha = mg_core::sha1::sha1(&data);
        let packed =
            mg_erf::compressedbuf::compress(&data, mg_erf::compressedbuf::MAGIC_NSYC).unwrap();
        cache.put(&sha, &packed[..packed.len() - 4]).unwrap();
        let store = Store { game: None, cache: &cache };
        let k = key("world.tlk");
        let e = write_file(&k, &sha, data.len() as u32, store).unwrap_err();
        assert!(e.contains("download"), "{e}");
        assert!(!cache.contains(&sha), "the cut file is still counted as here");
        // (A size the manifest disagrees with alone: the file stays.)
        cache.put(&sha, &packed).unwrap();
        assert!(write_file(&k, &sha, 3, store).is_err());
        assert!(cache.contains(&sha));
        assert!(write_file(&k, &sha, data.len() as u32, store).is_ok());
    }

    /// The module's own resources left out of a manifest built with them;
    /// the others as they were, in their order.
    #[test]
    fn a_module_s_resources_are_left_out() {
        let entries = [entry("area1.are", 1), entry("pw_head.mdl", 2), entry("area1.git", 3)];
        let module: HashSet<ResKey> = [key("area1.are"), key("area1.git"), key("x.nss")].into();
        assert_eq!(without_module(&entries, &module), [entry("pw_head.mdl", 2)]);
        assert_eq!(without_module(&entries, &HashSet::new()), entries);
    }

    #[test]
    fn an_empty_resource_s_size_is_known() {
        assert_eq!(EMPTY, mg_core::sha1::sha1(b""));
        let empty = (key("spikes.mtr"), EMPTY, 0);
        assert!(!size_unknown(&empty));
        assert!(size_unknown(&(key("x.mdl"), mg_core::sha1::sha1(b"x"), 0)));
        let cache = Cache::new(mg_testkit::scratch_dir("nwsync-haks-empty"));
        let store = Store { game: None, cache: &cache };
        assert_eq!(sizes(&[empty], store).unwrap(), [empty]);
    }

    #[test]
    fn a_stored_size_is_read_from_its_header() {
        let data = b"some bytes, some bytes, some bytes".to_vec();
        let packed =
            mg_erf::compressedbuf::compress(&data, mg_erf::compressedbuf::MAGIC_NSYC).unwrap();
        assert_eq!(stored_size(&packed).unwrap(), data.len() as u32);
        assert_eq!(stored_size(&data).unwrap(), data.len() as u32);
    }
}
