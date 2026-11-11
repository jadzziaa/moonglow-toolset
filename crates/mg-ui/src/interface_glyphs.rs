//! The symbol and emoji characters written into the interface's strings,
//! held to a list, exactly: an icon is one of the drawn set
//! (`mg_uikit::Icon`), never a character of a font. What may keep such a
//! character is text where it is a sign of its own: arithmetic and
//! measures in data and messages (`64 × 128`, `≠` for values that differ),
//! a turn's notation (`↺90°`), and an arrow (`→`) in a message only the
//! log shows, in its monospace font: each in the font that draws it
//! (`the_fonts_have_every_character_kept`; the proportional fonts have no
//! arrow, and a message they draw writes `->`). A file that adds one says
//! why here, or draws an icon. (String, byte-string and character
//! literals are read, with their escapes: not comments, not the tests.)
//!
//! The files read are the sources of every crate of the workspace whose
//! strings the application can show (its messages, its errors, its
//! widgets' words): all of `crates/` but the two that are tests alone
//! (`mg-testkit`, `mg-corpus-tests`), and the application `apps/moonglow`;
//! not `apps/mg`, whose words go to a terminal.

/// The font a kept character is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Font {
    /// The interface's (labels, buttons, tips, windows' messages).
    Proportional,
    /// The log's, where the message goes nowhere else.
    Monospace,
}

use Font::{Monospace, Proportional};

/// Each file (from the workspace's root) and character kept, how many
/// times, the font it is drawn in, and why.
const KEPT: &[(&str, char, usize, Font, &str)] = &[
    (
        "crates/mg-edit/src/lists.rs",
        '≠',
        1,
        Proportional,
        "the mark of a list's mixed rows (\"≠ 2 of 3\")",
    ),
    (
        "crates/mg-image/src/dds.rs",
        '×',
        1,
        Proportional,
        "a texture's size in an error (\"64×64\")",
    ),
    ("crates/mg-image/src/lib.rs", '×', 1, Proportional, "an image's size in an error"),
    ("crates/mg-module/src/new.rs", '×', 1, Proportional, "an area's size in an error (\"8×8\")"),
    (
        "crates/mg-ui/src/area_props.rs",
        '≠',
        1,
        Proportional,
        "the mark of values that differ, named in a message",
    ),
    (
        "crates/mg-ui/src/blueprint/creature_lists.rs",
        '×',
        1,
        Proportional,
        "a count (\"Bless ×2\"), arithmetic in data",
    ),
    (
        "crates/mg-ui/src/blueprint/inventory_together.rs",
        '×',
        1,
        Proportional,
        "a count (\"×3\"), arithmetic in data",
    ),
    (
        "crates/mg-ui/src/blueprint/mod.rs",
        '≠',
        3,
        Proportional,
        "the mark of values that differ (MIXED), and messages naming it",
    ),
    ("crates/mg-ui/src/lib.rs", '×', 1, Proportional, "an area's size in a message (\"8×8\")"),
    (
        "crates/mg-ui/src/lib.rs",
        '→',
        1,
        Monospace,
        "a verify finding's reference, from what to what: the log's alone",
    ),
    (
        "crates/mg-ui/src/model_textures.rs",
        '×',
        1,
        Proportional,
        "a texture's size (\"64×128\"), data",
    ),
    (
        "crates/mg-ui/src/nui_view/headless.rs",
        '×',
        1,
        Proportional,
        "a picture's size in an error message",
    ),
    (
        "crates/mg-ui/src/nui_view/preview/stage.rs",
        '×',
        2,
        Proportional,
        "the screen's size (\"1920 × 1080\"), data",
    ),
    (
        "crates/mg-ui/src/tile_select.rs",
        '↺',
        6,
        Proportional,
        "a turn's notation (\"↺90°\": counter-clockwise), data",
    ),
    ("crates/mg-ui/src/tileset_view.rs", '×', 1, Proportional, "a group's size (\"3×2\"), data"),
    (
        "crates/mg-ui/src/tileset_view.rs",
        '→',
        1,
        Monospace,
        "a terrain's rename in the palette, said in the log alone",
    ),
    (
        "crates/mg-ui/src/transfer.rs",
        '→',
        2,
        Monospace,
        "an export's missing reference, from what to what: warnings in the log alone",
    ),
    (
        "crates/mg-ui/src/wizards.rs",
        '×',
        1,
        Proportional,
        "between an area's width and height, arithmetic",
    ),
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

/// The characters of `source`'s string, byte-string and character
/// literals (their escapes read), outside comments and outside its test
/// modules (`#[cfg(test)]` before a `mod` with its body; the code after
/// one is read on).
pub(crate) fn literal_chars(source: &str) -> Vec<char> {
    let s: Vec<char> = source.chars().collect();
    let skipped = test_modules(&s);
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
    // (Whether the character before `i` continues a name: `r` and `b` of
    // `bar"` are not a literal's prefix.)
    let in_name =
        |i: usize| s.get(i.wrapping_sub(1)).is_some_and(|p| p.is_alphanumeric() || *p == '_');
    while i < s.len() {
        if let Some(&(_, end)) = skipped.iter().find(|(from, _)| *from == i) {
            i = end;
            continue;
        }
        let c = s[i];
        let next = s.get(i + 1).copied();
        // A byte string's or byte's `b` before it: read as the plain one.
        if c == 'b' && !in_name(i) && matches!(next, Some('"' | '\'' | 'r')) {
            if next == Some('r') && !matches!(s.get(i + 2), Some('"' | '#')) {
                i += 1;
                continue;
            }
            i += 1;
            if next != Some('r') {
                continue;
            }
            // (A raw byte string: the raw string's branch, from its `r`.)
        }
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
            && (!in_name(i) || s.get(i.wrapping_sub(1)) == Some(&'b') && !in_name(i - 1))
        {
            // A raw string: r"...", r#"..."#, br"...".
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

/// Where `s`'s test modules are, each from its `#[cfg(test)]` to past the
/// brace that closes its body (`mod tests { … }`; not a file's, `mod
/// chrome_colors;`). Braces in its strings, characters and comments are
/// not counted.
fn test_modules(s: &[char]) -> Vec<(usize, usize)> {
    let attr: Vec<char> = "#[cfg(test)]".chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + attr.len() <= s.len() {
        if s[i..i + attr.len()] != attr[..] {
            i += 1;
            continue;
        }
        let mut j = i + attr.len();
        while s.get(j).is_some_and(|c| c.is_whitespace()) {
            j += 1;
        }
        let rest: String = s[j..(j + 4).min(s.len())].iter().collect();
        if rest != "mod " {
            i += 1;
            continue;
        }
        j += 4;
        while s.get(j).is_some_and(|c| c.is_alphanumeric() || *c == '_' || c.is_whitespace()) {
            j += 1;
        }
        if s.get(j) != Some(&'{') {
            i += 1;
            continue;
        }
        // Its body, to the brace that closes it.
        let mut depth = 0;
        while j < s.len() {
            match s[j] {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        j += 1;
                        break;
                    }
                }
                '/' if s.get(j + 1) == Some(&'/') => {
                    while j < s.len() && s[j] != '\n' {
                        j += 1;
                    }
                    continue;
                }
                'r' if matches!(s.get(j + 1), Some('"' | '#'))
                    && !s
                        .get(j.wrapping_sub(1))
                        .is_some_and(|p| p.is_alphanumeric() || *p == '_') =>
                {
                    // (A raw string: to its quote and as many hashes.)
                    let mut k = j + 1;
                    let mut hashes = 0;
                    while s.get(k) == Some(&'#') {
                        hashes += 1;
                        k += 1;
                    }
                    if s.get(k) == Some(&'"') {
                        k += 1;
                        while k < s.len()
                            && !(s[k] == '"' && (0..hashes).all(|h| s.get(k + 1 + h) == Some(&'#')))
                        {
                            k += 1;
                        }
                        j = k + hashes;
                    }
                }
                '"' => {
                    // (A string: to its closing quote, escapes skipped.)
                    j += 1;
                    while j < s.len() && s[j] != '"' {
                        j += if s[j] == '\\' { 2 } else { 1 };
                    }
                }
                '\'' if s.get(j + 2) == Some(&'\'') => j += 2,
                '\'' if s.get(j + 1) == Some(&'\\') => {
                    while j + 1 < s.len() && s[j + 1] != '\'' {
                        j += 1;
                    }
                    j += 1;
                }
                _ => {}
            }
            j += 1;
        }
        out.push((i, j.min(s.len())));
        i = j;
    }
    out
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
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sources = vec![root.join("apps/moonglow/src")];
    for e in std::fs::read_dir(root.join("crates")).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !matches!(name.as_str(), "mg-testkit" | "mg-corpus-tests") {
            sources.push(e.path().join("src"));
        }
    }
    let mut files = Vec::new();
    for dir in sources.iter().filter(|d| d.is_dir()) {
        walk(dir, &mut files);
    }
    files.sort();
    assert!(files.len() > 300, "the workspace's sources: {} files", files.len());
    let mut wrong = Vec::new();
    let mut seen = Vec::new();
    for path in files {
        let name = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        let own = name == "crates/mg-ui/src/interface_glyphs.rs";
        if own || name.ends_with("/tests.rs") {
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
    for (f, c, n, ..) in KEPT {
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
        mod tests { const T: &str = "🗻 }"; fn f() { let _ = '}'; } }
        let h = "⛔ after the tests";
        let i = b"\xe2 ok";
        let j = br"raw \ bytes ✂";
        let k = br#"raw "quoted" ✈"#;
        let l = b'"';
        let m = "✉";
        let bar = 1; let brr = "☂";
    "##;
    let got: String = literal_chars(source).into_iter().filter(|c| symbol(*c)).collect();
    assert_eq!(got, "💾▸⚙✔→☰⛶⛔✂✈✉☂");
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
    // What each character of `text` is drawn with, in the font texture,
    // in `style`'s font. (Not `Fonts::has_glyphs`: it says no for every
    // glyph of the font that has the replacement square, egui's emoji
    // font.)
    let drawn = |text: &str, style: egui::TextStyle| {
        let font = style.resolve(&ctx.global_style());
        let galley =
            ctx.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font, egui::Color32::WHITE));
        let rows = galley.rows.iter().flat_map(|r| r.glyphs.iter());
        rows.map(|g| g.uv_rect).collect::<Vec<_>>()
    };
    let lacks = |text: &str, style: egui::TextStyle| {
        let square = drawn("◻", style.clone())[0];
        drawn(text, style).contains(&square)
    };
    let style = |f: Font| match f {
        Proportional => egui::TextStyle::Body,
        Monospace => egui::TextStyle::Monospace,
    };
    let missing: Vec<(char, Font)> =
        (KEPT.iter()).filter(|k| lacks(&k.1.to_string(), style(k.3))).map(|k| (k.1, k.3)).collect();
    assert!(missing.is_empty(), "no glyph for {missing:?}");
    // (The check can fail: no font has the last character, and the
    // proportional fonts have no arrow.)
    assert!(lacks("\u{10ffff}", egui::TextStyle::Body) && !lacks("A", egui::TextStyle::Body));
    assert!(lacks("→", egui::TextStyle::Body), "the proportional fonts have an arrow now");
}
