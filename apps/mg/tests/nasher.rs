//! `mg init` and `mg build`: a shipped module put into a nasher project and
//! built from it gives the module's resources back, scripts compiled.

use mg_core::ResType;
use mg_module::Module;
use mg_resman::ResKey;

#[test]
fn init_then_build_gives_the_module_back() {
    let root = mg_testkit::corpus!();
    let module = root.join("data/mod/Neverwinter Chess.mod");
    if !module.is_file() {
        eprintln!("skipped: {} not found", module.display());
        return;
    }
    let dir = mg_testkit::scratch_dir("mg-nasher");
    let project = dir.join("chess");
    let run = |args: &[&str]| {
        let out = crate::mg().args(args).output().unwrap();
        assert!(out.status.success(), "mg {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    };
    run(&["init", module.to_str().unwrap(), project.to_str().unwrap()]);
    assert!(project.join("nasher.cfg").is_file());
    // Lowercase names, as nasher's unpack writes them (the module has Module.ifo).
    assert!(project.join("src/module.ifo.json").is_file());
    assert!(project.join("src/bishop_b.utc.json").is_file());
    let out = dir.join("built.mod");
    run(&[
        "--root",
        root.to_str().unwrap(),
        "--no-user-dir",
        "build",
        project.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    let original = Module::open(&module).unwrap();
    let built = Module::open(&out).unwrap();
    // A compiled script without its source (this module has one) is kept
    // in the project as it is, and packed so.
    let orphans: Vec<ResKey> = (original.keys_of(ResType::NCS))
        .filter(|k| !original.contains(&ResKey::new(k.resref, ResType::NSS)))
        .copied()
        .collect();
    assert!(!orphans.is_empty());
    for k in &orphans {
        let file = project.join(format!("src/{}", k.to_string().to_ascii_lowercase()));
        assert!(file.is_file(), "{} kept", file.display());
        assert_eq!(built.get(k), original.get(k), "{k} packed as it was");
    }
    let mut want: Vec<_> = original.keys().copied().collect();
    let mut have: Vec<_> = built.keys().copied().collect();
    want.sort();
    have.sort();
    assert_eq!(have, want, "the built module has the original's resources, scripts compiled");
    // --ndb: each compiled script's debug file is packed with it.
    let debug = dir.join("debug.mod");
    let (r, p, d) = (root.to_str().unwrap(), project.to_str().unwrap(), debug.to_str().unwrap());
    run(&["--root", r, "--no-user-dir", "build", p, "-o", d, "--ndb"]);
    let with = Module::open(&debug).unwrap();
    let count = |m: &Module, t| m.keys_of(t).count();
    assert_eq!(count(&built, ResType::NDB), 0);
    assert!(count(&with, ResType::NDB) > 10);
    // (Each script compiled; the one kept without its source has none.)
    assert_eq!(count(&with, ResType::NDB), count(&with, ResType::NCS) - orphans.len());
}

/// `mg build --target` of a target that packs a talk table: the table made
/// of its `<name>.tlk.json` as nasher has nwn_tlk make it, byte for byte.
#[test]
fn a_talk_tables_target_builds_as_nasher_builds_it() {
    let project = mg_testkit::scratch_dir("mg-nasher-tlk");
    std::fs::create_dir_all(project.join("src/tlk")).unwrap();
    std::fs::write(
        project.join("nasher.cfg"),
        "[package]\n  [package.rules]\n  \"*\" = \"src\"\n\n[target]\nname = \"default\"\n\
         file = \"m.mod\"\n  [target.sources]\n  include = \"src/**/*.{nss,json}\"\n  \
         exclude = \"**/*.tlk.json\"\n\n[target]\nname = \"tlk\"\nfile = \"tlk/mytlk.tlk\"\n  \
         [target.sources]\n  include = \"src/tlk/*.json\"\n",
    )
    .unwrap();
    let json = project.join("src/tlk/mytlk.tlk.json");
    std::fs::write(
        &json,
        r#"{"language":0,"entries":[{"id":0,"text":"Señor"},{"id":3,"text":"","sound":"vs_x"},{"id":4,"text":"a","soundLen":1.25}]}"#,
    )
    .unwrap();
    let p = project.to_str().unwrap();
    let out = crate::mg().args(["--json", "build", p, "--target", "tlk"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let said: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(said["lines"], 5, "{said}");
    let ours = std::fs::read(project.join("tlk/mytlk.tlk")).unwrap();
    // As nwn_tlk makes it (nasher packs what it makes).
    let tool = mg_testkit::oracle_tool!("nwn_tlk");
    let theirs = project.join("theirs.tlk");
    let made = std::process::Command::new(tool)
        .args(["-i", json.to_str().unwrap(), "-o", theirs.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
    assert_eq!(ours, std::fs::read(&theirs).unwrap());
    // A target with no table of its name among its sources says so.
    std::fs::rename(&json, project.join("src/tlk/other.tlk.json")).unwrap();
    let out = crate::mg().args(["build", p, "--target", "tlk"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no mytlk.tlk.json"));
}
