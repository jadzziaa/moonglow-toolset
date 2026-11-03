//! `mg dialog-export` and `mg dialog-import`: a conversation of Contest of
//! Champions written as Ink and read back into a copy under another name
//! plays as the original.

use mg_core::ResType;
use mg_module::dialog_io::play_outline;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

#[test]
fn a_conversation_goes_out_as_ink_and_back() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-dialog-io");
    let path = dir.join("contest.mod");
    let mut m = Module::open(&root.join("data/mod/Contest Of Champions 0492.mod")).unwrap();
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    // The largest conversation that can be played out.
    let key = *m
        .keys_of(ResType::DLG)
        .filter(|k| play_outline(&m.gff(k).unwrap().unwrap(), 20_000).is_some())
        .max_by_key(|k| m.get(k).map_or(0, <[u8]>::len))
        .unwrap();
    let mg = |args: &[&str]| {
        let out = crate::mg_on(&root, None).args(args).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    };
    let ink = dir.join("story.ink");
    let (p, i) = (path.to_str().unwrap(), ink.to_str().unwrap());
    mg(&["dialog-export", p, &format!("{}.dlg", key.resref), i]);
    mg(&["dialog-import", p, i, "--name", "mg_back"]);
    let saved = Module::open(&path).unwrap();
    let outline = |k: ResKey| play_outline(&saved.gff(&k).unwrap().unwrap(), 20_000).unwrap();
    let back = ResKey::parse("mg_back", ResType::DLG).unwrap();
    assert!(!outline(key).is_empty(), "{key}");
    assert_eq!(outline(back), outline(key));
}

/// A CSV export for a translator: a column for each language asked for
/// (`--language`), filled in and read back into the conversation's lines;
/// no game install needed.
#[test]
fn a_csv_export_is_translated_and_read_back() {
    use mg_core::{Gender, Language};
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog, node};
    let dir = mg_testkit::scratch_dir("mg-dialog-csv-languages");
    let path = dir.join("town.mod");
    let mut m = Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello.");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Bye.");
    let key = ResKey::parse("greet", ResType::DLG).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let csv = dir.join("greet.csv");
    let (p, c) = (path.to_str().unwrap(), csv.to_str().unwrap());
    let run = |args: &[&str]| crate::mg_alone().args(args).output().unwrap();
    let out = run(&["dialog-export", p, "greet.dlg", c, "--language", "fr", "--language", "pl"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = std::fs::read_to_string(&csv).unwrap();
    assert!(
        text.starts_with("id,speaker,text,condition,action,parent,comment,text_fr,text_pl\r\n")
    );
    let text = text.replace("E0,,Hello.,,,start,,,", "E0,,Hello.,,,start,,Bonjour.,Cześć.");
    std::fs::write(&csv, text).unwrap();
    let out = run(&["dialog-import", p, c]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let g = Module::open(&path).unwrap().gff(&key).unwrap().unwrap();
    let t = node(&g, Kind::Entry, hello).unwrap().locstring("Text").unwrap().clone();
    assert_eq!(t.text(Language::ENGLISH, Gender::Male).unwrap(), "Hello.");
    assert_eq!(t.text(Language::FRENCH, Gender::Male).unwrap(), "Bonjour.");
    assert_eq!(t.get(Language::POLISH, Gender::Male).unwrap(), b"Cze\x9c\xe6.");
    // Twine and Ink hold one text a line: a language asked for is said
    // to be the CSV's.
    let ink = dir.join("greet.ink");
    let out = run(&["dialog-export", p, "greet.dlg", ink.to_str().unwrap(), "--language", "fr"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("for a CSV"));
    let out = run(&["dialog-export", p, "greet.dlg", c, "--language", "xx"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("\"xx\" isn't a language"));
}

/// Several conversations into a folder (`--to`, `--format`), each as
/// NAME.ink after its name, as the one-file form writes it; `--all` for
/// every one; files there already only with `--force`; a name the module
/// hasn't is said, the rest are written, and the command fails; `--json`
/// lists what was written.
#[test]
fn several_conversations_go_into_a_folder() {
    use mg_module::dialog::{Parent, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("mg-dialog-export-several");
    let path = dir.join("town.mod");
    let mut m = Module::new();
    m.set_info(&mg_gff::Gff::new(*b"IFO ")).unwrap();
    for (name, line) in [("gate", "Halt!"), ("inn", "A room?"), ("well", "Splash.")] {
        let mut g = new_dialog();
        add_node(&mut g, Parent::Root, line);
        m.set_gff(ResKey::parse(name, ResType::DLG).unwrap(), &g).unwrap();
    }
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let p = path.to_str().unwrap();
    let run = |args: &[&str]| crate::mg_alone().args(args).output().unwrap();
    let ok = |out: &std::process::Output| {
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    };
    // The one-file form, as it was.
    let single = dir.join("inn.ink");
    ok(&run(&["dialog-export", p, "inn.dlg", single.to_str().unwrap()]));
    let stories = dir.join("stories");
    let _ = std::fs::remove_dir_all(&stories);
    let s = stories.to_str().unwrap();
    let (done, json) = crate::json_out(crate::mg_alone().args([
        "--json",
        "dialog-export",
        p,
        "gate.dlg",
        "inn.dlg",
        "--to",
        s,
        "--format",
        "ink",
    ]));
    assert!(done, "{json}");
    let written: Vec<&str> = json["written"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["conversation"].as_str().unwrap())
        .collect();
    assert_eq!(written, ["gate.dlg", "inn.dlg"]);
    assert_eq!(json["format"], "Ink");
    assert!(json["failed"].as_array().unwrap().is_empty());
    assert_eq!(std::fs::read(stories.join("inn.ink")).unwrap(), std::fs::read(&single).unwrap());
    assert!(!stories.join("well.ink").exists());
    // There already: refused, naming them, without --force.
    let out = run(&["dialog-export", p, "--all", "--to", s, "--format", "ink"]);
    assert!(!out.status.success());
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("gate.ink, inn.ink") && said.contains("--force"), "{said}");
    assert!(!stories.join("well.ink").exists(), "nothing written");
    ok(&run(&["dialog-export", p, "--all", "--to", s, "--format", "ink", "--force"]));
    assert!(stories.join("well.ink").exists());
    // Each format by its extension; one named the module hasn't.
    for (format, ext) in [("twine", "twee"), ("text", "txt"), ("csv", "csv")] {
        let out =
            run(&["dialog-export", p, "inn.dlg", "nobody.dlg", "--to", s, "--format", format]);
        assert!(!out.status.success(), "{format}: nobody.dlg isn't in the module");
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(said.contains("nobody.dlg is not in the module"), "{said}");
        let one = dir.join(format!("inn.{ext}"));
        ok(&run(&["dialog-export", p, "inn.dlg", one.to_str().unwrap()]));
        let several = stories.join(format!("inn.{ext}"));
        assert_eq!(std::fs::read(several).unwrap(), std::fs::read(&one).unwrap(), "{format}");
    }
    // What is asked for wrongly.
    let out = run(&["dialog-export", p, "inn.dlg", "--to", s]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--to needs --format"));
    let out = run(&["dialog-export", p, "gate.dlg", "inn.dlg", "x.ink"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--to FOLDER"));
    let out = run(&["dialog-export", p, "--to", s, "--format", "ink"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("--all for every one"));
}
