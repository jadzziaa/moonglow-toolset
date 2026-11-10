//! `--hak`: archives and folders given on the command line over the
//! game's load order, the first given highest; and `mg cat --to`, many
//! resources written into a folder in one run, each with where it came
//! from, its size and its SHA-256.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// A folder `top` with its own `classes.2da` and `mg_zz_top.txt`, packed
/// into `top.hak`; a folder `low` with another `classes.2da` and
/// `mg_zz_low.txt`. Returns the scratch folder.
fn haks(name: &str) -> PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let src = dir.join("top-src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("classes.2da"), "2DA V2.0\n\n Label\n0 top\n").unwrap();
    std::fs::write(src.join("mg_zz_top.txt"), "top").unwrap();
    let out = crate::mg_output(&["pack", text(&src), text(&dir.join("top.hak"))]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let low = dir.join("low");
    std::fs::create_dir_all(&low).unwrap();
    std::fs::write(low.join("classes.2da"), "2DA V2.0\n\n Label\n0 low\n").unwrap();
    std::fs::write(low.join("mg_zz_low.txt"), "low").unwrap();
    dir
}

fn text(p: &Path) -> &str {
    p.to_str().unwrap()
}

fn sha256(data: &[u8]) -> String {
    mg_core::sha1::hex(&mg_core::sha256::sha256(data))
}

#[test]
fn many_resources_are_written_with_their_layers_sizes_and_hashes() {
    let root = mg_testkit::corpus!();
    let dir = haks("mg-cat-many");
    let (top, low, to) = (dir.join("top.hak"), dir.join("low"), dir.join("out"));
    let list = dir.join("names.txt");
    std::fs::write(&list, "# a list\nmg_zz_low.txt\n\nCLASSES.2DA\nmg_zz_nowhere.txt\n").unwrap();
    let run = |extra: &[&str]| -> (bool, Value) {
        let mut mg = crate::mg_on(&root, None);
        mg.args(["--json", "cat", "classes.2da", "mg_zz_top.txt", "racialtypes.2da"]);
        mg.args(["--hak", text(&top), "--hak", text(&low), "--list", text(&list)]);
        crate::json_out(mg.args(["--to", text(&to)]).args(extra))
    };

    // Asked for once each (CLASSES.2DA is classes.2da), in the order given;
    // the first hak given wins over the folder after it, and both over
    // the game. One found nowhere is said and fails the run, once the
    // rest are written.
    let (ok, json) = run(&["--dry-run"]);
    assert!(!ok, "{json}");
    assert_eq!(json["written"], false, "{json}");
    assert!(!to.exists(), "a dry run made the folder");
    let (ok, json) = run(&[]);
    assert!(!ok, "{json}");
    let got = json["resources"].as_array().unwrap();
    let names: Vec<&str> = got.iter().map(|r| r["resource"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["classes.2da", "mg_zz_top.txt", "racialtypes.2da", "mg_zz_low.txt", "mg_zz_nowhere.txt"]
    );
    let layer = |i: usize| got[i]["layer"].as_str().unwrap().to_string();
    assert_eq!(layer(0), format!("hak:{}", top.display()), "{json}");
    assert_eq!(layer(1), format!("hak:{}", top.display()), "{json}");
    assert!(layer(2).starts_with("key:") || layer(2).starts_with("bif"), "{json}");
    assert_eq!(layer(3), format!("folder:{}", low.display()), "{json}");
    assert_eq!(got[4]["found"], false, "{json}");
    assert_eq!(json["missing"], serde_json::json!(["mg_zz_nowhere.txt"]), "{json}");
    for r in &got[..4] {
        let data = std::fs::read(r["file"].as_str().unwrap()).unwrap();
        assert_eq!(r["size"], data.len(), "{r}");
        assert_eq!(r["sha256"], sha256(&data), "{r}");
    }
    assert_eq!(
        std::fs::read_to_string(to.join("classes.2da")).unwrap(),
        "2DA V2.0\n\n Label\n0 top\n"
    );
    // A dry run says the same of each, and writes nothing.
    let (_, dry) = run(&["--dry-run"]);
    assert_eq!(dry["resources"], json["resources"]);

    // Not written over unasked; with --force it is.
    let (ok, refused) = run(&[]);
    assert!(!ok && refused["error"].as_str().unwrap().contains("--force"), "{refused}");
    let (_, forced) = run(&["--force"]);
    assert_eq!(forced["resources"], json["resources"]);

    // Names from standard input; all found, it succeeds.
    let mut mg = crate::mg_on(&root, None);
    mg.args(["--json", "cat", "--list", "-", "--hak", text(&low), "--to"]).arg(dir.join("in"));
    mg.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped());
    let mut child = mg.spawn().unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(b"mg_zz_low.txt\nclasses.2da\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["missing"], serde_json::json!([]), "{json}");
    assert_eq!(json["resources"][1]["layer"], format!("folder:{}", low.display()));
}

#[test]
fn which_and_cat_read_the_haks_given() {
    let root = mg_testkit::corpus!();
    let dir = haks("mg-which-hak");
    let (top, low) = (dir.join("top.hak"), dir.join("low"));
    let (ok, json) = crate::json_out(crate::mg_on(&root, None).args([
        "--json",
        "which",
        "classes.2da",
        "--hak",
        text(&low),
        "--hak",
        text(&top),
    ]));
    assert!(ok, "{json}");
    // (The folder given first wins over the hak after it.)
    let layers: Vec<&str> =
        json["copies"].as_array().unwrap().iter().map(|c| c["layer"].as_str().unwrap()).collect();
    assert_eq!(
        layers[..2],
        [format!("folder:{}", low.display()), format!("hak:{}", top.display())]
    );
    assert!(layers.len() > 2, "the game's own copy: {layers:?}");
    let out = crate::mg_on(&root, None)
        .args(["cat", "mg_zz_top.txt", "--hak", text(&top)])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.stdout, b"top");
    // Several without --to: refused, and a path that is nothing.
    let several = crate::mg_on(&root, None).args(["cat", "a.2da", "b.2da"]).output().unwrap();
    assert!(!several.status.success());
    assert!(String::from_utf8_lossy(&several.stderr).contains("--to"));
    let nothing = crate::mg_on(&root, None)
        .args(["cat", "classes.2da", "--hak"])
        .arg(dir.join("mg_nothing_here"))
        .output()
        .unwrap();
    assert!(!nothing.status.success());
    assert!(String::from_utf8_lossy(&nothing.stderr).contains("no such file or folder"));
}
