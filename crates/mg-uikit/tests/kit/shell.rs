//! The shell's parts as the shell's tests will drive them: each found by
//! its name (within its dock's group, where two docks could share one) and
//! operated; the marks, not interactive, found as labels.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::shell::{self, DockTab, DockTabs, FoldStrip, PanelHeader, Side, StripClick};
use mg_uikit::status::{self, Hint, Hints, StatusItem, StatusJob, StatusSwitch};
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
    let names = ["Drag Move", "Shift Handles", "Alt + Drag Raise", "Q / E Turn by a step"];
    for name in names {
        h.get_by_label("Hints").get_by_label(name);
    }
    // Room for the first two and part of the third: two, the third gone.
    let first = h.get_by_label("Drag Move").rect();
    let second = h.get_by_label("Shift Handles").rect();
    room.set(second.right() - first.left() + 2.0 * status::HINTS_PADDING + 20.0);
    h.run();
    assert_eq!(shown.get(), 2);
    assert!(h.query_by_label("Alt + Drag Raise").is_none(), "dropped whole");
    assert!(h.query_by_label("G Drop to ground").is_none());
    // (Each whole inside the room given, its padding at each end.)
    let left = h.get_by_label("Drag Move").rect().left() - status::HINTS_PADDING;
    for name in ["Drag Move", "Shift Handles"] {
        let hint = h.get_by_label(name).rect();
        assert!(hint.right() <= left + room.get() - status::HINTS_PADDING, "{name} cut");
    }
    // No room: none.
    room.set(10.0);
    h.run();
    assert_eq!(shown.get(), 0);
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
