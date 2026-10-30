//! Engine check of how a module's hak names (`Mod_HakList`'s `Mod_Hak`, or
//! the older single `Mod_Hak`: CExoString bytes) become the file the game
//! opens, and that Moonglow finds the same file (`mg_resman::hak_name`).
//! On Linux and macOS, whose file names are any bytes.
#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_erf::{Erf, ErfWriter};
use mg_gff::{Gff, Struct, Value};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, scratch_dir};

const SCRIPT: &str = r#"
void main()
{
    WriteTimestampedLogEntry("MG_WHICH " + Get2DAString("mg_which", "Value", 0));
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

/// Haks in the user directory: each file's name's bytes (without `.hak`)
/// and the value its `mg_which.2da` answers.
type Haks<'a> = &'a [(&'a [u8], &'a str)];

/// A hak holding `mg_which.2da`, its one cell `value`.
fn hak(value: &str) -> Vec<u8> {
    let table = format!("2DA V2.0\r\n\r\n   Value\r\n0  {value}\r\n");
    let mut w = ErfWriter::new(*b"HAK ");
    w.add(ResRef::from_str("mg_which").unwrap(), ResType::TWODA, table.into_bytes()).unwrap();
    w.to_bytes().unwrap()
}

/// `<dir>/modules/<name>.mod`: a shipped module with the probe as its
/// OnModuleLoad and one hak, named by these bytes, in its list (or in the
/// older single field); and its IFO.
fn module(root: &Path, dir: &Path, name: &str, hak: &[u8], single: bool) -> Gff {
    let probe = crate::common::compile(dir, name, SCRIPT);
    let data = std::fs::read(root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let erf = Erf::read(&data).unwrap();
    let mut w = ErfWriter::new(*b"MOD ");
    let mut info = None;
    for e in &erf.entries {
        let mut bytes = erf.data(e).unwrap().into_owned();
        if e.restype == ResType::IFO {
            let mut ifo = Gff::read(&bytes).unwrap();
            ifo.root.set("Mod_OnModLoad", Value::ResRef(name.as_bytes().to_vec()));
            if single {
                ifo.root.fields.retain(|f| f.label.to_string_lossy() != "Mod_HakList");
                ifo.root.set("Mod_Hak", Value::String(hak.to_vec()));
            } else {
                let mut item = Struct::new(8);
                item.set("Mod_Hak", Value::String(hak.to_vec()));
                ifo.root.set("Mod_HakList", Value::List(vec![item]));
            }
            bytes = ifo.to_bytes().unwrap();
            info = Some(ifo);
        }
        w.add(e.resref, e.restype, bytes).unwrap();
    }
    w.add(ResRef::from_str(name).unwrap(), ResType::NCS, probe).unwrap();
    std::fs::write(dir.join(format!("modules/{name}.mod")), w.to_bytes().unwrap()).unwrap();
    info.unwrap()
}

/// What the server reads from a module naming its hak by `ifo_name`, with
/// these haks (file name bytes without `.hak`, and the value each answers)
/// in the scratch user directory's `hak/`: the value, or `None` when it
/// doesn't load the module; and what Moonglow reads: the value of the hak
/// file `find_hak` finds for the name `haks_of` reads.
fn loaded(
    root: &Path,
    test: &str,
    ifo_name: &[u8],
    single: bool,
    haks: Haks,
) -> (Option<String>, Option<String>) {
    use std::os::unix::ffi::OsStrExt;
    let dir = scratch_dir(test);
    std::fs::create_dir_all(dir.join("hak")).unwrap();
    std::fs::create_dir_all(dir.join("modules")).unwrap();
    let mut files = Vec::new();
    for (name, value) in haks {
        let file = [name, &b".hak"[..]].concat();
        let path = dir.join("hak").join(std::ffi::OsStr::from_bytes(&file));
        std::fs::write(&path, hak(value)).unwrap();
        files.push((path, value.to_string()));
    }
    let info = module(root, &dir, "mg_haknames", ifo_name, single);
    let install = mg_resman::GameInstall::new(root.to_path_buf(), Some(dir.clone()), "en");
    let names = mg_module::haks_of(&info.root);
    assert_eq!(names.len(), 1, "{names:?}");
    let found = install.find_hak(&names[0]);
    let ours = found.map(|f| files.iter().find(|(p, _)| *p == f).expect("ours").1.clone());
    let run = run_server(root, &dir, "mg_haknames", "MG_DONE", Duration::from_secs(90)).unwrap();
    let tail = &run.log[run.log.len().saturating_sub(1500)..];
    let theirs = run.finished.then(|| run.values("MG_WHICH").first().cloned().expect(tail));
    (theirs, ours)
}

/// The game opens the file whose name has the hak name's bytes, and
/// converts nothing: `ñ` in Windows-1252 (0xF1) opens `mg_\xF1hak.hak`,
/// `ñ` in UTF-8 (0xC3 0xB1) `mg_ñhak.hak`, each only its own, and the
/// older single `Mod_Hak` alike. Moonglow finds the same file each time
/// (and none where the server doesn't load the module).
#[test]
fn engine_opens_the_hak_file_named_by_the_name_s_bytes() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let cp = b"mg_\xf1hak".as_slice();
    let utf8 = "mg_ñhak".as_bytes();
    let both: Haks = &[(cp, "cp1252"), (utf8, "utf8")];
    let cases = [
        ("hak_names_cp", cp, false, both, Some("cp1252")),
        ("hak_names_utf8", utf8, false, both, Some("utf8")),
        ("hak_names_cp_alone", cp, false, &both[1..], None),
        ("hak_names_utf8_alone", utf8, false, &both[..1], None),
        ("hak_names_single", cp, true, both, Some("cp1252")),
    ];
    for (test, name, single, haks, want) in cases {
        let (theirs, ours) = loaded(&root, test, name, single, haks);
        assert_eq!(theirs.as_deref(), want, "{test}: the server");
        assert_eq!(ours.as_deref(), want, "{test}: Moonglow");
    }
}
