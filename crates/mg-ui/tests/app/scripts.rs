//! The script editor and compiling.

use crate::common::*;
use mg_ui::search_results::Hit;

fn script_cursor(h: &Harness<'_, Moonglow>, key: ResKey) -> Option<(usize, usize)> {
    let state = egui::text_edit::TextEditState::load(&h.ctx, egui::Id::new(("script", key)))?;
    let r = state.cursor.char_range()?;
    Some((r.primary.index.into(), r.secondary.index.into()))
}

#[test]
fn script_editor_completion_find_bookmarks() {
    let dir = mg_testkit::scratch_dir("ui-script-tools");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void MyHelper() {}\nvoid main()\n{\n    MyHe\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();

    // Completion (F2) after "MyHe".
    let after_prefix = text.find("MyHe\n").unwrap() + 4;
    h.state_mut().script_tools.jump = Some((key, after_prefix));
    h.run();
    assert_eq!(script_cursor(&h, key), Some((after_prefix, after_prefix)));
    h.key_press(egui::Key::F2);
    h.run();
    let c = h.state().script_tools.completion.clone().expect("completion list");
    assert_eq!(c.items[0].name, "MyHelper");
    h.key_press(egui::Key::Enter);
    h.run();
    let edited = h.state().script_text(key).unwrap();
    assert!(edited.contains("    MyHelper(\n"), "{edited:?}");
    // The cursor moved off the word (Home) closes the list: Enter is the
    // editor's again (accepting there replaced backwards, and panicked).
    let after_prefix = edited.find("MyHelper(\n").unwrap() + 4;
    h.state_mut().script_tools.jump = Some((key, after_prefix));
    h.run();
    h.key_press(egui::Key::F2);
    h.run();
    assert!(h.state().script_tools.completion.is_some());
    let line = edited[..after_prefix].rfind('\n').unwrap() + 1;
    h.state_mut().script_tools.jump = Some((key, line));
    h.run();
    assert!(h.state().script_tools.completion.is_none(), "closed off its word");
    h.key_press(egui::Key::Enter);
    h.run();
    let edited = h.state().script_text(key).unwrap();
    assert!(edited.contains("\n\n    MyHelper(\n"), "a new line, nothing completed: {edited:?}");

    // Find (F3) selects the next match.
    h.state_mut().script_tools.search.find = "main".into();
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.key_press(egui::Key::F3);
    h.run();
    let (a, b) = script_cursor(&h, key).unwrap();
    let start = edited.find("main").unwrap();
    assert_eq!((a.min(b), a.max(b)), (start, start + 4));

    // F3 from the Find Text window's field, which keeps the keyboard
    // after a search (Sacha's report): the next match, as in Aurora.
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run();
    h.get_by_label("Find What");
    h.state_mut().script_tools.search.find = "My".into();
    h.run();
    // F3 straight from the field, the text just typed (no Enter).
    h.key_press(egui::Key::F3);
    h.run();
    let first = edited.find("My").unwrap();
    let (a, b) = script_cursor(&h, key).unwrap();
    assert_eq!((a.min(b), a.max(b)), (first, first + 2), "F3 from the field finds the first");
    h.key_press(egui::Key::F3);
    h.run();
    let second = first + 2 + edited[first + 2..].find("My").unwrap();
    let (a, b) = script_cursor(&h, key).unwrap();
    assert_eq!((a.min(b), a.max(b)), (second, second + 2), "F3 finds the next");
    h.state_mut().script_tools.search.open = false;
    h.run();
    h.state_mut().script_tools.jump = Some((key, start));
    h.run();

    // Bookmark (F5) on that line.
    h.key_press(egui::Key::F5);
    h.run();
    assert!(h.state().script_bookmarks(key).contains(&1));

    // Numbered bookmark 2 on that line (Ctrl+Shift+2), then back to it from
    // the top (Ctrl+2).
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Num2);
    h.run();
    assert_eq!(h.state().script_numbered_bookmarks(key), [(2, 1)]);
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Num2);
    h.run();
    h.run();
    let line1 = edited.find('\n').unwrap() + 1;
    assert_eq!(script_cursor(&h, key), Some((line1, line1)));

    // Saving the module stores the editor's text. (File › Save: Ctrl+S
    // with the editor's keys is the script's own Save.)
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let saved = Module::open(&path).unwrap();
    assert!(String::from_utf8_lossy(saved.get(&key).unwrap()).contains("MyHelper("));
}

/// The Save key (Ctrl+S) while a script editor has the keyboard is that
/// script's Save: its text into the module, which stays unsaved and its
/// file as it was (El Grillo's request; Aurora's script editor's Ctrl+S).
/// With the keyboard elsewhere it saves the module; a Save key chosen in
/// Options › Keyboard does the same.
/// A function added to an include in its own tab is one its includers
/// complete, though their own text did not change.
#[test]
fn an_include_edited_in_its_tab_is_what_its_includers_see() {
    let dir = mg_testkit::scratch_dir("ui-script-include-edited");
    let path = sample_module(&dir);
    let nss = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(nss("inc_a"), b"int Old() { return 1; }\n".to_vec());
    m.set(nss("uses_a"), b"#include \"inc_a\"\nvoid main() { }\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = window_harness(app);
    let names = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let symbols = h.state().script_tools.symbols.get(&nss("uses_a"));
        symbols.map(|(_, s)| s.iter().map(|s| s.name.clone()).collect()).unwrap_or_default()
    };
    let open = |h: &mut Harness<'_, Moonglow>, name: &str| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss(name))));
        settle(h);
    };
    open(&mut h, "uses_a");
    assert!(names(&h).contains(&"Old".to_string()), "{:?}", names(&h));
    open(&mut h, "inc_a");
    h.state_mut().script_tools.jump = Some((nss("inc_a"), 0));
    settle(&mut h);
    h.event(egui::Event::Text("int New() { return 2; }\n".into()));
    settle(&mut h);
    assert!(h.state().script_text(nss("inc_a")).unwrap().starts_with("int New()"));
    open(&mut h, "uses_a");
    assert!(names(&h).contains(&"New".to_string()), "{:?}", names(&h));
}

#[test]
fn the_save_key_in_a_script_editor_saves_the_script_not_the_module() {
    use mg_ui::keys::Cmd;
    let dir = mg_testkit::scratch_dir("ui-script-save-key");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    let on_disk = || String::from_utf8(Module::open(&path).unwrap().get(&key).unwrap().to_vec());
    let in_module = |h: &Harness<'_, Moonglow>| {
        let ws = h.state().ws.as_ref().unwrap();
        String::from_utf8_lossy(ws.module.get(&key).unwrap()).into_owned()
    };
    let unsaved = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().is_modified();
    // Typed at the top, with the editor's keys.
    // (Steps, not `run`: the check of a script typed in waits half a second
    // for a pause, and on a slow machine the rest of that wait is shorter
    // than a step, which egui asks to repaint at once.)
    let type_in = |h: &mut Harness<'_, Moonglow>, text: &str| {
        h.state_mut().script_tools.jump = Some((key, 0));
        h.run_steps(3);
        h.event(egui::Event::Text(text.into()));
        h.run_steps(3);
    };
    let leave = |h: &mut Harness<'_, Moonglow>| {
        h.ctx.memory_mut(|m| m.surrender_focus(egui::Id::new(("script", key))));
        h.run();
    };
    type_in(&mut h, "// one\n");
    assert!(h.state().script_text(key).unwrap().starts_with("// one\n"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run_steps(3);
    assert!(in_module(&h).starts_with("// one\n"), "the script saved into the module");
    assert_eq!(h.state().script_text(key).unwrap(), in_module(&h), "the editor's text, saved");
    assert!(unsaved(&h), "the module not saved");
    assert_eq!(on_disk().unwrap(), "void main() { }\n", "its file as it was");
    let said = h.state().script_tools.messages.last().unwrap().text.clone();
    assert!(said.contains("saved, not compiled"), "{said}");
    // The keys elsewhere: the module's Save.
    leave(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run_steps(3);
    assert!(!unsaved(&h));
    assert!(on_disk().unwrap().starts_with("// one\n"), "the module written");

    // Save on F8: F8 in the editor is the script's Save, Ctrl+S nothing.
    h.state_mut()
        .keymap
        .set(Cmd::Save, vec![egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::F8)]);
    type_in(&mut h, "// two\n");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run_steps(3);
    assert!(!in_module(&h).starts_with("// two\n"), "Ctrl+S is no Save now");
    h.key_press(egui::Key::F8);
    h.run_steps(3);
    assert!(in_module(&h).starts_with("// two\n// one\n"), "{}", in_module(&h));
    assert!(unsaved(&h));
    assert!(on_disk().unwrap().starts_with("// one\n"));
    leave(&mut h);
    h.key_press(egui::Key::F8);
    h.run_steps(3);
    assert!(!unsaved(&h));
    assert!(on_disk().unwrap().starts_with("// two\n// one\n"));
}

#[test]
fn compile_errors_go_to_their_line() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-compile-errors");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main()\n{\n    int x = 1;\n    x = y + 2;\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    apart(&h, "Compile").click();
    h.run();
    let message = h.state().script_tools.messages.last().cloned().expect("a compiler message");
    assert!(message.error);
    assert_eq!(message.location, Some(("hello".to_string(), 4)));
    // The message is in the log too; both begin with its time. (The log
    // emptied: the Compiler tab's is clicked.)
    assert!(h.state().log.entries.iter().any(|(_, m)| *m == message.text));
    h.state_mut().log.clear();
    h.run();
    h.get_by_label(&format!("{} {}", message.at, message.text)).click();
    h.run();
    h.run();
    let line4 = text.match_indices('\n').nth(2).unwrap().0 + 1;
    assert_eq!(script_cursor(&h, key), Some((line4, line4)));
}

/// A script that includes the base game's `x2_inc_cutscene` compiles, as
/// in Aurora (El Grillo's report); the Compiler tab and the log say what
/// is odd in it: a warning at the include's line, not an error.
#[test]
fn x2_inc_cutscene_compiles_with_a_warning() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-compile-warning");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"#include \"x2_inc_cutscene\"\nvoid main() {}\n".to_vec());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    apart(&h, "Compile").click();
    h.run();
    let ncs = ResKey::new(key.resref, ResType::NCS);
    assert!(h.state().ws.as_ref().unwrap().module.contains(&ncs), "compiled");
    let messages = &h.state().script_tools.messages;
    assert!(messages.iter().all(|m| !m.error), "{messages:?}");
    let warning = messages.last().expect("a warning");
    assert_eq!(warning.location, Some(("x2_inc_cutscene".to_string(), 628)));
    assert!(warning.text.contains("CutCreateObjectCopy is declared object"), "{}", warning.text);
    assert!(h.state().log.entries.iter().any(|(_, m)| *m == warning.text));
}

#[test]
fn script_wizard_writes_compiles_and_sets_scripts() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-script-wizard");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    // The new line selected (the Input Text popup off).
    app.settings.dialog_no_text_popup = true;
    app.open_module(&path);
    app.new_dialog = Some("wizdlg".into());
    // Room for the conversation's window to show its tabs' buttons.
    let mut h = harness(app, 1280.0, 900.0);
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("wizdlg", ResType::DLG).unwrap();
    h.get_by_label("Add").click();
    h.run();
    h.run();

    let window =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::Window;
    let in_wizard = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_all_by_label("Script Wizard")
            .find(window)
            .expect("the wizard")
            .get_by_label(label)
            .click();
        h.run();
    };
    let type_in_wizard = |h: &mut Harness<'_, Moonglow>, text: &str| {
        let w = h.get_all_by_label("Script Wizard").find(window).unwrap();
        let field = w
            .get_all_by_role(egui::accesskit::Role::TextInput)
            .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
            .expect("a field");
        field.click();
        h.run();
        let w = h.get_all_by_label("Script Wizard").find(window).unwrap();
        w.get_all_by_role(egui::accesskit::Role::TextInput)
            .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
            .unwrap()
            .type_text(text);
        h.run();
    };

    // A condition: female, carrying key_1. (Each of the line's two
    // scripts has its own Script Wizard…, M4's U14.)
    in_section(&h, "Text Appears When…", "Script Wizard…").click();
    h.run();
    h.run();
    for label in ["Gender", "Item In Inventory", "Next >", "Female", "Next >"] {
        in_wizard(&mut h, label);
    }
    type_in_wizard(&mut h, "key_1");
    for label in ["Add", "Next >", "Finish"] {
        in_wizard(&mut h, label);
    }
    h.run();
    let module = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().module.clone();
    let sc = String::from_utf8(
        module(&h).get(&ResKey::parse("sc_001", ResType::NSS).unwrap()).unwrap().to_vec(),
    )
    .unwrap();
    assert!(sc.contains("if(GetGender(GetPCSpeaker()) != GENDER_FEMALE)"), "{sc}");
    assert!(sc.contains("if(!HasItem(GetPCSpeaker(), \"key_1\"))"), "{sc}");
    assert!(module(&h).contains(&ResKey::parse("sc_001", ResType::NCS).unwrap()));
    let doc = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone()
    };
    let g = doc(&mut h);
    let start = &g.root.list("StartingList").unwrap()[0];
    assert_eq!(start.resref("Active").unwrap().as_str(), Some("sc_001"));

    // An action: 50 gold.
    in_section(&h, "Actions Taken", "Script Wizard…").click();
    h.run();
    h.run();
    for label in ["Give rewards", "Next >"] {
        in_wizard(&mut h, label);
    }
    type_in_wizard(&mut h, "50");
    for label in ["Next >", "Finish"] {
        in_wizard(&mut h, label);
    }
    h.run();
    let at = String::from_utf8(
        module(&h).get(&ResKey::parse("at_001", ResType::NSS).unwrap()).unwrap().to_vec(),
    )
    .unwrap();
    assert!(at.contains("\tGiveGoldToCreature(GetPCSpeaker(), 50);\r\n"), "{at}");
    let g = doc(&mut h);
    assert_eq!(
        g.root.list("EntryList").unwrap()[0].resref("Script").unwrap().as_str(),
        Some("at_001")
    );

    // One undo takes the script, its bytecode and the field back.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(!module(&h).contains(&ResKey::parse("at_001", ResType::NSS).unwrap()));
    assert!(!module(&h).contains(&ResKey::parse("at_001", ResType::NCS).unwrap()));
    let g = doc(&mut h);
    assert!(g.root.list("EntryList").unwrap()[0].resref("Script").unwrap().is_empty());
}

/// A script open with a new line typed in it (Enter at the end of a line,
/// its indent with it), the module's last step "Earlier" somewhere else
/// (`window`: the script in a window of its own): the harness, the script
/// and its text before and after the new line.
fn script_with_a_new_line(
    name: &str,
    window: bool,
) -> (Harness<'static, Moonglow>, ResKey, &'static str, String) {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main()\n{\n    int a;\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let other = ResKey::parse("mg_note", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key: other, data: Some(b"void main() {}\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Earlier", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    h.run();
    if window {
        own_window(&mut h, &Tab::Script(key), egui::pos2(400.0, 150.0), egui::vec2(600.0, 450.0));
    }
    h.state_mut().script_tools.jump = Some((key, text.find("a;").unwrap() + 2));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let edited = h.state().script_text(key).unwrap();
    assert!(edited.contains("int a;\n    \n}"), "{edited:?}");
    (h, key, text, edited)
}

/// The module's Undo and Redo (the keys somewhere else than a script's
/// editor) dropped every script editor's text: what was typed and not
/// yet saved was gone without a word (the review of November 2026, S1).
#[test]
fn the_module_s_undo_keeps_a_script_s_unsaved_text() {
    let (mut h, key, text, edited) = script_with_a_new_line("ui-undo-keeps-script-text", false);
    // The keys leave the editor: Undo is the module's.
    h.ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
    h.run();
    assert!(!script_has_keys(&h, key));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    let ws = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().is_some();
    assert!(!ws(&h), "the module's step is taken back");
    assert_eq!(
        h.state().script_text(key).as_deref(),
        Some(edited.as_str()),
        "the typed line stays"
    );
    h.state_mut().actions.push(mg_ui::Action::Redo);
    h.run();
    assert!(ws(&h));
    assert_eq!(h.state().script_text(key).as_deref(), Some(edited.as_str()));
    assert_ne!(edited, text);
}

/// The open Edit menu's Undo or Redo row (`what`), as it reads (with its
/// key).
fn edit_row(h: &Harness<'_, Moonglow>, what: &str) -> String {
    let rows: Vec<String> = h
        .query_all_by_label_contains(what)
        .filter(|n| !in_toolbar(n))
        .filter_map(|n| n.accesskit_node().label())
        .filter(|l| l.starts_with(what))
        .collect();
    assert_eq!(rows.len(), 1, "{rows:?}");
    rows[0].clone()
}

/// Whether a script's editor has the keys.
fn script_has_keys(h: &Harness<'_, Moonglow>, key: ResKey) -> bool {
    h.ctx.memory(|m| m.focused()) == Some(egui::Id::new(("script", key)))
}

/// A builder's report (October 2026): Edit › Undo and the toolbar's in a
/// script were the module's, and took back a change somewhere else,
/// where Ctrl+Z took back the script's. With the keys in the script's
/// editor (the click on the menu or the button takes them from it), they
/// are the script's, say plain "Undo" and "Redo", and give the keys back
/// to the editor, so that Ctrl+Z after them is the script's too.
///
/// Ctrl+Z alone first: a new line (with its indent) is no typed text, and
/// Ctrl+Z after it undid the module's last change, somewhere else, and
/// left the script as it was.
#[test]
fn the_keys_the_menu_s_and_the_toolbar_s_undo_in_a_script_are_the_script_s() {
    let (mut h, key, text, edited) = script_with_a_new_line("ui-menu-undo-in-script", false);
    let undoable =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_owned);
    let script = |h: &Harness<'_, Moonglow>| h.state().script_text(key).unwrap();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(script(&h), text, "the new line is taken back");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "not the module's Undo");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run();
    assert_eq!(script(&h), edited, "and Redo gives it again");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"));
    menu(&h, "Edit").click();
    h.run_steps(2);
    assert_eq!(edit_row(&h, "Undo"), "Undo Ctrl+Z", "not the module's step");
    h.get_by_label("Undo Ctrl+Z").click();
    h.run_steps(2);
    assert_eq!(script(&h), text, "the new line is taken back");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "not the module's Undo");
    assert!(script_has_keys(&h, key), "the keys back in the editor");
    menu(&h, "Edit").click();
    h.run_steps(2);
    let redo = edit_row(&h, "Redo");
    h.get_by_label(&redo).click();
    h.run_steps(2);
    assert_eq!(script(&h), edited, "and Redo gives it again");
    assert!(script_has_keys(&h, key));
    // The toolbar's, the same.
    tool(&h, "Undo").click();
    h.run_steps(2);
    assert_eq!(script(&h), text);
    assert!(script_has_keys(&h, key));
    tool(&h, "Redo").click();
    h.run_steps(2);
    assert_eq!(script(&h), edited);
    assert!(script_has_keys(&h, key));
    // Ctrl+Z after them: the script's still.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(script(&h), text);
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"));
    // With the keys elsewhere (a click in the module tree) and the
    // pointer over the script, the module's, as the Save key is: the
    // menu names the module's step. (The builder unfolds the Left Dock,
    // folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    h.get_by_label("Areas (1)").click();
    h.run_steps(2);
    assert!(!script_has_keys(&h, key));
    menu(&h, "Edit").click();
    h.run_steps(2);
    assert_eq!(edit_row(&h, "Undo"), "Undo Earlier Ctrl+Z", "the module's step named");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    let editor = h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).next().unwrap();
    h.hover_at(editor.rect().center());
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_ne!(undoable(&h).as_deref(), Some("Earlier"), "the module's step undone");
    assert_eq!(script(&h), text, "the script as it was");
}

/// Replace All and Replace are steps of the script's own Undo, and Edit ›
/// Undo with the keys still in the Replace window takes them back: it
/// took back the module's last change, somewhere else, and for an editor
/// never clicked in the script's own history had nothing to go back to.
#[test]
fn replace_all_and_replace_are_steps_of_the_script_s_undo() {
    let dir = mg_testkit::scratch_dir("ui-replace-undo");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main()\n{\n    int a = a + a;\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let other = ResKey::parse("mg_note", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key: other, data: Some(b"void main() {}\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Earlier", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    h.run();
    let undoable =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_owned);
    let script = |h: &Harness<'_, Moonglow>| h.state().script_text(key).unwrap();
    let menu_undo = |h: &mut Harness<'_, Moonglow>| {
        menu(h, "Edit").click();
        h.run_steps(2);
        assert_eq!(edit_row(h, "Undo"), "Undo Ctrl+Z", "the script's step, not the module's");
        h.get_by_label("Undo Ctrl+Z").click();
        h.run_steps(3);
    };
    // Replace… from the toolbar, the text never clicked in.
    script_click(&mut h, "Replace…");
    h.run_steps(3);
    assert!(!script_has_keys(&h, key));
    let search = &mut h.state_mut().script_tools.search;
    (search.find, search.replace) = ("a".into(), "total".into());
    search.options.whole_word = true;
    h.run_steps(2);
    h.get_by_label("Replace All").click();
    h.run_steps(3);
    let replaced = "void main()\n{\n    int total = total + total;\n}\n";
    assert_eq!(script(&h), replaced);
    menu_undo(&mut h);
    assert_eq!(script(&h), text, "Replace All taken back");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "the module's step stays");
    // Replace: the match selected is replaced, one step too.
    h.get_by_label("Find Next").click();
    h.run_steps(3);
    h.get_by_label("Replace").click();
    h.run_steps(3);
    let once = "void main()\n{\n    int total = a + a;\n}\n";
    assert_eq!(script(&h), once);
    menu_undo(&mut h);
    assert_eq!(script(&h), text, "Replace taken back");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"));
}

/// The same for a script in a window of its own.
#[test]
fn the_menu_s_undo_in_a_script_in_a_window_of_its_own_is_the_script_s() {
    let (mut h, key, text, edited) = script_with_a_new_line("ui-menu-undo-script-window", true);
    let path = h.state().dock.find_tab(&Tab::Script(key)).unwrap();
    assert!(!path.surface.is_main(), "in a window of its own");
    let undoable =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_owned);
    // (Two seconds on, the new line is a point of the editor's history
    // of its own, cursor and all: one Undo still takes it back, not
    // only the cursor.)
    h.run_steps(8);
    menu(&h, "Edit").click();
    h.run_steps(2);
    h.get_by_label(&edit_row(&h, "Undo")).click();
    h.run_steps(2);
    assert_eq!(h.state().script_text(key).as_deref(), Some(text));
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "not the module's Undo");
    tool(&h, "Redo").click();
    h.run_steps(2);
    assert_eq!(h.state().script_text(key), Some(edited));
    assert!(script_has_keys(&h, key));
}

/// Undo and Redo's keys of Options › Keyboard are the script's in its
/// editor, as Ctrl+Z is (they did nothing there, a builder's report on
/// 1.25.1): its own step, not the module's.
#[test]
fn a_rebound_undo_key_is_the_script_s_in_its_editor() {
    let (mut h, key, text, edited) = script_with_a_new_line("ui-script-undo-key", false);
    let undoable =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_owned);
    let script = |h: &Harness<'_, Moonglow>| h.state().script_text(key).unwrap();
    h.state_mut().settings.key_bindings.insert("undo".into(), vec!["Ctrl+Alt+U".into()]);
    h.state_mut().settings.key_bindings.insert("redo".into(), vec!["Ctrl+Alt+R".into()]);
    h.run();
    assert!(script_has_keys(&h, key));
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::ALT, egui::Key::U);
    h.run_steps(2);
    assert_eq!(script(&h), text, "the script's step taken back");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "not the module's");
    assert!(script_has_keys(&h, key), "the keys still in the editor");
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::ALT, egui::Key::R);
    h.run_steps(2);
    assert_eq!(script(&h), edited, "and given again");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"));
    // The menu names the new key, and its Undo is the script's still.
    menu(&h, "Edit").click();
    h.run_steps(2);
    assert_eq!(edit_row(&h, "Undo"), "Undo Ctrl+Alt+U");
}

/// A menu opened from a script's editor and closed with nothing chosen
/// gives the keys back to the editor: Ctrl+Z after it is the script's,
/// where it was the module's (the keys were nowhere).
#[test]
fn a_menu_closed_unused_gives_the_keys_back_to_the_script() {
    let (mut h, key, text, edited) = script_with_a_new_line("ui-script-menu-closed", false);
    let undoable =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_owned);
    let script = |h: &Harness<'_, Moonglow>| h.state().script_text(key).unwrap();
    // Closed by Escape.
    menu(&h, "Edit").click();
    h.run_steps(2);
    assert!(!script_has_keys(&h, key), "the click on the menu took the keys");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(script_has_keys(&h, key), "the keys back in the editor");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(script(&h), text, "the script's step");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "not the module's");
    // Closed by a click on its name again.
    h.get_by_label("File").click();
    h.run_steps(2);
    h.get_by_label("File").click();
    h.run_steps(2);
    assert!(script_has_keys(&h, key), "the keys back again");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run_steps(2);
    assert_eq!(script(&h), edited);
    // Closed by a click outside the editor (the module tree): nowhere.
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    menu(&h, "Edit").click();
    h.run_steps(2);
    h.get_by_label("Areas (1)").click();
    h.run_steps(2);
    assert!(!script_has_keys(&h, key), "a click outside the editor");
}

#[test]
fn saving_a_script_compiles_it_with_debug_information_when_chosen() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-auto-compile");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main()\n{\n    PrintString(\"hi\");\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    // Options › Script Editor.
    app.settings.auto_compile = true;
    app.settings.debug_info = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    // An edit, then Save: the script is compiled too, with its .ndb.
    h.state_mut().script_tools.jump = Some((key, text.len()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    apart(&h, "Save").click();
    h.run();
    let module = &h.state().ws.as_ref().unwrap().module;
    let compiled = |t| module.contains(&ResKey::new(key.resref, t));
    assert!(compiled(ResType::NCS) && compiled(ResType::NDB), "{:?}", h.state().log.entries);
}

/// A script in Windows-1250, as Polish builders keep theirs (GitHub issue
/// 27), with Polish the editing language: it reads as Polish, saves its
/// untouched lines byte for byte, compiles its strings' bytes as they are,
/// warns of a character the codepage lacks, and is read again in English's
/// codepage when English is chosen.
#[test]
fn a_polish_script_reads_saves_and_compiles_in_its_codepage() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-script-cp1250");
    let path = sample_module(&dir);
    let key = ResKey::parse("pl_text", ResType::NSS).unwrap();
    let polish = "Zażółć gęślą jaźń. Źródło.";
    let text = format!("void main()\n{{\n    // {polish}\n    PrintString(\"{polish}\");\n}}\n");
    let cp1250 = mg_core::Codepage::WINDOWS_1250;
    let bytes = cp1250.encode(&text).unwrap().into_owned();
    let mut m = Module::open(&path).unwrap();
    m.set(key, bytes.clone());
    m.save().unwrap();
    mg_ui::set_edit_language(Language::POLISH);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), text);
    h.get_by_label("Windows-1250");
    // A line typed at the top, then Compile (which saves): the rest is
    // the file's bytes as they were.
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.event(egui::Event::Text("// ę\n".into()));
    h.run();
    apart(&h, "Compile").click();
    h.run();
    let module = |h: &Harness<'_, Moonglow>, k: ResKey| {
        h.state().ws.as_ref().unwrap().module.get(&k).map(<[u8]>::to_vec)
    };
    let saved = module(&h, key).unwrap();
    assert_eq!(saved[..b"// ".len()], *b"// ");
    assert_eq!(saved[3..6], [0xEA, b'\n', b'v'], "ę in Windows-1250");
    assert_eq!(saved[5..], bytes[..], "the lines not typed in, byte for byte");
    // The string's bytes in the compiled script are its Windows-1250 bytes.
    let ncs = module(&h, ResKey::new(key.resref, ResType::NCS)).expect("compiled");
    let literal = cp1250.encode(polish).unwrap().into_owned();
    assert!(ncs.windows(literal.len()).any(|w| w == literal), "the string's bytes, as written");
    // A character Windows-1250 lacks: saved as ?, and said so.
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.event(egui::Event::Text("// 日\n".into()));
    h.run();
    apart(&h, "Save").click();
    h.run();
    assert!(module(&h, key).unwrap().starts_with(b"// ?\n// "));
    let warned = |h: &Harness<'_, Moonglow>| {
        h.state().log.entries.iter().any(|(_, m)| m.contains("日") && m.contains("Windows-1250"))
    };
    assert!(warned(&h), "{:?}", h.state().log.entries);
    // English chosen: the script, without unsaved changes, is read again
    // as Windows-1252 reads its bytes.
    mg_ui::set_edit_language(Language::ENGLISH);
    h.run();
    let western = mg_core::Codepage::WINDOWS_1252.decode(&module(&h, key).unwrap()).into_owned();
    assert_eq!(h.state().script_text(key).unwrap(), western);
    h.get_by_label("Windows-1252");
}

#[test]
fn build_on_save_opens_problems_when_something_is_wrong() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-build-on-save");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    m.set(ResKey::parse("mg_bad", ResType::NSS).unwrap(), b"void main() { nope }".to_vec());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.settings.build_on_save = true; // Options › General
    app.open_module(&path);
    let mut h = window_harness(app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    // (Problems, M4's U28: its results there, brought to the front with
    // the Bottom Dock open; no window.)
    assert!(h.state().build.is_none(), "no window opened");
    let results = build_results(&h);
    assert!(results.iter().any(|r| r.starts_with("Error: mg_bad.nss")), "{results:?}");
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Problems));
    h.get_by_label_contains("Error: mg_bad.nss");
    assert!(h.state().log.entries.iter().any(|(_, m)| m == "Finished Building Module"));
}

#[test]
fn tab_indents_selected_lines_in_the_script_editor() {
    let dir = mg_testkit::scratch_dir("ui-script-indent");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main()\n{\n}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    // Select into the second line, then Tab: both lines indented.
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowRight);
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    // Four spaces, unless Options › Script Editor says otherwise.
    assert_eq!(h.state().script_text(key).unwrap(), "    void main()\n    {\n}\n");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), "void main()\n{\n}\n");
    // Tabs, chosen: a tab on each line.
    h.state_mut().settings.script_style.indent_tabs = true;
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), "\tvoid main()\n\t{\n}\n");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), "void main()\n{\n}\n");
    // Tab with no lines selected: the field's own tab no longer, but
    // spaces to the next multiple of the width (two, here: from column 2).
    h.state_mut().settings.script_style.indent_tabs = false;
    h.state_mut().script_tools.jump = Some((key, 2));
    h.run();
    h.key_press(egui::Key::Tab);
    h.event(egui::Event::Text("x".into()));
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), "vo  xid main()\n{\n}\n");
    h.state_mut().settings.script_style.indent_tabs = true;
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), "vo  x\tid main()\n{\n}\n");
}

/// Tab indents as the script open is indented (tabs, or so many spaces),
/// and as Options › Script Editor says where it has no indentation to go
/// by; Edit › Convert Indentation to Tabs and to Spaces (and the editor's
/// context menu) make every line so, one step for the editor's Undo
/// (Jasperre's report).
#[test]
fn tab_indents_as_the_script_does() {
    let dir = mg_testkit::scratch_dir("ui-script-indent-by-file");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let open = |text: &str| {
        let mut m = Module::open(&path).unwrap();
        m.set(key, text.as_bytes().to_vec());
        m.save().unwrap();
        let mut app = app_with(Vec::new());
        app.settings.no_last_area = true;
        app.open_module(&path);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        let mut h = harness(app, 1200.0, 800.0);
        h.run();
        h.state_mut().script_tools.jump = Some((key, 0));
        h.run();
        h
    };
    let tab_at_start = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().script_tools.jump = Some((key, 0));
        h.run();
        h.key_press(egui::Key::Tab);
        h.run();
        let text = h.state().script_text(key).unwrap();
        text[..text.find('v').unwrap()].to_string()
    };
    // Options say four spaces; the scripts say otherwise.
    let tabbed = "void main()\n{\n\tint n;\n\tif (n)\n\t\tn++;\n}\n";
    let mut h = open(tabbed);
    assert!(!h.state().settings.script_style.indent_tabs);
    assert_eq!(tab_at_start(&mut h), "\t");
    let two = "void main()\n{\n  int n;\n  if (n)\n    n++;\n}\n";
    let mut h = open(two);
    assert_eq!(tab_at_start(&mut h), "  ");
    let mut h = open("void main()\n{\n}\n");
    assert_eq!(tab_at_start(&mut h), "    ", "nothing to go by: as Options say");

    // Converted from the Edit menu: one step to undo.
    let mut h = open(two);
    menu(&h, "Edit").click();
    h.run_steps(2);
    h.get_by_label("Convert Indentation to Tabs").click();
    h.run_steps(3);
    let converted = "void main()\n{\n\tint n;\n\tif (n)\n\t\tn++;\n}\n";
    assert_eq!(h.state().script_text(key).unwrap(), converted);
    // (Back in the editor, as a click there would put it.)
    h.state_mut().script_tools.jump = Some((key, 3));
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), two, "taken back in one");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), converted);
    // Tab indents with a tab from then on.
    assert_eq!(tab_at_start(&mut h), "\t");
    // Taken back with the editor's Undo, Tab indents as before it (two
    // spaces), not as converted.
    let mut h = open(two);
    menu(&h, "Edit").click();
    h.run_steps(2);
    h.get_by_label("Convert Indentation to Tabs").click();
    h.run_steps(3);
    h.state_mut().script_tools.jump = Some((key, 3));
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), two);
    assert_eq!(tab_at_start(&mut h), "  ", "as before the conversion");

    // With lines selected (the editor's context menu): those lines alone.
    let mut h = open(two);
    let id = egui::Id::new(("script", key));
    let (from, to) = (two.find("  if").unwrap(), two.find("    n++").unwrap() + 2);
    let mut state = egui::text_edit::TextEditState::load(&h.ctx, id).unwrap_or_default();
    let range =
        egui::text::CCursorRange::two(egui::text::CCursor::new(from), egui::text::CCursor::new(to));
    state.cursor.set_char_range(Some(range));
    state.store(&h.ctx, id);
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.run_steps(2);
    // (Shift+F10: the menu at the caret, the selection kept. A right
    // click at the field's middle is outside it and places the caret.)
    press_now(&mut h, egui::Modifiers::SHIFT, egui::Key::F10);
    h.run_steps(2);
    h.get_by_label("Convert Indentation to Tabs").click();
    h.run_steps(3);
    assert_eq!(
        h.state().script_text(key).unwrap(),
        "void main()\n{\n  int n;\n\tif (n)\n\t\tn++;\n}\n",
        "the line before the selection as it was"
    );

    // From the editor's context menu: to spaces, Options' four.
    let mut h = open(tabbed);
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("void main")))
        .expect("the script's text")
        .click_secondary();
    h.run_steps(2);
    h.get_by_label("Convert Indentation to Spaces").click();
    h.run_steps(3);
    assert_eq!(
        h.state().script_text(key).unwrap(),
        "void main()\n{\n    int n;\n    if (n)\n        n++;\n}\n"
    );
    assert_eq!(tab_at_start(&mut h), "    ");
}

/// A CRLF script (BioWare's are, and Moonglow's new ones) in the editor:
/// no `\r` at the lines' ends for End to land after and typing to go
/// past (Jasperre's report: "trailing spaces" on every line); untouched,
/// the module keeps its bytes; edited and saved, CRLF everywhere (the
/// line typed and the text pasted too), no bare newline.
#[test]
fn a_crlf_script_has_no_carriage_returns_in_its_editor() {
    let dir = mg_testkit::scratch_dir("ui-script-crlf");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let file = b"void main()\r\n{\r\n    int n;\r\n}\r\n".to_vec();
    let mut m = Module::open(&path).unwrap();
    m.set(key, file.clone());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    let module = |h: &Harness<'_, Moonglow>| {
        h.state().ws.as_ref().unwrap().module.get(&key).map(<[u8]>::to_vec)
    };
    assert_eq!(h.state().script_text(key).unwrap(), "void main()\n{\n    int n;\n}\n");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert_eq!(module(&h).unwrap(), file, "untouched: the same bytes");
    let on_disk = Module::open(&path).unwrap().get(&key).map(<[u8]>::to_vec);
    assert_eq!(on_disk.unwrap(), file, "and saved so");
    // End on the first line, and a letter: at the line's end.
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("x".into()));
    h.run();
    assert!(h.state().script_text(key).unwrap().starts_with("void main()x\n{"));
    // A new line, and a paste of Windows text.
    h.key_press(egui::Key::Enter);
    h.event(egui::Event::Paste("// a\r\n// b".into()));
    h.run();
    let text = h.state().script_text(key).unwrap();
    assert!(!text.contains('\r'), "{text:?}");
    apart(&h, "Save").click();
    h.run();
    let saved = module(&h).unwrap();
    assert_eq!(saved, b"void main()x\r\n// a\r\n// b\r\n{\r\n    int n;\r\n}\r\n");
}

/// Save with compiling on save off says so in the Compiler tab, once, at
/// the time it was said (Jasperre's report: the last compile's "compiled"
/// read as the saved text's, and two compiles in a row looked alike).
#[test]
fn a_save_that_does_not_compile_says_so() {
    let dir = mg_testkit::scratch_dir("ui-script-save-note");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main()\n{\n}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    assert!(!h.state().settings.auto_compile);
    for letter in ["a", "b"] {
        h.state_mut().script_tools.jump = Some((key, 0));
        h.run();
        h.event(egui::Event::Text(letter.into()));
        h.run();
        apart(&h, "Save").click();
        h.run();
    }
    let said: Vec<_> = h.state().script_tools.messages.iter().filter(|m| m.script == key).collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].text.contains("saved, not compiled"), "{}", said[0].text);
    assert!(said[0].text.contains("Automatically Compile Scripts on Save"));
    let at = said[0].at.as_bytes();
    assert!(
        at.len() == 8
            && at[2] == b':'
            && at[5] == b':'
            && at.iter().filter(|b| b.is_ascii_digit()).count() == 6,
        "HH:MM:SS: {:?}",
        said[0].at
    );
    h.get_by_label_contains("saved, not compiled");
    assert_eq!(h.state().script_tools.info, mg_ui::script_tools::InfoTab::Compiler);
}

/// File › Save (Ctrl+S), which stores the script editors' unsaved text
/// too, says in the Compiler tab that it was saved and not compiled, as
/// the script's own Save does, with compiling on save off (Jasperre's
/// report).
#[test]
fn the_module_s_save_says_a_script_was_not_compiled() {
    let dir = mg_testkit::scratch_dir("ui-script-module-save-note");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main()\n{\n}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    // Nothing typed: nothing said of the script.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(h.state().script_tools.messages.is_empty());
    h.state_mut().script_tools.info = mg_ui::script_tools::InfoTab::Bookmarks;
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.event(egui::Event::Text("a".into()));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let saved = Module::open(&path).unwrap().get(&key).map(<[u8]>::to_vec);
    assert_eq!(saved.unwrap(), b"avoid main()\n{\n}\n");
    let said: Vec<_> = h.state().script_tools.messages.iter().filter(|m| m.script == key).collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].text.contains("hello.nss: saved, not compiled"), "{}", said[0].text);
    assert_eq!(h.state().script_tools.info, mg_ui::script_tools::InfoTab::Compiler);
    h.get_by_label_contains("saved, not compiled");
}

/// Help shows a name's documentation and declaration verbatim, as the
/// source has them (5.2.0, the owner's ruling on Jasperre's report, as
/// 4.2.1 had it): the comment with its `//`, its spaces and its columns,
/// then the declaration on its line, not wrapped (wider than the pane,
/// which scrolls); under the text in either mode (the Bottom Dock, the
/// script's window's bottom pane), brought there by a click on the name
/// in Reference.
#[test]
fn help_shows_the_comment_and_the_declaration_as_written() {
    for mode in MODES {
        help_as_written(mode);
    }
}

fn help_as_written(mode: Mode) {
    let dir = mg_testkit::scratch_dir(&mode.dir("ui-script-help"));
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let long = "// - nB: the second number, which is added to the first one and given back \
                with it as the sum, a line longer than the pane is wide, as nwscript.nss has some";
    let text = format!(
        "// Adds.\r\n// 0-10   :  Low.\r\n// 11-89  :  Middle.\r\n// - nA:    the first\r\n\
         //           (any)\r\n{long}\r\n// * Returns the sum.\r\n\
         int Add(int nA, int nB) {{ return nA; }}\r\nvoid main() {{ Add(1, 2); }}\r\n"
    );
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_in(mode, Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, 1200.0, 800.0);
    settle(&mut h);
    reference_row(&h, "Add").click();
    settle(&mut h);
    assert_eq!(h.state().script_tools.help.as_ref().map(|s| s.name.as_str()), Some("Add"));
    match mode.tabs() {
        true => assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Help)),
        false => {
            let pane = h.state().window_bottom_for_test(&Tab::Script(key));
            assert_eq!(pane, Some(mg_ui::Panel::Help), "the window's bottom pane");
        }
    }
    let want = format!(
        "// Adds.\n// 0-10   :  Low.\n// 11-89  :  Middle.\n// - nA:    the first\n\
         //           (any)\n{long}\n// * Returns the sum.\nint Add(int nA, int nB)"
    );
    let shown = h.get_by_label(&want).rect();
    // Not wrapped: a row a line, the long one past the pane's edge.
    let help = h.get_by_label("Help's text").rect();
    assert!(shown.width() > help.width(), "{mode:?}: {shown:?} wrapped in {help:?}");
    let rows = want.lines().count() as f32;
    assert!(shown.height() < rows * 24.0, "{mode:?}: {rows} rows in {shown:?}");
}

/// A `/* … */` comment directly above a function is its documentation in
/// Help, as BioWare's includes put theirs after a `//::` line (Jasperre's
/// report), shown as written with its markers (5.2.0, as 4.2.1 showed
/// it); above the definition, where the prototype has none.
#[test]
fn help_takes_a_block_comment_as_documentation() {
    let dir = mg_testkit::scratch_dir("ui-script-help-block");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "int Hidden(object oSelf);\r\n\r\n//:: Hidden\r\n/*\r\n    Returns TRUE if oSelf is hidden\r\n*/\r\n\
                int Hidden(object oSelf) { return FALSE; }\r\nvoid main() { Hidden(OBJECT_SELF); }\r\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, 1200.0, 800.0);
    settle(&mut h);
    reference_row(&h, "Hidden").click();
    settle(&mut h);
    h.get_by_label(
        "//:: Hidden\n/*\n    Returns TRUE if oSelf is hidden\n*/\nint Hidden(object oSelf)",
    );
}

#[test]
fn page_keys_move_through_the_script_editor() {
    // Sacha's report: Page Up and Page Down did nothing in the script editor.
    let dir = mg_testkit::scratch_dir("ui-script-page");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text: String = (0..200).map(|n| format!("// line {n}\n")).collect();
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.into_bytes());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    let line_of = |h: &Harness<'_, Moonglow>, mark: &str| {
        let t = h.state().script_text(key).unwrap();
        t.lines().position(|l| l.starts_with(mark)).expect(mark)
    };
    // Two pages down: as many lines on, at the start of a line.
    h.key_press(egui::Key::PageDown);
    h.run();
    h.key_press(egui::Key::PageDown);
    h.run();
    h.event(egui::Event::Text("X".into()));
    h.run();
    let down = line_of(&h, "X");
    assert!(down >= 10 && down % 2 == 0, "two pages down: line {down}");
    // A page up from before the "X": half way back, in the same column.
    h.key_press(egui::Key::ArrowLeft);
    h.key_press(egui::Key::PageUp);
    h.run();
    h.event(egui::Event::Text("Y".into()));
    h.run();
    assert_eq!(line_of(&h, "Y"), down / 2);
    // Shift selects: a page down from there, typed over, takes the lines
    // between with it.
    h.key_press(egui::Key::ArrowLeft);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::PageDown);
    h.run();
    h.event(egui::Event::Text("Z".into()));
    h.run();
    let t = h.state().script_text(key).unwrap();
    assert!(!t.contains('Y') && t.lines().nth(down / 2).unwrap().starts_with("ZX"), "{t:.200}");
    // Past the first line, the start of the text.
    for _ in 0..5 {
        h.key_press(egui::Key::PageUp);
        h.run();
    }
    h.event(egui::Event::Text("W".into()));
    h.run();
    assert!(h.state().script_text(key).unwrap().starts_with("W// line 0"));
}

#[test]
fn the_external_script_editor_s_saves_come_back() {
    let Some(editor) = true_program() else {
        eprintln!("skipped: no `true` program to stand in for an editor");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-external-editor");
    let path = sample_module(&dir);
    // (A script of its own: the copies for the external editor are in the
    // process's one scratch folder by the script's name, and other tests
    // open `hello` there at the same time.)
    let key = ResKey::parse("saves_back", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main() {}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.external_editor = Some(editor.into());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run_steps(3);
    script_click(&mut h, "External Editor");
    h.run_steps(3);
    // The editor's file: the script; an edit saved there comes back.
    // (In the run's own folder, of a name no one can guess: not the
    // system's temporary folder's `moonglow-PID`, which another user of
    // the machine could make first. The last pass's item 24.)
    let guessable = std::env::temp_dir().join(format!("moonglow-{}", std::process::id()));
    assert!(!guessable.join("saves_back.nss").exists());
    let file = mg_module::build::own_scratch_dir().unwrap().join("edit").join("saves_back.nss");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "void main() {}\n");
    std::fs::write(&file, "void main() { int n; }\n").unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(later).unwrap();
    h.run_steps(3);
    assert_eq!(h.state().script_text(key).unwrap(), "void main() { int n; }\n");
}

/// Find References' script strings show their lines in the scripts'
/// codepage (Options › Language: Windows-1250 for Polish, where 0xB9 is
/// "ą"; GitHub issue 27), read again when the language changes.
#[test]
fn references_show_a_script_s_line_in_the_scripts_codepage() {
    use mg_core::Language;
    use mg_ui::references::Query;
    let dir = mg_testkit::scratch_dir("ui-references-codepage");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let source = b"void main() { SpeakString(\"Id\xb9\"); ExecuteScript(\"next_one\"); }\n";
    m.set(ResKey::parse("polish", ResType::NSS).unwrap(), source.to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.find_references(Query::Resource(ResKey::parse("next_one", ResType::NSS).unwrap()));
    let mut h = window_harness(app);
    let line = |h: &mut Harness<'_, Moonglow>, language: Language| {
        mg_ui::set_edit_language(language);
        h.run_steps(2);
        h.state().references.mentions.first().map(|x| x.text.clone()).unwrap_or_default()
    };
    let polish = line(&mut h, Language::POLISH);
    let western = line(&mut h, Language::ENGLISH);
    assert_eq!(polish, "void main() { SpeakString(\"Idą\"); ExecuteScript(\"next_one\"); }");
    assert_eq!(western, "void main() { SpeakString(\"Id¹\"); ExecuteScript(\"next_one\"); }");
}

/// A resource whose name has a letter outside ASCII (`caf\xE9_01`) is
/// found where a script's string spells it: the name is asked for as
/// its own bytes. As text, the letter was two bytes and the string was
/// found nowhere, in Find References, in what a rename counts and in
/// what a delete says still names it.
#[test]
fn a_name_outside_ascii_is_found_spelled_in_a_script() {
    use mg_ui::references::Query;
    let dir = mg_testkit::scratch_dir("ui-references-name-outside-ascii");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let source = b"void main() { CreateObject(1, \"Caf\xe9_01\", GetLocation(OBJECT_SELF)); }\n";
    let script = ResKey::parse("spawner", ResType::NSS).unwrap();
    m.set(script, source.to_vec());
    let cafe = ResKey::new(mg_core::ResRef::from_bytes(b"caf\xe9_01").unwrap(), ResType::UTC);
    m.set(cafe, mg_gff::Gff::new(*b"UTC ").to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.find_references(Query::Resource(cafe));
    let mut h = window_harness(app);
    settle(&mut h);
    let found = h.state().references.mentions.clone();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].script, script);
}

#[test]
fn a_script_named_as_the_game_s_own_is_warned_about() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-standard-warning");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let add = |name: &str| {
        mg_ui::Action::Apply(mg_edit::Command::new(
            "add",
            vec![mg_edit::Edit::SetResource {
                key: ResKey::parse(name, ResType::NSS).unwrap(),
                data: Some(b"void main() {}".to_vec()),
            }],
        ))
    };
    app.actions.push(add("nw_c2_default9"));
    app.actions.push(add("mg_mine"));
    let mut h = window_harness(app);
    h.run();
    let warned: Vec<&String> = h
        .state()
        .log
        .entries
        .iter()
        .filter(|(l, _)| *l == mg_ui::Level::Warning)
        .map(|(_, m)| m)
        .collect();
    assert!(
        warned.iter().any(|m| m.contains("nw_c2_default9.nss replaces the game's own")),
        "{warned:?}"
    );
    assert!(!warned.iter().any(|m| m.contains("mg_mine")), "{warned:?}");
}

#[test]
fn print_opens_a_highlighted_page_of_the_script() {
    let dir = mg_testkit::scratch_dir("ui-script-print");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main() { int a = 1 < 2; }\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.print_dir = Some(dir.join("print"));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    script_click(&mut h, "Print…");
    h.run();
    let page = std::fs::read_to_string(dir.join("print").join("hello.html")).unwrap();
    assert!(page.contains(" &lt; ") && page.contains("window.print()"), "{page}");
    assert!(page.contains(">int</span>"), "keywords are coloured spans");
}

/// Edit beside a script's name makes a script that is nowhere (as Aurora
/// does) and opens it; the module's and the game's open as they are.
#[test]
fn edit_beside_a_script_s_name_creates_a_missing_script() {
    let Some((mut h, _)) = area_ui_harness("edit-script") else { return };
    let name = ResRef::from_str("fresh_one").unwrap();
    let key = ResKey::new(name, ResType::NSS);
    let has = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().module.contains(&key);
    assert!(!has(&h));
    h.state_mut().actions.push(mg_ui::Action::EditScript { name, condition: false });
    h.run_steps(3);
    assert!(has(&h), "made in the module");
    assert!(h.state().dock.find_tab(&Tab::Script(key)).is_some(), "and opened");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("New script fresh_one.nss"));
    // Again: opened, not made anew.
    h.state_mut().actions.push(mg_ui::Action::EditScript { name, condition: false });
    h.run_steps(3);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("New script fresh_one.nss"));
    // A condition starts as one.
    let name = ResRef::from_str("fresh_check").unwrap();
    h.state_mut().actions.push(mg_ui::Action::EditScript { name, condition: true });
    h.run_steps(3);
    let ws = h.state().ws.as_ref().unwrap();
    let text = ws.module.get(&ResKey::new(name, ResType::NSS)).unwrap();
    assert!(text.starts_with(b"int StartingConditional()"));
    // The game's own script is shown, not copied into the module.
    let name = ResRef::from_str("nw_c2_default1").unwrap();
    h.state_mut().actions.push(mg_ui::Action::EditScript { name, condition: false });
    h.run_steps(3);
    let key = ResKey::new(name, ResType::NSS);
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&key));
    assert!(h.state().dock.find_tab(&Tab::Resource(key)).is_some());
}

/// Options › Script Editor › Open scripts in the external editor: a
/// script opened from the module tree goes to the external editor too.
#[cfg(unix)]
#[test]
fn a_script_opened_from_the_tree_goes_to_the_external_editor() {
    let Some((mut h, _)) = area_ui_harness("script-external") else { return };
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key, data: Some(b"void main() { }\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    let Some(editor) = true_program() else { return };
    h.state_mut().settings.external_editor = Some(editor.clone().into());
    h.run_steps(3);
    // (As a double click on it in the module tree does.)
    let open = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::OpenResource(key));
        h.run_steps(4);
    };
    let said = |h: &Harness<'_, Moonglow>| {
        h.state()
            .log
            .entries
            .iter()
            .filter(|e| e.1.contains(&format!("editing in {editor}")))
            .count()
    };
    // Without the option: Moonglow's editor alone.
    open(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Script(key)).is_some());
    assert_eq!(said(&h), 0);
    // With it: the external editor is started as well.
    let tab = h.state().dock.find_tab(&Tab::Script(key)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().settings.scripts_external = true;
    h.run_steps(2);
    open(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Script(key)).is_some());
    assert_eq!(said(&h), 1, "{:?}", h.state().log.entries.last());
}

/// Open Script, Save As and New Script open a script as the module tree
/// does: in the external editor too, where Options has scripts open there.
#[cfg(unix)]
#[test]
fn the_script_editor_s_openers_go_to_the_external_editor_too() {
    let Some((mut h, _)) = area_ui_harness("script-external-openers") else { return };
    h.state_mut().settings.tree_stays = true;
    let nss = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let edits = [nss("hello"), nss("other_one")]
        .map(|key| mg_edit::Edit::SetResource { key, data: Some(b"void main() { }\n".to_vec()) })
        .to_vec();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    let Some(editor) = true_program() else { return };
    h.state_mut().settings.external_editor = Some(editor.clone().into());
    h.state_mut().settings.scripts_external = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("hello"))));
    h.run_steps(4);
    let said = |h: &Harness<'_, Moonglow>, name: &str| {
        let line = format!("{name}.nss: editing in {editor}");
        h.state().log.entries.iter().filter(|e| e.1 == line).count()
    };
    assert_eq!(said(&h, "hello"), 0, "a tab opened by itself: Moonglow's editor alone");
    // Open Script.
    script_click(&mut h, "Open…");
    h.run_steps(3);
    let at = h.get_by_label("other_one").rect().center();
    double_click_now(&mut h, at);
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Script(nss("other_one"))).is_some());
    assert_eq!(said(&h, "other_one"), 1, "{:?}", h.state().log.entries);
    // New Script.
    h.state_mut().new_script = Some("fresh".into());
    h.run_steps(2);
    h.get_by_label("Create").click();
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Script(nss("fresh"))).is_some());
    assert_eq!(said(&h, "fresh"), 1, "{:?}", h.state().log.entries);
    // Save As, from the script in front.
    script_click(&mut h, "Save As…");
    h.run_steps(2);
    let window =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::Window;
    let w = h.get_all_by_label("Save Script As").find(window).expect("the window");
    w.get_all_by_role(egui::accesskit::Role::TextInput)
        .find(|n| n.accesskit_node().label().as_deref() != Some("Quick Open"))
        .unwrap()
        .type_text("fresh_copy");
    h.run_steps(1);
    let w = h.get_all_by_label("Save Script As").find(window).unwrap();
    w.get_by_label("Save").click();
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Script(nss("fresh_copy"))).is_some());
    assert_eq!(said(&h, "fresh_copy"), 1, "{:?}", h.state().log.entries);
}

/// Compile and the check as you type read an include alike: as the module
/// has it, not its editor's unsaved text (the compiled script goes into
/// the module beside its sources); Compile says the include's changes
/// were left out.
#[test]
fn compile_and_the_live_check_read_an_include_as_saved() {
    let Some((mut h, _)) = area_ui_harness("include-as-saved") else { return };
    let nss = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let edits = vec![
        mg_edit::Edit::SetResource {
            key: nss("inc_n"),
            data: Some(b"int N() { return 1; }\n".into()),
        },
        mg_edit::Edit::SetResource {
            key: nss("uses_n"),
            data: Some(b"#include \"inc_n\"\nvoid main() { int n = N(); }\n".into()),
        },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    // The include changed in its editor, not saved: N gone, P new.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("inc_n"))));
    h.run_steps(3);
    h.state_mut().scripts_mut_for_test(nss("inc_n"), "int P() { return 1; }\n");
    h.run_steps(1);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("uses_n"))));
    h.run_steps(3);
    let checked = |h: &mut Harness<'_, Moonglow>| {
        std::thread::sleep(std::time::Duration::from_millis(600));
        settle(h);
        h.state().script_error_for_test(nss("uses_n"))
    };
    let compiled = |h: &mut Harness<'_, Moonglow>| {
        apart(h, "Compile").click();
        h.run_steps(3);
        let messages = &h.state().script_tools.messages;
        !messages.iter().any(|m| m.script == nss("uses_n") && m.error)
    };
    // N, as saved: both pass, and Compile says the include's changes were
    // left out.
    assert_eq!(checked(&mut h), None);
    assert!(compiled(&mut h), "{:?}", h.state().script_tools.messages);
    let left_out = "uses_n.nss: inc_n read as saved, not with the changes in its editor (save to compile with them)";
    assert!(h.state().script_tools.messages.iter().any(|m| m.text == left_out));
    // P, not saved: both fail.
    h.state_mut()
        .scripts_mut_for_test(nss("uses_n"), "#include \"inc_n\"\nvoid main() { int n = P(); }\n");
    h.run_steps(1);
    assert!(checked(&mut h).is_some(), "the check as you type fails");
    assert!(!compiled(&mut h), "and so does Compile");
}

#[test]
fn script_navigation_definition_references_rename_and_live_errors() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-script-nav");
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("NAV"));
    m.set_info(&info).unwrap();
    let inc = ResKey::parse("inc_util", ResType::NSS).unwrap();
    let main = ResKey::parse("main_a", ResType::NSS).unwrap();
    // (CRLF, as BioWare's and Aurora's scripts are: the places found are
    // in the editor's text, with plain newlines, and the rename keeps the
    // files' line ends.)
    m.set(inc, b"// Doubles a number.\r\nint Twice(int n) { return n * 2; }\r\n".to_vec());
    let crlf = b"#include \"inc_util\"\r\nvoid main()\r\n{\r\n    int x = Twice(2);\r\n}\r\n";
    m.set(main, crlf.to_vec());
    let path = dir.join("nav.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(main)));
    h.run_steps(3);

    // Definition: Twice is in the include.
    let text = "#include \"inc_util\"\nvoid main()\n{\n    int x = Twice(2);\n}\n";
    let at = text.find("Twice").unwrap() + 2;
    let d = h.state_mut().declaration_at(main, at).expect("a declaration");
    assert_eq!((d.at.file.as_str(), d.doc.as_str()), ("inc_util", "Doubles a number."));
    h.state_mut().go_to_declaration(&d);
    h.run_steps(3);
    assert!(h.state().dock.find_tab(&Tab::Script(inc)).is_some(), "the include opens");

    // References: its definition and its use.
    h.state_mut().show_references(&d, None);
    let results: Vec<String> = (h.state().search_results.hits.iter())
        .map(|Hit::Line { script, line, .. }| format!("{}:{line}", script.resref))
        .collect();
    assert_eq!(results, ["inc_util:1", "main_a:3"]);

    // Rename: both scripts, one undoable step.
    h.state_mut().rename_symbol(&d, "Double");
    h.run_steps(3);
    let text_of = |h: &Harness<'_, Moonglow>, k: ResKey| {
        String::from_utf8(h.state().ws.as_ref().unwrap().module.get(&k).unwrap().to_vec()).unwrap()
    };
    assert!(text_of(&h, inc).contains("int Double(int n)"));
    assert_eq!(
        text_of(&h, main),
        "#include \"inc_util\"\r\nvoid main()\r\n{\r\n    int x = Double(2);\r\n}\r\n"
    );
    assert!(!text_of(&h, inc).replace("\r\n", "").contains('\n'), "CRLF kept");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Rename Twice to Double"));

    // Errors as you type: checked once typing pauses, the line shown.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(main)));
    settle(&mut h);
    // (Where the text begins: it does not move as an error comes and
    // goes, Jasperre's report; since M4's U13 the error is the Status
    // Bar's message and a wavy underline, no row of its own.)
    let editor = egui::Id::new(("script", main));
    let top =
        |h: &Harness<'_, Moonglow>| h.ctx.read_response(editor).expect("the editor").rect.top();
    let before = top(&h);
    assert!(h.state().script_error_for_test(main).is_none());
    h.state_mut().scripts_mut_for_test(
        main,
        "#include \"inc_util\"\nvoid main()\n{\n    int x = Double(2) +;\n}\n",
    );
    h.run_steps(1);
    std::thread::sleep(std::time::Duration::from_millis(600));
    settle(&mut h);
    let error = h.state().script_error_for_test(main);
    assert_eq!(error.as_ref().map(|e| e.0), Some(Some(3)), "{error:?}");
    // (Its wavy underline, M4's U13: no row of its own under the header.)
    let error_color = mg_uikit::tokens(&h.ctx).error;
    assert_eq!(waves(&h, error_color).len(), 1, "a wavy underline on its line");
    assert_eq!(top(&h), before, "the text stays where it was");
    // Options › Script Editor: no checking as you type.
    h.state_mut().settings.no_live_check = true;
    h.run_steps(2);
    assert!(waves(&h, error_color).is_empty(), "no underline");
    assert!(h.state().script_error_for_test(main).is_none(), "nor its error");
    assert_eq!(top(&h), before, "and the text where it was");
}

/// A script's laid-out text (a galley a line) is kept while its tab is
/// open and no longer: it stayed, for every script ever opened, until
/// the module was closed.
#[test]
fn a_closed_script_keeps_no_laid_out_text() {
    let dir = mg_testkit::scratch_dir("ui-script-laid-out");
    let path = sample_module(&dir);
    let keys = ["one", "two"].map(|n| ResKey::parse(n, ResType::NSS).unwrap());
    let mut m = Module::open(&path).unwrap();
    for key in keys {
        m.set(key, b"void main()\n{\n}\n".to_vec());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    for key in keys {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        h.run_steps(4);
        assert!(h.state().script_laid_out_for_test(key), "{key} laid out");
    }
    // One closed by its tab, the other by its Cancel.
    close_tab(h.state_mut(), &Tab::Script(keys[1]));
    h.run_steps(3);
    assert!(!h.state().script_laid_out_for_test(keys[1]));
    assert!(h.state().script_laid_out_for_test(keys[0]), "the open one's is kept");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Script(keys[0])).is_none(), "closed");
    assert!(!h.state().script_laid_out_for_test(keys[0]));
}

/// Find In Files for a word a large module's scripts have on tens of
/// thousands of lines: Search Results keeps the first 10,000 (the log
/// says how many were found) and lays out the rows in sight, not all of
/// them every frame, each on one line.
#[test]
fn search_results_keep_ten_thousand_lines_and_draw_those_in_sight() {
    let dir = mg_testkit::scratch_dir("ui-script-search-results");
    let path = sample_module(&dir);
    let key = ResKey::parse("many", ResType::NSS).unwrap();
    let long = "x".repeat(400);
    let text: String = (0..12_500).map(|i| format!("int n{i} = {i}; // word {long}\n")).collect();
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.into_bytes());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(4);
    let search = &mut h.state_mut().script_tools.search;
    (search.open, search.script, search.in_files) = (true, Some(key), true);
    search.find = "word".into();
    h.run_steps(3);
    h.get_by_label("Find All").click();
    h.run_steps(3);
    assert_eq!(h.state().search_results.hits.len(), 10_000);
    let said = "\"word\": 12500 lines found: the first 10000 are in Search Results";
    assert!(h.state().log.entries.iter().any(|(_, m)| m == said), "{:?}", h.state().log.entries);
    h.state_mut().script_tools.search.open = false;
    h.run_steps(3);
    let rows: Vec<egui::Rect> = h.query_all_by_label_contains("many(").map(|n| n.rect()).collect();
    assert!((1..200).contains(&rows.len()), "{} rows laid out", rows.len());
    let line = h.ctx.global_style().spacing.interact_size.y;
    assert!(rows.iter().all(|r| r.height() <= line + 1.0), "one line each: {:?}", rows[0]);
    // A row in sight goes to its line.
    h.get_by_label_contains("many(1):").click();
    h.run_steps(3);
    assert_eq!(script_cursor(&h, key), Some((0, 0)));
}

/// Two scripts for the tests of Rename Symbol's places: `inc_a` (CRLF,
/// with a character of two bytes) declares `Twice` and uses it near its
/// end, `main_a` includes it. The harness, the two keys and `inc_a`'s
/// text.
fn rename_harness(name: &str) -> (Harness<'static, Moonglow>, ResKey, ResKey, String) {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let inc = ResKey::parse("inc_a", ResType::NSS).unwrap();
    let main = ResKey::parse("main_a", ResType::NSS).unwrap();
    let inc_text = "// caf\u{e9}\r\nint Twice(int n) { return n * 2; }\r\n\
                    int Four() { return Twice(2); }\r\n";
    let mut m = Module::open(&path).unwrap();
    m.set(inc, mg_core::Codepage::WINDOWS_1252.encode_lossy(inc_text));
    m.set(main, b"#include \"inc_a\"\r\nvoid main() { int x = Twice(2); }\r\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    (harness(app, 1280.0, 800.0), inc, main, inc_text.to_string())
}

/// A script as the module has it.
fn module_script(h: &Harness<'_, Moonglow>, key: ResKey) -> String {
    let bytes = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
    mg_core::Codepage::WINDOWS_1252.decode(&bytes).into_owned()
}

/// Where `Twice` is used in `main_a`, as the editor counts (characters).
const TWICE_IN_MAIN: usize = 41;

/// References and Rename Symbol take their places in the text the module
/// has: an include whose editor was closed with Don't Save is read from
/// the module again (the places were those of the text dropped, 200
/// characters further on: References cut the module's text past its end
/// and Rename wrote the new name over other bytes), and one never opened,
/// with CRLF line ends, is read with plain newlines as an editor has it.
#[test]
fn rename_symbol_s_places_are_the_module_s_after_an_editor_is_canceled() {
    let (mut h, inc, main, inc_text) = rename_harness("ui-script-rename-canceled");
    let open = |h: &mut Harness<'_, Moonglow>, key: ResKey| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        h.run_steps(4);
    };
    // A script never opened: its places are an editor's, its bytes kept.
    open(&mut h, main);
    let d = h.state_mut().declaration_at(main, TWICE_IN_MAIN).expect("Twice");
    assert_eq!(d.at.file, "inc_a");
    h.state_mut().rename_symbol(&d, "Thrice");
    h.run_steps(3);
    assert_eq!(module_script(&h, inc), inc_text.replace("Twice", "Thrice"));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(module_script(&h, inc), inc_text);

    // The include typed into, the name looked up from the other script,
    // then the include's editor canceled.
    open(&mut h, inc);
    let typed = format!("{}\n// caf\u{e9}\nint Twice(int n);\n", "/".repeat(200));
    h.state_mut().scripts_mut_for_test(inc, &typed);
    assert!(h.state_mut().declaration_at(main, TWICE_IN_MAIN).is_some());
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run_steps(4);
    h.get_by_label("Don't Save").click();
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Script(inc)).is_none(), "closed");
    let d = h.state_mut().declaration_at(main, TWICE_IN_MAIN).expect("Twice");
    h.state_mut().show_references(&d, None);
    let results: Vec<String> = (h.state().search_results.hits.iter())
        .map(|Hit::Line { script, line, text }| format!("{}:{line} {text}", script.resref))
        .collect();
    assert_eq!(
        results,
        [
            "inc_a:1 int Twice(int n) { return n * 2; }",
            "inc_a:2 int Four() { return Twice(2); }",
            "main_a:1 void main() { int x = Twice(2); }"
        ]
    );
    h.state_mut().rename_symbol(&d, "Double");
    h.run_steps(3);
    assert_eq!(module_script(&h, inc), inc_text.replace("Twice", "Double"));
    assert!(module_script(&h, main).contains("int x = Double(2);"));
}

/// A script open in a tab behind another, unchanged, that a command
/// rewrote in the module (the NUI Creator's, a plugin's, Undo): its
/// editor is not drawn, so it had the text of before, and Rename Symbol
/// from another script wrote the places of that text into the new one.
#[test]
fn rename_symbol_reads_a_script_a_command_rewrote_behind_another_tab() {
    let (mut h, inc, main, inc_text) = rename_harness("ui-script-rename-hidden-tab");
    for key in [inc, main] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        h.run_steps(4);
    }
    assert!(h.state().script_text(inc).is_some(), "the include's editor is open");
    // (The include is behind the other script's tab from here on.)
    let rewritten = format!("// A line a command put first.\r\n{inc_text}");
    let edit = mg_edit::Edit::SetResource {
        key: inc,
        data: Some(mg_core::Codepage::WINDOWS_1252.encode_lossy(&rewritten)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Rewrite", vec![edit])));
    h.run_steps(3);
    let d = h.state_mut().declaration_at(main, TWICE_IN_MAIN).expect("Twice");
    h.state_mut().rename_symbol(&d, "Double");
    h.run_steps(3);
    assert_eq!(module_script(&h, inc), rewritten.replace("Twice", "Double"));
}

/// A script's copy takes its compiled script along.
#[test]
fn a_script_is_copied_with_its_compiled_script() {
    let Some((mut h, _)) = plugin_harness("ui-copy-script", false) else { return };
    let (nss, ncs) = (
        ResKey::parse("greet", ResType::NSS).unwrap(),
        ResKey::parse("greet", ResType::NCS).unwrap(),
    );
    let edits = vec![
        mg_edit::Edit::SetResource { key: nss, data: Some(b"void main() { }\n".to_vec()) },
        mg_edit::Edit::SetResource { key: ncs, data: Some(b"NCS V1.0 compiled".to_vec()) },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(nss));
    h.run_steps(3);
    h.state_mut().copy_as.as_mut().expect("the Copy window").resref = "greet_two".into();
    h.run_steps(2);
    h.get_by_label("Create Copy").click();
    h.run_steps(3);
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    let copy = |t| ws.module.get(&ResKey::parse("greet_two", t).unwrap()).map(<[u8]>::to_vec);
    assert_eq!(copy(ResType::NSS).as_deref(), Some(&b"void main() { }\n"[..]));
    assert_eq!(copy(ResType::NCS).as_deref(), Some(&b"NCS V1.0 compiled"[..]));
}

/// Options › Script Editor › Automatically Compile Scripts on Save:
/// saving the module compiles the scripts whose text it stores (it was
/// only a script's own Save that did).
#[test]
fn saving_the_module_compiles_edited_scripts_when_asked() {
    let Some((mut h, _)) = area_ui_harness("save-compiles") else { return };
    let (nss, ncs) = (
        ResKey::parse("hello", ResType::NSS).unwrap(),
        ResKey::parse("hello", ResType::NCS).unwrap(),
    );
    let edit = mg_edit::Edit::SetResource { key: nss, data: Some(b"void main()\n{\n}\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss)));
    h.run_steps(3);
    let has = |h: &mut Harness<'_, Moonglow>, k: &ResKey| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        ws.module.contains(k)
    };
    // Typed in (a line indented), then the module saved: not compiled
    // while the option is off.
    let edit_and_save = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().script_tools.jump = Some((nss, 0));
        h.run_steps(2);
        h.key_press(egui::Key::Tab);
        h.run_steps(2);
        h.state_mut().actions.push(mg_ui::Action::Save);
        h.run_steps(3);
    };
    h.state_mut().settings.auto_compile = false;
    edit_and_save(&mut h);
    assert!(!has(&mut h, &ncs));
    h.state_mut().settings.auto_compile = true;
    edit_and_save(&mut h);
    assert!(has(&mut h, &ncs), "compiled on save");
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("Compiled on save: hello")));
}

/// A script another program saved in the module's folder is compiled
/// when it is read again, with Automatically Compile Scripts on Save: the
/// folder's compiled script was the older one until Compile All.
#[test]
fn a_script_saved_outside_is_compiled_when_asked() {
    use mg_module::ModuleLocation;
    use mg_module::new::new_module;
    let Some(root) = mg_testkit::nwn_root() else { return };
    let game = game_data(&root);
    let mut m = new_module(&game, "Outside", &mut fastrand::Rng::with_seed(7)).unwrap();
    let (nss, ncs) = (
        ResKey::parse("hello", ResType::NSS).unwrap(),
        ResKey::parse("hello", ResType::NCS).unwrap(),
    );
    m.set(nss, b"void main()\n{\n}\n".to_vec());
    let dir = mg_testkit::scratch_dir("ui-folder-compile").join("module");
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.settings.no_mod_beside_folder = true;
    app.settings.auto_compile = false;
    app.open_module(&dir);
    let mut h = harness(app, 1100.0, 800.0);
    h.run_steps(3);
    let compiled = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        ws.module.get(&ncs).map(<[u8]>::to_vec)
    };
    let save_outside = |h: &mut Harness<'_, Moonglow>, text: &str| {
        std::fs::write(dir.join("hello.nss"), text).unwrap();
        h.state_mut().reload_project_files();
        h.run_steps(3);
    };
    // Not asked: read again, not compiled.
    save_outside(&mut h, "void main()\n{\n    int n = 1;\n}\n");
    let text = h.state().ws.as_ref().unwrap().module.get(&nss).unwrap().to_vec();
    assert!(String::from_utf8_lossy(&text).contains("int n = 1;"));
    assert!(compiled(&mut h).is_none());
    // Asked: compiled, and again when its text changes.
    h.state_mut().settings.auto_compile = true;
    save_outside(&mut h, "void main()\n{\n    int n = 2;\n}\n");
    let first = compiled(&mut h).expect("compiled when read again");
    // The module had no unsaved changes, and has none after: the compiled
    // script is written beside its source at once.
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "no unsaved changes");
    assert_eq!(std::fs::read(dir.join("hello.ncs")).ok(), Some(first.clone()));
    h.state_mut().reload_project_files();
    h.run_steps(2);
    assert!(
        !h.state().log.entries.iter().any(|(_, m)| m.contains("Read again") && m.contains("ncs")),
        "the compiled script written is not read again as changed outside"
    );
    save_outside(&mut h, "void main()\n{\n    int n = 2;\n    n += GetHitDice(OBJECT_SELF);\n}\n");
    assert_ne!(compiled(&mut h).unwrap(), first);
    // An include saved outside: the script that includes it is compiled.
    std::fs::write(dir.join("inc_n.nss"), "int N() { return 1; }\n").unwrap();
    save_outside(&mut h, "#include \"inc_n\"\nvoid main()\n{\n    int n = N();\n}\n");
    let with_one = compiled(&mut h).unwrap();
    std::fs::write(dir.join("inc_n.nss"), "int N() { return 2 + GetHitDice(OBJECT_SELF); }\n")
        .unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(3);
    assert_ne!(compiled(&mut h).unwrap(), with_one, "its includer is compiled again");
    // One that does not compile keeps its compiled script, and is named.
    save_outside(&mut h, "void main()\n{\n    nothing();\n}\n");
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("Did not compile")));
    // The save writes the compiled script beside its source.
    save_outside(&mut h, "void main()\n{\n    int n = 3;\n}\n");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert_eq!(std::fs::read(dir.join("hello.ncs")).ok(), compiled(&mut h));
    assert_eq!(
        std::fs::read(dir.join("hello.nss")).unwrap(),
        b"void main()\n{\n    int n = 3;\n}\n"
    );
}

/// A checkout's many scripts changed outside are compiled a few each
/// frame, not as a job that holds the window (more than 24 went to one):
/// the window takes input meanwhile, and once they are done each has its
/// compiled script, written beside it, the module with no unsaved
/// changes.
#[test]
fn many_scripts_saved_outside_are_compiled_a_few_a_frame() {
    use mg_module::ModuleLocation;
    use mg_module::new::new_module;
    let Some(root) = mg_testkit::nwn_root() else { return };
    let game = game_data(&root);
    let mut m = new_module(&game, "Checkout", &mut fastrand::Rng::with_seed(7)).unwrap();
    let names: Vec<String> = (0..40).map(|i| format!("pulled_{i:02}")).collect();
    for n in &names {
        m.set(ResKey::parse(n, ResType::NSS).unwrap(), b"void main()\n{\n}\n".to_vec());
    }
    let dir = mg_testkit::scratch_dir("ui-folder-compile-many").join("module");
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.settings.no_mod_beside_folder = true;
    app.settings.auto_compile = true;
    app.open_module(&dir);
    let mut h = harness(app, 1100.0, 800.0);
    h.run_steps(3);
    for (i, n) in names.iter().enumerate() {
        std::fs::write(
            dir.join(format!("{n}.nss")),
            format!("void main()\n{{\n    int n = {i};\n}}\n"),
        )
        .unwrap();
    }
    h.state_mut().reload_project_files();
    assert!(!h.state().busy(), "no job holds the window");
    let mut frames = 0;
    while h.state().compiling_outside() {
        h.run_steps(1);
        assert!(!h.state().busy());
        frames += 1;
        assert!(frames < 400, "never done");
    }
    let ws = h.state().ws.as_ref().unwrap();
    assert!(!ws.is_modified(), "no unsaved changes");
    for n in &names {
        let ncs = ResKey::parse(n, ResType::NCS).unwrap();
        let made = ws.module.get(&ncs).map(<[u8]>::to_vec);
        assert!(made.is_some(), "{n} compiled");
        assert_eq!(std::fs::read(dir.join(format!("{n}.ncs"))).ok(), made, "{n} written");
    }
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(_, m)| m.starts_with("Compiled (changed outside Moonglow)")),
        "{log:#?}"
    );
    assert!(!log.iter().any(|(_, m)| m.contains("in the background")), "{log:#?}");
}

/// Compile All Scripts passes over the scripts that are as they were when
/// it last compiled them: after an include changes, only the scripts that
/// include it are compiled again.
#[test]
fn compile_all_compiles_only_what_changed() {
    let Some((mut h, _)) = area_ui_harness("compile-changed") else { return };
    let nss = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let set = |h: &mut Harness<'_, Moonglow>, name: &str, text: &str| {
        let edit = mg_edit::Edit::SetResource { key: nss(name), data: Some(text.into()) };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
        h.run_steps(2);
    };
    set(&mut h, "inc_n", "int N() { return 1; }\n");
    set(&mut h, "uses_n", "#include \"inc_n\"\nvoid main() { int n = N(); }\n");
    set(&mut h, "alone", "void main() { int a = 1; }\n");
    let compile = |h: &mut Harness<'_, Moonglow>| -> String {
        let before = h.state().log.entries.len();
        h.state_mut().actions.push(mg_ui::Action::CompileScripts);
        h.run_steps(3);
        let mut said = h.state().log.entries[before..].iter().map(|(_, m)| m.clone());
        said.rfind(|m| m.starts_with("Compiled ")).expect("Compile All's line")
    };
    let first = compile(&mut h);
    assert!(!first.contains("left alone"), "{first}");
    let again = compile(&mut h);
    assert!(again.starts_with("Compiled 0 scripts") && again.contains("left alone"), "{again}");
    // The include changes: it and its includer are compiled, the third
    // script is not; the includer's compiled script is another.
    set(&mut h, "inc_n", "int N() { return 2 + GetHitDice(OBJECT_SELF); }\n");
    let after = compile(&mut h);
    assert!(after.starts_with("Compiled 2 scripts: 0 failed, 1 changed"), "{after}");
    // A compiled script that went missing is made again.
    let ncs = ResKey::parse("alone", ResType::NCS).unwrap();
    let gone = mg_edit::Edit::SetResource { key: ncs, data: None };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Gone", vec![gone])));
    h.run_steps(2);
    let back = compile(&mut h);
    assert!(back.starts_with("Compiled 1 scripts: 0 failed, 1 changed"), "{back}");
    // Kept from one session to the next, in a folder of Moonglow's own:
    // the next session's first Compile All passes over them too.
    h.state_mut().forget_compiled();
    let all = compile(&mut h);
    assert!(!all.contains("left alone"), "nothing is kept without a folder: {all}");
    let kept = mg_testkit::scratch_dir("ui-compiled-kept");
    h.state_mut().compiled_dir = Some(kept.clone());
    h.state_mut().forget_compiled();
    compile(&mut h);
    assert_eq!(std::fs::read_dir(&kept).unwrap().count(), 1, "a file for the module");
    h.state_mut().forget_compiled();
    let next = compile(&mut h);
    assert!(next.starts_with("Compiled 0 scripts") && next.contains("left alone"), "{next}");
    // `mg compile` reads the same: what it makes its scripts with (its
    // own view of the game and the module's haks) is the window's.
    let module = h.state().module_path().unwrap();
    let install = h.state().install.clone().unwrap();
    let mut rm = mg_resman::ResMan::for_game(&install).unwrap();
    let haks = mg_module::Module::open(&module).unwrap().haks().unwrap();
    rm.add_haks(&install, &haks).unwrap();
    let with = mg_module::build::made_with(Some(&module), None, Some(&install.root), &rm);
    let read = mg_module::build::Remembered::kept(Some(&kept), Some(&module), with);
    assert_eq!(read.scripts.len(), 3, "mg compile's memory is the window's");
    // With Generate Debug Information (Options), the compiled scripts
    // that have no debug file are compiled again for one, once.
    h.state_mut().settings.debug_info = true;
    let debug = compile(&mut h);
    assert!(
        debug.starts_with("Compiled 2 scripts: 0 failed, 0 changed, 2 debug files changed"),
        "{debug}"
    );
    let has = |h: &Harness<'_, Moonglow>, name: &str| {
        let key = ResKey::parse(name, ResType::NDB).unwrap();
        h.state().ws.as_ref().unwrap().module.contains(&key)
    };
    assert!(has(&h, "uses_n") && has(&h, "alone") && !has(&h, "inc_n"));
    let again = compile(&mut h);
    assert!(again.starts_with("Compiled 0 scripts"), "{again}");
}

/// Compile All's line has the details under a fold: each script compiled,
/// why, and whether its bytes changed (changed ones first); the fold is
/// closed until its button is clicked; Copy Log has them either way.
#[test]
fn compile_all_says_what_it_compiled_and_why_under_a_fold() {
    let Some((mut h, _)) = area_ui_harness("compile-why") else { return };
    let nss = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let set = |h: &mut Harness<'_, Moonglow>, key: ResKey, data: Option<&[u8]>| {
        let edit = mg_edit::Edit::SetResource { key, data: data.map(<[u8]>::to_vec) };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
        h.run_steps(2);
    };
    set(&mut h, nss("cg_cond"), Some(b"void main() { int a = 1; }\n"));
    set(&mut h, nss("cg_same"), Some(b"void main() { int b = 2; }\n"));
    set(&mut h, nss("cg_bad"), Some(b"void main() { Nope(); }\n"));
    let compile = |h: &mut Harness<'_, Moonglow>| -> (String, Vec<String>) {
        let before = h.state().log.entries.len();
        h.state_mut().actions.push(mg_ui::Action::CompileScripts);
        h.run_steps(3);
        let log = &h.state().log;
        let line =
            (before..log.entries.len()).rfind(|&i| log.entries[i].1.starts_with("Compiled "));
        let line = line.expect("Compile All's line");
        (log.entries[line].1.clone(), log.details.get(&line).cloned().unwrap_or_default())
    };
    let (first, details) = compile(&mut h);
    assert!(first.contains("1 failed"), "{first}");
    assert!(
        details
            .iter()
            .any(|d| d.starts_with("cg_cond: it had no compiled script; compiled, a new")),
        "{details:#?}"
    );
    assert!(
        details.iter().any(|d| d.starts_with("cg_bad: ") && d.contains("failed: ")),
        "{details:#?}"
    );
    // Another tool's compiled script in place of Moonglow's.
    let ncs = ResKey::parse("cg_cond", ResType::NCS).unwrap();
    set(&mut h, ncs, Some(b"NCS from another tool"));
    let (line, details) = compile(&mut h);
    assert!(line.starts_with("Compiled 2 scripts: 1 failed, 1 changed"), "{line}");
    assert_eq!(
        details.iter().find(|d| d.starts_with("cg_cond")).map(String::as_str),
        Some(
            "cg_cond: its compiled script wasn't Moonglow's (another tool made it); compiled \
             again, the bytes differ"
        ),
        "{details:#?}"
    );
    // The changed ones are first.
    assert!(details[0].starts_with("cg_cond"), "{details:#?}");
    // Nothing remembered, the same bytes: compiled again, nothing changed.
    h.state_mut().forget_compiled();
    let (line, details) = compile(&mut h);
    let same = details.iter().find(|d| d.starts_with("cg_same")).expect("cg_same");
    assert_eq!(
        same, "cg_same: Moonglow hadn't compiled it before; compiled again, the same bytes",
        "{line}"
    );
    // The fold: closed, then open (the details shown), then closed.
    h.run_steps(2);
    assert!(h.query_by_label_contains("compiled again, the same bytes").is_none());
    let fold = |h: &mut Harness<'_, Moonglow>| {
        h.query_all_by_label("Details").last().expect("a Details button").click();
        h.run_steps(3);
    };
    fold(&mut h);
    h.get_by_label_contains("cg_same: Moonglow hadn't compiled it before");
    fold(&mut h);
    assert!(h.query_by_label_contains("cg_same: Moonglow hadn't").is_none());
    // Copy Log has the details, folded or not.
    let text = h.state().log.text();
    assert!(text.contains("\n    cg_same: Moonglow hadn't compiled it before"), "{text}");
}

/// A script compiled on its own (its editor's Compile, or a save that
/// compiles the scripts including the one saved) is noted as compiled:
/// the next Compile All passes over it. An external compiler given one
/// script is given it and what it includes, not the module's other
/// scripts.
#[test]
fn a_script_compiled_on_its_own_is_passed_over_by_compile_all() {
    let Some((mut h, _)) = area_ui_harness("compile-on-its-own") else { return };
    let nss = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let set = |h: &mut Harness<'_, Moonglow>, name: &str, text: &str| {
        let edit = mg_edit::Edit::SetResource { key: nss(name), data: Some(text.into()) };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
        h.run_steps(2);
    };
    set(&mut h, "inc_n", "int N() { return 1; }\n");
    set(&mut h, "uses_n", "#include \"inc_n\"\nvoid main() { int n = N(); }\n");
    set(&mut h, "alone", "void main() { int a = 1; }\n");
    let compile = |h: &mut Harness<'_, Moonglow>| -> String {
        let before = h.state().log.entries.len();
        h.state_mut().actions.push(mg_ui::Action::CompileScripts);
        h.run_steps(3);
        let mut said = h.state().log.entries[before..].iter().map(|(_, m)| m.clone());
        said.rfind(|m| m.starts_with("Compiled ")).expect("Compile All's line")
    };
    compile(&mut h);
    // Changed in its editor, and compiled with its Compile.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("alone"))));
    h.run_steps(3);
    h.state_mut().scripts_mut_for_test(nss("alone"), "void main() { int a = 2; }\n");
    h.run_steps(1);
    apart(&h, "Compile").click();
    h.run_steps(3);
    let ncs = ResKey::parse("alone", ResType::NCS).unwrap();
    let made = h.state().ws.as_ref().unwrap().module.get(&ncs).map(<[u8]>::to_vec);
    let after = compile(&mut h);
    assert!(after.starts_with("Compiled 0 scripts") && after.contains("left alone"), "{after}");
    assert_eq!(h.state().ws.as_ref().unwrap().module.get(&ncs).map(<[u8]>::to_vec), made);
    // An include saved, compiling on save: the script including it is
    // compiled then, and passed over by Compile All (the include, never
    // compiled on its own, is looked at again).
    h.state_mut().settings.auto_compile = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("inc_n"))));
    h.run_steps(3);
    h.state_mut().scripts_mut_for_test(nss("inc_n"), "int N() { return 2; }\n");
    h.state_mut().actions.push(mg_ui::Action::SaveScript(nss("inc_n")));
    h.run_steps(4);
    let saved = compile(&mut h);
    assert!(saved.starts_with("Compiled 1 scripts: 0 failed, 0 changed"), "{saved}");

    // An external compiler (a stand-in that lists what it is given, and
    // "compiles" a script by copying it) compiling one script.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = mg_testkit::scratch_dir("ui-external-one");
        let (program, listed) = (dir.join("listcomp"), dir.join("given.txt"));
        let line = format!(
            "#!/bin/sh\nsrc=$1; out=$2; shift 2\nls \"$src\" > '{}'\nfor f in \"$@\"; do \
             cp \"$f\" \"$out/$(basename \"$f\" .nss).ncs\"; done\n",
            listed.display()
        );
        std::fs::write(&program, line).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        h.state_mut().settings.external_compiler = Some(program);
        h.state_mut().settings.external_compiler_args = "{src} {out} {files}".into();
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("uses_n"))));
        h.run_steps(3);
        apart(&h, "Compile").click();
        h.run_steps(3);
        let given = std::fs::read_to_string(&listed).unwrap();
        assert_eq!(given.lines().collect::<Vec<_>>(), ["inc_n.nss", "uses_n.nss"], "{given}");
    }
}

/// An include changed in its editor: the scripts that include it are
/// compiled again with it, by its editor's Compile, by its own Save and
/// by the module's Save (with Automatically Compile Scripts on Save), and
/// the log names them; a script that includes nothing changed is not.
#[test]
fn an_include_s_includers_are_compiled_with_it() {
    let Some((mut h, _)) = area_ui_harness("compile-includers") else { return };
    let nss = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
    let set = |h: &mut Harness<'_, Moonglow>, name: &str, text: &str| {
        let edit = mg_edit::Edit::SetResource { key: nss(name), data: Some(text.into()) };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
        h.run_steps(2);
    };
    set(&mut h, "inc_n", "int N() { return 1; }\n");
    set(&mut h, "uses_n", "#include \"inc_n\"\nvoid main() { int n = N(); }\n");
    set(&mut h, "alone", "void main() { int a = 1; }\n");
    h.state_mut().actions.push(mg_ui::Action::CompileScripts);
    h.run_steps(3);
    let ncs = |h: &mut Harness<'_, Moonglow>, name: &str| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        ws.module.get(&ResKey::parse(name, ResType::NCS).unwrap()).map(<[u8]>::to_vec)
    };
    let (uses, alone) = (ncs(&mut h, "uses_n").expect("compiled"), ncs(&mut h, "alone"));
    let said = |h: &Harness<'_, Moonglow>, from: usize, what: &str| {
        h.state().log.entries[from..].iter().any(|(_, m)| m.contains(what))
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss("inc_n"))));
    h.run_steps(3);

    // Its editor's Compile.
    let from = h.state().log.entries.len();
    h.state_mut().scripts_mut_for_test(nss("inc_n"), "int N() { return 2; }\n");
    h.run_steps(1);
    apart(&h, "Compile").click();
    h.run_steps(3);
    let by_compile = ncs(&mut h, "uses_n").unwrap();
    assert_ne!(by_compile, uses, "compiled again by the include's Compile");
    assert!(said(&h, from, "Compiled, as they include inc_n: uses_n"));

    // Its own Save.
    h.state_mut().settings.auto_compile = true;
    let from = h.state().log.entries.len();
    h.state_mut().scripts_mut_for_test(nss("inc_n"), "int N() { return 3; }\n");
    h.state_mut().actions.push(mg_ui::Action::SaveScript(nss("inc_n")));
    h.run_steps(4);
    let by_save = ncs(&mut h, "uses_n").unwrap();
    assert_ne!(by_save, by_compile, "compiled again by the include's Save");
    assert!(said(&h, from, "uses_n"), "named in the log");

    // The module's Save, with the include's text typed and not stored.
    let from = h.state().log.entries.len();
    h.state_mut().scripts_mut_for_test(nss("inc_n"), "int N() { return 4; }\n");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(4);
    assert_ne!(ncs(&mut h, "uses_n").unwrap(), by_save, "compiled again by the module's Save");
    assert!(said(&h, from, "Compiled on save:") && said(&h, from, "uses_n"));
    assert_eq!(ncs(&mut h, "alone"), alone, "a script that includes it not, only those that do");
}

/// In Open Script, Down and Up choose among the scripts listed while the
/// name is typed, and Enter opens the one chosen (the first, unmoved).
#[test]
fn open_script_s_arrows_choose_among_those_listed() {
    let Some((mut h, _)) = area_ui_harness("open-script-keys") else { return };
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key: hello, data: Some(b"void main() { }\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    h.run_steps(4);
    script_click(&mut h, "Open…");
    h.run_steps(3);
    h.get_by_label("All Resources").click();
    h.run_steps(2);
    h.state_mut().script_tools.open_script.as_mut().unwrap().filter = "nw_s0_fire".into();
    h.run_steps(2);
    let listed: Vec<ResKey> = {
        let w = h.state().script_tools.open_script.as_ref().unwrap();
        assert_eq!(w.filter, "nw_s0_fire");
        let has = |k: &ResKey| k.resref.to_string().contains("nw_s0_fire");
        w.scripts.iter().map(|(k, _)| *k).filter(has).collect()
    };
    assert!(listed.len() > 2, "{listed:?}");
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    assert_eq!(h.state().script_tools.open_script.as_ref().unwrap().cursor, 1);
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert!(h.state().script_tools.open_script.is_none(), "the window closed");
    assert!(h.state().dock.find_tab(&Tab::Resource(listed[1])).is_some(), "{:?}", listed[1]);
}

/// In Open Script, Escape clears the name typed (the window stays) and
/// closes the window next, as every dialog's search field does; Clear clears
/// it too.
#[test]
fn open_script_s_escape_clears_the_name_then_closes() {
    let Some((mut h, _)) = area_ui_harness("open-script-escape") else { return };
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key: hello, data: Some(b"void main() { }\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    h.run_steps(4);
    script_click(&mut h, "Open…");
    h.run_steps(3);
    let filter = |h: &Harness<'_, Moonglow>| {
        h.state().script_tools.open_script.as_ref().map(|w| w.filter.clone())
    };
    let field =
        |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find by name");
    h.get(egui_kittest::kittest::by().predicate(field)).type_text("hel");
    h.run_steps(2);
    assert_eq!(filter(&h).as_deref(), Some("hel"));
    h.get_by_label("Clear").click();
    h.run_steps(2);
    assert_eq!(filter(&h).as_deref(), Some(""), "Clear clears it");
    h.get(egui_kittest::kittest::by().predicate(field)).click();
    h.run_steps(2);
    h.get(egui_kittest::kittest::by().predicate(field)).type_text("hel");
    h.run_steps(2);
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert_eq!(filter(&h).as_deref(), Some(""), "the window stays, its name cleared");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert_eq!(filter(&h), None, "the window closed");
}

/// A script editor's Open… lists the scripts there are to open, the
/// module's alone or its haks' or all of them, found by name; the game's
/// open to be read.
#[test]
fn a_script_editor_opens_other_scripts() {
    let Some((mut h, _)) = area_ui_harness("open-script") else { return };
    // (The module tree kept still: opened at a script, its rows would
    // have the scripts' names too.)
    h.state_mut().settings.tree_stays = true;
    let (hello, other) = (
        ResKey::parse("hello", ResType::NSS).unwrap(),
        ResKey::parse("other_one", ResType::NSS).unwrap(),
    );
    let edits = [hello, other]
        .map(|key| mg_edit::Edit::SetResource { key, data: Some(b"void main() { }\n".to_vec()) })
        .to_vec();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    h.run_steps(4);
    script_click(&mut h, "Open…");
    h.run_steps(3);
    // The module's two, as it opens.
    h.get_by_label("2 scripts");
    // All of them: the game's too, found by name.
    h.get_by_label("All Resources").click();
    h.run_steps(2);
    assert!(h.query_by_label("2 scripts").is_none());
    h.state_mut().script_tools.open_script.as_mut().unwrap().filter = "nw_s0_firebal".into();
    h.run_steps(2);
    let at = h.get_by_label("nw_s0_fireball").rect().center();
    double_click_now(&mut h, at);
    h.run_steps(4);
    let fireball = ResKey::parse("nw_s0_fireball", ResType::NSS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Resource(fireball)).is_some(), "read, not edited");
    assert!(h.state().script_tools.open_script.is_none(), "the window closed");
    // The module's own opens in its editor.
    let tab = h.state().dock.find_tab(&Tab::Resource(fireball)).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    h.run_steps(3);
    script_click(&mut h, "Open…");
    h.run_steps(3);
    h.get_by_label("Module Resources Only").click();
    h.run_steps(2);
    let at = h.get_by_label("other_one").rect().center();
    double_click_now(&mut h, at);
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Script(other)).is_some());
}

/// In Open Script a click selects a script and opens nothing; Shift+click
/// selects those between, Ctrl+click adds one or takes it away, the
/// arrows move the selection (Shift widens it), and Open or Enter opens
/// every one selected, each in its tab; a double click opens that one
/// alone (El Grillo's request).
#[test]
fn open_script_opens_the_scripts_selected_together() {
    let dir = mg_testkit::scratch_dir("ui-open-several");
    let path = sample_module(&dir);
    let names = ["s1", "s2", "s3", "s4", "s5", "s6"];
    let mut m = Module::open(&path).unwrap();
    for n in names {
        m.set(ResKey::parse(n, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    // (The module tree kept still: opened at a script, its rows would
    // have the scripts' names too.)
    app.settings.tree_stays = true;
    app.open_module(&path);
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    let mut h = window_harness(app);
    h.run();
    let open_now = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let mut open: Vec<String> = (h.state().dock.iter_all_tabs())
            .filter_map(|(_, t)| match t {
                Tab::Script(k) if *k != hello => Some(k.resref.to_string()),
                _ => None,
            })
            .collect();
        open.sort();
        open
    };
    let selected = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let w = h.state().script_tools.open_script.as_ref().expect("the window is open");
        w.selected.iter().map(|k| k.resref.to_string()).collect()
    };
    let window = |h: &mut Harness<'_, Moonglow>| {
        script_click(h, "Open…");
        h.run_steps(3);
        // (Past the double-click time of the click before.)
        h.run_steps(30);
    };
    window(&mut h);
    h.get_by_label("7 scripts");
    // Open is off with nothing selected.
    assert!(apart(&h, "Open").accesskit_node().is_disabled());
    h.get_by_label("s1").click();
    h.run_steps(2);
    assert_eq!(selected(&h), ["s1"]);
    assert!(open_now(&h).is_empty(), "a click opens nothing");
    h.get_by_label("s3").click_modifiers(egui::Modifiers::SHIFT);
    h.run_steps(2);
    assert_eq!(selected(&h), ["s1", "s2", "s3"]);
    h.get_by_label("s2").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(2);
    assert_eq!(selected(&h), ["s1", "s3"]);
    h.get_by_label("7 scripts, 2 selected");
    h.get_by_label("Open 2").click();
    h.run_steps(4);
    assert_eq!(open_now(&h), ["s1", "s3"]);
    assert!(h.state().script_tools.open_script.is_none(), "the window closed");

    // A double click opens that one alone.
    window(&mut h);
    h.get_by_label("s4").click();
    h.run_steps(2);
    h.get_by_label("s5").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(30);
    let at = h.get_by_label("s6").rect().center();
    double_click_now(&mut h, at);
    h.run_steps(4);
    assert_eq!(open_now(&h), ["s1", "s3", "s6"]);

    // The arrows: Down twice (from the top, hello) to s2, Shift+Down to
    // s3 as well; Enter opens both.
    window(&mut h);
    for _ in 0..2 {
        h.key_press(egui::Key::ArrowDown);
        h.run_steps(2);
    }
    assert_eq!(selected(&h), ["s2"]);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(selected(&h), ["s2", "s3"]);
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert_eq!(open_now(&h), ["s1", "s2", "s3", "s6"]);
    assert!(h.state().script_tools.open_script.is_none());

    // Home and End: to the first and the last listed (Shift widens);
    // Ctrl+A selects every one listed.
    let all = ["hello", "s1", "s2", "s3", "s4", "s5", "s6"];
    window(&mut h);
    h.key_press(egui::Key::End);
    h.run_steps(2);
    assert_eq!(selected(&h), ["s6"]);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Home);
    h.run_steps(2);
    assert_eq!(selected(&h), all);
    h.get_by_label("s2").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run_steps(2);
    assert_eq!(selected(&h), all);
    // While a name is typed in its field, they are the field's: Ctrl+A
    // selects its text, not the scripts.
    (h.get_all_by_role(egui::accesskit::Role::TextInput)
        .filter(|n| n.accesskit_node().label().as_deref() != Some("Quick Open")))
    .find(|n| n.accesskit_node().placeholder() == Some("Find by name"))
    .expect("the name field")
    .click();
    h.run_steps(2);
    h.event(egui::Event::Text("s".into()));
    h.run_steps(2);
    assert!(selected(&h).is_empty(), "a name typed starts over");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.key_press(egui::Key::End);
    h.run_steps(2);
    assert!(selected(&h).is_empty(), "the field's keys: {:?}", selected(&h));
    h.state_mut().script_tools.open_script.as_mut().unwrap().filter.clear();
    h.run_steps(2);
    // Scripts selected and then hidden by what is shown are selected no
    // more (Open would not open them).
    h.get_by_label("s1").click();
    h.run_steps(2);
    h.get_by_label("Hak Pak Resources Only").click();
    h.run_steps(2);
    h.get_by_label("Module Resources Only").click();
    h.run_steps(2);
    assert!(selected(&h).is_empty(), "{:?}", selected(&h));
    // A right click on a script: it alone selected, and a menu.
    h.get_by_label("s4").click_secondary();
    h.run_steps(2);
    assert_eq!(selected(&h), ["s4"]);
    h.get_by_label("Select All Listed").click();
    h.run_steps(2);
    assert_eq!(selected(&h), all);
    h.get_by_label("s5").click();
    h.run_steps(2);
    h.get_by_label("s6").click_modifiers(egui::Modifiers::COMMAND);
    h.run_steps(30);
    h.get_by_label("s6").click_secondary();
    h.run_steps(2);
    assert_eq!(selected(&h), ["s5", "s6"], "a script selected keeps the selection");
    h.get_by_label("Open 2 Selected").click();
    h.run_steps(4);
    assert_eq!(open_now(&h), ["s1", "s2", "s3", "s5", "s6"]);
}

/// Enter in the script editor begins the new line as far in as the line
/// it is started from, as Aurora does.
#[test]
fn a_new_line_in_a_script_is_as_far_in_as_the_last() {
    let dir = mg_testkit::scratch_dir("ui-script-new-line");
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let key = ResKey::parse("indented", ResType::NSS).unwrap();
    let text = "void main()\n{\n    int n;\n}\n";
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    h.run_steps(4);
    // The cursor after "int n;", and Enter, then a letter.
    let at = text.find("int n;").unwrap() + "int n;".len();
    let id = egui::Id::new(("script", key));
    let mut state = egui::text_edit::TextEditState::load(&h.ctx, id).unwrap_or_default();
    let cursor = egui::text::CCursor::new(at);
    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(cursor)));
    state.store(&h.ctx, id);
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.run_steps(3);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    h.event(egui::Event::Text("x".into()));
    h.run_steps(3);
    let now = h.state().script_text(key).unwrap();
    assert_eq!(now, "void main()\n{\n    int n;\n    x\n}\n");
}

/// Where a script editor shows the row that begins with `start` (the
/// accessibility tree's text run of it), if it is laid out.
fn script_row(h: &Harness<'_, Moonglow>, start: &str) -> Option<egui::Rect> {
    let run = egui_kittest::kittest::by().role(egui::accesskit::Role::TextRun);
    h.query_all(run)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with(start)))
        .map(|n| n.rect())
}

/// Whether a script editor's row that begins with `start` is in the window.
fn script_row_in_sight(h: &Harness<'_, Moonglow>, start: &str) -> bool {
    let window = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    script_row(h, start).is_some_and(|r| window.contains_rect(r))
}

/// A script far longer than its editor shows (3,000 lines), whose lines
/// are laid out one at a time and kept (`script_layout`): the cursor moved
/// down to a line out of sight, a selection over lines out of sight typed
/// over and taken back, Find going to a line far down, a large paste, and
/// text composed (IME), each drawn where the text has it.
#[test]
fn a_long_script_s_lines_out_of_sight_are_edited_where_they_are() {
    let dir = mg_testkit::scratch_dir("ui-script-long");
    let path = sample_module(&dir);
    let key = ResKey::parse("long", ResType::NSS).unwrap();
    let text: String = (0..3000).map(|n| format!("int nLine{n} = {n}; // line {n}\n")).collect();
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.clone().into_bytes());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    let line = |h: &Harness<'_, Moonglow>, n: usize| {
        h.state().script_text(key).unwrap().lines().nth(n).unwrap().to_string()
    };
    let start_of = |h: &Harness<'_, Moonglow>, n: usize| {
        mg_ui::script_tools::line_start(&h.state().script_text(key).unwrap(), n)
    };
    assert!(script_row_in_sight(&h, "int nLine0 "));
    assert!(!script_row_in_sight(&h, "int nLine120 "), "out of sight at first");

    // The cursor down 120 lines, a key at a time: the editor follows it.
    for _ in 0..120 {
        h.key_press(egui::Key::ArrowDown);
    }
    h.run();
    h.event(egui::Event::Text("A".into()));
    h.run();
    assert_eq!(line(&h, 120), "Aint nLine120 = 120; // line 120");
    assert!(script_row_in_sight(&h, "Aint nLine120 "));
    assert!(!script_row_in_sight(&h, "int nLine0 "));

    // A selection from there down 200 lines (in one frame), out of sight
    // at its start: typed over, the lines between go with it.
    for _ in 0..200 {
        h.input_mut().events.push(egui::Event::Key {
            key: egui::Key::ArrowDown,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        });
    }
    h.run();
    let at = start_of(&h, 120) + 1;
    assert_eq!(script_cursor(&h, key), Some((start_of(&h, 320) + 1, at)));
    assert!(script_row_in_sight(&h, "int nLine320 "), "the selection's end in sight");
    h.event(egui::Event::Text("S".into()));
    h.run();
    assert_eq!(line(&h, 120), "ASnt nLine320 = 320; // line 320");
    assert!(!h.state().script_text(key).unwrap().contains("nLine200 "));
    assert!(script_row_in_sight(&h, "ASnt nLine320 "));
    // Taken back: the lines are there again, each drawn on its own row.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    let back = h.state().script_text(key).unwrap();
    assert!(back.contains("\nint nLine200 = 200;") && !back.contains("ASnt"), "{:.100}", back);
    let (a, b) =
        (script_row(&h, "int nLine200 ").unwrap(), script_row(&h, "int nLine300 ").unwrap());
    assert!(((b.top() - a.top()) - 100.0 * a.height()).abs() < 1.0, "{a:?} {b:?}");

    // Find (F3) goes to a line far down, and shows it.
    h.state_mut().script_tools.search.find = "nLine2750 ".into();
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.key_press(egui::Key::F3);
    h.run();
    h.run();
    let found = h.state().script_text(key).unwrap();
    let found = mg_ui::script_tools::char_index(&found, found.find("nLine2750 ").unwrap());
    let (a, b) = script_cursor(&h, key).unwrap();
    assert_eq!((a.min(b), a.max(b)), (found, found + 10));
    assert!(script_row_in_sight(&h, "int nLine2750 "));
    assert!(!script_row_in_sight(&h, "int nLine2000 "));

    // A thousand lines pasted at line 10: the lines after them a thousand
    // rows down, drawn as the text has them.
    let at = start_of(&h, 10);
    h.state_mut().script_tools.jump = Some((key, at));
    h.run();
    let pasted: String = (0..1000).map(|n| format!("// pasted {n}\n")).collect();
    h.event(egui::Event::Paste(pasted));
    h.run();
    assert_eq!(line(&h, 10), "// pasted 0");
    assert_eq!(line(&h, 1010), "int nLine10 = 10; // line 10");
    assert!(script_row_in_sight(&h, "int nLine10 "), "the cursor, after the paste, in sight");
    let (a, b) =
        (script_row(&h, "// pasted 0").unwrap(), script_row(&h, "int nLine2999 ").unwrap());
    let rows = h.state().script_text(key).unwrap().lines().count() - 1 - 10;
    assert!(((b.top() - a.top()) - rows as f32 * a.height()).abs() < 1.0, "{a:?} {b:?}");

    // Text composed (IME) there: shown as it is composed, then put in.
    let preedit = egui::ImeEvent::Preedit { text: "ka".into(), active_range_chars: None };
    h.event(egui::Event::Ime(preedit));
    h.run();
    assert_eq!(line(&h, 1010), "kaint nLine10 = 10; // line 10");
    assert!(script_row_in_sight(&h, "kaint nLine10 "));
    h.event(egui::Event::Ime(egui::ImeEvent::Commit("か".into())));
    h.run();
    assert_eq!(line(&h, 1010), "かint nLine10 = 10; // line 10");
    assert!(script_row_in_sight(&h, "かint nLine10 "));
}

/// The script editor has no frame around its code, with the keys or not
/// (the mock's `.code`): egui framed it white while it had them (the
/// selection's stroke), the theme's fields' accent pass leaving it.
#[test]
fn the_script_editor_has_no_frame_with_the_keys() {
    let dir = mg_testkit::scratch_dir("ui-script-no-frame");
    let path = sample_module(&dir);
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(script, b"void main()\n{\n}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1300.0, 900.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(script)));
    settle(&mut h);
    let code = egui_kittest::kittest::by().role(egui::accesskit::Role::MultilineTextInput);
    for light in [false, true] {
        h.state_mut().settings.light_theme = light;
        settle(&mut h);
        h.get(code.clone()).click();
        settle(&mut h);
        assert!(h.ctx.memory(|m| m.focused()).is_some(), "the editor has the keys");
        let field = h.get(code.clone()).rect();
        // A rectangle stroked round the code (within a few points of the
        // field's edges), in any color.
        let framed: Vec<egui::epaint::RectShape> = drawn(&h)
            .into_iter()
            .filter_map(|s| match s {
                egui::Shape::Rect(r)
                    if r.stroke.width > 0.0
                        && r.stroke.color != egui::Color32::TRANSPARENT
                        && (r.rect.min - field.min).length() < 4.0
                        && (r.rect.max - field.max).length() < 4.0 =>
                {
                    Some(r)
                }
                _ => None,
            })
            .collect();
        assert!(framed.is_empty(), "no frame round the code ({light}): {framed:?}");
    }
}

/// The script editor's lines and gutter as Script.png draws them (M5's
/// W3, the owner's ruling 21: "match the board"): its lines 18 points
/// apart at the 13-point text (24 at Options' 17), the line numbers 12
/// points after the editor's edge and 10 before the text's field, the
/// text 8 points into it; a click on a line's lower part takes that line;
/// a line gone to far down (Go to Line, a result's or a bookmark's jump)
/// is shown whole; and the window comes to rest with the cursor's line
/// marked.
#[test]
fn the_script_editor_has_the_board_s_line_pitch_and_gutter() {
    let dir = mg_testkit::scratch_dir("ui-script-pitch");
    let path = sample_module(&dir);
    let key = ResKey::parse("long", ResType::NSS).unwrap();
    let text: String = (0..3000).map(|n| format!("int nLine{n} = {n}; // line {n}\n")).collect();
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.clone().into_bytes());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    h.run();
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    let (a, b) = (script_row(&h, "int nLine0 ").unwrap(), script_row(&h, "int nLine1 ").unwrap());
    assert_eq!((a.height(), b.top() - a.top()), (18.0, 18.0), "{a:?} {b:?}");
    // The gutter and the text's field: their room as the board's.
    let field = h.get_by_role(egui::accesskit::Role::MultilineTextInput).rect();
    assert_eq!(a.left() - field.left(), 8.0, "the text in its field");
    let (drawn, band) = Moonglow::script_gutter_for_test(&h.ctx, key).expect("a gutter");
    // The cursor's line (line 0) marked: its band on its row.
    assert_eq!(band, Some(a.y_range()), "the current line's band");
    let char_width = a.width() / "int nLine0 = 0; // line 0".chars().count() as f32;
    let gutter = 12.0 + 4.0 * char_width + 10.0;
    assert!((drawn.span() - gutter).abs() < 0.5, "the gutter {drawn:?}, the board's {gutter}");
    assert_eq!(field.left(), drawn.max, "the field right after the gutter");
    // The numbers painted on their rows, right-aligned 10 points before
    // the gutter's end; a bookmark's icon in the 12 points before them.
    h.state_mut().scripts_mut_bookmark_for_test(key, mg_ui::script_tools::line_start(&text, 3));
    h.run();
    h.run();
    let texts = painted_texts(&h);
    for n in 0..12 {
        let row = egui::Rangef::new(a.top() + n as f32 * 18.0, a.top() + (n + 1) as f32 * 18.0);
        let number = (n + 1).to_string();
        let at = texts.iter().find(|(t, r)| {
            *t == number && drawn.contains(r.center().x) && row.contains(r.center().y)
        });
        let (_, r) = at.unwrap_or_else(|| panic!("line {number}'s number not on its row"));
        assert!((r.right() - (drawn.max - 10.0)).abs() < 0.5, "{number} right-aligned: {r:?}");
    }
    // (The mark in the first 10 points, 2 clear before the numbers' column:
    // its ink, which fills about 70% of its square, 3 or more clear.)
    let pad =
        egui::Rect::from_x_y_ranges(drawn.min..=drawn.min + 10.0, a.top() + 54.0..=a.top() + 72.0);
    assert!(painted_inside(&h, pad), "the bookmark's icon before line 4's number");
    let room = egui::Rect::from_x_y_ranges(drawn.min..=drawn.min + 12.0, pad.y_range());
    let icon = painted_extent(&h, room).expect("the icon");
    let column = drawn.min + 12.0;
    assert!(column - icon.right() >= 3.0, "the icon clear of the numbers: {icon:?}");
    let above = pad.translate(egui::vec2(0.0, -18.0));
    assert!(!painted_inside(&h, above), "none before line 3's");
    // For a screen reader: the gutter by what it shows, the field by the
    // cursor's line (never one string of every number).
    let gutter = h.get_by_label("Line numbers 1 to 3001");
    assert_eq!(gutter.rect().x_range(), drawn, "the gutter named");
    let said = |h: &Harness<'_, Moonglow>| {
        let field = h.get_by_role(egui::accesskit::Role::MultilineTextInput);
        field.accesskit_node().description()
    };
    assert_eq!(said(&h).as_deref(), Some("Line 1 of 3001"));
    // A click on line 5's lower part (2 points above its foot, at the
    // board's pitch from line 0) takes line 5.
    let low = egui::pos2(a.left() + 30.0, a.top() + 5.0 * 18.0 + 16.0);
    click_pos(&h, low);
    h.run();
    let start = mg_ui::script_tools::line_start(&text, 5);
    let (at, _) = script_cursor(&h, key).unwrap();
    assert!((start..start + 20).contains(&at), "line 5 taken: {at}, its start {start}");
    // Page Down: the cursor as many lines on as are in sight less one, the
    // text up by as many of the board's rows, the cursor's line in sight.
    // (Line 30's row: "int nLine5 " also begins another run in the window.)
    let before = script_row(&h, "int nLine30 ").unwrap();
    h.key_press(egui::Key::PageDown);
    settle(&mut h);
    let (at, _) = script_cursor(&h, key).unwrap();
    let line = text[..at].matches('\n').count();
    let n = line - 5;
    assert!(n >= 10, "a page of lines: {n}");
    let after = script_row(&h, "int nLine30 ").map_or(f32::NAN, |r| r.top());
    let moved = before.top() - after;
    assert!((moved - n as f32 * 18.0).abs() < 0.5, "{n} lines, scrolled {moved}");
    assert!(script_row_in_sight(&h, &format!("int nLine{line} ")), "line {line} in sight");
    // A line far down gone to: shown whole.
    h.state_mut().script_tools.jump = Some((key, mg_ui::script_tools::line_start(&text, 2750)));
    h.run();
    h.run();
    assert!(script_row_in_sight(&h, "int nLine2750 "));
    settle(&mut h);
    let row = script_row(&h, "int nLine2750 ").unwrap();
    let band = Moonglow::script_gutter_for_test(&h.ctx, key).and_then(|(_, b)| b);
    assert_eq!(band, Some(row.y_range()), "the band follows the cursor");
    assert_eq!(said(&h).as_deref(), Some("Line 2751 of 3001"), "and the field says it");
    // With text selected, no band.
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    settle(&mut h);
    assert_ne!(script_cursor(&h, key).map(|(a, b)| a == b), Some(true), "a selection");
    let band = Moonglow::script_gutter_for_test(&h.ctx, key).and_then(|(_, b)| b);
    assert_eq!(band, None, "no band over a selection");
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    // Options › Script Editor's size: the pitch in the board's ratio.
    h.state_mut().settings.script_style.font_size = 17;
    h.run();
    h.run();
    let (a, b) =
        (script_row(&h, "int nLine2750 ").unwrap(), script_row(&h, "int nLine2751 ").unwrap());
    assert_eq!((a.height(), b.top() - a.top()), (24.0, 24.0), "{a:?} {b:?}");
    // At rest: a frame's run ends asking for no more, and draws the lines
    // where they were.
    rest(&mut h);
    assert!(h.try_run().is_ok(), "the editor keeps asking for frames");
    assert_eq!(script_row(&h, "int nLine2750 "), Some(a), "moved at rest");
}

/// The script editor's right-click menu (a builder's report, 5.2: it had
/// only Convert Indentation): the editor's Undo and Redo, Cut, Copy,
/// Paste and Select All, then Convert Indentation, each doing its work; a
/// right click in the selection keeps it, outside it moves the caret
/// there; Cut and Copy off with nothing selected, Undo off with nothing
/// to take back.
#[test]
fn the_script_editor_s_menu_cuts_copies_pastes_and_undoes() {
    use crate::fields::{choose_text_row, right_click_at, text_menu_row, text_menu_rows};
    let dir = mg_testkit::scratch_dir("ui-script-text-menu");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main()\n{\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    settle(&mut h);
    let editor = |h: &Harness<'_, Moonglow>| {
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .find(|n| n.accesskit_node().value().is_some_and(|v| v.contains("main")))
            .expect("the script's text")
            .rect()
    };
    // "void main()" selected, the editor with the keys.
    let id = egui::Id::new(("script", key));
    let select = |h: &mut Harness<'_, Moonglow>, a: usize, b: usize| {
        let mut state = egui::text_edit::TextEditState::load(&h.ctx, id).unwrap_or_default();
        let range =
            egui::text::CCursorRange::two(egui::text::CCursor::new(a), egui::text::CCursor::new(b));
        state.cursor.set_char_range(Some(range));
        state.store(&h.ctx, id);
        h.ctx.memory_mut(|m| m.request_focus(id));
        settle(h);
    };
    select(&mut h, 0, 11);
    // (On "void", the first line's start.)
    let on_void = editor(&h).left_top() + egui::vec2(14.0, 9.0);
    right_click_at(&mut h, on_void);
    assert_eq!(script_cursor(&h, key), Some((11, 0)), "the selection kept");
    assert_eq!(text_menu_rows(&h), [Some(true); 4]);
    let keys = |h: &Harness<'_, Moonglow>, id| h.state().keymap.label_of(id, &h.ctx);
    let (undo, redo) = (keys(&h, "undo"), keys(&h, "redo"));
    assert_eq!(text_menu_row(&h, "Undo", &undo).map(|r| r.0), Some(false), "nothing to undo");
    assert_eq!(text_menu_row(&h, "Redo", &redo).map(|r| r.0), Some(false));
    for row in ["Convert Indentation to Tabs", "Convert Indentation to Spaces"] {
        assert!(h.query_by_label(row).is_some(), "{row} kept");
    }
    // Copy: copied, the text as it was.
    assert_eq!(choose_text_row(&mut h, "Copy", "Ctrl+C").0, ["void main()"]);
    assert_eq!(h.state().script_text(key).unwrap(), text);
    // Cut: copied and taken out.
    right_click_at(&mut h, on_void);
    assert_eq!(choose_text_row(&mut h, "Cut", "Ctrl+X").0, ["void main()"]);
    assert_eq!(h.state().script_text(key).unwrap(), "\n{\n}\n");
    // Undo: back as it was.
    right_click_at(&mut h, on_void);
    assert_eq!(text_menu_rows(&h), [Some(false), Some(false), Some(true), Some(true)]);
    assert_eq!(text_menu_row(&h, "Undo", &undo).map(|r| r.0), Some(true));
    choose_text_row(&mut h, "Undo", &undo);
    assert_eq!(h.state().script_text(key).unwrap(), text);
    // Paste at the caret: the clipboard asked for, its text put in.
    select(&mut h, 11, 11);
    let past_the_end = editor(&h).right_top() + egui::vec2(-8.0, 9.0);
    right_click_at(&mut h, past_the_end);
    assert_eq!(script_cursor(&h, key), Some((11, 11)), "past the line's end: at its end");
    assert_eq!(choose_text_row(&mut h, "Paste", "Ctrl+V"), (vec![], true));
    h.event(egui::Event::Paste(" // pasted".into()));
    settle(&mut h);
    assert_eq!(h.state().script_text(key).unwrap(), "void main() // pasted\n{\n}\n");
    // A right click outside the selection: the caret where it was pressed.
    select(&mut h, 12, 13);
    right_click_at(&mut h, on_void);
    let (a, b) = script_cursor(&h, key).unwrap();
    assert!(a == b && a <= 2, "the caret at the click: {a} {b}");
    // Select All.
    choose_text_row(&mut h, "Select All", "Ctrl+A");
    let all = h.state().script_text(key).unwrap().chars().count();
    assert_eq!(script_cursor(&h, key), Some((all, 0)));
    // Without the keys: a right click gives the editor them, and opens its
    // menu (the editor draws its own text, not an egui field that takes
    // the keys at any press: `script_layout::code_edit`).
    h.ctx.memory_mut(|m| m.surrender_focus(id));
    settle(&mut h);
    assert!(!h.ctx.memory(|m| m.has_focus(id)), "the keys given up");
    right_click_at(&mut h, on_void);
    assert!(h.ctx.memory(|m| m.has_focus(id)), "the keys in the editor");
    assert_eq!(text_menu_row(&h, "Paste", "Ctrl+V").map(|r| r.0), Some(true), "its menu open");
}

/// The completion list (F2) is as wide as its widest name, each name on
/// one line (a builder's report, 5.2: a list after one of a single short
/// name kept its width, its names wrapped a few letters a line, with a
/// scroll bar): after a list of one short name, a list with a long one;
/// the same width at every place it scrolls to, and none narrower than
/// its least; the window comes to rest with it open.
#[test]
fn the_completion_list_is_as_wide_as_its_widest_name() {
    let dir = mg_testkit::scratch_dir("ui-script-completion-width");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let long = "MyHelperWithAMuchLongerNameThanTheOthers";
    let mut text = String::from("void Zq() {}\n");
    for i in 0..30 {
        text.push_str(&format!("void MyHelper{i:02}() {{}}\n"));
    }
    text.push_str(&format!("void {long}() {{}}\nvoid main()\n{{\n    Zq\n    MyHe\n}}\n"));
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    // (Room for the longest name to the window's edge.)
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    let complete = |h: &mut Harness<'_, Moonglow>, word: &str| {
        h.key_press(egui::Key::Escape);
        let at = text.rfind(word).unwrap() + word.len();
        h.state_mut().script_tools.jump = Some((key, at));
        settle(h);
        h.key_press(egui::Key::F2);
        settle(h);
    };
    // (Its rows, over the editor: Reference lists the same names beside
    // it.)
    let row = |h: &Harness<'_, Moonglow>, name: &str| {
        let editor = (h.get_all_by_role(egui::accesskit::Role::MultilineTextInput))
            .find(|n| n.accesskit_node().value().is_some_and(|v| v.contains("main")))
            .expect("the script's text")
            .rect();
        (h.query_all_by_role_and_label(egui::accesskit::Role::Button, name))
            .map(|n| n.rect())
            .find(|r| r.left() < editor.right())
    };
    complete(&mut h, "Zq");
    let one = row(&h, "Zq").expect("a list of one name");
    complete(&mut h, "MyHe");
    let names = h.state().script_tools.completion.clone().expect("the list").items;
    assert_eq!(names.len(), 31);
    let font = egui::TextStyle::Button.resolve(&h.ctx.global_style());
    let wide = |name: &str| {
        let name = name.to_string();
        h.ctx.fonts_mut(|f| f.layout_no_wrap(name, font.clone(), egui::Color32::WHITE).size().x)
    };
    let widest = wide(long);
    let first = row(&h, &names[0].name).expect("its first row");
    assert!(first.width() > one.width(), "wider than the list of one");
    assert!(first.width() >= widest, "{} for a name {widest} wide", first.width());
    assert!(first.height() < 2.0 * font.size + 4.0, "one line a row");
    // Down to its last row: the list scrolls, its width the same.
    for _ in 0..names.len() {
        h.key_press(egui::Key::ArrowDown);
        h.step();
        let all: Vec<egui::Rect> = names.iter().filter_map(|s| row(&h, &s.name)).collect();
        assert_eq!(all.len(), names.len());
        for r in all {
            assert_eq!(r.width(), first.width(), "the same width as it scrolls");
            assert_eq!(r.height(), first.height(), "one line a row");
        }
    }
    // (Scrolled to its last row: in sight, under the list's top as it
    // opened, the first row gone above it.)
    let last = row(&h, long).unwrap();
    assert!(last.bottom() <= first.top() + 200.0 + 1.0, "the last row in sight: {last:?}");
    assert!(row(&h, &names[0].name).unwrap().top() < first.top(), "it scrolled");
    // At rest with it open.
    rest(&mut h);
    assert!(h.try_run().is_ok(), "the list keeps asking for frames");
    // A list of one name is no narrower than the least.
    complete(&mut h, "Zq");
    assert!(row(&h, "Zq").unwrap().width() >= 150.0);
}

/// The completion list's detail (5.2.0, Jasperre's report): the chosen
/// name's declaration and its documentation's first line beside the list,
/// inside the window, never over the list: at its right where there is
/// room, at its left by the window's right edge (a list made wide by a
/// long name), under or over it where neither side has room (the least
/// window, or a small one at 300%).
#[test]
fn the_completion_detail_stays_inside_the_window() {
    let pad = " ".repeat(100);
    let text = format!(
        "// Squares it.\nfloat Sqrx(float f) {{ return f * f; }}\n\
         void SqrxWithANameLongEnoughToMakeTheListWide() {{}}\nvoid main()\n{{\n    Sqr\n}}\n\
         {pad}Sqr\n"
    );
    // (300% as the window's room in points: a test's window keeps its
    // points whatever the zoom, so a window 1290 by 990 at 300% is one of
    // 430 by 330.)
    for (far, size, side) in [
        (false, (1600.0, 900.0), "right"),
        (true, (1100.0, 700.0), "left"),
        (true, (430.0, 330.0), "under or over"),
    ] {
        let dir = mg_testkit::scratch_dir(&format!("ui-script-detail-{}", side.len()));
        let path = sample_module(&dir);
        let key = ResKey::parse("hello", ResType::NSS).unwrap();
        let mut m = Module::open(&path).unwrap();
        m.set(key, text.as_bytes().to_vec());
        m.save().unwrap();
        let mut app = app_with(Vec::new());
        app.open_module(&path);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        let mut h = harness(app, size.0, size.1);
        settle(&mut h);
        let at = if far { text.rfind("Sqr") } else { text.find("    Sqr").map(|i| i + 4) };
        h.state_mut().script_tools.jump = Some((key, at.unwrap() + 3));
        settle(&mut h);
        h.key_press(egui::Key::F2);
        settle(&mut h);
        // (Not Reference's row, which the list may stand over.)
        let reference: Vec<egui::Rect> = (h.query_all_by_label(mg_ui::SCRIPT_REFERENCE))
            .flat_map(|g| g.query_all_by_label("Sqrx").map(|n| n.rect()).collect::<Vec<_>>())
            .collect();
        let row = (h.query_all_by_role_and_label(egui::accesskit::Role::Button, "Sqrx"))
            .map(|n| n.rect())
            .find(|r| !reference.contains(r))
            .expect("the list's row");
        let detail = h.get_by_label("float Sqrx(float f)").rect();
        h.get_by_label("Squares it.");
        let window = h.ctx.content_rect();
        assert!(window.expand(0.5).contains_rect(detail), "{side}: {detail:?} in {window:?}");
        assert!(!detail.intersects(row), "{side}: {detail:?} over the list's {row:?}");
        let placed = match side {
            "right" => detail.left() > row.right(),
            "left" => detail.right() < row.left(),
            _ => detail.top() > row.bottom() || detail.bottom() < row.top(),
        };
        assert!(placed, "{side}: {detail:?} by {row:?}");
    }
    // A tall list on the text's first line and a declaration that wraps
    // to six lines, its documentation to two, in the small window: no room
    // under the list nor over it, the detail still inside the window (over
    // the list's end).
    let long = "float fAVeryLongParameterName, float fAnotherVeryLongParameterName, \
                int nYetAnotherVeryLongOne, int nAFourthVeryLongParameter, \
                object oAFifthVeryLongParameter, string sASixthVeryLongParameter, \
                location lASeventhVeryLongParameter, int nAnEighthVeryLongParameter";
    let doc = "Squares it, and then says a great deal more about it than will fit \
               on one line of the detail beside the list";
    let mut text = String::from("void main() { Sqr\n}\n");
    for i in 0..12 {
        text.push_str(&format!("// {doc}.\nfloat Sqrx{i:02}({long}) {{ return 0.0; }}\n"));
    }
    let dir = mg_testkit::scratch_dir("ui-script-detail-tall");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, 430.0, 330.0);
    settle(&mut h);
    h.state_mut().script_tools.jump = Some((key, text.find("Sqr").unwrap() + 3));
    settle(&mut h);
    h.key_press(egui::Key::F2);
    settle(&mut h);
    let rows = h.query_all_by_role(egui::accesskit::Role::Button);
    let rows: Vec<egui::Rect> = rows
        .filter(|n| n.accesskit_node().label().is_some_and(|l| l.starts_with("Sqrx")))
        .map(|n| n.rect())
        .collect();
    assert!(rows.len() >= 6, "a tall list: {rows:?}");
    // (Its group, round the rows of its text, each a run of its own
    // where it wraps.)
    fn round(n: &egui_kittest::kittest::AccessKitNode<'_>) -> Option<egui::Rect> {
        if let Some(r) = n.bounding_box() {
            let at = |x: f64, y: f64| egui::pos2(x as f32, y as f32);
            return Some(egui::Rect::from_min_max(at(r.x0, r.y0), at(r.x1, r.y1)));
        }
        n.children().filter_map(|c| round(&c)).reduce(|a, b| a.union(b))
    }
    let group = h.get_by_label("Completion detail");
    let detail = round(&group.accesskit_node()).expect("the detail drawn");
    let said: String = (group.query_all_by(|n| n.value().is_some() || n.label().is_some()))
        .map(|n| n.accesskit_node().value().or(n.accesskit_node().label()).unwrap_or_default())
        .collect();
    assert!(said.contains("Sqrx00(") && said.contains("great deal"), "{said:?}");
    assert!(detail.height() > 100.0, "six lines and its documentation: {detail:?}");
    let window = h.ctx.content_rect();
    assert!(window.expand(0.5).contains_rect(detail), "tall: {detail:?} in {window:?}");
}

/// The sample module's `hello` script open in its editor (a window of its
/// own, the default switch), and a way to type at its top with its keys.
fn hello_window(name: &str) -> (Harness<'static, Moonglow>, std::path::PathBuf, ResKey) {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = window_harness(app);
    settle(&mut h);
    (h, path, key)
}

/// Text typed at the script's top, with the editor's keys. (Steps, not
/// `run`: see [`the_save_key_in_a_script_editor_saves_the_script_not_the_module`].)
fn type_at_top(h: &mut Harness<'_, Moonglow>, key: ResKey, text: &str) {
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(3);
    h.event(egui::Event::Text(text.into()));
    h.run_steps(3);
}

/// Whether Edit › Undo (`redo`: Redo) can be chosen now.
fn edit_undo_enabled(h: &Harness<'_, Moonglow>, redo: bool) -> bool {
    let id = if redo { mg_ui::commands::Id::Redo } else { mg_ui::commands::Id::Undo };
    id.why_disabled(h.state()).is_empty()
}

/// Edit › Undo and Redo, with the keys in a script's editor, follow that
/// script's own history (Wave 1 read it, where before they were always
/// on): Undo off with nothing typed, on once something is, and off again
/// with it undone, Redo on then.
#[test]
fn edit_undo_follows_a_script_s_own_history() {
    let (mut h, _path, key) = hello_window("ui-script-edit-undo");
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(3);
    assert!(!edit_undo_enabled(&h, false), "nothing typed: Undo off");
    assert!(!edit_undo_enabled(&h, true), "nothing undone: Redo off");
    type_at_top(&mut h, key, "// typed\n");
    assert!(h.state().script_text(key).unwrap().starts_with("// typed\n"));
    assert!(edit_undo_enabled(&h, false), "typed: Undo on at once");
    assert!(!edit_undo_enabled(&h, true), "Redo still off");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(h.state().script_text(key).unwrap(), "void main() { }\n", "undone");
    assert!(!edit_undo_enabled(&h, false), "all undone: Undo off");
    assert!(edit_undo_enabled(&h, true), "Redo on");
}

/// A script's tab closed with unsaved text asks Save, Don't Save or
/// Cancel (Wave 1, kept by the owner's ruling): Save puts the text into
/// the module and closes, Cancel keeps the tab and its text. (Don't Save:
/// `script_bottom`'s `closing_a_dirty_script_tab_asks_to_save_and_dont_save_drops_edits`.)
#[test]
fn closing_a_script_with_unsaved_text_saves_or_stays_as_answered() {
    let (mut h, _path, key) = hello_window("ui-script-close-answers");
    let tab = Tab::Script(key);
    let in_module = |h: &Harness<'_, Moonglow>| {
        let ws = h.state().ws.as_ref().unwrap();
        String::from_utf8_lossy(ws.module.get(&key).unwrap()).into_owned()
    };
    type_at_top(&mut h, key, "// kept\n");
    // Cancel: the tab stays, its text with it.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab.clone()]));
    settle(&mut h);
    let ask = h.get_by_label("Unsaved Changes");
    ask.get_by_label("Cancel").click();
    settle(&mut h);
    assert!(h.query_by_label("Unsaved Changes").is_none(), "answered");
    assert!(h.state().dock.find_tab(&tab).is_some(), "Cancel: the tab stays");
    assert!(h.state().script_text(key).unwrap().starts_with("// kept\n"), "and its text");
    assert_eq!(in_module(&h), "void main() { }\n", "nothing saved");
    // Save: into the module, and closed.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab.clone()]));
    settle(&mut h);
    let ask = h.get_by_label("Unsaved Changes");
    ask.get_by_label("Save").click();
    settle(&mut h);
    assert!(h.query_by_label("Unsaved Changes").is_none(), "answered");
    assert!(h.state().dock.find_tab(&tab).is_none(), "Save: the tab closed");
    assert!(in_module(&h).starts_with("// kept\n"), "saved into the module: {}", in_module(&h));
}

/// A script's tab closed with unsaved text and answered Save keeps its
/// editor's state for the session, as a clean close does (the owner's
/// ruling of 2026-11-25): opened again, its bookmark and its history are
/// there, and nothing is unsaved in it. (Don't Save lets it go:
/// `script_bottom`'s `closing_a_dirty_script_tab_asks_to_save_and_dont_save_drops_edits`.)
#[test]
fn a_script_closed_with_save_keeps_its_bookmarks_and_history() {
    let (mut h, _path, key) = hello_window("ui-script-close-save-keeps");
    let tab = Tab::Script(key);
    let in_module = |h: &Harness<'_, Moonglow>| {
        let ws = h.state().ws.as_ref().unwrap();
        String::from_utf8_lossy(ws.module.get(&key).unwrap()).into_owned()
    };
    type_at_top(&mut h, key, "// kept\n");
    // Its first line bookmarked (F5).
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(3);
    h.key_press(egui::Key::F5);
    h.run_steps(3);
    assert_eq!(h.state().script_bookmarks(key), [0], "bookmarked");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab.clone()]));
    settle(&mut h);
    h.get_by_label("Unsaved Changes").get_by_label("Save").click();
    settle(&mut h);
    assert!(h.query_by_label("Unsaved Changes").is_none(), "answered");
    assert!(h.state().dock.find_tab(&tab).is_none(), "Save: the tab closed");
    assert!(in_module(&h).starts_with("// kept\n"), "saved into the module: {}", in_module(&h));
    assert_eq!(h.state().script_bookmarks(key), [0], "its bookmark kept while closed");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(&mut h);
    assert!(h.state().script_text(key).unwrap().starts_with("// kept\n"));
    assert_eq!(h.state().script_bookmarks(key), [0], "its bookmark kept");
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(3);
    assert!(edit_undo_enabled(&h, false), "its history kept: Undo on");
    // Not unsaved: closed again, nothing is asked.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab.clone()]));
    settle(&mut h);
    assert!(h.query_by_label("Unsaved Changes").is_none(), "the kept buffer is not unsaved");
    assert!(h.state().dock.find_tab(&tab).is_none(), "closed");
}

/// A clean script closed by its header's Cancel keeps its editor's state
/// for the session, as its × does (the owner's ruling of 2026-11-25):
/// opened again, its bookmark and its history are there.
#[test]
fn a_clean_script_closed_with_the_header_s_cancel_keeps_its_bookmarks_and_history() {
    let (mut h, _path, key) = hello_window("ui-script-cancel-clean");
    let tab = Tab::Script(key);
    type_at_top(&mut h, key, "// one\n");
    // The script's own Save (Ctrl+S with its keys): nothing unsaved.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run_steps(3);
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(3);
    h.key_press(egui::Key::F5);
    h.run_steps(3);
    assert_eq!(h.state().script_bookmarks(key), [0], "bookmarked");
    h.get_by_label(mg_ui::SCRIPT_HEADER).get_by_label("Cancel").click();
    settle(&mut h);
    assert!(h.query_by_label("Unsaved Changes").is_none(), "nothing asked");
    assert!(h.state().dock.find_tab(&tab).is_none(), "Cancel: the tab closed");
    assert_eq!(h.state().script_bookmarks(key), [0], "its bookmark kept while closed");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(&mut h);
    assert!(h.state().script_text(key).unwrap().starts_with("// one\n"));
    assert_eq!(h.state().script_bookmarks(key), [0], "its bookmark kept");
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(3);
    assert!(edit_undo_enabled(&h, false), "its history kept: Undo on");
}

/// A script's tab closed with nothing unsaved asks nothing, and its
/// editor's state stays for the session, as before Wave 1 (develop kept
/// the script's buffer on a close): opened again, its bookmarks and its
/// history are there.
#[test]
fn a_clean_script_closed_and_opened_again_keeps_its_bookmarks_and_history() {
    let (mut h, _path, key) = hello_window("ui-script-close-clean");
    let tab = Tab::Script(key);
    type_at_top(&mut h, key, "// one\n");
    // The script's own Save (Ctrl+S with its keys): nothing unsaved.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run_steps(3);
    // Its first line bookmarked (F5).
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(3);
    h.key_press(egui::Key::F5);
    h.run_steps(3);
    assert_eq!(h.state().script_bookmarks(key), [0], "bookmarked");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![tab.clone()]));
    settle(&mut h);
    assert!(h.query_by_label("Unsaved Changes").is_none(), "nothing asked");
    assert!(h.state().dock.find_tab(&tab).is_none(), "closed");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(&mut h);
    assert!(h.state().script_text(key).unwrap().starts_with("// one\n"));
    assert_eq!(h.state().script_bookmarks(key), [0], "its bookmark kept");
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run_steps(3);
    assert!(edit_undo_enabled(&h, false), "its history kept: Undo on");
}

/// A script's tab closed with nothing unsaved keeps its editor's state
/// for the session, but the script is no longer open: the Find window
/// left on it changes nothing (its Replace All made the hidden text
/// unsaved, with no tab to ask about it, and the module's Save wrote
/// it), and Find In Currently Open Scripts does not search it.
#[test]
fn a_closed_script_is_not_open_to_find_and_replace() {
    let dir = mg_testkit::scratch_dir("ui-replace-closed");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let other = ResKey::parse("other", ResType::NSS).unwrap();
    let text = "void main()\n{\n    int a = a + a;\n}\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.set(other, b"void main()\n{\n    int a = 2;\n}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    for k in [key, other] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(k)));
        settle(&mut h);
    }
    let search = &mut h.state_mut().script_tools.search;
    (search.open, search.replace_mode, search.script) = (true, true, Some(key));
    (search.find, search.replace) = ("a".into(), "total".into());
    search.options.whole_word = true;
    settle(&mut h);
    // (A bookmark: what only the editor's state has, not the module.)
    h.state_mut().scripts_mut_bookmark_for_test(key, 0);
    close_tab(h.state_mut(), &Tab::Script(key));
    settle(&mut h);
    assert_eq!(h.state().script_text(key).as_deref(), Some(text), "its editor's state kept");
    assert_eq!(h.state().script_bookmarks(key), [0], "its bookmark with it");
    h.get_by_label("Replace All").click();
    settle(&mut h);
    assert_eq!(h.state().script_text(key).as_deref(), Some(text), "the closed script untouched");
    assert!(!h.state().has_unsaved_work(), "nothing unsaved");
    let search = &mut h.state_mut().script_tools.search;
    (search.replace_mode, search.in_open, search.script) = (false, true, Some(other));
    settle(&mut h);
    h.get_by_label("Find All").click();
    settle(&mut h);
    let hits = &h.state().search_results.hits;
    assert!(!hits.is_empty(), "the open script searched");
    assert!(
        hits.iter().all(|hit| matches!(hit, Hit::Line { script, .. } if *script == other)),
        "the closed script not searched: {hits:?}"
    );
    // Deleted, its editor's state is let go of (kept, it stayed until the
    // module closed).
    let gone = mg_edit::Edit::SetResource { key, data: None };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Delete", vec![gone])));
    settle(&mut h);
    assert_eq!(h.state().script_text(key), None, "a deleted script's state let go of");
}
