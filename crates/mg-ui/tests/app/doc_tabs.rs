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
    h.run_steps(3);
    h.event(egui::Event::Text("// one\n".into()));
    h.run_steps(3);
    assert!(said_modified(&h, "hello.nss"));
    assert!(h.query_by_label("hello.nss *").is_none(), "no star in its name");
    let starred = h
        .get_all_by_role(Role::Tab)
        .filter(|n| n.accesskit_node().label().is_some_and(|l| l.contains('*')))
        .count();
    assert_eq!(starred, 0, "no tab named with a star");
    // Saved into the module (the editor's Ctrl+S): the dot goes.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run_steps(3);
    assert!(!said_modified(&h, "hello.nss"), "saved");

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
    h.run_steps(2);
    assert!(said_modified(&h, "start"));
    assert!(h.state().title().ends_with(" *"), "the window's title keeps its star");
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(2);
    assert!(!said_modified(&h, "start"), "undone back to what is saved");
    // Changed and saved with the module.
    h.state_mut().ws.as_mut().unwrap().apply(mg_edit::Command::new("Tag", vec![tag("X")])).unwrap();
    h.run_steps(2);
    assert!(said_modified(&h, "start"));
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
    h.run_steps(2);
    assert!(said_modified(&h, "mytable.2da"));
    assert!(mg_ui::table_view::save(h.state_mut(), id));
    h.run_steps(2);
    assert!(!said_modified(&h, "mytable.2da"));

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
    h.run_steps(2);
    assert!(!said_modified(&h, "Talk Table"));
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
    h.run_steps(2);
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
    h.run_steps(2);
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
    h.run_steps(1);
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
    h.run_steps(2);
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
    let dir = mg_testkit::scratch_dir(name);
    let path = module_with(&dir, scripts);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.window_places = ["script", "manual", "journal"]
        .iter()
        .map(|k| (k.to_string(), "middle".to_string()))
        .collect();
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
    assert_eq!(h.state().active_editor(), Some(&script));
    assert!(h.state().right_dock_rect().is_none(), "no Right Dock beside the script");
    click_tab(&mut h, &Tab::Area(area));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Area(area)));
    assert!(palette_shown(h.state()), "the palettes beside the area");
    click_tab(&mut h, &script);
    settle(&mut h);
    assert!(h.state().right_dock_rect().is_none());
}

/// The tab keys act on the focused group: Ctrl+Tab and Ctrl+Shift+Tab go
/// round its tabs, Ctrl and a digit to one of its tabs, Ctrl+W closes its
/// tab in front and no other group's.
#[test]
fn the_tab_keys_act_on_the_focused_group() {
    let mut h = one_group("ui-doc-tab-keys", 3, &[], egui::vec2(1400.0, 900.0));
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    h.state_mut().settings.window_places = vec![("script".into(), "middle".into())];
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
/// back as they were; the record says each group's place as Moonglow
/// 4.1.0 reads it (`split right 500`), and read without the groups' tree
/// (as 4.1.0 does), the two groups still come back.
#[test]
fn editor_groups_are_kept_with_the_module_s_tabs() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-kept");
    let path = module_with(&dir, 0);
    let kept = dir.join("open-tabs");
    let open = |kept: &std::path::Path| {
        let mut app = app_with(Vec::new());
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(kept.to_path_buf());
        app.settings.window_places = vec![("script".into(), "middle".into())];
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
    // Read as 4.1.0 reads it: the groups' tree (its `split` on the
    // middle's pane) left out.
    let mut old = record.clone();
    for pane in old["panes"].as_array_mut().unwrap() {
        if pane["place"] == "middle" {
            pane.as_object_mut().unwrap().remove("split");
        }
    }
    std::fs::write(&file, serde_json::to_string(&old).unwrap()).unwrap();
    let h = open(&kept);
    assert_eq!(groups(&h), laid, "the oldest form");
}

/// Three groups in a row come back in their order (the groups' tree on
/// the middle's pane).
#[test]
fn three_editor_groups_come_back_in_their_order() {
    let dir = mg_testkit::scratch_dir("ui-doc-tab-three");
    let path = module_with(&dir, 2);
    let kept = dir.join("open-tabs");
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    let open = |kept: &std::path::Path| {
        let mut app = app_with(Vec::new());
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
    let h = open(&kept);
    assert_eq!(groups(&h), laid);
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

/// Thirty tabs in an 800 point window: the titles are cut alike to their
/// least, each one's whole name in its tip; the bar scrolls to the tab in
/// front as another comes to the front, keeps Split Editor in reach, and
/// draws nothing past its ends.
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
    // Cut: a tab at its least, its whole name in its tip.
    let width = front.width();
    let least = mg_uikit::shell::doc_tab_width(mg_uikit::dock::DOC_TAB_LEAST_TITLE, false);
    assert!(width <= least + 0.5, "cut: {width} at most {least}");
    // (A tab's tip is its whole name: these names are short, so a long
    // one is given and its tab's tip read.)
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

/// From the narrowest editor area (the 420 point window with both docks
/// open) to 1600 points, with 1, 3 and 12 tabs: every tab and button of
/// the group's tab bar inside the bar or cut by it, none over another but
/// a close button in its own tab, and Split Editor where it is reached.
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
        // (Both docks open: the Right Dock asked for by name, as Tools ›
        // Palettes does, without the game's data.)
        h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Palette));
        settle(&mut h);
        assert_eq!(groups(&h)[0].len(), count.max(1));
        let mut width = 420.0;
        while width <= 1600.0 {
            h.set_size(egui::vec2(width, 600.0));
            settle(&mut h);
            assert!(h.state().right_dock_rect().is_some() && h.state().left_dock_shows().is_some());
            let front = groups(&h)[0].last().cloned().unwrap();
            let bar = bar_of(&h, &front);
            let right = h.state().right_dock_rect().unwrap();
            assert!(bar.right() <= right.left() + 0.5, "{count} at {width}: {bar:?} {right:?}");
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
                let shown = r.intersect(bar);
                // (Cut by the bar's ends where it scrolls; never past its
                // top or foot.)
                assert!(
                    r.top() >= bar.top() - 0.5 && r.bottom() <= bar.bottom() + 0.5,
                    "{count} at {width}: {name} {r:?} {bar:?}"
                );
                assert!(!shown.is_positive() || shown.width() <= bar.width(), "{name}");
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
    h.run_steps(2);
    let script = Tab::Script(ResKey::parse("s01", ResType::NSS).unwrap());
    let path = h.state().dock.find_tab(&script).unwrap();
    assert!(!path.surface.is_main(), "in a window of its own");
    let tab = doc_tab(&h, "s01.nss");
    assert_eq!(tab.accesskit_node().is_selected(), Some(true));
    let rect = tab.rect();
    assert!(rect.contains_rect(h.get_by_label("Close s01.nss").rect()));
    let drawn = mg_uikit::icons_drawn(&h.ctx);
    assert!(drawn.iter().any(|(i, r)| *i == mg_uikit::Icon::Script && rect.contains_rect(*r)));
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
    let mut h = one_group("ui-doc-tab-kept-kind", 2, &[], egui::vec2(1400.0, 900.0));
    h.state_mut().settings.window_places = vec![("script".into(), "split right 500".into())];
    let s = |n: usize| Tab::Script(ResKey::parse(&format!("s{n:02}"), ResType::NSS).unwrap());
    for n in [1, 2] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(s(n)));
        settle(&mut h);
    }
    assert_eq!(groups(&h), [vec![Tab::ModuleProperties, start()], vec![s(1), s(2)]]);
}

/// A tab that asks before it closes (a 2DA with unsaved changes), closed
/// by its close button from behind another: it comes to the front while
/// the question is asked of it, as egui_dock's close did.
#[test]
fn a_tab_that_asks_comes_to_the_front_when_closed_from_behind() {
    let mut h = one_group("ui-doc-tab-asks", 0, &[], egui::vec2(1400.0, 900.0));
    h.state_mut().settings.window_places = vec![("table".into(), "middle".into())];
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
