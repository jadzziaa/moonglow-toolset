//! A conversation's panels (M4's U14; the Conversation and Floating
//! boards): the Right Dock's Node section, its title before its Data and
//! Search, with Bookmark and Test on its bar, and the Conversation panel
//! under it, docked, or in a floating conversation's window's right pane;
//! the panels following the conversation in front; a field typed in kept
//! to its own line; a conversation only looked at; every panel fitting the
//! dock from its least width, the editor's larger text too; the window
//! coming to rest; and records from before reading without a fault.

use crate::common::*;
use mg_module::dialog::{Kind, Parent, add_node, new_dialog};
use mg_ui::Panel;

/// A conversation of two greetings, "Hello there." (its reply "Who are
/// you?") and "Go away.".
pub(crate) fn talk() -> mg_gff::Gff {
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    add_node(&mut g, Parent::Root, "Go away.");
    g
}

/// A module with the conversations `names` (each [`talk`]), open in
/// `mode` in a window `width` by 900, their tabs opened in their order
/// (the last in front).
pub(crate) fn dialogs_window(
    mode: Mode,
    name: &str,
    names: &[&str],
    width: f32,
) -> (Harness<'static, Moonglow>, Vec<ResKey>) {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let keys: Vec<ResKey> = names.iter().map(|n| ResKey::parse(n, ResType::DLG).unwrap()).collect();
    for key in &keys {
        m.set_gff(*key, &talk()).unwrap();
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings = mode.settings();
    app.settings.no_last_area = true;
    // (No backup's timer asking for frames: the rest test counts them.)
    app.settings.dialog_no_backup = true;
    app.open_module(&path);
    let mut h = harness(app, width, 900.0);
    settle(&mut h);
    for key in &keys {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(*key)));
        settle(&mut h);
    }
    (h, keys)
}

/// The rectangle `key`'s conversation is drawn in: its pane's (its tab's
/// group, or its window's).
pub(crate) fn leaf_rect(h: &Harness<'_, Moonglow>, key: ResKey) -> egui::Rect {
    let window = h.state().dock.find_tab(&Tab::Dialog(key)).expect("its tab").surface;
    h.state().dock.iter_leaves().find(|(p, _)| p.surface == window).unwrap().1.rect
}

/// A click at `at` (a press and a release), and the frames that follow.
fn click_point(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    h.step();
    press(h, at, true, egui::Modifiers::NONE);
    h.step();
    press(h, at, false, egui::Modifiers::NONE);
    settle(h);
}

/// The name of the tab bar the Node section's tools are on: the Right
/// Dock's, or a floating conversation's window's right pane's.
pub(crate) fn node_bar(mode: Mode) -> &'static str {
    match mode {
        Mode::Tabs => mg_ui::RIGHT_DOCK,
        Mode::Floating => "Window's Right Pane",
    }
}

/// Where the one-line field labeled `label` is drawn: beside its label,
/// or under it where the row puts the label over it.
fn field_of(h: &Harness<'_, Moonglow>, label: &str) -> egui::Rect {
    let label = at(h, label);
    let y = label.center().y;
    let fields = h.get_all_by_role(egui::accesskit::Role::TextInput).map(|n| n.rect());
    // (In its row: at the label's left edge, or past its column.)
    let x = label.left() - 4.0..=label.left() + 200.0;
    let near = fields
        .filter(|r| r.center().y >= y - 8.0 && r.center().y <= y + 40.0 && x.contains(&r.left()));
    near.min_by(|a, b| (a.center().y - y).abs().total_cmp(&(b.center().y - y).abs()))
        .unwrap_or_else(|| panic!("{label:?}'s field"))
}

/// Where a control named `name` is drawn, the first of the name.
pub(crate) fn at(h: &Harness<'_, Moonglow>, name: &str) -> egui::Rect {
    h.get_all_by_label(name).next().unwrap_or_else(|| panic!("{name}")).rect()
}

/// The Node section's tab `name` (Data, Search) or the Conversation
/// panel's tab (where the dock folds them into one bar) clicked.
pub(crate) fn tab(h: &mut Harness<'_, Moonglow>, name: &str) {
    let found =
        h.get_all_by_label(name).find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab);
    found.unwrap_or_else(|| panic!("the tab {name}")).click();
    settle(h);
}

/// Docked, the conversation's Data, Search and Conversation are the Right
/// Dock's, its title "Node" before the first two's tabs and Bookmark and
/// Test on their bar, Conversation under them with its word count;
/// floating, they are its window's right pane's, the main window's Right
/// Dock not drawn for it. Find (Ctrl+F) brings Search to the front in
/// either; F6 to the dock gives Data's first field the keyboard.
#[test]
fn the_line_s_fields_are_the_right_dock_s_or_the_window_s() {
    for mode in MODES {
        let (mut h, keys) = dialogs_window(mode, "ui-dialog-panels", &["talk"], 1600.0);
        let key = keys[0];
        h.get_by_label("[OWNER] - Hello there.").click();
        settle(&mut h);
        let room = match mode {
            Mode::Tabs => {
                assert_eq!(h.state().right_dock_shows(), Some(Panel::NodeData), "{mode:?}");
                right_dock_rect(&h)
            }
            Mode::Floating => {
                assert_eq!(h.state().right_dock_shows(), None, "no docked conversation");
                leaf_rect(&h, key)
            }
        };
        for name in ["Node", "Data", "Search", "Bookmark"] {
            assert!(room.contains_rect(at(&h, name)), "{mode:?}: {name} in {room:?}");
        }
        // (Test on the Node section's bar, by its group: the header's has
        // the same name.)
        let bar = node_bar(mode);
        assert!(room.contains_rect(h.get_by_label(bar).get_by_label("Test").rect()));
        // (Its header, named with its detail: a group, drawn nowhere of its
        // own.)
        h.get_by_label_contains("words in the file");
        for name in ["Speaker tag", "Text Appears When…", "Normal end", "Stop camera zoom in"] {
            assert!(room.contains(at(&h, name).center()), "{mode:?}: {name}");
        }
        // Bookmark and Test on the Node section's bar.
        h.get_by_label("Bookmark").click();
        settle(&mut h);
        assert_eq!(h.state().dialog_views[&key].bookmarks, [(Kind::Entry, 0)], "{mode:?}");
        h.get_by_label(bar).get_by_label("Test").click();
        settle(&mut h);
        assert!(h.state().dialog_views[&key].test.is_some(), "{mode:?}");
        h.state_mut().dialog_views.get_mut(&key).unwrap().test = None;
        settle(&mut h);
        // Ctrl+F over the lines: Search in front, its field the keyboard's.
        assert!(h.query_by_label("Find What").is_none());
        h.hover_at(at(&h, "[OWNER] - Go away.").center());
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
        settle(&mut h);
        assert!(room.contains(at(&h, "Find What").center()), "{mode:?}: Search in front");
        if mode == Mode::Tabs {
            assert_eq!(h.state().right_dock_shows(), Some(Panel::NodeSearch));
            tab(&mut h, "Data");
            // F6 from the lines to the Right Dock: Data's first field
            // (the speaker's tag) takes the keyboard.
            h.get_by_label("[OWNER] - Hello there.").click();
            settle(&mut h);
            assert!(!h.ctx.text_edit_focused());
            for _ in 0..6 {
                if mg_ui::palette_view::arrow_keys_are(&h.ctx) == "NodeData" {
                    break;
                }
                h.key_press(egui::Key::F6);
                settle(&mut h);
            }
            assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "NodeData");
            assert!(h.ctx.text_edit_focused(), "the speaker's tag has the keyboard");
        }
    }
}

/// The panels follow the conversation in front: of two in two groups, the
/// group with the focus; a floating one's are in its window, the Right
/// Dock left as it was. A line chosen in one shows in its panel, never in
/// the other's.
#[test]
fn the_panels_follow_the_conversation_in_front() {
    let (mut h, keys) =
        dialogs_window(Mode::Tabs, "ui-dialog-follow", &["aaa", "bbb", "ccc"], 1600.0);
    // ccc floats; bbb in a second group beside aaa.
    // (Wide enough for its Outliner's pane beside the tree's room.)
    own_window(&mut h, &Tab::Dialog(keys[2]), egui::pos2(20.0, 150.0), egui::vec2(1250.0, 600.0));
    click_tab(&mut h, &Tab::Dialog(keys[1]));
    settle(&mut h);
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    // Each its own line chosen: aaa's first greeting, bbb's second, ccc's
    // reply.
    let rows = [
        (Parent::Root, 0, "Speaker tag"),
        (Parent::Root, 1, "Speaker tag"),
        (Parent::Node(Kind::Entry, 0), 0, "Who are you?"),
    ];
    for (key, (parent, pos, _)) in keys.iter().zip(rows) {
        let row = mg_ui::dialog_view::Row { parent, pos };
        h.state_mut().dialog_views.get_mut(key).unwrap().selected = Some(row);
    }
    // (And each its own bookmark, for the Outliner: M4's U15.)
    let marks = [(Kind::Entry, 0), (Kind::Entry, 1), (Kind::Reply, 0)];
    for (key, mark) in keys.iter().zip(marks) {
        h.state_mut().dialog_views.get_mut(key).unwrap().bookmarks = vec![mark];
    }
    settle(&mut h);
    let main = h.ctx.content_rect();
    let dock = |h: &Harness<'_, Moonglow>| right_dock_rect(h);
    let shows = |h: &Harness<'_, Moonglow>, room: egui::Rect, value: &str| {
        let fields = h.get_all_by_role(egui::accesskit::Role::MultilineTextInput);
        fields
            .filter(|n| room.contains(n.rect().center()))
            .any(|n| n.accesskit_node().value().as_deref() == Some(value))
    };
    for (front, own, other) in
        [(keys[0], "Hello there.", "Go away."), (keys[1], "Go away.", "Hello there.")]
    {
        click_tab(&mut h, &Tab::Dialog(front));
        settle(&mut h);
        assert_eq!(h.state().active_editor(), Some(&Tab::Dialog(front)));
        assert_eq!(h.state().right_dock_shows(), Some(Panel::NodeData), "the dock stays");
        assert!(shows(&h, dock(&h), own), "{own} in the dock");
        assert!(!shows(&h, dock(&h), other), "not {other}");
        assert!(!shows(&h, dock(&h), "Who are you?"), "nor the floating one's");
        // Its bookmarks in the Outliner, not the other's, nor the floating
        // one's (listed in its window).
        let (mine, theirs) = match front == keys[0] {
            true => ("Entry 0: Hello there.", "Entry 1: Go away."),
            false => ("Entry 1: Go away.", "Entry 0: Hello there."),
        };
        let window = leaf_rect(&h, keys[2]);
        let outside = |h: &Harness<'_, Moonglow>, row: &str| {
            crate::conversation_outliner::outliner_rows(h)
                .iter()
                .any(|(name, at)| name == row && main.contains(*at) && !window.contains(*at))
        };
        assert!(outside(&h, mine), "{mine} in the Outliner");
        assert!(!outside(&h, theirs), "not {theirs}");
        assert!(!outside(&h, "Reply 0: Who are you?"), "nor the floating one's");
    }
    let window = leaf_rect(&h, keys[2]);
    assert!(shows(&h, window, "Who are you?"), "ccc's line in its window");
    let in_window = crate::conversation_outliner::outliner_rows(&h);
    let in_window =
        in_window.iter().any(|(n, at)| n == "Reply 0: Who are you?" && window.contains(*at));
    assert!(in_window, "ccc's bookmark in its window's Outliner");
}

/// A field typed in keeps to its own line: the line's text typed, then
/// another line chosen in the tree, the text is the first line's, not the
/// second's, as one step of Undo; and typing on while a change made
/// elsewhere (a plugin, the file changed outside) numbers the lines anew,
/// what is typed follows its line, never set on the line that took its
/// number.
#[test]
fn a_field_typed_in_keeps_to_its_own_line() {
    for mode in MODES {
        let (mut h, keys) = dialogs_window(mode, "ui-dialog-typed", &["talk"], 1600.0);
        let key = keys[0];
        let type_in = |h: &mut Harness<'_, Moonglow>, line: &str, words: &str| {
            h.get_by_label(&format!("[OWNER] - {line}")).click();
            settle(h);
            let text = h
                .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
                .find(|n| n.accesskit_node().value().as_deref() == Some(line))
                .expect("the line's text")
                .rect();
            click_point(h, text.center());
            h.key_press(egui::Key::End);
            h.event(egui::Event::Text(words.into()));
            h.run();
            assert!(h.ctx.text_edit_focused(), "typing");
        };
        type_in(&mut h, "Hello there.", " Welcome.");
        h.get_by_label("[OWNER] - Go away.").click();
        settle(&mut h);
        assert_eq!(
            dialog_lines(&mut h, key),
            ["Entry|Hello there. Welcome.|", "  Reply|Who are you?|", "Entry|Go away.|"],
            "{mode:?}"
        );
        h.state_mut().actions.push(mg_ui::Action::Undo);
        settle(&mut h);
        assert_eq!(dialog_lines(&mut h, key)[0], "Entry|Hello there.|", "{mode:?}: one step");
        // "Go away." (entry 1) typed in; the first greeting deleted
        // elsewhere: "Go away." is entry 0 now, and a line added after it
        // takes number 1.
        type_in(&mut h, "Go away.", " Now.");
        {
            let ws = h.state_mut().ws.as_mut().unwrap();
            let mut g = ws.doc(&key).unwrap().clone();
            mg_module::dialog::remove(&mut g, Parent::Root, 0);
            let i = add_node(&mut g, Parent::Root, "Farewell.");
            assert_eq!(i, 1, "Farewell takes number 1");
            let edit = mg_edit::Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) };
            ws.apply(mg_edit::Command::new("Outside", vec![edit])).unwrap();
        }
        h.run();
        h.event(egui::Event::Text(" Go!".into()));
        h.run();
        h.get_by_label("[OWNER] - Farewell.").click();
        settle(&mut h);
        // What was typed is "Go away."'s, typed on; "Farewell." shows its
        // own text, nothing typed elsewhere in its field.
        assert_eq!(
            dialog_lines(&mut h, key),
            ["Entry|Go away. Now. Go!|", "Entry|Farewell.|"],
            "{mode:?}"
        );
        let shown: Vec<String> = h
            .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .filter_map(|n| n.accesskit_node().value())
            .collect();
        assert!(shown.contains(&"Farewell.".to_string()), "{mode:?}: {shown:?}");
    }
}

/// A row of the tree dragged from `from` and dropped on `onto` (a move;
/// with Ctrl, a link).
fn drag_row(h: &mut Harness<'_, Moonglow>, from: &str, onto: &str, mods: egui::Modifiers) {
    let (a, b) = (at(h, from).center(), at(h, onto).center());
    h.hover_at(a);
    h.step();
    press(h, a, true, mods);
    h.step();
    for k in 1..=8 {
        h.event(egui::Event::PointerMoved(a + (b - a) * k as f32 / 8.0));
        h.step();
    }
    press(h, b, false, mods);
    settle(h);
}

/// A conversation only looked at (one of the game's, given to the
/// workspace to read) shows its panels read-only and its header's Close
/// alone, every command that would change it off or refused: Add, Cut,
/// Paste (a line copied, by the button and by Ctrl+V), Paste As Link,
/// Delete, Move Up and Down (and Alt and the arrows), Import Lines…, a
/// drag in its tree (a move, and a link with Ctrl), Script Wizard…,
/// Replace All, what is typed in its panels' fields; the module never
/// given it.
#[test]
fn a_conversation_only_looked_at_is_read_only() {
    let (mut h, _) = dialogs_window(Mode::Tabs, "ui-dialog-viewed", &[], 1600.0);
    let key = ResKey::parse("game_talk", ResType::DLG).unwrap();
    h.state_mut().ws.as_mut().unwrap().view(key, talk());
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    settle(&mut h);
    let as_it_was = dialog_lines(&mut h, key);
    h.get_by_label("[OWNER] - Go away.").click();
    settle(&mut h);
    assert!(h.query_by_label("OK").is_none() && h.query_by_label("Cancel").is_none());
    h.get_by_label(mg_ui::dialog_view::CONVERSATION_HEADER).get_by_label("Close");
    // A line copied (Copy changes nothing), then every command off.
    dialog_click(&mut h, "Copy");
    assert!(h.state().dialog_clip.is_some(), "Copy works");
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(&mut h);
    for name in ["Add", "Cut", "Paste As Link", "Delete", "Move Up", "Move Down", "Import Lines…"]
    {
        assert!(dialog_tool(&mut h, name).accesskit_node().is_disabled(), "{name}");
    }
    // (Paste where the line copied would go: under Root.)
    h.get_by_label("Root").click();
    settle(&mut h);
    assert!(dialog_tool(&mut h, "Paste").accesskit_node().is_disabled(), "Paste under Root");
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(&mut h);
    // Their keys, with the pointer over the tree.
    let over = at(&h, "[OWNER] - Hello there.").center();
    h.hover_at(over);
    h.step();
    for (mods, key) in [
        (egui::Modifiers::COMMAND, egui::Key::V),
        (egui::Modifiers::COMMAND, egui::Key::A),
        (egui::Modifiers::ALT, egui::Key::ArrowDown),
        (egui::Modifiers::NONE, egui::Key::Delete),
    ] {
        h.key_press_modifiers(mods, key);
        settle(&mut h);
    }
    h.event(egui::Event::Paste("anything".into()));
    settle(&mut h);
    assert_eq!(dialog_lines(&mut h, key), as_it_was, "the keys changed nothing");
    // A drag in its tree: a move, then a link.
    drag_row(&mut h, "[OWNER] - Go away.", "Who are you? [END DIALOGUE]", egui::Modifiers::NONE);
    drag_row(&mut h, "[OWNER] - Go away.", "[OWNER] - Hello there.", egui::Modifiers::COMMAND);
    assert_eq!(dialog_lines(&mut h, key), as_it_was, "a drag changed nothing");
    // Script Wizard… and Replace All off.
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(&mut h);
    for heading in ["Text Appears When…", "Actions Taken"] {
        let wizard = in_section(&h, heading, "Script Wizard…");
        assert!(wizard.accesskit_node().is_disabled(), "{heading}'s Script Wizard…");
    }
    h.state_mut().show_conversation_search(key);
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.find = "Hello".into();
    settle(&mut h);
    assert!(h.get_by_label("Replace All").accesskit_node().is_disabled(), "Replace All");
    tab(&mut h, "Data");
    // Its fields look read-only and keep nothing typed in them: the
    // conversation as it was, and the log says so.
    let text = h
        .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Hello there."))
        .expect("the line's text")
        .rect();
    click_point(&mut h, text.center());
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text(" Welcome.".into()));
    h.run();
    h.get_by_label("[OWNER] - Go away.").click();
    settle(&mut h);
    assert_eq!(dialog_lines(&mut h, key), as_it_was, "nothing kept");
    let log: Vec<String> = h.state().log.entries.iter().map(|(_, m)| m.clone()).collect();
    assert!(log.iter().any(|l| l.contains("only looked at")), "{log:?}");
    let ws = h.state().ws.as_ref().unwrap();
    assert!(!ws.module.contains(&key) && ws.is_viewed(&key), "never given to the module");
    // (M4's U15.) A bookmark is the view's, not the conversation's: set, it
    // is listed in the Outliner, and a click there goes to its line. The
    // Status Bar hints nothing that would change it.
    h.get_by_label("Bookmark").click();
    settle(&mut h);
    let row = "Entry 1: Go away.";
    // (Root chosen first: the click goes to the line.)
    h.get_by_label("Root").click();
    settle(&mut h);
    assert_eq!(h.state().dialog_views[&key].selected, None);
    crate::conversation_outliner::outliner_row(&h, row).expect("listed").click();
    settle(&mut h);
    assert_eq!(h.state().dialog_views[&key].bookmarks, [(Kind::Entry, 1)]);
    let go_away = mg_ui::dialog_view::Row { parent: Parent::Root, pos: 1 };
    assert_eq!(h.state().dialog_views[&key].selected, Some(go_away), "gone to its line");
    let said = h.state().status_said();
    assert!(said.iter().any(|s| s == "Double Click: Go to linked"), "{said:?}");
    for not in ["Drag: Move line", "Ctrl, Drag: Link", "Ctrl, A: Add", "Alt, Up: Move up"] {
        assert!(!said.iter().any(|s| s == not), "{not}: {said:?}");
    }
    assert_eq!(dialog_lines(&mut h, key), as_it_was, "nothing changed");
}

/// Every panel of a conversation fits the Right Dock from its least width
/// to 600, in the editor's larger text too: each drawn (a control of its
/// own found), nothing of it past the dock's edge.
#[test]
fn every_conversation_panel_fits_the_right_dock() {
    let (mut h, keys) = dialogs_window(Mode::Tabs, "ui-dialog-fit", &["talk"], 1600.0);
    let key = keys[0];
    // (The fullest line: a condition with a parameter, an action whose
    // script has its source, a sound, a journal update, a long comment;
    // and Save Set's name being typed.)
    {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let mut g = ws.doc(&key).unwrap().clone();
        let link = &mut g.root.list_mut("StartingList").unwrap()[0];
        link.set("Active", mg_gff::Value::resref(ResRef::from_str("m1q01a01aribsck1").unwrap()));
        let pairs = vec![("CHECK".to_string(), "12".to_string())];
        link.set("ConditionParams", mg_module::dialog::params_value(&pairs));
        let line = &mut g.root.list_mut("EntryList").unwrap()[0];
        line.set("Script", mg_gff::Value::resref(ResRef::from_str("a_act").unwrap()));
        line.set("Speaker", mg_gff::Value::String(b"m1q1_aribeth_x".to_vec()));
        line.set("Sound", mg_gff::Value::resref(ResRef::from_str("vs_naribetf_050").unwrap()));
        line.set("Quest", mg_gff::Value::String(b"q_rats".to_vec()));
        line.set("QuestEntry", mg_gff::Value::Dword(10));
        line.set(
            "Comment",
            mg_gff::Value::String(b"A comment long enough to wrap twice.".to_vec()),
        );
        let source = b"void main()\n{\n    SpeakString(\"A rather long line of a script, wider than the panel.\");\n}\n";
        let script = ResKey::parse("a_act", ResType::NSS).unwrap();
        let edits = vec![
            mg_edit::Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) },
            mg_edit::Edit::SetResource { key: script, data: Some(source.to_vec()) },
        ];
        ws.apply(mg_edit::Command::new("Fullest", edits)).unwrap();
    }
    h.state_mut().param_set_dir = Some(mg_testkit::scratch_dir("ui-dialog-fit-sets"));
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(&mut h);
    in_section(&h, "Text Appears When…", mg_ui::dialog_view::SET_COMMANDS).click();
    settle(&mut h);
    h.get_by_label("Save Set…").click();
    settle(&mut h);
    h.get_by_label("Set name");
    h.get_by_label("Journal entry");
    for larger in [None, Some(20)] {
        h.state_mut().settings.dialog_font_size = larger;
        for width in [280.0, 340.0, 440.0, 600.0] {
            right_dock_width(&mut h, width);
            settle(&mut h);
            for (front, own) in
                [("Data", "Script preview"), ("Search", "Find What"), ("Data", "Speaker tag")]
            {
                tab(&mut h, front);
                let dock = right_dock_rect(&h);
                let what = format!("{larger:?} {width} {front}");
                assert!((dock.width() - width - 1.0).abs() < 2.0, "{width}: {dock:?}");
                assert!(dock.contains(at(&h, own).center()), "{what}: {own} drawn");
                // (The Conversation panel under them, its first row in
                // sight: its rows are given their room, M4's U14 review.)
                assert!(dock.contains(at(&h, "Normal end").center()), "{what}");
                // (And its last row whole: the section given its rows'
                // room at the editor's text size.)
                let zoom = at(&h, "Stop camera zoom in");
                assert!(dock.contains_rect(zoom), "{what}: Stop camera zoom in whole {zoom:?}");
                fits(&h, dock, larger, &what);
                // (Each section of Data scrolled into sight in turn: rows
                // under the fold are checked too.)
                if front == "Data" {
                    for row in ["Actions Taken", "Other Actions", "Journal entry", "Comments"] {
                        h.get_all_by_label(row).next().unwrap().scroll_to_me();
                        settle(&mut h);
                        fits(&h, dock, larger, &format!("{what} at {row}"));
                    }
                    h.get_all_by_label("Text").next().unwrap().scroll_to_me();
                    settle(&mut h);
                }
            }
        }
    }
}

/// What a conversation's panels show in `dock` fits it: nothing of what
/// is in sight past its edge (but the script preview's code, which
/// scrolls sideways in its frame), and every one-line field in sight as
/// wide as its text at the editor's text size (`larger`: a name cut short
/// inside its field counts).
#[track_caller]
fn fits(h: &Harness<'_, Moonglow>, dock: egui::Rect, larger: Option<u8>, what: &str) {
    let in_sight = |n: &egui_kittest::Node<'_>| {
        let r = n.rect();
        dock.contains(r.left_top() + egui::vec2(1.0, 1.0)) && r.bottom() <= dock.bottom()
    };
    let past: Vec<String> = h
        .get_all_by(|n| n.bounding_box().is_some())
        .filter(in_sight)
        .filter(|n| n.rect().right() > dock.right() + 0.5)
        // (The script's preview scrolls sideways in its frame: its code is
        // as wide as its longest line.)
        .filter(|n| {
            let a = n.accesskit_node();
            let text = a.value().or(a.label()).unwrap_or_default();
            !text.contains("SpeakString")
        })
        .map(|n| {
            let a = n.accesskit_node();
            format!("{:?} {:?} {:?} {:?}", a.role(), a.label(), a.value(), n.rect())
        })
        .collect();
    assert!(past.is_empty(), "{what}: past the edge {past:?}");
    let size = larger.map_or(mg_uikit::metrics::BODY_SIZE, f32::from);
    let margins = {
        let m = mg_uikit::controls::FIELD_MARGIN;
        f32::from(m.left) + f32::from(m.right)
    };
    let cut: Vec<String> = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .filter(in_sight)
        .filter_map(|n| {
            let value = n.accesskit_node().value()?;
            let font = egui::FontId::proportional(size);
            let words = h.ctx.fonts_mut(|f| {
                f.layout_no_wrap(value.clone(), font, egui::Color32::WHITE).size().x
            });
            (n.rect().width() + 0.5 < words + margins)
                .then(|| format!("{value:?}: {} for {}", n.rect().width(), words + margins))
        })
        .collect();
    assert!(cut.is_empty(), "{what}: cut short in their fields {cut:?}");
}

/// A key pressed with the pointer over the conversation's tree, and the
/// window let come to rest ([`settle`] fails where it keeps repainting).
pub(crate) fn tree_key(h: &mut Harness<'_, Moonglow>, mods: egui::Modifiers, key: egui::Key) {
    let over = at(h, "Root").center();
    h.hover_at(over);
    h.step();
    h.key_press_modifiers(mods, key);
    settle(h);
}

/// The text field of the panel in front that the case types in: the
/// line's text (Data), Find What (Search), Normal end (Conversation).
pub(crate) fn type_in_front(h: &mut Harness<'_, Moonglow>, front: &str) {
    let field = match front {
        "Data" => h
            .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .find(|n| n.accesskit_node().value().as_deref() == Some("Hello there."))
            .map(|n| n.rect()),
        "Search" => Some(field_of(h, "Find What")),
        "Conversation" => Some(field_of(h, "Normal end")),
        _ => None,
    };
    let Some(field) = field else { return };
    click_point(h, field.center());
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("x".into()));
    settle(h);
    h.key_press(egui::Key::Tab);
    settle(h);
}

/// The window comes to rest with each of the conversation's panels in
/// front, behind and away, docked and floating: Data drawn first (with a
/// line chosen, so the Node section's Bookmark could be pressed), then
/// the case's panel in front (Data, Search, Conversation; Conversation in
/// front of Data in the one bar under 1400, the Node section behind it;
/// the Right Dock folded; floating, its window's pane), what it shows
/// typed in, a search
/// run (F3), the selection moved with the keys (to Root and back, so that
/// Data and the bar drawn before last showed another line and another
/// Bookmark), a line deleted (the selection gone) and the delete undone:
/// each settles (the endless redraw of M4's U12 review fails as "keeps
/// repainting"), and the keys moved the selection.
#[test]
fn the_window_comes_to_rest_with_each_conversation_panel() {
    let cases: [(Mode, &str, f32); 7] = [
        (Mode::Tabs, "Data", 1600.0),
        (Mode::Tabs, "Search", 1600.0),
        (Mode::Tabs, "Conversation", 1300.0),
        (Mode::Tabs, "folded", 1600.0),
        (Mode::Floating, "Data", 1600.0),
        (Mode::Floating, "Search", 1600.0),
        (Mode::Floating, "Conversation", 1600.0),
    ];
    for (mode, front, width) in cases {
        let (mut h, keys) = dialogs_window(mode, "ui-dialog-rest", &["talk"], width);
        let key = keys[0];
        let what = format!("{mode:?} {front}");
        h.get_by_label("[OWNER] - Hello there.").click();
        settle(&mut h);
        match front {
            "folded" => {
                h.state_mut().settings.hide_palettes = true;
                settle(&mut h);
            }
            // (Floating, the Conversation panel stands under the Node
            // section, drawn whatever is in front there.)
            "Conversation" if mode == Mode::Floating => {}
            front => tab(&mut h, front),
        }
        type_in_front(&mut h, front);
        // A search run, with the pointer over the tree.
        h.state_mut().dialog_views.get_mut(&key).unwrap().search.find = "Go".into();
        tree_key(&mut h, egui::Modifiers::NONE, egui::Key::F3);
        // (F3 unfolds a folded dock to show Search: folded again.)
        if front == "folded" {
            h.state_mut().settings.hide_palettes = true;
            settle(&mut h);
        }
        // (Back to the case's front: F3 brings Search to it.)
        if !matches!(front, "folded" | "Search")
            && !(front == "Conversation" && mode == Mode::Floating)
        {
            tab(&mut h, front);
        }
        h.get_all_by_label_contains("[OWNER] - Hello").next().unwrap().click();
        settle(&mut h);
        let chosen = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
        let before = chosen(&h);
        tree_key(&mut h, egui::Modifiers::NONE, egui::Key::Home);
        assert_eq!(chosen(&h), None, "{what}: Root");
        for k in [egui::Key::ArrowDown, egui::Key::ArrowDown, egui::Key::ArrowDown] {
            tree_key(&mut h, egui::Modifiers::NONE, k);
        }
        assert_ne!(chosen(&h), before, "{what}: the keys moved the selection");
        assert!(chosen(&h).is_some(), "{what}");
        // A line deleted: no selection; undone.
        tree_key(&mut h, egui::Modifiers::NONE, egui::Key::Delete);
        assert_eq!(chosen(&h), None, "{what}: deleted");
        tree_key(&mut h, egui::Modifiers::COMMAND, egui::Key::Z);
        assert_eq!(dialog_lines(&mut h, key).len(), 3, "{what}: undone");
    }
}

/// Records from before the conversation's line's fields were the Right
/// Dock's read without a fault: a layout kept for conversations naming
/// the old bottom pane's tabs (Bookmarks, Current File) or a newer one's
/// panel as its fronts; an open-tabs view naming the old pane, its tab
/// and its height. Each passes over what it does not know: Data in front,
/// the line chosen and its bookmark kept.
#[test]
fn records_naming_the_old_bottom_pane_read_without_error() {
    for (fronts, front) in [
        ("[\"Bookmarks\",\"Current File\"]", Panel::NodeData),
        ("[\"Search\"]", Panel::NodeSearch),
        ("[\"Data\",\"Comments\"]", Panel::NodeData),
    ] {
        let dir = mg_testkit::scratch_dir("ui-dialog-old-records");
        let path = sample_module(&dir);
        let key = ResKey::parse("talk", ResType::DLG).unwrap();
        let mut m = Module::open(&path).unwrap();
        m.set_gff(key, &talk()).unwrap();
        m.save().unwrap();
        let content = mg_ui::dialog_view::fingerprint(&talk());
        let tabs = dir.join("open-tabs");
        std::fs::create_dir_all(&tabs).unwrap();
        let record = format!(
            r#"{{"panes":[{{"place":"middle","tabs":["dialog:talk.dlg"]}}],
            "front":"dialog:talk.dlg",
            "views":{{"dialog:talk.dlg":{{"selected":"root 1","bookmarks":["entry 1"],
            "bottom":"Search","tab":"File","height":260,"content":{content}}}}}}}"#
        );
        std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
        let text = format!(
            "(dock_layouts:{{\"dialog\":(bottom:Some(\"Data\"),fronts:{fronts},shares:[1200,800])}})"
        );
        let (settings, unread) = mg_ui::Settings::read_fields(&text).expect("read");
        assert!(unread.is_empty(), "{unread:?}");
        let mut app = app_in(Mode::Tabs, Vec::new());
        app.settings.dock_layouts = settings.dock_layouts;
        app.settings.no_last_area = true;
        app.settings.dialog_no_backup = true;
        app.open_tabs_dir = Some(tabs);
        app.open_module(&path);
        let mut h = harness(app, 1600.0, 900.0);
        settle(&mut h);
        assert_eq!(h.state().active_editor(), Some(&Tab::Dialog(key)));
        assert_eq!(h.state().right_dock_shows(), Some(front), "{fronts}");
        h.get_by_label_contains("words in the file");
        assert_eq!(h.state().dialog_views[&key].bookmarks, [(Kind::Entry, 1)], "{fronts}");
        // (Kept across the restart, listed in the Outliner: M4's U15.)
        let listed = crate::conversation_outliner::outliner_row(&h, "Entry 1: Go away.");
        assert!(listed.is_some(), "{fronts}: listed in the Outliner");
    }
}

/// The conversation header's controls, in their order.
const DIALOG_CONTROLS: [&str; 15] = [
    "Add",
    "Copy",
    "Cut",
    "Paste",
    "Paste As Link",
    "Delete",
    "Move Up",
    "Move Down",
    "Expand All",
    "Collapse All",
    "Highlight",
    "Scripts",
    "Export",
    "Import Lines…",
    "Test",
];

/// The conversation header holds every control (on its row, as an icon,
/// or in More) at every window width from the least to 1600 points, both
/// ways editors open, OK and Cancel at its end, and its Data, Search and
/// Conversation stay in reach beside it (the Right Dock, its strip where
/// the window folds it, or the window's pane); at the least width a
/// highlight, Scripts and an Export format work from More.
#[test]
fn the_conversation_header_holds_every_control_from_420_to_1600() {
    use mg_uikit::header::{self, Place};
    let header_name = mg_ui::dialog_view::CONVERSATION_HEADER;
    let mut in_more = 0;
    for mode in MODES {
        let (mut h, keys) = dialogs_window(mode, "ui-dialog-header-widths", &["talk"], 1600.0);
        let key = keys[0];
        let mut widths: Vec<f32> = (21..=80).map(|n| n as f32 * 20.0).collect();
        widths.extend([1399.0, 1400.0, 961.0, 962.0, 795.0, 796.0]);
        for width in widths {
            h.set_size(egui::vec2(width, 800.0));
            settle(&mut h);
            let rec = header::records(&h.ctx, header_name);
            let rec = rec.into_iter().next().unwrap_or_else(|| panic!("no header at {width}"));
            // (The highlights' own rows where their group is in More.)
            let highlights = ["Comments", "Actions", "Quests", "Animations", "Sounds"];
            let names: Vec<&str> = (rec.places.iter())
                .map(|(n, _)| n.as_str())
                .filter(|n| !n.is_empty() && !highlights.contains(n))
                .collect();
            assert_eq!(names, DIALOG_CONTROLS, "{mode:?} {width}");
            in_more += rec.places.iter().filter(|(_, p)| *p == Place::More).count();
            let end = rec.end.expect("the end slot");
            for n in ["OK", "Cancel"] {
                let r = h.get_by_label(header_name).get_by_label(n).rect();
                assert!(end.expand(0.5).contains_rect(r), "{mode:?} {width}: {n} in {end:?}");
            }
            // (The highlights' group by its toggles, as its record lists
            // them: on the row or in More.)
            let held: Vec<&str> = (rec.places.iter())
                .map(|(n, _)| n.as_str())
                .filter(|n| !n.is_empty() && *n != "Highlight")
                .collect();
            assert!(held.contains(&"Comments") && held.contains(&"Sounds"), "{held:?}");
            header_holds(&mut h, header_name, &held);
            for panel in ["Data", "Search", "Conversation"] {
                let reach = h.query_all_by_label(panel).any(|n| !in_group(&n, "Top Bar"));
                let header = h.query_all_by_label_contains(&format!("{panel}, ")).next().is_some();
                assert!(reach || header, "{mode:?} {width}: {panel} out of reach");
            }
        }
        // At the least width: a highlight, Scripts and Export's CSV… from
        // More.
        h.set_size(egui::vec2(420.0, 800.0));
        settle(&mut h);
        let place = |h: &Harness<'_, Moonglow>, n: &str| {
            let rec = header::records(&h.ctx, header_name).into_iter().next().unwrap();
            rec.places.iter().find(|(p, _)| p == n).map(|(_, p)| *p)
        };
        // (Floating, in the window's room beside its pane; As Tabs the
        // docks fold to their strips at 420 and the header has the rest.)
        if mode == Mode::Floating {
            for n in ["Highlight", "Scripts", "Export"] {
                assert_eq!(place(&h, n), Some(Place::More), "{mode:?}: {n} in More at 420");
            }
        }
        dialog_click(&mut h, "Comments");
        settle(&mut h);
        assert!(h.state().dialog_views[&key].highlight[0], "{mode:?}: Comments from More");
        dialog_click(&mut h, "Scripts");
        settle(&mut h);
        assert!(h.state().settings.dialog_hide_scripts, "{mode:?}: Scripts from More");
        let csv = mg_testkit::scratch_dir(&mode.dir("ui-dialog-header-csv")).join("talk.csv");
        h.state_mut().dialogs =
            Box::new(NoDialogs { save: vec![csv.clone()], ..Default::default() });
        // (Export's formats are rows of More under its heading.)
        let _ = dialog_tool(&mut h, "Export");
        h.get_by_label(DIALOG_MORE).get_by_label("CSV…").click();
        settle(&mut h);
        assert!(csv.is_file(), "{mode:?}: exported from More");
    }
    assert!(in_more > 0, "the sweep reached widths where controls go into More");
}

/// What the panels changed is the conversation's window's to set back:
/// its text and its Normal end typed in Data and the Conversation panel,
/// then Cancel pressed in the header, both are as they were, in one step.
#[test]
fn cancel_sets_back_what_its_panels_changed() {
    for mode in MODES {
        let (mut h, keys) = dialogs_window(mode, "ui-dialog-cancel", &["talk"], 1600.0);
        let key = keys[0];
        h.get_by_label("[OWNER] - Hello there.").click();
        settle(&mut h);
        let text = h
            .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .find(|n| n.accesskit_node().value().as_deref() == Some("Hello there."))
            .expect("the line's text")
            .rect();
        click_point(&mut h, text.center());
        h.key_press(egui::Key::End);
        h.event(egui::Event::Text(" Welcome.".into()));
        h.run();
        // (Normal end's field, the Conversation panel's first.)
        let end = field_of(&h, "Normal end");
        click_point(&mut h, end.center());
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.event(egui::Event::Text("my_end".into()));
        h.run();
        h.key_press(egui::Key::Tab);
        settle(&mut h);
        let doc = |h: &mut Harness<'_, Moonglow>| {
            h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone()
        };
        let end_of = |g: &mg_gff::Gff| g.root.resref("EndConversation").unwrap().to_string();
        assert_eq!(dialog_lines(&mut h, key)[0], "Entry|Hello there. Welcome.|", "{mode:?}");
        assert_eq!(end_of(&doc(&mut h)), "my_end", "{mode:?}");
        h.get_by_label(mg_ui::dialog_view::CONVERSATION_HEADER).get_by_label("Cancel").click();
        settle(&mut h);
        assert!(h.state().dock.find_tab(&Tab::Dialog(key)).is_none(), "{mode:?}: closed");
        assert_eq!(dialog_lines(&mut h, key)[0], "Entry|Hello there.|", "{mode:?}: set back");
        assert_ne!(end_of(&doc(&mut h)), "my_end", "{mode:?}: set back");
    }
}

/// Find in a floating conversation brings its own Search to the front, in
/// its window, and leaves the docked conversation's Right Dock as it was;
/// and Find in the docked one leaves the floating one's window as it was.
#[test]
fn find_brings_the_search_of_its_own_conversation_forward() {
    let (mut h, keys) = dialogs_window(Mode::Tabs, "ui-dialog-find-own", &["aaa", "bbb"], 1600.0);
    // (Low and to the right: the docked conversation's first lines in
    // sight above it.)
    own_window(&mut h, &Tab::Dialog(keys[1]), egui::pos2(560.0, 320.0), egui::vec2(1000.0, 540.0));
    click_tab(&mut h, &Tab::Dialog(keys[0]));
    settle(&mut h);
    let window = leaf_rect(&h, keys[1]);
    let search_in = |h: &Harness<'_, Moonglow>, room: egui::Rect| {
        h.query_all_by_label("Find What").any(|n| room.contains(n.rect().center()))
    };
    assert_eq!(h.state().right_dock_shows(), Some(Panel::NodeData));
    // Find in the floating one.
    let line = h
        .get_all_by_label("[OWNER] - Go away.")
        .find(|n| window.contains(n.rect().center()))
        .unwrap()
        .rect();
    click_point(&mut h, line.center());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    settle(&mut h);
    assert!(search_in(&h, leaf_rect(&h, keys[1])), "its window's Search in front");
    assert_eq!(h.state().right_dock_shows(), Some(Panel::NodeData), "the dock as it was");
    // Find in the docked one, now in front of it: its dock's Search, the
    // floating one's window as it was.
    click_tab(&mut h, &Tab::Dialog(keys[0]));
    settle(&mut h);
    let docked = h
        .get_all_by_label("[OWNER] - Go away.")
        .find(|n| !leaf_rect(&h, keys[1]).contains(n.rect().center()))
        .unwrap()
        .rect();
    click_point(&mut h, docked.center());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(Panel::NodeSearch), "the dock's Search");
    assert!(search_in(&h, leaf_rect(&h, keys[1])), "the window's as it was");
}

/// Where `key`'s conversation's group is drawn (its pane, of several in
/// the main window).
fn group_rect(h: &Harness<'_, Moonglow>, key: ResKey) -> egui::Rect {
    let at = h.state().dock.find_tab(&Tab::Dialog(key)).expect("its tab");
    let leaves = h.state().dock.iter_leaves();
    let mut found = leaves.filter(|(p, _)| p.surface == at.surface && p.node == at.node);
    found.next().expect("its group").1.rect
}

/// Find (Ctrl+F) over a conversation docked in a group without the focus
/// works for that conversation, whose keys fired: its group takes the
/// focus, the Right Dock is its, Search in front, Find What the
/// keyboard's; nothing is left asked to take the keyboard later.
#[test]
fn find_over_a_conversation_in_another_group_is_its_own() {
    let (mut h, keys) = dialogs_window(Mode::Tabs, "ui-dialog-find-group", &["aaa", "bbb"], 1600.0);
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    click_tab(&mut h, &Tab::Dialog(keys[0]));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Dialog(keys[0])));
    let other = group_rect(&h, keys[1]);
    let line = h
        .get_all_by_label("[OWNER] - Go away.")
        .find(|n| other.contains(n.rect().center()))
        .expect("bbb's line")
        .rect();
    h.hover_at(line.center());
    h.step();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Dialog(keys[1])), "bbb's group in front");
    assert_eq!(h.state().right_dock_shows(), Some(Panel::NodeSearch));
    assert!(h.ctx.text_edit_focused(), "Find What has the keyboard");
    assert!(right_dock_rect(&h).contains(at(&h, "Find What").center()));
}

/// The Right Dock's seam between a conversation's Node section and its
/// Conversation panel, dragged, is kept for conversations across a
/// restart; and the seam in a floating conversation's window too (the
/// kind's shares: `DIALOG_SHARES`, `DIALOG_WINDOW_SHARES` until dragged).
/// The successor of the old pane's height test.
#[test]
fn a_conversation_s_section_seam_is_kept_across_a_restart() {
    for mode in MODES {
        let (mut h, keys) = dialogs_window(mode, "ui-dialog-seam", &["talk"], 1600.0);
        let key = keys[0];
        let room = match mode {
            Mode::Tabs => right_dock_rect(&h),
            Mode::Floating => leaf_rect(&h, key),
        };
        // (The seam: just over the Conversation panel's bar.)
        let normal = at(&h, "Normal end").top();
        let seam_y = |h: &Harness<'_, Moonglow>| at(h, "Normal end").top();
        let x = room.right() - 60.0;
        // (The label stands a few points into its row, under the bar's 26
        // and the panel's 8 at its top; the seam is the line over the bar.)
        let seam = (normal - 3.0 - 8.0 - 26.0 - 1.5).round();
        let from = egui::pos2(x, seam);
        h.hover_at(from);
        h.step();
        press(&h, from, true, egui::Modifiers::NONE);
        h.step();
        // (Up by 100: far from where an even split of the dock, or the
        // kind's first shares, would put it.)
        for k in 1..=5 {
            h.event(egui::Event::PointerMoved(from - egui::vec2(0.0, 20.0 * k as f32)));
            h.step();
        }
        press(&h, from - egui::vec2(0.0, 100.0), false, egui::Modifiers::NONE);
        settle(&mut h);
        let dragged = seam_y(&h);
        assert!(dragged < normal - 80.0, "{mode:?}: dragged up, {normal} to {dragged}");
        let kept = h.state().settings.dock_layouts.get("dialog").map(|l| l.shares.clone());
        let kept = kept.unwrap_or_default();
        assert!(kept.len() == 2 && kept[0] > kept[1] + 400, "{mode:?}: kept {kept:?}");
        // (Where the seam stands, as a share of the dock's or the pane's
        // height: the window of the next run need not be as large.)
        let share = |h: &Harness<'_, Moonglow>, room: egui::Rect| {
            (at(h, "Normal end").top() - room.top()) / room.height()
        };
        let first = share(&h, room);
        // A restart with the settings kept.
        let text = h.state().settings.saved_text();
        let (settings, unread) = mg_ui::Settings::read_fields(&text).expect("read");
        assert!(unread.is_empty(), "{unread:?}");
        let (mut again, keys) = dialogs_window(mode, "ui-dialog-seam-again", &["talk"], 1600.0);
        again.state_mut().settings.dock_layouts = settings.dock_layouts;
        settle(&mut again);
        let room = match mode {
            Mode::Tabs => right_dock_rect(&again),
            Mode::Floating => leaf_rect(&again, keys[0]),
        };
        let restored = share(&again, room);
        assert!((restored - first).abs() < 0.02, "{mode:?}: {first} again at {restored}");
    }
}

/// The top of the Data heading `name` drawn in the Right Dock (its
/// headings are named so elsewhere too: the highlight "Comments").
fn heading_top(h: &Harness<'_, Moonglow>, name: &str) -> Option<f32> {
    let dock = right_dock_rect(h);
    let x = dock.x_range();
    h.query_all_by_label(name).map(|n| n.rect()).find(|r| x.contains(r.center().x)).map(|r| r.top())
}

/// How tall Data's sections stand: from Text Appears When…'s heading to
/// Comments' (both script sections, their resref rows, Other Actions),
/// whatever Data is scrolled to.
fn data_height(h: &Harness<'_, Moonglow>) -> f32 {
    let top = heading_top(h, "Text Appears When…").expect("Text Appears When…");
    heading_top(h, "Comments").expect("Comments") - top
}

/// Data never stands taller where it is wider (M4's U15 review: U14's
/// `form_row_least` tested the room beside a label without the gap the
/// row puts after it, so at 319 to 322 points a script's row put its Edit
/// onto a line of its own, taller than at 311 with its label over it;
/// beside its scroll bar such a Data never came to rest), and it comes
/// to rest at every width: the Right Dock from 270 to 480 a point at a
/// time, at the editor's own text, at 14 and at 20 points, beside an NPC
/// line, a reply that ends the conversation and a greeting; around each
/// width where its layout changes, at window heights from 820 to 976.
#[test]
fn data_is_never_taller_where_wider_and_comes_to_rest() {
    let (mut h, _) = dialogs_window(Mode::Tabs, "ui-dialog-data-sweep", &["talk"], 1600.0);
    for size in [None, Some(14u8), Some(20u8)] {
        h.state_mut().settings.dialog_font_size = size;
        for line in ["[OWNER] - Hello there.", "Who are you? [END DIALOGUE]", "[OWNER] - Go away."]
        {
            h.state_mut().settings.right_dock_width = None;
            h.set_size(egui::vec2(1600.0, 900.0));
            settle(&mut h);
            h.get_by_label(line).click();
            settle(&mut h);
            let mut was: Option<(u32, f32)> = None;
            let mut least: Option<(u32, f32)> = None;
            let mut changes = Vec::new();
            for w in (270..=480u32).step_by(10) {
                h.state_mut().settings.right_dock_width = Some(w);
                settle(&mut h);
                let high = data_height(&h);
                // (Held to the least height at any narrower width, M4's
                // U17 review: step to step, a growth of a quarter point a
                // step went through.)
                if let Some((at, low)) = least {
                    assert!(high <= low + 0.5, "{size:?} {line}: {low} at {at}, {high} at {w}");
                }
                if let Some((_, before)) = was
                    && (high - before).abs() > 0.5
                {
                    changes.push(w);
                }
                was = Some((w, high));
                if least.is_none_or(|(_, low)| high < low) {
                    least = Some((w, high));
                }
            }
            // (Around each change, at window heights a few points apart:
            // the viewport where the content's height lands.)
            for w in changes {
                for at in w.saturating_sub(1)..=w + 1 {
                    h.state_mut().settings.right_dock_width = Some(at);
                    for high in (820..=976).step_by(52) {
                        h.set_size(egui::vec2(1600.0, high as f32));
                        settle(&mut h);
                    }
                }
                h.set_size(egui::vec2(1600.0, 900.0));
            }
        }
    }
}
