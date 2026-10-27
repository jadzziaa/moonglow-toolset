//! `mg 2da-diff` and `mg tlk-diff`: differences by row and column (or
//! line), and three-way merges.

use std::path::Path;

use serde_json::Value;

fn mg(args: &[&str]) -> (bool, Value) {
    crate::json_out(crate::mg().arg("--json").args(args))
}

fn write(dir: &Path, name: &str, rows: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, format!("2DA V2.0\r\n\r\n{rows}")).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn tables_are_compared_and_merged() {
    let dir = mg_testkit::scratch_dir("mg-2da-diff");
    let base = write(&dir, "base.2da", "   Label Name Cost\n0  a 10 1\n1  b 11 2\n");
    let a = write(&dir, "a.2da", "   Label Name Cost\n0  a 10 1\n1  b 99 2\n2  c 12 3\n");
    let b = write(&dir, "feat.2da", "   Label Name Cost Weight\n0  a 10 7 ****\n1  b 11 2 5\n");
    let at = |name: &str| dir.join(name).to_str().unwrap().to_string();

    let (ok, d) = mg(&["2da-diff", &base, &a]);
    assert!(ok && d["same"] == false, "{d}");
    assert_eq!(d["rows_added"], serde_json::json!([2, 3]));
    assert_eq!(d["cells"][0], serde_json::json!({"row":1,"column":"Name","from":"11","to":"99"}));
    assert_eq!(mg(&["2da-diff", &a, &a]).1["same"], true);

    // Merged: both sides' changes, written as a table and as an edit file.
    let (ok, m) = mg(&[
        "2da-diff",
        &a,
        &b,
        "--base",
        &base,
        "-o",
        &at("merged.2da"),
        "--edits",
        &at("e.json"),
    ]);
    assert!(ok && m["merged"] == true && m["conflicts"].as_array().unwrap().is_empty(), "{m}");
    let want = write(
        &dir,
        "want.2da",
        "   Label Name Cost Weight\n0  a 10 7 ****\n1  b 99 2 5\n2  c 12 3 ****\n",
    );
    assert_eq!(mg(&["2da-diff", &at("merged.2da"), &want]).1["same"], true);
    let edits: Value =
        serde_json::from_str(&std::fs::read_to_string(at("e.json")).unwrap()).unwrap();
    assert_eq!(edits["edits"][0]["op"], "set_resource");
    assert_eq!(edits["edits"][0]["resource"], "feat.2da");
    assert_eq!(
        edits["edits"][0]["text"].as_str().unwrap().as_bytes(),
        std::fs::read(at("merged.2da")).unwrap()
    );

    // A cell both changed: a failure and nothing written, unless told
    // whose to take.
    let c = write(&dir, "c.2da", "   Label Name Cost\n0  a 10 1\n1  b 50 2\n");
    let (ok, m) = mg(&["2da-diff", &a, &c, "--base", &base, "-o", &at("no.2da")]);
    assert!(!ok && m["merged"] == false && !dir.join("no.2da").exists(), "{m}");
    assert_eq!(
        m["conflicts"][0],
        serde_json::json!({"row":1,"column":"Name","base":"11","a":"99","b":"50"})
    );
    let (ok, _) = mg(&["2da-diff", &a, &c, "--base", &base, "-o", &at("yes.2da"), "--prefer", "b"]);
    assert!(ok);
    let text = std::fs::read_to_string(at("yes.2da")).unwrap();
    assert!(text.contains("50") && !text.contains("99"), "{text}");
}

#[test]
fn talk_tables_are_compared_and_merged() {
    use mg_tlk::{Tlk, TlkEntry};
    let dir = mg_testkit::scratch_dir("mg-tlk-diff");
    let table = |name: &str, lines: &[&str]| -> String {
        let mut t = Tlk::new(mg_core::Language::ENGLISH);
        t.entries = lines.iter().map(|l| TlkEntry::text(*l)).collect();
        let path = dir.join(name);
        std::fs::write(&path, t.to_bytes().unwrap()).unwrap();
        path.to_str().unwrap().to_string()
    };
    let at = |name: &str| dir.join(name).to_str().unwrap().to_string();
    let base = table("base.tlk", &["one", "two", "three"]);
    let a = table("a.tlk", &["one", "2", "three", "four"]);
    let b = table("b.tlk", &["one", "two", "drei"]);

    let (ok, d) = mg(&["tlk-diff", &base, &a, "--lines", &at("lines.json")]);
    assert!(ok && d["lines"].as_array().unwrap().len() == 2, "{d}");
    assert_eq!(d["lines"][0]["line"], 1);
    assert_eq!(d["lines"][0]["to"]["text"], "2");
    assert_eq!(d["lines"][1]["from"], Value::Null);
    // The lines that differ, read into the older table, make the newer.
    std::fs::copy(&base, at("copy.tlk")).unwrap();
    assert!(mg(&["tlk-import", &at("copy.tlk"), &at("lines.json")]).0);
    assert_eq!(mg(&["tlk-diff", &at("copy.tlk"), &a]).1["same"], true);

    let (ok, m) = mg(&["tlk-diff", &a, &b, "--base", &base, "-o", &at("merged.tlk")]);
    assert!(ok && m["merged"] == true, "{m}");
    let want = table("want.tlk", &["one", "2", "drei", "four"]);
    assert_eq!(mg(&["tlk-diff", &at("merged.tlk"), &want]).1["same"], true);

    let c = table("c.tlk", &["one", "zwei", "three"]);
    let (ok, m) = mg(&["tlk-diff", &a, &c, "--base", &base, "-o", &at("no.tlk")]);
    assert!(!ok && m["conflicts"][0]["line"] == 1 && !dir.join("no.tlk").exists(), "{m}");
}
