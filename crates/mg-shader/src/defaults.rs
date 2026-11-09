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
///
/// Only what the preprocessor keeps is read: nothing in a comment, and of
/// an `#if`, `#ifdef` or `#ifndef` with its `#elif`s and `#else` the
/// branch the program's `#define`s choose (the preamble's
/// `SHADER_QUALITY_MODE` among them), where its condition is of names
/// defined as whole numbers, `defined`, comparisons, `!`, `&&` and `||`.
/// Where a condition is more than that, its branches are all read, and a
/// uniform they declare with different values has no default.
pub fn uniform_defaults(text: &str) -> Vec<(String, UniformDefault)> {
    // Each name's first declaration: its value, if it is one of literals,
    // and whether it is certain (no condition this couldn't work out is
    // around it, and no other value such a condition may choose).
    let mut found: Vec<(String, Option<UniformDefault>, bool)> = Vec::new();
    let code = kept(text);
    for statement in code.split(';') {
        let certain = !statement.contains(UNSURE);
        let statement = statement.replace(UNSURE, "");
        // (After a function's body, which no `;` ends.)
        let s = statement.rsplit(['{', '}']).next().unwrap_or_default().trim();
        let Some(rest) = s.strip_prefix("uniform") else { continue };
        if !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let (declared, value) = match rest.split_once('=') {
            Some((declared, value)) => (declared, Some(value)),
            None => (rest, None),
        };
        let words: Vec<&str> = declared.split_whitespace().collect();
        // (`uniform lowp float x`: the type and the name are the last two.)
        let [.., ty, name] = words.as_slice() else { continue };
        if !is_name(name) {
            continue;
        }
        let value = value.and_then(|v| value_of(ty, v.trim()));
        match found.iter_mut().find(|(n, ..)| n == name) {
            // (Declared again where both are read for sure: the first.)
            Some((_, _, true)) if certain => {}
            Some((_, first, sure)) => {
                if *first != value {
                    *first = None;
                }
                *sure = false;
            }
            // (One with no value at all is no declaration to go by, unless
            // a condition may choose it over one that has.)
            None if value.is_some() || !certain => found.push((name.to_string(), value, certain)),
            None => {}
        }
    }
    found.into_iter().filter_map(|(name, value, _)| Some((name, value?))).collect()
}

/// Marks a line kept from under a condition that could not be worked out.
const UNSURE: char = '\u{1}';

/// Whether something holds, where it can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Known {
    Yes,
    No,
    Maybe,
}

impl Known {
    fn of(v: bool) -> Known {
        if v { Known::Yes } else { Known::No }
    }
    fn and(self, other: Known) -> Known {
        match (self, other) {
            (Known::No, _) | (_, Known::No) => Known::No,
            (Known::Yes, Known::Yes) => Known::Yes,
            _ => Known::Maybe,
        }
    }
    fn or(self, other: Known) -> Known {
        self.not().and(other.not()).not()
    }
    fn not(self) -> Known {
        match self {
            Known::Yes => Known::No,
            Known::No => Known::Yes,
            Known::Maybe => Known::Maybe,
        }
    }
}

/// The program's code as the preprocessor leaves it: comments and the
/// preprocessor's own lines out (a declaration's line may end in a
/// comment, and follow a `#define` with no `;` between), and the branches
/// of its conditions that are not taken. A line under a condition that
/// can't be worked out is kept, marked [`UNSURE`].
fn kept(text: &str) -> String {
    // (Name, whether it is defined, and its value where that is a number.)
    let mut macros: Vec<(String, Known, Option<i64>)> = Vec::new();
    // Each open condition: whether its lines are read now, whether one of
    // its branches was taken already, and whether the lines around it are.
    let mut open: Vec<(Known, Known, Known)> = Vec::new();
    let mut out = String::new();
    for line in without_comments(text).lines() {
        let now = open.last().map_or(Known::Yes, |o| o.0);
        let Some(directive) = line.trim_start().strip_prefix('#') else {
            match now {
                Known::Yes => out.push_str(line),
                Known::Maybe => {
                    out.push(UNSURE);
                    out.push_str(line);
                }
                Known::No => {}
            }
            out.push('\n');
            continue;
        };
        let directive = directive.trim_start();
        let word = directive.split(|c: char| !c.is_ascii_alphabetic()).next().unwrap_or_default();
        let rest = directive[word.len()..].trim();
        let name = rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).next();
        let name = name.unwrap_or_default();
        let defined = |macros: &[(String, Known, Option<i64>)]| {
            macros.iter().rev().find(|m| m.0 == name).map_or(Known::No, |m| m.1)
        };
        match word {
            "define" if now != Known::No && is_name(name) => {
                // (`NAME 2`; a macro with parameters, or of anything else,
                // is defined and no number.)
                let value = rest[name.len()..].strip_prefix(char::is_whitespace);
                let value = value.and_then(|v| number(v.trim(), &macros));
                macros.push((name.to_string(), now, value.filter(|_| now == Known::Yes)));
            }
            "undef" if now != Known::No && is_name(name) => {
                let gone = if now == Known::Yes { Known::No } else { Known::Maybe };
                macros.push((name.to_string(), gone, None));
            }
            "if" | "ifdef" | "ifndef" => {
                let holds = match word {
                    "if" => condition(rest, &macros),
                    "ifdef" => defined(&macros),
                    _ => defined(&macros).not(),
                };
                open.push((now.and(holds), holds, now));
            }
            "elif" | "else" => {
                if let Some((reads, taken, around)) = open.last_mut() {
                    let holds = if word == "else" { Known::Yes } else { condition(rest, &macros) };
                    *reads = around.and(taken.not()).and(holds);
                    *taken = taken.or(holds);
                }
            }
            "endif" => {
                open.pop();
            }
            _ => {}
        }
    }
    out
}

/// `text` with its comments blanked (a block comment's lines stay lines).
fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                while chars.peek().is_some_and(|c| *c != '\n') {
                    chars.next();
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    if c == '\n' {
                        out.push('\n');
                    }
                    last = c;
                }
                out.push(' ');
            }
            _ => out.push(c),
        }
    }
    out
}

/// A whole number as a condition holds it: a literal, or a name defined
/// as one.
fn number(word: &str, macros: &[(String, Known, Option<i64>)]) -> Option<i64> {
    if let Some(hex) = word.strip_prefix("0x").or_else(|| word.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    if word.starts_with(|c: char| c.is_ascii_digit()) {
        return word.trim_end_matches(['u', 'U']).parse().ok();
    }
    let found = macros.iter().rev().find(|m| m.0 == word)?;
    found.2.filter(|_| found.1 == Known::Yes)
}

/// Whether an `#if`'s condition holds: names defined as whole numbers and
/// literals, `defined(NAME)`, comparisons, `!`, `&&`, `||` and brackets.
/// Anything else (arithmetic, a name that is no number) can't be told.
fn condition(text: &str, macros: &[(String, Known, Option<i64>)]) -> Known {
    let mut tokens: Vec<&str> = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let len = match rest.chars().next() {
            Some(c) if word(c) => rest.find(|c| !word(c)).unwrap_or(rest.len()),
            _ if ["&&", "||", "==", "!=", "<=", ">="].iter().any(|op| rest.starts_with(op)) => 2,
            Some(c) => c.len_utf8(),
            None => break,
        };
        tokens.push(&rest[..len]);
        rest = rest[len..].trim_start();
    }
    let mut at = 0;
    let value = any_of(&tokens, &mut at, macros, 0);
    match value {
        Some(v) if at == tokens.len() => v,
        _ => Known::Maybe,
    }
}

/// `a || b || …` from token `at` on; `None` for what is not understood.
fn any_of(
    tokens: &[&str],
    at: &mut usize,
    macros: &[(String, Known, Option<i64>)],
    depth: usize,
) -> Option<Known> {
    let mut value = all_of(tokens, at, macros, depth)?;
    while tokens.get(*at) == Some(&"||") {
        *at += 1;
        value = value.or(all_of(tokens, at, macros, depth)?);
    }
    Some(value)
}

fn all_of(
    tokens: &[&str],
    at: &mut usize,
    macros: &[(String, Known, Option<i64>)],
    depth: usize,
) -> Option<Known> {
    let mut value = one(tokens, at, macros, depth)?;
    while tokens.get(*at) == Some(&"&&") {
        *at += 1;
        value = value.and(one(tokens, at, macros, depth)?);
    }
    Some(value)
}

/// One term: `!term`, `(…)`, `defined(NAME)`, `defined NAME`, a number,
/// or two numbers compared.
fn one(
    tokens: &[&str],
    at: &mut usize,
    macros: &[(String, Known, Option<i64>)],
    depth: usize,
) -> Option<Known> {
    // (Brackets as deep as a condition has them, and no deeper.)
    if depth > 32 {
        return None;
    }
    let token = *tokens.get(*at)?;
    *at += 1;
    match token {
        "!" => Some(one(tokens, at, macros, depth + 1)?.not()),
        "(" => {
            let value = any_of(tokens, at, macros, depth + 1)?;
            (tokens.get(*at) == Some(&")")).then(|| *at += 1)?;
            Some(value)
        }
        "defined" => {
            let bracket = tokens.get(*at) == Some(&"(");
            let name = *tokens.get(*at + usize::from(bracket))?;
            *at += 1 + usize::from(bracket);
            if bracket {
                (tokens.get(*at) == Some(&")")).then(|| *at += 1)?;
            }
            is_name(name)
                .then(|| macros.iter().rev().find(|m| m.0 == name).map_or(Known::No, |m| m.1))
        }
        word => {
            let left = number(word, macros);
            let compare: Option<fn(&i64, &i64) -> bool> = match tokens.get(*at).copied() {
                Some("==") => Some(i64::eq),
                Some("!=") => Some(i64::ne),
                Some("<") => Some(i64::lt),
                Some(">") => Some(i64::gt),
                Some("<=") => Some(i64::le),
                Some(">=") => Some(i64::ge),
                _ => None,
            };
            let Some(compare) = compare else {
                return Some(left.map_or(Known::Maybe, |v| Known::of(v != 0)));
            };
            *at += 1;
            let right = number(tokens.get(*at)?, macros);
            *at += 1;
            Some(match (left, right) {
                (Some(l), Some(r)) => Known::of(compare(&l, &r)),
                _ => Known::Maybe,
            })
        }
    }
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

    fn defaults(text: &str) -> Vec<(String, f32)> {
        let one = |d: UniformDefault| match d {
            UniformDefault::Float(v) => v[0],
            UniformDefault::Int(i) => i as f32,
        };
        uniform_defaults(text).into_iter().map(|(name, d)| (name, one(d))).collect()
    }

    /// Only what the preprocessor keeps is a declaration: of an `#if` on
    /// the preamble's defines, the branch taken (both were read, and the
    /// first's value was the uniform's at every quality setting: the
    /// review's M14); nothing in a comment.
    #[test]
    fn a_declaration_the_preprocessor_leaves_out_is_not_read() {
        let glow = "#if SHADER_QUALITY_MODE > 1\nuniform float fGlow = 1.0;\n#else\n\
                    uniform float fGlow = 0.25;\n#endif\n\
                    /* uniform float gone = 3.0;\n   uniform float gone2 = 3.0; */\n\
                    uniform float /* the cut */ kept = 0.5; // uniform float gone3 = 1.0;\n";
        let at = |quality: u8| {
            let preamble = crate::Preamble { quality, ..Default::default() };
            defaults(&format!("{}{glow}", preamble.text(crate::Stage::Fragment)))
        };
        let both = |glow: f32| vec![("fGlow".to_string(), glow), ("kept".to_string(), 0.5)];
        assert_eq!(at(0), both(0.25));
        assert_eq!(at(1), both(0.25));
        assert_eq!(at(2), both(1.0));
    }

    /// The conditions worked out: `#ifdef`, `#ifndef`, `#elif`, `defined`,
    /// `!`, `&&`, `||`, brackets, nesting, `#undef`, a name defined as
    /// another's number.
    #[test]
    fn conditions_choose_the_branch_that_is_read() {
        let text = "#define A 2\n#define B A\n#define FLAG\n#define GONE 1\n#undef GONE\n\
            #ifdef FLAG\nuniform float a = 1.0;\n#else\nuniform float a = 2.0;\n#endif\n\
            #ifndef FLAG\nuniform float b = 1.0;\n#elif B == 2\nuniform float b = 2.0;\n\
            #else\nuniform float b = 3.0;\n#endif\n\
            #if defined(GONE) || !defined FLAG\nuniform float c = 1.0;\n#elif (A >= 2 && B != 3)\n\
            uniform float c = 2.0;\n#elif 1\nuniform float c = 3.0;\n#endif\n\
            #if 0\n#if 1\nuniform float d = 1.0;\n#else\nuniform float d = 2.0;\n#endif\n\
            #define A 7\n#else\n#if A == 2\nuniform float d = 3.0;\n#endif\n#endif\n\
            #if 0x10 > 15 && !0\nuniform float e = 1.0;\n#endif\n\
            #if A\nuniform float f = 1.0;\n#endif\n";
        let all = [("a", 1.0), ("b", 2.0), ("c", 2.0), ("d", 3.0), ("e", 1.0), ("f", 1.0)];
        assert_eq!(defaults(text), all.map(|(n, v)| (n.to_string(), v)));
    }

    /// A condition that can't be worked out (arithmetic, a name that is no
    /// number, one never defined) has all its branches read: a uniform
    /// they give one value has it, one they give different values (or a
    /// value in one and none in another) has no default, and neither has
    /// one such a branch declares again after a declaration read for sure.
    #[test]
    fn a_condition_not_worked_out_gives_no_default_where_its_branches_differ() {
        let text = "#define N (1 + 1)\n\
            #if N > 1\nuniform float same = 1.0;\nuniform float differs = 1.0;\n\
            uniform float half = 1.0;\n#else\nuniform float same = 1.0;\n\
            uniform float differs = 2.0;\nuniform float half;\n#endif\n\
            uniform float sure = 4.0;\n\
            #if UNKNOWN_NAME\nuniform float sure = 5.0;\nuniform float alone = 6.0;\n#endif\n\
            #if 1 + 1\n#else\nuniform float alone2 = 7.0;\n#endif\n\
            uniform float after = 8.0;\n#if ((((\nuniform float odd = 9.0;\n#endif\n#endif\n#else\n";
        let all = [("same", 1.0), ("alone", 6.0), ("alone2", 7.0), ("after", 8.0), ("odd", 9.0)];
        assert_eq!(defaults(text), all.map(|(n, v)| (n.to_string(), v)));
    }

    /// No text makes it panic or run on: cut anywhere, unbalanced, deep.
    #[test]
    fn any_text_is_read_without_a_panic() {
        let text =
            "#if (A == 1) && defined(B)\n/* x */ uniform float a = 1.0; // y\n#elif !C\n#endif\n";
        for cut in 0..text.len() {
            let _ = uniform_defaults(&text[..cut]);
        }
        let deep = format!(
            "#if {}1{}\nuniform float a = 1.0;\n#endif\n",
            "(".repeat(5_000),
            ")".repeat(5_000)
        );
        assert_eq!(defaults(&deep), [("a".to_string(), 1.0)]);
        let _ = uniform_defaults(&"#if 1\n".repeat(50_000));
        let _ = uniform_defaults(&"#endif\n#else\n#elif\n/*".repeat(1_000));
    }
}
