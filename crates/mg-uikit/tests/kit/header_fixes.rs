//! The editor header, popover and shelf as U1's review found them
//! wanting: a disabled item disabled wherever it is drawn, More measured
//! again after any close, a popover not reopening by itself, the
//! keyboard going into a popover opened by it, the end slot kept inside
//! the row, a right click anywhere on the row, a menu closed only by its
//! enabled rows, two headers of one name, tall popovers and short
//! shelves, the switch's role, joined groups, compact forms, opening
//! from code, hanging above, and the header's text size kept.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use egui::accesskit::{Role, Toggled};
use egui::{Rect, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::header::{self, EditorHeader, Item, Joined, Place};
use mg_uikit::popover::{Hang, PopAction, PopCheck, Popover};
use mg_uikit::shelf::{Tool, ToolShelf};
use mg_uikit::{Icon, KitButton};

use crate::common::harness;

const H: &str = "Fix header";

// ------------------------------------------------------------------ A1

/// A disabled popover moved into More: its rows there are said disabled
/// and a click on one changes nothing.
#[test]
fn a_disabled_popover_is_disabled_in_more() {
    let grid = Cell::new(false);
    let mut h = harness(vec2(600.0, 400.0), |ui| {
        let mut g = grid.get();
        let pop = Popover::new("Overlays").check(PopCheck::new(&mut g, "Grid"));
        let _ = EditorHeader::new(H)
            .right(Item::popover(Some(Icon::Overlays), pop).enabled(false).in_more())
            .show(ui);
        grid.set(g);
    });
    h.run();
    h.get_by_label(H).get_by_label(header::MORE).click();
    h.run();
    let more = header::more_name(H);
    assert!(h.get_by_label(&more).get_by_label("Grid").accesskit_node().is_disabled());
    h.get_by_label(&more).get_by_label("Grid").click();
    h.run();
    assert!(!grid.get(), "a disabled popover's row flipped in More");
}

/// A disabled menu in More: its rows are drawn in a disabled `Ui`, so a
/// click on one does nothing.
#[test]
fn a_disabled_menu_is_disabled_in_more() {
    let hit = Cell::new(false);
    let mut h = harness(vec2(600.0, 400.0), |ui| {
        let _ = EditorHeader::new(H)
            .left(
                Item::menu(Some(Icon::Export), "Export", |ui| {
                    if ui.button("CSV…").clicked() {
                        hit.set(true);
                    }
                })
                .enabled(false)
                .in_more(),
            )
            .show(ui);
    });
    h.run();
    h.get_by_label(H).get_by_label(header::MORE).click();
    h.run();
    let more = header::more_name(H);
    assert!(h.get_by_label(&more).get_by_label("CSV…").accesskit_node().is_disabled());
    h.get_by_label(&more).get_by_label("CSV…").click();
    h.run();
    assert!(!hit.get(), "a disabled menu's row in More did its work");
}

/// A disabled part of the caller's: its `Ui` is disabled on the row, in
/// its compact form and in More.
#[test]
fn a_disabled_custom_item_is_disabled_everywhere() {
    let seen = RefCell::new(Vec::<(&'static str, bool)>::new());
    let mut h = harness(vec2(800.0, 400.0), |ui| {
        let _ = EditorHeader::new(H)
            .left(
                Item::custom("Field", 300.0, |ui| {
                    seen.borrow_mut().push(("row", ui.is_enabled()));
                    ui.label("row form");
                })
                .compact(120.0, |ui| {
                    seen.borrow_mut().push(("compact", ui.is_enabled()));
                    ui.label("compact form");
                })
                .more(160.0, |ui| {
                    seen.borrow_mut().push(("more", ui.is_enabled()));
                    ui.label("More form");
                })
                .enabled(false),
            )
            .show_with_end(ui, 60.0, |ui| {
                let _ = ui.add(KitButton::new(None, "OK"));
            });
    });
    for (w, form) in [(800.0, "row"), (260.0, "compact")] {
        seen.borrow_mut().clear();
        h.set_size(vec2(w, 400.0));
        h.run();
        let seen = seen.borrow();
        assert!(seen.iter().any(|(f, _)| *f == form), "{form} not drawn at {w}: {seen:?}");
        assert!(seen.iter().all(|(_, e)| !e), "{form} enabled: {seen:?}");
    }
    h.set_size(vec2(140.0, 400.0));
    h.run();
    h.get_by_label(H).get_by_label(header::MORE).click();
    seen.borrow_mut().clear();
    h.run();
    let seen = seen.borrow();
    assert!(seen.iter().any(|(f, _)| *f == "more"), "More form not drawn: {seen:?}");
    assert!(seen.iter().all(|(_, e)| !e), "the More form enabled: {seen:?}");
}

/// A disabled popover unfolded into the row: its toggles are said
/// disabled and a click changes nothing.
#[test]
fn a_disabled_popover_is_disabled_unfolded() {
    let grid = Cell::new(false);
    let mut h = harness(vec2(800.0, 300.0), |ui| {
        let mut g = grid.get();
        let pop = Popover::new("Overlays").check(PopCheck::new(&mut g, "Grid").icon(Icon::Grid));
        let out = EditorHeader::new(H)
            .right(Item::popover(Some(Icon::Overlays), pop).enabled(false).unfolds())
            .show(ui);
        assert_eq!(out.place("Overlays"), Some(Place::Unfolded));
        grid.set(g);
    });
    h.run();
    let node = h.get_by_label(H).get_by_label("Grid");
    assert!(node.accesskit_node().is_disabled(), "an unfolded row of a disabled popover");
    node.click();
    h.run();
    assert!(!grid.get());
}

/// A disabled joined group: its buttons on the row are said disabled.
#[test]
fn a_disabled_joined_group_is_disabled() {
    let on = Cell::new(false);
    let mut h = harness(vec2(800.0, 300.0), |ui| {
        let mut v = on.get();
        let parts = vec![Joined::toggle(Icon::Creature, "Show Creatures", &mut v)];
        let _ = EditorHeader::new(H).left(Item::joined("Show", parts).enabled(false)).show(ui);
        on.set(v);
    });
    h.run();
    let node = h.get_by_label(H).get_by_label("Show Creatures");
    assert!(node.accesskit_node().is_disabled());
    node.click();
    h.run();
    assert!(!on.get());
}

// ------------------------------------------------------------------ A3, B9

/// More opened narrow (a long command in it), closed by its own button,
/// and opened again wide is measured again: as wide as what is in it
/// now, as after a close by Escape.
#[test]
fn more_is_measured_again_after_any_close() {
    let rect = Cell::new(None::<Rect>);
    let mut h = harness(vec2(1000.0, 400.0), |ui| {
        let out = EditorHeader::new(H)
            .left(Item::icon(Icon::Open, "A very long command name that is wide indeed…"))
            .left(Item::icon(Icon::ToScratch, "To Scratch").in_more())
            .show(ui);
        rect.set(out.popover_rect);
    });
    h.set_size(vec2(60.0, 400.0));
    h.run();
    h.get_by_label(H).get_by_label(header::MORE).click();
    h.run();
    let narrow = rect.get().expect("More open").width();
    // Closed by its own button.
    h.get_by_label(H).get_by_label(header::MORE).click();
    h.run();
    assert!(rect.get().is_none());
    h.set_size(vec2(1000.0, 400.0));
    h.run();
    h.get_by_label(H).get_by_label(header::MORE).click();
    h.run();
    let wide = rect.get().expect("More open again").width();
    assert!(wide < narrow, "More kept its width: {wide} after {narrow}");
    assert!(wide <= 230.0, "{wide}");
}

// ------------------------------------------------------------------ A4

/// A popover open when its header stops being drawn (another tab in
/// front) does not open by itself when the header comes back.
#[test]
fn a_popover_does_not_reopen_by_itself() {
    let shown = Cell::new(true);
    let mut h = harness(vec2(1000.0, 400.0), |ui| {
        if !shown.get() {
            ui.label("another tab");
            return;
        }
        let mut a = false;
        let pop = Popover::new("Lighting").check(PopCheck::new(&mut a, "Grid"));
        let _ = EditorHeader::new(H).right(Item::popover(Some(Icon::Light), pop)).show(ui);
    });
    h.run();
    h.get_by_label(H).get_by_label("Lighting").click();
    h.run();
    assert!(h.query_by_label("Grid").is_some());
    shown.set(false);
    h.run();
    shown.set(true);
    h.run();
    assert!(h.query_by_label("Grid").is_none(), "the popover came back by itself");
}

// ------------------------------------------------------------------ A5

/// A kit button disabled by itself under a parent's opacity keeps that
/// opacity (box_painter divided egui's fade out even where there was
/// none).
#[test]
fn a_disabled_kit_button_keeps_its_parent_s_opacity() {
    let mut h = harness(vec2(300.0, 100.0), |ui| {
        ui.multiply_opacity(0.5);
        let _ = ui.add(KitButton::new(Some(Icon::Save), "Save").enabled(false));
        ui.add_enabled_ui(false, |ui| ui.add(KitButton::new(Some(Icon::Save), "Undo")));
    });
    h.run();
    let mut seen = 0;
    for c in &h.output().shapes {
        if let egui::Shape::Text(t) = &c.shape {
            let word = t.galley.text();
            if word == "Save" || word == "Undo" {
                seen += 1;
                // (The painter's opacity is in the color's alpha.)
                let alpha = t.fallback_color.a() as f32 * t.opacity_factor;
                assert!((alpha - 127.5).abs() < 2.0, "{word}: {:?}", t.fallback_color);
            }
        }
    }
    assert_eq!(seen, 2);
}

// ------------------------------------------------------------------ A6

/// A popover opened by the keyboard takes the focus to its first row;
/// Tab goes on through its rows.
#[test]
fn the_keyboard_goes_into_a_popover_it_opened() {
    let mut h = harness(vec2(1000.0, 400.0), |ui| {
        let (mut a, mut b) = (false, false);
        let pop = Popover::new("Lighting")
            .check(PopCheck::new(&mut a, "Grid"))
            .check(PopCheck::new(&mut b, "Minimap"));
        let _ = EditorHeader::new(H)
            .left(Item::popover(Some(Icon::Light), pop))
            .left(Item::button(Icon::Save, "Save"))
            .show(ui);
    });
    h.run();
    h.get_by_label(H).get_by_label("Lighting").focus();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.get_by_label("Grid").accesskit_node().is_focused(), "the focus stayed outside");
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("Minimap").accesskit_node().is_focused());
}

// ------------------------------------------------------------------ A7

/// Under the row's least width the end slot is drawn clipped to the row,
/// never outside it.
#[test]
fn the_end_slot_stays_inside_a_row_too_narrow() {
    let row = Cell::new(Rect::NOTHING);
    let mut h = harness(vec2(60.0, 100.0), |ui| {
        let out = EditorHeader::new(H).left(Item::button(Icon::Save, "Save")).show_with_end(
            ui,
            120.0,
            |ui| {
                let _ = ui.add(KitButton::new(None, "OK"));
                let _ = ui.add(KitButton::new(None, "Cancel"));
            },
        );
        row.set(out.response.rect);
    });
    h.run();
    let row = row.get();
    let mut drawn = 0;
    for c in &h.output().shapes {
        if let egui::Shape::Text(t) = &c.shape
            && (t.galley.text() == "OK" || t.galley.text() == "Cancel")
        {
            drawn += 1;
            assert!(row.contains_rect(c.clip_rect), "drawn outside the row: {:?}", c.clip_rect);
        }
    }
    assert!(drawn > 0);
}

// ------------------------------------------------------------------ B1, D1

/// A right click on an item reaches the row as on a gap; each item drawn
/// has its response in the output.
#[test]
fn a_right_click_anywhere_on_the_row_reaches_it() {
    let clicks = Cell::new(0);
    let menu = Cell::new(false);
    let save = Cell::new(None::<Rect>);
    let mut h = harness(vec2(800.0, 300.0), |ui| {
        let out = EditorHeader::new(H).left(Item::button(Icon::Save, "Save")).show(ui);
        if out.secondary_clicked {
            clicks.set(clicks.get() + 1);
        }
        save.set(out.response("Save").map(|r| r.rect));
        if out.context_menu(|ui| ui.label("Which tools to show")).is_some() {
            menu.set(true);
        }
    });
    h.run();
    let at = save.get().expect("Save's response").center();
    for p in [at, egui::pos2(600.0, at.y)] {
        h.event(egui::Event::PointerMoved(p));
        let press = |pressed| egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        };
        h.event(press(true));
        h.run();
        h.event(press(false));
        h.run();
    }
    assert_eq!(clicks.get(), 2, "a right click on Save and on a gap");
    assert!(menu.get(), "the header's own menu opened");
}

// ------------------------------------------------------------------ B2

/// A menu closes on a click on an enabled row only, not on a heading or
/// a disabled row.
#[test]
fn a_menu_closes_on_an_enabled_row_only() {
    let mut h = harness(vec2(800.0, 300.0), |ui| {
        let _ = EditorHeader::new(H)
            .left(Item::menu(Some(Icon::Export), "Export", |ui| {
                ui.label("A heading");
                ui.add_enabled(false, egui::Button::new("Disabled row"));
                let _ = ui.button("CSV…");
            }))
            .show(ui);
    });
    h.run();
    h.get_by_label(H).get_by_label("Export").click();
    h.run();
    h.get_by_label("Disabled row").click();
    h.run();
    h.get_by_label("A heading").click();
    h.run();
    assert!(h.query_by_label("CSV…").is_some(), "closed by a heading or a disabled row");
    h.get_by_label("CSV…").click();
    h.run();
    assert!(h.query_by_label("CSV…").is_none());
}

// ------------------------------------------------------------------ B3

/// The first command (Save) moves into More after the right group's
/// items, last of everything.
#[test]
fn the_first_command_goes_into_more_last() {
    let places = RefCell::new(Vec::new());
    let mut day = false;
    let mut h = harness(vec2(900.0, 300.0), |ui| {
        let out = EditorHeader::new(H)
            .left(Item::button(Icon::Save, "Save").primary())
            .left(Item::button(Icon::Compile, "Compile"))
            .left(Item::icon(Icon::Print, "Print…"))
            .right(Item::toggle(Some(Icon::Sun), "Day", &mut day))
            .right(Item::words("Area Properties"))
            .show_with_end(ui, 120.0, |_| ());
        *places.borrow_mut() = out.places;
    });
    let mut seen_day_alone = false;
    let mut w = 900.0;
    while w > 200.0 {
        h.set_size(vec2(w, 300.0));
        h.run();
        let p = places.borrow().clone();
        let of = |n: &str| p.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
        if of("Save") == Some(Place::More) {
            for n in ["Compile", "Print…", "Day", "Area Properties"] {
                assert_eq!(of(n), Some(Place::More), "Save into More before {n} at {w}");
            }
        }
        if of("Area Properties") == Some(Place::More) && of("Save") != Some(Place::More) {
            seen_day_alone = true;
        }
        w -= 10.0;
    }
    assert!(seen_day_alone, "never a width with the right group in More and Save not");
}

// ------------------------------------------------------------------ B4

/// Two headers of one name keep a record each, told apart by their ids
/// and ordered by where they are.
#[test]
fn two_headers_of_one_name_keep_a_record_each() {
    let mut h = harness(vec2(800.0, 300.0), |ui| {
        for salt in ["a", "b"] {
            ui.push_id(salt, |ui| {
                let _ = EditorHeader::new(H)
                    .left(Item::button(Icon::Save, format!("Save {salt}")))
                    .show(ui);
            });
        }
    });
    h.run();
    let records = header::records(&h.ctx, H);
    assert_eq!(records.len(), 2);
    assert_ne!(records[0].id, records[1].id);
    assert!(records[0].row.top() < records[1].row.top());
    assert_eq!(records[0].places[0].0, "Save a");
    assert_eq!(records[1].places[0].0, "Save b");
}

// ------------------------------------------------------------------ B5, D7

/// A tall popover in a short window scrolls: it ends inside the window
/// and stays under its button.
#[test]
fn a_tall_popover_scrolls_under_its_button() {
    let rect = Cell::new(None::<Rect>);
    let button = Cell::new(Rect::NOTHING);
    let mut values = [false; 30];
    let mut h = harness(vec2(800.0, 260.0), |ui| {
        let mut pop = Popover::new("Many");
        for (i, v) in values.iter_mut().enumerate() {
            pop = pop.check(PopCheck::new(v, format!("Row {i}")));
        }
        let out = EditorHeader::new(H).left(Item::popover(None, pop)).show(ui);
        rect.set(out.popover_rect);
        if let Some(r) = out.response("Many") {
            button.set(r.rect);
        }
    });
    h.run();
    h.get_by_label(H).get_by_label("Many").click();
    h.run();
    let p = rect.get().expect("open");
    assert!(p.bottom() <= 260.0 + 0.5, "past the window: {p:?}");
    assert!(p.top() >= button.get().bottom(), "over its button: {p:?}");
}

/// `Hang::Above` hangs a popover over its button, left edges aligned.
#[test]
fn a_popover_can_hang_above() {
    let out_rect = Cell::new(None::<Rect>);
    let button = Cell::new(Rect::NOTHING);
    let mut h = harness(vec2(600.0, 400.0), |ui| {
        ui.add_space(300.0);
        let mut a = false;
        let b = ui.add(KitButton::new(None, "Above").opens(false));
        button.set(b.rect);
        let p = Popover::new("Above popover")
            .hang(Hang::Above)
            .check(PopCheck::new(&mut a, "Grid"))
            .show(ui, &b);
        out_rect.set(p.rect);
    });
    h.run();
    h.get_by_label("Above").click();
    h.run();
    let (p, b) = (out_rect.get().expect("open"), button.get());
    assert!((p.bottom() - (b.top() - 4.0)).abs() < 1.0, "{p:?} {b:?}");
    assert!((p.left() - b.left()).abs() < 1.0, "{p:?} {b:?}");
}

// ------------------------------------------------------------------ B6

/// A shelf too short for its tools puts the rest in a More tool at its
/// foot; a tool chosen there is reported as on the shelf.
#[test]
fn a_short_shelf_puts_its_last_tools_in_more() {
    let clicked = Cell::new(None);
    let mut h = harness(vec2(300.0, 140.0), |ui| {
        let mut shelf = ToolShelf::new();
        for (i, icon) in [Icon::Select, Icon::SelectTiles, Icon::Paint, Icon::Terrain, Icon::Brush]
            .into_iter()
            .enumerate()
        {
            shelf = shelf.tool(Tool::new(icon, format!("Tool {i}")).on(i == 4));
        }
        if let Some(i) = shelf.show(ui).clicked {
            clicked.set(Some(i));
        }
    });
    h.run();
    let tools = h.get_by_label(mg_uikit::shelf::TOOLS);
    let more = tools.get_by_label(mg_uikit::shelf::MORE_TOOLS);
    assert_eq!(more.accesskit_node().toggled(), Some(Toggled::True), "on: a tool in it is");
    assert!(tools.query_by_label("Tool 4").is_none());
    h.get_by_label(mg_uikit::shelf::MORE_TOOLS).click();
    h.run();
    h.get_by_label("Tool 4").click();
    h.run();
    assert_eq!(clicked.get(), Some(4));
}

// ------------------------------------------------------------------ B7

/// The master switch is a switch to a screen reader, toggled as it is.
#[test]
fn the_master_switch_is_a_switch() {
    let mut h = harness(vec2(400.0, 300.0), |ui| {
        let mut on = true;
        let _ = Popover::new("Overlays").switch(&mut on, "Show overlays").show_in_place(ui);
    });
    h.run();
    let node = h.get_by_label("Show overlays");
    assert_eq!(node.accesskit_node().role(), Role::Switch);
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::True));
}

// ------------------------------------------------------------------ B8

/// An action row of a popover drawn in place closes no closable parent.
#[test]
fn an_action_in_place_closes_no_parent() {
    let parent_closed = Cell::new(false);
    let mut h = harness(vec2(400.0, 300.0), |ui| {
        let scope = ui.scope_builder(egui::UiBuilder::new().closable(), |ui| {
            let _ = Popover::new("Brush").action(PopAction::new("Scatter")).show_in_place(ui);
        });
        if scope.response.should_close() {
            parent_closed.set(true);
        }
    });
    h.run();
    h.get_by_label("Scatter").click();
    h.run();
    assert!(!parent_closed.get(), "the catalog's parent was asked to close");
}

// ------------------------------------------------------------------ C3

/// A joined group stands as one, its buttons 1 px apart, and yields as
/// one into More, where its toggles are check rows under its name.
#[test]
fn a_joined_group_yields_as_one() {
    let creature = Cell::new(false);
    let places = RefCell::new(Vec::new());
    let mut h = harness(vec2(800.0, 400.0), |ui| {
        let (mut c, mut d) = (creature.get(), true);
        let parts = vec![
            Joined::toggle(Icon::Creature, "Show Creatures", &mut c),
            Joined::toggle(Icon::Door, "Show Doors", &mut d),
        ];
        let out =
            EditorHeader::new(H).left(Item::joined("Show", parts)).show_with_end(ui, 60.0, |ui| {
                let _ = ui.add(KitButton::new(None, "OK"));
            });
        creature.set(c);
        *places.borrow_mut() = out.places;
    });
    h.run();
    let a = h.get_by_label(H).get_by_label("Show Creatures").rect();
    let b = h.get_by_label(H).get_by_label("Show Doors").rect();
    assert!((b.left() - a.right() - 1.0).abs() < 0.5, "a 1 px seam: {a:?} {b:?}");
    h.set_size(vec2(130.0, 400.0));
    h.run();
    let p = places.borrow().clone();
    for n in ["Show", "Show Creatures", "Show Doors"] {
        assert!(p.contains(&(n.to_string(), Place::More)), "{n} not in More: {p:?}");
    }
    h.get_by_label(H).get_by_label(header::MORE).click();
    h.run();
    let more = header::more_name(H);
    h.get_by_label(&more).get_by_label("Show Creatures").click();
    h.run();
    assert!(creature.get());
}

// ------------------------------------------------------------------ D3, D4

/// A named row of the caller's unfolded into the row is recorded by its
/// name.
#[test]
fn an_unfolded_custom_row_is_recorded() {
    let places = RefCell::new(Vec::new());
    let mut h = harness(vec2(900.0, 300.0), |ui| {
        let pop = Popover::new("Lighting").custom_unfolded(
            "Fade Geometry",
            |ui| {
                ui.label("in the popover");
            },
            120.0,
            |ui| {
                ui.label("on the row");
            },
        );
        let out = EditorHeader::new(H).right(Item::popover(None, pop).unfolds()).show(ui);
        *places.borrow_mut() = out.places;
    });
    h.run();
    assert!(places.borrow().contains(&("Fade Geometry".to_string(), Place::Unfolded)));
}

/// A part of the caller's with a compact form takes it where the row
/// narrows, before moving into More (U3's Snap and Turn without their
/// value text).
#[test]
fn a_custom_item_takes_its_compact_form() {
    let forms = RefCell::new(Vec::new());
    let mut h = harness(vec2(900.0, 300.0), |ui| {
        let out = EditorHeader::new(H)
            .left(
                Item::custom("Snap", 200.0, |ui| {
                    ui.label("Snap: off");
                })
                .compact(60.0, |ui| {
                    ui.label("Snap");
                })
                .more(100.0, |ui| {
                    ui.label("Snap in More");
                }),
            )
            .show_with_end(ui, 60.0, |_| ());
        forms.borrow_mut().push(out.place("Snap"));
    });
    let mut seen = Vec::new();
    for w in [900.0, 220.0, 150.0] {
        forms.borrow_mut().clear();
        h.set_size(vec2(w, 300.0));
        h.run();
        seen.push(*forms.borrow().last().unwrap());
    }
    assert_eq!(seen, vec![Some(Place::Row), Some(Place::Icon), Some(Place::More)]);
}

// ------------------------------------------------------------------ D5

/// A header popover opened and closed from code, and which is open said.
#[test]
fn a_header_popover_opens_from_code() {
    let ask = Rc::new(Cell::new(None::<Option<&'static str>>));
    let open = Rc::new(RefCell::new(None::<String>));
    let (a2, o2) = (ask.clone(), open.clone());
    let mut h = harness(vec2(800.0, 300.0), move |ui| {
        let mut g = false;
        let pop = Popover::new("Overlays").check(PopCheck::new(&mut g, "Grid"));
        let mut header = EditorHeader::new(H).right(Item::popover(None, pop));
        if let Some(asked) = a2.take() {
            header = header.set_open(asked);
        }
        *o2.borrow_mut() = header.show(ui).open;
    });
    h.run();
    ask.set(Some(Some("Overlays")));
    h.run();
    assert_eq!(open.borrow().as_deref(), Some("Overlays"));
    assert!(h.query_by_label("Grid").is_some());
    ask.set(Some(None));
    h.run();
    assert!(open.borrow().is_none());
    assert!(h.query_by_label("Grid").is_none());
}

// ------------------------------------------------------------------ D9

/// A window's larger text (the conversation editor's) does not grow the
/// header: its row, its buttons and their widths stay the kit's.
#[test]
fn a_larger_text_does_not_grow_the_header() {
    let widths = RefCell::new(Vec::new());
    let body = Cell::new(0.0);
    for by in [1.0, 20.0 / 13.0] {
        let mut h = harness(vec2(800.0, 300.0), |ui| {
            let mut style = (**ui.style()).clone();
            for font in style.text_styles.values_mut() {
                font.size *= by;
            }
            style.spacing.interact_size.y *= by;
            ui.set_style(style);
            let out = EditorHeader::new(H)
                .left(Item::button(Icon::Save, "Save"))
                .left(Item::custom("Note", 100.0, |ui| {
                    body.set(egui::TextStyle::Body.resolve(ui.style()).size);
                    ui.label("a note");
                }))
                .show(ui);
            let save = out.response("Save").map(|r| r.rect.width()).unwrap_or_default();
            widths.borrow_mut().push((out.response.rect.height(), save, body.get()));
        });
        h.run();
    }
    let w = widths.borrow();
    let first = w[0];
    assert!(w.iter().all(|x| *x == first), "{w:?}");
    assert_eq!(first.0, mg_uikit::metrics::EDITOR_HEADER);
    assert_eq!(first.2, mg_uikit::metrics::BODY_SIZE, "a part of the caller's in the kit's size");
}
