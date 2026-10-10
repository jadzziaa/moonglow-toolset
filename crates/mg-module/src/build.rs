//! Building a module: compiling its scripts (Aurora's Build › Compile).

use std::sync::Mutex;

use mg_core::ResType;
use mg_resman::{ResKey, ResMan};
use mg_script::{CompileError, Compiler};

use crate::Module;
use crate::external_compiler::ExternalCompiler;

/// The result of compiling one script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptResult {
    pub script: ResKey,
    pub result: Result<(), CompileError>,
}

/// A script and its bytecode (empty for include files) or error.
type Compiled = (ResKey, Result<Vec<u8>, CompileError>);

/// Which scripts to compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptSelection {
    /// Every script source in the module.
    All,
    /// Sources that have no compiled script in the module yet.
    Uncompiled,
}

/// Compiles the module's scripts on all cores, storing each `.ncs` in the
/// module. Sources are looked up in the module first, then in `resman` (its
/// haks and the game), as the game would. Scripts without an entry point
/// (include files) are skipped, as Aurora skips them: they produce nothing
/// and are not errors.
pub fn compile_scripts(
    module: &mut Module,
    resman: &ResMan,
    selection: ScriptSelection,
) -> Vec<ScriptResult> {
    compile_scripts_with(module, resman, selection, None)
}

/// [`compile_scripts`], by an external compiler when one is given (Options
/// › Script Editor) in place of the built-in one.
pub fn compile_scripts_with(
    module: &mut Module,
    resman: &ResMan,
    selection: ScriptSelection,
    external: Option<&ExternalCompiler>,
) -> Vec<ScriptResult> {
    // A nasher project's target may leave some scripts uncompiled.
    let skipped = |k: &ResKey| {
        module.project.as_ref().is_some_and(|p| p.target().skips_compiling(&k.to_string()))
    };
    let names: Vec<ResKey> = module
        .keys_of(ResType::NSS)
        .filter(|k| match selection {
            ScriptSelection::All => true,
            ScriptSelection::Uncompiled => !module.contains(&ResKey::new(k.resref, ResType::NCS)),
        })
        .filter(|k| !skipped(k))
        .copied()
        .collect();
    compile_named_scripts(module, resman, &names, external)
}

/// What a script compiles from, as one number: its source and the sources
/// it includes, however indirectly (those of the module: an include found
/// only in the haks or the game counts by its name). The same number, the
/// same compiled script, with the same compiler and game data.
pub fn script_stamps(module: &Module) -> std::collections::HashMap<ResKey, u64> {
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    fn stamp(
        key: ResKey,
        module: &Module,
        done: &mut HashMap<ResKey, u64>,
        under: &mut Vec<ResKey>,
    ) -> u64 {
        if let Some(s) = done.get(&key) {
            return *s;
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        let source = module.get(&key).unwrap_or_default();
        source.hash(&mut h);
        // (A script that includes itself, at whatever remove, ends here.)
        if !under.contains(&key) {
            under.push(key);
            for r in crate::refs::script_includes(key, source) {
                let name = r.target.to_lowercase();
                name.to_string().hash(&mut h);
                let include = ResKey::new(name, ResType::NSS);
                if module.contains(&include) {
                    stamp(include, module, done, under).hash(&mut h);
                }
            }
            under.pop();
        }
        let s = h.finish();
        if under.is_empty() {
            done.insert(key, s);
        }
        s
    }
    let mut done = HashMap::new();
    for key in module.keys_of(ResType::NSS) {
        stamp(*key, module, &mut done, &mut Vec::new());
    }
    done
}

/// [`compile_scripts_with`] for the scripts named.
pub fn compile_named_scripts(
    module: &mut Module,
    resman: &ResMan,
    names: &[ResKey],
    external: Option<&ExternalCompiler>,
) -> Vec<ScriptResult> {
    if names.is_empty() {
        return Vec::new();
    }
    if let Some(external) = external {
        return compile_externally(module, external, names);
    }
    let outputs: Mutex<Vec<Compiled>> = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(names.len());
    let chunk = names.len().div_ceil(threads);
    let snapshot: &Module = module;
    std::thread::scope(|s| {
        for part in names.chunks(chunk) {
            let outputs = &outputs;
            s.spawn(move || {
                let resolve = |name: &str, t: ResType| {
                    let k = ResKey::parse(name, t)?;
                    snapshot
                        .get(&k)
                        .map(<[u8]>::to_vec)
                        .or_else(|| resman.get(&k).ok().map(|d| d.into_owned()))
                };
                let mut c = Compiler::new(resolve);
                let mut local = Vec::new();
                for k in part {
                    let name = k.resref.to_lowercase().to_string();
                    let result = match c.compile(&name) {
                        Ok(out) => Ok(out.ncs),
                        // An include file (no `main`, no
                        // `StartingConditional`): nothing to compile or to
                        // store, as Aurora skips it. It is checked where it
                        // is included: on its own it may lean on what its
                        // includer brings.
                        Err(_)
                            if snapshot
                                .get(k)
                                .is_some_and(|src| !mg_script::outline::has_entry_point(src)) =>
                        {
                            Ok(Vec::new())
                        }
                        Err(e) => Err(e),
                    };
                    local.push((*k, result));
                }
                outputs.lock().expect("no panics while holding the lock").extend(local);
            });
        }
    });
    let mut results = outputs.into_inner().expect("threads joined");
    // Report in module order.
    let order: std::collections::HashMap<ResKey, usize> =
        names.iter().enumerate().map(|(i, k)| (*k, i)).collect();
    results.sort_by_key(|(k, _)| order[k]);
    results
        .into_iter()
        .map(|(k, r)| {
            let result = r.map(|ncs| {
                if !ncs.is_empty() {
                    module.set(ResKey::new(k.resref, ResType::NCS), ncs);
                }
            });
            ScriptResult { script: k, result }
        })
        .collect()
}

/// Compiles `names` with an external compiler, storing what it made in the
/// module. A script it made nothing of is an error with what it said,
/// unless it is an include file. A compiler that can't be run is said
/// once, of the first script.
fn compile_externally(
    module: &mut Module,
    external: &ExternalCompiler,
    names: &[ResKey],
) -> Vec<ScriptResult> {
    let name = |k: &ResKey| k.resref.to_lowercase().to_string();
    let made = {
        let sources: Vec<(String, &[u8])> =
            module.keys_of(ResType::NSS).filter_map(|k| Some((name(k), module.get(k)?))).collect();
        external.compile(&sources, &names.iter().map(name).collect::<Vec<_>>())
    };
    let made = match made {
        Ok(made) => made,
        Err(e) => {
            let message = format!("External compiler: {e}. Nothing was compiled");
            let result = Err(CompileError { code: -1, message });
            return vec![ScriptResult { script: names[0], result }];
        }
    };
    names
        .iter()
        .zip(made)
        .map(|(k, outcome)| {
            let include =
                || module.get(k).is_some_and(|src| !mg_script::outline::has_entry_point(src));
            let result = match outcome.ncs {
                Some(ncs) => {
                    module.set(ResKey::new(k.resref, ResType::NCS), ncs);
                    if let Some(ndb) = outcome.ndb {
                        module.set(ResKey::new(k.resref, ResType::NDB), ndb);
                    }
                    Ok(())
                }
                None if include() => Ok(()),
                None => Err(CompileError { code: -1, message: outcome.message }),
            };
            ScriptResult { script: *k, result }
        })
        .collect()
}

/// Build › Compile › Encounters: every encounter's creature list (the
/// module's encounter blueprints and the encounters placed in its areas)
/// given the challenge rating and appearance of the creature blueprints it
/// names (`creature` reads one: the module's, else the game's); an entry
/// whose creature does not exist is removed, as Aurora does. Returns how
/// many creature entries changed.
pub fn compile_encounters(
    module: &mut Module,
    creature: &dyn Fn(mg_core::ResRef) -> Option<mg_gff::Struct>,
) -> usize {
    use mg_gff::{Gff, Struct, Value};
    let refresh = |list: &mut Vec<Struct>| -> usize {
        let before = list.len();
        list.retain(|e| e.resref("ResRef").and_then(creature).is_some());
        let mut changed = before - list.len();
        for entry in list.iter_mut() {
            let Some(bp) = entry.resref("ResRef").and_then(creature) else { continue };
            let mut touched = false;
            if let Some(cr) = bp.float("ChallengeRating")
                && entry.float("CR") != Some(cr)
            {
                entry.set("CR", Value::Float(cr));
                touched = true;
            }
            if let Some(Value::Word(a)) = bp.get("Appearance_Type").cloned()
                && entry.integer("Appearance") != Some(i64::from(a))
            {
                entry.set("Appearance", Value::Int(i32::from(a)));
                touched = true;
            }
            changed += usize::from(touched);
        }
        changed
    };
    let mut changed = 0;
    let keys: Vec<mg_resman::ResKey> = module
        .keys()
        .filter(|k| matches!(k.restype, ResType::UTE | ResType::GIT))
        .copied()
        .collect();
    for key in keys {
        let Some(mut gff) = module.get(&key).and_then(|d| Gff::read(d).ok()) else { continue };
        let before = changed;
        if key.restype == ResType::UTE {
            if let Some(list) = gff.root.list_mut("CreatureList") {
                changed += refresh(list);
            }
        } else if let Some(encounters) = gff.root.list_mut("Encounter List") {
            for e in encounters {
                if let Some(list) = e.list_mut("CreatureList") {
                    changed += refresh(list);
                }
            }
        }
        if changed != before
            && let Ok(data) = gff.to_bytes()
        {
            module.set(key, data);
        }
    }
    changed
}

/// Build › Compile › Creature CR: every creature blueprint's
/// `ChallengeRating` recalculated as Aurora does
/// ([`GameData::challenge`](mg_rules::GameData::challenge)), its gear read
/// with `item` (the module's item blueprints, else the game's). Returns how
/// many changed.
pub fn compile_creature_cr(
    module: &mut Module,
    game: &mg_rules::GameData,
    item: &dyn Fn(mg_core::ResRef) -> Option<mg_gff::Struct>,
) -> usize {
    use mg_gff::{Gff, Value};
    let keys: Vec<ResKey> = module.keys_of(ResType::UTC).copied().collect();
    let mut changed = 0;
    for key in keys {
        let Some(mut gff) = module.get(&key).and_then(|d| Gff::read(d).ok()) else { continue };
        let mut sheet = mg_rules::CreatureSheet::from_gff(&gff.root);
        sheet.gear_value = game.gear_value(&gff.root, item);
        let rating = game.challenge(&sheet).rating;
        if gff.root.float("ChallengeRating") == Some(rating) {
            continue;
        }
        gff.root.set("ChallengeRating", Value::Float(rating));
        if let Ok(data) = gff.to_bytes() {
            module.set(key, data);
            changed += 1;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use mg_core::ResRef;
    use mg_resman::{LayerClass, MemContainer, priority};

    use super::*;

    fn key(s: &str, t: ResType) -> ResKey {
        ResKey::new(ResRef::from_str(s).unwrap(), t)
    }

    /// A script's stamp changes with its own text and with the text of
    /// what it includes, however indirectly; another script's does not.
    /// Scripts that include each other have stamps all the same.
    #[test]
    fn a_script_s_stamp_follows_what_it_includes() {
        let mut m = Module::new();
        let nss = |n: &str| key(n, ResType::NSS);
        m.set(nss("inc_a"), b"int A() { return 1; }\n".to_vec());
        m.set(nss("inc_b"), b"#include \"inc_a\"\nint B() { return A(); }\n".to_vec());
        m.set(nss("uses"), b"#include \"inc_b\"\nvoid main() { B(); }\n".to_vec());
        m.set(nss("alone"), b"#include \"nw_i0_generic\"\nvoid main() {}\n".to_vec());
        m.set(nss("loop_a"), b"#include \"loop_b\"\n".to_vec());
        m.set(nss("loop_b"), b"#include \"loop_a\"\n".to_vec());
        let before = script_stamps(&m);
        assert_eq!(before.len(), 6);
        assert_eq!(script_stamps(&m), before, "the same module, the same stamps");
        m.set(nss("inc_a"), b"int A() { return 2; }\n".to_vec());
        let after = script_stamps(&m);
        for changed in ["inc_a", "inc_b", "uses"] {
            assert_ne!(after[&nss(changed)], before[&nss(changed)], "{changed}");
        }
        for same in ["alone", "loop_a", "loop_b"] {
            assert_eq!(after[&nss(same)], before[&nss(same)], "{same}");
        }
    }

    #[test]
    fn compiles_module_scripts_and_skips_includes() {
        let mut base = MemContainer::new();
        base.insert(
            key("nwscript", ResType::NSS),
            &b"#define ENGINE_NUM_STRUCTURES 0\nvoid PrintString(string s);\n"[..],
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, base);
        let mut m = Module::new();
        m.set(key("inc_greet", ResType::NSS), b"string Hi() { return \"hi\"; }\n".to_vec());
        m.set(
            key("say", ResType::NSS),
            b"#include \"inc_greet\"\nvoid main() { PrintString(Hi()); }\n".to_vec(),
        );
        m.set(key("broken", ResType::NSS), b"void main() { Nope(); }\n".to_vec());
        // An include that leans on what its includer brings (here `Hi`),
        // with a `main` commented out at its foot: not compiled on its
        // own, as Aurora skips a script without an entry point.
        m.set(
            key("inc_leans", ResType::NSS),
            b"string Twice() { return Hi() + Hi(); }\n// void main() {}\n".to_vec(),
        );

        let results = compile_scripts(&mut m, &rm, ScriptSelection::All);
        let summary: Vec<(String, bool)> =
            results.iter().map(|r| (r.script.to_string(), r.result.is_ok())).collect();
        assert_eq!(
            summary,
            [
                ("inc_greet.nss".into(), true),
                ("say.nss".into(), true),
                ("broken.nss".into(), false),
                ("inc_leans.nss".into(), true)
            ]
        );
        assert!(m.contains(&key("say", ResType::NCS)));
        assert!(!m.contains(&key("inc_greet", ResType::NCS)), "includes produce no bytecode");
        assert!(results[2].result.as_ref().unwrap_err().message.contains("UNDEFINED IDENTIFIER"));

        // Only what has no compiled script yet.
        let again = compile_scripts(&mut m, &rm, ScriptSelection::Uncompiled);
        assert_eq!(again.len(), 3, "the includes and the broken script");
    }

    /// A long chain of `else if` (a generated dispatch script, as
    /// persistent worlds have) is deep for the compiler, which walks it
    /// by recursion: it compiles on a thread with the room for it.
    #[test]
    fn a_deep_script_compiles_on_a_worker_thread() {
        let mut base = MemContainer::new();
        base.insert(
            key("nwscript", ResType::NSS),
            &b"#define ENGINE_NUM_STRUCTURES 0\nvoid PrintString(string s);\n"[..],
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, base);
        let mut m = Module::new();
        let mut src =
            String::from("void main() { int n = 3;\nif (n == 0) { PrintString(\"0\"); }\n");
        for i in 1..DEEP {
            src.push_str(&format!("else if (n == {i}) {{ PrintString(\"{i}\"); }}\n"));
        }
        src.push_str("}\n");
        m.set(key("deep", ResType::NSS), src.into_bytes());
        let results = compile_scripts(&mut m, &rm, ScriptSelection::All);
        assert!(results[0].result.is_ok(), "{:?}", results[0].result);
        assert!(m.contains(&key("deep", ResType::NCS)));
    }

    const DEEP: usize = 60000;
}
