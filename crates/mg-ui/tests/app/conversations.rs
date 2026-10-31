//! The conversation editor and its tree's keys.

use crate::common::*;

#[test]
fn conversation_lines_show_their_talk_table_text() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-dialog-tlk");
    let path = sample_module(&dir);
    // A line whose text is the game's talk table's string 12 alone.
    let mut g = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "");
    let entries = g.root.list_mut("EntryList").unwrap();
    entries[0].set("Text", mg_gff::Value::LocString(LocString::from_strref(mg_core::StrRef(12))));
    let mut m = Module::open(&path).unwrap();
    let key = ResKey::parse("tlkdlg", ResType::DLG).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    h.run();
    h.get_by_label_contains("[OWNER] - Paladin").click();
    h.run();
    h.get_by_label_contains("From the talk table (string 12): Paladin");
}

#[test]
fn conversation_editor_builds_and_links() {
    let dir = mg_testkit::scratch_dir("ui-dialog");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    // New lines' text edited in place (the Input Text popup off).
    app.settings.dialog_no_text_popup = true;
    app.open_module(&path);
    app.new_dialog = Some("capdlg".into());
    let mut h = window_harness(app);
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("capdlg", ResType::DLG).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Dialog(key)).is_some());

    let is_text = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput
    };
    let add_line = |h: &mut Harness<'_, Moonglow>, text: &str| {
        h.get_by_label("Add").click();
        h.run();
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).find(is_text).unwrap().click();
        h.run();
        h.get_all_by_role(egui::accesskit::Role::MultilineTextInput)
            .find(is_text)
            .unwrap()
            .type_text(text);
        h.run();
        h.key_press(egui::Key::Tab);
        h.run();
    };
    let click = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_by_label(label).click();
        h.run();
    };
    add_line(&mut h, "Hello there.");
    add_line(&mut h, "Who are you?");
    add_line(&mut h, "A traveller.");
    click(&mut h, "Root");
    add_line(&mut h, "Go away.");
    click(&mut h, "Who are you?");
    click(&mut h, "Copy");
    click(&mut h, "[OWNER] - Go away.");
    click(&mut h, "Paste As Link");

    let doc = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone()
    };
    let ours = doc(&mut h);
    let texts = |g: &Gff| -> Vec<String> {
        mg_module::dialog::outline(g)
            .iter()
            .map(|l| l.split('|').take(3).collect::<Vec<_>>().join("|"))
            .collect()
    };
    assert_eq!(
        texts(&ours),
        [
            "Entry|Hello there.|",
            "  Reply|Who are you?|",
            "    Entry|A traveller.|",
            "Entry|Go away.|",
            "  Reply|Who are you?|link"
        ]
    );
    assert_eq!(ours.root.dword("NumWords"), Some(9));
    if let Some(capture) = mg_testkit::aurora_capture("dialog/capdlg.dlg") {
        let aurora = Gff::read(&std::fs::read(capture).unwrap()).unwrap();
        assert_eq!(texts(&ours), texts(&aurora));
    }

    // Deleting "Hello there." takes its branch and the link to it; undo
    // brings both back.
    click(&mut h, "[OWNER] - Hello there.");
    click(&mut h, "Delete");
    assert_eq!(texts(&doc(&mut h)), ["Entry|Go away.|"]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(texts(&doc(&mut h)).len(), 5);
}

/// The conversation editor's lower pane stays as tall as it is dragged
/// to: taller than what it shows (it went back to its content when that
/// was less), and shorter (its pages scroll).
#[test]
fn the_conversation_editor_s_lower_pane_keeps_its_size() {
    use mg_module::dialog::{Parent, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("ui-dialog-pane");
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    add_node(&mut g, Parent::Root, "Hello there.");
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = harness(app, 1200.0, 900.0);
    h.run_steps(3);
    // (The pane's top edge: just above its page tabs.)
    let edge = |h: &Harness<'_, Moonglow>| h.get_by_label("Data").rect().top() - 5.0;
    let drag = |h: &mut Harness<'_, Moonglow>, by: f32| {
        let from = egui::pos2(h.get_by_label("Data").rect().center().x + 200.0, edge(h));
        h.hover_at(from);
        press(h, from, true, egui::Modifiers::NONE);
        for k in 1..=5 {
            h.hover_at(from + egui::vec2(0.0, by * k as f32 / 5.0));
        }
        press(h, from + egui::vec2(0.0, by), false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    h.get_by_label_contains("Hello there.").click();
    h.run_steps(3);
    let before = edge(&h);
    drag(&mut h, -120.0);
    let taller = edge(&h);
    assert!(taller < before - 100.0, "dragged taller: {before} → {taller}");
    // Root chosen: less to show, and the pane stays as it is.
    h.get_by_label("Root").click();
    h.run_steps(4);
    assert!((edge(&h) - taller).abs() < 2.0, "{taller} → {}", edge(&h));
    // A line again, and the pane dragged shorter than what it shows.
    h.get_by_label_contains("Hello there.").click();
    h.run_steps(3);
    drag(&mut h, 250.0);
    let shorter = edge(&h);
    assert!(shorter > taller + 240.0, "dragged shorter: {taller} → {shorter}");
}

/// A script parameter typed in the conversation editor is saved in the
/// game's codepage, the one it is shown in: "é" as the byte 0xE9, shown
/// back as "é" (it was saved as UTF-8, two bytes shown back as "Ã©").
#[test]
fn conversation_script_parameters_are_saved_in_the_game_s_codepage() {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog, node};
    let dir = mg_testkit::scratch_dir("ui-dialog-params");
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    add_node(&mut g, Parent::Root, "Hello there.");
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = harness(app, 1200.0, 900.0);
    h.run_steps(3);
    h.get_by_label_contains("Hello there.").click();
    h.run_steps(3);
    h.get_by_label("Actions Taken").click();
    h.run_steps(3);
    h.get_by_label("+").click();
    h.run_steps(3);
    let fields = |h: &Harness<'_, Moonglow>| -> Vec<egui::Rect> {
        h.get_all_by_role(egui::accesskit::Role::TextInput).map(|n| n.rect()).collect()
    };
    // The new parameter's name and value: the last two fields.
    let at = fields(&h);
    let (name, value) = (at[at.len() - 2], at[at.len() - 1]);
    for (rect, text) in [(name, "who"), (value, "café")] {
        h.get_all_by_role(egui::accesskit::Role::TextInput)
            .find(|n| n.rect() == rect)
            .unwrap()
            .click();
        h.run();
        h.get_all_by_role(egui::accesskit::Role::TextInput)
            .find(|n| n.rect() == rect)
            .unwrap()
            .type_text(text);
        h.run();
    }
    h.key_press(egui::Key::Tab);
    h.run_steps(3);
    let g = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
    let line = node(&g, Kind::Entry, 0).unwrap();
    let p = &line.list("ActionParams").unwrap()[0];
    assert_eq!(p.string("Key"), Some(&b"who"[..]));
    assert_eq!(p.string("Value"), Some(&b"caf\xe9"[..]));
    // Shown back as typed.
    let shown: Vec<String> = h
        .get_all_by_role(egui::accesskit::Role::TextInput)
        .filter_map(|n| n.accesskit_node().value())
        .collect();
    assert!(shown.iter().any(|v| v == "café"), "{shown:?}");
    assert!(!shown.iter().any(|v| v.contains('Ã')), "{shown:?}");
}

/// The conversation editor's text has a size of its own (Options ›
/// Conversation Editor): its lines and the fields under them, and nothing
/// outside it.
#[test]
fn the_conversation_editor_s_text_has_its_own_size() {
    use mg_module::dialog::{Parent, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("ui-dialog-font");
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    add_node(&mut g, Parent::Root, "Hello there.");
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = window_harness(app);
    h.run();
    let heights = |h: &Harness<'_, Moonglow>| {
        let line = h.get_by_label_contains("Hello there.").rect().height();
        (line, h.get_by_label("Expand All").rect().height(), h.get_by_label("File").rect().height())
    };
    let (line, button, menu) = heights(&h);
    h.state_mut().settings.dialog_font_size = Some(24);
    h.run_steps(3);
    let (large, large_button, same_menu) = heights(&h);
    assert!(large > line * 1.5, "{line} to {large}");
    assert!(large_button > button * 1.5, "{button} to {large_button}");
    assert_eq!(same_menu, menu, "the window's own text is as it was");
}

#[test]
fn conversation_search_bookmarks_and_test() {
    use mg_module::dialog::{Kind, Parent, add_link, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("ui-dialog-tools");
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    add_node(&mut g, Parent::Node(Kind::Reply, who), "A traveller.");
    let go = add_node(&mut g, Parent::Root, "Go away.");
    add_link(&mut g, Parent::Node(Kind::Entry, go), who);
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = window_harness(app);
    h.run();
    let click = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_by_label(label).click();
        h.run();
    };

    // Search finds the line and selects it in the tree. Ctrl+F, with the
    // pointer over the editor, opens the Search pane; F3 finds.
    assert!(h.query_by_label("Find What").is_none());
    let over = h.get_by_label("Test").rect().center();
    h.hover_at(over);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run();
    h.get_by_label("Find What");
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.find = "traveller".into();
    h.run();
    // F3 with the Find What field still having the keyboard, as in
    // Aurora (Sacha's report: it took a click back into the editor).
    h.key_press(egui::Key::F3);
    h.run();
    let results = h.state().dialog_views[&key].search.results.clone();
    assert_eq!(results.len(), 1, "F3 finds");
    click(&mut h, "Find");
    let results = h.state().dialog_views[&key].search.results.clone();
    assert_eq!(results.len(), 1);
    // Match Case, ticked in the pane, is what Find and Replace All go by
    // (the script editor's options): "Traveller" finds nothing with it.
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.find = "Traveller".into();
    click(&mut h, "Match Case");
    assert!(h.state().dialog_views[&key].search.options.match_case);
    click(&mut h, "Find");
    assert!(h.state().dialog_views[&key].search.results.is_empty(), "Match Case");
    click(&mut h, "Match Case");
    click(&mut h, "Find");
    let results = h.state().dialog_views[&key].search.results.clone();
    assert_eq!(results.len(), 1, "case set aside");
    click(&mut h, &format!("Entry {}: A traveller.", results[0].2));
    let selected = h.state().dialog_views[&key].selected.unwrap();
    assert_eq!(selected.parent, Parent::Node(Kind::Reply, who));

    // A bookmark on it.
    click(&mut h, "Bookmark");
    assert_eq!(h.state().dialog_views[&key].bookmarks, [(Kind::Entry, results[0].2)]);

    // Replace All in this conversation.
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.find = "Go away.".into();
    h.state_mut().dialog_views.get_mut(&key).unwrap().search.replace = "Leave.".into();
    h.run();
    click(&mut h, "Replace All");
    let doc = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
    assert!(mg_module::dialog::outline(&doc).iter().any(|l| l.starts_with("Entry|Leave.|")));

    // Test mode: the greeting said, the reply chosen, the answer said;
    // back a turn.
    click(&mut h, "Test");
    h.get_by_label("NPC: Hello there.");
    click(&mut h, "1. Who are you?");
    h.get_by_label("NPC: A traveller.");
    h.get_by_label("[END DIALOGUE]");
    assert_eq!(h.state().dialog_views[&key].test.as_ref().unwrap().len(), 2);
    click(&mut h, "<-- Back");
    assert!(h.state().dialog_views[&key].test.as_ref().unwrap().is_empty());
    click(&mut h, "Done");
    assert!(h.state().dialog_views[&key].test.is_none());
}

/// A link goes to the line it links to, where that is owned, its parents
/// opened out: by a double click on it, as in Aurora, and by its menu's
/// Go to Linked (a builder's report: nothing happened).
#[test]
fn a_conversation_link_goes_to_its_line() {
    use mg_module::dialog::{Kind, Parent, add_link, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("ui-dialog-go-to-linked");
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    let go = add_node(&mut g, Parent::Root, "Go away.");
    add_link(&mut g, Parent::Node(Kind::Entry, go), who);
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    // (Frames a sixtieth of a second apart: the two clicks of a double
    // click in a frame each.)
    let mut h = Harness::builder()
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // The greeting folded: only the link's row shows.
    h.state_mut().dialog_views.get_mut(&key).unwrap().collapsed.insert((Kind::Entry, hello));
    h.run();
    assert!(h.query_by_label("Who are you? [END DIALOGUE]").is_none());
    let owned = mg_ui::dialog_view::Row { parent: Parent::Node(Kind::Entry, hello), pos: 0 };
    // A double click on the link.
    let at = h.get_by_label("Who are you?").rect().center();
    h.hover_at(at);
    h.run_steps(60);
    let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
    for pressed in [true, false, true, false] {
        h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
    }
    h.run();
    let view = &h.state().dialog_views[&key];
    assert_eq!(view.selected, Some(owned), "the line where it is owned");
    assert!(!view.collapsed.contains(&(Kind::Entry, hello)), "its parent opened out");
    h.get_by_label("Who are you? [END DIALOGUE]");
    // The link's menu: Go to Linked.
    let link = mg_ui::dialog_view::Row { parent: Parent::Node(Kind::Entry, go), pos: 0 };
    h.state_mut().dialog_views.get_mut(&key).unwrap().selected = Some(link);
    h.state_mut().dialog_views.get_mut(&key).unwrap().collapsed.insert((Kind::Entry, hello));
    h.run();
    h.get_by_label("Who are you?").click_secondary();
    h.run();
    h.get_by_label("Go to Linked").click();
    h.run();
    assert_eq!(h.state().dialog_views[&key].selected, Some(owned));
    // Not on a row that is no link.
    h.get_all_by_label_contains("Hello there.").next().unwrap().click_secondary();
    h.run();
    assert!(h.query_by_label("Go to Linked").is_none());
}

#[test]
fn add_popup_text_gives_a_placeable_a_one_line_conversation() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let game = game_data(&root);
    let mut rng = fastrand::Rng::with_seed(6);
    let mut m = new_module(&game, "Barrel", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bp = game.resman.get(&ResKey::parse("x3_plc_barrel1", ResType::UTP).unwrap()).unwrap();
    let bp = Gff::read(&bp).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let barrel = instance(&placing, ResType::UTP, &bp.root, at, &[]).unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Placeable List", mg_gff::Value::List(vec![barrel]));
    m.set_gff(git, &g).unwrap();
    let path = mg_testkit::scratch_dir("ui-popup-text").join("barrel.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.popup_text = Some(mg_ui::store_wizard::PopupText {
        area,
        placeable: 0,
        text: "Just an old barrel.".into(),
        name: "mg_popup".into(),
    });
    let mut h = window_harness(app);
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().popup_text.is_none());
    let ws = h.state_mut().ws.as_mut().unwrap();
    let dlg = ws.doc(&ResKey::parse("mg_popup", ResType::DLG).unwrap()).unwrap().root.clone();
    assert_eq!(dlg.list("EntryList").map(<[_]>::len), Some(1));
    assert_eq!(dlg.list("ReplyList").map(<[_]>::len), Some(0));
    let g = ws.doc(&git).unwrap();
    let barrel = &g.root.list("Placeable List").unwrap()[0];
    assert_eq!(barrel.resref("Conversation"), Some(ResRef::from_str("mg_popup").unwrap()));
}

#[test]
fn conversation_lines_without_speaker_names_when_chosen() {
    let dir = mg_testkit::scratch_dir("ui-dialog-names");
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_talk", ResType::DLG).unwrap();
    let mut g = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut g, mg_module::dialog::Parent::Root, "Hello there");
    let mut m = Module::open(&path).unwrap();
    m.set(key, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.dialog_hide_names = true; // Options › Conversation Editor
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = window_harness(app);
    h.run();
    h.get_by_label("Hello there");
    assert!(h.query_by_label("[OWNER] - Hello there").is_none());
}

#[test]
fn conversation_lines_show_their_scripts_and_play_as_the_game_plays_them() {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog, node_mut, set_condition};
    let dir = mg_testkit::scratch_dir("ui-dialog-play");
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_keeper", ResType::DLG).unwrap();
    let mut g = new_dialog();
    // Two greetings: the first only for those seen before.
    let again = add_node(&mut g, Parent::Root, "Back again?");
    set_condition(&mut g, Parent::Root, 0, "c_seen");
    let hello = add_node(&mut g, Parent::Root, "Hello, stranger.");
    let n = node_mut(&mut g, Kind::Entry, hello).unwrap();
    n.set("Script", mg_gff::Value::resref(ResRef::from_str("a_hello").unwrap()));
    n.set("Quest", mg_gff::Value::String(b"q_intro".to_vec()));
    n.set("QuestEntry", mg_gff::Value::Dword(10));
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    set_condition(&mut g, Parent::Node(Kind::Entry, hello), 0, "c_curious");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Bye.");
    add_node(&mut g, Parent::Node(Kind::Reply, who), "I am the keeper.");
    let _ = again;
    let mut m = Module::open(&path).unwrap();
    m.set(key, g.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = harness(app, 1100.0, 800.0);
    h.run();
    // The lines name their scripts and journal updates.
    h.get_by_label("if c_seen");
    h.get_by_label("do a_hello  journal q_intro 10");
    h.get_by_label("Scripts").click();
    h.run();
    assert!(h.state().settings.dialog_hide_scripts);
    assert!(h.query_by_label("if c_seen").is_none());
    // Played through: the first greeting whose condition passes.
    h.get_by_label("Test").click();
    h.run();
    h.get_by_label("NPC: Back again?");
    h.get_by_label("if c_seen: TRUE").click();
    h.run();
    h.get_by_label("(not said) Back again?");
    h.get_by_label("NPC: Hello, stranger.");
    h.get_by_label("do a_hello  journal q_intro 10");
    // A reply whose condition fails is hidden.
    h.get_by_label("if c_curious: TRUE").click();
    h.run();
    h.get_by_label("(hidden) Who are you?");
    h.get_by_label("if c_curious: FALSE").click();
    h.run();
    h.get_by_label("2. Bye.");
    h.get_by_label("1. Who are you?").click();
    h.run();
    h.get_by_label("NPC: I am the keeper.");
    h.get_by_label("You: Who are you?");
    h.get_by_label("[END DIALOGUE]");
}

#[test]
fn conversations_export_and_import() {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog, nodes, text};
    let dir = mg_testkit::scratch_dir("ui-dialog-io");
    let path = sample_module(&dir);
    let key = ResKey::parse("mg_guard", ResType::DLG).unwrap();
    let mut g = new_dialog();
    let halt = add_node(&mut g, Parent::Root, "Halt!");
    add_node(&mut g, Parent::Node(Kind::Entry, halt), "Sorry.");
    let mut m = Module::open(&path).unwrap();
    m.set(key, g.to_bytes().unwrap());
    m.save().unwrap();
    // A Twine export, then the CSV edited and read back.
    let twee = dir.join("guard.twee");
    let csv = dir.join("guard.csv");
    let mut app = Moonglow::new(
        None,
        Box::new(NoDialogs { save: vec![csv.clone(), twee.clone()], ..Default::default() }),
    );
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = harness(app, 1100.0, 800.0);
    h.run();
    // Options › Language French (this test thread's): the CSV has a
    // column for it, to be translated into.
    mg_ui::set_edit_language(mg_core::Language::FRENCH);
    for format in ["Twine (Twee)…", "CSV…"] {
        h.get_by_label("Export").click();
        h.run();
        h.get_by_label(format).click();
        h.run();
    }
    mg_ui::set_edit_language(mg_core::Language::ENGLISH);
    assert!(std::fs::read_to_string(&twee).unwrap().contains("[[Sorry.->END]]"));
    let exported = std::fs::read_to_string(&csv).unwrap();
    assert!(exported.starts_with("id,speaker,text,condition,action,parent,comment,text_fr\r\n"));
    let edited = (exported.replace("Halt!", "Stop right there!"))
        .replace("start,,\r\n", "start,,Halte-là !\r\n");
    std::fs::write(&csv, edited).unwrap();
    h.state_mut().dialogs = Box::new(NoDialogs { open: vec![csv], ..Default::default() });
    h.get_by_label("Import Lines…").click();
    h.run();
    let line = |h: &mut Harness<'_, Moonglow>, k: ResKey| {
        let g = h.state_mut().ws.as_mut().unwrap().doc(&k).unwrap().clone();
        text(&nodes(&g, Kind::Entry)[0])
    };
    assert_eq!(line(&mut h, key), "Stop right there!");
    let g = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
    let french = nodes(&g, Kind::Entry)[0].locstring("Text").unwrap().clone();
    let french = french.text(mg_core::Language::FRENCH, mg_core::Gender::Male).unwrap();
    assert_eq!(french, "Halte-là !");
    // An Ink story, as a new conversation named after its file.
    let ink = dir.join("Night Watch.ink");
    std::fs::write(&ink, "-> gate\n=== gate ===\nWho goes there?\n+ [A friend.] -> END\n").unwrap();
    let new = h.state_mut().import_conversation(&ink).expect("imported");
    h.run();
    assert_eq!(new.resref.to_string(), "night_watch");
    assert!(h.state().dock.find_tab(&Tab::Dialog(new)).is_some());
    assert_eq!(line(&mut h, new), "Who goes there?");
}

#[test]
fn speaker_tags_come_from_the_module_s_creatures() {
    use egui_kittest::kittest::Queryable;
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let game = game_data(&root);
    let mut rng = fastrand::Rng::with_seed(8);
    let mut m = new_module(&game, "Speakers", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    let bp = game.resman.get(&ResKey::parse("nw_bandit001", ResType::UTC).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let mut bandit =
        instance(&placing, ResType::UTC, &Gff::read(&bp).unwrap().root, at, &[]).unwrap();
    bandit.set("Tag", mg_gff::Value::String(b"MG_SPEAKER".to_vec()));
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![bandit]));
    m.set_gff(git, &g).unwrap();
    let key = ResKey::parse("mg_talk", ResType::DLG).unwrap();
    let mut d = mg_module::dialog::new_dialog();
    mg_module::dialog::add_node(&mut d, mg_module::dialog::Parent::Root, "Hello there");
    m.set(key, d.to_bytes().unwrap());
    let path = mg_testkit::scratch_dir("ui-speaker-tags").join("speakers.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app = game_app(&root, None);
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    let mut h = harness(app, 1100.0, 800.0);
    h.run();
    h.get_by_label("[OWNER] - Hello there").click();
    h.run();
    // (The speaker's: not the palette's dropdown of types.)
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let speaker = |n: &egui_kittest::Node<'_>| {
        let value = n.accesskit_node().value().unwrap_or_default();
        !PALETTE_TYPES.iter().any(|glyph| value.starts_with(glyph))
    };
    h.query_all(combo).find(speaker).expect("the speaker's dropdown").click();
    h.run();
    h.get_by_label("MG_SPEAKER").click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let dlg = ws.doc(&key).unwrap().root.clone();
    assert_eq!(dlg.list("EntryList").unwrap()[0].string("Speaker"), Some(&b"MG_SPEAKER"[..]));
}

#[test]
fn conversation_options_popup_link_directions_drag_and_backup() {
    let dir = mg_testkit::scratch_dir("ui-dialog-options");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.conversation_backups = dir.join("backups");
    app.new_dialog = Some("optdlg".into());
    let mut h = window_harness(app);
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("optdlg", ResType::DLG).unwrap();
    let outline = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let g = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
        mg_module::dialog::outline(&g)
            .iter()
            .map(|l| l.split('|').take(3).collect::<Vec<_>>().join("|"))
            .collect()
    };
    let click = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_all_by_label(label).last().unwrap().click();
        h.run();
    };
    // The popup's text field (its text is also a text run).
    fn input<'h>(h: &'h mut Harness<'_, Moonglow>) -> egui_kittest::Node<'h> {
        let role = egui::accesskit::Role::MultilineTextInput;
        h.get_all_by_value("<< Enter text here >>")
            .find(|n| n.accesskit_node().role() == role)
            .expect("the Input Text field")
    }
    // Add asks for the text (the placeholder selected, typed over); the
    // parent stays selected.
    let add = |h: &mut Harness<'_, Moonglow>, text: &str| {
        click(h, "Add");
        input(h).type_text(text);
        h.run();
        click(h, "OK");
    };
    add(&mut h, "Hello");
    assert!(h.query_by_label("Enter what the NPC says next:").is_none());
    click(&mut h, "Add");
    assert!(h.query_by_label("Enter what the NPC says next:").is_some());
    click(&mut h, "Cancel");
    add(&mut h, "Second");
    click(&mut h, "[OWNER] - Hello");
    click(&mut h, "Add");
    assert!(h.query_by_label("Enter what the player says next:").is_some());
    input(&mut h).type_text("Hi");
    h.run();
    click(&mut h, "OK");
    assert_eq!(outline(&mut h), ["Entry|Hello|", "  Reply|Hi|", "Entry|Second|"]);

    // Paste As Link, Aurora's default: the selected line links to the
    // copied one.
    click(&mut h, "[OWNER] - Second");
    click(&mut h, "Copy");
    click(&mut h, "Hi [END DIALOGUE]");
    click(&mut h, "Paste As Link");
    assert_eq!(
        outline(&mut h),
        ["Entry|Hello|", "  Reply|Hi|", "    Entry|Second|link", "Entry|Second|"]
    );
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    // The same from the line's right-click menu, which selects it.
    click(&mut h, "[OWNER] - Second");
    h.get_by_label("Hi [END DIALOGUE]").click_secondary();
    h.run();
    h.get_all_by_label("Paste As Link").last().unwrap().click();
    h.run();
    h.run();
    assert_eq!(
        outline(&mut h),
        ["Entry|Hello|", "  Reply|Hi|", "    Entry|Second|link", "Entry|Second|"]
    );
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    // Link Source To Destination: the copied line links to the selected.
    h.state_mut().settings.dialog_paste_source_to_dest = true;
    click(&mut h, "Hi [END DIALOGUE]");
    click(&mut h, "Paste As Link");
    assert_eq!(
        outline(&mut h),
        ["Entry|Hello|", "  Reply|Hi|", "Entry|Second|", "  Reply|Hi|link"]
    );
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();

    // A drag moves Hi under Second; with Ctrl, Hi links to Second.
    let drag = |h: &mut Harness<'_, Moonglow>, from: &str, to: &str, ctrl: bool| {
        let a = h.get_by_label(from).rect().center();
        let b = h.get_by_label(to).rect().center();
        h.hover_at(a);
        h.run();
        h.drag_at(a);
        h.run();
        h.hover_at(a + egui::vec2(10.0, 4.0));
        h.run();
        h.hover_at(b);
        h.run();
        if ctrl {
            h.event(egui::Event::ModifiersChanged(egui::Modifiers::CTRL));
        }
        h.drop_at(b);
        h.run();
        h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        h.run();
    };
    drag(&mut h, "Hi [END DIALOGUE]", "[OWNER] - Second", true);
    assert_eq!(
        outline(&mut h),
        ["Entry|Hello|", "  Reply|Hi|", "    Entry|Second|link", "Entry|Second|"]
    );
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    drag(&mut h, "Hi [END DIALOGUE]", "[OWNER] - Second", false);
    assert_eq!(outline(&mut h), ["Entry|Hello|", "Entry|Second|", "  Reply|Hi|"]);

    // The backup: the open, changed conversation as optdlg.bak, once.
    let written = h.state_mut().backup_conversations();
    let bak = dir.join("backups").join("sample").join("optdlg.bak");
    assert_eq!(written, std::slice::from_ref(&bak));
    let g = Gff::read(&std::fs::read(&bak).unwrap()).unwrap();
    assert_eq!(mg_module::dialog::outline(&g).len(), 3);
    assert!(h.state_mut().backup_conversations().is_empty(), "unchanged since");
}

/// The sample module with conversation `talk` (`g`), opened as a builder
/// opens it: a click on it in the module tree, which takes the keys, and
/// Enter. In its window, as a conversation opens.
fn dialog_from_tree(name: &str, g: &Gff) -> (Harness<'static, Moonglow>, ResKey) {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, g).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    h.get_by_label_contains("Conversations").click();
    h.run();
    h.get_by_label("talk").click();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Dialog(key)).is_some());
    (h, key)
}

/// A conversation's lines as its outline: kind, text and whether a link.
fn dialog_lines(h: &mut Harness<'_, Moonglow>, key: ResKey) -> Vec<String> {
    let g = h.state_mut().ws.as_mut().unwrap().doc(&key).unwrap().clone();
    mg_module::dialog::outline(&g)
        .iter()
        .map(|l| l.split('|').take(3).collect::<Vec<_>>().join("|"))
        .collect()
}

/// A greeting with a nested branch, a second greeting with a link to the
/// first's reply (a line linked from elsewhere).
fn branching_dialog() -> Gff {
    use mg_module::dialog::{Kind, Parent, add_link, add_node, new_dialog};
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    let traveller = add_node(&mut g, Parent::Node(Kind::Reply, who), "A traveller.");
    add_node(&mut g, Parent::Node(Kind::Entry, traveller), "Farewell.");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Bye.");
    let go = add_node(&mut g, Parent::Root, "Go away.");
    add_link(&mut g, Parent::Node(Kind::Entry, go), who);
    g
}

const BRANCHING: [&str; 7] = [
    "Entry|Hello there.|",
    "  Reply|Who are you?|",
    "    Entry|A traveller.|",
    "      Reply|Farewell.|",
    "  Reply|Bye.|",
    "Entry|Go away.|",
    "  Reply|Who are you?|link",
];

/// How a line is deleted.
#[derive(Debug, Clone, Copy)]
enum DeleteBy {
    /// The Delete key, after a click on the line.
    Key,
    /// Its right-click menu's Delete.
    Menu,
    /// The toolbar's Delete, after a click on the line.
    Button,
}

/// Deletes the `nth` row labelled `label` of conversation `key`.
fn delete_line(h: &mut Harness<'_, Moonglow>, key: ResKey, label: &str, nth: usize, by: DeleteBy) {
    let at = h.get_all_by_label(label).nth(nth).unwrap().rect().center();
    match by {
        DeleteBy::Key | DeleteBy::Button => {
            h.get_all_by_label(label).nth(nth).unwrap().click();
            h.run();
            h.hover_at(at);
            h.run();
            if matches!(by, DeleteBy::Key) {
                h.key_press(egui::Key::Delete);
            } else {
                let button = h.get_all_by_label("Delete").next().unwrap();
                button.click();
            }
        }
        DeleteBy::Menu => {
            h.get_all_by_label(label).nth(nth).unwrap().click_secondary();
            h.run();
            let delete = h.get_all_by_label("Delete").collect::<Vec<_>>();
            assert_eq!(delete.len(), 2, "the menu's Delete and the toolbar's");
            delete[1].click();
        }
    }
    h.run();
    h.run();
    assert_eq!(h.state().confirm_delete, None, "{label} ({by:?}): not the whole conversation");
    assert_eq!(h.state().dialog_views[&key].selected, None, "{label} ({by:?})");
}

/// Sacha's report: Delete on a line of a conversation opened from the
/// module tree asked to delete the whole conversation (the module tree
/// still had the keys), and Yes closed Moonglow. Each kind of line goes,
/// by the Delete key, the line's menu and the toolbar; with it its branch
/// and the links to what it owned; undo brings them back.
#[test]
fn deleting_conversation_lines_by_key_menu_and_button() {
    let mut cases: Vec<(&str, usize, Vec<&str>)> = Vec::new();
    let without = |gone: &[usize]| -> Vec<&str> {
        BRANCHING.iter().enumerate().filter(|(i, _)| !gone.contains(i)).map(|(_, l)| *l).collect()
    };
    // A link (the line it links to stays).
    cases.push(("Who are you?", 1, without(&[6])));
    // A line linked from elsewhere: its branch, and the link to it.
    cases.push(("Who are you?", 0, without(&[1, 2, 3, 6])));
    // A starting line with lines under it.
    cases.push(("[OWNER] - Hello there.", 0, without(&[0, 1, 2, 3, 4, 6])));
    // The last starting line, with a link under it.
    cases.push(("[OWNER] - Go away.", 0, without(&[5, 6])));
    // A line deep in a branch, with a line under it.
    cases.push(("[OWNER] - A traveller.", 0, without(&[2, 3])));
    // A line at the end of a branch.
    cases.push(("Farewell. [END DIALOGUE]", 0, without(&[3])));
    for (label, nth, after) in cases {
        for by in [DeleteBy::Key, DeleteBy::Menu, DeleteBy::Button] {
            let (mut h, key) = dialog_from_tree("ui-dialog-delete", &branching_dialog());
            assert_eq!(dialog_lines(&mut h, key), BRANCHING);
            delete_line(&mut h, key, label, nth, by);
            assert_eq!(dialog_lines(&mut h, key), after, "{label} ({by:?})");
            assert!(h.state().dock.find_tab(&Tab::Dialog(key)).is_some(), "still open");
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
            h.run();
            assert_eq!(dialog_lines(&mut h, key), BRANCHING, "{label} ({by:?}): undone");
        }
    }
}

/// Deleting lines one after another as the lines left are numbered anew,
/// down to the last; a selection the tree no longer has (redone under it)
/// is let go; Delete with nothing selected does nothing.
#[test]
fn deleting_conversation_lines_down_to_the_last() {
    let (mut h, key) = dialog_from_tree("ui-dialog-delete-all", &branching_dialog());
    // The first greeting: the second is the first line now.
    delete_line(&mut h, key, "[OWNER] - Hello there.", 0, DeleteBy::Key);
    assert_eq!(dialog_lines(&mut h, key), ["Entry|Go away.|"]);
    // Undone; a line deep in the branch selected; redone under it.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    h.get_by_label("Farewell. [END DIALOGUE]").click();
    h.run();
    assert!(h.state().dialog_views[&key].selected.is_some());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run();
    h.run();
    assert_eq!(dialog_lines(&mut h, key), ["Entry|Go away.|"]);
    assert_eq!(h.state().dialog_views[&key].selected, None, "no such line now");
    // Keys with that selection gone.
    for k in [egui::Key::ArrowRight, egui::Key::ArrowLeft, egui::Key::ArrowDown] {
        h.key_press(k);
        h.run();
    }
    assert_eq!(dialog_lines(&mut h, key), ["Entry|Go away.|"]);
    // The last line.
    delete_line(&mut h, key, "[OWNER] - Go away.", 0, DeleteBy::Key);
    assert!(dialog_lines(&mut h, key).is_empty());
    // Nothing selected (Root): Delete deletes nothing, asks nothing.
    h.key_press(egui::Key::Delete);
    h.run();
    h.run();
    assert_eq!(h.state().confirm_delete, None);
    assert!(h.state().dock.find_tab(&Tab::Dialog(key)).is_some());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(dialog_lines(&mut h, key), ["Entry|Go away.|"]);
}

/// The second half of Sacha's crash: deleting the conversation (the
/// module tree's Delete…, answered Delete) while it is open in its window
/// closed Moonglow (egui_dock drew the window left empty). The window
/// closes with it, and undo brings the conversation back.
#[test]
fn deleting_a_conversation_open_in_its_window() {
    let (mut h, key) = dialog_from_tree("ui-dialog-delete-file", &branching_dialog());
    let window = |h: &Harness<'_, Moonglow>| {
        h.state()
            .dock
            .iter_surfaces()
            .filter(|s| matches!(s, egui_dock::Surface::Window(..)))
            .count()
    };
    assert_eq!(window(&h), 1, "the conversation's window");
    h.state_mut().actions.push(mg_ui::Action::DeleteDialog(key));
    h.run();
    h.get_by_label("Delete from Module");
    // (The toolbar's Delete is off: no line is selected.)
    let delete = h.get_all_by_label("Delete").find(|n| !n.accesskit_node().is_disabled());
    delete.unwrap().click();
    h.run();
    h.run();
    h.run();
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&key));
    assert!(h.state().dock.find_tab(&Tab::Dialog(key)).is_none());
    assert_eq!(window(&h), 0, "its window went with it");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key));
}

/// A conversation opened from the module tree has the keys at once, as
/// Aurora gives them to the conversation it opens: Delete before any
/// click in the editor asks nothing of the whole conversation (the module
/// tree's Delete, which still had the keys, did), and Down goes to its
/// first line.
#[test]
fn a_conversation_opened_from_the_tree_has_the_keys() {
    let (mut h, key) = dialog_from_tree("ui-dialog-opened-keys", &branching_dialog());
    h.key_press(egui::Key::Delete);
    h.run();
    h.run();
    assert_eq!(h.state().confirm_delete, None, "not the module tree's Delete");
    assert!(h.query_by_label("Delete from Module").is_none());
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key));
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert!(h.state().dialog_views[&key].selected.is_some(), "the tree's Down");
}

/// A list of choices in the conversation editor reached with Tab keeps the
/// arrow keys, though the tree has the keys: Down chooses the next
/// animation, and the tree's selection stays.
#[test]
fn a_conversation_s_list_reached_with_tab_keeps_the_arrows() {
    use mg_module::dialog::ANIMATIONS;
    let (mut h, key) = dialog_from_tree("ui-dialog-list-arrows", &branching_dialog());
    let selected = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
    h.get_by_label("[OWNER] - Hello there.").click();
    h.run();
    let line = selected(&h);
    assert!(line.is_some());
    h.get_by_label("Other Actions").click();
    h.run_steps(2);
    let combo = egui::accesskit::Role::ComboBox;
    let shows = |h: &Harness<'_, Moonglow>, v: &str| {
        h.query_all_by_role(combo).any(|n| n.accesskit_node().value().as_deref() == Some(v))
    };
    let animation = |h: &Harness<'_, Moonglow>| {
        h.get_all_by_role(combo)
            .find(|n| n.accesskit_node().value().as_deref() == Some(ANIMATIONS[0].0))
            .map(|n| n.is_focused())
    };
    assert_eq!(animation(&h), Some(false), "Play Animation, not in focus");
    // From the page's button, Tab from widget to widget until the list's.
    h.get_by_label("Other Actions").focus();
    h.run_steps(2);
    for _ in 0..20 {
        if animation(&h) == Some(true) {
            break;
        }
        h.key_press(egui::Key::Tab);
        h.run_steps(2);
    }
    assert_eq!(animation(&h), Some(true), "Tab reached Play Animation");
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(3);
    assert!(shows(&h, ANIMATIONS[1].0), "Down chose the next animation");
    assert_eq!(selected(&h), line, "and left the tree's selection");
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(3);
    assert!(shows(&h, ANIMATIONS[0].0), "and Up the first again");
    assert_eq!(selected(&h), line);
}

/// Sacha's report: in Aurora the arrow keys go from line to line of a
/// conversation; in Moonglow nothing happened (they were the module
/// tree's). After a click in the tree, as Windows' tree control (Aurora's)
/// has them: Up and Down to the row above or below, Right opens a line
/// out and then goes to its first line, Left folds it and then goes to
/// the line it is under, Home and End to the first row (Root) and the
/// last, Page Up and Page Down a page.
#[test]
fn the_conversation_tree_by_the_arrow_keys() {
    use mg_module::dialog::{Kind, Parent};
    use mg_ui::dialog_view::Row;
    let (mut h, key) = dialog_from_tree("ui-dialog-arrows", &branching_dialog());
    let tree_cursor = h.state().tree_cursor;
    assert!(tree_cursor.is_some(), "the module tree had the keys");
    let selected = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
    let press = |h: &mut Harness<'_, Moonglow>, k: egui::Key| {
        h.key_press(k);
        h.run();
        h.run();
    };
    let row = |parent: Parent, pos: usize| Some(Row { parent, pos });
    let hello = row(Parent::Root, 0);
    let who = row(Parent::Node(Kind::Entry, 0), 0);
    let traveller = row(Parent::Node(Kind::Reply, 0), 0);
    let farewell = row(Parent::Node(Kind::Entry, 1), 0);
    let bye = row(Parent::Node(Kind::Entry, 0), 1);
    let go = row(Parent::Root, 1);
    let link = row(Parent::Node(Kind::Entry, 2), 0);
    h.get_by_label("[OWNER] - Hello there.").click();
    h.run();
    assert_eq!(selected(&h), hello);
    // Down and Up, row by row, as they are drawn.
    for want in [who, traveller, farewell, bye, go, link] {
        press(&mut h, egui::Key::ArrowDown);
        assert_eq!(selected(&h), want);
    }
    press(&mut h, egui::Key::ArrowDown);
    assert_eq!(selected(&h), link, "the last row stays");
    assert_eq!(h.state().tree_cursor, tree_cursor, "not the module tree's keys");
    press(&mut h, egui::Key::ArrowUp);
    assert_eq!(selected(&h), go);
    // Home: Root; Up there stays; End: the last row.
    press(&mut h, egui::Key::Home);
    assert_eq!(selected(&h), None);
    press(&mut h, egui::Key::ArrowUp);
    assert_eq!(selected(&h), None);
    press(&mut h, egui::Key::End);
    assert_eq!(selected(&h), link);
    // A link has nothing under it: Right does nothing, Left goes up.
    press(&mut h, egui::Key::ArrowRight);
    assert_eq!(selected(&h), link);
    press(&mut h, egui::Key::ArrowLeft);
    assert_eq!(selected(&h), go);
    // Left folds an open line; again, up to Root; Right from Root goes to
    // the first line; Right opens the folded one, and again goes into it.
    press(&mut h, egui::Key::Home);
    press(&mut h, egui::Key::ArrowRight);
    assert_eq!(selected(&h), hello);
    press(&mut h, egui::Key::ArrowLeft);
    assert!(h.state().dialog_views[&key].collapsed.contains(&(Kind::Entry, 0)));
    assert!(h.query_by_label("Bye. [END DIALOGUE]").is_none(), "folded");
    assert_eq!(selected(&h), hello);
    press(&mut h, egui::Key::ArrowDown);
    assert_eq!(selected(&h), go, "past the folded lines");
    press(&mut h, egui::Key::ArrowUp);
    press(&mut h, egui::Key::ArrowLeft);
    assert_eq!(selected(&h), None, "Root");
    press(&mut h, egui::Key::ArrowDown);
    press(&mut h, egui::Key::ArrowRight);
    assert!(!h.state().dialog_views[&key].collapsed.contains(&(Kind::Entry, 0)));
    h.get_by_label("Bye. [END DIALOGUE]");
    assert_eq!(selected(&h), hello);
    press(&mut h, egui::Key::ArrowRight);
    assert_eq!(selected(&h), who);
    // Left from a line with lines under it, open: folds it; again: up.
    press(&mut h, egui::Key::ArrowLeft);
    assert_eq!(selected(&h), who);
    press(&mut h, egui::Key::ArrowLeft);
    assert_eq!(selected(&h), hello);
    // Page Down and Page Up: as far as there are rows, here.
    press(&mut h, egui::Key::PageDown);
    assert_eq!(selected(&h), link);
    press(&mut h, egui::Key::PageUp);
    assert_eq!(selected(&h), None);
    // Alt+Up and Alt+Down still move the line.
    press(&mut h, egui::Key::End);
    press(&mut h, egui::Key::ArrowUp);
    assert_eq!(selected(&h), go);
    h.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowUp);
    h.run();
    h.run();
    assert_eq!(selected(&h), hello, "moved up: the first line now");
    assert_eq!(dialog_lines(&mut h, key)[0], "Entry|Go away.|");
}

/// The arrow keys are the tree's only while no field is typed in: in the
/// line's Text and in the Find What field they move the text cursor.
#[test]
fn the_conversation_tree_s_keys_leave_text_fields_alone() {
    let (mut h, key) = dialog_from_tree("ui-dialog-arrows-text", &branching_dialog());
    let selected = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
    h.get_all_by_label("Who are you?").next().unwrap().click();
    h.run();
    let who = selected(&h);
    assert!(who.is_some());
    // The line's text: Home, then typing, puts the text first.
    let is_text = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput
    };
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).find(is_text).unwrap().click();
    h.run();
    for k in [egui::Key::ArrowDown, egui::Key::ArrowUp, egui::Key::End, egui::Key::Home] {
        h.key_press(k);
        h.run();
    }
    assert_eq!(selected(&h), who, "the selection stays");
    h.event(egui::Event::Text("So, ".into()));
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    h.run();
    assert!(dialog_lines(&mut h, key).contains(&"  Reply|So, Who are you?|".to_string()));
    // The Find What field.
    h.get_all_by_label("So, Who are you?").next().unwrap().click();
    h.run();
    let over = h.get_by_label("Test").rect().center();
    h.hover_at(over);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run();
    h.run();
    assert!(h.ctx.egui_wants_keyboard_input(), "Find What has the keyboard");
    h.event(egui::Event::Text("ab".into()));
    h.run();
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    h.event(egui::Event::Text("X".into()));
    h.run();
    assert_eq!(h.state().dialog_views[&key].search.find, "aXb");
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(selected(&h), who, "the selection stays");
}

/// A step of the arrow keys scrolls the line it goes to into sight.
#[test]
fn the_conversation_tree_s_keys_scroll_to_the_line() {
    use mg_module::dialog::{Parent, add_node, new_dialog};
    let mut g = new_dialog();
    for i in 0..80 {
        add_node(&mut g, Parent::Root, &format!("Line {i}."));
    }
    let (mut h, key) = dialog_from_tree("ui-dialog-arrows-scroll", &g);
    h.get_by_label("[OWNER] - Line 0.").click();
    h.run();
    let line = |h: &Harness<'_, Moonglow>, i: usize| {
        h.get_by_label(&format!("[OWNER] - Line {i}.")).rect()
    };
    assert!(line(&h, 79).top() > 800.0, "out of sight");
    h.key_press(egui::Key::End);
    h.run();
    h.run();
    h.run();
    let shown = line(&h, 79);
    assert!(shown.top() > 0.0 && shown.bottom() <= 800.0, "{shown:?}");
    assert!(line(&h, 0).top() < 0.0, "scrolled away");
    // Page Up: a page of rows above, drawn.
    h.key_press(egui::Key::PageUp);
    h.run();
    h.run();
    h.run();
    let at = h.state().dialog_views[&key].selected.unwrap().pos;
    assert!((40..79).contains(&at), "{at}");
    let shown = line(&h, at);
    assert!(shown.top() > 0.0 && shown.bottom() <= 800.0, "{shown:?}");
    // Down from there, one row.
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(h.state().dialog_views[&key].selected.unwrap().pos, at + 1);
    // Ctrl+Up and Ctrl+Down scroll a row, the selection staying.
    let selected = h.state().dialog_views[&key].selected;
    let top = line(&h, at).top();
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::ArrowUp);
        h.run();
        h.run();
    }
    let up = line(&h, at).top() - top;
    assert!((30.0..80.0).contains(&up), "two rows down the window: {up}");
    assert_eq!(h.state().dialog_views[&key].selected, selected);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::ArrowDown);
    h.run();
    h.run();
    let down = line(&h, at).top() - top;
    assert!((up * 0.4..up * 0.6).contains(&down), "a row back: {down} of {up}");
    assert_eq!(h.state().dialog_views[&key].selected, selected);
}

/// The other keys of Windows' tree control (Aurora's): Backspace to the
/// line a row is under; the keypad's + and − open and fold the selected
/// line, * opens everything under it; a letter typed goes to the next
/// line whose text begins with it (not the speaker an NPC's row names
/// first), round the end; with the tree's keys only, not in a text field.
#[test]
fn the_conversation_tree_s_other_keys() {
    use mg_module::dialog::{Kind, Parent};
    use mg_ui::dialog_view::Row;
    use mg_ui::keys::Cmd;
    let (mut h, key) = dialog_from_tree("ui-dialog-tree-keys", &branching_dialog());
    let selected = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
    let folded = |h: &Harness<'_, Moonglow>, line| {
        h.state().dialog_views[&key].collapsed.contains(&(Kind::Entry, line))
    };
    let press = |h: &mut Harness<'_, Moonglow>, k: egui::Key| {
        h.key_press(k);
        h.run();
        h.run();
    };
    let typed = |h: &mut Harness<'_, Moonglow>, t: &str| {
        h.event(egui::Event::Text(t.into()));
        h.run();
        h.run();
    };
    let row = |parent: Parent, pos: usize| Some(Row { parent, pos });
    let hello = row(Parent::Root, 0);
    let who = row(Parent::Node(Kind::Entry, 0), 0);
    let traveller = row(Parent::Node(Kind::Reply, 0), 0);
    let bye = row(Parent::Node(Kind::Entry, 0), 1);
    let go = row(Parent::Root, 1);
    let link = row(Parent::Node(Kind::Entry, 2), 0);
    // Backspace: up the branch to Root, and no further. (On macOS Delete
    // Line has Backspace too, the Mac's delete key, and a command's key
    // wins: there it is Delete alone here.)
    h.state_mut().keymap.set(
        Cmd::DeleteLine,
        vec![egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Delete)],
    );
    h.get_by_label("Farewell. [END DIALOGUE]").click();
    h.run();
    for want in [traveller, who, hello, None, None] {
        press(&mut h, egui::Key::Backspace);
        assert_eq!(selected(&h), want);
    }
    // Letters, each after a pause, from Root: "b" goes to Bye; "w" to Who
    // and to the link to it, then round the end; "[" to the NPC lines, as
    // their rows begin (Aurora's go by the row: watched), and "[owner] -
    // g" typed at once to "Go away."; "h" to no row; "r" to Root; a letter
    // no row begins with stays.
    let pause = |h: &mut Harness<'_, Moonglow>| h.run_steps(5);
    typed(&mut h, "b");
    assert_eq!(selected(&h), bye);
    for want in [link, who, link] {
        pause(&mut h);
        typed(&mut h, "w");
        assert_eq!(selected(&h), want);
    }
    pause(&mut h);
    typed(&mut h, "[");
    assert_eq!(selected(&h), hello);
    pause(&mut h);
    typed(&mut h, "[owner] - G");
    assert_eq!(selected(&h), go);
    pause(&mut h);
    typed(&mut h, "h");
    assert_eq!(selected(&h), go, "no row begins with h");
    pause(&mut h);
    typed(&mut h, "r");
    assert_eq!(selected(&h), None, "Root");
    pause(&mut h);
    typed(&mut h, "x");
    assert_eq!(selected(&h), None);
    // The keypad's −, then +: Hello folded, opened, the selection
    // staying on it; each again does nothing.
    press(&mut h, egui::Key::ArrowDown);
    press(&mut h, egui::Key::Minus);
    assert!(folded(&h, 0) && h.query_by_label("Bye. [END DIALOGUE]").is_none());
    press(&mut h, egui::Key::Minus);
    assert!(folded(&h, 0));
    assert_eq!(selected(&h), hello);
    press(&mut h, egui::Key::Plus);
    assert!(!folded(&h, 0));
    h.get_by_label("Bye. [END DIALOGUE]");
    assert_eq!(selected(&h), hello);
    // A letter finds only lines drawn: in a folded branch, none.
    press(&mut h, egui::Key::Minus);
    typed(&mut h, "f");
    assert_eq!(selected(&h), hello);
    // *: Hello's branch opened all through, a line folded deeper too;
    // not Go's.
    press(&mut h, egui::Key::Plus);
    h.get_by_label("[OWNER] - A traveller.").click();
    h.run();
    press(&mut h, egui::Key::Minus);
    assert!(folded(&h, 1));
    h.get_by_label("[OWNER] - Go away.").click();
    h.run();
    press(&mut h, egui::Key::Minus);
    h.get_by_label("[OWNER] - Hello there.").click();
    h.run();
    press(&mut h, egui::Key::Minus);
    typed(&mut h, "*");
    assert!(!folded(&h, 0) && !folded(&h, 1), "Hello's branch open");
    h.get_by_label("Farewell. [END DIALOGUE]");
    assert!(folded(&h, 2), "Go away still folded");
    // From Root: everything.
    press(&mut h, egui::Key::Home);
    typed(&mut h, "*");
    assert!(h.state().dialog_views[&key].collapsed.is_empty());
    // Typing in the line's text types there.
    h.get_by_label("[OWNER] - Go away.").click();
    h.run();
    let text = h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).next().unwrap();
    text.click();
    h.run();
    press(&mut h, egui::Key::End);
    typed(&mut h, "b");
    press(&mut h, egui::Key::Minus);
    press(&mut h, egui::Key::Backspace);
    typed(&mut h, "-x*");
    h.key_press(egui::Key::Tab);
    h.run();
    h.run();
    assert_eq!(selected(&h), go, "the selection stays");
    assert!(!folded(&h, 2));
    assert!(dialog_lines(&mut h, key).contains(&"Entry|Go away.-x*|".to_string()));
}

/// The conversation Aurora was watched typing in: an NPC's greeting with
/// three replies, the second answered by an NPC line beginning with "O"
/// with a reply beginning with "O" under it.
fn typing_dialog() -> Gff {
    use mg_module::dialog::{Kind, Parent, add_node, new_dialog};
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Goodbye.");
    let gold = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Gold, please.");
    let over = add_node(&mut g, Parent::Node(Kind::Reply, gold), "Over here.");
    add_node(&mut g, Parent::Node(Kind::Entry, over), "Off I go.");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Hmm.");
    g
}

/// Letters typed in the tree go by its rows as drawn, as Aurora's did
/// (watched): "o" goes to "Off I go.", not to the NPC's "Over here.", whose
/// row begins with `[OWNER]`. Typed in quick succession they go by all of
/// them ("gol": "Gold, please."), the same letter again to the next row
/// beginning with it, and a pause starts over.
#[test]
fn the_conversation_tree_s_letters_typed_quickly() {
    use mg_module::dialog::{Kind, Parent};
    use mg_ui::dialog_view::Row;
    let (mut h, key) = dialog_from_tree("ui-dialog-type-ahead", &typing_dialog());
    let selected = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
    let row = |parent: Parent, pos: usize| Some(Row { parent, pos });
    let (goodbye, gold) =
        (row(Parent::Node(Kind::Entry, 0), 0), row(Parent::Node(Kind::Entry, 0), 1));
    let off = row(Parent::Node(Kind::Entry, 1), 0);
    let hmm = row(Parent::Node(Kind::Entry, 0), 2);
    // (Each letter a step after the last: a quarter of a second.)
    let typed = |h: &mut Harness<'_, Moonglow>, t: &str| {
        for c in t.chars() {
            h.event(egui::Event::Text(c.to_string()));
        }
        h.run();
        h.run();
    };
    let home = |h: &mut Harness<'_, Moonglow>| {
        h.key_press(egui::Key::Home);
        h.run();
        h.run();
    };
    h.get_by_label("Root").click();
    h.run();
    typed(&mut h, "o");
    assert_eq!(selected(&h), off, "the row's text, its speaker first");
    home(&mut h);
    typed(&mut h, "h");
    assert_eq!(selected(&h), hmm);
    home(&mut h);
    typed(&mut h, "gol");
    assert_eq!(selected(&h), gold);
    home(&mut h);
    typed(&mut h, "gg");
    assert_eq!(selected(&h), gold, "the same letter: Goodbye, then Gold");
    home(&mut h);
    typed(&mut h, "g");
    assert_eq!(selected(&h), goodbye);
    h.run_steps(5);
    typed(&mut h, "o");
    assert_eq!(selected(&h), off, "after a pause, o alone");
}

/// A key the tree reads that a conversation command was given in Options
/// › Keyboard is the command's (a Mac's delete key is Backspace): Delete
/// Line on Backspace deletes the line, rather than going to the one it is
/// under, and Add Line on N adds one rather than looking for a line.
#[test]
fn a_conversation_command_on_a_tree_key_wins() {
    let (mut h, key) = dialog_from_tree("ui-dialog-key-command", &branching_dialog());
    let chosen = &mut h.state_mut().settings.key_bindings;
    chosen.insert("delete-line".into(), vec!["Backspace".into()]);
    chosen.insert("add-line".into(), vec!["N".into()]);
    h.run();
    h.get_by_label("Bye. [END DIALOGUE]").click();
    h.run();
    h.key_press(egui::Key::Backspace);
    h.run();
    h.run();
    assert!(!dialog_lines(&mut h, key).contains(&"  Reply|Bye.|".to_string()), "deleted");
    h.get_by_label("[OWNER] - Go away.").click();
    h.run();
    let n = |pressed| egui::Event::Key {
        key: egui::Key::N,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(n(true));
    h.event(egui::Event::Text("n".into()));
    h.event(n(false));
    h.run();
    h.run();
    assert!(h.state().dialog_views[&key].input.is_some(), "Add Line's popup");
}

/// Delete and the conversation's other commands work while its tree has
/// the keys, wherever the pointer is (Aurora's control with the focus has
/// them), not only with the pointer over the editor.
#[test]
fn conversation_commands_go_where_the_tree_s_keys_are() {
    let (mut h, key) = dialog_from_tree("ui-dialog-keys-commands", &branching_dialog());
    h.get_by_label("Bye. [END DIALOGUE]").click();
    h.run();
    // The pointer over the module tree, left of the editor.
    let tree = h.get_by_label_contains("Conversations").rect();
    let editor = h.get_by_label("Root").rect();
    assert!(tree.right() < editor.left(), "{tree:?} {editor:?}");
    h.hover_at(tree.center());
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    h.run();
    assert_eq!(h.state().confirm_delete, None, "not the module tree's Delete");
    assert!(!dialog_lines(&mut h, key).contains(&"  Reply|Bye.|".to_string()), "deleted");
}

/// Escape hands the tree's keys back, as the module tree's Escape does
/// (Aurora's Escape does nothing in its tree: watched); not the Escape
/// that takes the keyboard from the line's Text, after which the arrows
/// are the tree's again.
#[test]
fn escape_hands_the_conversation_tree_s_keys_back() {
    use mg_module::dialog::{Kind, Parent};
    use mg_ui::dialog_view::Row;
    let (mut h, key) = dialog_from_tree("ui-dialog-keys-escape", &branching_dialog());
    let selected = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
    let press = |h: &mut Harness<'_, Moonglow>, k: egui::Key| {
        h.key_press(k);
        h.run();
        h.run();
    };
    let traveller = Some(Row { parent: Parent::Node(Kind::Reply, 0), pos: 0 });
    let farewell = Some(Row { parent: Parent::Node(Kind::Entry, 1), pos: 0 });
    h.get_all_by_label("Who are you?").next().unwrap().click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).next().unwrap().click();
    h.run();
    assert!(h.ctx.text_edit_focused());
    press(&mut h, egui::Key::Escape);
    assert!(!h.ctx.text_edit_focused());
    press(&mut h, egui::Key::ArrowDown);
    assert_eq!(selected(&h), traveller, "the tree's keys still");
    press(&mut h, egui::Key::Escape);
    press(&mut h, egui::Key::ArrowDown);
    assert_eq!(selected(&h), traveller, "handed back");
    h.get_by_label("[OWNER] - A traveller.").click();
    h.run();
    press(&mut h, egui::Key::ArrowDown);
    assert_eq!(selected(&h), farewell, "a click gives them again");
}

/// Tab out of the line's Text goes to a button; an arrow then moves the
/// tree's selection, not egui's focus from widget to widget too: the keys
/// are the tree's.
#[test]
fn an_arrow_after_tab_moves_the_tree_alone() {
    use mg_module::dialog::{Kind, Parent};
    use mg_ui::dialog_view::Row;
    let (mut h, key) = dialog_from_tree("ui-dialog-keys-tab", &branching_dialog());
    let selected = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
    h.get_all_by_label("Who are you?").next().unwrap().click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).next().unwrap().click();
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    h.run();
    let focused = h.ctx.memory(|m| m.focused());
    assert!(focused.is_some() && !h.ctx.text_edit_focused(), "a button has the focus");
    h.key_press(egui::Key::ArrowDown);
    h.run();
    h.run();
    assert_eq!(selected(&h), Some(Row { parent: Parent::Node(Kind::Reply, 0), pos: 0 }));
    assert_eq!(h.ctx.memory(|m| m.focused()), None, "the focus went nowhere else");
}

/// Bookmarks and folded lines follow their lines across a change the
/// editor didn't make (a plugin's, the file changed outside Moonglow):
/// found again in it by what they say and the lines over them, a line
/// gone with its bookmark; undo brings it back.
#[test]
fn bookmarks_and_folded_lines_follow_their_lines_across_a_change_made_elsewhere() {
    use mg_module::dialog::{Kind, Parent, remove};
    let (e, r) = (Kind::Entry, Kind::Reply);
    let (mut h, key) = dialog_from_tree("ui-dialog-bookmarks-elsewhere", &branching_dialog());
    let marks = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].bookmarks.clone();
    let folds = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].collapsed.clone();
    for label in ["[OWNER] - Go away.", "Farewell. [END DIALOGUE]", "Bye. [END DIALOGUE]"] {
        h.get_by_label(label).click();
        h.run();
        h.get_by_label("Bookmark").click();
        h.run();
    }
    h.get_by_label("[OWNER] - Go away.").click();
    h.run();
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    h.run();
    assert_eq!((marks(&h), folds(&h)), (vec![(e, 2), (r, 1), (r, 2)], [(e, 2)].into()));
    // The traveller deleted by something else: Go away is entry 1, Bye
    // reply 1.
    let mut g = branching_dialog();
    remove(&mut g, Parent::Node(r, 0), 0);
    let edit = mg_edit::Edit::SetResource { key, data: g.to_bytes().ok() };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Plugin", vec![edit])));
    h.run();
    h.run();
    assert_eq!(marks(&h), [(e, 1), (r, 1)]);
    assert_eq!(folds(&h), [(e, 1)].into());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    h.run();
    assert_eq!(dialog_lines(&mut h, key), BRANCHING);
    assert_eq!(marks(&h), [(e, 2), (r, 1), (r, 2)]);
    assert_eq!(folds(&h), [(e, 2)].into());
}

/// Folding a branch with its − button while a line in it is selected
/// selects the line folded, as Windows' tree control does (the selection
/// was left out of sight, and the keys went on from Root); Collapse All
/// selects the first line it is under.
#[test]
fn folding_over_the_selection_selects_the_line_folded() {
    use mg_module::dialog::{Kind, Parent};
    use mg_ui::dialog_view::Row;
    let (mut h, key) = dialog_from_tree("ui-dialog-fold-selection", &branching_dialog());
    let selected = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].selected;
    let who = Some(Row { parent: Parent::Node(Kind::Entry, 0), pos: 0 });
    let hello = Some(Row { parent: Parent::Root, pos: 0 });
    h.get_by_label("Farewell. [END DIALOGUE]").click();
    h.run();
    // Who's − (the second: Hello's, Who's, the traveller's, Go's).
    h.get_all_by_label("−").nth(1).unwrap().click();
    h.run();
    h.run();
    assert!(h.state().dialog_views[&key].collapsed.contains(&(Kind::Reply, 0)));
    assert_eq!(selected(&h), who);
    // The keys go on from there: Down to Bye.
    h.key_press(egui::Key::ArrowDown);
    h.run();
    h.run();
    assert_eq!(selected(&h), Some(Row { parent: Parent::Node(Kind::Entry, 0), pos: 1 }));
    // A fold elsewhere leaves the selection.
    h.get_all_by_label("−").last().unwrap().click();
    h.run();
    h.run();
    assert!(h.state().dialog_views[&key].collapsed.contains(&(Kind::Entry, 2)));
    assert_eq!(selected(&h), Some(Row { parent: Parent::Node(Kind::Entry, 0), pos: 1 }));
    h.get_by_label("Expand All").click();
    h.run();
    h.get_by_label("[OWNER] - A traveller.").click();
    h.run();
    h.get_by_label("Collapse All").click();
    h.run();
    h.run();
    assert_eq!(selected(&h), hello);
}

/// Sacha's report: deleting a line numbered the lines after it anew, and
/// the editor's bookmarks and folded lines, kept by number, then named
/// other lines. They follow their lines: a bookmark on a line deleted
/// goes; undo brings it back, in its place, and redo takes it again.
#[test]
fn bookmarks_and_folded_lines_follow_their_lines_across_a_delete() {
    use mg_module::dialog::Kind;
    let (e, r) = (Kind::Entry, Kind::Reply);
    let (mut h, key) = dialog_from_tree("ui-dialog-bookmarks-delete", &branching_dialog());
    let marks = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].bookmarks.clone();
    let folds = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].collapsed.clone();
    // Go away (entry 2), Farewell (reply 1) and Bye (reply 2) bookmarked;
    // Go away folded (its link to Who hidden).
    for label in ["[OWNER] - Go away.", "Farewell. [END DIALOGUE]", "Bye. [END DIALOGUE]"] {
        h.get_by_label(label).click();
        h.run();
        h.get_by_label("Bookmark").click();
        h.run();
    }
    assert_eq!(marks(&h), [(e, 2), (r, 1), (r, 2)]);
    h.get_by_label("[OWNER] - Go away.").click();
    h.run();
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    h.run();
    assert_eq!(folds(&h), [(e, 2)].into());
    assert_eq!(h.get_all_by_label("Who are you?").count(), 1, "the link hidden");
    // The traveller deleted, Farewell with it: Go away is entry 1 now,
    // Bye reply 1.
    delete_line(&mut h, key, "[OWNER] - A traveller.", 0, DeleteBy::Key);
    h.run();
    assert_eq!(marks(&h), [(e, 1), (r, 1)]);
    assert_eq!(folds(&h), [(e, 1)].into());
    // (Who has nothing under it now: the conversation ends there.)
    h.get_by_label("Who are you? [END DIALOGUE]");
    assert!(h.query_by_label("Who are you?").is_none(), "the link still hidden");
    h.get_by_label("Bookmarks").click();
    h.run();
    h.get_by_label("Entry 1: Go away.");
    h.get_by_label("Reply 1: Bye.");
    assert!(h.query_by_label_contains("Farewell").is_none());
    // Undone: as they were, in their order; redone: as after the delete.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    h.run();
    assert_eq!(dialog_lines(&mut h, key), BRANCHING);
    assert_eq!(marks(&h), [(e, 2), (r, 1), (r, 2)]);
    assert_eq!(folds(&h), [(e, 2)].into());
    h.get_by_label("Reply 1: Farewell.");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run();
    h.run();
    assert_eq!(marks(&h), [(e, 1), (r, 1)]);
    assert_eq!(folds(&h), [(e, 1)].into());
}

/// A letter typed to go to a line (the conversation tree's) or a row (the
/// module tree's, while it has the keys) moved the camera of the area view
/// the pointer was in last too: W, A, S, D, Z, C and the keypad's digits
/// are its keys wherever the pointer is. A tree that takes a letter has
/// it alone; W moves the camera again once the area's view has the keys.
#[test]
fn a_letter_typed_in_a_tree_leaves_the_area_camera() {
    use mg_module::dialog::{Kind, Parent};
    use mg_ui::dialog_view::Row;
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let prepare = |m: &mut Module, _: &mg_rules::GameData, _: ResRef| {
        m.set_gff(key, &branching_dialog()).unwrap();
    };
    let Some((mut h, area)) = area_harness_with("tree-letters", "ttr01", None, 4, None, &prepare)
    else {
        return;
    };
    let target = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().target;
    // W held a few frames, as typed: its key, then its letter.
    let hold = |h: &mut Harness<'_, Moonglow>, key: egui::Key, text: &str| {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        h.event(event(true));
        h.event(egui::Event::Text(text.into()));
        h.run_steps(4);
        h.event(event(false));
        h.run_steps(1);
    };
    let centre = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(centre);
    h.run_steps(2);
    let start = target(&h);
    // The conversation opened, clicked in: its tree has the keys.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    h.run_steps(3);
    h.get_by_label("Root").click();
    h.run_steps(2);
    hold(&mut h, egui::Key::W, "w");
    let who = Some(Row { parent: Parent::Node(Kind::Entry, 0), pos: 0 });
    assert_eq!(h.state().dialog_views[&key].selected, who, "the tree's W");
    assert_eq!(target(&h), start, "not the camera's");
    // The module tree, clicked, the pointer over it: its W.
    h.get_by_label_contains("Conversations").click();
    h.run_steps(2);
    let row = h.get_by_label_contains("Conversations").rect().center();
    h.hover_at(row);
    h.run_steps(2);
    hold(&mut h, egui::Key::W, "w");
    assert_eq!(target(&h), start, "the module tree's, not the camera's");
    // The pointer over the conversation (its window over the area): the
    // tree still has the keys, and W.
    h.hover_at(centre);
    h.run_steps(2);
    hold(&mut h, egui::Key::W, "w");
    assert_eq!(target(&h), start, "the module tree's, the pointer elsewhere");
    // Back in the area's view (the conversation closed): W is the
    // camera's.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Dialog(key)));
    h.run_steps(3);
    h.hover_at(centre);
    h.run_steps(2);
    hold(&mut h, egui::Key::W, "w");
    assert!((target(&h) - start).length() > 0.1, "W moves the view");
}

/// GitHub issue 18, in the conversation editor: Ctrl+Z with a line's text
/// typed in is the text's own, not the module's Undo (the conversation's
/// earlier change stays); the field left, it is the module's again.
#[test]
fn undo_in_a_conversation_line_s_text_is_the_text_s() {
    let (mut h, key) = dialog_from_tree("ui-dialog-undo-in-text", &branching_dialog());
    // An earlier change of the conversation: Delete on a line.
    delete_line(&mut h, key, "Bye. [END DIALOGUE]", 0, DeleteBy::Key);
    let earlier = dialog_lines(&mut h, key);
    assert_eq!(earlier.len(), 6);
    h.get_by_label("[OWNER] - Go away.").click();
    h.run();
    let text = egui::accesskit::Role::MultilineTextInput;
    h.get_all_by_role(text).next().unwrap().click();
    h.run();
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text(" Now".into()));
    h.run_steps(2);
    assert!(h.ctx.egui_wants_keyboard_input());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run_steps(2);
    let lines = dialog_lines(&mut h, key);
    assert_eq!(lines.len(), 6, "the delete is not undone: {lines:?}");
    // The field left: Ctrl+Z is the module's, back past the delete.
    h.get_by_label("Root").click();
    h.run_steps(2);
    assert!(!h.ctx.egui_wants_keyboard_input());
    for _ in 0..3 {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(2);
    }
    assert_eq!(dialog_lines(&mut h, key), BRANCHING, "the module's undo, down to the delete");
}

/// The window holds still as a conversation's line is switched to its
/// Actions Taken page (Troubletcat's report after 3.1.0: the jittering
/// came back there). At 125%, at some sizes, the area view beside it, in
/// egui_dock's scroll area, never settled: its scroll bars came and went
/// every frame. At each size of a sweep and the display scales builders
/// have, the window ends its repaints and its controls stay put.
#[test]
fn a_line_s_actions_taken_page_holds_still() {
    use mg_module::dialog::{Parent, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("ui-dialog-actions-still");
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    add_node(&mut g, Parent::Root, "Hello there, traveller. What brings you to these parts?");
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut moving = Vec::new();
    for scale in [1.0, 1.25, 1.5] {
        let mut app = app_with(Vec::new());
        app.open_module(&path);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
        let mut h = Harness::builder()
            .with_size(egui::vec2(1200.0, 900.0))
            .with_pixels_per_point(scale)
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        h.run_steps(6);
        h.get_by_label_contains("Hello there, traveller").click();
        h.run_steps(3);
        h.get_by_label("Actions Taken").click();
        h.run_steps(3);
        for w in (6000..=13000).step_by(431) {
            for ht in (5000..=9500).step_by(377) {
                let size = egui::vec2(w as f32 / 10.0, ht as f32 / 10.0);
                h.set_size(size);
                h.run_steps(10);
                // Switched to the page: it settles (its repaints end) and
                // stays where it settled.
                h.get_by_label("Text Appears When…").click();
                h.run_steps(3);
                h.get_by_label("Actions Taken").click();
                let settled = (0..3).any(|_| h.try_run().is_ok());
                let mut at = Vec::new();
                for _ in 0..6 {
                    h.run_steps(1);
                    let rects: Vec<egui::Rect> = (h.get_all_by_role(egui::accesskit::Role::Button))
                        .map(|n| n.rect())
                        .collect();
                    at.push(rects);
                }
                if !settled || at.windows(2).any(|p| p[0] != p[1]) {
                    moving.push(format!("x{scale} at {size:?}: settled {settled}"));
                }
            }
        }
    }
    assert!(moving.is_empty(), "{} sizes moved:\n{}", moving.len(), moving.join("\n"));
}
