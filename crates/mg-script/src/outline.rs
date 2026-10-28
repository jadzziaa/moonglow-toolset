//! Top-level declarations of an NWScript source: includes, defines, structs,
//! global variables and constants, function prototypes and definitions (with
//! the comment block above each as documentation). This is what the script
//! editor needs for its outline, go-to-definition, completion and signature
//! help; bodies are skipped, not parsed. The parser tolerates broken code and
//! resynchronises at the next `;` or top-level `}`.

use std::ops::Range;

use crate::lex::{Token, TokenKind, includes_in, tokenize};
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
    /// In declaration order.
    pub members: Vec<Member>,
    pub span: Range<usize>,
}

/// A member of a struct: `string sLabel;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub ty: String,
    pub name: String,
    /// The name's.
    pub span: Range<usize>,
    /// The comments directly above, without their markers.
    pub doc: String,
    /// The same lines as written ([`comment_as_written`]).
    pub doc_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Global {
    pub ty: String,
    pub name: String,
    pub is_const: bool,
    pub value: Option<String>,
    pub span: Range<usize>,
    /// The comments directly above (`//` lines, `/* … */` blocks), without
    /// their markers.
    pub doc: String,
    /// The same lines as written ([`comment_as_written`]).
    pub doc_source: String,
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
    /// The comments directly above (`//` lines, `/* … */` blocks), without
    /// their markers.
    pub doc: String,
    /// The same lines as written ([`comment_as_written`]).
    pub doc_source: String,
}

/// A comment as written (`span`, a comment's token: a `//` line or a
/// `/* … */` block): with the spaces and tabs before it where nothing else
/// is before it, so that a block of them keeps its columns, and without the
/// line's end.
pub fn comment_as_written<'a>(text: &'a str, span: &Range<usize>) -> &'a str {
    let line_start = text[..span.start].rfind('\n').map_or(0, |i| i + 1);
    let before = &text[line_start..span.start];
    let from = if before.chars().all(|c| c == ' ' || c == '\t') { line_start } else { span.start };
    text[from..span.end].trim_end_matches(['\r', '\n'])
}

/// A comment as documentation: a line without its `//`, the spaces
/// around it and the `*`s it begins with (BioWare's `// * text`); a block
/// without its `/*` and `*/`, and each of its lines without the spaces
/// and the `*`s it begins with (` * ` down the side, as documentation
/// comments are written), its empty first and last lines dropped (`/*`
/// and `*/` on lines of their own, as BioWare's includes have them).
pub(crate) fn comment_text(written: &str) -> String {
    let written = written.trim();
    let Some(block) = written.strip_prefix("/*") else {
        let line = written.trim_start_matches('/').trim();
        return line.trim_start_matches('*').trim().to_string();
    };
    let block = block.strip_suffix("*/").unwrap_or(block);
    let lines: Vec<&str> = block.lines().map(|l| l.trim().trim_start_matches('*').trim()).collect();
    let first = lines.iter().position(|l| !l.is_empty()).unwrap_or(lines.len());
    let last = lines.iter().rposition(|l| !l.is_empty()).map_or(first, |i| i + 1);
    lines[first..last].join("\n")
}

/// The comments directly above token `raw` of `toks` (all of `text`'s,
/// comments and spaces too): `//` lines and `/* … */` blocks, with no
/// empty line between, and none that follows code on its line (`int a;
/// // a` is about `a`, not what comes next). Without their markers
/// ([`comment_text`]), and as written ([`comment_as_written`]).
pub(crate) fn doc_before(text: &str, toks: &[Token], raw: usize) -> (String, String) {
    let mut lines = Vec::new();
    let mut i = raw.min(toks.len());
    while i > 0 {
        i -= 1;
        let t = &toks[i];
        let s = &text[t.span.clone()];
        match t.kind {
            TokenKind::LineComment | TokenKind::BlockComment { .. } => {
                let line_start = text[..t.span.start].rfind('\n').map_or(0, |n| n + 1);
                let after_code = (toks[..i].iter().rev())
                    .take_while(|p| p.span.end > line_start)
                    .any(|p| p.kind.is_significant());
                if after_code {
                    break;
                }
                lines.push(comment_as_written(text, &t.span));
            }
            TokenKind::Whitespace if s.matches('\n').count() <= 1 => {}
            _ => break,
        }
    }
    lines.reverse();
    let doc: Vec<String> = lines.iter().map(|l| comment_text(l)).collect();
    (doc.join("\n"), lines.join("\n").replace('\r', ""))
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

    /// [`doc_before`] significant token `k`.
    fn doc_before(&self, k: usize) -> (String, String) {
        let Some(&raw) = self.sig.get(k) else { return Default::default() };
        doc_before(self.text, &self.toks, raw)
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

    fn parse_struct_body(&mut self) -> Vec<Member> {
        // At `{`.
        self.pos += 1;
        let mut members = Vec::new();
        while !self.at_end() && self.peek() != "}" {
            let (doc, doc_source) = self.doc_before(self.pos);
            let Some(ty) = self.parse_type() else {
                self.recover();
                continue;
            };
            loop {
                if self.kind(self.pos) == Some(TokenKind::Ident) {
                    members.push(Member {
                        ty: ty.clone(),
                        name: self.peek().to_string(),
                        span: self.tok(self.pos).map(|t| t.span.clone()).unwrap_or_default(),
                        doc: doc.clone(),
                        doc_source: doc_source.clone(),
                    });
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
                    // (The includes are `lex::includes`', [`outline`].)
                    self.line_rest();
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
            let (doc, doc_source) = self.doc_before(start_k);
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
                    doc_source,
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
                    doc_source: doc_source.clone(),
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

/// Whether a source defines an entry point of its own: `main` or
/// `StartingConditional` with a body (one in a comment or a string is
/// none). A script without one is an include file: Aurora doesn't compile
/// it on its own, where it may lean on what its includer brings.
///
/// Also where broken code before it keeps the outline from seeing it
/// (`main() {` among the tokens): such a script is no include, and its
/// error is the compiler's to report, as Aurora reports it.
pub fn has_entry_point(src: &[u8]) -> bool {
    let entry = |name: &str| matches!(name, "main" | "StartingConditional");
    if outline(src).functions.iter().any(|f| f.body.is_some() && entry(&f.name)) {
        return true;
    }
    let text = String::from_utf8_lossy(src);
    let toks: Vec<&str> = (tokenize(text.as_bytes()).into_iter())
        .filter(|t| t.kind.is_significant())
        .map(|t| &text[t.span])
        .collect();
    toks.windows(4).any(|w| entry(w[0]) && w[1] == "(" && w[2] == ")" && w[3] == "{")
}

/// Parses the top-level declarations of a source.
pub fn outline(src: &[u8]) -> Outline {
    let text = String::from_utf8_lossy(src);
    let toks = tokenize(text.as_bytes());
    // (Read as the other script tools read them.)
    let includes = (includes_in(text.as_bytes(), &toks).into_iter())
        .map(|at| Include {
            name: text[at.name.clone()].to_string(),
            span: at.directive.start..at.name.end + 1,
        })
        .collect();
    let sig =
        toks.iter().enumerate().filter(|(_, t)| t.kind.is_significant()).map(|(i, _)| i).collect();
    Outline { includes, ..Parser { text: &text, toks, sig, pos: 0 }.parse() }
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
        let members: Vec<(&str, &str)> =
            o.structs[0].members.iter().map(|m| (m.ty.as_str(), m.name.as_str())).collect();
        assert_eq!(
            members,
            [
                ("vector", "vPos"),
                ("string", "sLabel"),
                ("string", "sOther"),
                ("struct Inner", "inner")
            ]
        );
        assert_eq!(&SRC[o.structs[0].members[1].span.clone()], "sLabel");
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
        assert_eq!(o.functions[0].doc_source, "// Returns a greeting.\n// Two lines of doc.");
        assert_eq!(o.functions[0].params[0].default.as_deref(), Some("OBJECT_SELF"));
        assert_eq!(o.functions[2].params[0].default.as_deref(), Some("[0.0,1.0,2.0]"));
        assert_eq!(&SRC[o.functions[3].name_span.clone()], "main");
        assert!(SRC[o.functions[3].body.clone().unwrap()].contains("PrintString"));
        // As written: the spaces before a comment line, not its line's end
        // (CRLF). A comment after code on its line is about that code, not
        // what follows (Jasperre's report).
        let o = outline(b"int a; // A.\r\n    //   B, in.\r\nint F();\r\n");
        assert_eq!(o.functions[0].doc_source, "    //   B, in.");
        assert_eq!(o.functions[0].doc, "B, in.");
        let o = outline(b"int a; /* A. */ /* B. */\nint F();\n");
        assert_eq!(o.functions[0].doc, "");
        // BioWare's `// * text`: the `*` is not the text's.
        let o = outline(b"// * Returns TRUE.\n//***********\n// ** Twice.\nint F();\n");
        assert_eq!(o.functions[0].doc, "Returns TRUE.\n\nTwice.");
        assert_eq!(o.functions[0].doc_source, "// * Returns TRUE.\n//***********\n// ** Twice.");
        // A struct's members have theirs.
        let o =
            outline(b"struct S {\n    // The count.\n    int n; // after n\n    string s;\n};\n");
        let docs: Vec<&str> = o.structs[0].members.iter().map(|m| m.doc.as_str()).collect();
        assert_eq!(docs, ["The count.", ""]);
    }

    /// A `/* … */` comment directly above a declaration is its
    /// documentation, as `//` lines are (Jasperre's report): BioWare's
    /// includes put theirs between `//::` lines; others write ` * ` down
    /// the side. A section's banner with an empty line under it is not.
    #[test]
    fn block_comments_document_what_follows() {
        let src = "/**********\n * CONSTANTS\n **********/\n\nconst int A = 1;\n\
            //::////\n//:: Hidden\n/*\n    Returns TRUE if oSelf is hidden\n\n    magically\n*/\n\
            //:: Created By: Brent\nint Hidden(object oSelf);\n\
            /** Javadoc style.\n *  - nKind: the kind\n */\nvoid Set(int nKind);\n\
            /* One line. */ int One();\n";
        let o = outline(src.as_bytes());
        assert_eq!(o.globals[0].doc, "");
        let doc = |name: &str| {
            let f = o.functions.iter().find(|f| f.name == name).unwrap();
            (f.doc.as_str(), f.doc_source.as_str())
        };
        assert_eq!(
            doc("Hidden").0,
            "::////\n:: Hidden\nReturns TRUE if oSelf is hidden\n\nmagically\n:: Created By: Brent"
        );
        assert_eq!(
            doc("Hidden").1,
            "//::////\n//:: Hidden\n/*\n    Returns TRUE if oSelf is hidden\n\n    magically\n*/\n\
             //:: Created By: Brent"
        );
        assert_eq!(doc("Set").0, "Javadoc style.\n- nKind: the kind");
        assert_eq!(doc("Set").1, "/** Javadoc style.\n *  - nKind: the kind\n */");
        assert_eq!(doc("One"), ("One line.", "/* One line. */"));
        // An open block runs to the end: nothing after it to document.
        assert!(outline(b"/* open\nint F();").functions.is_empty());
        // CRLF: no `\r` in either.
        let o = outline(b"/*\r\n * Two\r\n */\r\nint F();\r\n");
        assert_eq!(
            (o.functions[0].doc.as_str(), o.functions[0].doc_source.as_str()),
            ("Two", "/*\n * Two\n */")
        );
    }

    #[test]
    fn recovers_from_garbage() {
        let o = outline(b"int x = ; ) } garbage here;\nvoid ok() {}\nfloat 3;\nint y;");
        assert_eq!(o.functions.len(), 1);
        assert_eq!(o.functions[0].name, "ok");
        assert!(o.globals.iter().any(|g| g.name == "y"));
    }

    /// An entry point is one with a body, not in a comment or a string;
    /// one after broken code still counts (a nested block comment leaves
    /// `still outer? */` as code: Aurora reports the script's error, so it
    /// is no include to pass over).
    #[test]
    fn entry_points() {
        assert!(has_entry_point(b"void main() {}"));
        assert!(has_entry_point(b"int StartingConditional()\n{ return 1; }"));
        assert!(!has_entry_point(b"void main();\nint Inc() { return 1; }"));
        assert!(!has_entry_point(b"/*\nvoid main() {}\n*/\nint NotMain() { return 1; }\n"));
        assert!(!has_entry_point(b"// void main() {}\nstring s = \"void main() {}\";"));
        assert!(has_entry_point(b"/* outer /* inner */ still outer? */\nvoid main() {}\n"));
    }
}
