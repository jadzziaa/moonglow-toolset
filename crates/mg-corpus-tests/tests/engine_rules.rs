//! Game data compared with the engine: 2DA cells (`Get2DAString`) and
//! talk-table strings (`GetStringByStrRef`).

use std::time::Duration;

use mg_core::{ResType, StrRef};
use mg_resman::GameInstall;
use mg_rules::GameData;
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::probe_module;

/// NWScript string literal for `s` (2DA names and columns are plain ASCII).
fn lit(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// For cells sampled from every base-game 2DA (plus out-of-range rows and
/// unknown columns), Moonglow's value equals `Get2DAString`'s (empty for
/// `****`); for sampled StrRefs, its text equals `GetStringByStrRef`'s.
#[test]
fn rules_data_matches_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_rules");
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();

    // (label, our value) pairs, and the matching probe lines.
    let mut expected: Vec<(String, String)> = Vec::new();
    let mut probes = String::new();
    let mut probe = |label: String, expr: String, ours: String| {
        probes += &format!(
            "    WriteTimestampedLogEntry(\"MG_VAL {label}=\" + JsonDump(JsonString({expr})));\n"
        );
        expected.push((label, ours));
    };

    let mut n = 0usize;
    for name in gd.resman.list(ResType::TWODA) {
        let name = name.to_lowercase().to_string();
        let Ok(t) = gd.table(&name) else { continue };
        if t.columns().is_empty() {
            continue;
        }
        n += 1;
        // A few cells spread over the table, deterministic per table.
        let rows = t.len().max(1);
        for k in 0..3 {
            let row = (n * 7 + k * 13) % rows;
            let col = (n + k * 5) % t.columns().len();
            let c = &t.columns()[col];
            probe(
                format!("{name}[{row}].{c}"),
                format!("Get2DAString({}, {}, {row})", lit(&name), lit(c)),
                t.get(row, c).unwrap_or_default().to_string(),
            );
        }
        let c = &t.columns()[0];
        probe(
            format!("{name}[past end].{c}"),
            format!("Get2DAString({}, {}, {})", lit(&name), lit(c), t.len() + 5),
            String::new(),
        );
        probe(
            format!("{name}[0].nosuchcolumn"),
            format!("Get2DAString({}, \"nosuchcolumn\", 0)", lit(&name)),
            String::new(),
        );
    }
    for s in (0..130_000u32).step_by(997).chain([0, 1, 5197, 7492, 0x00FF_FFFF]) {
        probe(
            format!("strref {s}"),
            format!("GetStringByStrRef({s})"),
            gd.string(StrRef(s)).unwrap_or_default(),
        );
    }

    let script =
        format!("void main()\n{{\n{probes}    WriteTimestampedLogEntry(\"MG_DONE\");\n}}\n");
    probe_module(&root, &dir, "mg_rules", &script, &[], &[]);
    let run = run_server(&root, &dir, "mg_rules", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(
        run.finished,
        "server did not finish; log tail:\n{}",
        &run.log[run.log.len().saturating_sub(3000)..]
    );

    let engine: std::collections::HashMap<String, String> = run
        .values("MG_VAL")
        .into_iter()
        .filter_map(|v| {
            let (label, json) = v.split_once('=')?;
            Some((label.to_string(), serde_json::from_str::<String>(json).ok()?))
        })
        .collect();
    let failures: Vec<String> = expected
        .iter()
        .filter_map(|(label, ours)| match engine.get(label) {
            Some(theirs) if theirs == ours => None,
            Some(theirs) => Some(format!("{label}: ours {ours:?}, engine {theirs:?}")),
            None => Some(format!("{label}: not logged by the engine")),
        })
        .collect();
    eprintln!("compared {} values from {n} tables and the talk table", expected.len());
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
