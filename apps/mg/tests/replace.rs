//! `mg replace`: "champion" replaced in Contest of Champions' text; a dry
//! run lists the strings and changes nothing.

use std::process::Command;

use mg_module::text::{Options, TextKind, find};
use mg_module::{Module, ModuleLocation};

#[test]
fn replace_lists_then_changes_the_modules_text() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-replace");
    let path = dir.join("contest.mod");
    let mut m = Module::open(&root.join("data/mod/Contest Of Champions 0492.mod")).unwrap();
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let o = Options { match_case: false, whole_word: true };
    // The word the module uses most in its own text.
    let word = "champion";
    let hits = find(&m, word, o, &TextKind::ALL);
    assert!(!hits.is_empty(), "the module names {word}");
    let bytes = std::fs::read(&path).unwrap();
    let run = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_mg"))
            .args(["--root", root.to_str().unwrap(), "--no-user-dir", "replace", "--whole-word"])
            .args(extra)
            .arg(&path)
            .args([word, "victor"])
            .output()
            .unwrap()
    };
    let dry = run(&["--dry-run"]);
    assert!(dry.status.success(), "{}", String::from_utf8_lossy(&dry.stderr));
    assert_eq!(String::from_utf8_lossy(&dry.stdout).lines().count(), hits.len());
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "a dry run changes nothing");
    let out = run(&[]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let saved = Module::open(&path).unwrap();
    assert!(find(&saved, word, o, &TextKind::ALL).is_empty());
    assert!(!find(&saved, "victor", o, &TextKind::ALL).is_empty());
}
