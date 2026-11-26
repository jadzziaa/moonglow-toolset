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

/// A script, and an include, both in the module and in one of its haks
/// (`tools/aurora/capture_compile_scale.py`'s `hak-order.mod`): Aurora
/// compiles the module's copies, though the game's resource manager puts
/// a hak above the module, and so does Moonglow.
#[test]
fn scripts_in_the_module_and_a_hak_compile_from_the_module_s_copy() {
    let root = corpus!();
    let hak = aurora_capture!("script-compile-scale/hak_order.hak");
    let aurora =
        Module::open(&aurora_capture!("script-compile-scale/hak-order-build.mod")).unwrap();
    let mut m = Module::open(&aurora_capture!("script-compile-scale/hak-order.mod")).unwrap();
    // The hak from beside the captures, as a user directory's.
    let user = mg_testkit::test_output().join("aurora_scripts_hak_order");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    std::fs::copy(&hak, user.join("hak").join("hak_order.hak")).unwrap();
    let install = GameInstall::new(&root, Some(user), "en");
    let mut rm = ResMan::for_game(&install).unwrap();
    assert!(rm.add_haks(&install, &["hak_order"]).unwrap().is_empty());
    let hak_self = rm.get(&mg_resman::ResKey::parse("ho_self", ResType::NSS).unwrap()).unwrap();
    assert!(String::from_utf8_lossy(&hak_self).contains("\"hak\""), "the hak is read");

    let results = compile_scripts(&mut m, &rm, ScriptSelection::All);
    assert!(results.iter().all(|r| r.result.is_ok()));
    assert_eq!(compiled(&m), compiled(&aurora));
    for k in aurora.keys_of(ResType::NCS) {
        let ncs = m.get(k).unwrap();
        assert!(Some(ncs) == aurora.get(k), "{k}: not the bytes Aurora made");
        let text = String::from_utf8_lossy(ncs);
        assert!(text.contains("module") && !text.contains("hak"), "{k}: the hak's copy");
    }
}

/// Every fourth of the base game's script sources (1,036) as a module's
/// own (`tools/aurora/capture_compile_scale.py`'s `before.mod`: a third
/// of them with LF line ends), built in Aurora and saved (`build.mod`):
/// Moonglow makes the compiled scripts Aurora made, byte for byte, and no
/// others. (The line ends make no difference to either.)
#[test]
fn scripts_at_scale_compile_as_aurora_builds_them() {
    let root = corpus!();
    let aurora = Module::open(&aurora_capture!("script-compile-scale/build.mod")).unwrap();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let mut m = Module::open(&aurora_capture!("script-compile-scale/before.mod")).unwrap();
    compile_scripts(&mut m, &rm, ScriptSelection::All);
    let differ: Vec<String> = (aurora.keys_of(ResType::NCS))
        .filter(|k| m.get(k) != aurora.get(k))
        .map(|k| k.to_string())
        .collect();
    assert_eq!(compiled(&m), compiled(&aurora), "the scripts compiled");
    assert!(differ.is_empty(), "not the bytes Aurora made: {differ:?}");
}

/// Constructs a compiler of another date might make other code of
/// (`capture_compile_scale.py`'s `codegen.mod`, built in Aurora and saved:
/// `codegen-build.mod`): constant expressions, unary minus, strings,
/// floats, comparisons, `?:`, shifts, switch, structs, vectors, loops make
/// Aurora's bytes. One way of writing differs: a script that includes a
/// file with a `void main()` of its own. Aurora takes the include's `main`
/// as the script's entry point (an `int StartingConditional()` script
/// becomes the include's `main`, and a script that is only the
/// `#include` compiles to it too); Moonglow's compiler, as neverwinter.nim
/// since d20453e3 (2026-08), takes it from the script itself, and passes a
/// script with none over as an include. What the game makes of each:
/// `engine_include_entry`.
#[test]
fn codegen_compiles_as_aurora_builds_it() {
    let root = corpus!();
    let aurora = Module::open(&aurora_capture!("script-compile-scale/codegen-build.mod")).unwrap();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let mut m = Module::open(&aurora_capture!("script-compile-scale/codegen.mod")).unwrap();
    let results = compile_scripts(&mut m, &rm, ScriptSelection::All);
    assert!(results.iter().all(|r| r.result.is_ok()));
    let key = |n: &str| mg_resman::ResKey::parse(n, ResType::NCS).unwrap();
    let differ: BTreeSet<String> = (aurora.keys_of(ResType::NCS))
        .filter(|k| m.get(k) != aurora.get(k))
        .map(|k| k.resref.to_lowercase().to_string())
        .collect();
    assert_eq!(differ, BTreeSet::from(["cg_cond", "cg_inconly", "cg_usesc"].map(String::from)));
    // Aurora's: the include's entry point, compiled as the include alone.
    assert_eq!(aurora.get(&key("cg_cond")), aurora.get(&key("cg_incmain")));
    assert_eq!(aurora.get(&key("cg_inconly")), aurora.get(&key("cg_incmain")));
    assert_eq!(aurora.get(&key("cg_usesc")), aurora.get(&key("cg_incsc")));
    // Moonglow's: the script's own StartingConditional; none for the other two.
    assert!(m.get(&key("cg_cond")).is_some());
    assert!(m.get(&key("cg_inconly")).is_none() && m.get(&key("cg_usesc")).is_none());
    // The owner's ruling of 2026-11-26: Moonglow's kept, and each script
    // Aurora builds otherwise warned of, none else.
    let warned: BTreeSet<String> = (results.iter())
        .filter(|r| {
            r.warnings.iter().any(|w| {
                w.message.contains("has a main of its own")
                    || w.message.contains("has no main or StartingConditional")
            })
        })
        .map(|r| r.script.resref.to_lowercase().to_string())
        .collect();
    assert_eq!(warned, differ);
}
