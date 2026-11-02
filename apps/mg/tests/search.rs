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
    let mut known = Struct::new(3);
    known.set("Spell", Value::Word(107));
    class.set("KnownList1", Value::List(vec![known]));
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
    let mut priestly = Struct::new(0);
    priestly.set("Name", Value::String(b"nPriestly".to_vec()));
    priestly.set("Type", Value::Dword(1));
    priestly.set("Value", Value::Int(1));
    utc.root.set("VarTable", Value::List(vec![priestly]));
    m.set_gff(ResKey::parse("priest", ResType::UTC).unwrap(), &utc).unwrap();
    // An encounter that spawns the priest.
    let mut spawn = Struct::new(0);
    spawn.set("ResRef", Value::resref(ResRef::from_str("priest").unwrap()));
    let mut ute = Gff::new(*b"UTE ");
    ute.root.set("Tag", Value::String(b"SHRINE".to_vec()));
    ute.root.set("CreatureList", Value::List(vec![spawn]));
    m.set_gff(ResKey::parse("shrine", ResType::UTE).unwrap(), &ute).unwrap();
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
    // A creature blueprint's, also through the encounter that spawns it.
    let priestly = found(&["--var", "nPriestly"]);
    let places: Vec<&str> = priestly.iter().map(|f| f["place"].as_str().unwrap()).collect();
    assert_eq!(places, ["priest.utc", "shrine.ute › spawns priest.utc"]);
    assert_eq!(
        (priestly[0]["spawned"].is_null(), &priestly[1]["spawned"]),
        (true, &"priest.utc".into())
    );
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
    // By regular expressions: anywhere unless anchored, case ignored.
    assert_eq!(found(&["--var", "^sdeidad$", "--value", "^e.ra$", "--regex"]).len(), 2);
    assert!(found(&["--var", "^sdeidad$"]).is_empty(), "a * pattern, not an expression");
    assert_eq!(found(&["--field", "^tag$", "--text", "^gua", "--regex"]).len(), 1);
    // Several conditions, all met: the guard has 3 fighter levels and
    // appearance 103; nothing has a fighter level and the variable.
    assert_eq!(found(&["--class", "4", "--appearance", "103"]).len(), 1);
    assert!(found(&["--class", "4", "--var", "sDeidad"]).is_empty());
    // A class by its levels, and a number in a range.
    assert_eq!(found(&["--class", "4", "--levels", "2..3"]).len(), 1);
    assert_eq!(found(&["--class", "4", "--levels", "3"]).len(), 1);
    assert!(found(&["--class", "4", "--levels", "4.."]).is_empty());
    let ranged = found(&["--field", "Appearance_Type", "--number", "100..110"]);
    assert_eq!(ranged.len(), 1);
    assert!(found(&["--field", "Appearance_Type", "--number", "..50"]).is_empty());
    // What it can't do.
    let fails = |args: &[&str]| {
        let mut all = vec![p];
        all.extend_from_slice(args);
        let (ok, json, _) = run(&nowhere, &all);
        assert!(!ok, "{args:?}");
        json["error"].as_str().unwrap_or_default().to_string()
    };
    assert!(fails(&[]).contains("say what to search for"));
    assert!(fails(&["--var", "x", "--property", "52", "--value", "3"]).contains("--value"));
    assert!(fails(&["--class", "4", "--levels", "a..b"]).contains("not a number"));
    assert!(fails(&["--class", "fighter"]).contains("row number"));
    assert!(fails(&["--var", "x", "--var-type", "dog"]).contains("not a variable type"));
    assert!(fails(&["--var", "x", "--type", "nss"]).contains("not a kind of object"));
    assert!(fails(&["--var", "dei(dad", "--regex"]).contains("dei(dad"));
}

/// `--any` of the conditions, and `--not` before one to leave out what
/// meets it, where it is on the command line.
#[test]
fn search_for_any_condition_and_leave_one_out() {
    let dir = mg_testkit::scratch_dir("mg-search-any-not");
    let path = module(&dir);
    let p = path.to_str().unwrap();
    let nowhere = dir.join("no-game");
    let ask = |args: &[&str]| {
        let mut all = vec![p];
        all.extend_from_slice(args);
        run(&nowhere, &all)
    };
    let found = |args: &[&str]| -> Vec<String> {
        let (ok, json, err) = ask(args);
        assert!(ok, "{args:?}: {err}");
        let found = json["found"].as_array().unwrap();
        found.iter().map(|f| f["place"].as_str().unwrap().to_string()).collect()
    };
    // The priest's variable, but not on what carries an item with the
    // property (the priest carries the amulet); nor on fighters (none).
    assert!(found(&["--var", "nPriestly", "--not", "--property", "52"]).is_empty());
    let priestly = ["priest.utc", "shrine.ute › spawns priest.utc"];
    assert_eq!(found(&["--var", "nPriestly", "--not", "--class", "4"]), priestly);
    // The guard has appearance 103.
    assert!(found(&["--class", "4", "--not", "--appearance", "103"]).is_empty());
    // --not goes with the condition after it, wherever that is.
    assert_eq!(found(&["--not", "--var", "nPriestly", "--class", "4"]), ["keep › creature GUARD"]);
    // Any of them: each one's.
    assert_eq!(
        found(&["--class", "4", "--var", "nPriestly", "--any"]),
        ["keep › creature GUARD", "priest.utc", "shrine.ute › spawns priest.utc"]
    );
    assert!(found(&["--class", "4", "--var", "nPriestly"]).is_empty(), "all of them");
    let either = ["--any", "--class", "4", "--var", "sDeidad", "--not", "--property", "52"];
    assert_eq!(found(&either), ["keep › creature GUARD"]);
    // A spell by the class that knows it: the guard's, as a fighter (4).
    assert_eq!(found(&["--spell", "107", "--spell-class", "4"]), ["keep › creature GUARD"]);
    assert!(found(&["--spell", "107", "--spell-class", "2"]).is_empty());
    assert!(!ask(&["--spell-class", "4"]).0, "a spell's class, without a spell");
    // The haks' and the game's, without the game: the module's, and said.
    let (ok, json, err) = ask(&["--class", "4", "--haks-and-game"]);
    assert!(ok && json["found"].as_array().unwrap().len() == 1, "{err}");
    assert!(json["notes"].to_string().contains("only the module's blueprints"), "{json}");
    // What it can't do.
    let fails = |args: &[&str]| {
        let (ok, json, _) = ask(args);
        assert!(!ok, "{args:?}");
        json["error"].as_str().unwrap_or_default().to_string()
    };
    assert!(fails(&["--class", "4", "--not"]).contains("--not goes before"));
    assert!(fails(&["--not", "--class", "4"]).contains("besides what to leave out"));
    assert!(fails(&["--not", "--not", "--class", "4", "--var", "x"]).contains("--not twice"));
    let both = ["--not", "--var", "nPriestly", "--not", "--class", "4"];
    assert!(fails(&both).contains("besides what to leave out"));
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
    // A spell by the class that has it: a cleric's Heal, known or
    // memorized as a cleric (not a special ability, nor another class's).
    let heals = found(&["--spell", "heal", "--spell-class", "cleric"]);
    let as_cleric = |f: &Json| f["match"].as_str().unwrap().ends_with("spell: Heal (Cleric)");
    assert!(heals.iter().all(as_cleric), "{heals:#?}");
    let any = found(&["--spell", "heal"]);
    assert!(any.len() >= heals.len());
    // The game's blueprints too, read-only, each with the layer it is read
    // from, after the module's (which name none).
    let all = found(&["--class", "Fighter", "--blueprints", "--haks-and-game"]);
    let module = found(&["--class", "Fighter", "--blueprints"]);
    let outside: Vec<&Json> = all.iter().filter(|f| f["layer"].is_string()).collect();
    assert!(!outside.is_empty() && all.len() > module.len());
    assert!(outside.iter().all(|f| f["type"] == "utc" && f["placed"] == false));
    let first_outside = all.iter().position(|f| f["layer"].is_string()).unwrap();
    assert!(all[first_outside..].iter().all(|f| f["layer"].is_string()), "the module's first");
    let (ok, _, err) = run(&root, &[c, "--class", "Fighter", "--placed", "--haks-and-game"]);
    assert!(!ok && err.contains("--haks-and-game"), "{err}");
}
