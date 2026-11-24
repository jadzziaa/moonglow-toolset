//! What the area editor's tools (M4's U4) asked of the kit: a popover at
//! least as wide as asked, with a note at its title's right (the Brush
//! popover's 286 and its mode), and what stands over a 3D view (the
//! compass's column, its view buttons named and drawn in the tokens).

use std::cell::Cell;
use std::rc::Rc;

use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::overlay::{
    COMPASS, CORNER_GAP, CORNER_INSET, VIEW_BUTTON, ViewButton, corner_column, paint_compass,
};
use mg_uikit::popover::Popover;
use mg_uikit::{Icon, Theme, TokensExt as _};

use crate::common::rect_fills;

/// A popover given a least width is that wide (never under 220), and its
/// title's note stands at the title's right end, 10 in, in `text-3` at
/// the small size, where a popover without either keeps its 220.
#[test]
fn a_popover_is_as_wide_as_asked_with_its_title_s_note() {
    for (width, note, expect) in [(286.0, Some("Scatter"), 286.0), (100.0, None, 220.0)] {
        let rect = Rc::new(Cell::new(None));
        let seen = rect.clone();
        let mut h = Harness::builder().with_size(vec2(600.0, 400.0)).build_ui(move |ui| {
            mg_uikit::apply(ui.ctx(), Theme::Dark);
            let mut pop = Popover::new("Brush").title(Some(Icon::Brush), "Brush").min_width(width);
            if let Some(note) = note {
                pop = pop.title_note(note);
            }
            let out = pop.custom(|ui| {
                ui.label("A row of the caller's");
            });
            seen.set(out.show_in_place(ui).rect);
        });
        h.run();
        let r = rect.get().expect("drawn");
        // (Its frame's margin and its line, a point, at each side of the
        // rows' width.)
        let inner = r.width() - 2.0 * (f32::from(mg_uikit::menu::MENU_MARGIN.left) + 1.0);
        assert!((inner - expect).abs() < 1.5, "{width}: {inner} wide, not {expect}");
        let texts: Vec<(String, egui::Rect, egui::Color32)> = (h.output().shapes.iter())
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) => {
                    Some((t.galley.job.text.clone(), t.visual_bounding_rect(), t.fallback_color))
                }
                _ => None,
            })
            .collect();
        let found = texts.iter().find(|(w, ..)| w == "Scatter");
        match note {
            Some(_) => {
                let (_, at, color) = found.expect("the note drawn");
                assert_eq!(*color, mg_uikit::Tokens::DARK.text_3, "in text-3");
                let right = r.right() - mg_uikit::popover::ROW_PADDING;
                assert!((at.right() - right).abs() < 2.0, "at the title's end: {at:?} in {r:?}");
            }
            None => assert!(found.is_none(), "no note"),
        }
    }
}

/// The corner's column: the compass 8 in from the view's top right, its
/// buttons under it 4 apart, centered on it.
#[test]
fn the_corner_column_stands_at_the_view_s_top_right() {
    let view = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), vec2(800.0, 600.0));
    let (compass, buttons) = corner_column(view, 2);
    assert_eq!(compass.size(), egui::Vec2::splat(COMPASS));
    assert_eq!(compass.right_top(), egui::pos2(900.0 - CORNER_INSET, 50.0 + CORNER_INSET));
    assert_eq!(buttons.len(), 2);
    assert_eq!(buttons[0].top(), compass.bottom() + CORNER_GAP);
    assert_eq!(buttons[1].top(), buttons[0].bottom() + CORNER_GAP);
    for b in &buttons {
        assert_eq!(b.size(), egui::Vec2::splat(VIEW_BUTTON));
        assert_eq!(b.center().x, compass.center().x);
    }
}

/// A view button is a button named by its name, 24 square, reported
/// clicked, drawn on `overlay` with a `line-soft` edge (`raised` under
/// the pointer), in both themes; disabled, it is said so and not clicked.
#[test]
fn a_view_button_is_named_and_drawn_in_the_tokens() {
    for theme in [Theme::Dark, Theme::Light] {
        let clicked = Rc::new(Cell::new(0));
        let count = clicked.clone();
        let mut h = Harness::builder().with_size(vec2(200.0, 200.0)).build_ui(move |ui| {
            mg_uikit::apply(ui.ctx(), theme);
            let r = ui.add(ViewButton::new(Icon::Camera, "Reorient Camera").tip("Reorient"));
            if r.clicked() {
                count.set(count.get() + 1);
            }
            ui.add(ViewButton::new(Icon::Start, "Go to Start Location").enabled(false));
            let t = ui.tokens();
            paint_compass(ui.painter(), &t, egui::pos2(100.0, 140.0), 0.5);
        });
        h.run();
        let t = theme.tokens();
        let r = h.get_by_label("Reorient Camera").rect();
        assert_eq!(r.size(), egui::Vec2::splat(VIEW_BUTTON), "{theme:?}");
        let fills = rect_fills(&h, r);
        assert!(fills.contains(&t.overlay), "{theme:?}: on overlay: {fills:?}");
        h.get_by_label("Reorient Camera").hover();
        h.run();
        let fills = rect_fills(&h, r);
        assert!(fills.contains(&t.raised), "{theme:?}: raised under the pointer: {fills:?}");
        h.get_by_label("Reorient Camera").click();
        h.run();
        assert_eq!(clicked.get(), 1, "{theme:?}");
        let off = h.get_by_label("Go to Start Location");
        assert!(off.accesskit_node().is_disabled(), "{theme:?}");
    }
}
