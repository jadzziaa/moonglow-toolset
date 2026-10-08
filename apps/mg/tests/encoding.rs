//! A module whose hak has an `encoding.2da` (the character each byte of
//! the game's text stands for): `mg` reads and writes the module's text
//! by it, and as Windows-1252 without it.

use std::path::Path;
use std::process::Command;

use mg_core::{Gender, Language, LocString};
use mg_erf::ErfWriter;
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use serde_json::Value as Json;

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

fn guard(module: &Path) -> Struct {
    Module::open(module).unwrap().gff(&key("guard.utc")).unwrap().unwrap().root
}

fn mg(root: &Path, user: Option<&Path>, args: &[&str]) -> (bool, Json) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_mg"));
    c.args(["--json", "--root", root.to_str().unwrap()]);
    match user {
        Some(u) => c.args(["--user-dir", u.to_str().unwrap()]),
        None => c.arg("--no-user-dir"),
    };
    let out = c.args(args).output().unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or(Json::Null);
    (out.status.success(), json)
}

#[test]
fn a_module_s_encoding_table_is_the_text_of_its_commands() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-encoding");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    // The wiki's Turkish example: byte 0xF0 is "ğ", not "ð".
    let mut rows = String::from("2DA V2.0\n\n  Codepoint\n");
    for row in 0..240 {
        rows.push_str(&format!("{row} ****\n"));
    }
    rows.push_str("240 0x11f\n");
    let mut hak = ErfWriter::new(*b"HAK ");
    let table = key("encoding.2da");
    hak.add(table.resref, table.restype, rows.into_bytes()).unwrap();
    std::fs::write(user.join("hak/mg_turkish.hak"), hak.to_bytes().unwrap()).unwrap();

    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    let mut listed = Struct::new(8);
    listed.set("Mod_Hak", Value::String(b"mg_turkish".to_vec()));
    info.root.set("Mod_HakList", Value::List(vec![listed]));
    let name = |bytes: &[u8]| {
        Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, bytes.to_vec()))
    };
    info.root.set("Mod_Name", name(b"Da\xf0 Evi"));
    m.set_info(&info).unwrap();
    let mut utc = Gff::new(*b"UTC ");
    utc.root.set("Tag", Value::String(b"guard".to_vec()));
    utc.root.set("FirstName", name(b"Da\xf0"));
    m.set_gff(key("guard.utc"), &utc).unwrap();
    let path = dir.join("turkish.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let module = path.to_str().unwrap();

    // With the hak found: the table's letters, read and written.
    let (ok, info) = mg(&root, Some(&user), &["info", module]);
    assert!(ok);
    assert_eq!(info["name"], "Dağ Evi");
    let (ok, found) = mg(&root, Some(&user), &["find", module, "--name", "Dağ"]);
    assert!(ok);
    assert_eq!(found["found"][0]["name"], "Dağ", "{found}");
    let (ok, set) = mg(&root, Some(&user), &["set", module, "guard.utc", "Tag=dağ é"]);
    assert!(ok, "{set}");
    assert_eq!(guard(&path).string("Tag").unwrap(), b"da\xf0 \xe9");
    // (A letter the table gave up has no byte.)
    let (ok, _) = mg(&root, Some(&user), &["set", module, "guard.utc", "Tag=ð"]);
    assert!(!ok);
    let (ok, _) = mg(&root, Some(&user), &["replace", module, "Dağ", "Yağ"]);
    assert!(ok);
    let first = guard(&path).locstring("FirstName").unwrap().clone();
    assert_eq!(first.get(Language::ENGLISH, Gender::Male).unwrap(), b"Ya\xf0");

    // Without it (the hak not found): Windows-1252, as ever.
    let (ok, info) = mg(&root, None, &["info", module]);
    assert!(ok);
    assert_eq!(info["name"], "Yað Evi");
    let (ok, _) = mg(&root, None, &["set", module, "guard.utc", "Tag=ğ"]);
    assert!(!ok);
    let (ok, _) = mg(&root, None, &["set", module, "guard.utc", "Tag=ð"]);
    assert!(ok);
    assert_eq!(guard(&path).string("Tag").unwrap(), b"\xf0");
}
