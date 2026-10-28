//! Phase 4 exit check: the built-in compiler produces exactly what
//! `nwn_script_comp` (the same compiler, as a separate program) produces, for
//! every script source in the base game and in every shipped module (with its
//! haks). Where compiled scripts ship too, how many match is reported.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use mg_core::ResType;
use mg_module::Module;
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_script::Compiler;
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::script_common::{each_script_set, on_every_core};

/// A script compiled by the built-in compiler: its bytecode and what the
/// compile warned of.
type Ours = Option<(Vec<u8>, Vec<mg_script::CompileWarning>)>;

/// Compiles `names` with the built-in compiler on all cores; `None` for
/// scripts that do not compile.
fn compile_all(rm: &ResMan, names: &[String]) -> BTreeMap<String, Ours> {
    on_every_core(names, |part| {
        let mut c = Compiler::new(|name, t| {
            ResKey::parse(name, t).and_then(|k| rm.get(&k).ok()).map(|d| d.into_owned())
        });
        (part.iter())
            .map(|n| (n.clone(), c.compile(n).ok().map(|o| (o.ncs, o.warnings))))
            .collect::<Vec<_>>()
    })
    .into_iter()
    .flatten()
    .collect()
}

/// Compiles the sources in `src` with `nwn_script_comp`; `None` for scripts
/// it could not compile.
fn oracle_compile(
    tool: &Path,
    root: &Path,
    empty_user: &Path,
    src: &Path,
    out: &Path,
    erfs: &[std::path::PathBuf],
    names: &[String],
) -> BTreeMap<String, Option<Vec<u8>>> {
    std::fs::create_dir_all(out).unwrap();
    let mut cmd = Command::new(tool);
    cmd.arg("--root")
        .arg(root)
        .arg("--userdirectory")
        .arg(empty_user)
        .args(["--no-ovr", "--quiet", "-y"]);
    if !erfs.is_empty() {
        let list: Vec<String> = erfs.iter().map(|p| p.display().to_string()).collect();
        cmd.arg("--erfs").arg(list.join(","));
    }
    cmd.arg("--dirs").arg(src).arg("-c").arg("-d").arg(out).arg(src);
    let _ = cmd.output().unwrap();
    names.iter().map(|n| (n.clone(), std::fs::read(out.join(format!("{n}.ncs"))).ok())).collect()
}

#[derive(Default)]
struct Tally {
    compiled: usize,
    failed: usize,
    shipped_same: usize,
    shipped_total: usize,
    /// Compiled here, as Aurora does, and refused by nwn_script_comp for a
    /// function's return type (the return-type-mismatch patch).
    mismatched: usize,
    /// Of those, how many have a compiled script shipped beside them, and
    /// how many of those are byte for byte the same.
    mismatched_shipped: usize,
    mismatched_shipped_same: usize,
    /// The kinds of mismatch met (the warning's words after the lines).
    kinds: BTreeMap<String, usize>,
}

fn compare(
    label: &str,
    ours: &BTreeMap<String, Ours>,
    theirs: &BTreeMap<String, Option<Vec<u8>>>,
    shipped: impl Fn(&str) -> Option<Vec<u8>>,
    tally: &mut Tally,
    failures: &mut Vec<String>,
) {
    for (name, a) in ours {
        let b = theirs.get(name).cloned().flatten();
        match (a, &b) {
            (Some((x, _)), Some(y)) if x == y => tally.compiled += 1,
            (None, None) => tally.failed += 1,
            (Some(_), Some(_)) => {
                failures.push(format!("{label}: {name}.nss compiles differently"))
            }
            // The one divergence by design: nwn_script_comp 2.3.1 refuses
            // a function declared with one return type and defined with
            // another, which Aurora (and so Moonglow) compiles; the
            // compile warns of it. Settled by Aurora's own output
            // (`aurora_scripts`), by the game (`engine_return_types`) and
            // by the compiled scripts shipped beside these.
            (Some((x, warnings)), None) if !warnings.is_empty() => {
                tally.mismatched += 1;
                for w in warnings {
                    let kind = w.message.split_once("WARNING: ").map_or("", |(_, k)| k);
                    *tally.kinds.entry(kind.to_string()).or_default() += 1;
                }
                if let Some(s) = shipped(name) {
                    tally.mismatched_shipped += 1;
                    tally.mismatched_shipped_same += usize::from(*x == s);
                }
            }
            (Some(_), None) => failures
                .push(format!("{label}: {name}.nss compiles here but not with nwn_script_comp")),
            (None, Some(_)) => failures
                .push(format!("{label}: {name}.nss compiles with nwn_script_comp but not here")),
        }
        if let (Some((x, _)), Some(s)) = (a, shipped(name)) {
            tally.shipped_total += 1;
            tally.shipped_same += usize::from(*x == s);
        }
    }
}

#[test]
fn builtin_compiler_matches_nwn_script_comp() {
    let root = corpus!();
    let tool = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("scripts");
    let empty_user = dir.join("empty-user");
    std::fs::create_dir_all(&empty_user).unwrap();
    let mut tally = Tally::default();
    let mut failures = Vec::new();

    // The base game's script sources, then every shipped module's over its
    // haks and the game.
    each_script_set(&root, |set| {
        let (rm, names) = (set.rm, set.names);
        let ours = compile_all(rm, names);
        let Some((m, haks, i)) = set.module else {
            let src = dir.join("base-src");
            std::fs::create_dir_all(&src).unwrap();
            for n in names {
                let data = rm.get(&ResKey::parse(n, ResType::NSS).unwrap()).unwrap();
                std::fs::write(src.join(format!("{n}.nss")), data).unwrap();
            }
            let out = dir.join("base-out");
            let theirs = oracle_compile(&tool, &root, &empty_user, &src, &out, &[], names);
            let shipped =
                |n: &str| rm.get(&ResKey::parse(n, ResType::NCS)?).ok().map(|d| d.into_owned());
            compare("base", &ours, &theirs, shipped, &mut tally, &mut failures);
            eprintln!("base game: {} scripts", names.len());
            return;
        };
        let src = dir.join(format!("m{i}-src"));
        std::fs::create_dir_all(&src).unwrap();
        for k in m.keys_of(ResType::NSS) {
            std::fs::write(src.join(k.to_string()), m.get(k).unwrap()).unwrap();
        }
        // nwn_script_comp stacks each --erfs entry above the previous one,
        // so the first hak (highest priority) goes last.
        let hak_files: Vec<_> =
            haks.iter().rev().map(|h| root.join("data/hk").join(format!("{h}.hak"))).collect();
        let out = dir.join(format!("m{i}-out"));
        let theirs = oracle_compile(&tool, &root, &empty_user, &src, &out, &hak_files, names);
        let shipped = |n: &str| m.get(&ResKey::parse(n, ResType::NCS)?).map(<[u8]>::to_vec);
        compare(&set.label, &ours, &theirs, shipped, &mut tally, &mut failures);
        eprintln!("{}: {} scripts", set.label, names.len());
    });

    eprintln!(
        "compiled {} scripts identically to nwn_script_comp ({} fail to compile in both); \
         {} of {} equal the compiled scripts shipped beside them",
        tally.compiled, tally.failed, tally.shipped_same, tally.shipped_total
    );
    eprintln!(
        "{} compile here, as in Aurora, and not with nwn_script_comp (a return type that \
         differs); {} of {} equal the compiled scripts shipped beside them",
        tally.mismatched, tally.mismatched_shipped_same, tally.mismatched_shipped
    );
    for (kind, n) in &tally.kinds {
        eprintln!("  {n} × {kind}");
    }
    assert!(tally.compiled > if mg_testkit::full_corpus() { 5000 } else { 500 });
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// The external compiler option (Options › Script Editor): a module's
/// scripts compiled by `nwn_script_comp` run as a program come out as the
/// built-in compiler makes them, an include is left alone, and a script
/// that doesn't compile has the compiler's own words for why.
#[test]
fn an_external_compiler_compiles_a_module_s_scripts() {
    use mg_module::build::{ScriptSelection, compile_scripts, compile_scripts_with};
    use mg_module::external_compiler::ExternalCompiler;
    let root = corpus!();
    let tool = oracle_tool!("nwn_script_comp");
    let install = GameInstall::new(&root, None, "en");
    let rm = ResMan::for_game(&install).unwrap();
    let key = |name: &str, t| ResKey::parse(name, t).unwrap();
    let module = || {
        let mut m = Module::new();
        m.set(key("mg_inc", ResType::NSS), b"string Hi() { return \"hi\"; }\n".to_vec());
        m.set(
            key("mg_say", ResType::NSS),
            b"#include \"mg_inc\"\nvoid main() { SpeakString(Hi()); }\n".to_vec(),
        );
        m.set(key("mg_broken", ResType::NSS), b"void main() {\n  Nope();\n}\n".to_vec());
        m
    };
    let (mut built_in, mut external) = (module(), module());
    compile_scripts(&mut built_in, &rm, ScriptSelection::All);
    let compiler = ExternalCompiler {
        program: tool,
        game: Some(root),
        user: Some(scratch_dir("external-compiler-user")),
        ..Default::default()
    };
    let results =
        compile_scripts_with(&mut external, &rm, ScriptSelection::All, Some(&compiler), false);
    let said = |name: &str| {
        let r = results.iter().find(|r| r.script == key(name, ResType::NSS)).unwrap();
        r.result.clone()
    };
    let ncs = key("mg_say", ResType::NCS);
    assert!(external.get(&ncs).is_some(), "{:?}", said("mg_say"));
    assert_eq!(external.get(&ncs), built_in.get(&ncs), "the same compiler, the same bytes");
    assert_eq!(said("mg_inc"), Ok(()), "an include isn't an error");
    assert!(external.get(&key("mg_inc", ResType::NCS)).is_none());
    let error = said("mg_broken").unwrap_err();
    assert!(error.message.contains("UNDEFINED IDENTIFIER"), "{}", error.message);
    assert_eq!(error.location(), Some(("mg_broken".into(), 2)));
}
