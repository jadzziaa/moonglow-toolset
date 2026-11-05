//! `mg minimap`: Neverwinter Chess's board as a PNG, a tile's pictures'
//! size or the size asked for, a tile.

/// A PNG's width and height, from its header.
fn size(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[1..4], b"PNG");
    let be = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
    (be(16), be(20))
}

#[test]
fn an_area_s_minimap_as_a_png() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-minimap");
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let out = dir.join("chess.png");
    let mg = |extra: &[&str]| {
        let o = crate::mg_on(&root, None)
            .arg("minimap")
            .arg(&chess)
            .args(["chess", out.to_str().unwrap()])
            .args(extra)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        size(&std::fs::read(&out).unwrap())
    };
    let (w, h) = mg(&[]);
    assert!(w == h && w % 6 == 0 && w >= 6 * 16, "{w}×{h}");
    assert_eq!(mg(&["--size", "8"]), (48, 48));
}

/// `mg minimap --map-notes`: Chess's board with two waypoints given a map
/// note, one enabled and one not: the enabled one alone marked (and in
/// the JSON, with its text and where it is in the picture), the board
/// otherwise as without; and several areas' minimaps into a folder with
/// --to (--all), asking for --force before writing over them.
#[test]
fn map_notes_are_marked_and_minimaps_go_into_a_folder() {
    use mg_core::{ResRef, ResType};
    use mg_gff::Value;
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-minimap-notes");
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let mut m = mg_module::Module::open(&chess).unwrap();
    let area = ResRef::from_str("chess").unwrap();
    let key = mg_resman::ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&key).unwrap().unwrap();
    let mut list = match git.root.get("WaypointList") {
        Some(Value::List(l)) => l.clone(),
        _ => Vec::new(),
    };
    for (tag, at, enabled) in [("MG_SHOWN", [15.0, 25.0], 1), ("MG_HIDDEN", [45.0, 45.0], 0)] {
        let mut w = mg_module::instances::walk_waypoint(tag.as_bytes(), [at[0], at[1], 0.0]);
        w.set("HasMapNote", Value::Byte(1));
        w.set("MapNoteEnabled", Value::Byte(enabled));
        let mut note = mg_core::LocString::default();
        note.set(mg_core::Language::ENGLISH, mg_core::Gender::Male, format!("{tag} here"));
        w.set("MapNote", Value::LocString(note));
        list.push(w);
    }
    git.root.set("WaypointList", Value::List(list));
    m.set_gff(key, &git).unwrap();
    m.save().unwrap();
    let mg = |args: &[&str]| {
        let o = crate::mg_on(&root, None)
            .arg("--json")
            .arg("minimap")
            .arg(&chess)
            .args(args)
            .output()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or_default();
        (o.status.success(), json, String::from_utf8_lossy(&o.stderr).into_owned())
    };
    let at = |name: &str| dir.join(name).to_str().unwrap().to_string();
    let (ok, plain, err) = mg(&["chess", &at("plain.png"), "--size", "16"]);
    assert!(ok && plain.get("map_notes").is_none(), "{err}{plain}");
    let (ok, marked, err) = mg(&["chess", &at("marked.png"), "--size", "16", "--map-notes"]);
    assert!(ok, "{err}");
    let notes = marked["map_notes"].as_array().unwrap();
    let mine: Vec<_> =
        notes.iter().filter(|n| n["tag"].as_str().unwrap().starts_with("MG_")).collect();
    assert_eq!(mine.len(), 1, "the enabled one alone: {marked}");
    assert_eq!(mine[0]["text"], "MG_SHOWN here");
    // 15 m east, 25 m north on a board 6 tiles high at 16 pixels a tile
    // (and the marker's own pixel right and down).
    let (x, y) = (mine[0]["x"].as_f64().unwrap(), mine[0]["y"].as_f64().unwrap());
    assert!((x - 25.0).abs() < 0.01 && (y - (96.0 - 40.0 + 1.0)).abs() < 0.01, "{x}, {y}");
    let differ = |a: &str, b: &str| {
        let (a, b) = (std::fs::read(at(a)).unwrap(), std::fs::read(at(b)).unwrap());
        a != b
    };
    assert!(differ("plain.png", "marked.png"), "the note marked");

    // Into a folder: every area, then again only with --force.
    let (ok, into, err) = mg(&["--to", &at("maps"), "--all", "--map-notes"]);
    assert!(ok, "{err}");
    let written = into["written"].as_array().unwrap();
    assert!(!written.is_empty() && dir.join("maps/chess.png").is_file(), "{into}");
    assert_eq!(size(&std::fs::read(dir.join("maps/chess.png")).unwrap()).0 % 6, 0);
    // (With --json, an error is said in the JSON.)
    let said = |json: &serde_json::Value, err: &str| format!("{err}{}", json["error"]);
    let (ok, json, err) = mg(&["--to", &at("maps"), "chess"]);
    assert!(!ok && said(&json, &err).contains("--force"), "{err}{json}");
    let (ok, again, err) = mg(&["--to", &at("maps"), "chess", "--force"]);
    assert!(ok && again["written"].as_array().unwrap().len() == 1, "{err}{again}");
    // Without --to: one area and its PNG.
    let (ok, json, err) = mg(&["chess"]);
    assert!(!ok && said(&json, &err).contains("--to FOLDER"), "{err}{json}");
}
