//! The icon widgets as egui's own buttons behave: found by their words,
//! clicked, reached with Tab and pressed with the keyboard, disabled.

use std::cell::Cell;

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use mg_uikit::{Icon, Theme};

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
