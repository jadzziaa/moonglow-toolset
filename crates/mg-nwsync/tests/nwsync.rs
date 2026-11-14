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
    assert!(matches!(steps.last(), Some(fetch::Step::Downloading(p)) if p.files == 3));
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
