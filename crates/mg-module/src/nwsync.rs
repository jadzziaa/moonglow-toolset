//! Publishing a module's content for NWSync: the repository a web server
//! hands to players, which the game downloads before joining a server
//! (`-nwsyncurl`). Written as neverwinter.nim's `nwn_nwsync_write` writes
//! it, byte for byte in the manifest (`tests/nwsync.rs`):
//!
//! - `data/sha1/ab/cd/<sha1>`: each resource once, named by the SHA-1 of
//!   its bytes, zstd-compressed (`NSYC`);
//! - `manifests/<sha1>`: the manifest (`NSYM`, version 3): every resource's
//!   name and type, size and SHA-1, sorted by SHA-1, name and type, with
//!   resources of the same bytes listed once and mapped to the first;
//! - `manifests/<sha1>.json`: what it is (name, sizes, when);
//! - `latest`: the newest manifest's SHA-1.
//!
//! A module's manifest holds what players need: its haks' resources (the
//! first-listed hak's copy where several have one) and its talk table, but
//! not scripts' source and debug information or area comments (`nss`,
//! `ndb`, `gic`). With the module itself (for single-player modules, not
//! persistent worlds), its own resources come under the haks', and its
//! `module.ifo` without its hak list (the haks' files are in the manifest)
//! and with a UUID.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mg_core::ResType;
use mg_core::sha1::{hex, sha1};
use mg_erf::compressedbuf::{MAGIC_NSYC, compress};
use mg_resman::{Container, ErfContainer, GameInstall, ResKey, ResMan};

use crate::Module;

/// Types players don't need: scripts' source, their debug information and
/// areas' comments.
pub const SKIPPED: [ResType; 3] = [ResType::NSS, ResType::NDB, ResType::GIC];

/// nwn_nwsync_write's default limit on one file: 15 MB.
pub const FILE_LIMIT: u64 = 15 * 1024 * 1024;

/// Where a resource's bytes are.
#[derive(Debug, Clone)]
pub enum Source {
    Erf(Arc<ErfContainer>),
    File(PathBuf),
    Bytes(Arc<[u8]>),
}

impl Source {
    fn read(&self, key: &ResKey) -> Result<Vec<u8>, String> {
        match self {
            Source::Erf(c) => c.read(key).map(|d| d.into_owned()).map_err(|e| e.to_string()),
            Source::File(p) => std::fs::read(p).map_err(|e| format!("{}: {e}", p.display())),
            Source::Bytes(b) => Ok(b.to_vec()),
        }
    }
}

/// What a manifest is, beyond its resources.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The module's own resources too (single-player distribution).
    pub with_module: bool,
    pub name: String,
    pub description: String,
    /// The module's UUID, with the module (`Mod_UUID`; made when it has
    /// none).
    pub uuid: Option<String>,
    /// Servers sharing a repository: the game removes a group's older
    /// downloads.
    pub group_id: u32,
    /// Point `latest` at it.
    pub latest: bool,
    /// The largest file allowed (`None`: no limit).
    pub limit: Option<u64>,
    /// Write data files that exist already again.
    pub force: bool,
    /// Work everything out; write nothing.
    pub dry_run: bool,
}

/// A manifest written.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    /// The manifest's SHA-1: what `latest` names and `-nwsynchash` takes.
    pub sha1: String,
    pub files: usize,
    pub bytes: u64,
    /// The compressed data files' size, those written now and before.
    pub on_disk: u64,
    /// Data files written now (not there already).
    pub new_files: usize,
}

/// Resources and where their bytes are, in priority order.
pub type Contents = Vec<(ResKey, Source)>;

/// The resources a module's manifest holds, in priority order (the first
/// of a name wins), and the haks found nowhere.
pub fn module_contents(
    m: &Module,
    install: &GameInstall,
    resman: &ResMan,
    with_module: bool,
    uuid: Option<&str>,
) -> Result<(Contents, Vec<String>), String> {
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for name in m.haks().map_err(|e| e.to_string())? {
        let Some(path) = install.find_hak(&name) else {
            missing.push(name);
            continue;
        };
        let c = Arc::new(ErfContainer::open(&path).map_err(|e| e.to_string())?);
        for &(key, _) in c.index() {
            out.push((key, Source::Erf(c.clone())));
        }
    }
    if let Some(name) = m.custom_tlk().map_err(|e| e.to_string())?
        && let Some(found) = crate::talk::find_for(
            m,
            resman,
            &install.tlk_dirs(),
            &name,
            mg_core::Language::from_short_code(&install.language),
        )?
        && let Some(key) = ResKey::parse(&name, ResType::TLK)
    {
        // A table in a hak is among the hak's resources already; a
        // project's JSON goes as the game reads it.
        out.push((key, Source::Bytes(found.tlk_bytes()?.into())));
    }
    if with_module {
        for key in m.keys().copied() {
            let Some(data) = m.get(&key) else { continue };
            let data: Arc<[u8]> = if key == Module::info_key() {
                let mut ifo = m.info().map_err(|e| e.to_string())?;
                // As nwn_nwsync_write: the list goes, the old single
                // Mod_Hak stays.
                ifo.root.fields.retain(|f| f.label.to_string_lossy() != "Mod_HakList");
                let uuid = uuid.map(str::to_string).unwrap_or_else(new_uuid);
                ifo.root.set("Mod_UUID", mg_gff::Value::String(uuid.into_bytes()));
                ifo.to_bytes().map_err(|e| e.to_string())?.into()
            } else {
                data.into()
            };
            out.push((key, Source::Bytes(data)));
        }
    }
    Ok((out, missing))
}

/// A random version 4 UUID.
pub fn new_uuid() -> String {
    let mut b: [u8; 16] = std::array::from_fn(|_| fastrand::u8(..));
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h = hex(&b);
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// The manifest's bytes for resources (name and type, SHA-1, size).
pub fn manifest(entries: &[(ResKey, [u8; 20], u32)]) -> Vec<u8> {
    use mg_core::bin::WriteLe;
    let mut sorted: Vec<(&[u8; 20], String, u16, u32)> = entries
        .iter()
        .map(|(k, sha, size)| (sha, k.resref.to_lowercase().to_string(), k.restype.0, *size))
        .collect();
    sorted.sort_by(|a, b| (a.0, &a.1, a.2).cmp(&(b.0, &b.1, b.2)));
    let mut unique: Vec<&(&[u8; 20], String, u16, u32)> = Vec::new();
    let mut mappings = Vec::new();
    let mut first: BTreeMap<&[u8; 20], usize> = BTreeMap::new();
    for e in &sorted {
        match first.get(e.0) {
            Some(&i) => mappings.push((i, e)),
            None => {
                first.insert(e.0, unique.len());
                unique.push(e);
            }
        }
    }
    let mut out = Vec::new();
    out.put_bytes(b"NSYM");
    out.put_u32(3);
    out.put_u32_usize(unique.len());
    out.put_u32_usize(mappings.len());
    for (sha, name, restype, size) in unique {
        out.put_bytes(&sha[..]);
        out.put_u32(*size);
        out.put_fixed(name.as_bytes(), 16);
        out.put_u16(*restype);
    }
    for (i, (_, name, restype, _)) in mappings {
        out.put_u32_usize(i);
        out.put_fixed(name.as_bytes(), 16);
        out.put_u16(*restype);
    }
    out
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let part = path.with_extension("part");
    std::fs::write(&part, data)
        .and_then(|()| std::fs::rename(&part, path))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes a manifest of `contents` (the first of a name winning) into the
/// repository `root`, with each resource's data file. `progress` hears of
/// each resource done (of how many).
pub fn write(
    root: &Path,
    contents: &[(ResKey, Source)],
    o: &Options,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<Written, String> {
    write_until(root, contents, o, progress, &|| false)
}

/// What [`write_until`] says when `stop` stopped it.
pub const CANCELED: &str = "canceled";

/// [`write`], stopping before the next resource once `stop` says so (the
/// window's Cancel): then no manifest is written, `latest` stays as it
/// was, and the error is [`CANCELED`]. The data files written by then are
/// kept: publishing again finds them, and [`prune`] takes away those no
/// manifest names.
pub fn write_until(
    root: &Path,
    contents: &[(ResKey, Source)],
    o: &Options,
    progress: &mut dyn FnMut(usize, usize),
    stop: &dyn Fn() -> bool,
) -> Result<Written, String> {
    let mut seen = HashSet::new();
    let wanted: Vec<&(ResKey, Source)> = contents
        .iter()
        .filter(|(k, _)| !SKIPPED.contains(&k.restype))
        .filter(|(k, _)| seen.insert(ResKey::new(k.resref.to_lowercase(), k.restype)))
        .collect();
    if wanted.is_empty() {
        return Err(
            "nothing to publish: the module has no haks, talk table or (with the module) resources"
                .into(),
        );
    }
    let mut entries = Vec::with_capacity(wanted.len());
    let (mut bytes, mut on_disk, mut new_files) = (0u64, 0u64, 0usize);
    let mut done: HashSet<[u8; 20]> = HashSet::new();
    for (i, (key, source)) in wanted.iter().enumerate() {
        if stop() {
            return Err(CANCELED.into());
        }
        let data = source.read(key)?;
        if let Some(limit) = o.limit
            && data.len() as u64 > limit
        {
            return Err(format!(
                "{key} is {} MB, over the limit of {} MB a file",
                data.len() / (1 << 20),
                limit / (1 << 20)
            ));
        }
        let size = u32::try_from(data.len()).map_err(|_| format!("{key} is over 4 GiB"))?;
        let digest = sha1(&data);
        bytes += u64::from(size);
        entries.push((*key, digest, size));
        if done.insert(digest) {
            let h = hex(&digest);
            let path = root.join("data/sha1").join(&h[0..2]).join(&h[2..4]).join(&h);
            match std::fs::metadata(&path) {
                Ok(meta) if !o.force => on_disk += meta.len(),
                _ => {
                    let packed = compress(&data, MAGIC_NSYC);
                    on_disk += packed.len() as u64;
                    new_files += 1;
                    if !o.dry_run {
                        write_atomic(&path, &packed)?;
                    }
                }
            }
        }
        progress(i + 1, wanted.len());
    }
    let bytes_of_manifest = manifest(&entries);
    let digest = hex(&sha1(&bytes_of_manifest));
    let created = mg_core::time::unix_now();
    let mut json = serde_json::json!({
        "version": 3,
        "sha1": digest,
        "hash_tree_depth": 2,
        "module_name": o.name,
        "description": o.description,
        "includes_module_contents": o.with_module,
        "includes_client_contents": true,
        "total_files": entries.len(),
        "total_bytes": bytes,
        "on_disk_bytes": on_disk,
        "created": created,
        "created_with": format!("Moonglow Toolset {}", mg_core::release::version()),
    });
    if o.group_id != 0 {
        json["group_id"] = o.group_id.into();
    }
    if o.with_module {
        json["uuid"] = o.uuid.clone().into();
    }
    if !o.dry_run {
        let manifests = root.join("manifests");
        write_atomic(&manifests.join(&digest), &bytes_of_manifest)?;
        let text = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
        write_atomic(&manifests.join(format!("{digest}.json")), text.as_bytes())?;
        if o.latest {
            write_atomic(&root.join("latest"), digest.as_bytes())?;
        }
    }
    Ok(Written { sha1: digest, files: entries.len(), bytes, on_disk, new_files })
}

/// Reads a manifest back: each resource with its SHA-1 and size.
pub fn read_manifest(data: &[u8]) -> Result<Vec<(ResKey, [u8; 20], u32)>, String> {
    let mut r = mg_core::bin::Reader::new(data);
    let err = |e: mg_core::bin::BinError| format!("not a manifest: {e}");
    if r.bytes(4).map_err(err)? != b"NSYM" {
        return Err("not an NWSync manifest".into());
    }
    if r.u32().map_err(err)? != 3 {
        return Err("not a version 3 manifest".into());
    }
    let (n, mapped) = (r.u32_usize().map_err(err)?, r.u32_usize().map_err(err)?);
    let mut out: Vec<(ResKey, [u8; 20], u32)> = Vec::with_capacity(n.min(1 << 20));
    let name = |r: &mut mg_core::bin::Reader<'_>| -> Result<ResKey, String> {
        let resref = mg_core::ResRef::from_bytes(r.fixed_str(16).map_err(err)?)
            .map_err(|e| e.to_string())?;
        Ok(ResKey::new(resref, ResType(r.u16().map_err(err)?)))
    };
    for _ in 0..n {
        let sha: [u8; 20] = r.array().map_err(err)?;
        let size = r.u32().map_err(err)?;
        out.push((name(&mut r)?, sha, size));
    }
    for _ in 0..mapped {
        let i = r.u32_usize().map_err(err)?;
        let (_, sha, size) = *out.get(i).ok_or("a mapping past the entries")?;
        out.push((name(&mut r)?, sha, size));
    }
    Ok(out)
}

/// How old a data file no manifest names must be to be pruned, as
/// `nwn_nwsync_prune` has it by default: two weeks (a file written by a
/// publish still under way, or canceled a moment ago, is kept).
pub const PRUNE_MIN_AGE: u64 = 14 * 24 * 60 * 60;

/// What [`prune`] found in a repository and did (or, a dry run, would do).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pruned {
    /// The manifests read.
    pub manifests: usize,
    /// Data files no manifest names, removed.
    pub removed: Vec<PathBuf>,
    /// Their size on disk.
    pub removed_bytes: u64,
    /// Data files no manifest names, kept as younger than the age given.
    pub kept_young: usize,
    /// The metadata (`<sha1>.json`) of manifests no longer there, removed.
    pub dangling: Vec<PathBuf>,
    /// Folders under `data/sha1` left empty, removed.
    pub folders: usize,
    /// Data files a manifest names that aren't there: the SHA-1 and a
    /// manifest that names it.
    pub missing: Vec<(String, String)>,
    /// What is wrong and left as it is: `latest` naming a manifest that
    /// isn't there, a manifest without its metadata.
    pub problems: Vec<String>,
}

fn is_sha1(name: &str) -> bool {
    name.len() == 40 && name.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Housekeeping of an NWSync repository, as neverwinter.nim's
/// `nwn_nwsync_prune` does it (`tests/nwsync.rs` holds the two to the same
/// files): every data file under `data/sha1` that no manifest in
/// `manifests/` names, and that is older than `min_age` seconds, is
/// removed, with the folders it leaves empty, and the metadata of a
/// manifest that is gone. Taking away old manifests (`manifests/<sha1>`
/// and its `.json`) is the builder's; their data goes with the next prune.
/// It says, and leaves, what it can't mend: `latest` naming a manifest
/// that isn't there, a manifest without its metadata, data a manifest
/// needs that is missing. A manifest that can't be read stops it before
/// anything is removed (the data it names would be taken for unused).
pub fn prune(root: &Path, min_age: u64, dry_run: bool) -> Result<Pruned, String> {
    let manifests_dir = root.join("manifests");
    let read_dir = |d: &Path| -> Result<Vec<PathBuf>, String> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(d)
            .map_err(|e| format!("{}: {e}", d.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        v.sort();
        Ok(v)
    };
    let file_name = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned());
    let mut out = Pruned::default();
    let entries = read_dir(&manifests_dir)?;
    let manifests: Vec<String> = entries
        .iter()
        .filter(|p| p.is_file())
        .filter_map(|p| file_name(p))
        .filter(|n| is_sha1(n))
        .collect();
    let has_manifest = |sha: &str| manifests.iter().any(|m| m.eq_ignore_ascii_case(sha));
    if let Ok(latest) = std::fs::read_to_string(root.join("latest")) {
        let latest = latest.trim();
        if !has_manifest(latest) {
            out.problems.push(format!(
                "`latest` names the manifest {latest}, which isn't in manifests/: nwserver \
                 serving `latest` fails"
            ));
        }
    }
    // Every manifest read before anything is removed.
    let mut named: BTreeMap<String, String> = BTreeMap::new();
    for m in &manifests {
        let path = manifests_dir.join(m);
        let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let entries = read_manifest(&data).map_err(|e| format!("manifest {m}: {e}"))?;
        for (_, sha, _) in entries {
            named.entry(hex(&sha)).or_insert_with(|| m.clone());
        }
        if !manifests_dir.join(format!("{m}.json")).is_file() {
            out.problems.push(format!(
                "the manifest {m} has no metadata ({m}.json): publish it again from its module"
            ));
        }
    }
    out.manifests = manifests.len();
    for p in entries.iter().filter(|p| p.is_file()) {
        let Some(n) = file_name(p) else { continue };
        if let Some(stem) = n.strip_suffix(".json")
            && is_sha1(stem)
            && !has_manifest(stem)
        {
            if !dry_run {
                std::fs::remove_file(p).map_err(|e| format!("{}: {e}", p.display()))?;
            }
            out.dangling.push(p.clone());
        }
    }
    // The data files, and the folders they leave empty (data/sha1 itself
    // stays).
    let now = std::time::SystemTime::now();
    let mut present: HashSet<String> = HashSet::new();
    fn walk(
        dir: &Path,
        top: bool,
        visit: &mut dyn FnMut(&Path) -> Result<bool, String>,
        folders: &mut usize,
        dry_run: bool,
    ) -> Result<bool, String> {
        let Ok(rd) = std::fs::read_dir(dir) else { return Ok(false) };
        let mut paths: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        let mut empty = true;
        for p in paths {
            let gone =
                if p.is_dir() { walk(&p, false, visit, folders, dry_run)? } else { visit(&p)? };
            empty &= gone;
        }
        if empty && !top {
            if !dry_run {
                std::fs::remove_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            *folders += 1;
        }
        Ok(empty)
    }
    let mut folders = 0;
    let (mut removed, mut removed_bytes, mut kept_young) = (Vec::new(), 0u64, 0usize);
    walk(
        &root.join("data").join("sha1"),
        true,
        &mut |p| {
            let name = file_name(p).unwrap_or_default().to_ascii_lowercase();
            if named.contains_key(&name) {
                present.insert(name);
                return Ok(false);
            }
            let meta = std::fs::metadata(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let age = meta
                .modified()
                .ok()
                .and_then(|t| now.duration_since(t).ok())
                .map_or(0, |d| d.as_secs());
            if age <= min_age {
                kept_young += 1;
                return Ok(false);
            }
            if !dry_run {
                std::fs::remove_file(p).map_err(|e| format!("{}: {e}", p.display()))?;
            }
            removed_bytes += meta.len();
            removed.push(p.to_path_buf());
            Ok(true)
        },
        &mut folders,
        dry_run,
    )?;
    out.removed = removed;
    out.removed_bytes = removed_bytes;
    out.kept_young = kept_young;
    out.folders = folders;
    out.missing = named.into_iter().filter(|(sha, _)| !present.contains(sha)).collect();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: &str, t: ResType) -> ResKey {
        ResKey::parse(n, t).unwrap()
    }

    #[test]
    fn manifests_as_neverwinter_nim_writes_them() {
        // nwn_nwsync_write's manifest for mg_a.2da ("2DA…"), mg_b.txt and
        // mg_c.txt ("same"): 9d1204bbe1fd9d46958fe45d9176fb8e20da1c03.
        let a = b"2DA V2.0\n\n  Label\n0 a\n";
        let entries = [
            (key("mg_a", ResType::TWODA), sha1(a), a.len() as u32),
            (key("mg_c", ResType::TXT), sha1(b"same"), 4),
            (key("mg_b", ResType::TXT), sha1(b"same"), 4),
        ];
        let m = manifest(&entries);
        assert_eq!(hex(&sha1(&m)), "9d1204bbe1fd9d46958fe45d9176fb8e20da1c03");
        let back = read_manifest(&m).unwrap();
        assert_eq!(back.len(), 3);
        assert!(read_manifest(&m[..m.len() - 3]).is_err());
    }

    #[test]
    fn a_repository_written_and_read() {
        let dir = std::env::temp_dir().join(format!("mg-nwsync-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let contents = vec![
            (key("one", ResType::TWODA), Source::Bytes(Arc::from(&b"first"[..]))),
            (key("one", ResType::TWODA), Source::Bytes(Arc::from(&b"shadowed"[..]))),
            (key("src", ResType::NSS), Source::Bytes(Arc::from(&b"void main() {}"[..]))),
            (key("two", ResType::TXT), Source::Bytes(Arc::from(&b"first"[..]))),
        ];
        let o = Options { latest: true, limit: Some(FILE_LIMIT), ..Default::default() };
        let w = write(&dir, &contents, &o, &mut |_, _| {}).unwrap();
        assert_eq!((w.files, w.new_files, w.bytes), (2, 1, 10));
        assert_eq!(std::fs::read_to_string(dir.join("latest")).unwrap(), w.sha1);
        let m = std::fs::read(dir.join("manifests").join(&w.sha1)).unwrap();
        let entries = read_manifest(&m).unwrap();
        let h = hex(&entries[0].1);
        let data =
            std::fs::read(dir.join("data/sha1").join(&h[..2]).join(&h[2..4]).join(&h)).unwrap();
        assert_eq!(mg_erf::compressedbuf::decompress(&data, MAGIC_NSYC).unwrap(), b"first");
        // Written again: nothing new.
        assert_eq!(write(&dir, &contents, &o, &mut |_, _| {}).unwrap().new_files, 0);
        let big = vec![(key("big", ResType::MDL), Source::Bytes(vec![0u8; 100].into()))];
        assert!(write(&dir, &big, &Options { limit: Some(10), ..o }, &mut |_, _| {}).is_err());
        assert_eq!(new_uuid().len(), 36);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Makes a file look written `days` ago.
    fn age_file(p: &Path, days: u64) {
        let then = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86_400);
        std::fs::File::options().write(true).open(p).unwrap().set_modified(then).unwrap();
    }

    fn data_path(root: &Path, data: &[u8]) -> PathBuf {
        let h = hex(&sha1(data));
        root.join("data/sha1").join(&h[0..2]).join(&h[2..4]).join(&h)
    }

    /// Canceled, nothing is written but the data files done by then; a
    /// prune takes away, once they are old enough, the data files no
    /// manifest names and the metadata of manifests taken away, and says
    /// what it can't mend.
    #[test]
    fn canceled_and_pruned() {
        let dir = std::env::temp_dir().join(format!("mg-nwsync-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let bytes = |b: &'static [u8]| Source::Bytes(Arc::from(b));
        let o = Options { latest: true, ..Default::default() };
        let first =
            vec![(key("a", ResType::TXT), bytes(b"one")), (key("b", ResType::TXT), bytes(b"two"))];
        let second = vec![
            (key("a", ResType::TXT), bytes(b"one")),
            (key("c", ResType::TXT), bytes(b"three")),
        ];
        // Called off after the first resource: no manifest, no latest.
        let calls = std::cell::Cell::new(0);
        let stop = || {
            calls.set(calls.get() + 1);
            calls.get() > 1
        };
        let e = write_until(&dir, &first, &o, &mut |_, _| {}, &stop).unwrap_err();
        assert_eq!(e, CANCELED);
        assert!(!dir.join("latest").exists() && !dir.join("manifests").exists());
        assert!(data_path(&dir, b"one").is_file());
        let w1 = write(&dir, &first, &o, &mut |_, _| {}).unwrap();
        let w2 = write(&dir, &second, &o, &mut |_, _| {}).unwrap();
        // Nothing unused yet.
        let p = prune(&dir, 0, false).unwrap();
        assert_eq!((p.manifests, p.removed.len(), p.missing.len()), (2, 0, 0), "{p:?}");
        assert!(p.problems.is_empty(), "{:?}", p.problems);
        // The first manifest taken away: "two" is no one's.
        std::fs::remove_file(dir.join("manifests").join(&w1.sha1)).unwrap();
        let two = data_path(&dir, b"two");
        let p = prune(&dir, PRUNE_MIN_AGE, false).unwrap();
        assert_eq!((p.removed.len(), p.kept_young, p.dangling.len()), (0, 1, 1), "{p:?}");
        assert!(two.is_file(), "too young to go");
        assert!(!dir.join("manifests").join(format!("{}.json", w1.sha1)).exists());
        age_file(&two, 15);
        let p = prune(&dir, PRUNE_MIN_AGE, true).unwrap();
        assert_eq!((p.removed.clone(), p.folders), (vec![two.clone()], 2), "a dry run says");
        assert!(two.is_file(), "and leaves it");
        let p = prune(&dir, PRUNE_MIN_AGE, false).unwrap();
        assert_eq!(p.removed, vec![two.clone()]);
        assert!(!two.exists() && !two.parent().unwrap().exists());
        assert!(data_path(&dir, b"one").is_file() && data_path(&dir, b"three").is_file());
        // What it can't mend: said, left.
        std::fs::remove_file(data_path(&dir, b"three")).unwrap();
        std::fs::remove_file(dir.join("manifests").join(format!("{}.json", w2.sha1))).unwrap();
        std::fs::write(dir.join("latest"), "0".repeat(40)).unwrap();
        let p = prune(&dir, 0, false).unwrap();
        assert_eq!(p.missing, vec![(hex(&sha1(b"three")), w2.sha1.clone())]);
        assert_eq!(p.problems.len(), 2, "{:?}", p.problems);
        assert!(dir.join("manifests").join(&w2.sha1).is_file());
        // A manifest that can't be read: nothing is removed.
        std::fs::write(dir.join("manifests").join("f".repeat(40)), b"NSYM").unwrap();
        age_file(&data_path(&dir, b"one"), 30);
        assert!(prune(&dir, 0, false).is_err());
        assert!(data_path(&dir, b"one").is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
