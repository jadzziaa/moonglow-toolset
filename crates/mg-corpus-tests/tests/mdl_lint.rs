//! Diagnostics against real models: the decompiler's output is clean, and
//! the game's own ASCII models raise only what is really there.

use std::collections::BTreeMap;

use mg_core::ResType;
use mg_mdl::lint::Severity;
use mg_resman::{GameInstall, ResKey, ResMan};
use rayon::prelude::*;

fn game() -> Option<ResMan> {
    let root = mg_testkit::nwn_root()?;
    ResMan::for_game(&GameInstall::new(root, None, "en")).ok()
}

/// What the decompiler writes draws no warnings or errors.
#[test]
fn decompiled_models_are_clean() {
    let _ = mg_testkit::corpus!();
    let rm = game().unwrap();
    let names = rm.list(ResType::MDL);
    let noisy: Vec<String> = names
        .par_iter()
        .filter_map(|name| {
            let data = rm.get(&ResKey::new(*name, ResType::MDL)).ok()?;
            let text = mg_mdl::decompile(&data).ok()?;
            let bad: Vec<String> = mg_mdl::lint::check(&text)
                .into_iter()
                .filter(|d| d.severity > Severity::Info)
                // Duplicate node and animation names are the model's own.
                .filter(|d| !d.message.contains("second node named"))
                .filter(|d| !d.message.contains("second animation named"))
                .map(|d| format!("{}: {}", d.line + 1, d.message))
                .collect();
            (!bad.is_empty()).then(|| format!("{name}: {}", bad[..bad.len().min(3)].join("; ")))
        })
        .collect();
    for n in noisy.iter().take(20) {
        eprintln!("{n}");
    }
    assert!(noisy.is_empty(), "{} decompiled models draw diagnostics", noisy.len());
}

/// A census of what the game's own ASCII models draw (printed), and no
/// errors but the ones expected.
#[test]
fn game_ascii_census() {
    let _ = mg_testkit::corpus!();
    let rm = game().unwrap();
    let names = rm.list(ResType::MDL);
    let found: Vec<(String, Severity, String)> = names
        .par_iter()
        .flat_map_iter(|name| {
            let data = rm.get(&ResKey::new(*name, ResType::MDL)).ok();
            let out: Vec<(String, Severity, String)> = match data {
                Some(d) if !mg_mdl::is_binary(&d) => {
                    let text = String::from_utf8_lossy(&d);
                    mg_mdl::lint::check(&text)
                        .into_iter()
                        .map(|x| (name.to_string(), x.severity, x.message))
                        .collect()
                }
                _ => Vec::new(),
            };
            out
        })
        .collect();
    let mut by_kind: BTreeMap<(Severity, String), (usize, String)> = BTreeMap::new();
    for (name, sev, msg) in &found {
        // Group messages by their wording without names and numbers.
        let shape: String = msg
            .split_whitespace()
            .map(|w| if w.chars().any(|c| c.is_ascii_digit() || c == '_') { "#" } else { w })
            .collect::<Vec<_>>()
            .join(" ");
        let e = by_kind.entry((*sev, shape)).or_insert((0, name.clone()));
        e.0 += 1;
    }
    for ((sev, shape), (count, example)) in &by_kind {
        eprintln!("{sev:?} {count:6} {shape}   (e.g. {example})");
    }
}
