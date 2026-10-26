//! An armor whose torso has no armor class of its own (parts_chest.2da row
//! 0, and the other rows with a blank `ACBONUS`), settled in the engine:
//! the armor class, weight and value it gives such an armor, against a
//! torso whose `ACBONUS` is 0.

use std::time::Duration;

use mg_core::ResType;
use mg_gff::{Gff, Value};
use mg_resman::{GameInstall, ResKey};
use mg_rules::{GameData, ItemValue};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::probe_module;

/// The torsos tried: row 0 and row 2 (blank `ACBONUS`), row 1 (0.00), row
/// 4 (4.00), and a row past the end of the table.
const TORSOS: [u8; 5] = [0, 1, 2, 4, 250];

#[test]
fn an_armor_whose_torso_has_no_armor_class_is_clothing() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let chest = gd.table("parts_chest").unwrap();
    assert_eq!(chest.get(0, "ACBONUS"), None, "the game's torso 0 has a blank ACBONUS");
    assert_eq!(chest.get(2, "ACBONUS"), None, "the game's torso 2 has a blank ACBONUS");
    assert!(chest.len() < 250);

    // A plain armor (no properties), once per torso.
    let data = gd.resman.get(&ResKey::parse("nw_aarcl001", ResType::UTI).unwrap()).unwrap();
    let mut items = Vec::new();
    let mut script = String::from(
        "void Probe(string r)\n{\n\
         \x20   object o = CreateObject(OBJECT_TYPE_ITEM, r, GetStartingLocation());\n\
         \x20   SetIdentified(o, FALSE);\n\
         \x20   SetIdentified(o, TRUE);\n\
         \x20   WriteTimestampedLogEntry(\"MG_TORSO \" + r + \" \" + IntToString(GetIsObjectValid(o))\n\
         \x20       + \" \" + IntToString(GetItemAppearance(o, ITEM_APPR_TYPE_ARMOR_MODEL, ITEM_APPR_ARMOR_MODEL_TORSO))\n\
         \x20       + \" \" + IntToString(GetItemACValue(o)) + \" \" + IntToString(GetWeight(o))\n\
         \x20       + \" \" + IntToString(GetGoldPieceValue(o))\n\
         \x20       + \" \" + IntToString(JsonGetInt(JsonObjectGet(JsonObjectGet(ObjectToJson(o), \"Cost\"), \"value\"))));\n}\n\
         void main()\n{\n",
    );
    for torso in TORSOS {
        let name = format!("mg_torso_{torso}");
        let mut uti = Gff::read(&data).unwrap();
        uti.root.set("TemplateResRef", Value::ResRef(name.as_bytes().to_vec()));
        uti.root.set("ArmorPart_Torso", Value::Byte(torso));
        uti.root.set("PropertiesList", Value::List(Vec::new()));
        uti.root.set("Plot", Value::Byte(0));
        script += &format!("    Probe(\"{name}\");\n");
        items.push((name, uti));
    }
    script += "    WriteTimestampedLogEntry(\"MG_DONE\");\n}\n";
    let extra: Vec<_> = items
        .iter()
        .map(|(name, g)| (ResKey::parse(name, ResType::UTI).unwrap(), g.to_bytes().unwrap()))
        .collect();

    let dir = scratch_dir("engine_armor_torso");
    probe_module(&root, &dir, "mg_torso_probe", &script, &[], &extra);
    let run =
        run_server(&root, &dir, "mg_torso_probe", "MG_DONE", Duration::from_secs(60)).unwrap();
    assert!(
        run.finished,
        "server did not finish; log tail:\n{}",
        &run.log[run.log.len().saturating_sub(3000)..]
    );
    // Measured (EE 89.8193.37): every torso without an ACBONUS (row 0, row
    // 2, a row past the table) is armor class 0, 1.0 lb and 1 gp (its
    // `Cost` 1), as row 1 (0.00) is: armor.2da row 0, clothing.
    let engine = run.values("MG_TORSO");

    // Moonglow's: the armor class, its armor.2da weight (tenths of a
    // pound, as `GetWeight`), and the value (before the engine's
    // at-least-1, which would hide a 0), shown and kept in `Cost`.
    let armor = gd.table("armor").unwrap();
    let ours: Vec<String> = items
        .iter()
        .zip(TORSOS)
        .map(|((name, g), torso)| {
            let ac = gd.armor_class(u32::from(torso));
            let weight = armor.get_int(ac as usize, "WEIGHT").unwrap_or(-1);
            let cost = gd.item_cost(&ItemValue::from_gff(&g.root));
            format!("{name} 1 {torso} {ac} {weight} {cost} {cost}")
        })
        .collect();
    assert_eq!(ours, engine);
}
