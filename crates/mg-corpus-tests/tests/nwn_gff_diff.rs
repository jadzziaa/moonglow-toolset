//! Differential tests against neverwinter.nim's `nwn_gff`: for a sample of
//! shipped GFFs of every type, (a) its JSON of the file must equal ours, and
//! (b) the GFF it writes from our JSON must read back as the same tree.

use std::path::Path;
use std::process::Command;

use mg_core::{Codepage, ResType};
use mg_erf::Erf;
use mg_gff::{Gff, Struct, Value, from_json, to_json};
use mg_key::KeySet;
use mg_testkit::{corpus, oracle_tool, scratch_dir};
use rayon::prelude::*;

const PER_TYPE: usize = 25;

/// Sorts fields by label and localized-string variants by key: nwn_gff keeps
/// both in hash tables, so its order is arbitrary (and the game looks fields
/// up by label, so order carries no meaning).
fn normalize(s: &mut Struct) {
    s.fields.sort_by(|a, b| a.label.as_bytes().cmp(b.label.as_bytes()));
    for f in &mut s.fields {
        match &mut f.value {
            Value::LocString(ls) => ls.strings.sort_by_key(|(k, _)| *k),
            Value::Struct(c) => normalize(c),
            Value::List(items) => items.iter_mut().for_each(normalize),
            _ => {}
        }
    }
}

fn has_empty_label(s: &Struct) -> bool {
    s.fields.iter().any(|f| {
        f.label.as_bytes().is_empty()
            || match &f.value {
                Value::Struct(c) => has_empty_label(c),
                Value::List(items) => items.iter().any(has_empty_label),
                _ => false,
            }
    })
}

fn normalized(mut g: Gff) -> Gff {
    normalize(&mut g.root);
    g
}

/// Up to `PER_TYPE` evenly spaced items of each resource type.
fn sample<T>(items: Vec<(ResType, T)>) -> Vec<(ResType, T)> {
    let mut by_type: std::collections::BTreeMap<u16, Vec<T>> = Default::default();
    for (t, v) in items {
        by_type.entry(t.0).or_default().push(v);
    }
    let mut out = Vec::new();
    for (t, v) in by_type {
        let step = v.len().div_ceil(PER_TYPE).max(1);
        out.extend(v.into_iter().step_by(step).map(|x| (ResType(t), x)));
    }
    out
}

fn run(tool: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new(tool).args(args).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    Ok(out.stdout)
}

#[test]
fn gff_agrees_with_nwn_gff() {
    let root = corpus!();
    let tool = oracle_tool!("nwn_gff");
    let dir = scratch_dir("nwn_gff_diff");
    let cp = Codepage::WINDOWS_1252;

    let mut files: Vec<(ResType, (String, Vec<u8>))> = Vec::new();
    let ks = KeySet::open(&root.join("data/nwn_base.key"), &root).unwrap();
    for e in ks.table.entries.iter().filter(|e| e.restype.is_gff()) {
        files.push((e.restype, (format!("base:{}", e.resref), ks.data(e).unwrap().to_vec())));
    }
    for m in ["data/nwm/Prelude.nwm", "data/nwm/Chapter1.nwm"] {
        let Ok(data) = std::fs::read(root.join(m)) else { continue };
        let erf = Erf::read(&data).unwrap();
        for e in erf.entries.iter().filter(|e| e.restype.is_gff()) {
            files.push((
                e.restype,
                (format!("{m}:{}", e.resref), erf.data(e).unwrap().into_owned()),
            ));
        }
    }
    let files = sample(files);
    assert!(files.len() > 100);

    let failures: Vec<String> = files
        .par_iter()
        .enumerate()
        .filter_map(|(i, (ty, (name, bytes)))| {
            let fail = |m: String| Some(format!("{name}.{ty}: {m}"));
            let ours = normalized(Gff::read(bytes).unwrap());

            // (a) nwn_gff's JSON of the original file.
            let input = dir.join(format!("{i}.{ty}"));
            std::fs::write(&input, bytes).unwrap();
            let json = match run(&tool, &["-i", input.to_str().unwrap(), "-l", "gff", "-k", "json"])
            {
                Ok(j) => j,
                Err(e) => return fail(format!("nwn_gff failed to read: {e}")),
            };
            let json: serde_json::Value = serde_json::from_slice(&json).unwrap();
            match from_json(&json, cp) {
                Err(e) => return fail(format!("cannot read nwn_gff JSON: {e}")),
                Ok(theirs) if normalized(theirs.clone()) != ours => {
                    let d = mg_gff::diff(&ours, &normalized(theirs), 5);
                    return fail(format!(
                        "nwn_gff's JSON describes a different tree:\n  {}",
                        d.join("\n  ")
                    ));
                }
                Ok(_) => {}
            }

            // (b) nwn_gff's GFF written from our JSON. nwn_gff cannot read
            // JSON with an empty label, which BioWare's own palettes have
            // (tic01palstd.itp), so those are checked by (a) only.
            if has_empty_label(&ours.root) {
                return None;
            }
            let our_json = dir.join(format!("{i}.json"));
            std::fs::write(&our_json, serde_json::to_vec(&to_json(&ours, cp).unwrap()).unwrap())
                .unwrap();
            let out = dir.join(format!("{i}.out.{ty}"));
            if let Err(e) = run(
                &tool,
                &[
                    "-i",
                    our_json.to_str().unwrap(),
                    "-l",
                    "json",
                    "-k",
                    "gff",
                    "-o",
                    out.to_str().unwrap(),
                ],
            ) {
                return fail(format!("nwn_gff rejected our JSON: {e}"));
            }
            match Gff::read(&std::fs::read(&out).unwrap()) {
                Err(e) => fail(format!("cannot read nwn_gff's GFF: {e}")),
                Ok(theirs) if normalized(theirs.clone()) != ours => {
                    let d = mg_gff::diff(&ours, &normalized(theirs), 5);
                    fail(format!("nwn_gff's GFF from our JSON differs:\n  {}", d.join("\n  ")))
                }
                Ok(_) => None,
            }
        })
        .collect();
    eprintln!("compared {} GFFs with nwn_gff", files.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
