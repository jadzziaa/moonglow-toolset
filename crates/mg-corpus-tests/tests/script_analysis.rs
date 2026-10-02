//! The script analysis' locals and parameters against the compiler: for
//! every compiled function of every script the game and the shipped modules
//! contain, the names the compiler's debug output (NDB) gives its variables
//! are the parameters and locals the analysis finds in its body.

use std::collections::BTreeMap;
use std::sync::Arc;

use mg_core::ResType;
use mg_module::Module;
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_script::Compiler;
use mg_script::analysis::Index;
use mg_script::compiler::optimize;
use mg_testkit::{bundled_modules, corpus};

/// Each compiled function's variable names (parameters and locals, sorted),
/// from the NDB: variables belong to the function whose code holds their
/// first instruction.
fn ndb_locals(ndb: &[u8]) -> BTreeMap<String, Vec<String>> {
    let text = String::from_utf8_lossy(ndb);
    let hex = |s: &str| u32::from_str_radix(s, 16).unwrap_or(u32::MAX);
    let mut funcs: Vec<(u32, u32, String)> = Vec::new();
    let mut vars: Vec<(u32, String)> = Vec::new();
    for line in text.lines() {
        let w: Vec<&str> = line.split(' ').collect();
        match w.as_slice() {
            ["f", start, end, _, _, name] if *start != "ffffffff" => {
                funcs.push((hex(start), hex(end), name.to_string()))
            }
            ["v", start, end, _, _, name] if *end != "ffffffff" && !name.starts_with('#') => {
                vars.push((hex(start), name.to_string()))
            }
            _ => {}
        }
    }
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (start, end, name) in &funcs {
        if name.starts_with('#') {
            continue;
        }
        let mut names: Vec<String> =
            vars.iter().filter(|(s, _)| s >= start && s < end).map(|(_, n)| n.clone()).collect();
        names.sort();
        out.insert(name.clone(), names);
    }
    out
}

fn check(label: &str, rm: &ResMan, names: &[String]) -> (usize, Vec<String>) {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = names.len().div_ceil(threads).max(1);
    let parts: Vec<(usize, Vec<String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = names
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    let mut c = Compiler::new(|name, t| {
                        ResKey::parse(name, t).and_then(|k| rm.get(&k).ok()).map(|d| d.into_owned())
                    });
                    c.set_optimization(optimize::NOTHING);
                    c.set_debug_output(true);
                    let mut sources = |name: &str| -> Option<Arc<str>> {
                        let d = ResKey::parse(name, ResType::NSS).and_then(|k| rm.get(&k).ok())?;
                        Some(Arc::from(String::from_utf8_lossy(&d).as_ref()))
                    };
                    let mut index = Index::new();
                    let (mut compared, mut fails) = (0, Vec::new());
                    for name in part {
                        let Ok(out) = c.compile(name) else { continue };
                        let Some(ndb) = out.ndb else { continue };
                        for (func, theirs) in ndb_locals(&ndb) {
                            let Some(ours) = index.function_locals(name, &func, &mut sources)
                            else {
                                fails.push(format!("{label}: {name}.nss: {func} not found"));
                                continue;
                            };
                            let mut ours: Vec<String> = ours.into_iter().map(|(n, _)| n).collect();
                            ours.sort();
                            compared += 1;
                            if ours != theirs {
                                fails.push(format!(
                                    "{label}: {name}.nss: {func}: ours {ours:?}, NDB {theirs:?}"
                                ));
                            }
                        }
                    }
                    (compared, fails)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let compared = parts.iter().map(|p| p.0).sum();
    (compared, parts.into_iter().flat_map(|p| p.1).collect())
}

#[test]
fn locals_agree_with_the_compiler() {
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let base = ResMan::for_game(&install).unwrap();
    let names: Vec<String> =
        base.list(ResType::NSS).iter().map(|r| r.to_lowercase().to_string()).collect();
    let (mut compared, mut fails) = check("base", &base, &names);
    for path in bundled_modules(&root) {
        let m = Module::open(&path).unwrap();
        let names: Vec<String> =
            m.keys_of(ResType::NSS).map(|k| k.resref.to_lowercase().to_string()).collect();
        let mut rm = ResMan::for_game(&install).unwrap();
        let haks = m.haks().unwrap();
        rm.add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        let label = path.file_name().unwrap().to_string_lossy().into_owned();
        let (c, f) = check(&label, &rm, &names);
        compared += c;
        fails.extend(f);
    }
    eprintln!("{compared} compiled functions' locals compared with NDB; {} differ", fails.len());
    assert!(fails.is_empty(), "{}", fails.iter().take(25).cloned().collect::<Vec<_>>().join("\n"));
}
