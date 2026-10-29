//! `mg verify`: a clean module passes; one with a placed object naming a
//! placeables.2da row that doesn't exist fails, the JSON naming it.

use mg_core::{ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

#[test]
fn verify_fails_on_errors_and_names_them_in_json() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify");
    let verify = |path: &std::path::Path| {
        crate::mg()
            .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
            .arg(path)
            .output()
            .unwrap()
    };
    let clean = verify(&chess);
    assert!(clean.status.success(), "{}", String::from_utf8_lossy(&clean.stderr));

    let mut m = Module::open(&chess).unwrap();
    let git = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    let mut p = Struct::new(9);
    p.set("Appearance", Value::Dword(99_999));
    p.set("Tag", Value::String(b"BROKEN".to_vec()));
    g.root.set("Placeable List", Value::List(vec![p]));
    m.set_gff(git, &g).unwrap();
    let broken = dir.join("broken.mod");
    m.save_as(&ModuleLocation::Archive(broken.clone())).unwrap();
    let out = verify(&broken);
    assert!(!out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["errors"], 1);
    let f = &json["findings"][0];
    assert_eq!(f["check"], "object-row");
    assert_eq!(f["severity"], "error");
    assert!(
        f["at"].as_str().unwrap().eq_ignore_ascii_case("chess › placeable BROKEN › Appearance")
    );
    // Its group left out (`--skip`, repeated or separated by commas): it
    // isn't checked, so it passes, and says which groups it left out.
    for skip in [&["--skip", "objects", "--skip", "tilesets"][..], &["--skip", "objects,tilesets"]]
    {
        let out = crate::mg()
            .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
            .args(skip)
            .arg(&broken)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(json["errors"], 0);
        assert_eq!(json["skipped"], serde_json::json!({ "objects": null, "tilesets": null }));
    }
    // `--timings` says how long each part took, on stderr; the report is
    // the same.
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
        .arg("--timings")
        .arg(&broken)
        .output()
        .unwrap();
    let timed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(timed["findings"], json["findings"]);
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains(" s  open the module") && said.contains(" s  in all"), "{said}");
    let wrong = crate::mg().args(["verify", "--skip", "tiles"]).arg(&broken).output().unwrap();
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("one of references, tilesets"));
}

/// Variables in an area's ARE, which the game does not read (it reads an
/// area's from its GIT: `engine_area_variables`), as Moonglow wrote them
/// before 1.34.3: Verify fails, naming them.
#[test]
fn verify_finds_area_variables_the_game_does_not_read() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-area-vars");
    let mut m = Module::open(&chess).unwrap();
    let are = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::ARE);
    let mut g = m.gff(&are).unwrap().unwrap();
    let mut v = Struct::new(0);
    v.set("Name", Value::String(b"iNoMap".to_vec()));
    v.set("Type", Value::Dword(1));
    v.set("Value", Value::Int(1));
    g.root.set("VarTable", Value::List(vec![v]));
    m.set_gff(are, &g).unwrap();
    let path = dir.join("area-vars.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let f = &json["findings"][0];
    assert_eq!(f["check"], "area-variables");
    assert!(f["message"].as_str().unwrap().contains("iNoMap"), "{f}");
    // Its fix, as `mg apply` takes it, and made when asked: the error is
    // gone, so Verify passes; the variables are in the GIT.
    assert_eq!(f["fix"]["label"], "Move the variables to the GIT");
    assert_eq!(f["fix"]["command"]["edits"].as_array().unwrap().len(), 2, "{f}");
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify"])
        .args(["--fix", "area-variables"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("fixed\tarea-variables › chess.are: Move the variables"), "{text}");
    let m = Module::open(&path).unwrap();
    assert!(m.gff(&are).unwrap().unwrap().root.get("VarTable").is_none());
    let git = m.gff(&ResKey::new(are.resref, ResType::GIT)).unwrap().unwrap();
    assert_eq!(git.root.list("VarTable").unwrap()[0].string("Name"), Some(&b"iNoMap"[..]));
}

/// A placed creature of faction 0, the PC faction (Jasperre's report: the
/// game puts it in Hostile, `engine_factions.rs`): a warning, so Verify
/// still passes; `mg checks` lists the check.
#[test]
fn verify_warns_of_an_object_of_the_pc_faction() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-pc-faction");
    let mut m = Module::open(&chess).unwrap();
    let git = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    let mut creatures = g.root.list("Creature List").unwrap_or(&[]).to_vec();
    let mut c = creatures.first().cloned().expect("a creature in the chess area");
    c.set("FactionID", Value::Word(0));
    c.set("Tag", Value::String(b"MG_PC_GUARD".to_vec()));
    creatures.push(c);
    g.root.set("Creature List", Value::List(creatures));
    m.set_gff(git, &g).unwrap();
    let path = dir.join("pc-faction.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let found: Vec<&serde_json::Value> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == "pc-faction")
        .collect();
    assert_eq!(found.len(), 1, "{json}");
    assert_eq!(found[0]["severity"], "warning");
    assert!(found[0]["at"].as_str().unwrap().contains("MG_PC_GUARD"), "{}", found[0]);
    let checks = crate::mg().args(["--json", "checks"]).output().unwrap();
    let checks: serde_json::Value = serde_json::from_slice(&checks.stdout).unwrap();
    let check = checks["checks"].as_array().unwrap().iter().find(|c| c["id"] == "pc-faction");
    assert!(check.is_some_and(|c| c["group"] == "objects"), "{checks}");
}

/// A tag the project's moonglow.toml wants unique, on two waypoints of a
/// module folder: a warning (Verify passes), `tag-unique`, listed by `mg
/// checks` with the objects' checks.
#[test]
fn verify_warns_of_a_unique_tag_on_two_objects() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-unique-tag").join("chess");
    let mut m = Module::open(&chess).unwrap();
    let git = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    let mut w = Struct::new(5);
    w.set("Tag", Value::String(b"TDOD_START_KREZK".to_vec()));
    w.set("XPosition", Value::Float(1.0));
    w.set("YPosition", Value::Float(1.0));
    g.root.set("WaypointList", Value::List(vec![w.clone(), w]));
    m.set_gff(git, &g).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(dir.join("moonglow.toml"), "[tags]\nunique = [\"TDOD_START_*\"]\n").unwrap();
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--json"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let found: Vec<&serde_json::Value> = json["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["check"] == "tag-unique")
        .collect();
    assert_eq!(found.len(), 1, "{json}");
    assert_eq!(found[0]["severity"], "warning");
    assert_eq!(found[0]["at"], "waypoint 0 in chess, waypoint 1 in chess");
    assert_eq!(found[0]["fix"]["label"], "Retag the other as TDOD_START_KREZK_2");
    let checks = crate::mg().args(["--json", "checks"]).output().unwrap();
    let checks: serde_json::Value = serde_json::from_slice(&checks.stdout).unwrap();
    let check = checks["checks"].as_array().unwrap().iter().find(|c| c["id"] == "tag-unique");
    assert!(check.is_some_and(|c| c["group"] == "objects"), "{checks}");

    // The fix, said and not made (--dry-run), then made: the second
    // waypoint has a tag of its own, and Verify finds nothing.
    let fix = |more: &[&str]| {
        crate::mg()
            .args(["--root", root.to_str().unwrap(), "--no-user-dir", "--json", "verify"])
            .args(["--fix", "tag-unique"])
            .args(more)
            .arg(&dir)
            .output()
            .unwrap()
    };
    let tags = || -> Vec<Vec<u8>> {
        let m = Module::open(&dir).unwrap();
        let g = m.gff(&git).unwrap().unwrap();
        let points = g.root.list("WaypointList").unwrap();
        points.iter().map(|w| w.string("Tag").unwrap().to_vec()).collect()
    };
    let out = fix(&["--dry-run"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["fixes"]["made"][0]["label"], "Retag the other as TDOD_START_KREZK_2");
    assert_eq!(json["fixes"]["dry_run"], true);
    assert_eq!(tags(), [b"TDOD_START_KREZK".to_vec(), b"TDOD_START_KREZK".to_vec()]);
    let out = fix(&[]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(tags(), [b"TDOD_START_KREZK".to_vec(), b"TDOD_START_KREZK_2".to_vec()]);
    let out = fix(&[]);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(!json["findings"].as_array().unwrap().iter().any(|f| f["check"] == "tag-unique"));
    let said = json["notes"].to_string();
    assert!(said.contains("--fix tag-unique: no finding of that check has a fix"), "{said}");
    // Only with a check named.
    let none = crate::mg().args(["verify", "--dry-run"]).arg(&dir).output().unwrap();
    assert!(!none.status.success());
}

/// A module naming a talk table the game can't find doesn't load: Verify
/// fails on it. The table in the module's own resources is found, as the
/// game finds it there (`tests/engine_tlk.rs`).
#[test]
fn verify_finds_the_talk_table_where_the_game_does() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-tlk");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("tlk")).unwrap();
    let verify = |path: &std::path::Path| {
        let out = crate::mg()
            .args(["--root", root.to_str().unwrap(), "--user-dir", user.to_str().unwrap()])
            .args(["verify", "--json"])
            .arg(path)
            .output()
            .unwrap();
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()
    };
    let mut m = Module::open(&chess).unwrap();
    let mut ifo = m.info().unwrap();
    ifo.root.set("Mod_CustomTlk", Value::String(b"mg_verify".to_vec()));
    m.set_info(&ifo).unwrap();
    let path = dir.join("named.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let json = verify(&path);
    assert_eq!(json["errors"], 1, "{json}");
    assert_eq!(json["findings"][0]["check"], "custom-tlk");

    let tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH).to_bytes().unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_verify").unwrap(), ResType::TLK), tlk.clone());
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    assert_eq!(verify(&path)["errors"], 0);
    // In the tlk folder instead.
    std::fs::write(user.join("tlk/mg_verify.tlk"), tlk).unwrap();
    m.remove(&ResKey::new(ResRef::from_str("mg_verify").unwrap(), ResType::TLK));
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    assert_eq!(verify(&path)["errors"], 0);
}

/// `mg verify --unused` says a script once, with the files the module
/// has of it; a script the game's spells.2da runs, or a string in another
/// script names, is used; and what `mg used` counts as used (moonglow.toml's
/// `[used]`) is not said.
#[test]
fn verify_unused_says_a_script_once_and_counts_what_runs_it() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-unused").join("chess");
    let mut m = Module::open(&chess).unwrap();
    let key = |n: &str, t| ResKey::new(ResRef::from_str(n).unwrap(), t);
    let main = b"void main() {}\n".to_vec();
    for (n, t) in [
        ("mg_both", ResType::NSS),
        ("mg_both", ResType::NCS),
        ("mg_source", ResType::NSS),
        ("mg_compiled", ResType::NCS),
        ("mg_ai_fight", ResType::NCS),
        ("mg_named", ResType::NCS),
        // The game's spells.2da runs it (Acid Fog's impact script).
        ("nw_s0_acidfog", ResType::NCS),
    ] {
        m.set(key(n, t), main.clone());
    }
    m.set(
        key("mg_caller", ResType::NSS),
        b"void main() { ExecuteScript(\"mg_named\", OBJECT_SELF); }\n".to_vec(),
    );
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let unused = || {
        let out = crate::mg()
            .args(["--root", root.to_str().unwrap(), "--no-user-dir", "--json", "verify"])
            .arg("--unused")
            .arg(&dir)
            .output()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let mut ours: Vec<(String, serde_json::Value)> = json["unused"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|u| {
                let r = u["resource"].as_str().unwrap();
                r.starts_with("mg_") || r.starts_with("nw_s0_acidfog")
            })
            .map(|u| (u["resource"].as_str().unwrap().to_string(), u["files"].clone()))
            .collect();
        ours.sort_by(|a, b| a.0.cmp(&b.0));
        ours
    };
    let s = |v: &str| serde_json::json!(v);
    assert_eq!(
        unused(),
        [
            ("mg_ai_fight.ncs".into(), s("compiled")),
            ("mg_both.nss".into(), s("both")),
            ("mg_caller.nss".into(), s("source")),
            ("mg_compiled.ncs".into(), s("compiled")),
            ("mg_source.nss".into(), s("source")),
        ]
    );
    // As text: a line a script, with what it has.
    let out = crate::mg()
        .args(["--root", root.to_str().unwrap(), "--no-user-dir", "verify", "--unused"])
        .arg(&dir)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("unused\tmg_both.nss\tsource and compiled\n"), "{text}");
    assert!(text.contains("unused\tmg_compiled.ncs\tcompiled only (no source)\n"), "{text}");
    assert!(text.contains("unused\tmg_source.nss\tsource only (not compiled)\n"), "{text}");
    assert!(!text.contains("mg_both.ncs"), "{text}");

    // Marked as used: in moonglow.toml, made by `mg used`.
    let used = |args: &[&str]| {
        let out = crate::mg().args(["--json", "used"]).arg(&dir).args(args).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()
    };
    let json = used(&["--add", "mg_ai_*", "--add", "mg_compiled"]);
    assert_eq!(json["scripts"], serde_json::json!(["mg_ai_*", "mg_compiled"]));
    let toml = std::fs::read_to_string(dir.join("moonglow.toml")).unwrap();
    assert!(toml.contains("[used]") && toml.contains("mg_ai_*"), "{toml}");
    assert_eq!(
        unused(),
        [
            ("mg_both.nss".into(), s("both")),
            ("mg_caller.nss".into(), s("source")),
            ("mg_source.nss".into(), s("source")),
        ]
    );
    let json = used(&["--remove", "mg_compiled"]);
    assert_eq!(json["scripts"], serde_json::json!(["mg_ai_*"]));
    // Where it is used: by the game's table, or by the project file.
    let refs = |name: &str| {
        let out = crate::mg()
            .args(["--root", root.to_str().unwrap(), "--no-user-dir", "--json", "refs"])
            .arg(&dir)
            .arg(name)
            .output()
            .unwrap();
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()
    };
    let json = refs("nw_s0_acidfog.nss");
    let at = json["tables"].as_array().unwrap();
    let spell = |t: &serde_json::Value| {
        t["table"] == "spells.2da" && t["at"].as_str().unwrap().ends_with(", ImpactScript")
    };
    assert!(at.iter().any(spell), "{json}");
    assert_eq!(refs("mg_ai_fight.nss")["marked"], "mg_ai_*");
}
