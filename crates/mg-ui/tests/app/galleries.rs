//! The galleries' shared cell and grid (M4's U25): the palette's Gallery
//! asks for the pictures of the cells in sight and a row ahead alone,
//! however many blueprints its palette has, and comes to rest; its cells
//! are buttons named and said selected; its enlargement stands beside the
//! dock, never over the cell it tells of, and is gone while a picture is
//! dragged; its hints are the Status Bar's while it has the keys, and not
//! otherwise; a blueprint placed from a picture is placed as from its
//! name in the List.

use crate::common::*;
use mg_module::palette::{BlueprintKind, Palette, PaletteBlueprint, PaletteName, PaletteNode};

/// How many blueprints the large palette has.
const MANY: usize = 5000;

/// A Placeable palette of one category, "Many Things", of [`MANY`]
/// blueprints there are no files of ("Thing 0000", `mg_many_0000`...):
/// their pictures are made at once, as none.
fn many() -> Palette {
    let blueprints = (0..MANY)
        .map(|i| PaletteBlueprint {
            resref: ResRef::from_str(&format!("mg_many_{i:04}")).unwrap(),
            name: PaletteName::Text(format!("Thing {i:04}").into_bytes()),
            cr: None,
            faction: None,
        })
        .collect();
    let node = PaletteNode {
        name: PaletteName::Text(b"Many Things".to_vec()),
        id: Some(0),
        children: Vec::new(),
        blueprints,
    };
    Palette { nodes: vec![node] }
}

/// The area harness with the palette's Gallery on the large palette, its
/// category open, the pictures 64 points.
fn many_harness(name: &str) -> Option<Harness<'static, Moonglow>> {
    let (mut h, _) = area_harness(name)?;
    {
        let app = h.state_mut();
        app.palette.set_standard(BlueprintKind::Placeable, many());
        app.palette.kind = BlueprintKind::Placeable;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.settings.palette_gallery = true;
        app.settings.gallery_tile = Some(64);
        app.settings.palette_open = vec!["utp/standard/0".into()];
        show_palette(app);
    }
    settle(&mut h);
    Some(h)
}

/// The large palette's cells drawn (their names, as a screen reader has
/// them), with their rectangles.
fn cells(h: &Harness<'_, Moonglow>) -> Vec<(String, egui::Rect)> {
    h.query_all_by(|n| n.label().is_some_and(|l| l.starts_with("Thing ")))
        .map(|n| (n.accesskit_node().label().unwrap_or_default(), n.rect()))
        .collect()
}

/// A click of the primary button at `at`.
fn click_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    h.step();
    press(h, at, true, egui::Modifiers::NONE);
    h.step();
    press(h, at, false, egui::Modifiers::NONE);
    h.step();
}

/// How many cells a row of the grid holds: those level with the first.
fn per_row(cells: &[(String, egui::Rect)]) -> usize {
    let top = cells.iter().map(|(_, r)| r.top()).fold(f32::INFINITY, f32::min);
    cells.iter().filter(|(_, r)| (r.top() - top).abs() < 0.5).count()
}

/// The enlargements shown: the tooltip layer's visible areas (in
/// points), the whole of each, not one label in it. Not one being
/// measured: `show_beside` lays a new one out unseen at the window's top
/// left corner first (a sizing pass, which paints nothing but is in the
/// layers and the AccessKit tree), where a shown one never stands (under
/// the menu bar at least).
fn enlargements(h: &Harness<'_, Moonglow>) -> Vec<egui::Rect> {
    let corner = h.ctx.content_rect().min;
    h.ctx.memory(|m| {
        (m.layer_ids())
            .filter(|l| l.order == egui::Order::Tooltip && m.areas().is_visible(l))
            .filter_map(|l| m.area_rect(l.id))
            .filter(|r| r.min != corner)
            .collect()
    })
}

/// Hovers `at` and waits until the enlargement whose ResRef line is
/// `resref`'s is up; its whole rectangle.
fn enlarged(h: &mut Harness<'_, Moonglow>, at: egui::Pos2, resref: &str) -> egui::Rect {
    h.hover_at(at);
    let line = format!("ResRef {resref}");
    let started = std::time::Instant::now();
    while h.query_by_label(&line).is_none() || enlargements(h).is_empty() {
        assert!(started.elapsed().as_secs() < 10, "no enlargement of {resref}");
        h.step();
    }
    let tips = enlargements(h);
    assert_eq!(tips.len(), 1, "one enlargement: {tips:?}");
    tips[0]
}

/// A palette of 5,000 blueprints in one open category: a frame lays out
/// and paints the cells in sight alone (a few dozen, not 5,000) and asks
/// for the pictures of those and of one row ahead, counted at each ask;
/// scrolled to its end with the keys (End), the same, and the last cell is
/// drawn; with every picture made, the window comes to rest (no frame
/// asked for: `settle` fails on a window that keeps repainting).
#[test]
fn a_gallery_of_five_thousand_asks_for_the_pictures_in_sight_alone() {
    let Some(mut h) = many_harness("gallery-many") else { return };
    let check = |h: &mut Harness<'_, Moonglow>, at: &str| {
        h.step();
        let drawn = cells(h);
        let across = per_row(&drawn);
        assert!(!drawn.is_empty() && across > 0, "{at}: none drawn");
        assert!(drawn.len() < 200, "{at}: {} cells drawn of {MANY}", drawn.len());
        let asked = h.state().thumbnails.shown_now();
        // (Those in sight, the row ahead, a row half in sight at the
        // top: no more.)
        assert!(
            asked <= drawn.len() + 2 * across,
            "{at}: {asked} pictures asked for with {} drawn, {across} a row",
            drawn.len()
        );
        assert!(asked >= drawn.len(), "{at}: {asked} asked, {} drawn", drawn.len());
    };
    check(&mut h, "at the top");
    // The keys (a cell clicked gives the palette them), then End.
    let first = cells(&h).into_iter().find(|(n, _)| n == "Thing 0000").expect("the first cell");
    click_at(&mut h, first.1.center());
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Palette");
    h.key_press(egui::Key::End);
    settle(&mut h);
    check(&mut h, "at the end");
    let last = format!("Thing {:04}", MANY - 1);
    assert!(cells(&h).iter().any(|(n, _)| *n == last), "the last cell drawn at the end");
    let chosen = |h: &Harness<'_, Moonglow>| {
        let key = h.state().palette.selected.expect("a blueprint chosen by the keys");
        key.resref.to_string()
    };
    let index = |h: &Harness<'_, Moonglow>| -> usize {
        chosen(h).trim_start_matches("mg_many_").parse().unwrap()
    };
    assert_eq!(chosen(&h), format!("mg_many_{:04}", MANY - 1), "End chose the last");
    // Home: the first row, the category's heading, in sight; Down from it,
    // the first picture, chosen.
    h.key_press(egui::Key::Home);
    settle(&mut h);
    check(&mut h, "at the top again");
    assert!(cells(&h).iter().any(|(n, _)| n == "Thing 0000"), "the first cell drawn");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), "mg_many_0000", "Home went to the top");
    // Page Down: a page's rows on, the cell chosen in sight; Page Up back.
    let across = per_row(&cells(&h));
    // (A page: the list's height in sight, from its heading to the dock's
    // foot, in rows of pictures.)
    let rect_of = |h: &Harness<'_, Moonglow>, n: &str| h.get_by_label(n).rect();
    let pitch = rect_of(&h, &format!("Thing {across:04}")).top() - rect_of(&h, "Thing 0000").top();
    let page = right_dock_rect(&h).bottom() - rect_of(&h, "Many Things (5000)").top();
    h.key_press(egui::Key::PageDown);
    settle(&mut h);
    check(&mut h, "a page down");
    let paged = index(&h);
    assert!(paged >= 2 * across && paged % across == 0, "a page of rows on: {paged}");
    let rows = (paged / across) as f32;
    assert!((rows - page / pitch).abs() <= 1.0, "{rows} rows a page of {page} by {pitch}");
    let name = format!("Thing {paged:04}");
    assert!(cells(&h).iter().any(|(n, _)| *n == name), "{name} drawn");
    // Page Up: back a page, to the heading (the page reaches it: Page Down
    // went from the first picture); Down from it, the first picture.
    h.key_press(egui::Key::PageUp);
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(index(&h), 0, "Page Up went back a page, to the heading");
    assert!(cells(&h).iter().any(|(n, _)| n == "Thing 0000"), "Page Up to the top");
    // A page is the same rows each time (twice down: twice as far, as as
    // many Downs go), and Page Up from the end a page back, not Home.
    for _ in 0..2 {
        h.key_press(egui::Key::PageDown);
        settle(&mut h);
    }
    assert_eq!(index(&h), 2 * paged, "two pages down");
    h.key_press(egui::Key::PageUp);
    settle(&mut h);
    h.key_press(egui::Key::Home);
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    for _ in 0..paged / across {
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
    }
    assert_eq!(index(&h), paged, "a page down is as many Downs as its rows");
    h.key_press(egui::Key::End);
    settle(&mut h);
    h.key_press(egui::Key::PageUp);
    settle(&mut h);
    let back = MANY - 1 - index(&h);
    assert!(back + across >= paged && back <= paged + across, "Page Up from the end: {back} back");
    // A fast wheel: many notches a frame, each frame asking for no more.
    let at = cells(&h)[0].1.center();
    h.hover_at(at);
    for _ in 0..12 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, -3.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        check(&mut h, "a fast wheel");
    }
    settle(&mut h);
    check(&mut h, "after the wheel");
    assert!(!cells(&h).iter().any(|(n, _)| n == "Thing 0000"), "the wheel scrolled on");
    // The Scroll Bar dragged to its end: each frame asks for no more.
    h.key_press(egui::Key::Home);
    settle(&mut h);
    let dock = right_dock_rect(&h);
    let top = cells(&h)[0].1.top();
    let bar = egui::pos2(dock.right() - 5.0, top + 6.0);
    h.hover_at(bar);
    settle(&mut h);
    press(&h, bar, true, egui::Modifiers::NONE);
    check(&mut h, "the bar pressed");
    let bottom = h.ctx.content_rect().bottom() - 2.0;
    for t in [0.25, 0.5, 0.75, 1.0] {
        h.event(egui::Event::PointerMoved(egui::pos2(bar.x, bar.y + (bottom - bar.y) * t)));
        check(&mut h, "dragging the bar");
    }
    press(&h, egui::pos2(bar.x, bottom), false, egui::Modifiers::NONE);
    settle(&mut h);
    check(&mut h, "the bar at its end");
    assert!(cells(&h).iter().any(|(n, _)| *n == last), "the bar dragged to the end");
    settle(&mut h);
}

/// Each cell is a button named by its blueprint's name, said selected
/// once it is clicked (and the others not), with a click to act on.
#[test]
fn a_gallery_cell_is_a_named_button_said_selected() {
    let Some(mut h) = many_harness("gallery-cell-named") else { return };
    // The grid: a list named Pictures of the category's 5,000, each cell
    // built saying its place in it.
    let list = h.get_by_role(egui::accesskit::Role::List);
    assert_eq!(list.accesskit_node().label().as_deref(), Some("Pictures"));
    assert_eq!(list.accesskit_node().data().size_of_set(), Some(MANY));
    let node = h.get_by_label("Thing 0002");
    let data = node.accesskit_node();
    assert_eq!((data.data().position_in_set(), data.data().size_of_set()), (Some(3), Some(MANY)));
    assert!(h.query_by_label("Thing 4999").is_none(), "out of sight, not built");
    assert_eq!(node.accesskit_node().role(), egui::accesskit::Role::Button);
    assert!(node.accesskit_node().data().supports_action(egui::accesskit::Action::Click));
    assert_eq!(node.accesskit_node().data().is_selected(), Some(false));
    node.click();
    settle(&mut h);
    let key = ResKey::parse("mg_many_0002", ResType::UTP).unwrap();
    assert_eq!(h.state().palette.selected, Some(key), "chosen, to place");
    let selected = |h: &Harness<'_, Moonglow>, n: &str| {
        h.get_by_label(n).accesskit_node().data().is_selected()
    };
    assert_eq!(selected(&h, "Thing 0002"), Some(true));
    assert_eq!(selected(&h, "Thing 0001"), Some(false));
    // The keys' cell: the list's active one.
    let active =
        h.get_by_role(egui::accesskit::Role::List).accesskit_node().data().active_descendant();
    assert_eq!(active, Some(h.get_by_label("Thing 0002").accesskit_node().locate().0));
}

/// The pointer resting on a cell: its enlargement (its name and ResRef)
/// stands beside the Right Dock, left of it, never over the cell it tells
/// of nor over the dock, inside the window; a picture dragged, it is gone.
#[test]
fn the_enlargement_stands_beside_the_dock_and_goes_while_dragging() {
    let Some(mut h) = many_harness("gallery-enlarged") else { return };
    let cell = h.get_by_label("Thing 0001").rect();
    h.hover_at(cell.center());
    let tip = |h: &Harness<'_, Moonglow>| h.query_by_label("ResRef mg_many_0001").map(|n| n.rect());
    let started = std::time::Instant::now();
    while tip(&h).is_none() {
        assert!(started.elapsed().as_secs() < 10, "no enlargement");
        h.step();
    }
    // (The whole enlargement: the tooltip layer's areas, not the label of
    // one of its lines.)
    let tips: Vec<egui::Rect> = h.ctx.memory(|m| {
        (m.layer_ids())
            .filter(|l| l.order == egui::Order::Tooltip && m.areas().is_visible(l))
            .filter_map(|l| m.area_rect(l.id))
            .collect()
    });
    assert_eq!(tips.len(), 1, "one enlargement: {tips:?}");
    let at = tips[0];
    assert!(at.contains_rect(tip(&h).unwrap()), "its ResRef line in it");
    let window = h.ctx.content_rect();
    // (The Right Dock's palette pane: its cells, from the first one's left.)
    let dock_left = cells(&h).iter().map(|(_, r)| r.left()).fold(f32::INFINITY, f32::min);
    assert!(at.right() <= dock_left, "beside the dock, left of it: {at:?}, dock at {dock_left}");
    assert!(!at.intersects(cell), "over its cell: {at:?}, {cell:?}");
    assert!(window.contains(at.left_top()) && at.top() >= window.top(), "{at:?} in {window:?}");
    // Dragged: gone.
    press(&h, cell.center(), true, egui::Modifiers::NONE);
    h.step();
    for t in [0.2, 0.5] {
        h.event(egui::Event::PointerMoved(cell.center() - egui::vec2(300.0 * t, 0.0)));
        h.step();
    }
    h.step();
    assert!(tip(&h).is_none(), "the enlargement stays while dragging");
    press(&h, cell.center() - egui::vec2(150.0, 0.0), false, egui::Modifiers::NONE);
    settle(&mut h);
}

/// The Status Bar's hints while the palette's Gallery has the keys with
/// no blueprint held to place (its heading clicked, folded and opened
/// again): the GalleryPalette board's, Alt + Right Drag Turn only while
/// the area's selection turns (a waypoint, here); none of them in the
/// List, with another list having the keys, or with a menu open.
#[test]
fn the_gallery_s_hints_while_it_has_the_keys_and_not_otherwise() {
    let Some(mut h) = many_harness("gallery-hints") else { return };
    let turn = "Alt, Right Drag: Turn";
    let gallery = [
        "Drag: Place",
        "Up or Down: Row of pictures",
        "Left or Right: One picture",
        "Click: Select",
        "Ctrl, Drag: Move view",
    ];
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said().to_vec();
    let has_none = |h: &Harness<'_, Moonglow>, at: &str| {
        let now = said(h);
        for g in ["Up or Down: Row of pictures", "Left or Right: One picture"] {
            assert!(!now.iter().any(|s| s == g), "{at}: {g} said: {now:?}");
        }
    };
    // Not before the Gallery has the keys.
    has_none(&h, "the area with the keys");
    for _ in 0..2 {
        h.get_by_label("Many Things (5000)").click();
        settle(&mut h);
    }
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Palette");
    assert_eq!(h.state().palette.selected, None, "nothing held");
    assert!(h.query_by_label("Thing 0001").is_some(), "the category open again");
    let now = said(&h);
    for g in gallery {
        assert!(now.iter().any(|s| s == g), "{g} not said: {now:?}");
    }
    assert!(!now.iter().any(|s| s == turn), "nothing selected turns: {now:?}");
    // A waypoint selected in the area: it turns.
    let area = *h.state().area_views.keys().next().unwrap();
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    view.selection = vec![(mg_area::ObjectKind::Waypoint, 0)];
    settle(&mut h);
    assert!(said(&h).iter().any(|s| s == turn), "{:?}", said(&h));
    // A menu open (a cell's right-click menu): none.
    let at = h.get_by_label("Thing 0002").rect().center();
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    settle(&mut h);
    assert!(egui::Popup::is_any_open(&h.ctx), "the menu open");
    has_none(&h, "a menu open");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // The List: none.
    h.get_by_label("Thing 0001").click();
    settle(&mut h);
    h.state_mut().settings.palette_gallery = false;
    settle(&mut h);
    has_none(&h, "the List");
    // Another list with the keys (the module tree's): none.
    h.state_mut().settings.palette_gallery = true;
    settle(&mut h);
    h.get_by_label("Thing 0001").click();
    settle(&mut h);
    assert!(said(&h).iter().any(|s| s == "Up or Down: Row of pictures"));
    wide_window(&mut h);
    settle(&mut h);
    h.get_by_label("Module Tree Rows").get_by_label("Areas (1)").click();
    settle(&mut h);
    assert_ne!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Palette");
    has_none(&h, "the module tree with the keys");
}

/// A blueprint chosen to place from the Gallery (its picture clicked):
/// the Status Bar's hints are placing's ("Placing …", Click Place,
/// Shift and Click Place more, its turn), with the Gallery's arrows while
/// it has the keys, and never the Gallery's Click Select nor Drag Place.
#[test]
fn placing_s_hints_win_over_the_gallery_s() {
    let Some(mut h) = many_harness("gallery-placing-hints") else { return };
    h.get_by_label("Thing 0001").click();
    settle(&mut h);
    let key = ResKey::parse("mg_many_0001", ResType::UTP).unwrap();
    assert_eq!(h.state().palette.selected, Some(key), "held to place");
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Palette");
    let now = h.state().status_said().to_vec();
    for g in [
        "Placing mg_many_0001",
        "Click: Place",
        "Shift, Click: Place more",
        "Up or Down: Row of pictures",
        "Left or Right: One picture",
    ] {
        assert!(now.iter().any(|s| s == g), "{g} not said: {now:?}");
    }
    assert!(now.iter().any(|s| s.ends_with(": Turn")), "placing's turn not said: {now:?}");
    for g in ["Click: Select", "Drag: Place"] {
        assert!(!now.iter().any(|s| s == g), "{g} said while placing: {now:?}");
    }
    // Its keys taken away (the module tree's): placing's hints alone.
    wide_window(&mut h);
    settle(&mut h);
    h.get_by_label("Module Tree Rows").get_by_label("Areas (1)").click();
    settle(&mut h);
    let now = h.state().status_said().to_vec();
    assert_eq!(h.state().palette.selected, Some(key), "still held");
    assert!(now.iter().any(|s| s == "Click: Place"), "{now:?}");
    assert!(!now.iter().any(|s| s == "Up or Down: Row of pictures"), "{now:?}");
}

/// A placeable dragged from its picture into the area is placed as one
/// dragged from its name in the List: the same Undo step, at the same
/// place (snapped to the same grid); and chosen by a click on its picture,
/// a click in the area places it so too.
#[test]
fn placing_from_the_gallery_is_placing_from_the_list() {
    let Some((mut h, area)) = area_harness("gallery-place") else { return };
    h.state_mut().settings.snap_grid = Some(100);
    let chest = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
    let name = {
        let game = h.state().game.as_deref().unwrap();
        let utp = mg_gff::Gff::read(&game.resman.get(&chest).unwrap()).unwrap();
        game.locstring(utp.root.locstring("LocName").unwrap()).unwrap()
    };
    {
        let app = h.state_mut();
        app.palette.kind = BlueprintKind::Placeable;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.palette.filter = "plc_chest1".into();
        show_palette(app);
    }
    let placed = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        let list = git.root.list("Placeable List").map(<[mg_gff::Struct]>::to_vec);
        let last = list.unwrap_or_default().last().cloned();
        let at = last.map(|s| (s.float("X").unwrap_or(0.0), s.float("Y").unwrap_or(0.0)));
        (at, ws.can_undo().map(str::to_string))
    };
    let drag = |h: &mut Harness<'_, Moonglow>| {
        settle(h);
        let from = h.get_all_by_label(&name).next().expect("the chest").rect().center();
        let to = screen(h, area, glam::Vec3::new(23.3, 26.7, 0.0));
        h.event(egui::Event::PointerMoved(from));
        h.step();
        press(h, from, true, egui::Modifiers::NONE);
        h.step();
        for t in [0.1, 0.4, 0.7, 1.0] {
            h.event(egui::Event::PointerMoved(from + (to - from) * t));
            h.step();
        }
        press(h, to, false, egui::Modifiers::NONE);
        settle(h);
        placed(h)
    };
    let undo = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::Undo);
        settle(h);
    };
    let before = placed(&mut h);
    h.state_mut().settings.palette_gallery = false;
    let listed = drag(&mut h);
    assert_ne!(listed, before, "placed from the List");
    undo(&mut h);
    h.state_mut().settings.palette_gallery = true;
    let pictured = drag(&mut h);
    assert_eq!(pictured, listed, "placed from the Gallery as from the List");
    undo(&mut h);
    // Chosen by a click on its picture, then a click in the area.
    settle(&mut h);
    h.get_all_by_label(&name).next().unwrap().click();
    settle(&mut h);
    assert_eq!(h.state().palette.selected, Some(chest));
    let to = screen(&h, area, glam::Vec3::new(23.3, 26.7, 0.0));
    h.hover_at(to);
    h.step();
    press(&h, to, true, egui::Modifiers::NONE);
    h.step();
    press(&h, to, false, egui::Modifiers::NONE);
    settle(&mut h);
    let clicked = placed(&mut h);
    assert_eq!(clicked.0, listed.0, "placed where the List's drag placed it");
}

/// The palette's toolbar with the Gallery on (its size slider shown) is
/// never taller where the dock is wider: from the dock's least width to
/// the wide layout's, at the three zooms, the room above the first
/// heading (from the List button's top) never grows as the dock widens;
/// and the slider and every cell stay inside the dock.
#[test]
fn the_gallery_s_toolbar_is_never_taller_where_wider() {
    let Some(mut h) = many_harness("gallery-widths") else { return };
    wide_window(&mut h);
    for scale in [100, 125, 150] {
        h.state_mut().settings.ui_scale = (scale != 100).then_some(scale);
        let mut was: Option<(f32, f32)> = None;
        for width in (280..=741).step_by(39).chain(std::iter::once(741)) {
            right_dock_width(&mut h, width as f32);
            settle(&mut h);
            // (kittest gives AccessKit's boxes in points, the zoom's.)
            let list = h.get_by_label("List").rect();
            let heading = h.get_by_label("Many Things (5000)").rect();
            let above = heading.top() - list.top();
            if let Some((narrower, at)) = was {
                assert!(
                    above <= narrower + 0.5,
                    "x{scale} {width}: {above} above, {narrower} at {at}"
                );
            }
            was = Some((above, width as f32));
            let dock = right_dock_rect(&h);
            let slider = h.get_by_role(egui::accesskit::Role::Slider).rect();
            let inside = |r: egui::Rect| r.right() <= dock.right() + 0.5;
            assert!(inside(slider), "x{scale} {width}: the slider {slider:?} past {dock:?}");
            for (n, r) in cells(&h) {
                assert!(inside(r), "x{scale} {width}: {n} {r:?} past the dock {dock:?}");
            }
        }
    }
}

/// The large palette's Gallery comes to rest (`settle` fails on a window
/// that keeps repainting): with every picture missing (none of its
/// blueprints has a file), after its size slider moves (by its keys),
/// with an enlargement up, and gone again.
#[test]
fn the_palette_s_gallery_comes_to_rest() {
    let Some(mut h) = many_harness("gallery-rests") else { return };
    settle(&mut h);
    let slider = h.get_by_role(egui::accesskit::Role::Slider);
    slider.focus();
    settle(&mut h);
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_ne!(h.state().settings.gallery_tile, Some(64), "the slider moved");
    let cell = h.get_by_label("Thing 0001").rect();
    h.hover_at(cell.center());
    let started = std::time::Instant::now();
    while h.query_by_label("ResRef mg_many_0001").is_none() {
        assert!(started.elapsed().as_secs() < 10, "no enlargement");
        h.step();
    }
    settle(&mut h);
    assert!(h.query_by_label("ResRef mg_many_0001").is_some(), "the enlargement up at rest");
    h.hover_at(egui::pos2(5.0, 5.0));
    settle(&mut h);
}

/// The other galleries come to rest with their pictures made: the
/// placeables' palette with the game's chests (one picture of each made),
/// the tiles' palette's Gallery, the Appearance Gallery, and an armor's
/// thighs under its Appearance page.
#[test]
fn each_gallery_comes_to_rest() {
    let Some((mut h, _)) = area_harness("gallery-rests-game") else { return };
    {
        let app = h.state_mut();
        app.palette.kind = BlueprintKind::Placeable;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.palette.filter = "plc_chest".into();
        app.settings.palette_gallery = true;
        show_palette(app);
    }
    settle(&mut h);
    assert_eq!(h.state().thumbnails.waiting(), 0, "the chests' pictures made");
    palette_type(&mut h, "Tiles");
    h.state_mut().palette.filter = "well".into();
    settle(&mut h);
    assert!(h.query_all_by_label_contains("Well").next().is_some(), "the tiles' Gallery drawn");
    let Some((mut h, _)) = blueprint_harness("plc_chest1", "rests_chest", ResType::UTP) else {
        return;
    };
    h.get_by_label("Gallery…").click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::PlaceableGallery).is_some(), "the Appearance Gallery");
    let Some((mut h, key)) = blueprint_harness("nw_aarcl001", "rests_armor", ResType::UTI) else {
        return;
    };
    h.set_size(egui::vec2(1000.0, 1200.0));
    own_window(&mut h, &Tab::Blueprint(key), egui::pos2(10.0, 10.0), egui::vec2(980.0, 1180.0));
    page(&mut h, "Appearance");
    settle(&mut h);
    h.get_by(|n| {
        n.role() == egui::accesskit::Role::Button && n.label().as_deref() == Some("Thighs")
    })
    .click();
    settle(&mut h);
}

/// A whole item's icons under its Appearance page take its base item's
/// room: a tower shield made a medium miscellaneous item (142 icons, many
/// more than the grid shows at once), its grid's rows are those of a ring
/// made one (the review's C-7: the shield's size, the first drawn, was
/// kept for the icons out of sight).
#[test]
fn a_whole_item_s_icons_take_the_room_of_its_base_item_s() {
    let row = |h: &Harness<'_, Moonglow>, key: ResKey| {
        let id = egui::Id::new(("uti-icons", key, "ModelPart1")).with("cell");
        h.ctx.data(|d| d.get_temp::<(u64, egui::Vec2)>(id)).map(|(_, cell)| cell.y)
    };
    let made_medium = |name: &str, copy: &str| {
        let (mut h, key) = blueprint_harness(name, copy, ResType::UTI)?;
        page(&mut h, "Appearance");
        let before = row(&h, key).expect("the rows before");
        h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
            "Base item",
            vec![mg_edit::Edit::SetField {
                key,
                path: mg_edit::GffPath::root(),
                label: "BaseItem".into(),
                value: Some(mg_gff::Value::Int(29)),
            }],
        )));
        settle(&mut h);
        Some((before, row(&h, key).expect("the rows after")))
    };
    let Some((tower, from_tower)) = made_medium("nw_ashto001", "c7_tower") else { return };
    let Some((ring, from_ring)) = made_medium("nw_it_mring001", "c7_ring") else { return };
    assert!(ring < from_ring && from_ring < tower, "{ring}, {from_ring}, {tower}");
    assert_eq!(from_tower, from_ring, "a medium item's rows, from a shield's ({tower})");
}

/// A whole item's icons scrolled through: in sight in their order, line by
/// line, each in a cell of one size (the owner's ruling of 26 November),
/// and every one of them reached (the lines out of sight skipped whole):
/// a medium miscellaneous item's.
#[test]
fn a_whole_item_s_icons_keep_their_places_scrolled_through() {
    let Some((mut h, key)) = blueprint_harness("nw_it_mring001", "icons_kept", ResType::UTI) else {
        return;
    };
    page(&mut h, "Appearance");
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Base item",
        vec![mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "BaseItem".into(),
            value: Some(mg_gff::Value::Int(29)),
        }],
    )));
    settle(&mut h);
    // (Each icon in sight, by number, and where it stands.)
    let icons = |h: &Harness<'_, Moonglow>| -> Vec<(u16, egui::Rect)> {
        let mut v: Vec<(u16, egui::Rect)> = h
            .query_all_by_label_contains("Appearance ")
            .filter_map(|n| {
                let label = n.accesskit_node().label()?;
                let number = label.strip_prefix("Appearance ")?.parse().ok()?;
                Some((number, n.rect()))
            })
            .collect();
        v.sort_by_key(|(n, _)| *n);
        v.dedup_by_key(|(n, _)| *n);
        v
    };
    let first = icons(&h);
    assert!(first.len() > 10, "the icons in sight: {}", first.len());
    let at = first[0].1.center();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..60 {
        let shown = icons(&h);
        // (In their order, line by line: each after the one before, on its
        // line or under it.)
        for w in shown.windows(2) {
            let ((a, ra), (b, rb)) = (w[0], w[1]);
            let next_line = rb.top() > ra.center().y;
            assert!(next_line || rb.left() > ra.right(), "icon {b} {rb:?} after {a} {ra:?}");
        }
        // (Each in a cell of one size: their centers in columns and lines
        // as evenly apart, whatever each icon's own size.)
        for axis in [|r: &egui::Rect| r.center().x, |r: &egui::Rect| r.center().y] {
            let mut at: Vec<f32> = shown.iter().map(|(_, r)| axis(r)).collect();
            at.sort_by(f32::total_cmp);
            at.dedup_by(|a, b| (*a - *b).abs() < 1.0);
            let steps: Vec<f32> = at.windows(2).map(|w| w[1] - w[0]).collect();
            assert!(steps.iter().all(|s| (s - steps[0]).abs() < 1.0), "the cells apart: {steps:?}");
        }
        seen.extend(shown.iter().map(|(n, _)| *n));
        h.hover_at(at);
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -29.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        settle(&mut h);
    }
    let all: Vec<u16> = seen.iter().copied().collect();
    let (least, most) = (all[0], *all.last().unwrap());
    assert_eq!(all.len(), all.iter().filter(|n| (least..=most).contains(n)).count());
    assert!(all.len() > 100, "every icon reached: {} seen", all.len());
}

/// A palette of a few of the game's placeables (a chest, a barrel, a
/// pyre): their Tags are in their files.
fn game_things() -> Palette {
    let blueprints = ["plc_chest1", "plc_barrel", "plc_plaguepyre"]
        .map(|r| PaletteBlueprint {
            resref: ResRef::from_str(r).unwrap(),
            name: PaletteName::Text(r.as_bytes().to_vec()),
            cr: None,
            faction: None,
        })
        .to_vec();
    let node = PaletteNode {
        name: PaletteName::Text(b"Game Things".to_vec()),
        id: Some(0),
        children: Vec::new(),
        blueprints,
    };
    Palette { nodes: vec![node] }
}

/// The enlargement's Tag line: the hovered blueprint's Tag, read from its
/// file for it alone, once (no search typed, so no palette's tags were
/// read): not again each frame it stays up, nor when the pointer comes
/// back to it; another blueprint hovered, one read more.
#[test]
fn the_enlargement_reads_the_hovered_blueprint_s_tag_once() {
    let Some((mut h, _)) = area_harness("gallery-tag") else { return };
    {
        let app = h.state_mut();
        app.palette.set_standard(BlueprintKind::Placeable, game_things());
        app.palette.kind = BlueprintKind::Placeable;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.settings.palette_gallery = true;
        app.settings.palette_open = vec!["utp/standard/0".into()];
        show_palette(app);
    }
    settle(&mut h);
    let reads = |h: &Harness<'_, Moonglow>| h.state().palette.tags_read_one_by_one;
    assert_eq!(reads(&h), 0, "none read before a hover");
    let tag = {
        let game = h.state().game.as_deref().unwrap();
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let utp = mg_gff::Gff::read(&game.resman.get(&key).unwrap()).unwrap();
        String::from_utf8_lossy(utp.root.string("Tag").unwrap()).into_owned()
    };
    let chest = h.get_by_label("plc_chest1").rect().center();
    let at = enlarged(&mut h, chest, "plc_chest1");
    let line = h.get_by_label(&format!("Tag {tag}")).rect();
    assert!(at.contains_rect(line), "the Tag line in the enlargement");
    assert_eq!(reads(&h), 1, "read once for the hovered one");
    for _ in 0..10 {
        h.step();
    }
    assert_eq!(reads(&h), 1, "not again each frame");
    let barrel = h.get_by_label("plc_barrel").rect().center();
    enlarged(&mut h, barrel, "plc_barrel");
    assert_eq!(reads(&h), 2, "another hovered: one more");
    enlarged(&mut h, chest, "plc_chest1");
    assert_eq!(reads(&h), 2, "kept for the chest");
    // A search typed: every tag read for it, the hovered one's taken from
    // those, not read again.
    h.state_mut().palette.filter = "plague".into();
    settle(&mut h);
    let tag = {
        let game = h.state().game.as_deref().unwrap();
        let key = ResKey::parse("plc_plaguepyre", ResType::UTP).unwrap();
        let utp = mg_gff::Gff::read(&game.resman.get(&key).unwrap()).unwrap();
        String::from_utf8_lossy(utp.root.string("Tag").unwrap()).into_owned()
    };
    let pyre = h.get_by_label("plc_plaguepyre").rect().center();
    let at = enlarged(&mut h, pyre, "plc_plaguepyre");
    let line = h.get_by_label(&format!("Tag {tag}")).rect();
    assert!(at.contains_rect(line), "the searched one's Tag line in the enlargement");
    assert_eq!(reads(&h), 2, "the search's tags taken, none read one by one");
    settle(&mut h);
}

/// The enlargement stands left of the Right Dock, never over it (by the
/// dock's own rectangle, its edge included), never over its cell, inside
/// the window: at 1000 by 800, 800 by 700 and 1600 by 900, at each zoom.
#[test]
fn the_enlargement_never_reaches_over_the_dock() {
    let Some(mut h) = many_harness("gallery-beside-dock") else { return };
    for (w, ht) in [(1000.0, 800.0), (800.0, 700.0), (1600.0, 900.0)] {
        for scale in [100, 125, 150] {
            h.set_size(egui::vec2(w, ht));
            h.state_mut().settings.ui_scale = (scale != 100).then_some(scale);
            h.hover_at(egui::pos2(2.0, 2.0));
            settle(&mut h);
            if h.state().right_dock_rect().is_none() {
                unfold_right_dock(&mut h);
            }
            let at = format!("{w}x{ht} x{scale}");
            let cell = h.get_by_label("Thing 0001").rect();
            let tip = enlarged(&mut h, cell.center(), "mg_many_0001");
            let dock = right_dock_rect(&h);
            assert!(tip.right() <= dock.left() + 0.01, "{at}: {tip:?} over the dock {dock:?}");
            assert!(!tip.intersects(cell), "{at}: over its cell");
            let window = h.ctx.content_rect();
            assert!(window.contains_rect(tip), "{at}: {tip:?} out of {window:?}");
        }
    }
}

/// Each frame the enlargement is up it stands clear of its cell and
/// inside the window, at its own size from its first frame, not the size
/// of the one shown before it: a short one (a blueprint with no file: its
/// name and ResRef alone) and then a tall one (the chest's, with its
/// picture) for cells on the window's lowest row, where a tall one sized
/// as the short one would reach past the window's foot.
#[test]
fn the_enlargement_is_at_its_own_size_from_its_first_frame() {
    let Some((mut h, _)) = area_harness("gallery-first-frame") else { return };
    // The short one and the tall one after `pads` blueprints with no file.
    let things = |pads: usize| {
        let one = |r: &str| PaletteBlueprint {
            resref: ResRef::from_str(r).unwrap(),
            name: PaletteName::Text(r.as_bytes().to_vec()),
            cr: None,
            faction: None,
        };
        let mut all: Vec<PaletteBlueprint> =
            (0..pads).map(|i| one(&format!("mg_pad_{i:03}"))).collect();
        all.extend([one("mg_absent"), one("plc_chest1")]);
        let node = PaletteNode {
            name: PaletteName::Text(b"Things".to_vec()),
            id: Some(0),
            children: Vec::new(),
            blueprints: all,
        };
        Palette { nodes: vec![node] }
    };
    {
        let app = h.state_mut();
        app.palette.set_standard(BlueprintKind::Placeable, things(0));
        app.palette.kind = BlueprintKind::Placeable;
        (app.palette.tiles, app.palette.custom) = (false, false);
        app.settings.palette_gallery = true;
        app.settings.gallery_tile = Some(64);
        app.settings.palette_open = vec!["utp/standard/0".into()];
        show_palette(app);
    }
    settle(&mut h);
    // (The pair at the start of the dock's lowest row wholly in sight:
    // as many rows of blueprints with no file before them.)
    let dock = right_dock_rect(&h);
    h.state_mut().palette.set_standard(BlueprintKind::Placeable, things(12));
    settle(&mut h);
    let pads: Vec<egui::Rect> = (0..12)
        .filter_map(|i| h.query_by_label(&format!("mg_pad_{i:03}")).map(|n| n.rect()))
        .collect();
    let first = pads[0];
    let across = pads.iter().filter(|r| (r.top() - first.top()).abs() < 0.5).count();
    let pitch = pads[across].top() - first.top();
    let rows = ((dock.bottom() - 4.0 - first.bottom()) / pitch).floor() as usize;
    h.state_mut().palette.set_standard(BlueprintKind::Placeable, things(rows * across));
    settle(&mut h);
    let window = h.ctx.content_rect();
    let mut sizes = Vec::new();
    for (name, resref) in [("mg_absent", "mg_absent"), ("plc_chest1", "plc_chest1")] {
        let cell = h.get_by_label(name).rect();
        assert!(cell.bottom() > dock.bottom() - pitch, "{name}: on the lowest row: {cell:?}");
        h.hover_at(cell.center());
        let line = format!("ResRef {resref}");
        let started = std::time::Instant::now();
        let mut up = 0;
        let mut last = egui::Rect::NOTHING;
        while up < 30 {
            assert!(started.elapsed().as_secs() < 10, "no enlargement of {name}");
            h.step();
            for tip in enlargements(&h) {
                assert!(window.contains_rect(tip), "{name}: {tip:?} out of {window:?}");
                assert!(!tip.intersects(cell), "{name}: {tip:?} over its cell {cell:?}");
                assert!(tip.right() <= dock.left() + 0.01, "{name}: over the dock");
                last = tip;
            }
            if h.query_by_label(&line).is_some() {
                up += 1;
            }
        }
        sizes.push(last.height());
    }
    assert!(sizes[1] > sizes[0] + 20.0, "a short one, then a tall one: {sizes:?}");
    settle(&mut h);
}

/// After the window is resized and the pointer has rested on the menu
/// bar, a cell's enlargement still comes (the review's report), and after
/// a menu of the bar is opened and closed.
#[test]
fn the_enlargement_comes_after_a_resize_and_the_menu_bar() {
    let Some(mut h) = many_harness("gallery-after-resize") else { return };
    let cell = h.get_by_label("Thing 0001").rect();
    enlarged(&mut h, cell.center(), "mg_many_0001");
    h.set_size(egui::vec2(1100.0, 760.0));
    let file = h.get_all_by_label("File").next().unwrap().rect();
    h.hover_at(file.center());
    settle(&mut h);
    for name in ["Thing 0002", "Thing 0001"] {
        let cell = h.get_by_label(name).rect();
        let resref = format!("mg_many_{}", name.trim_start_matches("Thing "));
        enlarged(&mut h, cell.center(), &resref);
        h.hover_at(file.center());
        settle(&mut h);
    }
    // A menu of the bar opened and closed: the same.
    h.get_all_by_label("File").next().unwrap().click();
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    let cell = h.get_by_label("Thing 0003").rect();
    enlarged(&mut h, cell.center(), "mg_many_0003");
}

/// The size slider at 220 in a 320 dock: rows justified, each cell inside
/// the dock (none cut at its edge), one or more a row.
#[test]
fn the_largest_pictures_in_a_narrow_dock_are_never_cut() {
    let Some(mut h) = many_harness("gallery-220") else { return };
    right_dock_width(&mut h, 320.0);
    h.state_mut().settings.gallery_tile = Some(220);
    settle(&mut h);
    let dock = right_dock_rect(&h);
    let drawn: Vec<(String, egui::Rect)> = cells(&h);
    assert!(!drawn.is_empty() && per_row(&drawn) >= 1);
    for (n, r) in &drawn {
        assert!(r.left() >= dock.left() - 0.5 && r.right() <= dock.right() + 0.5, "{n} {r:?}");
    }
}

/// The palette's toolbar in the GalleryPalette board's order (the
/// orchestrator's ruling): Expand All, Collapse All, the count, List,
/// Gallery, the size slider in the room left, New…, More, on one row at
/// the board's 440; the slider on a row of its own only where less than
/// its least is left (at 280); the Group by row under it with the Gallery
/// on (Category, grayed: the project offers no other).
#[test]
fn the_palette_s_toolbar_is_in_the_board_s_order() {
    let Some(mut h) = many_harness("gallery-toolbar") else { return };
    wide_window(&mut h);
    let x = |r: egui::Rect| (r.center().x, r.center().y);
    let row = |h: &Harness<'_, Moonglow>| -> Vec<(f32, f32)> {
        let dock = right_dock_rect(h);
        let one = |label: &str| {
            let r = h
                .query_all_by_label(label)
                .map(|n| n.rect())
                .find(|r| dock.contains(r.center()))
                .unwrap_or_else(|| panic!("no {label} in the dock"));
            x(r)
        };
        let slider = x(h.get_by_role(egui::accesskit::Role::Slider).rect());
        let mut at: Vec<(f32, f32)> =
            ["Expand All", "Collapse All", "5000 items", "List", "Gallery"].map(one).to_vec();
        at.push(slider);
        at.extend(["New…", "More"].map(one));
        at
    };
    right_dock_width(&mut h, 440.0);
    settle(&mut h);
    let at = row(&h);
    for pair in at.windows(2) {
        assert!((pair[0].1 - pair[1].1).abs() < 1.0, "one row at 440: {at:?}");
        assert!(pair[0].0 < pair[1].0, "in order at 440: {at:?}");
    }
    let group = h.get_by_label("Group by").rect();
    assert!(group.top() > at[0].1, "Group by under the toolbar");
    // Narrow: the slider on a row of its own, under the rest.
    right_dock_width(&mut h, 280.0);
    settle(&mut h);
    let at = row(&h);
    assert!(at[5].1 > at[4].1 + 4.0, "the slider under the rest at 280: {at:?}");
    // (Where it wraps: the widest dock with the slider under the rest.)
    let mut wraps = None;
    for width in [280, 368, 369, 370, 440] {
        right_dock_width(&mut h, width as f32);
        settle(&mut h);
        let at = row(&h);
        if at[5].1 > at[4].1 + 4.0 {
            wraps = Some(width);
        }
    }
    // (The board's row holds down to a dock of 370.)
    assert_eq!(wraps, Some(369), "the slider wraps at a dock of {wraps:?} and narrower");
}

/// The tiles palette's toolbar in its Gallery in the GalleryTiles board's
/// order: List, Gallery, Exact Tile, the size slider in the room left,
/// Expand All, Collapse All, the count, on one row at the board's 440;
/// in its List (the AreaTiles board's) the count and folds first.
#[test]
fn the_tiles_palette_s_toolbar_is_in_the_boards_order() {
    let Some((mut h, _)) = area_harness("gallery-tiles-toolbar") else { return };
    wide_window(&mut h);
    show_palette(h.state_mut());
    settle(&mut h);
    palette_type(&mut h, "Tiles");
    settle(&mut h);
    assert!(h.state().palette.tiles, "the tiles palette");
    right_dock_width(&mut h, 440.0);
    let at = |h: &Harness<'_, Moonglow>, labels: &[&str], slider: bool| -> Vec<(f32, f32)> {
        let dock = right_dock_rect(h);
        let mut out = Vec::new();
        for label in labels {
            if *label == "slider" {
                if slider {
                    let r = h.get_by_role(egui::accesskit::Role::Slider).rect();
                    out.push((r.center().x, r.center().y));
                }
                continue;
            }
            let r = h
                .query_all_by(|n| {
                    let said = n.label().or(n.value());
                    said.is_some_and(|l| l == *label || l.ends_with(" items") && *label == "items")
                })
                .map(|n| n.rect())
                .find(|r| dock.contains(r.center()))
                .unwrap_or_else(|| panic!("no {label} in the dock"));
            out.push((r.center().x, r.center().y));
        }
        out
    };
    let in_order = |at: &[(f32, f32)], what: &str| {
        for pair in at.windows(2) {
            assert!((pair[0].1 - pair[1].1).abs() < 1.0, "{what}: one row: {at:?}");
            assert!(pair[0].0 < pair[1].0, "{what}: in order: {at:?}");
        }
    };
    h.state_mut().settings.palette_gallery = true;
    settle(&mut h);
    let gallery =
        ["List", "Gallery", "Exact Tile", "slider", "Expand All", "Collapse All", "items"];
    in_order(&at(&h, &gallery, true), "the Gallery");
    h.state_mut().settings.palette_gallery = false;
    settle(&mut h);
    let list = ["Expand All", "Collapse All", "items", "List", "Gallery", "Exact Tile"];
    in_order(&at(&h, &list, false), "the List");
}

/// The tiles palette's Gallery's terrain and crosser headings wear the
/// kit's chevrons, as the folders above them do (U30's #8: egui's filled
/// triangles), in Dark and Light: no filled triangle in the Right Dock.
#[test]
fn the_tiles_gallery_s_headings_have_the_kit_s_chevrons() {
    let Some((mut h, _)) = area_harness("gallery-tiles-chevrons") else { return };
    wide_window(&mut h);
    show_palette(h.state_mut());
    settle(&mut h);
    palette_type(&mut h, "Tiles");
    h.state_mut().settings.palette_gallery = true;
    settle(&mut h);
    for light in [false, true] {
        h.state_mut().settings.light_theme = light;
        settle(&mut h);
        let dock = right_dock_rect(&h);
        let three = |closed: bool| {
            (drawn(&h).iter())
                .filter(|s| match s {
                    egui::Shape::Path(p) => {
                        p.points.len() == 3
                            && p.closed == closed
                            && p.points.iter().all(|q| dock.contains(*q))
                    }
                    _ => false,
                })
                .count()
        };
        // (The terrains' headings shown: Grass's, the tileset's first.)
        let headings = h.query_all_by_label_contains("Grass").count();
        assert!(headings > 0, "a terrain's heading ({light})");
        assert_eq!(three(true), 0, "no filled triangle in the dock ({light})");
        assert!(three(false) > 2, "the chevrons ({light}): {}", three(false));
    }
}

/// The tiles palette's folders (Features, Groups, Terrain) have the
/// AreaTiles board's folder icons after their chevrons, closed or open,
/// before their names (U30's #7: none), in its List and its Gallery.
#[test]
fn the_tiles_palette_s_folders_have_folder_icons() {
    let Some((mut h, _)) = area_harness("tiles-folder-icons") else { return };
    wide_window(&mut h);
    show_palette(h.state_mut());
    settle(&mut h);
    palette_type(&mut h, "Tiles");
    settle(&mut h);
    mg_uikit::record_icons(&h.ctx);
    for gallery in [false, true] {
        h.state_mut().settings.palette_gallery = gallery;
        settle(&mut h);
        let drawn = mg_uikit::icons_drawn(&h.ctx);
        let dock = right_dock_rect(&h);
        let folders: Vec<(String, egui::Rect)> = ["Features", "Groups", "Terrain"]
            .iter()
            .flat_map(|name| {
                let rows = h.query_all_by_label(name).map(|n| n.rect());
                rows.filter(|r| dock.contains(r.center()))
                    .map(|r| (name.to_string(), r))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(!folders.is_empty(), "the folders (gallery {gallery})");
        for (name, row) in &folders {
            let icon = drawn.iter().find(|(i, r)| {
                matches!(i, mg_uikit::Icon::Folder | mg_uikit::Icon::FolderOpen)
                    && row.contains_rect(*r)
            });
            let (icon, at) =
                icon.unwrap_or_else(|| panic!("{name}: no folder (gallery {gallery})"));
            // (Before its name: within its row's first few dozen points.)
            assert!(at.left() < row.left() + 40.0, "{name}: {icon:?} at {at:?} in {row:?}");
        }
    }
}

/// A picture with nothing drawn in it shows the kind's icon and "No
/// picture", as one that failed does (U30's #12: Null Human, whose meshes
/// are wholly see-through, an empty cell in the Appearance Gallery).
#[test]
fn a_picture_with_nothing_drawn_says_no_picture() {
    // (With a GPU: the pictures drawn, as the window draws them.)
    let Some((mut h, _)) = area_harness("gallery-null-human") else { return };
    wide_window(&mut h);
    h.state_mut().actions.push(mg_ui::Action::PlaceableGallery);
    settle(&mut h);
    h.state_mut().placeable_gallery.as_mut().unwrap().kind =
        mg_ui::appearance_gallery::Kind::Creature;
    h.state_mut().placeable_gallery.as_mut().unwrap().filter = "Null Human".into();
    settle(&mut h);
    assert_eq!(h.state().thumbnails.waiting(), 0, "the pictures made");
    let cell = (h.get_all_by_label("Null Human").map(|n| n.rect()))
        .find(|r| r.height() > 60.0)
        .expect("Null Human's cell");
    let said = drawn(&h).iter().any(|s| match s {
        egui::Shape::Text(t) => t.galley.text() == "No picture" && cell.contains(t.pos),
        _ => false,
    });
    assert!(said, "\"No picture\" in Null Human's cell {cell:?}");
}
