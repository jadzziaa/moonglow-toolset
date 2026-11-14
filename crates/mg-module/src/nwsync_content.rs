//! A module's haks' content read from the NWSync server that hands it to
//! players (`[nwsync] source` in the project file), without the hak
//! files: the server's manifest as a layer of the game data, at NWSync's
//! priority (above the haks, below loose folders, as the game has it),
//! each resource read from the game's own NWSync storage (read only) or
//! from Moonglow's cache ([`mg_nwsync`]).
//!
//! Putting the layer in uses no network: the manifest is the newest here
//! (or the one pinned). Downloading the server's newest and the data
//! missing is asked for (`mg nwsync-pull`, Build › NWSync Content).
//!
//! The game loads no haks for a server with a manifest: the manifest is
//! the haks. So a hak of `Mod_HakList` that isn't on disk is not missing
//! where the manifest holds it: where the server publishes its origins
//! file (`manifests/<sha1>.origin`, which names each hak), the haks it
//! names; where it doesn't, every hak, as the game takes them.

use std::path::PathBuf;
use std::sync::Arc;

use mg_nwsync::{Cache, GameStorage, ManifestFrom, NwSyncContainer, Origins, Settings, Sha1};
use mg_resman::{GameInstall, LayerClass, ResMan, priority};

/// Moonglow's own NWSync cache folder: `nwsync` in its cache folder
/// ([`crate::build::cache_dir`]).
pub fn default_cache() -> Option<PathBuf> {
    crate::build::cache_dir().map(|d| d.join("nwsync"))
}

/// The cache `settings` use: theirs, else Moonglow's own.
pub fn cache_of(settings: &Settings) -> Option<Cache> {
    settings.cache.clone().or_else(default_cache).map(Cache::new)
}

/// The game's NWSync storage of the user folder `install` reads, where
/// it has one (open read only).
pub fn game_storage(install: &GameInstall) -> Result<Option<GameStorage>, String> {
    match &install.user_dir {
        Some(user) => GameStorage::open(&mg_nwsync::game::dir_in(user)),
        None => Ok(None),
    }
}

/// The layer put in.
#[derive(Debug, Clone)]
pub struct Layer {
    pub sha1: Sha1,
    pub from: ManifestFrom,
    /// The layer's label in the load order.
    pub label: String,
    /// Its resources.
    pub resources: usize,
    pub origins: Option<Origins>,
    /// The game's storage it reads, and the cache.
    pub game_storage: Option<PathBuf>,
    pub cache: Option<PathBuf>,
    /// The haks of the module not on disk that it holds: named by its
    /// origins file, or all of them where it has none (`by_origins`).
    pub haks: Vec<String>,
    pub by_origins: bool,
}

/// What became of a module's NWSync content when its game data was put
/// together.
#[derive(Debug, Clone)]
pub struct Mounted {
    pub settings: Settings,
    /// The layer, or why there is none.
    pub layer: Result<Layer, String>,
    /// What else to tell the builder (the game's storage unreadable).
    pub warnings: Vec<String>,
}

impl Mounted {
    /// What the builder is told (`mg` after `warning: `, the window's log).
    pub fn warnings(&self) -> Vec<String> {
        let mut out = self.warnings.clone();
        if let Err(e) = &self.layer {
            out.push(e.clone());
        }
        out
    }

    /// One line on what was put in, for the log: none where nothing was.
    pub fn summary(&self) -> Option<String> {
        let l = self.layer.as_ref().ok()?;
        let haks = match (l.haks.len(), l.by_origins) {
            (0, _) => String::new(),
            (1, true) => "; 1 hak taken from it, as its origins file names it".into(),
            (n, true) => format!("; {n} haks taken from it, as its origins file names them"),
            (1, false) => "; the hak not on disk taken to be in it".into(),
            (n, false) => format!("; the {n} haks not on disk taken to be in it"),
        };
        Some(format!(
            "NWSync content: manifest {} of {} ({} resources, from {}){haks}",
            &mg_nwsync::sha1_hex(&l.sha1)[..8],
            self.settings.url,
            l.resources,
            l.from.said(),
        ))
    }
}

/// Puts the module's NWSync layer into `resman` from what is here (no
/// network): the manifest [`mg_nwsync::find`] picks, read from the game's
/// storage and the cache. Takes out of `missing` (haks not found on disk)
/// those it holds.
pub fn mount(
    resman: &mut ResMan,
    install: &GameInstall,
    settings: &Settings,
    missing: &mut Vec<String>,
) -> Mounted {
    let mut warnings = Vec::new();
    let game = match game_storage(install) {
        Ok(g) => g,
        Err(e) => {
            warnings.push(format!("the game's NWSync storage can't be read: {e}"));
            None
        }
    };
    let cache = cache_of(settings);
    let layer = (|| {
        let found =
            mg_nwsync::find(&settings.url, settings.manifest, cache.as_ref(), game.as_ref())?;
        let label = mg_nwsync::label(&found.sha1);
        let game_dir = game.as_ref().map(|g| g.dir.clone());
        let cache_dir = cache.as_ref().map(|c| c.root().to_path_buf());
        let c = NwSyncContainer::new(
            found.sha1,
            &found.entries,
            game.map(Arc::new),
            cache.map(Arc::new),
        );
        let resources = mg_resman::Container::len(&c);
        // (One layer of NWSync content: a manifest put in before, gone.)
        let before: Vec<String> = (resman.layers().iter())
            .filter(|l| is_nwsync_layer(l))
            .map(|l| l.label.clone())
            .collect();
        for old in before {
            resman.remove(&old);
        }
        resman.add(priority::NWSYNC, label.clone(), LayerClass::NwSync, c);
        let (haks, kept): (Vec<String>, Vec<String>) = match &found.origins {
            Some(o) => missing.drain(..).partition(|h| o.hak(h).is_some()),
            None => (std::mem::take(missing), Vec::new()),
        };
        *missing = kept;
        Ok(Layer {
            sha1: found.sha1,
            from: found.from,
            label,
            resources,
            by_origins: found.origins.is_some(),
            origins: found.origins,
            game_storage: game_dir,
            cache: cache_dir,
            haks,
        })
    })();
    Mounted { settings: settings.clone(), layer, warnings }
}

/// Whether a layer of the load order is an NWSync manifest's.
pub fn is_nwsync_layer(layer: &mg_resman::Layer) -> bool {
    layer.class == LayerClass::NwSync && layer.label.starts_with(mg_nwsync::LABEL_PREFIX)
}
