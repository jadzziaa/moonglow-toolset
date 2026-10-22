//! `mg claims`: the project's claims table listed, areas claimed,
//! released and marked merged (The Domains of Dread's builder proposal,
//! C6 and C7), and `mg verify` holding merged areas to it.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

/// A module folder of two areas, its project naming a claims table
/// (not made yet), and the builder `ab`.
fn claimed_folder(name: &str) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name).join("project");
    let mut m = Module::new();
    let mut ifo = Gff::new(*b"IFO ");
    ifo.root.set("Mod_Tag", Value::String(b"KREZK".to_vec()));
    m.set_info(&ifo).unwrap();
    for (area, tag) in [("kz_square", "KZ_SQUARE"), ("kz_gate", "KZ_GATE")] {
        let mut are = Gff::new(*b"ARE ");
        are.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
        m.set_gff(ResKey::new(ResRef::from_str(area).unwrap(), ResType::ARE), &are).unwrap();
    }
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join("moonglow.toml"), "[claims]\nfile = \"builders/claims.2da\"\n")
        .unwrap();
    dir
}

fn run(args: &[&str], dir: &std::path::Path, rest: &[&str]) -> std::process::Output {
    crate::mg().args(args).arg(dir).args(rest).output().unwrap()
}

#[test]
fn claims_are_listed_claimed_released_and_merged() {
    let dir = claimed_folder("mg-claims");
    // No code yet: a claim says how to have one.
    let out = run(&["claims"], &dir, &["claim", "kz_square"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("mg builder-code"));
    let out = crate::mg().arg("builder-code").arg(&dir).arg("ab").output().unwrap();
    assert!(out.status.success());
    // Claimed: the table made, under builders/.
    let out = run(&["claims"], &dir, &["claim", "kz_square"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "kz_square: claimed by ab");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("builders/claims.2da written: commit it")
    );
    let table = std::fs::read_to_string(dir.join("builders/claims.2da")).unwrap();
    assert!(table.starts_with("2DA V2.0\n\n    Area             Builder  Status   Date        Tag\n0   kz_square        ab       claimed  "), "{table}");
    // Listed, with the module's areas no row names.
    let out = run(&["--json", "claims"], &dir, &[]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["code"], "ab");
    assert_eq!(json["file"], "builders/claims.2da");
    let areas = json["areas"].as_array().unwrap();
    assert_eq!(areas.len(), 2, "{json}");
    assert_eq!(areas[0]["area"], "kz_square");
    assert_eq!(areas[0]["standing"], "mine");
    assert_eq!(areas[0]["status"], "claimed");
    assert_eq!(areas[1]["area"], "kz_gate");
    assert_eq!(areas[1]["standing"], "free");
    assert_eq!(areas[1]["in_module"], true);
    // Merged: the Tag the module has recorded, the holder kept.
    let out = run(&["claims"], &dir, &["merged", "kz_square", "kz_gate"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "kz_square: merged, as KZ_SQUARE\nkz_gate: merged, as KZ_GATE"
    );
    let out = run(&["claims"], &dir, &["list"]);
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("kz_square\tyours\tab since ") && lines[0].ends_with("\tmerged"));
    assert!(lines[1].starts_with("kz_gate\tfree") && lines[1].ends_with("\tmerged"), "{text}");
    // Another builder's claim: refused, unless forced.
    std::fs::write(dir.join(".moonglow/user.toml"), "[builder]\ncode = \"cd\"\n").unwrap();
    let out = run(&["claims"], &dir, &["claim", "kz_square"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("kz_square is claimed by ab (since "),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run(&["claims"], &dir, &["release", "kz_square"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("claimed by ab, not by cd"));
    let out = run(&["claims"], &dir, &["release", "kz_square", "--force"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let out = run(&["--json", "claims"], &dir, &[]);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["areas"][0]["builder"], serde_json::Value::Null);
    assert_eq!(json["areas"][0]["status"], "merged", "released, still merged");
    // A merged area retagged: Verify fails, naming the table's row.
    let mut m = Module::open(&dir).unwrap();
    let key = ResKey::new(ResRef::from_str("kz_gate").unwrap(), ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    are.root.set("Tag", Value::String(b"KZ_GATE_OLD".to_vec()));
    m.set_gff(key, &are).unwrap();
    m.save().unwrap();
    let out = run(&["--json", "--no-user-dir", "verify"], &dir, &[]);
    assert!(!out.status.success(), "a merged area's Tag changed is an error");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("claims-tag") && text.contains("merged as KZ_GATE"), "{text}");
}

#[test]
fn a_project_without_a_claims_table_says_how_to_name_one() {
    let dir = claimed_folder("mg-claims-none");
    std::fs::write(dir.join("moonglow.toml"), "").unwrap();
    let out = run(&["claims"], &dir, &[]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("[claims] file = "));
    let out = run(&["claims"], &dir.join("nowhere"), &[]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a project's folder"));
}
