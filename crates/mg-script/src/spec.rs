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
    /// The comments directly above the declaration (`//` lines, `/* … */`
    /// blocks), without their markers.
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

/// The most engine structures a spec declares (the game's has ten): a
/// `#define ENGINE_STRUCTURE_<n>` at or past it is skipped.
const MAX_ENGINE_STRUCTURES: usize = 256;

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
                TokenKind::LineComment | TokenKind::BlockComment { .. } => {
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
                    // The game has ten: a number far past them is no
                    // engine structure, and sizes nothing.
                    && n < MAX_ENGINE_STRUCTURES
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
            // A comment trailing the `;` on its line documents the statement
            // (and is not the next one's comment above).
            let trailing = crate::outline::trailing_comment(&text, &toks, i);
            i = trailing.unwrap_or(i) + 1; // the `;`, and that comment
            let words: Vec<&str> = stmt.iter().map(|t| s(t)).collect();
            // A comment block counts as documentation only directly above.
            let above = if blank_lines > 0 {
                Default::default()
            } else {
                let plain: Vec<String> =
                    doc.iter().map(|l| crate::outline::comment_text(l)).collect();
                (plain.join("\n"), doc.join("\n").replace('\r', ""))
            };
            let (docs, doc_source) =
                crate::outline::doc_with_trailing(&text, &toks, above, trailing);
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
    let mut depth: usize = 0;
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
            "]" | ")" => depth = depth.saturating_sub(1),
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

    /// A module's or hak's `nwscript.nss` with an engine structure of an
    /// absurd number: skipped, not a list of that size (or an index past
    /// one that wrapped to nothing).
    #[test]
    fn engine_structure_numbers_size_nothing() {
        let src = "#define ENGINE_STRUCTURE_0 effect\n\
#define ENGINE_STRUCTURE_18446744073709551615 x\n\
#define ENGINE_STRUCTURE_99999999999 y\n\
#define ENGINE_STRUCTURE_256 z\n\
#define ENGINE_STRUCTURE_3 talent\n";
        let s = Spec::parse(src.as_bytes());
        assert_eq!(s.engine_structures, ["effect", "", "", "talent"]);
    }

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
        // A `/* … */` block is documentation too; one with an empty line
        // under it is not.
        let src = "/* Banner */\r\n\r\nint A = 1;\r\n/*\r\n * Sets it.\r\n */\r\nvoid Set();\r\n";
        let s = Spec::parse(src.as_bytes());
        assert_eq!(s.constants[0].doc, "");
        let f = s.function("Set").unwrap();
        assert_eq!((f.doc.as_str(), f.doc_source.as_str()), ("Sets it.", "/*\n * Sets it.\n */"));
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

    fn doc_of<'a>(s: &'a Spec, name: &str) -> &'a str {
        &s.constant(name).unwrap().doc
    }

    /// A comment trailing a constant on its line documents it (the report:
    /// `SPELL_TYPE_*`), as written too, and is not the next one's comment.
    #[test]
    fn a_trailing_comment_documents_the_constant() {
        let src = "\
const int SPELL_TYPE_INVALID        = 0;  // Invalid setting! Error!!!\r
const int SPELL_TYPE_SPELL          = 1;  // Standard spell.\r
const int SPELL_TYPE_CREATURE_POWER = 2;  // Or \"Spell ability\". Uses HD for resist spell.\r
const int SPELL_TYPE_NONE           = 3;\r
";
        let s = Spec::parse(src.as_bytes());
        assert_eq!(doc_of(&s, "SPELL_TYPE_INVALID"), "Invalid setting! Error!!!");
        assert_eq!(doc_of(&s, "SPELL_TYPE_SPELL"), "Standard spell.");
        assert_eq!(
            doc_of(&s, "SPELL_TYPE_CREATURE_POWER"),
            "Or \"Spell ability\". Uses HD for resist spell."
        );
        assert_eq!(doc_of(&s, "SPELL_TYPE_NONE"), "", "the one above's comment is not its");
        assert_eq!(s.constant("SPELL_TYPE_SPELL").unwrap().doc_source, "// Standard spell.");
    }

    /// Above and trailing: both, the one above first. A `/* */` trails
    /// too; one with code after it on the line is no trailing comment.
    #[test]
    fn above_and_trailing_comments_both_show() {
        let src = "// Above.\nint A = 1; // Beside.\nint B = 2; /* Block. */\n\
int C = 3; /* not trailing */ int D = 4;\n";
        let s = Spec::parse(src.as_bytes());
        assert_eq!(doc_of(&s, "A"), "Above.\nBeside.");
        assert_eq!(s.constant("A").unwrap().doc_source, "// Above.\n// Beside.");
        assert_eq!(doc_of(&s, "B"), "Block.");
        assert_eq!(doc_of(&s, "C"), "");
        assert_eq!(doc_of(&s, "D"), "not trailing", "directly above D, as it was");
    }

    /// An empty line ends a comment's reach on both sides: a constant after
    /// a trailing comment and an empty line has nothing; a heading for a
    /// group, an empty line under it, is no constant's doc (and the
    /// trailing comments stay each constant's own).
    #[test]
    fn a_heading_over_a_group_is_not_each_constants_doc() {
        let src = "// Heading for the group.\n\nint A = 1; // a\nint B = 2;\n// Own.\nint C = 3;\n\
int D = 4; // d\n\nint E = 5;\n";
        let s = Spec::parse(src.as_bytes());
        assert_eq!(doc_of(&s, "A"), "a");
        assert_eq!(doc_of(&s, "B"), "");
        assert_eq!(doc_of(&s, "C"), "Own.");
        assert_eq!(doc_of(&s, "D"), "d");
        assert_eq!(doc_of(&s, "E"), "");
        // Without the empty line the heading is the first constant's
        // comment above, as it was: shown before its trailing comment.
        let s = Spec::parse(b"// Group.\nint A = 1; // a\nint B = 2; // b\n");
        assert_eq!(doc_of(&s, "A"), "Group.\na");
        assert_eq!(doc_of(&s, "B"), "b");
    }

    /// A prototype's trailing comment documents it too.
    #[test]
    fn a_trailing_comment_documents_a_prototype() {
        let s = Spec::parse(b"int Get(int n); // Gets it.\nint Other();\n");
        assert_eq!(s.function("Get").unwrap().doc, "Gets it.");
        assert_eq!(s.function("Other").unwrap().doc, "");
    }

    /// `nwscript.nss` of the install, if there is one: the constants with
    /// a trailing comment all have a documentation now. (`mg-script` is
    /// below the test kit: the install is found as it does, `NWN_ROOT` or
    /// the Steam default, and the test skips without one.)
    #[test]
    fn the_games_trailing_comments_document_their_constants() {
        let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
        let roots = std::env::var_os("NWN_ROOT")
            .map(std::path::PathBuf::from)
            .into_iter()
            .chain(home.map(|h| h.join(".local/share/Steam/steamapps/common/Neverwinter Nights")));
        let Some(src) = roots.filter_map(|r| std::fs::read(r.join("ovr/nwscript.nss")).ok()).next()
        else {
            eprintln!("skipped: no game install with an ovr/nwscript.nss");
            return;
        };
        let s = Spec::parse(&src);
        let text = String::from_utf8_lossy(&src);
        let with_trailing: Vec<&str> = text
            .lines()
            .filter_map(|l| {
                let (code, comment) = l.split_once("//")?;
                let code = code.trim_end().strip_suffix(';')?;
                let name = code.split('=').next()?.split_whitespace().last()?;
                (!comment.trim().is_empty() && code.contains('=')).then_some(name)
            })
            .collect();
        let documented = s.constants.iter().filter(|c| !c.doc.is_empty()).count();
        eprintln!(
            "{} constants, {documented} with a doc, {} with a trailing comment",
            s.constants.len(),
            with_trailing.len()
        );
        assert!(with_trailing.len() >= 18, "{}", with_trailing.len());
        for name in &with_trailing {
            let c = s.constant(name).unwrap();
            assert!(!c.doc.is_empty(), "{name} has no doc");
        }
        assert_eq!(
            s.constant("ABILITY_STRENGTH").unwrap().doc,
            "should be the same as in nwseffectlist.cpp"
        );
    }

    #[test]
    fn parse_params_does_not_go_negative_on_stray_closing_brackets() {
        let p = parse_params(&["int", "a", "=", ")", ",", "float", "b"]);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].name, "a");
        assert_eq!(p[1].name, "b");
    }
}
