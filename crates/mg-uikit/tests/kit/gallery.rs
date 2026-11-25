//! The gallery cell, its grid and the size slider (`mg_uikit::gallery`,
//! M4's U25): a grid lays out and paints the rows in sight alone, however
//! many cells it has, is one Tab stop, and its keys move a cursor that is
//! brought into sight; each cell is a button named for a screen reader,
//! said selected or not, that a click acts on; and each state draws as
//! the Quiet Tiles cell has it.

use std::cell::{Cell, RefCell};

use egui::Key;
use egui::accesskit::{Action, Role};
use egui::{Color32, Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::gallery::{self, CellState, GalleryCell, GalleryGrid, Picture};
use mg_uikit::{Icon, Theme};

use crate::common::{colored_texts as texts, shapes};

/// The rim strokes drawn: (width, color).
fn rims(h: &Harness<'_>) -> Vec<(f32, Color32)> {
    shapes(h)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Rect(r) if r.stroke.width > 0.0 => Some((r.stroke.width, r.stroke.color)),
            _ => None,
        })
        .collect()
}

/// A grid of 5,000 cells in a scroll 300 by 400: a frame lays out and
/// paints only the cells of the rows in sight (and makes their
/// responses), not the 5,000; the rows ahead are one more row; scrolled
/// to the end, the last cells are those drawn.
#[test]
fn a_grid_lays_out_and_paints_only_the_rows_in_sight() {
    let count = 5000;
    let drawn = RefCell::new(Vec::<usize>::new());
    let output = RefCell::new(None);
    let mut h = Harness::builder().with_size(egui::vec2(300.0, 400.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        drawn.borrow_mut().clear();
        egui::ScrollArea::vertical().id_salt("grid").show(ui, |ui| {
            let id = egui::Id::new("cells");
            let out = GalleryGrid::new(id, count, 64.0).show(ui, |ui, i, rect, tile, r, state| {
                drawn.borrow_mut().push(i);
                let name = format!("Cell {i}");
                GalleryCell::new(&name, Picture::Making)
                    .kind(Icon::Placeable)
                    .paint(ui, rect, tile, state, r);
            });
            *output.borrow_mut() = Some(out);
        });
    });
    h.run();
    let out = output.borrow().clone().unwrap();
    let layout = out.layout.unwrap();
    let per_row = layout.columns;
    let rows_seen = (400.0 / (layout.cell.y + layout.gap.y)).ceil() as usize + 1;
    let n = drawn.borrow().len();
    assert!(n > 0 && n <= rows_seen * per_row, "{n} drawn, {per_row} a row");
    assert_eq!(out.drawn, 0..n);
    assert_eq!(out.ahead.len(), n + per_row, "one row ahead");
    assert!(out.rect.height() > 5000.0 / per_row as f32 * layout.cell.y, "the whole grid's room");
    // Only those cells are nodes for a screen reader.
    assert!(h.query_by_label("Cell 0").is_some());
    assert!(h.query_by_label(&format!("Cell {}", count - 1)).is_none());
    // Scrolled to its end (End, the grid with the keys): the last drawn.
    h.get_by_label("Cell 0").click();
    h.run();
    h.key_press(Key::End);
    h.run();
    h.run();
    let last = drawn.borrow().iter().copied().max().unwrap();
    assert_eq!(last, count - 1);
    assert!(drawn.borrow().len() <= rows_seen * per_row, "{}", drawn.borrow().len());
}

/// The grid is one Tab stop between the controls around it; with the keys
/// Down goes a row, Right a cell, End to the last (brought into sight:
/// drawn, and inside the scroll's view), Home to the first, Enter acts on
/// the cell; a cell clicked gives the grid the keys. Without the keys
/// (Tab on past it) the arrows move nothing.
#[test]
fn the_grid_is_one_tab_stop_and_its_keys_move_a_cursor_into_sight() {
    let count = 300;
    let cursor = Cell::new(None::<usize>);
    let activated = Cell::new(None::<usize>);
    let seen = RefCell::new(Vec::<(usize, Rect)>::new());
    let view = Cell::new(Rect::NOTHING);
    let mut h = Harness::builder().with_size(egui::vec2(300.0, 400.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let _ = ui.button("Before");
        seen.borrow_mut().clear();
        let shown = egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            let id = egui::Id::new("cells");
            GalleryGrid::new(id, count, 64.0).show(ui, |ui, i, rect, tile, r, state| {
                if state.cursor {
                    cursor.set(Some(i));
                }
                seen.borrow_mut().push((i, rect));
                let name = format!("Cell {i}");
                GalleryCell::new(&name, Picture::Making).paint(ui, rect, tile, state, r);
            })
        });
        view.set(shown.inner_rect);
        if let Some(a) = shown.inner.activated {
            activated.set(Some(a));
        }
        let _ = ui.button("After");
    });
    h.run();
    // Tab: Before, the grid, After.
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let focused = h
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().is_focused())
        .map(|n| n.accesskit_node().role());
    assert_eq!(focused, Some(Role::List), "the grid has the focus, not a cell");
    h.key_press(Key::Tab);
    h.run();
    assert!(h.get_by_label("After").accesskit_node().is_focused(), "one Tab stop for the grid");
    // Without the keys, the arrows move nothing.
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(cursor.get(), None);
    h.key_press_modifiers(egui::Modifiers::SHIFT, Key::Tab);
    h.run();
    h.key_press(Key::ArrowDown); // (the first press: the first cell)
    h.run();
    assert_eq!(cursor.get(), Some(0));
    let across = seen.borrow().iter().filter(|(_, r)| r.top() == seen.borrow()[0].1.top()).count();
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(cursor.get(), Some(across), "a row down");
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(cursor.get(), Some(across + 1), "a cell on");
    h.key_press(Key::End);
    h.run();
    h.run();
    assert_eq!(cursor.get(), Some(count - 1));
    let last = seen.borrow().iter().find(|(i, _)| *i == count - 1).map(|(_, r)| *r);
    let last = last.expect("the last cell drawn once it takes the keys");
    assert!(view.get().contains_rect(last), "in sight: {last:?} in {:?}", view.get());
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(activated.get(), Some(count - 1));
    h.key_press(Key::Home);
    h.run();
    h.run();
    assert_eq!(cursor.get(), Some(0));
    // A click: the cell clicked takes the cursor.
    h.get_by_label("Cell 2").click();
    h.run();
    assert_eq!(cursor.get(), Some(2));
    // Tab out (to After) and back: the cursor kept, the keys taken again.
    h.key_press(Key::Tab);
    h.run();
    assert!(h.get_by_label("After").accesskit_node().is_focused(), "tabbed out");
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(cursor.get(), Some(2), "no key taken while tabbed out");
    h.key_press_modifiers(egui::Modifiers::SHIFT, Key::Tab);
    h.run();
    assert_eq!(cursor.get(), Some(2), "kept, tabbed back");
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(cursor.get(), Some(3), "from where it was");
}

/// A grid is a list to a screen reader, named, with its count; each cell
/// built says its place in it and the count; the cells out of sight are
/// not built; the keys' cell is the list's active one.
#[test]
fn the_grid_is_a_list_its_cells_their_places_in_it() {
    let count = 300;
    let mut h = Harness::builder().with_size(egui::vec2(300.0, 400.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            let id = egui::Id::new("cells");
            GalleryGrid::new(id, count, 64.0).name("Pictures").show(
                ui,
                |ui, i, rect, tile, r, state| {
                    let name = format!("Cell {i}");
                    GalleryCell::new(&name, Picture::Making).paint(ui, rect, tile, state, r);
                },
            );
        });
    });
    h.run();
    let list = h.get_by_role(Role::List);
    assert_eq!(list.accesskit_node().label().as_deref(), Some("Pictures"));
    assert_eq!(list.accesskit_node().data().size_of_set(), Some(count));
    let cells: Vec<(String, Option<usize>, Option<usize>)> = h
        .get_all_by_role(Role::Button)
        .map(|n| {
            let n = n.accesskit_node();
            (n.label().unwrap_or_default(), n.data().position_in_set(), n.data().size_of_set())
        })
        .collect();
    assert!(!cells.is_empty() && cells.len() < 60, "{} built of {count}", cells.len());
    for (name, place, of) in &cells {
        let i: usize = name.trim_start_matches("Cell ").parse().unwrap();
        assert_eq!((*place, *of), (Some(i + 1), Some(count)), "{name}");
    }
    assert!(h.query_by_label("Cell 299").is_none(), "the last not built");
    assert_eq!(list.accesskit_node().data().active_descendant(), None, "no cursor yet");
    h.get_by_label("Cell 2").click();
    h.run();
    let active = h.get_by_role(Role::List).accesskit_node().data().active_descendant();
    let cell = h.get_by_label("Cell 2").accesskit_node().locate().0;
    assert_eq!(active, Some(cell), "the keys' cell, the list's active one");
}

/// The cursor a grid keeps is its own while it is shown frame after frame
/// on the same list from the same one chosen: shown again after a frame
/// without it (a gallery opened again), it starts from the one chosen it
/// is given, not the cell it was left on.
#[test]
fn a_grid_shown_again_starts_from_the_one_chosen() {
    let shown = Cell::new(true);
    let given = Cell::new(Some(5usize));
    let cursor = Cell::new(None::<usize>);
    let mut h = Harness::builder().with_size(egui::vec2(300.0, 400.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        if !shown.get() {
            return;
        }
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            let id = egui::Id::new("cells");
            GalleryGrid::new(id, 300, 64.0).cursor(given.get()).show(
                ui,
                |ui, i, rect, tile, r, state| {
                    if state.cursor {
                        cursor.set(Some(i));
                    }
                    let name = format!("Cell {i}");
                    GalleryCell::new(&name, Picture::Making).paint(ui, rect, tile, state, r);
                },
            );
        });
    });
    h.run();
    assert_eq!(cursor.get(), Some(5), "from the one chosen");
    h.get_by_label("Cell 2").click();
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(cursor.get(), Some(3), "moved, kept while shown");
    shown.set(false);
    h.run();
    shown.set(true);
    cursor.set(None);
    h.run();
    assert_eq!(cursor.get(), Some(5), "shown again: from the one chosen");
}

/// Each cell is a button named by its name (or what it says more), said
/// selected or not, with a click; drawn disabled, it says so.
#[test]
fn each_cell_is_a_named_button_said_selected_with_a_click() {
    let mut h = Harness::builder().with_size(egui::vec2(400.0, 300.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        GalleryGrid::new(egui::Id::new("cells"), 3, 100.0).show(ui, |ui, i, rect, tile, r, s| {
            let (name, said) = [("Wraith", "Wraith  (CR 5)"), ("Bat", "Bat"), ("Off", "Off")][i];
            GalleryCell::new(name, Picture::Missing)
                .said(said)
                .selected(i == 0)
                .disabled(i == 2)
                .paint(ui, rect, tile, s, r);
        });
    });
    h.run();
    let wraith = h.get_by_label("Wraith  (CR 5)");
    assert_eq!(wraith.accesskit_node().role(), Role::Button);
    assert_eq!(wraith.accesskit_node().data().is_selected(), Some(true));
    assert!(wraith.accesskit_node().data().supports_action(Action::Click));
    let bat = h.get_by_label("Bat");
    assert_eq!(bat.accesskit_node().data().is_selected(), Some(false));
    assert!(h.get_by_label("Off").accesskit_node().is_disabled());
}

/// The states as the Quiet Tiles cell draws them, in both themes: at rest
/// a 1 point `line` rim and the name `text-2`; hovered a `raised-hover`
/// rim and the name `text`; selected a 2 point `accent` ring, `text-3`
/// where the grid has not the keys; being made, the kind's icon and no
/// "…"; no picture, "No picture" under it; no model, the "No model" badge
/// in `thumb-badge`; a favorite, the bookmark filled `thumb-flag`;
/// disabled, the name `text-off`; the detail line `text-3`; a model name
/// in Hack, one line cut with an ellipsis; the small variant's name 11.
#[test]
fn each_state_draws_as_quiet_tiles() {
    for theme in Theme::ALL {
        let t = theme.tokens();
        let draw = |cell: &dyn Fn() -> GalleryCell<'static>, state: CellState| {
            let cell = cell();
            let mut h =
                Harness::builder().with_size(egui::vec2(200.0, 200.0)).build_ui(move |ui| {
                    mg_uikit::apply(ui.ctx(), theme);
                    let (rect, r) =
                        ui.allocate_exact_size(egui::vec2(128.0, 176.0), egui::Sense::click());
                    cell.paint(ui, rect, egui::Vec2::splat(128.0), state, &r);
                });
            h.run();
            h
        };
        let rest = CellState::default();
        let plain = || GalleryCell::new("Corpse Wagon", Picture::Making).kind(Icon::Placeable);
        let h = draw(&plain, rest);
        assert!(rims(&h).contains(&(1.0, t.line)), "{theme:?} at rest: {:?}", rims(&h));
        assert!(texts(&h).contains(&("Corpse Wagon".into(), t.text_2)), "{theme:?}");
        assert!(!texts(&h).iter().any(|(s, _)| s.contains('…')), "no … while made");
        let icon_drawn = shapes(&h).iter().any(|s| match s {
            Shape::Path(p) => p.stroke.color == egui::epaint::ColorMode::Solid(t.text_off),
            Shape::LineSegment { stroke, .. } => stroke.color == t.text_off,
            Shape::Mesh(_) => true,
            _ => false,
        });
        assert!(icon_drawn, "{theme:?}: the kind's icon while it is made");
        let h = draw(&plain, CellState { hovered: true, ..rest });
        // (Light: `text-off`, which shows on the panel the rim stands on.)
        let hover = if theme == Theme::Light { t.text_off } else { t.raised_hover };
        assert!(rims(&h).contains(&(1.0, hover)), "{theme:?} hovered");
        assert!(texts(&h).contains(&("Corpse Wagon".into(), t.text)), "{theme:?} hovered");
        let selected = || plain().selected(true);
        let h = draw(&selected, CellState { keys: true, ..rest });
        assert!(rims(&h).contains(&(2.0, t.accent)), "{theme:?} selected");
        assert!(texts(&h).contains(&("Corpse Wagon".into(), t.text)), "{theme:?} selected");
        let h = draw(&selected, rest);
        assert!(rims(&h).contains(&(2.0, t.text_3)), "{theme:?} selected without the keys");
        let missing = || GalleryCell::new("Basket", Picture::Missing).kind(Icon::Placeable);
        assert!(texts(&draw(&missing, rest)).iter().any(|(s, _)| s == "No picture"));
        let no_model = || plain().no_model(true);
        let h = draw(&no_model, rest);
        assert!(texts(&h).contains(&("No model".into(), t.thumb_badge_text)), "{theme:?}");
        let badge =
            shapes(&h).iter().any(|s| matches!(s, Shape::Rect(r) if r.fill == t.thumb_badge));
        assert!(badge, "{theme:?}: the badge");
        let favorite = || plain().favorite(true);
        let h = draw(&favorite, rest);
        let flag = shapes(&h).iter().any(|s| matches!(s, Shape::Path(p) if p.fill == t.thumb_flag));
        assert!(flag, "{theme:?}: the bookmark filled");
        assert!(!texts(&h).iter().any(|(s, _)| s.contains('★')), "no star before the name");
        let disabled = || plain().disabled(true);
        assert!(texts(&draw(&disabled, rest)).contains(&("Corpse Wagon".into(), t.text_off)));
        let detail = || GalleryCell::new("Wraith", Picture::Making).detail(Some("CR 5"));
        let h = draw(&detail, rest);
        assert!(texts(&h).contains(&("Wraith".into(), t.text_2)));
        assert!(texts(&h).contains(&("CR 5".into(), t.text_3)), "{theme:?}: the detail");
        // (Right under a one-line name, as GalleryWide's "Dire Wolf" over
        // "CR 5": not at the foot, a blank line between.)
        let tops: Vec<(String, f32, f32)> = shapes(&h)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text(t) => {
                    let r = t.visual_bounding_rect();
                    Some((t.galley.text().to_string(), r.top(), r.bottom()))
                }
                _ => None,
            })
            .collect();
        let name = tops.iter().find(|(s, ..)| s == "Wraith").expect("the name");
        let cr = tops.iter().find(|(s, ..)| s == "CR 5").expect("the detail");
        let between = cr.1 - name.2;
        assert!((-1.0..10.0).contains(&between), "{theme:?}: {between} between the name and CR 5");
        // A model name: Hack, one line, cut.
        let model =
            || GalleryCell::new("tcn01_a02_01_and_a_long_tail", Picture::Making).model(true);
        let h = draw(&model, rest);
        let fonts: Vec<(String, egui::FontFamily)> = shapes(&h)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Text(t) => Some((
                    t.galley.text().to_string(),
                    t.galley.job.sections[0].format.font_id.family.clone(),
                )),
                _ => None,
            })
            .collect();
        let name = fonts.iter().find(|(s, _)| s.starts_with("tcn01")).unwrap();
        assert_eq!(name.1, egui::FontFamily::Monospace);
        assert!(name.0.ends_with('…'), "{name:?}");
        assert_eq!(
            fonts.iter().filter(|(s, _)| s.starts_with("tcn") || s.ends_with("tail")).count(),
            1
        );
        // The small variant: its name 11.
        let small = || GalleryCell::new("Plague Victim Pyre", Picture::Making).small(true);
        let h = draw(&small, rest);
        let size = shapes(&h).into_iter().find_map(|s| match s {
            Shape::Text(t) if t.galley.text().starts_with("Plague") => {
                Some(t.galley.job.sections[0].format.font_id.size)
            }
            _ => None,
        });
        assert_eq!(size, Some(gallery::SMALL_NAME_SIZE));
    }
}

/// The size slider: a slider, its value the side, in its range, moved by
/// the keys a point at a time.
#[test]
fn the_size_slider_says_the_side() {
    let side = Cell::new(128.0_f32);
    let mut h = Harness::builder().with_size(egui::vec2(300.0, 60.0)).build_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let mut s = side.get();
        gallery::size_slider(ui, &mut s, gallery::SIDE_MIN..=gallery::SIDE_MAX, None)
            .on_hover_text("How large the pictures are");
        side.set(s);
    });
    h.run();
    let node =
        h.root().children_recursive().find(|n| n.accesskit_node().role() == Role::Slider).unwrap();
    assert_eq!(node.accesskit_node().numeric_value(), Some(128.0));
    assert_eq!(node.accesskit_node().min_numeric_value(), Some(64.0));
    assert_eq!(node.accesskit_node().max_numeric_value(), Some(220.0));
    node.focus();
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(side.get(), 129.0);
}

/// A picture that fills its tile (`Picture::Fill`, the portraits'): the
/// texture drawn over the whole tile, the part of it named (its UV
/// rectangle), not letterboxed, inside the tile.
#[test]
fn a_filled_picture_covers_its_tile_with_the_part_named() {
    let texture = egui::TextureId::User(7);
    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 100.0 / 128.0));
    let tile = std::rc::Rc::new(Cell::new(Rect::NOTHING));
    let seen = tile.clone();
    let mut h = Harness::builder().with_size(egui::vec2(200.0, 200.0)).build_ui(move |ui| {
        let (rect, r) = ui.allocate_exact_size(egui::vec2(54.0, 84.0), egui::Sense::click());
        seen.set(rect);
        GalleryCell::new("", Picture::Fill(texture, uv)).lines(0).paint(
            ui,
            rect,
            egui::vec2(54.0, 84.0),
            CellState::default(),
            &r,
        );
    });
    h.run();
    let meshes: Vec<egui::epaint::Mesh> = shapes(&h)
        .into_iter()
        .filter_map(|s| match s {
            Shape::Mesh(m) if m.texture_id == texture => Some((*m).clone()),
            _ => None,
        })
        .collect();
    assert_eq!(meshes.len(), 1, "the picture drawn once");
    let m = &meshes[0];
    let (mut at, mut uvs) = (Rect::NOTHING, Rect::NOTHING);
    for v in &m.vertices {
        at.extend_with(v.pos);
        uvs.extend_with(v.uv);
    }
    let tile = tile.get();
    assert!((at.min - tile.min).length() < 0.01 && (at.max - tile.max).length() < 0.01, "{at:?}");
    assert!((uvs.min - uv.min).length() < 0.001 && (uvs.max - uv.max).length() < 0.001, "{uvs:?}");
}

/// A grid's own row gap (`row_gap`): the rows that far apart, the gap
/// across left as the layout makes it (justified: at least its least).
#[test]
fn a_row_gap_sets_the_rows_apart() {
    let grid = GalleryGrid::new(egui::Id::new("rows"), 40, 54.0)
        .justified(egui::vec2(54.0, 84.0), 8.0)
        .row_gap(3.0)
        .label(0.0);
    let l = grid.layout(500.0);
    assert_eq!(l.gap.y, 3.0);
    assert!(l.gap.x >= 8.0, "{:?}", l.gap);
    let (first, below) = (l.rect(egui::Pos2::ZERO, 0), l.rect(egui::Pos2::ZERO, l.columns));
    assert!((below.top() - first.bottom() - 3.0).abs() < 0.01, "{first:?} {below:?}");
    assert!((l.height(2 * l.columns) - (2.0 * 84.0 + 3.0)).abs() < 0.01);
    // (Without it, the layout's own.)
    let plain = GalleryGrid::new(egui::Id::new("rows"), 40, 54.0)
        .justified(egui::vec2(54.0, 84.0), 8.0)
        .label(0.0)
        .layout(500.0);
    assert_ne!(plain.gap.y, 3.0);
}
