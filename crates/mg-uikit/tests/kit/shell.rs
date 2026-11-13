//! The shell's parts as the shell's tests will drive them: each found by
//! its name (within its dock's group, where two docks could share one) and
//! operated; the marks, not interactive, found as labels.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::shell::{self, DockTab, DockTabs, FoldStrip, PanelHeader, Side, StripClick};
use mg_uikit::status::{self, Hint, Hints, StatusItem, StatusJob, StatusLog, StatusSwitch};
use mg_uikit::{Badge, BadgeKind, Icon, IconButton, KeyCap, Theme};

/// A dock's tabs: found within the dock's tab list by their words, said
/// selected or not; a click selects one (and says it changed), Tab and
/// Enter choose one by the keyboard; a badge's count is the tab's value;
/// two docks with a tab of one name do not collide.
#[test]
fn a_dock_tab_clicked_is_selected() {
    let left = Cell::new(0usize);
    let changes = Cell::new(0);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let mut selected = left.get();
        let problems = Badge::count(3).kind(BadgeKind::Error);
        let out = DockTabs::new("Left Dock")
            .tab("Module")
            .tab("Resources")
            .with(DockTab::new("Problems").badge(problems))
            .show(ui, &mut selected, |ui| {
                ui.add(IconButton::new(Icon::ExpandAll, "Expand All").icon_only(true))
            });
        left.set(selected);
        if out.changed {
            changes.set(changes.get() + 1);
        }
        // Another dock with a tab of the same name.
        let mut other = 0;
        DockTabs::new("Bottom Dock").tab("Module").show(ui, &mut other, |_| {});
    });
    h.run();
    let dock = h.get_by_label("Left Dock");
    assert_eq!(dock.accesskit_node().role(), Role::TabList);
    let module = dock.get_by_label("Module");
    assert_eq!(module.accesskit_node().role(), Role::Tab);
    assert_eq!(module.accesskit_node().is_selected(), Some(true));
    let resources = h.get_by_label("Left Dock").get_by_label("Resources");
    assert_eq!(resources.accesskit_node().is_selected(), Some(false));
    resources.click();
    h.run();
    assert_eq!(left.get(), 1);
    assert_eq!(changes.get(), 1);
    let dock = h.get_by_label("Left Dock");
    assert_eq!(dock.get_by_label("Resources").accesskit_node().is_selected(), Some(true));
    assert_eq!(dock.get_by_label("Module").accesskit_node().is_selected(), Some(false));
    // The other dock's tab of the same name is its own.
    let other = h.get_by_label("Bottom Dock").get_by_label("Module");
    assert_eq!(other.accesskit_node().is_selected(), Some(true));
    // The count is the tab's value, not in its name.
    let problems = h.get_by_label("Left Dock").get_by_label("Problems");
    assert_eq!(problems.accesskit_node().value().as_deref(), Some("3"));
    // The tools are in the dock's group too.
    h.get_by_label("Left Dock").get_by_label("Expand All");
    // A click on the tab selected changes nothing.
    h.get_by_label("Left Dock").get_by_label("Resources").click();
    h.run();
    assert_eq!(changes.get(), 1);
    // By the keyboard: focused, then Enter.
    h.get_by_label("Left Dock").get_by_label("Problems").focus();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(left.get(), 2);
}

/// The selected tab reaches over the bar's line (one with the panel under
/// it), the others leave it; the bar is 26 high.
#[test]
fn the_selected_dock_tab_is_one_with_its_panel() {
    for theme in Theme::ALL {
        let t = theme.tokens();
        let bar = Cell::new(egui::Rect::NOTHING);
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            let mut selected = 0;
            let out = DockTabs::new("Left Dock").tab("Module").tab("Resources").show(
                ui,
                &mut selected,
                |_| {},
            );
            bar.set(out.response.rect);
        });
        h.run();
        assert_eq!(bar.get().height(), mg_uikit::metrics::DOCK_TABS);
        let module = h.get_by_label("Module").rect();
        let fills: Vec<egui::epaint::RectShape> = rects(&h);
        let on = fills.iter().find(|r| r.fill == t.panel).expect("the selected tab's fill");
        assert_eq!(on.rect.bottom(), bar.get().bottom(), "{theme:?}: over the line");
        assert_eq!(on.rect.left(), module.left());
        assert!(fills.iter().any(|r| r.fill == t.line), "{theme:?}: the bar's line");
    }
}

/// A folded dock's strip: its chevron named for what it unfolds and said
/// closed, reporting its click; each panel's button reporting its own;
/// the fold button on an open dock named and said open.
#[test]
fn the_fold_strip_s_chevron_unfolds() {
    let clicked = RefCell::new(Vec::new());
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        ui.horizontal(|ui| {
            ui.set_height(300.0);
            let strip = FoldStrip::new("Left Dock", Side::Left)
                .unfold_tip("Show the Left Dock (Ctrl+Alt+1)")
                .panel(Icon::Module, "Module", "Module: unfold the dock on the module tree")
                .panel(Icon::Folder, "Resources", "Resources: unfold the dock on the browser");
            if let Some(click) = strip.show(ui) {
                clicked.borrow_mut().push(click);
            }
            if shell::fold_button(ui, Side::Right, "Right Dock").clicked() {
                clicked.borrow_mut().push(StripClick::Panel(99));
            }
        });
    });
    h.run();
    let chevron = h.get_by_label("Left Dock").get_by_label("Show the Left Dock");
    assert_eq!(chevron.accesskit_node().data().is_expanded(), Some(false));
    chevron.click();
    h.run();
    h.get_by_label("Left Dock").get_by_label("Resources").click();
    h.run();
    assert_eq!(*clicked.borrow(), vec![StripClick::Unfold, StripClick::Panel(1)]);
    let fold = h.get_by_label("Fold the Right Dock");
    assert_eq!(fold.accesskit_node().data().is_expanded(), Some(true));
    fold.click();
    h.run();
    assert_eq!(clicked.borrow().last(), Some(&StripClick::Panel(99)));
    // 24 px wide, its buttons 22 inside it.
    let strip = h.get_by_label("Show the Left Dock").rect();
    assert_eq!(strip.width(), 22.0);
}

/// The fold button with a tooltip of its own (its keys, say): still named
/// for what it folds and said open, the tooltip shown under the pointer.
#[test]
fn a_fold_button_shows_the_tip_it_is_given() {
    let clicked = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let tip = "Fold the Left Dock (Ctrl+Alt+1)";
        if shell::fold_button_tip(ui, Side::Left, "Left Dock", tip).clicked() {
            clicked.set(true);
        }
    });
    h.run();
    let fold = h.get_by_label("Fold the Left Dock");
    assert_eq!(fold.accesskit_node().data().is_expanded(), Some(true));
    fold.hover();
    // (A tooltip waits a moment.)
    h.run_steps(60);
    h.get_by_label("Fold the Left Dock (Ctrl+Alt+1)");
    h.get_by_label("Fold the Left Dock").click();
    h.run();
    assert!(clicked.get());
}

/// A Status Bar switch: named by its words in the Status Bar's group, said
/// on or off; a click toggles what the caller keeps; its count is its
/// value, shown in its badge; with its icon alone, still named so, and its
/// words its tooltip.
#[test]
fn a_status_switch_toggles_and_shows_its_count() {
    let log = Cell::new(false);
    let icons = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Light);
        status::bar(
            ui,
            |ui| {
                let switch = StatusSwitch::new(Icon::Log, "Log", log.get()).icon_only(icons.get());
                if ui.add(switch).clicked() {
                    log.set(!log.get());
                }
                let problems = Badge::count(3).kind(BadgeKind::Error);
                ui.add(StatusSwitch::new(Icon::Problems, "Problems", false).badge(problems));
            },
            |_, _| {},
            |ui| ui.add(StatusItem::new("Chapter1")),
        );
    });
    h.run();
    let switch = h.get_by_label("Status Bar").get_by_label("Log");
    assert_eq!(switch.accesskit_node().role(), Role::Button);
    assert_eq!(switch.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    switch.click();
    h.run();
    assert!(log.get());
    let switch = h.get_by_label("Log");
    assert_eq!(switch.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    let problems = h.get_by_label("Problems");
    assert_eq!(problems.accesskit_node().value().as_deref(), Some("3"));
    // With its icon alone: narrower, the same name.
    let wide = h.get_by_label("Log").rect().width();
    icons.set(true);
    h.run();
    let narrow = h.get_by_label("Log").rect().width();
    assert!(narrow < wide, "{narrow} < {wide}");
    assert_eq!(narrow, 4.0 + 14.0 + 4.0);
    h.get_by_label("Log").click();
    h.run();
    assert!(!log.get());
}

/// A Status Bar given its right-hand group's width lays the group out
/// against its right end at once, and asks for no second pass when the
/// group's width changes (a module's name and its modified dot): a pass
/// asked for again runs without the frame's input.
#[test]
fn a_status_bar_given_its_right_width_needs_no_second_pass() {
    let name = RefCell::new(String::from("Chapter1"));
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let item = StatusItem::new(name.borrow().as_str()).modified(name.borrow().len() > 8);
        let width = item.width(ui);
        status::bar_sized(
            ui,
            width,
            |ui| ui.add(StatusSwitch::new(Icon::Log, "Log", true)),
            |_, _| {},
            |ui| ui.add(item),
        );
    });
    let passes = |h: &Harness<'_>| h.ctx.cumulative_pass_nr();
    h.step();
    let right = |h: &Harness<'_>, n: &str| h.get_by_label("Status Bar").get_by_label(n).rect();
    let edge = h.ctx.content_rect().right();
    let first = right(&h, "Chapter1");
    assert!(edge - first.right() < 12.0, "at the right end at once: {first:?}, {edge}");
    let before = passes(&h);
    *name.borrow_mut() = "Chapter1 (modified)".into();
    h.step();
    assert_eq!(passes(&h), before + 1, "one pass for the frame");
    let now = right(&h, "Chapter1 (modified)");
    assert!((now.right() - first.right()).abs() < 0.5, "{now:?} ends where {first:?} did");
    assert!(now.left() < first.left());
}

/// An item given a width at most cuts its text short with an ellipsis to
/// fit, its kept words (" (modified)") and the modified dot whole; named
/// by all its words still. With room, it is shown whole; with none, the
/// ellipsis and its kept words are left.
#[test]
fn a_status_item_cut_short_keeps_its_kept_words() {
    let long = "a_module_with_a_rather_long_file_name_for_the_status_bar_check";
    let item = || StatusItem::new(long).kept(" (modified)").modified(true);
    let found = RefCell::new(Vec::new());
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let whole = item().width(ui);
        let mut seen = Vec::new();
        for max in [10_000.0, 200.0, 120.0, 0.0] {
            let cut = item().max_width(max);
            seen.push((max, whole, cut.width(ui), cut.shown(ui), cut.is_cut(ui)));
        }
        *found.borrow_mut() = seen;
        ui.add(item().max_width(200.0));
    });
    h.run();
    for (max, whole, width, shown, cut) in found.borrow().iter() {
        assert!(shown.ends_with(" (modified)"), "{max}: {shown}");
        if *max >= *whole {
            assert_eq!((shown.as_str(), *cut), (format!("{long} (modified)").as_str(), false));
            assert_eq!(width, whole);
        } else {
            assert!(cut, "{max}: cut short");
            assert!(shown.contains('…'), "{max}: {shown}");
            assert!(shown.len() < long.len(), "{max}: {shown}");
            if *max > 0.0 {
                assert!(*width <= *max, "{max}: {width}");
                assert!(shown.starts_with("a_m"), "{max}: the start of it: {shown}");
            } else {
                assert_eq!(shown, "… (modified)", "no room: the ellipsis and its kept words");
            }
        }
    }
    let name = format!("{long} (modified)");
    let node = h.get_by_label(&name);
    assert!(node.rect().width() <= 200.0, "{:?}", node.rect());
}

/// A switch that is on draws the accent line along its bottom; off, no
/// fill and no line.
#[test]
fn a_status_switch_on_has_its_accent_line() {
    for theme in Theme::ALL {
        let t = theme.tokens();
        for on in [true, false] {
            let mut h = Harness::new_ui(|ui| {
                mg_uikit::apply(ui.ctx(), theme);
                ui.add(StatusSwitch::new(Icon::Log, "Log", on));
            });
            h.run();
            let switch = h.get_by_label("Log").rect();
            let line = rects(&h).into_iter().find(|r| r.fill == t.accent);
            assert_eq!(line.is_some(), on, "{theme:?} {on}");
            if let Some(line) = line {
                assert_eq!(line.rect.height(), status::SWITCH_ON_LINE);
                assert_eq!(line.rect.bottom(), switch.bottom());
            }
            assert_eq!(switch.height(), status::SWITCH_HEIGHT);
        }
    }
}

/// A key cap, a badge, the modified dot, a hint and a right-hand item are
/// labels: found by their words, not reached with Tab, a click doing
/// nothing.
#[test]
fn the_marks_are_not_interactive() {
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        ui.horizontal(|ui| {
            ui.add(KeyCap::new("Ctrl+P"));
            ui.add(Badge::new("NEW").kind(BadgeKind::Info).tag());
            mg_uikit::modified_dot(ui);
            ui.add(StatusItem::new("Chapter1 (modified)").modified(true));
            ui.add(StatusJob::new("Verify Module", Some(0.4)));
        });
    });
    h.run();
    for name in ["Ctrl+P", "NEW", "Modified", "Chapter1 (modified)"] {
        let node = h.get_by_label(name);
        assert_eq!(node.accesskit_node().role(), Role::Label, "{name}");
        node.click();
        h.run();
    }
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(h.ctx.memory(|m| m.focused()), None, "nothing takes the keys");
    let key = h.get_by_label("Ctrl+P").rect();
    assert_eq!(key.height(), 16.0);
    let job = h.get_by_label("Verify Module");
    assert_eq!(job.accesskit_node().role(), Role::ProgressIndicator);
    assert_eq!(job.accesskit_node().numeric_value(), Some(0.4f32 as f64));
}

/// The hints drop whole from the end where the room ends: each shown hint
/// found by its keys and word, none cut (each drawn whole inside the
/// room), the rest absent.
#[test]
fn hints_drop_whole_at_a_narrow_width() {
    let room = Cell::new(1000.0);
    let shown = Cell::new(0);
    let hints = [
        Hint::keys(["Drag"], "Move"),
        Hint::keys(["Shift"], "Handles"),
        Hint::keys(["Alt", "Drag"], "Raise"),
        Hint::either(["Q", "E"], "Turn by a step"),
        Hint::keys(["G"], "Drop to ground"),
    ];
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        ui.horizontal(|ui| shown.set(Hints::new(&hints).show(ui, room.get())));
    });
    h.run();
    assert_eq!(shown.get(), 5);
    let names = ["Drag: Move", "Shift: Handles", "Alt, Drag: Raise", "Q or E: Turn by a step"];
    for name in names {
        h.get_by_label("Hints").get_by_label(name);
    }
    // Room for the first two and part of the third: two, the third gone.
    let first = h.get_by_label("Drag: Move").rect();
    let second = h.get_by_label("Shift: Handles").rect();
    room.set(second.right() - first.left() + 2.0 * status::HINTS_PADDING + 20.0);
    h.run();
    assert_eq!(shown.get(), 2);
    assert!(h.query_by_label("Alt, Drag: Raise").is_none(), "dropped whole");
    assert!(h.query_by_label("G: Drop to ground").is_none());
    // (Each whole inside the room given, its padding at each end.)
    let left = h.get_by_label("Drag: Move").rect().left() - status::HINTS_PADDING;
    for name in ["Drag: Move", "Shift: Handles"] {
        let hint = h.get_by_label(name).rect();
        assert!(hint.right() <= left + room.get() - status::HINTS_PADDING, "{name} cut");
    }
    // No room: none.
    room.set(10.0);
    h.run();
    assert_eq!(shown.get(), 0);
}

/// With and without the theme applied (egui's own look), a notice, a
/// held key and hints of keys lay out alike: each one label in the group
/// "Hints" named by its keys and word, as wide as `Hint::width` says, in
/// order [`status::HINT_GAP`] apart after the padding, none taking the
/// keyboard (Tab passes them by); `Hints::width` is their whole width.
#[test]
fn hints_lay_out_with_and_without_the_theme() {
    for themed in [false, true] {
        let hints = [
            Hint::note("Painting 2 placeables"),
            Hint::new("Held").held("Shift"),
            Hint::keys(["Alt", "Right Drag"], "Turn"),
            Hint::new("Turn by a step").key("Q").key("E"),
        ];
        let widths = std::cell::RefCell::new(Vec::new());
        let whole = Cell::new(0.0);
        let mut h = Harness::new_ui(|ui| {
            if themed {
                mg_uikit::apply(ui.ctx(), Theme::Dark);
            }
            *widths.borrow_mut() = hints.iter().map(|h| h.width(ui)).collect();
            whole.set(Hints::new(&hints).width(ui));
            ui.horizontal(|ui| Hints::new(&hints).show(ui, 1000.0));
        });
        h.run();
        let names = [
            "Painting 2 placeables",
            "Shift: Held",
            "Alt, Right Drag: Turn",
            "Q, E: Turn by a step",
        ];
        let rects: Vec<egui::Rect> =
            names.iter().map(|n| h.get_by_label("Hints").get_by_label(n).rect()).collect();
        let widths = widths.borrow().clone();
        for (i, r) in rects.iter().enumerate() {
            assert!((r.width() - widths[i]).abs() < 0.01, "{themed}: {}", names[i]);
            assert_eq!(r.height(), status::HINT_HEIGHT);
            if i > 0 {
                let gap = r.left() - rects[i - 1].right();
                assert!((gap - status::HINT_GAP).abs() < 0.01, "{themed}: {gap}");
            }
        }
        let sum: f32 = widths.iter().sum();
        let expected = sum + 3.0 * status::HINT_GAP + 2.0 * status::HINTS_PADDING;
        assert!((whole.get() - expected).abs() < 0.01, "{themed}: {}", whole.get());
        h.key_press(egui::Key::Tab);
        h.run();
        assert_eq!(h.ctx.memory(|m| m.focused()), None, "{themed}: no hint takes the keys");
    }
}

/// A notice that does not fit whole is cut short with an ellipsis (named
/// by its whole words, inside the room), the hints after it dropped; in
/// less than `NOTE_LEAST` it drops as a hint does.
#[test]
fn a_notice_without_room_is_cut_short() {
    let room = Cell::new(1000.0);
    let shown = Cell::new(0);
    let note = "A static placeable takes no scale: clear Static in its properties first";
    let hints = [Hint::note(note), Hint::keys(["Ctrl", "Wheel"], "Zoom slowly")];
    let mut h = Harness::new_ui(|ui| {
        ui.horizontal(|ui| shown.set(Hints::new(&hints).show(ui, room.get())));
    });
    h.run();
    assert_eq!(shown.get(), 2);
    let whole = h.get_by_label(note).rect().width();
    room.set(whole / 2.0);
    h.run();
    assert_eq!(shown.get(), 1, "the notice cut short, the hint after it dropped");
    let cut = h.get_by_label(note).rect();
    assert!(cut.width() < whole && cut.width() <= room.get(), "{cut:?} in {}", room.get());
    assert!(h.query_by_label("Ctrl, Wheel: Zoom slowly").is_none());
    room.set(status::NOTE_LEAST - 1.0 + 2.0 * status::HINTS_PADDING);
    h.run();
    assert_eq!(shown.get(), 0, "too little room: dropped");
    assert!(h.query_by_label(note).is_none());
}

/// A panel's header: a group named by its title, its tools found within
/// it; its detail is not part of its name.
#[test]
fn a_panel_header_names_its_tools_group() {
    let clicks = Cell::new(0);
    let height = Cell::new(0.0);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let header = PanelHeader::new("Outliner").detail("map_m1q1a");
        let out = header.show(ui, |ui| {
            ui.add(IconButton::new(Icon::ExpandAll, "Expand All").icon_only(true));
            if ui.add(IconButton::new(Icon::CollapseAll, "Collapse All").icon_only(true)).clicked()
            {
                clicks.set(clicks.get() + 1);
            }
        });
        height.set(out.response.rect.height());
    });
    h.run();
    h.get_by_label("Outliner").get_by_label("Collapse All").click();
    h.run();
    assert_eq!(clicks.get(), 1);
    assert_eq!(height.get(), mg_uikit::metrics::DOCK_TABS);
    assert!(h.query_by_label_contains("map_m1q1a").is_none(), "the detail is not a name");
    // The tools in the order made, the last against the right end, 2 px
    // from it; and Tab goes through them so.
    let (expand, collapse) =
        (h.get_by_label("Expand All").rect(), h.get_by_label("Collapse All").rect());
    assert_eq!(expand.right(), collapse.left());
    assert_eq!(collapse.right(), h.ctx.content_rect().right() - 8.0 - 2.0);
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("Expand All").is_focused());
}

/// A panel header's detail starts 12 px after its title (the
/// stylesheet's `.docktab.title` padding, `.panel-tabs` having no gap,
/// and the 2 px the boards give the detail), the title 10 px from the
/// header's left.
#[test]
fn a_panel_header_s_detail_is_12_after_its_title() {
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        PanelHeader::new("Outliner").detail("map_m1q1a").show(ui, |_| {});
    });
    h.run();
    let words = texts(&h);
    let at = |w: &str| words.iter().find(|(t, _)| t == w).map(|(_, r)| *r).unwrap();
    let (title, detail) = (at("Outliner"), at("map_m1q1a"));
    assert_eq!(detail.left() - title.right(), shell::PANEL_DETAIL_GAP, "{title:?} {detail:?}");
    assert_eq!(shell::PANEL_DETAIL_GAP, 12.0);
    assert_eq!(title.left() - 8.0, shell::DOCK_TAB_PADDING, "the harness's margin, then 10");
}

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

/// The Top Bar's middle slot: centered in the room the two groups leave,
/// at most the cap wide (and less where the room is less); the group
/// named "Top Bar".
#[test]
fn the_top_bar_centers_its_field() {
    for (width, cap) in [(1600.0, 440.0), (1280.0, 300.0), (700.0, 440.0)] {
        let slot = Cell::new((0.0, egui::Rect::NOTHING));
        let groups = Cell::new((0.0, 0.0));
        let mut h = Harness::builder().with_size(egui::vec2(width, 30.0)).build_ui(|ui| {
            mg_uikit::apply(ui.ctx(), Theme::Dark);
            let (l, _, r) = shell::top_bar(
                ui,
                cap,
                |ui| ui.add_sized([300.0, 22.0], egui::Label::new("menus")).rect.right(),
                |ui, w| slot.set((w, ui.max_rect())),
                |ui| ui.add_sized([250.0, 22.0], egui::Label::new("build")).rect.left(),
            );
            groups.set((l, r));
        });
        h.run();
        h.run();
        let (w, rect) = slot.get();
        let (left_end, right_start) = groups.get();
        // (The row's 2 px gap at each side of the middle, then its 12.)
        let room = right_start - left_end - 2.0 * (shell::TOP_BAR_GAP + shell::TOP_BAR_MID_PADDING);
        assert_eq!(w, room.min(cap).floor(), "{width}");
        assert!(rect.left() >= left_end + 14.0 && rect.right() <= right_start - 14.0, "{width}");
        let middle = (left_end + right_start) / 2.0;
        assert!((rect.center().x - middle).abs() <= 1.0, "{width}: centered");
        h.get_by_label("Top Bar");
    }
}

/// The rectangles drawn in a frame.
fn rects(h: &Harness<'_>) -> Vec<egui::epaint::RectShape> {
    fn add(shape: &egui::Shape, to: &mut Vec<egui::epaint::RectShape>) {
        match shape {
            egui::Shape::Rect(r) => to.push(r.clone()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| add(s, to)),
            _ => {}
        }
    }
    let mut all = Vec::new();
    h.output().shapes.iter().for_each(|s| add(&s.shape, &mut all));
    all
}

/// A menu opened from a menu bar styled as the kit says: its rows 24 apart
/// with no gap, its title 22 high.
#[test]
fn a_menu_s_rows_are_24_apart() {
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let config = egui::containers::menu::MenuConfig::new().style(mg_uikit::menu_style);
        egui::MenuBar::new().style(mg_uikit::menu_bar_style).config(config).ui(ui, |ui| {
            ui.menu_button("Edit", |ui| {
                let _ = ui.button("Undo");
                let _ = ui.button("Redo");
                mg_uikit::menu_button(ui, "Position", |ui| {
                    let _ = ui.button("Center");
                });
            });
        });
    });
    h.run();
    assert_eq!(h.get_by_label("Edit").rect().height(), mg_uikit::metrics::MENU_TITLE);
    h.get_by_label("Edit").click();
    h.run();
    let tops: Vec<f32> =
        ["Undo", "Redo", "Position"].iter().map(|n| h.get_by_label(n).rect().top()).collect();
    for pair in tops.windows(2) {
        assert_eq!(pair[1] - pair[0], mg_uikit::metrics::MENU_ROW, "{tops:?}");
    }
    assert_eq!(h.get_by_label("Undo").rect().height(), mg_uikit::metrics::MENU_ROW);
}

/// A button and its arrow joined (the mock's `.join`: Test Module and
/// Choose Character's arrow on the Top Bar): the button's right corners
/// square, the arrow's left ones; the arrow framed as the button is
/// (`raised` at rest), 24 wide (its icon 4 px from each side), as high as
/// a control, named by its name.
#[test]
fn a_joined_arrow_is_framed_beside_its_button() {
    use mg_uikit::Join;
    for theme in Theme::ALL {
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            ui.horizontal(|ui| {
                // (egui puts the spacing after a widget as it adds it.)
                ui.spacing_mut().item_spacing.x = mg_uikit::joined_spacing(ui);
                ui.add(IconButton::new(Icon::Test, "Test Module").joined(Join::Next));
                ui.add(
                    IconButton::new(Icon::ChevronDown, "More")
                        .icon_only(true)
                        .joined(Join::Previous),
                );
            });
        });
        h.run();
        let t = mg_uikit::tokens(&h.ctx);
        let main = h.get_by_label("Test Module").rect();
        let arrow = h.get_by_label("More").rect();
        assert_eq!(arrow.width(), 24.0, "{theme:?}");
        assert_eq!(arrow.height(), mg_uikit::metrics::CONTROL_HEIGHT);
        let fills = rects(&h);
        // (A frame drawn over the widget's place: egui's button frame may
        // stand half a point in or out of it.)
        let at = |r: egui::Rect| {
            let near = |s: &&egui::epaint::RectShape| {
                s.fill != egui::Color32::TRANSPARENT
                    && (s.rect.center() - r.center()).length() < 1.0
                    && (s.rect.width() - r.width()).abs() <= 2.0
            };
            fills.iter().find(near).cloned()
        };
        let arrow_fill = at(arrow).expect("the arrow's frame");
        assert_eq!(arrow_fill.fill, t.raised, "{theme:?}: raised at rest");
        let c = arrow_fill.corner_radius;
        assert!(c.nw == 0 && c.sw == 0 && c.ne > 0 && c.se > 0, "{c:?}");
        let main_fill = at(main).expect("the button's frame");
        let c = main_fill.corner_radius;
        assert!(c.ne == 0 && c.se == 0 && c.nw > 0 && c.sw > 0, "{c:?}");
        // (Their frames 1 px apart, and as high as each other.)
        let gap = arrow_fill.rect.left() - main_fill.rect.right();
        assert!((gap - 1.0).abs() < 0.01, "1 px apart: {main_fill:?} {arrow_fill:?}");
        assert_eq!(arrow_fill.rect.y_range(), main_fill.rect.y_range());
        // An icon alone framed: `raised` at rest, an icon button's size.
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            ui.add(IconButton::new(Icon::Compile, "Compile").icon_only(true).framed(true));
        });
        h.run();
        let at = h.get_by_label("Compile").rect();
        assert_eq!(at.size(), egui::Vec2::splat(mg_uikit::metrics::ICON_BUTTON));
        let fills = rects(&h);
        let framed = fills.iter().any(|s| s.fill == t.raised && s.rect.contains_rect(at));
        assert!(framed, "{theme:?}: framed at rest");
    }
}

/// A dock's tabs show their icons alone where their words and the tools
/// do not fit the bar side by side, each tab still a tab named by its
/// words, said selected or not, its name in its tip; their words where
/// they fit. Swept a point at a time across the boundary, each width has
/// the one or the other on every frame, icons below the boundary and words
/// above it, and nothing of the bar is past its right end (from the
/// icons' and the tools' own width up).
#[test]
fn a_dock_s_tabs_show_their_icons_where_their_words_do_not_fit() {
    let width = Cell::new(300.0f32);
    let icons = Cell::new(false);
    let ends = Cell::new((0.0f32, 0.0f32));
    let mut h = Harness::builder().with_size(egui::vec2(400.0, 120.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let w = width.get();
        ui.allocate_ui(egui::vec2(w, 60.0), |ui| {
            ui.set_width(w);
            let mut selected = 1;
            let tools = 4.0 * 22.0;
            let out = DockTabs::new("Left Dock")
                .with(DockTab::new("Module").icon(Icon::Module))
                .with(DockTab::new("Resources").icon(Icon::Folder))
                .tools_width(tools)
                .show(ui, &mut selected, |ui| {
                    let mut last = egui::Rect::NOTHING;
                    for name in ["A", "B", "C", "D"] {
                        last = ui.add(IconButton::new(Icon::Plus, name).icon_only(true)).rect;
                    }
                    last.right()
                });
            icons.set(out.icons);
            ends.set((out.inner, out.response.rect.right()));
        });
    });
    // Each width: the same on three frames; icons only below a boundary.
    let mut seen = Vec::new();
    for w in (165..=320).map(|w| w as f32) {
        width.set(w);
        let mut each = Vec::new();
        for _ in 0..3 {
            h.run();
            each.push(icons.get());
        }
        assert!(each.iter().all(|i| *i == each[0]), "{w}: {each:?}");
        let (tools_end, bar_end) = ends.get();
        assert!(tools_end <= bar_end + 0.5, "{w}: the tools past the bar's end");
        seen.push((w, each[0]));
    }
    let first_words = seen.iter().position(|(_, i)| !i).expect("words somewhere");
    assert!(first_words > 0, "icons somewhere");
    assert!(seen[first_words..].iter().all(|(_, i)| !i), "no icons above the boundary");
    assert!(seen[..first_words].iter().all(|(_, i)| *i), "no words below it");
    // Icons: still named tabs, said selected; the name in the tip.
    width.set(170.0);
    h.run();
    assert!(icons.get());
    let tab = h.get_by_label("Left Dock").get_by_label("Resources");
    assert_eq!(tab.accesskit_node().role(), Role::Tab);
    assert_eq!(tab.accesskit_node().is_selected(), Some(true));
    let module = h.get_by_label("Left Dock").get_by_label("Module");
    assert_eq!(module.accesskit_node().is_selected(), Some(false));
    module.hover();
    for _ in 0..30 {
        h.run();
    }
    assert!(h.query_all_by_label("Module").count() >= 2, "its name in its tip");
}

/// A dock's fold button on its inner edge: across the edge, half on each
/// side, half way down; named and said open; its click reported.
#[test]
fn a_dock_s_fold_button_stands_on_its_inner_edge() {
    let clicked = Cell::new(0);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Light);
        let rect = shell::edge_fold_rect(200.0, egui::Rangef::new(40.0, 340.0));
        if shell::edge_fold_button(
            ui,
            rect,
            Side::Left,
            "Left Dock",
            "Fold the Left Dock (Ctrl+Alt+1)",
        )
        .clicked()
        {
            clicked.set(clicked.get() + 1);
        }
    });
    h.run();
    let fold = h.get_by_label("Fold the Left Dock");
    assert_eq!(fold.accesskit_node().data().is_expanded(), Some(true));
    let r = fold.rect();
    assert_eq!(r.center(), egui::pos2(200.0, 190.0), "across the edge, half way down");
    assert_eq!(r.size(), egui::vec2(shell::EDGE_FOLD_WIDTH, shell::EDGE_FOLD_LENGTH));
    fold.click();
    h.run();
    assert_eq!(clicked.get(), 1);
    // A short dock: as long as the dock.
    let short = shell::edge_fold_rect(10.0, egui::Rangef::new(0.0, 30.0));
    assert_eq!(short.height(), 30.0);
}

/// A dock's fold button beside its inner edge (`edge_fold_rect_beside`):
/// from the edge toward the middle of the window, never over the dock,
/// half way down; folded, the button that unfolds it in the same place
/// beside the strip's edge, as high and as large, pointing the other way,
/// named "Show the Left Dock" and said closed; each reports its click.
/// The Right Dock's stands at the left of its edge.
#[test]
fn a_dock_s_fold_button_beside_its_edge_and_the_unfold_button_in_its_place() {
    let folded = Cell::new(false);
    let clicks = Cell::new(0);
    let along = egui::Rangef::new(40.0, 340.0);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let edge = if folded.get() { 25.0 } else { 261.0 };
        let rect = shell::edge_fold_rect_beside(edge, along, Side::Left);
        let r = if folded.get() {
            shell::edge_unfold_button(ui, rect, Side::Left, "Left Dock", "the tip")
        } else {
            shell::edge_fold_button(ui, rect, Side::Left, "Left Dock", "the tip")
        };
        if r.clicked() {
            clicks.set(clicks.get() + 1);
        }
    });
    h.run();
    let fold = h.get_by_label("Fold the Left Dock");
    assert_eq!(fold.accesskit_node().data().is_expanded(), Some(true));
    let open = fold.rect();
    assert_eq!(open.left(), 261.0, "beside the edge, not over the dock");
    assert_eq!(open.center().y, along.center(), "half way down");
    assert_eq!(open.size(), egui::vec2(shell::EDGE_FOLD_WIDTH, shell::EDGE_FOLD_LENGTH));
    fold.click();
    h.run();
    assert_eq!(clicks.get(), 1);
    folded.set(true);
    h.run();
    assert!(h.query_by_label("Fold the Left Dock").is_none());
    let show = h.get_by_label("Show the Left Dock");
    assert_eq!(show.accesskit_node().data().is_expanded(), Some(false));
    let closed = show.rect();
    assert_eq!(closed.left(), 25.0);
    assert_eq!((closed.center().y, closed.size()), (open.center().y, open.size()), "its place");
    show.click();
    h.run();
    assert_eq!(clicks.get(), 2);
    // The Right Dock's: at the left of its edge.
    let right = shell::edge_fold_rect_beside(1000.0, along, Side::Right);
    assert_eq!(right.right(), 1000.0);
    // A short dock: as long as the dock.
    let short = shell::edge_fold_rect_beside(10.0, egui::Rangef::new(0.0, 30.0), Side::Left);
    assert_eq!((short.top(), short.height()), (0.0, 30.0));
}

/// A strip without its chevron (its dock unfolded by the button on its
/// edge): no "Show the Left Dock" in it, its panels' buttons from its top,
/// each reporting its click.
#[test]
fn a_fold_strip_without_its_chevron_starts_with_its_panels() {
    let clicked = RefCell::new(Vec::new());
    let top = Cell::new(0.0);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        ui.horizontal(|ui| {
            ui.set_height(300.0);
            top.set(ui.max_rect().top());
            let strip = FoldStrip::new("Left Dock", Side::Left)
                .chevron(false)
                .panel(Icon::Module, "Module", "Module")
                .panel(Icon::Folder, "Resources", "Resources");
            if let Some(click) = strip.show(ui) {
                clicked.borrow_mut().push(click);
            }
        });
    });
    h.run();
    let strip = h.get_by_label("Left Dock");
    assert!(strip.query_by_label("Show the Left Dock").is_none(), "no chevron");
    let top = top.get();
    let module = h.get_by_label("Left Dock").get_by_label("Module").rect();
    assert!((module.top() - top - shell::STRIP_PADDING).abs() < 1.0, "{module:?} {top}");
    h.get_by_label("Left Dock").get_by_label("Resources").click();
    h.run();
    assert_eq!(*clicked.borrow(), vec![StripClick::Panel(1)]);
}

/// Left and Right on a dock tab with the keyboard's focus choose the tab
/// before or after it, round the ends, and take the focus there; the keys
/// are the tab's (taken, not left for what is drawn after the bar).
#[test]
fn the_arrow_keys_go_between_a_dock_s_tabs() {
    let left = Cell::new(0usize);
    let after = Cell::new(0);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let mut selected = left.get();
        DockTabs::new("Left Dock").tab("Module").tab("Resources").tab("Outliner").show(
            ui,
            &mut selected,
            |_| {},
        );
        left.set(selected);
        // (What the panel under the bar would read.)
        let keys = [egui::Key::ArrowLeft, egui::Key::ArrowRight];
        if ui.input(|i| keys.iter().any(|k| i.key_pressed(*k))) {
            after.set(after.get() + 1);
        }
    });
    h.run();
    let focused = |h: &Harness<'_>, name: &str| {
        h.get_by_label("Left Dock").get_by_label(name).accesskit_node().is_focused()
    };
    h.get_by_label("Left Dock").get_by_label("Module").focus();
    h.run();
    h.key_press(egui::Key::ArrowRight);
    h.run();
    h.run();
    assert_eq!(left.get(), 1);
    assert!(focused(&h, "Resources"), "the focus goes with it");
    h.key_press(egui::Key::ArrowRight);
    h.run();
    h.run();
    assert_eq!(left.get(), 2);
    h.key_press(egui::Key::ArrowRight);
    h.run();
    h.run();
    assert_eq!(left.get(), 0, "round the end");
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    h.run();
    assert_eq!(left.get(), 2, "and back round it");
    assert!(focused(&h, "Outliner"));
    let said = h.get_by_label("Left Dock").get_by_label("Outliner").accesskit_node().is_selected();
    assert_eq!(said, Some(true));
    assert_eq!(after.get(), 0, "the keys the tabs'");
}

/// How soon the harness's window asks to be drawn again.
fn again(h: &Harness<'_>) -> std::time::Duration {
    let out = &h.output().viewport_output;
    out.get(&egui::ViewportId::ROOT).map_or(std::time::Duration::MAX, |v| v.repaint_delay)
}

/// A job whose progress is not known ([`StatusJob::unknown`]) draws a
/// moving mark along its bar (`accent`, a quarter of the bar), which moves
/// from frame to frame and asks for a frame each
/// [`status::JOB_MARK_FRAME`]; one given no part done
/// (`StatusJob::new(_, None)`) keeps its empty bar, still, and asks for
/// none, as before the mark; a job whose part done is known draws that
/// part, still, and asks for none. Its accessible
/// name is the one given it, its words cut short to its width at most,
/// its bar never.
#[test]
fn a_job_of_unknown_progress_has_a_moving_mark() {
    // (0: not known, 1: none given, 2: half done.)
    let mode = Cell::new(0);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let job = match mode.get() {
            0 => StatusJob::unknown("Verify Module"),
            1 => StatusJob::new("Verify Module", None),
            _ => StatusJob::new("Verify Module", Some(0.5)),
        };
        ui.add(job.name("Verify Module, under way"));
    });
    h.run_steps(1);
    let t = Theme::Dark.tokens();
    let mark = |h: &Harness<'_>| {
        let all: Vec<_> = rects(h).into_iter().filter(|r| r.fill == t.accent).collect();
        assert_eq!(all.len(), 1, "one mark");
        all[0].rect
    };
    let first = mark(&h);
    assert_eq!(first.width(), status::JOB_MARK);
    h.step();
    assert_ne!(mark(&h).left(), first.left(), "it moves");
    // (Read after a frame of its own: a harness's first frame may report
    // none.)
    assert!(again(&h) <= status::JOB_MARK_FRAME, "{:?}", again(&h));
    let node = h.get_by_label("Verify Module, under way");
    assert_eq!(node.accesskit_node().numeric_value(), None);
    mode.set(1);
    h.run_steps(1);
    h.step();
    let accent = rects(&h).into_iter().filter(|r| r.fill == t.accent).count();
    assert_eq!(accent, 0, "none given: an empty bar");
    assert!(again(&h) > std::time::Duration::from_secs(1), "still: {:?}", again(&h));
    mode.set(2);
    h.run_steps(1);
    assert_eq!(mark(&h).width(), status::JOB_BAR.x * 0.5);
    assert!(again(&h) > std::time::Duration::from_secs(1), "still: {:?}", again(&h));
    // The mark goes to the bar's end and back once each period.
    let end = status::JOB_BAR.x - status::JOB_MARK;
    assert_eq!(StatusJob::mark_offset(0.0, status::JOB_BAR.x), 0.0);
    let half = StatusJob::mark_offset(status::JOB_MARK_PERIOD / 2.0, status::JOB_BAR.x);
    assert!((half - end).abs() < 0.01, "{half}");
    let whole = StatusJob::mark_offset(status::JOB_MARK_PERIOD * 0.999, status::JOB_BAR.x);
    assert!(whole < 1.0, "{whole}");
    // Cut short: its words, never its bar.
    let found = RefCell::new(Vec::new());
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let job = || StatusJob::new("Compile All Scripts", Some(0.1));
        let whole = job().width(ui);
        let cut = job().max_width(100.0);
        *found.borrow_mut() = vec![(whole, job().is_cut(ui)), (cut.width(ui), cut.is_cut(ui))];
        ui.add(cut);
    });
    h.run();
    let found = found.borrow();
    assert!(!found[0].1 && found[0].0 > 100.0, "{found:?}");
    assert!(found[1].1 && found[1].0 <= 100.0, "{found:?}");
    let words: Vec<String> = (h.output().shapes.iter())
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => {
                Some(t.galley.rows.iter().flat_map(|r| r.glyphs.iter()).map(|g| g.chr).collect())
            }
            _ => None,
        })
        .collect();
    assert!(words.iter().any(|w| w.starts_with("Co") && w.ends_with('…')), "{words:?}");
    let bar = rects(&h).into_iter().find(|r| r.fill == t.inset).expect("the bar");
    assert_eq!(bar.rect.width(), status::JOB_BAR.x);
}

/// The log's last line: in `text-3`, or the color given it; cut short
/// with an ellipsis to its width at most (named by its first line whole,
/// that line alone shown); a button, which reports its click.
#[test]
fn the_log_s_last_line_is_cut_short_and_pressed() {
    let long = "Verify Module found 12 problems in the module's scripts and areas\nand more";
    let clicks = Cell::new(0);
    let color = Cell::new(None::<egui::Color32>);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Light);
        let mut line = StatusLog::new(long).max_width(160.0);
        if let Some(c) = color.get() {
            line = line.color(c);
        }
        assert!(line.is_cut(ui) && line.width(ui) <= 160.0);
        assert!(line.shown(ui).starts_with("Verify") && line.shown(ui).ends_with('…'));
        assert!(!StatusLog::new("Short").max_width(160.0).is_cut(ui));
        if ui.add(line).clicked() {
            clicks.set(clicks.get() + 1);
        }
    });
    h.run();
    let t = Theme::Light.tokens();
    let painted = |h: &Harness<'_>| {
        let mut out = Vec::new();
        for s in &h.output().shapes {
            if let egui::Shape::Text(text) = &s.shape {
                out.push(text.fallback_color);
            }
        }
        out
    };
    assert_eq!(painted(&h), [t.text_3]);
    // (Named by its first line, whole, where no name is given.)
    let node = h.get_by_label(long.lines().next().unwrap());
    assert_eq!(node.accesskit_node().role(), Role::Button);
    assert!(node.rect().width() <= 160.0);
    node.click();
    h.run();
    assert_eq!(clicks.get(), 1);
    color.set(Some(t.error));
    h.hover_at(egui::pos2(300.0, 300.0));
    h.run();
    assert_eq!(painted(&h), [t.error]);
}
