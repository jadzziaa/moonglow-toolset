//! `mg dialog-export` and `mg dialog-import`: a conversation of Contest of
//! Champions written as Ink and read back into a copy under another name
//! plays as the original.

use std::process::Command;

use mg_core::ResType;
use mg_module::dialog_io::play_outline;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

#[test]
fn a_conversation_goes_out_as_ink_and_back() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-dialog-io");
    let path = dir.join("contest.mod");
    let mut m = Module::open(&root.join("data/mod/Contest Of Champions 0492.mod")).unwrap();
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    // The largest conversation that can be played out.
    let key = *m
        .keys_of(ResType::DLG)
        .filter(|k| play_outline(&m.gff(k).unwrap().unwrap(), 20_000).is_some())
        .max_by_key(|k| m.get(k).map_or(0, <[u8]>::len))
        .unwrap();
    let mg = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_mg"))
            .args(["--root", root.to_str().unwrap(), "--no-user-dir"])
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    };
    let ink = dir.join("story.ink");
    let (p, i) = (path.to_str().unwrap(), ink.to_str().unwrap());
    mg(&["dialog-export", p, &format!("{}.dlg", key.resref), i]);
    mg(&["dialog-import", p, i, "--name", "mg_back"]);
    let saved = Module::open(&path).unwrap();
    let outline = |k: ResKey| play_outline(&saved.gff(&k).unwrap().unwrap(), 20_000).unwrap();
    let back = ResKey::parse("mg_back", ResType::DLG).unwrap();
    assert!(!outline(key).is_empty(), "{key}");
    assert_eq!(outline(back), outline(key));
}
