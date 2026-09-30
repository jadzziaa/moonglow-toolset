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

/// Walks every area and object during module load, before any heartbeat or
/// AI has run, yielding with `DelayCommand` only when the instruction budget
/// runs low (then resuming at the same area and object). Ends with the done
/// marker and padding that makes the server flush its buffered log.
#[allow(dead_code)]
pub(crate) const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

void Finish()
{
    Log("MG_DONE");
    int i;
    for (i = 0; i < 2000; i++)
        Log("MG_PAD ................................................................");
}

void ProbeFrom(int nArea, int nObj)
{
    int a = 0;
    object oArea = GetFirstArea();
    while (GetIsObjectValid(oArea) && a < nArea) { oArea = GetNextArea(); a++; }
    while (GetIsObjectValid(oArea))
    {
        string sArea = GetResRef(oArea);
        if (nObj == 0)
            Log("MG_AREA " + sArea + "|" + GetTag(oArea) + "|" + GetName(oArea));
        int i = 0;
        object o = GetFirstObjectInArea(oArea);
        while (GetIsObjectValid(o))
        {
            if (i >= nObj)
            {
                if (GetScriptInstructionsRemaining() < 20000)
                {
                    DelayCommand(0.0, ProbeFrom(a, i));
                    return;
                }
                vector v = GetPosition(o);
                Log("MG_OBJ " + sArea + "|" + IntToString(GetObjectType(o)) + "|" + GetTag(o)
                    + "|" + GetResRef(o) + "|" + FloatToString(v.x, 0, 2) + "," + FloatToString(v.y, 0, 2)
                    + "," + FloatToString(v.z, 0, 2) + "|" + FloatToString(GetFacing(o), 0, 1));
            }
            i++;
            o = GetNextObjectInArea(oArea);
        }
        nObj = 0;
        a++;
        oArea = GetNextArea();
    }
    DelayCommand(0.0, Finish());
}

void main()
{
    Log("MG_MODULE " + GetName(GetModule()) + "|" + GetTag(GetModule()));
    ProbeFrom(0, 0);
}
"#;

/// The probe's view of the world, sorted. Creatures keep only their area,
/// type, tag and blueprint: their AI may move them between probe chunks.
#[allow(dead_code)]
pub(crate) fn world(run: &mg_testkit::engine::ServerRun) -> Vec<String> {
    let mut lines: Vec<String> = ["MG_MODULE", "MG_AREA", "MG_OBJ"]
        .iter()
        .flat_map(|tag| run.values(tag).into_iter().map(move |v| format!("{tag} {v}")))
        .map(|l| match l.strip_prefix("MG_OBJ ") {
            Some(obj) if obj.split('|').nth(1) == Some("1") => {
                let identity: Vec<&str> = obj.split('|').take(4).collect();
                format!("MG_OBJ {} (creature)", identity.join("|"))
            }
            _ => l,
        })
        .collect();
    lines.sort();
    lines
}
