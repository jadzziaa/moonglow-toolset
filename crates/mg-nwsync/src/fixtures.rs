//! A game's NWSync storage made up for tests (feature `fixtures`), to
//! the client's schema (`nwsyncmeta.sqlite3` and its shards, as
//! `nwmain` 89.8193.37 makes them): written in a test's scratch folder,
//! never in a user's.

use std::path::Path;

use mg_resman::ResKey;
use rusqlite::{Connection, params};

use crate::{Sha1, sha1_hex};

/// The metadata's tables, as the client makes them.
const META_SCHEMA: &str = "
create table manifests (sha1 text not null primary key, origin_url text, created_at int not null,
    last_used_at int not null, pinned bool not null default false);
create table manifest_resrefs (manifest_sha1 text not null, resref_sha1 text not null,
    resref text not null, restype int not null, created_at int not null);
create unique index manifest_resrefs_unique on manifest_resrefs (manifest_sha1, resref, restype);
create table manifest_cache (manifest_sha1 text not null primary key, data blob not null,
    size int not null, created_at int not null);
create table manifest_metadata (manifest_sha1 text not null primary key, module_name text,
    description text, includes_module_contents bool, includes_client_contents bool, uuid text,
    group_id int, created_at int);
create table shards (id int not null primary key, serial int not null);
create table meta (name text not null primary key, value);
create table migrations (migration_label text not null primary key, created_at int not null);
insert into meta values ('serial', 1);
";

/// A shard's tables.
const SHARD_SCHEMA: &str = "
create table resrefs (sha1 text not null primary key, data blob not null, size int not null,
    created_at int not null);
create index resrefs_created_at on resrefs (created_at);
create index resrefs_sha1 on resrefs (sha1);
create index resrefs_size on resrefs (size);
create table meta (name text not null primary key, value);
create table migrations (migration_label text not null primary key, created_at int not null);
insert into meta values ('serial', 1);
";

/// A manifest the made-up game has registered.
#[derive(Debug, Clone)]
pub struct Registered {
    pub sha1: Sha1,
    pub origin_url: String,
    pub created_at: i64,
    pub last_used_at: i64,
    /// Its resources by name, with their SHA-1s.
    pub resources: Vec<(ResKey, Sha1)>,
    /// The manifest's bytes, kept in `manifest_cache` where given.
    pub cached: Option<Vec<u8>>,
}

/// Writes a storage into `dir`: the manifests registered, and the shards
/// (`shards[0]` is `nwsyncdata_0.sqlite3`), each with its data as stored
/// (SHA-1, bytes).
pub fn write_storage(dir: &Path, manifests: &[Registered], shards: &[Vec<(Sha1, Vec<u8>)>]) {
    std::fs::create_dir_all(dir).unwrap();
    let meta = Connection::open(dir.join(crate::game::META)).unwrap();
    meta.execute_batch(META_SCHEMA).unwrap();
    for m in manifests {
        let hex = sha1_hex(&m.sha1);
        meta.execute(
            "insert into manifests values (?1, ?2, ?3, ?4, 0)",
            params![hex, m.origin_url, m.created_at, m.last_used_at],
        )
        .unwrap();
        for (key, sha) in &m.resources {
            meta.execute(
                "insert into manifest_resrefs values (?1, ?2, ?3, ?4, ?5)",
                params![
                    hex,
                    sha1_hex(sha),
                    key.resref.to_lowercase().to_string(),
                    key.restype.0,
                    m.created_at
                ],
            )
            .unwrap();
        }
        if let Some(bytes) = &m.cached {
            meta.execute(
                "insert into manifest_cache values (?1, ?2, ?3, ?4)",
                params![hex, bytes, bytes.len() as i64, m.created_at],
            )
            .unwrap();
        }
    }
    for (i, data) in shards.iter().enumerate() {
        meta.execute("insert into shards values (?1, 1)", params![i as i64 + 1]).unwrap();
        let shard = Connection::open(dir.join(format!("nwsyncdata_{i}.sqlite3"))).unwrap();
        shard.execute_batch(SHARD_SCHEMA).unwrap();
        for (sha, stored) in data {
            shard
                .execute(
                    "insert into resrefs values (?1, ?2, ?3, 0)",
                    params![sha1_hex(sha), stored, stored.len() as i64],
                )
                .unwrap();
        }
    }
}
