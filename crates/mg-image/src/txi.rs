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

    /// The game's rippling of a texture (`proceduretype arturo`, with
    /// `distort`: what its water has): how far the picture is pushed about,
    /// as a part of its width (`distortionamplitude` texels of
    /// `defaultwidth`), and how fast (`speed`). The client's own rule is
    /// [`Txi::ripple_texels`]'s, which the renderer draws by.
    pub fn ripple(&self) -> Option<(f32, f32)> {
        if !self.value("proceduretype")?.eq_ignore_ascii_case("arturo") {
            return None;
        }
        if self.int("distort").is_some_and(|v| v == 0) {
            return None;
        }
        let width = self.float("defaultwidth").filter(|w| *w > 0.0).unwrap_or(64.0);
        let amplitude = self.float("distortionamplitude").unwrap_or(0.0) / width;
        (amplitude > 0.0).then(|| (amplitude, self.float("speed").unwrap_or(20.0)))
    }

    /// Whether the texture runs the game's rippling procedure
    /// (`proceduretype arturo`), whatever its `distort`.
    pub fn arturo(&self) -> bool {
        self.value("proceduretype").is_some_and(|v| v.eq_ignore_ascii_case("arturo"))
    }

    /// The game's rippling of a texture as the client draws it
    /// ([`Txi::arturo`] with `distort` not 0): how far its picture is
    /// pushed about, `distortionamplitude` in texels of the texture itself
    /// (whatever `defaultwidth` says), and how fast (`speed`, 20 without
    /// one). Read in the client (`batch6_ripple`; the findings, "How far
    /// and how fast a texture ripples"), where [`Txi::ripple`] took the
    /// amplitude as a part of `defaultwidth`.
    pub fn ripple_texels(&self) -> Option<(f32, f32)> {
        if !self.arturo() || self.int("distort").is_some_and(|v| v == 0) {
            return None;
        }
        let amplitude = self.float("distortionamplitude").unwrap_or(0.0);
        (amplitude > 0.0).then(|| (amplitude, self.float("speed").unwrap_or(20.0)))
    }

    /// What the rippling procedure with `distort 0` makes of the
    /// texture's channels (red, green, blue, alpha): each `c × scale +
    /// translate`, by `channelscale` and `channeltranslate` (1 and 0 where
    /// a value is missing), no ripple (the client draws the Domains of
    /// Dread's zombie's scale 0 and translation 1 white: `batch6_ripple`).
    /// `None` for a texture without the procedure, or one that ripples.
    pub fn channels(&self) -> Option<([f32; 4], [f32; 4])> {
        if !self.arturo() || self.int("distort") != Some(0) {
            return None;
        }
        let list = |key: &str, default: f32| {
            let lines = self.get(key).map(|e| e.lines.as_slice()).unwrap_or_default();
            std::array::from_fn(|i| {
                lines
                    .get(i)
                    .and_then(|l| l.split_whitespace().next()?.parse().ok())
                    .unwrap_or(default)
            })
        };
        Some((list("channelscale", 1.0), list("channeltranslate", 0.0)))
    }

    /// The game's water (`bumpmaptexture shinywater`): what Enhanced
    /// Edition draws with its water shader, waves and reflections, where
    /// the old game rippled the texture ([`Txi::ripple`]).
    pub fn water(&self) -> bool {
        self.value("bumpmaptexture").is_some_and(|v| v.eq_ignore_ascii_case("shinywater"))
    }

    /// Unlit, drawn over coplanar geometry.
    pub fn decal(&self) -> bool {
        self.int("decal").is_some_and(|v| v != 0)
    }

    /// The environment map, `envmaptexture` (`default`: the object's or
    /// area's). Not `bumpyshinytexture`, which Enhanced Edition reads
    /// nothing of: a texture naming one is drawn with the plain lit
    /// programs, its alpha its opacity, reflecting the area's map as any
    /// other (read from the client: `program_choice_in_the_client` in
    /// `mg-corpus-tests`' `client_render.rs`).
    pub fn envmap(&self) -> Option<&str> {
        self.value("envmaptexture")
    }

    /// `bumpyshinytexture`: what the game's shiny water reflects (its
    /// water TXIs name a cube map there).
    pub fn bumpy_shiny(&self) -> Option<&str> {
        self.value("bumpyshinytexture")
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

    #[test]
    fn water_ripples() {
        // The game's ttr01_water01.txi, in short.
        let txi = Txi::parse(
            b"proceduretype arturo\ndistort 1\narturowidth 32\ndistortionamplitude 6\nspeed 20\n\
              defaultwidth 64\n",
        );
        assert_eq!(txi.ripple(), Some((6.0 / 64.0, 20.0)));
        assert!(!txi.water());
        assert!(Txi::parse(b"bumpyshinytexture ttr01__env\nbumpmaptexture shinywater\n").water());
        assert_eq!(Txi::parse(b"proceduretype cycle\nnumx 2\nnumy 2\n").ripple(), None);
        // In texels of the texture, `defaultwidth` changing nothing.
        assert_eq!(txi.ripple_texels(), Some((6.0, 20.0)));
        assert_eq!(txi.channels(), None);
        assert!(txi.arturo());
    }

    /// The Domains of Dread's `dd_ce3_txhotcoal`, and the same with
    /// `distort 0`: its channels' scale and translation, no ripple.
    #[test]
    fn a_ripple_without_distortion_scales_the_channels() {
        let hot = "proceduretype arturo\ndistort 1\narturowidth 32\narturoheight 32\n\
                   distortionamplitude 10\nspeed 16\nchannelscale 4\n0\n0\n0\n0\n\
                   channeltranslate 4\n1\n1\n1\n1\n";
        let txi = Txi::parse(hot.as_bytes());
        assert_eq!(txi.ripple_texels(), Some((10.0, 16.0)));
        assert_eq!(txi.channels(), None);
        let flat = Txi::parse(hot.replace("distort 1", "distort 0").as_bytes());
        assert_eq!(flat.ripple_texels(), None);
        assert_eq!(flat.channels(), Some(([0.0; 4], [1.0; 4])));
        let bare = Txi::parse(b"proceduretype Arturo\ndistort 0\nchannelscale 2\n0.5\n0.25\n");
        assert_eq!(bare.channels(), Some(([0.5, 0.25, 1.0, 1.0], [0.0; 4])));
        assert!(Txi::parse(b"distort 0\n").channels().is_none());
    }
}
