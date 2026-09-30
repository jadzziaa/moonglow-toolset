//! Prints what verification finds in the shipped modules (missing and
//! unused resources by kind), for reviewing the reference classifier.
//!
//!     cargo run -p mg-corpus-tests --release --example verify_summary
use std::collections::BTreeMap;

use mg_module::Module;
use mg_module::verify::{missing, module_references, unused};
use mg_resman::{GameInstall, LayerClass, ResMan, priority};

fn main() {
    let root = mg_testkit::nwn_root().expect("no game install");
    let install = GameInstall::new(&root, None, "en");
    for path in mg_testkit::bundled_modules(&root) {
        let m = Module::open(&path).unwrap();
        let mut rm = ResMan::for_game(&install).unwrap();
        let haks = m.haks().unwrap();
        rm.add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>()).unwrap();
        rm.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        let refs = module_references(&m);
        let miss = missing(&m, &rm);
        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
        for x in &miss {
            *by_kind
                .entry(format!(
                    "{:?}{}",
                    x.reference.kind,
                    if x.uncompiled { "(src only)" } else { "" }
                ))
                .or_default() += 1;
        }
        let un = unused(&m);
        println!(
            "{}: {} refs, missing {:?}, unused {}",
            path.file_name().unwrap().to_string_lossy(),
            refs.len(),
            by_kind,
            un.len()
        );
        for x in miss.iter().take(4) {
            println!(
                "    {} {} -> {:?} {}",
                x.reference.from, x.reference.path, x.reference.kind, x.reference.target
            );
        }
    }
}
