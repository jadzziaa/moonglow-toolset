//! The values a program's source gives its uniforms where it declares
//! them (`uniform float rowHair = 0.172363;`): OpenGL keeps them as the
//! uniform's value until the application sets another, and the engine sets
//! only what the contract and the material's MTR name. glslang's relaxed
//! rules gather the loose uniforms into one block, which no initializer
//! reaches, so they are read from the text ([`Program::defaults`]).
//!
//! [`Program::defaults`]: crate::Program::defaults

/// A uniform's value as its declaration gives it.
#[derive(Debug, Clone, PartialEq)]
pub enum UniformDefault {
    /// A `float` or a float vector's components (`vec4(1.0)` is four).
    Float(Vec<f32>),
    /// An `int`, `uint` or `bool` (1 for `true`).
    Int(i32),
}

/// Each uniform declared with a value of literals in `text` (an assembled
/// program), by name, the first declaration of a name. A value that isn't
/// literals (a `#define`d name, an expression) is left out: the uniform is
/// zero, as it was.
pub fn uniform_defaults(text: &str) -> Vec<(String, UniformDefault)> {
    let mut out: Vec<(String, UniformDefault)> = Vec::new();
    // (Comments and the preprocessor's lines out: a declaration's line may
    // end in a comment, and follow a `#define` with no `;` between.)
    let code: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(|l| l.split("//").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    for statement in code.split(';') {
        // (After a function's body, which no `;` ends.)
        let s = statement.rsplit(['{', '}']).next().unwrap_or_default().trim();
        let Some(rest) = s.strip_prefix("uniform") else { continue };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((declared, value)) = rest.split_once('=') else { continue };
        let words: Vec<&str> = declared.split_whitespace().collect();
        // (`uniform lowp float x`: the type and the name are the last two.)
        let [.., ty, name] = words.as_slice() else { continue };
        if !is_name(name) || out.iter().any(|(n, _)| n == name) {
            continue;
        }
        if let Some(d) = value_of(ty, value.trim()) {
            out.push((name.to_string(), d));
        }
    }
    out
}

fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !s.starts_with(|c: char| c.is_ascii_digit())
}

/// `value` (`0.5`, `vec4(1.0, 1.0, 1.0, 1.0)`, `true`) as a uniform of
/// type `ty` holds it.
fn value_of(ty: &str, value: &str) -> Option<UniformDefault> {
    let number = |v: &str| -> Option<f32> {
        let v = v.trim().trim_end_matches(['f', 'F', 'u', 'U']);
        match v {
            "true" => Some(1.0),
            "false" => Some(0.0),
            _ => v.parse().ok(),
        }
    };
    let size = |prefix: &str| -> Option<usize> {
        ty.strip_prefix(prefix).and_then(|n| n.parse().ok()).filter(|n| (2..=4).contains(n))
    };
    match ty {
        "float" => Some(UniformDefault::Float(vec![number(value)?])),
        "int" | "uint" | "bool" => {
            let v = number(value)?;
            (v.fract() == 0.0).then_some(UniformDefault::Int(v as i32))
        }
        _ => {
            let n = size("vec")?;
            let args = value.strip_prefix(ty)?.trim();
            let args = args.strip_prefix('(')?.strip_suffix(')')?;
            let v: Vec<f32> = args.split(',').map(number).collect::<Option<_>>()?;
            match v.len() {
                1 => Some(UniformDefault::Float(vec![v[0]; n])),
                m if m == n => Some(UniformDefault::Float(v)),
                _ => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A builder's tinting shader's declarations (`fs_plt_tinter`), and
    /// what is left out.
    #[test]
    fn uniforms_declared_with_values_are_read() {
        let text = "#define SHADER_TYPE 2\n\
            uniform float rowHair   = 0.172363;\n\
            uniform vec4 illumColor = vec4(1.0, 1.0, 1.0, 1.0);\n\
            uniform float fAlphaClipRef = 0.01; // the cut\n\
            uniform lowp int texture9Bound;\n\
            uniform int xivOpacityUV = 0;\n\
            uniform vec4 RGBBalance = vec4 (0.5);\n\
            uniform bool on = true;\n\
            uniform int DOFFocusType = DOF_FOCUS_TYPE_CAMERA_PITCH;\n\
            uniform float later = 2.0;\n\
            uniform float rowHair = 9.0;\n\
            uniformfloat odd = 1.0;\n\
            float notUniform = 3.0;\n\
            void f() { }\n\
            uniform float afterBody = 4.0;\n";
        assert_eq!(
            uniform_defaults(text),
            [
                ("rowHair".to_string(), UniformDefault::Float(vec![0.172363])),
                ("illumColor".to_string(), UniformDefault::Float(vec![1.0; 4])),
                ("fAlphaClipRef".to_string(), UniformDefault::Float(vec![0.01])),
                ("xivOpacityUV".to_string(), UniformDefault::Int(0)),
                ("RGBBalance".to_string(), UniformDefault::Float(vec![0.5; 4])),
                ("on".to_string(), UniformDefault::Int(1)),
                ("later".to_string(), UniformDefault::Float(vec![2.0])),
                ("afterBody".to_string(), UniformDefault::Float(vec![4.0])),
            ]
        );
    }
}
