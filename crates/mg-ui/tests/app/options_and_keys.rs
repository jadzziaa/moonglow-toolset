//! Options, key bindings, menus by the keyboard, the command palette,
//! Help and the update check.

use crate::common::*;

#[test]
fn options_choose_the_game_folder() {
    let dir = mg_testkit::scratch_dir("ui-options");
    let app = Moonglow::new(
        None,
        Box::new(NoDialogs { folders: vec![dir.clone()], ..Default::default() }),
    );
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    // Browse picks a folder that is not a game install.
    h.get_all_by_label("Browse…").next().unwrap().click();
    h.run();
    h.get_by_label("Not a game installation (no data/nwn_base.key).");
    h.get_by_label("OK").click();
    h.run();
    assert_eq!(h.state().settings.game_root.as_deref(), Some(dir.as_path()));
    assert_eq!(h.state().install.as_ref().map(|i| i.root.clone()), Some(dir));
    assert!(h.state().game.is_none());
    assert!(
        h.state().log.entries.iter().any(|(_, m)| m.starts_with("Could not load the game data"))
    );
}

/// `moonglow --user-dir DIR`: Options › Folders says which user folder this
/// run uses, and choosing another there (saved for the next run) neither
/// changes this run's nor reads the game again.
#[test]
fn options_say_the_user_folder_of_this_run() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-options-run-user");
    let (run, saved, other) = (dir.join("run"), dir.join("saved"), dir.join("other"));
    for d in [&run, &saved, &other] {
        std::fs::create_dir_all(d.join("modules")).unwrap();
    }
    let settings = mg_ui::Settings {
        game_root: Some(root),
        user_dir: Some(saved),
        user_dir_this_run: Some(run.clone()),
        ..Default::default()
    };
    let app = Moonglow::with_settings(
        settings,
        Box::new(NoDialogs { folders: vec![other.clone()], ..Default::default() }),
    );
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let user = |h: &Harness<Moonglow>| h.state().install.as_ref().and_then(|i| i.user_dir.clone());
    assert_eq!(user(&h), Some(run.clone()));
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label(&format!("This run uses {} (--user-dir), not this folder.", run.display()));
    // (The user folder's Browse is the second.)
    h.get_all_by_label("Browse…").nth(1).unwrap().click();
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert_eq!(h.state().settings.user_dir.as_deref(), Some(other.as_path()));
    assert_eq!(user(&h), Some(run));
}

/// Options › OK leaves the module open, its unsaved work with it, and asks
/// nothing: only another game or user folder has it read again, and then
/// it is opened again.
#[test]
fn options_leave_the_module_open() {
    let dir = mg_testkit::scratch_dir("ui-options-open");
    let path = sample_module(&dir);
    let other = dir.join("elsewhere");
    std::fs::create_dir_all(&other).unwrap();
    let dialogs = NoDialogs { folders: vec![other.clone()], ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // Unsaved work: the module's tag changed.
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let edit = mg_edit::Edit::SetField {
        key: ifo,
        path: mg_edit::GffPath::root(),
        label: "Mod_Tag".into(),
        value: Some(mg_gff::Value::String(b"CHANGED".to_vec())),
    };
    let command = mg_edit::Command::new("Tag", vec![edit]);
    h.state_mut().actions.push(mg_ui::Action::Apply(command));
    h.run();
    assert!(h.state().has_unsaved_work());

    // A switch changed, OK: applied, the module and its change still there.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("General").click();
    h.run();
    h.get_by_label("Light theme").click();
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert!(h.state().settings.light_theme);
    assert!(h.state().confirm_discard.is_none(), "nothing to ask");
    assert!(h.state().has_unsaved_work(), "the change is kept");
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tag"));

    // Another game folder: unsaved work is asked about first.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_all_by_label("Browse…").next().unwrap().click();
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert!(h.state().confirm_discard.is_some(), "asked before the module is read again");
    assert_eq!(h.state().settings.game_root, None, "not applied until answered");
    // Saved, the same options reopen the module where it was.
    let ask = h.state_mut().confirm_discard.take().unwrap();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    h.state_mut().actions.push(ask);
    h.run();
    assert_eq!(h.state().settings.game_root.as_deref(), Some(other.as_path()));
    assert_eq!(h.state().module_path().as_deref(), Some(path.as_path()), "opened again");
    assert!(!h.state().has_unsaved_work());
}

#[test]
fn the_command_palette_finds_a_command_and_runs_it() {
    let dir = mg_testkit::scratch_dir("ui-command-palette");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    let open = |h: &mut Harness<'_, Moonglow>| {
        h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
        h.run();
        assert!(h.state().command_palette.is_some(), "Ctrl+Shift+P opens it");
    };
    let hint = "Type a command's name";

    // Every command is there until something is typed; then those whose
    // name or menu has every word.
    open(&mut h);
    h.get_by_label("Verify Module");
    type_into_hint(&mut h, hint, "mod build");
    assert!(h.query_by_label("Open Module…").is_none());
    // (By name first; then the Build menu's other commands with "mod".)
    h.get_by_label("Build Module…");
    h.get_by_label("Verify Module");
    // Enter runs the one chosen, and the palette closes.
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().build.is_some() && h.state().command_palette.is_none());
    h.state_mut().build = None;
    h.run();

    // The arrows choose among several: the second of the two editors.
    open(&mut h);
    type_into_hint(&mut h, hint, "editor");
    h.get_by_label("Faction Editor");
    h.key_press(egui::Key::ArrowDown);
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Journal).is_some());
    assert!(h.state().dock.find_tab(&Tab::Factions).is_none());

    // A command that can't be chosen now is listed, and Enter leaves it.
    open(&mut h);
    type_into_hint(&mut h, hint, "redo");
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().command_palette.is_some(), "nothing to redo: nothing done");
    // A click runs one; Escape closes without running anything.
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().command_palette.is_none());
    open(&mut h);
    type_into_hint(&mut h, hint, "about");
    h.get_by_label("About Moonglow Toolset").click();
    h.run();
    assert!(h.state().command_palette.is_none());
    h.get_by_label_contains("GNU General Public License");
    // While it is open, keys are its own: Ctrl+N starts no module.
    open(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    assert!(h.state().command_palette.is_some() && h.state().ws.is_some());
}

#[test]
fn keys_remapped_in_options_and_used() {
    use mg_ui::keys::Cmd;
    let dir = mg_testkit::scratch_dir("ui-keys");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.dialog_no_text_popup = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Keyboard").click();
    h.run();
    // Every command of the menus is listed, with or without keys; the list
    // narrows to what is typed.
    h.get_by_label("Build Module");
    type_into_hint(&mut h, "Find a command", "manual");
    h.run();
    assert!(h.query_by_label("Build Module").is_none());
    // User Manual: F1 taken away, Ctrl+M added by pressing it.
    h.get_by_label("F1 ×").click();
    h.run();
    h.state_mut().options.as_mut().unwrap().recording = Some(Cmd::Manual.id().to_string());
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::M);
    h.run();
    // A key two commands share is named.
    h.state_mut().options.as_mut().unwrap().recording = Some(Cmd::Manual.id().to_string());
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run();
    assert!(h.query_by_label_contains("is the key of both Save and User Manual").is_some());
    // The User Manual's Ctrl+S is taken away again (the only row shown).
    h.get_by_label("Ctrl+S ×").click();
    h.run();
    // A menu command that never had a key takes one.
    h.state_mut().options.as_mut().unwrap().recording = Some("build-module".to_string());
    h.run();
    h.key_press(egui::Key::F8);
    h.run();
    h.get_by_label("OK").click();
    h.run();
    let chosen = &h.state().settings.key_bindings;
    assert_eq!(chosen.get("manual"), Some(&vec!["Ctrl+M".to_string()]), "{chosen:?}");
    assert_eq!(chosen.get("build-module"), Some(&vec!["F8".to_string()]), "{chosen:?}");
    assert_eq!(chosen.len(), 2);
    // Its key opens its window, and its menu entry shows the key.
    h.key_press(egui::Key::F8);
    h.run();
    assert!(h.state().build.is_some(), "F8 opens Build Module");
    h.state_mut().build = None;
    h.run();
    h.get_by_label("Build").click();
    h.run();
    assert!(h.query_by_label_contains("F8").is_some());
    h.key_press(egui::Key::Escape);
    h.run();

    // F1 no longer opens the manual; Ctrl+M does.
    h.key_press(egui::Key::F1);
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Manual).is_none());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::M);
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Manual).is_some());
    // The menu shows the new key.
    h.get_by_label("Help").click();
    h.run();
    assert!(h.query_by_label_contains("Ctrl+M").is_some() || cfg!(target_os = "macos"));
    h.key_press(egui::Key::Escape);
    h.run();

    // Ctrl+A adds a line in the conversation editor, with the pointer over
    // it, as in Aurora.
    let tab = h.state().dock.find_tab(&Tab::Manual).unwrap();
    h.state_mut().dock.remove_tab(tab);
    h.state_mut().new_dialog = Some("keysdlg".into());
    h.run();
    h.get_by_label("Create").click();
    h.run();
    let key = ResKey::parse("keysdlg", ResType::DLG).unwrap();
    let starts = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&key).unwrap().root.list("StartingList").map_or(0, <[_]>::len)
    };
    assert_eq!(starts(&mut h), 0);
    // (The conversation editor's: the module tree and the palettes have one too.)
    let at = h.get_all_by_label("Expand All").last().unwrap().rect().center();
    h.hover_at(at);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run();
    assert_eq!(starts(&mut h), 1);
}

/// GitHub issue 9: the menus by the keyboard. F10 opens the first (and
/// closes them), Left and Right go from menu to menu, Up and Down from row
/// to row past those that can't be chosen, Enter chooses, Right opens a
/// submenu and Left closes it; and with a menu open, the pointer over
/// another's name opens that one.
#[test]
fn menus_by_the_keyboard() {
    let dir = mg_testkit::scratch_dir("ui-menu-keys");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run();
        h.run();
        h.run();
    };
    let shown = |h: &Harness<'_, Moonglow>, label: &str| {
        h.query_all_by_label_contains(label).next().is_some()
    };
    // F10: File, open; F10 again: closed.
    assert!(!shown(&h, "Save As"));
    key(&mut h, egui::Key::F10);
    assert!(shown(&h, "Save As"));
    key(&mut h, egui::Key::F10);
    assert!(!shown(&h, "Save As"));
    // Right twice from File: View (round the ends the other way: Help).
    key(&mut h, egui::Key::F10);
    key(&mut h, egui::Key::ArrowLeft);
    assert!(shown(&h, "About Moonglow") && !shown(&h, "Save As"));
    key(&mut h, egui::Key::ArrowRight);
    key(&mut h, egui::Key::ArrowRight);
    key(&mut h, egui::Key::ArrowRight);
    assert!(shown(&h, "Hide All Panels"));
    // Its first row is marked; Down passes the palettes (no game here:
    // they can't be chosen) to the log, and Enter folds it away.
    key(&mut h, egui::Key::ArrowDown);
    assert!(!h.state().settings.hide_log);
    key(&mut h, egui::Key::Enter);
    assert!(h.state().settings.hide_log && !h.state().settings.hide_tree);
    assert!(!shown(&h, "Hide All Panels"), "chosen, the menus close");
    // Up from the first row is the last: Reset Layout brings the log back.
    key(&mut h, egui::Key::F10);
    key(&mut h, egui::Key::ArrowRight);
    key(&mut h, egui::Key::ArrowRight);
    key(&mut h, egui::Key::ArrowUp);
    key(&mut h, egui::Key::Space);
    assert!(!h.state().settings.hide_log);

    // A submenu: Tools, up to Options and three more to Tilesets; Right
    // opens it, Left closes it, and the menu stays.
    key(&mut h, egui::Key::F10);
    for _ in 0..4 {
        key(&mut h, egui::Key::ArrowRight);
    }
    assert!(shown(&h, "Faction Editor") && !shown(&h, "Open Tileset"));
    for _ in 0..4 {
        key(&mut h, egui::Key::ArrowUp);
    }
    key(&mut h, egui::Key::ArrowRight);
    assert!(shown(&h, "Open Tileset"), "the submenu is open");
    key(&mut h, egui::Key::ArrowLeft);
    key(&mut h, egui::Key::ArrowLeft);
    assert!(shown(&h, "Rotate Area") || shown(&h, "Area Wizard"), "Left again: the menu before");
    key(&mut h, egui::Key::Escape);

    // Alt and a menu's letter opens it; a letter then goes to the next
    // row that begins with it (L: the Log), which Enter chooses.
    let alt = |h: &mut Harness<'_, Moonglow>, k: egui::Key| {
        h.key_press_modifiers(egui::Modifiers::ALT, k);
        h.run();
        h.run();
        h.run();
    };
    alt(&mut h, egui::Key::V);
    assert!(shown(&h, "Hide All Panels") && !shown(&h, "Save As"));
    key(&mut h, egui::Key::L);
    key(&mut h, egui::Key::Enter);
    assert!(h.state().settings.hide_log, "the row the letter went to");
    alt(&mut h, egui::Key::V);
    key(&mut h, egui::Key::R);
    key(&mut h, egui::Key::Enter);
    assert!(!h.state().settings.hide_log, "Reset Layout");
    // Alt and another menu's letter, with one open: that one.
    alt(&mut h, egui::Key::V);
    alt(&mut h, egui::Key::H);
    assert!(shown(&h, "About Moonglow") && !shown(&h, "Hide All Panels"));
    key(&mut h, egui::Key::Escape);
    assert!(!shown(&h, "About Moonglow"));
    // Escape in a submenu closes the submenu, and the menu stays.
    alt(&mut h, egui::Key::T);
    for _ in 0..4 {
        key(&mut h, egui::Key::ArrowUp);
    }
    key(&mut h, egui::Key::ArrowRight);
    assert!(shown(&h, "Open Tileset"));
    key(&mut h, egui::Key::Escape);
    assert!(!shown(&h, "Open Tileset"), "the submenu closed");
    assert!(shown(&h, "Faction Editor"), "one level");
    key(&mut h, egui::Key::Escape);
    assert!(!shown(&h, "Faction Editor"));

    // By the mouse: with File open, the pointer over Edit opens Edit.
    h.get_by_label("File").click();
    h.run();
    assert!(shown(&h, "Save As"));
    h.get_by_label("Edit").hover();
    h.run();
    h.run();
    h.run();
    assert!(shown(&h, "Module Properties") && !shown(&h, "Save As"));
}

#[test]
fn the_user_manual_opens_from_help_and_follows_its_links() {
    let mut h = harness(app_with(Vec::new()), 1280.0, 800.0);
    h.run();
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label_contains("User Manual").click();
    h.run();
    // The contents, with the chapters beside them.
    h.get_by_label("Moonglow Toolset: User Manual");
    // Chapters in the list are buttons; links in the text are not.
    let listed =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::Button;
    assert!(h.get_all_by_label("Troubleshooting").any(|n| listed(&n)));
    // A link in the text opens its chapter in place.
    h.get_all_by_label("Getting started").find(|n| !listed(n)).unwrap().click();
    h.run();
    h.get_by_label("What you need");
    // The chapter list picks another.
    h.get_all_by_label("Areas").find(|n| listed(n)).unwrap().click();
    h.run();
    h.get_by_label("Placing objects");
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label("About Moonglow Toolset").click();
    h.run();
    h.get_by_label(&format!("Version {}", mg_core::release::version()));
}

/// Options › General › Interface size: the whole interface larger, and
/// back to the usual size when set back.
#[test]
fn the_interface_size_is_set_in_options() {
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(Vec::new()));
    h.run();
    assert_eq!(h.ctx.zoom_factor(), 1.0);
    h.state_mut().settings.ui_scale = Some(150);
    h.run();
    assert_eq!(h.ctx.zoom_factor(), 1.5);
    h.state_mut().settings.ui_scale = None;
    h.run();
    assert_eq!(h.ctx.zoom_factor(), 1.0);
}

/// Options › General › Graphics backend: Automatic and this platform's
/// backends only (Direct3D 12 on Windows alone, Metal on macOS alone),
/// saved in the settings, from the next start.
#[test]
fn the_graphics_backend_is_chosen_in_options() {
    use mg_ui::GraphicsBackend;
    let mut h = harness(app_with(Vec::new()), 1280.0, 800.0);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("General").click();
    h.run();
    h.get_by_label("Takes effect the next time Moonglow starts.");
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let automatic =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().value().as_deref() == Some("Automatic");
    h.query_all(combo).find(automatic).expect("the graphics backend's dropdown").click();
    h.run();
    let offered = GraphicsBackend::offered(mg_ui::graphics::Platform::current());
    let all = [GraphicsBackend::Vulkan, GraphicsBackend::Direct3d12, GraphicsBackend::Metal];
    for b in all {
        let shown = h.query_by_label(b.name()).is_some();
        assert_eq!(shown, offered.contains(&b), "{} offered here", b.name());
    }
    // (The last one offered: Vulkan on Linux, Direct3D 12 on Windows, Metal
    // on macOS.)
    let last = *offered.last().unwrap();
    h.get_by_label(last.name()).click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert_eq!(h.state().settings.graphics_backend, last);
    // Kept as the settings are, for the next start.
    let saved = serde_json::to_string(&h.state().settings).unwrap();
    let back: mg_ui::Settings = serde_json::from_str(&saved).unwrap();
    assert_eq!(back.graphics_backend, last);
}

/// Options › General › Toolbar buttons, Icons only: the toolbar's buttons
/// show their glyphs alone at once, each named in its tip (with its key),
/// and the toolbar takes less room; kept in the settings, and back to
/// icons and text.
#[test]
fn the_toolbar_shows_icons_only_when_asked() {
    let mut h = harness(app_with(Vec::new()), 1280.0, 800.0);
    h.run();
    let width = |h: &Harness<'_, Moonglow>, first: &str, last: &str| {
        h.get_by_label(last).rect().right() - h.get_by_label(first).rect().left()
    };
    let wide = width(&h, "🗋 New", "✔ Verify");
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("General").click();
    h.run();
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let both = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().value().as_deref() == Some("Icons and text")
    };
    h.query_all(combo).find(both).expect("the toolbar buttons' dropdown").click();
    h.run();
    h.get_by_label("Icons only").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().settings.toolbar_icons_only);
    let saved = serde_json::to_string(&h.state().settings).unwrap();
    let back: mg_ui::Settings = serde_json::from_str(&saved).unwrap();
    assert!(back.toolbar_icons_only, "kept");
    let old: mg_ui::Settings = serde_json::from_str(r#"{"light_theme":true}"#).unwrap();
    assert!(!old.toolbar_icons_only, "icons and text in settings from before");
    h.run();
    // The glyphs alone.
    assert!(h.query_by_label("💾 Save").is_none() && h.query_by_label("📦 Palettes").is_none());
    let narrow = width(&h, "🗋", "✔");
    assert!(narrow < wide * 0.6, "the toolbar narrower: {narrow} of {wide}");
    // A tip named for the button: first where it doesn't begin so.
    let palettes = h.get_by_label("📦").rect().center();
    h.hover_at(palettes);
    // (A tooltip waits a moment.)
    h.run_steps(60);
    h.get_by_label("Palettes\nBlueprint palettes");
    // Save (disabled without a module): its tip too, with its key.
    let save = h.get_by_label("💾").rect().center();
    h.hover_at(save);
    h.run_steps(60);
    let tip = h.get_by_label_contains("Save module").accesskit_node();
    let tip = tip.label().or(tip.value()).unwrap_or_default();
    assert!(tip.starts_with("Save module ("), "named, with its key: {tip}");
    // Back to icons and text.
    h.state_mut().settings.toolbar_icons_only = false;
    h.run();
    h.get_by_label("💾 Save");
    assert!(h.query_by_label("💾").is_none());
}

/// Options › General › Light theme.
#[test]
fn the_theme_is_set_in_options() {
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app_with(Vec::new()));
    h.run();
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    h.state_mut().settings.light_theme = true;
    h.run();
    assert_eq!(h.ctx.theme(), egui::Theme::Light);
}

/// GitHub's answer for the latest release (as `/releases/latest` gives
/// it), tagged `tag`.
fn release_answer(tag: &str) -> String {
    serde_json::json!({
        "tag_name": tag,
        "html_url": format!("https://github.com/jadzziaa/moonglow-toolset/releases/tag/{tag}"),
        "draft": false,
        "prerelease": false,
    })
    .to_string()
}

/// An app whose checks for updates get `answer` (never the network), and
/// how many times it was asked.
fn app_with_releases(
    answer: Result<String, String>,
) -> (Moonglow, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let asked = std::sync::Arc::new(AtomicUsize::new(0));
    let mut app = app_with(Vec::new());
    let counted = asked.clone();
    app.updates.fetch = Some(std::sync::Arc::new(move || {
        counted.fetch_add(1, Ordering::SeqCst);
        answer.clone()
    }));
    (app, asked)
}

/// Help › Check for Updates… says a newer release is out, with its page,
/// or that this one is the newest, or why GitHub couldn't be asked; a
/// copy a package manager updates says how.
#[test]
fn check_for_updates_says_what_is_newest() {
    use std::sync::atomic::Ordering;
    let current = mg_core::release::version();
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    // (No automatic check: one was made a moment ago.)
    app.settings.update_checked = Some(mg_ui::updates::now());
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 0, "not due");
    h.get_by_label("Help").click();
    h.run();
    h.get_by_label_contains("Check for Updates").click();
    h.run();
    h.get_by_label(&format!("Moonglow 99.0.0 is out (you have {current})."));
    h.get_by_label("Open the release page");
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    assert!(h.state().updates.notice.is_none(), "the notice is the automatic check's");
    // The newest.
    h.state_mut().updates.fetch =
        Some(std::sync::Arc::new(move || Ok(release_answer(&format!("v{current}")))));
    h.get_by_label("Check Again").click();
    h.run();
    h.get_by_label(&format!("Moonglow {current} is the newest."));
    // Offline.
    h.state_mut().updates.fetch =
        Some(std::sync::Arc::new(|| Err("not connected to the Internet".into())));
    h.get_by_label("Check Again").click();
    h.run();
    h.get_by_label("Couldn't ask GitHub: not connected to the Internet.");
    // Installed by Scoop: still asked, and said how it is updated.
    h.state_mut().updates.installer = Some(mg_ui::updates::Installer::Scoop);
    h.state_mut().updates.fetch = Some(std::sync::Arc::new(|| Ok(release_answer("v99.0.0"))));
    // (Drawn so first: the window is taller, centered again a frame
    // later, its button elsewhere.)
    h.run_steps(2);
    h.get_by_label("Check Again").click();
    h.run();
    h.get_by_label(&format!("Moonglow 99.0.0 is out (you have {current})."));
    h.get_by_label("This copy is updated by Scoop: scoop update moonglow.");
    h.get_by_label("Close").click();
    h.run();
    assert!(h.query_by_label("Check Again").is_none());
    // And from Help › About.
    h.state_mut().about = true;
    h.run();
    h.get_by_label("Check for Updates…").click();
    h.run();
    h.get_by_label("Check Again");
}

/// The automatic check, due, tells of a newer release under the toolbar
/// at start; Don't remind me about this version keeps it from telling of
/// that one again (a later one is told), and × closes it for now.
#[test]
fn the_update_notice_appears_and_is_dismissed() {
    use std::sync::atomic::Ordering;
    let current = mg_core::release::version();
    let told = format!("Moonglow 99.0.0 is out (you have {current}).");
    let (app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    assert_eq!(app.settings.update_checked, None);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    h.get_by_label(&told);
    h.get_by_label("Release Page");
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    let checked = h.state().settings.update_checked.expect("the check's time is kept");
    assert!(checked.abs_diff(mg_ui::updates::now()) < 60);
    // (Once: frames go on without asking again.)
    h.run_steps(5);
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    h.get_by_label("Don't remind me about this version").click();
    h.run();
    assert!(h.query_by_label(&told).is_none());
    assert_eq!(h.state().settings.update_skipped.as_deref(), Some("99.0.0"));

    // The next day: the skipped version isn't told; a later one is.
    let mut settings = h.state().settings.clone();
    settings.update_checked = Some(checked - mg_ui::updates::DAY);
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings = settings.clone();
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 1, "asked, a day having gone by");
    assert!(h.query_by_label(&told).is_none(), "the skipped version isn't told again");
    let (mut app, _) = app_with_releases(Ok(release_answer("v99.1.0")));
    app.settings = settings;
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    h.get_by_label(&format!("Moonglow 99.1.0 is out (you have {current})."));
    h.get_by_label("×").click();
    h.run();
    assert!(h.state().updates.notice.is_none());
    assert_eq!(h.state().settings.update_skipped.as_deref(), Some("99.0.0"));

    // Offline: nothing said, in the log or elsewhere.
    let (app, asked) = app_with_releases(Err("not connected to the Internet".into()));
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    assert!(h.state().updates.notice.is_none());
    assert!(!h.state().log.entries.iter().any(|(_, m)| m.contains("GitHub")));
}

/// Options › Updates › Check for updates automatically, off: no request
/// at start; and none where a package manager updates Moonglow, whose
/// page says so.
#[test]
fn the_updates_option_turns_automatic_checks_off() {
    use std::sync::atomic::Ordering;
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings.update_checked = Some(mg_ui::updates::now());
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Updates").click();
    h.run();
    h.get_by_label("Check for updates automatically").click();
    h.run();
    h.get_by_label("OK").click();
    h.run();
    assert!(h.state().settings.no_update_check);
    assert_eq!(asked.load(Ordering::SeqCst), 0);

    // At the next start, a day later: not asked.
    let mut settings = h.state().settings.clone();
    settings.update_checked = None;
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings = settings.clone();
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 0, "switched off");
    assert!(h.state().updates.notice.is_none());

    // On, but installed by Flatpak: not asked, and the page says why.
    settings.no_update_check = false;
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.settings = settings;
    app.updates.installer = Some(mg_ui::updates::Installer::Flatpak);
    let mut h = harness(app, 1280.0, 800.0);
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 0, "a package manager updates it");
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Updates").click();
    h.run();
    h.get_by_label_contains("This copy is updated by Flatpak");
    assert!(h.get_by_label("Check for updates automatically").accesskit_node().is_disabled());
    // Help › Check for Updates… in the Flatpak, whose sandbox has no
    // network: nothing asked, only how Flatpak updates it.
    h.get_by_label("Cancel").click();
    h.run();
    mg_ui::updates::check_now(h.state_mut());
    h.run();
    assert_eq!(asked.load(Ordering::SeqCst), 0, "the Flatpak asks nothing");
    h.get_by_label_contains("This copy is updated by Flatpak: flatpak update");
    assert!(h.query_by_label("Check Again").is_none());
    assert!(h.query_by_label_contains("Couldn't ask GitHub").is_none());
}

/// As the desktop app runs it (jobs in the background): the request goes
/// on its own thread, the window asks while it waits, and the answer is
/// shown when it comes; turned on in Options, the automatic check asks
/// at once.
#[test]
fn the_update_check_asks_on_its_own_thread() {
    use std::sync::atomic::Ordering;
    let current = mg_core::release::version();
    let (mut app, asked) = app_with_releases(Ok(release_answer("v99.0.0")));
    app.background_jobs = true;
    app.settings.no_update_check = true;
    let mut h = harness(app, 1280.0, 800.0);
    h.run_steps(2);
    mg_ui::updates::check_now(h.state_mut());
    h.run_steps(1);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while h.state().updates.report.is_none() {
        assert!(std::time::Instant::now() < deadline, "no answer");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.run_steps(1);
    }
    h.run_steps(2);
    h.get_by_label(&format!("Moonglow 99.0.0 is out (you have {current})."));
    assert!(h.query_by_label("Asking GitHub for the latest release…").is_none());
    assert!(!h.state().updates.checking());
    assert!(h.state().updates.notice.is_none(), "the window's check, not the automatic one");
    assert_eq!(asked.load(Ordering::SeqCst), 1);

    // Switched on in Options: asked at the next frames, not in an hour.
    h.state_mut().updates.window = false;
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run_steps(2);
    h.get_by_label("Updates").click();
    h.run_steps(2);
    h.get_by_label("Check for updates automatically").click();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(1);
    assert!(!h.state().settings.no_update_check);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while h.state().updates.notice.is_none() {
        assert!(std::time::Instant::now() < deadline, "no notice");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.run_steps(1);
    }
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}
