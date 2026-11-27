//! `mg nwsync-status` and `mg nwsync-pull`: a module's content read from
//! the NWSync server that hands it to players (`[nwsync] source` in its
//! project file, or `--source`), what of it is here (the game's NWSync
//! storage, read only, and Moonglow's own cache), and the download of
//! what isn't into the cache ([`mg_nwsync`]). `mg nwsync-haks` writes
//! the module's haks from it (as `mg serve` does for its server), and
//! `mg nwsync-cache-prune` takes out of the cache what is no longer
//! needed.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use mg_nwsync::fetch::{self, Fetch, Step};
use mg_nwsync::{Cache, GameStorage, Settings, Survey, Wanted, sha1_hex};
use serde_json::json;

use crate::{Cli, Output, path_text};

/// What the two commands are given.
#[derive(Debug, Clone, clap::Args)]
pub(crate) struct Source {
    /// The module, whose project names its server ([nwsync] source in
    /// moonglow.toml); none where --source is given.
    pub module: Option<PathBuf>,
    /// The NWSync server, as nwserver's -nwsyncurl, or a repository
    /// folder on this computer (a path, or a file:// URL) (default: the
    /// project's [nwsync] source).
    #[arg(long, value_name = "URL")]
    pub source: Option<String>,
    /// Which manifest: latest, or one's SHA-1 (default: the project's
    /// [nwsync] manifest, else latest).
    #[arg(long, value_name = "SHA1")]
    pub manifest: Option<String>,
    /// Moonglow's cache folder for NWSync data (default: the project's
    /// [nwsync] cache, else nwsync in Moonglow's cache folder).
    #[arg(long, value_name = "DIR")]
    pub cache: Option<PathBuf>,
}

/// The settings: the project's, with what the command line says over them
/// (and what to say of that).
fn settings(s: &Source) -> Result<(Settings, Vec<String>)> {
    let mut notes = Vec::new();
    let project = match &s.module {
        Some(path) => crate::open_module(path)?.project_file.nwsync_settings(),
        None => None,
    };
    let url = match (&s.source, &project) {
        (Some(u), _) => {
            // (A folder's path from where mg runs: one that is there by
            // its name alone too, where a name alone that isn't is taken
            // for a server's name without its https://.)
            let here = std::env::current_dir().ok();
            let plain = (!u.contains("://")).then(|| Path::new(u.trim()).to_path_buf());
            let plain = plain.filter(|p| p.is_absolute() || p.is_dir());
            let path = plain.map(|p| if p.is_absolute() { p } else { Path::new(".").join(p) });
            let text = path.map_or_else(|| u.clone(), |p| p.to_string_lossy().into_owned());
            mg_nwsync::source_url(&text, here.as_deref()).ok_or_else(|| {
                anyhow!(
                    "--source is the NWSync server's address, as nwserver's -nwsyncurl \
                     (https://nwsync.example.org), or a repository folder: {u:?}"
                )
            })?
        }
        (None, Some(p)) => p.url.clone(),
        (None, None) if s.module.is_some() => bail!(
            "the module's project names no NWSync server ([nwsync] source in moonglow.toml): give \
             one, --source https://nwsync.example.org"
        ),
        (None, None) => bail!("give a module whose project names its NWSync server, or --source"),
    };
    let manifest = match &s.manifest {
        Some(m) if m.trim().eq_ignore_ascii_case("latest") => Wanted::Latest,
        Some(m) => Wanted::Pinned(
            mg_nwsync::parse_sha1(m.trim())
                .ok_or_else(|| anyhow!("--manifest is latest or a manifest's SHA-1: {m:?}"))?,
        ),
        None => match &project {
            // (Another server than the project's: the project's pin is a
            // manifest of its own server's.)
            Some(p) if mg_nwsync::server_key(&p.url) != mg_nwsync::server_key(&url) => {
                if let Wanted::Pinned(sha) = p.manifest {
                    notes.push(format!(
                        "the project's [nwsync] manifest {} is its own server's ({}), not taken \
                         for {url}: its latest instead (--manifest picks one)",
                        &sha1_hex(&sha)[..8],
                        p.url
                    ));
                }
                Wanted::Latest
            }
            Some(p) => p.manifest,
            None => Wanted::Latest,
        },
    };
    let cache = s.cache.clone().or_else(|| project.and_then(|p| p.cache));
    Ok((Settings { url, manifest, cache }, notes))
}

/// The game's NWSync storage, as `mg` finds the game's user folder:
/// `--user-dir`, else where the game keeps it (none with `--no-user-dir`).
fn game_storage(cli: &Cli) -> Result<Option<GameStorage>> {
    if cli.no_user_dir {
        return Ok(None);
    }
    let user = cli.user_dir.clone().or_else(mg_resman::GameInstall::detect_user_dir);
    match user {
        Some(u) => GameStorage::open(&mg_nwsync::game::dir_in(&u)).map_err(anyhow::Error::msg),
        None => Ok(None),
    }
}

fn cache_of(settings: &Settings) -> Result<Cache> {
    mg_module::nwsync_content::cache_of(settings)
        .context("no cache folder for NWSync data: give one, --cache DIR")
}

/// The source as a line says it: the server's address, or the folder.
fn source_line(settings: &Settings) -> String {
    match mg_nwsync::local_folder(&settings.url) {
        Some(dir) => format!("repository folder: {}", dir.display()),
        None => format!("server: {}", settings.url),
    }
}

/// What makes the requests for `settings` ([`fetch::for_settings`]).
fn fetcher(settings: &Settings) -> Result<std::sync::Arc<dyn Fetch>> {
    fetch::for_settings(settings).map_err(anyhow::Error::msg)
}

fn megabytes(n: u64) -> String {
    format!("{:.1} MB", n as f64 / 1_048_576.0)
}

/// What is said of what is here (both commands).
fn survey_json(s: &Survey) -> serde_json::Value {
    json!({
        "resources": s.resources,
        "data_files": s.data_files,
        "in_game_storage": s.in_game,
        "in_cache": s.in_cache,
        "missing": s.missing.len(),
        "missing_bytes": s.missing_bytes,
    })
}

fn survey_lines(out: &mut Output, s: &Survey, game: Option<&Path>, cache: &Path) {
    out.line(format!("resources: {} in {} data files", s.resources, s.data_files));
    match game {
        Some(dir) => {
            out.line(format!("in the game's NWSync storage: {} ({})", s.in_game, dir.display()))
        }
        None => out.line(
            "in the game's NWSync storage: none (no user folder, or no NWSync storage in it)"
                .to_string(),
        ),
    }
    out.line(format!("in Moonglow's cache: {} ({})", s.in_cache, cache.display()));
    out.line(format!(
        "missing: {}{}",
        s.missing.len(),
        if s.missing_bytes > 0 { format!(" ({})", megabytes(s.missing_bytes)) } else { "".into() }
    ));
}

fn origins_line(origins: Option<&mg_nwsync::Origins>) -> String {
    match origins {
        Some(o) => {
            let haks = o.haks().count();
            let others = o.containers.len().saturating_sub(haks);
            format!(
                "origins: {haks} {} and {others} other {} named",
                if haks == 1 { "hak" } else { "haks" },
                if others == 1 { "container" } else { "containers" }
            )
        }
        None => "origins: none (the server publishes no origins file for it)".into(),
    }
}

/// `mg nwsync-status`.
pub(crate) fn status(cli: &Cli, s: &Source, online: bool) -> Result<Output> {
    let (settings, said) = settings(s)?;
    let cache = cache_of(&settings)?;
    let game = game_storage(cli)?;
    let mut out = Output::default();
    for n in said {
        out.note(n);
    }
    let (found, server_latest) = if online {
        let r = fetch::resolve(&*fetcher(&settings)?, &settings, &cache, game.as_ref())
            .map_err(anyhow::Error::msg)?;
        for w in &r.warnings {
            out.note(format!("warning: {w}"));
        }
        (r.found, r.server_latest)
    } else {
        let found = mg_nwsync::find(&settings.url, settings.manifest, Some(&cache), game.as_ref())
            .map_err(anyhow::Error::msg)?;
        (found, None)
    };
    let survey = mg_nwsync::survey(&found.entries, game.as_ref(), Some(&cache))
        .map_err(anyhow::Error::msg)?;
    let game_dir = game.as_ref().map(|g| g.dir.clone());
    out.line(source_line(&settings));
    out.line(format!("manifest: {} ({})", sha1_hex(&found.sha1), found.from.said()));
    if let Some(latest) = server_latest.filter(|l| *l != found.sha1) {
        out.line(format!("the server's latest: {}", sha1_hex(&latest)));
    }
    survey_lines(&mut out, &survey, game_dir.as_deref(), cache.root());
    out.line(origins_line(found.origins.as_ref()));
    if !survey.complete() {
        out.note(format!(
            "{} data files ({}) are to be downloaded: mg nwsync-pull",
            survey.missing.len(),
            megabytes(survey.missing_bytes)
        ));
    }
    out.json = json!({
        "server": settings.url,
        "manifest": sha1_hex(&found.sha1),
        "from": from_word(found.from),
        "server_latest": server_latest.map(|l| sha1_hex(&l)),
        "game_storage": game_dir.as_deref().map(path_text),
        "cache": path_text(cache.root()),
        "origins": found.origins.as_ref().map(|o| o.haks().collect::<Vec<_>>()),
        "survey": survey_json(&survey),
    });
    Ok(out)
}

fn from_word(from: mg_nwsync::ManifestFrom) -> &'static str {
    match from {
        mg_nwsync::ManifestFrom::Server => "server",
        mg_nwsync::ManifestFrom::Folder => "folder",
        mg_nwsync::ManifestFrom::Cache => "cache",
        mg_nwsync::ManifestFrom::GameStorage => "game_storage",
    }
}

/// `mg nwsync-pull`.
pub(crate) fn pull(cli: &Cli, s: &Source, jobs: usize, sample: Option<usize>) -> Result<Output> {
    let (settings, said) = settings(s)?;
    let cache = cache_of(&settings)?;
    let game = game_storage(cli)?;
    let mut out = pull_with(
        &*fetcher(&settings)?,
        &settings,
        &cache,
        game.as_ref(),
        jobs,
        sample,
        !cli.json,
    )?;
    out.notes.splice(0..0, said);
    Ok(out)
}

/// [`pull`] through `fetch`, with progress on standard error where
/// `show` (each few seconds).
fn pull_with(
    fetch: &dyn Fetch,
    settings: &Settings,
    cache: &Cache,
    game: Option<&GameStorage>,
    jobs: usize,
    sample: Option<usize>,
    show: bool,
) -> Result<Output> {
    let last = Mutex::new(Instant::now());
    let step = |s: Step| {
        if !show {
            return;
        }
        match s {
            Step::Asking => match mg_nwsync::local_folder(&settings.url) {
                Some(dir) => eprintln!("reading the manifest of {}", dir.display()),
                None => eprintln!("asking {} for its manifest", settings.url),
            },
            Step::Looking => eprintln!("looking for its data in the game's storage and the cache"),
            Step::Downloading(p) => {
                let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
                if last.elapsed() >= Duration::from_secs(3) || p.files == p.of_files {
                    *last = Instant::now();
                    eprintln!(
                        "{} of {} data files, {} of {} ({} received)",
                        p.files,
                        p.of_files,
                        megabytes(p.bytes),
                        megabytes(p.of_bytes),
                        megabytes(p.received)
                    );
                }
            }
        }
    };
    let o = fetch::Options { jobs: jobs.max(1), most: sample };
    let p = fetch::pull(fetch, settings, cache, game, o, &step, &|| false)
        .map_err(anyhow::Error::msg)?;
    let mut out = Output::default();
    for w in &p.resolved.warnings {
        out.note(format!("warning: {w}"));
    }
    let found = &p.resolved.found;
    let game_dir = game.map(|g| g.dir.clone());
    out.line(source_line(settings));
    out.line(format!("manifest: {} ({})", sha1_hex(&found.sha1), found.from.said()));
    survey_lines(&mut out, &p.before, game_dir.as_deref(), cache.root());
    out.line(origins_line(found.origins.as_ref()));
    let d = &p.downloaded;
    out.line(format!(
        "downloaded: {} data files ({} received){}",
        d.files,
        megabytes(d.received),
        match sample {
            Some(n) if n < p.before.missing.len() => format!(", a sample of {n}"),
            _ => String::new(),
        }
    ));
    for (sha, why) in d.failed.iter().take(20) {
        out.note(format!("warning: data file {} not downloaded: {why}", sha1_hex(sha)));
    }
    if d.failed.len() > 20 {
        out.note(format!("warning: {} more data files not downloaded", d.failed.len() - 20));
    }
    let left = p.before.missing.len().saturating_sub(d.files);
    if left == 0 {
        out.note("all of the manifest's data is here");
    } else {
        out.note(format!("{left} data files still to be downloaded: mg nwsync-pull again"));
    }
    out.json = json!({
        "server": settings.url,
        "manifest": sha1_hex(&found.sha1),
        "from": from_word(found.from),
        "server_latest": p.resolved.server_latest.map(|l| sha1_hex(&l)),
        "game_storage": game_dir.as_deref().map(path_text),
        "cache": path_text(cache.root()),
        "origins": found.origins.as_ref().map(|o| o.haks().collect::<Vec<_>>()),
        "before": survey_json(&p.before),
        "downloaded": d.files,
        "received_bytes": d.received,
        "failed": d.failed.iter().map(|(s, why)| json!({"sha1": sha1_hex(s), "error": why})).collect::<Vec<_>>(),
        "left": left,
        "warnings": p.resolved.warnings,
    });
    if !d.failed.is_empty() {
        out.failed = true;
    }
    Ok(out)
}

/// A line on how far a write of haks is.
pub(crate) fn hak_progress(p: &mg_nwsync::haks::Progress) -> String {
    format!(
        "writing haks from NWSync: {} of {} haks, {} of {}",
        p.haks,
        p.of_haks,
        megabytes(p.bytes),
        megabytes(p.of_bytes)
    )
}

/// What `mg nwsync-haks` is told besides the content's source.
#[derive(Debug, Clone)]
pub(crate) struct HakArgs {
    pub out: Option<PathBuf>,
    pub all: bool,
    pub copy: bool,
    pub jobs: usize,
}

/// Puts a written file at `to`: a hard link (it stays when the cache is
/// pruned), else a copy; a copy with `copy`. Made beside it and moved
/// into place: what was at `to` is replaced whole (never written through
/// a link), and stays until the new file is there.
fn put(from: &Path, to: &Path, copy: bool) -> Result<()> {
    if same_file(from, to) {
        return Ok(());
    }
    let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let part = to.with_file_name(format!(".{name}.{}.mg-part", std::process::id()));
    let _ = std::fs::remove_file(&part);
    let made = match !copy && std::fs::hard_link(from, &part).is_ok() {
        true => Ok(()),
        false => std::fs::copy(from, &part).map(|_| ()),
    };
    made.and_then(|()| std::fs::rename(&part, to)).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        anyhow::Error::new(e).context(to.display().to_string())
    })
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// `mg nwsync-haks`.
pub(crate) fn haks(cli: &Cli, s: &Source, a: &HakArgs) -> Result<Output> {
    let (settings, said) = settings(s)?;
    // (Before anything is made in it: not the player's own folder, with
    // the game installed or not.)
    if let Some(dir) = &a.out {
        match crate::install(cli) {
            Ok(gi) => crate::serve::refuse_dir(&gi, dir, None)?,
            Err(_) => crate::serve::refuse_players_dir(cli.user_dir.as_deref(), dir, None)?,
        }
    }
    let game = game_storage(cli)?;
    let module = match &s.module {
        Some(path) => Some(crate::open_module(path)?),
        None => None,
    };
    let (names, tlk) = match &module {
        Some(m) => (m.haks()?, m.custom_tlk()?),
        None => {
            // (Without a module, the haks the origins file names, and the
            // manifest's talk table.)
            let cache = cache_of(&settings)?;
            let found =
                mg_nwsync::find(&settings.url, settings.manifest, Some(&cache), game.as_ref())
                    .map_err(anyhow::Error::msg)?;
            if found.origins.is_none() {
                bail!(
                    "the server publishes no origins file for manifest {}: give the module, whose \
                     haks the manifest's resources are spread over",
                    found.short()
                );
            }
            let tlks: Vec<String> = (found.entries.iter())
                .filter(|(k, _, _)| k.restype == mg_core::ResType::TLK)
                .map(|(k, _, _)| k.resref.to_string())
                .collect();
            (Vec::new(), (tlks.len() == 1).then(|| tlks[0].clone()))
        }
    };
    // What is on disk, for the rule (a hak or talk table newer than the
    // manifest is taken from disk, not written); the module's resources.
    let on_disk = match (&module, crate::install(cli)) {
        (Some(m), Ok(gi)) => m.nwsync_on_disk(&gi),
        (Some(m), Err(_)) => mg_module::nwsync_content::OnDisk {
            module: Some(m.keys().copied().collect()),
            ..Default::default()
        },
        (None, _) => Default::default(),
    };
    let all = a.all || module.is_none();
    let last = Mutex::new(Instant::now() - Duration::from_secs(60));
    let show = !cli.json;
    let progress = |p: mg_nwsync::haks::Progress| {
        if !show || p.of_bytes == 0 {
            return;
        }
        let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
        if last.elapsed() >= Duration::from_secs(3) {
            *last = Instant::now();
            eprintln!("{}", hak_progress(&p));
        }
    };
    let o = mg_module::nwsync_content::WriteOptions {
        jobs: a.jobs.max(1),
        all,
        progress: &progress,
        ..Default::default()
    };
    let started = Instant::now();
    let files = mg_module::nwsync_content::write_haks(
        &settings,
        game.as_ref(),
        &names,
        tlk.as_deref(),
        &on_disk,
        &o,
    )
    .map_err(anyhow::Error::msg)?;
    let mut out = Output::default();
    for n in said.into_iter().chain(files.notes()) {
        out.note(n);
    }
    out.line(source_line(&settings));
    out.line(format!("manifest: {} ({})", sha1_hex(&files.manifest), files.from.said()));
    let mut placed: Vec<(String, PathBuf)> = Vec::new();
    if let Some(dir) = &a.out {
        for sub in ["hak", "tlk"] {
            std::fs::create_dir_all(dir.join(sub))
                .with_context(|| dir.join(sub).display().to_string())?;
        }
        for h in &files.haks {
            let file = mg_resman::named_file(&h.name, "hak")
                .ok_or_else(|| anyhow!("hak {}: {}", h.name, mg_resman::NO_FILE_NAME))?;
            let to = dir.join("hak").join(file);
            put(&h.path, &to, a.copy)?;
            placed.push((h.name.clone(), to));
        }
        for (name, path) in files.tlk.iter().chain(&files.tlk_f) {
            let to = dir.join("tlk").join(name);
            put(path, &to, a.copy)?;
            placed.push((name.clone(), to));
        }
    }
    let at = |name: &str, cached: &Path| {
        (placed.iter().find(|(n, _)| n == name).map(|(_, p)| p.clone()))
            .unwrap_or_else(|| cached.to_path_buf())
    };
    for h in &files.haks {
        out.line(format!(
            "{}.hak: {} resources, {}, {} ({})",
            h.name,
            h.resources,
            megabytes(h.bytes),
            if h.written { "written" } else { "written before" },
            at(&h.name, &h.path).display()
        ));
    }
    for (name, path) in files.tlk.iter().chain(&files.tlk_f) {
        out.line(format!("{name}: {}", at(name, path).display()));
    }
    if !files.not_listed.is_empty() {
        out.note(format!(
            "the origins file names haks the module doesn't list, not written (--all writes them): \
             {}",
            files.not_listed.join(", ")
        ));
    }
    let (written, bytes) = (files.haks.iter().filter(|h| h.written))
        .fold((0usize, 0u64), |(n, b), h| (n + 1, b + h.bytes));
    let took = started.elapsed().as_secs_f64();
    out.note(format!(
        "{} {} ({}), {written} written now ({}), in {took:.1} s{}",
        files.haks.len(),
        if files.haks.len() == 1 { "hak" } else { "haks" },
        megabytes(files.haks.iter().map(|h| h.bytes).sum()),
        megabytes(bytes),
        match &a.out {
            Some(dir) => format!("; in {}", dir.display()),
            None => String::new(),
        }
    ));
    out.json = json!({
        "server": settings.url,
        "manifest": sha1_hex(&files.manifest),
        "from": from_word(files.from),
        "plan": match files.plan {
            mg_nwsync::haks::PlanFrom::Origins => "origins",
            mg_nwsync::haks::PlanFrom::Spread => "spread",
        },
        "haks": files.haks.iter().map(|h| json!({
            "name": h.name,
            "resources": h.resources,
            "bytes": h.bytes,
            "written": h.written,
            "file": path_text(&at(&h.name, &h.path)),
            "cached": path_text(&h.path),
        })).collect::<Vec<_>>(),
        "tlk": files.tlk.as_ref().map(|(n, p)| json!({"name": n, "file": path_text(&at(n, p))})),
        "tlk_feminine": files.tlk_f.as_ref().map(|(n, p)| json!({"name": n, "file": path_text(&at(n, p))})),
        "newer_on_disk": files.newer_haks.iter().map(|h| format!("{h}.hak"))
            .chain(files.newer_tlk.iter().map(|t| format!("{t}.tlk"))).collect::<Vec<_>>(),
        "kept_on_disk": files.kept_on_disk,
        "not_listed": files.not_listed,
        "seconds": took,
    });
    Ok(out)
}

/// What `mg nwsync-cache-prune` is told.
#[derive(Debug, Clone, clap::Args)]
pub(crate) struct PruneArgs {
    /// A module whose current manifest is kept (and whose project's
    /// [nwsync] cache and keep are used).
    pub module: Option<PathBuf>,
    /// Moonglow's cache folder for NWSync data (default: the project's
    /// [nwsync] cache, else nwsync in Moonglow's cache folder).
    #[arg(long, value_name = "DIR")]
    pub cache: Option<PathBuf>,
    /// How many manifests last used are kept besides the module's
    /// (default: the project's [nwsync] keep, else 3).
    #[arg(long, value_name = "N")]
    pub keep: Option<usize>,
    /// Files changed within this many hours are left alone (a download
    /// or a write may be under way).
    #[arg(long, value_name = "HOURS", default_value_t = 1.0)]
    pub min_age: f64,
    /// Say what would be removed, and remove nothing.
    #[arg(long)]
    pub dry_run: bool,
}

/// `mg nwsync-cache-prune`.
pub(crate) fn cache_prune(cli: &Cli, a: &PruneArgs) -> Result<Output> {
    let game = game_storage(cli)?;
    let module = match &a.module {
        Some(path) => Some(crate::open_module(path)?),
        None => None,
    };
    let project = module.as_ref().map(|m| &m.project_file);
    let settings = project.and_then(|p| p.nwsync_settings());
    let cache_dir = (a.cache.clone())
        .or_else(|| settings.as_ref().and_then(|s| s.cache.clone()))
        .or_else(mg_module::nwsync_content::default_cache)
        .context("no cache folder for NWSync data: give one, --cache DIR")?;
    let cache = Cache::new(cache_dir);
    let mut keep = mg_nwsync::prune::Keep {
        recent: a
            .keep
            .or_else(|| project.and_then(|p| p.nwsync_keep))
            .unwrap_or(mg_nwsync::prune::RECENT),
        min_age: Duration::from_secs_f64(a.min_age.max(0.0) * 3600.0),
        ..Default::default()
    };
    let mut out = Output::default();
    if let Some(s) = &settings {
        if let Wanted::Pinned(sha) = s.manifest {
            keep.manifests.push(sha);
        }
        match mg_nwsync::find(&s.url, s.manifest, Some(&cache), game.as_ref()) {
            Ok(found) => keep.manifests.push(found.sha1),
            Err(e) => out.note(format!("warning: the module's manifest: {e}")),
        }
    }
    let p = mg_nwsync::prune::prune(&cache, game.as_ref(), &keep, a.dry_run)
        .map_err(anyhow::Error::msg)?;
    let would = if p.dry_run { "would be removed" } else { "removed" };
    out.line(format!("cache: {}", cache.root().display()));
    for sha in &p.kept {
        out.line(format!("kept: manifest {}", sha1_hex(sha)));
    }
    for sha in &p.removed {
        out.line(format!("{would}: manifest {}", sha1_hex(sha)));
    }
    out.line(format!("data files {would}: {} ({})", p.data_files, megabytes(p.data_bytes)));
    out.line(format!("written haks {would}: {} ({})", p.haks, megabytes(p.hak_bytes)));
    out.line(format!("other files {would}: {} ({})", p.other, megabytes(p.other_bytes)));
    if let Some(why) = &p.data_left {
        out.note(format!("warning: {why}"));
    }
    for n in &p.notes {
        out.note(n.clone());
    }
    out.note(format!(
        "{} {would}{}",
        megabytes(p.bytes()),
        if p.dry_run { " (a dry run: nothing was)" } else { "" }
    ));
    out.json = json!({
        "cache": path_text(cache.root()),
        "dry_run": p.dry_run,
        "kept": p.kept.iter().map(sha1_hex).collect::<Vec<_>>(),
        "removed": p.removed.iter().map(sha1_hex).collect::<Vec<_>>(),
        "data_files": p.data_files,
        "data_bytes": p.data_bytes,
        "haks": p.haks,
        "hak_bytes": p.hak_bytes,
        "other": p.other,
        "other_bytes": p.other_bytes,
        "data_left": p.data_left,
        "passed_over": p.notes,
    });
    Ok(out)
}
