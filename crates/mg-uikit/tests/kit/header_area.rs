//! What the area editor's header (M4's U3) asked of the kit: what follows
//! a header begins under it, a popover folded early unfolds again where
//! it has room, rows shown only folded or only unfolded (the area's
//! Shadows and Walkmesh), a switch unfolded with its words, a check row
//! and an item named apart from their words, a button's words without
//! its icon, and an open combo box's look.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::header::{self, EditorHeader, Item, Place};
use mg_uikit::popover::{PopCheck, Popover};
use mg_uikit::{Icon, KitButton, TokensExt as _};

use crate::common::harness;

const H: &str = "Area-like header";

fn place(h: &Harness<'_>, name: &str) -> Option<Place> {
    header::places(&h.ctx, H).iter().find(|(n, _)| n == name).map(|(_, p)| *p)
}

/// What is laid out after a header in the same `Ui` begins under the
/// header's row, not over its lower half (the row's items, centered in
/// it, had left the cursor after themselves).
#[test]
fn what_follows_the_header_begins_under_it() {
    let seen = Rc::new(Cell::new((egui::Rect::NOTHING, egui::Rect::NOTHING)));
    let out = seen.clone();
    let mut on = false;
    let mut h = harness(vec2(600.0, 300.0), move |ui| {
        let row = EditorHeader::new(H)
            .left(Item::toggle(Some(Icon::Grid), "Grid", &mut on))
            .show(ui)
            .response
            .rect;
        let next = ui.label("Under the header").rect;
        out.set((row, next));
    });
    h.run();
    let (row, next) = seen.get();
    assert!(next.top() >= row.bottom(), "{next:?} over the header {row:?}");
}

/// Two popovers that unfold, the left one wide and the right one narrow:
/// where the row has no room for the wide one, the narrow one (folded
/// first, from the right group's end) unfolds again; where it has room
/// for both, both unfold; the left group's words go only once both are
/// folded.
#[test]
fn a_popover_folded_early_unfolds_again_where_it_has_room() {
    let width = Rc::new(Cell::new(1200.0));
    let w = width.clone();
    let mut values = [false; 9];
    let mut h = harness(vec2(1200.0, 300.0), move |ui| {
        let [a, b, c, d, e, f, g, x, y] = &mut values;
        let wide = Popover::new("Wide")
            .check(PopCheck::new(a, "Alpha one").icon(Icon::Grid).unfolded_words(true))
            .check(PopCheck::new(b, "Beta two").icon(Icon::Minimap).unfolded_words(true))
            .check(PopCheck::new(c, "Gamma three").icon(Icon::Fog).unfolded_words(true))
            .check(PopCheck::new(d, "Delta four").icon(Icon::Grass).unfolded_words(true))
            .check(PopCheck::new(e, "Epsilon five").icon(Icon::Shaders).unfolded_words(true))
            .check(PopCheck::new(f, "Zeta six").icon(Icon::Music).unfolded_words(true));
        let narrow = Popover::new("Narrow")
            .check(PopCheck::new(x, "Ex").icon(Icon::Speaker))
            .check(PopCheck::new(y, "Why").icon(Icon::Ambient));
        let r = ui.allocate_ui(vec2(w.get(), 40.0), |ui| {
            EditorHeader::new(H)
                .left(Item::button(Icon::Save, "Save words"))
                .left(Item::toggle(Some(Icon::Grid), "Plain", g))
                .right(Item::popover(Some(Icon::Light), wide).unfolds())
                .right(Item::popover(Some(Icon::Speaker), narrow).unfolds())
                .show(ui)
        });
        drop(r);
    });
    let at = |h: &mut Harness<'_>, px: f32| {
        width.set(px);
        h.run();
        (place(h, "Wide"), place(h, "Narrow"), place(h, "Save words"))
    };
    let (u, r) = (Some(Place::Unfolded), Some(Place::Row));
    assert_eq!(at(&mut h, 1190.0), (u, u, r), "room for both");
    // (Room for the narrow one alone: it unfolds again.)
    let mut found = false;
    for px in (300..1190).rev().step_by(10) {
        let (wide, narrow, save) = at(&mut h, px as f32);
        if wide == r && narrow == u {
            assert_eq!(save, r, "no word went at {px}");
            found = true;
        }
        if save != r {
            assert_ne!(narrow, u, "words went while a popover stayed unfolded at {px}");
        }
    }
    assert!(found, "the narrow one never unfolded beside the wide one folded");
}

/// Rows shown only folded and a part shown only unfolded: hung, the rows
/// and not the part; unfolded, the part (in the record by its name) and
/// not the rows; in More, the rows and not the part. A folded-only row of
/// the caller's lets the popover unfold.
#[test]
fn rows_folded_only_and_a_part_unfolded_only() {
    let unfolded = Rc::new(Cell::new(false));
    let into_more = Rc::new(Cell::new(false));
    let (u, m) = (unfolded.clone(), into_more.clone());
    let can = Rc::new(Cell::new(false));
    let c = can.clone();
    let mut kinds = [true, false];
    let mut h = harness(vec2(900.0, 400.0), move |ui| {
        let [sun, creatures] = &mut kinds;
        let pop = Popover::new("Lighting")
            .heading("Shadows")
            .folded_only()
            .check(PopCheck::new(sun, "Sun shadows"))
            .folded_only()
            .check(PopCheck::new(creatures, "Creature shadows"))
            .folded_only()
            .custom(|ui| {
                ui.label("Quality row");
            })
            .folded_only()
            .unfolded_only("Shadows", 90.0, |ui| {
                let _ = ui.add(KitButton::new(Some(Icon::Shadow), "Shadows").toggle(true));
            });
        c.set(pop.can_unfold());
        let mut item = Item::popover(Some(Icon::Light), pop);
        item = if u.get() { item.unfolds() } else { item };
        item = if m.get() { item.in_more() } else { item };
        let _ = EditorHeader::new(H).right(item).show(ui);
    });
    h.run();
    assert!(can.get(), "a folded-only row of the caller's keeps it from unfolding");
    h.get_by_label(H).get_by_label("Lighting").click();
    h.run();
    assert!(h.query_by_label("Sun shadows").is_some(), "hung: its rows");
    assert!(h.query_by_label("Quality row").is_some(), "hung: its own row");
    let part = |h: &Harness<'_>| {
        h.query_all_by_label("Shadows")
            .any(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
    };
    assert!(!part(&h), "hung: not the part");
    h.key_press(egui::Key::Escape);
    h.run();
    unfolded.set(true);
    h.run();
    assert_eq!(place(&h, "Lighting"), Some(Place::Unfolded));
    assert_eq!(place(&h, "Shadows"), Some(Place::Unfolded), "the part in the record");
    assert_eq!(place(&h, "Sun shadows"), None, "a folded-only row is not");
    h.get_by_label(H).get_by_label("Shadows");
    assert!(h.query_by_label("Sun shadows").is_none(), "unfolded: not the rows");
    unfolded.set(false);
    into_more.set(true);
    h.run();
    h.get_by_label(H).get_by_label(header::MORE).click();
    h.run();
    let more = header::more_name(H);
    h.get_by_label(&more).get_by_label("Creature shadows");
    assert!(!part(&h), "in More: not the part");
}

/// A master switch given words unfolds into a toggle with the title's
/// icon and those words (WideArea's Lighting), named as the switch; one
/// without, its icon alone.
#[test]
fn a_switch_unfolds_with_its_words() {
    let words = Rc::new(Cell::new(true));
    let w = words.clone();
    let mut on = true;
    let mut grid = false;
    let mut h = harness(vec2(900.0, 300.0), move |ui| {
        let mut pop = Popover::new("Lighting")
            .title(Some(Icon::Lightbulb), "Lighting")
            .switch(&mut on, "Use the area's lighting")
            .check(PopCheck::new(&mut grid, "Grid").icon(Icon::Grid));
        if w.get() {
            pop = pop.switch_words("Lighting");
        }
        let _ =
            EditorHeader::new(H).right(Item::popover(Some(Icon::Light), pop).unfolds()).show(ui);
    });
    h.run();
    let switch = h.get_by_label(H).get_by_label("Use the area's lighting");
    let with = switch.rect().width();
    assert!(with > mg_uikit::metrics::ICON_BUTTON + 20.0, "with its words: {with}");
    assert_eq!(switch.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    words.set(false);
    h.run();
    let alone = h.get_by_label(H).get_by_label("Use the area's lighting").rect().width();
    assert_eq!(alone, mg_uikit::metrics::ICON_BUTTON, "its icon alone");
}

/// A check row named apart from its words (the area's Audio row "Sounds",
/// named "Play Sounds"): found by its name, hung and unfolded, its words
/// no node's name; the header's record names it so.
#[test]
fn a_check_row_is_named_apart_from_its_words() {
    let unfolded = Rc::new(Cell::new(false));
    let u = unfolded.clone();
    let mut placed = false;
    let mut h = harness(vec2(900.0, 300.0), move |ui| {
        let pop = Popover::new("Audio")
            .check(PopCheck::new(&mut placed, "Sounds").name("Play Sounds").icon(Icon::Speaker));
        let item = Item::popover(Some(Icon::Speaker), pop);
        let _ = EditorHeader::new(H).right(if u.get() { item.unfolds() } else { item }).show(ui);
    });
    h.run();
    h.get_by_label(H).get_by_label("Audio").click();
    h.run();
    h.get_by_label("Play Sounds");
    assert!(h.query_by_label("Sounds").is_none(), "hung: named by its name");
    h.key_press(egui::Key::Escape);
    h.run();
    unfolded.set(true);
    h.run();
    h.get_by_label(H).get_by_label("Play Sounds");
    assert_eq!(place(&h, "Play Sounds"), Some(Place::Unfolded));
}

/// An item showing words apart from its name (the compact area header's
/// "Show 5 of 10", named "Show"): its button reads the words and is named
/// by the name, as wide as the words; a button whose icon is left out
/// beside its words (the Main board's Area Properties) is narrower by the
/// icon and its gap.
#[test]
fn an_item_shows_words_apart_from_its_name_and_a_button_its_words_alone() {
    let widths = Rc::new(RefCell::new(Vec::new()));
    let seen = widths.clone();
    let mut pop_on = false;
    let mut h = harness(vec2(900.0, 300.0), move |ui| {
        let words = KitButton::new(Some(Icon::Eye), "Show 5 of 10").opens(false).width(ui);
        let pop = Popover::new("Show").check(PopCheck::new(&mut pop_on, "Creatures"));
        let out = EditorHeader::new(H)
            .left(Item::popover(Some(Icon::Eye), pop).words_icon(true).shown_words("Show 5 of 10"))
            .right(Item::button(Icon::AreaProperties, "Area Properties").words_icon(false))
            .right(Item::button(Icon::AreaProperties, "Area Properties 2"))
            .show(ui);
        let w = |n: &str| out.response(n).map(|r| r.rect.width()).unwrap_or_default();
        *seen.borrow_mut() = vec![w("Show"), words, w("Area Properties"), w("Area Properties 2")];
    });
    h.run();
    h.get_by_label(H).get_by_label("Show");
    assert!(h.query_by_label("Show 5 of 10").is_none(), "named by its name");
    let w = widths.borrow().clone();
    assert_eq!(w[0], w[1], "as wide as its words: {w:?}");
    assert!(w[2] + metrics_icon() < w[3], "Area Properties without its icon: {w:?}");
}

/// The icon and its gap a word button's words take beside them.
fn metrics_icon() -> f32 {
    mg_uikit::metrics::ICON - mg_uikit::button::BUTTON_INSET + mg_uikit::button::BUTTON_GAP
}

/// A combo box drawn open in a `Ui` given the kit's combo look is
/// `raised-hover` (`.combo.open`), not the theme's `bar`, which the
/// windows' title bars keep; a `Ui` without the suite's look is left as it
/// is.
#[test]
fn an_open_combo_box_is_raised_hover() {
    let seen = Rc::new(Cell::new(None));
    let s = seen.clone();
    let mut h = harness(vec2(300.0, 200.0), move |ui| {
        let theme = ui.visuals().widgets.open.weak_bg_fill;
        ui.scope(|ui| {
            mg_uikit::combo_style(ui);
            s.set(Some((theme, ui.visuals().widgets.open.weak_bg_fill, ui.tokens())));
        });
    });
    h.run();
    let (theme, combo, t) = seen.get().unwrap();
    assert_eq!(theme, t.bar, "the theme's open fill");
    assert_eq!(combo, t.raised_hover, "the combo box's");
    let mut plain = Harness::builder().with_size(vec2(300.0, 200.0)).build_ui(|ui| {
        let before = ui.visuals().widgets.open;
        mg_uikit::combo_style(ui);
        assert_eq!(ui.visuals().widgets.open, before, "egui's look left as it is");
    });
    plain.run();
}

/// With only a right group of popovers that unfold (no left group's words
/// to go first), a popover's own words going, or one going into More,
/// leaves no popover unfolded: unfolding again is for a row where only
/// popovers were folded.
#[test]
fn no_popover_unfolds_again_once_words_went() {
    let width = Rc::new(Cell::new(1200.0));
    let w = width.clone();
    let mut values = [false; 6];
    let mut h = harness(vec2(1200.0, 300.0), move |ui| {
        let [a, b, c, d, x, y] = &mut values;
        let wide = Popover::new("Wide")
            .check(PopCheck::new(a, "Alpha one").icon(Icon::Grid).unfolded_words(true))
            .check(PopCheck::new(b, "Beta two").icon(Icon::Minimap).unfolded_words(true))
            .check(PopCheck::new(c, "Gamma three").icon(Icon::Fog).unfolded_words(true))
            .check(PopCheck::new(d, "Delta four").icon(Icon::Grass).unfolded_words(true));
        let narrow = Popover::new("Narrow")
            .check(PopCheck::new(x, "Ex").icon(Icon::Speaker))
            .check(PopCheck::new(y, "Why").icon(Icon::Ambient));
        let _ = ui.allocate_ui(vec2(w.get(), 40.0), |ui| {
            EditorHeader::new(H)
                .right(Item::popover(Some(Icon::Light), wide).unfolds())
                .right(Item::popover(Some(Icon::Speaker), narrow).unfolds())
                .show(ui)
        });
    });
    for px in (40..1200).rev().step_by(16) {
        width.set(px as f32);
        h.run();
        let places = header::places(&h.ctx, H);
        let mine: Vec<(String, Place)> =
            places.into_iter().filter(|(n, _)| n == "Wide" || n == "Narrow").collect();
        let yielded = mine.iter().any(|(_, p)| matches!(p, Place::Icon | Place::More));
        let unfolded = mine.iter().any(|(_, p)| *p == Place::Unfolded);
        assert!(!(yielded && unfolded), "at {px}: {mine:?}");
    }
}

/// A weak count after an item's words (the compact area header's Show
/// "5 of 10", `.count-w`): the button wider by the count and its gap, its
/// name its words alone; with its icon alone, no count.
#[test]
fn an_item_s_count_follows_its_words() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let out = seen.clone();
    let mut on = false;
    let mut h = harness(vec2(900.0, 300.0), move |ui| {
        let plain = KitButton::new(Some(Icon::Eye), "Show").opens(false).width(ui);
        let font = egui::FontId::proportional(mg_uikit::metrics::SMALL_SIZE);
        let galley = ui.painter().layout_no_wrap("5 of 10".into(), font, egui::Color32::WHITE);
        let pop = Popover::new("Show").check(PopCheck::new(&mut on, "Creatures"));
        let r = EditorHeader::new(H)
            .left(Item::popover(Some(Icon::Eye), pop).words_icon(true).count("5 of 10"))
            .show(ui);
        let w = r.response("Show").map(|r| r.rect.width()).unwrap_or_default();
        *out.borrow_mut() = vec![w, plain + mg_uikit::button::COUNT_GAP + galley.size().x];
    });
    h.run();
    h.get_by_label(H).get_by_label("Show");
    assert!(h.query_by_label_contains("5 of 10").is_none(), "not in its name");
    let w = seen.borrow().clone();
    assert!((w[0] - w[1].ceil()).abs() <= 1.0, "wider by the count: {w:?}");
}

/// A joined group's words between its buttons (`Joined::words`, the
/// AreaTiles board's ‹ Appearance ›): drawn between them, as wide as the
/// words and their padding; not a control (no node, no name in the
/// header's record) and no row of its own in More, where the group's
/// buttons go with the group's name as their heading.
#[test]
fn a_joined_group_s_words_stand_between_its_buttons() {
    use mg_uikit::header::{JOIN_SEAM, JOIN_WORD_PADDING, Joined};
    let seen = Rc::new(RefCell::new(Vec::new()));
    let out = seen.clone();
    let mut h = harness(vec2(600.0, 300.0), move |ui| {
        let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
        let words = ui.painter().layout_no_wrap("Appearance".into(), font, egui::Color32::WHITE);
        let r = EditorHeader::new(H)
            .left(Item::joined(
                "Appearance",
                vec![
                    Joined::button(Icon::ChevronLeft, "Previous"),
                    Joined::words("Appearance"),
                    Joined::button(Icon::ChevronRight, "Next"),
                ],
            ))
            .show(ui);
        let rect = |n: &str| r.response(n).map(|r| r.rect).unwrap_or(egui::Rect::NOTHING);
        let (a, b) = (rect("Previous"), rect("Next"));
        let gap = b.left() - a.right();
        let want = (words.size().x + 2.0 * JOIN_WORD_PADDING).ceil() + 2.0 * JOIN_SEAM;
        *out.borrow_mut() = vec![gap, want];
    });
    h.run();
    let w = seen.borrow().clone();
    assert!((w[0] - w[1]).abs() <= 1.0, "the words between the buttons: {w:?}");
    h.get_by_label(H).get_by_label("Previous");
    h.get_by_label(H).get_by_label("Next");
    assert!(h.query_by_label("Appearance").is_none(), "the words are no control");
    let names: Vec<String> = header::places(&h.ctx, H).into_iter().map(|(n, _)| n).collect();
    assert!(names.contains(&"Previous".to_string()) && names.contains(&"Next".to_string()));
    assert_eq!(names.iter().filter(|n| *n == "Appearance").count(), 1, "the group's name alone");
}

/// A check row given its icon for its unfolded toggle alone
/// (`PopCheck::unfolded_icon`, M5's W2, ruling 11: the Main board's
/// Overlays rows have no icons, WideArea's toggles have theirs): hung,
/// the row draws no icon and is no wider for one; unfolded into the
/// header, its toggle has the icon beside its words, as a row given
/// `PopCheck::icon` has.
#[test]
fn a_check_row_s_unfolded_icon_is_its_toggle_s_alone() {
    const WORDS: &str = "Grid drawn over the whole of the ground";
    // (The hung row's width and the shapes other than text painted in it,
    // and the unfolded toggle's width, for a row given `icon` or
    // `unfolded_icon`.)
    let measure = |lead: bool| {
        let unfold = Rc::new(Cell::new(false));
        let u = unfold.clone();
        let mut on = false;
        let mut h = harness(vec2(900.0, 400.0), move |ui| {
            let row = PopCheck::new(&mut on, WORDS).unfolded_words(true);
            let row = if lead { row.icon(Icon::Grid) } else { row.unfolded_icon(Icon::Grid) };
            let item = Item::popover(Some(Icon::Overlays), Popover::new("Overlays").check(row));
            let _ =
                EditorHeader::new(H).right(if u.get() { item.unfolds() } else { item }).show(ui);
        });
        h.run();
        h.get_by_label(H).get_by_label("Overlays").click();
        h.run();
        let row = h.get_by_label(WORDS).rect();
        // (Its box and its icon's strokes: its words are text.)
        let painted = (h.output().shapes.iter())
            .filter(|c| !matches!(c.shape, egui::Shape::Text(_)))
            .filter(|c| row.contains_rect(c.shape.visual_bounding_rect()))
            .count();
        h.key_press(egui::Key::Escape);
        h.run();
        unfold.set(true);
        h.run();
        let toggle = h.get_by_label(H).get_by_label(WORDS).rect().width();
        (row.width(), painted, toggle)
    };
    let (lead_w, lead_painted, lead_toggle) = measure(true);
    let (bare_w, bare_painted, bare_toggle) = measure(false);
    let icon = mg_uikit::metrics::ICON;
    assert!(bare_w < lead_w - icon, "no room for an icon in the row: {bare_w} of {lead_w}");
    assert!(bare_painted < lead_painted, "no icon painted: {bare_painted} of {lead_painted}");
    assert_eq!(bare_toggle, lead_toggle, "the toggle has its icon and words");
}
