//! The editor header, the popover and the tool shelf, as the editors
//! will drive them: the header's yields at every width, found by name on
//! the row or in More; a popover's hang, switch, rows and a combo box in
//! it; the shelf's tools; and the kit button's disabled look.

use std::cell::{Cell, RefCell};

use egui::accesskit::Toggled;
use egui::{Rect, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::header::{self, EditorHeader, Item, Place};
use mg_uikit::popover::{Hang, PopAction, PopCheck, Popover};
use mg_uikit::shelf::{Tool, ToolShelf};
use mg_uikit::{Icon, KitButton, Theme, TokensExt as _};

const HEADER: &str = "Test header";
/// The end slot's width (OK and Cancel).
const END: f32 = 120.0;
/// The row's least width with More and its line beside the end slot:
/// under it what is in More is cut (the end slot keeps the right end).
const MORE_ROOM: f32 = END + 2.0 * header::HEADER_PADDING + 22.0 + 9.0 + 2.0 * header::HEADER_GAP;

/// The values the header's toggles and rows change.
#[derive(Default)]
struct Values {
    day: bool,
    grid: bool,
    minimap: bool,
    bookmark: bool,
}

/// The header the yield tests narrow: a script's left group, a codepage
/// that never yields and a toggle and a popover (which unfolds) at the
/// right, OK and Cancel at the end.
fn test_header(ui: &mut egui::Ui, v: &mut Values) -> header::HeaderOutput<()> {
    let lighting = Popover::new("Lighting")
        .title(Some(Icon::Light), "Lighting")
        .check(PopCheck::new(&mut v.grid, "Grid").icon(Icon::Grid).unfolded_words(true))
        .check(PopCheck::new(&mut v.minimap, "Minimap").icon(Icon::Minimap));
    EditorHeader::new(HEADER)
        .left(Item::button(Icon::Save, "Save").primary())
        .left(Item::button(Icon::Compile, "Compile"))
        .left(Item::separator())
        .left(Item::icon(Icon::Open, "Open…"))
        .left(Item::icon(Icon::Print, "Print…"))
        .left(Item::separator())
        .left(Item::button(Icon::Find, "Find…"))
        .left(Item::button(Icon::Replace, "Replace…"))
        .left(Item::toggle(Some(Icon::Bookmark), "Bookmark", &mut v.bookmark).icon_only())
        .left(Item::icon(Icon::ToScratch, "To Scratch").in_more())
        .right(Item::custom("Codepage", 90.0, |ui| {
            ui.weak("Windows-1252");
        }))
        .right(Item::toggle(Some(Icon::Sun), "Day", &mut v.day))
        .right(Item::popover(Some(Icon::Light), lighting).unfolds())
        .show_with_end(ui, END, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let _ = ui.add(KitButton::new(None, "OK"));
            let _ = ui.add(KitButton::new(None, "Cancel"));
        })
}

/// The names the header holds (its items and its popover's rows).
const NAMES: [&str; 12] = [
    "Save",
    "Compile",
    "Open…",
    "Print…",
    "Find…",
    "Replace…",
    "Bookmark",
    "To Scratch",
    "Codepage",
    "Day",
    "Lighting",
    "OK",
];

struct Run {
    places: Vec<(String, Place)>,
    row: Rect,
    end: Option<Rect>,
}

fn harness(width: f32) -> (Harness<'static>, std::rc::Rc<RefCell<Option<Run>>>) {
    let run = std::rc::Rc::new(RefCell::new(None));
    let seen = run.clone();
    let mut values = Values::default();
    let h = Harness::builder().with_size(vec2(width, 300.0)).build_ui(move |ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        header::record_headers(ui.ctx());
        let out = test_header(ui, &mut values);
        *seen.borrow_mut() =
            Some(Run { places: out.places, row: out.response.rect, end: out.end_rect });
    });
    (h, run)
}

fn rank(place: Place) -> u8 {
    match place {
        Place::Unfolded => 0,
        Place::Row => 1,
        Place::Icon => 2,
        Place::More => 3,
        Place::Cut => 4,
    }
}

fn place_of(places: &[(String, Place)], name: &str) -> Place {
    places.iter().find(|(n, _)| n == name).map(|(_, p)| *p).unwrap_or(Place::Row)
}

/// Each name is found exactly once: on the row (within the header's
/// group) or, having opened More, in its popover; More is closed after.
fn found_once(h: &mut Harness<'_>, places: &[(String, Place)], width: f32) {
    let more_name = header::more_name(HEADER);
    let mut in_more = Vec::new();
    for name in NAMES.iter().filter(|n| **n != "OK" && **n != "Codepage") {
        match place_of(places, name) {
            Place::Row | Place::Icon => {
                let count = h.query_all_by_label(name).count();
                assert_eq!(count, 1, "{name} at {width}: {count} found");
                h.get_by_label(HEADER).get_by_label(name);
            }
            Place::Unfolded => {
                assert!(h.query_by_label(name).is_none(), "{name} unfolded at {width}");
                h.get_by_label(HEADER).get_by_label("Grid");
                h.get_by_label(HEADER).get_by_label("Minimap");
            }
            Place::More => in_more.push(*name),
            Place::Cut => assert!(width < MORE_ROOM, "{name} cut at {width}"),
        }
    }
    if in_more.is_empty() {
        assert!(h.query_by_label(header::MORE).is_none(), "a More with nothing in it at {width}");
        return;
    }
    h.get_by_label(HEADER).get_by_label(header::MORE).click();
    h.run();
    for name in &in_more {
        let count = h.query_all_by_label(name).count();
        assert_eq!(count, 1, "{name} in More at {width}: {count} found");
        h.get_by_label(&more_name).get_by_label(name);
    }
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.query_by_label(&more_name).is_none(), "More stays open at {width}");
}

/// Narrowed from its full width to its end slot's and widened again, the
/// header yields in order: an unfolded popover folds, the left group's
/// words go from its end, then the right group's, then they move into
/// More in the same order; the end slot never yields, keeps the right
/// end and nothing is drawn over it or past the row; a width gives the
/// same row both ways; every name is found once, on the row or in More.
#[test]
fn the_header_yields_words_then_icons_then_more() {
    let (mut h, run) = harness(1400.0);
    let full = 1400.0;
    // (The harness's margins: 8 at each side of the row.)
    let least = END + 2.0 * header::HEADER_PADDING + 16.0;
    let mut widths: Vec<f32> = Vec::new();
    let mut w = full;
    while w >= least {
        widths.push(w);
        w -= 20.0;
    }
    widths.push(least);
    let mut down: Vec<Vec<(String, Place)>> = Vec::new();
    let mut last: Option<Vec<(String, Place)>> = None;
    for (step, &w) in widths.iter().enumerate() {
        h.set_size(vec2(w, 300.0));
        h.run();
        let r = run.borrow_mut().take().expect("drawn");
        // The end slot keeps the right end, at its width.
        let end = r.end.expect("an end slot");
        assert!((end.width() - END).abs() < 0.5, "{w}: {end:?}");
        assert!(
            (end.right() - (r.row.right() - header::HEADER_PADDING)).abs() < 0.5,
            "{w}: {end:?} in {:?}",
            r.row
        );
        // Nothing past the row, nothing over the end slot.
        for n in h.get_by_label(HEADER).query_all_by(|n| n.label().is_some()) {
            let Some(b) = n.accesskit_node().bounding_box() else { continue };
            let label = n.accesskit_node().label().unwrap_or_default();
            if label == "OK" || label == "Cancel" {
                continue;
            }
            assert!(b.x1 <= r.row.right() as f64 + 0.5, "{label} past the row at {w}");
            let x0 = end.left() as f64 - 0.5;
            assert!(b.x1 <= x0 || b.x0 >= end.right() as f64, "{label} over the end at {w}");
        }
        // Monotone: nothing comes back as the row narrows.
        if let Some(last) = &last {
            for (name, place) in &r.places {
                let before = place_of(last, name);
                assert!(rank(*place) >= rank(before), "{name} {before:?} to {place:?} at {w}");
            }
        }
        // The order: words go from the left group's end, and nothing goes
        // into More while a word button keeps its words.
        let p = |n: &str| place_of(&r.places, n);
        let words = ["Save", "Compile", "Find…", "Replace…", "Day"];
        let gone: Vec<bool> = words.iter().map(|n| rank(p(n)) >= rank(Place::Icon)).collect();
        for i in 0..3 {
            if gone[i] {
                assert!(gone[i + 1], "{} lost its words before {} at {w}", words[i], words[i + 1]);
            }
        }
        if gone[4] {
            assert!(gone[..4].iter().all(|g| *g), "Day lost its words first at {w}");
        }
        let in_more = |n: &str| p(n) == Place::More;
        let moved = ["Save", "Compile", "Open…", "Print…", "Find…", "Replace…", "Bookmark", "Day"];
        if moved.iter().any(|n| in_more(n)) {
            assert!(gone.iter().all(|g| *g), "into More before the words were gone at {w}");
        }
        // An unfolded popover folds before anything loses its words.
        if gone.iter().any(|g| *g) {
            assert_ne!(p("Lighting"), Place::Unfolded, "words went before the fold at {w}");
        }
        // Into More: the left group from its end (each one in More has
        // every later one of the left group in More), then the right
        // group, and Save, the first command, last of everything.
        let gone_far = |n: &str| matches!(p(n), Place::More | Place::Cut);
        let left = ["Compile", "Open…", "Print…", "Find…", "Replace…", "Bookmark"];
        for i in 0..left.len() {
            if gone_far(left[i]) {
                for later in &left[i + 1..] {
                    assert!(gone_far(later), "{} into More before {later} at {w}", left[i]);
                }
            }
        }
        if gone_far("Day") || gone_far("Lighting") {
            assert!(left.iter().all(|n| gone_far(n)), "the right group before the left at {w}");
        }
        if gone_far("Save") {
            for n in left.iter().chain(&["Day", "Lighting"]) {
                assert!(gone_far(n), "Save into More before {n} at {w}");
            }
        }
        let row = r.row.width();
        assert!(in_more("To Scratch") || (row < MORE_ROOM && p("To Scratch") == Place::Cut));
        assert_ne!(p("Codepage"), Place::More, "a part that never yields in More");
        if step % 4 == 0 || w == least {
            found_once(&mut h, &r.places, row);
        }
        last = Some(r.places.clone());
        down.push(r.places);
    }
    // At full width the popover is unfolded and every word is shown.
    assert_eq!(place_of(&down[0], "Lighting"), Place::Unfolded);
    assert_eq!(place_of(&down[0], "Save"), Place::Row);
    // At the least, everything that may be is in More.
    let least_places = down.last().unwrap();
    for name in ["Save", "Lighting"] {
        assert!(matches!(place_of(least_places, name), Place::More | Place::Cut), "{name}");
    }
    // And back up: the same row at each width.
    for (i, &w) in widths.iter().enumerate().rev() {
        h.set_size(vec2(w, 300.0));
        h.run();
        let r = run.borrow_mut().take().expect("drawn");
        assert_eq!(r.places, down[i], "widened to {w}");
    }
}

/// A button in More is reported clicked and closes More; a toggle there
/// is a check row that flips its value and leaves it open.
#[test]
fn more_holds_buttons_and_toggles() {
    let clicked = Cell::new(false);
    let bookmark = Cell::new(false);
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let mut on = bookmark.get();
        let out = EditorHeader::new(HEADER)
            .left(Item::button(Icon::Save, "Save"))
            .left(Item::toggle(Some(Icon::Bookmark), "Bookmark", &mut on).in_more())
            .left(Item::icon(Icon::ToScratch, "To Scratch").in_more())
            .show(ui);
        bookmark.set(on);
        if out.clicked("To Scratch") {
            clicked.set(true);
        }
    });
    h.run();
    let more = header::more_name(HEADER);
    h.get_by_label(HEADER).get_by_label(header::MORE).click();
    h.run();
    h.get_by_label(&more).get_by_label("Bookmark").click();
    h.run();
    assert!(bookmark.get());
    assert!(h.query_by_label(&more).is_some(), "a check row closed More");
    h.get_by_label(&more).get_by_label("To Scratch").click();
    h.run();
    assert!(clicked.get());
    assert!(h.query_by_label(&more).is_none(), "a button's row left More open");
}

/// Where the popover hangs: below-right of a button right of the
/// header's middle, its right edge at the button's and its top 4 px under
/// it, so that it never reaches the top-right corner beside the button;
/// below a button left of the middle, left edges aligned.
#[test]
fn a_popover_hangs_below_right_never_over_the_top_right_corner() {
    let shown = Cell::new(None::<Rect>);
    let mut h = Harness::builder().with_size(vec2(1000.0, 400.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let mut a = false;
        let mut b = false;
        let left = Popover::new("Snap").check(PopCheck::new(&mut a, "Snap to grid"));
        let right = Popover::new("Overlays").check(PopCheck::new(&mut b, "Grid"));
        let out = EditorHeader::new(HEADER)
            .left(Item::popover(None, left))
            .right(Item::popover(None, right))
            .right(Item::words("Area Properties"))
            .show(ui);
        shown.set(out.popover_rect);
    });
    h.run();
    for name in ["Overlays", "Snap"] {
        h.get_by_label(HEADER).get_by_label(name).click();
        h.run();
        let b = h.get_by_label(HEADER).get_by_label(name).rect();
        let p = shown.get().unwrap_or_else(|| panic!("{name} not shown"));
        assert!(p.top() >= b.bottom() + 3.5 && p.top() <= b.bottom() + 4.5, "{name}: {p:?} {b:?}");
        if name == "Overlays" {
            assert!((p.right() - b.right()).abs() < 1.0, "{p:?} {b:?}");
            // The corner right of the button, over the view, is clear.
            let corner = Rect::from_min_max(
                egui::pos2(b.right() + 1.0, b.bottom()),
                egui::pos2(1000.0, 400.0),
            );
            assert!(!p.intersects(corner), "{p:?} over {corner:?}");
        } else {
            assert!((p.left() - b.left()).abs() < 1.0, "{p:?} {b:?}");
        }
        h.key_press(egui::Key::Escape);
        h.run();
        assert!(shown.get().is_none(), "{name} stays open after Escape");
    }
}

/// A master switch: off, its rows are said disabled and keep their
/// values; on again, they are enabled as they were.
#[test]
fn a_master_switch_turns_its_rows_off_and_back() {
    let on = Cell::new(true);
    let grid = Cell::new(true);
    let mut h = Harness::builder().with_size(vec2(600.0, 300.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let (mut o, mut g) = (on.get(), grid.get());
        let button = ui.add(KitButton::new(None, "Overlays").opens(true));
        let pop = Popover::new("Overlays popover")
            .title(Some(Icon::Overlays), "Overlays")
            .switch(&mut o, "Show overlays")
            .check(PopCheck::new(&mut g, "Grid"));
        let _ = pop.show(ui, &button);
        on.set(o);
        grid.set(g);
    });
    h.run();
    h.get_by_label("Overlays").click();
    h.run();
    let pop = h.get_by_label("Overlays popover");
    let switch = pop.get_by_label("Show overlays");
    assert_eq!(switch.accesskit_node().toggled(), Some(Toggled::True));
    assert!(!h.get_by_label("Grid").accesskit_node().is_disabled());
    h.get_by_label("Show overlays").click();
    h.run();
    assert!(!on.get());
    assert_eq!(h.get_by_label("Show overlays").accesskit_node().toggled(), Some(Toggled::False));
    let grid_row = h.get_by_label("Grid");
    assert!(grid_row.accesskit_node().is_disabled(), "the rows follow the switch");
    assert_eq!(grid_row.accesskit_node().toggled(), Some(Toggled::True), "and keep their values");
    h.get_by_label("Show overlays").click();
    h.run();
    assert!(on.get());
    assert!(!h.get_by_label("Grid").accesskit_node().is_disabled());
    assert!(grid.get());
}

/// A combo box inside a popover: choosing from its list leaves the
/// popover open (egui keeps one popup open at a time; the popover is the
/// kit's own layer); a click outside then closes it.
#[test]
fn a_combo_inside_a_popover_leaves_it_open() {
    let chosen = Cell::new("Never");
    let mut h = Harness::builder().with_size(vec2(600.0, 400.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let button = ui.add(KitButton::new(None, "Lighting").opens(false));
        let mut value = chosen.get();
        let pop = Popover::new("Lighting popover").custom(|ui| {
            ui.label("Fade geometry");
            egui::ComboBox::from_id_salt("fade").selected_text(value).show_ui(ui, |ui| {
                for v in ["Never", "Object Mode Only", "Always"] {
                    ui.selectable_value(&mut value, v, v);
                }
            });
        });
        let _ = pop.show(ui, &button);
        chosen.set(value);
        ui.add_space(200.0);
        ui.label("Outside");
    });
    h.run();
    h.get_by_label("Lighting").click();
    h.run();
    h.get_by_label("Lighting popover").get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    h.get_by_label("Always").click();
    h.run();
    assert_eq!(chosen.get(), "Always");
    assert!(h.query_by_label("Lighting popover").is_some(), "the combo closed the popover");
    h.get_by_label("Outside").click();
    h.run();
    assert!(h.query_by_label("Lighting popover").is_none(), "a click outside left it open");
}

/// An action row is reported chosen and closes its popover.
#[test]
fn an_action_row_closes_its_popover() {
    let chosen = Cell::new(false);
    let mut h = Harness::builder().with_size(vec2(600.0, 300.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let button = ui.add(KitButton::new(None, "Export").opens(false));
        let pop = Popover::new("Export popover").action(PopAction::new("CSV…").end("Ctrl+E"));
        if pop.show(ui, &button).clicked("CSV…") {
            chosen.set(true);
        }
    });
    h.run();
    h.get_by_label("Export").click();
    h.run();
    h.get_by_label("CSV…").click();
    h.run();
    assert!(chosen.get());
    assert!(h.query_by_label("Export popover").is_none());
}

/// The shelf: one group named Tools; the tool on said selected; a click
/// reports its index; a disabled tool is said so and does nothing; it is
/// 36 wide with 28 px tools.
#[test]
fn the_tool_shelf_reports_its_tools() {
    let clicked = Cell::new(None);
    let mut h = Harness::builder().with_size(vec2(200.0, 300.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let out = ToolShelf::new()
            .tool(Tool::new(Icon::Select, "Select Objects").on(true))
            .tool(Tool::new(Icon::SelectTiles, "Select Tiles").tip("Select tiles (F10)"))
            .tool(Tool::new(Icon::Paint, "Paint").enabled(false))
            .separator()
            .tool(Tool::new(Icon::Terrain, "Terrain"))
            .show(ui);
        assert_eq!(out.response.rect.width(), mg_uikit::metrics::TOOL_SHELF);
        if out.clicked.is_some() {
            clicked.set(out.clicked);
        }
    });
    h.run();
    let tools = h.get_by_label(mg_uikit::shelf::TOOLS);
    let select = tools.get_by_label("Select Objects");
    assert_eq!(select.accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(select.rect().width(), mg_uikit::metrics::TOOL_BUTTON);
    assert_eq!(tools.get_by_label("Select Tiles").accesskit_node().toggled(), Some(Toggled::False));
    assert!(tools.get_by_label("Paint").accesskit_node().is_disabled());
    h.get_by_label(mg_uikit::shelf::TOOLS).get_by_label("Terrain").click();
    h.run();
    assert_eq!(clicked.get(), Some(3));
    clicked.set(None);
    h.get_by_label(mg_uikit::shelf::TOOLS).get_by_label("Paint").click();
    h.run();
    assert_eq!(clicked.get(), None);
}

/// A disabled kit button inside a disabled `Ui` is painted in
/// `text-off` with no fade (egui's would multiply its colors by the
/// disabled opacity), in both themes.
#[test]
fn a_disabled_kit_button_is_not_faded() {
    for theme in Theme::ALL {
        let mut h = Harness::builder().with_size(vec2(300.0, 100.0)).build_ui(move |ui| {
            mg_uikit::apply(ui.ctx(), theme);
            ui.add_enabled_ui(false, |ui| ui.add(KitButton::new(Some(Icon::Save), "Save")));
        });
        h.run();
        let t = h.ctx.tokens();
        let mut found = false;
        for clipped in &h.output().shapes {
            crate::common::visit_shape(&clipped.shape, &mut |shape| {
                if let egui::Shape::Text(text) = shape
                    && text.galley.text() == "Save"
                {
                    found = true;
                    assert!((text.opacity_factor - 1.0).abs() < 1e-4, "{theme:?}: faded");
                    assert_eq!(text.fallback_color, t.text_off, "{theme:?}");
                }
                if let egui::Shape::Rect(rect) = shape
                    && rect.rect.height() == mg_uikit::metrics::CONTROL_HEIGHT
                    && rect.corner_radius == mg_uikit::metrics::RADIUS.into()
                {
                    assert_eq!(rect.fill, t.raised, "{theme:?}: its fill {rect:?}");
                }
            });
        }
        assert!(found, "no Save drawn");
    }
}

/// `Hang::Auto` resolves by the button's side of the middle.
#[test]
fn auto_hangs_by_the_side_of_the_middle() {
    let frame = Rect::from_min_size(egui::Pos2::ZERO, vec2(1000.0, 30.0));
    let left = Rect::from_min_size(egui::pos2(100.0, 4.0), vec2(60.0, 22.0));
    let right = Rect::from_min_size(egui::pos2(800.0, 4.0), vec2(60.0, 22.0));
    assert_eq!(Hang::Auto.resolved(left, frame), Hang::Below);
    assert_eq!(Hang::Auto.resolved(right, frame), Hang::BelowRight);
    assert_eq!(Hang::Below.resolved(right, frame), Hang::Below);
}

/// A popover's button forced to its icon (the area's compact header):
/// its icon and chevron at any width, never unfolded, and it still opens
/// its popover.
#[test]
fn a_popover_button_can_be_kept_to_its_icon() {
    let place = Cell::new(None);
    let open = Cell::new(false);
    let mut h = Harness::builder().with_size(vec2(1200.0, 300.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        header::record_headers(ui.ctx());
        let mut grid = false;
        let pop = Popover::new("Overlays").check(PopCheck::new(&mut grid, "Grid").icon(Icon::Grid));
        let out = EditorHeader::new(HEADER)
            .right(Item::popover(Some(Icon::Overlays), pop).unfolds().icon_only())
            .show(ui);
        place.set(out.place("Overlays"));
        open.set(out.popover_rect.is_some());
    });
    h.run();
    assert_eq!(place.get(), Some(Place::Icon));
    let button = h.get_by_label(HEADER).get_by_label("Overlays");
    assert_eq!(button.rect().width(), 36.0, "its icon and chevron");
    button.click();
    h.run();
    assert!(open.get());
    h.get_by_label("Grid");
}

/// The row takes no stop of Tab of its own: from a button before it, Tab
/// goes to its first item.
#[test]
fn tab_goes_from_item_to_item() {
    let mut h = Harness::builder().with_size(vec2(800.0, 200.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let _ = ui.button("Before");
        let _ = EditorHeader::new(HEADER)
            .left(Item::button(Icon::Save, "Save"))
            .left(Item::button(Icon::Compile, "Compile"))
            .show(ui);
    });
    h.run();
    h.get_by_label("Before").focus();
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("Save").accesskit_node().is_focused(), "Tab went elsewhere");
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("Compile").accesskit_node().is_focused());
}

/// A menu's button opens its rows under it; a click on a row does what it
/// does and closes the menu.
#[test]
fn a_menu_opens_from_its_button_and_a_row_closes_it() {
    let chosen = Cell::new(false);
    let mut h = Harness::builder().with_size(vec2(800.0, 300.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let _ = EditorHeader::new(HEADER)
            .left(Item::menu(Some(Icon::Export), "Export", |ui| {
                if ui.button("CSV…").clicked() {
                    chosen.set(true);
                }
                let _ = ui.button("JSON…");
            }))
            .show(ui);
    });
    h.run();
    assert!(h.query_by_label("CSV…").is_none());
    h.get_by_label(HEADER).get_by_label("Export").click();
    h.run();
    h.get_by_label("CSV…").click();
    h.run();
    assert!(chosen.get());
    assert!(h.query_by_label("JSON…").is_none(), "the menu stayed open");
}

/// In More: a menu's rows under its heading, a part of the caller's in
/// its More form, a popover's rows under its name with its switch as a
/// check row; a check row there flips its value and leaves More open.
#[test]
fn more_holds_menus_parts_and_popovers() {
    let grid = Cell::new(false);
    let shown = Cell::new(true);
    let csv = Cell::new(false);
    let mut h = Harness::builder().with_size(vec2(260.0, 400.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let (mut g, mut s) = (grid.get(), shown.get());
        let pop = Popover::new("Overlays")
            .switch(&mut s, "Show overlays")
            .check(PopCheck::new(&mut g, "Grid"));
        let _ = EditorHeader::new(HEADER)
            .left(Item::menu(Some(Icon::Export), "Export", |ui| {
                if ui.button("CSV…").clicked() {
                    csv.set(true);
                }
            }))
            .left(
                Item::custom("Go to", 120.0, |ui| {
                    ui.label("on the row");
                })
                .more(160.0, |ui| {
                    ui.label("Go to, in More");
                }),
            )
            .right(Item::popover(Some(Icon::Overlays), pop))
            .show_with_end(ui, 150.0, |ui| {
                let _ = ui.add(KitButton::new(None, "OK"));
            });
        grid.set(g);
        shown.set(s);
    });
    h.run();
    let more = header::more_name(HEADER);
    h.get_by_label(HEADER).get_by_label(header::MORE).click();
    h.run();
    let pop = h.get_by_label(&more);
    pop.get_by_label("Export");
    pop.get_by_label("CSV…");
    pop.get_by_label("Go to, in More");
    pop.get_by_label("Overlays");
    assert!(h.query_by_label("on the row").is_none());
    // (Kept inside the window, More being near its left edge.)
    let rect = h.get_by_label(&more).get_by_label("Grid").rect();
    assert!(rect.left() >= 0.0, "More's popover past the window's edge: {rect:?}");
    h.get_by_label(&more).get_by_label("Grid").click();
    h.run();
    assert!(grid.get(), "the popover's row in More");
    h.get_by_label(&more).get_by_label("Show overlays").click();
    h.run();
    assert!(!shown.get(), "the switch's check row in More");
    assert!(h.get_by_label(&more).get_by_label("Grid").accesskit_node().is_disabled());
    h.get_by_label(&more).get_by_label("CSV…").click();
    h.run();
    assert!(csv.get());
    assert!(h.query_by_label(&more).is_none(), "a menu row left More open");
}

/// `more_after` puts More right after the item it names; `more_words`
/// makes it "More" with a chevron.
#[test]
fn more_stands_where_it_is_asked() {
    let mut h = Harness::builder().with_size(vec2(1000.0, 200.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let _ = EditorHeader::new(HEADER)
            .left(Item::words("Design"))
            .left(Item::words("Export…"))
            .left(Item::words("Import…").in_more())
            .left(Item::words("Interact"))
            .right(Item::words("View"))
            .more_after("Export…")
            .more_words(true)
            .show(ui);
    });
    h.run();
    let row = h.get_by_label(HEADER);
    let export = row.get_by_label("Export…").rect();
    let more = row.get_by_label(header::MORE).rect();
    let interact = row.get_by_label("Interact").rect();
    assert!(more.left() > export.right() && more.right() < interact.left(), "{more:?}");
    assert!(more.width() > 40.0, "More with its words: {more:?}");
}

/// A line at the right group's end stays where More follows it, and goes
/// where nothing follows it.
#[test]
fn a_line_stands_before_more() {
    fn lines(with_more: bool) -> usize {
        let mut h = Harness::builder().with_size(vec2(800.0, 200.0)).build_ui(move |ui| {
            mg_uikit::apply(ui.ctx(), Theme::Dark);
            let mut header = EditorHeader::new(HEADER)
                .left(Item::button(Icon::Save, "Save"))
                .right(Item::custom("Codepage", 90.0, |ui| {
                    ui.weak("Windows-1252");
                }))
                .right(Item::separator());
            if with_more {
                header = header.left(Item::icon(Icon::ToScratch, "To Scratch").in_more());
            }
            let _ = header.show(ui);
        });
        h.run();
        let line = h.ctx.tokens().line_soft;
        let mut count = 0;
        for clipped in &h.output().shapes {
            crate::common::visit_shape(&clipped.shape, &mut |shape| {
                if let egui::Shape::Rect(r) = shape
                    && r.fill == line
                    && r.rect.size() == vec2(1.0, 16.0)
                {
                    count += 1;
                }
            });
        }
        count
    }
    assert_eq!(lines(true), 1, "the line before More");
    assert_eq!(lines(false), 0, "a line with nothing after it");
}

/// A popover shown open is seen by `Popover::any_open` (Escape's rule
/// asks it: egui's popup memory does not see the kit's popovers) and
/// closed is not.
#[test]
fn an_open_popover_is_seen_open() {
    let mut h = Harness::builder().with_size(vec2(600.0, 300.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let mut grid = false;
        let button = ui.add(KitButton::new(None, "Overlays").opens(false));
        let _ = Popover::new("Overlays popover")
            .check(PopCheck::new(&mut grid, "Grid"))
            .show(ui, &button);
    });
    h.run();
    assert!(!Popover::any_open(&h.ctx));
    h.get_by_label("Overlays").click();
    h.run();
    assert!(Popover::any_open(&h.ctx));
    assert!(!egui::Popup::is_any_open(&h.ctx), "egui does not see it");
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!Popover::any_open(&h.ctx));
}
