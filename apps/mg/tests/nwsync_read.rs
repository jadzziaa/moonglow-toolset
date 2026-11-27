//! `mg nwsync-status` and `mg nwsync-pull` with a server of the test's own
//! on this computer, serving a repository Moonglow published
//! (`mg_module::nwsync::write`), and a game's NWSync storage made up to the
//! client's schema: the module's project names the server, what the game
//! has is not downloaded, what is downloaded lands in the cache alone, and
//! with the server down the manifest here is used, with a warning.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use mg_core::ResType;
use mg_module::nwsync::{Options, Source, write};
use mg_nwsync::fixtures::{Registered, write_storage};
use mg_resman::ResKey;

/// A server of `root`'s files on 127.0.0.1 (a connection a request,
/// each on a thread): its address, whether it is to answer 503 as a
/// server that is down, and how many data files it was asked for.
pub(crate) fn serve(root: PathBuf) -> (String, Arc<AtomicBool>, Arc<AtomicUsize>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let down = Arc::new(AtomicBool::new(false));
    let asked = Arc::new(AtomicUsize::new(0));
    let (is_down, data) = (down.clone(), asked.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (root, is_down, data) = (root.clone(), is_down.clone(), data.clone());
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                let _ = reader.read_line(&mut first);
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                }
                let path = first.split(' ').nth(1).unwrap_or("/").trim_start_matches('/');
                if path.starts_with("data/") {
                    data.fetch_add(1, Ordering::Relaxed);
                }
                let file = (!path.contains("..")).then(|| std::fs::read(root.join(path)).ok());
                let (status, body) = match file.flatten() {
                    _ if is_down.load(Ordering::Relaxed) => ("503 Service Unavailable", Vec::new()),
                    Some(b) => ("200 OK", b),
                    None => ("404 Not Found", Vec::new()),
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let mut stream = stream;
                let _ = stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(&body));
            });
        }
    });
    (url, down, asked)
}

fn key(name: &str, restype: ResType) -> ResKey {
    ResKey::parse(name, restype).unwrap()
}

/// A module folder whose project names `url` as its NWSync server.
fn module_naming(dir: &Path, url: &str) -> PathBuf {
    let folder = dir.join("module");
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    info.root.set("Mod_HakList", mg_gff::Value::List(Vec::new()));
    m.set_info(&info).unwrap();
    m.save_as(&mg_module::ModuleLocation::Folder(folder.clone())).unwrap();
    std::fs::write(folder.join("moonglow.toml"), format!("[nwsync]\nsource = \"{url}/\"\n"))
        .unwrap();
    folder
}

#[test]
fn a_module_s_nwsync_content_is_pulled_into_the_cache() {
    let dir = mg_testkit::scratch_dir("mg-nwsync-read");
    let resources: Vec<(ResKey, &[u8])> = vec![
        (key("pw_appear", ResType::TWODA), b"2DA V2.0\n\n A\n0 x\n"),
        (key("pw_head", ResType::MDL), b"a model of a head, a model of a head"),
        (key("world", ResType::TLK), b"TLK V3.0 and the rest of a talk table"),
    ];
    let contents: Vec<(ResKey, Source)> =
        resources.iter().map(|(k, b)| (*k, Source::Bytes((*b).into()))).collect();
    let o = Options { latest: true, ..Default::default() };
    let written = write(&dir.join("repo"), &contents, &o, &mut |_, _| {}).unwrap();
    let (url, down, asked) = serve(dir.join("repo"));
    let folder = module_naming(&dir, &url);
    let m = folder.to_str().unwrap();

    // The game joined the server once: it has the manifest and the head.
    let sha = |b: &[u8]| mg_core::sha1::sha1(b);
    let manifest = mg_nwsync::parse_sha1(&written.sha1).unwrap();
    let head = resources[1].1;
    let user = dir.join("user");
    write_storage(
        &user.join("nwsync"),
        &[Registered {
            sha1: manifest,
            origin_url: url.clone(),
            created_at: 1,
            last_used_at: 1,
            resources: resources.iter().map(|(k, b)| (*k, sha(b))).collect(),
            cached: None,
        }],
        &[vec![(sha(head), head.to_vec())]],
    );
    let storage_before: Vec<Vec<u8>> = ["nwsyncmeta.sqlite3", "nwsyncdata_0.sqlite3"]
        .iter()
        .map(|f| std::fs::read(user.join("nwsync").join(f)).unwrap())
        .collect();
    let cache = dir.join("cache");
    let (c, u) = (cache.to_str().unwrap(), user.to_str().unwrap());

    // What is here, without asking the server: the game's manifest.
    let (ok, j) = crate::mg_json(&["--user-dir", u, "nwsync-status", m, "--cache", c]);
    assert!(ok, "{j}");
    assert_eq!(j["manifest"], written.sha1.as_str());
    assert_eq!(j["from"], "game_storage");
    assert_eq!(j["server"], url.as_str());
    let s = &j["survey"];
    assert_eq!((s["resources"].as_u64(), s["data_files"].as_u64()), (Some(3), Some(3)));
    assert_eq!((s["in_game_storage"].as_u64(), s["missing"].as_u64()), (Some(1), Some(2)));
    assert_eq!(asked.load(Ordering::Relaxed), 0);

    // Pulled: the two the game hasn't, into the cache.
    let (ok, j) = crate::mg_json(&["--user-dir", u, "nwsync-pull", m, "--cache", c]);
    assert!(ok, "{j}");
    assert_eq!((j["downloaded"].as_u64(), j["left"].as_u64()), (Some(2), Some(0)));
    assert_eq!(j["server_latest"], written.sha1.as_str());
    assert_eq!(asked.load(Ordering::Relaxed), 2);
    let kept = mg_nwsync::Cache::new(&cache);
    assert!(kept.contains(&sha(resources[0].1)) && kept.contains(&sha(resources[2].1)));
    assert!(!kept.contains(&sha(head)));
    let (ok, j) = crate::mg_json(&["--user-dir", u, "nwsync-status", m, "--cache", c]);
    assert!(ok, "{j}");
    let s = &j["survey"];
    assert_eq!((s["in_cache"].as_u64(), s["missing"].as_u64()), (Some(2), Some(0)));
    // Again: nothing more downloaded.
    let (ok, j) = crate::mg_json(&["--user-dir", u, "nwsync-pull", m, "--cache", c]);
    assert!(ok && j["downloaded"] == 0, "{j}");
    assert_eq!(asked.load(Ordering::Relaxed), 2);

    // The server down: the manifest here, with a warning.
    down.store(true, Ordering::Relaxed);
    let (ok, j) = crate::mg_json(&["--user-dir", u, "nwsync-status", m, "--cache", c, "--online"]);
    assert!(ok, "{j}");
    assert_eq!(j["manifest"], written.sha1.as_str());
    let notes = j["notes"].to_string();
    assert!(notes.contains("could not be asked") && notes.contains("503"), "{notes}");

    // The game's storage is as it was.
    for (f, before) in ["nwsyncmeta.sqlite3", "nwsyncdata_0.sqlite3"].iter().zip(storage_before) {
        assert_eq!(std::fs::read(user.join("nwsync").join(f)).unwrap(), before, "{f}");
    }
    // A module whose project names no server, and nothing given: said.
    let (ok, j) = crate::mg_json(&["--no-user-dir", "nwsync-status", "--cache", c]);
    assert!(!ok && j["error"].to_string().contains("--source"), "{j}");
}

/// `mg nwsync-haks`: a module's haks written from its NWSync content (the
/// server publishes no origins file: the resources spread over the
/// module's haks), and its talk table, put in `--out`'s `hak` and `tlk`;
/// written once, then as written before. `mg nwsync-cache-prune` keeps
/// what the module's manifest needs.
#[test]
fn a_module_s_haks_are_written_from_its_nwsync_content() {
    let dir = mg_testkit::scratch_dir("mg-nwsync-haks");
    let resources: Vec<(ResKey, &[u8])> = vec![
        (key("pw_appear", ResType::TWODA), b"2DA V2.0\n\n A\n0 x\n"),
        (key("pw_head", ResType::MDL), b"a model of a head, a model of a head"),
        (key("pw_tex", ResType::TGA), b"a texture"),
        (key("world", ResType::TLK), b"TLK V3.0 and the rest of a talk table"),
    ];
    let contents: Vec<(ResKey, Source)> =
        resources.iter().map(|(k, b)| (*k, Source::Bytes((*b).into()))).collect();
    let o = Options { latest: true, ..Default::default() };
    let written = write(&dir.join("repo"), &contents, &o, &mut |_, _| {}).unwrap();
    let (url, _, _) = serve(dir.join("repo"));
    let folder = module_naming(&dir, &url);
    let mut m = mg_module::Module::open(&folder).unwrap();
    let mut info = m.info().unwrap();
    let haks = ["pw_one", "pw_two"].map(|h| {
        let mut s = mg_gff::Struct::new(8);
        s.set("Mod_Hak", mg_gff::Value::String(h.as_bytes().to_vec()));
        s
    });
    info.root.set("Mod_HakList", mg_gff::Value::List(haks.to_vec()));
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"world".to_vec()));
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let (cache, out) = (dir.join("cache"), dir.join("out"));
    let (c, o, f) = (cache.to_str().unwrap(), out.to_str().unwrap(), folder.to_str().unwrap());

    // Nothing downloaded: said how to.
    let (ok, j) = crate::mg_json(&["--no-user-dir", "nwsync-haks", f, "--cache", c]);
    assert!(!ok && j["error"].to_string().contains("mg nwsync-pull"), "{j}");
    let (ok, j) = crate::mg_json(&["--no-user-dir", "nwsync-pull", f, "--cache", c]);
    assert!(ok, "{j}");

    let (ok, j) = crate::mg_json(&["--no-user-dir", "nwsync-haks", f, "--cache", c, "--out", o]);
    assert!(ok, "{j}");
    assert_eq!(j["manifest"], written.sha1.as_str());
    assert_eq!(j["plan"], "spread");
    let haks = j["haks"].as_array().unwrap();
    assert_eq!(haks.len(), 2);
    assert!(haks.iter().all(|h| h["written"] == true));
    let total: u64 = haks.iter().map(|h| h["resources"].as_u64().unwrap()).sum();
    assert_eq!(total, 3);
    assert_eq!(j["tlk"]["name"], "world.tlk");
    assert_eq!(std::fs::read(out.join("tlk/world.tlk")).unwrap(), resources[3].1);
    let mut found = 0;
    for name in ["pw_one", "pw_two"] {
        let bytes = std::fs::read(out.join(format!("hak/{name}.hak"))).unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        for (k, b) in &resources[..3] {
            if let Some(got) = erf.get(&k.resref, k.restype) {
                assert_eq!(&*got.unwrap(), *b);
                found += 1;
            }
        }
    }
    assert_eq!(found, 3);
    let (ok, j) = crate::mg_json(&["--no-user-dir", "nwsync-haks", f, "--cache", c, "--out", o]);
    assert!(ok, "{j}");
    assert!(j["haks"].as_array().unwrap().iter().all(|h| h["written"] == false), "{j}");

    let (ok, j) = crate::mg_json(&["--no-user-dir", "nwsync-cache-prune", f, "--cache", c]);
    assert!(ok, "{j}");
    assert_eq!(j["kept"][0], written.sha1.as_str());
    assert_eq!((j["data_files"].as_u64(), j["haks"].as_u64()), (Some(0), Some(0)));
    let (ok, _) = crate::mg_json(&["--no-user-dir", "nwsync-haks", f, "--cache", c]);
    assert!(ok);
}

/// `--source` another server than the project's: the project's pinned
/// manifest (its own server's) is not taken for it, and that is said.
/// `nwsync-haks --out` replaces what is at a file it puts whole: a link
/// there of something else's is not written through. A prune says the
/// manifests it passed over.
#[test]
fn another_source_a_link_in_the_out_folder_and_what_a_prune_passed_over() {
    let dir = mg_testkit::scratch_dir("mg-nwsync-other-source");
    let resources: Vec<(ResKey, &[u8])> = vec![(key("pw_appear", ResType::TWODA), b"2DA V2.0\n")];
    let contents: Vec<(ResKey, Source)> =
        resources.iter().map(|(k, b)| (*k, Source::Bytes((*b).into()))).collect();
    let o = Options { latest: true, ..Default::default() };
    let written = write(&dir.join("repo"), &contents, &o, &mut |_, _| {}).unwrap();
    let (theirs, _, _) = serve(dir.join("repo"));
    let (other, _, _) = serve(dir.join("repo"));
    let folder = module_naming(&dir, &theirs);
    let pin = "0123456789abcdef0123456789abcdef01234567";
    let toml = format!("[nwsync]\nsource = \"{theirs}\"\nmanifest = \"{pin}\"\n");
    std::fs::write(folder.join("moonglow.toml"), toml).unwrap();
    let mut m = mg_module::Module::open(&folder).unwrap();
    let mut info = m.info().unwrap();
    let mut hak = mg_gff::Struct::new(8);
    hak.set("Mod_Hak", mg_gff::Value::String(b"pw_one".to_vec()));
    info.root.set("Mod_HakList", mg_gff::Value::List(vec![hak]));
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let (cache, out) = (dir.join("cache"), dir.join("out"));
    let (c, o, f) = (cache.to_str().unwrap(), out.to_str().unwrap(), folder.to_str().unwrap());

    let (ok, j) =
        crate::mg_json(&["--no-user-dir", "nwsync-pull", f, "--source", &other, "--cache", c]);
    assert!(ok, "{j}");
    assert_eq!(j["manifest"], written.sha1.as_str());
    let notes = j["notes"].to_string();
    assert!(notes.contains("[nwsync] manifest 01234567") && notes.contains("not taken"), "{j}");

    let victim = dir.join("victim");
    std::fs::write(&victim, b"not to be written").unwrap();
    std::fs::create_dir_all(out.join("hak")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&victim, out.join("hak/pw_one.hak")).unwrap();
    let args = ["--no-user-dir", "nwsync-haks", f, "--source", &other, "--cache", c, "--out", o];
    let (ok, j) = crate::mg_json(&args);
    assert!(ok, "{j}");
    assert_eq!(std::fs::read(&victim).unwrap(), b"not to be written");
    let placed = out.join("hak/pw_one.hak");
    assert!(!std::fs::symlink_metadata(&placed).unwrap().file_type().is_symlink());
    assert!(mg_erf::Erf::read(&std::fs::read(&placed).unwrap()).is_ok());

    // A server's latest whose manifest isn't here: passed over, and said.
    std::fs::write(
        cache.join("latest/gone.example.org"),
        "ffffffffffffffffffffffffffffffffffffffff",
    )
    .unwrap();
    let (ok, j) = crate::mg_json(&["--no-user-dir", "nwsync-cache-prune", "--cache", c]);
    assert!(ok, "{j}");
    assert!(j["passed_over"][0].as_str().unwrap().contains("passed over"), "{j}");
    assert!(j["notes"].to_string().contains("ffffffff"), "{j}");
}

/// `mg serve`, `mg test` and `mg watch` take `--no-nwsync` (the haks and
/// talk table from disk, as without [nwsync] source).
#[test]
fn serve_test_and_watch_take_no_nwsync() {
    for cmd in ["serve", "test", "watch"] {
        let out = crate::mg_output(&[cmd, "--help"]);
        let help = String::from_utf8_lossy(&out.stdout);
        assert!(help.contains("--no-nwsync"), "mg {cmd} --help: {help}");
    }
}
