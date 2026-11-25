//! Three dialogs built to the Dialogs board (M4, U27's fix): the Creature
//! Wizard's Class and Level (the classes in a well, Add Class across its
//! column, a row a class with its level and a ×, "N of 8 classes"), the
//! Area Wizard's Size (Tiny, Small, Medium and Large as a segmented
//! control over the rows × columns) and Find and Replace Text (its fields
//! across the dialog, the results in a well).

use crate::common::*;
use egui::accesskit::{Role, Toggled};
use mg_ui::creature_wizard::CreatureWizard;
use mg_ui::wizards::Wizard;

/// classes.2da's rows: Barbarian, Bard, Cleric, Druid, Fighter, Monk,
/// Paladin, Ranger.
const CLERIC: u32 = 2;
const FIGHTER: u32 = 4;

/// The Creature Wizard at Class and Level with `classes` added and
/// `chosen` chosen in the list, in a window of `size`.
fn class_page(
    name: &str,
    size: egui::Vec2,
    classes: Vec<(u32, u32)>,
    chosen: Option<u32>,
) -> Option<Harness<'static, Moonglow>> {
    let mut h = game_harness(name)?;
    h.set_size(size);
    h.state_mut().creature_wizard = Some(CreatureWizard {
        page: 2,
        race: Some(6),
        classes,
        chosen_class: chosen,
        ..Default::default()
    });
    settle(&mut h);
    Some(h)
}

fn classes(h: &Harness<'_, Moonglow>) -> Vec<(u32, u32)> {
    h.state().creature_wizard.as_ref().expect("open").classes.clone()
}

fn enabled(h: &Harness<'_, Moonglow>, name: &str) -> bool {
    !h.get_by_label(name).accesskit_node().is_disabled()
}

/// A class chosen in the list and added by Add Class gets a row (level 1)
/// and the count says two; its × takes it out again, and the count says
/// one; the last class left has no ×. Add Class is off with nothing chosen.
#[test]
fn the_class_page_adds_and_removes_classes_and_counts_them() {
    let Some(mut h) =
        class_page("board-classes", egui::vec2(1280.0, 720.0), vec![(FIGHTER, 4)], None)
    else {
        return;
    };
    h.get_by_label("1 of 8 classes");
    assert!(!enabled(&h, "Add Class"), "nothing chosen: Add Class off");
    assert!(h.query_by_label("Remove Fighter").is_none(), "the only class has no ×");
    h.get_by_label("Cleric").click();
    settle(&mut h);
    assert!(enabled(&h, "Add Class"), "a class chosen: Add Class on");
    h.get_by_label("Add Class").click();
    settle(&mut h);
    assert_eq!(classes(&h), [(FIGHTER, 4), (CLERIC, 1)]);
    h.get_by_label("2 of 8 classes");
    assert!(
        h.query_by_label("Cleric").is_some_and(|_| h.query_by_label("Remove Cleric").is_some())
    );
    // (The row: its name, its level, its × in a line, the × at the right.)
    let fighter = h.get_by_label("Remove Fighter").rect();
    let cleric = h.get_by_label("Remove Cleric").rect();
    assert!((fighter.left() - cleric.left()).abs() < 0.5, "×s in a column: {fighter:?} {cleric:?}");
    assert!(cleric.top() > fighter.bottom(), "Cleric's row under Fighter's");
    let add = h.get_by_label("Add Class").rect();
    assert!(cleric.right() <= add.right() + 0.5, "the × within Add Class's column");
    h.get_by_label("Remove Cleric").click();
    settle(&mut h);
    assert_eq!(classes(&h), [(FIGHTER, 4)]);
    h.get_by_label("1 of 8 classes");
}

/// With eight classes Add Class is off though one is chosen, a
/// double-click adds none, and the count says eight of eight.
#[test]
fn add_class_is_off_at_eight_classes() {
    let eight: Vec<(u32, u32)> = (0..8).map(|k| (k, 1)).collect();
    let Some(mut h) = class_page("board-eight", egui::vec2(1280.0, 720.0), eight.clone(), Some(10))
    else {
        return;
    };
    h.get_by_label("8 of 8 classes");
    assert!(!enabled(&h, "Add Class"), "eight classes: Add Class off");
    let wizard = h.get_by_label("Wizard").rect().center();
    double_click_now(&mut h, wizard);
    settle(&mut h);
    assert_eq!(classes(&h), eight, "a double-click adds none");
}

/// The Class and Level page holds in the wizard at 1280 by 720 and at the
/// least window with eight classes: Add Class, each row's level field and
/// ×, and the count in the window, right of the list's rows, none over
/// another.
#[test]
fn the_class_page_fits_at_1280_and_at_the_least_window() {
    for size in [egui::vec2(1280.0, 720.0), mg_ui::LEAST_WINDOW_SIZE] {
        let eight: Vec<(u32, u32)> = (0..8).map(|k| (k, 2)).collect();
        let Some(h) = class_page("board-fit", size, eight, None) else { return };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        let add = h.get_by_label("Add Class").rect();
        assert!(screen.contains_rect(add), "{size:?}: Add Class {add:?}");
        assert!(add.height() < 30.0 && add.width() > 100.0, "{size:?}: Add Class {add:?}");
        // (The list's first row in sight, left of the column.)
        let list = h.get_by_label("Wizard").rect();
        assert!(list.right() < add.left(), "{size:?}: the list {list:?} left of {add:?}");
        let count = h.get_by_label("8 of 8 classes").rect();
        assert!(screen.contains_rect(count), "{size:?}: the count {count:?}");
        assert!(count.top() > add.bottom(), "{size:?}: the count under Add Class");
        let close = h.get_by_label("Remove Barbarian").rect();
        assert!(close.right() <= add.right() + 0.5, "{size:?}: × {close:?} past {add:?}");
        assert!(close.top() >= add.bottom(), "{size:?}: × {close:?} over Add Class");
    }
}

fn area_size(h: &Harness<'_, Moonglow>) -> (u32, u32) {
    match &h.state().wizard {
        Some(Wizard::NewArea(w)) => (w.width, w.height),
        other => panic!("the Area Wizard: {other:?}"),
    }
}

fn set_area_size(h: &mut Harness<'_, Moonglow>, size: (u32, u32)) {
    if let Some(Wizard::NewArea(w)) = &mut h.state_mut().wizard {
        (w.width, w.height) = size;
    }
}

/// Which of the Size segments is chosen (toggled), by name.
fn chosen_segment(h: &Harness<'_, Moonglow>) -> Vec<&'static str> {
    ["Tiny", "Small", "Medium", "Large"]
        .into_iter()
        .filter(|s| {
            let node = h.get_by_label(s);
            assert_eq!(node.accesskit_node().role(), Role::RadioButton, "{s} a radio button");
            node.accesskit_node().toggled() == Some(Toggled::True)
        })
        .collect()
}

/// The Area Wizard's Size page: a radio group named Area size of Tiny, Small, Medium
/// and Large in a row, Medium chosen as it opens (8 × 8); a click on each
/// sets the rows and columns to its size; the arrows, Home and End move
/// the choice and set them too; rows and columns of another size choose
/// none.
#[test]
fn the_area_wizard_s_size_segments_set_rows_and_columns() {
    let Some(mut h) = game_harness("board-area-size") else { return };
    h.set_size(egui::vec2(1280.0, 720.0));
    h.state_mut().actions.push(mg_ui::Action::AreaWizard);
    settle(&mut h);
    // (The wizard's second page.)
    area_wizard_next(&mut h, 1);
    let groups = h.query_all_by(|n| {
        n.role() == Role::RadioGroup && n.label().as_deref() == Some("Area size")
    });
    assert_eq!(groups.count(), 1, "a radio group named Area size");
    assert_eq!(area_size(&h), (8, 8));
    assert_eq!(chosen_segment(&h), ["Medium"]);
    let rects: Vec<egui::Rect> =
        ["Tiny", "Small", "Medium", "Large"].iter().map(|s| h.get_by_label(s).rect()).collect();
    for pair in rects.windows(2) {
        assert!((pair[0].center().y - pair[1].center().y).abs() < 0.5, "in a row: {rects:?}");
        assert!(pair[1].left() > pair[0].right() - 0.5, "side by side: {rects:?}");
    }
    // (The rows × columns and "tiles, 10 m each" under the segments, as on
    // the board: not laid out in the middle of the body's height.)
    let tiles = h.get_by_label("tiles, 10 m each").rect();
    let gap = tiles.top() - rects[0].bottom();
    assert!((0.0..12.0).contains(&gap), "the rows and columns {gap} under them: {tiles:?}");
    for (label, n) in [("Tiny", 2), ("Small", 4), ("Large", 16), ("Medium", 8)] {
        h.get_by_label(label).click();
        settle(&mut h);
        assert_eq!(area_size(&h), (n, n), "{label}");
        assert_eq!(chosen_segment(&h), [label]);
    }
    // The keys: Medium has the focus (clicked last).
    h.get_by_label("Medium").focus();
    settle(&mut h);
    for (key, n, label) in [
        (egui::Key::ArrowRight, 16, "Large"),
        (egui::Key::ArrowLeft, 8, "Medium"),
        (egui::Key::Home, 2, "Tiny"),
        (egui::Key::End, 16, "Large"),
    ] {
        h.key_press(key);
        settle(&mut h);
        assert_eq!(area_size(&h), (n, n), "{key:?}");
        assert_eq!(chosen_segment(&h), [label], "{key:?}");
    }
    set_area_size(&mut h, (5, 7));
    settle(&mut h);
    assert!(chosen_segment(&h).is_empty(), "5 × 7 is none of them");
    h.get_by_label("Small").click();
    settle(&mut h);
    assert_eq!(area_size(&h), (4, 4), "Small from 5 × 7");
}

/// Find and Replace Text with two hits found (M4's U28: in Search
/// Results, the Bottom Dock's): its form across the dock's top, Find what
/// and Replace with side by side at 1280 wide, each its 200 points, and
/// the strings found under the form, inside the dock.
#[test]
fn find_and_replace_shows_its_form_over_its_strings_in_the_bottom_dock() {
    use mg_edit::{Command, Edit};
    let dir = mg_testkit::scratch_dir("ui-board-replace");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let keys: Vec<ResKey> =
        ["camp_a", "camp_b"].iter().map(|n| ResKey::parse(n, ResType::UTW).unwrap()).collect();
    let edits = keys
        .iter()
        .map(|k| {
            let mut g = Gff::new(*b"UTW ");
            let name = LocString::from_text(Language::ENGLISH, Gender::Male, "Orc camp");
            g.root.set("LocalizedName", mg_gff::Value::LocString(name));
            Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    {
        let d = h.state_mut().text_replace.as_mut().expect("Ctrl+H opens it");
        d.find = "orc".into();
        d.with = "Goblin".into();
    }
    settle(&mut h);
    h.get_by_label("Find").click();
    settle(&mut h);
    h.get_by_label("2 strings, 2 times");
    dock_high(&mut h, 340);
    settle(&mut h);
    let dock = h.state().bottom_dock_rect().expect("open");
    let field = |value: &str| {
        h.get_all_by(|n| n.role() == Role::TextInput && n.value().as_deref() == Some(value))
            .next()
            .unwrap_or_else(|| panic!("the field holding {value}"))
            .rect()
    };
    let (find, with) = (field("orc"), field("Goblin"));
    let replace = h.get_by_label("Replace in 2").rect();
    for (what, f) in [("Find what", find), ("Replace with", with)] {
        assert!(dock.contains_rect(f), "{what} {f:?} in the dock {dock:?}");
        assert!(f.width() >= 190.0, "{what} {f:?}: its width");
    }
    assert!((find.top() - with.top()).abs() < 0.5, "side by side: {find:?} {with:?}");
    assert!(with.left() > find.right(), "Replace with after Find what");
    for hit in ["camp_a.utw", "camp_b.utw"] {
        let hit = h
            .get_all_by_label_contains(hit)
            .find(|n| n.accesskit_node().bounding_box().is_some())
            .unwrap()
            .rect();
        assert!(dock.contains_rect(hit), "the hit {hit:?} in the dock {dock:?}");
        assert!(hit.top() >= replace.bottom(), "the hit {hit:?} under the form's {replace:?}");
    }
}

/// Appearance and Portrait at the least window (420 by 320): its body
/// scrolls under the page's bar (the bar stays), and scrolled to its end, All portraits and the portraits'
/// grid's first row stand in the window over the footer; the search field and the
/// grid stay within the window's width.
#[test]
fn the_appearance_page_scrolls_at_the_least_window() {
    let Some(mut h) = game_harness("board-look-least") else { return };
    let size = mg_ui::LEAST_WINDOW_SIZE;
    h.set_size(size);
    h.state_mut().creature_wizard = Some(CreatureWizard {
        page: 3,
        race: Some(6),
        classes: vec![(FIGHTER, 1)],
        ..Default::default()
    });
    settle(&mut h);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    // (The search field, the page's one text field.)
    for find in h.query_all_by(|n| n.role() == Role::TextInput).map(|n| n.rect()) {
        assert!(find.right() <= size.x && find.left() >= 0.0, "a field {find:?}");
    }
    // (Scrolled to its end by the wheel over the page's words, which no
    // well under them takes.)
    let top = h.get_by_label("Appearance and Portrait").rect().top();
    let words = h.get_by_label("Please choose an Appearance and Portrait for this creature.");
    h.hover_at(words.rect().center());
    for _ in 0..10 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -200.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
    }
    settle(&mut h);
    let grid = h.get_all_by_label("Portraits").next().unwrap().rect();
    let all = h.get_by_label("All portraits").rect();
    let next = h.get_by_label("Next >").rect();
    // (The page scrolls under its bar, which stays where it was: not the
    // whole dialog's body, bar and all.)
    let bar = h.get_by_label("Appearance and Portrait").rect();
    assert!((bar.top() - top).abs() < 0.5, "the page's bar {bar:?} moved from {top}");
    assert!(screen.contains_rect(all), "All portraits {all:?} in the window");
    assert!(grid.left() >= 0.0 && grid.right() <= size.x, "the portraits {grid:?}");
    assert!(all.bottom() <= next.top() - 8.0, "All portraits {all:?} over the footer {next:?}");
    // (The grid's node is its whole content, in a well of its own: its top
    // row in sight, over the footer.)
    assert!(
        grid.top() >= all.bottom() && grid.top() + 60.0 <= next.top(),
        "the portraits {grid:?}"
    );
}
