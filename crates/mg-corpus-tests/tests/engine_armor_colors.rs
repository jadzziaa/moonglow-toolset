//! An armor's per-part colors (EE: `APart_<part>_Col_<channel>`, which
//! Aurora neither shows nor keeps), settled in the engine: it reads the
//! fields from a blueprint, and writes them, as bytes, for a color set by
//! script.

use std::time::Duration;

use mg_core::ResType;
use mg_gff::{Gff, Value};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_rules::items::{ArmorChannel, armor_part_color, set_armor_part_color};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::probe_module;

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
string Color(object o, int n) { return IntToString(GetItemAppearance(o, ITEM_APPR_TYPE_ARMOR_COLOR, n)); }
string Field(json j, string s) { return JsonDump(JsonObjectGet(j, s)); }

void main()
{
    object o = CreateObject(OBJECT_TYPE_ITEM, "mg_parts", GetStartingLocation());
    // The torso's cloth 1 and the left hand's metal 2 (set in the
    // blueprint), the item's cloth 1, and a part color that is not set.
    int nTorso = ITEM_APPR_ARMOR_NUM_COLORS + ITEM_APPR_ARMOR_MODEL_TORSO * ITEM_APPR_ARMOR_NUM_COLORS
        + ITEM_APPR_ARMOR_COLOR_CLOTH1;
    int nHand = ITEM_APPR_ARMOR_NUM_COLORS + ITEM_APPR_ARMOR_MODEL_LHAND * ITEM_APPR_ARMOR_NUM_COLORS
        + ITEM_APPR_ARMOR_COLOR_METAL2;
    int nFoot = ITEM_APPR_ARMOR_NUM_COLORS + ITEM_APPR_ARMOR_MODEL_RFOOT * ITEM_APPR_ARMOR_NUM_COLORS
        + ITEM_APPR_ARMOR_COLOR_LEATHER1;
    Log("MG_READ " + IntToString(GetIsObjectValid(o)) + " " + Color(o, nTorso) + " " + Color(o, nHand)
        + " " + Color(o, ITEM_APPR_ARMOR_COLOR_CLOTH1) + " " + Color(o, nFoot));
    // A color set by script: what the engine then holds.
    int nBicep = ITEM_APPR_ARMOR_NUM_COLORS + ITEM_APPR_ARMOR_MODEL_RBICEP * ITEM_APPR_ARMOR_NUM_COLORS
        + ITEM_APPR_ARMOR_COLOR_METAL1;
    object c = CopyItemAndModify(o, ITEM_APPR_TYPE_ARMOR_COLOR, nBicep, 44, TRUE);
    json j = ObjectToJson(c);
    Log("MG_WROTE " + IntToString(GetIsObjectValid(c)) + " " + Field(j, "APart_12_Col_4")
        + " " + Field(j, "APart_7_Col_2") + " " + Field(j, "APart_17_Col_5")
        + " " + Field(j, "APart_0_Col_0"));
    Log("MG_DONE");
}
"#;

#[test]
fn the_engine_reads_and_writes_an_armor_s_part_colors() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let data = gd.resman.get(&ResKey::parse("nw_aarcl001", ResType::UTI).unwrap()).unwrap();
    let mut uti = Gff::read(&data).unwrap();
    uti.root.set("TemplateResRef", Value::ResRef(b"mg_parts".to_vec()));
    uti.root.set("Cloth1Color", Value::Byte(5));
    // As Moonglow's item editor writes them.
    set_armor_part_color(&mut uti.root, 7, ArmorChannel::Cloth1, Some(33));
    set_armor_part_color(&mut uti.root, 17, ArmorChannel::Metal2, Some(171));
    assert_eq!(uti.root.get("APart_7_Col_2"), Some(&Value::Byte(33)));
    assert_eq!(armor_part_color(&uti.root, 17, ArmorChannel::Metal2), Some(171));
    let extra = [(ResKey::parse("mg_parts", ResType::UTI).unwrap(), uti.to_bytes().unwrap())];

    let dir = scratch_dir("engine_armor_colors");
    probe_module(&root, &dir, "mg_parts_probe", PROBE, &[], &extra);
    let run =
        run_server(&root, &dir, "mg_parts_probe", "MG_DONE", Duration::from_secs(60)).unwrap();
    assert!(
        run.finished,
        "server did not finish; log tail:\n{}",
        &run.log[run.log.len().saturating_sub(3000)..]
    );
    // Read from the blueprint; 255 where a part has no color of its own.
    assert_eq!(run.values("MG_READ"), ["1 33 171 5 255"]);
    // Written as bytes; a part color that is not set has no field.
    let byte = |v: u8| format!("{{\"type\":\"byte\",\"value\":{v}}}");
    assert_eq!(run.values("MG_WROTE"), [format!("1 {} {} {} null", byte(44), byte(33), byte(171))]);
}
