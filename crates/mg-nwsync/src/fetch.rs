//! A server asked: its latest manifest, a manifest with its side files,
//! and the data files the game's storage and the cache don't have,
//! downloaded into the cache (several at once, each checked against its
//! SHA-1 before it is kept). What was downloaded stays: a download cut off
//! or canceled goes on from there the next time.
//!
//! [`Fetch`] is the seam: `mg` and the window give [`Client`] (feature
//! `http`) or, for a repository folder on this computer (a `file://`
//! source), [`Folder`] ([`for_settings`] picks), the tests answers of
//! their own ([`from_fn`]).
//!
//! A folder's files are read in place, without a server, and go through
//! the same steps as a server's: checked against their SHA-1 and copied
//! into the cache, its `latest/` and `used/` marks kept as for a server.
//! The cache so stays whole on its own: a module opens offline whatever
//! becomes of the folder (a removable or network drive gone, the
//! repository pruned or published again), and a clean-up of either never
//! takes what the other holds. The cost is the disk space of the data
//! copied.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::manifest::{self, Entry};
use crate::{
    Cache, Found, GameStorage, ManifestFrom, Origins, Settings, Sha1, Survey, Wanted, find,
    parse_sha1, sha1_hex, survey_until, unpack_checked,
};

/// The most of a manifest that is read (a large persistent world's, 220,000 resources, is
/// 9 MB).
pub const MANIFEST_LIMIT: u64 = 256 << 20;
/// The most of a side file (`latest`, `.json`, `.origin`) that is read.
pub const SIDE_LIMIT: u64 = 64 << 20;
/// The most of a data file whose size the manifest doesn't say (one the
/// game kept without its file: every size 0) that is read. The largest
/// resource seen on a persistent world's server is 49.8 MB; five times
/// that leaves room, and [`JOBS`] of them at once are 2 GiB of memory at
/// most, not the 8 GiB a gigabyte each would be.
pub const UNKNOWN_SIZE_LIMIT: u64 = 256 << 20;
/// How many data files are downloaded at once, unless told.
pub const JOBS: usize = 8;
/// How many times a data file is asked for before it is given up.
const TRIES: u32 = 3;

/// What makes the requests.
pub trait Fetch: Send + Sync {
    /// `GET` of the address: its body, at most `limit` bytes; `Ok(None)`
    /// where the server has no such file (404).
    fn get(&self, url: &str, limit: u64) -> Result<Option<Vec<u8>>, String>;
}

/// Answers made up by `answer` (a URL's body, none for 404, or why there
/// is none), for tests.
pub fn from_fn(
    answer: impl Fn(&str) -> Result<Option<Vec<u8>>, String> + Send + Sync + 'static,
) -> Arc<dyn Fetch> {
    struct Answers<F>(F);
    impl<F: Fn(&str) -> Result<Option<Vec<u8>>, String> + Send + Sync> Fetch for Answers<F> {
        fn get(&self, url: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
            let body = (self.0)(url)?;
            if body.as_ref().is_some_and(|b| b.len() as u64 > limit) {
                return Err(format!("{url} is larger than it should be"));
            }
            Ok(body)
        }
    }
    Arc::new(Answers(answer))
}

/// A repository folder on this computer, read in place: `base` is the
/// source's `file://` URL as [`at`] builds the files' addresses from it.
#[derive(Debug, Clone)]
pub struct Folder {
    base: String,
    root: std::path::PathBuf,
}

impl Folder {
    /// The folder a `file://` source names; `None` for a server.
    pub fn of(url: &str) -> Option<Folder> {
        let root = crate::local_folder(url)?;
        Some(Folder { base: at(url, ""), root })
    }
}

impl Fetch for Folder {
    fn get(&self, url: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
        let rel =
            url.strip_prefix(&self.base).ok_or_else(|| format!("{url} is not in the folder"))?;
        // (A path of the repository's own, nothing above or beside it.)
        let rel = std::path::Path::new(rel);
        if rel.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            return Err(format!("{url} is not in the folder"));
        }
        if !self.root.is_dir() {
            return Err(format!("the repository folder {} is not there", self.root.display()));
        }
        let path = self.root.join(rel);
        let said = |e: std::io::Error| format!("{}: {e}", path.display());
        let meta = match std::fs::metadata(&path) {
            Ok(m) if m.is_file() => m,
            Ok(_) => return Ok(None),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(said(e)),
        };
        if meta.len() > limit {
            return Err(format!("{} is larger than it should be", path.display()));
        }
        use std::io::Read as _;
        let mut bytes = Vec::with_capacity(meta.len() as usize);
        let file = std::fs::File::open(&path).map_err(said)?;
        // (Read to the limit: a file grown since it was looked at.)
        file.take(limit + 1).read_to_end(&mut bytes).map_err(said)?;
        if bytes.len() as u64 > limit {
            return Err(format!("{} is larger than it should be", path.display()));
        }
        Ok(Some(bytes))
    }
}

/// A file of the repository at `url`.
pub fn at(url: &str, path: &str) -> String {
    format!("{}/{path}", url.trim().trim_end_matches('/'))
}

/// The newest manifest the server names (`<url>/latest`).
pub fn latest(fetch: &dyn Fetch, url: &str) -> Result<Sha1, String> {
    let body = fetch
        .get(&at(url, "latest"), SIDE_LIMIT)?
        .ok_or_else(|| format!("{url} names no latest manifest (it has no /latest)"))?;
    let text = String::from_utf8_lossy(&body);
    parse_sha1(text.trim())
        .ok_or_else(|| format!("{url}/latest is not a manifest's SHA-1: {:?}", text.trim()))
}

/// A manifest from the server into the cache, its SHA-1 checked, with
/// its `.json` and `.origin` where the server has them.
pub fn get_manifest(
    fetch: &dyn Fetch,
    url: &str,
    sha1: &Sha1,
    cache: &Cache,
) -> Result<Vec<Entry>, String> {
    let hex = sha1_hex(sha1);
    let bytes = fetch
        .get(&at(url, &format!("manifests/{hex}")), MANIFEST_LIMIT)?
        .ok_or_else(|| format!("{url} has no manifest {hex}"))?;
    let got = mg_core::sha1::sha1(&bytes);
    if got != *sha1 {
        return Err(format!("manifest {hex} from {url} came with SHA-1 {}", sha1_hex(&got)));
    }
    let entries = manifest::read(&bytes).map_err(|e| format!("manifest {hex}: {e}"))?;
    let keep = |ext: &str, data: &[u8]| {
        cache.put_manifest(sha1, ext, data).map_err(|e| format!("{}: {e}", cache.root().display()))
    };
    // (Its side files first: a manifest kept is one whose origins were
    // asked for.)
    for ext in ["json", "origin"] {
        if let Ok(Some(side)) = fetch.get(&at(url, &format!("manifests/{hex}.{ext}")), SIDE_LIMIT) {
            keep(ext, &side)?;
        }
    }
    keep("", &bytes)?;
    Ok(entries)
}

/// The manifest to read, the server asked first.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub found: Found,
    /// The newest the server named, where it was asked and answered.
    pub server_latest: Option<Sha1>,
    /// What to tell the builder (the server not reached, an older
    /// manifest used).
    pub warnings: Vec<String>,
}

/// The manifest of `settings`, the server asked: its latest (kept as the
/// one to use when it can't be asked, once it is here), or the pinned
/// one, downloaded into the cache where neither the cache nor the game's
/// storage has it.
/// Where the server can't be reached, the newest one here ([`find`]),
/// with a warning; an error where there is none.
pub fn resolve(
    fetch: &dyn Fetch,
    settings: &Settings,
    cache: &Cache,
    game: Option<&GameStorage>,
) -> Result<Resolved, String> {
    let url = &settings.url;
    let wanted = match settings.manifest {
        Wanted::Pinned(sha) => sha,
        Wanted::Latest => match latest(fetch, url) {
            Ok(sha) => sha,
            Err(e) => {
                let found = find(url, Wanted::Latest, Some(cache), game).map_err(|none| {
                    format!("{url} could not be asked for its latest manifest ({e}), and {none}")
                })?;
                let warning = format!(
                    "{url} could not be asked for its latest manifest ({e}): using manifest {} \
                     from {}, which may be older",
                    found.short(),
                    found.from.said()
                );
                return Ok(Resolved { found, server_latest: None, warnings: vec![warning] });
            }
        },
    };
    let server_latest = matches!(settings.manifest, Wanted::Latest).then_some(wanted);
    // (The server's latest is kept as the one to use offline only once it
    // is here: one named and then not given would be kept, and kept from
    // a prune, with nothing here to read it from.)
    let keep_latest = || match server_latest {
        Some(sha) => {
            cache.set_latest(url, &sha).map_err(|e| format!("{}: {e}", cache.root().display()))
        }
        None => Ok(()),
    };
    if let Ok(found) = find(url, Wanted::Pinned(wanted), Some(cache), game) {
        keep_latest()?;
        // (The game's has no origins, and one kept before the server
        // published them has none either: they are asked for, where the
        // server has them now. Publishing them again doesn't change the
        // manifest's SHA-1.)
        if found.origins.is_none() {
            let hex = sha1_hex(&wanted);
            if let Ok(Some(side)) =
                fetch.get(&at(url, &format!("manifests/{hex}.origin")), SIDE_LIMIT)
            {
                let _ = cache.put_manifest(&wanted, "origin", &side);
                let origins = Some(Origins::parse(&String::from_utf8_lossy(&side)));
                return Ok(Resolved {
                    found: Found { origins, ..found },
                    server_latest,
                    warnings: Vec::new(),
                });
            }
        }
        return Ok(Resolved { found, server_latest, warnings: Vec::new() });
    }
    match get_manifest(fetch, url, &wanted, cache) {
        Ok(entries) => {
            keep_latest()?;
            let origins = cache
                .manifest(&wanted, "origin")
                .map(|b| Origins::parse(&String::from_utf8_lossy(&b)));
            let includes_module = crate::source::includes_module(&wanted, Some(cache), game);
            let from = match crate::local_folder(url) {
                Some(_) => ManifestFrom::Folder,
                None => ManifestFrom::Server,
            };
            let at = crate::source::time_of(&wanted, from, Some(cache), game);
            let found = Found { sha1: wanted, entries, from, origins, includes_module, at };
            Ok(Resolved { found, server_latest, warnings: Vec::new() })
        }
        Err(e) if matches!(settings.manifest, Wanted::Pinned(_)) => Err(e),
        Err(e) => {
            // (The server named it and then didn't give it: the newest here.)
            let found = find(url, Wanted::Latest, Some(cache), game).map_err(|_| e.clone())?;
            let warning = format!(
                "{e}: using manifest {} from {}, which may be older",
                found.short(),
                found.from.said()
            );
            Ok(Resolved { found, server_latest, warnings: vec![warning] })
        }
    }
}

/// How a download is done.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// How many data files at once.
    pub jobs: usize,
    /// At most this many data files (a sample); all where `None`.
    pub most: Option<usize>,
}

impl Default for Options {
    fn default() -> Options {
        Options { jobs: JOBS, most: None }
    }
}

/// How far a download is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    /// Data files done (kept or given up), of how many.
    pub files: usize,
    pub of_files: usize,
    /// Their uncompressed bytes, of how many (as the manifest says them).
    pub bytes: u64,
    pub of_bytes: u64,
    /// Bytes received, as the server sent them (compressed).
    pub received: u64,
}

/// What a download did.
#[derive(Debug, Clone, Default)]
pub struct Downloaded {
    /// Data files kept, and the bytes received for them.
    pub files: usize,
    pub received: u64,
    /// Those given up, and why.
    pub failed: Vec<(Sha1, String)>,
    /// Whether it was canceled before the end.
    pub stopped: bool,
}

/// One data file asked for, checked and kept: the bytes received.
fn one(fetch: &dyn Fetch, url: &str, sha: &Sha1, size: u32, cache: &Cache) -> Result<u64, String> {
    let hex = sha1_hex(sha);
    let path = format!("data/sha1/{}/{}/{hex}", &hex[0..2], &hex[2..4]);
    // (Compressed data is no bigger than the bytes and a header, but for
    // data that doesn't compress; a size not known: `UNKNOWN_SIZE_LIMIT`.)
    let limit = if size == 0 { UNKNOWN_SIZE_LIMIT } else { u64::from(size) * 2 + (1 << 20) };
    let stored = fetch
        .get(&at(url, &path), limit)?
        .ok_or_else(|| "the server doesn't have it".to_string())?;
    unpack_checked(&stored, sha, size)?;
    cache.put(sha, &stored).map_err(|e| format!("{}: {e}", cache.root().display()))?;
    Ok(stored.len() as u64)
}

/// Downloads `files` (SHA-1 and size) into the cache, `o.jobs` at a time,
/// each tried a few times before it is given up; `progress` hears of each
/// one done, from the threads; `stop` ends it before the next.
pub fn download(
    fetch: &dyn Fetch,
    url: &str,
    files: &[(Sha1, u32)],
    cache: &Cache,
    o: Options,
    progress: &(dyn Fn(Progress) + Sync),
    stop: &(dyn Fn() -> bool + Sync),
) -> Downloaded {
    let files = &files[..o.most.map_or(files.len(), |n| n.min(files.len()))];
    let of_bytes = files.iter().map(|(_, n)| u64::from(*n)).sum();
    let next = AtomicUsize::new(0);
    let (done, bytes, received, kept) =
        (AtomicUsize::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicUsize::new(0));
    let failed = Mutex::new(Vec::new());
    let stopped = AtomicBool::new(false);
    let work = || {
        loop {
            if stop() {
                stopped.store(true, Ordering::Relaxed);
                return;
            }
            let i = next.fetch_add(1, Ordering::Relaxed);
            let Some((sha, size)) = files.get(i) else { return };
            let mut got = Err(String::new());
            for attempt in 0..TRIES {
                got = one(fetch, url, sha, *size, cache);
                match &got {
                    Ok(_) => break,
                    Err(e) if e == "the server doesn't have it" => break,
                    Err(_) if attempt + 1 < TRIES && !stop() => {
                        std::thread::sleep(Duration::from_millis(500 << attempt));
                    }
                    Err(_) => {}
                }
            }
            match got {
                Ok(n) => {
                    received.fetch_add(n, Ordering::Relaxed);
                    kept.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => failed.lock().unwrap_or_else(|e| e.into_inner()).push((*sha, e)),
            }
            bytes.fetch_add(u64::from(*size), Ordering::Relaxed);
            progress(Progress {
                files: done.fetch_add(1, Ordering::Relaxed) + 1,
                of_files: files.len(),
                bytes: bytes.load(Ordering::Relaxed),
                of_bytes,
                received: received.load(Ordering::Relaxed),
            });
        }
    };
    std::thread::scope(|s| {
        for _ in 0..o.jobs.clamp(1, 64).min(files.len().max(1)) {
            s.spawn(work);
        }
    });
    let mut failed = failed.into_inner().unwrap_or_else(|e| e.into_inner());
    failed.sort();
    Downloaded {
        files: kept.into_inner(),
        received: received.into_inner(),
        failed,
        stopped: stopped.into_inner(),
    }
}

/// What a pull did: the manifest, what was here before, what was
/// downloaded.
#[derive(Debug, Clone)]
pub struct Pulled {
    pub resolved: Resolved,
    pub before: Survey,
    pub downloaded: Downloaded,
}

/// What a pull is doing, for a progress line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Asking the server for its manifest.
    Asking,
    /// Looking at which data files are here.
    Looking,
    Downloading(Progress),
}

/// The manifest of `settings` resolved ([`resolve`]) and the data files
/// neither the game's storage nor the cache has downloaded into the
/// cache ([`download`]). Stopped while it looks, the error is
/// [`crate::CANCELED`]; while it downloads, [`Downloaded::stopped`].
pub fn pull(
    fetch: &dyn Fetch,
    settings: &Settings,
    cache: &Cache,
    game: Option<&GameStorage>,
    o: Options,
    step: &(dyn Fn(Step) + Sync),
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<Pulled, String> {
    step(Step::Asking);
    let resolved = resolve(fetch, settings, cache, game)?;
    step(Step::Looking);
    let before = survey_until(&resolved.found.entries, game, Some(cache), stop)?;
    let progress = |p: Progress| step(Step::Downloading(p));
    let downloaded = if before.missing.is_empty() || stop() {
        Downloaded { stopped: stop(), ..Default::default() }
    } else {
        download(fetch, &settings.url, &before.missing, cache, o, &progress, stop)
    };
    Ok(Pulled { resolved, before, downloaded })
}

#[cfg(feature = "http")]
pub use client::Client;

/// What makes the requests for `settings`: a repository folder's
/// [`Folder`], else the [`Client`].
#[cfg(feature = "http")]
pub fn for_settings(settings: &Settings) -> Result<Arc<dyn Fetch>, String> {
    match Folder::of(&settings.url) {
        Some(f) => Ok(Arc::new(f)),
        None => Ok(Arc::new(Client::new())),
    }
}

#[cfg(feature = "http")]
mod client {
    //! What is sent is the request alone: `GET` of the address with
    //! `User-Agent: Moonglow/<version>`. No identifier, cookie or token;
    //! a proxy is used where the environment names one (`HTTPS_PROXY`).

    use std::time::Duration;

    use super::Fetch;

    const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
    const READ_TIMEOUT: Duration = Duration::from_secs(30);

    /// The requests, with ureq (rustls, its own certificates). A server
    /// may be HTTP or HTTPS, as the game's NWSync allows.
    #[derive(Debug, Clone)]
    pub struct Client {
        agent: ureq::Agent,
    }

    impl Client {
        pub fn new() -> Client {
            let agent = ureq::Agent::config_builder()
                .user_agent(format!("Moonglow/{}", mg_core::release::version()))
                .max_redirects(5)
                .timeout_connect(Some(CONNECT_TIMEOUT))
                .timeout_recv_response(Some(READ_TIMEOUT))
                .timeout_per_read(Some(READ_TIMEOUT))
                .build()
                .into();
            Client { agent }
        }
    }

    impl Default for Client {
        fn default() -> Client {
            Client::new()
        }
    }

    impl Fetch for Client {
        fn get(&self, url: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
            let mut answer = match self.agent.get(url).call() {
                Ok(a) => a,
                Err(ureq::Error::StatusCode(404 | 410)) => return Ok(None),
                Err(ureq::Error::StatusCode(code)) => {
                    return Err(format!("the server answered {code}"));
                }
                Err(ureq::Error::HostNotFound) => {
                    return Err("the server's name was not found (not connected?)".into());
                }
                Err(ureq::Error::Timeout(_)) => {
                    return Err("the server didn't answer in time".into());
                }
                Err(e) => return Err(e.to_string()),
            };
            if answer.body().content_length().is_some_and(|n| n > limit) {
                return Err(format!("{url} is larger than it should be"));
            }
            answer
                .body_mut()
                .with_config()
                .limit(limit)
                .read_to_vec()
                .map(Some)
                .map_err(|e| format!("the download broke off: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A data file of a size not known (a manifest the game kept without
    /// its file) is read to a limit a few of at once can hold in memory.
    #[test]
    fn a_size_not_known_is_read_to_a_bounded_limit() {
        let limits = Arc::new(Mutex::new(Vec::new()));
        let seen = limits.clone();
        struct Limits(Arc<Mutex<Vec<u64>>>);
        impl Fetch for Limits {
            fn get(&self, _: &str, limit: u64) -> Result<Option<Vec<u8>>, String> {
                self.0.lock().unwrap().push(limit);
                Ok(None)
            }
        }
        let cache = Cache::new(mg_testkit::scratch_dir("nwsync-fetch-limit"));
        let sha = mg_core::sha1::sha1(b"x");
        let d = download(
            &Limits(seen),
            "https://a.example",
            &[(sha, 0), (mg_core::sha1::sha1(b"y"), 1000)],
            &cache,
            Options::default(),
            &|_| {},
            &|| false,
        );
        assert_eq!(d.failed.len(), 2);
        let mut limits = limits.lock().unwrap().clone();
        limits.sort();
        assert_eq!(limits, [1000 * 2 + (1 << 20), UNKNOWN_SIZE_LIMIT]);
        assert!(limits.iter().all(|l| *l <= 256 << 20), "{limits:?}");
    }
}
