//! The Areas group's height in the Module panel (builders' request of
//! 2026-11-24, amending ruling 10): the handle along the well's foot
//! drags it taller or shorter in whole rows, kept in the settings from run
//! to run; dragged open, or Open Out on the group's menu, the areas open
//! out in the tree, which alone scrolls, as before 5.1; a double click on
//! the handle, or Fit, fits the well again. With modules of empty areas
//! made for the test: no game needed.

use mg_ui::{AreasHeight, TreeAt};

use crate::common::*;
use crate::module_panel::{areas, group, in_well, n_areas, view_bottom, well, well_rows, wheel};

/// The handle's name ([`mg_ui`]'s `tree::WELL_HANDLE`).
const HANDLE: &str = "Resize the Areas Group";

/// A module of `n` areas (`area_000`...) and 40 scripts open in a window
/// `width` by `height`, with `settings` (a run's before, for a restart).
fn many(
    name: &str,
    n: usize,
    (width, height): (f32, f32),
    settings: Option<mg_ui::Settings>,
) -> Harness<'static, Moonglow> {
    let mut app = app_with(Vec::new());
    if let Some(s) = settings {
        app.settings = s;
    }
    app.settings.no_last_area = true;
    app.open_module(&n_areas(name, n));
    let mut h = harness(app, width, height);
    settle(&mut h);
    // (The builder unfolds the Left Dock where the width folds it: M3 T14.)
    unfold_left_dock(&mut h);
    h
}

/// The handle along the well's foot, where it is drawn.
fn handle(h: &Harness<'_, Moonglow>) -> Option<egui::Rect> {
    h.query_by_label(HANDLE).map(|n| n.rect())
}

/// The handle dragged `rows` rows down (up where fewer than none), from
/// a point a little off its line (where it was taken stays under the
/// pointer), by small steps.
fn drag_handle(h: &mut Harness<'_, Moonglow>, rows: f32) {
    let at = handle(h).expect("the handle drawn").center() + egui::vec2(0.0, 1.0);
    drag(h, at, at + egui::vec2(0.0, rows * mg_uikit::metrics::ROW_HEIGHT));
    settle(h);
}

/// The Areas group's menu's Areas Group Height › `choice`.
fn choose(h: &mut Harness<'_, Moonglow>, choice: &str) {
    group(h, "Areas").click_secondary();
    settle(h);
    open_submenu(h, "Areas Group Height");
    popup_item(h, choice).click();
    settle(h);
}

/// The handle dragged down three and a half rows makes the well three
/// rows taller (whole rows: the groups under it pushed down, the tree
/// then scrolling to them), up again to its least, five rows, however far;
/// the height dragged to is the settings', and a window started anew with
/// them (and the settings written and read back) shows the well so.
#[test]
fn dragging_the_well_s_foot_sizes_it_in_whole_rows_kept_after_a_restart() {
    let mut h = many("ui-areas-height-drag", 200, (1000.0, 800.0), None);
    let fit = well_rows(&h);
    assert!(fit > 8.0, "fitted: {fit} rows");
    assert_eq!(h.state().settings.areas_height, AreasHeight::Fit);
    // The handle along the foot, its line across the pane: a tip, the
    // resize cursor.
    let at = handle(&h).expect("the handle drawn");
    assert!((at.center().y - group(&h, "Conversations").rect().top()).abs() < 0.5, "{at:?}");
    let tip = tip_at(&mut h, at);
    assert!(tip.as_deref().is_some_and(|t| t.contains("Drag to resize")), "{tip:?}");
    drag_handle(&mut h, 3.4);
    assert_eq!(well_rows(&h), fit + 3.0, "three rows taller");
    let taller = AreasHeight::Rows((fit + 3.0) as u16);
    assert_eq!(h.state().settings.areas_height, taller);
    // (The other groups pushed down: the last out of sight, reached by
    // the tree's own scroll.)
    let last = group(&h, "Journal and Factions").rect();
    assert!(last.bottom() > view_bottom(&h), "pushed down: {last:?}");
    let at = group(&h, "Conversations").rect().center();
    wheel(&mut h, at, -200.0, 10);
    let last = group(&h, "Journal and Factions").rect();
    assert!(last.bottom() <= view_bottom(&h) + 0.5, "the tree scrolls to it: {last:?}");
    assert_eq!(well_rows(&h), fit + 3.0);
    let at = group(&h, "Conversations").rect().center();
    wheel(&mut h, at, 200.0, 10);
    // Kept: a window started anew with these settings, and with them
    // written and read back.
    let settings = h.state().settings.clone();
    let (read, unread) = mg_ui::Settings::read_fields(&settings.saved_text()).unwrap();
    assert!(unread.is_empty(), "{unread:?}");
    assert_eq!(read.areas_height, taller, "written and read");
    drop(h);
    let mut h = many("ui-areas-height-drag-2", 200, (1000.0, 800.0), Some(read));
    assert_eq!(well_rows(&h), fit + 3.0, "after a restart");
    // Up past its least: five rows.
    drag_handle(&mut h, -18.0);
    assert_eq!(well_rows(&h), 5.0, "no fewer than five rows");
    assert_eq!(h.state().settings.areas_height, AreasHeight::Rows(5));
    // Wheeled, it scrolls in itself, the groups staying.
    let first = areas(&h)[0].to_string();
    let groups = group(&h, "Conversations").rect();
    let at = h.get_by_label(&first).rect().center();
    wheel(&mut h, at, -100.0, 3);
    assert!(!in_well(&h, &first), "the well scrolled");
    assert_eq!(group(&h, "Conversations").rect(), groups);
}

/// A double click on the handle fits the well again; so does the menu's
/// Fit, as its Open Out opens the areas out, and View › Reset Layout.
#[test]
fn a_double_click_on_the_handle_or_fit_fits_the_well_again() {
    let mut h = many("ui-areas-height-fit", 200, (1000.0, 800.0), None);
    let fit = well_rows(&h);
    h.state_mut().settings.areas_height = AreasHeight::Rows(7);
    settle(&mut h);
    assert_eq!(well_rows(&h), 7.0);
    let at = handle(&h).expect("the handle").center();
    double_click_now(&mut h, at);
    settle(&mut h);
    assert_eq!(h.state().settings.areas_height, AreasHeight::Fit);
    assert_eq!(well_rows(&h), fit, "fitted again");
    // The menu: Open Out, then Fit.
    choose(&mut h, "Open Out");
    assert_eq!(h.state().settings.areas_height, AreasHeight::Open);
    choose(&mut h, "Fit");
    assert_eq!(h.state().settings.areas_height, AreasHeight::Fit);
    assert_eq!(well_rows(&h), fit);
    // The height dragged to is on the menu, chosen.
    h.state_mut().settings.areas_height = AreasHeight::Rows(9);
    settle(&mut h);
    group(&h, "Areas").click_secondary();
    settle(&mut h);
    open_submenu(&mut h, "Areas Group Height");
    let dragged = popup_item(&h, "As Dragged (9 Rows)");
    assert_eq!(dragged.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // View › Reset Layout fits it too.
    menu_command(&mut h, "View", "Reset Layout");
    settle(&mut h);
    assert_eq!(h.state().settings.areas_height, AreasHeight::Fit, "Reset Layout");
}

/// Opened out (Open Out on the Areas group's menu), there is no well:
/// every area is a row of the tree, which alone scrolls (one view: the
/// wheel over the areas moves the whole tree, Module Properties too);
/// an area far down brought into view (a tab chosen) is in the tree's
/// view; End over the tree goes to its end, the groups under every area;
/// Up from Conversations goes to the last area, in sight. It stays opened
/// out as areas are added.
#[test]
fn opened_out_the_areas_are_the_tree_s_rows_and_it_alone_scrolls() {
    let mut h = many("ui-areas-height-open", 300, (1000.0, 800.0), None);
    choose(&mut h, "Open Out");
    assert_eq!(h.state().settings.areas_height, AreasHeight::Open);
    let all = areas(&h);
    // The groups under the areas, out of sight; the handle too.
    let conversations = group(&h, "Conversations").rect();
    assert!(conversations.top() > view_bottom(&h), "below every area: {conversations:?}");
    assert!(handle(&h).is_none(), "the handle out of sight under the last area");
    // The wheel over an area's row moves the tree.
    let top = |h: &Harness<'_, Moonglow>| tree_row(h, "Module Properties").rect().top();
    let was = top(&h);
    let at = h.get_by_label(&all[3].to_string()).rect().center();
    wheel(&mut h, at, -200.0, 3);
    assert!(top(&h) < was - 300.0, "the tree scrolled: {} from {was}", top(&h));
    // An area far down brought into view, by the tree's scroll.
    let far = all[250];
    h.state_mut().tree_reveal = Some((ResKey::new(far, ResType::ARE), false));
    settle(&mut h);
    let row = h.get_by_label(&far.to_string()).rect();
    assert!(row.top() >= 0.0 && row.bottom() <= view_bottom(&h) + 0.5, "{row:?}");
    // End over the tree: its end, the groups under the last area.
    h.hover_at(row.center());
    h.run_steps(2);
    h.key_press(egui::Key::End);
    h.run_steps(2);
    settle(&mut h);
    let last = h.get_by_label(&all[all.len() - 1].to_string()).rect();
    let conversations = group(&h, "Conversations").rect();
    assert!(last.bottom() <= conversations.top() + 0.5, "{last:?} {conversations:?}");
    let journal = group(&h, "Journal and Factions").rect();
    assert!(journal.bottom() <= view_bottom(&h) + 0.5, "at the tree's end: {journal:?}");
    // The handle under the last area, over Conversations' row.
    let foot = handle(&h).expect("the handle under the last area");
    assert!((foot.center().y - conversations.top()).abs() < 0.5, "{foot:?}");
    // The keys: Up from Conversations to the last area, in sight.
    group(&h, "Conversations").click();
    settle(&mut h);
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    settle(&mut h);
    let last_key = ResKey::new(all[all.len() - 1], ResType::ARE);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(last_key)));
    let last = h.get_by_label(&all[all.len() - 1].to_string()).rect();
    assert!(last.bottom() <= view_bottom(&h) + 0.5, "{last:?}");
    // A name typed goes to its row, in sight; Home to the tree's top.
    h.event(egui::Event::Text("area_04".into()));
    h.run_steps(2);
    settle(&mut h);
    let typed = ResKey::new(all[40], ResType::ARE);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(typed)), "typed to area_040");
    let row = h.get_by_label(&all[40].to_string()).rect();
    assert!(row.top() >= 0.0 && row.bottom() <= view_bottom(&h) + 0.5, "{row:?}");
    h.key_press(egui::Key::Home);
    h.run_steps(2);
    settle(&mut h);
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Group("Areas")), "Home");
    let areas_row = group(&h, "Areas").rect();
    assert!(areas_row.top() >= 0.0 && top(&h) > was - 300.0, "at the tree's top: {areas_row:?}");
    // Kept opened out, with more areas.
    let settings = h.state().settings.clone();
    drop(h);
    let h = many("ui-areas-height-open-2", 400, (1000.0, 800.0), Some(settings));
    assert_eq!(h.state().settings.areas_height, AreasHeight::Open);
    let conversations = group(&h, "Conversations").rect();
    assert!(conversations.top() > view_bottom(&h), "still opened out: {conversations:?}");
}

/// The handle dragged down to all the areas' rows and past them makes the
/// well as high as they are while it is held, and opens them out once let
/// go; dragged up from under the last area, it is a well again.
#[test]
fn dragged_past_all_its_rows_the_well_opens_out() {
    let mut h = many("ui-areas-height-past", 24, (1000.0, 800.0), None);
    let fit = well_rows(&h);
    assert!(fit < 24.0, "{fit}");
    drag_handle(&mut h, 24.0 - fit + 2.0);
    assert_eq!(h.state().settings.areas_height, AreasHeight::Open);
    let rows = well(&h);
    assert_eq!((rows.end() - rows.start()) / mg_uikit::metrics::ROW_HEIGHT, 24.0, "every area");
    // Up from under the last area: a well of the height dragged to.
    drag_handle(&mut h, -10.0);
    assert_eq!(h.state().settings.areas_height, AreasHeight::Rows(14));
    assert_eq!(well_rows(&h), 14.0);
}

/// No handle where there is nothing to give: the areas fitting as they
/// are (three areas), the Areas group closed.
#[test]
fn no_handle_where_the_areas_fit_or_are_folded() {
    let mut h = many("ui-areas-height-three", 3, (1000.0, 800.0), None);
    assert!(handle(&h).is_none(), "three areas fit");
    h.state_mut().settings.areas_height = AreasHeight::Open;
    settle(&mut h);
    assert!(handle(&h).is_none(), "three areas, opened out");
    let mut h = many("ui-areas-height-folded", 200, (1000.0, 800.0), None);
    assert!(handle(&h).is_some());
    group(&h, "Areas").click();
    settle(&mut h);
    assert!(handle(&h).is_none(), "the group closed");
}

/// While the Filter has something typed, the well of a height dragged to
/// shows what it finds in no more rows than that, and dragged past them
/// it is not opened out (it shows only some areas); opened out, what it
/// finds is laid out in the tree.
#[test]
fn the_filter_keeps_the_well_s_height_and_does_not_open_it_out() {
    let mut h = many("ui-areas-height-filter", 200, (1000.0, 800.0), None);
    h.state_mut().settings.areas_height = AreasHeight::Rows(6);
    settle(&mut h);
    // (area_190 to area_199: ten.)
    type_into_tree_filter(&mut h, "area_19");
    settle(&mut h);
    assert_eq!(well_rows(&h), 6.0, "six rows of what it finds");
    drag_handle(&mut h, 8.0);
    assert_eq!(well_rows(&h), 10.0, "as high as what it finds");
    assert!(
        matches!(h.state().settings.areas_height, AreasHeight::Rows(_)),
        "not opened out: {:?}",
        h.state().settings.areas_height
    );
    h.state_mut().settings.areas_height = AreasHeight::Open;
    settle(&mut h);
    assert!(handle(&h).is_none(), "ten rows fit");
    let last = h.get_by_label("Module Tree Rows").get_by_label("area_199").rect();
    assert_eq!(last.bottom(), group(&h, "Conversations").rect().top(), "laid out in the tree");
}

/// A well dragged taller than the tree's view (30 rows in an 800 px
/// window): Up from Conversations goes to the last area, in sight (the
/// well scrolled to it, the tree to where the row then is, not only to the
/// well's top); an area far down brought into view is in sight; wheeled
/// over the well past its last area, the tree goes on to the groups
/// under it.
#[test]
fn a_well_taller_than_the_tree_s_view_brings_its_rows_into_sight() {
    let mut h = many("ui-areas-height-tall", 200, (1000.0, 800.0), None);
    h.state_mut().settings.areas_height = AreasHeight::Rows(30);
    settle(&mut h);
    assert_eq!(well_rows(&h), 30.0);
    let all = areas(&h);
    let in_sight = |h: &Harness<'_, Moonglow>, label: &str| {
        let r = h.get_by_label("Module Tree Rows").get_by_label(label).rect();
        let filter = h.get(egui_kittest::kittest::by().predicate(tree_filter)).rect();
        r.top() >= filter.bottom() - 0.5 && r.bottom() <= view_bottom(h) + 0.5 && in_well(h, label)
    };
    // (Conversations' row in sight first: the tree wheeled down to it.)
    let at = group(&h, "Areas").rect().center() + egui::vec2(0.0, 200.0);
    wheel(&mut h, at, -400.0, 40);
    let conversations = group(&h, "Conversations").rect();
    assert!(conversations.bottom() <= view_bottom(&h) + 0.5, "wheeled on: {conversations:?}");
    group(&h, "Conversations").click();
    settle(&mut h);
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    settle(&mut h);
    let last = all[all.len() - 1];
    assert_eq!(h.state().tree_cursor, Some(TreeAt::Resource(ResKey::new(last, ResType::ARE))));
    assert!(in_sight(&h, &last.to_string()), "the last area in sight");
    // An area far up the well brought into view (a tab chosen).
    let far = all[40];
    h.state_mut().tree_reveal = Some((ResKey::new(far, ResType::ARE), false));
    settle(&mut h);
    assert!(in_sight(&h, &far.to_string()), "{far} in sight");
}

/// The handle held while its foot goes out of the tree's view (dragged
/// past the Status Bar) or the well comes to all its rows (fewer areas
/// than the fitted well would show) is still the builder's: the height
/// dragged to taken, and opened out once let go at all its rows.
#[test]
fn the_handle_stays_held_out_of_sight_and_at_all_its_rows() {
    let mut h = many("ui-areas-height-held", 200, (1000.0, 800.0), None);
    let fit = well_rows(&h);
    drag_handle(&mut h, 25.0);
    assert_eq!(h.state().settings.areas_height, AreasHeight::Rows((fit + 25.0) as u16));
    assert_eq!(well_rows(&h), fit + 25.0);
    // Taken again, the tree wheeled down to it.
    let at = group(&h, "Areas").rect().center() + egui::vec2(0.0, 100.0);
    wheel(&mut h, at, -400.0, 20);
    drag_handle(&mut h, -10.0);
    assert_eq!(well_rows(&h), fit + 15.0, "taken again");
    // Twelve areas, the well six rows where fitted it shows them all:
    // dragged down to all twelve and let go, opened out.
    let mut h = many("ui-areas-height-twelve", 12, (1000.0, 800.0), None);
    h.state_mut().settings.areas_height = AreasHeight::Rows(6);
    settle(&mut h);
    assert_eq!(well_rows(&h), 6.0);
    drag_handle(&mut h, 8.0);
    assert_eq!(h.state().settings.areas_height, AreasHeight::Open);
    assert!(handle(&h).is_none(), "twelve rows fit");
}

/// The Module panel's settled frame with 2,000 areas (a builder has
/// 1,850) fitted, dragged to 30 rows and opened out, at the top and with
/// area_1000 brought into view: the median of 60 frames each, printed
/// (`--ignored --nocapture`). Opened out costs no more than a well: rows
/// out of sight take their room only, as in the well.
#[test]
#[ignore]
fn frame_times_with_two_thousand_areas() {
    let mut h = many("ui-areas-height-perf", 2000, (1280.0, 900.0), None);
    let mut medians = Vec::new();
    for height in [AreasHeight::Fit, AreasHeight::Rows(30), AreasHeight::Open] {
        for middle in [false, true] {
            h.state_mut().settings.areas_height = height;
            h.state_mut().tree_cursor = None;
            let to = if middle { 1000 } else { 0 };
            let key = ResKey::new(areas(&h)[to], ResType::ARE);
            h.state_mut().tree_reveal = Some((key, false));
            settle(&mut h);
            h.hover_at(egui::pos2(1000.0, 500.0));
            settle(&mut h);
            // (The row brought into view is drawn, in sight.)
            let label = key.resref.to_string();
            let row = h.get_by_label("Module Tree Rows").get_by_label(&label);
            assert!(row.rect().bottom() <= view_bottom(&h) + 0.5, "{height:?} {label}");
            let mut times: Vec<f64> = (0..60)
                .map(|_| {
                    let t = std::time::Instant::now();
                    h.step();
                    t.elapsed().as_secs_f64() * 1000.0
                })
                .collect();
            times.sort_by(f64::total_cmp);
            let median = times[30];
            println!("{height:?}, {}: {median:.2} ms", if middle { "middle" } else { "top" });
            medians.push((height, median));
        }
    }
    let fit = medians[0].1.max(medians[1].1);
    let open = medians[4].1.max(medians[5].1);
    assert!(open < fit * 1.5 + 1.0, "opened out {open:.2} ms, fitted {fit:.2} ms");
}
