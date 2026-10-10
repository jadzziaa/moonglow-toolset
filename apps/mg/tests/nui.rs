//! Real CLI round trips. Every write stays in a disposable module in target.
use mg_core::ResType;
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use serde_json::Value;
use std::process::Command;

#[test]
fn nui_cli_creates_validates_compiles_and_does_not_save_failures() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("nui-cli");
    let path = dir.join("fixture.mod");
    let mut module = Module::new();
    module.set_info(&Gff::new(*b"IFO ")).unwrap();
    module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let run = |verb: &str| {
        Command::new(env!("CARGO_BIN_EXE_mg"))
            .arg("--root")
            .arg(&root)
            .args(["--no-user-dir", "--json", "nui", verb])
            .arg(&path)
            .arg("cli_window")
            .output()
            .unwrap()
    };
    assert!(run("new").status.success());
    let created = std::fs::read(&path).unwrap();
    let checked = run("validate");
    assert!(checked.status.success());
    let json: Value = serde_json::from_slice(&checked.stdout).unwrap();
    assert_eq!(json["runtime_verified"], false);
    assert_eq!(std::fs::read(&path).unwrap(), created, "validate is read-only");
    let compiled = run("generate");
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stdout));
    let mut module = Module::open(&path).unwrap();
    assert!(module.get(&mg_nui::key("cli_window_o", ResType::NCS)).is_none());
    assert!(module.get(&mg_nui::key("cli_window_e", ResType::NCS)).is_some());
    let generated_bytes = std::fs::read(&path).unwrap();
    let repeated = run("generate");
    assert!(repeated.status.success());
    assert_eq!(serde_json::from_slice::<Value>(&repeated.stdout).unwrap()["saved"], false);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        generated_bytes,
        "unchanged generation should not rewrite the archive"
    );
    module.set(
        mg_nui::key("cli_window_e", ResType::NSS),
        b"void main() { InvalidFunction(); }".to_vec(),
    );
    module.save().unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(!run("generate").status.success());
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "compile failure must not save a partial result"
    );
    assert!(!run("new").status.success(), "must reject existing resources");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // Module::save may leave its one rollback backup; all files in this
    // dedicated test directory were produced by this test.
    let expected = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
        .join("target/test-output/nui-cli");
    assert_eq!(dir.canonicalize().unwrap(), expected);
    assert!(!std::fs::symlink_metadata(&dir).unwrap().file_type().is_symlink());
    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        assert!(entry.file_type().unwrap().is_file());
        std::fs::remove_file(entry.path()).unwrap();
    }
    std::fs::remove_dir(&dir).unwrap();
}
