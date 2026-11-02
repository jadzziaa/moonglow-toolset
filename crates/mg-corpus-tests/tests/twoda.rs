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
            let blank = ours.rows.iter().all(|r| r.iter().all(Option::is_none));
            if blank || bytes.windows(2).any(|w| w == b"\"\"") {
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
            // - it drops the rows of empty cells at a table's end (the game's
            //   ambientsound.2da's last 52); the engine counts them, as
            //   Moonglow reads them (engine_counts_an_empty_last_row):
            //   compared without them.
            let trimmed = |t: &TwoDa| {
                let mut t = t.clone();
                while t.rows.last().is_some_and(|r| r.iter().all(Option::is_none)) {
                    t.rows.pop();
                }
                t
            };
            let (ours, theirs) = (trimmed(&ours), trimmed(&theirs));
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

/// Find References to Row with the game's data: a constant of `nwscript`
/// is the row it stands for (`SPELL_FIREBALL`), and an item property's
/// subtype a row of its property's table (`mg_module::row_uses`).
#[test]
fn a_constant_and_a_property_s_subtype_name_rows() {
    use mg_core::ResRef;
    use mg_gff::{Gff, Struct, Value};
    use mg_module::row_uses::row_uses;
    use mg_resman::{GameInstall, ResKey};
    let root = corpus!();
    let game = mg_rules::GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let key = |n: &str, t| ResKey::new(ResRef::from_str(n).unwrap(), t);
    let mut m = mg_module::Module::new();
    let src = b"void main()\n{\n    ActionCastSpellAtObject(SPELL_FIREBALL, OBJECT_SELF);\n}\n";
    m.set(key("cast", ResType::NSS), src.to_vec());
    let spells = game.table("spells").unwrap();
    let fireball = (0..spells.len()).find(|&r| spells.get(r, "Label") == Some("Fireball")).unwrap();
    let cp = game.codepage();
    let found = row_uses(&m, "spells", fireball, Some(&game), cp);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].place.contains("line 3: SPELL_FIREBALL"), "{found:#?}");
    // An item with a Cast Spell property (itempropdef 15) of subtype 0: a
    // row of its subtype table.
    let mut uti = Gff::new(*b"UTI ");
    let mut p = Struct::new(0);
    p.set("PropertyName", Value::Word(15));
    p.set("Subtype", Value::Word(0));
    uti.root.set("PropertiesList", Value::List(vec![p]));
    m.set(key("wand", ResType::UTI), uti.to_bytes().unwrap());
    let table = game.table("itempropdef").unwrap().get(15, "SubTypeResRef").unwrap().to_lowercase();
    let found = row_uses(&m, &table, 0, Some(&game), cp);
    assert!(found.iter().any(|u| u.place.ends_with("Subtype")), "{table}: {found:#?}");
}
