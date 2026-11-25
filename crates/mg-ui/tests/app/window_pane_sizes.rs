//! A floating window's pane sizes, kept for every kind of editor (Wave 2,
//! kept by the owner's ruling): a script's are tested in `script_bottom`
//! (`a_script_window_s_pane_sizes_are_kept_for_the_next`); this is
//! another kind's.

use crate::common::*;
use crate::conversation_panels::{dialogs_window, leaf_rect};

/// The tab bar of `key`'s conversation window's right pane (its tabs, the
/// bar's node having no rect of its own): where the pane begins, give or
/// take the bar's padding.
fn right_pane_bar(h: &Harness<'_, Moonglow>, key: ResKey) -> egui::Rect {
    let window = leaf_rect(h, key);
    (h.get_all_by_label("Window's Right Pane"))
        .filter_map(|bar| {
            (bar.get_all_by(|n| n.bounding_box().is_some()))
                .map(|n| n.rect())
                .reduce(|a, b| a.union(b))
        })
        .find(|r| window.contains(r.center()))
        .unwrap_or_else(|| panic!("{key}'s window's right pane in {window:?}"))
}

/// The x of the edge of `key`'s window's right pane: where the pointer
/// shows it can be resized, left of its bar.
fn right_pane_edge(h: &mut Harness<'_, Moonglow>, key: ResKey) -> f32 {
    let (bar, y) = (right_pane_bar(h, key).left(), leaf_rect(h, key).center().y);
    for dx in 0..=24 {
        let x = bar - dx as f32;
        h.hover_at(egui::pos2(x, y));
        h.run_steps(2);
        if h.output().platform_output.cursor_icon == egui::CursorIcon::ResizeHorizontal {
            return x;
        }
    }
    panic!("no resize edge left of {bar}");
}

/// A drag from `from` to `to` a little at a time, held still there a
/// few frames before the release. (The pane's size is kept on the frames
/// it is dragged, and it follows the pointer a frame late: a drag let go
/// as it moves keeps the size of the frame before, the last step short.)
fn held_drag(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(1);
    for step in 1..=8 {
        h.hover_at(from + (to - from) * (step as f32 / 8.0));
        h.run_steps(1);
    }
    h.run_steps(3);
    h.drop_at(to);
    h.run_steps(3);
}

/// A drag from `from` to `to` a little at a time, let go while the pointer
/// still moves (no frames held still before the release).
fn moving_drag(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run_steps(2);
    h.drag_at(from);
    h.run_steps(1);
    for step in 1..=8 {
        h.hover_at(from + (to - from) * (step as f32 / 8.0));
        h.run_steps(1);
    }
    h.drop_at(to);
    h.run_steps(3);
}

/// A drag let go while the pointer still moves keeps the pane's final
/// size, not the one of a frame before.
#[test]
fn a_pane_drag_released_while_moving_keeps_its_final_size() {
    let (mut h, keys) = dialogs_window(Mode::Floating, "ui-dialog-pane-moving", &["aaa"], 1600.0);
    own_window(&mut h, &Tab::Dialog(keys[0]), egui::pos2(60.0, 60.0), egui::vec2(1300.0, 800.0));
    settle(&mut h);
    let edge = right_pane_edge(&mut h, keys[0]);
    let y = leaf_rect(&h, keys[0]).center().y;
    moving_drag(&mut h, egui::pos2(edge, y), egui::pos2(edge - 80.0, y));
    settle(&mut h);
    let now = leaf_rect(&h, keys[0]).right() - right_pane_edge(&mut h, keys[0]);
    let kept = h.state().settings.dock_layouts.get("dialog").cloned().unwrap_or_default();
    let width = kept.window_width.expect("kept for conversations") as f32;
    assert!((width - now).abs() < 1.5, "kept {width}, the pane ends at {now}");
}

/// Two conversations' windows open at once: the size kept is that of the
/// pane dragged, not of the other window's, untouched (whichever of the
/// two is drawn last).
#[test]
fn with_two_windows_open_the_dragged_pane_s_size_is_kept() {
    let (mut h, keys) =
        dialogs_window(Mode::Floating, "ui-dialog-pane-two", &["aaa", "bbb"], 1800.0);
    own_window(&mut h, &Tab::Dialog(keys[0]), egui::pos2(40.0, 40.0), egui::vec2(820.0, 700.0));
    settle(&mut h);
    own_window(&mut h, &Tab::Dialog(keys[1]), egui::pos2(900.0, 60.0), egui::vec2(820.0, 700.0));
    settle(&mut h);
    let side = |h: &mut Harness<'_, Moonglow>, key: ResKey| {
        leaf_rect(h, key).right() - right_pane_edge(h, key)
    };
    let kept = |h: &Harness<'_, Moonglow>| {
        let layout = h.state().settings.dock_layouts.get("dialog").cloned().unwrap_or_default();
        layout.window_width.expect("kept for conversations") as f32
    };
    // Each window's pane dragged in turn, the other left as it is.
    for (key, by) in [(keys[0], 80.0), (keys[1], 40.0)] {
        let edge = right_pane_edge(&mut h, key);
        let y = leaf_rect(&h, key).center().y;
        held_drag(&mut h, egui::pos2(edge, y), egui::pos2(edge - by, y));
        settle(&mut h);
        let (now, width) = (side(&mut h, key), kept(&h));
        assert!((width - now).abs() < 2.0, "{key}: kept {width}, dragged to {now}");
    }
}

/// The right pane of a floating conversation's window, dragged wider, is
/// kept for conversations (the settings' `dialog` layout), and the next
/// conversation opened in a window of its own opens with it.
#[test]
fn a_conversation_window_s_right_pane_size_is_kept_for_the_next() {
    let (mut h, keys) =
        dialogs_window(Mode::Floating, "ui-dialog-pane-sizes", &["aaa", "bbb"], 1600.0);
    // (Measured from its resize edge: the bar's tabs begin a few points
    // in, by as many as its front tab's padding.)
    let side = |h: &mut Harness<'_, Moonglow>, key: ResKey| {
        leaf_rect(h, key).right() - right_pane_edge(h, key)
    };
    own_window(&mut h, &Tab::Dialog(keys[0]), egui::pos2(60.0, 60.0), egui::vec2(1300.0, 800.0));
    settle(&mut h);
    let wide = side(&mut h, keys[0]);
    // The pane's edge 80 to the left.
    let edge = right_pane_edge(&mut h, keys[0]);
    let y = leaf_rect(&h, keys[0]).center().y;
    held_drag(&mut h, egui::pos2(edge, y), egui::pos2(edge - 80.0, y));
    settle(&mut h);
    let wide2 = side(&mut h, keys[0]);
    assert!((wide2 - wide - 80.0).abs() < 4.0, "dragged: {wide} to {wide2}");
    let kept = h.state().settings.dock_layouts.get("dialog").cloned().unwrap_or_default();
    let width = kept.window_width.expect("kept for conversations") as f32;
    assert!((width - wide2).abs() < 2.0, "kept {width}, dragged to {wide2}");
    assert!(!h.state().settings.dock_layouts.contains_key("script"), "not as a script's");
    // The next window: at that size (bbb closed and opened again, its
    // pane's own size from before forgotten).
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Dialog(keys[1])));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Dialog(keys[1])));
    settle(&mut h);
    own_window(&mut h, &Tab::Dialog(keys[1]), egui::pos2(200.0, 80.0), egui::vec2(1300.0, 800.0));
    settle(&mut h);
    let wide3 = side(&mut h, keys[1]);
    assert!(
        (wide3 - wide2).abs() < 2.0,
        "the next window's right pane: {wide3}, not {wide2} (first {wide}, kept {width})"
    );
}
