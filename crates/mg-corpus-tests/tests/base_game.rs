//! Corpus tests over the base game's KEY/BIF archives.

use std::collections::BTreeMap;
use std::sync::Mutex;

use mg_gff::Gff;
use mg_key::KeySet;
use mg_testkit::corpus;
use rayon::prelude::*;

/// Every resource listed in the shipped keys is reachable, and every GFF among
/// them parses and round-trips to an identical tree.
#[test]
fn every_key_resource_is_readable_and_gffs_round_trip() {
    let root = corpus!();
    let counts = Mutex::new(BTreeMap::<String, usize>::new());
    let mut failures = Vec::new();
    for key in ["nwn_base.key", "nwn_retail.key"] {
        let ks = KeySet::open(&root.join("data").join(key), &root).unwrap();
        assert!(!ks.table.entries.is_empty(), "{key} is empty");
        let f: Vec<String> = ks
            .table
            .entries
            .par_iter()
            .filter_map(|e| {
                let name = format!("{key}:{}.{}", e.resref, e.restype);
                let data = match ks.data(e) {
                    Ok(d) => d,
                    Err(err) => return Some(format!("{name}: {err}")),
                };
                *counts.lock().unwrap().entry(e.restype.to_string()).or_default() += 1;
                if !e.restype.is_gff() {
                    return None;
                }
                match Gff::read(data) {
                    Err(err) => Some(format!("{name}: {err}")),
                    Ok(g) => {
                        let back = Gff::read(&g.to_bytes().unwrap()).unwrap();
                        (back != g).then(|| format!("{name}: round trip differs"))
                    }
                }
            })
            .collect();
        failures.extend(f);
    }
    eprintln!("resources by type: {:?}", counts.lock().unwrap());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
