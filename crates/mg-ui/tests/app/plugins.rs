//! Plugins: installed and project plugins, their commands, events,
//! checks and settings.

use crate::common::*;

/// API 0.4: a plugin's command for waypoints is in the area view's menu on
/// them, above Variables and Properties, and runs on every object
/// selected; with nothing selected, the area's commands are there. Its
/// edits are one undoable step. The menu is drawn to a picture.
#[test]
fn a_plugin_s_command_runs_on_the_objects_selected_in_the_area() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("plugin-menu") else { return };
    let git = ResKey::new(area, ResType::GIT);
    h.state_mut().plugin_dir = Some(example_plugins());
    h.state_mut().load_plugins();
    h.state_mut().enable_plugin("example.set-local", true);
    h.run_steps(2);
    let right_click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run_steps(3);
    };
    // Both waypoints selected; the menu opened on the first keeps both.
    let both = vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    h.state_mut().area_views.get_mut(&area).unwrap().selection = both.clone();
    right_click(&mut h, Vec3::new(20.0, 20.0, 0.9));
    assert_eq!(h.state().area_views[&area].selection, both);
    let command = h.get_by_label("Set Local Variable…").rect();
    let properties = h.get_all_by_label("Properties").last().unwrap().rect();
    assert!(command.bottom() <= properties.top(), "{command:?} {properties:?}");
    assert!(h.query_by_label("Count Objects").is_none(), "that one is for areas");
    let img = h.render().expect("render");
    let picture = mg_testkit::scratch_dir("ui-area-plugin-menu").join("menu.png");
    img.save(&picture).unwrap();
    eprintln!("the menu: {}", picture.display());
    h.state_mut().plugins.answers.push_back(set_local_answer());
    h.get_by_label("Set Local Variable…").click();
    h.run_steps(3);
    for i in 0..2 {
        let on = Some(("WaypointList", i));
        assert_eq!(variables(&mut h, &git, on), ["role = workbench"], "waypoint {i}");
    }
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Set role"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert!(variables(&mut h, &git, Some(("WaypointList", 1))).is_empty(), "one undo");

    // Nothing selected: the area's command, run on the area.
    h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
    right_click(&mut h, Vec3::new(8.0, 30.0, 0.0));
    assert!(h.query_by_label("Set Local Variable…").is_none());
    h.get_by_label("Count Objects").click();
    h.run_steps(3);
    let log: Vec<String> = h.state().log.entries.iter().map(|(_, m)| m.clone()).collect();
    assert!(
        log.iter().any(|m| m.starts_with("Set local: ") && m.ends_with(": 2 waypoints")),
        "{log:#?}"
    );
}

/// A plugin that says so acts by itself when the module is saved, with
/// the resources the save changed; one that is off does nothing.
#[test]
fn a_plugin_acts_when_the_module_is_saved() {
    let Some((mut h, key)) = plugin_harness("ui-plugin-events", false) else { return };
    h.run();
    let said = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let log = h.state().log.entries.iter().map(|(_, m)| m.clone());
        log.filter(|m| m.contains("module-saved")).collect()
    };
    let save = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::Save);
        h.run_steps(4);
    };
    // Off: a save is only a save.
    save(&mut h);
    assert!(said(&h).is_empty(), "{:?}", said(&h));
    h.state_mut().enable_plugin("example.tag-conventions", true);
    h.state_mut().plugins.window = true;
    h.run();
    h.get_by_label("By itself, when the module is saved: saved");
    h.state_mut().plugins.window = false;
    // Nothing changed since the last save: nothing to say.
    save(&mut h);
    assert!(said(&h).is_empty(), "{:?}", said(&h));
    // The guard changes and is saved: the plugin is told which.
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("Tag", mg_gff::Value::String(b"night_guard".to_vec()));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Another guard",
        vec![mg_edit::Edit::SetResource { key, data: Some(guard.to_bytes().unwrap()) }],
    )));
    h.run_steps(2);
    save(&mut h);
    assert_eq!(said(&h), ["Tag conventions: module-saved: guard.utc"]);
    assert!(!h.state().busy());
    save(&mut h);
    assert_eq!(said(&h).len(), 1, "once for a change");
    // What a plugin's events were allowed is kept in the settings, by
    // plugin and version, shown in the Plugins window and forgotten from
    // there, or when the plugin is enabled or disabled. (Another of the
    // plugins installed: this one's are what its events just did.)
    let (id, name) = (h.state().plugins.installed.iter())
        .filter_map(|i| i.plugin.as_ref().ok())
        .map(|p| (p.manifest.id.clone(), format!("{} {}", p.manifest.id, p.manifest.version)))
        .find(|(id, _)| id != "example.tag-conventions")
        .unwrap();
    let grant = |h: &mut Harness<'_, Moonglow>| {
        let other = ("other 1.0.0".to_string(), "q".to_string(), "yes".to_string());
        h.state_mut().settings.plugin_grants =
            vec![(name.clone(), "q".into(), "yes".into()), other];
    };
    grant(&mut h);
    assert_eq!(h.state().plugin_grants(&id), 1);
    h.state_mut().plugins.window = true;
    h.run_steps(3);
    h.get_by_label("Allowed before, and not asked again: 1").scroll_to_me();
    h.run_steps(2);
    h.get_by_label("Forget").click();
    h.run_steps(3);
    assert_eq!(h.state().plugin_grants(&id), 0);
    assert_eq!(h.state().settings.plugin_grants.len(), 1, "another plugin's stay");
    grant(&mut h);
    h.state_mut().enable_plugin(&id, true);
    assert_eq!(h.state().plugin_grants(&id), 0);
}

/// A plugin that asks before it acts on a save asks once: what it was
/// allowed is kept in the settings, so it is not asked again at the next
/// save nor after a restart, until it is forgotten.
#[test]
fn a_plugin_s_event_asks_once_and_not_after_a_restart() {
    let Some((mut h, key)) = plugin_harness("ui-plugin-asks", false) else { return };
    h.run();
    h.state_mut().enable_plugin("test.asks", true);
    let said = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let log = h.state().log.entries.iter().map(|(_, m)| m.clone());
        log.filter(|m| m.starts_with("Asks: ")).collect()
    };
    let mut n = 0;
    let mut save_changed = |h: &mut Harness<'_, Moonglow>| {
        n += 1;
        let mut guard = Gff::new(*b"UTC ");
        guard.root.set("Tag", mg_gff::Value::String(format!("guard_{n}").into_bytes()));
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "A guard",
            vec![mg_edit::Edit::SetResource { key, data: Some(guard.to_bytes().unwrap()) }],
        )));
        h.run_steps(2);
        h.state_mut().actions.push(mg_ui::Action::Save);
        h.run_steps(6);
    };
    // Asked, and answered yes.
    h.state_mut().plugins.answers.push_back(Some(mg_plugin::Answer::Yes));
    save_changed(&mut h);
    assert_eq!(said(&h), ["Asks: noted"]);
    assert!(h.state().plugins.answers.is_empty(), "the question was asked");
    // The next save: not asked (there is no answer to give), and done.
    save_changed(&mut h);
    assert_eq!(said(&h).len(), 2, "{:?}", h.state().log.entries);
    assert!(!h.state().busy());
    assert_eq!(h.state().plugin_grants("test.asks"), 1, "kept in the settings");
    // After a restart (what this run holds is gone): still not asked.
    h.state_mut().forget_plugin_session();
    save_changed(&mut h);
    assert_eq!(said(&h), ["Asks: noted"; 3]);
    assert!(!h.state().busy());
    // Forgotten: asked again, and a no this time is a no.
    h.state_mut().forget_plugin_grants("test.asks");
    h.state_mut().plugins.answers.push_back(Some(mg_plugin::Answer::No));
    save_changed(&mut h);
    assert_eq!(said(&h).last().unwrap(), "Asks: not noted");
    assert!(h.state().plugins.answers.is_empty());
    assert_eq!(h.state().plugin_grants("test.asks"), 0, "a no is not kept");
}

#[test]
fn a_plugin_is_off_until_enabled_and_its_command_is_one_undoable_step() {
    let Some((mut h, key)) = plugin_harness("ui-plugins", false) else { return };
    h.run();
    // Installed, none enabled: the Plugins menu has only its manager.
    assert_eq!(h.state().plugins.installed.len(), 5);
    assert!(h.state().plugin_commands().is_empty());
    h.get_by_label("Plugins").click();
    h.run();
    assert!(h.query_by_label("Fix Creature Tags").is_none());
    h.get_by_label("Manage Plugins…").click();
    h.run();
    // The window says what each adds; enabling one is a tick.
    h.get_by_label_contains("cannot reach your files");
    h.get_by_label_contains("Plugins are experimental: the plugin API (0.4)");
    h.get_by_label("Command: Fix Creature Tags");
    h.get_by_label("Check: Creature tags are upper case");
    // (The last of the four installed: below the list's fold.)
    h.get_by_label("Tag conventions 1.0.0").scroll_to_me();
    h.run();
    h.get_by_label("Tag conventions 1.0.0").click();
    h.run();
    assert_eq!(h.state().settings.plugins_enabled, ["example.tag-conventions"]);
    h.state_mut().plugins.window = false;
    h.run();

    // Its commands are in the Plugins menu; one runs as one undoable step.
    h.get_by_label("Plugins").click();
    h.run();
    h.get_by_label("Promote the Guards");
    h.get_by_label("Fix Creature Tags").click();
    h.run();
    assert_eq!(tag_of(&mut h, &key), "GATE_GUARD");
    let log: Vec<String> = h.state().log.entries.iter().map(|(_, m)| m.clone()).collect();
    assert!(log.contains(&"Tag conventions: 1 creature tags changed".to_string()), "{log:#?}");
    assert!(
        log.iter().any(|m| m.contains("Fix Creature Tags changed 1 resource (Edit › Undo")),
        "{log:#?}"
    );
    assert_eq!(
        h.state().ws.as_ref().unwrap().can_undo(),
        Some("Upper-case creature tags"),
        "named by the plugin"
    );
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(tag_of(&mut h, &key), "gate_guard");
    // Run again with nothing to do: it says so, and adds no step.
    h.state_mut().run_plugin_command("example.tag-conventions", "fix-tags");
    h.run();
    h.state_mut().run_plugin_command("example.tag-conventions", "fix-tags");
    h.run();
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("changed nothing")));

    // The Command Palette finds it, under its plugin's name.
    h.key_press_modifiers(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
    h.run();
    type_into_hint(&mut h, "Type a command's name", "tag conv");
    h.get_by_label("Promote the Guards");
    h.key_press(egui::Key::Escape);
    h.run();
    // And it takes a key like any command: listed with the plugins'.
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run();
    h.get_by_label("Keyboard").click();
    h.run();
    type_into_hint(&mut h, "Find a command", "plugins");
    h.get_by_label("Tag conventions: Fix Creature Tags");
    h.state_mut().options.as_mut().unwrap().recording =
        Some("example.tag-conventions/fix-tags".to_string());
    h.run();
    h.key_press(egui::Key::F8);
    h.run();
    h.get_by_label("OK").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(tag_of(&mut h, &key), "gate_guard");
    h.key_press(egui::Key::F8);
    h.run();
    assert_eq!(tag_of(&mut h, &key), "GATE_GUARD", "F8 runs the plugin's command");

    // Disabled again: its commands are gone.
    h.state_mut().enable_plugin("example.tag-conventions", false);
    h.run();
    assert!(h.state().plugin_commands().is_empty());
    // A plugin whose code fails changes nothing and says why.
    h.state_mut().enable_plugin("test.hostile", true);
    h.state_mut().run_plugin_command("test.hostile", "fails");
    h.run();
    assert!(
        h.state()
            .log
            .entries
            .iter()
            .any(|(l, m)| { *l == mg_ui::Level::Error && m.contains("something went wrong") })
    );
    assert_eq!(tag_of(&mut h, &key), "GATE_GUARD");
}

/// The example plugins of `docs/plugins/examples`, installed.
fn example_plugins() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples")
}

/// The answer to `set-local`'s form: `role`, a string, `workbench`.
fn set_local_answer() -> Option<mg_plugin::Answer> {
    let values = std::collections::BTreeMap::from([
        ("name".to_string(), serde_json::json!("role")),
        ("type".to_string(), serde_json::json!("string")),
        ("value".to_string(), serde_json::json!("workbench")),
    ]);
    Some(mg_plugin::Answer::Values(values))
}

/// The names of the variables of a struct of a resource (at `list`'s item,
/// or the root).
fn variables(
    h: &mut Harness<'_, Moonglow>,
    key: &ResKey,
    at: Option<(&str, usize)>,
) -> Vec<String> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let g = ws.doc(key).unwrap();
    let s = match at {
        Some((list, i)) => &g.root.list(list).unwrap()[i],
        None => &g.root,
    };
    (s.list("VarTable").unwrap_or_default().iter())
        .map(|v| {
            let text = |l: &str| String::from_utf8_lossy(v.string(l).unwrap()).into_owned();
            format!("{} = {}", text("Name"), text("Value"))
        })
        .collect()
}

/// API 0.4: a plugin's command for a kind of object, or for areas, is in
/// their right-click menus (the module tree's here), before the Properties
/// that end an area's; chosen there, it runs on what was clicked, and its
/// edits are one undoable step. Off, or for another kind, it isn't there.
#[test]
fn a_plugin_s_command_is_in_the_menus_of_what_it_is_for() {
    let Some((mut h, key)) = plugin_harness("ui-plugin-menus", false) else { return };
    // The area's objects: a chest.
    let git = ResKey::parse("start", ResType::GIT).unwrap();
    let mut placed = Gff::new(*b"GIT ");
    let mut chest = mg_gff::Struct::new(9);
    chest.set("Tag", mg_gff::Value::String(b"CHEST".to_vec()));
    placed.root.set("Placeable List", mg_gff::Value::List(vec![chest]));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A chest",
        vec![mg_edit::Edit::SetResource { key: git, data: Some(placed.to_bytes().unwrap()) }],
    )));
    h.state_mut().plugin_dir = Some(example_plugins());
    h.state_mut().load_plugins();
    h.run();
    h.get_by_label_contains("Creatures (1)").click();
    h.run();
    // Off: not in the menu.
    h.get_by_label("guard.utc").click_secondary();
    h.run();
    h.get_by_label("Copy to Scratch Folder");
    assert!(h.query_by_label("Set Local Variable…").is_none());
    h.key_press(egui::Key::Escape);
    h.run();
    h.state_mut().enable_plugin("example.set-local", true);
    h.run();
    // A blueprint's row: its command, run on it.
    h.get_by_label("guard.utc").click_secondary();
    h.run();
    assert!(h.query_by_label("Count Objects").is_none(), "that one is for areas");
    h.state_mut().plugins.answers.push_back(set_local_answer());
    h.get_by_label("Set Local Variable…").click();
    h.run_steps(3);
    assert!(h.state().plugins.answers.is_empty(), "its form was asked");
    assert_eq!(variables(&mut h, &key, None), ["role = workbench"]);
    let log: Vec<String> = h.state().log.entries.iter().map(|(_, m)| m.clone()).collect();
    assert!(log.contains(&"Set local: gate_guard: role = workbench".to_string()), "{log:#?}");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Set role"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert!(variables(&mut h, &key, None).is_empty(), "one undo");

    // An area's row: its command above the Properties that end the menu,
    // run on the area.
    h.get_by_label("start").click_secondary();
    h.run();
    assert!(h.query_by_label("Set Local Variable…").is_none(), "that one is for objects");
    let count = h.get_by_label("Count Objects").rect();
    let properties = h.get_all_by_label("Properties").last().unwrap().rect();
    assert!(count.bottom() <= properties.top(), "{count:?} {properties:?}");
    h.get_by_label("Count Objects").click();
    h.run_steps(3);
    let log: Vec<String> = h.state().log.entries.iter().map(|(_, m)| m.clone()).collect();
    assert!(log.contains(&"Set local: start: 1 placeables".to_string()), "{log:#?}");

    // From the Plugins menu, with nothing selected: it says so. (And
    // the Plugins window says which menus each is in.)
    h.state_mut().run_plugin_command("example.set-local", "set-local");
    h.run_steps(3);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("A chest"), "nothing changed");
    h.state_mut().plugins.window = true;
    h.run();
    h.get_by_label_contains("Command: Set Local Variable (right-click: creature, door,");
    h.get_by_label("Command: Count Objects (right-click: area)");
}

/// API 0.4: in a palette, a plugin's command for the kind is last in a
/// blueprint's menu, and runs on the blueprint (a module's: changed, as
/// one undoable step; one of the game's is the game's, and the example
/// leaves it alone).
#[test]
fn a_plugin_s_command_is_in_a_palette_s_menu() {
    let Some((mut h, _)) = plugin_harness("ui-plugin-palette", true) else { return };
    // A waypoint blueprint of the module's, in its palette's Custom 1.
    let key = ResKey::parse("mg_gate", ResType::UTW).unwrap();
    {
        let game = h.state().game.as_deref().unwrap();
        let bytes = game.resman.get_named("nw_waypoint001", ResType::UTW).unwrap();
        let mut g = Gff::read(&bytes).unwrap();
        g.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
        let name = LocString::from_text(Language::ENGLISH, Gender::Male, "Gate A");
        g.root.set("LocalizedName", mg_gff::Value::LocString(name));
        g.root.set("PaletteID", mg_gff::Value::Byte(0));
        let set = mg_edit::Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) };
        h.state_mut()
            .actions
            .push(mg_ui::Action::Apply(mg_edit::Command::new("A gate", vec![set])));
    }
    h.state_mut().plugin_dir = Some(example_plugins());
    h.state_mut().load_plugins();
    h.state_mut().enable_plugin("example.set-local", true);
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    {
        let app = h.state_mut();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
        app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
        app.palette.tiles = false;
        app.palette.filter = "nw_wp_tavern".into();
    }
    h.run();
    // The game's: the command gets it, says it is not the module's, and
    // changes nothing.
    h.get_by_label(&tavern).click_secondary();
    h.run();
    h.get_by_label("Set Local Variable…").click();
    h.run_steps(3);
    let ws = h.state().ws.as_ref().unwrap();
    assert_eq!(ws.can_undo(), Some("A gate"), "nothing changed");
    // The module's, in the custom palette.
    {
        let app = h.state_mut();
        app.palette.custom = true;
        app.palette.filter = "gate".into();
    }
    h.run_steps(2);
    h.get_by_label("Gate A").click_secondary();
    h.run();
    h.state_mut().plugins.answers.push_back(set_local_answer());
    h.get_by_label("Set Local Variable…").click();
    h.run_steps(3);
    assert_eq!(variables(&mut h, &key, None), ["role = workbench"]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Set role"));
}

/// A plugin's questions show in its job's window, one after another, and
/// its work goes on with the answers.
#[test]
fn a_plugin_asks_in_its_job_s_window() {
    let Some((mut h, key)) = plugin_harness("ui-plugin-forms", false) else { return };
    h.state_mut().enable_plugin("test.forms", true);
    h.state_mut().background_jobs = true;
    h.run();
    h.state_mut().run_plugin_command("test.forms", "rename");
    // Frames until something with this label shows (the job's thread asks).
    let wait_for = |h: &mut Harness<'_, Moonglow>, label: &str| {
        let started = std::time::Instant::now();
        while h.query_by_label(label).is_none() {
            h.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(5));
            assert!(started.elapsed() < std::time::Duration::from_secs(20), "no {label:?}");
        }
        // (The window settles where it stands before it is clicked.)
        h.run_steps(3);
    };
    // The form, as the plugin described it, with its defaults.
    wait_for(&mut h, "Prefix Tags");
    h.get_by_label("Forms: Prefix Tags");
    h.get_by_label("At most");
    assert!(h.get_all_by_value("NPC_").next().is_some(), "the prefix's default");
    h.get_by_label("OK").click();
    // Then its question, then its message.
    wait_for(&mut h, "Prefix up to 10 creatures?");
    h.get_by_label("Yes").click();
    wait_for(&mut h, "Done.");
    h.get_by_label("OK").click();
    let started = std::time::Instant::now();
    while h.state().busy() {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "the job never ended");
    }
    h.run_steps(2);
    assert_eq!(tag_of(&mut h, &key), "NPC_GATE_GUARD");

    // Canceled at the form: nothing changes.
    h.state_mut().run_plugin_command("test.forms", "rename");
    wait_for(&mut h, "Prefix Tags");
    h.get_by_label("Cancel").click();
    let started = std::time::Instant::now();
    while h.state().busy() {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "the job never ended");
    }
    h.run_steps(2);
    assert_eq!(tag_of(&mut h, &key), "NPC_GATE_GUARD");
}

#[test]
fn the_plugin_console_runs_what_is_typed() {
    let Some((mut h, key)) = plugin_harness("ui-plugin-console", false) else { return };
    h.run();
    h.state_mut().plugins.window = true;
    h.state_mut().plugins.console =
        "ctx.edit:set(\"guard.utc\", \"Tag\", \"TYPED\")\nreturn ctx.module:gff(\"guard.utc\").Tag"
            .into();
    h.run();
    h.get_by_label("Console").click();
    h.run();
    h.get_by_label("Run").click();
    h.run();
    assert_eq!(tag_of(&mut h, &key), "TYPED");
    h.get_by_label("\"TYPED\"");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Plugin console"));
    // A fault shows under the console, and changes nothing.
    h.state_mut().plugins.console = "return nothing.here".into();
    h.run();
    h.get_by_label("Run").click();
    h.run();
    // (Under the console, and in the log.)
    assert_eq!(h.get_all_by_label_contains("attempt to index nil").count(), 2);
    assert_eq!(tag_of(&mut h, &key), "TYPED");
}

/// A plugin's checks run with Verify Module (which needs the game).
#[test]
fn a_plugin_s_checks_run_with_verify() {
    let Some((mut h, _)) = plugin_harness("ui-plugin-verify", true) else {
        eprintln!("skipped: no game install");
        return;
    };
    h.state_mut().enable_plugin("example.tag-conventions", true);
    h.state_mut().enable_plugin("test.hostile", true);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Verify);
    h.run();
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Warning
            && m.contains("guard.utc › Tag: tag \"gate_guard\" is not upper case")),
        "{log:#?}"
    );
    // A check that breaks the rules is an error of its own, not a crash.
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Error && m.contains("a check only reads")),
        "{log:#?}"
    );
    // The log's Clear empties it, before a second Verify.
    h.get_by_label("Clear Log").click();
    h.run();
    assert!(h.state().log.entries.is_empty());
    // The plugins' checks left out (Options › Verify): the plugins aren't
    // run, so neither what they would find nor what they would say is
    // reported, and the summary names them as not checked.
    h.state_mut().settings.verify_skip = vec!["plugins".into()];
    h.state_mut().actions.push(mg_ui::Action::Verify);
    h.run();
    let log = &h.state().log.entries;
    assert!(
        !log.iter().any(|(_, m)| m.contains("gate_guard") || m.contains("a check only reads")),
        "{log:#?}"
    );
    assert!(
        log.iter().any(|(_, m)| m.starts_with("Verify: ")
            && m.ends_with("; left out, not checked (Options › Verify): Plugins' checks")),
        "{log:#?}"
    );
}

/// A finding's fix is a button beside its line in Verify's report: made
/// only when clicked, one step to undo; refused once what it changes is
/// not as Verify found it.
#[test]
fn a_finding_s_fix_is_made_when_its_button_is_clicked() {
    let Some((mut h, key)) = plugin_harness("ui-plugin-fix", true) else {
        eprintln!("skipped: no game install");
        return;
    };
    let tag = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let tag = ws.doc(&key).unwrap().root.string("Tag").unwrap().to_vec();
        String::from_utf8(tag).unwrap()
    };
    h.state_mut().enable_plugin("example.tag-conventions", true);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Verify);
    h.run();
    assert_eq!(tag(&mut h), "gate_guard", "nothing is changed unasked");
    let line = h.state().log.entries.iter().position(|(_, m)| m.contains("gate_guard")).unwrap();
    assert_eq!(h.state().log.fix_at(line).unwrap().fix.label, "Upper-case this tag");
    h.get_by_label("Upper-case this tag").click();
    h.run();
    assert_eq!(tag(&mut h), "GATE_GUARD");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Upper-case this tag"));
    assert!(h.state().log.fix_at(line).unwrap().made);
    h.get_by_label("✔ Fixed");
    // Undone, and verified again: then changed by hand, its fix is
    // refused, and says why.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(tag(&mut h), "gate_guard");
    h.get_by_label("Clear Log").click();
    h.run();
    assert!(h.state().log.fixes.is_empty(), "the log's fixes go with it");
    h.state_mut().actions.push(mg_ui::Action::Verify);
    h.run();
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("Tag", mg_gff::Value::String(b"gate_guard".to_vec()));
    guard.root.set("Comment", mg_gff::Value::String(b"changed".to_vec()));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "By hand",
        vec![mg_edit::Edit::SetResource { key, data: Some(guard.to_bytes().unwrap()) }],
    )));
    h.run();
    h.get_by_label("Upper-case this tag").click();
    h.run();
    let log = &h.state().log.entries;
    assert!(
        log.iter()
            .any(|(l, m)| *l == mg_ui::Level::Error && m.contains("has changed since Verify")),
        "{log:#?}"
    );
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("By hand"));
}

/// Plugins › Install Plugin from File…: a plugin's archive goes into the
/// plugins folder, off; installing it again asks before it replaces; what
/// is no plugin's archive changes nothing.
#[test]
fn a_plugin_installs_from_a_file_and_is_off() {
    let dir = mg_testkit::scratch_dir("ui-plugin-install");
    let plugins = dir.join("plugins");
    // The fixture plugin's folder, to pack as it is and as a later version.
    let work = dir.join("work/tag-conventions");
    std::fs::create_dir_all(&work).unwrap();
    for file in ["plugin.cfg", "main.luau", "rules.luau"] {
        std::fs::copy(plugin_fixtures().join("tag-conventions").join(file), work.join(file))
            .unwrap();
    }
    let pack = |to: &str| {
        let plugin = mg_plugin::Plugin::load(&work).unwrap();
        let path = dir.join(to);
        std::fs::write(&path, mg_plugin::pack(&plugin).unwrap()).unwrap();
        path
    };
    let first = pack("tag-conventions-1.0.0.zip");
    let manifest = std::fs::read_to_string(work.join("plugin.cfg")).unwrap();
    std::fs::write(work.join("plugin.cfg"), manifest.replace("1.0.0", "1.1.0")).unwrap();
    let second = pack("tag-conventions-1.1.0.zip");
    let junk = dir.join("junk.zip");
    std::fs::write(&junk, "not an archive").unwrap();

    let mut app = app_with(vec![first]);
    app.plugin_dir = Some(plugins.clone());
    // (Enabled once, and removed since: installing does not bring that back.)
    app.settings.plugins_enabled = vec!["example.tag-conventions".to_string()];
    app.load_plugins();
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    h.get_by_label("Plugins").click();
    h.run();
    h.get_by_label("Install Plugin from File…").click();
    h.run();
    // (In the window, and in the log, after its time.)
    let said =
        |h: &Harness<'_, Moonglow>, text: &str| h.get_all_by_label_contains(text).count() == 2;
    assert!(said(
        &h,
        "Installed Tag conventions 1.0.0 from tag-conventions-1.0.0.zip: it is off until you \
         enable it"
    ));
    assert_eq!(h.state().plugins.installed.len(), 1);
    assert!(!h.state().plugin_enabled("example.tag-conventions"));
    assert!(plugins.join("example.tag-conventions/main.luau").is_file());
    h.get_by_label("Tag conventions 1.0.0").click();
    h.run();
    assert!(h.state().plugin_enabled("example.tag-conventions"));

    // Again, from the window: asked first, and Cancel leaves it.
    let install = |h: &mut Harness<'_, Moonglow>, file: &std::path::Path| {
        h.state_mut().dialogs =
            Box::new(NoDialogs { open: vec![file.to_path_buf()], ..Default::default() });
        h.get_by_label("Install from File…").click();
        h.run();
        h.run();
    };
    install(&mut h, &second);
    h.get_by_label("Replace Tag conventions?");
    h.get_by_label(
        "Tag conventions 1.0.0 is installed. Replace it with 1.1.0 from \
         tag-conventions-1.1.0.zip?",
    );
    h.get_by_label_contains("It is enabled, and stays enabled.");
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().plugins.replace.is_none());
    h.get_by_label("Tag conventions 1.0.0");
    install(&mut h, &second);
    h.get_by_label("Replace").click();
    h.run();
    h.get_by_label("Tag conventions 1.1.0");
    assert!(said(
        &h,
        "Installed Tag conventions 1.1.0 from tag-conventions-1.1.0.zip: it stays enabled"
    ));
    assert!(h.state().plugin_enabled("example.tag-conventions"));

    // What is no plugin's archive says why, and changes nothing.
    install(&mut h, &junk);
    assert!(said(&h, "junk.zip was not installed: it is not a zip archive"));
    assert!(h.state().log.entries.iter().any(|(l, m)| *l == mg_ui::Level::Error
        && m == "junk.zip was not installed: it is not a zip archive"));
    assert_eq!(h.state().plugins.installed.len(), 1);
    h.get_by_label("Tag conventions 1.1.0");

    // Remove: what Install from File put there, after a question. A
    // plugin copied in by hand has no Remove: Moonglow deletes only what
    // it installed.
    let by_hand = plugins.join("by-hand");
    std::fs::create_dir_all(&by_hand).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        std::fs::copy(plugin_fixtures().join("forms").join(file), by_hand.join(file)).unwrap();
    }
    h.get_by_label("Reload").click();
    h.run();
    let disabled: Vec<bool> =
        h.get_all_by_label("Remove…").map(|n| n.accesskit_node().is_disabled()).collect();
    assert_eq!(disabled, [true, false], "by-hand, then the one installed");
    let remove = |h: &mut Harness<'_, Moonglow>| {
        let button =
            h.get_all_by_label("Remove…").find(|n| !n.accesskit_node().is_disabled()).unwrap();
        button.scroll_to_me();
        h.run();
        h.get_all_by_label("Remove…").find(|n| !n.accesskit_node().is_disabled()).unwrap().click();
        h.run();
        h.run();
    };
    remove(&mut h);
    h.get_by_label("Remove Tag conventions 1.1.0?");
    h.get_by_label("Cancel").click();
    h.run();
    assert!(plugins.join("example.tag-conventions").is_dir());
    remove(&mut h);
    h.get_by_label("Remove").click();
    h.run();
    assert!(!plugins.join("example.tag-conventions").exists());
    assert!(said(&h, "Removed Tag conventions 1.1.0"));
    assert!(!h.state().plugin_enabled("example.tag-conventions"));
    assert_eq!(h.state().plugins.installed.len(), 1);
    assert!(by_hand.join("main.luau").is_file());
}

/// Started with `--no-plugins`: none is loaded, whatever is installed and
/// was enabled, and Manage Plugins says so.
#[test]
fn started_without_plugins_none_is_loaded() {
    let mut app = app_with(Vec::new());
    app.plugin_dir = Some(plugin_fixtures());
    app.settings.plugins_enabled = vec!["example.tag-conventions".to_string()];
    app.no_plugins = true;
    app.load_plugins();
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    assert!(h.state().plugins.installed.is_empty());
    assert!(h.state().plugin_commands().is_empty());
    h.get_by_label("Plugins").click();
    h.run();
    assert!(h.query_by_label("Fix Creature Tags").is_none());
    h.get_by_label("Manage Plugins…").click();
    h.run();
    h.get_by_label("Started with --no-plugins: none are loaded.");
    assert!(h.query_by_label_contains("No plugins are installed").is_none());
    // (What was enabled stays enabled for the next start.)
    assert_eq!(h.state().settings.plugins_enabled, ["example.tag-conventions"]);
}

/// A plugin asks for a folder and for leave to write a hak in its job's
/// window; the hak is written into the user folder and the module lists
/// it.
#[test]
fn a_plugin_reads_a_chosen_folder_and_writes_a_hak() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-plugin-outside");
    let path = sample_module(&dir);
    let (user, export) = (dir.join("user"), dir.join("export"));
    std::fs::create_dir_all(&user).unwrap();
    std::fs::create_dir_all(&export).unwrap();
    std::fs::write(export.join("note.txt"), "from the folder").unwrap();
    let dialogs = NoDialogs { folders: vec![export], ..Default::default() };
    let mut app = game_app_with(&root, Some(user.clone()), dialogs);
    app.open_module(&path);
    app.open_palette = false;
    app.background_jobs = true;
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    h.state_mut().plugins.console = r#"
        local folder = ctx.ui:open_folder({ title = "The folder to take" })
        folder:to_hak("made", "note.txt")
        ctx.hak:attach("made")
    "#
    .into();
    h.state_mut().run_console();
    let wait_for = |h: &mut Harness<'_, Moonglow>, label: &str| {
        let started = std::time::Instant::now();
        while h.query_by_label(label).is_none() {
            h.run_steps(1);
            std::thread::sleep(std::time::Duration::from_millis(5));
            assert!(started.elapsed() < std::time::Duration::from_secs(20), "no {label:?}");
        }
        h.run_steps(3);
    };
    wait_for(&mut h, "Choose Folder…");
    h.get_by_label("The folder to take");
    h.get_by_label("Choose Folder…").click();
    // Then its leave to write the hak, in the API's own words.
    wait_for(&mut h, "Yes");
    h.get_by_label_contains("made.hak in your hak folder");
    h.get_by_label("Yes").click();
    let started = std::time::Instant::now();
    while h.state().busy() {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(started.elapsed() < std::time::Duration::from_secs(20), "the job never ended");
    }
    h.run_steps(2);
    let hak = mg_module::hak_edit::Hak::open(&user.join("hak/made.hak")).unwrap();
    assert_eq!(hak.data(ResKey::parse("note", ResType::TXT).unwrap()).unwrap(), b"from the folder");
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.flush().unwrap();
    assert_eq!(ws.module.haks().unwrap(), ["made"]);
    assert!(
        h.state().log.entries.iter().any(|(_, m)| m.contains("put 1 resources into")),
        "{:?}",
        h.state().log.entries
    );
}

/// A project (a module folder) with a creature whose tag lacks the team's
/// prefix, that prefix in its moonglow.toml, and the builder-tags example
/// in its `plugins/` folder.
fn plugin_project(name: &str) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    // (Its answers, beside it: none from an earlier run.)
    let _ = std::fs::remove_file(dir.with_extension("answers.toml"));
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut captain = Gff::new(*b"UTC ");
    captain.root.set("Tag", mg_gff::Value::String(b"captain".to_vec()));
    m.set_gff(ResKey::parse("captain", ResType::UTC).unwrap(), &captain).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(
        dir.join("moonglow.toml"),
        "# The team's.\n[plugins.\"example.builder-tags\"]\nprefix = \"KRZ_\"\n",
    )
    .unwrap();
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/plugins/examples/builder-tags");
    let into = dir.join("plugins/builder-tags");
    std::fs::create_dir_all(&into).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        std::fs::copy(example.join(file), into.join(file)).unwrap();
    }
    dir
}

/// The window on a project of [`plugin_project`]'s, opened, with the
/// fixture plugins installed (none enabled); with the game where `game`.
fn project_harness(dir: &std::path::Path, game: bool) -> Option<Harness<'static, Moonglow>> {
    let mut app = if game {
        let root = mg_testkit::nwn_root()?;
        game_app(&root, None)
    } else {
        app_with(Vec::new())
    };
    app.plugin_dir = Some(plugin_fixtures());
    // (Moonglow's own folder, for the answers: a scratch one beside the
    // project, never in it.)
    app.plugin_answers = Some(dir.with_extension("answers.toml"));
    app.load_plugins();
    app.open_module(dir);
    app.open_palette = false;
    Some(harness(app, 1200.0, 900.0))
}

fn runs(h: &Harness<'_, Moonglow>, id: &str) -> bool {
    h.state().enabled_plugins().iter().any(|p| p.manifest.id == id)
}

/// A project's plugins are asked about when it is opened, with what each
/// adds and keeps; none runs until allowed, and the answer is kept by the
/// plugin's content in the builder's own file: a change asks again, a no
/// stays a no, and Decide Later asks at the next opening.
#[test]
fn a_project_s_plugins_are_asked_about_when_it_is_opened() {
    let dir = plugin_project("ui-project-plugins");
    let id = "example.builder-tags";
    let mut h = project_harness(&dir, false).unwrap();
    h.run();
    assert_eq!(h.state().plugins.project_asking, [id]);
    h.get_by_label("This project comes with plugins");
    h.get_by_label_contains("Its folder plugins/ holds plugins the team shares");
    h.get_by_label("Builder tags 1.0.0");
    h.get_by_label("Command: Prefix Tags (right-click: creature, placeable)");
    h.get_by_label("Check: Tags begin with the team's prefix");
    h.get_by_label(
        "Settings the team shares (the project's moonglow.toml): Team prefix, Checked, Longest tag",
    );
    h.get_by_label("Settings of yours for this project: Your code");
    assert!(!runs(&h, id), "not before it is allowed");
    assert!(!h.state().plugin_commands().iter().any(|c| c.plugin == id));
    // Put off: still off, and asked at the next opening.
    h.get_by_label("Decide Later").click();
    h.run();
    assert!(h.state().plugins.project_asking.is_empty() && !runs(&h, id));
    assert!(h.query_by_label("This project comes with plugins").is_none());
    h.state_mut().open_module(&dir);
    h.run();
    assert_eq!(h.state().plugins.project_asking, [id]);
    // Allowed: it runs, its commands are there, and it is not asked again.
    h.get_by_label("Allow").click();
    h.run();
    assert!(runs(&h, id));
    assert!(h.state().plugin_commands().iter().any(|c| c.plugin == id && c.title == "Prefix Tags"));
    // Kept in Moonglow's own folder, by the project and the plugin's
    // content; nothing written into the project.
    let kept = std::fs::read_to_string(dir.with_extension("answers.toml")).unwrap();
    assert!(kept.contains("plugin = \"example.builder-tags\""), "{kept}");
    assert!(kept.contains("allow = true") && kept.contains("digest = \"sha256:"), "{kept}");
    assert!(!dir.join(".moonglow").exists(), "nothing in the project");
    assert!(
        h.state()
            .log
            .entries
            .iter()
            .any(|(_, m)| m.starts_with("The project's plugin Builder tags 1.0.0 is allowed"))
    );
    h.state_mut().open_module(&dir);
    h.run();
    assert!(h.state().plugins.project_asking.is_empty() && runs(&h, id));
    // Changed (a pull brought other code): off, and asked again.
    let main = dir.join("plugins/builder-tags/main.luau");
    let code = std::fs::read_to_string(&main).unwrap();
    std::fs::write(&main, format!("{code}\n-- changed\n")).unwrap();
    h.state_mut().open_module(&dir);
    h.run();
    assert!(!runs(&h, id));
    h.get_by_label("Changed since you allowed 1.0.0: it is off until you allow it again.");
    h.get_by_label("Don't Allow").click();
    h.run();
    assert!(!runs(&h, id));
    // A no stays a no: not asked again, and listed as off.
    h.state_mut().open_module(&dir);
    h.run();
    assert!(h.state().plugins.project_asking.is_empty());
    h.state_mut().plugins.window = true;
    h.run();
    h.get_by_label("This project's plugins").scroll_to_me();
    h.run();
    h.get_by_label("You said no: it is off.");
    // Enabled from the Plugins window: allowed as it is now.
    h.get_by_label("Builder tags 1.0.0").click();
    h.run();
    assert!(runs(&h, id));
    h.get_by_label("Allowed, as it is now: it runs.");
    // Closed, the project's plugins go with it.
    h.state_mut().close();
    h.run();
    assert!(!h.state().plugins.installed.iter().any(|i| i.project));
}

/// A project that comes with answers of its own (a committed
/// `.moonglow/user.toml` with its plugin "allowed", digest and all) allows
/// nothing: the section is said in the log, and the plugin is asked about.
#[test]
fn answers_a_project_carries_are_not_taken() {
    let dir = plugin_project("ui-project-plugins-carried");
    let example = dir.join("plugins/builder-tags");
    let digest = mg_plugin::Plugin::load_project(&example).unwrap().digest().unwrap().to_string();
    let claim = format!(
        "[project_plugins]\n\"example.builder-tags\" = {{ allow = true, version = \"1.0.0\", \
         digest = {digest:?} }}\n"
    );
    mg_module::project_file::ProjectFile::write_user(&dir, &claim).unwrap();
    let mut h = project_harness(&dir, false).unwrap();
    h.run();
    assert_eq!(h.state().plugins.project_asking, ["example.builder-tags"]);
    assert!(!runs(&h, "example.builder-tags"));
    h.get_by_label("This project comes with plugins");
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Warning
            && m.starts_with(".moonglow/user.toml:1: [project_plugins] is not read")),
        "{log:#?}"
    );
}

/// A project's plugin and an installed one of the same id: while the
/// project is open and its own is allowed, that one runs in its stead.
#[test]
fn a_project_s_plugin_stands_in_for_one_installed_of_its_id() {
    let dir = plugin_project("ui-project-plugins-shadow");
    let plugins = mg_testkit::scratch_dir("ui-project-plugins-shadow-installed");
    let installed = plugins.join("builder-tags");
    std::fs::create_dir_all(&installed).unwrap();
    for file in ["plugin.cfg", "main.luau"] {
        std::fs::copy(dir.join("plugins/builder-tags").join(file), installed.join(file)).unwrap();
    }
    let mut h = project_harness(&dir, false).unwrap();
    h.state_mut().plugin_dir = Some(plugins);
    h.state_mut().load_plugins();
    h.state_mut().enable_plugin("example.builder-tags", true);
    h.run();
    let ours = |h: &Harness<'_, Moonglow>| -> Vec<bool> {
        h.state().enabled_plugins().iter().map(|p| p.is_project()).collect()
    };
    assert_eq!(ours(&h), [false], "the installed one, until the project's is allowed");
    h.get_by_label("Allow").click();
    h.run();
    assert_eq!(ours(&h), [true], "the project's, in its stead");
    h.state_mut().plugins.window = true;
    h.run();
    h.get_by_label_contains("The project has a plugin of this id, which runs in its stead");
}

/// A plugin's settings in their form: each kept where its scope says (the
/// team's in moonglow.toml, the builder's own in .moonglow/user.toml,
/// the user's in Moonglow's settings), the rest of each file as it was;
/// Defaults takes them out again.
#[test]
fn a_plugin_s_settings_are_edited_in_their_form() {
    let dir = plugin_project("ui-plugin-settings-form");
    let id = "example.builder-tags";
    let mut h = project_harness(&dir, false).unwrap();
    h.run();
    h.get_by_label("Allow").click();
    h.run();
    h.state_mut().plugins.window = true;
    h.run();
    h.get_by_label("This project's plugins").scroll_to_me();
    h.run();
    h.get_by_label("Settings…").click();
    h.run();
    h.get_by_label("Settings: Builder tags");
    for label in ["Team prefix", "Checked", "Longest tag", "Your code", "Report as errors"] {
        h.get_by_label(label);
    }
    assert_eq!(h.get_all_by_label("the team's").count(), 3, "prefix, kinds, longest");
    assert_eq!(h.get_all_by_label("yours, for this project").count(), 1);
    {
        let form = h.state().plugins.settings_form.clone().unwrap();
        assert_eq!(form.values["prefix"], mg_plugin::SettingValue::Text("KRZ_".into()));
        assert_eq!(form.values["max_length"], mg_plugin::SettingValue::Int(32));
    }
    // Changed in the form, and saved.
    {
        let form = h.state_mut().plugins.settings_form.as_mut().unwrap();
        let text = |t: &str| mg_plugin::SettingValue::Text(t.into());
        form.values.insert("prefix".into(), text("QQ_"));
        form.values.insert("code".into(), text("ab"));
        form.values.insert("kinds".into(), text("creatures"));
        form.values.insert("strict".into(), mg_plugin::SettingValue::Bool(true));
    }
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert!(h.state().plugins.settings_form.is_none());
    let team = std::fs::read_to_string(dir.join("moonglow.toml")).unwrap();
    assert_eq!(
        team,
        "# The team's.\n[plugins.\"example.builder-tags\"]\nprefix = \"QQ_\"\nkinds = \"creatures\"\n"
    );
    let own = std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap();
    assert!(own.contains("[plugins.\"example.builder-tags\"]\ncode = \"ab\"\n"), "{own}");
    assert_eq!(h.state().settings.plugin_settings[id]["strict"], serde_json::json!(true));
    let s = h.state().plugin_settings(h.state().project_plugins()[0]);
    assert_eq!(s.values["code"], mg_plugin::SettingValue::Text("ab".into()));
    assert!(h.state().log.entries.iter().any(|(_, m)| m
        == "Builder tags: Team prefix is \"QQ_\" (the team's, in the project's moonglow.toml)"));
    // Defaults: each taken out of where it was kept.
    h.state_mut().open_plugin_settings(id);
    h.run();
    h.get_by_label("Defaults").click();
    h.run();
    h.get_by_label("Save").click();
    h.run();
    // (The table stays, for the comment over it.)
    assert_eq!(
        std::fs::read_to_string(dir.join("moonglow.toml")).unwrap(),
        "# The team's.\n[plugins.\"example.builder-tags\"]\n"
    );
    assert!(!h.state().settings.plugin_settings.contains_key(id));
    // A .mod has no project: those of the project's scopes can't be set.
    let other = mg_testkit::scratch_dir("ui-plugin-settings-mod");
    let path = sample_module(&other);
    h.state_mut().open_module(&path);
    h.state_mut().plugin_dir = Some(dir.join("plugins"));
    h.state_mut().load_plugins();
    h.state_mut().open_plugin_settings(id);
    h.run();
    let prefix = h.state().plugins.settings_form.as_ref().unwrap().values["prefix"].clone();
    assert_eq!(prefix, mg_plugin::SettingValue::Text(String::new()), "its default");
}

/// ctx.settings in the window: a command's settings are kept when it is
/// done, and a check reads them (the team's prefix, the user's strictness).
#[test]
fn a_plugin_reads_and_sets_its_settings_in_the_window() {
    let dir = plugin_project("ui-plugin-ctx-settings");
    let Some(mut h) = project_harness(&dir, true) else {
        eprintln!("skipped: no game install");
        return;
    };
    h.run();
    h.get_by_label("Allow").click();
    h.run();
    let id = "example.builder-tags";
    // Its command asks for the code, and sets it.
    let values = std::collections::BTreeMap::from([("code".to_string(), serde_json::json!("zz"))]);
    h.state_mut().plugins.answers.push_back(Some(mg_plugin::Answer::Values(values)));
    h.state_mut().run_plugin_command(id, "remember-code");
    h.run_steps(4);
    let own = std::fs::read_to_string(dir.join(".moonglow/user.toml")).unwrap();
    assert!(own.contains("code = \"zz\""), "{own}");
    assert!(h.state().log.entries.iter().any(|(_, m)| m == "Builder tags: Your code is \"zz\" (yours, for this project, in its .moonglow/user.toml)"));
    // Its check, with the team's prefix and the user's strictness.
    h.state_mut().settings.plugin_settings.insert(
        id.into(),
        std::collections::BTreeMap::from([("strict".to_string(), serde_json::json!(true))]),
    );
    h.state_mut().actions.push(mg_ui::Action::Verify);
    h.run();
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Error
            && m.contains("captain.utc › Tag: tag \"captain\" does not begin with KRZ_")),
        "{log:#?}"
    );
}

/// A module (kept in one file) with an area tagged TAVERN, opened with the
/// example plugins installed and House rules enabled.
fn house_rules_harness(name: &str) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(name);
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    let mut a = ifo::MOD_AREA_LIST.new_item();
    a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str("tavern").unwrap());
    info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
    m.set_info(&info).unwrap();
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", mg_gff::Value::String(b"TAVERN".to_vec()));
    m.set(ResKey::parse("tavern", ResType::ARE).unwrap(), are.to_bytes().unwrap());
    let path = dir.join("rules.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.open_palette = false;
    app.plugin_dir = Some(example_plugins());
    app.load_plugins();
    app.enable_plugin("example.house-rules", true);
    harness(app, 1200.0, 900.0)
}

fn set_area_tag(h: &mut Harness<'_, Moonglow>, area: &str, tag: &str) {
    let edit = mg_edit::Edit::SetField {
        key: ResKey::parse(area, ResType::ARE).unwrap(),
        path: mg_edit::GffPath::root(),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(tag.as_bytes().to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
    h.run_steps(2);
}

/// A plugin that refuses what a save would write stops it before anything
/// is written, says why, and offers its fix; made, the save is asked
/// again, and goes on.
#[test]
fn a_plugin_refuses_a_save_and_says_why() {
    let mut h = house_rules_harness("ui-plugin-save-refused");
    h.run();
    let saved = |h: &Harness<'_, Moonglow>| !h.state().ws.as_ref().unwrap().is_modified();
    set_area_tag(&mut h, "tavern", "INN");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert!(!saved(&h), "nothing written");
    h.get_by_label("Not saved");
    h.get_by_label("Refused");
    h.get_by_label_contains("its tag was TAVERN, now INN: players' places are kept by it");
    assert!(h.query_by_label("Save Anyway").is_none(), "a refusal is not saved anyway");
    // Its fix: made (a step of Undo), and the save asked again: it goes on.
    h.get_by_label("Put back TAVERN").click();
    h.run_steps(4);
    assert_eq!(area_tag(&mut h, "tavern"), "TAVERN");
    assert!(h.state().plugins.save.held.is_none());
    assert!(saved(&h), "saved once the fix was made");
    assert!(h.state().log.entries.iter().any(|(_, m)| m == "House rules: fixed: Put back TAVERN"));
    // Close, without the fix: not saved, and the log says why.
    set_area_tag(&mut h, "tavern", "INN");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    h.get_by_label("Close").click();
    h.run_steps(2);
    assert!(!saved(&h));
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Error
            && m == "House rules: tavern.are: its tag was TAVERN, now INN: players' places are kept by it"),
        "{log:#?}"
    );
}

/// A plugin's warning about what a save would write is asked about before
/// the save: Cancel writes nothing, Save Anyway writes it; a save it says
/// nothing of is not held.
#[test]
fn a_plugin_warns_before_a_save() {
    let mut h = house_rules_harness("ui-plugin-save-warned");
    h.run();
    let saved = |h: &Harness<'_, Moonglow>| !h.state().ws.as_ref().unwrap().is_modified();
    // A new area, its tag not upper case.
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", mg_gff::Value::String(b"Cellar".to_vec()));
    let key = ResKey::parse("cellar", ResType::ARE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A cellar",
        vec![mg_edit::Edit::SetResource { key, data: Some(are.to_bytes().unwrap()) }],
    )));
    h.run_steps(2);
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    h.get_by_label("Before saving");
    h.get_by_label("Warning");
    h.get_by_label("tag Cellar is not upper case");
    h.get_by_label("Make it CELLAR");
    h.get_by_label("Cancel").click();
    h.run_steps(2);
    assert!(!saved(&h), "canceled");
    assert!(h.state().plugins.save.held.is_none());
    // Asked again; Save Anyway writes it as it is.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    h.get_by_label("Save Anyway").click();
    h.run_steps(3);
    assert!(saved(&h), "saved anyway");
    assert_eq!(area_tag(&mut h, "cellar"), "Cellar");
    // Nothing more to say: the next save is not held.
    set_area_tag(&mut h, "tavern", "TAVERN");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    assert!(h.state().plugins.save.held.is_none());
    // And the Plugins window says what the plugin does by itself.
    h.state_mut().plugins.window = true;
    h.run();
    h.get_by_label_contains("before a save, reads each resource it writes (are)");
    h.get_by_label_contains("when objects (waypoint) are pasted: may change them");
}

/// What a plugin does on a resource made (a copy here; a wizard's alike):
/// its edits are part of the making's command, and one Undo takes both
/// back. A plugin that fails leaves the making as core made it.
#[test]
fn a_plugin_edits_what_is_made_in_the_same_undo() {
    let plugins = mg_testkit::scratch_dir("ui-plugin-made-plugins");
    let dir = plugins.join("made");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("plugin.cfg"),
        "[plugin]\nid = \"test.made\"\nname = \"Made\"\nversion = \"1\"\napi = \"0.4\"\n\
         license = \"MIT\"\n[event]\nid = \"made\"\non = \"resource-created\"\nfor = \"creature\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.luau"),
        r#"
        local mg = require("@moonglow")
        mg.on("made", function(ctx)
            local e = ctx.event
            local tag = ctx.module:gff(e.resource).Tag
            if tag == "BROKEN" then error("this one breaks") end
            ctx.edit:set(e.resource, "Tag", `{tag}_{e.by}`)
        end)
        "#,
    )
    .unwrap();
    let Some((mut h, key)) = plugin_harness("ui-plugin-made", false) else { return };
    h.state_mut().plugin_dir = Some(plugins);
    h.state_mut().load_plugins();
    h.state_mut().enable_plugin("test.made", true);
    h.run();
    let copy = |h: &mut Harness<'_, Moonglow>, name: &str| {
        h.state_mut().actions.push(mg_ui::Action::CopyDialog(key));
        h.run_steps(3);
        h.state_mut().copy_as.as_mut().unwrap().resref = name.into();
        h.run_steps(2);
        h.get_by_label("Create Copy").click();
        h.run_steps(3);
        ResKey::parse(name, ResType::UTC).unwrap()
    };
    let new = copy(&mut h, "guard_two");
    let tag = |h: &mut Harness<'_, Moonglow>, k: &ResKey| -> Option<String> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let doc = ws.doc(k).ok()?;
        Some(String::from_utf8_lossy(doc.root.string("Tag")?).into_owned())
    };
    assert_eq!(tag(&mut h, &new).as_deref(), Some("gate_guard_copy"));
    let undo = h.state().ws.as_ref().unwrap().can_undo().map(String::from);
    assert_eq!(undo.as_deref(), Some("Copy guard.utc as guard_two.utc"));
    assert!(h.state().log.entries.iter().any(|(_, m)| m
        == "Made: resource-created changed 1 resource with Copy guard.utc as guard_two.utc (one \
            Undo takes both back)"));
    // One Undo: the copy and the plugin's edit of it, both gone.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(2);
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&new));
    assert_eq!(tag(&mut h, &key).as_deref(), Some("gate_guard"), "the original as it was");
    // A plugin that fails: the copy is made as core makes it, and said.
    let mut guard = Gff::new(*b"UTC ");
    guard.root.set("Tag", mg_gff::Value::String(b"BROKEN".to_vec()));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Broken",
        vec![mg_edit::Edit::SetResource { key, data: Some(guard.to_bytes().unwrap()) }],
    )));
    h.run_steps(2);
    let other = copy(&mut h, "guard_three");
    assert_eq!(tag(&mut h, &other).as_deref(), Some("BROKEN"));
    let log = &h.state().log.entries;
    assert!(
        log.iter().any(|(l, m)| *l == mg_ui::Level::Error
            && m.contains("this one breaks")
            && m.ends_with("Copy guard.utc as guard_three.utc is made without it")),
        "{log:#?}"
    );
}

/// What a plugin does on objects pasted: its edits are part of the paste,
/// one Undo takes both back, and the core's check of doubled tags comes
/// after it (a tag it made the copy's own is not one).
#[test]
fn a_plugin_edits_what_is_pasted_in_the_same_undo() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("ui-plugin-pasted") else { return };
    // (Options › General › Show duplicate tags warning: off by default.)
    h.state_mut().settings.warn_duplicate_tags = true;
    h.state_mut().plugin_dir = Some(example_plugins());
    h.state_mut().load_plugins();
    h.state_mut().enable_plugin("example.house-rules", true);
    h.run_steps(2);
    let git = ResKey::new(area, ResType::GIT);
    let tags = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let g = ws.doc(&git).unwrap();
        let list = g.root.list("WaypointList").unwrap();
        list.iter()
            .map(|w| String::from_utf8_lossy(w.string("Tag").unwrap()).into_owned())
            .collect()
    };
    // The second waypoint, with a tag of its own, copied and pasted.
    let edit = mg_edit::Edit::SetField {
        key: git,
        path: mg_edit::GffPath::root().item("WaypointList", 1),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(b"WP_DOOR".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
    h.run_steps(2);
    let before = tags(&mut h);
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 1)];
    let at = Vec3::new(15.0, 30.0, 0.0);
    let over = screen(&h, area, at);
    h.hover_at(over);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(2);
    let over = screen(&h, area, at);
    h.hover_at(over);
    press(&h, over, true, egui::Modifiers::NONE);
    press(&h, over, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let after = tags(&mut h);
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(after.last().map(String::as_str), Some("WP_DOOR_2"), "the plugin's tag");
    assert!(h.state().tag_clash.is_none(), "no tag doubled: the plugin made it the copy's own");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Paste"));
    assert!(
        h.state()
            .log
            .entries
            .iter()
            .any(|(_, m)| m == "House rules: 1 pasted waypoints have tags of their own")
    );
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(tags(&mut h), before, "one Undo: the paste and the plugin's edit");
}

/// A save that both the claims and a plugin ask about: each is asked once,
/// the claims first; Save Anyway in each, and it is saved.
#[test]
fn a_save_the_claims_and_a_plugin_ask_about_is_asked_once_each() {
    let (_dir, mut app) = claimed_project(
        "ui-claims-and-plugin",
        "[claims]\nfile = \"claims.2da\"\n",
        &claims_table("cd", "claimed"),
    );
    app.plugin_dir = Some(example_plugins());
    app.load_plugins();
    app.enable_plugin("example.house-rules", true);
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    // Another builder's area changed, and a new area whose tag the plugin
    // warns of.
    h.state_mut().actions.push(rename_start("Market"));
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", mg_gff::Value::String(b"Cellar".to_vec()));
    let key = ResKey::parse("cellar", ResType::ARE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A cellar",
        vec![mg_edit::Edit::SetResource { key, data: Some(are.to_bytes().unwrap()) }],
    )));
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    h.get_by_label("Areas Not Yours");
    h.get_by_label("Save Anyway").click();
    h.run_steps(3);
    h.get_by_label("Before saving");
    h.get_by_label("tag Cellar is not upper case");
    h.get_by_label("Save Anyway").click();
    h.run_steps(3);
    assert!(h.query_by_label("Areas Not Yours").is_none(), "the claims are not asked again");
    assert!(!h.state().has_unsaved_work(), "saved");
}
