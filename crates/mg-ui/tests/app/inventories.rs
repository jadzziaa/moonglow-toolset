//! Inventories: what creatures, placeables and stores hold, and held
//! items' windows.

use crate::common::*;

#[test]
fn placeable_editor_fills_its_inventory() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_copy", ResType::UTP) else {
        return;
    };
    h.run();
    h.get_by_label("Inventory…").click();
    h.run();
    type_into_hint(&mut h, "Find", "nw_it_torch001");
    let torch = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button && n.label().is_some_and(|l| l == "Torch")
    };
    h.get(egui_kittest::kittest::by().predicate(torch)).click();
    h.run();
    for _ in 0..2 {
        h.get_by_label("Add Item").click();
        h.run();
    }
    // Torches are 1×3: the second goes beside the first.
    let items = field(&mut h, &key).list("ItemList").unwrap().to_vec();
    let at: Vec<_> = items
        .iter()
        .map(|i| {
            (
                i.resref("InventoryRes").unwrap().to_string(),
                i.integer("Repos_PosX").unwrap(),
                i.integer("Repos_Posy").unwrap(),
            )
        })
        .collect();
    assert_eq!(at, [("nw_it_torch001".into(), 0, 0), ("nw_it_torch001".into(), 1, 0)]);

    // Another item, to tell the rows apart: a potion.
    let potion = {
        let game = h.state().game.as_deref().unwrap();
        let key = ResKey::parse("nw_it_mpotion001", ResType::UTI).unwrap();
        let uti = Gff::read(&game.resman.get(&key).unwrap()).unwrap();
        game.locstring(uti.root.locstring("LocalizedName").unwrap()).unwrap()
    };
    let typed =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value("nw_it_torch001").find(typed).unwrap().click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("nw_it_torch001").find(typed).unwrap().type_text("nw_it_mpotion001");
    h.run();
    // (The palette's row is the leftmost of that name; the list's rows are
    // right of it, top to bottom.)
    let named = |h: &Harness<'_, Moonglow>, name: &str| -> Vec<egui::Rect> {
        let mut rects: Vec<egui::Rect> = h
            .get_all_by_label(name)
            .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
            .map(|n| n.rect())
            .collect();
        rects.sort_by(|a, b| a.left().total_cmp(&b.left()).then(a.top().total_cmp(&b.top())));
        rects
    };
    let at = named(&h, &potion)[0].center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run();
    h.get_by_label("Add Item").click();
    h.run();
    let held = |h: &mut Harness<'_, Moonglow>| -> Vec<String> {
        let items = field(h, &key).list("ItemList").unwrap().to_vec();
        items.iter().map(|i| i.resref("InventoryRes").unwrap().to_string()).collect()
    };
    assert_eq!(held(&mut h), ["nw_it_torch001", "nw_it_torch001", "nw_it_mpotion001"]);
    // Copy, from its row's menu; Paste, from the list's heading: another.
    let row = *named(&h, &potion).last().unwrap();
    h.hover_at(row.center());
    h.event(egui::Event::PointerButton {
        pos: row.center(),
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::PointerButton {
        pos: row.center(),
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    // The row's menu ends with Properties, as Aurora's menus do (Jasperre's
    // ask).
    assert_eq!(menu_items(&h, "Properties"), ["Copy", "Paste", "Remove", "Properties"]);
    h.get_by_label("Copy").click();
    h.run();
    assert_eq!(h.state().item_clip.len(), 1);
    h.get_by_label("Paste (1)").click();
    h.run();
    assert_eq!(held(&mut h).len(), 4);
    assert_eq!(held(&mut h)[3], "nw_it_mpotion001", "pasted");
    // The last row dragged onto the first: it takes that place (one step).
    let last = *named(&h, &potion).last().unwrap();
    let first =
        named(&h, "Torch").into_iter().find(|r| r.left() > 400.0).expect("the list's torch");
    let (from, to) = (last.center(), first.center());
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.run_steps(1);
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let order = held(&mut h);
    assert_eq!(order, ["nw_it_mpotion001", "nw_it_torch001", "nw_it_torch001", "nw_it_mpotion001"]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(held(&mut h)[0], "nw_it_torch001", "one undo puts it back");
    // An older blueprint's portrait is a resref (po_ and the portraits.2da
    // base resref); without a repute.fac, the standard factions.
    h.get_by_label("Advanced").click();
    h.run();
    assert!(h.query_by_value("PLC_A08_").is_some());
    assert!(h.query_by_value("Hostile").is_some());
    // A static placeable holds nothing: Has Inventory is disabled.
    h.get_by_label("Basic").click();
    h.run();
    h.get_by_label("Static").click();
    h.run();
    assert_eq!(field(&mut h, &key).integer("Static"), Some(1));
    assert!(h.get_by_label("Has Inventory").accesskit_node().is_disabled());
}

#[test]
fn placed_chest_holds_whole_items() {
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("held") else { return };
    let git_key = ResKey::new(area, ResType::GIT);
    {
        let app = h.state_mut();
        let game = app.game.as_deref().unwrap();
        let data = game.resman.get(&ResKey::parse("plc_chest1", ResType::UTP).unwrap()).unwrap();
        let mut chest = Gff::read(&data).unwrap().root;
        chest.set("HasInventory", mg_gff::Value::Byte(1));
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let at = Placement { position: [12.0, 30.0, 0.0], rotation: 0.0 };
        let item = instance(&placing, ResType::UTP, &chest, at, &[]).unwrap();
        let edit = mg_edit::Edit::InsertItem {
            key: git_key,
            path: mg_edit::GffPath::root(),
            list: "Placeable List".into(),
            index: 0,
            item,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
        let path = mg_edit::GffPath::root().item("Placeable List", 0);
        app.actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path }));
    }
    h.run_steps(3);
    h.get_by_label("Inventory…").click();
    h.run_steps(2);
    // The chest's item palette, not the Palettes pane's.
    close_tab(h.state_mut(), &Tab::Palette);
    h.run_steps(2);
    type_into_hint(&mut h, "Find", "nw_it_torch001");
    let torch = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::Button && n.label().is_some_and(|l| l == "Torch")
    };
    h.get(egui_kittest::kittest::by().predicate(torch)).click();
    h.run_steps(2);
    for _ in 0..2 {
        h.get_by_label("Add Item").click();
        h.run_steps(2);
    }
    let ws = h.state_mut().ws.as_mut().unwrap();
    let chest = ws.doc(&git_key).unwrap().root.list("Placeable List").unwrap()[0].clone();
    let items = chest.list("ItemList").unwrap();
    let at: Vec<_> = items
        .iter()
        .map(|i| {
            (
                i.id,
                i.resref("TemplateResRef").unwrap().to_string(),
                i.integer("BaseItem").is_some(),
                i.integer("Repos_PosX").unwrap(),
                i.float("XPosition"),
            )
        })
        .collect();
    let whole = |id, x| (id, "nw_it_torch001".to_string(), true, x, Some(-1.0));
    assert_eq!(at, [whole(0, 0), whole(1, 1)], "whole items, as Aurora holds them");
    assert!(h.query_all_by_label("Torch").count() >= 2, "named by the items themselves");
}

#[test]
fn equipping_without_the_feat_asks_to_add_it() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_unskilled", ResType::UTC)
    else {
        return;
    };
    let edit = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: "FeatList".into(),
        value: Some(mg_gff::Value::List(Vec::new())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("no feats", vec![edit])));
    h.run();
    h.get_by_label("Inventory").click();
    h.run();
    assert!(h.query_by_label_contains("The game may unequip items").is_some(), "the notice");
    type_into_hint(&mut h, "Find", "nw_waxbt001");
    h.get_by_label("Battleaxe").click();
    h.run();
    let primary = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        let list = s.list("Equip_ItemList").unwrap_or(&[]).to_vec();
        list.iter().find(|s| s.id == 0x10).map(|s| s.resref("EquippedRes").unwrap().to_string())
    };
    let feats = |h: &mut Harness<'_, Moonglow>| -> Vec<i64> {
        let s = field(h, &key);
        s.list("FeatList").unwrap_or(&[]).iter().filter_map(|f| f.integer("Feat")).collect()
    };
    let before = primary(&mut h);
    // The Primary Weapon slot's Equip (the sixth: the palette's Equip is first): No leaves it as it was.
    h.get_all_by_label("Equip").nth(5).unwrap().click();
    h.run();
    assert!(h.query_by_label_contains("Weapon Proficiency (martial)").is_some());
    h.get_by_label("No").click();
    h.run();
    assert_eq!(primary(&mut h), before);
    assert!(feats(&mut h).is_empty());
    // Yes adds the first feat and equips it, one undoable step.
    h.get_all_by_label("Equip").nth(5).unwrap().click();
    h.run();
    h.get_by_label("Yes").click();
    h.run();
    assert_eq!(primary(&mut h).as_deref(), Some("nw_waxbt001"));
    assert_eq!(feats(&mut h), [45]);
    // The slot shows the axe's icon.
    let image = egui::accesskit::Role::Image;
    assert!(h.get_all_by_label("Battleaxe").any(|n| n.accesskit_node().role() == image));
    // (Rendering opens a GPU device.)
    mg_testkit::gpu::hold();
    if let Ok(img) = h.render() {
        let _ = img.save(mg_testkit::scratch_dir("ui-inventory-icons").join("inventory.png"));
    }
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert!(feats(&mut h).is_empty());
    assert_eq!(primary(&mut h), before);
}

#[test]
fn creature_items_can_be_dropable_and_pickpocketable() {
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_flags", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Inventory").click();
    h.run();
    let flag = |h: &mut Harness<'_, Moonglow>, label: &str| {
        field(h, &key).list("Equip_ItemList").unwrap()[0].integer(label)
    };
    assert_eq!(flag(&mut h, "Dropable"), None, "unset, as the game's blueprints leave it");
    h.get_by_label_contains("Selected Item: none");
    // Aurora's checkboxes are there before an item is selected, grayed.
    for label in ["Dropable", "Pickpocketable"] {
        assert!(h.get_by_label(label).accesskit_node().is_disabled(), "{label}");
    }
    // Select the armor (its name, not its icon), as in Aurora's inventory.
    let image = egui::accesskit::Role::Image;
    let pick = |h: &mut Harness<'_, Moonglow>, label: &str, modifiers: egui::Modifiers| {
        let node = h.get_all_by_label(label).find(|n| n.accesskit_node().role() != image);
        node.unwrap().click_modifiers(modifiers);
        h.run();
    };
    pick(&mut h, "Padded Armor", egui::Modifiers::NONE);
    h.get_by_label("Selected Item: Padded Armor");
    h.get_by_label("Dropable").click();
    h.run();
    assert_eq!(flag(&mut h, "Dropable"), Some(1));
    h.get_by_label("Dropable").click();
    h.run();
    assert_eq!(flag(&mut h, "Dropable"), None, "cleared: left out again");
    h.get_by_label("Pickpocketable").click();
    h.run();
    assert_eq!(flag(&mut h, "Pickpocketable"), Some(1));
    // With the sword too (Ctrl), Pickpocketable is mixed; a click sets
    // both, as one step.
    pick(&mut h, "Longsword", egui::Modifiers::COMMAND);
    h.get_by_label("Selected Items: 2");
    let toggled = |h: &mut Harness<'_, Moonglow>, label: &str| {
        h.get_by_label(label).accesskit_node().toggled()
    };
    use egui::accesskit::Toggled;
    assert_eq!(toggled(&mut h, "Pickpocketable"), Some(Toggled::Mixed));
    assert_eq!(toggled(&mut h, "Dropable"), Some(Toggled::False));
    let both = |h: &mut Harness<'_, Moonglow>, label: &str| -> Vec<Option<i64>> {
        let s = field(h, &key);
        s.list("Equip_ItemList").unwrap().iter().map(|e| e.integer(label)).collect()
    };
    h.get_by_label("Pickpocketable").click();
    h.run();
    assert_eq!(both(&mut h, "Pickpocketable"), [Some(1), Some(1)]);
    assert_eq!(toggled(&mut h, "Pickpocketable"), Some(Toggled::True));
    h.get_by_label("Dropable").click();
    h.run();
    assert_eq!(both(&mut h, "Dropable"), [Some(1), Some(1)]);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run();
    assert_eq!(both(&mut h, "Dropable"), [None, None], "both taken back by one Undo");
    // Saved: the bytes in the file.
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run();
    let file = h.state().ws.as_ref().unwrap().module.location.as_ref().unwrap().path().to_owned();
    let saved = Module::open(&file).unwrap().gff(&key).unwrap().unwrap().root;
    let items = saved.list("Equip_ItemList").unwrap();
    let byte = |i: usize, label: &str| items[i].get(label).cloned();
    assert_eq!(byte(0, "Pickpocketable"), Some(mg_gff::Value::Byte(1)));
    assert_eq!(byte(1, "Pickpocketable"), Some(mg_gff::Value::Byte(1)));
    assert_eq!((byte(0, "Dropable"), byte(1, "Dropable")), (None, None));
}

/// A placed creature's items (whole items in the area's GIT) have
/// Aurora's Dropable and Pickpocketable too, set on the item the creature
/// holds there.
#[test]
fn a_placed_creature_s_items_can_be_dropable_and_pickpocketable() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("held-flags") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let bandit = {
        let game = h.state().game.clone().unwrap();
        let read = |name: ResRef, t: ResType| {
            let data = game.resman.get(&ResKey::new(name, t)).ok()?;
            Some(Gff::read(&data).ok()?.root)
        };
        let items = |r: ResRef| read(r, ResType::UTI);
        let placing = Placing { game: &game, item: &items };
        let at = Placement { position: [12.0, 12.0, 0.0], rotation: 0.0 };
        let utc = read(ResRef::from_str("nw_bandit001").unwrap(), ResType::UTC).unwrap();
        instance(&placing, ResType::UTC, &utc, at, &[]).unwrap()
    };
    let placed = Edit::InsertItem {
        key: git,
        path: GffPath::root(),
        list: "Creature List".into(),
        index: 0,
        item: bandit,
    };
    h.state_mut().settings.no_inventory_warning = true;
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    let path = GffPath::root().item("Creature List", 0);
    h.state_mut().blueprint_pages.insert((git, path.clone()), "Inventory");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path: path.clone() }));
    h.run_steps(6);
    // (A window wide enough for the slots' items to show.)
    let tab = Tab::Instance { area, path: path.clone() };
    let at = h.state().dock.find_tab(&tab).unwrap();
    if let Some(w) = h.state_mut().dock.get_window_state_mut(at.surface) {
        w.set_position(egui::pos2(20.0, 60.0)).set_size(egui::vec2(1000.0, 700.0));
    }
    h.run_steps(4);
    let image = egui::accesskit::Role::Image;
    h.get_all_by_label("Padded Armor")
        .find(|n| n.accesskit_node().role() != image)
        .unwrap()
        .click();
    h.run_steps(2);
    h.get_by_label("Selected Item: Padded Armor");
    h.get_by_label("Pickpocketable").click();
    h.run_steps(2);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let creature = path.get(&ws.doc(&git).unwrap().root).cloned().unwrap();
    let armor = creature.list("Equip_ItemList").unwrap().iter().find(|e| e.id == 0x2).unwrap();
    assert_eq!(armor.get("Pickpocketable"), Some(&mg_gff::Value::Byte(1)));
    assert_eq!(armor.get("Dropable"), None);
    assert!(armor.integer("BaseItem").is_some(), "the whole item the creature holds");
}

/// Properties on an item in a placed object's inventory opens that item's
/// own Properties (the item as the object holds it), as in Aurora; what is
/// changed there is changed in the object's inventory.
#[test]
fn a_held_item_s_properties_open_from_the_inventory() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("held-item") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let chest = {
        let game = h.state().game.clone().unwrap();
        let read = |name: &str, t: ResType| {
            Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
        };
        let none = |_: ResRef| None;
        let placing = Placing { game: &game, item: &none };
        let at = Placement { position: [12.0, 12.0, 0.0], rotation: 0.0 };
        let mut chest =
            instance(&placing, ResType::UTP, &read("plc_chest1", ResType::UTP), at, &[]).unwrap();
        let mut sword = read("nw_wswls001", ResType::UTI);
        sword.set("Repos_PosX", mg_gff::Value::Word(0));
        sword.set("Repos_Posy", mg_gff::Value::Word(0));
        chest.set("HasInventory", mg_gff::Value::Byte(1));
        chest.set("ItemList", mg_gff::Value::List(vec![sword]));
        chest
    };
    let placed = Edit::InsertItem {
        key: git,
        path: GffPath::root(),
        list: "Placeable List".into(),
        index: 0,
        item: chest,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    let path = GffPath::root().item("Placeable List", 0);
    h.state_mut().blueprint_pages.insert((git, path.clone()), "Inventory");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path: path.clone() }));
    h.run_steps(6);
    // (The one held, right of the one in the palette beside it.)
    let row = h
        .query_all_by_label("Longsword")
        .map(|n| n.rect().center())
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    h.hover_at(row);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Properties").click();
    h.run_steps(6);
    let held = path.item("ItemList", 0);
    let tab = Tab::Instance { area, path: held.clone() };
    assert!(h.state().dock.find_tab(&tab).is_some(), "{:?}", h.state().log.entries);
    // An item's pages, for the item the chest holds.
    assert!(h.query_all_by_label("Appearance").count() >= 1);
    // Changed there, it is changed in the chest.
    let set = Edit::SetField {
        key: git,
        path: held.clone(),
        label: "Charges".into(),
        value: Some(mg_gff::Value::Byte(7)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Charges", vec![set])));
    h.run_steps(4);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let item = held.get(&ws.doc(&git).unwrap().root).cloned().unwrap();
    assert_eq!(item.integer("Charges"), Some(7));
    assert!(item.integer("Cost").is_some());
}

/// Places a chest holding three longswords tagged `ITEM_A`, `ITEM_B` and
/// `ITEM_C` in the harness's area: its path.
fn chest_of_three(h: &mut Harness<'_, Moonglow>, area: ResRef) -> mg_edit::GffPath {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let game = h.state().game.clone().unwrap();
    let read = |name: &str, t: ResType| {
        Gff::read(&game.resman.get(&ResKey::parse(name, t).unwrap()).unwrap()).unwrap().root
    };
    let none = |_: ResRef| None;
    let placing = Placing { game: &game, item: &none };
    let at = Placement { position: [12.0, 12.0, 0.0], rotation: 0.0 };
    let mut chest =
        instance(&placing, ResType::UTP, &read("plc_chest1", ResType::UTP), at, &[]).unwrap();
    let items = ["ITEM_A", "ITEM_B", "ITEM_C"]
        .into_iter()
        .enumerate()
        .map(|(i, tag)| {
            let mut sword = read("nw_wswls001", ResType::UTI);
            sword.id = i as u32;
            sword.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
            sword.set("Repos_PosX", mg_gff::Value::Word(i as u16));
            sword.set("Repos_Posy", mg_gff::Value::Word(0));
            sword
        })
        .collect();
    chest.set("HasInventory", mg_gff::Value::Byte(1));
    chest.set("ItemList", mg_gff::Value::List(items));
    let placed = Edit::InsertItem {
        key: ResKey::new(area, ResType::GIT),
        path: GffPath::root(),
        list: "Placeable List".into(),
        index: 0,
        item: chest,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    h.run_steps(2);
    GffPath::root().item("Placeable List", 0)
}

/// Whether a field shows `tag` (the Tag of an item's General page).
fn shows_tag(h: &Harness<'_, Moonglow>, tag: &str) -> bool {
    let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some(tag)
    };
    h.query(egui_kittest::kittest::by().predicate(is)).is_some()
}

/// The window of a held item open (`area`'s, at `path`) and showing the
/// item tagged `tag` (its Tag field): where it is.
fn held_window(h: &Harness<'_, Moonglow>, area: ResRef, tag: &str) -> mg_edit::GffPath {
    let found = h.state().dock.iter_all_tabs().find_map(|(_, t)| match t {
        Tab::Instance { area: a, path } if *a == area && path.0.len() > 1 => Some(path.clone()),
        _ => None,
    });
    let path = found.expect("the item's window is open");
    assert!(shows_tag(h, tag), "it shows {tag}");
    path
}

/// A held item's Properties follow the item (Undo and Redo too) as items
/// are added, removed or moved before it in its inventory, and close
/// when it is removed; their Cancel sets back the item where it went.
#[test]
fn a_held_item_s_properties_follow_the_item() {
    use mg_edit::{Command, Edit};
    let Some((mut h, area)) = area_harness("held-follow") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let chest = chest_of_three(&mut h, area);
    let open = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let path = chest.item("ItemList", i);
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path }));
        h.run_steps(4);
    };
    let apply = |h: &mut Harness<'_, Moonglow>, what: &str, edits: Vec<Edit>| {
        h.state_mut().actions.push(mg_ui::Action::Apply(Command::new(what, edits)));
        h.run_steps(4);
    };
    let held = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        chest.get(&ws.doc(&git).unwrap().root).unwrap().list("ItemList").unwrap().to_vec()
    };
    let at = |i: usize| chest.item("ItemList", i);
    let remove = |i: usize| Edit::remove_item(git, chest.clone(), "ItemList", i);
    open(&mut h, 1);
    assert_eq!(held_window(&h, area, "ITEM_B"), at(1));
    // Changed in its window (for its Cancel, below).
    h.get_by_label("Plot Item").click();
    h.run_steps(3);
    assert_eq!(held(&mut h)[1].integer("Plot"), Some(1));
    // The one before it removed: the window follows it up the list.
    apply(&mut h, "Remove item", vec![remove(0)]);
    assert_eq!(held_window(&h, area, "ITEM_B"), at(0));
    assert!(!shows_tag(&h, "ITEM_C"));
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(4);
    assert_eq!(held_window(&h, area, "ITEM_B"), at(1), "undone");
    h.state_mut().actions.push(mg_ui::Action::Redo);
    h.run_steps(4);
    assert_eq!(held_window(&h, area, "ITEM_B"), at(0), "redone");
    // One added before it, and it moved to the end (as Move item does).
    let mut added = held(&mut h)[1].clone();
    added.set("Tag", mg_gff::Value::String(b"ITEM_D".to_vec()));
    apply(&mut h, "Add item", vec![Edit::insert_item(git, chest.clone(), "ItemList", 0, added)]);
    assert_eq!(held_window(&h, area, "ITEM_B"), at(1));
    let b = held(&mut h)[1].clone();
    let moved = vec![remove(1), Edit::insert_item(git, chest.clone(), "ItemList", 2, b)];
    apply(&mut h, "Move item", moved);
    assert_eq!(held_window(&h, area, "ITEM_B"), at(2), "moved");
    // Cancel sets back the item it shows, where it is now, and no other.
    h.get_by_label("Cancel").click();
    h.run_steps(4);
    let items = held(&mut h);
    let tags: Vec<_> = items.iter().map(|s| s.string("Tag").unwrap().to_vec()).collect();
    assert_eq!(tags, [&b"ITEM_D"[..], b"ITEM_C", b"ITEM_B"]);
    assert_eq!(items[2].integer("Plot"), Some(0), "set back");
    assert_eq!(items[1].integer("Plot"), items[0].integer("Plot"), "the others as they were");
    assert!(!shows_tag(&h, "ITEM_B"), "closed");
    // Removed, its window closes (as a placed object's when the object is
    // deleted).
    open(&mut h, 2);
    held_window(&h, area, "ITEM_B");
    apply(&mut h, "Remove item", vec![remove(2)]);
    let open_items = h.state().dock.iter_all_tabs().filter(
        |(_, t)| matches!(t, Tab::Instance { area: a, path } if *a == area && path.0.len() > 1),
    );
    assert_eq!(open_items.count(), 0, "closed with its item");
}

/// A placed store's items open their own Properties too (a page's item,
/// right click › Properties), which follow the item as the page changes.
#[test]
fn a_placed_store_s_items_open_their_properties_and_follow_them() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("held-store") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let store = {
        let game = h.state().game.clone().unwrap();
        let read = |name: ResRef, t: ResType| {
            let data = game.resman.get(&ResKey::new(name, t)).ok()?;
            Some(Gff::read(&data).ok()?.root)
        };
        let items = |r: ResRef| read(r, ResType::UTI);
        let placing = Placing { game: &game, item: &items };
        let at = Placement { position: [12.0, 12.0, 0.0], rotation: 0.0 };
        let utm = read(ResRef::from_str("nw_storebar01").unwrap(), ResType::UTM).unwrap();
        instance(&placing, ResType::UTM, &utm, at, &[]).unwrap()
    };
    // The page with the bar's drinks, and the second of them.
    let (page, items) = store
        .list("StoreList")
        .unwrap()
        .iter()
        .enumerate()
        .find_map(|(i, p)| Some((i, p.list("ItemList").filter(|l| l.len() > 1)?.to_vec())))
        .unwrap();
    assert!(items[1].integer("BaseItem").is_some(), "a placed store holds whole items");
    let tag = String::from_utf8_lossy(items[1].string("Tag").unwrap()).into_owned();
    let game = h.state().game.clone().unwrap();
    let name = game.locstring(items[1].locstring("LocalizedName").unwrap()).unwrap();
    let placed = Edit::InsertItem {
        key: git,
        path: GffPath::root(),
        list: "StoreList".into(),
        index: 0,
        item: store,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    let path = GffPath::root().item("StoreList", 0);
    h.state_mut().blueprint_pages.insert((git, path.clone()), "Inventory");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path: path.clone() }));
    h.run_steps(6);
    let row = h
        .query_all_by_label_contains(&name)
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .map(|n| n.rect().center())
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    h.hover_at(row);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Properties").click();
    h.run_steps(6);
    let page = path.item("StoreList", page);
    assert_eq!(
        held_window(&h, area, &tag),
        page.item("ItemList", 1),
        "{:?}",
        h.state().log.entries
    );
    assert!(h.query_all_by_label("Appearance").count() >= 1, "an item's pages");
    // The first of the page's items removed: the window follows.
    let remove = Edit::remove_item(git, page.clone(), "ItemList", 0);
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Remove item", vec![remove])));
    h.run_steps(4);
    assert_eq!(held_window(&h, area, &tag), page.item("ItemList", 0));
}

/// A placed creature's Properties open about 480 points wide beside the
/// area: on its Inventory page, the slots' items are in sight at that
/// width (once they were laid out past the window's right edge; Sacha's
/// report on 2.5.4).
#[test]
fn a_placed_creature_s_equipment_is_in_sight_in_a_narrow_window() {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let Some((mut h, area)) = area_harness("equipment-narrow") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let bandit = {
        let game = h.state().game.clone().unwrap();
        let read = |name: ResRef, t: ResType| {
            let data = game.resman.get(&ResKey::new(name, t)).ok()?;
            Some(Gff::read(&data).ok()?.root)
        };
        let items = |r: ResRef| read(r, ResType::UTI);
        let placing = Placing { game: &game, item: &items };
        let at = Placement { position: [12.0, 12.0, 0.0], rotation: 0.0 };
        let utc = read(ResRef::from_str("nw_bandit001").unwrap(), ResType::UTC).unwrap();
        instance(&placing, ResType::UTC, &utc, at, &[]).unwrap()
    };
    let placed = Edit::InsertItem {
        key: git,
        path: GffPath::root(),
        list: "Creature List".into(),
        index: 0,
        item: bandit,
    };
    h.state_mut().settings.no_inventory_warning = true;
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    let path = GffPath::root().item("Creature List", 0);
    h.state_mut().blueprint_pages.insert((git, path.clone()), "Inventory");
    let tab = Tab::Instance { area, path };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(6);
    let window = egui::Rect::from_min_size(egui::pos2(20.0, 60.0), egui::vec2(480.0, 672.0));
    let at = h.state().dock.find_tab(&tab).unwrap();
    if let Some(w) = h.state_mut().dock.get_window_state_mut(at.surface) {
        w.set_position(window.min).set_size(window.size());
    }
    h.run_steps(4);
    for item in ["Padded Armor", "Longsword"] {
        let name = h
            .get_all_by_label(item)
            .find(|n| n.accesskit_node().role() != egui::accesskit::Role::Image)
            .unwrap()
            .rect();
        assert!(name.right() < window.right(), "{item} at {name:?}, in a window at {window:?}");
        assert!(name.left() > window.left() + 200.0, "{item}: right of the palette");
    }
}

/// Places a bandit (armor and longsword equipped) holding `held` in its
/// backpack in the harness's area, and opens its Properties on the
/// Inventory page, in a window at the left: its path. `held` is given the
/// game and makes the backpack's items.
fn bandit_holding(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    held: &dyn Fn(&mg_rules::GameData) -> Vec<mg_gff::Struct>,
) -> mg_edit::GffPath {
    use mg_edit::{Command, Edit, GffPath};
    use mg_module::instances::{Placement, Placing, instance};
    let game = h.state().game.clone().unwrap();
    let mut bandit = {
        let read = |name: ResRef, t: ResType| {
            let data = game.resman.get(&ResKey::new(name, t)).ok()?;
            Some(Gff::read(&data).ok()?.root)
        };
        let items = |r: ResRef| read(r, ResType::UTI);
        let placing = Placing { game: &game, item: &items };
        let at = Placement { position: [12.0, 12.0, 0.0], rotation: 0.0 };
        let utc = read(ResRef::from_str("nw_bandit001").unwrap(), ResType::UTC).unwrap();
        instance(&placing, ResType::UTC, &utc, at, &[]).unwrap()
    };
    bandit.set("ItemList", mg_gff::Value::List(held(&game)));
    let placed = Edit::InsertItem {
        key: ResKey::new(area, ResType::GIT),
        path: GffPath::root(),
        list: "Creature List".into(),
        index: 0,
        item: bandit,
    };
    h.state_mut().settings.no_inventory_warning = true;
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Setup", vec![placed])));
    let path = GffPath::root().item("Creature List", 0);
    let git = ResKey::new(area, ResType::GIT);
    h.state_mut().blueprint_pages.insert((git, path.clone()), "Inventory");
    let tab = Tab::Instance { area, path: path.clone() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.set_size(egui::vec2(1500.0, 1400.0));
    h.run_steps(6);
    place_window(h, &tab, egui::pos2(10.0, 40.0), egui::vec2(900.0, 1320.0));
    path
}

/// Puts the window `tab` is in at `at`, `size` big.
fn place_window(h: &mut Harness<'_, Moonglow>, tab: &Tab, at: egui::Pos2, size: egui::Vec2) {
    let found = h.state().dock.find_tab(tab).unwrap();
    if let Some(w) = h.state_mut().dock.get_window_state_mut(found.surface) {
        w.set_position(at).set_size(size);
    }
    h.run_steps(4);
}

/// A whole item of blueprint `resref` as a placed creature's backpack
/// holds one, item `id` of it, its own: tagged `tag`, named `name`,
/// dropable and pickpocketable, with a field the game doesn't know.
fn own_item(
    game: &mg_rules::GameData,
    resref: &str,
    id: u32,
    (tag, name): (&str, &str),
) -> mg_gff::Struct {
    let read = |r: ResRef| {
        let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
        Some(Gff::read(&data).ok()?.root)
    };
    let placing = mg_module::instances::Placing { game, item: &read };
    let bp = read(ResRef::from_str(resref).unwrap()).unwrap();
    let mut item = mg_module::instances::held(&placing, &bp, id);
    item.set("Tag", mg_gff::Value::String(tag.as_bytes().to_vec()));
    let name = LocString::from_text(Language::ENGLISH, Gender::Male, name);
    item.set("LocalizedName", mg_gff::Value::LocString(name));
    item.set("Dropable", mg_gff::Value::Byte(1));
    item.set("Pickpocketable", mg_gff::Value::Byte(1));
    item.set("MG_Probe", mg_gff::Value::Int(7));
    item.set("Repos_PosX", mg_gff::Value::Word(id as u16));
    item.set("Repos_Posy", mg_gff::Value::Word(0));
    item
}

/// Opens the context menu of the button labeled `label` farthest left
/// (the creature window's, at the left) and clicks its item `choice`.
fn menu_of(h: &mut Harness<'_, Moonglow>, label: &str, choice: &str) {
    let row = h
        .query_all_by_label(label)
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .map(|n| n.rect().center())
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap_or_else(|| panic!("no {label} to click"));
    h.hover_at(row);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: row,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    // (The menu's: the one nearest below where it was opened.)
    let item = h
        .query_all_by_label(choice)
        .map(|n| n.rect())
        .filter(|r| r.top() >= row.y - 4.0)
        .min_by(|a, b| a.center().distance(row).total_cmp(&b.center().distance(row)))
        .unwrap_or_else(|| panic!("no {choice} in the menu"));
    click_pos(h, item.center());
    h.run_steps(6);
}

/// The two lists of the creature at `path` in `area`'s GIT: its equipment
/// and its backpack.
fn creature_items(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    path: &mg_edit::GffPath,
) -> (Vec<mg_gff::Struct>, Vec<mg_gff::Struct>) {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    let c = path.get(&git.root).unwrap();
    let list = |l: &str| c.list(l).unwrap_or(&[]).to_vec();
    (list("Equip_ItemList"), list("ItemList"))
}

/// `item` without where it is held (struct id, place in the grid), to
/// compare an item moved with itself.
fn as_held_anywhere(item: &mg_gff::Struct) -> mg_gff::Struct {
    let mut s = item.clone();
    s.id = 0;
    s.remove("Repos_PosX");
    s.remove("Repos_Posy");
    s
}

/// A placed creature's own item equipped from its backpack, and put back
/// in it, stays as it is (Aurora moves the item itself: its own name,
/// tag, flags, fields Moonglow keeps besides), one step each; its
/// Properties window follows it. An item no blueprint is left of moves
/// too.
#[test]
fn a_held_item_moved_to_a_slot_and_back_stays_as_it_is() {
    let Some((mut h, area)) = area_harness("held-equip") else { return };
    let held = |game: &mg_rules::GameData| {
        let mut gone = own_item(game, "nw_it_mring022", 1, ("MG_NOWHERE", "Nowhere Probe"));
        gone.set("TemplateResRef", mg_gff::Value::resref(ResRef::from_str("mg_gone").unwrap()));
        vec![own_item(game, "nw_it_mneck020", 0, ("MG_OWN", "Moved Probe")), gone]
    };
    let bandit = bandit_holding(&mut h, area, &held);
    let (_, pack) = creature_items(&mut h, area, &bandit);
    let own = pack[0].clone();
    // Its own Properties open beside the creature's.
    let at = bandit.item("ItemList", 0);
    let tab = Tab::Instance { area, path: at.clone() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(6);
    own_window(&mut h, &tab, egui::pos2(920.0, 40.0), egui::vec2(560.0, 1320.0));
    assert_eq!(held_window(&h, area, "MG_OWN"), at);
    // Equip (its row's menu): in the amulet's slot, itself.
    menu_of(&mut h, "Moved Probe", "Equip");
    let (worn, pack) = creature_items(&mut h, area, &bandit);
    assert!(!pack.iter().any(|s| s.string("Tag") == Some(b"MG_OWN")), "out of the backpack");
    let i = worn.iter().position(|s| s.id == 0x200).expect("in the amulet's slot");
    assert_eq!(worn[i].get("Repos_PosX"), None, "no place in the grid");
    assert_eq!(as_held_anywhere(&worn[i]), as_held_anywhere(&own), "every field kept");
    assert_eq!(held_window(&h, area, "MG_OWN"), bandit.item("Equip_ItemList", i), "followed");
    // One step: Undo puts it back where it was.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(4);
    let (_, pack) = creature_items(&mut h, area, &bandit);
    assert_eq!(pack[0], own, "undone");
    assert_eq!(held_window(&h, area, "MG_OWN"), at, "followed back");
    h.state_mut().actions.push(mg_ui::Action::Redo);
    h.run_steps(4);
    // To Backpack (its slot's menu): at the backpack's end, at a free place.
    menu_of(&mut h, "Moved Probe", "To Backpack");
    let (worn, pack) = creature_items(&mut h, area, &bandit);
    assert!(!worn.iter().any(|s| s.id == 0x200), "out of its slot");
    let last = pack.last().unwrap();
    assert_eq!(as_held_anywhere(last), as_held_anywhere(&own));
    assert_eq!(last.id as usize, pack.len() - 1);
    assert!(last.get("Repos_PosX").is_some() && last.get("Repos_Posy").is_some());
    assert_eq!(held_window(&h, area, "MG_OWN"), bandit.item("ItemList", pack.len() - 1));
    // An item whose blueprint is gone: equipped as it is.
    let gone = pack[0].clone();
    assert_eq!(gone.string("Tag"), Some(&b"MG_NOWHERE"[..]));
    menu_of(&mut h, "Nowhere Probe", "Equip");
    let (worn, _) = creature_items(&mut h, area, &bandit);
    let ring = worn.iter().find(|s| s.string("Tag") == Some(b"MG_NOWHERE"));
    let ring = ring.unwrap_or_else(|| panic!("equipped: {:?}", h.state().log.entries));
    assert!(ring.id == 0x80 || ring.id == 0x100, "a ring's slot");
    assert_eq!(as_held_anywhere(ring), as_held_anywhere(&gone));
}

/// The creature window's Cancel sets its lists back whole: the window of
/// an item it had equipped follows the item back into the backpack.
#[test]
fn a_held_item_s_window_follows_it_when_its_list_is_set_back() {
    let Some((mut h, area)) = area_harness("held-cancel") else { return };
    let held = |game: &mg_rules::GameData| {
        vec![own_item(game, "nw_it_mneck020", 0, ("MG_OWN", "Moved Probe"))]
    };
    let bandit = bandit_holding(&mut h, area, &held);
    let at = bandit.item("ItemList", 0);
    let tab = Tab::Instance { area, path: at.clone() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(6);
    own_window(&mut h, &tab, egui::pos2(920.0, 40.0), egui::vec2(560.0, 1320.0));
    menu_of(&mut h, "Moved Probe", "Equip");
    assert_ne!(held_window(&h, area, "MG_OWN"), at, "equipped");
    // The creature window's Cancel (at the left).
    let cancel = h
        .query_all_by_label("Cancel")
        .map(|n| n.rect().center())
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    click_pos(&h, cancel);
    h.run_steps(6);
    let (worn, pack) = creature_items(&mut h, area, &bandit);
    assert!(!worn.iter().any(|s| s.id == 0x200), "set back");
    assert_eq!(pack[0].string("Tag"), Some(&b"MG_OWN"[..]));
    assert_eq!(held_window(&h, area, "MG_OWN"), at, "followed back");
}

/// A held item's window that follows its item keeps its page as it was:
/// what is typed in a field, not yet set, stays typed, in the field.
#[test]
fn a_held_item_s_window_keeps_what_is_typed_as_it_follows() {
    use mg_edit::{Command, Edit};
    let Some((mut h, area)) = area_harness("held-typed") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let chest = chest_of_three(&mut h, area);
    let at = chest.item("ItemList", 1);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Instance { area, path: at.clone() }));
    h.run_steps(6);
    held_window(&h, area, "ITEM_B");
    let is_tag = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some("ITEM_B")
    };
    let field = h.query(egui_kittest::kittest::by().predicate(is_tag)).unwrap().rect().center();
    click_pos(&h, field);
    h.run_steps(2);
    h.event(egui::Event::Key {
        key: egui::Key::End,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.event(egui::Event::Text("_TYPED".into()));
    h.run_steps(3);
    assert!(shows_tag(&h, "ITEM_B_TYPED"), "typed");
    // The item before it taken out: the window follows, still typing.
    let remove = Edit::remove_item(git, chest.clone(), "ItemList", 0);
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Remove item", vec![remove])));
    h.run_steps(4);
    assert!(shows_tag(&h, "ITEM_B_TYPED"), "still typed where it went");
    let ws = h.state_mut().ws.as_mut().unwrap();
    let items = chest.get(&ws.doc(&git).unwrap().root).unwrap().list("ItemList").unwrap();
    assert_eq!(items[0].string("Tag"), Some(&b"ITEM_B"[..]), "not set yet");
}

/// A drag from `from` to `to` with the pointer's first button.
fn drag_pos(h: &mut Harness<'_, Moonglow>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run_steps(1);
    press(h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.run_steps(1);
    }
    press(h, to, false, egui::Modifiers::NONE);
    h.run_steps(4);
}

/// An item dragged from a placed chest's window into a placed creature's
/// backpack is carried there as it is, taken from the chest in the same
/// step; its own window follows it from the chest to the creature.
#[test]
fn a_held_item_dragged_to_another_object_is_carried_as_it_is() {
    let Some((mut h, area)) = area_harness("held-carried") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let chest = chest_of_three(&mut h, area);
    // (The chest is the first placeable, the bandit the first creature.)
    let bandit = bandit_holding(&mut h, area, &|_| Vec::new());
    let ws = h.state_mut().ws.as_mut().unwrap();
    let taken = chest.item("ItemList", 1).get(&ws.doc(&git).unwrap().root).unwrap().clone();
    let tab = Tab::Instance { area, path: chest.clone() };
    h.state_mut().blueprint_pages.insert((git, chest.clone()), "Inventory");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(6);
    own_window(&mut h, &tab, egui::pos2(920.0, 40.0), egui::vec2(560.0, 640.0));
    let item = Tab::Instance { area, path: chest.item("ItemList", 1) };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(item.clone()));
    h.run_steps(6);
    own_window(&mut h, &item, egui::pos2(920.0, 700.0), egui::vec2(560.0, 660.0));
    assert_eq!(held_window(&h, area, "ITEM_B"), chest.item("ItemList", 1));
    // The chest's second sword (its rows at the right, top to bottom), onto
    // the creature's backpack (under its heading, at the left).
    let mut rows: Vec<egui::Rect> = h
        .query_all_by_label("Longsword")
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .map(|n| n.rect())
        .filter(|r| r.left() > 920.0 && r.top() < 680.0 && r.width() > 80.0)
        .collect();
    rows.sort_by(|a, b| a.top().total_cmp(&b.top()));
    assert_eq!(rows.len(), 3, "the chest's three swords");
    let heading = h
        .query_all_by_label("Backpack")
        .map(|n| n.rect())
        .min_by(|a, b| a.left().total_cmp(&b.left()))
        .unwrap();
    drag_pos(&mut h, rows[1].center(), heading.left_bottom() + egui::vec2(80.0, 40.0));
    let (_, pack) = creature_items(&mut h, area, &bandit);
    assert_eq!(pack.len(), 1, "carried: {:?}", h.state().log.entries);
    assert_eq!(as_held_anywhere(&pack[0]), as_held_anywhere(&taken), "as it is");
    assert_eq!(pack[0].id, 0);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let left = chest.get(&ws.doc(&git).unwrap().root).unwrap().list("ItemList").unwrap().to_vec();
    let tags: Vec<_> = left.iter().map(|s| s.string("Tag").unwrap().to_vec()).collect();
    assert_eq!(tags, [&b"ITEM_A"[..], b"ITEM_C"], "taken from the chest");
    assert_eq!(held_window(&h, area, "ITEM_B"), bandit.item("ItemList", 0), "followed");
    // One step: Undo puts it back in the chest, and its window with it.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(4);
    assert!(creature_items(&mut h, area, &bandit).1.is_empty());
    assert_eq!(held_window(&h, area, "ITEM_B"), chest.item("ItemList", 1), "back");
}

/// Equipping a held item the creature lacks the feat for asks first: No
/// leaves the item in the backpack as it was (it was once taken out
/// before the question); Yes adds the feat and equips the item itself,
/// one step that Undo takes back whole.
#[test]
fn a_held_item_needing_a_feat_stays_in_the_backpack_on_no() {
    use mg_edit::{Command, Edit};
    let Some((mut h, area)) = area_harness("held-feat") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let held = |game: &mg_rules::GameData| {
        vec![own_item(game, "nw_wswls001", 0, ("MG_SWORD", "Sword Probe"))]
    };
    let bandit = bandit_holding(&mut h, area, &held);
    let featless = Edit::SetField {
        key: git,
        path: bandit.clone(),
        label: "FeatList".into(),
        value: Some(mg_gff::Value::List(Vec::new())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("No feats", vec![featless])));
    h.run_steps(4);
    let feats = |h: &mut Harness<'_, Moonglow>| -> Vec<i64> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let c = bandit.get(&ws.doc(&git).unwrap().root).unwrap().clone();
        c.list("FeatList").unwrap_or(&[]).iter().filter_map(|f| f.integer("Feat")).collect()
    };
    let (worn, pack) = creature_items(&mut h, area, &bandit);
    let sword = pack[0].clone();
    menu_of(&mut h, "Sword Probe", "Equip");
    assert!(h.query_by_label_contains("Weapon Proficiency (martial)").is_some(), "asked");
    h.get_by_label("No").click();
    h.run_steps(4);
    assert_eq!(
        creature_items(&mut h, area, &bandit),
        (worn.clone(), vec![sword.clone()]),
        "as it was"
    );
    assert!(feats(&mut h).is_empty());
    // Yes: the feat added, the sword itself in the free weapon slot.
    menu_of(&mut h, "Sword Probe", "Equip");
    h.get_by_label("Yes").click();
    h.run_steps(4);
    let (now, pack) = creature_items(&mut h, area, &bandit);
    assert!(pack.is_empty(), "out of the backpack");
    let it = now.iter().find(|s| s.string("Tag") == Some(b"MG_SWORD")).expect("equipped");
    assert_eq!(it.id, 0x20, "the secondary weapon's slot, the primary's taken");
    assert_eq!(as_held_anywhere(it), as_held_anywhere(&sword));
    assert_eq!(feats(&mut h), [45]);
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(4);
    assert_eq!(creature_items(&mut h, area, &bandit), (worn, vec![sword]), "undone whole");
    assert!(feats(&mut h).is_empty());
}

/// An item dropped on a slot that is taken: what the slot held goes to
/// the backpack as it is (once it was deleted), in the same step.
#[test]
fn an_item_dropped_on_a_taken_slot_sends_its_item_to_the_backpack() {
    use mg_edit::{Command, Edit};
    let Some((mut h, area)) = area_harness("taken-slot") else { return };
    let git = ResKey::new(area, ResType::GIT);
    let held = |game: &mg_rules::GameData| {
        vec![own_item(game, "nw_it_mneck020", 0, ("MG_NEW", "New Probe"))]
    };
    let bandit = bandit_holding(&mut h, area, &held);
    // An amulet of its own worn (after the armor and the sword).
    let game = h.state().game.clone().unwrap();
    let old = own_item(&game, "nw_it_mneck020", 0, ("MG_OLD", "Old Probe"));
    let old = mg_module::instances::equipped_item(&old, 0x200);
    let worn = Edit::insert_item(git, bandit.clone(), "Equip_ItemList", 2, old.clone());
    h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Worn", vec![worn])));
    h.run_steps(4);
    let before = creature_items(&mut h, area, &bandit);
    let new = before.1[0].clone();
    // The backpack's amulet (its row's name) onto the amulet's slot (its
    // name, in the creature window).
    let row = h
        .query_all_by_label("New Probe")
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .map(|n| n.rect().center())
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .unwrap();
    let slot = h
        .query_all_by_label("Amulet")
        .map(|n| n.rect())
        .filter(|r| r.left() > 270.0 && r.right() < 900.0)
        .min_by(|a, b| a.left().total_cmp(&b.left()))
        .unwrap();
    drag_pos(&mut h, row, slot.center());
    let (worn, pack) = creature_items(&mut h, area, &bandit);
    let on = worn.iter().find(|s| s.id == 0x200).expect("the slot holds one");
    assert_eq!(on.string("Tag"), Some(&b"MG_NEW"[..]), "{:?}", h.state().log.entries);
    assert_eq!(as_held_anywhere(on), as_held_anywhere(&new));
    assert_eq!(pack.len(), 1, "the one it held in the backpack");
    assert_eq!(as_held_anywhere(&pack[0]), as_held_anywhere(&old), "as it was");
    assert_eq!(pack[0].id, 0);
    assert!(pack[0].get("Repos_PosX").is_some() && pack[0].get("Repos_Posy").is_some());
    // One step.
    h.state_mut().actions.push(mg_ui::Action::Undo);
    h.run_steps(4);
    assert_eq!(creature_items(&mut h, area, &bandit), before, "undone whole");
}
