//! Options as the Options board has it (M4's U23): its pages listed with
//! their icons, OK and Cancel at its header's right end, the General page
//! in three columns that fold to fewer where it is narrow; OK setting
//! the settings once, Cancel and Escape setting back every page's
//! changes; and Options at rest on every page.

use crate::common::*;

/// The pages, as the page list names them, with their icons.
const PAGES: [(&str, mg_uikit::Icon); 10] = [
    ("Folders", mg_uikit::Icon::Folder),
    ("Area", mg_uikit::Icon::Area),
    ("General", mg_uikit::Icon::Settings),
    ("Script Editor", mg_uikit::Icon::Script),
    ("Conversation Editor", mg_uikit::Icon::Conversation),
    ("Sounds", mg_uikit::Icon::Sound),
    ("Verify", mg_uikit::Icon::Verify),
    ("Language", mg_uikit::Icon::TalkTable),
    ("Keyboard", mg_uikit::Icon::Command),
    ("Updates", mg_uikit::Icon::ArrowDown),
];

/// The window `width` by 900 in `mode`, no module open, Options open on
/// `page`.
fn options(mode: Mode, width: f32, page: &str) -> Harness<'static, Moonglow> {
    let mut h = harness(app_in(mode, Vec::new()), width, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    page_row(&h, page).click();
    settle(&mut h);
    h
}

/// The page list's row of `page` (the list is the leftmost of the names:
/// the menus and the Status Bar have some of them too).
fn page_row<'h>(h: &'h Harness<'_, Moonglow>, page: &'h str) -> egui_kittest::Node<'h> {
    let header = options_header(h);
    (h.get_all_by_label(page))
        .filter(|n| n.rect().top() > header.bottom() && n.rect().left() < header.left() + 200.0)
        .min_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .unwrap_or_else(|| panic!("{page} in the page list"))
}

/// The row Options' header stands in.
fn options_header(h: &Harness<'_, Moonglow>) -> egui::Rect {
    mg_uikit::header::records(&h.ctx, "Options header").first().expect("Options' header").row
}

/// The pointer over Options where nothing is under it: the page list's
/// room under its rows.
fn over_options(h: &mut Harness<'_, Moonglow>) {
    let last = page_row(h, "Updates").rect();
    h.hover_at(egui::pos2(last.center().x, last.bottom() + 120.0));
    settle(h);
}

/// The page list lists the ten pages, each with its icon drawn in its
/// row, the one chosen shown under its bar; OK and Cancel stand at the
/// header's right end, Cancel last; the window has no Right Dock beside
/// Options ("None on this screen").
#[test]
fn options_lists_its_pages_with_icons_and_ok_and_cancel_end_its_header() {
    let mut h = options(Mode::Tabs, 1600.0, "Folders");
    for (page, icon) in PAGES {
        let row = page_row(&h, page).rect();
        assert!(icon_drawn(&h, icon, row), "{page}'s icon in {row:?}");
        page_row(&h, page).click();
        settle(&mut h);
        let chosen = h.state().options.as_ref().map(|o| format!("{:?}", o.page));
        assert_eq!(chosen, Some(page.replace(' ', "")), "{page} chosen");
        // (The page's bar: its icon at its left, over the page.)
        let header = options_header(&h);
        let bar = egui::Rect::from_min_max(
            egui::pos2(row.right(), header.bottom()),
            egui::pos2(row.right() + 60.0, header.bottom() + 40.0),
        );
        assert!(icon_drawn(&h, icon, bar), "{page}'s bar with its icon");
    }
    let header = options_header(&h);
    let ok = h.get_by_label("Options header").get_by_label("OK").rect();
    let cancel = h.get_by_label("Options header").get_by_label("Cancel").rect();
    assert!(ok.right() <= cancel.left(), "OK before Cancel: {ok:?} {cancel:?}");
    assert!(header.right() - cancel.right() <= 12.0, "at the right end: {cancel:?} {header:?}");
    assert!(h.state().right_dock_shows().is_none(), "no Right Dock beside Options");
}

/// One control changed on each of the ten pages (General's interface
/// size and theme too, which show at once once set), as [`changed`] reads
/// them back.
fn change_every_page(h: &mut Harness<'_, Moonglow>) {
    page_row(h, "Folders").click();
    settle(h);
    click_in_sight(h, "Read userpatch.ini's haks");
    page_row(h, "Area").click();
    settle(h);
    click_in_sight(h, "Middle drag moves the view");
    page_row(h, "General").click();
    settle(h);
    click_in_sight(h, "Light theme");
    let size = (h.get_all_by_label("Interface size"))
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .unwrap();
    size.click();
    settle(h);
    h.get_by_label("125%").click();
    settle(h);
    page_row(h, "Script Editor").click();
    settle(h);
    click_in_sight(h, "Check for errors as you type");
    page_row(h, "Conversation Editor").click();
    settle(h);
    click_in_sight(h, "Show speaker name before text");
    page_row(h, "Sounds").click();
    settle(h);
    click_in_sight(h, "Play ambient music in area");
    page_row(h, "Verify").click();
    settle(h);
    click_in_sight(h, mg_module::doctor::Group::ALL[0].title());
    page_row(h, "Language").click();
    settle(h);
    click_in_sight(h, "Specified Language:");
    page_row(h, "Keyboard").click();
    settle(h);
    // (What was typed in its Find field stays, Options opened again.)
    if h.query_by_label("Remove F1 from User Manual").is_none() {
        type_into_hint(h, "Find a command", "manual");
        h.run();
    }
    // (Its cross shows with the pointer over its row.)
    click_in_sight(h, "Remove F1 from User Manual");
    settle(h);
    page_row(h, "Updates").click();
    settle(h);
    click_in_sight(h, "Include pre-releases");
}

/// Which of [`change_every_page`]'s ten changes `now` has against
/// `before`, by page.
fn changed(before: &mg_ui::Settings, now: &mg_ui::Settings) -> Vec<(&'static str, bool)> {
    let manual = now.key_bindings.get("manual").is_some_and(|k| k.is_empty());
    vec![
        ("Folders", now.read_userpatch != before.read_userpatch),
        ("Area", now.middle_drag_moves != before.middle_drag_moves),
        ("General", now.light_theme != before.light_theme && now.ui_scale == Some(125)),
        ("Script Editor", now.no_live_check != before.no_live_check),
        ("Conversation Editor", now.dialog_hide_names != before.dialog_hide_names),
        ("Sounds", now.ambient_music != before.ambient_music),
        ("Verify", now.verify_skip != before.verify_skip),
        ("Language", now.edit_language != before.edit_language),
        ("Keyboard", manual),
        ("Updates", now.update_prereleases != before.update_prereleases),
    ]
}

/// Frames run until Options comes to rest: how many changed the settings
/// (each frame's settings against the frame's before).
fn writes_while_settling(h: &mut Harness<'_, Moonglow>) -> usize {
    let mut last = h.state().settings.clone();
    let mut writes = 0;
    for _ in 0..30 {
        h.step();
        if h.state().settings != last {
            writes += 1;
            last = h.state().settings.clone();
        }
    }
    writes
}

/// Options' changes, one on each of its ten pages: OK sets every one, in
/// one apply and one change of the settings, and shows the theme and the
/// interface size at once; Cancel, Escape and its tab's × set nothing,
/// in no apply and no change of the settings, the theme and the size as
/// they were (M4's U23 fix: the counts and the ten pages). A number typed
/// and not yet left is set by OK; the Status Bar's Enter OK is there with
/// the pointer over Options and nothing typed in, and not while a field
/// is typed in.
#[test]
fn options_ok_sets_the_settings_once_and_cancel_and_esc_set_every_page_back() {
    for mode in MODES {
        ok_and_cancel(mode);
    }
}

fn ok_and_cancel(mode: Mode) {
    let mut h = options(mode, 1600.0, "Folders");
    let before = h.state().settings.clone();
    let none = |h: &Harness<'_, Moonglow>| -> Vec<&'static str> {
        (changed(&before, &h.state().settings).into_iter()).filter(|c| c.1).map(|c| c.0).collect()
    };
    let shown_as_before = |h: &Harness<'_, Moonglow>, how: &str| {
        assert_eq!(mg_uikit::applied(&h.ctx), Some(mg_uikit::Theme::Dark), "{mode:?} {how}");
        assert_eq!(h.ctx.zoom_factor(), 1.0, "{mode:?} {how}: the size as it was");
    };
    for how in ["Cancel", "Escape", "×"] {
        h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
        settle(&mut h);
        change_every_page(&mut h);
        assert_eq!(h.state().settings, before, "{mode:?} {how}: nothing set while open");
        match how {
            "Cancel" => h.get_by_label("Options header").get_by_label("Cancel").click(),
            "Escape" => {
                over_options(&mut h);
                h.key_press(egui::Key::Escape);
            }
            _ => click_tab_close(&mut h, &Tab::Options),
        }
        let writes = writes_while_settling(&mut h);
        assert!(h.state().options.is_none(), "{mode:?}: {how} closed it");
        assert_eq!(none(&h), Vec::<&str>::new(), "{mode:?}: {how} sets nothing back");
        assert_eq!(h.state().settings, before, "{mode:?}: {how}");
        assert_eq!(writes, 0, "{mode:?}: {how} changes no setting");
        assert_eq!(h.state().options_applied(), 0, "{mode:?}: {how} applies nothing");
        shown_as_before(&h, how);
    }
    // OK: each change set, in one apply and one change, with a number
    // typed and not left.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    change_every_page(&mut h);
    page_row(&h, "General").click();
    settle(&mut h);
    let minutes = number_beside(&h, "Keep a recovery copy of unsaved work every", 0);
    minutes.click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("17".into()));
    h.run();
    // (Typing: no Enter OK, which would be the field's.)
    over_options(&mut h);
    let said = h.state().status_said();
    assert!(!said.iter().any(|l| l.starts_with("Enter")), "typing: {said:?}");
    h.get_by_label("Options header").get_by_label("OK").click();
    let writes = writes_while_settling(&mut h);
    let all = changed(&before, &h.state().settings);
    assert!(all.iter().all(|c| c.1), "{mode:?}: every page's change set on OK: {all:?}");
    assert_eq!(h.state().settings.autosave_every(), 17, "the number typed and not left");
    assert_eq!(h.state().options_applied(), 1, "{mode:?}: one apply");
    assert_eq!(writes, 1, "{mode:?}: one change of the settings");
    h.run_steps(5);
    assert_eq!(h.state().options_applied(), 1, "and not again");
    assert_eq!(mg_uikit::applied(&h.ctx), Some(mg_uikit::Theme::Light), "the theme at once");
    assert_eq!(h.ctx.zoom_factor(), 1.25, "the size at once");
    // Enter OK, with the pointer over Options and nothing typed in.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    over_options(&mut h);
    let said = h.state().status_said();
    assert!(said.iter().any(|l| l == "Enter: OK"), "{said:?}");
    assert!(!said.iter().any(|l| l.contains("Save")), "no Save: {said:?}");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().options.is_none(), "Enter is OK");
    assert_eq!(h.state().options_applied(), 2);
}

/// The main window closed with Options open and changed (its X, then
/// Close Moonglow at its question; File › Exit, which does not ask): the
/// changes go with the window, unset, as at the tip, where the draft was
/// dropped unapplied (M4's U23 fix: tested).
#[test]
fn closing_the_window_with_options_changed_sets_nothing() {
    for how in ["X", "Exit"] {
        let mut h = options(Mode::Tabs, 1600.0, "Folders");
        let before = h.state().settings.clone();
        change_every_page(&mut h);
        match how {
            "X" => {
                h.state_mut().actions.push(mg_ui::Action::CloseWindow);
                h.run_steps(3);
                h.get_by_label("Close Moonglow").click();
            }
            _ => h.state_mut().actions.push(mg_ui::Action::Quit),
        }
        h.run_steps(3);
        assert!(h.state().quit_requested, "{how}: quits");
        let set: Vec<_> = (changed(&before, &h.state().settings).into_iter())
            .filter(|c| c.1)
            .map(|c| c.0)
            .collect();
        assert_eq!(set, Vec::<&str>::new(), "{how}: nothing of Options' set");
        assert_eq!(h.state().options_applied(), 0, "{how}: no apply");
        assert_eq!(mg_uikit::applied(&h.ctx), Some(mg_uikit::Theme::Dark), "{how}");
    }
}

/// Every way Options opens lands on the page it means (M4's U23 fix):
/// Tools › Options… and the Welcome page's Options… under "No game data"
/// on Folders, the first page, where the installation is chosen (the
/// tabs kept from the last session open it on their page:
/// `windows::side_editors_and_tools_open_again_with_the_module`).
#[test]
fn every_way_options_opens_lands_on_its_page() {
    let installation = "Neverwinter Nights installation";
    // Tools › Options…, after another page was left in front last time.
    let mut h = options(Mode::Tabs, 1600.0, "Sounds");
    h.get_by_label("Options header").get_by_label("Cancel").click();
    settle(&mut h);
    menu_command(&mut h, "Tools", "Options…");
    settle(&mut h);
    assert_eq!(h.state().options.as_ref().map(|o| o.page), Some(mg_ui::OptionsPage::Folders));
    assert!(h.query_by_label(installation).is_some(), "Folders drawn");
    // The Welcome page's, with no game data.
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    assert!(h.state().game.is_none());
    h.get_by_label("Options…").click();
    settle(&mut h);
    assert_eq!(h.state().options.as_ref().map(|o| o.page), Some(mg_ui::OptionsPage::Folders));
    assert!(h.query_by_label(installation).is_some(), "Folders drawn");
}

/// While Options › Keyboard waits for a key (its plus clicked), Enter is
/// the key taken, not OK, and the Status Bar offers no Enter OK (one rule
/// for both, M4's U23 fix); Escape there gives up the waiting alone,
/// Options staying open with nothing changed.
#[test]
fn enter_and_escape_while_a_key_is_awaited_are_the_keyboard_page_s() {
    let mut h = options(Mode::Tabs, 1600.0, "Keyboard");
    type_into_hint(&mut h, "Find a command", "manual");
    h.run();
    let manual =
        |h: &Harness<'_, Moonglow>| h.state().options.as_ref().unwrap().keymap.keys_of("manual");
    let before = manual(&h);
    h.get_by_label("Add a key for User Manual").click();
    settle(&mut h);
    over_options(&mut h);
    let said = h.state().status_said();
    assert!(!said.iter().any(|l| l.starts_with("Enter")), "no Enter OK while waiting: {said:?}");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().options.is_some(), "Enter was not OK");
    assert_eq!(h.state().options_applied(), 0);
    let enter = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Enter);
    assert!(manual(&h).contains(&enter), "Enter taken as the key: {:?}", manual(&h));
    assert!(h.state().options.as_ref().unwrap().recording.is_none(), "waiting no more");
    // Escape: the waiting given up, nothing else.
    let keys = manual(&h);
    h.get_by_label("Add a key for User Manual").click();
    settle(&mut h);
    over_options(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    let draft = h.state().options.as_ref().expect("Escape left Options open");
    assert!(draft.recording.is_none(), "Escape gave up the waiting");
    assert_eq!(manual(&h), keys, "no key added by Escape");
    assert_ne!(keys, before);
    assert_eq!(h.state().options_applied(), 0);
    // And Enter is OK again once nothing waits.
    let said = h.state().status_said();
    assert!(said.iter().any(|l| l == "Enter: OK"), "{said:?}");
}

/// At the board's setup (1600 wide, a module open, the Left Dock showing
/// its tree) Questions' labels stand beside their dropdowns, each choice
/// whole, as the board's (M4's U23 fix); the first column's widest row
/// (Keep a recovery copy … minutes) within its 380, the board's. Labels go
/// over their dropdowns only where the column cannot hold both.
#[test]
fn questions_stand_beside_their_dropdowns_at_the_board_s_width() {
    let dir = mg_testkit::scratch_dir("ui-u23-questions");
    let path = sample_module(&dir);
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    // (The window 1600 wide as the app is drawn: the harness puts the app
    // in a panel with egui's 8-point margin, which the app's own window
    // has not: `Moonglow::ui` is flush.)
    let mut h = harness(app, 1616.0, 916.0);
    settle(&mut h);
    assert!(h.state().left_dock_shows().is_some(), "the Left Dock open");
    assert_eq!(h.state().editor_area_rect().map(|r| r.right() - r.left() + 260.0), Some(1600.0));
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    page_row(&h, "General").click();
    settle(&mut h);
    // Flush in its pane, as the board: the page list from the pane's left
    // edge (no tab margin).
    let pages = page_row(&h, "Folders").rect();
    let pane = h.state().editor_area_rect().expect("the editor area");
    assert_eq!(
        pages.left(),
        pane.left(),
        "the page list at the pane's edge: {pages:?} in {pane:?}"
    );
    let beside = |h: &Harness<'_, Moonglow>| -> Vec<bool> {
        let questions = [
            "A .mod with its folder beside it",
            "Exporting to an archive there",
            "Files there already",
            "Equipping without the feats",
        ];
        questions
            .iter()
            .map(|q| {
                let all: Vec<egui::Rect> = h.get_all_by_label(q).map(|n| n.rect()).collect();
                let label = all.iter().min_by(|a, b| a.left().total_cmp(&b.left())).unwrap();
                let combo = all.iter().max_by(|a, b| a.left().total_cmp(&b.left())).unwrap();
                (label.center().y - combo.center().y).abs() < 4.0 && label.right() <= combo.left()
            })
            .collect()
    };
    assert_eq!(beside(&h), [true; 4], "beside at 1600 with the Left Dock");
    // (Each dropdown has its longest choice's room whole: its text, the
    // box's padding and its chevron.)
    let font = egui::FontId::proportional(13.0);
    let text = h.ctx.fonts_mut(|f| {
        f.layout_no_wrap("Ask: replace or skip them".into(), font, egui::Color32::WHITE).size().x
    });
    for q in ["A .mod with its folder beside it", "Files there already"] {
        let combo = (h.get_all_by_label(q).map(|n| n.rect()))
            .max_by(|a, b| a.left().total_cmp(&b.left()))
            .unwrap();
        assert!(combo.width() >= text + 36.0, "{q}'s dropdown {} for {text}", combo.width());
    }
    // The first column's widest row within the board's 380.
    let minutes = h.get_by_label("minutes").rect();
    let column_two = h.get_by_label("Windows and Tabs").rect().left();
    assert!(minutes.right() <= column_two - 24.0 + 0.5, "{minutes:?} past 380: {column_two}");
    // At the harness's own 1600 (the app 16 narrower) the label column
    // gives up its spare room to the longest label, and they stay beside.
    h.set_size(egui::vec2(1600.0, 916.0));
    settle(&mut h);
    assert_eq!(beside(&h), [true; 4], "beside, the label column narrower");
    let label = (h.get_all_by_label("A .mod with its folder beside it").map(|n| n.rect()))
        .min_by(|a, b| a.left().total_cmp(&b.left()))
        .unwrap();
    let words = h.ctx.fonts_mut(|f| {
        let font = egui::FontId::proportional(13.0);
        f.layout_no_wrap("A .mod with its folder beside it".into(), font, egui::Color32::WHITE)
            .size()
            .x
    });
    assert!(label.width() + 0.5 >= words, "the longest label whole: {} for {words}", label.width());
    // Narrower, where three columns still stand but the third cannot hold
    // a label and a whole choice: over.
    h.set_size(egui::vec2(1586.0, 916.0));
    settle(&mut h);
    let left = |h: &Harness<'_, Moonglow>, s: &str| h.get_by_label(s).rect().left();
    assert!(left(&h, "Questions") > left(&h, "Windows and Tabs"), "three columns at 1570");
    assert_eq!(beside(&h), [false; 4], "over at 1570");
}

/// The General page stands in three columns where the window has the
/// room (the board's), in two and then one where it is narrower, in the
/// code's order; at every width from 640 to 1900 and at three text sizes
/// no control of it is cut (each inside the page's width, the page not
/// scrolling sideways) and each has its name for a screen reader, and the
/// page is never taller where it is wider.
#[test]
fn the_general_page_folds_its_columns_never_cut_never_taller_where_wider() {
    use egui::accesskit::Role;
    let mut h = options(Mode::Tabs, 1600.0, "General");
    let order = ["Editors", "Saving and Exporting", "Opening Modules", "Testing"];
    let tops: Vec<f32> = order.iter().map(|s| h.get_by_label(s).rect().top()).collect();
    assert!(tops.windows(2).all(|t| t[0] < t[1]), "column 1 in the code's order: {tops:?}");
    let left = |h: &Harness<'_, Moonglow>, s: &str| h.get_by_label(s).rect().left();
    let columns = |h: &Harness<'_, Moonglow>| {
        let mut xs: Vec<f32> =
            ["Editors", "Windows and Tabs", "Questions"].iter().map(|s| left(h, s)).collect();
        xs.dedup_by(|a, b| (*a - *b).abs() < 1.0);
        xs.len()
    };
    assert_eq!(columns(&h), 3, "three columns at 1600");
    let controls = |h: &Harness<'_, Moonglow>| -> Vec<(String, egui::Rect)> {
        let ppp = h.ctx.pixels_per_point();
        let roles = [Role::CheckBox, Role::ComboBox, Role::SpinButton, Role::RadioButton];
        let mut all = Vec::new();
        for role in roles {
            for n in h.get_all_by_role(role) {
                let Some(b) = n.accesskit_node().bounding_box() else { continue };
                let r = egui::Rect::from_min_max(
                    egui::pos2(b.x0 as f32 / ppp, b.y0 as f32 / ppp),
                    egui::pos2(b.x1 as f32 / ppp, b.y1 as f32 / ppp),
                );
                let name = n.accesskit_node().label().unwrap_or_default();
                all.push((name, r));
            }
        }
        all
    };
    for scale in [1.0_f32, 1.25, 1.5] {
        h.ctx.set_zoom_factor(scale);
        settle(&mut h);
        let mut seen: Vec<(f32, f32, usize)> = Vec::new();
        for &width in &[640, 800, 1000, 1200, 1450, 1700, 1900] {
            // (Tall enough for the whole page in sight at one column: what
            // is out of sight has no box.)
            h.set_size(egui::vec2(width as f32, 3200.0));
            settle(&mut h);
            settle(&mut h);
            let page = options_header(&h);
            let all = controls(&h);
            assert!(all.len() >= 30, "the General page's controls at {width} x{scale}");
            let unnamed: Vec<_> = all.iter().filter(|(n, _)| n.is_empty()).collect();
            assert!(unnamed.is_empty(), "controls with no name at {width} x{scale}: {unnamed:?}");
            for (name, r) in &all {
                assert!(
                    r.right() <= page.right() + 0.5,
                    "{name} cut at {width} x{scale}: {r:?} in {page:?}"
                );
            }
            let top = all.iter().map(|(_, r)| r.top()).fold(f32::INFINITY, f32::min);
            let bottom = all.iter().map(|(_, r)| r.bottom()).fold(f32::NEG_INFINITY, f32::max);
            let tall = bottom - top;
            for (w, t, _) in &seen {
                assert!(tall <= t + 0.5, "taller at {width} ({tall}) than at {w} ({t}) x{scale}");
            }
            seen.push((width as f32, tall, columns(&h)));
        }
        let counts: Vec<usize> = seen.iter().map(|s| s.2).collect();
        assert!(
            counts.contains(&1) && counts.contains(&3),
            "one to three columns x{scale}: {counts:?}"
        );
        assert!(counts.windows(2).all(|c| c[0] <= c[1]), "more columns where wider: {counts:?}");
    }
}

/// Options comes to rest on each of its ten pages, while a field is
/// typed in, after a theme and a zoom change, and after Cancel, docked and
/// floating; the last check that Cancel closed it.
#[test]
fn options_comes_to_rest_on_every_page() {
    for mode in MODES {
        comes_to_rest(mode);
    }
}

fn comes_to_rest(mode: Mode) {
    let mut h = options(mode, 1600.0, "Folders");
    for (page, _) in PAGES {
        page_row(&h, page).click();
        rest(&mut h);
        h.run_steps(3);
        rest(&mut h);
    }
    page_row(&h, "General").click();
    settle(&mut h);
    number_beside(&h, "Keep a recovery copy of unsaved work every", 0).click();
    h.event(egui::Event::Text("3".into()));
    rest(&mut h);
    h.state_mut().settings.light_theme = true;
    rest(&mut h);
    h.ctx.set_zoom_factor(1.25);
    rest(&mut h);
    h.get_by_label("Options header").get_by_label("Cancel").click();
    rest(&mut h);
    assert!(h.state().options.is_none(), "Cancel closed it");
}

/// Options › Verify's field, under its label "Also leave out".
fn verify_field<'t>(h: &'t Harness<'_, Moonglow>) -> egui_kittest::Node<'t> {
    let label = h.get_by_label("Also leave out").rect();
    (h.get_all_by_role(egui::accesskit::Role::TextInput))
        .find(|n| (0.0..40.0).contains(&(n.rect().top() - label.top())))
        .expect("the field under Also leave out")
}

/// Options › Verify shows whatever it leaves out, where it can be taken
/// out (M5's W7): what only a player's own content has
/// (`references:override`, `userpatch`, `workshop`, `local`), which has
/// no box, is typed in its field, applied, back in the field when Options
/// opens again (it was taken for a box that is never drawn: out of sight
/// while it applied, nothing could remove it, and Missing references
/// opened with every box ticked) and taken out there, the settings
/// written as typed; a check left out of a plugin not enabled is a box
/// that ticks it back.
#[test]
fn verify_s_words_without_a_box_stay_in_its_field() {
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.verify_skip = vec!["tilesets".into(), "gone.plugin/old-check".into()];
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let open = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
        settle(h);
        page_row(h, "Verify").click();
        settle(h);
    };
    let ok = |h: &mut Harness<'_, Moonglow>| {
        h.get_by_label("Options header").get_by_label("OK").click();
        settle(h);
        assert!(h.state().options.is_none(), "OK closed it");
    };
    let type_over = |h: &mut Harness<'_, Moonglow>, text: &str| {
        verify_field(h).click();
        settle(h);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.event(egui::Event::Text(text.into()));
        settle(h);
    };
    open(&mut h);
    type_over(&mut h, "tdod_tiles01.hak, references:override");
    ok(&mut h);
    let skip = |h: &Harness<'_, Moonglow>| h.state().settings.verify_skip.clone();
    assert_eq!(
        skip(&h),
        ["tilesets", "gone.plugin/old-check", "tdod_tiles01.hak", "references:override"]
    );
    assert!(h.state().settings.saved_text().contains("\"references:override\""));
    // Opened again: in the field, and Missing references not opened on
    // boxes that are all ticked.
    open(&mut h);
    assert_eq!(
        verify_field(&h).value().as_deref(),
        Some("tdod_tiles01.hak, references:override"),
        "what applies, in sight"
    );
    assert!(h.query_by_label("Of creatures").is_none(), "Missing references closed");
    // The plugin's check, shown though the plugin is not enabled, unticked.
    let gone = h.get_by_label("gone.plugin/old-check");
    assert!(!gone.accesskit_node().is_disabled());
    assert_eq!(gone.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    // Taken out of the field, and the plugin's check ticked back.
    type_over(&mut h, "tdod_tiles01.hak");
    h.get_by_label("gone.plugin/old-check").scroll_to_me();
    settle(&mut h);
    h.get_by_label("gone.plugin/old-check").click();
    settle(&mut h);
    assert!(h.query_by_label("gone.plugin/old-check").is_some(), "kept while Options is open");
    ok(&mut h);
    assert_eq!(skip(&h), ["tilesets", "tdod_tiles01.hak"]);
    let saved = h.state().settings.saved_text();
    assert!(!saved.contains("references:override") && !saved.contains("old-check"), "{saved}");
    assert!(saved.contains("verify_skip:[\"tilesets\",\"tdod_tiles01.hak\"]"), "{saved}");
}

/// One switch for the Workshop and the test (the owner's ruling of
/// 2026-11-22): General's Testing has no Steam row of its own any more,
/// and Folders' Read Steam Workshop content, ticked and set by OK, has a
/// test start a Steam copy through Steam; unticked, without.
#[test]
fn read_steam_workshop_content_decides_how_a_test_starts() {
    let mut h = options(Mode::Tabs, 1600.0, "General");
    assert!(h.query_by_label("Minimize Toolset on test module").is_some(), "Testing shown");
    assert!(
        h.query_by_label("Start a Steam copy of the game through Steam for a test").is_none(),
        "no Steam row on General"
    );
    assert!(!h.state().settings.test_through_steam(), "without Steam by default");
    for on in [true, false] {
        page_row(&h, "Folders").click();
        settle(&mut h);
        click_in_sight(&mut h, "Read Steam Workshop content");
        settle(&mut h);
        h.get_by_label("Options header").get_by_label("OK").click();
        settle(&mut h);
        let s = &h.state().settings;
        assert_eq!((s.read_workshop, s.test_through_steam()), (on, on), "ticked: {on}");
        h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
        settle(&mut h);
    }
}

/// Options › General › Appearance › Smooth scrolling (5.2.0, a builder's
/// ask, the owner's ruling q4: one option for the wheel and the jumps):
/// ticked by default, with its tip; unticked and set by OK, the window's
/// scrolling is at once (`mg_uikit::scrolling`), and ticked again, smooth.
#[test]
fn smooth_scrolling_is_an_appearance_option_ticked_by_default() {
    let mut h = options(Mode::Tabs, 1600.0, "General");
    let ticked = |h: &Harness<'_, Moonglow>| {
        let row = apart(h, "Smooth scrolling");
        row.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
    };
    bring_in_sight(&mut h, "Smooth scrolling");
    assert!(ticked(&h), "ticked by default");
    assert!(mg_uikit::scrolling::is_smooth(&h.ctx), "smooth by default");
    // (In Appearance: under the theme, before the graphics backend.)
    let theme = apart(&h, "Light theme").rect();
    let row = apart(&h, "Smooth scrolling").rect();
    assert!(row.top() >= theme.bottom() && (row.left() - theme.left()).abs() < 1.0, "{row:?}");
    for on in [false, true] {
        click_in_sight(&mut h, "Smooth scrolling");
        settle(&mut h);
        h.get_by_label("Options header").get_by_label("OK").click();
        settle(&mut h);
        assert_eq!(h.state().settings.no_smooth_scrolling, !on, "set by OK");
        assert_eq!(mg_uikit::scrolling::is_smooth(&h.ctx), on);
        h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
        settle(&mut h);
        page_row(&h, "General").click();
        settle(&mut h);
        bring_in_sight(&mut h, "Smooth scrolling");
        assert_eq!(ticked(&h), on, "the row as set");
    }
}

/// Options › General › Appearance › Colored icons (5.2.0, a builder's ask,
/// the owner's ruling: off by default): unticked, under Smooth scrolling;
/// ticked and set by OK, the window's kind icons are colored at once
/// (`mg_uikit::hues`), and unticked again, gray.
#[test]
fn colored_icons_is_an_appearance_option_off_by_default() {
    let mut h = options(Mode::Tabs, 1600.0, "General");
    let ticked = |h: &Harness<'_, Moonglow>| {
        let row = apart(h, "Colored icons");
        row.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
    };
    bring_in_sight(&mut h, "Colored icons");
    assert!(!ticked(&h), "off by default");
    assert!(!mg_uikit::hues::are_colored(&h.ctx));
    let above = apart(&h, "Smooth scrolling").rect();
    let row = apart(&h, "Colored icons").rect();
    assert!(row.top() >= above.bottom() && (row.left() - above.left()).abs() < 1.0, "{row:?}");
    for on in [true, false] {
        click_in_sight(&mut h, "Colored icons");
        settle(&mut h);
        h.get_by_label("Options header").get_by_label("OK").click();
        settle(&mut h);
        assert_eq!(h.state().settings.colored_icons, on, "set by OK");
        assert_eq!(mg_uikit::hues::are_colored(&h.ctx), on);
        h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
        settle(&mut h);
        page_row(&h, "General").click();
        settle(&mut h);
        bring_in_sight(&mut h, "Colored icons");
        assert_eq!(ticked(&h), on, "the row as set");
    }
}

/// Smooth scrolling off: a mouse wheel's notch moves a long list (Options
/// › Keyboard's commands) its whole step, a line's 40 points, at once (the
/// frame after it arrives, as a scroll area shows any scroll), and no
/// further after. On (the control), the same notch
/// moves it part of the way in that frame and the rest after.
#[test]
fn smooth_scrolling_off_moves_a_list_by_a_whole_notch_at_once() {
    use egui::accesskit::Role;
    let step = egui::InputOptions::default().line_scroll_speed;
    for on in [false, true] {
        // (Frames of a 60th of a second, as on a screen.)
        let mut app = app_in(Mode::Tabs, Vec::new());
        app.settings.no_smooth_scrolling = !on;
        // (The common harness: it has the headers recorded, which
        // `page_row` finds the page list by.)
        let mut h = timed_harness(app, 1600.0, 900.0);
        settle(&mut h);
        h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
        settle(&mut h);
        page_row(&h, "Keyboard").click();
        settle(&mut h);
        // (The list's row of a command, its name's label.)
        let top = |h: &Harness<'_, Moonglow>| {
            let row = h.query_all_by(|n| {
                n.role() == Role::Label && n.value().as_deref() == Some("Open Folder")
            });
            row.map(|n| n.rect().top()).next().expect("Open Folder's row")
        };
        let before = top(&h);
        let at = egui::pos2(300.0, before + 6.0);
        h.hover_at(at);
        settle(&mut h);
        assert_eq!(top(&h), before, "the pointer alone moves nothing");
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -1.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        // (A scroll area moves by what it was given as it ends a frame:
        // the content shows it in the next.)
        h.step();
        h.step();
        let first = before - top(&h);
        settle(&mut h);
        let all = before - top(&h);
        if on {
            assert!(first > 1.0 && first < step - 1.0, "smooth: part of the step first: {first}");
            assert!((all - step).abs() < 0.5, "smooth: the whole step in the end: {all}");
        } else {
            assert!((first - step).abs() < 0.5, "at once: the whole step at once: {first}");
            assert_eq!(all, first, "at once: nothing after");
        }
    }
}

/// Smooth scrolling off: a jump in code (the script editor's Go to Line,
/// a result's or a bookmark's) lands at once: in sight in the frame after
/// the editor asks for it (egui sets a scroll's target as a frame ends);
/// on (the control), it is on its way for a while, and lands later.
#[test]
fn smooth_scrolling_off_lands_a_jump_at_once() {
    let dir = mg_testkit::scratch_dir("ui-smooth-jump");
    let path = sample_module(&dir);
    let key = ResKey::parse("long", ResType::NSS).unwrap();
    let text: String = (0..3000).map(|n| format!("int nLine{n} = {n};\n")).collect();
    let mut m = Module::open(&path).unwrap();
    m.set(key, text.clone().into_bytes());
    m.save().unwrap();
    let in_sight = |h: &Harness<'_, Moonglow>, start: &str| {
        let run = egui_kittest::kittest::by().role(egui::accesskit::Role::TextRun);
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        h.query_all(run)
            .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with(start)))
            .is_some_and(|n| window.contains_rect(n.rect()))
    };
    for on in [true, false] {
        let mut app = app_with(Vec::new());
        app.settings.no_smooth_scrolling = !on;
        app.open_module(&path);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        // (Frames of a 60th of a second, as on a screen: kittest's own
        // quarter of a second is nearly the animation's whole length.)
        let mut h = Harness::builder()
            .with_size(egui::vec2(800.0, 600.0))
            .with_step_dt(1.0 / 60.0)
            .build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
        // (egui's own animation, as the app has it: kittest's harness
        // turns it off.)
        let smooth = egui::style::ScrollAnimation::default();
        h.ctx.all_styles_mut(|s| s.scroll_animation = smooth);
        settle(&mut h);
        assert!(in_sight(&h, "int nLine0 ") && !in_sight(&h, "int nLine2000 "), "at the top");
        let line = mg_ui::script_tools::line_start(&text, 2000);
        h.state_mut().script_tools.jump = Some((key, line));
        let mut frames = 0;
        while !in_sight(&h, "int nLine2000 ") && frames < 60 {
            h.step();
            frames += 1;
        }
        // (Off: the frame that takes the jump, the one whose editor asks
        // for the scroll, and the next, where it lands. On, its animation
        // a tenth to three tenths of a second after that.)
        if on {
            assert!(frames > 6, "smooth: on its way for a while: {frames} frames");
        } else {
            assert!(frames <= 3, "at once: in sight in three frames: {frames}");
        }
        settle(&mut h);
        assert!(in_sight(&h, "int nLine2000 "), "smooth {on}: in the end");
    }
}

/// View › Interface Size (5.2.0, a builder on a 4K screen did not find
/// Options' row): its sizes, the one in use ticked, set the interface
/// size at once, and Options' row shows it; its Zoom In, Zoom Out and
/// Reset Zoom step through the sizes by their keys (Ctrl with +, = or -,
/// and 0: egui's own zoom keys turned off), stopping at the ends, with
/// Options open too (its row follows, and OK keeps it); 250% and 300%
/// among them.
#[test]
fn view_interface_size_sets_the_size_and_options_follows() {
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    assert!(!h.ctx.options(|o| o.zoom_with_keyboard), "egui's own zoom keys off");
    let open_sizes = |h: &mut Harness<'_, Moonglow>| {
        h.get_by_label("Top Bar").get_by_label("View").click();
        settle(h);
        let entry = menu_row(h, "Interface Size").expect("View › Interface Size").1;
        click_pos(h, entry.center());
        settle(h);
    };
    open_sizes(&mut h);
    for scale in mg_ui::UI_SCALES {
        assert!(menu_row(&h, &format!("{scale}%")).is_some(), "{scale}% offered");
    }
    let ticks = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        h.query_all_by(|n| {
            n.role() == egui::accesskit::Role::CheckBox
                && n.toggled() == Some(egui::accesskit::Toggled::True)
                && n.label().is_some_and(|l| l.ends_with('%'))
        })
        .map(|n| n.accesskit_node().label().unwrap_or_default())
        .collect()
    };
    assert_eq!(ticks(&h), ["100%"], "the size in use ticked");
    let rect = menu_row(&h, "250%").unwrap().1;
    click_pos(&h, rect.center());
    settle(&mut h);
    assert_eq!(h.state().settings.ui_scale, Some(250));
    assert!((h.ctx.zoom_factor() - 2.5).abs() < 0.001, "the size at once");
    open_sizes(&mut h);
    assert_eq!(ticks(&h), ["250%"]);
    h.key_press(egui::Key::Escape);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // Options' row shows it.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    page_row(&h, "General").click();
    settle(&mut h);
    assert_eq!(h.state().options.as_ref().unwrap().ui_scale, 250);
    let shown = (h.get_all_by_label("Interface size"))
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .map(|n| n.accesskit_node().value().unwrap_or_default());
    assert_eq!(shown.as_deref(), Some("250%"), "Options' row");
    // The keys, with Options open: its row follows them.
    let ctrl = egui::Modifiers::COMMAND;
    let keys = [
        (ctrl, egui::Key::Minus, Some(200)),
        (ctrl, egui::Key::Plus, Some(250)),
        (ctrl, egui::Key::Equals, Some(300)),
        (ctrl | egui::Modifiers::SHIFT, egui::Key::Plus, Some(300)),
        (ctrl, egui::Key::Num0, None),
        (ctrl, egui::Key::Minus, Some(90)),
        (ctrl, egui::Key::Minus, Some(90)),
        (ctrl, egui::Key::Plus, None),
    ];
    for (modifiers, key, want) in keys {
        h.key_press_modifiers(modifiers, key);
        settle(&mut h);
        assert_eq!(h.state().settings.ui_scale, want, "{modifiers:?} {key:?}");
        let zoom = f32::from(want.unwrap_or(100)) / 100.0;
        assert!((h.ctx.zoom_factor() - zoom).abs() < 0.001, "{key:?}: {}", h.ctx.zoom_factor());
        assert_eq!(h.state().options.as_ref().unwrap().ui_scale, want.unwrap_or(100), "row");
    }
    h.key_press_modifiers(ctrl, egui::Key::Equals);
    settle(&mut h);
    h.get_by_label("Options header").get_by_label("OK").click();
    settle(&mut h);
    assert_eq!(h.state().settings.ui_scale, Some(110), "OK keeps the keys' size");
}

/// The Command Palette has Zoom In, Zoom Out and Reset Zoom, each with
/// its keys, and runs them.
#[test]
fn the_command_palette_zooms_the_interface() {
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    for (typed, name, want) in [
        ("zoom in", "Zoom In", Some(110)),
        ("zoom in", "Zoom In", Some(125)),
        ("zoom out", "Zoom Out", Some(110)),
        ("reset zoom", "Reset Zoom", None),
    ] {
        command_palette(&mut h, typed);
        let first = quick_open_rows(&h).into_iter().next().unwrap_or_default();
        assert!(first.starts_with(name), "{typed}: {first}");
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(h.state().settings.ui_scale, want, "{typed}");
    }
    assert_eq!(mg_ui::commands::name_of("zoom-in"), "Zoom In");
    let keys = |id: &str| h.state().keymap.label_of(id, &h.ctx);
    for id in ["zoom-in", "zoom-out", "reset-zoom"] {
        assert!(keys(id).starts_with("Ctrl"), "{id}: {}", keys(id));
    }
}

/// Options › General › Interface size on a screen with no room for the
/// largest sizes (1366×768: the least window at 250% is 800 pixels high):
/// those are shown but not offered, with the reason, as in View ›
/// Interface Size; 200% is.
#[test]
fn interface_sizes_the_screen_has_no_room_for_are_not_offered() {
    let mut h = options(Mode::Tabs, 1600.0, "General");
    let root = h.input_mut().viewports.entry(egui::ViewportId::ROOT).or_default();
    root.monitor_size = Some(egui::vec2(1366.0, 768.0));
    settle(&mut h);
    fn combo<'h>(h: &'h Harness<'_, Moonglow>) -> egui_kittest::Node<'h> {
        (h.get_all_by_label("Interface size"))
            .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
            .unwrap()
    }
    let open = |h: &mut Harness<'static, Moonglow>| {
        combo(h).scroll_to_me();
        at_rest(h, "Interface size", &|h| combo(h).rect());
        combo(h).click();
        settle(h);
    };
    open(&mut h);
    let off =
        |h: &Harness<'_, Moonglow>, size: &str| h.get_by_label(size).accesskit_node().is_disabled();
    assert!(!off(&h, "100%") && !off(&h, "200%"), "sizes that fit");
    assert!(off(&h, "250%") && off(&h, "300%"), "sizes that do not");
    let at = h.get_by_label("300%").rect();
    let tip = tip_at(&mut h, at);
    assert_eq!(tip.as_deref(), Some("The screen has no room for this size"));
    // (A size that fits, chosen.)
    h.get_by_label("200%").click();
    settle(&mut h);
    h.get_by_label("Options header").get_by_label("OK").click();
    settle(&mut h);
    assert_eq!(h.state().settings.ui_scale, Some(200));
    // A larger size saved (for a larger screen) stays as saved and
    // offered; the others too large are not. (At 100% again: the screen
    // given above is in points at 100%.)
    h.state_mut().settings.ui_scale = None;
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    page_row(&h, "General").click();
    settle(&mut h);
    h.state_mut().options.as_mut().unwrap().ui_scale = 300;
    settle(&mut h);
    open(&mut h);
    assert!(!off(&h, "300%") && off(&h, "250%"), "the saved size offered");
}
