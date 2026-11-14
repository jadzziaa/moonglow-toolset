//! How the shell yields at narrow widths, and the wide layout (the
//! redesign's M3, T14; proposal sections 5.2 and 5.3): what the window
//! shows at 1280, 1399, 1499, 1600 and 3440, by name; every width from the
//! least window to 3440 whole, the editor area never under its least, in
//! both modes of the Open editors switch; a window narrowed and widened
//! again writing nothing the builder chose and giving it all back; a dock
//! folded by the width unfolded by hand, by F6 and by View, and how long
//! it stays so; the sizes older versions kept, taken once.

use crate::common::*;
use crate::shell::{bar_controls, bars_wrong};
use mg_uikit::shell as kit;

/// The margin kittest's harness keeps round the window's contents (its
/// `outer_margin`): the application's frame starts 8 points in.
const HARNESS_MARGIN: f32 = 8.0;

/// Frames until `done`, for twenty seconds at most (an area's view asks
/// for frames as it draws, so `settle` may not end beside one).
fn until(h: &mut Harness<'_, Moonglow>, what: &str, done: impl Fn(&Harness<'_, Moonglow>) -> bool) {
    let started = std::time::Instant::now();
    while !done(h) {
        assert!(started.elapsed().as_secs() < 20, "never: {what}");
        h.step();
    }
}

/// What the shell's layout is at this frame, to wait for it to rest: the
/// bars' and the docks' controls and where the editor area and the docks
/// are.
fn layout(h: &Harness<'_, Moonglow>) -> String {
    let s = h.state();
    let bars: Vec<_> = ["Top Bar", "Status Bar", "Left Dock", mg_ui::RIGHT_DOCK]
        .iter()
        .map(|b| bar_controls(h, b))
        .collect();
    format!("{bars:?} {:?} {:?}", s.editor_area_rect(), s.right_dock_rect())
}

/// Frames until the layout is the same three frames running (a bar's
/// right-hand group and a dock's width settle over a frame or two), with
/// a deadline.
fn at_rest(h: &mut Harness<'_, Moonglow>) {
    let started = std::time::Instant::now();
    let (mut was, mut same) = (String::new(), 0);
    while same < 3 {
        assert!(started.elapsed().as_secs() < 20, "the window never rests");
        h.step();
        let now = layout(h);
        same = if now == was { same + 1 } else { 0 };
        was = now;
    }
}

/// The window `width` by `height`, at rest.
fn size(h: &mut Harness<'_, Moonglow>, width: f32, height: f32) {
    h.set_size(egui::vec2(width, height));
    at_rest(h);
}

/// [`area_harness`]'s window (an area open in front, the game's data, the
/// log open) with the docks folding by the window's width as the
/// application's do, and the Open editors switch as `mode` has it.
fn yielding(name: &str, mode: Mode) -> Option<(Harness<'static, Moonglow>, mg_core::ResRef)> {
    let (mut h, area) = area_harness(name)?;
    h.state_mut().docks_fold_by_width = true;
    h.state_mut().settings.open_editors = match mode {
        Mode::Floating => mg_ui::OpenEditors::Floating,
        Mode::Tabs => mg_ui::OpenEditors::Tabs,
    };
    until(&mut h, "the area in front", |h| h.state().active_editor() == Some(&Tab::Area(area)));
    at_rest(&mut h);
    Some((h, area))
}

/// Quick Open as the Top Bar shows it: `"field"` (its text field) or
/// `"icon"` (its icon button), by its name; `"none"`.
fn quick_open_as(h: &Harness<'_, Moonglow>) -> &'static str {
    use egui::accesskit::Role;
    let bar = h.get_by_label("Top Bar");
    let roles: Vec<Role> = (bar.query_all_by_label(mg_ui::quick_open::NAME))
        .map(|n| n.accesskit_node().role())
        .collect();
    match roles.as_slice() {
        [Role::Button] => "icon",
        [] => "none",
        _ => "field",
    }
}

/// The Left Dock as shown: `"strip"` (folded: the strip's three panel
/// buttons by name, the button beside it that unfolds it, no tabs) or
/// `"open"` (its tabs, the button that folds it).
fn left_as(h: &Harness<'_, Moonglow>) -> &'static str {
    use egui::accesskit::Role;
    let dock = h.get_by_label("Left Dock");
    let tabs = dock.query_all_by(|n| n.role() == Role::Tab).count();
    if h.query_by_label("Show the Left Dock").is_some() {
        assert_eq!(tabs, 0, "a strip has no tabs");
        for panel in ["Module", "Resources", "Outliner"] {
            dock.get_by_label(panel);
        }
        "strip"
    } else {
        assert!(tabs >= 2, "the Module and Resources tabs");
        h.get_by_label("Fold the Left Dock");
        "open"
    }
}

/// The Right Dock as shown: `"strip"`, `"columns"` (the Palette's tab bar
/// and the Preview's side by side, each drawing its panel) or `"tabs"`
/// (the Palette and the Preview as tabs of one bar).
fn right_as(h: &Harness<'_, Moonglow>) -> &'static str {
    use egui::accesskit::Role;
    if h.query_by_label("Show the Right Dock").is_some() {
        h.get_by_label(mg_ui::RIGHT_DOCK).get_by_label("Palette");
        return "strip";
    }
    let tabs = |name: &str| -> Vec<String> {
        h.query_by_label(name).map_or(Vec::new(), |bar| {
            (bar.query_all_by(|n| n.role() == Role::Tab))
                .map(|n| n.accesskit_node().label().unwrap_or_default())
                .collect()
        })
    };
    match (tabs(mg_ui::RIGHT_DOCK).as_slice(), tabs(mg_ui::RIGHT_COLUMN_TWO).as_slice()) {
        ([one], [two]) => {
            assert_eq!((one.as_str(), two.as_str()), ("Palette", "Preview"));
            "columns"
        }
        ([a, b], []) => {
            assert_eq!((a.as_str(), b.as_str()), ("Palette", "Preview"));
            "tabs"
        }
        other => panic!("the Right Dock's tabs: {other:?}"),
    }
}

/// Whether the Status Bar's Log switch shows its icon alone (its words
/// gone: as narrow as an icon button and its padding).
fn switches_as_icons(h: &Harness<'_, Moonglow>) -> bool {
    let switch = h.get_by_label("Status Bar").get_by_label("Log").rect();
    switch.width() < 30.0
}

/// Whether the Status Bar shows the game folder (`…/` and its name).
fn folder_shown(h: &Harness<'_, Moonglow>) -> bool {
    let root = h.state().install.as_ref().unwrap().root.clone();
    let name = format!("…/{}", root.file_name().unwrap().to_string_lossy());
    h.get_by_label("Status Bar").query_by_label(&name).is_some()
}

/// Whether Compile on the Top Bar shows its word (wider than its icon).
fn build_words(h: &Harness<'_, Moonglow>) -> bool {
    h.get_by_label("Top Bar").get_by_label("Compile").rect().width() > 40.0
}

/// What the window shows at the widths section 5.2 measures and at the
/// wide boards', by name, in both modes: at 1280 (the Narrow board) Quick
/// Open's field, the build actions as icons, the Left Dock folded to its
/// strip of three, the Right Dock 280 with Palette and Preview as tabs,
/// the Status Bar's switches as icons and no game folder; at 1399 the
/// build actions' words back; at 1499 the Right Dock 320, the switches'
/// words and the folder back, the Left Dock still a strip; at 1600 (Main)
/// the Left Dock open, 260; at 3440 by 1440 (WideArea) the Left Dock 320,
/// the Right Dock 741 in two columns (Palette | Preview, each drawing its
/// panel), the Module panel 760 high over the Outliner.
#[test]
fn what_the_shell_shows_at_each_width() {
    for mode in MODES {
        let Some((mut h, _)) = yielding("yields-shown", mode) else { return };
        let at = |h: &mut Harness<'_, Moonglow>, width: f32, height: f32| {
            size(h, width, height);
            let right = h.state().right_dock_rect().map(|r| r.width());
            let shown = (
                quick_open_as(h),
                build_words(h),
                left_as(h),
                right_as(h),
                switches_as_icons(h),
                folder_shown(h),
            );
            (shown, right)
        };
        let (shown, right) = at(&mut h, 1280.0, 800.0);
        assert_eq!(shown, ("field", false, "strip", "tabs", true, false), "{mode:?} 1280");
        assert_eq!(right, Some(kit::RIGHT_DOCK_NARROW), "{mode:?} 1280");
        let editor = h.state().editor_area_rect().unwrap();
        // (The board's 974: the strip and the narrow dock, with their
        // lines, beside an editor area flush with them and the Top Bar;
        // here in the harness's margin, 16 less.)
        let want = 974.0 - 2.0 * HARNESS_MARGIN;
        assert!((editor.width() - want).abs() <= 1.0, "{mode:?} the editor at 1280: {editor:?}");
        let dock = h.state().right_dock_rect().unwrap();
        assert!((dock.left() - editor.right()).abs() <= 0.5, "{mode:?} flush: {editor:?} {dock:?}");
        let top = HARNESS_MARGIN + mg_uikit::metrics::TOP_BAR + 1.0;
        assert!((editor.top() - top).abs() <= 0.5, "{mode:?} under the Top Bar: {editor:?}");
        let (shown, right) = at(&mut h, 1399.0, 800.0);
        assert_eq!(shown, ("field", true, "strip", "tabs", true, false), "{mode:?} 1399");
        assert_eq!(right, Some(kit::RIGHT_DOCK_NARROW), "{mode:?} 1399");
        let (shown, right) = at(&mut h, 1499.0, 800.0);
        assert_eq!(shown, ("field", true, "strip", "tabs", false, true), "{mode:?} 1499");
        assert_eq!(right, Some(mg_uikit::metrics::RIGHT_DOCK), "{mode:?} 1499");
        let (shown, right) = at(&mut h, 1600.0, 900.0);
        assert_eq!(shown, ("field", true, "open", "tabs", false, true), "{mode:?} 1600");
        assert_eq!(right, Some(mg_uikit::metrics::RIGHT_DOCK), "{mode:?} 1600");
        let tabs = h.get_by_label("Left Dock").get_by_label("Module").rect();
        let left = h.state().editor_area_rect().unwrap().left() - HARNESS_MARGIN;
        assert!((left - mg_uikit::metrics::LEFT_DOCK).abs() <= 1.0, "{mode:?} 260: {left}");
        assert!(tabs.left() < 10.0);
        let (shown, right) = at(&mut h, 3440.0, 1440.0);
        assert_eq!(shown, ("field", true, "open", "columns", false, true), "{mode:?} 3440");
        assert_eq!(right, Some(kit::RIGHT_DOCK_WIDE), "{mode:?} 3440");
        let left = h.state().editor_area_rect().unwrap().left() - HARNESS_MARGIN;
        assert!((left - kit::LEFT_DOCK_WIDE).abs() <= 1.0, "{mode:?} 320: {left}");
        // Each column its panel: the palette's types in the first, the
        // Preview's body in the second, the board's 340 and 400 apart by
        // a line.
        let dock = h.state().right_dock_rect().unwrap();
        // (The palette's Expand All, at the right of its list's toolbar.)
        let types = (h.get_all_by_label("Expand All"))
            .map(|n| n.rect())
            .find(|r| dock.contains(r.center()))
            .expect("the palette's Expand All in the dock");
        let preview = h.state().preview_panel_rect().expect("the Preview drawn in its column");
        assert!(types.right() <= dock.left() + kit::RIGHT_COLUMN + 0.5, "{types:?} {dock:?}");
        assert!(preview.left() >= dock.left() + kit::RIGHT_COLUMN, "{preview:?} {dock:?}");
        assert!(preview.right() <= dock.right() + 0.5 && preview.width() > 300.0, "{preview:?}");
        // The Module panel 760 high over the Outliner (GalleryWide's):
        // from the middle of its tab bar to the middle of the Outliner's
        // header, each 26 high.
        let module = h.get_by_label("Left Dock").get_by_label("Module").rect().center().y;
        let outliner = h.get_by_label(mg_ui::OUTLINER).get_by_label("Collapse All").rect();
        let seam = outliner.center().y - module;
        assert!((seam - kit::MODULE_PANEL_WIDE).abs() <= 2.0, "{mode:?} the seam at {seam}");
    }
}

/// Every width from the least window's 420 to 3440, in 20 point steps,
/// in both modes, the area in front (and Module Properties opened, docked
/// behind it or in a window of its own): no two of the window's named
/// widgets over each other, no control of the bars or the Left Dock
/// outside the window or over another, the Right Dock inside the window
/// and at least its narrow 280 where it is open, the Left Dock at least
/// its least where it is open, and the editor area never under its least
/// (`EDITOR_LEAST`, 490), or, in a window too narrow for that beside the
/// two docks' strips, the whole room they leave it.
#[test]
fn every_width_is_whole_and_the_editor_keeps_its_least() {
    for mode in MODES {
        let Some((mut h, area)) = yielding("yields-sweep", mode) else { return };
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
        until(&mut h, "Module Properties", |h| {
            h.state().dock.find_tab(&Tab::ModuleProperties).is_some()
        });
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(area)));
        until(&mut h, "the area in front", |h| h.state().active_editor() == Some(&Tab::Area(area)));
        let mut wrong = Vec::new();
        let mut strips = (0, 0);
        for width in (420..=3440).step_by(20) {
            let width = width as f32;
            size(&mut h, width, 800.0);
            let at = |what: String| format!("{mode:?} {width}: {what}");
            let window = h.ctx.content_rect();
            // (An Outliner kind's eye lies in its row, by design: the
            // kit's `TreeGroup::eye`.)
            let eye_in_row = |found: &str| {
                found.split_once(" over Button \"Show ").is_some_and(|(row, kind)| {
                    let kind = kind.trim_end_matches('"').to_lowercase();
                    row.trim_end_matches('"').ends_with(&format!(" {kind}"))
                })
            };
            for found in overlapping(&h) {
                if !eye_in_row(&found) {
                    wrong.push(at(found));
                }
            }
            for found in bars_wrong(&h, true) {
                wrong.push(at(found));
            }
            let editor = h.state().editor_area_rect().unwrap();
            let strips_room = window.width() - 2.0 * (kit::STRIP + 1.0);
            let least = kit::EDITOR_LEAST.min(strips_room);
            // (The harness's margin off the window: the thresholds are the
            // window's, 16 points wider than the room the shell has here.)
            if editor.width() + 2.0 * HARNESS_MARGIN < least - 1.0 {
                wrong.push(at(format!("the editor area {} under {least}", editor.width())));
            }
            match h.state().right_dock_rect() {
                Some(dock) => {
                    if !window.expand(0.5).contains_rect(dock) {
                        wrong.push(at(format!("the Right Dock {dock:?} outside")));
                    }
                    if dock.width() < kit::RIGHT_DOCK_NARROW - 0.5 {
                        wrong.push(at(format!("the Right Dock {} wide", dock.width())));
                    }
                }
                None => strips.1 += 1,
            }
            if left_as(&h) == "open" {
                // (The Left Dock from the window's left to the editor
                // area.)
                let dock = editor.left() - HARNESS_MARGIN;
                if dock < 190.0 - 0.5 {
                    wrong.push(at(format!("the Left Dock {dock} wide")));
                }
            } else {
                strips.0 += 1;
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
        // (Folded by the width: the Left Dock under 1500, the Right Dock
        // under 796.)
        assert_eq!(strips, (54, 19), "{mode:?}: the widths with each dock a strip");
    }
}

/// The settings the builder chose (the docks' widths, the seam over the
/// Outliner, each dock's fold) through a window narrowed from 3440 to the
/// least and widened again: none of it written at any width, and at 3440
/// again each dock as wide as chosen; and with nothing chosen, the wide
/// layout's widths (320, 741, the seam 760) at 3440, Main's at 1600, and
/// nothing stored by any width.
#[test]
fn a_window_narrowed_and_widened_writes_nothing_and_gives_all_back() {
    let Some((mut h, _)) = yielding("yields-chosen", Mode::Tabs) else { return };
    let sweep = |h: &mut Harness<'_, Moonglow>, saved: &str| {
        let widths = (420..=3440).rev().step_by(60).chain((420..=3440).step_by(60));
        for width in widths {
            size(h, width as f32, 1000.0);
            assert_eq!(h.state().settings.saved_text(), saved, "written at {width}");
        }
        size(h, 3440.0, 1000.0);
    };
    for folded in [false, true] {
        {
            let s = &mut h.state_mut().settings;
            (s.left_dock_width, s.right_dock_width, s.module_panel_height) =
                (Some(300), Some(420), Some(400));
            (s.hide_tree, s.hide_palettes) = (folded, folded);
        }
        size(&mut h, 3440.0, 1000.0);
        let saved = h.state().settings.saved_text();
        sweep(&mut h, &saved);
        if folded {
            assert_eq!((left_as(&h), right_as(&h)), ("strip", "strip"), "folded as chosen");
        } else {
            assert_eq!(h.state().right_dock_rect().map(|r| r.width()), Some(420.0));
            let left = h.state().editor_area_rect().unwrap().left() - HARNESS_MARGIN;
            assert!((left - 300.0).abs() <= 1.0, "the Left Dock as chosen: {left}");
        }
    }
    // Nothing chosen: the widths each layout gives, nothing stored.
    {
        let s = &mut h.state_mut().settings;
        (s.left_dock_width, s.right_dock_width, s.module_panel_height) = (None, None, None);
        (s.hide_tree, s.hide_palettes) = (false, false);
    }
    size(&mut h, 3440.0, 1440.0);
    let saved = h.state().settings.saved_text();
    sweep(&mut h, &saved);
    size(&mut h, 3440.0, 1440.0);
    assert_eq!(h.state().right_dock_rect().map(|r| r.width()), Some(kit::RIGHT_DOCK_WIDE));
    let left = h.state().editor_area_rect().unwrap().left() - HARNESS_MARGIN;
    assert!((left - kit::LEFT_DOCK_WIDE).abs() <= 1.0, "320: {left}");
    size(&mut h, 1600.0, 900.0);
    assert_eq!(h.state().right_dock_rect().map(|r| r.width()), Some(mg_uikit::metrics::RIGHT_DOCK));
    let left = h.state().editor_area_rect().unwrap().left() - HARNESS_MARGIN;
    assert!((left - mg_uikit::metrics::LEFT_DOCK).abs() <= 1.0, "260: {left}");
    let s = &h.state().settings;
    assert_eq!((s.left_dock_width, s.right_dock_width, s.module_panel_height), (None, None, None));
}

/// The tip of a dock's edge button (shown under the pointer).
fn edge_tip(h: &mut Harness<'_, Moonglow>, name: &str) -> String {
    let rect = h.get_by_label(name).rect();
    tip_at(h, rect).unwrap_or_default()
}

/// The Left Dock folded by a 1280 window (the builder's own fold off):
/// the button beside its strip says it shows the dock this narrow window
/// folds; clicked, the dock unfolds beside the editor area (pushing it,
/// as at any width), `hide_tree` untouched, the button now saying it
/// folds it again; it stays unfolded as the window widens to 1450 and
/// narrows to 1300; from 1500 the builder's own fold has it (unfolded);
/// narrowed to 1280 again, it is folded again. With the builder's own
/// fold on, unfolding it at 1280 leaves `hide_tree` on, and at 1600 it is
/// folded, as the builder left it.
#[test]
fn a_dock_folded_by_the_width_unfolds_until_the_window_is_wide_again() {
    let Some((mut h, _)) = yielding("yields-unfold", Mode::Tabs) else { return };
    size(&mut h, 1280.0, 800.0);
    assert_eq!(left_as(&h), "strip");
    let tip = edge_tip(&mut h, "Show the Left Dock");
    assert!(tip.starts_with("Show the Left Dock, which this narrow window folds"), "{tip}");
    h.get_by_label("Show the Left Dock").click();
    at_rest(&mut h);
    assert_eq!(left_as(&h), "open");
    assert!(!h.state().settings.hide_tree);
    let editor = h.state().editor_area_rect().unwrap();
    assert!(editor.left() >= mg_uikit::metrics::LEFT_DOCK - 1.0, "pushed: {editor:?}");
    let tip = edge_tip(&mut h, "Fold the Left Dock");
    assert!(tip.starts_with("Fold the Left Dock again, as this narrow window has it"), "{tip}");
    for width in [1450.0, 1300.0] {
        size(&mut h, width, 800.0);
        assert_eq!(left_as(&h), "open", "still unfolded at {width}");
    }
    size(&mut h, 1600.0, 800.0);
    assert_eq!(left_as(&h), "open");
    size(&mut h, 1280.0, 800.0);
    assert_eq!(left_as(&h), "strip", "folded again by the narrow window");
    // The builder's own fold on: unfolded by hand at 1280, the setting
    // kept; at 1600, folded as the builder left it.
    h.state_mut().settings.hide_tree = true;
    at_rest(&mut h);
    h.get_by_label("Show the Left Dock").click();
    at_rest(&mut h);
    assert_eq!(left_as(&h), "open");
    assert!(h.state().settings.hide_tree, "the builder's fold kept");
    size(&mut h, 1600.0, 800.0);
    assert_eq!(left_as(&h), "strip", "folded as the builder left it");
    let tip = edge_tip(&mut h, "Show the Left Dock");
    assert!(!tip.contains("narrow"), "the builder's own fold: {tip}");
}

/// In a 900 window (the Right Dock open, the Left Dock folded by the
/// width) both docks do not fit beside the editor area's least: the Left
/// Dock unfolded by View › Left Dock folds the Right Dock to its strip,
/// whose button says it shows the Right Dock in place of the Left Dock;
/// that button unfolds the Right Dock and folds the Left Dock. Neither
/// fold is written. F6 in a 1280 window, the keys in the editor area,
/// stops at the Left Dock folded by the width and unfolds it, the keys
/// its panel's.
#[test]
fn both_docks_where_they_do_not_fit_and_f6() {
    let Some((mut h, _)) = yielding("yields-both", Mode::Tabs) else { return };
    size(&mut h, 900.0, 800.0);
    assert_eq!((left_as(&h), right_as(&h)), ("strip", "tabs"));
    menu_command(&mut h, "View", "Left Dock");
    at_rest(&mut h);
    assert_eq!((left_as(&h), right_as(&h)), ("open", "strip"));
    let tip = edge_tip(&mut h, "Show the Right Dock");
    assert!(tip.starts_with("Show the Right Dock in place of the Left Dock"), "{tip}");
    h.get_by_label("Show the Right Dock").click();
    at_rest(&mut h);
    assert_eq!((left_as(&h), right_as(&h)), ("strip", "tabs"));
    let s = &h.state().settings;
    assert!(!s.hide_tree && !s.hide_palettes, "nothing written");
    // F6 at 1280: the editor area's, then the Left Dock's, unfolded. (By
    // way of 1600, where the window no longer folds it: unfolded by hand
    // under 1500, it stays so until the window is that wide.)
    size(&mut h, 1600.0, 800.0);
    size(&mut h, 1280.0, 800.0);
    assert_eq!(left_as(&h), "strip");
    // (The pointer over the Top Bar: the area's view takes the keys back
    // while the pointer is in it.)
    let view = h.state().editor_area_rect().unwrap().center();
    h.hover_at(egui::pos2(view.x, 20.0));
    h.key_press(egui::Key::F6);
    at_rest(&mut h);
    // (F6 goes from the editor area to the Right Dock, then the log, then
    // round to the Left Dock and the Outliner: once round at most.)
    for _ in 0..5 {
        if left_as(&h) == "open" {
            break;
        }
        h.key_press(egui::Key::F6);
        at_rest(&mut h);
    }
    assert_eq!(left_as(&h), "open", "F6 unfolds the Left Dock folded by the width");
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Tree", "the keys its panel's");
    assert!(!h.state().settings.hide_tree);
}

/// The sizes older versions kept, taken once: 4.2.0's module tree width
/// kept by egui (its 240 at first: not chosen; a width dragged to, 300:
/// chosen), this branch's sizes stored before anything was chosen (260,
/// 320, 180: not chosen), a width dragged to and stored (410: chosen);
/// and taken only once, so that a width a narrow window drew is never
/// taken.
#[test]
fn sizes_kept_by_older_versions_are_taken_once() {
    let Some((mut h, _)) = yielding("yields-taken", Mode::Tabs) else { return };
    let take = |h: &mut Harness<'_, Moonglow>, kept: Option<u32>, egui_width: Option<f32>| {
        h.state_mut().settings.left_dock_width = kept;
        h.state_mut().settings.dock_sizes_taken = false;
        if let Some(width) = egui_width {
            let rect = egui::Rect::from_min_size(egui::pos2(0.0, 30.0), egui::vec2(width, 600.0));
            let state = egui::containers::panel::PanelState { outer_rect: rect };
            h.ctx.data_mut(|d| d.insert_persisted(egui::Id::new("tree"), state));
        }
        // (One frame: they are taken as it begins.)
        h.step();
        assert!(h.state().settings.dock_sizes_taken, "marked taken");
        h.state().settings.left_dock_width
    };
    size(&mut h, 1600.0, 900.0);
    assert_eq!(take(&mut h, None, Some(240.0)), None, "4.2.0's width at first");
    assert_eq!(take(&mut h, None, Some(300.0)), Some(300), "4.2.0's width dragged to");
    assert_eq!(take(&mut h, Some(260), None), None, "this branch's width at first");
    assert_eq!(take(&mut h, Some(240), None), None, "4.2.0's taken by this branch");
    assert_eq!(take(&mut h, Some(410), None), Some(410), "a width dragged to");
    // Once: a narrow window's drawn width, kept by egui, is not taken.
    h.state_mut().settings.left_dock_width = None;
    size(&mut h, 1280.0, 800.0);
    h.get_by_label("Show the Left Dock").click();
    size(&mut h, 700.0, 800.0);
    size(&mut h, 1600.0, 900.0);
    assert_eq!(h.state().settings.left_dock_width, None, "nothing taken from what was drawn");
    let left = h.state().editor_area_rect().unwrap().left() - HARNESS_MARGIN;
    assert!((left - mg_uikit::metrics::LEFT_DOCK).abs() <= 1.0, "260 again: {left}");
}

/// The palette's types in the Right Dock as wide as the window has it: a
/// row of thirteen icons in Main's 320 (1600 wide), a dropdown with the
/// type's name in the Narrow board's 280 (1280 wide: under 300 points of
/// panel, `palette_view::TYPE_COMBO_UNDER`), which chooses a type as the
/// row does; the row again at 1600.
#[test]
fn the_palette_s_types_are_a_dropdown_in_the_narrow_dock() {
    let Some((mut h, _)) = yielding("yields-palette-types", Mode::Tabs) else { return };
    let dropdown = |h: &Harness<'_, Moonglow>| -> Option<String> {
        let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
        (h.query_all(combo))
            .filter_map(|n| n.accesskit_node().value())
            .find(|v| PALETTE_TYPES.contains(&v.as_str()))
    };
    let row = |h: &Harness<'_, Moonglow>| {
        h.query_by_label("Palette types").map_or(0, |g| {
            g.query_all(egui_kittest::kittest::by().role(egui::accesskit::Role::Button)).count()
        })
    };
    size(&mut h, 1600.0, 900.0);
    assert_eq!((row(&h), dropdown(&h)), (13, None), "the row at 320");
    size(&mut h, 1280.0, 800.0);
    assert_eq!(row(&h), 0, "no row at 280");
    let shown = dropdown(&h).expect("the dropdown at 280");
    let other = if shown == "Placeables" { "Doors" } else { "Placeables" };
    palette_type(&mut h, other);
    at_rest(&mut h);
    assert_eq!(dropdown(&h).as_deref(), Some(other), "chosen in the dropdown");
    size(&mut h, 1600.0, 900.0);
    assert_eq!((row(&h), dropdown(&h)), (13, None), "the row again");
}

/// Nothing takes the keys away as the window is resized: the arrow keys
/// given to the module tree by F6 at 1600 are still the tree's with the
/// dock folded by a 1280 window, and the tree's as the window widens
/// again; nor does a resize move them to the dock folded by it.
#[test]
fn a_resize_leaves_the_keys_where_they_are() {
    let Some((mut h, _)) = yielding("yields-keys", Mode::Tabs) else { return };
    size(&mut h, 1600.0, 900.0);
    let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
    // (The pointer over the Top Bar: the area's view takes the keys back
    // while the pointer is in it.)
    h.hover_at(egui::pos2(800.0, 20.0));
    for _ in 0..5 {
        if keys(&h) == "Tree" {
            break;
        }
        h.key_press(egui::Key::F6);
        at_rest(&mut h);
    }
    assert_eq!(keys(&h), "Tree");
    size(&mut h, 1280.0, 800.0);
    assert_eq!(left_as(&h), "strip");
    assert_eq!(keys(&h), "Tree", "kept with the dock folded by the width");
    size(&mut h, 1600.0, 900.0);
    assert_eq!(left_as(&h), "open");
    assert_eq!(keys(&h), "Tree", "kept as the window widens");
    // The keys elsewhere (the Right Dock's palette), the Left Dock folded
    // and unfolded by the width: they stay there.
    for _ in 0..6 {
        if keys(&h) == "Palette" {
            break;
        }
        h.key_press(egui::Key::F6);
        at_rest(&mut h);
    }
    assert_eq!(keys(&h), "Palette");
    size(&mut h, 1280.0, 800.0);
    size(&mut h, 1600.0, 900.0);
    assert_eq!(keys(&h), "Palette");
}

/// The wide layout's two columns with a blueprint chosen (3440 by 1440):
/// the Palette's list in the first and, beside it at once, the chest in
/// the Preview's column (its ResRef there, its model's view made: no tab
/// to bring to the front); F6 stops at each column, the Palette's then the
/// Preview's, each then the dock's panel; a press in a column makes its
/// panel the dock's.
#[test]
fn the_wide_dock_s_columns_by_the_keys_and_a_chosen_blueprint() {
    let Some((mut h, _)) = yielding("yields-columns", Mode::Tabs) else { return };
    size(&mut h, 3440.0, 1440.0);
    assert_eq!(right_as(&h), "columns");
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    h.state_mut().palette.selected = Some(chest);
    let source = mg_ui::model_view::Source::Resource(chest);
    until(&mut h, "the chest's model view in the Preview's column", |h| {
        h.state().model_views.contains_key(&source)
    });
    at_rest(&mut h);
    let preview = h.state().preview_panel_rect().expect("the Preview's column");
    let named = h.query_all_by_label("plc_chest1").any(|n| preview.contains(n.rect().center()));
    assert!(named, "the chest's ResRef in the Preview's column");
    // F6, from the editor area: the Palette's column, then the Preview's.
    let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
    h.hover_at(egui::pos2(1200.0, 20.0));
    let mut seen = Vec::new();
    for _ in 0..6 {
        h.key_press(egui::Key::F6);
        at_rest(&mut h);
        seen.push((keys(&h), h.state().right_dock_shows()));
    }
    let palette = seen.iter().position(|s| s.0 == "Palette").expect("a stop at the Palette");
    let next = &seen[palette + 1];
    assert_eq!(next.0, "Preview", "the Preview's column next: {seen:?}");
    assert_eq!(seen[palette].1, Some(mg_ui::Panel::Palette));
    assert_eq!(next.1, Some(mg_ui::Panel::Preview));
    // A press in the Palette's column: its panel the dock's.
    let dock = h.state().right_dock_rect().unwrap();
    let at = egui::pos2(dock.left() + 100.0, dock.bottom() - 40.0);
    h.hover_at(at);
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    at_rest(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Palette));
}

/// A dock unfolded by hand in a window that folds it, from the least
/// window to 1500 in 40 point steps: the Left Dock by its button, then the
/// Right Dock by its (which folds the Left Dock where both do not fit):
/// each time no two named widgets over each other, no control of the bars
/// or the Left Dock outside the window or over another, the docks inside
/// the window and at their least where the window has the room, never
/// both open where they do not fit, and the editor area never less than
/// nothing (it is narrow beside a dock asked for in the least windows:
/// the builder's choice there).
#[test]
fn a_dock_unfolded_by_hand_in_a_narrow_window_is_whole() {
    let Some((mut h, _)) = yielding("yields-by-hand", Mode::Tabs) else { return };
    let eye_in_row = |found: &str| {
        found.split_once(" over Button \"Show ").is_some_and(|(row, kind)| {
            let kind = kind.trim_end_matches('"').to_lowercase();
            row.trim_end_matches('"').ends_with(&format!(" {kind}"))
        })
    };
    let mut wrong = Vec::new();
    for width in (420..1500).step_by(40) {
        let width = width as f32;
        // (From 1600 each time: neither dock unfolded by hand yet.)
        size(&mut h, 1600.0, 800.0);
        size(&mut h, width, 800.0);
        for side in ["Left", "Right"] {
            let button = format!("Show the {side} Dock");
            if h.query_by_label(&button).is_none() {
                continue;
            }
            h.get_by_label(&button).click();
            at_rest(&mut h);
            let at = |what: String| format!("{width} {side}: {what}");
            let window = h.ctx.content_rect();
            for found in overlapping(&h) {
                if !eye_in_row(&found) {
                    wrong.push(at(found));
                }
            }
            for found in bars_wrong(&h, true) {
                wrong.push(at(found));
            }
            let editor = h.state().editor_area_rect().unwrap();
            if editor.width() < 0.0 {
                wrong.push(at(format!("the editor area {editor:?}")));
            }
            let left_open = left_as(&h) == "open";
            let right = h.state().right_dock_rect();
            if let Some(dock) = right {
                if !window.expand(0.5).contains_rect(dock) {
                    wrong.push(at(format!("the Right Dock {dock:?} outside")));
                }
                let room = window.width() - 2.0 * HARNESS_MARGIN - kit::STRIP - 1.0;
                if dock.width() < kit::RIGHT_DOCK_NARROW.min(room) - 1.0 {
                    wrong.push(at(format!("the Right Dock {} wide", dock.width())));
                }
            }
            if left_open {
                let dock = editor.left() - HARNESS_MARGIN;
                if dock < 190.0 - 0.5 {
                    wrong.push(at(format!("the Left Dock {dock} wide")));
                }
            }
            if left_open && right.is_some() && width < 962.0 {
                wrong.push(at("both docks open where they do not fit".into()));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
