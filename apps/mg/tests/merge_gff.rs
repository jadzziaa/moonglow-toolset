//! `mg merge-gff`: three-way merges of GFF files and of nasher's JSON of
//! them, and the command as git's merge driver.

use std::path::{Path, PathBuf};
use std::process::Command;

use mg_core::ResType;
use mg_gff::{Gff, Struct, Value};
use mg_module::nasher::Conversion;
use serde_json::Value as Json;

fn mg(args: &[&str]) -> (bool, Json) {
    crate::json_out(crate::mg().arg("--json").args(args))
}

fn placeable(template: &str, tag: &str, x: f32) -> Struct {
    let mut s = Struct::new(9);
    s.set("Tag", Value::String(tag.into()));
    s.set("TemplateResRef", Value::ResRef(template.into()));
    s.set("X", Value::Float(x));
    s.set("Y", Value::Float(1.0));
    s.set("Z", Value::Float(0.0));
    s
}

fn area_git(objects: &[Struct]) -> Gff {
    let mut g = Gff::new(*b"GIT ");
    g.root.set("Placeable List", Value::List(objects.to_vec()));
    g.root.set("Creature List", Value::List(vec![]));
    g
}

fn module_ifo(areas: &[&str]) -> Gff {
    let mut g = Gff::new(*b"IFO ");
    let list = areas
        .iter()
        .map(|a| {
            let mut s = Struct::new(6);
            s.set("Area_Name", Value::ResRef((*a).into()));
            s
        })
        .collect();
    g.root.set("Mod_Area_list", Value::List(list));
    g.root.set("Mod_Entry_Area", Value::ResRef(areas[0].into()));
    g
}

fn json_text(restype: ResType, gff: &Gff) -> String {
    Conversion::default().to_source(restype, gff).unwrap()
}

fn read(path: &Path) -> Gff {
    let data = std::fs::read(path).unwrap();
    if data.first() == Some(&b'{') {
        Conversion::default().from_source("x", &data).unwrap()
    } else {
        Gff::read(&data).unwrap()
    }
}

fn put(dir: &Path, name: &str, data: impl AsRef<[u8]>) -> String {
    let path = dir.join(name);
    std::fs::write(&path, data).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn objects_both_sides_placed_are_both_kept() {
    let dir = mg_testkit::scratch_dir("mg-merge-gff");
    let chest = placeable("plc_chest", "CHEST", 1.0);
    let (lamp, rug) = (placeable("plc_lamp", "LAMP", 2.0), placeable("plc_rug", "RUG", 3.0));
    let bytes = |g: &Gff| g.to_bytes().unwrap();
    let base = put(&dir, "base", bytes(&area_git(std::slice::from_ref(&chest))));
    let ours = put(&dir, "ours", bytes(&area_git(&[chest.clone(), lamp.clone()])));
    let theirs = put(&dir, "theirs", bytes(&area_git(&[chest.clone(), rug.clone()])));
    let (ok, m) = mg(&["merge-gff", &base, &ours, &theirs]);
    assert!(ok && m["merged"] == true && m["result"] == "merged", "{m}");
    assert_eq!(read(Path::new(&ours)), area_git(&[chest.clone(), lamp.clone(), rug.clone()]));

    // Both changed one field, each its own way: a failure, OURS as it was,
    // and the conflict named by the object and the field.
    let mut red = chest.clone();
    red.set("Tag", Value::String(b"RED".to_vec()));
    red.set("X", Value::Float(1.0));
    let mut blue = chest.clone();
    blue.set("Tag", Value::String(b"BLUE".to_vec()));
    let ours = put(&dir, "ours", bytes(&area_git(&[red.clone()])));
    let theirs = put(&dir, "theirs", bytes(&area_git(&[blue.clone()])));
    let (ok, m) = mg(&["merge-gff", &base, &ours, &theirs]);
    assert!(!ok && m["merged"] == false && m["written"].is_null(), "{m}");
    assert_eq!(
        m["conflicts"][0],
        serde_json::json!({
            "path": "/Placeable List[plc_chest \"CHEST\" at 1, 1]/Tag",
            "base": "\"CHEST\"", "ours": "\"RED\"", "theirs": "\"BLUE\"",
        })
    );
    assert_eq!(read(Path::new(&ours)), area_git(&[red.clone()]));
    // --prefer takes a side, and the merge is written.
    let out = dir.join("out").to_str().unwrap().to_string();
    let (ok, m) = mg(&["merge-gff", &base, &ours, &theirs, "--prefer", "theirs", "-o", &out]);
    assert!(ok && m["merged"] == true && m["written"] == out.as_str(), "{m}");
    assert_eq!(read(Path::new(&out)), area_git(&[blue]));
    assert_eq!(read(Path::new(&ours)), area_git(&[red]), "-o leaves OURS be");
}

#[test]
fn nasher_json_merges_and_keeps_its_bytes() {
    let dir = mg_testkit::scratch_dir("mg-merge-gff-json");
    let base = put(&dir, "base", json_text(ResType::IFO, &module_ifo(&["start", "town"])));
    let ours = put(&dir, "ours", json_text(ResType::IFO, &module_ifo(&["start", "town", "cave"])));
    let theirs_text = json_text(ResType::IFO, &module_ifo(&["start", "town", "keep"]));
    let theirs = put(&dir, "theirs", &theirs_text);
    let (ok, m) = mg(&["merge-gff", &base, &ours, &theirs]);
    assert!(ok && m["result"] == "merged", "{m}");
    // As nasher writes it.
    let want = json_text(ResType::IFO, &module_ifo(&["start", "town", "cave", "keep"]));
    assert_eq!(std::fs::read_to_string(&ours).unwrap(), want);

    // Nothing changed: OURS keeps every byte, though no writer of ours
    // would write them so (keys unsorted, odd spacing).
    let odd = "{ \"Mod_Entry_Area\": {\"type\": \"resref\", \"value\": \"start\"},\n\"__data_type\": \
               \"IFO \",   \"Mod_Area_list\": {\"type\": \"list\", \"value\": []} }";
    let (b, o, t) = (put(&dir, "b", odd), put(&dir, "o", odd), put(&dir, "t", odd));
    let (ok, m) = mg(&["merge-gff", &b, &o, &t]);
    assert!(ok && m["result"] == "ours" && m["written"].is_null(), "{m}");
    assert_eq!(std::fs::read_to_string(&o).unwrap(), odd);
    // Only theirs changed: THEIRS's bytes.
    let (ok, m) = mg(&["merge-gff", &b, &o, &theirs]);
    assert!(ok && m["result"] == "theirs", "{m}");
    assert_eq!(std::fs::read_to_string(&o).unwrap(), theirs_text);
}

#[test]
fn what_is_no_gff_is_an_error() {
    let dir = mg_testkit::scratch_dir("mg-merge-gff-bad");
    let good = area_git(&[]).to_bytes().unwrap();
    let base = put(&dir, "base", &good);
    let cut = put(&dir, "cut", &good[..good.len() / 2]);
    let out = crate::mg().args(["merge-gff", &base, &cut, &base]).output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("cut") && err.contains("no GFF"), "{err}");
    let bad = put(&dir, "bad", "{\"__data_type\": 3}");
    assert!(!mg(&["merge-gff", &base, &bad, &base]).0);
    assert!(!mg(&["merge-gff", &base, &base, &base, "--prefer", "mine"]).0);
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

/// Two builders' branches, each adding an object to one area and an area
/// to the module, merge with no conflict: what git alone can't do.
#[test]
fn git_merges_two_builders_branches() {
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipped: no git");
        return;
    }
    let root = mg_testkit::scratch_dir("mg-merge-gff-git");
    let repo: PathBuf = root.join("repo");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    ok(&repo, &["init", "-q", "-b", "main"]);
    let driver = format!("'{}' merge-gff %O %A %B --path %P", env!("CARGO_BIN_EXE_mg"));
    ok(&repo, &["config", "merge.mg-gff.name", "Moonglow GFF merge"]);
    ok(&repo, &["config", "merge.mg-gff.driver", &driver]);
    std::fs::write(
        repo.join(".gitattributes"),
        "*.git.json merge=mg-gff\n*.ifo.json merge=mg-gff\n*.git -text merge=mg-gff\n",
    )
    .unwrap();
    let chest = placeable("plc_chest", "CHEST", 1.0);
    let write = |objects: &[Struct], areas: &[&str]| {
        std::fs::write(
            repo.join("src/area1.git.json"),
            json_text(ResType::GIT, &area_git(objects)),
        )
        .unwrap();
        std::fs::write(
            repo.join("src/module.ifo.json"),
            json_text(ResType::IFO, &module_ifo(areas)),
        )
        .unwrap();
        std::fs::write(repo.join("src/area1.git"), area_git(objects).to_bytes().unwrap()).unwrap();
    };
    write(std::slice::from_ref(&chest), &["area1"]);
    ok(&repo, &["add", "-A"]);
    ok(&repo, &["commit", "-q", "-m", "base"]);

    let (lamp, rug) = (placeable("plc_lamp", "LAMP", 2.0), placeable("plc_rug", "RUG", 3.0));
    ok(&repo, &["checkout", "-q", "-b", "anna"]);
    write(&[chest.clone(), lamp.clone()], &["area1", "area2"]);
    ok(&repo, &["commit", "-q", "-am", "anna: a lamp, area2"]);
    ok(&repo, &["checkout", "-q", "main"]);
    ok(&repo, &["checkout", "-q", "-b", "boris"]);
    write(&[chest.clone(), rug.clone()], &["area1", "area3"]);
    ok(&repo, &["commit", "-q", "-am", "boris: a rug, area3"]);

    ok(&repo, &["merge", "-q", "--no-edit", "anna"]);
    // Ours (boris's) first, then what anna added; the JSON as nasher
    // would write it, so the next diff is only what changes.
    let objects = [chest, rug, lamp];
    let text = |name: &str| std::fs::read_to_string(repo.join("src").join(name)).unwrap();
    assert_eq!(text("area1.git.json"), json_text(ResType::GIT, &area_git(&objects)));
    assert_eq!(read(&repo.join("src/area1.git")), area_git(&objects));
    let ifo = module_ifo(&["area1", "area3", "area2"]);
    assert_eq!(text("module.ifo.json"), json_text(ResType::IFO, &ifo));
    let status = git(&repo, &["status", "--porcelain"]);
    assert!(status.stdout.is_empty(), "{}", String::from_utf8_lossy(&status.stdout));
}
