//! What names in NWScript sources refer to: a local variable, a parameter,
//! or a top-level declaration (function, global, constant, struct) of the
//! script, of its includes (in the compiler's order) or of `nwscript.nss`,
//! which every script implicitly includes. On that rest go to definition,
//! find references and rename, for the script editor and `mg lsp`.
//!
//! Bodies aren't parsed: locals are found by their shape (at the start of a
//! statement, a type, a name, then `=`, `;`, `,` or `[`), scoped to the
//! block they're declared in. Struct members aren't resolved.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

use crate::lex::{Token, TokenKind, tokenize};
use crate::outline::{Outline, outline};

/// The engine's implicit include.
pub const NWSCRIPT: &str = "nwscript";

/// Loads a script's source by name (lowercase, without `.nss`).
pub trait Sources {
    fn source(&mut self, name: &str) -> Option<Arc<str>>;
}

impl<F: FnMut(&str) -> Option<Arc<str>>> Sources for F {
    fn source(&mut self, name: &str) -> Option<Arc<str>> {
        self(name)
    }
}

/// A place in a script: its name and a byte range.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    pub file: String,
    pub span: Range<usize>,
}

/// What a name declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeclKind {
    Function,
    Variable,
    Constant,
    Struct,
    Local,
    Parameter,
}

/// A declaration: where its name is, how it reads, its documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub name: String,
    pub kind: DeclKind,
    /// The name in the declaration (a function's definition when it has
    /// one, else its prototype).
    pub at: Location,
    pub signature: String,
    pub doc: String,
}

/// A local variable or parameter.
#[derive(Debug, Clone)]
struct Local {
    name: String,
    ty: String,
    kind: DeclKind,
    /// The name's span.
    decl: Range<usize>,
    /// Where the name can be used.
    scope: Range<usize>,
}

/// A source, indexed.
#[derive(Debug)]
struct File {
    text: Arc<str>,
    outline: Outline,
    /// Identifiers, except member names after `.`.
    idents: Vec<Range<usize>>,
    locals: Vec<Local>,
}

const TYPE_KEYWORDS: &[&str] = &["int", "float", "string", "object", "vector", "void", "action"];

fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && c.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl File {
    fn new(text: Arc<str>) -> File {
        let toks = tokenize(text.as_bytes());
        // Characters the language doesn't use are skipped, as the compiler
        // skips them (BioWare's x2_inc_banter.nss has a stray backtick).
        let sig: Vec<&Token> = toks
            .iter()
            .filter(|t| t.kind.is_significant() && t.kind != TokenKind::Unknown)
            .collect();
        let s = |i: usize| sig.get(i).map_or("", |t| &text[t.span.clone()]);
        let idents = sig
            .iter()
            .enumerate()
            .filter(|(i, t)| t.kind == TokenKind::Ident && (*i == 0 || s(i - 1) != "."))
            .map(|(_, t)| t.span.clone())
            .collect();
        let outline = outline(text.as_bytes());
        let mut locals = Vec::new();
        for f in &outline.functions {
            let Some(body) = f.body.clone() else { continue };
            // Parameters: in the declaration before the body, each a name
            // after a type.
            let head: Vec<&Token> = sig
                .iter()
                .copied()
                .filter(|t| t.span.start >= f.name_span.end && t.span.end <= body.start)
                .collect();
            for (k, t) in head.iter().enumerate() {
                if t.kind != TokenKind::Ident || k == 0 {
                    continue;
                }
                let prev = &text[head[k - 1].span.clone()];
                let next = head.get(k + 1).map_or("", |n| &text[n.span.clone()]);
                let typed = TYPE_KEYWORDS.contains(&prev) || head[k - 1].kind == TokenKind::Ident;
                if typed && matches!(next, "," | ")" | "=") {
                    locals.push(Local {
                        name: text[t.span.clone()].to_string(),
                        ty: prev.to_string(),
                        kind: DeclKind::Parameter,
                        decl: t.span.clone(),
                        scope: body.clone(),
                    });
                }
            }
            locals.extend(body_locals(&text, &sig, body));
        }
        File { text, outline, idents, locals }
    }

    fn word(&self, span: &Range<usize>) -> &str {
        &self.text[span.clone()]
    }

    /// The identifier at (or just before) a byte offset.
    fn ident_at(&self, offset: usize) -> Option<Range<usize>> {
        self.idents.iter().find(|r| r.start <= offset && offset <= r.end).cloned()
    }

    /// The local or parameter a name at `at` refers to.
    fn local(&self, name: &str, at: usize) -> Option<&Local> {
        self.locals
            .iter()
            .filter(|l| l.name == name && l.decl.start <= at && l.scope.contains(&at))
            .max_by_key(|l| l.decl.start)
    }

    /// A name's span inside a declaration's span (after its type).
    fn name_in(&self, span: &Range<usize>, name: &str) -> Range<usize> {
        self.idents
            .iter()
            .filter(|r| r.start >= span.start && r.end <= span.end)
            .find(|r| self.word(r) == name)
            .cloned()
            .unwrap_or(span.clone())
    }
}

/// Locals declared in a function body, each scoped to its block.
fn body_locals(text: &str, sig: &[&Token], body: Range<usize>) -> Vec<Local> {
    let toks: Vec<&Token> = sig
        .iter()
        .copied()
        .filter(|t| t.span.start >= body.start && t.span.end <= body.end)
        .collect();
    let s = |i: usize| toks.get(i).map_or("", |t| &text[t.span.clone()]);
    // Each `{`'s matching `}` end.
    let mut block_end = HashMap::new();
    let mut stack = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        match s(i) {
            "{" => stack.push(i),
            "}" => {
                if let Some(open) = stack.pop() {
                    block_end.insert(open, t.span.end);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    let mut blocks: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        let w = s(i);
        if w == "{" {
            blocks.push(i);
        } else if w == "}" {
            blocks.pop();
        }
        // (A `case …:` or `default:` label starts one too.)
        let statement_start = i == 0 || matches!(s(i - 1), "{" | "}" | ";" | ":");
        if !statement_start {
            i += 1;
            continue;
        }
        let mut k = i;
        if s(k) == "const" {
            k += 1;
        }
        let (ty, name_at) =
            if s(k) == "struct" && toks.get(k + 1).is_some_and(|t| t.kind == TokenKind::Ident) {
                (format!("struct {}", s(k + 1)), k + 2)
            } else if TYPE_KEYWORDS.contains(&s(k))
                || toks.get(k).is_some_and(|t| t.kind == TokenKind::Ident)
            {
                (s(k).to_string(), k + 1)
            } else {
                i += 1;
                continue;
            };
        let scope_end = blocks.last().and_then(|b| block_end.get(b)).copied().unwrap_or(body.end);
        // `T a = ..., b;`: names at bracket depth 0 up to the `;`.
        let mut j = name_at;
        let mut first = true;
        while let Some(t) = toks.get(j) {
            if t.kind != TokenKind::Ident || !matches!(s(j + 1), "=" | ";" | "," | "[") {
                break;
            }
            if first && ty == "void" {
                break;
            }
            out.push(Local {
                name: s(j).to_string(),
                ty: ty.clone(),
                kind: DeclKind::Local,
                decl: t.span.clone(),
                scope: t.span.start..scope_end,
            });
            first = false;
            // To the next `,` at depth 0, or the end of the statement.
            let mut depth = 0;
            j += 1;
            while toks.get(j).is_some() {
                match s(j) {
                    "(" | "[" | "{" => depth += 1,
                    ")" | "]" | "}" => depth -= 1,
                    "," if depth == 0 => {
                        j += 1;
                        break;
                    }
                    ";" if depth == 0 => break,
                    _ => {}
                }
                j += 1;
            }
            if s(j) == ";" || toks.get(j).is_none() {
                break;
            }
        }
        i += 1;
    }
    out
}

/// Scripts, indexed as they're needed.
#[derive(Debug, Default)]
pub struct Index {
    files: HashMap<String, Option<File>>,
}

impl Index {
    pub fn new() -> Index {
        Index::default()
    }

    /// Replaces a file's text (an editor's buffer); later lookups use it.
    pub fn set(&mut self, name: &str, text: Arc<str>) {
        self.files.insert(name.to_ascii_lowercase(), Some(File::new(text)));
    }

    /// Forgets a file (it is read again from the sources when needed).
    pub fn forget(&mut self, name: &str) {
        self.files.remove(&name.to_ascii_lowercase());
    }

    fn file(&mut self, name: &str, sources: &mut dyn Sources) -> Option<&File> {
        let name = name.to_ascii_lowercase();
        if !self.files.contains_key(&name) {
            let f = sources.source(&name).map(File::new);
            self.files.insert(name.clone(), f);
        }
        self.files.get(&name).and_then(Option::as_ref)
    }

    /// A script and everything it includes, in the compiler's order (each
    /// include where it first appears), `nwscript` last.
    pub fn unit(&mut self, name: &str, sources: &mut dyn Sources) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        self.visit(&name.to_ascii_lowercase(), sources, &mut seen, &mut out);
        if !seen.contains(NWSCRIPT) {
            out.push(NWSCRIPT.to_string());
        }
        out
    }

    fn visit(
        &mut self,
        name: &str,
        sources: &mut dyn Sources,
        seen: &mut HashSet<String>,
        out: &mut Vec<String>,
    ) {
        if seen.len() > 256 || !seen.insert(name.to_string()) {
            return;
        }
        out.push(name.to_string());
        let includes: Vec<String> = match self.file(name, sources) {
            Some(f) => f.outline.includes.iter().map(|i| i.name.to_ascii_lowercase()).collect(),
            None => return,
        };
        for inc in includes {
            self.visit(&inc, sources, seen, out);
        }
    }

    /// The top-level declaration `name` has in a script's unit.
    pub fn top_level(
        &mut self,
        file: &str,
        name: &str,
        sources: &mut dyn Sources,
    ) -> Option<Declaration> {
        let unit = self.unit(file, sources);
        let mut prototype = None;
        for f in &unit {
            let Some(idx) = self.file(f, sources) else { continue };
            let o = &idx.outline;
            for func in o.functions.iter().filter(|x| x.name == name) {
                let params: Vec<String> = func
                    .params
                    .iter()
                    .map(|p| match &p.default {
                        Some(d) => format!("{} {} = {d}", p.ty, p.name),
                        None => format!("{} {}", p.ty, p.name),
                    })
                    .collect();
                let d = Declaration {
                    name: name.to_string(),
                    kind: DeclKind::Function,
                    at: Location { file: f.clone(), span: func.name_span.clone() },
                    signature: format!("{} {}({})", func.return_type, func.name, params.join(", ")),
                    doc: func.doc.clone(),
                };
                if func.body.is_some() {
                    // The documentation may be on the prototype.
                    return Some(match prototype {
                        Some(Declaration { doc, .. }) if d.doc.is_empty() => {
                            Declaration { doc, ..d }
                        }
                        _ => d,
                    });
                }
                prototype.get_or_insert(d);
            }
            if prototype.is_some() {
                continue;
            }
            if let Some(g) = o.globals.iter().find(|g| g.name == name) {
                let value = g.value.as_ref().map(|v| format!(" = {v}")).unwrap_or_default();
                return Some(Declaration {
                    name: name.to_string(),
                    kind: if g.is_const { DeclKind::Constant } else { DeclKind::Variable },
                    at: Location { file: f.clone(), span: idx.name_in(&g.span, name) },
                    signature: format!(
                        "{}{} {name}{value}",
                        if g.is_const { "const " } else { "" },
                        g.ty
                    ),
                    doc: g.doc.clone(),
                });
            }
            if let Some(st) = o.structs.iter().find(|s| s.name == name) {
                return Some(Declaration {
                    name: name.to_string(),
                    kind: DeclKind::Struct,
                    at: Location { file: f.clone(), span: idx.name_in(&st.span, name) },
                    signature: format!("struct {name}"),
                    doc: String::new(),
                });
            }
        }
        prototype
    }

    /// What the name at `offset` in `file` declares.
    pub fn declaration_at(
        &mut self,
        file: &str,
        offset: usize,
        sources: &mut dyn Sources,
    ) -> Option<Declaration> {
        let (span, name) = {
            let f = self.file(file, sources)?;
            let span = f.ident_at(offset)?;
            let name = f.word(&span).to_string();
            (span, name)
        };
        self.resolve(file, &span, &name, sources)
    }

    fn resolve(
        &mut self,
        file: &str,
        span: &Range<usize>,
        name: &str,
        sources: &mut dyn Sources,
    ) -> Option<Declaration> {
        let f = self.file(file, sources)?;
        if let Some(l) = f.local(name, span.start) {
            return Some(Declaration {
                name: name.to_string(),
                kind: l.kind,
                at: Location { file: file.to_ascii_lowercase(), span: l.decl.clone() },
                signature: format!("{} {name}", l.ty),
                doc: String::new(),
            });
        }
        self.top_level(file, name, sources)
    }

    /// Where a declaration's name is used (its declarations included), in
    /// `files` (the scripts that may use it: a local's own script is enough).
    pub fn references(
        &mut self,
        decl: &Declaration,
        files: &[String],
        sources: &mut dyn Sources,
    ) -> Vec<Location> {
        let mut candidates: Vec<String> = match decl.kind {
            DeclKind::Local | DeclKind::Parameter => vec![decl.at.file.clone()],
            _ => files.iter().map(|f| f.to_ascii_lowercase()).collect(),
        };
        if !candidates.contains(&decl.at.file) && decl.at.file != NWSCRIPT {
            candidates.push(decl.at.file.clone());
        }
        let mut out = Vec::new();
        for file in candidates {
            let spans: Vec<Range<usize>> = match self.file(&file, sources) {
                Some(f) => f.idents.iter().filter(|r| f.word(r) == decl.name).cloned().collect(),
                None => continue,
            };
            if spans.is_empty() {
                continue;
            }
            // A top-level name is only this declaration in scripts whose
            // unit holds it.
            if !matches!(decl.kind, DeclKind::Local | DeclKind::Parameter)
                && !self.unit(&file, sources).contains(&decl.at.file)
            {
                continue;
            }
            for span in spans {
                if self.resolve(&file, &span, &decl.name, sources).is_some_and(|d| d.at == decl.at)
                {
                    out.push(Location { file: file.clone(), span });
                }
            }
        }
        out.sort_by(|a, b| (&a.file, a.span.start).cmp(&(&b.file, b.span.start)));
        out.dedup();
        out
    }

    /// The edits that rename a declaration in `files`: each place to change
    /// to `new`. Refused for the engine's names, names that aren't
    /// identifiers, and names already taken where the declaration is used.
    pub fn rename(
        &mut self,
        decl: &Declaration,
        new: &str,
        files: &[String],
        sources: &mut dyn Sources,
    ) -> Result<Vec<Location>, String> {
        if decl.at.file == NWSCRIPT {
            return Err(format!("{} is the engine's", decl.name));
        }
        if !is_ident(new) || crate::lex::KEYWORDS.contains(&new) {
            return Err(format!("{new} isn't a name a script can use"));
        }
        if new == decl.name {
            return Err("the new name is the old one".into());
        }
        let places = self.references(decl, files, sources);
        let mut checked = HashSet::new();
        for p in &places {
            let taken = match decl.kind {
                DeclKind::Local | DeclKind::Parameter => {
                    let f = self.file(&p.file, sources).expect("indexed");
                    f.local(new, p.span.start).is_some()
                        || (checked.insert(p.file.clone())
                            && self.top_level(&p.file, new, sources).is_some())
                }
                _ => {
                    checked.insert(p.file.clone())
                        && self.top_level(&p.file, new, sources).is_some()
                }
            };
            if taken {
                return Err(format!("{new} already names something in {}", p.file));
            }
        }
        Ok(places)
    }

    /// The parameters and locals of the function `name` defined in a
    /// script's unit, in declaration order.
    pub fn function_locals(
        &mut self,
        file: &str,
        name: &str,
        sources: &mut dyn Sources,
    ) -> Option<Vec<(String, DeclKind)>> {
        for f in self.unit(file, sources) {
            let Some(idx) = self.file(&f, sources) else { continue };
            let Some(body) = idx.outline.functions.iter().find(|x| x.name == name).and_then(|x| {
                idx.outline
                    .functions
                    .iter()
                    .find(|y| y.name == x.name && y.body.is_some())?
                    .body
                    .clone()
            }) else {
                continue;
            };
            let mut locals: Vec<&Local> = idx
                .locals
                .iter()
                .filter(|l| l.scope.start >= body.start && l.scope.end <= body.end)
                .collect();
            locals.sort_by_key(|l| l.decl.start);
            return Some(locals.into_iter().map(|l| (l.name.clone(), l.kind)).collect());
        }
        None
    }

    /// The names a script can use at `offset`: the locals and parameters in
    /// scope, then every top-level declaration of its unit (engine names
    /// last), each once.
    pub fn completions(
        &mut self,
        file: &str,
        offset: usize,
        sources: &mut dyn Sources,
    ) -> Vec<Declaration> {
        let name = file.to_ascii_lowercase();
        let mut out: Vec<Declaration> = Vec::new();
        let mut seen = HashSet::new();
        if let Some(f) = self.file(&name, sources) {
            let mut locals: Vec<&Local> = f
                .locals
                .iter()
                .filter(|l| l.decl.start <= offset && l.scope.contains(&offset))
                .collect();
            locals.sort_by_key(|l| std::cmp::Reverse(l.decl.start));
            for l in locals {
                if seen.insert(l.name.clone()) {
                    out.push(Declaration {
                        name: l.name.clone(),
                        kind: l.kind,
                        at: Location { file: name.clone(), span: l.decl.clone() },
                        signature: format!("{} {}", l.ty, l.name),
                        doc: String::new(),
                    });
                }
            }
        }
        for f in self.unit(&name, sources) {
            let names: Vec<String> = match self.file(&f, sources) {
                Some(idx) => idx
                    .outline
                    .functions
                    .iter()
                    .map(|x| x.name.clone())
                    .chain(idx.outline.globals.iter().map(|g| g.name.clone()))
                    .chain(idx.outline.structs.iter().map(|s| s.name.clone()))
                    .collect(),
                None => continue,
            };
            for n in names {
                if seen.insert(n.clone())
                    && let Some(d) = self.top_level(&name, &n, sources)
                {
                    out.push(d);
                }
            }
        }
        out
    }

    /// The declarations of a script itself, for an outline.
    pub fn symbols(&mut self, file: &str, sources: &mut dyn Sources) -> Vec<Declaration> {
        let name = file.to_ascii_lowercase();
        let Some(f) = self.file(&name, sources) else { return Vec::new() };
        let mut out = Vec::new();
        let names: Vec<String> = f
            .outline
            .functions
            .iter()
            .filter(|x| x.body.is_some())
            .map(|x| x.name.clone())
            .chain(f.outline.globals.iter().map(|g| g.name.clone()))
            .chain(f.outline.structs.iter().map(|s| s.name.clone()))
            .collect();
        for n in names {
            if let Some(d) = self.top_level(&name, &n, sources).filter(|d| d.at.file == name)
                && !out.iter().any(|o: &Declaration| o.at == d.at)
            {
                out.push(d);
            }
        }
        out.sort_by_key(|d| d.at.span.start);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(
        files: &'static [(&'static str, &'static str)],
    ) -> impl FnMut(&str) -> Option<Arc<str>> {
        move |name: &str| files.iter().find(|(n, _)| *n == name).map(|(_, t)| Arc::from(*t))
    }

    const NWS: &str = "// Gets the caller.\nobject GetSelf();\nint TRUE = 1;\n";
    const INC: &str = "// Doubles.\nint Twice(int n);\nint Twice(int n) { return n * 2; }\nconst int LIMIT = 10;\nstruct Pair { int a; int b; };\n";
    const MAIN: &str = "#include \"inc_math\"\nvoid main()\n{\n    int x = Twice(LIMIT), y;\n    if (x > 0) {\n        int x = 3;\n        y = x;\n    }\n    y = x + Twice(y);\n    struct Pair p;\n    p.a = 1;\n    object o = GetSelf();\n}\n";
    const OTHER: &str = "#include \"inc_math\"\nint Use() { return Twice(2); }\n";
    const FILES: &[(&str, &str)] =
        &[("nwscript", NWS), ("inc_math", INC), ("main", MAIN), ("other", OTHER)];

    fn at(text: &str, needle: &str, nth: usize) -> usize {
        text.match_indices(needle).nth(nth).unwrap().0
    }

    #[test]
    fn names_resolve_to_locals_includes_and_the_engine() {
        let mut src = sources(FILES);
        let mut idx = Index::new();
        let d = idx.declaration_at("main", at(MAIN, "Twice", 0) + 1, &mut src).unwrap();
        assert_eq!((d.kind, d.at.file.as_str()), (DeclKind::Function, "inc_math"));
        assert_eq!(&INC[d.at.span.clone()], "Twice");
        assert!(d.at.span.start > INC.find("{").unwrap() - 30, "the definition, not the prototype");
        assert_eq!(d.doc, "Doubles.");
        assert_eq!(d.signature, "int Twice(int n)");
        let limit = idx.declaration_at("main", at(MAIN, "LIMIT", 0), &mut src).unwrap();
        assert_eq!(limit.kind, DeclKind::Constant);
        let engine = idx.declaration_at("main", at(MAIN, "GetSelf", 0), &mut src).unwrap();
        assert_eq!(
            (engine.at.file.as_str(), engine.doc.as_str()),
            ("nwscript", "Gets the caller.")
        );
        // The inner x shadows the outer one inside its block only.
        let inner = idx.declaration_at("main", at(MAIN, "y = x;", 0) + 4, &mut src).unwrap();
        assert_eq!(inner.at.span.start, at(MAIN, "x = 3", 0));
        let outer = idx.declaration_at("main", at(MAIN, "y = x +", 0) + 4, &mut src).unwrap();
        assert_eq!(outer.at.span.start, at(MAIN, "x = Twice", 0));
        let y = idx.declaration_at("main", at(MAIN, "Twice(y)", 0) + 6, &mut src).unwrap();
        assert_eq!((y.kind, y.at.span.start), (DeclKind::Local, at(MAIN, "y;", 0)));
        let pair = idx.declaration_at("main", at(MAIN, "Pair p", 0), &mut src).unwrap();
        assert_eq!(pair.kind, DeclKind::Struct);
        // Members after `.` aren't resolved.
        assert!(idx.declaration_at("main", at(MAIN, "p.a", 0) + 2, &mut src).is_none());
        let n = idx.declaration_at("inc_math", at(INC, "n * 2", 0), &mut src).unwrap();
        assert_eq!(n.kind, DeclKind::Parameter);
    }

    #[test]
    fn references_and_rename() {
        let mut src = sources(FILES);
        let mut idx = Index::new();
        let files: Vec<String> = ["main", "other", "inc_math"].map(String::from).to_vec();
        let twice = idx.declaration_at("main", at(MAIN, "Twice", 0), &mut src).unwrap();
        let refs = idx.references(&twice, &files, &mut src);
        let by_file = |f: &str| refs.iter().filter(|r| r.file == f).count();
        assert_eq!((by_file("main"), by_file("other"), by_file("inc_math")), (2, 1, 2));
        let edits = idx.rename(&twice, "Double", &files, &mut src).unwrap();
        assert_eq!(edits.len(), 5);
        assert!(idx.rename(&twice, "LIMIT", &files, &mut src).is_err(), "taken");
        assert!(idx.rename(&twice, "2x", &files, &mut src).is_err());
        let engine = idx.declaration_at("main", at(MAIN, "GetSelf", 0), &mut src).unwrap();
        assert!(idx.rename(&engine, "Me", &files, &mut src).is_err());
        // A local's references stay in its scope.
        let outer = idx.declaration_at("main", at(MAIN, "x = Twice", 0), &mut src).unwrap();
        let refs = idx.references(&outer, &files, &mut src);
        let starts: Vec<usize> = refs.iter().map(|r| r.span.start).collect();
        assert_eq!(
            starts,
            [at(MAIN, "x = Twice", 0), at(MAIN, "x > 0", 0), at(MAIN, "x + Twice", 0)]
        );
        // Renaming the outer x to y clashes with the local y.
        assert!(idx.rename(&outer, "y", &files, &mut src).is_err());
        assert_eq!(idx.symbols("inc_math", &mut src).len(), 3);
        let names: Vec<String> = idx
            .completions("main", at(MAIN, "y = x +", 0), &mut src)
            .into_iter()
            .map(|d| d.name)
            .collect();
        assert_eq!(&names[..2], ["y", "x"], "locals in scope first");
        for n in ["Twice", "LIMIT", "Pair", "GetSelf", "TRUE"] {
            assert!(names.contains(&n.to_string()), "{n}");
        }
    }
}
