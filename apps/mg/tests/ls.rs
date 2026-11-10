//! `mg ls`: an archive's resources, each with its size, and with
//! `--sha256` its hash, which a hak's inventory compares.

#[test]
fn ls_gives_each_resource_s_size_and_its_hash_when_asked() {
    let dir = mg_testkit::scratch_dir("mg-ls-sha256");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), "abc").unwrap();
    std::fs::write(src.join("b.2da"), "").unwrap();
    let hak = dir.join("x.hak");
    let out = crate::mg_output(&["pack", src.to_str().unwrap(), hak.to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let hak = hak.to_str().unwrap();

    // Without it: names and sizes, no hash.
    let (ok, json) = crate::mg_json(&["ls", hak]);
    assert!(ok, "{json}");
    assert_eq!(
        json["resources"],
        serde_json::json!([{ "name": "a.txt", "size": 3 }, { "name": "b.2da", "size": 0 }])
    );
    // With it: each one's SHA-256 (of "abc", and of nothing).
    let (ok, json) = crate::mg_json(&["ls", hak, "--sha256"]);
    assert!(ok, "{json}");
    let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    assert_eq!(json["resources"][0]["sha256"], abc, "{json}");
    assert_eq!(json["resources"][1]["sha256"], empty, "{json}");
    let text = String::from_utf8(crate::mg_output(&["ls", hak, "--sha256"]).stdout).unwrap();
    assert_eq!(text, format!("         3  {abc}  a.txt\n         0  {empty}  b.2da\n"));

    // An entry that runs past the archive's end: an error, not a panic.
    let mut bytes = std::fs::read(hak).unwrap();
    bytes.truncate(bytes.len() - 1);
    let cut = dir.join("cut.hak");
    std::fs::write(&cut, bytes).unwrap();
    let out = crate::mg_output(&["ls", cut.to_str().unwrap(), "--sha256"]);
    assert!(!out.status.success());
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.starts_with("mg: ") && said.contains("truncated"), "{said}");
}
