//! The script editor's header and its Reference and Help panels (M4's
//! U12; the Script and Floating boards): every control of the header
//! reachable at every width, docked and floating, with Reference and
//! Help in reach beside it; Reference on the right and Help under the
//! text in either host (the docks, the script's window's panes: 5.2.0's
//! ruling on Jasperre's report); Help brought forward without anything
//! kept; and records from before read as they were.

use crate::common::*;
use mg_uikit::header::{self, Place};

/// The header's controls by their names (the codepage's, its words, is
/// added by the test: it is the script's).
const SCRIPT_CONTROLS: [&str; 14] = [
    "Save",
    "Compile",
    "Save As…",
    "Print…",
    "Open…",
    "External Editor",
    "Find…",
    "Replace…",
    "Bookmark",
    "Definition",
    "References",
    "Rename Symbol…",
    "Used By",
    "To Scratch",
];

/// A module with one script, `hello`, of `text`, open in `mode` in a
/// window `width` wide, the script's tab open.
fn script_window(
    mode: Mode,
    name: &str,
    text: &str,
    width: f32,
) -> (Harness<'static, Moonglow>, ResKey) {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = app_in(mode, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, width, 700.0);
    settle(&mut h);
    (h, key)
}

/// Whether the panel `name` (Reference or Help) is in reach: its header
/// ("Reference, nwscript and includes"), its tab, its button on the
/// folded Right Dock's strip or its Status Bar switch (Help's).
fn in_reach(h: &Harness<'_, Moonglow>, name: &str) -> bool {
    let header = h.query_all_by_label_contains(&format!("{name}, ")).next().is_some();
    // (Help's own tab or strip button, apart from the Top Bar's Help menu.)
    let own = h.query_all_by_label(name).filter(|n| !in_group(n, "Top Bar")).count();
    header || own > 0
}

/// The script header holds every control (on its row, as an icon, or in
/// More) at every window width from the least to 1600 points, both ways
/// editors open, To Scratch always in More, the codepage at the row's
/// right end; and Reference and Help stay in reach beside it (Reference
/// in the Right Dock, its strip where the window folds it, or the
/// window's pane, narrowed to half the window; Help's tab or its Status
/// Bar switch, under the text).
#[test]
fn the_script_header_holds_every_control_from_420_to_1600() {
    let mut codepage_in_more = 0;
    for mode in MODES {
        let text = "void main()\n{\n}\n";
        let (mut h, _) = script_window(mode, "ui-script-header-widths", text, 1600.0);
        let mut widths: Vec<f32> =
            (21..=80).step_by(4).map(|n| n as f32 * 20.0).chain(std::iter::once(1600.0)).collect();
        widths.extend([1399.0, 1400.0, 961.0, 962.0, 795.0, 796.0]);
        for width in widths {
            h.set_size(egui::vec2(width, 700.0));
            settle(&mut h);
            let rec = header::records(&h.ctx, mg_ui::SCRIPT_HEADER);
            let rec = rec.into_iter().next().unwrap_or_else(|| panic!("no header at {width}"));
            let names: Vec<String> =
                rec.places.iter().map(|(n, _)| n.clone()).filter(|n| !n.is_empty()).collect();
            // (Every control, and the codepage after them.)
            assert_eq!(names.len(), SCRIPT_CONTROLS.len() + 1, "{mode:?} {width}: {names:?}");
            assert_eq!(&names[..SCRIPT_CONTROLS.len()], SCRIPT_CONTROLS, "{mode:?} {width}");
            let codepage = names.last().unwrap().clone();
            assert!(codepage.starts_with("Windows-"), "{codepage}");
            let place = |n: &str| rec.places.iter().find(|(p, _)| p == n).map(|(_, p)| *p);
            assert_eq!(place("To Scratch"), Some(Place::More), "{mode:?} {width}");
            // The codepage yields into More only after the left group's
            // secondary commands (Save and Compile go last of all).
            if place(&codepage) == Some(Place::More) {
                codepage_in_more += 1;
                for n in &SCRIPT_CONTROLS[2..] {
                    assert_eq!(
                        place(n),
                        Some(Place::More),
                        "{mode:?} {width}: {n} before the codepage"
                    );
                }
            }
            // OK and Cancel at the end, on the row whatever the width.
            let end = rec.end.expect("the end slot");
            for n in ["OK", "Cancel"] {
                let r = h.get_by_label(mg_ui::SCRIPT_HEADER).get_by_label(n).rect();
                assert!(end.expand(0.5).contains_rect(r), "{mode:?} {width}: {n} {r:?} in {end:?}");
            }
            let mut all: Vec<&str> = SCRIPT_CONTROLS.to_vec();
            all.push(&codepage);
            header_holds(&mut h, mg_ui::SCRIPT_HEADER, &all);
            for panel in ["Reference", "Help"] {
                assert!(in_reach(&h, panel), "{mode:?} {width}: {panel} out of reach");
            }
            // (Where Reference is drawn, what it lists is chosen whole: its
            // segmented control where the four words fit, else a list.)
            if let Some(body) = h.query_by_label(mg_ui::SCRIPT_REFERENCE) {
                use egui::accesskit::Role;
                let field = body.get_by_role(Role::TextInput).rect();
                let choices = body.query_all_by(|n| {
                    matches!(n.role(), Role::RadioButton | Role::ComboBox)
                        && n.bounding_box().is_some()
                });
                let mut seen = 0;
                for choice in choices {
                    let r = choice.rect();
                    let inside = r.left() >= field.left() - 1.0 && r.right() <= field.right() + 1.0;
                    assert!(inside, "{mode:?} {width}: {r:?} past {field:?}");
                    seen += 1;
                }
                assert!(seen == 1 || seen == 4, "{mode:?} {width}: {seen} choices");
            }
        }
    }
    // (The sweep reaches a row short enough for the codepage to go: a
    // small window's.)
    assert!(codepage_in_more > 0, "the codepage never went into More");
}

/// Reference stands on the right and Help under the text, in either host
/// (5.2.0, the owner's ruling on Jasperre's report, as 4.2.1 had them):
/// docked, Reference in the Right Dock and Help in the Bottom Dock;
/// floating, the script's window's right and bottom panes, not the main
/// window's docks. A click on a function shows it in Help, brought to the
/// front under the text, as wide as it, the declaration as written; a
/// double click puts it in at the cursor.
#[test]
fn reference_on_the_right_and_help_under_the_text_in_either_host() {
    for mode in MODES {
        let text = "int Add(int nA) { return nA; }\nvoid main() { }\n";
        let (mut h, key) = script_window(mode, "ui-script-reference-hosts", text, 1600.0);
        let script = Tab::Script(key);
        let floating = !h.state().dock.find_tab(&script).unwrap().surface.is_main();
        assert_eq!(floating, !mode.tabs(), "{mode:?}");
        let add = reference_row(&h, "Add").rect();
        let dock = h.state().right_dock_rect();
        match floating {
            // (The window's right pane: inside the script's window, right
            // of its text; the main window's dock, if drawn, not it.)
            true => {
                let at = h.state().dock.find_tab(&script).unwrap();
                let pane = h.state().dock.iter_leaves().find(|(p, _)| p.surface == at.surface);
                let window = pane.expect("its window").1.rect;
                assert!(window.expand(1.0).contains_rect(add), "{add:?} in {window:?}");
                assert!(dock.is_none_or(|d| !d.contains_rect(add)), "not the dock's");
            }
            false => {
                let dock = dock.expect("the Right Dock beside the docked script");
                assert!(dock.contains_rect(add), "{add:?} in {dock:?}");
                assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Reference));
            }
        }
        reference_row(&h, "Add").click();
        settle(&mut h);
        // (Help's body, under the editor's text and across it.)
        let help = h.get_by_label("Help's text").rect();
        let editor = h.ctx.read_response(egui::Id::new(("script", key))).unwrap().rect;
        assert!(help.top() >= editor.bottom() - 1.0, "{mode:?}: {help:?} under {editor:?}");
        assert!(
            help.left() <= editor.left() + 1.0 && help.right() >= editor.right() - 1.0,
            "{mode:?}: {help:?} as wide as {editor:?}"
        );
        assert!(!help.intersects(add), "{mode:?}: not on Reference's side");
        match floating {
            true => {
                let window = h.state().window_bottom_for_test(&script);
                assert_eq!(window, Some(mg_ui::Panel::Help), "the window's bottom pane");
                assert_ne!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Help), "not the dock");
            }
            false => {
                assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Help));
                let dock = h.state().bottom_dock_rect().expect("the Bottom Dock");
                assert!(dock.expand(1.0).contains_rect(help), "{help:?} in {dock:?}");
            }
        }
        // (The declaration as the script has it, on its line.)
        h.get_by_label("int Add(int nA)");
        // A double click puts it in, at the cursor (the text's start).
        h.state_mut().script_tools.jump = Some((key, 0));
        settle(&mut h);
        let at = reference_row(&h, "Add").rect().center();
        double_click_now(&mut h, at);
        settle(&mut h);
        let now = h.state().script_text(key).unwrap();
        assert!(now.starts_with("Add(int Add("), "{mode:?}: {now:?}");
    }
}

/// The Status Bar's switch `name` turned on, if it is off.
fn switch_on(h: &mut Harness<'_, Moonglow>, name: &str) {
    let switch = h.get_by_label("Status Bar").get_by_label(name);
    if switch.accesskit_node().toggled() != Some(egui::accesskit::Toggled::True) {
        switch.click();
        settle(h);
    }
}

/// Help comes forward under the text by the dock's panel alone, nothing
/// kept in the settings for the script's kind: a click on a name in
/// Reference brings it to the Bottom Dock's front, opening the dock where
/// it was closed; with another panel in front, Definition (here from the
/// header, as F12 does) brings it forward again. Reference stays in front
/// on the right, alone there.
#[test]
fn help_comes_forward_in_the_bottom_dock() {
    let text = "int Add(int nA) { return nA; }\nvoid main() { Add(1); }\n";
    let (mut h, key) = script_window(Mode::Tabs, "ui-script-help-forward", text, 1200.0);
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Reference));
    // (No tab or header named Help in the Right Dock.)
    let on_right = h
        .query_all_by_label("Help")
        .filter_map(|n| n.accesskit_node().bounding_box().map(|_| n.rect()))
        .any(|r| h.state().right_dock_rect().is_some_and(|d| d.contains_rect(r)));
    assert!(!on_right, "not on the right");
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    assert!(h.state().bottom_dock_rect().is_none(), "the Bottom Dock closed");
    reference_row(&h, "Add").click();
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Help), "opened on Help");
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Reference), "the list kept");
    switch_on(&mut h, "Compiler");
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Compiler));
    // The cursor on the call, then Definition.
    let call = text.find("Add(1)").unwrap() + 1;
    h.state_mut().script_tools.jump = Some((key, call));
    settle(&mut h);
    script_click(&mut h, "Definition");
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Script(key)));
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Help), "Help forward");
    let kept = h.state().settings.dock_layouts.get("script").cloned().unwrap_or_default();
    assert_eq!(kept.folded, None, "nothing kept: {kept:?}");
}

/// What a newer or older Moonglow could have kept naming the script's old
/// panes reads without a fault: the settings' layout for scripts (with the
/// two sections' shares a dragged seam kept) naming
/// Help as the Bottom Dock's front (where it is again since 5.2.0: kept)
/// and as the Right Dock's (no longer there: passed over), and a side
/// list's tab or Help as the folded bar's (passed over), an open-tabs record
/// whose script view names its side list and Help tab (fields this
/// version has not: passed over, the scroll and cursor kept). The script
/// opens with Reference on the right, and the Bottom Dock on Help.
#[test]
fn records_naming_the_old_side_pane_or_help_tab_read_as_before() {
    // (The Right Dock's two sections' shares, which a dragged seam kept, and
    // a folded bar naming Help or a side list: Reference is alone now, at
    // either width.)
    for (folded, width) in [("Constants", 1600.0), ("Help", 1200.0)] {
        let dir = mg_testkit::scratch_dir(&format!("ui-script-old-records-{width}"));
        let path = sample_module(&dir);
        let key = ResKey::parse("hello", ResType::NSS).unwrap();
        let mut m = Module::open(&path).unwrap();
        m.set(key, b"void main()\n{\n    int n = 1;\n}\n".to_vec());
        m.save().unwrap();
        let kept = dir.join("open-tabs");
        std::fs::create_dir_all(&kept).unwrap();
        let record = r#"{"panes":[{"place":"middle","tabs":["script:hello.nss"]}],
            "front":"script:hello.nss",
            "views":{"script:hello.nss":{"cursor":[20,20],"side":"Constants","info":"Help",
            "filter":"PROPERTY_"}}}"#;
        std::fs::write(mg_ui::open_tabs::file(&kept, &path), record).unwrap();
        let text = format!(
            r#"(dock_layouts:{{"script":(fronts:["Help","Functions"],shares:[56,44],bottom:Some("Help"),folded:Some("{folded}"))}})"#
        );
        let (settings, unread) = mg_ui::Settings::read_fields(&text).expect("read");
        assert!(unread.is_empty(), "{unread:?}");
        let mut app = app_in(Mode::Tabs, Vec::new());
        app.settings.dock_layouts = settings.dock_layouts;
        app.settings.hide_log = false;
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(kept);
        app.open_module(&path);
        let mut h = harness(app, width, 800.0);
        settle(&mut h);
        assert_eq!(h.state().active_editor(), Some(&Tab::Script(key)));
        let at = egui::text_edit::TextEditState::load(&h.ctx, egui::Id::new(("script", key)))
            .and_then(|s| s.cursor.char_range())
            .map(|r| usize::from(r.primary.index));
        assert_eq!(at, Some(20), "{folded}: its cursor kept");
        reference_row(&h, "main");
        assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Reference));
        assert_eq!(
            h.state().bottom_dock_shows(),
            Some(mg_ui::Panel::Help),
            "the Bottom Dock on Help"
        );
        h.get_by_label("Help's text");
    }
}

/// The header has no "modified" label: an unsaved script shows the dot
/// on its tab, nothing more on its header.
#[test]
fn the_header_says_nothing_of_unsaved_changes() {
    let (mut h, key) =
        script_window(Mode::Tabs, "ui-script-no-modified", "void main() {}\n", 1600.0);
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    h.event(egui::Event::Text("a".into()));
    settle(&mut h);
    assert!(h.state().script_text(key).unwrap().starts_with("avoid"));
    assert!(!script_tool(&mut h, "Save").accesskit_node().is_disabled(), "Save enabled");
    let header = h.get_by_label(mg_ui::SCRIPT_HEADER);
    assert!(header.query_by_label_contains("modified").is_none());
}

/// A script in a small window of its own (450 points: its pane at half
/// the window): nothing of the editor takes the pointer over the pane,
/// Reference and Help, drawn before it (the bottom pane's tabs, wider
/// than the editor's room, were drawn and answered over them); and
/// Reference's choice of list is a list to choose from, whole, where its
/// four words have no room.
#[test]
fn a_small_window_s_editor_keeps_off_its_pane() {
    let (mut h, key) =
        script_window(Mode::Floating, "ui-script-small-window", "void main() {}\n", 1000.0);
    let script = Tab::Script(key);
    own_window(&mut h, &script, egui::pos2(100.0, 60.0), egui::vec2(450.0, 560.0));
    settle(&mut h);
    // The pane: from Reference's body's margin (8 points before its
    // filter) to the window's right edge.
    let body = h.get_by_label(mg_ui::SCRIPT_REFERENCE);
    let pane_left = body.get_by_role(egui::accesskit::Role::TextInput).rect().left() - 8.0;
    let at = h.state().dock.find_tab(&script).unwrap();
    let leaf = h.state().dock.iter_leaves().find(|(p, _)| p.surface == at.surface).unwrap().1.rect;
    assert!(
        pane_left > leaf.left() + 100.0 && pane_left < leaf.right() - 100.0,
        "{pane_left} {leaf:?}"
    );
    let pane = egui::Rect::from_x_y_ranges(pane_left + 1.0..=leaf.right(), leaf.y_range());
    // (The widgets as they take the pointer, cut to where they are drawn.)
    let mut over = Vec::new();
    h.ctx.viewport(|v| {
        for (layer, widgets) in v.prev_pass.widgets.layers() {
            // (The window's, a click's controls: not the window's own bar
            // and frame, nor the pane's edge to drag.)
            if layer.order != egui::Order::Middle {
                continue;
            }
            for w in widgets.iter() {
                let r = w.interact_rect;
                let control = w.sense.senses_click() && r.height() <= 30.0 && r.width() >= 12.0;
                let below_bar = r.top() >= leaf.top() + 30.0;
                if r.is_positive()
                    && control
                    && below_bar
                    && r.intersects(pane)
                    && r.left() < pane.left() - 1.0
                {
                    over.push(r);
                }
            }
        }
    });
    assert!(over.is_empty(), "widgets reaching over the pane at {pane_left}: {over:?}");
    let body = h.get_by_label(mg_ui::SCRIPT_REFERENCE);
    assert_eq!(body.query_all_by_role(egui::accesskit::Role::RadioButton).count(), 0);
    body.get_by_label("What to list");
}

/// F6 goes to Reference as to any panel, and puts the keyboard in its
/// filter (`Keys::Reference`), so that a name can be typed at once.
#[test]
fn f6_puts_the_keyboard_in_reference_s_filter() {
    let text = "int Add(int nA) { return nA; }\nvoid main() { }\n";
    let (mut h, _) = script_window(Mode::Tabs, "ui-script-f6", text, 1600.0);
    let focused = |h: &Harness<'_, Moonglow>| {
        let body = h.get_by_label(mg_ui::SCRIPT_REFERENCE);
        body.get_by_role(egui::accesskit::Role::TextInput).accesskit_node().is_focused()
    };
    assert!(!focused(&h));
    let mut stops = 0;
    while !focused(&h) {
        stops += 1;
        assert!(stops <= 8, "F6 never reached Reference's filter");
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    // Typed there: the list narrowed to it.
    h.event(egui::Event::Text("Ad".into()));
    settle(&mut h);
    assert_eq!(h.state().script_tools.filter, "Ad");
    reference_row(&h, "Add");
    assert!(h.get_by_label(mg_ui::SCRIPT_REFERENCE).query_by_label("main").is_none());
}

/// The window comes to rest where Reference is not drawn (M4's U12
/// review, A1: it asked for a frame again every frame while what it last
/// drew differed from what the editor made): Help in the Bottom Dock's
/// front, then typed in; with the Right Dock folded, then typed in; Help
/// drawn, then behind Compiler, a name chosen; nothing chosen, then
/// Definition (Help forward). Each `settle` fails where the window keeps
/// drawing.
#[test]
fn the_window_comes_to_rest_where_reference_is_not_drawn() {
    let text = "int Add(int nA) { return nA; }\nvoid main() { Add(1); }\n";
    // Help in front under the text, typed in.
    let (mut h, key) = script_window(Mode::Tabs, "ui-script-rest-help", text, 1200.0);
    switch_on(&mut h, "Help");
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Help));
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    h.event(egui::Event::Text("int Sub(int n) { return n; }\n".into()));
    settle(&mut h);
    assert!(h.state().script_text(key).unwrap().starts_with("int Sub"));
    // The dock folded, typed in.
    let (mut h, key) = script_window(Mode::Tabs, "ui-script-rest-folded", text, 1600.0);
    fold_palette(h.state_mut());
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), None, "folded");
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    h.event(egui::Event::Text("int Sub(int n) { return n; }\n".into()));
    settle(&mut h);
    // Help drawn, then Compiler in front (Help not drawn), then a name
    // chosen by the text's cursor (a double click).
    let (mut h, key) = script_window(Mode::Tabs, "ui-script-rest-hidden-help", text, 1200.0);
    switch_on(&mut h, "Help");
    switch_on(&mut h, "Compiler");
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Compiler));
    h.state_mut().script_tools.jump = Some((key, text.find("Add(1)").unwrap() + 1));
    settle(&mut h);
    let symbol = {
        let (_, symbols) = &h.state().script_tools.symbols[&key];
        symbols.iter().find(|s| s.name == "Add").cloned()
    };
    // (As the cursor on it chooses it, with the editor drawn after.)
    h.state_mut().script_tools.help = symbol;
    settle(&mut h);
    // Nothing chosen, Definition.
    let (mut h, key) = script_window(Mode::Tabs, "ui-script-rest-definition", text, 1200.0);
    assert!(h.state().script_tools.help.is_none());
    h.state_mut().script_tools.jump = Some((key, text.find("Add(1)").unwrap() + 1));
    settle(&mut h);
    script_click(&mut h, "Definition");
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Help));
}

/// What Reference does goes to its own script, with two open (each in a
/// window of its own, each with its pane): a double click in one's list
/// puts the name in that one, not the other; and a function typed shows
/// in Reference by the frames the typing asks for (no settling).
#[test]
fn reference_works_on_its_own_script_and_follows_typing() {
    let dir = mg_testkit::scratch_dir("ui-script-two-references");
    let path = sample_module(&dir);
    let a = ResKey::parse("aaa", ResType::NSS).unwrap();
    let b = ResKey::parse("bbb", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(a, b"int InA() { return 1; }\nvoid main() { }\n".to_vec());
    m.set(b, b"int InB() { return 2; }\nvoid main() { }\n".to_vec());
    m.save().unwrap();
    let mut app = app_in(Mode::Floating, Vec::new());
    app.settings.no_last_area = true;
    app.settings.scripts_own_windows = true;
    app.open_module(&path);
    let mut h = harness(app, 2000.0, 1000.0);
    settle(&mut h);
    for key in [a, b] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        settle(&mut h);
    }
    own_window(&mut h, &Tab::Script(a), egui::pos2(40.0, 60.0), egui::vec2(900.0, 700.0));
    own_window(&mut h, &Tab::Script(b), egui::pos2(1000.0, 60.0), egui::vec2(900.0, 700.0));
    settle(&mut h);
    for key in [a, b] {
        h.state_mut().script_tools.jump = Some((key, 0));
        settle(&mut h);
    }
    // A double click in each one's list (each its own function): into
    // that one alone, whichever window is drawn first.
    for (name, left) in [("InA", true), ("InB", false)] {
        let row = (h.get_all_by_label(mg_ui::SCRIPT_REFERENCE))
            .find_map(|g| g.query_by_label(name).map(|n| n.rect()))
            .expect("in a Reference");
        assert_eq!(row.left() < 950.0, left, "{name}'s window's pane: {row:?}");
        double_click_now(&mut h, row.center());
        settle(&mut h);
    }
    let (ta, tb) = (h.state().script_text(a).unwrap(), h.state().script_text(b).unwrap());
    assert!(ta.starts_with("InA(int InA"), "{ta:?}");
    assert!(tb.starts_with("InB(int InB"), "{tb:?}");
    // A function typed into b: in b's Reference by the frames it asks for.
    h.state_mut().script_tools.jump = Some((b, 0));
    settle(&mut h);
    // (No other frames asked for: the caret not blinking, no check as you
    // type.)
    h.state_mut().settings.no_live_check = true;
    h.ctx.all_styles_mut(|s| s.visuals.text_cursor.blink = false);
    settle(&mut h);
    h.event(egui::Event::Text("int Typed() { return 3; }\n".into()));
    // (Frame by frame: while Reference does not show it, a frame more is
    // asked for; a window that stops asking would show the old list.)
    let shown = |h: &Harness<'_, Moonglow>| {
        h.get_all_by_label(mg_ui::SCRIPT_REFERENCE).any(|g| g.query_by_label("Typed").is_some())
    };
    let mut frames = 0;
    loop {
        h.step();
        frames += 1;
        if shown(&h) {
            break;
        }
        assert!(
            h.ctx.has_requested_repaint(),
            "no frame asked for after {frames}, Typed not shown"
        );
        assert!(frames < 10, "never shown");
    }
}

/// Definition in a floating script leaves the docked script's Right Dock
/// as it is (A4): Help comes forward in the window that asked, whose pane
/// shows it already, not in the dock of another script.
#[test]
fn definition_in_a_floating_script_leaves_the_docked_one_s_dock() {
    let dir = mg_testkit::scratch_dir("ui-script-forward-floating");
    let path = sample_module(&dir);
    let a = ResKey::parse("aaa", ResType::NSS).unwrap();
    let b = ResKey::parse("bbb", ResType::NSS).unwrap();
    let text = b"int Add(int nA) { return nA; }\nvoid main() { Add(1); }\n".to_vec();
    let mut m = Module::open(&path).unwrap();
    m.set(a, text.clone());
    m.set(b, text);
    m.save().unwrap();
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 900.0);
    settle(&mut h);
    for key in [a, b] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        settle(&mut h);
    }
    own_window(&mut h, &Tab::Script(b), egui::pos2(100.0, 80.0), egui::vec2(1000.0, 700.0));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Script(a)));
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Reference));
    let call = "int Add(int nA) { return nA; }\nvoid main() { Add(1); }\n".find("Add(1)").unwrap();
    h.state_mut().script_tools.jump = Some((b, call + 1));
    settle(&mut h);
    // (b's header: the one in its window, by where its Definition is.)
    let window = {
        let at = h.state().dock.find_tab(&Tab::Script(b)).unwrap();
        h.state().dock.iter_leaves().find(|(p, _)| p.surface == at.surface).unwrap().1.rect
    };
    let definition = h
        .get_all_by_label(mg_ui::SCRIPT_HEADER)
        .filter_map(|g| g.query_by_label("Definition").map(|n| n.rect()))
        .find(|r| window.contains_rect(*r))
        .expect("b's Definition on its header's row");
    click_pos(&h, definition.center());
    settle(&mut h);
    assert_eq!(h.state().script_tools.help.as_ref().map(|s| s.name.as_str()), Some("Add"));
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Reference), "a's dock as it was");
    assert!(h.state().script_tools.help_forward_pending().is_none());
}

/// Definition of the game's function (`nwscript.nss` opens, read-only, in
/// front), then back to the script: Help is in the Bottom Dock's front
/// (A3: kept until a script is in front again).
#[test]
fn help_comes_forward_once_the_script_is_back_in_front() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-script-forward-game");
    let path = sample_module(&dir);
    let key = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = "void main() { ActionSit(OBJECT_SELF); }\n";
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.as_bytes().to_vec());
    m.save().unwrap();
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let mut h = harness(app, 1200.0, 800.0);
    settle(&mut h);
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Reference));
    h.state_mut().script_tools.jump = Some((key, text.find("ActionSit").unwrap() + 2));
    settle(&mut h);
    script_click(&mut h, "Definition");
    settle(&mut h);
    assert!(
        matches!(h.state().active_editor(), Some(Tab::Resource(_))),
        "nwscript.nss in front: {:?}",
        h.state().active_editor()
    );
    click_tab(&mut h, &Tab::Script(key));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Script(key)));
    assert_eq!(h.state().bottom_dock_shows(), Some(mg_ui::Panel::Help), "Help in front");
}

/// F6 to Help (under the text) takes the keyboard from the script's text
/// (A5): what is typed then does not go into the script.
#[test]
fn f6_to_help_takes_the_keyboard_from_the_text() {
    let text = "void main() { }\n";
    let (mut h, key) = script_window(Mode::Tabs, "ui-script-f6-help", text, 1600.0);
    switch_on(&mut h, "Help");
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    h.event(egui::Event::Text("a".into()));
    settle(&mut h);
    assert!(h.state().script_text(key).unwrap().starts_with("avoid"), "the text has the keys");
    let mut stops = 0;
    while mg_ui::palette_view::arrow_keys_are(&h.ctx) != "Help" {
        stops += 1;
        assert!(stops <= 8, "F6 never reached Help");
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    assert!(h.get_by_label("Help's text").accesskit_node().is_focused(), "Help has the focus");
    h.event(egui::Event::Text("z".into()));
    settle(&mut h);
    assert!(h.state().script_text(key).unwrap().starts_with("avoid"), "nothing typed into it");
}

/// The list goes to the name Help shows when it is chosen elsewhere (the
/// text's cursor on it): far down the list, its row comes into sight.
#[test]
fn reference_s_list_goes_to_the_name_chosen_in_the_text() {
    let mut text = String::new();
    for i in 0..80 {
        text.push_str(&format!("int F{i:02}() {{ return {i}; }}\n"));
    }
    text.push_str("void main() { F77(); }\n");
    let (mut h, key) = script_window(Mode::Tabs, "ui-script-list-goes", &text, 1600.0);
    assert!(
        h.get_by_label(mg_ui::SCRIPT_REFERENCE).query_by_label("F77").is_none(),
        "out of sight"
    );
    // A click in the text on the call chooses it for Help.
    let at = text.find("F77();").unwrap() + 1;
    h.state_mut().script_tools.jump = Some((key, at));
    settle(&mut h);
    let symbol = {
        let (_, symbols) = &h.state().script_tools.symbols[&key];
        symbols.iter().find(|s| s.name == "F77").cloned()
    };
    h.state_mut().script_tools.help = symbol;
    settle(&mut h);
    reference_row(&h, "F77");
}

/// OK and Cancel stand at the header's right end (the proposal's ruling
/// 10 of section 15.2: every tab with OK and Cancel keeps them, at the
/// header's end), docked and floating, with their tips and what they do:
/// OK puts the text into the module and closes the tab; Cancel with
/// unsaved text asks first (the tab stays), and without closes it.
#[test]
fn ok_and_cancel_stand_at_the_header_s_end() {
    for mode in MODES {
        let text = "void main() {}\n";
        let (mut h, key) = script_window(mode, "ui-script-ok-cancel", text, 1600.0);
        let script = Tab::Script(key);
        let rec = header::records(&h.ctx, mg_ui::SCRIPT_HEADER).into_iter().next().unwrap();
        let end = rec.end.expect("an end slot");
        let header = h.get_by_label(mg_ui::SCRIPT_HEADER);
        let (ok, cancel) = (header.get_by_label("OK").rect(), header.get_by_label("Cancel").rect());
        assert!(end.expand(0.5).contains_rect(ok) && end.expand(0.5).contains_rect(cancel));
        assert!(ok.right() <= cancel.left() && (end.right() - cancel.right()).abs() < 1.0);
        // (No pair at the foot: in the script's pane, the header's OK alone.)
        let pane = {
            let at = h.state().dock.find_tab(&script).unwrap();
            let leaf = h
                .state()
                .dock
                .iter_leaves()
                .find(|(p, _)| p.node == at.node && p.surface == at.surface);
            leaf.unwrap().1.rect
        };
        let in_pane = h.get_all_by_label("OK").filter(|n| pane.contains_rect(n.rect())).count();
        assert_eq!(in_pane, 1, "{mode:?}");
        // Typed, then Cancel: asked first, the tab still open.
        h.state_mut().script_tools.jump = Some((key, 0));
        settle(&mut h);
        h.event(egui::Event::Text("int n;\n".into()));
        settle(&mut h);
        h.get_by_label(mg_ui::SCRIPT_HEADER).get_by_label("Cancel").click();
        settle(&mut h);
        assert!(h.state().dock.find_tab(&script).is_some(), "{mode:?}: asked, not closed");
        // (The question's own Cancel: keep editing.)
        h.get_by_label("Don't Save");
        h.get_all_by_label("Cancel").last().unwrap().click();
        settle(&mut h);
        assert!(h.query_by_label("Don't Save").is_none(), "{mode:?}: the question answered");
        // OK: into the module, closed, its editor's state kept for the
        // session as the question's Save keeps it (a bookmark: what only
        // the editor's state has).
        h.state_mut().scripts_mut_bookmark_for_test(key, 0);
        h.get_by_label(mg_ui::SCRIPT_HEADER).get_by_label("OK").click();
        settle(&mut h);
        assert!(h.state().dock.find_tab(&script).is_none(), "{mode:?}: closed by OK");
        let saved = h.state().ws.as_ref().unwrap().module.get(&key).map(<[u8]>::to_vec);
        assert!(saved.unwrap().starts_with(b"int n;"), "{mode:?}: saved into the module");
        assert_eq!(h.state().script_bookmarks(key), [0], "{mode:?}: its bookmark kept");
        // Opened again: nothing unsaved in it, so its close asks nothing.
        h.state_mut().actions.push(mg_ui::Action::OpenTab(script.clone()));
        settle(&mut h);
        assert_eq!(h.state().script_bookmarks(key), [0], "{mode:?}: reopened, its bookmark");
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![script.clone()]));
        settle(&mut h);
        assert!(h.query_by_label("Unsaved Changes").is_none(), "{mode:?}: kept clean");
        assert!(h.state().dock.find_tab(&script).is_none(), "{mode:?}: closed again");
    }
}

/// A function typed into a docked script shows in its Reference by the
/// frames the typing asks for (the dock is drawn before the editor, which
/// makes the script's symbols anew after it: it asks for one frame more),
/// the caret not blinking and no check as you type to ask for others.
#[test]
fn a_function_typed_shows_in_reference_by_the_frames_asked_for() {
    let (mut h, key) =
        script_window(Mode::Tabs, "ui-script-follow-typing", "void main() {}\n", 1600.0);
    h.state_mut().settings.no_live_check = true;
    // (Nor the conversations' backups' timer, which asks for a frame in
    // five minutes: any request counts as one here.)
    h.state_mut().settings.dialog_no_backup = true;
    h.ctx.all_styles_mut(|s| s.visuals.text_cursor.blink = false);
    h.state_mut().script_tools.jump = Some((key, 0));
    settle(&mut h);
    let shown = |h: &Harness<'_, Moonglow>| {
        h.get_by_label(mg_ui::SCRIPT_REFERENCE).query_by_label("Typed").is_some()
    };
    assert!(!shown(&h));
    h.event(egui::Event::Text("int Typed() { return 3; }\n".into()));
    // (The typing's frame, then each frame asked for.)
    h.step();
    let mut frames = 1;
    while !shown(&h) {
        assert!(
            h.ctx.has_requested_repaint(),
            "no frame asked for after {frames}, Typed not shown"
        );
        assert!(frames < 10, "never shown");
        h.step();
        frames += 1;
    }
    // (The typing's frame, the one that makes the symbols anew, the one
    // that shows them.)
    assert!(frames <= 3, "shown after {frames} frames");
}

/// Two docked scripts side by side, the right one's group with the focus:
/// a double click in the Right Dock's Reference (the right one's) puts the
/// name into the right one, though the left one's editor is drawn first.
#[test]
fn an_insert_goes_to_the_script_whose_reference_it_is() {
    let dir = mg_testkit::scratch_dir("ui-script-insert-target");
    let path = sample_module(&dir);
    let a = ResKey::parse("aaa", ResType::NSS).unwrap();
    let b = ResKey::parse("bbb", ResType::NSS).unwrap();
    let mut m = Module::open(&path).unwrap();
    m.set(a, b"int InA() { return 1; }\nvoid main() { }\n".to_vec());
    m.set(b, b"int InB() { return 2; }\nvoid main() { }\n".to_vec());
    m.save().unwrap();
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Script(a)));
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    {
        let dock = &mut h.state_mut().dock;
        let node = dock.find_tab(&Tab::Script(a)).unwrap().node;
        dock.main_surface_mut().split_right(node, 0.5, vec![Tab::Script(b)]);
    }
    settle(&mut h);
    click_tab(&mut h, &Tab::Script(b));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Script(b)));
    for key in [a, b] {
        h.state_mut().script_tools.jump = Some((key, 0));
        settle(&mut h);
    }
    let row = reference_row(&h, "InB").rect();
    double_click_now(&mut h, row.center());
    settle(&mut h);
    let (ta, tb) = (h.state().script_text(a).unwrap(), h.state().script_text(b).unwrap());
    assert!(tb.starts_with("InB(int InB"), "{tb:?}");
    assert!(ta.starts_with("int InA"), "a untouched: {ta:?}");
}
