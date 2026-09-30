//! Talk-table corpus tests.

use mg_core::StrRef;
use mg_testkit::corpus;
use mg_tlk::Tlk;

/// Every shipped talk table (all installed languages) parses, and writing it
/// back loses nothing. (Not byte for byte: shipped tables disagree on the
/// unused string offset of empty entries.)
#[test]
fn shipped_talk_tables_round_trip() {
    let root = corpus!();
    let mut checked = 0;
    for lang in std::fs::read_dir(root.join("lang")).unwrap().flatten() {
        for name in ["dialog.tlk", "dialogf.tlk"] {
            let path = lang.path().join("data").join(name);
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let tlk = Tlk::read(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(tlk.entries.len() > 100_000, "{}", path.display());
            assert!(tlk.text(StrRef(0)).is_some(), "{}: no row 0 text", path.display());
            let out = tlk.to_bytes().unwrap();
            assert_eq!(out.len(), bytes.len(), "{}", path.display());
            let back = Tlk::read(&out).unwrap();
            if let Some(i) = (0..tlk.entries.len()).find(|&i| back.entries[i] != tlk.entries[i]) {
                panic!("{}: entry {i} changed on rewrite", path.display());
            }
            checked += 1;
        }
    }
    eprintln!("checked {checked} talk tables");
    assert!(checked > 0);
}
