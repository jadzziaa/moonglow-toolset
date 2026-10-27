//! Tiles: terrain brushes, tile tools, the tile gallery, tile properties
//! and resizing an area.

use crate::common::*;

#[test]
fn tileset_made_edited_saved_with_its_palette() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("ui-tileset");
    let set_path = dir.join("zzz01.set");
    let _ = std::fs::remove_file(&set_path);
    // A game tileset's copy, to edit too.
    let game =
        mg_resman::ResMan::for_game(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let tic = game.get(&ResKey::parse("tic01", ResType::SET).unwrap()).unwrap().into_owned();
    let tic_path = dir.join("tic01.set");
    std::fs::write(&tic_path, &tic).unwrap();
    let dialogs = NoDialogs {
        save: vec![set_path.clone()],
        open: vec![tic_path.clone()],
        ..Default::default()
    };
    let mut app = game_app_with(&root, None, dialogs);
    app.open_module(&sample_module(&dir));
    app.open_palette = false;
    let mut h = harness(app, 1300.0, 900.0);
    h.run();
    mg_ui::tileset_view::new_tileset(h.state_mut());
    h.run();
    assert!(set_path.is_file());
    let id = h.state().tilesets[0].id;
    assert!(h.state().dock.find_tab(&Tab::Tileset(id)).is_some());

    // Terrains and a crosser.
    h.get_by_label("Terrains and Crossers").click();
    h.run();
    for (name, button) in
        [("Grass", "Add Terrain"), ("Water", "Add Terrain"), ("Road", "Add Crosser")]
    {
        h.state_mut().tilesets[0].new_type = name.into();
        h.run();
        h.get_by_label(button).click();
        h.run();
    }
    let t = h.state().tilesets[0].tileset().unwrap().clone();
    assert_eq!((t.terrains.len(), t.crossers.len()), (2, 1));

    // A tile, a group; an undo and a redo.
    h.get_by_label("Tiles").click();
    h.run();
    h.get_by_label("Add Tile").click();
    h.run();
    h.get_by_label("Groups").click();
    h.run();
    h.get_by_label("Add Group").click();
    h.run();
    h.get_by_label("Undo").click();
    h.run();
    assert!(h.state().tilesets[0].tileset().unwrap().groups.is_empty());
    h.get_by_label("Redo").click();
    h.run();
    let t = h.state().tilesets[0].tileset().unwrap().clone();
    assert_eq!((t.tiles.len(), t.groups.len()), (1, 1));
    assert_eq!(t.tiles[0].corners[0].0, "Grass");

    // Saved, its palette made, checked.
    h.get_by_label("Save").click();
    h.run();
    let saved =
        mg_set::Tileset::parse(&std::fs::read(&set_path).unwrap(), mg_core::Codepage::default())
            .unwrap();
    assert_eq!(saved.groups.len(), 1);
    h.get_by_label("Make Palette").click();
    h.run();
    let pal = mg_gff::Gff::read(&std::fs::read(dir.join("zzz01palstd.itp")).unwrap()).unwrap();
    assert_eq!(pal.file_type, *b"ITP ");
    h.get_by_label("Check").click();
    h.run();
    let findings = h.state().tilesets[0].findings.clone().unwrap();
    assert!(
        findings.iter().any(|f| f.message.contains("zzz01_a01_01")),
        "the tile's model is missing: {findings:?}"
    );

    // A game tileset's copy: one value changed, one line saved.
    mg_ui::tileset_view::open(h.state_mut());
    h.run();
    let doc = h.state_mut().tilesets.iter_mut().find(|d| d.path == tic_path).unwrap();
    doc.change(|f| f.set("GENERAL", "Transition", "6"));
    assert!(doc.is_dirty());
    doc.save().unwrap();
    let after = std::fs::read(&tic_path).unwrap();
    let (a, b) =
        (String::from_utf8_lossy(&tic).into_owned(), String::from_utf8_lossy(&after).into_owned());
    assert_eq!(a.lines().zip(b.lines()).filter(|(x, y)| x != y).count(), 1);
}

/// A tileset group's name is typed like any other field: kept while it is
/// typed, set when focus leaves.
#[test]
fn a_tileset_group_s_name_is_typed() {
    let dir = mg_testkit::scratch_dir("ui-tileset-group");
    let set_path = dir.join("zzz02.set");
    let dialogs = NoDialogs { save: vec![set_path.clone()], ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.open_module(&sample_module(&dir));
    app.open_palette = false;
    let mut h = harness(app, 1300.0, 900.0);
    h.run();
    mg_ui::tileset_view::new_tileset(h.state_mut());
    h.run();
    h.get_by_label("Terrains and Crossers").click();
    h.run();
    h.state_mut().tilesets[0].new_type = "Grass".into();
    h.run();
    h.get_by_label("Add Terrain").click();
    h.run();
    h.get_by_label("Tiles").click();
    h.run();
    h.get_by_label("Add Tile").click();
    h.run();
    h.get_by_label("Groups").click();
    h.run();
    h.get_by_label("Add Group").click();
    h.run();
    let group =
        |h: &Harness<'_, Moonglow>| h.state().tilesets[0].tileset().unwrap().groups[0].clone();
    let before = group(&h).name;
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(&before).find(is_input).expect("the group's name field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value(&before).find(is_input).expect("the group's name field").type_text("Bridge");
    h.run();
    h.run();
    assert_eq!(group(&h).name, before, "not before focus leaves");
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(group(&h).name, "Bridge");
    // What Check found is of the file as it was: an undo or a redo, like
    // any change, clears it.
    for step in [mg_ui::tileset_view::TilesetDoc::undo, mg_ui::tileset_view::TilesetDoc::redo] {
        h.state_mut().tilesets[0].findings = Some(Vec::new());
        step(&mut h.state_mut().tilesets[0]);
        assert!(h.state().tilesets[0].findings.is_none());
    }
    assert_eq!(group(&h).name, "Bridge");
}

#[test]
fn the_crosser_cursor_lies_on_raised_ground_under_the_pointer() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("crosser-cursor") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    // Corners (1, 1) and (1, 2) raised twice: ground well above the tiles'
    // own corners nearby.
    h.get_by_label("↕ Raise/Lower").click();
    h.run_steps(2);
    for corner in [(10.0, 10.0), (10.0, 20.0)] {
        for _ in 0..2 {
            let at = screen(&h, area, Vec3::new(corner.0, corner.1, 0.0));
            h.hover_at(at);
            press(&h, at, true, egui::Modifiers::NONE);
            press(&h, at, false, egui::Modifiers::NONE);
            h.run_steps(3);
        }
    }
    h.get_by_label("Road").click();
    h.run_steps(2);
    // Wherever the pointer rests on raised ground, the quarter drawn for it
    // lies on that ground.
    let rect = h.state().area_views[&area].rect;
    let mut checked = 0;
    for i in 1..12 {
        for j in 1..12 {
            let at = rect.min + rect.size() * egui::vec2(i as f32 / 12.0, j as f32 / 12.0);
            h.hover_at(at);
            h.run_steps(3);
            let view = &h.state().area_views[&area];
            let (Some(pointer), [(triangle, _)]) = (view.pointer, view.brush_cursor.as_slice())
            else {
                continue;
            };
            if pointer.z < 4.0 {
                continue;
            }
            // (Its corners may stand lower or higher on a slope or a cliff;
            // drawn at the tile corners' lattice heights, it was 10 m off.)
            let mean = triangle.iter().map(|p| p.z).sum::<f32>() / 3.0;
            assert!(
                (mean - pointer.z).abs() < 5.0,
                "the pointer at {at:?} is on the ground at {pointer}, the cursor at {triangle:?}"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no point on raised ground was found");
}

#[test]
fn area_viewer_paints_terrain() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("terrain") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    let button = |h: &mut Harness<'_, Moonglow>, p: Vec3, b: egui::PointerButton| {
        let at = screen(h, area, p);
        h.hover_at(at);
        for pressed in [true, false] {
            let m = egui::Modifiers::NONE;
            h.event(egui::Event::PointerButton { pos: at, button: b, pressed, modifiers: m });
        }
        h.run_steps(2);
    };
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| button(h, p, egui::PointerButton::Primary);
    let water = index.terrain("Water").unwrap();

    // The Tiles palette: the area's tileset, its Terrain branch open.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("Water").click();
    h.run_steps(2);
    h.get_by_label_contains("Water: click a corner");

    // Water at corner (2, 2): the four tiles around it are a pond.
    click(&mut h, Vec3::new(20.5, 19.5, 0.0));
    h.run_steps(2);
    assert_eq!(lattice(&mut h).corner(2, 2).terrain, water);
    // One command: undone, the grass is back.
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(2);
    assert_ne!(lattice(&mut h).corner(2, 2).terrain, water);

    // Shift + click with Grass on a grass corner: the four tiles around it
    // step to their next variants; the terrain stays.
    h.get_by_label("Grass").click();
    h.run_steps(2);
    let tiles = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let before = tiles(&mut h);
    let at = screen(&h, area, Vec3::new(30.0, 30.0, 0.0));
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::SHIFT);
    press(&h, at, false, egui::Modifiers::SHIFT);
    h.run_steps(2);
    let after = tiles(&mut h);
    assert_eq!(after.lattice, before.lattice);
    let moved = [(2, 2), (3, 2), (2, 3), (3, 3)]
        .iter()
        .filter(|&&(x, y)| after.tile(x, y) != before.tile(x, y))
        .count();
    assert!(moved > 0, "the tiles step to their next variants");

    // Raise and lower a corner.
    h.get_by_label("↕ Raise/Lower").click();
    h.run_steps(2);
    click(&mut h, Vec3::new(10.0, 30.0, 0.0));
    assert_eq!(lattice(&mut h).corner(1, 3).height, 1);
    button(&mut h, Vec3::new(10.0, 30.0, 5.0), egui::PointerButton::Secondary);
    assert_eq!(lattice(&mut h).corner(1, 3).height, 0);

    // A road dragged from tile (0, 0) to (2, 0), starting in the east
    // quarter of the first: on the two edges crossed.
    h.get_by_label("Road").click();
    h.run_steps(2);
    let from = screen(&h, area, Vec3::new(7.0, 5.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for x in [9.0, 12.0, 15.0, 18.0, 21.0, 24.0, 25.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 5.0, 0.0)));
        h.run_steps(1);
    }
    let to = screen(&h, area, Vec3::new(25.0, 5.0, 0.0));
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(2);
    let road = index.crosser("Road").unwrap();
    let l = lattice(&mut h);
    let edges: Vec<_> = (0..4).map(|x| l.cell(x, 0).edges).collect();
    assert_eq!(edges[0], [None, Some(road), None, None]);
    assert_eq!(edges[1], [None, Some(road), None, Some(road)]);
    assert_eq!(edges[2], [None, None, None, Some(road)]);

    // Trees beside the straight road: no tile has that, nothing changes.
    h.get_by_label("Trees").click();
    h.run_steps(2);
    let before = lattice(&mut h);
    click(&mut h, Vec3::new(20.0, 10.0, 0.0));
    assert_eq!(lattice(&mut h), before);
    h.get_by_label_contains("no tile fits the tiles shown in red");
    // The tiles in the way are shown in red for a moment.
    let refused = h.state().area_views[&area].refused.clone().expect("the tiles that refused");
    assert!(!refused.0.is_empty());

    // The Eraser on the road's middle tile takes the road away.
    h.get_by_label("🗑 Eraser").click();
    h.run_steps(2);
    click(&mut h, Vec3::new(15.0, 5.0, 0.0));
    let l = lattice(&mut h);
    assert!((0..4).all(|x| l.cell(x, 0).edges == [None; 4]), "{:?}", l.cell(0, 0));

    // A group (Barn 1: tiles 145, 147 / 144, 146 from the south-west),
    // turned a quarter by a right click, its first tile where clicked.
    h.get_by_label("Groups").click();
    h.run_steps(2);
    h.get_by_label("Barn 1 2x2").click();
    h.run_steps(2);
    button(&mut h, Vec3::new(25.0, 15.0, 0.0), egui::PointerButton::Secondary);
    click(&mut h, Vec3::new(25.0, 15.0, 0.0));
    let tiles = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let g = tiles(&mut h);
    let at = |x: u32, y: u32| (g.tile(x, y).tile, g.tile(x, y).orientation);
    assert_eq!([at(2, 1), at(2, 2), at(1, 1), at(1, 2)], [(145, 1), (147, 1), (144, 1), (146, 1)]);
    assert!(h.state().palette.tile_brush.is_some(), "the group stays chosen");
}

/// Tile Properties of one tile shows the tiles that fit there as
/// pictures; a click puts that one there, in a step of its own.
#[test]
fn tile_properties_choose_a_variant_by_picture() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-variants") else { return };
    let tile = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[i].clone()
    };
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(2);
    h.get_by_label("Tile Properties…").click();
    h.run_steps(2);
    // Its variants: the tiles that fit there, the one there now among
    // them; another chosen is put there at once, the window staying.
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!(props.tiles, [5]);
    let (cell, variants) = props.variants.clone().expect("one tile: its variants");
    assert_eq!(cell, (1, 1));
    let now = props.variant.unwrap();
    assert!(variants.len() > 1 && variants.iter().any(|(p, _)| *p == now), "{variants:?}");
    h.render().unwrap().save(mg_testkit::scratch_dir("ui-tile-variants").join("a.png")).unwrap();
    let other = variants.iter().find(|(p, _)| p.tile != now.tile).unwrap();
    h.get_by_label_contains(&format!("Variant ({} fit here)", variants.len()));
    let before_undo = h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    // (A picture has no name of its own: the one over its model's name.)
    let turn = ["", " ↺90°", " ↺180°", " ↺270°"][usize::from(other.0.orientation % 4)];
    let picture = h
        .query_all_by_label(&format!("{}{turn}", other.1))
        .next()
        .map(|n| n.rect().center_top() - egui::vec2(0.0, 40.0));
    if let Some(at) = picture {
        h.hover_at(at);
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        h.run_steps(4);
        // (Which picture that was is not told apart here: one of them.)
        let id = tile(&mut h, 5).integer("Tile_ID").unwrap();
        assert!(variants.iter().any(|(p, _)| i64::from(p.tile) == id), "tile {id}");
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tile variant"));
        let after = h.state().tile_props.clone().expect("the window stays");
        assert_eq!(after.variant.map(|p| i64::from(p.tile)), Some(id));
        assert_ne!(after.variant, Some(now), "another than the one that was there");
        assert_eq!(
            after.base[0].integer("Tile_ID"),
            Some(id),
            "taken up: the window is the new tile's"
        );
        h.state_mut().ws.as_mut().unwrap().undo().unwrap();
        h.state_mut().tile_props = Some(props);
        h.run_steps(2);
        assert_eq!(h.state().ws.as_ref().unwrap().can_undo().map(str::to_string), before_undo);
    } else {
        panic!("no variant pictures");
    }
}

#[test]
fn area_viewer_selects_tiles_and_sets_their_properties() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness_on("tiles", "tic01", None) else { return };
    let tile = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[i].clone()
    };
    // A replacement texture in the file (BioWare's unused `Tile_ReplaceTex`,
    // set by hand): kept, and not in the window.
    let set = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::ARE),
        path: mg_edit::GffPath::root().item("Tile_List", 5),
        label: "Tile_ReplaceTex".into(),
        value: Some(mg_gff::Value::Byte(1)),
    };
    let set = mg_edit::Command::new("Replacement texture", vec![set]);
    h.state_mut().actions.push(mg_ui::Action::Apply(set));
    h.run_steps(2);
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    // The room in the middle: tile (1, 1), index 5.
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].tile_selection, [(1, 1)]);
    let original = tile(&mut h, 5);
    // Right click: the tile menu, Tile Properties.
    let button = egui::PointerButton::Secondary;
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(2);
    h.get_by_label("Tile Properties…").click();
    h.run_steps(2);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!(props.tiles, [5]);
    // The colour picker, opened from its swatch: in front of the window
    // it is opened from, at once. (Opened again, it came up behind it for
    // a frame, and with nothing to draw another, stayed there: the click
    // looked to do nothing.)
    let picker = egui::LayerId::new(egui::Order::Middle, egui::Id::new(Some("Select A Color")));
    let on_top = |h: &Harness<'_, Moonglow>| {
        h.ctx.memory(|m| m.areas().top_layer_id(egui::Order::Middle)) == Some(picker)
    };
    for time in 0..2 {
        h.get_by_label("Main Light 1 Color").click();
        // (The frames the application asks for, and no more: nothing else
        // draws another.)
        h.run();
        let top = h.ctx.memory(|m| m.areas().top_layer_id(egui::Order::Middle));
        assert!(on_top(&h), "the picker in front, as it opens ({time}): {top:?}");
        h.run_steps(2);
        assert!(h.state().tile_props.as_ref().unwrap().picking.is_some());
        // Closed, and Tile Properties clicked: it is in front then.
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
        assert!(h.state().tile_props.as_ref().unwrap().picking.is_none());
        let beside = h.get_by_label("Main Light 2 Color").rect().center() + egui::vec2(-60.0, 0.0);
        h.hover_at(beside);
        press(&h, beside, true, egui::Modifiers::NONE);
        press(&h, beside, false, egui::Modifiers::NONE);
        h.run_steps(2);
    }
    // Main light 1 to colour 9, through the colour picker.
    h.state_mut().tile_props.as_mut().unwrap().main[0] = 9;
    h.state_mut().tile_props.as_mut().unwrap().loops = [false, false, false];
    assert!(h.query_by_label_contains("Replacement Texture").is_none(), "no longer in the window");
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(2);
    let t = tile(&mut h, 5);
    assert_eq!(t.integer("Tile_ReplaceTex"), Some(1), "the file's field kept");
    if props.has_main[0] {
        assert_eq!(t.integer("Tile_MainLight1"), Some(9));
    }
    if props.has_loops[0] {
        assert_eq!(t.integer("Tile_AnimLoop1"), Some(0));
    }
    assert!(h.state().tile_props.is_none());
    // Undone in one step.
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(1);
    assert_eq!(tile(&mut h, 5), original);
}

/// Sets the tile at `index` of `area`'s `Tile_List` to tile `id`, as it
/// lies, through the workspace.
fn set_tile_id(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize, id: i32) {
    let set = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::ARE),
        path: mg_edit::GffPath::root().item("Tile_List", index),
        label: "Tile_ID".into(),
        value: Some(mg_gff::Value::Int(id)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tile", vec![set])));
    h.run_steps(3);
}

/// In Select Tiles, clicks the tiles at `points` (Ctrl + click after the
/// first) and opens Tile Properties from the tile menu.
fn tile_properties_of(h: &mut Harness<'_, Moonglow>, area: ResRef, points: &[glam::Vec3]) {
    if !h.state().area_views[&area].tile_mode {
        h.get_by_label("⛶ Select Tiles").click();
        h.run_steps(2);
    }
    for (i, p) in points.iter().enumerate() {
        let at = screen(h, area, *p);
        let m = if i == 0 { egui::Modifiers::NONE } else { egui::Modifiers::COMMAND };
        h.hover_at(at);
        press(h, at, true, m);
        press(h, at, false, m);
        h.run_steps(2);
    }
    let at = screen(h, area, points[0]);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(2);
    h.get_by_label("Tile Properties…").click();
    h.run_steps(3);
}

/// Tile Properties enables only the lights and loops the tile's model has
/// (its `…ml1`/`ml2` light nodes, `…sl1`/`sl2` nodes and `animloop01..03`
/// animations), the others greyed out and a loop not had unchecked, as
/// Aurora shows them (seen on the oracle: tin01 tiles 18, 0, 134; the
/// game's tilesets say every tile has all seven). Several tiles: what any
/// has; each tile is written only what it has, the rest of its fields
/// kept. (Jasperre's tile feedback.)
#[test]
fn tile_properties_enable_only_the_tile_s_lights_and_loops() {
    use glam::Vec3;
    // tin01 tile 18 (tin01_d11_01): main light 1 alone, no loops.
    let Some((mut h, area)) = area_harness_on("tile-lights", "tin01", Some(18)) else { return };
    let tile = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[i].clone()
    };
    let (one, two) = (Vec3::new(15.0, 15.0, 0.0), Vec3::new(25.0, 15.0, 0.0));
    tile_properties_of(&mut h, area, &[one]);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!(props.tiles, [5]);
    assert_eq!((props.has_main, props.has_source), ([true, false], [false, false]));
    assert_eq!(props.has_loops, [false; 3]);
    let disabled = |h: &Harness<'_, Moonglow>, label: &str| {
        h.get_by_label(label).accesskit_node().is_disabled()
    };
    assert!(!disabled(&h, "Main Light 1 Color"));
    for label in ["Main Light 2 Color", "Source Light 1 Color", "Source Light 2 Color"] {
        assert!(disabled(&h, label), "{label}");
    }
    for k in 1..=3 {
        let label = format!("Animation Loop {k}");
        let node = h.get_by_label(&label);

        assert!(node.accesskit_node().is_disabled());
        assert_eq!(node.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    }
    let dir = mg_testkit::scratch_dir("ui-tile-lights");
    h.render().unwrap().save(dir.join("one-light.png")).unwrap();
    h.state_mut().tile_props = None;
    h.run_steps(2);

    // tin01 tile 134 (tin01_f01_02): both main lights and the three loops.
    set_tile_id(&mut h, area, 5, 134);
    tile_properties_of(&mut h, area, &[one]);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!((props.has_main, props.has_source), ([true, true], [false, false]));
    assert_eq!(props.has_loops, [true; 3]);
    assert!(!h.get_by_label("Animation Loop 3").accesskit_node().is_disabled());
    h.render().unwrap().save(dir.join("lights-and-loops.png")).unwrap();
    h.state_mut().tile_props = None;
    h.run_steps(2);

    // Tile 18 and, beside it, tile 0 (all four lights, no loops): the
    // lights any of them has.
    set_tile_id(&mut h, area, 5, 18);
    set_tile_id(&mut h, area, 6, 0);
    let (was5, was6) = (tile(&mut h, 5), tile(&mut h, 6));
    tile_properties_of(&mut h, area, &[one, two]);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!(props.tiles, [5, 6]);
    assert_eq!((props.has_main, props.has_source), ([true, true], [true, true]));
    assert_eq!(props.has_loops, [false; 3]);
    {
        let p = h.state_mut().tile_props.as_mut().unwrap();
        (p.main, p.source) = ([7, 9], [3, 4]);
        // (Chosen in the colour picker: each is set, even to the value the
        // first tile shows, and goes to every tile that has it.)
        p.touched[..4].fill(true);
    }
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(2);
    let (t5, t6) = (tile(&mut h, 5), tile(&mut h, 6));
    assert_eq!(t5.integer("Tile_MainLight1"), Some(7));
    for label in ["Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2", "Tile_AnimLoop1"] {
        assert_eq!(t5.get(label), was5.get(label), "tile 18 has no {label}: kept");
    }
    assert_eq!(
        ["Tile_MainLight1", "Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2"]
            .map(|l| t6.integer(l)),
        [Some(7), Some(9), Some(3), Some(4)]
    );
    assert_eq!(t6.get("Tile_AnimLoop1"), was6.get("Tile_AnimLoop1"), "no loops: kept");
}

/// Tile Properties closed with changes not applied (Escape, its tab's X)
/// asks Save, Don't Save or Cancel, as a side editor's tab does; with
/// none, it closes; its Cancel button discards. (Jasperre's tile
/// feedback: Aurora's X discards without asking.)
#[test]
fn tile_properties_ask_before_closing_with_changes() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-props-close") else { return };
    let tile = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[5].clone()
    };
    let undo_label =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    let at = Vec3::new(15.0, 15.0, 0.0);
    let was = tile(&mut h);
    let before = undo_label(&h);
    let change = |h: &mut Harness<'_, Moonglow>| {
        let p = h.state_mut().tile_props.as_mut().expect("the window is open");
        assert!(p.has_main[0], "ttr01's tiles have main light 1");
        p.main[0] = (p.main[0] + 5) % 32;
        h.run_steps(1);
    };
    // Unchanged: Escape closes it at once.
    tile_properties_of(&mut h, area, &[at]);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(h.state().tile_props.is_none(), "closed: nothing to ask");
    // Changed: Escape asks; Cancel keeps the window and its change.
    tile_properties_of(&mut h, area, &[at]);
    change(&mut h);
    assert_ne!(tile(&mut h), was, "the change in the area at once");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    h.get_by_label("Save changes to Tile Properties?");
    h.render()
        .unwrap()
        .save(mg_testkit::scratch_dir("ui-tile-props-close").join("ask.png"))
        .unwrap();
    let cancel = h.get_all_by_label("Cancel").last().unwrap().rect().center();
    h.hover_at(cancel);
    press(&h, cancel, true, egui::Modifiers::NONE);
    press(&h, cancel, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let p = h.state().tile_props.clone().expect("Cancel: the window stays");
    assert!(!p.closing && p.changed());
    assert!(h.query_by_label("Save changes to Tile Properties?").is_none());
    // Closed as its tab's X closes it: asked; Don't Save takes the change back.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::TileProperties]));
    h.run_steps(3);
    h.get_by_label("Don't Save").click();
    h.run_steps(3);
    assert!(h.state().tile_props.is_none());
    assert!(h.state().dock.find_tab(&Tab::TileProperties).is_none(), "its tab goes");
    assert_eq!(tile(&mut h), was);
    assert_eq!(undo_label(&h), before, "nothing left of it");
    assert_eq!(h.state().ws.as_ref().unwrap().can_redo(), None, "nor to redo");
    // Again, and Save: applied, as OK does.
    tile_properties_of(&mut h, area, &[at]);
    change(&mut h);
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::TileProperties]));
    h.run_steps(3);
    h.get_by_label("Save").click();
    h.run_steps(3);
    assert!(h.state().tile_props.is_none());
    assert_ne!(tile(&mut h).integer("Tile_MainLight1"), was.integer("Tile_MainLight1"));
    assert_eq!(undo_label(&h).as_deref(), Some("Tile Properties"));
    // Its Cancel button: discarded without asking.
    tile_properties_of(&mut h, area, &[at]);
    change(&mut h);
    let cancel = h.get_all_by_label("Cancel").last().unwrap().rect().center();
    h.hover_at(cancel);
    press(&h, cancel, true, egui::Modifiers::NONE);
    press(&h, cancel, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert!(h.state().tile_props.is_none());
    assert!(h.query_by_label("Save changes to Tile Properties?").is_none());
}

/// A group or feature is one thing (Jasperre's tile feedback): Next Tile
/// Appearance, Rotate Tile, Refine Tile and Shift + right click leave its
/// tiles as they are, with the status line saying why and how, its tiles
/// flashing red; Tile Properties offers it no variants; a group placed
/// over part of it is refused, as Aurora refuses it; Delete takes it whole.
/// Options › Area › Allow editing tiles of groups and features lets each
/// change, with a warning.
#[test]
fn groups_and_features_change_only_whole() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("group-locked") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let grouped = |h: &mut Harness<'_, Moonglow>| -> Vec<(u32, u32)> {
        let g = grid(h);
        let cells = (0..4).flat_map(|y| (0..4).map(move |x| (x, y)));
        cells.filter(|&(x, y)| index.is_grouped(g.tile(x, y).tile)).collect()
    };
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3, m: egui::Modifiers| {
        let at = screen(h, area, p);
        h.hover_at(at);
        h.run_steps(2);
        press(h, at, true, m);
        press(h, at, false, m);
        h.run_steps(3);
    };
    // (The status line's text.)
    let says = |h: &Harness<'_, Moonglow>, text: &str| h.query_by_label_contains(text).is_some();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    // A barn, its first tile in (1, 1).
    h.get_by_label("Groups").click();
    h.run_steps(2);
    h.get_by_label("Barn 1 2x2").click();
    h.run_steps(2);
    click(&mut h, Vec3::new(15.0, 15.0, 0.0), egui::Modifiers::NONE);
    let barn = vec![(1, 1), (2, 1), (1, 2), (2, 2)];
    assert_eq!(grouped(&mut h), [(1, 1), (2, 1), (1, 2), (2, 2)]);
    let placed = grid(&mut h);
    // Another over its east half: refused, the first in red.
    click(&mut h, Vec3::new(25.0, 15.0, 0.0), egui::Modifiers::NONE);
    assert_eq!(grid(&mut h), placed, "nothing placed");
    assert!(says(&h, "Place Barn 1 2x2: tiles of a group or feature (in red) change only"));
    let mut red = h.state().area_views[&area].refused.clone().expect("in red").0;
    red.sort();
    let mut whole = barn.clone();
    whole.sort();
    assert_eq!(red, whole);
    // (The cursor over it is red, as Aurora's outline turns.)
    h.render().unwrap().save(mg_testkit::scratch_dir("ui-group-locked").join("over.png")).unwrap();
    // (The groups closed: the tools above in view.)
    h.get_by_label("Groups").click();
    h.run_steps(2);
    // The tile tools on one of its tiles: refused, saying why.
    let east = Vec3::new(25.0, 15.0, 0.0);
    for (tool, label) in [
        ("⏭ Next Tile Appearance", "Next tile appearance"),
        ("↺ Rotate Tile", "Rotate tile"),
        ("🔁 Refine Tile", "Refine tile"),
    ] {
        h.get_by_label(tool).click();
        h.run_steps(2);
        click(&mut h, east, egui::Modifiers::NONE);
        assert_eq!(grid(&mut h), placed, "{tool}: nothing changed");
        assert!(says(&h, &format!("{label}: tiles of a group or feature")), "{tool}");
    }
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    // Shift + right click in Select Tiles; and Tile Properties offers no
    // variants.
    h.get_by_label("⛶ Select Tiles (F10)").click();
    h.run_steps(2);
    let at = screen(&h, area, east);
    h.hover_at(at);
    for pressed in [true, false] {
        h.event_modifiers(
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::SHIFT,
            },
            egui::Modifiers::SHIFT,
        );
    }
    h.run_steps(3);
    assert_eq!(grid(&mut h), placed, "Shift + right click: nothing changed");
    assert!(says(&h, "Next tile: tiles of a group or feature"));
    tile_properties_of(&mut h, area, &[east]);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert!(props.grouped && props.variants.is_none(), "no variants for a group's tile");
    h.get_by_label_contains("A tile of a group or a feature");
    h.state_mut().tile_props = None;
    h.run_steps(2);
    // Out of Select Tiles, the area's own menu on the ground: Rotate Tile
    // refused, the log saying why (the status line is the objects').
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    assert!(!h.state().area_views[&area].tile_mode);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Rotate Tile").click();
    h.run_steps(3);
    assert_eq!(grid(&mut h), placed, "the ground menu's Rotate Tile: nothing changed");
    assert!(says(&h, "Rotate tile: tiles of a group or feature"));

    // Allowed: the next appearance is put there, with a warning.
    h.state_mut().settings.edit_group_tiles = true;
    h.get_by_label("⏭ Next Tile Appearance").click();
    h.run_steps(2);
    click(&mut h, east, egui::Modifiers::NONE);
    assert_ne!(grid(&mut h).tile(2, 1), placed.tile(2, 1), "allowed: changed");
    assert!(says(&h, "⚠ Next tile appearance: a group or feature left part changed"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(grid(&mut h), placed, "undone");
    // A barn over its east half: the first goes whole.
    h.get_by_label("Groups").click();
    h.run_steps(2);
    h.get_by_label("Barn 1 2x2").click();

    h.run_steps(2);
    click(&mut h, east, egui::Modifiers::NONE);
    assert_eq!(grouped(&mut h), [(2, 1), (3, 1), (2, 2), (3, 2)]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(grid(&mut h), placed, "undone");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);

    // Not allowed: Delete on one of its tiles takes the whole barn.
    h.state_mut().settings.edit_group_tiles = false;
    if !h.state().area_views[&area].tile_mode {
        h.get_by_label("⛶ Select Tiles").click();
        h.run_steps(2);
    }
    click(&mut h, east, egui::Modifiers::NONE);
    assert_eq!(h.state().area_views[&area].tile_selection, [(2, 1)]);
    let at = screen(&h, area, east);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(3);
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert_eq!(grouped(&mut h), [], "the whole barn deleted");
}

#[test]
fn resize_and_rotate_area_from_the_edit_menu() {
    let Some((mut h, area)) = area_harness("reshape") else { return };
    let size = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        let tiles = are.root.list("Tile_List").unwrap().len();
        (are.root.integer("Width").unwrap(), are.root.integer("Height").unwrap(), tiles)
    };
    assert_eq!(size(&mut h), (4, 4, 16));
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Resize Area…").click();
    h.run_steps(2);
    let d = h.state_mut().resize_area.as_mut().expect("the window is open");
    (d.columns, d.rows) = (6, 3);
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(size(&mut h), (6, 3, 18));
    // The waypoints at (20, 20) and (30, 20) stay: inside 60 by 30.
    let waypoints = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root
            .list("WaypointList")
            .unwrap()
            .iter()
            .map(|w| (w.float("XPosition").unwrap(), w.float("YPosition").unwrap()))
            .collect::<Vec<_>>()
    };
    assert_eq!(waypoints(&mut h).len(), 2);
    // A quarter turn counter-clockwise: 3 by 6, (20, 20) to (10, 20).
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Rotate Area…").click();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(size(&mut h), (3, 6, 18));
    let w = waypoints(&mut h);
    assert!((w[0].0 - 10.0).abs() < 1e-3 && (w[0].1 - 20.0).abs() < 1e-3, "{w:?}");
    // Both undone, one step each.
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(1);
    assert_eq!(size(&mut h), (4, 4, 16));
    // At the south and west edges: two columns come at the west, a row
    // goes at the south, and the waypoints keep their places against the
    // north and east edges: (20, 20) to (40, 10).
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Resize Area…").click();
    h.run_steps(2);
    let d = h.state_mut().resize_area.as_mut().expect("the window is open");
    (d.columns, d.rows, d.edges) = (6, 3, 2);
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(size(&mut h), (6, 3, 18));
    let w = waypoints(&mut h);
    assert!((w[0].0 - 40.0).abs() < 1e-3 && (w[0].1 - 10.0).abs() < 1e-3, "{w:?}");
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(1);
    assert_eq!(size(&mut h), (4, 4, 16));
    let w = waypoints(&mut h);
    assert!((w[0].0 - 20.0).abs() < 1e-3 && (w[0].1 - 20.0).abs() < 1e-3, "{w:?}");
}

/// Resize Area from the Edit menu to `columns` by `rows` (the new ground
/// at the north and east), the camera then over all of it.
fn resize_area_to(h: &mut Harness<'_, Moonglow>, area: ResRef, columns: u32, rows: u32) {
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Resize Area…").click();
    h.run_steps(2);
    let d = h.state_mut().resize_area.as_mut().expect("the window is open");
    (d.columns, d.rows) = (columns, rows);
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    let model = view.model.as_ref().unwrap();
    assert_eq!((model.width, model.height), (columns, rows));
    view.orbit = Some(mg_ui::area_view::Orbit::overview(model));
    h.run_steps(2);
}

/// The area's ARE as the workspace has it.
fn area_are(h: &mut Harness<'_, Moonglow>, area: ResRef) -> mg_gff::Struct {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().root.clone()
}

/// The Tiles palette open on the area's tileset, brush `brush` chosen.
fn choose_tile_brush(h: &mut Harness<'_, Moonglow>, brush: &str) {
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label(brush).click();
    h.run_steps(2);
}

/// GitHub issue 31: a terrain brush over ground that only the larger area
/// had, when Undo took Resize Area back (twice, as reported: a stroke on
/// the new ground, then the resize). The next frame's preview of the brush
/// read the corner under the pointer, as the larger area had it, in the
/// smaller area's lattice, and panicked ("corner (2, 6) outside the
/// lattice"). The pointer's spot goes with the ground: the brush shows
/// nothing there, and a click there does nothing.
#[test]
fn a_terrain_brush_over_ground_an_undo_took_away_does_nothing() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("terrain-undo-resize") else { return };
    let before = area_are(&mut h, area);
    choose_tile_brush(&mut h, "Water");
    resize_area_to(&mut h, area, 4, 8);
    // Water at corner (2, 6), on the new ground: one command.
    let at = screen(&h, area, Vec3::new(20.0, 60.0, 0.0));
    click_pos(&h, at);
    h.run_steps(3);
    assert!(!h.state().area_views[&area].brush_cursor.is_empty(), "the brush's cursor there");
    assert_ne!(area_are(&mut h, area), before, "painted");
    // The stroke, then the resize, undone with the pointer resting there.
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(1);
    }
    h.run_steps(2);
    let view = &h.state().area_views[&area];
    let model = view.model.as_ref().unwrap();
    assert_eq!((model.width, model.height), (4, 4));
    // (Past the area's north edge now.)
    assert!(view.brush_cursor.is_empty() && view.tile_preview.is_empty());
    assert_eq!(area_are(&mut h, area), before, "both undone");
    // A click there: nothing painted.
    click_pos(&h, at);
    h.run_steps(3);
    assert_eq!(area_are(&mut h, area), before);
    // Redo makes the larger area again, and the pointer's corner with it.
    for _ in 0..2 {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
        h.run_steps(1);
    }
    h.run_steps(2);
    let view = &h.state().area_views[&area];
    assert_eq!(view.model.as_ref().map(|m| (m.width, m.height)), Some((4, 8)));
    assert!(!view.brush_cursor.is_empty());
}

/// Issue 31's other ways in: a terrain drag going on as Undo takes the
/// larger area back marked corners only the larger area had. The drag
/// goes with them, and letting go paints nothing (nor panics).
#[test]
fn a_terrain_drag_across_an_undo_of_resize_area_paints_nothing() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("terrain-drag-undo-resize") else { return };
    let before = area_are(&mut h, area);
    choose_tile_brush(&mut h, "↕ Raise/Lower");
    resize_area_to(&mut h, area, 4, 8);
    let (from, to) = (
        screen(&h, area, Vec3::new(20.0, 50.0, 0.0)),
        screen(&h, area, Vec3::new(20.0, 70.0, 0.0)),
    );
    h.hover_at(from);
    h.run_steps(1);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for k in 1..=4 {
        h.hover_at(from + (to - from) * (k as f32 / 4.0));
        h.run_steps(1);
    }
    // (The cursor: the squares round the corners marked, and the pointer's.)
    let shapes = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].brush_cursor.len();
    assert!(shapes(&h) > 2, "a drag going on");
    // Undone, the button still down.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(shapes(&h), 0, "the drag goes with the ground it marked");
    assert_eq!(area_are(&mut h, area), before);
    h.hover_at(to + egui::vec2(0.0, -5.0));
    h.run_steps(1);
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(4);
    assert_eq!(area_are(&mut h, area), before, "letting go paints nothing");
}

/// The tile menu's Tile Properties… on the tiles selected, the menu opened
/// at `at`.
fn tile_properties_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    for pressed in [true, false] {
        let button = egui::PointerButton::Secondary;
        let m = Default::default();
        h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers: m });
    }
    h.run_steps(2);
    h.get_by_label("Tile Properties…").click();
    h.run_steps(3);
}

/// Issue 31 in tile mode: the tiles laid out anew by an Undo of Resize
/// Area, the selection goes, and Tile Properties open on them closes;
/// Delete then deletes nothing (it panicked, reading the tile at an index
/// past the smaller area's).
#[test]
fn tiles_selected_on_ground_an_undo_took_away_are_dropped() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tiles-undo-resize") else { return };
    let before = area_are(&mut h, area);
    resize_area_to(&mut h, area, 4, 8);
    if !h.state().area_views[&area].tile_mode {
        h.get_by_label("⛶ Select Tiles").click();
        h.run_steps(2);
    }
    let south = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    let north = screen(&h, area, Vec3::new(15.0, 65.0, 0.0));
    click_pos(&h, south);
    h.run_steps(2);
    h.hover_at(north);
    press(&h, north, true, egui::Modifiers::COMMAND);
    press(&h, north, false, egui::Modifiers::COMMAND);
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].tile_selection, [(1, 1), (1, 6)]);
    // Their Tile Properties: by their places in the list, 5 and 25.
    tile_properties_at(&mut h, north);
    assert_eq!(h.state().tile_props.as_ref().map(|p| p.tiles.clone()), Some(vec![5, 25]));
    h.hover_at(screen(&h, area, Vec3::new(15.0, 15.0, 0.0)));
    h.run_steps(1);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(area_are(&mut h, area), before);
    assert!(h.state().tile_props.is_none(), "Tile Properties closed, its places gone");
    assert!(h.state().area_views[&area].tile_selection.is_empty(), "the selection gone");
    // Delete over the view: nothing to delete.
    h.hover_at(screen(&h, area, Vec3::new(15.0, 15.0, 0.0)));
    h.run_steps(1);
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(area_are(&mut h, area), before);
}

/// The coordinator's case of issue 31: Rotate Area on a square area keeps
/// its size, but every tile has another place. Tiles selected and Tile
/// Properties open on them before the turn: the selection goes and the
/// window closes, so that neither a change in it (a height) nor Delete
/// lands on the tiles turned into those places; the same across the
/// turn's Undo.
#[test]
fn tiles_selected_across_rotate_area_are_dropped() {
    use glam::Vec3;
    use mg_edit::Command;
    let Some((mut h, area)) = area_harness("tiles-rotate") else { return };
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    // (Where tile (0, 0) is on screen: Tile Properties' pane takes room.)
    let at = |h: &Harness<'_, Moonglow>| screen(h, area, Vec3::new(5.0, 5.0, 0.0));
    click_pos(&h, at(&h));
    h.run_steps(2);
    assert_eq!(h.state().area_views[&area].tile_selection, [(0, 0)]);
    let rotate = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let (are_key, git_key) = (ResKey::new(area, ResType::ARE), ResKey::new(area, ResType::GIT));
        let are = ws.doc(&are_key).unwrap().root.clone();
        let git = ws.doc(&git_key).unwrap().root.clone();
        let edits = mg_area::reshape::rotate(are_key, git_key, &are, &git, 1);
        h.state_mut().actions.push(mg_ui::Action::Apply(Command::new("Rotate Area", edits)));
        h.run_steps(3);
    };
    let tiles =
        |h: &mut Harness<'_, Moonglow>| area_are(h, area).list("Tile_List").unwrap().to_vec();
    for undo in [false, true] {
        let here = at(&h);
        tile_properties_at(&mut h, here);
        assert_eq!(h.state().tile_props.as_ref().map(|p| p.tiles.clone()), Some(vec![0]));
        if undo {
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
            h.run_steps(3);
        } else {
            rotate(&mut h);
        }
        let turned = tiles(&mut h);
        // A height asked of the window, were it open: written into tile 0.
        if let Some(p) = h.state_mut().tile_props.as_mut() {
            p.height = Some(p.first_height + 2);
        }
        h.run_steps(3);
        assert_eq!(tiles(&mut h), turned, "the height went to no tile (undo: {undo})");
        assert!(h.state().tile_props.is_none(), "Tile Properties closed (undo: {undo})");
        assert!(h.state().area_views[&area].tile_selection.is_empty(), "undo: {undo}");
        h.hover_at(at(&h));
        h.run_steps(1);
        h.key_press(egui::Key::Delete);
        h.run_steps(3);
        assert_eq!(tiles(&mut h), turned, "no tile changed (undo: {undo})");
        // Chosen again, for the next round.
        click_pos(&h, at(&h));
        h.run_steps(2);
    }
}

#[test]
fn tiles_copy_and_paste() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness_on("tile-paste", "tic01", None) else { return };
    let tile = |h: &mut Harness<'_, Moonglow>, i: usize| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[i].clone()
    };
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3| {
        let at = screen(h, area, p);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    // Tile (1, 1): the room's south-west quarter, a Stone corner at its
    // north-east. Copied, pasted with its south-west tile at (0, 0).
    click(&mut h, Vec3::new(15.0, 15.0, 0.0));
    let copied = tile(&mut h, 5);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
    h.run_steps(1);
    assert!(h.state().tile_clip.is_some());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
    h.run_steps(1);
    assert!(h.state().area_views[&area].tile_pasting);
    click(&mut h, Vec3::new(5.0, 5.0, 0.0));
    h.run_steps(2);
    assert!(!h.state().area_views[&area].tile_pasting);
    let pasted = tile(&mut h, 0);
    for label in ["Tile_ID", "Tile_Orientation", "Tile_Height", "Tile_MainLight1", "Tile_SrcLight1"]
    {
        assert_eq!(pasted.get(label), copied.get(label), "{label}");
    }
    // The tiles around fit it: the area's tiles make one lattice.
    let game = h.state().game.as_deref().unwrap();
    let set = mg_area::tileset(game, ResRef::from_str("tic01").unwrap()).unwrap();
    let index = mg_tiles::TileIndex::new(&set);
    let ws = h.state_mut().ws.as_mut().unwrap();
    let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
    let g = mg_area::terrain::grid(&are.root, &index).unwrap();
    let (_, bad) = mg_tiles::Lattice::from_tiles(&index, 4, 4, &g.tiles).unwrap();
    assert!(bad.is_empty(), "tiles disagree at {bad:?}");
}

#[test]
fn a_terrain_brush_paints_every_corner_it_is_dragged_across() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("terrain-drag") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let water = index.terrain("Water").unwrap();
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("Water").click();
    h.run_steps(2);
    let before = lattice(&mut h);
    // Dragged from corner (1, 2) to corner (3, 2), 10 m apart each, then
    // run back to (2, 2): (3, 2) is let go.
    let from = screen(&h, area, Vec3::new(10.0, 20.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for x in [12.0, 16.0, 20.0, 24.0, 28.0, 30.0, 27.0, 23.0, 20.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 20.0, 0.0)));
        h.run_steps(1);
    }
    // Nothing is painted before the button is let go.
    assert_eq!(lattice(&mut h), before);
    let to = screen(&h, area, Vec3::new(20.0, 20.0, 0.0));
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let after = lattice(&mut h);
    for x in 1..=2 {
        assert_eq!(after.corner(x, 2).terrain, water, "corner ({x}, 2)");
    }
    for x in [0, 3] {
        assert_eq!(after.corner(x, 2).terrain, before.corner(x, 2).terrain, "corner ({x}, 2)");
    }
    // The drag is one command: one undo takes all of it back.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), before);
    // With Ctrl held the press is the camera's (Ctrl + drag moves the
    // view): the brush paints nothing, dragged or let go where it was.
    let ctrl = egui::Modifiers::COMMAND;
    press(&h, from, true, ctrl);
    h.run_steps(1);
    for x in [12.0, 16.0, 20.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 20.0, 0.0)));
        h.run_steps(1);
    }
    let at = h.state().area_views[&area].screen_pos(Vec3::new(20.0, 20.0, 0.0)).unwrap_or(to);
    press(&h, at, false, ctrl);
    h.run_steps(3);
    let spot = screen(&h, area, Vec3::new(20.0, 20.0, 0.0));
    h.hover_at(spot);
    press(&h, spot, true, ctrl);
    press(&h, spot, false, ctrl);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), before, "a Ctrl + drag and a Ctrl + click paint nothing");
    // And the next plain click paints as before.
    press(&h, spot, true, egui::Modifiers::NONE);
    press(&h, spot, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_ne!(lattice(&mut h), before);
}

#[test]
fn cursors_that_only_choose_tiles_again_are_blue() {
    use glam::Vec3;
    use mg_ui::terrain_mode::CYCLE;
    let Some((mut h, area)) = area_harness("cycle-cursor") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    let colors = |h: &Harness<'_, Moonglow>| -> Vec<egui::Color32> {
        h.state().area_views[&area].brush_cursor.iter().map(|(_, c)| *c).collect()
    };
    // A crosser over grass: a drag from there lays it, green.
    let green = egui::Color32::from_rgb(80, 220, 80);
    h.get_by_label("Road").click();
    h.run_steps(2);
    let at = screen(&h, area, Vec3::new(13.0, 15.0, 0.0));
    h.hover_at(at);
    h.run_steps(3);
    assert_eq!(colors(&h), [green]);
    press(&h, at, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for x in [16.0, 19.0, 22.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 15.0, 0.0)));
        h.run_steps(1);
    }
    h.run_steps(2);
    assert!(!colors(&h).contains(&CYCLE), "{:?}", colors(&h));
    let to = screen(&h, area, Vec3::new(22.0, 15.0, 0.0));
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    // Over the road it laid, a click only chooses the tile again: blue.
    h.hover_at(screen(&h, area, Vec3::new(19.0, 15.0, 0.0)));
    h.run_steps(3);
    assert_eq!(colors(&h), [CYCLE]);
    // The Eraser erases, and with Shift steps the tile through those that
    // fit: blue then.
    h.get_by_label("🗑 Eraser").click();
    h.run_steps(2);
    h.hover_at(at);
    h.run_steps(3);
    assert!(!colors(&h).contains(&CYCLE), "{:?}", colors(&h));
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    h.run_steps(3);
    assert_eq!(colors(&h), [CYCLE]);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(1);
    // A terrain brush on a corner of its own terrain adds none, it chooses
    // the tiles again: blue; elsewhere it paints. (Corner (3, 3), clear of
    // the road.)
    h.get_by_label("Water").click();
    h.run_steps(2);
    let corner = screen(&h, area, Vec3::new(30.0, 30.0, 0.0));
    h.hover_at(corner);
    h.run_steps(3);
    assert_eq!(colors(&h), [egui::Color32::from_rgb(80, 220, 80)], "grass: it paints");
    for pressed in [true, false] {
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.event(egui::Event::PointerButton { pos: corner, button, pressed, modifiers });
    }
    h.run_steps(3);
    assert_eq!(colors(&h), [CYCLE], "water on water");
}

#[test]
fn the_eraser_erases_every_tile_it_is_dragged_across() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("eraser-drag") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    let drag = |h: &mut Harness<'_, Moonglow>, path: &[Vec3]| {
        let from = screen(h, area, path[0]);
        h.hover_at(from);
        press(h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for &p in &path[1..] {
            h.hover_at(screen(h, area, p));
            h.run_steps(1);
        }
        press(h, screen(h, area, path[path.len() - 1]), false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("Road").click();
    h.run_steps(2);
    // Two roads, along the first row and the third.
    for y in [5.0, 25.0] {
        let path: Vec<Vec3> =
            [7.0, 12.0, 18.0, 24.0, 25.0].iter().map(|&x| Vec3::new(x, y, 0.0)).collect();
        drag(&mut h, &path);
    }
    let roads = lattice(&mut h);
    let road = |l: &mg_tiles::Lattice, y: u32| l.cell(1, y).edges.iter().any(Option::is_some);
    assert!(road(&roads, 0) && road(&roads, 2), "two roads drawn");

    // One Eraser drag down across both: each is erased.
    h.get_by_label("🗑 Eraser").click();
    h.run_steps(2);
    let path: Vec<Vec3> =
        [5.0, 10.0, 15.0, 20.0, 25.0].iter().map(|&y| Vec3::new(15.0, y, 0.0)).collect();
    drag(&mut h, &path);
    let l = lattice(&mut h);
    assert!(!road(&l, 0) && !road(&l, 2), "both roads erased");

    // As one command.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), roads);
}

#[test]
fn a_right_click_with_a_crosser_on_its_own_tile_erases_it() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("crosser-right-click") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
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
        h.run_steps(2);
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("Road").click();
    h.run_steps(2);
    // A road along the first row.
    let from = screen(&h, area, Vec3::new(7.0, 5.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    for x in [12.0, 18.0, 24.0, 25.0] {
        h.hover_at(screen(&h, area, Vec3::new(x, 5.0, 0.0)));
        h.run_steps(1);
    }
    press(&h, screen(&h, area, Vec3::new(25.0, 5.0, 0.0)), false, egui::Modifiers::NONE);
    h.run_steps(2);
    let road = lattice(&mut h);
    assert!(road.cell(1, 0).edges.iter().any(Option::is_some), "a road drawn");

    // On grass, the brush still chosen: nothing is erased.
    right_click(&mut h, Vec3::new(15.0, 25.0, 0.0));
    assert_eq!(lattice(&mut h), road);
    // On the road's east quarter of tile (1, 0): the tile is erased, as the
    // Eraser's click there, and the brush stays.
    right_click(&mut h, Vec3::new(18.0, 5.0, 0.0));
    assert!((0..4).all(|x| lattice(&mut h).cell(x, 0).edges == [None; 4]));
    assert!(h.state().palette.tile_brush.is_some());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), road);

    // A stream across the road (along the second row, the road turned up
    // the second column): a right click with Road on their tile takes the
    // road, and the stream stays.
    let drag = |h: &mut Harness<'_, Moonglow>, path: &[Vec3]| {
        let from = screen(h, area, path[0]);
        h.hover_at(from);
        press(h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for &p in &path[1..] {
            h.hover_at(screen(h, area, p));
            h.run_steps(1);
        }
        press(h, screen(h, area, path[path.len() - 1]), false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    right_click(&mut h, Vec3::new(18.0, 5.0, 0.0));
    h.get_by_label("Stream").click();
    h.run_steps(2);
    let along: Vec<Vec3> =
        [3.0, 8.0, 14.0, 20.0, 26.0, 32.0, 37.0].iter().map(|&x| Vec3::new(x, 15.0, 0.0)).collect();
    drag(&mut h, &along);
    h.get_by_label("Road").click();
    h.run_steps(2);
    let up: Vec<Vec3> =
        [3.0, 8.0, 14.0, 20.0, 26.0, 32.0, 37.0].iter().map(|&y| Vec3::new(15.0, y, 0.0)).collect();
    drag(&mut h, &up);
    let (road_c, stream_c) = (index.crosser("Road").unwrap(), index.crosser("Stream").unwrap());
    let has = |l: &mg_tiles::Lattice, c| l.cell(1, 1).edges.contains(&Some(c));
    let crossing = lattice(&mut h);
    assert!(has(&crossing, road_c) && has(&crossing, stream_c), "{:?}", crossing.cell(1, 1));
    right_click(&mut h, Vec3::new(15.0, 18.0, 0.0));
    let l = lattice(&mut h);
    assert!(!has(&l, road_c) && has(&l, stream_c), "{:?}", l.cell(1, 1));
}

#[test]
fn refine_tile_steps_a_tile_through_those_that_fit_and_paints_nothing() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("refine-tile") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    // (Its place among the tools: `tile_tools_change_a_tile_s_appearance_and_turn_it`.)
    h.get_by_label("🔁 Refine Tile").click();
    h.run_steps(2);
    let before = grid(&mut h);
    let next = mg_tiles::paint::next_fit(&index, &before.lattice.cell(1, 1), before.tile(1, 1))
        .expect("Rural's grass has other tiles");
    // (Previous Variant: a step back from it is where it was.)
    let back = mg_tiles::paint::step_fit(&index, &before.lattice.cell(1, 1), next, true);
    assert_eq!(back, Some(before.tile(1, 1)));
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    h.run_steps(3);
    let cursor: Vec<_> = h.state().area_views[&area].brush_cursor.iter().map(|(_, c)| *c).collect();
    assert_eq!(cursor, [mg_ui::terrain_mode::CYCLE]);
    for pressed in [true, false] {
        let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
        h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
    }
    h.run_steps(3);
    let after = grid(&mut h);
    assert_eq!(after.tile(1, 1), next, "the next tile that fits");
    assert_eq!(after.lattice, before.lattice, "no terrain painted");
    for (x, y) in [(0, 1), (2, 1), (1, 0), (1, 2)] {
        assert_eq!(after.tile(x, y), before.tile(x, y), "({x}, {y}) is as it was");
    }
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(grid(&mut h).tile(1, 1), before.tile(1, 1));
}

/// A right click on a tile opens the tile's items without Select Tiles
/// (a builder's tile feedback): on the ground with no object there, the
/// area's menu has them (as Aurora's has Tile Properties for a tile); on
/// an object, not. With a terrain brush, the tile menu, the brush kept.
#[test]
fn a_right_click_on_a_tile_opens_its_menu_without_select_tiles() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-menu") else { return };
    let right_click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
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
    assert!(!h.state().area_views[&area].tile_mode);
    // The ground of tile (1, 1): its items first, the area's after.
    let ground = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    right_click(&mut h, ground);
    let items = menu_items(&h, "Set Start Location Here");
    let dir = mg_testkit::scratch_dir("ui-tile-menu-ground");
    h.render().unwrap().save(dir.join("ground.png")).unwrap();
    assert_eq!(
        items[..4],
        ["Next Tile Appearance", "Previous Tile Appearance", "Rotate Tile", "Tile Properties…"],
        "{items:?}"
    );
    h.get_by_label("Tile Properties…").click();
    h.run_steps(2);
    let props = h.state().tile_props.clone().expect("Tile Properties is open");
    assert_eq!(props.tiles, [5]);
    assert!(!h.state().area_views[&area].tile_mode, "still working on objects");
    h.state_mut().tile_props = None;
    h.run_steps(2);
    // A waypoint: its menu, no tile's items.
    let waypoint = screen(&h, area, Vec3::new(20.0, 20.0, 0.0));
    right_click(&mut h, waypoint);
    let items = menu_items(&h, "Properties");
    assert!(!items.iter().any(|i| i.contains("Tile")), "{items:?}");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    // Painting Grass: a right click is the tile menu, and Grass stays.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("Grass").click();
    h.run_steps(2);
    right_click(&mut h, ground);
    let items = menu_items(&h, "Tile Properties…");
    assert_eq!(
        items,
        [
            "Put Down Grass",
            "Next Tile Appearance",
            "Previous Tile Appearance",
            "Rotate Tile",
            "Tile Properties…"
        ]
    );
    assert!(h.state().palette.tile_brush.is_some(), "the brush kept");
    let dir = mg_testkit::scratch_dir("ui-tile-menu");
    h.render().unwrap().save(dir.join("painting.png")).unwrap();
    h.get_by_label("Put Down Grass").click();
    h.run_steps(2);
    assert!(h.state().palette.tile_brush.is_none());
    // The palette's Select Tiles: the view's tile mode, the brush let go.
    h.get_by_label("↕ Raise/Lower").click();
    h.run_steps(2);
    h.get_by_label("⛶ Select Tiles (F10)").click();
    h.run_steps(2);
    assert!(h.state().area_views[&area].tile_mode);
    assert!(h.state().palette.tile_brush.is_none());
    // A brush chosen: out of tile mode.
    h.get_by_label("Grass").click();
    h.run_steps(2);
    assert!(!h.state().area_views[&area].tile_mode);
}

/// Next and Previous Tile Appearance and Rotate Tile, from the tiles
/// palette and the tile menu (a builder's tile feedback): another tile of
/// the kind, the same tile turned (its lights kept), each one undoable
/// step; the terrain as it was.
#[test]
fn tile_tools_change_a_tile_s_appearance_and_turn_it() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    use mg_tiles::paint::{step_appearance, turn_fit};
    let Some((mut h, area)) = area_harness("tile-tools") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let tile = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        are.root.list("Tile_List").unwrap()[5].clone()
    };
    let undo_label =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_string);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    // The tools, in order: Select Tiles, the Eraser, Refine Tile, the
    // three, Raise/Lower; then the terrains (Rural's Grass, Road, Water…).
    let top = |h: &Harness<'_, Moonglow>, label: &str| h.get_by_label(label).rect().top();
    let order = [
        "⛶ Select Tiles (F10)",
        "🗑 Eraser",
        "🔁 Refine Tile",
        "⏭ Next Tile Appearance",
        "⏮ Previous Tile Appearance",
        "↺ Rotate Tile",
        "↕ Raise/Lower",
    ];
    for pair in order.windows(2) {
        assert!(top(&h, pair[0]) < top(&h, pair[1]), "{} above {}", pair[0], pair[1]);
    }
    for terrain in ["Grass", "Road", "Water"] {
        assert!(top(&h, "↕ Raise/Lower") < top(&h, terrain), "Raise/Lower is above {terrain}");
    }
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    let click = |h: &mut Harness<'_, Moonglow>| {
        h.hover_at(at);
        h.run_steps(2);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    let before = grid(&mut h);
    let cell = before.lattice.cell(1, 1);
    let now = before.tile(1, 1);
    // Next, then Previous: another tile, then back.
    h.get_by_label("⏭ Next Tile Appearance").click();
    h.run_steps(2);
    click(&mut h);
    let next = step_appearance(&index, &cell, now, false).expect("Rural's grass has others");
    assert_ne!(next.tile, now.tile);
    let g = grid(&mut h);
    assert_eq!(g.tile(1, 1), next);
    assert_eq!(g.lattice, before.lattice, "no terrain painted");
    assert_eq!(undo_label(&h).as_deref(), Some("Next tile appearance"));
    h.get_by_label("⏮ Previous Tile Appearance").click();
    h.run_steps(2);
    click(&mut h);
    assert_eq!(grid(&mut h).tile(1, 1), now, "a step back");
    assert_eq!(undo_label(&h).as_deref(), Some("Previous tile appearance"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(grid(&mut h).tile(1, 1), next, "undone");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(grid(&mut h).tile(1, 1), now, "undone");
    // Rotate: the same tile a quarter round, its lights as they were.
    let lights = |t: &mg_gff::Struct| {
        ["Tile_MainLight1", "Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2"]
            .map(|l| t.integer(l))
    };
    let was = tile(&mut h);
    h.get_by_label("↺ Rotate Tile").click();
    h.run_steps(2);
    click(&mut h);
    let turned = turn_fit(&index, &cell, now, false).expect("grass turns");
    assert_eq!(turned.tile, now.tile);
    assert_eq!(grid(&mut h).tile(1, 1), turned);
    assert_eq!(lights(&tile(&mut h)), lights(&was), "the lights kept");
    assert_eq!(undo_label(&h).as_deref(), Some("Rotate tile"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(tile(&mut h), was, "undone");
    // The tile menu's, in tile mode: on the tile selected.
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    h.get_by_label("⛶ Select Tiles (F10)").click();
    h.run_steps(2);
    click(&mut h);
    assert_eq!(h.state().area_views[&area].tile_selection, [(1, 1)]);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Rotate Tile").click();
    h.run_steps(3);
    assert_eq!(grid(&mut h).tile(1, 1), turned);
    assert_eq!(undo_label(&h).as_deref(), Some("Rotate tile"));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Next Tile Appearance").click();
    h.run_steps(3);
    let other = step_appearance(&index, &cell, turned, false).unwrap();
    assert_eq!(grid(&mut h).tile(1, 1), other);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(tile(&mut h), was, "both undone");
}

/// The tiles palette's Next and Previous Tile Appearance and Rotate Tile
/// on the tiles selected in Select Tiles (the owner's report: Next Tile
/// Appearance dropped the selection and changed nothing): each selected
/// tile steps on its own, as the tile menu steps them, one step to undo;
/// Select Tiles and the selection stay, so another click steps them again.
/// A group's tile refuses, saying why, the group in red.
#[test]
fn palette_tile_tools_step_the_selected_tiles() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    use mg_tiles::paint::{step_appearance, turn_fit};
    let Some((mut h, area)) = area_harness("tile-tools-selected") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let tiles = |h: &mut Harness<'_, Moonglow>| tile_list(h, area);
    let click = |h: &mut Harness<'_, Moonglow>, p: Vec3, m: egui::Modifiers| {
        let at = screen(h, area, p);
        h.hover_at(at);
        h.run_steps(2);
        press(h, at, true, m);
        press(h, at, false, m);
        h.run_steps(3);
    };
    // Select Tiles, its selection, and no brush chosen.
    let selecting = |h: &Harness<'_, Moonglow>| {
        let view = &h.state().area_views[&area];
        (view.tile_mode, view.tile_selection.clone(), h.state().palette.tile_brush.is_none())
    };
    let undo = |h: &mut Harness<'_, Moonglow>| {
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        h.run_steps(3);
    };
    let shots = mg_testkit::scratch_dir("ui-tile-tools-selected");
    let shot = |h: &mut Harness<'_, Moonglow>, name: &str| {
        h.run_steps(10);
        h.render().unwrap().save(shots.join(format!("{name}.png"))).unwrap();
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("⛶ Select Tiles (F10)").click();
    h.run_steps(2);
    let (a, b) = (Vec3::new(15.0, 15.0, 0.0), Vec3::new(25.0, 15.0, 0.0));
    click(&mut h, a, egui::Modifiers::NONE);
    let one = vec![(1, 1)];
    assert_eq!(selecting(&h), (true, one.clone(), true));
    let before = grid(&mut h);
    let list = tiles(&mut h);
    let cell = |c: (u32, u32)| before.lattice.cell(c.0, c.1);
    let now = before.tile(1, 1);

    // Next: the selected tile steps, the selection stays; again, a step
    // further; each one step to undo.
    shot(&mut h, "one-before");
    h.get_by_label("⏭ Next Tile Appearance").click();
    h.run_steps(3);
    shot(&mut h, "one-next");
    let next = step_appearance(&index, &cell((1, 1)), now, false).expect("grass has others");
    assert_ne!(next.tile, now.tile);
    assert_eq!(grid(&mut h).tile(1, 1), next, "the selected tile's next appearance");
    assert_eq!(grid(&mut h).lattice, before.lattice, "no terrain painted");
    assert_eq!(selecting(&h), (true, one.clone(), true), "Select Tiles and the selection kept");
    assert_eq!(undo_label(&h).as_deref(), Some("Next tile appearance"));
    only_changed(&list, &tiles(&mut h), &[5]);
    h.get_by_label("⏭ Next Tile Appearance").click();
    h.run_steps(3);
    let further = step_appearance(&index, &cell((1, 1)), next, false).unwrap();
    assert_eq!(grid(&mut h).tile(1, 1), further, "a second click: a step further");
    undo(&mut h);
    assert_eq!(grid(&mut h).tile(1, 1), next, "one undo: the second step");
    undo(&mut h);
    assert_eq!(grid(&mut h), before, "one undo: the first");
    assert_eq!(tiles(&mut h), list);

    // Previous: a step back.
    h.get_by_label("⏮ Previous Tile Appearance").click();
    h.run_steps(3);
    let back = step_appearance(&index, &cell((1, 1)), now, true).unwrap();
    assert_eq!(grid(&mut h).tile(1, 1), back);
    assert_eq!(selecting(&h), (true, one.clone(), true));
    assert_eq!(undo_label(&h).as_deref(), Some("Previous tile appearance"));
    undo(&mut h);
    assert_eq!(grid(&mut h), before, "undone");

    // Rotate: the same tile a quarter round, its lights kept; with Shift,
    // the other way.
    h.get_by_label("↺ Rotate Tile").click();
    h.run_steps(3);
    let turned = turn_fit(&index, &cell((1, 1)), now, false).expect("grass turns");
    assert_eq!(grid(&mut h).tile(1, 1), turned);
    let lights = |t: &mg_gff::Struct| {
        ["Tile_MainLight1", "Tile_MainLight2", "Tile_SrcLight1", "Tile_SrcLight2"]
            .map(|l| t.integer(l))
    };
    assert_eq!(lights(&tiles(&mut h)[5]), lights(&list[5]), "the lights kept");
    assert_eq!(selecting(&h), (true, one.clone(), true));
    assert_eq!(undo_label(&h).as_deref(), Some("Rotate tile"));
    undo(&mut h);
    assert_eq!(tiles(&mut h), list, "undone");
    h.get_by_label("↺ Rotate Tile").click_modifiers(egui::Modifiers::SHIFT);
    h.run_steps(3);
    let clockwise = turn_fit(&index, &cell((1, 1)), now, true).unwrap();
    assert_eq!(grid(&mut h).tile(1, 1), clockwise, "Shift: clockwise");
    undo(&mut h);
    assert_eq!(tiles(&mut h), list, "undone");

    // Two tiles (Ctrl + click adds the second): each its own step, one
    // step to undo for both.
    click(&mut h, b, egui::Modifiers::COMMAND);
    let two = vec![(1, 1), (2, 1)];
    assert_eq!(selecting(&h), (true, two.clone(), true));
    shot(&mut h, "two-before");
    for (tool, label) in [
        ("⏭ Next Tile Appearance", "Next tile appearance"),
        ("⏮ Previous Tile Appearance", "Previous tile appearance"),
        ("↺ Rotate Tile", "Rotate tile"),
    ] {
        h.get_by_label(tool).click();
        h.run_steps(3);
        let g = grid(&mut h);
        for c in [(1, 1), (2, 1)] {
            let was = before.tile(c.0, c.1);
            let made = match tool {
                "↺ Rotate Tile" => turn_fit(&index, &cell(c), was, false),
                _ => step_appearance(&index, &cell(c), was, tool.contains("Previous")),
            };
            assert_eq!(g.tile(c.0, c.1), made.unwrap(), "{tool}: tile {c:?} on its own");
        }
        assert_eq!(g.lattice, before.lattice, "{tool}: no terrain painted");
        only_changed(&list, &tiles(&mut h), &[5, 6]);
        assert_eq!(selecting(&h), (true, two.clone(), true), "{tool}: the selection kept");
        assert_eq!(undo_label(&h).as_deref(), Some(label));
        if label == "Next tile appearance" {
            shot(&mut h, "two-next");
        }
        undo(&mut h);
        assert_eq!(tiles(&mut h), list, "{tool}: one undo for both");
    }

    // A group's tile: refused, saying why, the group in red; the
    // selection kept.
    h.get_by_label("Groups").click();
    h.run_steps(2);
    h.get_by_label("Barn 1 2x2").click();
    h.run_steps(2);
    click(&mut h, a, egui::Modifiers::NONE);
    h.get_by_label("Groups").click();
    h.run_steps(2);
    let placed = grid(&mut h);
    assert!(index.is_grouped(placed.tile(2, 1).tile), "the barn placed");
    h.get_by_label("⛶ Select Tiles (F10)").click();
    h.run_steps(2);
    click(&mut h, b, egui::Modifiers::NONE);
    let east = vec![(2, 1)];
    assert_eq!(selecting(&h), (true, east.clone(), true));
    for (tool, label) in
        [("⏭ Next Tile Appearance", "Next tile appearance"), ("↺ Rotate Tile", "Rotate tile")]
    {
        h.state_mut().area_views.get_mut(&area).unwrap().refused = None;
        h.get_by_label(tool).click();
        h.run_steps(3);
        assert_eq!(grid(&mut h), placed, "{tool}: nothing changed");
        let says = format!("{label}: tiles of a group or feature");
        assert!(h.query_by_label_contains(&says).is_some(), "{tool}: the status line says why");
        let mut red = h.state().area_views[&area].refused.clone().expect("in red").0;
        red.sort();
        assert_eq!(red, [(1, 1), (1, 2), (2, 1), (2, 2)], "{tool}: the barn in red");
        assert_eq!(selecting(&h), (true, east.clone(), true), "{tool}: the selection kept");
    }
    shot(&mut h, "group-refused");

    // No selection: the tool is chosen, as ever, Select Tiles let go.
    h.state_mut().area_views.get_mut(&area).unwrap().tile_selection.clear();
    h.run_steps(2);
    h.get_by_label("⏭ Next Tile Appearance").click();
    h.run_steps(2);
    let brush = h.state().palette.tile_brush.clone().map(|b| b.brush);
    assert_eq!(brush, Some(mg_area::terrain::Brush::Appearance { back: false }));
    assert!(!h.state().area_views[&area].tile_mode);
    assert_eq!(grid(&mut h), placed, "nothing changed until a tile is clicked");
}

/// Tile Properties' variants' pictures take their size from their
/// slider's setting, kept in the settings (a builder's tile feedback:
/// tiles much alike couldn't be told apart small).
#[test]
fn tile_variant_pictures_take_the_size_set() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-variant-size") else { return };
    h.state_mut().settings.tile_variant_side = Some(200);
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
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
    h.get_by_label("Tile Properties…").click();
    h.run_steps(2);
    let (_, variants) = h.state().tile_props.clone().unwrap().variants.unwrap();
    // (The pictures made, to look at.)
    for _ in 0..400 {
        h.run_steps(1);
        if h.state().thumbnails.waiting() == 0 {
            break;
        }
    }
    // How far apart the first two are, across (their names, under them).
    let apart = |h: &Harness<'_, Moonglow>| {
        let rect = |k: usize| {
            let (p, model) = &variants[k];
            let turn = ["", " ↺90°", " ↺180°", " ↺270°"][usize::from(p.orientation % 4)];
            h.get_by_label(&format!("{model}{turn}")).rect()
        };
        let (a, b) = (rect(0), rect(1));
        assert_eq!(a.top(), b.top(), "on one row");
        b.left() - a.left()
    };
    assert!((apart(&h) - 214.0).abs() < 1.0, "200 points and the gaps: {}", apart(&h));
    h.get_by_label("Size");
    let dir = mg_testkit::scratch_dir("ui-tile-variant-size");
    h.render().unwrap().save(dir.join("large.png")).unwrap();
    // The usual size, unless set.
    h.state_mut().settings.tile_variant_side = None;
    h.run_steps(3);
    assert!((apart(&h) - 98.0).abs() < 1.0, "{}", apart(&h));
    // Small, a picture under the pointer shows at its largest beside it.
    let (p, model) = &variants[1];
    let turn = ["", " ↺90°", " ↺180°", " ↺270°"][usize::from(p.orientation % 4)];
    let name = h.get_by_label(&format!("{model}{turn}")).rect();
    h.hover_at(name.center_top() - egui::vec2(0.0, 40.0));
    h.run_steps(60);
    h.render().unwrap().save(dir.join("hovered.png")).unwrap();
    h.render().unwrap().save(dir.join("usual.png")).unwrap();
}

/// The model of tile `tile` of the test area's tileset (Rural): its
/// picture's name in the tiles palette's Gallery.
fn index_model(h: &Harness<'_, Moonglow>, tile: u32) -> String {
    let game = h.state().game.as_deref().unwrap();
    let set = mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap();
    set.tiles[tile as usize].model.clone()
}

/// Finds `name` (a heading's or a tile's model) in the tiles palette's
/// search: what it finds shows, its heading opened out. (A picture out of
/// sight has no name to find it by.)
fn open_tile_heading(h: &mut Harness<'_, Moonglow>, name: &str) {
    h.state_mut().palette.filter = name.to_lowercase();
    h.run_steps(3);
}

/// The tiles palette's Gallery: a terrain's tiles as pictures; one chosen
/// is placed as it is in every tile a drag passes, as one step.
#[test]
fn a_tile_chosen_in_the_gallery_is_painted_along_a_drag() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("tile-gallery") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let before = grid(&mut h);
    // A grass tile other than the one in (1, 1): flat grass all round, no
    // crosser (a road's would bring the road into the tiles beside it).
    let grass = index.terrain("Grass").unwrap();
    let all_grass = index.tiles_where(|c| {
        c.corners.iter().all(|k| k.terrain == grass && k.height == 0) && c.edges == [None; 4]
    });
    let tile = *all_grass.iter().find(|t| **t != before.tile(1, 1).tile).expect("grass variants");
    h.state_mut().settings.palette_gallery = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(3);
    // (Scrolled to: the pictures under the headings above take the
    // pane's height.)
    // (Its picture is named by its model, under its heading, opened.)
    let picture = index_model(&h, tile);
    open_tile_heading(&mut h, &picture);
    h.get_all_by_label(&picture).next().expect("its picture under Grass").scroll_to_me();
    h.run_steps(3);
    h.get_all_by_label(&picture).next().unwrap().click();
    h.run_steps(2);
    let brush = h.state().palette.tile_brush.clone().expect("chosen");
    assert_eq!(brush.brush, mg_area::terrain::Brush::Tile(tile));
    // Dragged across the second row's tiles.
    let from = screen(&h, area, Vec3::new(5.0, 15.0, 0.0));
    let to = screen(&h, area, Vec3::new(35.0, 15.0, 0.0));
    h.hover_at(from);
    h.run_steps(2);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=8 {
        h.hover_at(from + (to - from) * (k as f32 / 8.0));
        h.run_steps(1);
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let after = grid(&mut h);
    for x in 0..4 {
        assert_eq!(after.tile(x, 1).tile, tile, "({x}, 1)");
    }
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(grid(&mut h).tiles, before.tiles, "one step");
}

/// The area view as it is now, and seen from the south-west, a little
/// above: saved as `path` (`-top.png` and `-angled.png`), the camera put
/// back after.
fn area_shots(h: &mut Harness<'_, Moonglow>, area: ResRef, path: &std::path::Path) {
    h.hover_at(egui::pos2(960.0, 610.0));
    h.run_steps(20);
    h.render().unwrap().save(path.with_extension("top.png")).unwrap();
    let was = h.state().area_views[&area].orbit;
    if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
        o.target = glam::Vec3::new(20.0, 20.0, 0.0);
        (o.distance, o.pitch, o.yaw) = (48.0, 0.55, (-120f32).to_radians());
    }
    h.run_steps(20);
    h.render().unwrap().save(path.with_extension("angled.png")).unwrap();
    h.state_mut().area_views.get_mut(&area).unwrap().orbit = was;
    h.run_steps(3);
}

/// The area's `Tile_List`, as the workspace has it.
fn tile_list(h: &mut Harness<'_, Moonglow>, area: ResRef) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().root.list("Tile_List").unwrap().to_vec()
}

/// Asserts that `after` is `before` but for the tiles of `changed`
/// (indices), struct for struct.
fn only_changed(before: &[mg_gff::Struct], after: &[mg_gff::Struct], changed: &[usize]) {
    assert_eq!(before.len(), after.len());
    for (i, (b, a)) in before.iter().zip(after).enumerate() {
        if changed.contains(&i) {
            assert_ne!(b, a, "tile {i} changed");
        } else {
            assert_eq!(b, a, "tile {i} as it was");
        }
    }
}

/// The tiles palette's Exact tile switch (#29): shown beside List and
/// Gallery, off at first and remembered; on, a tile chosen in the Gallery
/// goes in alone where it is clicked (or dragged), as it is, turned as the
/// right click turns it, at the height of the tile it replaces, though it
/// fits none of its neighbors: no other tile changes. One step to undo.
#[test]
fn an_exact_tile_goes_in_alone() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("exact-tile") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    // The end of a wall (Wall1 across one edge, grass all round): in open
    // grass it fits nothing (painted as ever, the wall would run on into
    // the next tile).
    let (wall, grass) = (index.crosser("Wall1").unwrap(), index.terrain("Grass").unwrap());
    let tile = *index
        .tiles_where(|c| {
            c.edges.iter().filter(|e| **e == Some(wall)).count() == 1
                && c.corners.iter().all(|k| k.terrain == grass && k.height == 0)
        })
        .first()
        .expect("Rural has wall ends");
    let shots = mg_testkit::scratch_dir("ui-exact-tile");
    let away = egui::pos2(960.0, 610.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(3);
    // Not shown with the List and no tile chosen; beside the Gallery's
    // switch with it.
    assert!(h.query_by_label("📌 Exact tile").is_none());
    h.get_by_label("⊞").click();
    h.run_steps(2);
    assert!(!h.state().settings.exact_tiles, "off at first");
    h.get_by_label("📌 Exact tile").click();
    h.run_steps(2);
    assert!(h.state().settings.exact_tiles, "on, and remembered");
    let picture = index_model(&h, tile);
    open_tile_heading(&mut h, &picture);
    h.get_all_by_label(&picture).next().expect("its picture").scroll_to_me();
    h.run_steps(3);
    h.get_all_by_label(&picture).next().unwrap().click();
    h.run_steps(2);
    h.hover_at(away);
    h.run_steps(30);
    h.render().unwrap().save(shots.join("palette.png")).unwrap();
    let before = grid(&mut h);
    let list = tile_list(&mut h, area);
    area_shots(&mut h, area, &shots.join("before"));
    // A click in tile (1, 1): that tile there, its neighbors untouched.
    let at = screen(&h, area, Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    h.run_steps(2);
    h.render().unwrap().save(shots.join("preview.png")).unwrap();
    // (The preview shows that tile alone.)
    let view = &h.state().area_views[&area];
    let shown = view.tile_preview.clone();
    assert_eq!(shown.len(), 1, "that tile alone");
    assert_eq!((shown[0].id, shown[0].column, shown[0].row), (i64::from(tile), 1, 1));
    assert_eq!(view.brush_cursor.len(), 1, "one tile's square");
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let after = grid(&mut h);
    let height = before.tile(1, 1).height;
    assert_eq!(after.tile(1, 1), mg_tiles::Placement { tile, orientation: 0, height });
    only_changed(&list, &tile_list(&mut h, area), &[5]);
    assert_eq!(
        h.state().ws.as_ref().unwrap().can_undo(),
        Some(format!("Exact tile {picture}").as_str())
    );
    area_shots(&mut h, area, &shots.join("after"));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), list, "one step");
    // Turned with a right click, dragged along the third row: those four
    // tiles, turned, and no other.
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(2);
    let from = screen(&h, area, Vec3::new(5.0, 25.0, 0.0));
    let to = screen(&h, area, Vec3::new(35.0, 25.0, 0.0));
    h.hover_at(from);
    h.run_steps(2);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=8 {
        h.hover_at(from + (to - from) * (k as f32 / 8.0));
        h.run_steps(1);
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let row = grid(&mut h);
    for x in 0..4 {
        assert_eq!((row.tile(x, 2).tile, row.tile(x, 2).orientation), (tile, 1), "({x}, 2)");
    }
    only_changed(&list, &tile_list(&mut h, area), &[8, 9, 10, 11]);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), list, "the drag: one step");
    // Off: the same tile is fitted in, as before, or refused where it can't.
    h.get_by_label("📌 Exact tile").click();
    h.run_steps(2);
    assert!(!h.state().settings.exact_tiles);
}

/// Tile Properties' Height (#29): a tile raised two steps with its +
/// button stands two steps up, nothing else of the area's tiles changed,
/// what stands on it raised with it; one step to undo. Several tiles of
/// different heights show as mixed; Escape with only the height changed
/// asks first.
#[test]
fn tile_properties_raise_one_tile_alone() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-height") else { return };
    let list = tile_list(&mut h, area);
    let waypoints = |h: &mut Harness<'_, Moonglow>| -> Vec<f32> {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let w = git.root.list("WaypointList").unwrap();
        w.iter().map(|s| s.float("ZPosition").unwrap_or(0.0)).collect()
    };
    let shots = mg_testkit::scratch_dir("ui-tile-height");
    area_shots(&mut h, area, &shots.join("before"));
    // Tile (2, 2), under the waypoint at (20, 20); the one at (30, 20) is
    // on its neighbor.
    tile_properties_of(&mut h, area, &[Vec3::new(25.0, 25.0, 0.0)]);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!(props.tiles, [10]);
    let was = props.height.expect("one tile: its height");
    assert!(!props.height_locked);
    h.get_by_label("Raise Height").click();
    h.run_steps(2);
    h.get_by_label("Raise Height").click();
    h.run_steps(2);
    assert_eq!(h.state().tile_props.as_ref().unwrap().height, Some(was + 2));
    h.render().unwrap().save(shots.join("window.png")).unwrap();
    h.get_by_label("Lower Height").click();
    h.run_steps(2);
    h.get_by_label("Raise Height").click();
    h.run_steps(2);
    h.get_by_label("OK").click();
    h.run_steps(4);
    let now = tile_list(&mut h, area);
    assert_eq!(now[10].integer("Tile_Height"), Some(i64::from(was + 2)));
    only_changed(&list, &now, &[10]);
    let step = h.state().area_views[&area].model.as_ref().unwrap().height_step;
    let shown = &h.state().area_views[&area].model.as_ref().unwrap().tiles[10];
    assert_eq!(shown.position.z, (was + 2) as f32 * step, "drawn at its height");
    assert_eq!(waypoints(&mut h), [2.0 * step, 0.0], "what stands on it rises with it");
    h.get_by_label("⛶ Select Tiles").click();
    area_shots(&mut h, area, &shots.join("after"));
    // Its walkmesh with it: the ground the game walks on (green).
    h.state_mut().area_views.get_mut(&area).unwrap().walkmesh = true;
    area_shots(&mut h, area, &shots.join("walkmesh"));
    h.state_mut().area_views.get_mut(&area).unwrap().walkmesh = false;
    h.get_by_label("⛶ Select Tiles").click();
    h.run_steps(2);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tile Properties"));
    // Two tiles of different heights: mixed; Escape with it set asks.
    tile_properties_of(&mut h, area, &[Vec3::new(25.0, 25.0, 0.0), Vec3::new(35.0, 25.0, 0.0)]);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert_eq!(props.height, None, "mixed");
    assert!(!props.changed());
    h.get_by_label("Raise Height").click();
    h.run_steps(2);
    let p = h.state().tile_props.clone().unwrap();
    assert_eq!(p.height, Some(was + 3), "from the first tile's");
    assert!(p.changed());
    h.state_mut().tile_props.as_mut().unwrap().closing = true;
    h.run_steps(2);
    h.get_by_label("Save changes to Tile Properties?");
    h.get_by_label("Don't Save").click();
    h.run_steps(2);
    assert!(h.state().tile_props.is_none());
    // Undone: as it was.
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), list);
    assert_eq!(waypoints(&mut h), [0.0, 0.0]);
}

/// Tile Properties' All tiles (ignore fit) (#29): off at first, the
/// variants that fit as ever; on, every tile of the tileset as pictures
/// (drawn as they come into sight), and one clicked goes in the tile
/// chosen as it is, turned as chosen, though it fits nothing there: no
/// other tile changes, one step to undo.
#[test]
fn tile_properties_put_any_tile_alone() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-all") else { return };
    let list = tile_list(&mut h, area);
    tile_properties_of(&mut h, area, &[Vec3::new(15.0, 15.0, 0.0)]);
    let props = h.state().tile_props.clone().expect("the window is open");
    assert!(!props.all_tiles, "off at first");
    assert!(h.query_by_label_contains("fit here)").is_some(), "the variants as ever");
    let fits: Vec<u32> = props.variants.clone().unwrap().1.iter().map(|(p, _)| p.tile).collect();
    h.get_by_label(mg_ui::tile_select::ALL_TILES).click();
    h.run_steps(3);
    assert!(h.state().tile_props.as_ref().unwrap().all_tiles);
    assert!(h.query_by_label_contains("fit here)").is_none());
    h.get_by_label_contains("in the tileset)").scroll_to_me();
    h.get_by_label("↺90°").click();
    h.run_steps(3);
    // (The Turn turns the tile chosen at once, a step of its own.)
    assert_eq!(list[5].integer("Tile_Orientation"), Some(0));
    assert_eq!(tile_list(&mut h, area)[5].integer("Tile_Orientation"), Some(1));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Turn tile (ignoring fit)"));
    // (Some frames, for the pictures in sight to be made.)
    let start = std::time::Instant::now();
    h.run_steps(20);
    eprintln!("20 frames of every tile's pictures: {:?}", start.elapsed());
    let shots = mg_testkit::scratch_dir("ui-tile-all");
    h.render().unwrap().save(shots.join("all-tiles.png")).unwrap();
    // A tile in sight that doesn't fit there, clicked by its picture.
    let props = h.state().tile_props.clone().unwrap();
    let (id, model) = props
        .every
        .iter()
        .find(|(id, m)| !fits.contains(id) && h.query_by_label(&format!("{m} ↺90°")).is_some())
        .cloned()
        .expect("a tile that doesn't fit, in sight");
    let rows = props.every.len();
    let drawn = h.query_all_by_label_contains(" ↺90°").count();
    assert!(drawn < rows, "only the rows in sight are drawn: {drawn} of {rows}");
    let at = h.get_all_by_label(&format!("{model} ↺90°")).next().unwrap().rect().center_top()
        - egui::vec2(0.0, 40.0);
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(4);
    let now = tile_list(&mut h, area);
    assert_eq!(now[5].integer("Tile_ID"), Some(i64::from(id)));
    assert_eq!(now[5].integer("Tile_Orientation"), Some(1));
    assert_eq!(now[5].integer("Tile_Height"), list[5].integer("Tile_Height"));
    only_changed(&list, &now, &[5]);
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tile (ignoring fit)"));
    let after = h.state().tile_props.clone().expect("the window stays");
    assert!(after.all_tiles && after.turn == 1, "still every tile, turned so");
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area)[5].integer("Tile_ID"), list[5].integer("Tile_ID"));
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Turn tile (ignoring fit)"));
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), list, "one step each");
}

/// The area's GIT root, as the workspace has it.
fn git_root(h: &mut Harness<'_, Moonglow>, area: ResRef) -> mg_gff::Struct {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&ResKey::new(area, ResType::GIT)).unwrap().root.clone()
}

/// Tile Properties' window moved over the palettes, so that the pictures
/// show the area beside it.
fn props_aside(h: &mut Harness<'_, Moonglow>) {
    let path = h.state().dock.find_tab(&Tab::TileProperties).expect("Tile Properties is open");
    if let Some(w) = h.state_mut().dock.get_window_state_mut(path.surface) {
        w.set_position(egui::pos2(842.0, 60.0)).set_size(egui::vec2(255.0, 600.0));
    }
    h.run_steps(3);
}

/// [`area_shots`] with Tile Properties open beside the area: the view
/// looking at `at` from the south-west, the area moved left of the
/// window (`PATH.top.png` straight down as it opened, `PATH.angled.png`).
fn shots_beside(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    at: glam::Vec3,
    path: &std::path::Path,
) {
    h.hover_at(egui::pos2(960.0, 610.0));
    h.run_steps(20);
    h.render().unwrap().save(path.with_extension("top.png")).unwrap();
    let was = h.state().area_views[&area].orbit;
    if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
        // (East of the tile: it shows left of the window.)
        o.target = at + glam::Vec3::new(9.0, -4.0, 0.0);
        (o.distance, o.pitch, o.yaw) = (34.0, 0.55, (-120f32).to_radians());
    }
    h.run_steps(20);
    h.render().unwrap().save(path.with_extension("angled.png")).unwrap();
    h.state_mut().area_views.get_mut(&area).unwrap().orbit = was;
    h.run_steps(3);
}

/// The workspace's last command's name.
fn undo_label(h: &Harness<'_, Moonglow>) -> Option<String> {
    h.state().ws.as_ref().unwrap().can_undo().map(str::to_string)
}

/// Tile Properties' changes show in the area as they are made (the
/// owner's report: Height and the lights waited for OK): a tile raised
/// with + stands raised in the area's Tile_List and its model at once,
/// what stands on it with it, the window still open; more raising, a
/// height dragged frame by frame and a light are one step to undo, which
/// OK keeps.
#[test]
fn tile_properties_changes_show_at_once() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-live") else { return };
    let (list, git, before) = (tile_list(&mut h, area), git_root(&mut h, area), undo_label(&h));
    let shots = mg_testkit::scratch_dir("ui-tile-live");
    // Tile (2, 2), index 10, under the waypoint at (20, 20).
    tile_properties_of(&mut h, area, &[Vec3::new(25.0, 25.0, 0.0)]);
    let props = h.state().tile_props.clone().expect("the window is open");
    let was = props.height.expect("one tile: its height");
    props_aside(&mut h);
    shots_beside(&mut h, area, glam::Vec3::new(25.0, 25.0, 0.0), &shots.join("height-before"));
    h.get_by_label("Raise Height").click();
    h.run_steps(2);
    // At once, before OK: in the area's file and in its view.
    assert!(h.state().tile_props.is_some(), "the window stays open");
    let now = tile_list(&mut h, area);
    assert_eq!(now[10].integer("Tile_Height"), Some(i64::from(was + 1)));
    only_changed(&list, &now, &[10]);
    let model = h.state().area_views[&area].model.clone().unwrap();
    let step = model.height_step;
    assert_eq!(model.tiles[10].position.z, (was + 1) as f32 * step, "drawn raised");
    let point = |h: &mut Harness<'_, Moonglow>| {
        let git = git_root(h, area);
        let w = git.list("WaypointList").unwrap();
        w.iter().map(|s| s.float("ZPosition").unwrap_or(0.0)).collect::<Vec<_>>()
    };
    assert_eq!(point(&mut h), [step, 0.0], "what stands on it with it");
    assert_eq!(undo_label(&h).as_deref(), Some("Tile Properties"));
    shots_beside(&mut h, area, glam::Vec3::new(25.0, 25.0, 0.0), &shots.join("height-raised"));
    // More: raised again, then a height dragged through frame by frame,
    // and a light chosen. One step for all.
    h.get_by_label("Raise Height").click();
    h.run_steps(2);
    for k in 3..=6 {
        h.state_mut().tile_props.as_mut().unwrap().height = Some(was + k);
        h.run_steps(1);
        assert_eq!(tile_list(&mut h, area)[10].integer("Tile_Height"), Some(i64::from(was + k)));
    }
    h.state_mut().tile_props.as_mut().unwrap().height = Some(was + 3);
    let light = props.has_main[0].then(|| (props.main[0] + 5) % 32);
    if let Some(l) = light {
        let p = h.state_mut().tile_props.as_mut().unwrap();
        p.main[0] = l;
        p.touched[0] = true;
    }
    h.run_steps(2);
    let now = tile_list(&mut h, area);
    assert_eq!(now[10].integer("Tile_Height"), Some(i64::from(was + 3)));
    if let Some(l) = light {
        assert_eq!(now[10].integer("Tile_MainLight1"), Some(i64::from(l)), "the light at once");
    }
    assert_eq!(point(&mut h), [3.0 * step, 0.0]);
    let model = h.state().area_views[&area].model.clone().unwrap();
    assert_eq!(model.tiles[10].position.z, (was + 3) as f32 * step);
    // Defaults: at once too, every control the tile has.
    h.get_by_label("Defaults").click();
    h.run_steps(2);
    let p = h.state().tile_props.clone().expect("the window stays");
    assert_eq!(p.touched, [true; 7]);
    let now = tile_list(&mut h, area);
    for (k, label) in mg_ui::tile_select::LIGHTS.iter().enumerate() {
        let (has, value) = match k {
            0 | 1 => (p.has_main[k], p.main[k]),
            2 | 3 => (p.has_source[k - 2], p.source[k - 2]),
            _ => (p.has_loops[k - 4], 1),
        };
        if has {
            assert_eq!(now[10].integer(label), Some(i64::from(value)), "Defaults' {label}");
        }
    }
    assert_eq!(now[10].integer("Tile_Height"), Some(i64::from(was + 3)), "the height stays");
    // OK keeps them: one step, which Undo takes back whole.
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert!(h.state().tile_props.is_none());
    assert_eq!(tile_list(&mut h, area), now, "kept");
    assert_eq!(undo_label(&h).as_deref(), Some("Tile Properties"));
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), list, "one step");
    assert_eq!(git_root(&mut h, area), git);
    assert_eq!(undo_label(&h), before);
}

/// Tile Properties' Cancel takes back what the window changed, byte for
/// byte, nothing left to redo: while its step is the last command, by
/// taking it back; after a variant put in from the pictures (its own
/// step, under the window's), the variant stays and the window's height
/// and light are taken off it; after a command made elsewhere, by a
/// step of its own that leaves that command's change.
#[test]
fn tile_properties_cancel_takes_the_changes_back() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-live-cancel") else { return };
    let (list, git, before) = (tile_list(&mut h, area), git_root(&mut h, area), undo_label(&h));
    let at = Vec3::new(25.0, 25.0, 0.0);
    let cancel = |h: &mut Harness<'_, Moonglow>| {
        let c = h.get_all_by_label("Cancel").last().unwrap().rect().center();
        h.hover_at(c);
        press(h, c, true, egui::Modifiers::NONE);
        press(h, c, false, egui::Modifiers::NONE);
        h.run_steps(3);
        assert!(h.state().tile_props.is_none(), "closed");
    };
    let set_light = |h: &mut Harness<'_, Moonglow>| {
        let p = h.state_mut().tile_props.as_mut().unwrap();
        assert!(p.has_main[0], "ttr01's tiles have main light 1");
        p.main[0] = (p.main[0] + 7) % 32;
        p.touched[0] = true;
        h.run_steps(2);
    };
    // Raised twice and a light: Cancel, and the area is as it was.
    tile_properties_of(&mut h, area, &[at]);
    let was = h.state().tile_props.as_ref().unwrap().height.unwrap();
    for _ in 0..2 {
        h.get_by_label("Raise Height").click();
        h.run_steps(2);
    }
    set_light(&mut h);
    assert_ne!(tile_list(&mut h, area), list, "shown at once");
    assert_ne!(git_root(&mut h, area), git);
    cancel(&mut h);
    assert_eq!(tile_list(&mut h, area), list, "the tiles byte for byte");
    assert_eq!(git_root(&mut h, area), git, "what stands on them too");
    assert_eq!(undo_label(&h), before);
    assert_eq!(h.state().ws.as_ref().unwrap().can_redo(), None, "nothing to redo");

    // Raised and a light, then a variant put in: the variant goes in at
    // the window's height and light (its step under the window's), and
    // Cancel takes them off it, the variant staying.
    tile_properties_of(&mut h, area, &[at]);
    h.get_by_label("Raise Height").click();
    h.run_steps(2);
    set_light(&mut h);
    let light = tile_list(&mut h, area)[10].integer("Tile_MainLight1");
    let props = h.state().tile_props.clone().unwrap();
    let (_, variants) = props.variants.clone().expect("one tile: its variants");
    let now = props.variant.unwrap();
    let other = variants.iter().find(|(p, _)| p.tile != now.tile).expect("another fits").clone();
    let turn = ["", " ↺90°", " ↺180°", " ↺270°"][usize::from(other.0.orientation % 4)];
    let picture = h
        .get_all_by_label(&format!("{}{turn}", other.1))
        .next()
        .map(|n| n.rect().center_top() - egui::vec2(0.0, 40.0))
        .expect("its picture");
    h.hover_at(picture);
    press(&h, picture, true, egui::Modifiers::NONE);
    press(&h, picture, false, egui::Modifiers::NONE);
    h.run_steps(4);
    let t = tile_list(&mut h, area)[10].clone();
    let id = t.integer("Tile_ID").unwrap();
    assert_ne!(id, list[10].integer("Tile_ID").unwrap(), "the variant is in");
    assert_eq!(t.integer("Tile_Height"), Some(i64::from(was + 1)), "at the window's height");
    assert_eq!(t.integer("Tile_MainLight1"), light, "with the window's light");
    assert_eq!(undo_label(&h).as_deref(), Some("Tile Properties"), "the window's step on top");
    let p = h.state().tile_props.clone().expect("the window stays");
    assert_eq!(p.base[0].integer("Tile_ID"), Some(id), "the window of the tile put in");
    cancel(&mut h);
    let t = tile_list(&mut h, area)[10].clone();
    assert_eq!(t.integer("Tile_ID"), Some(id), "the variant stays");
    assert_eq!(t.integer("Tile_Height"), list[10].integer("Tile_Height"), "its height back");
    assert_eq!(git_root(&mut h, area), git);
    assert_eq!(undo_label(&h).as_deref(), Some("Tile variant"));
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), list);
    assert_eq!(undo_label(&h), before);

    // Raised, then a tile changed elsewhere (the window's step no longer
    // the last): Cancel puts the height back in a step of its own, the
    // other change staying.
    tile_properties_of(&mut h, area, &[at]);
    h.get_by_label("Raise Height").click();
    h.run_steps(2);
    let elsewhere = list[0].integer("Tile_ID").unwrap() as i32 + 1;
    set_tile_id(&mut h, area, 0, elsewhere);
    assert!(h.state().tile_props.is_some());
    cancel(&mut h);
    let now = tile_list(&mut h, area);
    assert_eq!(now[10], list[10], "the window's tile as it was");
    assert_eq!(now[0].integer("Tile_ID"), Some(i64::from(elsewhere)), "the other change stays");
    assert_eq!(git_root(&mut h, area), git);
    assert_eq!(undo_label(&h).as_deref(), Some("Cancel Tile Properties"));
}

/// All tiles' Turn turns the tiles chosen at once, each its own tile (its
/// lights kept), as it is: no neighbor changes; one step to undo a turn.
/// The pictures show the turn chosen, for the next one clicked.
#[test]
fn tile_properties_turn_turns_the_tiles_at_once() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("tile-live-turn") else { return };
    // (A tile that looks turned: Rural's first, a cliff's corner.)
    set_tile_id(&mut h, area, 5, 0);
    let list = tile_list(&mut h, area);
    let before = undo_label(&h);
    let shots = mg_testkit::scratch_dir("ui-tile-live-turn");
    tile_properties_of(&mut h, area, &[Vec3::new(15.0, 15.0, 0.0)]);
    h.get_by_label(mg_ui::tile_select::ALL_TILES).click();
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), list, "the switch alone changes nothing");
    let o = list[5].integer("Tile_Orientation").unwrap();
    let to = (o + 2) % 4;
    let name = ["0°", "↺90°", "↺180°", "↺270°"][to as usize];
    props_aside(&mut h);
    shots_beside(&mut h, area, glam::Vec3::new(15.0, 15.0, 0.0), &shots.join("turn-before"));
    h.get_by_label_contains("in the tileset)").scroll_to_me();
    h.get_by_label(name).click();
    h.run_steps(3);
    let now = tile_list(&mut h, area);
    assert_eq!(now[5].integer("Tile_Orientation"), Some(to), "turned at once");
    assert_eq!(now[5].integer("Tile_ID"), list[5].integer("Tile_ID"), "its own tile");
    for label in mg_ui::tile_select::LIGHTS {
        assert_eq!(now[5].get(label), list[5].get(label), "{label} kept");
    }
    only_changed(&list, &now, &[5]);
    let shown = &h.state().area_views[&area].model.as_ref().unwrap().tiles[5];
    assert_eq!(i64::from(shown.orientation), to, "the area shows it turned");
    assert_eq!(undo_label(&h).as_deref(), Some("Turn tile (ignoring fit)"));
    let p = h.state().tile_props.clone().expect("the window stays");
    assert!(p.all_tiles && i64::from(p.turn) == to, "the pictures turned so");
    assert!(h.query_all_by_label_contains(&format!(" {name}")).next().is_some(), "shown so");
    shots_beside(&mut h, area, glam::Vec3::new(15.0, 15.0, 0.0), &shots.join("turn-after"));
    h.state_mut().ws.as_mut().unwrap().undo().unwrap();
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), list, "one step");
    assert_eq!(undo_label(&h), before);
}

/// Refine Tile dragged: the first tile steps to the next that fits, and
/// every tile of its kind passed becomes that tile too; with a tile chosen
/// in the Gallery first, those it fits become it.
#[test]
fn refine_tile_dragged_makes_tiles_of_a_kind_alike() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("refine-drag") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    let drag = |h: &mut Harness<'_, Moonglow>, y: f32| {
        let from = screen(h, area, Vec3::new(5.0, y, 0.0));
        let to = screen(h, area, Vec3::new(35.0, y, 0.0));
        h.hover_at(from);
        h.run_steps(2);
        press(h, from, true, egui::Modifiers::NONE);
        for k in 1..=8 {
            h.hover_at(from + (to - from) * (k as f32 / 8.0));
            h.run_steps(1);
        }
        press(h, to, false, egui::Modifiers::NONE);
        h.run_steps(3);
    };
    h.state_mut().settings.palette_gallery = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(3);
    h.get_by_label("🔁 Refine Tile").click();
    h.run_steps(2);
    let before = grid(&mut h);
    let next = mg_tiles::paint::next_fit(&index, &before.lattice.cell(0, 2), before.tile(0, 2))
        .expect("Rural's grass has other tiles");
    drag(&mut h, 25.0);
    let after = grid(&mut h);
    assert_eq!(after.tile(0, 2), next, "the first: its next");
    for x in 1..4 {
        assert_eq!(after.tile(x, 2).tile, next.tile, "({x}, 2): the first's tile");
    }
    assert_eq!(after.lattice, before.lattice, "no terrain painted");
    assert_eq!(after.tile(0, 1), before.tile(0, 1), "a row not passed is as it was");
    // A tile chosen in the Gallery, then Refine Tile: the tiles passed
    // become it.
    let grass = index.terrain("Grass").unwrap();
    let all_grass = index.tiles_where(|c| {
        c.corners.iter().all(|k| k.terrain == grass && k.height == 0) && c.edges == [None; 4]
    });
    let tile =
        *all_grass.iter().find(|t| **t != next.tile && **t != before.tile(0, 0).tile).unwrap();
    let picture = index_model(&h, tile);
    open_tile_heading(&mut h, &picture);
    h.get_all_by_label(&picture).next().unwrap().scroll_to_me();
    h.run_steps(3);
    h.get_all_by_label(&picture).next().unwrap().click();
    h.run_steps(2);
    // (The search cleared: it found the tile alone.)
    h.state_mut().palette.filter.clear();
    h.run_steps(3);
    h.get_by_label_contains("Refine Tile →").scroll_to_me();
    h.run_steps(3);
    h.get_by_label_contains("Refine Tile →").click();
    h.run_steps(2);
    assert_eq!(h.state().palette.tile_brush.as_ref().unwrap().target, Some(tile));
    drag(&mut h, 5.0);
    let row = grid(&mut h);
    for x in 0..4 {
        assert_eq!(row.tile(x, 0).tile, tile, "({x}, 0): the tile chosen");
    }
}

/// The tiles palette as the blueprint palettes: its search finds
/// brushes and tiles, its headings open and close; a tile's picture double
/// clicked opens its view, and one dragged into the area is placed where it
/// is dropped.
#[test]
fn the_tile_gallery_searches_opens_a_tile_and_drops_one_in_the_area() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("tile-gallery-more") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    h.state_mut().settings.palette_gallery = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(3);
    // Headings closed at first: no picture shows.
    let grass = index.terrain("Grass").unwrap();
    let flat = index.tiles_where(|c| {
        c.corners.iter().all(|k| k.terrain == grass && k.height == 0) && c.edges == [None; 4]
    });
    let before = grid(&mut h);
    let tile = *flat.iter().find(|t| **t != before.tile(2, 2).tile).unwrap();
    let model = index_model(&h, tile);
    assert!(h.query_by_label(&model).is_none(), "under a closed heading");
    // The search: by a tile's model, its heading opened with it.
    open_tile_heading(&mut h, &model);
    h.get_by_label(&model);
    assert!(h.query_by_label("Water").is_none(), "a heading without it is not shown");
    // A double click: its view.
    let at = h.get_by_label(&model).rect().center();
    h.run_steps(30);
    let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
    for pressed in [true, false, true, false] {
        h.event(egui::Event::PointerButton { pos: at, button, pressed, modifiers });
    }
    h.run_steps(3);
    let tileset = ResRef::from_str("ttr01").unwrap();
    let tab = h.state().dock.find_tab(&Tab::Tile { tileset, tile }).expect("its view");
    h.get_by_label("Corners");
    h.state_mut().dock.remove_tab(tab);
    h.run_steps(3);
    // Dragged into the area: placed in the tile it is dropped on.
    let from = h.get_by_label(&model).rect().center();
    let to = screen(&h, area, Vec3::new(25.0, 25.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    for k in 1..=10 {
        h.hover_at(from + (to - from) * (k as f32 / 10.0));
        h.run_steps(1);
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(grid(&mut h).tile(2, 2).tile, tile, "placed where it was dropped");
}

/// In the tiles palette's Gallery, a tileset's features and groups are
/// pictures of their tiles together: a click chooses one, and one dragged
/// into the area is placed where it is dropped.
#[test]
fn the_tile_gallery_s_groups_are_pictures_to_choose_and_drop() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("tile-gallery-groups") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    h.state_mut().settings.palette_gallery = true;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    open_tile_heading(&mut h, "well");
    // Rural's Well: a group of one tile (117), its picture named.
    let well = h.get_all_by_label_contains("Well").next().expect("its picture").rect().center();
    h.get_all_by_label_contains("Well").next().unwrap().click();
    h.run_steps(2);
    let brush = h.state().palette.tile_brush.clone().expect("chosen");
    assert!(matches!(brush.brush, mg_area::terrain::Brush::Group(_)), "{brush:?}");
    h.state_mut().palette.tile_brush = None;
    h.run_steps(2);
    // Dragged into the area: placed where it is dropped.
    let to = screen(&h, area, Vec3::new(25.0, 15.0, 0.0));
    h.hover_at(well);
    press(&h, well, true, egui::Modifiers::NONE);
    for k in 1..=10 {
        h.hover_at(well + (to - well) * (k as f32 / 10.0));
        h.run_steps(1);
    }
    press(&h, to, false, egui::Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(grid(&mut h).tile(2, 1).tile, 117, "the Well where it was dropped");
}

/// The tiles palette's search finds a group by the model of one of its
/// tiles that no terrain or crosser makes (#35: Medieval City's Inn_2x3,
/// "Inn 1 (2x3)", whose `tcm02_s01_05` is in no other brush): in the list,
/// and in the Gallery as the group's picture, not as a heading with its
/// tiles.
#[test]
fn the_tile_search_finds_a_group_by_its_tiles_models() {
    const INN: &str = "Inn 1 (2x3)";
    let Some((mut h, _)) = area_harness_on("tile-group-search", "tcm02", None) else { return };
    // (The count is the row's text: a label's value.)
    let one_item = |h: &Harness<'_, Moonglow>, when: &str| {
        assert!(h.query_all_by_value("1 item").next().is_some(), "{when}: the count, 1 item");
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(3);
    let model = "tcm02_s01_05";
    h.state_mut().palette.filter = model.into();
    h.run_steps(3);
    // The list: the group's row, alone.
    h.get_by_label(INN);
    one_item(&h, "the list");
    // The Gallery: the group's picture, alone; no picture of the tile
    // under a heading.
    h.state_mut().settings.palette_gallery = true;
    h.run_steps(5);
    assert_eq!(h.get_all_by_label_contains(INN).count(), 1, "the group's picture");
    one_item(&h, "the Gallery");
    assert_eq!(h.query_all_by_label_contains(model).count(), 0, "no heading's tile picture");
    // Its name finds it as before.
    h.state_mut().palette.filter = INN.to_lowercase();
    h.run_steps(3);
    assert_eq!(h.get_all_by_label_contains(INN).count(), 1);
    one_item(&h, "by its name");
}

#[test]
fn a_terrain_drag_with_shift_fills_its_rectangle() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("terrain-fill") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let water = index.terrain("Water").unwrap();
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("Water").click();
    h.run_steps(2);
    let before = lattice(&mut h);
    // Corner (1, 1) diagonally to (3, 2), Shift held.
    let from = screen(&h, area, Vec3::new(10.0, 10.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    for t in [0.2, 0.4, 0.6, 0.8, 1.0] {
        let p = Vec3::new(10.0, 10.0, 0.0).lerp(Vec3::new(30.0, 20.0, 0.0), t);
        h.hover_at(screen(&h, area, p));
        h.run_steps(1);
    }
    let to = screen(&h, area, Vec3::new(30.0, 20.0, 0.0));
    press(&h, to, false, egui::Modifiers::SHIFT);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let after = lattice(&mut h);
    for y in 1..=2 {
        for x in 1..=3 {
            assert_eq!(after.corner(x, y).terrain, water, "corner ({x}, {y})");
        }
    }
    for (x, y) in [(0, 1), (4, 2), (2, 0), (2, 3)] {
        assert_eq!(after.corner(x, y).terrain, before.corner(x, y).terrain, "({x}, {y})");
    }
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(lattice(&mut h), before);
}

#[test]
fn a_crosser_drag_with_shift_follows_its_rectangle_s_outline() {
    use glam::Vec3;
    use mg_tiles::{EAST, NORTH, SOUTH, TileIndex, WEST};
    let Some((mut h, area)) = area_harness("crosser-outline") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let road = index.crosser("Road").unwrap();
    let lattice = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap().lattice
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("Road").click();
    h.run_steps(2);
    let before = lattice(&mut h);
    // Tile (0, 0) diagonally to tile (2, 2), Shift held.
    let from = screen(&h, area, Vec3::new(5.0, 5.0, 0.0));
    h.hover_at(from);
    press(&h, from, true, egui::Modifiers::NONE);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    for t in [0.25, 0.5, 0.75, 1.0] {
        let p = Vec3::new(5.0, 5.0, 0.0).lerp(Vec3::new(25.0, 25.0, 0.0), t);
        h.hover_at(screen(&h, area, p));
        h.run_steps(1);
    }
    let to = screen(&h, area, Vec3::new(25.0, 25.0, 0.0));
    press(&h, to, false, egui::Modifiers::SHIFT);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(3);
    let l = lattice(&mut h);
    let edges = |x, y| l.cell(x, y).edges;
    let on = |list: &[usize]| {
        let mut e = [None; 4];
        for &i in list {
            e[i] = Some(road);
        }
        e
    };
    assert_eq!(edges(0, 0), on(&[EAST, NORTH]), "{:?}", h.state().log.entries);
    assert_eq!(edges(1, 0), on(&[EAST, WEST]));
    assert_eq!(edges(2, 0), on(&[WEST, NORTH]));
    assert_eq!(edges(2, 1), on(&[SOUTH, NORTH]));
    assert_eq!(edges(2, 2), on(&[SOUTH, WEST]));
    assert_eq!(edges(1, 2), on(&[EAST, WEST]));
    assert_eq!(edges(0, 2), on(&[EAST, SOUTH]));
    assert_eq!(edges(0, 1), on(&[NORTH, SOUTH]));
    assert_eq!(edges(1, 1), [None; 4], "nothing inside");
    assert_eq!(edges(3, 3), before.cell(3, 3).edges);
}

#[test]
fn painting_with_shift_held_the_camera_still_zooms_and_turns() {
    let Some((mut h, area)) = area_harness("shift-camera") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("Road").click();
    h.run_steps(2);
    let orbit = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit.unwrap();
    let at = screen(&h, area, glam::Vec3::new(20.0, 10.0, 0.0));
    h.hover_at(at);
    h.run_steps(1);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    h.run_steps(1);
    // The wheel (which egui turns sideways with Shift) zooms.
    let before = orbit(&h);
    for _ in 0..5 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 60.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::SHIFT,
        });
        h.run_steps(1);
    }
    h.run_steps(10);
    let zoomed = orbit(&h);
    assert!(zoomed.distance < before.distance - 0.5, "{} → {}", before.distance, zoomed.distance);
    // A right drag turns it.
    let button = egui::PointerButton::Secondary;
    let modifiers = egui::Modifiers::SHIFT;
    h.event(egui::Event::PointerButton { pos: at, button, pressed: true, modifiers });
    h.run_steps(1);
    for dx in [20.0, 40.0, 60.0] {
        h.event(egui::Event::PointerMoved(at + egui::vec2(dx, 0.0)));
        h.run_steps(1);
    }
    let pos = at + egui::vec2(60.0, 0.0);
    h.event(egui::Event::PointerButton { pos, button, pressed: false, modifiers });
    h.run_steps(1);
    let turned = orbit(&h);
    assert!((turned.yaw - zoomed.yaw).abs() > 0.3, "turned: {} → {}", zoomed.yaw, turned.yaw);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(1);
}

#[test]
fn a_tile_brush_previews_the_tiles_its_click_makes() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("tile-preview") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    // What the preview shows is what the click puts down.
    let check = |h: &mut Harness<'_, Moonglow>, at: Vec3| {
        let pos = screen(h, area, at);
        h.hover_at(pos);
        h.run_steps(3);
        let shown = h.state().area_views[&area].tile_preview.clone();
        assert!(!shown.is_empty(), "a preview at {at}");
        for pressed in [true, false] {
            let (button, modifiers) = (egui::PointerButton::Primary, egui::Modifiers::NONE);
            h.event(egui::Event::PointerButton { pos, button, pressed, modifiers });
        }
        h.run_steps(3);
        let g = grid(h);
        for t in &shown {
            let placed = g.tile(t.column, t.row);
            assert_eq!(
                (i64::from(placed.tile), placed.orientation, placed.height),
                (t.id, t.orientation, t.height),
                "tile ({}, {})",
                t.column,
                t.row
            );
        }
    };
    // Water on a corner (its four tiles, chosen at random among those that
    // fit), and a barn.
    h.get_by_label("Water").click();
    h.run_steps(2);
    check(&mut h, Vec3::new(20.0, 20.0, 0.0));
    h.get_by_label("Groups").click();
    h.run_steps(2);
    h.get_by_label("Barn 1 2x2").click();
    h.run_steps(2);
    check(&mut h, Vec3::new(5.0, 5.0, 0.0));
}

#[test]
fn a_drag_previews_the_tiles_letting_go_paints() {
    use glam::Vec3;
    use mg_tiles::TileIndex;
    let Some((mut h, area)) = area_harness("drag-preview") else { return };
    let index = {
        let game = h.state().game.as_deref().unwrap();
        TileIndex::new(&mg_area::tileset(game, ResRef::from_str("ttr01").unwrap()).unwrap())
    };
    let grid = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let are = ws.doc(&ResKey::new(area, ResType::ARE)).unwrap();
        mg_area::terrain::grid(&are.root, &index).unwrap()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    // Dragged along `path` (metres): before it is let go, the preview; let
    // go, those very tiles.
    let check = |h: &mut Harness<'_, Moonglow>, path: &[Vec3]| {
        let before = grid(h);
        let from = screen(h, area, path[0]);
        h.hover_at(from);
        press(h, from, true, egui::Modifiers::NONE);
        h.run_steps(1);
        for p in &path[1..] {
            h.hover_at(screen(h, area, *p));
            h.run_steps(1);
        }
        h.run_steps(2);
        let shown = h.state().area_views[&area].tile_preview.clone();
        assert!(!shown.is_empty(), "a preview while dragging");
        assert_eq!(grid(h).tiles, before.tiles, "nothing painted yet");
        let to = screen(h, area, *path.last().unwrap());
        press(h, to, false, egui::Modifiers::NONE);
        h.run_steps(3);
        let g = grid(h);
        for t in &shown {
            let placed = g.tile(t.column, t.row);
            assert_eq!(
                (i64::from(placed.tile), placed.orientation, placed.height),
                (t.id, t.orientation, t.height),
                "tile ({}, {})",
                t.column,
                t.row
            );
        }
    };
    h.get_by_label("Water").click();
    h.run_steps(2);
    check(&mut h, &[10.0, 15.0, 20.0, 25.0, 30.0].map(|x| Vec3::new(x, 20.0, 0.0)));
    h.get_by_label("Road").click();
    h.run_steps(2);
    check(&mut h, &[7.0, 12.0, 18.0, 24.0, 25.0].map(|x| Vec3::new(x, 5.0, 0.0)));
}

/// With a terrain brush in hand, a right click is the brush's (Raise/
/// Lower lowers) though an object lies under the pointer: its menu does
/// not open.
#[test]
fn a_right_click_with_a_terrain_brush_is_the_brush_s_over_an_object() {
    use glam::Vec3;
    let Some((mut h, area)) = area_harness("brush-over-object") else { return };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.run_steps(3);
    palette_type(&mut h, "🗻", "Tiles");
    h.run_steps(2);
    h.get_by_label("↕ Raise/Lower").click();
    h.run_steps(2);
    // (On the waypoint at 20, 20.)
    let at = screen(&h, area, Vec3::new(20.0, 20.0, 0.02));
    h.hover_at(at);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    assert!(h.query_by_label("Properties").is_none(), "no object menu");
    assert!(h.query_by_label("Go To").is_none());
    assert!(h.state().palette.tile_brush.is_some(), "the brush stays in hand");
    // A right click where no ground of the area is under the pointer (off
    // its edge; a cliff face with nothing to stand on, the ray going on
    // past the area): Raise/Lower lowers nothing there, and stays in hand.
    // It was dropped without a word, and the next click was the object's.
    let off = h.state().area_views[&area].rect.left_top() + egui::vec2(12.0, 40.0);
    assert!(h.state().area_views[&area].ground_spot(off).is_none(), "off the area");
    h.hover_at(off);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: off,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    assert!(h.state().palette.tile_brush.is_some(), "Raise/Lower stays in hand");
    h.hover_at(at);
    h.run_steps(2);
    // Fast and sloppy, as when working along a slope: presses and
    // releases a few points apart, left and right, single and double,
    // some begun before the last stroke is drawn. The object under them
    // is never selected and its menu never opens.
    let sel = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].selection.clone();
    assert!(sel(&h).is_empty());
    for round in 0..24 {
        let button = if round % 2 == 0 {
            egui::PointerButton::Primary
        } else {
            egui::PointerButton::Secondary
        };
        let slip = egui::vec2((round % 5) as f32 * 2.0, (round % 3) as f32 * 2.0);
        let modifiers = egui::Modifiers::NONE;
        h.event(egui::Event::PointerButton { pos: at, button, pressed: true, modifiers });
        if round % 3 != 0 {
            h.run_steps(1);
        }
        h.event(egui::Event::PointerMoved(at + slip));
        h.event(egui::Event::PointerButton { pos: at + slip, button, pressed: false, modifiers });
        h.run_steps(if round % 4 == 0 { 1 } else { 2 });
        assert!(sel(&h).is_empty(), "round {round}: selected {:?}", sel(&h));
        assert!(h.query_by_label("Go To").is_none(), "round {round}: the object's menu");
        assert!(h.query_by_label("Properties").is_none(), "round {round}: the object's menu");
        assert!(h.state().palette.tile_brush.is_some(), "round {round}: the brush dropped");
    }
}

/// A tile's source light burns: its flame (`fx_flame01`) is emitters
/// and a light, and its particles are drawn with the tiles' own. (A
/// torch lit the wall and showed no flame.)
#[test]
fn a_tile_s_source_light_burns() {
    let Some((mut h, area)) = area_harness_on("source-lights", "tic01", Some(0)) else { return };
    let key = ResKey::new(area, ResType::ARE);
    h.state_mut().area_views.get_mut(&area).unwrap().orbit = Some(mg_ui::area_view::Orbit {
        target: glam::Vec3::new(15.0, 15.0, 1.5),
        yaw: std::f32::consts::PI,
        pitch: 0.35,
        distance: 14.0,
    });
    h.run_steps(10);
    let unlit = h.state().area_views[&area].particles_shown;
    let edits = ["Tile_SrcLight1", "Tile_SrcLight2"]
        .map(|label| mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root().item("Tile_List", 5),
            label: label.into(),
            value: Some(mg_gff::Value::Byte(1)),
        })
        .to_vec();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Lit", edits)));
    h.run_steps(10);
    let lit = h.state().area_views[&area].particles_shown;
    assert!(lit > unlit, "the flames' particles: {unlit} → {lit}");
}

/// Between objects and tiles without the toolbar. F10 with the pointer
/// over an area's view is Select Tiles or Objects, as in Aurora; the menus
/// don't take it there (they do everywhere else, and Alt opens them
/// anywhere). A double click on the ground selects its tile (Select Tiles
/// comes on), and a click on an object there selects it (and Select Tiles
/// goes off). The view's readout names the tile under the pointer: its
/// model, as Aurora's status bar does.
#[test]
fn f10_and_a_click_go_between_objects_and_tiles() {
    use glam::Vec3;
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("tiles-and-objects") else { return };
    let view = &h.state().area_views[&area];
    let named = view.tile_named(1, 2).expect("a tile there");
    assert!(named.starts_with("ttr01_") && named.contains(" at 1, 2"), "{named}");
    assert_eq!(view.tile_named(40, 40), None, "outside the area");
    h.run_steps(40);

    // F10.
    let shown = |h: &Harness<'_, Moonglow>, label: &str| {
        h.query_all_by_label_contains(label).next().is_some()
    };
    h.hover_at(screen(&h, area, Vec3::new(5.0, 5.0, 0.0)));
    h.run_steps(3);
    h.key_press(egui::Key::F10);
    h.run_steps(3);
    assert!(h.state().area_views[&area].tile_mode, "over to the tiles");
    assert!(!shown(&h, "Save As"), "and no menu opened");
    h.key_press(egui::Key::F10);
    h.run_steps(3);
    assert!(!h.state().area_views[&area].tile_mode, "back to the objects");
    // Off the view (over the menu bar), F10 is the menus'.
    h.hover_at(egui::pos2(300.0, 4.0));
    h.run_steps(3);
    h.key_press(egui::Key::F10);
    h.run_steps(3);
    assert!(shown(&h, "Save As"));
    assert!(!h.state().area_views[&area].tile_mode);
    // And with a menu open it closes them, wherever the pointer is.
    h.hover_at(screen(&h, area, Vec3::new(5.0, 5.0, 0.0)));
    h.run_steps(3);
    h.key_press(egui::Key::F10);
    h.run_steps(3);
    assert!(!shown(&h, "Save As"));
    assert!(!h.state().area_views[&area].tile_mode);

    // A click.
    let ground = screen(&h, area, Vec3::new(5.0, 5.0, 0.0));
    h.hover_at(ground);
    for _ in 0..2 {
        press(&h, ground, true, egui::Modifiers::NONE);
        press(&h, ground, false, egui::Modifiers::NONE);
    }
    h.run_steps(3);
    let view = &h.state().area_views[&area];
    assert!(view.tile_mode, "over to the tiles");
    assert_eq!(view.tile_selection, [(0, 0)]);
    // (Later: not a third click of the same.)
    h.run_steps(60);
    let flag = screen(&h, area, Vec3::new(20.0, 20.0, 0.9));
    h.hover_at(flag);
    press(&h, flag, true, egui::Modifiers::NONE);
    press(&h, flag, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let view = &h.state().area_views[&area];
    assert!(!view.tile_mode, "back to the objects");
    assert_eq!(view.selection, [(ObjectKind::Waypoint, 0)]);
    assert!(view.tile_selection.is_empty());
}

/// A builder's report: a big old tileset, merged by hand from many, drew
/// no tiles at all, though the game and Aurora draw them. A tileset
/// whose file has holes (a `[TILEn]` section missing, a tile without
/// `Model=`, a terrain's or a rule's section missing) failed whole; the
/// game reads it tile by tile (`engine_tileset_holes`): a tile without
/// its section or model is drawn as nothing, the others as ever, and the
/// area's problems name the holes. The tileset is ttr01's file with holes
/// cut in it, in a user folder's override.
#[test]
fn an_area_of_a_tileset_with_holes_draws_its_other_tiles() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let user = mg_testkit::scratch_dir("ui-tileset-holes-user");
    let game = mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
    let ttr01 = game.resman.get(&ResKey::parse("ttr01", ResType::SET).unwrap()).unwrap();
    let ttr01 = String::from_utf8_lossy(&ttr01).into_owned();
    drop(game);
    let cut = |set: &str, name: &str| {
        let header = format!("[{name}]\r\n");
        let at = set.find(&header).unwrap();
        let rest = at + header.len();
        let end = set[rest..].find("\r\n[").map_or(set.len(), |e| rest + e + 2);
        format!("{}{}", &set[..at], &set[end..])
    };
    let mut holed = cut(&ttr01, "TILE20");
    holed = holed.replacen("Model=ttr01_k01_01\r\n", "", 1);
    holed = holed.replacen("Model=ttr01_k02_01\r\n", "Model=TTR01_K02_01\r\n", 1);
    for s in ["TERRAIN1", "CROSSER0", "PRIMARY RULE0"] {
        holed = cut(&holed, s);
    }
    // And tile 103's model one that is there and can't be read.
    holed = holed.replacen("Model=ttr01_k03_01\r\n", "Model=mg_unreadable\r\n", 1);
    std::fs::create_dir_all(user.join("override")).unwrap();
    std::fs::write(user.join("override/mghole.set"), holed).unwrap();
    std::fs::write(user.join("override/mg_unreadable.mdl"), b"no model at all").unwrap();
    // Tiles 20 (no section), 101 (no Model=), 102 (its model in capitals)
    // and 103 among ttr01's grass.
    let holes = [(5, 20), (6, 101), (9, 102), (10, 103)];
    let prepare = |m: &mut mg_module::Module, _: &mg_rules::GameData, area: ResRef| {
        let key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&key).unwrap().unwrap();
        are.root.set("Tileset", mg_gff::Value::ResRef(b"mghole".to_vec()));
        let tiles = are.root.list_mut("Tile_List").unwrap();
        for (at, id) in holes {
            tiles[at].set("Tile_ID", mg_gff::Value::Int(id));
            tiles[at].set("Tile_Height", mg_gff::Value::Int(0));
        }
        m.set_gff(key, &are).unwrap();
    };
    let Some((mut h, area)) =
        area_harness_with("tileset-holes", "ttr01", None, 4, Some(user), &prepare)
    else {
        return;
    };
    h.run_steps(10);
    let dir = mg_testkit::output_dir("tileset-holes");
    h.render().expect("render").save(dir.join("area.png")).unwrap();
    let view = &h.state().area_views[&area];
    let model = view.model.as_ref().unwrap();
    let models: Vec<Option<&str>> = model.tiles.iter().map(|t| t.model.as_deref()).collect();
    assert_eq!((models[5], models[6]), (None, None), "the holes have no model: {models:?}");
    assert_eq!(models[9], Some("ttr01_k02_01"), "a model in capitals is the model");
    let drawn = models.iter().filter(|m| m.is_some()).count();
    assert_eq!(drawn, 14, "the other tiles keep their models: {models:?}");
    // The log says why: the holes, and a model that can't be read is not
    // one "not found".
    let log: Vec<&str> = h.state().log.entries.iter().map(|e| e.1.as_str()).collect();
    let log = log.join("\n");
    assert!(!log.contains("not found"), "the tileset and the model are there:\n{log}");
    assert!(log.contains("no model for tile 20, tile 101"), "{log}");
    assert!(log.contains("model mg_unreadable (in "), "{log}");
    assert!(log.contains("can't be read"), "{log}");
}
