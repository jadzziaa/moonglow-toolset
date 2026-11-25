//! A script's Bottom Dock panels and Search Results (M4's U13; the Script
//! and Floating boards): Compiler and Bookmarks, a script's own, with
//! their switches after the Log's and Search Results' after them; the
//! panels following the script in front, docked, and in a floating
//! script's window's bottom pane; a search's hits and a click opening
//! the line; the live error as the Status Bar's message and a wavy
//! underline on its line; the cursor's line and column; the window coming
//! to rest; and records from before reading without a fault.

use crate::common::*;
use mg_ui::Panel;
use mg_ui::search_results::Hit;

/// A module with the scripts `scripts` (name and text), open in `mode`
/// in a window `width` by 800, the scripts' tabs opened in their order
/// (the last in front).
fn scripts_window(
    mode: Mode,
    name: &str,
    scripts: &[(&str, &str)],
    width: f32,
) -> (Harness<'static, Moonglow>, Vec<ResKey>) {
    let bytes: Vec<(&str, &[u8])> = scripts.iter().map(|(n, t)| (*n, t.as_bytes())).collect();
    scripts_window_with(mode, name, &bytes, width, None)
}

/// [`scripts_window`] of the scripts' bytes, on the game installed at
/// `game` where given (the check as you type reads its `nwscript.nss`).
fn scripts_window_with(
    mode: Mode,
    name: &str,
    scripts: &[(&str, &[u8])],
    width: f32,
    game: Option<&std::path::Path>,
) -> (Harness<'static, Moonglow>, Vec<ResKey>) {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let keys: Vec<ResKey> =
        scripts.iter().map(|(n, _)| ResKey::parse(n, ResType::NSS).unwrap()).collect();
    for (key, (_, text)) in keys.iter().zip(scripts) {
        m.set(*key, text.to_vec());
    }
    m.save().unwrap();
    let mut app = match game {
        Some(root) => game_app(root, None),
        None => app_with(Vec::new()),
    };
    app.settings = mode.settings();
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, width, 800.0);
    settle(&mut h);
    for key in &keys {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(*key)));
        settle(&mut h);
    }
    (h, keys)
}

/// The Status Bar's switches' names, left to right.
fn switches(h: &Harness<'_, Moonglow>) -> Vec<String> {
    let names = ["Log", "Problems", "Compiler", "Bookmarks", "Search Results"];
    let bar = h.get_by_label("Status Bar");
    let mut shown: Vec<(f32, String)> = names
        .iter()
        .filter_map(|n| bar.query_by_label(n).map(|s| (s.rect().left(), n.to_string())))
        .collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0));
    shown.into_iter().map(|s| s.1).collect()
}

/// Clicks the Status Bar's switch `name` and settles.
fn switch(h: &mut Harness<'_, Moonglow>, name: &str) {
    h.get_by_label("Status Bar").get_by_label(name).click();
    settle(h);
}

/// The cursor of `key`'s editor (characters: where it is, where the
/// selection began).
fn cursor_of(h: &Harness<'_, Moonglow>, key: ResKey) -> Option<(usize, usize)> {
    let state = egui::text_edit::TextEditState::load(&h.ctx, egui::Id::new(("script", key)))?;
    let r = state.cursor.char_range()?;
    Some((r.primary.index.into(), r.secondary.index.into()))
}

/// Where `node` is drawn, if it is: its own place, or a group's (which
/// has none of its own) round the places of what it holds.
fn place(node: &egui_kittest::Node<'_>) -> Option<egui::Rect> {
    fn of(n: &egui_kittest::kittest::AccessKitNode<'_>) -> Option<egui::Rect> {
        if let Some(r) = n.bounding_box() {
            let at = |x: f64, y: f64| egui::pos2(x as f32, y as f32);
            return Some(egui::Rect::from_min_max(at(r.x0, r.y0), at(r.x1, r.y1)));
        }
        n.children().filter_map(|c| of(&c)).reduce(|a, b| a.union(b))
    }
    of(&node.accesskit_node())
}

/// The tab bar of `key`'s window's bottom pane (a floating script's).
fn window_bottom<'t>(h: &'t Harness<'_, Moonglow>, key: ResKey) -> egui_kittest::Node<'t> {
    let window = h.state().dock.find_tab(&Tab::Script(key)).expect("open").surface;
    let rect = h.state().dock.iter_leaves().find(|(p, _)| p.surface == window).unwrap().1.rect;
    h.query_all_by_label("Window's Bottom Pane")
        .find(|n| place(n).is_some_and(|p| rect.expand(2.0).contains_rect(p)))
        .expect("the script's window's bottom pane")
}

/// `key`'s window's bottom pane: from its tab bar's top to the window's
/// foot, across the bar.
fn window_pane(h: &Harness<'_, Moonglow>, key: ResKey) -> egui::Rect {
    let bar = place(&window_bottom(h, key)).expect("the bar drawn");
    let window = h.state().dock.find_tab(&Tab::Script(key)).expect("open").surface;
    let rect = h.state().dock.iter_leaves().find(|(p, _)| p.surface == window).unwrap().1.rect;
    egui::Rect::from_x_y_ranges(bar.x_range(), bar.top()..=rect.bottom())
}

/// Whether a node labeled `label` is drawn inside `rect`.
fn drawn_in(h: &Harness<'_, Moonglow>, label: &str, rect: egui::Rect) -> bool {
    h.query_all_by_label(label).any(|n| place(&n).is_some_and(|p| rect.intersects(p)))
}

/// Whether a node whose label holds `label` is drawn inside `rect`.
fn drawn_in_contains(h: &Harness<'_, Moonglow>, label: &str, rect: egui::Rect) -> bool {
    h.query_all_by_label_contains(label).any(|n| place(&n).is_some_and(|p| rect.intersects(p)))
}

/// The switches open and close their panels: a script docked in front
/// brings Compiler and Bookmarks, their switches after the Log's and
/// Search Results' after them (the Script board's order); a switch off
/// opens the Bottom Dock on its panel, on closes it; another editor in
/// front leaves the Log and Search Results alone. Floating, the script's
/// panels are its window's bottom pane's tabs, the main window's Status
/// Bar the Log's and Search Results' alone.
#[test]
fn the_switches_open_and_close_their_panels() {
    let (mut h, keys) =
        scripts_window(Mode::Tabs, "ui-script-switches", &[("hello", "void main() {}\n")], 1600.0);
    assert_eq!(switches(&h), ["Log", "Problems", "Compiler", "Bookmarks", "Search Results"]);
    assert_eq!(h.state().bottom_dock_shows(), None, "closed, a new installation's");
    for (name, panel, body) in [
        ("Compiler", Panel::Compiler, mg_ui::COMPILER_MESSAGES),
        ("Bookmarks", Panel::Bookmarks, mg_ui::BOOKMARKED_LINES),
        ("Search Results", Panel::SearchResults, mg_ui::SEARCH_HITS),
        ("Log", Panel::Log, "Log"),
    ] {
        switch(&mut h, name);
        assert_eq!(h.state().bottom_dock_shows(), Some(panel), "{name} opened");
        let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
        assert!(drawn_in(&h, body, dock) || panel == Panel::Log, "{name}'s body");
        assert!(h.get_by_label("Bottom Dock").query_by_label(name).is_some(), "{name}'s tab");
        switch(&mut h, name);
        assert_eq!(h.state().bottom_dock_shows(), None, "{name} closed it");
    }
    // The Bottom Dock's tabs in the board's order.
    switch(&mut h, "Compiler");
    let dock = h.get_by_label("Bottom Dock");
    let mut tabs: Vec<(f32, &str)> = ["Compiler", "Bookmarks", "Search Results", "Log"]
        .iter()
        .map(|n| (dock.get_by_label(n).rect().left(), *n))
        .collect();
    tabs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let order: Vec<&str> = tabs.iter().map(|t| t.1).collect();
    assert_eq!(order, ["Compiler", "Bookmarks", "Search Results", "Log"]);
    // Another editor in front: the Log's and Search Results' switches
    // alone, the dock on the Log.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    assert_eq!(switches(&h), ["Log", "Problems", "Search Results"]);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Log));
    // The script again: its Compiler.
    click_tab(&mut h, &Tab::Script(keys[0]));
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Compiler));

    // Floating: the window's bottom pane has the three, the main window's
    // switches the Log's and Search Results'.
    let (mut h, keys) = scripts_window(
        Mode::Floating,
        "ui-script-switches",
        &[("hello", "void main() {}\n")],
        1600.0,
    );
    assert_eq!(switches(&h), ["Log", "Problems", "Search Results"]);
    for (name, body) in [
        ("Bookmarks", mg_ui::BOOKMARKED_LINES),
        ("Search Results", mg_ui::SEARCH_HITS),
        ("Compiler", mg_ui::COMPILER_MESSAGES),
    ] {
        window_bottom(&h, keys[0]).get_by_label(name).click();
        settle(&mut h);
        let pane = window_pane(&h, keys[0]);
        assert!(drawn_in(&h, body, pane), "{name}'s body in the window's bottom pane");
    }
}

/// A search's hits go into Search Results, brought to the front (the
/// Bottom Dock opened on it; floating, the window's bottom pane), and a
/// click on a hit opens the right line in the right script, two scripts
/// open.
#[test]
fn a_search_s_hits_open_the_right_line_in_the_right_script() {
    for mode in MODES {
        let a = "void main()\n{\n    int word = 1;\n}\n";
        let b = "// b\nvoid main()\n{\n    int n = 2;\n    string s = \"word\";\n}\n";
        let (mut h, keys) =
            scripts_window(mode, "ui-script-hits", &[("aaa", a), ("bbb", b)], 1600.0);
        // (Floating, the main window's Bottom Dock closed: the hits go to
        // the script's window's pane, and the dock stays closed.)
        h.state_mut().settings.hide_log = true;
        settle(&mut h);
        let search = &mut h.state_mut().script_tools.search;
        (search.open, search.script, search.in_open) = (true, Some(keys[0]), true);
        search.find = "word".into();
        settle(&mut h);
        h.get_by_label("Find All").click();
        settle(&mut h);
        h.state_mut().script_tools.search.open = false;
        settle(&mut h);
        let hits = &h.state().search_results.hits;
        assert_eq!(
            hits,
            &[
                Hit::Line { script: keys[0], line: 2, text: "int word = 1;".into() },
                Hit::Line { script: keys[1], line: 4, text: "string s = \"word\";".into() },
            ],
            "{mode:?}"
        );
        assert_eq!(h.state().search_results.what, "\"word\": 2 lines found");
        let row = "bbb(5): string s = \"word\";";
        match mode {
            Mode::Tabs => {
                assert_eq!(h.state().bottom_dock_shows(), Some(Panel::SearchResults));
                let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
                let hit =
                    h.get_all_by_label(row).find(|n| place(n).is_some_and(|p| dock.intersects(p)));
                hit.expect("the hit in the Bottom Dock").click();
            }
            Mode::Floating => {
                assert_eq!(h.state().bottom_dock_shows(), None, "the main dock closed still");
                let window = h.state().window_bottom_for_test(&Tab::Script(keys[0]));
                assert_eq!(window, Some(Panel::SearchResults), "the window's pane on the hits");
                let pane = window_pane(&h, keys[0]);
                let hit =
                    h.get_all_by_label(row).find(|n| place(n).is_some_and(|p| pane.intersects(p)));
                hit.expect("the hit in aaa's window's bottom pane").click();
            }
        }
        settle(&mut h);
        let at = b.find("string").unwrap() - 4;
        assert_eq!(cursor_of(&h, keys[1]), Some((at, at)), "{mode:?}: bbb at its line");
        let aaa = cursor_of(&h, keys[0]);
        assert!(aaa.is_none_or(|c| c.0 == 0), "{mode:?}: aaa untouched, {aaa:?}");
        if mode == Mode::Tabs {
            assert_eq!(h.state().active_editor(), Some(&Tab::Script(keys[1])));
        }
    }
}

/// The live error: a wavy underline in the theme's `error` under what is
/// on its line (from its first character that is not a space to its
/// last), in both themes and either host, the error in its tip; docked
/// in front, the Status Bar's message, which a click takes to the line;
/// none of them where checking as you type is off.
#[test]
fn an_error_has_a_wavy_underline_and_the_status_bar_says_it() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let text = "void main()\n{\n    int n = 1 +;\n}\n";
    for (mode, light) in [(Mode::Tabs, false), (Mode::Tabs, true), (Mode::Floating, false)] {
        let scripts = [("broken", text.as_bytes())];
        let (mut h, keys) =
            scripts_window_with(mode, "ui-script-live-error", &scripts, 1600.0, Some(&root));
        let key = keys[0];
        h.state_mut().settings.light_theme = light;
        settle(&mut h);
        let error = h.state().script_error_for_test(key);
        assert_eq!(error.as_ref().map(|e| e.0), Some(Some(2)), "{error:?}");
        let t = mg_uikit::tokens(&h.ctx);
        let painted = waves(&h, t.error);
        assert_eq!(painted.len(), 1, "{mode:?} light {light}: one wavy line");
        let wave = &painted[0];
        // Under the line's own text, `int n = 1 +;`, from its first
        // character to after its last.
        let editor = h.ctx.read_response(egui::Id::new(("script", key))).unwrap().rect;
        let (left, right) = (wave[0].x, wave.last().unwrap().x);
        assert!(left > editor.left() + 20.0, "after the indentation: {left} in {editor:?}");
        let char_w = (right - left) / "int n = 1 +;".len() as f32;
        assert!((5.0..12.0).contains(&char_w), "{char_w} a character");
        let ys: Vec<f32> = wave.iter().map(|p| p.y).collect();
        let spread = ys.iter().copied().fold(f32::MIN, f32::max)
            - ys.iter().copied().fold(f32::MAX, f32::min);
        assert!((spread - mg_uikit::WAVE_HEIGHT).abs() < 0.01, "a wave: {ys:?}");
        // The error in its tip.
        let under = egui::Rect::from_x_y_ranges(left..=right, wave[0].y - 1.0..=wave[0].y + 1.0);
        let tip = tip_at(&mut h, under).unwrap_or_default();
        assert!(tip.starts_with("broken.nss(3): "), "{mode:?}: the tip {tip:?}");
        assert!(tip.ends_with("\nFound as you type."), "{tip:?}");
        h.hover_at(egui::pos2(1.0, 1.0));
        settle(&mut h);
        // Docked in front: the Status Bar's message, in place of the log's
        // last line.
        let message = h.get_by_label("Status Bar").query_by_label_contains("Found as you type:");
        match mode {
            Mode::Tabs => {
                let message = message.expect("the Status Bar's message");
                let said = message.accesskit_node().label().unwrap_or_default();
                assert!(said.contains("broken.nss(3): "), "{said:?}");
                // (Whole, past the Script board's old cap of 170: the
                // error takes the bar's room, 5.2.0.)
                let width = message.rect().width();
                assert!(width > 200.0, "the whole error: {width}");
                message.click();
                settle(&mut h);
                let line_start = text.find("    int").unwrap();
                assert_eq!(cursor_of(&h, key), Some((line_start, line_start)), "at its line");
            }
            Mode::Floating => {
                assert!(message.is_none(), "the main window's bar is not the script's")
            }
        }
        h.state_mut().settings.no_live_check = true;
        settle(&mut h);
        assert!(waves(&h, t.error).is_empty(), "{mode:?}: no underline with the check off");
        assert!(
            h.get_by_label("Status Bar").query_by_label_contains("Found as you type:").is_none()
        );
    }
}

/// The Status Bar counts the cursor's line and column (each from 1, the
/// column in characters), "Line 1, column 1" before it moves, beside the
/// script in front; another editor's has no such count.
#[test]
fn the_status_bar_counts_the_cursor_s_line_and_column() {
    // (An "é" of one byte, as the scripts' codepage has it.)
    let text = "// Héllo\nvoid main()\n{\n}\n";
    let bytes = b"// H\xe9llo\nvoid main()\n{\n}\n";
    let (mut h, keys) =
        scripts_window_with(Mode::Tabs, "ui-script-count", &[("hello", bytes)], 1600.0, None);
    let count = |h: &Harness<'_, Moonglow>| {
        h.state().status_said().into_iter().find(|s| s.starts_with("Line "))
    };
    assert_eq!(count(&h).as_deref(), Some("Line 1, column 1"));
    h.state_mut().script_tools.jump = Some((keys[0], text.chars().position(|c| c == '(').unwrap()));
    settle(&mut h);
    assert_eq!(count(&h).as_deref(), Some("Line 2, column 10"));
    h.state_mut().script_tools.jump = Some((keys[0], 8));
    settle(&mut h);
    assert_eq!(count(&h).as_deref(), Some("Line 1, column 9"), "characters, not bytes");
    h.get_by_label("Status Bar").get_by_label("Line 1, column 9");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    assert_eq!(count(&h), None);
}

/// A floating script under the pointer has the main window's Status Bar:
/// its count and its live error's message (the bar follows a floating
/// window's editor under the pointer or pressed in last,
/// `shell::update_shown`), and with the pointer elsewhere neither (M5's
/// W7: U13's decisions 2 and 8 said a floating script had neither).
#[test]
fn a_floating_script_under_the_pointer_has_the_status_bar_s_count_and_message() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let text = "void main()\n{\n    int n = 1 +;\n}\n";
    let scripts = [("broken", text.as_bytes())];
    let (mut h, keys) = scripts_window_with(
        Mode::Floating,
        "ui-script-floating-bar",
        &scripts,
        1600.0,
        Some(&root),
    );
    let key = keys[0];
    settle(&mut h);
    assert!(h.state().script_error_for_test(key).is_some(), "the check found its error");
    let editor = h.ctx.read_response(egui::Id::new(("script", key))).unwrap().rect;
    let bar = |h: &Harness<'_, Moonglow>| {
        let said = h.state().status_said();
        let count = said.iter().any(|s| s == "Line 1, column 1");
        let message = h.get_by_label("Status Bar").query_by_label_contains("Found as you type:");
        (count, message.map(|m| m.accesskit_node().label().unwrap_or_default()))
    };
    h.hover_at(egui::pos2(1.0, 1.0));
    settle(&mut h);
    assert_eq!(bar(&h), (false, None), "the pointer elsewhere: not the script's");
    h.hover_at(editor.center());
    settle(&mut h);
    let (count, message) = bar(&h);
    assert!(count, "its count: {:?}", h.state().status_said());
    assert!(message.is_some_and(|m| m.contains("broken.nss(3): ")), "its message");
}

/// A panel in the Bottom Dock follows the script in front: two scripts
/// docked in two groups, Bookmarks lists the focused one's lines, and
/// Compiler its messages; a third floating keeps its own in its window's
/// bottom pane, and the docked one's dock is left as it was.
#[test]
fn a_bottom_dock_panel_follows_the_script_in_front() {
    let a = "void main()\n{\n    int a = 1;\n}\n";
    let b = "void main()\n{\n    int b = 2;\n}\n";
    let c = "void main()\n{\n    int c = 3;\n}\n";
    let (mut h, keys) = scripts_window(
        Mode::Tabs,
        "ui-script-follow",
        &[("aaa", a), ("bbb", b), ("ccc", c)],
        1600.0,
    );
    // ccc floats; bbb in a second group beside aaa.
    own_window(&mut h, &Tab::Script(keys[2]), egui::pos2(200.0, 150.0), egui::vec2(700.0, 500.0));
    click_tab(&mut h, &Tab::Script(keys[1]));
    settle(&mut h);
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    // A bookmark in each, on its third line.
    for key in &keys {
        let at = a.find("    int").unwrap();
        h.state_mut().scripts_mut_bookmark_for_test(*key, at);
    }
    let marked = |h: &Harness<'_, Moonglow>, what: &str| {
        drawn_in_contains(h, what, h.state().bottom_dock_rect().expect("the Bottom Dock"))
    };
    for (front, own, other) in [(keys[0], "int a", "int b"), (keys[1], "int b", "int a")] {
        click_tab(&mut h, &Tab::Script(front));
        settle(&mut h);
        assert_eq!(h.state().active_editor(), Some(&Tab::Script(front)));
        switch_on(&mut h, "Bookmarks");
        assert!(marked(&h, own), "{own}'s line");
        assert!(!marked(&h, other), "not {other}'s");
        assert!(!marked(&h, "int c"), "nor the floating one's");
    }
    // The floating one's own pane.
    window_bottom(&h, keys[2]).get_by_label("Bookmarks").click();
    settle(&mut h);
    let pane = window_pane(&h, keys[2]);
    assert!(drawn_in_contains(&h, "int c", pane), "ccc's bookmark in its window");
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Bookmarks), "the dock as it was");
}

/// The Status Bar's switch `name` turned on, if it is off.
fn switch_on(h: &mut Harness<'_, Moonglow>, name: &str) {
    let on = h.get_by_label("Status Bar").get_by_label(name).accesskit_node().toggled();
    if on != Some(egui::accesskit::Toggled::True) {
        switch(h, name);
    }
}

/// The window comes to rest with each panel in front and behind while
/// typing (each new panel is drawn before the script's editor, which
/// changes what it shows; Help among them since 5.2.0): Bookmarks drawn,
/// then each panel in front (so
/// Bookmarks behind it, but where it is itself), then typed in, a
/// bookmark set, compiled.
#[test]
fn the_window_comes_to_rest_with_each_panel_in_front_or_behind() {
    for mode in MODES {
        for front in ["Compiler", "Help", "Bookmarks", "Search Results", "Log"] {
            let (mut h, keys) =
                scripts_window(mode, "ui-script-rest", &[("hello", "void main()\n{\n}\n")], 1200.0);
            let key = keys[0];
            // (Bookmarks drawn first, then the panel `front` in front: a
            // panel drawn before and now behind asks for nothing.)
            for shown in ["Bookmarks", front] {
                match mode {
                    Mode::Tabs => switch_on(&mut h, shown),
                    Mode::Floating if shown != "Log" => {
                        window_bottom(&h, key).get_by_label(shown).click();
                        settle(&mut h);
                    }
                    Mode::Floating => {}
                }
            }
            h.state_mut().script_tools.jump = Some((key, 0));
            settle(&mut h);
            h.event(egui::Event::Text("int x;\n".into()));
            settle(&mut h);
            // (A bookmark set where nothing brings Bookmarks forward: what
            // it shows changed while it may be behind.)
            h.state_mut().scripts_mut_bookmark_for_test(key, 0);
            settle(&mut h);
            h.event(egui::Event::Key {
                key: egui::Key::F5,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
            settle(&mut h);
            assert!(!h.state().script_bookmarks_for_test(key).is_empty(), "{mode:?} {front}");
            script_click(&mut h, "Compile");
            settle(&mut h);
            assert!(h.state().script_tools.messages.iter().any(|m| m.script == key));
        }
    }
}

/// Records from before the Bottom Dock held a script's panels read
/// without a fault: a layout kept for scripts naming the old bottom
/// pane's Help tab (a panel of the Bottom Dock again since 5.2.0: kept),
/// a newer one's panel, or a global panel as its front;
/// an open-tabs view naming the old pane's tab. Each passes over what it
/// does not know, and Compiler comes first where nothing else is kept.
#[test]
fn records_naming_the_old_bottom_pane_or_its_tabs_read_without_error() {
    for (kept, front) in [
        ("Some(\"Help\")", Panel::Help),
        ("Some(\"Script Info\")", Panel::Compiler),
        ("Some(\"Bookmarks\")", Panel::Bookmarks),
        ("Some(\"Search Results\")", Panel::SearchResults),
        ("None", Panel::Compiler),
    ] {
        let dir = mg_testkit::scratch_dir("ui-script-old-bottom");
        let path = sample_module(&dir);
        let key = ResKey::parse("hello", ResType::NSS).unwrap();
        let mut m = Module::open(&path).unwrap();
        m.set(key, b"void main()\n{\n}\n".to_vec());
        m.save().unwrap();
        let tabs = dir.join("open-tabs");
        std::fs::create_dir_all(&tabs).unwrap();
        let record = r#"{"panes":[{"place":"middle","tabs":["script:hello.nss"]}],
            "front":"script:hello.nss",
            "views":{"script:hello.nss":{"cursor":[3,3],"info":"SearchResults",
            "bottom":"script-info","lines":[1]}}}"#;
        std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
        let text = format!("(dock_layouts:{{\"script\":(bottom:{kept},fronts:[\"Help\"])}})");
        let (settings, unread) = mg_ui::Settings::read_fields(&text).expect("read");
        assert!(unread.is_empty(), "{unread:?}");
        let mut app = app_in(Mode::Tabs, Vec::new());
        app.settings.dock_layouts = settings.dock_layouts;
        app.settings.hide_log = false;
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(tabs);
        app.open_module(&path);
        let mut h = harness(app, 1600.0, 800.0);
        settle(&mut h);
        assert_eq!(h.state().active_editor(), Some(&Tab::Script(key)));
        assert_eq!(h.state().bottom_dock_shows(), Some(front), "{kept}");
        assert_eq!(cursor_of(&h, key), Some((3, 3)), "its cursor kept");
        assert_eq!(h.state().script_bookmarks_for_test(key), [1], "its bookmark kept");
    }
}

/// In a small script window, the bottom pane (Compiler, Bookmarks,
/// Search Results) takes at most a third of the window under its tab bar,
/// however tall it was dragged: the code keeps two thirds (the known
/// difference "In small script windows the Compiler and Help strip crowds
/// the code", settled); and at least its tab bar and a line.
#[test]
fn a_small_script_window_keeps_two_thirds_for_its_code() {
    let (mut h, keys) = scripts_window(
        Mode::Floating,
        "ui-script-small-bottom",
        &[("hello", "void main()\n{\n}\n")],
        1000.0,
    );
    let script = Tab::Script(keys[0]);
    for high in [300.0, 420.0, 640.0] {
        own_window(&mut h, &script, egui::pos2(60.0, 60.0), egui::vec2(700.0, high));
        settle(&mut h);
        let at = h.state().dock.find_tab(&script).unwrap();
        let leaf = h.state().dock.iter_leaves().find(|(p, _)| p.surface == at.surface).unwrap().1;
        let room = leaf.rect.height();
        let pane = window_pane(&h, keys[0]);
        let pane_high = leaf.rect.bottom() - pane.top();
        let editor = h.ctx.read_response(egui::Id::new(("script", keys[0]))).unwrap().rect;
        assert!(pane_high <= room / 3.0 + 2.0, "{high}: the pane {pane_high} of {room}");
        assert!(pane_high >= 26.0, "{high}: its tab bar at least: {pane_high}");
        assert!(editor.top() < pane.top(), "{high}: the code above it");
    }
}

/// F6 reaches the Bottom Dock's panel in front, which takes the
/// keyboard's focus from the script's text: what is typed then goes
/// nowhere (as F6 to Help, U12's review).
#[test]
fn f6_to_a_bottom_dock_panel_takes_the_keyboard_from_the_text() {
    for (panel, keys, body) in [
        ("Compiler", "Compiler", mg_ui::COMPILER_MESSAGES),
        ("Bookmarks", "Bookmarks", mg_ui::BOOKMARKED_LINES),
        ("Search Results", "SearchResults", mg_ui::SEARCH_HITS),
    ] {
        let (mut h, keys_of) = scripts_window(
            Mode::Tabs,
            "ui-script-f6-bottom",
            &[("hello", "void main() {}\n")],
            1600.0,
        );
        let key = keys_of[0];
        switch_on(&mut h, panel);
        h.state_mut().script_tools.jump = Some((key, 0));
        settle(&mut h);
        h.event(egui::Event::Text("a".into()));
        settle(&mut h);
        assert!(h.state().script_text(key).unwrap().starts_with("avoid"), "the text has the keys");
        let mut stops = 0;
        while mg_ui::palette_view::arrow_keys_are(&h.ctx) != keys {
            stops += 1;
            assert!(stops <= 8, "F6 never reached {panel}");
            h.key_press(egui::Key::F6);
            settle(&mut h);
        }
        let name = format!("{body}, the panel");
        let focused = h.get_by_label(&name).accesskit_node().is_focused();
        assert!(focused, "{panel} has the focus");
        h.event(egui::Event::Text("z".into()));
        settle(&mut h);
        assert!(h.state().script_text(key).unwrap().starts_with("avoid"), "nothing typed into it");
    }
}

/// The log's last line beside a script is cut at the Script board's 170
/// (beside another editor, 160): a long line, the log closed.
#[test]
fn the_last_line_beside_a_script_is_cut_at_170() {
    let (mut h, _) =
        scripts_window(Mode::Tabs, "ui-script-cap", &[("hello", "void main() {}\n")], 1600.0);
    h.state_mut().settings.no_live_check = true;
    h.state_mut().log.info("a line long enough to be cut short by the Status Bar ".repeat(4));
    settle(&mut h);
    let width = |h: &Harness<'_, Moonglow>| {
        let bar = h.get_by_label("Status Bar");
        bar.get_by_label_contains("Last log line:").rect().width()
    };
    let beside = width(&h);
    // (Cut where a character ends, a few points short of its cap.)
    assert!((160.5..=170.5).contains(&beside), "beside the script: {beside}");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    let other = width(&h);
    assert!((150.0..=160.5).contains(&other), "beside Module Properties: {other}");
}

/// Docked, a click on a Compiler message goes to its line: in an
/// include it names, whose tab opens in front at the line, and in the
/// script compiled; a click on a Bookmarks row goes to its line.
#[test]
fn compiler_and_bookmarks_clicks_go_to_their_lines_docked() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let inc = "// An include.\nint Broken() { return 1 +; }\n";
    let main = "#include \"inc_bad\"\nvoid main()\n{\n    int n = Broken();\n}\n";
    let scripts: [(&str, &[u8]); 2] = [("inc_bad", inc.as_bytes()), ("uses", main.as_bytes())];
    let (mut h, keys) =
        scripts_window_with(Mode::Tabs, "ui-script-clicks", &scripts, 1600.0, Some(&root));
    // (Only `uses` open, in front.)
    click_tab_close(&mut h, &Tab::Script(keys[0]));
    settle(&mut h);
    let key = keys[1];
    // A message's click: the script it names opened in front at its line.
    let goes = |h: &mut Harness<'_, Moonglow>, to: ResKey, line: usize| {
        script_click(h, "Compile");
        settle(h);
        assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Compiler), "a failure opens it");
        let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
        let said = format!("{}.nss({line}): ERROR", to.resref);
        let n = h
            .query_all_by_label_contains(&said)
            .find(|n| place(n).is_some_and(|p| dock.intersects(p)));
        n.unwrap_or_else(|| panic!("{said:?} in Compiler")).click();
        settle(h);
        assert_eq!(h.state().active_editor(), Some(&Tab::Script(to)), "{said}: in front");
        let text = h.state().script_text(to).unwrap();
        let at = mg_ui::script_tools::line_start(&text, line - 1);
        assert_eq!(cursor_of(h, to), Some((at, at)), "{said}: at its line");
    };
    goes(&mut h, keys[0], 2);
    click_tab(&mut h, &Tab::Script(key));
    settle(&mut h);
    let own = "void main()\n{\n    int n = 1 +;\n}\n";
    h.state_mut().scripts_mut_for_test(key, own);
    settle(&mut h);
    goes(&mut h, key, 3);
    // A Bookmarks row.
    let line3 = own.find("    int").unwrap();
    h.state_mut().scripts_mut_bookmark_for_test(key, line3);
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    switch_on(&mut h, "Bookmarks");
    let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
    let row = h
        .query_all_by_label_contains("3: int n")
        .find(|n| place(n).is_some_and(|p| dock.intersects(p)));
    row.expect("the bookmark's row").click();
    settle(&mut h);
    assert_eq!(cursor_of(&h, key), Some((line3, line3)), "at the bookmark's line");
}

// ---------------------------------------------------------------- review

/// The names of the script's eight hints shown in the Status Bar.
fn hints_shown(h: &Harness<'_, Moonglow>) -> usize {
    let names = ["Find", "Find next", "Definition", "References", "Complete", "Bookmark"];
    let bar = h.get_by_label("Status Bar");
    let shown = |n: &str| bar.query_all_by_label_contains(&format!(": {n}")).count() > 0;
    names.iter().filter(|n| shown(n)).count()
        + ["Compile all", "Replace"].iter().filter(|n| shown(n)).count()
}

/// From 1400 the script's hints drop whole before the Status Bar's
/// switches lose their words (SPEC 12, Script.png at 1600: the
/// coordinator's ruling at U13's review); under 1400 the switches show
/// their icons first, as M3's T14 made them.
#[test]
fn from_1400_hints_drop_before_the_switches_lose_their_words() {
    let (mut h, _) =
        scripts_window(Mode::Tabs, "ui-script-yields", &[("hello", "void main() {}\n")], 1600.0);
    h.state_mut().settings.no_live_check = true;
    h.state_mut().log.info("m1q3_formosa_9.nss: compiled, a line long enough to take its room");
    let words = |h: &Harness<'_, Moonglow>| {
        h.get_by_label("Status Bar").get_by_label("Log").rect().width() > 30.0
    };
    for width in [1400.0, 1440.0, 1500.0, 1600.0] {
        h.set_size(egui::vec2(width, 800.0));
        settle(&mut h);
        assert!(words(&h), "{width}: the switches' words");
    }
    h.set_size(egui::vec2(1400.0, 800.0));
    settle(&mut h);
    assert!(hints_shown(&h) < 8, "1400: hints dropped beside the words: {}", hints_shown(&h));
    h.set_size(egui::vec2(1399.0, 800.0));
    settle(&mut h);
    assert!(!words(&h), "under 1400: the icons");
}

/// The wavy line in `color` painted in the last frame, the one expected.
fn one_wave(h: &Harness<'_, Moonglow>, color: egui::Color32) -> Vec<egui::Pos2> {
    let painted = waves(h, color);
    assert_eq!(painted.len(), 1, "one wavy line: {painted:?}");
    painted.into_iter().next().unwrap()
}

/// An error at the end of the file (its line blank or past the text) is
/// underlined on the nearest line with text above; one in an include on
/// the `#include` line naming it, its tip naming the include and the line.
/// Docked, the Status Bar's message goes there; floating, the script's
/// window's bottom pane says it at its bar's right end, red, a click going
/// there, needing no hover.
#[test]
fn end_of_file_and_include_errors_are_underlined_and_said_in_either_host() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let eof = "void main()\n{\n    int n = 1;\n\n\n";
    let inc = "// An include.\nint Broken() { return 1 +; }\n";
    let uses = "// Uses it.\n#include \"inc_bad\"\nvoid main() { int n = Broken(); }\n";
    for mode in MODES {
        let scripts: [(&str, &[u8]); 3] =
            [("inc_bad", inc.as_bytes()), ("ateof", eof.as_bytes()), ("uses", uses.as_bytes())];
        let (mut h, keys) =
            scripts_window_with(mode, "ui-script-fallback", &scripts, 1600.0, Some(&root));
        let error = mg_uikit::tokens(&h.ctx).error;
        for (key, text, line, tip) in
            [(keys[1], eof, 2, None), (keys[2], uses, 1, Some("In the include inc_bad, line 2."))]
        {
            // (Only the script looked at open, docked or in a window.)
            for other in &keys {
                if *other != key && h.state().dock.find_tab(&Tab::Script(*other)).is_some() {
                    click_tab_close(&mut h, &Tab::Script(*other));
                    settle(&mut h);
                }
            }
            h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
            settle(&mut h);
            let found = h.state().script_error_for_test(key);
            assert!(found.is_some(), "{mode:?} {key}: an error found");
            let wave = one_wave(&h, error);
            if let Some(tip) = tip {
                let at = egui::Rect::from_x_y_ranges(
                    wave[0].x..=wave.last().unwrap().x,
                    wave[0].y - 1.0..=wave[0].y + 1.0,
                );
                let said = tip_at(&mut h, at).unwrap_or_default();
                assert!(said.contains(tip), "{mode:?} {key}: {said:?}");
                h.hover_at(egui::pos2(1.0, 1.0));
                settle(&mut h);
            }
            // The message, needing no hover, and its click: to the line
            // underlined.
            let message = match mode {
                Mode::Tabs => {
                    h.get_by_label("Status Bar").query_by_label_contains("Found as you type:")
                }
                Mode::Floating => {
                    let pane = window_pane(&h, key);
                    h.query_all_by_label_contains("Found as you type:")
                        .find(|n| place(n).is_some_and(|p| pane.intersects(p)))
                }
            };
            message.unwrap_or_else(|| panic!("{mode:?} {key}: the message")).click();
            settle(&mut h);
            let at = mg_ui::script_tools::line_start(text, line);
            assert_eq!(cursor_of(&h, key), Some((at, at)), "{mode:?} {key}: to line {line}");
            let editor = h.ctx.read_response(egui::Id::new(("script", key))).unwrap().rect;
            assert!(editor.contains(wave[0]), "{mode:?} {key}: in its editor");
        }
    }
}

/// A script not in front asking (File › Save of a script in the other
/// group, saved not compiled) leaves the Bottom Dock as it was.
#[test]
fn a_script_not_in_front_leaves_the_bottom_dock_as_it_was() {
    let (mut h, keys) = scripts_window(
        Mode::Tabs,
        "ui-script-not-front",
        &[("aaa", "void main() {}\n"), ("bbb", "void main() {}\n")],
        1600.0,
    );
    h.state_mut().settings.no_live_check = true;
    click_tab(&mut h, &Tab::Script(keys[1]));
    settle(&mut h);
    h.get_by_label("Split Editor").click();
    settle(&mut h);
    click_tab(&mut h, &Tab::Script(keys[0]));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Script(keys[0])));
    switch_on(&mut h, "Search Results");
    h.state_mut().settings.auto_compile = false;
    h.state_mut().scripts_mut_for_test(keys[1], "void main() { }\n");
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert!(h.state().script_tools.messages.iter().any(|m| m.script == keys[1]), "bbb saved");
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::SearchResults), "as it was");
}

/// What brings a panel forward, and what does not: a compile that passed
/// and a bookmark leave a closed Bottom Dock closed and an open one on the
/// Log; a compile that fails opens it on Compiler; a search that finds
/// nothing leaves it closed, one that finds opens it on Search Results.
#[test]
fn only_a_failure_or_hits_open_the_dock_or_take_the_log_s_place() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let scripts: [(&str, &[u8]); 1] = [("hello", b"void main()\n{\n}\n")];
    let (mut h, keys) =
        scripts_window_with(Mode::Tabs, "ui-script-asks", &scripts, 1600.0, Some(&root));
    let key = keys[0];
    let f5 = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().script_tools.jump = Some((key, 0));
        settle(h);
        h.key_press(egui::Key::F5);
        settle(h);
    };
    assert_eq!(h.state().bottom_dock_shows(), None, "closed at first");
    script_click(&mut h, "Compile");
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), None, "a compile that passed");
    f5(&mut h);
    assert_eq!(h.state().script_bookmarks_for_test(key), [0], "the bookmark set");
    assert_eq!(h.state().bottom_dock_shows(), None, "a bookmark");
    let search = &mut h.state_mut().script_tools.search;
    (search.open, search.script, search.in_open) = (true, Some(key), true);
    search.find = "nowhere".into();
    settle(&mut h);
    h.get_by_label("Find All").click();
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), None, "nothing found");
    h.state_mut().script_tools.search.open = false;
    settle(&mut h);
    // The log in front.
    switch_on(&mut h, "Log");
    f5(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Log), "a bookmark over the log");
    script_click(&mut h, "Compile");
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Log), "a compile that passed");
    h.state_mut().scripts_mut_for_test(key, "void main()\n{\n    int n = 1 +;\n}\n");
    settle(&mut h);
    script_click(&mut h, "Compile");
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Compiler), "a compile that failed");
}

/// References from a docked script's header after typing in a floating
/// one: the hits go where the script it was run from is (the main
/// window's Bottom Dock), not to the window typed in last.
#[test]
fn references_go_where_the_script_they_are_run_from_is() {
    let a = "int Twice(int n) { return n * 2; }\nvoid main() { int x = Twice(2); }\n";
    let (mut h, keys) = scripts_window(
        Mode::Tabs,
        "ui-script-references-from",
        &[("bbb", "void main() {}\n"), ("aaa", a)],
        1600.0,
    );
    h.state_mut().settings.no_live_check = true;
    // (aaa docked in front, its cursor on the name; bbb floating, typed in
    // last, the keys in it.)
    h.state_mut().script_tools.jump = Some((keys[1], a.find("Twice(2)").unwrap() + 1));
    settle(&mut h);
    own_window(&mut h, &Tab::Script(keys[0]), egui::pos2(500.0, 300.0), egui::vec2(700.0, 400.0));
    h.state_mut().script_tools.jump = Some((keys[0], 0));
    h.state_mut().script_tools.keys_back_for_test(keys[0]);
    settle(&mut h);
    h.event(egui::Event::Text("x".into()));
    settle(&mut h);
    assert!(h.state().script_text(keys[0]).unwrap().starts_with('x'), "typed in bbb last");
    assert_eq!(h.state().active_editor(), Some(&Tab::Script(keys[1])), "aaa docked in front");
    // (aaa's header: the one higher up, over the editor area's top.)
    let top = |n: &egui_kittest::Node<'_>| place(n).map_or(f32::MAX, |p| p.top());
    let header = (h.get_all_by_label(mg_ui::SCRIPT_HEADER))
        .min_by(|a, b| top(a).total_cmp(&top(b)))
        .unwrap();
    header.get_by_label("References").click();
    settle(&mut h);
    assert_eq!(h.state().search_results.hits.len(), 2, "{:?}", h.state().search_results.hits);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::SearchResults), "the main dock");
    assert_ne!(
        h.state().window_bottom_for_test(&Tab::Script(keys[0])),
        Some(Panel::SearchResults),
        "not bbb's window"
    );
}

/// File › Save of several scripts compiled on saving, the one in front
/// failing: Compiler opens, whatever the order the others asked in.
#[test]
fn a_failure_in_front_among_several_saved_opens_compiler() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let ok = b"void main() { }\n";
    let scripts: [(&str, &[u8]); 3] = [("aaa", ok), ("bbb", ok), ("zzz", ok)];
    let (mut h, keys) =
        scripts_window_with(Mode::Tabs, "ui-script-multi-save", &scripts, 1600.0, Some(&root));
    h.state_mut().settings.auto_compile = true;
    h.state_mut().settings.no_live_check = true;
    // (The script in front, whichever it is here, made to fail; the
    // others asking too, before it and after it.)
    let front = h.state().active_editor().cloned();
    let Some(Tab::Script(failing)) = front else { panic!("a script in front: {front:?}") };
    for key in &keys {
        let text = if *key == failing {
            "void main() { int n = 1 +; }\n"
        } else {
            "void main() { int n; }\n"
        };
        h.state_mut().scripts_mut_for_test(*key, text);
    }
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), None, "closed");
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    let failed = h.state().script_tools.messages.iter().any(|m| m.script == failing && m.error);
    assert!(failed, "{:?}", h.state().script_tools.messages);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Compiler), "opened on the failure");
    // Saved not compiled, the dock open on Search Results: the ask of the
    // script in front kept, whichever asked last.
    switch_on(&mut h, "Search Results");
    h.state_mut().settings.auto_compile = false;
    for key in &keys {
        h.state_mut().scripts_mut_for_test(*key, "void main() { int m; }\n");
    }
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Compiler), "the front's ask");
    // A bookmark in front and another script saved after it, in one frame:
    // the front's ask kept though the other's came last.
    switch_on(&mut h, "Search Results");
    let other = *keys.iter().find(|k| **k != failing).unwrap();
    h.state_mut().scripts_mut_for_test(other, "void main() { int k; }\n");
    h.state_mut().script_tools.jump = Some((failing, 0));
    h.state_mut().script_tools.keys_back_for_test(failing);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.key_press(egui::Key::F5);
    settle(&mut h);
    assert_eq!(h.state().script_bookmarks_for_test(failing), [0], "the bookmark set");
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Bookmarks), "the front's, not the last");
}

/// An error's line of 2,000 characters: its wave is made only for what is
/// in sight, a few points at a time across the editor, not along the
/// whole line.
#[test]
fn a_long_error_line_s_wave_is_made_for_what_is_in_sight() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let text = format!("void main()\n{{\n    int n = {} +;\n}}\n", "1 + ".repeat(500));
    let scripts: [(&str, &[u8]); 1] = [("long", text.as_bytes())];
    let (h, keys) =
        scripts_window_with(Mode::Tabs, "ui-script-long-error", &scripts, 1600.0, Some(&root));
    let wave = one_wave(&h, mg_uikit::tokens(&h.ctx).error);
    // (The editor area in sight: the text's own field is the line's width.)
    let _ = keys;
    let editor = h.state().editor_area_rect().expect("the editor area");
    let most = (editor.width() / (mg_uikit::marks::WAVE_LENGTH / 2.0)) as usize + 3;
    assert!(wave.len() <= most, "{} points, {most} at most", wave.len());
    assert!(wave.iter().all(|p| p.x <= editor.right() + 1.0), "in sight");
}

/// The Bottom Dock panels are F6's stops, not Tab's: shown, none adds a
/// focusable node of its own.
#[test]
fn a_bottom_dock_panel_adds_no_tab_stop() {
    let (mut h, _) = scripts_window(
        Mode::Tabs,
        "ui-script-no-tab-stop",
        &[("hello", "void main() {}\n")],
        1600.0,
    );
    for (panel, body) in [
        ("Compiler", mg_ui::COMPILER_MESSAGES),
        ("Bookmarks", mg_ui::BOOKMARKED_LINES),
        ("Search Results", mg_ui::SEARCH_HITS),
    ] {
        switch_on(&mut h, panel);
        let name = format!("{body}, the panel");
        assert!(h.query_by_label(&name).is_none(), "{panel}: a focusable node of its own");
    }
}

/// Bookmarks shows a marked line's text as typed, by the frames the
/// typing asks for: not a frame behind.
#[test]
fn bookmarks_follow_typing() {
    let (mut h, keys) = scripts_window(
        Mode::Tabs,
        "ui-script-marks-typed",
        &[("hello", "void main() {}\n")],
        1600.0,
    );
    let key = keys[0];
    h.state_mut().settings.no_live_check = true;
    switch_on(&mut h, "Bookmarks");
    h.state_mut().scripts_mut_bookmark_for_test(key, 0);
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    h.event(egui::Event::Text("x".into()));
    h.run();
    let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
    assert!(drawn_in_contains(&h, "1: xvoid main", dock), "the row as typed");
}

/// A closed window's tab keeps no window pane's front.
#[test]
fn a_closed_window_keeps_no_front() {
    let (mut h, keys) = scripts_window(
        Mode::Floating,
        "ui-script-window-front",
        &[("hello", "void main() {}\n")],
        1600.0,
    );
    let tab = Tab::Script(keys[0]);
    window_bottom(&h, keys[0]).get_by_label("Bookmarks").click();
    settle(&mut h);
    assert_eq!(h.state().window_bottom_for_test(&tab), Some(Panel::Bookmarks));
    click_tab_close(&mut h, &tab);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&tab).is_none(), "closed");
    assert_eq!(h.state().window_bottom_for_test(&tab), None);
}

/// A warning or an error logged after the live error was found takes its
/// place in the Status Bar (the log closed); an information line does not.
#[test]
fn a_warning_logged_since_takes_the_live_error_s_place() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let scripts: [(&str, &[u8]); 1] = [("broken", b"void main()\n{\n    int n = 1 +;\n}\n")];
    let (mut h, _) =
        scripts_window_with(Mode::Tabs, "ui-script-note-log", &scripts, 1600.0, Some(&root));
    let note = |h: &Harness<'_, Moonglow>| {
        h.get_by_label("Status Bar").query_by_label_contains("Found as you type:").is_some()
    };
    assert!(note(&h), "the error");
    h.state_mut().log.info("Opened something");
    settle(&mut h);
    assert!(note(&h), "an information line does not take its place");
    h.state_mut().log.warn("Careful now");
    settle(&mut h);
    assert!(!note(&h), "a warning does");
    let line = h.get_by_label("Status Bar").query_by_label_contains("Last log line: Careful now");
    assert!(line.is_some(), "the warning in its place");
}

// ------------------------------------------------------------------ 5.2's W4

/// The open menu's `item` clicked: what it copied to the clipboard.
fn menu_copies(h: &mut Harness<'_, Moonglow>, item: &str) -> Vec<String> {
    h.get_all_by_label(item).last().unwrap_or_else(|| panic!("{item} in the menu")).click();
    h.step();
    let out = (h.output().platform_output.commands.iter())
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    h.run();
    out
}

/// Ctrl+C (the backend's Copy): what it copied to the clipboard.
fn copy_now(h: &mut Harness<'_, Moonglow>) -> Vec<String> {
    h.event(egui::Event::Copy);
    h.step();
    let out = (h.output().platform_output.commands.iter())
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    h.run();
    out
}

/// Shift+F10 with the keyboard on a row: its menu open, Copy taking the
/// keyboard; Escape closes it and gives the keyboard back to the row
/// (`row` finds it).
fn menu_by_keys(h: &mut Harness<'_, Moonglow>, row: &dyn Fn(&Harness<'_, Moonglow>) -> bool) {
    use mg_ui::row_copy::{COPY, COPY_ALL};
    press_now(h, egui::Modifiers::SHIFT, egui::Key::F10);
    h.run();
    let copy = h.get_all_by_label(COPY).last().expect("Shift+F10 opens its menu");
    assert!(copy.accesskit_node().is_focused(), "Copy has the keyboard");
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.query_by_label(COPY_ALL).is_none(), "Escape closes it");
    assert!(row(h), "Escape gives the keyboard back to the row");
}

/// Compiler's messages and Bookmarks' rows (no row chosen in either): a
/// row's menu has Copy (the message whole, without its time; a bookmark's
/// line and text) and Copy All (every row, in the list's order); with the
/// keyboard on a row (Tab), Ctrl+C copies it and Shift+F10 opens its menu.
#[test]
fn compiler_and_bookmarks_rows_copy_from_their_menus() {
    use mg_ui::row_copy::{COPY, COPY_ALL};
    use mg_ui::script_tools::Message;
    let own = "void main()\n{\n    int n = 1 +;\n}\n";
    let (mut h, keys) = scripts_window(Mode::Tabs, "w4-compiler-copy", &[("uses", own)], 1600.0);
    let key = keys[0];
    let said = |text: &str, error| Message {
        script: key,
        text: text.into(),
        location: Some(("uses".into(), 3)),
        error,
        at: "14:03:27".into(),
    };
    let first = "uses.nss(3): ERROR: PARSING VARIABLE LIST";
    let second = "uses.nss(3): Compile failed: 1 error";
    h.state_mut().script_tools.messages.extend([said(first, true), said(second, false)]);
    switch_on(&mut h, "Compiler");
    settle(&mut h);
    let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
    let row = |h: &Harness<'_, Moonglow>| {
        (h.query_all_by_label_contains(first))
            .find(|n| place(n).is_some_and(|p| dock.intersects(p)))
            .expect("the message in Compiler")
            .rect()
            .center()
    };
    let at = row(&h);
    let right = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
    };
    right(&mut h, at);
    assert_eq!(menu_copies(&mut h, COPY), [first], "Copy: the message, without its time");
    right(&mut h, at);
    // (The live error first, its Found as You Type row: the include the
    // script names is not there.)
    let live = "uses.nss(1): ERROR: FILE NOT FOUND";
    assert_eq!(menu_copies(&mut h, COPY_ALL), [format!("{live}\n{first}\n{second}")], "Copy All");
    // The keyboard on a message: Ctrl+C copies it, Shift+F10 its menu.
    let message = move |h: &Harness<'_, Moonglow>| {
        (h.query_all_by_label_contains(first))
            .find(|n| place(n).is_some_and(|p| dock.intersects(p)))
            .expect("the message in Compiler")
            .accesskit_node()
            .is_focused()
    };
    (h.query_all_by_label_contains(first))
        .find(|n| place(n).is_some_and(|p| dock.intersects(p)))
        .unwrap()
        .focus();
    h.run();
    assert!(message(&h), "a message takes the keyboard");
    assert_eq!(copy_now(&mut h), [first], "Ctrl+C: the message the keyboard is on");
    menu_by_keys(&mut h, &message);
    // A bookmark's row.
    let line3 = own.find("    int").unwrap();
    h.state_mut().scripts_mut_bookmark_for_test(key, line3);
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    switch_on(&mut h, "Bookmarks");
    let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
    let at = (h.query_all_by_label_contains("3: int n"))
        .find(|n| place(n).is_some_and(|p| dock.intersects(p)))
        .expect("the bookmark's row")
        .rect()
        .center();
    right(&mut h, at);
    assert_eq!(menu_copies(&mut h, COPY), ["3: int n = 1 +;"], "its line and text");
    let bookmark = move |h: &Harness<'_, Moonglow>| {
        (h.query_all_by_label_contains("3: int n"))
            .find(|n| place(n).is_some_and(|p| dock.intersects(p)))
            .expect("the bookmark's row")
            .accesskit_node()
            .is_focused()
    };
    (h.query_all_by_label_contains("3: int n"))
        .find(|n| place(n).is_some_and(|p| dock.intersects(p)))
        .unwrap()
        .focus();
    h.run();
    assert!(bookmark(&h), "a bookmark's row takes the keyboard");
    assert_eq!(copy_now(&mut h), ["3: int n = 1 +;"], "Ctrl+C: the row the keyboard is on");
    menu_by_keys(&mut h, &bookmark);
}

// ------------------------------------------------------------------ 5.2's W5

/// A right click at `at`: its menu open.
fn right_click(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
}

/// Whether a node labeled `label` shows, the window settled and
/// then stepped for up to `within` seconds (a tip's delay), the pointer and
/// keys as they are: a tip expected is waited for up to 10, one not
/// expected for 1.
fn shows_soon(h: &mut Harness<'_, Moonglow>, label: &str, within: u64) -> bool {
    settle(h);
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(within) {
        if h.query_by_label(label).is_some() {
            return true;
        }
        h.step();
    }
    h.query_by_label(label).is_some()
}

/// The labels of the Status Bar's nodes drawn after the left edge of
/// `from` (each with its place), the live error's apart.
fn bar_after(h: &Harness<'_, Moonglow>, from: egui::Rect) -> Vec<(String, egui::Rect)> {
    let bar = h.get_by_label("Status Bar");
    (bar.query_all_by(|n| n.bounding_box().is_some()))
        .map(|n| {
            let a = n.accesskit_node();
            (a.label().or(a.value()).unwrap_or_default(), n.rect())
        })
        .filter(|(l, r)| {
            !l.is_empty() && !l.contains("Found as you type") && r.left() > from.left()
        })
        .collect()
}

/// The live error on the Status Bar (5.2.0, the owner's rulings on
/// Jasperre's report and at review): drawn whole wherever the bar has
/// room, as wide in a narrow window as in a wide one, past the Script
/// board's old cap; the others give way only as far as it needs, the
/// folder first, then the hints from the last, the cursor's line and
/// column kept; with the check off, the folder kept.
#[test]
fn the_live_error_takes_the_status_bar_s_room() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let text = "void main()\n{\n    int n = 1 +;\n}\n";
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let folder = format!("…{}{name}", std::path::MAIN_SEPARATOR);
    let drawn = |h: &Harness<'_, Moonglow>, said: &str| {
        h.get_by_label("Status Bar").query_by_label_contains(said).is_some()
    };
    let mut widths = Vec::new();
    for width in [2400.0, 1100.0] {
        let scripts = [("broken", text.as_bytes())];
        let (mut h, keys) =
            scripts_window_with(Mode::Tabs, "ui-script-error-room", &scripts, width, Some(&root));
        assert!(h.state().script_error_for_test(keys[0]).is_some(), "the check found it");
        // (The script's hints, as the bar would say them: its count apart.)
        let hints: Vec<String> = (h.state().status_said().into_iter())
            .filter(|s| !s.starts_with("Line ") && !s.contains("broken.nss"))
            .collect();
        let message = h.get_by_label("Status Bar").get_by_label_contains("Found as you type:");
        let said = message.accesskit_node().label().unwrap_or_default();
        assert!(said.contains("broken.nss(3): "), "{said:?}");
        let at = message.rect();
        widths.push(at.width());
        assert!(drawn(&h, "Line 1, column 1"), "{width}: the cursor's count kept");
        for switch in ["Log", "Compiler", "Help"] {
            h.get_by_label("Status Bar").get_by_label(switch);
        }
        // (Nothing it stands over.)
        for (l, r) in bar_after(&h, at) {
            assert!(!r.intersects(at.shrink(0.5)), "{width}: {at:?} over {l:?} at {r:?}");
        }
        let shown: Vec<&String> = hints.iter().filter(|s| drawn(&h, s)).collect();
        if width == 2400.0 {
            // (Room for all: nothing gave way.)
            assert!(drawn(&h, &folder), "the folder beside it");
            assert_eq!(shown.len(), hints.len(), "every hint beside it: {shown:?}");
            h.state_mut().settings.no_live_check = true;
            settle(&mut h);
            assert!(drawn(&h, &folder), "the folder kept");
        } else {
            // (Some give way, from the last, the first kept.)
            assert!(!shown.is_empty() && shown.len() < hints.len(), "{width}: {shown:?}");
            assert_eq!(shown, hints.iter().take(shown.len()).collect::<Vec<_>>(), "from the last");
        }
    }
    assert!(widths[0] > 200.0, "whole, past the old cap: {widths:?}");
    assert!((widths[0] - widths[1]).abs() < 0.5, "as wide in a narrow window: {widths:?}");
    // Where even so it has no room (a window 500 wide): cut inside the bar,
    // clear of the count and the module's name, the whole of it in its tip.
    let scripts = [("broken", text.as_bytes())];
    let (mut h, _) =
        scripts_window_with(Mode::Tabs, "ui-script-error-narrow", &scripts, 500.0, Some(&root));
    let bar = h.get_by_label("Status Bar");
    let message = bar.get_by_label_contains("Found as you type:");
    let said = message.accesskit_node().label().unwrap_or_default();
    let at = message.rect();
    let room = place(&bar).expect("the bar drawn");
    assert!(at.width() < widths[0] - 1.0, "cut: {at:?}");
    assert!(room.expand(0.5).contains_rect(at), "{at:?} in the bar {room:?}");
    let after = bar_after(&h, at);
    assert!(!after.is_empty(), "the module's name after it");
    for (l, r) in &after {
        assert!(!r.intersects(at.shrink(0.5)), "{at:?} over {l:?} at {r:?}");
    }
    let tip = tip_at(&mut h, at).unwrap_or_default();
    let whole = said.trim_start_matches("Found as you type: ");
    assert!(tip.contains(whole), "the whole error in its tip: {tip:?}, not {whole:?}");
}

/// A long module name beside the live error (5.2.0, review): the name is
/// cut short to `ERROR_LEAVES` before the cursor's count goes, the count
/// kept wherever the error is shown whole, and the error, the name and
/// the count never over each other, from a wide window to a narrow one.
#[test]
fn a_long_module_name_leaves_the_error_its_room() {
    // (`shell::ERROR_LEAVES`, not exported.)
    const ERROR_LEAVES: f32 = 120.0;
    let Some(root) = mg_testkit::nwn_root() else { return };
    let long = "the_tdod_krezk_development_module_with_a_long_name_v52";
    let text = "void main()\n{\n    int n = 1 +;\n}\n";
    let key = ResKey::parse("broken", ResType::NSS).unwrap();
    let mut full: Option<(f32, f32)> = None;
    // (1450: from 1400 up the folder may show; here the switches' words
    // and the folder do not both fit beside the error.)
    for width in [2400.0, 1450.0, 1100.0, 700.0, 500.0] {
        let dir = mg_testkit::scratch_dir(&format!("ui-script-error-long-{width}"));
        let sample = sample_module(&dir);
        let path = dir.join(format!("{long}.mod"));
        std::fs::rename(&sample, &path).unwrap();
        let mut m = Module::open(&path).unwrap();
        m.set(key, text.as_bytes().to_vec());
        m.save().unwrap();
        let mut app = game_app(&root, None);
        app.settings = Mode::Tabs.settings();
        app.settings.no_last_area = true;
        app.open_module(&path);
        let mut h = harness(app, width, 800.0);
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        settle(&mut h);
        assert!(h.state().script_error_for_test(key).is_some(), "{width}: the check found it");
        let bar = h.get_by_label("Status Bar");
        let error = bar.get_by_label_contains("Found as you type:").rect();
        let after = bar_after(&h, error);
        let name = (after.iter().find(|(l, _)| l.starts_with("the_tdod")))
            .unwrap_or_else(|| panic!("{width}: the module's name in {after:?}"))
            .1;
        let count = after.iter().find(|(l, _)| l.starts_with("Line 1, column 1")).map(|c| c.1);
        let (whole_error, whole_name) = *full.get_or_insert((error.width(), name.width()));
        assert!(count.is_some() || width < 2400.0, "all of it at 2400: {after:?}");
        assert!(whole_name > ERROR_LEAVES + 40.0, "the name long enough to cut: {whole_name}");
        // (Apart.)
        let mut drawn = vec![("error", error), ("name", name)];
        drawn.extend(count.map(|c| ("count", c)));
        for (i, (a, ra)) in drawn.iter().enumerate() {
            for (b, rb) in &drawn[i + 1..] {
                assert!(
                    !ra.shrink(0.5).intersects(rb.shrink(0.5)),
                    "{width}: {a} {ra:?} over {b} {rb:?}"
                );
            }
        }
        // The folder goes first: before the hints, and so before the
        // switches' words.
        let icons = h.get_by_label("Status Bar").get_by_label("Compiler").rect().width() < 40.0;
        let folder = after.iter().any(|(l, _)| l.starts_with('…'));
        let hints: Vec<String> = (h.state().status_said().into_iter())
            .filter(|s| !s.starts_with("Line ") && !s.contains("broken.nss"))
            .collect();
        let hints_drawn = hints.iter().filter(|s| after.iter().any(|(l, _)| l == *s)).count();
        assert!(!(icons && folder), "{width}: the switches' words gone before the folder");
        assert!(
            !folder || hints_drawn == hints.len(),
            "{width}: the folder kept, {hints_drawn} of {} hints",
            hints.len()
        );
        // The name is cut no shorter than ERROR_LEAVES (less a letter, where
        // it is cut at one), and only once the error needs its room.
        assert!(name.width() >= ERROR_LEAVES - 12.0, "{width}: the name {name:?}");
        let error_whole = error.width() >= whole_error - 0.5;
        if name.width() < whole_name - 0.5 {
            assert!(width < 2400.0, "cut with room: {name:?}");
        }
        // The count goes only where the error, the name cut, still lacks
        // room: wherever the error is whole, the count is kept.
        if error_whole {
            assert!(count.is_some(), "{width}: the error whole and the count gone: {after:?}");
        } else {
            assert!(count.is_none(), "{width}: the error cut, the count kept: {after:?}");
            assert!(name.width() < ERROR_LEAVES + 1.0, "{width}: the error cut first: {name:?}");
        }
    }
}

/// With the Resources panel in use beside a script with a live error,
/// its count comes before the script's (the board's order), and goes
/// before it: the cursor's line and column is the last count to go
/// (5.2.0, review), from a wide bar to a narrow one.
#[test]
fn the_cursor_s_count_goes_last_beside_the_resources_count() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let text = "void main()\n{\n    int n = 1 +;\n}\n";
    let key = ResKey::parse("broken", ResType::NSS).unwrap();
    let mut both = false;
    let mut cursor_alone = false;
    for width in [1600.0, 1100.0, 900.0, 800.0, 700.0, 600.0, 500.0] {
        let dir = mg_testkit::scratch_dir(&format!("ui-script-error-counts-{width}"));
        let path = sample_module(&dir);
        let mut m = Module::open(&path).unwrap();
        m.set(key, text.as_bytes().to_vec());
        m.save().unwrap();
        let mut app = game_app(&root, None);
        app.settings = Mode::Tabs.settings();
        app.settings.no_last_area = true;
        app.open_module(&path);
        show_resources(&mut app);
        app.browser.filter = "nw_it_torch00".into();
        let mut h = harness(app, width, 800.0);
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        settle(&mut h);
        assert!(h.state().script_error_for_test(key).is_some(), "{width}: the check found it");
        // (The panel in use: the pointer on one of its rows.)
        let row = h.get_by_label("nw_it_torch001.uti").rect().center();
        h.hover_at(row);
        h.step();
        settle(&mut h);
        let said = h.state().status_said().to_vec();
        let at = said.iter().position(|s| s.starts_with("Line ")).expect("the cursor's count said");
        assert!(at > 0, "{width}: the panel's count before it: {said:?}");
        let panel = said[at - 1].clone();
        let drawn = |w: &str| h.get_by_label("Status Bar").query_by_label(w).is_some();
        let (p, c) = (drawn(&panel), drawn(&said[at]));
        assert!(!(p && !c), "{width}: the cursor's count went before the panel's {panel:?}");
        both |= p && c;
        cursor_alone |= c && !p;
    }
    assert!(both, "both drawn in a wide bar");
    assert!(cursor_alone, "the cursor's kept alone in a narrower one");
}

/// Compiler's Found as You Type row (5.2.0): the live error whole, over
/// the compile's messages, in either host; a click goes to its line; its
/// menu copies it (Copy), and every row with it first (Copy All); and it
/// goes once the error is mended, the panel drawn again for it.
#[test]
fn compiler_lists_the_live_error_found_as_you_type() {
    use mg_ui::row_copy::{COPY, COPY_ALL};
    let Some(root) = mg_testkit::nwn_root() else { return };
    let text = "void main()\n{\n    int n = 1 +;\n}\n";
    for mode in MODES {
        let scripts = [("broken", text.as_bytes())];
        let (mut h, keys) =
            scripts_window_with(mode, "ui-script-found-row", &scripts, 1600.0, Some(&root));
        let key = keys[0];
        let (_, error) = h.state().script_error_for_test(key).expect("the check found it");
        match mode {
            Mode::Tabs => switch_on(&mut h, "Compiler"),
            Mode::Floating => {
                window_bottom(&h, key).get_by_label("Compiler").click();
                settle(&mut h);
            }
        }
        let host = match mode {
            Mode::Tabs => h.state().bottom_dock_rect().expect("the Bottom Dock"),
            Mode::Floating => window_pane(&h, key),
        };
        // (Under the pane's tab bar, which says the error too where the
        // script floats.)
        let row = |h: &Harness<'_, Moonglow>| {
            (h.query_all_by_label_contains(&format!("{}: ", mg_ui::FOUND_AS_YOU_TYPE)))
                .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
                .filter(|n| place(n).is_some_and(|p| host.contains_rect(p)))
                .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
                .max_by(|a, b| a.1.top().total_cmp(&b.1.top()))
        };
        let (said, at) = row(&h).expect("the row in Compiler");
        assert_eq!(said, format!("{}: {error}", mg_ui::FOUND_AS_YOU_TYPE), "{mode:?}: whole");
        // Its menu.
        right_click(&mut h, at.center());
        assert_eq!(menu_copies(&mut h, COPY), std::slice::from_ref(&error), "{mode:?}: Copy");
        right_click(&mut h, at.center());
        let all = menu_copies(&mut h, COPY_ALL);
        assert!(all.len() == 1 && all[0].starts_with(&error), "{mode:?}: Copy All {all:?}");
        // A click: to its line, the third.
        h.state_mut().script_tools.jump = Some((key, 0));
        settle(&mut h);
        click_pos(&h, at.center());
        settle(&mut h);
        let (c, _) = cursor_of(&h, key).expect("its cursor");
        assert_eq!(text[..c].matches('\n').count(), 2, "{mode:?}: at its line, {c}");
        // Mended: the row goes.
        h.state_mut().script_tools.jump = Some((key, text.find("+;").unwrap() + 1));
        settle(&mut h);
        h.event(egui::Event::Text(" 2".into()));
        settle(&mut h);
        let now = h.state().script_text(key).unwrap();
        assert!(now.contains("1 + 2;"), "{mode:?}: {now:?}");
        // (Checked a moment after the typing stops: by a deadline.)
        let started = std::time::Instant::now();
        while h.state().script_error_for_test(key).is_some() && started.elapsed().as_secs() < 10 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            h.step();
        }
        // (The frame the check's result came in: the row drawn before the
        // editor still had the error, so another frame is asked for, or the
        // row would stay until something else drew the window.)
        assert!(
            row(&h).is_none() || h.ctx.has_requested_repaint(),
            "{mode:?}: the row stale with no frame asked for"
        );
        settle(&mut h);
        assert!(h.state().script_error_for_test(key).is_none(), "{mode:?}: mended");
        assert!(row(&h).is_none(), "{mode:?}: the row gone");
    }
}

/// The tips over a name (5.2.0, Options › Script Editor: Show declarations
/// in tips, on by default): under the pointer, a function's declaration
/// and its documentation's first line; where the arrow keys put the
/// cursor, the same, gone at the next key; the error's own tip wins over
/// its line; with the option off, neither.
#[test]
fn a_name_s_declaration_shows_in_a_tip() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let text = "// Adds one.\n// More words.\nint Add(int nA) { return nA + 1; }\n\
                void main()\n{\n    int n = Add(1) +;\n}\n";
    let scripts = [("tips", text.as_bytes())];
    let (mut h, keys) =
        scripts_window_with(Mode::Tabs, "ui-script-name-tips", &scripts, 1600.0, Some(&root));
    let key = keys[0];
    assert!(!h.state().settings.no_symbol_tips, "on by default");
    let t = mg_uikit::tokens(&h.ctx);
    let wave = waves(&h, t.error).pop().expect("the error's wave");
    // (Over Add on the error's line: where the wave says its characters
    // are, in its row over the wave.)
    let line = "int n = Add(1) +;";
    let char_w = (wave.last().unwrap().x - wave[0].x) / line.len() as f32;
    let over = egui::pos2(wave[0].x + 9.5 * char_w, wave[0].y - 6.0);
    let declaration = "int Add(int nA)";
    // The error's tip wins.
    h.hover_at(over);
    // (The error's own tip there: its words, then "Found as you type.")
    let started = std::time::Instant::now();
    while h.query_by_label_contains("\nFound as you type.").is_none() {
        assert!(started.elapsed().as_secs() < 10, "the error's own tip there");
        h.step();
    }
    assert!(!shows_soon(&mut h, declaration, 1), "the error's line: not the name's tip");
    // The check off: the name's.
    h.state_mut().settings.no_live_check = true;
    h.hover_at(egui::pos2(1.0, 1.0));
    settle(&mut h);
    h.hover_at(over);
    assert!(shows_soon(&mut h, declaration, 10), "the pointer's tip");
    h.get_by_label("Adds one.");
    // The cursor moved onto it by an arrow key, the pointer away.
    h.hover_at(egui::pos2(1.0, 1.0));
    let at = text.rfind("Add(1)").unwrap() + 1;
    h.state_mut().script_tools.jump = Some((key, at));
    settle(&mut h);
    assert!(!shows_soon(&mut h, declaration, 1), "not where the cursor was put");
    h.key_press(egui::Key::ArrowRight);
    assert!(shows_soon(&mut h, declaration, 10), "the cursor's tip");
    // Shift+F10: the editor's menu, and no tip over it, its keys too.
    press_now(&mut h, egui::Modifiers::SHIFT, egui::Key::F10);
    let menu = |h: &mut Harness<'_, Moonglow>| {
        settle(h);
        crate::fields::text_menu_rows(h)[3].is_some()
    };
    assert!(menu(&mut h), "the editor's menu");
    assert!(!shows_soon(&mut h, declaration, 1), "no cursor's tip over the menu");
    h.key_press(egui::Key::ArrowDown);
    assert!(!shows_soon(&mut h, declaration, 1), "nor at the menu's keys");
    // (Nor the pointer's, over Add where it is declared, three lines up,
    // clear of the menu at the cursor.)
    let declared = egui::pos2(wave[0].x + 1.5 * char_w, wave[0].y - 6.0 - 3.0 * 18.0);
    h.hover_at(declared);
    assert!(!shows_soon(&mut h, declaration, 1), "no pointer's tip while the menu is open");
    assert!(menu(&mut h), "the menu still open");
    h.key_press(egui::Key::Escape);
    assert!(!menu(&mut h), "the menu closed");
    assert!(shows_soon(&mut h, declaration, 10), "the pointer's tip there, the menu closed");
    h.hover_at(egui::pos2(1.0, 1.0));
    // The cursor's tip again, then the wheel: gone.
    h.key_press(egui::Key::ArrowLeft);
    h.key_press(egui::Key::ArrowRight);
    assert!(shows_soon(&mut h, declaration, 10), "the cursor's tip again");
    h.hover_at(over);
    h.hover_at(over + egui::vec2(0.0, -40.0));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -40.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.hover_at(egui::pos2(1.0, 1.0));
    assert!(!shows_soon(&mut h, declaration, 1), "gone at the wheel");
    // A right click on the name: its menu, no pointer's tip over it.
    right_click(&mut h, over);
    assert!(menu(&mut h), "the menu at the right click");
    assert!(!shows_soon(&mut h, declaration, 1), "no pointer's tip over the menu");
    h.key_press(egui::Key::Escape);
    h.hover_at(egui::pos2(1.0, 1.0));
    settle(&mut h);
    h.key_press(egui::Key::ArrowLeft);
    h.key_press(egui::Key::ArrowRight);
    assert!(shows_soon(&mut h, declaration, 10), "the cursor's tip once more");
    // The window made so short that the cursor's line is out of the
    // text's sight: no tip, not over what stands there; back in sight, it
    // is back (nothing pressed meanwhile).
    h.set_size(egui::vec2(1600.0, 200.0));
    assert!(!shows_soon(&mut h, declaration, 1), "the cursor out of sight: no tip");
    h.set_size(egui::vec2(1600.0, 800.0));
    assert!(shows_soon(&mut h, declaration, 10), "back in sight");
    h.event(egui::Event::Text("x".into()));
    assert!(!shows_soon(&mut h, declaration, 1), "gone at a key typed");
    // The option off: neither.
    h.state_mut().settings.no_symbol_tips = true;
    h.key_press(egui::Key::ArrowLeft);
    assert!(!shows_soon(&mut h, declaration, 1), "off: no cursor's tip");
    h.hover_at(over);
    assert!(!shows_soon(&mut h, declaration, 1), "off: no pointer's tip");
}

/// A floating script's window keeps the side and bottom panes' sizes
/// they were dragged to (5.2.0, Jasperre's report): kept in the settings
/// for the window's kind (every kind's, by the owner's ruling of 2026-11-25),
/// a script opened in a window of its own after it has them.
#[test]
fn a_script_window_s_pane_sizes_are_kept_for_the_next() {
    let (mut h, keys) = scripts_window(
        Mode::Floating,
        "ui-script-pane-sizes",
        &[("aaa", "void main()\n{\n}\n"), ("bbb", "void main()\n{\n}\n")],
        1600.0,
    );
    let leaf = |h: &Harness<'_, Moonglow>, key: ResKey| {
        let at = h.state().dock.find_tab(&Tab::Script(key)).unwrap();
        h.state().dock.iter_leaves().find(|(p, _)| p.surface == at.surface).unwrap().1.rect
    };
    // (Reference's body's left, 8 points after the pane's edge.)
    let side = |h: &Harness<'_, Moonglow>, key: ResKey| {
        let window = leaf(h, key);
        let left = (h.get_all_by_label(mg_ui::SCRIPT_REFERENCE))
            .map(|g| g.get_by_role(egui::accesskit::Role::TextInput).rect().left() - 8.0)
            .find(|l| window.x_range().contains(*l))
            .expect("its Reference");
        window.right() - left
    };
    let bottom =
        |h: &Harness<'_, Moonglow>, key: ResKey| leaf(h, key).bottom() - window_pane(h, key).top();
    own_window(&mut h, &Tab::Script(keys[0]), egui::pos2(60.0, 60.0), egui::vec2(1000.0, 700.0));
    settle(&mut h);
    let (wide, high) = (side(&h, keys[0]), bottom(&h, keys[0]));
    // The side pane's edge 80 left, the bottom pane's 40 up.
    let edge = leaf(&h, keys[0]).right() - wide;
    let y = leaf(&h, keys[0]).center().y;
    drag(&mut h, egui::pos2(edge, y), egui::pos2(edge - 80.0, y));
    settle(&mut h);
    let top = window_pane(&h, keys[0]).top();
    let x = leaf(&h, keys[0]).left() + 200.0;
    drag(&mut h, egui::pos2(x, top), egui::pos2(x, top - 40.0));
    settle(&mut h);
    let (wide2, high2) = (side(&h, keys[0]), bottom(&h, keys[0]));
    assert!((wide2 - wide - 80.0).abs() < 4.0, "dragged: {wide} to {wide2}");
    assert!((high2 - high - 40.0).abs() < 4.0, "dragged: {high} to {high2}");
    let kept = h.state().settings.dock_layouts.get("script").cloned().unwrap_or_default();
    assert!(kept.window_width.is_some() && kept.window_bottom.is_some(), "kept: {kept:?}");
    // The next window: at those sizes (bbb closed and opened again, its
    // pane's own size from before forgotten).
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Script(keys[1])));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(keys[1])));
    settle(&mut h);
    own_window(&mut h, &Tab::Script(keys[1]), egui::pos2(500.0, 100.0), egui::vec2(1000.0, 700.0));
    settle(&mut h);
    let (wide3, high3) = (side(&h, keys[1]), bottom(&h, keys[1]));
    assert!((wide3 - wide2).abs() < 2.0, "the next window's side pane: {wide3}, not {wide2}");
    assert!((high3 - high2).abs() < 2.0, "the next window's bottom pane: {high3}, not {high2}");
}

#[test]
fn closing_a_dirty_script_tab_asks_to_save_and_dont_save_drops_edits() {
    let (mut h, keys) = scripts_window(
        Mode::Tabs,
        "ui-script-dirty-close-guard",
        &[("aaa", "void main()\n{\n}\n")],
        1600.0,
    );
    let key = keys[0];
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    h.event(egui::Event::Text("/* dirty */ ".into()));
    settle(&mut h);
    assert!(
        h.state().script_text(key).unwrap().starts_with("/* dirty */ "),
        "script edited in buffer"
    );

    // Closing the tab via `Action::CloseTabs` (matching `×` / Ctrl+W) prompts first.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Script(key)]));
    settle(&mut h);
    h.get_by_label("Unsaved Changes");
    h.get_by_label("Don't Save").click();
    settle(&mut h);

    assert!(h.query_by_label("Unsaved Changes").is_none());
    assert!(h.state().dock.find_tab(&Tab::Script(key)).is_none(), "tab closed");
    // Reopening reads the unmodified module script back (dirty buffer was dropped).
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    settle(&mut h);
    assert_eq!(
        h.state().script_text(key).as_deref(),
        Some("void main()\n{\n}\n"),
        "dirty buffer discarded on Don't Save"
    );
}
