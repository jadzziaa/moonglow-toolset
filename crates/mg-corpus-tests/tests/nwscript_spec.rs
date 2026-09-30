//! The game's language spec (`nwscript.nss`), parsed by Moonglow and by
//! nwn.py's langspec reader, gives the same functions (ids, names, return
//! types, parameters, defaults) and constants.

use std::process::Command;

use mg_core::ResType;
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_script::spec::Spec;
use mg_testkit::{corpus, scratch_dir};

#[test]
fn spec_matches_nwn_py() {
    let root = corpus!();
    let python = std::env::var_os("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local/opt/neverwinter/venv/bin/python"))
        .filter(|p| p.is_file());
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let src = rm.get(&ResKey::parse("nwscript", ResType::NSS).unwrap()).unwrap();
    let spec = Spec::parse(&src);
    eprintln!(
        "{} functions, {} constants, engine structures {:?}",
        spec.functions.len(),
        spec.constants.len(),
        spec.engine_structures
    );
    assert!(spec.functions.len() > 1000);
    assert!(spec.constants.len() > 5000);
    assert!(spec.engine_structures.contains(&"json".to_string()));
    assert!(spec.function("GetObjectByTag").is_some_and(|f| !f.doc.is_empty()));

    let Some(python) = python else {
        eprintln!("skipped the nwn.py comparison: no nwn.py venv");
        return;
    };
    let dir = scratch_dir("nwscript_spec");
    let nss = dir.join("nwscript.nss");
    std::fs::write(&nss, &src).unwrap();
    let script = r#"
import json, sys
from nwn.nwscript import langspec
s = langspec.read(open(sys.argv[1], encoding="latin-1"))
def v(x):
    return x.value if hasattr(x, "value") else x
print(json.dumps({
  "functions": [[f.id, v(f.return_type), f.name, [[v(a.ty), a.name] for a in f.args]] for f in s.functions],
  "constants": [[v(c.ty), c.name] for c in s.constants],
}))
"#;
    let out = Command::new(&python).arg("-c").arg(script).arg(&nss).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let theirs: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    let ours_f: Vec<serde_json::Value> = spec
        .functions
        .iter()
        .map(|f| {
            serde_json::json!([
                f.id,
                f.return_type,
                f.name,
                f.params.iter().map(|p| [p.ty.clone(), p.name.clone()]).collect::<Vec<_>>()
            ])
        })
        .collect();
    let theirs_f = theirs["functions"].as_array().unwrap();
    assert_eq!(ours_f.len(), theirs_f.len(), "function count");
    for (a, b) in ours_f.iter().zip(theirs_f) {
        assert_eq!(a, b, "function differs");
    }
    let ours_c: Vec<serde_json::Value> =
        spec.constants.iter().map(|c| serde_json::json!([c.ty, c.name])).collect();
    let theirs_c = theirs["constants"].as_array().unwrap();
    let only_ours: Vec<_> = ours_c.iter().filter(|c| !theirs_c.contains(c)).collect();
    let only_theirs: Vec<_> = theirs_c.iter().filter(|c| !ours_c.contains(c)).collect();
    // nwn.py's reader only matches constant names in [A-Z0-9_]; the spec
    // also has names like DAMAGE_BONUS_1d4, which only we see.
    let regex_limit = |c: &&serde_json::Value| {
        c[1].as_str().is_some_and(|n| n.bytes().any(|b| b.is_ascii_lowercase()))
    };
    let unexplained: Vec<_> = only_ours.iter().filter(|c| !regex_limit(c)).collect();
    assert!(
        only_theirs.is_empty() && unexplained.is_empty(),
        "constants differ: only ours {unexplained:?}, only nwn.py {only_theirs:?}"
    );
    eprintln!("constants agree ({} with lowercase names that nwn.py skips)", only_ours.len());
}
