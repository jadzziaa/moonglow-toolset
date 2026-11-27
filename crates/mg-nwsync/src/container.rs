//! A manifest as a layer of the resource manager: its resources by name,
//! their bytes read from the game's NWSync storage, else from Moonglow's
//! cache.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mg_resman::{Container, ResError, ResKey};

use crate::{Cache, GameStorage, Sha1, sha1_hex, unpack};

/// The layer's label: `nwsync:` and the manifest's SHA-1.
pub fn label(manifest: &Sha1) -> String {
    format!("{LABEL_PREFIX}{}", sha1_hex(manifest))
}

/// What every NWSync layer's label starts with.
pub const LABEL_PREFIX: &str = "nwsync:";

/// A manifest's resources, read where they are.
///
/// It lists every resource of the manifest, whether its data is here or
/// not: the manifest's copy of a name is the one that wins, and a lower
/// layer's (the game's own `appearance.2da` under a hak's) is not taken
/// for it. Reading one whose data is neither in the game's storage nor in
/// the cache is an error that says so; once it is downloaded it reads.
pub struct NwSyncContainer {
    manifest: Sha1,
    entries: HashMap<ResKey, (Sha1, u32)>,
    game: Option<Arc<GameStorage>>,
    cache: Option<Arc<Cache>>,
}

impl std::fmt::Debug for NwSyncContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NwSyncContainer")
            .field("manifest", &sha1_hex(&self.manifest))
            .field("entries", &self.entries.len())
            .field("game", &self.game.as_ref().map(|g| &g.dir))
            .field("cache", &self.cache.as_ref().map(|c| c.root()))
            .finish()
    }
}

impl NwSyncContainer {
    /// The manifest's resources (the first of a name, where it has one
    /// twice), read from `game`, then `cache`.
    pub fn new(
        manifest: Sha1,
        entries: &[crate::manifest::Entry],
        game: Option<Arc<GameStorage>>,
        cache: Option<Arc<Cache>>,
    ) -> NwSyncContainer {
        let mut map = HashMap::with_capacity(entries.len());
        for (key, sha, size) in entries {
            map.entry(*key).or_insert((*sha, *size));
        }
        NwSyncContainer { manifest, entries: map, game, cache }
    }

    /// The manifest's SHA-1.
    pub fn manifest(&self) -> &Sha1 {
        &self.manifest
    }

    /// A resource's SHA-1 and size (0: not known).
    pub fn entry(&self, key: &ResKey) -> Option<(Sha1, u32)> {
        self.entries.get(key).copied()
    }

    fn where_(&self) -> PathBuf {
        self.cache
            .as_ref()
            .map(|c| c.root().to_path_buf())
            .or_else(|| self.game.as_ref().map(|g| g.dir.clone()))
            .unwrap_or_else(|| PathBuf::from("nwsync"))
    }
}

impl Container for NwSyncContainer {
    fn contains(&self, key: &ResKey) -> bool {
        self.entries.contains_key(key)
    }

    fn read(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        let (sha, size) = *self.entries.get(key).ok_or(ResError::NotFound(*key))?;
        let wrong = |message: String| ResError::Container { path: self.where_(), message };
        let mut stored = None;
        if let Some(g) = &self.game {
            stored = g.read(&sha).map_err(|e| wrong(format!("{key}: {e}")))?;
        }
        let mut cached = None;
        if stored.is_none()
            && let Some(c) = &self.cache
        {
            stored = c.read(&sha).map_err(|e| wrong(format!("{key}: {e}")))?;
            cached = stored.is_some().then_some(c);
        }
        let Some(stored) = stored else {
            return Err(wrong(format!(
                "{key} is in NWSync manifest {} but its data ({}) is not downloaded yet",
                &sha1_hex(&self.manifest)[..8],
                sha1_hex(&sha)
            )));
        };
        let data = unpack(&stored).map_err(|e| match cached {
            // (The cache's copy cut off: taken away, for the next pull.)
            Some(c) => {
                c.discard(&sha);
                wrong(format!(
                    "{key}: its data in the cache was damaged ({e}) and is removed: download it \
                     again"
                ))
            }
            None => wrong(format!("{key}: {e}")),
        })?;
        if size != 0 && data.len() != size as usize {
            return Err(wrong(format!(
                "{key}: its data is {} bytes, the manifest says {size}",
                data.len()
            )));
        }
        Ok(Cow::Owned(data))
    }

    fn keys(&self) -> Box<dyn Iterator<Item = ResKey> + '_> {
        Box::new(self.entries.keys().copied())
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}
