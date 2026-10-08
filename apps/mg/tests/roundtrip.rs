//! `mg roundtrip`: a module saved, read back and written anew is what it
//! was; the module itself is not touched.

use std::process::Command;

use mg_gff::{Gff, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use serde_json::Value as Json;

fn mg(args: &[&str]) -> (bool, Json) {
    let out = Command::new(env!("CARGO_BIN_EXE_mg"))
        .args(["--no-user-dir", "--json"])
        .args(args)
        .output()
        .unwrap();
    let json = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{args:?}: {e}: {}", String::from_utf8_lossy(&out.stderr)));
    (out.status.success(), json)
}

#[test]
fn a_module_comes_back_as_it_was() {
    let dir = mg_testkit::scratch_dir("mg-roundtrip");
    let key = |name: &str| ResKey::from_filename(name).unwrap();
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut utc = Gff::new(*b"UTC ");
    utc.root.set("Tag", Value::String(b"guard".to_vec()));
    // (A field Moonglow has no name for is kept like any other.)
    utc.root.set("SomethingOfAHak", Value::Dword(7));
    m.set_gff(key("guard.utc"), &utc).unwrap();
    m.set(key("things.2da"), &b"2DA V2.0\n\n   Label\n0  one\n1  ****\n"[..]);
    m.set(key("hello.nss"), &b"void main() {}\n"[..]);
    // A file that is no GFF under a GFF's name: kept, and said.
    m.set(key("broken.utp"), &b"not a gff"[..]);
    let path = dir.join("sample.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let before = std::fs::read(&path).unwrap();

    let (ok, report) = mg(&["roundtrip", path.to_str().unwrap()]);
    assert!(ok, "{report}");
    assert_eq!(report["resources"], 5);
    assert_eq!(report["gffs"], 2);
    assert_eq!(report["tables"], 1);
    assert_eq!(report["differences"].as_array().unwrap().len(), 0);
    assert_eq!(report["unread"][0]["resource"], "broken.utp");
    assert_eq!(std::fs::read(&path).unwrap(), before, "the module is not touched");

    // A folder, and the copy kept: it has the same resources.
    let folder = dir.join("folder");
    m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let copy = dir.join("copy.mod");
    let (ok, _) = mg(&["roundtrip", folder.to_str().unwrap(), "--keep", copy.to_str().unwrap()]);
    assert!(ok);
    let kept = Module::open(&copy).unwrap();
    assert_eq!(kept.get(&key("guard.utc")), m.get(&key("guard.utc")));
    // (A copy that is there already is not written over.)
    let out = Command::new(env!("CARGO_BIN_EXE_mg"))
        .args(["roundtrip", folder.to_str().unwrap(), "--keep", copy.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

/// The game's own modules, where there is a game.
#[test]
fn the_game_s_modules_come_back_as_they_were() {
    let root = mg_testkit::corpus!();
    let mut modules = mg_testkit::bundled_modules(&root);
    modules.truncate(4);
    for path in modules {
        let (ok, report) = mg(&["roundtrip", path.to_str().unwrap()]);
        assert!(ok, "{}: {}", path.display(), report["differences"]);
    }
}
