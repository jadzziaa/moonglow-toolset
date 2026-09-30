//! Phase 4 exit check: the built-in compiler produces exactly what
//! `nwn_script_comp` (the same compiler, as a separate program) produces, for
//! every script source in the base game and in every shipped module (with its
//! haks). Where compiled scripts ship too, how many match is reported.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use mg_core::ResType;
use mg_module::Module;
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_script::Compiler;
use mg_testkit::{bundled_modules, corpus, oracle_tool, scratch_dir};

/// Compiles `names` with the built-in compiler on all cores; `None` for
/// scripts that do not compile.
fn compile_all(rm: &ResMan, names: &[String]) -> BTreeMap<String, Option<Vec<u8>>> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = names.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = names
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    let mut c = Compiler::new(|name, t| {
                        ResKey::parse(name, t).and_then(|k| rm.get(&k).ok()).map(|d| d.into_owned())
                    });
                    part.iter()
                        .map(|n| (n.clone(), c.compile(n).ok().map(|o| o.ncs)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
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

struct Tally {
    compiled: usize,
    failed: usize,
    shipped_same: usize,
    shipped_total: usize,
}

fn compare(
    label: &str,
    ours: &BTreeMap<String, Option<Vec<u8>>>,
    theirs: &BTreeMap<String, Option<Vec<u8>>>,
    shipped: impl Fn(&str) -> Option<Vec<u8>>,
    tally: &mut Tally,
    failures: &mut Vec<String>,
) {
    for (name, a) in ours {
        let b = theirs.get(name).cloned().flatten();
        match (a, &b) {
            (Some(x), Some(y)) if x == y => tally.compiled += 1,
            (None, None) => tally.failed += 1,
            (Some(_), Some(_)) => {
                failures.push(format!("{label}: {name}.nss compiles differently"))
            }
            (Some(_), None) => failures
                .push(format!("{label}: {name}.nss compiles here but not with nwn_script_comp")),
            (None, Some(_)) => failures
                .push(format!("{label}: {name}.nss compiles with nwn_script_comp but not here")),
        }
        if let (Some(x), Some(s)) = (a, shipped(name)) {
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
    let install = GameInstall::new(&root, None, "en");
    let mut tally = Tally { compiled: 0, failed: 0, shipped_same: 0, shipped_total: 0 };
    let mut failures = Vec::new();

    // The base game's script sources.
    let base = ResMan::for_game(&install).unwrap();
    let names: Vec<String> =
        base.list(ResType::NSS).iter().map(|r| r.to_lowercase().to_string()).collect();
    let src = dir.join("base-src");
    std::fs::create_dir_all(&src).unwrap();
    for n in &names {
        let data = base.get(&ResKey::parse(n, ResType::NSS).unwrap()).unwrap();
        std::fs::write(src.join(format!("{n}.nss")), data).unwrap();
    }
    let ours = compile_all(&base, &names);
    let theirs =
        oracle_compile(&tool, &root, &empty_user, &src, &dir.join("base-out"), &[], &names);
    let shipped = |n: &str| base.get(&ResKey::parse(n, ResType::NCS)?).ok().map(|d| d.into_owned());
    compare("base", &ours, &theirs, shipped, &mut tally, &mut failures);
    eprintln!("base game: {} scripts", names.len());

    // Every shipped module, over its haks and the game.
    for (i, path) in bundled_modules(&root).iter().enumerate() {
        let m = Module::open(path).unwrap();
        let names: Vec<String> =
            m.keys_of(ResType::NSS).map(|k| k.resref.to_lowercase().to_string()).collect();
        if names.is_empty() {
            continue;
        }
        let mut rm = ResMan::for_game(&install).unwrap();
        let haks = m.haks().unwrap();
        rm.add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        let src = dir.join(format!("m{i}-src"));
        std::fs::create_dir_all(&src).unwrap();
        for k in m.keys_of(ResType::NSS) {
            std::fs::write(src.join(k.to_string()), m.get(k).unwrap()).unwrap();
        }
        // nwn_script_comp stacks each --erfs entry above the previous one,
        // so the first hak (highest priority) goes last.
        let hak_files: Vec<_> =
            haks.iter().rev().map(|h| root.join("data/hk").join(format!("{h}.hak"))).collect();
        let ours = compile_all(&rm, &names);
        let theirs = oracle_compile(
            &tool,
            &root,
            &empty_user,
            &src,
            &dir.join(format!("m{i}-out")),
            &hak_files,
            &names,
        );
        let label = path.file_name().unwrap().to_string_lossy().into_owned();
        let shipped = |n: &str| m.get(&ResKey::parse(n, ResType::NCS)?).map(<[u8]>::to_vec);
        compare(&label, &ours, &theirs, shipped, &mut tally, &mut failures);
        eprintln!("{label}: {} scripts", names.len());
    }

    eprintln!(
        "compiled {} scripts identically to nwn_script_comp ({} fail to compile in both); \
         {} of {} equal the compiled scripts shipped beside them",
        tally.compiled, tally.failed, tally.shipped_same, tally.shipped_total
    );
    assert!(tally.compiled > 5000);
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}
