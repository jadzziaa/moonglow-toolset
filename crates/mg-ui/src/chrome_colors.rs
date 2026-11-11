//! The colors written into the code (`Color32::from_rgb` and its kin),
//! held to a list: the interface's own colors (fills, borders, text,
//! selection, hover) are the theme's tokens (`mg_uikit::tokens`,
//! `ui.tokens()`), which follow Options › General › Light theme. What may
//! keep colors of its own is a view's picture (the 3D views' handles,
//! outlines and overlays), the game's look (NUI windows, Aurora's faction
//! colors), data (a color typed, read or picked) and the tests. A file
//! that adds one says why here, or takes a token.

/// Each file that writes colors of its own, how many, and why.
const KEPT: &[(&str, usize, &str)] = &[
    ("area_grass.rs", 1, "the light ground behind a grass texture's tufts (a picture)"),
    ("area_view.rs", 46, "handles, outlines, kind colors and labels drawn over the 3D view"),
    ("bake_view.rs", 2, "the walkmeshes before and after, drawn over the 3D view"),
    ("color_button.rs", 11, "colors typed, parsed and picked (data), and its tests"),
    ("dialog_view.rs", 1, "the speakers' colors of Options › Conversation (data)"),
    ("faction_view.rs", 3, "Aurora's hostile, neutral and friendly colors (the game's look)"),
    ("minimap.rs", 2, "the minimap's frame over the 3D view"),
    ("options.rs", 2, "its tests"),
    ("paint_mode.rs", 2, "the brush's circle and eraser over the 3D view"),
    ("plugins.rs", 1, "a plugin choice's color swatch (data)"),
    ("terrain_mode.rs", 5, "the tile cursors over the 3D view"),
    ("tile_select.rs", 5, "the tileset's own toolset color, and tile cursors over the view"),
    ("widgets.rs", 1, "a PLT palette's tones (data)"),
    ("script_view.rs", 2, "the script colors of Options › Script Editor (data)"),
    ("blueprint/situated.rs", 2, "a door's transitions marked on the area's map (a picture)"),
    ("nui_view/preview.rs", 2, "NUI windows as the game draws them"),
    ("nui_view/skin.rs", 5, "NUI windows as the game draws them, and its tests"),
    ("nui_view/preview/node.rs", 3, "NUI windows as the game draws them"),
    ("nui_view/preview/sliders.rs", 1, "NUI windows as the game draws them"),
    ("nui_view/preview/stage.rs", 1, "the game's screen behind a NUI window"),
    ("nui_view/preview/tests/canvas.rs", 3, "tests"),
    ("nui_view/preview/tests/native.rs", 1, "tests"),
];

/// How many colors `text` writes of its own.
fn written(text: &str) -> usize {
    [
        "Color32::from_rgb(",
        "Color32::from_gray(",
        "Color32::from_rgba_unmultiplied(",
        "Color32::from_rgba_premultiplied(",
        "Color32::from_black_alpha(",
        "Color32::from_white_alpha(",
    ]
    .iter()
    .map(|c| text.matches(c).count())
    .sum()
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
        let n = written(&std::fs::read_to_string(&path).unwrap());
        let kept = KEPT.iter().find(|(f, ..)| *f == name).map_or(0, |k| k.1);
        if n > kept {
            wrong.push(format!(
                "{name}: {n} colors written, {kept} kept (a token of the theme, or a reason here)"
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
