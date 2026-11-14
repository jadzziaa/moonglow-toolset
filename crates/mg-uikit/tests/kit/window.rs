//! A dialog's title row: across its window, the title at its left and
//! the window named by it once, a Close at its right found by its name
//! and clicked, with the theme applied or not.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::{Theme, metrics, window};

/// A dialog of 440 points titled "Rename m1q1faribeth.dlg" (themed where
/// `themed`), with a Close where `close`: its row's and body's rectangles,
/// and the Close's clicks counted.
fn dialog<'a>(
    themed: bool,
    close: bool,
    row: &'a Cell<Option<egui::Rect>>,
    body: &'a Cell<Option<egui::Rect>>,
    closed: &'a Cell<u32>,
) -> Harness<'a> {
    Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui(move |ui| {
        let ctx = &ui.ctx().clone();
        if themed {
            mg_uikit::apply(ctx, Theme::Dark);
        }
        egui::Window::new("Rename m1q1faribeth.dlg")
            .title_bar(false)
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(0))
            .fixed_size(egui::vec2(440.0, 140.0))
            .show(ctx, |ui| {
                let (kept, _) = ui.allocate_exact_size(
                    egui::vec2(10.0, metrics::TITLE_ROW),
                    egui::Sense::hover(),
                );
                ui.label("New name");
                let rect = egui::Rect::from_x_y_ranges(ui.min_rect().x_range(), kept.y_range());
                let tip = close.then_some("Close the dialog (Esc)");
                let shown = mg_uikit::title_row(ui, rect, "Rename m1q1faribeth.dlg", tip);
                if shown.closed() {
                    closed.set(closed.get() + 1);
                }
                row.set(Some(shown.rect));
                body.set(Some(ui.min_rect()));
            });
    })
}

#[test]
fn a_title_row_spans_its_window_its_close_found_by_name_themed_or_not() {
    for themed in [false, true] {
        let (row, body, closed) = (Cell::new(None), Cell::new(None), Cell::new(0));
        let mut h = dialog(themed, true, &row, &body, &closed);
        h.run();
        let (row, body) = (row.get().unwrap(), body.get().unwrap());
        assert_eq!(row.height(), metrics::TITLE_ROW, "themed {themed}");
        assert_eq!(row.x_range(), body.x_range(), "the window's width");
        // The window is named by its title, once: the row paints it.
        let named = h.query_all_by_label("Rename m1q1faribeth.dlg").count();
        assert_eq!(named, 1, "the window alone named so");
        let win = h.get_by_label("Rename m1q1faribeth.dlg");
        assert_eq!(win.accesskit_node().role(), Role::Window);
        let close = h.get_by_label(window::CLOSE_NAME);
        assert_eq!(close.accesskit_node().role(), Role::Button);
        let at = close.rect();
        assert!(row.contains_rect(at), "{at:?} in {row:?}");
        assert!((row.right() - at.right() - window::TITLE_END).abs() < 0.5, "at its right");
        close.click();
        h.run();
        assert_eq!(closed.get(), 1, "clicked");
    }
}

#[test]
fn a_title_row_without_close_has_none() {
    let (row, body, closed) = (Cell::new(None), Cell::new(None), Cell::new(0));
    let mut h = dialog(true, false, &row, &body, &closed);
    h.run();
    assert!(h.query_by_label(window::CLOSE_NAME).is_none());
    assert!(row.get().is_some());
}

#[test]
fn a_long_title_leaves_room_for_the_close() {
    let mut h = Harness::new_ui(|ui| {
        let with = mg_uikit::title_row_width(ui, "Rename m1q1faribeth.dlg", true);
        let without = mg_uikit::title_row_width(ui, "Rename m1q1faribeth.dlg", false);
        // (The Close and the room round it, where the row ends in its
        // padding without.)
        let close = 4.0 + metrics::ICON_BUTTON + window::TITLE_END - window::TITLE_PADDING;
        assert!((with - without - close).abs() < 0.01, "{with} {without}");
    });
    h.run();
}

/// In a window's sizing pass (its first frame, laid out unseen, before
/// egui has put it where it shows) the row's buttons take no click: there
/// are none; from the next frame its Close and its fold arrow are there.
#[test]
fn a_title_row_has_no_buttons_in_its_window_s_sizing_pass() {
    let seen: Cell<Vec<(bool, bool, bool)>> = Cell::new(Vec::new());
    let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui(|ui| {
        let ctx = ui.ctx().clone();
        egui::Window::new("Find and Replace Text")
            .title_bar(false)
            .frame(egui::Frame::window(&ctx.global_style()).inner_margin(0))
            .show(&ctx, |ui| {
                let (kept, _) = ui.allocate_exact_size(
                    egui::vec2(200.0, metrics::TITLE_ROW),
                    egui::Sense::hover(),
                );
                ui.label("Find what");
                let rect = egui::Rect::from_x_y_ranges(ui.min_rect().x_range(), kept.y_range());
                let tip = Some("Close the dialog (Esc)");
                let row =
                    mg_uikit::title_row_with(ui, rect, "Find and Replace Text", tip, Some(false));
                let mut all = seen.take();
                all.push((ui.is_sizing_pass(), row.close.is_some(), row.fold.is_some()));
                seen.set(all);
            });
    });
    h.run_steps(3);
    let frames = seen.take();
    assert_eq!(frames.first(), Some(&(true, false, false)), "{frames:?}");
    assert_eq!(frames.last(), Some(&(false, true, true)), "{frames:?}");
    assert!(h.query_by_label(window::FOLD_NAME).is_some() && h.query_by_label("Close").is_some());
}
