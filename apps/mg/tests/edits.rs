//! `mg set` and `mg apply` on a module made here (no game needed): fields
//! keep their types, a failing edit leaves the module as it was, and what
//! `mg set` prints is a file `mg apply` reads.

use std::path::{Path, PathBuf};

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
    let out = crate::mg_alone().arg("--json").args(args).output().unwrap();
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

/// Setting an object's faction to 0, the PC faction, or taking its faction
/// field away (which the game reads as 0) is done, and warned of as Verify
/// warns of it after; another faction is not; nor a trigger's 0.
#[test]
fn set_warns_of_the_pc_faction() {
    let dir = mg_testkit::scratch_dir("mg-set-pc-faction");
    let path = module(&dir);
    let m = path.to_str().unwrap();
    let notes = |out: &Json| -> Vec<String> {
        let notes = out["notes"].as_array().cloned().unwrap_or_default();
        let notes = notes.iter().filter_map(|n| n.as_str().map(str::to_string));
        notes.filter(|n| n.starts_with("warning:")).collect()
    };
    let (ok, out) = mg(&["set", m, "guard.utc", "FactionID=2"]);
    assert!(ok, "{out}");
    assert!(notes(&out).is_empty(), "{out}");
    let (ok, out) = mg(&["set", m, "guard.utc", "FactionID=0"]);
    assert!(ok, "{out}");
    assert_eq!(guard(&path).get("FactionID"), Some(&Value::Word(0)));
    let said = notes(&out);
    assert_eq!(said.len(), 1, "{out}");
    assert!(said[0].contains("guard.utc /FactionID: puts it in the PC faction (0)"), "{said:?}");
    assert!(said[0].contains("pc-faction"), "{said:?}");
    let (ok, out) = mg(&["set", m, "guard.utc", "--remove", "FactionID"]);
    assert!(ok, "{out}");
    assert!(notes(&out)[0].contains("leaves it no faction"), "{out}");
    // A placed trigger of faction 0 is the owner's decision: not warned of.
    let mut module = Module::open(&path).unwrap();
    let mut area = Gff::new(*b"GIT ");
    let mut trigger = Struct::new(1);
    trigger.set("Faction", Value::Dword(2));
    let mut door = Struct::new(8);
    door.set("Faction", Value::Dword(2));
    area.root.set("TriggerList", Value::List(vec![trigger]));
    area.root.set("Door List", Value::List(vec![door]));
    module.set_gff(key("area.git"), &area).unwrap();
    module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let (ok, out) = mg(&["set", m, "area.git", "/TriggerList[0]/Faction=0"]);
    assert!(ok && notes(&out).is_empty(), "{out}");
    let (ok, out) = mg(&["set", m, "area.git", "/Door List[0]/Faction=0"]);
    assert!(ok, "{out}");
    assert!(notes(&out)[0].contains("area.git /Door List[0]/Faction"), "{out}");
}

/// A tag `mg set`, `mg apply` or `mg areas --set` gives an object that
/// another object of the module has is warned of, as the window's
/// Duplicate Tags says it (the project's `[tags]`, else each tag a
/// command sets); the change is made.
#[test]
fn a_tag_set_twice_is_warned_of() {
    let dir = mg_testkit::scratch_dir("mg-set-tag-twice");
    let path = module(&dir);
    let m = path.to_str().unwrap();
    let mut module = Module::open(&path).unwrap();
    let mut area = Gff::new(*b"ARE ");
    area.root.set("Tag", Value::String(b"INN".to_vec()));
    area.root.set("ResRef", Value::resref(mg_core::ResRef::from_str("inn").unwrap()));
    module.set_gff(key("inn.are"), &area).unwrap();
    let mut placed = Gff::new(*b"GIT ");
    let waypoint = |tag: &[u8]| {
        let mut w = Struct::new(5);
        w.set("Tag", Value::String(tag.to_vec()));
        w
    };
    placed.root.set("WaypointList", Value::List(vec![waypoint(b"WP_DOOR"), waypoint(b"WP_BAR")]));
    module.set_gff(key("inn.git"), &placed).unwrap();
    module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let warnings = |out: &Json| -> Vec<String> {
        let notes = out["notes"].as_array().cloned().unwrap_or_default();
        let notes = notes.iter().filter_map(|n| n.as_str().map(str::to_string));
        notes.filter(|n| n.starts_with("warning:")).collect()
    };
    let twice = |tag: &str| {
        format!(
            "warning: {tag} is now on two objects: a script finds one of them by it \
             (GetObjectByTag)"
        )
    };
    let (ok, out) = mg(&["set", m, "inn.git", "/WaypointList[1]/Tag=WP_STAGE"]);
    assert!(ok && warnings(&out).is_empty(), "{out}");
    let (ok, out) = mg(&["set", m, "inn.git", "/WaypointList[1]/Tag=WP_DOOR", "--dry-run"]);
    assert!(ok, "{out}");
    assert_eq!(warnings(&out), [twice("WP_DOOR")]);
    // mg apply placing a third with WP_STAGE: its blueprint is not the
    // module's, so its tag is taken as the blueprint's, and not a
    // builder's (which only the project's paste_warning = "any" says).
    let file = dir.join("place.json");
    let edits = json!({ "label": "Place", "edits": [
        { "op": "insert_item", "resource": "inn.git", "list": "/WaypointList", "index": 2,
          "item": { "__struct_id": 5, "Tag": { "type": "cexostring", "value": "WP_STAGE" } } },
    ]});
    std::fs::write(&file, edits.to_string()).unwrap();
    let (ok, out) = mg(&["apply", m, file.to_str().unwrap(), "--dry-run"]);
    assert!(ok && warnings(&out).is_empty(), "{out}");
    std::fs::write(dir.join("keep.moonglow.toml"), "[tags]\npaste_warning = \"any\"\n").unwrap();
    let (ok, out) = mg(&["apply", m, file.to_str().unwrap(), "--dry-run"]);
    assert!(ok, "{out}");
    assert_eq!(warnings(&out), [twice("WP_STAGE")]);
    std::fs::remove_file(dir.join("keep.moonglow.toml")).unwrap();
    // An area given a waypoint's tag by mg areas.
    let (ok, out) = mg(&["areas", m, "inn", "--set", "Tag=WP_DOOR", "--dry-run"]);
    assert!(ok, "{out}");
    assert_eq!(warnings(&out), [twice("WP_DOOR")], "{out}");
    // And by mg replace in tags, by a regular expression, of a dry run.
    let (ok, out) =
        mg(&["replace", m, "^WP_(STAGE)$", "WP_DOOR", "--regex", "--only", "tags", "--dry-run"]);
    assert!(ok, "{out}");
    assert_eq!(out["strings"].as_array().unwrap().len(), 1, "{out}");
    assert_eq!(warnings(&out), [twice("WP_DOOR")], "{out}");
}

/// `mg replace --only` takes the plain fields (tags, resrefs, variables,
/// fields, or all) beside the players' text, which is what it looks in
/// without; `--regex` replaces by a regular expression and its groups,
/// and one that isn't one is an error.
#[test]
fn replace_takes_plain_fields_and_regular_expressions() {
    let dir = mg_testkit::scratch_dir("mg-replace-fields");
    let path = module(&dir);
    let m = path.to_str().unwrap();
    let (ok, out) = mg(&["replace", m, "guard", "warden", "--dry-run"]);
    assert!(ok, "{out}");
    assert!(out["strings"].as_array().unwrap().is_empty(), "the tag isn't players' text: {out}");
    let (ok, out) = mg(&["replace", m, "^(g)uard$", "${1}ate", "--regex", "--only", "tags"]);
    assert!(ok, "{out}");
    assert_eq!(out["replaced"], json!(1), "{out}");
    assert_eq!(guard(&path).get("Tag"), Some(&Value::String(b"gate".to_vec())));
    let (ok, out) = mg(&["replace", m, "dr(aft", "x", "--regex", "--only", "all"]);
    assert!(!ok);
    assert!(out["error"].as_str().unwrap().contains("dr(aft"), "{out}");
    let (ok, out) = mg(&["replace", m, "draft", "final", "--only", "fields"]);
    assert!(ok, "{out}");
    assert_eq!(guard(&path).get("Comment"), Some(&Value::String(b"final".to_vec())));
    let (ok, out) = mg(&["replace", m, "x", "y", "--only", "tag"]);
    assert!(!ok && out["error"].as_str().unwrap().contains("tag: not names"), "{out}");
}
