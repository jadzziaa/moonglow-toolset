//! The colors written into the code (`Color32::from_rgb` and its kin,
//! and the named ones, `Color32::WHITE`), held to a list, exactly: the interface's own colors (fills, borders, text,
//! selection, hover) are the theme's tokens (`mg_uikit::tokens`,
//! `ui.tokens()`), which follow Options › General › Light theme. What may
//! keep colors of its own is a view's picture (the 3D views' handles,
//! outlines and overlays), the game's look (NUI windows, Aurora's faction
//! colors), data (a color typed, read or picked) and the tests. A file
//! that adds one says why here, or takes a token.

/// Each file that writes colors of its own, how many, and why.
const KEPT: &[(&str, usize, &str)] = &[
    ("area_grass.rs", 1, "the light ground behind a grass texture's tufts (a picture)"),
    ("area_view.rs", 67, "handles, outlines, kind colors and labels drawn over the 3D view"),
    ("bake_view.rs", 2, "the walkmeshes before and after, drawn over the 3D view"),
    ("color_button.rs", 11, "colors typed, parsed and picked (data), and its tests"),
    ("dialog_view.rs", 1, "the speakers' colors of Options › Conversation (data)"),
    ("faction_view.rs", 3, "Aurora's hostile, neutral and friendly colors (the game's look)"),
    ("minimap.rs", 2, "the minimap's frame over the 3D view"),
    ("browser.rs", 2, "its tests"),
    ("options.rs", 4, "its tests"),
    ("script_layout.rs", 9, "its tests"),
    ("paint_mode.rs", 2, "the brush's circle and eraser over the 3D view"),
    ("plugins.rs", 1, "a plugin choice's color swatch (data)"),
    ("terrain_mode.rs", 5, "the tile cursors over the 3D view"),
    ("tile_select.rs", 6, "the tileset's own toolset color, and tile cursors over the view"),
    ("widgets.rs", 1, "a PLT palette's tones (data)"),
    ("script_view.rs", 7, "the script colors of Options › Script Editor (data), and its tests"),
    ("blueprint/situated.rs", 6, "a door's transitions marked on the area's map (a picture)"),
    ("nui_view/draw.rs", 2, "NUI draw lists as the game draws them"),
    ("nui_view/preview.rs", 2, "NUI windows as the game draws them"),
    ("nui_view/preview/chart.rs", 3, "NUI windows as the game draws them"),
    ("nui_view/preview/color_picker.rs", 5, "NUI windows as the game draws them"),
    ("nui_view/preview/combo.rs", 1, "NUI windows as the game draws them"),
    ("nui_view/preview/picture.rs", 1, "NUI windows as the game draws them"),
    ("nui_view/preview/typeset.rs", 3, "NUI windows as the game draws them"),
    ("nui_view/skin.rs", 18, "NUI windows as the game draws them, and its tests"),
    ("nui_view/preview/node.rs", 11, "NUI windows as the game draws them"),
    ("nui_view/preview/sliders.rs", 1, "NUI windows as the game draws them"),
    ("nui_view/preview/stage.rs", 10, "the game's screen behind a NUI window"),
    ("nui_view/preview/tests/canvas.rs", 3, "tests"),
    ("nui_view/preview/tests/native.rs", 1, "tests"),
];

/// Named colors that are no color of the interface, by what is on their
/// line: not counted.
const NOT_COLORS: &[(&str, &str)] = &[
    (
        "layout_no_wrap(",
        "a text laid out only to be measured (its width): the color is never drawn",
    ),
    (".image(", "an image's tint: white draws the picture as it is"),
];

/// Named colors that are never a color: egui's `PLACEHOLDER` (the
/// widget's own text color takes its place) and `TRANSPARENT` (nothing).
const NOT_A_COLOR: &[&str] = &["PLACEHOLDER", "TRANSPARENT"];

/// How many colors `text` writes of its own: each of `Color32`'s
/// constructors named (called or not: `let rgb = Color32::from_rgb;`
/// would hide every color made with `rgb(..)`, [`aliased`]), and each of
/// its named colors (`Color32::WHITE`), but those [`NOT_A_COLOR`] and those
/// on a line [`NOT_COLORS`] says is no color of the interface.
fn written(text: &str) -> usize {
    text.lines()
        .map(|line| {
            let constructors = line.matches("Color32::from_").count();
            let measured = NOT_COLORS.iter().any(|(on, _)| line.contains(on));
            let named = named_colors(line).filter(|c| !NOT_A_COLOR.contains(c)).count();
            constructors + if measured { 0 } else { named }
        })
        .sum()
}

/// The named colors on `line` (`WHITE` of `Color32::WHITE`).
fn named_colors(line: &str) -> impl Iterator<Item = &str> {
    line.match_indices("Color32::").filter_map(|(at, m)| {
        let rest = &line[at + m.len()..];
        let end = rest.find(|c: char| !(c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()));
        let name = &rest[..end.unwrap_or(rest.len())];
        (!name.is_empty() && name.starts_with(|c: char| c.is_ascii_uppercase())).then_some(name)
    })
}

/// A constructor named and not called (an alias), whose colors would not
/// be counted: `Color32::from_rgb` not followed by `(`.
fn aliased(text: &str) -> bool {
    text.match_indices("Color32::from_").any(|(at, m)| {
        let rest = &text[at + m.len()..];
        let name = rest.trim_start_matches(|c: char| c.is_ascii_lowercase() || c == '_');
        !name.starts_with('(')
    })
}

#[test]
fn no_interface_color_is_written_into_the_code() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&src, &mut files);
    let mut wrong = Vec::new();
    for path in files {
        let name = path.strip_prefix(&src).unwrap().to_string_lossy().replace('\\', "/");
        if name == "chrome_colors.rs" {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        if aliased(&text) {
            wrong.push(format!("{name}: a color constructor named without a call (an alias)"));
        }
        let n = written(&text);
        let kept = KEPT.iter().find(|(f, ..)| *f == name).map_or(0, |k| k.1);
        // (Exactly: one taken out lowers the count kept, so that none
        // goes back in unseen.)
        if n != kept {
            wrong.push(format!(
                "{name}: {n} colors written, {kept} kept (a token of the theme, or a reason here)"
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn constructors_named_colors_and_aliases_are_counted() {
    assert_eq!(written("Color32::from_rgb(1, 2, 3)"), 1);
    assert_eq!(written("let rgb = Color32::from_rgb;"), 1);
    assert!(aliased("let rgb = Color32::from_rgb;"));
    assert!(!aliased("Color32::from_rgb(1, 2, 3)"));
    assert_eq!(written("(egui::Color32::YELLOW, egui::Color32::BLACK)"), 2);
    assert_eq!(written("Color32::TRANSPARENT; Color32::PLACEHOLDER"), 0);
    assert_eq!(written("f.layout_no_wrap(t, font, egui::Color32::WHITE)"), 0);
    assert_eq!(written("painter.image(id, rect, uv, egui::Color32::WHITE);"), 0);
    assert_eq!(written("Stroke::new(1.0, egui::Color32::WHITE)"), 1);
}
