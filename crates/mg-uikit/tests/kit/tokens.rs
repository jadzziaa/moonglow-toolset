//! `Tokens::DARK` and `Tokens::LIGHT` against the mockups' token tables
//! (`tests/fixtures/tokens.css`, a copy of them): every token there, with
//! the mockups' value, and no field of the struct left out.

use egui::Color32;
use mg_uikit::Tokens;

/// A block's custom properties (`--name: value;`), `var(--other)`
/// resolved within the block.
fn block(css: &str, selector: &str) -> Vec<(String, Color32)> {
    let start = css.find(&format!("\n{selector} {{")).expect(selector) + selector.len() + 3;
    let body = &css[start..start + css[start..].find('}').unwrap()];
    let mut raw: Vec<(String, String)> = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("--") else { continue };
        let (name, value) = rest.split_once(':').unwrap();
        raw.push((name.trim().into(), value.trim().trim_end_matches(';').trim().into()));
    }
    let lookup = raw.clone();
    raw.into_iter()
        .map(|(name, value)| {
            let value = match value.strip_prefix("var(--") {
                Some(other) => {
                    let other = other.trim_end_matches(')');
                    lookup.iter().find(|(n, _)| n == other).unwrap().1.clone()
                }
                None => value,
            };
            (name, color(&value))
        })
        .collect()
}

fn color(value: &str) -> Color32 {
    if value == "transparent" {
        return Color32::TRANSPARENT;
    }
    if let Some(hex) = value.strip_prefix('#') {
        let v = u32::from_str_radix(hex, 16).unwrap();
        return Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
    }
    let inner = value.strip_prefix("rgba(").and_then(|v| v.strip_suffix(')')).expect(value);
    let parts: Vec<f32> = inner.split(',').map(|p| p.trim().parse().unwrap()).collect();
    let alpha = (parts[3] * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(parts[0] as u8, parts[1] as u8, parts[2] as u8, alpha)
}

#[test]
fn the_tokens_are_the_mockups() {
    let css = include_str!("../fixtures/tokens.css");
    for (selector, tokens) in [(".mg", Tokens::DARK), (".mg.light", Tokens::LIGHT)] {
        let table = block(css, selector);
        let ours = tokens.all();
        for (name, value) in &table {
            let field = ours.iter().find(|(n, _)| n == name);
            let field = field.unwrap_or_else(|| panic!("{selector}: --{name} has no token"));
            assert_eq!(field.1, *value, "{selector}: --{name}");
        }
        for (name, _) in &ours {
            assert!(table.iter().any(|(n, _)| n == name), "{selector}: {name} not in the tables");
        }
        assert_eq!(table.len(), ours.len(), "{selector}: a token twice");
    }
}

/// The two themes differ where the mockups say they do, and the
/// thumbnails' tokens keep one value.
#[test]
fn the_themes_differ_but_for_the_thumbnails() {
    let (dark, light) = (Tokens::DARK.all(), Tokens::LIGHT.all());
    for ((name, d), (_, l)) in dark.iter().zip(&light) {
        let same =
            name.starts_with("thumb-") || matches!(*name, "on-fill" | "syn-error" | "on-select");
        if same {
            assert_eq!(d, l, "{name} keeps one value");
        } else {
            assert_ne!(d, l, "{name} has a light value of its own");
        }
    }
}
