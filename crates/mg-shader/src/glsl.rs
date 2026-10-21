//! glslang: an assembled program to Vulkan SPIR-V, under glslang's relaxed
//! Vulkan rules (`glslangValidator -V -R`): the loose uniforms of GLSL 330
//! are gathered into one block, `gl_DefaultUniformBlock`. Locations and
//! bindings are left to [`crate::spirv`] and the IR (glslang-rs links and
//! generates in one call, so glslang's own mapping never runs).

use std::panic::{AssertUnwindSafe, catch_unwind};

use glslang::{
    Compiler, CompilerOptions, Shader, ShaderInput, ShaderMessage, ShaderOptions, ShaderSource,
    ShaderStage, SourceLanguage, SpirvVersion, Target, VulkanVersion,
};

use crate::assemble::{Assembled, Stage};
use crate::{Diagnostic, ShaderError};

/// The program as SPIR-V words.
pub(crate) fn compile(a: &Assembled) -> Result<Vec<u32>, ShaderError> {
    let compiler = Compiler::acquire().ok_or(ShaderError::NoCompiler)?;
    let stage = match a.stage {
        Stage::Vertex => ShaderStage::Vertex,
        Stage::Fragment => ShaderStage::Fragment,
    };
    let text = a.text.clone();
    // glslang-rs panics on what it doesn't expect (a log that isn't UTF-8):
    // that is this program's error, not the caller's.
    let run = catch_unwind(AssertUnwindSafe(|| -> Result<Vec<u32>, String> {
        let source = ShaderSource::from(text);
        let options = CompilerOptions {
            source_language: SourceLanguage::GLSL,
            target: Target::Vulkan {
                version: VulkanVersion::Vulkan1_0,
                spirv_version: SpirvVersion::SPIRV1_0,
            },
            version_profile: None,
            messages: ShaderMessage::DEFAULT,
        };
        let input =
            ShaderInput::new(&source, stage, &options, None, None).map_err(|e| e.to_string())?;
        let mut builder = Shader::new(compiler, input);
        builder.options(
            ShaderOptions::VULKAN_RULES_RELAXED
                | ShaderOptions::AUTO_MAP_BINDINGS
                | ShaderOptions::AUTO_MAP_LOCATIONS,
        );
        let shader = builder.parse().map_err(log)?;
        shader.compile().map_err(log)
    }));
    match run {
        Ok(Ok(words)) => Ok(words),
        Ok(Err(log)) => Err(ShaderError::Compile(diagnostics(a, &log))),
        Err(_) => Err(ShaderError::Compile(vec![Diagnostic {
            at: None,
            message: "glslang stopped on this program".into(),
        }])),
    }
}

/// The log of a glslang error.
fn log(e: glslang::error::GlslangError) -> String {
    use glslang::error::GlslangError as E;
    match e {
        E::PreprocessError(l) | E::ParseError(l) | E::MapIoError(l) | E::LinkError(l) => l.log,
        e => e.to_string(),
    }
}

/// glslang's log as messages at their files and lines. Its lines read
/// `ERROR: 0:123: 'x' : undeclared identifier` (string 0, line 123 of the
/// assembled text). What follows the first error (`'' : compilation
/// terminated`, `ERROR: 2 compilation errors. No code generated.`) is
/// left out.
pub(crate) fn diagnostics(a: &Assembled, log: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for line in log.lines() {
        let Some(rest) = line.strip_prefix("ERROR: ") else { continue };
        if rest.contains("compilation errors") || rest.contains("compilation error.") {
            continue;
        }
        let mut parts = rest.splitn(3, ':');
        let (string, number, message) = (parts.next(), parts.next(), parts.next());
        let at = match (string, number, message) {
            (Some(s), Some(n), Some(_)) if s.trim().parse::<u32>().is_ok() => {
                n.trim().parse::<u32>().ok().and_then(|n| a.origin(n))
            }
            _ => None,
        };
        let message = match (&at, message) {
            (Some(_), Some(m)) => m.trim(),
            _ => rest.trim(),
        };
        if message.ends_with("compilation terminated") && !out.is_empty() {
            continue;
        }
        out.push(Diagnostic { at, message: message.to_string() });
    }
    if out.is_empty() {
        let message =
            log.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("glslang failed");
        out.push(Diagnostic { at: None, message: message.to_string() });
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::assemble::{Preamble, assemble};

    #[test]
    fn messages_are_placed_in_their_files() {
        let src: BTreeMap<String, String> = [
            ("prog".to_string(), "#include \"inc\"\nvoid main() { x = 1.0; }\n".to_string()),
            ("inc".to_string(), "float a;\nfloat b = c;\n".to_string()),
        ]
        .into();
        let a = assemble("prog", Stage::Fragment, &Preamble::default(), &src).unwrap();
        let ShaderError::Compile(d) = compile(&a).unwrap_err() else { panic!() };
        // glslang stops at the first error in a declaration.
        assert_eq!(d[0].at.as_ref().map(ToString::to_string).as_deref(), Some("inc.shd:2"));
        assert!(d[0].message.contains("'c'") && d[0].message.contains("undeclared"), "{d:?}");
    }

    #[test]
    fn a_log_without_lines_is_kept_whole() {
        let src: BTreeMap<String, String> = [("p".to_string(), String::new())].into();
        let a = assemble("p", Stage::Fragment, &Preamble::default(), &src).unwrap();
        let d = diagnostics(&a, "ERROR: Linking fragment stage: Missing entry point\n");
        assert_eq!(d.len(), 1);
        assert!(d[0].at.is_none() && d[0].message.contains("Missing entry point"), "{d:?}");
        let d = diagnostics(
            &a,
            "ERROR: 0:99999: 'x' : oops\nERROR: 0:99999: '' : compilation terminated\n\
             ERROR: 1 compilation errors.  No code generated.\n",
        );
        assert!(d.len() == 1 && d[0].at.is_none(), "{d:?}");
    }
}
