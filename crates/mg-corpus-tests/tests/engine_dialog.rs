//! A conversation made with Moonglow's Conversation Editor operations runs
//! in the engine: an NPC speaking it as a one-liner checks the greetings'
//! conditions in order, each receiving its parameters (EE), and runs the
//! chosen greeting's action with its parameters.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::dialog::{Parent, add_node, new_dialog, params_value};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo, utc};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::compile;

const LOG: &str = "void Log(string s) { WriteTimestampedLogEntry(s); }\n";

fn condition(name: &str, result: bool) -> String {
    format!(
        "{LOG}int StartingConditional()\n{{\n    Log(\"MG_COND {name}|\" + GetScriptParam(\"n\") + \"|\" + GetScriptParam(\"x\"));\n    return {};\n}}\n",
        if result { "TRUE" } else { "FALSE" }
    )
}

const ACTION: &str = "void Log(string s) { WriteTimestampedLogEntry(s); }\nvoid main() { Log(\"MG_ACT \" + GetScriptParam(\"who\")); }\n";

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
void Finish()
{
    Log("MG_DONE");
    int i;
    for (i = 0; i < 2000; i++)
        Log("MG_PAD ................................................................");
}
void main()
{
    object o = CreateObject(OBJECT_TYPE_CREATURE, "mg_npc", GetStartingLocation());
    Log("MG_NPC " + IntToString(GetIsObjectValid(o)));
    AssignCommand(o, SpeakOneLinerConversation("mg_dlg"));
    DelayCommand(2.0, Finish());
}
"#;

#[test]
fn conversation_runs_in_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_dialog");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(9);
    let mut m = new_module(&game, "Dialog", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Room".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 2,
        height: 2,
    };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();

    // Three greetings: the first's condition fails, the second's passes.
    let mut g = new_dialog();
    for (text, cond, params, action) in [
        ("First", "mg_c1", vec![("n", "1")], None),
        ("Second", "mg_c2", vec![("n", "2"), ("x", "b")], Some("second")),
        ("Third", "mg_c3", vec![("n", "3")], Some("third")),
    ] {
        let i = add_node(&mut g, Parent::Root, text);
        let start = g.root.list_mut("StartingList").unwrap().last_mut().unwrap();
        start.set("Active", Value::resref(ResRef::from_str(cond).unwrap()));
        let pairs: Vec<(String, String)> =
            params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        start.set("ConditionParams", params_value(&pairs));
        if let Some(who) = action {
            let n = &mut g.root.list_mut("EntryList").unwrap()[i as usize];
            n.set("Script", Value::resref(ResRef::from_str("mg_act").unwrap()));
            n.set("ActionParams", params_value(&[("who".into(), who.into())]));
        }
    }
    m.set_gff(ResKey::parse("mg_dlg", ResType::DLG).unwrap(), &g).unwrap();
    for (name, src) in [
        ("mg_c1", condition("mg_c1", false)),
        ("mg_c2", condition("mg_c2", true)),
        ("mg_c3", condition("mg_c3", true)),
        ("mg_act", ACTION.to_string()),
    ] {
        let ncs = compile(&dir, name, &src);
        m.set(ResKey::parse(name, ResType::NCS).unwrap(), ncs);
    }
    let npc = Gff::read(&game.resman.get_named("nw_commale", ResType::UTC).unwrap()).unwrap();
    let mut npc = npc.clone();
    npc.root.write(&utc::TEMPLATE_RES_REF, ResRef::from_str("mg_npc").unwrap());
    npc.root.write(&utc::CONVERSATION, ResRef::from_str("mg_dlg").unwrap());
    m.set_gff(ResKey::parse("mg_npc", ResType::UTC).unwrap(), &npc).unwrap();

    let probe = compile(&dir, "mg_probe", PROBE);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::new(ResRef::from_str("mg_probe").unwrap(), ResType::NCS), probe);
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join("modules/mg_dialog.mod"))).unwrap();
    let run = run_server(&root, &user, "mg_dialog", "MG_DONE", Duration::from_secs(300)).unwrap();
    assert!(run.finished, "the server did not finish");

    assert_eq!(run.values("MG_NPC"), ["1"]);
    assert_eq!(
        run.values("MG_COND"),
        ["mg_c1|1|", "mg_c2|2|b"],
        "conditions in order, with their parameters"
    );
    assert_eq!(
        run.values("MG_ACT"),
        ["second"],
        "the chosen greeting's action, with its parameter"
    );
}
