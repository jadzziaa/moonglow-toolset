//! Corpus tests over the modules and haks that ship with the game.

use std::sync::atomic::{AtomicUsize, Ordering};

use mg_erf::{Erf, ErfWriter};
use mg_gff::Gff;
use mg_testkit::{bundled_haks, bundled_modules, corpus};
use rayon::prelude::*;

/// Every archive indexes; every GFF inside parses, and writing it back gives
/// an identical tree; rewriting the archive keeps every entry byte for byte.
#[test]
fn shipped_archives_and_their_gffs_round_trip() {
    let root = corpus!();
    let mut archives = bundled_modules(&root);
    archives.extend(bundled_haks(&root));
    assert!(archives.len() > 20, "expected the shipped modules and haks");

    let gffs = AtomicUsize::new(0);
    let failures: Vec<String> = archives
        .par_iter()
        .flat_map_iter(|path| {
            let mut fails = Vec::new();
            let data = std::fs::read(path).unwrap();
            let erf = match Erf::read(&data) {
                Ok(e) => e,
                Err(e) => return vec![format!("{}: {e}", path.display())],
            };
            let mut w = ErfWriter::new(erf.file_type);
            for entry in &erf.entries {
                let name = format!("{}:{}", path.display(), entry.filename());
                let bytes = match erf.data(entry) {
                    Ok(b) => b,
                    Err(e) => {
                        fails.push(format!("{name}: {e}"));
                        continue;
                    }
                };
                if entry.restype.is_gff() {
                    gffs.fetch_add(1, Ordering::Relaxed);
                    match Gff::read(&bytes) {
                        Err(e) => fails.push(format!("{name}: {e}")),
                        Ok(g) => {
                            let back = Gff::read(&g.to_bytes().unwrap()).unwrap();
                            if back != g {
                                fails.push(format!("{name}: GFF round trip differs"));
                            }
                        }
                    }
                }
                // Some shipped haks list a name twice (differing in case);
                // lookups use the first, so the writer keeps the first.
                if erf.find(&entry.resref, entry.restype) == Some(entry) {
                    w.add(entry.resref, entry.restype, bytes.into_owned()).unwrap();
                }
            }
            let rewritten = w.to_bytes().unwrap();
            let again = Erf::read(&rewritten).unwrap();
            assert_eq!(again.entries.len(), w.len());
            for b in &again.entries {
                let a = erf.find(&b.resref, b.restype).expect("entry exists");
                if erf.data(a).unwrap() != again.data(b).unwrap() {
                    fails.push(format!("{}: {} changed on rewrite", path.display(), a.filename()));
                }
            }
            fails
        })
        .collect();
    eprintln!("checked {} archives, {} GFFs", archives.len(), gffs.load(Ordering::Relaxed));
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
