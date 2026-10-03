//! `mg areas`: a module's areas listed and narrowed by its filters, and
//! fields, flags and variables set on those chosen; a dry run and a change
//! that can't be made change nothing.

use std::process::Command;

use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::new(ResRef::from_str(name).unwrap(), t)
}

/// Two caves (underground) and an inn.
fn module(path: &std::path::Path) {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for (name, title, tileset, flags) in [
        ("cave1", "Wolf Cave", "tdc01", 0x7u32),
        ("cave2", "Bear Cave", "tdc01", 0x3 | 0x100),
        ("inn", "Apple Inn", "tin01", 0x1),
    ] {
        let mut are = Gff::new(*b"ARE ");
        let title = LocString::from_text(Language::ENGLISH, Gender::Male, title);
        are.root.set("Name", Value::LocString(title));
        are.root.set("Tileset", Value::resref(ResRef::from_str(tileset).unwrap()));
        are.root.set("Flags", Value::Dword(flags));
        are.root.set("SunFogAmount", Value::Byte(2));
        m.set_gff(key(name, ResType::ARE), &are).unwrap();
        let mut git = Gff::new(*b"GIT ");
        let mut props = Struct::new(100);
        props.set("MusicDay", Value::Int(3));
        git.root.set("AreaProperties", Value::Struct(props));
        m.set_gff(key(name, ResType::GIT), &git).unwrap();
    }
    m.save_as(&ModuleLocation::Archive(path.into())).unwrap();
}

#[test]
fn areas_are_listed_and_changed_together() {
    let dir = mg_testkit::scratch_dir("mg-areas");
    let path = dir.join("caves.mod");
    module(&path);
    let run = |args: &[&str]| {
        let out =
            Command::new(env!("CARGO_BIN_EXE_mg")).arg("areas").arg(&path).args(args).output();
        let out = out.unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        (out.status.success(), text(&out.stdout), text(&out.stderr))
    };
    let are =
        |name: &str| Module::open(&path).unwrap().gff(&key(name, ResType::ARE)).unwrap().unwrap();
    let music = |name: &str| {
        let git = Module::open(&path).unwrap().gff(&key(name, ResType::GIT)).unwrap().unwrap();
        git.root.child("AreaProperties").unwrap().integer("MusicDay")
    };

    // Listed: all, then the underground ones.
    let (ok, all, _) = run(&[]);
    assert!(ok && all.lines().count() == 3, "{all}");
    let (ok, under, notes) = run(&["--underground"]);
    assert!(ok, "{notes}");
    let listed: Vec<&str> = under.lines().map(|l| l.split('\t').next().unwrap()).collect();
    assert_eq!(listed, ["cave1", "cave2"]);
    assert!(under.contains("Wolf Cave") && under.contains("underground"));

    // A dry run lists the changes and changes nothing.
    let bytes = std::fs::read(&path).unwrap();
    let change = ["--underground", "--set", "MusicDay=57", "--set", "SunFogAmount=9"];
    let (ok, would, notes) = run(&[&change[..], &["--var", "nMusic=57", "--dry-run"]].concat());
    assert!(ok, "{notes}");
    assert!(would.contains("cave1\tMusicDay\t3 -> 57") && would.contains("cave2\tvar nMusic"));
    assert!(notes.contains("would change 2 of 2"), "{notes}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    // Changed: the caves, each keeping its other flags; not the inn.
    let (ok, _, notes) =
        run(&[&change[..], &["--var", "nMusic=57", "--set", "Natural=no"]].concat());
    assert!(ok, "{notes}");
    assert_eq!((music("cave1"), music("cave2"), music("inn")), (Some(57), Some(57), Some(3)));
    assert_eq!(are("cave1").root.integer("Flags"), Some(0x3));
    assert_eq!(are("cave2").root.integer("Flags"), Some(0x3 | 0x100));
    assert_eq!(are("cave2").root.get("SunFogAmount"), Some(&Value::Byte(9)));
    assert_eq!(are("inn").root.get("SunFogAmount"), Some(&Value::Byte(2)));
    assert_eq!(are("cave1").root.list("VarTable").unwrap()[0].integer("Value"), Some(57));

    // Every area only when asked; a value that doesn't fit changes nothing.
    let bytes = std::fs::read(&path).unwrap();
    let (ok, _, notes) = run(&["--set", "SunFogAmount=1"]);
    assert!(!ok && notes.contains("--all"), "{notes}");
    let (ok, _, notes) = run(&["--all", "--set", "SunFogAmount=1", "--set", "SunFogAmount=999"]);
    assert!(!ok && notes.contains("SunFogAmount"), "{notes}");
    let (ok, _, notes) = run(&["nowhere", "--set", "SunFogAmount=1"]);
    assert!(!ok && notes.contains("nowhere"), "{notes}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    // JSON: the areas chosen and what changed.
    let out = Command::new(env!("CARGO_BIN_EXE_mg"))
        .args(["--json", "areas"])
        .arg(&path)
        .args(["inn", "--remove-var", "nMusic", "--set", "Underground=yes"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["areas"][0]["resref"], "inn");
    assert_eq!(json["areas"][0]["name"], "Apple Inn");
    assert_eq!(json["changes"].as_array().unwrap().len(), 1, "the inn has no nMusic to delete");
    assert_eq!(json["changes"][0]["field"], "Underground");
    assert_eq!(are("inn").root.integer("Flags"), Some(0x3));
}
