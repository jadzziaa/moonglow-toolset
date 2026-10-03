//! `mg plugin` and `mg verify --plugins`, on a module made here and the
//! plugin host's fixture plugins: a command's edits are saved (or printed),
//! a form is answered from the arguments, a plugin's checks count in
//! verify.

use std::path::{Path, PathBuf};
use std::process::Command;

use mg_gff::{Gff, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use serde_json::{Value as Json, json};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/mg-plugin/tests/fixtures")
}

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

/// A module with a guard whose tag is in lower case.
fn module(dir: &Path) -> PathBuf {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("Tag", Value::String(b"gate_guard".to_vec()));
    m.set_gff(key("guard.utc"), &guard).unwrap();
    let path = dir.join("keep.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

fn tag(module: &Path) -> String {
    let g = Module::open(module).unwrap().gff(&key("guard.utc")).unwrap().unwrap();
    String::from_utf8(g.root.string("Tag").unwrap().to_vec()).unwrap()
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
fn plugins_are_listed_and_checked() {
    let fixtures = fixtures();
    let (ok, listed) = mg(&["plugin", "list", fixtures.to_str().unwrap()]);
    assert!(ok, "{listed}");
    let plugins = listed["plugins"].as_array().unwrap();
    let tags = plugins.iter().find(|p| p["id"] == "example.tag-conventions").unwrap();
    assert_eq!(
        tags["commands"][0],
        json!({
            "id": "fix-tags", "title": "Fix Creature Tags",
            "hint": "Upper-case every creature's tag", "key": null,
        })
    );
    assert_eq!(tags["checks"][0]["severity"], "warning");

    // One plugin's folder, whose code and manifest agree.
    let one = fixtures.join("tag-conventions");
    let (ok, checked) = mg(&["plugin", "check", one.to_str().unwrap()]);
    assert!(ok, "{checked}");
    assert_eq!(checked["plugins"], json!([{ "plugin": "example.tag-conventions", "faults": [] }]));
    // One whose code and manifest do not: it fails, saying how.
    let broken = fixtures.join("broken");
    let (ok, checked) = mg(&["plugin", "check", broken.to_str().unwrap()]);
    assert!(!ok);
    assert_eq!(checked["plugins"][0]["faults"].as_array().unwrap().len(), 3, "{checked}");
}

#[test]
fn a_plugin_s_command_runs_on_a_module() {
    let dir = mg_testkit::scratch_dir("mg-plugin-run");
    let path = module(&dir);
    let m = path.to_str().unwrap();
    let plugin = fixtures().join("tag-conventions");
    let plugin = plugin.to_str().unwrap();

    // A dry run prints the edits (a file `mg apply` reads) and saves nothing.
    let (ok, dry) = mg(&["plugin", "run", m, plugin, "fix-tags", "--dry-run"]);
    assert!(ok, "{dry}");
    assert_eq!((&dry["label"], &dry["edits"]), (&json!("Upper-case creature tags"), &json!(1)));
    assert_eq!(dry["command"]["edits"][0]["value"]["value"], "GATE_GUARD");
    assert_eq!(dry["log"], json!([{ "level": "info", "text": "1 creature tags changed" }]));
    assert_eq!(tag(&path), "gate_guard");

    let (ok, done) = mg(&["plugin", "run", m, plugin, "fix-tags"]);
    assert!(ok, "{done}");
    assert_eq!(done["resources"], json!(["guard.utc"]));
    assert_eq!(tag(&path), "GATE_GUARD");

    // A form is answered from the arguments: its defaults, and what is
    // given; a question, with --yes.
    let forms = fixtures().join("forms");
    let forms = forms.to_str().unwrap();
    let (ok, declined) = mg(&["plugin", "run", m, forms, "rename", "--answer", "prefix=SGT_"]);
    assert!(ok, "{declined}");
    assert_eq!(declined["edits"], 0);
    assert!(declined["log"][0]["text"].as_str().unwrap().ends_with("no (pass --yes to agree)"));
    let (ok, done) = mg(&["plugin", "run", m, forms, "rename", "--answer", "prefix=SGT_", "--yes"]);
    assert!(ok, "{done}");
    assert_eq!(tag(&path), "SGT_GATE_GUARD");
    // An answer nobody asked for, or of the wrong kind, is an error, and
    // the module stays as it was.
    let fails = |args: &[&str]| {
        let (ok, json) = mg(&[&["plugin", "run", m, forms, "rename", "--yes"][..], args].concat());
        assert!(!ok, "{args:?}: {json}");
        json["error"].as_str().unwrap().to_string()
    };
    assert!(fails(&["--answer", "prefx=A_"]).contains("no form asked for it"));
    assert!(fails(&["--answer", "count=many"]).contains("is not a number"));
    assert!(fails(&["--answer", "count=500"]).contains("outside what the field takes"));
    assert!(fails(&["--answer", "kind=dragons"]).contains("is not one of creatures, doors"));
    assert_eq!(tag(&path), "SGT_GATE_GUARD");

    // Code that fails changes nothing, and says where.
    let hostile = fixtures().join("hostile");
    let (ok, failed) = mg(&["plugin", "run", m, hostile.to_str().unwrap(), "fails"]);
    assert!(!ok);
    let error = failed["error"].as_str().unwrap();
    assert!(error.contains("something went wrong") && error.contains("main.luau"), "{error}");
    assert_eq!(tag(&path), "SGT_GATE_GUARD");
}

/// A plugin's folder is packed into an archive, and the archive installed
/// into a folder of plugins: the plugin there is the one packed.
#[test]
fn a_plugin_is_packed_and_installed() {
    let dir = mg_testkit::scratch_dir("mg-plugin-pack");
    let path = module(&dir);
    let source = fixtures().join("tag-conventions");
    let archive = dir.join("tags.zip");
    let plugins = dir.join("plugins");
    let (zip, to) = (archive.to_str().unwrap(), plugins.to_str().unwrap());

    let (ok, packed) = mg(&["plugin", "pack", source.to_str().unwrap(), "-o", zip]);
    assert!(ok, "{packed}");
    assert_eq!(packed["files"], json!(["main.luau", "plugin.cfg", "rules.luau"]));
    assert_eq!(packed["bytes"], std::fs::metadata(&archive).unwrap().len());

    let (ok, installed) = mg(&["plugin", "install", zip, to]);
    assert!(ok, "{installed}");
    let folder = plugins.join("example.tag-conventions");
    assert_eq!(installed["folder"], folder.to_str().unwrap());
    assert_eq!(installed["replaced"], Json::Null);
    // It is listed there, and runs from there.
    let (ok, listed) = mg(&["plugin", "list", to]);
    assert!(ok, "{listed}");
    assert_eq!(listed["plugins"][0]["id"], "example.tag-conventions");
    let (ok, done) =
        mg(&["plugin", "run", path.to_str().unwrap(), folder.to_str().unwrap(), "fix-tags"]);
    assert!(ok, "{done}");
    assert_eq!(tag(&path), "GATE_GUARD");

    // Again: only over itself, and only when told to.
    let (ok, again) = mg(&["plugin", "install", zip, to]);
    assert!(!ok);
    assert!(again["error"].as_str().unwrap().contains("1.0.0 is installed already (--replace"));
    let (ok, again) = mg(&["plugin", "install", zip, to, "--replace"]);
    assert!(ok, "{again}");
    assert_eq!(again["replaced"], "1.0.0");

    // The archive is checked like a folder: its layout and manifest, and
    // that its code registers what the manifest declares.
    let (ok, checked) = mg(&["plugin", "check", zip]);
    assert!(ok, "{checked}");
    assert_eq!(checked["plugins"], json!([{ "plugin": "example.tag-conventions", "faults": [] }]));

    // A plugin whose code and manifest disagree is not packed; a file
    // that is no archive is not installed, and fails the check.
    let broken = fixtures().join("broken");
    let (ok, failed) = mg(&["plugin", "pack", broken.to_str().unwrap(), "-o", zip]);
    assert!(!ok, "{failed}");
    let junk = dir.join("junk.zip");
    std::fs::write(&junk, "junk").unwrap();
    let (ok, failed) = mg(&["plugin", "install", junk.to_str().unwrap(), to]);
    assert!(!ok);
    assert!(failed["error"].as_str().unwrap().ends_with("junk.zip: it is not a zip archive"));
    let (ok, checked) = mg(&["plugin", "check", junk.to_str().unwrap()]);
    assert!(!ok);
    assert_eq!(checked["plugins"][0]["faults"], json!(["it is not a zip archive"]));

    // Removed by its id: what was installed, not what was copied in.
    let copied = plugins.join("by-hand");
    std::fs::create_dir_all(&copied).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        std::fs::copy(fixtures().join("forms").join(file), copied.join(file)).unwrap();
    }
    let (ok, removed) = mg(&["plugin", "remove", "example.tag-conventions", to]);
    assert!(ok, "{removed}");
    assert_eq!(removed["version"], "1.0.0");
    assert!(!folder.exists());
    let (ok, refused) = mg(&["plugin", "remove", "test.forms", to]);
    assert!(!ok);
    assert!(refused["error"].as_str().unwrap().contains("was not installed from a file"));
    assert!(copied.join("main.luau").is_file());
    let (ok, missing) = mg(&["plugin", "remove", "example.tag-conventions", to]);
    assert!(!ok);
    assert!(
        missing["error"].as_str().unwrap().ends_with("no plugin example.tag-conventions there")
    );
}

/// A plugin's checks run with the doctor's (where there is a game to
/// verify against).
#[test]
fn a_plugin_s_checks_count_in_verify() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("mg-plugin-verify");
    let path = module(&dir);
    let plugins = fixtures().join("tag-conventions");
    let args = ["--root", root.to_str().unwrap(), "verify", path.to_str().unwrap()];
    let (_, plain) = mg(&args);
    assert_eq!(plain["findings"], json!([]));
    let (ok, with) = mg(&[&args[..], &["--plugins", plugins.to_str().unwrap()]].concat());
    assert!(ok, "a warning is no failure: {with}");
    assert_eq!(with["warnings"], plain["warnings"].as_u64().unwrap() + 1);
    assert_eq!(
        with["findings"],
        json!([{
            "severity": "warning",
            "check": "example.tag-conventions/tag-case",
            "source": "module",
            "resource": "guard.utc",
            "at": "Tag",
            "message": "tag \"gate_guard\" is not upper case",
        }])
    );
}
