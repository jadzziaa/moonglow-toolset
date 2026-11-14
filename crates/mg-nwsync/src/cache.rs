//! Moonglow's own NWSync cache: what was downloaded that the game's
//! storage hasn't, in a folder of Moonglow's (never the game's), laid out
//! as a server's repository is:
//!
//! - `data/sha1/ab/cd/<sha1>`: a resource's data file as the server sent
//!   it (`NSYC`, compressed), kept once its SHA-1 was checked;
//! - `manifests/<sha1>`, with `.json` and `.origin` where the server had
//!   them;
//! - `latest/<server>`: the SHA-1 of the newest manifest a server named,
//!   when it was last asked (the manifest used when it can't be reached).
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

/// Writes `data` to `path` through a file beside it, so that a download
/// cut off leaves no half file under the name.
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
    std::fs::write(&part, data)?;
    match std::fs::rename(&part, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            // (Another writer got there first, with the same bytes.)
            if path.is_file() { Ok(()) } else { Err(e) }
        }
    }
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
    }
}
