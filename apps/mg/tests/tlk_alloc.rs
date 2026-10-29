//! `mg tlk-alloc`: a custom talk table from strings kept by name, each on
//! a line that stays its own.

use serde_json::Value;

use crate::mg_json as mg;

#[test]
fn names_get_lines_that_stay() {
    let dir = mg_testkit::scratch_dir("mg-tlk-alloc");
    let at = |name: &str| dir.join(name).to_str().unwrap().to_string();
    let (table, names, nss) = (at("tdod.tlk"), at("names.toml"), at("strrefs.nss"));
    let run = |more: &[&str]| {
        mg(&[&["tlk-alloc", &table, "--from", &names, "--constants", &nss][..], more].concat())
    };
    std::fs::write(
        &names,
        "[feat.cleave]\nname = \"Great Cleave\"\ndesc = \"Cuts on.\"\n\n[skill]\nhide = { text = \
         \"Hide\", sound = \"vs_hide\", sound_length = 1.5 }\n",
    )
    .unwrap();
    // Not yet: nothing has a line.
    let (ok, r) = run(&["--check"]);
    assert!(!ok && r["up_to_date"] == false && !dir.join("tdod.tlk").exists(), "{r}");

    let (ok, r) = run(&[]);
    assert!(ok && r["added"].as_array().unwrap().len() == 3, "{r}");
    let lines = |file: &str| -> Value {
        assert!(mg(&["tlk-export", file, &at("out.json")]).0);
        serde_json::from_str(&std::fs::read_to_string(at("out.json")).unwrap()).unwrap()
    };
    let t = lines(&table);
    let texts: Vec<&str> =
        t["entries"].as_array().unwrap().iter().map(|e| e["text"].as_str().unwrap()).collect();
    // In the order of the file, not of the names (desc before name).
    assert_eq!(texts, ["Great Cleave", "Cuts on.", "Hide"]);
    assert_eq!(t["entries"][2]["sound"], "vs_hide");
    let constants = std::fs::read_to_string(&nss).unwrap();
    assert!(constants.contains("const int STRREF_FEAT_CLEAVE_NAME = 16777216;"), "{constants}");
    let map_file = at("names.strrefs.json");
    let map: Value = serde_json::from_str(&std::fs::read_to_string(&map_file).unwrap()).unwrap();
    assert_eq!(map["lines"]["skill.hide"], 2);
    assert!(run(&["--check"]).0, "as written");
    std::fs::copy(&map_file, at("main.json")).unwrap();

    // A key before the others, one taken away, a text changed: the keys
    // that stay keep their lines.
    std::fs::write(
        &names,
        "[aaa]\nfirst = \"First\"\n\n[feat.cleave]\nname = \"Cleave\"\n\n[skill]\nhide = \"Hide\"\n",
    )
    .unwrap();
    assert!(!run(&["--check"]).0);
    let (ok, r) = run(&["--against", &at("main.json")]);
    assert!(ok && r["retired"] == serde_json::json!(["feat.cleave.desc"]), "{r}");
    assert_eq!(r["added"][0], serde_json::json!({"key": "aaa.first", "line": 3}));
    let t = lines(&table);
    let line = |id: u64| {
        let e = t["entries"].as_array().unwrap().iter().find(|e| e["id"] == id).unwrap();
        e["text"].as_str().unwrap().to_string()
    };
    assert_eq!((line(0), line(2), line(3)), ("Cleave".into(), "Hide".into(), "First".into()));
    // The retired key's line keeps its text: a number saved in a character
    // or an item still shows what it said.
    assert_eq!(line(1), "Cuts on.");
    assert!(run(&["--check", "--against", &at("main.json")]).0);

    // A map in which a key has another line than it had: refused.
    let moved = std::fs::read_to_string(&map_file)
        .unwrap()
        .replace("\"skill.hide\": 2", "\"skill.hide\": 9");
    std::fs::write(at("moved.json"), moved).unwrap();
    let (ok, r) = mg(&[
        "tlk-alloc",
        &table,
        "--from",
        &names,
        "--map",
        &at("moved.json"),
        "--check",
        "--against",
        &at("main.json"),
    ]);
    assert!(!ok && r["moved"][0]["key"] == "skill.hide", "{r}");

    // A key's own feminine text makes the feminine table beside the table
    // (tdodf.tlk), where every other line is the text: female characters
    // read it.
    let with_feminine = r#"[aaa]
first = "First"

[feat.cleave]
name = "Cleave"

[skill]
hide = { text = "Hidden", feminine = "Hidden (she)" }
"#;
    std::fs::write(&names, with_feminine).unwrap();
    assert!(run(&[]).0);
    assert!(dir.join("tdodf.tlk").is_file());
    let csv = || {
        assert!(mg(&["tlk-export", &table, &at("out.csv")]).0);
        std::fs::read_to_string(at("out.csv")).unwrap()
    };
    let t = csv();
    assert!(t.starts_with("StrRef,Text,Feminine,"), "{t}");
    assert!(t.contains("16777216,Cleave,Cleave,"), "{t}");
    assert!(t.contains("16777218,Hidden,Hidden (she),"), "{t}");
    assert!(run(&["--check"]).0, "as written");
    // Its feminine taken out of the names file, the line is the text in
    // both tables again.
    std::fs::write(&names, with_feminine.replace(", feminine = \"Hidden (she)\"", "")).unwrap();
    assert!(!run(&["--check"]).0, "the feminine table's line is not as named");
    assert!(run(&[]).0);
    let t = csv();
    assert!(t.contains("16777218,Hidden,Hidden,"), "{t}");
    std::fs::write(&names, "[skill]\nhide = { text = \"Hide\", feminine = 3 }\n").unwrap();
    let (ok, r) = run(&[]);
    assert!(!ok && r.to_string().contains("feminine is not a text"), "{r}");
}
