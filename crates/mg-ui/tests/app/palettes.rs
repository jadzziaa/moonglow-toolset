//! The palettes: categories, finding, favorites, the gallery, and
//! blueprints added to them and instances updated from them.

use crate::common::*;

/// Saving writes the custom palettes with their blueprints' names as the
/// game reads them: the bytes the names have in the module (an accent in
/// the game's codepage, a color token's bytes), not UTF-8, which the DM's
/// Creator in the game showed as "HipÃ³lito".
#[test]
fn a_saved_module_s_custom_palette_has_names_in_the_game_s_bytes() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-palette-bytes");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let name: &[u8] = b"Hip\xf3lito <c\xfe\x81\x01>rojo</c>";
    let key = ResKey::parse("mg_book", ResType::UTI).unwrap();
    {
        let game = app.game.as_deref().unwrap();
        let torch = game.resman.get_named("nw_it_torch001", ResType::UTI).unwrap();
        let mut g = Gff::read(&torch).unwrap();
        let named = mg_core::LocString::from_text(Language::ENGLISH, mg_core::Gender::Male, name);
        g.root.set("LocalizedName", mg_gff::Value::LocString(named));
        g.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
        let data = g.to_bytes().unwrap();
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "book",
            vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
        )));
    }
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    // The file on disk, read apart from the toolset's own palette reader.
    let saved = Module::open(&path).unwrap();
    let palette = saved.gff(&ResKey::parse("itempalcus", ResType::ITP).unwrap()).unwrap().unwrap();
    fn leaves(list: &[mg_gff::Struct], out: &mut Vec<(String, Vec<u8>)>) {
        for node in list {
            if let Some(r) = node.resref("RESREF") {
                out.push((r.to_string(), node.string("NAME").unwrap_or_default().to_vec()));
            }
            leaves(node.list("LIST").unwrap_or(&[]), out);
        }
    }
    let mut found = Vec::new();
    leaves(palette.root.list("MAIN").unwrap(), &mut found);
    let book = found.iter().find(|(r, _)| r == "mg_book").expect("the book in the palette");
    assert_eq!(book.1, name, "{:?}", String::from_utf8_lossy(&book.1));

    // A module saved by 1.19.3 has the names as UTF-8 in its palette:
    // opened and saved, with nothing changed, it keeps its palette as it
    // is (a save changes the entries of the blueprints changed alone, as
    // Aurora's does); Build's Palettes makes them right.
    let mut broken = Module::open(&path).unwrap();
    let palcus = ResKey::parse("itempalcus", ResType::ITP).unwrap();
    let mut palette = broken.gff(&palcus).unwrap().unwrap();
    fn spoil(list: &mut [mg_gff::Struct]) {
        for node in list {
            if node.resref("RESREF").is_some()
                && let Some(name) = node.string("NAME").map(<[u8]>::to_vec)
            {
                // (As 1.19.3 wrote them: the text, read as Windows-1252, in UTF-8.)
                let text = mg_core::Codepage::WINDOWS_1252.decode(&name).into_owned();
                node.set("NAME", mg_gff::Value::String(text.into_bytes()));
            }
            if let Some(mg_gff::Value::List(children)) = node.get_mut("LIST") {
                spoil(children);
            }
        }
    }
    if let Some(mg_gff::Value::List(main)) = palette.root.get_mut("MAIN") {
        spoil(main);
    }
    broken.set_gff(palcus, &palette).unwrap();
    broken.save().unwrap();
    let mut found = Vec::new();
    let spoiled = Module::open(&path).unwrap().gff(&palcus).unwrap().unwrap();
    leaves(spoiled.root.list("MAIN").unwrap(), &mut found);
    assert_ne!(found.iter().find(|(r, _)| r == "mg_book").unwrap().1, name, "spoiled");
    h.state_mut().open_module(&path);
    h.run();
    assert!(!h.state().ws.as_ref().unwrap().is_modified());
    let spoiled_bytes = Module::open(&path).unwrap().get(&palcus).unwrap().to_vec();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let mut saved = Module::open(&path).unwrap();
    assert_eq!(saved.get(&palcus).unwrap(), &spoiled_bytes[..], "a plain save leaves it");
    let game = h.state().game.clone().unwrap();
    mg_module::palette::rebuild_custom_palettes(&mut saved, &game).unwrap();
    let mut found = Vec::new();
    let repaired = saved.gff(&palcus).unwrap().unwrap();
    leaves(repaired.root.list("MAIN").unwrap(), &mut found);
    let book = found.iter().find(|(r, _)| r == "mg_book").expect("the book in the palette");
    assert_eq!(book.1, name, "built: {:?}", String::from_utf8_lossy(&book.1));
}

/// The custom palette shows what the module's palette lists, as Aurora's
/// does: the entries of the hak's blueprints (a talk-table name read from
/// the talk table), of the override folder's and of one there is no more;
/// such an entry is viewed, not edited. A save after a new blueprint adds
/// its entry and leaves every other as it was (El Grillo's and Jasperre's
/// report: entries of their haks' blueprints deleted, STRREFs turned into
/// names). The module is the Aurora palette probe's
/// (`crates/mg-corpus-tests/tests/aurora_palettes.rs`).
#[test]
fn the_custom_palette_keeps_the_entries_of_hak_blueprints() {
    let root = mg_testkit::corpus!();
    let probe = mg_testkit::aurora_capture!("palettes/before.mod");
    let hak = mg_testkit::aurora_capture!("palettes/mgpal.hak");
    let dir = mg_testkit::scratch_dir("ui-palette-haks");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    std::fs::copy(&hak, user.join("hak/mgpal.hak")).unwrap();
    let path = dir.join("palette_probe.mod");
    std::fs::copy(&probe, &path).unwrap();
    let palcus = ResKey::parse("itempalcus", ResType::ITP).unwrap();
    let before = Module::open(&path).unwrap().gff(&palcus).unwrap().unwrap();
    let mut app = game_app(&root, Some(user));
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Item;
    app.palette.tiles = false;
    app.palette.custom = true;
    app.palette.filter = "Drow".into();
    let mut h = harness(app, 900.0, 700.0);
    h.run_steps(4);
    // The hak's blueprint, named from the talk table (its entry's STRREF).
    // (The menu bar has a View and an Edit of its own.)
    let count = |h: &Harness<'_, Moonglow>, label: &str| h.query_all_by_label(label).count();
    let menus = ["View", "Edit", "Delete", "Edit Copy…"].map(|l| count(&h, l));
    h.get_by_label("Drow Full Plate +1").click_secondary();
    h.run_steps(4);
    let now = ["View", "Edit", "Delete", "Edit Copy…"].map(|l| count(&h, l));
    assert_eq!(now, [menus[0] + 1, menus[1], menus[2], menus[3] + 1], "viewed, not edited");
    h.key_press(egui::Key::Escape);
    h.run_steps(4);
    for (filter, shown) in [("Ghost", "Ghost Sword"), ("Override", "Override Sword")] {
        h.state_mut().palette.filter = filter.into();
        h.run_steps(4);
        assert!(h.query_by_label(shown).is_some(), "{shown} listed");
    }

    // A new blueprint, then Save: its entry added, every other as it was.
    {
        let game = h.state().game.clone().unwrap();
        let torch = game.resman.get_named("nw_it_torch001", ResType::UTI).unwrap();
        let mut g = Gff::read(&torch).unwrap();
        let named = mg_core::LocString::from_text(Language::ENGLISH, mg_core::Gender::Male, "Axe");
        g.root.set("LocalizedName", mg_gff::Value::LocString(named));
        g.root.set("PaletteID", mg_gff::Value::Byte(36));
        let key = ResKey::parse("mg_axe", ResType::UTI).unwrap();
        g.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
        let data = g.to_bytes().unwrap();
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "axe",
            vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
        )));
    }
    h.run_steps(4);
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(4);
    let mut saved = Module::open(&path).unwrap().gff(&palcus).unwrap().unwrap();
    fn take(list: &mut Vec<mg_gff::Struct>, resref: &str) -> Option<mg_gff::Struct> {
        let at =
            list.iter().position(|n| n.resref("RESREF").is_some_and(|r| r.to_string() == resref));
        match at {
            Some(i) => Some(list.remove(i)),
            None => list.iter_mut().find_map(|n| take(n.list_mut("LIST")?, resref)),
        }
    }
    let axe = take(saved.root.list_mut("MAIN").unwrap(), "mg_axe").expect("the axe's entry");
    assert_eq!(axe.string("NAME"), Some(&b"Axe"[..]));
    assert_eq!(saved, before, "every other entry as it was");
}

#[test]
fn palette_edit_copy_and_delete() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-palette");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    // The standard waypoints: the "Tavern" waypoint (nw_wp_tavern).
    let tavern = app.game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
    app.palette.tiles = false;
    let mut h = harness(app, 900.0, 700.0);
    h.run();
    // Seen collapsed first, as a builder opening the palette would; finding
    // it opens its categories.
    assert!(h.query_by_label(&tavern).is_none());
    h.state_mut().palette.filter = "nw_wp_tavern".into();
    h.run();
    h.get_by_label(&tavern).click_secondary();
    h.run();
    // View opens the game's own to look at: a change made there isn't kept
    // and the module doesn't get the blueprint.
    let stock = ResKey::parse("nw_wp_tavern", ResType::UTW).unwrap();
    // (The palette's View: the menu bar has a View too.)
    h.get_all_by_label("View").last().unwrap().click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Blueprint(stock)).is_some());
    assert!(h.query_by_label_contains("the game's blueprint").is_some());
    let set = mg_edit::Edit::SetField {
        key: stock,
        path: mg_edit::GffPath::root(),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(b"MINE".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![set])));
    h.run();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(!ws.module.contains(&stock) && !ws.is_modified(), "nothing of it is the module's");
    close_windows(&mut h);
    h.run();
    // Edit Copy asks for the copy's ResRef and Tag, offering a free one.
    h.get_by_label(&tavern).click_secondary();
    h.run();
    h.get_by_label("Edit Copy…").click();
    h.run();
    let draft = h.state().copy_as.clone().expect("the Copy window");
    assert_eq!(draft.resref, "nw_wp_tavern001");
    let tag = draft.tag.expect("a waypoint has a tag");
    h.state_mut().copy_as.as_mut().unwrap().tag = Some("MY_TAVERN".into());
    h.get_by_label("Create Copy").click();
    h.run();
    // A copy in the module, shown in the Custom palette.
    let copy = ResKey::parse("nw_wp_tavern001", ResType::UTW).unwrap();
    let gff = h.state_mut().ws.as_mut().unwrap().doc(&copy).unwrap().clone();
    assert_eq!(gff.root.resref("TemplateResRef").unwrap().to_string(), "nw_wp_tavern001");
    assert_eq!(gff.root.string("Tag"), Some(&b"MY_TAVERN"[..]), "not {tag}");
    assert!(h.state().palette.custom);
    // Custom palettes list blueprints under their own names.
    let name = mg_module::palette::blueprint_name(
        mg_module::palette::BlueprintKind::Waypoint,
        &gff.root,
        h.state().game.as_deref().unwrap(),
    );
    // (The copy's editor, opened in a window, closed first.)
    close_windows(&mut h);
    h.run();
    h.get_by_label(&name).click_secondary();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    assert!(!h.state().ws.as_ref().unwrap().module.contains(&copy));
    // Undo brings it back.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert!(h.state().ws.as_ref().unwrap().module.contains(&copy));
}

#[test]
fn palette_finds_by_tag_keeps_favorites_and_moves_between_categories() {
    use mg_edit::{Command, Edit};
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-palette-more");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let game = app.game.as_deref().unwrap();
    let base = Gff::read(&game.resman.get_named("nw_waypoint001", ResType::UTW).unwrap()).unwrap();
    // Gate A (tag MG_SECRET_GATE) in Waypoints, Gate B in Custom 1.
    let keys: Vec<ResKey> = ["mg_gate_a", "mg_gate_b"]
        .iter()
        .map(|n| ResKey::parse(n, ResType::UTW).unwrap())
        .collect();
    let edits = keys
        .iter()
        .zip([("Gate A", "MG_SECRET_GATE", 5u8), ("Gate B", "MG_PLAIN", 0)])
        .map(|(k, (name, tag, cat))| {
            let mut g = base.clone();
            g.root.set("TemplateResRef", mg_gff::Value::resref(k.resref));
            let ls = LocString::from_text(Language::ENGLISH, Gender::Male, name);
            g.root.set("LocalizedName", mg_gff::Value::LocString(ls));
            g.root.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
            g.root.set("PaletteID", mg_gff::Value::Byte(cat));
            Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
    app.palette.tiles = false;
    app.palette.custom = true;
    let mut h = harness(app, 900.0, 700.0);
    h.run();
    // Found by its tag, a word at a time.
    h.state_mut().palette.filter = "secret gate".into();
    h.run();
    h.get_by_label("Gate A");
    assert!(h.query_by_label("Gate B").is_none());
    // Letters in order, when nothing has the words.
    h.state_mut().palette.filter = "gtb".into();
    h.run();
    h.get_by_label("No exact matches: close ones");
    h.get_by_label("Gate B");
    assert!(h.query_by_label("Gate A").is_none());
    // A favorite, shown first.
    h.state_mut().palette.filter.clear();
    h.run();
    h.get_by_label("Gate A").click_secondary();
    h.run();
    h.get_by_label("Add to Favorites").click();
    h.run();
    assert_eq!(h.state().settings.palette_favorites, ["utw:mg_gate_a"]);
    h.get_by_label("Favorites (1)");
    // Dragged onto Custom 1, it moves there (one undoable step).
    // (Finding opened the categories; they stay open.)
    if h.query_by_label("Custom 1 (1)").is_none() {
        h.get_by_label("Special").click();
        h.run();
    }
    let from = h.get_all_by_label("★ Gate A").last().unwrap().rect().center();
    let to = h.get_by_label("Custom 1 (1)").rect().center();
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(10.0, 4.0));
    h.run();
    h.hover_at(to);
    h.run();
    h.drop_at(to);
    h.run();
    h.run();
    let category = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&keys[0]).unwrap().root.integer("PaletteID")
    };
    assert_eq!(category(&mut h), Some(0));
    h.get_by_label("Custom 1 (2)");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(category(&mut h), Some(5));
}

/// GitHub issue 16: the palette's header. The types are one row of icons
/// (a dropdown in a pane too narrow for it), the one shown named under
/// it; the search says what it looks in and has a button to clear it; the
/// list's toolbar counts its blueprints and keeps the rarer commands
/// under "more".
#[test]
fn the_palette_s_header() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-palette-header");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, 1800.0, 900.0);
    h.run_steps(3);
    // The row of icons: a click on the door shows the doors, named.
    // (All of it inside the palettes' pane, at the width it has at first.)
    let pane = {
        let mut leaves = h.state().dock.iter_leaves().map(|(_, l)| l);
        leaves.find(|l| l.tabs.contains(&Tab::Palette)).expect("the palettes' pane").rect
    };
    let last = h.get_by_label("🗐").rect();
    assert!(last.right() <= pane.right() && last.left() > pane.left(), "{last:?} in {pane:?}");
    h.get_by_label("🚪").click();
    h.run_steps(2);
    assert_eq!(h.state().palette.kind, mg_module::palette::BlueprintKind::Door);
    h.get_by_label("Doors");
    // How many the list has; a search that finds none, and cleared.
    let count = |h: &Harness<'_, Moonglow>| -> usize {
        let counts = h.query_all_by_label_contains(" item").filter_map(|n| {
            let node = n.accesskit_node();
            let text = node.value().or(node.label()).unwrap_or_default();
            text.split(' ').next()?.parse().ok()
        });
        { counts }.next().expect("the count")
    };
    let all = count(&h);
    assert!(all > 10, "{all} doors");
    assert!(h.query_by_label("×").is_none());
    type_into_hint(&mut h, "Find", "zzzzqq");
    h.run_steps(2);
    assert!(count(&h) < 5);
    h.get_by_label("×").click();
    h.run_steps(2);
    assert!(h.state().palette.filter.is_empty());
    assert_eq!(count(&h), all);
    // The rarer commands are under "more".
    assert!(h.query_by_label("Categories…").is_none());
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…");
}

/// The arrow keys step through a blueprint's palette category too, as
/// through any list of choices (the list once had them alone).
#[test]
fn arrow_keys_step_through_a_palette_category() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "category_arrows", ResType::UTP)
    else {
        return;
    };
    h.run();
    let category = |h: &mut Harness<'_, Moonglow>| field(h, &key).integer("PaletteID");
    let before = category(&mut h);
    // The Category dropdown: the one beside its label.
    let line = h.get_by_label("Category").rect().center().y;
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .map(|n| n.rect().center())
        .find(|c| (c.y - line).abs() < 6.0)
        .expect("the Category dropdown");
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run();
    // (The chest's may be the last category: then Up is the one to try.)
    let (mut step, mut back) = (egui::Key::ArrowDown, egui::Key::ArrowUp);
    h.key_press(step);
    h.run();
    if category(&mut h) == before {
        (step, back) = (back, step);
        h.key_press(step);
        h.run();
    }
    assert_ne!(category(&mut h), before, "another category, by {step:?}");
    h.key_press(back);
    h.run();
    assert_eq!(category(&mut h), before, "and back, by {back:?}");
}

/// Palette › Categories…: a category of the module's own, which its
/// blueprints can then be given; one with blueprints in it isn't removed.
#[test]
fn a_module_gets_palette_categories_of_its_own() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_cat", ResType::UTP) else {
        return;
    };
    h.run();
    let skeleton = BlueprintKind::Placeable.skeleton_key();
    let own = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&skeleton).ok().map(mg_module::palette::Palette::read)
    };
    assert!(own(&mut h).is_none(), "the game's categories, to begin with");
    {
        let app = h.state_mut();
        // (The chest's editor, a window, would lie over the palette.)
        close_tab(app, &Tab::Blueprint(key));
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
        app.palette.kind = BlueprintKind::Placeable;
        app.palette.tiles = false;
        app.palette.custom = true;
    }
    h.run();
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…").click();
    h.run();
    h.get_by_label("The game's categories. A change gives the module categories of its own.");
    // Nothing to keep yet.
    type_into_hint(&mut h, "a category's or group's name", "Ruins");
    h.get_by_label("Add Category").click();
    h.run();
    let ruins = h.state().palette_categories.as_ref().unwrap().selected.clone().expect("chosen");
    // With a row chosen and the pointer resting on it, the wheel still
    // scrolls the list.
    let top_of = |h: &Harness<'_, Moonglow>, label: &str| {
        h.query_all_by_label_contains("  (")
            .find(|n| n.accesskit_node().label().is_some_and(|l| l == label))
            .map(|n| n.rect().top())
    };
    let (third, third_rect) = {
        let mut rows: Vec<(String, egui::Rect)> = h
            .query_all_by_label_contains("  (")
            .filter_map(|n| Some((n.accesskit_node().label()?, n.rect())))
            .collect();
        rows.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        rows[2].clone()
    };
    let chosen = h.state().palette_categories.as_ref().unwrap().selected.clone();
    h.hover_at(third_rect.center());
    h.event(egui::Event::PointerButton {
        pos: third_rect.center(),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: third_rect.center(),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    // (Long enough for a tooltip, had the row one.)
    h.run_steps(90);
    assert_ne!(h.state().palette_categories.as_ref().unwrap().selected, chosen, "chosen");
    let was = top_of(&h, &third).unwrap();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -120.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(30);
    let now = top_of(&h, &third).unwrap();
    assert!(now < was - 40.0, "the list scrolled: {was} to {now}");
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 400.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(30);
    // (The category added last is chosen again, for what follows.)
    h.state_mut().palette_categories.as_mut().unwrap().selected = chosen;
    h.run();

    // A row dragged onto the upper half of the first goes before it.
    let rows = |h: &Harness<'_, Moonglow>| -> Vec<(String, egui::Rect)> {
        let mut rows: Vec<(String, egui::Rect)> = h
            .query_all_by_label_contains("  (")
            .filter_map(|n| Some((n.accesskit_node().label()?, n.rect())))
            .collect();
        rows.sort_by(|a, b| a.1.top().total_cmp(&b.1.top()));
        rows
    };
    let before = rows(&h);
    // (The second of the list: the new category is at its end, out of
    // sight until the list is scrolled.)
    let (moved, dragged) = before[1].clone();
    let first = before[0].1;
    let button = egui::PointerButton::Primary;
    let modifiers = egui::Modifiers::NONE;
    let (from, to) = (dragged.center(), first.center() - egui::vec2(0.0, first.height() * 0.3));
    h.hover_at(from);
    h.run();
    h.event(egui::Event::PointerButton { pos: from, button, pressed: true, modifiers });
    h.run_steps(2);
    // (Held near the list's edge, a dragged row keeps it scrolling: frames
    // are counted, not waited out.)
    for k in 1..=4 {
        h.hover_at(from + (to - from) * (k as f32 / 4.0));
        h.run_steps(2);
    }
    h.event(egui::Event::PointerButton { pos: to, button, pressed: false, modifiers });
    h.run_steps(3);

    // Dragged to the list's lower edge and held there, a row scrolls the
    // list: what was out of sight below comes up.
    let seen = |h: &Harness<'_, Moonglow>| top_of(h, &third).unwrap();
    let (top, grab) = {
        let r = rows(&h);
        (seen(&h), r[1].1.center())
    };
    let edge = egui::pos2(grab.x, grab.y + 400.0);
    h.hover_at(grab);
    h.run_steps(2);
    h.event(egui::Event::PointerButton { pos: grab, button, pressed: true, modifiers });
    h.run_steps(2);
    for k in 1..=4 {
        h.hover_at(grab + (edge - grab) * (k as f32 / 4.0));
        h.run_steps(2);
    }
    h.run_steps(20);
    assert!(seen(&h) < top - 60.0, "the list scrolled as the row was held at its edge");
    // Still held, away from the edges, the wheel scrolls the list too.
    let middle = egui::pos2(grab.x, grab.y + 150.0);
    h.hover_at(middle);
    h.run_steps(3);
    let scrolled = seen(&h);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 200.0),
        phase: egui::TouchPhase::Move,
        modifiers,
    });
    h.run_steps(20);
    assert!(seen(&h) > scrolled + 40.0, "the wheel scrolls the list while a row is held");
    // (Let go where nothing takes it: the order stays.)
    h.key_press(egui::Key::Escape);
    h.event(egui::Event::PointerButton { pos: middle, button, pressed: false, modifiers });
    h.run_steps(3);
    let after = rows(&h);
    assert_eq!(after[0].0, moved, "moved to the top");
    assert_eq!(after[1].0, before[0].0);
    let first_category = moved.split("  (").next().unwrap().to_string();
    h.get_by_label("OK").click();
    h.run();
    let palette = own(&mut h).expect("the module has its own skeleton now");
    let game = h.state().game.clone().unwrap();
    let categories = palette.categories(&game);
    let id = categories.iter().find(|(_, name)| name == "Ruins").expect("the new category").0;
    assert_eq!(categories[0].1, first_category, "the Category lists follow the order given");
    assert!(categories.iter().any(|(_, name)| name.ends_with("Custom 1")), "the game's stay");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Palette categories"));

    // The chest, given the category: listed under it in the custom palette.
    let edit = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: "PaletteID".into(),
        value: Some(mg_gff::Value::Byte(id)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Category", vec![edit])));
    h.run();
    assert!(h.query_by_label("Ruins (1)").is_some(), "the custom palette has the category");

    // With a blueprint in it, it isn't removed.
    h.get_by_label("More").click();
    h.run();
    h.get_by_label("Categories…").click();
    h.run();
    h.get_by_label("This module's own categories, kept in the module.");
    {
        let w = h.state_mut().palette_categories.as_mut().unwrap();
        w.selected = Some(ruins);
    }
    h.run();
    h.get_by_label("Remove").click();
    h.run();
    let w = h.state().palette_categories.as_ref().unwrap();
    assert!(
        w.error.as_deref().is_some_and(|e| e.starts_with("1 of the module's blueprint is in it"))
    );
    h.get_by_label("Cancel").click();
    h.run();
    assert!(own(&mut h).unwrap().ids().contains(&id));
}

#[test]
fn add_to_palette_create_waypoint_and_set() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("palette-add") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let items = |r: ResRef| {
            Gff::read(&game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?).ok().map(|g| g.root)
        };
        let placing = Placing { game, item: &items };
        let mut edits = Vec::new();
        for (name, t, x) in
            [("plc_chest1", ResType::UTP, 12.0), ("nw_bandit001", ResType::UTC, 28.0)]
        {
            let at = Placement { position: [x, 30.0, 0.0], rotation: 0.0 };
            let item = instance(&placing, t, &read(name, t), at, &[]).unwrap();
            let (list, _) = mg_module::instances::git_list(t).unwrap();
            edits.push(mg_edit::Edit::InsertItem {
                key: git_key,
                path: mg_edit::GffPath::root(),
                list: list.into(),
                index: 0,
                item,
            });
        }
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    h.run_steps(3);
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
    let git = |h: &mut Harness<'_, Moonglow>| -> mg_gff::Struct {
        h.state_mut().ws.as_mut().unwrap().doc(&git_key).unwrap().root.clone()
    };

    // Add to Palette on the chest: a new blueprint, which the chest names.
    right_click(&mut h, Vec3::new(12.0, 30.0, 0.3));
    h.get_by_label("Add to Palette").click();
    h.run_steps(3);
    let chest = git(&mut h).list("Placeable List").unwrap()[0].clone();
    let new = chest.resref("TemplateResRef").unwrap();
    assert!(new.to_string().starts_with("plc_chest"), "{new}");
    let key = ResKey::new(new, ResType::UTP);
    assert!(h.state().ws.as_ref().unwrap().module.contains(&key), "the new blueprint");
    assert!(h.state().dock.find_tab(&Tab::Blueprint(key)).is_some(), "its editor opens");

    // Create Waypoint on the bandit, where the menu was opened.
    close_windows(&mut h);
    h.run_steps(3);
    right_click(&mut h, Vec3::new(28.0, 30.0, 0.9));
    h.get_by_label("Create Waypoint").click();
    h.run_steps(3);
    let tag = String::from_utf8_lossy(
        git(&mut h).list("Creature List").unwrap()[0].string("Tag").unwrap(),
    )
    .into_owned();
    let waypoints = git(&mut h).list("WaypointList").unwrap().to_vec();
    assert_eq!(waypoints.len(), 3);
    assert_eq!(waypoints[2].string("Tag").unwrap(), format!("WP_{tag}_01").as_bytes());

    // Create Set on the first two waypoints: Patrol_01, Patrol_02.
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Waypoint, 0), (ObjectKind::Waypoint, 1)];
    right_click(&mut h, Vec3::new(20.0, 20.0, 0.9));
    h.get_by_label("Create Set…").click();
    h.run_steps(2);
    type_into_hint(&mut h, "set name", "Patrol");
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let waypoints = git(&mut h).list("WaypointList").unwrap().to_vec();
    assert_eq!(waypoints[0].string("Tag"), Some(&b"Patrol_01"[..]));
    assert_eq!(waypoints[1].string("Tag"), Some(&b"Patrol_02"[..]));
}

#[test]
fn update_instances_remakes_placed_sounds() {
    use mg_edit::{Command, Edit, GffPath};
    let Some((mut h, area)) = area_harness("update-instances") else { return };
    // A custom sound blueprint (a copy of hawkcry) placed twice, then its
    // volume changed.
    let game = h.state().game.as_deref().unwrap();
    let data = game.resman.get(&ResKey::parse("hawkcry", ResType::UTS).unwrap()).unwrap();
    let mut bp = Gff::read(&data).unwrap();
    let key = ResKey::parse("mg_hawk", ResType::UTS).unwrap();
    bp.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
    let none = |_: ResRef| None;
    let placing = mg_module::instances::Placing { game, item: &none };
    let placed: Vec<mg_gff::Struct> = [[10.0, 10.0], [30.0, 20.0]]
        .map(|p| {
            let at = mg_module::instances::Placement { position: [p[0], p[1], 1.5], rotation: 0.0 };
            mg_module::instances::instance(&placing, ResType::UTS, &bp.root, at, &[]).unwrap()
        })
        .to_vec();
    let bytes = bp.to_bytes().unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.apply(Command::new(
        "setup",
        vec![
            Edit::SetResource { key, data: Some(bytes) },
            Edit::SetField {
                key: git,
                path: GffPath::root(),
                label: "SoundList".into(),
                value: Some(mg_gff::Value::List(placed)),
            },
            Edit::SetField {
                key,
                path: GffPath::root(),
                label: "Volume".into(),
                value: Some(mg_gff::Value::Byte(17)),
            },
        ],
    ))
    .unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::for_resource(key).unwrap()));
    h.run_steps(3);
    h.get_by_label("Advanced").click();
    h.run_steps(2);
    h.get_by_label("Update Instances").click();
    h.run_steps(3);
    // The window lists both, every area chosen.
    let draft = h.state().update_draft.clone().expect("the Update Instances window");
    assert_eq!(draft.objects.len(), 2);
    h.get_by_label("Update 2").click();
    h.run_steps(3);
    assert!(h.state().update_draft.is_none());
    let ws = h.state_mut().ws.as_mut().unwrap();
    let sounds = ws.doc(&git).unwrap().root.list("SoundList").unwrap().to_vec();
    assert_eq!(sounds.len(), 2);
    for (s, p) in sounds.iter().zip([[10.0, 10.0], [30.0, 20.0]]) {
        assert_eq!(s.integer("Volume"), Some(17), "made again from the blueprint");
        assert_eq!(
            (s.float("XPosition"), s.float("YPosition")),
            (Some(p[0]), Some(p[1])),
            "where it stood"
        );
    }
}

#[test]
fn palette_updates_the_instances_of_a_selection_and_a_category() {
    use mg_edit::{Command, Edit};
    use mg_module::instances::{Placement, Placing, instance};
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-bulk-update");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    // Two trigger blueprints, each placed once with an outline.
    let game = app.game.as_deref().unwrap();
    let base = Gff::read(&game.resman.get_named("trackstrigger", ResType::UTT).unwrap()).unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game, item: &none };
    let mut edits = Vec::new();
    let mut placed = Vec::new();
    for (i, name) in ["mg_trig_a", "mg_trig_b"].into_iter().enumerate() {
        let key = ResKey::parse(name, ResType::UTT).unwrap();
        let mut bp = base.clone();
        bp.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
        let at = Placement { position: [10.0 + 10.0 * i as f32, 10.0, 0.0], rotation: 0.0 };
        let outline = [[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 3.0, 0.0]];
        placed.push(instance(&placing, ResType::UTT, &bp.root, at, &outline).unwrap());
        // The blueprint changes after placing.
        bp.root.set("Tag", mg_gff::Value::String(format!("NEW_{i}").into_bytes()));
        edits.push(Edit::SetResource { key, data: Some(bp.to_bytes().unwrap()) });
    }
    let mut git = Gff::new(*b"GIT ");
    git.root.set("TriggerList", mg_gff::Value::List(placed));
    let git_key = ResKey::parse("start", ResType::GIT).unwrap();
    edits.push(Edit::SetResource { key: git_key, data: Some(git.to_bytes().unwrap()) });
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Trigger;
    app.palette.tiles = false;
    app.palette.custom = true;
    app.palette.filter = "mg_trig".into();
    let mut h = harness(app, 900.0, 700.0);
    h.run();
    let triggers = |h: &mut Harness<'_, Moonglow>| -> Vec<mg_gff::Struct> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list("TriggerList").unwrap().to_vec()
    };
    let tag = |s: &mg_gff::Struct| String::from_utf8_lossy(s.string("Tag").unwrap()).into_owned();
    let before = triggers(&mut h);
    // Both chosen (Ctrl+click), the second object unticked: one updated.
    // The blueprints' name in the palette, and their category's title.
    let (a, title) = {
        let app = h.state();
        let game = app.game.as_deref().unwrap();
        let kind = mg_module::palette::BlueprintKind::Trigger;
        let gff = mg_module::palette::rebuild_custom_palette(
            &app.ws.as_ref().unwrap().module,
            game,
            kind,
        )
        .unwrap();
        let palette = mg_module::palette::Palette::read(&gff);
        fn find(
            nodes: &[mg_module::palette::PaletteNode],
        ) -> Option<&mg_module::palette::PaletteNode> {
            nodes.iter().find_map(|n| {
                if n.blueprints.iter().any(|b| b.resref.to_string() == "mg_trig_a") {
                    Some(n)
                } else {
                    find(&n.children)
                }
            })
        }
        let category = find(&palette.nodes).unwrap();
        let names: Vec<String> = category.blueprints.iter().map(|b| b.name.text(game)).collect();
        assert_eq!(names[0], names[1], "copies of one blueprint share its name");
        let title = format!("{} ({})", category.name.text(game), category.blueprints.len());
        (names[0].clone(), title)
    };
    let buttons: Vec<_> = h.get_all_by_label(&a).collect();
    assert_eq!(buttons.len(), 2);
    buttons[0].click();
    h.run();
    let buttons: Vec<_> = h.get_all_by_label(&a).collect();
    buttons[1].click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    assert_eq!(h.state().palette.chosen.len(), 1);
    h.get_all_by_label(&a).next().unwrap().click_secondary();
    h.run();
    h.get_by_label("Update Instances of 2").click();
    h.run();
    let draft = h.state().update_draft.clone().expect("the window");
    assert_eq!(draft.objects.len(), 2);
    h.get_all_by_label_contains("(mg_trig_b)").next().unwrap().click();
    h.run();
    h.get_by_label("Update 1").click();
    h.run();
    let after = triggers(&mut h);
    assert_eq!((tag(&after[0]), tag(&after[1])), ("NEW_0".to_string(), tag(&before[1])));
    // The log says which were updated: the one ticked, by its area, kind,
    // tag and blueprint; not the one left out.
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    assert!(log.iter().any(|l| l == "Updated 1 object(s) from 2 blueprint(s):"), "{log:?}");
    let listed: Vec<&String> = log.iter().filter(|l| l.starts_with("  ")).collect();
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert!(listed[0].contains("trigger") && listed[0].contains("(mg_trig_a)"), "{listed:?}");
    // Its outline and place are kept.
    assert_eq!(after[0].get("Geometry"), before[0].get("Geometry"));
    assert_eq!(after[0].float("XPosition"), Some(10.0));
    // The whole category, from its menu.
    h.get_by_label(&title).click_secondary();
    h.run();
    h.get_by_label("Update Instances").click();
    h.run();
    h.get_by_label("Update 2").click();
    h.run();
    let after = triggers(&mut h);
    assert_eq!((tag(&after[0]), tag(&after[1])), ("NEW_0".to_string(), "NEW_1".to_string()));
    // One undo step.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(tag(&triggers(&mut h)[1]), tag(&before[1]));
}

/// A palette's row is one line however narrow the pane: a name too long
/// for it is cut short, not wrapped onto a second line (which made the
/// list hard to read), and the pointer over it shows the whole.
#[test]
fn a_palette_row_too_long_for_the_pane_stays_one_line() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, _)) = area_harness("palette-one-line") else { return };
    h.state_mut().palette.kind = BlueprintKind::Waypoint;
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.custom = false;
    // (ResRefs beside the names: longer than the pane is wide.)
    h.state_mut().settings.name_resrefs = true;
    h.run_steps(3);
    h.get_by_label_contains("(13)").click();
    h.run_steps(4);
    let rows: Vec<egui::Rect> =
        h.query_all_by_label_contains("Beholder AI Exit").map(|n| n.rect()).collect();
    assert_eq!(rows.len(), 2, "both waypoints of the name are listed");
    let line = h.get_by_label_contains("Waypoints (13)").rect().height();
    for r in &rows {
        assert!(r.height() <= line + 2.0, "one line: {r:?} against {line}");
    }
    // One under the other, a line apart.
    assert!((rows[1].top() - rows[0].top()) < line * 1.5, "{rows:?}");
}

/// The arrow keys move through the palette's tree after a click in it, as
/// in Aurora: Up and Down through its rows, Left to the category a
/// blueprint is in and then closing it, Right opening it and going in.
/// They do not move the camera meanwhile, until the pointer is back in
/// the area's view.
#[test]
fn the_arrow_keys_move_through_the_palette_after_a_click_in_it() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, area)) = area_harness("palette-arrows") else { return };
    h.state_mut().palette.kind = BlueprintKind::Waypoint;
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.custom = false;
    h.run_steps(3);
    let target = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap().target;
    let selected = |h: &Harness<'_, Moonglow>| h.state().palette.selected;
    let press = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(4);
    };
    // The pointer in the area's view: the arrows are its camera's.
    let centre = screen(&h, area, glam::Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(centre);
    h.run_steps(2);
    // The standard waypoints are one category: clicked, it opens, and the
    // arrow keys are the palette's.
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    assert!(h.query_by_label(&tavern).is_none());
    h.get_by_label_contains("(13)").click();
    h.run_steps(4);
    h.get_by_label(&tavern);
    assert_eq!(selected(&h), None);
    let start = target(&h);
    // Down: into the category, a blueprint at a time, each selected.
    press(&mut h, egui::Key::ArrowDown);
    let first = selected(&h).expect("the first waypoint");
    press(&mut h, egui::Key::ArrowDown);
    let second = selected(&h).expect("the second");
    assert_ne!(first, second);
    press(&mut h, egui::Key::ArrowUp);
    assert_eq!(selected(&h), Some(first));
    // Enter opens the blueprint, as a double click does.
    let tabs: Vec<Tab> = h.state().dock.iter_all_tabs().map(|(_, t)| t.clone()).collect();
    press(&mut h, egui::Key::Enter);
    let opened: Vec<Tab> = h
        .state()
        .dock
        .iter_all_tabs()
        .map(|(_, t)| t.clone())
        .filter(|t| !tabs.contains(t))
        .collect();
    assert_eq!(opened.len(), 1, "{opened:?}");
    let at = h.state().dock.find_tab(&opened[0]).unwrap();
    h.state_mut().dock.remove_tab(at);
    h.run_steps(3);
    // Right does nothing on a blueprint. Left: to its category (the
    // blueprint stays in hand), then the category closes.
    press(&mut h, egui::Key::ArrowRight);
    assert_eq!(selected(&h), Some(first));
    press(&mut h, egui::Key::ArrowLeft);
    assert!(h.state().palette.at_category());
    assert_eq!(selected(&h), Some(first));
    h.get_by_label(&tavern);
    press(&mut h, egui::Key::ArrowLeft);
    assert!(h.query_by_label(&tavern).is_none(), "closed");
    // Enter on a category opens and closes it.
    press(&mut h, egui::Key::Enter);
    h.get_by_label(&tavern);
    press(&mut h, egui::Key::Enter);
    assert!(h.query_by_label(&tavern).is_none(), "closed by Enter");
    // Right: it opens, then the cursor goes in.
    press(&mut h, egui::Key::ArrowRight);
    h.get_by_label(&tavern);
    h.state_mut().palette.selected = None;
    press(&mut h, egui::Key::ArrowRight);
    assert_eq!(selected(&h), Some(first));
    // A blueprint clicked, then Left (as a builder described Aurora's): to
    // its category.
    h.get_by_label(&tavern).click();
    h.run_steps(3);
    let tavern_key = ResKey::parse("nw_wp_tavern", ResType::UTW);
    assert_eq!(selected(&h), tavern_key);
    assert!(!h.state().palette.at_category());
    press(&mut h, egui::Key::ArrowLeft);
    assert!(h.state().palette.at_category());
    assert_eq!(selected(&h), tavern_key, "still in hand");
    // With the pointer elsewhere (the log, under the area), the keys are
    // the palette's still: they are until the pointer is in an area's
    // view (GitHub issue 12).
    let palette_at = h.get_by_label(&tavern).rect().center();
    h.hover_at(egui::pos2(300.0, 700.0));
    h.run_steps(2);
    press(&mut h, egui::Key::ArrowDown);
    assert!(!h.state().palette.at_category(), "the keys are the palette's off it too");
    press(&mut h, egui::Key::ArrowLeft);
    assert!(h.state().palette.at_category());
    h.hover_at(palette_at);
    h.run_steps(2);
    // None of that moved the camera, nor does an arrow held down.
    let hold = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        h.event(event(true));
        h.run_steps(4);
        h.event(event(false));
        h.run_steps(1);
    };
    hold(&mut h, egui::Key::ArrowUp);
    assert_eq!(target(&h), start, "the arrows are the palette's");
    // W still does (the pointer was in this area's view last).
    hold(&mut h, egui::Key::W);
    let moved = target(&h);
    assert!((moved - start).length() > 0.1, "W moves the view all along");
    // The pointer back in the area's view: the arrows are the camera's.
    h.hover_at(centre);
    h.run_steps(2);
    let was = selected(&h);
    hold(&mut h, egui::Key::ArrowUp);
    assert!((target(&h) - moved).length() > 0.1, "the arrow moves the view again");
    assert_eq!(selected(&h), was, "and the palette stays");
    // The search's keys, as the module tree's Filter has them (GitHub
    // issue 12): Ctrl+F from the list goes to it, Escape clears it, and
    // Down goes into the list at its first row.
    h.get_by_label(&tavern).click();
    h.run_steps(3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run_steps(3);
    assert!(h.ctx.egui_wants_keyboard_input(), "the search has the keyboard");
    h.event(egui::Event::Text("zzz".into()));
    h.run_steps(3);
    assert_eq!(h.state().palette.filter, "zzz");
    press(&mut h, egui::Key::Escape);
    assert_eq!(h.state().palette.filter, "", "Escape clears what is typed");
    assert!(h.ctx.egui_wants_keyboard_input(), "and stays in the search");
    press(&mut h, egui::Key::ArrowDown);
    assert!(!h.ctx.egui_wants_keyboard_input(), "Down leaves it for the list");
    press(&mut h, egui::Key::ArrowDown);
    assert_eq!(selected(&h), Some(first), "at the list's first row, then on");
    // Escape in an empty search goes back to the list too.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    h.run_steps(3);
    assert!(h.ctx.egui_wants_keyboard_input());
    press(&mut h, egui::Key::Escape);
    assert!(!h.ctx.egui_wants_keyboard_input());
    press(&mut h, egui::Key::ArrowDown);
    assert!(selected(&h).is_some_and(|s| s != first), "the keys are the list's again");
}

/// The palette's hover says something of a blueprint that has no model to
/// show: an encounter's creatures, here.
#[test]
fn the_palette_s_hover_tells_of_blueprints_without_a_model() {
    let Some(mut h) = game_harness("palette-about") else { return };
    {
        let app = h.state_mut();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
        app.palette.kind = mg_module::palette::BlueprintKind::Encounter;
        app.palette.tiles = false;
        app.palette.custom = false;
        app.palette.filter = "nw_verminbeet".into();
    }
    h.run();
    // The one blueprint found: the row under the search's.
    let rows: Vec<_> = h.query_all_by_role(egui::accesskit::Role::Button).collect();
    let row = rows
        .iter()
        .filter(|n| n.accesskit_node().label().is_some_and(|l| l.contains("Beetle")))
        .map(|n| n.rect().center())
        .next_back()
        .expect("the encounter's row");
    assert!(h.query_by_label_contains("Spawns ").is_none());
    h.hover_at(row);
    // (A tooltip waits a moment.)
    h.run_steps(60);
    assert!(h.query_by_label_contains("Spawns ").is_some(), "its creatures are named");
    assert!(h.query_by_label_contains(" creatures; ").is_some());
}

/// Prefabs are found where builders look: a button on the area's toolbar
/// when several objects are selected, and a Prefabs section in the palette
/// that lists them, places them and says how to make one.
#[test]
fn prefabs_are_offered_on_the_toolbar_and_in_the_palette() {
    use mg_area::ObjectKind::Waypoint;
    let Some((mut h, area)) = area_harness("prefab-found") else { return };
    let prefabs = mg_testkit::scratch_dir("ui-prefab-found");
    h.state_mut().prefab_dir = Some(prefabs.clone());
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    // The palette's Prefabs: none yet, and how to make one.
    palette_type(&mut h, "🗐", "Prefabs");
    h.run_steps(2);
    assert!(h.query_by_label_contains("No prefabs yet").is_some());
    assert!(h.query_by_label_contains("Save 2 as Prefab").is_none());

    // Two objects selected: the toolbar offers to save them.
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(Waypoint, 0), (Waypoint, 1)];
    h.run_steps(2);
    h.get_by_label("Save 2 as Prefab…").click();
    h.run_steps(2);
    let (_, clip) = h.state().prefab_save.clone().expect("the Save as Prefab window");
    assert_eq!(clip.objects.len(), 2);
    h.state_mut().prefab_save = None;
    // The palette's button does the same.
    h.get_by_label("Save Selection as Prefab…").click();
    h.run_steps(2);
    assert!(h.state().prefab_save.is_some());
    h.state_mut().prefab_save = None;
    h.state_mut().save_prefab("Camp", &clip).unwrap();
    h.run_steps(2);

    // Saved, it is listed in the palette; a click takes it up to place.
    assert!(h.query_by_label_contains("No prefabs yet").is_none());
    h.get_by_label("Camp").click();
    h.run_steps(3);
    assert!(h.state().area_views[&area].pasting, "it follows the pointer, to be placed");
    assert!(h.state().log.entries.iter().any(|e| e.1.contains("Prefab Camp: click in the area")));
    // Over the area, its objects show see-through where they would go.
    let over = screen(&h, area, glam::Vec3::new(12.0, 30.0, 0.0));
    h.hover_at(over);
    h.run_steps(3);
    assert_eq!(h.state().area_views[&area].pasted_shown, 2);
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-prefab-ghost").join("prefab_ghost.png")).unwrap();
    // What it holds shows while the pointer rests on it.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    let holds = mg_ui::prefabs::summary(Some(&prefabs), "Camp").unwrap();
    assert!(holds.starts_with("2 "), "{holds}");
    // Renamed from its menu.
    h.get_by_label("Camp").click_secondary();
    h.run_steps(3);
    h.get_by_label("Rename…").click();
    h.run_steps(3);
    let field = h.get_by(|n| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some("Camp")
    });
    field.type_text(" Site");
    h.run_steps(2);
    h.get_by_label("Rename").click();
    h.run_steps(3);
    assert!(prefabs.join("Camp Site.prefab.json").is_file(), "{:?}", h.state().log.entries);
    assert!(!prefabs.join("Camp.prefab.json").exists());
    // Deleted from the palette, and put back by Undo Delete.
    h.get_by_label("Camp Site").click_secondary();
    h.run_steps(3);
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert!(!prefabs.join("Camp Site.prefab.json").exists());
    assert!(h.query_by_label_contains("No prefabs yet").is_some());
    h.get_by_label("Undo Delete").click();
    h.run_steps(3);
    assert!(prefabs.join("Camp Site.prefab.json").is_file());
    assert!(h.query_by_label("Camp Site").is_some());
    assert!(h.query_by_label("Undo Delete").is_none());
}

#[test]
fn the_palette_opens_with_a_module_as_wide_as_the_tree() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-palette-opens");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, 1600.0, 900.0);
    h.run_steps(3);
    let palette_width = |h: &Harness<'_, Moonglow>| {
        let path = h.state().dock.find_tab(&Tab::Palette).expect("the palette is open");
        let pane = h.state().dock.iter_leaves().find(|(p, _)| p.node == path.node).unwrap().1;
        pane.rect.width()
    };
    // The tree's default width, 240 points (give or take the separators).
    let w = palette_width(&h);
    assert!((w - 240.0).abs() <= 12.0, "the palette is {w} points wide");
    // On the area's tiles.
    assert!(h.state().palette.tiles);
    // With every tab closed, the palette comes back as the only pane.
    close_tab(h.state_mut(), &Tab::ModuleProperties);
    close_tab(h.state_mut(), &Tab::Palette);
    h.run();
    h.get_by_label("📦 Palettes").click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Palette).is_some());
}

/// The palette shown last is the one the palettes open on at the next
/// start: its type is kept in the settings when chosen.
#[test]
fn the_palette_shown_last_is_remembered() {
    use mg_module::palette::BlueprintKind;
    let Some((mut h, _)) = plugin_harness("ui-palette-remembered", true) else { return };
    h.state_mut().open_palette = true;
    h.run_steps(3);
    assert_eq!(h.state().settings.palette_type, None);
    palette_type(&mut h, "🚪", "Doors");
    h.run_steps(2);
    assert_eq!(h.state().settings.palette_type.as_deref(), Some("utd"));
    // Another start with those settings: the doors again.
    let settings = h.state().settings.clone();
    let Some((mut again, _)) = plugin_harness("ui-palette-remembered-2", true) else { return };
    again.state_mut().settings = settings;
    again.state_mut().open_palette = true;
    again.run_steps(3);
    assert_eq!(again.state().palette.kind, BlueprintKind::Door);
    assert!(!again.state().palette.tiles);
}

/// A blueprint copied into an open module folder from elsewhere (one
/// another builder sent, copied in as with Aurora), read again: it shows
/// in its custom palette at once, and the palette changed is unsaved
/// work; changed there, its entry follows; the save writes the palette
/// with it in the category its `PaletteID` names; deleted, it leaves; the
/// module's other entry is as it was throughout (Troubletcat's report:
/// the item showed in the module's tree, not in the palette). One opened
/// with the module, the palette not listing it, is put in by More ›
/// Refresh Palette, which Undo takes back. `template`: one of the game's
/// blueprints, copied with another name.
fn a_blueprint_copied_into_the_folder_shows_in_its_palette(
    kind: mg_module::palette::BlueprintKind,
    template: &str,
    shown: &str,
) {
    use mg_gff::Value;
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir(&format!("ui-palette-outside-{}", kind.name()));
    let folder = dir.join("module");
    std::fs::create_dir_all(dir.join("user")).unwrap();
    let mut app = game_app(&root, Some(dir.join("user")));
    let game = app.game.clone().unwrap();
    let original = game.resman.get_named(template, kind.restype()).unwrap().to_vec();
    let category = Gff::read(&original).unwrap().root.integer(kind.palette_field()).unwrap();
    let blueprint = |resref: &str, name: &str| {
        let mut g = Gff::read(&original).unwrap();
        let named = LocString::from_text(Language::ENGLISH, Gender::Male, name);
        g.root.set(kind.name_field(), Value::LocString(named));
        g.root.set(kind.resref_field(), Value::resref(ResRef::from_str(resref).unwrap()));
        g.to_bytes().unwrap()
    };
    let file = |resref: &str| folder.join(format!("{resref}.{}", kind.restype()));
    // The module's palettes made (with a blueprint of its own), then a
    // blueprint put in while it is closed (its palette doesn't list it).
    let mut m = Module::open(&sample_module(&dir)).unwrap();
    let kept = format!("{shown} Kept");
    m.set(
        ResKey::new(ResRef::from_str("mg_kept").unwrap(), kind.restype()),
        blueprint("mg_kept", &kept),
    );
    mg_module::palette::rebuild_custom_palettes(&mut m, &game).unwrap();
    m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    let palcus = folder.join(format!("{}.itp", kind.custom_key().resref));
    fn entry_of(
        list: &[mg_gff::Struct],
        resref: &str,
        id: Option<i64>,
    ) -> Option<(Option<i64>, mg_gff::Struct)> {
        list.iter().find_map(|node| match node.resref("RESREF") {
            Some(r) => (r.to_string() == resref).then(|| (id, node.clone())),
            None => entry_of(node.list("LIST").unwrap_or(&[]), resref, node.integer("ID")),
        })
    }
    let entry = |resref: &str| {
        let g = Gff::read(&std::fs::read(&palcus).unwrap()).unwrap();
        entry_of(g.root.list("MAIN").unwrap(), resref, None)
    };
    let was = entry("mg_kept").expect("the module's own");
    assert_eq!(was.0, Some(category));
    let closed = format!("{shown} Closed");
    std::fs::write(file("mg_closed"), blueprint("mg_closed", &closed)).unwrap();
    // (Not held here: the window takes the game data to itself.)
    drop(game);
    app.settings.no_last_area = true;
    app.settings.no_mod_beside_folder = true;
    app.settings.palette_gallery = false;
    app.open_module(&folder);
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = kind;
    app.palette.tiles = false;
    app.palette.custom = true;
    app.palette.filter = shown.into();
    let mut h = harness(app, 900.0, 700.0);
    h.run_steps(4);
    // (A creature's row has its challenge rating after the name.)
    let listed = |h: &Harness<'_, Moonglow>, name: &str| {
        let cr = format!("{name}  (CR");
        (h.query_all_by_label_contains(name))
            .filter_map(|n| n.accesskit_node().label())
            .any(|l| l == name || l.starts_with(&cr))
    };
    let modified = |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().is_modified();
    // (The search typed again, which opens the categories with a match:
    // one that had none stays closed as the builder left it.)
    let search = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().palette.filter.clear();
        h.run_steps(2);
        h.state_mut().palette.filter = shown.into();
        h.run_steps(4);
    };
    let reload = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().reload_project_files();
        h.run_steps(4);
        search(h);
    };
    let save = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::Save);
        h.run_steps(4);
    };
    assert!(listed(&h, &kept));
    assert!(!listed(&h, shown) && !listed(&h, &closed));
    assert!(!modified(&h));

    // Copied in: listed after the reload.
    std::fs::write(file("mg_outside"), blueprint("mg_outside", shown)).unwrap();
    reload(&mut h);
    assert!(listed(&h, shown), "{shown} in the palette after the reload");
    assert!(modified(&h), "the palette changed is unsaved work");
    // Renamed there: its entry follows.
    let renamed = format!("{shown} Renamed");
    std::fs::write(file("mg_outside"), blueprint("mg_outside", &renamed)).unwrap();
    reload(&mut h);
    assert!(listed(&h, &renamed) && !listed(&h, shown), "renamed");
    // Saved: the file has its entry, in its category.
    save(&mut h);
    assert!(!modified(&h));
    let (id, saved) = entry("mg_outside").expect("written with the palette");
    assert_eq!(id, Some(category));
    let named = saved.string("NAME").map(|n| String::from_utf8_lossy(n).into_owned());
    assert_eq!(named.as_deref(), Some(renamed.as_str()));
    assert!(entry("mg_closed").is_none(), "not refreshed");
    assert_eq!(entry("mg_kept").as_ref(), Some(&was), "the other entry as it was");
    // Deleted there: it leaves.
    std::fs::remove_file(file("mg_outside")).unwrap();
    reload(&mut h);
    assert!(!listed(&h, &renamed), "deleted");
    assert!(listed(&h, &kept));
    save(&mut h);
    assert!(entry("mg_outside").is_none());
    assert_eq!(entry("mg_kept").as_ref(), Some(&was), "the other entry as it was");

    // More › Refresh Palette: the one put in while the module was closed
    // is listed; Undo takes it out again.
    h.get_by_label("More").click();
    h.run_steps(2);
    h.get_by_label("Refresh Palette").click();
    h.run_steps(4);
    search(&mut h);
    assert!(listed(&h, &closed), "refreshed");
    assert!(modified(&h), "refreshed: to be saved");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(4);
    search(&mut h);
    assert!(!listed(&h, &closed), "undone");
}

#[test]
fn an_item_copied_into_the_folder_shows_in_its_palette() {
    let kind = mg_module::palette::BlueprintKind::Item;
    a_blueprint_copied_into_the_folder_shows_in_its_palette(kind, "nw_wswls001", "Outside Sword");
}

#[test]
fn a_creature_copied_into_the_folder_shows_in_its_palette() {
    let kind = mg_module::palette::BlueprintKind::Creature;
    a_blueprint_copied_into_the_folder_shows_in_its_palette(kind, "nw_chicken", "Outside Hen");
}

#[test]
fn a_placeable_copied_into_the_folder_shows_in_its_palette() {
    let kind = mg_module::palette::BlueprintKind::Placeable;
    a_blueprint_copied_into_the_folder_shows_in_its_palette(kind, "plc_chest1", "Outside Chest");
}

/// The palette's Gallery shows a category's blueprints as pictures: a
/// click on one chooses it to place, as a click on its name does.
#[test]
fn the_palette_s_gallery_shows_blueprints_as_pictures() {
    let Some((mut h, _area)) = area_harness("gallery") else { return };
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Placeable;
        (p.tiles, p.custom) = (false, false);
        p.filter = "chest".into();
    }
    h.state_mut().settings.palette_gallery = true;
    // (A few are made a frame.)
    h.run_steps(30);
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let name = {
        let game = h.state().game.as_deref().unwrap();
        let utp = Gff::read(&game.resman.get(&chest).unwrap()).unwrap();
        game.locstring(utp.root.locstring("LocName").unwrap()).unwrap()
    };
    let tile = h.get_all_by_label(&name).next().expect("the chest's picture");
    let size = tile.rect().size();
    assert!(size.x > 80.0 && size.y > size.x, "a picture over a name: {size:?}");
    tile.click();
    h.run_steps(3);
    assert!(h.state().palette.selected.is_some(), "chosen, to place");
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-gallery").join("gallery.png")).unwrap();
    // List goes back to the names, and Gallery to the pictures.
    h.get_by_label("☰").click();
    h.run_steps(3);
    assert!(!h.state().settings.palette_gallery);
    let row = h.get_all_by_label(&name).next().expect("the chest's row").rect().size();
    assert!(row.y < 30.0, "a row of the list: {row:?}");
    h.get_by_label("⊞").click();
    h.run_steps(3);
    assert!(h.state().settings.palette_gallery);
}

/// A gallery's pictures are made a few a frame and kept while they are in
/// sight, however many: more of them than are kept out of sight aren't let
/// go and made again frame after frame. An edit that leaves the blueprints
/// looking as they did draws none of them anew; and those gone out of
/// sight go, down to how many are kept.
#[test]
fn a_gallery_s_pictures_are_made_once_and_kept_while_in_sight() {
    let Some((mut h, area)) = area_harness("gallery-kept") else { return };
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Placeable;
        (p.tiles, p.custom) = (false, false);
        p.filter = "a".into();
    }
    h.state_mut().settings.palette_gallery = true;
    h.state_mut().settings.gallery_tile = Some(64);
    // (Few kept out of sight, for the test: fewer than are in sight.)
    h.state_mut().thumbnails.keep = Some(4);
    let settle = |h: &mut Harness<'static, Moonglow>| {
        for _ in 0..400 {
            h.run_steps(1);
            if h.state().thumbnails.waiting() == 0 {
                break;
            }
        }
        h.run_steps(2);
        assert_eq!(h.state().thumbnails.waiting(), 0, "every picture in sight is made");
    };
    settle(&mut h);
    let (kept, drawn) = (h.state().thumbnails.kept(), h.state().thumbnails.drawn);
    assert!(kept > 4, "all those in sight are kept: {kept}");
    assert_eq!(drawn, kept as u64, "each drawn once");
    h.run_steps(10);
    assert_eq!(h.state().thumbnails.drawn, drawn, "none drawn again");
    assert_eq!(h.state().thumbnails.kept(), kept);
    // An edit elsewhere (a waypoint's tag): the pictures are looked at
    // again, found to be of what the blueprints still look like, and kept.
    let set = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::GIT),
        path: mg_edit::GffPath::root().item("WaypointList", 0),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String("moved".into())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![set])));
    h.run_steps(2);
    settle(&mut h);
    assert_eq!(h.state().thumbnails.drawn, drawn, "none drawn anew after an edit");
    // Other pictures in sight: the first ones, out of sight, go.
    h.state_mut().palette.filter = "chest".into();
    settle(&mut h);
    h.run_steps(3);
    let (now, more) = (h.state().thumbnails.kept(), h.state().thumbnails.drawn - drawn);
    assert!(more > 0, "the chests' pictures are drawn");
    assert!(
        (now as u64) < kept as u64 + more && now as u64 <= more + 4,
        "those out of sight are let go, but for four: {now} kept of {kept} and {more} more"
    );
}

/// Replace Selected with This, on a blueprint in the palette: the selected
/// object of its type becomes one of that blueprint, where it stands; the
/// others stay; one undo.
#[test]
fn the_palette_replaces_the_selected_object() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("replace") else { return };
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    {
        let app = h.state_mut();
        app.palette.kind = mg_module::palette::BlueprintKind::Waypoint;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.palette.filter = "nw_wp_tavern".into();
        app.area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    }
    h.run_steps(5);
    let read = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let w = &git.root.list("WaypointList").unwrap()[i];
        (
            w.resref("TemplateResRef").map(|r| r.to_string()),
            w.float("XPosition"),
            w.float("YPosition"),
        )
    };
    let (first, second) = (read(&mut h, 0), read(&mut h, 1));
    h.get_by_label(&tavern).click_secondary();
    h.run_steps(2);
    h.get_by_label("Replace Selected with This").click();
    h.run_steps(3);
    let now = read(&mut h, 0);
    assert_eq!(now.0.as_deref(), Some("nw_wp_tavern"));
    assert_eq!((now.1, now.2), (first.1, first.2), "where it stood");
    assert_eq!(read(&mut h, 1), second, "the other is untouched");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(3);
    assert_eq!(read(&mut h, 0), first, "one undo");
}

/// The Placeable Gallery (Tools, and the palette's All Appearances…) is
/// open without a placeable's Properties: a click on a picture gives its
/// appearance to the placeables selected in the area. Models that are
/// only an effect have pictures too.
#[test]
fn the_placeable_gallery_gives_the_selection_an_appearance() {
    use mg_area::ObjectKind;
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("placeable-gallery") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let chest = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [12.0, 20.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
        // (A small window, so some of the area shows beside it.)
        app.settings.window_sizes = vec![("placeable-gallery".into(), [330, 430])];
        app.actions.push(mg_ui::Action::PlaceableGallery);
    }
    h.run_steps(5);
    assert!(h.query_by_label_contains("Select placeables in an area").is_some());
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Placeable, 0)];
    h.state_mut().placeable_gallery.as_mut().unwrap().filter = "Flame".into();
    h.run_steps(40);
    assert!(h.query_by_label_contains("1 placeable selected in the area").is_some());
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-placeable-gallery").join("gallery.png")).unwrap();
    let appearance = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&git_key).unwrap();
        git.root.list("Placeable List").unwrap()[0].integer("Appearance").unwrap()
    };
    let was = appearance(&mut h);
    let tile = |n: &egui_kittest::Node<'_>| n.rect().height() > 90.0;
    let flame = h.get_all_by_label("Flame").find(tile).expect("a flame's picture");
    flame.click();
    h.run_steps(3);
    let now = appearance(&mut h);
    assert_ne!(now, was, "the chest is a flame");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Appearance"));
    // Dragged into the area's view (beside the gallery's window), a picture
    // places a placeable of that appearance there.
    let count = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap().len()
    };
    let from = h.get_all_by_label("Flame").find(tile).unwrap().rect().center();
    let window = h
        .state()
        .dock
        .iter_leaves()
        .find(|(p, _)| !p.surface.is_main())
        .map(|(_, leaf)| leaf.rect)
        .unwrap();
    // (In the area's pane, clear of its toolbar.)
    let at_area = h.state().dock.find_tab(&Tab::Area(area)).unwrap();
    let pane = h
        .state()
        .dock
        .iter_leaves()
        .find(|(p, _)| p.surface == at_area.surface && p.node == at_area.node)
        .map(|(_, leaf)| leaf.viewport)
        .unwrap();
    let pane = egui::Rect::from_min_max(
        pane.min + egui::vec2(10.0, 200.0),
        pane.max - egui::vec2(10.0, 10.0),
    );
    let spots = (0..=20).flat_map(|x| (0..=20).map(move |y| (x as f32 * 2.0, y as f32 * 2.0)));
    let to = spots
        .map(|(x, y)| screen(&h, area, glam::Vec3::new(x, y, 0.0)))
        .find(|p| !window.expand(8.0).contains(*p) && pane.contains(*p))
        .expect("ground beside the gallery");
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for k in 1..=6 {
        h.hover_at(from + (to - from) * (k as f32 / 6.0));
        h.run_steps(1);
    }
    // Over the area it shows, see-through, where it would go.
    let ghost = h.state().area_views[&area].ghost_shown.clone().expect("its ghost");
    assert_eq!(ghost.kind, ObjectKind::Placeable);
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(count(&mut h), 2, "a second placeable");
    let ws = h.state_mut().ws.as_mut().unwrap();
    let placed = ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[1].clone();
    assert_eq!(placed.integer("Appearance"), Some(now), "of the flame dragged");
    assert_eq!(placed.integer("Static"), Some(1));
}

/// The palette's pictures of items are kept by the body armor is shown on
/// (Options › General): on a woman, hers are made; on a man again, his are
/// there, not made again.
#[test]
fn the_palette_s_item_pictures_are_kept_by_who_wears_them() {
    let Some((mut h, _)) = blueprint_harness("nw_aarcl001", "gallery_gender", ResType::UTI) else {
        return;
    };
    {
        let app = h.state_mut();
        app.settings.palette_gallery = true;
        app.palette.kind = mg_module::palette::BlueprintKind::Item;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.palette.filter = "nw_aarcl001".into();
        app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    }
    h.run_steps(30);
    let drawn = |h: &Harness<'_, Moonglow>| h.state().thumbnails.drawn;
    let his = drawn(&h);
    assert!(his > 0, "the man's pictures");
    h.state_mut().settings.armor_on_woman = true;
    h.run_steps(30);
    let hers = drawn(&h);
    assert!(hers > his, "the woman's, made");
    h.state_mut().settings.armor_on_woman = false;
    h.run_steps(30);
    assert_eq!(drawn(&h), hers, "the man's, kept");
}
