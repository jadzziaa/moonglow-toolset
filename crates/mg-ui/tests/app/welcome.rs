//! The Welcome page and the empty editor area, "No area open" (M4's
//! U24, the Welcome board): the recent modules, the Start, Without a
//! module, Learn and Help rows and the commands they run, the Status
//! lines, the columns folding where the page is narrow, and the empty
//! state's two buttons.

use crate::common::*;
use std::sync::{Arc, Mutex};

/// The recent modules named `names` in a folder that is not there (the
/// rows are told by their names, never by the disk).
fn recent(names: &[&str]) -> Vec<std::path::PathBuf> {
    let place = std::path::Path::new("/srv/nwn/modules");
    names.iter().map(|n| place.join(n)).collect()
}

/// The accessible names of the nodes whose names begin with `name`.
fn named_from(h: &Harness<'_, Moonglow>, name: &str) -> Vec<String> {
    (h.query_all(
        egui_kittest::kittest::by().predicate(|n| n.label().is_some_and(|l| l.starts_with(name))),
    ))
    .map(|n| n.accesskit_node().label().unwrap_or_default())
    .collect()
}

/// Finding 13.3: a recent `.nwm` is shown by its name, with no "/" after
/// it (it is a module file, as a `.mod` is, in any case of its
/// extension), and a module folder or a nasher project with one.
#[test]
fn a_recent_module_s_name_has_a_slash_only_where_it_is_a_folder() {
    let mut app = app_with(Vec::new());
    app.settings.recent =
        recent(&["Chapter1.nwm", "Prelude.NWM", "castle.mod", "Keep.MOD", "castle", "tdod-module"]);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    for (name, folder) in [
        ("Chapter1.nwm", false),
        ("Prelude.NWM", false),
        ("castle.mod", false),
        ("Keep.MOD", false),
        ("castle", true),
        ("tdod-module", true),
    ] {
        let rows: Vec<String> = (named_from(&h, name).into_iter())
            .filter(|l| !l[name.len()..].starts_with('.'))
            .collect();
        assert!(!rows.is_empty(), "no row for {name}");
        for row in &rows {
            let slash = row[name.len()..].starts_with('/');
            assert_eq!(slash, folder, "{name}: shown as {row:?}");
        }
    }
}

/// The file dialogs asked for, each as `open Hak`, `save Tileset`,
/// `folder <title>`; none answered.
#[derive(Clone, Default)]
struct Asked(Arc<Mutex<Vec<String>>>);

impl mg_ui::Dialogs for Asked {
    fn open_file(
        &mut self,
        kind: mg_ui::FileKind,
        _: Option<&std::path::Path>,
    ) -> Option<std::path::PathBuf> {
        self.0.lock().unwrap().push(format!("open {kind:?}"));
        None
    }
    fn save_file(
        &mut self,
        kind: mg_ui::FileKind,
        _: Option<&std::path::Path>,
    ) -> Option<std::path::PathBuf> {
        self.0.lock().unwrap().push(format!("save {kind:?}"));
        None
    }
    fn pick_folder(
        &mut self,
        title: &str,
        _: Option<&std::path::Path>,
    ) -> Option<std::path::PathBuf> {
        self.0.lock().unwrap().push(format!("folder {title}"));
        None
    }
}

/// A Welcome page with no module, its dialogs `asked`.
fn welcome(asked: &Asked, width: f32, height: f32) -> Harness<'static, Moonglow> {
    let mut app = Moonglow::new(None, Box::new(asked.clone()));
    app.settings.hide_log = true;
    let mut h = harness(app, width, height);
    settle(&mut h);
    h
}

/// Whether `tab` is open (in the dock or a window of its own).
fn has(h: &Harness<'_, Moonglow>, tab: &Tab) -> bool {
    h.state().dock.find_tab(tab).is_some()
}

/// Each Start, Without a module and Help row (and Learn's User Manual)
/// runs its command: what the command does is done, in a window of its
/// own each time.
#[test]
fn each_start_without_and_help_row_runs_its_command() {
    type Done = fn(&Harness<'_, Moonglow>, &[String]) -> bool;
    let rows: [(&str, Done); 15] = [
        ("New Module…", |h, _| h.state().wizard.is_some()),
        ("Open Module…", |_, asked| asked == ["open Module"]),
        ("Open Folder…", |_, asked| asked == ["folder Open Module Folder or nasher Project"]),
        ("Open Hak…", |_, asked| asked == ["open Hak"]),
        ("New Hak", |h, _| h.state().haks.len() == 1),
        ("Open Tileset…", |_, asked| asked.len() == 1 && asked[0].starts_with("open ")),
        ("New Tileset…", |_, asked| asked == ["save Tileset"]),
        ("2DA File…", |_, asked| asked == ["open Table"]),
        ("Talk Table", |h, _| has(h, &Tab::TalkTable)),
        ("Resource Browser", |h, _| h.state().left_dock_shows() == Some(mg_ui::Panel::Resources)),
        ("Appearance Gallery", |h, _| h.state().placeable_gallery.is_some()),
        ("User Manual", |h, _| has(h, &Tab::Manual)),
        ("Command Palette…", |h, _| h.state().quick_open.is_open()),
        ("Check for Updates…", |h, _| h.state().updates.window),
        ("About Moonglow Toolset", |h, _| h.state().about),
    ];
    for (row, done) in rows {
        let asked = Asked::default();
        let mut h = welcome(&asked, 1600.0, 900.0);
        let said = asked.0.lock().unwrap().clone();
        assert!(!done(&h, &said), "{row}: done before the click");
        h.get_by_label(row).click();
        h.run_steps(3);
        let said = asked.0.lock().unwrap().clone();
        assert!(done(&h, &said), "{row}: not done ({said:?})");
    }
}

/// The Start rows show their commands' keys as bound (told to a screen
/// reader as the row's shortcut), and none where a command has none.
#[test]
fn the_start_rows_show_their_keys_as_bound() {
    let asked = Asked::default();
    let mut h = welcome(&asked, 1600.0, 900.0);
    let keys = |h: &Harness<'_, Moonglow>, row: &str| {
        h.get_by_label(row).accesskit_node().data().keyboard_shortcut().map(str::to_owned)
    };
    assert_eq!(keys(&h, "New Module…").as_deref(), Some("Ctrl+N"));
    assert_eq!(keys(&h, "Open Module…").as_deref(), Some("Ctrl+O"));
    assert_eq!(keys(&h, "Open Folder…"), None, "no key by default");
    let ctrl_shift_n = egui::KeyboardShortcut::new(
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        egui::Key::N,
    );
    h.state_mut().keymap.set(mg_ui::keys::Cmd::NewModule, vec![ctrl_shift_n]);
    h.state_mut().keymap.set(mg_ui::keys::Cmd::OpenModule, Vec::new());
    h.run_steps(2);
    assert_eq!(keys(&h, "New Module…").as_deref(), Some("Ctrl+Shift+N"));
    assert_eq!(keys(&h, "Open Module…"), None);
}

/// Learn lists the manual's chapters by their titles, in the board's
/// order, and each opens the manual on its own chapter, whatever chapter
/// it showed before.
#[test]
fn learn_s_links_open_their_chapters() {
    let chapters = [
        ("Getting started", "01-getting-started.md"),
        ("Coming from Aurora", "14-coming-from-aurora.md"),
        ("The main window", "02-main-window.md"),
        ("Modules", "03-modules.md"),
        ("Areas", "04-areas.md"),
        ("Blueprints", "05-blueprints.md"),
        ("Conversations", "06-conversations.md"),
        ("Scripts", "07-scripts.md"),
        ("Journal and factions", "08-journal-and-factions.md"),
        ("NUI Creator", "18-nui-creator.md"),
        ("Build, verify and test", "09-build-and-test.md"),
        ("Differences from Aurora", "12-differences.md"),
        ("Options", "10-options.md"),
        ("Command-line tools", "11-command-line.md"),
        ("Troubleshooting", "13-troubleshooting.md"),
        ("Plugins", "15-plugins.md"),
    ];
    let asked = Asked::default();
    let mut h = welcome(&asked, 1600.0, 1200.0);
    // (In the board's order, down the column.)
    let link = |h: &Harness<'_, Moonglow>, title: &str| {
        let by = egui_kittest::kittest::by().label(title).role(egui::accesskit::Role::Link);
        h.get(by).rect()
    };
    let tops: Vec<f32> = chapters.iter().map(|(t, _)| link(&h, t).top()).collect();
    assert!(tops.windows(2).all(|w| w[0] < w[1]), "in order: {tops:?}");
    for (title, file) in chapters.iter().rev() {
        egui_kittest::kittest::Queryable::get(
            &h,
            egui_kittest::kittest::by().label(title).role(egui::accesskit::Role::Link),
        )
        .click();
        h.run_steps(3);
        assert!(has(&h, &Tab::Manual), "{title}: the manual");
        assert_eq!(h.state().manual_chapter(), *file, "{title}");
        h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Manual));
        settle(&mut h);
    }
}

/// A module with areas `areas` (none if empty), its start in the first.
fn module_with_areas(dir: &std::path::Path, areas: &[&str]) -> std::path::PathBuf {
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    for area in areas {
        m.set(ResKey::parse(area, ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
        let mut a = ifo::MOD_AREA_LIST.new_item();
        a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str(area).unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
    }
    if let Some(first) = areas.first() {
        info.root.write(&ifo::MOD_ENTRY_AREA, ResRef::from_str(first).unwrap());
    }
    m.set_info(&info).unwrap();
    let path = dir.join("module.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

/// The module at `path` open with nothing in the middle: "No area open".
fn no_area(path: &std::path::Path, opened_lately: &[&str]) -> Harness<'static, Moonglow> {
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.settings.no_reopen_tabs = true;
    app.settings.hide_log = true;
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.settings.recent_areas =
        vec![(path.to_path_buf(), opened_lately.iter().map(|a| a.to_string()).collect())];
    app.open_module(path);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::ModuleProperties));
    settle(&mut h);
    assert!(has(&h, &Tab::NoArea), "nothing in the middle");
    h
}

/// "No area open" is the kit's empty state: what it is, how to open one
/// (with Quick Open's key as bound), Area Wizard… (its wizard) and Go to
/// Start (the start area's tab, at the start location); Go to Start is
/// off in a module with no areas, which has nothing to start in.
#[test]
fn the_empty_state_s_buttons_run_area_wizard_and_go_to_start() {
    let dir = mg_testkit::scratch_dir("ui-welcome-no-area");
    let path = module_with_areas(&dir, &["harbor", "market"]);
    let mut h = no_area(&path, &[]);
    h.get_by_label("No area open");
    h.get_by_label("Double-click an area in the Module panel, or find one with Ctrl+P.");
    let start = h.get_by_label("Go to Start");
    assert!(!start.accesskit_node().is_disabled());
    start.click();
    settle(&mut h);
    let harbor = Tab::Area(ResRef::from_str("harbor").unwrap());
    assert!(has(&h, &harbor), "the start area opened");
    assert!(!has(&h, &Tab::NoArea), "never over an open editor");
    // (Closed again: the empty state again; the wizard needs the game's
    // tilesets, so here it says so and opens nothing, as the menu's.)
    h.state_mut().actions.push(mg_ui::Action::CloseTab(harbor));
    settle(&mut h);
    assert!(has(&h, &Tab::NoArea));
    let wizard = h.get_by_label("Area Wizard…");
    assert!(!wizard.accesskit_node().is_disabled());
    // Quick Open's key unbound: said by its name.
    h.state_mut().keymap.set(mg_ui::keys::Cmd::QuickOpen, Vec::new());
    h.run_steps(2);
    h.get_by_label("Double-click an area in the Module panel, or find one with Quick Open.");

    let dir = mg_testkit::scratch_dir("ui-welcome-no-areas");
    let path = module_with_areas(&dir, &[]);
    let h = no_area(&path, &[]);
    assert!(h.get_by_label("Go to Start").accesskit_node().is_disabled(), "no area to start in");
    assert!(!h.get_by_label("Area Wizard…").accesskit_node().is_disabled());
}

/// With the game: Area Wizard… opens the Area Wizard.
#[test]
fn the_empty_state_s_area_wizard_opens_the_wizard() {
    let Some(mut h) = game_harness("welcome-no-area") else { return };
    h.state_mut().settings.no_last_area = true;
    for tab in h.state().dock.iter_all_tabs().map(|(_, t)| t.clone()).collect::<Vec<_>>() {
        if tab != Tab::NoArea {
            h.state_mut().actions.push(mg_ui::Action::CloseTab(tab));
        }
    }
    settle(&mut h);
    assert!(has(&h, &Tab::NoArea));
    assert!(h.state().wizard.is_none());
    h.get_by_label("Area Wizard…").click();
    settle(&mut h);
    assert!(
        matches!(h.state().wizard, Some(mg_ui::Wizard::NewArea(_))),
        "{:?}",
        h.state().wizard.is_some()
    );
}

/// The areas today's page listed (those opened lately, and the module's)
/// are where the board has them: Quick Open with nothing typed lists the
/// one opened lately first, and the Module panel lists every area.
#[test]
fn no_area_s_areas_are_in_the_module_panel_and_quick_open() {
    let dir = mg_testkit::scratch_dir("ui-welcome-areas");
    let path = module_with_areas(&dir, &["harbor", "market", "temple"]);
    let mut h = no_area(&path, &["market"]);
    unfold_left_dock(&mut h);
    settle(&mut h);
    for area in ["harbor", "market", "temple"] {
        h.get_by_label(area);
    }
    quick_open(&mut h, "");
    let (rows, _) = h.state().quick_open.shown();
    assert_eq!(rows.first().map(String::as_str), Some("market.are"), "{rows:?}");
}

/// The page comes to rest in every state that does not animate: no
/// recent modules, recents, no game, an update told, ready and failed;
/// checking and downloading animate (a spinner) while they last.
#[test]
fn the_welcome_page_comes_to_rest() {
    use mg_ui::updates::{GetState, Getting};
    let release = || mg_ui::updates::Release {
        tag: "v99.0.0".into(),
        version: mg_ui::updates::Version::parse("99.0.0").unwrap(),
        page: "https://example.invalid/release".into(),
        notes: String::new(),
        assets: Vec::new(),
    };
    for case in ["none", "recent", "available", "ready", "failed", "checking", "downloading"] {
        let mut app = app_with(Vec::new());
        app.settings.hide_log = true;
        if case != "none" {
            app.settings.recent = recent(&["castle.mod", "Chapter1.nwm", "tdod-module"]);
        }
        match case {
            "available" => app.updates.notice = Some(release()),
            "ready" => {
                let staged = mg_ui::updates::Staged {
                    version: release().version,
                    kind: mg_update::Kind::Other,
                    ready: mg_update::Ready::Installed,
                };
                app.updates.getting =
                    Some(Getting::shown(release(), true, GetState::Ready(staged), 0, None));
            }
            "failed" => {
                let why = GetState::Failed("the download stopped".into());
                app.updates.getting = Some(Getting::shown(release(), true, why, 0, None));
            }
            "checking" => app.updates.check_never_answered(),
            "downloading" => {
                let state = GetState::Downloading;
                app.updates.getting = Some(Getting::shown(release(), true, state, 1, Some(2)));
            }
            _ => {}
        }
        let mut h = harness(app, 1280.0, 800.0);
        h.run_steps(3);
        let rests = h.try_run().is_ok();
        let animates = matches!(case, "checking" | "downloading");
        assert_eq!(rests, !animates, "{case}");
        if case == "checking" {
            h.get_by_label("Checking for updates…");
        }
    }
}

/// Only the rows in sight are laid out: two hundred recent modules in a
/// short window make a few dozen rows, and the page scrolls to the last.
#[test]
fn only_the_recent_rows_in_sight_are_laid_out() {
    let names: Vec<String> = (0..200).map(|i| format!("module {i:03}.mod")).collect();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.recent = recent(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let mut h = harness(app, 1280.0, 700.0);
    settle(&mut h);
    let rows = named_from(&h, "module ").len();
    assert!((5..40).contains(&rows), "{rows} rows laid out");
    let at = h.get_by_label("module 000.mod in /srv/nwn/modules").rect().center();
    wheel_to(&mut h, at, "module 199.mod in /srv/nwn/modules");
    let rows = named_from(&h, "module ").len();
    assert!((5..40).contains(&rows), "{rows} rows laid out at the end");
}

/// A recent module's row says when it was opened where the settings know
/// it (in its name for a screen reader), and says nothing where they do
/// not (modules from settings written before the times).
#[test]
fn a_recent_row_tells_when_it_was_opened() {
    let now = mg_core::time::unix_now();
    for width in [1600.0, 700.0] {
        let mut app = app_with(Vec::new());
        app.settings.hide_log = true;
        app.settings.recent = recent(&["castle.mod", "keep.mod", "tower.mod"]);
        let times = [(now - 7300, "2 hours ago"), (now - 400, "6 minutes ago")];
        for (name, (time, _)) in ["castle.mod", "keep.mod"].iter().zip(times) {
            let path = std::path::Path::new("/srv/nwn/modules").join(name);
            app.settings.recent_opened.insert(path, time);
        }
        let mut h = harness(app, width, 900.0);
        settle(&mut h);
        h.get_by_label("castle.mod in /srv/nwn/modules, opened 2 hours ago");
        h.get_by_label("keep.mod in /srv/nwn/modules, opened 6 minutes ago");
        // (No time known: the name as it was, with nothing about when.)
        h.get_by_label("tower.mod in /srv/nwn/modules");
        assert!(h.query_by_label_contains("opened less than").is_none());
    }
}

/// A recent module's row is named for a screen reader with the folder it
/// is in, takes the keys (Enter opens it, as a click does), and its box
/// is where it is drawn.
#[test]
fn a_recent_row_is_named_and_opens_by_its_keys() {
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.recent = recent(&["castle.mod", "keep.mod", "tower.mod"]);
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let row = h.get_by_label("castle.mod in /srv/nwn/modules");
    let rect = row.rect();
    let heading = h.get_by_label("Recent Modules").rect();
    assert!(rect.top() > heading.bottom() && rect.left() < heading.left(), "{rect:?}");
    assert!((rect.height() - 43.0).abs() < 1.0, "the board's row: {rect:?}");
    // (Each row opens its own module: the second by Enter, the third by
    // Space.)
    let opened = |h: &mut Harness<'_, Moonglow>, name: &str, key: egui::Key| {
        h.get_by_label(&format!("{name} in /srv/nwn/modules")).focus();
        h.run_steps(2);
        let from = h.state().log.entries.len();
        h.key_press(key);
        settle(h);
        let said: Vec<String> =
            h.state().log.entries[from..].iter().map(|(_, m)| m.clone()).collect();
        assert!(said.iter().any(|m| m.contains(name)), "tried to open {name}: {said:?}");
        for other in ["castle.mod", "keep.mod", "tower.mod"].iter().filter(|o| **o != name) {
            assert!(!said.iter().any(|m| m.contains(other)), "{other} for {name}: {said:?}");
        }
    };
    opened(&mut h, "keep.mod", egui::Key::Enter);
    opened(&mut h, "tower.mod", egui::Key::Space);
}

/// The Welcome page's hints are its keys while it has them, none while a
/// menu takes the keys; never Save's (no module to save).
#[test]
fn the_welcome_page_s_hints_go_while_a_menu_is_open() {
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    let mut h = harness(app, 1400.0, 700.0);
    settle(&mut h);
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said().to_vec();
    assert!(said(&h).iter().any(|s| s == "Ctrl, N: New module"), "{:?}", said(&h));
    assert!(!said(&h).iter().any(|s| s.contains("Save")), "{:?}", said(&h));
    h.get_by_label("File").click();
    h.run_steps(3);
    assert!(said(&h).is_empty(), "a menu has the keys: {:?}", said(&h));
}

/// A file dropped onto the window, as the window's system hands it over.
#[derive(Debug)]
struct Dropped(std::path::PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

/// A module folder or a nasher project dropped on the window with no
/// module open opens, as the page's note says; a folder that is neither
/// is not opened.
#[test]
fn a_module_folder_dropped_with_no_module_opens() {
    let dir = mg_testkit::scratch_dir("ui-welcome-drop");
    let path = sample_module(&dir);
    let folder = dir.join("sample");
    Module::open(&path).unwrap().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let other = dir.join("notes");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("readme.txt"), "not a module").unwrap();
    let drop = |h: &mut Harness<'_, Moonglow>, p: &std::path::Path| {
        h.input_mut().dropped_files.push(std::sync::Arc::new(Dropped(p.to_path_buf())));
        settle(h);
    };
    let mut app = app_with(Vec::new());
    app.settings.module_folders = mg_ui::settings::ModuleFolders::Folder;
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    drop(&mut h, &other);
    assert!(h.state().ws.is_none(), "not a module");
    drop(&mut h, &folder);
    let location = h.state().ws.as_ref().map(|ws| ws.module.location.clone());
    assert_eq!(location, Some(Some(ModuleLocation::Folder(folder))));
    // (A nasher project dropped: it opens as one.)
    let project = dir.join("project");
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("nasher.cfg"),
        "[package]\n  [package.sources]\n  include = \"src/**/*\"\n  \
         [package.rules]\n  \"*\" = \"src\"\n[target]\nname = \"m\"\nfile = \"m.mod\"\n",
    )
    .unwrap();
    let at = ModuleLocation::Project { root: project.clone(), target: "m".into() };
    Module::open(&path).unwrap().save_as(&at).unwrap();
    let mut h = harness(app_with(Vec::new()), 1400.0, 800.0);
    settle(&mut h);
    drop(&mut h, &project);
    let location = h.state().ws.as_ref().and_then(|ws| ws.module.location.clone());
    assert!(matches!(location, Some(ModuleLocation::Project { .. })), "{location:?}");
    // (A folder whose module.ifo is MODULE.IFO opens, as the module reader
    // reads it; one whose nasher.cfg is NASHER.CFG does not, as the reader
    // takes no such project.)
    let upper = dir.join("upper");
    let _ = std::fs::remove_dir_all(&upper);
    Module::open(&path).unwrap().save_as(&ModuleLocation::Folder(upper.clone())).unwrap();
    std::fs::rename(upper.join("module.ifo"), upper.join("MODULE.IFO")).unwrap();
    let shouting = dir.join("shouting");
    let _ = std::fs::remove_dir_all(&shouting);
    std::fs::create_dir_all(&shouting).unwrap();
    std::fs::write(shouting.join("NASHER.CFG"), "").unwrap();
    let mut h = harness(app_with(Vec::new()), 1400.0, 800.0);
    settle(&mut h);
    drop(&mut h, &shouting);
    assert!(h.state().ws.is_none(), "NASHER.CFG is no nasher project");
    drop(&mut h, &upper);
    let location = h.state().ws.as_ref().and_then(|ws| ws.module.location.clone());
    assert_eq!(location, Some(ModuleLocation::Folder(upper)));
}

/// A module folder dropped while a module is open (with unsaved changes)
/// is imported into it, as before U24: the module stays, and nothing asks
/// to save it.
#[test]
fn a_module_folder_dropped_on_an_open_module_is_imported() {
    let dir = mg_testkit::scratch_dir("ui-welcome-drop-open");
    let path = sample_module(&dir);
    let folder = dir.join("other");
    let _ = std::fs::remove_dir_all(&folder);
    Module::open(&path).unwrap().save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let asked = Asked::default();
    let mut app = Moonglow::new(None, Box::new(asked.clone()));
    app.settings.hide_log = true;
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.apply(mg_edit::Command::new(
        "unsaved",
        vec![mg_edit::Edit::SetResource {
            key: ResKey::parse("mg_note", ResType::NSS).unwrap(),
            data: Some(b"void main() {}".to_vec()),
        }],
    ))
    .unwrap();
    assert!(h.state().ws.as_ref().unwrap().is_modified());
    h.input_mut().dropped_files.push(std::sync::Arc::new(Dropped(folder.clone())));
    settle(&mut h);
    let location = h.state().ws.as_ref().and_then(|ws| ws.module.location.clone());
    assert_eq!(location, Some(ModuleLocation::Archive(path)), "the module stays");
    assert!(h.state().ws.as_ref().unwrap().is_modified(), "its changes kept");
    assert!(asked.0.lock().unwrap().is_empty(), "nothing asked: {:?}", asked.0.lock().unwrap());
    assert!(h.query_by_label_contains("Import Resources").is_some(), "the import window");
}

/// While the Welcome page is in sight, the update is told in its Status,
/// not in the strip under the Top Bar too; over another tab the strip
/// tells it. While the Check for Updates window is open, it alone tells.
#[test]
fn the_update_is_told_once() {
    let release = mg_ui::updates::Release {
        tag: "v99.0.0".into(),
        version: mg_ui::updates::Version::parse("99.0.0").unwrap(),
        page: "https://example.invalid/release".into(),
        notes: String::new(),
        assets: Vec::new(),
    };
    let current = mg_core::release::version();
    let told = format!("Moonglow 99.0.0 is out (you have {current}).");
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.updates.notice = Some(release);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    assert_eq!(h.get_all_by_label(&told).count(), 1);
    let status = h.get_by_label("Status").rect();
    assert!(h.get_by_label(&told).rect().top() > status.bottom(), "in the page's Status");
    assert!(h.query_by_label("Close Update Notice").is_none(), "no strip on the Welcome page");
    // (Another tab in front: the strip.)
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Manual));
    settle(&mut h);
    assert_eq!(h.get_all_by_label(&told).count(), 1);
    h.get_by_label("Close Update Notice");
    // (The window open on it: the window alone.)
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Manual));
    settle(&mut h);
    h.get_by_label("What's New…").click();
    settle(&mut h);
    assert!(h.state().updates.window);
    assert_eq!(h.get_all_by_label(&told).count(), 1, "the window's");
}

/// Where the page scrolls its Status out of sight (the log open in a
/// short window), the strip under the Top Bar tells the update, once; the
/// page comes to rest, and its line tells it once it is scrolled to.
#[test]
fn an_update_out_of_the_page_s_sight_is_told_in_the_strip() {
    let release = mg_ui::updates::Release {
        tag: "v99.0.0".into(),
        version: mg_ui::updates::Version::parse("99.0.0").unwrap(),
        page: "https://example.invalid/release".into(),
        notes: String::new(),
        assets: Vec::new(),
    };
    let current = mg_core::release::version();
    let told = format!("Moonglow 99.0.0 is out (you have {current}).");
    let mut app = app_with(Vec::new());
    app.settings.hide_log = false;
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.updates.notice = Some(release);
    let mut h = harness(app, 1280.0, 760.0);
    settle(&mut h);
    h.get_by_label("Close Update Notice");
    assert_eq!(h.get_all_by_label(&told).count(), 1, "the strip's alone");
    h.run_steps(3);
    assert!(h.try_run().is_ok(), "comes to rest");
    // (Scrolled to the Status, the wheel turned over the page's heading:
    // the page's line, and no strip.)
    let heading = h.get_by_label("Moonglow Toolset").rect();
    h.hover_at(heading.center());
    let started = std::time::Instant::now();
    while h.query_by_label("Close Update Notice").is_some() {
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "never scrolled to");
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -40.0),
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        h.run_steps(2);
    }
    settle(&mut h);
    assert!(h.query_by_label("Close Update Notice").is_none(), "no strip");
    assert_eq!(h.get_all_by_label(&told).count(), 1, "the page's alone");
    h.run_steps(3);
    assert!(h.try_run().is_ok(), "comes to rest");
}

/// The release the update tests tell of, and what the page says of it.
fn release_99() -> (mg_ui::updates::Release, String) {
    let release = mg_ui::updates::Release {
        tag: "v99.0.0".into(),
        version: mg_ui::updates::Version::parse("99.0.0").unwrap(),
        page: "https://example.invalid/release".into(),
        notes: String::new(),
        assets: Vec::new(),
    };
    let told = format!("Moonglow 99.0.0 is out (you have {}).", mg_core::release::version());
    (release, told)
}

/// The label of the node with the keys, if any.
fn focused(h: &Harness<'_, Moonglow>) -> Option<String> {
    let by = egui_kittest::kittest::by().predicate(|n| n.is_focused());
    h.query_all(by).next().map(|n| n.accesskit_node().label().unwrap_or_default())
}

/// Tab goes through every recent module's row in order, those below the
/// fold too, each brought into sight as it takes the keys (two hundred at
/// 1280 by 700); the rows laid out stay those in sight.
#[test]
fn tab_reaches_the_recent_rows_past_the_fold() {
    let names: Vec<String> = (0..200).map(|i| format!("module {i:03}.mod")).collect();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.recent = recent(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let mut h = harness(app, 1280.0, 700.0);
    settle(&mut h);
    // Shift+Tab from Start's first button lands on the last row, below the
    // fold, and brings it into sight.
    h.get_by_label("New Module…").focus();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    h.run_steps(2);
    let last = "module 199.mod in /srv/nwn/modules";
    assert_eq!(focused(&h).as_deref(), Some(last), "Shift+Tab to the last row");
    at_rest(&mut h, last, &|h| h.get_by_label(last).rect());
    let area = h.state().editor_area_rect().expect("the editor area");
    assert!(area.contains_rect(h.get_by_label(last).rect()), "the last row in sight");
    drop(h);
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.recent = recent(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let mut h = harness(app, 1280.0, 700.0);
    settle(&mut h);
    h.get_by_label("module 000.mod in /srv/nwn/modules").focus();
    h.run_steps(2);
    let area = h.state().editor_area_rect().expect("the editor area");
    for (i, name) in names.iter().enumerate() {
        let label = format!("{name} in /srv/nwn/modules");
        assert_eq!(focused(&h).as_deref(), Some(label.as_str()), "row {i}");
        at_rest(&mut h, &label, &|h| h.get_by_label(&label).rect());
        let rect = h.get_by_label(&label).rect();
        assert!(area.contains_rect(rect), "row {i} in sight: {rect:?} in {area:?}");
        let rows = named_from(&h, "module ").len();
        assert!(rows < 40, "{rows} rows laid out at row {i}");
        h.key_press(egui::Key::Tab);
        h.run_steps(2);
    }
    assert_eq!(focused(&h).as_deref(), Some("New Module…"), "then Start");
}

/// With floating editors, a window over the page's update line hides it:
/// the strip under the Top Bar tells it then, once.
#[test]
fn a_window_over_the_update_line_has_the_strip_tell_it() {
    let (release, told) = release_99();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.open_editors = mg_ui::OpenEditors::Floating;
    app.updates.notice = Some(release);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    assert!(h.query_by_label("Close Update Notice").is_none(), "the page tells it");
    let line = h.get_by_label(&told).rect();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Manual));
    settle(&mut h);
    let window = h.get_by_label("Moonglow Toolset: User Manual").rect();
    assert!(window.left() < line.right(), "the manual stands over the line: {window:?} {line:?}");
    h.get_by_label("Close Update Notice");
    assert_eq!(h.get_all_by_label(&told).count(), 1, "the strip alone");
}

/// The pages walk the module, scan the manual, check the install and read
/// a dragged folder once, not each frame: the Welcome page with an update
/// told, No area open, and a folder dragged over the window.
#[test]
fn the_pages_walk_scan_check_and_read_once() {
    let (release, _) = release_99();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.recent = recent(&["castle.mod"]);
    app.updates.notice = Some(release);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    let before = h.state().welcome_counts();
    h.run_steps(20);
    assert_eq!(h.state().welcome_counts(), before, "the Welcome page, twenty frames");
    // (A folder dragged over the window: read once for the drag.)
    let dir = mg_testkit::scratch_dir("ui-welcome-counts");
    let path = module_with_areas(&dir, &["harbor"]);
    // (kittest keeps the hovered files from step to step: pushed once.)
    h.input_mut()
        .hovered_files
        .push(egui::HoveredFile { path: Some(dir.clone()), ..Default::default() });
    for _ in 0..10 {
        h.step();
        assert_eq!(h.ctx.input(|i| i.raw.hovered_files.len()), 1, "one folder dragged");
    }
    let read = h.state().welcome_counts()[3] - before[3];
    assert_eq!(read, 1, "the dragged folder read once in ten frames");
    // No area open: the module's areas walked once, and again after a
    // change to the module.
    drop(h);
    let mut h = no_area(&path, &[]);
    let before = h.state().welcome_counts();
    h.run_steps(20);
    assert_eq!(h.state().welcome_counts(), before, "No area open, twenty frames");
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.apply(mg_edit::Command::new(
        "a script",
        vec![mg_edit::Edit::SetResource {
            key: ResKey::parse("mg_note", ResType::NSS).unwrap(),
            data: Some(b"void main() {}".to_vec()),
        }],
    ))
    .unwrap();
    h.run_steps(5);
    let walked = h.state().welcome_counts()[0] - before[0];
    assert_eq!(walked, 1, "walked again once after the change");
}

/// Check for Updates… from the page, with an update told: the page stays
/// where it is while the window asks, and after it is closed.
#[test]
fn check_for_updates_from_the_page_moves_nothing() {
    let (release, told) = release_99();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.updates.notice = Some(release);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    let heading = h.get_by_label("Moonglow Toolset").rect();
    let line = h.get_by_label(&told).rect();
    let link = egui::accesskit::Role::Link;
    let check = h.get_all_by_label("Check for Updates…");
    check.into_iter().find(|n| n.accesskit_node().role() == link).unwrap().click();
    settle(&mut h);
    assert!(h.state().updates.window, "the window is open");
    assert_eq!(h.get_by_label("Moonglow Toolset").rect(), heading, "the heading stays");
    assert_eq!(h.get_by_label(&told).rect(), line, "the line stays");
    assert!(h.query_by_label("Close Update Notice").is_none(), "no strip");
    h.state_mut().updates.window = false;
    settle(&mut h);
    assert_eq!(h.get_by_label("Moonglow Toolset").rect(), heading, "the heading stays");
    assert!(h.query_by_label("Close Update Notice").is_none(), "no strip");
}

/// About standing over the page's update line (900 by 900) is a dialog,
/// not a window the line hides under: no strip comes, the page stays where
/// it is, and closing About moves nothing either.
#[test]
fn a_dialog_over_the_update_line_moves_nothing() {
    let (release, told) = release_99();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.updates.notice = Some(release);
    let mut h = harness(app, 900.0, 900.0);
    settle(&mut h);
    assert!(h.query_by_label("Close Update Notice").is_none(), "the page tells it");
    let line = h.get_by_label(&told).rect();
    let heading = h.get_by_label("Moonglow Toolset").rect();
    h.state_mut().about = true;
    for step in 0..10 {
        h.step();
        let mut about = h.get_all_by_label("About Moonglow Toolset");
        let window = about.find(|n| n.accesskit_node().role() == egui::accesskit::Role::Window);
        if step >= 2 {
            let window = window.expect("About").rect();
            assert!(window.intersects(line), "About over the line: {window:?} {line:?}");
        }
        assert!(h.query_by_label("Close Update Notice").is_none(), "no strip at step {step}");
        let page = h.get_all_by_label("Moonglow Toolset").any(|n| n.rect() == heading);
        assert!(page, "the page stays at step {step}");
    }
    h.state_mut().about = false;
    settle(&mut h);
    assert_eq!(h.get_by_label("Moonglow Toolset").rect(), heading, "the page stays");
    assert_eq!(h.get_by_label(&told).rect(), line, "the line stays");
}

/// Under About, and under Check for Updates, nothing on the page acts by
/// the keys: Enter on a focused recent row, Learn link, button and update
/// offer does nothing. A click of the pointer on the page beside the
/// dialog is still the page's.
#[test]
fn a_recent_row_opens_nothing_under_a_dialog() {
    for dialog in ["About", "Check for Updates"] {
        let (release, _) = release_99();
        let mut app = app_with(Vec::new());
        app.settings.hide_log = true;
        app.settings.no_update_check = true;
        app.settings.recent = recent(&["castle.mod"]);
        app.updates.notice = Some(release);
        let mut h = harness(app, 1400.0, 800.0);
        settle(&mut h);
        match dialog {
            "About" => h.state_mut().about = true,
            _ => h.state_mut().updates.window = true,
        }
        h.run_steps(4);
        let link = egui::accesskit::Role::Link;
        let enter_on = |h: &mut Harness<'_, Moonglow>, label: &str, role: Option<_>| {
            let node = h
                .get_all_by_label(label)
                .find(|n| role.is_none_or(|r| n.accesskit_node().role() == r))
                .unwrap_or_else(|| panic!("{label}"));
            node.focus();
            h.run_steps(2);
            h.key_press(egui::Key::Enter);
            h.run_steps(4);
        };
        let from = h.state().log.entries.len();
        enter_on(&mut h, "castle.mod in /srv/nwn/modules", None);
        let said: Vec<String> =
            h.state().log.entries[from..].iter().map(|(_, m)| m.clone()).collect();
        assert!(!said.iter().any(|m| m.contains("castle.mod")), "{dialog}: opened {said:?}");
        enter_on(&mut h, "Getting started", Some(link));
        assert!(h.state().dock.find_tab(&Tab::Manual).is_none(), "{dialog}: the manual opened");
        enter_on(&mut h, "Options…", None);
        assert!(h.state().options.is_none(), "{dialog}: Options opened");
        let report = format!("{:?}", h.state().updates.report);
        enter_on(&mut h, "What's New…", None);
        assert_eq!(format!("{:?}", h.state().updates.report), report, "{dialog}: What's New");
        // (The pointer: Getting started is beside the dialog, not under it.)
        let r = h.get_all_by_label("Getting started").find(|n| n.accesskit_node().role() == link);
        click_pos(&h, r.unwrap().rect().center());
        h.run_steps(4);
        assert!(h.state().dock.find_tab(&Tab::Manual).is_some(), "{dialog}: a click beside it");
    }
}

/// While the page scrolls its Status into sight a few points at a time,
/// the update is told once at every step: the page's line only where it is
/// wholly in sight, the strip where it is in part.
#[test]
fn an_update_line_half_in_sight_is_the_strip_s() {
    let (release, told) = release_99();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = false;
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.updates.notice = Some(release);
    // (Recent modules enough for the page to scroll the line out at its top.)
    let names: Vec<String> = (0..60).map(|i| format!("module {i:02}.mod")).collect();
    app.settings.recent = recent(&names.iter().map(String::as_str).collect::<Vec<_>>());
    let mut h = harness(app, 1280.0, 760.0);
    settle(&mut h);
    let heading = h.get_by_label("Moonglow Toolset").rect();
    h.hover_at(heading.center());
    let mut steps = 0;
    while h.query_by_label("Close Update Notice").is_some() {
        steps += 1;
        assert!(steps < 400, "never scrolled to");
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -3.0),
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        h.run_steps(2);
        assert_eq!(h.get_all_by_label(&told).count(), 1, "told once at step {steps}");
        let strip = h.query_by_label("Close Update Notice").is_some();
        let area = h.state().editor_area_rect().expect("the editor area");
        let line = h.get_all_by_label(&told).next().unwrap().rect();
        // (The page's scroll ends at the tab's bottom margin, 6 points.)
        let clip = egui::Rect::from_min_max(area.min, egui::pos2(area.max.x, area.bottom() - 6.0));
        if !strip {
            assert!(clip.contains_rect(line), "the page's line wholly in sight: {line:?} {clip:?}");
        }
    }
    assert!(steps > 3, "scrolled in small steps: {steps}");
    // On down until the line goes out at the page's top, under the tabs:
    // there the strip's coming does not move the line against the page's
    // edge, so a line told while half under the tabs would stay so.
    let welcome = h
        .get_all_by_label("Welcome")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab)
        .expect("the Welcome tab")
        .rect();
    // (The page's scroll starts at the tab's top margin, 2 points.)
    let top = welcome.bottom() + 2.0;
    let mut gone = 0;
    while h.query_by_label("Close Update Notice").is_none() {
        gone += 1;
        assert!(gone < 400, "never scrolled past");
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -3.0),
            source: egui::MouseWheelSource::Unknown,
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        h.run_steps(2);
        assert_eq!(h.get_all_by_label(&told).count(), 1, "told once at step {gone} past");
        if h.query_by_label("Close Update Notice").is_none() {
            let line = h.get_all_by_label(&told).next().unwrap().rect();
            assert!(line.top() >= top, "the page's line wholly below the tabs: {line:?}, {top}");
        }
    }
    assert!(gone > 3, "scrolled past in small steps: {gone}");
}

/// The first frame the page is drawn shows no strip where the page tells
/// the update, and the page stands still into the second.
#[test]
fn the_first_frame_has_no_strip_and_the_page_stays() {
    let (release, told) = release_99();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.updates.notice = Some(release);
    let before = app.welcome_counts();
    let mut h = harness(app, 1400.0, 800.0);
    h.step();
    assert!(h.query_by_label("Close Update Notice").is_none(), "no strip at the first frame");
    let heading = h.get_by_label("Moonglow Toolset").rect();
    h.step();
    assert!(h.query_by_label("Close Update Notice").is_none(), "no strip at the second");
    assert_eq!(h.get_by_label("Moonglow Toolset").rect(), heading, "the page stays");
    assert_eq!(h.get_all_by_label(&told).count(), 1);
    // (A frame thrown away is not shown, so the strip is never seen above;
    // the page starts from the line in sight and throws none away.)
    assert_eq!(h.state().welcome_counts()[4], before[4], "no pass thrown away");
}

/// Back on the Welcome page after the window was made smaller behind
/// another tab: at every frame the update is told, by the page or by the
/// strip, never by neither.
#[test]
fn back_on_the_page_after_a_resize_the_update_is_always_told() {
    let (release, told) = release_99();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = false;
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    app.updates.notice = Some(release);
    let mut h = harness(app, 1600.0, 1000.0);
    settle(&mut h);
    assert!(h.query_by_label("Close Update Notice").is_none(), "the page tells it");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Manual));
    settle(&mut h);
    h.set_size(egui::vec2(1280.0, 700.0));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Manual));
    for frame in 0..6 {
        h.step();
        let strip = h.query_by_label("Close Update Notice").is_some();
        let page = h.query_all_by_label(&told).count() - usize::from(strip);
        assert_eq!(usize::from(strip) + page, 1, "frame {frame}: strip {strip}, page {page}");
    }
}

/// A list edited by hand that names a module twice shows two rows, each
/// its own (one id each); a module with no folder (a name alone) says no
/// folder.
#[test]
fn a_module_named_twice_has_two_rows_and_a_bare_name_no_folder() {
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    let twice = recent(&["castle.mod"]).remove(0);
    app.settings.recent = vec![twice.clone(), twice, std::path::PathBuf::from("bare.mod")];
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    assert_eq!(h.get_all_by_label("castle.mod in /srv/nwn/modules").count(), 2);
    h.get_by_label("bare.mod");
    assert!(h.query_by_label_contains("bare.mod in").is_none(), "no folder said");
}

/// The page's section headings and its two sub-headings are headings to a
/// screen reader.
#[test]
fn the_page_s_headings_are_headings() {
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    let heading = egui::accesskit::Role::Heading;
    for name in ["Recent Modules", "Start", "Without a Module", "Status", "Learn", "Help"] {
        let by = egui_kittest::kittest::by().label(name).role(heading);
        assert_eq!(h.query_all(by).count(), 1, "{name}");
    }
}

/// Covering only the upper (un-shifted) position of Welcome's update line with
/// a floating window reveals the notice strip without uncovering the rect in
/// the next pass and oscillating `request_discard`.
#[test]
fn a_window_covering_only_the_upper_update_line_does_not_oscillate() {
    let (release, told) = release_99();
    let mut app = app_with(Vec::new());
    app.settings.hide_log = true;
    app.settings.open_editors = mg_ui::OpenEditors::Floating;
    app.updates.notice = Some(release);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    assert!(h.query_by_label("Close Update Notice").is_none(), "the page tells it");
    let line = h.get_by_label(&told).rect();
    let surface = {
        let dock = &mut h.state_mut().dock;
        let surface = dock.add_window(vec![Tab::Manual]);
        let state = dock.get_window_state_mut(surface).unwrap();
        state
            .set_position(egui::pos2(line.left() - 20.0, 100.0))
            .set_size(egui::vec2((line.width() + 40.0).max(400.0), 220.0));
        surface
    };
    settle(&mut h);
    let middle_rects = |h: &Harness<'_, Moonglow>| -> Vec<egui::Rect> {
        h.ctx.memory(|m| {
            m.layer_ids()
                .filter(|l| l.order == egui::Order::Middle && m.areas().is_visible(l))
                .filter_map(|l| m.area_rect(l.id))
                .collect()
        })
    };
    let actual = *middle_rects(&h).first().expect("manual floating window layer rect");
    let target_y = (line.top() + 5.0 - actual.height()).max(40.0);
    if let Some(state) = h.state_mut().dock.get_window_state_mut(surface) {
        state.set_position(egui::pos2(line.left() - 20.0, target_y));
    }
    settle(&mut h);
    let rects = middle_rects(&h);
    assert!(
        rects.iter().any(|r| r.intersects(line) && r.bottom() <= line.top() + 10.0),
        "expected manual window {rects:?} bottom near {} to cover only upper line {line:?}",
        line.top() + 5.0
    );
    h.run_steps(2);
    h.get_by_label("Close Update Notice");
    let discards = h.state().welcome_counts()[4];
    h.run_steps(10);
    assert_eq!(
        h.state().welcome_counts()[4],
        discards,
        "floating window covering upper position must not oscillate request_discard"
    );
    h.get_by_label("Close Update Notice");
    assert_eq!(h.get_all_by_label(&told).count(), 1, "strip alone tells the update");
}
