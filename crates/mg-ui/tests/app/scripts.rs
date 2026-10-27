//! The script editor and compiling.

use crate::common::*;

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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    let names = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let symbols = h.state().script_tools.symbols.get(&nss("uses_a"));
        symbols.map(|(_, s)| s.iter().map(|s| s.name.clone()).collect()).unwrap_or_default()
    };
    let open = |h: &mut Harness<'_, Moonglow>, name: &str| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(nss(name))));
        h.run();
    };
    open(&mut h, "uses_a");
    assert!(names(&h).contains(&"Old".to_string()), "{:?}", names(&h));
    open(&mut h, "inc_a");
    h.state_mut().script_tools.jump = Some((nss("inc_a"), 0));
    h.run();
    h.event(egui::Event::Text("int New() { return 2; }\n".into()));
    h.run();
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Compile").click();
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
        let field = w.get_all_by_role(egui::accesskit::Role::TextInput).next().expect("a field");
        field.click();
        h.run();
        let w = h.get_all_by_label("Script Wizard").find(window).unwrap();
        w.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().type_text(text);
        h.run();
    };

    // A condition: female, carrying key_1.
    h.get_by_label("Script Wizard…").click();
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
    h.get_by_label("Actions Taken").click();
    h.run();
    h.get_by_label("Script Wizard…").click();
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

/// The open Edit menu's Undo or Redo row (`what`), as it reads (with its
/// key).
fn edit_row(h: &Harness<'_, Moonglow>, what: &str) -> String {
    let rows: Vec<String> = h
        .query_all_by_label_contains(what)
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
    h.get_by_label("Edit").click();
    h.run_steps(2);
    assert_eq!(edit_row(&h, "Undo"), "Undo Ctrl+Z", "not the module's step");
    h.get_by_label("Undo Ctrl+Z").click();
    h.run_steps(2);
    assert_eq!(script(&h), text, "the new line is taken back");
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "not the module's Undo");
    assert!(script_has_keys(&h, key), "the keys back in the editor");
    h.get_by_label("Edit").click();
    h.run_steps(2);
    let redo = edit_row(&h, "Redo");
    h.get_by_label(&redo).click();
    h.run_steps(2);
    assert_eq!(script(&h), edited, "and Redo gives it again");
    assert!(script_has_keys(&h, key));
    // The toolbar's, the same.
    h.get_by_label("⟲ Undo").click();
    h.run_steps(2);
    assert_eq!(script(&h), text);
    assert!(script_has_keys(&h, key));
    h.get_by_label("⟳ Redo").click();
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
    // menu names the module's step.
    h.get_by_label("Areas (1)").click();
    h.run_steps(2);
    assert!(!script_has_keys(&h, key));
    h.get_by_label("Edit").click();
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
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label(&edit_row(&h, "Undo")).click();
    h.run_steps(2);
    assert_eq!(h.state().script_text(key).as_deref(), Some(text));
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "not the module's Undo");
    h.get_by_label("⟳ Redo").click();
    h.run_steps(2);
    assert_eq!(h.state().script_text(key), Some(edited));
    assert!(script_has_keys(&h, key));
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // An edit, then Save: the script is compiled too, with its .ndb.
    h.state_mut().script_tools.jump = Some((key, text.len()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    h.get_by_label("Save").click();
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    assert_eq!(h.state().script_text(key).unwrap(), text);
    h.get_by_label("Windows-1250");
    // A line typed at the top, then Compile (which saves): the rest is
    // the file's bytes as they were.
    h.state_mut().script_tools.jump = Some((key, 0));
    h.run();
    h.event(egui::Event::Text("// ę\n".into()));
    h.run();
    h.get_by_label("Compile").click();
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
    h.get_by_label("Save").click();
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
fn build_on_save_opens_the_results_when_something_is_wrong() {
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let build = h.state().build.clone().expect("the Build Module window, with the problem");
    assert!(build.results.iter().any(|f| f.text.starts_with("Error:")), "{:?}", build.results);
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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
    h.get_by_label("Edit").click();
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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
    h.get_by_label("Save").click();
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    assert!(!h.state().settings.auto_compile);
    for letter in ["a", "b"] {
        h.state_mut().script_tools.jump = Some((key, 0));
        h.run();
        h.event(egui::Event::Text(letter.into()));
        h.run();
        h.get_by_label("Save").click();
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Nothing typed: nothing said of the script.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(h.state().script_tools.messages.is_empty());
    h.state_mut().script_tools.info = mg_ui::script_tools::InfoTab::Help;
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

/// Help shows a function's documentation as the source has it: the
/// comment lines with their `//` and their columns, then the declaration
/// (Jasperre's report: lists in nwscript.nss's comments lost their
/// alignment, the `//` and each line's indentation stripped).
#[test]
fn help_shows_documentation_as_the_source_has_it() {
    let dir = mg_testkit::scratch_dir("ui-script-help");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "// Adds.\r\n// - nA:    the first\r\n//           (any)\r\nint Add(int nA) { return nA; }\r\nvoid main() { Add(1); }\r\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.get_by_label("Add").click();
    h.run();
    assert_eq!(h.state().script_tools.info, mg_ui::script_tools::InfoTab::Help);
    let shown = h.get_by_label_contains("// - nA:").accesskit_node().value().unwrap_or_default();
    assert_eq!(
        shown, "// Adds.\n// - nA:    the first\n//           (any)\nint Add(int nA)",
        "as written, then the declaration"
    );
}

/// A `/* … */` comment directly above a function is its documentation in
/// Help, as written, as BioWare's includes put theirs between `//::` lines
/// (Jasperre's report); above the definition, where the prototype has none.
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
    h.run();
    h.get_by_label("Hidden").click();
    h.run();
    let shown =
        h.get_by_label_contains("Returns TRUE").accesskit_node().value().unwrap_or_default();
    assert_eq!(
        shown,
        "//:: Hidden\n/*\n    Returns TRUE if oSelf is hidden\n*/\nint Hidden(object oSelf)"
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, b"void main() {}\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.external_editor = Some(editor.into());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    h.get_by_label("External Editor").click();
    h.run_steps(3);
    // The editor's file: the script; an edit saved there comes back.
    let file =
        std::env::temp_dir().join(format!("moonglow-{}", std::process::id())).join("hello.nss");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "void main() {}\n");
    std::fs::write(&file, "void main() { int n; }\n").unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(later).unwrap();
    h.run_steps(3);
    assert_eq!(h.state().script_text(key).unwrap(), "void main() { int n; }\n");
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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
    app.print_dir = dir.join("print");
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Print…").click();
    h.run();
    let page = std::fs::read_to_string(dir.join("print").join("hello.html")).unwrap();
    assert!(page.contains(" &lt; ") && page.contains("window.print()"), "{page}");
    assert!(page.contains(">int</span>"), "keywords are coloured spans");
}

/// Edit beside a script's name makes a script that is nowhere (as Aurora
/// does) and opens it; the module's and the game's open as they are.
#[test]
fn edit_beside_a_script_s_name_creates_a_missing_script() {
    let Some((mut h, _)) = area_harness("edit-script") else { return };
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
    let Some((mut h, _)) = area_harness("script-external") else { return };
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
    h.state_mut().show_references(&d);
    let results: Vec<String> = h
        .state()
        .script_tools
        .search
        .results
        .iter()
        .map(|(k, l, _)| format!("{}:{l}", k.resref))
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
    h.run_steps(3);
    // (Where the text begins: the error's line is kept for it, there or
    // not, so the text does not move as one comes and goes; Jasperre's
    // report.)
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
    h.run_steps(2);
    let error = h.state().script_error_for_test(main);
    assert_eq!(error.as_ref().map(|e| e.0), Some(Some(3)), "{error:?}");
    h.get_by_label_contains("⚠ line 4:");
    assert_eq!(top(&h), before, "the text stays where it was");
    // Options › Script Editor: no checking as you type.
    h.state_mut().settings.no_live_check = true;
    h.run_steps(2);
    assert!(h.query_by_label_contains("⚠").is_none());
    assert!(h.state().script_error_for_test(main).is_none(), "nor its underline");
    assert!(top(&h) < before, "and no line kept for one");
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
    let Some((mut h, _)) = area_harness("save-compiles") else { return };
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
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut m = new_module(&game, "Outside", &mut fastrand::Rng::with_seed(7)).unwrap();
    let (nss, ncs) = (
        ResKey::parse("hello", ResType::NSS).unwrap(),
        ResKey::parse("hello", ResType::NCS).unwrap(),
    );
    m.set(nss, b"void main()\n{\n}\n".to_vec());
    let dir = mg_testkit::scratch_dir("ui-folder-compile").join("module");
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
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

/// Compile All Scripts passes over the scripts that are as they were when
/// it last compiled them: after an include changes, only the scripts that
/// include it are compiled again.
#[test]
fn compile_all_compiles_only_what_changed() {
    let Some((mut h, _)) = area_harness("compile-changed") else { return };
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
    assert!(debug.starts_with("Compiled 2 scripts: 0 failed, 2 changed"), "{debug}");
    let has = |h: &Harness<'_, Moonglow>, name: &str| {
        let key = ResKey::parse(name, ResType::NDB).unwrap();
        h.state().ws.as_ref().unwrap().module.contains(&key)
    };
    assert!(has(&h, "uses_n") && has(&h, "alone") && !has(&h, "inc_n"));
    let again = compile(&mut h);
    assert!(again.starts_with("Compiled 0 scripts"), "{again}");
}

/// In Open Script, Down and Up choose among the scripts listed while the
/// name is typed, and Enter opens the one chosen (the first, unmoved).
#[test]
fn open_script_s_arrows_choose_among_those_listed() {
    let Some((mut h, _)) = area_harness("open-script-keys") else { return };
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key: hello, data: Some(b"void main() { }\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(hello)));
    h.run_steps(4);
    h.get_by_label("Open…").click();
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

/// A script editor's Open… lists the scripts there are to open, the
/// module's alone or its haks' or all of them, found by name; the game's
/// open to be read.
#[test]
fn a_script_editor_opens_other_scripts() {
    let Some((mut h, _)) = area_harness("open-script") else { return };
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
    h.get_by_label("Open…").click();
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
    h.get_by_label("Open…").click();
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
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
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
        h.get_by_label("Open…").click();
        h.run_steps(3);
        // (Past the double-click time of the click before.)
        h.run_steps(30);
    };
    window(&mut h);
    h.get_by_label("7 scripts");
    // Open is off with nothing selected.
    assert!(h.get_by_label("Open").accesskit_node().is_disabled());
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

/// A double click at `at`: two clicks, a frame each.
/// A double click on a widget: both clicks in one frame (the harness gives
/// each event it queues a frame of its own, a quarter of a second by
/// default, longer than egui's double-click time), a second past any
/// click before at 60 frames a second (else the three are a triple click).
fn double_click_now(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    h.run_steps(60);
    let button = egui::PointerButton::Primary;
    for pressed in [true, false, true, false] {
        let modifiers = egui::Modifiers::NONE;
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers,
        });
    }
    h.run_steps(2);
}
