//! `mg set` and `mg apply` on a module made here (no game needed): fields
//! keep their types, a failing edit leaves the module as it was, and what
//! `mg set` prints is a file `mg apply` reads.

use std::path::{Path, PathBuf};
use std::process::Command;

use mg_core::{LocString, LocStringKey, StrRef};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use serde_json::{Value as Json, json};

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

/// A module with a guard (a tag, a name from the talk table, one class).
fn module(dir: &Path) -> PathBuf {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("Tag", Value::String(b"guard".to_vec()));
    guard.root.set("FirstName", Value::LocString(LocString::from_strref(StrRef(77))));
    guard.root.set("Comment", Value::String(b"draft".to_vec()));
    let mut class = Struct::new(2);
    class.set("Class", Value::Int(4));
    class.set("ClassLevel", Value::Short(1));
    guard.root.set("ClassList", Value::List(vec![class]));
    m.set_gff(key("guard.utc"), &guard).unwrap();
    let path = dir.join("keep.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

fn guard(module: &Path) -> Struct {
    Module::open(module).unwrap().gff(&key("guard.utc")).unwrap().unwrap().root
}

fn mg(args: &[&str]) -> (bool, Json) {
    let out = Command::new(env!("CARGO_BIN_EXE_mg"))
        .args(["--no-user-dir", "--json"])
        .args(args)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.trim().is_empty(), "{args:?}: stderr {stderr}");
    let json = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{args:?}: {e}: {}", String::from_utf8_lossy(&out.stdout)));
    (out.status.success(), json)
}

#[test]
fn set_keeps_each_field_s_type() {
    let dir = mg_testkit::scratch_dir("mg-set");
    let path = module(&dir);
    let m = path.to_str().unwrap();
    let before = guard(&path);
    let fields = ["Tag=GUARD", "/ClassList[0]/ClassLevel=5", "FirstName=Bob", "Wis=12"];

    // A dry run changes nothing, and prints the edits.
    let (ok, dry) = mg(&[&["set", m, "guard.utc"][..], &fields, &["--dry-run"]].concat());
    assert!(ok, "{dry}");
    assert_eq!((&dry["dry_run"], &dry["edits"]), (&json!(true), &json!(4)));
    assert_eq!(dry["resources"], json!(["guard.utc"]));
    assert_eq!(dry["command"]["edits"][1]["field"], "/ClassList[0]/ClassLevel");
    assert_eq!(dry["command"]["edits"][1]["value"], json!({"type": "short", "value": 5}));
    assert_eq!(guard(&path), before);

    let (ok, done) = mg(&[&["set", m, "guard.utc"][..], &fields].concat());
    assert!(ok, "{done}");
    assert_eq!(done["dry_run"], false);
    let g = guard(&path);
    assert_eq!(g.get("Tag"), Some(&Value::String(b"GUARD".to_vec())));
    // The class level stays a short; the name keeps its talk-table
    // reference and takes the text as its English; a field the creature
    // lacked takes the type the game's creatures give it.
    assert_eq!(g.list("ClassList").unwrap()[0].get("ClassLevel"), Some(&Value::Short(5)));
    let mut name = LocString::from_strref(StrRef(77));
    name.strings.push((LocStringKey(0), b"Bob".to_vec()));
    assert_eq!(g.get("FirstName"), Some(&Value::LocString(name)));
    assert_eq!(g.get("Wis"), Some(&Value::Byte(12)));
    // Nothing else moved.
    assert_eq!(g.get("Comment"), before.get("Comment"));
    assert_eq!(g.list("ClassList").unwrap()[0].get("Class"), Some(&Value::Int(4)));

    // A field of no known type needs one; a value must fit its type.
    let fails = |args: &[&str]| {
        let (ok, json) = mg(&[&["set", m, "guard.utc"][..], args].concat());
        assert!(!ok, "{args:?}: {json}");
        json["error"].as_str().unwrap().to_string()
    };
    assert!(fails(&["MyField=3"]).contains("give its type"));
    assert!(fails(&["/ClassList[0]/ClassLevel=70000"]).contains("does not fit a short"));
    assert!(fails(&["/ClassList[4]/ClassLevel=1"]).contains("no struct at /ClassList[4]"));
    assert!(fails(&["Tag"]).contains("FIELD=VALUE"));
    assert!(fails(&["--remove", "Nope"]).contains("no field Nope"));
    assert_eq!(guard(&path), g, "a refused change leaves the module as it was");

    let (ok, _) = mg(&["set", m, "guard.utc", "MyField:int=3", "--remove", "Comment"]);
    assert!(ok);
    let g = guard(&path);
    assert_eq!((g.get("MyField"), g.get("Comment")), (Some(&Value::Int(3)), None));
}

#[test]
fn apply_takes_all_of_a_file_or_none() {
    let dir = mg_testkit::scratch_dir("mg-apply");
    let path = module(&dir);
    let m = path.to_str().unwrap();
    let before = guard(&path);

    // What `mg set --dry-run` prints is a file `mg apply` reads.
    let (_, dry) = mg(&["set", m, "guard.utc", "Tag=CAPTAIN", "--dry-run"]);
    let file = dir.join("tag.json");
    std::fs::write(&file, dry["command"].to_string()).unwrap();
    let (ok, done) = mg(&["apply", m, file.to_str().unwrap()]);
    assert!(ok, "{done}");
    assert_eq!((&done["label"], &done["edits"]), (&json!("Set Tag"), &json!(1)));
    assert_eq!(guard(&path).get("Tag"), Some(&Value::String(b"CAPTAIN".to_vec())));

    // Every kind of edit.
    let file = dir.join("all.json");
    let edits = json!({ "label": "Promote", "edits": [
        { "op": "insert_item", "resource": "guard.utc", "list": "/ClassList", "index": 1,
          "item": { "__struct_id": 2, "Class": { "type": "int", "value": 7 },
                    "ClassLevel": { "type": "short", "value": 2 } } },
        { "op": "remove_item", "resource": "guard.utc", "item": "/ClassList[0]" },
        { "op": "remove_field", "resource": "guard.utc", "field": "/Comment" },
        { "op": "set_resource", "resource": "on_spawn.nss", "text": "void main() { }\n" },
        { "op": "set_resource", "resource": "icon.tga", "base64": "AAEC/w==" },
    ]});
    std::fs::write(&file, edits.to_string()).unwrap();
    let (ok, done) = mg(&["apply", m, file.to_str().unwrap()]);
    assert!(ok, "{done}");
    assert_eq!(done["resources"], json!(["guard.utc", "on_spawn.nss", "icon.tga"]));
    let module = Module::open(&path).unwrap();
    let g = guard(&path);
    let classes = g.list("ClassList").unwrap();
    assert_eq!((classes.len(), classes[0].get("Class")), (1, Some(&Value::Int(7))));
    assert_eq!(g.get("Comment"), None);
    assert_eq!(module.get(&key("on_spawn.nss")), Some(&b"void main() { }\n"[..]));
    assert_eq!(module.get(&key("icon.tga")), Some(&[0u8, 1, 2, 255][..]));

    // One edit that does not apply: none of the file does.
    let file = dir.join("bad.json");
    let edits = json!({ "edits": [
        { "op": "remove_resource", "resource": "icon.tga" },
        { "op": "remove_item", "resource": "guard.utc", "item": "/ClassList[9]" },
    ]});
    std::fs::write(&file, edits.to_string()).unwrap();
    let (ok, failed) = mg(&["apply", m, file.to_str().unwrap()]);
    assert!(!ok);
    assert!(failed["error"].as_str().unwrap().contains("/ClassList: no item 9"), "{failed}");
    assert!(Module::open(&path).unwrap().contains(&key("icon.tga")));

    // A file that is not a command says where it is wrong.
    std::fs::write(&file, r#"{"edits": [{"op": "set_field", "resource": "guard.utc"}]}"#).unwrap();
    let (ok, failed) = mg(&["apply", m, file.to_str().unwrap()]);
    assert!(!ok);
    assert!(failed["error"].as_str().unwrap().contains("edits[0]: no \"field\""), "{failed}");
    let _ = before;
}
