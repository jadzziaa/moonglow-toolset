//! The crate's widgets as egui's own behave: found by their words,
//! clicked, reached with Tab and pressed with the keyboard, disabled.

use std::cell::Cell;

use egui::Color32;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::{Icon, Theme, Tokens};

#[test]
fn an_icon_button_is_named_by_its_tooltip_and_clicks() {
    let clicks = Cell::new(0);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        if mg_uikit::icon_button(ui, Icon::Undo, "Undo the last change").clicked() {
            clicks.set(clicks.get() + 1);
        }
    });
    h.run();
    h.get_by_label("Undo the last change").click();
    h.run();
    assert_eq!(clicks.get(), 1);
    // Reached with Tab, pressed with Enter and with Space.
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("Undo the last change").is_focused());
    h.key_press(egui::Key::Enter);
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert_eq!(clicks.get(), 3);
}

#[test]
fn an_icon_toggle_says_it_is_on() {
    let on = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        let mut value = on.get();
        if mg_uikit::icon_toggle(ui, &mut value, Icon::Grid, "Show the grid").changed() {
            on.set(value);
        }
    });
    h.run();
    h.get_by_label("Show the grid").click();
    h.run();
    assert!(on.get());
    let toggled = h.get_by_label("Show the grid").accesskit_node().toggled();
    assert_eq!(toggled, Some(egui::accesskit::Toggled::True));
    h.get_by_label("Show the grid").click();
    h.run();
    assert!(!on.get());
}

/// An icon and words: the words alone name it.
#[test]
fn an_icon_and_words_are_named_by_the_words() {
    let clicks = Cell::new(0);
    let fog = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Light);
        if mg_uikit::icon_text_button(ui, Icon::Save, "Save").clicked() {
            clicks.set(clicks.get() + 1);
        }
        let mut value = fog.get();
        if mg_uikit::icon_text_toggle(ui, &mut value, Icon::Fog, "Fog").changed() {
            fog.set(value);
        }
    });
    h.run();
    let save = h.get_by_label("Save");
    assert_eq!(save.accesskit_node().role(), egui::accesskit::Role::Button);
    save.click();
    h.run();
    assert_eq!(clicks.get(), 1);
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("Save").is_focused());
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(clicks.get(), 2);
    h.get_by_label("Fog").click();
    h.run();
    assert!(fog.get());
    assert!(h.query_by_label("save").is_none(), "the icon's name is not in the label");
}

/// Disabled: said to be, not clicked, and the tooltip still there.
#[test]
fn a_disabled_icon_button() {
    let clicks = Cell::new(0);
    let mut h = Harness::new_ui(|ui| {
        ui.add_enabled_ui(false, |ui| {
            if mg_uikit::icon_button(ui, Icon::Cut, "Cut").clicked() {
                clicks.set(clicks.get() + 1);
            }
            if mg_uikit::icon_text_button(ui, Icon::Compile, "Compile").clicked() {
                clicks.set(clicks.get() + 1);
            }
        });
    });
    h.run();
    for name in ["Cut", "Compile"] {
        let node = h.get_by_label(name);
        assert!(node.accesskit_node().is_disabled(), "{name}");
        node.click();
        h.run();
    }
    assert_eq!(clicks.get(), 0);
}

/// `icon` and `Icon::paint` paint strokes in the color asked for.
#[test]
fn an_icon_paints_in_the_text_color() {
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Dark);
        mg_uikit::icon(ui, Icon::Info);
    });
    h.run();
    let text = Theme::Dark.tokens().text;
    let mut painted = 0;
    for s in &h.output().shapes {
        match &s.shape {
            egui::Shape::Path(p) => {
                assert_eq!(p.stroke.color, egui::epaint::ColorMode::Solid(text));
                painted += 1;
            }
            egui::Shape::Circle(c) => assert_eq!(c.fill, text),
            _ => {}
        }
    }
    assert!(painted >= 3, "the circle, the bar and the dot: {painted}");
}

/// A text field is as high as the other controls (22 px), in both themes.
#[test]
fn a_text_field_is_as_high_as_a_control() {
    for theme in Theme::ALL {
        let mut text = String::from("Aribeth");
        let height = std::cell::Cell::new(0.0);
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            height.set(ui.add(mg_uikit::text_field(&mut text)).rect.height());
        });
        h.run();
        assert_eq!(height.get(), mg_uikit::metrics::CONTROL_HEIGHT, "{theme:?}");
    }
}

/// A row (a selectable label, drawn without a frame at rest) is as wide
/// at rest as hovered or chosen, in the theme and in a menu's style: it
/// kept moving as the pointer went over it, and wrapped to the next row
/// and back where it was the last of a row.
#[test]
fn a_row_keeps_its_width_hovered_and_chosen() {
    for menu in [false, true] {
        let widths = std::cell::RefCell::new(Vec::new());
        let chosen = Cell::new(false);
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), Theme::Dark);
            if menu {
                mg_uikit::menu_rows(ui);
            }
            let r = ui.selectable_label(chosen.get(), "Waypoints (40)");
            widths.borrow_mut().push(r.rect.width());
        });
        h.run();
        let rest = *widths.borrow().last().unwrap();
        h.get_by_label("Waypoints (40)").hover();
        h.run();
        assert_eq!(*widths.borrow().last().unwrap(), rest, "hovered (menu: {menu})");
        chosen.set(true);
        h.run();
        assert_eq!(*widths.borrow().last().unwrap(), rest, "chosen (menu: {menu})");
    }
}

/// The rectangles drawn in a frame, with their fills and borders.
fn rects(h: &Harness<'_>) -> Vec<egui::epaint::RectShape> {
    fn add(shape: &egui::Shape, to: &mut Vec<egui::epaint::RectShape>) {
        match shape {
            egui::Shape::Rect(r) => to.push(r.clone()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| add(s, to)),
            _ => {}
        }
    }
    let mut all = Vec::new();
    h.output().shapes.iter().for_each(|s| add(&s.shape, &mut all));
    all
}

/// The check box: found by its words as a check box, toggled by a click
/// and by Space, saying whether it is on; on, its box is `accent-fill`;
/// off, an `inset` well bordered in `text-3`; disabled and on, the
/// `accent-fill-off` token; mixed, said as neither.
#[test]
fn a_check_box_is_named_toggles_and_shows_its_state() {
    use mg_uikit::Controls as _;
    for theme in Theme::ALL {
        let t = theme.tokens();
        let on = Cell::new(false);
        let (enabled, mixed) = (Cell::new(true), Cell::new(false));
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            let mut value = on.get();
            ui.add_enabled_ui(enabled.get(), |ui| {
                ui.add(mg_uikit::Checkbox::new(&mut value, "Plot").indeterminate(mixed.get()))
            });
            on.set(value);
        });
        h.run();
        let node = h.get_by_label("Plot");
        assert_eq!(node.accesskit_node().role(), egui::accesskit::Role::CheckBox);
        assert_eq!(node.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
        assert!(rects(&h).iter().any(|r| r.fill == t.inset && r.stroke.color == t.text_3));
        node.click();
        h.run();
        assert!(on.get(), "{theme:?}");
        let node = h.get_by_label("Plot");
        assert_eq!(node.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
        // (The pointer still on it: hovered.)
        assert!(rects(&h).iter().any(|r| r.fill == t.accent_fill_hover));
        h.key_press(egui::Key::Tab);
        h.run();
        assert!(h.get_by_label("Plot").is_focused());
        h.key_press(egui::Key::Space);
        h.run();
        assert!(!on.get(), "Space toggles it");
        h.key_press(egui::Key::Space);
        h.run();
        mixed.set(true);
        h.run();
        let mixed_state = Some(egui::accesskit::Toggled::Mixed);
        assert_eq!(h.get_by_label("Plot").accesskit_node().toggled(), mixed_state);
        mixed.set(false);
        enabled.set(false);
        h.run();
        assert!(h.get_by_label("Plot").accesskit_node().is_disabled());
        assert!(rects(&h).iter().any(|r| r.fill == t.accent_fill_off), "{theme:?}");
        // The ui's own stays egui's where no theme was applied.
        let mut plain = Harness::new_ui(|ui| {
            let mut v = true;
            ui.add(mg_uikit::Checkbox::new(&mut v, "Static"));
            let _ = ui.check_box(&mut v, "Useable");
        });
        plain.run();
        assert!(rects(&plain).iter().all(|r| r.fill != Tokens::DARK.accent_fill));
    }
}

/// The radio: named, said on or off, chosen by a click and with the
/// keyboard; on, an `on-fill` dot in an `accent-fill` ring.
#[test]
fn a_radio_is_named_and_chooses() {
    use mg_uikit::Controls as _;
    let fast = Cell::new(true);
    let mut h = Harness::new_ui(|ui| {
        mg_uikit::apply(ui.ctx(), Theme::Light);
        let mut v = fast.get();
        ui.radio_choice(&mut v, true, "Fast");
        ui.radio_choice(&mut v, false, "High");
        fast.set(v);
    });
    h.run();
    let high = h.get_by_label("High");
    assert_eq!(high.accesskit_node().role(), egui::accesskit::Role::RadioButton);
    assert_eq!(high.accesskit_node().toggled(), Some(egui::accesskit::Toggled::False));
    let dot = |h: &Harness<'_>, c: Color32| {
        h.output().shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Circle(k) if k.fill == c))
    };
    assert!(dot(&h, Tokens::LIGHT.accent_fill) && dot(&h, Tokens::LIGHT.on_fill));
    high.click();
    h.run();
    assert!(!fast.get());
    assert_eq!(
        h.get_by_label("High").accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    h.get_by_label("Fast").focus();
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert!(fast.get(), "Space chooses it");
}

/// A focused text field's border is `accent` (egui's is the selection's
/// stroke, white in the dark theme); at rest it is not; a field no longer
/// drawn gives the keys up (the pass that paints the border asked egui for
/// the field's rectangle once, which kept the keys on it).
#[test]
fn a_focused_field_is_bordered_in_accent_and_lets_go_when_gone() {
    for theme in Theme::ALL {
        let t = theme.tokens();
        let shown = Cell::new(true);
        let mut text = String::from("M1Q1FAribeth");
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            if shown.get() {
                ui.add(mg_uikit::text_field(&mut text));
            }
        });
        h.run();
        let accent = |h: &Harness<'_>| rects(h).iter().any(|r| r.stroke.color == t.accent);
        assert!(!accent(&h), "at rest");
        h.get_by_role(egui::accesskit::Role::TextInput).focus();
        h.run();
        assert!(accent(&h), "{theme:?}: focused");
        assert!(rects(&h).iter().all(|r| r.stroke.color != t.on_fill || t.on_fill == t.accent));
        shown.set(false);
        h.run();
        h.run();
        assert_eq!(h.ctx.memory(|m| m.focused()), None, "the keys let go");
    }
}

/// A tooltip is filled with `tooltip-bg`, not the popups' `panel`.
#[test]
fn a_tooltip_has_its_own_fill() {
    for theme in Theme::ALL {
        let mut h = Harness::new_ui(|ui| {
            mg_uikit::apply(ui.ctx(), theme);
            let _ = ui.button("Fog").on_hover_text("Show the area's fog");
        });
        h.run();
        h.get_by_label("Fog").hover();
        // (Tooltips wait for the pointer to rest.)
        for _ in 0..90 {
            h.step();
        }
        assert!(h.query_by_label("Show the area's fog").is_some(), "the tooltip shows");
        let t = theme.tokens();
        assert!(rects(&h).iter().any(|r| r.fill == t.tooltip_bg), "{theme:?}");
    }
}
