//! A placeable's illumination (`LightState`, a byte Aurora neither shows
//! nor writes; Jasperre's report), settled in the engine: where the server
//! reads it (a placed placeable's GIT entry, a blueprint created by
//! script), what it is without the field (on, for a placeable whose
//! appearance has a light), and what the server writes for it. A
//! placeable whose appearance has no light is never lit.

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

// The illumination the engine reports, and the field it would save.
void Lit(string sTag, object o)
{
    Log("MG_LIT " + sTag + "|" + IntToString(GetIsObjectValid(o)) + "|"
        + IntToString(GetPlaceableIllumination(o)) + "|"
        + JsonDump(JsonObjectGet(ObjectToJson(o), "LightState")));
}

void main()
{
    string sTags = "MG_OFF MG_ON MG_TWO MG_ABSENT MG_CHEST_OFF MG_CHEST_ON MG_CHEST";
    int n = 0;
    while (n < GetStringLength(sTags))
    {
        int nEnd = FindSubString(sTags, " ", n);
        if (nEnd < 0) nEnd = GetStringLength(sTags);
        string sTag = GetSubString(sTags, n, nEnd - n);
        Lit(sTag, GetObjectByTag(sTag));
        n = nEnd + 1;
    }
    // Blueprints created by script, with and without the field.
    object oArea = GetArea(GetObjectByTag("MG_ON"));
    Lit("MG_MADE_OFF", CreateObject(OBJECT_TYPE_PLACEABLE, "mg_braz_off", Location(oArea, Vector(5.0, 5.0, 0.0), 0.0)));
    Lit("MG_MADE", CreateObject(OBJECT_TYPE_PLACEABLE, "mg_braz", Location(oArea, Vector(7.0, 5.0, 0.0), 0.0)));
    // Set by script: what the engine then holds.
    object oAbsent = GetObjectByTag("MG_ABSENT");
    SetPlaceableIllumination(oAbsent, FALSE);
    Lit("MG_SET_OFF", oAbsent);
    object oOff = GetObjectByTag("MG_OFF");
    SetPlaceableIllumination(oOff, TRUE);
    Lit("MG_SET_ON", oOff);
    object oChest = GetObjectByTag("MG_CHEST");
    SetPlaceableIllumination(oChest, TRUE);
    Lit("MG_SET_CHEST", oChest);
    Log("MG_DONE");
}
"#;

#[test]
fn the_engine_reads_a_placeable_s_light_state() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_light_state");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Light State", &mut rng).unwrap();
    let spec = AreaSpec { name: "Lights".into(), tileset: r("tic01"), width: 4, height: 4 };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();

    let blueprint = |name: &str| -> Struct {
        Gff::read(&game.resman.get(&ResKey::new(r(name), ResType::UTP)).unwrap()).unwrap().root
    };
    // A brazier (placeables.2da row 57 has a light) and a chest (no light),
    // not static, so that they are objects in the game.
    let brazier = || {
        let mut s = blueprint("plc_brazier");
        assert_eq!(s.integer("Appearance"), Some(57));
        assert!(s.get("LightState").is_none(), "the game's brazier has no LightState");
        s.set("Static", Value::Byte(0));
        s
    };
    let chest = || {
        let mut s = blueprint("plc_chest1");
        s.set("Static", Value::Byte(0));
        s
    };
    let items = |_: ResRef| -> Option<Struct> { None };
    let placing = Placing { game: &game, item: &items };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let (list, _) = git_list(ResType::UTP).unwrap();
    let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
    for (i, (mut s, tag, state)) in [
        (brazier(), "MG_OFF", Some(0)),
        (brazier(), "MG_ON", Some(1)),
        (brazier(), "MG_TWO", Some(2)),
        (brazier(), "MG_ABSENT", None),
        (chest(), "MG_CHEST_OFF", Some(0)),
        (chest(), "MG_CHEST_ON", Some(1)),
        (chest(), "MG_CHEST", None),
    ]
    .into_iter()
    .enumerate()
    {
        s.set("Tag", Value::String(tag.as_bytes().to_vec()));
        if let Some(v) = state {
            s.set("LightState", Value::Byte(v));
        }
        let at = Placement { position: [3.0 + 3.0 * i as f32, 20.0, 0.0], rotation: 0.0 };
        let placed = instance(&placing, ResType::UTP, &s, at, &[]).unwrap();
        // (Placing keeps a field Aurora doesn't know.)
        assert_eq!(placed.integer("LightState"), state.map(i64::from), "{tag}");
        objects.push(placed);
    }
    git.root.set(list, Value::List(objects));
    m.set_gff(git_key, &git).unwrap();

    for (name, state) in [("mg_braz_off", Some(0)), ("mg_braz", None)] {
        let mut s = brazier();
        s.set("TemplateResRef", Value::resref(r(name)));
        if let Some(v) = state {
            s.set("LightState", Value::Byte(v));
        }
        let mut g = Gff::new(*b"UTP ");
        g.root = s;
        m.set_gff(ResKey::new(r(name), ResType::UTP), &g).unwrap();
    }

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, r("mg_probe"));
    m.set_info(&info).unwrap();
    m.set(ResKey::new(r("mg_probe"), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_light.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_light", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish:\n{}", run.log);

    let got = run.values("MG_LIT");
    let byte = |v: u8| format!("{{\"type\":\"byte\",\"value\":{v}}}");
    let want = [
        // Read from the placed object: 0 is off, anything else on; without
        // the field, on. The engine keeps the field as a byte.
        format!("MG_OFF|1|0|{}", byte(0)),
        format!("MG_ON|1|1|{}", byte(1)),
        format!("MG_TWO|1|1|{}", byte(1)),
        format!("MG_ABSENT|1|1|{}", byte(1)),
        // A placeable whose appearance has no light is never lit, whatever
        // the field says.
        format!("MG_CHEST_OFF|1|0|{}", byte(0)),
        format!("MG_CHEST_ON|1|0|{}", byte(0)),
        format!("MG_CHEST|1|0|{}", byte(0)),
        // Read from a blueprint created by script.
        format!("MG_MADE_OFF|1|0|{}", byte(0)),
        format!("MG_MADE|1|1|{}", byte(1)),
        // Set by script (and, on the chest, nothing happens).
        format!("MG_SET_OFF|1|0|{}", byte(0)),
        format!("MG_SET_ON|1|1|{}", byte(1)),
        format!("MG_SET_CHEST|1|0|{}", byte(0)),
    ];
    assert_eq!(got, want);
}
