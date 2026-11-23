//! Colored icons (Options › General › Colored icons, 5.2.0, a builder's
//! ask, the owner's ruling): off, every kind's icon in the Module panel,
//! the Outliner and the Palette is `text-2`, as the redesign drew them;
//! on, each in its kind's hue (`mg_uikit::hues`), from the next frame,
//! with no restart, in the dark and the light theme; the Palette's type
//! that is chosen keeps `on-fill`.

use crate::common::*;
use mg_uikit::Icon;
use mg_uikit::hues::kind_hue;

/// Frames until `done`, for ten seconds at most (an area's view asks for
/// frames as it draws).
fn until(h: &mut Harness<'_, Moonglow>, what: &str, done: impl Fn(&Harness<'_, Moonglow>) -> bool) {
    let started = std::time::Instant::now();
    while !done(h) {
        assert!(started.elapsed().as_secs() < 10, "never: {what}");
        h.step();
    }
}

/// The colors the last frame stroked `icon` in, where it was drawn within
/// `within` (each place it was drawn, the colors of the strokes inside it).
fn icon_colors(
    h: &Harness<'_, Moonglow>,
    icon: Icon,
    within: egui::Rect,
) -> Vec<Vec<egui::Color32>> {
    fn walk(s: &egui::Shape, at: egui::Rect, out: &mut Vec<egui::Color32>) {
        match s {
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, at, out)),
            egui::Shape::Path(p) if at.contains_rect(p.visual_bounding_rect()) => {
                if let egui::epaint::ColorMode::Solid(c) = p.stroke.color {
                    out.push(c);
                }
            }
            _ => {}
        }
    }
    let places: Vec<egui::Rect> = mg_uikit::icons_drawn(&h.ctx)
        .into_iter()
        .filter(|(i, r)| *i == icon && within.contains_rect(*r))
        .map(|(_, r)| r.expand(1.0))
        .collect();
    places
        .into_iter()
        .map(|at| {
            let mut out = Vec::new();
            h.output().shapes.iter().for_each(|c| walk(&c.shape, at, &mut out));
            out
        })
        .collect()
}

/// Whether every place `icon` was drawn within `within` (one at least) is
/// stroked in `color` alone.
fn all_in(
    h: &Harness<'_, Moonglow>,
    icon: Icon,
    within: egui::Rect,
    color: egui::Color32,
) -> Result<(), String> {
    let seen = icon_colors(h, icon, within);
    let ok =
        !seen.is_empty() && seen.iter().all(|c| !c.is_empty() && c.iter().all(|c| *c == color));
    if ok { Ok(()) } else { Err(format!("{icon:?} wants {color:?}, drawn {seen:?}")) }
}

/// The Module panel's Areas group and the area's kinds in the Outliner
/// (Left Dock), and the Palette's row of types (Right Dock): `text-2`
/// while off, the kind's hue while on, set at once in the running window,
/// in both themes, and `text-2` again off; the type chosen (Placeables)
/// `on-fill` throughout.
#[test]
fn kind_icons_take_their_hue_at_once_when_the_option_is_on() {
    let Some((app, rs, area)) = area_app("colored-icons", "ttr01", None, 4, None, &|_, _, _| ())
    else {
        return;
    };
    let mut h = area_gpu_harness(app, rs, 1600.0, 1000.0);
    until(&mut h, "the area in front", |h| h.state().area_views.contains_key(&area));
    show_palette(h.state_mut());
    until(&mut h, "the Outliner and the Palette", |h| {
        h.query_by_label(mg_ui::OUTLINER).is_some() && h.query_by_label("Palette types").is_some()
    });
    palette_type(&mut h, "Placeables");
    settle(&mut h);
    // (The Module panel's tree and the Outliner under it: the Left Dock;
    // the Palette's row of types: the band of the Right Dock its first,
    // Tiles, is drawn in, above the palette's pictures.)
    let left = left_dock_rect(&h);
    let right = right_dock_rect(&h);
    let tiles = mg_uikit::icons_drawn(&h.ctx)
        .into_iter()
        .find(|(i, r)| *i == Icon::Tile && right.contains_rect(*r))
        .expect("the Palette's Tiles")
        .1;
    let types = right.with_min_y(tiles.top() - 4.0).with_max_y(tiles.bottom() + 4.0);
    let kinds = [Icon::Creature, Icon::Door, Icon::Encounter, Icon::Trigger, Icon::Waypoint];
    for light in [false, true] {
        h.state_mut().settings.light_theme = light;
        let theme = if light { mg_uikit::Theme::Light } else { mg_uikit::Theme::Dark };
        let t = theme.tokens();
        for on in [false, true, false] {
            h.state_mut().settings.colored_icons = on;
            // (Drawn with it from the frame it is set in, or the next.)
            let first = if on { kind_hue(Icon::Area, theme).unwrap() } else { t.text_2 };
            until(&mut h, "the Areas group's icon in its color", |h| {
                all_in(h, Icon::Area, left, first).is_ok()
            });
            assert_eq!(mg_uikit::hues::are_colored(&h.ctx), on);
            let want = |i: Icon| if on { kind_hue(i, theme).unwrap() } else { t.text_2 };
            all_in(&h, Icon::Area, left, want(Icon::Area)).unwrap();
            for kind in kinds {
                all_in(&h, kind, left, want(kind)).unwrap();
                all_in(&h, kind, types, want(kind)).unwrap();
            }
            all_in(&h, Icon::Placeable, types, t.on_fill).unwrap();
            // (Not kinds: the palette's Tiles keeps `text-2`.)
            all_in(&h, Icon::Tile, types, t.text_2).unwrap();
        }
    }
}
