//! Which resource names the game takes, where Moonglow's checks of a
//! name disagree (ASCII of 16; letters, digits and `_`; those and `-`;
//! lower case alone): one compiled script kept in a module under names of
//! each kind, each looked up (`ResManGetAliasFor`) and run
//! (`ExecuteScript`) by its name, and the names the engine lists
//! (`ResManFindPrefix`), run by the names listed. The engine takes every
//! one: upper case (found by either case, listed in lower case), `-`, `.`,
//! a space, sixteen characters and a byte past ASCII.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_resman::ResKey;
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common::{compile, probe_module};

const RAN: &str = r#"
void main()
{
    object m = GetModule();
    SetLocalInt(m, "mg_ran", GetLocalInt(m, "mg_ran") + 1);
}
"#;

/// The names the script is kept under in the module.
const STORED: [&[u8]; 7] = [
    b"mg_lower",
    b"MG_UPPER",
    b"mg-dash",
    b"mg.dot",
    b"mg sp",
    b"mg_sixteen_chars",
    b"mg\xe9t\xe9",
];

/// The names a script asks for (each of the ASCII ones, and two in the
/// other case).
const ASKED: [&str; 8] = [
    "mg_lower",
    "MG_LOWER",
    "mg_upper",
    "MG_UPPER",
    "mg-dash",
    "mg.dot",
    "mg sp",
    "mg_sixteen_chars",
];

#[test]
fn the_engine_takes_any_resource_name_of_sixteen_bytes() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_resource_names");
    let ran = compile(&dir, "mg_ran", RAN);
    let extra: Vec<(ResKey, Vec<u8>)> = STORED
        .iter()
        .map(|name| (ResKey::new(ResRef::from_bytes(name).unwrap(), ResType::NCS), ran.clone()))
        .collect();
    let mut probe = String::from(
        "void Try(string sMark, string s)\n{\n    object m = GetModule();\n    int n = \
         GetLocalInt(m, \"mg_ran\");\n    ExecuteScript(s, m);\n    \
         WriteTimestampedLogEntry(sMark + \" \" + s + \"|\" + ResManGetAliasFor(s, RESTYPE_NCS) \
         + \"|\" + IntToString(GetLocalInt(m, \"mg_ran\") - n));\n}\n\nvoid main()\n{\n",
    );
    for asked in ASKED {
        probe += &format!("    Try(\"MG_ASKED\", \"{asked}\");\n");
    }
    probe += "    int i;\n    for (i = 1; i < 20; i++)\n    {\n        string s = \
              ResManFindPrefix(\"mg\", RESTYPE_NCS, i);\n        if (s == \"\") break;\n        \
              Try(\"MG_LISTED\", s);\n    }\n    WriteTimestampedLogEntry(\"MG_DONE\");\n}\n";
    probe_module(&root, &dir, "names_probe", &probe, &[], &extra);
    let run = run_server(&root, &dir, "names_probe", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "{}", &run.log[run.log.len().saturating_sub(3000)..]);
    let found = "|CURRENTGAME:names_probe|1";
    let wanted: Vec<String> = ASKED.iter().map(|a| format!("{a}{found}")).collect();
    assert_eq!(run.values("MG_ASKED"), wanted, "found and run by its name");
    // Listed in lower case, the byte past ASCII as it is (read here as
    // U+FFFD), each run by the name listed.
    let mut listed = run.values("MG_LISTED");
    listed.sort();
    let mut wanted: Vec<String> = [
        "mg sp",
        "mg-dash",
        "mg.dot",
        "mg_lower",
        "mg_sixteen_chars",
        "mg_upper",
        "mg\u{fffd}t\u{fffd}",
    ]
    .iter()
    .map(|n| format!("{n}{found}"))
    .collect();
    wanted.sort();
    assert_eq!(listed, wanted, "{}", run.log);
}
