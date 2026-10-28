//! Build › Compile (scripts) compared with Aurora's Build Module: the probe
//! module of `tools/aurora/capture_scripts.py` (the base game's
//! `x2_inc_cutscene` included, return types declared one way and defined
//! another, include files, comments of every form) compiled by Aurora
//! (`script-compile/build.mod`) and by Moonglow.
//!
//! Moonglow makes the compiled scripts Aurora made, byte for byte, and no
//! others; it fails the scripts Aurora reported "Compile Error" for, but
//! the include files (no `main`): those Moonglow passes over by decision
//! (docs/deferred.md, "Include files"), where Aurora reports one that
//! doesn't compile alone.

use std::collections::BTreeSet;

use mg_core::ResType;
use mg_module::Module;
use mg_module::build::{ScriptSelection, compile_scripts};
use mg_resman::{GameInstall, ResMan};
use mg_testkit::{aurora_capture, corpus};

/// The Results list of Aurora's Build Module (`build-results.png`).
const AURORA_ERRORS: [&str; 6] =
    ["cm_backslash2", "cm_nested", "inc_bad", "inc_lean", "inc_syntax", "uses_inc_bad"];

/// Of those, the include files.
const INCLUDES: [&str; 3] = ["inc_bad", "inc_lean", "inc_syntax"];

fn compiled(m: &Module) -> BTreeSet<String> {
    m.keys_of(ResType::NCS).map(|k| k.resref.to_lowercase().to_string()).collect()
}

#[test]
fn scripts_compile_as_aurora_builds_them() {
    let root = corpus!();
    let before = aurora_capture!("script-compile/before.mod");
    let aurora = Module::open(&aurora_capture!("script-compile/build.mod")).unwrap();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let mut m = Module::open(&before).unwrap();
    assert!(compiled(&m).is_empty(), "the probe comes without compiled scripts");
    let results = compile_scripts(&mut m, &rm, ScriptSelection::All);

    assert_eq!(compiled(&m), compiled(&aurora), "the scripts compiled");
    for k in aurora.keys_of(ResType::NCS) {
        assert!(m.get(k) == aurora.get(k), "{k}: not the bytes Aurora made");
    }
    let failed: BTreeSet<String> = (results.iter())
        .filter(|r| r.result.is_err())
        .map(|r| r.script.resref.to_lowercase().to_string())
        .collect();
    let expected: BTreeSet<String> =
        (AURORA_ERRORS.iter()).filter(|s| !INCLUDES.contains(s)).map(|s| s.to_string()).collect();
    assert_eq!(failed, expected);

    // The return types that differ compile with a warning naming the
    // function and both lines; the other scripts warn of nothing.
    let warned = |name: &str| {
        let r = results.iter().find(|r| r.script.resref.to_lowercase().to_string() == name);
        let r = r.unwrap();
        r.warnings.iter().map(|w| w.message.clone()).collect::<Vec<_>>()
    };
    for name in ["cut_inc", "cut_call", "cut_use"] {
        let said = warned(name);
        assert_eq!(said.len(), 1, "{name}: {said:?}");
        assert!(
            said[0].starts_with(
                "x2_inc_cutscene.nss(628): WARNING: CutCreateObjectCopy is declared object at \
                 x2_inc_cutscene.nss(303) and defined void at x2_inc_cutscene.nss(628)"
            ),
            "{}",
            said[0]
        );
    }
    assert!(warned("rt_declvoid")[0].contains("overwrites the caller's last variable"));
    for name in ["rt_declint", "rt_usedint", "rt_unused"] {
        assert!(warned(name)[0].contains("is always 0"), "{name}: {:?}", warned(name));
    }
    let quiet = results.iter().filter(|r| r.warnings.is_empty()).count();
    assert_eq!(quiet, results.len() - 7);
}
