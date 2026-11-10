//! `mg 2da-set`: cells of a 2DA file set in a row by number, by a
//! column's value or added at the end, the file written as it is laid
//! out; quoted cells and `****` read and written by mg-2da.

use std::path::Path;

/// Columns in line, a quoted cell, an empty one, LF line ends.
const TABLE: &str = "2DA V2.0\n\n       Label        Name          Cost\n\
0      Sword        \"Long Sword\"  10\n\
1      Axe          ****          5\n\
2      Axe2         ****          5\n";

fn set(table: &Path, args: &[&str]) -> (bool, serde_json::Value) {
    let mut all = vec!["2da-set", table.to_str().unwrap()];
    all.extend(args);
    crate::mg_json(&all)
}

#[test]
fn cells_are_set_by_row_by_a_column_s_value_and_in_a_row_added() {
    let dir = mg_testkit::scratch_dir("mg-2da-set");
    let table = dir.join("weapons.2da");
    std::fs::write(&table, TABLE).unwrap();
    let read = || std::fs::read_to_string(&table).unwrap();
    let lines = |t: &str| t.lines().map(str::to_string).collect::<Vec<_>>();

    // By number: the cell quoted where it has a space; the other lines
    // as they were, LF kept.
    let (ok, json) = set(&table, &["--row", "1", "Name=Battle Axe"]);
    assert!(ok, "{json}");
    assert_eq!(
        json["cells"],
        serde_json::json!([{ "column": "Name", "from": null, "to": "Battle Axe" }])
    );
    let now = read();
    assert!(!now.contains('\r'), "{now:?}");
    let (was, is) = (lines(TABLE), lines(&now));
    assert_eq!(is.len(), was.len());
    assert_eq!(is[3], was[3], "{now}");
    assert!(is[4].contains("\"Battle Axe\"") && is[4].starts_with("1      Axe"), "{now}");
    assert_eq!(is[5], was[5], "{now}");
    let t = mg_2da::TwoDa::parse(now.as_bytes(), mg_core::Codepage::default()).unwrap();
    assert_eq!(t.get(0, "Name"), Some("Long Sword"));

    // By a column's value, two cells, one emptied.
    let (ok, json) = set(&table, &["--where", "Label=Sword", "Cost=12", "Name=****"]);
    assert!(ok, "{json}");
    assert_eq!(json["row"], 0);
    let t = mg_2da::TwoDa::parse(read().as_bytes(), mg_core::Codepage::default()).unwrap();
    assert_eq!((t.get(0, "Cost"), t.get(0, "Name")), (Some("12"), None));

    // A row added at the end, numbered after the last, its other cells
    // empty.
    let (ok, json) = set(&table, &["--append", "Label=Mace", "Cost=3"]);
    assert!(ok, "{json}");
    assert_eq!((json["row"].as_u64(), json["appended"].as_bool()), (Some(3), Some(true)));
    let now = read();
    let last = now.lines().last().unwrap();
    assert!(last.starts_with("3 ") && last.contains("Mace") && last.contains("****"), "{now}");

    // A dry run, and a cell set to what it holds: nothing written.
    let before = read();
    let (ok, json) = set(&table, &["--row", "2", "Cost=99", "--dry-run"]);
    assert!(ok && json["written"] == false && json["cells"][0]["to"] == "99", "{json}");
    let (ok, json) = set(&table, &["--row", "2", "Cost=5"]);
    assert!(ok && json["written"] == false && json["cells"] == serde_json::json!([]), "{json}");
    assert_eq!(read(), before);
    // -o: written there, the table left as it is.
    let other = dir.join("other.2da");
    let (ok, _) = set(&table, &["--row", "2", "Cost=6", "-o", other.to_str().unwrap()]);
    assert!(ok);
    assert_eq!(read(), before);
    assert!(std::fs::read_to_string(&other).unwrap().contains("Axe2         ****          6"));
}

#[test]
fn what_cannot_be_set_is_refused_and_nothing_written() {
    let dir = mg_testkit::scratch_dir("mg-2da-set-refused");
    let table = dir.join("weapons.2da");
    std::fs::write(&table, TABLE).unwrap();
    for (args, said) in [
        (&["--row", "1", "Weight=3"][..], "no column Weight"),
        (&["--row", "9", "Cost=3"], "no row 9"),
        (&["--where", "Label=Club", "Cost=3"], "no row has Label Club"),
        (&["--where", "Cost=5", "Name=x"], "rows 1, 2 have Cost 5"),
        (&["--row", "1", "Name=say \"hi\""], "Name=say"),
        (&["--row", "1", "Name"], "COLUMN=VALUE"),
        (&["Cost=1"], "--append"),
    ] {
        let (ok, json) = set(&table, args);
        let error = json["error"].as_str().unwrap_or_default();
        assert!(!ok && error.contains(said), "{args:?}: {json}");
        assert_eq!(std::fs::read_to_string(&table).unwrap(), TABLE, "{args:?}");
    }
}
