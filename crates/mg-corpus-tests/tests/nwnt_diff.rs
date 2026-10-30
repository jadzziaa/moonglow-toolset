//! Differential test against WilliamDraco's `nwn_nwnt`, the converter nasher
//! keeps a project's GFF files with when its `gffFormat` is `nwnt`: for a
//! sample of shipped GFFs of every type nasher converts, (a) its NWNT of the
//! file must be ours to the byte, (b) the GFF it writes back from that text
//! must be the tree we read from it, and (c) ours written again from what we
//! read must be the same text.

use mg_core::Codepage;
use mg_erf::Erf;
use mg_gff::{Gff, from_nwnt, to_nwnt};
use mg_key::KeySet;
use mg_module::nasher::GFF_EXTENSIONS;
use mg_testkit::{corpus, oracle_tool, scratch_dir};
use rayon::prelude::*;

use crate::nwn_gff_diff::{has_empty_label, normalized, run, sample};

/// The first line where two texts differ, for messages.
fn first_difference(ours: &str, theirs: &str) -> String {
    for (i, (x, y)) in ours.split('\n').zip(theirs.split('\n')).enumerate() {
        if x != y {
            return format!("line {}: ours {x:?}, nwn_nwnt's {y:?}", i + 1);
        }
    }
    format!("lengths {} and {}", ours.len(), theirs.len())
}

#[test]
fn nwnt_agrees_with_nwn_nwnt() {
    let root = corpus!();
    let tool = oracle_tool!("nwn_nwnt");
    let dir = scratch_dir("nwnt_diff");
    let cp = Codepage::WINDOWS_1252;
    let nasher_keeps = |ty: mg_core::ResType| {
        ty.is_gff() && ty.extension().is_some_and(|e| GFF_EXTENSIONS.contains(&e))
    };

    let mut files = Vec::new();
    let ks = KeySet::open(&root.join("data/nwn_base.key"), &root).unwrap();
    for e in ks.table.entries.iter().filter(|e| nasher_keeps(e.restype)) {
        files.push((e.restype, (format!("base:{}", e.resref), ks.data(e).unwrap().to_vec())));
    }
    for m in ["data/nwm/Prelude.nwm", "data/nwm/Chapter1.nwm"] {
        let Ok(data) = std::fs::read(root.join(m)) else { continue };
        let erf = Erf::read(&data).unwrap();
        for e in erf.entries.iter().filter(|e| nasher_keeps(e.restype)) {
            files.push((
                e.restype,
                (format!("{m}:{}", e.resref), erf.data(e).unwrap().into_owned()),
            ));
        }
    }
    let files = sample(files);
    assert!(files.len() > 100, "{} files", files.len());

    let failures: Vec<String> = files
        .par_iter()
        .enumerate()
        .filter_map(|(i, (ty, (name, bytes)))| {
            let fail = |m: String| Some(format!("{name}.{ty}: {m}"));
            let gff = Gff::read(bytes).unwrap();

            // (a) nwn_nwnt's text of the file.
            let input = dir.join(format!("{i}.{ty}"));
            std::fs::write(&input, bytes).unwrap();
            let base = dir.join(format!("{i}.out"));
            if let Err(e) = run(&tool, &[input.to_str().unwrap(), "-o", base.to_str().unwrap()]) {
                return fail(format!("nwn_nwnt failed: {e}"));
            }
            let text_path = dir.join(format!("{i}.out.nwnt"));
            let theirs = String::from_utf8(std::fs::read(&text_path).unwrap()).unwrap();
            let ours = to_nwnt(&gff, cp, 4);
            if ours != theirs {
                return fail(first_difference(&ours, &theirs));
            }

            let read = match from_nwnt(&theirs, cp) {
                Ok(g) => g,
                Err(e) => return fail(format!("can't read nwn_nwnt's text: {e}")),
            };
            // (b) the GFF nwn_nwnt writes back from it. It can't read back
            // a field with an empty label, which BioWare's own palettes
            // have (tic01palstd.itp): those are checked by (a) and (c).
            if has_empty_label(&gff.root) {
                return (to_nwnt(&read, cp, 4) != theirs)
                    .then(|| format!("{name}.{ty}: not stable"));
            }
            let back = dir.join(format!("{i}.back.{ty}"));
            if let Err(e) = run(&tool, &[text_path.to_str().unwrap(), "-o", back.to_str().unwrap()])
            {
                return fail(format!("nwn_nwnt failed to read its own text: {e}"));
            }
            let theirs_back = normalized(Gff::read(&std::fs::read(&back).unwrap()).unwrap());
            if normalized(read.clone()) != theirs_back {
                let d = mg_gff::diff(&normalized(read), &theirs_back, 5);
                return fail(format!(
                    "read otherwise than nwn_nwnt reads it:\n  {}",
                    d.join("\n  ")
                ));
            }

            // (c) written again, the same text.
            let again = to_nwnt(&read, cp, 4);
            (again != theirs)
                .then(|| format!("{name}.{ty}: not stable: {}", first_difference(&again, &theirs)))
        })
        .collect();
    eprintln!("compared {} GFFs with nwn_nwnt", files.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
