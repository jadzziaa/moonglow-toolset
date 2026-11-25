//! The journal and the faction editor (M4's U22; the Journal and Factions
//! boards). The journal: its header (Add, Copy, Cut, Paste, Delete, Expand
//! All, Collapse All; OK and Cancel), its tree, the Right Dock's Entry and
//! Category following the choice, what is typed in them following its
//! category, their edits one Undo step each and set back by Cancel. The
//! faction editor: its header (Basic | Advanced, Add Faction…, Remove
//! Faction, Change Name…), the matrix (a click chooses a cell, the arrows
//! move, a number typed and Enter set it, a drag changes it: one step
//! each; Tab is not bound; no key while a window or popover is open), the
//! Faction, Reaction and Reactions panels, the Outliner's factions and its
//! keys. Both: the hints and counts, the window coming to rest, the file
//! read once a change (not once a frame) and only the cells in sight laid
//! out with 100 factions, a floating window's first size, the panels
//! following the editor in front, records from before read without a
//! fault.

use crate::common::*;
use mg_module::factions::Factions;
use mg_ui::faction_view::{FACTION_HEADER, MATRIX};
use mg_ui::journal_view::{JOURNAL_HEADER, JOURNAL_TREE, Node};

/// The journal's key.
fn jrl() -> ResKey {
    ResKey::parse("module", ResType::JRL).unwrap()
}

/// A module of ours: a journal of two categories (Quest A, tag
/// Category000, with Entry001 and Entry002; Quest B, tag Category001,
/// with Entry001), and the standard
/// factions with Guards and Thieves (Hostile's children) added, `more`
/// factions of the module's own after them.
fn module_of_ours(name: &str, more: usize) -> std::path::PathBuf {
    use mg_module::journal::{entries, list_value, new_category, new_entry, new_journal};
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut g = new_journal();
    let mut cats = Vec::new();
    for (name, n) in [("Quest A", 2), ("Quest B", 1)] {
        let mut cat = new_category(&cats);
        let name = mg_core::LocString::from_text(
            mg_core::Language::ENGLISH,
            mg_core::Gender::Male,
            name.as_bytes().to_vec(),
        );
        cat.write(&mg_schema::jrl::categories::NAME, name);
        let mut list = Vec::new();
        for _ in 0..n {
            let e = new_entry(&list);
            list.push(e);
        }
        cat.set("EntryList", list_value(list));
        assert_eq!(entries(&cat).len(), n);
        cats.push(cat);
    }
    g.root.set("Categories", list_value(cats));
    m.set_gff(jrl(), &g).unwrap();
    let mut f = Factions::of_module(&m);
    f.add("Guards", true, 1);
    f.add("Thieves", true, 1);
    for i in 0..more {
        f.add(&format!("Band {i:03}"), true, 2);
    }
    m.set_gff(ResKey::parse("repute", ResType::FAC).unwrap(), &f.to_gff()).unwrap();
    m.save().unwrap();
    path
}

/// Our module open in `mode` in a window `width` by 900, `tab` opened and
/// in front, the window at rest.
fn opened(mode: Mode, name: &str, width: f32, tab: Tab, more: usize) -> Harness<'static, Moonglow> {
    let path = module_of_ours(&mode.dir(name), more);
    let mut app = app_in(mode, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    close_tab(&mut app, &Tab::ModuleProperties);
    let mut h = harness(app, width, 900.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
    settle(&mut h);
    h
}

/// The journal's categories as the workspace has them.
fn categories(h: &mut Harness<'_, Moonglow>) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&jrl()).unwrap().root.list("Categories").unwrap().to_vec()
}

/// The factions as the workspace has them.
fn factions(h: &mut Harness<'_, Moonglow>) -> Factions {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    Factions::of_module(&ws.module)
}

fn undo_depth(h: &Harness<'_, Moonglow>) -> usize {
    h.state().ws.as_ref().unwrap().undo_depth()
}

/// The tree's row named `label` (a category's "Quest A [Category000]",
/// an entry's "[0001] Entry001").
fn tree_row<'t>(h: &'t Harness<'_, Moonglow>, label: &'t str) -> egui_kittest::Node<'t> {
    tree(h).get_by_label(label)
}

/// The tree's entry row whose label begins with `start` (its ID and the
/// start of its text: the row shows what is typed once it is set).
fn entry_row<'t>(h: &'t Harness<'_, Moonglow>, start: &'t str) -> egui_kittest::Node<'t> {
    let found = tree(h).query_all_by_label_contains(start).next();
    found.unwrap_or_else(|| panic!("the tree's row {start:?}…"))
}

/// The journal's tree (named "Journal" as the board's is, as its tab is).
fn tree<'t>(h: &'t Harness<'_, Moonglow>) -> egui_kittest::Node<'t> {
    let found = h.query_all_by_label(JOURNAL_TREE).find(|n| n.query_by_label("Root").is_some());
    found.expect("the journal's tree")
}

/// Category000 opened and its second entry chosen by the pointer.
fn choose_second_entry(h: &mut Harness<'_, Moonglow>) {
    h.get_by_label("Expand Quest A").click();
    settle(h);
    tree_row(h, "[0002] Entry002").click();
    settle(h);
}

/// The center of the matrix's cell of `who` toward `whom`, whatever its
/// value.
fn cell(h: &Harness<'_, Moonglow>, who: &str, whom: &str) -> egui::Pos2 {
    let toward = format!(" toward {whom}");
    let found = h
        .get_by_label(MATRIX)
        .query_all_by_label_contains(&toward)
        .find(|n| n.accesskit_node().label().is_some_and(|l| l.starts_with(&format!("{who} is "))));
    found.unwrap_or_else(|| panic!("no cell of {who} toward {whom}")).rect().center()
}

/// The faction editor's chosen cell: its row's faction and its column's.
fn chosen(h: &Harness<'_, Moonglow>) -> (u32, u32) {
    let v = &h.state().faction_view;
    (v.selected, v.target)
}

/// The journal's header holds its controls (the Journal board's), OK and
/// Cancel at its end; the Entry and Category panels follow the choice
/// (their subtitles "[0002] of Category000" and "Category000"), each
/// saying what to choose while nothing is chosen; Finish
/// category clicked is one Undo step, the tree's row then saying the entry
/// finishes its category, and the Top Bar's Undo, with the pointer over
/// the panel, takes it back.
#[test]
fn the_journal_s_header_and_panels_follow_the_choice() {
    let mut h = opened(Mode::Tabs, "ui-u22-journal-panels", 1600.0, Tab::Journal, 0);
    let controls = ["Add", "Copy", "Cut", "Paste", "Delete", "Expand All", "Collapse All"];
    header_holds(&mut h, JOURNAL_HEADER, &controls);
    for end in ["OK", "Cancel"] {
        assert!(h.get_by_label(JOURNAL_HEADER).query_by_label(end).is_some(), "{end}");
    }
    assert!(h.query_by_label_contains("Choose an entry").is_some(), "the Entry panel says so");
    // (Nothing chosen, the Category panel too: the moves file's "Select a
    // category or an entry" row.)
    let said = "Choose a category or an entry. Add with nothing chosen adds a category.";
    assert!(h.query_by_label(said).is_some(), "the Category panel says so");
    choose_second_entry(&mut h);
    assert_eq!(h.state().journal_view.selected, Some(Node::Entry(0, 1)));
    assert!(h.query_by_label_contains("[0002] of Category000").is_some(), "Entry's subtitle");
    let before = undo_depth(&h);
    h.get_by_label("Finish category").click();
    settle(&mut h);
    assert_eq!(undo_depth(&h), before + 1, "one step");
    let end = |h: &mut Harness<'_, Moonglow>| {
        mg_module::journal::entries(&categories(h)[0])[1].get("End").cloned()
    };
    assert_eq!(end(&mut h), Some(mg_gff::Value::Word(1)));
    assert!(tree_row(&h, "[0002] Entry002, finishes the category").rect().is_positive());
    h.hover_at(h.get_by_label("Finish category").rect().center());
    settle(&mut h);
    h.get_by_label("Top Bar").get_by_label("Undo").click();
    settle(&mut h);
    assert_eq!(end(&mut h), Some(mg_gff::Value::Word(0)), "taken back");
    assert_eq!(undo_depth(&h), before);
}

/// Expand All opens every category, Collapse All closes them; a closed
/// category's row says how many entries it holds.
#[test]
fn expand_all_and_collapse_all_open_and_close_every_category() {
    let mut h = opened(Mode::Tabs, "ui-u22-journal-fold", 1600.0, Tab::Journal, 0);
    assert!(h.query_by_label("Expand Quest A").is_some(), "closed at first");
    h.get_by_label(JOURNAL_HEADER).get_by_label("Expand All").click();
    settle(&mut h);
    assert_eq!(h.state().journal_view.open.len(), 2, "both open");
    assert!(h.query_by_label("Collapse Quest A").is_some(), "its arrow says so");
    assert_eq!(tree(&h).query_all_by_label("[0001] Entry001").count(), 2);
    h.get_by_label(JOURNAL_HEADER).get_by_label("Collapse All").click();
    settle(&mut h);
    assert!(h.state().journal_view.open.is_empty(), "both closed");
    assert_eq!(tree(&h).query_all_by_label("[0001] Entry001").count(), 0);
}

/// What is typed into Category001's Tag, not yet set, follows the
/// category as Category000 is deleted before it (elsewhere: a command of
/// another window's): Enter sets it on Category001, now the first, and
/// on nothing else.
#[test]
fn typed_text_follows_its_category() {
    let mut h = opened(Mode::Tabs, "ui-u22-journal-typed", 1600.0, Tab::Journal, 0);
    tree_row(&h, "Quest B [Category001]").click();
    settle(&mut h);
    let field = (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| n.accesskit_node().value().as_deref() == Some("Category001"))
        .expect("the Tag field")
        .rect()
        .center();
    click_pos(&h, field);
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    settle(&mut h);
    let focused = (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| n.accesskit_node().is_focused())
        .expect("typing in the Tag field");
    focused.type_text("QUEST");
    settle(&mut h);
    let mut cats = categories(&mut h);
    cats.remove(0);
    let gone = mg_edit::Edit::SetField {
        key: jrl(),
        path: mg_edit::GffPath::root(),
        label: "Categories".into(),
        value: Some(mg_module::journal::list_value(cats)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Gone", vec![gone])));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let cats = categories(&mut h);
    assert_eq!(cats.len(), 1, "Category000 deleted");
    let tag = cats[0].read(&mg_schema::jrl::categories::TAG);
    assert_eq!(tag.as_bytes(), b"QUEST", "typed into Category001, set on it");
    // Typed after a move: both steps taken back (Quest B second again,
    // chosen still), its Tag typed in and set on it.
    for _ in 0..2 {
        h.state_mut().actions.push(mg_ui::Action::Undo);
        settle(&mut h);
    }
    assert_eq!(h.state().journal_view.selected, Some(Node::Category(1)), "followed back");
    let field = (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| n.accesskit_node().value().as_deref() == Some("Category001"))
        .expect("the Tag field")
        .rect()
        .center();
    click_pos(&h, field);
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("AFTER".into()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let tags: Vec<Vec<u8>> = (categories(&mut h).iter())
        .map(|c| c.read(&mg_schema::jrl::categories::TAG).as_bytes().to_vec())
        .collect();
    assert_eq!(tags, [b"Category000".to_vec(), b"AFTER".to_vec()], "on Quest B alone");
}

/// An entry's Text typed and not yet set follows its entry: Entry001 deleted
/// before it (Entry002 now the first), and that taken back; no Redo while
/// it is pending (`no_redo_is_offered_while_journal_text_is_pending`);
/// OK sets it on Entry002 without Enter. A category's Tag typed and the
/// tab closed is set too.
#[test]
fn typed_text_follows_its_entry_and_is_set_on_ok_and_closing() {
    let mut h = opened(Mode::Tabs, "ui-u22-journal-entry-typed", 1600.0, Tab::Journal, 0);
    choose_second_entry(&mut h);
    type_into_text(&mut h, "Entry002", " typed");
    assert_eq!(entry_text(&mut h, 0, 1), "Entry002", "pending, not set yet");
    // (Entry001 deleted, as the header's Delete deletes it: the category's
    // entry list set whole.)
    let mut list = mg_module::journal::entries(&categories(&mut h)[0]).to_vec();
    list.remove(0);
    let gone = mg_edit::Edit::SetField {
        key: jrl(),
        path: mg_edit::GffPath::root().item("Categories", 0),
        label: "EntryList".into(),
        value: Some(mg_module::journal::list_value(list)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Gone", vec![gone])));
    let count =
        |h: &mut Harness<'_, Moonglow>| mg_module::journal::entries(&categories(h)[0]).len();
    until(&mut h, "Entry001 gone", |h| count(h) == 1);
    assert_eq!(h.state().journal_view.selected, Some(Node::Entry(0, 0)), "followed");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    until(&mut h, "Entry001 back", |h| count(h) == 2);
    assert_eq!(h.state().journal_view.selected, Some(Node::Entry(0, 1)), "followed back");
    assert_eq!(focused_text(&h).as_deref(), Some("Entry002 typed"), "still typed in");
    h.get_by_label(JOURNAL_HEADER).get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Journal).is_none(), "closed");
    assert_eq!(all_texts(&mut h), [vec!["Entry001", "Entry002 typed"], vec!["Entry001"]]);
    // A category's Tag typed, the tab closed: set.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Journal));
    settle(&mut h);
    tree_row(&h, "Quest B [Category001]").click();
    settle(&mut h);
    let field = (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| n.accesskit_node().value().as_deref() == Some("Category001"))
        .expect("the Tag field")
        .rect()
        .center();
    click_pos(&h, field);
    until(&mut h, "the Tag has the keyboard", |h| {
        focused_text(h).as_deref() == Some("Category001")
    });
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("CLOSED".into()));
    until(&mut h, "typed", |h| focused_text(h).as_deref() == Some("CLOSED"));
    close_tab(h.state_mut(), &Tab::Journal);
    settle(&mut h);
    let tag = categories(&mut h)[1].read(&mg_schema::jrl::categories::TAG);
    assert_eq!(tag.as_bytes(), b"CLOSED", "set on closing");
}

/// No Redo is offered while an Entry or Category field's typing waits to
/// be set (the ruling: the side editors' rule): the Top Bar's Redo off
/// (it had set the text and lost the step to redo, the review's probe 1),
/// the Status Bar's hint and the key doing nothing; clicked, it leaves the
/// field (set, one step) and redoes nothing. A number field (XP) with the
/// keyboard likewise. Before and after, with nothing typed, Redo is on.
#[test]
fn no_redo_is_offered_while_journal_text_is_pending() {
    let redo_off = |h: &Harness<'_, Moonglow>| {
        h.get_by_label("Top Bar").get_by_label("Redo").accesskit_node().is_disabled()
    };
    let end = |h: &mut Harness<'_, Moonglow>| {
        mg_module::journal::entries(&categories(h)[0])[1].get("End").cloned()
    };
    let mut h = opened(Mode::Tabs, "ui-u22-fix-redo", 1600.0, Tab::Journal, 0);
    choose_second_entry(&mut h);
    h.get_by_label("Finish category").click();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    until(&mut h, "Finish category undone", |h| end(h) == Some(mg_gff::Value::Word(0)));
    assert!(!redo_off(&h), "Redo on, nothing typed");
    type_into_text(&mut h, "Entry002", " typed");
    assert!(redo_off(&h), "Redo off while the text is pending");
    assert!(!h.state().status_said().iter().any(|l| l.contains("Redo")), "no hint");
    let depth = undo_depth(&h);
    for (mods, key) in [
        (egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z),
        (egui::Modifiers::COMMAND, egui::Key::Y),
    ] {
        h.key_press_modifiers(mods, key);
        settle(&mut h);
    }
    h.state_mut().actions.push(mg_ui::Action::Redo);
    settle(&mut h);
    assert_eq!(undo_depth(&h), depth, "the keys and a Redo asked for redo nothing");
    assert_eq!(end(&mut h), Some(mg_gff::Value::Word(0)));
    assert_eq!(focused_text(&h).as_deref(), Some("Entry002 typed"), "still typed in");
    h.get_by_label("Top Bar").get_by_label("Redo").click();
    settle(&mut h);
    assert_eq!(end(&mut h), Some(mg_gff::Value::Word(0)), "nothing redone");
    assert_eq!(entry_text(&mut h, 0, 1), "Entry002 typed", "the field left: set");
    assert_eq!(undo_depth(&h), depth + 1, "one step");
    // A number field with the keyboard: XP.
    tree_row(&h, "Quest A [Category000]").click();
    settle(&mut h);
    set_number(&h, &number_beside(&h, "XP", 0), 500.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    assert!(!redo_off(&h), "Redo on: the XP to set again");
    click_pos(&h, number_beside(&h, "XP", 0).rect().center());
    until(&mut h, "XP has the keyboard", |h| h.ctx.memory(|m| m.focused().is_some()));
    h.event(egui::Event::Text("7".into()));
    settle(&mut h);
    assert!(redo_off(&h), "Redo off while XP is typed in");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(!redo_off(&h) || h.state().ws.as_ref().unwrap().can_redo().is_none(), "on again");
}

/// The journal written whole by a plugin's command (a job) while an
/// entry's Text is typed in: what is typed is set as the job begins, on
/// the entry typed into, so the plugin reads it and its write keeps it
/// there (Entry001 removed before it; the review's finding 10). The
/// selection follows the entry by its category's Tag and its ID; where
/// the ID is not unique (a pasted copy keeps it) the selection goes, and
/// the typing, set before, stays on its entry. (Set only as the job's
/// window took the keyboard, it was replaced by the plugin's write: CI's
/// run 45640613492.)
#[test]
fn typed_text_follows_its_entry_through_a_whole_write() {
    let dir = mg_testkit::scratch_dir("ui-u22-fix-whole-plugins");
    let plugin = dir.join("drop");
    std::fs::create_dir_all(&plugin).unwrap();
    let cfg = "[plugin]\nid = \"test.drop-entry\"\nname = \"Drop Entry\"\nversion = \"1\"\n\
               api = \"0.1\"\nlicense = \"MIT\"\n\n[command]\nid = \"drop\"\n\
               title = \"Drop the First Entry\"\n";
    std::fs::write(plugin.join("plugin.cfg"), cfg).unwrap();
    let luau = "local mg = require(\"@moonglow\")\n\
                mg.command(\"drop\", function(ctx)\n\
                \x20   local doc = ctx.module:raw(\"module.jrl\")\n\
                \x20   local n = 0 for i = 1, 30000000 do n = n + i end\n\
                \x20   table.remove(doc.Categories.value[1].EntryList.value, 1)\n\
                \x20   ctx.edit:write_gff(\"module.jrl\", doc)\n\
                end)\n";
    std::fs::write(plugin.join("main.luau"), luau).unwrap();
    for (case, paste) in [("found", false), ("dropped", true)] {
        let mut h =
            opened(Mode::Tabs, &format!("ui-u22-fix-whole-{case}"), 1600.0, Tab::Journal, 0);
        h.state_mut().plugin_dir = Some(dir.clone());
        h.state_mut().load_plugins();
        h.state_mut().enable_plugin("test.drop-entry", true);
        h.state_mut().background_jobs = true;
        settle(&mut h);
        // Quest A: Entry001, Entry002 and a third after them (Add), or a
        // copy of Entry002 (Copy, Paste: its ID kept).
        choose_second_entry(&mut h);
        if paste {
            h.get_by_label(JOURNAL_HEADER).get_by_label("Copy").click();
            settle(&mut h);
            h.get_by_label(JOURNAL_HEADER).get_by_label("Paste").click();
        } else {
            tree_row(&h, "Quest A [Category000]").click();
            settle(&mut h);
            h.get_by_label(JOURNAL_HEADER).get_by_label("Add").click();
        }
        settle(&mut h);
        entry_row(&h, "[0002] Entry002").click();
        settle(&mut h);
        let before = all_texts(&mut h);
        type_into_text(&mut h, "Entry002", " typed");
        // (Run while the Text keeps the keyboard: a command run by its key
        // or menu leaves the field first, a plugin's event does not. The
        // plugin takes a while, so that its window takes the keyboard from
        // the Text while it runs.)
        h.state_mut().run_plugin_command("test.drop-entry", "drop");
        until(&mut h, "the plugin done", |h| !h.state().busy());
        settle(&mut h);
        let mut want = before.clone();
        want[0].remove(0);
        // (Set as the job began, on the entry typed into.)
        want[0][0] = "Entry002 typed".into();
        assert_eq!(all_texts(&mut h), want, "{case}: on Entry002 alone");
        let selected = h.state().journal_view.selected;
        match paste {
            true => assert_eq!(selected, None, "{case}: not to be told"),
            false => assert_eq!(selected, Some(Node::Entry(0, 0)), "{case}: followed"),
        }
    }
}

/// The factions written whole (as a plugin's write or a module-wide Find
/// and Replace writes repute.fac) while a value is typed in the Reaction
/// panel. A typed value is set as it is typed (egui's DragValue sets it at
/// each key), so none waits to be set; but the field's typing goes on
/// after the write: the chosen faction, Guards, removed, Thieves takes its
/// place, and a key typed then would make "95" of the field's "9" and set
/// it on Thieves. The field's typing ends with its pair (the review's
/// finding 10, the Reaction panel's field): nothing set after the write.
#[test]
fn a_reaction_value_typed_as_the_factions_are_written_whole_goes_no_further() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-whole-factions", 1600.0, Tab::Factions, 1);
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    click_pos(&h, cell(&h, "Guards", "Band 000"));
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 7));
    let value = h.get_by_label("Reputation value").rect().center();
    click_pos(&h, value);
    until(&mut h, "the value has the keyboard", |h| h.ctx.memory(|m| m.focused().is_some()));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("9".into()));
    settle(&mut h);
    assert_eq!(factions(&mut h).reputation(5, 7), Some(9), "set as typed");
    let mut staged = factions(&mut h);
    let _ = staged.remove(5).expect("Guards removed");
    let key = ResKey::new(ResRef::from_str("repute").unwrap(), ResType::FAC);
    let set = mg_edit::Edit::SetResource { key, data: Some(staged.to_gff().to_bytes().unwrap()) };
    let depth = undo_depth(&h);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Written", vec![set])));
    until(&mut h, "the write applied", |h| undo_depth(h) > depth);
    assert_eq!(chosen(&h), (5, 6), "Thieves toward Band 000, in Guards' place");
    h.event(egui::Event::Text("5".into()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let now = factions(&mut h);
    let n = now.factions.len() as u32;
    for p in 1..n {
        for t in 0..n {
            assert_eq!(now.reputation(p, t), staged.reputation(p, t), "({p}, {t}) as written");
        }
    }
}

/// What is typed into an entry's Text follows it, typed after each move:
/// a cut of the entry before it, a paste after it, another entry chosen
/// and chosen back, Collapse All (the entry's category closed: its Text
/// set as it goes), the module saved by its key (set before it is written).
#[test]
fn typed_text_follows_its_entry_through_cut_paste_choice_and_saving() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-follow-more", 1600.0, Tab::Journal, 0);
    choose_second_entry(&mut h);
    // Entry001 cut: Entry002 the first.
    tree_row(&h, "[0001] Entry001").click();
    settle(&mut h);
    h.get_by_label(JOURNAL_HEADER).get_by_label("Cut").click();
    settle(&mut h);
    entry_row(&h, "[0002] Entry002").click();
    settle(&mut h);
    type_into_text(&mut h, "Entry002", " a");
    // Pasted (into the category, after it): Entry001 back, at the end.
    h.get_by_label(JOURNAL_HEADER).get_by_label("Paste").click();
    settle(&mut h);
    assert_eq!(all_texts(&mut h)[0], ["Entry002 a", "Entry001"], "set as the field was left");
    entry_row(&h, "[0002] Entry002").click();
    settle(&mut h);
    type_into_text(&mut h, "Entry002 a", " b");
    // Another entry chosen: set as it is left, on Entry002.
    tree_row(&h, "[0001] Entry001").click();
    settle(&mut h);
    assert_eq!(all_texts(&mut h)[0], ["Entry002 a b", "Entry001"], "another chosen");
    entry_row(&h, "[0002] Entry002").click();
    settle(&mut h);
    type_into_text(&mut h, "Entry002 a b", " c");
    // Collapse All: the category closed, its entry no longer chosen.
    h.get_by_label(JOURNAL_HEADER).get_by_label("Collapse All").click();
    settle(&mut h);
    assert_eq!(all_texts(&mut h)[0], ["Entry002 a b c", "Entry001"], "Collapse All");
    h.get_by_label(JOURNAL_HEADER).get_by_label("Expand All").click();
    settle(&mut h);
    entry_row(&h, "[0002] Entry002").click();
    settle(&mut h);
    type_into_text(&mut h, "Entry002 a b c", " d");
    // Saved by its key: set first, then written with the module.
    let path =
        h.state().ws.as_ref().unwrap().module.location.as_ref().unwrap().path().to_path_buf();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    settle(&mut h);
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "saved");
    let m = Module::open(&path).unwrap();
    let g = m.gff(&jrl()).unwrap().unwrap();
    let cats = g.root.list("Categories").unwrap().to_vec();
    let text = mg_module::journal::entries(&cats[0])[0]
        .read(&mg_schema::jrl::categories::entry_list::TEXT);
    let (en, male) = (mg_core::Language::ENGLISH, mg_core::Gender::Male);
    let text = text.text_in(en, male, mg_core::Codepage::default()).unwrap_or_default();
    assert_eq!(text, "Entry002 a b c d", "set and saved before the module closed");
}

/// Cancel sets back what the panels changed: Category000's XP set in the
/// Category panel is gone again, and the tab closed.
#[test]
fn cancel_sets_back_the_panels_edits() {
    let mut h = opened(Mode::Tabs, "ui-u22-journal-cancel", 1600.0, Tab::Journal, 0);
    tree_row(&h, "Quest A [Category000]").click();
    settle(&mut h);
    let before = undo_depth(&h);
    set_number(&h, &number_beside(&h, "XP", 0), 500.0);
    settle(&mut h);
    assert_eq!(categories(&mut h)[0].get("XP"), Some(&mg_gff::Value::Dword(500)));
    assert_eq!(undo_depth(&h), before + 1, "one step");
    h.get_by_label(JOURNAL_HEADER).get_by_label("Cancel").click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Journal).is_none(), "closed");
    assert_eq!(categories(&mut h)[0].get("XP"), Some(&mg_gff::Value::Dword(0)), "set back");
}

/// The journal's hints and counts (the Journal board's): over the tree,
/// Click Select and Double Click Open or close a category; elsewhere none;
/// none while a field is typed in; never Save. "2 categories" and "3
/// entries" always.
#[test]
fn the_journal_hints_and_counts_are_as_bound() {
    let mut h = opened(Mode::Tabs, "ui-u22-journal-hints", 1600.0, Tab::Journal, 0);
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said();
    h.hover_at(tree_row(&h, "Quest A [Category000]").rect().center());
    settle(&mut h);
    let s = said(&h);
    for w in ["Click: Select", "Double Click: Open or close a category"] {
        assert!(s.contains(&w.to_string()), "{w}: {s:?}");
    }
    for w in ["2 categories", "3 entries"] {
        assert!(s.contains(&w.to_string()), "{w}: {s:?}");
    }
    assert!(!s.iter().any(|l| l.contains("Save")), "{s:?}");
    h.hover_at(left_dock_rect(&h).center());
    settle(&mut h);
    let s = said(&h);
    assert!(!s.iter().any(|l| l.contains("Click")), "elsewhere: {s:?}");
    assert!(s.contains(&"3 entries".to_string()), "{s:?}");
    // Typing in a field: none.
    tree_row(&h, "Quest A [Category000]").click();
    settle(&mut h);
    let field = (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| n.accesskit_node().value().as_deref() == Some("Category000"))
        .expect("the Tag field");
    field.click();
    settle(&mut h);
    h.hover_at(tree_row(&h, "Quest A [Category000]").rect().center());
    settle(&mut h);
    assert!(!said(&h).iter().any(|l| l.contains("Click")), "typing: {:?}", said(&h));
}

/// The journal comes to rest and is read once a change, not once a frame:
/// after it opens, a category opened, an entry chosen, a change; frames
/// at rest read it no more, the last check that the change was read.
#[test]
fn the_journal_comes_to_rest_and_is_read_once_a_change() {
    let mut h = opened(Mode::Tabs, "ui-u22-journal-rest", 1600.0, Tab::Journal, 0);
    let walks = |h: &Harness<'_, Moonglow>| h.state().journal_view.walks();
    let steady = |h: &mut Harness<'_, Moonglow>, what: &str| {
        settle(h);
        let was = walks(h);
        h.run_steps(10);
        assert_eq!(walks(h), was, "{what}: read again with nothing changed");
    };
    steady(&mut h, "opened");
    choose_second_entry(&mut h);
    steady(&mut h, "an entry chosen");
    let was = walks(&h);
    h.get_by_label("Finish category").click();
    steady(&mut h, "changed");
    assert!(walks(&h) > was, "the change read");
    assert!(tree_row(&h, "[0002] Entry002, finishes the category").rect().is_positive());
}

/// The Right Dock follows the editor in front: the journal's Entry and
/// Category beside it, the faction editor's Faction, Reaction and
/// Reactions beside it; floating, each window holds its own.
#[test]
fn the_panels_follow_the_editor_in_front() {
    let mut h = opened(Mode::Tabs, "ui-u22-follow", 1600.0, Tab::Journal, 0);
    h.state_mut().faction_view.advanced = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    let shows = |h: &Harness<'_, Moonglow>, label: &str| h.query_by_label(label).is_some();
    assert!(shows(&h, "Global effect") && shows(&h, "Full detail"), "the faction editor's");
    assert!(!shows(&h, "Finish category"), "not the journal's");
    doc_tab(&h, "Journal").click();
    settle(&mut h);
    choose_second_entry(&mut h);
    assert!(shows(&h, "Finish category"), "the journal's");
    assert!(!shows(&h, "Global effect"), "not the faction editor's");
    // Floating: the window's own.
    let mut h = opened(Mode::Floating, "ui-u22-follow-float", 1600.0, Tab::Journal, 0);
    let at = h.state().dock.find_tab(&Tab::Journal).expect("open");
    assert!(!at.surface.is_main(), "a window of its own");
    choose_second_entry(&mut h);
    let pane = h.state().tab_rect(&Tab::Journal).expect("drawn");
    let window = (h.state().dock.iter_leaves())
        .find(|(_, leaf)| leaf.tabs.contains(&Tab::Journal))
        .map(|(_, leaf)| leaf.rect)
        .unwrap();
    let check = h.get_by_label("Finish category").rect();
    assert!(window.contains_rect(check) && !pane.contains_rect(check), "in its window's pane");
    let screen = h.ctx.content_rect();
    assert!(screen.contains_rect(window), "within the main window: {window:?}");
}

/// The faction editor's header holds its controls (the Factions board's),
/// at the board's width and at a narrow one; Remove Faction and Change
/// Name… are off for a standard faction, on for one of the module's.
#[test]
fn the_faction_header_holds_its_controls() {
    let controls = ["Mode", "Add Faction…", "Remove Faction", "Change Name…"];
    for width in [1600.0, 1000.0] {
        let mut h = opened(Mode::Tabs, "ui-u22-faction-header", width, Tab::Factions, 0);
        header_holds(&mut h, FACTION_HEADER, &controls);
    }
    let mut h = opened(Mode::Tabs, "ui-u22-faction-header-on", 1600.0, Tab::Factions, 0);
    let off = |h: &Harness<'_, Moonglow>, name: &str| {
        h.get_by_label(FACTION_HEADER).get_by_label(name).accesskit_node().is_disabled()
    };
    h.state_mut().faction_view.selected = 1;
    settle(&mut h);
    assert!(off(&h, "Remove Faction") && off(&h, "Change Name…"), "Hostile is standard");
    h.state_mut().faction_view.selected = 5;
    settle(&mut h);
    assert!(!off(&h, "Remove Faction") && !off(&h, "Change Name…"), "Guards is ours");
}

/// The matrix: a click chooses its cell and gives it the keys; the arrows
/// move the chosen cell; Tab is not bound (it moves nothing); a number
/// typed and Enter set the reaction, one Undo step.
#[test]
fn the_matrix_is_chosen_by_a_click_and_its_keys_move_and_type() {
    let mut h = opened(Mode::Tabs, "ui-u22-matrix-keys", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    click_pos(&h, cell(&h, "Guards", "Commoner"));
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 2), "Guards toward Commoner");
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 3), "right");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), (6, 3), "down");
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert_eq!(chosen(&h), (6, 3), "Tab is not bound");
    let before = undo_depth(&h);
    h.event(egui::Event::Text("7".into()));
    settle(&mut h);
    h.event(egui::Event::Text("5".into()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(factions(&mut h).reputation(6, 3), Some(75), "Thieves toward Merchant");
    assert_eq!(undo_depth(&h), before + 1, "one step");
}

/// The matrix's keys do nothing while the Add Faction window is open, nor
/// while another editor is the active one.
#[test]
fn the_matrix_keys_wait_for_a_window_and_the_active_editor() {
    let mut h = opened(Mode::Tabs, "ui-u22-matrix-blocked", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    click_pos(&h, cell(&h, "Guards", "Commoner"));
    settle(&mut h);
    h.get_by_label(FACTION_HEADER).get_by_label("Add Faction…").click();
    settle(&mut h);
    assert!(h.state().faction_view.adding.is_some(), "its window open");
    // (Its Name field let go of: the window alone holds the keys back.)
    // (The window's Global effect, not the Faction panel's in the Right
    // Dock: both are named so.)
    let dock = right_dock_rect(&h);
    h.get_all_by_label("Global effect")
        .find(|n| !dock.contains(n.rect().center()))
        .expect("the window's Global effect")
        .click();
    settle(&mut h);
    assert!(!h.ctx.egui_wants_keyboard_input(), "nothing typed in");
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 2), "no key with its window open");
    h.state_mut().faction_view.adding = None;
    settle(&mut h);
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 3), "the keys again");
    // Another editor in front: not the matrix's.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Journal));
    settle(&mut h);
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 3), "the journal in front");
}

/// A cell dragged sideways changes its reaction as it is let go: one
/// Undo step, however far it went.
#[test]
fn a_dragged_cell_changes_in_one_step() {
    let mut h = opened(Mode::Tabs, "ui-u22-matrix-drag", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    let was = factions(&mut h).reputation(5, 2).unwrap();
    let before = undo_depth(&h);
    let at = cell(&h, "Guards", "Commoner");
    drag(&mut h, at, at + egui::vec2(40.0, 0.0));
    settle(&mut h);
    let now = factions(&mut h).reputation(5, 2).unwrap();
    assert!(now > was, "raised: {was} to {now}");
    assert_eq!(undo_depth(&h), before + 1, "one step");
}

/// The Faction panel's Global effect and the Reaction panel's slider each
/// change in one Undo step; Name and Parent are read-only.
#[test]
fn the_faction_and_reaction_panels_change_in_one_step_each() {
    let mut h = opened(Mode::Tabs, "ui-u22-panels", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    h.state_mut().faction_view.selected = 5;
    h.state_mut().faction_view.target = 2;
    settle(&mut h);
    let before = undo_depth(&h);
    h.get_by_label("Global effect").click();
    settle(&mut h);
    assert!(!factions(&mut h).factions[5].global, "Guards' global effect off");
    assert_eq!(undo_depth(&h), before + 1, "one step");
    set_number(&h, &h.get_by_label("Reputation"), 7.0);
    settle(&mut h);
    assert_eq!(factions(&mut h).reputation(5, 2), Some(7), "Guards toward Commoner");
    assert_eq!(undo_depth(&h), before + 2, "one step more");
    let said = h.query_all_by_label_contains("Guards is hostile toward Commoner.").count();
    assert_eq!(said, 1, "the Reaction panel's sentence");
    let name = (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| n.accesskit_node().value().as_deref() == Some("Guards"))
        .expect("Name")
        .rect()
        .center();
    click_pos(&h, name);
    settle(&mut h);
    h.event(egui::Event::Text("x".into()));
    settle(&mut h);
    assert_eq!(factions(&mut h).factions[5].name, "Guards", "read-only");
}

/// The Reactions panel: a row clicked chooses its cell; Full detail adds,
/// under them, how the others regard the chosen faction.
#[test]
fn the_reactions_rows_choose_and_full_detail_adds_the_others() {
    let mut h = opened(Mode::Tabs, "ui-u22-reactions", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    h.state_mut().faction_view.selected = 5;
    settle(&mut h);
    let row = h.get_by_label_contains("Merchant: Guards is");
    row.click();
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 3), "Merchant chosen");
    assert!(h.query_by_label_contains("From Thieves: ").is_none());
    h.get_by_label("Full detail").click();
    settle(&mut h);
    assert!(h.query_by_label_contains("From Thieves: Thieves is").is_some(), "the others' too");
}

/// The Outliner lists the factions: a click chooses one and gives the list
/// the keys; Up chooses the one before; a letter goes to a faction by its
/// name.
#[test]
fn the_outliner_s_factions_choose_and_its_keys_move() {
    let mut h = opened(Mode::Tabs, "ui-u22-outliner", 1600.0, Tab::Factions, 0);
    let row = crate::conversation_outliner::outliner_row(&h, "Thieves").expect("its row");
    row.click();
    settle(&mut h);
    assert_eq!(h.state().faction_view.selected, 6, "Thieves");
    h.key_press(egui::Key::ArrowUp);
    settle(&mut h);
    assert_eq!(h.state().faction_view.selected, 5, "Guards");
    h.event(egui::Event::Text("m".into()));
    settle(&mut h);
    assert_eq!(h.state().faction_view.selected, 3, "Merchant");
}

/// F6 goes from the Outliner's factions to the matrix and on to the Right
/// Dock's panels; the Outliner after it holds the arrow keys (Down chooses
/// the next faction), and the matrix after it its own (Right moves the
/// chosen cell).
#[test]
fn f6_goes_through_the_outliner_the_matrix_and_the_panels() {
    let mut h = opened(Mode::Tabs, "ui-u22-f6", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    let mut seen = Vec::new();
    for _ in 0..12 {
        h.key_press(egui::Key::F6);
        settle(&mut h);
        seen.push(mg_ui::palette_view::arrow_keys_are(&h.ctx));
    }
    let order = ["Tree", "Marks", "Factions", "FactionPanel", "Reaction", "Reactions"];
    let at = seen.iter().position(|s| s == "Tree").expect("the module tree");
    let lap: Vec<&str> = seen[at..at + 6].iter().map(String::as_str).collect();
    assert_eq!(lap, order, "{seen:?}");
    // To the Outliner: Down chooses the next faction.
    while mg_ui::palette_view::arrow_keys_are(&h.ctx) != "Marks" {
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    let before = h.state().faction_view.selected;
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(h.state().faction_view.selected, before + 1, "the Outliner's Down");
    // To the matrix: Right moves the chosen cell.
    h.key_press(egui::Key::F6);
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Factions");
    let (p, t) = chosen(&h);
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_eq!(chosen(&h), (p, t + 1), "the matrix's Right");
}

/// The faction editor's hints and counts (the Factions board's): over
/// the matrix, Click Select a reaction, Drag Change a reaction and Double
/// Click Type a value (Basic's chart has no typing); elsewhere none; never
/// Save. "7 factions" and "2 added by the module" always.
#[test]
fn the_faction_hints_and_counts_are_as_bound() {
    let mut h = opened(Mode::Tabs, "ui-u22-faction-hints", 1600.0, Tab::Factions, 0);
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said();
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    h.hover_at(cell(&h, "Guards", "Commoner"));
    settle(&mut h);
    let s = said(&h);
    let hints =
        ["Click: Select a reaction", "Drag: Change a reaction", "Double Click: Type a value"];
    for w in hints.iter().chain(&["7 factions", "2 added by the module"]) {
        assert!(s.contains(&w.to_string()), "{w}: {s:?}");
    }
    assert!(!s.iter().any(|l| l.contains("Save")), "{s:?}");
    h.hover_at(left_dock_rect(&h).center());
    settle(&mut h);
    assert!(!said(&h).iter().any(|l| l.contains("Click")), "elsewhere: {:?}", said(&h));
    // (Over the column headers: no cell, no hint.)
    let heads = h.get_by_label(MATRIX).rect().min - egui::vec2(0.0, 14.0);
    h.hover_at(heads + egui::vec2(40.0, 0.0));
    settle(&mut h);
    assert!(!said(&h).iter().any(|l| l.contains("Click")), "the headers: {:?}", said(&h));
    // Basic, Hostile chosen: Click over a row's name (it chooses that
    // reaction), Drag over its slider; neither over the heading, no Double
    // Click (the review's finding 11: each was said over all the chart).
    h.state_mut().faction_view.advanced = false;
    h.state_mut().faction_view.selected = 1;
    settle(&mut h);
    let only = |s: &[String], on: &[&str], what: &str| {
        for (i, w) in hints.iter().enumerate() {
            let want = on.contains(&["click", "drag", "double"][i]);
            assert_eq!(s.contains(&w.to_string()), want, "{what}: {w}: {s:?}");
        }
    };
    h.hover_at(h.get_by_label_contains("How Hostile regards the others").rect().center());
    settle(&mut h);
    only(&said(&h), &[], "the heading");
    let left = left_dock_rect(&h);
    let name = (h.query_all_by_label("Commoner"))
        .map(|n| n.rect())
        .find(|r| !left.contains_rect(*r))
        .expect("the chart's row");
    h.hover_at(name.center());
    settle(&mut h);
    only(&said(&h), &["click"], "a row's name");
    let slider = (h.query_all_by_role(egui::accesskit::Role::Slider))
        .find(|n| {
            let l = n.accesskit_node().label().unwrap_or_default();
            l.starts_with("Hostile is") && l.ends_with("toward Commoner")
        })
        .expect("its slider")
        .rect();
    h.hover_at(slider.center());
    settle(&mut h);
    only(&said(&h), &["drag"], "its slider");
}

/// A hundred factions and more: the file is read once a change, not once a
/// frame; only the cells in sight are laid out, as it opens and scrolled
/// to its end; the window comes to rest in Basic, Advanced, a value typed
/// and a change, the last check that the change was read.
#[test]
fn a_hundred_factions_are_read_once_and_drawn_in_sight() {
    let mut h = opened(Mode::Tabs, "ui-u22-hundred", 1600.0, Tab::Factions, 100);
    let walks = |h: &Harness<'_, Moonglow>| h.state().faction_view.walks();
    let steady = |h: &mut Harness<'_, Moonglow>, what: &str| {
        settle(h);
        let was = walks(h);
        h.run_steps(10);
        assert_eq!(walks(h), was, "{what}: read again with nothing changed");
    };
    steady(&mut h, "Basic");
    h.state_mut().faction_view.advanced = true;
    steady(&mut h, "Advanced");
    let cells = |h: &Harness<'_, Moonglow>| {
        h.get_by_label(MATRIX).query_all_by_label_contains(" toward ").count()
    };
    let shown = cells(&h);
    assert!((50..2000).contains(&shown), "{shown} cells laid out of 11,449");
    click_pos(&h, cell(&h, "Guards", "Commoner"));
    settle(&mut h);
    for _ in 0..110 {
        h.key_press(egui::Key::ArrowRight);
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
    }
    assert_eq!(chosen(&h), (106, 106), "the last cell");
    steady(&mut h, "moved to the far right");
    assert!(cells(&h) < 2000, "still only those in sight: {}", cells(&h));
    h.event(egui::Event::Text("3".into()));
    settle(&mut h);
    steady(&mut h, "a value typed");
    let was = walks(&h);
    h.key_press(egui::Key::Enter);
    steady(&mut h, "set");
    assert!(walks(&h) > was, "the change read");
    let (p, t) = chosen(&h);
    assert_eq!(factions(&mut h).reputation(p, t), Some(3));
}

/// A floating faction editor's first size (the Factions board's 780, its
/// Faction, Reaction and Reactions and its factions beside it) is held
/// within the main window; its header keeps every control on its row.
#[test]
fn a_floating_faction_editor_opens_within_the_window() {
    for width in [1600.0, 1280.0] {
        let h = opened(Mode::Floating, "ui-u22-floating", width, Tab::Factions, 0);
        let what = format!("{width}");
        let at = h.state().dock.find_tab(&Tab::Factions).expect("open");
        assert!(!at.surface.is_main(), "{what}: a window of its own");
        let window = (h.state().dock.iter_leaves())
            .find(|(_, leaf)| leaf.tabs.contains(&Tab::Factions))
            .map(|(_, leaf)| leaf.rect)
            .unwrap();
        let screen = h.ctx.content_rect();
        assert!(screen.contains_rect(window), "{what}: within the main window: {window:?}");
        // (Its first size: the editor's 780, its panels (280) and its
        // factions (260) beside it, held to the main window less its
        // margins: the coordinator's ruling at U17's review.)
        // (Less, where the Left Dock is folded, the room its strip and a
        // window's resize handles take: `a_floating_editor_opens_clear_of_the_docks_strips`.)
        let strip = h.query_by_label("Show the Left Dock").map_or(0.0, |c| c.rect().right() + 11.0);
        let want = (780.0 + mg_ui::RIGHT_DOCK_LEAST + 1.0 + 260.0 + 1.0).min(width - strip - 48.0);
        assert!(window.width() >= want - 12.0, "{what}: {} wide, wants {want}", window.width());
        let global = h.get_by_label("Global effect").rect();
        assert!(window.contains_rect(global), "{what}: its Faction in its pane");
        let rec = mg_uikit::header::records(&h.ctx, FACTION_HEADER);
        let rec = rec.first().expect("its header");
        for (name, place) in &rec.places {
            assert_ne!(*place, mg_uikit::header::Place::More, "{what}: {name} in More");
            assert_ne!(*place, mg_uikit::header::Place::Cut, "{what}: {name} cut");
        }
    }
}

/// Records from before read without a fault, as the tip before U22
/// (248f3515) writes them: the window sizes of the faction editor and the
/// journal (780 by 632: its `Tab::window_size`, kept by kind "factions"
/// and "journal", `Moonglow::remember_window_sizes`); their dock layouts,
/// which the tip keeps for a kind that brings no panels only as the Right
/// Dock's column seam (`column`) and its width alone (`wide_width`)
/// dragged beside it (`shell::keep_front` returns before `fronts` for a
/// kind that brings none); open-tabs records with both open, docked
/// ("middle") and floating ("window" with its rect, `rects_scaled`).
/// Both open where they were, their panels shown.
#[test]
fn records_of_the_journal_and_factions_read_without_error() {
    let text = "(window_sizes:[(\"factions\",(780,632)),(\"journal\",(780,632))],\
                window_sizes_scaled:true,dock_layouts:{\"factions\":(column:320),\
                \"journal\":(wide_width:500)},name_resrefs:true)";
    let (settings, _unread) = mg_ui::Settings::read_fields(text).expect("read");
    assert!(settings.name_resrefs, "the rest read");
    for mode in MODES {
        let path = module_of_ours(&mode.dir("ui-u22-records"), 0);
        let tabs = path.parent().unwrap().join("open-tabs");
        std::fs::create_dir_all(&tabs).unwrap();
        let record = match mode {
            Mode::Floating => {
                r#"{"panes":[{"place":"middle","tabs":["module-properties"],"active":"module-properties"},
                {"place":"window","tabs":["factions"],"active":"factions","rect":[300.0,60.0,780.0,632.0]},
                {"place":"window","tabs":["journal"],"active":"journal","rect":[200.0,90.0,780.0,632.0]}],
                "front":"factions","rects_scaled":true}"#
            }
            Mode::Tabs => {
                r#"{"panes":[{"place":"middle","tabs":["module-properties","journal","factions"],"active":"factions"}],
                "front":"factions","rects_scaled":true}"#
            }
        };
        std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
        let mut app = app_in(mode, Vec::new());
        app.settings.window_sizes = settings.window_sizes.clone();
        app.settings.window_sizes_scaled = settings.window_sizes_scaled;
        app.settings.dock_layouts = settings.dock_layouts.clone();
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(tabs.clone());
        app.open_module(&path);
        let mut h = harness(app, 1600.0, 900.0);
        settle(&mut h);
        let what = format!("{mode:?}");
        for tab in [Tab::Factions, Tab::Journal] {
            assert!(h.state().dock.find_tab(&tab).is_some(), "{what}: {tab:?} open");
        }
        assert!(h.query_by_label("Global effect").is_some(), "{what}: the Faction panel");
        assert!(h.state().log.entries.iter().all(|e| !e.1.contains("error")), "{what}");
    }
}

/// How high the controls drawn in the Right Dock between `top` and
/// `bottom` reach below `top` (labels, fields, buttons, checks: not the
/// containers, whose rects are their sections').
fn reach(h: &Harness<'_, Moonglow>, top: f32, bottom: f32) -> f32 {
    use egui::accesskit::Role;
    // (AccessKit's boxes are in pixels; the dock's and a node's `rect` in points.)
    let ppp = h.ctx.pixels_per_point();
    let dock = right_dock_rect(h);
    let leaf = |r: Role| {
        matches!(
            r,
            Role::Label
                | Role::TextInput
                | Role::MultilineTextInput
                | Role::Button
                | Role::CheckBox
                | Role::ComboBox
                | Role::SpinButton
                | Role::Slider
        )
    };
    let mut most = top;
    for n in h.query_all(egui_kittest::kittest::by()) {
        let Some(b) = n.accesskit_node().bounding_box() else { continue };
        if !leaf(n.accesskit_node().role()) {
            continue;
        }
        let r = egui::Rect::from_min_max(
            egui::pos2(b.x0 as f32 / ppp, b.y0 as f32 / ppp),
            egui::pos2(b.x1 as f32 / ppp, b.y1 as f32 / ppp),
        );
        if dock.contains_rect(r) && r.top() >= top - 0.5 && r.bottom() <= bottom {
            most = most.max(r.bottom());
        }
    }
    most - top
}

/// The journal's Entry and Category and the faction editor's Faction and
/// Reaction are never taller where they are wider (the U14 rule: a
/// scrolled panel taller where wider never rests): their controls' reach
/// swept over the Right Dock's widths at three text sizes against every
/// narrower width, the window resting at each.
#[test]
fn the_panels_are_never_taller_where_wider() {
    for tab in [Tab::Journal, Tab::Factions] {
        let mut h = opened(Mode::Tabs, "ui-u22-tall", 1600.0, tab.clone(), 0);
        // (Wide enough at each size for the sections to stand stacked:
        // under 1400 points they share one tab bar.)
        h.set_size(egui::vec2(2200.0, 1300.0));
        match tab {
            Tab::Journal => choose_second_entry(&mut h),
            _ => {
                h.state_mut().faction_view.advanced = true;
                h.state_mut().faction_view.selected = 5;
                h.state_mut().faction_view.target = 2;
            }
        }
        settle(&mut h);
        for scale in [1.0_f32, 1.25, 1.5] {
            h.ctx.set_zoom_factor(scale);
            let mut seen: Vec<(u32, f32, f32)> = Vec::new();
            for width in (280..=480).step_by(8) {
                right_dock_width(&mut h, width as f32);
                settle(&mut h);
                let (upper, lower) = match tab {
                    Tab::Journal => {
                        let id = h.get_by_label("Finish category").rect().top() - 40.0;
                        let name = (h.get_all_by_role(egui::accesskit::Role::TextInput))
                            .find(|n| n.accesskit_node().value().as_deref() == Some("Quest A"))
                            .expect("Name")
                            .rect()
                            .top();
                        // The line between the sections: the last full-width
                        // seam above the Category's Name, its grip's foot
                        // (a widget sensing a drag alone, which has no
                        // AccessKit node: M5's W8).
                        let dock = right_dock_rect(&h);
                        let seams: Vec<egui::Rect> = h.ctx.viewport(|v| {
                            (v.prev_pass.widgets.layers())
                                .flat_map(|(_, w)| w.iter())
                                .filter(|w| w.sense == egui::Sense::DRAG)
                                .map(|w| w.rect)
                                .collect()
                        });
                        let line = (seams.iter())
                            .filter(|r| r.width() >= dock.width() - 2.0 && r.height() < 10.0)
                            .map(|r| r.bottom())
                            .filter(|y| *y < name && *y > id)
                            .fold(f32::NAN, f32::max);
                        assert!(line.is_finite(), "the line between the sections");
                        (reach(&h, id, line - 8.0), reach(&h, line, f32::INFINITY))
                    }
                    _ => {
                        let global = h.get_by_label("Global effect").rect();
                        // (The sentence, not the Reactions row of the same words.)
                        let dock = right_dock_rect(&h);
                        let docked = |r: egui::Rect| dock.x_range().contains(r.center().x);
                        let sentence = (h.get_all_by_label_contains("Guards is "))
                            .map(|n| n.rect())
                            .filter(|r| docked(*r))
                            .map(|r| r.top())
                            .fold(f32::INFINITY, f32::min);
                        let faction = reach(&h, global.top() - 70.0, sentence - 30.0);
                        let scale_words = (h.get_all_by_label_contains("Hostile 0 to 10"))
                            .map(|n| n.rect())
                            .filter(|r| docked(*r))
                            .map(|r| r.bottom())
                            .fold(f32::NEG_INFINITY, f32::max);
                        (faction, reach(&h, sentence - 0.5, scale_words + 0.5))
                    }
                };
                for (w, a, b) in &seen {
                    let what = format!("{tab:?} {scale}: {upper}, {lower} at {width}");
                    assert!(upper <= a + 0.5 && lower <= b + 0.5, "{what} against {a}, {b} at {w}");
                }
                seen.push((width, upper, lower));
            }
        }
    }
}

/// The cases the window comes to rest in: the panels stacked, behind one
/// another (a tab bar shared under 1400 points, the other's tab in front),
/// the Right Dock folded, both docks closed, floating in a pane of its own
/// and in a small window.
const REST_CASES: [(Mode, &str, f32); 7] = [
    (Mode::Tabs, "stacked", 1600.0),
    (Mode::Tabs, "first behind", 1300.0),
    (Mode::Tabs, "second behind", 1300.0),
    (Mode::Tabs, "folded", 1600.0),
    (Mode::Tabs, "closed", 1600.0),
    (Mode::Floating, "pane", 1600.0),
    (Mode::Floating, "small", 1280.0),
];

/// Puts the window in `case` of [`REST_CASES`]: `panels` the two panels
/// sharing a tab bar when the window is narrow.
fn rest_case(h: &mut Harness<'_, Moonglow>, case: &str, panels: [&str; 2]) {
    let tab = |h: &Harness<'_, Moonglow>, name: &str| {
        let bar = h.get_by_label(mg_ui::RIGHT_DOCK);
        let found = bar
            .query_all_by_label(name)
            .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab);
        found.unwrap_or_else(|| panic!("{case}: the tab {name}")).rect().center()
    };
    match case {
        "first behind" => click_pos(h, tab(h, panels[1])),
        "second behind" => click_pos(h, tab(h, panels[0])),
        "folded" => h.state_mut().settings.hide_palettes = true,
        "closed" => {
            h.state_mut().settings.hide_palettes = true;
            h.state_mut().settings.hide_tree = true;
        }
        _ => {}
    }
    settle(h);
}

/// The window comes to rest (`settle`) with the journal and its panels in
/// front, behind and folded, the docks closed, docked and floating: an
/// entry chosen, its Text typed in, a category added, an entry copied and
/// pasted, the chosen entry deleted, an Undo; the last check that the
/// delete changed the journal.
#[test]
fn the_window_comes_to_rest_with_the_journal() {
    for (mode, case, width) in REST_CASES {
        let what = format!("{mode:?} {case}");
        let mut h = opened(mode, "ui-u22-journal-rests", width, Tab::Journal, 0);
        choose_second_entry(&mut h);
        rest_case(&mut h, case, ["Entry", "Category"]);
        let text = (h.query_all_by_role(egui::accesskit::Role::MultilineTextInput))
            .find(|n| n.accesskit_node().value().as_deref() == Some("Entry002"));
        let drawn = matches!(case, "stacked" | "second behind" | "pane" | "small");
        assert_eq!(text.is_some(), drawn, "{what}: the Entry panel drawn");
        if let Some(text) = text {
            click_pos(&h, text.rect().center());
            settle(&mut h);
            h.event(egui::Event::Text(" more".into()));
            settle(&mut h);
        }
        let header = |h: &Harness<'_, Moonglow>, name: &str| {
            h.get_by_label(JOURNAL_HEADER).get_by_label(name).rect().center()
        };
        click_pos(&h, header(&h, "Copy"));
        settle(&mut h);
        click_pos(&h, header(&h, "Paste"));
        settle(&mut h);
        let entries =
            |h: &mut Harness<'_, Moonglow>| mg_module::journal::entries(&categories(h)[0]).len();
        assert_eq!(entries(&mut h), 3, "{what}: pasted");
        // (Its text typed: "Entry002 more" where the panel is drawn, set as
        // Copy left the field; the copy pasted with it.)
        let typed = if drawn { "Entry002 more" } else { "Entry002" };
        assert_eq!(entry_text(&mut h, 0, 1), typed, "{what}: what was typed set");
        // (The pasted copy keeps its ID: the first is the original.)
        let original = tree(&h).query_all_by_label_contains("[0002] Entry002").next().unwrap();
        original.click();
        settle(&mut h);
        click_pos(&h, header(&h, "Delete"));
        settle(&mut h);
        assert_eq!(entries(&mut h), 2, "{what}: the chosen entry deleted");
        h.state_mut().journal_view.selected = None;
        settle(&mut h);
        click_pos(&h, header(&h, "Add"));
        settle(&mut h);
        assert_eq!(categories(&mut h).len(), 3, "{what}: a category added");
        h.state_mut().actions.push(mg_ui::Action::Undo);
        settle(&mut h);
        assert_eq!(categories(&mut h).len(), 2, "{what}: taken back");
    }
}

/// The window comes to rest with the faction editor and its panels in
/// front, behind and folded, the docks closed, docked and floating: in
/// Advanced a cell clicked, an arrow, a number typed and set, the Reaction
/// panel's slider dragged and let go (one Undo step), an Outliner row
/// clicked where the Left Dock is drawn; in Basic the chosen faction
/// removed and an Undo; the last check that the removal changed the
/// factions.
#[test]
fn the_window_comes_to_rest_with_the_faction_editor() {
    for (mode, case, width) in REST_CASES {
        let what = format!("{mode:?} {case}");
        let mut h = opened(mode, "ui-u22-faction-rests", width, Tab::Factions, 0);
        h.state_mut().faction_view.advanced = true;
        settle(&mut h);
        rest_case(&mut h, case, ["Faction", "Reaction"]);
        click_pos(&h, cell(&h, "Guards", "Commoner"));
        settle(&mut h);
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
        h.event(egui::Event::Text("4".into()));
        settle(&mut h);
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(factions(&mut h).reputation(6, 2), Some(4), "{what}: Thieves typed");
        // (The Reaction panel's slider: not one of the matrix's cells, which
        // are sliders too.)
        let matrix = h.get_by_label(MATRIX);
        let cells: Vec<egui::Rect> =
            matrix.query_all_by_role(egui::accesskit::Role::Slider).map(|n| n.rect()).collect();
        let slider = (h.query_all_by_role(egui::accesskit::Role::Slider))
            .map(|n| n.rect())
            .find(|r| !cells.contains(r));
        let drawn = matches!(case, "stacked" | "first behind" | "pane" | "small");
        assert_eq!(slider.is_some(), drawn, "{what}: the Reaction panel drawn");
        if let Some(r) = slider {
            let before = undo_depth(&h);
            let from = egui::pos2(r.left() + 4.0, r.center().y);
            drag(&mut h, from, from + egui::vec2(r.width() * 0.5, 0.0));
            settle(&mut h);
            assert_eq!(undo_depth(&h), before + 1, "{what}: the drag one step");
        }
        // (The Left Dock folded by the width under 1500, closed by the case.)
        let row = crate::conversation_outliner::outliner_row(&h, "Thieves");
        let listed = matches!(case, "stacked" | "folded" | "pane" | "small");
        assert_eq!(row.is_some(), listed, "{what}: the Outliner's factions drawn");
        if let Some(row) = row {
            row.click();
            settle(&mut h);
            assert_eq!(h.state().faction_view.selected, 6, "{what}: Thieves chosen");
        }
        h.state_mut().faction_view.selected = 6;
        h.state_mut().faction_view.advanced = false;
        settle(&mut h);
        let count = factions(&mut h).factions.len();
        h.get_by_label(FACTION_HEADER).get_by_label("Remove Faction").click();
        settle(&mut h);
        assert_eq!(factions(&mut h).factions.len(), count - 1, "{what}: Thieves removed");
        h.state_mut().actions.push(mg_ui::Action::Undo);
        settle(&mut h);
        assert_eq!(factions(&mut h).factions.len(), count, "{what}: taken back");
    }
}

// ---- The fix round of M4's U22 review (`target/u22-fix/breaks.txt`). ----

/// Runs frames one at a time until `done` holds, with a deadline, then
/// until the window rests (what it shows drawn; where no job runs): a wait for what is
/// looked for, not for a count of frames.
fn until(
    h: &mut Harness<'_, Moonglow>,
    what: &str,
    done: impl Fn(&mut Harness<'_, Moonglow>) -> bool,
) {
    let started = std::time::Instant::now();
    while !done(h) {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "never: {what}");
    }
    // (Not while a job runs, a plugin's waiting on its question: the
    // window repaints until it ends.)
    if !h.state().busy() {
        settle(h);
    }
}

/// The text (English) of entry `i` of category `c`, as the workspace has it.
fn entry_text(h: &mut Harness<'_, Moonglow>, c: usize, i: usize) -> String {
    let cats = categories(h);
    let entry = &mg_module::journal::entries(&cats[c])[i];
    let text = entry.read(&mg_schema::jrl::categories::entry_list::TEXT);
    let (en, male) = (mg_core::Language::ENGLISH, mg_core::Gender::Male);
    text.text_in(en, male, mg_core::Codepage::default()).unwrap_or_default().into_owned()
}

/// Every entry's text, category by category.
fn all_texts(h: &mut Harness<'_, Moonglow>) -> Vec<Vec<String>> {
    let cats = categories(h);
    (0..cats.len())
        .map(|c| {
            (0..mg_module::journal::entries(&cats[c]).len()).map(|i| entry_text(h, c, i)).collect()
        })
        .collect()
}

/// The value of the text field (one line or many) that has the keyboard.
fn focused_text(h: &Harness<'_, Moonglow>) -> Option<String> {
    let roles = [egui::accesskit::Role::TextInput, egui::accesskit::Role::MultilineTextInput];
    roles.iter().find_map(|r| {
        h.query_all_by_role(*r)
            .find(|n| n.accesskit_node().is_focused())
            .map(|n| n.accesskit_node().value().unwrap_or_default().to_string())
    })
}

/// `typed` put at the end of the Entry panel's Text that holds `value`,
/// waited for until the field shows it.
fn type_into_text(h: &mut Harness<'_, Moonglow>, value: &str, typed: &str) {
    let memo = (h.get_all_by_role(egui::accesskit::Role::MultilineTextInput))
        .find(|n| n.accesskit_node().value().as_deref() == Some(value))
        .unwrap_or_else(|| panic!("the Text holding {value:?}"))
        .rect();
    click_pos(h, egui::pos2(memo.right() - 8.0, memo.top() + 8.0));
    until(h, "the Text has the keyboard", |h| focused_text(h).as_deref() == Some(value));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::End);
    h.event(egui::Event::Text(typed.into()));
    let want = format!("{value}{typed}");
    until(h, "the Text shows it", |h| focused_text(h).as_deref() == Some(want.as_str()));
}

/// The matrix in Advanced, Guards toward Commoner chosen by a click.
fn matrix_chosen(name: &str, more: usize) -> Harness<'static, Moonglow> {
    let mut h = opened(Mode::Tabs, name, 1600.0, Tab::Factions, more);
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    click_pos(&h, cell(&h, "Guards", "Commoner"));
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 2));
    h
}

/// Whether a value is being typed in a cell: its field has the keyboard.
fn cell_typing(h: &Harness<'_, Moonglow>) -> bool {
    let Some(matrix) = h.query_by_label(MATRIX).map(|m| m.rect()) else { return false };
    h.query_all_by_role(egui::accesskit::Role::TextInput).any(|n| {
        let a = n.accesskit_node();
        a.is_focused() && a.bounding_box().is_some() && matrix.contains(n.rect().center())
    })
}

/// Tab and Shift+Tab while a value is typed in a cell set it, as Enter
/// does, one step; the keys stay the matrix's (nothing else has the
/// keyboard, the chosen cell where it was): the next Enter types in the
/// cell again (it closed the editor through its tab's Close, which Tab
/// had given the focus), Right moves (the review's probe 3).
#[test]
fn tab_while_typing_in_a_cell_sets_it_and_keeps_the_keys() {
    for (tag, mods) in [("tab", egui::Modifiers::NONE), ("shift-tab", egui::Modifiers::SHIFT)] {
        let mut h = matrix_chosen(&format!("ui-u22-fix-{tag}"), 0);
        let before = undo_depth(&h);
        h.event(egui::Event::Text("4".into()));
        until(&mut h, "typing in the cell", |h| cell_typing(h));
        h.key_press_modifiers(mods, egui::Key::Tab);
        settle(&mut h);
        assert_eq!(factions(&mut h).reputation(5, 2), Some(4), "{tag}: set");
        assert_eq!(undo_depth(&h), before + 1, "{tag}: one step");
        assert_eq!(h.ctx.memory(|m| m.focused()), None, "{tag}: no control has the keyboard");
        assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Factions", "{tag}");
        assert_eq!(chosen(&h), (5, 2), "{tag}: where it was");
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert!(h.state().dock.find_tab(&Tab::Factions).is_some(), "{tag}: still open");
        assert!(cell_typing(&h), "{tag}: Enter types in the cell");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        h.key_press(egui::Key::ArrowRight);
        settle(&mut h);
        assert_eq!(chosen(&h), (5, 3), "{tag}: Right moves");
    }
}

/// Escape while a value is typed in a cell sets nothing: the reaction and
/// the Undo history as they were (the review's B21).
#[test]
fn escape_while_typing_in_a_cell_sets_nothing() {
    let mut h = matrix_chosen("ui-u22-fix-escape", 0);
    let (was, before) = (factions(&mut h).reputation(5, 2), undo_depth(&h));
    h.event(egui::Event::Text("7".into()));
    until(&mut h, "typing in the cell", |h| cell_typing(h));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!cell_typing(&h), "typing let go of");
    assert_eq!(factions(&mut h).reputation(5, 2), was, "nothing set");
    assert_eq!(undo_depth(&h), before, "no step");
}

/// A value typed in a cell scrolled out of sight (100 factions more, the
/// matrix scrolled by the wheel) is set, as when its field is left, and
/// the matrix's keys work on: Down moves the chosen cell (they were left
/// waiting on the field not drawn, the review's probe 2).
#[test]
fn a_value_typed_in_a_cell_scrolled_out_of_sight_is_set() {
    let mut h = matrix_chosen("ui-u22-fix-scrolled", 100);
    h.event(egui::Event::Text("4".into()));
    until(&mut h, "typing in the cell", |h| cell_typing(h));
    let at = cell(&h, "Guards", "Commoner");
    h.hover_at(at);
    let gone = |h: &mut Harness<'_, Moonglow>| {
        let n = h.get_by_label(MATRIX).query_all_by_label_contains(" toward Commoner").count();
        let shown = h
            .get_by_label(MATRIX)
            .query_all_by_label_contains(" toward Commoner")
            .any(|n| n.accesskit_node().label().is_some_and(|l| l.starts_with("Guards is ")));
        n == 0 || !shown
    };
    let started = std::time::Instant::now();
    while !gone(&mut h) {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -400.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "never scrolled away");
    }
    settle(&mut h);
    assert_eq!(factions(&mut h).reputation(5, 2), Some(4), "set as it went out of sight");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), (6, 2), "the keys work on");
}

/// Remove Faction with Guards toward Band 000 chosen: Thieves, which takes
/// Guards' number, is the chosen row, its column still Band 000 (it chose
/// Thieves toward Thieves: the names noted before the removal, read after
/// it, the review's probe 9); with the removed faction the chosen column
/// too, the faction in its place is both. Add Faction keeps the faction
/// added chosen.
#[test]
fn removing_the_chosen_faction_chooses_the_one_in_its_place() {
    let names = |h: &mut Harness<'_, Moonglow>| {
        let f = factions(h);
        let (p, t) = chosen(h);
        (f.factions[p as usize].name.clone(), f.factions[t as usize].name.clone())
    };
    let mut h = opened(Mode::Tabs, "ui-u22-fix-remove", 1600.0, Tab::Factions, 1);
    h.state_mut().faction_view.advanced = true;
    (h.state_mut().faction_view.selected, h.state_mut().faction_view.target) = (5, 7);
    settle(&mut h);
    assert_eq!(names(&mut h), ("Guards".into(), "Band 000".into()));
    h.get_by_label(FACTION_HEADER).get_by_label("Remove Faction").click();
    settle(&mut h);
    assert_eq!(names(&mut h), ("Thieves".into(), "Band 000".into()), "Thieves toward Band 000");
    // The removed faction the chosen column too.
    (h.state_mut().faction_view.selected, h.state_mut().faction_view.target) = (5, 5);
    settle(&mut h);
    h.get_by_label(FACTION_HEADER).get_by_label("Remove Faction").click();
    settle(&mut h);
    assert_eq!(names(&mut h), ("Band 000".into(), "Band 000".into()), "the one in its place");
    // Add Faction: the faction added chosen.
    h.get_by_label(FACTION_HEADER).get_by_label("Add Faction…").click();
    until(&mut h, "its Name has the keyboard", |h| focused_text(h).is_some());
    h.event(egui::Event::Text("Rangers".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(names(&mut h).0, "Rangers", "the faction added");
}

/// The Reactions panel's rows reach its foot (Full detail): it left about
/// two rows undrawn there, its rows measured with spacing they did not
/// have (the review's finding 5, under Grommin in the board's picture).
#[test]
fn the_reactions_rows_reach_its_foot() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-rx-foot", 1600.0, Tab::Factions, 100);
    h.state_mut().faction_view.advanced = true;
    h.state_mut().faction_view.selected = 5;
    settle(&mut h);
    let last = (h.query_all_by_label_contains(": Guards is").map(|n| n.rect().bottom()))
        .fold(f32::MIN, f32::max);
    let foot = h.get_by_label("Full detail").rect().top();
    let gap = foot - last;
    assert!(gap < mg_uikit::metrics::ROW_HEIGHT, "{gap} under the last row drawn");
}

/// The PC chosen from the Outliner in Advanced stays chosen (it became
/// Hostile: the matrix held its row to the first that regards, the
/// review's probe 1), and through a switch to Basic and back. The matrix
/// has no row of it, so none is chosen; the Faction panel shows the PC,
/// the Reaction panel says its feelings are not stored, Reactions how to
/// see the others' toward it; a digit with the matrix's keys types
/// nothing.
#[test]
fn the_pc_is_chosen_from_the_outliner_in_advanced_and_kept() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-pc", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    click_pos(&h, cell(&h, "Guards", "Commoner"));
    settle(&mut h);
    crate::conversation_outliner::outliner_row(&h, "PC").expect("its row").click();
    settle(&mut h);
    assert_eq!(h.state().faction_view.selected, 0, "the PC");
    let in_dock = |h: &Harness<'_, Moonglow>, label: &str, value: &str| {
        h.query_all_by_role(egui::accesskit::Role::TextInput).any(|n| {
            let n = n.accesskit_node();
            n.label().as_deref() == Some(label) && n.value().as_deref() == Some(value)
        })
    };
    assert!(in_dock(&h, "Name", "PC"), "the Faction panel shows the PC");
    assert!(
        h.query_by_label_contains("The PC faction's own feelings are not stored. Choose").is_some()
    );
    assert!(h.query_by_label_contains("Full detail shows how the others regard PCs").is_some());
    // (Reactions' title says what its "From …" rows are, not "How PC
    // regards the others".)
    assert!(h.query_by_label_contains("How the others regard PCs").is_some(), "Reactions' title");
    assert!(h.query_by_label_contains("How PC regards").is_none(), "not the PC's own");
    let selected = h
        .get_by_label(MATRIX)
        .query_all_by_label_contains(" toward ")
        .filter(|n| n.accesskit_node().is_selected() == Some(true))
        .count();
    assert_eq!(selected, 0, "no cell chosen: the PC has no row");
    // The matrix's keys: a digit and Enter type nothing.
    while mg_ui::palette_view::arrow_keys_are(&h.ctx) != "Factions" {
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    let before = undo_depth(&h);
    h.event(egui::Event::Text("7".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(!cell_typing(&h) && undo_depth(&h) == before, "nothing typed");
    assert_eq!(h.state().faction_view.selected, 0, "still the PC");
    for advanced in [false, true] {
        h.state_mut().faction_view.advanced = advanced;
        settle(&mut h);
        assert_eq!(h.state().faction_view.selected, 0, "kept (advanced {advanced})");
    }
}

/// A floating editor's first place is clear of the folded Left Dock's
/// strip (its resize handle lay over the strip's Show the Left Dock, which
/// a click then missed: the review's probe 8): a click on the chevron
/// unfolds the dock, at 1100, 1200 and 1280 wide, for the faction editor
/// and the journal (one path places every floating editor).
#[test]
fn a_floating_editor_opens_clear_of_the_docks_strips() {
    for tab in [Tab::Factions, Tab::Journal] {
        for width in [1100.0, 1200.0, 1280.0] {
            let name = format!("ui-u22-fix-strip-{tab:?}-{width}").to_lowercase();
            let mut h = opened(Mode::Floating, &name, width, tab.clone(), 0);
            let what = format!("{tab:?} {width}");
            let window = (h.state().dock.iter_leaves())
                .find(|(_, leaf)| leaf.tabs.contains(&tab))
                .map(|(_, leaf)| leaf.rect)
                .unwrap();
            let chevron = h.get_by_label("Show the Left Dock").rect();
            assert!(
                window.left() - 10.0 > chevron.right(),
                "{what}: its handle clear of the strip: {window:?} {chevron:?}"
            );
            assert!(h.ctx.content_rect().contains_rect(window), "{what}: within the window");
            click_pos(&h, chevron.center());
            settle(&mut h);
            assert!(h.state().left_dock_shows().is_some(), "{what}: the Left Dock unfolded");
        }
    }
}

/// Name and Parent in the Faction panel and the Reaction panel's value
/// field are named for a screen reader; a matrix cell set by one (its
/// SetValue, Increment and Decrement) changes the reaction, one Undo step
/// each, and becomes the chosen cell, which says it is selected (the
/// review's probe 7: they did nothing).
#[test]
fn the_panels_fields_are_named_and_a_cell_answers_a_screen_reader() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-access", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    (h.state_mut().faction_view.selected, h.state_mut().faction_view.target) = (5, 2);
    settle(&mut h);
    for (label, value) in [("Name", "Guards"), ("Parent", "Hostile")] {
        let found = h.query_all_by_role(egui::accesskit::Role::TextInput).any(|n| {
            let n = n.accesskit_node();
            n.label().as_deref() == Some(label) && n.value().as_deref() == Some(value)
        });
        assert!(found, "{label} named");
    }
    let value = h.get_by_label("Reputation value");
    assert!(right_dock_rect(&h).contains_rect(value.rect()), "the value field in Reaction");
    let thieves_merchant = |h: &Harness<'_, Moonglow>| {
        let found = h
            .get_by_label(MATRIX)
            .query_all_by_label_contains(" toward Merchant")
            .find(|n| n.accesskit_node().label().is_some_and(|l| l.starts_with("Thieves is")));
        found.expect("Thieves toward Merchant").accesskit_node().locate()
    };
    let ask = |h: &Harness<'_, Moonglow>, action, data| {
        let (target_node, target_tree) = thieves_merchant(h);
        h.event(egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action,
            target_node,
            target_tree,
            data,
        }));
    };
    let before = undo_depth(&h);
    use egui::accesskit::{Action, ActionData};
    ask(&h, Action::SetValue, Some(ActionData::NumericValue(80.0)));
    settle(&mut h);
    assert_eq!(factions(&mut h).reputation(6, 3), Some(80), "set");
    assert_eq!(undo_depth(&h), before + 1, "one step");
    assert_eq!(chosen(&h), (6, 3), "the chosen cell");
    ask(&h, Action::Increment, None);
    settle(&mut h);
    assert_eq!(factions(&mut h).reputation(6, 3), Some(81), "one more");
    ask(&h, Action::Decrement, None);
    settle(&mut h);
    assert_eq!(factions(&mut h).reputation(6, 3), Some(80), "one less");
    assert_eq!(undo_depth(&h), before + 3, "a step each");
    let selected: Vec<String> = h
        .get_by_label(MATRIX)
        .query_all_by_label_contains(" toward ")
        .filter(|n| n.accesskit_node().is_selected() == Some(true))
        .map(|n| n.accesskit_node().label().unwrap_or_default().to_string())
        .collect();
    assert_eq!(selected, ["Thieves is neutral toward Merchant"], "the chosen cell selected");
}

/// No key of the matrix while a popover is open (the header's More, at a
/// width that puts commands into it), though the matrix holds the keys:
/// the review's B5.
#[test]
fn the_matrix_keys_wait_for_a_popover() {
    let mut h = matrix_chosen("ui-u22-fix-popover", 0);
    let mut width = 1000.0;
    loop {
        h.set_size(egui::vec2(width, 900.0));
        settle(&mut h);
        let rec = mg_uikit::header::records(&h.ctx, FACTION_HEADER);
        let more = rec
            .first()
            .is_some_and(|r| r.places.iter().any(|(_, p)| *p == mg_uikit::header::Place::More));
        if more {
            break;
        }
        width -= 20.0;
        assert!(width > 300.0, "no width puts a command into More");
    }
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Factions", "the matrix's keys");
    let (was, before) = (chosen(&h), undo_depth(&h));
    h.get_by_label(FACTION_HEADER).get_by_label(mg_uikit::header::MORE).click();
    settle(&mut h);
    assert!(mg_uikit::popover::Popover::any_open(&h.ctx), "More open");
    assert!(!h.ctx.egui_wants_keyboard_input(), "nothing typed in");
    h.key_press(egui::Key::ArrowRight);
    h.event(egui::Event::Text("7".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(chosen(&h), was, "no arrow");
    assert!(!cell_typing(&h) && undo_depth(&h) == before, "no value typed");
}

/// With the faction editor and the journal side by side, a click in the
/// journal's tree makes the journal the active editor: the matrix's keys
/// are not taken, though its Keys stay (the review's B25, which the test
/// with one group of tabs passed without, the matrix not being drawn).
#[test]
fn the_matrix_keys_wait_for_the_active_editor_beside_it() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-two-groups", 1600.0, Tab::Factions, 0);
    h.state_mut().faction_view.advanced = true;
    {
        let s = h.state_mut();
        let path = s.dock.find_tab(&Tab::Factions).expect("open");
        s.dock[path.surface].split_right(path.node, 0.5, vec![Tab::Journal]);
    }
    settle(&mut h);
    click_pos(&h, cell(&h, "Guards", "Commoner"));
    settle(&mut h);
    tree_row(&h, "Quest A [Category000]").click();
    settle(&mut h);
    assert_eq!(h.state().journal_view.selected, Some(Node::Category(0)));
    let (before, merchant) = (undo_depth(&h), factions(&mut h).reputation(5, 3));
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    h.event(egui::Event::Text("7".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(chosen(&h), (5, 2), "not moved");
    assert_eq!(undo_depth(&h), before, "no reaction changed");
    assert_eq!(factions(&mut h).reputation(5, 3), merchant, "Guards toward Merchant as it was");
}

/// A journal of 300 categories, every one open: only the tree's rows in
/// sight are laid out (the review's B17).
#[test]
fn only_the_journal_rows_in_sight_are_laid_out() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-rows", 1600.0, Tab::Journal, 0);
    let mut cats = categories(&mut h);
    while cats.len() < 300 {
        let c = mg_module::journal::new_category(&cats);
        cats.push(c);
    }
    let all = mg_edit::Edit::SetField {
        key: jrl(),
        path: mg_edit::GffPath::root(),
        label: "Categories".into(),
        value: Some(mg_module::journal::list_value(cats)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Many", vec![all])));
    settle(&mut h);
    h.get_by_label(JOURNAL_HEADER).get_by_label("Expand All").click();
    settle(&mut h);
    assert_eq!(h.state().journal_view.open.len(), 300, "every one open");
    let laid = tree(&h).query_all_by_role(egui::accesskit::Role::ListItem).count()
        + tree(&h).query_all_by_label_contains("[Category").count();
    assert!((5..120).contains(&laid), "{laid} rows laid out of 303");
}

/// With no category, the journal's hints over its tree are Click Select
/// alone: no Double Click (the review's B24).
#[test]
fn no_double_click_hint_without_categories() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-no-cats", 1600.0, Tab::Journal, 0);
    let none = mg_edit::Edit::SetField {
        key: jrl(),
        path: mg_edit::GffPath::root(),
        label: "Categories".into(),
        value: Some(mg_module::journal::list_value(Vec::new())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("None", vec![none])));
    settle(&mut h);
    h.hover_at(tree_row(&h, "Root").rect().center());
    settle(&mut h);
    let s = h.state().status_said();
    assert!(s.contains(&"Click: Select".to_string()), "{s:?}");
    assert!(!s.iter().any(|l| l.contains("Double Click")), "{s:?}");
}

/// The tree shows a category's name and an entry's text that are the
/// game's talk table's (Chapter1's are StrRefs): with the game installed
/// (the review's B18; skipped without it).
#[test]
fn the_tree_shows_talk_table_names() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let path = module_of_ours("ui-u22-fix-tlk", 0);
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&path);
    close_tab(&mut app, &Tab::ModuleProperties);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let strref = mg_core::StrRef(5);
    let words = (h.state().game.as_deref())
        .and_then(|g| g.string(strref))
        .filter(|s| !s.is_empty())
        .expect("a string of the game's");
    let mut cats = categories(&mut h);
    cats[0].write(&mg_schema::jrl::categories::NAME, mg_core::LocString::from_strref(strref));
    let set = mg_edit::Edit::SetField {
        key: jrl(),
        path: mg_edit::GffPath::root(),
        label: "Categories".into(),
        value: Some(mg_module::journal::list_value(cats)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Named", vec![set])));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Journal));
    settle(&mut h);
    let label = format!("{words} [Category000]");
    assert!(tree(&h).query_by_label(&label).is_some(), "the game's words: {label:?}");
}

/// In Basic the faction is chosen in the Outliner; with the Left Dock
/// closed, its strip's Outliner button opens it, and a row clicked there
/// chooses the faction whose chart Basic shows (the Faction panel says
/// it). The review's finding 11: how a faction is chosen in Basic.
#[test]
fn in_basic_a_faction_is_chosen_in_the_outliner_its_strip_opening_it() {
    let mut h = opened(Mode::Tabs, "ui-u22-fix-basic-outliner", 1600.0, Tab::Factions, 0);
    h.state_mut().settings.hide_tree = true;
    settle(&mut h);
    assert!(!h.state().faction_view.advanced, "Basic");
    assert!(crate::conversation_outliner::outliner_row(&h, "Thieves").is_none(), "closed");
    let strip =
        h.get_all_by_label(mg_ui::OUTLINER).find(|n| n.accesskit_node().children().len() == 0);
    strip.expect("the strip's Outliner button").click();
    settle(&mut h);
    crate::conversation_outliner::outliner_row(&h, "Thieves").expect("Thieves' row").click();
    settle(&mut h);
    assert_eq!(h.state().faction_view.selected, 6, "Thieves chosen");
    let named = h.query_all_by_role(egui::accesskit::Role::TextInput).any(|n| {
        let n = n.accesskit_node();
        n.label().as_deref() == Some("Name") && n.value().as_deref() == Some("Thieves")
    });
    assert!(named, "the Faction panel shows Thieves");
}

/// Where two GFF structs differ, by the labels down to each value that
/// differs (lists by their items' places).
fn differences(a: &mg_gff::Struct, b: &mg_gff::Struct, at: &str, out: &mut Vec<String>) {
    use mg_gff::Value;
    if a.id != b.id || a.fields.len() != b.fields.len() {
        out.push(format!("{at} (its fields)"));
        return;
    }
    for (x, y) in a.fields.iter().zip(&b.fields) {
        let here = format!("{at}/{}", x.label);
        match (&x.value, &y.value) {
            _ if x.label != y.label => out.push(format!("{here} (label)")),
            (Value::Struct(p), Value::Struct(q)) => differences(p, q, &here, out),
            (Value::List(p), Value::List(q)) if p.len() == q.len() => {
                for (i, (p, q)) in p.iter().zip(q).enumerate() {
                    differences(p, q, &format!("{here}[{i}]"), out);
                }
            }
            (p, q) if p != q => out.push(here),
            _ => {}
        }
    }
}

/// The module's saved journal and factions against what they were: OK
/// in each editor opened, then saved, changes no byte; a Category's Tag
/// typed in its panel and a reaction typed in the matrix, then saved,
/// change that field alone, and the files read back as the game reads
/// them (the review: saved bytes).
#[test]
fn saving_changes_only_the_fields_changed() {
    let fac = ResKey::new(ResRef::from_str("repute").unwrap(), ResType::FAC);
    let mut h = opened(Mode::Tabs, "ui-u22-fix-bytes", 1600.0, Tab::Journal, 0);
    let path =
        h.state().ws.as_ref().unwrap().module.location.as_ref().unwrap().path().to_path_buf();
    let saved = |key: &ResKey| {
        let m = Module::open(&path).unwrap();
        m.get(key).expect("written").to_vec()
    };
    let (jrl0, fac0) = (saved(&jrl()), saved(&fac));
    // Opened, OK: nothing changed.
    h.get_by_label(JOURNAL_HEADER).get_by_label("OK").click();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    h.get_by_label(FACTION_HEADER).get_by_label("OK").click();
    settle(&mut h);
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "OK changed nothing");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    settle(&mut h);
    assert!(saved(&jrl()) == jrl0 && saved(&fac) == fac0, "no byte changed by OK and a save");
    // A Tag and a reaction.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Journal));
    settle(&mut h);
    tree_row(&h, "Quest B [Category001]").click();
    settle(&mut h);
    let field = (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| n.accesskit_node().value().as_deref() == Some("Category001"))
        .expect("the Tag field")
        .rect()
        .center();
    click_pos(&h, field);
    until(&mut h, "the Tag has the keyboard", |h| {
        focused_text(h).as_deref() == Some("Category001")
    });
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("QUEST".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    h.state_mut().faction_view.advanced = true;
    settle(&mut h);
    click_pos(&h, cell(&h, "Guards", "Commoner"));
    settle(&mut h);
    let was = factions(&mut h).reputation(5, 2);
    h.event(egui::Event::Text("7".into()));
    until(&mut h, "typing in the cell", |h| cell_typing(h));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_ne!(was, Some(7));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    settle(&mut h);
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "saved");
    let read = |b: &[u8]| mg_gff::Gff::read(b).expect("read back");
    let mut out = Vec::new();
    differences(&read(&jrl0).root, &read(&saved(&jrl())).root, "", &mut out);
    assert_eq!(out, ["/Categories[1]/Tag"], "the journal: the Tag alone");
    out.clear();
    differences(&read(&fac0).root, &read(&saved(&fac)).root, "", &mut out);
    assert_eq!(out.len(), 1, "the factions: one field: {out:?}");
    assert!(out[0].ends_with("/FactionRep"), "a reaction: {out:?}");
    assert_eq!(factions(&mut h).reputation(5, 2), Some(7));
    let m = Module::open(&path).unwrap();
    assert_eq!(Factions::of_module(&m).reputation(5, 2), Some(7), "as written");
}

#[test]
fn journal_add_allows_sibling_entries_and_delete_keeps_selection() {
    let mut h = opened(Mode::Tabs, "ui-journal-add-del", 1600.0, Tab::Journal, 0);
    choose_second_entry(&mut h);
    assert_eq!(h.state().journal_view.selected, Some(Node::Entry(0, 1)));
    assert!(!h.get_by_label(JOURNAL_HEADER).get_by_label("Add").accesskit_node().is_disabled());

    // Add while an entry is selected appends a sibling entry in the same category.
    h.get_by_label(JOURNAL_HEADER).get_by_label("Add").click();
    settle(&mut h);
    assert_eq!(h.state().journal_view.selected, Some(Node::Entry(0, 2)));
    assert_eq!(mg_module::journal::entries(&categories(&mut h)[0]).len(), 3);

    // Deleting keeps focus on a remaining sibling or parent category instead of jumping to Root.
    h.get_by_label(JOURNAL_HEADER).get_by_label("Delete").click();
    settle(&mut h);
    assert_eq!(h.state().journal_view.selected, Some(Node::Entry(0, 1)));
    h.get_by_label(JOURNAL_HEADER).get_by_label("Delete").click();
    settle(&mut h);
    h.get_by_label(JOURNAL_HEADER).get_by_label("Delete").click();
    settle(&mut h);
    assert_eq!(h.state().journal_view.selected, Some(Node::Category(0)));
}
