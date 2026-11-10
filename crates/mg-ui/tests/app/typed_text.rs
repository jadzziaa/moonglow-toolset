//! Text typed into the module's talk table (Options › Language › Put text
//! typed into the talk table, `docs/typed-text-proposal.md`): a name, a
//! description or a conversation line typed becomes a line of the custom
//! table at the next free line of the project's range, the field naming
//! it; the module's Undo and Redo take the line with the field; a text the
//! table has already is asked about; the table is saved with the module,
//! and a table that can't be saved keeps the module from being saved.

use crate::common::*;
use mg_module::talk::{Line, Table};

const BASE: u32 = mg_core::StrRef::CUSTOM_FLAG;

fn line(text: &str) -> Line {
    Line { text: text.into(), feminine: None, sound: String::new(), sound_length: 0.0 }
}

/// The sample module named "Sample", its custom talk table `mg_typed` in
/// the user's tlk folder with one line ("The Keep") and a feminine table,
/// typed text put in it or not (`on`); `None` without the game.
fn typed_harness(
    name: &str,
    on: bool,
) -> Option<(Harness<'static, Moonglow>, std::path::PathBuf, std::path::PathBuf)> {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return None;
    };
    let dir = mg_testkit::scratch_dir(name);
    let user = user_folder(&dir, &[]);
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_CUSTOM_TLK, ExoString::from("mg_typed"));
    let name = LocString::from_text(Language::ENGLISH, Gender::Male, "Sample");
    info.root.write(&ifo::MOD_NAME, name);
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let mut t = Table::create("mg_typed", &user.join("tlk"), Language::ENGLISH, true);
    t.add_line(&line("The Keep")).unwrap();
    t.save().unwrap();
    let mut app = game_app(&root, Some(user.clone()));
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
    app.settings.talk_typed = on;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    settle(&mut h);
    Some((h, user, path))
}

/// The module's name as it is now.
fn module_name(h: &mut Harness<'_, Moonglow>) -> LocString {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap().root.read(&ifo::MOD_NAME)
}

/// Types `to` over the text field that shows `from`, and leaves it.
fn retype(h: &mut Harness<'_, Moonglow>, from: &str, to: &str) {
    let input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(from).find(input).expect("the field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run();
    h.event(egui::Event::Text(to.into()));
    h.run();
    h.key_press(egui::Key::Tab);
    settle(h);
}

fn english(ls: &LocString) -> Option<&[u8]> {
    ls.get(Language::ENGLISH, Gender::Male)
}

/// The name typed is line 10 of the table (the project's range, set in
/// Module Properties), the field naming it; Undo takes both back, Redo
/// puts both back, and saving the module writes the table.
#[test]
fn a_name_typed_becomes_a_line_of_the_talk_table_and_undo_takes_both() {
    let Some((mut h, user, path)) = typed_harness("ui-typed-name", true) else { return };
    // The range, set in Module Properties › Custom Content, goes into the
    // project's file beside the .mod.
    h.get_by_label("Custom Content").click();
    settle(&mut h);
    h.get_by_label_contains("Next: line 1 (StrRef 16777217), at the table's end");
    type_into_hint(&mut h, "from", "10");
    type_into_hint(&mut h, "to", "19");
    h.get_by_label("Set").click();
    settle(&mut h);
    let toml = path.with_file_name("sample.moonglow.toml");
    let text = std::fs::read_to_string(&toml).unwrap();
    assert!(text.contains("[talk]\nlines = \"10-19\""), "{text}");
    h.get_by_label_contains("Next: line 10 (StrRef 16777226); 10 of 10 left");

    h.get_by_label("Basic").click();
    settle(&mut h);
    retype(&mut h, "Sample", "Castle Krezk");
    let name = module_name(&mut h);
    assert_eq!((name.strref.0, english(&name)), (BASE + 10, None));
    // The field shows the line's text at once, the table's (not saved yet).
    assert!(h.state().typed.ask.is_none(), "not asked of its own line");
    h.get_all_by_value("Castle Krezk").next().expect("the field shows the line");
    h.get_by_label("StrRef 16777226");
    let t = h.state().talk.as_ref().unwrap();
    assert_eq!((t.len(), t.line(10).text.as_str()), (11, "Castle Krezk"));
    assert_eq!(t.line(10).feminine.as_deref(), Some("Castle Krezk"), "the feminine table too");
    assert!(t.is_dirty());
    let said =
        h.state().log.entries.iter().any(|(_, e)| e.contains("is line 10 of the talk table"));
    assert!(said, "{:?}", h.state().log.entries);

    // Undo: the field and the line together; Redo: both back.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    let name = module_name(&mut h);
    assert_eq!((name.strref, english(&name)), (mg_core::StrRef::NONE, Some(&b"Sample"[..])));
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 1, "the line and the empty ones taken");
    h.state_mut().actions.push(mg_ui::Action::Redo);
    settle(&mut h);
    assert_eq!(module_name(&mut h).strref.0, BASE + 10);
    assert_eq!(h.state().talk.as_ref().unwrap().line(10).text, "Castle Krezk");

    // Saved with the module: the table's file has the line, and the game
    // data reads the name from it.
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert!(!h.state().has_unsaved_work());
    let tlk = mg_tlk::Tlk::read(&std::fs::read(user.join("tlk/mg_typed.tlk")).unwrap()).unwrap();
    assert_eq!(tlk.text(mg_core::StrRef(10)).as_deref(), Some("Castle Krezk"));
    let game = h.state().game.as_deref().unwrap();
    assert_eq!(game.string(mg_core::StrRef(BASE + 10)).as_deref(), Some("Castle Krezk"));
    let saved = Module::open(&path).unwrap();
    let name: LocString = saved.info().unwrap().root.read(&ifo::MOD_NAME);
    assert_eq!((name.strref.0, name.strings.len()), (BASE + 10, 0));
}

/// A text the table has a line with already is asked about: that line
/// named, another added, or the text kept in the file; Options can say
/// it without asking.
#[test]
fn a_text_the_talk_table_has_is_asked_about() {
    let Some((mut h, _, _)) = typed_harness("ui-typed-reuse", true) else { return };
    retype(&mut h, "Sample", "The Keep");
    h.get_by_label("Text in the Talk Table");
    assert_eq!(module_name(&mut h).strref, mg_core::StrRef::NONE, "nothing set before the answer");
    h.get_by_label("Use Line 0").click();
    settle(&mut h);
    let name = module_name(&mut h);
    assert_eq!((name.strref.0, english(&name)), (BASE, None));
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 1, "no line added");
    // Undo: the field back; the line, the table's own, stays.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert_eq!(english(&module_name(&mut h)), Some(&b"Sample"[..]));
    assert_eq!(h.state().talk.as_ref().unwrap().line(0).text, "The Keep");

    // Kept in the file: Aurora's way, for that text; Always do this makes
    // it Options › Language's choice, not asked again.
    retype(&mut h, "Sample", "The Keep");
    h.get_by_label("Always do this").click();
    h.run();
    h.get_by_label("Keep in the File").click();
    settle(&mut h);
    let name = module_name(&mut h);
    assert_eq!((name.strref, english(&name)), (mg_core::StrRef::NONE, Some(&b"The Keep"[..])));
    assert_eq!(h.state().settings.talk_reuse, mg_ui::settings::TalkReuse::File);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    retype(&mut h, "Sample", "The Keep");
    assert!(h.query_by_label("Text in the Talk Table").is_none(), "not asked");
    assert_eq!(module_name(&mut h).strref, mg_core::StrRef::NONE, "kept in the file");

    // Options say "Add a line": another, not asked; without a range, at
    // the table's end.
    h.state_mut().settings.talk_reuse = mg_ui::settings::TalkReuse::Add;
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    retype(&mut h, "Sample", "The Keep");
    assert!(h.query_by_label("Text in the Talk Table").is_none());
    assert_eq!(module_name(&mut h).strref.0, BASE + 1);
    assert_eq!(h.state().talk.as_ref().unwrap().line(1).text, "The Keep");
}

/// The question held one command: a second text the table has, typed
/// while the first was asked about, took its place, and the first field's
/// edit was dropped without a word (the review of November 2026, D15).
/// The second waits its turn and is asked about once the first is
/// answered; the question stops the editor meanwhile, and Escape keeps
/// the text in the file.
#[test]
fn two_texts_the_talk_table_has_are_each_asked_about() {
    let Some((mut h, _, _)) = typed_harness("ui-typed-two-asks", true) else { return };
    let key = ResKey::parse("module", ResType::IFO).unwrap();
    let typed = |label: &str| {
        let text = LocString::from_text(Language::ENGLISH, Gender::Male, "The Keep");
        mg_ui::Action::ApplyTyped(mg_edit::Command::new(
            label,
            vec![mg_edit::Edit::SetField {
                key,
                path: mg_edit::GffPath::root(),
                label: label.into(),
                value: Some(mg_gff::Value::LocString(text)),
            }],
        ))
    };
    h.state_mut().actions.push(typed("Mod_Name"));
    h.state_mut().actions.push(typed("Mod_Description"));
    settle(&mut h);
    let info = |h: &mut Harness<'_, Moonglow>| -> (LocString, LocString) {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let root = ws.doc(&key).unwrap().root.clone();
        (root.read(&ifo::MOD_NAME), root.read(&ifo::MOD_DESCRIPTION))
    };
    h.get_by_label("Text in the Talk Table");
    assert_eq!(info(&mut h).0.strref, mg_core::StrRef::NONE, "nothing set before the answer");
    // The first answered: its field set, and the second asked about.
    h.get_by_label("Use Line 0").click();
    settle(&mut h);
    let (name, description) = info(&mut h);
    assert_eq!(name.strref.0, BASE, "the first field's edit was not dropped");
    assert_eq!(description.strref, mg_core::StrRef::NONE, "asked about in its turn");
    h.get_by_label("Text in the Talk Table");
    // Escape: kept in the file, as its Keep in the File.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.query_by_label("Text in the Talk Table").is_none());
    let (name, description) = info(&mut h);
    assert_eq!(name.strref.0, BASE);
    assert_eq!(
        (description.strref, english(&description)),
        (mg_core::StrRef::NONE, Some(&b"The Keep"[..]))
    );
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 1, "no line added");
}

/// A text typed into the talk table leaves the game data's tables as they
/// were read: every one forgot `appearance.2da` and the rest, which the
/// open editors then read again (the review of November 2026, D27). With
/// the game data held by something else past the wait, the new line was
/// silently not shown: the log says so, and it shows once the game data
/// is free.
#[test]
fn a_typed_text_keeps_the_tables_read_and_shows_once_the_game_data_is_free() {
    let Some((mut h, _, _)) = typed_harness("ui-typed-tables-kept", true) else { return };
    let table =
        |h: &Harness<'_, Moonglow>| h.state().game.as_deref().unwrap().table("appearance").unwrap();
    let before = table(&h);
    retype(&mut h, "Sample", "Castle Krezk");
    assert_eq!(module_name(&mut h).strref.0, BASE + 1);
    assert!(std::sync::Arc::ptr_eq(&before, &table(&h)), "appearance.2da as it was read");
    let shown = |h: &Harness<'_, Moonglow>, line: u32| {
        h.state().game.as_deref().unwrap().string(mg_core::StrRef(BASE + line))
    };
    assert_eq!(shown(&h, 1).as_deref(), Some("Castle Krezk"));
    drop(before);
    // Held by another reader (past the wait for it): said, and shown when
    // it lets go.
    let held = h.state().game.clone();
    retype(&mut h, "Castle Krezk", "Castle Ravenloft");
    assert_eq!(h.state().talk.as_ref().unwrap().line(2).text, "Castle Ravenloft");
    let said = |h: &Harness<'_, Moonglow>, what: &str| {
        h.state().log.entries.iter().any(|(_, e)| e.contains(what))
    };
    assert!(said(&h, "The game data is in use"), "{:?}", h.state().log.entries);
    assert_eq!(shown(&h, 2), None, "not yet");
    drop(held);
    h.step();
    assert_eq!(shown(&h, 2).as_deref(), Some("Castle Ravenloft"));
    assert!(said(&h, "show in the fields now"));
}

/// Off (as by default), the text is the field's own, as Aurora writes it.
#[test]
fn off_by_default_typed_text_is_the_field_s_own() {
    assert!(!mg_ui::settings::Settings::default().talk_typed);
    let Some((mut h, _, _)) = typed_harness("ui-typed-off", false) else { return };
    retype(&mut h, "Sample", "Castle Krezk");
    let name = module_name(&mut h);
    assert_eq!((name.strref, english(&name)), (mg_core::StrRef::NONE, Some(&b"Castle Krezk"[..])));
    assert_eq!(h.state().talk.as_ref().map_or(1, Table::len), 1);
}

/// A conversation's line typed, in the editor and in Input Text, is a line
/// of the table; Undo of the line's adding takes its talk-table line too.
#[test]
fn a_conversation_line_typed_goes_into_the_talk_table() {
    let Some((mut h, _, _)) = typed_harness("ui-typed-dialog", true) else { return };
    h.state_mut().new_dialog = Some("typeddlg".into());
    h.run();
    h.get_by_label("Create").click();
    settle(&mut h);
    let key = ResKey::parse("typeddlg", ResType::DLG).unwrap();
    let input = |h: &mut Harness<'_, Moonglow>| {
        let role = egui::accesskit::Role::MultilineTextInput;
        h.get_all_by_value("<< Enter text here >>")
            .find(|n| n.accesskit_node().role() == role)
            .expect("the Input Text field")
            .type_text("Welcome, traveler.");
        h.run();
    };
    h.get_all_by_label("Add").last().unwrap().click();
    h.run();
    input(&mut h);
    h.get_all_by_label("OK").last().unwrap().click();
    settle(&mut h);
    let entry = |h: &mut Harness<'_, Moonglow>| -> Option<LocString> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let g = ws.doc(&key).unwrap();
        g.root.list("EntryList")?.first()?.locstring("Text").cloned()
    };
    let text = entry(&mut h).expect("the line");
    assert_eq!((text.strref.0, english(&text)), (BASE + 1, None));
    assert_eq!(h.state().talk.as_ref().unwrap().line(1).text, "Welcome, traveler.");
    // One step: Undo takes the line of the conversation and of the table.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert!(entry(&mut h).is_none());
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 1);
}

/// The range used up: the text stays in the file, and the log says why;
/// a table that can't be saved keeps the module from being saved.
#[test]
fn a_full_range_and_a_table_that_cannot_be_saved() {
    let Some((mut h, user, path)) = typed_harness("ui-typed-full", true) else { return };
    let place = mg_module::project_file::Place::beside(&path).unwrap();
    let only = mg_module::talk_range::Lines { from: 0, to: 0 };
    mg_module::project_file::ProjectFile::set_talk_lines(&place, Some(only)).unwrap();
    h.state_mut().ws.as_mut().unwrap().module.reread_project_file();
    retype(&mut h, "Sample", "Castle Krezk");
    let name = module_name(&mut h);
    assert_eq!(english(&name), Some(&b"Castle Krezk"[..]), "kept in the file");
    let said = h.state().log.entries.iter().any(|(_, e)| e.contains("are all used"));
    assert!(said, "{:?}", h.state().log.entries);

    // Lines at the end again; the table's file made unwritable (a folder
    // of its name): the module isn't saved either.
    mg_module::project_file::ProjectFile::set_talk_lines(&place, None).unwrap();
    h.state_mut().ws.as_mut().unwrap().module.reread_project_file();
    retype(&mut h, "Castle Krezk", "Vallaki");
    assert_eq!(module_name(&mut h).strref.0, BASE + 1);
    let file = user.join("tlk/mg_typed.tlk");
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir_all(file.join("blocked")).unwrap();
    let before = std::fs::read(&path).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert_eq!(std::fs::read(&path).unwrap(), before, "the module not written");
    assert!(h.state().has_unsaved_work());
    let said = h.state().log.entries.iter().any(|(_, e)| e.contains("The module was not saved"));
    assert!(said, "{:?}", h.state().log.entries);
}

/// One text typed into several fields at once (Edit Together's) is one
/// line; a string with text in another language too stays in the file,
/// as the game would show that text before the line.
#[test]
fn several_fields_take_one_line_and_other_languages_stay_in_the_file() {
    let Some((mut h, _, _)) = typed_harness("ui-typed-several", true) else { return };
    let key = ResKey::parse("module", ResType::IFO).unwrap();
    let set = |label: &str, ls: &LocString| mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(mg_gff::Value::LocString(ls.clone())),
    };
    let same = LocString::from_text(Language::ENGLISH, Gender::Male, "Barovia");
    let cmd =
        mg_edit::Command::new("Both", vec![set("Mod_Name", &same), set("Mod_Description", &same)]);
    h.state_mut().actions.push(mg_ui::Action::ApplyTyped(cmd));
    settle(&mut h);
    let info = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&key).unwrap().root.clone()
    };
    let i = info(&mut h);
    let (name, desc): (LocString, LocString) =
        (i.read(&ifo::MOD_NAME), i.read(&ifo::MOD_DESCRIPTION));
    assert_eq!((name.strref.0, desc.strref.0), (BASE + 1, BASE + 1));
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 2, "one line for both");

    let mut two = LocString::from_text(Language::ENGLISH, Gender::Male, "Vallaki");
    two.set(Language::FRENCH, Gender::Male, b"Vallaki (fr)".to_vec());
    h.state_mut().actions.push(mg_ui::Action::ApplyTyped(mg_edit::Command::new(
        "Name",
        vec![set("Mod_Name", &two)],
    )));
    settle(&mut h);
    let name: LocString = info(&mut h).read(&ifo::MOD_NAME);
    assert_eq!(name, two, "kept as typed");
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 2);
    let said = h.state().log.entries.iter().any(|(_, e)| e.contains("in another language"));
    assert!(said, "{:?}", h.state().log.entries);
}

/// A talk table kept in the module (a `.tlk` among its files) takes typed
/// lines too: saved into the module with it, outside the module's history,
/// so that Undo after the save takes the field and its line back and the
/// next save writes the table without it.
#[test]
fn a_table_kept_in_the_module_takes_typed_lines_and_saves_with_it() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-typed-in-module");
    let user = user_folder(&dir, &[]);
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_inmod", ResType::TLK).unwrap();
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_CUSTOM_TLK, ExoString::from("mg_inmod"));
    info.root
        .write(&ifo::MOD_NAME, LocString::from_text(Language::ENGLISH, Gender::Male, "Sample"));
    m.set_info(&info).unwrap();
    let mut t = Table::create("mg_inmod", &dir, Language::ENGLISH, false);
    t.add_line(&line("Old words")).unwrap();
    m.set(key, t.tlk.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = game_app(&root, Some(user.clone()));
    app.settings.no_last_area = true;
    app.settings.talk_typed = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    settle(&mut h);
    retype(&mut h, "Sample", "Castle Krezk");
    assert_eq!(module_name(&mut h).strref.0, BASE + 1);
    let saved_line = |path: &std::path::Path| {
        let saved = Module::open(path).unwrap();
        mg_tlk::Tlk::read(saved.get(&key).unwrap()).unwrap().text(mg_core::StrRef(1))
    };
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert!(!h.state().has_unsaved_work());
    assert_eq!(saved_line(&path).as_deref(), Some("Castle Krezk"));
    assert!(!user.join("tlk/mg_inmod.tlk").exists(), "in the module, not the tlk folder");
    // Undo after the save: the field and the line (not a step of the
    // table's own), and the next save writes the table without it.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert_eq!(english(&module_name(&mut h)), Some(&b"Sample"[..]));
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 1);
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert_eq!(saved_line(&path), None);
    let saved = Module::open(&path).unwrap();
    let name: LocString = saved.info().unwrap().root.read(&ifo::MOD_NAME);
    assert_eq!(english(&name), Some(&b"Sample"[..]));
}

/// String Edit's Move to Talk Table is not offered for a string with text
/// in another language too, which the game would show before the line.
#[test]
fn move_to_talk_table_waits_for_another_language_s_text_to_go() {
    let Some((mut h, _, _)) = typed_harness("ui-typed-move", false) else { return };
    let mut name = LocString::from_text(Language::ENGLISH, Gender::Male, "Sample");
    name.set(Language::GERMAN, Gender::Male, b"Beispiel".to_vec());
    let target = mg_ui::widgets::FieldTarget::new(
        ResKey::parse("module", ResType::IFO).unwrap(),
        mg_edit::GffPath::root(),
        "Mod_Name",
    );
    h.state_mut().loc_edit = Some(mg_ui::widgets::LocStringEdit::new(target, "Module name", &name));
    h.run();
    assert!(h.get_by_label("Move to Talk Table").accesskit_node().is_disabled());
    h.state_mut().loc_edit.as_mut().unwrap().entries.retain(|e| e.0 == Language::ENGLISH);
    h.run();
    assert!(!h.get_by_label("Move to Talk Table").accesskit_node().is_disabled());
}

/// A project's `[talk] typed` turns text typed into the talk table on (or
/// off) for everyone who opens the module, whatever their own switch
/// says: Options › Language shows the project's word, its switch not to
/// be changed for that module, and keeps the builder's own for others.
#[test]
fn the_project_turns_typed_text_on_for_everyone() {
    let Some((mut h, _, path)) = typed_harness("ui-typed-project", false) else { return };
    let toml = path.with_file_name("sample.moonglow.toml");
    std::fs::write(&toml, "[talk]\ntyped = true\n").unwrap();
    h.state_mut().ws.as_mut().unwrap().module.reread_project_file();
    retype(&mut h, "Sample", "Castle Krezk");
    assert_eq!(module_name(&mut h).strref.0, BASE + 1, "on, though the builder's switch is off");
    assert!(!h.state().settings.talk_typed, "the builder's own switch left as it was");

    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    h.get_by_label("Language").click();
    settle(&mut h);
    let switch = h.get_by_label("Put text typed into the talk table");
    assert!(switch.accesskit_node().is_disabled(), "the project's to change");
    assert_eq!(switch.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    h.get_by_label_contains("The open module's project turns it on for everyone");
    h.state_mut().options = None;
    settle(&mut h);

    // Off for everyone: the builder's switch on, the text stays in the file.
    std::fs::write(&toml, "[talk]\ntyped = false\n").unwrap();
    h.state_mut().ws.as_mut().unwrap().module.reread_project_file();
    h.state_mut().settings.talk_typed = true;
    retype(&mut h, "Castle Krezk", "Vallaki");
    // (Its own text, which the game shows before the line its StrRef still
    // names, as Aurora writes a text typed over a talk-table one.)
    assert_eq!(english(&module_name(&mut h)), Some(&b"Vallaki"[..]));
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 2, "no line added");
}
