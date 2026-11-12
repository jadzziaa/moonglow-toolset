//! Quick Open (M3's T8): the Top Bar's field "Go to resource or command"
//! and its popover: what it finds and opens, its keys, the keys it gives
//! back, its key's binding, and how it yields as the window narrows.

use crate::common::*;

/// A module of its own: the areas `start` ("Village Square"), `inn` ("The
/// Inn") and `town`, the scripts `hello` and `s000` to `s059`.
fn quick_module(dir: &std::path::Path) -> std::path::PathBuf {
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("QUICK"));
    for name in ["start", "inn", "town"] {
        let mut a = ifo::MOD_AREA_LIST.new_item();
        a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str(name).unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
    }
    m.set_info(&info).unwrap();
    for (name, shown) in [("start", "Village Square"), ("inn", "The Inn"), ("town", "")] {
        let mut are = Gff::new(*b"ARE ");
        if !shown.is_empty() {
            let text = LocString::from_text(Language::ENGLISH, Gender::Male, shown);
            are.root.set("Name", mg_gff::Value::LocString(text));
        }
        m.set(ResKey::parse(name, ResType::ARE).unwrap(), are.to_bytes().unwrap());
    }
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    for i in 0..60 {
        let key = ResKey::parse(&format!("s{i:03}"), ResType::NSS).unwrap();
        m.set(key, b"void main() { }\n".to_vec());
    }
    let path = dir.join("quick.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

/// [`quick_module`] open in a window `width` by 800.
fn quick_harness(name: &str, width: f32) -> (Harness<'static, Moonglow>, std::path::PathBuf) {
    let dir = mg_testkit::scratch_dir(&format!("ui-quick-open-{name}"));
    let path = quick_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, width, 800.0);
    settle(&mut h);
    (h, path)
}

fn field_has_keys(h: &Harness<'_, Moonglow>) -> bool {
    h.ctx.memory(|m| m.has_focus(mg_ui::quick_open::field_id()))
}

/// The popover's rectangle (its group's).
fn popover(h: &Harness<'_, Moonglow>) -> egui::Rect {
    h.state().quick_open.popover_rect().expect("the popover")
}

/// Ctrl+P, a resource's name, Enter: its tab opens; an area by its name,
/// a script by its ResRef. Nothing open at rest.
#[test]
fn a_resource_is_opened_by_its_name() {
    let (mut h, _) = quick_harness("resource", 1400.0);
    assert!(!h.state().quick_open.is_open());
    assert!(h.query_by_label(mg_ui::quick_open::RESULTS).is_none(), "no popover at rest");
    quick_open(&mut h, "hello");
    assert!(field_has_keys(&h));
    assert_eq!(h.state().quick_open.shown().0.first().map(String::as_str), Some("hello.nss"));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(!h.state().quick_open.is_open());
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Script(hello)).is_some(), "its editor opened");
    // An area by its name: its tab.
    quick_open(&mut h, "inn");
    assert_eq!(quick_open_rows(&h).first().map(String::as_str), Some("The Inn"));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let inn = ResRef::from_str("inn").unwrap();
    assert!(h.state().dock.find_tab(&Tab::Area(inn)).is_some(), "the area's tab");
    // By its ResRef too, its name shown, the ResRef and type beside it.
    // (And a command of that word after it.)
    quick_open(&mut h, "start");
    assert_eq!(quick_open_rows(&h), ["Village Square", "Go to Start Location"]);
    // Choosing a resource already open in a window of its own, behind
    // another tab of that window (the scripts' window holds both), brings
    // its tab to the front of the window, once, as the Module panel's
    // double-click does (the same action).
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    let other = ResKey::parse("s000", ResType::NSS).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenResource(other));
    settle(&mut h);
    let active = |h: &Harness<'_, Moonglow>| {
        let at = h.state().dock.find_tab(&Tab::Script(hello)).unwrap();
        let leaf = h.state().dock.leaf(egui_dock::NodePath { surface: at.surface, node: at.node });
        let leaf = leaf.unwrap();
        (leaf.tabs.contains(&Tab::Script(other)), leaf.tabs[leaf.active.0].clone())
    };
    assert_eq!(active(&h), (true, Tab::Script(other)), "the other in front of one window");
    quick_open(&mut h, "hello");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let tabs = h.state().dock.iter_all_tabs().filter(|(_, t)| **t == Tab::Script(hello)).count();
    assert_eq!(tabs, 1, "not opened twice");
    assert_eq!(active(&h), (true, Tab::Script(hello)), "brought to the front");
}

/// With nothing typed: this module's areas opened lately, most recent
/// first, and nothing else.
#[test]
fn nothing_typed_shows_the_areas_opened_lately() {
    let (mut h, path) = quick_harness("recent", 1400.0);
    for area in ["town", "inn", "start"] {
        h.state_mut().settings.remember_area(&path, area);
    }
    // (One no longer in the module is left out.)
    h.state_mut().settings.remember_area(&path, "gone");
    quick_open(&mut h, "");
    assert_eq!(
        h.state().quick_open.shown(),
        (vec!["Village Square".to_string(), "The Inn".to_string(), "town.are".to_string()], 0)
    );
    // The first, Enter: its tab.
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Area(ResRef::from_str("start").unwrap())).is_some());
}

/// At most 50 rows; a last line says how many more, and typing narrows
/// them.
#[test]
fn fifty_rows_and_how_many_more() {
    let (mut h, _) = quick_harness("more", 1400.0);
    quick_open(&mut h, "s0");
    let (rows, more) = h.state().quick_open.shown();
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0], "s000.nss");
    assert_eq!(more, 10, "s000 to s059: 60");
    h.get_by_label("10 more: type more to narrow them");
    quick_open_field(&h).type_text("5");
    settle(&mut h);
    let (rows, more) = h.state().quick_open.shown();
    assert_eq!((rows.len(), more), (10, 0));
    assert!(h.query_by_label_contains("more: type more").is_none());
}

/// Down and Up move the choice, wrapping at the ends; Page Down and Page
/// Up move it by a page and stop at the ends; Tab leaves it in the field.
#[test]
fn the_arrows_and_the_page_keys_choose() {
    let (mut h, _) = quick_harness("keys", 1400.0);
    command_palette(&mut h, "");
    let n = h.state().quick_open.shown().0.len();
    assert_eq!(n, 50);
    let chosen = |h: &Harness<'_, Moonglow>| h.state().quick_open.selected;
    assert_eq!(chosen(&h), 0);
    let press = |h: &mut Harness<'_, Moonglow>, key| {
        h.key_press(key);
        settle(h);
    };
    press(&mut h, egui::Key::ArrowUp);
    assert_eq!(chosen(&h), n - 1, "Up at the top wraps to the last");
    assert_eq!(
        quick_open_chosen(&h).as_deref(),
        Some(h.state().quick_open.shown().0[n - 1].as_str())
    );
    press(&mut h, egui::Key::ArrowDown);
    assert_eq!(chosen(&h), 0, "Down at the last wraps to the top");
    press(&mut h, egui::Key::ArrowDown);
    assert_eq!(chosen(&h), 1);
    press(&mut h, egui::Key::PageDown);
    let page = chosen(&h) - 1;
    assert!(page > 5 && page < n, "a page: {page}");
    for _ in 0..20 {
        press(&mut h, egui::Key::PageDown);
    }
    assert_eq!(chosen(&h), n - 1, "Page Down stops at the last");
    press(&mut h, egui::Key::PageUp);
    assert_eq!(chosen(&h), n - 1 - page);
    for _ in 0..20 {
        press(&mut h, egui::Key::PageUp);
    }
    assert_eq!(chosen(&h), 0, "Page Up stops at the top");
    // Tab keeps the keys in the field.
    press(&mut h, egui::Key::Tab);
    assert!(field_has_keys(&h) && h.state().quick_open.is_open());
}

/// Ctrl+Shift+P finds Build Module and runs it; Help › Command Palette…
/// opens Quick Open with `>`.
#[test]
fn the_command_palette_s_key_and_menu_row_look_among_the_commands() {
    let (mut h, _) = quick_harness("palette", 1400.0);
    command_palette(&mut h, "build mod");
    assert_eq!(quick_open_rows(&h).first().map(String::as_str), Some("Build Module…"));
    // (No resource among the commands: "s0" finds no command.)
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().build.is_some(), "Build Module's window");
    h.state_mut().build = None;
    settle(&mut h);
    menu_command(&mut h, "Help", "Command Palette…");
    assert!(h.state().quick_open.is_open() && field_has_keys(&h));
    assert_eq!(h.state().quick_open.text, ">");
    quick_open_field(&h).type_text("s0");
    settle(&mut h);
    assert!(h.state().quick_open.shown().0.iter().all(|r| !r.ends_with(".nss")));
}

/// Escape closes it and gives the keys back to the field that had them:
/// typing goes on there.
#[test]
fn escape_gives_the_keys_back_to_a_field() {
    let (mut h, _) = quick_harness("escape-field", 1400.0);
    type_into_tree_filter(&mut h, "in");
    let filter = h.ctx.memory(|m| m.focused()).expect("the Filter has the keys");
    quick_open(&mut h, "zz");
    assert!(field_has_keys(&h));
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().quick_open.is_open() && h.state().quick_open.text.is_empty());
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(filter), "back to the Filter");
    h.event(egui::Event::Text("n".into()));
    settle(&mut h);
    let typed = h.query_by(tree_filter).unwrap().accesskit_node().value();
    assert_eq!(typed.as_deref(), Some("inn"), "typing goes on in the Filter");
}

/// Escape with a Properties window open closes Quick Open alone; a
/// click outside closes it without running anything; a click on a row
/// runs it.
#[test]
fn escape_and_a_click_outside_close_it_alone() {
    let (mut h, _) = quick_harness("outside", 1400.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    quick_open(&mut h, "inn");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().quick_open.is_open());
    assert!(h.state().dock.find_tab(&Tab::ModuleProperties).is_some(), "Properties stays");
    // A dialog's too (Find Instance, which Escape closes).
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    menu_command(&mut h, "Edit", "Find Instance…");
    assert!(h.state().find_instance.is_some());
    quick_open(&mut h, "inn");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().quick_open.is_open());
    assert!(h.state().find_instance.is_some(), "Find Instance stays");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().find_instance.is_none(), "its own Escape next");
    // A click outside (on the Status Bar): closed, nothing opened.
    quick_open(&mut h, "inn");
    let tabs = h.state().dock.iter_all_tabs().count();
    // (The Status Bar's middle, with nothing in it.)
    let window = h.ctx.content_rect();
    click_pos(&h, egui::pos2(window.center().x, window.bottom() - 12.0));
    settle(&mut h);
    assert!(!h.state().quick_open.is_open());
    assert_eq!(h.state().dock.iter_all_tabs().count(), tabs, "nothing opened");
    // A click on a row: it opens.
    quick_open(&mut h, "inn");
    quick_open_row(&h, "The Inn").click();
    settle(&mut h);
    assert!(!h.state().quick_open.is_open());
    assert!(h.state().dock.find_tab(&Tab::Area(ResRef::from_str("inn").unwrap())).is_some());
}

/// A command that cannot run now is listed grayed, its tip says why, and
/// Enter and a click do nothing.
#[test]
fn a_grayed_command_says_why_and_does_nothing() {
    let (mut h, _) = quick_harness("grayed", 1400.0);
    command_palette(&mut h, "redo");
    let row = quick_open_row(&h, "Redo");
    assert!(row.accesskit_node().is_disabled());
    let rect = row.rect();
    assert_eq!(tip_at(&mut h, rect).as_deref(), Some("Nothing to redo"));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().quick_open.is_open(), "Enter did nothing");
    click_pos(&h, rect.center());
    settle(&mut h);
    assert!(h.state().quick_open.is_open(), "a click did nothing");
}

/// While Quick Open has the keys, a key that types is the field's (a
/// plain key given to a command runs nothing); a key with Ctrl runs its
/// command and closes it.
#[test]
fn the_window_s_single_keys_wait_while_typing() {
    let (mut h, _) = quick_harness("single-keys", 1400.0);
    let keys = vec!["A".to_string(), "Ctrl+B".to_string()];
    h.state_mut().settings.key_bindings.insert("about".into(), keys);
    settle(&mut h);
    quick_open(&mut h, "a");
    assert_eq!(h.state().quick_open.text, "a");
    assert!(!h.state().about, "A typed, About not run");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::B);
    settle(&mut h);
    assert!(h.state().about && !h.state().quick_open.is_open());
    h.state_mut().about = false;
    // Ctrl+S saves the module, as from any text field.
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table",
        vec![mg_edit::Edit::SetResource { key, data: Some(b"2DA V2.0\n\n   A\n".to_vec()) }],
    )));
    settle(&mut h);
    assert!(h.state().ws.as_ref().unwrap().is_modified());
    quick_open(&mut h, "s");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    settle(&mut h);
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "saved");
    assert!(!h.state().quick_open.is_open());
}

/// No module open: the recent modules first, then the commands that work
/// without one; a recent module chosen opens it.
#[test]
fn with_no_module_open_it_offers_recent_modules_and_commands() {
    let dir = mg_testkit::scratch_dir("ui-quick-open-no-module");
    let path = quick_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.remember(&path);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    quick_open(&mut h, "");
    let (rows, _) = h.state().quick_open.shown();
    assert_eq!(rows[0], "quick.mod");
    assert!(rows.contains(&"Open Module…".to_string()));
    assert!(!rows.contains(&"Build Module…".to_string()), "it needs a module");
    // (Its row as wide as its folder's path wants, wider than the field:
    // in a narrow window the popover is moved in, inside the window.)
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    h.set_size(egui::vec2(760.0, 800.0));
    settle(&mut h);
    quick_open(&mut h, "");
    let (window, pop) = (h.ctx.content_rect(), popover(&h));
    let field = quick_open_field(&h).rect();
    assert!(pop.width() > field.width() + 100.0, "wider than the field: {pop:?}");
    assert!(window.contains_rect(pop), "{pop:?} inside {window:?}");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().module_path(), Some(path));
}

/// A key the user gave another command on Ctrl+P is theirs: Quick Open
/// is without one (its field shows no key cap); a key given Quick Open
/// is its cap, and opens it.
#[test]
fn the_user_s_own_ctrl_p_wins_and_the_cap_shows_the_key() {
    let (mut h, _) = quick_harness("ctrl-p", 1400.0);
    let description = |h: &Harness<'_, Moonglow>| {
        h.get_by_label(mg_ui::quick_open::NAME).accesskit_node().description()
    };
    assert_eq!(description(&h).as_deref(), Some("Ctrl+P"));
    h.state_mut().settings.key_bindings.insert("about".into(), vec!["Ctrl+P".into()]);
    settle(&mut h);
    assert_eq!(description(&h), None, "no key cap");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::P);
    settle(&mut h);
    assert!(h.state().about && !h.state().quick_open.is_open(), "Ctrl+P is About's");
    h.state_mut().about = false;
    h.state_mut().settings.key_bindings.insert("quick-open".into(), vec!["Ctrl+K".into()]);
    settle(&mut h);
    assert_eq!(description(&h).as_deref(), Some("Ctrl+K"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    settle(&mut h);
    assert!(h.state().quick_open.is_open());
}

/// A job has the window: Ctrl+P does nothing, typing goes nowhere, and
/// the field is grayed.
#[test]
fn a_job_keeps_it_closed() {
    let (mut h, _) = quick_harness("job", 1400.0);
    quick_open(&mut h, "in");
    let (tell, told) = std::sync::mpsc::channel::<()>();
    h.state_mut().start_job_for_tests("Compile All Scripts", move |_| {
        let _ = told.recv();
    });
    // (Frames while the job runs: `settle` waits for it to end.)
    h.run_steps(3);
    assert!(h.state().busy());
    assert!(!h.state().quick_open.is_open(), "closed by the job");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::P);
    h.event(egui::Event::Text("abc".into()));
    h.run_steps(2);
    assert!(!h.state().quick_open.is_open());
    assert!(h.state().quick_open.text.is_empty());
    assert!(h.get_by_label(mg_ui::quick_open::NAME).accesskit_node().is_disabled());
    tell.send(()).unwrap();
    let deadline = std::time::Instant::now() + SETTLE_TIME;
    while h.state().busy() && std::time::Instant::now() < deadline {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    settle(&mut h);
    quick_open(&mut h, "in");
}

/// The index is made the first time Quick Open opens, and again only
/// when the workspace's revision or the load order's generation changes.
#[test]
fn the_index_is_made_again_only_when_the_module_changes() {
    let (mut h, _) = quick_harness("index", 1400.0);
    assert_eq!(h.state().quick_open.builds().0, 0, "not before it opens");
    quick_open(&mut h, "inn");
    assert_eq!(h.state().quick_open.builds().0, 1);
    quick_open_field(&h).type_text("x");
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    quick_open(&mut h, "inn");
    assert_eq!(h.state().quick_open.builds().0, 1, "typed and opened again: the same");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // A command (the workspace's revision): made again.
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.apply(mg_edit::Command::new("Nothing", Vec::new())).unwrap();
    quick_open(&mut h, "inn");
    assert_eq!(h.state().quick_open.builds().0, 2);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // A resource put in past the commands (a file read again from
    // outside): made again, and found.
    let key = ResKey::parse("cellar", ResType::ARE).unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.module.set(key, Gff::new(*b"ARE ").to_bytes().unwrap());
    quick_open(&mut h, "cellar");
    assert_eq!(h.state().quick_open.builds().0, 3);
    assert_eq!(quick_open_rows(&h), ["cellar.are"]);
    // (A new load order: `formosa_opens_its_script...`, with the game.)
}

/// With the game: "formosa" finds Chapter 1's resources by name and
/// ResRef, the creature named Formosa first; `m1q3_formosa_9` and Enter
/// open the script's editor (its source before its compiled form). A
/// game resource (the module doesn't have it) opens read-only.
#[test]
fn formosa_opens_its_script_and_a_game_resource_opens_read_only() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    quick_open(&mut h, "formosa");
    let (rows, _) = h.state().quick_open.shown();
    assert_eq!(rows.first().map(String::as_str), Some("Formosa"), "{rows:?}");
    assert!(rows.iter().any(|r| r == "m1q3_formosa_9.nss"), "{rows:?}");
    quick_open_field(&h).type_text("_9");
    settle(&mut h);
    assert_eq!(quick_open_rows(&h).first().map(String::as_str), Some("m1q3_formosa_9.nss"));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let script = ResKey::parse("m1q3_formosa_9", ResType::NSS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Script(script)).is_some(), "its editor");
    // A resource of the game's alone: read-only, as the Resources panel
    // opens it.
    quick_open(&mut h, "nw_c2_default1");
    let (rows, _) = h.state().quick_open.shown();
    assert!(rows.iter().any(|r| r == "nw_c2_default1.nss"), "{rows:?}");
    let at = rows.iter().position(|r| r == "nw_c2_default1.nss").unwrap();
    for _ in 0..at {
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
    }
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let key = ResKey::parse("nw_c2_default1", ResType::NSS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Resource(key)).is_some(), "read-only");
    assert_eq!(mg_ui::quick_open::game_tab(h.state(), key), Tab::Resource(key));
    // (The index's load-order part made once: `quick_open`'s unit test
    // has it made again for a new load order.)
    assert_eq!(h.state().quick_open.builds().1, 1);
}

/// Escape gives the keys back to the area view: W then moves its camera.
#[test]
fn escape_gives_the_keys_back_to_the_area_view() {
    let Some((mut h, area)) = area_harness("quick-open-escape") else { return };
    h.run_steps(3);
    let target = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().target;
    let centre = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(centre);
    h.run_steps(1);
    let hold = |h: &mut Harness<'_, Moonglow>| {
        let event = |pressed| egui::Event::Key {
            key: egui::Key::W,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        h.event(event(true));
        h.event(egui::Event::Text("w".into()));
        h.run_steps(4);
        h.event(event(false));
        h.run_steps(1);
    };
    quick_open(&mut h, "");
    let start = target(&h);
    hold(&mut h);
    assert_eq!(target(&h), start, "W typed into Quick Open");
    assert_eq!(h.state().quick_open.text, "w");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(!h.state().quick_open.is_open());
    let start = target(&h);
    hold(&mut h);
    assert!((target(&h) - start).length() > 0.1, "W moves the camera again");
}

/// The window's frame round the application in a test (8 points a side;
/// `shell.rs`'s `TEST_FRAME`): a harness this much wider than the window.
const TEST_FRAME: f32 = 16.0;

/// The field's stages as a window narrows from 1600 to 420 by 8: the
/// field (440 wide, 300 under 1300, narrower with the room), its icon,
/// nothing; each stage the one the room between the bar's groups calls
/// for, the field as wide as that room allows; the build actions go only
/// after the field became its icon, the icon before the quick actions;
/// none of the bar's controls overlaps or leaves the bar, with a long
/// module name; and opened at each width (in the bar or over it), the
/// field and its popover inside the window.
#[test]
fn the_field_yields_in_order_and_the_popover_stays_inside() {
    let dir = mg_testkit::scratch_dir("ui-quick-open-sweep");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let path = dir.join(format!("{}.mod", "a_module_with_a_long_name".repeat(3)));
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1600.0 + TEST_FRAME, 700.0);
    settle(&mut h);
    let in_bar = |h: &Harness<'_, Moonglow>, name: &str| {
        h.get_by_label("Top Bar").query_by_label(name).map(|n| n.rect())
    };
    // 3: the field; 2: its icon; 1: neither (by the key alone).
    let mut was = 3;
    let mut seen = std::collections::BTreeSet::new();
    let (mut widest, mut capped) = (false, false);
    for step in (0..=147).rev() {
        let width = 420.0 + 8.0 * step as f32;
        h.set_size(egui::vec2(width + TEST_FRAME, 700.0));
        settle(&mut h);
        let window = h.ctx.content_rect();
        // (The bar's row: 30 high round the menus' middle.)
        let middle = in_bar(&h, "File").unwrap().center().y;
        let half = mg_uikit::metrics::TOP_BAR / 2.0;
        let bar = egui::Rect::from_x_y_ranges(window.x_range(), middle - half..=middle + half);
        let field = in_bar(&h, mg_ui::quick_open::NAME);
        let role = field.map(|_| {
            let n = h.get_by_label("Top Bar").get_by_label(mg_ui::quick_open::NAME);
            n.accesskit_node().role()
        });
        let stage = match role {
            Some(egui::accesskit::Role::Button) => 2,
            Some(_) => 3,
            None => 1,
        };
        assert!(stage <= was, "{width}: stage {stage} after {was}");
        was = stage;
        seen.insert(stage);
        // The room the groups leave it, and what that calls for.
        let quick = in_bar(&h, "Redo");
        let build = in_bar(&h, "Compile");
        let left = quick.or_else(|| in_bar(&h, "Help")).unwrap().right();
        // (The bar's inside: the window's width less its padding, from
        // where the menus begin.)
        let inside =
            in_bar(&h, "File").unwrap().left() + width - 2.0 * mg_uikit::shell::TOP_BAR_PADDING;
        let right = build.map_or(inside, |b| b.left());
        // (Less the groups' gaps, 2 a side, and the 12 kept a side.)
        let room = right - left - 4.0 - 2.0 * mg_uikit::shell::TOP_BAR_MID_PADDING;
        let cap = if window.width() < 1300.0 { 300.0 } else { 440.0 };
        let wanted = match () {
            _ if quick.is_none() => 1,
            _ if room >= mg_uikit::shell::QUICK_OPEN_ICON_UNDER + 2.0 => 3,
            _ if (mg_uikit::metrics::ICON_BUTTON + 2.0
                ..mg_uikit::shell::QUICK_OPEN_ICON_UNDER - 2.0)
                .contains(&room) =>
            {
                2
            }
            _ if room < mg_uikit::metrics::ICON_BUTTON - 2.0 => 1,
            // (Within a rounding of a line: either.)
            _ => stage,
        };
        assert_eq!(stage, wanted, "{width}: room {room}");
        if stage == 3 {
            let w = field.unwrap().width();
            let expected = room.min(cap).floor();
            assert!((w - expected).abs() <= 2.0, "{width}: the field {w} wide, {expected} wanted");
            widest |= w == 440.0;
            capped |= window.width() < 1300.0 && w == 300.0;
        }
        assert!(build.is_none() || stage >= 2, "{width}: the build actions with no field or icon");
        if let Some(f) = field {
            assert!(bar.contains_rect(f), "{width}: {f:?} leaves the bar {bar:?}");
        }
        let wrong = crate::shell::bars_wrong(&h, true);
        assert!(wrong.is_empty(), "{width}: {wrong:#?}");
        // Opened at every width, in the bar or over it: the bar's controls
        // apart still (the field in the bar), the field and its popover
        // inside the window.
        quick_open(&mut h, "a");
        let window = h.ctx.content_rect();
        let fields = h.get_all_by_label(mg_ui::quick_open::NAME).map(|n| n.rect());
        for r in fields.chain([popover(&h)]) {
            assert!(window.contains_rect(r), "{width}: {r:?} leaves {window:?}");
        }
        assert!(field_has_keys(&h), "{width}");
        if stage == 3 {
            // (What is typed is a node of its own, inside the field.)
            let typed = "and a [";
            let wrong: Vec<String> = (crate::shell::bars_wrong(&h, true).into_iter())
                .filter(|w| !(w.starts_with("Top Bar: Quick Open [") && w.contains(typed)))
                .collect();
            assert!(wrong.is_empty(), "{width}, open: {wrong:#?}");
        }
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert!(!h.state().quick_open.is_open());
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), [1, 2, 3], "every stage seen");
    assert!(widest && capped, "440 wide in a wide window, 300 just under 1300");
}

/// The popover inside the window at 420, 760, 1280 and 3440 wide, and in a
/// window 320 tall; the icon in a narrow window opens the same popover.
#[test]
fn the_popover_stays_inside_the_window() {
    let (mut h, _) = quick_harness("inside", 1400.0);
    let sizes = [
        (420.0, 700.0),
        (760.0, 700.0),
        (1280.0, 800.0),
        (3440.0, 1440.0),
        (900.0, 320.0),
        // (The least window.)
        (420.0 + TEST_FRAME, 320.0 + TEST_FRAME),
    ];
    for (w, ht) in sizes {
        h.set_size(egui::vec2(w, ht));
        settle(&mut h);
        quick_open(&mut h, "s");
        let window = h.ctx.content_rect();
        let pop = popover(&h);
        assert!(window.contains_rect(pop), "{w}x{ht}: {pop:?} leaves {window:?}");
        assert!(pop.height() >= 24.0, "{w}x{ht}: a row at least");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
    }
    // The icon at 760: pressed, the field over the bar and its popover.
    h.set_size(egui::vec2(760.0, 700.0));
    settle(&mut h);
    let icon = h.get_by_label("Top Bar").get_by_label(mg_ui::quick_open::NAME);
    assert_eq!(icon.accesskit_node().role(), egui::accesskit::Role::Button);
    icon.click();
    settle(&mut h);
    assert!(h.state().quick_open.is_open() && field_has_keys(&h));
    h.event(egui::Event::Text("inn".into()));
    settle(&mut h);
    assert_eq!(quick_open_rows(&h).first().map(String::as_str), Some("The Inn"));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Area(ResRef::from_str("inn").unwrap())).is_some());
}

/// What Quick Open's index and a keystroke cost, on the largest load
/// order here: the game's, with a builder's haks where `MOONGLOW_HAKS`
/// or `~/Projects/tdod-module/build/hak` has them. Run by hand, in
/// release: `cargo test -p mg-ui --release --test app quick_open_costs --
/// --ignored --nocapture`.
#[test]
#[ignore]
fn quick_open_costs() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let mut game = mg_rules::GameData::open(&install).unwrap();
    let mut dirs = mg_testkit::builder_hak_dirs();
    if dirs.is_empty() {
        let tdod = std::env::var_os("HOME")
            .map(|h| std::path::PathBuf::from(h).join("Projects/tdod-module/build/hak"));
        dirs.extend(tdod.filter(|d| d.is_dir()));
    }
    let mut haks = 0;
    for dir in &dirs {
        for hak in mg_testkit::files_with_ext(dir, "hak") {
            let label = format!("hak:{}", hak.file_stem().unwrap().to_string_lossy());
            let erf = mg_resman::ErfContainer::open(&hak).unwrap();
            game.resman.add(mg_resman::priority::HAK_USER, label, mg_resman::LayerClass::Erf, erf);
            haks += 1;
        }
    }
    let queries = ["a", "ar", "are", "area", "area_", "nw_c2", "nw_c2_default1", "zzzz"];
    let (count, built, typed) = mg_ui::quick_open::time_keystrokes(&game.resman, &queries, 7);
    println!("load order: the game and {haks} haks from {dirs:?}: {count} resources");
    println!("index made in {:.2} ms (best of 7)", built.as_secs_f64() * 1000.0);
    for (q, found, took) in typed {
        println!("{q:>16}: {found:>7} found, {:.3} ms (best of 7)", took.as_secs_f64() * 1000.0);
    }
    // A whole keystroke (the popover's rows made, commands and all, the
    // module's and the load order's found anew): Chapter 1 on the game and
    // the haks above.
    let mut app = Moonglow::new(None, Box::new(NoDialogs::default()));
    app.game = Some(std::sync::Arc::new(game));
    let module = Module::open(&root.join("data/nwm/Chapter1.nwm")).unwrap();
    app.ws = Some(mg_edit::Workspace::new(module));
    let typed = ["a", "ar", "formosa", "build module", "nw_c2_default1", "zzzz"];
    for (q, found, took) in mg_ui::quick_open::time_whole_keystrokes(&mut app, &typed, 7) {
        let ms = took.as_secs_f64() * 1000.0;
        println!("whole keystroke {q:>16}: {found:>7} found, {ms:.3} ms (best of 7)");
    }
    // A large module's part: 50,000 named resources.
    let typed = ["t", "tr", "tree", "tree 1", "iron sword", "zzzz"];
    for (q, found, took) in mg_ui::quick_open::time_module_keystrokes(50_000, &typed, 7) {
        let ms = took.as_secs_f64() * 1000.0;
        println!("module of 50,000 {q:>12}: {found:>6} found, {ms:.3} ms (best of 7)");
    }
    // The module's part, Chapter 1's.
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&root.join("data/nwm/Chapter1.nwm"));
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    let began = std::time::Instant::now();
    quick_open(&mut h, "");
    let opened = began.elapsed();
    let (module, _) = h.state().quick_open.build_times();
    println!(
        "Chapter1: module part {:.2} ms (names apart); first open with names {:.1} ms",
        module.as_secs_f64() * 1000.0,
        opened.as_secs_f64() * 1000.0
    );
}

/// Tab passes through the field (it takes the keys, opens nothing) to the
/// build actions after it; a click or typing opens it.
#[test]
fn tab_passes_through_the_field() {
    let (mut h, _) = quick_harness("tab", 1400.0);
    let id = mg_ui::quick_open::field_id();
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.run_steps(2);
    assert!(field_has_keys(&h) && !h.state().quick_open.is_open(), "the keys, not open");
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert!(!field_has_keys(&h), "Tab went on");
    let compile = h.get_by_label("Top Bar").get_by_label("Compile");
    assert!(compile.accesskit_node().is_focused(), "to Compile, next on the bar");
    // Shift+Tab back to it, and typing opens it.
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    settle(&mut h);
    assert!(field_has_keys(&h) && !h.state().quick_open.is_open());
    h.event(egui::Event::Text("inn".into()));
    settle(&mut h);
    assert!(h.state().quick_open.is_open());
    assert_eq!(quick_open_rows(&h).first().map(String::as_str), Some("The Inn"));
    // Open, Tab stays.
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert!(field_has_keys(&h) && h.state().quick_open.is_open());
    // A click on the field opens it too.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    quick_open_field(&h).click();
    settle(&mut h);
    assert!(h.state().quick_open.is_open() && field_has_keys(&h));
}

/// How soon the window asks to be drawn again after its last frame.
fn again(h: &Harness<'_, Moonglow>) -> std::time::Duration {
    let out = &h.output().viewport_output;
    out.get(&egui::ViewportId::ROOT).map_or(std::time::Duration::MAX, |v| v.repaint_delay)
}

/// Where the window does its work in the background (the application):
/// at rest Quick Open asks for no frames; open, the areas opened lately
/// are named at once, frames are asked for while its names and the load
/// order's resources are taken a little at a time ("Indexing…" said
/// meanwhile), and the load order's rows come.
#[test]
fn in_the_background_it_works_while_it_indexes() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.background_jobs = true;
    let chapter = root.join("data/nwm/Chapter1.nwm");
    app.settings.remember_area(&chapter, "map_m1q1a");
    app.open_module(&chapter);
    let mut h = harness(app, 1400.0, 900.0);
    settle(&mut h);
    h.step();
    assert!(again(&h) >= std::time::Duration::from_secs(1), "at rest: {:?}", again(&h));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::P);
    h.step();
    h.step();
    assert!(h.state().quick_open.is_open());
    assert_eq!(again(&h), std::time::Duration::ZERO, "a frame asked for while it indexes");
    let (rows, _) = h.state().quick_open.shown();
    assert!(!rows[0].ends_with(".are"), "the recent area named at once: {rows:?}");
    quick_open_field(&h).type_text("nw_c2_default1");
    // (One frame: the load order's part is still being made.)
    h.step();
    assert!(h.query_by_label("Indexing…").is_some(), "said while it indexes");
    let deadline = std::time::Instant::now() + SETTLE_TIME;
    while h.query_by_label("Indexing…").is_some() {
        assert!(std::time::Instant::now() < deadline, "the index never came");
        assert_eq!(again(&h), std::time::Duration::ZERO, "a frame asked for meanwhile");
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let (rows, _) = h.state().quick_open.shown();
    assert!(rows.iter().any(|r| r == "nw_c2_default1.nss"), "{rows:?}");
}

/// The popover is over the area's view: a press on one of its rows (here
/// a grayed one, which runs nothing) is the popover's, not the view's:
/// the object selected stays so.
#[test]
fn a_press_on_the_popover_is_not_the_area_view_s() {
    let Some((mut h, area)) = area_harness("quick-open-over-area") else { return };
    settle(&mut h);
    let selection = vec![(mg_area::ObjectKind::Waypoint, 0)];
    h.state_mut().area_views.get_mut(&area).unwrap().selection = selection.clone();
    settle(&mut h);
    command_palette(&mut h, "");
    let view = h.state().area_views[&area].rect;
    let grayed = h.state().quick_open.shown().0.iter().position(|r| r == "Redo").unwrap();
    let rows = quick_open_rows(&h);
    let row = quick_open_row(&h, &rows[grayed.min(rows.len() - 1)]).rect();
    assert!(view.contains(row.center()), "the row {row:?} over the view {view:?}");
    click_pos(&h, row.center());
    settle(&mut h);
    assert!(h.state().quick_open.is_open(), "a grayed row: open still");
    assert_eq!(h.state().area_views[&area].selection, selection, "the view took no press");
    // (The same press with Quick Open closed is the view's: it lets go of
    // the object.)
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    click_pos(&h, row.center());
    settle(&mut h);
    assert_ne!(h.state().area_views[&area].selection, selection, "the view's press");
}
