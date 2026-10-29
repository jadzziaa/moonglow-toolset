//! `--json`: every command prints one JSON object on standard output and
//! nothing on standard error (its warnings and summaries go in "notes"); a
//! failure prints {"error": …} and exits with an error. `mg find` and
//! `mg info` answer questions about Neverwinter Chess.

use std::path::Path;

use serde_json::Value;

fn run(root: &Path, user: &Path, args: &[&str]) -> (bool, Value) {
    let (ok, json) = crate::quiet_json_out(crate::mg_on(root, Some(user)).arg("--json").args(args));
    assert!(json.is_object(), "{args:?}: {json}");
    (ok, json)
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
    assert_eq!(ok(&["roundtrip", c])["differences"], serde_json::json!([]));
    // A model's files (an armoire's: all the game's own, so all written),
    // and not written over unasked.
    let files = ok(&["model-files", "plc_a01", &at("files")]);
    let names: Vec<&str> =
        files["files"].as_array().unwrap().iter().map(|f| f["file"].as_str().unwrap()).collect();
    assert!(names.contains(&"plc_a01.mdl") && names.contains(&"plc_a01.pwk"), "{names:?}");
    assert!(names.iter().all(|n| std::path::Path::new(&at("files")).join(n).is_file()));
    assert_eq!(files["left_out"], 0);
    let listed = ok(&["model-files", "plc_a01", &at("files"), "--dry-run"]);
    assert_eq!(listed["files"], files["files"]);
    ok(&["model-files", "plc_a01", &at("files"), "--force"]);
    let refused =
        crate::mg_on(&root, None).args(["model-files", "plc_a01", &at("files")]).output().unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--force"));
    // (What it names and the game lacks: the armoire's emitter names a
    // cloud texture the game has none of.)
    let missing = files["missing"].as_array().unwrap();
    assert!(missing.iter().all(|m| !names.contains(&m.as_str().unwrap())), "{missing:?}");
    // A blueprint's: the blueprint itself first, then its row of
    // placeables.2da, a 2DA of that row as it is numbered there.
    let chest = ok(&["model-files", "plc_chest1.utp", &at("chest")]);
    let names: Vec<&str> =
        chest["files"].as_array().unwrap().iter().map(|f| f["file"].as_str().unwrap()).collect();
    assert_eq!(names[0], "plc_chest1.utp", "{names:?}");
    assert!(names[1].starts_with("placeables.2da-row-"), "{names:?}");
    let row = std::fs::read_to_string(std::path::Path::new(&at("chest")).join(names[1])).unwrap();
    assert!(row.starts_with("2DA V2.0") && row.contains("ModelName"), "{row}");
    assert!(names.iter().any(|n| n.ends_with(".mdl")), "{names:?}");
    // A waypoint's: its flag, waypoint.2da's model of its appearance (the
    // module's own blueprint: the game's have no appearance, no flag).
    let utw = serde_json::json!({
        "__data_type": "UTW ",
        "Tag": { "type": "cexostring", "value": "MG_FLAG" },
        "TemplateResRef": { "type": "resref", "value": "mg_flag" },
        "Appearance": { "type": "byte", "value": 3 },
    });
    std::fs::write(at("mg_flag.utw.json"), utw.to_string()).unwrap();
    ok(&["gff", &at("mg_flag.utw.json"), "-o", &at("unpacked/mg_flag.utw")]);
    let module = at("unpacked");
    let flag = ok(&["model-files", "mg_flag.utw", &at("flag"), "--all", "--module", &module]);
    let names: Vec<&str> =
        flag["files"].as_array().unwrap().iter().map(|f| f["file"].as_str().unwrap()).collect();
    assert_eq!(names[0], "mg_flag.utw", "{names:?}");
    assert!(names.contains(&"gi_waypoint03.mdl"), "{names:?}");
    assert!(names[1].starts_with("waypoint.2da-row-"), "{names:?}");
    // An area's: its own .are and .git, its tileset, its tiles' models and
    // walkmeshes, and the models of what is placed in it (the chess
    // pieces, creatures).
    let area = ok(&["model-files", "chess.are", &at("area"), "--module", c]);
    let names: Vec<&str> =
        area["files"].as_array().unwrap().iter().map(|f| f["file"].as_str().unwrap()).collect();
    assert_eq!(names[..2], ["chess.are", "chess.git"], "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".set")), "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".wok")), "{names:?}");
    assert!(names.iter().filter(|n| n.ends_with(".mdl")).count() > 5, "{names:?}");
    // Plugins: the plugin host's own fixture, listed, checked, run and
    // its checks counted.
    let plugins = concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/mg-plugin/tests/fixtures");
    let plugin = format!("{plugins}/tag-conventions");
    assert!(ok(&["plugin", "list", plugins])["plugins"].as_array().unwrap().len() >= 4);
    assert_eq!(ok(&["plugin", "check", &plugin])["plugins"][0]["faults"], serde_json::json!([]));
    ok(&["plugin", "run", c, &plugin, "fix-tags", "--dry-run"]);
    assert_eq!(ok(&["verify", c, "--plugins", &plugin])["errors"], 0);
    let packed = ok(&["plugin", "pack", &plugin, "-o", &at("tags.zip")]);
    assert_eq!(packed["id"], "example.tag-conventions");
    let installed = ok(&["plugin", "install", &at("tags.zip"), &at("plugins")]);
    assert_eq!(installed["replaced"], Value::Null);
    ok(&["plugin", "remove", "example.tag-conventions", &at("plugins")]);
    let checks = ok(&["checks"]);
    assert!(checks["checks"].as_array().unwrap().iter().any(|c| c["id"] == "set-model"));
    assert!(ok(&["haks", c])["haks"].as_array().unwrap().is_empty());
    let exported = ok(&["export", c, "pawn_w.utc", "-o", &at("pawn.erf")]);
    assert!(exported["resources"].as_array().unwrap().iter().any(|r| r == "pawn_w.utc"));
    assert_eq!(exported["stamp"]["moonglow"], env!("CARGO_PKG_VERSION"));
    assert_eq!(exported["stamp"]["module"], "Neverwinter Chess");
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
    let set = ok(&["set", c, "module.ifo", "Mod_Tag=CHESS", "--dry-run"]);
    assert_eq!(set["command"]["edits"][0]["field"], "/Mod_Tag");
    std::fs::write(at("edits.json"), set["command"].to_string()).unwrap();
    assert_eq!(
        ok(&["apply", c, &at("edits.json")])["resources"],
        serde_json::json!(["module.ifo"])
    );
    assert!(ok(&["update-instances", c])["updated"].is_number());
    let init = ok(&["init", c, &at("project")]);
    assert_eq!(init["target"], "default");
    let built = ok(&["build", &at("project"), "-o", &at("built.mod")]);
    assert_eq!(built["compiled"]["failed"], 0);
    let imported = ok(&["import", c, &at("pawn.erf")]);
    assert_eq!(imported["stamp"], exported["stamp"], "the stamp reads back");
    let hak = dir.join("mg_json.hak");
    std::fs::write(&hak, mg_erf::ErfWriter::new(*b"HAK ").to_bytes().unwrap()).unwrap();
    assert_eq!(ok(&["attach", c, hak.to_str().unwrap()])["haks"][0], "mg_json");
    let map = ok(&["minimap", c, "chess", &at("map.png")]);
    assert!(map["width"].as_u64().unwrap() > 0);
    let set = ok(&["cat", "tcn01.set"])["text"].as_str().unwrap().to_string();
    std::fs::write(dir.join("tcn01.set"), set).unwrap();
    let pal = ok(&["tileset-palette", &at("tcn01.set")]);
    assert!(pal["groups"].as_u64().unwrap() > 10 && dir.join("tcn01palstd.itp").is_file());
    // A NUI window: added, checked, its scripts written.
    assert_eq!(ok(&["nui", "new", c, "mg_window"])["saved"], true);
    assert_eq!(ok(&["nui", "validate", c, "mg_window"])["errors"], 0);
    let generated = ok(&["nui", "generate", c, "mg_window"]);
    assert!(generated["resources"].as_array().unwrap().iter().any(|r| r == "mg_window_e.ncs"));
    let synced = ok(&["nwsync", c, &at("repo"), "--with-module"]);
    assert_eq!(synced["sha1"].as_str().unwrap().len(), 40);
    assert!(synced["files"].as_u64().unwrap() > 100);
    // Nothing unused in it: nothing to prune.
    let pruned = ok(&["nwsync-prune", &at("repo"), "--dry-run"]);
    assert_eq!(
        (pruned["manifests"].as_u64(), pruned["removed"].as_array().map(Vec::len)),
        (Some(1), Some(0))
    );
    assert_eq!(pruned["problems"], serde_json::json!([]));

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
    // A type that is no blueprint's is refused, as `mg search` refuses
    // one that is no object's.
    let (ok, refused) = run(&root, &dir, &["find", c, "--type", "nss"]);
    let why = refused["error"].as_str().unwrap_or_default();
    assert!(!ok && why.contains("nss: not a blueprint type"), "{refused}");
    // The pawn blueprints, by name.
    let pawns = find(&["--type", ".utc", "--name", "pawn"]);
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
