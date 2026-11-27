//! Which manifest a module's content is read from, found among what is
//! here (no network), and how much of its data is here.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::manifest::{self, Entry};
use crate::{Cache, GameStorage, Origins, Sha1, server_key, sha1_hex};

/// Which manifest of the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wanted {
    /// The newest the server names (`<url>/latest`), or the newest here
    /// when it can't be asked.
    Latest,
    /// This one alone.
    Pinned(Sha1),
}

/// Where a module's content comes from (`[nwsync]` in the project file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// The server's repository (`https://nwsync.example.org`).
    pub url: String,
    pub manifest: Wanted,
    /// Moonglow's cache folder, where not the default one.
    pub cache: Option<PathBuf>,
}

/// Where a manifest was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestFrom {
    /// Downloaded now.
    Server,
    /// Moonglow's cache.
    Cache,
    /// The game's NWSync storage.
    GameStorage,
}

impl ManifestFrom {
    /// As a sentence says it.
    pub fn said(self) -> &'static str {
        match self {
            ManifestFrom::Server => "downloaded from the server",
            ManifestFrom::Cache => "Moonglow's cache",
            ManifestFrom::GameStorage => "the game's NWSync storage",
        }
    }
}

/// A manifest found.
#[derive(Debug, Clone)]
pub struct Found {
    pub sha1: Sha1,
    pub entries: Vec<Entry>,
    pub from: ManifestFrom,
    /// Its origins file, where the server had one.
    pub origins: Option<Origins>,
    /// Whether the server built it with the module's own resources
    /// (`includes_module_contents`: `nwn_nwsync_write --with-module`), as
    /// its `.json` side file or the game's metadata says; `None` where
    /// neither says. Those are the server's copies of the module's
    /// resources: a layer or haks made from the manifest would put them
    /// over the module being edited ([`crate::haks::without_module`]).
    pub includes_module: Option<bool>,
    /// When the manifest came here, which a hak on disk is compared with
    /// (newer on disk: the builder's, over the manifest): the game's
    /// `created_at` for one from its storage, else when Moonglow's cache
    /// kept it; `None` where neither says.
    pub at: Option<SystemTime>,
}

impl Found {
    /// The first eight digits of its SHA-1, as messages name it.
    pub fn short(&self) -> String {
        sha1_hex(&self.sha1)[..8].to_string()
    }
}

/// Whether two addresses name the same server (`https://a/` is `HTTPS://A`).
fn same_server(a: &str, b: &str) -> bool {
    server_key(a) == server_key(b)
}

/// Whether a manifest holds the module's own resources, as its `.json`
/// side file in the cache says, else the game's metadata.
pub fn includes_module(
    sha1: &Sha1,
    cache: Option<&Cache>,
    game: Option<&GameStorage>,
) -> Option<bool> {
    cache
        .and_then(|c| c.includes_module_contents(sha1))
        .or_else(|| game.and_then(|g| g.includes_module_contents(sha1)))
}

/// When the game registered a manifest (its `created_at`), where it says.
fn registered(sha1: &Sha1, game: Option<&GameStorage>) -> Option<SystemTime> {
    let m = game?.manifests().ok()?.into_iter().find(|m| m.sha1 == *sha1)?;
    let secs = u64::try_from(m.created_at).ok().filter(|s| *s > 0)?;
    Some(UNIX_EPOCH + Duration::from_secs(secs))
}

/// When a manifest found in `from` came here ([`Found::at`]).
pub(crate) fn time_of(
    sha1: &Sha1,
    from: ManifestFrom,
    cache: Option<&Cache>,
    game: Option<&GameStorage>,
) -> Option<SystemTime> {
    let cached = || cache.and_then(|c| c.manifest_time(sha1));
    match from {
        ManifestFrom::GameStorage => registered(sha1, game).or_else(cached),
        ManifestFrom::Cache | ManifestFrom::Server => cached().or_else(|| registered(sha1, game)),
    }
}

/// A manifest of the cache or the game's storage by its SHA-1.
fn here(
    sha1: &Sha1,
    cache: Option<&Cache>,
    game: Option<&GameStorage>,
) -> Result<Option<Found>, String> {
    let origins = cache
        .and_then(|c| c.manifest(sha1, "origin"))
        .map(|b| Origins::parse(&String::from_utf8_lossy(&b)));
    if let Some(bytes) = cache.and_then(|c| c.manifest(sha1, "")) {
        // (One the cache can't read is downloaded again.)
        if mg_core::sha1::sha1(&bytes) == *sha1
            && let Ok(entries) = manifest::read(&bytes)
        {
            let includes_module = includes_module(sha1, cache, game);
            let from = ManifestFrom::Cache;
            let at = time_of(sha1, from, cache, game);
            return Ok(Some(Found { sha1: *sha1, entries, from, origins, includes_module, at }));
        }
    }
    if let Some(g) = game
        && let Some(entries) = g.manifest_entries(sha1)?
    {
        let includes_module = includes_module(sha1, cache, game);
        let from = ManifestFrom::GameStorage;
        let at = time_of(sha1, from, cache, game);
        return Ok(Some(Found { sha1: *sha1, entries, from, origins, includes_module, at }));
    }
    Ok(None)
}

/// The manifest `wanted` from what is here, without the network: a pinned
/// one from the cache or the game's storage; the latest, the newer of the
/// one the server last named to Moonglow and the one the game last
/// downloaded from it. An error that says what to do where there is none.
pub fn find(
    url: &str,
    wanted: Wanted,
    cache: Option<&Cache>,
    game: Option<&GameStorage>,
) -> Result<Found, String> {
    match wanted {
        Wanted::Pinned(sha) => here(&sha, cache, game)?.ok_or_else(|| {
            format!(
                "NWSync manifest {} is not here yet: download it (mg nwsync-pull, or Sync in \
                 Build › NWSync Content)",
                sha1_hex(&sha)
            )
        }),
        Wanted::Latest => {
            let named = cache.and_then(|c| c.latest(url)).map(|(sha, at)| {
                (sha, at.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64))
            });
            let games = match game {
                Some(g) => g.manifests()?,
                None => Vec::new(),
            };
            // (The one the game registered last from this server.)
            let played = games
                .iter()
                .filter(|m| same_server(&m.origin_url, url))
                .max_by_key(|m| (m.created_at, m.last_used_at));
            let mut order = Vec::new();
            match (named, played) {
                (Some((n, at)), Some(p)) if p.created_at > at => order.extend([p.sha1, n]),
                (Some((n, _)), Some(p)) => order.extend([n, p.sha1]),
                (Some((n, _)), None) => order.push(n),
                (None, Some(p)) => order.push(p.sha1),
                (None, None) => {}
            }
            for sha in order {
                if let Some(found) = here(&sha, cache, game)? {
                    return Ok(found);
                }
            }
            Err(format!(
                "no NWSync manifest of {url} is here yet: download it (mg nwsync-pull, or Sync in \
                 Build › NWSync Content)"
            ))
        }
    }
}

/// How much of a manifest's data is here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Survey {
    /// The manifest's resources (names), and its data files (distinct
    /// SHA-1s).
    pub resources: usize,
    pub data_files: usize,
    /// Data files in the game's storage, and of the rest in the cache.
    pub in_game: usize,
    pub in_cache: usize,
    /// The data files neither has, with their sizes (uncompressed; 0 where
    /// the manifest doesn't say).
    pub missing: Vec<(Sha1, u32)>,
    pub missing_bytes: u64,
}

impl Survey {
    /// Whether every data file is here.
    pub fn complete(&self) -> bool {
        self.missing.is_empty()
    }
}

/// Looks for each data file of `entries` in the game's storage, then the
/// cache.
pub fn survey(
    entries: &[Entry],
    game: Option<&GameStorage>,
    cache: Option<&Cache>,
) -> Result<Survey, String> {
    let mut files: HashMap<Sha1, u32> = HashMap::new();
    for (_, sha, size) in entries {
        let s = files.entry(*sha).or_insert(0);
        *s = (*s).max(*size);
    }
    let in_storage = match game {
        Some(g) => g.hashes()?,
        None => Default::default(),
    };
    let mut rest: Vec<(Sha1, u32)> =
        files.iter().filter(|(s, _)| !in_storage.contains(*s)).map(|(s, n)| (*s, *n)).collect();
    rest.sort();
    let in_game = files.len() - rest.len();
    // (A look at each file the cache may have, a few threads at a time:
    // two hundred thousand on a slow disk otherwise take a while.)
    let missing: Vec<(Sha1, u32)> = match cache {
        Some(c) => {
            let chunk = rest.len().div_ceil(8).max(1);
            std::thread::scope(|s| {
                let parts: Vec<_> = rest
                    .chunks(chunk)
                    .map(|part| {
                        s.spawn(move || {
                            part.iter().filter(|(sha, _)| !c.contains(sha)).copied().collect()
                        })
                    })
                    .collect();
                parts.into_iter().flat_map(|p| p.join().unwrap_or_else(|_| Vec::new())).collect()
            })
        }
        None => rest.clone(),
    };
    Ok(Survey {
        resources: entries.len(),
        data_files: files.len(),
        in_game,
        in_cache: rest.len() - missing.len(),
        missing_bytes: missing.iter().map(|(_, n)| u64::from(*n)).sum(),
        missing,
    })
}
