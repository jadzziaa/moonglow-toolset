//! A program's text as the engine builds it: its preamble of defines, then
//! the `.shd` with each `#include "name"` replaced by that file's text, as
//! the load order resolves it (notes §A.2). Every line of the result
//! remembers the file and line it came from, so that glslang's messages
//! can name them.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

use mg_core::ResType;
use mg_resman::{ResKey, ResMan};

use crate::{Diagnostic, ShaderError};

/// How deep includes may nest. The stock ones nest four deep; a file that
/// includes itself (a guard doesn't stop a text splice) stops here.
pub const MAX_DEPTH: usize = 16;

/// The longest name an `#include` takes: the engine reads it with
/// `#include "%16[^"]"`, so a longer one isn't an include to it (and
/// glslang then refuses the line).
pub const MAX_NAME: usize = 16;

/// A shader stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stage {
    Vertex,
    Fragment,
}

impl Stage {
    pub fn id(self) -> &'static str {
        match self {
            Stage::Vertex => "vertex",
            Stage::Fragment => "fragment",
        }
    }

    /// The stage a `.shd` declares: `#define SHADER_TYPE 1` is a vertex
    /// shader, `2` a fragment shader (`inc_common` takes 2 when none is
    /// defined). Every stock program and every custom one seen declares it.
    pub fn declared(text: &str) -> Option<Stage> {
        text.lines().find_map(|line| {
            let mut w = line.split_whitespace();
            (w.next() == Some("#define") && w.next() == Some("SHADER_TYPE")).then_some(())?;
            match w.next() {
                Some("1") => Some(Stage::Vertex),
                Some("2") => Some(Stage::Fragment),
                _ => None,
            }
        })
    }

    /// The stage the name says, as the game's programs are named: `vs…` and
    /// `…_vs` vertex, `fs…` and `…_fs` fragment.
    pub fn from_name(name: &str) -> Option<Stage> {
        let n = name.to_ascii_lowercase();
        if n.starts_with("inc_") {
            None
        } else if n.ends_with("_vs") || (n.starts_with("vs") && !n.ends_with("_fs")) {
            Some(Stage::Vertex)
        } else if n.ends_with("_fs") || n.starts_with("fs") {
            Some(Stage::Fragment)
        } else {
            None
        }
    }

    /// The stage of a program: declared, else named, else a fragment
    /// shader (what `inc_common` takes).
    pub fn of(name: &str, text: &str) -> Stage {
        Stage::declared(text).or_else(|| Stage::from_name(name)).unwrap_or(Stage::Fragment)
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Whether a `.shd`'s own text defines `main` (a program, not an include).
pub fn defines_main(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.split("//").next().unwrap_or_default();
        let mut w = line.split_whitespace();
        w.next() == Some("void") && w.next().is_some_and(|m| m == "main" || m.starts_with("main("))
    })
}

/// A `.shd`'s stage if it is a program (its own text defines `main`), as
/// [`Stage::of`] finds it; `None` for an include.
pub fn program_stage(name: &str, data: &[u8]) -> Option<Stage> {
    let text = String::from_utf8_lossy(data);
    defines_main(&text).then(|| Stage::of(name, &text))
}

/// The defines the engine puts before every shader (notes §A.2), with the
/// values of the game's settings they stand for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Preamble {
    /// `MAX_NUM_LIGHTS`: the client's "max lights" (3 to 128; 32 by default).
    pub max_lights: u32,
    /// `MAX_NUM_BONES`: 64 since 8193.21.
    pub max_bones: u32,
    /// `GAMMA_CORRECTION`: enhanced lighting.
    pub gamma_correction: bool,
    /// `FRAGMENT_LIGHTING`: per-pixel lighting (enhanced lighting).
    pub fragment_lighting: bool,
    /// `SHADER_QUALITY_MODE`: 0, 1, 2 for low, medium, high.
    pub quality: u8,
    /// `KEYHOLING_ENABLED`.
    pub keyholing: bool,
    /// `SHADER_DEBUG_MODE`: 0 to 18 (`inc_common`).
    pub debug_mode: u32,
    /// `BUILD_VERSION` and `BUILD_REVISION`: 8193 and 37 for 89.8193.37.
    pub build_version: u32,
    pub build_revision: u32,
    /// `NO_DISCARD`: per material, 1 for an opaque one (`discard` is then
    /// defined away), 0 for alpha-tested and keyholed ones.
    pub no_discard: bool,
    /// `POSTPROCESSING_TYPES_ENABLED`: a bit mask (notes §H).
    pub postprocessing: u32,
}

impl Default for Preamble {
    /// The game at its highest settings, keyholing on, a material that may
    /// discard: what the spike compiled every program with.
    fn default() -> Self {
        Preamble {
            max_lights: 32,
            max_bones: 64,
            gamma_correction: true,
            fragment_lighting: true,
            quality: 2,
            keyholing: true,
            debug_mode: 0,
            build_version: 8193,
            build_revision: 37,
            no_discard: false,
            postprocessing: 0,
        }
    }
}

impl Preamble {
    /// The preamble's text for a stage, ending in a newline.
    pub fn text(&self, stage: Stage) -> String {
        let b = |v: bool| u32::from(v);
        let mut s = format!(
            "#version 330 core\n\
             #define mediump\n\
             #define lowp\n\
             #define highp\n\
             #define MAX_NUM_LIGHTS {}\n\
             #define MAX_NUM_BONES {}\n\
             #define GAMMA_CORRECTION {}\n\
             #define FRAGMENT_LIGHTING {}\n\
             #define SHADER_QUALITY_MODE {}\n\
             #define KEYHOLING_ENABLED {}\n\
             #define SHADER_DEBUG_MODE {}\n\
             #define BUILD_VERSION {}\n\
             #define BUILD_REVISION {}\n\
             #define NO_DISCARD {}\n\
             #define POSTPROCESSING_TYPES_ENABLED {}\n\
             #define attribute in\n\
             #define texture2D texture\n\
             #define textureCube texture\n",
            self.max_lights,
            self.max_bones,
            b(self.gamma_correction),
            b(self.fragment_lighting),
            self.quality,
            b(self.keyholing),
            self.debug_mode,
            self.build_version,
            self.build_revision,
            b(self.no_discard),
            self.postprocessing,
        );
        s.push_str(match stage {
            Stage::Vertex => "#define varying out\n",
            Stage::Fragment => {
                "#define varying in\n\
                 #define gl_FragColor compat_glFragColor\n\
                 out vec4 compat_glFragColor;\n"
            }
        });
        s
    }
}

/// Where shaders come from: a name (no extension, any case) to its text
/// and the label of where it was found.
pub trait Sources {
    fn shader(&self, name: &str) -> Option<(Cow<'_, [u8]>, String)>;
}

/// Through the load order: the highest layer that has `name.shd`.
impl Sources for ResMan {
    fn shader(&self, name: &str) -> Option<(Cow<'_, [u8]>, String)> {
        let key = ResKey::parse(name, ResType::SHD)?;
        let layer = self.find(&key)?;
        let data = self.get(&key).ok()?;
        Some((data, self.layers()[layer].label.clone()))
    }
}

/// Texts by name (tests, a shader being edited): found as `memory`.
impl Sources for BTreeMap<String, String> {
    fn shader(&self, name: &str) -> Option<(Cow<'_, [u8]>, String)> {
        let (_, text) = self.iter().find(|(k, _)| k.eq_ignore_ascii_case(name))?;
        Some((Cow::Borrowed(text.as_bytes()), "memory".into()))
    }
}

/// A file a program was assembled from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceFile {
    /// Its name, lowercase, without `.shd`; `(preamble)` for the engine's.
    pub name: String,
    /// Where the load order found it: a layer's label (`hak:tdod_shaders`,
    /// `key:nwn_retail`), `memory`, or empty for the preamble.
    pub origin: String,
}

/// Where a line of an assembled program came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineOrigin {
    /// An index into [`Assembled::files`].
    pub file: u16,
    /// 1-based.
    pub line: u32,
}

/// A source position: a file of the program and its line.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct At {
    pub file: SourceFile,
    pub line: u32,
}

impl fmt::Display for At {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.shd:{}", self.file.name, self.line)
    }
}

/// A program's assembled text and where each of its lines came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assembled {
    pub name: String,
    pub stage: Stage,
    /// The preamble and the spliced files, as glslang reads it.
    pub text: String,
    /// The preamble first, then every file in the order it was first spliced.
    pub files: Vec<SourceFile>,
    /// One per line of `text`.
    pub lines: Vec<LineOrigin>,
}

impl Assembled {
    /// Where line `line` (1-based) of the text came from.
    pub fn origin(&self, line: u32) -> Option<At> {
        let o = self.lines.get(usize::try_from(line).ok()?.checked_sub(1)?)?;
        Some(At { file: self.files.get(usize::from(o.file))?.clone(), line: o.line })
    }

    /// The program's own file (the one named, after the preamble).
    pub fn program_file(&self) -> Option<&SourceFile> {
        self.files.get(1)
    }

    /// Whether a file of a layer `origin` took part (an include a hak overrides).
    pub fn uses_origin(&self, test: impl Fn(&str) -> bool) -> bool {
        self.files.iter().skip(1).any(|f| test(&f.origin))
    }
}

/// The name an `#include` line splices, as the engine reads it: `#include`
/// (after any indentation), then `"name"` of 1 to 16 characters.
pub fn include_name(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("#include")?.trim_start().strip_prefix('"')?;
    let (name, _) = rest.split_once('"')?;
    (!name.is_empty() && name.chars().count() <= MAX_NAME).then_some(name)
}

/// A file's bytes as text: Latin-1, as the engine's are 8-bit; carriage
/// returns dropped. A character outside ASCII (in a comment, as a rule)
/// becomes a backquote, which GLSL has no use for: glslang then refuses it
/// where it would matter, as the driver would.
fn text_of(data: &[u8]) -> String {
    data.iter()
        .filter(|&&b| b != b'\r')
        .map(|&b| match b {
            b'\n' | b'\t' | 0x0b | 0x0c => b as char,
            0x20..=0x7e => b as char,
            _ => '`',
        })
        .collect()
}

struct Splicer<'a> {
    sources: &'a dyn Sources,
    out: Assembled,
}

impl Splicer<'_> {
    fn file(&mut self, name: &str, origin: String) -> Result<u16, ShaderError> {
        let file = SourceFile { name: name.to_ascii_lowercase(), origin };
        let i = u16::try_from(self.out.files.len()).map_err(|_| {
            ShaderError::Include(Diagnostic { at: None, message: "too many includes".into() })
        })?;
        self.out.files.push(file);
        Ok(i)
    }

    fn splice(&mut self, name: &str, depth: usize, from: Option<At>) -> Result<(), ShaderError> {
        let Some((data, origin)) = self.sources.shader(name) else {
            let message = match &from {
                Some(_) => format!("the include {name:?} isn't in the haks or the game"),
                None => format!("the shader {name:?} isn't in the haks or the game"),
            };
            return Err(ShaderError::Include(Diagnostic { at: from, message }));
        };
        let text = text_of(&data);
        let file = self.file(name, origin)?;
        // Its lines, the last ended by the end of the file or a newline.
        let body = text.strip_suffix('\n').unwrap_or(&text);
        for (i, line) in body.split('\n').enumerate() {
            let line_no = u32::try_from(i + 1).unwrap_or(u32::MAX);
            if let Some(inc) = include_name(line) {
                let at = At { file: self.out.files[usize::from(file)].clone(), line: line_no };
                if depth >= MAX_DEPTH {
                    return Err(ShaderError::Include(Diagnostic {
                        at: Some(at),
                        message: format!(
                            "includes nest more than {MAX_DEPTH} deep here (does {inc:?} include itself?)"
                        ),
                    }));
                }
                self.splice(inc, depth + 1, Some(at))?;
            } else {
                self.out.text.push_str(line);
                self.out.text.push('\n');
                self.out.lines.push(LineOrigin { file, line: line_no });
            }
        }
        Ok(())
    }
}

/// Assembles program `name` for `stage`: the preamble, then the file with
/// its includes spliced in.
pub fn assemble(
    name: &str,
    stage: Stage,
    preamble: &Preamble,
    sources: &dyn Sources,
) -> Result<Assembled, ShaderError> {
    let mut s = Splicer {
        sources,
        out: Assembled {
            name: name.to_ascii_lowercase(),
            stage,
            text: String::new(),
            files: Vec::new(),
            lines: Vec::new(),
        },
    };
    let pre = s.file("(preamble)", String::new())?;
    for (i, line) in preamble.text(stage).lines().enumerate() {
        s.out.text.push_str(line);
        s.out.text.push('\n');
        s.out.lines.push(LineOrigin { file: pre, line: u32::try_from(i + 1).unwrap_or(0) });
    }
    s.splice(name, 0, None)?;
    Ok(s.out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(list: &[(&str, &str)]) -> BTreeMap<String, String> {
        list.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn includes_are_spliced_and_lines_remembered() {
        let src = files(&[
            ("prog", "#define SHADER_TYPE 2\n  #include \"Inc_A\"\nvoid main() {}\n"),
            ("inc_a", "#ifndef INC_A\r\n#define INC_A\r\n#include \"inc_b\" // b\r\n#endif\r\n"),
            ("inc_b", "float b;\n"),
        ]);
        let a = assemble("prog", Stage::Fragment, &Preamble::default(), &src).unwrap();
        let pre = Preamble::default().text(Stage::Fragment).lines().count() as u32;
        assert!(!a.text.contains("#include") && !a.text.contains('\r'));
        let names: Vec<_> = a.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["(preamble)", "prog", "inc_a", "inc_b"]);
        // `float b;` is the fourth line after the preamble: prog 1, inc_a 1, 2, inc_b 1.
        let at = a.origin(pre + 4).unwrap();
        assert_eq!((at.file.name.as_str(), at.line), ("inc_b", 1));
        assert_eq!(a.text.lines().nth(pre as usize + 3), Some("float b;"));
        let at = a.origin(pre + 6).unwrap();
        assert_eq!((at.to_string().as_str()), "prog.shd:3");
        assert_eq!(a.lines.len(), a.text.lines().count());
    }

    #[test]
    fn a_missing_include_is_named_at_its_line() {
        let src = files(&[("prog", "\n#include \"nowhere\"\n")]);
        let e = assemble("prog", Stage::Vertex, &Preamble::default(), &src).unwrap_err();
        let d = &e.diagnostics()[0];
        assert_eq!(d.at.as_ref().unwrap().to_string(), "prog.shd:2");
        assert!(d.message.contains("nowhere"), "{d:?}");
        let e = assemble("absent", Stage::Vertex, &Preamble::default(), &src).unwrap_err();
        assert!(e.diagnostics()[0].at.is_none());
    }

    #[test]
    fn a_file_that_includes_itself_stops_at_the_depth_limit() {
        let src = files(&[("loop", "#ifndef L\n#define L\n#include \"loop\"\n#endif\n")]);
        let e = assemble("loop", Stage::Fragment, &Preamble::default(), &src).unwrap_err();
        assert!(e.to_string().contains("loop.shd:3"), "{e}");
    }

    #[test]
    fn include_lines_as_the_engine_reads_them() {
        assert_eq!(include_name("#include \"inc_standard\""), Some("inc_standard"));
        assert_eq!(include_name("\t#include\"x\" // y"), Some("x"));
        assert_eq!(include_name("// #include \"x\""), None);
        assert_eq!(include_name("#include <x>"), None);
        assert_eq!(include_name("#include \"x"), None);
        assert_eq!(include_name("#include \"\""), None);
        assert_eq!(include_name("#include \"seventeen_chars_x\""), None);
        assert_eq!(include_name("#include \"sixteen_chars_xx\""), Some("sixteen_chars_xx"));
    }

    #[test]
    fn stages_and_programs() {
        assert_eq!(Stage::declared("#define  SHADER_TYPE 1 // vs"), Some(Stage::Vertex));
        assert_eq!(Stage::declared("#if SHADER_TYPE == 1"), None);
        assert_eq!(Stage::from_name("shd_windgrass_vs"), Some(Stage::Vertex));
        assert_eq!(Stage::from_name("vslit_sm"), Some(Stage::Vertex));
        assert_eq!(Stage::from_name("fs_pltgen"), Some(Stage::Fragment));
        assert_eq!(Stage::from_name("inc_lighting"), None);
        assert_eq!(Stage::of("odd", ""), Stage::Fragment);
        assert!(defines_main("void main ()\n{}"));
        assert!(defines_main("  void main(){}"));
        assert!(!defines_main("// void main()\nvoid mainly()"));
    }

    #[test]
    fn bytes_outside_ascii_do_not_reach_glslang_as_they_are() {
        let t = text_of(b"// caf\xe9\r\nfloat a;\0\n");
        assert_eq!(t, "// caf`\nfloat a;`\n");
    }
}
