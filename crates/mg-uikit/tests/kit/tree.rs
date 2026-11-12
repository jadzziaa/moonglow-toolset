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
