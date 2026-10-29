//! The native decompiler against the game's models: every binary model
//! decompiles, and reading the ASCII back gives the same model. One sweep
//! (each model decompiled once) for the tests here and the linter's
//! (`mdl_lint.rs`).

use crate::mdl_common as common;

use std::sync::OnceLock;

use mg_core::{ResRef, ResType};
use mg_mdl::Model;
use mg_mdl::lint::Severity;
use mg_resman::{GameInstall, ResKey, ResMan};
use rayon::prelude::*;

/// What the sweep found in one decompiled model.
#[derive(Default)]
struct One {
    /// How the ASCII reads differently from the binary, if it does.
    differs: Option<Vec<String>>,
    /// It differs, and has two nodes of one name.
    duplicate: bool,
    /// For a model with part numbers of its own: whether they are out of
    /// its tree's order, and whether the text keeps them.
    order: Option<(bool, bool)>,
    /// The warnings and errors the linter has for the text.
    noise: Option<String>,
}

/// The sweep's findings over the game's binary models.
pub(crate) struct Swept {
    binaries: usize,
    differ: Vec<(String, Vec<String>)>,
    duplicates: Vec<String>,
    numbered: usize,
    reordered: usize,
    unordered: Vec<String>,
    /// Decompiled models the linter has warnings or errors for.
    pub(crate) noisy: Vec<String>,
    /// Each decompiled model compiled back (`mdl_compile.rs`).
    pub(crate) compiled_back: crate::mdl_compile::CompiledBack,
}

/// Whether the text's nodes, in its order, are numbered 0, 1, 2… as the
/// compiled model numbers them; `None` where the model's numbers are not
/// its own.
fn order(data: &[u8], model: &Model, text: &str) -> Option<(bool, bool)> {
    let parts = mg_mdl::binary_write::PartNumbers::read(data)?;
    // Numbers of its own, each once, and names that tell nodes apart.
    let mut sorted = parts.numbers.clone();
    sorted.sort_unstable();
    // (A skin is written after its bones, whatever its number.)
    if model.supermodel.is_some()
        || sorted.iter().enumerate().any(|(i, n)| *n != i as i32)
        || common::duplicate_names(model)
        || model.nodes.iter().any(|n| n.kind.type_name() == "skin")
    {
        return None;
    }
    let reordered = parts.numbers.iter().enumerate().any(|(i, n)| *n != i as i32);
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
    Some((reordered, kept))
}

fn one(rm: &ResMan, back: &crate::mdl_compile::CompiledBack, name: &ResRef, data: &[u8]) -> One {
    let mut found = One::default();
    // The toolset's reader is tested on these; a model it cannot read is
    // only linted.
    let model = Model::read(data).ok();
    let text = match (mg_mdl::decompile(data), &model) {
        (Ok(text), _) => text,
        (Err(e), Some(_)) => panic!("{name}: binary models decompile: {e}"),
        (Err(_), None) => return found,
    };
    let bad: Vec<String> = mg_mdl::lint::check(&text)
        .into_iter()
        .filter(|d| d.severity > Severity::Info)
        // Duplicate node and animation names are the model's own.
        .filter(|d| !d.message.contains("second node named"))
        .filter(|d| !d.message.contains("second animation named"))
        .map(|d| format!("{}: {}", d.line + 1, d.message))
        .collect();
    found.noise =
        (!bad.is_empty()).then(|| format!("{name}: {}", bad[..bad.len().min(3)].join("; ")));
    let Some(model) = model else { return found };
    back.add(rm, name, data, &model, &text);
    found.order = order(data, &model, &text);
    let d = match Model::read(text.as_bytes()) {
        Ok(asc) => common::differences(&model, &asc),
        Err(e) => vec![format!("unreadable: {e}")],
    };
    // Two geometry nodes with one name cannot be told apart in ASCII
    // (parents are names): every reader, the game's included, hangs the
    // second's children on the first.
    if !d.is_empty() && !d[0].starts_with("unreadable") && common::duplicate_names(&model) {
        found.duplicate = true;
    } else if !d.is_empty() {
        found.differs = Some(d);
    }
    found
}

/// Decompiles the game's binary models (a tenth of them unless the run is
/// full), once for all the tests that ask.
pub(crate) fn swept(root: &std::path::Path) -> &'static Swept {
    static SWEPT: OnceLock<Swept> = OnceLock::new();
    SWEPT.get_or_init(|| {
        let rm = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
        let names = mg_testkit::sweep(rm.list(ResType::MDL), 10);
        let back = crate::mdl_compile::CompiledBack::default();
        let all: Vec<(String, One)> = names
            .par_iter()
            .filter_map(|name| {
                let data = rm.get(&ResKey::new(*name, ResType::MDL)).ok()?;
                mg_mdl::is_binary(&data).then(|| (name.to_string(), one(&rm, &back, name, &data)))
            })
            .collect();
        let mut s = Swept {
            binaries: all.len(),
            differ: Vec::new(),
            duplicates: Vec::new(),
            numbered: 0,
            reordered: 0,
            unordered: Vec::new(),
            noisy: Vec::new(),
            compiled_back: back,
        };
        for (name, found) in all {
            if let Some(d) = found.differs {
                s.differ.push((name.clone(), d));
            }
            if found.duplicate {
                s.duplicates.push(name.clone());
            }
            if let Some((reordered, kept)) = found.order {
                s.numbered += 1;
                s.reordered += usize::from(reordered);
                if !kept {
                    s.unordered.push(name.clone());
                }
            }
            s.noisy.extend(found.noise);
        }
        s.differ.sort();
        s
    })
}

#[test]
fn every_binary_model_reads_back_the_same() {
    let root = mg_testkit::corpus!();
    let s = swept(&root);
    eprintln!(
        "{} binary models, {} differ; {} differ through duplicate node names: {:?}",
        s.binaries,
        s.differ.len(),
        s.duplicates.len(),
        s.duplicates
    );
    for (name, d) in s.differ.iter().take(15) {
        eprintln!("{name}: {}", d.iter().take(4).cloned().collect::<Vec<_>>().join("; "));
    }
    assert!(s.binaries > mg_testkit::at_least(25_000, 10), "only {} binary models", s.binaries);
    assert!(s.duplicates.len() <= 1, "{:?}", s.duplicates);
    assert!(s.differ.is_empty(), "{} of {} models differ", s.differ.len(), s.binaries);
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
    let s = swept(&root);
    eprintln!(
        "{} models with numbers of their own ({} not in their tree's order): {} lose them",
        s.numbered,
        s.reordered,
        s.unordered.len()
    );
    eprintln!("{:?}", &s.unordered[..s.unordered.len().min(20)]);
    assert!(s.numbered > mg_testkit::at_least(20_000, 10));
    assert!(s.reordered > mg_testkit::at_least(3_000, 10));
    assert!(s.unordered.is_empty(), "{} models", s.unordered.len());
}
