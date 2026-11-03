//! `mg import MODULE PATH…`: loose files and folders, as well as an ERF
//! (El Grillo's request, 2026-11-03): each file a resource by its name;
//! those the module has skipped and named without `--overwrite`; what is
//! no resource of a module's refused and said why; a script that comes
//! without its compiled script compiled; `--dry-run` saves nothing.

use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use serde_json::Value;

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::new(ResRef::from_str(name).unwrap(), t)
}

fn names(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect()
}

#[test]
fn files_and_folders_are_imported() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-import-files");
    let path = dir.join("town.mod");
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.set("Mod_Name", mg_gff::Value::String(b"x".to_vec()));
    m.set_info(&info).unwrap();
    m.set(key("hello", ResType::NSS), b"void main() {}".to_vec());
    m.set(key("hello", ResType::NCS), b"NCS V1.0B".to_vec());
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let files = dir.join("files");
    std::fs::create_dir_all(files.join("more")).unwrap();
    let gff = |t: &[u8; 4]| Gff::new(*t).to_bytes().unwrap();
    for (name, bytes) in [
        ("hello.nss", b"void main() { PrintString(\"new\"); }".to_vec()),
        ("guard.utc", gff(b"UTC ")),
        ("more/market.are", gff(b"ARE ")),
        ("more/market.git", gff(b"GIT ")),
        ("broken.uti", b"junk".to_vec()),
        ("module.ifo", gff(b"IFO ")),
    ] {
        std::fs::write(files.join(name), bytes).unwrap();
    }
    let run = |args: &[&str]| -> (bool, Value) {
        let mut mg = crate::mg_on(&root, None);
        mg.args(["--json", "import"]).arg(&path).args(args);
        crate::json_out(&mut mg)
    };
    let f = files.to_str().unwrap();

    // A dry run: what would be done, nothing saved.
    let (ok, dry) = run(&[f, "--dry-run"]);
    assert!(ok, "{dry}");
    assert_eq!(dry["dry_run"], true);
    let mut added = names(&dry["added"]);
    added.sort();
    assert_eq!(added, ["guard.utc", "market.are", "market.git"]);
    assert_eq!(names(&dry["skipped"]), ["hello.nss"]);
    let refused: Vec<&str> =
        dry["refused"].as_array().unwrap().iter().map(|r| r["why"].as_str().unwrap()).collect();
    assert_eq!(refused.len(), 2, "{dry}");
    assert!(refused.iter().any(|w| w.contains("doesn't read as a GFF")), "{refused:?}");
    assert!(refused.iter().any(|w| w.contains("Module Properties")), "{refused:?}");
    let notes = dry["notes"].to_string();
    assert!(notes.contains("--overwrite replaces them): hello.nss"), "{notes}");
    assert!(!Module::open(&path).unwrap().contains(&key("guard", ResType::UTC)));

    // Overwriting: the script replaced, and compiled.
    let (ok, done) = run(&[f, "--overwrite"]);
    assert!(ok, "{done}");
    assert_eq!(names(&done["replaced"]), ["hello.nss"]);
    assert_eq!(names(&done["compiled"]), ["hello.nss"]);
    assert_eq!(names(&done["new_areas"]), ["market"]);
    let m = Module::open(&path).unwrap();
    assert!(m.contains(&key("guard", ResType::UTC)));
    assert_ne!(m.get(&key("hello", ResType::NCS)).unwrap(), b"NCS V1.0B", "compiled anew");
    assert_eq!(m.areas().unwrap(), [ResRef::from_str("market").unwrap()]);

    // A file of its own, and one that doesn't compile: said.
    std::fs::write(files.join("bad.nss"), "void main() { nope }").unwrap();
    let (ok, one) = run(&[files.join("bad.nss").to_str().unwrap()]);
    assert!(ok, "{one}");
    assert_eq!(names(&one["not_compiled"]), ["bad.nss"]);
    assert!(one["notes"].to_string().contains("bad.nss doesn't compile"), "{one}");
    // A path not there: an error.
    let (ok, gone) = run(&[files.join("gone.nss").to_str().unwrap()]);
    assert!(!ok && gone["error"].as_str().unwrap().contains("no such file"), "{gone}");
}
