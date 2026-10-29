//! `mg tlk-import` and `mg tlk-export`: lines read from JSON into a new
//! talk table come out again as they went in, as CSV too, and the JSON is
//! what neverwinter.nim's `nwn_tlk` writes of the same table.

use std::process::Command;

use crate::mg_output as mg;

fn ok(args: &[&str]) -> String {
    let out = mg(args);
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_talk_table_goes_in_and_out_as_json_and_csv() {
    let dir = mg_testkit::scratch_dir("mg-tlk-io");
    let at = |name: &str| dir.join(name).to_string_lossy().into_owned();
    let lines = r#"{"language": 0, "entries": [
        {"id": 0, "text": "Fenwick's Ferry"},
        {"id": 2, "text": "A line, with a comma and \"quotes\"", "sound": "vo_line", "soundLength": 1.5},
        {"id": 5, "text": "Señor"}
    ]}"#;
    std::fs::write(dir.join("in.json"), lines).unwrap();
    let table = at("custom.tlk");

    // A dry run writes nothing; the import makes the table.
    let said = ok(&["tlk-import", &table, &at("in.json"), "--dry-run"]);
    assert!(said.contains("3 added") && said.contains("nothing written"), "{said}");
    assert!(!dir.join("custom.tlk").exists());
    let said = ok(&["--json", "tlk-import", &table, &at("in.json")]);
    let v: serde_json::Value = serde_json::from_str(&said).unwrap();
    assert_eq!((v["added"].as_u64(), v["lines"].as_u64()), (Some(3), Some(6)), "{said}");
    // The same again changes nothing.
    let said = ok(&["tlk-import", &table, &at("in.json")]);
    assert!(said.contains("0 lines changed, 0 added"), "{said}");

    // Out as JSON: the lines that went in.
    ok(&["tlk-export", &table, &at("out.json")]);
    let read = |name: &str| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(dir.join(name)).unwrap()).unwrap()
    };
    let out = read("out.json");
    let entries = out["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[1]["id"], 2);
    assert_eq!(entries[1]["sound"], "vo_line");
    assert_eq!(entries[2]["text"], "Señor");

    // Out as CSV, changed there, and read back.
    ok(&["tlk-export", &table, &at("out.csv")]);
    let csv = std::fs::read_to_string(dir.join("out.csv")).unwrap();
    assert!(csv.contains("Fenwick's Ferry") && csv.contains("\"\"quotes\"\""), "{csv}");
    std::fs::write(dir.join("back.csv"), csv.replace("Fenwick's Ferry", "Fenwick's Landing"))
        .unwrap();
    let said = ok(&["tlk-import", &table, &at("back.csv")]);
    assert!(said.contains("1 lines changed, 0 added"), "{said}");
    ok(&["tlk-export", &table, &at("again.json")]);
    assert_eq!(read("again.json")["entries"][0]["text"], "Fenwick's Landing");
    // A CSV's StrRefs are a custom table's (16777216 and up); --base
    // numbers the lines from 0, as the game's own table's are named, and
    // either is read back.
    let first = |csv: &str| csv.lines().nth(1).unwrap().split(',').next().unwrap().to_string();
    assert_eq!(first(&std::fs::read_to_string(dir.join("out.csv")).unwrap()), "16777216");
    ok(&["tlk-export", &table, &at("base.csv"), "--base"]);
    let base = std::fs::read_to_string(dir.join("base.csv")).unwrap();
    assert_eq!(first(&base), "0");
    // (Read back by its line numbers: the same lines, nothing changed.)
    let said = ok(&["tlk-import", &table, &at("base.csv")]);
    assert!(said.contains("0 lines changed, 0 added"), "{said}");

    // Another extension is refused, and a file that can't be read
    // changes nothing.
    assert!(!mg(&["tlk-export", &table, &at("out.txt")]).status.success());
    std::fs::write(dir.join("bad.json"), "{").unwrap();
    assert!(!mg(&["tlk-import", &table, &at("bad.json")]).status.success());

    // As nwn_tlk writes the same table.
    let tool = mg_testkit::oracle_tool!("nwn_tlk");
    let theirs = Command::new(&tool).args(["-i", &table, "-k", "json"]).output().unwrap();
    assert!(theirs.status.success(), "{}", String::from_utf8_lossy(&theirs.stderr));
    let theirs: serde_json::Value = serde_json::from_slice(&theirs.stdout).unwrap();
    assert_eq!(read("again.json"), theirs);
    // Written as `nwn_tlk --pretty` writes it, to the byte.
    let pretty = Command::new(tool).args(["-i", &table, "-k", "json", "-p"]).output().unwrap();
    assert_eq!(std::fs::read(dir.join("again.json")).unwrap(), pretty.stdout);
}

/// A table made by an import is in the language asked for (`--language`),
/// else a JSON file's own, else English; its text is written in that
/// language's codepage (Polish's Windows-1250). A table that is there
/// keeps its language.
#[test]
fn a_table_made_by_an_import_is_in_its_language() {
    let dir = mg_testkit::scratch_dir("mg-tlk-io-language");
    let at = |name: &str| dir.join(name).to_string_lossy().into_owned();
    std::fs::write(dir.join("pl.csv"), "StrRef,Text\n16777216,Łódź\n").unwrap();
    let polish = at("polish.tlk");
    let said = ok(&["--json", "tlk-import", &polish, &at("pl.csv"), "--language", "pl"]);
    let v: serde_json::Value = serde_json::from_str(&said).unwrap();
    assert_eq!(v["added"], 1, "{said}");
    let t = mg_tlk::Tlk::read(&std::fs::read(&polish).unwrap()).unwrap();
    assert_eq!(
        (t.language, &t.entries[0].text[..]),
        (mg_core::Language::POLISH, &b"\xa3\xf3d\x9f"[..])
    );
    // Back out as it went in.
    ok(&["tlk-export", &polish, &at("pl-out.csv")]);
    assert!(std::fs::read_to_string(dir.join("pl-out.csv")).unwrap().contains("16777216,Łódź,"));
    // There already: its language is not changed by the import.
    let out = mg(&["tlk-import", &polish, &at("pl.csv"), "--language", "de"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("is in Polish already"));
    assert!(mg(&["tlk-import", &polish, &at("pl.csv"), "--language", "pl"]).status.success());
    // A JSON file's own language.
    std::fs::write(
        dir.join("de.json"),
        r#"{"language": 2, "entries": [{"id": 0, "text": "Fähre"}]}"#,
    )
    .unwrap();
    let german = at("german.tlk");
    ok(&["tlk-import", &german, &at("de.json")]);
    let t = mg_tlk::Tlk::read(&std::fs::read(&german).unwrap()).unwrap();
    assert_eq!(t.language, mg_core::Language::GERMAN);
    // Without either, English, which has no letters for Polish's.
    let english = at("english.tlk");
    let out = mg(&["tlk-import", &english, &at("pl.csv")]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("can't write"));
    std::fs::write(dir.join("en.csv"), "StrRef,Text\n16777216,Ferry\n").unwrap();
    ok(&["tlk-import", &english, &at("en.csv")]);
    let t = mg_tlk::Tlk::read(&std::fs::read(&english).unwrap()).unwrap();
    assert_eq!(t.language, mg_core::Language::ENGLISH);
    let out = mg(&["tlk-import", &at("other.tlk"), &at("pl.csv"), "--language", "xx"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("\"xx\" isn't a language"));
}
