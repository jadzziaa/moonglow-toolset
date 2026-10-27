//! `mg 2da-check`: a builder's own table held to its description
//! (`name.2da.toml` beside it), its lists read from a script's constants.

use serde_json::Value;

fn mg(args: &[&str]) -> (bool, Value) {
    crate::json_out(crate::mg().arg("--json").args(args))
}

const DESCRIPTION: &str = r#"
rows = "loot_kinds"

[columns.Chance]
kind = "int"
min = 0
max = 100

[columns.Kinds]
kind = "flags"
constants = "loot_inc"
prefix = "LOOT_KIND_"

[columns.Rarity]
kind = "list"
values = { Common = 0, Rare = 1 }

[[rules]]
when = { column = "Rarity", is = "Rare" }
require = "Chance"
"#;

#[test]
fn a_folder_s_tables_are_held_to_their_descriptions() {
    let dir = mg_testkit::scratch_dir("mg-2da-check");
    let tables = dir.join("src/2da");
    std::fs::create_dir_all(&tables).unwrap();
    std::fs::create_dir_all(dir.join("src/nss")).unwrap();
    // (A project: a table named by its file finds the scripts from here.)
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    let write = |name: &str, text: &str| std::fs::write(tables.join(name), text).unwrap();
    write("loot.2da.toml", DESCRIPTION);
    write("loot_kinds.2da", "2DA V2.0\r\n\r\n  Label\r\n0 a\r\n1 b\r\n2 c\r\n");
    // (A table with no description is not looked at.)
    write("other.2da", "2DA V2.0\r\n\r\n  Label\r\n0 #N/A\r\n");
    std::fs::write(
        dir.join("src/nss/loot_inc.nss"),
        "const int LOOT_KIND_GOLD = 0x01;\nconst int LOOT_KIND_GEMS = 0x02;\n",
    )
    .unwrap();
    let good = "2DA V2.0\r\n\r\n  Label Chance Kinds Rarity\r\n0 a 50 0x03 0\r\n1 b 5 0x01 1\r\n";
    write("loot.2da", good);
    let at = dir.to_str().unwrap();
    let (ok, out) = mg(&["2da-check", at]);
    assert!(ok, "{out}");
    assert_eq!((out["wrong"].as_u64(), out["tables"].as_array().unwrap().len()), (Some(0), 1));
    assert_eq!(out["tables"][0]["rows"], 2);

    // A fault of each kind, and a row the other table does not have.
    let bad = "2DA V2.0\r\n\r\n  Label Chance Kinds Rarity\r\n0 a 150 0x03 0\r\n1 b **** 0x04 \
               1\r\n2 c 5 #N/A 7\r\n3 d 1 **** ****\r\n";
    write("loot.2da", bad);
    let (ok, out) = mg(&["2da-check", at]);
    assert!(!ok, "{out}");
    let faults = out["tables"][0]["faults"].as_array().unwrap();
    let said: Vec<(u64, &str, &str)> = faults
        .iter()
        .map(|f| {
            (
                f["row"].as_u64().unwrap(),
                f["column"].as_str().unwrap(),
                f["message"].as_str().unwrap(),
            )
        })
        .collect();
    let has = |row, column: &str, part: &str| {
        assert!(
            said.iter().any(|(r, c, m)| (*r, *c) == (row, column) && m.contains(part)),
            "row {row} {column}: {part} in {said:#?}"
        );
    };
    has(0, "Chance", "150 is above");
    has(1, "Chance", "where Rarity is Rare");
    has(1, "Kinds", "0x4 is no flag");
    has(2, "Kinds", "not a set of flags");
    has(2, "Rarity", "none of the column's 2 values");
    has(3, "Label", "loot_kinds.2da's, which has 3");
    assert_eq!((out["wrong"].as_u64(), faults.len()), (Some(6), 6), "{said:#?}");
    assert_eq!(faults[3]["value"], "#N/A");

    // One table by its file; a table without a description, and a
    // description that cannot be read, are errors said plainly.
    let one = tables.join("loot.2da");
    assert_eq!(mg(&["2da-check", one.to_str().unwrap()]).1["wrong"], 6);
    let (ok, out) = mg(&["2da-check", tables.join("other.2da").to_str().unwrap()]);
    assert!(!ok && out["error"].as_str().unwrap().contains("has no description"), "{out}");
    write("loot.2da.toml", "[columns.Kinds]\nkind = \"flags\"\nconstants = \"nowhere\"\n");
    let (ok, out) = mg(&["2da-check", at]);
    assert!(!ok && out["unread"] == 1, "{out}");
    assert!(out["tables"][0]["error"].as_str().unwrap().contains("nowhere was not found"));
}
