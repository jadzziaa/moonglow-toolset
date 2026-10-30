//! Compiled scripts (`.ncs`) read by Moonglow (`mg_script::ncs`) and
//! disassembled by neverwinter.nim's `nwn_asm`: the same instructions at the
//! same offsets, and the same engine function at each `ACTION` (its id is
//! its place in `nwscript.nss`). Over the game's own scripts it also counts
//! how many calls of the functions that take a table's row by a number
//! have that row as a constant, which the usage walk follows.

use std::collections::BTreeMap;
use std::process::Command;

use mg_core::ResType;
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_script::ncs::{self, Arg, Const};
use mg_script::spec::Spec;
use mg_testkit::{at_least, corpus, oracle_tool, scratch_dir, sweep};

/// The functions that take a row by a number, and which argument it is
/// (`mg_preview::usage`'s). Over the whole install (2026-10-30): 4,520
/// calls with the row a constant, 502 without; `EffectVisualEffect` 3,342
/// of 3,449, the spells cast 1,115 of 1,462, `SetCreatureAppearanceType` 1
/// of 46 (the game's scripts pass a variable).
const ROW_TAKERS: [(&str, usize); 9] = [
    ("EffectVisualEffect", 0),
    ("EffectBeam", 0),
    ("ActionCastSpellAtObject", 0),
    ("ActionCastSpellAtLocation", 0),
    ("ActionCastFakeSpellAtObject", 0),
    ("ActionCastFakeSpellAtLocation", 0),
    ("ItemPropertyVisualEffect", 0),
    ("ItemPropertyCastSpell", 0),
    ("SetCreatureAppearanceType", 1),
];

/// `nwn_asm -G -d`'s lines: each instruction's offset and mnemonic, and
/// for an `ACTION` the function's name.
fn disassembled(asm: &std::path::Path, file: &std::path::Path) -> Vec<(usize, String, String)> {
    let out = Command::new(asm).args(["-G", "-d"]).arg(file).output().unwrap();
    assert!(out.status.success(), "{}: {}", file.display(), String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    let mut plain = String::new();
    let mut esc = false;
    for c in text.chars() {
        match (esc, c) {
            (_, '\u{1b}') => esc = true,
            (true, 'm') => esc = false,
            (true, _) => {}
            (false, c) => plain.push(c),
        }
    }
    plain
        .lines()
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            let offset = words.next()?.parse().ok()?;
            let mnemonic = words.next()?.to_string();
            let operand = words.next().unwrap_or_default().to_string();
            Some((offset, mnemonic, operand))
        })
        .collect()
}

#[test]
fn compiled_scripts_read_as_nwn_asm_reads_them() {
    let root = corpus!();
    let asm = oracle_tool!("nwn_asm");
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let spec = Spec::parse(&rm.get(&ResKey::parse("nwscript", ResType::NSS).unwrap()).unwrap());
    let mut keys: Vec<ResKey> =
        rm.entries().iter().map(|(k, _)| *k).filter(|k| k.restype == ResType::NCS).collect();
    keys.sort_by_key(|k| k.to_string());
    let total = keys.len();
    let keys = sweep(keys, 50);
    let dir = scratch_dir("ncs");
    std::fs::create_dir_all(&dir).unwrap();
    let mut read = 0;
    let mut instructions = 0;
    // Per function: calls with the row a constant, and without.
    let mut rows: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for key in &keys {
        let data = rm.get(key).unwrap();
        let code = ncs::decode(&data).unwrap_or_else(|e| panic!("{key}: {e}"));
        let file = dir.join(key.to_string());
        std::fs::write(&file, &data).unwrap();
        let theirs = disassembled(&asm, &file);
        let ours: Vec<usize> = code.iter().map(|i| i.offset).collect();
        let their_offsets: Vec<usize> = theirs.iter().map(|t| t.0).collect();
        assert_eq!(ours, their_offsets, "{key}: the instructions' offsets");
        for (i, (_, mnemonic, operand)) in code.iter().zip(&theirs) {
            if let Arg::Action { routine, .. } = i.arg {
                assert_eq!(mnemonic, "ACTION", "{key} at {}", i.offset);
                let name = &spec.functions[usize::from(routine)].name;
                assert_eq!(name, operand, "{key} at {}: the function", i.offset);
            }
        }
        for call in ncs::calls(&code, &spec) {
            let name = spec.functions[usize::from(call.routine)].name.as_str();
            let Some((f, arg)) = ROW_TAKERS.iter().find(|(f, _)| *f == name) else { continue };
            let e = rows.entry(f).or_default();
            match call.args.get(*arg) {
                Some(Some(Const::Int(_))) => e.0 += 1,
                _ => e.1 += 1,
            }
        }
        instructions += code.len();
        read += 1;
    }
    eprintln!(
        "{read} of {total} compiled scripts, {instructions} instructions, as nwn_asm reads them"
    );
    for (f, (known, not)) in &rows {
        eprintln!("  {f}: {known} calls with the row a constant, {not} without");
    }
    assert!(read >= at_least(total, 50) && total > 1000, "{read} of {total}");
    let (known, not) = rows.values().fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    assert!(known > not, "most rows are constants: {known} told, {not} not");
}
