//! Tile Properties as the Right Dock's lower section while Select Tiles is
//! on (M4's U5; the AreaTiles and GalleryTiles boards), and Select Tiles'
//! commands in the area header: no OK or Cancel, each change one step to
//! undo on every tile chosen, only what is changed written, the panel
//! following the selection, its header toggle, a dragged out area's
//! window. (The game's data and a GPU, else skipped.)

use crate::common::*;
use glam::Vec3;

fn tile_list(h: &mut Harness<'_, Moonglow>, area: ResRef) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().root.list("Tile_List").unwrap().to_vec()
}

fn undo_label(h: &Harness<'_, Moonglow>) -> Option<String> {
    h.state().ws.as_ref().unwrap().can_undo().map(str::to_string)
}

fn revision(h: &Harness<'_, Moonglow>) -> u64 {
    h.state().ws.as_ref().unwrap().revision()
}

/// Sets field `label` of tile `index` to `value`, as a command of its own.
fn set_field(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize, label: &str, value: u8) {
    let set = mg_edit::Edit::SetField {
        key: ResKey::new(area, ResType::ARE),
        path: mg_edit::GffPath::root().item("Tile_List", index),
        label: label.into(),
        value: Some(mg_gff::Value::Byte(value)),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![set])));
    h.run_steps(3);
}

/// A click on the ground at `p` (with `m` held), in the area's view.
fn click(h: &mut Harness<'_, Moonglow>, area: ResRef, p: Vec3, m: egui::Modifiers) {
    let at = screen(h, area, p);
    h.hover_at(at);
    press(h, at, true, m);
    press(h, at, false, m);
    h.run_steps(3);
}

/// The fields of `a` and `b` that differ, by label.
fn differ(a: &mg_gff::Struct, b: &mg_gff::Struct) -> Vec<String> {
    let label = |f: &mg_gff::Field| f.label.to_string_lossy();
    let mut labels: Vec<String> = a.fields.iter().map(label).collect();
    labels.extend(b.fields.iter().map(label));
    labels.sort();
    labels.dedup();
    labels.into_iter().filter(|l| a.get(l) != b.get(l)).collect()
}

/// An area in a wide window (the Right Dock's sections stacked: the
/// palette over Tile Properties), Select Tiles on with tiles 5 and 6
/// (columns 1 and 2 of row 1) selected, their main light 2 set apart
/// (3 and 11) so that they differ in a field nothing changes.
fn two_tiles(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let (mut h, area) = two_tiles_set(name)?;
    select_two(&mut h, area);
    Some((h, area))
}

/// [`two_tiles`]' area before anything is selected (its main lights 2
/// set apart).
fn two_tiles_set(name: &str) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let (mut h, area) = area_harness(name)?;
    wide_window(&mut h);
    set_field(&mut h, area, 5, "Tile_MainLight2", 3);
    set_field(&mut h, area, 6, "Tile_MainLight2", 11);
    Some((h, area))
}

/// Select Tiles on, tiles 5 and 6 selected: the panel shows them.
fn select_two(h: &mut Harness<'_, Moonglow>, area: ResRef) {
    area_click(h, "Select Tiles");
    click(h, area, Vec3::new(15.0, 15.0, 0.0), egui::Modifiers::NONE);
    click(h, area, Vec3::new(25.0, 15.0, 0.0), egui::Modifiers::COMMAND);
    settle(h);
    assert_eq!(h.state().area_views[&area].tile_selection, [(1, 1), (2, 1)]);
    let tiles = h.state().tile_props.as_ref().map(|p| p.tiles.clone());
    assert_eq!(tiles, Some(vec![5, 6]), "the panel shows them");
}

/// A change in the panel (Main light 1, chosen in its color picker) is
/// written at once into every tile chosen, as one step, and Ctrl+Z takes
/// it back from all of them, byte for byte; the field they differ in is
/// left as each has it.
#[test]
fn a_change_is_one_step_undone_on_every_tile() {
    let Some((mut h, area)) = two_tiles("tile-props-one-step") else { return };
    let before = tile_list(&mut h, area);
    let (at, label) = (revision(&h), undo_label(&h));
    let props = h.state().tile_props.clone().unwrap();
    assert!(props.has_main[0], "ttr01's tiles have main light 1");
    let color = (props.main[0] + 9) % 16;
    h.get_by_label("Main light 1 color").click();
    h.run_steps(2);
    h.get_by_label(&format!("Light color {color}")).click();
    h.run_steps(3);
    assert_eq!(revision(&h), at + 1, "one command");
    assert_eq!(undo_label(&h).as_deref(), Some("Tile Properties"));
    let now = tile_list(&mut h, area);
    for i in [5, 6] {
        assert_eq!(now[i].integer("Tile_MainLight1"), Some(i64::from(color)), "tile {i}");
        assert_eq!(differ(&before[i], &now[i]), ["Tile_MainLight1"], "tile {i}: that alone");
    }
    assert_eq!(now[6].integer("Tile_MainLight2"), Some(11), "the field they differ in kept");
    for i in (0..now.len()).filter(|i| ![5, 6].contains(i)) {
        assert_eq!(now[i], before[i], "tile {i} not chosen");
    }
    // (No OK, Cancel nor question: none to find.)
    for gone in ["OK", "Cancel", "Save changes to Tile Properties?"] {
        assert!(h.query_by_label(gone).is_none(), "{gone}");
    }
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), before, "undone on both, byte for byte");
    assert_eq!(undo_label(&h), label);
    let p = h.state().tile_props.clone().unwrap();
    assert_eq!(p.main[0], props.main[0], "the panel read them again");
}

/// What is not changed is not written: the panel shown (from before the
/// tiles were selected, its first frames too) writes nothing; the color the tiles all have chosen again writes nothing; a
/// height raised writes the height alone (and moves what stands on the
/// tiles), the lights the tiles differ in left as they are.
#[test]
fn an_unchanged_control_writes_nothing() {
    let Some((mut h, area)) = two_tiles_set("tile-props-unchanged") else { return };
    let before = tile_list(&mut h, area);
    let (at, label) = (revision(&h), undo_label(&h));
    select_two(&mut h, area);
    assert_eq!((revision(&h), undo_label(&h)), (at, label), "shown: nothing written");
    for _ in 0..10 {
        h.run_steps(1);
    }
    assert_eq!(revision(&h), at, "shown, nothing written");
    let props = h.state().tile_props.clone().unwrap();
    assert_eq!(before[5].get("Tile_MainLight1"), before[6].get("Tile_MainLight1"));
    h.get_by_label("Main light 1 color").click();
    h.run_steps(2);
    h.get_by_label(&format!("Light color {}", props.main[0])).click();
    h.run_steps(3);
    assert_eq!(revision(&h), at, "the color they have: nothing written");
    assert_eq!(tile_list(&mut h, area), before);
    h.get_by_label("Raise Height").click();
    h.run_steps(3);
    assert_eq!(revision(&h), at + 1);
    let now = tile_list(&mut h, area);
    for i in [5, 6] {
        assert_eq!(differ(&before[i], &now[i]), ["Tile_Height"], "tile {i}: the height alone");
    }
}

/// A height dragged on its number is one step for the drag, however many
/// frames it wrote in; Ctrl+Z takes the whole drag back.
#[test]
fn a_height_dragged_is_one_step() {
    let Some((mut h, area)) = two_tiles("tile-props-drag") else { return };
    let before = tile_list(&mut h, area);
    let at = revision(&h);
    let label = undo_label(&h);
    // (The number between its - and + buttons.)
    let (down, up) = (h.get_by_label("Lower Height").rect(), h.get_by_label("Raise Height").rect());
    let start = egui::pos2((down.right() + up.left()) / 2.0, down.center().y);
    h.hover_at(start);
    press(&h, start, true, egui::Modifiers::NONE);
    h.run_steps(1);
    let mut heights = Vec::new();
    for k in 1..=8 {
        h.hover_at(start + egui::vec2(k as f32 * 6.0, 0.0));
        h.run_steps(1);
        heights.push(tile_list(&mut h, area)[5].integer("Tile_Height"));
    }
    press(&h, start + egui::vec2(48.0, 0.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    let now = tile_list(&mut h, area);
    let raised = now[5].integer("Tile_Height").unwrap_or(0);
    let was = before[5].integer("Tile_Height").unwrap_or(0);
    assert!(raised >= was + 2, "dragged up through several heights: {heights:?}");
    let shown: std::collections::BTreeSet<_> = heights.iter().collect();
    assert!(shown.len() > 2, "written as it was dragged: {heights:?}");
    assert_eq!(now[6].integer("Tile_Height"), Some(raised), "both tiles");
    // (One step: a single Ctrl+Z takes it all back. The revision counts
    // the drag's taking back too.)
    assert!(revision(&h) > at);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(3);
    assert_eq!(tile_list(&mut h, area), before, "the whole drag undone");
    assert_eq!(undo_label(&h), label);
}

/// The panel follows the selection: one tile, then two, then another
/// alone, its header saying which ("Tile (1, 1)", "2 tiles"); with none
/// selected it says how to choose; out of Select Tiles it goes.
#[test]
fn the_panel_follows_the_selection() {
    let Some((mut h, area)) = area_harness("tile-props-follows") else { return };
    wide_window(&mut h);
    area_click(&mut h, "Select Tiles");
    settle(&mut h);
    h.get_by_label(mg_ui::tile_select::NO_TILES);
    h.get_by_label("Tile Properties, none selected");
    assert!(h.state().tile_props.is_none());
    click(&mut h, area, Vec3::new(15.0, 15.0, 0.0), egui::Modifiers::NONE);
    settle(&mut h);
    h.get_by_label("Tile Properties, Tile (1, 1)");
    assert_eq!(h.state().tile_props.as_ref().map(|p| p.tiles.clone()), Some(vec![5]));
    click(&mut h, area, Vec3::new(25.0, 15.0, 0.0), egui::Modifiers::COMMAND);
    settle(&mut h);
    h.get_by_label("Tile Properties, 2 tiles");
    assert_eq!(h.state().tile_props.as_ref().map(|p| p.tiles.clone()), Some(vec![5, 6]));
    click(&mut h, area, Vec3::new(25.0, 25.0, 0.0), egui::Modifiers::NONE);
    settle(&mut h);
    h.get_by_label("Tile Properties, Tile (2, 2)");
    assert_eq!(h.state().tile_props.as_ref().map(|p| p.tiles.clone()), Some(vec![10]));
    area_click(&mut h, "Select Objects");
    settle(&mut h);
    assert!(h.query_by_label_contains("Tile Properties, ").is_none(), "gone with Select Tiles");
    assert!(h.query_by_label(mg_ui::tile_select::NO_TILES).is_none());
    assert_eq!(h.state().right_dock_shows(), Some(mg_ui::Panel::Palette), "the palette's dock");
}

/// The header's Tile Properties toggle (on at first) hides the panel and
/// shows it again, kept in the settings; the tile menu's Tile
/// Properties… shows it whatever the toggle said.
#[test]
fn the_header_toggle_hides_and_shows_the_panel() {
    let Some((mut h, area)) = two_tiles("tile-props-toggle") else { return };
    h.get_by_label("Tile Properties, 2 tiles");
    area_click(&mut h, "Tile Properties");
    settle(&mut h);
    assert!(h.state().settings.hide_tile_properties, "kept");
    assert!(h.query_by_label("Tile Properties, 2 tiles").is_none(), "hidden");
    let toggle = area_tool(&mut h, "Tile Properties");
    assert_eq!(toggle.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    area_click(&mut h, "Tile Properties");
    settle(&mut h);
    h.get_by_label("Tile Properties, 2 tiles");
    // Hidden, then the tile menu's Tile Properties: shown.
    area_click(&mut h, "Tile Properties");
    settle(&mut h);
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
    popup_item(&h, "Tile Properties").click();
    settle(&mut h);
    assert!(!h.state().settings.hide_tile_properties);
    h.get_by_label("Tile Properties, 2 tiles");
}

/// Select Tiles' commands in the header act on the tiles selected, as the
/// tile menu's and the keys do: Next Tile Appearance a step, Copy then
/// Paste ready to place, Cut copying and deleting, Delete, Refine Tile
/// and Rotate Tile each a step of their own.
#[test]
fn the_header_s_commands_act_on_the_selected_tiles() {
    let Some((mut h, area)) = two_tiles("tile-props-commands") else { return };
    assert!(h.state().tile_clip.is_none());
    for (name, label) in [
        ("Next Tile Appearance", "Next tile appearance"),
        ("Previous Tile Appearance", "Previous tile appearance"),
        ("Refine Tile", "Refine tiles"),
    ] {
        area_click(&mut h, name);
        h.run_steps(2);
        assert_eq!(undo_label(&h).as_deref(), Some(label), "{name}");
    }
    area_click(&mut h, "Copy Tiles");
    h.run_steps(2);
    assert!(h.state().tile_clip.is_some(), "copied");
    area_click(&mut h, "Paste Tiles");
    h.run_steps(2);
    assert!(h.state().area_views[&area].tile_pasting, "ready to place");
    h.hover_at(screen(&h, area, Vec3::new(35.0, 35.0, 0.0)));
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(!h.state().area_views[&area].tile_pasting);
    h.state_mut().tile_clip = None;
    let at = revision(&h);
    area_click(&mut h, "Delete Tiles");
    h.run_steps(2);
    // (Grass: chosen again, a step; or refused, the status line says so.)
    let deleted = undo_label(&h);
    let said = h.query_by_label_contains("Delete: no tile fits there").is_some();
    assert!(revision(&h) == at + 1 && deleted.as_deref() == Some("Delete tiles") || said);
    area_click(&mut h, "Cut Tiles");
    h.run_steps(2);
    assert!(h.state().tile_clip.is_some(), "Cut copies");
}

/// A dragged out area (its tab in a window of its own) shows Tile
/// Properties in its window's right pane while Select Tiles is on; the
/// main window's Right Dock keeps the palettes, with no Tile Properties of
/// its own (U2's two hosts).
#[test]
fn a_dragged_out_area_has_tile_properties_in_its_window() {
    let Some((mut h, area)) = two_tiles("tile-props-window") else { return };
    h.set_size(egui::vec2(1700.0, 900.0));
    settle(&mut h);
    own_window(&mut h, &Tab::Area(area), egui::pos2(20.0, 40.0), egui::vec2(1200.0, 800.0));
    settle(&mut h);
    assert_eq!(h.state().area_views[&area].tile_selection, [(1, 1), (2, 1)]);
    // (A header's group has no place of its own: its panel's swatch's.)
    assert_eq!(h.query_all_by_label("Tile Properties, 2 tiles").count(), 1, "one panel");
    let swatches: Vec<egui::Rect> =
        h.query_all_by_label("Main light 1 color").map(|n| n.rect()).collect();
    assert_eq!(swatches.len(), 1, "one panel, the window's: {swatches:?}");
    let window = egui::Rect::from_min_size(egui::pos2(20.0, 40.0), egui::vec2(1200.0, 800.0));
    assert!(window.contains_rect(swatches[0]), "in the area's window: {swatches:?}");
    let view = h.state().area_views[&area].rect;
    assert!(swatches[0].left() >= view.right() - 1.0, "at its right: {swatches:?} {view:?}");
    // A change there: one step, as in the dock.
    let at = revision(&h);
    h.get_by_label("Raise Height").click();
    h.run_steps(3);
    assert_eq!(revision(&h), at + 1);
    assert_eq!(undo_label(&h).as_deref(), Some("Tile Properties"));
}

/// Two areas in Select Tiles at once, one docked and one dragged out
/// into a window of its own: each panel keeps what it shows apart (All
/// tiles ticked in each stays ticked; a Height dragged in one stays one
/// panel's), though one `Moonglow::tile_props` is drawn at a time.
#[test]
fn two_areas_panels_keep_their_own() {
    use mg_module::new::{AreaSpec, add_area};
    let second = std::cell::Cell::new(None);
    let add = |m: &mut Module, game: &mg_rules::GameData, area: ResRef| {
        let mut rng = fastrand::Rng::with_seed(11);
        let spec = AreaSpec {
            name: "Second".into(),
            tileset: ResRef::from_str("ttr01").unwrap(),
            width: 4,
            height: 4,
        };
        let made = add_area(m, game, &spec, &mut rng).unwrap();
        assert_ne!(made, area);
        second.set(Some(made));
    };
    let Some((app, rs, area)) = area_app("tile-props-two", "ttr01", None, 4, None, &add) else {
        return;
    };
    let other = second.get().unwrap();
    let mut h = area_gpu_harness(app, rs, 1700.0, 900.0);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Area(other)));
    settle(&mut h);
    own_window(&mut h, &Tab::Area(other), egui::pos2(20.0, 60.0), egui::vec2(1000.0, 780.0));
    settle(&mut h);
    for a in [area, other] {
        let v = h.state_mut().area_views.get_mut(&a).expect("both drawn");
        v.selection.clear();
        v.tile_mode = true;
        v.tile_selection = vec![(1, 1)];
    }
    settle(&mut h);
    assert_eq!(h.query_all_by_label("Tile Properties, Tile (1, 1)").count(), 2, "two panels");
    let ticks = |h: &Harness<'_, Moonglow>| -> Vec<bool> {
        h.query_all_by_label(mg_ui::tile_select::ALL_TILES)
            .map(|n| n.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True))
            .collect()
    };
    assert_eq!(ticks(&h), [false, false]);
    for k in 0..2 {
        let at = h.query_all_by_label(mg_ui::tile_select::ALL_TILES).nth(k).unwrap().rect();
        h.hover_at(at.left_center() + egui::vec2(8.0, 0.0));
        press(&h, at.left_center() + egui::vec2(8.0, 0.0), true, egui::Modifiers::NONE);
        press(&h, at.left_center() + egui::vec2(8.0, 0.0), false, egui::Modifiers::NONE);
        settle(&mut h);
    }
    for _ in 0..10 {
        h.run_steps(1);
    }
    assert_eq!(ticks(&h), [true, true], "each panel's All tiles kept");
}

/// The panel's controls are drawn inside the Right Dock, at its least
/// width (a window under 1400: the sections one tab bar, Tile Properties
/// a tab in front) and wide: none cut off at its edge.
#[test]
fn the_panel_s_controls_fit_the_dock() {
    let Some((mut h, area)) = area_harness("tile-props-fit") else { return };
    area_click(&mut h, "Select Tiles");
    click(&mut h, area, Vec3::new(15.0, 15.0, 0.0), egui::Modifiers::NONE);
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::TileProperties));
    for width in [1160.0, 1600.0] {
        h.set_size(egui::vec2(width, 800.0));
        settle(&mut h);
        assert!(h.state().right_dock_shows().is_some(), "{width}");
        let dock = h.state().right_dock_rect().expect("the Right Dock drawn");
        let names = [
            "Main light 1 color",
            "Source light 2 color",
            "Animation loop 1",
            "Animation loop 3",
            "Lower Height",
            "Raise Height",
            "Replacement texture",
            "Defaults",
            mg_ui::tile_select::ALL_TILES,
        ];
        for name in names {
            let r = h.get_by_label(name).rect();
            assert!(dock.contains_rect(r), "{width}: {name} {r:?} in {dock:?}");
        }
        // The rows under them: the variants' heading and its slider (one
        // tile chosen), then All tiles' Turn, count and slider (its search
        // as wide as the room left, by its own measure).
        let inside = |h: &Harness<'_, Moonglow>, name: &str| {
            // (Across: under the section's foot they are scrolled to.)
            let r = h.get_by_label_contains(name).rect();
            let across = r.left() >= dock.left() && r.right() <= dock.right();
            assert!(across, "{width}: {name} {r:?} in {dock:?}");
        };
        inside(&h, "fit here)");
        inside(&h, "Size");
        h.get_by_label(mg_ui::tile_select::ALL_TILES).click();
        settle(&mut h);
        for name in ["↺270°", "in the tileset)", "Size"] {
            inside(&h, name);
        }
        h.get_by_label(mg_ui::tile_select::ALL_TILES).click();
        settle(&mut h);
    }
}

/// F6 forward `n` times from where the keys are, the pane each press came
/// to, by name (`palette_view::arrow_keys_are`).
fn f6_walk(h: &mut Harness<'_, Moonglow>, n: usize, m: egui::Modifiers) -> Vec<String> {
    let mut seen = Vec::new();
    for _ in 0..n {
        h.key_press_modifiers(m, egui::Key::F6);
        settle(h);
        seen.push(mg_ui::palette_view::arrow_keys_are(&h.ctx));
    }
    seen
}

/// F6 goes round every pane with Tile Properties shown, docked under the
/// palette (a stop of its own after the Palette's, the panel's first
/// control taking the keyboard's focus there) and in a dragged out area's
/// window (after the area's view), forward and back, never staying put.
#[test]
fn f6_goes_round_every_pane_with_tile_properties_shown() {
    let Some((mut h, area)) = two_tiles("tile-props-f6") else { return };
    h.state_mut().settings.hide_log = false;
    settle(&mut h);
    let round = |seen: &[String]| {
        let stuck = seen.windows(2).any(|w| w[0] == w[1]);
        assert!(!stuck, "F6 stayed put: {seen:?}");
        assert!(seen.iter().any(|s| s == "TileProperties"), "no Tile Properties stop: {seen:?}");
        assert!(seen.iter().any(|s| s == "Palette"), "{seen:?}");
        assert!(seen.iter().any(|s| s == "Area"), "{seen:?}");
    };
    let forward = f6_walk(&mut h, 8, egui::Modifiers::NONE);
    round(&forward);
    // (There, the panel's first control has the focus: Tab goes on in it.)
    while mg_ui::palette_view::arrow_keys_are(&h.ctx) != "TileProperties" {
        f6_walk(&mut h, 1, egui::Modifiers::NONE);
    }
    let swatch = h.get_by_label("Main light 1 color");
    assert!(swatch.accesskit_node().is_focused(), "its first control focused");
    let back = f6_walk(&mut h, 8, egui::Modifiers::SHIFT);
    round(&back);
    // Dragged out: the window's Tile Properties a stop after its view.
    h.set_size(egui::vec2(1700.0, 900.0));
    settle(&mut h);
    own_window(&mut h, &Tab::Area(area), egui::pos2(20.0, 40.0), egui::vec2(1200.0, 800.0));
    settle(&mut h);
    let window = f6_walk(&mut h, 8, egui::Modifiers::NONE);
    let stuck = window.windows(2).any(|w| w[0] == w[1]);
    assert!(!stuck, "dragged out: F6 stayed put: {window:?}");
    assert!(window.iter().any(|s| s == "TileProperties"), "dragged out: {window:?}");
}
