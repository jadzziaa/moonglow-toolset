//! The area editor's header (M4's U3; the Main, Narrow and WideArea
//! boards): every control reachable at every width, the yields at the
//! editor widths the proposal names (section 5.2), the wide row's words
//! (section 5.3), the Overlays switch, and Escape closing a popover alone.

use crate::common::*;
use mg_uikit::header::{self, Place};

/// The names `Settings::hidden_area_tools` hides the header's controls
/// by (`icons::AREA_TOOLS`), but the Show toggles' ("Show Creatures").
const AREA_TOOLS: [&str; 29] = [
    "Start",
    "All",
    "None",
    "Night",
    "Lighting",
    "Shadows",
    "Fog",
    "Fade Geometry",
    "Animations",
    "Grass",
    "Tile Border",
    "Shaders",
    "Grid",
    "Minimap",
    "Sounds",
    "Sound Ranges",
    "Ambient",
    "Music",
    "Snap",
    "Turn",
    "Together",
    "Onto Objects",
    "Walkmesh…",
    "Select Tiles",
    "Paint",
    "Area Properties",
    "Reorient Camera",
    "Go to Start Location",
    "To Scratch",
];

/// The header's record: where each item went.
fn places(h: &Harness<'_, Moonglow>) -> Vec<(String, Place)> {
    header::places(&h.ctx, AREA_HEADER)
}

fn place(h: &Harness<'_, Moonglow>, name: &str) -> Option<Place> {
    places(h).iter().find(|(n, _)| n == name).map(|(_, p)| *p)
}

/// The header row's width (the editor's, `Yields`' E).
fn row_width(h: &Harness<'_, Moonglow>) -> f32 {
    header::records(&h.ctx, AREA_HEADER)[0].row.width()
}

/// The window narrowed or widened until the header's row is `width`
/// wide (the docks fold and narrow by the window's width: a few tries).
fn row_at(h: &mut Harness<'_, Moonglow>, width: f32) {
    for _ in 0..8 {
        settle(h);
        let off = width - row_width(h);
        if off.abs() < 0.01 {
            return;
        }
        let window = h.ctx.content_rect();
        h.set_size(egui::vec2(window.width() + off, window.height()));
    }
    panic!("the header's row never {width} wide: {}", row_width(h));
}

/// What the header holds, by the names its record gives them at this
/// width: the full header's Show toggles one by one (a joined group has
/// no node of its own) and its arrow, or the compact header's Show
/// popover; the rest by their items' names. While Paint is on, its
/// options in place of the Select tool's, as its mode has them (Radius
/// in Scatter and Erase, Density in Scatter), the Show popover in More;
/// while Select Tiles is, its commands (the AreaTiles board), the Show
/// popover in More.
fn held_names(h: &Harness<'_, Moonglow>) -> Vec<String> {
    let right = ["Night", "Lighting", "Overlays", "Audio", "Area Properties", "To Scratch"]
        .map(String::from);
    if !h.state().paint && h.state().area_views.values().any(|v| v.tile_mode) {
        let names = TILES_COMMANDS.iter().chain(&["Show"]).map(|n| n.to_string());
        return names.chain(right).collect();
    }
    if h.state().paint {
        let p = h.state().settings.paint;
        let mut names = vec!["How the brush paints".to_string()];
        if !p.by_drag() {
            names.push("Radius".into());
        }
        if !p.by_drag() && !p.erase {
            names.push("Density".into());
        }
        names.extend(["Brush", "Show"].map(String::from));
        return names.into_iter().chain(right).collect();
    }
    let full = place(h, "Show All or None").is_some();
    let kinds: Vec<String> = mg_area::ObjectKind::ALL
        .iter()
        .map(|k| format!("Show {}", k.label()))
        .chain(["Show Start".to_string(), "Show All or None".to_string()])
        .collect();
    let show = if full { kinds } else { vec!["Show".to_string()] };
    let rest = ["Snap", "Turn", "Together", "Onto Objects"];
    show.into_iter().chain(rest.iter().map(|n| n.to_string())).chain(right).collect()
}

/// The controls inside the popovers, menus and held menus: each found by
/// [`area_tool`] (which opens what holds it) at the widths it is tried at.
const IN_POPOVERS: [&str; 22] = [
    "Show Creatures",
    "Show Start",
    "All",
    "None",
    "Use the area's lighting",
    "Sun and moon shadows",
    "Creature shadows",
    "Light shadows",
    "High",
    "Fog",
    "Fade Geometry",
    "Animations",
    "Grass",
    "Tile border",
    "Shaders",
    "Show overlays",
    "Grid",
    "Minimap",
    "Ground walkmesh",
    "Sound ranges",
    "Play Sounds",
    "Music",
];

/// Every control that was on the area's toolbar is the header's, on its
/// row, as an icon or in More (`header_holds`: drawn inside the row,
/// clear of More, or inside More's popover, scrolled to) at every window
/// width from 420 to 1600 and every 40 to 3000 (an unfolded popover's
/// parts each, Shadows, Fade Geometry, Walkmesh and every row, inside the
/// row and clear of each other), and at 420 with the Left Dock unfolded
/// beside it, in both modes (the area is docked in both); and the
/// controls of its popovers are found in what holds them at the widths
/// where the header changes its form, and the widest.
const HEADER_BREAKPOINTS: &[usize] = &[
    420, 489, 490, 549, 550, 660, 795, 796, 961, 962, 989, 990, 1160, 1294, 1295, 1499, 1500, 1600,
    1850, 2200, 2600, 3000,
];

#[test]
fn the_area_header_holds_every_control_from_420_to_1600() {
    for mode in MODES {
        let Some((mut h, _)) = area_ui_harness(&mode.dir("header-holds")) else { return };
        h.state_mut().settings.open_editors = mode.settings().open_editors;
        // (And on to 3000, where Lighting, Overlays and Audio
        // unfold into the row and fold back: their parts each checked.)
        for &width in HEADER_BREAKPOINTS {
            h.set_size(egui::vec2(width as f32, 800.0));
            settle(&mut h);
            let names = held_names(&h);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            header_holds(&mut h, AREA_HEADER, &names);
            if [420, 990, 1295, 3000].contains(&width) {
                for name in IN_POPOVERS {
                    area_tool(&mut h, name);
                    close_area_popover(&mut h);
                }
            }
        }
        // (Narrower still: at the window's least, the Left Dock unfolded by
        // hand beside the editor, everything but the end in More.)
        h.set_size(egui::vec2(420.0, 800.0));
        unfold_left_dock(&mut h);
        settle(&mut h);
        let names = held_names(&h);
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        header_holds(&mut h, AREA_HEADER, &names);
    }
}

/// Paint's options in the header's left group (the AreaPaint board),
/// each mode's (Scatter: the mode, Radius, Density, Brush; Row and Fill:
/// the mode and Brush; Erase: Radius too) on the row, as an icon or in
/// More at every window width from 420 to 1600 (and 989, 990) and every
/// 40 to 3000, in both
/// modes; the Show popover in More (Start, All and None kept); and the
/// Brush popover's controls, each mode's, found where they are at the
/// widest and the narrowest. No control of today's strip is lost.
#[test]
fn paint_s_options_are_held_at_every_width_in_each_mode() {
    use mg_ui::settings::PaintSettings;
    let modes: [(&str, PaintSettings, &[&str]); 4] = [
        (
            "Scatter",
            PaintSettings::default(),
            &[
                "Count all",
                "Spacing",
                "By size",
                "Flow",
                "Turn spread",
                "Scale",
                "Scale to",
                "Height",
                "Lock",
            ],
        ),
        ("Row", PaintSettings { row: true, ..Default::default() }, &["Gap", "Height", "Lock"]),
        ("Fill", PaintSettings { fill: true, ..Default::default() }, &["Gap", "Height", "Lock"]),
        ("Erase", PaintSettings { erase: true, ..Default::default() }, &["Snap", "Turn"]),
    ];
    for mode in MODES {
        let Some((mut h, _)) = area_ui_harness(&mode.dir("header-paint")) else { return };
        h.state_mut().settings.open_editors = mode.settings().open_editors;
        h.state_mut().paint = true;
        for (name, settings, rows) in &modes {
            h.state_mut().settings.paint = *settings;
            for &width in HEADER_BREAKPOINTS {
                h.set_size(egui::vec2(width as f32, 800.0));
                settle(&mut h);
                // (A segmented control come onto the row this frame has
                // its group's place said the frame after: egui's scope.)
                h.run_steps(1);
                let names = held_names(&h);
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                header_holds(&mut h, AREA_HEADER, &names);
                if [420, 1600, 3000].contains(&width) {
                    for row in rows.iter().chain(&["Show Start", "All", "None"]) {
                        area_tool(&mut h, row);
                        close_area_popover(&mut h);
                    }
                }
            }
            assert_eq!(h.state().settings.paint, *settings, "{mode:?} {name}: as it was");
        }
    }
}

/// Select Tiles' commands in the header's left group (the AreaTiles
/// board), by the names a screen reader and the tests find them by.
const TILES_COMMANDS: [&str; 9] = [
    "Refine Tile",
    "Previous Tile Appearance",
    "Next Tile Appearance",
    "Rotate Tile",
    "Cut Tiles",
    "Copy Tiles",
    "Paste Tiles",
    "Delete Tiles",
    "Tile Properties",
];

/// Select Tiles' commands in the header's left group (the AreaTiles
/// board) in place of the Select tool's options: each on the row, as an
/// icon or in More at every window width from 420 to 1600 (and 989, 990)
/// and every 40 to 3000, in both modes, with tiles selected and with
/// none (grayed then, but Tile Properties); the Show popover in More
/// (Start, All and None kept). No control of today's tile menu, keys or
/// Tile Properties window is lost (the panel's: `tile_properties.rs`).
#[test]
fn select_tiles_commands_are_held_at_every_width() {
    for mode in MODES {
        let Some((mut h, area)) = area_ui_harness(&mode.dir("header-tiles")) else { return };
        h.state_mut().settings.open_editors = mode.settings().open_editors;
        area_click(&mut h, "Select Tiles");
        for selected in [vec![(1, 1), (2, 1)], Vec::new()] {
            h.state_mut().area_views.get_mut(&area).unwrap().tile_selection = selected.clone();
            for &width in HEADER_BREAKPOINTS {
                h.set_size(egui::vec2(width as f32, 800.0));
                settle(&mut h);
                h.run_steps(1);
                let names = held_names(&h);
                assert!(names.iter().any(|n| n == "Refine Tile"), "{mode:?}: the Tiles state");
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                header_holds(&mut h, AREA_HEADER, &names);
                if [420, 1600, 3000].contains(&width) {
                    for name in ["Refine Tile", "Delete Tiles", "Show Start", "All", "None"] {
                        let node = area_tool(&mut h, name);
                        // (What acts on the tiles selected is grayed with
                        // none.)
                        let grayed = node.accesskit_node().is_disabled();
                        let acts = ["Refine Tile", "Delete Tiles"].contains(&name);
                        assert_eq!(grayed, acts && selected.is_empty(), "{mode:?} {width} {name}");
                        close_area_popover(&mut h);
                    }
                }
            }
            assert_eq!(h.state().area_views[&area].tile_selection, selected, "{mode:?}");
        }
    }
}

/// The same in the area's own floating window (its tab dragged out of the
/// editor area): the header yields by the window's width, every 40 from
/// 420 to 1600, in Scatter and Row, Paint's options and the Select tool's
/// each held.
#[test]
fn the_header_holds_its_controls_in_a_floating_area() {
    use mg_ui::settings::PaintSettings;
    let Some((mut h, area)) = area_ui_harness("header-floating") else { return };
    h.set_size(egui::vec2(1700.0, 900.0));
    settle(&mut h);
    let tab = mg_ui::Tab::Area(area);
    let mut moved = false;
    for paint in [
        None,
        Some(PaintSettings::default()),
        Some(PaintSettings { row: true, ..Default::default() }),
    ] {
        h.state_mut().paint = paint.is_some();
        if let Some(p) = paint {
            h.state_mut().settings.paint = p;
        }
        for &width in &[420, 489, 490, 549, 550, 760, 989, 990, 1294, 1295, 1600] {
            let size = egui::vec2(width as f32, 700.0);
            if moved {
                let dock = &mut h.state_mut().dock;
                let surface = dock.find_tab(&tab).expect("the area's tab").surface;
                dock.get_window_state_mut(surface).unwrap().set_size(size);
                h.run_steps(4);
            } else {
                own_window(&mut h, &tab, egui::pos2(20.0, 40.0), size);
                moved = true;
            }
            settle(&mut h);
            h.run_steps(1);
            let row = header::records(&h.ctx, AREA_HEADER)[0].row;
            assert!(row.width() < width as f32, "{width}: the window's header: {row:?}");
            let names = held_names(&h);
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            header_holds(&mut h, AREA_HEADER, &names);
        }
    }
}

/// The yields of section 5.2 at the editor widths that change them: the
/// header compact under 990 (the Show popover with its count, the moon,
/// the popovers and Area Properties as icons), Snap and Turn without
/// their value text under 550, Together and Onto Objects in More under
/// 490, each whatever room the row has (the others hidden for the last
/// two, so that the row has room for them).
#[test]
fn the_area_header_yields_at_990_550_and_490() {
    let Some((mut h, _)) = area_ui_harness("header-yields") else { return };
    row_at(&mut h, 990.0);
    assert_eq!(place(&h, "Show All or None"), Some(Place::Row), "990: {:?}", places(&h));
    assert!(place(&h, "Show Creatures").is_some(), "990: the toggles on the row");
    assert!(h.get_by_label(AREA_HEADER).query_by_label("Day").is_some(), "990: Day | Night");
    for name in ["Lighting", "Overlays", "Audio"] {
        // (With its words, or unfolded where the row has room: Audio.)
        let p = place(&h, name);
        assert!(
            matches!(p, Some(Place::Row | Place::Unfolded)),
            "990: {name} with its words: {p:?}"
        );
    }
    row_at(&mut h, 989.0);
    assert_eq!(place(&h, "Show All or None"), None, "989: {:?}", places(&h));
    assert!(place(&h, "Show Creatures").is_none(), "989: no toggles on the row");
    let show = h.get_by_label(AREA_HEADER).get_by_label("Show").rect();
    assert!(show.width() > 60.0, "989: Show with its count: {show:?}");
    // (Its count drawn after its words: the moves file's "5 of 10".)
    let words = crate::shell::drawn_texts(&h, show);
    // (Its words and count one line: "Show10 of 10".)
    let counted = |w: &String| {
        w.starts_with("Show")
            && w.strip_suffix(" of 10").is_some_and(|n| n.ends_with(|c: char| c.is_ascii_digit()))
    };
    assert!(words.iter().any(counted), "989: Show's \"N of 10\" drawn: {words:?}");
    assert!(h.get_by_label(AREA_HEADER).query_by_label("Day").is_none(), "989: the moon alone");
    let night = h.get_by_label(AREA_HEADER).get_by_label("Night");
    assert!(night.rect().width() <= mg_uikit::metrics::ICON_BUTTON + 0.5, "989: the moon");
    for name in ["Lighting", "Overlays", "Audio", "Area Properties"] {
        assert_eq!(place(&h, name), Some(Place::Icon), "989: {name} an icon");
    }

    // Snap and Turn: their value text (the rest hidden: the row has room).
    let all: Vec<String> = AREA_TOOLS.iter().map(|n| n.to_string()).collect();
    let but = |keep: &[&str]| -> Vec<String> {
        let kinds = mg_area::ObjectKind::ALL.iter().map(|k| format!("Show {}", k.label()));
        let names = all.iter().cloned().chain(kinds);
        names.filter(|n| !keep.contains(&n.as_str())).collect()
    };
    h.state_mut().settings.hidden_area_tools = but(&["Snap", "Turn"]);
    row_at(&mut h, 550.0);
    for name in ["Snap", "Turn"] {
        let w = h.get_by_label(AREA_HEADER).get_by_label(name).rect().width();
        assert!(w > 40.0, "550: {name} with its value: {w}");
    }
    row_at(&mut h, 549.0);
    for name in ["Snap", "Turn"] {
        let w = h.get_by_label(AREA_HEADER).get_by_label(name).rect().width();
        assert!(w <= 36.5, "549: {name} without its value: {w}");
    }
    h.state_mut().settings.hidden_area_tools = but(&["Together", "Onto Objects"]);
    row_at(&mut h, 490.0);
    for name in ["Together", "Onto Objects"] {
        assert_eq!(place(&h, name), Some(Place::Row), "490: {name} on the row (the joined group)");
    }
    row_at(&mut h, 489.0);
    for name in ["Together", "Onto Objects"] {
        assert_eq!(place(&h, name), Some(Place::More), "489: {name} in More");
    }
    header_holds(&mut h, AREA_HEADER, &["Together", "Onto Objects"]);
}

/// The Overlays switch off hides its five overlays from the view, each
/// kept as it was ticked (its row grayed); on again, they are back as
/// they were.
#[test]
fn the_overlays_switch_hides_its_five_and_gives_them_back() {
    let Some((mut h, area)) = area_harness("header-overlays") else { return };
    {
        let v = h.state_mut().area_views.get_mut(&area).unwrap();
        (v.grid, v.minimap, v.walkmesh, v.object_walkmesh, v.sound_ranges) =
            (true, true, false, true, true);
    }
    settle(&mut h);
    let drawn = |h: &Harness<'_, Moonglow>| {
        let v = &h.state().area_views[&area];
        [v.grid, v.minimap, v.walkmesh, v.object_walkmesh, v.sound_ranges].map(|on| v.overlay(on))
    };
    assert_eq!(drawn(&h), [true, true, false, true, true]);
    assert!(h.state().area_views[&area].map_rect.is_some(), "the minimap drawn");
    area_click(&mut h, "Show overlays");
    settle(&mut h);
    assert_eq!(drawn(&h), [false; 5], "none drawn");
    assert!(h.state().area_views[&area].map_rect.is_none(), "the minimap gone");
    let v = &h.state().area_views[&area];
    assert_eq!(
        [v.grid, v.minimap, v.walkmesh, v.object_walkmesh, v.sound_ranges],
        [true, true, false, true, true],
        "each kept"
    );
    // (Its rows grayed, ticked as they were.)
    let grid = area_tool(&mut h, "Grid").accesskit_node();
    assert!(grid.is_disabled(), "Grid grayed");
    assert_eq!(grid.toggled(), Some(egui::accesskit::Toggled::True));
    close_area_popover(&mut h);
    area_click(&mut h, "Show overlays");
    settle(&mut h);
    assert_eq!(drawn(&h), [true, true, false, true, true], "back as they were");
    assert!(h.state().area_views[&area].map_rect.is_some(), "the minimap again");
}

/// The wide row (section 5.3, WideArea): the three popovers unfolded into
/// it; Day, Night, Lighting, Shadows, Fog, Grid, Minimap, Walkmesh and
/// Sound ranges with their words, Fade Geometry a combo box, and
/// Animations, Grass, Tile border, Shaders and the three Audio switches
/// icon toggles.
#[test]
fn the_wide_row_unfolds_the_popovers_with_their_words() {
    let Some((mut h, _)) = area_ui_harness("header-wide") else { return };
    h.set_size(egui::vec2(3000.0, 800.0));
    settle(&mut h);
    for name in ["Lighting", "Overlays", "Audio"] {
        assert_eq!(place(&h, name), Some(Place::Unfolded), "{name}: {:?}", places(&h));
    }
    let width = |h: &Harness<'_, Moonglow>, name: &str| {
        h.get_by_label(AREA_HEADER).get_by_label(name).rect().width()
    };
    let icon = mg_uikit::metrics::ICON_BUTTON + 0.5;
    for name in [
        "Day",
        "Night",
        "Use the area's lighting",
        "Shadows",
        "Fog",
        "Grid",
        "Minimap",
        "Walkmesh",
        "Sound ranges",
    ] {
        assert!(width(&h, name) > icon + 10.0, "{name} with its words: {}", width(&h, name));
    }
    for name in ["Animations", "Grass", "Tile border", "Shaders", "Play Sounds", "Ambient", "Music"]
    {
        assert!(width(&h, name) <= icon, "{name} an icon: {}", width(&h, name));
    }
    let fade = h.get_by_label(AREA_HEADER).get_by_label("Fade Geometry");
    assert_eq!(fade.accesskit_node().role(), egui::accesskit::Role::ComboBox);
    // (Its words the switch's: the area's lighting, named for what it does.)
    h.get_by_label(AREA_HEADER).get_by_label("Use the area's lighting").hover();
    let lighting = h.get_by_label(AREA_HEADER).get_by_label("Use the area's lighting");
    assert_eq!(lighting.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
}

/// The Overlays popover's rows have no icons (the Main board; M5's W2,
/// ruling 11): Grid, Minimap and Sound ranges paint what the walkmesh
/// rows paint, a box and their words; in the wide row their toggles keep
/// their icons beside their words, as WideArea draws them.
#[test]
fn the_overlays_rows_have_no_icons_and_their_toggles_keep_theirs() {
    let Some((mut h, area)) = area_harness("header-overlays-icons") else { return };
    // (Each unticked, so that each row's box paints alike.)
    {
        let v = h.state_mut().area_views.get_mut(&area).unwrap();
        (v.grid, v.minimap, v.walkmesh, v.object_walkmesh, v.sound_ranges) =
            (false, false, false, false, false);
    }
    settle(&mut h);
    // (What a row paints but its words, in the popover.)
    let painted = |h: &mut Harness<'_, Moonglow>, name: &str| {
        let row = area_tool(h, name).rect();
        (h.output().shapes.iter())
            .filter(|c| !matches!(c.shape, egui::Shape::Text(_)))
            .filter(|c| row.contains_rect(c.shape.visual_bounding_rect()))
            .count()
    };
    let walkmesh = painted(&mut h, "Ground walkmesh");
    for name in ["Grid", "Minimap", "Sound ranges", "Object walkmeshes"] {
        assert_eq!(painted(&mut h, name), walkmesh, "{name} paints no icon");
    }
    close_area_popover(&mut h);
    h.set_size(egui::vec2(3000.0, 800.0));
    settle(&mut h);
    assert_eq!(place(&h, "Overlays"), Some(Place::Unfolded), "{:?}", places(&h));
    let font = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
    for (name, words) in
        [("Grid", "Grid"), ("Minimap", "Minimap"), ("Sound ranges", "Sound ranges")]
    {
        let toggle = h.get_by_label(AREA_HEADER).get_by_label(name).rect().width();
        let text = (h.ctx)
            .fonts_mut(|f| f.layout_no_wrap(words.into(), font.clone(), egui::Color32::WHITE))
            .size()
            .x;
        let room = toggle - text;
        assert!(room > 2.0 * mg_uikit::metrics::ICON, "{name}: its icon beside its words: {room}");
    }
}

/// An Escape that closes a popover of the header's closes nothing else
/// (the kit's popovers are not egui's popups, which the app's Escape rule
/// asks about): the selection in the view under the pointer stays
/// selected (and after a menu's Escape too), and a window with Cancel
/// over the area (Area Properties) stays open; a second Escape does what
/// it did before.
#[test]
fn an_escape_that_closes_a_header_popover_closes_nothing_else() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("header-escape") else { return };
    settle(&mut h);
    let selected = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].selection.len();
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    let view = h.state().area_views[&area].rect;
    h.hover_at(view.center());
    settle(&mut h);
    area_tool(&mut h, "Fog");
    h.hover_at(view.center());
    h.run_steps(2);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    let open = header::records(&h.ctx, AREA_HEADER)[0].open.clone();
    assert_eq!(open, None, "the popover closed");
    assert_eq!(selected(&h), 1, "the selection kept");
    // A menu's Escape, the same (the menu closed before the view reads
    // the key).
    h.get_by_label("View").click();
    h.run_steps(2);
    h.hover_at(view.center());
    h.run_steps(2);
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(!egui::Popup::is_any_open(&h.ctx), "the menu closed");
    assert_eq!(selected(&h), 1, "the selection kept after the menu's Escape");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert_eq!(selected(&h), 0, "a second Escape: nothing selected");

    // A window with Cancel, clicked in last (the window wide enough for
    // the header's More to be clear of it).
    h.set_size(egui::vec2(1800.0, 800.0));
    settle(&mut h);
    let tab = Tab::AreaProperties(area);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(&mut h);
    assert!(h.state().dock.find_tab(&tab).is_some());
    let advanced = h.get_by_label("Advanced").rect().center();
    h.hover_at(advanced);
    h.run_steps(2);
    h.get_by_label("Advanced").click();
    settle(&mut h);
    // (More opened by the pointer, a click of its own: the window over
    // the area covers the popovers' buttons.)
    let more = h.get_by_label(AREA_HEADER).get_by_label(header::MORE).rect().center();
    h.hover_at(more);
    h.run_steps(2);
    press(&h, more, true, egui::Modifiers::NONE);
    press(&h, more, false, egui::Modifiers::NONE);
    h.run_steps(3);
    let open = header::records(&h.ctx, AREA_HEADER)[0].open.clone();
    assert_eq!(open.as_deref(), Some(header::MORE), "More open");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    let open = header::records(&h.ctx, AREA_HEADER)[0].open.clone();
    assert_eq!(open, None, "More closed");
    assert!(h.state().dock.find_tab(&tab).is_some(), "Area Properties stays open");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&tab).is_none(), "a second Escape: cancelled");
}

/// What could not be read for the view (Part 2, decision 8): a warning
/// sign with its count at the right group's end, its tip saying what; in
/// More where the row is too narrow; none while all was read.
#[test]
fn the_warning_says_what_could_not_be_read() {
    let Some((mut h, area)) = area_harness("header-warning") else { return };
    settle(&mut h);
    assert_eq!(place(&h, "Warnings"), None, "none while all was read");
    {
        let v = h.state_mut().area_views.get_mut(&area).unwrap();
        let problems = &mut v.model.as_mut().unwrap().problems;
        problems.push("A test's problem".into());
        problems.push("Another".into());
    }
    h.set_size(egui::vec2(1600.0, 800.0));
    settle(&mut h);
    assert_eq!(place(&h, "Warnings"), Some(Place::Row), "{:?}", places(&h));
    let sign = h.get_by_label(AREA_HEADER).get_by_label("Warnings").rect();
    assert!(
        mg_uikit::icons_drawn(&h.ctx)
            .iter()
            .any(|(i, r)| { *i == mg_uikit::Icon::Warning && sign.contains_rect(*r) })
    );
    h.hover_at(sign.center());
    let started = std::time::Instant::now();
    while h.query_by_label_contains("A test's problem").is_none() {
        assert!(started.elapsed().as_secs() < 10, "its tip says what");
        h.step();
    }
    h.get_by_label_contains("What could not be read in the area:\nA test's problem\nAnother");
    // (Narrow: in More, a line saying how many.)
    h.hover_at(egui::pos2(5.0, 700.0));
    let mut in_more = false;
    for width in (420..=560).rev().step_by(10) {
        h.set_size(egui::vec2(width as f32, 800.0));
        settle(&mut h);
        header_holds(&mut h, AREA_HEADER, &["Warnings"]);
        in_more |= place(&h, "Warnings") == Some(Place::More);
    }
    // (The editor narrower still: the Left Dock unfolded by hand beside it.)
    unfold_left_dock(&mut h);
    settle(&mut h);
    header_holds(&mut h, AREA_HEADER, &["Warnings"]);
    in_more |= place(&h, "Warnings") == Some(Place::More);
    assert!(in_more, "never in More: {:?} at {}", places(&h), row_width(&h));
}

/// A popover of the header's is not modal (U3's review, the coordinator's
/// decision): the view's keys go on acting while it is open, so the
/// Status Bar keeps the view's hints (`shell::keys_blocked` counts no
/// popover), and Delete over the view deletes what is selected.
#[test]
fn a_header_popover_in_front_keeps_the_view_s_hints_and_keys() {
    use mg_area::ObjectKind;
    let Some((mut h, area)) = area_harness("header-hints") else { return };
    h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(ObjectKind::Waypoint, 0)];
    settle(&mut h);
    let said = |h: &Harness<'_, Moonglow>, w: &str| h.state().status_said().iter().any(|s| s == w);
    assert!(said(&h, "Drag: Move") && said(&h, "1 selected"), "{:?}", h.state().status_said());
    let waypoints = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("WaypointList").map_or(0, |l| l.len())
    };
    let before = waypoints(&mut h);
    area_tool(&mut h, "Fog");
    let view = h.state().area_views[&area].rect;
    h.hover_at(egui::pos2(view.left() + 40.0, view.center().y));
    h.run_steps(3);
    assert!(header_open(&h).is_some(), "the popover open");
    assert!(said(&h, "Drag: Move"), "the hints under the popover: {:?}", h.state().status_said());
    h.key_press(egui::Key::Delete);
    h.run_steps(3);
    assert_eq!(waypoints(&mut h), before - 1, "Delete deleted the waypoint");
}

/// The area header's open popover (or More), where one is.
fn header_open(h: &Harness<'_, Moonglow>) -> Option<String> {
    header::records(&h.ctx, AREA_HEADER)[0].open.clone()
}

/// The Overlays switch is kept as Grid's neighbours Minimap and Sound
/// ranges are (U3's review): off, it is off in the views opened after,
/// saved with the settings, and a settings file from before it reads it
/// on; and while it is off, the Overlays button stays on the header with
/// all four of its rows hidden, the one way to turn it on again.
#[test]
fn the_overlays_switch_is_kept_and_stays_reachable() {
    let Some((mut h, area)) = area_harness("header-overlays-kept") else { return };
    settle(&mut h);
    area_click(&mut h, "Show overlays");
    assert!(h.state().settings.hide_overlays, "kept in the settings");
    let saved = serde_json::to_string(&h.state().settings).unwrap();
    let back: mg_ui::Settings = serde_json::from_str(&saved).unwrap();
    assert!(back.hide_overlays, "saved");
    let old: mg_ui::Settings = serde_json::from_str(r#"{"light_theme":true}"#).unwrap();
    assert!(!old.hide_overlays, "on in settings from before");
    // (A view opened after: off.)
    h.state_mut().area_views.remove(&area);
    settle(&mut h);
    assert!(h.state().area_views[&area].hide_overlays, "off in a view opened after");
    // Its four rows hidden: the button stays while the switch is off.
    h.state_mut().settings.hidden_area_tools =
        ["Grid", "Minimap", "Walkmesh…", "Sound Ranges"].map(String::from).to_vec();
    settle(&mut h);
    assert!(place(&h, "Overlays").is_some(), "Overlays kept while off: {:?}", places(&h));
    area_click(&mut h, "Show overlays");
    settle(&mut h);
    assert!(!h.state().area_views[&area].hide_overlays, "on again");
    assert_eq!(place(&h, "Overlays"), None, "and, nothing in it, gone");
}
