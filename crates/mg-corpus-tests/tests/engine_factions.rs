//! Factions made with Moonglow's Faction Editor operations behave in the
//! engine as written: creatures of each faction regard each other with the
//! reputations in `repute.fac` (`GetReputation`).

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_module::ModuleLocation;
use mg_module::factions::Factions;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo, utc};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
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
