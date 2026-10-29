//! Rows by name on the command line, as the window's fields view shows
//! them: `mg gff --names` names the row a field's number stands for,
//! `mg areas --set` and `mg find --where` take a row's label or name in
//! place of its number, and `mg areas --names` shows what it changed by
//! name. Read from the game's 2DAs (a game install needed).

use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey};
use mg_rules::rows::row_table;
use serde_json::Value as Json;

fn key(name: &str, t: ResType) -> ResKey {
    ResKey::new(ResRef::from_str(name).unwrap(), t)
}

#[test]
fn rows_are_named_and_taken_by_name() {
    let root = mg_testkit::corpus!();
    let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let dir = mg_testkit::scratch_dir("mg-row-names");
    let mg = |args: &[&str]| {
        let out = crate::mg_on(&root, None).arg("--json").args(args).output().unwrap();
        let json = serde_json::from_slice(&out.stdout).unwrap_or(Json::Null);
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        (out.status.success(), json, stderr)
    };
    // A command that fails, and what it said.
    let fails = |args: &[&str]| {
        let out = crate::mg_on(&root, None).args(args).output().unwrap();
        assert!(!out.status.success(), "{args:?}");
        String::from_utf8_lossy(&out.stderr).into_owned()
    };
    // What the game's own tables name the rows used here.
    let race = row_table("UTC ", &[], "Race").unwrap();
    let human = game.row_name(&race, 6).unwrap();
    let label = |table: &str, row: usize, column: &str| {
        game.table(table).unwrap().get(row, column).unwrap().trim().to_string()
    };
    let dwarf_look = label("appearance", 0, "LABEL");
    let music = row_table("GIT ", &["AreaProperties"], "MusicDay").unwrap();
    let tune = label("ambientmusic", 5, "Resource");

    // A creature and an area in a module.
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut utc = Gff::new(*b"UTC ");
    utc.root.set("Tag", Value::String(b"guard".to_vec()));
    utc.root.set("Race", Value::Byte(6));
    utc.root.set("Appearance_Type", Value::Word(0));
    let mut skill = Struct::new(0);
    skill.set("Rank", Value::Byte(4));
    utc.root.set("SkillList", Value::List(vec![skill.clone(), skill]));
    m.set_gff(key("guard", ResType::UTC), &utc).unwrap();
    let mut utp = Gff::new(*b"UTP ");
    utp.root.set("Tag", Value::String(b"chest".to_vec()));
    utp.root.set("Appearance", Value::Dword(0));
    m.set_gff(key("chest", ResType::UTP), &utp).unwrap();
    let mut are = Gff::new(*b"ARE ");
    let title = LocString::from_text(Language::ENGLISH, Gender::Male, "Cave");
    are.root.set("Name", Value::LocString(title));
    m.set_gff(key("cave", ResType::ARE), &are).unwrap();
    let mut git = Gff::new(*b"GIT ");
    let mut props = Struct::new(100);
    props.set("MusicDay", Value::Int(3));
    git.root.set("AreaProperties", Value::Struct(props));
    m.set_gff(key("cave", ResType::GIT), &git).unwrap();
    let path = dir.join("rows.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let module = path.to_str().unwrap();

    // mg gff --names: the row beside the number; read back, the same GFF.
    let file = dir.join("guard.utc");
    std::fs::write(&file, utc.to_bytes().unwrap()).unwrap();
    let (ok, plain, _) = mg(&["gff", file.to_str().unwrap()]);
    assert!(ok && plain["Race"].get("row").is_none(), "names only when asked: {plain}");
    let (ok, named, e) = mg(&["gff", file.to_str().unwrap(), "--names"]);
    assert!(ok, "{e}");
    assert_eq!(
        (&named["Race"]["value"], named["Race"]["row"].as_str()),
        (&Json::from(6), Some(&*human))
    );
    let skills = mg_rules::rows::list_table("UTC ", &[], "SkillList").unwrap();
    let second = game.row_name(&skills, 1).unwrap();
    assert_eq!(named["SkillList"]["value"][1]["__row"].as_str(), Some(&*second));
    let json = dir.join("guard.utc.json");
    std::fs::write(&json, serde_json::to_string(&named).unwrap()).unwrap();
    let back = dir.join("back.utc");
    let (ok, _, e) = mg(&["gff", json.to_str().unwrap(), "-o", back.to_str().unwrap()]);
    assert!(ok, "{e}");
    assert_eq!(Gff::read(&std::fs::read(&back).unwrap()).unwrap(), utc, "names set aside");

    // mg find --where: a row by its label or its name, for each type the
    // field names a row of.
    let (ok, found, e) = mg(&["find", module, "--where", &format!("Appearance_Type={dwarf_look}")]);
    assert!(ok, "{e}");
    assert_eq!(found["found"].as_array().unwrap().len(), 1, "{found}");
    assert_eq!(found["found"][0]["tag"], "guard");
    let (ok, found, e) = mg(&["find", module, "--where", &format!("Race={human}")]);
    assert!(ok && found["found"][0]["tag"] == "guard", "{e} {found}");
    let chest = label("placeables", 0, "Label");
    let (ok, found, e) = mg(&["find", module, "--where", &format!("Appearance={chest}")]);
    assert!(ok, "{e}");
    let tags: Vec<&str> =
        found["found"].as_array().unwrap().iter().filter_map(|f| f["tag"].as_str()).collect();
    assert_eq!(tags, ["chest"], "a placeables.2da row, not a door's");
    let e = fails(&["find", module, "--where", "Race=Nobody At All"]);
    assert!(e.contains("no row named"), "{e}");
    // Numbers and patterns as before.
    let (ok, found, _) = mg(&["find", module, "--where", "Race=6"]);
    assert!(ok && found["found"].as_array().unwrap().len() == 1);

    // mg areas --set by a row's label, and --names says it by name.
    let (ok, done, e) =
        mg(&["areas", module, "--all", "--set", &format!("MusicDay={tune}"), "--names"]);
    assert!(ok, "{e}");
    let change = &done["changes"][0];
    assert_eq!((change["from"].as_str(), change["to"].as_str()), (Some("3"), Some("5")), "{done}");
    assert_eq!(change["to_name"].as_str(), game.row_name(&music, 5).as_deref());
    let git = Module::open(&path).unwrap().gff(&key("cave", ResType::GIT)).unwrap().unwrap();
    assert_eq!(git.root.child("AreaProperties").unwrap().integer("MusicDay"), Some(5));
    let out = crate::mg_on(&root, None)
        .args(["areas", module, "--all", "--set", "MusicDay=4", "--names", "--dry-run"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    let (from, to) = (game.row_name(&music, 5).unwrap(), game.row_name(&music, 4).unwrap());
    assert!(text.contains(&format!("5 ({from}) -> 4 ({to})")), "{text}");
    let e = fails(&["areas", module, "--all", "--set", "MusicDay=Nothing Like It"]);
    assert!(e.contains("ambientmusic.2da has no row named"), "{e}");
}

/// `mg gff --names` names what the window's fields view names beyond a
/// fixed table's row: an item property's subtype, cost table and value
/// (of the tables its other fields name), a creature's faction (with
/// --module, the module's own), a conversation line's animation, a
/// store's pages.
#[test]
fn item_properties_factions_animations_and_store_pages_are_named() {
    let root = mg_testkit::corpus!();
    let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let dir = mg_testkit::scratch_dir("mg-row-names-more");
    let names = |file: &std::path::Path, module: Option<&std::path::Path>| {
        let mut c = crate::mg_on(&root, None);
        c.args(["--json", "gff", file.to_str().unwrap(), "--names"]);
        if let Some(m) = module {
            c.arg("--module").arg(m);
        }
        let out = c.output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        serde_json::from_slice::<Json>(&out.stdout).unwrap()
    };
    let string = |table: &str, row: usize| {
        let t = game.table(table).unwrap();
        let strref = t.get_int(row, "Name").unwrap();
        game.string(mg_core::StrRef(strref as u32)).unwrap()
    };
    // An Ability Bonus of +2 to Strength (itempropdef.2da row 0; its
    // subtypes iprp_abilities.2da, its values iprp_costtable.2da row 1's
    // table, iprp_bonuscost.2da).
    let mut uti = Gff::new(*b"UTI ");
    let mut p = Struct::new(0);
    for (l, v) in [("PropertyName", 0u16), ("Subtype", 0), ("CostValue", 2)] {
        p.set(l, Value::Word(v));
    }
    p.set("CostTable", Value::Byte(1));
    p.set("Param1", Value::Byte(255));
    p.set("Param1Value", Value::Byte(0));
    uti.root.set("PropertiesList", Value::List(vec![p]));
    let file = dir.join("ring.uti");
    std::fs::write(&file, uti.to_bytes().unwrap()).unwrap();
    let named = names(&file, None);
    let p = &named["PropertiesList"]["value"][0];
    let cost = game.table("iprp_costtable").unwrap().get(1, "Name").unwrap().to_lowercase();
    assert_eq!(p["Subtype"]["row"].as_str(), Some(&*string("iprp_abilities", 0)), "{p}");
    assert_eq!(p["CostValue"]["row"].as_str(), Some(&*string(&cost, 2)), "{p}");
    assert!(p["CostTable"]["row"].is_string(), "{p}");
    assert!(p["Param1"].get("row").is_none(), "255: no parameter");
    // A creature's faction: the module's (its own named "Smugglers").
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut fac = mg_module::factions::Factions::of_module(&m);
    let smugglers = fac.add("Smugglers", false, 1);
    m.set_gff(key("repute", ResType::FAC), &fac.to_gff()).unwrap();
    let module = dir.join("factions.mod");
    m.save_as(&ModuleLocation::Archive(module.clone())).unwrap();
    let mut utc = Gff::new(*b"UTC ");
    utc.root.set("FactionID", Value::Word(smugglers as u16));
    let file = dir.join("smuggler.utc");
    std::fs::write(&file, utc.to_bytes().unwrap()).unwrap();
    assert_eq!(names(&file, Some(&module))["FactionID"]["row"].as_str(), Some("Smugglers"));
    assert!(names(&file, None)["FactionID"].get("row").is_none(), "no module: the number");
    // A conversation line's animation.
    let mut dlg = Gff::new(*b"DLG ");
    let mut line = Struct::new(0);
    line.set("Animation", Value::Dword(29));
    dlg.root.set("EntryList", Value::List(vec![line]));
    let file = dir.join("hello.dlg");
    std::fs::write(&file, dlg.to_bytes().unwrap()).unwrap();
    let named = names(&file, None);
    assert_eq!(named["EntryList"]["value"][0]["Animation"]["row"].as_str(), Some("Greeting"));
    // A store's pages, by their structs' ids.
    let mut utm = Gff::new(*b"UTM ");
    utm.root.set("StoreList", Value::List(vec![Struct::new(0), Struct::new(4)]));
    let file = dir.join("shop.utm");
    std::fs::write(&file, utm.to_bytes().unwrap()).unwrap();
    let named = names(&file, None);
    let pages: Vec<&str> = named["StoreList"]["value"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["__row"].as_str())
        .collect();
    assert_eq!(pages, ["Armor", "Weapons"]);
}
