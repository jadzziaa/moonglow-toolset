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
/// as one step of Undo. (Its pictures: `screens.rs`, `plugin_kinds`.)
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
/// goes when its plugin is disabled. (Its picture: `screens.rs`.)
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

/// A panel that follows the module's edits is drawn once when edits that
/// come one after another (a drag's steps) stop, not for each: its code
/// is not run, nor stopped for the game's data, between them.
#[test]
fn a_plugin_s_panel_is_drawn_once_after_edits_one_after_another() {
    let Some(mut h) = with_example("ui-plugin-panel-quiet", "example.module-overview") else {
        return;
    };
    h.state_mut().background_jobs = true;
    let tab =
        Tab::PluginPanel { plugin: "example.module-overview".into(), panel: "overview".into() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
    h.step();
    settle(&mut h);
    let drawn = |h: &Harness<'_, Moonglow>| h.state().plugins.panels.values().next().unwrap().drawn;
    assert_eq!(drawn(&h), 1);
    let started = std::time::Instant::now();
    let mut longest = std::time::Duration::ZERO;
    for i in 0..10 {
        let before = std::time::Instant::now();
        let key = ResKey::parse(&format!("extra{i}"), ResType::UTC).unwrap();
        let utc = Gff::new(*b"UTC ").to_bytes().unwrap();
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "Another",
            vec![mg_edit::Edit::SetResource { key, data: Some(utc) }],
        )));
        h.step();
        longest = longest.max(before.elapsed());
    }
    let quick = longest < mg_ui::plugin_kinds::PANEL_QUIET;
    if quick {
        assert_eq!(drawn(&h), 1, "not drawn between the edits ({:?})", started.elapsed());
    } else {
        eprintln!("an edit took {longest:?}: the panel may be drawn between them");
    }
    settle(&mut h);
    h.run_steps(2);
    if quick {
        assert_eq!(drawn(&h), 2, "once, after them");
    }
    let shown = h.state().plugins.panels.values().next().unwrap().shown.clone();
    let texts = shown.unwrap().unwrap().texts();
    assert!(texts.contains(&"Creature blueprints: 11".to_string()), "{texts:?}");
}

/// The sample module with a plugin `test.probe` written here (a panel
/// `probe` whose code is `panel`, and a command `apply` that logs the
/// field `n` and what it runs on), enabled, its panel open and drawn.
fn probe_panel(name: &str, panel: &str) -> Option<Harness<'static, Moonglow>> {
    let (mut h, _) = plugin_harness(name, false)?;
    let dir = mg_testkit::scratch_dir(&format!("{name}-plugins"));
    let plugin = dir.join("probe");
    std::fs::create_dir_all(&plugin).unwrap();
    let cfg = "[plugin]\nid = \"test.probe\"\nname = \"Probe\"\nversion = \"0.0.1\"\n\
               api = \"0.9\"\nlicense = \"GPL-3.0-or-later\"\n\n[panel]\nid = \"probe\"\n\
               title = \"Probe\"\n\n[command]\nid = \"apply\"\ntitle = \"Apply\"\n";
    std::fs::write(plugin.join("plugin.cfg"), cfg).unwrap();
    let code = format!(
        "local mg = require(\"@moonglow\")\n\
         mg.panel(\"probe\", function(ctx)\n{panel}\nend)\n\
         mg.command(\"apply\", function(ctx)\n\
             local on = if ctx.selection[1] then ctx.selection[1].resource else \"nothing\"\n\
             ctx.log:info(`applied {{ctx.values.n}} on {{on}}`)\n\
         end)\n"
    );
    std::fs::write(plugin.join("main.luau"), code).unwrap();
    h.state_mut().plugin_dir = Some(dir);
    h.state_mut().load_plugins();
    h.state_mut().enable_plugin("test.probe", true);
    let tab = Tab::PluginPanel { plugin: "test.probe".into(), panel: "probe".into() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
    settle(&mut h);
    h.run_steps(3);
    Some(h)
}

/// A panel's number field given no value, with a `min` above 0, is drawn
/// at its `min` and runs nothing: its command ran as soon as the panel
/// was drawn (the widget made 0 into 1, which was taken for the user's
/// setting it), with nobody having touched the field.
#[test]
fn a_panel_s_number_field_runs_nothing_as_it_is_drawn() {
    let panel = "return { { type = \"field\", kind = \"number\", id = \"n\", label = \"N\", \
                 min = 1, command = \"apply\" } }";
    let Some(mut h) = probe_panel("ui-plugin-panel-number", panel) else { return };
    h.run_steps(4);
    let state = h.state().plugins.panels.values().next().unwrap();
    assert_eq!(state.fields["n"].value, serde_json::json!(1.0), "{:?}", state.shown);
    assert!(!logged(&h, "applied"), "{:?}", h.state().log.entries);
    assert!(!h.state().busy());
}

/// A panel's item or button that names a place (`on`, `open`) acts on it
/// only in the module the panel was drawn from: after an edit the place
/// may be another object's (an object is its area, list and index), so
/// the click runs nothing, says so, and the panel is drawn again. A
/// button that names none runs on what is selected, as ever.
#[test]
fn a_panel_s_places_are_not_acted_on_in_a_module_that_changed() {
    let panel = "return {\n\
                 { type = \"button\", text = \"Run on Selection\", command = \"apply\" },\n\
                 { type = \"list\", items = { { text = \"The guard\", command = \"apply\", \
                 on = { \"guard.utc\" } }, { text = \"Open the guard\", open = \"guard.utc\" } } },\n\
                 }";
    let Some(mut h) = probe_panel("ui-plugin-panel-stale-places", panel) else { return };
    let drawn = |h: &Harness<'_, Moonglow>| h.state().plugins.panels.values().next().unwrap().drawn;
    let edit = |h: &mut Harness<'_, Moonglow>, n: usize| {
        let key = ResKey::parse(&format!("extra{n}"), ResType::UTC).unwrap();
        let utc = Gff::new(*b"UTC ").to_bytes().unwrap();
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "Another",
            vec![mg_edit::Edit::SetResource { key, data: Some(utc) }],
        )));
        h.run_steps(2);
    };
    let said = "Probe: the module changed since this panel was drawn, so nothing was done";
    let guard = Tab::for_resource(ResKey::parse("guard", ResType::UTC).unwrap()).unwrap();
    // As drawn: the item runs its command on the place it names.
    h.get_by_label("The guard").click();
    settle(&mut h);
    h.run_steps(2);
    assert!(logged(&h, "applied nil on guard.utc"), "{:?}", h.state().log.entries);
    let before = (h.state().log.entries.len(), drawn(&h));
    // The module edited, the panel not drawn again (it follows nothing):
    // neither item acts.
    edit(&mut h, 1);
    assert_eq!(drawn(&h), before.1, "it does not follow edits");
    h.get_by_label("The guard").click();
    h.run_steps(1);
    assert!(logged(&h, said), "{:?}", h.state().log.entries);
    settle(&mut h);
    h.run_steps(2);
    let ran = |h: &Harness<'_, Moonglow>| {
        h.state().log.entries.iter().filter(|(_, m)| m.contains("applied")).count()
    };
    assert_eq!(ran(&h), 1, "nothing run: {:?}", h.state().log.entries);
    assert_eq!(drawn(&h), before.1 + 1, "drawn again");
    // A button that names no place runs on what is selected, whatever
    // changed since.
    edit(&mut h, 3);
    h.get_by_label("Run on Selection").click();
    settle(&mut h);
    h.run_steps(2);
    assert_eq!(ran(&h), 2, "{:?}", h.state().log.entries);
    assert!(logged(&h, "applied nil on nothing"));
    edit(&mut h, 2);
    h.get_by_label("Open the guard").click();
    settle(&mut h);
    h.run_steps(2);
    assert!(h.state().dock.find_tab(&guard).is_none(), "nothing opened");
    // Drawn again, both do.
    h.get_by_label("Open the guard").click();
    settle(&mut h);
    h.run_steps(2);
    assert!(h.state().dock.find_tab(&guard).is_some(), "opened");
}

/// A panel is kept with the module's open windows: the module opened again
/// has it again, while its plugin is enabled.
#[test]
fn a_plugin_s_panel_opens_again_with_its_module() {
    let Some(mut h) = with_example("ui-plugin-panel-kept", "example.module-overview") else {
        return;
    };
    h.state_mut().open_tabs_dir = Some(mg_testkit::scratch_dir("ui-plugin-panel-kept-tabs"));
    let path = h.state().module_path().unwrap();
    let tab =
        Tab::PluginPanel { plugin: "example.module-overview".into(), panel: "overview".into() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(4);
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run_steps(2);
    assert!(h.state().dock.find_tab(&tab).is_none(), "closed with the module");
    h.state_mut().open_module(&path);
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&tab).is_some(), "open again");
    h.get_by_label("Nothing is selected.");
    // Its plugin disabled: not opened again.
    h.state_mut().actions.push(mg_ui::Action::Close);
    h.run_steps(2);
    h.state_mut().enable_plugin("example.module-overview", false);
    h.state_mut().open_module(&path);
    h.run_steps(4);
    assert!(h.state().dock.find_tab(&tab).is_none());
}

/// The tint of a placeable of an area's GIT (`tdod_tint`), if it has one.
fn tint_of(h: &mut Harness<'_, Moonglow>, git: &ResKey, i: usize) -> Option<String> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let g = ws.doc(git).unwrap();
    let vars = g.root.list("Placeable List").unwrap()[i].list("VarTable").unwrap_or_default();
    (vars.iter())
        .find(|v| v.string("Name") == Some(&b"tdod_tint"[..]))
        .map(|v| String::from_utf8_lossy(v.string("Value").unwrap()).into_owned())
}

/// An area with two tinted chests (the first white, the second crimson),
/// the example `tint-panel` enabled and its panel open, the first chest
/// selected (in the area's view, the pane last clicked in).
fn tint_panel(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let (mut h, area) = area_harness(name)?;
    let git = ResKey::new(area, ResType::GIT);
    place_tinted_chest(h.state_mut(), git);
    place_tinted_chest(h.state_mut(), git);
    h.run_steps(2);
    let path = mg_edit::GffPath::root().item("Placeable List", 1).item("VarTable", 0);
    let crimson = mg_edit::Edit::SetField {
        key: git,
        path,
        label: "Value".into(),
        value: Some(mg_gff::Value::String(b"#a01010".to_vec())),
    };
    h.state_mut()
        .actions
        .push(mg_ui::Action::Apply(mg_edit::Command::new("Crimson", vec![crimson])));
    h.state_mut().plugin_dir = Some(example_plugins());
    h.state_mut().load_plugins();
    h.state_mut().enable_plugin("example.tint-panel", true);
    h.state_mut().plugins.pane = Some(mg_ui::plugins::Pane::Area(area));
    select_chest(&mut h, area, 0);
    let tab = Tab::PluginPanel { plugin: "example.tint-panel".into(), panel: "tint".into() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
    settle(&mut h);
    h.run_steps(2);
    Some((h, area))
}

/// The chest `i` of the area selected in its view, and the panel drawn
/// again for it.
fn select_chest(h: &mut Harness<'_, Moonglow>, area: ResRef, i: usize) {
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(mg_area::ObjectKind::Placeable, i)];
    settle(h);
    h.run_steps(2);
}

/// What the panel's color field shows (its `#rrggbb` beside the swatch).
fn shown_tint(h: &Harness<'_, Moonglow>) -> String {
    let state = h.state().plugins.panels.values().next().unwrap();
    state.fields["tint"].value.as_str().unwrap().to_string()
}

/// API 0.9: a panel's color field (the example `tint-panel`) shows the
/// selected placeable's own tint and follows the selection; while its
/// picker is open the color is drawn on the placeable (its `variable`) and
/// nothing is set; when the picker closes its command sets it (ctx.values),
/// one step of Undo, and the panel shows it; Undo, and it shows the old
/// tint again. The window is drawn to pictures, before and while picking.
#[test]
fn a_plugin_s_panel_field_follows_the_selection_and_sets_the_tint() {
    let Some((mut h, area)) = tint_panel("plugin-tint-panel") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let pictures = mg_testkit::scratch_dir("ui-plugin-tint-panel");
    h.render().expect("render").save(pictures.join("panel.png")).unwrap();
    eprintln!("the panel: {}", pictures.join("panel.png").display());
    assert_eq!(shown_tint(&h), "#ffffff", "the first chest's own");
    assert!(h.get_all_by_label("#ffffff").next().is_some());
    h.get_by_value("Custom");
    // Another chest selected: its tint, and its tint's name.
    select_chest(&mut h, area, 1);
    assert_eq!(shown_tint(&h), "#a01010");
    assert!(h.get_all_by_label("#a01010").next().is_some());
    h.get_by_value("Crimson");
    select_chest(&mut h, area, 0);
    assert_eq!(shown_tint(&h), "#ffffff", "and back");
    let before = undo(&h);

    // Picked: drawn on the chest, nothing set.
    let white = vec![1.0, 1.0, 1.0, 1.0];
    let well = h.get_by(|n| n.role() == egui::accesskit::Role::ColorWell).rect();
    h.get_by(|n| n.role() == egui::accesskit::Role::ColorWell).click();
    h.run_steps(3);
    drag_picker(&mut h, well, 0.0, 80.0);
    h.render().expect("render").save(pictures.join("picking.png")).unwrap();
    let picked = drawn_tint(&h, area).expect("drawn with a tint");
    assert_ne!(picked, white, "the color being picked, drawn");
    assert_eq!(tint_of(&mut h, &git, 0).as_deref(), Some("#ffffff"), "nothing set");
    assert_eq!(undo(&h), before);
    let color = shown_tint(&h);
    assert_ne!(color, "#ffffff");

    // The picker closed (a click on the panel): its command, one step.
    let away = h.get_by_label_contains("Tint panel").rect().center();
    click_away(&mut h, away);
    settle(&mut h);
    h.run_steps(2);
    assert_eq!(tint_of(&mut h, &git, 0), Some(color.clone()), "set as picked");
    assert_eq!(tint_of(&mut h, &git, 1).as_deref(), Some("#a01010"), "only on the one selected");
    assert_eq!(undo(&h), Some(format!("Tint {color}")));
    assert_eq!(drawn_tint(&h, area), Some(picked), "drawn as set");
    assert_eq!(shown_tint(&h), color, "the panel shows it");
    // Undo: the old tint, in the panel too (it gives another value).
    h.state_mut().actions.push(mg_ui::Action::Undo);
    settle(&mut h);
    h.run_steps(2);
    assert_eq!(tint_of(&mut h, &git, 0).as_deref(), Some("#ffffff"));
    assert_eq!(undo(&h), before, "one step");
    assert_eq!(shown_tint(&h), "#ffffff");
}

/// API 0.9: a panel's choice whose options have colors (the example's
/// named tints): each drawn with its swatch, in the list and in the box;
/// a choice made runs its command with the panel's values, and the panel
/// follows. The open list is drawn to a picture.
#[test]
fn a_plugin_s_panel_choice_has_swatches_and_runs_its_command_when_changed() {
    let Some((mut h, area)) = tint_panel("plugin-tint-choice") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let pictures = mg_testkit::scratch_dir("ui-plugin-tint-choice");
    h.get_by_value("Custom").click();
    h.run_steps(3);
    for name in ["Crimson", "Forest", "Night blue", "Bone"] {
        h.get_by_label(name);
    }
    h.render().expect("render").save(pictures.join("choice.png")).unwrap();
    eprintln!("the named tints: {}", pictures.join("choice.png").display());
    h.get_by_label("Forest").click();
    settle(&mut h);
    h.run_steps(2);
    assert_eq!(tint_of(&mut h, &git, 0).as_deref(), Some("#2f6b2a"));
    assert_eq!(undo(&h).as_deref(), Some("Tint #2f6b2a"));
    assert_eq!(shown_tint(&h), "#2f6b2a", "the color field follows");
    h.get_by_value("Forest");
    h.render().expect("render").save(pictures.join("chosen.png")).unwrap();
}
