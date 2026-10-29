//! Factions made with Moonglow's Faction Editor operations behave in the
//! engine as written: creatures of each faction regard each other with the
//! reputations in `repute.fac` (`GetReputation`). And what the engine does
//! with an object of faction 0, the PC faction.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::ModuleLocation;
use mg_module::factions::Factions;
use mg_module::instances::{Placement, Placing, git_list, instance};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo, utc};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::compile;

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

void main()
{
    location l = GetStartingLocation();
    int n = 6;
    int i; int j;
    for (i = 1; i <= n; i++)
        SetLocalObject(GetModule(), "c" + IntToString(i), CreateObject(OBJECT_TYPE_CREATURE, "mg_fac_" + IntToString(i), l));
    for (i = 1; i <= n; i++)
        for (j = 1; j <= n; j++)
        {
            object a = GetLocalObject(GetModule(), "c" + IntToString(i));
            object b = GetLocalObject(GetModule(), "c" + IntToString(j));
            Log("MG_REP " + IntToString(i) + "|" + IntToString(j) + "|" + IntToString(GetReputation(a, b))
                + "|" + IntToString(GetFactionEqual(a, b)));
        }
    Log("MG_DONE");
    for (i = 0; i < 2000; i++)
        Log("MG_PAD ................................................................");
}
"#;

#[test]
fn faction_reputations_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_factions");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(3);
    let mut m = new_module(&game, "Factions", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Arena".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 4,
        height: 4,
    };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();

    // Two custom factions, one reputation changed, one faction removed.
    let mut f = Factions::of_module(&m);
    let spare = f.add("Spare", true, 3);
    let guards = f.add("Guards", true, 4);
    let bandits = f.add("Bandits", false, 1);
    f.set_reputation(guards, bandits, 7);
    f.set_reputation(bandits, 2, 42);
    let map = f.remove(spare).unwrap();
    let (guards, bandits) = (map(guards).unwrap(), map(bandits).unwrap());
    assert_eq!((guards, bandits), (5, 6));
    m.set_gff(ResKey::parse("repute", ResType::FAC).unwrap(), &f.to_gff()).unwrap();

    // A creature blueprint per faction 1..=6.
    let commoner = Gff::read(&game.resman.get_named("nw_commale", ResType::UTC).unwrap()).unwrap();
    for id in 1..=6u16 {
        let mut c = commoner.clone();
        let name = format!("mg_fac_{id}");
        c.root.write(&utc::FACTION_ID, id);
        c.root.write(&utc::TEMPLATE_RES_REF, ResRef::from_str(&name).unwrap());
        m.set_gff(ResKey::parse(&name, ResType::UTC).unwrap(), &c).unwrap();
    }
    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_factions.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_factions", "MG_DONE", Duration::from_secs(300)).unwrap();
    assert!(run.finished, "the server did not finish");

    let got = run.values("MG_REP");
    assert_eq!(got.len(), 36, "{got:?}");
    let mut differences = Vec::new();
    for line in got {
        let v: Vec<u32> = line.split('|').map(|x| x.parse().unwrap()).collect();
        let (a, b, rep, equal) = (v[0], v[1], v[2], v[3]);
        let want = f.reputation(a, b).unwrap_or(mg_module::factions::DEFAULT_REPUTATION);
        if rep != want || (equal == 1) != (a == b) {
            differences.push(format!("{a} regards {b}: engine {rep} (equal {equal}), file {want}"));
        }
    }
    assert!(differences.is_empty(), "{differences:#?}");
}

const PC_FACTION_PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
string I(int n) { return IntToString(n); }
string Field(object o, string sField) { return JsonDump(JsonObjectGet(ObjectToJson(o), sField)); }

void main()
{
    location l = GetStartingLocation();
    object a = CreateObject(OBJECT_TYPE_CREATURE, "mg_fac0", l);
    object b = CreateObject(OBJECT_TYPE_CREATURE, "mg_fac0", l);
    object h = CreateObject(OBJECT_TYPE_CREATURE, "mg_fac1", l);
    object c = CreateObject(OBJECT_TYPE_CREATURE, "mg_fac2", l);
    // Whether two creatures of faction 0 are of one faction, and how they
    // and the standard factions regard each other.
    Log("MG_PC0 " + I(GetIsObjectValid(a)) + "|" + I(GetFactionEqual(a, b)) + "|" + I(GetFactionEqual(a, h))
        + "|" + I(GetFactionEqual(a, c)) + "|" + I(GetReputation(a, c)) + "|" + I(GetReputation(c, a))
        + "|" + I(GetIsEnemy(c, a)) + "|" + Field(a, "FactionID"));
    // Created from blueprints with no faction field: a creature and a
    // placeable.
    object bx = CreateObject(OBJECT_TYPE_CREATURE, "mg_facx", l);
    object px = CreateObject(OBJECT_TYPE_PLACEABLE, "mg_plcx", l);
    Log("MG_BPX " + I(GetIsObjectValid(bx)) + "|" + I(GetFactionEqual(bx, h)) + "|" + Field(bx, "FactionID")
        + "|" + I(GetIsObjectValid(px)) + "|" + I(GetReputation(c, px)) + "|" + Field(px, "Faction"));
    // Placed objects of faction 0, of faction 2 to compare, and with no
    // faction field.
    string sTags = "MG_CRE0 MG_CRE2 MG_CREX MG_PLC0 MG_PLC2 MG_PLCX MG_DOOR0 MG_DOOR2 MG_DOORX MG_TRIG0 MG_TRIG2 MG_TRIGX";
    int n = 0;
    while (n < GetStringLength(sTags))
    {
        int nEnd = FindSubString(sTags, " ", n);
        if (nEnd < 0) nEnd = GetStringLength(sTags);
        string sTag = GetSubString(sTags, n, nEnd - n);
        object o = GetObjectByTag(sTag);
        string sField = GetObjectType(o) == OBJECT_TYPE_CREATURE ? "FactionID" : "Faction";
        Log("MG_PLACED " + sTag + "|" + I(GetIsObjectValid(o)) + "|" + I(GetFactionEqual(o, h))
            + "|" + I(GetFactionEqual(o, c)) + "|" + I(GetReputation(c, o)) + "|" + I(GetReputation(h, o))
            + "|" + Field(o, sField));
        n = nEnd + 1;
    }
    Log("MG_DONE");
    int i;
    for (i = 0; i < 2000; i++)
        Log("MG_PAD ................................................................");
}
"#;

/// Objects of faction 0 ("PC", which Moonglow does not offer objects;
/// Jasperre's report): the game puts a creature of it in faction 1,
/// Hostile; placeables, doors and triggers keep it, and creatures regard
/// them as they regard the PC faction. An object with no faction field is
/// read as one of faction 0, placed or created from its blueprint. (No
/// player can join a headless server, so how one regards a player is not
/// measured, nor an encounter's creatures, which a player's entering
/// spawns.)
#[test]
fn objects_of_the_pc_faction_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_pc_faction");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(4);
    let mut m = new_module(&game, "PC Faction", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Arena".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 4,
        height: 4,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let blueprint = |name: &str, t: ResType| -> Struct {
        Gff::read(&game.resman.get_named(name, t).unwrap()).unwrap().root
    };
    let commoner = blueprint("nw_commale", ResType::UTC);
    for id in 0..=2u16 {
        let mut c = Gff::new(*b"UTC ");
        c.root = commoner.clone();
        let name = format!("mg_fac{id}");
        c.root.write(&utc::FACTION_ID, id);
        c.root.write(&utc::TEMPLATE_RES_REF, ResRef::from_str(&name).unwrap());
        m.set_gff(ResKey::parse(&name, ResType::UTC).unwrap(), &c).unwrap();
    }

    // Blueprints with no faction field.
    let mut c = Gff::new(*b"UTC ");
    c.root = commoner.clone();
    c.root.remove("FactionID");
    c.root.write(&utc::TEMPLATE_RES_REF, ResRef::from_str("mg_facx").unwrap());
    m.set_gff(ResKey::parse("mg_facx", ResType::UTC).unwrap(), &c).unwrap();
    let mut p = Gff::new(*b"UTP ");
    p.root = blueprint("plc_chest1", ResType::UTP);
    p.root.remove("Faction");
    p.root.set("TemplateResRef", Value::ResRef(b"mg_plcx".to_vec()));
    m.set_gff(ResKey::parse("mg_plcx", ResType::UTP).unwrap(), &p).unwrap();

    // One placed object of each kind with a faction, of faction 0 and of
    // faction 2.
    let first = |t: ResType| blueprint(&game.resman.list(t)[0].to_string(), t);
    let items = |_: ResRef| -> Option<Struct> { None };
    let placing = Placing { game: &game, item: &items };
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let mut chest = blueprint("plc_chest1", ResType::UTP);
    chest.set("Static", Value::Byte(0));
    let kinds = [
        (ResType::UTC, commoner.clone(), "MG_CRE", "FactionID"),
        (ResType::UTP, chest, "MG_PLC", "Faction"),
        (ResType::UTD, first(ResType::UTD), "MG_DOOR", "Faction"),
        (ResType::UTT, first(ResType::UTT), "MG_TRIG", "Faction"),
    ];
    for (i, (t, s, tag, field)) in kinds.into_iter().enumerate() {
        // Of faction 0, of faction 2, and with no faction field ("X").
        for faction in [Some(0u16), Some(2), None] {
            let mut s = s.clone();
            let name = faction.map_or("X".to_string(), |f| f.to_string());
            s.set("Tag", Value::String(format!("{tag}{name}").into_bytes()));
            let v = |faction: u16| {
                if t == ResType::UTC {
                    Value::Word(faction)
                } else {
                    Value::Dword(u32::from(faction))
                }
            };
            match faction {
                Some(f) => s.set(field, v(f)),
                None => {
                    s.remove(field);
                }
            }
            let x = 4.0 + 4.0 * i as f32;
            let y = 20.0 + 4.0 * f32::from(faction.unwrap_or(4));
            let at = Placement { position: [x, y, 0.0], rotation: 0.0 };
            let outline: Vec<[f32; 3]> = if t == ResType::UTT {
                vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]]
            } else {
                vec![]
            };
            let mut placed = instance(&placing, t, &s, at, &outline).unwrap();
            if faction.is_none() {
                placed.remove(field);
            }
            let (list, _) = git_list(t).unwrap();
            let mut objects = git.root.list(list).unwrap_or(&[]).to_vec();
            objects.push(placed);
            git.root.set(list, Value::List(objects));
        }
    }
    m.set_gff(git_key, &git).unwrap();

    let probe = compile(&dir, "mg_probe", PC_FACTION_PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_pcfac.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_pcfac", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "the server did not finish");
    // A creature of faction 0, created or placed, is put in Hostile: it is
    // of one faction with a hostile creature, regarded as one, and saved
    // with faction 1.
    let word = |v: u16| format!("{{\"type\":\"word\",\"value\":{v}}}");
    assert_eq!(run.values("MG_PC0"), [format!("1|1|1|0|0|0|1|{}", word(1))]);
    // Placeables, doors and triggers keep faction 0, and the creatures
    // regard them as repute.fac has them regard the PC faction.
    let f = Factions::of_module(&m);
    let rep = |by: u32, of: u32| f.reputation(by, of).unwrap();
    let (commoner_pc, hostile_pc) = (rep(2, 0), rep(1, 0));
    let dword = |v: u32| format!("{{\"type\":\"dword\",\"value\":{v}}}");
    // An object with no faction field is of faction 0 (a creature, put in
    // Hostile as one of 0 is), placed or made from a blueprint so.
    let mut want = vec![
        format!("MG_CRE0|1|1|0|0|100|{}", word(1)),
        format!("MG_CRE2|1|0|1|100|0|{}", word(2)),
        format!("MG_CREX|1|1|0|0|100|{}", word(1)),
    ];
    for tag in ["MG_PLC", "MG_DOOR", "MG_TRIG"] {
        want.push(format!("{tag}0|1|0|0|{commoner_pc}|{hostile_pc}|{}", dword(0)));
        want.push(format!("{tag}2|1|0|1|100|0|{}", dword(2)));
        want.push(format!("{tag}X|1|0|0|{commoner_pc}|{hostile_pc}|{}", dword(0)));
    }
    assert_eq!(run.values("MG_PLACED"), want);
    assert_eq!(run.values("MG_BPX"), [format!("1|1|{}|1|{commoner_pc}|{}", word(1), dword(0))]);
}
