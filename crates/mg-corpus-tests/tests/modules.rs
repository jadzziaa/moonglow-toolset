//! Module workspace round trips over the shipped modules.

use mg_core::ResType;
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_testkit::{bundled_modules, corpus, scratch_dir};
use rayon::prelude::*;

mod common;
use common::rewrite_all_gffs;

/// Every shipped module opens; its module info names areas that exist and
/// haks that ship; after rewriting every GFF and saving as an archive and as
/// a folder, both reopen with the same resources, GFFs as identical trees and
/// everything else byte for byte.
#[test]
fn shipped_modules_round_trip_through_the_workspace() {
    let root = corpus!();
    let dir = scratch_dir("modules_round_trip");
    let modules = bundled_modules(&root);
    assert!(modules.len() > 20);
    let failures: Vec<String> = modules
        .par_iter()
        .enumerate()
        .flat_map_iter(|(i, path)| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let mut fails = Vec::new();
            let original = Module::open(path).unwrap();
            // Areas resolve through the module and its haks (premium modules
            // keep areas in their haks).
            let install = GameInstall::new(&root, None, "en");
            let mut rm = ResMan::new();
            let haks = original.haks().unwrap();
            let hak_names: Vec<&str> = haks.iter().map(String::as_str).collect();
            for missing in rm.add_haks(&install, &hak_names).unwrap() {
                fails.push(format!("{name}: hak {missing} not shipped"));
            }
            rm.add(priority::MODULE, "module", LayerClass::Erf, original.container());
            for area in original.areas().unwrap() {
                if !rm.contains(&ResKey::new(area, ResType::ARE)) {
                    fails.push(format!("{name}: area {area} listed but found nowhere"));
                }
            }
            let mut m = original.clone();
            rewrite_all_gffs(&mut m);
            let archive = ModuleLocation::Archive(dir.join(format!("{i}.mod")));
            let folder = ModuleLocation::Folder(dir.join(format!("{i}")));
            m.save_as(&archive).unwrap();
            m.save_as(&folder).unwrap();
            for loc in [archive, folder] {
                let back = Module::open(loc.path()).unwrap();
                if back.len() != original.len() {
                    fails.push(format!(
                        "{name}: {} resources after saving to {loc:?}, {} before",
                        back.len(),
                        original.len()
                    ));
                }
                for k in original.keys() {
                    let (a, b) = (original.get(k).unwrap(), back.get(k));
                    let same = match b {
                        None => false,
                        Some(b) if k.restype.is_gff() => {
                            Gff::read(a).unwrap() == Gff::read(b).unwrap()
                        }
                        Some(b) => a == b,
                    };
                    if !same {
                        fails.push(format!("{name}: {k} differs after saving to {loc:?}"));
                    }
                }
            }
            fails
        })
        .collect();
    eprintln!("round-tripped {} modules", modules.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}
