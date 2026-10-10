//! The native decompiler against the game's models: every binary model
//! decompiles, and reading the ASCII back gives the same model.

#[path = "mdl_common/mod.rs"]
mod common;

use std::sync::atomic::{AtomicUsize, Ordering};

use mg_core::ResType;
use mg_mdl::Model;
use mg_resman::{GameInstall, ResKey, ResMan};
use rayon::prelude::*;

#[test]
fn every_binary_model_reads_back_the_same() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let names = rm.list(ResType::MDL);
    let binaries = AtomicUsize::new(0);
    let duplicates = std::sync::Mutex::new(Vec::new());
    let mut failures: Vec<(String, Vec<String>)> = names
        .par_iter()
        .filter_map(|name| {
            let data = rm.get(&ResKey::new(*name, ResType::MDL)).ok()?;
            if !mg_mdl::is_binary(&data) {
                return None;
            }
            binaries.fetch_add(1, Ordering::Relaxed);
            let bin = match Model::read(&data) {
                Ok(m) => m,
                // The toolset's reader is tested on these; nothing to
                // decompile.
                Err(_) => return None,
            };
            let text = mg_mdl::decompile(&data).expect("binary models decompile");
            let asc = match Model::read(text.as_bytes()) {
                Ok(m) => m,
                Err(e) => return Some((name.to_string(), vec![format!("unreadable: {e}")])),
            };
            let d = common::differences(&bin, &asc);
            // Two geometry nodes with one name cannot be told apart in
            // ASCII (parents are names): every reader, the game's
            // included, hangs the second's children on the first.
            if !d.is_empty() && common::duplicate_names(&bin) {
                duplicates.lock().unwrap().push(name.to_string());
                return None;
            }
            (!d.is_empty()).then(|| (name.to_string(), d))
        })
        .collect();
    failures.sort();
    let total = binaries.load(Ordering::Relaxed);
    let duplicates = duplicates.into_inner().unwrap();
    eprintln!(
        "{total} binary models, {} differ; {} differ through duplicate node names: {duplicates:?}",
        failures.len(),
        duplicates.len()
    );
    for (name, d) in failures.iter().take(15) {
        eprintln!("{name}: {}", d.iter().take(4).cloned().collect::<Vec<_>>().join("; "));
    }
    assert!(total > 25_000, "only {total} binary models");
    assert!(duplicates.len() <= 1, "{duplicates:?}");
    assert!(failures.is_empty(), "{} of {total} models differ", failures.len());
}

/// Decompiling is deterministic: the same bytes twice.
#[test]
fn decompiling_twice_gives_the_same_text() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    for name in ["a_ba", "plc_a01", "c_golemerald", "tcn01_a01_01"] {
        let data = rm.get(&ResKey::parse(name, ResType::MDL).unwrap()).unwrap();
        let (a, b) = (mg_mdl::decompile(&data).unwrap(), mg_mdl::decompile(&data).unwrap());
        assert!(a == b, "{name}");
    }
}

/// A decompiled model's nodes are in the order of their part numbers, so a
/// compiler numbers them again as they were: the game finds a supermodel's
/// animations by those numbers. Checked where a model's numbers are its
/// own (no supermodel).
#[test]
fn decompiled_nodes_keep_the_order_of_their_numbers() {
    let root = mg_testkit::corpus!();
    let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let names = rm.list(ResType::MDL);
    let checked = AtomicUsize::new(0);
    let reordered = AtomicUsize::new(0);
    let wrong: Vec<String> = names
        .par_iter()
        .filter_map(|name| {
            let data = rm.get(&ResKey::new(*name, ResType::MDL)).ok()?;
            let model = Model::read(&data).ok().filter(|_| mg_mdl::is_binary(&data))?;
            let parts = mg_mdl::binary_write::PartNumbers::read(&data)?;
            // Numbers of its own, each once, and names that tell nodes apart.
            let mut sorted = parts.numbers.clone();
            sorted.sort_unstable();
            // (A skin is written after its bones, whatever its number.)
            if model.supermodel.is_some()
                || sorted.iter().enumerate().any(|(i, n)| *n != i as i32)
                || common::duplicate_names(&model)
                || model.nodes.iter().any(|n| n.kind.type_name() == "skin")
            {
                return None;
            }
            checked.fetch_add(1, Ordering::Relaxed);
            if parts.numbers.iter().enumerate().any(|(i, n)| *n != i as i32) {
                reordered.fetch_add(1, Ordering::Relaxed);
            }
            // The text's nodes, in its order, are numbered 0, 1, 2…
            let text = mg_mdl::decompile(&data).ok()?;
            let geometry = text.split("endmodelgeom").next()?;
            let written: Vec<String> = geometry
                .lines()
                .filter_map(|l| {
                    let mut words = l.split_whitespace();
                    (words.next() == Some("node")).then(|| words.nth(1).map(str::to_lowercase))?
                })
                .collect();
            let by_number = |i: usize| {
                let node = parts.numbers.iter().position(|&n| n == i as i32)?;
                Some(model.nodes[node].name.to_lowercase())
            };
            let kept = written.len() == model.nodes.len()
                && written.iter().enumerate().all(|(i, n)| by_number(i).as_ref() == Some(n));
            (!kept).then(|| name.to_string())
        })
        .collect();
    eprintln!(
        "{} models with numbers of their own ({} not in their tree's order): {} lose them",
        checked.load(Ordering::Relaxed),
        reordered.load(Ordering::Relaxed),
        wrong.len()
    );
    eprintln!("{:?}", &wrong[..wrong.len().min(20)]);
    assert!(checked.load(Ordering::Relaxed) > 20_000);
    assert!(reordered.load(Ordering::Relaxed) > 3_000);
    assert!(wrong.is_empty(), "{} models", wrong.len());
}
