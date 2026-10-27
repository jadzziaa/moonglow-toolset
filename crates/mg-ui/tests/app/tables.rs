//! Talk tables and 2DA tables.

use crate::common::*;

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
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("tlk")).unwrap();
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
    let user = dir.join("user");
    std::fs::create_dir_all(&user).unwrap();
    let path = sample_module(&dir);
    let install = mg_resman::GameInstall::new(&root, Some(user.clone()), "en");
    // (Export CSV and Import CSV are answered with the same file.)
    let csv = dir.join("lines.csv");
    let dialogs =
        NoDialogs { open: vec![csv.clone(); 2], save: vec![csv.clone(); 2], ..Default::default() };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
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
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
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
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let info = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap().root.clone();
    let name: LocString = info.read(&ifo::MOD_NAME);
    assert_eq!((name.strref.0, name.strings.len()), (16_777_217, 0));
    let t = h.state().talk.as_ref().unwrap();
    assert_eq!((t.line(1).text.as_str(), t.is_dirty()), ("The Keep", true));
}

#[test]
fn two_da_view_shows_where_rows_come_from() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-2da-layers");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    // The game's ambientmusic.2da with a row changed and one added, in
    // override.
    let game =
        mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let key = ResKey::parse("ambientmusic", ResType::TWODA).unwrap();
    let cp = mg_core::Codepage::default();
    let t = mg_2da::TwoDa::parse(&game.get(&key).unwrap(), cp).unwrap();
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
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
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
    // (The empty row at the end is not in the file: a table read has no
    // empty rows after its last.)
    assert_eq!(stored(&mut h).rows[..], table_of(&h, id).rows[..2]);
    assert!(h.query_by_label("(unsaved)").is_none(), "saved: not marked");
    // (Closed, the table's grid has the keys no more: Undo is the
    // module's.)
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Table(id)]));
    h.run_steps(2);
    assert!(h.state().tables.is_empty(), "saved: closed without a question");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(stored(&mut h).get(1, "Label"), Some("Cheese"), "the module's Undo takes the save");
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

/// With the game's data: a cell is checked by what its column holds as
/// it is typed, a row of another table is chosen by name, where a row is
/// named is found, and rows are taken from the game's copy.
#[test]
fn a_table_s_cells_are_checked_and_rows_taken_from_another_copy() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-table-kinds");
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut t = (*game.table("feat").unwrap()).clone();
    let whole = t.len();
    let (prereq, label) = (t.column("PREREQFEAT1").unwrap(), t.column("LABEL").unwrap());
    t.rows.truncate(whole - 25);
    t.set(5, label, Some("MyOwnShot".into())).unwrap();
    let file = dir.join("feat.2da");
    std::fs::write(&file, t.to_bytes(mg_core::Codepage::default()).unwrap()).unwrap();
    let app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
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
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let file = dir.join("skills.2da");
    let dialogs = NoDialogs { save: vec![file.clone()], ..Default::default() };
    let mut app = Moonglow::new(Some(install), Box::new(dialogs));
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
    assert_eq!(wide[0].0, "loose");
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
    for delta in [egui::vec2(0.0, 100_000.0), egui::vec2(-600.0, 0.0)] {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(30);
    }
    h.get_by_label("Acid_Fog");
    assert!(h.query_by_label("Label").is_none_or(|n| n.rect().left() < label - 300.0));
    let acid = h.get_by_label("Acid_Fog").rect().left();
    assert!(acid < label - 300.0, "the rows went sideways with the names: {acid}");
    // The rows' numbers stay in sight at the left.
    assert!(numbers(&h).contains(&0), "{:?}", numbers(&h));
}
