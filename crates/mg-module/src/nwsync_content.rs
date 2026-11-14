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
//!
//! What can't read NWSync (the game's server, the game on a module of its
//! own) gets those haks written from the manifest into Moonglow's cache
//! ([`write_haks`], [`mg_nwsync::haks`]).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use mg_nwsync::haks::{self, Index, PlanFrom, Store, WrittenHak};
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
        if let Some(c) = &cache {
            c.touch(&found.sha1);
        }
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

/// A module's haks that come from its NWSync content, written as hak
/// files ([`write_haks`]).
#[derive(Debug, Clone)]
pub struct HakFiles {
    pub manifest: Sha1,
    pub from: ManifestFrom,
    /// Whether the origins file said which resources each hak has (else
    /// they were spread over the haks).
    pub plan: PlanFrom,
    /// The haks, by the module's names for them, and their files in the
    /// cache.
    pub haks: Vec<WrittenHak>,
    /// The module's talk table, where the manifest has it: its file name
    /// (`world.tlk`) and its file in the cache.
    pub tlk: Option<(String, PathBuf)>,
    /// Haks the origins file names that the module doesn't list, not
    /// written (the game loads the module's haks alone).
    pub not_listed: Vec<String>,
    pub took: Duration,
}

impl HakFiles {
    /// The written file of the hak of this name (case ignored).
    pub fn hak(&self, name: &str) -> Option<&WrittenHak> {
        self.haks.iter().find(|h| h.name.eq_ignore_ascii_case(name))
    }

    /// What to say of them, in a line.
    pub fn summary(&self) -> String {
        let (now, bytes) = (self.haks.iter().filter(|h| h.written))
            .fold((0, 0u64), |(n, b), h| (n + 1, b + h.bytes));
        let how = match self.plan {
            PlanFrom::Origins => "as its origins file names them",
            PlanFrom::Spread => {
                "its resources spread over them (the server publishes no origins file)"
            }
        };
        let written = match now {
            0 => "all written before".to_string(),
            n => format!(
                "{n} written now ({:.1} GB in {:.0} s), the others written before",
                bytes as f64 / 1e9,
                self.took.as_secs_f64()
            ),
        };
        let tlk = match &self.tlk {
            Some((name, _)) => format!(", and the talk table {name}"),
            None => String::new(),
        };
        format!(
            "{} haks from NWSync manifest {}{tlk}, {how}: {written}",
            self.haks.len(),
            &mg_nwsync::sha1_hex(&self.manifest)[..8]
        )
    }
}

/// How haks are written.
pub struct WriteOptions<'a> {
    /// How many at once.
    pub jobs: usize,
    /// Every hak the origins file names, not only the module's.
    pub all: bool,
    pub progress: &'a (dyn Fn(haks::Progress) + Sync),
    pub stop: &'a (dyn Fn() -> bool + Sync),
}

impl std::fmt::Debug for WriteOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriteOptions").field("jobs", &self.jobs).field("all", &self.all).finish()
    }
}

impl Default for WriteOptions<'_> {
    fn default() -> Self {
        WriteOptions { jobs: 4, all: false, progress: &|_| {}, stop: &|| false }
    }
}

/// Writes the haks `names` (the module's `Mod_HakList`) that come from
/// the NWSync content of `settings` into Moonglow's cache, and the talk
/// table `tlk` where the manifest has it, from what is here (no network:
/// the manifest [`mg_nwsync::find`] picks, as [`mount`] does). With an
/// origins file, the haks it names, each with what it lists; without,
/// every hak, the manifest's resources spread over them. A hak written
/// before (for this manifest or another) is not written again. An error
/// where the manifest or its data isn't here, saying how to download it.
pub fn write_haks(
    settings: &Settings,
    game: Option<&GameStorage>,
    names: &[String],
    tlk: Option<&str>,
    o: &WriteOptions<'_>,
) -> Result<HakFiles, String> {
    let cache = cache_of(settings)
        .ok_or("there is no cache folder to write haks into: set [nwsync] cache")?;
    let found = mg_nwsync::find(&settings.url, settings.manifest, Some(&cache), game)?;
    let store = Store { game, cache: &cache };
    let mut entries = found.entries;
    if entries.iter().any(haks::size_unknown) {
        entries = haks::sizes(&entries, store)?;
    }
    let (plan, not_listed) = match &found.origins {
        Some(o_) => {
            let listed = |h: &str| names.iter().any(|n| n.eq_ignore_ascii_case(h));
            let others: Vec<String> =
                o_.haks().filter(|h| !listed(h)).map(str::to_string).collect();
            let mut wanted: Vec<String> = names.to_vec();
            let not_listed = if o.all {
                wanted.extend(others);
                Vec::new()
            } else {
                others
            };
            (haks::from_origins(o_, &entries, &wanted)?, not_listed)
        }
        None => (haks::spread(&entries, names)?, Vec::new()),
    };
    let written = haks::write_all(&plan, store, o.jobs, o.progress, o.stop)?;
    let mut files = Vec::new();
    let tlk = match tlk.map(str::trim).filter(|t| !t.is_empty()) {
        Some(name) => {
            let key = mg_resman::ResKey::parse(name, mg_core::ResType::TLK);
            let entry = key.and_then(|k| entries.iter().find(|(e, _, _)| *e == k).copied());
            match entry {
                Some((key, sha, size)) => {
                    let path = haks::write_file(&key, &sha, size, store)?;
                    let file = format!("{}.tlk", key.resref.to_lowercase());
                    files.push((file.clone(), sha));
                    Some((file, path))
                }
                None => None,
            }
        }
        None => None,
    };
    let index =
        Index { haks: (plan.haks.iter()).map(|h| (h.name.clone(), h.key())).collect(), files };
    let cache_dir = cache.root().display().to_string();
    cache.put_written(&found.sha1, index).map_err(|e| format!("{cache_dir}: {e}"))?;
    cache.touch(&found.sha1);
    Ok(HakFiles {
        manifest: found.sha1,
        from: found.from,
        plan: plan.from,
        haks: written.haks,
        tlk,
        not_listed,
        took: written.took,
    })
}

impl crate::Module {
    /// [`write_haks`] for the module's haks and talk table, where its
    /// project names an NWSync server (`None` where it names none).
    pub fn write_nwsync_haks(
        &self,
        install: &GameInstall,
        o: &WriteOptions<'_>,
    ) -> Option<Result<HakFiles, String>> {
        let settings = self.project_file.nwsync_settings()?;
        Some((|| {
            let game = game_storage(install)?;
            let names = self.haks().map_err(|e| e.to_string())?;
            let tlk = self.custom_tlk().map_err(|e| e.to_string())?;
            write_haks(&settings, game.as_ref(), &names, tlk.as_deref(), o)
        })())
    }
}
