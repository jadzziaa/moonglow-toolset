//! `mg search`: a module the test builds searched by variable, item
//! property, class and number, without the game (things are named by
//! their rows); then Darkness over Daggerford's or the Original
//! Campaign's searched by names the game gives (a skill, a class).

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use serde_json::Value as Json;

fn run(root: &std::path::Path, args: &[&str]) -> (bool, Json, String) {
    let out = crate::mg_on(root, None).args(["--json", "search"]).args(args).output().unwrap();
    let json = serde_json::from_slice(&out.stdout).unwrap_or(Json::Null);
    (out.status.success(), json, String::from_utf8_lossy(&out.stderr).into_owned())
}

fn module(dir: &std::path::Path) -> std::path::PathBuf {
    let mut m = Module::new();
    let mut var = Struct::new(0);
    var.set("Name", Value::String(b"sDeidad".to_vec()));
    var.set("Type", Value::Dword(3));
    var.set("Value", Value::String(b"Ezra".to_vec()));
    let mut bonus = Struct::new(0);
    for (l, v) in [("PropertyName", 52), ("Subtype", 5), ("CostValue", 3)] {
        bonus.set(l, Value::Word(v));
    }
    bonus.set("CostTable", Value::Byte(25));
    bonus.set("Param1", Value::Byte(255));
    let mut uti = Gff::new(*b"UTI ");
    uti.root.set("Tag", Value::String(b"AMULET".to_vec()));
    uti.root.set("VarTable", Value::List(vec![var]));
    uti.root.set("PropertiesList", Value::List(vec![bonus]));
    m.set_gff(ResKey::parse("amulet", ResType::UTI).unwrap(), &uti).unwrap();
    let mut class = Struct::new(2);
    class.set("Class", Value::Int(4));
    class.set("ClassLevel", Value::Short(3));
    let mut guard = Struct::new(4);
    guard.set("Tag", Value::String(b"GUARD".to_vec()));
    guard.set("ClassList", Value::List(vec![class]));
    guard.set("Appearance_Type", Value::Word(103));
    let mut git = Gff::new(*b"GIT ");
    git.root.set("Creature List", Value::List(vec![guard]));
    m.set_gff(ResKey::parse("keep", ResType::GIT).unwrap(), &git).unwrap();
    let mut held = Struct::new(0);
    held.set("InventoryRes", Value::resref(ResRef::from_str("amulet").unwrap()));
    let mut utc = Gff::new(*b"UTC ");
    utc.root.set("Tag", Value::String(b"PRIEST".to_vec()));
    utc.root.set("ItemList", Value::List(vec![held]));
    m.set_gff(ResKey::parse("priest", ResType::UTC).unwrap(), &utc).unwrap();
    let path = dir.join("search.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

#[test]
fn search_a_module_without_the_game() {
    let dir = mg_testkit::scratch_dir("mg-search");
    let path = module(&dir);
    let p = path.to_str().unwrap();
    // (No game here: things are named by their rows.)
    let nowhere = dir.join("no-game");
    let found = |args: &[&str]| -> Vec<Json> {
        let mut all = vec![p];
        all.extend_from_slice(args);
        let (ok, json, err) = run(&nowhere, &all);
        assert!(ok, "{args:?}: {err}");
        json["found"].as_array().unwrap().clone()
    };
    let vars = found(&["--var", "*sdeidad*"]);
    let places: Vec<&str> = vars.iter().map(|f| f["place"].as_str().unwrap()).collect();
    assert_eq!(places, ["amulet.uti", "priest.utc › carries amulet.uti"]);
    assert_eq!(vars[0]["match"], "sDeidad (string) = \"Ezra\"");
    assert_eq!(
        (vars[0]["resource"].as_str(), vars[0]["path"].as_str()),
        (Some("amulet.uti"), Some("/VarTable[0]"))
    );
    assert_eq!(vars[1]["carried"], "amulet.uti");
    assert_eq!(found(&["--var", "sDeidad", "--var-type", "int"]).len(), 0);
    assert_eq!(found(&["--var", "sDeidad", "--value", "ez*", "--blueprints"]).len(), 2);
    let bonus = found(&["--property", "52", "--subtype", "5", "--value", "3"]);
    assert_eq!(bonus.len(), 2);
    assert_eq!(bonus[0]["type"], "uti");
    let fighters = found(&["--class", "4", "--placed"]);
    assert_eq!(fighters.len(), 1);
    let f = &fighters[0];
    assert_eq!(
        (f["place"].as_str(), f["area"].as_str()),
        (Some("keep › creature GUARD"), Some("keep"))
    );
    assert_eq!(
        (f["match"].as_str(), f["index"].as_u64()),
        (Some("Class: row 4, level 3"), Some(0))
    );
    // Every field holding 103, and the creature's appearance 103.
    let n = found(&["--int", "103"]);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0]["path"], "/Creature List[0]/Appearance_Type");
    assert_eq!(found(&["--appearance", "103"]).len(), 1);
    assert!(found(&["--appearance", "103", "--of", "placeable"]).is_empty());
    assert_eq!(found(&["--field", "tag", "--text", "GUA"]).len(), 1);
    assert_eq!(found(&["--text", "ezra", "--type", "utc"]).len(), 0);
    // What it can't do.
    let fails = |args: &[&str]| {
        let mut all = vec![p];
        all.extend_from_slice(args);
        let (ok, json, _) = run(&nowhere, &all);
        assert!(!ok, "{args:?}");
        json["error"].as_str().unwrap_or_default().to_string()
    };
    assert!(fails(&[]).contains("say what to search for"));
    assert!(fails(&["--class", "4", "--feat", "1"]).contains("one kind"));
    assert!(fails(&["--class", "fighter"]).contains("row number"));
    assert!(fails(&["--var", "x", "--var-type", "dog"]).contains("not a variable type"));
    assert!(fails(&["--var", "x", "--type", "nss"]).contains("not a kind of object"));
}

#[test]
fn search_a_campaign_by_the_names_the_game_gives() {
    let root = mg_testkit::corpus!();
    let chapter = root.join("data/nwm/Chapter1.nwm");
    if !chapter.is_file() {
        eprintln!("skipped: the Original Campaign isn't installed");
        return;
    }
    let c = chapter.to_str().unwrap();
    let found = |args: &[&str]| -> Vec<Json> {
        let mut all = vec![c];
        all.extend_from_slice(args);
        let (ok, json, err) = run(&root, &all);
        assert!(ok, "{args:?}: {err}");
        json["found"].as_array().unwrap().clone()
    };
    // Items giving a bonus to a skill, by the names the game gives.
    let traps = found(&["--property", "skill bonus", "--subtype", "set trap"]);
    assert!(!traps.is_empty());
    assert!(
        traps.iter().all(|f| f["match"].as_str().unwrap().starts_with("Skill Bonus: Set Trap"))
    );
    assert!(traps.iter().all(|f| f["type"] == "uti"));
    // Creatures with a class, placed in areas.
    let fighters = found(&["--class", "Fighter", "--placed"]);
    assert!(fighters.len() > 5, "{}", fighters.len());
    assert!(fighters.iter().all(|f| f["match"].as_str().unwrap().starts_with("Class: Fighter")));
    assert!(fighters.iter().all(|f| f["placed"] == true && f["area"].is_string()));
}
