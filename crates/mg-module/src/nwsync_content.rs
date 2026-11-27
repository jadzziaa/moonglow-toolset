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
//!
//! **A builder's own files newer than the manifest win** (Moonglow's
//! rule, which every caller shares: the views, `mg serve`, `mg test`,
//! `mg nwsync-haks` and Test Module). A hak the module lists that is on
//! disk, whose file is newer than the manifest came here
//! ([`mg_nwsync::Found::at`]), is read over the manifest's layer
//! ([`priority::HAK_OVER_NWSYNC`]) and taken from disk, not written from
//! the manifest; so is the custom talk table (the project's own source or
//! the `tlk` folder's file), its feminine table with it. Each such case
//! is said. An older one stays under the manifest, as in the game. Where
//! the server publishes no origins file, the manifest's resources are
//! spread over the listed haks that are not on disk only: a hak on disk
//! is the builder's file (perhaps never published), never replaced by a
//! guessed slice of the manifest.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use mg_nwsync::haks::{self, Index, PlanFrom, Store, WrittenHak};
use mg_nwsync::{Cache, GameStorage, ManifestFrom, NwSyncContainer, Origins, Settings, Sha1};
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};

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

/// What of a module's content is on disk, which the precedence rule
/// weighs against its manifest ([`OnDisk::of`], [`crate::Module::nwsync_on_disk`]).
#[derive(Debug, Clone, Default)]
pub struct OnDisk {
    /// The haks the module lists that are on disk (found as the game finds
    /// them), by the module's names, highest first, and their files.
    pub haks: Vec<(String, PathBuf)>,
    /// The custom talk table's name (`Mod_CustomTlk`), where it names one.
    pub tlk_name: Option<String>,
    /// Its files on disk and its feminine table's: the project's own
    /// sources, else the `tlk` folders' files.
    pub tlk: Vec<PathBuf>,
    /// The module's own resources, left out of a manifest built with them
    /// ([`mg_nwsync::Found::includes_module`]).
    pub module: Option<HashSet<ResKey>>,
}

impl OnDisk {
    /// What is on disk for haks `haks` and talk table `tlk`: the haks
    /// where `install` finds them; the table's files `own` (the project's
    /// sources of it and of its feminine table), else those of the `tlk`
    /// folders; `module`, the module's own resources.
    pub fn of(
        install: &GameInstall,
        haks: &[String],
        tlk: Option<&str>,
        own: Vec<PathBuf>,
        module: Option<HashSet<ResKey>>,
    ) -> OnDisk {
        let haks = (haks.iter()).filter_map(|h| Some((h.clone(), install.find_hak(h)?))).collect();
        let tlk_name = tlk.map(str::trim).filter(|t| !t.is_empty()).map(str::to_string);
        let tlk = match (&tlk_name, own.is_empty()) {
            (Some(name), true) => [name.clone(), crate::talk::feminine(name)]
                .iter()
                .filter_map(|n| mg_resman::named_file(n, "tlk"))
                .filter_map(|file| {
                    install.tlk_dirs().iter().map(|d| d.join(&file)).find(|p| p.is_file())
                })
                .collect(),
            (Some(_), false) => own,
            (None, _) => Vec::new(),
        };
        OnDisk { haks, tlk_name, tlk, module }
    }

    /// The talk table on disk is newer than a manifest of time `at`.
    pub fn tlk_newer(&self, at: Option<SystemTime>) -> bool {
        self.tlk_name.is_some() && self.tlk.iter().any(|p| newer_than(p, at))
    }

    /// The haks on disk newer than a manifest of time `at`.
    pub fn newer_haks(&self, at: Option<SystemTime>) -> Vec<String> {
        (self.haks.iter()).filter(|(_, p)| newer_than(p, at)).map(|(h, _)| h.clone()).collect()
    }

    /// What is on disk but for the files of Moonglow's NWSync cache
    /// `cache` (a link to one there that `mg serve --real-user-dir` or a
    /// builder put in a hak folder is the manifest's, not the builder's).
    pub fn without_cache(&self, cache: Option<&Path>) -> OnDisk {
        let Some(root) = cache.and_then(|c| std::fs::canonicalize(c).ok()) else {
            return self.clone();
        };
        let ours = |p: &Path| std::fs::canonicalize(p).is_ok_and(|p| p.starts_with(&root));
        OnDisk {
            haks: self.haks.iter().filter(|(_, p)| !ours(p)).cloned().collect(),
            tlk: self.tlk.iter().filter(|p| !ours(p)).cloned().collect(),
            ..self.clone()
        }
    }

    fn is_on_disk(&self, hak: &str) -> bool {
        self.haks.iter().any(|(h, _)| h.eq_ignore_ascii_case(hak))
    }
}

/// Whether a file on disk is newer than a manifest that came here at
/// `at` (the rule: then the file wins). Not where either time is unknown.
pub fn newer_than(path: &Path, at: Option<SystemTime>) -> bool {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    matches!((modified, at), (Some(m), Some(at)) if m > at)
}

/// What is said where a file on disk wins (`what`: "hak x", "talk table
/// x").
pub fn newer_note(what: &str) -> String {
    format!("{what} on disk is newer than the NWSync manifest: used over it")
}

/// The manifest's entries but for the module's own resources where it
/// was built with them (`includes_module`; not said: as without).
fn entries_for(found: &mg_nwsync::Found, on_disk: &OnDisk) -> Vec<mg_nwsync::manifest::Entry> {
    match (found.includes_module, &on_disk.module) {
        (Some(true), Some(module)) => haks::without_module(&found.entries, module),
        _ => found.entries.clone(),
    }
}

/// Whether a manifest's entry is the custom talk table `name` or its
/// feminine table.
fn is_table(key: &ResKey, name: &str) -> bool {
    key.restype == mg_core::ResType::TLK
        && [name.to_string(), crate::talk::feminine(name)]
            .iter()
            .any(|n| key.resref.to_string().eq_ignore_ascii_case(n))
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
    /// The haks on disk newer than the manifest, read over it.
    pub newer_haks: Vec<String>,
    /// The custom talk table on disk is newer than the manifest: the
    /// manifest's (and its feminine table) left out of the layer.
    pub newer_tlk: Option<String>,
}

impl Layer {
    /// What is said of the files on disk newer than the manifest.
    pub fn notes(&self) -> Vec<String> {
        let haks = self.newer_haks.iter().map(|h| newer_note(&format!("hak {h}")));
        let tlk = self.newer_tlk.iter().map(|t| newer_note(&format!("talk table {t}")));
        haks.chain(tlk).collect()
    }
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
        match &self.layer {
            Ok(l) => out.extend(l.notes()),
            Err(e) => out.push(e.clone()),
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
/// those it holds. The rule (see the module's notes) with `on_disk`: its
/// haks newer than the manifest (their layers `hak:<name>`, put in
/// before) moved over it, a newer talk table left out of it, and the
/// module's own resources where the manifest was built with them.
pub fn mount(
    resman: &mut ResMan,
    install: &GameInstall,
    settings: &Settings,
    on_disk: &OnDisk,
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
        let on_disk = &on_disk.without_cache(cache.as_ref().map(|c| c.root()));
        let mut entries = entries_for(&found, on_disk);
        let newer_tlk = match &on_disk.tlk_name {
            Some(name) if on_disk.tlk_newer(found.at) => {
                entries.retain(|(k, _, _)| !is_table(k, name));
                Some(name.clone())
            }
            _ => None,
        };
        let c = NwSyncContainer::new(found.sha1, &entries, game.map(Arc::new), cache.map(Arc::new));
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
        let newer_haks = on_disk.newer_haks(found.at);
        place_haks(resman, install, on_disk, &newer_haks);
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
            newer_haks,
            newer_tlk,
        })
    })();
    Mounted { settings: settings.clone(), layer, warnings }
}

/// Gives the layers of the module's haks on disk (`hak:<name>`) their
/// places by the rule: those `newer` than the manifest over it, in the
/// module's order; the others under it, where the game has them (where
/// one was moved over an earlier manifest, all of them again, in order).
fn place_haks(resman: &mut ResMan, install: &GameInstall, on_disk: &OnDisk, newer: &[String]) {
    let label = |h: &str| format!("hak:{h}");
    let is_newer = |h: &str| newer.iter().any(|n| n == h);
    let over = |rm: &ResMan, h: &str| {
        rm.layers().iter().any(|l| l.label == label(h) && l.priority == priority::HAK_OVER_NWSYNC)
    };
    let back = on_disk.haks.iter().any(|(h, _)| !is_newer(h) && over(resman, h));
    let stock = install.root.join("data").join("hk");
    for (hak, path) in &on_disk.haks {
        if is_newer(hak) {
            resman.move_to(&label(hak), priority::HAK_OVER_NWSYNC);
        } else if back {
            let p = if path.parent() == Some(stock.as_path()) {
                priority::HAK
            } else {
                priority::HAK_USER
            };
            resman.move_to(&label(hak), p);
        }
    }
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
    /// The module's talk table, where the manifest has it (and the one on
    /// disk isn't newer): its file name (`world.tlk`) and its file in the
    /// cache.
    pub tlk: Option<(String, PathBuf)>,
    /// Its feminine table (`worldf.tlk`), where the manifest has it too:
    /// it comes from where the table comes from.
    pub tlk_f: Option<(String, PathBuf)>,
    /// The listed haks on disk newer than the manifest, taken from disk
    /// (not written).
    pub newer_haks: Vec<String>,
    /// The talk table on disk is newer than the manifest: taken from disk.
    pub newer_tlk: Option<String>,
    /// Without an origins file, the listed haks on disk (not newer than
    /// the manifest) taken from disk as they are, the manifest's resources
    /// spread over the others.
    pub kept_on_disk: Vec<String>,
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

    /// What else is said: the files on disk used over the manifest, and
    /// the haks on disk a spread left as they are.
    pub fn notes(&self) -> Vec<String> {
        let mut out: Vec<String> =
            self.newer_haks.iter().map(|h| newer_note(&format!("hak {h}"))).collect();
        out.extend(self.newer_tlk.iter().map(|t| newer_note(&format!("talk table {t}"))));
        if !self.kept_on_disk.is_empty() {
            let others = match self.haks.len() {
                0 => "nothing written from it".to_string(),
                _ => "its resources spread over the others".to_string(),
            };
            out.push(format!(
                "the server publishes no origins file: {} on disk taken from disk as {}, \
                 {others}; on the server a hak listed above another is read over it",
                self.kept_on_disk.join(", "),
                if self.kept_on_disk.len() == 1 { "it is" } else { "they are" },
            ));
        }
        out
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
            0 if self.haks.is_empty() => "nothing to write".to_string(),
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
        let haks = match self.haks.len() {
            1 => "1 hak".to_string(),
            n => format!("{n} haks"),
        };
        format!(
            "{haks} from NWSync manifest {}{tlk}, {how}: {written}",
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
/// table `tlk` (and its feminine table) where the manifest has it, from
/// what is here (no network: the manifest [`mg_nwsync::find`] picks, as
/// [`mount`] does). With an origins file, the haks it names, each with
/// what it lists; without, the haks not on disk, the manifest's resources
/// spread over them. By the rule (see the module's notes), a hak or talk
/// table on disk (`on_disk`) newer than the manifest is not written: the
/// caller takes it from disk. A hak written before (for this manifest or
/// another) is not written again. An error where the manifest or its data
/// isn't here, saying how to download it.
pub fn write_haks(
    settings: &Settings,
    game: Option<&GameStorage>,
    names: &[String],
    tlk: Option<&str>,
    on_disk: &OnDisk,
    o: &WriteOptions<'_>,
) -> Result<HakFiles, String> {
    let cache = cache_of(settings)
        .ok_or("there is no cache folder to write haks into: set [nwsync] cache")?;
    let found = mg_nwsync::find(&settings.url, settings.manifest, Some(&cache), game)?;
    let store = Store { game, cache: &cache };
    let on_disk = &on_disk.without_cache(Some(cache.root()));
    let mut entries = found.entries.clone();
    let newer_haks = on_disk.newer_haks(found.at);
    let is_newer = |h: &str| newer_haks.iter().any(|n| n.eq_ignore_ascii_case(h));
    let mut kept_on_disk = Vec::new();
    // The haks to write: by the origins file, or to spread the manifest
    // over (the listed haks not on disk).
    let (wanted, not_listed) = match &found.origins {
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
            wanted.retain(|h| !is_newer(h) && o_.hak(h).is_some());
            (wanted, not_listed)
        }
        None => {
            let (kept, wanted): (Vec<String>, Vec<String>) =
                names.iter().cloned().partition(|h| on_disk.is_on_disk(h));
            kept_on_disk = kept.into_iter().filter(|h| !is_newer(h)).collect();
            (wanted, Vec::new())
        }
    };
    let plan = if wanted.is_empty() {
        let from = match found.origins {
            Some(_) => PlanFrom::Origins,
            None => PlanFrom::Spread,
        };
        haks::Plan { haks: Vec::new(), from }
    } else {
        if entries.iter().any(haks::size_unknown) {
            entries = haks::sizes(&entries, store)?;
        }
        // (The module's own resources, where the manifest was built with
        // them, left out of what is written: the origins file still lists
        // them, so they go after the plan is made.)
        let module = match (found.includes_module, &on_disk.module) {
            (Some(true), Some(m)) => Some(m),
            _ => None,
        };
        let mut plan = match &found.origins {
            Some(o_) => haks::from_origins(o_, &entries, &wanted)?,
            None => match module {
                Some(m) => haks::spread(&haks::without_module(&entries, m), &wanted)?,
                None => haks::spread(&entries, &wanted)?,
            },
        };
        for h in plan.haks.iter_mut() {
            h.resources.retain(|(k, _, _)| module.is_none_or(|m| !m.contains(k)));
        }
        plan
    };
    let written = haks::write_all(&plan, store, o.jobs, o.progress, o.stop)?;
    let mut files = Vec::new();
    let tlk_name = tlk.map(str::trim).filter(|t| !t.is_empty());
    let newer_tlk = tlk_name.filter(|_| on_disk.tlk_newer(found.at)).map(str::to_string);
    let mut table = |name: &str| -> Result<Option<(String, PathBuf)>, String> {
        let key = mg_resman::ResKey::parse(name, mg_core::ResType::TLK);
        let entry = key.and_then(|k| entries.iter().find(|(e, _, _)| *e == k).copied());
        let Some((key, sha, size)) = entry else { return Ok(None) };
        let path = haks::write_file(&key, &sha, size, store)?;
        let file = format!("{}.tlk", key.resref.to_lowercase());
        files.push((file.clone(), sha));
        Ok(Some((file, path)))
    };
    let (tlk, tlk_f) = match tlk_name.filter(|_| newer_tlk.is_none()) {
        Some(name) => match table(name)? {
            Some(male) => (Some(male), table(&crate::talk::feminine(name))?),
            None => (None, None),
        },
        None => (None, None),
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
        tlk_f,
        newer_haks,
        newer_tlk,
        kept_on_disk,
        not_listed,
        took: written.took,
    })
}

impl crate::Module {
    /// [`write_haks`] for the module's haks and talk table, where its
    /// project names an NWSync server (`None` where it names none), by
    /// the rule with what is on disk ([`crate::Module::nwsync_on_disk`]).
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
            let on_disk = self.nwsync_on_disk(install);
            let mut files =
                write_haks(&settings, game.as_ref(), &names, tlk.as_deref(), &on_disk, o)?;
            // (A newer table of the project's own, kept as JSON: made, as
            // the game reads no JSON, into the cache beside the haks.)
            if let (Some(name), Some(p), Some(cache)) =
                (&files.newer_tlk, &self.project, cache_of(&settings))
            {
                let language = mg_core::Language::from_short_code(&install.language);
                let made = |table: &str| -> Result<Option<(String, PathBuf)>, String> {
                    let tables = p.talk_tables(table).map_err(|e| e.to_string())?;
                    let Some(t) = crate::nasher::in_language(tables, language) else {
                        return Ok(None);
                    };
                    let bytes = t.tlk.to_bytes_as_nwn_tlk().map_err(|e| e.to_string())?;
                    let sha = mg_core::sha1::sha1(&bytes);
                    let path = cache.file_path(&sha, "tlk");
                    if !path.is_file() {
                        let dir = path.parent().unwrap_or(cache.root());
                        let part = dir.join(format!(".{}.part", mg_nwsync::sha1_hex(&sha)));
                        (std::fs::create_dir_all(dir))
                            .and_then(|()| std::fs::write(&part, &bytes))
                            .and_then(|()| std::fs::rename(&part, &path))
                            .map_err(|e| format!("{}: {e}", path.display()))?;
                    }
                    let file = format!("{}.tlk", table.to_lowercase());
                    let index = Index { haks: Vec::new(), files: vec![(file.clone(), sha)] };
                    let _ = cache.put_written(&files.manifest, index);
                    Ok(Some((file, path)))
                };
                if let Some(male) = made(name)? {
                    files.tlk_f = made(&crate::talk::feminine(name))?;
                    files.tlk = Some(male);
                }
            }
            Ok(files)
        })())
    }

    /// What of the module's content is on disk, for the rule: its haks
    /// where `install` finds them, its talk table's files (its project's
    /// own sources of it and of its feminine table, else the `tlk`
    /// folders'), and its own resources.
    pub fn nwsync_on_disk(&self, install: &GameInstall) -> OnDisk {
        let haks = self.haks().unwrap_or_default();
        let tlk = self.custom_tlk().ok().flatten();
        let own = match (&self.project, &tlk) {
            (Some(p), Some(name)) => [name.clone(), crate::talk::feminine(name)]
                .iter()
                .flat_map(|n| p.talk_tables(n).unwrap_or_default())
                .map(|t| t.path)
                .collect(),
            _ => Vec::new(),
        };
        let module = self.keys().copied().collect();
        OnDisk::of(install, &haks, tlk.as_deref(), own, Some(module))
    }

    /// [`mount`] for the module, by the rule with what is on disk: the
    /// layers of its haks are in `resman` already ([`crate::Module::stack`]
    /// puts them in; the window has them from the module's opening).
    pub fn mount_nwsync(
        &self,
        resman: &mut ResMan,
        install: &GameInstall,
        settings: &Settings,
        missing: &mut Vec<String>,
    ) -> Mounted {
        mount(resman, install, settings, &self.nwsync_on_disk(install), missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::ResType;
    use mg_core::sha1::sha1;
    use mg_nwsync::fixtures::{Registered, write_metadata, write_storage};
    use std::time::UNIX_EPOCH;

    const URL: &str = "https://nwsync.example.org";

    /// A game storage holding a manifest of `resources` (name, type,
    /// bytes), all of their data, registered at `came` (Unix seconds); the
    /// settings that read it, with a cache in `dir`.
    fn content(
        dir: &Path,
        resources: &[(&str, ResType, &[u8])],
        came: u64,
        with_module: Option<bool>,
    ) -> (GameStorage, Settings, Sha1) {
        let entries: Vec<_> = (resources.iter())
            .map(|(n, t, b)| (ResKey::parse(n, *t).unwrap(), sha1(b), b.len() as u32))
            .collect();
        let manifest = mg_nwsync::manifest::write(&entries);
        let storage = dir.join("user/nwsync");
        let _ = std::fs::remove_dir_all(&storage);
        write_storage(
            &storage,
            &[Registered {
                sha1: sha1(&manifest),
                origin_url: URL.into(),
                created_at: came as i64,
                last_used_at: came as i64,
                resources: entries.iter().map(|(k, s, _)| (*k, *s)).collect(),
                cached: Some(manifest.clone()),
            }],
            &[resources.iter().map(|(_, _, b)| (sha1(b), b.to_vec())).collect()],
        );
        if with_module.is_some() {
            write_metadata(&storage, &sha1(&manifest), with_module);
        }
        let settings = Settings {
            url: URL.into(),
            manifest: mg_nwsync::Wanted::Latest,
            cache: Some(dir.join("cache")),
        };
        (GameStorage::open(&storage).unwrap().unwrap(), settings, sha1(&manifest))
    }

    /// A file at `path`, its time `secs` (Unix seconds).
    fn file_at(path: &Path, secs: u64) -> PathBuf {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"the builder's").unwrap();
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
        path.to_path_buf()
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn resources_of(files: &HakFiles, hak: &str) -> Vec<String> {
        let bytes = std::fs::read(&files.hak(hak).unwrap().path).unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        let mut out: Vec<String> =
            (erf.entries.iter()).map(|e| ResKey::new(e.resref, e.restype).to_string()).collect();
        out.sort();
        out
    }

    const CAME: u64 = 1_700_000_000;
    const TABLES: [(&str, ResType, &[u8]); 2] =
        [("mine", ResType::TLK, b"TLK male"), ("minef", ResType::TLK, b"TLK female")];

    /// Without an origins file the manifest's resources go to the listed
    /// haks that are not on disk: a hak on disk (perhaps the builder's
    /// work never published) is never replaced by a slice of it, and that
    /// is said. One newer than the manifest is said to be used over it.
    /// Where every listed hak is on disk, nothing is written, and data not
    /// here is no error. The talk table and its feminine table come from
    /// the manifest, or both from disk where the table there is newer.
    #[test]
    fn a_spread_leaves_the_haks_on_disk() {
        let dir = mg_testkit::scratch_dir("nwsync-content-spread");
        let mut resources: Vec<(&str, ResType, &[u8])> = vec![
            ("a", ResType::TWODA, b"a"),
            ("b", ResType::TWODA, b"b"),
            ("c", ResType::NSS, b"c"),
        ];
        resources.extend(TABLES);
        let (game, settings, _) = content(&dir, &resources, CAME, None);
        let wip = file_at(&dir.join("user/hak/wip.hak"), CAME - 60);
        let mut on_disk = OnDisk {
            haks: vec![("wip".into(), wip.clone())],
            tlk_name: Some("mine".into()),
            tlk: vec![file_at(&dir.join("user/tlk/mine.tlk"), CAME - 60)],
            module: None,
        };
        let o = WriteOptions::default();
        let list = names(&["wip", "gone1", "gone2"]);
        let f = write_haks(&settings, Some(&game), &list, Some("mine"), &on_disk, &o).unwrap();
        assert_eq!(f.plan, PlanFrom::Spread);
        assert!(f.hak("wip").is_none());
        let mut spread = resources_of(&f, "gone1");
        spread.extend(resources_of(&f, "gone2"));
        spread.sort();
        assert_eq!(spread, ["a.2da", "b.2da", "c.nss"]);
        assert_eq!(f.kept_on_disk, ["wip"]);
        assert!(f.newer_haks.is_empty() && f.newer_tlk.is_none());
        assert_eq!(f.tlk.as_ref().map(|t| t.0.as_str()), Some("mine.tlk"));
        assert_eq!(std::fs::read(&f.tlk_f.as_ref().unwrap().1).unwrap(), b"TLK female");
        let notes = f.notes();
        assert!(notes[0].starts_with("the server publishes no origins file: wip on disk"));
        assert!(notes[0].contains("as it is, its resources spread"), "one space: {}", notes[0]);
        assert!(f.summary().starts_with("2 haks from NWSync manifest"), "{}", f.summary());

        // Newer than the manifest: used over it, said.
        file_at(&wip, CAME + 60);
        file_at(&on_disk.tlk[0], CAME + 60);
        let f = write_haks(&settings, Some(&game), &list, Some("mine"), &on_disk, &o).unwrap();
        assert!(f.hak("wip").is_none() && f.kept_on_disk.is_empty());
        assert_eq!(f.newer_haks, ["wip"]);
        assert_eq!(f.newer_tlk.as_deref(), Some("mine"));
        assert!(f.tlk.is_none() && f.tlk_f.is_none());
        assert_eq!(
            f.notes(),
            [
                "hak wip on disk is newer than the NWSync manifest: used over it",
                "talk table mine on disk is newer than the NWSync manifest: used over it",
            ]
        );

        // Every listed hak on disk: nothing to write, and the data need
        // not be here.
        // The storage is written again: the reader on the old one is
        // closed first, or Windows keeps its files (open) and the folder.
        drop(game);
        registered_without_data(&dir, &resources);
        std::fs::remove_dir_all(dir.join("cache")).unwrap();
        let game = GameStorage::open(&dir.join("user/nwsync")).unwrap().unwrap();
        file_at(&wip, CAME - 60);
        on_disk.tlk.clear();
        let f = write_haks(&settings, Some(&game), &names(&["wip"]), None, &on_disk, &o).unwrap();
        assert!(f.haks.is_empty());
        assert!(f.summary().starts_with("0 haks from"), "{}", f.summary());
        assert!(f.notes().iter().any(|n| n.contains("nothing written from it")), "{:?}", f.notes());
        // (With one not on disk, the missing data is the error it was.)
        let e = write_haks(&settings, Some(&game), &list, None, &on_disk, &o).unwrap_err();
        assert!(e.contains("mg nwsync-pull"), "{e}");
    }

    /// With an origins file, a hak it names that is on disk and newer than
    /// the manifest is taken from disk (not written), one older is written
    /// from the manifest (it is under the manifest, as in the game); a
    /// manifest built with the module's resources has them left out.
    #[test]
    fn origins_and_the_haks_on_disk() {
        let dir = mg_testkit::scratch_dir("nwsync-content-origins");
        let resources: [(&str, ResType, &[u8]); 3] = [
            ("a", ResType::TWODA, b"a"),
            ("b", ResType::TWODA, b"b"),
            ("mg_own", ResType::NSS, b"server's"),
        ];
        let (game, settings, sha) = content(&dir, &resources, CAME, Some(true));
        let origins =
            "Erf:/srv/hak/one.hak\n\ta.2da\nErf:/srv/hak/two.hak\n\tb.2da\n\tmg_own.nss\n";
        mg_nwsync::Cache::new(dir.join("cache"))
            .put_manifest(&sha, "origin", origins.as_bytes())
            .unwrap();
        let one = file_at(&dir.join("user/hak/one.hak"), CAME + 60);
        let two = file_at(&dir.join("user/hak/two.hak"), CAME - 60);
        let own = ResKey::parse("mg_own", ResType::NSS).unwrap();
        let on_disk = OnDisk {
            haks: vec![("one".into(), one), ("two".into(), two)],
            module: Some([own].into_iter().collect()),
            ..Default::default()
        };
        let o = WriteOptions::default();
        let f = write_haks(&settings, Some(&game), &names(&["one", "two"]), None, &on_disk, &o)
            .unwrap();
        assert_eq!(f.plan, PlanFrom::Origins);
        assert!(f.hak("one").is_none());
        assert_eq!(f.newer_haks, ["one"]);
        assert_eq!(resources_of(&f, "two"), ["b.2da"]);
        assert!(f.summary().starts_with("1 hak from NWSync manifest"), "{}", f.summary());
        assert_eq!(f.notes(), ["hak one on disk is newer than the NWSync manifest: used over it"]);
    }

    /// A hak at `path` holding `resources` (name, type, bytes), its time
    /// `secs`.
    fn hak_at(path: &Path, resources: &[(&str, ResType, &[u8])], secs: u64) -> PathBuf {
        let mut w = mg_erf::ErfWriter::new(*b"HAK ");
        for (name, t, bytes) in resources {
            w.add(mg_core::ResRef::from_str(name).unwrap(), *t, bytes.to_vec()).unwrap();
        }
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, w.to_bytes().unwrap()).unwrap();
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
        path.to_path_buf()
    }

    /// End to end: a module published with its origins file (`mg nwsync
    /// --write-origins`) into a folder while it listed one hak, read
    /// through a file source (`source = "file://…"`) once it lists a
    /// second that is on disk and not in the manifest. The reader takes
    /// Moonglow's origins: the first hak comes from the manifest (written
    /// from it, held by the layer), the second from disk (not planned,
    /// not missing, and not by the newer-on-disk rule: it is older). A
    /// hak neither on disk nor in the manifest stays missing; without the
    /// origins file every hak not on disk would be taken to be in it.
    #[test]
    fn a_publish_with_origins_read_back_takes_the_hak_outside_it_from_disk() {
        let dir = mg_testkit::scratch_dir("nwsync-content-published-origins");
        let gi = GameInstall::new(dir.join("game"), Some(dir.join("user")), "en");
        let one = [("pw_head", ResType::MDL, &b"a head"[..]), ("pw_parts", ResType::TWODA, b"2DA")];
        hak_at(&dir.join("user/hak/pw_one.hak"), &one, CAME);
        let mut m = crate::Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        info.root.set("Mod_HakList", crate::attach::hak_list(&info.root, &names(&["pw_one"])));
        m.set_info(&info).unwrap();
        let rm = ResMan::new();
        let (contents, missing) =
            crate::nwsync::module_contents(&m, &gi, &rm, false, None).unwrap();
        assert!(missing.is_empty(), "{missing:?}");
        let repo = dir.join("repo");
        let o = crate::nwsync::Options { origins: true, latest: true, ..Default::default() };
        let written = crate::nwsync::write(&repo, &contents, &o, &mut |_, _| {}).unwrap();
        assert_eq!(written.origins, Some(1));

        // The builder's machine: pw_one only on the server now, pw_two on
        // disk (older than the manifest came), pw_gone nowhere.
        std::fs::remove_file(dir.join("user/hak/pw_one.hak")).unwrap();
        let two = hak_at(&dir.join("user/hak/pw_two.hak"), &[("pw_tex", ResType::TGA, b"x")], CAME);
        // (Read through a file source: the folder Moonglow published into.)
        let settings = Settings {
            url: mg_nwsync::folder_url(&repo),
            manifest: mg_nwsync::Wanted::Latest,
            cache: Some(dir.join("cache")),
        };
        let fetch = mg_nwsync::fetch::Folder::of(&settings.url).unwrap();
        let cache = cache_of(&settings).unwrap();
        let pulled = mg_nwsync::fetch::pull(
            &fetch,
            &settings,
            &cache,
            None,
            Default::default(),
            &|_| {},
            &|| false,
        )
        .unwrap();
        assert!(pulled.resolved.found.origins.is_some());
        assert_eq!(pulled.resolved.found.from, ManifestFrom::Folder);

        let listed = names(&["pw_one", "pw_two", "pw_gone"]);
        let on_disk = OnDisk { haks: vec![("pw_two".into(), two)], ..Default::default() };
        let f =
            write_haks(&settings, None, &listed, None, &on_disk, &WriteOptions::default()).unwrap();
        assert_eq!(f.plan, PlanFrom::Origins);
        assert_eq!(f.haks.len(), 1);
        assert_eq!(resources_of(&f, "pw_one"), ["pw_head.mdl", "pw_parts.2da"]);
        assert!(f.hak("pw_two").is_none() && f.hak("pw_gone").is_none());
        assert!(f.newer_haks.is_empty() && f.kept_on_disk.is_empty(), "{:?}", f.newer_haks);

        let mut rm = ResMan::new();
        let mut missing = names(&["pw_one", "pw_gone"]);
        let mounted = mount(&mut rm, &gi, &settings, &on_disk, &mut missing);
        let layer = mounted.layer.unwrap();
        assert!(layer.by_origins && layer.newer_haks.is_empty());
        assert_eq!(layer.haks, ["pw_one"]);
        assert_eq!(missing, ["pw_gone"]);
        assert!(rm.contains(&ResKey::parse("pw_head", ResType::MDL).unwrap()));

        // Without the origins file, every hak not on disk counts as held.
        let sha = mg_nwsync::parse_sha1(&written.sha1).unwrap();
        std::fs::remove_file(cache.root().join(format!("manifests/{}.origin", written.sha1)))
            .unwrap();
        assert!(cache.manifest(&sha, "origin").is_none());
        let mut missing = names(&["pw_one", "pw_gone"]);
        let layer = mount(&mut ResMan::new(), &gi, &settings, &on_disk, &mut missing).layer;
        assert!(!layer.unwrap().by_origins);
        assert!(missing.is_empty());
    }

    /// A project's own talk table, kept as JSON and newer than the
    /// manifest: [`crate::Module::write_nwsync_haks`] (Test Module's,
    /// `mg nwsync-haks`'s) makes it into the cache in place of the
    /// manifest's, the game reading no JSON.
    #[test]
    fn a_projects_newer_talk_table_is_made_for_the_game() {
        let dir = mg_testkit::scratch_dir("nwsync-content-project-tlk");
        let (_, settings, _) = content(&dir, &TABLES, CAME, None);
        let mut m = crate::Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mine".to_vec()));
        m.set_info(&info).unwrap();
        let project = dir.join("project");
        let at = crate::ModuleLocation::Project { root: project.clone(), target: "default".into() };
        m.save_as(&at).unwrap();
        let mut cfg = std::fs::read_to_string(project.join("nasher.cfg")).unwrap();
        cfg.push_str("\n[target]\nname = \"tlk\"\nfile = \"mine.tlk\"\n  [target.sources]\n  include = \"src/tlk/*.json\"\n");
        std::fs::write(project.join("nasher.cfg"), cfg).unwrap();
        std::fs::create_dir_all(project.join("src/tlk")).unwrap();
        let json = r#"{"language":0,"entries":[{"id":0,"text":"Ours"}]}"#;
        std::fs::write(project.join("src/tlk/mine.tlk.json"), json).unwrap();
        let mut m = crate::Module::open(&project).unwrap();
        m.project_file.nwsync_source = Some(settings.url.clone());
        m.project_file.nwsync_cache = settings.cache.clone();
        let gi = GameInstall::new(dir.join("game"), Some(dir.join("user")), "en");
        let f = m.write_nwsync_haks(&gi, &WriteOptions::default()).unwrap().unwrap();
        assert_eq!(f.newer_tlk.as_deref(), Some("mine"));
        let (name, path) = f.tlk.as_ref().unwrap();
        assert_eq!(name, "mine.tlk");
        assert_eq!(
            mg_tlk::Tlk::read(&std::fs::read(path).unwrap()).unwrap().entries[0].text,
            b"Ours"
        );
        assert!(f.tlk_f.is_none());
    }

    /// The manifest's registration in a storage that holds none of its
    /// data (the data of the manifest is not here).
    fn registered_without_data(
        dir: &Path,
        resources: &[(&str, ResType, &[u8])],
    ) -> mg_nwsync::fixtures::Registered {
        let entries: Vec<_> = (resources.iter())
            .map(|(n, t, b)| (ResKey::parse(n, *t).unwrap(), sha1(b), b.len() as u32))
            .collect();
        let manifest = mg_nwsync::manifest::write(&entries);
        let storage = dir.join("user/nwsync");
        let _ = std::fs::remove_dir_all(&storage);
        let r = Registered {
            sha1: sha1(&manifest),
            origin_url: URL.into(),
            created_at: CAME as i64,
            last_used_at: CAME as i64,
            resources: entries.iter().map(|(k, s, _)| (*k, *s)).collect(),
            cached: Some(manifest),
        };
        write_storage(&storage, std::slice::from_ref(&r), &[Vec::new()]);
        r
    }
}
