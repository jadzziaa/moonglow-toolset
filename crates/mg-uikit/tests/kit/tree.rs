//! A tree's rows (`mg_uikit::tree`): a group's row opens and closes its
//! rows, is named with its count and said expanded or not; its parts
//! stand where the mock's `.tree-row` has them (the name 42 px in, the
//! count against the right edge, 22 px rows); a row that opens nothing is
//! a selectable label, or a quiet label.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::tree::{self, TreeGroup, TreeRow};
use mg_uikit::{Icon, Theme};

/// The words drawn in a frame, where.
fn texts(h: &Harness<'_>) -> Vec<(String, egui::Rect)> {
    fn add(shape: &egui::Shape, to: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(t) => {
                to.push((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2())))
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| add(s, to)),
            _ => {}
        }
    }
    let mut all = Vec::new();
    h.output().shapes.iter().for_each(|s| add(&s.shape, &mut all));
    all
}

/// A group's row: named "Areas (113)" as a collapsing header, said open,
/// its rows shown; a click closes it (said closed, its rows gone) and
/// opens it again; `open` opens or closes it from outside. Its name is
/// 42 px into the row, its count's right edge 6 px from the row's; the
/// row is 22 px high, its rows under it indented 14.
#[test]
fn a_tree_group_opens_and_closes_its_rows() {
    for theme in Theme::ALL {
        let open = Cell::new(None::<bool>);
        let row = Cell::new(egui::Rect::NOTHING);
        let child = Cell::new(egui::Rect::NOTHING);
        let mut h = Harness::builder().with_size(egui::vec2(260.0, 200.0)).build_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            mg_uikit::list_rows(ui);
            let shown = TreeGroup::new("areas", "Areas")
                .icon(Icon::Area)
                .count(113)
                .default_open(true)
                .open(open.take())
                .show(ui, |ui| child.set(ui.label("map_m1q1a").rect));
            row.set(shown.header_response.rect);
        });
        h.run();
        let group = h.get_by_label("Areas (113)");
        assert_eq!(group.accesskit_node().data().is_expanded(), Some(true));
        assert!(h.query_by_label("map_m1q1a").is_some());
        let r = row.get();
        assert_eq!(r.height(), 22.0, "{theme:?}");
        assert_eq!(r.width(), 260.0 - 16.0, "as wide as what holds it");
        assert_eq!(child.get().left() - r.left(), 14.0, "its rows indented");
        let drawn = texts(&h);
        let at = |w: &str| drawn.iter().find(|(t, _)| t == w).map(|(_, r)| *r).unwrap();
        assert_eq!(at("Areas").left() - r.left(), tree::name_offset(true));
        assert_eq!(tree::name_offset(true), 42.0);
        assert_eq!(tree::name_offset(false), 22.0);
        assert!((r.right() - at("113").right() - tree::ROW_PADDING_RIGHT).abs() < 1.0);
        // A click closes it, and another opens it.
        h.get_by_label("Areas (113)").click();
        h.run();
        let group = h.get_by_label("Areas (113)");
        assert_eq!(group.accesskit_node().data().is_expanded(), Some(false));
        assert!(h.query_by_label("map_m1q1a").is_none());
        h.get_by_label("Areas (113)").click();
        h.run();
        assert!(h.query_by_label("map_m1q1a").is_some());
        // Closed and opened from outside (Collapse All, a filter typed).
        open.set(Some(false));
        h.run();
        assert!(h.query_by_label("map_m1q1a").is_none());
        open.set(Some(true));
        h.run();
        assert!(h.query_by_label("map_m1q1a").is_some());
    }
}

/// A group's count stays at the right edge of what is in sight when what
/// holds it is scrolled sideways (a long name further down), and a name
/// longer than the row pushes its count after it rather than under it.
#[test]
fn a_tree_group_s_count_stays_in_sight() {
    for offset in [0.0, 60.0] {
        let mut h = Harness::builder().with_size(egui::vec2(200.0, 120.0)).build_ui(|ui| {
            mg_uikit::apply(ui.ctx(), Theme::Dark);
            let area = egui::ScrollArea::horizontal().horizontal_scroll_offset(offset);
            area.show(ui, |ui| {
                mg_uikit::list_rows(ui);
                let short = TreeGroup::new("short", "Scripts").icon(Icon::Script).count(1952);
                short.show(ui, |_| {});
                let long = "A group with a name far longer than the panel is wide";
                TreeGroup::new("long", long).icon(Icon::Script).count(7).show(ui, |_| {});
            });
        });
        h.run();
        let drawn = texts(&h);
        let at = |w: &str| drawn.iter().find(|(t, _)| t.starts_with(w)).map(|(_, r)| *r).unwrap();
        // (The harness's 8 px frame: the scroll area's edge at 192, the
        // count 6 px inside it, scrolled sideways or not.)
        let count = at("1952");
        assert!((count.right() - (192.0 - tree::ROW_PADDING_RIGHT)).abs() < 1.0, "{count:?}");
        let (name, count) = (at("A group"), at("7"));
        assert!(count.left() >= name.right() + tree::COUNT_GAP - 0.5, "{name:?} {count:?}");
    }
}

/// A row that opens nothing: a selectable label said selected (toggled) or not,
/// clicked as one, its name 42 px in after its icon; a quiet one a label.
#[test]
fn a_tree_row_is_selectable_or_quiet() {
    let clicked = Cell::new(0);
    let mut h = Harness::builder().with_size(egui::vec2(260.0, 120.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Light);
        mg_uikit::list_rows(ui);
        let r = TreeRow::new("Module Properties").icon(Icon::Module).selected(true).show(ui);
        if r.clicked() {
            clicked.set(clicked.get() + 1);
        }
        TreeRow::new("Encounters").icon(Icon::Encounter).count(0).dim().show(ui);
    });
    h.run();
    let row = h.get_by_label("Module Properties");
    assert_eq!(row.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True), "selected");
    assert_ne!(row.accesskit_node().role(), Role::Label);
    row.click();
    h.run();
    assert_eq!(clicked.get(), 1);
    let quiet = h.get_by_label("Encounters (0)");
    assert_eq!(quiet.accesskit_node().role(), Role::Label);
    let drawn = texts(&h);
    let name = drawn.iter().find(|(t, _)| t == "Module Properties").unwrap().1;
    assert_eq!(name.left() - 8.0, tree::name_offset(true), "the harness's margin, then 42");
}

/// The rects filled in a frame.
fn fills(h: &Harness<'_>) -> Vec<(egui::Rect, egui::Color32)> {
    fn add(shape: &egui::Shape, to: &mut Vec<(egui::Rect, egui::Color32)>) {
        match shape {
            egui::Shape::Rect(r) => to.push((r.rect, r.fill)),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| add(s, to)),
            _ => {}
        }
    }
    let mut all = Vec::new();
    h.output().shapes.iter().for_each(|s| add(&s.shape, &mut all));
    all
}

/// A row as a module tree's resource rows are drawn: indented under its
/// group in a scroll area, selected, its fill from the scroll area's left
/// edge to its right (the board fills the indent too), its name 22 px
/// into the row; a row that drags (`sense`) starts a drag; and a row whose
/// arrow's column is the caller's ([`TreeRow::arrow_apart`]): the button
/// put there before the row takes its clicks and the keyboard's focus
/// before it, its fill under the button (`fill_under`), and the row
/// answers from its name on; marks put after its name
/// (`TreeRowShown::after`) stand after the name.
#[test]
fn a_tree_row_fills_the_row_drags_and_leaves_its_arrow_to_a_button() {
    let arrow_clicks = Cell::new(0);
    let row_clicks = Cell::new(0);
    let parts = Cell::new(None::<(egui::Rect, egui::Rect, egui::Pos2)>);
    let mut h = Harness::builder().with_size(egui::vec2(260.0, 200.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            mg_uikit::list_rows(ui);
            ui.indent("rows", |ui| {
                // (The row's fill kept a place under the button.)
                let fill = ui.painter().add(egui::Shape::Noop);
                let at = tree::arrow_column(ui);
                let button = egui::Rect::from_center_size(at.center(), egui::vec2(22.0, 22.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(button));
                if child.add(egui::Button::new(">").frame(false)).clicked() {
                    arrow_clicks.set(arrow_clicks.get() + 1);
                }
                let shown = TreeRow::new("map_m1q1a").selected(true).arrow_apart().fill_under(fill);
                let shown = shown.show_parts(ui);
                if shown.response.clicked() {
                    row_clicks.set(row_clicks.get() + 1);
                }
                parts.set(Some((at, shown.response.rect, shown.after)));
                TreeRow::new("nw_chicken.utc").sense(egui::Sense::click_and_drag()).show(ui);
            });
        });
    });
    h.run();
    let (arrow, row, after) = parts.get().unwrap();
    // (The harness's 8 px margin, the indent 14: the row's own left at 22.)
    assert_eq!(arrow.left(), 22.0 + tree::ROW_PADDING_LEFT);
    assert_eq!(arrow.width(), tree::ARROW);
    assert_eq!(row.left(), 22.0 + tree::name_offset(false), "the row from its name on");
    let drawn = texts(&h);
    let name = drawn.iter().find(|(t, _)| t == "map_m1q1a").unwrap().1;
    assert_eq!(name.left(), 22.0 + tree::name_offset(false), "the name 22 px into the row");
    assert!((after.x - (name.right() + tree::ROW_GAP)).abs() < 0.5, "{after:?} {name:?}");
    // The selected row's fill: from the scroll area's left edge (8) to
    // the right edge of what is in sight.
    let accent = mg_uikit::tokens(&h.ctx).accent_fill;
    let fill = fills(&h).into_iter().find(|(_, c)| *c == accent).expect("the row's fill").0;
    assert_eq!(fill.left(), 8.0, "{fill:?}");
    assert_eq!(fill.right(), row.right());
    assert_eq!(fill.y_range(), row.y_range());
    // Under the button, drawn before it ([`TreeRow::fill_under`]).
    let order: Vec<String> = (h.output().shapes.iter())
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.fill == accent => Some("fill".to_string()),
            egui::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        })
        .collect();
    let at = |w: &str| order.iter().position(|o| o == w).unwrap();
    assert!(at("fill") < at(">"), "{order:?}");
    // A click on the arrow is the button's, one on the row the row's.
    h.get_by_label(">").click();
    h.run();
    assert_eq!((arrow_clicks.get(), row_clicks.get()), (1, 0));
    h.get_by_label("map_m1q1a").click();
    h.run();
    assert_eq!((arrow_clicks.get(), row_clicks.get()), (1, 1));
    // The keyboard's focus: the button, then the row.
    h.get_by_label(">").focus();
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("map_m1q1a").is_focused());
    // The row that drags.
    let from = h.get_by_label("nw_chicken.utc").rect().center();
    h.drag_at(from);
    h.run();
    for k in 1..=4 {
        h.hover_at(from + egui::vec2(10.0, 5.0) * k as f32);
        h.run();
    }
    assert!(h.ctx.dragged_id().is_some(), "a drag from the row");
    h.drop_at(from + egui::vec2(40.0, 20.0));
    h.run();
}
