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
    /// Its documentation, the comment lines above it as the source has
    /// them (`//`, spaces and columns), shown so in Help.
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
    /// When it was said, the local time of day (`14:03:27`): two
    /// compiles in a row read as two (Jasperre's report).
    pub at: String,
}

impl Message {
    /// A message said now.
    pub fn new(
        script: ResKey,
        text: String,
        location: Option<(String, usize)>,
        error: bool,
    ) -> Self {
        Message { script, text, location, error, at: clock() }
    }
}

/// The local time of day, `HH:MM:SS` (UTC where the system's time zone
/// can't be read).
pub fn clock() -> String {
    jiff::Zoned::now().strftime("%H:%M:%S").to_string()
}

/// Find / Replace options.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchOptions {
    /// Off, case and accents are set aside: `arbol` finds `Árbol`.
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
    /// Find in the scripts that are open (Aurora's Find in Currently Open
    /// Scripts), into the Search Results.
    pub in_open: bool,
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

/// Which scripts the Open Script window lists (Aurora's Resources to
/// Show).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScriptsShown {
    /// The module's, its haks' and the game's.
    All,
    #[default]
    Module,
    Haks,
}

/// Where a script the Open Script window lists is from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptFrom {
    Module,
    Hak,
    Game,
}

/// The Open Script window: a script editor's Open….
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OpenScript {
    pub filter: String,
    pub shown: ScriptsShown,
    /// Every script there is to open, by name, read when the window
    /// opens.
    pub scripts: Vec<(ResKey, ScriptFrom)>,
    /// The row the keys are at, among those listed: the first, until Down
    /// and Up move it (Enter opens it while nothing is selected).
    pub cursor: usize,
    /// The scripts selected (a click, Ctrl+click, Shift+click, the
    /// arrows): Enter and Open open them all. By name, so that they stay
    /// selected as the list is narrowed.
    pub selected: std::collections::BTreeSet<ResKey>,
    /// The script a Shift+click or Shift+arrow selects from: the one
    /// clicked or moved to last.
    pub anchor: Option<ResKey>,
}

/// The editor tools' state (shared by the open script editors).
#[derive(Debug, Clone, Default)]
pub struct ScriptTools {
    /// The Open Script window, while it is open.
    pub open_script: Option<OpenScript>,
    /// Scripts to open in the external editor once their editor has them
    /// (opened from the module tree, with the option on).
    pub(crate) open_externally: std::collections::HashSet<ResKey>,
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
    /// A script's indentation to convert, to tabs (`true`) or spaces, once
    /// its editor is drawn (Edit › Convert Indentation…).
    pub convert_indent: Option<(ResKey, bool)>,
    /// A step of a script's own history to take, back (`false`) or again
    /// (`true`), once its editor is drawn: Edit › Undo or Redo, or the
    /// toolbar's, while the script is the one worked in
    /// (`script_view::undo_target`).
    pub(crate) undo: Option<(ResKey, bool)>,
    /// The script whose editor had the keyboard last.
    pub(crate) last_typed: Option<ResKey>,
    /// `nwscript.nss`, parsed once.
    pub spec: Option<Arc<Spec>>,
    /// Symbols of each script (with its includes), by its text.
    pub symbols: HashMap<ResKey, (u64, Arc<Vec<Symbol>>)>,
    /// Each script's text when its editor was last drawn, and the
    /// workspace revision then: an include edited or the module changed
    /// leaves other scripts' symbols and live checks out of date.
    pub(crate) seen: HashMap<ResKey, u64>,
    pub(crate) revision: u64,
}

/// A script's text as a number, to tell when it changed.
pub(crate) fn hash(text: &str) -> u64 {
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
            doc: f.doc_source.clone(),
            custom: false,
        })
        .collect();
    out.extend(spec.constants.iter().map(|c| Symbol {
        name: c.name.clone(),
        kind: SymbolKind::Constant,
        signature: format!("{} {} = {}", c.ty, c.name, c.value),
        doc: c.doc_source.clone(),
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
                doc: f.doc_source.clone(),
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
                doc: g.doc_source.clone(),
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
    // Prototypes and definitions name a function twice. (The first's,
    // with the documentation of whichever has some: BioWare's includes
    // document a prototype with `//` lines, or only the definition, with a
    // `/* … */` block.)
    let mut unique: Vec<Symbol> = Vec::new();
    for s in out {
        match unique.iter_mut().find(|u| u.name == s.name && u.kind == s.kind) {
            Some(u) if u.doc.is_empty() => u.doc = s.doc,
            Some(_) => {}
            None => unique.push(s),
        }
    }
    unique
}

impl ScriptTools {
    /// Says in the Compiler tab that `key` was saved and not compiled
    /// (compiling on save is off), in place of what it said of the script
    /// before: a builder takes the last compile's "compiled" for this
    /// text's otherwise (Jasperre's report). A script's own Save and the
    /// module's both say it.
    pub(crate) fn saved_not_compiled(&mut self, key: ResKey) {
        self.messages.retain(|m| m.script != key);
        let said = format!(
            "{key}: saved, not compiled (Options › Script Editor › Automatically Compile \
             Scripts on Save is off)"
        );
        self.messages.push(Message::new(key, said, None, false));
        self.info = InfoTab::Compiler;
    }

    /// All symbols a script can use: its own and its includes' (custom),
    /// then `nwscript.nss`'s; cached by the script's text (`h`, its
    /// [`hash`]). In a frame of `typing` (a key went to the text), those
    /// read before: read again on every key, a script of 14,000 lines took
    /// a third of the frame; the next frame without one reads them.
    pub fn symbols_for(
        &mut self,
        key: ResKey,
        h: u64,
        typing: bool,
        text: &str,
        load_spec: impl FnOnce() -> Option<Spec>,
        source: &mut dyn FnMut(&str) -> Option<String>,
    ) -> Arc<Vec<Symbol>> {
        if self.spec.is_none() {
            self.spec = Some(Arc::new(load_spec().unwrap_or_default()));
        }
        if let Some((cached, symbols)) = self.symbols.get(&key)
            && (*cached == h || typing)
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

/// The character index `lines` lines down from `at` (up, if negative), in
/// the same column or at the end of a shorter line; past the first line the
/// text's start, past the last its end (Page Up and Page Down).
pub fn line_moved(text: &str, at: usize, lines: isize) -> usize {
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.chars().enumerate().filter(|(_, c)| *c == '\n').map(|(i, _)| i + 1))
        .collect();
    let line = starts.partition_point(|s| *s <= at) - 1;
    let target = line as isize + lines;
    if target < 0 {
        return 0;
    }
    let Some(start) = starts.get(target as usize) else { return text.chars().count() };
    let end = starts.get(target as usize + 1).map_or(text.chars().count(), |s| s - 1);
    (start + (at - starts[line])).min(end)
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

/// What a new line begins with, started at character `at`: the spaces and
/// tabs its line begins with (those before `at`), so that the new line is
/// as far in as the one it is started from, as in Aurora.
pub fn new_line_indent(text: &str, at: usize) -> String {
    let before: String = text.chars().take(at).collect();
    let line = before.rsplit('\n').next().unwrap_or_default();
    line.chars().take_while(|c| *c == ' ' || *c == '\t').collect()
}

/// One step of indentation: a tab, or `width` spaces; as the script is
/// indented ([`Indent::of_script`]), else as Options › Script Editor has
/// it ([`Indent::of`]: four spaces unless chosen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Indent {
    pub tabs: bool,
    pub width: usize,
}

/// How a script is indented, as guessed from its text ([`Indent::guess`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guess {
    Tabs,
    /// This many spaces a step.
    Spaces(usize),
}

impl Indent {
    /// The style's: [`ScriptStyle`](crate::settings::ScriptStyle)'s.
    pub fn of(style: &crate::settings::ScriptStyle) -> Indent {
        Indent { tabs: style.indent_tabs, width: usize::from(style.indent_width.clamp(1, 16)) }
    }

    /// A script's: as it is indented itself, else (nothing indented to go
    /// by) the style's, `style`. A tab is as wide as the style says.
    pub fn of_script(guess: Option<Guess>, style: Indent) -> Indent {
        match guess {
            Some(Guess::Tabs) => Indent { tabs: true, width: style.width },
            Some(Guess::Spaces(width)) => Indent { tabs: false, width },
            None => style,
        }
    }

    /// How `text` is indented, the common rule: by tabs or spaces as most
    /// of its indented lines are, and a step of the spaces by which a
    /// line is most often further in than the one before it (the lines
    /// of a `/* … */` block's ` * ` side aside). `None` with no line
    /// indented.
    pub fn guess(text: &str) -> Option<Guess> {
        let (mut tabs, mut spaces) = (0usize, 0usize);
        let mut steps: HashMap<usize, usize> = HashMap::new();
        let mut before: Option<usize> = None;
        let mut least = usize::MAX;
        for line in text.lines() {
            let rest = line.trim_start_matches([' ', '\t']);
            if rest.is_empty() || rest.starts_with('*') {
                continue;
            }
            let lead = &line[..line.len() - rest.len()];
            if lead.contains('\t') {
                tabs += 1;
                before = None;
                continue;
            }
            let n = lead.len();
            if n > 0 {
                spaces += 1;
                least = least.min(n);
            }
            if let Some(b) = before.filter(|b| n > *b) {
                *steps.entry(n - b).or_default() += 1;
            }
            before = Some(n);
        }
        if tabs + spaces == 0 {
            return None;
        }
        if tabs > spaces {
            return Some(Guess::Tabs);
        }
        // (The most often; of as often, the narrowest. Every line as far
        // in: that far.)
        let step = steps.into_iter().max_by_key(|&(w, n)| (n, std::cmp::Reverse(w)));
        Some(Guess::Spaces(step.map_or(least, |(w, _)| w).clamp(1, 16)))
    }

    /// One step: a tab, or the width's spaces.
    pub fn unit(self) -> String {
        if self.tabs { "\t".into() } else { " ".repeat(self.width) }
    }

    /// What Tab puts in at character `at`: a tab, or spaces to the next
    /// column that is a multiple of the width (a tab before it counted to
    /// its own).
    pub fn at(self, text: &str, at: usize) -> String {
        if self.tabs {
            return "\t".into();
        }
        let before: String = text.chars().take(at).collect();
        let line = before.rsplit('\n').next().unwrap_or_default();
        let column = line.chars().fold(0, |col, c| {
            if c == '\t' { (col / self.width + 1) * self.width } else { col + 1 }
        });
        " ".repeat(self.width - column % self.width)
    }
}

/// Indents (one step before each line that has anything on it) or
/// outdents (one leading tab, or up to a step's spaces, off each) the
/// lines a selection touches, as Tab and Shift+Tab do in the script
/// editor. Character indexes in and out; the selection returned covers the
/// lines, from the first's start.
pub fn indent_lines(
    text: &str,
    from: usize,
    to: usize,
    outdent: bool,
    indent: Indent,
) -> (String, usize, usize) {
    let unit = indent.unit();
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
            // (An empty line is left empty: no trailing spaces.)
            if !outdent {
                return if line.is_empty() { String::new() } else { format!("{unit}{line}") };
            }
            if let Some(rest) = line.strip_prefix('\t') {
                return rest.to_string();
            }
            let spaces = line.chars().take(indent.width).take_while(|&c| c == ' ').count();
            line[spaces..].to_string()
        })
        .collect();
    let new_block = lines.join("\n");
    let head: String = chars[..start].iter().collect();
    let tail: String = chars[end..].iter().collect();
    let new_end = start + new_block.chars().count();
    (format!("{head}{new_block}{tail}"), start, new_end)
}

/// `text` with each line's indentation (the spaces and tabs it begins
/// with) made tabs or spaces, as Convert Indentation to Tabs and to
/// Spaces do: as far in as before, a tab `width` columns. To tabs, what is
/// left of a step stays spaces (a ` * ` under `/*`, a column lined up).
pub fn convert_indentation(text: &str, tabs: bool, width: usize) -> String {
    let width = width.max(1);
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| {
            let rest = line.trim_start_matches([' ', '\t']);
            let lead = &line[..line.len() - rest.len()];
            let columns = lead
                .chars()
                .fold(0, |col, c| if c == '\t' { (col / width + 1) * width } else { col + 1 });
            let lead = if tabs {
                format!("{}{}", "\t".repeat(columns / width), " ".repeat(columns % width))
            } else {
                " ".repeat(columns)
            };
            format!("{lead}{rest}")
        })
        .collect();
    lines.join("\n")
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
/// as a character range. Without Match case, letter by letter with case
/// and accents aside ([`mg_core::text::same_letter`]), so what is found
/// is as long as `needle`.
pub fn find(text: &str, needle: &str, from: usize, o: &SearchOptions) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let hay: Vec<char> = text.chars().collect();
    let pat: Vec<char> = needle.chars().collect();
    let last = hay.len().checked_sub(pat.len())?;
    let same =
        |a: char, b: char| if o.match_case { a == b } else { mg_core::text::same_letter(a, b) };
    let whole = |s: usize| {
        let before = s == 0 || !is_ident(hay[s - 1]);
        let after = hay.get(s + pat.len()).is_none_or(|&c| !is_ident(c));
        !o.whole_word || (before && after)
    };
    // (Matches don't overlap, from the text's start: as Replace All
    // replaces them.)
    let mut matches = Vec::new();
    let mut s = 0;
    while s <= last {
        if hay[s..s + pat.len()].iter().zip(&pat).all(|(&h, &p)| same(h, p)) && whole(s) {
            matches.push(s);
            s += pat.len();
        } else {
            s += 1;
        }
    }
    let pick = if o.backwards {
        matches.iter().rev().find(|s| **s < from).or(matches.last())
    } else {
        matches.iter().find(|s| **s >= from).or(matches.first())
    };
    pick.map(|&s| (s, s + pat.len()))
}

/// Whether the characters `start..end` of `text` are a match of `needle`
/// as [`find`] finds one (its case and whole-word rules): what Replace
/// replaces. (Replace compared the selection itself, A to Z alone and
/// whole words or not, and refused what Find had found: "É" for "é".)
pub fn is_match_at(text: &str, needle: &str, start: usize, end: usize, o: &SearchOptions) -> bool {
    let forward = SearchOptions { backwards: false, ..o.clone() };
    start != end && find(text, needle, start, &forward) == Some((start, end))
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
    use super::{Guess, Indent};

    /// A script's indentation is guessed from its indented lines (Jasperre's
    /// report: BioWare's use tabs and spaces both): tabs or spaces as most
    /// are, the step most often taken; nothing to go by, none.
    #[test]
    fn indentation_is_guessed_from_the_script() {
        let guess = Indent::guess;
        assert_eq!(guess("void main()\n{\n}\n"), None);
        assert_eq!(guess(""), None);
        assert_eq!(guess("void main()\n{\n\tint n;\n\tif (n)\n\t\tn++;\n}\n"), Some(Guess::Tabs));
        let two = "void main()\n{\n  int n;\n  if (n)\n  {\n    n++;\n  }\n}\n";
        assert_eq!(guess(two), Some(Guess::Spaces(2)));
        let four = "void main()\n{\n    int n;\n    if (n)\n    {\n        n++;\n    }\n}\n";
        assert_eq!(guess(four), Some(Guess::Spaces(4)));
        // More lines in spaces than in tabs: spaces. (A line lined up
        // further in doesn't make the step.)
        let mixed = "{\n\tint n;\n    int m;\n    int k = 1 +\n             2;\n    \
                     if (m)\n        m++;\n    if (k)\n        k++;\n}\n";
        assert_eq!(guess(mixed), Some(Guess::Spaces(4)));
        // A block comment's ` * ` side isn't indentation.
        let block = "/*\n * Doc.\n * More.\n */\nvoid main()\n{\n\tint n;\n}\n";
        assert_eq!(guess(block), Some(Guess::Tabs));
        // Every line as far in, three spaces: three.
        assert_eq!(guess("   a;\n   b;\n"), Some(Guess::Spaces(3)));
        // As the script is, else as Options say.
        let style = Indent { tabs: false, width: 4 };
        assert_eq!(Indent::of_script(None, style), style);
        assert_eq!(
            Indent::of_script(Some(Guess::Spaces(2)), style),
            Indent { tabs: false, width: 2 }
        );
        assert_eq!(Indent::of_script(Some(Guess::Tabs), style), Indent { tabs: true, width: 4 });
    }

    #[test]
    fn indentation_converts_to_tabs_and_to_spaces() {
        let spaces = "void main()\n{\n    int n;\n      // lined up\n        n++;\n\n}\n";
        let tabs = super::convert_indentation(spaces, true, 4);
        assert_eq!(tabs, "void main()\n{\n\tint n;\n\t  // lined up\n\t\tn++;\n\n}\n");
        assert_eq!(super::convert_indentation(&tabs, false, 4), spaces);
        // A tab after spaces goes to the next stop; two-space steps.
        assert_eq!(super::convert_indentation("  \tx\n", false, 4), "    x\n");
        assert_eq!(super::convert_indentation("  a\n    b\n", true, 2), "\ta\n\t\tb\n");
    }

    #[test]
    fn page_keys_move_by_lines_in_the_same_column() {
        let text = "abcd\nx\nefgh\nij";
        // From "c" (column 2) two lines down: "g".
        assert_eq!(super::line_moved(text, 2, 2), 9);
        // One down: the end of the short line "x".
        assert_eq!(super::line_moved(text, 2, 1), 6);
        // Back up from "g": "c".
        assert_eq!(super::line_moved(text, 9, -2), 2);
        // Past the first line, the start; past the last, the end.
        assert_eq!(super::line_moved(text, 9, -10), 0);
        assert_eq!(super::line_moved(text, 2, 10), text.chars().count());
        // A cursor at a line's end (on its "\n") stays in that line.
        assert_eq!(super::line_moved(text, 4, 2), 11);
        assert_eq!(super::line_moved("", 0, 5), 0);
    }

    #[test]
    fn tab_indents_and_shift_tab_outdents_the_selected_lines() {
        use super::Indent;
        let tab = Indent { tabs: true, width: 4 };
        let text = "a\nb\nc\n";
        // From inside "a" to inside "b": lines a and b.
        let (t, from, to) = super::indent_lines(text, 0, 3, false, tab);
        assert_eq!(t, "\ta\n\tb\nc\n");
        assert_eq!((from, to), (0, 5));
        let (back, _, _) = super::indent_lines(&t, from, to, true, tab);
        assert_eq!(back, text);
        // A selection ending at a line's start leaves that line alone.
        assert_eq!(super::indent_lines(text, 0, 2, false, tab).0, "\ta\nb\nc\n");
        // Up to a step's spaces come off.
        assert_eq!(super::indent_lines("      x", 2, 2, true, tab).0, "  x");
        // Spaces, the default: four (or as many as chosen) on, as many off,
        // or a tab; an empty line stays empty.
        let four = Indent { tabs: false, width: 4 };
        let (t, from, to) = super::indent_lines("a\n\nb", 0, 4, false, four);
        assert_eq!(t, "    a\n\n    b");
        assert_eq!(super::indent_lines(&t, from, to, true, four).0, "a\n\nb");
        let two = Indent { tabs: false, width: 2 };
        assert_eq!(super::indent_lines("     x\n\ty", 0, 8, true, two).0, "   x\ny");
        // Tab alone: to the next multiple of the width, or a tab.
        assert_eq!(four.at("ab", 2), "  ");
        assert_eq!(four.at("abcd", 4), "    ");
        assert_eq!(four.at("x\n\ta", 4), "   ", "the tab counts to column 4");
        assert_eq!(two.at("", 0), "  ");
        assert_eq!(tab.at("abc", 1), "\t");
        // A new line begins as far in as the line it is started from.
        let text = "void main()\n{\n    int n;\n\tint m;\n}\n";
        let at = |part: &str| text.find(part).unwrap() + part.len();
        assert_eq!(super::new_line_indent(text, at("int n;")), "    ");
        assert_eq!(super::new_line_indent(text, at("int m;")), "\t");
        assert_eq!(super::new_line_indent(text, at("main()")), "");
        assert_eq!(super::new_line_indent(text, at("{\n  ")), "  ", "within the indent: to there");
        assert_eq!(super::new_line_indent("", 0), "");
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
        let whole = SearchOptions { whole_word: true, match_case: true, ..o };
        assert_eq!(find(t, "Foo", 1, &whole), Some((13, 16)));
        let back = SearchOptions { backwards: true, ..o };
        assert_eq!(find(t, "foo", 4, &back), Some((0, 3)));
        assert_eq!(replace_all(t, "foo", "bar", &whole), ("Foo bar food Foo".into(), 1));
        assert_eq!(
            replace_all(t, "foo", "x", &SearchOptions { whole_word: true, ..o }),
            ("x x food x".into(), 3)
        );
        // A selection is replaced when it is what Find finds there: by its
        // case rule (É for é, as Find takes it) and whole words.
        let accents = "Été été";
        assert_eq!(find(accents, "été", 0, &o), Some((0, 3)));
        assert!(is_match_at(accents, "été", 0, 3, &o));
        assert!(!is_match_at(t, "foo", 8, 11, &SearchOptions { whole_word: true, ..o }), "food");
        assert!(is_match_at(t, "foo", 8, 11, &o));
        assert!(!is_match_at(t, "foo", 0, 3, &SearchOptions { match_case: true, ..o }));
        // Without Match case, accents are set aside as case is; with it,
        // both kept.
        let tree = "// Árbol, arbol";
        assert_eq!(find(tree, "arbol", 0, &o), Some((3, 8)));
        assert_eq!(find(tree, "ARBOL", 4, &o), Some((10, 15)));
        let exact = SearchOptions { match_case: true, ..o };
        assert_eq!(find(tree, "arbol", 0, &exact), Some((10, 15)));
        assert_eq!(replace_all(tree, "arbol", "roble", &o), ("// roble, roble".into(), 2));
        assert_eq!(find_lines("Árbol\nx\narbol", "arbol", &o).len(), 2);
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
