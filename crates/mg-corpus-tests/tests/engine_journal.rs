//! A journal made with the Journal Editor's operations is read by the
//! engine: each category's XP is what `GetJournalQuestExperience` returns for
//! its tag. (Entries show only to players, which a headless server has none
//! of; their structure is checked against Aurora's in `aurora_journal.rs`.)

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::Value;
use mg_module::ModuleLocation;
use mg_module::journal::{new_category, new_entry, new_journal};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{ExoString, StructExt, ifo, jrl};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::compile;

const QUESTS: [(&str, u32); 3] = [("q_rats", 150), ("Q_Mixed_Case", 1), ("q_final", 123456)];

#[test]
fn journal_xp_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_journal");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Journal", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Room".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 2,
        height: 2,
    };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();

    let mut g = new_journal();
    let mut cats = Vec::new();
    for (tag, xp) in QUESTS {
        let mut c = new_category(&cats);
        c.write(&jrl::categories::TAG, ExoString::from(tag));
        c.write(&jrl::categories::XP, xp);
        let list = vec![new_entry(&[])];
        c.set(jrl::categories::ENTRY_LIST.label, Value::List(list));
        cats.push(c);
    }
    g.root.set(jrl::CATEGORIES.label, Value::List(cats));
    m.set_gff(ResKey::parse("module", ResType::JRL).unwrap(), &g).unwrap();

    let mut script =
        String::from("void Log(string s) { WriteTimestampedLogEntry(s); }\nvoid main()\n{\n");
    for (tag, _) in QUESTS.iter().map(|q| (q.0, ())).chain([("q_missing", ())]) {
        script += &format!(
            "    Log(\"MG_XP {tag}|\" + IntToString(GetJournalQuestExperience(\"{tag}\")));\n"
        );
    }
    script += "    Log(\"MG_DONE\");\n    int i;\n    for (i = 0; i < 2000; i++) Log(\"MG_PAD ................................................................\");\n}\n";
    let probe = compile(&dir, "mg_probe", &script);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_journal.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_journal", "MG_DONE", Duration::from_secs(300)).unwrap();
    assert!(run.finished, "the server did not finish");

    let mut want: Vec<String> = QUESTS.iter().map(|(t, xp)| format!("{t}|{xp}")).collect();
    want.push("q_missing|0".into());
    assert_eq!(run.values("MG_XP"), want);
}
