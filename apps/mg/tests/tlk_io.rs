//! `mg tlk-import` and `mg tlk-export`: lines read from JSON into a new
//! talk table come out again as they went in, as CSV too, and the JSON is
//! what neverwinter.nim's `nwn_tlk` writes of the same table.

use std::process::Command;

fn mg(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mg")).args(args).output().unwrap()
}

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

    // Another extension is refused, and a file that can't be read
    // changes nothing.
    assert!(!mg(&["tlk-export", &table, &at("out.txt")]).status.success());
    std::fs::write(dir.join("bad.json"), "{").unwrap();
    assert!(!mg(&["tlk-import", &table, &at("bad.json")]).status.success());

    // As nwn_tlk writes the same table.
    let tool = mg_testkit::oracle_tool!("nwn_tlk");
    let theirs = Command::new(tool).args(["-i", &table, "-k", "json"]).output().unwrap();
    assert!(theirs.status.success(), "{}", String::from_utf8_lossy(&theirs.stderr));
    let theirs: serde_json::Value = serde_json::from_slice(&theirs.stdout).unwrap();
    assert_eq!(read("again.json"), theirs);
}
