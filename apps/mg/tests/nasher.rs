//! `mg init` and `mg build`: a shipped module put into a nasher project and
//! built from it gives the module's resources back, scripts compiled.

use std::process::Command;

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
    let mg = env!("CARGO_BIN_EXE_mg");
    let run = |args: &[&str]| {
        let out = Command::new(mg).args(args).output().unwrap();
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
    // Compiled scripts without source (this module has one) aren't kept.
    let mut want: Vec<_> = original
        .keys()
        .filter(|k| {
            k.restype != ResType::NCS || original.contains(&ResKey::new(k.resref, ResType::NSS))
        })
        .copied()
        .collect();
    let mut have: Vec<_> = built.keys().copied().collect();
    want.sort();
    have.sort();
    assert_eq!(have, want, "the built module has the original's resources, scripts compiled");
}
