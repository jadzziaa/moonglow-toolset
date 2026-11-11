//! The symbol and emoji characters written into the interface's strings,
//! held to a list, exactly: an icon is one of the drawn set
//! (`mg_uikit::Icon`), never a character of a font. What may keep such a
//! character is text where it is a sign of its own: arithmetic and
//! measures in data and messages (`64 × 128`, `≠` for values that differ),
//! a turn's notation (`↺90°`), each in the app's fonts
//! (`the_fonts_have_every_character_kept`: an arrow, `→`, is not, and a
//! message writes `->`). A
//! file that adds one says why here, or draws an icon. (String and
//! character literals are read, with their escapes: not comments, not the
//! tests.)

/// Each file and character kept, how many times, and why.
const KEPT: &[(&str, char, usize, &str)] = &[
    ("area_props.rs", '≠', 1, "the mark of values that differ, named in a message"),
    ("blueprint/creature_lists.rs", '×', 1, "a count (\"Bless ×2\"), arithmetic in data"),
    ("blueprint/inventory_together.rs", '×', 1, "a count (\"×3\"), arithmetic in data"),
    ("blueprint/mod.rs", '≠', 3, "the mark of values that differ (MIXED), and messages naming it"),
    ("lib.rs", '×', 1, "an area's size in a message (\"8×8\")"),
    ("model_textures.rs", '×', 1, "a texture's size (\"64×128\"), data"),
    ("nui_view/headless.rs", '×', 1, "a picture's size in an error message"),
    ("nui_view/preview/stage.rs", '×', 2, "the screen's size (\"1920 × 1080\"), data"),
    ("tile_select.rs", '↺', 6, "a turn's notation (\"↺90°\": counter-clockwise), data"),
    ("tileset_view.rs", '×', 1, "a group's size (\"3×2\"), data"),
    ("wizards.rs", '×', 1, "between an area's width and height, arithmetic"),
];

/// Whether `c` is a symbol or an emoji: the blocks of letterlike symbols,
/// arrows, mathematical operators, technical symbols, enclosed
/// characters, box drawing, geometric shapes, symbols and dingbats, and
/// everything past the Basic Multilingual Plane's letters (the emoji), and
/// the multiplication sign (the close mark that was).
pub(crate) fn symbol(c: char) -> bool {
    let o = u32::from(c);
    c == '×'
        || (0x2100..=0x214F).contains(&o)
        || (0x2190..=0x23FF).contains(&o)
        || (0x2460..=0x27FF).contains(&o)
        || (0x2900..=0x2BFF).contains(&o)
        || (0x1F000..=0x1FFFF).contains(&o)
        || o == 0xFE0F
}

/// The characters of `source`'s string and character literals (their
/// escapes read), outside comments and before its tests (`#[cfg(test)]`
/// and a `mod`).
pub(crate) fn literal_chars(source: &str) -> Vec<char> {
    let end = test_module_start(source).unwrap_or(source.len());
    let s: Vec<char> = source[..end].chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    // (Read one escape at `i`, after its backslash: the character, and
    // where the literal goes on.)
    let escape = |i: usize| -> (Option<char>, usize) {
        match s.get(i) {
            Some('u') if s.get(i + 1) == Some(&'{') => {
                let close = (i + 2..s.len()).find(|&j| s[j] == '}').unwrap_or(s.len() - 1);
                let hex: String = s[i + 2..close].iter().filter(|c| **c != '_').collect();
                (u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32), close + 1)
            }
            Some('x') => (None, i + 3),
            Some('\n') => {
                // (A line continued: its next line's spaces are dropped.)
                let mut j = i + 1;
                while s.get(j).is_some_and(|c| c.is_whitespace()) {
                    j += 1;
                }
                (None, j)
            }
            Some(c) => (Some(*c), i + 1),
            None => (None, i),
        }
    };
    while i < s.len() {
        let c = s[i];
        let next = s.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < s.len() && s[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < s.len() {
                if s[i] == '/' && s.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if s[i] == '*' && s.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else if c == 'r'
            && (next == Some('"') || next == Some('#'))
            && !s.get(i.wrapping_sub(1)).is_some_and(|p| p.is_alphanumeric() || *p == '_')
        {
            // A raw string: r"...", r#"..."#.
            let mut j = i + 1;
            let mut hashes = 0;
            while s.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if s.get(j) != Some(&'"') {
                i += 1;
                continue;
            }
            j += 1;
            while j < s.len() {
                if s[j] == '"' && (0..hashes).all(|h| s.get(j + 1 + h) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                out.push(s[j]);
                j += 1;
            }
            i = j;
        } else if c == '"' {
            i += 1;
            while i < s.len() && s[i] != '"' {
                if s[i] == '\\' {
                    let (ch, at) = escape(i + 1);
                    out.extend(ch);
                    i = at;
                } else {
                    out.push(s[i]);
                    i += 1;
                }
            }
            i += 1;
        } else if c == '\'' {
            // A character, or a lifetime ('a, 'static).
            if next == Some('\\') {
                let (ch, at) = escape(i + 2);
                out.extend(ch);
                i = at + 1;
            } else if s.get(i + 2) == Some(&'\'') {
                out.extend(next);
                i += 3;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Where `source`'s tests begin: `#[cfg(test)]` before a `mod` with its
/// body (`mod tests {`; not a file's, `mod chrome_colors;`).
fn test_module_start(source: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(at) = source[from..].find("#[cfg(test)]") {
        let at = from + at;
        let rest = source[at + "#[cfg(test)]".len()..].trim_start();
        let body = rest.strip_prefix("mod ").and_then(|r| {
            let r = r.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_');
            r.trim_start().strip_prefix('{')
        });
        if body.is_some() {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

#[test]
fn no_interface_string_has_a_glyph_for_an_icon() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                // (A folder of tests is no interface.)
                if !p.ends_with("tests") {
                    walk(&p, out);
                }
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&src, &mut files);
    files.sort();
    let mut wrong = Vec::new();
    let mut seen = Vec::new();
    for path in files {
        let name = path.strip_prefix(&src).unwrap().to_string_lossy().replace('\\', "/");
        if name == "interface_glyphs.rs" || name == "tests.rs" || name.ends_with("/tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut counts: Vec<(char, usize)> = Vec::new();
        for c in literal_chars(&text).into_iter().filter(|c| symbol(*c)) {
            match counts.iter_mut().find(|(k, _)| *k == c) {
                Some((_, n)) => *n += 1,
                None => counts.push((c, 1)),
            }
        }
        for (c, n) in counts {
            let kept = KEPT.iter().find(|(f, k, ..)| *f == name && *k == c).map_or(0, |k| k.2);
            // (Exactly: one taken out lowers the count kept, so that none
            // goes back in unseen.)
            if n != kept {
                wrong.push(format!("{name}: {n} {c:?} (U+{:04X}), {kept} kept", u32::from(c)));
            }
            seen.push((name.clone(), c));
        }
    }
    for (f, c, n, _) in KEPT {
        if !seen.iter().any(|(sf, sc)| sf == f && sc == c) {
            wrong.push(format!("{f}: 0 {c:?}, {n} kept"));
        }
    }
    assert!(
        wrong.is_empty(),
        "an icon is drawn (mg_uikit::Icon), or the list says why: {wrong:#?}"
    );
}

#[test]
fn literals_are_read_with_their_escapes_and_not_comments() {
    let source = r##"
        // "🔍 in a comment"
        /* "⏷" /* nested "⏶" */ */
        let a = "💾 Save";
        let b = "\u{25B8} more \"⚙\"";
        let c = '✔';
        let d = '\u{2192}';
        fn f<'a>(x: &'a str) {}
        let e = r#"raw "☰""#;
        let f = "a\
                 b";
        #[cfg(test)]
        mod files;
        let g = "⛶";
        #[cfg(test)]
        mod tests { const T: &str = "🗻"; }
    "##;
    let got: String = literal_chars(source).into_iter().filter(|c| symbol(*c)).collect();
    assert_eq!(got, "💾▸⚙✔→☰⛶");
    assert!(symbol('ℹ') && symbol('×') && symbol('🗻') && symbol('≠'));
    assert!(!symbol('°') && !symbol('…') && !symbol('—') && !symbol('é') && !symbol('›'));
}

/// The characters kept are in the app's fonts (none drawn as a square).
#[test]
fn the_fonts_have_every_character_kept() {
    let ctx = egui::Context::default();
    crate::widgets::install_fonts(&ctx);
    // (The fonts are there from the next frame.)
    for _ in 0..2 {
        ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
    }
    let font = egui::TextStyle::Body.resolve(&ctx.global_style());
    // What each character of `text` is drawn with, in the font texture.
    // (Not `Fonts::has_glyphs`: it says no for every glyph of the font
    // that has the replacement square, egui's emoji font.)
    let drawn = |text: &str| {
        let galley = ctx
            .fonts_mut(|f| f.layout_no_wrap(text.to_string(), font.clone(), egui::Color32::WHITE));
        let rows = galley.rows.iter().flat_map(|r| r.glyphs.iter());
        rows.map(|g| g.uv_rect).collect::<Vec<_>>()
    };
    let square = drawn("◻")[0];
    let lacks = |text: &str| drawn(text).contains(&square);
    let missing: Vec<char> = KEPT.iter().map(|k| k.1).filter(|c| lacks(&c.to_string())).collect();
    assert!(missing.is_empty(), "no glyph for {missing:?}");
    assert!(lacks("\u{10ffff}") && !lacks("A"), "the check can fail");
}
