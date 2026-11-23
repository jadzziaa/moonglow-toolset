//! A conversation's Outliner, guides and hints (M4's U15; the
//! Conversation board): its bookmarks listed in the Outliner under the
//! Left Dock's tabs, or in its window where it floats, a click or Enter
//! going to the line; the rows following their lines across a delete, its
//! undo, a move and a change another program made; the tree's indent
//! guides; the Status Bar's hints as bound and the word count; and the
//! window coming to rest.

use crate::common::*;
use crate::conversation_panels::{
    at, dialogs_window, leaf_rect, node_bar, tree_key, type_in_front,
};
use mg_module::dialog::{Kind, Parent, add_node, new_dialog, remove, word_count};
use mg_ui::dialog_view::Row;

/// The Outliner's bookmark rows drawn, each its accessible name ("Entry
/// 0: Hello there.") and where it is drawn: of every Outliner (the main
/// window's, a floating conversation's window's).
pub(crate) fn outliner_rows(h: &Harness<'_, Moonglow>) -> Vec<(String, egui::Pos2)> {
    let mut rows = Vec::new();
    for group in h.query_all_by_label(mg_ui::OUTLINER) {
        for n in group.query_all_by_label_contains(": ") {
            let name = n.accesskit_node().label().unwrap_or_default();
            rows.push((name, n.rect().center()));
        }
    }
    rows
}

/// The Outliner's row named `name`, if one is drawn.
pub(crate) fn outliner_row<'t>(
    h: &'t Harness<'_, Moonglow>,
    name: &'t str,
) -> Option<egui_kittest::Node<'t>> {
    h.query_all_by_label(mg_ui::OUTLINER).find_map(|g| g.query_by_label(name))
}

/// The names of the bookmark rows the Outliner lists, in their order.
fn listed(h: &Harness<'_, Moonglow>) -> Vec<String> {
    let mut rows = outliner_rows(h);
    rows.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
    rows.into_iter().map(|(n, _)| n).collect()
}

/// The Bookmarks group's row (or the quiet row where there is none).
fn group<'t>(h: &'t Harness<'_, Moonglow>) -> egui_kittest::Node<'t> {
    let found =
        h.get_all_by_label(mg_ui::OUTLINER).find_map(|g| g.query_by_label_contains(" bookmark"));
    found.expect("the Bookmarks group's row")
}

/// The conversation's line chosen.
pub(crate) fn chosen(h: &Harness<'_, Moonglow>, key: ResKey) -> Option<Row> {
    h.state().dialog_views[&key].selected
}

/// Bookmark on the Node section's bar pressed (`mode`'s bar).
fn bookmark(h: &mut Harness<'_, Moonglow>, mode: Mode) {
    h.get_by_label(node_bar(mode)).get_by_label("Bookmark").click();
    settle(h);
}

/// F6 pressed from the conversation's tree until the bookmarks have the
/// keys, within the window's stops.
fn f6_to_marks(h: &mut Harness<'_, Moonglow>, mode: Mode) {
    for _ in 0..10 {
        h.key_press(egui::Key::F6);
        settle(h);
        if mg_ui::palette_view::arrow_keys_are(&h.ctx) == "Marks" {
            return;
        }
    }
    panic!("{mode:?}: F6 never reaches the bookmarks");
}

/// The reply of `conversation_panels`'s conversation, which ends it.
const WHO_ENDS: &str = "Who are you? [END DIALOGUE]";

/// A conversation's bookmarks are its Outliner's (the Conversation board's
/// left dock), docked under the Left Dock's tabs and floating in its
/// window: none, a quiet row saying 0 with no arrow; each set with the
/// Node section's Bookmark listed as "Entry 0: Hello there.", in the order
/// set, and the group's count; a click on a row goes to its line, opening
/// out the line folded over it, and gives the list the keys (Up, Enter
/// going to the line; Home, Left closing the group, Right opening it;
/// Escape handing them back); F6 comes to the list; Bookmark again takes
/// a bookmark off.
#[test]
fn the_outliner_lists_the_conversation_s_bookmarks() {
    for mode in MODES {
        let (mut h, keys) = dialogs_window(mode, "ui-dialog-outliner", &["talk"], 1600.0);
        let key = keys[0];
        // None: the quiet row, no arrow (a label, not a header).
        let none = group(&h);
        let said = none.accesskit_node().label().or(none.accesskit_node().value());
        assert_eq!(said.as_deref(), Some("0 bookmarks"), "{mode:?}");
        assert_eq!(none.accesskit_node().role(), egui::accesskit::Role::Label, "{mode:?}");
        assert!(listed(&h).is_empty());
        // Floating, in its window; docked, in the main window's Left Dock.
        let room = match mode {
            Mode::Tabs => h.state().left_dock_rect().expect("the Left Dock"),
            Mode::Floating => leaf_rect(&h, key),
        };
        assert!(room.contains(group(&h).rect().center()), "{mode:?}: in {room:?}");
        // Docked, the Outliner stands 166 high, the board's, the seam not
        // dragged: its header, then the group's row 2 points under it and
        // the room egui leaves between them (the area's default seam
        // leaves it about 350).
        if mode == Mode::Tabs {
            let row = group(&h).rect();
            let header = mg_uikit::metrics::DOCK_TABS;
            let high = room.bottom() - row.top() + header + 2.0;
            assert!((high - 166.0).abs() <= 5.0, "the board's 166: {high}");
        }
        // The keys on the list of none (F6 to it): each key does nothing,
        // and nothing breaks.
        h.get_by_label("Root").click();
        settle(&mut h);
        f6_to_marks(&mut h, mode);
        for k in [egui::Key::Enter, egui::Key::ArrowDown, egui::Key::ArrowRight, egui::Key::End] {
            h.key_press(k);
            settle(&mut h);
        }
        assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks", "{mode:?}: still its");
        assert_eq!(chosen(&h, key), None, "{mode:?}: nothing gone to");
        // Two set, in their order; a third as the view keeps them.
        for line in ["[OWNER] - Hello there.", "[OWNER] - Go away."] {
            h.get_by_label(line).click();
            settle(&mut h);
            bookmark(&mut h, mode);
        }
        let set = ["Entry 0: Hello there.", "Entry 1: Go away."];
        assert_eq!(listed(&h), set, "{mode:?}");
        let said = group(&h).accesskit_node().label();
        assert_eq!(said.as_deref(), Some("2 bookmarks"), "{mode:?}");
        // (Its count drawn at the row's right.)
        let row = group(&h).rect();
        let count = (h.output().shapes.iter()).any(|c| match &c.shape {
            egui::Shape::Text(s) => {
                s.galley.text() == "2" && row.contains(s.visual_bounding_rect().center())
            }
            _ => false,
        });
        assert!(count, "{mode:?}: the count 2 drawn on its row");
        h.state_mut().dialog_views.get_mut(&key).unwrap().bookmarks.push((Kind::Reply, 0));
        settle(&mut h);
        assert_eq!(listed(&h), [set[0], set[1], "Reply 0: Who are you?"], "{mode:?}");
        // Hello folded, Root chosen: a click on Who's row goes to it, Hello
        // opened out.
        h.get_by_label("Collapse [OWNER] - Hello there.").click();
        settle(&mut h);
        h.get_by_label("Root").click();
        settle(&mut h);
        assert!(h.query_by_label(WHO_ENDS).is_none(), "{mode:?}: folded away");
        outliner_row(&h, "Reply 0: Who are you?").unwrap().click();
        settle(&mut h);
        let who = Row { parent: Parent::Node(Kind::Entry, 0), pos: 0 };
        assert_eq!(chosen(&h, key), Some(who), "{mode:?}: the click went to its line");
        assert!(!h.state().dialog_views[&key].collapsed.contains(&(Kind::Entry, 0)));
        h.get_by_label(WHO_ENDS);
        // The keys: the list's after the click.
        assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks", "{mode:?}");
        h.key_press(egui::Key::ArrowUp);
        settle(&mut h);
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        let go = Row { parent: Parent::Root, pos: 1 };
        assert_eq!(chosen(&h, key), Some(go), "{mode:?}: Up and Enter");
        h.key_press(egui::Key::Home);
        settle(&mut h);
        h.key_press(egui::Key::ArrowLeft);
        settle(&mut h);
        assert!(listed(&h).is_empty(), "{mode:?}: the group closed");
        h.key_press(egui::Key::ArrowRight);
        settle(&mut h);
        assert_eq!(listed(&h).len(), 3, "{mode:?}: opened");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Area", "{mode:?}: given back");
        // F6 comes to the list (from the editor, within its stops).
        h.get_by_label("Root").click();
        settle(&mut h);
        f6_to_marks(&mut h, mode);
        // Bookmark on a bookmarked line takes it off.
        outliner_row(&h, "Entry 0: Hello there.").unwrap().click();
        settle(&mut h);
        bookmark(&mut h, mode);
        assert_eq!(listed(&h), ["Entry 1: Go away.", "Reply 0: Who are you?"], "{mode:?}");
    }
}

/// A conversation another program changes (a plugin, the file changed
/// outside Moonglow): "Go away." (entry 2) bookmarked, then the lines
/// written anew in another order, Go away first; "A traveller." deleted
/// elsewhere. The deferred entry's rule ("Bookmarks and folded lines
/// across a change Moonglow didn't make"): each found again by its text
/// and speaker and those of the lines over it up to Root.
fn reordered() -> mg_gff::Gff {
    let mut g = new_dialog();
    let go = add_node(&mut g, Parent::Root, "Go away.");
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    let traveller = add_node(&mut g, Parent::Node(Kind::Reply, who), "A traveller.");
    add_node(&mut g, Parent::Node(Kind::Entry, traveller), "Farewell.");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Bye.");
    mg_module::dialog::add_link(&mut g, Parent::Node(Kind::Entry, go), who);
    g
}

/// The conversation of `conversations.rs`'s branching tests: Hello there
/// (Who are you, its traveller and Farewell; Bye), Go away linking Who.
fn branching() -> mg_gff::Gff {
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    let traveller = add_node(&mut g, Parent::Node(Kind::Reply, who), "A traveller.");
    add_node(&mut g, Parent::Node(Kind::Entry, traveller), "Farewell.");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Bye.");
    let go = add_node(&mut g, Parent::Root, "Go away.");
    mg_module::dialog::add_link(&mut g, Parent::Node(Kind::Entry, go), who);
    let words = word_count(&g);
    g.root.set("NumWords", mg_gff::Value::Dword(words));
    g
}

/// The branching conversation `talk` open As Tabs in a window 1600 by
/// 900, its tree with the keys.
pub(crate) fn branching_window(name: &str) -> (Harness<'static, Moonglow>, ResKey) {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &branching()).unwrap();
    m.save().unwrap();
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.settings.dialog_no_backup = true;
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    settle(&mut h);
    (h, key)
}

/// A change of conversation `key` made elsewhere (a plugin's, as the file
/// changed outside Moonglow comes): the conversation set whole to `g`.
fn changed_elsewhere(h: &mut Harness<'_, Moonglow>, key: ResKey, g: &mg_gff::Gff) {
    let edit = mg_edit::Edit::SetResource { key, data: g.to_bytes().ok() };
    let command = mg_edit::Command::new("Plugin", vec![edit]);
    h.state_mut().actions.push(mg_ui::Action::Apply(command));
    settle(h);
}

/// The Outliner's rows follow their lines (the plan's test): across a
/// line moved by another program (the lines written anew in another
/// order: each found again by its text and speaker up to Root, as the
/// deferred entry "Bookmarks and folded lines across a change Moonglow
/// didn't make" has it), a delete made elsewhere and its undo, the
/// editor's own delete and its undo, and a move in the tree (Alt+Down,
/// which numbers nothing anew).
#[test]
fn bookmarks_follow_a_line_moved_by_another_program() {
    let (mut h, key) = branching_window("ui-dialog-outliner-follow");
    for label in ["[OWNER] - Go away.", "Farewell. [END DIALOGUE]", "Bye. [END DIALOGUE]"] {
        h.get_by_label(label).click();
        settle(&mut h);
        bookmark(&mut h, Mode::Tabs);
    }
    let first = ["Entry 2: Go away.", "Reply 1: Farewell.", "Reply 2: Bye."];
    assert_eq!(listed(&h), first);
    // Another program writes the lines in another order: Go away is entry
    // 0 now, Farewell and Bye keep their numbers.
    changed_elsewhere(&mut h, key, &reordered());
    assert_eq!(listed(&h), ["Entry 0: Go away.", "Reply 1: Farewell.", "Reply 2: Bye."]);
    // Undone: back where they were.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(listed(&h), first, "undone");
    // A delete made elsewhere: the traveller and Farewell under it gone,
    // Go away entry 1, Bye reply 1.
    let mut g = branching();
    remove(&mut g, Parent::Node(Kind::Reply, 0), 0);
    changed_elsewhere(&mut h, key, &g);
    assert_eq!(listed(&h), ["Entry 1: Go away.", "Reply 1: Bye."]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert_eq!(listed(&h), first, "undone, Farewell back in its place");
    // The editor's own delete of a bookmarked line, and its undo.
    h.get_by_label("Farewell. [END DIALOGUE]").click();
    settle(&mut h);
    tree_key(&mut h, egui::Modifiers::NONE, egui::Key::Delete);
    assert_eq!(listed(&h), ["Entry 2: Go away.", "Reply 1: Bye."]);
    tree_key(&mut h, egui::Modifiers::COMMAND, egui::Key::Z);
    assert_eq!(listed(&h), first);
    // A move (Alt+Down: Hello's replies in another order): no line
    // numbered anew, the rows as they were, Bye now first under Hello.
    // (The line's own row, over the link to it under Go away.)
    h.get_all_by_label("Who are you?").next().unwrap().click();
    settle(&mut h);
    tree_key(&mut h, egui::Modifiers::ALT, egui::Key::ArrowDown);
    let lines = dialog_lines(&mut h, key);
    assert_eq!(lines[1], "  Reply|Bye.|", "moved: {lines:?}");
    assert_eq!(listed(&h), first, "a move numbers nothing anew");
}

/// The x of the vertical lines drawn in the theme's `guide` color, each
/// with its span, as the last frame painted them.
pub(crate) fn guides_drawn(h: &Harness<'_, Moonglow>) -> Vec<(f32, egui::Rangef)> {
    let t = mg_uikit::tokens(&h.ctx);
    guides_in(h, &[t.guide, t.guide_hover, t.guide_on_fill])
}

/// [`guides_drawn`] in the colors `colors`.
fn guides_in(h: &Harness<'_, Moonglow>, colors: &[egui::Color32]) -> Vec<(f32, egui::Rangef)> {
    (h.output().shapes.iter())
        .filter_map(|c| match &c.shape {
            egui::Shape::LineSegment { points: [a, b], stroke }
                if colors.contains(&stroke.color) && (a.x - b.x).abs() < 0.01 =>
            {
                Some((a.x, egui::Rangef::new(a.y.min(b.y), a.y.max(b.y))))
            }
            _ => None,
        })
        .collect()
}

/// Whether a guide is drawn at `x` (within a pixel) down the whole of
/// `row`'s height.
fn guide_at(h: &Harness<'_, Moonglow>, x: f32, row: egui::Rect) -> bool {
    guides_drawn(h).iter().any(|(at, span)| {
        (at - x).abs() <= 1.0 && span.min <= row.top() && span.max >= row.bottom()
    })
}

/// The conversation tree's indent guides (the Conversation board's): a
/// row four levels under Root has a line down the middle of the arrow of
/// each line it is under, Root's place too, as high as the row; at the
/// editor's larger text and on the line chosen too; a branch folded draws
/// none for what it hides; a row in sight, not one scrolled out of it.
#[test]
fn the_conversation_tree_draws_indent_guides() {
    let (mut h, key) = branching_window("ui-dialog-guides");
    for larger in [None, Some(20)] {
        h.state_mut().settings.dialog_font_size = larger;
        settle(&mut h);
        let what = format!("{larger:?}");
        let root = at(&h, "Root");
        let farewell = at(&h, "Farewell. [END DIALOGUE]");
        // (Its row from the tree's left edge, as high as its label.)
        let row = egui::Rect::from_x_y_ranges(root.left()..=farewell.right(), farewell.y_range());
        // (Each level's arrow's middle: 16 points a level, the arrow 22.)
        let x = |level: f32| root.left() + level * 16.0 + 11.0;
        for level in 0..4 {
            assert!(guide_at(&h, x(level as f32), row), "{what}: level {level} at Farewell");
        }
        assert!(!guide_at(&h, x(4.0), row), "{what}: none past its own level");
        // Root's row has none.
        assert!(!guides_drawn(&h).iter().any(|(_, s)| s.contains(root.center().y)), "{what}");
        // Chosen: drawn as before.
        h.get_by_label("Farewell. [END DIALOGUE]").click();
        settle(&mut h);
        assert_eq!(
            chosen(&h, key),
            Some(Row { parent: Parent::Node(Kind::Entry, 1), pos: 0 }),
            "{what}"
        );
        for level in 0..4 {
            assert!(guide_at(&h, x(level as f32), row), "{what}: chosen, level {level}");
        }
        // The chosen row filled the tree's whole width (the board's), its
        // guides lighter on it.
        let t = mg_uikit::tokens(&h.ctx);
        let filled = (h.output().shapes.iter()).any(|c| match &c.shape {
            egui::Shape::Rect(r) => {
                r.fill == t.accent_fill
                    && r.rect.left() <= root.left() + 0.5
                    && r.rect.contains(farewell.center())
            }
            _ => false,
        });
        assert!(filled, "{what}: the chosen row filled from the tree's left");
        let light = guides_in(&h, &[t.guide_on_fill]);
        assert!(light.iter().any(|(_, s)| s.contains(farewell.center().y)), "{what}: lighter");
        // Under the pointer, a row's guides its own, in sight on its fill.
        let hello = at(&h, "[OWNER] - Hello there.");
        h.hover_at(egui::pos2(hello.right() - 2.0, hello.center().y));
        h.run();
        let hover = guides_in(&h, &[t.guide_hover]);
        assert!(hover.iter().any(|(_, s)| s.contains(hello.center().y)), "{what}: hovered");
    }
    // Hello folded: nothing under it drawn, and no guide where its rows
    // stood.
    h.state_mut().settings.dialog_font_size = None;
    settle(&mut h);
    let who = at(&h, "Who are you?");
    h.get_by_label("Collapse [OWNER] - Hello there.").click();
    settle(&mut h);
    let hello = at(&h, "[OWNER] - Hello there.");
    let root = at(&h, "Root");
    let row = egui::Rect::from_x_y_ranges(root.left()..=hello.right(), hello.y_range());
    assert!(guide_at(&h, root.left() + 11.0, row), "Hello's own");
    let deeper = guides_drawn(&h).into_iter().filter(|(x, _)| *x > root.left() + 20.0);
    assert!(
        !deeper.into_iter().any(|(_, s)| s.contains(who.center().y)),
        "nothing where Who stood"
    );
}

/// The Status Bar beside a conversation (the Conversation board's): Drag
/// Move line, Ctrl + Drag Link, Double Click Go to linked, Ctrl + A Add,
/// Alt + Up Move up, * Open branch, Ctrl + F Find, and the file's words;
/// a key remapped shows remapped, an unbound one drops its hint. Each
/// does what it says: a double click on a link goes to the line, `*` opens
/// a folded branch, Alt + Up moves the line up. Typing in Data leaves the
/// pointer's alone; the bookmarks with the keys and the pointer away, no
/// `*` (the tree's), no Add; a menu open, none, the count kept.
#[test]
fn a_conversation_s_hints_are_as_bound() {
    let (mut h, key) = branching_window("ui-dialog-hints");
    let words = format!("{} words", word_count(&branching()));
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(&mut h);
    let all = [
        "Drag: Move line",
        "Ctrl, Drag: Link",
        "Double Click: Go to linked",
        "Ctrl, A: Add",
        "Alt, Up: Move up",
        "*: Open branch",
        "Ctrl, F: Find",
        "F5: Bookmark",
        words.as_str(),
    ];
    says(&mut h, &all, &[]);
    assert_eq!(h.state().status_said(), all, "in the board's order, nothing else");
    // Remapped, and unbound.
    let shift_a = egui::KeyboardShortcut::new(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::A,
    );
    h.state_mut().keymap.set(mg_ui::keys::Cmd::AddLine, vec![shift_a]);
    h.state_mut().keymap.set(mg_ui::keys::Cmd::DialogFind, Vec::new());
    says(&mut h, &["Ctrl, Shift, A: Add"], &["Ctrl, A: Add", "Ctrl, F: Find"]);
    h.state_mut().keymap = mg_ui::keys::Keymap::default();
    says(&mut h, &all, &[]);
    // Each does what it says. Double Click on the link (Go away's Who) goes
    // to the line it links to.
    let link = h
        .get_all_by_label("Who are you?")
        .map(|n| n.rect())
        .max_by(|a, b| a.top().total_cmp(&b.top()));
    let link = link.expect("the link's row").center();
    double_click_now(&mut h, link);
    settle(&mut h);
    let who = Row { parent: Parent::Node(Kind::Entry, 0), pos: 0 };
    assert_eq!(chosen(&h, key), Some(who), "Double Click went to the linked line");
    // `*` opens a folded branch (Who folded, chosen).
    h.get_by_label("Collapse Who are you?").click();
    settle(&mut h);
    assert!(h.state().dialog_views[&key].collapsed.contains(&(Kind::Reply, 0)));
    h.event(egui::Event::Text("*".into()));
    settle(&mut h);
    assert!(h.state().dialog_views[&key].collapsed.is_empty(), "* opened the branch");
    // Alt + Up: Bye up before Who.
    h.get_by_label("Bye. [END DIALOGUE]").click();
    settle(&mut h);
    tree_key(&mut h, egui::Modifiers::ALT, egui::Key::ArrowUp);
    assert_eq!(dialog_lines(&mut h, key)[1], "  Reply|Bye.|", "Alt + Up moved it up");
    // Typing in Data: the keys' hints go, the pointer's stay.
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(&mut h);
    let text = h
        .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Hello there."))
        .expect("the line's text")
        .rect();
    h.hover_at(text.center());
    h.step();
    press(&h, text.center(), true, egui::Modifiers::NONE);
    h.step();
    press(&h, text.center(), false, egui::Modifiers::NONE);
    settle(&mut h);
    assert!(h.ctx.text_edit_focused(), "typing in Data");
    let keys = ["Ctrl, A: Add", "Alt, Up: Move up", "*: Open branch", "Ctrl, F: Find"];
    says(&mut h, &["Drag: Move line", "Double Click: Go to linked", words.as_str()], &keys);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // A bookmark's row with the keys, the pointer away from the editor:
    // neither the tree's `*` nor the commands' keys (Add acts with the
    // pointer over the editor), the pointer's ways kept.
    h.get_by_label(node_bar(Mode::Tabs)).get_by_label("Bookmark").click();
    settle(&mut h);
    outliner_row(&h, "Entry 0: Hello there.").unwrap().click();
    settle(&mut h);
    let away = h.state().left_dock_rect().unwrap().center();
    h.hover_at(away);
    settle(&mut h);
    says(&mut h, &["Drag: Move line", words.as_str()], &keys);
    // Over the editor: still none of them, the list keeping the keys (M4's
    // U15 review: the commands do nothing there meanwhile).
    h.hover_at(at(&h, "Root").center());
    settle(&mut h);
    let none = ["Ctrl, A: Add", "Ctrl, F: Find", "*: Open branch"];
    says(&mut h, &["Drag: Move line"], &none);
    // At 1400, the board's yielding (U13's ruling): the hints drop whole
    // from the last before the switches lose their words.
    h.state_mut().log.info("talk.dlg: a line long enough to take its room in the Status Bar");
    h.set_size(egui::vec2(1400.0, 900.0));
    settle(&mut h);
    let words_kept = h.get_by_label("Status Bar").get_by_label("Log").rect().width() > 30.0;
    assert!(words_kept, "1400: the switches' words");
    // (Of the hints said, those drawn: the first ones, in order.)
    let said: Vec<String> =
        h.state().status_said().into_iter().filter(|s| s.contains(": ")).collect();
    let drawn = |w: &String| h.get_by_label("Status Bar").query_by_label(w).is_some();
    let shown: Vec<String> = said.iter().filter(|w| drawn(w)).cloned().collect();
    assert!(!shown.is_empty() && said.starts_with(&shown), "from the last: {shown:?} of {said:?}");
    h.set_size(egui::vec2(1600.0, 900.0));
    settle(&mut h);
    // A menu open: no hints, the count kept.
    h.get_all_by_label("File").next().unwrap().click();
    settle(&mut h);
    says(&mut h, &[words.as_str()], &["Drag: Move line", "Double Click: Go to linked"]);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
}

/// Hello's bookmark's row once "x" is typed after its text (Data's,
/// `type_in_front`): the row says the line's text as it is now.
const HELLO_TYPED: &str = "Entry 0: Hello there.x";

/// Hello's row in the tree, whatever was typed after its text (not its
/// arrow, named with it).
fn hello_row<'t>(h: &'t Harness<'_, Moonglow>) -> egui_kittest::Node<'t> {
    let row = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().label().is_some_and(|l| l.starts_with("[OWNER] - Hello"))
    };
    h.get_all_by_label_contains("[OWNER] - Hello").find(row).expect("Hello's row")
}

/// The window comes to rest with the Outliner's bookmarks in front (As
/// Tabs, the Module panel over them), behind (the Left Dock's Resources
/// tab in front of the Module panel, the Outliner under it), folded (the
/// Bookmarks group closed), the Left Dock closed, and floating (its
/// window's Outliner): Data typed in, the selection moved with the keys,
/// a bookmark set and taken off, a bookmarked line deleted and the delete
/// undone, each settles (an Outliner asking for a frame each frame fails
/// as "keeps repainting"); and the last step shows the bookmark back.
#[test]
fn the_window_comes_to_rest_with_the_outliner_s_bookmarks() {
    let cases = [
        (Mode::Tabs, "front"),
        (Mode::Tabs, "behind"),
        (Mode::Tabs, "folded"),
        (Mode::Tabs, "closed"),
        (Mode::Floating, "window"),
    ];
    for (mode, case) in cases {
        let (mut h, keys) = dialogs_window(mode, "ui-dialog-outliner-rest", &["talk"], 1600.0);
        let key = keys[0];
        h.get_by_label("[OWNER] - Hello there.").click();
        settle(&mut h);
        bookmark(&mut h, mode);
        match case {
            "behind" => {
                h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Resources));
                settle(&mut h);
            }
            "folded" => {
                group(&h).click();
                settle(&mut h);
                assert!(listed(&h).is_empty(), "folded");
            }
            "closed" => {
                h.state_mut().settings.hide_tree = true;
                settle(&mut h);
            }
            _ => {}
        }
        // (Drawn where the case has it.)
        let shown = !matches!(case, "folded" | "closed");
        assert_eq!(!listed(&h).is_empty(), shown, "{mode:?} {case}: listed");
        type_in_front(&mut h, "Data");
        hello_row(&h).click();
        settle(&mut h);
        let before = chosen(&h, key);
        // (Root, then down through Who, a reply that ends the
        // conversation, to Go away: U14's Data never came to rest beside
        // Who docked, found by U15.)
        tree_key(&mut h, egui::Modifiers::NONE, egui::Key::Home);
        for k in [egui::Key::ArrowDown, egui::Key::ArrowDown, egui::Key::ArrowDown] {
            tree_key(&mut h, egui::Modifiers::NONE, k);
        }
        assert_ne!(chosen(&h, key), before, "{mode:?} {case}: the keys moved the selection");
        // A bookmark set on Go away and taken off again.
        h.get_by_label("[OWNER] - Go away.").click();
        settle(&mut h);
        bookmark(&mut h, mode);
        bookmark(&mut h, mode);
        // Hello (bookmarked) deleted, the delete undone.
        hello_row(&h).click();
        settle(&mut h);
        tree_key(&mut h, egui::Modifiers::NONE, egui::Key::Delete);
        let lines = dialog_lines(&mut h, key);
        assert!(
            h.state().dialog_views[&key].bookmarks.is_empty(),
            "{mode:?} {case}: gone: {:?} {:?} {lines:?}",
            h.state().dialog_views[&key].bookmarks,
            chosen(&h, key),
        );
        if shown {
            assert!(listed(&h).is_empty(), "{mode:?} {case}: its row gone");
        }
        tree_key(&mut h, egui::Modifiers::COMMAND, egui::Key::Z);
        assert_eq!(h.state().dialog_views[&key].bookmarks, [(Kind::Entry, 0)], "{case}");
        if shown {
            assert_eq!(listed(&h), [HELLO_TYPED], "{mode:?} {case}: back");
        }
        if case == "folded" {
            group(&h).click();
            settle(&mut h);
            assert_eq!(listed(&h), [HELLO_TYPED], "{mode:?} {case}: opened, back");
        }
    }
}

/// A long list of bookmarks scrolls: of 30, the last is out of the
/// Outliner's sight at first; End with the list's keys brings it there.
#[test]
fn a_long_list_of_bookmarks_scrolls() {
    let dir = mg_testkit::scratch_dir("ui-dialog-outliner-long");
    let path = sample_module(&dir);
    let key = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    for i in 0..30 {
        add_node(&mut g, Parent::Root, &format!("Line {i:02}."));
    }
    let mut m = Module::open(&path).unwrap();
    m.set_gff(key, &g).unwrap();
    m.save().unwrap();
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.settings.dialog_no_backup = true;
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(key)));
    settle(&mut h);
    let marks = (0..30).map(|i| (Kind::Entry, i)).collect();
    h.state_mut().dialog_views.get_mut(&key).unwrap().bookmarks = marks;
    settle(&mut h);
    let dock = h.state().left_dock_rect().expect("the Left Dock");
    let in_sight = |h: &Harness<'_, Moonglow>, name: &str| {
        outliner_row(h, name).is_some_and(|n| dock.contains_rect(n.rect()))
    };
    assert!(in_sight(&h, "Entry 0: Line 00."), "the first in sight");
    assert!(!in_sight(&h, "Entry 29: Line 29."), "the last out of sight at first");
    outliner_row(&h, "Entry 0: Line 00.").unwrap().click();
    settle(&mut h);
    h.key_press(egui::Key::End);
    settle(&mut h);
    assert!(in_sight(&h, "Entry 29: Line 29."), "End: the last in sight");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(chosen(&h, key), Some(Row { parent: Parent::Root, pos: 29 }), "Enter goes to it");
}

// ---------------------------------------------------------------- review

/// A module with the conversations `names`, each `g`, open in `mode` in
/// a window `size`, the window sizes kept `kept` (a kind's remembered
/// window), their tabs opened in their order.
pub(crate) fn window_with(
    mode: Mode,
    name: &str,
    names: &[&str],
    g: &mg_gff::Gff,
    size: egui::Vec2,
    kept: Vec<(String, [u32; 2])>,
) -> (Harness<'static, Moonglow>, Vec<ResKey>) {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let keys: Vec<ResKey> = names.iter().map(|n| ResKey::parse(n, ResType::DLG).unwrap()).collect();
    for key in &keys {
        m.set_gff(*key, g).unwrap();
    }
    m.save().unwrap();
    let mut app = app_in(mode, Vec::new());
    app.settings.no_last_area = true;
    app.settings.dialog_no_backup = true;
    app.settings.window_sizes = kept;
    app.open_module(&path);
    let mut h = harness(app, size.x, size.y);
    settle(&mut h);
    for key in &keys {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(*key)));
        settle(&mut h);
    }
    (h, keys)
}

/// Two greetings, the first with a reply: `conversation_panels`' talk.
fn talk() -> mg_gff::Gff {
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    add_node(&mut g, Parent::Root, "Go away.");
    g
}

/// Thirty greetings, "Line 00." to "Line 29.".
fn thirty() -> mg_gff::Gff {
    let mut g = new_dialog();
    for i in 0..30 {
        add_node(&mut g, Parent::Root, &format!("Line {i:02}."));
    }
    g
}

/// How far a header's item has yielded: on its row in its first form, on
/// it in its yielded form, gone into More (or cut).
fn yielded(place: mg_uikit::header::Place) -> u8 {
    use mg_uikit::header::Place;
    match place {
        Place::Row | Place::Unfolded => 0,
        Place::Icon => 1,
        Place::More | Place::Cut => 2,
    }
}

/// Where the conversation header's items went at the tip (`d11e8657`,
/// before the window held an Outliner), both in a new window in a 1280 by
/// 720 screen and in a window the kind kept at 920 by 700: recorded there
/// by `target/u15/u15_header_probe.rs`.
const TIP_HEADER: [(&str, u8); 20] = [
    ("Add", 1),
    ("Copy", 1),
    ("Cut", 1),
    ("Paste", 1),
    ("Paste As Link", 1),
    ("Delete", 1),
    ("Move Up", 1),
    ("Move Down", 1),
    ("Expand All", 2),
    ("Collapse All", 2),
    ("Highlight", 0),
    ("Comments", 0),
    ("Actions", 0),
    ("Quests", 0),
    ("Animations", 0),
    ("Sounds", 0),
    ("Scripts", 1),
    ("Export", 1),
    ("Import Lines…", 1),
    ("Test", 1),
];

/// A floating conversation's header yields no sooner than before its
/// window held an Outliner (the coordinator's ruling at U15's review: the
/// pane gives way before the header does): a new window in a 1280 by 720
/// screen (it opens wider there), and a window the kind kept at 920 by 700
/// (left as it is), each item where the tip had it or less yielded. In the
/// kept window the Outliner is folded to a strip at the tree's left: its
/// icon unfolds the bookmarks over the tree, a click on one goes to its
/// line, and the icon folds them again. In a window with the room, a 1920
/// screen's new one, the pane is drawn, 260 wide, and no strip.
#[test]
fn a_floating_conversation_s_header_yields_no_sooner_than_before() {
    let cases = [
        ("new", egui::vec2(1280.0, 720.0), Vec::new()),
        ("kept", egui::vec2(1600.0, 900.0), vec![("dialog".to_string(), [920u32, 700])]),
    ];
    for (case, size, kept) in cases {
        let (mut h, keys) = window_with(
            Mode::Floating,
            &format!("ui-dialog-header-{case}"),
            &["talk"],
            &talk(),
            size,
            kept,
        );
        let key = keys[0];
        let header = mg_ui::dialog_view::CONVERSATION_HEADER;
        let rec = mg_uikit::header::records(&h.ctx, header).into_iter().next().expect("its header");
        for (name, tip) in TIP_HEADER {
            let place = rec.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p);
            let place = place.unwrap_or_else(|| panic!("{case}: {name} not in the header"));
            assert!(yielded(place) <= tip, "{case}: {name} {place:?}, the tip's {tip}");
        }
        if case == "kept" {
            // Folded to its strip: no pane, the strip's icon.
            assert!(group_of(&h).is_none(), "kept: no pane");
            h.state_mut().dialog_views.get_mut(&key).unwrap().bookmarks = vec![(Kind::Entry, 1)];
            settle(&mut h);
            let strip = h.get_by_label(mg_ui::dialog_view::OUTLINER_STRIP);
            let icon = strip.get_by_label(mg_ui::OUTLINER).rect();
            let tree = at(&h, "Root");
            assert!(icon.right() <= tree.left() + 0.5, "the strip at the tree's left: {icon:?}");
            h.get_by_label(mg_ui::dialog_view::OUTLINER_STRIP)
                .get_by_label(mg_ui::OUTLINER)
                .click();
            settle(&mut h);
            assert_eq!(listed(&h), ["Entry 1: Go away."], "unfolded over the tree");
            outliner_row(&h, "Entry 1: Go away.").unwrap().click();
            settle(&mut h);
            assert_eq!(chosen(&h, key), Some(Row { parent: Parent::Root, pos: 1 }));
            h.get_by_label(mg_ui::dialog_view::OUTLINER_STRIP)
                .get_by_label(mg_ui::OUTLINER)
                .click();
            settle(&mut h);
            assert!(listed(&h).is_empty(), "folded again");
        } else {
            // (The window 0.85 of the screen's room: the pane where it
            // fits beside the tree's room, its strip where not.)
            let strip = h.query_by_label(mg_ui::dialog_view::OUTLINER_STRIP).is_some();
            assert!(group_of(&h).is_some() || strip, "new: the bookmarks in reach");
        }
    }
    let (h, _) = window_with(
        Mode::Floating,
        "ui-dialog-header-wide",
        &["talk"],
        &talk(),
        egui::vec2(1920.0, 1080.0),
        Vec::new(),
    );
    let row = group(&h).rect();
    assert!((250.0..=262.0).contains(&row.width()), "the pane 260 at first: {row:?}");
}

/// A floating conversation's Outliner folded to its strip (its window
/// too narrow for the pane beside the tree's room) unfolded over the
/// tree, where it is so; else nothing.
pub(crate) fn unfold_window_outliner(h: &mut Harness<'_, Moonglow>) {
    let strip = h.query_by_label(mg_ui::dialog_view::OUTLINER_STRIP);
    if let Some(strip) = strip
        && group_of(h).is_none()
    {
        strip.get_by_label(mg_ui::OUTLINER).click();
        settle(h);
    }
}

/// The Bookmarks group's row, or none where no Outliner lists bookmarks.
fn group_of<'t>(h: &'t Harness<'_, Moonglow>) -> Option<egui_kittest::Node<'t>> {
    h.query_all_by_label(mg_ui::OUTLINER).find_map(|g| g.query_by_label_contains(" bookmark"))
}

/// Of two conversations' lists, one docked and one floating, each brings
/// its own keys' row into sight (the review's probe: End on the floating
/// one's left its row 29 out of sight, the docked list drawn first having
/// taken the move).
#[test]
fn each_list_brings_its_own_row_into_sight() {
    let (mut h, keys) = window_with(
        Mode::Tabs,
        "ui-dialog-two-lists",
        &["aaa", "bbb"],
        &thirty(),
        egui::vec2(1600.0, 900.0),
        Vec::new(),
    );
    own_window(&mut h, &Tab::Dialog(keys[1]), egui::pos2(300.0, 150.0), egui::vec2(1250.0, 500.0));
    settle(&mut h);
    for k in &keys {
        h.state_mut().dialog_views.get_mut(k).unwrap().bookmarks =
            (0..30).map(|i| (Kind::Entry, i)).collect();
    }
    settle(&mut h);
    let win = leaf_rect(&h, keys[1]);
    let in_window = |h: &Harness<'_, Moonglow>, name: &str| {
        outliner_rows(h).into_iter().find(|(n, at)| n == name && win.contains(*at)).map(|r| r.1)
    };
    let first = in_window(&h, "Entry 0: Line 00.").expect("the floating one's list");
    h.hover_at(first);
    h.step();
    press(&h, first, true, egui::Modifiers::NONE);
    h.step();
    press(&h, first, false, egui::Modifiers::NONE);
    settle(&mut h);
    h.key_press(egui::Key::End);
    settle(&mut h);
    let last = outliner_row_in(&h, "Entry 29: Line 29.", win).expect("listed");
    assert!(
        win.contains_rect(last),
        "End: the floating list's last row in sight: {last:?} in {win:?}"
    );
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(chosen(&h, keys[1]), Some(Row { parent: Parent::Root, pos: 29 }));
    assert_eq!(chosen(&h, keys[0]), None, "the docked one's as it was");
}

/// The rectangle of the Outliner's row `name` whose middle is in `room`.
fn outliner_row_in(h: &Harness<'_, Moonglow>, name: &str, room: egui::Rect) -> Option<egui::Rect> {
    let groups = h.query_all_by_label(mg_ui::OUTLINER);
    groups
        .flat_map(|g| g.query_all_by_label(name).map(|n| n.rect()).collect::<Vec<_>>())
        .find(|r| room.contains(r.center()))
}

/// While the bookmarks hold the keys, the conversation's commands do
/// nothing with the pointer alone over its tree (the review: Delete, Add,
/// Copy, Cut, Paste and Alt + Up acted there): the list keeps them, and
/// the Status Bar hints none of them; a click in the tree gives them back.
#[test]
fn the_bookmarks_keep_the_keys_from_the_tree_under_the_pointer() {
    let (mut h, key) = branching_window("ui-dialog-marks-keep");
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(&mut h);
    bookmark(&mut h, Mode::Tabs);
    let lines = dialog_lines(&mut h, key);
    outliner_row(&h, "Entry 0: Hello there.").unwrap().click();
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks");
    let clip = h.state().dialog_clip.clone();
    h.hover_at(at(&h, "[OWNER] - Hello there.").center());
    settle(&mut h);
    for (mods, k) in [
        (egui::Modifiers::NONE, egui::Key::Delete),
        (egui::Modifiers::COMMAND, egui::Key::A),
        (egui::Modifiers::ALT, egui::Key::ArrowUp),
        (egui::Modifiers::ALT, egui::Key::ArrowDown),
    ] {
        h.key_press_modifiers(mods, k);
        settle(&mut h);
    }
    for e in [egui::Event::Copy, egui::Event::Cut] {
        h.event(e);
        settle(&mut h);
    }
    assert_eq!(dialog_lines(&mut h, key), lines, "nothing done to the conversation");
    assert_eq!(h.state().dialog_clip, clip, "nothing copied");
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks", "the list's still");
    let said = h.state().status_said();
    assert!(!said.iter().any(|s| s.ends_with(": Add")), "{said:?}");
    // A click in the tree: its keys, and Delete deletes there.
    h.get_by_label("[OWNER] - Go away.").click();
    settle(&mut h);
    tree_key(&mut h, egui::Modifiers::NONE, egui::Key::Delete);
    assert!(dialog_lines(&mut h, key).len() < lines.len(), "deleted from the tree");
}

/// F6 goes left to right, then down (T14's order): to a floating
/// conversation's window's Outliner before its right pane.
#[test]
fn f6_comes_to_a_window_s_outliner_before_its_right_pane() {
    let (mut h, keys) = window_with(
        Mode::Floating,
        "ui-dialog-f6-window",
        &["talk"],
        &talk(),
        egui::vec2(1600.0, 900.0),
        Vec::new(),
    );
    h.get_by_label("Root").click();
    settle(&mut h);
    let mut seen = Vec::new();
    for _ in 0..8 {
        h.key_press(egui::Key::F6);
        settle(&mut h);
        seen.push(mg_ui::palette_view::arrow_keys_are(&h.ctx));
    }
    // (The window's Outliner, then its editor, then its right pane.)
    let marks = seen.iter().position(|s| s == "Marks").unwrap_or_else(|| panic!("{seen:?}"));
    let after: Vec<&str> = seen[marks..].iter().take(3).map(|s| s.as_str()).collect();
    assert_eq!(after, ["Marks", "Area", "NodeData"], "left to right: {seen:?}");
    let _ = keys;
}

/// Letters typed while the bookmarks hold the keys go to the bookmark
/// whose line's text begins with them, as the Outliner's rows of an area
/// go by their names; the same letter again goes on to the next.
#[test]
fn letters_go_to_a_bookmark_by_its_line_s_text() {
    let mut g = new_dialog();
    for words in ["Hello.", "Go away.", "Goodbye.", "Gold, please."] {
        add_node(&mut g, Parent::Root, words);
    }
    let (mut h, keys) = window_with(
        Mode::Tabs,
        "ui-dialog-marks-letters",
        &["talk"],
        &g,
        egui::vec2(1600.0, 900.0),
        Vec::new(),
    );
    let key = keys[0];
    h.state_mut().dialog_views.get_mut(&key).unwrap().bookmarks =
        (0..4).map(|i| (Kind::Entry, i)).collect();
    settle(&mut h);
    outliner_row(&h, "Entry 0: Hello.").unwrap().click();
    settle(&mut h);
    let go = |h: &mut Harness<'_, Moonglow>, text: &str| {
        h.event(egui::Event::Text(text.into()));
        settle(h);
        h.key_press(egui::Key::Enter);
        settle(h);
        chosen(h, key).map(|r| r.pos)
    };
    assert_eq!(go(&mut h, "g"), Some(1), "g: Go away");
    // (A pause, then the same letter: on to the next.)
    h.run_steps(8);
    assert_eq!(go(&mut h, "g"), Some(2), "g again: Goodbye");
    h.run_steps(8);
    assert_eq!(go(&mut h, "gol"), Some(3), "gol: Gold, please.");
}

/// A bookmark set on a link's row is the line it links to (the review
/// could not reach it): listed by that line's kind, number and text, a
/// click going to the line where it is owned, not to the link.
#[test]
fn a_bookmark_on_a_link_s_row_is_its_line_s() {
    let (mut h, key) = branching_window("ui-dialog-marks-link");
    // (The link: Go away's "Who are you?", the second of the name.)
    let link = h
        .get_all_by_label("Who are you?")
        .map(|n| n.rect())
        .max_by(|a, b| a.top().total_cmp(&b.top()));
    let link = link.expect("the link's row").center();
    h.hover_at(link);
    h.step();
    press(&h, link, true, egui::Modifiers::NONE);
    h.step();
    press(&h, link, false, egui::Modifiers::NONE);
    settle(&mut h);
    assert_eq!(
        chosen(&h, key),
        Some(Row { parent: Parent::Node(Kind::Entry, 2), pos: 0 }),
        "the link"
    );
    bookmark(&mut h, Mode::Tabs);
    assert_eq!(listed(&h), ["Reply 0: Who are you?"]);
    h.get_by_label("Root").click();
    settle(&mut h);
    outliner_row(&h, "Reply 0: Who are you?").unwrap().click();
    settle(&mut h);
    assert_eq!(
        chosen(&h, key),
        Some(Row { parent: Parent::Node(Kind::Entry, 0), pos: 0 }),
        "where owned"
    );
}

/// A tree forty lines deep (the review could not reach it): its deepest
/// row, gone to from its bookmark, has a guide for each of its forty
/// levels, each where its level's arrow is from the tree's left edge (at
/// 1920 the tree has room for them all beside the line's least room, a
/// third of its width; the tree no longer scrolls sideways since M5's W4).
#[test]
fn a_deep_tree_draws_every_guide() {
    let mut g = new_dialog();
    let mut parent = Parent::Root;
    for i in 0..40 {
        let index = add_node(&mut g, parent, &format!("Level {i:02}."));
        parent = Parent::Node(parent.child_kind(), index);
    }
    let (mut h, keys) = window_with(
        Mode::Tabs,
        "ui-dialog-deep",
        &["talk"],
        &g,
        egui::vec2(1920.0, 1080.0),
        Vec::new(),
    );
    let key = keys[0];
    // (The deepest: a reply, the twentieth of its kind.)
    h.state_mut().dialog_views.get_mut(&key).unwrap().bookmarks = vec![(Kind::Reply, 19)];
    settle(&mut h);
    outliner_row(&h, "Reply 19: Level 39.").unwrap().click();
    settle(&mut h);
    let deepest =
        h.get_all_by_label_contains("Level 39.").map(|n| n.rect()).find(|r| r.left() > 300.0);
    let deepest = deepest.expect("the deepest row");
    let root = at(&h, "Root");
    let row = egui::Rect::from_x_y_ranges(root.left()..=deepest.right(), deepest.y_range());
    for level in 0..40 {
        let x = root.left() + level as f32 * 16.0 + 11.0;
        let drawn = guides_drawn(&h)
            .iter()
            .any(|(at, span)| (at - x).abs() <= 1.0 && span.contains(row.center().y));
        // (A guide scrolled out of the tree's sight at its left is not
        // drawn: the guides in sight are each where their level is.)
        let tree = h.state().tab_rect(&Tab::Dialog(key)).map_or(f32::MIN, |r| r.left());
        if x > tree + 8.0 {
            assert!(drawn, "level {level} at {x}");
        }
    }
    let any_off = guides_drawn(&h).iter().any(|(x, span)| {
        span.contains(row.center().y) && ((x - root.left() - 11.0) / 16.0).fract().abs() > 0.1
    });
    assert!(!any_off, "every guide on its level");
}

/// A line dragged over another shows where it would go (a line above it,
/// or a frame round it), drawn clear of the guides: the guides stand left
/// of a row's words, where the drop's marks are not.
#[test]
fn a_line_dragged_over_the_guides_shows_where_it_goes() {
    let (mut h, _) = branching_window("ui-dialog-drag-guides");
    let (a, b) = (at(&h, "Bye. [END DIALOGUE]").center(), at(&h, "Farewell. [END DIALOGUE]"));
    h.hover_at(a);
    h.step();
    press(&h, a, true, egui::Modifiers::NONE);
    h.step();
    for k in 1..=8 {
        h.event(egui::Event::PointerMoved(a + (b.center() - a) * k as f32 / 8.0));
        h.step();
    }
    let accent = mg_uikit::tokens(&h.ctx).accent;
    let marks: Vec<egui::Rect> = (h.output().shapes.iter())
        .filter_map(|c| match &c.shape {
            egui::Shape::LineSegment { points: [p, q], stroke } if stroke.color == accent => {
                Some(egui::Rect::from_two_pos(*p, *q))
            }
            egui::Shape::Rect(r) if r.stroke.color == accent && r.stroke.width >= 2.0 => {
                Some(r.rect)
            }
            _ => None,
        })
        .filter(|r| r.intersects(b.expand(4.0)))
        .collect();
    assert!(!marks.is_empty(), "the drop's mark drawn over Farewell");
    for (x, span) in guides_drawn(&h) {
        if span.contains(b.center().y) {
            assert!(marks.iter().all(|m| x < m.left() - 0.5), "a guide at {x} under {marks:?}");
        }
    }
    press(&h, b.center(), false, egui::Modifiers::NONE);
    settle(&mut h);
}

/// Add Set follows the editor's larger text, as Add Parameter and Script
/// Wizard… beside it do (panel content: U14's ruling).
#[test]
fn add_set_follows_the_editor_s_larger_text() {
    let (mut h, _) = branching_window("ui-dialog-add-set-size");
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(&mut h);
    let high =
        |h: &Harness<'_, Moonglow>| in_section(h, "Text Appears When…", "Add Set").rect().height();
    let usual = high(&h);
    h.state_mut().settings.dialog_font_size = Some(20);
    settle(&mut h);
    let larger = high(&h);
    assert!(larger > usual * 1.3, "Add Set {usual} to {larger}");
}

/// The bookmarks' seam and the areas' are each their own (the
/// coordinator's ruling at U15's review): one dragged beside an area
/// leaves the Outliner beside a conversation the board's 166 high, and
/// one dragged beside a conversation is kept for conversations, the
/// areas' as it was.
#[test]
fn the_bookmarks_seam_is_kept_apart_from_the_areas() {
    let (mut h, _) = branching_window("ui-dialog-seam");
    h.state_mut().settings.module_panel_height = Some(300);
    settle(&mut h);
    let dock = h.state().left_dock_rect().unwrap();
    let row = group(&h).rect();
    let high = dock.bottom() - row.top() + mg_uikit::metrics::DOCK_TABS + 2.0;
    assert!((high - 166.0).abs() <= 5.0, "the board's 166 whatever the areas': {high}");
    // Dragged up 100 points: the bookmarks', the areas' kept.
    let seam = egui::pos2(60.0, row.top() - mg_uikit::metrics::DOCK_TABS - 5.0);
    drag(&mut h, seam, seam - egui::vec2(0.0, 100.0));
    settle(&mut h);
    let kept = h.state().settings.bookmarks_panel_height.expect("kept for the bookmarks");
    assert_eq!(h.state().settings.module_panel_height, Some(300), "the areas' as it was");
    let row = group(&h).rect();
    let high = dock.bottom() - row.top() + mg_uikit::metrics::DOCK_TABS + 2.0;
    assert!((high - 266.0).abs() <= 8.0, "dragged: {high} ({kept})");
}
