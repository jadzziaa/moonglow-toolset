//! `mg export --files`: resources as loose files in a folder, a script with
//! its compiled script and an area with its `.git` and `.gic`; no game
//! install needed.

use std::process::Command;

use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::new(ResRef::from_str(name).unwrap(), t)
}

#[test]
fn resources_are_exported_as_files() {
    let dir = mg_testkit::scratch_dir("mg-export-files");
    let path = dir.join("town.mod");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for (t, tag) in [(ResType::ARE, b"ARE "), (ResType::GIT, b"GIT "), (ResType::GIC, b"GIC ")] {
        m.set_gff(key("town", t), &Gff::new(*tag)).unwrap();
    }
    m.set(key("fix_me", ResType::NSS), b"void main() {}".to_vec());
    m.set(key("fix_me", ResType::NCS), b"NCS V1.0B".to_vec());
    m.set(key("draft", ResType::NSS), b"void main() {}".to_vec());
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let out_dir = dir.join("development");
    let run = |resources: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_mg"))
            .arg("export")
            .arg(&path)
            .args(resources)
            .arg("--files")
            .arg("-o")
            .arg(&out_dir)
            .output()
            .unwrap();
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        (out.status.success(), text(&out.stdout), text(&out.stderr))
    };
    // A script named by its source or its compiled script: both files; an
    // area: its three. A script not compiled: a warning.
    let (ok, listed, notes) = run(&["fix_me.ncs", "town.are", "draft.nss"]);
    assert!(ok, "{notes}");
    let mut names: Vec<String> = std::fs::read_dir(&out_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["draft.nss", "fix_me.ncs", "fix_me.nss", "town.are", "town.gic", "town.git"]
    );
    assert_eq!(listed.lines().count(), 6);
    assert!(notes.contains("draft.nss isn't compiled"), "{notes}");
    assert_eq!(std::fs::read(out_dir.join("fix_me.ncs")).unwrap(), b"NCS V1.0B");
    // What the module lacks is refused, and nothing is written for it.
    let (ok, _, notes) = run(&["nowhere.nss"]);
    assert!(!ok && notes.contains("nowhere.nss"), "{notes}");
}
