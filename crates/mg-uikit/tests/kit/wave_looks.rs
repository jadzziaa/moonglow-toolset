//! The looks the UI/UX waves changed in the kit, held to what they are:
//! one focus ring on each focused control, the one on an `accent-fill`
//! ground in `on-fill` (where `accent` hardly shows in the light theme),
//! a switch that is off, a color swatch button's states, a gallery tile's
//! hover rim outside the tile, the compass's N.

use egui::epaint::RectShape;
use egui::{Color32, Rect, Shape, Stroke, StrokeKind, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use mg_uikit::tokens::contrast;
use mg_uikit::{Icon, IconButton, Theme, Tokens};

use crate::common::{colored_texts, rects, shapes};

/// A harness of `add` under `theme`, its control `name` given the
/// keyboard's focus.
fn focused(theme: Theme, name: &str, add: impl FnMut(&mut egui::Ui) + 'static) -> Harness<'static> {
    let mut add = add;
    let mut h = Harness::builder().with_size(vec2(300.0, 200.0)).build_ui(move |ui| {
        mg_uikit::apply(ui.ctx(), theme);
        add(ui);
    });
    h.run();
    h.get_by_label(name).focus();
    h.run();
    h.run();
    h
}

/// The rings (rectangles stroked, whatever their fill) drawn within `at`.
fn rings(h: &Harness<'_>, at: Rect) -> Vec<RectShape> {
    rects(h)
        .into_iter()
        .filter(|r| r.stroke.width > 0.0 && r.stroke.color.a() > 0)
        .filter(|r| at.expand(0.6).contains_rect(r.rect))
        .collect()
}

/// The fill of the widest rectangle filled within `at` (a control's own
/// ground).
fn ground(h: &Harness<'_>, at: Rect) -> Color32 {
    rects(h)
        .into_iter()
        .filter(|r| r.fill.a() > 0 && at.expand(0.6).contains_rect(r.rect))
        .max_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
        .map(|r| r.fill)
        .expect("a ground")
}

/// A focused control on `accent-fill`: one ring, 1.5 `on-fill` 1.5 inside
/// its edge, at least 3:1 on the ground it is drawn on, in both themes.
fn one_on_fill_ring(h: &Harness<'_>, name: &str, theme: Theme) {
    let t = theme.tokens();
    let at = h.get_by_label(name).rect();
    let rings = rings(h, at);
    assert_eq!(rings.len(), 1, "{theme:?} {name}: one ring, not {rings:?}");
    let ring = &rings[0];
    assert_eq!(ring.stroke, Stroke::new(1.5, t.on_fill), "{theme:?} {name}");
    assert!((ring.rect.min - at.min).length() > 1.0, "{theme:?} {name}: inside its edge");
    let under = ground(h, at);
    assert!(under == t.accent_fill || under == t.accent_fill_hover, "{theme:?} {name}: {under:?}");
    let r = contrast(t.on_fill, under);
    assert!(r >= 3.0, "{theme:?} {name}: the ring {r:.2}:1 on its ground");
}

/// A focused control off `accent-fill`: one ring, 1 px `accent`.
fn one_accent_ring(h: &Harness<'_>, name: &str, theme: Theme) {
    let t = theme.tokens();
    let at = h.get_by_label(name).rect();
    let rings = rings(h, at);
    assert_eq!(rings.len(), 1, "{theme:?} {name}: one ring, not {rings:?}");
    assert_eq!(rings[0].stroke, Stroke::new(1.0, t.accent), "{theme:?} {name}");
}

/// An icon button selected (an icon toggle on) with the keyboard's focus
/// has one ring, not the accent ring of a pressed button inside the focus
/// ring as well; one off has the accent ring alone.
#[test]
fn a_focused_icon_button_has_one_ring() {
    for theme in Theme::ALL {
        let h = focused(theme, "Gallery", |ui| {
            ui.add(IconButton::new(Icon::Grid, "Gallery").icon_only(true).selected(true));
        });
        one_on_fill_ring(&h, "Gallery", theme);
        let h = focused(theme, "Snap", |ui| {
            let mut on = true;
            let _ = mg_uikit::icon_toggle(ui, &mut on, Icon::Magnet, "Snap");
        });
        one_on_fill_ring(&h, "Snap", theme);
        let h = focused(theme, "Undo", |ui| {
            ui.add(IconButton::new(Icon::Undo, "Undo").icon_only(true).selected(false));
        });
        one_accent_ring(&h, "Undo", theme);
    }
}

/// A focused primary button (and a toggle on) keeps its `accent-fill` and
/// is ringed `on-fill` inside, a plain one `accent` at its edge.
#[test]
fn a_focused_kit_button_is_ringed_for_its_ground() {
    use mg_uikit::{ButtonLook, KitButton};
    for theme in Theme::ALL {
        let h = focused(theme, "OK", |ui| {
            ui.add(KitButton::new(None, "OK").look(ButtonLook::Primary));
        });
        one_on_fill_ring(&h, "OK", theme);
        let h = focused(theme, "Fog", |ui| {
            ui.add(KitButton::new(Some(Icon::Fog), "Fog").toggle(true));
        });
        one_on_fill_ring(&h, "Fog", theme);
        let h = focused(theme, "Cancel", |ui| {
            ui.add(KitButton::new(None, "Cancel"));
        });
        one_accent_ring(&h, "Cancel", theme);
    }
}

/// A page list's row, a page rail's page and a tool shelf's tool, on and
/// focused, are ringed `on-fill`; off and focused, `accent`.
#[test]
fn focused_pages_and_tools_are_ringed_for_their_ground() {
    use mg_uikit::pages::{PageList, PageRail};
    use mg_uikit::shelf::{Tool, ToolShelf};
    let list = |ui: &mut egui::Ui| {
        let _ = PageList::new("list")
            .page(Icon::Info, "List on")
            .page(Icon::Lock, "List off")
            .width(120.0)
            .show(ui, &mut 0);
    };
    let rail = |ui: &mut egui::Ui| {
        let _ = PageRail::new("rail")
            .page(Icon::Info, "Rail on")
            .page(Icon::Lock, "Rail off")
            .show(ui, &mut 0);
    };
    let shelf = |ui: &mut egui::Ui| {
        ToolShelf::new()
            .tool(Tool::new(Icon::Select, "Select Objects").on(true))
            .tool(Tool::new(Icon::Paint, "Paint"))
            .show(ui);
    };
    for theme in Theme::ALL {
        one_on_fill_ring(&focused(theme, "List on", list), "List on", theme);
        one_accent_ring(&focused(theme, "List off", list), "List off", theme);
        one_on_fill_ring(&focused(theme, "Rail on", rail), "Rail on", theme);
        one_accent_ring(&focused(theme, "Rail off", rail), "Rail off", theme);
        one_on_fill_ring(&focused(theme, "Select Objects", shelf), "Select Objects", theme);
        one_accent_ring(&focused(theme, "Paint", shelf), "Paint", theme);
    }
}

/// A selected tree row's eye, focused, is lit `accent-fill-hover` and
/// ringed `on-fill` (at least 3:1 on it); an unselected row's, `raised`
/// and `accent`.
#[test]
fn a_focused_eye_is_ringed_for_its_row() {
    use mg_uikit::tree::TreeRow;
    for theme in Theme::ALL {
        let t = theme.tokens();
        for (selected, fill, ring) in
            [(true, t.accent_fill_hover, t.on_fill), (false, t.raised, t.accent)]
        {
            let h = focused(theme, "Show Halion", move |ui| {
                mg_uikit::list_rows(ui);
                let _ = TreeRow::new("Halion")
                    .selected(selected)
                    .eye(true, "Show Halion", "Shown")
                    .show(ui);
            });
            let at = h.get_by_label("Show Halion").rect();
            let found = rects(&h).into_iter().filter(|r| at.expand(0.6).contains_rect(r.rect));
            let found: Vec<RectShape> = found.collect();
            assert!(found.iter().any(|r| r.fill == fill), "{theme:?} {selected}: {found:?}");
            let rings: Vec<&RectShape> = found.iter().filter(|r| r.stroke.width > 0.0).collect();
            assert_eq!(rings.len(), 1, "{theme:?} {selected}: {rings:?}");
            assert_eq!(rings[0].stroke.color, ring, "{theme:?} {selected}");
            assert!(
                contrast(ring, fill) >= 3.0,
                "{theme:?} {selected}: the ring on the eye's fill"
            );
        }
    }
}

/// A popover's switch that is off: its track `switch-off`, its knob
/// `switch-knob` (the proposal's section 4.2: dark `raised-hover` and
/// `text-2`, light `#a9b0b9` and white, kept by the owner's ruling though
/// the light knob is 2.2:1 on its track).
#[test]
fn a_switch_that_is_off_has_the_proposal_s_colors() {
    for theme in Theme::ALL {
        let t = theme.tokens();
        let mut h = Harness::builder().with_size(vec2(300.0, 120.0)).build_ui(move |ui| {
            mg_uikit::apply(ui.ctx(), theme);
            let mut off = false;
            mg_uikit::popover::Popover::new("Switch").switch(&mut off, "Off").show_in_place(ui);
        });
        h.run();
        let all = shapes(&h);
        let track = all.iter().any(|s| matches!(s, Shape::Rect(r) if r.fill == t.switch_off));
        assert!(track, "{theme:?}: the track");
        let knob = all.iter().any(|s| matches!(s, Shape::Circle(c) if c.fill == t.switch_knob));
        assert!(knob, "{theme:?}: the knob");
        let (dark, light) = (Tokens::DARK, Tokens::LIGHT);
        assert_eq!((dark.switch_off, dark.switch_knob), (dark.raised_hover, dark.text_2));
        assert_eq!(light.switch_off, Color32::from_rgb(0xa9, 0xb0, 0xb9));
        assert_eq!(light.switch_knob, Color32::WHITE);
    }
}

/// A color swatch button: `raised`, `raised-hover` hovered or open; its
/// color 3 in, lined `swatch-line`; open or focused, a 1 px `accent`
/// ring on the button's own edge, inside it (not outside, where a tight
/// grid's neighbor covers it).
#[test]
fn a_swatch_button_shows_hover_open_and_focus_inside_itself() {
    use mg_uikit::SwatchLook;
    for theme in Theme::ALL {
        let t = theme.tokens();
        let looks = [
            (SwatchLook::default(), t.raised, false),
            (SwatchLook { hovered: true, ..Default::default() }, t.raised_hover, false),
            (SwatchLook { open: true, ..Default::default() }, t.raised_hover, true),
            (SwatchLook { focused: true, ..Default::default() }, t.raised, true),
        ];
        for (look, fill, ringed) in looks {
            let button = Rect::from_min_size(egui::pos2(20.0, 20.0), vec2(36.0, 22.0));
            let mut h = Harness::builder().with_size(vec2(100.0, 60.0)).build_ui(move |ui| {
                mg_uikit::apply(ui.ctx(), theme);
                let inner =
                    mg_uikit::paint_swatch_button(ui.painter(), &t, button, look, |p, r| {
                        p.rect_filled(r, 2.0, Color32::from_rgb(200, 147, 106));
                    });
                assert_eq!(inner, button.shrink(mg_uikit::SWATCH_PADDING));
            });
            h.run();
            let all = rects(&h);
            let own = all.iter().find(|r| r.rect == button && r.fill.a() > 0).expect("its fill");
            assert_eq!(own.fill, fill, "{theme:?} {look:?}");
            let line = all.iter().find(|r| r.stroke.color == t.swatch_line).expect("the line");
            assert_eq!(line.rect, button.shrink(mg_uikit::SWATCH_PADDING), "{theme:?}");
            let ring = all.iter().find(|r| r.stroke == Stroke::new(1.0, t.accent));
            assert_eq!(ring.is_some(), ringed, "{theme:?} {look:?}");
            if let Some(ring) = ring {
                assert_eq!((ring.rect, ring.stroke_kind), (button, StrokeKind::Inside));
            }
        }
    }
}

/// A gallery tile's rim stands outside the tile (the owner's ruling);
/// hovered, it is at least 2.8:1 on the panel it stands on in the light
/// theme (`text-off`; `raised-hover` was 1.5:1), and weaker than a
/// selected tile's `text-3`.
#[test]
fn a_hovered_tile_s_rim_shows_outside_it_in_the_light_theme() {
    use mg_uikit::gallery::{CellState, GalleryCell, Picture};
    for theme in Theme::ALL {
        let t = theme.tokens();
        let mut h = Harness::builder().with_size(vec2(200.0, 200.0)).build_ui(move |ui| {
            mg_uikit::apply(ui.ctx(), theme);
            let (rect, r) = ui.allocate_exact_size(vec2(128.0, 176.0), egui::Sense::click());
            let state = CellState { hovered: true, ..Default::default() };
            GalleryCell::new("Barrel", Picture::Making).paint(
                ui,
                rect,
                vec2(128.0, 128.0),
                state,
                &r,
            );
        });
        h.run();
        let rim = rects(&h).into_iter().find(|r| r.stroke.width == 1.0).expect("the rim");
        assert_eq!(rim.stroke_kind, StrokeKind::Outside, "{theme:?}: outside the tile");
        let expected = match theme {
            Theme::Light => t.text_off,
            Theme::Dark => t.raised_hover,
        };
        assert_eq!(rim.stroke.color, expected, "{theme:?}");
        if theme == Theme::Light {
            let r = contrast(rim.stroke.color, t.panel);
            assert!(r >= 2.8, "light: {r:.2}:1 on the panel");
            assert!(contrast(rim.stroke.color, t.inset) >= 3.0, "light: on a well");
            assert!(r < contrast(t.text_3, t.panel), "weaker than selected");
        }
    }
}

/// The compass's N is `text`, as the proposal's light board has the
/// labels: it stands on the disc (`overlay`), where `on-fill` (white) was
/// white on near-white in the light theme.
#[test]
fn the_compass_s_n_reads_on_its_disc() {
    for theme in Theme::ALL {
        let t = theme.tokens();
        let mut h = Harness::builder().with_size(vec2(120.0, 120.0)).build_ui(move |ui| {
            mg_uikit::apply(ui.ctx(), theme);
            mg_uikit::overlay::paint_compass(ui.painter(), &t, egui::pos2(60.0, 60.0), 0.3);
        });
        h.run();
        let n = colored_texts(&h).into_iter().find(|(s, _)| s == "N").expect("the N");
        assert_eq!(n.1, t.text, "{theme:?}");
        // (The disc is `overlay`, mostly opaque: the N on its color.)
        let disc = t.overlay.to_opaque();
        assert!(contrast(n.1, disc) >= 4.5, "{theme:?}: the N on its disc");
    }
}
