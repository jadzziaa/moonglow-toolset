//! Engine check of how a module's hak names (`Mod_HakList`'s `Mod_Hak`, or
//! the older single `Mod_Hak`) and its custom talk table's
//! (`Mod_CustomTlk`), CExoString bytes, become the file the game opens, and
//! that Moonglow finds the same file (`mg_resman::file_name_text`).
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

/// `<dir>/modules/<name>.mod`: a shipped module with `script` as its
/// OnModuleLoad and its IFO's root changed by `edit`; and its IFO.
fn module(root: &Path, dir: &Path, name: &str, script: &str, edit: &dyn Fn(&mut Struct)) -> Gff {
    let probe = crate::common::compile(dir, name, script);
    let data = std::fs::read(root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let erf = Erf::read(&data).unwrap();
    let mut w = ErfWriter::new(*b"MOD ");
    let mut info = None;
    for e in &erf.entries {
        let mut bytes = erf.data(e).unwrap().into_owned();
        if e.restype == ResType::IFO {
            let mut ifo = Gff::read(&bytes).unwrap();
            ifo.root.set("Mod_OnModLoad", Value::ResRef(name.as_bytes().to_vec()));
            edit(&mut ifo.root);
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
    // One hak, named by these bytes, in the list (or in the older single
    // field).
    let name_it = |root: &mut Struct| {
        if single {
            root.fields.retain(|f| f.label.to_string_lossy() != "Mod_HakList");
            root.set("Mod_Hak", Value::String(ifo_name.to_vec()));
        } else {
            let mut item = Struct::new(8);
            item.set("Mod_Hak", Value::String(ifo_name.to_vec()));
            root.set("Mod_HakList", Value::List(vec![item]));
        }
    };
    let info = module(root, &dir, "mg_haknames", SCRIPT, &name_it);
    let install = mg_resman::GameInstall::new(root.to_path_buf(), Some(dir.clone()), "en");
    let names = mg_module::haks_of(&info.root);
    assert_eq!(names.len(), 1, "{names:?}");
    let found = install.find_hak(&names[0]);
    let ours = found.map(|f| files.iter().find(|(p, _)| *p == f).expect("ours").1.clone());
    // A file there of its name in another case is named where it isn't
    // found (Linux's file system: macOS's finds it).
    if ours.is_none() && !mg_resman::CASE_IGNORED {
        let said = install.hak_not_found(&names[0]);
        let other = (haks.iter())
            .map(|(n, _)| mg_resman::file_name_text(n))
            .find(|n| mg_resman::differs_in_case(&names[0], n));
        match other {
            Some(n) => assert!(said.contains(&format!("{n}.hak")), "{said}"),
            None => assert_eq!(said, format!("hak {} not found", names[0])),
        }
    }
    let run = run_server(root, &dir, "mg_haknames", "MG_DONE", Duration::from_secs(90)).unwrap();
    let tail = &run.log[run.log.len().saturating_sub(1500)..];
    let theirs = run.finished.then(|| run.values("MG_WHICH").first().cloned().expect(tail));
    (theirs, ours)
}

/// The game opens the file whose name has the hak name's bytes, and
/// converts nothing: `ñ` in Windows-1252 (0xF1) opens `mg_\xF1hak.hak`,
/// `ñ` in UTF-8 (0xC3 0xB1) `mg_ñhak.hak`, each only its own, and the
/// older single `Mod_Hak` alike; nor does it fold case (`MG_PLAIN` doesn't
/// open `mg_plain.hak`). Moonglow finds the same file each time (and none
/// where the server doesn't load the module, naming the file in another
/// case).
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
    let plain: Haks = &[(b"mg_plain", "plain")];
    let cases = [
        ("hak_names_case", b"MG_PLAIN".as_slice(), false, plain, None),
        ("hak_names_plain", b"mg_plain".as_slice(), false, plain, Some("plain")),
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

const TLK_SCRIPT: &str = r#"
void main()
{
    WriteTimestampedLogEntry("MG_WHICH " + GetStringByStrRef(16777216));
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

/// What the server reads from a module naming its custom talk table by
/// `ifo_name`, with these tables (file name bytes without `.tlk`, and the
/// text of each one's first row) in the scratch user directory's `tlk/`:
/// the text, or `None` when it doesn't load the module; and what Moonglow
/// reads: the first row of the table `talk::of_module` finds.
fn talk_table(
    root: &Path,
    test: &str,
    ifo_name: &[u8],
    tables: Haks,
) -> (Option<String>, Option<String>) {
    use std::os::unix::ffi::OsStrExt;
    let dir = scratch_dir(test);
    std::fs::create_dir_all(dir.join("tlk")).unwrap();
    std::fs::create_dir_all(dir.join("modules")).unwrap();
    for (name, value) in tables {
        let file = [name, &b".tlk"[..]].concat();
        let mut t = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
        t.entries.push(mg_tlk::TlkEntry::text(*value));
        let path = dir.join("tlk").join(std::ffi::OsStr::from_bytes(&file));
        std::fs::write(&path, t.to_bytes().unwrap()).unwrap();
    }
    let name_it = |root: &mut Struct| root.set("Mod_CustomTlk", Value::String(ifo_name.to_vec()));
    module(root, &dir, "mg_tlknames", TLK_SCRIPT, &name_it);
    let m = mg_module::Module::open(&dir.join("modules/mg_tlknames.mod")).unwrap();
    let found = mg_module::talk::of_module(&m, &mg_resman::ResMan::new(), &[dir.join("tlk")], None);
    let ours = match found {
        mg_module::talk::Custom::Found { found, .. } => {
            let first = found.tlk().unwrap().entries[0].text.clone();
            Some(String::from_utf8(first).unwrap())
        }
        _ => None,
    };
    let run = run_server(root, &dir, "mg_tlknames", "MG_DONE", Duration::from_secs(90)).unwrap();
    let tail = &run.log[run.log.len().saturating_sub(1500)..];
    let theirs = run.finished.then(|| run.values("MG_WHICH").first().cloned().expect(tail));
    (theirs, ours)
}

/// A custom talk table's name (`Mod_CustomTlk`) is its file's name's bytes
/// as a hak's is: `ñ` in Windows-1252 opens `mg_\xF1tlk.tlk` in the user's
/// `tlk/`, in UTF-8 `mg_ñtlk.tlk`, each only its own (with only the other
/// there the module doesn't load), and `MG_PLAIN` not `mg_plain.tlk`.
/// Moonglow finds the same table each time, and none where the server
/// doesn't load the module.
#[test]
fn engine_opens_the_talk_table_file_named_by_the_name_s_bytes() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let cp = b"mg_\xf1tlk".as_slice();
    let utf8 = "mg_ñtlk".as_bytes();
    let both: Haks = &[(cp, "cp1252"), (utf8, "utf8")];
    let plain: Haks = &[(b"mg_plain", "plain")];
    let cases = [
        ("tlk_names_cp", cp, both, Some("cp1252")),
        ("tlk_names_utf8", utf8, both, Some("utf8")),
        ("tlk_names_cp_alone", cp, &both[1..], None),
        ("tlk_names_utf8_alone", utf8, &both[..1], None),
        ("tlk_names_case", b"MG_PLAIN".as_slice(), plain, None),
        ("tlk_names_plain", b"mg_plain".as_slice(), plain, Some("plain")),
    ];
    for (test, name, tables, want) in cases {
        let (theirs, ours) = talk_table(&root, test, name, tables);
        assert_eq!(theirs.as_deref(), want, "{test}: the server");
        assert_eq!(ours.as_deref(), want, "{test}: Moonglow");
    }
}
