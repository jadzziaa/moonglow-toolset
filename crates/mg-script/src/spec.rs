//! The language spec, `nwscript.nss`: the engine's functions (in action-id
//! order, with signatures and the comment block above each as
//! documentation), its constants and its engine structure types. Drives the
//! script editor's completion, signature help and function browser.

use crate::lex::{Token, TokenKind, tokenize};

/// A function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub ty: String,
    pub name: String,
    /// The default value as written (e.g. `OBJECT_SELF`, `1.0f`,
    /// `[0.0,0.0,0.0]`).
    pub default: Option<String>,
}

/// An engine function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// The action id (position in the spec).
    pub id: usize,
    pub return_type: String,
    pub name: String,
    pub params: Vec<Param>,
    /// The `//` comment lines directly above the declaration, without `//`.
    pub doc: String,
    /// The same lines as written, `//` and the spaces before it too
    /// ([`comment_as_written`](crate::outline::comment_as_written)):
    /// `nwscript.nss`'s aligned columns line up in it.
    pub doc_source: String,
}

impl Function {
    /// `float GetDistanceToObject(object oObject)`.
    pub fn signature(&self) -> String {
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| match &p.default {
                Some(d) => format!("{} {}={d}", p.ty, p.name),
                None => format!("{} {}", p.ty, p.name),
            })
            .collect();
        format!("{} {}({})", self.return_type, self.name, params.join(", "))
    }
}

/// A constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constant {
    pub ty: String,
    pub name: String,
    /// The value as written.
    pub value: String,
    pub doc: String,
    /// [`Function::doc_source`].
    pub doc_source: String,
}

/// A parsed language spec.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spec {
    pub functions: Vec<Function>,
    pub constants: Vec<Constant>,
    /// Engine structure types (`effect`, `event`, `location`, ...), by index.
    pub engine_structures: Vec<String>,
}

impl Spec {
    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }

    pub fn constant(&self, name: &str) -> Option<&Constant> {
        self.constants.iter().find(|c| c.name == name)
    }

    /// Parses `nwscript.nss`. Anything that is not a constant, a function
    /// declaration or an engine-structure define is skipped.
    pub fn parse(src: &[u8]) -> Spec {
        let text = String::from_utf8_lossy(src);
        let toks = tokenize(text.as_bytes());
        let mut spec = Spec::default();
        // (Each comment line as written: `doc` without the `//`.)
        let mut doc: Vec<&str> = Vec::new();
        let mut blank_lines = 0;
        let mut i = 0;
        let s = |t: &Token| &text[t.span.clone()];
        while i < toks.len() {
            let t = &toks[i];
            match t.kind {
                TokenKind::LineComment => {
                    if blank_lines > 0 {
                        doc.clear();
                    }
                    doc.push(crate::outline::comment_as_written(&text, &t.span));
                    blank_lines = 0;
                    i += 1;
                    continue;
                }
                TokenKind::Whitespace => {
                    blank_lines += s(t).matches('\n').count().saturating_sub(1);
                    i += 1;
                    continue;
                }
                TokenKind::BlockComment { .. } => {
                    i += 1;
                    continue;
                }
                _ => {}
            }
            // A statement: collect significant tokens up to `;` or the end of
            // a `#define` line.
            if t.kind == TokenKind::Directive {
                let mut words = Vec::new();
                i += 1;
                while i < toks.len()
                    && !(toks[i].kind == TokenKind::Whitespace && s(&toks[i]).contains('\n'))
                {
                    if toks[i].kind.is_significant() {
                        words.push(s(&toks[i]).to_string());
                    }
                    i += 1;
                }
                if s(t) == "#define"
                    && let [name, value] = words.as_slice()
                    && let Some(n) =
                        name.strip_prefix("ENGINE_STRUCTURE_").and_then(|n| n.parse::<usize>().ok())
                {
                    if spec.engine_structures.len() <= n {
                        spec.engine_structures.resize(n + 1, String::new());
                    }
                    spec.engine_structures[n] = value.clone();
                }
                doc.clear();
                continue;
            }
            let mut stmt: Vec<&Token> = Vec::new();
            while i < toks.len() && !(toks[i].kind == TokenKind::Punct && s(&toks[i]) == ";") {
                if toks[i].kind.is_significant() {
                    stmt.push(&toks[i]);
                }
                i += 1;
            }
            i += 1; // the `;`
            let words: Vec<&str> = stmt.iter().map(|t| s(t)).collect();
            // A comment block counts as documentation only directly above.
            let (docs, doc_source) = if blank_lines > 0 {
                Default::default()
            } else {
                let plain: Vec<&str> =
                    doc.iter().map(|l| crate::outline::comment_text(l)).collect();
                (plain.join("\n"), doc.join("\n"))
            };
            doc.clear();
            blank_lines = 0;
            let words = match words.first() {
                Some(&"const") => &words[1..],
                _ => &words[..],
            };
            match words {
                [ty, name, "=", value @ ..] if !value.is_empty() => spec.constants.push(Constant {
                    ty: ty.to_string(),
                    name: name.to_string(),
                    value: value.concat(),
                    doc: docs,
                    doc_source,
                }),
                [ret, name, "(", rest @ ..] if rest.last() == Some(&")") => {
                    let params = parse_params(&rest[..rest.len() - 1]);
                    spec.functions.push(Function {
                        id: spec.functions.len(),
                        return_type: ret.to_string(),
                        name: name.to_string(),
                        params,
                        doc: docs,
                        doc_source,
                    });
                }
                _ => {}
            }
        }
        spec
    }
}

/// `type name [= default], ...` from the tokens between the parentheses.
fn parse_params(words: &[&str]) -> Vec<Param> {
    let mut params = Vec::new();
    let mut depth = 0;
    let mut current: Vec<&str> = Vec::new();
    let mut flush = |current: &mut Vec<&str>| {
        if let [ty, name, rest @ ..] = current.as_slice() {
            let default = match rest {
                ["=", d @ ..] if !d.is_empty() => Some(d.concat()),
                _ => None,
            };
            params.push(Param { ty: ty.to_string(), name: name.to_string(), default });
        }
        current.clear();
    };
    for &w in words {
        match w {
            "[" | "(" => depth += 1,
            "]" | ")" => depth -= 1,
            "," if depth == 0 => {
                flush(&mut current);
                continue;
            }
            _ => {}
        }
        current.push(w);
    }
    flush(&mut current);
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
// Header, not attached

#define ENGINE_NUM_STRUCTURES 2
#define ENGINE_STRUCTURE_0 effect
#define ENGINE_STRUCTURE_1 location

int    TRUE                     = 1;
const float  DIRECTION_EAST     = 0.0;
string SOMETHING = \"a;b\";
int NEG = -1;

// Get an integer between 0 and nMaxInteger-1.
// Return value on error: 0
int Random(int nMaxInteger);

// Moves.
void ActionMoveToObject(object oMoveTo, int bRun=FALSE, float fRange=1.0f);
void Jump(vector vDest=[0.0,0.0,0.0], location lLoc=LOCATION_INVALID);
";

    /// A function's documentation as written too, for Help to show as
    /// `nwscript.nss` has it: the `//`, the spaces before it and inside
    /// it (aligned columns), no `\r` (the file is CRLF).
    #[test]
    fn documentation_is_kept_as_written() {
        let src = "// Sets a thing.\r\n// - nKind:   KIND_A\r\n//             KIND_B\r\n  // indented\r\nvoid Set(int nKind);\r\n";
        let s = Spec::parse(src.as_bytes());
        let f = s.function("Set").unwrap();
        assert_eq!(
            f.doc_source,
            "// Sets a thing.\n// - nKind:   KIND_A\n//             KIND_B\n  // indented"
        );
        assert_eq!(f.doc, "Sets a thing.\n- nKind:   KIND_A\nKIND_B\nindented");
        let r = Spec::parse(SAMPLE.as_bytes());
        assert_eq!(
            r.function("Random").unwrap().doc_source,
            "// Get an integer between 0 and nMaxInteger-1.\n// Return value on error: 0"
        );
        assert_eq!(r.function("Jump").unwrap().doc_source, "");
    }

    #[test]
    fn parses_constants_functions_and_structures() {
        let s = Spec::parse(SAMPLE.as_bytes());
        assert_eq!(s.engine_structures, ["effect", "location"]);
        let consts: Vec<(&str, &str, &str)> = s
            .constants
            .iter()
            .map(|c| (c.ty.as_str(), c.name.as_str(), c.value.as_str()))
            .collect();
        assert_eq!(
            consts,
            [
                ("int", "TRUE", "1"),
                ("float", "DIRECTION_EAST", "0.0"),
                ("string", "SOMETHING", "\"a;b\""),
                ("int", "NEG", "-1")
            ]
        );
        assert_eq!(s.functions.len(), 3);
        let r = s.function("Random").unwrap();
        assert_eq!(r.id, 0);
        assert_eq!(r.doc, "Get an integer between 0 and nMaxInteger-1.\nReturn value on error: 0");
        let m = s.function("ActionMoveToObject").unwrap();
        assert_eq!(
            m.signature(),
            "void ActionMoveToObject(object oMoveTo, int bRun=FALSE, float fRange=1.0f)"
        );
        let j = s.function("Jump").unwrap();
        assert_eq!(j.params[0].default.as_deref(), Some("[0.0,0.0,0.0]"));
        assert_eq!(j.params[1].ty, "location");
        assert_eq!(j.doc, "", "no comment directly above");
    }
}
