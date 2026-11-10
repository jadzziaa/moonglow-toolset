//! Talk tables and 2DA tables.

use crate::common::*;

/// File dialogs that answer nothing and note what they were asked for.
struct Asked(std::rc::Rc<std::cell::RefCell<Vec<(mg_ui::FileKind, bool)>>>);

impl mg_ui::Dialogs for Asked {
    fn open_file(
        &mut self,
        kind: mg_ui::FileKind,
        _: Option<&std::path::Path>,
    ) -> Option<std::path::PathBuf> {
        self.0.borrow_mut().push((kind, false));
        None
    }
    fn save_file(
        &mut self,
        kind: mg_ui::FileKind,
        _: Option<&std::path::Path>,
    ) -> Option<std::path::PathBuf> {
        self.0.borrow_mut().push((kind, true));
        None
    }
    fn pick_folder(&mut self, _: &str, _: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
        None
    }
}

/// The 2DA and talk table editors' file dialogs are of their own kinds
/// (titled for them, showing their files), not any file's.
#[test]
fn tables_and_talk_tables_ask_for_their_own_files() {
    use mg_ui::FileKind;
    let asked = std::rc::Rc::default();
    let mut h =
        harness(Moonglow::new(None, Box::new(Asked(std::rc::Rc::clone(&asked)))), 900.0, 700.0);
    mg_ui::table_view::open_file(h.state_mut());
    let asks = |h: &mut Harness<'_, Moonglow>, actions: Vec<mg_ui::Action>| {
        for a in actions {
            h.state_mut().actions.push(a);
            h.run_steps(1);
        }
    };
    asks(&mut h, vec![mg_ui::Action::TalkFile(false), mg_ui::Action::TalkFile(true)]);
    // (Its lines, of a table open.)
    let dir = mg_testkit::scratch_dir("ui-talk-kinds");
    h.state_mut().talk =
        Some(mg_module::talk::Table::create("kinds", &dir, mg_core::Language::ENGLISH, false));
    let lines = [true, false].map(mg_ui::Action::TalkCsv).into_iter();
    asks(&mut h, lines.chain([mg_ui::Action::TalkJson(false)]).collect());
    let lines = |json| FileKind::TalkLines { json };
    assert_eq!(
        *asked.borrow(),
        [
            (FileKind::Table, false),
            (FileKind::TalkTable, false),
            (FileKind::TalkTable, true),
            (lines(false), false),
            (lines(false), true),
            (lines(true), true),
        ]
    );
    assert_eq!(FileKind::Table.filter(false), Some(("2DA tables", &["2da"][..])));
    assert_eq!(FileKind::TalkTable.title(true), "New Talk Table");
    assert_eq!(lines(true).filter(true), Some(("JSON", &["json"][..])));
}

/// The Talk Table editor makes and opens a `.tlk` file on its own, with no
/// module open.
/// A line of the talk table is chosen by a click anywhere on its row, the
/// arrow keys choose from there, and Enter goes to the line's text
/// (GitHub issue 24).
#[test]
fn a_talk_table_s_rows_begin_under_its_buttons() {
    // Open File… and New File… stood in a sliver past the end of the row
    // before them, one under the other, and the list began a third of
    // the window down.
    use mg_module::talk::{Line, Table};
    let dir = mg_testkit::scratch_dir("ui-talk-top");
    let mut h = harness(Moonglow::new(None, Box::new(NoDialogs::default())), 1000.0, 800.0);
    let mut table = Table::create("top", &dir, mg_core::Language::ENGLISH, false);
    let line =
        Line { text: "Text 0".into(), feminine: None, sound: String::new(), sound_length: 0.0 };
    table.add_line(&line).unwrap();
    h.state_mut().talk = Some(table);
    h.state_mut().talk_view.outside = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run_steps(6);
    let buttons = ["Open File…", "New File…"].map(|l| h.get_by_label(l).rect());
    let go_to = h.get_by_label("Go to").rect();
    let row = h.get_by_label("Text 0").rect();
    for b in buttons {
        assert!(b.width() > 40.0, "a button of its text's width: {b:?}");
        assert!(b.top() < go_to.bottom() + 30.0, "in the row, or the next: {b:?}");
        assert!(row.top() < b.bottom() + 30.0, "the list right under them: {row:?}, {b:?}");
    }
}

#[test]
fn a_talk_table_s_row_is_clicked_anywhere() {
    use mg_module::talk::{Line, Table};
    let dir = mg_testkit::scratch_dir("ui-talk-rows");
    let mut h = harness(Moonglow::new(None, Box::new(NoDialogs::default())), 1000.0, 800.0);
    let mut table = Table::create("rows", &dir, mg_core::Language::ENGLISH, false);
    for row in 0..8 {
        let line = Line {
            text: format!("Text {row}"),
            feminine: None,
            sound: String::new(),
            sound_length: 0.0,
        };
        table.add_line(&line).unwrap();
    }
    h.state_mut().talk = Some(table);
    h.state_mut().talk_view.outside = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run_steps(6);
    assert_eq!(h.state().talk_view.selected, None);
    // The text of a row, well to the right of its number.
    let at = h.get_by_label("Text 3").rect().center() + egui::vec2(200.0, 0.0);
    let click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(2);
    };
    click(&mut h, at);
    assert_eq!(h.state().talk_view.selected, Some(3));
    // The keys are the list's now, wherever the pointer is.
    h.hover_at(egui::pos2(5.0, 5.0));
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(h.state().talk_view.selected, Some(5));
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    h.key_press(egui::Key::End);
    h.run_steps(2);
    assert_eq!(h.state().talk_view.selected, Some(7));
    h.key_press(egui::Key::Home);
    h.run_steps(2);
    assert_eq!(h.state().talk_view.selected, Some(0));
    // Enter: to the line's text, where what is typed goes; the arrow keys
    // are the text's there.
    h.key_press(egui::Key::Enter);
    h.run_steps(2);
    h.event(egui::Event::Text("!".into()));
    h.run_steps(2);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(h.state().talk_view.selected, Some(0));
    assert!(h.state().talk.as_ref().unwrap().line(0).text.contains('!'));
}

/// The line between the talk table's list and the chosen line's text is
/// dragged up and down (GitHub issue 24), held between the text boxes'
/// three lines and seven tenths of the tab.
#[test]
fn the_talk_table_s_split_is_dragged() {
    use mg_module::talk::{Line, Table};
    let dir = mg_testkit::scratch_dir("ui-talk-split");
    let mut h = harness(Moonglow::new(None, Box::new(NoDialogs::default())), 1000.0, 800.0);
    let mut table = Table::create("split", &dir, mg_core::Language::ENGLISH, false);
    for row in 0..60 {
        let text = format!("Text {row}");
        table
            .add_line(&Line { text, feminine: None, sound: String::new(), sound_length: 0.0 })
            .unwrap();
    }
    h.state_mut().talk = Some(table);
    h.state_mut().talk_view.outside = true;
    h.state_mut().talk_view.selected = Some(5);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run_steps(8);
    let strref = |h: &Harness<'_, Moonglow>| h.get_by_label("StrRef 16777221").rect().top();
    let top = h.get_by_label("Text 0").rect().top();
    let drag = |h: &mut Harness<'_, Moonglow>, by: f32| {
        // (The line is just above the chosen line's StrRef.)
        let from = egui::pos2(500.0, strref(h) - 5.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        h.hover_at(from);
        h.run_steps(1);
        h.event(button(from, true));
        h.run_steps(1);
        for k in 1..=6 {
            h.hover_at(from + egui::vec2(0.0, by * k as f32 / 6.0));
            h.run_steps(1);
        }
        h.event(button(from + egui::vec2(0.0, by), false));
        h.run_steps(3);
    };
    let before = strref(&h);
    drag(&mut h, -80.0);
    let after = strref(&h);
    assert!((before - 80.0 - after).abs() < 1.0, "dragged up: {before} to {after}");
    // Up to seven tenths of the tab: the list keeps its room.
    drag(&mut h, -2000.0);
    let highest = strref(&h);
    assert!(highest < after - 20.0 && highest > top + 150.0, "held: {highest}, {top}");
    drag(&mut h, -300.0);
    assert!((strref(&h) - highest).abs() < 1.0, "no higher");
    // Down to the text box's three lines, the sound's row still in sight.
    drag(&mut h, 2000.0);
    let lowest = strref(&h);
    assert!(lowest > before + 50.0, "dragged down: {lowest}");
    assert!(h.get_by_label("Sound").rect().bottom() < 800.0);
    drag(&mut h, 300.0);
    assert!((strref(&h) - lowest).abs() < 1.0, "no lower");
}

#[test]
fn a_talk_table_line_with_crlf_shows_and_keeps_plain_line_ends() {
    // Jasperre's report: a `\r` kept in the field is a stop of the caret
    // no one sees; End put what was typed after it ("one\rX\r\ntwo").
    use mg_module::talk::{Line, Table};
    let dir = mg_testkit::scratch_dir("ui-talk-crlf");
    let mut h = harness(Moonglow::new(None, Box::new(NoDialogs::default())), 1000.0, 800.0);
    let mut table = Table::create("crlf", &dir, mg_core::Language::ENGLISH, false);
    for text in ["one\r\ntwo", "three"] {
        let line =
            Line { text: text.into(), feminine: None, sound: String::new(), sound_length: 0.0 };
        table.add_line(&line).unwrap();
    }
    h.state_mut().talk = Some(table);
    h.state_mut().talk_view.outside = true;
    h.state_mut().talk_view.selected = Some(0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run_steps(6);
    let input = |h: &Harness<'_, Moonglow>, value: &str| {
        h.get_all_by_value(value)
            .find(|n| n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput)
            .is_some()
    };
    assert!(input(&h, "one\ntwo"), "shown with a plain line end");
    h.get_all_by_value("one\ntwo")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput)
        .unwrap()
        .click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Home);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("X".into()));
    h.run_steps(2);
    let talk = h.state().talk.as_ref().unwrap();
    assert_eq!(talk.line(0).text, "oneX\r\ntwo", "its line end kept, the X before it");
    // A line of a single line given a second: plain, as the game's own
    // talk table has them all.
    h.key_press(egui::Key::Escape);
    h.state_mut().talk_view.selected = Some(1);
    h.run_steps(4);
    h.get_all_by_value("three")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput)
        .unwrap()
        .click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::End);
    h.key_press(egui::Key::Enter);
    h.event(egui::Event::Text("four".into()));
    h.run_steps(2);
    assert_eq!(h.state().talk.as_ref().unwrap().line(1).text, "three\nfour");
}

#[test]
fn a_talk_table_file_is_made_and_opened_on_its_own() {
    let dir = mg_testkit::scratch_dir("ui-talk-file");
    let file = dir.join("strings.tlk");
    let dialogs =
        NoDialogs { open: vec![file.clone()], save: vec![file.clone()], ..Default::default() };
    let mut h = harness(Moonglow::new(None, Box::new(dialogs)), 1000.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run();
    h.get_by_label("New File…").click();
    h.run();
    assert!(file.is_file());
    h.get_by_label("Add Line").click();
    h.run();
    type_into_hint(&mut h, "the line's text", "On its own");
    h.get_by_label("Save").click();
    h.run();
    let read = mg_tlk::Tlk::read(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(read.text(mg_core::StrRef(0)).as_deref(), Some("On its own"));
    // Opened again from the file.
    h.state_mut().talk = None;
    h.state_mut().talk_view = Default::default();
    h.run();
    h.get_by_label("Open File…").click();
    h.run();
    h.get_by_label("16777216");
    assert_eq!(h.state().talk.as_ref().unwrap().line(0).text, "On its own");
    // Go to takes a StrRef or a line's number: that line is chosen and
    // the list goes to it. A long text scrolls in its box: the sound's row
    // under it stays in the window.
    let long = (1..=60).map(|n| format!("Line {n}")).collect::<Vec<_>>().join("\n");
    for row in 1..120 {
        let line = mg_module::talk::Line {
            text: if row == 100 { long.clone() } else { format!("Text {row}") },
            feminine: None,
            sound: String::new(),
            sound_length: 0.0,
        };
        h.state_mut().talk.as_mut().unwrap().add_line(&line).unwrap();
    }
    h.run();
    assert!(h.query_by_label("16777316").is_none(), "far down the list");
    type_into_hint(&mut h, "StrRef or line", "16777316");
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
    assert_eq!(h.state().talk_view.selected, Some(100));
    h.get_by_label("16777316");
    let sound = h.get_by_label("Sound").rect();
    assert!(
        sound.bottom() < 800.0 && sound.top() > 400.0,
        "the sound's row is in sight: {sound:?}"
    );
    // A line's number goes there too; past the end, it says what there is.
    h.state_mut().talk_view.go_to.clear();
    h.run();
    type_into_hint(&mut h, "StrRef or line", "3");
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().talk_view.selected, Some(3));
    h.state_mut().talk_view.go_to.clear();
    h.run();
    type_into_hint(&mut h, "StrRef or line", "5000");
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().talk_view.selected, Some(3), "nowhere to go");
    assert!(h.query_by_label_contains("The table has 120 lines").is_some());
}

/// A talk table the module names that is there but cannot be read is tried
/// once and said once, not every frame the editor is drawn.
#[test]
fn a_talk_table_that_cannot_be_read_is_tried_once() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-bad");
    let user = user_folder(&dir, &["tlk"]);
    std::fs::write(user.join("tlk/mg_bad.tlk"), b"not a talk table").unwrap();
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_bad".to_vec()));
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run_steps(8);
    let said: Vec<&String> = h
        .state()
        .log
        .entries
        .iter()
        .map(|(_, m)| m)
        .filter(|m| m.starts_with("Talk table mg_bad"))
        .collect();
    assert_eq!(said.len(), 1, "{said:?}");
}

#[test]
fn talk_table_made_edited_saved_and_used_by_strings() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-table");
    let user = user_folder(&dir, &[]);
    let path = sample_module(&dir);
    // (Export CSV and Import CSV are answered with the same file.)
    let csv = dir.join("lines.csv");
    let dialogs =
        NoDialogs { open: vec![csv.clone(); 2], save: vec![csv.clone(); 2], ..Default::default() };
    let mut app = game_app_with(&root, Some(user.clone()), dialogs);
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run();
    h.get_by_label("This module has no talk table of its own.");
    type_into_hint(&mut h, "name", "mg_talk");
    h.get_by_label("With a feminine table").click();
    h.run();
    h.get_by_label("Create").click();
    h.run();
    h.run();
    // The table is made in the tlk folder and named by the module.
    assert!(user.join("tlk/mg_talk.tlk").is_file());
    assert!(user.join("tlk/mg_talkf.tlk").is_file());
    let named = |h: &mut Harness<'_, Moonglow>| {
        h.state().ws.as_ref().unwrap().module.custom_tlk().unwrap().unwrap_or_default()
    };
    assert_eq!(named(&mut h), "mg_talk");
    assert!(h.state().game.as_deref().unwrap().custom_tlk().is_some());

    // A line, typed: one undo takes the typing back.
    h.get_by_label("Add Line").click();
    h.run();
    h.get_by_label("16777216");
    type_into_hint(&mut h, "the line's text", "Greetings");
    type_into_hint(&mut h, "the feminine text", "Greetings, lady");
    let line = |h: &Harness<'_, Moonglow>| h.state().talk.as_ref().unwrap().line(0);
    assert_eq!(line(&h).text, "Greetings");
    assert_eq!(line(&h).feminine.as_deref(), Some("Greetings, lady"));
    h.get_by_label("Undo").click();
    h.run();
    assert_eq!((line(&h).text.as_str(), line(&h).feminine.as_deref()), ("Greetings", Some("")));
    h.get_by_label("Redo").click();
    h.run();
    assert!(h.state().has_unsaved_work());
    // Ctrl+Z, with the pointer over the editor, is the table's too.
    let over = h.get_by_label("Add Line").rect().center();
    h.hover_at(over);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(line(&h).feminine.as_deref(), Some(""), "the table's undo");
    h.get_by_label("Redo").click();
    h.run();
    assert_eq!(line(&h).feminine.as_deref(), Some("Greetings, lady"));
    // In the line's text, typed in: Ctrl+Z is the field's, not the
    // table's (GitHub issue 18): the feminine text stays.
    let typed = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.value().as_deref() == Some("Greetings")
            && matches!(
                n.role(),
                egui::accesskit::Role::TextInput | egui::accesskit::Role::MultilineTextInput
            )
    };
    h.get(egui_kittest::kittest::by().predicate(typed)).click();
    h.run();
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text(" friend".into()));
    h.run_steps(2);
    assert!(h.ctx.egui_wants_keyboard_input());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(line(&h).feminine.as_deref(), Some("Greetings, lady"), "not the table's undo");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);

    // The lines go out as CSV; changed in a spreadsheet, they come back.
    h.get_by_label("Export CSV…").click();
    h.run();
    let sheet = std::fs::read_to_string(&csv).unwrap();
    assert!(sheet.contains("16777216,Greetings,\"Greetings, lady\""), "{sheet}");
    std::fs::write(&csv, sheet.replace("16777216,Greetings,", "16777216,Well met,")).unwrap();
    h.get_by_label("Import CSV…").click();
    h.run();
    assert_eq!(line(&h).text, "Well met");
    h.get_by_label("Undo").click();
    h.run();
    assert_eq!(line(&h).text, "Greetings", "the import is one undo");

    // As JSON too (nwn_tlk's, as a nasher repository keeps a table): the
    // lines with text, by their numbers in the table.
    h.get_by_label("Export JSON…").click();
    h.run();
    let json = std::fs::read_to_string(&csv).unwrap();
    let read: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(read["entries"][0], serde_json::json!({"id": 0, "text": "Greetings"}));
    let other = r#"{"language":0,"entries":[{"id":0,"text":"Hail"},{"id":40,"text":"Far"}]}"#;
    std::fs::write(&csv, other).unwrap();
    h.get_by_label("Import JSON…").click();
    h.run();
    assert_eq!((line(&h).text.as_str(), h.state().talk.as_ref().unwrap().len()), ("Hail", 41));
    // Only Lines with Text leaves the empty ones between out of the list;
    // the selected line stays selected when they come back.
    h.get_by_label("16777220");
    h.get_by_label("Only lines with text").click();
    h.run();
    assert!(h.query_by_label("16777220").is_none(), "an empty line is not listed");
    h.get_by_label("16777256").click();
    h.run();
    h.get_by_label("Only lines with text").click();
    h.run();
    h.run();
    assert_eq!(h.state().talk_view.selected, Some(40));
    h.get_by_label("16777256");
    h.get_by_label("16777255");
    h.get_by_label("Undo").click();
    h.run();
    assert_eq!((line(&h).text.as_str(), h.state().talk.as_ref().unwrap().len()), ("Greetings", 1));

    // Saving the module saves the table, and the game data reads it.
    // (What the watches hear of the save is taken now: on a busy machine,
    // heard and read again in the frames of a later step, it made more of
    // them than a run's.)
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert!(!h.state().talk.as_ref().unwrap().is_dirty());
    let game = h.state().game.as_deref().unwrap();
    assert_eq!(game.string(mg_core::StrRef(16_777_216)).as_deref(), Some("Greetings"));
    let f = mg_tlk::Tlk::read(&std::fs::read(user.join("tlk/mg_talkf.tlk")).unwrap()).unwrap();
    assert_eq!(f.text(mg_core::StrRef(0)).as_deref(), Some("Greetings, lady"));

    // String Edit moves a name into the table (the talk table's window
    // closed: it lies over Module Properties).
    let tab = h.state().dock.find_tab(&Tab::TalkTable).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.run();
    // (The line is added at OK: Cancel leaves the table as it was, with
    // no line that nothing names. The review of November 2026, S27.)
    for answer in ["Cancel", "OK"] {
        h.get_by_label("…").click();
        h.run();
        h.get_by_label("Add Text").click();
        h.run();
        h.state_mut().loc_edit.as_mut().unwrap().entries[0].2 = "The Keep".into();
        h.run();
        h.get_by_label("Move to Talk Table").click();
        h.run();
        let edit = h.state().loc_edit.clone().unwrap();
        assert_eq!((edit.strref.as_str(), edit.entries.len()), ("16777217", 0));
        h.get_by_label("Talk table: The Keep (added with OK)");
        let t = h.state().talk.as_ref().unwrap();
        assert_eq!((t.len(), t.is_dirty()), (1, false), "not added yet");
        h.get_all_by_label(answer).last().unwrap().click();
        h.run();
        assert!(h.state().loc_edit.is_none());
        let t = h.state().talk.as_ref().unwrap();
        assert_eq!(t.len(), if answer == "OK" { 2 } else { 1 }, "{answer}");
    }
    let ws = h.state_mut().ws.as_mut().unwrap();
    let info = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap().root.clone();
    let name: LocString = info.read(&ifo::MOD_NAME);
    assert_eq!((name.strref.0, name.strings.len()), (16_777_217, 0));
    let t = h.state().talk.as_ref().unwrap();
    assert_eq!((t.line(1).text.as_str(), t.is_dirty()), ("The Keep", true));

    // Palette › Categories… moves a category's name into it too: the
    // category named by the new line, to be translated there.
    {
        let app = h.state_mut();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.tiles = false;
        app.palette.custom = true;
    }
    h.run();
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…").click();
    h.run();
    // (Its lines are added at OK, each where a category still names it:
    // Cancel leaves the table as it was. Two in one window take a line
    // each. The review of November 2026, the last fixers' item 9.)
    let name_of = |h: &Harness<'_, Moonglow>, path: &[usize]| {
        let w = h.state().palette_categories.as_ref().unwrap();
        let mut node = &w.skeleton.root.list("MAIN").unwrap()[path[0]];
        for i in &path[1..] {
            node = &node.list("LIST").unwrap()[*i];
        }
        assert!(node.get("NAME").is_none() && node.get("DELETE_ME").is_none(), "no name");
        mg_module::palette::node_name(node)
    };
    let moved = |h: &mut Harness<'_, Moonglow>, name: &str, strref: u32| {
        h.state_mut().palette_categories.as_mut().unwrap().name = name.into();
        h.state_mut().palette_categories.as_mut().unwrap().selected = None;
        h.run();
        h.get_by_label("Add Category").click();
        h.run();
        h.get_by_label("Move Name to Talk Table").click();
        h.run();
        let w = h.state().palette_categories.as_ref().unwrap();
        assert_eq!(w.strref, strref.to_string());
        let path = w.selected.clone().unwrap();
        let named = name_of(h, &path);
        assert_eq!(named, mg_module::palette::PaletteName::StrRef(strref));
        // (Shown by its text before it is added.)
        h.get_by_label_contains(&format!("{name}  (category"));
        path
    };
    moved(&mut h, "Ruins", 16_777_218);
    moved(&mut h, "Tombs", 16_777_219);
    let t = h.state().talk.as_ref().unwrap();
    assert_eq!(t.len(), 2, "not added before OK");
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert!(h.state().palette_categories.is_none());
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 2, "Cancel adds nothing");
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…").click();
    h.run();
    moved(&mut h, "Ruins", 16_777_218);
    moved(&mut h, "Caves", 16_777_219);
    // A third moved and then named by another line: not added.
    moved(&mut h, "Halls", 16_777_220);
    h.state_mut().palette_categories.as_mut().unwrap().strref = "5".into();
    h.run();
    h.get_by_label("Name by StrRef").click();
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert!(h.state().palette_categories.is_none());
    let t = h.state().talk.as_ref().unwrap();
    let lines: Vec<String> = (0..t.len()).map(|r| t.line(r).text).collect();
    assert_eq!(lines, ["Greetings", "The Keep", "Ruins", "Caves"]);
    settle(&mut h);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let key = mg_module::palette::BlueprintKind::Placeable.skeleton_key();
    let skeleton = mg_gff::Gff::read(ws.module.get(&key).unwrap()).unwrap();
    let mut named = Vec::new();
    fn walk(list: &[mg_gff::Struct], out: &mut Vec<mg_module::palette::PaletteName>) {
        for n in list {
            out.push(mg_module::palette::node_name(n));
            walk(n.list("LIST").unwrap_or(&[]), out);
        }
    }
    walk(skeleton.root.list("MAIN").unwrap(), &mut named);
    use mg_module::palette::PaletteName::StrRef;
    assert!(named.contains(&StrRef(16_777_218)) && named.contains(&StrRef(16_777_219)));
    assert!(named.contains(&StrRef(5)) && !named.contains(&StrRef(16_777_220)), "{named:?}");
}

#[test]
fn two_da_view_shows_where_rows_come_from() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-2da-layers");
    let user = user_folder(&dir, &["override"]);
    // The game's ambientmusic.2da with a row changed and one added, in
    // override.
    let game = game_data(&root);
    let key = ResKey::parse("ambientmusic", ResType::TWODA).unwrap();
    let cp = mg_core::Codepage::default();
    let t = mg_2da::TwoDa::parse(&game.resman.get(&key).unwrap(), cp).unwrap();
    let resource = t.column("Resource").unwrap();
    let mut rows: Vec<Vec<String>> = (0..t.len())
        .map(|r| (0..t.columns().len()).map(|c| t.cell(r, c).unwrap_or("****").into()).collect())
        .collect();
    rows[1][resource] = "mus_mine".into();
    let mut added = rows[1].clone();
    added[resource] = "mus_new".into();
    rows.push(added);
    let quote = |c: &String| if c.contains(' ') { format!("\"{c}\"") } else { c.clone() };
    let mut text = format!("2DA V2.0\n\n   {}\n", t.columns().join(" "));
    for (i, r) in rows.iter().enumerate() {
        let cells: Vec<String> = r.iter().map(quote).collect();
        text += &format!("{i} {}\n", cells.join(" "));
    }
    std::fs::write(user.join("override/ambientmusic.2da"), text).unwrap();

    let path = sample_module(&dir);
    let mut app = game_app(&root, Some(user));
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    h.run();
    h.get_by_label("From");
    // The From column is widened by its edge, as the others are, and kept.
    let edge = h.get_by_label("Width of From").rect().center();
    let resource_at = h.get_by_label("Resource").rect().left();
    drag(&mut h, edge, edge + egui::vec2(60.0, 0.0));
    h.run();
    assert!(h.get_by_label("Resource").rect().left() > resource_at + 40.0, "From widened");
    let kept = &h.state().settings.table_widths;
    assert!(kept.iter().any(|(t, w)| t == "ambientmusic" && w.iter().any(|(c, _)| c == "<from>")));
    let summary = h.query_by_label_contains("override: adds 1, changes 1");
    assert!(summary.is_some(), "the summary names what override does");
    h.get_by_label("mus_mine");
    // The Description column shows the talk table's text.
    let description = t.get(0, "Description").and_then(|s| s.parse::<u32>().ok()).unwrap();
    let text = h.state().game.as_deref().unwrap().string(mg_core::StrRef(description)).unwrap();
    assert!(h.query_by_label(&text).is_some(), "{text}");
    // Only override's rows.
    h.state_mut().browser.rows_from = Some(0);
    h.run();
    assert!(h.query_by_label("mus_new").is_some());
    assert!(h.query_by_label(&text).is_none());
}

/// The 2DA editor's "a row of X.2da" is the same list: a cell naming a
/// row far down spells.2da opens it there, with a Filter field (its rows'
/// names are made only while it is in use, and are there when a click
/// opens it), and a row typed for is chosen.
#[test]
fn a_table_s_row_list_opens_at_the_row_named() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-table-row-list");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    let file = dir.join("my_scrolls.2da");
    std::fs::write(&file, "2DA V2.0\r\n\r\n  Label Spell\r\n0 Scroll_A 300\r\n1 Scroll_B 2\r\n")
        .unwrap();
    let toml = "[columns.Spell]\nkind = \"row\"\ntable = \"spells\"\n";
    std::fs::write(dir.join("my_scrolls.2da.toml"), toml).unwrap();
    let app = game_app(&root, None);
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    // (The description's folder is walked on a thread of its own.)
    settle(&mut h);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(0, 1);
    h.run_steps(3);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("300  ")))
        .map(|n| n.rect().center())
        .expect("the rows' list");
    h.hover_at(at);
    h.run_steps(1);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("🔍 Filter");
    let top = h.get(egui_kittest::kittest::by().predicate(hint)).rect().bottom();
    let row = (h.query_all_by_label_contains("300  "))
        .map(|n| n.rect().center().y)
        .find(|y| *y > top)
        .expect("the named row, in the list");
    assert!(row - top < 260.0, "the named row is {} under the Filter field", row - top);
    type_into_hint(&mut h, "Filter", "302");
    h.get_by_label_contains("302  ").click();
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(0, "Spell"), Some("302"));
}

/// A flags cell's list of ticks by the keys (GitHub issue 7): once its
/// box is clicked, Down opens it, the arrows move along the flags and
/// Space ticks the one there (the list staying open).
#[test]
fn a_flags_list_is_ticked_by_the_keys() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-table-flags-keys");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    let file = dir.join("my_flags.2da");
    std::fs::write(&file, "2DA V2.0\r\n\r\n  Label Mask\r\n0 One 0x1\r\n").unwrap();
    let toml = "[columns.Mask]\nkind = \"flags\"\nvalues = { Red = 1, Green = 2, Blue = 4 }\n";
    std::fs::write(dir.join("my_flags.2da.toml"), toml).unwrap();
    let app = game_app(&root, None);
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    // (The description's folder is walked on a thread of its own.)
    settle(&mut h);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(0, 1);
    h.run_steps(3);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Red"))
        .map(|n| n.rect().center())
        .expect("the flags' list");
    h.hover_at(at);
    h.run_steps(1);
    // Opened and closed by clicks: the box keeps the keys.
    for _ in 0..2 {
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
    }
    assert!(h.query_by_label("Blue  (4)").is_none(), "closed");
    for key in [egui::Key::ArrowDown, egui::Key::ArrowDown, egui::Key::ArrowDown] {
        h.key_press(key);
        h.run_steps(2);
    }
    h.key_press(egui::Key::Space);
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(0, "Mask"), Some("0x5"), "Blue ticked");
    assert!(h.query_by_label("Blue  (4)").is_some(), "still open");
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(0, "Mask"), Some("0x7"), "Green ticked");
}

/// A talk table kept in the module (a `.tlk` among its resources) is
/// edited and saved with it: into the module, not a file of its own.
#[test]
fn a_talk_table_kept_in_the_module_is_edited_and_saved_with_it() {
    use mg_module::talk::{Line, Source};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-talk-in-module");
    let user = user_folder(&dir, &[]);
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_inmod", ResType::TLK).unwrap();
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_CUSTOM_TLK, ExoString::from("mg_inmod"));
    m.set_info(&info).unwrap();
    let mut t = mg_module::talk::Table::create("mg_inmod", &dir, Language::ENGLISH, false);
    let line = |text: &str| Line {
        text: text.into(),
        feminine: None,
        sound: String::new(),
        sound_length: 0.0,
    };
    t.add_line(&line("Old words")).unwrap();
    m.set(key, t.tlk.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = game_app(&root, Some(user.clone()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run();
    h.run();
    let talk = h.state().talk.as_ref().expect("the module's table open");
    assert_eq!((&talk.source, talk.editable()), (&Source::Module, true));
    assert!(h.query_by_label_contains("This table is in a hak").is_none(), "not read-only");
    h.get_by_label("16777216").click();
    h.run();
    h.get_all_by_value("Old words")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput)
        .unwrap()
        .click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::End);
    h.event(egui::Event::Text(", new".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert_eq!(h.state().talk.as_ref().unwrap().line(0).text, "Old words, new");
    // Saving the module saves the table into it; the game data reads it,
    // and nothing is written to the tlk folder.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    assert!(!h.state().talk.as_ref().unwrap().is_dirty());
    assert!(!h.state().has_unsaved_work(), "the module's step saved too");
    let game = h.state().game.as_deref().unwrap();
    assert_eq!(game.string(mg_core::StrRef(16_777_216)).as_deref(), Some("Old words, new"));
    assert!(!user.join("tlk/mg_inmod.tlk").exists());
    let saved = Module::open(&path).unwrap();
    let tlk = mg_tlk::Tlk::read(saved.get(&key).unwrap()).unwrap();
    assert_eq!(tlk.text(mg_core::StrRef(0)).as_deref(), Some("Old words, new"));
    let said = h.state().log.entries.iter().any(|(_, e)| e.contains("put into the module"));
    assert!(said, "{:?}", h.state().log.entries);
}

#[test]
fn premium_campaigns_find_their_talk_tables_in_the_install() {
    // Tyrants of the Moonsea names `tyrants`, which the game ships in
    // `data/tlk/` beside the campaign's haks in `data/hk/`.
    let Some(root) = mg_testkit::nwn_root() else { return };
    let path = root.join("data/nwm/Neverwinter Nights - Tyrants of the Moonsea.nwm");
    if !path.is_file() {
        eprintln!("skipped: Tyrants of the Moonsea is not installed");
        return;
    }
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let loaded = app.log.entries.iter().any(|(_, e)| e == "Custom talk table tyrants loaded");
    assert!(loaded, "{:?}", app.log.entries);
    assert!(app.game.as_deref().unwrap().custom_tlk().is_some());
    // Opened in the editor, the game's own table is read only: its Save
    // never writes into the install.
    let mut h = harness(app, 1000.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    h.run();
    h.run();
    let t = h.state().talk.as_ref().expect("the campaign's table open");
    assert!(!t.editable(), "{}", t.source);
    assert!(t.source.to_string().starts_with("the game's files ("), "{}", t.source);
    assert!(h.query_by_label_contains("This table is in the game's files").is_some());
}

/// The talk table editor's split is kept for each table in Moonglow's
/// settings: the next session shows it as it was left, and another table
/// has its own.
#[test]
fn the_talk_table_s_split_is_kept_for_each_table() {
    use mg_module::talk::{Line, Table};
    let dir = mg_testkit::scratch_dir("ui-talk-split-kept");
    let table = |name: &str| {
        let mut table = Table::create(name, &dir, mg_core::Language::ENGLISH, false);
        for row in 0..60 {
            let text = format!("Text {row}");
            let line = Line { text, feminine: None, sound: String::new(), sound_length: 0.0 };
            table.add_line(&line).unwrap();
        }
        table
    };
    let session = |settings: mg_ui::Settings, name: &str| {
        let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
        app.settings = settings;
        app.talk = Some(table(name));
        app.talk_view.outside = true;
        app.talk_view.selected = Some(5);
        app.actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
        let mut h = harness(app, 1000.0, 800.0);
        h.run_steps(8);
        h
    };
    let strref = |h: &Harness<'_, Moonglow>| h.get_by_label("StrRef 16777221").rect().top();
    let mut h = session(mg_ui::Settings::default(), "split");
    let before = strref(&h);
    let from = egui::pos2(500.0, before - 5.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.hover_at(from);
    h.run_steps(1);
    h.event(button(from, true));
    h.run_steps(1);
    for k in 1..=6 {
        h.hover_at(from + egui::vec2(0.0, -100.0 * k as f32 / 6.0));
        h.run_steps(1);
    }
    h.event(button(from + egui::vec2(0.0, -100.0), false));
    h.run_steps(3);
    let dragged = strref(&h);
    assert!((before - 100.0 - dragged).abs() < 1.0, "{before} {dragged}");
    let settings = h.state().settings.clone();
    drop(h);
    let h = session(settings.clone(), "split");
    assert!((strref(&h) - dragged).abs() < 1.5, "kept: {} {dragged}", strref(&h));
    // (Against the other table's own first look: its name lays out the
    // header otherwise on macOS.)
    let fresh = strref(&session(mg_ui::Settings::default(), "other"));
    let h = session(settings, "other");
    assert!((strref(&h) - fresh).abs() < 1.5, "its own: {} {fresh}", strref(&h));
}

/// A drag selecting cells past the grid's foot scrolls the table on, the
/// selection reaching the rows that come into sight, as a spreadsheet's;
/// past its top, back.
#[test]
fn a_drag_past_the_grid_s_edge_scrolls_it() {
    let dir = mg_testkit::scratch_dir("ui-table-edge-drag");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("longone", ResType::TWODA).unwrap();
    let mut text = String::from("2DA V2.0\r\n\r\n   Label   Cost\r\n");
    for r in 0..300 {
        text.push_str(&format!("{r}  Row{r}  {r}\r\n"));
    }
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(text.into_bytes()) }],
    )));
    let mut h = harness(app, 1000.0, 700.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    h.run_steps(4);
    let from = h.get_by_label("Row0").rect().center();
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    // Held under the window's foot for a while.
    let below = egui::pos2(from.x, 699.0);
    h.hover_at(below);
    h.run_steps(30);
    let doc = || h.state().tables.iter().find(|d| d.id == id).unwrap();
    let reached = doc().area().map(|a| a.1).unwrap();
    assert!(doc().scrolled().y > 200.0, "scrolled on: {}", doc().scrolled().y);
    assert!(reached > 40, "the selection reaches the rows come into sight: {reached}");
    assert_eq!(doc().area().map(|a| a.0), Some(0), "from where it began");
    // Back up past the top: scrolled back, the selection to the top.
    let above = egui::pos2(from.x, 1.0);
    h.hover_at(above);
    h.run_steps(60);
    let doc = || h.state().tables.iter().find(|d| d.id == id).unwrap();
    assert!(doc().scrolled().y < 1.0, "back to the top: {}", doc().scrolled().y);
    press(&h, above, false, egui::Modifiers::NONE);
    h.run_steps(2);
    let doc = || h.state().tables.iter().find(|d| d.id == id).unwrap();
    assert_eq!(doc().area().map(|a| (a.0, a.1)), Some((0, 0)));
    // Let go: no more scrolling.
    let at = doc().scrolled().y;
    h.hover_at(below);
    h.run_steps(10);
    assert_eq!(h.state().tables.iter().find(|d| d.id == id).unwrap().scrolled().y, at);
}

/// The 2DA editor's cells are selected several at a time (Shift and the
/// arrows, Shift and a click, a drag), copied as a spreadsheet's and
/// emptied in one step; a paste begins at the selection's first cell. A
/// column's name, right-clicked, renames or removes it. Tools › 2DA File…
/// lists the module's tables.
#[test]
fn a_table_s_cells_are_selected_and_its_columns_changed() {
    let dir = mg_testkit::scratch_dir("ui-table-select");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(SMALL_TABLE.to_vec()) }],
    )));
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    // The module's tables are listed to open, by name.
    mg_ui::table_view::open_table_window(h.state_mut());
    h.run_steps(3);
    assert_eq!(h.state().open_table.as_ref().map(|w| w.tables.len()), Some(1));
    h.get_by_label("mystore.2da").click();
    h.run_steps(4);
    assert!(h.state().open_table.is_none());
    let id = h.state().tables[0].id;
    let area = |h: &Harness<'_, Moonglow>| h.state().tables[0].area();
    // Shift and the arrows reach from Bread to Cheese's cost.
    h.get_by_label("Bread").click();
    h.run_steps(2);
    assert_eq!(area(&h), Some((0, 0, 0, 0)));
    for key in [egui::Key::ArrowDown, egui::Key::ArrowRight] {
        h.key_press_modifiers(egui::Modifiers::SHIFT, key);
        h.run_steps(2);
    }
    assert_eq!(area(&h), Some((0, 1, 0, 1)));
    // (Away without Shift and back: one cell, not the rectangle again.)
    for key in [egui::Key::ArrowUp, egui::Key::ArrowDown] {
        h.key_press(key);
        h.run_steps(2);
    }
    assert_eq!(area(&h), Some((1, 1, 1, 1)));
    h.get_by_label("Bread").click();
    h.run_steps(2);
    for key in [egui::Key::ArrowDown, egui::Key::ArrowRight] {
        h.key_press_modifiers(egui::Modifiers::SHIFT, key);
        h.run_steps(2);
    }
    assert_eq!(area(&h), Some((0, 1, 0, 1)));
    // Copied: a line a row, tabs between the cells.
    h.event(egui::Event::Copy);
    h.step();
    let copied = h.output().platform_output.commands.iter().find_map(|c| match c {
        egui::OutputCommand::CopyText(t) => Some(t.clone()),
        _ => None,
    });
    assert_eq!(copied.as_deref(), Some("Bread\t1\nCheese\t2"));
    // Delete empties them all, in one step.
    h.key_press(egui::Key::Delete);
    h.run_steps(2);
    let t = table_of(&h, id);
    assert_eq!((t.get(0, "Label"), t.get(1, "Cost")), (None, None));
    h.get_by_label("Undo").click();
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(1, "Cost"), Some("2"));
    // A paste begins at the selection's first cell, and adds the rows it
    // runs past the end by; cells past the last column are left out.
    h.event(egui::Event::Paste("Ale\t5\tx\nEggs\t3\nFish\t8".into()));
    h.run_steps(2);
    let t = table_of(&h, id);
    assert_eq!((t.len(), t.get(0, "Label"), t.get(2, "Cost")), (3, Some("Ale"), Some("8")));
    // Ctrl+Z takes the paste back and Ctrl+Y makes it again: the table's
    // own steps, with the keys in its grid, not the module's.
    let module_step = h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    let t = table_of(&h, id);
    assert_eq!((t.len(), t.get(0, "Label")), (2, Some("Bread")), "the paste is undone");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo().map(str::to_string), module_step);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(0, "Label"), Some("Ale"));
    // A plain arrow is one cell again; Shift and a click reaches to it.
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    let cell = h.get_by_label("Fish").rect().center();
    h.hover_at(cell);
    press(&h, cell, true, egui::Modifiers::SHIFT);
    press(&h, cell, false, egui::Modifiers::SHIFT);
    h.run_steps(2);
    assert_eq!(area(&h).map(|a| a.1), Some(2), "to Fish's row");
    // A drag selects from where it began to where it is.
    let (from, to) = (h.get_by_label("Ale").rect().center(), h.get_by_label("3").rect().center());
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    h.hover_at(egui::pos2((from.x + to.x) / 2.0, to.y));
    h.run_steps(1);
    h.hover_at(to);
    h.run_steps(2);
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(area(&h), Some((0, 1, 0, 1)));

    // A column's name, right-clicked: renamed, then removed.
    let secondary = |h: &mut Harness<'_, Moonglow>, label: &str| {
        let pos = h.get_by_label(label).rect().center();
        h.hover_at(pos);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    secondary(&mut h, "Cost");
    h.get_by_label("Rename Column…").click();
    h.run_steps(3);
    h.get_by_label("Rename Column");
    h.event(egui::Event::Text("s".into()));
    h.run_steps(2);
    h.get_by_label("Rename").click();
    h.run_steps(3);
    assert_eq!(table_of(&h, id).columns(), ["Label", "Costs"]);
    secondary(&mut h, "Label");
    h.get_by_label("Remove Column").click();
    h.run_steps(3);
    let t = table_of(&h, id);
    assert_eq!((t.columns().to_vec(), t.cell(0, 0)), (vec!["Costs".to_string()], Some("5")));
    h.get_by_label("Undo").click();
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(0, "Label"), Some("Ale"), "back where it was");
    // A column is moved by its menu, and by a drag of its name onto
    // another's: its cells go with it, and the selected cell with its.
    h.get_by_label("Ale").click();
    h.run_steps(2);
    secondary(&mut h, "Label");
    h.get_by_label("Move Right").click();
    h.run_steps(3);
    let t = table_of(&h, id);
    assert_eq!(
        (t.columns().to_vec(), t.cell(0, 1)),
        (vec!["Costs".to_string(), "Label".into()], Some("Ale"))
    );
    assert_eq!(h.state().tables[0].selected, Some((0, 1)));
    let (from, to) =
        (h.get_by_label("Label").rect().center(), h.get_by_label("Costs").rect().center());
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    h.hover_at(egui::pos2((from.x + to.x) / 2.0, to.y));
    h.run_steps(1);
    h.hover_at(to);
    h.run_steps(2);
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(table_of(&h, id).columns(), ["Label", "Costs"]);
    h.get_by_label("Undo").click();
    h.run_steps(2);
    assert_eq!(table_of(&h, id).columns(), ["Costs", "Label"], "a step to take back");
}

/// Undo and Redo in a 2DA's grid go through the keymap: a key given to
/// Undo in Options › Keyboard reaches the grid that has the keys (the
/// pointer elsewhere), and takes the table's step back, not the module's.
/// Under a Find, what is cut from the rows shown goes back into them:
/// the lines of a paste were counted on from the cursor's row number,
/// into the rows between that the Find hides, where nothing showed it
/// (the review of November 2026, D8). A paste with more lines than the
/// Find shows rows for is refused, and says so.
#[test]
fn a_paste_under_a_find_goes_into_the_rows_shown() {
    let dir = mg_testkit::scratch_dir("ui-table-paste-find");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mypeople", ResType::TWODA).unwrap();
    let table =
        b"2DA V2.0\r\n\r\n   Label  Race\r\n0  Ann    elf\r\n1  Bob    orc\r\n2  Cid    elf\r\n";
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(table.to_vec()) }],
    )));
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    settle(&mut h);
    h.state_mut().tables[0].filter = "elf".into();
    settle(&mut h);
    assert!(h.query_by_label("Bob").is_none(), "hidden by the Find");
    // Ann's and Cid's names selected (the rows shown), and cut.
    h.get_by_label("Ann").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(h.state().tables[0].area(), Some((0, 2, 0, 0)));
    h.event(egui::Event::Cut);
    settle(&mut h);
    let labels = |h: &Harness<'_, Moonglow>| -> Vec<Option<String>> {
        let t = table_of(h, id);
        (0..t.len()).map(|r| t.get(r, "Label").map(str::to_string)).collect()
    };
    assert_eq!(labels(&h), [None, Some("Bob".into()), None]);
    // Pasted on the same selection: back where they were, Bob as he was.
    h.event(egui::Event::Paste("Ann\nCid".into()));
    settle(&mut h);
    assert_eq!(labels(&h), [Some("Ann".into()), Some("Bob".into()), Some("Cid".into())]);
    // Three lines, and the Find shows two rows: nothing pasted, and said.
    h.event(egui::Event::Paste("x\ny\nz".into()));
    settle(&mut h);
    assert_eq!(labels(&h), [Some("Ann".into()), Some("Bob".into()), Some("Cid".into())]);
    let problem = h.state().tables[0].problem.clone().unwrap_or_default();
    assert!(problem.contains("Not pasted: 3 lines"), "{problem}");
}

#[test]
fn a_rebound_undo_key_reaches_a_table_s_grid() {
    let dir = mg_testkit::scratch_dir("ui-table-undo-key");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.key_bindings.insert("undo".into(), vec!["Ctrl+Alt+U".into()]);
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(SMALL_TABLE.to_vec()) }],
    )));
    let mut h = harness(app, 1800.0, 1000.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    h.run_steps(4);
    let module_step =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    // A cell emptied in the grid, which has the keys.
    h.get_by_label("Bread").click();
    h.run_steps(2);
    h.key_press(egui::Key::Delete);
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(0, "Label"), None);
    // The pointer away over the module tree: the keys are the grid's still.
    h.hover_at(h.get_by_label("Areas (1)").rect().center());
    h.run_steps(2);
    // Ctrl+Z is not Undo's key now: nothing.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(0, "Label"), None);
    // Its new key is: the table's step, not the module's.
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::ALT, egui::Key::U);
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(0, "Label"), Some("Bread"), "the table's undo");
    assert_eq!(module_step(&h).as_deref(), Some("A table"), "not the module's");
    // Redo's other key, Ctrl+Shift+Z, reaches it too.
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(0, "Label"), None, "redone");
    // Edit › Undo is the table's, names its step, and takes it.
    h.get_by_label("Edit").click();
    h.run_steps(2);
    let undo = h.get_by_label_contains("Undo ").accesskit_node().label().unwrap_or_default();
    assert!(undo.starts_with("Undo Empty 1 cell"), "{undo}");
    h.get_by_label_contains("Undo Empty 1 cell").click();
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(0, "Label"), Some("Bread"), "the table's undo, from the menu");
    assert_eq!(module_step(&h).as_deref(), Some("A table"), "not the module's");
}

/// The 2DA editor on a table of the module's: a cell typed in, rows
/// added and emptied, each a step of the table's own to take back; Save
/// is one step of the module's.
#[test]
fn a_module_s_table_is_edited_cell_by_cell() {
    let dir = mg_testkit::scratch_dir("ui-table-module");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(SMALL_TABLE.to_vec()) }],
    )));
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    // (A description beside the module's file is its tables'.)
    std::fs::write(
        path.with_file_name("mystore.2da.toml"),
        "[columns.Cost]\nkind = \"int\"\nmax = 500\n",
    )
    .unwrap();
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    h.run_steps(4);
    // (The description's folder is walked on a thread of its own.)
    settle(&mut h);
    assert!(h.state().tables[0].description().is_some(), "read from beside the .mod");
    assert_eq!(mg_ui::table_view::open_in_module(h.state_mut(), key), Some(id), "open once");
    h.get_by_label("mystore.2da");
    // A click selects a cell and F2 (or Enter, or a double click) types
    // into it; Enter keeps what is typed.
    h.get_by_label("Cheese").click();
    h.run_steps(2);
    h.key_press(egui::Key::F2);
    h.run_steps(3);
    h.event(egui::Event::Text("y".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(1, "Label"), Some("Cheesey"));
    h.get_by_label("(unsaved)");
    // The keys are the grid's: Up and Right to Bread's cost, Delete
    // empties it; Escape while typing leaves a cell as it was.
    let key_ = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(2);
    };
    let at = |h: &Harness<'_, Moonglow>| h.state().tables[0].selected;
    key_(&mut h, egui::Key::ArrowUp);
    key_(&mut h, egui::Key::ArrowRight);
    assert_eq!(at(&h), Some((0, 1)));
    key_(&mut h, egui::Key::Delete);
    assert_eq!(table_of(&h, id).get(0, "Cost"), None);
    key_(&mut h, egui::Key::F2);
    h.event(egui::Event::Text("77".into()));
    h.run_steps(2);
    key_(&mut h, egui::Key::Escape);
    assert_eq!(table_of(&h, id).get(0, "Cost"), None, "Escape: not kept");
    // A digit or letter typed begins typing, in place of what the cell
    // held, as in a spreadsheet: Cheese's cost 2 typed over as 42.
    key_(&mut h, egui::Key::ArrowDown);
    h.event(egui::Event::Text("4".into()));
    h.run_steps(2);
    h.event(egui::Event::Text("2".into()));
    h.run_steps(2);
    key_(&mut h, egui::Key::Enter);
    assert_eq!(table_of(&h, id).get(1, "Cost"), Some("42"), "typed over");
    h.get_by_label("Undo").click();
    h.run_steps(2);
    key_(&mut h, egui::Key::ArrowUp);
    assert_eq!(at(&h), Some((0, 1)));
    // Add Row puts one at the end; Empty Row empties the selected one,
    // which stays.
    h.get_by_label("Add Row").click();
    h.run_steps(2);
    assert_eq!((table_of(&h, id).len(), at(&h)), (3, Some((2, 1))));
    // Delete Row takes the selected row away (here the last), and Add
    // Row adds as many as its number says, in one step.
    h.get_by_label("Delete Row").click();
    h.run_steps(2);
    assert_eq!((table_of(&h, id).len(), at(&h)), (2, Some((1, 1))));
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().add_count = 5;
    h.run_steps(2);
    h.get_by_label("Add 5 Rows").click();
    h.run_steps(2);
    assert_eq!((table_of(&h, id).len(), at(&h)), (7, Some((2, 1))));
    h.get_by_label("Undo").click();
    h.run_steps(2);
    assert_eq!(table_of(&h, id).len(), 2);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().add_count = 1;
    h.run_steps(2);
    h.get_by_label("Add Row").click();
    h.run_steps(2);
    // A row between others: those after it move up, and Undo puts it back.
    h.get_by_label("Cheesey").click();
    h.run_steps(2);
    h.get_by_label("Delete Row").click();
    h.run_steps(2);
    let t = table_of(&h, id);
    assert_eq!((t.len(), t.get(1, "Label")), (2, None), "the added row is row 1 now");
    h.get_by_label("Undo").click();
    h.run_steps(2);
    assert_eq!(table_of(&h, id).len(), 3);
    // (The row put back is the one selected.)
    h.get_by_label("Empty Row").click();
    h.run_steps(2);
    let t = table_of(&h, id);
    assert_eq!((t.len(), t.get(1, "Label"), t.get(1, "Cost")), (3, None, None));
    // Undo is the table's own, step by step.
    h.get_by_label("Undo").click();
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(1, "Label"), Some("Cheesey"));
    // Save: the module's table is as edited, in one step of the module's.
    let stored = |h: &mut Harness<'_, Moonglow>| {
        let data = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
        mg_2da::TwoDa::parse(&data, mg_core::Codepage::default()).unwrap()
    };
    assert_eq!(stored(&mut h).get(1, "Label"), Some("Cheese"));
    h.get_by_label("Save").click();
    h.run_steps(3);
    // (The empty row at the end is in the file, a row as the game counts
    // it: `engine_counts_an_empty_last_row`.)
    assert_eq!(stored(&mut h).rows, table_of(&h, id).rows);
    assert!(h.query_by_label("(unsaved)").is_none(), "saved: not marked");
    // The module's Undo takes the save back with the table open: the
    // editor reads it again.
    // (The module's own Undo: the one under the menu is the grid's while
    // it is open.)
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while table_of(&h, id).get(1, "Label") != Some("Cheese") {
        assert!(std::time::Instant::now() < deadline, "read again after the Undo");
        h.run_steps(5);
    }
    h.state_mut().ws.as_mut().unwrap().redo().unwrap();
    while table_of(&h, id).get(1, "Label") != Some("Cheesey") {
        assert!(std::time::Instant::now() < deadline, "read again after the Redo");
        h.run_steps(5);
    }
    // (Closed, the table's grid has the keys no more: Undo is the
    // module's.)
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Table(id)]));
    h.run_steps(2);
    assert!(h.state().tables.is_empty(), "saved: closed without a question");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(stored(&mut h).get(1, "Label"), Some("Cheese"), "the module's Undo takes the save");
}

/// The hak editor's view of a table shows the table as the hak has it
/// now: viewed, changed in the 2DA editor (a cell, no row or column),
/// saved into the hak and viewed again, it shows the new cell. The view
/// was kept by where its table lay in memory, and a table read again
/// could lie where the old one did (the review of November 2026, D28):
/// it is kept by which reading it was made for.
#[test]
fn a_hak_s_table_viewed_again_shows_it_as_it_is_now() {
    use mg_module::hak_edit::{Hak, Source};
    let dir = mg_testkit::scratch_dir("ui-table-hak-view");
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    let hak_path = dir.join("stuff.hak");
    let mut hak = Hak::new();
    hak.add_sources(vec![(key, Source::Bytes(SMALL_TABLE.to_vec().into()))]).unwrap();
    hak.save(Some(&hak_path)).unwrap();
    let dialogs = NoDialogs { open: vec![hak_path.clone()], ..Default::default() };
    let mut h = harness(Moonglow::new(None, Box::new(dialogs)), 1200.0, 800.0);
    h.run();
    mg_ui::hak_view::open_hak(h.state_mut());
    settle(&mut h);
    let hak_id = h.state().haks[0].id;
    let view = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Hak(hak_id)));
        settle(h);
        h.get_all_by_label("mystore.2da").next().unwrap().click_secondary();
        settle(h);
        h.get_all_by_label("View").last().unwrap().click();
        settle(h);
        assert_eq!(h.state().haks[0].viewing().map(|v| v.1), Some("table"));
    };
    // (Several times over: a table read again often lies where the last
    // one did.)
    let mut was = "Bread".to_string();
    for now in ["Toast", "Rolls", "Buns", "Loaf"] {
        view(&mut h);
        h.get_by_label(was.as_str());
        let id = mg_ui::table_view::open_in_hak(h.state_mut(), hak_id, key).unwrap();
        settle(&mut h);
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.edit.set(0, 0, Some(now.into())).unwrap();
        assert!(mg_ui::table_view::save(h.state_mut(), id));
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Table(id)]));
        settle(&mut h);
        view(&mut h);
        h.get_by_label(now);
        assert!(h.query_by_label(was.as_str()).is_none(), "{was} was the cell before");
        was = now.to_string();
    }
}

/// A table of a hak open in the hak editor is saved into the hak (a step
/// of the hak's, written by its own Save); one of a file, to the file.
#[test]
fn a_hak_s_table_and_a_file_s_are_saved_where_they_live() {
    use mg_module::hak_edit::{Hak, Source};
    let dir = mg_testkit::scratch_dir("ui-table-hak");
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    let hak_path = dir.join("stuff.hak");
    let mut hak = Hak::new();
    hak.add_sources(vec![(key, Source::Bytes(SMALL_TABLE.to_vec().into()))]).unwrap();
    hak.save(Some(&hak_path)).unwrap();
    let file = dir.join("loose.2da");
    std::fs::write(&file, SMALL_TABLE).unwrap();
    let dialogs = NoDialogs { open: vec![file.clone(), hak_path.clone()], ..Default::default() };
    let mut h = harness(Moonglow::new(None, Box::new(dialogs)), 1200.0, 800.0);
    h.run_steps(2);
    mg_ui::hak_view::open_hak(h.state_mut());
    h.run_steps(3);
    let hak_id = h.state().haks[0].id;
    // (A description beside the hak's file is its tables'.)
    std::fs::write(dir.join("mystore.2da.toml"), "[columns.Cost]\nkind = \"int\"\nmax = 5\n")
        .unwrap();
    let id = mg_ui::table_view::open_in_hak(h.state_mut(), hak_id, key).unwrap();
    h.run_steps(3);
    // (The description's folder is walked on a thread of its own.)
    settle(&mut h);
    assert!(h.state().tables[0].description().is_some(), "read from beside the hak");
    {
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        assert_eq!(doc.home, mg_ui::table_view::Home::Hak(hak_id));
        doc.edit.set(0, 1, Some("9".into())).unwrap();
    }
    assert!(mg_ui::table_view::save(h.state_mut(), id));
    let in_hak = |h: &Harness<'_, Moonglow>| {
        let data = h.state().haks[0].hak.data(key).unwrap();
        mg_2da::TwoDa::parse(&data, mg_core::Codepage::default()).unwrap()
    };
    assert_eq!(in_hak(&h).get(0, "Cost"), Some("9"));
    assert!(h.state().haks[0].hak.is_dirty(), "the hak's own Save writes its file");
    assert!(!h.state().tables[0].edit.is_dirty());
    h.state_mut().haks[0].hak.undo();
    assert_eq!(in_hak(&h).get(0, "Cost"), Some("1"), "a step of the hak's");

    // Tools › 2DA File…: a file anywhere.
    mg_ui::table_view::open_file(h.state_mut());
    h.run_steps(3);
    let doc = h.state_mut().tables.iter_mut().find(|d| d.name() == "loose").unwrap();
    let loose = doc.id;
    doc.edit.add_rows(1);
    doc.edit.set(2, 0, Some("Two Words".into())).unwrap();
    assert!(mg_ui::table_view::save(h.state_mut(), loose));
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.ends_with("2  \"Two Words\" ****\r\n"), "{text:?}");
    // Compact: saved with one space between cells (ticking it is a change
    // to save), and a file read so is saved so again.
    assert!(!h.state().tables.iter().find(|d| d.id == loose).unwrap().compact);
    // (The hak's table is open beside it: the file's is the one opened last.)
    h.get_all_by_label("Compact").last().unwrap().click();
    h.run_steps(3);
    let doc = h.state().tables.iter().find(|d| d.id == loose).unwrap();
    assert!(doc.compact && doc.edit.is_dirty());
    assert!(mg_ui::table_view::save(h.state_mut(), loose));
    let text = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        text,
        "2DA V2.0\r\n\r\n Label Cost\r\n0 Bread 1\r\n1 Cheese 2\r\n2 \"Two Words\" ****\r\n"
    );
    let other = dir.join("tabs.2da");
    std::fs::write(&other, "2DA V2.0\n\n\tLabel\tCost\n0\tBread\t1\n").unwrap();
    let tabs = mg_ui::table_view::open_path(h.state_mut(), &other).unwrap();
    assert!(h.state().tables.iter().find(|d| d.id == tabs).unwrap().compact, "as the file was");
    h.state_mut().tables.retain(|d| d.id != tabs);
    // A cell a 2DA cannot hold is refused, and said.
    let doc = h.state_mut().tables.iter_mut().find(|d| d.id == loose).unwrap();
    assert!(doc.edit.set(0, 0, Some("a \"b\"".into())).is_err());
    // A cell being typed is unsaved too: closing the tab asked nothing,
    // and what was typed was lost.
    (doc.selected, doc.typing) = (Some((0, 1)), Some("Bun".into()));
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Table(loose)]));
    h.run_steps(3);
    h.get_by_label("Save changes to loose.2da?");
    h.get_all_by_label("Save").last().unwrap().click();
    h.run_steps(3);
    assert!(std::fs::read_to_string(&file).unwrap().contains("Bun"), "saved what was typed");
    let loose = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(3);
    let doc = h.state_mut().tables.iter_mut().find(|d| d.id == loose).unwrap();
    // Closing a table with unsaved changes asks first.
    doc.edit.set(0, 0, Some("Loaf".into())).unwrap();
    h.run_steps(2);
    let tab = h.state().dock.find_tab(&Tab::Table(loose)).unwrap();
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Table(loose)]));
    h.run_steps(3);
    let _ = tab;
    h.get_by_label("Save changes to loose.2da?");
    h.get_by_label("Don't Save").click();
    h.run_steps(3);
    assert!(h.state().tables.iter().all(|d| d.id != loose));
    assert!(std::fs::read_to_string(&file).unwrap().contains("Bread"), "discarded");
}

/// A cell as the game's own copy has it, wrong there too, is the game's:
/// a hak's appearance.2da keeps seven rows of footstep 30 (of 18), which
/// are not the builder's to fix; one the builder types is said.
#[test]
fn the_game_s_own_wrong_cells_are_not_said_in_a_copy() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-table-game-s-own");
    let game = game_data(&root);
    let key = ResKey::parse("appearance", ResType::TWODA).unwrap();
    let file = dir.join("appearance.2da");
    std::fs::write(&file, game.resman.get(&key).unwrap()).unwrap();
    let footsteps = game.table("appearance").unwrap().column("FOOTSTEPTYPE").unwrap();
    assert_eq!(game.table("appearance").unwrap().cell(562, footsteps), Some("30"));
    let app = game_app(&root, None);
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    assert!(h.query_by_label_contains("wrong").is_none(), "the game's own seven");
    {
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.edit.set(0, footsteps, Some("30".into())).unwrap();
    }
    h.get_by_label("Add Row").click();
    h.run_steps(3);
    h.get_by_label("1 cell is wrong").click();
    h.run_steps(4);
    assert_eq!(h.state().tables[0].selected, Some((0, footsteps)));
}

/// With the game's data: a cell is checked by what its column holds as
/// it is typed, a row of another table is chosen by name, where a row is
/// named is found, and rows are taken from the game's copy.
#[test]
fn a_table_s_cells_are_checked_and_rows_taken_from_another_copy() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-table-kinds");
    let game = game_data(&root);
    let mut t = (*game.table("feat").unwrap()).clone();
    let whole = t.len();
    let (prereq, label) = (t.column("PREREQFEAT1").unwrap(), t.column("LABEL").unwrap());
    t.rows.truncate(whole - 25);
    t.set(5, label, Some("MyOwnShot".into())).unwrap();
    let file = dir.join("feat.2da");
    std::fs::write(&file, t.to_bytes(mg_core::Codepage::default()).unwrap()).unwrap();
    let app = game_app(&root, None);
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    assert!(h.query_by_label_contains("wrong").is_none(), "the game's own table is sound");
    // A feat that is not there: said, and gone to.
    {
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.edit.set(3, prereq, Some("4000".into())).unwrap();
        doc.go_to(0, 0);
    }
    // (The table changed under the window: as an edit there would say.)
    h.get_by_label("Add Row").click();
    h.run_steps(3);
    h.get_by_label("1 cell is wrong").click();
    h.run_steps(4);
    assert_eq!(h.state().tables[0].selected, Some((3, prereq)));
    h.get_by_label_contains("row 4000 of feat.2da doesn't exist");
    // Where row 3 (Armor Proficiency, Heavy) is named: other feats
    // require it.
    let own = table_of(&h, id);
    let found = mg_ui::table_view::references(h.state(), "feat", &own, 3);
    assert!(found.iter().any(|r| r.table == "feat" && r.column == "PREREQFEAT1"), "{found:?}");
    assert!(found.len() > 3, "{found:?}");
    // Take Rows: the game's copy has the 25 rows this one lacks, and two
    // it has differently (this is the lowest copy: whose change, untold).
    h.get_by_label("Take Rows…").click();
    h.run_steps(4);
    h.get_by_label_contains("25 new, 2 that differ");
    // (The Differ box ticks the two that differ; Clear Selections unticks;
    // New, the 25; both boxes, all 27, and a box unticked takes its own away.)
    h.get_by_label("Differ").click();
    h.run_steps(2);
    h.get_by_label("Take 2 Rows");
    h.get_by_label("Clear Selections").click();
    h.run_steps(2);
    h.get_by_label("New").click();
    h.run_steps(2);
    h.get_by_label("Differ").click();
    h.run_steps(2);
    h.get_by_label("Take 27 Rows");
    h.get_by_label("Differ").click();
    h.run_steps(2);
    h.get_by_label("Take 25 Rows").click();
    h.run_steps(3);
    let now = table_of(&h, id);
    assert_eq!(now.len(), whole);
    assert_eq!(now.get(whole - 1, "LABEL"), game.table("feat").unwrap().get(whole - 1, "LABEL"));
    assert_eq!(now.get(5, "LABEL"), Some("MyOwnShot"), "what was not chosen is as it was");
    assert!(h.state().tables[0].taking.is_none(), "the window closed");
    // One step to take back (and the row added above, another).
    h.get_by_label("Undo").click();
    h.run_steps(2);
    assert_eq!(table_of(&h, id).len(), whole - 24);
}

/// The game's own tables are not changed: Edit… in the resource browser
/// makes a copy to edit, in a file, a hak or the module, which is not
/// there until it is saved.
#[test]
fn a_game_table_is_edited_as_a_copy() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-table-copy");
    let path = sample_module(&dir);
    let file = dir.join("skills.2da");
    let dialogs = NoDialogs { save: vec![file.clone()], ..Default::default() };
    let mut app = game_app_with(&root, None, dialogs);
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(3);
    let key = ResKey::parse("skills", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    h.run_steps(4);
    h.get_by_label("Edit…").click();
    h.run_steps(2);
    h.get_by_label("A Copy in the Module");
    h.get_by_label("A Copy in a File…").click();
    h.run_steps(4);
    let doc = &h.state().tables[0];
    assert_eq!(doc.home, mg_ui::table_view::Home::File(file.clone()));
    assert!(doc.edit.is_dirty(), "a copy: not there until saved");
    assert!(!file.exists());
    let rows = doc.edit.table().len();
    assert!(rows > 20);
    let id = doc.id;
    assert!(mg_ui::table_view::save(h.state_mut(), id));
    let written =
        mg_2da::TwoDa::parse(&std::fs::read(&file).unwrap(), mg_core::Codepage::default());
    assert_eq!(written.unwrap().len(), rows);
    // In the module: one step of the module's at Save, the module's own
    // table from then on.
    let id =
        mg_ui::table_view::edit_copy(h.state_mut(), key, mg_ui::table_view::Home::Module).unwrap();
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&key));
    assert!(mg_ui::table_view::save(h.state_mut(), id));
    h.run_steps(3);
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key));
    // The module closed: its table's editor goes with it (kept, the next
    // module's Save wrote the table into that one); the file's stays.
    let module = |h: &Harness<'_, Moonglow>| {
        h.state().tables.iter().any(|d| d.home == mg_ui::table_view::Home::Module)
    };
    assert!(module(&h));
    h.state_mut().close();
    assert!(!module(&h));
    assert!(h.state().tables.iter().any(|d| d.home == mg_ui::table_view::Home::File(file.clone())));
}

fn double_click(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    h.run_steps(2);
    for pressed in [true, false, true, false] {
        press(h, at, pressed, egui::Modifiers::NONE);
        h.run_steps(1);
    }
    h.run_steps(2);
}

/// A builder's report: the 2DA editor's columns could not be made wider.
/// The edge at the right of a column's name is dragged to set its width
/// (kept for the table's name, between runs too), and a double click
/// there makes it as wide as its widest cell; a drag on the rest of the
/// name still moves the column.
#[test]
fn a_table_s_column_is_widened_by_the_edge_of_its_name() {
    let dir = mg_testkit::scratch_dir("ui-table-widths");
    let file = dir.join("loose.2da");
    std::fs::write(&file, SMALL_TABLE).unwrap();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(Vec::new()));
    h.run_steps(2);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    let left_of = |h: &Harness<'_, Moonglow>, name: &str| h.get_by_label(name).rect().left();
    let (label, cost) = (left_of(&h, "Label"), left_of(&h, "Cost"));
    let widths = |h: &Harness<'_, Moonglow>| h.state().settings.table_widths.clone();
    assert!(widths(&h).is_empty());
    // Label's edge dragged 100 points on: Cost is that much further right.
    let edge = h.get_by_label("Width of Label").rect().center();
    drag(&mut h, edge, edge + egui::vec2(100.0, 0.0));
    let wide = widths(&h);
    assert_eq!(wide.len(), 1);
    // (Kept by the file's folder and its name: another folder's
    // loose.2da keeps its own.)
    assert!(wide[0].0.ends_with("/loose") && wide[0].0.len() > 6, "{}", wide[0].0);
    let (column, width) = &wide[0].1[0];
    assert_eq!(column, "Label");
    // (116 points at first.)
    assert!(width.abs_diff(216) <= 1, "{width}");
    assert!((left_of(&h, "Cost") - cost - 100.0).abs() < 3.0, "the next column moved on");
    assert_eq!(table_of(&h, id).columns(), ["Label", "Cost"], "not moved");
    // A double click on the edge: as wide as "Cheese", much narrower.
    let edge = h.get_by_label("Width of Label").rect().center();
    double_click(&mut h, edge);
    let fitted = widths(&h)[0].1[0].1;
    assert!((40..90).contains(&fitted), "fitted to its widest cell: {fitted}");
    assert!(left_of(&h, "Cost") < cost, "{} < {cost}", left_of(&h, "Cost"));
    // Dragged by its name, Cost moves before Label, and Label's width goes
    // with it.
    let (from, to) =
        (h.get_by_label("Cost").rect().center(), h.get_by_label("Label").rect().center());
    drag(&mut h, from, to);
    assert_eq!(table_of(&h, id).columns(), ["Cost", "Label"], "moved by its name");
    let now = left_of(&h, "Label") - left_of(&h, "Cost");
    assert!((now - (cost - label)).abs() < 3.0, "Cost is as wide as before: {now}");
    // Another table of the name (the same file, opened again) is as wide.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Table(id)]));
    h.run_steps(3);
    h.get_by_label("Don't Save").click();
    h.run_steps(3);
    assert!(h.state().tables.is_empty());
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    let _ = id;
    let label_width = h.get_by_label("Width of Label").rect().center().x - left_of(&h, "Label");
    assert!((label_width - fitted as f32).abs() < 12.0, "{label_width} for {fitted}");
    // Another folder's table of the name has widths of its own.
    let other = mg_testkit::scratch_dir("ui-table-widths-other").join("loose.2da");
    std::fs::write(&other, SMALL_TABLE).unwrap();
    h.state_mut().tables.retain(|d| d.id != id);
    h.run_steps(4);
    let second = mg_ui::table_view::open_path(h.state_mut(), &other).unwrap();
    h.run_steps(4);
    let label_width = h.get_by_label("Width of Label").rect().center().x - left_of(&h, "Label");
    assert!((label_width - 116.0).abs() < 12.0, "its first width: {label_width}");
    h.state_mut().tables.retain(|d| d.id != second);
    h.run_steps(4);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    // Reset Width, on the column's menu: its first width again.
    h.get_by_label("Label").click_secondary();
    h.run_steps(3);
    h.get_by_label("Reset Width").click();
    h.run_steps(3);
    assert!(widths(&h).iter().all(|(_, w)| w.iter().all(|(c, _)| c != "Label")), "reset");
    let label_width = h.get_by_label("Width of Label").rect().center().x - left_of(&h, "Label");
    assert!((label_width - 116.0).abs() < 12.0, "its first width again: {label_width}");
    // Hide Column: out of sight, its cells kept; Show Hidden Columns.
    h.get_by_label("Cost").click_secondary();
    h.run_steps(3);
    h.get_by_label("Hide Column").click();
    h.run_steps(3);
    assert!(h.query_by_label("Cost").is_none(), "hidden");
    assert_eq!(table_of(&h, id).columns(), ["Label", "Cost"], "kept");
    h.get_by_label("Show Hidden Columns (1)").click();
    h.run_steps(3);
    assert!(h.query_by_label("Cost").is_some(), "shown again");
}

/// A builder's report: a game's table read in the resource browser (not
/// editable) had no scroll bar. The rows' scroll bar is drawn in sight,
/// at the right of the rows, and scrolls them; so does the wheel, down
/// and sideways, and the column names and the rows' numbers follow.
#[test]
fn a_game_table_read_only_scrolls() {
    mg_testkit::gpu::hold();
    let root = mg_testkit::corpus!();
    let app = game_app(&root, None);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_step_dt(1.0 / 60.0)
        .wgpu()
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run_steps(3);
    let key = ResKey::parse("spells", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    h.run_steps(5);
    h.get_by_label("Acid_Fog");
    let pane = h
        .state()
        .dock
        .iter_leaves()
        .find(|(_, l)| l.tabs.contains(&Tab::Resource(key)))
        .map(|(_, l)| l.viewport)
        .unwrap();
    // The scroll bar at the right of the rows, in the picture: a column of
    // pixels near the pane's right edge, from the rows' top to their
    // bottom, is the bar's track, with its handle at the top.
    let (top, bottom) = (h.get_by_label("Acid_Fog").rect().top(), pane.bottom() - 30.0);
    let image = h.render().expect("render");
    let visuals = h.ctx.global_style().visuals.clone();
    let (track, handle) = (visuals.extreme_bg_color, visuals.widgets.inactive.bg_fill);
    let is = |x: f32, y: u32, c: egui::Color32| {
        let p = image.get_pixel(x as u32, y);
        (0..3).all(|i| p[i].abs_diff(c.to_array()[i]) <= 3)
    };
    let x = (1..=24).map(|dx| pane.right() - dx as f32).find(|&x| {
        let ys = top as u32..bottom as u32;
        let handle_at_top = (top as u32 + 1..top as u32 + 10).all(|y| is(x, y, handle));
        let bar = ys.clone().filter(|&y| is(x, y, track) || is(x, y, handle)).count();
        let track = ys.clone().filter(|&y| is(x, y, track)).count();
        handle_at_top && bar as f32 > 0.95 * ys.len() as f32 && track > ys.len() / 2
    });
    let x = x.expect("a scroll bar at the right of the rows, its handle at the top");
    // A press low on it brings rows far down into view.
    let bar = egui::pos2(x, pane.bottom() - 40.0);
    h.hover_at(bar);
    h.run_steps(3);
    press(&h, bar, true, egui::Modifiers::NONE);
    h.run_steps(2);
    press(&h, bar, false, egui::Modifiers::NONE);
    h.run_steps(30);
    assert!(h.query_by_label("Acid_Fog").is_none(), "scrolled down by the scroll bar");
    // (The rows' numbers, at the left: those in view are far down.)
    let numbers = |h: &Harness<'_, Moonglow>| -> Vec<usize> {
        h.query_all(egui_kittest::kittest::by().role(egui::accesskit::Role::Label))
            .filter(|n| (pane.left()..pane.left() + 60.0).contains(&n.rect().center().x))
            .filter_map(|n| n.accesskit_node().value().and_then(|l| l.parse::<usize>().ok()))
            .collect()
    };
    let far = numbers(&h);
    assert!(far.iter().min().is_some_and(|n| *n > 300), "{far:?}");
    // The wheel: back up, and sideways; the column names go with the rows.
    let over = pane.center();
    h.hover_at(over);
    h.run_steps(2);
    let label = h.get_by_label("Label").rect().left();
    let wheel = |h: &mut Harness<'_, Moonglow>, delta: egui::Vec2| {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(30);
    };
    wheel(&mut h, egui::vec2(0.0, 100_000.0));
    h.get_by_label("Acid_Fog");
    // (A cell scrolled out of sight sideways is no longer laid out: it is
    // gone, or as far left as its column's name.)
    wheel(&mut h, egui::vec2(-600.0, 0.0));
    let went = |n: egui_kittest::Node<'_>| n.rect().left() < label - 300.0;
    assert!(h.query_by_label("Label").is_none_or(went));
    assert!(h.query_by_label("Acid_Fog").is_none_or(went), "the rows went with the names");
    // (And the columns that came into sight are laid out: row 0's cells.)
    let cells = h
        .query_all(egui_kittest::kittest::by().role(egui::accesskit::Role::Label))
        .filter(|n| pane.contains(n.rect().center()))
        .count();
    assert!(cells > 20, "{cells} labels in the pane");
    // The rows' numbers stay in sight at the left.
    assert!(numbers(&h).contains(&0), "{:?}", numbers(&h));
}

/// GitHub issue 18, in the 2DA editor: Ctrl+Z with a cell's text typed in
/// is the cell's own, not the table's Undo (its earlier change stays).
#[test]
fn undo_in_a_table_cell_being_typed_is_the_cell_s() {
    let dir = mg_testkit::scratch_dir("ui-table-undo-in-cell");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(SMALL_TABLE.to_vec()) }],
    )));
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    h.run_steps(4);
    // An earlier change: a cell typed and kept.
    h.get_by_label("Cheese").click();
    h.run_steps(2);
    h.key_press(egui::Key::F2);
    h.run_steps(3);
    h.event(egui::Event::Text("y".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert_eq!(table_of(&h, id).get(1, "Label"), Some("Cheesey"));
    // Another cell typed in, and Ctrl+Z: the typing's, not the table's.
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    h.key_press(egui::Key::F2);
    h.run_steps(3);
    h.event(egui::Event::Text("zz".into()));
    h.run_steps(2);
    assert!(h.ctx.egui_wants_keyboard_input());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(1, "Label"), Some("Cheesey"), "not the table's Undo");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(!h.ctx.egui_wants_keyboard_input());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(table_of(&h, id).get(1, "Label"), Some("Cheese"), "the table's, once left");
}

/// Next Pane (F6) gives the middle's keys to what its tab in front has (GitHub
/// issue 12): a table's cells, at its first cell where none is selected,
/// then the arrows move through them; Shift+F6 goes back to the tree.
#[test]
fn next_pane_gives_the_keys_to_a_table_in_the_middle() {
    let dir = mg_testkit::scratch_dir("ui-table-next-pane");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(SMALL_TABLE.to_vec()) }],
    )));
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    h.run_steps(4);
    let at =
        |h: &Harness<'_, Moonglow>| h.state().tables.iter().find(|d| d.id == id).unwrap().selected;
    assert_eq!(at(&h), None);
    let key_ = |h: &mut Harness<'_, Moonglow>, m: egui::Modifiers, k: egui::Key| {
        h.key_press_modifiers(m, k);
        h.run_steps(2);
    };
    let none = egui::Modifiers::NONE;
    // From the middle's: the log (no palettes here), the tree, then the
    // middle: the table's first cell.
    key_(&mut h, none, egui::Key::F6);
    assert!(h.state().tree_cursor.is_none(), "the log has the keys");
    key_(&mut h, none, egui::Key::F6);
    assert!(h.state().tree_cursor.is_some(), "the tree has the keys");
    key_(&mut h, none, egui::Key::F6);
    assert_eq!(at(&h), Some((0, 0)));
    key_(&mut h, none, egui::Key::ArrowDown);
    assert_eq!(at(&h), Some((1, 0)), "the arrows are the table's");
    let tree = h.state().tree_cursor;
    key_(&mut h, egui::Modifiers::SHIFT, egui::Key::F6);
    key_(&mut h, none, egui::Key::ArrowDown);
    assert_eq!(at(&h), Some((1, 0)), "the table's keys left");
    assert_ne!(h.state().tree_cursor, tree, "the tree's again");
}

/// A table saved keeps its file's layout: its row labels and its tabs.
/// Changed outside while open, it is read again (nothing of the
/// builder's unsaved), or said to have changed (something unsaved: a
/// save writes over it).
#[test]
fn a_file_s_table_keeps_its_layout_and_is_read_again_when_changed() {
    let dir = mg_testkit::scratch_dir("ui-table-read-again");
    let file = dir.join("tabs.2da");
    std::fs::write(&file, "2DA V2.0\n\n\tLabel\tCost\nbread\tBread\t1\ncheese\tCheese\t2\n")
        .unwrap();
    let mut h = harness(Moonglow::new(None, Box::new(NoDialogs::default())), 1200.0, 800.0);
    h.run_steps(2);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(3);
    let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
    doc.edit.set(1, 1, Some("3".into())).unwrap();
    assert!(mg_ui::table_view::save(h.state_mut(), id));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "2DA V2.0\n\n\tLabel\tCost\nbread\tBread\t1\ncheese\tCheese\t3\n",
        "its labels and tabs kept"
    );
    // Changed outside: read again.
    std::fs::write(&file, "2DA V2.0\n\n\tLabel\tCost\nbread\tBread\t1\ncheese\tCheese\t4\n")
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while table_of(&h, id).get(1, "Cost") != Some("4") {
        assert!(std::time::Instant::now() < deadline, "read again");
        h.run_steps(5);
    }
    // With a change not saved: said, and the builder's change kept.
    let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
    doc.edit.set(0, 1, Some("8".into())).unwrap();
    std::fs::write(&file, "2DA V2.0\n\n\tLabel\tCost\nbread\tBread\t1\ncheese\tCheese\t5\n")
        .unwrap();
    let said = |h: &Harness<'_, Moonglow>| {
        let doc = h.state().tables.iter().find(|d| d.id == id).unwrap();
        doc.problem.as_deref().is_some_and(|p| p.contains("Changed where it was read from"))
    };
    while !said(&h) {
        assert!(std::time::Instant::now() < deadline, "said");
        h.run_steps(5);
    }
    assert_eq!(table_of(&h, id).get(0, "Cost"), Some("8"));
    assert_eq!(table_of(&h, id).get(1, "Cost"), Some("4"));
}

/// A model's cell chosen says whether the game has the model (and where
/// it reads it from), and Choose Model… lists the models the game reads,
/// found by a part of the name: one chosen is put in the cell.
#[test]
fn a_model_s_cell_is_checked_and_chosen_from_the_game_s_models() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-table-models");
    let file = dir.join("placeables.2da");
    std::fs::write(
        &file,
        "2DA V2.0\n\n   Label   ModelName\n0  Chest   plc_c01\n1  Nothing mg_none\n",
    )
    .unwrap();
    let mut h = harness(game_app(&root, None), 1280.0, 1300.0);
    // (Tall: the list of models opens under the cell's panel.)
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    let choose = |h: &mut Harness<'_, Moonglow>, row: usize| {
        h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(row, 1);
        h.run_steps(4);
    };
    choose(&mut h, 0);
    h.get_by_label_contains("a model, read from");
    choose(&mut h, 1);
    h.get_by_label("no model of this name in the game, its haks or the module");
    h.get_by_label("Choose Model…").click();
    h.run_steps(3);
    let field = h.get_all_by_role(egui::accesskit::Role::TextInput).last().unwrap().rect().center();
    click_pos(&h, field);
    h.run_steps(2);
    h.event(egui::Event::Text("plc_c0".into()));
    h.run_steps(4);
    let at = h.get_by_label("plc_c02").rect().center();
    click_pos(&h, at);
    h.run_steps(4);
    assert_eq!(table_of(&h, id).get(1, "ModelName"), Some("plc_c02"));
}

/// The resource browser's read-only view of a table is the editor's grid
/// with the editing left out: a cell selected by a click and the arrows,
/// Ctrl+C copies, Find narrows the rows, Go to goes to one, a column is
/// hidden; Delete, typing and a double click change nothing.
#[test]
fn a_game_table_read_only_has_the_editor_s_keys_and_find() {
    let root = mg_testkit::corpus!();
    let mut h = harness(game_app(&root, None), 1280.0, 800.0);
    h.run_steps(3);
    let key = ResKey::parse("spells", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    h.run_steps(5);
    let before = h.state().game.as_deref().unwrap().table("spells").unwrap();
    let label = |r: usize| before.get(r, "Label").unwrap().to_string();
    h.get_by_label(&label(0)).click();
    h.run_steps(2);
    h.get_by_label_contains(&format!("Row 0, Label: {}", label(0)));
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowDown);
    h.run_steps(2);
    h.get_by_label_contains(&format!("Row 2, Label: {}", label(2)));
    h.event(egui::Event::Copy);
    h.step();
    let copied = h.output().platform_output.commands.iter().find_map(|c| match c {
        egui::OutputCommand::CopyText(t) => Some(t.clone()),
        _ => None,
    });
    assert_eq!(copied, Some(format!("{}\n{}", label(1), label(2))));
    // Nothing is typed into or emptied.
    let fields =
        |h: &Harness<'_, Moonglow>| h.get_all_by_role(egui::accesskit::Role::TextInput).count();
    let typing = fields(&h);
    h.key_press(egui::Key::Delete);
    h.key_press(egui::Key::Enter);
    h.event(egui::Event::Text("x".into()));
    h.run_steps(2);
    let fifth = h.get_by_label(&label(5)).rect().center();
    double_click(&mut h, fifth);
    h.run_steps(2);
    assert_eq!(fields(&h), typing, "no cell is typed in");
    assert!(h.query_all_by_label(&label(1)).count() > 0, "not emptied");
    // Find narrows the rows; Escape clears it.
    let find = h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap();
    click_pos(&h, find.rect().center());
    h.run_steps(2);
    h.event(egui::Event::Text("fireball".into()));
    h.run_steps(3);
    assert!(h.query_all_by_label("Fireball").count() > 0);
    assert!(h.query_by_label(&label(0)).is_none(), "narrowed");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    h.get_by_label(&label(0));
    // A column hidden, from its name's menu, and shown again.
    h.get_by_label("IconResRef").click_secondary();
    h.run_steps(2);
    h.get_by_label("Hide Column").click();
    h.run_steps(2);
    assert!(h.query_by_label("IconResRef").is_none());
    h.get_by_label("Show Hidden Columns (1)").click();
    h.run_steps(2);
    h.get_by_label("IconResRef");
    assert_eq!(
        h.state().game.as_deref().unwrap().table("spells").unwrap().get(1, "Label"),
        Some(label(1).as_str()),
        "the game's table as it was"
    );
}

/// A hak built from a folder: its table's description is the family's in
/// that folder (`tables = "loot_*"`), not looked for beside the hak's
/// file; and a copy of the game's spells.2da with no description of its
/// own is held to Moonglow's, the cells the game's copy has as they are
/// not said.
#[test]
fn a_hak_s_folder_s_family_and_moonglow_s_spells_describe_tables() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-table-families");
    let content = dir.join("content");
    std::fs::create_dir_all(&content).unwrap();
    std::fs::write(content.join("loot_a.2da"), "2DA V2.0\n\n   Label Chance\n0  a     500\n")
        .unwrap();
    std::fs::write(
        content.join("loot.2da.toml"),
        "tables = \"loot_*\"\n\n[columns.Chance]\nkind = \"int\"\nmax = 100\n",
    )
    .unwrap();
    let dialogs = NoDialogs { folders: vec![content.clone()], ..Default::default() };
    let mut h = harness(game_app_with(&root, None, dialogs), 1280.0, 800.0);
    h.run_steps(3);
    mg_ui::hak_view::build_from_folder(h.state_mut());
    h.run_steps(3);
    let hak = h.state().haks[0].id;
    let key = ResKey::parse("loot_a", ResType::TWODA).unwrap();
    mg_ui::table_view::open_in_hak(h.state_mut(), hak, key).unwrap();
    h.run_steps(3);
    settle(&mut h);
    h.get_by_label_contains("Described by loot.2da.toml");
    h.get_by_label("1 cell is wrong");
    // The game's spells.2da, a school changed to one the engine lacks.
    let game = game_data(&root);
    let mut t = (*game.table("spells").unwrap()).clone();
    let school = t.column("School").unwrap();
    t.set(1, school, Some("X".into())).unwrap();
    let file = dir.join("spells.2da");
    std::fs::write(&file, t.to_bytes(mg_core::Codepage::default()).unwrap()).unwrap();
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(3);
    settle(&mut h);
    h.get_by_label_contains("Described by Moonglow (the game's spells.2da)");
    // (Bigby's Crushing Hand's MetaMagic of 0xff, the game's, is not
    // said: one cell in each table open, the hak's and this.)
    assert_eq!(h.get_all_by_label("1 cell is wrong").count(), 2);
    let doc = h.state().tables.iter().find(|d| d.id == id).unwrap();
    let description = doc.description().unwrap();
    let school = &description.column("School").unwrap().kind;
    assert_eq!(school.named("V").as_deref(), Some("Evocation"));
}

/// Take Rows: a row both copies changed is taken cell by cell, each cell
/// saying whose change it is against the lowest copy (the game's).
#[test]
fn a_row_both_copies_changed_is_taken_cell_by_cell() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-table-take-cells");
    let user = user_folder(&dir, &["override"]);
    let game = game_data(&root);
    let base = (*game.table("feat").unwrap()).clone();
    let cell = |t: &mg_2da::TwoDa, c: &str| t.get(3, c).unwrap_or("****").to_string();
    let set = |t: &mut mg_2da::TwoDa, c: &str, v: &str| {
        let c = t.column(c).unwrap();
        t.set(3, c, Some(v.into())).unwrap();
    };
    // Theirs (the override's) changed row 3's LABEL and MINDEX; ours its
    // LABEL and MINSTR.
    let mut theirs = base.clone();
    set(&mut theirs, "LABEL", "Theirs");
    set(&mut theirs, "MINDEX", "15");
    let cp = mg_core::Codepage::default();
    std::fs::write(user.join("override/feat.2da"), theirs.to_bytes(cp).unwrap()).unwrap();
    let mut ours = base.clone();
    set(&mut ours, "LABEL", "Ours");
    set(&mut ours, "MINSTR", "13");
    let file = dir.join("feat.2da");
    std::fs::write(&file, ours.to_bytes(cp).unwrap()).unwrap();
    let mut h = harness(game_app(&root, Some(user)), 1280.0, 900.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    h.run_steps(4);
    h.get_by_label("Take Rows…").click();
    h.run_steps(4);
    h.get_by_label_contains("0 new, 0 changed there, 1 changed in both");
    h.get_by_label("Cells…").click();
    h.run_steps(3);
    h.get_by_label_contains("Row 3's cells");
    // (The row's, in the list, and LABEL's.)
    assert_eq!(h.get_all_by_label("changed in both").count(), 2);
    h.get_by_label("changed here");
    h.get_by_label("changed there");
    let mindex = format!("MINDEX: {} \u{2192} 15", cell(&base, "MINDEX"));
    h.get_by_label(&mindex).click();
    h.run_steps(2);
    h.get_by_label("Take 1 Cells").click();
    h.run_steps(3);
    let now = table_of(&h, id);
    assert_eq!(now.get(3, "MINDEX"), Some("15"), "taken");
    assert_eq!((now.get(3, "LABEL"), now.get(3, "MINSTR")), (Some("Ours"), Some("13")), "kept");
    assert!(h.state().tables[0].taking.is_none(), "the window closed");
    assert_eq!(h.state().tables[0].edit.undo_label(), Some("Take 1 cells"));
}

/// Find References to Row lists the module's blueprints, areas and
/// scripts that name the row, beside the tables' cells, and a click goes
/// there.
#[test]
fn find_references_to_row_finds_the_module_s_blueprints_and_scripts() {
    let dir = mg_testkit::scratch_dir("ui-table-row-uses");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let table = ResKey::parse("mg_looks", ResType::TWODA).unwrap();
    let guard = ResKey::parse("mg_guard", ResType::UTC).unwrap();
    let script = ResKey::parse("mg_look", ResType::NSS).unwrap();
    let mut utc = mg_gff::Gff::new(*b"UTC ");
    utc.root.set("Appearance_Type", mg_gff::Value::Word(2));
    let src = b"void main()\n{\n    string s = Get2DAString(\"mg_looks\", \"Label\", 2);\n}\n";
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table, a creature and a script",
        vec![
            mg_edit::Edit::SetResource {
                key: table,
                data: Some(b"2DA V2.0\n\n   Label\n0  a\n1  b\n2  c\n".to_vec()),
            },
            mg_edit::Edit::SetResource { key: guard, data: Some(utc.to_bytes().unwrap()) },
            mg_edit::Edit::SetResource { key: script, data: Some(src.to_vec()) },
        ],
    )));
    let mut h = harness(app, 1280.0, 900.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_in_module(h.state_mut(), table).unwrap();
    h.run_steps(4);
    // (The creature's appearance is appearance.2da's row, not this one's.)
    let uses = mg_ui::table_view::module_row_uses(h.state(), "appearance", 2);
    assert_eq!(uses.len(), 1, "{uses:?}");
    assert!(uses[0].place.ends_with("Appearance_Type"), "{uses:?}");
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(2, 0);
    h.run_steps(3);
    h.get_by_label("Find References to Row 2").click();
    h.run_steps(3);
    let line = h.get_by_label_contains("mg_look line 3: Get2DAString");
    line.click();
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&Tab::Script(script)).is_some(), "the script opened");
}

/// A talk-table string's cell is chosen from the talk tables by a part of
/// its text, the module's own and the game's, and given a new line of the
/// module's talk table, whose number goes in the cell.
#[test]
fn a_talk_table_string_s_cell_is_chosen_and_given_a_new_line() {
    use mg_module::talk::Line;
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-table-strings");
    let user = user_folder(&dir, &[]);
    let path = sample_module(&dir);
    let tlk_key = ResKey::parse("mg_strs", ResType::TLK).unwrap();
    let key = ResKey::parse("mg_strs", ResType::TWODA).unwrap();
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_CUSTOM_TLK, ExoString::from("mg_strs"));
    m.set_info(&info).unwrap();
    let mut t = mg_module::talk::Table::create("mg_strs", &dir, Language::ENGLISH, false);
    let line = |text: &str| Line {
        text: text.into(),
        feminine: None,
        sound: String::new(),
        sound_length: 0.0,
    };
    t.add_line(&line("Old words of the module")).unwrap();
    m.set(tlk_key, t.tlk.to_bytes().unwrap());
    m.set(key, b"2DA V2.0\n\n   Label Name\n0  a     ****\n1  b     ****\n".to_vec());
    m.save().unwrap();
    // (The column is a string's by the table's description.)
    std::fs::write(path.with_file_name("mg_strs.2da.toml"), "[columns.Name]\nkind = \"strref\"\n")
        .unwrap();
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1280.0, 1300.0);
    h.run_steps(3);
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    h.run_steps(4);
    // (The description's folder is walked on a thread of its own.)
    settle(&mut h);
    let choose = |h: &mut Harness<'_, Moonglow>, row: usize| {
        h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(row, 1);
        h.run_steps(4);
    };
    let type_in_last = |h: &mut Harness<'_, Moonglow>, role, text: &str| {
        let field = h.get_all_by_role(role).last().unwrap().rect().center();
        click_pos(h, field);
        h.run_steps(2);
        h.event(egui::Event::Text(text.into()));
        h.run_steps(4);
    };
    // The module's string, found by a part of its text.
    choose(&mut h, 0);
    h.get_by_label("Choose String…").click();
    h.run_steps(3);
    type_in_last(&mut h, egui::accesskit::Role::TextInput, "old words");
    h.get_by_label("16777216  Old words of the module").click();
    h.run_steps(4);
    assert_eq!(table_of(&h, id).get(0, "Name"), Some("16777216"));
    h.get_by_label("\u{201c}Old words of the module\u{201d}");
    // The game's, as well.
    let (n, text) = {
        let game = h.state().game.as_deref().unwrap();
        let tlk = game.tlk();
        (100..tlk.entries.len() as u32)
            .find_map(|n| {
                let t = tlk.text_in(mg_core::StrRef(n), game.codepage())?;
                (t.len() > 30 && t.len() < 60 && !t.contains('\n')).then_some((n, t))
            })
            .unwrap()
    };
    let found = mg_ui::table_view::strings_found(None, h.state().game.as_deref(), &text);
    assert!(found.0.iter().any(|(m, _)| *m == n), "{n} {text:?} in {found:?}");
    // A new line of the module's talk table, its number put in the cell.
    // (Added as the table is saved, where a cell still names it: Cancel
    // leaves the talk table as it was. The review of November 2026, the
    // last fixers' item 9.)
    let new_line = |h: &mut Harness<'_, Moonglow>, row: usize, text: &str| {
        choose(h, row);
        h.get_by_label("New Line…").click();
        h.run_steps(3);
        type_in_last(h, egui::accesskit::Role::MultilineTextInput, text);
        h.get_by_label("Add to Talk Table").click();
        h.run_steps(4);
    };
    new_line(&mut h, 1, "Brand new words");
    assert_eq!(table_of(&h, id).get(1, "Name"), Some("16777217"));
    // (Shown by its text before it is added.)
    h.get_by_label("\u{201c}Brand new words\u{201d}");
    new_line(&mut h, 0, "Other new words");
    assert_eq!(table_of(&h, id).get(0, "Name"), Some("16777218"), "a line of its own");
    let talk = h.state().talk.as_ref().expect("the module's talk table");
    assert_eq!((talk.len(), talk.is_dirty()), (1, false), "not added before the table's Save");
    // Cancel, Don't Save: nothing added.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Table(id)]));
    h.run_steps(3);
    h.get_by_label("Don't Save").click();
    h.run_steps(4);
    assert!(!h.state().tables.iter().any(|d| d.id == id));
    let talk = h.state().talk.as_ref().unwrap();
    assert_eq!((talk.len(), talk.is_dirty()), (1, false), "Cancel adds nothing");
    // Opened again: two lines, the first named by a cell no more; OK adds
    // the one named, by the number the cell shows.
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    h.run_steps(4);
    settle(&mut h);
    let choose = |h: &mut Harness<'_, Moonglow>, row: usize| {
        h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(row, 1);
        h.run_steps(4);
    };
    choose(&mut h, 0);
    h.get_by_label("New Line…").click();
    h.run_steps(3);
    type_in_last(&mut h, egui::accesskit::Role::MultilineTextInput, "Gone again");
    h.get_by_label("Add to Talk Table").click();
    h.run_steps(4);
    assert_eq!(table_of(&h, id).get(0, "Name"), Some("16777217"));
    choose(&mut h, 1);
    h.get_by_label("New Line…").click();
    h.run_steps(3);
    type_in_last(&mut h, egui::accesskit::Role::MultilineTextInput, "Brand new words");
    h.get_by_label("Add to Talk Table").click();
    h.run_steps(4);
    assert_eq!(table_of(&h, id).get(1, "Name"), Some("16777218"));
    // (Row 0 given the module's first string again.)
    let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
    doc.edit.set(0, 1, Some("16777216".into())).unwrap();
    h.run_steps(2);
    // (The module's Save: the line is in the talk table it saves.)
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    let talk = h.state().talk.as_ref().unwrap();
    assert_eq!((talk.len(), talk.line(1).text.as_str()), (2, "Brand new words"));
    assert!(!talk.is_dirty(), "saved with the module");
    let saved = h.state_mut().ws.as_mut().unwrap().module.get(&key).unwrap().to_vec();
    let saved = mg_2da::TwoDa::parse(&saved, mg_core::Codepage::default()).unwrap();
    assert_eq!(saved.get(1, "Name"), Some("16777217"), "the number the line took");
}
