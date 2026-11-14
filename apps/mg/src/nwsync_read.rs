//! `mg nwsync-status` and `mg nwsync-pull`: a module's content read from
//! the NWSync server that hands it to players (`[nwsync] source` in its
//! project file, or `--source`), what of it is here (the game's NWSync
//! storage, read only, and Moonglow's own cache), and the download of
//! what isn't into the cache ([`mg_nwsync`]).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use mg_nwsync::fetch::{self, Client, Fetch, Step};
use mg_nwsync::{Cache, GameStorage, Settings, Survey, Wanted, sha1_hex};
use serde_json::json;

use crate::{Cli, Output, path_text};

/// What the two commands are given.
#[derive(Debug, Clone, clap::Args)]
pub(crate) struct Source {
    /// The module, whose project names its server ([nwsync] source in
    /// moonglow.toml); none where --source is given.
    pub module: Option<PathBuf>,
    /// The NWSync server, as nwserver's -nwsyncurl (default: the
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

/// The settings: the project's, with what the command line says over them.
fn settings(s: &Source) -> Result<Settings> {
    let project = match &s.module {
        Some(path) => crate::open_module(path)?.project_file.nwsync_settings(),
        None => None,
    };
    let url = match (&s.source, &project) {
        (Some(u), _) => u.trim().trim_end_matches('/').to_string(),
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
        None => project.as_ref().map_or(Wanted::Latest, |p| p.manifest),
    };
    let cache = s.cache.clone().or_else(|| project.and_then(|p| p.cache));
    Ok(Settings { url, manifest, cache })
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
        None => out.line("in the game's NWSync storage: none (it has none)".to_string()),
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
        Some(o) => format!(
            "origins: {} haks and {} other containers named",
            o.haks().count(),
            o.containers.len() - o.haks().count()
        ),
        None => "origins: none (the server publishes no origins file for it)".into(),
    }
}

/// `mg nwsync-status`.
pub(crate) fn status(cli: &Cli, s: &Source, online: bool) -> Result<Output> {
    let settings = settings(s)?;
    let cache = cache_of(&settings)?;
    let game = game_storage(cli)?;
    let mut out = Output::default();
    let (found, server_latest) = if online {
        let r = fetch::resolve(&Client::new(), &settings, &cache, game.as_ref())
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
    out.line(format!("server: {}", settings.url));
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
        mg_nwsync::ManifestFrom::Cache => "cache",
        mg_nwsync::ManifestFrom::GameStorage => "game_storage",
    }
}

/// `mg nwsync-pull`.
pub(crate) fn pull(cli: &Cli, s: &Source, jobs: usize, sample: Option<usize>) -> Result<Output> {
    let settings = settings(s)?;
    let cache = cache_of(&settings)?;
    let game = game_storage(cli)?;
    pull_with(&Client::new(), &settings, &cache, game.as_ref(), jobs, sample, !cli.json)
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
            Step::Asking => eprintln!("asking {} for its manifest", settings.url),
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
    out.line(format!("server: {}", settings.url));
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
    let left = p.before.missing.len() - d.files;
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
