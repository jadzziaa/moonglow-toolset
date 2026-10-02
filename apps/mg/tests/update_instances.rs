//! `mg update-instances`: Neverwinter Chess's game master (the one creature
//! placed in it; the pieces are made by script), after its blueprint's tag
//! changes, takes it where it stands; the placeables are untouched.

use std::process::Command;

use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

fn objects(m: &Module, list: &str) -> Vec<Struct> {
    let git = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT);
    m.gff(&git).unwrap().unwrap().root.list(list).unwrap_or(&[]).to_vec()
}

fn template(s: &Struct) -> String {
    s.resref("TemplateResRef").map(|r| r.to_string()).unwrap_or_default()
}

#[test]
fn update_instances_remakes_a_blueprints_objects_and_saves() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-update-instances");
    let mut m = Module::open(&root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let master = ResKey::new(ResRef::from_str("gamemaster").unwrap(), ResType::UTC);
    let mut bp = m.gff(&master).unwrap().unwrap();
    bp.root.set("Tag", Value::String(b"MG_MASTER".to_vec()));
    m.set_gff(master, &bp).unwrap();
    let path = dir.join("chess.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let before = objects(&m, "Creature List");
    let placeables = objects(&m, "Placeable List");

    let out = Command::new(env!("CARGO_BIN_EXE_mg"))
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "update-instances"])
        .arg(&path)
        .arg("gamemaster.utc")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let saved = Module::open(&path).unwrap();
    let after = objects(&saved, "Creature List");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "chess: 1");
    assert_eq!((before.len(), after.len()), (1, 1));
    let (b, a) = (&before[0], &after[0]);
    assert_eq!(template(a), "gamemaster");
    assert_eq!(a.string("Tag"), Some(&b"MG_MASTER"[..]));
    // Where it stood (the facing goes through its angle and back).
    for l in ["XPosition", "YPosition", "XOrientation", "YOrientation"] {
        let (x, y) = (a.float(l).unwrap(), b.float(l).unwrap());
        assert!((x - y).abs() < 1e-5, "{l}: {x} {y}");
    }
    assert_eq!(objects(&saved, "Placeable List"), placeables);
}
