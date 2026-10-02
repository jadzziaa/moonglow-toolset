//! The hak editor's save, on every hak the game ships: opened and saved
//! again (to a scratch folder), each holds the same resources with the same
//! bytes, in the same order, and neverwinter.nim's `nwn_erf` lists them all.
//! Saving streams one resource at a time, so the 700 MB haks aren't held in
//! memory.

use std::process::Command;

use mg_module::hak_edit::Hak;
use mg_resman::{Container, ErfContainer};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

#[test]
fn shipped_haks_survive_the_editor() {
    let root = corpus!();
    let nwn_erf = oracle_tool!("nwn_erf");
    let dir = scratch_dir("hak-edit");
    let mut haks: Vec<_> = std::fs::read_dir(root.join("data/hk"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "hak"))
        .collect();
    haks.sort();
    assert!(haks.len() > 10, "{haks:?}");
    for path in haks {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let out = dir.join(&name);
        let mut hak = Hak::open(&path).unwrap();
        hak.save(Some(&out)).unwrap();
        let (a, b) = (ErfContainer::open(&path).unwrap(), ErfContainer::open(&out).unwrap());
        assert_eq!(a.index(), b.index(), "{name}");
        assert_eq!(a.header.description, b.header.description, "{name}");
        for (key, _) in a.index() {
            assert!(a.read(key).unwrap() == b.read(key).unwrap(), "{name}: {key}");
        }
        let listed = Command::new(&nwn_erf).args(["-t", "-f"]).arg(&out).output().unwrap();
        assert!(listed.status.success(), "{name}: {}", String::from_utf8_lossy(&listed.stderr));
        let lines = String::from_utf8_lossy(&listed.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        assert_eq!(lines, a.index().len(), "{name}: nwn_erf lists");
        std::fs::remove_file(&out).unwrap();
    }
}
