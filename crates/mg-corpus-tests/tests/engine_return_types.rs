//! A function declared with one return type and defined with another,
//! compiled as Aurora compiles it (the built-in compiler's
//! return-type-mismatch patch, `crates/mg-script/nwscript/README.md`), run
//! in the game: what its callers get, and what becomes of their
//! variables. Newer builds of the compiler (nwn_script_comp 2.3.1) refuse
//! all of these; the base game's `x2_inc_cutscene` is one.

use std::time::Duration;

use mg_core::ResType;
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::probe_module;

/// Each case: a script of its own (one that stops on an error stops alone).
const CASES: [(&str, &str); 4] = [
    // Declared int, defined void, its value kept.
    (
        "mg_rt_void",
        "int Foo();\n\
         void main()\n{\n    int a = 11;\n    int n = Foo();\n    int b = 22;\n\
         WriteTimestampedLogEntry(\"MG_RT_VOID \" + IntToString(a) + \" \" + IntToString(n) + \" \" + IntToString(b));\n}\n\
         void Foo() { int x = 5; }\n",
    ),
    // Declared void, defined int: its value goes where no room was kept.
    (
        "mg_rt_unkept",
        "void Foo();\n\
         void main()\n{\n    int a = 11;\n    int b = 22;\n    Foo();\n\
         WriteTimestampedLogEntry(\"MG_RT_UNKEPT \" + IntToString(a) + \" \" + IntToString(b));\n}\n\
         int Foo() { return 99; }\n",
    ),
    // Declared string, defined int: an int where a string is kept.
    (
        "mg_rt_other",
        "string Foo();\n\
         void main()\n{\n    string s = Foo();\n    WriteTimestampedLogEntry(\"MG_RT_OTHER_GOT\");\n\
         WriteTimestampedLogEntry(\"MG_RT_OTHER [\" + s + \"]\");\n}\n\
         int Foo() { return 99; }\n",
    ),
    // The base game's: CutCreateObjectCopy, declared object, defined void.
    (
        "mg_rt_cut",
        "#include \"x2_inc_cutscene\"\n\
         void main()\n{\n    object o = CutCreateObjectCopy(0.0, OBJECT_INVALID, GetStartingLocation());\n\
         WriteTimestampedLogEntry(\"MG_RT_CUT \" + IntToString(GetIsObjectValid(o)) + \" \" + ObjectToString(o));\n}\n",
    ),
];

#[test]
fn a_mismatched_return_type_runs_in_the_game_as_compiled() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut extra = Vec::new();
    for (name, text) in CASES {
        let mut c = mg_script::Compiler::new(|n: &str, t: ResType| {
            if t == ResType::NSS && n == name {
                return Some(text.as_bytes().to_vec());
            }
            ResKey::parse(n, t).and_then(|k| gd.resman.get(&k).ok()).map(|d| d.into_owned())
        });
        let out = c.compile(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(out.warnings.len(), 1, "{name}: {:?}", out.warnings);
        extra.push((ResKey::parse(name, ResType::NCS).unwrap(), out.ncs));
    }
    // (The module's own OnModuleLoad, compiled by nwn_script_comp, runs
    // each case in turn.)
    let mut load = String::from("void main()\n{\n");
    for (i, (name, _)) in CASES.iter().enumerate() {
        load +=
            &format!("    DelayCommand({}.0, ExecuteScript(\"{name}\", GetModule()));\n", i + 1);
    }
    load += &format!(
        "    DelayCommand({}.0, WriteTimestampedLogEntry(\"MG_DONE\"));\n}}\n",
        CASES.len() + 1
    );
    let dir = scratch_dir("mg_rettypes");
    probe_module(&root, &dir, "mg_rettypes", &load, &[], &extra);
    let run = run_server(&root, &dir, "mg_rettypes", "MG_DONE", Duration::from_secs(60)).unwrap();
    assert!(
        run.finished,
        "server did not finish; log tail:\n{}",
        &run.log[run.log.len().saturating_sub(3000)..]
    );
    eprintln!("{}", run.log);

    // Declared int, defined void: an empty int, the others untouched.
    assert_eq!(run.values("MG_RT_VOID"), ["11 0 22"]);
    // Declared void, defined int: its value overwrites the caller's last
    // variable.
    assert_eq!(run.values("MG_RT_UNKEPT"), ["11 99"]);
    // An int where a string is kept: the script stops where it is used.
    assert_eq!(run.values("MG_RT_OTHER_GOT"), [""]);
    assert!(run.values("MG_RT_OTHER ").is_empty(), "{:?}", run.values("MG_RT_OTHER"));
    // x2_inc_cutscene's: OBJECT_INVALID, always.
    assert_eq!(run.values("MG_RT_CUT"), ["0 7f000000"]);
}
