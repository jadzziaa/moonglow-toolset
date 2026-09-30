//! Top-level declarations of an NWScript source: includes, defines, structs,
//! global variables and constants, function prototypes and definitions (with
//! the comment block above each as documentation). This is what the script
//! editor needs for its outline, go-to-definition, completion and signature
//! help; bodies are skipped, not parsed. The parser tolerates broken code and
//! resynchronises at the next `;` or top-level `}`.

use std::ops::Range;

use crate::lex::{Token, TokenKind, tokenize};
use crate::spec::Param;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Include {
    pub name: String,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Define {
    pub name: String,
    pub value: String,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDecl {
    pub name: String,
    /// `(type, name)` in declaration order.
    pub members: Vec<(String, String)>,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub ty: String,
    pub name: String,
    pub is_const: bool,
    pub value: Option<String>,
    pub span: Range<usize>,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDecl {
    pub return_type: String,
    pub name: String,
    pub params: Vec<Param>,
    /// The whole declaration or definition.
    pub span: Range<usize>,
    pub name_span: Range<usize>,
    /// The body `{ ... }` of a definition; `None` for a prototype.
    pub body: Option<Range<usize>>,
    pub doc: String,
}

/// A source's top-level declarations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outline {
    pub includes: Vec<Include>,
    pub defines: Vec<Define>,
    pub structs: Vec<StructDecl>,
    pub globals: Vec<Global>,
    pub functions: Vec<FunctionDecl>,
}

struct Parser<'a> {
    text: &'a str,
    toks: Vec<Token>,
    /// Indices of significant tokens.
    sig: Vec<usize>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn tok(&self, k: usize) -> Option<&Token> {
        self.sig.get(k).map(|&i| &self.toks[i])
    }

    fn text_at(&self, k: usize) -> &'a str {
        self.tok(k).map_or("", |t| &self.text[t.span.clone()])
    }

    fn peek(&self) -> &'a str {
        self.text_at(self.pos)
    }

    fn kind(&self, k: usize) -> Option<TokenKind> {
        self.tok(k).map(|t| t.kind)
    }

    fn span_start(&self, k: usize) -> usize {
        self.tok(k).map_or(self.text.len(), |t| t.span.start)
    }

    fn span_end(&self, k: usize) -> usize {
        self.tok(k.min(self.sig.len().saturating_sub(1))).map_or(self.text.len(), |t| t.span.end)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.sig.len()
    }

    /// The `//` comment lines directly above significant token `k`.
    fn doc_before(&self, k: usize) -> String {
        let Some(&raw) = self.sig.get(k) else { return String::new() };
        let mut lines = Vec::new();
        let mut i = raw;
        while i > 0 {
            i -= 1;
            let t = &self.toks[i];
            let s = &self.text[t.span.clone()];
            match t.kind {
                TokenKind::LineComment => lines.push(s.trim_start_matches('/').trim().to_string()),
                TokenKind::Whitespace if s.matches('\n').count() <= 1 => {}
                _ => break,
            }
        }
        lines.reverse();
        lines.join("\n")
    }

    /// Skips to just after the next `;` at bracket depth 0, or to (and
    /// including) a closing `}` that ends the current top-level construct.
    fn recover(&mut self) {
        let mut depth = 0i32;
        while !self.at_end() {
            let s = self.peek();
            self.pos += 1;
            match s {
                "{" | "(" | "[" => depth += 1,
                "}" | ")" | "]" => {
                    depth -= 1;
                    if depth <= 0 && s == "}" {
                        if self.peek() == ";" {
                            self.pos += 1;
                        }
                        return;
                    }
                }
                ";" if depth <= 0 => return,
                _ => {}
            }
        }
    }

    /// Skips a balanced `{ ... }` starting at `pos`; returns its span.
    fn skip_block(&mut self) -> Range<usize> {
        let start = self.span_start(self.pos);
        let mut depth = 0i32;
        while !self.at_end() {
            let s = self.peek();
            self.pos += 1;
            if s == "{" {
                depth += 1;
            } else if s == "}" {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
        }
        start..self.span_end(self.pos - 1)
    }

    /// A type at `pos` (`int`, `effect`, `struct Name`), consumed.
    fn parse_type(&mut self) -> Option<String> {
        match (self.kind(self.pos)?, self.peek()) {
            (TokenKind::Keyword, "struct") => {
                if self.kind(self.pos + 1) != Some(TokenKind::Ident) {
                    return None;
                }
                let name = self.text_at(self.pos + 1);
                self.pos += 2;
                Some(format!("struct {name}"))
            }
            (TokenKind::Keyword, t)
                if matches!(
                    t,
                    "int" | "float" | "string" | "object" | "void" | "vector" | "action"
                ) =>
            {
                self.pos += 1;
                Some(t.to_string())
            }
            (TokenKind::Ident, t) => {
                self.pos += 1;
                Some(t.to_string())
            }
            _ => None,
        }
    }

    /// Tokens up to (not including) the next `;` or `,` at depth 0, joined.
    fn expression(&mut self) -> String {
        let mut depth = 0i32;
        let mut parts = Vec::new();
        while !self.at_end() {
            let s = self.peek();
            match s {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                ";" | "," if depth <= 0 => break,
                _ => {}
            }
            if depth < 0 {
                break;
            }
            parts.push(s);
            self.pos += 1;
        }
        parts.join(" ")
    }

    fn parse_params(&mut self) -> Vec<Param> {
        // At `(`.
        self.pos += 1;
        let mut params = Vec::new();
        while !self.at_end() && self.peek() != ")" {
            if self.peek() == "const" {
                self.pos += 1;
            }
            let Some(ty) = self.parse_type() else {
                // Not a parameter list we understand: skip to `)`.
                while !self.at_end() && self.peek() != ")" {
                    self.pos += 1;
                }
                break;
            };
            let name = if self.kind(self.pos) == Some(TokenKind::Ident) {
                let n = self.peek().to_string();
                self.pos += 1;
                n
            } else {
                String::new()
            };
            let default = if self.peek() == "=" {
                self.pos += 1;
                Some(self.expression().replace(' ', ""))
            } else {
                None
            };
            params.push(Param { ty, name, default });
            if self.peek() == "," {
                self.pos += 1;
            } else if self.peek() != ")" {
                while !self.at_end() && self.peek() != ")" {
                    self.pos += 1;
                }
            }
        }
        if self.peek() == ")" {
            self.pos += 1;
        }
        params
    }

    fn parse_struct_body(&mut self) -> Vec<(String, String)> {
        // At `{`.
        self.pos += 1;
        let mut members = Vec::new();
        while !self.at_end() && self.peek() != "}" {
            let Some(ty) = self.parse_type() else {
                self.recover();
                continue;
            };
            loop {
                if self.kind(self.pos) == Some(TokenKind::Ident) {
                    members.push((ty.clone(), self.peek().to_string()));
                    self.pos += 1;
                }
                if self.peek() == "," {
                    self.pos += 1;
                    continue;
                }
                break;
            }
            if self.peek() == ";" {
                self.pos += 1;
            } else if self.peek() != "}" {
                self.recover();
            }
        }
        if self.peek() == "}" {
            self.pos += 1;
        }
        if self.peek() == ";" {
            self.pos += 1;
        }
        members
    }

    fn line_rest(&mut self) -> (Vec<&'a str>, usize) {
        // The significant tokens after `pos` on the same line.
        let mut words = Vec::new();
        let mut end = self.span_end(self.pos);
        self.pos += 1;
        while !self.at_end() {
            let prev_end = self.span_end(self.pos - 1);
            let start = self.span_start(self.pos);
            if self.text[prev_end..start].contains('\n') {
                break;
            }
            words.push(self.peek());
            end = self.span_end(self.pos);
            self.pos += 1;
        }
        (words, end)
    }

    fn parse(mut self) -> Outline {
        let mut out = Outline::default();
        while !self.at_end() {
            let start_k = self.pos;
            let start = self.span_start(start_k);
            let word = self.peek();
            match (self.kind(start_k), word) {
                (Some(TokenKind::Directive), "#include") => {
                    let (words, end) = self.line_rest();
                    if let Some(name) = words
                        .first()
                        .and_then(|w| w.strip_prefix('"'))
                        .map(|w| w.trim_end_matches('"'))
                    {
                        out.includes.push(Include { name: name.to_string(), span: start..end });
                    }
                    continue;
                }
                (Some(TokenKind::Directive), "#define") => {
                    let (words, end) = self.line_rest();
                    if let Some((name, value)) = words.split_first() {
                        out.defines.push(Define {
                            name: name.to_string(),
                            value: value.join(" "),
                            span: start..end,
                        });
                    }
                    continue;
                }
                (Some(TokenKind::Directive), _) => {
                    self.line_rest();
                    continue;
                }
                (Some(TokenKind::Keyword), "struct")
                    if self.kind(start_k + 1) == Some(TokenKind::Ident)
                        && self.text_at(start_k + 2) == "{" =>
                {
                    let name = self.text_at(start_k + 1).to_string();
                    self.pos += 2;
                    let members = self.parse_struct_body();
                    out.structs.push(StructDecl {
                        name,
                        members,
                        span: start..self.span_end(self.pos - 1),
                    });
                    continue;
                }
                _ => {}
            }
            let is_const = word == "const";
            if is_const {
                self.pos += 1;
            }
            let doc = self.doc_before(start_k);
            let Some(ty) = self.parse_type() else {
                self.recover();
                continue;
            };
            if self.kind(self.pos) != Some(TokenKind::Ident) {
                self.recover();
                continue;
            }
            let name_k = self.pos;
            let name = self.peek().to_string();
            self.pos += 1;
            if self.peek() == "(" {
                let params = self.parse_params();
                let body = match self.peek() {
                    "{" => Some(self.skip_block()),
                    ";" => {
                        self.pos += 1;
                        None
                    }
                    _ => {
                        self.recover();
                        None
                    }
                };
                let name_span = self.tok(name_k).map(|t| t.span.clone()).unwrap_or_default();
                out.functions.push(FunctionDecl {
                    return_type: ty,
                    name,
                    params,
                    span: start..self.span_end(self.pos - 1),
                    name_span,
                    body,
                    doc,
                });
                continue;
            }
            // Globals: `type a [= x], b [= y];`
            let mut name = Some(name);
            while let Some(n) = name.take() {
                let value = if self.peek() == "=" {
                    self.pos += 1;
                    Some(self.expression())
                } else {
                    None
                };
                out.globals.push(Global {
                    ty: ty.clone(),
                    name: n,
                    is_const,
                    value,
                    span: start..self.span_end(self.pos.saturating_sub(1)),
                    doc: doc.clone(),
                });
                if self.peek() == "," && self.kind(self.pos + 1) == Some(TokenKind::Ident) {
                    name = Some(self.text_at(self.pos + 1).to_string());
                    self.pos += 2;
                }
            }
            if self.peek() == ";" {
                self.pos += 1;
            } else {
                self.recover();
            }
        }
        out
    }
}

/// Parses the top-level declarations of a source.
pub fn outline(src: &[u8]) -> Outline {
    let text = String::from_utf8_lossy(src);
    let toks = tokenize(text.as_bytes());
    let sig =
        toks.iter().enumerate().filter(|(_, t)| t.kind.is_significant()).map(|(i, _)| i).collect();
    Parser { text: &text, toks, sig, pos: 0 }.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
#include "nw_i0_generic"
#define MAX_THINGS 10

// A position with a label.
struct Mark {
    vector vPos;
    string sLabel, sOther;
    struct Inner inner;
};

const int COUNT = 3;
int gA, gB = 2;
effect eGlobal;

// Returns a greeting.
// Two lines of doc.
string Greet(object oTarget = OBJECT_SELF, int bLoud = FALSE);

string Greet(object oTarget = OBJECT_SELF, int bLoud = FALSE)
{
    if (bLoud) { return "HI"; }
    return "hi";
}

struct Mark MakeMark(vector v = [0.0, 1.0, 2.0]) { struct Mark m; return m; }

void main()
{
    int broken = ;
    PrintString(Greet());
}
"#;

    #[test]
    fn outline_of_a_script() {
        let o = outline(SRC.as_bytes());
        assert_eq!(
            o.includes.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["nw_i0_generic"]
        );
        assert_eq!(o.defines[0].name, "MAX_THINGS");
        assert_eq!(o.defines[0].value, "10");
        assert_eq!(o.structs.len(), 1);
        assert_eq!(
            o.structs[0].members,
            [
                ("vector".into(), "vPos".into()),
                ("string".into(), "sLabel".into()),
                ("string".into(), "sOther".into()),
                ("struct Inner".into(), "inner".into())
            ]
        );
        let globals: Vec<(&str, &str, bool, Option<&str>)> = o
            .globals
            .iter()
            .map(|g| (g.ty.as_str(), g.name.as_str(), g.is_const, g.value.as_deref()))
            .collect();
        assert_eq!(
            globals,
            [
                ("int", "COUNT", true, Some("3")),
                ("int", "gA", false, None),
                ("int", "gB", false, Some("2")),
                ("effect", "eGlobal", false, None)
            ]
        );
        let fns: Vec<(&str, &str, usize, bool)> = o
            .functions
            .iter()
            .map(|f| (f.return_type.as_str(), f.name.as_str(), f.params.len(), f.body.is_some()))
            .collect();
        assert_eq!(
            fns,
            [
                ("string", "Greet", 2, false),
                ("string", "Greet", 2, true),
                ("struct Mark", "MakeMark", 1, true),
                ("void", "main", 0, true)
            ]
        );
        assert_eq!(o.functions[0].doc, "Returns a greeting.\nTwo lines of doc.");
        assert_eq!(o.functions[0].params[0].default.as_deref(), Some("OBJECT_SELF"));
        assert_eq!(o.functions[2].params[0].default.as_deref(), Some("[0.0,1.0,2.0]"));
        assert_eq!(&SRC[o.functions[3].name_span.clone()], "main");
        assert!(SRC[o.functions[3].body.clone().unwrap()].contains("PrintString"));
    }

    #[test]
    fn recovers_from_garbage() {
        let o = outline(b"int x = ; ) } garbage here;\nvoid ok() {}\nfloat 3;\nint y;");
        assert_eq!(o.functions.len(), 1);
        assert_eq!(o.functions[0].name, "ok");
        assert!(o.globals.iter().any(|g| g.name == "y"));
    }
}
