//! The blueprint editors' pages (creatures, items, placeables, doors,
//! stores, sounds, triggers, encounters, waypoints) and the wizards that
//! make blueprints.

use crate::common::*;

/// A blueprint's Category list is in the palette's order, or in
/// alphabetical order when Options › General asks (a builder's report:
/// "the dropdown list is not ordered").
#[test]
fn a_blueprint_s_categories_are_listed_alphabetically_if_asked() {
    use mg_module::palette::{BlueprintKind, Palette};
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_category", ResType::UTP) else {
        return;
    };
    h.run();
    let names: Vec<String> = {
        let game = h.state().game.clone().unwrap();
        let skeleton = game.resman.get(&BlueprintKind::Placeable.skeleton_key()).unwrap();
        Palette::read(&Gff::read(&skeleton).unwrap()).categories(&game).into_iter()
    }
    .map(|(_, name)| name)
    .collect();
    let mut sorted = names.clone();
    sorted.sort_by_key(|n| n.to_lowercase());
    assert_ne!(names[0], sorted[0], "the game's palette isn't in alphabetical order");
    let id = field(&mut h, &key).integer("PaletteID").unwrap();
    let game = h.state().game.clone().unwrap();
    let skeleton = game.resman.get(&BlueprintKind::Placeable.skeleton_key()).unwrap();
    let all = Palette::read(&Gff::read(&skeleton).unwrap()).categories(&game);
    let current = all.iter().find(|(c, _)| i64::from(*c) == id).unwrap().1.clone();
    // The first row of the list, opened: the one highest up of those named.
    let first_row = |h: &mut Harness<'_, Moonglow>| {
        h.get_all_by_value(&current).last().expect("the Category list").click();
        h.run();
        let top = |name: &String| {
            h.query_all_by_label(name).map(|n| n.rect().top()).fold(f32::NAN, f32::max)
        };
        let first =
            names.iter().filter(|n| !top(n).is_nan()).min_by(|a, b| top(a).total_cmp(&top(b)));
        let first = first.unwrap().clone();
        h.key_press(egui::Key::Escape);
        h.run();
        first
    };
    assert_eq!(first_row(&mut h), names[0], "the palette's order");
    h.state_mut().settings.categories_by_name = true;
    h.run();
    assert_eq!(first_row(&mut h), sorted[0], "alphabetical");
}

/// Export Files, on a placeable's picture: the files the game draws it
/// with, written into a folder (a chest of the game's own: all of them).
#[test]
fn a_placeable_s_model_files_are_exported() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_files", ResType::UTP) else {
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-export-model-files");
    h.state_mut().dialogs =
        Box::new(NoDialogs { folders: vec![dir.clone()], ..Default::default() });
    h.run();
    // (The button of the model's picture: there is none without a GPU.)
    let source = mg_ui::model_view::Source::Resource(key);
    mg_ui::model_view::export_files(h.state_mut(), &source);
    let mut names: Vec<String> = (std::fs::read_dir(&dir).unwrap().flatten())
        .map(|e| e.file_name().into_string().unwrap())
        .collect();
    names.sort();
    let has = |ext: &str| names.iter().any(|n| n.ends_with(ext));
    assert!(has(".mdl") && has(".pwk"), "{names:?}");
    assert!(has(".dds") || has(".tga"), "{names:?}");
    let log: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    let said = log.iter().find(|m| m.starts_with("Exported ")).expect("said in the log");
    assert!(said.contains(&format!("{} files", names.len())) && said.contains(".mdl ("), "{said}");
    // (Every file is the game's own here: none left out.)
    assert!(!said.contains("left out"), "{said}");

    // Again into the same folder: it asks first. One file changed and one
    // gone: Skip Those writes the one gone and keeps the other.
    let model = names.iter().find(|n| n.ends_with(".mdl")).unwrap();
    let walk = names.iter().find(|n| n.ends_with(".pwk")).unwrap();
    std::fs::write(dir.join(model), b"mine").unwrap();
    std::fs::remove_file(dir.join(walk)).unwrap();
    h.state_mut().dialogs =
        Box::new(NoDialogs { folders: vec![dir.clone()], ..Default::default() });
    mg_ui::model_view::export_files(h.state_mut(), &source);
    h.run();
    assert_eq!(h.state().model_export.as_ref().unwrap().existing.len(), names.len() - 1);
    assert_eq!(std::fs::read(dir.join(model)).unwrap(), b"mine", "nothing written yet");
    h.get_by_label("Skip Those").click();
    h.run();
    assert!(h.state().model_export.is_none());
    assert_eq!(std::fs::read(dir.join(model)).unwrap(), b"mine");
    assert!(dir.join(walk).is_file());
    // Replace writes them all; Cancel none.
    for (button, replaced) in [("Cancel", false), ("Replace", true)] {
        let again = NoDialogs { folders: vec![dir.clone()], ..Default::default() };
        h.state_mut().dialogs = Box::new(again);
        mg_ui::model_view::export_files(h.state_mut(), &source);
        h.run();
        h.get_all_by_label_contains(button).last().unwrap().click();
        h.run();
        assert!(h.state().model_export.is_none());
        assert_eq!(std::fs::read(dir.join(model)).unwrap() != b"mine", replaced, "{button}");
    }
}

#[test]
fn waypoint_editor_edits_and_renames() {
    let Some((mut h, key)) = blueprint_harness("nw_waypoint001", "waypoint_copy", ResType::UTW)
    else {
        return;
    };
    h.run();
    // Tag (commits when focus leaves).
    let tag = String::from_utf8_lossy(field(&mut h, &key).string("Tag").unwrap()).into_owned();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(&tag).find(is_input).expect("the tag field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&tag).find(is_input).unwrap().type_text("WP_RENAMED");
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(field(&mut h, &key).string("Tag").unwrap(), b"WP_RENAMED");
    h.get_by_label("Advanced").click();
    h.run();
    // A map note.
    h.get_by_label("Waypoint Contains a Map Note").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("HasMapNote"), Some(1));
    // Rename: the tab follows.
    h.state_mut().actions.push(mg_ui::Action::RenameBlueprint {
        from: key,
        to: ResRef::from_str("my_waypoint").unwrap(),
    });
    h.run();
    let new = ResKey::parse("my_waypoint", ResType::UTW).unwrap();
    let ws = h.state().ws.as_ref().unwrap();
    assert!(ws.module.contains(&new) && !ws.module.contains(&key));
    assert!(h.state().dock.find_tab(&Tab::Blueprint(new)).is_some());
    assert_eq!(field(&mut h, &new).resref("TemplateResRef").unwrap().to_string(), "my_waypoint");
}

#[test]
fn sound_editor_lists_positions_and_times() {
    let Some((mut h, key)) = blueprint_harness("animalcriesday", "sound_copy", ResType::UTS) else {
        return;
    };
    h.run();
    let before = field(&mut h, &key).list("Sounds").map_or(0, <[_]>::len);
    // Add a sound through the picker.
    h.get_by_label("Add Sounds…").click();
    h.run();
    // One the blueprint does not list yet (its own list shows the others).
    h.state_mut().picker.as_mut().unwrap().filter = "as_cv_".into();
    h.run();
    h.get_all_by_label_contains("as_cv_").next().unwrap().click();
    h.run();
    let sounds = field(&mut h, &key).list("Sounds").unwrap().len();
    assert_eq!(sounds, before + 1);
    // Positioning: a random position sets both fields.
    h.get_by_label("Positioning").click();
    h.run();
    h.get_by_label("Plays from a random position each time it is played").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!((s.integer("Positional"), s.integer("RandomPosition")), (Some(1), Some(1)));
    // Specific hours.
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("Specific Hours").click();
    h.run();
    h.get_by_label("3 PM").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!(s.integer("Times"), Some(0));
    assert_ne!(s.integer("Hours").unwrap() & (1 << 15), 0);
}

/// A creature made with a single model keeps no body parts: given a
/// part-based appearance it gets them in the same step (the game draws no
/// body without them, and no head), and one that has parts keeps its own.
#[test]
fn a_part_based_appearance_brings_a_body() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_body", ResType::UTC) else {
        return;
    };
    h.run();
    assert_eq!(field(&mut h, &key).integer("BodyPart_Torso"), None, "the bandit keeps none");
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    // The Appearance dropdown: the one beside Gallery….
    let line = h.get_by_label("Gallery…").rect().center().y;
    let at = h
        .query_all(combo)
        .map(|n| n.rect().center())
        .find(|c| (c.y - line).abs() < 6.0)
        .expect("the Appearance dropdown");
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run();
    type_into_hint(&mut h, "Filter", "Human");
    // (Typed in, the list is at its top again: the row is in sight.)
    let row = h.get_by_label("Human").rect().center();
    assert!((row.y - line).abs() < 120.0, "the row is at {row:?}");
    h.hover_at(row);
    press(&h, row, true, egui::Modifiers::NONE);
    press(&h, row, false, egui::Modifiers::NONE);
    h.run();
    let c = field(&mut h, &key);
    assert_eq!(c.integer("Appearance_Type"), Some(6));
    assert_eq!(c.integer("BodyPart_Torso"), Some(1));
    assert_eq!(c.integer("ArmorPart_RFoot"), Some(1));
    assert_eq!(c.integer("Appearance_Head"), Some(1));
    assert_eq!(c.integer("BodyPart_Belt"), Some(0), "a bare body has no belt");
    // One step: Undo takes the parts away with the appearance.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    let c = field(&mut h, &key);
    assert_ne!(c.integer("Appearance_Type"), Some(6));
    assert_eq!(c.integer("BodyPart_Torso"), None);
}

/// An item property's sub-properties are a long list like any other: Cast
/// Spell's hundreds of spells open at the one chosen, with a Filter field
/// to narrow them by name.
#[test]
fn an_item_property_s_spells_have_a_filter() {
    let Some((mut h, key)) = blueprint_harness("nw_it_mpotion001", "potion_filter", ResType::UTI)
    else {
        return;
    };
    let subtype = |h: &mut Harness<'_, Moonglow>| {
        field(h, &key).list("PropertiesList").unwrap()[0].integer("Subtype")
    };
    h.run();
    h.get_by_label("Properties").click();
    h.run();
    // The potion's Cast Spell, chosen: its parameters show under it.
    h.get_all_by_label_contains("Cure Light Wounds").last().unwrap().click();
    h.run();
    let before = subtype(&mut h);
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.contains("Cure Light Wounds")))
        .map(|n| n.rect().center())
        .expect("the Sub-Property dropdown");
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run();
    // Opened at the chosen spell (far down the list: Aid, Barkskin… come
    // first), under the Filter field.
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
    let top = h.get(egui_kittest::kittest::by().predicate(hint)).rect().bottom();
    let row = (h.query_all_by_label_contains("Cure Light Wounds"))
        .map(|n| n.rect().center().y)
        .find(|y| *y > top)
        .expect("the chosen spell's row");
    assert!(row - top < 260.0, "the chosen spell is {} under the Filter field", row - top);
    // (A potion's spells: those iprp_spells allows on a potion.)
    assert!(h.query_all_by_label_contains("Barkskin").count() > 0, "the list is whole");
    type_into_hint(&mut h, "Filter", "bull's");
    assert!(h.query_all_by_label_contains("Bull's Strength").count() > 0, "the list is narrowed");
    assert!(h.query_all_by_label_contains("Barkskin").count() == 0, "to the spells that match");
    h.get_all_by_label_contains("Bull's Strength").next().unwrap().click();
    h.run();
    let after = subtype(&mut h);
    assert_ne!(after, before, "Bull's Strength chosen");
    let base = field(&mut h, &key).integer("BaseItem").unwrap() as u32;
    let game = h.state().game.clone().unwrap();
    let spells = game.property_subtypes(&game.property_type(15).unwrap(), base);
    let name = mg_rules::choice_text(&spells, after.unwrap());
    assert!(name.contains("Bull's Strength"), "{name}");
    assert!(h.query(egui_kittest::kittest::by().predicate(hint)).is_none(), "closed");
}

/// Hovering a skill shows what the game says of it (Aurora's F1).
#[test]
fn a_skill_s_description_shows_on_hover() {
    let Some((mut h, _)) = blueprint_harness("nw_bartender", "skill_help", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Skills").click();
    h.run();
    assert!(h.query_by_label_contains("opposed test").is_none());
    let hide = h.get_by_label("Hide").rect().center();
    h.hover_at(hide);
    // (A tooltip waits a moment.)
    h.run_steps(60);
    assert!(
        h.query_by_label_contains("Spot check").is_some(),
        "the game's description of Hide is shown"
    );
}

#[test]
fn trigger_editor_sets_the_type_and_trap() {
    let Some((mut h, key)) = blueprint_harness("trackstrigger", "trigger_copy", ResType::UTT)
    else {
        return;
    };
    h.run();
    assert_eq!(field(&mut h, &key).integer("Type"), Some(0));
    // Area transition settings apply to transitions only.
    h.get_by_label("Area Transition").click();
    h.run();
    assert!(h.query_by_label_contains("apply to Area Transition triggers").is_some());
    h.get_by_label("Basic").click();
    h.run();
    // Trigger Type › Trap (the option in the combo's popup, after the tab).
    h.get_by_value("Generic").click();
    h.run();
    h.get_all_by_label("Trap").last().unwrap().click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Type"), Some(2));
    h.get_all_by_label("Trap").next().unwrap().click();
    h.run();
    let trapped = field(&mut h, &key).integer("TrapFlag").unwrap_or(0);
    h.get_by_label("Is Trapped").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("TrapFlag"), Some(1 - trapped));
}

#[test]
fn encounter_editor_lists_creatures_and_respawns() {
    let Some((mut h, key)) = blueprint_harness("nw_verminbeet", "encounter_copy", ResType::UTE)
    else {
        return;
    };
    h.run();
    let before = field(&mut h, &key).list("CreatureList").map_or(0, <[_]>::len);
    h.get_by_label("Creature List").click();
    h.run();
    // Find a badger in the standard palette and add it.
    type_into_hint(&mut h, "Find", "nw_badger");
    let badger = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button
            && n.label().is_some_and(|l| l.starts_with("Badger"))
    };
    h.get(egui_kittest::kittest::by().predicate(badger)).click();
    h.run();
    h.get_by_label("Add Creature").click();
    h.run();
    let list = field(&mut h, &key).list("CreatureList").unwrap().to_vec();
    assert_eq!(list.len(), before + 1);
    let added = list.last().unwrap();
    assert_eq!(added.resref("ResRef").unwrap().to_string(), "nw_badger");
    assert!(added.float("CR").unwrap() > 0.0);
    // Remove it again.
    h.get_by_label("Badger").click();
    h.run();
    h.get_by_label("Remove Creature").click();
    h.run();
    assert_eq!(field(&mut h, &key).list("CreatureList").unwrap().len(), before);
    // Infinite respawns. Showing the page changes nothing, though the
    // blueprint's Respawns (0) is outside the field's range.
    let respawns = field(&mut h, &key).integer("Respawns");
    h.get_by_label("Advanced").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Respawns"), respawns);
    assert_ne!(h.state().ws.as_ref().unwrap().can_undo(), Some("Respawns"));
    if field(&mut h, &key).integer("Reset") != Some(1) {
        h.get_by_label("Encounter Respawns").click();
        h.run();
    }
    // Infinite is −1; turning it off leaves one respawn.
    let infinite = field(&mut h, &key).integer("Respawns") == Some(-1);
    h.get_by_label("Infinite Respawn").click();
    h.run();
    let expected = if infinite { 1 } else { -1 };
    assert_eq!(field(&mut h, &key).integer("Respawns"), Some(expected));
}

#[test]
fn store_editor_stocks_prices_and_restricts() {
    let Some((mut h, key)) = blueprint_harness("nw_storebar01", "store_copy", ResType::UTM) else {
        return;
    };
    h.run();
    // Limited gold (a store without the field has unlimited gold).
    assert!(matches!(field(&mut h, &key).integer("StoreGold"), None | Some(-1)));
    h.get_by_label("Has Limited Gold").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("StoreGold"), Some(1000));
    // A longsword goes on the weapons page, at the first free place.
    h.get_by_label("Inventory…").click();
    h.run();
    let weapons = |s: &mg_gff::Struct| {
        s.list("StoreList")
            .unwrap()
            .iter()
            .find(|p| p.id == 4)
            .and_then(|p| p.list("ItemList"))
            .map_or(Vec::new(), <[_]>::to_vec)
    };
    let before = weapons(&field(&mut h, &key));
    type_into_hint(&mut h, "Find", "nw_wswls001");
    let sword = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button && n.label().is_some_and(|l| l == "Longsword")
    };
    h.get(egui_kittest::kittest::by().predicate(sword)).click();
    h.run();
    // The chosen item's statistics show before it is added.
    h.get_by_label("Chosen in the palette");
    h.get_by_label("Damage");
    h.get_by_label("1d8");
    h.get_by_label("19-20 / x2");
    h.get_by_label("Add Item").click();
    h.run();
    let after = weapons(&field(&mut h, &key));
    assert_eq!(after.len(), before.len() + 1);
    let added = after.last().unwrap();
    assert_eq!(added.resref("InventoryRes").unwrap().to_string(), "nw_wswls001");
    let at = (added.integer("Repos_PosX").unwrap(), added.integer("Repos_Posy").unwrap());
    let at_before: Vec<_> = before
        .iter()
        .map(|i| (i.integer("Repos_PosX").unwrap(), i.integer("Repos_Posy").unwrap()))
        .collect();
    assert!(!at_before.contains(&at), "{at:?} is taken");
    // Each item is listed with what the store sells it for: its cost at
    // the store's mark up, as Aurora lists it.
    let price = {
        let mark_up = field(&mut h, &key).integer("MarkUp").unwrap() as u64;
        let game = h.state().game.as_deref().unwrap();
        let uti = game.resman.get_named("nw_wswls001", ResType::UTI).unwrap();
        let item = mg_gff::Gff::read(&uti).unwrap();
        u64::from(game.item_cost(&mg_rules::ItemValue::from_gff(&item.root))) * mark_up / 100
    };
    assert!(price > 0);
    let listed = format!("Longsword  ·  {price} gp");
    assert!(h.query_all_by_label_contains(&listed).next().is_some(), "{listed}");
    // The store will not buy torches, then will only buy them.
    h.get_by_label("Restrictions").click();
    h.run();
    type_into_hint(&mut h, "Find", "Torch");
    h.get_by_label("Torch").click();
    h.run();
    h.get_by_label("Add").click();
    h.run();
    let bases = |s: &mg_gff::Struct, list: &str| -> Vec<i64> {
        s.list(list).unwrap_or(&[]).iter().filter_map(|i| i.integer("BaseItem")).collect()
    };
    assert_eq!(bases(&field(&mut h, &key), "WillNotBuy"), [15]);
    h.get_by_label("Store will ONLY buy the following items").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!((bases(&s, "WillNotBuy"), bases(&s, "WillOnlyBuy")), (vec![], vec![15]));
    assert_eq!(s.list("WillOnlyBuy").unwrap()[0].id, 97869);
}

/// A store page of hundreds of items lays out the rows in sight only (a
/// page of thousands took seconds to show); scrolled to its end, the
/// last row is there, and its menu's Remove takes that item out.
#[test]
fn a_long_store_page_lays_out_the_rows_in_sight() {
    use mg_gff::{Struct, Value};
    let Some((mut h, key)) = blueprint_harness("nw_storebar01", "long_store", ResType::UTM) else {
        return;
    };
    h.run();
    let entry = |i: usize, item: &str| {
        let mut s = Struct::new(i as u32);
        s.set("InventoryRes", Value::resref(ResRef::from_str(item).unwrap()));
        s.set("Repos_PosX", Value::Word((i % 10) as u16));
        s.set("Repos_Posy", Value::Word((i / 10) as u16));
        s.set("Infinite", Value::Byte(0));
        s
    };
    let mut items: Vec<Struct> = (0..299).map(|i| entry(i, "nw_wswls001")).collect();
    items.push(entry(299, "nw_wswss001"));
    let mut page = Struct::new(4);
    page.set("ItemList", Value::List(items));
    let set = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: "StoreList".into(),
        value: Some(Value::List(vec![page])),
    };
    h.state_mut().ws.as_mut().unwrap().apply(mg_edit::Command::new("stock", vec![set])).unwrap();
    h.get_by_label("Inventory…").click();
    h.run();
    let rows = |h: &Harness<'_, Moonglow>| h.query_all_by_label("Remove").count();
    let shown = rows(&h);
    assert!((3..40).contains(&shown), "{shown} rows laid out of 300");
    let last = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button
            && n.label().is_some_and(|l| l.contains("  ·  ") && !l.starts_with("Longsword"))
    };
    assert!(h.query(egui_kittest::kittest::by().predicate(last)).is_none());
    // The wheel over the list, to its end.
    let at = h.get_all_by_label("Remove").next().unwrap().rect().center() - egui::vec2(200.0, 0.0);
    h.hover_at(at);
    for _ in 0..20 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -100_000.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(10);
    }
    assert!(rows(&h) < 40, "{} rows laid out", rows(&h));
    h.get(egui_kittest::kittest::by().predicate(last)).click_secondary();
    h.run();
    h.get_all_by_label("Remove").last().unwrap().click();
    h.run();
    let items =
        field(&mut h, &key).list("StoreList").unwrap()[0].list("ItemList").unwrap().to_vec();
    assert_eq!(items.len(), 299);
    assert!(items.iter().all(|i| i.resref("InventoryRes").unwrap().to_string() == "nw_wswls001"));
}

#[test]
fn showing_blueprint_editors_changes_nothing() {
    // Every page of every editor, on blueprints with values outside the
    // fields' ranges (Respawns 0, missing fields, generic doors).
    for (i, (name, t)) in [
        ("nw_waypoint001", ResType::UTW),
        ("animalcriesday", ResType::UTS),
        ("x0_trapavg_shuri", ResType::UTT),
        ("nw_verminbeet", ResType::UTE),
        ("nw_storebar01", ResType::UTM),
        ("x2_storethief003", ResType::UTM),
        ("nw_door_ttr_01", ResType::UTD),
        ("plc_chest1", ResType::UTP),
        ("nw_wswls001", ResType::UTI),
        ("nw_aarcl001", ResType::UTI),
        ("nw_it_mpotion001", ResType::UTI),
        ("nw_maarcl002", ResType::UTI),
        ("nw_bandit001", ResType::UTC),
        ("nw_bartender", ResType::UTC),
        ("nw_badger", ResType::UTC),
        ("db_tanarukk_do", ResType::UTC),
        ("nw_bandit004", ResType::UTC),
    ]
    .into_iter()
    .enumerate()
    {
        let Some((mut h, key)) = blueprint_harness(name, &format!("copy{i}"), t) else {
            return;
        };
        for page in mg_ui::blueprint::pages(t) {
            h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), page);
            h.run();
        }
        let ws = h.state().ws.as_ref().unwrap();
        assert_eq!(ws.can_undo(), Some("copy"), "{name}");
    }
}

#[test]
fn door_editor_sets_appearance_lock_and_transition() {
    let Some((mut h, key)) = blueprint_harness("nw_door_ttr_01", "door_copy", ResType::UTD) else {
        return;
    };
    h.run();
    // A generic door: its Generic Appearance (and the old byte field).
    let generic = {
        let game = h.state().game.as_deref().unwrap();
        let cols = mg_rules::ChoiceColumns { name: Some("Name"), label: Some("Label") };
        game.choices("genericdoors", cols).unwrap()
    };
    assert_eq!(field(&mut h, &key).integer("Appearance"), Some(0));
    h.get_by_value(&generic[0].text).click();
    h.run();
    // (A long list, by name: the row scrolled to.)
    h.get_by_label(&generic[1].text).scroll_to_me();
    h.run();
    h.get_by_label(&generic[1].text).click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!(s.integer("GenericType_New"), Some(generic[1].row as i64));
    assert_eq!(s.integer("GenericType"), Some(generic[1].row as i64));
    // Lock.
    h.get_by_label("Lock").click();
    h.run();
    h.get_by_label("Locked").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Locked"), Some(1));
    // Area transition to a waypoint.
    h.get_by_label("Area Transition").click();
    h.run();
    h.get_by_label("Waypoint").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("LinkedToFlags"), Some(2));
    // No Interrupt is Interruptable 0.
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("No Interrupt").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Interruptable"), Some(0));
}

/// A placeable given an appearance whose model is nowhere is told so under
/// its Appearance Type, naming the model and the table's row (a builder's
/// report: the preview was empty, and nothing said why; Aurora's area view
/// often crashes on one), as is a reserved row (`USER`); one with a model
/// is not.
#[test]
fn a_placeable_s_appearance_without_a_model_is_said() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    // (With a GPU: the preview beside the fields says it too.)
    let Some(rs) = gpu() else { return };
    let dir = mg_testkit::scratch_dir("ui-bp-no-model");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.set_render_state(rs.clone());
    // The game's placeables.2da in the module, with a row more: a model
    // no one has.
    let (chest, row) = {
        let game = app.game.as_deref().unwrap();
        let mut table = game.resman.get_named("placeables", ResType::TWODA).unwrap().into_owned();
        let row = game.table("placeables").unwrap().len();
        if !table.ends_with(b"\n") {
            table.extend_from_slice(b"\r\n");
        }
        table.extend_from_slice(format!("{row} Missing_Thing **** plc_x99\r\n").as_bytes());
        let mut m = Module::open(&path).unwrap();
        m.set(ResKey::parse("placeables", ResType::TWODA).unwrap(), table);
        m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        (game.resman.get_named("plc_chest1", ResType::UTP).unwrap().into_owned(), row)
    };
    app.open_module(&path);
    let key = ResKey::parse("my_crate", ResType::UTP).unwrap();
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "copy",
        vec![mg_edit::Edit::SetResource { key, data: Some(chest) }],
    )));
    app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    close_tab(&mut app, &Tab::ModuleProperties);
    app.open_palette = false;
    let mut h = gpu_harness(app, rs, 1100.0, 700.0);
    h.run_steps(3);
    let shots = mg_testkit::scratch_dir("ui-bp-no-model-shot");
    h.render().expect("render").save(shots.join("chest.png")).unwrap();
    let warning = "No model: ";
    assert!(h.query_by_label_contains(warning).is_none(), "the chest has its model");

    // Chosen in the Appearance Type dropdown (it shows "Chest").
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let at = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("Chest")))
        .map(|n| n.rect().center())
        .expect("the Appearance Type dropdown");
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    // (Typed into its Filter: frames are stepped, as the preview plays.)
    let filter = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
    h.get(egui_kittest::kittest::by().predicate(filter)).click();
    h.run_steps(2);
    h.get(egui_kittest::kittest::by().predicate(filter)).type_text("Missing_Thing");
    h.run_steps(2);
    let choice = h.get_by_label("Missing_Thing").rect().center();
    h.hover_at(choice);
    press(&h, choice, true, egui::Modifiers::NONE);
    press(&h, choice, false, egui::Modifiers::NONE);
    h.run_steps(3);
    h.render().expect("render").save(shots.join("no-model.png")).unwrap();
    assert_eq!(field(&mut h, &key).integer("Appearance"), Some(row as i64));
    let said = format!(
        "No model: plc_x99 (placeables.2da row {row}, ModelName) isn't in the module, its haks \
         or the game"
    );
    assert!(h.query_by_label(&said).is_some(), "the warning under the field");
    let pane = format!("my_crate.utp: {said}");
    assert!(h.query_by_label(&pane).is_some(), "the preview says it, not the chest");

    // A reserved row (the game's 1000 on are USER), then the chest again.
    let set = |h: &mut Harness<'_, Moonglow>, v: u32| {
        let edit = mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "Appearance".into(),
            value: Some(mg_gff::Value::Dword(v)),
        };
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Set", vec![edit])));
        h.run_steps(3);
    };
    set(&mut h, 1000);
    let reserved = "No model: placeables.2da row 1000 is reserved (its ModelName is USER)";
    assert!(h.query_by_label(reserved).is_some(), "a reserved row is said");
    set(&mut h, 1);
    assert!(h.query_by_label_contains(warning).is_none(), "a row with a model says nothing");
    let pane = h.query_by_label_contains("my_crate.utp: No model");
    assert!(pane.is_none(), "the preview shows the model again");
    // The Appearance Gallery marks the row (a picture to look at).
    h.get_by_label("Gallery…").click();
    h.run_steps(3);
    h.state_mut().placeable_gallery.as_mut().unwrap().filter = "Missing_Thing".into();
    h.run_steps(10);
    h.render().expect("render").save(shots.join("gallery.png")).unwrap();
    let tile = |n: &egui_kittest::Node<'_>| n.rect().height() > 90.0;
    assert!(h.get_all_by_label("Missing_Thing").any(|n| tile(&n)), "its picture in the gallery");
}

/// A placeable's illumination (`LightState`, which Aurora doesn't show;
/// Jasperre's reports): a brazier without the field is lit, and keeps no
/// field until the box is unticked (0) or ticked again (1), one undoable
/// change each. A chest, whose appearance has no light, shows it on too
/// and can be unticked (its row may gain a light later).
#[test]
fn a_placeable_s_illumination_is_written_only_when_changed() {
    let Some((mut h, key)) = blueprint_harness("plc_brazier", "brazier_light", ResType::UTP) else {
        return;
    };
    h.run();
    h.get_by_label("Advanced").click();
    h.run();
    let state =
        |h: &Harness<'_, Moonglow>| h.get_by_label("Illuminated").accesskit_node().toggled();
    assert_eq!(state(&h), Some(egui::accesskit::Toggled::True));
    assert!(!h.get_by_label("Illuminated").accesskit_node().is_disabled());
    assert_eq!(field(&mut h, &key).get("LightState"), None, "shown, not written");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("copy"));
    h.get_by_label("Illuminated").click();
    h.run();
    assert_eq!(field(&mut h, &key).get("LightState"), Some(&mg_gff::Value::Byte(0)));
    assert_eq!(state(&h), Some(egui::accesskit::Toggled::False));
    h.get_by_label("Illuminated").click();
    h.run();
    assert_eq!(field(&mut h, &key).get("LightState"), Some(&mg_gff::Value::Byte(1)));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(field(&mut h, &key).get("LightState"), None, "undone, the field is gone");

    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_light", ResType::UTP) else {
        return;
    };
    h.run();
    h.get_by_label("Advanced").click();
    h.run();
    let node = h.get_by_label("Illuminated");
    assert!(!node.accesskit_node().is_disabled());
    assert_eq!(node.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    assert_eq!(field(&mut h, &key).get("LightState"), None, "shown, not written");
    h.get_by_label("Illuminated").click();
    h.run();
    assert_eq!(field(&mut h, &key).get("LightState"), Some(&mg_gff::Value::Byte(0)));
}

/// The PC faction is the players' alone (Jasperre's report; the game puts
/// a creature of it in Hostile): an object's faction list doesn't offer
/// it. A creature whose file has it keeps it, shown as such, until
/// another is picked.
#[test]
fn an_object_s_factions_leave_out_the_pc_faction() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "pc_faction", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Advanced").click();
    h.run();
    let shown = field(&mut h, &key).integer("FactionID").unwrap();
    let name = mg_module::factions::STANDARD[shown as usize];
    h.get_by_value(name).click();
    h.run();
    for other in &mg_module::factions::STANDARD[1..] {
        assert!(h.query_by_label(other).is_some(), "{other} is offered");
    }
    assert!(h.query_by_label("PC").is_none(), "PC is not offered");
    h.key_press(egui::Key::Escape);
    h.run();

    // A file with faction 0: kept, and shown as the PC faction.
    let mut g = Gff::new(*b"UTC ");
    g.root = field(&mut h, &key);
    g.root.set("FactionID", mg_gff::Value::Word(0));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "pc",
        vec![mg_edit::Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) }],
    )));
    h.run();
    assert_eq!(field(&mut h, &key).integer("FactionID"), Some(0));
    h.get_by_value("PC (not for objects)").click();
    h.run();
    assert!(h.query_by_label("PC").is_none(), "PC is not offered");
    h.get_by_label("Commoner").click();
    h.run();
    assert_eq!(field(&mut h, &key).get("FactionID"), Some(&mg_gff::Value::Word(2)));
    assert!(h.query_by_value("PC (not for objects)").is_none());
}

#[test]
fn visuals_page_sets_replacements_and_misc_visuals() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_visuals", ResType::UTP) else {
        return;
    };
    h.run();
    h.get_by_label("Visuals").click();
    h.run();
    // A texture replacement, added from the empty row.
    type_into_hint(&mut h, "Texture", "plc_chest1");
    type_into_hint(&mut h, "Drawn as", "mg_gold");
    h.get_all_by_label("Add").next().unwrap().click();
    h.run();
    let tex = |h: &mut Harness<'_, Moonglow>| -> Vec<(String, String)> {
        let root = field(h, &key);
        let list = root.child("TextureReplace").and_then(|s| s.list("TextureReplaceLi"));
        list.unwrap_or(&[])
            .iter()
            .map(|e| {
                let r = |l: &str| e.resref(l).unwrap().to_string();
                (r("OldTexture"), r("NewTexture"))
            })
            .collect()
    };
    assert_eq!(tex(&mut h), [("plc_chest1".to_string(), "mg_gold".to_string())]);
    assert_eq!(field(&mut h, &key).child("TextureReplace").unwrap().id, 9, "the game's struct id");
    // The area view and model viewer draw it.
    let chest = field(&mut h, &key);
    let game = h.state().game.as_deref().unwrap();
    let preview = mg_preview::replaced(mg_preview::placeable(game, &chest).unwrap(), &chest);
    assert_eq!(preview.base.textures.get("plc_chest1").map(String::as_str), Some("mg_gold"));
    // Reset: the field goes.
    h.get_all_by_label("Reset").next().unwrap().click();
    h.run();
    assert!(field(&mut h, &key).get("TextureReplace").is_none());
    // MiscVisuals: what shows, set apart from the game's choice; the other
    // fields are left to the game.
    h.get_by_label("As the game decides").click();
    h.run();
    h.get_by_label("Name with Tab").click();
    h.run();
    let misc = field(&mut h, &key).child("MiscVisuals").cloned().unwrap();
    assert_eq!(misc.integer("UiDiscoverMask"), Some(15 & !8));
    assert_eq!(misc.id, 8);
    assert!(misc.get("VisibleDistance").is_none());
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(
        field(&mut h, &key).child("MiscVisuals").unwrap().integer("UiDiscoverMask"),
        Some(15)
    );
}

/// A replacement: old, new, and whether the entry kept its extra field.
type Entry = (String, String, bool);

/// An object's replacements as the Visuals page writes them: its list's
/// entries (old, new) and the extra field a test puts on the list's
/// struct and on each entry.
fn replacements(
    h: &mut Harness<'_, Moonglow>,
    key: &ResKey,
    (outer, list, old, new): (&str, &str, &str, &str),
) -> Option<(Vec<Entry>, bool)> {
    let root = field(h, key);
    let s = root.child(outer)?;
    let items = s.list(list).unwrap_or(&[]);
    let pairs = items
        .iter()
        .map(|e| {
            let r = |l: &str| e.resref(l).map(|r| r.to_string()).unwrap_or_default();
            (r(old), r(new), e.get("MgKept").is_some())
        })
        .collect();
    Some((pairs, s.get("MgKept").is_some()))
}

const TEXTURES: (&str, &str, &str, &str) =
    ("TextureReplace", "TextureReplaceLi", "OldTexture", "NewTexture");

/// Puts a replacement list on the blueprint: one entry `old` → `new`, the
/// list's struct and the entry each with a field of their own (kept).
fn put_replacement(
    h: &mut Harness<'_, Moonglow>,
    key: ResKey,
    (outer, list, old_label, new_label): (&str, &str, &str, &str),
    id: u32,
    (old, new): (&str, &str),
) {
    use mg_gff::{Struct, Value};
    let mut e = Struct::new(0);
    e.set(old_label, Value::resref(ResRef::from_str(old).unwrap()));
    e.set(new_label, Value::resref(ResRef::from_str(new).unwrap()));
    e.set("MgKept", Value::Int(1));
    let mut s = Struct::new(id);
    s.set(list, Value::List(vec![e]));
    s.set("MgKept", Value::Int(1));
    let mut g = Gff::new(*b"UTP ");
    g.root = field(h, &key);
    g.root.set(outer, Value::Struct(s));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "replacement",
        vec![mg_edit::Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) }],
    )));
    h.run();
}

/// GitHub issue 32: the Visuals page lists the textures of the object's
/// model, each with its picture. One picked in Select Texture (searched by
/// name, among the game's textures) is written as the game writes it:
/// after the entries there (one for a texture the model lacks, shown so
/// and kept, with the fields it has), and Reset takes each out again.
#[test]
fn visuals_page_picks_a_texture_in_the_gallery_and_resets_it() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_gallery", ResType::UTP) else {
        return;
    };
    h.run();
    let (own, pick) = {
        let chest = field(&mut h, &key);
        let game = h.state().game.as_deref().unwrap();
        let p = mg_preview::placeable(game, &chest).unwrap();
        let own = mg_preview::replaceable::textures(game, &p);
        // A texture of the game's to draw it with.
        let mut all = game.resman.list(ResType::DDS);
        all.sort();
        let pick = all
            .iter()
            .map(|r| r.to_lowercase().to_string())
            .find(|n| n.starts_with("plc_") && !own.iter().any(|t| t.name == *n))
            .expect("a placeable's texture");
        (own, pick)
    };
    let first = own.first().expect("the chest's model has textures");
    assert!(!first.plt);
    put_replacement(&mut h, key, TEXTURES, 9, ("mg_absent", "mg_other"));
    h.get_by_label("Visuals").click();
    h.run();
    // The model's textures, and the entry for one it lacks.
    for t in own.iter().map(|t| t.name.as_str()).chain(["mg_absent"]) {
        assert!(h.query_all_by_label(t).next().is_some(), "{t} is listed");
    }
    h.get_by_label("(not in the model)");
    // Select Texture for the model's first texture: its pictures made over
    // a few frames, found by its name, picked.
    h.get_all_by_label("Select Texture…").next().unwrap().click();
    settle(&mut h);
    h.get_by_label("Select Texture");
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    settle(&mut h);
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text(&pick);
    settle(&mut h);
    h.get_by_label(&pick).click();
    settle(&mut h);
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert!(h.query_by_label("Select Texture").is_none(), "closed");
    let (pairs, kept) = replacements(&mut h, &key, TEXTURES).unwrap();
    assert_eq!(
        pairs,
        [
            ("mg_absent".to_string(), "mg_other".to_string(), true),
            (first.name.clone(), pick.clone(), false),
        ]
    );
    assert!(kept, "the list's struct keeps its fields");
    assert_eq!(field(&mut h, &key).child("TextureReplace").unwrap().id, 9);
    // Opened again and cancelled: nothing changes.
    h.get_all_by_label("Select Texture…").next().unwrap().click();
    settle(&mut h);
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert_eq!(replacements(&mut h, &key, TEXTURES).unwrap().0.len(), 2);
    // Reset, the model's texture's (first), then the other's.
    h.get_all_by_label("Reset").next().unwrap().click();
    h.run();
    let (pairs, _) = replacements(&mut h, &key, TEXTURES).unwrap();
    assert_eq!(pairs, [("mg_absent".to_string(), "mg_other".to_string(), true)]);
    h.get_all_by_label("Reset").next().unwrap().click();
    h.run();
    assert!(field(&mut h, &key).get("TextureReplace").is_none());
}

/// GitHub issue 32: an animation replacement picked from the animations of
/// the object's model and its supermodels, after an entry for one the
/// model lacks (shown so, kept), and Reset.
#[test]
fn visuals_page_picks_an_animation_replacement_and_resets_it() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_anims", ResType::UTP) else {
        return;
    };
    h.run();
    let anims = {
        let chest = field(&mut h, &key);
        let game = h.state().game.as_deref().unwrap();
        let p = mg_preview::placeable(game, &chest).unwrap();
        mg_preview::replaceable::animations(game, &p)
    };
    assert!(anims.len() >= 2, "{anims:?}");
    put_replacement(&mut h, key, ANIMATIONS, 11, ("mg_gone", &anims[0]));
    h.get_by_label("Visuals").click();
    h.run();
    h.get_by_value("mg_gone (not in the model)");
    // A new one: the model's first animation played as its second.
    h.get_by_value("Choose an animation").click();
    h.run();
    h.get_all_by_label(&anims[0]).last().unwrap().click();
    h.run();
    h.get_by_value("Choose the one to play").click();
    h.run();
    h.get_all_by_label(&anims[1]).last().unwrap().click();
    h.run();
    h.get_all_by_label("Add").nth(1).unwrap().click();
    h.run();
    let (pairs, kept) = replacements(&mut h, &key, ANIMATIONS).unwrap();
    assert_eq!(
        pairs,
        [
            ("mg_gone".to_string(), anims[0].clone(), true),
            (anims[0].clone(), anims[1].clone(), false),
        ]
    );
    assert!(kept);
    assert_eq!(field(&mut h, &key).child("AnimationReplace").unwrap().id, 11);
    // Reset: the entry the model lacks, then the other.
    h.get_all_by_label("Reset").next().unwrap().click();
    h.run();
    let (pairs, _) = replacements(&mut h, &key, ANIMATIONS).unwrap();
    assert_eq!(pairs, [(anims[0].clone(), anims[1].clone(), false)]);
    h.get_all_by_label("Reset").next().unwrap().click();
    h.run();
    assert!(field(&mut h, &key).get("AnimationReplace").is_none());
}

/// A creature of body parts lists its parts' textures, mostly PLTs, which
/// the game doesn't replace: shown, with no Select Texture to pick one (the
/// game's hands have a simple texture, which has one). Its animations are
/// its skeleton's and their supermodels'.
#[test]
fn visuals_page_lists_a_creature_s_plt_textures_without_a_gallery() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_vis", ResType::UTC) else {
        return;
    };
    h.run();
    // A human (appearance.2da row 6: a body of parts), wearing nothing:
    // its own parts, number 1.
    let mut g = Gff::new(*b"UTC ");
    g.root = field(&mut h, &key);
    g.root.set("Appearance_Type", mg_gff::Value::Word(6));
    g.root.set("Equip_ItemList", mg_gff::Value::List(Vec::new()));
    for part in [
        "Torso", "Pelvis", "Belt", "Neck", "LThigh", "RThigh", "LShin", "RShin", "LFoot", "LFArm",
        "RFArm", "LBicep", "RBicep", "LHand", "RHand",
    ] {
        g.root.set(&format!("BodyPart_{part}"), mg_gff::Value::Byte(1));
    }
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "human",
        vec![mg_edit::Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) }],
    )));
    h.run();
    h.get_by_label("Visuals").click();
    h.run();
    let (textures, anims) = {
        let c = field(&mut h, &key);
        let game = h.state().game.as_deref().unwrap();
        let p = mg_preview::creature(game, &c, &|_| None).unwrap();
        let t = mg_preview::replaceable::textures(game, &p);
        (t, mg_preview::replaceable::animations(game, &p))
    };
    let plts = textures.iter().filter(|t| t.plt).count();
    assert!(textures.len() > 5 && plts * 2 > textures.len(), "{textures:?}");
    assert!(textures.iter().any(|t| t.name.contains("chest")), "{textures:?}");
    assert!(anims.len() > 50, "a skeleton's animations: {}", anims.len());
    for t in &textures {
        assert!(h.query_all_by_label(&t.name).next().is_some(), "{} is listed", t.name);
    }
    let buttons: Vec<bool> =
        h.get_all_by_label("Select Texture…").map(|b| b.accesskit_node().is_disabled()).collect();
    let plt: Vec<bool> = textures.iter().map(|t| t.plt).collect();
    assert_eq!(buttons, plt, "a PLT has none to pick");
}

/// A placed placeable's Properties have the same pickers: Reset on its
/// replacement edits the area's GIT entry.
#[test]
fn visuals_page_of_a_placed_placeable_lists_its_model_s_textures() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("placed-visuals") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let own = {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let data = game.resman.get(&ResKey::parse("plc_chest1", ResType::UTP).unwrap()).unwrap();
        let chest = Gff::read(&data).unwrap().root;
        let p = mg_preview::placeable(game, &chest).unwrap();
        let own = mg_preview::replaceable::textures(game, &p);
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [12.0, 30.0, 0.0], rotation: 0.0 };
        let mut item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let mut e = mg_gff::Struct::new(0);
        e.set("OldTexture", mg_gff::Value::resref(ResRef::from_str(&own[0].name).unwrap()));
        e.set("NewTexture", mg_gff::Value::resref(ResRef::from_str("mg_gold").unwrap()));
        let mut s = mg_gff::Struct::new(9);
        s.set("TextureReplaceLi", mg_gff::Value::List(vec![e]));
        item.set("TextureReplace", mg_gff::Value::Struct(s));
        let edit = mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
        let path = mg_edit::GffPath::root().item("Placeable List", 0);
        app.blueprint_pages.insert((git, path.clone()), "Visuals");
        app.actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path: path.clone() }));
        (own, path)
    };
    let (own, path) = own;
    h.run_steps(4);
    // (A window clear of the area's own panes.)
    let at = h.state().dock.find_tab(&Tab::Instance { area, path }).unwrap();
    if let Some(w) = h.state_mut().dock.get_window_state_mut(at.surface) {
        w.set_position(egui::pos2(20.0, 60.0)).set_size(egui::vec2(1000.0, 700.0));
    }
    h.run_steps(4);
    for t in &own {
        assert!(h.query_all_by_label(&t.name).next().is_some(), "{} is listed", t.name);
    }
    assert_eq!(h.query_all_by_label("Select Texture…").count(), own.len());
    let placed = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git).unwrap().root.list("Placeable List").unwrap()[0].clone()
    };
    assert!(placed(&mut h).get("TextureReplace").is_some());
    h.get_all_by_label("Reset").next().unwrap().click();
    h.run_steps(3);
    assert!(placed(&mut h).get("TextureReplace").is_none());
}

#[test]
fn classes_page_sets_domains_and_associates() {
    let Some((mut h, key)) = blueprint_harness("nw_mumcleric", "cleric_copy", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run();
    let cleric = |h: &mut Harness<'_, Moonglow>| -> mg_gff::Struct {
        let root = field(h, &key);
        root.list("ClassList")
            .unwrap()
            .iter()
            .find(|c| c.integer("Class") == Some(2))
            .unwrap()
            .clone()
    };
    // The cleric's first domain, from domains.2da (the blueprint has none:
    // the game decides).
    assert_eq!(cleric(&mut h).integer("Domain1"), None);
    let animal = {
        let game = h.state().game.as_deref().unwrap();
        let t = game.table("domains").unwrap();
        game.string(mg_core::StrRef(t.get_int(1, "Name").unwrap() as u32)).unwrap()
    };
    let current = "Not set";
    h.get_all_by_value(current).next().expect("the domain picker").click();
    h.run();
    h.get_by_label(&animal).click();
    h.run();
    assert_eq!(cleric(&mut h).integer("Domain1"), Some(1));
    // A cleric has neither a familiar nor a companion: the game ignores
    // them, and the page says so.
    assert!(h.query_by_label_contains("a familiar only for a creature with an arcane").is_some());
    assert!(h.query_by_label_contains("companion only for a creature with a divine").is_some());
}

/// A weapon's parts are chosen from pictures of the part alone: the
/// icon's texture is the whole weapon's size (256 points for a great
/// sword), and three lists of those were more than a window.
#[test]
fn a_weapon_s_parts_are_pictured_alone() {
    let Some((mut h, key)) = blueprint_harness("nw_wswgs001", "sword_copy", ResType::UTI) else {
        return;
    };
    h.run();
    h.get_by_label("Appearance").click();
    h.run_steps(4);
    for part in ["Top", "Middle", "Bottom"] {
        let size = h.get_by_label(&format!("{part} 12")).rect().size();
        assert!(size.y <= 96.0 && size.x <= 64.0, "{part}: {size:?}");
    }
    // A click on one chooses it.
    h.get_by_label("Middle 21").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("ModelPart2"), Some(21));
}

/// A creature's Race lists the racial types that have a name, as Aurora
/// does: not `DELETED` and the like (rows with a label and no name, as
/// CEP's `CEP_RESERVED` are).
#[test]
fn a_creature_s_races_are_those_with_a_name() {
    let Some((mut h, _key)) = blueprint_harness("nw_chicken", "race_list", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_value("Animal").click();
    h.run();
    assert!(h.query_by_label("Human").is_some(), "the races are listed");
    assert!(h.query_by_label("Elf").is_some());
    assert!(h.query_by_label("DELETED").is_none(), "a row without a name is not");
    assert!(h.query_by_label("INVALID_RACE").is_none());
}

#[test]
fn item_editor_adds_properties_and_keeps_the_cost() {
    let Some((mut h, key)) = blueprint_harness("nw_wswls001", "sword_copy", ResType::UTI) else {
        return;
    };
    let cost_of = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let value = mg_rules::ItemValue::from_gff(&s);
        let game = h.state().game.as_deref().unwrap();
        (s.integer("Cost"), Some(i64::from(game.item_cost(&value))))
    };
    h.run();
    h.get_by_label("Properties").click();
    h.run();
    let before = field(&mut h, &key).list("PropertiesList").map_or(0, <[_]>::len);
    // Far down the list: scrolled to first.
    h.get_by_label("Enhancement Bonus").scroll_to_me();
    h.run();
    h.get_by_label("Enhancement Bonus").click();
    h.run();
    h.get_by_label("Add").click();
    h.run();
    let props = field(&mut h, &key).list("PropertiesList").unwrap().to_vec();
    assert_eq!(props.len(), before + 1);
    assert_eq!(props.last().unwrap().integer("PropertyName"), Some(6));
    // The stored cost follows the properties.
    let (stored, computed) = cost_of(&mut h);
    assert_eq!(stored, computed);
    let plus_one = stored;
    // The new property is selected: its value, +1, becomes +3.
    // (Under the lists: scrolled to, above the window's OK and Cancel.)
    h.get_by_value("+1").scroll_to_me();
    h.run();
    h.get_by_value("+1").click();
    h.run();
    h.get_by_label("+3").click();
    h.run();
    assert_eq!(
        field(&mut h, &key).list("PropertiesList").unwrap().last().unwrap().integer("CostValue"),
        Some(3)
    );
    let (stored, computed) = cost_of(&mut h);
    assert_eq!(stored, computed);
    assert!(stored > plus_one);
    // One undo takes back the value and its cost.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(cost_of(&mut h).0, plus_one);
    // Search: the kinds and choices that have every word typed, opened
    // out (a builder: Cast Spell alone has hundreds).
    let count = |h: &Harness<'_, Moonglow>, label: &str| h.query_all_by_label(label).count();
    let before = count(&h, "Enhancement Bonus");
    assert!(before >= 1);
    // (Back up the page, to the search field.)
    let search = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.placeholder() == Some("🔍 Search properties")
    };
    h.get(egui_kittest::kittest::by().predicate(search)).scroll_to_me();
    h.run();
    type_into_hint(&mut h, "🔍 Search properties", "cast fireball");
    h.run_steps(2);
    assert_eq!(count(&h, "Cast Spell"), 1);
    assert_eq!(count(&h, "Enhancement Bonus"), before - 1, "not among those found");
    assert!(h.query_all_by_label_contains("Fireball").next().is_some(), "its choices show");
    assert!(h.query_all_by_label_contains("Magic Missile").next().is_none());
    h.get_all_by_value("cast fireball").next().unwrap().type_text(" zzz");
    h.run_steps(2);
    h.get_by_label("No property has that.");
}

#[test]
fn creature_editor_levels_and_aligns() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_copy", ResType::UTC) else {
        return;
    };
    let max_hp = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let sheet = mg_rules::CreatureSheet::from_gff(&s);
        let game = h.state().game.as_deref().unwrap();
        (s.integer("MaxHitPoints"), i64::from(game.creature_stats(&sheet).max_hit_points))
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run();
    // A second class: one more level, so more hit points from Constitution,
    // stored with the class in one undo step.
    let (before, _) = max_hp(&mut h);
    h.get_by_label("Add Class").click();
    h.run();
    assert_eq!(field(&mut h, &key).list("ClassList").unwrap().len(), 2);
    let (stored, computed) = max_hp(&mut h);
    assert_eq!(stored, Some(computed));
    assert_ne!(stored, before);
    // So is the challenge rating, recalculated as Aurora does on OK.
    let s = field(&mut h, &key);
    let game = h.state().game.as_deref().unwrap();
    let item = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let mut sheet = mg_rules::CreatureSheet::from_gff(&s);
    sheet.gear_value = game.gear_value(&s, &item);
    assert!(sheet.gear_value > 0, "the bandit's gear");
    assert_eq!(s.float("ChallengeRating"), Some(game.challenge(&sheet).rating));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(field(&mut h, &key).list("ClassList").unwrap().len(), 1);
    assert_eq!(max_hp(&mut h).0, before);
    // An alignment preset sets both axes.
    h.get_by_value("Chaotic Neutral").click();
    h.run();
    h.get_by_label("Lawful Good").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!((s.integer("GoodEvil"), s.integer("LawfulChaotic")), (Some(100), Some(100)));
}

#[test]
fn creature_editor_lists() {
    let Some((mut h, key)) = blueprint_harness("db_tanarukk_do", "sorc_copy", ResType::UTC) else {
        return;
    };
    let count =
        |h: &mut Harness<'_, Moonglow>, list: &str| field(h, &key).list(list).map_or(0, <[_]>::len);
    h.run();
    // A feat.
    h.get_by_label("Feats").click();
    h.run();
    // Alertness is feat 0; the checkbox toggles it.
    let has_alertness = |h: &mut Harness<'_, Moonglow>| {
        field(h, &key).list("FeatList").unwrap().iter().any(|f| f.integer("Feat") == Some(0))
    };
    let had = has_alertness(&mut h);
    type_into_hint(&mut h, "Find", "Alertness");
    // (Its checkbox, the first of that name; one it has is listed beside
    // the feats too, under Assigned.)
    assert_eq!(h.query_all_by_label("Alertness").count(), 1 + usize::from(had));
    h.get_all_by_label("Alertness").next().unwrap().click();
    h.run();
    assert_eq!(has_alertness(&mut h), !had);
    assert_eq!(h.query_all_by_label("Alertness").count(), 1 + usize::from(!had));
    h.get_by_label("Assigned");
    // A known spell for the sorcerer (level 1: Magic Missile).
    h.get_by_label("Spells").click();
    h.run();
    let known = |h: &mut Harness<'_, Moonglow>| {
        let classes = field(h, &key).list("ClassList").unwrap().to_vec();
        let sorcerer = classes.iter().find(|c| c.integer("Class") == Some(9)).unwrap();
        sorcerer.list("KnownList1").map_or(0, <[_]>::len)
    };
    let before = known(&mut h);
    type_into_hint(&mut h, "Find", "Magic Missile");
    h.get_by_label("Magic Missile").click();
    h.run();
    assert_eq!(known(&mut h), before + 1);
    // A special ability.
    h.get_by_label("Special Abilities").click();
    h.run();
    let specials = count(&mut h, "SpecAbilityList");
    type_into_hint(&mut h, "Find", "Fireball");
    h.get_all_by_label("Fireball").next().unwrap().click();
    h.run();
    assert_eq!(count(&mut h, "SpecAbilityList"), specials + 1);
    // Ready as added (the game reads the flags as yes or no: the one
    // switch); switched off, its uses are spent.
    let flags = |h: &mut Harness<'_, Moonglow>| {
        let list = field(h, &key).list("SpecAbilityList").unwrap().to_vec();
        list.last().unwrap().integer("SpellFlags")
    };
    assert_eq!(flags(&mut h), Some(1));
    assert!(h.query_by_label("Unlimited").is_none() && h.query_by_label("Spontaneous").is_none());
    let ready = egui_kittest::kittest::by().role(egui::accesskit::Role::CheckBox);
    h.query_all(ready.clone()).last().unwrap().click();
    h.run();
    assert_eq!(flags(&mut h), Some(0));
    h.query_all(ready).last().unwrap().click();
    h.run();
    assert_eq!(flags(&mut h), Some(1));
    // Armor, chosen in the palette, equipped in the armor slot (the second).
    h.get_by_label("Inventory").click();
    h.run();
    type_into_hint(&mut h, "Find", "nw_aarcl001");
    h.get_by_label("Leather Armor").click();
    h.run();
    // (The Equip under the palette comes first, then each slot's.)
    h.get_all_by_label("Equip").nth(2).unwrap().click();
    h.run();
    let worn = |h: &mut Harness<'_, Moonglow>, slot: u32| {
        let equipped = field(h, &key).list("Equip_ItemList").unwrap().to_vec();
        equipped.iter().find(|s| s.id == slot).map(|s| s.resref("EquippedRes").unwrap().to_string())
    };
    let packed = |h: &mut Harness<'_, Moonglow>| {
        let items = field(h, &key).list("ItemList").unwrap_or(&[]).to_vec();
        items.iter().map(|s| s.resref("InventoryRes").unwrap().to_string()).collect::<Vec<_>>()
    };
    assert_eq!(worn(&mut h, 2).as_deref(), Some("nw_aarcl001"));
    // To Backpack, on the equipped item selected: unequipped and packed, in
    // one step.
    let had = packed(&mut h).len();
    h.get_all_by_label("Leather Armor").last().unwrap().click();
    h.run();
    h.get_by_label("To Backpack").click();
    h.run();
    assert_eq!(worn(&mut h, 2), None);
    assert_eq!(packed(&mut h).last().map(String::as_str), Some("nw_aarcl001"));
    assert_eq!(packed(&mut h).len(), had + 1);
    // The Equip under the palette puts the chosen item in the first free
    // slot it goes in: the armor's, again.
    h.get_all_by_label("Equip").next().unwrap().click();
    h.run();
    assert_eq!(worn(&mut h, 2).as_deref(), Some("nw_aarcl001"), "in the armor slot");
    // Taken, a second can't be equipped: said in the log, nothing changed.
    let said = h.state().log.entries.len();
    h.get_all_by_label("Equip").next().unwrap().click();
    h.run();
    assert!(h.state().log.entries[said..].iter().any(|e| e.1.contains("is taken")));

    // Dragged and dropped. Where things are: the palette's row is the
    // topmost "Leather Armor" left of the equipment, the armor slot's name the rightmost
    // "Armor", the backpack's rows those under its heading.
    let drag = |h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2| {
        h.hover_at(from);
        h.run_steps(1);
        press(h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for k in 1..=5 {
            h.hover_at(from + (to - from) * (k as f32 / 5.0));
            h.run_steps(1);
        }
        press(h, to, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    let places = |h: &Harness<'_, Moonglow>| {
        let rects = |label: &str| -> Vec<egui::Rect> {
            h.get_all_by_label(label).map(|n| n.rect()).collect()
        };
        let slot = rects("Armor").into_iter().max_by(|a, b| a.left().total_cmp(&b.left())).unwrap();
        let heading = h.get_by_label("Backpack").rect();
        let armors = rects("Leather Armor");
        // (The palette's row: left of the equipment, above what it says of
        // the item chosen.)
        let palette = armors
            .iter()
            .copied()
            .filter(|r| r.right() < slot.left())
            .min_by(|a, b| a.top().total_cmp(&b.top()))
            .unwrap();
        // (Its name, right of its icon.)
        let worn = armors
            .iter()
            .copied()
            .filter(|r| r.left() > slot.left() && (r.center().y - slot.center().y).abs() < 12.0)
            .max_by(|a, b| a.left().total_cmp(&b.left()));
        // (Their names, wider than their icons.)
        let packed: Vec<egui::Rect> = armors
            .iter()
            .copied()
            .filter(|r| r.top() > heading.bottom() && r.width() > 80.0)
            .collect();
        (slot, heading, palette, worn, packed)
    };
    // (A window tall enough to show the equipment and the backpack under it.)
    h.set_size(egui::vec2(1000.0, 1600.0));
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(Tab::Blueprint(key)));
    h.run_steps(10);
    // The equipped armor, onto the backpack: off, and packed.
    let had = packed(&mut h).len();
    let (_, heading, _, on, _) = places(&h);
    let backpack = heading.left_bottom() + egui::vec2(80.0, 40.0);
    drag(&mut h, on.expect("the armor worn").center(), backpack);
    assert_eq!(worn(&mut h, 2), None, "dragged off");
    assert_eq!(packed(&mut h).len(), had + 1);
    // A backpack row, onto the armor slot: worn again, out of the backpack.
    let (slot, _, _, _, rows) = places(&h);
    let row = rows.iter().copied().max_by(|a, b| a.width().total_cmp(&b.width())).unwrap();
    drag(&mut h, row.center(), slot.center());
    assert_eq!(worn(&mut h, 2).as_deref(), Some("nw_aarcl001"), "dropped on its slot");
    assert_eq!(packed(&mut h).len(), had);
    // The palette's row, onto the backpack: one more in it.
    let (_, heading, palette, _, _) = places(&h);
    drag(&mut h, palette.center(), heading.left_bottom() + egui::vec2(80.0, 40.0));
    assert_eq!(packed(&mut h).len(), had + 1, "added from the palette");
    // And onto a slot it doesn't go in (the helmet's): refused, in the log.
    let (_, _, palette, _, _) = places(&h);
    let helmet = h
        .get_all_by_label("Helmet")
        .map(|n| n.rect())
        .max_by(|a, b| a.left().total_cmp(&b.left()))
        .unwrap();
    let said = h.state().log.entries.len();
    drag(&mut h, palette.center(), helmet.center());
    assert_eq!(worn(&mut h, 1), None);
    assert!(h.state().log.entries[said..].iter().any(|e| e.1.contains("does not go in that slot")));
}

#[test]
fn store_wizard_makes_aurora_s_store() {
    use mg_module::palette::BlueprintKind;
    let Some(mut h) = game_harness("store") else { return };
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Store));
    h.run();
    h.get_by_label("Merchants").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    // The suggested name follows the category.
    assert!(h.query_all_by_value("Merchants 001").next().is_some());
    h.get_by_label("Finish").click();
    h.run();
    let made = module_gff(&mut h, "merchants001", ResType::UTM).expect("the store");
    let r = ResRef::from_str("merchants001").unwrap();
    assert_eq!(made, mg_module::blueprints::store(r, "Merchants 001", 5));
    assert!(h.state().blueprint_wizard.is_none());
}

#[test]
fn sound_wizard_steps_through_timing_positioning_and_waves() {
    use mg_module::palette::BlueprintKind;
    let Some(mut h) = game_harness("sound") else { return };
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Sound));
    h.run();
    h.get_by_label("Civilization").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_by_label("Seamlessly looping").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    // Random positions are for single shots only.
    assert!(h.get_by_label("Random Positional").accesskit_node().is_disabled());
    h.get_by_label("Positional").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_by_label("Add Sounds…").click();
    h.run();
    h.state_mut().picker.as_mut().unwrap().filter = "al_cv_firecamp1".into();
    h.run();
    h.get_all_by_label_contains("al_cv_firecamp1").next().unwrap().click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_by_label("Finish").click();
    h.run();
    let made = module_gff(&mut h, "civilization001", ResType::UTS).expect("the sound");
    let r = ResRef::from_str("civilization001").unwrap();
    let fire = ResRef::from_str("al_cv_firecamp1").unwrap();
    let expected = mg_module::blueprints::sound(
        r,
        "Civilization 001",
        13,
        mg_module::blueprints::SoundStyle::LoopingPositional,
        &[fire],
    );
    assert_eq!(made, expected);
    // Its properties open, as Aurora's Sound Wizard does by default.
    let key = ResKey::parse("civilization001", ResType::UTS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Blueprint(key)).is_some());
}

#[test]
fn item_wizard_makes_a_weapon_with_its_cost() {
    use mg_module::palette::BlueprintKind;
    let Some(mut h) = game_harness("item") else { return };
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Item));
    h.run();
    h.get_by_label("Bastard Sword").scroll_to_me();
    h.run();
    h.get_by_label("Bastard Sword").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    type_into_hint(&mut h, "Name", "Wizard Sword");
    // Enter is Next, and only Next: it went on and finished at once, in
    // category 0, with the page before the last's Enter.
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    assert!(module_gff(&mut h, "wizardsword", ResType::UTI).is_none(), "not finished yet");
    assert!(h.state().blueprint_wizard.is_some());
    for branch in ["Weapons", "Bladed"] {
        h.get_by_label(branch).click();
        h.run();
    }
    h.get_by_label("Bastard Swords").click();
    h.run();
    h.get_by_label("Finish").click();
    h.run();
    let made = module_gff(&mut h, "wizardsword", ResType::UTI).expect("the item");
    assert_eq!(made.root.integer("BaseItem"), Some(3));
    assert_eq!(made.root.integer("Cost"), Some(70));
    assert_eq!(made.root.integer("PaletteID"), Some(33));

    // A second of the same name, with a ResRef and Tag of the builder's
    // own; the ResRef the module has already is refused.
    h.state_mut().blueprint_wizard =
        Some(mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Item));
    h.run();
    h.get_by_label("Bastard Sword").scroll_to_me();
    h.run();
    h.get_by_label("Bastard Sword").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    type_into_hint(&mut h, "Name", "Wizard Sword");
    h.state_mut().blueprint_wizard.as_mut().unwrap().resref = "wizardsword".into();
    h.run();
    assert!(h.query_by_label_contains("has a wizardsword.uti already").is_some());
    let w = h.state_mut().blueprint_wizard.as_mut().unwrap();
    (w.resref, w.tag) = ("it_sword_ws2".into(), "IT_WS_TWO".into());
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    // (The categories stay open from the first time.)
    for branch in ["Weapons", "Bladed"] {
        if h.query_by_label("Bastard Swords").is_none() {
            h.get_by_label(branch).click();
            h.run();
        }
    }
    h.get_by_label("Bastard Swords").click();
    h.run();
    h.get_by_label("Finish").click();
    h.run();
    let second = module_gff(&mut h, "it_sword_ws2", ResType::UTI).expect("the second item");
    assert_eq!(second.root.string("Tag"), Some(&b"IT_WS_TWO"[..]));
    assert_eq!(
        second.root.resref("TemplateResRef").map(|r| r.to_string()).as_deref(),
        Some("it_sword_ws2")
    );
}

/// The Item Wizard's Magical, as Aurora's: ticked, the item level and
/// quality are chosen before going on, and the item gets random
/// properties its base item can have, the same for the same seed, and
/// is written unidentified. A torch can't be made magical.
#[test]
fn item_wizard_makes_a_magical_item_of_a_level_and_quality() {
    use mg_module::palette::BlueprintKind;
    let Some(mut h) = game_harness("item-magic") else { return };
    let make = |h: &mut Harness<'_, Moonglow>, base: &str, name: &str| {
        let mut w = mg_ui::blueprint_wizard::BlueprintWizard::new(BlueprintKind::Item);
        w.seed = 7;
        h.state_mut().blueprint_wizard = Some(w);
        h.run();
        h.get_by_label(base).scroll_to_me();
        h.run();
        h.get_by_label(base).click();
        h.run();
        h.get_by_label("Next >").click();
        h.run();
        type_into_hint(h, "Name", name);
    };
    make(&mut h, "Longsword", "Magic Sword");
    // The level and quality wait for Magical; with it, Next waits for
    // them.
    assert!(h.get_by_label("Godly").accesskit_node().is_disabled());
    h.get_by_label("Magical").click();
    h.run();
    assert!(!h.get_by_label("Godly").accesskit_node().is_disabled());
    assert!(h.get_by_label("Next >").accesskit_node().is_disabled());
    h.get_by_label("16-20").click();
    h.run();
    assert!(h.get_by_label("Next >").accesskit_node().is_disabled());
    h.get_by_label("Godly").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_by_label("Plot Item").click();
    h.run();
    h.get_by_label("Finish").click();
    h.run();
    let made = module_gff(&mut h, "magicsword", ResType::UTI).expect("the item");
    let game = h.state().game.clone().unwrap();
    let props: Vec<mg_rules::ItemProperty> = made
        .root
        .list("PropertiesList")
        .unwrap()
        .iter()
        .map(mg_rules::ItemProperty::from_gff)
        .collect();
    assert!(!props.is_empty());
    let available = game.available_properties(1);
    assert!(props.iter().all(|p| available.iter().any(|a| a.row == p.property)), "{props:?}");
    // 16-20, Godly: a longsword's first property is of the melee
    // table's top tier.
    let melee = game.table("itmwizmelee").unwrap();
    let first = props[0];
    let top = (0..melee.len()).any(|r| {
        melee.get_int(r, "Value") == Some(8)
            && melee.get_int(r, "Prop") == Some(first.property.into())
            && melee.get_int(r, "CostVal").unwrap_or(0) == i32::from(first.cost_value)
    });
    assert!(top, "{first:?}");
    assert_eq!(made.root.integer("Identified"), Some(0));
    let cost = game.item_cost(&mg_rules::ItemValue::from_gff(&made.root));
    assert_eq!(made.root.integer("Cost"), Some(cost.into()));
    assert!(cost > 30000, "a magical sword's cost: {cost}");
    // The same seed, the same properties.
    let plain = mg_module::blueprints::item(&game, ResRef::from_str("x").unwrap(), "X", 1, 0);
    let mut again = plain.clone();
    let m = mg_rules::item_wizard::Magical { level: 3, quality: 3, seed: 7 };
    mg_module::blueprints::make_magical(&game, &mut again, m);
    assert_eq!(again.root.list("PropertiesList"), made.root.list("PropertiesList"));

    // A torch: no Magical.
    make(&mut h, "Torch", "Plain Torch");
    assert!(h.get_by_label("Magical").accesskit_node().is_disabled());
}

#[test]
fn several_blueprints_are_edited_together() {
    use mg_edit::{Command, Edit};
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-edit-together");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let game = app.game.as_deref().unwrap();
    let base = Gff::read(&game.resman.get_named("plc_chest1", ResType::UTP).unwrap()).unwrap();
    let keys: Vec<ResKey> =
        ["mg_box_a", "mg_box_b"].iter().map(|n| ResKey::parse(n, ResType::UTP).unwrap()).collect();
    let edits = keys
        .iter()
        .map(|k| {
            let mut g = base.clone();
            g.root.set("TemplateResRef", mg_gff::Value::resref(k.resref));
            Edit::SetResource { key: *k, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    app.actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
    app.palette.tiles = false;
    app.palette.custom = true;
    app.palette.filter = "mg_box".into();
    let mut h = harness(app, 1000.0, 800.0);
    h.run();
    let name = {
        let game = h.state().game.as_deref().unwrap();
        mg_module::palette::blueprint_name(
            mg_module::palette::BlueprintKind::Placeable,
            &base.root,
            game,
        )
    };
    let buttons: Vec<_> = h.get_all_by_label(&name).collect();
    assert_eq!(buttons.len(), 2);
    buttons[0].click();
    h.run();
    h.get_all_by_label(&name).nth(1).unwrap().click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    h.get_all_by_label(&name).next().unwrap().click_secondary();
    h.run();
    h.get_by_label("Edit 2 Together").click();
    h.run();
    assert!(h.state().dock.find_tab(&Tab::Blueprints(keys.clone())).is_some());
    // Lists are edited one at a time: no Inventory page.
    assert!(h.query_by_label_contains("edited one blueprint at a time").is_some());
    assert!(h.query_by_label("Inventory").is_none());
    h.get_by_label("Plot").click();
    h.run();
    for k in &keys {
        assert_eq!(field(&mut h, k).integer("Plot"), Some(1), "{k}");
    }
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Plot"));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    for k in &keys {
        assert_eq!(field(&mut h, k).integer("Plot"), Some(0), "{k}");
    }
}

#[test]
fn variable_sets_are_saved_and_added_elsewhere() {
    let Some((mut h, key)) = blueprint_harness("nw_waypoint001", "waypoint_vars", ResType::UTW)
    else {
        return;
    };
    let sets = mg_testkit::scratch_dir("ui-var-sets").join("sets");
    h.state_mut().var_set_dir = Some(sets.clone());
    h.run();
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("Variables (0)…").click();
    h.run();
    // Two variables, kept as a set.
    {
        let edit = h.state_mut().var_edit.as_mut().expect("the Variables window");
        edit.rows.push(mg_ui::widgets::VarRow::new("MG_LOOT", 1, "3"));
        edit.rows.push(mg_ui::widgets::VarRow::new("MG_SAY", 3, "Hail"));
    }
    h.run();
    h.get_by_label("Save Set…").click();
    h.run();
    h.get_all_by_role(egui::accesskit::Role::TextInput).last().unwrap().type_text("Loot");
    h.run();
    h.get_by_label("Save").click();
    h.run();
    assert!(sets.join("Loot.vars.json").is_file());
    h.get_all_by_label("Cancel").last().unwrap().click();
    h.run();
    assert!(field(&mut h, &key).list("VarTable").is_none_or(|l| l.is_empty()), "not applied");
    // Added to the variables (one already named MG_LOOT takes its value).
    h.get_by_label("Variables (0)…").click();
    h.run();
    h.state_mut()
        .var_edit
        .as_mut()
        .unwrap()
        .rows
        .push(mg_ui::widgets::VarRow::new("MG_LOOT", 1, "9"));
    h.run();
    h.get_by_label("Add Set").click();
    h.run();
    h.get_by_label("Loot").click();
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    let vars: Vec<(String, Option<i64>)> = field(&mut h, &key)
        .list("VarTable")
        .unwrap()
        .iter()
        .map(|v| {
            (String::from_utf8_lossy(v.string("Name").unwrap()).into_owned(), v.integer("Value"))
        })
        .collect();
    assert_eq!(vars, [("MG_LOOT".to_string(), Some(3)), ("MG_SAY".to_string(), None)]);
}

#[test]
fn script_sets_save_and_load() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_scripts", ResType::UTC)
    else {
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-script-set");
    let path = dir.join("bandit.ini");
    h.run();
    h.get_by_label("Scripts").click();
    h.run();
    // Saved in Aurora's layout: [ResRefs], Aurora's event names.
    let spawn = field(&mut h, &key).resref("ScriptSpawn").unwrap();
    h.state_mut().dialogs =
        Box::new(mg_ui::NoDialogs { save: vec![path.clone()], ..Default::default() });
    h.get_by_label("Save Script Set").click();
    h.run();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("[ResRefs]\r\nOnBlocked="), "{text}");
    assert!(text.contains(&format!("OnSpawn={spawn}\r\n")), "{text}");
    assert!(text.contains("OnSpellCast="), "Aurora's name for OnSpellCastAt: {text}");
    // A set that names only OnSpawn: loading it sets OnSpawn and clears
    // the others, in one undoable step.
    std::fs::write(&path, "[ResRefs]\r\nOnSpawn=mg_spawn\r\n").unwrap();
    h.state_mut().dialogs = Box::new(mg_ui::NoDialogs { open: vec![path], ..Default::default() });
    h.get_by_label("Load Script Set").click();
    h.run();
    let s = field(&mut h, &key);
    assert_eq!(s.resref("ScriptSpawn"), Some(ResRef::from_str("mg_spawn").unwrap()));
    assert_eq!(s.resref("ScriptHeartbeat"), Some(ResRef::EMPTY));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(field(&mut h, &key).resref("ScriptSpawn"), Some(spawn));
}

/// The spells.2da row whose label is `label`.
fn spell_row(h: &Harness<'_, Moonglow>, label: &str) -> i64 {
    let game = h.state().game.clone().unwrap();
    let t = game.table("spells").unwrap();
    (0..t.len()).find(|&r| t.get(r, "Label") == Some(label)).unwrap() as i64
}

/// A cleric's spell list `list`: (Spell, SpellMetaMagic, SpellFlags) of each.
fn spell_list(h: &mut Harness<'_, Moonglow>, key: &ResKey, list: &str) -> Vec<(i64, i64, i64)> {
    let classes = field(h, key).list("ClassList").unwrap().to_vec();
    let n = |s: &mg_gff::Struct, l: &str| s.integer(l).unwrap_or(-1);
    // (The cleric's: the mummy is undead first.)
    let cleric = classes.iter().find(|c| c.integer("Class") == Some(2)).unwrap();
    cleric
        .list(list)
        .unwrap_or(&[])
        .iter()
        .map(|s| (n(s, "Spell"), n(s, "SpellMetaMagic"), n(s, "SpellFlags")))
        .collect()
}

/// A cleric's Spells page, laid out as the Feats page: a click on a spell
/// of those to choose from prepares one more of it, each row of those
/// prepared has its count, with arrows, and × takes it away; the other
/// entries stay as they were, a metamagic one too. One change each.
#[test]
fn the_spells_page_prepares_spells_beside_those_to_choose_from() {
    use mg_gff::Value;
    let Some((mut h, key)) = blueprint_harness("nw_mumcleric", "cleric_spells", ResType::UTC)
    else {
        return;
    };
    h.run();
    // An Empowered Bless among the level 3 spells (as Aurora writes a spell
    // prepared with metamagic: at its level raised by it).
    let bless = spell_row(&h, "Bless");
    let path = mg_edit::GffPath::root().item("ClassList", 1);
    let mut empowered = mg_gff::Struct::new(3);
    empowered.set("Spell", Value::Word(bless as u16));
    empowered.set("SpellMetaMagic", Value::Byte(1));
    empowered.set("SpellFlags", Value::Byte(1));
    let at = spell_list(&mut h, &key, "MemorizedList3").len();
    let edit = mg_edit::Edit::insert_item(key, path, "MemorizedList3", at, empowered);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("meta", vec![edit])));
    h.run();
    h.get_by_label("Spells").click();
    h.run();
    let level1 = spell_list(&mut h, &key, "MemorizedList1");
    let level3 = spell_list(&mut h, &key, "MemorizedList3");
    // Bane, which the cleric hasn't prepared: one at level 1, at the end.
    let bane = spell_row(&h, "Bane");
    h.get_by_label("Bane").click();
    h.run();
    let mut more = level1.clone();
    more.push((bane, 0, 1));
    assert_eq!(spell_list(&mut h, &key, "MemorizedList1"), more);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(spell_list(&mut h, &key, "MemorizedList1"), level1);
    // The Empowered Bless is shown, and × takes it alone away.
    let shown = h.get_by_label("Bless (Empower Spell)").rect();
    let x = h
        .get_all_by_label("×")
        .find(|n| (n.rect().center().y - shown.center().y).abs() < 4.0)
        .expect("its ×");
    x.click();
    h.run();
    let without: Vec<_> = level3.iter().copied().filter(|e| *e != (bless, 1, 1)).collect();
    assert_eq!(spell_list(&mut h, &key, "MemorizedList3"), without);
    assert_eq!(spell_list(&mut h, &key, "MemorizedList1"), level1, "the others as they were");
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(spell_list(&mut h, &key, "MemorizedList3"), level3);
    // Light's count: its arrow up prepares one more, a copy of it.
    let light = spell_row(&h, "Light");
    let level0 = spell_list(&mut h, &key, "MemorizedList0");
    let lights = level0.iter().filter(|e| e.0 == light).count();
    let row = h.get_all_by_label("Light").last().expect("Light among those prepared").rect();
    let up = h
        .get_all_by_label("⏶")
        .find(|n| (n.rect().center().y - row.center().y).abs() < 4.0)
        .expect("its arrow up")
        .rect()
        .center();
    h.hover_at(up);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: up,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    let after = spell_list(&mut h, &key, "MemorizedList0");
    assert_eq!(after.iter().filter(|e| e.0 == light).count(), lights + 1);
    assert_eq!(after.len(), level0.len() + 1);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(spell_list(&mut h, &key, "MemorizedList0"), level0);
    // How many the cleric may have of each level, as Aurora counts them.
    assert!(h.query_by_label_contains("Level 1: ").is_some());
}

/// Aurora's Meta-Magic: the metamagic feats the creature has, and with one
/// chosen, a spell is prepared with it at its casting level (Bless, level
/// 1, Empowered: level 3), its count is of those, and it is listed as
/// Aurora's summary names it.
#[test]
fn the_spells_page_prepares_with_the_metamagic_chosen() {
    use mg_gff::Value;
    let Some((mut h, key)) = blueprint_harness("nw_mumcleric", "cleric_meta", ResType::UTC) else {
        return;
    };
    h.run();
    // Empower Spell (feat.2da 11).
    let mut feat = mg_gff::Struct::new(1);
    feat.set("Feat", Value::Word(11));
    let feats = field(&mut h, &key).list("FeatList").map_or(0, <[_]>::len);
    let edit = mg_edit::Edit::insert_item(key, mg_edit::GffPath::root(), "FeatList", feats, feat);
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("feat", vec![edit])));
    h.run();
    h.get_by_label("Spells").click();
    h.run();
    // The Meta-Magic list (showing None).
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    h.query_all(combo)
        .find(|n| n.accesskit_node().value() == Some("None".into()))
        .expect("the Meta-Magic list")
        .click();
    h.run();
    h.get_by_label("Empower Spell").click();
    h.run();
    let bless = spell_row(&h, "Bless");
    let level3 = spell_list(&mut h, &key, "MemorizedList3");
    let level1 = spell_list(&mut h, &key, "MemorizedList1");
    // (Those to choose from come first.)
    h.get_all_by_label("Bless").next().expect("Bless to choose").click();
    h.run();
    let mut more = level3.clone();
    more.push((bless, 1, 1));
    assert_eq!(spell_list(&mut h, &key, "MemorizedList3"), more);
    assert_eq!(spell_list(&mut h, &key, "MemorizedList1"), level1);
    assert!(h.query_by_label("Bless (Empower Spell)").is_some());
    assert!(h.query_by_label("Bless  ×1").is_some(), "the count is of the Empowered ones");
}

/// A sorcerer's known spells: ticked among those to choose from, listed
/// by level beside them, × to forget one.
#[test]
fn the_spells_page_ticks_known_spells() {
    let Some((mut h, key)) = blueprint_harness("db_tanarukk_do", "sorc_known", ResType::UTC) else {
        return;
    };
    let known = |h: &mut Harness<'_, Moonglow>, l: usize| -> Vec<i64> {
        let classes = field(h, &key).list("ClassList").unwrap().to_vec();
        let sorcerer = classes.iter().find(|c| c.integer("Class") == Some(9)).unwrap().clone();
        let list = sorcerer.list(&format!("KnownList{l}")).unwrap_or(&[]).to_vec();
        list.iter().filter_map(|s| s.integer("Spell")).collect()
    };
    h.run();
    h.get_by_label("Spells").click();
    h.run();
    let acid_splash = spell_row(&h, "Acid_Splash");
    let was = known(&mut h, 0);
    assert!(!was.contains(&acid_splash));
    h.get_by_label("Acid Splash").click();
    h.run();
    let mut more = was.clone();
    more.push(acid_splash);
    assert_eq!(known(&mut h, 0), more);
    // Listed beside, where its × forgets it.
    let shown = h.get_all_by_label("Acid Splash").last().unwrap().rect();
    h.get_all_by_label("×")
        .find(|n| (n.rect().center().y - shown.center().y).abs() < 4.0)
        .expect("its ×")
        .click();
    h.run();
    assert_eq!(known(&mut h, 0), was);
}

#[test]
fn class_spell_lists_save_clear_and_load() {
    let Some((mut h, key)) = blueprint_harness("db_tanarukk_do", "sorc_lists", ResType::UTC) else {
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-spell-list");
    let path = dir.join("sorcerer.ini");
    let known = |h: &mut Harness<'_, Moonglow>| -> Vec<Vec<i64>> {
        let classes = field(h, &key).list("ClassList").unwrap().to_vec();
        let sorcerer = classes.iter().find(|c| c.integer("Class") == Some(9)).unwrap().clone();
        (0..=9)
            .map(|l| {
                sorcerer
                    .list(&format!("KnownList{l}"))
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(|s| s.integer("Spell"))
                    .collect()
            })
            .collect()
    };
    h.run();
    h.get_by_label("Spells").click();
    h.run();
    let before = known(&mut h);
    assert!(before.iter().any(|l| !l.is_empty()), "the sorcerer knows spells");
    h.state_mut().dialogs =
        Box::new(mg_ui::NoDialogs { save: vec![path.clone()], ..Default::default() });
    h.get_by_label("Save Class Spell List").click();
    h.run();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("[Spells9]\r\nSpell000="), "{text}");
    h.get_by_label("Clear Class Spell List").click();
    h.run();
    assert!(known(&mut h).iter().all(Vec::is_empty));
    h.state_mut().dialogs = Box::new(mg_ui::NoDialogs { open: vec![path], ..Default::default() });
    h.get_by_label("Load Class Spell List").click();
    h.run();
    assert_eq!(known(&mut h), before, "each spell back at its level");
}

#[test]
fn setup_store_makes_the_conversation_script_and_store() {
    use mg_module::instances::{Placement, Placing, instance};
    use mg_module::new::{AreaSpec, add_area, new_module};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let game = mg_rules::GameData::open(&install).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&game, "Shop", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Field".into(),
        tileset: ResRef::from_str("ttr01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, &game, &spec, &mut rng).unwrap();
    // A hostile bandit at (5, 5).
    let bp =
        Gff::read(&game.resman.get(&ResKey::parse("nw_bandit001", ResType::UTC).unwrap()).unwrap())
            .unwrap();
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [5.0, 5.0, 0.0], rotation: 0.0 };
    let bandit = instance(&placing, ResType::UTC, &bp.root, at, &[]).unwrap();
    let git = ResKey::new(area, ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    g.root.set("Creature List", mg_gff::Value::List(vec![bandit]));
    m.set_gff(git, &g).unwrap();
    let path = mg_testkit::scratch_dir("ui-setup-store").join("shop.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let mut app = Moonglow::new(Some(install), Box::new(NoDialogs::default()));
    app.open_module(&path);
    app.open_palette = false;
    mg_ui::store_wizard::open(&mut app, area, "Creature List", 0);
    let mut h = harness(app, 1000.0, 800.0);
    h.run();
    // Aurora's defaults; then a standard store, and Finish.
    let w = h.state().store_wizard.clone().unwrap();
    assert_eq!((w.dialog.as_str(), w.script.as_str()), ("store001", "openstore001"));
    assert!(w.hostile, "the bandit's faction");
    h.get_by_label("Next >").click();
    h.run();
    type_into_hint(&mut h, "Find", "nw_storethief001");
    h.run();
    h.get_by_label_contains("(nw_storethief001)").click();
    h.run();
    h.get_by_label("Next >").click();
    h.run();
    h.get_all_by_label("Finish").last().unwrap().click();
    h.run();
    assert!(h.state().store_wizard.is_none(), "{:?}", h.state().log.entries);
    let ws = h.state_mut().ws.as_mut().unwrap();
    for (name, t) in
        [("store001", ResType::DLG), ("openstore001", ResType::NSS), ("openstore001", ResType::NCS)]
    {
        assert!(ws.module.contains(&ResKey::parse(name, t).unwrap()), "{name}.{t:?}");
    }
    let g = ws.doc(&git).unwrap();
    let c = &g.root.list("Creature List").unwrap()[0];
    assert_eq!(c.resref("Conversation"), Some(ResRef::from_str("store001").unwrap()));
    assert_eq!(c.integer("FactionID"), Some(3), "Merchant");
    let store = &g.root.list("StoreList").unwrap()[0];
    assert_eq!(store.resref("ResRef"), Some(ResRef::from_str("nw_storethief001").unwrap()));
    assert_eq!((store.float("XPosition"), store.float("YPosition")), (Some(5.0), Some(5.0)));
}

#[test]
fn the_levelup_wizard_levels_a_creature_up_as_aurora_does() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_lvl", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run();
    h.get_by_label("Levelup Wizard").click();
    h.run();
    // Fighter 1 to 5, as captured in Aurora (aurora_levelup.rs).
    h.state_mut().levelup.as_mut().expect("the wizard").slots[0].1 = 5;
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert!(h.state().levelup.is_none());
    let s = field(&mut h, &key);
    let level = s.list("ClassList").unwrap()[0].integer("ClassLevel");
    assert_eq!(level, Some(5));
    assert_eq!((s.integer("HitPoints"), s.integer("MaxHitPoints")), (Some(32), Some(42)));
    let feats: Vec<i64> =
        s.list("FeatList").unwrap().iter().filter_map(|f| f.integer("Feat")).collect();
    assert!(feats.ends_with(&[28, 6, 391]), "{feats:?}");
    // One undo step.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(field(&mut h, &key).list("ClassList").unwrap()[0].integer("ClassLevel"), Some(1));
}

#[test]
fn the_levelup_wizard_adds_a_class_with_its_spells_and_gear() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_wiz", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run();
    h.get_by_label("Levelup Wizard").click();
    h.run();
    // Wizard 1, as captured in Aurora on a placed bandit.
    h.state_mut().levelup.as_mut().expect("the wizard").slots.push((10, 1, 0));
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    let s = field(&mut h, &key);
    let wizard = &s.list("ClassList").unwrap()[1];
    let cantrips: Vec<i64> =
        wizard.list("MemorizedList0").unwrap().iter().filter_map(|e| e.integer("Spell")).collect();
    assert_eq!(cantrips, [37, 100, 144]);
    let bolts = s.list("Equip_ItemList").unwrap().iter().find(|e| e.id == 0x2000).cloned();
    assert_eq!(
        bolts.and_then(|e| e.resref("EquippedRes")),
        Some(ResRef::from_str("nw_wambo001").unwrap())
    );
    let carried: Vec<(String, i64, i64)> = s
        .list("ItemList")
        .unwrap()
        .iter()
        .map(|e| {
            let res = e.resref("InventoryRes").unwrap().to_string();
            (res, e.integer("Repos_PosX").unwrap(), e.integer("Repos_Posy").unwrap())
        })
        .collect();
    assert_eq!(carried.len(), 11, "{carried:?}");
    assert_eq!(carried[0], ("nw_wswdg001".to_string(), 0, 0));
}

#[test]
fn the_creature_wizard_makes_aurora_s_creature() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-creature-wizard");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.open_palette = false;
    let mut h = harness(app, 1000.0, 900.0);
    h.run();
    h.get_by_label("Wizards").click();
    h.run();
    h.get_by_label_contains("Creature Wizard…").click();
    h.run();
    // Each page as it comes, nothing drawn over anything else.
    let next = |h: &mut Harness<'_, Moonglow>| {
        let over = overlapping(h);
        assert!(over.is_empty(), "drawn over each other: {over:?}");
        h.get_by_label("Next >").click();
        h.run();
    };
    next(&mut h);
    h.get_by_label("Human").click();
    h.run();
    next(&mut h);
    // Fighter 1, the Human's default class.
    assert_eq!(h.state().creature_wizard.as_ref().unwrap().classes, [(4, 1)]);
    next(&mut h);
    // The page opens with the first portrait chosen.
    let first = h.state().creature_wizard.as_ref().unwrap().portrait;
    assert!(first.is_some(), "a portrait is chosen when the page opens");
    // The portraits are pictures (named after them once loaded).
    h.get_by_label("po_hu_m_01_").click();
    h.run();
    next(&mut h);
    next(&mut h); // Hostile
    let w = h.state_mut().creature_wizard.as_mut().unwrap();
    (w.first_name, w.last_name) = ("Hent".into(), "Fynolds".into());
    h.run();
    next(&mut h);
    h.get_by_label("Tutorial").click();
    h.run();
    next(&mut h); // the review
    next(&mut h);
    // The button (the last page's heading is "Finish" too).
    h.query_all_by_label("Finish")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .unwrap()
        .click();
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let c = ws.doc(&ResKey::parse("hent", ResType::UTC).unwrap()).unwrap().root.clone();
    assert_eq!((c.integer("HitPoints"), c.integer("MaxHitPoints")), (Some(10), Some(13)));
    let feats: Vec<i64> =
        c.list("FeatList").unwrap().iter().filter_map(|f| f.integer("Feat")).collect();
    assert_eq!(feats, [258, 46, 3, 4, 2, 32, 45, 1089, 28, 106, 10]);
    assert_eq!(c.integer("PortraitId"), Some(93));
}

#[test]
fn creature_names_can_be_random() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_named", ResType::UTC) else {
        return;
    };
    h.run();
    let name = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let l = s.locstring("FirstName").cloned().unwrap_or_default();
        l.strings.first().map(|(_, b)| String::from_utf8_lossy(b).into_owned())
    };
    let before = name(&mut h);
    h.get_all_by_label("🎲").next().unwrap().click();
    h.run();
    let after = name(&mut h).expect("a name");
    assert_ne!(Some(after.clone()), before);
    assert!((4..=13).contains(&after.len()), "{after}");
}

#[test]
fn the_spells_page_warns_about_invalid_assignments() {
    use mg_gff::{Struct, Value};
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_wizard", ResType::UTC)
    else {
        return;
    };
    // Wizard 1 with Bull's Strength (level 2) prepared.
    let mut spell = Struct::new(3);
    spell.set("Spell", Value::Word(9));
    spell.set("SpellMetaMagic", Value::Byte(0));
    spell.set("SpellFlags", Value::Byte(1));
    let mut wizard = Struct::new(2);
    wizard.set("Class", Value::Int(10));
    wizard.set("ClassLevel", Value::Short(1));
    wizard.set("MemorizedList2", Value::List(vec![spell]));
    let set = |label: &str, value: Value| mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(value),
    };
    let edits = vec![set("ClassList", Value::List(vec![wizard])), set("Int", Value::Byte(18))];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("wizard", edits)));
    h.run();
    h.get_by_label("Spells").click();
    h.run();
    let warning = "This creature has spells assigned to it that are too high for its current \
                   Wizard level.";
    assert!(h.query_by_label(warning).is_some(), "Aurora's warning, without its question");
    h.get_by_label("Never warn again").click();
    h.run();
    assert!(h.state().settings.no_spell_warning);
    assert!(h.query_by_label(warning).is_none());
}

#[test]
fn sound_blueprints_play_their_sounds() {
    use mg_ui::audio::{Channel, Silence};
    let Some((mut h, key)) = blueprint_harness("animalcriesday", "sound_play", ResType::UTS) else {
        return;
    };
    let speaker = std::rc::Rc::new(std::cell::RefCell::new(Silence::default()));
    h.state_mut().speaker = Box::new(speaker.clone());
    h.run();
    let s = field(&mut h, &key);
    let first = s.list("Sounds").unwrap()[0].resref("Sound").unwrap();
    let volume = s.integer("Volume").unwrap() as f32 / 127.0;
    // Play: the first sound (none selected), at the blueprint's volume.
    h.get_by_label("Play").click();
    h.run();
    assert_eq!(speaker.borrow().channels.get(&Channel::Preview), Some(&(first, volume, false)));
    h.get_by_label("Stop").click();
    h.run();
    assert!(speaker.borrow().channels.is_empty());
    // A sound that is not there goes to the log.
    assert!(!h.state_mut().play_sound(
        Channel::Preview,
        ResRef::from_str("no_such").unwrap(),
        1.0,
        false
    ));
    assert!(h.state().log.entries.iter().any(|(_, m)| m.contains("no_such not found")));
}

#[test]
fn a_creature_s_sound_set_plays_a_sample() {
    use mg_ui::audio::{Channel, Silence};
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_voice", ResType::UTC) else {
        return;
    };
    let speaker = std::rc::Rc::new(std::cell::RefCell::new(Silence::default()));
    h.state_mut().speaker = Box::new(speaker.clone());
    h.run();
    h.get_by_label("Advanced").click();
    h.run();
    h.get_by_label("▶").click();
    h.run();
    let played = speaker.borrow().channels.get(&Channel::Preview).cloned();
    assert!(played.is_some(), "{:?}", h.state().log.entries);
    // Female sound sets only: a male one is no longer offered.
    let name = |h: &Harness<'_, Moonglow>, row: usize| -> Option<String> {
        let game = h.state().game.as_deref().unwrap();
        let strref = game.table("soundset").unwrap().get_int(row, "STRREF")?;
        game.string(mg_core::StrRef(strref as u32))
    };
    let rows = |h: &Harness<'_, Moonglow>, gender: i32| -> Vec<String> {
        let t = h.state().game.as_deref().unwrap().table("soundset").unwrap();
        (0..t.len())
            .filter(|&r| t.get_int(r, "GENDER") == Some(gender))
            .filter_map(|r| name(h, r))
            .collect()
    };
    let (male, female) = (rows(&h, 0), rows(&h, 1));
    let male_only = male.iter().find(|m| !female.contains(m)).unwrap().clone();
    let set = field(&mut h, &key).integer("SoundSetFile").unwrap() as usize;
    let current = name(&h, set).unwrap();
    h.get_by_value("Both").click();
    h.run();
    h.get_by_label("Female").click();
    h.run();
    h.get_by_value(&current).click();
    h.run();
    assert!(h.query_by_label(&male_only).is_none(), "{male_only} offered");
    assert!(female.iter().any(|f| h.query_all_by_label(f).next().is_some()));
}

#[test]
fn portraits_are_chosen_from_their_pictures() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_face", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Portraits…").click();
    h.run();
    assert!(h.query_by_label("Select Portrait").is_some());
    let count = |h: &Harness<'_, Moonglow>| -> usize {
        let n = h.get_by_label_contains(" portraits");
        let a = n.accesskit_node();
        let text = a.label().or_else(|| a.value()).unwrap();
        text.split(' ').next().unwrap().parse().unwrap()
    };
    let creatures = count(&h);
    // The first creature portrait is portraits.2da row 1, a dwarf woman:
    // its picture is loaded (its name is the picture's).
    h.get_by_label("po_dw_f_01_").click();
    h.run();
    // Placeables and doors have their own.
    h.get_by_label("Placeable Objects and Doors").click();
    h.run();
    let things = count(&h);
    assert!(creatures > 100 && things > 100 && creatures != things, "{creatures} {things}");
    assert!(h.query_by_label("po_dw_f_02_").is_none());
    h.get_by_label("Characters and Creatures").click();
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("PortraitId"), Some(1));
    assert!(h.query_by_label("Select Portrait").is_none());
    // A look at the window (ignored test `screens`-style output).
    h.get_by_label("Portraits…").click();
    h.run();
    // (Rendering opens a GPU device.)
    mg_testkit::gpu::hold();
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-portraits").join("select.png"));
    }
}

#[test]
fn items_show_their_icons_and_choose_appearances_by_icon() {
    let Some((mut h, key)) = blueprint_harness("nw_it_gem001", "gem_icons", ResType::UTI) else {
        return;
    };
    h.run();
    h.get_by_label("Appearance").click();
    h.run();
    h.get_by_label("Icon");
    // (Rendering opens a GPU device.)
    mg_testkit::gpu::hold();
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::output_dir("ui-item-icons").join("gem.png"));
    }
    // The grid offers each model by its icon; a click takes it.
    let before = field(&mut h, &key).integer("ModelPart1").unwrap();
    let other = h
        .get_all_by_label_contains("Appearance ")
        .filter_map(|n| n.accesskit_node().label())
        .find(|l| l != &format!("Appearance {before}"))
        .expect("another appearance");
    h.get_by_label(&other).click();
    h.run();
    let n: i64 = other.trim_start_matches("Appearance ").parse().unwrap();
    assert_eq!(field(&mut h, &key).integer("ModelPart1"), Some(n));
    // A look at a weapon's and an armor's icons.
    let dir = mg_testkit::output_dir("ui-item-icons");
    for (bp, name) in [("nw_wswls001", "sword"), ("nw_aarcl001", "armor"), ("nw_arhe001", "helm")] {
        let Some((mut h, _)) = blueprint_harness(bp, &format!("icon_{name}"), ResType::UTI) else {
            return;
        };
        h.run();
        h.get_by_label("Appearance").click();
        h.run();
        h.get_by_label("Icon");
        if let Ok(img) = h.render() {
            let _ = img.save(dir.join(format!("{name}.png")));
        }
    }
}

#[test]
fn colours_are_chosen_from_palette_swatches() {
    let Some((mut h, key)) = blueprint_harness("nw_arhe001", "helm_colours", ResType::UTI) else {
        return;
    };
    h.run();
    h.get_by_label("Appearance").click();
    h.run();
    let cloth = field(&mut h, &key).integer("Cloth1Color").unwrap();
    let combo = egui::accesskit::Role::ComboBox;
    h.get_all_by_value(&cloth.to_string())
        .find(|n| n.accesskit_node().role() == combo)
        .expect("the Cloth 1 colour")
        .click();
    h.run();
    // (Rendering opens a GPU device.)
    mg_testkit::gpu::hold();
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-colours").join("swatches.png"));
    }
    let pick = if cloth == 5 { 6 } else { 5 };
    h.get_by_label(&format!("Cloth 1 {pick}")).click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Cloth1Color"), Some(pick));
}

/// The mouse wheel over a part's list steps through it without opening it
/// (a builder's request): down to the next part, up to the one before.
#[test]
fn the_mouse_wheel_steps_through_an_armor_s_parts() {
    let Some((mut h, key)) = blueprint_harness("nw_aarcl001", "part_wheel", ResType::UTI) else {
        return;
    };
    h.get_by_label("Appearance").click();
    h.run();
    let torso = |h: &mut Harness<'_, Moonglow>| field(h, &key).integer("ArmorPart_Torso");
    let before = torso(&mut h).expect("a torso");
    // The torso's list: beside its label, on its row.
    let label = h.get_by_label("Torso").rect();
    let list = h
        .get_all_by_role(egui::accesskit::Role::ComboBox)
        .map(|n| n.rect())
        .find(|r| r.min.x > label.max.x && (r.center().y - label.center().y).abs() < 6.0)
        .expect("the torso's list");
    let wheel = |h: &mut Harness<'_, Moonglow>, y: f32| {
        h.hover_at(list.center());
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, y),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(2);
    };
    wheel(&mut h, -50.0);
    let next = torso(&mut h).unwrap();
    assert!(next > before, "down: the next part ({before} → {next})");
    assert!(h.query_by_label(&format!("Torso {next}")).is_none(), "the list stays closed");
    wheel(&mut h, 50.0);
    assert_eq!(torso(&mut h), Some(before), "up: the one before");
    // Away from the list, the wheel scrolls the page and picks nothing.
    h.hover_at(label.center() - egui::vec2(0.0, 30.0));
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -50.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run_steps(2);
    assert_eq!(torso(&mut h), Some(before));
}

/// An armor's part can be 0, no part (a builder's report: it couldn't be
/// chosen, as the game's parts_bicep.2da leaves row 0's `ACBONUS` blank):
/// chosen in its list, it is saved as 0, its EE twin too.
#[test]
fn an_armor_s_part_can_be_none() {
    let Some((mut h, key)) = blueprint_harness("nw_aarcl001", "part_none", ResType::UTI) else {
        return;
    };
    h.get_by_label("Appearance").click();
    h.run();
    let bicep = |h: &mut Harness<'_, Moonglow>, label: &str| field(h, &key).integer(label);
    assert!(bicep(&mut h, "ArmorPart_LBicep").is_some_and(|n| n > 0), "a bicep to start with");
    let label = h.get_by_label("Left Bicep").rect();
    h.get_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| {
            let r = n.rect();
            r.min.x > label.max.x && (r.center().y - label.center().y).abs() < 6.0
        })
        .expect("the left bicep's list")
        .click();
    h.run();
    // (The open list's row; Belt and Robe show it in their boxes.)
    h.get_all_by_label("0 (none)")
        .find(|n| {
            n.accesskit_node().role() != egui::accesskit::Role::ComboBox
                && n.rect().min.y > label.max.y
        })
        .expect("the row of none")
        .click();
    h.run();
    assert_eq!(bicep(&mut h, "ArmorPart_LBicep"), Some(0));
    let wide = mg_rules::items::wide_label("ArmorPart_LBicep");
    assert_eq!(bicep(&mut h, &wide), Some(0), "its EE twin");
    // The list says so.
    assert!(h.get_all_by_value("0 (none)").next().is_some(), "the list shows none");
}

/// An armor whose torso is 0 (its `ACBONUS` blank, as in most of the
/// game's parts_chest.2da rows) is clothing, as the engine has it
/// (`engine_armor_torso.rs`): the General page shows its weight and armor
/// class 0, and prices it as armor.2da row 0 (a builder's report: no
/// weight, and the price of nothing).
#[test]
fn an_armor_whose_torso_is_0_is_clothing() {
    let Some((mut h, key)) = blueprint_harness("nw_aarcl001", "torso_none", ResType::UTI) else {
        return;
    };
    let set = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(value),
    };
    let edits = vec![
        set("ArmorPart_Torso", mg_gff::Value::Byte(0)),
        set("PropertiesList", mg_gff::Value::List(Vec::new())),
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Torso", edits)));
    h.run();
    // Each statistic's value, beside its label on its row.
    let stat = |h: &Harness<'_, Moonglow>, label: &str, value: &str| {
        let at = h.get_by_label(label).rect();
        let beside = h.get_all_by_label(value).any(|n| {
            let r = n.rect();
            r.min.x > at.max.x && (r.center().y - at.center().y).abs() < 8.0
        });
        assert!(beside, "{label}: {value}");
    };
    stat(&h, "Base Weight", "1.0 lbs");
    stat(&h, "Armor Class", "0");
    stat(&h, "Total Cost", "1");
}

#[test]
fn an_armor_s_parts_take_colors_of_their_own_and_keep_them() {
    use mg_gff::Value;
    let Some((mut h, key)) = blueprint_harness("nw_aarcl001", "part_colors", ResType::UTI) else {
        return;
    };
    // A color the armor came with (the left hand's Metal 2), as the game
    // writes it.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "part color",
        vec![mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "APart_17_Col_5".into(),
            value: Some(Value::Byte(171)),
        }],
    )));
    h.run();
    h.get_by_label("Appearance").click();
    h.run();
    // (Rendering opens a GPU device.)
    mg_testkit::gpu::hold();
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-part-colors").join("page.png"));
    }
    // The torso (shown first) has none of its own: each channel shows the
    // armor's. Its Cloth 1 is given one.
    let combo = egui::accesskit::Role::ComboBox;
    h.get_all_by_value("Default")
        .find(|n| n.accesskit_node().role() == combo)
        .expect("the torso's Cloth 1")
        .click();
    h.run();
    h.get_by_label("Torso Cloth 1 33").click();
    h.run();
    assert_eq!(field(&mut h, &key).get("APart_7_Col_2"), Some(&Value::Byte(33)));

    // Another edit, and a save: both parts' colors are in the saved file.
    let cloth = field(&mut h, &key).integer("Cloth1Color").unwrap();
    // (Its colors are in the right column; part numbers, some the same,
    // in the left.)
    h.get_all_by_value(&cloth.to_string())
        .find(|n| n.accesskit_node().role() == combo && n.rect().min.x > 590.0)
        .expect("the armor's Cloth 1")
        .click();
    h.run();
    let pick = if cloth == 5 { 6 } else { 5 };
    h.get_by_label(&format!("Cloth 1 {pick}")).click();
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-output/ui-bp-part_colors/sample.mod");
    let saved = mg_module::Module::open(&path).unwrap();
    let uti = saved.gff(&key).unwrap().unwrap().root;
    assert_eq!(uti.integer("Cloth1Color"), Some(pick));
    assert_eq!(uti.get("APart_7_Col_2"), Some(&Value::Byte(33)));
    assert_eq!(uti.get("APart_17_Col_5"), Some(&Value::Byte(171)));

    // Reset: the part takes the armor's color again, and the field goes.
    h.get_by_label("Reset Torso Cloth 1").click();
    h.run();
    assert_eq!(field(&mut h, &key).get("APart_7_Col_2"), None);
    assert_eq!(field(&mut h, &key).get("APart_17_Col_5"), Some(&Value::Byte(171)));
}

#[test]
fn sound_play_styles_set_aurora_s_fields_and_priority() {
    let Some((mut h, key)) = blueprint_harness("animalcriesday", "sound_styles", ResType::UTS)
    else {
        return;
    };
    h.run();
    let int = |h: &mut Harness<'_, Moonglow>, l: &str| field(h, &key).integer(l);
    h.get_by_label("Advanced").click();
    h.run();
    // Four sounds: no seamless looping (Aurora greys it).
    assert!(h.get_by_label("Seamlessly looping").accesskit_node().is_disabled());
    h.get_by_label("Once").click();
    h.run();
    assert_eq!((int(&mut h, "Looping"), int(&mut h, "Continuous")), (Some(0), Some(0)));
    assert_eq!(int(&mut h, "Priority"), Some(20), "a positional single shot");
    h.get_by_label("Positioning").click();
    h.run();
    h.get_by_label("Plays everywhere in area").click();
    h.run();
    assert_eq!(int(&mut h, "Priority"), Some(19), "an area-wide single shot");
}

#[test]
fn blueprint_editor_pages_draw_nothing_over_anything_else() {
    // Every page of every editor, roomy and narrow (a tab beside the
    // palette and tree).
    let editors = [
        ("nw_bandit001", ResType::UTC),
        ("nw_arhe001", ResType::UTI),
        ("plc_chest1", ResType::UTP),
        ("nw_door_ttr_01", ResType::UTD),
        ("trackstrigger", ResType::UTT),
        ("nw_verminbeet", ResType::UTE),
        ("nw_storebar01", ResType::UTM),
        ("animalcriesday", ResType::UTS),
        ("nw_waypoint001", ResType::UTW),
    ];
    let mut found = Vec::new();
    for (name, t) in editors {
        let copy = format!("over_{}", t.extension().unwrap_or("x"));
        let Some((mut h, key)) = blueprint_harness(name, &copy, t) else { return };
        if t == ResType::UTC {
            // A human of parts (the bandit is one model), for the body
            // parts on the Appearance page.
            h.run();
            h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
                "appearance",
                vec![mg_edit::Edit::SetField {
                    key,
                    path: mg_edit::GffPath::root(),
                    label: "Appearance_Type".into(),
                    value: Some(mg_gff::Value::Word(6)),
                }],
            )));
        }
        for size in [
            egui::vec2(1280.0, 800.0),
            egui::vec2(1000.0, 800.0),
            egui::vec2(800.0, 700.0),
            egui::vec2(600.0, 700.0),
        ] {
            h.set_size(size);
            for page in mg_ui::blueprint::pages(t) {
                h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), page);
                h.run();
                for o in overlapping(&h) {
                    found.push(format!("{t:?} {page} at {}: {o}", size.x));
                }
            }
        }
    }
    assert!(found.is_empty(), "drawn over each other:\n{}", found.join("\n"));
}

/// The Creature Wizard offers the racial types that have a name (not
/// racialtypes.2da's DELETED and INVALID_RACE rows), and a monster's race,
/// whose portraits are of no gender, has portraits to choose from.
#[test]
fn the_creature_wizard_offers_real_races_and_monsters_portraits() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-creature-wizard-races");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    app.open_palette = false;
    app.creature_wizard = Some(Default::default());
    let mut h = harness(app, 1000.0, 900.0);
    h.run();
    let next = |h: &mut Harness<'_, Moonglow>| {
        h.get_by_label("Next >").click();
        h.run();
    };
    next(&mut h);
    assert!(h.query_by_label("DELETED").is_none() && h.query_by_label("INVALID_RACE").is_none());
    h.get_by_label("Outsider").click();
    h.run();
    next(&mut h);
    next(&mut h);
    // Appearance and Portrait: one is chosen, and Next goes on.
    let w = h.state().creature_wizard.as_ref().unwrap();
    assert_eq!(w.race, Some(20));
    assert!(w.portrait.is_some() && !w.no_portraits, "an outsider has portraits");
    next(&mut h);
    assert_eq!(h.state().creature_wizard.as_ref().unwrap().page, 4);
}

/// A placeable's Properties open a gallery of every appearance in
/// placeables.2da at its own: a click on a neighbour gives it that one.
#[test]
fn a_placeable_s_appearance_is_chosen_from_pictures() {
    let Some((mut h, _area)) = area_harness("appearances") else { return };
    let key = ResKey::parse("my_chest", ResType::UTP).unwrap();
    {
        let app = h.state_mut();
        let stock = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let data = app.game.as_deref().unwrap().resman.get(&stock).unwrap().into_owned();
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "copy",
            vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
        )));
        app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    }
    h.run_steps(5);
    let was = field(&mut h, &key).integer("Appearance").unwrap();
    h.get_by_label("Gallery…").click();
    h.run_steps(30);
    // The row after its own, by its name in the table.
    let next = {
        let game = h.state().game.as_deref().unwrap();
        let cols = mg_rules::ChoiceColumns { name: Some("StrRef"), label: Some("Label") };
        let all = game.choices("placeables", cols).unwrap();
        let at = all.iter().position(|c| c.row as i64 == was).unwrap();
        all[at + 1].clone()
    };
    let img = h.render().expect("render");
    img.save(mg_testkit::scratch_dir("ui-appearances").join("appearances.png")).unwrap();
    let tile = |n: &egui_kittest::Node<'_>| n.rect().height() > 90.0;
    h.get_all_by_label(&next.text).find(tile).expect("the next appearance's picture").click();
    h.run_steps(3);
    assert_eq!(field(&mut h, &key).integer("Appearance"), Some(next.row as i64));
    assert!(h.state().dock.find_tab(&Tab::PlaceableGallery).is_some(), "the gallery stays, a tab");
    assert!(h.query_by_label_contains("A click gives my_chest.utp the appearance").is_some());
}

/// An item's Appearance page shows its model in the page, over the fields
/// in a window too narrow to have it beside them.
#[test]
fn an_item_s_appearance_page_shows_its_model() {
    let Some((mut h, _area)) = area_harness("item-model") else { return };
    let key = ResKey::parse("nw_wswls001", ResType::UTI).unwrap();
    {
        let app = h.state_mut();
        let data = app.game.as_ref().unwrap().resman.get(&key).unwrap();
        app.ws.as_mut().unwrap().view(key, mg_gff::Gff::read(&data).unwrap());
        app.blueprint_pages.insert((key, mg_edit::GffPath::root()), "Appearance");
        app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    }
    h.run_steps(10);
    // (The viewer's own buttons: it is there, and has the game's data.)
    assert!(h.query_by_label("Pop Out").is_some());
    assert!(h.query_by_label_contains("No game data").is_none());
    // Each of the sword's parts as pictures.
    assert!(h.query_all_by_label_contains("Top ").count() > 1);
}

/// A placeable's and a door's Basic page show the model in the page, as
/// Aurora's dialogs have a 3D preview.
#[test]
fn a_placeable_s_and_a_door_s_page_show_the_model() {
    let Some((mut h, _area)) = area_harness("situated-model") else { return };
    for (name, restype) in [("plc_chest1", ResType::UTP), ("nw_door_ttr_01", ResType::UTD)] {
        let key = ResKey::parse(name, restype).unwrap();
        {
            let app = h.state_mut();
            let data = app.game.as_ref().unwrap().resman.get(&key).unwrap();
            app.ws.as_mut().unwrap().view(key, mg_gff::Gff::read(&data).unwrap());
            app.blueprint_pages.insert((key, mg_edit::GffPath::root()), "Basic");
            // (As a tab of the middle: as wide as the window lets it be.)
            let kind = if restype == ResType::UTP { "placeable" } else { "door" };
            app.settings.window_places = vec![(kind.to_string(), "middle".to_string())];
            app.actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
        }
        // Too narrow for the fields and the model: a Preview button.
        h.set_size(egui::vec2(900.0, 800.0));
        h.run_steps(10);
        assert!(h.query_by_label("Pop Out").is_none(), "{key}: no room beside the fields");
        assert!(h.query_by_label("Preview").is_some(), "{key}");
        // Wide enough: the viewer beside the fields.
        h.set_size(egui::vec2(1900.0, 1000.0));
        h.run_steps(10);
        assert!(h.query_by_label("Pop Out").is_some(), "{key}: the viewer is in the page");
        assert!(h.query_by_label_contains("No game data").is_none(), "{key}");
        let tab = h.state().dock.find_tab(&Tab::Blueprint(key)).unwrap();
        h.state_mut().dock.remove_tab(tab);
        h.run_steps(3);
        assert!(h.query_by_label("Pop Out").is_none(), "{key}: closed again");
    }
}

/// Shown on a woman, an armor and a cloak are her models, and the armor's
/// inventory icon the one her inventory shows.
#[test]
fn armor_and_cloaks_are_shown_on_a_woman_when_asked() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let game = mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let read = |name: &str| {
        Gff::read(&game.resman.get(&ResKey::parse(name, ResType::UTI).unwrap()).unwrap())
            .unwrap()
            .root
    };
    let cloak = read("nw_maarcl055");
    let model = |female| mg_preview::item_on(&game, &cloak, female).unwrap().base.model;
    assert!(model(false).starts_with("pmh0_cloak_"), "{}", model(false));
    assert!(model(true).starts_with("pfh0_cloak_"), "{}", model(true));
    let armor = read("nw_aarcl001");
    let parts = |female| {
        let p = mg_preview::item_on(&game, &armor, female).unwrap();
        std::iter::once(p.base.model)
            .chain(p.parts.into_iter().map(|p| p.model))
            .collect::<Vec<_>>()
    };
    assert!(parts(false).iter().any(|m| m.starts_with("pmh0_")));
    assert!(parts(true).iter().any(|m| m.starts_with("pfh0_")), "{:?}", parts(true));
    assert!(!parts(true).iter().any(|m| m.starts_with("pmh0_chest")));
}

/// The pictures shown: images, and "(no picture)" where none could be
/// drawn (a harness without a GPU makes none).
fn pictures_shown(h: &Harness<'_, Moonglow>) -> usize {
    h.query_all_by_role(egui::accesskit::Role::Image).count()
        + h.query_all_by_label("(no picture)").count()
}

/// The pointer resting on the first list whose value `value` accepts:
/// how many more pictures that shows (the pointer moved off it first),
/// and how many more were made.
fn picture_on_hover(h: &mut Harness<'_, Moonglow>, value: &dyn Fn(&str) -> bool) -> (usize, usize) {
    h.hover_at(egui::pos2(1.0, 1.0));
    h.run_steps(10);
    let (without, made) = (pictures_shown(h), h.state().thumbnails.drawn);
    let at = h
        .get_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| n.accesskit_node().value().as_deref().is_some_and(value))
        .unwrap_or_else(|| panic!("no such list"))
        .rect()
        .center();
    h.hover_at(at);
    h.run_steps(90);
    (pictures_shown(h) - without, (h.state().thumbnails.drawn - made) as usize)
}

/// A door's Appearance Type and Generic Appearance lists show the door's
/// picture beside the row the pointer rests on, as a placeable's and a
/// creature's do (GitHub issue 7): here the chosen one's, the pointer on
/// the list's box.
#[test]
fn a_door_s_appearance_lists_show_its_picture() {
    let Some((mut h, key)) = blueprint_harness("nw_door_strong", "door_pictures", ResType::UTD)
    else {
        return;
    };
    h.run();
    let game = h.state().game.clone().unwrap();
    let name = |table: &str, name: &str, label: &str, row: i64| {
        let columns = mg_rules::ChoiceColumns { name: Some(name), label: Some(label) };
        let choices = game.choices(table, columns).unwrap();
        choices.into_iter().find(|c| c.row as i64 == row).unwrap().text
    };
    let door = field(&mut h, &key);
    let generic = door.integer("GenericType_New").or(door.integer("GenericType")).unwrap();
    let generic = name("genericdoors", "Name", "Label", generic);
    let (shown, made) = picture_on_hover(&mut h, &|v| v == generic);
    assert_eq!((shown, made), (1, 1), "the generic door's picture, made");
    // (The tileset's door, none: the generic one has no model of its own.)
    let kind = name("doortypes", "StringRefGame", "Label", 0);
    let (shown, _) = picture_on_hover(&mut h, &|v| v == kind);
    assert_eq!(shown, 1, "the door type's picture, or that it has none");
}

/// An item's Appearance lists show the item as it would be with the row
/// the pointer rests on (GitHub issue 7): a simple item's model, a
/// weapon's parts, an armor's parts; on a woman, where the page shows it
/// on one, a picture of its own.
#[test]
fn an_item_s_appearance_lists_show_the_item_with_each() {
    for (name, list) in [
        ("nw_it_mring001", &(|v: &str| v.parse::<u32>().is_ok()) as &dyn Fn(&str) -> bool),
        ("nw_wswls001", &|v: &str| v.starts_with("Model ")),
        ("nw_aarcl001", &|v: &str| v.contains("(AC ")),
    ] {
        let Some((mut h, key)) = blueprint_harness(name, &format!("p{}", &name[3..]), ResType::UTI)
        else {
            return;
        };
        h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), "Appearance");
        h.run_steps(4);
        // (Its icon too.)
        let (shown, made) = picture_on_hover(&mut h, list);
        assert_eq!((shown, made), (2, 1), "{name}: the item's picture, made");
        if name == "nw_aarcl001" {
            // On a woman: hers, made anew; on a man again, his, kept.
            h.get_by_label("Female").click();
            h.run_steps(4);
            let (shown, made) = picture_on_hover(&mut h, list);
            assert_eq!((shown, made), (2, 1), "the woman's picture, made");
            h.get_by_label("Male").click();
            h.run_steps(4);
            let (shown, made) = picture_on_hover(&mut h, list);
            assert_eq!((shown, made), (2, 0), "the man's, as it was made");
        }
    }
}

/// Shown on (the item's Appearance page) is Options › General's Show armor
/// and cloaks on a woman, kept in the settings, and the inventory icons of
/// armor are a woman's while it is on (the game has none of a cloak's: a
/// woman's inventory shows the man's).
#[test]
fn shown_on_a_woman_is_kept_and_the_icons_follow_it() {
    let Some((mut h, key)) = blueprint_harness("nw_aarcl001", "shown_on_kept", ResType::UTI) else {
        return;
    };
    h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), "Appearance");
    h.run_steps(4);
    let game = h.state().game.clone().unwrap();
    let read = |name: &str| {
        Gff::read(&game.resman.get(&ResKey::parse(name, ResType::UTI).unwrap()).unwrap())
            .unwrap()
            .root
    };
    let (armor, cloak) = (read("nw_aarcl001"), read("nw_maarcl055"));
    let names = |h: &mut Harness<'_, Moonglow>, item: &mg_gff::Struct| {
        let ctx = h.ctx.clone();
        let layers = h.state_mut().item_icon(&ctx, item);
        layers.iter().map(|p| p.texture.name()).collect::<Vec<_>>()
    };
    let his = names(&mut h, &armor);
    assert!(!his.is_empty() && his.iter().all(|n| n.starts_with("ipm_")), "{his:?}");
    h.get_by_label("Female").click();
    h.run_steps(4);
    assert!(h.state().settings.armor_on_woman, "kept in the settings");
    let hers = names(&mut h, &armor);
    assert_eq!(hers.len(), his.len());
    assert!(hers.iter().all(|n| n.starts_with("ipf_")), "{hers:?}");
    let cloak_icon = names(&mut h, &cloak);
    assert!(
        !cloak_icon.is_empty() && cloak_icon.iter().all(|n| n.starts_with("icloak_m_")),
        "{cloak_icon:?}"
    );
    // Options open with it, and set it back.
    let mut draft = mg_ui::OptionsDraft::from_settings(&h.state().settings);
    assert!(draft.armor_on_woman);
    draft.armor_on_woman = false;
    let settings = draft.apply(&h.state().settings);
    h.state_mut().settings = settings;
    h.run_steps(4);
    assert!(names(&mut h, &armor).iter().all(|n| n.starts_with("ipm_")));
}
