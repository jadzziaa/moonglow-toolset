//! `--json`: every command prints one JSON object on standard output and
//! nothing on standard error (its warnings and summaries go in "notes"); a
//! failure prints {"error": …} and exits with an error. `mg find` and
//! `mg info` answer questions about Neverwinter Chess.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn run(root: &Path, user: &Path, args: &[&str]) -> (bool, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_mg"))
        .args(["--root", root.to_str().unwrap(), "--user-dir", user.to_str().unwrap(), "--json"])
        .args(args)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.trim().is_empty(), "{args:?}: stderr {stderr}");
    let json: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{args:?}: {e}: {}", String::from_utf8_lossy(&out.stdout)));
    assert!(json.is_object(), "{args:?}: {json}");
    (out.status.success(), json)
}

#[test]
fn every_command_answers_in_json() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-json");
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let chess = dir.join("chess.mod");
    std::fs::copy(root.join("data/mod/Neverwinter Chess.mod"), &chess).unwrap();
    let c = chess.to_str().unwrap();
    let at = |name: &str| dir.join(name).to_str().unwrap().to_string();
    let ok = |args: &[&str]| -> Value {
        let (success, json) = run(&root, &user, args);
        assert!(success, "{args:?}: {json}");
        assert!(json.get("error").is_none(), "{args:?}: {json}");
        json
    };

    // Archives and files.
    let ls = ok(&["ls", c]);
    assert!(ls["resources"].as_array().unwrap().len() > 300);
    ok(&["unpack", c, &at("unpacked")]);
    let packed = ok(&["pack", &at("unpacked"), &at("repacked.mod")]);
    assert_eq!(packed["files"], ls["resources"].as_array().unwrap().len());
    let gff = ok(&["gff", &at("unpacked/module.ifo")]);
    assert_eq!(gff["__data_type"], "IFO ");
    ok(&["gff", &at("unpacked/module.ifo"), "-o", &at("module.json")]);
    ok(&["gff", &at("module.json"), "-o", &at("module.ifo")]);

    // The game's resources.
    assert_eq!(ok(&["which", "classes.2da"])["resource"], "classes.2da");
    assert!(ok(&["cat", "classes.2da"])["text"].as_str().unwrap().starts_with("2DA"));
    assert!(ok(&["layers"])["layers"].as_array().unwrap().len() > 3);
    assert_eq!(ok(&["tlk", "12"])["strings"][0]["text"], "Paladin");

    // Modules.
    assert_eq!(ok(&["verify", c])["errors"], 0);
    assert!(ok(&["haks", c])["haks"].as_array().unwrap().is_empty());
    let exported = ok(&["export", c, "pawn_w.utc", "-o", &at("pawn.erf")]);
    assert!(exported["resources"].as_array().unwrap().iter().any(|r| r == "pawn_w.utc"));
    let compiled = ok(&["compile", c, "--uncompiled"]);
    assert_eq!(compiled["failed"], 0);
    let refs = ok(&["refs", c, "pawn_w.utc"]);
    assert!(refs["uses"].is_array() && refs["script_strings"].is_array());
    let renamed = ok(&["rename", c, "pawn_w.utc", "pawn_white"]);
    assert_eq!(renamed["to"], "pawn_white");
    let dialogs = ok(&["find", c, "--type", "utc", "--blueprints"]);
    assert!(!dialogs["found"].as_array().unwrap().is_empty());
    let info = ok(&["info", c]);
    assert_eq!(info["name"], "Neverwinter Chess");
    let dlg = info["resources"]["dlg"].as_u64().unwrap();
    assert!(dlg > 0);
    let m = mg_module::Module::open(&chess).unwrap();
    let conversation = m.keys_of(mg_core::ResType::DLG).next().unwrap().resref.to_string();
    ok(&["dialog-export", c, &format!("{conversation}.dlg"), &at("talk.ink")]);
    let imported = ok(&["dialog-import", c, &at("talk.ink"), "--name", "mg_talk"]);
    assert_eq!(imported["conversation"], "mg_talk.dlg");
    let replaced = ok(&["replace", c, "Pawn", "Footman", "--dry-run"]);
    assert!(replaced["times"].as_u64().unwrap() > 0);
    assert!(ok(&["update-instances", c])["updated"].is_number());
    let init = ok(&["init", c, &at("project")]);
    assert_eq!(init["target"], "default");
    let built = ok(&["build", &at("project"), "-o", &at("built.mod")]);
    assert_eq!(built["compiled"]["failed"], 0);
    ok(&["import", c, &at("pawn.erf")]);
    let hak = dir.join("mg_json.hak");
    std::fs::write(&hak, mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap()).unwrap();
    assert_eq!(ok(&["attach", c, hak.to_str().unwrap()])["haks"][0], "mg_json");
    let map = ok(&["minimap", c, "chess", &at("map.png")]);
    assert!(map["width"].as_u64().unwrap() > 0);
    let set = ok(&["cat", "tcn01.set"])["text"].as_str().unwrap().to_string();
    std::fs::write(dir.join("tcn01.set"), set).unwrap();
    let pal = ok(&["tileset-palette", &at("tcn01.set")]);
    assert!(pal["groups"].as_u64().unwrap() > 10 && dir.join("tcn01palstd.itp").is_file());
    let synced = ok(&["nwsync", c, &at("repo"), "--with-module"]);
    assert_eq!(synced["sha1"].as_str().unwrap().len(), 40);
    assert!(synced["files"].as_u64().unwrap() > 100);

    // A failure.
    let (success, json) = run(&root, &user, &["which", "nothing_at_all.2da"]);
    assert!(!success);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[test]
fn find_and_info() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-find");
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let c = chess.to_str().unwrap();
    let find = |args: &[&str]| -> Vec<Value> {
        let mut all = vec!["find", c];
        all.extend_from_slice(args);
        let (ok, json) = run(&root, &dir, &all);
        assert!(ok, "{json}");
        json["found"].as_array().unwrap().clone()
    };
    // The pawn blueprints, by name.
    let pawns = find(&["--type", "utc", "--name", "pawn"]);
    let resrefs: Vec<&str> = pawns.iter().map(|f| f["resref"].as_str().unwrap()).collect();
    assert_eq!(resrefs, ["pawn_b", "pawn_w"]);
    assert!(pawns.iter().all(|f| f["kind"] == "blueprint"));
    // What's placed in the area, where.
    let placed = find(&["--placed", "--area", "chess"]);
    let master = placed.iter().find(|f| f["tag"] == "gamemaster").unwrap();
    assert_eq!((master["type"].as_str(), master["area"].as_str()), (Some("utc"), Some("chess")));
    assert_eq!(master["position"].as_array().unwrap().len(), 3);
    // By a field's value, with a wildcard tag.
    let white = find(&["--blueprints", "--tag", "*_w", "--where", "Appearance_Type"]);
    assert_eq!(white.len(), 6, "{white:?}");
    assert!(find(&["--tag", "no_such_tag"]).is_empty());
}
