//! The INI dialect of tileset (`.set`) files: `[SECTION]` headers and
//! `Key=Value` lines, `;` comments on their own lines, CRLF or LF. Section and
//! key lookups ignore ASCII case; order and duplicates are kept.

/// A `[SECTION]` with its entries in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub entries: Vec<(String, String)>,
}

impl Section {
    /// The first value for a key (trimmed; may be empty).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }

    /// A value parsed like the game's integer reader (sign, digits, trailing
    /// junk ignored); `None` if missing or not a number.
    pub fn int(&self, key: &str) -> Option<i32> {
        let v = self.get(key)?.trim();
        let end = v
            .char_indices()
            .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
            .map_or(v.len(), |(i, _)| i);
        v[..end].parse().ok()
    }

    /// A value parsed as a float (the longest valid prefix).
    pub fn float(&self, key: &str) -> Option<f32> {
        let v = self.get(key)?.trim();
        (1..=v.len()).rev().filter(|&n| v.is_char_boundary(n)).find_map(|n| v[..n].parse().ok())
    }

    /// A non-empty string value.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.get(key).filter(|v| !v.is_empty())
    }
}

/// A parsed INI file: sections in file order (entries before the first header
/// go into a section with an empty name).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ini {
    pub sections: Vec<Section>,
}

impl Ini {
    pub fn parse(text: &str) -> Ini {
        let mut sections = vec![Section::default()];
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                sections.push(Section { name: name.trim().to_string(), entries: Vec::new() });
            } else if let Some((k, v)) = line.split_once('=') {
                let current = sections.last_mut().expect("at least one section");
                current.entries.push((k.trim().to_string(), v.trim().to_string()));
            }
            // Other lines carry no data; the game ignores them too.
        }
        if sections[0].entries.is_empty() {
            sections.remove(0);
        }
        Ini { sections }
    }

    /// The first section with this name.
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_values() {
        let ini = Ini::parse(
            "; comment\r\n[GENERAL]\r\nName=TTR01\r\nCount = 3\r\nEmpty=\r\n\r\n[tile0]\nModel=a;b\nX=1.5f\nN=-4x\n",
        );
        let g = ini.section("general").unwrap();
        assert_eq!(g.get("name"), Some("TTR01"));
        assert_eq!(g.int("COUNT"), Some(3));
        assert_eq!(g.get("Empty"), Some(""));
        assert_eq!(g.text("Empty"), None);
        let t = ini.section("TILE0").unwrap();
        assert_eq!(t.get("Model"), Some("a;b"), "no inline comments");
        assert_eq!(t.float("X"), Some(1.5));
        assert_eq!(t.int("N"), Some(-4));
        assert!(ini.section("nope").is_none());
    }
}
