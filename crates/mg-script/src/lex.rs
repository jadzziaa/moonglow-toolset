//! NWScript lexer for the editor: every byte of the source lands in exactly
//! one token (whitespace and comments included), so highlighting and
//! navigation work on any text, valid or not. Token rules follow the official
//! compiler's lexer (`nwscript/native/scriptcomplexical.cpp`).

use std::ops::Range;

/// What a token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Whitespace,
    LineComment,
    /// `/* ... */`; `terminated` is false when the file ends inside it.
    BlockComment {
        terminated: bool,
    },
    Ident,
    /// A language keyword (`if`, `int`, `struct`, ...).
    Keyword,
    /// A built-in constant keyword (`OBJECT_SELF`, `JSON_NULL`, `__LINE__`, ...).
    BuiltinConstant,
    /// `#include` or `#define`.
    Directive,
    /// Decimal, `0x`, `0b` or `0o` integer.
    Int,
    /// `1.5`, `.5`, `2f`, `1.5f`.
    Float,
    /// `"..."`; unterminated at a newline or the end of the file.
    String {
        terminated: bool,
    },
    /// `r"..."` (may span lines; `""` is a quote).
    RawString {
        terminated: bool,
    },
    /// `h"..."`: compiled to the string's hash.
    HashedString {
        terminated: bool,
    },
    Punct,
    /// A character the language does not use.
    Unknown,
}

/// A token: its kind and byte range in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Range<usize>,
}

impl TokenKind {
    /// Whether the token carries meaning for the parser (not whitespace or a
    /// comment).
    pub fn is_significant(self) -> bool {
        !matches!(
            self,
            TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
        )
    }
}

pub const KEYWORDS: &[&str] = &[
    "action", "break", "case", "const", "continue", "default", "do", "else", "float", "for", "if",
    "int", "object", "return", "string", "struct", "switch", "vector", "void", "while",
];

pub const BUILTIN_CONSTANTS: &[&str] = &[
    "OBJECT_SELF",
    "OBJECT_INVALID",
    "LOCATION_INVALID",
    "JSON_ARRAY",
    "JSON_FALSE",
    "JSON_NULL",
    "JSON_OBJECT",
    "JSON_STRING",
    "JSON_TRUE",
    "__FILE__",
    "__FUNCTION__",
    "__LINE__",
    "__DATE__",
    "__TIME__",
];

/// Multi-character operators, longest first.
const OPERATORS: &[&str] = &[
    ">>>=", ">>>", "<<=", ">>=", "==", "!=", "<=", ">=", "&&", "||", "++", "--", "+=", "-=", "*=",
    "/=", "%=", "&=", "|=", "^=", "<<", ">>",
];

const SINGLE_PUNCT: &[u8] = b"+-*/%=<>!~&|^?:;,.(){}[]";

/// Splits `src` into tokens covering it completely.
pub fn tokenize(src: &[u8]) -> Vec<Token> {
    let mut out = Vec::new();
    let mut i = 0;
    let n = src.len();
    let is_ident_start = |c: u8| c.is_ascii_alphabetic() || c == b'_';
    let is_ident = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    while i < n {
        let start = i;
        let c = src[i];
        let next = src.get(i + 1).copied();
        let kind = if c.is_ascii_whitespace() {
            while i < n && src[i].is_ascii_whitespace() {
                i += 1;
            }
            TokenKind::Whitespace
        } else if c == b'/' && next == Some(b'/') {
            while i < n && src[i] != b'\n' {
                i += 1;
            }
            TokenKind::LineComment
        } else if c == b'/' && next == Some(b'*') {
            i += 2;
            let mut terminated = false;
            while i < n {
                if src[i] == b'*' && src.get(i + 1) == Some(&b'/') {
                    i += 2;
                    terminated = true;
                    break;
                }
                i += 1;
            }
            TokenKind::BlockComment { terminated }
        } else if matches!(c, b'r' | b'R') && next == Some(b'"') {
            i += 2;
            let mut terminated = false;
            while i < n {
                if src[i] == b'"' {
                    if src.get(i + 1) == Some(&b'"') {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    terminated = true;
                    break;
                }
                i += 1;
            }
            TokenKind::RawString { terminated }
        } else if c == b'"' || (matches!(c, b'h' | b'H') && next == Some(b'"')) {
            let hashed = c != b'"';
            i += if hashed { 2 } else { 1 };
            let mut terminated = false;
            while i < n {
                match src[i] {
                    b'\\' if i + 1 < n && src[i + 1] != b'\n' => i += 2,
                    b'"' => {
                        i += 1;
                        terminated = true;
                        break;
                    }
                    b'\n' => break,
                    _ => i += 1,
                }
            }
            if hashed {
                TokenKind::HashedString { terminated }
            } else {
                TokenKind::String { terminated }
            }
        } else if c.is_ascii_digit() || (c == b'.' && next.is_some_and(|d| d.is_ascii_digit())) {
            let radix = if c == b'0' { next.map(|x| x.to_ascii_lowercase()) } else { None };
            match radix {
                Some(b'x') => {
                    i += 2;
                    while i < n && src[i].is_ascii_hexdigit() {
                        i += 1;
                    }
                    TokenKind::Int
                }
                Some(b'b') => {
                    i += 2;
                    while i < n && matches!(src[i], b'0' | b'1') {
                        i += 1;
                    }
                    TokenKind::Int
                }
                Some(b'o') => {
                    i += 2;
                    while i < n && (b'0'..=b'7').contains(&src[i]) {
                        i += 1;
                    }
                    TokenKind::Int
                }
                _ => {
                    let mut float = false;
                    while i < n && src[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i < n && src[i] == b'.' {
                        float = true;
                        i += 1;
                        while i < n && src[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    if i < n && src[i] == b'f' {
                        float = true;
                        i += 1;
                    }
                    if float { TokenKind::Float } else { TokenKind::Int }
                }
            }
        } else if is_ident_start(c) {
            while i < n && is_ident(src[i]) {
                i += 1;
            }
            let word = &src[start..i];
            if KEYWORDS.iter().any(|k| k.as_bytes() == word) {
                TokenKind::Keyword
            } else if BUILTIN_CONSTANTS.iter().any(|k| k.as_bytes() == word) {
                TokenKind::BuiltinConstant
            } else {
                TokenKind::Ident
            }
        } else if c == b'#' && next.is_some_and(is_ident_start) {
            i += 1;
            while i < n && is_ident(src[i]) {
                i += 1;
            }
            TokenKind::Directive
        } else if let Some(op) = OPERATORS.iter().find(|op| src[i..].starts_with(op.as_bytes())) {
            i += op.len();
            TokenKind::Punct
        } else if SINGLE_PUNCT.contains(&c) {
            i += 1;
            TokenKind::Punct
        } else {
            // One UTF-8 character or one byte of anything else.
            i += utf8_len(c).min(n - i).max(1);
            TokenKind::Unknown
        };
        out.push(Token { kind, span: start..i });
    }
    out
}

fn utf8_len(first: u8) -> usize {
    match first {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(TokenKind, &str)> {
        tokenize(src.as_bytes())
            .into_iter()
            .filter(|t| t.kind != TokenKind::Whitespace)
            .map(|t| (t.kind, &src[t.span]))
            .collect()
    }

    #[test]
    fn covers_the_source() {
        let src = "#include \"x\"\nvoid main() { int i = 0x1F + .5f; } // c\n/* b */ ";
        let toks = tokenize(src.as_bytes());
        let joined: String = toks.iter().map(|t| &src[t.span.clone()]).collect();
        assert_eq!(joined, src);
    }

    #[test]
    fn literals_like_the_compiler() {
        use TokenKind::*;
        assert_eq!(
            kinds(r#"0x1F 0b101 0o17 12 1.5 .5 2f 1.5f"#),
            [
                (Int, "0x1F"),
                (Int, "0b101"),
                (Int, "0o17"),
                (Int, "12"),
                (Float, "1.5"),
                (Float, ".5"),
                (Float, "2f"),
                (Float, "1.5f")
            ]
        );
        assert_eq!(
            kinds(r#""a\"b\\" r"x""y" h"key" "open"#),
            [
                (String { terminated: true }, r#""a\"b\\""#),
                (RawString { terminated: true }, r#"r"x""y""#),
                (HashedString { terminated: true }, r#"h"key""#),
                (String { terminated: false }, r#""open"#),
            ]
        );
        assert_eq!(kinds("r\"a\nb\"")[0], (RawString { terminated: true }, "r\"a\nb\""));
        assert_eq!(kinds("\"a\nb")[0], (String { terminated: false }, "\"a"));
    }

    #[test]
    fn words_and_operators() {
        use TokenKind::*;
        assert_eq!(
            kinds("struct S; OBJECT_SELF #define x >>>= a<=b é"),
            [
                (Keyword, "struct"),
                (Ident, "S"),
                (Punct, ";"),
                (BuiltinConstant, "OBJECT_SELF"),
                (Directive, "#define"),
                (Ident, "x"),
                (Punct, ">>>="),
                (Ident, "a"),
                (Punct, "<="),
                (Ident, "b"),
                (Unknown, "é"),
            ]
        );
        assert_eq!(kinds("/* open")[0].0, BlockComment { terminated: false });
    }
}
