//! `mg compile` passes over the scripts that are as they were when last
//! compiled, and keeps what it remembers in Moonglow's data folder (where
//! the window keeps the same), never beside the module.

use std::path::Path;

use mg_core::ResType;
use mg_resman::ResKey;
use serde_json::Value;

fn compile(root: &Path, dir: &Path, module: &Path, more: &[&str]) -> Value {
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--json", "--user-dir"])
        .arg(dir.join("user"))
        .arg("compile")
        .arg(module)
        .args(more)
        // (A data folder that starts empty: what was compiled before is
        // remembered there.)
        .env("MOONGLOW_DATA_DIR", dir.join("data"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn a_second_compile_passes_over_what_is_unchanged() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-compile");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let at = dir.join("module");
    std::fs::create_dir_all(&at).unwrap();
    let chess = at.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();

    let first = compile(&root, &dir, &chess, &[]);
    let scripts = first["scripts"].as_u64().unwrap();
    assert!(scripts > 10 && first["failed"] == 0 && first["unchanged"] == 0, "{first}");
    // Remembered in the data folder; nothing new beside the module.
    let kept: Vec<_> = walk(&dir.join("data")).into_iter().filter(|p| is_json(p)).collect();
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert!(kept[0].to_string_lossy().contains("compiled"), "{kept:?}");
    assert!(!walk(&at).iter().any(|p| is_json(p)));

    // As they were: none compiled. Named another way, the same module.
    let again = compile(&root, &dir, &at.join("../module/chess.mod"), &[]);
    assert!(again["scripts"] == 0 && again["unchanged"] == scripts, "{again}");

    // One script changed: it alone, unless others include it.
    let mut m = mg_module::Module::open(&chess).unwrap();
    let key = *(m.keys_of(ResType::NSS))
        .find(|k| m.contains(&ResKey::new(k.resref, ResType::NCS)))
        .expect("a script with a compiled one");
    let mut source = m.get(&key).unwrap().to_vec();
    source.extend_from_slice(b"\n// changed\n");
    m.set(key, source);
    m.save().unwrap();
    let includers = mg_module::refs::includers(&m, &[key]).len() as u64;
    let third = compile(&root, &dir, &chess, &[]);
    assert_eq!(third["scripts"], 1 + includers, "{third}");

    // --all compiles them all, whatever is remembered.
    let all = compile(&root, &dir, &chess, &["--all"]);
    assert!(all["scripts"] == scripts && all["unchanged"] == 0, "{all}");

    // --ndb: those compiled without a debug file are compiled again for
    // one, once.
    // (Of the scripts with a source: one compiled script has none.)
    let debug_files = |m: &mg_module::Module| {
        let made = |t| {
            (m.keys_of(ResType::NSS))
                .filter(|k| m.get(&ResKey::new(k.resref, t)).is_some_and(|d| !d.is_empty()))
                .count()
        };
        (made(ResType::NCS), made(ResType::NDB))
    };
    assert_eq!(debug_files(&mg_module::Module::open(&chess).unwrap()).1, 0);
    let with = compile(&root, &dir, &chess, &["--ndb"]);
    let (ncs, ndb) = debug_files(&mg_module::Module::open(&chess).unwrap());
    assert!(ncs > 10 && ndb == ncs, "{ncs} compiled scripts, {ndb} debug files");
    assert_eq!(with["scripts"], ncs as u64, "{with}");
    let again = compile(&root, &dir, &chess, &["--ndb"]);
    assert_eq!(again["scripts"], 0, "{again}");
}

/// `mg compile --uncompiled` and `mg build` remember what they compiled
/// where `mg compile` (and the window) do: the next compile passes over
/// it. A project is one to them whether named by its folder or by its
/// `nasher.cfg`.
#[test]
fn uncompiled_and_build_remember_what_they_compiled() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-compile-remembered");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let mut m = mg_module::Module::open(&chess).unwrap();
    m.set(ResKey::parse("fresh", ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    m.save().unwrap();
    // (Those without a compiled script: the new one, and include files.)
    let none = (m.keys_of(ResType::NSS))
        .filter(|k| !m.contains(&ResKey::new(k.resref, ResType::NCS)))
        .count() as u64;

    let first = compile(&root, &dir, &chess, &["--uncompiled"]);
    assert_eq!(first["scripts"], none, "{first}");
    // The others were never compiled here: all but those just compiled.
    let next = compile(&root, &dir, &chess, &[]);
    assert_eq!(next["unchanged"], none, "{next}");

    let kept = || walk(&dir.join("data")).into_iter().filter(|p| is_json(p)).count();
    assert_eq!(kept(), 1);
    let project = dir.join("project");
    let mg = |args: &[&str]| {
        let out = crate::mg()
            .args(["--root", root.to_str().unwrap(), "--user-dir"])
            .arg(dir.join("user"))
            .args(args)
            .env("MOONGLOW_DATA_DIR", dir.join("data"))
            .output()
            .unwrap();
        assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    };
    mg(&["init", chess.to_str().unwrap(), project.to_str().unwrap()]);
    let built = dir.join("built.mod");
    mg(&["build", project.to_str().unwrap(), "-o", built.to_str().unwrap()]);
    assert_eq!(kept(), 2, "mg build remembers what it compiled");
    // (A project's tree keeps no compiled scripts: they are made again.)
    let by_cfg = compile(&root, &dir, &project.join("nasher.cfg"), &[]);
    assert!(by_cfg["scripts"].as_u64().unwrap() > 10, "{by_cfg}");
    assert_eq!(kept(), 2, "named by its nasher.cfg, the same project");
}

fn is_json(p: &Path) -> bool {
    p.extension().is_some_and(|e| e == "json")
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        match e.path() {
            p if p.is_dir() => out.extend(walk(&p)),
            p => out.push(p),
        }
    }
    out
}
