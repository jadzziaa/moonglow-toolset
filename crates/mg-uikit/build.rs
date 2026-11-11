//! Makes the icon set, `icons/icons.json` (the one source of the icons:
//! each name's SVG markup in a 24 by 24 box), into Rust: the `Icon` enum
//! and each icon's strokes as polylines, written to `OUT_DIR/icons.rs`
//! for `src/icons.rs` to include.

use std::fmt::Write as _;

#[path = "src/svg.rs"]
#[allow(dead_code)]
mod svg;

fn main() {
    println!("cargo:rerun-if-changed=icons/icons.json");
    println!("cargo:rerun-if-changed=src/svg.rs");
    let json = std::fs::read_to_string("icons/icons.json").expect("read icons/icons.json");
    let icons: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&json).expect("icons/icons.json is an object of names");
    let mut variants = String::new();
    let mut names = String::new();
    let mut all = String::new();
    let mut from = String::new();
    let mut shapes = String::new();
    for (name, markup) in &icons {
        let markup = markup.as_str().unwrap_or_else(|| panic!("icon {name}: not a string"));
        let polys = svg::parse_icon(markup).unwrap_or_else(|e| panic!("icon {name}: {e}"));
        let variant = camel(name);
        writeln!(variants, "    #[doc = \"`{name}`\"]\n    {variant},").unwrap();
        writeln!(names, "            Icon::{variant} => \"{name}\",").unwrap();
        writeln!(all, "        Icon::{variant},").unwrap();
        writeln!(from, "            \"{name}\" => Some(Icon::{variant}),").unwrap();
        shapes.push_str("    &[\n");
        for p in &polys {
            let points: Vec<String> =
                p.points.iter().map(|[x, y]| format!("[{}, {}]", num(*x), num(*y))).collect();
            writeln!(
                shapes,
                "        IconShape {{ closed: {}, filled: {}, points: &[{}] }},",
                p.closed,
                p.filled,
                points.join(", ")
            )
            .unwrap();
        }
        shapes.push_str("    ],\n");
    }
    let count = icons.len();
    let code = format!(
        "/// One icon of Moonglow's drawn set (24 by 24, strokes 1.5 wide, drawn at 16 px):\n\
         /// its name in `icons/icons.json` beside each.\n\
         #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\n\
         pub enum Icon {{\n{variants}}}\n\n\
         impl Icon {{\n\
         \x20   /// Every icon, in the order of their names.\n\
         \x20   pub const ALL: [Icon; {count}] = [\n{all}    ];\n\n\
         \x20   /// The icon's name (`chevron-down`), as in `icons/icons.json`.\n\
         \x20   pub const fn name(self) -> &'static str {{\n\
         \x20       match self {{\n{names}        }}\n\
         \x20   }}\n\n\
         \x20   /// The icon named `name` (`chevron-down`), if the set has it.\n\
         \x20   pub fn from_name(name: &str) -> Option<Icon> {{\n\
         \x20       match name {{\n{from}            _ => None,\n        }}\n\
         \x20   }}\n\
         }}\n\n\
         /// Each icon's strokes, in the order of [`Icon::ALL`]. (Coordinates, some\n\
         /// near a constant such as 3.14 by chance.)\n\
         #[allow(clippy::approx_constant)]\n\
         static SHAPES: [&[IconShape]; {count}] = [\n{shapes}];\n"
    );
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out.join("icons.rs"), code).expect("write icons.rs");
}

/// `chevron-down` as `ChevronDown`.
fn camel(name: &str) -> String {
    let mut out = String::new();
    for part in name.split(['-', '_']).filter(|p| !p.is_empty()) {
        let mut chars = part.chars();
        if let Some(c) = chars.next() {
            out.extend(c.to_uppercase());
            out.extend(chars);
        }
    }
    assert!(
        out.starts_with(|c: char| c.is_ascii_alphabetic()),
        "icon {name}: a name must begin with a letter"
    );
    out
}

/// A coordinate to three places, written as an `f32` literal.
fn num(v: f32) -> String {
    let v = (v * 1000.0).round() / 1000.0;
    let v = if v == 0.0 { 0.0 } else { v };
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{s}.0") }
}
