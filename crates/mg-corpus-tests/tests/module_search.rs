//! Advanced search (`mg_module::search`) over the shipped modules and
//! campaigns: what it finds is what a plain count of the files' fields
//! finds (every local variable, every Fighter and Wizard, every Skill
//! Bonus property, every field holding 103), each hit's path leads to
//! the field or struct it names, an item blueprint found is found again
//! where blueprints carry it, and what is found is named as the game
//! names it.

use mg_core::{Codepage, ResType};
use mg_gff::{Struct, Value};
use mg_module::Module;
use mg_module::search::{FieldValue, Hit, Query, Search, search};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::{at_least, bundled_modules, corpus, sweep};
use rayon::prelude::*;

/// Every struct of a resource with its path, the root's `""`.
fn structs<'a>(s: &'a Struct, path: String, out: &mut Vec<(String, &'a Struct)>) {
    out.push((path.clone(), s));
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        match &f.value {
            Value::Struct(c) => structs(c, format!("{path}/{label}"), out),
            Value::List(items) => {
                for (i, c) in items.iter().enumerate() {
                    structs(c, format!("{path}/{label}[{i}]"), out);
                }
            }
            _ => {}
        }
    }
}

/// The struct or field a path leads to (a field's value; a struct as a
/// struct value).
fn at(s: &Struct, path: &str) -> Option<Value> {
    let mut s = s.clone();
    let steps: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    for (n, step) in steps.iter().enumerate() {
        let last = n + 1 == steps.len();
        match step.split_once('[') {
            Some((list, rest)) => {
                let i: usize = rest.strip_suffix(']')?.parse().ok()?;
                s = s.list(list)?.get(i)?.clone();
            }
            None if last => return s.get(step).cloned(),
            None => match s.get(step)? {
                Value::Struct(c) => s = c.clone(),
                _ => return None,
            },
        }
    }
    Some(Value::Struct(s))
}

/// What one module's searches found, and what was counted.
#[derive(Default)]
struct Tally {
    variables: usize,
    fighters: usize,
    bonuses: usize,
    numbers: usize,
    carried: usize,
    spawned: usize,
}

fn check(m: &Module, game: &GameData, name: &str, fails: &mut Vec<String>) -> Tally {
    let cp = Codepage::WINDOWS_1252;
    let mut keys: Vec<ResKey> = m.keys().filter(|k| k.restype.is_gff()).copied().collect();
    keys.sort();
    // The plain count: by resource, the paths of what each search should
    // find.
    let mut want: [Vec<(ResKey, String)>; 5] = Default::default();
    for key in &keys {
        let Some(Ok(g)) = m.gff(key) else { continue };
        let mut all = Vec::new();
        structs(&g.root, String::new(), &mut all);
        for (path, s) in all {
            let list = path.rsplit('/').next().and_then(|l| l.split_once('[')).map(|(l, _)| l);
            match list {
                Some("VarTable") => want[0].push((*key, path.clone())),
                Some("ClassList") if s.integer("Class") == Some(4) => {
                    want[1].push((*key, path.clone()))
                }
                Some("ClassList") if s.integer("Class") == Some(10) => {
                    want[2].push((*key, path.clone()))
                }
                Some("PropertiesList") if s.integer("PropertyName") == Some(52) => {
                    want[3].push((*key, path.clone()))
                }
                _ => {}
            }
            for f in &s.fields {
                let n = match f.value {
                    Value::Byte(x) => Some(i64::from(x)),
                    Value::Char(x) => Some(i64::from(x)),
                    Value::Word(x) => Some(i64::from(x)),
                    Value::Short(x) => Some(i64::from(x)),
                    Value::Dword(x) => Some(i64::from(x)),
                    Value::Int(x) => Some(i64::from(x)),
                    Value::Dword64(x) => i64::try_from(x).ok(),
                    Value::Int64(x) => Some(x),
                    _ => None,
                };
                if n == Some(103) {
                    want[4].push((*key, format!("{path}/{}", f.label.to_string_lossy())));
                }
            }
        }
    }
    let searches = [
        Search::Variable { name: "*".into(), kind: None, value: None },
        Search::Class(4),
        Search::Class(10),
        Search::ItemProperty { property: 52, subtype: None, value: None, param: None },
        Search::Field { label: None, value: Some(FieldValue::Number(103)) },
    ];
    let mut tally = Tally::default();
    for (s, want) in searches.iter().zip(&want) {
        let hits = search(m, &Query::new(s.clone()), Some(game), cp);
        let (carried, direct): (Vec<&Hit>, Vec<&Hit>) =
            hits.iter().partition(|h| h.carried.is_some());
        let (spawned, direct): (Vec<&Hit>, Vec<&Hit>) =
            direct.into_iter().partition(|h| h.spawned.is_some());
        let mut got: Vec<(ResKey, String)> =
            direct.iter().map(|h| (h.key, h.path.clone())).collect();
        got.sort();
        let mut want = want.clone();
        want.sort();
        if got != want {
            fails.push(format!(
                "{name}: {s:?}: {} found, {} counted; first apart: {:?}",
                got.len(),
                want.len(),
                got.iter().zip(&want).find(|(a, b)| a != b)
            ));
        }
        // Each leads where it says, and placed ones to their object.
        for h in &direct {
            let Some(Ok(g)) = m.gff(&h.key) else { continue };
            if at(&g.root, &h.path).is_none() {
                fails.push(format!("{name}: {} {} leads nowhere", h.key, h.path));
            }
            if h.placed() && h.instance().is_none() {
                fails.push(format!("{name}: {} {} is no placed object", h.key, h.path));
            }
        }
        // What is carried is an item blueprint found.
        for h in &carried {
            let item = ResKey::new(h.carried.unwrap(), ResType::UTI);
            if !direct.iter().any(|d| d.key == item) {
                fails.push(format!("{name}: {} carries {item}, which wasn't found", h.key));
            }
        }
        match s {
            Search::Variable { .. } => tally.variables += direct.len(),
            Search::Class(4) => tally.fighters += direct.len(),
            Search::ItemProperty { .. } => tally.bonuses += direct.len(),
            Search::Field { .. } => tally.numbers += direct.len(),
            _ => {}
        }
        // What is spawned is a creature blueprint found, named where an
        // encounter's creature list names it.
        for h in &spawned {
            let creature = ResKey::new(h.spawned.unwrap(), ResType::UTC);
            if !direct.iter().any(|d| d.key == creature) {
                fails.push(format!("{name}: {} spawns {creature}, which wasn't found", h.key));
            }
            let named = m
                .gff(&h.key)
                .and_then(Result::ok)
                .and_then(|g| at(&g.root, &h.path).and_then(|v| v.as_resref()));
            if named != h.spawned {
                fails.push(format!("{name}: {} {} names {named:?}", h.key, h.path));
            }
        }
        tally.carried += carried.len();
        tally.spawned += spawned.len();
        // Named as the game names them.
        let named = |h: &&Hit| match s {
            Search::Class(4) => h.matched.starts_with("Class: Fighter, level "),
            Search::Class(_) => h.matched.starts_with("Class: Wizard, level "),
            Search::ItemProperty { .. } => h.matched.starts_with("Skill Bonus"),
            _ => true,
        };
        if let Some(h) = direct.iter().find(|h| !named(h)) {
            fails.push(format!("{name}: {} {} named {:?}", h.key, h.path, h.matched));
        }
    }
    tally
}

#[test]
fn advanced_search_finds_what_the_shipped_modules_hold() {
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let modules = sweep(bundled_modules(&root), 3);
    let results: Vec<(Tally, Vec<String>)> = modules
        .par_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let m = Module::open(path).unwrap();
            let mut fails = Vec::new();
            let tally = check(&m, &game, &name, &mut fails);
            (tally, fails)
        })
        .collect();
    let fails: Vec<&String> = results.iter().flat_map(|r| &r.1).collect();
    assert!(fails.is_empty(), "{} problems:\n{:#?}", fails.len(), &fails[..fails.len().min(20)]);
    let sum = |f: fn(&Tally) -> usize| results.iter().map(|r| f(&r.0)).sum::<usize>();
    let (variables, fighters, bonuses, numbers, carried, spawned) = (
        sum(|t| t.variables),
        sum(|t| t.fighters),
        sum(|t| t.bonuses),
        sum(|t| t.numbers),
        sum(|t| t.carried),
        sum(|t| t.spawned),
    );
    println!(
        "{} modules: {variables} variables, {fighters} fighters, {bonuses} skill bonuses, \
         {numbers} fields holding 103, {carried} found through inventories, {spawned} through \
         encounters",
        modules.len()
    );
    assert!(variables >= at_least(1000, 3), "{variables} variables");
    assert!(fighters >= at_least(300, 3), "{fighters} fighters");
    assert!(numbers >= at_least(300, 3), "{numbers} fields holding 103");
}
