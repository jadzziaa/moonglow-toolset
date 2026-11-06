//! `mg merge-claims`: a project's claims table merged by area, and the
//! command as git's merge driver.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::mg_json as mg;

const BASE: &str = "2DA V2.0\n\n    Area             Builder  Status   Date\n\
                    0   kz_square        ab       claimed  2026-10-22\n\
                    1   kz_bridge        cd       claimed  2026-10-20\n";

/// The base with rows added after its last, each `(label, area, code)`.
fn with(rows: &[(usize, &str, &str)]) -> String {
    let mut text = BASE.to_string();
    for (n, area, code) in rows {
        text.push_str(&format!("{n:<4}{area:<17}{code:<9}claimed  2026-11-06\n"));
    }
    text
}

fn put(dir: &Path, name: &str, data: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, data).unwrap();
    path.to_str().unwrap().to_string()
}

/// Two new claims are both kept, written over OURS; an area both changed
/// fails and writes nothing, unless --prefer takes a side; -o writes
/// elsewhere.
#[test]
fn new_claims_merge_and_conflicts_are_said() {
    let dir = mg_testkit::scratch_dir("mg-merge-claims");
    let base = put(&dir, "base", BASE);
    let ours = put(&dir, "ours", &with(&[(2, "kz_inn", "ab")]));
    let theirs = put(&dir, "theirs", &with(&[(2, "kz_mill", "cd")]));
    let (ok, j) = mg(&["merge-claims", &base, &ours, &theirs]);
    assert!(ok && j["merged"] == true && j["written"] == ours.as_str(), "{j}");
    let merged = std::fs::read_to_string(&ours).unwrap();
    assert_eq!(merged, with(&[(2, "kz_inn", "ab"), (3, "kz_mill", "cd")]));

    // kz_square handed on by ours, released by theirs.
    let ours = put(&dir, "ours", &BASE.replace("kz_square        ab", "kz_square        ef"));
    let theirs = put(&dir, "theirs", &BASE.replace("ab       claimed", "****     free   "));
    let (ok, j) = mg(&["merge-claims", &base, &ours, &theirs]);
    assert!(!ok && j["merged"] == false && j["written"].is_null(), "{j}");
    assert_eq!(j["conflicts"][0]["area"], "kz_square", "{j}");
    let before = std::fs::read_to_string(&ours).unwrap();
    let out = dir.join("out").to_str().unwrap().to_string();
    let (ok, j) = mg(&["merge-claims", &base, &ours, &theirs, "--prefer", "theirs", "-o", &out]);
    assert!(ok && j["written"] == out.as_str(), "{j}");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), std::fs::read_to_string(&theirs).unwrap());
    assert_eq!(std::fs::read_to_string(&ours).unwrap(), before, "-o leaves OURS be");
    assert!(!mg(&["merge-claims", &base, &ours, &theirs, "--prefer", "mine"]).0);
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    let home = dir.parent().unwrap().join("git-home");
    std::fs::create_dir_all(&home).unwrap();
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.join("gitconfig"))
        .env("GIT_AUTHOR_NAME", "Builder")
        .env("GIT_AUTHOR_EMAIL", "builder@example.com")
        .env("GIT_COMMITTER_NAME", "Builder")
        .env("GIT_COMMITTER_EMAIL", "builder@example.com")
        .output()
        .unwrap()
}

fn ok(dir: &Path, args: &[&str]) {
    let out = git(dir, args);
    assert!(
        out.status.success(),
        "git {args:?}: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Two builders' branches, each claiming a new area: git's own merge
/// calls them a conflict (both a line under number 2 at the end), the
/// driver merges them.
#[test]
fn git_merges_two_builders_claims() {
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipped: no git");
        return;
    }
    let root = mg_testkit::scratch_dir("mg-merge-claims-git");
    let repo: PathBuf = root.join("repo");
    std::fs::create_dir_all(repo.join("claims")).unwrap();
    ok(&repo, &["init", "-q", "-b", "main"]);
    let table = repo.join("claims/claims.2da");
    std::fs::write(&table, BASE).unwrap();
    ok(&repo, &["add", "-A"]);
    ok(&repo, &["commit", "-q", "-m", "base"]);
    ok(&repo, &["checkout", "-q", "-b", "anna"]);
    std::fs::write(&table, with(&[(2, "kz_inn", "ab")])).unwrap();
    ok(&repo, &["commit", "-q", "-am", "anna claims kz_inn"]);
    ok(&repo, &["checkout", "-q", "main"]);
    ok(&repo, &["checkout", "-q", "-b", "boris"]);
    std::fs::write(&table, with(&[(2, "kz_mill", "cd")])).unwrap();
    ok(&repo, &["commit", "-q", "-am", "boris claims kz_mill"]);

    // Without the driver: a conflict.
    assert!(!git(&repo, &["merge", "-q", "--no-edit", "anna"]).status.success());
    ok(&repo, &["merge", "--abort"]);

    let driver = format!("'{}' merge-claims %O %A %B --path %P", env!("CARGO_BIN_EXE_mg"));
    ok(&repo, &["config", "merge.mg-claims.name", "Moonglow claims table merge"]);
    ok(&repo, &["config", "merge.mg-claims.driver", &driver]);
    std::fs::write(repo.join(".gitattributes"), "claims/claims.2da merge=mg-claims\n").unwrap();
    ok(&repo, &["merge", "-q", "--no-edit", "anna"]);
    let merged = std::fs::read_to_string(&table).unwrap();
    assert_eq!(merged, with(&[(2, "kz_mill", "cd"), (3, "kz_inn", "ab")]));
    let status = git(&repo, &["status", "--porcelain"]);
    let status = String::from_utf8_lossy(&status.stdout).replace("?? .gitattributes\n", "");
    assert!(status.is_empty(), "{status}");
}
