//! The editor area's document tabs and editor groups (M3's T9): each tab's
//! kind icon, name and modified dot, its close button, Split Editor and
//! the groups it makes, the tab keys per group, a crowded tab bar, and the
//! groups kept in a module's open-tabs record.

use crate::common::*;
use egui::accesskit::Role;

/// The sample module with a 2DA of its own and `scripts` more scripts
/// (`s01`, `s02`…), for tabs of every kind of state.
fn module_with(dir: &std::path::Path, scripts: usize) -> std::path::PathBuf {
    let path = sample_module(dir);
    let mut m = Module::open(&path).unwrap();
    m.set(
        ResKey::parse("mytable", ResType::TWODA).unwrap(),
        b"2DA V2.0\n\n   LABEL\n0  one\n".to_vec(),
    );
    for i in 1..=scripts {
        let key = ResKey::parse(&format!("s{i:02}"), ResType::NSS).unwrap();
        m.set(key, b"void main() { }\n".to_vec());
    }
    m.save().unwrap();
    path
}

/// The tabs of the main surface's panes (the editor groups), left to
/// right, each pane's tabs in order.
fn groups(h: &Harness<'_, Moonglow>) -> Vec<Vec<Tab>> {
    let mut panes: Vec<(f32, Vec<Tab>)> = (h.state().dock.iter_leaves())
        .filter(|(p, l)| p.surface.is_main() && !l.tabs.is_empty())
        .map(|(_, l)| (l.rect.left(), l.tabs.clone()))
        .collect();
    panes.sort_by(|a, b| a.0.total_cmp(&b.0));
    panes.into_iter().map(|p| p.1).collect()
}

/// Whether a tab's node says it has unsaved changes.
fn said_modified(h: &Harness<'_, Moonglow>, name: &str) -> bool {
    doc_tab(h, name).accesskit_node().description().as_deref() == Some("Modified")
}

/// The color of the line drawn along the top of `tab` at the last frame
/// (the front tab of its pane: `accent` in the pane with the focus,
/// `line-soft` in the others), while the icons are recorded.
fn line_over(h: &Harness<'_, Moonglow>, tab: &Tab) -> Option<egui::Color32> {
    let rect = h.state().tab_rect(tab)?;
    let lines = mg_uikit::front_lines_drawn(&h.ctx);
    lines.iter().find(|(r, _)| (r.min - rect.min).length() < 0.5).map(|l| l.1)
}

/// Frames until `done` says so, for ten seconds at most (the window's
/// frames as they come: a dot compared once its document has rested, a
/// tip after its delay), failing with `what`.
fn until(h: &mut Harness<'_, Moonglow>, what: &str, done: impl Fn(&Harness<'_, Moonglow>) -> bool) {
    let started = std::time::Instant::now();
    while !done(h) {
        assert!(started.elapsed().as_secs() < 10, "never: {what}");
        h.step();
    }
}

/// A script's tab, an area's and a side editor's (a 2DA's) and the talk
/// table's show the modified dot where their document has unsaved
/// changes, said "Modified" to a screen reader, never as a star in the
/// tab's name; it goes once the change is saved, or undone back to what
/// is saved. The window's own title keeps its star for the module.
#[test]
fn the_modified_dot_follows_each_tab_s_own_unsaved_changes() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-dot");
    let path = module_with(&dir, 0);
    let script = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(ResRef::from_str("start").unwrap())));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(script)));
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    assert!(!said_modified(&h, "hello.nss") && !said_modified(&h, "start"));

    // A script typed in: its tab, named without a star.
    h.state_mut().script_tools.jump = Some((script, 0));
    let editor = egui::Id::new(("script", script));
    until(&mut h, "the script's editor has the keys", |h| h.ctx.memory(|m| m.has_focus(editor)));
    h.event(egui::Event::Text("// one\n".into()));
    until(&mut h, "the script's dot", |h| said_modified(h, "hello.nss"));
    assert!(h.query_by_label("hello.nss *").is_none(), "no star in its name");
    let starred = h
        .get_all_by_role(Role::Tab)
        .filter(|n| n.accesskit_node().label().is_some_and(|l| l.contains('*')))
        .count();
    assert_eq!(starred, 0, "no tab named with a star");
    // Saved into the module (the editor's Ctrl+S): the dot goes.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    until(&mut h, "the dot gone with the save", |h| !said_modified(h, "hello.nss"));

    // An area's own files changed: its tab, until undone.
    let are = ResKey::parse("start", ResType::ARE).unwrap();
    let tag = |v: &str| mg_edit::Edit::SetField {
        key: are,
        path: mg_edit::GffPath::root(),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(v.into())),
    };
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.apply(mg_edit::Command::new("Tag", vec![tag("MOVED")])).unwrap();
    until(&mut h, "the area's dot", |h| said_modified(h, "start"));
    assert!(h.state().title().ends_with(" *"), "the window's title keeps its star");
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    until(&mut h, "undone back to what is saved", |h| !said_modified(h, "start"));
    // Changed and saved with the module.
    h.state_mut().ws.as_mut().unwrap().apply(mg_edit::Command::new("Tag", vec![tag("X")])).unwrap();
    until(&mut h, "the area's dot again", |h| said_modified(h, "start"));
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert!(!said_modified(&h, "start"), "saved with the module");

    // A 2DA's editor (a side editor), until saved.
    let id = mg_ui::table_view::open_in_module(
        h.state_mut(),
        ResKey::parse("mytable", ResType::TWODA).unwrap(),
    )
    .unwrap();
    settle(&mut h);
    assert!(!said_modified(&h, "mytable.2da"));
    h.state_mut().tables.iter_mut().find(|t| t.id == id).unwrap().edit.add_rows(1);
    until(&mut h, "the 2DA's dot", |h| said_modified(h, "mytable.2da"));
    assert!(mg_ui::table_view::save(h.state_mut(), id));
    until(&mut h, "the 2DA saved", |h| !said_modified(h, "mytable.2da"));

    // The talk table, until saved.
    let mut table =
        mg_module::talk::Table::create("words", &dir, mg_core::Language::ENGLISH, false);
    table.source = mg_module::talk::Source::File(dir.join("words.tlk"));
    table.save().unwrap();
    h.state_mut().talk = Some(table);
    h.state_mut().talk_view.outside = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::TalkTable));
    settle(&mut h);
    assert!(!said_modified(&h, "Talk Table"));
    h.get_all_by_label("Add Line").last().unwrap().click();
    settle(&mut h);
    assert!(said_modified(&h, "Talk Table"));
    h.state_mut().talk.as_mut().unwrap().save().unwrap();
    until(&mut h, "the talk table saved", |h| !said_modified(h, "Talk Table"));
}

/// Each tab is drawn with its kind's icon, in the tab's room before its
/// name: an area's, a script's and a creature blueprint's.
#[test]
fn each_tab_shows_its_kind_s_icon() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-icons");
    let path = module_with(&dir, 0);
    let mut m = Module::open(&path).unwrap();
    let utc = ResKey::parse("guard", ResType::UTC).unwrap();
    m.set(utc, Gff::new(*b"UTC ").to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let area = Tab::Area(ResRef::from_str("start").unwrap());
    let script = Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap());
    for tab in [area.clone(), script.clone(), Tab::Blueprint(utc)] {
        app.actions.push(mg_ui::Action::OpenTab(tab));
    }
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    mg_uikit::record_icons(&h.ctx);
    until(&mut h, "the tabs' icons recorded", |h| !mg_uikit::icons_drawn(&h.ctx).is_empty());
    let drawn = mg_uikit::icons_drawn(&h.ctx);
    for (name, icon) in [
        ("start", mg_uikit::Icon::Area),
        ("hello.nss", mg_uikit::Icon::Script),
        ("guard.utc", mg_uikit::Icon::Creature),
        ("Module Properties", mg_uikit::Icon::Module),
    ] {
        let tab = doc_tab(&h, name).rect();
        // (At the tab's left, after its padding: windows over the main
        // surface's tabs draw theirs over these.)
        let at =
            |r: &egui::Rect| (r.left() - tab.left() - 10.0).abs() < 0.5 && tab.contains_rect(*r);
        assert!(drawn.iter().any(|(i, r)| *i == icon && at(r)), "{name}: {tab:?} {drawn:?}");
    }
}

/// `common::click_tab_close` closes the docked tab it is given with a
/// floating window's tab over the same tab bar, left of it (its tabs are
/// counted in the bar's own layer alone: M3 T14's second review, where a
/// floating window's tabs shifted which tab it took).
#[test]
fn a_docked_tab_closes_by_its_button_with_a_window_s_tab_over_its_bar() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-close-under-window");
    let path = module_with(&dir, 0);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.window_places = vec![("manual".into(), "middle".into())];
    app.open_module(&path);
    let start = Tab::Area(ResRef::from_str("start").unwrap());
    app.actions.push(mg_ui::Action::OpenTab(start.clone()));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Manual));
    let mut h = harness(app, 1200.0, 800.0);
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start.clone(), Tab::Manual]]);
    // The Faction Editor in a window of its own, its tab over the editor
    // area's tab bar, its window wholly left of the manual's tab.
    let target = h.state().tab_rect(&Tab::Manual).expect("the manual's tab");
    let bar = h.state().dock.iter_leaves().find(|(p, _)| p.surface.is_main()).unwrap().1.rect;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    {
        let dock = &mut h.state_mut().dock;
        let found = dock.find_tab(&Tab::Factions).unwrap();
        let tab = dock.remove_tab(found).unwrap();
        let surface = dock.add_window(vec![tab]);
        let window = dock.get_window_state_mut(surface).unwrap();
        let width = 214.0;
        window
            .set_position(egui::pos2(target.left() - width - 6.0, bar.top() - 1.0))
            .set_size(egui::vec2(width, 300.0));
    }
    settle(&mut h);
    let factions = h.state().tab_rect(&Tab::Factions).expect("the window's tab");
    let over = factions.top() >= bar.top() - 0.5 && factions.right() < target.left();
    assert!(over, "over the bar, left of the manual's tab: {factions:?} {target:?} {bar:?}");
    click_tab_close(&mut h, &Tab::Manual);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Manual).is_none(), "the manual's tab closed");
    assert!(h.state().dock.find_tab(&Tab::Factions).is_some(), "the window's tab open");
    assert!(h.state().dock.find_tab(&start).is_some(), "the area's tab open");
}

/// The close button: on the tab in front of its pane always, on another
/// only under the pointer, named for its tab; its room kept whether shown
/// or not, so the tab and its name do not move as it comes; pressed, it
/// closes its own tab and not the one in front. A middle click closes a
/// tab too, and its menu's Close.
#[test]
fn a_tab_s_close_button_shows_on_the_front_tab_and_under_the_pointer() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-close");
    let path = module_with(&dir, 0);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(ResRef::from_str("start").unwrap())));
    let mut h = harness(app, 1200.0, 800.0);
    settle(&mut h);
    // (Module Properties docked as the module opened, the area in front.)
    assert_eq!(
        groups(&h),
        [vec![Tab::ModuleProperties, Tab::Area(ResRef::from_str("start").unwrap())]]
    );
    h.get_by_label("Close start");
    assert!(h.query_by_label("Close Module Properties").is_none(), "not under the pointer");
    let before = doc_tab(&h, "Module Properties").rect();
    h.hover_at(egui::pos2(before.left() + 20.0, before.center().y));
    until(&mut h, "its close button under the pointer", |h| {
        h.query_by_label("Close Module Properties").is_some()
    });
    let close = h.get_by_label("Close Module Properties").rect();
    assert!(before.contains_rect(close), "in its tab: {close:?} {before:?}");
    assert_eq!(doc_tab(&h, "Module Properties").rect(), before, "the tab as it was");
    h.hover_at(close.center());
    let started = std::time::Instant::now();
    while h.query_by_label("Close this tab").is_none() && started.elapsed().as_secs() < 10 {
        h.step();
    }
    h.get_by_label("Close this tab");
    // Pressed: that tab closes, the one in front stays.
    click_tab_close(&mut h, &Tab::ModuleProperties);
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::Area(ResRef::from_str("start").unwrap())]]);
    // A middle click on a tab closes it; the menu's Close too.
    h.state_mut().settings.window_places =
        vec![("manual".into(), "middle".into()), ("journal".into(), "middle".into())];
    for tab in [Tab::Manual, Tab::Journal] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        settle(&mut h);
    }
    let manual = doc_tab(&h, "User Manual").rect();
    h.hover_at(manual.center());
    until(&mut h, "the manual's tab under the pointer", |h| {
        h.query_by_label("Close User Manual").is_some()
    });
    // (A step for each event: the press and the release in frames of their
    // own, as a click.)
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: manual.center(),
            button: egui::PointerButton::Middle,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(1);
    }
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Manual).is_none(), "middle click");
    doc_tab(&h, "Journal").click_secondary();
    until(&mut h, "the tab's menu", |h| h.query_by_label("Close Others").is_some());
    h.get_by_label("Close").click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Journal).is_none(), "its menu's Close");
}

/// The app with the sample module (and `scripts` more scripts) open, the
/// area `start` and Module Properties in one editor group, and `tabs` more
/// opened in it (scripts, the manual and the journal kept in the middle,
/// as a builder who dragged one there has them).
fn one_group(
    name: &str,
    scripts: usize,
    tabs: &[Tab],
    size: egui::Vec2,
) -> Harness<'static, Moonglow> {
    one_group_in(Mode::Floating, name, scripts, tabs, size)
}

/// [`one_group`] in `mode`: in As Tabs, nothing kept for the kinds (the
/// scripts, the manual and the journal open in the group as any editor
/// does there).
fn one_group_in(
    mode: Mode,
    name: &str,
    scripts: usize,
    tabs: &[Tab],
    size: egui::Vec2,
) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = module_with(&dir, scripts);
    let mut app = app_in(mode, Vec::new());
    app.settings.no_last_area = true;
    if !mode.tabs() {
        app.settings.window_places = ["script", "manual", "journal"]
            .iter()
            .map(|k| (k.to_string(), "middle".to_string()))
            .collect();
    }
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Area(ResRef::from_str("start").unwrap())));
    for tab in tabs {
        app.actions.push(mg_ui::Action::OpenTab(tab.clone()));
    }
    let mut h = harness(app, size.x, size.y);
    settle(&mut h);
    h
}

fn start() -> Tab {
    Tab::Area(ResRef::from_str("start").unwrap())
}

fn hello() -> Tab {
    Tab::Script(ResKey::parse("hello", ResType::NSS).unwrap())
}

/// Split Editor (the button at the right end of the focused group's tab
/// bar, View › Split Editor and its key): the tab in front goes to a
/// second group at the right, which has the focus; a second press moves
/// the focused group's front tab into the other group. None with one tab
/// in the group (the boards draw none), nor on the other group's bar. A
/// group emptied goes, the other taking its room; the last tab closed
/// leaves the placeholder.
#[test]
fn split_editor_makes_a_second_group_from_the_front_tab() {
    let mut h = one_group("ui-doc-tab-split", 0, &[hello()], egui::vec2(1400.0, 900.0));
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start(), hello()]]);
    let split = h.get_by_label("Split Editor");
    assert_eq!(split.accesskit_node().role(), Role::Button);
    let bar = (h.state().dock.iter_leaves()).find(|(p, _)| p.surface.is_main()).unwrap().1.rect;
    assert!((bar.right() - split.rect().right()).abs() < 8.0, "at the bar's right end");
    split.click();
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start()], vec![hello()]]);
    assert_eq!(h.state().active_editor(), Some(&hello()), "the focus in the new group");
    // (Half the room each.)
    let widths: Vec<f32> = (h.state().dock.iter_leaves())
        .filter(|(p, _)| p.surface.is_main())
        .map(|(_, l)| l.rect.width())
        .collect();
    assert!((widths[0] - widths[1]).abs() < 4.0, "{widths:?}");
    // The new group has one tab: no Split there, nor on the other's bar.
    assert!(h.query_by_label("Split Editor").is_none());
    // The other group focused: its Split moves its front tab across.
    click_tab(&mut h, &start());
    settle(&mut h);
    menu_command(&mut h, "View", "Split Editor");
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties], vec![hello(), start()]]);
    assert_eq!(h.state().active_editor(), Some(&start()));
    // Its key, from the group of two: back to the first.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Backslash);
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start()], vec![hello()]]);
    // A group emptied goes; the other takes its room.
    let whole = h.state().editor_area_rect().unwrap();
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![hello()]));
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start()]]);
    let leaf = (h.state().dock.iter_leaves()).find(|(p, _)| p.surface.is_main()).unwrap().1.rect;
    assert!(leaf.width() > whole.width() - 20.0, "{leaf:?} in {whole:?}");
    // The last tab closed: the placeholder.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::ModuleProperties, start()]));
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::NoArea]]);
    h.get_by_label("No area open");
    assert!(h.query_by_label("Split Editor").is_none());
}

/// The docks follow the focused group: an area in one, a script in the
/// other, the Right Dock shown and gone as the focus moves; the front
/// tab's line is the accent in the focused group, `line-soft` in the
/// other.
#[test]
fn the_docks_follow_the_focused_editor_group() {
    let Some((mut h, area)) = area_harness("doc-tab-groups") else { return };
    let script = hello();
    h.state_mut().settings.window_places = vec![("script".into(), "middle".into())];
    h.state_mut().actions.push(mg_ui::Action::OpenTab(script.clone()));
    settle(&mut h);
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    mg_uikit::record_icons(&h.ctx);
    let t = mg_uikit::tokens(&h.ctx);
    let area = Tab::Area(area);
    // The front tabs' lines: the accent over the focused group's, `line-soft`
    // over the other's.
    let lines = |h: &mut Harness<'_, Moonglow>, focused: &Tab, other: &Tab| {
        let (f, o) = (focused.clone(), other.clone());
        until(h, "the front tabs' lines", move |h| {
            line_over(h, &f) == Some(t.accent) && line_over(h, &o) == Some(t.line_soft)
        });
    };
    assert_eq!(h.state().active_editor(), Some(&script));
    assert!(h.state().right_dock_rect().is_none(), "no Right Dock beside the script");
    lines(&mut h, &script, &area);
    click_tab(&mut h, &area);
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&area));
    assert!(palette_shown(h.state()), "the palettes beside the area");
    lines(&mut h, &area, &script);
    click_tab(&mut h, &script);
    settle(&mut h);
    assert!(h.state().right_dock_rect().is_none());
    lines(&mut h, &script, &area);
}

/// The tab keys act on the focused group: Ctrl+Tab and Ctrl+Shift+Tab go
/// round its tabs, Ctrl and a digit to one of its tabs, Ctrl+W closes its
/// tab in front and no other group's.
#[test]
fn the_tab_keys_act_on_the_focused_group() {
    tab_keys_per_group(Mode::Floating);
}

/// [`the_tab_keys_act_on_the_focused_group`] where editors open as tabs (a
/// new installation's).
#[test]
fn the_tab_keys_act_on_the_focused_group_as_tabs() {
    tab_keys_per_group(Mode::Tabs);
}

fn tab_keys_per_group(mode: Mode) {
    let mut h = one_group_in(mode, "ui-doc-tab-keys", 3, &[], egui::vec2(1400.0, 900.0));
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    if !mode.tabs() {
        h.state_mut().settings.window_places = vec![("script".into(), "middle".into())];
    }
    for n in 1..=3 {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(s(n)));
        settle(&mut h);
    }
    // s03 to a group of its own, then s01 and s02 after it.
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    for n in [1, 2] {
        click_tab(&mut h, &s(n));
        h.get_by_label("Split Editor").click();
        settle(&mut h);
    }
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start()], vec![s(3), s(1), s(2)]]);
    let front = |h: &Harness<'_, Moonglow>| h.state().active_editor().cloned();
    let key = |h: &mut Harness<'_, Moonglow>, m: egui::Modifiers, k: egui::Key| {
        h.key_press_modifiers(m, k);
        settle(h);
    };
    let ctrl = egui::Modifiers::COMMAND;
    assert_eq!(front(&h), Some(s(2)));
    key(&mut h, ctrl, egui::Key::Tab);
    assert_eq!(front(&h), Some(s(3)), "round its own tabs");
    key(&mut h, ctrl | egui::Modifiers::SHIFT, egui::Key::Tab);
    assert_eq!(front(&h), Some(s(2)));
    key(&mut h, ctrl, egui::Key::Num2);
    assert_eq!(front(&h), Some(s(1)), "its second tab");
    key(&mut h, ctrl, egui::Key::Num9);
    assert_eq!(front(&h), Some(s(2)), "its last");
    key(&mut h, ctrl, egui::Key::W);
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start()], vec![s(3), s(1)]]);
    // The other group focused: its keys there.
    click_tab(&mut h, &Tab::ModuleProperties);
    settle(&mut h);
    key(&mut h, ctrl, egui::Key::Num2);
    assert_eq!(front(&h), Some(start()));
    key(&mut h, ctrl, egui::Key::W);
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties], vec![s(3), s(1)]]);
}

/// Two editor groups are kept in the module's open-tabs record and come
/// back as they were. The record gives each group its place in the form
/// Moonglow 4.1.0 reads (`split right 500`) beside the groups' tree on
/// the middle's pane, which 4.1.0's reader passes over (its `place_pane`
/// reads a pane's `split` only for a window). Read with that tree taken
/// out, by this version's reader (4.1.0 itself is not run here), the two
/// groups come back as they were from their places alone.
#[test]
fn editor_groups_are_kept_and_come_back_from_their_places_alone() {
    groups_kept(Mode::Floating);
}

/// [`editor_groups_are_kept_and_come_back_from_their_places_alone`] where
/// editors open as tabs (a new installation's).
#[test]
fn editor_groups_are_kept_and_come_back_from_their_places_alone_as_tabs() {
    groups_kept(Mode::Tabs);
}

fn groups_kept(mode: Mode) {
    let dir = mg_testkit::scratch_dir(&mode.dir("ui-doc-tab-kept"));
    let path = module_with(&dir, 0);
    let kept = dir.join("open-tabs");
    let open = |kept: &std::path::Path| {
        let mut app = app_in(mode, Vec::new());
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(kept.to_path_buf());
        if !mode.tabs() {
            app.settings.window_places = vec![("script".into(), "middle".into())];
        }
        app.open_module(&path);
        let mut h = harness(app, 1400.0, 900.0);
        settle(&mut h);
        h
    };
    let mut h = open(&kept);
    for tab in [start(), hello()] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        settle(&mut h);
    }
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    let laid = groups(&h);
    assert_eq!(laid, [vec![Tab::ModuleProperties, start()], vec![hello()]]);
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    let file = mg_ui::open_tabs::file(&kept, &path);
    let text = std::fs::read_to_string(&file).unwrap();
    let record: serde_json::Value = serde_json::from_str(&text).unwrap();
    let panes = record["panes"].as_array().unwrap();
    let place = |tab: &str| {
        panes.iter().find(|p| p["tabs"].as_array().unwrap().iter().any(|t| t == tab)).unwrap()
            ["place"]
            .clone()
    };
    assert_eq!(place("script:hello.nss"), "split right 500", "{text}");
    assert_eq!(place("area:start"), "middle", "{text}");
    // Opened again: the two groups.
    let h = open(&kept);
    assert_eq!(groups(&h), laid);
    drop(h);
    // The groups' tree (the middle's pane's `split`) taken out: their
    // places alone, what 4.1.0's reader uses.
    let mut old = record.clone();
    for pane in old["panes"].as_array_mut().unwrap() {
        if pane["place"] == "middle" {
            pane.as_object_mut().unwrap().remove("split");
        }
    }
    std::fs::write(&file, serde_json::to_string(&old).unwrap()).unwrap();
    let h = open(&kept);
    assert_eq!(groups(&h), laid, "from their places alone");
}

/// Three groups in a row come back in their order (the groups' tree on
/// the middle's pane).
#[test]
fn three_editor_groups_come_back_in_their_order() {
    three_groups(Mode::Floating);
}

/// [`three_editor_groups_come_back_in_their_order`] where editors open as
/// tabs (a new installation's). The record is read as in the other mode
/// (its places are kept ones); what this mode adds is where an editor of a
/// kind with no place kept opens once the groups are back: a tab of the
/// focused group (a window of its own in the other).
#[test]
fn three_editor_groups_come_back_in_their_order_as_tabs() {
    three_groups(Mode::Tabs);
}

fn three_groups(mode: Mode) {
    let dir = mg_testkit::scratch_dir(&mode.dir("ui-doc-tab-three"));
    let path = module_with(&dir, 2);
    let kept = dir.join("open-tabs");
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    let open = |kept: &std::path::Path| {
        let mut app = app_in(mode, Vec::new());
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(kept.to_path_buf());
        app.open_module(&path);
        let mut h = harness(app, 1400.0, 900.0);
        settle(&mut h);
        h
    };
    let mut h = open(&kept);
    {
        let dock = &mut h.state_mut().dock;
        let node = dock.find_tab(&Tab::ModuleProperties).unwrap().node;
        let [_, right] = dock.main_surface_mut().split_right(node, 0.4, vec![s(1)]);
        dock.main_surface_mut().split_right(right, 0.5, vec![s(2)]);
    }
    settle(&mut h);
    let laid = groups(&h);
    assert_eq!(laid, [vec![Tab::ModuleProperties], vec![s(1)], vec![s(2)]]);
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    let mut h = open(&kept);
    assert_eq!(groups(&h), laid);
    // An editor of a kind with no place kept: where editors open.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    let main = h.state().dock.find_tab(&Tab::Factions).unwrap().surface.is_main();
    assert_eq!(main, mode.tabs(), "{mode:?}: a tab of the focused group or a window");
    if mode.tabs() {
        let focused = h.state().dock.main_surface().focused_leaf().unwrap();
        assert_eq!(h.state().dock.find_tab(&Tab::Factions).unwrap().node, focused);
    }
}

/// GitHub issues 11 and 20's last part: a tool's group left alone (the
/// Faction Editor split off an area's right, the area and Module
/// Properties closed) is an editor group as any, and takes the whole
/// editor area, the placeholder not shown beside it (the boards' editor
/// area holds editors alone); closed too, the placeholder takes the area.
#[test]
fn a_tool_s_group_left_alone_takes_the_editor_area() {
    let mut h = one_group("ui-doc-tab-alone", 0, &[], egui::vec2(1400.0, 900.0));
    {
        let dock = &mut h.state_mut().dock;
        let node = dock.find_tab(&start()).unwrap().node;
        dock.main_surface_mut().split_right(node, 0.6, vec![Tab::Factions]);
    }
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start()], vec![Tab::Factions]]);
    let whole = h.state().editor_area_rect().unwrap();
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![start(), Tab::ModuleProperties]));
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::Factions]]);
    assert!(h.query_by_label("No area open").is_none());
    let leaf = (h.state().dock.iter_leaves()).find(|(p, _)| p.surface.is_main()).unwrap().1.rect;
    assert!(leaf.width() > whole.width() - 20.0, "the whole area: {leaf:?} {whole:?}");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Factions]));
    settle(&mut h);
    assert_eq!(groups(&h), [vec![Tab::NoArea]]);
    h.get_by_label("No area open");
}

/// The main surface's tab bar: its rect (the group's top to its body).
fn bar_of(h: &Harness<'_, Moonglow>, tab: &Tab) -> egui::Rect {
    let (_, leaf) = (h.state().dock.iter_leaves()).find(|(_, l)| l.tabs.contains(tab)).unwrap();
    egui::Rect::from_x_y_ranges(leaf.rect.x_range(), leaf.rect.top()..=leaf.viewport.top())
}

/// The tab nodes of a bar and the buttons in it, by the accessibility
/// tree: what each says and where it is.
fn bar_nodes(h: &Harness<'_, Moonglow>, bar: egui::Rect) -> Vec<(String, egui::Rect)> {
    let mut out = Vec::new();
    for role in [Role::Tab, Role::Button] {
        for n in h.get_all_by_role(role) {
            let r = n.rect();
            if r.intersects(bar.shrink(1.0))
                && r.top() >= bar.top() - 1.0
                && r.bottom() <= bar.bottom() + 1.0
            {
                out.push((n.accesskit_node().label().unwrap_or_default(), r));
            }
        }
    }
    out
}

/// Thirty tabs in an 800 point window: no tab wider than one cut to the
/// least (these names are shorter; a long one cut, its whole name in its
/// tip: `a_title_cut_short_shows_whole_in_its_tip`); the bar scrolls to
/// the tab in front as another comes to the front, keeps Split Editor in
/// reach, and nothing of it takes the pointer past its ends.
#[test]
fn thirty_tabs_in_a_narrow_window() {
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    let tabs: Vec<Tab> = (1..=30).map(s).collect();
    let mut h = one_group("ui-doc-tab-thirty", 30, &tabs, egui::vec2(800.0, 600.0));
    assert_eq!(groups(&h)[0].len(), 32);
    let bar = bar_of(&h, &s(30));
    // The tab in front in sight, whole, and Split Editor at the bar's end.
    let front = doc_tab(&h, "s30.nss").rect();
    let split = h.get_by_label("Split Editor").rect();
    assert!(bar.contains_rect(split), "{split:?} {bar:?}");
    assert!(front.right() <= split.left() && front.left() >= bar.left(), "{front:?} {split:?}");
    // No wider than a tab cut to its least.
    let width = front.width();
    let least = mg_uikit::shell::doc_tab_width(mg_uikit::dock::DOC_TAB_LEAST_TITLE, false);
    assert!(width <= least + 0.5, "cut: {width} at most {least}");
    // The first tab clicked to the front by its number: the bar scrolls.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Num1);
    settle(&mut h);
    let first = doc_tab(&h, "Module Properties").rect();
    assert!(first.left() >= bar.left() - 0.5 && first.right() <= split.left(), "{first:?}");
    // Nothing of the bar past its ends, nor under Split Editor: the tabs as
    // they take the pointer (cut where the bar cuts them).
    for r in tab_hits(&h, bar) {
        assert!(r.left() >= bar.left() - 0.5 && r.right() <= split.left() + 0.5, "{r:?} {split:?}");
    }
}

/// Where the tabs of a bar take the pointer (their buttons cut to what of
/// them is in sight), from the last frame's widgets: what can be clicked
/// and dragged there.
fn tab_hits(h: &Harness<'_, Moonglow>, bar: egui::Rect) -> Vec<egui::Rect> {
    let mut out = Vec::new();
    h.ctx.viewport(|v| {
        for (_, widgets) in v.prev_pass.widgets.layers() {
            for w in widgets.iter() {
                let r = w.interact_rect;
                if w.sense.senses_click()
                    && w.sense.senses_drag()
                    && r.is_positive()
                    && (r.center().y - bar.center().y).abs() < 2.0
                    && r.height() < bar.height() + 1.0
                {
                    out.push(r);
                }
            }
        }
    });
    out
}

/// A long name cut short in a crowded bar shows whole in its tab's tip,
/// and only there: its tab is still named by it whole.
#[test]
fn a_title_cut_short_shows_whole_in_its_tip() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-cut");
    let path = module_with(&dir, 0);
    let mut m = Module::open(&path).unwrap();
    let long = ResKey::parse("a_long_name_here", ResType::NSS).unwrap();
    m.set(long, b"void main() { }\n".to_vec());
    for i in 1..=12 {
        m.set(
            ResKey::parse(&format!("other_name_{i:02}"), ResType::NSS).unwrap(),
            b"void main() { }\n".to_vec(),
        );
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.window_places = vec![("script".into(), "middle".into())];
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(long)));
    for i in 1..=12 {
        let key = ResKey::parse(&format!("other_name_{i:02}"), ResType::NSS).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    }
    let mut h = harness(app, 800.0, 600.0);
    settle(&mut h);
    // (Brought to the front again: the bar scrolls to it.)
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(long)));
    settle(&mut h);
    let tab = doc_tab(&h, "a_long_name_here.nss").rect();
    let bar = bar_of(&h, &Tab::Script(long));
    assert!(bar.contains_rect(tab), "scrolled to: {tab:?} {bar:?}");
    let whole = mg_uikit::shell::doc_tab_width(140.0, false);
    assert!(tab.width() < whole, "cut short: {tab:?}");
    // (The tip says the name the tab is named by: two of that name then.)
    let named = |h: &Harness<'_, Moonglow>| h.query_all_by_label("a_long_name_here.nss").count();
    assert_eq!(named(&h), 1);
    h.hover_at(tab.min + egui::vec2(36.0, 14.0));
    let started = std::time::Instant::now();
    while named(&h) < 2 && started.elapsed().as_secs() < 10 {
        h.step();
    }
    assert_eq!(named(&h), 2, "its whole name in its tip");
    // A tab not cut has none.
    let module = doc_tab(&h, "Module Properties").rect();
    assert!(module.width() < mg_uikit::shell::doc_tab_width(110.0, false), "all cut alike");
}

/// A crowded bar shows both ends of a name it cuts (the owner's ruling of
/// 2026-11-14, "Tab cut in the middle to keep"): the tab's words are its
/// name's start, an ellipsis and its name's end with the extension whole
/// ("lo…nd.nss" in its least room); the tab is still
/// named by its whole name.
#[test]
fn a_crowded_bar_shows_both_ends_of_a_name_it_cuts() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-middle-cut");
    let path = module_with(&dir, 0);
    let mut m = Module::open(&path).unwrap();
    let names: Vec<String> = (1..=12).map(|i| format!("long_name{i:02}_end")).collect();
    for n in &names {
        m.set(ResKey::parse(n, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.window_places = vec![("script".into(), "middle".into())];
    app.open_module(&path);
    for n in &names {
        let key = ResKey::parse(n, ResType::NSS).unwrap();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    }
    let mut h = harness(app, 800.0, 600.0);
    settle(&mut h);
    let words: Vec<(String, egui::Rect)> = drawn(&h)
        .into_iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => {
                Some((t.galley.text().to_string(), t.galley.rect.translate(t.pos.to_vec2())))
            }
            _ => None,
        })
        .collect();
    let last = names.last().unwrap();
    let tab = doc_tab(&h, &format!("{last}.nss")).rect();
    let (shown, _) = words
        .iter()
        .find(|(_, r)| tab.contains(r.center()) && r.width() > 10.0)
        .unwrap_or_else(|| panic!("no words in the tab {tab:?}: {words:?}"));
    assert!(shown.contains('…'), "cut: {shown}");
    assert!(shown.starts_with("lo"), "its start: {shown}");
    assert!(shown.ends_with("d.nss"), "its end and extension: {shown}");
    assert!(shown.chars().count() < last.len() + 4, "shorter: {shown}");
}

/// From the narrowest editor area (the 420 point window with the Right
/// Dock unfolded by hand, the Left Dock folded by the width: the yields
/// keep both from standing side by side under 962, M3 T14) to 1600
/// points, both docks open from 962 (the Left Dock unfolded by hand too
/// under 1500), with 1, 3 and 12 tabs: every tab and button of the
/// group's tab bar inside the bar or cut by it, none over another but a
/// close button in its own tab, and Split Editor where it is reached.
#[test]
fn the_tab_bar_holds_at_every_width() {
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    for count in [1usize, 3, 12] {
        // (Module Properties and the area are two tabs of their own.)
        let tabs: Vec<Tab> = (1..=count.saturating_sub(2)).map(s).collect();
        let mut h =
            one_group(&format!("ui-doc-tab-sweep-{count}"), 12, &tabs, egui::vec2(420.0, 600.0));
        if count == 1 {
            h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::ModuleProperties]));
            settle(&mut h);
        }
        // (The Right Dock asked for by name, as Tools › Palettes does,
        // without the game's data: unfolded by hand in the least window.)
        h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Palette));
        settle(&mut h);
        assert_eq!(groups(&h)[0].len(), count.max(1));
        let mut width = 420.0;
        while width <= 1600.0 {
            h.set_size(egui::vec2(width, 600.0));
            settle(&mut h);
            // (From 962, `shell::BOTH_DOCKS_UNDER`, both side docks fit
            // beside the editor area's least: the Left Dock unfolded too.)
            let both = width >= 962.0;
            if both {
                unfold_left_dock(&mut h);
                settle(&mut h);
            }
            assert!(h.state().right_dock_rect().is_some(), "{count} at {width}: the Right Dock");
            let left = h.state().left_dock_shows().is_some();
            assert_eq!(left, both, "{count} at {width}: the Left Dock");
            let front = groups(&h)[0].last().cloned().unwrap();
            let bar = bar_of(&h, &front);
            let right = h.state().right_dock_rect().unwrap();
            assert!(bar.right() <= right.left() + 0.5, "{count} at {width}: {bar:?} {right:?}");
            if let Some(left) = h.state().left_dock_rect().filter(|_| both) {
                let beside = bar.left() >= left.right() - 0.5;
                assert!(beside, "{count} at {width}: {bar:?} beside {left:?}");
            }
            // The tabs, as they take the pointer, between the bar's left
            // end and the room kept for Split Editor at its right.
            // (`tabs::SPLIT_ROOM`: a 22 point button and 3 at each side.)
            let end = bar.right() - 28.0;
            for r in tab_hits(&h, bar) {
                assert!(
                    r.left() >= bar.left() - 0.5 && r.right() <= end + 0.5,
                    "{count} at {width}: a tab at {r:?} in {bar:?}"
                );
            }
            let nodes = bar_nodes(&h, bar);
            for (name, r) in &nodes {
                // (A tab's node is its whole button, cut by the bar's ends
                // where it scrolls: `tab_hits` holds what takes the pointer
                // to the bar. Every node is within the bar's height.)
                assert!(
                    r.top() >= bar.top() - 0.5 && r.bottom() <= bar.bottom() + 0.5,
                    "{count} at {width}: {name} {r:?} {bar:?}"
                );
                // A button (a close, Split Editor) wholly inside the bar.
                if name.starts_with("Close ") || name == "Split Editor" {
                    assert!(
                        r.left() >= bar.left() - 0.5 && r.right() <= bar.right() + 0.5,
                        "{count} at {width}: {name} {r:?} past the bar {bar:?}"
                    );
                }
            }
            // (The buttons among themselves: the tabs' nodes are their
            // whole buttons, cut where the bar cuts them, which `tab_hits`
            // holds to the bar above.)
            let nodes: Vec<(String, egui::Rect)> = nodes
                .into_iter()
                .filter(|(n, _)| n.starts_with("Close ") || n == "Split Editor")
                .collect();
            let hits = tab_hits(&h, bar);
            for (name, r) in &nodes {
                let at = r.center();
                let tabs_at = hits.iter().filter(|t| t.contains(at)).count();
                assert!(
                    tabs_at <= usize::from(name.starts_with("Close ")),
                    "{count} at {width}: {name} {r:?} under {tabs_at} tabs"
                );
            }
            for (i, (a, ra)) in nodes.iter().enumerate() {
                for (b, rb) in &nodes[i + 1..] {
                    let both = ra.intersect(*rb).intersect(bar);
                    assert!(
                        !both.is_positive() || both.area() < 1.0,
                        "{count} at {width}: {a} {ra:?} over {b} {rb:?}"
                    );
                }
            }
            let split = nodes.iter().find(|(n, _)| n == "Split Editor");
            assert_eq!(split.is_some(), count > 1 && bar.width() >= 28.0, "{count} at {width}");
            if let Some((_, r)) = split {
                assert!(r.left() >= end - 0.5 && r.right() <= bar.right() + 0.5, "{r:?} {bar:?}");
            }
            width += 60.0;
        }
    }
}

/// A floating window's tabs have the same look as the groups': each named
/// for its name, its front tab's close button named for it, its kind's
/// icon; the window's Maximize still at its bar's end and no Split Editor
/// there (the Floating board draws none).
#[test]
fn a_floating_window_s_tabs_look_as_the_groups() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-window");
    let path = module_with(&dir, 1);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    for key in ["hello", "s01"] {
        let tab = Tab::Script(ResKey::parse(key, ResType::NSS).unwrap());
        app.actions.push(mg_ui::Action::OpenTab(tab));
    }
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    mg_uikit::record_icons(&h.ctx);
    let script = Tab::Script(ResKey::parse("s01", ResType::NSS).unwrap());
    until(&mut h, "the window's tab drawn and its line", |h| line_over(h, &script).is_some());
    let path = h.state().dock.find_tab(&script).unwrap();
    assert!(!path.surface.is_main(), "in a window of its own");
    let tab = doc_tab(&h, "s01.nss");
    assert_eq!(tab.accesskit_node().is_selected(), Some(true));
    let rect = tab.rect();
    assert!(rect.contains_rect(h.get_by_label("Close s01.nss").rect()));
    let drawn = mg_uikit::icons_drawn(&h.ctx);
    assert!(drawn.iter().any(|(i, r)| *i == mg_uikit::Icon::Script && rect.contains_rect(*r)));
    // The window has the focus: its front tab's line the accent, the main
    // group's front tab's `line-soft` (the Floating board).
    let t = mg_uikit::tokens(&h.ctx);
    assert_eq!(line_over(&h, &script), Some(t.accent));
    assert_eq!(line_over(&h, &Tab::ModuleProperties), Some(t.line_soft));
    let bar = bar_of(&h, &script);
    let maximize = h.get_by_label("Maximize").rect();
    assert!(bar.contains_rect(maximize));
    let split = h.query_by_label("Split Editor").map(|n| n.rect());
    assert!(split.is_none_or(|r| !bar.intersects(r)), "no Split Editor on a window's bar");
}

/// By hand (`--ignored --nocapture`, in release, on a quiet machine): what
/// a frame of the window costs with thirty tabs in one group, against two
/// (the document tabs' titles laid out from a cache, their parts painted,
/// every frame). Prints the medians in microseconds.
#[test]
#[ignore]
fn thirty_tabs_frame_cost() {
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    let median = |tabs: &[Tab]| {
        let mut h = one_group("ui-doc-tab-cost", 30, tabs, egui::vec2(1600.0, 900.0));
        for _ in 0..30 {
            h.step();
        }
        let mut times: Vec<std::time::Duration> = (0..300)
            .map(|_| {
                let at = std::time::Instant::now();
                h.step();
                at.elapsed()
            })
            .collect();
        times.sort();
        times[times.len() / 2].as_secs_f64() * 1e6
    };
    let two = median(&[]);
    let thirty = median(&(1..=30).map(s).collect::<Vec<_>>());
    println!("frame with 2 tabs: {two:.0} us; with 32 tabs: {thirty:.0} us");
}

/// A kind kept off a side of the middle (`split right 500`, as a group
/// of scripts left by Split Editor) opens in the group there, one group
/// for all of them, not one more split each (GitHub issue 22's place, for
/// editor groups).
#[test]
fn a_kind_kept_beside_the_middle_opens_in_that_group() {
    kept_beside(Mode::Floating);
}

/// [`a_kind_kept_beside_the_middle_opens_in_that_group`] where editors
/// open as tabs (a new installation's): a kept place goes before the
/// focused group (the scripts in the group kept for them), and a kind with
/// no place kept (the Faction Editor) opens where the focus is, the
/// scripts' group (a window of its own in the other mode).
#[test]
fn a_kind_kept_beside_the_middle_opens_in_that_group_as_tabs() {
    kept_beside(Mode::Tabs);
}

fn kept_beside(mode: Mode) {
    let mut h = one_group_in(mode, "ui-doc-tab-kept-kind", 2, &[], egui::vec2(1400.0, 900.0));
    h.state_mut().settings.window_places = vec![("script".into(), "split right 500".into())];
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    for n in [1, 2] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(s(n)));
        settle(&mut h);
    }
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start()], vec![s(1), s(2)]]);
    // A kind with no place kept: where editors open (the focus is in the
    // scripts' group, where the last opened).
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    match mode.tabs() {
        true => assert_eq!(groups(&h)[1], [s(1), s(2), Tab::Factions], "the focused group"),
        false => {
            let at = h.state().dock.find_tab(&Tab::Factions).unwrap();
            assert!(!at.surface.is_main(), "a window of its own");
        }
    }
}

/// A tab that asks before it closes (a 2DA with unsaved changes), closed
/// by its close button from behind another: it comes to the front while
/// the question is asked of it, as egui_dock's close did.
#[test]
fn a_tab_that_asks_comes_to_the_front_when_closed_from_behind() {
    tab_that_asks(Mode::Floating);
}

/// [`a_tab_that_asks_comes_to_the_front_when_closed_from_behind`] where
/// editors open as tabs (a new installation's), the 2DA a tab where it
/// opens.
#[test]
fn a_tab_that_asks_comes_to_the_front_when_closed_from_behind_as_tabs() {
    tab_that_asks(Mode::Tabs);
}

fn tab_that_asks(mode: Mode) {
    let mut h = one_group_in(mode, "ui-doc-tab-asks", 0, &[], egui::vec2(1400.0, 900.0));
    if !mode.tabs() {
        h.state_mut().settings.window_places = vec![("table".into(), "middle".into())];
    }
    let key = ResKey::parse("mytable", ResType::TWODA).unwrap();
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    settle(&mut h);
    let table = Tab::Table(id);
    assert!(groups(&h)[0].contains(&table), "{:?}", groups(&h));
    h.state_mut().tables.iter_mut().find(|t| t.id == id).unwrap().edit.add_rows(1);
    click_tab(&mut h, &start());
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&start()));
    click_tab_close(&mut h, &table);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&table).is_some(), "asks first");
    assert_eq!(h.state().active_editor(), Some(&table), "in front while asked");
    // Its Cancel: the tab stays, its change kept.
    h.get_all_by_label("Cancel").last().unwrap().click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&table).is_some(), "Cancel: kept");
    assert!(h.query_by_label("Don't Save").is_none(), "no longer asked");
    click_tab_close(&mut h, &table);
    settle(&mut h);
    h.get_all_by_label("Don't Save").last().unwrap().click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&table).is_none());
}

/// By hand (`--ignored --nocapture`, in release, with the game): what the
/// modified dot's question costs an area's tab after each change of it,
/// on the largest area of the game's modules (its GIT and ARE changed,
/// not flushed: written out and compared with the saved copy). Prints
/// the medians in microseconds.
#[test]
#[ignore]
fn an_area_s_dot_cost() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let mut best: Option<(usize, std::path::PathBuf, ResRef)> = None;
    for entry in std::fs::read_dir(root.join("data/nwm")).unwrap().flatten() {
        let path = entry.path();
        let Ok(m) = Module::open(&path) else { continue };
        for key in m.keys_of(ResType::GIT) {
            let len = m.get(key).map_or(0, <[u8]>::len);
            if m.contains(&ResKey::new(key.resref, ResType::ARE))
                && best.as_ref().is_none_or(|b| len > b.0)
            {
                best = Some((len, path.clone(), key.resref));
            }
        }
    }
    let (len, path, area) = best.unwrap();
    let mut ws = mg_edit::Workspace::new(Module::open(&path).unwrap());
    // (The worst place for the comparison: a field of the last object of
    // the longest list, after every other.)
    let git = Gff::read(ws.module.get(&ResKey::new(area, ResType::GIT)).unwrap()).unwrap();
    let (list, n) = ["Creature List", "Placeable List", "WaypointList", "Door List", "SoundList"]
        .iter()
        .map(|l| (*l, git.root.list(l).map_or(0, |v| v.len())))
        .max_by_key(|(_, n)| *n)
        .unwrap();
    let last = mg_edit::GffPath::root().item(list, n - 1);
    let mut median = |t: ResType, path: mg_edit::GffPath, label: &str| {
        let key = ResKey::new(area, t);
        let mut times = Vec::new();
        for i in 0..21 {
            let edit = mg_edit::Edit::SetField {
                key,
                path: path.clone(),
                label: label.into(),
                value: Some(if label == "Tag" {
                    mg_gff::Value::String(format!("T{i}").into())
                } else {
                    mg_gff::Value::Int(i)
                }),
            };
            ws.apply(mg_edit::Command::new("x", vec![edit])).unwrap();
            let at = std::time::Instant::now();
            assert!(ws.differs_from_saved(&key));
            times.push(at.elapsed());
        }
        times.sort();
        times[times.len() / 2].as_secs_f64() * 1e6
    };
    let deep = median(ResType::GIT, last, "Tag");
    let root = median(ResType::GIT, mg_edit::GffPath::root(), "MgProbe");
    let are = median(ResType::ARE, mg_edit::GffPath::root(), "Tag");
    println!(
        "{area} in {} (GIT {len} bytes, {n} in its {list}): GIT's last object {deep:.1} us, \
         GIT's root {root:.1} us, ARE {are:.1} us",
        path.display()
    );
}

/// A 2DA's look for its description, under way as its tab goes behind
/// another, is taken in all the same: the window's work ends. (It was
/// taken in only while the tab was drawn, so `settle` waited for "a
/// table's description's folder being read" for good: what made
/// `a_tab_that_asks_comes_to_the_front_when_closed_from_behind` fail now
/// and then, where the folder's walk was slow on a first run.)
#[test]
fn a_table_s_look_ends_with_its_tab_behind_another() {
    let mut h = one_group("ui-doc-tab-look", 0, &[], egui::vec2(1400.0, 900.0));
    h.state_mut().settings.window_places = vec![("table".into(), "middle".into())];
    let hold = h.state().hold_description_looks.clone();
    hold.store(true, std::sync::atomic::Ordering::Relaxed);
    let key = ResKey::parse("mytable", ResType::TWODA).unwrap();
    let id = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    let walking = |h: &Harness<'_, Moonglow>| h.state().tables.iter().any(|t| t.walking());
    let started = std::time::Instant::now();
    while !walking(&h) && started.elapsed().as_secs() < 10 {
        h.step();
    }
    assert!(walking(&h), "its look under way");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(start()));
    let started = std::time::Instant::now();
    while h.state().active_editor() != Some(&start()) && started.elapsed().as_secs() < 10 {
        h.step();
    }
    assert_ne!(h.state().active_editor(), Some(&Tab::Table(id)), "the table behind the area");
    hold.store(false, std::sync::atomic::Ordering::Relaxed);
    settle(&mut h);
    assert!(!walking(&h));
}

/// The sample module with its area `start`'s placed objects (a GIT with a
/// placeable) and their comments (a GIC with the placeable's), open with
/// the area's tab in front.
fn area_with_objects(name: &str) -> Harness<'static, Moonglow> {
    use mg_gff::{Struct, Value};
    let dir = mg_testkit::scratch_dir(name);
    let path = module_with(&dir, 0);
    let mut m = Module::open(&path).unwrap();
    let placeable = |fields: &[(&str, Value)]| {
        let mut s = Struct::new(9);
        for (label, value) in fields {
            s.set(label, value.clone());
        }
        s
    };
    let mut git = Gff::new(*b"GIT ");
    let at = [("XPosition", Value::Float(1.0)), ("Tag", Value::String("CHEST".into()))];
    git.root.set("Placeable List", Value::List(vec![placeable(&at)]));
    m.set(ResKey::parse("start", ResType::GIT).unwrap(), git.to_bytes().unwrap());
    let mut gic = Gff::new(*b"GIC ");
    let comment = [("Comment", Value::String("a chest".into()))];
    gic.root.set("Placeable List", Value::List(vec![placeable(&comment)]));
    m.set(ResKey::parse("start", ResType::GIC).unwrap(), gic.to_bytes().unwrap());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(start()));
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    h
}

/// An area's dot covers its placed objects and their comments, not its
/// ARE alone: an object moved (the GIT), an object added (the GIT), a
/// comment changed (the GIC alone); each undone back to what is saved,
/// and each saved with the module.
#[test]
fn an_area_s_dot_covers_its_objects_and_their_comments() {
    use mg_edit::{Command, Edit, GffPath};
    let mut h = area_with_objects("ui-doc-tab-area-objects");
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    let gic = ResKey::parse("start", ResType::GIC).unwrap();
    let first = GffPath::root().item("Placeable List", 0);
    let set = |key, path: &GffPath, label: &str, value| Edit::SetField {
        key,
        path: path.clone(),
        label: label.into(),
        value: Some(value),
    };
    let edits = [
        ("an object moved", set(git, &first, "XPosition", mg_gff::Value::Float(5.0))),
        (
            "an object added",
            Edit::InsertItem {
                key: git,
                path: GffPath::root(),
                list: "Placeable List".into(),
                index: 1,
                item: mg_gff::Struct::new(9),
            },
        ),
        ("a comment changed", set(gic, &first, "Comment", mg_gff::Value::String("x".into()))),
    ];
    assert!(!said_modified(&h, "start"));
    for (what, edit) in edits {
        let apply = |h: &mut Harness<'_, Moonglow>| {
            let ws = h.state_mut().ws.as_mut().unwrap();
            ws.apply(Command::new(what, vec![edit.clone()])).unwrap();
        };
        apply(&mut h);
        until(&mut h, &format!("{what}: the dot"), |h| said_modified(h, "start"));
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        until(&mut h, &format!("{what}, undone: no dot"), |h| !said_modified(h, "start"));
        apply(&mut h);
        until(&mut h, &format!("{what} again: the dot"), |h| said_modified(h, "start"));
        h.state_mut().actions.push(mg_ui::Action::Save);
        settle(&mut h);
        assert!(!said_modified(&h, "start"), "{what}, saved: no dot");
    }
}

/// The dot shows at once as its document changes, and the comparison with
/// the saved copy (up to a millisecond on a large area) waits until the
/// document has rested with no button held: none across a drag of many
/// frames, one after it; and an edit undone back to what is saved loses
/// its dot with no input after (the window asks for that frame itself).
#[test]
fn the_dot_s_comparison_waits_for_the_document_to_rest() {
    use mg_edit::{Command, Edit, GffPath};
    let mut h = area_with_objects("ui-doc-tab-area-rest");
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    let first = GffPath::root().item("Placeable List", 0);
    let move_to = |h: &mut Harness<'_, Moonglow>, x: f32| {
        let edit = Edit::SetField {
            key: git,
            path: first.clone(),
            label: "XPosition".into(),
            value: Some(mg_gff::Value::Float(x)),
        };
        h.state_mut().ws.as_mut().unwrap().apply(Command::new("Move", vec![edit])).unwrap();
    };
    let compared = |h: &Harness<'_, Moonglow>| h.state().dot_comparisons();
    let before = compared(&h);
    // A drag: the button held over the area's view, the object moved a
    // frame at a time.
    let view = h.state().editor_area_rect().unwrap().center();
    h.hover_at(view);
    h.event(egui::Event::PointerButton {
        pos: view,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    for i in 0..40 {
        move_to(&mut h, 2.0 + i as f32);
        h.event(egui::Event::PointerMoved(view + egui::vec2(i as f32, 0.0)));
        // (One frame for each step of the drag.)
        h.step();
        assert!(said_modified(&h, "start"), "the dot at once, frame {i}");
    }
    assert_eq!(compared(&h), before, "no comparison during the drag");
    // Held still, the button down (a drag paused): still none.
    // (Frames for a second of the window's clock, past the rest asked.)
    let paused = h.ctx.input(|i| i.time);
    while h.ctx.input(|i| i.time) < paused + 1.0 {
        h.step();
    }
    assert_eq!(compared(&h), before, "none while the button is held");
    h.event(egui::Event::PointerButton {
        pos: view + egui::vec2(40.0, 0.0),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut h);
    assert_eq!(compared(&h), before + 1, "one comparison once it rested");
    assert!(said_modified(&h, "start"));
    // Back to what is saved: the dot goes by itself (the frames are the
    // window's own: `settle` gives no input).
    move_to(&mut h, 1.0);
    // (Two frames: the change seen, then the frame it asked for, made for
    // the dot's sake.)
    h.step();
    h.step();
    let asked = h.ctx.repaint_causes();
    assert!(asked.iter().any(|c| c.file.ends_with("tabs.rs")), "a frame asked for: {asked:?}");
    settle(&mut h);
    assert!(!said_modified(&h, "start"), "the dot gone once compared");
    assert_eq!(compared(&h), before + 2);
}
