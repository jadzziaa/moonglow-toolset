//! The game's own NWSync storage, read and never written: `nwsync/` in
//! the user's folder, where the client keeps what it downloaded to join
//! servers.
//!
//! `nwsyncmeta.sqlite3` lists the manifests the game registered
//! (`manifests`), each one's resources (`manifest_resrefs`: the manifest's
//! SHA-1, a resource's SHA-1, its name and type) and the data files
//! (`shards`: shard `N` is `nwsyncdata_<N-1>.sqlite3`). Each data file
//! holds resources by SHA-1 (`resrefs`: `sha1`, `data`), stored as the
//! client compressed them (`NSYC`) or raw. A SHA-1 is in one shard; every
//! shard is looked in.
//!
//! The game checks its shards' commit counters against the metadata at
//! start-up and marks every manifest to be downloaded again when they
//! differ, so nothing here writes: each file is opened read only, never
//! as unchanging (the game may be writing it), and each read is one short
//! statement, so that the game's own writes wait for it no more than a
//! moment (a rollback journal: a reader's lock holds a commit back).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use mg_core::ResType;
use mg_resman::ResKey;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::{Sha1, manifest, parse_sha1, sha1_hex};

/// The storage's metadata file.
pub const META: &str = "nwsyncmeta.sqlite3";

/// How long a read waits for the game to finish writing.
const BUSY_WAIT: Duration = Duration::from_secs(5);

/// A manifest the game has registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameManifest {
    pub sha1: Sha1,
    /// The server it came from (`https://nwsync.example.org`).
    pub origin_url: String,
    /// When the game registered it and last used it (Unix seconds; 0
    /// where not said).
    pub created_at: i64,
    pub last_used_at: i64,
}

/// The game's NWSync storage, open read only.
#[derive(Debug)]
pub struct GameStorage {
    pub dir: PathBuf,
    meta: Mutex<Connection>,
    shards: Vec<(PathBuf, Mutex<Connection>)>,
}

/// The storage folder in a user folder.
pub fn dir_in(user_dir: &Path) -> PathBuf {
    user_dir.join("nwsync")
}

/// Opens a database read only, with a wait for the game's writes.
fn open_read_only(path: &Path) -> Result<Connection, String> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let c = Connection::open_with_flags(path, flags).map_err(|e| said(path, &e))?;
    c.busy_timeout(BUSY_WAIT).map_err(|e| said(path, &e))?;
    Ok(c)
}

/// What a failed read says: with a journal the game left behind (it was
/// stopped while writing), what to do about it, which a reader that
/// writes nothing can't.
fn said(path: &Path, e: &rusqlite::Error) -> String {
    let mut journal = path.as_os_str().to_owned();
    journal.push("-journal");
    if Path::new(&journal).exists() {
        format!(
            "{}: {e} (the game left it half written: start the game once and leave it, then try \
             again)",
            path.display()
        )
    } else {
        format!("{}: {e}", path.display())
    }
}

/// An integer column that may hold text (a date written by another
/// tool): 0 where it isn't one.
fn int_of(v: ValueRef<'_>) -> i64 {
    match v {
        ValueRef::Integer(i) => i,
        ValueRef::Real(f) => f as i64,
        ValueRef::Text(t) => std::str::from_utf8(t).ok().and_then(|t| t.parse().ok()).unwrap_or(0),
        _ => 0,
    }
}

impl GameStorage {
    /// Opens the storage in `dir` (`<user folder>/nwsync`): `None` where
    /// the game has none (it never downloaded anything).
    pub fn open(dir: &Path) -> Result<Option<GameStorage>, String> {
        let meta_path = dir.join(META);
        if !meta_path.is_file() {
            return Ok(None);
        }
        let meta = open_read_only(&meta_path)?;
        // (The shards the metadata lists; where it can't be read, the files
        // there are.)
        let ids: Vec<i64> = match meta.prepare("select id from shards order by id") {
            Ok(mut s) => s
                .query_map([], |r| r.get::<_, i64>(0))
                .and_then(|rows| rows.collect::<Result<_, _>>())
                .map_err(|e| said(&meta_path, &e))?,
            Err(_) => (0..64)
                .filter(|n| dir.join(format!("nwsyncdata_{n}.sqlite3")).is_file())
                .map(|n| n + 1)
                .collect(),
        };
        let mut shards = Vec::new();
        for id in ids {
            let n = id.checked_sub(1).ok_or_else(|| {
                format!("{}: shard {id} is no data file's (the first is 1)", meta_path.display())
            })?;
            let path = dir.join(format!("nwsyncdata_{n}.sqlite3"));
            if path.is_file() {
                shards.push((path.clone(), Mutex::new(open_read_only(&path)?)));
            }
        }
        Ok(Some(GameStorage { dir: dir.to_path_buf(), meta: Mutex::new(meta), shards }))
    }

    /// How many data files (shards) it has.
    pub fn shards(&self) -> usize {
        self.shards.len()
    }

    /// The manifests the game has registered, the last used first.
    pub fn manifests(&self) -> Result<Vec<GameManifest>, String> {
        let c = self.meta.lock().unwrap_or_else(|e| e.into_inner());
        let path = self.dir.join(META);
        let mut s = c
            .prepare("select sha1, origin_url, created_at, last_used_at from manifests")
            .map_err(|e| said(&path, &e))?;
        let rows = s
            .query_map([], |r| {
                let sha: String = r.get(0)?;
                let url = match r.get_ref(1)? {
                    ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                    _ => String::new(),
                };
                Ok((sha, url, int_of(r.get_ref(2)?), int_of(r.get_ref(3)?)))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(|e| said(&path, &e))?;
        let mut out: Vec<GameManifest> = rows
            .into_iter()
            .filter_map(|(sha, origin_url, created_at, last_used_at)| {
                Some(GameManifest { sha1: parse_sha1(&sha)?, origin_url, created_at, last_used_at })
            })
            .collect();
        out.sort_by_key(|m| std::cmp::Reverse((m.last_used_at, m.created_at)));
        Ok(out)
    }

    /// A manifest's resources as the game keeps them: from the manifest's
    /// own bytes where it kept them (`manifest_cache`), else from its list
    /// (`manifest_resrefs`, which has no sizes: 0). `None`: the game has
    /// no such manifest. Names that are no resource's are left out.
    pub fn manifest_entries(&self, sha1: &Sha1) -> Result<Option<Vec<manifest::Entry>>, String> {
        let c = self.meta.lock().unwrap_or_else(|e| e.into_inner());
        let path = self.dir.join(META);
        let hex = sha1_hex(sha1);
        let cached: Option<Vec<u8>> = c
            .query_row("select data from manifest_cache where manifest_sha1 = ?1", [&hex], |r| {
                r.get(0)
            })
            .optional()
            .unwrap_or(None);
        if let Some(entries) = cached.as_deref().and_then(|b| manifest::read(b).ok()) {
            return Ok(Some(entries));
        }
        let mut s = c
            .prepare(
                "select resref_sha1, resref, restype from manifest_resrefs where manifest_sha1 = \
                 ?1",
            )
            .map_err(|e| said(&path, &e))?;
        let rows = s
            .query_map([&hex], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(|e| said(&path, &e))?;
        if rows.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            rows.into_iter()
                .filter_map(|(sha, name, restype)| {
                    let restype = ResType(u16::try_from(restype).ok()?);
                    Some((ResKey::parse(&name, restype)?, parse_sha1(&sha)?, 0))
                })
                .collect(),
        ))
    }

    /// Whether the server built a manifest with the module's own resources,
    /// as the game's metadata says (`manifest_metadata`'s
    /// `includes_module_contents`): `None` where it doesn't say (no such
    /// table, row or value).
    pub fn includes_module_contents(&self, sha1: &Sha1) -> Option<bool> {
        let c = self.meta.lock().unwrap_or_else(|e| e.into_inner());
        c.query_row(
            "select includes_module_contents from manifest_metadata where manifest_sha1 = ?1",
            [sha1_hex(sha1)],
            |r| {
                Ok(match r.get_ref(0)? {
                    ValueRef::Integer(i) => Some(i != 0),
                    ValueRef::Real(f) => Some(f != 0.0),
                    ValueRef::Text(t) => match t {
                        b"1" | b"true" | b"TRUE" => Some(true),
                        b"0" | b"false" | b"FALSE" => Some(false),
                        _ => None,
                    },
                    _ => None,
                })
            },
        )
        .ok()
        .flatten()
    }

    /// Every SHA-1 the shards hold, read by their first two digits, 256
    /// reads of a shard (each read short: a reader's lock holds the game's
    /// commit back while it lasts, and a persistent world's storage has
    /// some 350,000).
    pub fn hashes(&self) -> Result<HashSet<Sha1>, String> {
        // ("01" to "ff": what sorts below the first, between two, and from
        // the last on, so every text is in one range, uppercase too.)
        let bounds: Vec<String> = (1..=255u8).map(|b| format!("{b:02x}")).collect();
        let last = bounds.len() - 1;
        let mut out = HashSet::new();
        for (path, c) in &self.shards {
            let c = c.lock().unwrap_or_else(|e| e.into_inner());
            let mut s = c
                .prepare("select sha1 from resrefs where sha1 >= ?1 and sha1 < ?2")
                .map_err(|e| said(path, &e))?;
            let mut first = c
                .prepare("select sha1 from resrefs where sha1 < ?1")
                .map_err(|e| said(path, &e))?;
            let mut from_last = c
                .prepare("select sha1 from resrefs where sha1 >= ?1")
                .map_err(|e| said(path, &e))?;
            let mut take = |rows: rusqlite::Rows<'_>| -> Result<(), rusqlite::Error> {
                for sha in rows.mapped(|r| r.get::<_, String>(0)) {
                    out.extend(parse_sha1(&sha?));
                }
                Ok(())
            };
            let mut each = || -> Result<(), rusqlite::Error> {
                take(first.query([&bounds[0]])?)?;
                for w in bounds.windows(2) {
                    take(s.query([&w[0], &w[1]])?)?;
                }
                take(from_last.query([&bounds[last]])?)
            };
            each().map_err(|e| said(path, &e))?;
        }
        Ok(out)
    }

    /// Whether a shard holds the resource of this SHA-1.
    pub fn contains(&self, sha1: &Sha1) -> bool {
        let hex = sha1_hex(sha1);
        self.shards.iter().any(|(_, c)| {
            let c = c.lock().unwrap_or_else(|e| e.into_inner());
            c.query_row("select 1 from resrefs where sha1 = ?1", [&hex], |_| Ok(()))
                .optional()
                .ok()
                .flatten()
                .is_some()
        })
    }

    /// The resource of this SHA-1 as stored (compressed, `NSYC`, or raw):
    /// `None` where no shard holds it.
    pub fn read(&self, sha1: &Sha1) -> Result<Option<Vec<u8>>, String> {
        let hex = sha1_hex(sha1);
        for (path, c) in &self.shards {
            let c = c.lock().unwrap_or_else(|e| e.into_inner());
            let found = c
                .query_row("select data from resrefs where sha1 = ?1", [&hex], |r| r.get(0))
                .optional()
                .map_err(|e| said(path, &e))?;
            if found.is_some() {
                return Ok(found);
            }
        }
        Ok(None)
    }
}
