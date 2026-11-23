//! A conversation's lines cut at the tree's right edge and its Bookmark's
//! key (M5's W4, the owner's rulings 29 and 28): a long line's words cut
//! with an ellipsis, its marks and `[END DIALOGUE]` kept after them, its
//! whole text its row's name and tip; a deep line keeping its least room;
//! the cut laid out only for rows in sight and only when something
//! changed; Toggle Bookmark on F5, in the row's right-click menu, the
//! Command Palette and the Status Bar's hints.

use crate::common::*;
use crate::conversation_outliner::{branching_window, chosen, guides_drawn, window_with};
use crate::conversation_panels::{at, tree_key};
use mg_module::dialog::{Kind, Parent, add_node, new_dialog};
use mg_ui::dialog_view::Row;

/// The width a conversation's rows have at 1000 by 700 (a third of it,
/// less the kit's 6-point row padding, is (659 - 6) / 3 = 217.67, more
/// than twelve ems of 13 points, 156)...
const WIDE: f32 = 659.0;
/// ...and in the least window (a third of 334 - 6 is 109.33, less than
/// 156).
const NARROW: f32 = 334.0;
/// The conversation editor's text sizes (Options › Conversation Editor,
/// up to 48) at which the least window's tree has no room for an ellipsis
/// of a reply's words beside its end words, and none for the end words
/// themselves.
const ALONE_SIZE: u8 = 34;
const CUT_SIZE: u8 = 48;

/// A line long enough to run past any tree.
fn long(words: &str) -> String {
    format!("{words} {}", "and so the tale goes on, ".repeat(30)).trim().to_string()
}

/// A long greeting with a condition and a sound, and a long reply under it
/// that ends the conversation.
fn long_talk() -> mg_gff::Gff {
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, &long("Hello there."));
    add_node(&mut g, Parent::Node(Kind::Entry, hello), &long("Who are you?"));
    let rr = |s: &str| mg_gff::Value::resref(mg_core::ResRef::from_str(s).unwrap());
    g.root.list_mut("StartingList").unwrap()[0].set("Active", rr("c_long"));
    g.root.list_mut("EntryList").unwrap()[0].set("Sound", rr("vs_long"));
    g
}

/// The words painted in the last frame: each text's words, where they
/// stand, and whether they were cut.
fn painted(h: &Harness<'_, Moonglow>) -> Vec<(String, egui::Rect, bool)> {
    (h.output().shapes.iter())
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => {
                let rect = egui::Rect::from_min_size(t.pos, t.galley.size());
                Some((t.galley.text().to_string(), rect, t.galley.elided))
            }
            _ => None,
        })
        .collect()
}

/// The words painted that begin with `start`.
fn painted_from(h: &Harness<'_, Moonglow>, start: &str) -> (String, egui::Rect, bool) {
    let found = painted(h).into_iter().find(|(t, _, _)| t.starts_with(start));
    found.unwrap_or_else(|| panic!("nothing painted begins with {start:?}"))
}

/// The tree's room: its editor group's (the header over it included).
fn tree_rect(h: &Harness<'_, Moonglow>, key: ResKey) -> egui::Rect {
    let tab = Tab::Dialog(key);
    let leaf = h.state().dock.iter_leaves().find(|(_, l)| l.tabs.contains(&tab));
    leaf.expect("the conversation's group").1.rect
}

/// A long line is cut at the tree's right edge with an ellipsis, at 1600
/// and in the least window: its words, its marks after them and the
/// reply's ` [END DIALOGUE]` all within the tree, nothing scrolled
/// sideways (Root at the tree's left); its row is named by its whole text,
/// and its tip says it, then its condition.
#[test]
fn a_long_line_is_cut_at_the_tree_s_right_edge() {
    for (name, size) in [("1600", egui::vec2(1600.0, 900.0)), ("least", mg_ui::LEAST_WINDOW_SIZE)] {
        let (mut h, keys) = window_with(
            Mode::Tabs,
            &format!("ui-dialog-long-{name}"),
            &["talk"],
            &long_talk(),
            size,
            Vec::new(),
        );
        let key = keys[0];
        settle(&mut h);
        let tree = tree_rect(&h, key);
        // (Clear of the tree's right edge, where the selected row's fill
        // ends, by a row's padding at least: Hello chosen.)
        let view = h.state_mut().dialog_views.get_mut(&key).unwrap();
        view.selected = Some(Row { parent: Parent::Root, pos: 0 });
        settle(&mut h);
        let fill = mg_uikit::tokens(&h.ctx).accent_fill;
        let chosen = h.get_by_label(&format!("[OWNER] - {}", long("Hello there."))).rect().center();
        let filled = (h.output().shapes.iter()).find_map(|c| match &c.shape {
            egui::Shape::Rect(r) if r.fill == fill && r.rect.contains(chosen) => Some(r.rect),
            _ => None,
        });
        let edge = filled.expect("the chosen row's fill").right()
            - mg_uikit::tree::ROW_PADDING_RIGHT
            + 0.5;
        let hello = format!("[OWNER] - {}", long("Hello there."));
        let (_, words, cut) = painted_from(&h, "[OWNER] - Hello there.");
        assert!(cut, "{name}: the greeting cut");
        assert!(words.right() <= edge, "{name}: {words:?} to {edge}");
        let who = long("Who are you?");
        let (_, reply, cut) = painted_from(&h, "Who are you?");
        assert!(cut, "{name}: the reply cut");
        let (_, end, _) = painted_from(&h, " [END DIALOGUE]");
        assert!(end.left() >= reply.right() - 0.5 && end.right() <= edge, "{name}");
        // (The marks shown at 1600, where the words keep room for them.)
        if name == "1600" {
            let (_, marks, _) = painted_from(&h, "if c_long");
            assert!(marks.left() >= words.right(), "the marks after the words");
            assert!(marks.right() <= edge, "{marks:?} in {tree:?}");
        }
        // (Not scrolled sideways: Root where the tree begins.)
        assert!(at(&h, "Root").left() >= tree.left(), "{name}: Root in sight");
        let row = h.get_by_label(&hello).rect();
        assert!(row.right() <= tree.right() + 0.5, "{name}: the row within the tree");
        let reply_row = h.get_by_label(&format!("{who} [END DIALOGUE]")).rect();
        assert!(reply_row.right() <= tree.right() + 0.5, "{name}");
        if name == "1600" {
            let tip = tip_at(&mut h, row).unwrap_or_default();
            assert!(tip.contains(&hello), "the tip says the whole line: {tip:?}");
            assert!(tip.contains("Appears when c_long returns TRUE"), "{tip:?}");
        }
    }
}

/// In a floating window the selected row's fill (Root's, as a conversation
/// opens) ends at the tree's edge, short of the window's right pane (Node,
/// Data, Search): the tree's vertical scroll area clips up and down only,
/// its clip rect as wide as the window, and the fill ran on under the pane
/// (M5 P1, the README's picture). Docked, the pane is the Right Dock's.
#[test]
fn the_selected_row_s_fill_ends_at_the_tree_in_a_window() {
    let size = egui::vec2(1600.0, 900.0);
    let (mut h, keys) = window_with(
        Mode::Floating,
        "ui-dialog-fill-window",
        &["talk"],
        &long_talk(),
        size,
        Vec::new(),
    );
    let key = keys[0];
    settle(&mut h);
    let fill = mg_uikit::tokens(&h.ctx).accent_fill;
    let root = at(&h, "Root").center();
    let filled = (h.output().shapes.iter()).find_map(|c| match &c.shape {
        egui::Shape::Rect(r) if r.fill == fill && r.rect.contains(root) => Some(r.rect),
        _ => None,
    });
    let filled = filled.expect("Root's fill");
    let tree = h.state().dialog_views[&key].rows_right();
    assert!(filled.right() <= tree + 0.5, "{filled:?} past the tree's edge {tree}");
    // (The window's right pane, beside the tree.)
    let node = h.query_all_by_label("Node").map(|n| n.rect()).find(|r| r.left() > root.x);
    let node = node.expect("the window's Node section");
    assert!(filled.right() < node.left(), "{filled:?} under the Node section at {node:?}");
}

/// A conversation forty levels deep in a window `size`, its deepest line
/// chosen and in sight: the width its rows have (from the chosen row's
/// fill at their left to where they end, the scroll bar's side), the room
/// from the deepest line's words to there, and the levels it stands in by.
fn deepest(name: &str, size: egui::Vec2) -> (f32, f32, usize) {
    let mut g = new_dialog();
    let mut parent = Parent::Root;
    for i in 0..40 {
        let index = add_node(&mut g, parent, &format!("Level {i:02}."));
        parent = Parent::Node(parent.child_kind(), index);
    }
    let (mut h, keys) = window_with(Mode::Tabs, name, &["talk"], &g, size, vec![]);
    let key = keys[0];
    settle(&mut h);
    let root = at(&h, "Root");
    let view = h.state_mut().dialog_views.get_mut(&key).unwrap();
    view.selected = Some(Row { parent: Parent::Node(Kind::Entry, 19), pos: 0 });
    view.reveal = true;
    settle(&mut h);
    let tree = tree_rect(&h, key);
    let found =
        painted(&h).into_iter().find(|(t, r, _)| t == "Level 39." && tree.contains_rect(*r));
    let words = found.expect("the deepest line in the tree").1;
    let fill = mg_uikit::tokens(&h.ctx).accent_fill;
    let filled = (h.output().shapes.iter()).find_map(|c| match &c.shape {
        egui::Shape::Rect(r) if r.fill == fill && r.rect.contains(words.center()) => Some(r.rect),
        _ => None,
    });
    let row = filled.expect("the chosen row's fill");
    let levels = ((words.left() - root.left()) / 16.0).floor() as usize;
    // (Its guides with it: one for each level it stands in by, none past.)
    let y = words.center().y;
    let guides = guides_drawn(&h).iter().filter(|(_, span)| span.contains(y)).count();
    assert!(guides > 1 && guides <= levels + 1, "{name}: {guides} guides, {levels} levels");
    let right = h.state().dialog_views[&key].rows_right();
    (right - row.left(), right - words.left(), levels)
}

/// A line forty levels deep stands in no further than leaves its words
/// their least room: twelve ems (156 points of the 13-point text) or a
/// third of the tree's width, whichever is more. Worked out by hand for
/// two trees, one where each rule decides (the room measured from the
/// words' left to the tree's edge, within a level's indent and a row's
/// padding of the least), and the rule itself.
#[test]
fn a_deep_line_keeps_its_least_room() {
    use mg_ui::dialog_view::least_line;
    assert_eq!(least_line(900.0, 13.0), 300.0, "a third of 900");
    assert_eq!(least_line(300.0, 13.0), 156.0, "twelve ems of 13");
    // (WIDE, then NARROW: the tree's width as laid out, and the least
    // room by hand.)
    for (name, size, tree, least) in [
        ("ui-dialog-deep-third", egui::vec2(1000.0, 700.0), WIDE, 217.67),
        ("ui-dialog-deep-ems", mg_ui::LEAST_WINDOW_SIZE, NARROW, 156.0),
    ] {
        let (width, room, levels) = deepest(name, size);
        assert!((width - tree).abs() < 1.0, "{name}: the tree {width} wide");
        assert!(levels < 39, "{name}: {levels} levels");
        // (The words' own room: the kit's row padding at the right, the
        // row's own at both sides, the measure taken at the words' left.)
        let pad = 8.0;
        let words = room - mg_uikit::tree::ROW_PADDING_RIGHT - pad;
        assert!(words >= least - 0.5 && words < least + 32.0, "{name}: {words} for {least}");
    }
}

/// Beside the scroll bar, shown (a conversation longer than the tree), a
/// cut line's words end the kit's row padding (and the row's own) short
/// of where the rows end: cut exactly as egui cuts them in that room, a
/// line of narrow letters, so that six points less or more cut it
/// elsewhere.
#[test]
fn the_words_end_a_row_s_padding_short_of_the_scroll_bar() {
    let mut g = new_dialog();
    let narrow = format!("Thin {}", "il".repeat(400));
    add_node(&mut g, Parent::Root, &narrow);
    for i in 0..80 {
        add_node(&mut g, Parent::Root, &format!("Line {i:02}."));
    }
    let size = egui::vec2(1600.0, 900.0);
    let (mut h, keys) = window_with(Mode::Tabs, "ui-dialog-pad", &["talk"], &g, size, vec![]);
    let key = keys[0];
    settle(&mut h);
    let view = h.state_mut().dialog_views.get_mut(&key).unwrap();
    view.selected = Some(Row { parent: Parent::Root, pos: 0 });
    settle(&mut h);
    let (_, words, cut) = painted_from(&h, "[OWNER] - Thin");
    assert!(cut, "the thin line cut");
    let right = h.state().dialog_views[&key].rows_right();
    // (The scroll bar shown: the rows end short of the tree by its width.)
    let bar = h.ctx.global_style().spacing.scroll.bar_width;
    assert!(tree_rect(&h, key).right() - right >= bar, "the scroll bar shown");
    // (The row's own padding before and after its words, then the tree's:
    // the words cut as egui cuts them in just that room.)
    let pad = h.ctx.global_style().spacing.button_padding.x;
    let room = right - mg_uikit::tree::ROW_PADDING_RIGHT - (words.left() - pad) - 2.0 * pad;
    let galley = (h.output().shapes.iter()).find_map(|c| match &c.shape {
        egui::Shape::Text(t) if t.galley.text().starts_with("[OWNER] - Thin") => {
            Some(t.galley.clone())
        }
        _ => None,
    });
    let mut job = (*galley.expect("the thin line's words").job).clone();
    job.wrap = egui::text::TextWrapping::truncate_at_width(room.floor());
    let expected = h.ctx.fonts_mut(|f| f.layout_job(job)).size().x;
    assert!((words.width() - expected).abs() < 0.5, "{words:?}: {expected} wide in {room}");
}

/// A line with three marks (its condition, action and sound), deep in a
/// branch: at 1600 its words cut and every mark whole after them; in the
/// least window its words cut first, then the marks dropped from the
/// right, never one cut, and the ones dropped said in the row's name and
/// its tip.
#[test]
fn marks_are_drawn_whole_or_not_at_all() {
    let mut g = new_dialog();
    // (Twenty greetings first, so that the deep line can be out of sight.)
    for i in 0..20 {
        add_node(&mut g, Parent::Root, &format!("Greeting {i}."));
    }
    let mut parent = Parent::Root;
    for i in 0..8 {
        let index = add_node(&mut g, parent, &format!("Step {i}."));
        parent = Parent::Node(parent.child_kind(), index);
    }
    // (Under the eighth, an NPC line: entry 4 under reply 3.)
    let deep = add_node(&mut g, parent, &long("Deep down."));
    let rr = |s: &str| mg_gff::Value::resref(mg_core::ResRef::from_str(s).unwrap());
    let link = (g.root.list_mut("ReplyList").unwrap()[3].list_mut("EntriesList").unwrap())
        .get_mut(0)
        .unwrap();
    link.set("Active", rr("c_deep"));
    let entry = &mut g.root.list_mut("EntryList").unwrap()[deep as usize];
    entry.set("Script", rr("a_deep"));
    entry.set("Sound", rr("vs_deep"));
    let all = ["if c_deep", "do a_deep", "sound vs_deep"];
    let label = format!("[OWNER] - {}", long("Deep down."));
    for (name, size) in [("1600", egui::vec2(1600.0, 900.0)), ("least", mg_ui::LEAST_WINDOW_SIZE)] {
        let (mut h, keys) = window_with(
            Mode::Tabs,
            &format!("ui-dialog-marks-{name}"),
            &["talk"],
            &g,
            size,
            vec![],
        );
        let key = keys[0];
        settle(&mut h);
        let view = h.state_mut().dialog_views.get_mut(&key).unwrap();
        view.selected = Some(Row { parent: Parent::Node(Kind::Reply, 3), pos: 0 });
        view.reveal = true;
        settle(&mut h);
        let shown = painted(&h);
        let (_, words, cut) = painted_from(&h, "[OWNER] - Deep down.");
        assert!(cut, "{name}: the words cut");
        // (Each mark painted whole, after the words, or not at all: those
        // drawn the first ones.)
        let drawn: Vec<&str> =
            all.iter().copied().filter(|m| shown.iter().any(|(t, _, _)| t == m)).collect();
        assert!(all.starts_with(&drawn), "{name}: {drawn:?}");
        for (t, r, elided) in &shown {
            if t.starts_with("if c_") || t.starts_with("do a_") || t.starts_with("sound ") {
                assert!(all.contains(&t.as_str()) && !elided, "{name}: a mark cut: {t:?}");
                assert!(r.left() >= words.right(), "{name}: {t} after the words");
            }
        }
        let dropped = &all[drawn.len()..];
        // (One rule for the names: the row by its whole line, each mark a
        // label by its words, drawn or dropped.)
        let row = h.get_by_label(&label).rect();
        for m in all {
            h.get_by_label(m);
        }
        if name == "1600" {
            assert_eq!(drawn, all, "every mark at 1600");
        } else {
            assert!(!dropped.is_empty(), "least: a mark dropped");
            let tip = tip_at(&mut h, row).unwrap_or_default();
            assert!(tip.contains(&label) && tip.contains(&dropped.join("  ")), "{tip:?}");
            // Out of sight (Root chosen, the tree at its top): named alike.
            let view = h.state_mut().dialog_views.get_mut(&key).unwrap();
            view.selected = None;
            view.nudge = true;
            settle(&mut h);
            assert!(
                !painted(&h).iter().any(|(t, _, _)| t.starts_with("[OWNER] - Deep")),
                "out of sight"
            );
            h.get_by_label(&label);
            for m in all {
                h.get_by_label(m);
            }
        }
    }
}

/// A tree with no room for a row's padding, ` [END DIALOGUE]` and an
/// ellipsis of the words (the least window, the editor's text at 34
/// points): the reply's words go and its end words show alone, within the
/// tree; at 48 points, the end words cut with an ellipsis. The row named
/// by its whole line either way.
#[test]
fn a_very_narrow_tree_shows_the_end_words_alone() {
    let who = long("Who are you?");
    for (name, size, cut) in [("alone", ALONE_SIZE, false), ("cut", CUT_SIZE, true)] {
        let (mut h, keys) = window_with(
            Mode::Tabs,
            &format!("ui-dialog-narrow-{name}"),
            &["talk"],
            &long_talk(),
            mg_ui::LEAST_WINDOW_SIZE,
            Vec::new(),
        );
        let key = keys[0];
        h.state_mut().settings.dialog_font_size = Some(size);
        settle(&mut h);
        let right = h.state().dialog_views[&key].rows_right();
        let tree = tree_rect(&h, key);
        let shown: Vec<_> =
            painted(&h).into_iter().filter(|(_, r, _)| tree.intersects(*r)).collect();
        let end = shown.iter().find(|(t, _, _)| t.starts_with("[END DIALOGUE]"));
        let (_, end, elided) = end.unwrap_or_else(|| panic!("{name}: the end words alone"));
        assert_eq!(*elided, cut, "{name}: the end words cut");
        assert!(
            !shown.iter().any(|(t, r, _)| t.starts_with("Who are you?") && r.width() > 0.0),
            "{name}"
        );
        let pad = h.ctx.global_style().spacing.button_padding.x;
        let edge = right - mg_uikit::tree::ROW_PADDING_RIGHT - pad + 0.5;
        assert!(end.right() <= edge, "{name}: {end:?} past {edge}");
        let row = h.get_by_label(&format!("{who} [END DIALOGUE]")).rect();
        assert!(row.right() <= right + 0.5, "{name}: the row within the tree");
    }
}

/// A line found by the header's highlights (Sounds: the greeting and the
/// reply given a sound) is marked behind its words as drawn: the cut words
/// to their ellipsis and the reply's ` [END DIALOGUE]`, not the marks
/// after them nor the row's padding.
#[test]
fn a_highlight_covers_the_line_as_drawn() {
    let mut g = long_talk();
    let rr = |s: &str| mg_gff::Value::resref(mg_core::ResRef::from_str(s).unwrap());
    g.root.list_mut("ReplyList").unwrap()[0].set("Sound", rr("vs_who"));
    let size = egui::vec2(1600.0, 900.0);
    let (mut h, keys) = window_with(Mode::Tabs, "ui-dialog-highlight", &["talk"], &g, size, vec![]);
    let key = keys[0];
    h.state_mut().dialog_views.get_mut(&key).unwrap().highlight =
        [false, false, false, false, true];
    settle(&mut h);
    let found = mg_uikit::tokens(&h.ctx).select.gamma_multiply(0.3);
    let marked: Vec<egui::Rect> = (h.output().shapes.iter())
        .filter_map(|c| match &c.shape {
            egui::Shape::Rect(r) if r.fill == found => Some(r.rect),
            _ => None,
        })
        .collect();
    let (_, hello, cut) = painted_from(&h, "[OWNER] - Hello there.");
    assert!(cut, "the greeting cut");
    let (_, reply, _) = painted_from(&h, "Who are you?");
    let (_, end, _) = painted_from(&h, " [END DIALOGUE]");
    let (_, sound, _) = painted_from(&h, "sound vs_long");
    let near = |a: f32, b: f32| (a - b).abs() < 0.5;
    let on = |left: f32, right: f32| {
        marked.iter().any(|m| near(m.left(), left) && near(m.right(), right))
    };
    assert!(on(hello.left(), hello.right()), "the greeting's words: {marked:?} {hello:?}");
    assert!(on(reply.left(), end.right()), "the reply's words and end words: {marked:?}");
    assert!(marked.iter().all(|m| m.right() <= sound.left()), "not the marks");
}

/// The cut words are laid out for the rows in sight only, and again only
/// where something changed: three thousand long lines open with far fewer
/// cuts made than lines; frames at rest make none; the tree made narrower
/// cuts again (the rows in sight only); and so does a scroll to the end.
#[test]
fn rows_are_cut_once_and_only_in_sight() {
    let mut g = new_dialog();
    for i in 0..3000 {
        add_node(&mut g, Parent::Root, &long(&format!("Line {i:04}.")));
    }
    let size = egui::vec2(1600.0, 900.0);
    let (mut h, keys) = window_with(Mode::Tabs, "ui-dialog-cuts", &["talk"], &g, size, vec![]);
    let key = keys[0];
    settle(&mut h);
    let made = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].cuts_made();
    let first = made(&h);
    assert!(first > 0 && first < 300, "{first} cuts made for 3000 lines");
    h.run_steps(10);
    assert_eq!(made(&h), first, "none at rest");
    h.set_size(egui::vec2(1400.0, 900.0));
    settle(&mut h);
    let narrower = made(&h) - first;
    assert!(narrower > 0 && narrower < 300, "{narrower} cuts made again");
    let before = made(&h);
    let view = h.state_mut().dialog_views.get_mut(&key).unwrap();
    view.selected = Some(Row { parent: Parent::Root, pos: 2999 });
    view.reveal = true;
    settle(&mut h);
    let scrolled = made(&h) - before;
    assert!(scrolled > 0 && scrolled < 300, "{scrolled} cuts made for the end");
}

/// The line chosen at the tree's start: Hello's, its keys the tree's.
fn hello(h: &mut Harness<'_, Moonglow>) {
    h.get_by_label("[OWNER] - Hello there.").click();
    settle(h);
}

/// F5 puts a bookmark on the line chosen and takes it off (the script's
/// Toggle Bookmark's key, the owner's ruling 28), as rebound; unbound,
/// nothing; typing in Data, nothing; and the Node section's Bookmark's
/// tip names it.
#[test]
fn f5_toggles_the_line_s_bookmark() {
    let (mut h, key) = branching_window("ui-dialog-f5");
    hello(&mut h);
    let marks = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].bookmarks.clone();
    tree_key(&mut h, egui::Modifiers::NONE, egui::Key::F5);
    assert_eq!(marks(&h), [(Kind::Entry, 0)], "F5 put it on");
    tree_key(&mut h, egui::Modifiers::NONE, egui::Key::F5);
    assert!(marks(&h).is_empty(), "F5 took it off");
    // Rebound to Shift+F5: F5 alone does nothing.
    let shift_f5 = egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::F5);
    h.state_mut().keymap.set(mg_ui::keys::Cmd::DialogBookmark, vec![shift_f5]);
    tree_key(&mut h, egui::Modifiers::NONE, egui::Key::F5);
    assert!(marks(&h).is_empty(), "F5 is no longer its key");
    tree_key(&mut h, egui::Modifiers::SHIFT, egui::Key::F5);
    assert_eq!(marks(&h), [(Kind::Entry, 0)], "Shift+F5 put it on");
    h.state_mut().keymap.set(mg_ui::keys::Cmd::DialogBookmark, Vec::new());
    tree_key(&mut h, egui::Modifiers::SHIFT, egui::Key::F5);
    assert_eq!(marks(&h), [(Kind::Entry, 0)], "unbound: nothing");
    h.state_mut().keymap = mg_ui::keys::Keymap::default();
    // The Node section's Bookmark says its key.
    let bar = h.get_by_label(crate::conversation_panels::node_bar(Mode::Tabs));
    let rect = bar.get_by_label("Bookmark").rect();
    let tip = tip_at(&mut h, rect).unwrap_or_default();
    assert_eq!(tip, "Bookmark the line chosen, or take its bookmark off (F5)");
    // Typing in Data: F5 is not the field's, and bookmarks nothing.
    let text = h
        .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Hello there."))
        .expect("the line's text")
        .rect();
    click_pos(&h, text.center());
    settle(&mut h);
    assert!(h.ctx.text_edit_focused(), "typing in Data");
    h.key_press(egui::Key::F5);
    settle(&mut h);
    assert_eq!(marks(&h), [(Kind::Entry, 0)], "typing: nothing");
}

/// A row's right-click menu has Bookmark (Aurora's), with Toggle
/// Bookmark's key beside it; the Command Palette has Toggle Bookmark with
/// its key, grayed with no line chosen; each puts a bookmark on the line
/// or takes it off.
#[test]
fn bookmark_in_the_row_s_menu_and_the_command_palette() {
    let (mut h, key) = branching_window("ui-dialog-bookmark-menu");
    let marks = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].bookmarks.clone();
    let row = at(&h, "Bye. [END DIALOGUE]");
    h.hover_at(row.center());
    h.step();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: row.center(),
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    settle(&mut h);
    // (Its name and its key beside it.)
    h.get_by_label("Bookmark F5").click();
    settle(&mut h);
    let bye = Row { parent: Parent::Node(Kind::Entry, 0), pos: 1 };
    assert_eq!(chosen(&h, key), Some(bye), "the row chosen");
    assert_eq!(marks(&h), [(Kind::Reply, 2)], "the menu's Bookmark put it on");
    // The Command Palette's Toggle Bookmark, with its key.
    command_palette(&mut h, "toggle bookmark");
    let found = quick_open_row(&h, "Toggle Bookmark");
    assert!(!found.accesskit_node().is_disabled());
    assert!(painted(&h).iter().any(|(t, _, _)| t == "F5"), "its key on its row");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(marks(&h).is_empty(), "the palette took it off");
    // Root chosen: grayed, saying why.
    h.state_mut().dialog_views.get_mut(&key).unwrap().selected = None;
    settle(&mut h);
    command_palette(&mut h, "toggle bookmark");
    let rect = quick_open_row(&h, "Toggle Bookmark").rect();
    assert!(quick_open_row(&h, "Toggle Bookmark").accesskit_node().is_disabled());
    assert_eq!(tip_at(&mut h, rect).as_deref(), Some("Choose a line first"));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
}

/// The Status Bar hints F5 Bookmark last exactly where F5 works: with a
/// line chosen, and the tree having the keys or the pointer over the
/// editor (as Find's hint); not with Root chosen, where it is unbound, with
/// the keys elsewhere (typing in Data), or with a menu open.
#[test]
fn the_bookmark_s_hint_is_as_bound() {
    let (mut h, key) = branching_window("ui-dialog-bookmark-hint");
    let marks = |h: &Harness<'_, Moonglow>| h.state().dialog_views[&key].bookmarks.clone();
    hello(&mut h);
    says(&mut h, &["F5: Bookmark"], &[]);
    let said = h.state().status_said();
    let hints: Vec<&String> = said.iter().filter(|s| s.contains(": ")).collect();
    assert_eq!(hints.last().map(|s| s.as_str()), Some("F5: Bookmark"), "last: {hints:?}");
    // The keys handed back (Escape), the pointer over the editor: said, and
    // F5 works.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    h.hover_at(at(&h, "Bye. [END DIALOGUE]").center());
    says(&mut h, &["Ctrl, F: Find", "F5: Bookmark"], &["*: Open branch"]);
    h.key_press(egui::Key::F5);
    settle(&mut h);
    assert_eq!(marks(&h), [(Kind::Entry, 0)], "F5 with the pointer over the editor");
    // Root chosen: no line for it, unsaid, F5 doing nothing.
    h.state_mut().dialog_views.get_mut(&key).unwrap().selected = None;
    says(&mut h, &["Ctrl, F: Find"], &["F5: Bookmark"]);
    h.key_press(egui::Key::F5);
    settle(&mut h);
    assert_eq!(marks(&h), [(Kind::Entry, 0)], "Root chosen: nothing");
    hello(&mut h);
    says(&mut h, &["F5: Bookmark"], &[]);
    h.state_mut().keymap.set(mg_ui::keys::Cmd::DialogBookmark, Vec::new());
    says(&mut h, &["Drag: Move line"], &["F5: Bookmark"]);
    h.state_mut().keymap = mg_ui::keys::Keymap::default();
    says(&mut h, &["F5: Bookmark"], &[]);
    h.get_all_by_label("File").next().unwrap().click();
    settle(&mut h);
    says(&mut h, &[], &["F5: Bookmark", "Drag: Move line"]);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    let text = h
        .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Hello there."))
        .expect("the line's text")
        .rect();
    click_pos(&h, text.center());
    settle(&mut h);
    assert!(h.ctx.text_edit_focused(), "typing in Data");
    says(&mut h, &["Drag: Move line"], &["F5: Bookmark"]);
}
