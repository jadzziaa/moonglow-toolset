//! The editor's NWScript front end against the compiler, over every script
//! the game and the shipped modules contain:
//! - the lexer covers every byte of every source;
//! - for every script that compiles, the outline of the script and its
//!   includes lists exactly the functions (with return and parameter types)
//!   and structs the compiler's debug output (NDB) reports;
//! - and the analysis' locals agree with it (`script_analysis.rs`), from
//!   the same compile.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use std::sync::Arc;

use mg_core::ResType;
use mg_resman::{ResKey, ResMan};
use mg_script::Compiler;
use mg_script::analysis::Index;
use mg_script::compiler::optimize;
use mg_script::lex::{TokenKind, tokenize};
use mg_script::outline::{Outline, outline};
use mg_script::spec::Spec;
use mg_testkit::corpus;

use crate::script_analysis::check_locals;
use crate::script_common::{each_script_set, on_every_core};

/// Functions and structs as NDB describes them: name → `ret(param,...)` in
/// NDB type letters with structs resolved to names.
#[derive(Debug, Default, PartialEq, Eq)]
struct Shape {
    functions: BTreeMap<String, String>,
    structs: BTreeMap<String, Vec<(String, String)>>,
}

fn parse_ndb(ndb: &[u8]) -> Shape {
    let text = String::from_utf8_lossy(ndb);
    let mut struct_names: Vec<String> = Vec::new();
    let mut structs: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut funcs: Vec<(String, String, Vec<String>)> = Vec::new();
    for line in text.lines() {
        let w: Vec<&str> = line.split(' ').collect();
        match w.as_slice() {
            ["s", _, name] => {
                struct_names.push(name.to_string());
                structs.push((name.to_string(), Vec::new()));
            }
            ["sf", ty, name] => {
                structs.last_mut().unwrap().1.push((ty.to_string(), name.to_string()))
            }
            ["f", _, _, _, ret, name] => {
                funcs.push((name.to_string(), ret.to_string(), Vec::new()))
            }
            ["fp", ty] => funcs.last_mut().unwrap().2.push(ty.to_string()),
            _ => {}
        }
    }
    let ty = |t: &str| match t.strip_prefix('t') {
        Some(n) => format!("struct {}", struct_names[n.parse::<usize>().unwrap()]),
        None => t.to_string(),
    };
    Shape {
        functions: funcs
            .into_iter()
            .filter(|(n, _, _)| !n.starts_with('#'))
            .map(|(n, r, ps)| {
                (
                    n,
                    format!(
                        "{}({})",
                        ty(&r),
                        ps.iter().map(|p| ty(p)).collect::<Vec<_>>().join(",")
                    ),
                )
            })
            .collect(),
        structs: structs
            .into_iter()
            .filter(|(n, _)| n != "vector")
            .map(|(n, fields)| (n, fields.into_iter().map(|(t, f)| (ty(&t), f)).collect()))
            .collect(),
    }
}

/// The same shape from outlines, with types in NDB letters.
fn shape_of(outlines: &[&Outline], spec: &Spec) -> Shape {
    let ty = |t: &str| -> String {
        match t {
            "int" => "i".into(),
            "float" => "f".into(),
            "string" => "s".into(),
            "object" => "o".into(),
            "void" => "v".into(),
            "vector" => "struct vector".into(),
            t if t.starts_with("struct ") => t.to_string(),
            t => match spec.engine_structures.iter().position(|e| e == t) {
                Some(i) => format!("e{i}"),
                None => format!("?{t}"),
            },
        }
    };
    let mut s = Shape::default();
    for o in outlines {
        // Prototypes count too: the NDB lists declared-only functions. The
        // first of a file's declarations gives the return type: one defined
        // with another (x2_inc_cutscene's CutCreateObjectCopy) keeps the
        // one met first, as Aurora compiles it (the return-type-mismatch
        // patch, `crates/mg-script/nwscript/README.md`).
        let mut own = std::collections::BTreeMap::new();
        for f in &o.functions {
            let params: Vec<String> = f.params.iter().map(|p| ty(&p.ty)).collect();
            (own.entry(f.name.clone()))
                .or_insert_with(|| format!("{}({})", ty(&f.return_type), params.join(",")));
        }
        s.functions.extend(own);
        // The compiler implements global constants as functions returning
        // the value, and lists them so in the NDB.
        for g in o.globals.iter().filter(|g| g.is_const) {
            s.functions.insert(g.name.clone(), format!("{}()", ty(&g.ty)));
        }
        for st in &o.structs {
            s.structs.insert(
                st.name.clone(),
                st.members.iter().map(|m| (ty(&m.ty), m.name.clone())).collect(),
            );
        }
    }
    s
}

struct Results {
    scripts: usize,
    compared: usize,
    locals: usize,
    lex_unknown: usize,
    failures: Vec<String>,
}

fn check_scripts(label: &str, rm: &ResMan, names: &[String], spec: &Spec, results: &mut Results) {
    let partial: Vec<(usize, usize, usize, Vec<String>)> = on_every_core(names, |part| {
        let source = |name: &str| {
            ResKey::parse(name, ResType::NSS).and_then(|k| rm.get(&k).ok()).map(|d| d.into_owned())
        };
        let mut c = Compiler::new(|name, t| {
            ResKey::parse(name, t).and_then(|k| rm.get(&k).ok()).map(|d| d.into_owned())
        });
        c.set_optimization(optimize::NOTHING);
        c.set_debug_output(true);
        let mut cache: HashMap<String, Outline> = HashMap::new();
        let mut sources = |name: &str| -> Option<Arc<str>> {
            Some(Arc::from(String::from_utf8_lossy(&source(name)?).as_ref()))
        };
        let mut index = Index::new();
        let (mut compared, mut locals, mut unknown) = (0, 0, 0);
        let mut fails = Vec::new();
        for name in part {
            let Some(src) = source(name) else { continue };
            // The lexer covers the source exactly.
            let toks = tokenize(&src);
            let covered: usize = toks.iter().map(|t| t.span.len()).sum();
            if covered != src.len() || toks.windows(2).any(|w| w[0].span.end != w[1].span.start) {
                fails.push(format!("{label}: {name}.nss: tokens do not cover the source"));
            }
            unknown += toks.iter().filter(|t| t.kind == TokenKind::Unknown).count();

            let Ok(out) = c.compile(name) else { continue };
            let Some(ndb) = out.ndb else { continue };
            locals += check_locals(label, name, &ndb, &mut index, &mut sources, &mut fails);
            // The script's include closure (each file once).
            let mut order: Vec<String> = Vec::new();
            let mut stack = vec![name.clone()];
            let mut seen = BTreeSet::new();
            while let Some(n) = stack.pop() {
                let n = n.to_ascii_lowercase();
                if !seen.insert(n.clone()) {
                    continue;
                }
                if !cache.contains_key(&n) {
                    let Some(src) = source(&n) else { continue };
                    cache.insert(n.clone(), outline(&src));
                }
                stack.extend(cache[&n].includes.iter().map(|i| i.name.clone()));
                order.push(n);
            }
            let outlines: Vec<&Outline> = order.iter().filter_map(|n| cache.get(n)).collect();
            let ours = shape_of(&outlines, spec);
            let theirs = parse_ndb(&ndb);
            compared += 1;
            if ours != theirs {
                let fa: Vec<_> = ours
                    .functions
                    .iter()
                    .filter(|(k, v)| theirs.functions.get(*k) != Some(v))
                    .take(3)
                    .collect();
                let fb: Vec<_> = theirs
                    .functions
                    .iter()
                    .filter(|(k, v)| ours.functions.get(*k) != Some(v))
                    .take(3)
                    .collect();
                let sa: Vec<_> = ours
                    .structs
                    .iter()
                    .filter(|(k, v)| theirs.structs.get(*k) != Some(v))
                    .take(2)
                    .collect();
                let sb: Vec<_> = theirs
                    .structs
                    .iter()
                    .filter(|(k, v)| ours.structs.get(*k) != Some(v))
                    .take(2)
                    .collect();
                fails.push(format!(
                    "{label}: {name}.nss: outline {fa:?} {sa:?} vs NDB {fb:?} {sb:?}"
                ));
            }
        }
        (compared, locals, unknown, fails)
    });
    results.scripts += names.len();
    for (c, l, u, f) in partial {
        results.compared += c;
        results.locals += l;
        results.lex_unknown += u;
        results.failures.extend(f);
    }
}

#[test]
fn front_end_agrees_with_the_compiler() {
    let root = corpus!();
    let mut spec = None;
    let mut results =
        Results { scripts: 0, compared: 0, locals: 0, lex_unknown: 0, failures: Vec::new() };
    each_script_set(&root, |set| {
        // (The base game's set comes first: nwscript.nss is in it.)
        let spec = spec.get_or_insert_with(|| {
            Spec::parse(&set.rm.get(&ResKey::parse("nwscript", ResType::NSS).unwrap()).unwrap())
        });
        check_scripts(&set.label, set.rm, set.names, spec, &mut results);
    });
    eprintln!(
        "lexed {} scripts ({} unknown characters); outlines of {} compiled scripts and the \
         locals of {} functions compared with NDB",
        results.scripts, results.lex_unknown, results.compared, results.locals
    );
    if mg_testkit::full_corpus() {
        assert!(results.compared > 20_000);
    }
    assert!(results.locals > results.compared);
    let f = &results.failures;
    let report = mg_testkit::scratch_dir("script_frontend").join("failures.txt");
    std::fs::write(&report, f.join("\n")).unwrap();
    assert!(
        f.is_empty(),
        "{} failures:\n{}",
        f.len(),
        f.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
    );
}
