//! A module whose hak has an `encoding.2da` (the character each byte of
//! the game's text stands for): `mg` reads and writes the module's text
//! by it, and as Windows-1252 without it.

use std::path::Path;

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
    let out = crate::mg_on(root, user).arg("--json").args(args).output().unwrap();
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

    // A lone file of the module's by its table, given the module: as JSON
    // and back.
    let lone = dir.join("guard.utc");
    let bytes = Module::open(&path).unwrap().get(&key("guard.utc")).unwrap().to_vec();
    std::fs::write(&lone, &bytes).unwrap();
    let lone_path = lone.to_str().unwrap();
    let (ok, json) = mg(&root, Some(&user), &["gff", lone_path, "--module", module]);
    assert!(ok, "{json}");
    assert!(json.to_string().contains("Yağ"), "{json}");
    let (ok, json) = mg(&root, Some(&user), &["gff", lone_path]);
    assert!(ok && json.to_string().contains("Yað"), "without the module: {json}");
    let (text, back) = (dir.join("guard.utc.json"), dir.join("back.utc"));
    let (text, back) = (text.to_str().unwrap(), back.to_str().unwrap());
    let (ok, _) = mg(&root, Some(&user), &["gff", lone_path, "--module", module, "-o", text]);
    assert!(ok);
    let (ok, _) = mg(&root, Some(&user), &["gff", text, "--module", module, "-o", back]);
    assert!(ok);
    assert_eq!(std::fs::read(&back).unwrap(), bytes, "the same bytes back");

    // Its name published to NWSync in the table's letters.
    let repository = dir.join("nwsync");
    let (ok, made) =
        mg(&root, Some(&user), &["nwsync", module, repository.to_str().unwrap(), "--with-module"]);
    assert!(ok, "{made}");
    let sha = made["sha1"].as_str().unwrap();
    let manifest = std::fs::read(repository.join(format!("manifests/{sha}.json"))).unwrap();
    let manifest: Json = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(manifest["module_name"], "Yağ Evi", "{manifest}");

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

/// The lines `mg refs` shows of a module's scripts are read in the
/// codepage the game client reads them in (GitHub issue 27): a Polish
/// game's (its settings.tml's language) Windows-1250, where 0xB9 is "ą";
/// Windows-1252 without one. The scripts' bytes are never changed.
#[test]
fn mg_refs_reads_a_script_s_line_in_the_game_s_codepage() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-refs-codepage");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let settings = "[game]\n\t[game.language]\n\t\tcodepage = \"\"\n\t\toverride = \"pl\"\n";
    std::fs::write(user.join("settings.tml"), settings).unwrap();
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let source = b"void main() { SpeakString(\"Id\xb9\"); ExecuteScript(\"next_one\"); }\n";
    m.set(key("polish.nss"), source.to_vec());
    let path = dir.join("polish.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let module = path.to_str().unwrap();
    let line = |user: Option<&Path>| {
        let (ok, refs) = mg(&root, user, &["refs", module, "next_one.nss"]);
        assert!(ok, "{refs}");
        refs["script_strings"][0]["text"].as_str().unwrap_or_default().to_string()
    };
    let polish = "void main() { SpeakString(\"Idą\"); ExecuteScript(\"next_one\"); }";
    assert_eq!(line(Some(&user)), polish);
    let western = "void main() { SpeakString(\"Id¹\"); ExecuteScript(\"next_one\"); }";
    assert_eq!(line(None), western);
    assert_eq!(Module::open(&path).unwrap().get(&key("polish.nss")), Some(&source[..]));
}
