//! The script editor's tools, as in Aurora's Script Editor: the lists of
//! functions, variables, constants and templates (with help), compiler
//! messages that jump to their line, find and replace (also in all module
//! scripts), bookmarks and completion. Text positions are character
//! indices, as egui's text cursor counts them.

use std::collections::HashMap;
use std::sync::Arc;

use mg_core::ResType;
use mg_resman::ResKey;
use mg_script::outline::outline;
use mg_script::spec::Spec;

/// What a symbol in the lists is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolKind {
    Function,
    Variable,
    Constant,
}

/// A function, variable or constant the script can use.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// The declaration as shown in Help.
    pub signature: String,
    pub doc: String,
    /// Declared by the script or its includes (not `nwscript.nss`); Aurora
    /// shows these in bold.
    pub custom: bool,
}

/// The side lists' tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SideTab {
    #[default]
    Functions,
    Variables,
    Constants,
    Templates,
}

/// The bottom panel's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InfoTab {
    #[default]
    Compiler,
    Help,
    Bookmarks,
    SearchResults,
}

/// A compiler message, with where it points if it says.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub script: ResKey,
    pub text: String,
    /// Script name (an include when the error is in one) and 1-based line.
    pub location: Option<(String, usize)>,
    pub error: bool,
}

/// Find / Replace options.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchOptions {
    pub match_case: bool,
    pub whole_word: bool,
    pub backwards: bool,
}

/// The Find Text window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Search {
    pub open: bool,
    pub replace_mode: bool,
    pub find: String,
    pub replace: String,
    pub options: SearchOptions,
    pub in_files: bool,
    /// The script the window works on.
    pub script: Option<ResKey>,
    pub results: Vec<(ResKey, usize, String)>,
}

/// An open completion list.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub script: ResKey,
    /// Where the word being completed starts.
    pub start: usize,
    pub items: Vec<Symbol>,
    pub selected: usize,
}

/// The editor tools' state (shared by the open script editors).
#[derive(Debug, Clone, Default)]
pub struct ScriptTools {
    pub side: SideTab,
    pub filter: String,
    pub info: InfoTab,
    pub help: Option<Symbol>,
    pub messages: Vec<Message>,
    pub search: Search,
    pub completion: Option<Completion>,
    /// A cursor move waiting for its editor: script and character index.
    pub jump: Option<(ResKey, usize)>,
    /// An editor to scroll to its cursor once laid out.
    pub scroll: Option<ResKey>,
    /// `nwscript.nss`, parsed once.
    pub spec: Option<Arc<Spec>>,
    /// Symbols of each script (with its includes), by its text.
    pub symbols: HashMap<ResKey, (u64, Arc<Vec<Symbol>>)>,
}

fn hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

fn from_spec(spec: &Spec) -> Vec<Symbol> {
    let mut out: Vec<Symbol> = spec
        .functions
        .iter()
        .map(|f| Symbol {
            name: f.name.clone(),
            kind: SymbolKind::Function,
            signature: f.signature(),
            doc: f.doc.clone(),
            custom: false,
        })
        .collect();
    out.extend(spec.constants.iter().map(|c| Symbol {
        name: c.name.clone(),
        kind: SymbolKind::Constant,
        signature: format!("{} {} = {}", c.ty, c.name, c.value),
        doc: c.doc.clone(),
        custom: false,
    }));
    out
}

/// The symbols declared by a script and (recursively) its includes;
/// `source` loads an include's text.
pub fn script_symbols(text: &str, source: &mut dyn FnMut(&str) -> Option<String>) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut queue = vec![text.to_string()];
    while let Some(src) = queue.pop() {
        let o = outline(src.as_bytes());
        for f in &o.functions {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| match &p.default {
                    Some(d) => format!("{} {} = {}", p.ty, p.name, d),
                    None => format!("{} {}", p.ty, p.name),
                })
                .collect();
            out.push(Symbol {
                name: f.name.clone(),
                kind: SymbolKind::Function,
                signature: format!("{} {}({})", f.return_type, f.name, params.join(", ")),
                doc: f.doc.clone(),
                custom: true,
            });
        }
        for g in &o.globals {
            let value = g.value.as_ref().map(|v| format!(" = {v}")).unwrap_or_default();
            out.push(Symbol {
                name: g.name.clone(),
                kind: if g.is_const { SymbolKind::Constant } else { SymbolKind::Variable },
                signature: format!(
                    "{}{} {}{value}",
                    if g.is_const { "const " } else { "" },
                    g.ty,
                    g.name
                ),
                doc: g.doc.clone(),
                custom: true,
            });
        }
        for inc in &o.includes {
            let name = inc.name.to_ascii_lowercase();
            if seen.len() < 64
                && seen.insert(name.clone())
                && let Some(text) = source(&name)
            {
                queue.push(text);
            }
        }
    }
    // Prototypes and definitions name a function twice.
    let mut unique: Vec<Symbol> = Vec::new();
    for s in out {
        if !unique.iter().any(|u| u.name == s.name && u.kind == s.kind) {
            unique.push(s);
        }
    }
    unique
}

impl ScriptTools {
    /// All symbols a script can use: its own and its includes' (custom),
    /// then `nwscript.nss`'s; cached by the script's text.
    pub fn symbols_for(
        &mut self,
        key: ResKey,
        text: &str,
        load_spec: impl FnOnce() -> Option<Spec>,
        source: &mut dyn FnMut(&str) -> Option<String>,
    ) -> Arc<Vec<Symbol>> {
        if self.spec.is_none() {
            self.spec = Some(Arc::new(load_spec().unwrap_or_default()));
        }
        let h = hash(text);
        if let Some((cached, symbols)) = self.symbols.get(&key)
            && *cached == h
        {
            return symbols.clone();
        }
        let mut all = script_symbols(text, source);
        all.extend(from_spec(self.spec.as_ref().expect("loaded")));
        let all = Arc::new(all);
        self.symbols.insert(key, (h, all.clone()));
        all
    }
}

/// The byte index of a character index.
pub fn byte_index(text: &str, chars: usize) -> usize {
    text.char_indices().nth(chars).map_or(text.len(), |(b, _)| b)
}

/// The character index of a byte index.
pub fn char_index(text: &str, bytes: usize) -> usize {
    text[..bytes.min(text.len())].chars().count()
}

/// The character index where a line (0-based) starts.
pub fn line_start(text: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    let mut n = 0;
    for (i, c) in text.chars().enumerate() {
        if c == '\n' {
            n += 1;
            if n == line {
                return i + 1;
            }
        }
    }
    text.chars().count()
}

/// The line (0-based) of a character index.
pub fn line_of(text: &str, chars: usize) -> usize {
    text.chars().take(chars).filter(|c| *c == '\n').count()
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The identifier ending at a character index: its start and text.
pub fn word_before(text: &str, chars: usize) -> (usize, String) {
    let before: Vec<char> = text.chars().take(chars).collect();
    let start = before.iter().rposition(|c| !is_ident(*c)).map_or(0, |p| p + 1);
    (start, before[start..].iter().collect())
}

/// Indents (a tab before each) or outdents (one leading tab, or up to four
/// spaces, off each) the lines a selection touches, as Tab and Shift+Tab do
/// in the script editor. Character indexes in and out; the selection
/// returned covers the lines, from the first's start.
pub fn indent_lines(text: &str, from: usize, to: usize, outdent: bool) -> (String, usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let (from, to) = (from.min(chars.len()), to.min(chars.len()).max(from.min(chars.len())));
    let start = chars[..from].iter().rposition(|&c| c == '\n').map_or(0, |i| i + 1);
    // A selection ending at a line's start leaves that line alone.
    let last = if to > from && chars[to - 1] == '\n' { to - 1 } else { to };
    let end = chars[last..].iter().position(|&c| c == '\n').map_or(chars.len(), |i| last + i);
    let block: String = chars[start..end].iter().collect();
    let lines: Vec<String> = block
        .split('\n')
        .map(|line| {
            if !outdent {
                return format!("\t{line}");
            }
            if let Some(rest) = line.strip_prefix('\t') {
                return rest.to_string();
            }
            let spaces = line.chars().take(4).take_while(|&c| c == ' ').count();
            line[spaces..].to_string()
        })
        .collect();
    let new_block = lines.join("\n");
    let head: String = chars[..start].iter().collect();
    let tail: String = chars[end..].iter().collect();
    let new_end = start + new_block.chars().count();
    (format!("{head}{new_block}{tail}"), start, new_end)
}

/// The identifier around a character index.
pub fn word_at(text: &str, chars: usize) -> Option<String> {
    let all: Vec<char> = text.chars().collect();
    let mut start = chars.min(all.len());
    while start > 0 && is_ident(all[start - 1]) {
        start -= 1;
    }
    let mut end = chars.min(all.len());
    while end < all.len() && is_ident(all[end]) {
        end += 1;
    }
    (end > start).then(|| all[start..end].iter().collect())
}

/// The next match of `needle` from a character index (wrapping around),
/// as a character range.
pub fn find(text: &str, needle: &str, from: usize, o: &SearchOptions) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let (hay, pat) = if o.match_case {
        (text.to_string(), needle.to_string())
    } else {
        (text.to_lowercase(), needle.to_lowercase())
    };
    // Lower-casing can change byte lengths; fall back to exact case then.
    let (hay, pat) =
        if hay.len() == text.len() { (hay, pat) } else { (text.to_string(), needle.to_string()) };
    let whole = |b: usize| {
        let before = hay[..b].chars().next_back().is_none_or(|c| !is_ident(c));
        let after = hay[b + pat.len()..].chars().next().is_none_or(|c| !is_ident(c));
        !o.whole_word || (before && after)
    };
    let matches: Vec<usize> =
        hay.match_indices(&pat).map(|(b, _)| b).filter(|b| whole(*b)).collect();
    let from_b = byte_index(text, from);
    let pick = if o.backwards {
        matches.iter().rev().find(|b| **b < from_b).or(matches.last())
    } else {
        matches.iter().find(|b| **b >= from_b).or(matches.first())
    };
    pick.map(|b| (char_index(text, *b), char_index(text, b + pat.len())))
}

/// Every line containing `needle`: (0-based line, the line's text).
pub fn find_lines(text: &str, needle: &str, o: &SearchOptions) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| {
            find(l, needle, 0, &SearchOptions { backwards: false, ..o.clone() }).is_some()
        })
        .map(|(i, l)| (i, l.trim().to_string()))
        .collect()
}

/// Replaces every match; returns the new text and how many were replaced.
pub fn replace_all(text: &str, needle: &str, with: &str, o: &SearchOptions) -> (String, usize) {
    let mut out = text.to_string();
    let mut n = 0;
    let mut from = 0;
    let forward = SearchOptions { backwards: false, ..o.clone() };
    while let Some((s, e)) = find(&out, needle, from, &forward) {
        if s < from || n > out.len() {
            break;
        }
        let (bs, be) = (byte_index(&out, s), byte_index(&out, e));
        out.replace_range(bs..be, with);
        from = s + with.chars().count();
        n += 1;
    }
    (out, n)
}

/// Completion candidates: symbols starting with the prefix (any case),
/// custom ones first, then by name.
pub fn completions(symbols: &[Symbol], prefix: &str) -> Vec<Symbol> {
    let p = prefix.to_ascii_lowercase();
    let mut out: Vec<Symbol> =
        symbols.iter().filter(|s| s.name.to_ascii_lowercase().starts_with(&p)).cloned().collect();
    out.sort_by(|a, b| b.custom.cmp(&a.custom).then_with(|| a.name.cmp(&b.name)));
    out.dedup_by(|a, b| a.name == b.name);
    out.truncate(200);
    out
}

/// Script templates: the `.txt` files of the install's `data/scr`, the
/// user's `scripttemplates` and the chosen templates folder (`extra`), by
/// name.
pub fn templates(
    install: Option<&mg_resman::GameInstall>,
    extra: Option<&std::path::Path>,
) -> Vec<(String, std::path::PathBuf)> {
    let mut dirs = Vec::new();
    if let Some(i) = install {
        dirs.push(i.root.join("data").join("scr"));
        if let Some(u) = &i.user_dir {
            dirs.push(u.join("scripttemplates"));
        }
    }
    if let Some(e) = extra {
        dirs.push(e.to_path_buf());
    }
    let mut out: Vec<(String, std::path::PathBuf)> = dirs
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("txt")))
        .filter_map(|p| Some((p.file_stem()?.to_string_lossy().into_owned(), p)))
        .collect();
    out.sort();
    out
}

/// A script's resource key.
pub fn nss(name: &str) -> Option<ResKey> {
    ResKey::parse(name, ResType::NSS)
}

#[cfg(test)]
mod tests {
    #[test]
    fn tab_indents_and_shift_tab_outdents_the_selected_lines() {
        let text = "a\nb\nc\n";
        // From inside "a" to inside "b": lines a and b.
        let (t, from, to) = super::indent_lines(text, 0, 3, false);
        assert_eq!(t, "\ta\n\tb\nc\n");
        assert_eq!((from, to), (0, 5));
        let (back, _, _) = super::indent_lines(&t, from, to, true);
        assert_eq!(back, text);
        // A selection ending at a line's start leaves that line alone.
        assert_eq!(super::indent_lines(text, 0, 2, false).0, "\ta\nb\nc\n");
        // Up to four spaces come off.
        assert_eq!(super::indent_lines("      x", 2, 2, true).0, "  x");
    }

    use super::*;

    #[test]
    fn positions() {
        let t = "ab\ncdé\nf";
        assert_eq!(line_start(t, 1), 3);
        assert_eq!(line_start(t, 2), 7);
        assert_eq!(line_of(t, 7), 2);
        assert_eq!(byte_index(t, 6), 7, "é is two bytes");
        assert_eq!(word_before("x = GetFir", 10), (4, "GetFir".into()));
        assert_eq!(word_at("int nCount;", 6).as_deref(), Some("nCount"));
    }

    #[test]
    fn find_and_replace() {
        let o = SearchOptions::default();
        let t = "Foo foo food Foo";
        assert_eq!(find(t, "foo", 0, &o), Some((0, 3)));
        assert_eq!(find(t, "foo", 1, &o), Some((4, 7)));
        let whole = SearchOptions { whole_word: true, match_case: true, ..o.clone() };
        assert_eq!(find(t, "Foo", 1, &whole), Some((13, 16)));
        let back = SearchOptions { backwards: true, ..o.clone() };
        assert_eq!(find(t, "foo", 4, &back), Some((0, 3)));
        assert_eq!(replace_all(t, "foo", "bar", &whole), ("Foo bar food Foo".into(), 1));
        assert_eq!(
            replace_all(t, "foo", "x", &SearchOptions { whole_word: true, ..o }),
            ("x x food x".into(), 3)
        );
    }

    #[test]
    fn symbols_of_a_script_and_its_includes() {
        let main = "#include \"inc_a\"\nconst int LIMIT = 3;\nint nCount;\nvoid main() {}\n";
        let mut source = |n: &str| {
            (n == "inc_a").then(|| "// Adds.\nint Add(int a, int b = 1);\nint Add(int a, int b = 1) { return a + b; }\n".to_string())
        };
        let s = script_symbols(main, &mut source);
        let names: Vec<(&str, SymbolKind)> = s.iter().map(|s| (s.name.as_str(), s.kind)).collect();
        assert!(names.contains(&("LIMIT", SymbolKind::Constant)));
        assert!(names.contains(&("nCount", SymbolKind::Variable)));
        assert!(names.contains(&("main", SymbolKind::Function)));
        let add = s.iter().find(|s| s.name == "Add").unwrap();
        assert_eq!(add.signature, "int Add(int a, int b = 1)");
        assert_eq!(s.iter().filter(|s| s.name == "Add").count(), 1);
        let c = completions(&s, "ad");
        assert_eq!(c[0].name, "Add");
    }
}
