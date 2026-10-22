//! The game's own shaders, compiled for wgpu on the user's machine
//! (`docs/shaders-proposal.md`, milestone M0). A program is built the way
//! the engine builds it and taken to a validated naga module:
//!
//! 1. [`assemble`]: the engine's preamble ([`Preamble`]), then the `.shd`
//!    with its `#include`s spliced in as the load order resolves them
//!    ([`Sources`]: a hak's `inc_lighting` replaces the game's);
//! 2. glslang compiles it to Vulkan SPIR-V under relaxed rules (the loose
//!    uniforms of GLSL 330 gathered into one block);
//! 3. combined image samplers are split and inputs and outputs given
//!    locations, on the SPIR-V words;
//! 4. naga reads it, the position's depth is taken from OpenGL's clip
//!    space to wgpu's, every resource is bound in group 0 (uniform blocks,
//!    then each texture with its sampler) and the module is validated.
//!
//! A renderer then fits a program to its pipeline ([`Program::link_inputs`],
//! [`Program::read_input_as_uint`], [`Program::set_group`]).
//!
//! The [`Program`] carries the module and its [`Reflection`]: the uniform
//! block's members with their offsets and types, the textures and their
//! samplers, the inputs and outputs, all by the shader's names. A program
//! that doesn't compile is a [`ShaderError`] whose [`Diagnostic`]s name the
//! file and line (back through the splicing) and glslang's message.
//!
//! glslang is built from the sources its crate ships (C++, with `cc`); the
//! game's shaders are only ever read from the user's install and haks.

pub mod assemble;
mod cache;
mod glsl;
mod link;
mod reflect;
mod spirv;

use std::fmt;

use thiserror::Error;

pub use assemble::{
    Assembled, At, LineOrigin, Preamble, SourceFile, Sources, Stage, assemble, defines_main,
    program_stage,
};
pub use cache::{Cached, ProgramCache};
pub use link::AlphaKeep;
pub use reflect::{Member, Part, Reflection, Scalar, Texture, UniformBlock, ValueType, Varying};
pub use spirv::{DRAWS, LIGHT_ARRAYS, LIGHT_INDEX, LIGHT_TABLE, LIGHTS, ORDER};

/// A message about a program, where it applies.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// The file and line, when the message names one.
    pub at: Option<At>,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.at {
            Some(at) => write!(f, "{at}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

fn list(d: &[Diagnostic]) -> String {
    d.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")
}

/// Why a program didn't compile.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ShaderError {
    /// The program or an include isn't found, or includes nest too deep.
    #[error("{0}")]
    Include(Diagnostic),
    /// glslang refused it: what it said, at the files and lines it said it.
    #[error("{}", list(.0))]
    Compile(Vec<Diagnostic>),
    /// The SPIR-V has something the passes don't take (an array of samplers).
    #[error("SPIR-V: {0}")]
    Spirv(String),
    /// naga couldn't read or validate it.
    #[error("naga: {0}")]
    Naga(String),
    #[error("glslang could not be started")]
    NoCompiler,
}

impl ShaderError {
    /// What it says, as messages at their files and lines.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        match self {
            ShaderError::Include(d) => vec![d.clone()],
            ShaderError::Compile(d) => d.clone(),
            e => vec![Diagnostic { at: None, message: e.to_string() }],
        }
    }
}

/// A compiled program.
#[derive(Debug, Clone)]
pub struct Program {
    pub name: String,
    pub stage: Stage,
    /// The files it was assembled from, the preamble first.
    pub files: Vec<SourceFile>,
    /// Validated, with every resource bound (group 0).
    pub module: naga::Module,
    pub info: naga::valid::ModuleInfo,
    pub reflection: Reflection,
}

/// An error and the errors it came from, as one line.
fn chain(e: &dyn std::error::Error) -> String {
    let mut s = e.to_string();
    let mut source = e.source();
    while let Some(inner) = source {
        let text = inner.to_string();
        if !s.contains(&text) {
            s.push_str(": ");
            s.push_str(&text);
        }
        source = inner.source();
    }
    s
}

/// Compiles program `name` for `stage` from `sources`.
pub fn compile(
    name: &str,
    stage: Stage,
    preamble: &Preamble,
    sources: &dyn Sources,
) -> Result<Program, ShaderError> {
    compile_assembled(&assemble(name, stage, preamble, sources)?)
}

/// Compiles an assembled program.
pub fn compile_assembled(a: &Assembled) -> Result<Program, ShaderError> {
    let (words, io) = to_words(a)?;
    from_words(a, &words, &io)
}

/// glslang and the passes on the words: what [`ProgramCache`] keeps.
pub(crate) fn to_words(a: &Assembled) -> Result<(Vec<u32>, Vec<spirv::Io>), ShaderError> {
    let words = glsl::compile(a)?;
    let words = spirv::split_samplers(&words)?;
    let words = spirv::index_lights(&words)?;
    let words = spirv::add_spare_blocks(&words)?;
    let words = spirv::add_draw_index(&words)?;
    spirv::assign_locations(&words)
}

/// naga on the words: the program.
pub(crate) fn from_words(
    a: &Assembled,
    words: &[u32],
    io: &[spirv::Io],
) -> Result<Program, ShaderError> {
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let options = naga::front::spv::Options {
        // (No y flip: the game's shaders are OpenGL's, whose clip space has
        // y up as wgpu's has; only the depth range differs, below.)
        adjust_coordinate_space: false,
        strict_capabilities: false,
        block_ctx_dump_prefix: None,
    };
    let mut module = naga::front::spv::parse_u8_slice(&bytes, &options)
        .map_err(|e| ShaderError::Naga(chain(&e)))?;
    link::gl_depth_to_wgpu(&mut module).map_err(ShaderError::Naga)?;
    let reflection = reflect::bind(&mut module, io);
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    )
    .validate(&module)
    .map_err(|e| ShaderError::Naga(chain(e.as_inner())))?;
    Ok(Program {
        name: a.name.clone(),
        stage: a.stage,
        files: a.files.clone(),
        module,
        info,
        reflection,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use naga::ImageDimension;

    use super::*;

    /// Authored in the stock shaders' manner: loose uniforms (arrays and
    /// matrices among them), a `sampler2D` passed to a function, a cube
    /// map, varyings, an include with a guard included twice.
    fn sources() -> BTreeMap<String, String> {
        let inc = r#"
#ifndef INC_TEST
#define INC_TEST
uniform mat4 m_mvp;
uniform mat4 m_bones[MAX_NUM_BONES];
uniform int numLights;
uniform vec4 lightPosition[MAX_NUM_LIGHTS];
uniform float fSwaySpeed;
#if SHADER_TYPE == 1
attribute vec4 vPos;
attribute vec2 vTcIn;
attribute vec3 vNormal;
#endif
varying vec2 vTexCoord;
varying vec3 vNormalView;
#endif
"#;
        let vs = r#"
#define SHADER_TYPE 1
#include "inc_test"
#include "inc_test"
void main ()
{
    vTexCoord = vTcIn;
    vNormalView = vNormal * fSwaySpeed;
    gl_Position = m_mvp * m_bones[numLights] * vPos;
}
"#;
        let fs = r#"
#define SHADER_TYPE 2
#include "inc_test"
uniform sampler2D texUnit0;
uniform sampler2D texUnit4;
uniform samplerCube texUnitEnvCube;
void Shift(inout vec2 uv, sampler2D height)
{
    uv += texture2D(height, uv).rg * lightPosition[0].xy;
}
void main ()
{
    vec2 uv = vTexCoord;
    Shift(uv, texUnit4);
    vec4 c = texture2D(texUnit0, uv) + textureCube(texUnitEnvCube, vNormalView);
    gl_FragColor = c * fSwaySpeed;
}
"#;
        [("inc_test", inc), ("vs_test", vs), ("fs_test", fs)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn a_vertex_program_compiles_with_its_uniforms_and_attributes() {
        let p = compile("vs_test", Stage::Vertex, &Preamble::default(), &sources()).unwrap();
        let r = &p.reflection;
        let (block, mvp) = r.uniform("m_mvp").unwrap();
        assert_eq!(block.name, "gl_DefaultUniformBlock");
        assert_eq!((block.binding, mvp.ty, mvp.size), (0, ValueType::Matrix(4, 4), 64));
        let bones = block.member("m_bones").unwrap();
        assert_eq!((bones.count, bones.stride, bones.size), (Some(64), 64, 4096));
        assert!(block.size >= 4096 + 64, "{block:?}");
        let mut offsets: Vec<u32> = block.members.iter().map(|m| m.offset).collect();
        offsets.dedup();
        assert_eq!(offsets.len(), block.members.len(), "members overlap: {block:?}");
        // Locations follow glslang's order of declaration (M1 sets its own).
        let mut inputs: Vec<_> = r.inputs.iter().map(|v| v.name.as_str()).collect();
        inputs.sort();
        assert_eq!(inputs, ["vNormal", "vPos", "vTcIn"]);
        let locations: Vec<_> = r.inputs.iter().map(|v| v.location).collect();
        assert_eq!(locations, [0, 1, 2]);
        let pos = r.inputs.iter().find(|v| v.name == "vPos").unwrap();
        assert_eq!(pos.ty, ValueType::Vector(4, Scalar::Float));
        let mut outputs: Vec<_> = r.outputs.iter().map(|v| v.name.as_str()).collect();
        outputs.sort();
        // (And the draw's places for the fragment program, flat.)
        assert_eq!(outputs, ["mg_draw", "mg_light", "vNormalView", "vTexCoord"]);
        assert!(r.textures.is_empty());
        let names: Vec<_> = p.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["(preamble)", "vs_test", "inc_test", "inc_test"]);
    }

    #[test]
    fn a_fragment_program_has_its_samplers_split_and_paired() {
        let p = compile("fs_test", Stage::Fragment, &Preamble::default(), &sources()).unwrap();
        let r = &p.reflection;
        let mut t: Vec<_> = r.textures.iter().map(|t| (t.name.as_str(), t.dim)).collect();
        t.sort_by_key(|t| t.0);
        assert_eq!(
            t,
            [
                ("texUnit0", ImageDimension::D2),
                ("texUnit4", ImageDimension::D2),
                ("texUnitEnvCube", ImageDimension::Cube),
            ]
        );
        // Each texture's sampler is bound next to it.
        assert!(r.textures.iter().all(|t| t.binding > 0 && t.sampler == Some(t.binding + 1)));
        let mut inputs: Vec<_> = r.inputs.iter().map(|v| v.name.as_str()).collect();
        inputs.sort();
        assert_eq!(inputs, ["mg_draw", "mg_light", "vNormalView", "vTexCoord"]);
        let outputs: Vec<_> = r.outputs.iter().map(|v| (v.name.as_str(), v.location)).collect();
        assert_eq!(outputs, [("compat_glFragColor", 0)]);
        assert!(r.uniform("fSwaySpeed").is_some() && r.uniform("lightPosition").is_some());
        // Every resource has a binding of its own (the two spare blocks a
        // split takes, the draws' and lights' storage and the frame's
        // lights last).
        let mut b: Vec<u32> = p
            .module
            .global_variables
            .iter()
            .filter_map(|(_, g)| g.binding.map(|b| b.binding))
            .collect();
        b.sort();
        assert_eq!(b, (0..12).collect::<Vec<_>>());
        // And the module writes as WGSL, as wgpu would take it.
        naga::back::wgsl::write_string(&p.module, &p.info, naga::back::wgsl::WriterFlags::empty())
            .unwrap();
    }

    #[test]
    fn the_preamble_decides_what_compiles() {
        let mut src = sources();
        src.insert(
            "fs_bones".into(),
            "#if MAX_NUM_BONES != 64\n#error bones\n#endif\nvoid main() { gl_FragColor = vec4(1.0); }\n"
                .into(),
        );
        assert!(compile("fs_bones", Stage::Fragment, &Preamble::default(), &src).is_ok());
        let pre = Preamble { max_bones: 32, ..Preamble::default() };
        let e = compile("fs_bones", Stage::Fragment, &pre, &src).unwrap_err();
        assert_eq!(e.diagnostics()[0].at.as_ref().unwrap().to_string(), "fs_bones.shd:2", "{e}");
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        let mut src = sources();
        // Truncated midway, binary, empty, unbalanced: each an error.
        let fs = src["fs_test"].clone();
        for cut in [1, fs.len() / 3, fs.len() / 2, fs.len() - 3] {
            src.insert("cut".into(), fs[..cut].to_string());
            assert!(compile("cut", Stage::Fragment, &Preamble::default(), &src).is_err(), "{cut}");
        }
        src.insert("bin".into(), String::from_utf8_lossy(&[0, 159, 146, 150, 255, 10, 0]).into());
        assert!(compile("bin", Stage::Vertex, &Preamble::default(), &src).is_err());
        src.insert("empty".into(), String::new());
        assert!(compile("empty", Stage::Vertex, &Preamble::default(), &src).is_err());
        src.insert("brace".into(), "void main() {\n".into());
        let e = compile("brace", Stage::Fragment, &Preamble::default(), &src).unwrap_err();
        assert!(!e.diagnostics().is_empty(), "{e}");
    }
}
