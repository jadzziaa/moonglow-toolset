//! A conversation's condition script (`int StartingConditional()`) that
//! includes a file with a `void main()` of its own, run in the game as
//! each compiler makes it.
//!
//! Aurora 8193.37 compiles such a script from the include's `main`: its
//! compiled script is the include's `main` compiled alone, byte for byte
//! (`aurora_scripts::codegen_compiles_as_aurora_builds_it`), so the
//! condition's own code never runs. Moonglow's compiler (neverwinter.nim
//! d20453e3, 2026-08, its issue 150) takes the entry point from the
//! script itself: the condition runs and returns what it returns. In the
//! game Aurora's runs the include's `main` and fails, always: a line it
//! guards is never shown.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Value};
use mg_module::ModuleLocation;
use mg_module::dialog::{Parent, add_node, new_dialog};
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo, utc};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::compile;

/// The include, with a `main` of its own (as builders give an include so
/// that Aurora's editor compiles it alone).
const LIB: &str = "void Log(string s) { WriteTimestampedLogEntry(s); }\n\
                   void main() { Log(\"MG_LIBMAIN\"); }\n";

/// A condition that passes, including it.
const COND: &str = "#include \"mg_lib\"\n\
                    int StartingConditional() { Log(\"MG_COND\"); return TRUE; }\n";

const ACTION: &str =
    "void main() { WriteTimestampedLogEntry(\"MG_ACT \" + GetScriptParam(\"who\")); }\n";

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
    AssignCommand(o, SpeakOneLinerConversation("mg_dlg"));
    DelayCommand(2.0, Finish());
}
"#;

/// The greetings `conds` (each its condition's compiled script, and the
/// name its action says), and a last one without a condition: which one
/// the game chose, and what it logged.
fn speak(
    root: &std::path::Path,
    name: &str,
    conds: &[(&str, Vec<u8>)],
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let dir = scratch_dir(name);
    let game = GameData::open(&GameInstall::new(root, None, "en")).unwrap();
    let mut rng = fastrand::Rng::with_seed(3);
    let mut m = new_module(&game, "Entry", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Room".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 2,
        height: 2,
    };
    add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let mut g = new_dialog();
    let action = |g: &mut Gff, i: u32, who: &str| {
        let n = &mut g.root.list_mut("EntryList").unwrap()[i as usize];
        n.set("Script", Value::resref(ResRef::from_str("mg_act").unwrap()));
        n.set("ActionParams", mg_module::dialog::params_value(&[("who".into(), who.into())]));
    };
    for (cond, ncs) in conds {
        let i = add_node(&mut g, Parent::Root, cond);
        let start = g.root.list_mut("StartingList").unwrap().last_mut().unwrap();
        start.set("Active", Value::resref(ResRef::from_str(cond).unwrap()));
        action(&mut g, i, cond);
        m.set(ResKey::parse(cond, ResType::NCS).unwrap(), ncs.clone());
    }
    let i = add_node(&mut g, Parent::Root, "Last");
    action(&mut g, i, "last");
    m.set_gff(ResKey::parse("mg_dlg", ResType::DLG).unwrap(), &g).unwrap();
    m.set(ResKey::parse("mg_act", ResType::NCS).unwrap(), compile(&dir, "mg_act", ACTION));
    let mut npc = Gff::read(&game.resman.get_named("nw_commale", ResType::UTC).unwrap()).unwrap();
    npc.root.write(&utc::TEMPLATE_RES_REF, ResRef::from_str("mg_npc").unwrap());
    npc.root.write(&utc::CONVERSATION, ResRef::from_str("mg_dlg").unwrap());
    m.set_gff(ResKey::parse("mg_npc", ResType::UTC).unwrap(), &npc).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("mg_probe").unwrap());
    m.set_info(&info).unwrap();
    m.set(ResKey::parse("mg_probe", ResType::NCS).unwrap(), compile(&dir, "mg_probe", PROBE));
    let user = dir.join("user");
    m.save_as(&ModuleLocation::Archive(user.join(format!("modules/{name}.mod")))).unwrap();
    let run = run_server(root, &user, name, "MG_DONE", Duration::from_secs(300)).unwrap();
    assert!(run.finished, "the server did not finish");
    let values = |k: &str| run.values(k).iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let said =
        |k: &str| (run.log.lines()).filter(|l| l.contains(k)).map(|_| k.to_string()).collect();
    (values("MG_ACT"), said("MG_LIBMAIN"), said("MG_COND"))
}

#[test]
fn a_condition_whose_include_has_a_main_runs_as_each_compiler_makes_it() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let built = |name: &str| {
        let mut c = mg_script::Compiler::new(|n: &str, t: ResType| match (n, t) {
            ("mg_lib", ResType::NSS) => Some(LIB.as_bytes().to_vec()),
            ("mg_cond", ResType::NSS) => Some(COND.as_bytes().to_vec()),
            _ => ResKey::parse(n, t).and_then(|k| gd.resman.get(&k).ok()).map(|d| d.into_owned()),
        });
        c.compile(name).unwrap_or_else(|e| panic!("{name}: {e}")).ncs
    };
    // Moonglow's: the condition itself, which passes; the include's main
    // never runs.
    let moonglow = built("mg_cond");
    // Aurora's: the include's main.
    let aurora = built("mg_lib");
    assert_ne!(moonglow, aurora);

    let (chosen, lib, cond) = speak(&root, "mg_entry_moonglow", &[("mg_c_moon", moonglow)]);
    assert_eq!(chosen, ["mg_c_moon"], "the condition passes");
    assert_eq!((lib.len(), cond.len()), (0, 1));

    let (chosen, lib, cond) = speak(&root, "mg_entry_aurora", &[("mg_c_aur", aurora)]);
    // The include's main runs in the condition's place, and the line is
    // passed over: a condition that returns nothing fails, whatever the
    // script says it returns.
    assert_eq!((lib.len(), cond.len()), (1, 0));
    assert_eq!(chosen, ["last"], "what the game makes of no return value");
}
