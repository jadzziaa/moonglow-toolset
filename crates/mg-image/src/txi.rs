//! TXI: a texture's settings, `keyword values…` per line (case-insensitive,
//! `#` comments). A few keywords take a count and then that many lines
//! (`channelscale 4`, `upperleftcoords N`, …).

/// Keywords followed by a count of extra lines.
const LISTS: [&str; 4] =
    ["channelscale", "channeltranslate", "upperleftcoords", "lowerrightcoords"];

/// One keyword with its values and, for list keywords, the lines after it.
#[derive(Debug, Clone, PartialEq)]
pub struct TxiEntry {
    /// Lower case.
    pub key: String,
    pub values: Vec<String>,
    pub lines: Vec<String>,
}

/// How a texture blends (`blending`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Blending {
    #[default]
    Default,
    /// Added to what is behind (black is transparent), drawn last.
    Additive,
    /// Cut out at half alpha.
    Punchthrough,
}

/// A parsed TXI.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Txi {
    pub entries: Vec<TxiEntry>,
}

impl Txi {
    pub fn parse(data: &[u8]) -> Txi {
        let text = String::from_utf8_lossy(data);
        let mut lines = text.lines();
        let mut entries = Vec::new();
        while let Some(line) = lines.next() {
            let line = line.split('#').next().unwrap_or_default();
            let mut words = line.split_whitespace();
            let Some(key) = words.next() else { continue };
            let key = key.to_ascii_lowercase();
            let values: Vec<String> = words.map(str::to_string).collect();
            let mut extra = Vec::new();
            if LISTS.contains(&key.as_str()) {
                let n = values.first().and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
                for _ in 0..n {
                    match lines.next() {
                        Some(l) => extra.push(l.trim().to_string()),
                        None => break,
                    }
                }
            }
            entries.push(TxiEntry { key, values, lines: extra });
        }
        Txi { entries }
    }

    /// The last entry for a keyword (later lines win).
    pub fn get(&self, key: &str) -> Option<&TxiEntry> {
        self.entries.iter().rev().find(|e| e.key.eq_ignore_ascii_case(key))
    }

    /// A keyword's first value.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.get(key)?.values.first().map(String::as_str)
    }

    fn int(&self, key: &str) -> Option<i64> {
        self.value(key)?.parse().ok()
    }

    fn float(&self, key: &str) -> Option<f32> {
        self.value(key)?.parse().ok()
    }

    /// Mip-mapping (default on).
    pub fn mipmap(&self) -> bool {
        self.int("mipmap").is_none_or(|v| v != 0)
    }

    /// Linear filtering (default on).
    pub fn filter(&self) -> bool {
        self.int("filter").is_none_or(|v| v != 0)
    }

    /// Clamping: (u, v).
    pub fn clamp(&self) -> (bool, bool) {
        let c = self.int("clamp").unwrap_or(0);
        (c & 1 != 0, c & 2 != 0)
    }

    pub fn blending(&self) -> Blending {
        match self.value("blending").map(str::to_ascii_lowercase).as_deref() {
            Some("additive") => Blending::Additive,
            Some("punchthrough") => Blending::Punchthrough,
            _ => Blending::Default,
        }
    }

    /// Unlit, drawn over coplanar geometry.
    pub fn decal(&self) -> bool {
        self.int("decal").is_some_and(|v| v != 0)
    }

    /// The environment map (`default`: the area's).
    pub fn envmap(&self) -> Option<&str> {
        self.value("envmaptexture")
    }

    /// A cube map stored as six files `name0`…`name5`.
    pub fn cube(&self) -> bool {
        self.int("cube").is_some_and(|v| v != 0)
    }

    pub fn alpha_mean(&self) -> Option<f32> {
        self.float("alphamean")
    }

    /// A flip-book (`proceduretype cycle`): columns, rows, frames per second.
    pub fn cycle(&self) -> Option<(u32, u32, f32)> {
        if !self.value("proceduretype")?.eq_ignore_ascii_case("cycle") {
            return None;
        }
        let n = |k| self.int(k).and_then(|v| u32::try_from(v).ok()).filter(|&v| v > 0);
        Some((n("numx")?, n("numy")?, self.float("fps").unwrap_or(1.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_values_and_lists() {
        let t = Txi::parse(
            b"proceduretype cycle\r\nNumX 4\nnumy 4\nfps 32\nblending additive # fire\n\
              channelscale 2\n1 1 1\n0.5 0.5 0.5\nmipmap 0\nclamp 3\n",
        );
        assert_eq!(t.cycle(), Some((4, 4, 32.0)));
        assert_eq!(t.blending(), Blending::Additive);
        assert_eq!(t.get("channelscale").unwrap().lines, ["1 1 1", "0.5 0.5 0.5"]);
        assert!(!t.mipmap());
        assert!(t.filter());
        assert_eq!(t.clamp(), (true, true));
        assert!(Txi::parse(b"").mipmap());
    }
}
