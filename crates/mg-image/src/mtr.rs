//! MTR: an EE material. One directive per line, `//` comments, keywords
//! case-insensitive: textures for the shader's slots (0 diffuse, 1 normal,
//! 2 specular, 3 roughness, 4 height, 5 self-illumination, 6–10 custom),
//! custom shaders, parameters and render flags.

/// Texture slots an MTR can bind.
pub const SLOTS: usize = 16;

/// A shader parameter's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Float(Vec<f32>),
    Int(i32),
}

/// How the mesh is lit and which tangents it needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderHint {
    #[default]
    None,
    NormalAndSpecMapped,
    NormalTangents,
}

/// A parsed material.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mtr {
    /// Texture per slot (`null` or absent: none).
    pub textures: [Option<String>; SLOTS],
    pub renderhint: RenderHint,
    pub shader_vs: Option<String>,
    pub shader_fs: Option<String>,
    pub shader_gs: Option<String>,
    pub params: Vec<(String, Param)>,
    /// Drawn in the transparent pass.
    pub transparency: bool,
    pub twosided: bool,
    pub sample_framebuffer: u8,
    pub volumetric: bool,
}

fn name(v: Option<&str>) -> Option<String> {
    v.filter(|s| !s.eq_ignore_ascii_case("null") && !s.is_empty()).map(str::to_ascii_lowercase)
}

impl Mtr {
    pub fn parse(data: &[u8]) -> Mtr {
        let text = String::from_utf8_lossy(data);
        let mut m = Mtr::default();
        let mut bitmap = None;
        for line in text.lines() {
            let line = line.split("//").next().unwrap_or_default();
            let mut words = line.split_whitespace();
            let Some(key) = words.next() else { continue };
            let key = key.to_ascii_lowercase().replace('_', "");
            let args: Vec<&str> = words.collect();
            let first = args.first().copied();
            let flag = || first.and_then(|v| v.parse::<i32>().ok()).is_some_and(|v| v != 0);
            match key.as_str() {
                "bitmap" => bitmap = name(first),
                "renderhint" => {
                    m.renderhint = match first.map(str::to_ascii_lowercase).as_deref() {
                        Some("normalandspecmapped") => RenderHint::NormalAndSpecMapped,
                        Some("normaltangents") => RenderHint::NormalTangents,
                        _ => RenderHint::None,
                    }
                }
                "customshadervs" => m.shader_vs = name(first),
                "customshaderfs" => m.shader_fs = name(first),
                "customshadergs" => m.shader_gs = name(first),
                "parameter" if args.len() >= 3 => {
                    let value = if args[0].eq_ignore_ascii_case("int") {
                        args[2].parse().ok().map(Param::Int)
                    } else {
                        let v: Vec<f32> = args[2..].iter().filter_map(|a| a.parse().ok()).collect();
                        (!v.is_empty()).then_some(Param::Float(v))
                    };
                    if let Some(v) = value {
                        m.params.push((args[1].to_string(), v));
                    }
                }
                "transparency" => m.transparency = flag(),
                "twosided" => m.twosided = flag(),
                "volumetric" => m.volumetric = flag(),
                "sampleframebuffer" => {
                    m.sample_framebuffer = first.and_then(|v| v.parse().ok()).unwrap_or(0)
                }
                k => {
                    if let Some(slot) =
                        k.strip_prefix("texture").and_then(|n| n.parse::<usize>().ok())
                        && slot < SLOTS
                    {
                        m.textures[slot] = name(first);
                    }
                }
            }
        }
        if m.textures[0].is_none() {
            m.textures[0] = bitmap;
        }
        m
    }

    /// A float parameter (case-insensitive name).
    pub fn float(&self, param: &str) -> Option<&[f32]> {
        self.params.iter().find(|(n, _)| n.eq_ignore_ascii_case(param)).and_then(|(_, v)| match v {
            Param::Float(f) => Some(f.as_slice()),
            Param::Int(_) => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives() {
        let m = Mtr::parse(
            b"// ice\r\ncustomshadervs vslit_sm\ncustomshaderfs fslit_sm\n\
              parameter float Metallicness 0.9\nparameter float CustomSpecularColor 1 0.5 0\n\
              parameter int Mode 2\nTexture1 ice_n\ntexture2 null\nbitmap ice\ntwo_sided 1\n",
        );
        assert_eq!(m.shader_fs.as_deref(), Some("fslit_sm"));
        assert_eq!(m.float("metallicness"), Some(&[0.9][..]));
        assert_eq!(m.float("CustomSpecularColor"), Some(&[1.0, 0.5, 0.0][..]));
        assert_eq!(m.params[2].1, Param::Int(2));
        assert_eq!(m.textures[0].as_deref(), Some("ice"));
        assert_eq!(m.textures[1].as_deref(), Some("ice_n"));
        assert_eq!(m.textures[2], None);
        assert!(m.twosided);
    }
}
