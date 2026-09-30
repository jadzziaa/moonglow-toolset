//! Helpers shared by the engine tests.

use std::path::Path;
use std::process::Command;

use mg_core::{ResRef, ResType};
use mg_erf::{Erf, ErfWriter};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;

/// Builds `<dir>/modules/<name>.mod` from a shipped module with `script`
/// (NWScript source) as OnModuleLoad, the given haks attached and `extra`
/// resources added.
#[allow(dead_code)]
pub(crate) fn probe_module(
    root: &Path,
    dir: &Path,
    name: &str,
    script: &str,
    haks: &[&str],
    extra: &[(ResKey, Vec<u8>)],
) {
    let compiler = mg_testkit::nwn_tool("nwn_script_comp").expect("nwn_script_comp");
    std::fs::create_dir_all(dir.join("modules")).unwrap();
    let nss = dir.join(format!("{name}.nss"));
    let ncs = dir.join(format!("{name}.ncs"));
    std::fs::write(&nss, script).unwrap();
    let out = Command::new(&compiler)
        .args(["-o", ncs.to_str().unwrap(), nss.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "compile failed: {}", String::from_utf8_lossy(&out.stderr));

    let data = std::fs::read(root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let erf = Erf::read(&data).unwrap();
    let mut w = ErfWriter::new(*b"MOD ");
    for e in &erf.entries {
        let mut bytes = erf.data(e).unwrap().into_owned();
        if e.restype == ResType::IFO {
            let mut ifo = Gff::read(&bytes).unwrap();
            ifo.root.set("Mod_OnModLoad", Value::ResRef(name.as_bytes().to_vec()));
            let list = haks
                .iter()
                .map(|h| {
                    let mut s = Struct::new(8);
                    s.set("Mod_Hak", Value::String(h.as_bytes().to_vec()));
                    s
                })
                .collect();
            ifo.root.set("Mod_HakList", Value::List(list));
            bytes = ifo.to_bytes().unwrap();
        }
        w.add(e.resref, e.restype, bytes).unwrap();
    }
    for (k, data) in extra {
        w.add(k.resref, k.restype, data.clone()).unwrap();
    }
    w.add(ResRef::from_str(name).unwrap(), ResType::NCS, std::fs::read(&ncs).unwrap()).unwrap();
    std::fs::write(dir.join(format!("modules/{name}.mod")), w.to_bytes().unwrap()).unwrap();
}

/// Rewrites every GFF in the module with Moonglow's writer, as a module
/// whose every resource was edited would be saved.
#[allow(dead_code)] // not every test binary uses every helper
pub(crate) fn rewrite_all_gffs(m: &mut mg_module::Module) {
    let keys: Vec<ResKey> = m.keys().copied().filter(|k| k.restype.is_gff()).collect();
    for k in keys {
        let gff = m.gff(&k).unwrap().unwrap();
        m.set_gff(k, &gff).unwrap();
    }
}

/// Compiles NWScript source with the official compiler (`nwn_script_comp`).
#[allow(dead_code)]
pub(crate) fn compile(dir: &Path, name: &str, source: &str) -> Vec<u8> {
    let compiler = mg_testkit::nwn_tool("nwn_script_comp").expect("nwn_script_comp");
    let nss = dir.join(format!("{name}.nss"));
    let ncs = dir.join(format!("{name}.ncs"));
    std::fs::write(&nss, source).unwrap();
    let out = Command::new(&compiler)
        .args(["-o", ncs.to_str().unwrap(), nss.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "compile failed: {}", String::from_utf8_lossy(&out.stderr));
    std::fs::read(&ncs).unwrap()
}
