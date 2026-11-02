//! Where the game reads a module's custom talk table from, and how it reads
//! one. The wiki says a table in a hak isn't read; the game reads it from
//! the module's haks, the module or the user's `tlk/` folder, in that order
//! (as Moonglow's resource manager does), and looks up the feminine table
//! `<name>f` the same way on its own. A table only in a hak userpatch.ini
//! names, or only in the user's `override/`, is read too (the resource
//! manager has them): a player without them can't load the module, which
//! the content doctor warns of. A name it can't find stops the module
//! from loading: one with `.tlk` on the end, or (on Linux) a file name in
//! another case. The content doctor reports both. A row's text counts only
//! with its text flag set; rows past the end are empty.

use std::time::Duration;

use mg_core::{Language, ResRef, ResType};
use mg_erf::{Erf, ErfWriter};
use mg_gff::{Gff, Value};
use mg_resman::ResKey;
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};
use mg_tlk::{FLAG_SOUND, Tlk, TlkEntry};

use crate::common;
use common::probe_module;

const PROBE: &str = r#"void Say(string sName, int nStrRef, int nGender)
{
    WriteTimestampedLogEntry("MG_" + sName + " [" + GetStringByStrRef(nStrRef, nGender) + "]");
}

void main()
{
    Say("BASE", 12, GENDER_MALE);
    Say("ROW0M", 16777216, GENDER_MALE);
    Say("ROW0F", 16777216, GENDER_FEMALE);
    Say("NOFLAG", 16777217, GENDER_MALE);
    Say("SOUNDONLY", 16777218, GENDER_MALE);
    Say("PAST", 16777226, GENDER_MALE);
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

/// A table whose row 0 says `row0`, row 1 has text without the text flag
/// and row 2 a sound only.
fn table(row0: &str) -> Vec<u8> {
    let mut t = Tlk::new(Language::ENGLISH);
    t.entries.push(TlkEntry::text(row0));
    t.entries.push(TlkEntry { text: b"unflagged".to_vec(), ..Default::default() });
    t.entries.push(TlkEntry {
        flags: FLAG_SOUND,
        sound: b"vs_hello".to_vec(),
        ..Default::default()
    });
    t.to_bytes().unwrap()
}

/// Names `tlk` as the module's custom talk table.
fn name_table(module: &std::path::Path, tlk: &str) {
    let data = std::fs::read(module).unwrap();
    let erf = Erf::read(&data).unwrap();
    let mut w = ErfWriter::new(*b"MOD ");
    for e in &erf.entries {
        let mut bytes = erf.data(e).unwrap().into_owned();
        if e.restype == ResType::IFO {
            let mut ifo = Gff::read(&bytes).unwrap();
            ifo.root.set("Mod_CustomTlk", Value::String(tlk.as_bytes().to_vec()));
            bytes = ifo.to_bytes().unwrap();
        }
        w.add(e.resref, e.restype, bytes).unwrap();
    }
    std::fs::write(module, w.to_bytes().unwrap()).unwrap();
}

struct Case {
    name: &'static str,
    /// What Mod_CustomTlk says.
    named: &'static str,
    /// Row 0 of the table in the `tlk/` folder (and of the feminine table
    /// there, with " (f)"), in a hak and in the module.
    folder: Option<&'static str>,
    hak: Option<&'static str>,
    module: Option<&'static str>,
    /// Row 0 of the table in a hak userpatch.ini names, and in the user's
    /// `override/` (as `mg_tlk.tlk`): the player's own, not the module's.
    elsewhere: Option<(&'static str, Elsewhere)>,
}

#[derive(Clone, Copy)]
enum Elsewhere {
    Userpatch,
    Override,
}

#[test]
fn where_the_game_reads_custom_talk_tables() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let dir = scratch_dir("engine_tlk");
    let key = ResKey::new(ResRef::from_str("mg_tlk").unwrap(), ResType::TLK);
    let case = |name, named, folder, hak, module| Case {
        name,
        named,
        folder,
        hak,
        module,
        elsewhere: None,
    };
    let away = |name, row, place| Case {
        elsewhere: Some((row, place)),
        ..case(name, "mg_tlk", None, None, None)
    };
    let cases = [
        away("userpatch", "userpatch", Elsewhere::Userpatch),
        away("override", "override", Elsewhere::Override),
        case("folder", "mg_tlk", Some("folder"), None, None),
        case("hak", "mg_tlk", None, Some("hak"), None),
        case("module", "mg_tlk", None, None, Some("module")),
        case("hak over folder", "mg_tlk", Some("folder"), Some("hak"), None),
        case("hak over module", "mg_tlk", None, Some("hak"), Some("module")),
        case("upper case", "MG_TLK", Some("folder"), None, None),
        case("extension", "mg_tlk.tlk", Some("folder"), None, None),
        case("missing", "mg_tlk", None, None, None),
    ];
    let mut seen = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let user = dir.join(format!("case{i}"));
        std::fs::create_dir_all(user.join("tlk")).unwrap();
        std::fs::create_dir_all(user.join("hak")).unwrap();
        if let Some(t) = case.folder {
            std::fs::write(user.join("tlk/mg_tlk.tlk"), table(t)).unwrap();
            std::fs::write(user.join("tlk/mg_tlkf.tlk"), table(&format!("{t} (f)"))).unwrap();
        }
        let mut haks = Vec::new();
        if let Some(t) = case.hak {
            let mut w = ErfWriter::new(*b"HAK ");
            w.add(key.resref, ResType::TLK, table(t)).unwrap();
            std::fs::write(user.join("hak/mg_tlkhak.hak"), w.to_bytes().unwrap()).unwrap();
            haks.push("mg_tlkhak");
        }
        match case.elsewhere {
            Some((t, Elsewhere::Userpatch)) => {
                std::fs::create_dir_all(user.join("patch")).unwrap();
                std::fs::write(
                    user.join("userpatch.ini"),
                    "[Patch]\r\nPatchFile000=mg_tlkpatch\r\n",
                )
                .unwrap();
                let mut w = ErfWriter::new(*b"HAK ");
                w.add(key.resref, ResType::TLK, table(t)).unwrap();
                std::fs::write(user.join("patch/mg_tlkpatch.hak"), w.to_bytes().unwrap()).unwrap();
            }
            Some((t, Elsewhere::Override)) => {
                std::fs::create_dir_all(user.join("override")).unwrap();
                std::fs::write(user.join("override/mg_tlk.tlk"), table(t)).unwrap();
            }
            None => {}
        }
        let extra: Vec<_> = case.module.map(|t| (key, table(t))).into_iter().collect();
        probe_module(&root, &user, "mgtlk", PROBE, &haks, &extra);
        name_table(&user.join("modules/mgtlk.mod"), case.named);
        let run = run_server(&root, &user, "mgtlk", "MG_DONE", Duration::from_secs(120)).unwrap();
        let got = if run.finished {
            let v = |t: &str| run.values(&format!("MG_{t} ")).join(",");
            assert_eq!(v("BASE"), "[Paladin]");
            assert_eq!(
                (v("NOFLAG"), v("SOUNDONLY"), v("PAST")),
                ("[]".into(), "[]".into(), "[]".into())
            );
            format!("{} / {}", v("ROW0M"), v("ROW0F"))
        } else {
            assert!(run.log.contains("Server shutting down"), "{}:\n{}", case.name, run.log);
            "not loaded".into()
        };
        seen.push(format!("{}: {got}", case.name));
    }
    let report = seen.join("\n");
    println!("{report}");
    assert_eq!(
        seen,
        [
            // The player's own, through the resource manager: a player
            // without them can't load the module.
            "userpatch: [userpatch] / [userpatch]",
            "override: [override] / [override]",
            "folder: [folder] / [folder (f)]",
            "hak: [hak] / [hak]",
            "module: [module] / [module]",
            // The feminine table isn't in the hak.
            "hak over folder: [hak] / [folder (f)]",
            "hak over module: [hak] / [hak]",
            "upper case: not loaded",
            "extension: not loaded",
            "missing: not loaded",
        ],
        "{report}"
    );
}

const TYPED_PROBE: &str = r#"void Name(string sTag, string sRes)
{
    object o = CreateObject(OBJECT_TYPE_ITEM, sRes, GetStartingLocation());
    WriteTimestampedLogEntry("MG_" + sTag + " [" + GetName(o) + "]");
}

void Probe()
{
    Name("TYPED", "mg_typed");
    Name("BOTH", "mg_both");
    Name("OTHER", "mg_other");
    WriteTimestampedLogEntry("MG_DONE");
}

void main()
{
    DelayCommand(2.0, Probe());
}
"#;

/// Text typed into a field and put in the module's custom talk table
/// (`mg_module::talk_range::in_line`, what Moonglow writes with Options ›
/// Language › Put text typed into the talk table): the game reads the
/// field's name from the table's line by its StrRef (16,777,216 and up),
/// the field's own English text taken out. A field with its own English
/// text beside the StrRef shows its own text; so does one with only
/// another language's text (French, in an English game): any text of the
/// string's own goes before the line.
#[test]
fn a_typed_name_is_read_from_the_custom_talk_table() {
    use mg_core::{Gender, LocString};
    use mg_module::talk_range::in_line;
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let dir = scratch_dir("engine_tlk_typed");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("tlk")).unwrap();
    let game = mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let gem = ResKey::new(ResRef::from_str("nw_it_gem001").unwrap(), ResType::UTI);
    let base = Gff::read(&game.resman.get(&gem).unwrap()).unwrap();
    let item = |resref: &str, name: LocString| {
        let mut g = base.clone();
        g.root.set("LocalizedName", Value::LocString(name));
        g.root.set("TemplateResRef", Value::ResRef(resref.as_bytes().to_vec()));
        g.root.set("Identified", Value::Byte(1));
        let key = ResKey::new(ResRef::from_str(resref).unwrap(), ResType::UTI);
        (key, g.to_bytes().unwrap())
    };
    let typed = LocString::from_text(Language::ENGLISH, Gender::Male, "Typed here");
    let mut both = LocString::from_text(Language::ENGLISH, Gender::Male, "Own text");
    both.strref = in_line(&typed, Language::ENGLISH, 3).strref;
    let mut other = typed.clone();
    other.set(Language::FRENCH, Gender::Male, b"Nom".to_vec());
    let mut t = Tlk::new(Language::ENGLISH);
    for text in ["zero", "one", "two", "From the table"] {
        t.entries.push(TlkEntry::text(text));
    }
    let key = ResKey::new(ResRef::from_str("mg_tlk").unwrap(), ResType::TLK);
    let extra = [
        (key, t.to_bytes().unwrap()),
        item("mg_typed", in_line(&typed, Language::ENGLISH, 3)),
        item("mg_both", both),
        item("mg_other", in_line(&other, Language::ENGLISH, 3)),
    ];
    probe_module(&root, &user, "mgtyped", TYPED_PROBE, &[], &extra);
    name_table(&user.join("modules/mgtyped.mod"), "mg_tlk");
    let run = run_server(&root, &user, "mgtyped", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "{}", run.log);
    let v = |t: &str| run.values(&format!("MG_{t} ")).join(",");
    println!("typed {} / both {} / other {}", v("TYPED"), v("BOTH"), v("OTHER"));
    assert_eq!(v("TYPED"), "[From the table]", "the line, by its StrRef");
    // Any text of the string's own goes before its StrRef, in the game's
    // language or another: Moonglow takes the language's text out, and
    // keeps a string with another language's text in the file.
    assert_eq!(v("BOTH"), "[Own text]", "the field's own text, not the line");
    assert_eq!(v("OTHER"), "[Nom]", "another language's text, not the line");
}
