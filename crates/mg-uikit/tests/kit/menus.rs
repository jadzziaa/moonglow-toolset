//! A menu's parts beyond its rows at the mock's measures (`.menu-pop`,
//! `.pop-h`, `.pop-sep`, `.menu-row .keys`): the popover's rows edge to
//! edge, headings and notes padded, separators 9 high, check boxes as
//! rows, keys lit with their row, a form's gaps and heights.

use std::cell::RefCell;

use egui::{Color32, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::menu::{MENU_FORM_GAP, MENU_SEPARATOR};
use mg_uikit::metrics::{MENU_ROW, POPOVER_RADIUS};
use mg_uikit::{Icon, IconButton, Theme, Tokens};

/// The shapes drawn in a frame, flattened.
fn shapes(h: &Harness<'_>) -> Vec<egui::Shape> {
    fn add(shape: &egui::Shape, to: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| add(s, to)),
            s => to.push(s.clone()),
        }
    }
    let mut all = Vec::new();
    h.output().shapes.iter().for_each(|s| add(&s.shape, &mut all));
    all
}

/// The color the words `text` were painted in.
fn text_color(h: &Harness<'_>, text: &str) -> Option<Color32> {
    shapes(h).into_iter().find_map(|s| match s {
        // (Not a row's keys as egui lays them out, drawn clear for the
        // kit to draw over.)
        egui::Shape::Text(t) if t.galley.text() == text => {
            let own = t.galley.job.sections.first().map(|s| s.format.color);
            let own = own.filter(|c| *c != Color32::PLACEHOLDER);
            let color = t.override_text_color.or(own).unwrap_or(t.fallback_color);
            Some(color).filter(|c| c.a() > 0)
        }
        _ => None,
    })
}

/// A heading, rows, a separator, a note and a check box's row in a menu
/// (a popover as the kit's menus have it): the heading padded 6 above and
/// 2 below its line, the rows right under it 24 high with no gap, the
/// separator 9 high with its 1 px `line-soft` line 4 down, the note a
/// 16 px line and 4 above and below (a row's 24), the check box's row 24 high with its box
/// where a row's icon goes (8 in, in a 16 px room); named by their words.
#[test]
fn a_menu_s_parts_are_at_the_mock_s_measures() {
    for theme in Theme::ALL {
        let t = theme.tokens();
        let rects = RefCell::new(Vec::new());
        let line = RefCell::new(0.0);
        let mut on = true;
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            *line.borrow_mut() = mg_uikit::metrics::LINE_HEIGHT;
            let frame = egui::Frame::menu(ui.style()).inner_margin(mg_uikit::menu::MENU_MARGIN);
            frame.show(ui, |ui| {
                ui.set_width(240.0);
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    mg_uikit::menu_rows(ui);
                    let mut r = rects.borrow_mut();
                    r.clear();
                    r.push(mg_uikit::menu_heading(ui, "On this area's tiles").rect);
                    r.push(ui.button("Grass (3)").rect);
                    r.push(ui.button("Stone (4)").rect);
                    r.push(mg_uikit::menu_separator(ui).rect);
                    r.push(mg_uikit::menu_note(ui, "None saved yet").rect);
                    r.push(mg_uikit::menu_check(ui, &mut on, "Tiles' Lights Too").rect);
                    r.push(ui.button("Save This Lighting…").rect);
                });
            });
        });
        h.run();
        let r = rects.borrow().clone();
        let line = *line.borrow();
        let [heading, grass, stone, sep, note, check, save] = r[..] else { panic!("{r:?}") };
        assert_eq!(heading.height(), 6.0 + line + 2.0, "{theme:?}: {heading:?}");
        assert_eq!(grass.top(), heading.bottom(), "{theme:?}");
        assert_eq!((grass.height(), stone.top()), (MENU_ROW, grass.bottom()));
        assert_eq!(sep.height(), MENU_SEPARATOR);
        assert_eq!((sep.top(), note.top()), (stone.bottom(), sep.bottom()));
        assert_eq!(note.height(), 4.0 + line + 4.0, "{theme:?}: {note:?}");
        assert_eq!((check.top(), check.height()), (note.bottom(), MENU_ROW));
        assert_eq!(save.top(), check.bottom());
        // The rows, separator and parts as wide as the menu.
        for p in [heading, grass, sep, note, check] {
            assert_eq!((p.left(), p.right()), (grass.left(), grass.right()), "{theme:?}: {p:?}");
        }
        let all = shapes(&h);
        // The separator's line: 1 px of `line-soft` across, 4 down.
        let lines: Vec<Rect> = all
            .iter()
            .filter_map(|s| match s {
                egui::Shape::Rect(r) if r.fill == t.line_soft && r.rect.height() == 1.0 => {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 1, "{theme:?}: {lines:?}");
        assert_eq!(lines[0].top(), sep.top() + 4.0);
        assert_eq!(lines[0].x_range(), sep.x_range());
        // The heading's and note's words 10 in, in `text-3`.
        assert_eq!(text_color(&h, "On this area's tiles"), Some(t.text_3));
        assert_eq!(text_color(&h, "None saved yet"), Some(t.text_3));
        // The check box's box: 14, centered in the 16 px room 8 in.
        let boxes: Vec<Rect> = all
            .iter()
            .filter_map(|s| match s {
                egui::Shape::Rect(r) if r.rect.size() == egui::vec2(14.0, 14.0) => Some(r.rect),
                _ => None,
            })
            .collect();
        assert_eq!(boxes.len(), 1, "{theme:?}: {boxes:?}");
        assert_eq!(boxes[0].center(), egui::pos2(check.left() + 8.0 + 8.0, check.center().y));
        // Found by their words: the check box as one, on.
        let node = h.get_by_label("Tiles' Lights Too");
        assert_eq!(node.accesskit_node().role(), egui::accesskit::Role::CheckBox);
        h.get_by_label("On this area's tiles");
        h.get_by_label("None saved yet");
    }
}

/// A check box's and a radio's rows work as the controls they are: a
/// click toggles the check box (and says it changed), a radio's click
/// chooses it; Space on the focused row too.
#[test]
fn a_menu_s_check_and_radio_rows_toggle_and_choose() {
    let state = RefCell::new((false, 0));
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Light);
        let (on, choice) = &mut *state.borrow_mut();
        mg_uikit::menu_check(ui, on, "Preview on screen");
        mg_uikit::menu::menu_radio_choice(ui, choice, 0, "1280 × 720");
        mg_uikit::menu::menu_radio_choice(ui, choice, 1, "1920 × 1080");
    });
    h.run();
    h.get_by_label("Preview on screen").click();
    h.run();
    h.get_by_label("1920 × 1080").click();
    h.run();
    assert_eq!(*state.borrow(), (true, 1));
    let radio = h.get_by_label("1920 × 1080");
    assert_eq!(radio.accesskit_node().role(), egui::accesskit::Role::RadioButton);
    h.get_by_label("Preview on screen").focus();
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert!(!state.borrow().0, "Space toggles it back");
}

/// A row's keys: in the weak text's color at rest, in `on-fill` while the
/// row is under the pointer (`.menu-row.hover .keys`: the weak text on
/// `accent-fill` measured 1.28:1 in Light); a disabled row's keys stay
/// weak under the pointer (no fill to stand on).
#[test]
fn a_menu_row_s_keys_are_lit_with_their_row() {
    for theme in Theme::ALL {
        let t: Tokens = theme.tokens();
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                ui.set_width(240.0);
                mg_uikit::menu_rows(ui);
                mg_uikit::menu_row_with_keys(ui, egui::Button::new("Undo"), "Ctrl+Z");
                ui.add_enabled_ui(false, |ui| {
                    mg_uikit::menu_row_with_keys(ui, egui::Button::new("Redo"), "Ctrl+Y")
                });
            });
        });
        h.run();
        assert_eq!(text_color(&h, "Ctrl+Z"), Some(t.text_3), "{theme:?}: at rest");
        h.get_by_label("Undo Ctrl+Z").hover();
        h.run();
        assert_eq!(text_color(&h, "Ctrl+Z"), Some(t.on_fill), "{theme:?}: lit");
        h.get_by_label("Redo Ctrl+Y").hover();
        h.run();
        assert_eq!(text_color(&h, "Ctrl+Z"), Some(t.text_3), "{theme:?}: unlit again");
        // (Weak, faded with its disabled row, as egui fades it.)
        let disabled = text_color(&h, "Ctrl+Y").unwrap();
        assert_ne!(disabled, t.on_fill, "{theme:?}: disabled stays weak");
        assert!(disabled.a() < 255, "{theme:?}: faded: {disabled:?}");
        // (Named by its words and keys, as egui names a row with
        // shortcut text.)
        h.get_by_label("Undo Ctrl+Z");
    }
}

/// A kit menu's popover (an icon button's, a context menu's): its rows
/// from one side of it to the other inside its 1 px border, 4 px of
/// padding above the first (its rounded corners' radius: a lit first
/// row's square fill stays inside them) and under the last; and a
/// separator in it 9 high.
#[test]
fn a_menu_s_rows_reach_the_popover_s_edges() {
    let rows = RefCell::new(Vec::new());
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let button = IconButton::new(Icon::MoreHorizontal, "Actions").icon_only(true);
        button.menu(ui, |ui| {
            let mut r = rows.borrow_mut();
            r.clear();
            r.push(ui.button("Move Up").rect);
            r.push(mg_uikit::menu_separator(ui).rect);
            r.push(ui.button("Delete Element").rect);
        });
        let target = ui.label("Right-click here");
        mg_uikit::context_menu(&target, |ui| {
            let mut r = rows.borrow_mut();
            r.clear();
            r.push(ui.button("Copy").rect);
            r.push(ui.button("Paste As Link").rect);
        });
    });
    h.run();
    for open in ["Actions", "Right-click here"] {
        match open {
            "Actions" => h.get_by_label(open).click(),
            _ => h.get_by_label(open).click_secondary(),
        }
        h.run();
        h.run();
        let r = rows.borrow().clone();
        let (first, last) = (r[0], r[r.len() - 1]);
        // (The popover: the area its frame was drawn in.)
        let whole = h.ctx.content_rect().width() - 1.0;
        let area = h
            .ctx
            .memory(|m| {
                let layers = m.areas().visible_layer_ids();
                let rects: Vec<Rect> =
                    layers.into_iter().filter_map(|l| m.area_rect(l.id)).collect();
                rects.into_iter().find(|a| a.contains_rect(first) && a.width() < whole)
            })
            .unwrap_or_else(|| panic!("{open}: the popover"));
        assert_eq!(first.left(), area.left() + 1.0, "{open}: {first:?} in {area:?}");
        assert_eq!(first.right(), area.right() - 1.0, "{open}: {first:?} in {area:?}");
        assert_eq!(first.top(), area.top() + 1.0 + 4.0, "{open}: {first:?} in {area:?}");
        assert_eq!(last.bottom(), area.bottom() - 1.0 - 4.0, "{open}: {last:?} in {area:?}");
        assert!(first.top() - area.top() >= f32::from(POPOVER_RADIUS), "inside the corners");
        if open == "Actions" {
            assert_eq!(r[1].height(), MENU_SEPARATOR);
        }
        h.key_press(egui::Key::Escape);
        h.run();
    }
}

/// A menu's form under a window's larger text (the context's style the
/// window's while it is drawn, as Options › Conversation Editor makes it)
/// keeps the window's taller controls, not a control's 22; and the
/// `.pop-body` gap between its parts.
#[test]
fn a_menu_s_form_keeps_a_larger_text_s_height() {
    let heights = RefCell::new(Vec::new());
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        let usual = ui.ctx().global_style();
        let mut large = (*usual).clone();
        for font in large.text_styles.values_mut() {
            font.size *= 20.0 / 13.0;
        }
        large.spacing.interact_size.y = 34.0;
        ui.ctx().set_global_style(large.clone());
        ui.set_style(large);
        mg_uikit::menu_button(ui, "Rename…", |ui| {
            mg_uikit::menu_form(ui, |ui| {
                let mut h = heights.borrow_mut();
                h.clear();
                h.push(ui.button("Rename").rect);
                h.push(ui.button("Cancel").rect);
            });
        });
        ui.ctx().set_global_style(usual);
    });
    h.run();
    h.get_by_label("Rename…").click();
    h.run();
    h.run();
    let r = heights.borrow().clone();
    assert_eq!(r[0].height(), 34.0, "{r:?}");
    assert_eq!(r[1].top() - r[0].bottom(), MENU_FORM_GAP, "{r:?}");
}

/// A form's menu stays open when a field in it is clicked into, and
/// closes with a click outside (a menu of rows closes with any click).
#[test]
fn a_form_s_menu_stays_open_for_its_fields() {
    let mut name = String::from("Dusk");
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        mg_uikit::form_menu_button(ui, "Rename…", |ui| {
            let label = mg_uikit::field_label(ui, "Name");
            ui.add(mg_uikit::text_field(&mut name)).labelled_by(label.id);
        });
        ui.add_space(200.0);
        let _ = ui.label("Elsewhere");
    });
    h.run();
    h.get_by_label("Rename…").click();
    h.run();
    h.get_by_label("Name").click();
    h.run();
    h.run();
    assert!(h.query_by_label("Name").is_some(), "open after a click in its field");
    h.get_by_label("Elsewhere").click();
    h.run();
    h.run();
    assert!(h.query_by_label("Name").is_none(), "closed by a click outside");
}
