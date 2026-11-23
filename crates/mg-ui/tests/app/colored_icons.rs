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

/// The colors of the strokes the last frame painted inside `within` (the
/// icons a view paints itself, not as the kit's recorded widgets).
fn strokes_in(h: &Harness<'_, Moonglow>, within: egui::Rect) -> Vec<egui::Color32> {
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
    let mut out = Vec::new();
    h.output().shapes.iter().for_each(|c| walk(&c.shape, within, &mut out));
    out
}

/// Whether any of `colors` is a kind's hue in the dark theme.
fn any_hue(colors: &[egui::Color32]) -> bool {
    let hues: Vec<_> = mg_uikit::hues::KIND_HUES.iter().map(|k| k.1).collect();
    colors.iter().any(|c| hues.contains(c))
}

/// Turns Colored icons `on` in the running window, and steps until `ok`
/// holds of the strokes inside `within` (ten seconds at most).
fn colored_until(
    h: &mut Harness<'_, Moonglow>,
    on: bool,
    within: impl Fn(&Harness<'_, Moonglow>) -> egui::Rect,
    ok: impl Fn(&[egui::Color32]) -> bool,
) -> Vec<egui::Color32> {
    h.state_mut().settings.colored_icons = on;
    let started = std::time::Instant::now();
    loop {
        h.step();
        let seen = strokes_in(h, within(h));
        if ok(&seen) {
            return seen;
        }
        assert!(started.elapsed().as_secs() < 10, "on {on}: never; drawn {seen:?}");
    }
}

/// The Resources panel: a creature blueprint's row (`nw_bandit001.utc`,
/// the game's) shows its icon in the creature's hue while on, `text-2`
/// while off (and no hue at all).
#[test]
fn the_resources_panel_s_rows_take_their_kind_s_hue() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-colored-resources");
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.settings.no_last_area = true;
    app.open_module(&path);
    show_resources(&mut app);
    app.browser.filter = "nw_bandit001".into();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    // (The row, from the dock's left edge: its icon is the only stroke
    // in it.)
    let icon_of = |h: &Harness<'_, Moonglow>| {
        let name = h.get_all_by_label("nw_bandit001.utc").next().expect("the row").rect();
        let left = left_dock_rect(h).left();
        egui::Rect::from_x_y_ranges(left..=name.right(), name.top() - 2.0..=name.bottom() + 2.0)
    };
    let hue = kind_hue(Icon::Creature, mg_uikit::Theme::Dark).unwrap();
    let t = mg_uikit::Theme::Dark.tokens();
    let on = colored_until(&mut h, true, icon_of, |c| c.contains(&hue));
    assert!(on.iter().all(|c| *c == hue), "on: {on:?}");
    let off = colored_until(&mut h, false, icon_of, |c| !c.is_empty() && !any_hue(c));
    assert!(off.iter().all(|c| *c == t.text_2), "off: {off:?}");
}

/// Quick Open: a script's row that is not the chosen one shows its icon in
/// the script's hue while on; the chosen row keeps `on-fill`; off, no hue.
#[test]
fn quick_open_s_rows_take_their_kind_s_hue() {
    let dir = mg_testkit::scratch_dir("ui-colored-quick-open");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for name in ["hello", "hello_two"] {
        m.set(ResKey::parse(name, ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    }
    let path = dir.join("quick.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    quick_open(&mut h, "hello");
    assert!(h.state().quick_open.shown().0.len() >= 2, "two rows");
    let popover = |h: &Harness<'_, Moonglow>| h.state().quick_open.popover_rect().expect("open");
    let hue = kind_hue(Icon::Script, mg_uikit::Theme::Dark).unwrap();
    let t = mg_uikit::Theme::Dark.tokens();
    let on = colored_until(&mut h, true, popover, |c| c.contains(&hue));
    assert!(on.contains(&t.on_fill), "the chosen row white: {on:?}");
    let off = colored_until(&mut h, false, popover, |c| !c.is_empty() && !any_hue(c));
    assert!(off.contains(&t.text_3), "off, as before: {off:?}");
}

/// The hak editor's table: a script's and a sound's rows show their icons
/// in their hues while on (the row that is selected, if any, white); a
/// picture's (no hue) stays gray; off, no hue.
#[test]
fn the_hak_view_s_rows_take_their_kind_s_hue() {
    use mg_module::hak_edit::{Hak, Source};
    let path = mg_testkit::scratch_dir("ui-colored-hak").join("zz_ours.hak");
    let mut hak = Hak::new();
    let key = |n: &str, t| ResKey::parse(n, t).unwrap();
    let items = vec![
        (key("zz_script", ResType::NSS), Source::Bytes(b"void main() { }\n".to_vec().into())),
        (key("zz_sound", ResType::WAV), Source::Bytes(crate::area_view::pcm_track().into())),
        (key("zz_other", ResType::NSS), Source::Bytes(b"void main() { }\n".to_vec().into())),
    ];
    hak.add_sources(items).unwrap();
    hak.save(Some(&path)).unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    mg_ui::hak_view::open_at(h.state_mut(), &path).expect("opened");
    settle(&mut h);
    // (The table's rows by their names, from a little before them: each
    // icon is in its name's cell, left of the words.)
    let table = |h: &Harness<'_, Moonglow>| {
        let rows = ["zz_script.nss", "zz_sound.wav", "zz_other.nss"].map(|name| {
            h.get_all_by_label(mg_ui::hak_view::HAK_TABLE)
                .filter_map(|t| t.query_by_label(name).map(|n| n.rect()))
                .next_back()
                .unwrap_or_else(|| panic!("no row {name}"))
        });
        let all = rows.iter().fold(rows[0], |a, r| a.union(*r));
        all.with_min_x(all.left() - 40.0).expand2(egui::vec2(0.0, 2.0))
    };
    let dark = mg_uikit::Theme::Dark;
    let (script, sound) =
        (kind_hue(Icon::Script, dark).unwrap(), kind_hue(Icon::Sound, dark).unwrap());
    colored_until(&mut h, true, table, |c| c.contains(&script) && c.contains(&sound));
    colored_until(&mut h, false, table, |c| !c.is_empty() && !any_hue(c));
}
