//! The galleries' keys (M4's U25 review): each of the seven kit grids
//! (the Appearance Gallery, Select Texture, an armor's parts, the Creature
//! Wizard's appearances, Tile Properties' variants, the tiles palette's
//! groups and tiles) takes no key while a menu or popup is open over the
//! window, and takes them again once it is gone; the three that open at
//! the one chosen open there, far down their lists; the variants' size
//! slider runs from 64 to 224, at 84 until it is moved.

use crate::common::*;
use egui::accesskit::Role;

/// The kit grid named `name`: a list, as a screen reader has it (the
/// first of that name).
fn grid<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    let mut all = h.query_all_by(|n| n.role() == Role::List && n.label().as_deref() == Some(name));
    all.next().unwrap_or_else(|| panic!("no grid {name}"))
}

/// The grid's keys' cell, as a screen reader is told it (the grid's active
/// one): its name and its place in the list (from 1). None where the grid
/// has none, or it is out of sight (not built).
pub(crate) fn grid_cursor(h: &Harness<'_, Moonglow>, name: &str) -> Option<(String, usize)> {
    let at = grid(h, name).accesskit_node().data().active_descendant()?;
    let cell = h.query_by(|n| n.locate().0 == at)?;
    let n = cell.accesskit_node();
    Some((n.label().unwrap_or_default(), n.data().position_in_set()?))
}

/// The places (from 1) of the grid's cells built: those in sight and a
/// row ahead.
fn built(h: &Harness<'_, Moonglow>, name: &str) -> Vec<usize> {
    let count = grid(h, name).accesskit_node().data().size_of_set();
    h.query_all_by(|n| {
        n.role() != Role::List
            && n.data().size_of_set() == count
            && n.data().position_in_set().is_some()
    })
    .filter_map(|n| n.accesskit_node().data().position_in_set())
    .collect()
}

/// The grid's cell at `place` (from 1) clicked, which gives the grid the
/// keys (its cursor there).
fn click_cell(h: &mut Harness<'_, Moonglow>, name: &str, place: usize) {
    let count = grid(h, name).accesskit_node().data().size_of_set();
    h.get_by(|n| {
        n.role() != Role::List
            && n.data().size_of_set() == count
            && n.data().position_in_set() == Some(place)
    })
    .click();
    settle(h);
}

/// The grid named `name` given the keys (its first cell in sight
/// clicked): its arrows, End and Page Down move nothing while a menu or
/// popup is open (one held open for each frame, as a menu shown is), and
/// the Right arrow moves its cursor once it is closed.
fn keys_wait_for_menus(h: &mut Harness<'_, Moonglow>, name: &str) {
    let first = *built(h, name).iter().min().expect("cells in sight");
    click_cell(h, name, first);
    let before = grid_cursor(h, name);
    assert_eq!(
        before.as_ref().map(|c| c.1),
        Some(first),
        "{name}: the cell clicked has the cursor"
    );
    let menu = egui::Id::new("mg-test-menu");
    for key in [egui::Key::ArrowRight, egui::Key::ArrowDown, egui::Key::End, egui::Key::PageDown] {
        egui::Popup::open_id(&h.ctx, menu);
        h.key_press(key);
        egui::Popup::open_id(&h.ctx, menu);
        h.step();
        egui::Popup::open_id(&h.ctx, menu);
        h.step();
        assert_eq!(grid_cursor(h, name), before, "{name}: {key:?} taken with a menu open");
    }
    egui::Popup::close_all(&h.ctx);
    settle(h);
    h.key_press(egui::Key::ArrowRight);
    settle(h);
    let after = grid_cursor(h, name).map(|c| c.1);
    assert_eq!(after, Some(first + 1), "{name}: the Right arrow, the menu closed");
}

/// The Appearance Gallery of a placeable.
fn appearance_gallery() -> Option<Harness<'static, Moonglow>> {
    let (mut h, _) = blueprint_harness("plc_chest1", "keys_appearances", ResType::UTP)?;
    h.get_by_label("Gallery…").click();
    settle(&mut h);
    Some(h)
}

/// Select Texture, opened for a chest's first texture.
fn select_texture(name: &str) -> Option<Harness<'static, Moonglow>> {
    let (mut h, _) = blueprint_harness("plc_chest1", name, ResType::UTP)?;
    page(&mut h, "Visuals");
    settle(&mut h);
    h.get_all_by_label("Select Texture…").next().unwrap().click();
    settle(&mut h);
    Some(h)
}

/// An armor's Appearance page, tall enough for the parts' pictures, with
/// its thighs' pictures shown; `thighs` given to it first where given.
fn armor_thighs(name: &str, thighs: Option<i64>) -> Option<Harness<'static, Moonglow>> {
    let (mut h, key) = blueprint_harness("nw_aarcl001", name, ResType::UTI)?;
    h.set_size(egui::vec2(1000.0, 1200.0));
    own_window(&mut h, &Tab::Blueprint(key), egui::pos2(10.0, 10.0), egui::vec2(980.0, 1180.0));
    page(&mut h, "Appearance");
    settle(&mut h);
    if let Some(n) = thighs {
        let set = |label: &str| mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: label.into(),
            value: Some(mg_gff::Value::Byte(n as u8)),
        };
        let edits = vec![set("ArmorPart_LThigh"), set("ArmorPart_RThigh")];
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("thighs", edits)));
        settle(&mut h);
    }
    h.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("Thighs")).click();
    settle(&mut h);
    let started = std::time::Instant::now();
    while h.query_by(|n| n.role() == Role::List && n.label().is_some()).is_none() {
        assert!(started.elapsed().as_secs() < 20, "no thighs' pictures");
        settle(&mut h);
    }
    Some(h)
}

/// The Creature Wizard on its Appearance page, `appearance` chosen.
fn creature_wizard(name: &str, appearance: u16) -> Option<Harness<'static, Moonglow>> {
    let (mut h, _) = area_harness(name)?;
    let mut w = mg_ui::creature_wizard::CreatureWizard::default();
    (w.page, w.race, w.appearance) = (3, Some(6), appearance);
    w.classes = vec![(4, 1)];
    h.state_mut().creature_wizard = Some(w);
    settle(&mut h);
    Some(h)
}

/// Tile Properties of a tile of the area (one chosen with Select Tiles,
/// its right-click menu's item): its variants as pictures.
fn tile_variants(name: &str) -> Option<Harness<'static, Moonglow>> {
    let (mut h, area) = area_harness(name)?;
    area_click(&mut h, "Select Tiles");
    settle(&mut h);
    let at = screen(&h, area, glam::Vec3::new(15.0, 15.0, 0.0));
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    settle(&mut h);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        });
    }
    settle(&mut h);
    popup_item(&h, "Tile Properties").click();
    settle(&mut h);
    Some(h)
}

/// The tiles palette's Gallery, narrowed to `find` (a search opens its
/// headings).
fn tiles_gallery(name: &str, find: &str) -> Option<Harness<'static, Moonglow>> {
    let (mut h, _) = area_harness(name)?;
    // (Tall enough for the tiles under the groups to be in sight.)
    h.set_size(egui::vec2(1600.0, 1400.0));
    h.state_mut().settings.palette_gallery = true;
    show_palette(h.state_mut());
    settle(&mut h);
    palette_type(&mut h, "Tiles");
    settle(&mut h);
    h.state_mut().palette.filter = find.into();
    settle(&mut h);
    Some(h)
}

#[test]
fn the_appearance_gallery_takes_no_keys_with_a_menu_open() {
    let Some(mut h) = appearance_gallery() else { return };
    keys_wait_for_menus(&mut h, "Placeables");
}

#[test]
fn select_texture_takes_no_keys_with_a_menu_open() {
    let Some(mut h) = select_texture("keys_texture") else { return };
    keys_wait_for_menus(&mut h, "Textures");
}

#[test]
fn an_armor_s_parts_take_no_keys_with_a_menu_open() {
    let Some(mut h) = armor_thighs("keys_armor", None) else { return };
    keys_wait_for_menus(&mut h, "Pictures of Thighs");
}

#[test]
fn the_creature_wizard_s_appearances_take_no_keys_with_a_menu_open() {
    let Some(mut h) = creature_wizard("keys_wizard", 6) else { return };
    keys_wait_for_menus(&mut h, "Appearances");
}

#[test]
fn tile_variants_take_no_keys_with_a_menu_open() {
    let Some(mut h) = tile_variants("keys_variants") else { return };
    keys_wait_for_menus(&mut h, "Variants");
}

#[test]
fn the_tiles_palette_s_groups_take_no_keys_with_a_menu_open() {
    let Some(mut h) = tiles_gallery("keys_groups", "wall") else { return };
    keys_wait_for_menus(&mut h, "Groups");
}

#[test]
fn the_tiles_palette_s_tiles_take_no_keys_with_a_menu_open() {
    let Some(mut h) = tiles_gallery("keys_tiles", "wall") else { return };
    keys_wait_for_menus(&mut h, "Tiles");
}

/// A gallery opened at the one chosen far down its list: that cell is the
/// grid's cursor, built (in sight), and the list's first is not.
fn opened_far(h: &mut Harness<'_, Moonglow>, name: &str, at: &str) {
    let started = std::time::Instant::now();
    loop {
        settle(h);
        if grid_cursor(h, name).is_some() || started.elapsed().as_secs() > 10 {
            break;
        }
    }
    let cursor = grid_cursor(h, name);
    let (label, place) = cursor.unwrap_or_else(|| panic!("{at}: the chosen not in sight"));
    let built = built(h, name);
    assert!(built.contains(&place), "{at}: {label} ({place}) not built: {built:?}");
    assert!(!built.contains(&1), "{at}: {label} is not far down ({place}): {built:?}");
    // (In the window, not merely built.)
    let window = h.ctx.content_rect();
    let cell = h
        .query_by(|n| n.data().position_in_set() == Some(place) && n.label() == Some(label.clone()))
        .unwrap()
        .rect();
    assert!(window.contains_rect(cell), "{at}: {label} at {cell:?}, out of {window:?}");
}

/// (A texture far down the list chosen, OK, and Select Texture opened
/// again for the same texture.)
#[test]
fn select_texture_opens_at_the_texture_chosen() {
    let Some(mut h) = select_texture("far_texture") else { return };
    let by_hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Find");
    h.get(egui_kittest::kittest::by().predicate(by_hint)).click();
    settle(&mut h);
    h.get(egui_kittest::kittest::by().predicate(by_hint)).type_text(FAR_TEXTURE);
    settle(&mut h);
    click_cell(&mut h, "Textures", 1);
    let (chosen, _) = grid_cursor(&h, "Textures").expect("the texture found, chosen");
    h.get_all_by_label("OK").last().unwrap().click();
    settle(&mut h);
    h.get_all_by_label("Select Texture…").next().unwrap().click();
    settle(&mut h);
    opened_far(&mut h, "Textures", "Select Texture");
    assert_eq!(grid_cursor(&h, "Textures").unwrap().0, chosen);
}

/// What finds textures far down Select Texture's list (the rural
/// tileset's).
const FAR_TEXTURE: &str = "ttr01_";

/// (Thighs 186: the last of an armor's thighs, its third row, two rows
/// in sight.)
#[test]
fn an_armor_s_parts_open_at_the_armor_s_own() {
    let Some(mut h) = armor_thighs("far_armor", Some(FAR_THIGHS)) else { return };
    opened_far(&mut h, "Pictures of Thighs", "the armor's thighs");
    let (label, _) = grid_cursor(&h, "Pictures of Thighs").unwrap();
    assert_eq!(label, FAR_THIGHS.to_string());
}

/// An armor's thighs far down its pictures.
const FAR_THIGHS: i64 = 186;

/// (Appearance 198, a zombie: far down the appearances by name.)
#[test]
fn the_creature_wizard_opens_at_the_appearance_chosen() {
    let Some(mut h) = creature_wizard("far_wizard", FAR_APPEARANCE) else { return };
    opened_far(&mut h, "Appearances", "the Creature Wizard");
}

/// An appearance far down the Creature Wizard's pictures (by name).
const FAR_APPEARANCE: u16 = 198;

/// Tile Properties' variants' Size slider: 64 to 224 (the pictures' own
/// size), 84 until it is moved.
#[test]
fn the_variants_slider_runs_from_64_to_224_from_84() {
    let Some(h) = tile_variants("variants_slider") else { return };
    let slider = h.get_by(|n| n.role() == Role::Slider && n.label().as_deref() == Some("Size"));
    let n = slider.accesskit_node();
    let data = n.data();
    assert_eq!(data.min_numeric_value(), Some(64.0), "its least");
    assert_eq!(data.max_numeric_value(), Some(224.0), "its most");
    assert_eq!(data.numeric_value(), Some(84.0), "its first");
}
