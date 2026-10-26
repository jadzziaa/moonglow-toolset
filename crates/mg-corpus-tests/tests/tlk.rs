//! Talk-table corpus tests.

use mg_core::{Codepage, StrRef};
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

/// Shipped talk tables as JSON, as neverwinter.nim's `nwn_tlk` writes them
/// (how nasher keeps a project's table): byte for byte, both ways. The
/// game's dialog.tlk has thousands of sound lengths, control characters
/// and bytes windows-1252 has no character for. A sample of the tables on
/// an everyday run (English's always), all of them in a full one.
#[test]
fn shipped_talk_tables_as_json_as_nwn_tlk_has_them() {
    use mg_tlk::json::{JsonStyle, from_json, to_json};
    let root = corpus!();
    let tool = mg_testkit::oracle_tool!("nwn_tlk");
    let mut tables: Vec<std::path::PathBuf> = std::fs::read_dir(root.join("lang"))
        .unwrap()
        .flatten()
        .flat_map(|l| ["dialog.tlk", "dialogf.tlk"].map(|n| l.path().join("data").join(n)))
        .filter(|p| p.is_file())
        .collect();
    tables.sort_by_key(|p| !p.starts_with(root.join("lang/en")));
    let dir = mg_testkit::scratch_dir("tlk-json");
    let mut checked = 0;
    for path in mg_testkit::sweep(tables, 4) {
        let name = path.strip_prefix(&root).unwrap().display().to_string();
        let tlk = Tlk::read(&std::fs::read(&path).unwrap()).unwrap();
        for pretty in [false, true] {
            let mut args =
                vec!["-i".into(), path.display().to_string(), "-k".into(), "json".into()];
            if pretty {
                args.push("-p".into());
            }
            let theirs = std::process::Command::new(&tool).args(&args).output().unwrap();
            assert!(theirs.status.success(), "{name}: {}", String::from_utf8_lossy(&theirs.stderr));
            let ours = to_json(&tlk, JsonStyle { pretty, ..Default::default() });
            if ours != theirs.stdout {
                let at = ours.iter().zip(&theirs.stdout).position(|(a, b)| a != b).unwrap_or(0);
                let around = |b: &[u8]| {
                    String::from_utf8_lossy(&b[at.saturating_sub(80)..(at + 80).min(b.len())])
                        .into_owned()
                };
                panic!(
                    "{name} (pretty {pretty}): differs at byte {at}:\nours   {}\ntheirs {}",
                    around(&ours),
                    around(&theirs.stdout)
                );
            }
        }
        // Back from that JSON: the file nwn_tlk makes of it.
        let json = dir.join("table.json");
        std::fs::write(&json, to_json(&tlk, JsonStyle::default())).unwrap();
        let made = dir.join("made.tlk");
        let out = std::process::Command::new(&tool)
            .args(["-i", &json.display().to_string(), "-o", &made.display().to_string()])
            .output()
            .unwrap();
        assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
        let theirs = std::fs::read(&made).unwrap();
        let back = from_json(&std::fs::read(&json).unwrap(), Codepage::WINDOWS_1252).unwrap();
        let ours = back.to_bytes_as_nwn_tlk().unwrap();
        assert!(ours == theirs, "{name}: the table made of the JSON differs from nwn_tlk's");
        checked += 1;
    }
    eprintln!("checked {checked} talk tables as JSON");
    assert!(checked > 0);
}
