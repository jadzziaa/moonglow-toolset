//! NWSync publishing (`mg_module::nwsync`) against neverwinter.nim's
//! `nwn_nwsync_write`: Neverwinter Chess with two of the game's haks and a
//! premium campaign's talk table, published by both into repositories of
//! their own.
//!
//! - **Haks and talk table:** the manifests are the same, byte for byte
//!   (the same SHA-1); the data files have the same names and, unpacked, the
//!   same bytes (the compression differs); the .json says the same but for
//!   when, by what and the compressed size; `nwn_nwsync_print` reads
//!   Moonglow's manifest and `nwn_compressedbuf` its data files.
//! - **With the module:** the same resources, and the same `module.ifo`
//!   field for field (each writer lays out the GFF its own way): no hak
//!   list, the UUID given.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use mg_core::sha1::hex;
use mg_gff::{Gff, Value};
use mg_module::Module;
use mg_module::nwsync::{Options, module_contents, read_manifest, write};
use mg_resman::{GameInstall, ResMan};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

const UUID: &str = "6f2b3c1e-4d5a-4b6c-8d7e-9f0a1b2c3d4e";

/// Each data file's name and unpacked bytes.
fn data(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut dirs = vec![root.join("data")];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else {
                let packed = std::fs::read(&p).unwrap();
                let bytes = mg_erf::compressedbuf::decompress(&packed, *b"NSYC").unwrap();
                out.insert(p.file_name().unwrap().to_string_lossy().into_owned(), bytes);
            }
        }
    }
    out
}

fn latest(root: &Path) -> String {
    std::fs::read_to_string(root.join("latest")).unwrap().trim().to_string()
}

/// A GFF's fields by label, as JSON, for comparing writers' output.
fn fields(data: &[u8]) -> BTreeMap<String, String> {
    let g = Gff::read(data).unwrap();
    g.root.fields.iter().map(|f| (f.label.to_string_lossy(), format!("{:?}", f.value))).collect()
}

#[test]
fn published_as_nwn_nwsync_write_publishes() {
    let root = corpus!();
    let nim = oracle_tool!("nwn_nwsync_write");
    let print = oracle_tool!("nwn_nwsync_print");
    let dir = scratch_dir("nwsync");
    let install = GameInstall::new(&root, None, "en");

    // Chess with haks and a talk table the game ships.
    let path = dir.join("chess.mod");
    let mut m = Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let mut ifo = m.info().unwrap();
    let haks = ["sg_resources".to_string(), "potsc_top".to_string()];
    ifo.root.set("Mod_HakList", mg_module::attach::hak_list(&ifo.root, &haks));
    ifo.root.set("Mod_CustomTlk", Value::String(b"tyrants".to_vec()));
    m.set_info(&ifo).unwrap();
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let m = Module::open(&path).unwrap();
    let mut rm = ResMan::for_game(&install).unwrap();
    let (missing, wrong) = m.stack_onto(&mut rm, &install);
    assert!(missing.is_empty() && wrong.is_none(), "{missing:?} {wrong:?}");

    for with_module in [false, true] {
        let label = if with_module { "with the module" } else { "haks and talk table" };
        let (theirs, ours) =
            (dir.join(format!("nim-{with_module}")), dir.join(format!("mg-{with_module}")));
        let _ = std::fs::remove_dir_all(&theirs);
        let _ = std::fs::remove_dir_all(&ours);
        std::fs::create_dir_all(&theirs).unwrap();
        let mut args = vec!["--quiet".to_string()];
        for p in ["data/hk", "data/tlk"] {
            args.extend(["-p".to_string(), root.join(p).display().to_string()]);
        }
        if with_module {
            args.extend(["--with-module".to_string(), format!("--mod-uuid={UUID}")]);
        }
        let out = Command::new(&nim).args(&args).arg(&theirs).arg(&path).output().unwrap();
        assert!(out.status.success(), "{label}: {}", String::from_utf8_lossy(&out.stderr));

        let (contents, missing) =
            module_contents(&m, &install, &rm, with_module, Some(UUID)).unwrap();
        assert!(missing.is_empty(), "{missing:?}");
        let name = if with_module { "Neverwinter Chess" } else { "" };
        let o = Options {
            with_module,
            name: name.into(),
            uuid: with_module.then(|| UUID.into()),
            latest: true,
            limit: Some(mg_module::nwsync::FILE_LIMIT),
            ..Default::default()
        };
        let w = write(&ours, &contents, &o, &mut |_, _| {}).unwrap();
        assert_eq!(latest(&ours), w.sha1);

        let read = |r: &Path| {
            read_manifest(&std::fs::read(r.join("manifests").join(latest(r))).unwrap()).unwrap()
        };
        let (a, b) = (read(&theirs), read(&ours));
        assert!(a.len() > 1000, "{label}: {}", a.len());
        if !with_module {
            assert_eq!(latest(&ours), latest(&theirs), "{label}: the manifests differ");
            assert_eq!(data(&ours), data(&theirs), "{label}: the data files differ");
            let json = |r: &Path| -> serde_json::Value {
                let mut j: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(r.join("manifests").join(format!("{}.json", latest(r))))
                        .unwrap(),
                )
                .unwrap();
                for k in ["created", "created_with", "on_disk_bytes"] {
                    j.as_object_mut().unwrap().remove(k);
                }
                j
            };
            assert_eq!(json(&ours), json(&theirs), "{label}: the .json differs");
            let listed = Command::new(&print)
                .arg(ours.join("manifests").join(latest(&ours)))
                .output()
                .unwrap();
            assert!(listed.status.success(), "{}", String::from_utf8_lossy(&listed.stderr));
            // neverwinter.nim unpacks Moonglow's data files (zstd frames of
            // ruzstd's making).
            let unpack = oracle_tool!("nwn_compressedbuf");
            for (name, bytes) in data(&ours).iter().step_by(97).take(25) {
                let file = ours.join("data/sha1").join(&name[..2]).join(&name[2..4]).join(name);
                let out = Command::new(&unpack)
                    .args(["-d", "NSYC"])
                    .stdin(std::fs::File::open(&file).unwrap())
                    .output()
                    .unwrap();
                assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
                assert!(out.stdout == *bytes, "{name}: nwn_compressedbuf unpacks other bytes");
            }
            continue;
        }
        // With the module: the same resources; module.ifo the same fields.
        let names = |e: &[(mg_resman::ResKey, [u8; 20], u32)]| -> BTreeMap<String, String> {
            e.iter().map(|(k, s, _)| (k.to_string().to_lowercase(), hex(s))).collect()
        };
        let (mut na, mut nb) = (names(&a), names(&b));
        let (ia, ib) = (na.remove("module.ifo").unwrap(), nb.remove("module.ifo").unwrap());
        assert_eq!(na, nb, "{label}: the resources differ");
        let file = |r: &Path, h: &str| data(r).remove(h).unwrap();
        let (fa, fb) = (fields(&file(&theirs, &ia)), fields(&file(&ours, &ib)));
        let differ: Vec<&String> =
            fa.keys().chain(fb.keys()).filter(|k| fa.get(*k) != fb.get(*k)).collect();
        assert!(differ.is_empty(), "{label}: module.ifo differs in {differ:?}");
        assert!(!fb.contains_key("Mod_HakList"));
        assert_eq!(fb["Mod_UUID"], format!("{:?}", Value::String(UUID.as_bytes().to_vec())));
        assert!(nb.keys().all(|k| !k.ends_with(".nss")));
    }
}

/// The files under a folder, relative to it, sorted.
fn tree(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            let shown = p.strip_prefix(root).unwrap().display().to_string().replace('\\', "/");
            if p.is_dir() {
                dirs.push(p);
                out.push(format!("{shown}/"));
            } else {
                out.push(shown);
            }
        }
    }
    out.sort();
    out
}

/// Pruning (`mg_module::nwsync::prune`) against neverwinter.nim's
/// `nwn_nwsync_prune`: a repository of two manifests, one taken away, with
/// a stray file and an empty folder in its data, pruned by both; then
/// again with a young unused file and the default age. The same files are
/// left.
#[test]
fn pruned_as_nwn_nwsync_prune_prunes() {
    use mg_module::nwsync::{PRUNE_MIN_AGE, Source, prune};
    use mg_resman::ResKey;
    let nim = oracle_tool!("nwn_nwsync_prune");
    let dir = scratch_dir("nwsync-prune");
    let source = dir.join("source");
    let _ = std::fs::remove_dir_all(&source);
    let res = |n: &str, b: &'static [u8]| {
        (ResKey::parse(n, mg_core::ResType::TXT).unwrap(), Source::Bytes(b.into()))
    };
    let o = Options { latest: true, ..Default::default() };
    let first = write(&source, &[res("a", b"one"), res("b", b"two")], &o, &mut |_, _| {}).unwrap();
    write(&source, &[res("a", b"one"), res("c", b"three")], &o, &mut |_, _| {}).unwrap();
    std::fs::remove_file(source.join("manifests").join(&first.sha1)).unwrap();
    std::fs::write(source.join("data/sha1/stray.part"), b"half").unwrap();
    std::fs::create_dir_all(source.join("data/sha1/zz/yy")).unwrap();
    std::fs::write(source.join("notes.txt"), b"mine").unwrap();
    let age = |root: &Path, days: u64| {
        let then = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86_400);
        for f in tree(root).iter().filter(|f| !f.ends_with('/')) {
            let file = std::fs::File::options().write(true).open(root.join(f)).unwrap();
            file.set_modified(then).unwrap();
        }
    };
    age(&source, 30);
    let copy = |name: &str| {
        let to = dir.join(name);
        let _ = std::fs::remove_dir_all(&to);
        for f in tree(&source) {
            if let Some(d) = f.strip_suffix('/') {
                std::fs::create_dir_all(to.join(d)).unwrap();
            } else {
                std::fs::create_dir_all(to.join(&f).parent().unwrap()).unwrap();
                std::fs::copy(source.join(&f), to.join(&f)).unwrap();
            }
        }
        age(&to, 30);
        to
    };
    let theirs_run = |root: &Path, args: &[&str]| {
        let out = Command::new(&nim).args(args).arg(root).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    };

    // Every file old: what no manifest names goes.
    let (theirs, ours) = (copy("nim"), copy("mg"));
    theirs_run(&theirs, &["--quiet", "--min-age", "0"]);
    let p = prune(&ours, 0, false).unwrap();
    assert_eq!(tree(&ours), tree(&theirs), "{p:?}");
    assert_eq!((p.removed.len(), p.dangling.len()), (2, 1), "{p:?}");
    assert!(tree(&ours).contains(&"notes.txt".to_string()), "only data/ is pruned");

    // A young unused file stays, at the default age.
    let (theirs, ours) = (copy("nim-young"), copy("mg-young"));
    let h = hex(&mg_core::sha1::sha1(b"two"));
    for root in [&theirs, &ours] {
        let file = root.join("data/sha1").join(&h[..2]).join(&h[2..4]).join(&h);
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(std::time::SystemTime::now())
            .unwrap();
    }
    theirs_run(&theirs, &["--quiet"]);
    let p = prune(&ours, PRUNE_MIN_AGE, false).unwrap();
    assert_eq!(tree(&ours), tree(&theirs), "{p:?}");
    assert_eq!(p.kept_young, 1);
}
