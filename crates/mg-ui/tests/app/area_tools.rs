//! The area view's tools (M4's U4; the Main and AreaPaint boards): the
//! tool shelf at the view's left, one tool on at a time whichever way it
//! is chosen; Terrain with the tiles palette's brushes; the compass and
//! its buttons at the view's top right; the minimap at its bottom right;
//! the palette's Brush section.

use crate::common::*;
use egui::accesskit::Toggled;

/// The shelf's tools, top down.
const SHELF: [&str; 4] = ["Select Objects", "Select Tiles", "Paint", "Terrain"];

/// The shelf's tools that are on, by their names.
fn on(h: &Harness<'_, Moonglow>) -> Vec<&'static str> {
    let shelf = h.get_by_label(AREA_TOOLS);
    let toggled = |name: &str| shelf.get_by_label(name).accesskit_node().toggled();
    SHELF.into_iter().filter(|n| toggled(n) == Some(Toggled::True)).collect()
}

/// Clicks the shelf's tool `name`.
fn shelf_click(h: &mut Harness<'_, Moonglow>, name: &str) {
    h.get_by_label(AREA_TOOLS).get_by_label(name).click();
    h.run_steps(3);
}

/// A tile brush of the harness's tileset (`ttr01`), as the tiles palette
/// gives it.
fn raise_lower() -> mg_ui::terrain_mode::TileBrush {
    mg_ui::terrain_mode::TileBrush {
        tileset: ResRef::from_str("ttr01").unwrap(),
        label: "Raise/Lower".into(),
        brush: mg_area::terrain::Brush::RaiseLower,
        target: None,
        exact: false,
    }
}

/// One tool on at a time, by the shelf, by the keys and by a tile brush
/// chosen: Select Objects at first; Select Tiles, Paint and Terrain each
/// putting the others down; a tool clicked again back to Select Objects.
/// Docked (both modes of Open editors) and in a floating window of its
/// own (the area's tab dragged out).
#[test]
fn one_tool_is_on_at_a_time() {
    for (mode, floats) in [(Mode::Floating, false), (Mode::Tabs, false), (Mode::Floating, true)] {
        let Some((mut h, area)) = area_harness(&mode.dir(&format!("tools-one-{floats}"))) else {
            return;
        };
        h.state_mut().settings.open_editors = mode.settings().open_editors;
        if floats {
            float_area(&mut h, area);
        }
        settle(&mut h);
        assert_eq!(on(&h), ["Select Objects"], "{mode:?}: at first");
        shelf_click(&mut h, "Select Tiles");
        assert_eq!(on(&h), ["Select Tiles"], "{mode:?}");
        assert!(h.state().area_views[&area].tile_mode);
        shelf_click(&mut h, "Paint");
        assert_eq!(on(&h), ["Paint"], "{mode:?}");
        assert!(h.state().paint && !h.state().area_views[&area].tile_mode, "{mode:?}");
        // Select Tiles puts Paint down.
        shelf_click(&mut h, "Select Tiles");
        assert_eq!(on(&h), ["Select Tiles"], "{mode:?}: Paint put down");
        assert!(!h.state().paint, "{mode:?}: Paint put down");
        shelf_click(&mut h, "Paint");
        // A tile brush chosen (the tiles palette's): Terrain, Paint put
        // down.
        shelf_click(&mut h, "Terrain");
        assert_eq!(on(&h), ["Select Objects"], "{mode:?}: no brush in hand yet");
        assert!(!h.state().paint, "{mode:?}: Paint put down");
        h.state_mut().palette.tile_brush = Some(raise_lower());
        h.state_mut().paint = false;
        h.run_steps(3);
        assert_eq!(on(&h), ["Terrain"], "{mode:?}: a brush in hand");
        shelf_click(&mut h, "Paint");
        assert_eq!(on(&h), ["Paint"], "{mode:?}");
        assert!(h.state().palette.tile_brush.is_none(), "{mode:?}: the brush put down");
        // Clicked again: Select Objects.
        shelf_click(&mut h, "Paint");
        assert_eq!(on(&h), ["Select Objects"], "{mode:?}");
        assert!(!h.state().paint);
        // Select Tiles, then Select Objects.
        shelf_click(&mut h, "Select Tiles");
        shelf_click(&mut h, "Select Objects");
        assert_eq!(on(&h), ["Select Objects"], "{mode:?}");
        assert!(!h.state().area_views[&area].tile_mode, "{mode:?}");
    }
}

/// F10 and B, with the pointer over the view, turn the shelf's Select
/// Tiles and Paint on and off, each putting the other down (the tips
/// name the keys).
#[test]
fn f10_and_b_turn_the_shelf_s_buttons() {
    let Some((mut h, area)) = area_harness("tools-keys") else { return };
    settle(&mut h);
    let middle = h.state().area_views[&area].rect.center();
    h.hover_at(middle);
    h.run_steps(2);
    h.key_press(egui::Key::B);
    h.run_steps(3);
    assert_eq!(on(&h), ["Paint"], "B");
    h.key_press(egui::Key::F10);
    h.run_steps(3);
    assert_eq!(on(&h), ["Select Tiles"], "F10: Paint put down");
    assert!(!h.state().paint);
    h.key_press(egui::Key::F10);
    h.run_steps(3);
    assert_eq!(on(&h), ["Select Objects"], "F10 again");
    h.key_press(egui::Key::B);
    h.run_steps(3);
    h.key_press(egui::Key::B);
    h.run_steps(3);
    assert_eq!(on(&h), ["Select Objects"], "B twice");
    // The tips name the keys.
    for (name, key) in [("Select Tiles", "(F10)"), ("Paint", "(B)")] {
        let rect = h.get_by_label(AREA_TOOLS).get_by_label(name).rect();
        let tip = tip_at(&mut h, rect).unwrap_or_default();
        assert!(tip.ends_with(key), "{name}'s tip: {tip:?}");
    }
}

/// The shelf's tools and the compass's buttons are reached by Tab, one
/// after the other, each named.
#[test]
fn the_shelf_and_the_compass_are_reached_by_tab() {
    let Some((mut h, _)) = area_harness("tools-tab") else { return };
    settle(&mut h);
    h.get_by_label(AREA_TOOLS).get_by_label("Select Objects").focus();
    h.run_steps(2);
    let focused = |h: &Harness<'_, Moonglow>| {
        h.query_all_by(|n| n.is_focused()).next().and_then(|n| n.accesskit_node().label())
    };
    assert_eq!(focused(&h).as_deref(), Some("Select Objects"));
    for next in ["Select Tiles", "Paint", "Terrain"] {
        h.key_press(egui::Key::Tab);
        h.run_steps(2);
        assert_eq!(focused(&h).as_deref(), Some(next), "Tab to {next}");
    }
    h.get_by_label("Compass").get_by_label("Reorient Camera").focus();
    h.run_steps(2);
    h.key_press(egui::Key::Tab);
    h.run_steps(2);
    assert_eq!(focused(&h).as_deref(), Some("Go to Start Location"));
    // Enter on it does what a click does: the camera off the start, back.
    let start = start_of(&mut h);
    let area = *h.state().area_views.keys().next().expect("the area");
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().target =
        glam::Vec3::new(start.x + 9.0, start.y + 9.0, 0.0);
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    let after = h.state().area_views[&area].orbit.unwrap().target;
    assert!((after.truncate() - start).length() < 0.5, "Enter: at the start {start}: {after}");
}

/// Where the module's start location is (its area is the harness's).
fn start_of(h: &mut Harness<'_, Moonglow>) -> glam::Vec2 {
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let r = ws.doc(&ifo).unwrap().root.clone();
    glam::Vec2::new(r.float("Mod_Entry_X").unwrap(), r.float("Mod_Entry_Y").unwrap())
}

/// The area's tab dragged out of the editor area into a window of its
/// own (as egui_dock lets a builder), wide enough for the header.
fn float_area(h: &mut Harness<'_, Moonglow>, area: ResRef) {
    let size = h.ctx.content_rect().size();
    own_window(h, &Tab::Area(area), egui::pos2(40.0, 40.0), size - egui::vec2(80.0, 80.0));
    settle(h);
    let in_window = h.state().area_views[&area].rect;
    assert!(in_window.width() > 100.0, "the area drawn in its window: {in_window:?}");
}

/// Terrain (Part 2, decision 8): a click shows the Palette on its Tiles
/// type; it is on while one of the tiles palette's brushes is in hand
/// (choosing one turns it on, Paint and Select Tiles put down); clicked
/// on, the brush is put down.
#[test]
fn terrain_is_on_with_a_tile_brush_in_hand() {
    let Some((mut h, area)) = area_harness("tools-terrain") else { return };
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
    }
    fold_palette(h.state_mut());
    settle(&mut h);
    shelf_click(&mut h, "Terrain");
    settle(&mut h);
    assert!(h.state().palette.tiles, "the tiles palette");
    assert!(palette_shown(h.state()), "in the Right Dock");
    // Paint on, then a brush chosen: Terrain on, Paint off.
    shelf_click(&mut h, "Paint");
    h.state_mut().palette.tile_brush = Some(raise_lower());
    h.state_mut().paint = false;
    h.run_steps(3);
    assert_eq!(on(&h), ["Terrain"]);
    assert!(!h.state().area_views[&area].tile_mode);
    // Paint taken from the shelf puts the brush down; the brush again.
    shelf_click(&mut h, "Paint");
    assert!(h.state().palette.tile_brush.is_none(), "Paint puts the brush down");
    assert_eq!(on(&h), ["Paint"]);
    h.state_mut().palette.tile_brush = Some(raise_lower());
    h.state_mut().paint = false;
    h.run_steps(3);
    shelf_click(&mut h, "Terrain");
    assert!(h.state().palette.tile_brush.is_none(), "put down");
    assert_eq!(on(&h), ["Select Objects"]);
    // Paint on, then a brush chosen in the tiles palette itself: Paint
    // put down, Terrain lit.
    shelf_click(&mut h, "Paint");
    assert_eq!(on(&h), ["Paint"]);
    h.state_mut().palette.tiles = true;
    settle(&mut h);
    palette_row(&h, "Raise/Lower").click();
    h.run_steps(3);
    assert!(!h.state().paint, "Paint put down by the brush");
    assert_eq!(on(&h), ["Terrain"], "the brush chosen in the palette");
}

/// The palette's Brush section while Paint is on: two chosen, a row each;
/// × on one chosen with Ctrl + click takes it out, × on the one clicked
/// takes it out too (none left: the section says how to choose them).
#[test]
fn x_takes_a_blueprint_out_of_the_brush() {
    let Some((mut h, _)) = area_ui_harness("tools-brush-x") else { return };
    let statue = ResKey::parse("plc_statue1", ResType::UTP).unwrap();
    let tree = ResKey::parse("plc_tree", ResType::UTP).unwrap();
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.selected = Some(statue);
        app.palette.chosen = vec![tree];
        app.paint = true;
    }
    show_palette(h.state_mut());
    settle(&mut h);
    h.get_by_label("Brush blueprints").get_by_label("2 chosen");
    let x = |key: ResKey| {
        format!("Take {} out of the brush (Ctrl+click it in the palette)", key.resref)
    };
    h.get_by_label(&x(tree)).click();
    h.run_steps(3);
    assert_eq!(h.state().palette.chosen, []);
    assert_eq!(h.state().palette.selected, Some(statue), "the one clicked stays");
    h.get_by_label("Brush blueprints").get_by_label("1 chosen");
    h.get_by_label(&x(statue)).click();
    h.run_steps(3);
    assert_eq!(h.state().palette.selected, None);
    h.get_by_label("Brush blueprints").get_by_label("none chosen");
    h.get_by_label_contains("Choose placeables in the palette");
}

/// The compass's buttons: Reorient Camera turns the camera to face north,
/// Go to Start Location looks at the start location; over them nothing
/// follows the pointer and a click places nothing, with Paint on and a
/// placeable chosen or a blueprint in hand.
#[test]
fn the_compass_s_buttons_run_their_commands() {
    let Some((mut h, area)) = area_harness("tools-compass") else { return };
    let placed = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("Placeable List").map_or(0, <[mg_gff::Struct]>::len)
    };
    settle(&mut h);
    let turn = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw = 0.7;
        h.run_steps(2);
    };
    turn(&mut h);
    h.get_by_label("Compass").get_by_label("Reorient Camera").click();
    h.run_steps(3);
    let yaw = h.state().area_views[&area].orbit.unwrap().yaw;
    assert!((yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-4, "north: {yaw}");
    // With Paint on and a placeable chosen; then a blueprint in hand.
    let statue = ResKey::parse("plc_statue1", ResType::UTP).unwrap();
    for paint in [true, false] {
        {
            let app = h.state_mut();
            app.palette.tiles = false;
            app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
            app.palette.selected = Some(statue);
            app.paint = paint;
        }
        turn(&mut h);
        let before = placed(&mut h);
        let at = h.get_by_label("Compass").get_by_label("Reorient Camera").rect().center();
        h.hover_at(at);
        h.run_steps(2);
        // (Nothing follows the pointer over it: no coordinates, no dab.)
        let v = &h.state().area_views[&area];
        assert!(v.pointer.is_none(), "paint {paint}: the coordinates over the compass");
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        h.run_steps(3);
        assert_eq!(placed(&mut h), before, "paint {paint}: nothing placed");
        let yaw = h.state().area_views[&area].orbit.unwrap().yaw;
        assert!((yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-4, "paint {paint}: north");
    }
    h.state_mut().paint = false;
    h.state_mut().palette.selected = None;
    // Go to Start Location: the area's view looks at it (this module
    // starts in this area).
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let start = {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let r = ws.doc(&ifo).unwrap().root.clone();
        glam::Vec2::new(r.float("Mod_Entry_X").unwrap(), r.float("Mod_Entry_Y").unwrap())
    };
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().target =
        glam::Vec3::new(start.x + 9.0, start.y + 9.0, 0.0);
    h.run_steps(2);
    h.get_by_label("Compass").get_by_label("Go to Start Location").click();
    h.run_steps(3);
    let after = h.state().area_views[&area].orbit.unwrap().target;
    assert!((after.truncate() - start).length() < 0.5, "at the start {start}: {after}");
}

/// Reorient Camera is a command: its key (Ctrl+Alt+R, wherever the
/// pointer is) turns the camera of the area shown to face north, and
/// Quick Open finds it.
#[test]
fn reorient_by_its_key_and_quick_open() {
    let Some((mut h, area)) = area_harness("tools-reorient-key") else { return };
    settle(&mut h);
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw = 1.1;
    h.run_steps(2);
    let ctrl_alt = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    h.key_press_modifiers(ctrl_alt, egui::Key::R);
    h.run_steps(3);
    let yaw = h.state().area_views[&area].orbit.unwrap().yaw;
    assert!((yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-4, "north by its key: {yaw}");
    // Quick Open: by its name, with its key.
    h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut().unwrap().yaw = 0.4;
    command_palette(&mut h, "reorient");
    assert!(
        quick_open_rows(&h).iter().any(|r| r.starts_with("Reorient Camera")),
        "{:?}",
        quick_open_rows(&h)
    );
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    let yaw = h.state().area_views[&area].orbit.unwrap().yaw;
    assert!((yaw + std::f32::consts::FRAC_PI_2).abs() < 1e-4, "north from Quick Open: {yaw}");
}

/// The minimap stands at the view's bottom right, under the compass's
/// column and clear of it (in a view too short for both it is left out),
/// and the coordinates' line at the bottom left is cut short before it.
#[test]
fn the_minimap_stands_at_the_bottom_right() {
    let Some((mut h, area)) = area_harness("tools-minimap") else { return };
    h.state_mut().area_views.get_mut(&area).unwrap().minimap = true;
    settle(&mut h);
    let v = &h.state().area_views[&area];
    let (view, map) = (v.rect, v.map_rect.expect("the map drawn"));
    let corner = v.corner_rect.expect("the compass drawn");
    assert!((view.right() - map.right()).abs() < 14.0, "at the right: {map:?} in {view:?}");
    assert!((view.bottom() - map.bottom()).abs() < 14.0, "at the bottom: {map:?} in {view:?}");
    assert!(corner.top() - view.top() < 14.0 && view.right() - corner.right() < 14.0);
    assert!(!map.intersects(corner), "clear of the compass: {map:?} {corner:?}");
    // A short view: no map over the compass.
    let size = h.ctx.content_rect().size();
    h.set_size(egui::vec2(size.x, 300.0));
    settle(&mut h);
    let v = &h.state().area_views[&area];
    assert!(v.corner_rect.is_some(), "short: the compass drawn");
    if let (Some(map), Some(corner)) = (v.map_rect, v.corner_rect) {
        assert!(!map.intersects(corner), "short: clear of the compass: {map:?} {corner:?}");
    }
    // A narrow view, a tile selected (the coordinates' line names it too,
    // its longest), the pointer at the bottom left: the line is cut short
    // before the map.
    h.set_size(egui::vec2(520.0, 700.0));
    h.state_mut().area_views.get_mut(&area).unwrap().tile_selection = vec![(1, 1)];
    settle(&mut h);
    let rect = h.state().area_views[&area].rect;
    h.hover_at(rect.left_bottom() + egui::vec2(40.0, -40.0));
    h.run_steps(3);
    let v = &h.state().area_views[&area];
    let (map, line) = (v.map_rect.expect("the map"), v.readout_rect.expect("the line"));
    assert!(line.right() < map.left(), "the line clear of the map: {line:?} {map:?} in {rect:?}");
    assert!(map.left() - line.right() < 16.0, "cut short just before it: {line:?} {map:?}");
}

/// `hidden_area_tools` keeps its names: Select Tiles and Paint leave the
/// shelf, Reorient Camera and Go to Start Location the compass, from the
/// header's right-click list as from the toolbar before; B still turns
/// Paint on; Select Objects and Terrain stay; with Snap and Turn hidden
/// the Brush popover has no Placing heading over nothing.
#[test]
fn hidden_names_leave_the_shelf_and_the_compass() {
    let Some((mut h, area)) = area_harness("tools-hidden") else { return };
    let names = ["Select Tiles", "Paint", "Reorient Camera", "Go to Start Location"];
    h.state_mut().settings.hidden_area_tools = names.map(String::from).to_vec();
    settle(&mut h);
    let shelf = h.get_by_label(AREA_TOOLS);
    for name in ["Select Tiles", "Paint"] {
        assert!(shelf.query_by_label(name).is_none(), "{name} hidden from the shelf");
    }
    for name in ["Select Objects", "Terrain"] {
        assert!(shelf.query_by_label(name).is_some(), "{name} stays");
    }
    for name in ["Reorient Camera", "Go to Start Location"] {
        assert!(h.query_by_label(name).is_none(), "{name} hidden from the compass");
    }
    let middle = h.state().area_views[&area].rect.center();
    h.hover_at(middle);
    h.run_steps(2);
    h.key_press(egui::Key::B);
    h.run_steps(3);
    assert!(h.state().paint, "B with Paint hidden");
    // Snap and Turn hidden too: the Brush popover has no heading over
    // them.
    h.state_mut().settings.hidden_area_tools.extend(["Snap", "Turn"].map(String::from));
    h.run_steps(3);
    area_tool(&mut h, "Lock");
    assert!(h.query_by_label("Placing, as with the Select tool").is_none(), "no empty heading");
    close_area_popover(&mut h);
    h.state_mut().paint = false;
    h.state_mut().settings.hidden_area_tools.clear();
    h.run_steps(3);
    for name in names {
        area_tool(&mut h, name);
    }
}

/// F10 takes Select Tiles with the pointer anywhere over the area editor:
/// its view, its shelf and its header (not the menu bar's then: its menus
/// stay closed).
#[test]
fn f10_takes_select_tiles_over_the_whole_editor() {
    let Some((mut h, area)) = area_harness("tools-f10-editor") else { return };
    settle(&mut h);
    let shelf =
        h.get_by_label(AREA_TOOLS).get_by_label("Terrain").rect().center() + egui::vec2(0.0, 80.0);
    let header = mg_uikit::header::records(&h.ctx, AREA_HEADER)[0].row.center();
    for (at, what) in [(shelf, "the shelf"), (header, "the header")] {
        h.hover_at(at);
        h.run_steps(2);
        h.key_press(egui::Key::F10);
        h.run_steps(3);
        assert!(h.state().area_views[&area].tile_mode, "F10 over {what}");
        assert!(!egui::Popup::is_any_open(&h.ctx), "F10 over {what}: no menu");
        h.key_press(egui::Key::F10);
        h.run_steps(3);
        assert!(!h.state().area_views[&area].tile_mode, "F10 again over {what}");
    }
}

/// In a read-only area (another builder's), F10 leaves tile mode off as
/// before (the area keeps none: what changes its tiles is off), and a
/// tile mode set before it became read-only is off; Paint on elsewhere
/// shows neither its header group, its hints nor the palette's Brush
/// section there.
#[test]
fn a_read_only_area_takes_no_tool_but_select() {
    let Some((mut h, area)) = read_only_area_harness("tools-read-only") else { return };
    settle(&mut h);
    h.state_mut().area_views.get_mut(&area).unwrap().tile_mode = true;
    h.run_steps(2);
    assert!(!h.state().area_views[&area].tile_mode, "tile mode off in a read-only area");
    let middle = h.state().area_views[&area].rect.center();
    h.hover_at(middle);
    h.run_steps(2);
    h.key_press(egui::Key::F10);
    h.run_steps(3);
    assert!(!h.state().area_views[&area].tile_mode, "F10: still off");
    assert_eq!(on(&h), ["Select Objects"]);
    // Paint on (taken in another area), placeables chosen: none of it here.
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.selected = ResKey::parse("plc_statue1", ResType::UTP);
        app.paint = true;
    }
    show_palette(h.state_mut());
    settle(&mut h);
    assert!(h.query_by_label("Brush blueprints").is_none(), "no Brush section");
    assert!(h.get_by_label(AREA_HEADER).query_by_label("How the brush paints").is_none());
    let status = h.get_by_label("Status Bar");
    assert!(status.query_by_label_contains("Painting").is_none(), "no Paint hints");
}

/// The Tile window's Paint with It takes Terrain as the shelf does: Paint
/// and Select Tiles put down, the tile in hand, Terrain lit.
#[test]
fn paint_with_it_takes_terrain() {
    let Some((mut h, area)) = area_harness("tools-paint-with-it") else { return };
    settle(&mut h);
    shelf_click(&mut h, "Paint");
    let tileset = ResRef::from_str("ttr01").unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Tile { tileset, tile: 10 }));
    settle(&mut h);
    h.get_by_label("Paint with It").click();
    h.run_steps(3);
    assert!(!h.state().paint, "Paint put down");
    assert!(h.state().palette.tile_brush.is_some(), "the tile in hand");
    assert!(!h.state().area_views[&area].tile_mode);
    assert_eq!(on(&h), ["Terrain"]);
}

#[test]
fn paint_blocks_non_placeable_fallthrough_and_select_tiles_exits_on_escape() {
    let Some((mut h, area)) = area_harness("tools-paint-guard-tile-esc") else { return };
    settle(&mut h);

    // Paint on with a non-placeable blueprint selected: warns cleanly, doesn't place, Esc stops Paint.
    let was_objects = h.state().area_views[&area].model.as_ref().map_or(0, |m| m.objects.len());
    shelf_click(&mut h, "Paint");
    h.state_mut().palette.selected = ResKey::parse("nw_waypoint001", ResType::UTW);
    settle(&mut h);
    let status = h.get_by_label("Status Bar");
    assert!(
        status.query_by_label_contains("Paint works with placeables").is_some(),
        "Status Bar explains Paint works with placeables"
    );
    let spot = h.state().area_views[&area].rect.center();
    h.hover_at(spot);
    h.step();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: spot,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    settle(&mut h);
    let now_objects = h.state().area_views[&area].model.as_ref().map_or(0, |m| m.objects.len());
    assert_eq!(now_objects, was_objects, "no non-placeable placed while Paint active");
    h.hover_at(spot);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().paint, "Esc exits Paint even with non-placeable selected");

    // Select Tiles: first Esc clears tile selection, second Esc exits to Select Objects.
    shelf_click(&mut h, "Select Tiles");
    h.state_mut().area_views.get_mut(&area).unwrap().tile_selection = vec![(0, 0), (1, 0)];
    settle(&mut h);
    assert!(
        h.state().status_said().iter().any(|s| s == "Esc: Clear selection"),
        "Status Bar hints Esc clears tile selection: {:?}",
        h.state().status_said()
    );
    h.hover_at(spot);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().area_views[&area].tile_mode, "first Esc keeps tile mode");
    assert!(h.state().area_views[&area].tile_selection.is_empty(), "first Esc clears tiles");
    assert!(
        h.state().status_said().iter().any(|s| s == "Esc: Select objects"),
        "Status Bar hints Esc exits tile mode: {:?}",
        h.state().status_said()
    );
    h.hover_at(spot);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!h.state().area_views[&area].tile_mode, "second Esc exits tile mode");
}
