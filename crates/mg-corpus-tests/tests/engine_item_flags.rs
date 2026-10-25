//! A creature's items' `Dropable` and `Pickpocketable` (Aurora's
//! inventory checkboxes; a builder could not find them in Moonglow),
//! settled in the engine: the fields' names, that a byte 1 sets them and
//! a field left out (or 0) does not, in the backpack (`ItemList`) and
//! equipped (`Equip_ItemList`), for a placed creature (whole items in the
//! GIT) and for a blueprint created by script (`InventoryRes` entries).

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::compile;

fn r(s: &str) -> ResRef {
    ResRef::from_str(s).unwrap()
}

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

void Flags(string sWho, string sWhere, object oItem)
{
    Log("MG_ITEM " + sWho + "|" + sWhere + "|" + GetResRef(oItem) + "|"
        + IntToString(GetDroppableFlag(oItem)) + "|"
        + IntToString(GetPickpocketableFlag(oItem)));
}

void Items(string sWho, object o)
{
    object oItem = GetFirstItemInInventory(o);
    while (GetIsObjectValid(oItem))
    {
        Flags(sWho, "pack", oItem);
        oItem = GetNextItemInInventory(o);
    }
    int i;
    for (i = 0; i < NUM_INVENTORY_SLOTS; i++)
    {
        object oEquipped = GetItemInSlot(i, o);
        if (GetIsObjectValid(oEquipped)) Flags(sWho, "slot" + IntToString(i), oEquipped);
    }
}

void main()
{
    object oPlaced = GetObjectByTag("MG_BANDIT");
    Items("placed", oPlaced);
    object oArea = GetArea(oPlaced);
    Items("made", CreateObject(OBJECT_TYPE_CREATURE, "mg_bandit", Location(oArea, Vector(5.0, 5.0, 0.0), 0.0)));
    Log("MG_DONE");
}
"#;

/// The items and the flags given them: (resref, slot bit or 0 for the
/// backpack, Dropable, Pickpocketable), `None` leaving the field out.
const ITEMS: [(&str, u32, Option<u8>, Option<u8>); 6] = [
    ("nw_it_mpotion001", 0, Some(1), None),
    ("nw_it_torch001", 0, None, Some(1)),
    ("nw_it_gem001", 0, None, None),
    ("nw_it_mpotion002", 0, Some(0), Some(0)),
    ("nw_wswls001", 0x10, Some(1), Some(1)),
    ("nw_aarcl004", 0x2, None, None),
];

#[test]
fn the_engine_reads_a_creature_s_item_flags() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_item_flags");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(7);
    let mut m = new_module(&game, "Item Flags", &mut rng).unwrap();
    let spec = AreaSpec { name: "Flags".into(), tileset: r("tic01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();

    let read = |name: ResRef, t: ResType| -> Option<Struct> {
        Some(Gff::read(&game.resman.get(&ResKey::new(name, t)).ok()?).ok()?.root)
    };
    // The game's bandit, its items replaced by ours.
    let mut utc = read(r("nw_bandit001"), ResType::UTC).unwrap();
    let (mut pack, mut equipped) = (Vec::new(), Vec::new());
    for (resref, slot, dropable, pickpocketable) in ITEMS {
        let mut s = if slot == 0 {
            let mut s = Struct::new(pack.len() as u32);
            s.set("InventoryRes", Value::resref(r(resref)));
            s.set("Repos_PosX", Value::Word(2 * pack.len() as u16));
            s.set("Repos_Posy", Value::Word(0));
            s
        } else {
            let mut s = Struct::new(slot);
            s.set("EquippedRes", Value::resref(r(resref)));
            s
        };
        if let Some(v) = dropable {
            s.set("Dropable", Value::Byte(v));
        }
        if let Some(v) = pickpocketable {
            s.set("Pickpocketable", Value::Byte(v));
        }
        if slot == 0 { pack.push(s) } else { equipped.push(s) }
    }
    equipped.sort_by_key(|s| s.id);
    utc.set("ItemList", Value::List(pack));
    utc.set("Equip_ItemList", Value::List(equipped));
    utc.set("TemplateResRef", Value::resref(r("mg_bandit")));
    let mut g = Gff::new(*b"UTC ");
    g.root = utc.clone();
    m.set_gff(ResKey::new(r("mg_bandit"), ResType::UTC), &g).unwrap();

    // Placed: whole items, each keeping its flags.
    let items = |name: ResRef| read(name, ResType::UTI);
    let placing = Placing { game: &game, item: &items };
    let mut tagged = utc;
    tagged.set("Tag", Value::String(b"MG_BANDIT".to_vec()));
    let at = Placement { position: [20.0, 20.0, 0.0], rotation: 0.0 };
    let placed = instance(&placing, ResType::UTC, &tagged, at, &[]).unwrap();
    let held = placed.list("ItemList").unwrap();
    assert_eq!(held[0].get("Dropable"), Some(&Value::Byte(1)), "placing keeps the flags");
    assert!(held[0].integer("BaseItem").is_some(), "a whole item");
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTC).unwrap();
    let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
    objects.push(placed);
    git.root.set(list, Value::List(objects));
    m.set_gff(git_key, &git).unwrap();

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_flags.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_flags", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);

    let mut got = run.values("MG_ITEM");
    got.sort();
    // Set by a byte 1 alone: left out or 0, neither dropped nor stolen.
    let mut want = Vec::new();
    for who in ["made", "placed"] {
        want.extend([
            format!("{who}|pack|nw_it_gem001|0|0"),
            format!("{who}|pack|nw_it_mpotion001|1|0"),
            format!("{who}|pack|nw_it_mpotion002|0|0"),
            format!("{who}|pack|nw_it_torch001|0|1"),
            format!("{who}|slot1|nw_aarcl004|0|0"),
            format!("{who}|slot4|nw_wswls001|1|1"),
        ]);
    }
    assert_eq!(got, want, "{}", run.log);
}
