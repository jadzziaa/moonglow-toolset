//! A talk table's editor and panels (M4's U17; the Talk board): the
//! header on the kit's row (every control on it or in More at every width;
//! Close for a table only looked at, OK and Cancel otherwise), a header row
//! on the list, the Right Dock's String, References and Talk Table (or a
//! floating talk table's window's right pane), the Status Bar's count and
//! hints; the game's `dialog.tlk` opened read-only under its own StrRefs;
//! and what holds them: the String panel editing a line, its typed text
//! kept to its own line, References finding the line's users on a thread
//! of its own, Find looking through a long table a slice a frame, every
//! panel fitting the dock from its least width, the panels following the
//! table in front, the window coming to rest, and records from before
//! reading without a fault.

use std::sync::Arc;

use crate::common::*;
use mg_module::talk::{Line, Table};
use mg_ui::Panel;
use mg_ui::talk_view::{self, Which};

/// The lines of the module's table of ours: 30, some with a sound, 12 and
/// 13 empty, line 4 of several lines.
fn our_line(n: usize) -> Line {
    let (text, sound) = match n {
        12 | 13 => (String::new(), String::new()),
        4 => {
            ("The first line of four.\nThe second.\nThe third.\nThe fourth.".into(), String::new())
        }
        n if n % 3 == 0 => (format!("The watch asks for line {n}."), format!("vs_watch_{n:03}")),
        n => (format!("Line {n} of the harbor."), String::new()),
    };
    let sound_length = if sound.is_empty() { 0.0 } else { 2.5 };
    Line { text, feminine: None, sound, sound_length }
}

/// The game's `dialog.tlk` of ours: 40 lines, line 7 "Well met, traveler."
fn our_dialog() -> mg_tlk::Tlk {
    let mut tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
    for n in 0..40 {
        let text = match n {
            7 => "Well met, traveler.".to_string(),
            20 => String::new(),
            n => format!("Game line {n}"),
        };
        tlk.entries.push(mg_tlk::TlkEntry::text(text.as_str()));
    }
    tlk
}

/// A module open in `mode` in a window `width` by 900 that names its own
/// talk table `mg_mine` (a file of ours in a scratch folder, open in the
/// editor as one found in the user folder's tlk is: no game install), with
/// game data of ours whose `dialog.tlk` is [`our_dialog`]; a creature
/// blueprint `mg_guard` names line 7 of the game's table (its first name)
/// and line 2 of the module's (its last name).
fn talk_window(mode: Mode, name: &str, width: f32) -> Harness<'static, Moonglow> {
    talk_window_with(mode, name, width, our_dialog())
}

/// [`our_dialog`] and 150,000 lines more, the last "A needle in the hay":
/// long enough that Find looks through it over many frames.
fn long_dialog() -> mg_tlk::Tlk {
    let mut tlk = our_dialog();
    let filler = mg_tlk::TlkEntry::text(
        "The watch patrols the outer gate until dusk and keeps an eye on passing wagons, \
         merchants, and travelers arriving along the cobbled road from the east.",
    );
    tlk.entries.resize(150_039, filler);
    tlk.entries.push(mg_tlk::TlkEntry::text("A needle in the hay"));
    tlk
}

/// [`talk_window`] with `dialog` the game data's `dialog.tlk`.
fn talk_window_with(
    mode: Mode,
    name: &str,
    width: f32,
    dialog: mg_tlk::Tlk,
) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_mine".to_vec()));
    m.set_info(&info).unwrap();
    let mut utc = mg_gff::Gff::new(*b"UTC ");
    let named = |n| mg_gff::Value::LocString(LocString::from_strref(mg_core::StrRef(n)));
    utc.root.set("FirstName", named(7));
    utc.root.set("LastName", named(16_777_218));
    utc.root.set("Tag", mg_gff::Value::String(b"MG_GUARD".to_vec()));
    m.set_gff(ResKey::parse("mg_guard", ResType::UTC).unwrap(), &utc).unwrap();
    m.save().unwrap();
    let tables = dir.join("tlk");
    std::fs::create_dir_all(&tables).unwrap();
    let mut t = Table::create("mg_mine", &tables, mg_core::Language::ENGLISH, false);
    for n in 0..30 {
        t.add_line(&our_line(n)).unwrap();
    }
    t.save().unwrap();
    // (Read again from its file, as the editor opens it: no history.)
    let file = tables.join("mg_mine.tlk");
    let found = mg_module::talk::Found {
        source: mg_module::talk::Source::File(file.clone()),
        data: std::fs::read(&file).unwrap(),
    };
    let t = Table::open("mg_mine", found, None).unwrap();
    let mut app = app_in(mode, Vec::new());
    app.settings.no_last_area = true;
    app.game = Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), dialog)));
    app.open_module(&path);
    app.talk = Some(t);
    let mut h = harness(app, width, 900.0);
    settle(&mut h);
    h
}

/// Opens the module's talk table's tab and lets the window come to rest.
fn open_own(h: &mut Harness<'_, Moonglow>) {
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    settle(h);
    assert!(h.state().tab_rect(&Tab::TalkTable).is_some(), "its tab drawn");
}

/// Chooses line `row` of the module's table (as Go to does), its list given
/// the keys.
fn choose(h: &mut Harness<'_, Moonglow>, which: Which, row: usize) {
    match which {
        Which::Module => h.state_mut().talk_view.go_to_line(row),
        Which::Game => h.state_mut().game_talk.as_mut().unwrap().view.go_to_line(row),
    }
    talk_view::take_keys(&h.ctx, which);
    settle(h);
}

/// Line `row` of the module's table as it is now.
fn line(h: &Harness<'_, Moonglow>, row: usize) -> Line {
    h.state().talk.as_ref().unwrap().line(row)
}

/// The module's table's line chosen.
fn chosen(h: &Harness<'_, Moonglow>) -> Option<usize> {
    h.state().talk_view.selected
}

/// Types `text` into the String panel's field `name`, clicked first.
fn type_in(h: &mut Harness<'_, Moonglow>, name: &str, text: &str) {
    string_field(h, name).click();
    settle(h);
    h.event(egui::Event::Text(text.into()));
    settle(h);
}

/// Whether the String panel's field `name` is shown with `value`.
fn shows(h: &Harness<'_, Moonglow>, name: &str, value: &str) -> bool {
    use egui::accesskit::Role;
    h.query_all_by_label(name).any(|n| {
        matches!(n.accesskit_node().role(), Role::TextInput | Role::MultilineTextInput)
            && n.accesskit_node().value().as_deref() == Some(value)
    })
}

/// `dialog.tlk` opens read-only, numbered by its own StrRefs (the plan's
/// test; ruling 7): from the Resources panel's row and from Quick Open, in
/// the Talk Table tab; its lines 0 to 39, none of a custom table's numbers;
/// Add Line, Remove Last Line, Save and the imports off, Close alone at the
/// header's end; String's Text read-only (what is typed changes nothing);
/// the Status Bar's "40 lines, read-only"; Go to by its own numbers. Its
/// lines are shared, never copied: the same `Arc` after frames, a Find,
/// Only lines with text, the keys and a scroll.
#[test]
fn dialog_tlk_opens_read_only_and_numbered_by_its_strrefs() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-dialog", 1600.0);
    // From the Resources panel: its row, then Enter.
    h.get_by_label("Resources").click();
    settle(&mut h);
    type_into_hint(&mut h, "Name", "dialog");
    let row = h.get_by_label_contains("dialog.tlk");
    row.click();
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "opened from Resources");
    assert!(h.state().game_talk.is_some(), "showing dialog.tlk");
    h.get_by_label(talk_view::TALK_HEADER).get_by_label("Close").click();
    settle(&mut h);
    assert!(h.state().tab_rect(&Tab::TalkTable).is_none(), "closed by Close");
    assert!(h.state().game_talk.is_none(), "its lines let go of with it");
    // From Quick Open: "dialog.tlk" found, chosen.
    h.get_by_label(mg_ui::quick_open::NAME).click();
    settle(&mut h);
    h.event(egui::Event::Text("dialog.tlk".into()));
    settle(&mut h);
    h.get_by_label(mg_ui::quick_open::RESULTS).get_by_label_contains("dialog.tlk").click();
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "opened from Quick Open");
    assert!(h.state().game_talk.is_some(), "showing dialog.tlk");
    // Numbered by its own StrRefs.
    h.get_by_label("0");
    h.get_by_label("7");
    assert!(h.query_by_label("16777216").is_none(), "no custom table's numbers");
    // Nothing changes it.
    for name in [talk_view::ADD_LINE, talk_view::REMOVE_LAST_LINE, "Save"] {
        assert!(talk_tool(&mut h, name).accesskit_node().is_disabled(), "{name} off");
    }
    for name in [talk_view::IMPORT_CSV, talk_view::IMPORT_JSON] {
        assert!(talk_tool(&mut h, name).accesskit_node().is_disabled(), "{name} off");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
    }
    let header = h.get_by_label(talk_view::TALK_HEADER);
    header.get_by_label("Close");
    assert!(header.query_by_label("OK").is_none() && header.query_by_label("Cancel").is_none());
    choose(&mut h, Which::Game, 7);
    h.get_by_label_contains("Read-only: the game's own talk table");
    assert!(shows(&h, "Text", "Well met, traveler."), "its text shown");
    string_field(&h, "Text").click();
    settle(&mut h);
    h.event(egui::Event::Text("X".into()));
    settle(&mut h);
    assert!(shows(&h, "Text", "Well met, traveler."), "the field as it was");
    assert!(!h.state().has_unsaved_work(), "nothing changed");
    says(&mut h, &["40 lines, read-only"], &[]);
    // Go to by its own numbers.
    let shared = Arc::as_ptr(&h.state().game_talk.as_ref().unwrap().table);
    type_into_hint(&mut h, "StrRef or line", "16777220");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    h.get_by_label_contains("The table has 40 lines: StrRefs 0 to 39.");
    h.state_mut().game_talk.as_mut().unwrap().view.go_to.clear();
    settle(&mut h);
    type_into_hint(&mut h, "StrRef or line", "30");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().game_talk.as_ref().unwrap().view.selected, Some(30));
    // Shared, never copied: after the keys, a Find, Only lines with text
    // and a scroll, the same lines.
    h.key_press(egui::Key::End);
    settle(&mut h);
    type_into_hint(&mut h, "words or a StrRef", "traveler");
    settle(&mut h);
    h.get_by_label("7");
    assert!(h.query_by_label("8").is_none(), "Find narrows the list");
    // Find takes the game's numbers: a custom table's StrRef of line 7
    // finds nothing here.
    h.state_mut().game_talk.as_mut().unwrap().view.filter = "16777223".into();
    settle(&mut h);
    assert!(h.query_by_label("7").is_none(), "16777223 is no line of dialog.tlk");
    h.state_mut().game_talk.as_mut().unwrap().view.filter = "7".into();
    settle(&mut h);
    h.get_by_label("7");
    talk_click(&mut h, talk_view::ONLY_TEXT);
    h.state_mut().game_talk.as_mut().unwrap().view.filter.clear();
    settle(&mut h);
    assert!(h.query_by_label("20").is_none(), "an empty line not listed");
    h.get_by_label("8").scroll_to_me();
    settle(&mut h);
    let now = Arc::as_ptr(&h.state().game_talk.as_ref().unwrap().table);
    assert_eq!(now, shared, "the same lines, never copied");
    // A step of the module's and its Save: the same lines still.
    let ws = h.state().ws.as_ref().unwrap();
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    let _ = ws;
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A tag",
        vec![mg_edit::Edit::SetField {
            key: info,
            path: mg_edit::GffPath::root(),
            label: "Mod_Tag".into(),
            value: Some(mg_gff::Value::String(b"OTHER".to_vec())),
        }],
    )));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert!(!h.state().has_unsaved_work(), "saved");
    let now = Arc::as_ptr(&h.state().game_talk.as_ref().unwrap().table);
    assert_eq!(now, shared, "the same lines after a step and a Save");
    // Export CSV… writes its lines by their own numbers.
    let dir = mg_testkit::scratch_dir("ui-talk-dialog-export");
    let csv = dir.join("dialog.csv");
    h.state_mut().dialogs = Box::new(NoDialogs { save: vec![csv.clone()], ..Default::default() });
    talk_export(&mut h, "Export CSV…");
    let written = std::fs::read_to_string(&csv).unwrap();
    assert!(written.lines().any(|l| l == "7,\"Well met, traveler.\",,"), "{written}");
    assert_eq!(written.lines().count(), 41, "a heading and its 40 lines");
}

/// The install's own `dialog.tlk` (skipped without one): its 112,228-odd
/// lines (as many as the game data read), read-only, line 18608 as the
/// game data has it, the Status Bar's count.
#[test]
fn the_install_s_dialog_tlk_opens_read_only() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    let lines = app.game.as_deref().unwrap().tlk().entries.len();
    let text = app.game.as_deref().unwrap().string(mg_core::StrRef(18608)).unwrap_or_default();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    choose(&mut h, Which::Game, 18608);
    let g = h.state().game_talk.as_ref().unwrap();
    assert_eq!((g.table.len(), g.table.editable()), (lines, false));
    assert_eq!(g.table.line(18608).text, text);
    h.get_by_label("18608");
    let count = format!("{lines} lines, read-only");
    says(&mut h, &[count.as_str()], &[]);
}

/// The String panel edits a line (the plan's test), docked and floating:
/// its Text typed (a burst one step of the Top Bar's Undo), its Sound and
/// Length; Ctrl+Z with the pointer over the docked panel the table's; the
/// list follows; Cancel, Don't Save, leaves the file as it was.
#[test]
fn the_string_panel_edits_a_line() {
    for mode in MODES {
        let mut h = talk_window(mode, "ui-talk-string", 1920.0);
        open_own(&mut h);
        choose(&mut h, Which::Module, 5);
        let what = format!("{mode:?}");
        assert!(shows(&h, "Text", "Line 5 of the harbor."), "{what}: its text");
        type_in(&mut h, "Text", "!");
        type_in(&mut h, "Text", "?");
        assert!(line(&h, 5).text.ends_with("!?"), "{what}: typed: {:?}", line(&h, 5));
        type_in(&mut h, "Sound", "vs_new");
        assert_eq!(line(&h, 5).sound, "vs_new", "{what}: the sound");
        // The Length, typed (its field clicked: its value typed anew).
        let length = h
            .get_all_by_label("Length (s)")
            .find(|n| n.accesskit_node().role() != egui::accesskit::Role::Label);
        length.expect("Length's field").click();
        settle(&mut h);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.event(egui::Event::Text("3.5".into()));
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(line(&h, 5).sound_length, 3.5, "{what}: the length");
        tool(&h, "Undo").click();
        settle(&mut h);
        assert_eq!(line(&h, 5).sound_length, 0.0, "{what}: the length undone");
        // (The list shows it.)
        h.get_by_label("16777221");
        // One step a burst: the sound, then the text's two letters.
        tool(&h, "Undo").click();
        settle(&mut h);
        assert_eq!(line(&h, 5).sound, "", "{what}: the sound undone");
        assert!(line(&h, 5).text.ends_with("!?"), "{what}: the text kept");
        tool(&h, "Undo").click();
        settle(&mut h);
        assert_eq!(line(&h, 5).text, "Line 5 of the harbor.", "{what}: one step");
        tool(&h, "Redo").click();
        settle(&mut h);
        assert!(line(&h, 5).text.ends_with("!?"), "{what}: redone");
        // Ctrl+Z over the docked panel (its fields let go of): the table's.
        if mode == Mode::Tabs {
            let over = string_field(&h, "Sound").rect().center();
            h.get_by_label("16777221").click();
            settle(&mut h);
            h.hover_at(over);
            settle(&mut h);
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
            settle(&mut h);
            assert_eq!(line(&h, 5).text, "Line 5 of the harbor.", "{what}: Ctrl+Z over String");
            tool(&h, "Redo").click();
            settle(&mut h);
        }
        // Cancel, Don't Save: the file as it was.
        let file = h.state().talk.as_ref().unwrap().source.file().unwrap().to_path_buf();
        let before = std::fs::read(&file).unwrap();
        h.get_by_label(talk_view::TALK_HEADER).get_by_label("Cancel").click();
        settle(&mut h);
        h.get_by_label("Don't Save").click();
        settle(&mut h);
        assert_eq!(std::fs::read(&file).unwrap(), before, "{what}: the file as it was");
        assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "{what}: closed");
    }
}

/// What is typed in String keeps to its own line: the line chosen
/// elsewhere meanwhile (Go to) while Text, Length and Sound are typed in, a
/// line added, the last removed with it chosen, an import replacing the
/// line, the file changed outside, and
/// String not drawn (the dock folded): nothing lands on another line, and
/// what was typed is the line's.
#[test]
fn text_typed_keeps_to_its_own_line() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-typed", 1920.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    type_in(&mut h, "Text", "a");
    // Another line chosen while the field has the keyboard: what follows
    // lands nowhere else.
    h.state_mut().talk_view.go_to_line(5);
    settle(&mut h);
    h.event(egui::Event::Text("Z".into()));
    settle(&mut h);
    assert!(
        line(&h, 3).text.ends_with('a') && !line(&h, 3).text.contains('Z'),
        "{:?}",
        line(&h, 3)
    );
    assert!(!line(&h, 5).text.contains('Z'), "{:?}", line(&h, 5));
    // The Length being typed (not yet set) as another line is chosen: the
    // other line's Length takes none of it.
    choose(&mut h, Which::Module, 3);
    let length = h
        .get_all_by_label("Length (s)")
        .find(|n| n.accesskit_node().role() != egui::accesskit::Role::Label);
    length.expect("Length's field").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("9".into()));
    settle(&mut h);
    h.state_mut().talk_view.go_to_line(5);
    settle(&mut h);
    // (Typed after the move: with one id for every line's Length, the
    // field would still be the one typed in, now line 5's.)
    h.event(egui::Event::Text("1".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(line(&h, 5).sound_length, 0.0, "line 5's length as it was");
    // The Sound the same.
    choose(&mut h, Which::Module, 3);
    type_in(&mut h, "Sound", "x");
    h.state_mut().talk_view.go_to_line(5);
    settle(&mut h);
    h.event(egui::Event::Text("y".into()));
    settle(&mut h);
    assert_eq!(line(&h, 5).sound, "", "line 5's sound as it was");
    // A line added while typing: the new line's field, empty.
    choose(&mut h, Which::Module, 3);
    type_in(&mut h, "Text", "b");
    talk_click(&mut h, talk_view::ADD_LINE);
    assert_eq!(chosen(&h), Some(30));
    h.event(egui::Event::Text("Y".into()));
    settle(&mut h);
    assert!(line(&h, 3).text.ends_with("ab"), "{:?}", line(&h, 3));
    assert!(!line(&h, 3).text.contains('Y'), "{:?}", line(&h, 3));
    // The last removed, chosen and typed in: let go of.
    type_in(&mut h, "Text", "gone");
    assert_eq!(line(&h, 30).text, "gone");
    talk_click(&mut h, talk_view::REMOVE_LAST_LINE);
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 30);
    assert_eq!(chosen(&h), None, "nothing chosen past the end");
    h.event(egui::Event::Text("W".into()));
    settle(&mut h);
    assert!((0..30).all(|r| !line(&h, r).text.contains('W')), "nowhere");
    // An import replacing the line typed in: the field shows the import's,
    // what is typed after it goes nowhere else.
    choose(&mut h, Which::Module, 2);
    type_in(&mut h, "Text", "q");
    let json = mg_testkit::scratch_dir("ui-talk-typed-import").join("lines.json");
    std::fs::write(&json, r#"{"language":0,"entries":[{"id":2,"text":"Imported"}]}"#).unwrap();
    h.state_mut().dialogs = Box::new(NoDialogs { open: vec![json], ..Default::default() });
    talk_click(&mut h, talk_view::IMPORT_JSON);
    assert_eq!(line(&h, 2).text, "Imported", "imported");
    assert!(shows(&h, "Text", "Imported"), "the field shows the import's");
    h.event(egui::Event::Text("V".into()));
    settle(&mut h);
    assert!((0..30).all(|r| !line(&h, r).text.contains('V')), "nowhere");
    // The lines read again from the file, changed outside: the field shows
    // the file's.
    talk_click(&mut h, "Save");
    choose(&mut h, Which::Module, 1);
    let file = h.state().talk.as_ref().unwrap().source.file().unwrap().to_path_buf();
    let mut t = Table::open(
        "mg_mine",
        mg_module::talk::Found {
            source: mg_module::talk::Source::File(file.clone()),
            data: std::fs::read(&file).unwrap(),
        },
        None,
    )
    .unwrap();
    let mut changed = line(&h, 1);
    changed.text = "From outside".into();
    t.set_line(1, &changed, None).unwrap();
    // (A second later: the file's time moves.)
    std::thread::sleep(std::time::Duration::from_millis(1100));
    t.save().unwrap();
    h.state_mut().reload_resources(false);
    settle(&mut h);
    assert!(shows(&h, "Text", "From outside"), "read again");
    // String not drawn (the dock folded): what was typed is the line's.
    type_in(&mut h, "Text", "!");
    h.state_mut().settings.hide_palettes = true;
    settle(&mut h);
    assert!(h.query_by_label_contains("StrRef 16777217, line 1").is_none(), "String not drawn");
    assert_eq!(line(&h, 1).text, "From outside!");
}

/// References finds the line's users (the plan's test): held at its start,
/// it says it is looking (the window under way) and is stopped as another
/// line is chosen; let go, "Used in 1 place:" and the creature that names
/// the line, a link that opens it; a line nothing names says so. What it
/// finds is what the Find References tab finds for the same StrRef, of the
/// game's table and the module's.
#[test]
fn references_finds_the_line_s_users() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-references", 1920.0);
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    choose(&mut h, Which::Game, 7);
    let hold = h.state().hold_reference_looks.clone();
    hold.store(true, std::sync::atomic::Ordering::Relaxed);
    talk_click_held(&mut h, talk_view::FIND_REFERENCES);
    h.get_by_label_contains("Looking through the module and its haks");
    assert!(h.state().under_way().is_some(), "under way");
    // Another line chosen: stopped.
    choose_held(&mut h, Which::Game, 8);
    assert!(!h.state().game_talk.as_ref().unwrap().view.looking(), "stopped");
    hold.store(false, std::sync::atomic::Ordering::Relaxed);
    choose(&mut h, Which::Game, 7);
    talk_click(&mut h, talk_view::FIND_REFERENCES);
    h.get_by_label("Used in 1 place:");
    // The Find References tab's, for the same StrRef: the same place.
    let theirs = {
        h.state_mut().actions.push(mg_ui::Action::FindStrRef(7));
        settle(&mut h);
        h.state().references.usages.iter().map(|u| u.place.clone()).collect::<Vec<_>>()
    };
    assert_eq!(theirs.len(), 1, "{theirs:?}");
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    // (The board's row: the line's text quoted before the field.)
    let (before, last) = theirs[0].rsplit_once(" › ").unwrap();
    let quoted = format!("{before} “Well met, traveler.” › {last}");
    let link = h.get_by_label(&quoted);
    link.click();
    settle(&mut h);
    let utc = Tab::Blueprint(ResKey::parse("mg_guard", ResType::UTC).unwrap());
    assert_eq!(h.state().active_editor(), Some(&utc), "gone there");
    // The module's line 2 (16777218): the same creature (the tab back on
    // the module's table).
    show(&mut h, Which::Module);
    choose(&mut h, Which::Module, 2);
    talk_click(&mut h, talk_view::FIND_REFERENCES);
    h.get_by_label("Used in 1 place:");
    // A step of the module's: what was found let go of.
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A tag",
        vec![mg_edit::Edit::SetField {
            key: info,
            path: mg_edit::GffPath::root(),
            label: "Mod_Tag".into(),
            value: Some(mg_gff::Value::String(b"OTHER".to_vec())),
        }],
    )));
    settle(&mut h);
    assert!(h.query_by_label("Used in 1 place:").is_none(), "let go of after a step");
    // Line 6: nothing.
    choose(&mut h, Which::Module, 6);
    talk_click(&mut h, talk_view::FIND_REFERENCES);
    h.get_by_label_contains("Nothing names line 16777222");
    // Held, its tab closed: stopped, the window at rest.
    hold.store(true, std::sync::atomic::Ordering::Relaxed);
    choose(&mut h, Which::Module, 2);
    talk_click_held(&mut h, talk_view::FIND_REFERENCES);
    assert!(h.state().talk_view.looking(), "looking");
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::TalkTable));
    h.run_steps(4);
    assert!(!h.state().talk_view.looking(), "stopped with its tab");
    hold.store(false, std::sync::atomic::Ordering::Relaxed);
    settle(&mut h);
}

/// [`talk_click`] with a look held at its start: frames run, not waited to
/// rest (the look stays under way).
fn talk_click_held(h: &mut Harness<'_, Moonglow>, name: &str) {
    talk_tool(h, name).click();
    h.run_steps(4);
}

/// [`choose`] with a look held: frames run, not waited to rest.
fn choose_held(h: &mut Harness<'_, Moonglow>, which: Which, row: usize) {
    match which {
        Which::Module => h.state_mut().talk_view.go_to_line(row),
        Which::Game => h.state_mut().game_talk.as_mut().unwrap().view.go_to_line(row),
    }
    h.run_steps(4);
}

/// What a talk table's panel shows in `dock` fits it: nothing in sight past
/// its edge, every one-line field in sight as wide as its text.
#[track_caller]
fn fits(h: &Harness<'_, Moonglow>, dock: egui::Rect, what: &str) {
    fits_dock(h, dock, what, egui::FontId::monospace(mg_uikit::metrics::BODY_SIZE), false);
}

/// Every panel of a talk table fits the Right Dock from its least width
/// (280, 340, 440, 600): the module's line with a sound, and the game's
/// table (its read-only note, Module's Table for a module that names none),
/// each section scrolled to its foot; nothing past the dock's edge, every
/// field as wide as its text.
#[test]
fn every_talk_panel_fits_the_right_dock() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-fit", 1600.0);
    open_own(&mut h);
    for width in [280.0, 340.0, 440.0, 600.0] {
        right_dock_width(&mut h, width);
        settle(&mut h);
        choose(&mut h, Which::Module, 6);
        let dock = right_dock_rect(&h);
        let what = format!("{width} own");
        assert!((dock.width() - width - 1.0).abs() < 2.0, "{what}: {dock:?}");
        assert!(shows(&h, "Sound", "vs_watch_006"), "{what}: its sound");
        fits(&h, dock, &what);
        string_field(&h, "Sound").scroll_to_me();
        settle(&mut h);
        fits(&h, dock, &format!("{what} at String's foot"));
        h.get_all_by_label_contains("mg_mine.tlk").last().unwrap().scroll_to_me();
        settle(&mut h);
        fits(&h, dock, &format!("{what} at Talk Table's foot"));
    }
    // The game's, the module naming none: Module's Table (a user folder of
    // the test's own, where it would make one).
    let user = user_folder(&mg_testkit::scratch_dir("ui-talk-fit-user"), &["tlk"]);
    let root = mg_testkit::scratch_dir("ui-talk-fit-root");
    h.state_mut().install = Some(mg_resman::GameInstall::new(&root, Some(user), "en"));
    h.state_mut().talk = None;
    let ws = h.state_mut().ws.as_mut().unwrap();
    let mut info = ws.module.info().unwrap();
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(Vec::new()));
    ws.module.set_info(&info).unwrap();
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    for width in [280.0, 340.0, 440.0, 600.0] {
        right_dock_width(&mut h, width);
        settle(&mut h);
        choose(&mut h, Which::Game, 7);
        let dock = right_dock_rect(&h);
        let what = format!("{width} game");
        fits(&h, dock, &what);
        h.get_by_label("Create").scroll_to_me();
        settle(&mut h);
        h.get_by_label("With a feminine table");
        fits(&h, dock, &format!("{what} at Module's Table"));
    }
}

/// The String panel's content's height, as drawn: from its Text to the
/// foot of its Length.
fn string_height(h: &Harness<'_, Moonglow>) -> f32 {
    let top = string_field(h, "Text").rect().top();
    let foot = h.get_all_by_label("Length (s)").map(|n| n.rect().bottom()).fold(0.0, f32::max);
    foot - top
}

/// String is never taller where wider, and comes to rest: the Right Dock
/// swept from 270 to 480 a point at a time beside a line of several long
/// lines, its height never growing with its width; around each width where
/// its layout changes, settled at window heights a few points apart.
#[test]
fn the_string_panel_is_never_taller_where_wider_and_comes_to_rest() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-sweep", 1600.0);
    open_own(&mut h);
    let long = (1..=12)
        .map(|n| format!("Line {n} of a long text that wraps in the panel, as a book's page does."))
        .collect::<Vec<_>>()
        .join("\n");
    let mut l = line(&h, 9);
    l.text = long;
    h.state_mut().talk.as_mut().unwrap().set_line(9, &l, None).unwrap();
    for row in [9, 4, 6] {
        choose(&mut h, Which::Module, row);
        sweep_dock_heights(&mut h, &row.to_string(), string_height, true);
    }
}

/// The window comes to rest with each of String, References and Talk Table
/// in front, behind and folded, the docks closed, docked and floating:
/// each case types in String, moves the selection with the keys, runs a
/// Find (looking and done), References (looking and done), Undo, and
/// removes the last line with it chosen (the header's path); each settles
/// (an endless redraw fails as "keeps repainting"), the last shows the
/// line gone.
#[test]
fn the_window_comes_to_rest_with_each_talk_panel() {
    let cases: [(Mode, &str, f32); 7] = [
        (Mode::Tabs, "String", 1600.0),
        (Mode::Tabs, "References", 1300.0),
        (Mode::Tabs, "Talk Table", 1300.0),
        (Mode::Tabs, "folded", 1600.0),
        (Mode::Tabs, "closed", 1600.0),
        (Mode::Floating, "pane", 1920.0),
        (Mode::Floating, "small", 1280.0),
    ];
    for (mode, case, width) in cases {
        let mut h = talk_window_with(mode, "ui-talk-rest", width, long_dialog());
        open_own(&mut h);
        let what = format!("{mode:?} {case}");
        match case {
            "References" | "Talk Table" => {
                let tab = h
                    .get_all_by_label(case)
                    .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab);
                tab.unwrap_or_else(|| panic!("{what}: its tab")).click();
                settle(&mut h);
            }
            "folded" => {
                h.state_mut().settings.hide_palettes = true;
                settle(&mut h);
            }
            "closed" => {
                h.state_mut().settings.hide_tree = true;
                settle(&mut h);
            }
            _ => {}
        }
        choose(&mut h, Which::Module, 1);
        if string_drawn(&h) {
            type_in(&mut h, "Text", "q");
            assert!(line(&h, 1).text.ends_with('q'), "{what}: typed");
        }
        // The keys.
        h.get_by_label("16777218").click();
        settle(&mut h);
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
        assert_eq!(chosen(&h), Some(3), "{what}: Down");
        // Find, looking and done.
        type_into_hint(&mut h, "words or a StrRef", "watch");
        settle(&mut h);
        assert!(h.query_by_label("16777217").is_none(), "{what}: found");
        h.state_mut().talk_view.filter.clear();
        settle(&mut h);
        // References.
        choose(&mut h, Which::Module, 2);
        talk_click(&mut h, talk_view::FIND_REFERENCES);
        tool(&h, "Undo").click();
        settle(&mut h);
        // A Find through 150,040 lines of the game's: looking over many
        // frames (under way), then done, the window at rest.
        h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
        settle(&mut h);
        h.state_mut().game_talk.as_mut().unwrap().view.filter = "needle".into();
        h.step();
        h.step();
        let looking = h.state().game_talk.as_ref().unwrap().view.sweeping();
        assert!(looking, "{what}: Find still looking after two frames");
        settle(&mut h);
        h.get_by_label("150039");
        h.state_mut().game_talk.as_mut().unwrap().view.filter.clear();
        // (The tab back on the module's table: More › Module's Table.)
        show(&mut h, Which::Module);
        // The last line removed with it chosen.
        choose(&mut h, Which::Module, 29);
        let len = h.state().talk.as_ref().unwrap().len();
        assert_eq!(len, 30, "{what}: before");
        talk_click(&mut h, talk_view::REMOVE_LAST_LINE);
        assert_eq!(h.state().talk.as_ref().unwrap().len(), 29, "{what}: removed");
        assert_eq!(chosen(&h), None, "{what}: nothing chosen");
    }
}

/// Whether the String panel's Text is drawn now.
fn string_drawn(h: &Harness<'_, Moonglow>) -> bool {
    use egui::accesskit::Role;
    h.query_all_by_label("Text").any(|n| n.accesskit_node().role() == Role::MultilineTextInput)
}

/// The Status Bar beside a talk table (the Talk board's): its lines ("30
/// lines"; the game's "40 lines, read-only"), and the list's keys while it
/// has them: Up / Down Choose line, Page Up / Page Down By ten, Enter To
/// its text, Double Click To its text (always), Esc Leave list, each doing
/// what it says; typing in String, Double Click alone; a menu open, none.
/// The game's table has the same: none says what it cannot do.
#[test]
fn a_talk_table_s_hints_are_as_bound() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-hints", 1920.0);
    open_own(&mut h);
    let all = [
        "Up or Down: Choose line",
        "Page Up or Page Down: By ten",
        "Enter: To its text",
        "Double Click: To its text",
        "Esc: Leave list",
    ];
    choose(&mut h, Which::Module, 0);
    says(&mut h, &all, &[]);
    let mut said = all.to_vec();
    said.push("30 lines");
    assert_eq!(h.state().status_said(), said, "in the board's order, nothing else");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(1), "Down: Choose line");
    h.key_press(egui::Key::PageDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(11), "Page Down: By ten");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.ctx.egui_wants_keyboard_input(), "Enter: To its text");
    says(&mut h, &["Double Click: To its text"], &["Enter: To its text", "Esc: Leave list"]);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    h.get_by_label("16777227").click();
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert_ne!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Talk", "Esc: Leave list");
    says(&mut h, &["Double Click: To its text"], &["Up or Down: Choose line"]);
    // A menu open: none.
    h.get_by_label(talk_view::TALK_HEADER).get_by_label("More").click();
    settle(&mut h);
    says(&mut h, &["30 lines"], &["Double Click: To its text"]);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // The game's: the same keys, its count, nothing that changes it.
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    choose(&mut h, Which::Game, 3);
    let mut game = all.to_vec();
    game.push("40 lines, read-only");
    says(&mut h, &game, &[]);
    assert_eq!(h.state().status_said(), game, "nothing it cannot do");
}

/// The talk table header's controls, in their order.
const TALK_CONTROLS: [&str; 13] = [
    "Find",
    "Go to",
    "Add Line",
    "Remove Last Line",
    "Save",
    "Find References",
    "Only lines with text",
    "Export",
    "Import CSV…",
    "Import JSON…",
    "Open File…",
    "New File…",
    "Module's Table",
];

/// The header holds every control from 420 to 1600 points, docked and
/// floating (the window narrowed with the screen), the game's table too:
/// each on the row or in More at every width (`header_holds`), Find and
/// Go to on the row at the widest and into More after the rest, Close (or
/// OK and Cancel) in the end slot; at the least, Only lines with text and
/// Open File… worked from More.
#[test]
fn the_talk_header_holds_every_control_from_420_to_1600() {
    use mg_uikit::header::{self, Place};
    let header_name = talk_view::TALK_HEADER;
    for mode in MODES {
        let mut h = talk_window(mode, "ui-talk-header-widths", 1600.0);
        for which in [Which::Game, Which::Module] {
            // (Closed first, and the size its kind's window was left at
            // forgotten: a floating window made anew at the widest.)
            h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::TalkTable));
            h.set_size(egui::vec2(1600.0, 800.0));
            settle(&mut h);
            h.state_mut().settings.window_sizes.clear();
            show(&mut h, which);
            let tab = which;
            let mut widths: Vec<f32> = (21..=80).rev().map(|n| n as f32 * 20.0).collect();
            widths.extend([1399.0, 1400.0, 961.0, 962.0]);
            let mut top = true;
            let end_names: &[&str] = match which {
                Which::Game => &["Close"],
                Which::Module => &["OK", "Cancel"],
            };
            for width in widths {
                h.set_size(egui::vec2(width, 800.0));
                settle(&mut h);
                let what = format!("{mode:?} {tab:?} {width}");
                let rec = header::records(&h.ctx, header_name);
                let rec = rec.into_iter().next().unwrap_or_else(|| panic!("{what}: no header"));
                let names: Vec<&str> = rec.places.iter().map(|(n, _)| n.as_str()).collect();
                let want: Vec<&str> = TALK_CONTROLS
                    .iter()
                    .copied()
                    .filter(|n| *n != "Module's Table" || which == Which::Game)
                    .collect();
                assert_eq!(names, want, "{what}");
                if std::mem::take(&mut top) {
                    for n in ["Find", "Go to", "Add Line"] {
                        let place = rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
                        let shown = matches!(place, Some(Place::Row | Place::Icon));
                        assert!(shown, "{what}: {n} on the row ({place:?})");
                    }
                }
                let end = rec.end.expect("the end slot");
                for n in end_names {
                    let r = h.get_by_label(header_name).get_by_label(n).rect();
                    assert!(end.expand(0.5).contains_rect(r), "{what}: {n} in {end:?}");
                }
                header_holds(&mut h, header_name, &want);
                let place = |n: &str| rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
                for kept in ["Find", "Go to"] {
                    if place(kept) == Some(Place::More) {
                        for n in ["Add Line", "Remove Last Line", "Find References", "Export"] {
                            let gone = place(n) == Some(Place::More);
                            assert!(gone, "{what}: {n} into More before {kept}");
                        }
                    }
                }
            }
            // At the least: Only lines with text from More.
            h.set_size(egui::vec2(420.0, 800.0));
            settle(&mut h);
            let was = |h: &Harness<'_, Moonglow>| match which {
                Which::Module => h.state().talk_view.only_text,
                Which::Game => h.state().game_talk.as_ref().unwrap().view.only_text,
            };
            let before = was(&h);
            talk_click(&mut h, talk_view::ONLY_TEXT);
            assert_ne!(was(&h), before, "{mode:?} {tab:?}: Only lines with text from More");
        }
    }
}

/// Shows `which` in the Talk Table tab, through the window's own ways: the
/// game's `dialog.tlk` as the Resources panel's row shows it, the module's
/// by More › Module's Table; the window at rest.
fn show(h: &mut Harness<'_, Moonglow>, which: Which) {
    match which {
        Which::Game => h.state_mut().actions.push(mg_ui::Action::ShowGameTalk),
        // (As Tools › Talk Table asks for it.)
        Which::Module => h.state_mut().actions.push(mg_ui::Action::ShowModuleTalk),
    }
    settle(h);
    assert_eq!(Which::shown(h.state()), which, "the tab shows it");
}

/// The panels follow the table the Talk Table tab shows (the owner's
/// ruling: one tab): the module's line in String, then the game's
/// `dialog.tlk` shown in the same tab, its line; More › Module's Table, the
/// module's line again, as it was left; beside a 2DA in another group, the
/// table's Cell, and back; floating, in its window.
#[test]
fn the_talk_panels_follow_the_table_in_front() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-follow", 1920.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 5);
    let detail = |h: &Harness<'_, Moonglow>| {
        let shown = |d: &&str| h.query_by_label_contains(d).is_some();
        let both = ["StrRef 16777221, line 5", "StrRef 7, line 7"];
        let found: Vec<&str> = both.into_iter().filter(shown).collect();
        (found.len() == 1).then(|| found[0])
    };
    assert_eq!(detail(&h), Some("StrRef 16777221, line 5"), "the module's");
    show(&mut h, Which::Game);
    choose(&mut h, Which::Game, 7);
    assert_eq!(detail(&h), Some("StrRef 7, line 7"), "the game's");
    h.get_by_label_contains("dialog.tlk");
    show(&mut h, Which::Module);
    assert_eq!(detail(&h), Some("StrRef 16777221, line 5"), "the module's, as it was left");
    // Beside a 2DA in another group: its Cell, then the talk table's again.
    let dir = mg_testkit::scratch_dir("ui-talk-follow-table");
    std::fs::write(dir.join("mine.2da"), "2DA V2.0\n\n   Label\n0  Bread\n").unwrap();
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("mine.2da")).unwrap();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(0, 0);
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::Cell), "the table's Cell");
    assert_eq!(detail(&h), None, "no talk table's");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    settle(&mut h);
    assert_eq!(detail(&h), Some("StrRef 16777221, line 5"), "the module's again");
    // Floating: in its window, the docks' panels its own.
    let mut h = talk_window(Mode::Floating, "ui-talk-follow", 1920.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 5);
    h.get_by_label_contains("StrRef 16777221, line 5");
    assert!(h.state().right_dock_shows() != Some(Panel::String), "not the main window's dock");
    show(&mut h, Which::Game);
    choose(&mut h, Which::Game, 7);
    h.get_by_label_contains("StrRef 7, line 7");
}

/// Records from before the String panel read without a fault: settings
/// that keep the old splitter's shares (`talk_splits`), an open-tabs view
/// of the talk table with its line (and the splitter's height and the old
/// tool rows, never kept, among its fields): the line chosen again, String
/// in front.
#[test]
fn records_naming_the_old_splitter_read_without_error() {
    let dir = mg_testkit::scratch_dir("ui-talk-old-records");
    let text = "(talk_splits:[(\"mg_mine\",450),(\"other+f\",700)],name_resrefs:true)";
    let (settings, _unread) = mg_ui::Settings::read_fields(text).expect("read");
    assert!(settings.name_resrefs, "the rest read");
    let mut h = talk_window(Mode::Tabs, "ui-talk-old-records", 1600.0);
    let path = h.state().module_path().unwrap();
    let tabs = dir.join("open-tabs");
    std::fs::create_dir_all(&tabs).unwrap();
    let record = r#"{"panes":[{"place":"middle","tabs":["talk-table"]}],
        "front":"talk-table",
        "views":{"talk-table":{"line":3,"split":450,"rows":"two"}}}"#;
    std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
    let talk = h.state_mut().talk.take();
    drop(h);
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.open_tabs_dir = Some(tabs);
    app.game = Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), our_dialog())));
    app.open_module(&path);
    app.talk = talk;
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "the tab kept");
    assert_eq!(chosen(&h), Some(3), "its line kept");
    assert_eq!(h.state().right_dock_shows(), Some(Panel::String));
}

/// Find never holds a frame: in a table of 300,000 lines it looks a slice
/// a frame (the window under way meanwhile, the lines found so far
/// listed), what is typed meanwhile begins it again, and once done the
/// list is the lines that have the words.
#[test]
fn find_looks_through_a_long_table_a_slice_a_frame() {
    let mut tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
    for n in 0..300_000 {
        let text = match n % 50_000 {
            49_999 => format!("A needle in the hay {n}"),
            _ => format!("Hay of line {n}"),
        };
        tlk.entries.push(mg_tlk::TlkEntry::text(text.as_str()));
    }
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.game = Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), tlk)));
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    let sweeping =
        |h: &Harness<'_, Moonglow>| h.state().game_talk.as_ref().unwrap().view.sweeping();
    h.state_mut().game_talk.as_mut().unwrap().view.filter = "hay needle".into();
    h.step();
    assert!(sweeping(&h), "looking on after a frame");
    assert!(h.state().under_way().is_some(), "under way");
    // Typed meanwhile: begun again, for the new words.
    h.state_mut().game_talk.as_mut().unwrap().view.filter = "needle 99999".into();
    h.step();
    assert!(sweeping(&h));
    settle(&mut h);
    assert!(!sweeping(&h), "done");
    h.get_by_label("99999");
    assert!(h.query_by_label("49999").is_none(), "only what has the words");
}

/// The list's keys are the active editor's and its table's (U16's grid's
/// rule): the module's table given them; the tab switched to the game's
/// `dialog.tlk` (the owner's ruling: one tab), Down moves neither table's
/// line (the module's is not shown, the game's list was not given them); a
/// 2DA in another group pressed, the keys left with the talk table: Down
/// moves nothing; a popover open over the list: nothing; floating, a press
/// in the main window: nothing; a click on the list gives them back.
#[test]
fn the_list_s_keys_are_the_active_editor_s() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-keys", 1920.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(4), "the list has them");
    // The tab switched to the game's table: the keys left with the
    // module's, which is not shown.
    show(&mut h, Which::Game);
    choose(&mut h, Which::Game, 10);
    talk_view::keys_left_with(&h.ctx, Which::Module);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(4), "not the module's: not shown");
    let game = h.state().game_talk.as_ref().unwrap().view.selected;
    assert_eq!(game, Some(10), "nor the game's: its list was not given them");
    show(&mut h, Which::Module);
    // A 2DA in another group pressed: the talk table's keys do nothing.
    let dir = mg_testkit::scratch_dir("ui-talk-keys-table");
    std::fs::write(dir.join("mine.2da"), "2DA V2.0\n\n   Label\n0  Bread\n").unwrap();
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("mine.2da")).unwrap();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Table(id)));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Table(id)), "the other's");
    talk_view::keys_left_with(&h.ctx, Which::Module);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(4), "not the module's: its editor is not the active one");
    // A popover open over the module's list: nothing.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    settle(&mut h);
    choose(&mut h, Which::Module, 4);
    talk_tool(&mut h, talk_view::IMPORT_CSV);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(4), "a popover open");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // Floating: the table in its window, Module Properties docked; a press
    // in the main window's editor lets go of them.
    let mut h = talk_window(Mode::Floating, "ui-talk-keys", 1920.0);
    open_own(&mut h);
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    choose(&mut h, Which::Module, 3);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(4), "floating: the list has them");
    let pane = |h: &Harness<'_, Moonglow>, tab: &Tab| {
        let at = h.state().dock.find_tab(tab).expect("its tab");
        let path = egui_dock::NodePath { surface: at.surface, node: at.node };
        h.state().dock.leaf(path).unwrap().rect
    };
    let main = pane(&h, &Tab::ModuleProperties);
    let window = pane(&h, &Tab::TalkTable).expand(8.0);
    let corners = [main.left_bottom(), main.right_bottom(), main.left_top(), main.right_top()];
    let spot = corners
        .into_iter()
        .map(|c| c + (main.center() - c).normalized() * 30.0)
        .find(|p| !window.contains(*p))
        .expect("Module Properties in sight beside the window");
    h.hover_at(spot);
    h.step();
    press(&h, spot, true, egui::Modifiers::NONE);
    h.step();
    press(&h, spot, false, egui::Modifiers::NONE);
    settle(&mut h);
    talk_view::keys_left_with(&h.ctx, Which::Module);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(4), "floating, the main window pressed: nothing");
    h.get_by_label("16777220").click();
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), Some(5), "a click gives them back");
}

/// F6 stops at the talk table's panes in order: the list (the module's
/// rows), then the Right Dock's String, whose Text takes the keyboard;
/// Enter in the list with References in front of String (one bar, under
/// 1400) brings String to the front and its Text the keyboard.
#[test]
fn f6_and_enter_reach_the_string_panel() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-f6", 1600.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 2);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Talk");
    h.key_press(egui::Key::F6);
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::String));
    let text = talk_view::panels::text_id(Which::Module, 2);
    assert!(h.ctx.memory(|m| m.has_focus(text)), "F6: String's Text has the keyboard");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F6);
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Talk", "Shift+F6: the list");
    // One bar under 1400, References in front: Enter brings String.
    h.set_size(egui::vec2(1300.0, 900.0));
    settle(&mut h);
    let tab = h
        .get_all_by_label("References")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab);
    tab.expect("References' tab").click();
    settle(&mut h);
    choose(&mut h, Which::Module, 5);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::String), "String brought");
    let text = talk_view::panels::text_id(Which::Module, 5);
    assert!(h.ctx.memory(|m| m.has_focus(text)), "Enter: its Text has the keyboard");
    // The Right Dock folded: Enter unfolds it on String.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    h.state_mut().settings.hide_palettes = true;
    settle(&mut h);
    choose(&mut h, Which::Module, 6);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let text = talk_view::panels::text_id(Which::Module, 6);
    assert!(h.ctx.memory(|m| m.has_focus(text)), "folded: unfolded, its Text the keyboard's");
}

/// A floating editor of panes at its first size (the coordinator's ruling
/// at M4's U17 review): its window clipped to the main window less a
/// margin (24 a side), not to the room between the docks, so that a talk
/// table, a 2DA and a conversation open as wide as they ask where the
/// screen has it (980 + 281, 1040 + 281 + 261, 920 + 261), in a 1280 by
/// 720 screen and a 1600 by 900 one; beside the right pane, the header
/// holds every control (on its row or in More, by the kit's rule); String
/// shows every field (Text, Sound, Length) whole at that size, a 2DA's Cell
/// its Value, a conversation's Data its text. Printed: where the talk
/// header puts each item. (The tip, `c5f3e341`, had no kit header: its two
/// rows of buttons wrapped and never yielded, and its window had no right
/// pane.)
#[test]
fn a_floating_editor_of_panes_opens_at_its_first_size() {
    for (width, height) in [(1280.0, 720.0), (1600.0, 900.0)] {
        let what = format!("{width} by {height}");
        let mut h = talk_window(Mode::Floating, "ui-talk-first-size", width);
        h.set_size(egui::vec2(width, height));
        settle(&mut h);
        // A conversation of ours and a 2DA of ours beside the talk table.
        let dir = mg_testkit::scratch_dir("ui-talk-first-size-files");
        std::fs::write(dir.join("mine.2da"), "2DA V2.0\n\n   Label\n0  Bread\n").unwrap();
        let mut g = mg_module::dialog::new_dialog();
        mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "Well met.");
        let dlg = ResKey::parse("mg_hello", ResType::DLG).unwrap();
        h.state_mut().ws.as_mut().unwrap().module.set_gff(dlg, &g).unwrap();
        let screen = h.ctx.content_rect();
        let pane = |h: &Harness<'_, Moonglow>, tab: &Tab| {
            let at = h.state().dock.find_tab(tab).expect("its tab");
            let path = egui_dock::NodePath { surface: at.surface, node: at.node };
            h.state().dock.leaf(path).unwrap().rect
        };
        // (Each kind's first width, the screen less its margins at most.)
        let fits = |h: &Harness<'_, Moonglow>, tab: &Tab, asked: f32| {
            let w = pane(h, tab).width();
            let room = screen.width() - 48.0;
            let want = asked.min(room);
            assert!(w >= want - 12.0, "{what}: {tab:?} {w} wide, asks {asked}, room {room}");
        };
        open_own(&mut h);
        choose(&mut h, Which::Module, 3);
        fits(&h, &Tab::TalkTable, 980.0 + 281.0);
        let window = pane(&h, &Tab::TalkTable);
        let records = mg_uikit::header::records(&h.ctx, talk_view::TALK_HEADER);
        let rec = records.into_iter().next().expect("its header");
        eprintln!("{what}: the talk header's places {:?}", rec.places);
        assert!(rec.row.right() <= string_field(&h, "Text").rect().left(), "{what}: beside String");
        let held: Vec<&str> =
            TALK_CONTROLS.iter().copied().filter(|n| *n != "Module's Table").collect();
        header_holds(&mut h, talk_view::TALK_HEADER, &held);
        // (Whole in String's own section: above References' header, which
        // stands under it in the window's right pane.)
        let references = h
            .get_all_by_label_contains("References")
            .filter_map(|n| n.accesskit_node().bounding_box().map(|_| n.rect()))
            .filter(|r| window.contains_rect(*r) && r.left() > rec.row.right())
            .map(|r| r.top())
            .fold(f32::MAX, f32::min);
        let string = egui::Rect::from_min_max(window.min, egui::pos2(window.right(), references));
        for name in ["Text", "Sound"] {
            let r = string_field(&h, name).rect();
            assert!(string.contains_rect(r), "{what}: String's {name} whole: {r:?} in {string:?}");
        }
        let length = h
            .get_all_by_label("Length (s)")
            .find(|n| n.accesskit_node().role() != egui::accesskit::Role::Label)
            .expect("Length's field")
            .rect();
        assert!(
            string.contains_rect(length),
            "{what}: String's Length whole: {length:?} in {string:?}"
        );
        h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::TalkTable));
        settle(&mut h);
        // A 2DA's: Cell's Value whole.
        let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("mine.2da")).unwrap();
        settle(&mut h);
        h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(0, 0);
        settle(&mut h);
        let tab = Tab::Table(id);
        fits(&h, &tab, 1040.0 + 281.0 + 261.0);
        let value = cell_value(&h).rect();
        assert!(pane(&h, &tab).contains_rect(value), "{what}: Cell's Value whole");
        h.state_mut().actions.push(mg_ui::Action::CloseTab(tab));
        settle(&mut h);
        // A conversation's: its window as wide as it asks.
        let tab = Tab::Dialog(dlg);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
        settle(&mut h);
        fits(&h, &tab, 920.0 + 261.0);
    }
}

/// With no module open, docked (As Tabs): a talk table file opened on its
/// own (Open File…) has String, References and Talk Table in the Right
/// Dock, References saying it needs a module; the Left Dock holds no panel
/// for it (U16's ruling: only a table's columns are the Outliner's alone),
/// so it is not drawn. The game's `dialog.tlk` opens the tab showing it
/// without a module, Open File… works from it;
/// a module opened and closed beside it leaves its tab as it was.
#[test]
fn a_talk_table_without_a_module_has_its_panels_docked() {
    let dir = mg_testkit::scratch_dir("ui-talk-no-module");
    let file = dir.join("lines.tlk");
    let mut t = Table::create("lines", &dir, mg_core::Language::ENGLISH, false);
    t.source = mg_module::talk::Source::File(file.clone());
    for n in 0..5 {
        t.add_line(&our_line(n)).unwrap();
    }
    t.save().unwrap();
    let mut app = app_in(Mode::Tabs, vec![file.clone()]);
    app.game = Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), our_dialog())));
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    // No module, no tab: dialog.tlk opens the Talk Table tab showing it.
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "the tab, opened");
    assert_eq!(Which::shown(h.state()), Which::Game, "showing dialog.tlk");
    // Open File… from it: the file's table in the tab.
    h.state_mut().actions.push(mg_ui::Action::TalkFile(false));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "opened on its own");
    assert_eq!(Which::shown(h.state()), Which::Module, "the file's table shown");
    assert!(h.state().game_talk.is_none(), "the game's copy let go of");
    choose(&mut h, Which::Module, 3);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::String));
    assert!(shows(&h, "Text", "The watch asks for line 3."), "its line in String");
    h.get_by_label_contains("Open a module: what names the line is found in it");
    assert!(h.state().left_dock_rect().is_none(), "the Left Dock not drawn: no panel for it");
    // The game's table, and a module opened and closed beside it.
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    assert_eq!(Which::shown(h.state()), Which::Game, "dialog.tlk over the file's table");
    choose(&mut h, Which::Game, 7);
    let path = sample_module(&mg_testkit::scratch_dir("ui-talk-no-module-module"));
    h.state_mut().open_module(&path);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    let g = h.state().game_talk.as_ref().map(|g| g.view.selected);
    let open = h.state().dock.find_tab(&Tab::TalkTable).is_some();
    assert!(!open || g == Some(Some(7)), "its tab gone, or on its line: {open} {g:?}");
    assert!(h.state().under_way().is_none(), "nothing left under way");
}

/// Two typing sessions on one line are two steps of Undo (the review's
/// probe): "a" typed into line 3's Text, line 4 clicked, line 3 clicked,
/// "b" typed: one Undo takes back "b" alone. And the field left (Sound
/// clicked) between two bursts in the Text: two steps too.
#[test]
fn two_typing_sessions_on_one_line_are_two_steps() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-bursts", 1920.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    let was = line(&h, 3).text;
    type_in(&mut h, "Text", "a");
    h.get_by_label("16777220").click();
    settle(&mut h);
    h.get_by_label("16777219").click();
    settle(&mut h);
    type_in(&mut h, "Text", "b");
    assert_eq!(line(&h, 3).text, format!("{was}ab"));
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(line(&h, 3).text, format!("{was}a"), "one Undo: \"b\" alone");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(line(&h, 3).text, was, "then \"a\"");
    // The field left between two bursts: two steps.
    type_in(&mut h, "Text", "c");
    string_field(&h, "Sound").click();
    settle(&mut h);
    type_in(&mut h, "Text", "d");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(line(&h, 3).text, format!("{was}c"), "the field left: \"d\" alone");
}

/// A hak's own resource named `dialog.tlk` is a resource of the load order
/// as any other (the game reads its talk table from the install's `lang`
/// folder, never from a hak): the Resources panel lists both, the hak's
/// opening as its resources do (read-only, its bytes), the game's in its
/// own tab; Quick Open's "dialog.tlk" opens the game's.
#[test]
fn a_hak_s_dialog_tlk_is_a_resource_of_its_own() {
    use mg_resman::{LayerClass, MemContainer, ResMan};
    let dir = mg_testkit::scratch_dir("ui-talk-hak-dialog");
    let path = sample_module(&dir);
    let mut c = MemContainer::new();
    c.insert(talk_dialog_key(), our_dialog().to_bytes().unwrap());
    let mut rm = ResMan::new();
    rm.add(5, "hak:mine", LayerClass::Erf, c);
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    // (Put after the module is opened, which reads the load order anew.)
    app.game = Some(Arc::new(mg_rules::GameData::new(rm, our_dialog())));
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    h.get_by_label("Resources").click();
    settle(&mut h);
    type_into_hint(&mut h, "Name", "dialog");
    let rows: Vec<String> = h
        .query_all_by_label_contains("dialog.tlk")
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    assert!(rows.len() >= 2, "both listed: {rows:?}");
    // Quick Open: the game's.
    h.get_by_label(mg_ui::quick_open::NAME).click();
    settle(&mut h);
    h.event(egui::Event::Text("dialog.tlk".into()));
    settle(&mut h);
    let hits = h.get_by_label(mg_ui::quick_open::RESULTS);
    // (The game's first, the hak's after it among the load order's.)
    let found: Vec<_> = hits.query_all_by_label_contains("dialog.tlk").collect();
    assert!(!found.is_empty(), "the game's dialog.tlk among the hits");
    found[0].click();
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "the Talk Table tab");
    assert!(h.state().game_talk.is_some(), "showing the game's");
}

/// `dialog.tlk`'s key.
fn talk_dialog_key() -> ResKey {
    ResKey::parse("dialog", ResType::TLK).unwrap()
}

/// With `dialog.tlk` in front, what changes nothing of it: the Top Bar's
/// Undo is the module's (the table has no steps: the module's last step is
/// taken back, as with a read-only 2DA in front), a paste over its list
/// changes nothing; a tab kept from a session with no game data shows why
/// it is empty, and comes back with the game data.
#[test]
fn dialog_tlk_in_front_changes_nothing() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-dialog-front", 1600.0);
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A tag",
        vec![mg_edit::Edit::SetField {
            key: info,
            path: mg_edit::GffPath::root(),
            label: "Mod_Tag".into(),
            value: Some(mg_gff::Value::String(b"OTHER".to_vec())),
        }],
    )));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    choose(&mut h, Which::Game, 7);
    h.event(egui::Event::Paste("Pasted".into()));
    settle(&mut h);
    let g = h.state().game_talk.as_ref().unwrap();
    assert_eq!(g.table.line(7).text, "Well met, traveler.", "a paste changes nothing");
    let tag = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&info).unwrap().root.read(&mg_schema::ifo::MOD_TAG).0
    };
    assert_eq!(tag(&mut h), b"OTHER".to_vec());
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(tag(&mut h), b"SAMPLE".to_vec(), "the module's step taken back");
    // No game data: the tab says why it is empty.
    h.state_mut().game = None;
    settle(&mut h);
    h.get_by_label_contains("dialog.tlk is the game data's");
    // (The tab still on dialog.tlk, saying why it is empty: its lines not
    // listed, the module's table not shown in its place.)
    assert_eq!(Which::shown(h.state()), Which::Game, "still showing dialog.tlk");
    assert!(h.query_by_label("7").is_none(), "no line listed without game data");
    h.state_mut().game =
        Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), our_dialog())));
    settle(&mut h);
    h.get_by_label("7");
}

/// Export writes what the editor before M4's U17 wrote (the same
/// `Table::to_csv` and `to_json`, through the same path): a table of ours
/// of two lines, byte for byte against the text the old editor wrote.
#[test]
fn a_talk_table_s_export_is_as_before() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-export", 1600.0);
    open_own(&mut h);
    let dir = mg_testkit::scratch_dir("ui-talk-export-files");
    let (csv, json) = (dir.join("lines.csv"), dir.join("lines.json"));
    h.state_mut().dialogs =
        Box::new(NoDialogs { save: vec![json.clone(), csv.clone()], ..Default::default() });
    talk_export(&mut h, "Export CSV…");
    talk_export(&mut h, "Export JSON…");
    let t = h.state().talk.as_ref().unwrap();
    assert_eq!(std::fs::read_to_string(&csv).unwrap(), t.to_csv());
    assert_eq!(std::fs::read(&json).unwrap(), t.to_json());
    let csv = std::fs::read_to_string(&csv).unwrap();
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some("StrRef,Text,Sound,SoundLength"));
    assert_eq!(lines.next(), Some("16777216,The watch asks for line 0.,vs_watch_000,2.5"));
    assert_eq!(lines.next(), Some("16777217,Line 1 of the harbor.,,"));
}

/// The game's `dialog.tlk` is shown in the one Talk Table tab (the owner's
/// ruling) and the module's table waits behind it: an unsaved line typed
/// in the module's table, its step, its chosen line and its list's place;
/// `dialog.tlk` shown as Quick Open shows it (a second time nothing
/// more), nothing typed reaching the
/// module's table nor changing the game's (String read-only, a burst ended
/// as the tab left the module's); More › Module's Table: the module's table
/// as it was left (the line, the step one Undo takes back, the chosen line),
/// and the game's copy let go of (the `Arc` held by the test alone). Then,
/// another tab in front, `dialog.tlk` from the Resources panel's row: the
/// tab brought forward, showing it.
#[test]
fn dialog_tlk_shows_in_the_talk_table_tab_and_the_module_s_table_waits() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab", 1600.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    let was = line(&h, 3).text;
    type_in(&mut h, "Text", "a");
    assert!(h.state().talk.as_ref().unwrap().is_dirty(), "unsaved");
    // dialog.tlk shown as Quick Open shows it, Text still focused: the same
    // tab, the burst of typing ended as it leaves the module's table.
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "the Talk Table tab");
    assert_eq!(Which::shown(h.state()), Which::Game, "showing dialog.tlk");
    let held = h.state().game_talk.as_ref().unwrap().table.clone();
    // A second open: nothing more (the same copy, one tab).
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    let tabs = h.state().dock.iter_all_tabs().filter(|(_, t)| **t == Tab::TalkTable).count();
    assert_eq!(tabs, 1, "one tab");
    let same = Arc::ptr_eq(&held, &h.state().game_talk.as_ref().unwrap().table);
    assert!(same, "the same copy");
    // Typed while it is shown: nothing reaches either table.
    choose(&mut h, Which::Game, 7);
    string_field(&h, "Text").click();
    settle(&mut h);
    h.event(egui::Event::Text("X".into()));
    settle(&mut h);
    assert_eq!(held.line(7).text, "Well met, traveler.", "the game's as it was");
    assert_eq!(line(&h, 3).text, format!("{was}a"), "the module's as it was left");
    // Module's Table: the module's table as it was left.
    talk_click(&mut h, talk_view::MODULE_S_TABLE);
    assert_eq!(Which::shown(h.state()), Which::Module);
    assert_eq!(chosen(&h), Some(3), "its chosen line");
    assert!(shows(&h, "Text", &format!("{was}a")), "its line in String");
    assert!(h.state().talk.as_ref().unwrap().is_dirty(), "still unsaved");
    assert_eq!(Arc::strong_count(&held), 1, "the game's copy let go of");
    // Typed again: a step of its own (the burst ended as the tab left).
    type_in(&mut h, "Text", "b");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(line(&h, 3).text, format!("{was}a"), "one Undo: \"b\" alone");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(line(&h, 3).text, was, "then the step from before");
    // Another tab in front, dialog.tlk from the Resources panel's row: the
    // tab comes forward, showing it again.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::ModuleProperties));
    h.get_by_label("Resources").click();
    settle(&mut h);
    type_into_hint(&mut h, "Name", "dialog");
    h.get_by_label_contains("dialog.tlk").click();
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "brought forward");
    assert_eq!(Which::shown(h.state()), Which::Game, "showing dialog.tlk");
    let tabs = h.state().dock.iter_all_tabs().filter(|(_, t)| **t == Tab::TalkTable).count();
    assert_eq!(tabs, 1, "one tab");
}

/// Cancel after a round trip (the module's table, `dialog.tlk`, the
/// module's table again): one span, as if the tab never left; Don't Save
/// leaves the file as it was, both edits gone. And the tab closed while it
/// shows `dialog.tlk` with unsaved lines behind it (only looked at: M5's
/// W5, nothing asked) closes, the unsaved lines kept for the module's
/// Save, the game's copy let go of; closed by its × on the module's own
/// table, it asks first (the owner's ruling of 2026-11-21).
#[test]
fn cancel_and_close_across_dialog_tlk() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-cancel", 1600.0);
    open_own(&mut h);
    let file = h.state().talk.as_ref().unwrap().source.file().unwrap().to_path_buf();
    let before = std::fs::read(&file).unwrap();
    choose(&mut h, Which::Module, 3);
    type_in(&mut h, "Text", "a");
    show(&mut h, Which::Game);
    show(&mut h, Which::Module);
    choose(&mut h, Which::Module, 5);
    type_in(&mut h, "Text", "b");
    h.get_by_label(talk_view::TALK_HEADER).get_by_label("Cancel").click();
    settle(&mut h);
    h.get_by_label("Don't Save").click();
    settle(&mut h);
    assert_eq!(std::fs::read(&file).unwrap(), before, "the file as it was");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "closed");
    // Both edits gone from the table itself, not only from its file: let
    // go of, read again from its file when next shown.
    let kept = h.state().talk.as_ref().is_some_and(|t| t.is_dirty());
    assert!(!kept, "nothing unsaved kept in the window");
    // The close: the tab showing dialog.tlk, an unsaved line behind it.
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-close", 1600.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    type_in(&mut h, "Text", "c");
    show(&mut h, Which::Game);
    let held = h.state().game_talk.as_ref().unwrap().table.clone();
    h.get_by_label(talk_view::TALK_HEADER).get_by_label("Close").click();
    settle(&mut h);
    assert_eq!(talk_asked(&h), 1, "asked with dialog.tlk shown, as its × asks");
    answer(&mut h, "Don't Save");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "closed");
    let kept = h.state().talk.as_ref().is_some_and(|t| t.is_dirty());
    assert!(!kept, "the line behind it taken back");
    assert_eq!(Arc::strong_count(&held), 1, "the game's copy let go of");
    // The module's own table closed by its ×: asked first.
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-close-own", 1600.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    type_in(&mut h, "Text", "c");
    click_tab_close(&mut h, &Tab::TalkTable);
    settle(&mut h);
    assert_eq!(talk_asked(&h), 1, "asked");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_some(), "open while asked");
    answer(&mut h, "Don't Save");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "closed");
    let kept = h.state().talk.as_ref().is_some_and(|t| t.is_dirty());
    assert!(!kept, "its line taken back");
}

/// With `dialog.tlk` shown, the module's table waiting behind it takes
/// nothing: the Top Bar's Undo and Redo (Undo last worked in the module's
/// table, `dialog.tlk` shown as Quick Open shows it), Ctrl+Z over the list
/// and over the Top Bar, Ctrl+Y, a paste over it, a press on the tab's own title. Back on the
/// module's table, its line and its step are as left: one Undo takes the
/// line back.
#[test]
fn the_waiting_module_table_takes_no_undo_nor_paste() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-undo", 1600.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    let was = line(&h, 3).text;
    type_in(&mut h, "Text", "a");
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    assert_eq!(Which::shown(h.state()), Which::Game);
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(line(&h, 3).text, format!("{was}a"), "the Top Bar's Undo: not the module's table");
    tool(&h, "Redo").click();
    settle(&mut h);
    let title = h.state().tab_rect(&Tab::TalkTable).unwrap();
    h.hover_at(title.center());
    h.step();
    press(&h, title.center(), true, egui::Modifiers::NONE);
    h.step();
    press(&h, title.center(), false, egui::Modifiers::NONE);
    settle(&mut h);
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(line(&h, 3).text, format!("{was}a"), "after a press on its title");
    choose(&mut h, Which::Game, 7);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(line(&h, 3).text, format!("{was}a"), "Ctrl+Z over the list");
    // (Ctrl+Z with the pointer on the Top Bar: Undo is the one last worked
    // in, which was the module's table.)
    let bar = tool(&h, "Undo").rect().center();
    h.hover_at(bar);
    h.step();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(line(&h, 3).text, format!("{was}a"), "Ctrl+Z over the Top Bar");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    settle(&mut h);
    h.event(egui::Event::Paste("Pasted".into()));
    settle(&mut h);
    assert_eq!(line(&h, 3).text, format!("{was}a"), "the keys over the list");
    assert_eq!(line(&h, 7).text, "Line 7 of the harbor.", "nor a paste");
    talk_click(&mut h, talk_view::MODULE_S_TABLE);
    assert_eq!(Which::shown(h.state()), Which::Module);
    assert!(h.state().talk.as_ref().unwrap().is_dirty(), "still unsaved");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(line(&h, 3).text, was, "its step as left: one Undo");
}

/// Find looking through a long table (a file's, opened by Open File…)
/// when `dialog.tlk` is shown over it: the look is let go of, its words
/// kept, and the window comes to rest (the look begins again as the list
/// is drawn, as where its tab is behind another).
/// (The fixture's 300,000 lines, as `find_looks_through_a_long_table_a_slice_a_frame`'s.)
#[test]
fn the_module_s_find_under_way_rests_while_dialog_tlk_is_shown() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-sweep", 1600.0);
    let dir = mg_testkit::scratch_dir("ui-talk-one-tab-sweep-table");
    let mut t = Table::create("mg_long", &dir, mg_core::Language::ENGLISH, false);
    for n in 0..300_000 {
        let text = if n == 299_999 { "A needle".to_string() } else { format!("Hay of line {n}") };
        t.add_line(&Line { text, feminine: None, sound: String::new(), sound_length: 0.0 })
            .unwrap();
    }
    t.save().unwrap();
    // (Opened as Open File… opens it: a module's table of this length is
    // not one the harness's user folder holds.)
    let file = dir.join("mg_long.tlk");
    open_own(&mut h);
    h.state_mut().dialogs = Box::new(mg_ui::NoDialogs { open: vec![file], ..Default::default() });
    h.state_mut().actions.push(mg_ui::Action::TalkFile(false));
    settle(&mut h);
    assert_eq!(h.state().talk.as_ref().unwrap().len(), 300_000);
    h.state_mut().talk_view.filter = "needle".into();
    h.step();
    assert!(h.state().talk_view.sweeping(), "looking on after a frame");
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    assert_eq!(Which::shown(h.state()), Which::Game);
    assert!(h.state().under_way().is_none(), "at rest");
    assert!(!h.state().talk_view.sweeping(), "its look let go of");
    assert_eq!(h.state().talk_view.filter, "needle", "its Find's words kept");
}

/// More › Module's Table with a file's table opened on its own (Open
/// File…) behind `dialog.tlk`: the module's table, as the file's own
/// Module's Table goes to it.
#[test]
fn module_s_table_from_dialog_tlk_over_a_file_s_table() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-file", 1600.0);
    open_own(&mut h);
    let dir = mg_testkit::scratch_dir("ui-talk-one-tab-file-table");
    let file = dir.join("lines.tlk");
    let mut t = Table::create("lines", &dir, mg_core::Language::ENGLISH, false);
    t.source = mg_module::talk::Source::File(file.clone());
    let text = "A file's line.".to_string();
    t.add_line(&Line { text, feminine: None, sound: String::new(), sound_length: 0.0 }).unwrap();
    t.save().unwrap();
    h.state_mut().dialogs = Box::new(mg_ui::NoDialogs { open: vec![file], ..Default::default() });
    h.state_mut().actions.push(mg_ui::Action::TalkFile(false));
    settle(&mut h);
    assert!(h.state().talk_view.outside, "the file's table");
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    talk_click(&mut h, talk_view::MODULE_S_TABLE);
    assert_eq!(Which::shown(h.state()), Which::Module);
    // (The file's table let go of: the module's is read again where the
    // module names it, which this harness's user folder does not hold.)
    let file_s = h.state().talk.as_ref().is_some_and(|t| t.name == "lines");
    assert!(!file_s, "not the file's table");
    assert!(!h.state().talk_view.outside, "the module's");
}

/// A record of U17's first tab for `dialog.tlk` (`game-talk-table`, written
/// by 27ce7852 to 92ede11d) reads without a fault and opens the Talk Table
/// tab showing `dialog.tlk`.
#[test]
fn a_record_of_the_game_talk_table_tab_opens_the_talk_table_tab() {
    let dir = mg_testkit::scratch_dir("ui-talk-old-game-tab");
    let h = talk_window(Mode::Tabs, "ui-talk-old-game-tab-module", 1600.0);
    let path = h.state().module_path().unwrap();
    drop(h);
    let tabs = dir.join("open-tabs");
    std::fs::create_dir_all(&tabs).unwrap();
    let record = r#"{"panes":[{"place":"middle","tabs":["game-talk-table"]}],
        "front":"game-talk-table","views":{"game-talk-table":{"line":7}}}"#;
    std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.open_tabs_dir = Some(tabs);
    app.game = Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), our_dialog())));
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "the Talk Table tab");
    assert_eq!(Which::shown(h.state()), Which::Game, "showing dialog.tlk");
    // (The line kept under the old kind is not restored: none chosen.)
    let view = &h.state().game_talk.as_ref().unwrap().view;
    assert_eq!(view.selected, None, "the old line not restored");
    h.get_by_label("0");
}

/// A record of U17's sessions with both tabs, the module's table's and
/// `dialog.tlk`'s, in one pane or in two (either in front): one Talk
/// Table tab, showing `dialog.tlk`, in front where either was.
#[test]
fn a_record_of_both_talk_tabs_opens_one_talk_table_tab() {
    let records = [
        r#"{"panes":[{"place":"middle","tabs":["talk-table","game-talk-table"]}],"front":"game-talk-table"}"#,
        r#"{"panes":[{"place":"middle","tabs":["game-talk-table","talk-table"]}],"front":"talk-table"}"#,
        r#"{"panes":[{"place":"middle","tabs":["module-properties","talk-table"]},{"place":"split right 500","tabs":["game-talk-table"]}],"front":"game-talk-table"}"#,
    ];
    for (n, record) in records.into_iter().enumerate() {
        let dir = mg_testkit::scratch_dir(&format!("ui-talk-old-both-{n}"));
        let h = talk_window(Mode::Tabs, &format!("ui-talk-old-both-module-{n}"), 1600.0);
        let path = h.state().module_path().unwrap();
        drop(h);
        let tabs = dir.join("open-tabs");
        std::fs::create_dir_all(&tabs).unwrap();
        std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
        let mut app = app_in(Mode::Tabs, Vec::new());
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(tabs);
        app.game = Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), our_dialog())));
        app.open_module(&path);
        let mut h = harness(app, 1600.0, 900.0);
        settle(&mut h);
        let count = h.state().dock.iter_all_tabs().filter(|(_, t)| **t == Tab::TalkTable).count();
        assert_eq!(count, 1, "{n}: one Talk Table tab");
        assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "{n}: in front");
        assert_eq!(Which::shown(h.state()), Which::Game, "{n}: showing dialog.tlk");
    }
}

/// The window comes to rest across the switch, docked and floating: the
/// module's table, `dialog.tlk` shown, a Find, back to the module's.
#[test]
fn the_window_comes_to_rest_across_the_switch() {
    for mode in MODES {
        let mut h = talk_window(mode, "ui-talk-one-tab-rest", 1920.0);
        open_own(&mut h);
        choose(&mut h, Which::Module, 2);
        show(&mut h, Which::Game);
        choose(&mut h, Which::Game, 7);
        h.state_mut().game_talk.as_mut().unwrap().view.filter = "line".into();
        settle(&mut h);
        show(&mut h, Which::Module);
        assert_eq!(chosen(&h), Some(2), "{mode:?}: its line");
        show(&mut h, Which::Game);
        show(&mut h, Which::Module);
    }
}

/// Undo noted for the module's table on the way to the Top Bar (the
/// pointer gone straight from String's Text to the Top Bar's Undo, then
/// `dialog.tlk` shown): the Top Bar's Undo, Ctrl+Z over the bar and the
/// Edit menu's label are the module's own then, never the hidden table's.
#[test]
fn undo_noted_on_the_way_to_the_bar_never_reaches_the_hidden_table() {
    for by_key in [false, true] {
        let name = format!("ui-talk-one-tab-bar-{by_key}");
        let mut h = talk_window(Mode::Tabs, &name, 1600.0);
        let info = ResKey::parse("module", ResType::IFO).unwrap();
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "A tag",
            vec![mg_edit::Edit::SetField {
                key: info,
                path: mg_edit::GffPath::root(),
                label: "Mod_Tag".into(),
                value: Some(mg_gff::Value::String(b"OTHER".to_vec())),
            }],
        )));
        settle(&mut h);
        open_own(&mut h);
        choose(&mut h, Which::Module, 3);
        let was = line(&h, 3).text;
        type_in(&mut h, "Text", "a");
        let bar = tool(&h, "Undo").rect().center();
        h.hover_at(bar);
        h.step();
        h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
        settle(&mut h);
        assert_eq!(Which::shown(h.state()), Which::Game);
        if by_key {
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        } else {
            // The Edit menu names the module's step.
            h.get_by_label("Edit").click();
            h.run_steps(2);
            h.get_by_label_contains("Undo A tag");
            h.key_press(egui::Key::Escape);
            h.run_steps(2);
            h.hover_at(bar);
            h.step();
            tool(&h, "Undo").click();
        }
        settle(&mut h);
        assert_eq!(line(&h, 3).text, format!("{was}a"), "{by_key}: the hidden line kept");
        let ws = h.state_mut().ws.as_mut().unwrap();
        let tag = ws.doc(&info).unwrap().root.read(&mg_schema::ifo::MOD_TAG).0;
        assert_eq!(tag, b"SAMPLE".to_vec(), "{by_key}: the module's step taken back");
    }
}

/// Every other way to the module's table shows it, `dialog.tlk` let go
/// of where the tab showed it: Tools › Talk Table, Module Properties'
/// Edit…, a 2DA StrRef cell's Talk Table.
#[test]
fn every_way_to_the_module_s_table_shows_it() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-ways", 1600.0);
    open_own(&mut h);
    // Tools › Talk Table.
    show(&mut h, Which::Game);
    h.get_by_label("Tools").click();
    h.run_steps(2);
    let item = h.get_all_by_label("Talk Table").find(|n| {
        let role = n.accesskit_node().role();
        role != egui::accesskit::Role::Tab && role != egui::accesskit::Role::Label
    });
    item.expect("Tools › Talk Table").click();
    settle(&mut h);
    assert_eq!(Which::shown(h.state()), Which::Module, "Tools › Talk Table");
    // Module Properties' Edit… (its Custom Content page).
    show(&mut h, Which::Game);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    h.get_by_label("Custom Content").click();
    settle(&mut h);
    h.get_by_label("Edit…").click();
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "Edit…: the tab");
    assert_eq!(Which::shown(h.state()), Which::Module, "Module Properties' Edit…");
    // A 2DA StrRef cell's Talk Table.
    show(&mut h, Which::Game);
    let dir = mg_testkit::scratch_dir("ui-talk-one-tab-ways-table");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    std::fs::write(dir.join("mine.2da"), "2DA V2.0\n\n   Label  Name\n0  Bread  16777219\n")
        .unwrap();
    std::fs::write(dir.join("mine.2da.toml"), "[columns.Name]\nkind = \"strref\"\n").unwrap();
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("mine.2da")).unwrap();
    settle(&mut h);
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(0, 1);
    settle(&mut h);
    let button = h
        .get_all_by_label("Talk Table")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button);
    button.expect("the cell's Talk Table").click();
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::TalkTable), "the cell's: the tab");
    assert_eq!(Which::shown(h.state()), Which::Module, "a 2DA cell's Talk Table");
}

/// Create in the Talk Table panel's Module's Table, with `dialog.tlk`
/// shown in the tab: the table made is shown in its place.
#[test]
fn a_table_made_with_dialog_tlk_shown_takes_its_place() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-one-tab-create");
    let user = user_folder(&dir, &[]);
    let path = sample_module(&dir);
    let mut app = game_app_with(&root, Some(user.clone()), NoDialogs::default());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    assert_eq!(Which::shown(h.state()), Which::Game);
    let field = h
        .get_all_by_label("New talk table")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::TextInput);
    field.expect("Module's Table's name").scroll_to_me();
    settle(&mut h);
    type_into_hint(&mut h, "name", "mg_made");
    h.get_by_label("Create").scroll_to_me();
    settle(&mut h);
    h.get_by_label("Create").click();
    settle(&mut h);
    assert!(user.join("tlk/mg_made.tlk").is_file(), "made");
    assert_eq!(Which::shown(h.state()), Which::Module, "the made table shown");
    assert_eq!(h.state().talk.as_ref().unwrap().name, "mg_made");
    assert!(h.state().game_talk.is_none(), "the game's copy let go of");
}

/// A line of the module's talk table renamed and saved in the Talk Table
/// tab: the palette's Find finds the blueprint named by it by its new name
/// (the review's C-5: what is kept by the load order kept the old one).
#[test]
fn a_line_renamed_and_saved_is_found_by_its_new_name() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-renamed-found");
    let user = user_folder(&dir, &["tlk"]);
    let line = |text: &str| Line { text: text.into(), ..our_line(1) };
    let mut t = Table::create("mg_mine", &user.join("tlk"), mg_core::Language::ENGLISH, false);
    t.add_line(&line("Grimbold the Old")).unwrap();
    t.save().unwrap();
    let path = sample_module(&dir);
    let mut m = mg_module::Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_mine".to_vec()));
    m.set_info(&info).unwrap();
    // (The game's bandit, named by the module's table's first line.)
    let game = game_data(&root);
    let bandit = game.resman.get_named("nw_bandit001", ResType::UTC).unwrap();
    let mut utc = mg_gff::Gff::read(&bandit).unwrap();
    let named = |n| mg_gff::Value::LocString(LocString::from_strref(mg_core::StrRef(n)));
    utc.root.set("FirstName", named(16_777_216));
    utc.root.set("LastName", named(u32::MAX));
    utc.root.set("Tag", mg_gff::Value::String(b"C5_NAMED".to_vec()));
    utc.root.set("TemplateResRef", mg_gff::Value::ResRef(b"c5_named".to_vec()));
    m.set_gff(ResKey::parse("c5_named", ResType::UTC).unwrap(), &utc).unwrap();
    m.save().unwrap();
    let mut app = game_app(&root, Some(user.clone()));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let file = user.join("tlk/mg_mine.tlk");
    let found = mg_module::talk::Found {
        source: mg_module::talk::Source::File(file.clone()),
        data: std::fs::read(&file).unwrap(),
    };
    app.talk = Some(Table::open("mg_mine", found, None).unwrap());
    app.palette.kind = mg_module::palette::BlueprintKind::Creature;
    (app.palette.tiles, app.palette.custom) = (false, true);
    app.palette.filter = "grimbold".into();
    show_palette(&mut app);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let shown = |h: &Harness<'_, Moonglow>, name: &str| {
        let palette = right_dock_rect(h);
        h.query_all_by_label_contains(name).any(|n| palette.contains(n.rect().center()))
    };
    assert!(shown(&h, "Grimbold the Old"), "found by its name");
    h.state_mut().talk.as_mut().unwrap().set_line(0, &line("Hrothgar the Young"), None).unwrap();
    h.state_mut().actions.push(mg_ui::Action::SaveTalkTable);
    settle(&mut h);
    h.state_mut().palette.filter = "hrothgar".into();
    settle(&mut h);
    assert!(shown(&h, "Hrothgar the Young"), "found by its new name");
}

/// Create looks for the file again as it is clicked: one that landed in
/// the tlk folder since the panel last looked (it looks every two
/// seconds) is not written over (the review's C-4).
#[test]
fn create_writes_over_no_table_that_landed_meanwhile() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-talk-create-landed");
    let user = user_folder(&dir, &["tlk"]);
    let path = sample_module(&dir);
    let mut app = game_app_with(&root, Some(user.clone()), NoDialogs::default());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    let field = h
        .get_all_by_label("New talk table")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::TextInput);
    field.expect("Module's Table's name").scroll_to_me();
    settle(&mut h);
    type_into_hint(&mut h, "name", "mg_landed");
    h.get_by_label("Create").scroll_to_me();
    settle(&mut h);
    // (Theirs, written as the panel still has none there.)
    let theirs = user.join("tlk/mg_landed.tlk");
    std::fs::write(&theirs, b"theirs").unwrap();
    h.get_by_label("Create").click();
    settle(&mut h);
    assert_eq!(std::fs::read(&theirs).unwrap(), b"theirs", "not written over");
    let log = &h.state().log.entries;
    assert!(log.iter().any(|(_, m)| m.contains("is there already")), "said: {log:?}");
    assert!(h.state().talk.as_ref().is_none_or(|t| t.name != "mg_landed"), "not made");
}

/// The way out of `dialog.tlk`, More › Module's Table, with no game data
/// (the tab saying why it is empty) and with no module open.
#[test]
fn module_s_table_leaves_dialog_tlk_without_game_data_or_module() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-no-data", 1600.0);
    open_own(&mut h);
    show(&mut h, Which::Game);
    h.state_mut().game = None;
    settle(&mut h);
    h.get_by_label_contains("dialog.tlk is the game data's");
    talk_click(&mut h, talk_view::MODULE_S_TABLE);
    assert_eq!(Which::shown(h.state()), Which::Module, "no game data: the module's");
    // No module: the tab back on no table of its own, still open.
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.game = Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), our_dialog())));
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::ShowGameTalk);
    settle(&mut h);
    talk_click(&mut h, talk_view::MODULE_S_TABLE);
    assert_eq!(Which::shown(h.state()), Which::Module, "no module");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_some(), "the tab still open");
}

/// F6 and the list's keys while `dialog.tlk` is shown: Shift+F6 from
/// String back to the list gives its keys to the game's list, whose Down
/// moves its line; the module's waiting table's line stays.
#[test]
fn f6_and_the_list_s_keys_go_to_dialog_tlk_s_list() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-f6", 1600.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 2);
    show(&mut h, Which::Game);
    choose(&mut h, Which::Game, 7);
    h.key_press(egui::Key::F6);
    settle(&mut h);
    let text = talk_view::panels::text_id(Which::Game, 7);
    assert!(h.ctx.memory(|m| m.has_focus(text)), "F6: the game's line's Text");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F6);
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    let game = h.state().game_talk.as_ref().unwrap().view.selected;
    assert_eq!(game, Some(8), "Down in the game's list");
    assert_eq!(chosen(&h), Some(2), "the module's line as left");
}

/// The panels read the game's copy as the list does: the game data
/// changed, String shows the new line's text in the very next frame.
#[test]
fn the_panels_follow_changed_game_data_at_once() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-one-tab-refresh", 1600.0);
    show(&mut h, Which::Game);
    choose(&mut h, Which::Game, 7);
    assert!(shows(&h, "Text", "Well met, traveler."));
    let mut tlk = our_dialog();
    tlk.entries[7] = mg_tlk::TlkEntry::text("Well met again.");
    h.state_mut().game = Some(Arc::new(mg_rules::GameData::new(mg_resman::ResMan::new(), tlk)));
    h.step();
    assert!(shows(&h, "Text", "Well met again."), "String in the same frame as the list");
}

/// How a test closes the Talk Table tab (M5's W5).
#[derive(Clone, Copy, Debug)]
enum Closing {
    /// The tab's ×.
    TabX,
    /// The tab menu's Close (`Action::CloseTabs` of the tab alone, as a
    /// middle click asks too).
    MenuClose,
    /// Ctrl+W over the editor.
    CtrlW,
    /// Close Others on another tab: every tab but that one.
    CloseOthers,
    /// Close All: every tab.
    CloseAll,
    /// The × of the floating window the tab is in.
    WindowX,
}

/// Closes the Talk Table tab as `how` does, and lets the window come to
/// rest.
fn close_talk(h: &mut Harness<'_, Moonglow>, how: Closing) {
    let all: Vec<Tab> = h.state().dock.iter_all_tabs().map(|(_, t)| t.clone()).collect();
    match how {
        Closing::TabX => click_tab_close(h, &Tab::TalkTable),
        Closing::MenuClose => {
            h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::TalkTable]))
        }
        Closing::CtrlW => {
            let title = h.state().tab_rect(&Tab::TalkTable).expect("its tab drawn");
            h.hover_at(title.center_bottom() + egui::vec2(0.0, 120.0));
            h.step();
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
        }
        Closing::CloseOthers => {
            let keep = all.iter().find(|t| **t != Tab::TalkTable && t.closeable()).cloned();
            let others = all.into_iter().filter(|t| Some(t) != keep.as_ref()).collect();
            h.state_mut().actions.push(mg_ui::Action::CloseTabs(others));
        }
        Closing::CloseAll => h.state_mut().actions.push(mg_ui::Action::CloseTabs(all)),
        Closing::WindowX => click_window_close(h, &Tab::TalkTable),
    }
    settle(h);
}

/// How many talk table closing questions are on screen (by their words:
/// the module's own question, also "Unsaved Changes", names the module).
fn talk_asked(h: &Harness<'_, Moonglow>) -> usize {
    h.query_all_by_label_contains("Save changes to the talk table").count()
}

/// Presses the button `word` of the question on screen ("Unsaved
/// Changes"), and lets the window come to rest.
fn answer(h: &mut Harness<'_, Moonglow>, word: &str) {
    h.get_by_label("Unsaved Changes").get_by_label(word).click();
    settle(h);
}

/// How many "Save changes to …?" questions of any kind are on screen.
fn any_asked(h: &Harness<'_, Moonglow>) -> usize {
    h.query_all_by_label_contains("Save changes to").count()
}

/// Closing the Talk Table tab with a line changed in its editor and not
/// saved asks Save, Don't Save or Cancel (the owner's ruling of
/// 2026-11-21), once, by every way the tab closes: its ×, the tab menu's
/// Close, Ctrl+W, Close Others, Close All, the × of its floating window.
/// Escape is Cancel: the tab stays with its line; asked again, Don't Save
/// closes it, the file as it was and nothing unsaved kept; Save writes the
/// line to the table's file and closes it. Either way it is noted for
/// Reopen Closed Tab, which brings it back.
#[test]
fn closing_the_talk_table_tab_asks_once_by_every_way() {
    use Closing::*;
    for (n, how) in [TabX, MenuClose, CtrlW, CloseOthers, CloseAll, WindowX].into_iter().enumerate()
    {
        let mode = if matches!(how, WindowX) { Mode::Floating } else { Mode::Tabs };
        let mut h = talk_window(mode, &format!("ui-talk-close-asks-{n}"), 1600.0);
        open_own(&mut h);
        let file = h.state().talk.as_ref().unwrap().source.file().unwrap().to_path_buf();
        let before = std::fs::read(&file).unwrap();
        choose(&mut h, Which::Module, 3);
        let was = line(&h, 3).text;
        type_in(&mut h, "Text", "a");
        close_talk(&mut h, how);
        assert_eq!((talk_asked(&h), any_asked(&h)), (1, 1), "{how:?}: asked once");
        assert!(h.state().dock.find_tab(&Tab::TalkTable).is_some(), "{how:?}: open meanwhile");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert_eq!(any_asked(&h), 0, "{how:?}: Escape answers Cancel");
        assert!(h.state().dock.find_tab(&Tab::TalkTable).is_some(), "{how:?}: Cancel keeps it");
        assert_eq!(line(&h, 3).text, format!("{was}a"), "{how:?}: the line kept");
        // (Close Others and Close All closed the others meanwhile: the
        // tab alone is closed again, as its menu's Close.)
        let again = if matches!(how, CloseOthers | CloseAll) { MenuClose } else { how };
        close_talk(&mut h, again);
        assert_eq!(talk_asked(&h), 1, "{how:?}: asked again");
        let save = n % 2 == 1;
        answer(&mut h, if save { "Save" } else { "Don't Save" });
        settle(&mut h);
        assert_eq!(any_asked(&h), 0, "{how:?}: asked no more");
        assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "{how:?}: closed");
        let dirty = h.state().talk.as_ref().is_some_and(|t| t.is_dirty());
        assert!(!dirty, "{how:?}: nothing unsaved kept");
        let now = std::fs::read(&file).unwrap();
        if save {
            let saved = mg_tlk::Tlk::read(&now).unwrap();
            assert_eq!(
                saved.entries[3].text,
                format!("{was}a").into_bytes(),
                "{how:?}: Save wrote the line"
            );
        } else {
            assert_eq!(now, before, "{how:?}: Don't Save left the file as it was");
        }
        // (Reopen Closed Tab's keys.)
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::T);
        settle(&mut h);
        assert!(h.state().dock.find_tab(&Tab::TalkTable).is_some(), "{how:?}: reopened");
        assert_eq!(any_asked(&h), 0);
    }
}

/// Closing the Talk Table tab asks nothing where nothing of its own editor
/// is unsaved: a clean table; lines
/// typed for the module's fields in other editors alone (a blueprint's
/// name: the module's, which its Undo takes back), by its × or its
/// Cancel. Where the tab's own
/// line and a typed one are both unsaved, the question says that Don't
/// Save keeps the typed one, and it does.
/// The owner's ruling 35 has no exception: with the game's `dialog.tlk`
/// shown in the tab over the module's table with a line not saved,
/// closing the tab asks once, naming the table behind; Save saves it and
/// closes, Cancel keeps both.
#[test]
fn the_talk_table_tab_asks_with_dialog_tlk_in_front() {
    for (way, word) in [(Closing::TabX, "Save"), (Closing::CtrlW, "Cancel")] {
        let mut h = talk_window(Mode::Tabs, "ui-talk-close-game", 1600.0);
        open_own(&mut h);
        choose(&mut h, Which::Module, 3);
        type_in(&mut h, "Text", "c");
        show(&mut h, Which::Game);
        let name = h.state().talk.as_ref().unwrap().name.clone();
        close_talk(&mut h, way);
        assert_eq!(talk_asked(&h), 1, "{way:?}: asked once with dialog.tlk shown");
        let named = format!("Save changes to the talk table {name}?");
        assert!(h.query_by_label_contains(&named).is_some(), "{way:?}: names the table behind");
        answer(&mut h, word);
        assert_eq!(any_asked(&h), 0, "{way:?}: answered");
        let open = h.state().dock.find_tab(&Tab::TalkTable).is_some();
        let dirty = h.state().talk.as_ref().is_some_and(|t| t.is_dirty());
        match word {
            "Save" => assert!(!open && !dirty, "{way:?}: saved and closed"),
            _ => assert!(open && dirty, "{way:?}: Cancel keeps it and its line"),
        }
    }
}

#[test]
fn the_talk_table_tab_closes_unasked_where_nothing_of_its_own_is_unsaved() {
    // A clean table.
    let mut h = talk_window(Mode::Tabs, "ui-talk-close-clean", 1600.0);
    open_own(&mut h);
    close_talk(&mut h, Closing::TabX);
    assert_eq!(any_asked(&h), 0, "clean: nothing asked");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "clean: closed");
    // A line typed for a module's field alone.
    let mut h = talk_window(Mode::Tabs, "ui-talk-close-typed", 1600.0);
    open_own(&mut h);
    let typed = Line { text: "Typed in a blueprint.".into(), ..our_line(5) };
    h.state_mut().talk.as_mut().unwrap().put_typed(5, &typed).unwrap();
    settle(&mut h);
    close_talk(&mut h, Closing::TabX);
    assert_eq!(any_asked(&h), 0, "typed alone: nothing asked");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "typed alone: closed");
    assert!(h.state().talk.as_ref().unwrap().has_typed_unsaved(), "kept for the module");
    // (Its Cancel likewise.)
    open_own(&mut h);
    h.get_by_label(talk_view::TALK_HEADER).get_by_label("Cancel").click();
    settle(&mut h);
    assert_eq!(any_asked(&h), 0, "typed alone, Cancel: nothing asked");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "typed alone, Cancel: closed");
    assert!(h.state().talk.as_ref().unwrap().has_typed_unsaved(), "still kept");
    // Both: asked, saying what Don't Save keeps.
    let mut h = talk_window(Mode::Tabs, "ui-talk-close-both", 1600.0);
    open_own(&mut h);
    h.state_mut().talk.as_mut().unwrap().put_typed(5, &typed).unwrap();
    choose(&mut h, Which::Module, 3);
    let was = line(&h, 3).text;
    type_in(&mut h, "Text", "d");
    close_talk(&mut h, Closing::TabX);
    assert_eq!(talk_asked(&h), 1, "both: asked");
    assert!(h.query_by_label(talk_view::CLOSE_TYPED_NOTE).is_some(), "says the typed line stays");
    assert!(h.query_by_label(talk_view::CLOSE_SAVE_NOTE).is_none(), "a file's table: no word");
    answer(&mut h, "Don't Save");
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "closed");
    let t = h.state().talk.as_ref().unwrap();
    assert_eq!(t.line(3).text, was, "its own line taken back");
    assert_eq!(t.line(5).text, "Typed in a blueprint.", "the typed line kept");
}

/// A table kept in the module: the question says that its Save puts the
/// lines into the module, whose Save writes them; Save does so (the module
/// changed, the table clean, the tab closed).
#[test]
fn the_talk_question_says_what_save_does_for_the_module_s_table() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-close-module", 1600.0);
    h.state_mut().talk.as_mut().unwrap().source = mg_module::talk::Source::Module;
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    let was = line(&h, 3).text;
    type_in(&mut h, "Text", "e");
    let modified = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().is_modified();
    assert!(!modified(&h), "the module as saved");
    close_talk(&mut h, Closing::TabX);
    assert_eq!(talk_asked(&h), 1);
    assert!(h.query_by_label(talk_view::CLOSE_SAVE_NOTE).is_some(), "says what Save does");
    assert!(h.query_by_label(talk_view::CLOSE_TYPED_NOTE).is_none(), "nothing typed elsewhere");
    answer(&mut h, "Save");
    assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "closed");
    assert!(modified(&h), "the lines put into the module, for its Save");
    let t = h.state().talk.as_ref().unwrap();
    assert!(!t.is_dirty());
    assert_eq!(t.line(3).text, format!("{was}e"));
}

/// Quitting, File › Close, opening another module and opening the same
/// one again (as File › Recent reloads it) with the Talk Table
/// tab open and a line of its own not saved ask the module's question
/// alone, once: not the tab's as well (the module's Save writes the
/// table, its Don't Save lets it go).
#[test]
fn closing_the_module_asks_of_the_talk_table_once() {
    for (n, action) in ["quit", "close", "open", "reopen"].into_iter().enumerate() {
        let mut h = talk_window(Mode::Tabs, &format!("ui-talk-close-module-{n}"), 1600.0);
        open_own(&mut h);
        choose(&mut h, Which::Module, 3);
        type_in(&mut h, "Text", "f");
        let other = mg_testkit::scratch_dir(&format!("ui-talk-close-other-{n}"));
        let other = sample_module(&other);
        let same = h.state().ws.as_ref().unwrap().module.location.as_ref().unwrap().path().into();
        h.state_mut().actions.push(match action {
            "quit" => mg_ui::Action::CloseWindow,
            "close" => mg_ui::Action::Close,
            "open" => mg_ui::Action::OpenModule(other),
            _ => mg_ui::Action::OpenModule(same),
        });
        settle(&mut h);
        assert_eq!((any_asked(&h), talk_asked(&h)), (1, 0), "{action}: the module's alone");
        answer(&mut h, "Don't Save");
        settle(&mut h);
        assert_eq!(any_asked(&h), 0, "{action}: asked once");
        match action {
            "quit" => assert!(h.state().quit_requested, "quit"),
            "reopen" => {
                let kept = h.state().talk.as_ref().is_some_and(|t| t.is_dirty());
                assert!(!kept, "reopen: the line let go of, the module read again");
            }
            _ => assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "{action}: gone"),
        }
    }
}

/// Close All with the talk table and a tileset both unsaved asks of each
/// in turn, once, the talk table first (its tab's order); Cancel at the
/// first keeps both open and asks nothing more, Don't Save at each closes
/// both.
#[test]
fn close_all_asks_of_each_unsaved_tab_in_turn() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-close-all-two", 1600.0);
    open_own(&mut h);
    choose(&mut h, Which::Module, 3);
    type_in(&mut h, "Text", "f");
    let dir = mg_testkit::scratch_dir(&Mode::Tabs.dir("ui-talk-close-all-two-set"));
    let set = dir.join("zzt01.set");
    let text = "[GENERAL]\r\nName=ZZT01\r\n[TILES]\r\nCount=0\r\n[GROUPS]\r\nCount=0\r\n";
    std::fs::write(&set, text).unwrap();
    mg_ui::tileset_view::open_path(h.state_mut(), set);
    settle(&mut h);
    h.state_mut().tilesets[0].change(|f| f.set("GENERAL", "Transition", "5"));
    settle(&mut h);
    let tileset = Tab::Tileset(h.state().tilesets[0].id);
    let all = || -> mg_ui::Action {
        mg_ui::Action::CloseTabs(vec![Tab::ModuleProperties, Tab::TalkTable, tileset.clone()])
    };
    let open = |h: &Harness<'_, Moonglow>, tab: &Tab| h.state().dock.find_tab(tab).is_some();
    h.state_mut().actions.push(all());
    settle(&mut h);
    assert_eq!((any_asked(&h), talk_asked(&h)), (1, 1), "the talk table first, alone");
    answer(&mut h, "Cancel");
    assert_eq!(any_asked(&h), 0, "Cancel: nothing more asked");
    assert!(open(&h, &Tab::TalkTable) && open(&h, &tileset), "Cancel: both kept");
    assert!(h.state().tilesets[0].is_dirty(), "Cancel: the tileset's change kept");
    h.state_mut().actions.push(all());
    settle(&mut h);
    assert_eq!(talk_asked(&h), 1, "asked again");
    answer(&mut h, "Don't Save");
    assert_eq!((any_asked(&h), talk_asked(&h)), (1, 0), "then the tileset, once");
    assert!(h.query_by_label_contains("Save changes to zzt01").is_some(), "by its name");
    answer(&mut h, "Don't Save");
    assert_eq!(any_asked(&h), 0, "each asked once");
    assert!(!open(&h, &Tab::TalkTable) && !open(&h, &tileset), "both closed");
}

/// A talk table kept in a file (the module's own in `tlk/`, or another
/// opened by Open File…) with a line not saved, as another module is
/// opened: the module's question alone, once; its Save writes the table's
/// file, and the next module opens without it.
#[test]
fn a_file_talk_table_across_a_module_switch() {
    for other_file in [false, true] {
        let name = format!("ui-talk-switch-file-{other_file}");
        let mut h = talk_window(Mode::Tabs, &name, 1600.0);
        open_own(&mut h);
        if other_file {
            let dir = mg_testkit::scratch_dir(&Mode::Tabs.dir(&format!("{name}-lines")));
            let mut t = Table::create("lines", &dir, mg_core::Language::ENGLISH, false);
            for n in 0..10 {
                t.add_line(&our_line(n)).unwrap();
            }
            t.save().unwrap();
            let open = vec![dir.join("lines.tlk")];
            h.state_mut().dialogs = Box::new(mg_ui::NoDialogs { open, ..Default::default() });
            h.state_mut().actions.push(mg_ui::Action::TalkFile(false));
            settle(&mut h);
            assert_eq!(h.state().talk.as_ref().unwrap().name, "lines", "Open File…'s table");
        }
        let file = h.state().talk.as_ref().unwrap().source.file().unwrap().to_path_buf();
        let before = std::fs::read(&file).unwrap();
        choose(&mut h, Which::Module, 3);
        type_in(&mut h, "Text", "f");
        let other = sample_module(&mg_testkit::scratch_dir(&format!("{name}-other")));
        h.state_mut().actions.push(mg_ui::Action::OpenModule(other));
        settle(&mut h);
        assert_eq!((any_asked(&h), talk_asked(&h)), (1, 0), "{other_file}: the module's alone");
        answer(&mut h, "Save");
        assert_eq!(any_asked(&h), 0, "{other_file}: asked once");
        assert_ne!(std::fs::read(&file).unwrap(), before, "{other_file}: its file written");
        assert!(h.state().talk.is_none(), "{other_file}: not carried to the next module");
        assert!(h.state().dock.find_tab(&Tab::TalkTable).is_none(), "{other_file}: its tab gone");
    }
}

/// A talk table opened from its file has its file's time taken as it
/// opens: the first Reload Resources after (a hak's save does one) looks
/// at the time and reads the lines only where it moved. It read the whole
/// table to see, 50,000 lines in a frame (the editors' timing view).
#[test]
fn a_talk_table_opened_is_read_again_only_where_its_file_s_time_moved() {
    let mut h = talk_window(Mode::Tabs, "ui-talk-opened-stamps", 1600.0);
    let file = mg_testkit::scratch_dir("ui-talk-opened-stamps-file").join("mg_other.tlk");
    let write = |text: &str| {
        let mut t =
            Table::create("mg_other", file.parent().unwrap(), mg_core::Language::ENGLISH, false);
        t.source = mg_module::talk::Source::File(file.clone());
        let mut first = our_line(0);
        first.text = text.into();
        t.add_line(&first).unwrap();
        t.save().unwrap();
    };
    write("As opened");
    h.state_mut().dialogs = Box::new(NoDialogs { open: vec![file.clone()], ..Default::default() });
    h.state_mut().actions.push(mg_ui::Action::TalkFile(false));
    settle(&mut h);
    assert_eq!(line(&h, 0).text, "As opened");
    let revision = h.state().talk.as_ref().unwrap().revision();
    // Written over with its time as it was: the time alone is looked at.
    let at = std::fs::metadata(&file).unwrap().modified().unwrap();
    write("Written over");
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(at).unwrap();
    h.state_mut().reload_resources(false);
    settle(&mut h);
    assert_eq!(line(&h, 0).text, "As opened", "not read: its time as it was");
    assert_eq!(h.state().talk.as_ref().unwrap().revision(), revision);
    // Its time moved: read again.
    let later = at + std::time::Duration::from_secs(2);
    std::fs::File::options().write(true).open(&file).unwrap().set_modified(later).unwrap();
    h.state_mut().reload_resources(false);
    settle(&mut h);
    assert_eq!(line(&h, 0).text, "Written over", "read again");
}
