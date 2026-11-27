//! Moonglow's own NWSync cache: what was downloaded that the game's
//! storage hasn't, in a folder of Moonglow's (never the game's), laid out
//! as a server's repository is:
//!
//! - `data/sha1/ab/cd/<sha1>`: a resource's data file as the server sent
//!   it (`NSYC`, compressed), kept once its SHA-1 was checked;
//! - `manifests/<sha1>`, with `.json` and `.origin` where the server had
//!   them;
//! - `latest/<server>`: the SHA-1 of the newest manifest a server named,
//!   when it was last asked (the manifest used when it can't be reached);
//! - `haks/<key>.hak`: a hak written from a manifest's data, named by its
//!   content ([`crate::haks`]), and `haks/<sha1>.tlk` a talk table;
//! - `written/<sha1>`: which of those are a manifest's haks
//!   ([`crate::haks::Index`]);
//! - `used/<sha1>`: an empty file, its time when the manifest was last
//!   used (one read from the game's storage alone has no other file here).
//!
//! A manifest's `used/` mark and its index of written haks are touched
//! when it is used ([`Cache::touch`]), so that what was used last is kept
//! when the cache is pruned ([`crate::prune`]). Its own file is not: its
//! time stays when it came here ([`Cache::manifest_time`]).
//!
//! Content is named by its SHA-1, so one cache serves every server.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::{Sha1, parse_sha1, sha1_hex};

/// A cache folder.
#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

/// Moves a part file written whole into place. Where the move fails and
/// the file is there all the same (another writer got there first, with
/// the same bytes, or on Windows a server has it open), that is done: the
/// part goes.
pub(crate) fn into_place(part: &Path, path: &Path) -> std::io::Result<()> {
    match std::fs::rename(part, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(part);
            if path.is_file() { Ok(()) } else { Err(e) }
        }
    }
}

/// Writes `data` to `path` through a file beside it, on the disk before
/// it is moved into place, so that a download cut off or a crash leaves
/// no half file under the name.
fn write_whole(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // (A number of its own for each write: two threads of one process
    // may write the same file.)
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let part = path.with_file_name(format!(".{name}.{}-{n}.part", std::process::id()));
    let written = (|| {
        use std::io::Write as _;
        let mut f = std::fs::File::create(&part)?;
        f.write_all(data)?;
        f.sync_all()
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    into_place(&part, path)
}

/// A server's address as a file name: `https://nwsync.example.org/` is
/// `nwsync.example.org`, a port or path kept with `_` for what a file
/// name can't hold.
pub fn server_key(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    let bare = url.split_once("://").map_or(url, |(_, rest)| rest);
    bare.chars()
        .map(|c| {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }
        })
        .collect()
}

impl Cache {
    pub fn new(root: impl Into<PathBuf>) -> Cache {
        Cache { root: root.into() }
    }

    /// The folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where a resource's data file is (whether or not it is there).
    pub fn data_path(&self, sha1: &Sha1) -> PathBuf {
        let h = sha1_hex(sha1);
        self.root.join("data/sha1").join(&h[0..2]).join(&h[2..4]).join(&h)
    }

    /// Whether the cache has a resource's data file.
    pub fn contains(&self, sha1: &Sha1) -> bool {
        self.data_path(sha1).is_file()
    }

    /// A resource's data file (`None`: not here).
    pub fn read(&self, sha1: &Sha1) -> std::io::Result<Option<Vec<u8>>> {
        match std::fs::read(self.data_path(sha1)) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Takes a resource's data file away (one that doesn't unpack to what
    /// it is named: the next pull downloads it again).
    pub fn discard(&self, sha1: &Sha1) {
        let _ = std::fs::remove_file(self.data_path(sha1));
    }

    /// Keeps a resource's data file, as downloaded (its SHA-1 checked by
    /// the caller).
    pub fn put(&self, sha1: &Sha1, stored: &[u8]) -> std::io::Result<()> {
        write_whole(&self.data_path(sha1), stored)
    }

    fn manifest_path(&self, sha1: &Sha1, ext: &str) -> PathBuf {
        let name = sha1_hex(sha1);
        let name = if ext.is_empty() { name } else { format!("{name}.{ext}") };
        self.root.join("manifests").join(name)
    }

    /// A manifest's bytes (`ext` empty), or its `json` or `origin` file.
    pub fn manifest(&self, sha1: &Sha1, ext: &str) -> Option<Vec<u8>> {
        std::fs::read(self.manifest_path(sha1, ext)).ok()
    }

    /// Keeps a manifest's bytes (`ext` empty), or its `json` or `origin`
    /// file.
    pub fn put_manifest(&self, sha1: &Sha1, ext: &str, data: &[u8]) -> std::io::Result<()> {
        write_whole(&self.manifest_path(sha1, ext), data)
    }

    /// When a manifest came into the cache (its file's time: a use doesn't
    /// change it, [`Cache::touch`]), where it is here.
    pub fn manifest_time(&self, sha1: &Sha1) -> Option<SystemTime> {
        std::fs::metadata(self.manifest_path(sha1, "")).ok()?.modified().ok()
    }

    /// Whether a manifest holds the module's own resources, as its `.json`
    /// side file says (`includes_module_contents`): `None` where there is
    /// none, or it doesn't say.
    pub fn includes_module_contents(&self, sha1: &Sha1) -> Option<bool> {
        let json = self.manifest(sha1, "json")?;
        let v: serde_json::Value = serde_json::from_slice(&json).ok()?;
        v.get("includes_module_contents")?.as_bool()
    }

    /// The manifests kept, the newest first (by when they were kept).
    pub fn manifests(&self) -> Vec<(Sha1, SystemTime)> {
        let Ok(dir) = std::fs::read_dir(self.root.join("manifests")) else { return Vec::new() };
        let mut out: Vec<(Sha1, SystemTime)> = dir
            .flatten()
            .filter_map(|e| {
                let sha = parse_sha1(e.file_name().to_str()?)?;
                Some((sha, e.metadata().ok()?.modified().ok()?))
            })
            .collect();
        out.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
        out
    }

    /// Where the hak of this key is ([`crate::haks::Planned::key`]).
    pub fn hak_path(&self, key: &Sha1) -> PathBuf {
        self.root.join("haks").join(format!("{}.hak", sha1_hex(key)))
    }

    /// Where a resource written as a file of its own is (a talk table).
    pub fn file_path(&self, sha1: &Sha1, ext: &str) -> PathBuf {
        self.root.join("haks").join(format!("{}.{ext}", sha1_hex(sha1)))
    }

    /// Where a manifest's index of written haks is.
    pub fn written_path(&self, manifest: &Sha1) -> PathBuf {
        self.root.join("written").join(sha1_hex(manifest))
    }

    /// A manifest's written haks, as last kept (empty: none).
    pub fn written(&self, manifest: &Sha1) -> crate::haks::Index {
        std::fs::read_to_string(self.written_path(manifest))
            .map(|t| crate::haks::Index::parse(&t))
            .unwrap_or_default()
    }

    /// Keeps a manifest's written haks, with those kept before for it.
    pub fn put_written(
        &self,
        manifest: &Sha1,
        index: crate::haks::Index,
    ) -> std::io::Result<crate::haks::Index> {
        let index = index.merged(&self.written(manifest));
        write_whole(&self.written_path(manifest), index.to_text().as_bytes())?;
        Ok(index)
    }

    /// Where the mark of a manifest's last use is.
    pub fn used_path(&self, manifest: &Sha1) -> PathBuf {
        self.root.join("used").join(sha1_hex(manifest))
    }

    /// Marks a manifest used now (its mark in `used/`, and its index of
    /// written haks where it has one): a prune keeps the last used, the
    /// game's storage's too. A cache folder that isn't there is not made
    /// for it. The manifest's own file keeps its time: when it came here,
    /// which a hak on disk is compared with ([`crate::Found::at`]).
    pub fn touch(&self, manifest: &Sha1) {
        let now = SystemTime::now();
        if self.root.is_dir() {
            let used = self.used_path(manifest);
            if let Some(dir) = used.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let open =
                std::fs::File::options().write(true).create(true).truncate(false).open(&used);
            if let Ok(f) = open {
                let _ = f.set_modified(now);
            }
        }
        if let Ok(f) = std::fs::File::options().write(true).open(self.written_path(manifest)) {
            let _ = f.set_modified(now);
        }
    }

    fn latest_path(&self, url: &str) -> PathBuf {
        self.root.join("latest").join(server_key(url))
    }

    /// The newest manifest the server named when last asked, and when.
    pub fn latest(&self, url: &str) -> Option<(Sha1, SystemTime)> {
        let path = self.latest_path(url);
        let sha = parse_sha1(std::fs::read_to_string(&path).ok()?.trim())?;
        Some((sha, std::fs::metadata(&path).ok()?.modified().ok()?))
    }

    /// Keeps the newest manifest the server named.
    pub fn set_latest(&self, url: &str, sha1: &Sha1) -> std::io::Result<()> {
        write_whole(&self.latest_path(url), sha1_hex(sha1).as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn servers_are_file_names() {
        assert_eq!(server_key("https://nwsync.example.com"), "nwsync.example.com");
        assert_eq!(server_key("https://NWSync.Example.com/"), "nwsync.example.com");
        assert_eq!(server_key("http://host:4443/nw/sync"), "host_4443_nw_sync");
        assert_eq!(server_key("host"), "host");
    }

    #[test]
    fn data_manifests_and_latest_are_kept() {
        let c = Cache::new(mg_testkit::scratch_dir("nwsync-cache-kept"));
        let sha = mg_core::sha1::sha1(b"x");
        assert!(!c.contains(&sha));
        assert_eq!(c.read(&sha).unwrap(), None);
        c.put(&sha, b"stored").unwrap();
        assert!(c.contains(&sha));
        assert_eq!(c.read(&sha).unwrap().as_deref(), Some(&b"stored"[..]));
        let h = sha1_hex(&sha);
        assert!(c.data_path(&sha).ends_with(format!("data/sha1/{}/{}/{h}", &h[0..2], &h[2..4])));
        // (No part file left beside it.)
        let dir = c.data_path(&sha).parent().unwrap().to_path_buf();
        assert_eq!(std::fs::read_dir(dir).unwrap().count(), 1);
        c.put_manifest(&sha, "", b"NSYM").unwrap();
        c.put_manifest(&sha, "origin", b"Erf:a.hak\n").unwrap();
        assert_eq!(c.manifest(&sha, "").as_deref(), Some(&b"NSYM"[..]));
        assert_eq!(c.manifest(&sha, "json"), None);
        // (Only manifests are listed, not their side files.)
        assert_eq!(c.manifests().len(), 1);
        assert_eq!(c.latest("https://a.example"), None);
        c.set_latest("https://a.example/", &sha).unwrap();
        assert_eq!(c.latest("https://a.example").map(|l| l.0), Some(sha));
        assert_eq!(c.latest("https://b.example"), None);
        c.discard(&sha);
        assert!(!c.contains(&sha));
    }

    /// A move that fails is done where the file is there all the same,
    /// and the part goes; else it is the error.
    #[test]
    fn a_failed_move_onto_a_file_is_done() {
        let dir = mg_testkit::scratch_dir("nwsync-cache-into-place");
        let (part, path) = (dir.join(".x.part"), dir.join("x"));
        assert!(into_place(&part, &path).is_err());
        std::fs::write(&path, b"there").unwrap();
        into_place(&part, &path).unwrap();
        std::fs::write(&part, b"part").unwrap();
        // (A folder where the file goes: the move fails, and it is no file.)
        let folder = dir.join("folder");
        std::fs::create_dir_all(folder.join("inside")).unwrap();
        assert!(into_place(&part, &folder).is_err());
        assert!(!part.exists());
        assert_eq!(std::fs::read(&path).unwrap(), b"there");
    }
}
