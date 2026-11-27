//! NWSync content read: a made-up game storage (the client's schema,
//! `mg_nwsync::fixtures`) read without being changed, a made-up server
//! downloaded from, the manifest picked offline, and the layer the
//! resource manager reads through.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;
use std::sync::{Arc, Mutex};

use mg_core::ResType;
use mg_core::sha1::sha1;
use mg_erf::compressedbuf::{MAGIC_NSYC, compress, store};
use mg_nwsync::fetch::{self, Fetch, Options, from_fn};
use mg_nwsync::fixtures::{Registered, write_storage};
use mg_nwsync::{
    Cache, GameStorage, ManifestFrom, NwSyncContainer, Settings, Sha1, Wanted, find, manifest,
    sha1_hex, survey,
};
use mg_resman::{Container, LayerClass, ResKey, ResMan, priority};

fn key(name: &str, restype: ResType) -> ResKey {
    ResKey::parse(name, restype).unwrap()
}

/// `data` in a zlib stream, as neverwinter.nim (`-d:zlib`) and older
/// NWSync servers store it.
fn zlib(data: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(data).unwrap();
    let mut out = Vec::new();
    out.extend_from_slice(b"NSYC");
    for word in [3u32, 1, data.len() as u32, 1] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&z.finish().unwrap());
    out
}

/// A module's content: resources by name, with their bytes.
struct Content {
    resources: Vec<(ResKey, Vec<u8>)>,
}

impl Content {
    fn new() -> Content {
        let text = |s: &str| s.repeat(40).into_bytes();
        Content {
            resources: vec![
                (key("appearance", ResType::TWODA), text("2DA V2.0 appearance ")),
                (key("pw_head001", ResType::MDL), text("model head ")),
                (key("pw_head002", ResType::MDL), text("model head ")),
                (key("pw_tex", ResType::TGA), text("texture ")),
                (key("world", ResType::TLK), text("TLK V3.0 ")),
                (key("pw_sound", ResType::WAV), text("RIFF sound ")),
            ],
        }
    }

    fn entries(&self) -> Vec<manifest::Entry> {
        self.resources.iter().map(|(k, d)| (*k, sha1(d), d.len() as u32)).collect()
    }

    fn manifest(&self) -> (Sha1, Vec<u8>) {
        let bytes = manifest::write(&self.entries());
        (sha1(&bytes), bytes)
    }

    fn bytes(&self, name: &str, restype: ResType) -> &[u8] {
        &self.resources.iter().find(|(k, _)| *k == key(name, restype)).unwrap().1
    }
}

/// A made-up server of `content`: its files by path, each data file
/// stored with zstd, zlib or none in turn. What it was asked for is
/// written down.
struct Server {
    files: HashMap<String, Vec<u8>>,
    asked: Arc<Mutex<Vec<String>>>,
}

const URL: &str = "https://nwsync.example.org";

impl Server {
    fn of(content: &Content) -> Server {
        let (sha, bytes) = content.manifest();
        let hex = sha1_hex(&sha);
        let mut files = HashMap::new();
        files.insert("latest".to_string(), format!("{hex}\n").into_bytes());
        files.insert(format!("manifests/{hex}"), bytes);
        files.insert(format!("manifests/{hex}.json"), b"{\"version\": 3}".to_vec());
        for (i, (_, data)) in content.resources.iter().enumerate() {
            let h = sha1_hex(&sha1(data));
            let stored = match i % 3 {
                0 => compress(data, MAGIC_NSYC).unwrap(),
                1 => zlib(data),
                _ => store(data, MAGIC_NSYC),
            };
            files.insert(format!("data/sha1/{}/{}/{h}", &h[0..2], &h[2..4]), stored);
        }
        Server { files, asked: Default::default() }
    }

    fn fetch(&self) -> Arc<dyn Fetch> {
        let files = self.files.clone();
        let asked = self.asked.clone();
        from_fn(move |url| {
            let path = url.strip_prefix(&format!("{URL}/")).ok_or("another server")?;
            asked.lock().unwrap().push(path.to_string());
            Ok(files.get(path).cloned())
        })
    }

    fn data_asked(&self) -> usize {
        self.asked.lock().unwrap().iter().filter(|p| p.starts_with("data/")).count()
    }
}

/// A server that can't be reached.
fn offline() -> Arc<dyn Fetch> {
    from_fn(|_| Err("not connected".to_string()))
}

fn settings(manifest: Wanted) -> Settings {
    Settings { url: URL.into(), manifest, cache: None }
}

/// A game storage that has registered `content`'s manifest (from `url`,
/// at `created`) and holds the data of `held` of its resources, over two
/// shards, stored as the client stores it (zstd) or raw.
fn game_storage(
    dir: &Path,
    content: &Content,
    url: &str,
    created: i64,
    held: &[&str],
    cached: bool,
) -> GameStorage {
    let (sha, bytes) = content.manifest();
    let resources = content.entries().iter().map(|(k, s, _)| (*k, *s)).collect();
    let mut shards = vec![Vec::new(), Vec::new()];
    for (i, (k, data)) in content.resources.iter().enumerate() {
        if !held.contains(&k.resref.to_string_lossy().as_str()) {
            continue;
        }
        let stored = if i % 2 == 0 { compress(data, MAGIC_NSYC).unwrap() } else { data.clone() };
        let shard = &mut shards[i % 2];
        if !shard.iter().any(|(s, _)| *s == sha1(data)) {
            shard.push((sha1(data), stored));
        }
    }
    let registered = Registered {
        sha1: sha,
        origin_url: url.into(),
        created_at: created,
        last_used_at: created,
        resources,
        cached: cached.then_some(bytes),
    };
    write_storage(dir, &[registered], &shards);
    GameStorage::open(dir).unwrap().unwrap()
}

/// Each file of a folder with its bytes and time, to tell it unchanged.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>, std::time::SystemTime)> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| {
            let p = e.path();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
                e.metadata().unwrap().modified().unwrap(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The game's storage is read, every shard, raw and compressed data, its
/// manifests by server; and is left exactly as it was: no file changed,
/// none added (no journal, no lock file).
#[test]
fn the_game_storage_is_read_and_left_as_it_was() {
    let dir = mg_testkit::scratch_dir("nwsync-game-read");
    let content = Content::new();
    let all = ["appearance", "pw_head001", "pw_head002", "pw_tex", "world", "pw_sound"];
    let g = game_storage(&dir, &content, URL, 100, &all, false);
    let before = snapshot(&dir);
    assert_eq!(g.shards(), 2);
    let (sha, _) = content.manifest();
    let listed = g.manifests().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!((listed[0].sha1, listed[0].origin_url.as_str()), (sha, URL));
    // (From its list of resources: no sizes.)
    let entries = g.manifest_entries(&sha).unwrap().unwrap();
    assert_eq!(entries.len(), 6);
    assert!(entries.iter().all(|(_, _, size)| *size == 0));
    assert_eq!(g.manifest_entries(&sha1(b"another")).unwrap(), None);
    // (Five data files: the two heads are the same bytes.)
    assert_eq!(g.hashes().unwrap().len(), 5);
    for (k, data) in &content.resources {
        let stored = g.read(&sha1(data)).unwrap().unwrap_or_else(|| panic!("{k:?}"));
        assert_eq!(&mg_nwsync::unpack(&stored).unwrap(), data);
        assert!(g.contains(&sha1(data)));
    }
    assert_eq!(g.read(&sha1(b"nothing")).unwrap(), None);
    drop(g);
    assert_eq!(snapshot(&dir), before, "the game's storage was changed");
    // (No storage: none, not an error.)
    assert!(GameStorage::open(&dir.join("nowhere")).unwrap().is_none());
}

/// The game writing while Moonglow reads: a write under way doesn't stop
/// a read (it reads what was committed), and what the game commits is
/// read after.
#[test]
fn the_game_writing_meanwhile_doesnt_stop_a_read() {
    let dir = mg_testkit::scratch_dir("nwsync-game-busy");
    let content = Content::new();
    let g = game_storage(&dir, &content, URL, 100, &["appearance"], true);
    let game = rusqlite::Connection::open(dir.join("nwsyncdata_0.sqlite3")).unwrap();
    game.execute_batch("begin immediate").unwrap();
    let late = b"written by the game".to_vec();
    game.execute(
        "insert into resrefs values (?1, ?2, ?3, 0)",
        rusqlite::params![sha1_hex(&sha1(&late)), late, late.len() as i64],
    )
    .unwrap();
    let appearance = sha1(content.bytes("appearance", ResType::TWODA));
    assert!(g.read(&appearance).unwrap().is_some());
    assert_eq!(g.read(&sha1(&late)).unwrap(), None);
    game.execute_batch("commit").unwrap();
    assert_eq!(g.read(&sha1(&late)).unwrap(), Some(late));
    // (The manifest's own bytes, where the game kept them: with sizes.)
    let (sha, _) = content.manifest();
    let entries = g.manifest_entries(&sha).unwrap().unwrap();
    assert!(entries.iter().all(|(_, _, size)| *size > 0));
}

/// Pulling: the latest manifest asked for and kept, with its side files;
/// only the data files neither the game's storage nor the cache has
/// downloaded (zstd, zlib and stored ones), each checked; a second pull
/// downloads nothing.
#[test]
fn a_pull_downloads_what_is_missing_alone() {
    let dir = mg_testkit::scratch_dir("nwsync-pull");
    let content = Content::new();
    let server = Server::of(&content);
    let g = game_storage(&dir.join("game"), &content, URL, 100, &["appearance", "pw_tex"], false);
    let cache = Cache::new(dir.join("cache"));
    let (sha, _) = content.manifest();
    let steps = Mutex::new(Vec::new());
    let pulled = fetch::pull(
        &*server.fetch(),
        &settings(Wanted::Latest),
        &cache,
        Some(&g),
        Options::default(),
        &|s| steps.lock().unwrap().push(s),
        &|| false,
    )
    .unwrap();
    assert_eq!(pulled.resolved.found.sha1, sha);
    assert_eq!(pulled.resolved.server_latest, Some(sha));
    assert!(pulled.resolved.warnings.is_empty());
    // (The game had registered it: its list read, not downloaded.)
    assert_eq!(pulled.resolved.found.from, ManifestFrom::GameStorage);
    assert_eq!((pulled.before.data_files, pulled.before.in_game), (5, 2));
    assert_eq!(pulled.before.missing.len(), 3);
    assert_eq!(pulled.downloaded.files, 3);
    assert!(pulled.downloaded.failed.is_empty(), "{:?}", pulled.downloaded.failed);
    assert_eq!(server.data_asked(), 3);
    let steps = steps.into_inner().unwrap();
    assert_eq!(steps[..2], [fetch::Step::Asking, fetch::Step::Looking]);
    // (Each download's step is said from its thread: the one that says
    // "3 of 3" is not always heard last.)
    let most = steps.iter().filter_map(|s| match s {
        fetch::Step::Downloading(p) => Some(p.files),
        _ => None,
    });
    assert_eq!(most.max(), Some(3));
    assert!(steps[2..].iter().all(|s| matches!(s, fetch::Step::Downloading(_))));
    assert_eq!(cache.latest(URL).map(|l| l.0), Some(sha));
    // Everything is here: from the game, the rest from the cache.
    let s = survey(&pulled.resolved.found.entries, Some(&g), Some(&cache)).unwrap();
    assert_eq!((s.in_game, s.in_cache, s.missing.len()), (2, 3, 0));
    // Again: nothing to download.
    let again = fetch::pull(
        &*server.fetch(),
        &settings(Wanted::Latest),
        &cache,
        Some(&g),
        Options::default(),
        &|_| {},
        &|| false,
    )
    .unwrap();
    assert_eq!(again.downloaded.files, 0);
    assert_eq!(server.data_asked(), 3);

    // Without the game's storage, into an empty cache: the manifest
    // downloaded (and its side files), the six resources' five files.
    let cache = Cache::new(dir.join("cache2"));
    let server = Server::of(&content);
    let pulled = fetch::pull(
        &*server.fetch(),
        &settings(Wanted::Latest),
        &cache,
        None,
        Options { jobs: 2, most: None },
        &|_| {},
        &|| false,
    )
    .unwrap();
    assert_eq!(pulled.resolved.found.from, ManifestFrom::Server);
    assert_eq!(pulled.downloaded.files, 5);
    assert!(cache.manifest(&sha, "").is_some() && cache.manifest(&sha, "json").is_some());
    // (The server had no origins file: none, and no error.)
    assert!(cache.manifest(&sha, "origin").is_none() && pulled.resolved.found.origins.is_none());
}

/// A data file that isn't what its name says, or that the server doesn't
/// have, is given up and said, the rest kept; a sample (`most`) asks for
/// that many; Cancel stops before the next.
#[test]
fn bad_and_missing_data_is_given_up_and_said() {
    let dir = mg_testkit::scratch_dir("nwsync-bad-data");
    let content = Content::new();
    let mut server = Server::of(&content);
    let tex = sha1_hex(&sha1(content.bytes("pw_tex", ResType::TGA)));
    let sound = sha1_hex(&sha1(content.bytes("pw_sound", ResType::WAV)));
    let path = |h: &str| format!("data/sha1/{}/{}/{h}", &h[0..2], &h[2..4]);
    server.files.insert(path(&tex), compress(b"something else", MAGIC_NSYC).unwrap());
    server.files.remove(&path(&sound));
    let cache = Cache::new(dir.join("cache"));
    let pulled = fetch::pull(
        &*server.fetch(),
        &settings(Wanted::Latest),
        &cache,
        None,
        Options::default(),
        &|_| {},
        &|| false,
    )
    .unwrap();
    assert_eq!(pulled.downloaded.files, 3);
    let failed: HashMap<String, String> =
        pulled.downloaded.failed.iter().map(|(s, e)| (sha1_hex(s), e.clone())).collect();
    assert!(failed[&tex].contains("manifest says"), "{failed:?}");
    assert_eq!(failed[&sound], "the server doesn't have it");
    assert!(!cache.contains(&sha1(content.bytes("pw_tex", ResType::TGA))));
    // (A file that isn't what it says is asked for again before it is
    // given up; one the server doesn't have, once.)
    let asked = server.asked.lock().unwrap().clone();
    assert_eq!(asked.iter().filter(|p| **p == path(&tex)).count(), 3);
    assert_eq!(asked.iter().filter(|p| **p == path(&sound)).count(), 1);

    let cache = Cache::new(dir.join("sample"));
    let server = Server::of(&content);
    let sample = Options { jobs: 4, most: Some(2) };
    let pulled = fetch::pull(
        &*server.fetch(),
        &settings(Wanted::Latest),
        &cache,
        None,
        sample,
        &|_| {},
        &|| false,
    )
    .unwrap();
    assert_eq!((pulled.downloaded.files, server.data_asked()), (2, 2));
    let stopped = fetch::pull(
        &*server.fetch(),
        &settings(Wanted::Latest),
        &cache,
        None,
        Options::default(),
        &|_| {},
        &|| true,
    )
    .unwrap();
    assert!(stopped.downloaded.stopped);
    assert_eq!(server.data_asked(), 2);
}

/// The server out of reach: the newest manifest here, with a warning
/// that says so (the one Moonglow last downloaded, or the game's, the
/// newer of the two); none here is an error that says what to do.
#[test]
fn offline_the_newest_manifest_here_is_used() {
    let dir = mg_testkit::scratch_dir("nwsync-offline");
    let content = Content::new();
    let (sha, _) = content.manifest();
    let cache = Cache::new(dir.join("cache"));
    let none = fetch::resolve(&*offline(), &settings(Wanted::Latest), &cache, None).unwrap_err();
    assert!(none.contains("could not be asked") && none.contains("mg nwsync-pull"), "{none}");

    // The game's storage has it (registered from the same server, written
    // another way).
    let g =
        game_storage(&dir.join("game"), &content, "HTTPS://NWSync.Example.org/", 100, &[], true);
    let r = fetch::resolve(&*offline(), &settings(Wanted::Latest), &cache, Some(&g)).unwrap();
    assert_eq!((r.found.sha1, r.found.from), (sha, ManifestFrom::GameStorage));
    assert!(r.warnings[0].contains("not connected") && r.warnings[0].contains("may be older"));
    // (Another server's isn't taken for it.)
    let other = Settings { url: "https://other.example".into(), ..settings(Wanted::Latest) };
    assert!(fetch::resolve(&*offline(), &other, &cache, Some(&g)).is_err());

    // Moonglow downloaded a newer one since: that one.
    let mut newer = Content::new();
    newer.resources.push((key("pw_new", ResType::MDL), b"new model".to_vec()));
    let (newer_sha, _) = newer.manifest();
    let server = Server::of(&newer);
    fetch::resolve(&*server.fetch(), &settings(Wanted::Latest), &cache, Some(&g)).unwrap();
    let r = fetch::resolve(&*offline(), &settings(Wanted::Latest), &cache, Some(&g)).unwrap();
    assert_eq!((r.found.sha1, r.found.from), (newer_sha, ManifestFrom::Cache));
    // Without asking at all ([`find`]): the same.
    let f = find(URL, Wanted::Latest, Some(&cache), Some(&g)).unwrap();
    assert_eq!(f.sha1, newer_sha);
    // A pinned one is that one, from wherever it is.
    let f = find(URL, Wanted::Pinned(sha), Some(&cache), Some(&g)).unwrap();
    assert_eq!((f.sha1, f.from), (sha, ManifestFrom::GameStorage));
    let e = find(URL, Wanted::Pinned(sha1(b"no such")), Some(&cache), Some(&g)).unwrap_err();
    assert!(e.contains("not here yet"), "{e}");
    let e = fetch::resolve(&*offline(), &settings(Wanted::Pinned(sha1(b"no"))), &cache, None)
        .unwrap_err();
    assert!(e.contains("not connected"), "{e}");
}

/// When a manifest came here ([`mg_nwsync::Found::at`]): the game's
/// `created_at` for one read from its storage, the time Moonglow's cache
/// kept it for one read from there; a use ([`Cache::touch`]) leaves that
/// time as it was (it is what a hak on disk is compared with).
#[test]
fn a_manifest_s_time_is_when_it_came_here() {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let dir = mg_testkit::scratch_dir("nwsync-manifest-time");
    let content = Content::new();
    let (sha, bytes) = content.manifest();
    let g = game_storage(&dir.join("game"), &content, URL, 1_700_000_000, &[], true);
    let f = find(URL, Wanted::Pinned(sha), None, Some(&g)).unwrap();
    assert_eq!(f.from, ManifestFrom::GameStorage);
    assert_eq!(f.at, Some(UNIX_EPOCH + Duration::from_secs(1_700_000_000)));
    let cache = Cache::new(dir.join("cache"));
    cache.put_manifest(&sha, "", &bytes).unwrap();
    let kept = SystemTime::now() - Duration::from_secs(3600);
    let file = dir.join("cache/manifests").join(sha1_hex(&sha));
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(kept).unwrap();
    cache.touch(&sha);
    assert_eq!(cache.manifest_time(&sha), Some(kept));
    let f = find(URL, Wanted::Pinned(sha), Some(&cache), Some(&g)).unwrap();
    assert_eq!((f.from, f.at), (ManifestFrom::Cache, Some(kept)));
    assert!(cache.used_path(&sha).is_file());
}

/// The server's origins file is kept with its manifest and read back,
/// and asked for where the game's storage had the manifest.
#[test]
fn origins_come_with_the_manifest() {
    let dir = mg_testkit::scratch_dir("nwsync-origins");
    let content = Content::new();
    let (sha, _) = content.manifest();
    let mut server = Server::of(&content);
    let text = "Erf:/srv/hak/pw_parts.hak\n\tpw_head001.mdl\nResFile:/srv/tlk/world.tlk\n\
                \tworld.tlk\n";
    server.files.insert(format!("manifests/{}.origin", sha1_hex(&sha)), text.as_bytes().to_vec());
    let cache = Cache::new(dir.join("cache"));
    let r = fetch::resolve(&*server.fetch(), &settings(Wanted::Latest), &cache, None).unwrap();
    let origins = r.found.origins.unwrap();
    assert!(origins.hak("pw_parts").is_some());
    let f = find(URL, Wanted::Latest, Some(&cache), None).unwrap();
    assert_eq!(f.origins, Some(origins));

    let cache = Cache::new(dir.join("cache2"));
    let g = game_storage(&dir.join("game"), &content, URL, 100, &[], false);
    let r = fetch::resolve(&*server.fetch(), &settings(Wanted::Latest), &cache, Some(&g)).unwrap();
    assert_eq!(r.found.from, ManifestFrom::GameStorage);
    assert!(r.found.origins.is_some_and(|o| o.hak("pw_parts").is_some()));
}

/// The layer in a resource manager, at NWSync's priority: above a hak,
/// below the development folder; its resources read from the game's
/// storage, else the cache; one not downloaded is listed (the manifest's
/// copy wins) but its read says why it fails; after a download it reads.
#[test]
fn the_layer_reads_from_the_game_then_the_cache() {
    let dir = mg_testkit::scratch_dir("nwsync-layer");
    let content = Content::new();
    let (sha, _) = content.manifest();
    let g = Arc::new(game_storage(&dir.join("game"), &content, URL, 100, &["appearance"], true));
    let cache = Arc::new(Cache::new(dir.join("cache")));
    let tex = content.bytes("pw_tex", ResType::TGA);
    cache.put(&sha1(tex), &zlib(tex)).unwrap();
    let found = find(URL, Wanted::Latest, Some(&cache), Some(&g)).unwrap();
    let layer = NwSyncContainer::new(sha, &found.entries, Some(g.clone()), Some(cache.clone()));
    assert_eq!(layer.len(), 6);
    let mut rm = ResMan::new();
    let mut hak = mg_resman::MemContainer::new();
    hak.insert(key("appearance", ResType::TWODA), b"the hak's".to_vec());
    hak.insert(key("only_in_hak", ResType::TWODA), b"the hak's".to_vec());
    rm.add(priority::HAK_USER, "hak:pw_2da", LayerClass::Erf, hak);
    rm.add(priority::NWSYNC, mg_nwsync::label(&sha), LayerClass::NwSync, layer);
    assert_eq!(rm.layers()[0].label, format!("nwsync:{}", sha1_hex(&sha)));
    let appearance = key("appearance", ResType::TWODA);
    assert_eq!(&*rm.get(&appearance).unwrap(), content.bytes("appearance", ResType::TWODA));
    assert_eq!(&*rm.get(&key("only_in_hak", ResType::TWODA)).unwrap(), b"the hak's");
    assert_eq!(&*rm.get(&key("pw_tex", ResType::TGA)).unwrap(), tex);
    // Not downloaded: the manifest's, which can't be read yet.
    let head = key("pw_head001", ResType::MDL);
    assert_eq!(rm.origin(&head), Some(mg_nwsync::label(&sha).as_str()));
    let e = rm.get(&head).unwrap_err().to_string();
    assert!(e.contains("not downloaded yet"), "{e}");
    let data = content.bytes("pw_head001", ResType::MDL);
    cache.put(&sha1(data), &compress(data, MAGIC_NSYC).unwrap()).unwrap();
    assert_eq!(&*rm.get(&head).unwrap(), data);
    // The talk table is one of its resources.
    assert!(rm.contains(&key("world", ResType::TLK)));
    // A data file of the wrong size is an error, not the wrong bytes.
    let sound = content.bytes("pw_sound", ResType::WAV);
    cache.put(&sha1(sound), &store(b"short", MAGIC_NSYC)).unwrap();
    let e = rm.get(&key("pw_sound", ResType::WAV)).unwrap_err().to_string();
    assert!(e.contains("the manifest says"), "{e}");
    assert!(cache.contains(&sha1(sound)));
    // One cut off (a write a crash broke): taken away, for the next pull.
    let packed = compress(sound, MAGIC_NSYC).unwrap();
    cache.put(&sha1(sound), &packed[..packed.len() - 3]).unwrap();
    let e = rm.get(&key("pw_sound", ResType::WAV)).unwrap_err().to_string();
    assert!(e.contains("damaged") && e.contains("removed"), "{e}");
    assert!(!cache.contains(&sha1(sound)));
}

/// The client with a server of its own on this computer: a file, a
/// file the server hasn't (none, not an error), one over the limit.
#[test]
fn the_client_downloads_over_http() {
    use std::io::{BufRead, BufReader};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        for _ in 0..3 {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut first = String::new();
            reader.read_line(&mut first).unwrap();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let mut stream = stream;
            let answer = if first.starts_with("GET /latest ") {
                "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello".to_string()
            } else if first.starts_with("GET /big ") {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: 4096\r\nConnection: close\r\n\r\n{}",
                    "x".repeat(4096)
                )
            } else {
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string()
            };
            stream.write_all(answer.as_bytes()).unwrap();
        }
    });
    let c = fetch::Client::new();
    let base = format!("http://127.0.0.1:{port}");
    assert_eq!(c.get(&format!("{base}/latest"), 100).unwrap().as_deref(), Some(&b"hello"[..]));
    assert_eq!(c.get(&format!("{base}/manifests/none"), 100).unwrap(), None);
    assert!(c.get(&format!("{base}/big"), 100).is_err());
    server.join().unwrap();
}

/// The haks of `content` as an origins file names them: `one` with the
/// models, `two` with the rest but the talk table.
fn origins_of() -> mg_nwsync::Origins {
    mg_nwsync::Origins::parse(
        "Erf:/srv/hak/one.hak\n\tpw_head001.mdl\n\tpw_head002.mdl\nErf:/srv/hak/two.hak\n\t\
         appearance.2da\n\tpw_tex.tga\n\tpw_sound.wav\nResFile:/srv/tlk/world.tlk\n\t\
         world.tlk\n",
    )
}

/// Haks written from the game's storage and the cache: each with what the
/// origins file lists, read back byte for byte; the same bytes when
/// written again into another cache; not written again when there; and
/// the game's storage left as it was.
#[test]
fn haks_are_written_from_the_game_and_the_cache() {
    use mg_nwsync::haks::{self, Store};
    let dir = mg_testkit::scratch_dir("nwsync-haks-written");
    let content = Content::new();
    // (The game holds the models and the texture; the cache the rest.)
    let game = game_storage(
        &dir.join("game"),
        &content,
        URL,
        1,
        &["pw_head001", "pw_head002", "pw_tex"],
        true,
    );
    let before = snapshot(&dir.join("game"));
    let cache = Cache::new(dir.join("cache"));
    let other = Cache::new(dir.join("other"));
    for (_, data) in &content.resources {
        if !game.contains(&sha1(data)) {
            cache.put(&sha1(data), &compress(data, MAGIC_NSYC).unwrap()).unwrap();
            other.put(&sha1(data), &store(data, MAGIC_NSYC)).unwrap();
        }
    }
    let entries = content.entries();
    let names = vec!["One".to_string(), "two".to_string(), "mine".to_string()];
    let plan = haks::from_origins(&origins_of(), &entries, &names).unwrap();
    let store = Store { game: Some(&game), cache: &cache };
    let heard = Mutex::new(Vec::new());
    let progress = |p: haks::Progress| heard.lock().unwrap().push(p);
    let written = haks::write_all(&plan, store, 2, &progress, &|| false).unwrap();
    assert_eq!(written.haks.len(), 2);
    assert_eq!(written.written(), (2, plan.bytes()));
    let last = *heard.lock().unwrap().iter().max_by_key(|p| (p.haks, p.bytes)).unwrap();
    assert_eq!(
        (last.haks, last.of_haks, last.bytes, last.of_bytes),
        (2, 2, plan.bytes(), plan.bytes())
    );
    for (h, p) in written.haks.iter().zip(&plan.haks) {
        assert_eq!(h.name, p.name);
        assert_eq!(h.path, cache.hak_path(&p.key()));
        let bytes = std::fs::read(&h.path).unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        assert_eq!(&erf.file_type, b"HAK ");
        assert_eq!((erf.build_year, erf.build_day), (0, 0));
        assert_eq!(erf.entries.len(), p.resources.len());
        for (k, _, _) in &p.resources {
            let got = erf.get(&k.resref, k.restype).unwrap().unwrap();
            assert_eq!(&*got, content.bytes(&k.resref.to_string_lossy(), k.restype));
        }
    }
    assert!(erf_names(&written.haks[0].path).iter().all(|n| n.ends_with(".mdl")));
    // (No part file left.)
    let left: Vec<_> = std::fs::read_dir(dir.join("cache/haks")).unwrap().flatten().collect();
    assert_eq!(left.len(), 2);

    // Written again elsewhere (from data stored otherwise): the same
    // bytes. And not again here.
    let again =
        haks::write_all(&plan, Store { game: Some(&game), cache: &other }, 1, &|_| {}, &|| false);
    for h in &again.unwrap().haks {
        let first = written.haks.iter().find(|w| w.name == h.name).unwrap();
        assert_eq!(std::fs::read(&h.path).unwrap(), std::fs::read(&first.path).unwrap());
    }
    let modified = std::fs::metadata(&written.haks[0].path).unwrap().modified().unwrap();
    let third = haks::write_all(&plan, store, 2, &|_| {}, &|| false).unwrap();
    assert_eq!(third.written(), (0, 0));
    assert_eq!(std::fs::metadata(&third.haks[0].path).unwrap().modified().unwrap(), modified);
    assert_eq!(snapshot(&dir.join("game")), before);
}

fn erf_names(path: &Path) -> Vec<String> {
    let bytes = std::fs::read(path).unwrap();
    let erf = mg_erf::Erf::read(&bytes).unwrap();
    erf.entries.iter().map(|e| e.filename()).collect()
}

/// A newer manifest that changes one hak's resource: that hak is written
/// anew, the other is the file written for the first manifest; each
/// manifest's index names its own.
#[test]
fn an_unchanged_hak_is_kept_across_manifests() {
    use mg_nwsync::haks::{self, Index, Store};
    let dir = mg_testkit::scratch_dir("nwsync-haks-reused");
    let cache = Cache::new(dir.join("cache"));
    let mut content = Content::new();
    let put = |c: &Content| {
        for (_, data) in &c.resources {
            cache.put(&sha1(data), &compress(data, MAGIC_NSYC).unwrap()).unwrap();
        }
    };
    put(&content);
    let names = vec!["one".to_string(), "two".to_string()];
    let store = Store { game: None, cache: &cache };
    let first = haks::from_origins(&origins_of(), &content.entries(), &names).unwrap();
    let a = haks::write_all(&first, store, 2, &|_| {}, &|| false).unwrap();
    let (m1, _) = content.manifest();
    let index = |plan: &haks::Plan| Index {
        haks: plan.haks.iter().map(|h| (h.name.clone(), h.key())).collect(),
        files: vec![],
    };
    cache.put_written(&m1, index(&first)).unwrap();

    content.resources[3].1 = b"another texture".repeat(30);
    put(&content);
    let second = haks::from_origins(&origins_of(), &content.entries(), &names).unwrap();
    let b = haks::write_all(&second, store, 2, &|_| {}, &|| false).unwrap();
    let (m2, _) = content.manifest();
    cache.put_written(&m2, index(&second)).unwrap();
    assert_eq!(b.written(), (1, second.haks[1].bytes()));
    assert_eq!(a.haks[0].path, b.haks[0].path);
    assert!(!b.haks[0].written && b.haks[1].written);
    assert_ne!(a.haks[1].path, b.haks[1].path);
    assert_eq!(cache.written(&m1).haks[1].1, first.haks[1].key());
    assert_eq!(cache.written(&m2).haks[1].1, second.haks[1].key());
}

/// Data neither the game nor the cache has: an error before anything is
/// written, saying how to download it.
#[test]
fn haks_wait_for_their_data() {
    use mg_nwsync::haks::{self, Store};
    let dir = mg_testkit::scratch_dir("nwsync-haks-missing");
    let content = Content::new();
    let cache = Cache::new(dir.join("cache"));
    let plan = haks::from_origins(&origins_of(), &content.entries(), &["one".into()]).unwrap();
    let e = haks::write_all(&plan, Store { game: None, cache: &cache }, 1, &|_| {}, &|| false)
        .unwrap_err();
    // (The two models are the same bytes: one data file.)
    assert!(e.contains("1 data files") && e.contains("mg nwsync-pull"), "{e}");
    assert!(!dir.join("cache/haks").exists());
}

/// The cache pruned: the kept manifests' data and haks stay (the module's,
/// the newest a server named, the last used), the others go; a dry run
/// removes nothing; what was just changed is left alone; a kept manifest
/// whose resources can't be read keeps every data file.
#[test]
fn the_cache_is_pruned_of_what_no_kept_manifest_needs() {
    use mg_nwsync::haks::{self, Index, Store};
    use mg_nwsync::prune::{Keep, prune};
    use std::time::Duration;
    let dir = mg_testkit::scratch_dir("nwsync-cache-prune");
    let cache = Cache::new(dir.join("cache"));
    let store = Store { game: None, cache: &cache };
    // Three manifests: each with one resource of its own, and the rest
    // shared.
    let mut manifests = Vec::new();
    for n in 0..3 {
        let mut c = Content::new();
        c.resources[3].1 = format!("texture {n} ").repeat(30).into_bytes();
        let (sha, bytes) = c.manifest();
        cache.put_manifest(&sha, "", &bytes).unwrap();
        cache.put_manifest(&sha, "origin", b"Erf:one.hak\n").unwrap();
        for (_, data) in &c.resources {
            cache.put(&sha1(data), &compress(data, MAGIC_NSYC).unwrap()).unwrap();
        }
        let plan = haks::from_origins(&origins_of(), &c.entries(), &["two".into()]).unwrap();
        haks::write_all(&plan, store, 1, &|_| {}, &|| false).unwrap();
        let index = Index { haks: vec![("two".into(), plan.haks[0].key())], files: vec![] };
        cache.put_written(&sha, index).unwrap();
        manifests.push((sha, c));
        // (Each used after the one before.)
        std::thread::sleep(Duration::from_millis(20));
    }
    let texture = |n: usize| sha1(&manifests[n].1.resources[3].1);
    let all_old = Keep { recent: 1, min_age: Duration::ZERO, ..Default::default() };

    // A dry run: what would go, nothing gone.
    let p = prune(&cache, None, &all_old, true).unwrap();
    assert_eq!(p.kept, [manifests[2].0]);
    assert_eq!((p.removed.len(), p.data_files, p.haks), (2, 2, 2));
    assert!(cache.contains(&texture(0)) && cache.contains(&texture(1)));

    // The first named by its server too: it stays.
    cache.set_latest(URL, &manifests[0].0).unwrap();
    let p = prune(&cache, None, &all_old, false).unwrap();
    assert_eq!(p.removed, [manifests[1].0]);
    assert_eq!((p.data_files, p.haks, p.other), (1, 1, 3));
    assert!(cache.contains(&texture(0)) && !cache.contains(&texture(1)));
    assert!(cache.contains(&texture(2)));
    assert!(cache.manifest(&manifests[1].0, "").is_none());
    assert!(cache.manifest(&manifests[1].0, "origin").is_none());
    assert!(cache.written(&manifests[1].0).haks.is_empty());
    for (sha, c) in [&manifests[0], &manifests[2]] {
        assert!(cache.manifest(sha, "").is_some());
        let key = cache.written(sha).haks[0].1;
        assert!(cache.hak_path(&key).is_file());
        assert!(c.resources.iter().all(|(_, d)| cache.contains(&sha1(d))));
    }

    // Nothing kept but the module's: what was just changed stays.
    let fresh = Keep { manifests: vec![manifests[2].0], recent: 0, ..Default::default() };
    let p = prune(&cache, None, &fresh, false).unwrap();
    assert_eq!((p.data_files, p.haks, p.other), (0, 0, 0));

    // A kept manifest whose resources can't be read here: no data file goes.
    let unknown = sha1(b"a manifest neither has");
    let keep = Keep { manifests: vec![unknown], recent: 0, min_age: Duration::ZERO };
    let p = prune(&cache, None, &keep, false).unwrap();
    assert!(p.data_left.is_some());
    assert_eq!(p.data_files, 0);
    assert!(cache.contains(&texture(0)));
}

/// The server names a manifest it then doesn't give: `latest/` is not
/// pointed at it (a prune would keep a manifest nothing here can read),
/// and the one here is used with a warning.
#[test]
fn latest_is_kept_only_once_its_manifest_is_here() {
    let dir = mg_testkit::scratch_dir("nwsync-latest-kept");
    let content = Content::new();
    let (sha, _) = content.manifest();
    let mut server = Server::of(&content);
    server.files.remove(&format!("manifests/{}", sha1_hex(&sha)));
    let cache = Cache::new(dir.join("cache"));
    let e = fetch::resolve(&*server.fetch(), &settings(Wanted::Latest), &cache, None).unwrap_err();
    assert!(e.contains("has no manifest"), "{e}");
    assert_eq!(cache.latest(URL), None, "latest/ names a manifest that isn't here");
    // An older one here: used, with a warning; latest/ still names it.
    let mut older = Content::new();
    older.resources[0].1 = b"an older table".to_vec();
    let (old_sha, old_bytes) = older.manifest();
    cache.put_manifest(&old_sha, "", &old_bytes).unwrap();
    cache.set_latest(URL, &old_sha).unwrap();
    let r = fetch::resolve(&*server.fetch(), &settings(Wanted::Latest), &cache, None).unwrap();
    assert_eq!((r.found.sha1, r.server_latest), (old_sha, Some(sha)));
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(cache.latest(URL).map(|l| l.0), Some(old_sha));
    // Given: latest/ names it.
    let server = Server::of(&content);
    fetch::resolve(&*server.fetch(), &settings(Wanted::Latest), &cache, None).unwrap();
    assert_eq!(cache.latest(URL).map(|l| l.0), Some(sha));
}

/// A manifest named in `latest/`, or one of the last used, that can't be
/// read here is passed over with a note: data files no readable kept
/// manifest needs still go. Only a manifest the caller named keeps them
/// all.
#[test]
fn an_unreadable_latest_or_recent_manifest_doesnt_stop_the_prune() {
    use mg_nwsync::prune::{Keep, prune};
    use std::time::Duration;
    let dir = mg_testkit::scratch_dir("nwsync-prune-unreadable");
    let cache = Cache::new(dir.join("cache"));
    let content = Content::new();
    let (sha, bytes) = content.manifest();
    cache.put_manifest(&sha, "", &bytes).unwrap();
    for (_, data) in &content.resources {
        cache.put(&sha1(data), &compress(data, MAGIC_NSYC).unwrap()).unwrap();
    }
    let stale = b"needed by nothing".to_vec();
    cache.put(&sha1(&stale), &store(&stale, MAGIC_NSYC)).unwrap();
    // latest/ names a manifest neither has; a manifest file here that
    // isn't what its name says.
    cache.set_latest("https://other.example.org", &sha1(b"never downloaded")).unwrap();
    let broken = sha1(b"a manifest cut off");
    cache.put_manifest(&broken, "", b"NSYM cut").unwrap();
    let keep = Keep { manifests: vec![sha], recent: 3, min_age: Duration::ZERO };
    let p = prune(&cache, None, &keep, false).unwrap();
    assert_eq!(p.data_left, None);
    assert_eq!(p.data_files, 1);
    assert!(!cache.contains(&sha1(&stale)));
    assert!(content.resources.iter().all(|(_, d)| cache.contains(&sha1(d))));
    assert_eq!(p.notes.len(), 2, "{:?}", p.notes);
    assert!(p.notes.iter().all(|n| n.contains("passed over")), "{:?}", p.notes);
}

/// A manifest used from the game's storage alone (pinned, never in the
/// cache's `manifests/`) leaves a mark when it is used: a prune counts it
/// among the last used, and keeps the data the cache holds for it. With
/// the game's storage not given it can't be read, and is passed over with
/// a note.
#[test]
fn a_manifest_used_from_the_game_s_storage_keeps_its_data() {
    use mg_nwsync::prune::{Keep, prune};
    use std::time::Duration;
    let dir = mg_testkit::scratch_dir("nwsync-prune-used");
    let content = Content::new();
    let (sha, _) = content.manifest();
    let g = game_storage(&dir.join("game"), &content, URL, 100, &["appearance"], false);
    let cache = Cache::new(dir.join("cache"));
    for (_, data) in &content.resources[1..] {
        cache.put(&sha1(data), &compress(data, MAGIC_NSYC).unwrap()).unwrap();
    }
    cache.touch(&sha);
    let keep = Keep { manifests: vec![], recent: 1, min_age: Duration::ZERO };
    let p = prune(&cache, Some(&g), &keep, true).unwrap();
    assert_eq!(p.kept, [sha]);
    assert_eq!(p.data_files, 0);
    let p = prune(&cache, None, &keep, true).unwrap();
    assert_eq!(p.data_left, None);
    assert!(p.notes.iter().any(|n| n.contains(&sha1_hex(&sha)[..8])), "{:?}", p.notes);
    // The mark of one used before it, not kept, goes with a prune; the
    // kept one's stays.
    let before = sha1(b"a manifest used before");
    cache.touch(&before);
    let past = std::time::SystemTime::now() - Duration::from_secs(60);
    let f = std::fs::File::options().write(true).open(cache.used_path(&before)).unwrap();
    f.set_modified(past).unwrap();
    drop(f);
    let p = prune(&cache, Some(&g), &keep, false).unwrap();
    assert_eq!((p.kept, p.removed), (vec![sha], vec![before]));
    assert!(!cache.used_path(&before).exists());
    assert!(cache.used_path(&sha).is_file());
    assert_eq!(p.data_files, 0);
    // (Touching a cache that isn't there makes no folder.)
    let none = Cache::new(dir.join("no cache"));
    none.touch(&sha);
    assert!(!none.root().exists());
}

/// A shard's id the game's metadata can't have (no file before the
/// first): an error that says so, not an overflow.
#[test]
fn a_shard_id_out_of_range_is_an_error() {
    let dir = mg_testkit::scratch_dir("nwsync-game-shard-id");
    let content = Content::new();
    drop(game_storage(&dir, &content, URL, 100, &["appearance"], false));
    let meta = rusqlite::Connection::open(dir.join(mg_nwsync::game::META)).unwrap();
    meta.execute("insert into shards values (?1, 1)", [i64::MIN]).unwrap();
    drop(meta);
    let e = GameStorage::open(&dir).unwrap_err();
    assert!(e.contains("shard"), "{e}");
}

/// The game holding its data file exclusively (a commit being written):
/// a read waits for it, without writing, and once the game lets go it
/// reads; the storage's files are as they were.
#[test]
fn a_read_waits_for_the_game_s_exclusive_lock() {
    let dir = mg_testkit::scratch_dir("nwsync-game-exclusive");
    let content = Content::new();
    let all = ["appearance", "pw_head001", "pw_head002", "pw_tex", "world", "pw_sound"];
    let g = game_storage(&dir, &content, URL, 100, &all, false);
    let stamp = |dir: &Path| -> Vec<(String, u64, std::time::SystemTime)> {
        let mut out: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| {
                let m = e.metadata().unwrap();
                (e.file_name().to_string_lossy().into_owned(), m.len(), m.modified().unwrap())
            })
            .collect();
        out.sort();
        out
    };
    let before = stamp(&dir);
    let game = rusqlite::Connection::open(dir.join("nwsyncdata_0.sqlite3")).unwrap();
    game.execute_batch("begin exclusive").unwrap();
    let appearance = sha1(content.bytes("appearance", ResType::TWODA));
    let (started, done) = (std::sync::mpsc::channel(), std::sync::mpsc::channel());
    std::thread::scope(|s| {
        let reader = s.spawn(|| {
            started.0.send(()).unwrap();
            let read = g.read(&appearance);
            let hashes = g.hashes();
            done.0.send(()).unwrap();
            (read, hashes)
        });
        started.1.recv().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(done.1.try_recv().is_err(), "the read went on under the game's exclusive lock");
        game.execute_batch("rollback").unwrap();
        let (read, hashes) = reader.join().unwrap();
        assert!(read.unwrap().is_some());
        assert_eq!(hashes.unwrap().len(), 5);
    });
    drop(game);
    drop(g);
    assert_eq!(stamp(&dir), before, "the game's storage was changed");
}

/// Every SHA-1 a shard holds is read, whatever its first digits (the
/// reads go a small range at a time), uppercase ones too.
#[test]
fn every_hash_is_read_range_by_range() {
    let dir = mg_testkit::scratch_dir("nwsync-game-hashes");
    let mut held = Vec::new();
    for b in 0..=255u8 {
        let mut sha = sha1(&[b]);
        sha[0] = b;
        held.push((sha, vec![b]));
    }
    write_storage(&dir, &[], &[held.clone()]);
    let upper = "FF".repeat(20);
    let shard = rusqlite::Connection::open(dir.join("nwsyncdata_0.sqlite3")).unwrap();
    shard.execute("insert into resrefs values (?1, x'00', 1, 0)", [&upper]).unwrap();
    drop(shard);
    let g = GameStorage::open(&dir).unwrap().unwrap();
    let got = g.hashes().unwrap();
    assert_eq!(got.len(), 257);
    assert!(held.iter().all(|(s, _)| got.contains(s)));
    assert!(got.contains(&[0xff; 20]));
}

/// Whether a manifest holds the module's own resources (`--with-module`):
/// as its `.json` side file says, else the game's metadata; not known
/// where neither says.
#[test]
fn a_manifest_says_whether_it_holds_the_module() {
    use mg_nwsync::fixtures::write_metadata;
    let dir = mg_testkit::scratch_dir("nwsync-includes-module");
    let content = Content::new();
    let (sha, _) = content.manifest();
    let game_dir = dir.join("game");
    drop(game_storage(&game_dir, &content, URL, 100, &[], false));
    write_metadata(&game_dir, &sha, Some(true));
    let g = GameStorage::open(&game_dir).unwrap().unwrap();
    assert_eq!(g.includes_module_contents(&sha), Some(true));
    assert_eq!(g.includes_module_contents(&sha1(b"another")), None);
    let f = find(URL, Wanted::Pinned(sha), None, Some(&g)).unwrap();
    assert_eq!(f.includes_module, Some(true));
    // The cache's side file says otherwise: it is the word.
    let cache = Cache::new(dir.join("cache"));
    cache
        .put_manifest(&sha, "json", br#"{"version": 3, "includes_module_contents": false}"#)
        .unwrap();
    let f = find(URL, Wanted::Pinned(sha), Some(&cache), Some(&g)).unwrap();
    assert_eq!(f.includes_module, Some(false));
    // Downloaded from the server, with its side file.
    let mut server = Server::of(&content);
    let json = format!("manifests/{}.json", sha1_hex(&sha));
    server.files.insert(json.clone(), br#"{"includes_module_contents": true}"#.to_vec());
    let cache = Cache::new(dir.join("cache2"));
    let r = fetch::resolve(&*server.fetch(), &settings(Wanted::Latest), &cache, None).unwrap();
    assert_eq!((r.found.from, r.found.includes_module), (ManifestFrom::Server, Some(true)));
    // Neither says (a side file without it, a game's storage without the
    // value, or without the table at all).
    let server = Server::of(&content);
    let cache = Cache::new(dir.join("cache3"));
    let r = fetch::resolve(&*server.fetch(), &settings(Wanted::Latest), &cache, None).unwrap();
    assert_eq!(r.found.includes_module, None);
    let empty = dir.join("game-empty");
    drop(game_storage(&empty, &content, URL, 100, &[], false));
    write_metadata(&empty, &sha, None);
    let g = GameStorage::open(&empty).unwrap().unwrap();
    assert_eq!(g.includes_module_contents(&sha), None);
    let meta = rusqlite::Connection::open(empty.join(mg_nwsync::game::META)).unwrap();
    meta.execute_batch("drop table manifest_metadata").unwrap();
    drop(meta);
    let g = GameStorage::open(&empty).unwrap().unwrap();
    assert_eq!(g.includes_module_contents(&sha), None);
}
