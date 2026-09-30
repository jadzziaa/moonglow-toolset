//! 2DA corpus and differential tests.

use std::process::Command;

use mg_2da::TwoDa;
use mg_core::{Codepage, ResType};
use mg_erf::Erf;
use mg_key::KeySet;
use mg_testkit::{bundled_haks, corpus, nwn_tool, scratch_dir};
use rayon::prelude::*;

fn shipped_2das(root: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for key in ["nwn_base.key", "nwn_retail.key"] {
        let ks = KeySet::open(&root.join("data").join(key), root).unwrap();
        for e in ks.table.entries.iter().filter(|e| e.restype == ResType::TWODA) {
            out.push((format!("{key}:{}", e.resref), ks.data(e).unwrap().to_vec()));
        }
    }
    for hak in bundled_haks(root) {
        let data = std::fs::read(&hak).unwrap();
        let erf = Erf::read(&data).unwrap();
        for e in erf.entries.iter().filter(|e| e.restype == ResType::TWODA) {
            out.push((
                format!("{}:{}", hak.display(), e.resref),
                erf.data(e).unwrap().into_owned(),
            ));
        }
    }
    out
}

/// Every shipped 2DA parses and survives our writer; where nwn_twoda is
/// installed, its normalized rewrite of each file parses to the same table.
#[test]
fn shipped_2das_parse_and_agree_with_nwn_twoda() {
    let root = corpus!();
    let tool = nwn_tool("nwn_twoda");
    let dir = scratch_dir("twoda_diff");
    let cp = Codepage::WINDOWS_1252;
    let files = shipped_2das(&root);
    assert!(files.len() > 500);

    let failures: Vec<String> = files
        .par_iter()
        .enumerate()
        .filter_map(|(i, (name, bytes))| {
            let ours = match TwoDa::parse(bytes, cp) {
                Ok(t) => t,
                Err(e) => return Some(format!("{name}: {e}")),
            };
            let back = TwoDa::parse(&ours.to_bytes(cp).unwrap(), cp).unwrap();
            if back != ours {
                return Some(format!("{name}: our rewrite differs"));
            }
            let tool = tool.as_ref()?;
            // Known nwn_twoda divergences, not compared:
            // - it crashes on tables with no non-empty rows (retail iprp_base1);
            // - it drops `""` cells instead of reading them as empty cells in
            //   place, which misaligns rows in Beamdog's own id_resources.hak
            //   (e.g. random_hostile row 10 puts "trivial" in GREETING instead
            //   of ADJECTIVE). The engine reads it as we do; see
            //   engine_2da.rs.
            if ours.is_empty() || bytes.windows(2).any(|w| w == b"\"\"") {
                return None;
            }
            let (input, output) = (dir.join(format!("{i}.2da")), dir.join(format!("{i}.out.2da")));
            std::fs::write(&input, bytes).unwrap();
            let status = Command::new(tool)
                .args(["-i", input.to_str().unwrap(), "-o", output.to_str().unwrap()])
                .output()
                .unwrap();
            if !status.status.success() {
                return Some(format!("{name}: nwn_twoda failed: {}", String::from_utf8_lossy(&status.stderr)));
            }
            let theirs = TwoDa::parse(&std::fs::read(&output).unwrap(), cp).unwrap();
            if theirs != ours {
                let row = (0..ours.len().max(theirs.len())).find(|&r| ours.rows.get(r) != theirs.rows.get(r));
                return Some(format!(
                    "{name}: nwn_twoda reads it differently (columns equal: {}, rows {} vs {}, first differing row {row:?}: {:?} vs {:?})",
                    ours.columns() == theirs.columns(),
                    ours.len(),
                    theirs.len(),
                    row.and_then(|r| ours.rows.get(r)),
                    row.and_then(|r| theirs.rows.get(r)),
                ));
            }
            None
        })
        .collect();
    eprintln!("checked {} 2DAs (nwn_twoda: {})", files.len(), tool.is_some());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
