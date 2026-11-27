//! `mg merge-gff`: three-way merges of GFF files and of nasher's JSON of
//! them, and the command as git's merge driver.

use std::path::{Path, PathBuf};
use std::process::Command;

use mg_core::ResType;
use mg_gff::{Gff, Struct, Value};
use mg_module::nasher::Conversion;

use crate::mg_json as mg;

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
    // (Its edit in the conflict file is the first: nothing else merged.)
    assert_eq!(
        m["conflicts"][0],
        serde_json::json!({
            "path": "/Placeable List[plc_chest \"CHEST\" at 1, 1]/Tag",
            "base": "\"CHEST\"", "ours": "\"RED\"", "theirs": "\"BLUE\"", "edits": [0],
        })
    );
    assert_eq!(m["conflict_file"], format!("{ours}.conflicts").as_str());
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

/// NWNT (a project's with `gffFormat = nwnt`) merges as JSON does, and is
/// written as `nwn_nwnt` writes it.
#[test]
fn nwnt_merges_and_is_written_as_nwnt() {
    let dir = mg_testkit::scratch_dir("mg-merge-gff-nwnt");
    let conv = Conversion::default();
    let nwnt = |areas: &[&str]| conv.to_nwnt_source(ResType::IFO, &module_ifo(areas)).unwrap();
    let base = put(&dir, "base", nwnt(&["start", "town"]));
    let ours = put(&dir, "ours", nwnt(&["start", "town", "cave"]));
    let theirs = put(&dir, "theirs", nwnt(&["start", "town", "keep"]));
    let (ok, m) = mg(&["merge-gff", &base, &ours, &theirs]);
    assert!(ok && m["result"] == "merged", "{m}");
    assert_eq!(std::fs::read_to_string(&ours).unwrap(), nwnt(&["start", "town", "cave", "keep"]));
    // A conflict is said, as in JSON, and nothing is written.
    let (red, blue) = (placeable("plc_chest", "RED", 1.0), placeable("plc_chest", "BLUE", 1.0));
    let git = |o: Struct| conv.to_nwnt_source(ResType::GIT, &area_git(&[o])).unwrap();
    let b = put(&dir, "b", git(placeable("plc_chest", "CHEST", 1.0)));
    let (o, t) = (put(&dir, "o", git(red)), put(&dir, "t", git(blue)));
    let (ok, m) = mg(&["merge-gff", &b, &o, &t]);
    assert!(!ok && m["conflicts"][0]["theirs"] == "\"BLUE\"", "{m}");
}

/// The float places a merge is written with come from the nasher project
/// `--project` names, where it isn't above the file.
#[test]
fn a_project_elsewhere_gives_its_settings() {
    let dir = mg_testkit::scratch_dir("mg-merge-gff-project");
    let project = dir.join("module");
    std::fs::create_dir_all(project.join(".nasher")).unwrap();
    std::fs::write(
        project.join("nasher.cfg"),
        "[package]\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
    )
    .unwrap();
    std::fs::write(project.join(".nasher/user.cfg"), "truncateFloats = 2\n").unwrap();
    let files = dir.join("elsewhere");
    std::fs::create_dir_all(&files).unwrap();
    let chest = placeable("plc_chest", "CHEST", 1.23456);
    let git = |o: &[Struct]| json_text(ResType::GIT, &area_git(o));
    let base = put(&files, "base", git(std::slice::from_ref(&chest)));
    let ours = put(&files, "ours", git(&[chest.clone(), placeable("plc_lamp", "LAMP", 2.0)]));
    let theirs = put(&files, "theirs", git(&[chest.clone(), placeable("plc_rug", "RUG", 3.0)]));
    let ours_text = std::fs::read_to_string(&ours).unwrap();
    let (ok, m) = mg(&["merge-gff", &base, &ours, &theirs]);
    assert!(ok, "{m}");
    assert!(std::fs::read_to_string(&ours).unwrap().contains("1.2346"), "nasher's 4 without one");
    std::fs::write(&ours, ours_text).unwrap();
    let p = project.to_str().unwrap();
    let (ok, m) = mg(&["merge-gff", &base, &ours, &theirs, "--project", p]);
    assert!(ok, "{m}");
    let text = std::fs::read_to_string(&ours).unwrap();
    assert!(text.contains("1.23,") || text.contains("1.23\n"), "{text}");
    // A folder without a nasher.cfg is refused.
    assert!(!mg(&["merge-gff", &base, &ours, &theirs, "--project", files.to_str().unwrap()]).0);
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

/// A conflict file's edits, those of the conflicts not in `keep` left out.
fn conflict_edits(file: &str, m: &serde_json::Value, keep: &[usize]) -> serde_json::Value {
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    let mut out_of: Vec<u64> = Vec::new();
    for (k, c) in m["conflicts"].as_array().unwrap().iter().enumerate() {
        if !keep.contains(&k) {
            out_of.extend(c["edits"].as_array().unwrap().iter().map(|e| e.as_u64().unwrap()));
        }
    }
    let edits = json["edits"].as_array().unwrap().iter().enumerate();
    let kept: Vec<_> =
        edits.filter(|(i, _)| !out_of.contains(&(*i as u64))).map(|(_, e)| e.clone()).collect();
    json["edits"] = serde_json::json!(kept);
    json
}

/// On a conflict, the edits that make OURS the merge are written beside
/// it for `mg apply`: what merged, then theirs' side of each conflict.
/// Applied to a module whose area is OURS, with any of the conflicts' edits
/// left out, they give the merge with ours' side of those: three
/// conflicts in one list (a tag both changed, an object one removed and
/// the other changed, one both moved), with theirs' change to another
/// object and an object each side added.
#[test]
fn a_conflict_file_takes_either_side_of_each_conflict() {
    let dir = mg_testkit::scratch_dir("mg-merge-gff-conflicts");
    let (chest, lamp, rug, table) = (
        placeable("plc_chest", "CHEST", 1.0),
        placeable("plc_lamp", "LAMP", 2.0),
        placeable("plc_rug", "RUG", 3.0),
        placeable("plc_table", "TABLE", 4.0),
    );
    let with = |s: &Struct, label: &str, v: Value| {
        let mut s = s.clone();
        s.set(label, v);
        s
    };
    let tag = |s: &Struct, t: &str| with(s, "Tag", Value::String(t.into()));
    let base = area_git(&[chest.clone(), lamp.clone(), rug.clone(), table.clone()]);
    let ours = area_git(&[
        tag(&chest, "RED"),
        with(&rug, "X", Value::Float(3.5)),
        table.clone(),
        placeable("plc_stool", "STOOL", 5.0),
    ]);
    let theirs = area_git(&[
        tag(&chest, "BLUE"),
        with(&lamp, "Locked", Value::Byte(1)),
        with(&rug, "X", Value::Float(3.75)),
        tag(&table, "DESK"),
        placeable("plc_bed", "BED", 6.0),
    ]);
    let bytes = |g: &Gff| g.to_bytes().unwrap();
    let (b, t) = (put(&dir, "base", bytes(&base)), put(&dir, "theirs", bytes(&theirs)));
    let module = dir.join("module");
    std::fs::create_dir_all(&module).unwrap();
    let area = module.join("area1.git");
    let ours_path = area.to_str().unwrap().to_string();
    std::fs::write(&area, bytes(&ours)).unwrap();
    let (ok, m) = mg(&["merge-gff", &b, &ours_path, &t]);
    assert!(!ok && m["conflicts"].as_array().unwrap().len() == 3, "{m}");
    assert_eq!(read(&area), ours, "OURS as it was");
    let file = format!("{ours_path}.conflicts");
    assert_eq!(m["conflict_file"], file.as_str());
    // Each edit says what it is for.
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let notes: Vec<&str> =
        json["edits"].as_array().unwrap().iter().map(|e| e["note"].as_str().unwrap()).collect();
    assert!(notes[0].starts_with("merged"), "{notes:?}");
    assert!(notes.iter().any(|n| n.starts_with("conflict 2 of 3, theirs' side:")), "{notes:?}");
    // (The folder is a module, and the file in it no resource.)
    let module_dir = module.to_str().unwrap();
    let edits = dir.join("edits.json");
    let edits_path = edits.to_str().unwrap();
    for keep in 0..8usize {
        let kept: Vec<usize> = (0..3).filter(|k| keep & (1 << k) != 0).collect();
        std::fs::write(&area, bytes(&ours)).unwrap();
        std::fs::write(&edits, conflict_edits(&file, &m, &kept).to_string()).unwrap();
        let (ok, applied) = mg(&["apply", module_dir, edits_path]);
        assert!(ok, "{applied}");
        let choose = |i: usize| {
            if kept.contains(&i) {
                mg_module::merge::Prefer::Theirs
            } else {
                mg_module::merge::Prefer::Ours
            }
        };
        let want = mg_module::merge::merge_choosing(&base, &ours, &theirs, &choose).gff;
        assert_eq!(read(&area), want, "theirs' side of {kept:?}");
    }
    // Whole, it is --prefer theirs; without a conflict's edits, --prefer ours.
    let out = dir.join("out").to_str().unwrap().to_string();
    std::fs::write(&area, bytes(&ours)).unwrap();
    assert!(mg(&["merge-gff", &b, &ours_path, &t, "--prefer", "theirs", "-o", &out]).0);
    assert!(mg(&["apply", module_dir, &file]).0);
    assert_eq!(read(&area), read(Path::new(&out)));
    std::fs::write(&area, bytes(&ours)).unwrap();
    assert!(mg(&["merge-gff", &b, &ours_path, &t, "--prefer", "ours", "-o", &out]).0);
    assert_eq!(read(Path::new(&out)).root.list("Placeable List").unwrap().len(), 5);
}

/// Two builders adding lines to one conversation (a nasher project's JSON)
/// both keep theirs: theirs numbered after ours, their links with them.
#[test]
fn lines_both_builders_added_to_a_conversation_are_both_kept() {
    use mg_module::dialog::{Kind, Parent, add_link, add_node, new_dialog, outline};
    let dir = mg_testkit::scratch_dir("mg-merge-gff-dlg");
    let mut base = new_dialog();
    let hello = add_node(&mut base, Parent::Root, "Hello.");
    add_node(&mut base, Parent::Node(Kind::Entry, hello), "Hi.");
    let mut ours = base.clone();
    add_node(&mut ours, Parent::Node(Kind::Entry, hello), "Bye.");
    let theirs_adds = |g: &mut Gff| {
        let r = add_node(g, Parent::Node(Kind::Entry, hello), "Who are you?");
        let e = add_node(g, Parent::Node(Kind::Reply, r), "Nobody.");
        add_link(g, Parent::Node(Kind::Entry, e), r);
    };
    let mut theirs = base.clone();
    theirs_adds(&mut theirs);
    let text = |g: &Gff| json_text(ResType::DLG, g);
    let (b, o, t) = (
        put(&dir, "base", text(&base)),
        put(&dir, "ours", text(&ours)),
        put(&dir, "theirs", text(&theirs)),
    );
    let (ok, m) = mg(&["merge-gff", &b, &o, &t]);
    assert!(ok && m["result"] == "merged", "{m}");
    let mut want = ours.clone();
    theirs_adds(&mut want);
    assert_eq!(outline(&read(Path::new(&o))), outline(&want));
    assert_eq!(std::fs::read_to_string(&o).unwrap(), text(&want));
}

/// A conflict in a git merge leaves the conflict file beside the file in
/// the work tree, which `mg apply` on the project takes: the merge, and no
/// other file changed.
#[test]
fn a_conflict_in_a_git_merge_is_settled_by_its_conflict_file() {
    if Command::new("git").arg("--version").output().is_err() {
        eprintln!("skipped: no git");
        return;
    }
    let root = mg_testkit::scratch_dir("mg-merge-gff-git-conflict");
    let repo: PathBuf = root.join("repo");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    ok(&repo, &["init", "-q", "-b", "main"]);
    let driver = format!("'{}' merge-gff %O %A %B --path %P", env!("CARGO_BIN_EXE_mg"));
    ok(&repo, &["config", "merge.mg-gff.driver", &driver]);
    std::fs::write(repo.join(".gitattributes"), "*.git.json merge=mg-gff\n").unwrap();
    std::fs::write(
        repo.join("nasher.cfg"),
        "[package]\n[target]\nname = \"m\"\nfile = \"m.mod\"\n[target.sources]\ninclude = \"src/**/*.json\"\n",
    )
    .unwrap();
    let file = repo.join("src/area1.git.json");
    let write = |objects: &[Struct]| {
        std::fs::write(&file, json_text(ResType::GIT, &area_git(objects))).unwrap();
    };
    let chest = placeable("plc_chest", "CHEST", 1.0);
    let tagged = |t: &str| {
        let mut s = chest.clone();
        s.set("Tag", Value::String(t.into()));
        s
    };
    write(std::slice::from_ref(&chest));
    ok(&repo, &["add", "-A"]);
    ok(&repo, &["commit", "-q", "-m", "base"]);
    let (lamp, rug) = (placeable("plc_lamp", "LAMP", 2.0), placeable("plc_rug", "RUG", 3.0));
    ok(&repo, &["checkout", "-q", "-b", "anna"]);
    write(&[tagged("RED"), lamp.clone()]);
    ok(&repo, &["commit", "-q", "-am", "anna"]);
    ok(&repo, &["checkout", "-q", "main"]);
    ok(&repo, &["checkout", "-q", "-b", "boris"]);
    write(&[tagged("BLUE"), rug.clone()]);
    ok(&repo, &["commit", "-q", "-am", "boris"]);
    assert!(!git(&repo, &["merge", "-q", "--no-edit", "anna"]).status.success());
    // (As JSON reads it: its fields sorted.)
    let as_read = |g: Gff| {
        Conversion::default().from_source("x", json_text(ResType::GIT, &g).as_bytes()).unwrap()
    };
    assert_eq!(read(&file), as_read(area_git(&[tagged("BLUE"), rug.clone()])), "ours in the file");
    let conflicts = repo.join("src/area1.git.json.conflicts");
    assert!(conflicts.is_file());
    let (applied, out) = mg(&["apply", repo.to_str().unwrap(), conflicts.to_str().unwrap()]);
    assert!(applied, "{out}");
    assert_eq!(read(&file), as_read(area_git(&[tagged("RED"), rug, lamp])));
    let status = git(&repo, &["status", "--porcelain"]);
    let status = String::from_utf8_lossy(&status.stdout);
    let mut lines: Vec<&str> = status.lines().collect();
    lines.sort();
    assert_eq!(lines, ["?? src/area1.git.json.conflicts", "UU src/area1.git.json"], "{status}");
}

/// A line deleted on one side while the other changed lines is one
/// conflict of the lines, which the conflict file takes theirs' side of
/// like any other: applied, it is `--prefer theirs`.
#[test]
fn a_conversation_s_lines_conflict_is_settled_by_its_conflict_file() {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog, node_mut, remove};
    let dir = mg_testkit::scratch_dir("mg-merge-gff-dlg-conflict");
    let mut base = new_dialog();
    let hello = add_node(&mut base, Parent::Root, "Hello.");
    for text in ["One.", "Two.", "Three."] {
        add_node(&mut base, Parent::Node(Kind::Entry, hello), text);
    }
    let mut ours = base.clone();
    remove(&mut ours, Parent::Node(Kind::Entry, hello), 0);
    let mut theirs = base.clone();
    node_mut(&mut theirs, Kind::Reply, 1).unwrap().set("Comment", Value::String(b"two".to_vec()));
    theirs.root.set("DelayReply", Value::Dword(2));
    let module = dir.join("module");
    std::fs::create_dir_all(&module).unwrap();
    let conv = module.join("talk.dlg");
    let bytes = |g: &Gff| g.to_bytes().unwrap();
    std::fs::write(&conv, bytes(&ours)).unwrap();
    let (b, t) = (put(&dir, "base", bytes(&base)), put(&dir, "theirs", bytes(&theirs)));
    let o = conv.to_str().unwrap().to_string();
    let (ok, m) = mg(&["merge-gff", &b, &o, &t]);
    assert!(!ok && m["conflicts"][0]["path"] == "/(lines)", "{m}");
    assert_eq!(m["conflicts"][0]["ours"], "EntryList 1, ReplyList 2, StartingList 1", "{m}");
    let out = dir.join("out").to_str().unwrap().to_string();
    assert!(mg(&["merge-gff", &b, &o, &t, "--prefer", "theirs", "-o", &out]).0);
    let (ok, applied) = mg(&["apply", module.to_str().unwrap(), &format!("{o}.conflicts")]);
    assert!(ok, "{applied}");
    assert_eq!(read(&conv), read(Path::new(&out)));
    assert_eq!(read(&conv).root.dword("DelayReply"), Some(2));
}

/// A list's order both changed and a value in it both changed: the
/// order's edits put items in other places, so the two are one group in
/// the conflict file, taken together; applied whole it is still `--prefer
/// theirs`, and without them `--prefer ours`.
#[test]
fn conflicts_whose_edits_overlap_are_taken_together() {
    let dir = mg_testkit::scratch_dir("mg-merge-gff-overlap");
    let var = |name: &str, value: i32| {
        let mut s = Struct::new(0);
        s.set("Name", Value::String(name.into()));
        s.set("Type", Value::Dword(1));
        s.set("Value", Value::Int(value));
        s
    };
    let utc = |vars: &[Struct]| {
        let mut g = Gff::new(*b"UTC ");
        g.root.set("Tag", Value::String(b"GUARD".to_vec()));
        g.root.set("VarTable", Value::List(vars.to_vec()));
        g
    };
    let (a, c) = (var("a", 1), var("c", 3));
    let base = utc(&[a.clone(), var("b", 2), c.clone()]);
    let ours = utc(&[var("b", 20), a.clone(), c.clone()]);
    let mut theirs = utc(&[c.clone(), a.clone(), var("b", 21)]);
    theirs.root.set("Tag", Value::String(b"GATE".to_vec()));
    let module = dir.join("module");
    std::fs::create_dir_all(&module).unwrap();
    let file = module.join("guard.utc");
    let bytes = |g: &Gff| g.to_bytes().unwrap();
    std::fs::write(&file, bytes(&ours)).unwrap();
    let (b, t) = (put(&dir, "base", bytes(&base)), put(&dir, "theirs", bytes(&theirs)));
    let o = file.to_str().unwrap().to_string();
    let (ok, m) = mg(&["merge-gff", &b, &o, &t]);
    assert!(!ok, "{m}");
    let paths: Vec<&str> =
        m["conflicts"].as_array().unwrap().iter().map(|c| c["path"].as_str().unwrap()).collect();
    assert_eq!(paths, ["/VarTable[b, 1]/Value", "/VarTable/(order)"]);
    assert_eq!(m["conflicts"][0]["edits"], m["conflicts"][1]["edits"], "{m}");
    let conflicts = format!("{o}.conflicts");
    let text = std::fs::read_to_string(&conflicts).unwrap();
    assert!(text.contains("conflicts 1 and 2 of 2, theirs' side of each"), "{text}");
    let out = dir.join("out").to_str().unwrap().to_string();
    let module_dir = module.to_str().unwrap();
    for (keep, prefer) in [(&[0usize, 1][..], "theirs"), (&[][..], "ours")] {
        std::fs::write(&file, bytes(&ours)).unwrap();
        assert!(mg(&["merge-gff", &b, &o, &t, "--prefer", prefer, "-o", &out]).0);
        let edits = dir.join("edits.json");
        std::fs::write(&edits, conflict_edits(&conflicts, &m, keep).to_string()).unwrap();
        let (ok, applied) = mg(&["apply", module_dir, edits.to_str().unwrap()]);
        assert!(ok, "{applied}");
        assert_eq!(read(&file), read(Path::new(&out)), "--prefer {prefer}");
    }
}
