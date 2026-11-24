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

/// Symbols available to a script: its own and its includes' declarations, plus
/// the shared `nwscript.nss` spec symbols, merged lazily in sorted name order
/// (`own` before `spec` on equal names).
#[derive(Debug, Clone, Default)]
pub struct SymbolSet {
    pub own: Arc<Vec<Symbol>>,
    pub spec: Arc<Vec<Symbol>>,
}

impl SymbolSet {
    /// Total symbol count (`own` plus `spec`).
    pub fn len(&self) -> usize {
        self.own.len() + self.spec.len()
    }

    /// Whether neither `own` nor `spec` has symbols.
    pub fn is_empty(&self) -> bool {
        self.own.is_empty() && self.spec.is_empty()
    }

    /// Whether `other` is this very set (the same `own` and `spec`). A
    /// cache that asks this holds the set it was made from: keyed by the
    /// sets' addresses alone, a set dropped and another made where it was
    /// took the old one's answer (review D3).
    pub fn same(&self, other: &SymbolSet) -> bool {
        Arc::ptr_eq(&self.own, &other.own) && Arc::ptr_eq(&self.spec, &other.spec)
    }

    /// Sorted iterator yielding all symbols (`own` before `spec` on ties).
    pub fn iter(&self) -> SymbolIter<'_> {
        SymbolIter { own: &self.own, spec: &self.spec, i: 0, j: 0 }
    }

    /// Binary search by exact identifier name (`own` before `spec`).
    pub fn find_by_name(&self, name: &str) -> Option<&Symbol> {
        let i = self.own.partition_point(|s| s.name.as_str() < name);
        if let Some(s) = self.own.get(i)
            && s.name == name
        {
            return Some(s);
        }
        let j = self.spec.partition_point(|s| s.name.as_str() < name);
        self.spec.get(j).filter(|s| s.name == name)
    }

    /// Completion candidates starting with `prefix` (custom first, then by name).
    pub fn completions(&self, prefix: &str) -> Vec<Symbol> {
        completions_from(self.own.iter().chain(self.spec.iter()), prefix)
    }
}

/// Lazy 2-way merge iterator over a [`SymbolSet`].
#[derive(Debug)]
pub struct SymbolIter<'a> {
    own: &'a [Symbol],
    spec: &'a [Symbol],
    i: usize,
    j: usize,
}

impl<'a> SymbolIter<'a> {
    /// Yields `(is_own, index_in_vec, symbol)` in sorted order.
    pub(crate) fn next_indexed(&mut self) -> Option<(bool, usize, &'a Symbol)> {
        match (self.own.get(self.i), self.spec.get(self.j)) {
            (Some(a), Some(b)) => {
                if a.name <= b.name {
                    let idx = self.i;
                    self.i += 1;
                    Some((true, idx, a))
                } else {
                    let idx = self.j;
                    self.j += 1;
                    Some((false, idx, b))
                }
            }
            (Some(a), None) => {
                let idx = self.i;
                self.i += 1;
                Some((true, idx, a))
            }
            (None, Some(b)) => {
                let idx = self.j;
                self.j += 1;
                Some((false, idx, b))
            }
            (None, None) => None,
        }
    }
}

impl<'a> Iterator for SymbolIter<'a> {
    type Item = &'a Symbol;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_indexed().map(|(_, _, s)| s)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = (self.own.len() - self.i) + (self.spec.len() - self.j);
        (rem, Some(rem))
    }
}

struct SymbolJob {
    key: ResKey,
    h: u64,
    /// The includes it read (their names, in lower case), as they were
    /// as it began: one of them edited since, what it finds is of the
    /// text before ([`ScriptTools::invalidate_dependents`]).
    reads: Vec<String>,
    rx: std::sync::mpsc::Receiver<(Vec<Symbol>, Vec<ResKey>)>,
}

#[derive(Default)]
struct PendingSymbolJob(Option<SymbolJob>);

impl Clone for PendingSymbolJob {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl std::fmt::Debug for PendingSymbolJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PendingSymbolJob").field(&self.0.as_ref().map(|j| (j.key, j.h))).finish()
    }
}

/// What the Reference panel lists (its segmented control's choices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SideTab {
    #[default]
    Functions,
    Variables,
    Constants,
    Templates,
}

/// The script's panels of the Bottom Dock (M4's U13: Compiler and
/// Bookmarks, its own, and the global Search Results; Help, its own too
/// since 5.2.0, is asked forward by [`ScriptTools::help_forward`]): the one
/// a script asked to the front last ([`ScriptTools::info`],
/// [`ScriptTools::ask_forward`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InfoTab {
    #[default]
    Compiler,
    Bookmarks,
    SearchResults,
}

/// A Bottom Dock panel asked to the front ([`ScriptTools::ask_forward`]):
/// by the script `from` (none: a search not of one script's), opening the
/// dock where it is closed if `open` (a search's hits, a compile that
/// failed), else only where it is open. The docks act on it as the next
/// frame begins (`shell::bottom_forward`): the main window's Bottom Dock
/// where `from` is the docked script in front (or for a search, where it
/// does not float), its window's bottom pane where it floats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Forward {
    pub(crate) from: Option<ResKey>,
    pub(crate) tab: InfoTab,
    pub(crate) open: bool,
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
}

/// The most lines Search Results keeps (Find In Files, References): a
/// common word has tens of thousands in a large module's scripts, more
/// than anyone reads through.
pub const MAX_RESULTS: usize = 10_000;

/// What the log says after a count of `found` results where not all are
/// kept ([`MAX_RESULTS`]); nothing where they are.
pub fn results_cut(found: usize) -> String {
    match found > MAX_RESULTS {
        true => format!(": the first {MAX_RESULTS} are in Search Results"),
        false => String::new(),
    }
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
    /// The scripts listed (those of `scripts` that the name typed and
    /// what is shown leave) and their names, and what they were made
    /// for: made again when that changes, not every frame.
    pub(crate) listed: Vec<(ResKey, ScriptFrom)>,
    pub(crate) names: Vec<ResKey>,
    pub(crate) listed_for: Option<(String, ScriptsShown)>,
}

/// The editor tools' state (shared by the open script editors).
#[derive(Debug, Clone, Default)]
pub struct ScriptTools {
    /// The Open Script window, while it is open.
    pub open_script: Option<OpenScript>,
    /// Scripts to open in the external editor once their editor has them
    /// (opened from the module tree, with the option on).
    pub(crate) open_externally: std::collections::HashSet<ResKey>,
    /// What the Reference panel lists, and its filter.
    pub side: SideTab,
    pub filter: String,
    /// The Bottom Dock panel a script asked to the front last.
    pub info: InfoTab,
    /// The asks of this frame, until the docks act on them as the next
    /// frame begins (`shell::bottom_forward`).
    pub(crate) forward: Vec<Forward>,
    /// Counts what the Bottom Dock panels show changing (a compile's
    /// messages, a bookmark, a search's hits): each panel drawn notes the
    /// count it showed and the pass ([`ScriptTools::bottom_drawn`]); one
    /// drawn this pass before the change is drawn again
    /// (`shell::follow_bottom`), and nothing is asked for otherwise.
    pub(crate) made: u64,
    /// The pass a Bottom Dock panel was last drawn in and the least count
    /// ([`ScriptTools::made`]) a panel drawn in it showed.
    pub(crate) bottom_drawn: Option<(u64, u64)>,
    /// The Bottom Dock panel (its keys) to take the keyboard's focus as
    /// it is drawn (F6 to it): from the script's text, so that what is
    /// typed then goes nowhere.
    pub(crate) list_focus: Option<crate::palette_view::Keys>,
    /// The function or constant the Help panel shows.
    pub help: Option<Symbol>,
    /// Text the Reference panel put in (a double click on a name), for the
    /// script's editor to put in at its cursor once it is drawn (the panel
    /// is drawn before it).
    pub(crate) insert: Option<(ResKey, String)>,
    /// Help asked to the front by the script it names (a double click on
    /// a name in its text, its Go to Definition, a click in its Reference's
    /// list): where that script is docked, the Help panel brought to the
    /// front of the Bottom Dock, opened, once a script is the editor in
    /// front (`shell::help_forward`); where it floats, to the front of its
    /// window's bottom pane.
    pub(crate) help_forward: Option<ResKey>,
    /// The Reference panel's filter to take the keyboard's focus as it is
    /// drawn (F6 to it).
    pub(crate) reference_focus: bool,
    /// The Help panel to take the keyboard's focus as it is drawn (F6 to
    /// it): from the script's text, so that what is typed then goes
    /// nowhere rather than into the script.
    pub(crate) help_focus: bool,
    /// The name Reference's list was last scrolled to, or clicked: a name
    /// Help shows that differs (chosen by the cursor in the text, Go to
    /// Definition) scrolls the list to it once.
    pub(crate) list_at: Option<String>,
    /// What the Reference panel showed as it was last drawn: the pass
    /// (`Context::cumulative_pass_nr`), the script, its symbols' list and
    /// the name marked chosen. Drawn before the editor, it is drawn again
    /// where the editor changed them in the same pass, and only then: a
    /// Reference not drawn this pass (behind Help's tab, the dock folded)
    /// asks for nothing (M4's U12 review, A1).
    pub(crate) reference_drawn: Option<(u64, ResKey, Option<SymbolSet>, Option<String>)>,
    /// What the Help panel showed as it was last drawn: the pass and the
    /// name, drawn again where the editor chose another in that pass.
    pub(crate) help_drawn: Option<(u64, Option<String>)>,
    /// What Help shows of the name, laid out once (in the editor's colors)
    /// and kept while it is shown.
    pub(crate) help_read: Option<Arc<crate::script_reference::HelpRead>>,
    pub messages: Vec<Message>,
    pub search: Search,
    pub completion: Option<Completion>,
    /// A cursor move waiting for its editor: script and character index.
    pub jump: Option<(ResKey, usize)>,
    /// An editor to scroll to its cursor once laid out.
    pub scroll: Option<ResKey>,
    /// A script's indentation to convert, to tabs (`true`) or spaces, once
    /// its editor is drawn (Edit › Convert Indentation…); of the lines
    /// selected if the last `bool`, asked while the editor had the keys
    /// (else a selection left from before, not shown, would be taken).
    pub convert_indent: Option<(ResKey, bool, bool)>,
    /// A step of a script's own history to take, back (`false`) or again
    /// (`true`), once its editor is drawn: Edit › Undo or Redo, or the
    /// Top Bar's, while the script is the one worked in
    /// (`script_view::undo_target`).
    pub(crate) undo: Option<(ResKey, bool)>,
    /// A script's editor to give the keys back to once it is drawn (a
    /// menu opened from it closed with nothing chosen).
    pub(crate) keys_back: Option<ResKey>,
    /// The script the Find or Replace window works on, while that window
    /// has the keys: Edit › Undo and Redo are that script's then.
    pub(crate) find_keys: Option<ResKey>,
    /// The script whose editor had the keyboard last.
    pub(crate) last_typed: Option<ResKey>,
    /// `nwscript.nss`, parsed once.
    pub spec: Option<Arc<Spec>>,
    /// Its symbols by name, and the spec they were built from.
    spec_symbols: Option<(Arc<Spec>, Arc<Vec<Symbol>>)>,
    /// Symbols of each script (with its includes), by its text.
    pub symbols: HashMap<ResKey, (u64, SymbolSet)>,
    /// Transitive `#include` keys recorded per script for targeted invalidation.
    symbol_deps: HashMap<ResKey, Vec<ResKey>>,
    /// Cached filtered list for the Reference panel.
    pub(crate) ref_cache: Option<crate::script_reference::RefCache>,
    /// Background symbol extraction job for large scripts.
    symbol_job: PendingSymbolJob,
    /// How many symbol jobs have gone to a worker (for tests: one can come
    /// back between two frames a test looks at).
    pub(crate) symbol_jobs_started: usize,
    /// Each script's text when its editor was last drawn, and the
    /// workspace revision then: an include edited or the module changed
    /// leaves other scripts' symbols and live checks out of date.
    pub(crate) seen: HashMap<ResKey, u64>,
    pub(crate) revision: u64,
    /// The templates listed ([`ScriptTools::templates`]), when (egui's
    /// time) and from which folders they were read.
    #[allow(clippy::type_complexity)]
    templates: Option<(f64, Vec<std::path::PathBuf>, Arc<Vec<(String, std::path::PathBuf)>>)>,
}

/// How long the templates' listing is kept before their folders are read
/// again (a template saved meanwhile shows then): not read every frame.
const TEMPLATES_KEPT: f64 = 2.0;

impl ScriptTools {
    /// The script Help was asked to the front by, while the docks have not
    /// yet brought it ([`ScriptTools::help_forward`]): for tests.
    #[doc(hidden)]
    pub fn help_forward_pending(&self) -> Option<ResKey> {
        self.help_forward
    }

    /// Gives `key`'s editor the keyboard once it is drawn, as a menu
    /// closed with nothing chosen does (for tests).
    #[doc(hidden)]
    pub fn keys_back_for_test(&mut self, key: ResKey) {
        self.keys_back = Some(key);
    }

    /// [`templates`], read at most [`TEMPLATES_KEPT`] seconds ago from the
    /// same folders, else now.
    pub(crate) fn templates(
        &mut self,
        install: Option<&mg_resman::GameInstall>,
        extra: Option<&std::path::Path>,
        now: f64,
    ) -> Arc<Vec<(String, std::path::PathBuf)>> {
        let dirs = template_dirs(install, extra);
        match &self.templates {
            Some((read, from, listed))
                if *from == dirs && (0.0..TEMPLATES_KEPT).contains(&(now - read)) =>
            {
                listed.clone()
            }
            _ => {
                let listed = Arc::new(templates_in(&dirs));
                self.templates = Some((now, dirs, listed.clone()));
                listed
            }
        }
    }
}

/// A script's text as a number, to tell when it changed.
pub(crate) fn hash(text: &str) -> u64 {
    // (egui's hasher: std's took a third of a millisecond a frame on a
    // script of 14,000 lines, this a twentieth.)
    egui::epaint::util::hash(text)
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
    script_symbols_with_deps(text, source).0
}

fn script_symbols_with_deps(
    text: &str,
    source: &mut dyn FnMut(&str) -> Option<String>,
) -> (Vec<Symbol>, Vec<ResKey>) {
    let mut out = Vec::new();
    let mut deps = Vec::new();
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
            if seen.len() < 64 && seen.insert(name.clone()) {
                if let Some(k) = nss(&name) {
                    deps.push(k);
                }
                if let Some(text) = source(&name) {
                    queue.push(text);
                }
            }
        }
    }
    // Prototypes and definitions name a function twice. (The first's,
    // with the documentation of whichever has some: BioWare's includes
    // document a prototype with `//` lines, or only the definition, with a
    // `/* … */` block.)
    // (Found by name and kind in a map: looked through, nwscript.nss's
    // 6,000 symbols took 10 ms.)
    let mut kept = vec![false; out.len()];
    let mut docs = Vec::new();
    {
        let mut first = std::collections::HashMap::with_capacity(out.len());
        let mut documented = std::collections::HashSet::new();
        for (i, s) in out.iter().enumerate() {
            match first.entry((s.name.as_str(), s.kind)) {
                std::collections::hash_map::Entry::Occupied(f) => {
                    let f: usize = *f.get();
                    if out[f].doc.is_empty() && !s.doc.is_empty() && documented.insert(f) {
                        docs.push((f, i));
                    }
                }
                std::collections::hash_map::Entry::Vacant(v) => {
                    v.insert(i);
                    kept[i] = true;
                }
            }
        }
    }
    for (to, from) in docs {
        out[to].doc = std::mem::take(&mut out[from].doc);
    }
    let syms = out.into_iter().zip(kept).filter(|(_, k)| *k).map(|(s, _)| s).collect();
    (syms, deps)
}

const ASYNC_SYMBOL_BYTES: usize = 16_384;

fn collect_includes(
    text: &str,
    source: &mut dyn FnMut(&str) -> Option<String>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    let mut queue = vec![text.to_string()];
    while let Some(src) = queue.pop() {
        if !src.contains("include") {
            continue;
        }
        for inc in mg_script::lex::includes(src.as_bytes()) {
            let Some(name) = src.get(inc.name).map(str::to_ascii_lowercase) else { continue };
            if seen.len() < 64
                && seen.insert(name.clone())
                && let Some(inc_src) = source(&name)
            {
                queue.push(inc_src.clone());
                out.insert(name, inc_src);
            }
        }
    }
    out
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
        self.ask_forward(Some(key), InfoTab::Compiler, false);
    }

    /// The Bottom Dock panel `tab` asked to the front by the script `from`
    /// (none: a search not of one script's), the dock opened where it is
    /// closed if `open` ([`Forward`]); what the panels show changed
    /// ([`ScriptTools::made`]).
    pub(crate) fn ask_forward(&mut self, from: Option<ResKey>, tab: InfoTab, open: bool) {
        self.info = tab;
        self.forward.push(Forward { from, tab, open });
        self.made += 1;
    }

    /// Clears all cached symbol tables, dependency edges, and pending jobs.
    pub(crate) fn clear_symbols(&mut self) {
        self.symbols.clear();
        self.symbol_deps.clear();
        self.ref_cache = None;
        self.symbol_job.0 = None;
    }

    /// Drops cached symbol tables of other scripts that depend on `edited` (or
    /// have no recorded dependency list yet).
    pub(crate) fn invalidate_dependents(&mut self, edited: ResKey) {
        let deps = &self.symbol_deps;
        self.symbols
            .retain(|k, _| *k == edited || deps.get(k).is_some_and(|d| !d.contains(&edited)));
        self.symbol_deps.retain(|k, _| self.symbols.contains_key(k));
        if self.ref_cache.as_ref().is_some_and(|c| !self.symbols.contains_key(&c.key)) {
            self.ref_cache = None;
        }
        // (A worker reading another script's symbols with `edited` as it
        // was among its includes: what it finds is let go, and read again
        // once that script's editor asks. Its result had been kept with
        // the include's old declarations: review D4 of the adversarial
        // branch.)
        let name = edited.resref.to_lowercase().to_string();
        if edited.restype == ResType::NSS
            && self
                .symbol_job
                .0
                .as_ref()
                .is_some_and(|j| j.key != edited && j.reads.contains(&name))
        {
            self.symbol_job.0 = None;
        }
    }

    /// Whether a background symbol extraction job is currently running.
    pub(crate) fn reading_symbols(&self) -> bool {
        self.symbol_job.0.is_some()
    }

    fn ensure_spec_symbols(
        &mut self,
        load_spec: impl FnOnce() -> Option<Spec>,
    ) -> Arc<Vec<Symbol>> {
        if self.spec.is_none() {
            self.spec = Some(Arc::new(load_spec().unwrap_or_default()));
        }
        let spec = self.spec.as_ref().expect("loaded");
        match &self.spec_symbols {
            Some((made, symbols)) if Arc::ptr_eq(made, spec) => symbols.clone(),
            _ => {
                let mut symbols = from_spec(spec);
                symbols.sort_by(|a, b| a.name.cmp(&b.name));
                let symbols = Arc::new(symbols);
                self.spec_symbols = Some((spec.clone(), symbols.clone()));
                symbols
            }
        }
    }

    /// Polls any in-flight background symbol extraction job, applying completed
    /// results and requesting a UI repaint when `repaint` is set.
    pub(crate) fn poll_symbol_job(&mut self, ctx: &egui::Context, repaint: bool) {
        let Some(job) = self.symbol_job.0.as_ref() else { return };
        match job.rx.try_recv() {
            Ok((mut own, deps)) => {
                let (key, h) = (job.key, job.h);
                self.symbol_job.0 = None;
                own.sort_by(|a, b| a.name.cmp(&b.name));
                let spec = self.ensure_spec_symbols(|| None);
                if self.seen.get(&key).is_none_or(|&cur| cur == h) {
                    let set = SymbolSet { own: Arc::new(own), spec };
                    self.symbols.insert(key, (h, set));
                    self.symbol_deps.insert(key, deps);
                    if self.ref_cache.as_ref().is_some_and(|c| c.key == key) {
                        self.ref_cache = None;
                    }
                    if repaint {
                        ctx.request_repaint();
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.symbol_job.0 = None;
            }
        }
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
    ) -> SymbolSet {
        self.symbols_for_inner(key, h, typing, text, load_spec, source, None)
    }

    /// UI variant of [`Self::symbols_for`] that offloads re-parsing huge scripts
    /// (> 16 KiB) after an edit to a background worker thread when `background` is set.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn symbols_for_ui(
        &mut self,
        key: ResKey,
        h: u64,
        typing: bool,
        text: &str,
        load_spec: impl FnOnce() -> Option<Spec>,
        source: &mut dyn FnMut(&str) -> Option<String>,
        ctx: &egui::Context,
        background: bool,
    ) -> SymbolSet {
        self.poll_symbol_job(ctx, true);
        self.symbols_for_inner(key, h, typing, text, load_spec, source, background.then_some(ctx))
    }

    #[allow(clippy::too_many_arguments)]
    fn symbols_for_inner(
        &mut self,
        key: ResKey,
        h: u64,
        typing: bool,
        text: &str,
        load_spec: impl FnOnce() -> Option<Spec>,
        source: &mut dyn FnMut(&str) -> Option<String>,
        ctx: Option<&egui::Context>,
    ) -> SymbolSet {
        let spec_symbols = self.ensure_spec_symbols(load_spec);
        if let Some((cached, set)) = self.symbols.get(&key)
            && (*cached == h || typing)
        {
            return set.clone();
        }
        if let Some(ctx) = ctx
            && text.len() > ASYNC_SYMBOL_BYTES
            && let Some((_, existing)) = self.symbols.get(&key)
        {
            if let Some(job) = self.symbol_job.0.as_ref()
                && job.key == key
                && job.h == h
            {
                return existing.clone();
            }
            let existing = existing.clone();
            let owned_text = text.to_string();
            let includes = collect_includes(text, source);
            let reads = includes.keys().cloned().collect();
            let ctx_bg = ctx.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let res =
                    script_symbols_with_deps(&owned_text, &mut |name| includes.get(name).cloned());
                let _ = tx.send(res);
                ctx_bg.request_repaint();
            });
            self.symbol_job.0 = Some(SymbolJob { key, h, reads, rx });
            self.symbol_jobs_started += 1;
            return existing;
        }
        let (mut own, deps) = script_symbols_with_deps(text, source);
        own.sort_by(|a, b| a.name.cmp(&b.name));
        let set = SymbolSet { own: Arc::new(own), spec: spec_symbols };
        self.symbols.insert(key, (h, set.clone()));
        self.symbol_deps.insert(key, deps);
        set
    }
}

/// The byte index of a character index.
pub fn byte_index(text: &str, chars: usize) -> usize {
    if text.is_ascii() {
        return chars.min(text.len());
    }
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
    if text.is_ascii() {
        return bytes.min(text.len());
    }
    // (A byte inside a character, as a place of another text may be:
    // that character's index.)
    text.char_indices().take_while(|(b, _)| *b < bytes).count()
}

/// The character index where a line (0-based) starts.
pub fn line_start(text: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    let mut n = 0;
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            n += 1;
            if n == line {
                return char_index(text, i + 1);
            }
        }
    }
    if text.is_ascii() { text.len() } else { text.chars().count() }
}

/// The line (0-based) of a character index.
pub fn line_of(text: &str, chars: usize) -> usize {
    let b = byte_index(text, chars);
    text.as_bytes()[..b].iter().filter(|&&c| c == b'\n').count()
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The identifier ending at a character index: its start and text.
pub fn word_before(text: &str, chars: usize) -> (usize, String) {
    // (From the character's byte backwards, as `word_at`: every character
    // before the cursor was copied out, each frame a completion list was
    // open.)
    let at = byte_index(text, chars);
    let before = text[..at].char_indices().rev().find(|(_, c)| !is_ident(*c));
    let start = before.map_or(0, |(i, c)| i + c.len_utf8());
    let word = &text[start..at];
    // (An index past the text's end is its end.)
    let chars = if at == text.len() { chars.min(text.chars().count()) } else { chars };
    (chars - word.chars().count(), word.to_string())
}

/// Whether the characters `from..to` of `text` hold a line end.
pub fn spans_lines(text: &str, from: usize, to: usize) -> bool {
    let (a, b) = (byte_index(text, from.min(to)), byte_index(text, from.max(to)));
    text.as_bytes()[a..b].contains(&b'\n')
}

/// The text of each of `lines` (0-based) of `text`, trimmed, in one pass
/// over it; a line it does not have is empty.
pub fn lines_text(text: &str, lines: impl IntoIterator<Item = usize>) -> HashMap<usize, &str> {
    let mut found: HashMap<usize, &str> = lines.into_iter().map(|l| (l, "")).collect();
    let Some(last) = found.keys().copied().max() else { return found };
    for (i, line) in text.lines().enumerate().take(last.saturating_add(1)) {
        if let Some(text) = found.get_mut(&i) {
            *text = line.trim();
        }
    }
    found
}

/// What a new line begins with, started at character `at`: the spaces and
/// tabs its line begins with (those before `at`), so that the new line is
/// as far in as the one it is started from, as in Aurora.
pub fn new_line_indent(text: &str, at: usize) -> String {
    let b = byte_index(text, at);
    let line_start = text[..b].rfind('\n').map_or(0, |i| i + 1);
    let line = &text[line_start..b];
    let n = line.bytes().take_while(|&c| c == b' ' || c == b'\t').count();
    line[..n].to_string()
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
        let b = byte_index(text, at);
        let line_start = text[..b].rfind('\n').map_or(0, |i| i + 1);
        let line = &text[line_start..b];
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
    let from_b = byte_index(text, from);
    let to_b = byte_index(text, to).max(from_b);
    let start_b = text[..from_b].rfind('\n').map_or(0, |i| i + 1);
    // A selection ending at a line's start leaves that line alone.
    let last_b = if to_b > from_b && text.as_bytes()[to_b - 1] == b'\n' { to_b - 1 } else { to_b };
    let end_b = text[last_b..].find('\n').map_or(text.len(), |i| last_b + i);
    let block = &text[start_b..end_b];
    let mut new_block = String::with_capacity(block.len() + unit.len() * 4);
    for (idx, line) in block.split('\n').enumerate() {
        if idx > 0 {
            new_block.push('\n');
        }
        // (An empty line is left empty: no trailing spaces.)
        if !outdent {
            if !line.is_empty() {
                new_block.push_str(&unit);
                new_block.push_str(line);
            }
        } else if let Some(rest) = line.strip_prefix('\t') {
            new_block.push_str(rest);
        } else {
            let spaces = line.bytes().take(indent.width).take_while(|&b| b == b' ').count();
            new_block.push_str(&line[spaces..]);
        }
    }
    let start_char = char_index(text, start_b);
    let new_block_chars =
        if new_block.is_ascii() { new_block.len() } else { new_block.chars().count() };
    let mut out = String::with_capacity(start_b + new_block.len() + (text.len() - end_b));
    out.push_str(&text[..start_b]);
    out.push_str(&new_block);
    out.push_str(&text[end_b..]);
    (out, start_char, start_char + new_block_chars)
}

/// `text` with each line's indentation (the spaces and tabs it begins
/// with) made tabs or spaces, as Convert Indentation to Tabs and to
/// Spaces do: as far in as before, a tab `width` columns. To tabs, what is
/// left of a step stays spaces (a ` * ` under `/*`, a column lined up).
pub fn convert_indentation(text: &str, tabs: bool, width: usize) -> String {
    let width = width.max(1);
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let rest = line.trim_start_matches([' ', '\t']);
        let lead = &line[..line.len() - rest.len()];
        let columns = lead
            .bytes()
            .fold(0, |col, b| if b == b'\t' { (col / width + 1) * width } else { col + 1 });
        if tabs {
            for _ in 0..(columns / width) {
                out.push('\t');
            }
            for _ in 0..(columns % width) {
                out.push(' ');
            }
        } else {
            for _ in 0..columns {
                out.push(' ');
            }
        }
        out.push_str(rest);
    }
    out
}

/// [`convert_indentation`] of the lines a selection (character indexes
/// `from` to `to`) touches, as Tab indents them (a selection ending at a
/// line's start leaves that line alone); with nothing selected, of every
/// line.
pub fn convert_indentation_in(
    text: &str,
    tabs: bool,
    width: usize,
    from: usize,
    to: usize,
) -> String {
    let (from, to) = (from.min(to), from.max(to));
    if from == to {
        return convert_indentation(text, tabs, width);
    }
    let from_b = byte_index(text, from);
    let to_b = byte_index(text, to).max(from_b);
    let start_b = text[..from_b].rfind('\n').map_or(0, |i| i + 1);
    let last_b = if to_b > from_b && text.as_bytes()[to_b - 1] == b'\n' { to_b - 1 } else { to_b };
    let end_b = text[last_b..].find('\n').map_or(text.len(), |i| last_b + i);
    let converted = convert_indentation(&text[start_b..end_b], tabs, width);
    let mut out = String::with_capacity(start_b + converted.len() + (text.len() - end_b));
    out.push_str(&text[..start_b]);
    out.push_str(&converted);
    out.push_str(&text[end_b..]);
    out
}

/// The identifier around a character index.
pub fn word_at(text: &str, chars: usize) -> Option<String> {
    // (From the character's byte on: a script's characters were all
    // copied out, a frame's millisecond in a script of 14,000 lines.)
    let at = byte_index(text, chars);
    let before = text[..at].char_indices().rev().find(|(_, c)| !is_ident(*c));
    let start = before.map_or(0, |(i, c)| i + c.len_utf8());
    let end = text[at..].find(|c| !is_ident(c)).map_or(text.len(), |i| at + i);
    (end > start).then(|| text[start..end].to_string())
}

/// Where `pat` is found in `hay`, each match's first character: matches
/// don't overlap, from the text's start (as Replace All replaces them).
/// Without Match case, letter by letter with case and accents aside
/// ([`mg_core::text::same_letter`]), so what is found is as long as `pat`.
fn matches(hay: &[char], pat: &[char], o: &SearchOptions) -> Vec<usize> {
    let mut found = Vec::new();
    let Some(last) = hay.len().checked_sub(pat.len()).filter(|_| !pat.is_empty()) else {
        return found;
    };
    let same =
        |a: char, b: char| if o.match_case { a == b } else { mg_core::text::same_letter(a, b) };
    let whole = |s: usize| {
        let before = s == 0 || !is_ident(hay[s - 1]);
        let after = hay.get(s + pat.len()).is_none_or(|&c| !is_ident(c));
        !o.whole_word || (before && after)
    };
    let mut s = 0;
    while s <= last {
        if hay[s..s + pat.len()].iter().zip(pat).all(|(&h, &p)| same(h, p)) && whole(s) {
            found.push(s);
            s += pat.len();
        } else {
            s += 1;
        }
    }
    found
}

fn whole_at(text: &str, start_b: usize, end_b: usize) -> bool {
    let before = text[..start_b].chars().next_back().is_none_or(|c| !is_ident(c));
    let after = text[end_b..].chars().next().is_none_or(|c| !is_ident(c));
    before && after
}

fn byte_matches(text: &str, needle: &str, o: &SearchOptions) -> Option<(Vec<usize>, usize)> {
    if needle.is_empty() {
        return Some((Vec::new(), 0));
    }
    if o.match_case {
        let mut out = Vec::new();
        let mut at = 0;
        while let Some(rel) = text[at..].find(needle) {
            let s = at + rel;
            let e = s + needle.len();
            if !o.whole_word || whole_at(text, s, e) {
                out.push(s);
                at = e;
            } else {
                at = s + text[s..].chars().next().map_or(1, char::len_utf8);
            }
        }
        return Some((out, needle.len()));
    }
    if text.is_ascii() && needle.is_ascii() {
        let hay = text.as_bytes();
        let pat = needle.as_bytes();
        let mut out = Vec::new();
        if let Some(last) = hay.len().checked_sub(pat.len()) {
            let mut s = 0;
            while s <= last {
                if hay[s..s + pat.len()].eq_ignore_ascii_case(pat)
                    && (!o.whole_word || whole_at(text, s, s + pat.len()))
                {
                    out.push(s);
                    s += pat.len();
                } else {
                    s += 1;
                }
            }
        }
        return Some((out, pat.len()));
    }
    None
}

/// The next match of `needle` from a character index (wrapping around),
/// as a character range. Without Match case, letter by letter with case
/// and accents aside ([`mg_core::text::same_letter`]), so what is found
/// is as long as `needle`.
pub fn find(text: &str, needle: &str, from: usize, o: &SearchOptions) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    if let Some((found, _)) = byte_matches(text, needle, o) {
        let from_b = byte_index(text, from);
        let pick = if o.backwards {
            found.iter().rev().find(|b| **b < from_b).or(found.last())
        } else {
            found.iter().find(|b| **b >= from_b).or(found.first())
        };
        let pat_chars = if needle.is_ascii() { needle.len() } else { needle.chars().count() };
        return pick.map(|&b| {
            let s = char_index(text, b);
            (s, s + pat_chars)
        });
    }
    let hay: Vec<char> = text.chars().collect();
    let pat: Vec<char> = needle.chars().collect();
    let matches = matches(&hay, &pat, o);
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
    if start >= end || needle.is_empty() {
        return false;
    }
    let sb = byte_index(text, start);
    let eb = byte_index(text, end);
    if sb >= eb {
        return false;
    }
    if o.whole_word && !whole_at(text, sb, eb) {
        return false;
    }
    let sub = &text[sb..eb];
    if o.match_case {
        return sub == needle;
    }
    let mut sc = sub.chars();
    let mut nc = needle.chars();
    loop {
        match (sc.next(), nc.next()) {
            (Some(a), Some(b)) if mg_core::text::same_letter(a, b) => {}
            (None, None) => return true,
            _ => return false,
        }
    }
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
/// The matches are those of the text as it is ([`matches`]: what Find
/// finds one after another), each replaced once, in one pass: what a
/// replacement leaves is not looked through again. (It was, from the
/// text's start for each match: two spaces for one left about half of a
/// run, and a name used a thousand times took a thousand passes.)
pub fn replace_all(text: &str, needle: &str, with: &str, o: &SearchOptions) -> (String, usize) {
    if needle.is_empty() {
        return (text.to_string(), 0);
    }
    if let Some((found, pat_bytes)) = byte_matches(text, needle, o) {
        if found.is_empty() {
            return (text.to_string(), 0);
        }
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for &b in &found {
            out.push_str(&text[at..b]);
            out.push_str(with);
            at = b + pat_bytes;
        }
        out.push_str(&text[at..]);
        return (out, found.len());
    }
    let hay: Vec<char> = text.chars().collect();
    let pat: Vec<char> = needle.chars().collect();
    let found = matches(&hay, &pat, o);
    if found.is_empty() {
        return (text.to_string(), 0);
    }
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for &s in &found {
        out.extend(&hay[at..s]);
        out.push_str(with);
        at = s + pat.len();
    }
    out.extend(&hay[at..]);
    (out, found.len())
}

/// Completion candidates: symbols starting with the prefix (any case),
/// custom ones first, then by name.
pub fn completions(symbols: &[Symbol], prefix: &str) -> Vec<Symbol> {
    completions_from(symbols.iter(), prefix)
}

fn completions_from<'a>(symbols: impl Iterator<Item = &'a Symbol>, prefix: &str) -> Vec<Symbol> {
    let p = prefix.to_ascii_lowercase();
    let mut out: Vec<Symbol> =
        symbols.filter(|s| s.name.to_ascii_lowercase().starts_with(&p)).cloned().collect();
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
    templates_in(&template_dirs(install, extra))
}

/// The folders [`templates`] reads.
fn template_dirs(
    install: Option<&mg_resman::GameInstall>,
    extra: Option<&std::path::Path>,
) -> Vec<std::path::PathBuf> {
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
    dirs
}

/// The templates in `dirs`, by name.
fn templates_in(dirs: &[std::path::PathBuf]) -> Vec<(String, std::path::PathBuf)> {
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

    /// With text selected, the lines it touches alone (from anywhere in
    /// the first; one it ends at the start of is left), either way round.
    #[test]
    fn a_selection_s_lines_convert_alone() {
        let text = "  a\n  b\n  c\n  d\n";
        let at = |s: &str| text.find(s).unwrap();
        let converted = super::convert_indentation_in(text, true, 2, at("b"), at("  d"));
        assert_eq!(converted, "  a\n\tb\n\tc\n  d\n");
        assert_eq!(super::convert_indentation_in(text, true, 2, at("  d"), at("b")), converted);
        assert_eq!(super::convert_indentation_in(text, true, 2, 3, 3), "\ta\n\tb\n\tc\n\td\n");
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
        assert_eq!(word_at("int nCount;", 4).as_deref(), Some("nCount"));
        assert_eq!(word_at("int nCount;", 10).as_deref(), Some("nCount"));
        assert_eq!(word_at("int nCount;", 11), None);
        assert_eq!(word_at("nA", 0).as_deref(), Some("nA"));
        assert_eq!(word_at("nA", 2).as_deref(), Some("nA"));
        assert_eq!(word_at("nA", 9).as_deref(), Some("nA"), "past the end: the last word");
        assert_eq!(
            word_at("// é sA = ü;", 6).as_deref(),
            Some("sA"),
            "after a letter of two bytes"
        );
        assert_eq!(word_at("é", 1), None);
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

    /// The word before the cursor, the lines a selection spans and the
    /// bookmarked lines' texts are read where they are, not through a copy
    /// of the text's characters or a walk from its top for each: as the
    /// plain way has them.
    #[test]
    fn the_cursor_s_word_a_selection_s_lines_and_marked_lines_texts() {
        let text = "int n\u{e9}e = GetLo\n  caf\u{e9}_x;\n\nlast";
        let plain = |chars: usize| {
            let before: Vec<char> = text.chars().take(chars).collect();
            let start = before.iter().rposition(|c| !is_ident(*c)).map_or(0, |p| p + 1);
            (start, before[start..].iter().collect::<String>())
        };
        for chars in 0..text.chars().count() + 3 {
            assert_eq!(word_before(text, chars), plain(chars), "at {chars}");
        }
        assert_eq!(word_before(text, 15), (10, "GetLo".into()));
        assert_eq!(word_before("", 4), (0, String::new()));
        for (from, to) in [(0, 3), (0, 15), (0, 16), (15, 16), (16, 15), (16, 16), (30, 99)] {
            let (a, b) = (from.min(to), from.max(to));
            let plain = text.chars().skip(a).take(b - a).any(|c| c == '\n');
            assert_eq!(spans_lines(text, from, to), plain, "{from}..{to}");
        }
        let texts = lines_text(text, [1, 3, 0, 9]);
        assert_eq!(texts[&0], "int n\u{e9}e = GetLo");
        assert_eq!((texts[&1], texts[&3], texts[&9]), ("caf\u{e9}_x;", "last", ""));
        assert!(lines_text(text, []).is_empty());
    }

    /// A selection kept from a longer text (the external editor emptied
    /// the file in the frame a conversion was pending) converts the line
    /// it is held to, and nothing of an empty text: it read the character
    /// before the text's start.
    #[test]
    fn convert_indentation_of_a_selection_past_the_text_s_end() {
        assert_eq!(convert_indentation_in("", true, 4, 3, 9), "");
        assert_eq!(convert_indentation_in("    a\n    b", true, 4, 30, 90), "    a\n\tb");
        assert_eq!(convert_indentation_in("    a\n    b\n", true, 4, 30, 90), "    a\n    b\n");
        assert_eq!(convert_indentation_in("    a\n    b\n", true, 4, 2, 6), "\ta\n    b\n");
    }

    /// Replace All replaces the matches the text has, each once: a needle
    /// that overlaps itself is not missed in what a replacement leaves
    /// (two spaces for one left three of four), and a text of many
    /// matches is gone through once, not once a match.
    #[test]
    fn replace_all_replaces_each_match_of_the_text_once() {
        let o = SearchOptions::default();
        assert_eq!(replace_all("    ", "  ", " ", &o), ("  ".into(), 2));
        assert_eq!(replace_all("a\n\n\n\n\n\nb", "\n\n\n", "\n\n", &o), ("a\n\n\n\nb".into(), 2));
        assert_eq!(replace_all("aaa", "a", "aa", &o), ("aaaaaa".into(), 3));
        assert_eq!(replace_all("abc", "", "x", &o), ("abc".into(), 0));
        assert_eq!(replace_all("ab", "abc", "x", &o), ("ab".into(), 0));
        // A megabyte with 5,000 matches (each pass copied the text).
        let line = format!("    int nValue = GetLocalInt(oPC, \"{}\");\n", "x".repeat(180));
        let text = line.repeat(5_000);
        assert!(text.len() > 1_000_000);
        let started = std::time::Instant::now();
        let (out, n) = replace_all(&text, "GetLocalInt", "GetInt", &o);
        assert_eq!(n, 5_000);
        assert_eq!(out, text.replace("GetLocalInt", "GetInt"));
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "{:?}", started.elapsed());
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

    /// A script's symbols come sorted by name, as the side lists show them
    /// (not sorted every frame): of one name, the script's own first.
    #[test]
    fn a_script_s_symbols_are_kept_by_name() {
        let key = super::nss("hello").unwrap();
        let mut tools = super::ScriptTools::default();
        let spec = "int TRUE = 1;\nint Add(int a);\nvoid Zap();\n";
        let text = "int Add(int a) { return a; }\nvoid Bar() {}\nvoid main() {}\n";
        let symbols = tools.symbols_for(
            key,
            1,
            false,
            text,
            || Some(mg_script::spec::Spec::parse(spec.as_bytes())),
            &mut |_| None,
        );
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert!(names.contains(&"Zap") && names.contains(&"Bar"), "{names:?}");
        let adds: Vec<bool> =
            symbols.iter().filter(|s| s.name == "Add").map(|s| s.custom).collect();
        assert_eq!(adds, [true, false], "the script's own first");
    }

    /// A long script's symbols read on a worker (`symbols_for_ui`) with an
    /// include as it was: the include edited meanwhile, what the worker
    /// found is let go, and the script's symbols are the edited include's.
    /// They had been kept with its old declarations (review D4).
    #[test]
    fn a_worker_s_symbols_go_when_an_include_it_read_is_edited() {
        let (main, inc) = (super::nss("big").unwrap(), super::nss("inc_a").unwrap());
        let ctx = egui::Context::default();
        let mut tools = super::ScriptTools::default();
        let filler: String = (0..1200).map(|i| format!("int nFill{i} = {i};\n")).collect();
        let text = format!("#include \"inc_a\"\n{filler}void main() {{}}\n");
        assert!(text.len() > super::ASYNC_SYMBOL_BYTES);
        let include = |name: &'static str| {
            move |n: &str| (n == "inc_a").then(|| format!("void {name}();\nvoid {name}() {{}}\n"))
        };
        let names = |s: &super::SymbolSet| -> Vec<String> {
            s.own.iter().map(|s| s.name.clone()).filter(|n| n.ends_with("Name")).collect()
        };
        let first = tools.symbols_for_ui(
            main,
            1,
            false,
            &text,
            || None,
            &mut include("OldName"),
            &ctx,
            true,
        );
        assert_eq!(names(&first), ["OldName"]);
        // Typed in: read on a worker, the include as it is now.
        let typed = format!("{text}// typed\n");
        tools.symbols_for_ui(main, 2, false, &typed, || None, &mut include("OldName"), &ctx, true);
        assert!(tools.reading_symbols(), "read on a worker");
        // The include edited in its own editor before the worker is done.
        tools.seen.insert(inc, 7);
        tools.invalidate_dependents(inc);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while tools.reading_symbols() && std::time::Instant::now() < deadline {
            tools.poll_symbol_job(&ctx, false);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let now = tools.symbols_for_ui(
            main,
            2,
            false,
            &typed,
            || None,
            &mut include("NewName"),
            &ctx,
            true,
        );
        assert_eq!(names(&now), ["NewName"], "the edited include's");
    }

    /// The templates' folders are read again a couple of seconds on, or
    /// when another folder is chosen, not every frame.
    #[test]
    fn the_templates_are_read_again_after_a_while() {
        let dir = mg_testkit::scratch_dir("ui-script-templates");
        let other = dir.join("other");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(dir.join("first.txt"), "// first").unwrap();
        let mut tools = super::ScriptTools::default();
        let names = |listed: &[(String, std::path::PathBuf)]| -> Vec<String> {
            listed.iter().map(|(n, _)| n.clone()).collect()
        };
        assert_eq!(names(&tools.templates(None, Some(&dir), 10.0)), ["first"]);
        std::fs::write(dir.join("second.txt"), "// second").unwrap();
        let kept = tools.templates(None, Some(&dir), 11.0);
        assert_eq!(names(&kept), ["first"], "kept a while");
        assert!(names(&tools.templates(None, Some(&other), 11.0)).is_empty(), "another folder");
        assert_eq!(names(&tools.templates(None, Some(&dir), 11.5)), ["first", "second"]);
        let read = tools.templates(None, Some(&dir), 11.6);
        assert!(std::sync::Arc::ptr_eq(&read, &tools.templates(None, Some(&dir), 13.0)));
        assert_eq!(names(&tools.templates(None, Some(&dir), 14.0)), ["first", "second"]);
    }
}
