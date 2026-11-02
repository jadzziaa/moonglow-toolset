//! What plugins add besides commands and checks (API 0.6): exporters in
//! File › Export, importers in File › Import, wizards in the Wizards menu
//! and panels in panes of their own.

use crate::common::*;

fn example_plugins() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples")
}

/// The sample module with a lower-case guard, the example plugins
/// installed and `enabled` enabled.
fn with_example(name: &str, enabled: &str) -> Option<Harness<'static, Moonglow>> {
    let (mut h, _) = plugin_harness(name, false)?;
    h.state_mut().plugin_dir = Some(example_plugins());
    h.state_mut().load_plugins();
    h.state_mut().enable_plugin(enabled, true);
    h.run_steps(2);
    Some(h)
}

fn logged(h: &Harness<'_, Moonglow>, part: &str) -> bool {
    h.state().log.entries.iter().any(|(_, m)| m.contains(part))
}

fn undo(h: &Harness<'_, Moonglow>) -> Option<String> {
    h.state().ws.as_ref().unwrap().can_undo().map(String::from)
}

/// File › Export becomes a submenu with Moonglow's own Export… first and
/// the plugin's exporters after; one writes the file chosen in the save
/// dialog, of the module, and changes nothing (nothing to undo). One for
/// what is selected is offered only when that is selected.
#[test]
fn a_plugin_s_exporter_writes_the_file_chosen_and_changes_nothing() {
    let Some(mut h) = with_example("ui-plugin-export", "example.creature-sheet") else { return };
    let out = mg_testkit::scratch_dir("ui-plugin-export-out").join("creatures.csv");
    h.state_mut().dialogs = Box::new(NoDialogs { save: vec![out.clone()], ..Default::default() });
    let before = undo(&h);
    h.get_by_label("File").click();
    h.run();
    assert!(h.query_by_label("Export…").is_none(), "in its submenu, not the menu");
    h.get_by_label("Export").click();
    h.run();
    h.get_by_label("Export…");
    assert!(
        h.get_by_label("Selected Creatures as CSV…").accesskit_node().is_disabled(),
        "nothing is selected that it exports"
    );
    h.get_by_label("Creatures as CSV…").click();
    h.run_steps(3);
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.starts_with("resref,tag,name,level\r\n"), "{text}");
    assert!(text.contains("guard,gate_guard,"), "{text}");
    assert_eq!(undo(&h), before, "an export changes nothing");
    assert!(logged(&h, "Creature sheet: 1 creatures written to creatures.csv"));
    assert!(logged(&h, "Undo doesn't take them back"), "{:?}", h.state().log.entries);

    // The guard chosen in the palette: the one for what is selected runs.
    let guard = ResKey::parse("guard", ResType::UTC).unwrap();
    h.state_mut().palette.selected = Some(guard);
    let picked = out.with_file_name("picked.csv");
    h.state_mut().dialogs =
        Box::new(NoDialogs { save: vec![picked.clone()], ..Default::default() });
    h.state_mut().actions.push(mg_ui::Action::RunPluginEntry {
        kind: mg_ui::plugin_kinds::Added::Exporter,
        plugin: "example.creature-sheet".into(),
        id: "selected-csv".into(),
    });
    h.run_steps(3);
    let text = std::fs::read_to_string(&picked).unwrap();
    assert_eq!(text.lines().count(), 2, "{text}");
}

/// File › Import › an importer: the file chosen is read, and its edits
/// are one step of Undo, named as the plugin says; Undo takes them back.
#[test]
fn a_plugin_s_importer_reads_the_file_chosen_as_one_undoable_step() {
    let Some(mut h) = with_example("ui-plugin-import", "example.creature-sheet") else { return };
    let sheet = mg_testkit::scratch_dir("ui-plugin-import-in").join("sheet.csv");
    std::fs::write(&sheet, "resref,tag,name,level\nguard,NIGHT_GUARD,Night Guard,1\n").unwrap();
    h.state_mut().dialogs = Box::new(NoDialogs { open: vec![sheet], ..Default::default() });
    let guard = ResKey::parse("guard", ResType::UTC).unwrap();
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Import").click();
    h.run();
    h.get_by_label("Import…");
    h.get_by_label("Creatures from CSV…").click();
    h.run_steps(3);
    assert_eq!(tag_of(&mut h, &guard), "NIGHT_GUARD");
    assert_eq!(undo(&h).as_deref(), Some("Import 1 creatures"));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert_eq!(tag_of(&mut h, &guard), "gate_guard", "one step");
}

/// Without a plugin that exports or imports, File › Export and Import are
/// Moonglow's own commands, as in Aurora.
#[test]
fn without_plugins_file_export_and_import_are_moonglow_s_own() {
    let Some((mut h, _)) = plugin_harness("ui-plugin-no-export", false) else { return };
    h.run();
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Export…");
    h.get_by_label("Import…");
}

/// A wizard in the Wizards menu runs as a job whose pages are asked in its
/// window: Back keeps what was typed, a page's check keeps the page with
/// what it says, a page skipped is not shown, and Finish makes the store
/// as one step of Undo. The page is drawn to a picture.
#[test]
fn a_plugin_s_wizard_asks_page_by_page_and_finishes_in_one_step() {
    let Some(mut h) = with_example("ui-plugin-wizard", "example.store-wizard") else { return };
    h.state_mut().background_jobs = true;
    let shows = |h: &mut Harness<'_, Moonglow>, label: &str| {
        let started = std::time::Instant::now();
        while h.query_by_label(label).is_none() {
            assert!(started.elapsed().as_secs() < 60, "{label:?} never showed");
            std::thread::sleep(std::time::Duration::from_millis(10));
            h.step();
        }
    };
    h.get_by_label("Wizards").click();
    h.run();
    h.get_by_label("Shop Wizard…").click();
    h.step();
    shows(&mut h, "Page 1 of 3");
    h.get_by_label("The store");
    assert!(h.get_by_label("< Back").accesskit_node().is_disabled(), "the first page");
    // An empty name: the check says so, and the page stays.
    let name = typed_name(&h, "General Store");
    let field = name.accesskit_node().id();
    name.focus();
    h.step();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.key_press(egui::Key::Backspace);
    h.step();
    h.get_by_label("Next >").click();
    h.step();
    shows(&mut h, "A store needs a name.");
    h.get_by_label("Page 1 of 3");
    h.get_by(move |n| n.id() == field).focus();
    h.step();
    h.event(egui::Event::Text("Iron Hand".into()));
    h.step();
    h.step();
    typed_name(&h, "Iron Hand");
    h.get_by_label("Next >").click();
    h.step();
    shows(&mut h, "Iron Hand's prices");
    h.get_by_label("Page 2 of 3");
    // Back: the name typed is kept. (A page's buttons take a click once
    // it has been drawn.)
    h.step();
    h.get_by_label("< Back").click();
    h.step();
    shows(&mut h, "Page 1 of 3");
    typed_name(&h, "Iron Hand");
    h.step();
    h.get_by_label("Next >").click();
    h.step();
    shows(&mut h, "Page 2 of 3");
    let picture = mg_testkit::scratch_dir("ui-plugin-wizard-page").join("page.png");
    if let Ok(img) = h.render() {
        img.save(&picture).unwrap();
        eprintln!("the page: {}", picture.display());
    }
    // (A general store: the third page is skipped, and Next finishes.)
    h.get_by_label("Next >").click();
    h.step();
    settle(&mut h);
    let store = ResKey::parse("ironhand", ResType::UTM).unwrap();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(ws.module.contains(&store), "the store is made");
    assert_eq!(undo(&h).as_deref(), Some("New store Iron Hand"));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&store), "one step");
}

/// A wizard canceled on a page makes nothing.
#[test]
fn a_plugin_s_wizard_canceled_makes_nothing() {
    let Some(mut h) = with_example("ui-plugin-wizard-cancel", "example.store-wizard") else {
        return;
    };
    let before = undo(&h);
    let stores = |h: &Harness<'_, Moonglow>| {
        h.state().ws.as_ref().unwrap().module.keys().filter(|k| k.restype == ResType::UTM).count()
    };
    let had = stores(&h);
    // (The first page answered, the second closed.)
    let first = [("name", serde_json::json!("Pawn")), ("kind", serde_json::json!("General"))];
    let values = first.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    h.state_mut().plugins.answers.push_back(Some(mg_plugin::Answer::Page { values, back: false }));
    h.state_mut().plugins.answers.push_back(None);
    h.state_mut().actions.push(mg_ui::Action::RunPluginEntry {
        kind: mg_ui::plugin_kinds::Added::Wizard,
        plugin: "example.store-wizard".into(),
        id: "new-store".into(),
    });
    h.run_steps(3);
    assert_eq!(stores(&h), had);
    assert_eq!(undo(&h), before);
    assert!(logged(&h, "Store wizard: Shop Wizard canceled"), "{:?}", h.state().log.entries);
}

/// Plugins › Panels › a panel opens it as a pane of its own, which shows
/// what its code returns: drawn again after an edit (its manifest says
/// `edits`) and when what is selected changes (`selection`); its button
/// runs its plugin's command on what is selected, one step of Undo; and it
/// goes when its plugin is disabled. The panel is drawn to a picture.
#[test]
fn a_plugin_s_panel_shows_what_it_returns_and_follows_the_module() {
    let Some(mut h) = with_example("ui-plugin-panel", "example.module-overview") else { return };
    h.get_by_label("Plugins").click();
    h.run();
    h.get_by_label("Panels").click();
    h.run();
    h.get_by_label("    Module Overview").click();
    settle(&mut h);
    let tab =
        Tab::PluginPanel { plugin: "example.module-overview".into(), panel: "overview".into() };
    assert!(h.state().dock.find_tab(&tab).is_some(), "open");
    h.get_by_label("The module");
    h.get_by_label("Nothing is selected.");
    let count = |h: &Harness<'_, Moonglow>| {
        let shown = h.state().plugins.panels.values().next().unwrap().shown.clone();
        let texts = shown.unwrap().unwrap().texts();
        texts.into_iter().find(|t| t.starts_with("Creature blueprints: ")).unwrap()
    };
    assert_eq!(count(&h), "Creature blueprints: 1");
    // An edit: another creature, and the panel counts it.
    let key = ResKey::parse("captain", ResType::UTC).unwrap();
    let mut captain = Gff::new(*b"UTC ");
    captain.root.set("Tag", mg_gff::Value::String(b"captain".to_vec()));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A captain",
        vec![mg_edit::Edit::SetResource { key, data: Some(captain.to_bytes().unwrap()) }],
    )));
    settle(&mut h);
    h.run_steps(2);
    assert_eq!(count(&h), "Creature blueprints: 2");
    // What is selected: the guard in the palette, and the button for its
    // tag, which runs the plugin's command on it.
    let guard = ResKey::parse("guard", ResType::UTC).unwrap();
    h.state_mut().palette.selected = Some(guard);
    settle(&mut h);
    h.run_steps(2);
    h.get_by_label("1 of them not upper case");
    let picture = mg_testkit::scratch_dir("ui-plugin-panel-shot").join("panel.png");
    if let Ok(img) = h.render() {
        img.save(&picture).unwrap();
        eprintln!("the panel: {}", picture.display());
    }
    h.get_by_label("Upper-Case Their Tags").click();
    settle(&mut h);
    h.run_steps(2);
    assert_eq!(tag_of(&mut h, &guard), "GATE_GUARD");
    assert_eq!(undo(&h).as_deref(), Some("Upper-case tags"));
    assert!(h.query_by_label("Upper-Case Their Tags").is_none(), "drawn again after");
    // Disabled: its panel goes.
    h.state_mut().enable_plugin("example.module-overview", false);
    h.run_steps(2);
    assert!(h.state().dock.find_tab(&tab).is_none(), "closed with its plugin");
}

/// A panel's render runs beside the window, not as a job: the window is
/// not busy while it draws, and what it shows comes when it is done.
#[test]
fn a_plugin_s_panel_draws_beside_the_window() {
    let Some(mut h) = with_example("ui-plugin-panel-thread", "example.module-overview") else {
        return;
    };
    h.state_mut().background_jobs = true;
    let tab =
        Tab::PluginPanel { plugin: "example.module-overview".into(), panel: "overview".into() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
    h.step();
    assert!(!h.state().busy(), "no job");
    settle(&mut h);
    h.run_steps(2);
    h.get_by_label("Nothing is selected.");
    h.get_by_label("⟳ Refresh").click();
    settle(&mut h);
    h.get_by_label("Nothing is selected.");
}
