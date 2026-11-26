//! Timing views of the review of November 2026's fixes V15, V16 and V17
//! (the deferred list's "performance fixes without a timing view"): a
//! second Find with the haks and the game, Delete's question, and Rename,
//! each at the size its finding names and again at a small size beside it.
//! They are run by hand, in release, and print `TIMING name = ms` lines
//! (and the machine's load):
//! `cargo test --release -p mg-module --test timing -- --ignored --nocapture --test-threads 1`.
//! The worlds are made here, of resources of our own: no game data.

use std::time::{Duration, Instant};

use mg_core::{Codepage, Language, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_module::rename::{self, UsageIndex};
use mg_module::search::{Kept, Query, Search};
use mg_resman::{LayerClass, MemContainer, ResKey, ResMan, priority};
use mg_rules::GameData;

fn load() -> String {
    std::fs::read_to_string("/proc/loadavg").unwrap_or_default().trim().to_string()
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn report(name: &str, took: &[Duration]) {
    let list: Vec<String> = took.iter().map(|d| format!("{:.2}", ms(*d))).collect();
    println!("TIMING {name} = {} ms (load {})", list.join(" "), load());
}

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::parse(name, t).unwrap()
}

fn text(s: String) -> Value {
    Value::String(s.into_bytes())
}

/// `n` item blueprints of their own: each a few fields.
fn blueprints(prefix: &str, n: usize) -> MemContainer {
    let mut c = MemContainer::new();
    for i in 0..n {
        let mut g = Gff::new(*b"UTI ");
        g.root.set("Tag", text(format!("{prefix}_{i}")));
        g.root.set(
            "TemplateResRef",
            Value::resref(ResRef::from_str(&format!("{prefix}{i}")).unwrap()),
        );
        g.root.set("BaseItem", Value::Int((i % 100) as i32));
        let mut var = Struct::new(0);
        var.set("Name", text(format!("v{}", i % 50)));
        var.set("Type", Value::Dword(1));
        var.set("Value", Value::Int(i as i32));
        g.root.set("VarTable", Value::List(vec![var]));
        c.insert(key(&format!("{prefix}{i}"), ResType::UTI), g.to_bytes().unwrap());
    }
    c
}

/// V15: the second Find with "haks and game" on keeps what it read of the
/// layers' blueprints, and drops what is no longer read from where it
/// was: that was a walk of every wanted key for each one kept.
fn second_find(label: &str, game_items: usize, hak_items: usize) {
    let mut rm = ResMan::new();
    rm.add(priority::KEY, "base", LayerClass::Key, blueprints("g", game_items));
    rm.add(priority::HAK_USER, "hak:big", LayerClass::Erf, blueprints("h", hak_items));
    let game = GameData::new(rm, mg_tlk::Tlk::new(Language::ENGLISH));
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let q = Query {
        haks_and_game: true,
        ..Query::new(Search::Variable { name: "v7".into(), kind: None, value: None })
    };
    let mut kept = Kept::default();
    let t = Instant::now();
    let first = kept.search(&m, &q, Some(&game), Codepage::WINDOWS_1252);
    let first_took = t.elapsed();
    assert!(first.len() >= (game_items + hak_items) / 50 - 1, "{} found", first.len());
    let mut again = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        let hits = kept.search(&m, &q, Some(&game), Codepage::WINDOWS_1252);
        again.push(t.elapsed());
        assert_eq!(kept.read, 0, "nothing is read again");
        assert_eq!(hits.len(), first.len());
    }
    report(&format!("V15 {label}: first Find"), &[first_took]);
    report(&format!("V15 {label}: second Find"), &again);
}

#[test]
#[ignore]
fn v15_a_second_find_with_the_haks_and_the_game() {
    println!("load at start {}", load());
    second_find("100 blueprints", 50, 50);
    second_find("10,000 of the game, 35,000 of a hak", 10_000, 35_000);
}

/// A world of `areas` areas with `objects` placeables each (a tag of its
/// own each), two doors each, one leading into another area's first
/// object; 4,000 scripts, one of which, `s_target`, several areas and
/// scripts name (and one more spells in a string).
fn world(areas: usize, objects: usize, scripts: usize) -> Module {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for a in 0..areas {
        let mut git = Gff::new(*b"GIT ");
        let mut list = Vec::with_capacity(objects);
        for i in 0..objects {
            let mut p = Struct::new(9);
            p.set("Tag", text(format!("A{a}_{i}")));
            p.set("TemplateResRef", Value::resref(ResRef::from_str("plc_chest1").unwrap()));
            list.push(p);
        }
        git.root.set("Placeable List", Value::List(list));
        let mut doors = Vec::new();
        for d in 0..2 {
            let mut door = Struct::new(8);
            door.set("Tag", text(format!("D{a}_{d}")));
            let to = (a * 7 + d + 1) % areas;
            door.set("LinkedTo", text(format!("A{to}_0")));
            doors.push(door);
        }
        git.root.set("Door List", Value::List(doors));
        m.set_gff(key(&format!("area{a}"), ResType::GIT), &git).unwrap();
        let mut are = Gff::new(*b"ARE ");
        are.root.set("Tag", text(format!("AREA{a}")));
        let script = if a % 1000 == 3 || a % 20 == 7 {
            "s_target".to_string()
        } else {
            format!("s_enter{}", a % scripts)
        };
        are.root.set("OnEnter", Value::resref(ResRef::from_str(&script).unwrap()));
        m.set_gff(key(&format!("area{a}"), ResType::ARE), &are).unwrap();
    }
    for s in 0..scripts {
        let body = match s % 800 {
            1 => "void main() { ExecuteScript(\"s_target\", OBJECT_SELF); }\n".to_string(),
            _ => {
                format!("void main() {{ int n = {s}; SendMessageToPC(OBJECT_SELF, \"hello\"); }}\n")
            }
        };
        m.set(key(&format!("s_enter{s}"), ResType::NSS), body.into_bytes());
    }
    m.set(key("s_target", ResType::NSS), b"void main() {}\n".to_vec());
    m
}

/// V16: Delete's question, what leads into the areas about to be
/// deleted: for each of their own tags, every tag of the other areas was
/// walked.
fn delete_question(label: &str, m: &Module, deleted: usize) {
    let mut index = UsageIndex::default();
    let t = Instant::now();
    index.refresh(m);
    report(&format!("V16 {label}: the index's first read"), &[t.elapsed()]);
    let mut took = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        let mut found = 0;
        for a in 0..deleted {
            found += index.links_into(ResRef::from_str(&format!("area{}", a * 3)).unwrap()).len();
        }
        took.push(t.elapsed());
        assert!(found > 0 || deleted < 3, "{found} doors lead there");
    }
    report(&format!("V16 {label}: Delete's question of {deleted} areas"), &took);
}

#[test]
#[ignore]
fn v16_delete_s_question_in_a_large_world() {
    println!("load at start {}", load());
    delete_question("50 areas", &world(50, 150, 40), 20);
    delete_question("5,000 areas of 150 objects", &world(5_000, 150, 4_000), 20);
}

/// V17: the window's Rename of a script (with the strings that spell it):
/// the module copied for the staging, and the references read: all of
/// the module's resources, or those the index the window keeps names.
fn rename_script(label: &str, m: &Module) {
    let mut index = UsageIndex::default();
    let t = Instant::now();
    index.refresh(m);
    report(&format!("V17 {label}: the index's first read (Find References)"), &[t.elapsed()]);
    let from = key("s_target", ResType::NSS);
    let to = ResRef::from_str("s_renamed").unwrap();
    let mut took = Vec::new();
    let mut shown = 0;
    for _ in 0..3 {
        let t = Instant::now();
        // The window's: the references it keeps say which to read.
        let holders = Some(index.holders(from, true)); // AFTER
        let mut staged = m.clone();
        let report = rename::rename_among(&mut staged, from, to, true, holders.as_deref()); // AFTER
        let report = report.unwrap();
        took.push(t.elapsed());
        shown = report.references;
        assert!(staged.contains(&key("s_renamed", ResType::NSS)));
    }
    assert!(shown >= 3, "{shown} references");
    report(&format!("V17 {label}: Rename of a script, {shown} references"), &took);
}

#[test]
#[ignore]
fn v17_rename_in_a_large_world() {
    println!("load at start {}", load());
    rename_script("50 areas", &world(50, 20, 40));
    rename_script("5,000 areas, 4,000 scripts", &world(5_000, 20, 4_000));
}

/// P19: opening a nasher project of `files` source files, whose target
/// takes in two kinds of them and leaves one out: the whole open, and
/// the walk for the files the includes leave out (what a save that
/// wrote uncovered files and a scan that found a file gone do again).
fn open_project(label: &str, files: usize, left: usize) {
    use mg_module::ModuleLocation;
    use mg_module::nasher::Project;
    let root = mg_testkit::scratch_dir("timing-nasher-project");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.set(key("start", ResType::NSS), b"void main() {}\n".to_vec());
    m.save_as(&ModuleLocation::Project { root: root.clone(), target: "default".into() }).unwrap();
    let cfg = "[package]\n  [package.sources]\n  include = \"src/**/*.nss\"\n  include = \"src/**/*.{json,jui}\"\n  exclude = \"src/old/**/*\"\n\n  [package.rules]\n  \"*\" = \"src\"\n\n[target]\nname = \"default\"\nfile = \"timing.mod\"\ndescription = \"\"\n";
    std::fs::write(root.join("nasher.cfg"), cfg).unwrap();
    let src = root.join("src");
    for d in 0..(files / 500 + 1) {
        std::fs::create_dir_all(src.join(format!("dir{d}"))).unwrap();
    }
    std::fs::create_dir_all(src.join("old")).unwrap();
    for i in 0..files {
        let body = format!("void main() {{ int n = {i}; }}\n");
        std::fs::write(src.join(format!("dir{}/s{i}.nss", i / 500)), body).unwrap();
    }
    // (Left out by the includes: a text model's folder of files of a kind
    // Moonglow reads that none of them takes in.)
    for i in 0..left {
        std::fs::write(src.join(format!("old/m{i}.nss")), "void main() {}\n").unwrap();
        std::fs::write(src.join(format!("dir0/w{i}.txt")), "n\n").unwrap();
    }
    let mut opened = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        let (project, resources) = Project::open(&root, None).unwrap();
        opened.push(t.elapsed());
        assert!(resources.len() >= files, "{} resources", resources.len());
        drop(project);
    }
    report(&format!("P19 {label}: Project::open of {files} sources"), &opened);
    let (project, _) = Project::open(&root, None).unwrap();
    let mut walked = Vec::new();
    for _ in 0..3 {
        let t = Instant::now();
        let out = project.left_out();
        walked.push(t.elapsed());
        let _ = out;
    }
    report(
        &format!(
            "P19 {label}: the walk for the files left out ({} found)",
            project.left_out().len()
        ),
        &walked,
    );
}

#[test]
#[ignore]
fn p19_opening_a_large_nasher_project() {
    println!("load at start {}", load());
    open_project("small", 200, 5);
    open_project("large", 30_000, 100);
}
