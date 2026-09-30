//! Blueprints Moonglow wrote, in the engine: a new module with an edited
//! copy of a base-game blueprint of every type. The engine creates the
//! creature, item, placeable, store and waypoint from their blueprints
//! (`CreateObject`); the door, trigger, encounter and sound, which scripts
//! cannot create, stand in the area as instances of theirs. It reports each
//! object with the values the edits set, and the values Moonglow derives
//! (an item's cost, a creature's hit points) agree with the engine's.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::{CreatureSheet, GameData, ItemValue};
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::compile;

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
string I(int n) { return IntToString(n); }

void Report(object o, string sExtra)
{
    Log("MG_OBJ " + GetTag(o) + "|" + I(GetIsObjectValid(o)) + "|" + I(GetObjectType(o)) + "|" + sExtra);
}

void main()
{
    location l = GetStartingLocation();
    object oArea = GetAreaFromLocation(l);

    object c = CreateObject(OBJECT_TYPE_CREATURE, "mg_utc", l);
    Report(c, I(GetAbilityScore(c, ABILITY_STRENGTH, TRUE)) + "|" + I(GetMaxHitPoints(c)) + "|" + I(GetHitDice(c)));

    object i = CreateObject(OBJECT_TYPE_ITEM, "mg_uti", l);
    Report(i, I(GetGoldPieceValue(i)) + "|" + I(GetItemHasItemProperty(i, ITEM_PROPERTY_ENHANCEMENT_BONUS)));

    object p = CreateObject(OBJECT_TYPE_PLACEABLE, "mg_utp", l);
    Report(p, GetTag(GetFirstItemInInventory(p)) + "|" + I(GetHasInventory(p)));

    object s = CreateObject(OBJECT_TYPE_STORE, "mg_utm", l);
    Report(s, I(GetStoreGold(s)) + "|" + GetTag(GetFirstItemInInventory(s)));

    object w = CreateObject(OBJECT_TYPE_WAYPOINT, "mg_utw", l);
    Report(w, GetName(w));

    object d = GetObjectByTag("MG_UTD");
    Report(d, I(GetLocked(d)) + "|" + GetResRef(d));

    object t = GetObjectByTag("MG_UTT");
    Report(t, GetResRef(t));

    object e = GetObjectByTag("MG_UTE");
    Report(e, I(GetEncounterActive(e)) + "|" + GetResRef(e));

    object n = GetObjectByTag("MG_UTS");
    Report(n, "");

    Log("MG_DONE");
}
"#;

fn resref(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

/// A base-game blueprint's fields, renamed to `name` and tagged `MG_<TYPE>`.
fn copy(game: &GameData, from: &str, name: &str, t: ResType) -> Gff {
    let data = game.resman.get(&ResKey::new(resref(from), t)).unwrap();
    let mut g = Gff::read(&data).unwrap();
    let field = if t == ResType::UTM { "ResRef" } else { "TemplateResRef" };
    g.root.set(field, Value::resref(resref(name)));
    let tag = format!("MG_{}", t.extension().unwrap_or_default().to_uppercase());
    g.root.set("Tag", Value::String(tag.into_bytes()));
    g
}

#[test]
fn every_blueprint_type_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_blueprints");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::new();
    let mut m = new_module(&game, "Moonglow Blueprints", &mut rng).unwrap();
    let spec =
        AreaSpec { name: "Blueprints".into(), tileset: resref("ttr01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();

    // An item with Enhancement +3 and the cost Moonglow derives for it.
    let mut uti = copy(&game, "nw_wswls001", "mg_uti", ResType::UTI);
    let enhancement = game.property_type(6).unwrap();
    let mut p = game.new_property(&enhancement, 1);
    p.cost_value = 3;
    let mut ps = Struct::new(0);
    for (label, v) in [
        ("PropertyName", Value::Word(p.property)),
        ("Subtype", Value::Word(p.subtype)),
        ("CostTable", Value::Byte(p.cost_table)),
        ("CostValue", Value::Word(p.cost_value)),
        ("Param1", Value::Byte(p.param1)),
        ("Param1Value", Value::Byte(p.param1_value)),
        ("ChanceAppear", Value::Byte(p.chance)),
    ] {
        ps.set(label, v);
    }
    uti.root.set("PropertiesList", Value::List(vec![ps]));
    let cost = game.item_cost(&ItemValue::from_gff(&uti.root));
    uti.root.set("Cost", Value::Dword(cost));

    // A stronger bandit with two more levels, and its hit points.
    let mut utc = copy(&game, "nw_bandit001", "mg_utc", ResType::UTC);
    utc.root.set("Str", Value::Byte(18));
    let mut classes = utc.root.list("ClassList").unwrap().to_vec();
    classes[0].set("ClassLevel", Value::Short(3));
    utc.root.set("ClassList", Value::List(classes));
    let stats = game.creature_stats(&CreatureSheet::from_gff(&utc.root));
    utc.root.set("MaxHitPoints", Value::Short(stats.max_hit_points as i16));

    // A chest holding the item, a store selling it with 1234 gold.
    let mut utp = copy(&game, "plc_chest1", "mg_utp", ResType::UTP);
    let mut held = Struct::new(0);
    held.set("InventoryRes", Value::resref(resref("mg_uti")));
    held.set("Repos_PosX", Value::Word(0));
    held.set("Repos_Posy", Value::Word(0));
    utp.root.set("HasInventory", Value::Byte(1));
    utp.root.set("ItemList", Value::List(vec![held.clone()]));
    let mut utm = copy(&game, "nw_storebar01", "mg_utm", ResType::UTM);
    utm.root.set("StoreGold", Value::Int(1234));
    let mut weapons = Struct::new(4);
    weapons.set("ItemList", Value::List(vec![held]));
    utm.root.set("StoreList", Value::List(vec![weapons]));

    let utw = copy(&game, "nw_waypoint001", "mg_utw", ResType::UTW);
    let mut utd = copy(&game, "nw_door_ttr_01", "mg_utd", ResType::UTD);
    utd.root.set("Locked", Value::Byte(1));
    let utt = copy(&game, "trackstrigger", "mg_utt", ResType::UTT);
    let mut ute = copy(&game, "nw_verminbeet", "mg_ute", ResType::UTE);
    ute.root.set("Active", Value::Byte(1));
    let uts = copy(&game, "animalcriesday", "mg_uts", ResType::UTS);

    // The door, trigger, encounter and sound stand in the area.
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let square = [[-2.0, -2.0], [2.0, -2.0], [2.0, 2.0], [-2.0, 2.0]];
    for (t, g, x) in [
        (ResType::UTD, &utd, 10.0),
        (ResType::UTT, &utt, 20.0),
        (ResType::UTE, &ute, 30.0),
        (ResType::UTS, &uts, 15.0),
    ] {
        let at = Placement { position: [x, 30.0, 0.0], facing: 0.0 };
        let placed = instance(t, &g.root, at, &square).unwrap();
        let (list, _) = git_list(t).unwrap();
        let mut items = git.root.list(list).unwrap_or(&[]).to_vec();
        items.push(placed);
        git.root.set(list, Value::List(items));
    }
    m.set_gff(git_key, &git).unwrap();

    for (name, t, g) in [
        ("mg_utc", ResType::UTC, &utc),
        ("mg_uti", ResType::UTI, &uti),
        ("mg_utp", ResType::UTP, &utp),
        ("mg_utm", ResType::UTM, &utm),
        ("mg_utw", ResType::UTW, &utw),
        ("mg_utd", ResType::UTD, &utd),
        ("mg_utt", ResType::UTT, &utt),
        ("mg_ute", ResType::UTE, &ute),
        ("mg_uts", ResType::UTS, &uts),
    ] {
        m.set_gff(ResKey::new(resref(name), t), g).unwrap();
    }

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, resref("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(resref("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_bp.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_bp", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(
        run.finished,
        "the server did not finish:\n{}",
        &run.log[run.log.len().saturating_sub(3000)..]
    );

    let got: Vec<String> = run.values("MG_OBJ");
    let waypoint_name =
        game.locstring(&utw.root.locstring("LocalizedName").cloned().unwrap_or_default());
    let expected = [
        format!("MG_UTC|1|1|{}|{}|3", 18 + stats.racial[0], stats.max_hit_points),
        format!("MG_UTI|1|2|{cost}|1"),
        "MG_UTP|1|64|MG_UTI|1".to_string(),
        "MG_UTM|1|128|1234|MG_UTI".to_string(),
        format!("MG_UTW|1|32|{}", waypoint_name.unwrap_or_default()),
        "MG_UTD|1|8|1|mg_utd".to_string(),
        "MG_UTT|1|4|mg_utt".to_string(),
        "MG_UTE|1|256|1|mg_ute".to_string(),
    ];
    for e in &expected {
        assert!(got.contains(e), "expected {e:?}; the engine reported:\n{}", got.join("\n"));
    }
    // The sound object is found by its tag.
    assert!(got.iter().any(|g| g.starts_with("MG_UTS|1|")), "{}", got.join("\n"));
}
