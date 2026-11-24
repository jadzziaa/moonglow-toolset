//! The script editor's code navigation, on `mg_script::analysis`: Go to
//! Definition (F12, Ctrl+click), References (Shift+F12), Rename Symbol
//! (Ctrl+Shift+R), and errors as you type (the script compiled a moment
//! after typing stops, its error line underlined).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_resman::ResKey;
use mg_script::Compiler;
use mg_script::analysis::{Declaration, Index, NWSCRIPT};

use crate::script_tools as tools;
use crate::script_view::ScriptBuffer;
use crate::text::{decode_script as decode, encode_script as encode};
use crate::{Action, Moonglow, Tab};

/// How long typing must pause before the script is checked.
const PAUSE: Duration = Duration::from_millis(500);

/// A script's latest check.
#[derive(Debug, Clone, Default)]
pub(crate) struct Live {
    text: u64,
    changed: Option<Instant>,
    checked: Option<u64>,
    /// The error: its 0-based line in this script (`None`: in an include)
    /// and message.
    pub(crate) error: Option<(Option<usize>, String)>,
    /// Where an error in an include is: the include's name (as the
    /// compiler gives it, without `.nss`) and its line (from 1).
    pub(crate) include: Option<(String, usize)>,
    /// How many entries the log had when the error was found (a warning
    /// logged since takes its place in the Status Bar).
    pub(crate) logged: usize,
}

/// The navigation state.
#[derive(Debug, Default)]
pub struct Nav {
    index: Index,
    /// The workspace revision the index was built for.
    revision: Option<u64>,
    /// The buffer texts the index has.
    texts: HashMap<ResKey, u64>,
    pub(crate) live: HashMap<ResKey, Live>,
    /// Generation bumped whenever module revision or another script invalidates live checks.
    live_gen: u64,
    /// In-flight background compilation checks keyed by script (`(text_hash, live_gen, rx)`).
    pending_checks:
        HashMap<ResKey, (u64, u64, std::sync::mpsc::Receiver<Option<mg_script::CompileError>>)>,
    /// How many checks have gone to a worker (for tests: one can come back
    /// between two frames a test looks at).
    checks_started: usize,
    /// The Rename Symbol window: the declaration and the new name.
    pub rename: Option<(Declaration, String)>,
    /// The script a declaration was last looked for in (`declaration_at`):
    /// the one Go to Definition was asked from, whose window a game
    /// script opens in.
    asked_from: Option<ResKey>,
}

/// Collects saved module overrides of all `#include` files transitively
/// reachable from `root_source` (plus module `nwscript.nss`, if overridden).
fn collect_module_includes(
    module: Option<&mg_module::Module>,
    resman: Option<&mg_resman::ResMan>,
    root_name: &str,
    root_source: &[u8],
) -> HashMap<String, Vec<u8>> {
    let mut module_files = HashMap::new();
    let Some(module) = module else { return module_files };
    if let Some(nw) = ResKey::parse("nwscript", ResType::NSS).and_then(|k| module.get(&k)) {
        module_files.insert("nwscript".to_string(), nw.to_vec());
    }
    let mut seen = std::collections::HashSet::new();
    seen.insert(root_name.to_ascii_lowercase());
    let mut queue = vec![root_source.to_vec()];
    while let Some(src) = queue.pop() {
        if !src.windows(7).any(|w| w == b"include") {
            continue;
        }
        for inc in mg_script::lex::includes(&src) {
            let Some(slice) = src.get(inc.name) else { continue };
            let Ok(raw) = std::str::from_utf8(slice) else { continue };
            let low = raw.to_ascii_lowercase();
            if seen.len() >= 128 || !seen.insert(low.clone()) {
                continue;
            }
            let Some(k) = ResKey::parse(&low, ResType::NSS) else { continue };
            if let Some(own) = module.get(&k) {
                let bytes = own.to_vec();
                queue.push(bytes.clone());
                module_files.insert(low, bytes);
            } else if let Some(raw) = resman.and_then(|r| r.get(&k).ok()) {
                queue.push(raw.into_owned());
            }
        }
    }
    module_files
}

/// Scripts by name: editors' text, then the module's, then the game's.
fn source<'a>(
    scripts: &'a HashMap<ResKey, ScriptBuffer>,
    ws: Option<&'a mg_edit::Workspace>,
    game: Option<&'a mg_rules::GameData>,
) -> impl FnMut(&str) -> Option<Arc<str>> + 'a {
    move |name: &str| {
        let text = crate::script_view::script_source(scripts, ws, game, &tools::nss(name)?)?;
        Some(Arc::from(text.as_str()))
    }
}

/// `text` with `new` at each of `spans` (byte ranges, in order), each of
/// which holds `old`; `None` where one does not (it overlaps the one
/// before, lies past the text's end or inside a character, or holds
/// something else): the spans are another text's.
fn renamed(text: &str, spans: &[std::ops::Range<usize>], old: &str, new: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for s in spans {
        if text.get(s.clone()) != Some(old) {
            return None;
        }
        out.push_str(text.get(at..s.start)?);
        out.push_str(new);
        at = s.end;
    }
    out.push_str(text.get(at..)?);
    Some(out)
}

impl Nav {
    /// Every script's live check but `but`'s to be made again when next
    /// shown: an include of theirs may have changed.
    pub(crate) fn check_again(&mut self, but: Option<ResKey>) {
        self.live_gen = self.live_gen.wrapping_add(1);
        for (k, live) in &mut self.live {
            if Some(*k) != but {
                live.checked = None;
            }
        }
        if let Some(keep) = but {
            self.pending_checks.retain(|k, _| *k == keep);
        } else {
            self.pending_checks.clear();
        }
    }

    /// Whether any background live-check compile is still running.
    /// How many checks have gone to a worker.
    pub(crate) fn checks_started(&self) -> usize {
        self.checks_started
    }

    pub(crate) fn checking(&self) -> bool {
        !self.pending_checks.is_empty()
    }
}

impl Moonglow {
    /// Brings the index up to date: the module changed (everything is read
    /// again), or an editor's text did.
    fn sync_nav(&mut self) {
        let Some(ws) = &self.ws else { return };
        if self.script_nav.revision != Some(ws.revision()) {
            self.script_nav.index = Index::new();
            self.script_nav.texts.clear();
            self.script_nav.revision = Some(ws.revision());
        }
        // An editor closed without saving (Cancel), or its script renamed
        // or gone: the index had its text, places in which are no places
        // in the module's. Read again from the module when next needed.
        let (scripts, nav) = (&self.scripts, &mut self.script_nav);
        nav.texts.retain(|k, _| {
            let open = scripts.contains_key(k);
            if !open {
                nav.index.forget(&k.resref.to_string());
            }
            open
        });
        for (k, b) in &mut self.scripts {
            // (An editor behind another tab is not drawn, and follows the
            // module only when it is: a script the NUI Creator, a plugin
            // or Undo wrote meanwhile is read here as the module has it.)
            if let Some(bytes) = ws.module.get(k) {
                b.follow_module(bytes, crate::tree::stored_stamp(ws, k, bytes));
            }
            let h = crate::script_tools::hash(&b.text);
            if self.script_nav.texts.get(k) != Some(&h) {
                self.script_nav.index.set(&k.resref.to_string(), Arc::from(b.text.as_str()));
                self.script_nav.texts.insert(*k, h);
            }
        }
    }

    /// What the name at character `at` of a script declares.
    pub fn declaration_at(&mut self, key: ResKey, at: usize) -> Option<Declaration> {
        self.sync_nav();
        self.script_nav.asked_from = Some(key);
        let text = self.scripts.get(&key)?.text.clone();
        let offset = tools::byte_index(&text, at);
        let Moonglow { scripts, ws, game, script_nav, .. } = self;
        let mut src = source(scripts, ws.as_ref(), game.as_deref());
        script_nav.index.declaration_at(&key.resref.to_string(), offset, &mut src)
    }

    /// Opens a declaration: in its script's editor at the name, or (a game
    /// script, `nwscript.nss`) read-only, with its help shown.
    pub fn go_to_declaration(&mut self, d: &Declaration) {
        let Some(k) = tools::nss(&d.at.file) else { return };
        self.script_tools.help = Some(tools::Symbol {
            name: d.name.clone(),
            kind: tools::SymbolKind::Function,
            signature: d.signature.clone(),
            doc: d.doc_source.clone(),
            custom: d.at.file != NWSCRIPT,
        });
        if self.ws.as_ref().is_some_and(|w| w.module.contains(&k)) {
            let text = self.script_source(&k).unwrap_or_default();
            self.script_tools.jump = Some((k, tools::char_index(&text, d.at.span.start)));
            self.actions.push(Action::OpenTab(Tab::Script(k)));
        } else {
            // A game script, read-only: at the declaration's line, marked
            // (its text as the read-only view has it, with plain newlines).
            let text = self.script_source(&k).unwrap_or_default();
            let start = d.at.span.start.min(text.len());
            let line = mg_core::text::line_of(text.as_bytes(), start).saturating_sub(1);
            self.browser.go_to_line(k, None, line);
            self.open_definition(Tab::Resource(k));
        }
    }

    /// Where a game script's declaration opens (the owner's ruling of
    /// 2026-11-23): already open, its tab brought to the front; else, as
    /// Floating Windows, a tab of the window (or the group, the script
    /// docked) of the script it was asked from (`Nav::asked_from`), not
    /// a window of its own; as Tabs, where a read-only tab opens
    /// (`Action::OpenTab`, the main window). The one place that decides
    /// it.
    fn open_definition(&mut self, tab: Tab) {
        let floating = self.settings.open_editors == crate::settings::OpenEditors::Floating;
        let from = self.script_nav.asked_from.map(Tab::Script);
        let group = from.and_then(|t| self.dock.find_tab(&t));
        if floating
            && self.dock.find_tab(&tab).is_none()
            && let Some(at) = group
        {
            let pane = egui_dock::NodePath { surface: at.surface, node: at.node };
            self.dock.set_focused_node_and_surface(pane);
            self.dock.push_to_focused_leaf(tab.clone());
        }
        // (Open now, it is brought to the front; else opened.)
        self.actions.push(Action::OpenTab(tab));
    }

    /// The module's scripts (the files references are looked for in).
    fn module_scripts(&self) -> Vec<String> {
        self.ws
            .as_ref()
            .map(|w| w.module.keys_of(ResType::NSS).map(|k| k.resref.to_string()).collect())
            .unwrap_or_default()
    }

    /// Lists where a declaration is used in Search Results, brought to
    /// the front (the Bottom Dock, or the window's bottom pane of the
    /// script `from`, which it was run from, where it floats).
    pub fn show_references(&mut self, d: &Declaration, from: Option<ResKey>) {
        self.sync_nav();
        let files = self.module_scripts();
        let mut hits = Vec::new();
        {
            let Moonglow { scripts, ws, game, script_nav, .. } = self;
            let mut src = source(scripts, ws.as_ref(), game.as_deref());
            let refs = script_nav.index.references(d, &files, &mut src);
            for r in &refs {
                let (Some(k), Some(text)) = (tools::nss(&r.file), src(&r.file)) else { continue };
                if text.get(..r.span.start).is_none() {
                    continue;
                }
                let line = mg_core::text::line_of(text.as_bytes(), r.span.start) - 1;
                let shown = text.lines().nth(line).unwrap_or("").trim().to_string();
                hits.push(crate::search_results::Hit::Line { script: k, line, text: shown });
            }
        }
        let n = hits.len();
        hits.truncate(tools::MAX_RESULTS);
        let cut = tools::results_cut(n);
        let what = format!("{}: {n} reference(s) in the module{cut}", d.name);
        self.log.info(what.clone());
        self.show_search_results(what, hits, from);
    }

    /// Renames a declaration everywhere it's used in the module's scripts,
    /// as one undoable command (compiled scripts stay valid: names aren't
    /// in them).
    pub fn rename_symbol(&mut self, d: &Declaration, new: &str) {
        self.store_script_text();
        self.sync_nav();
        let files = self.module_scripts();
        let places = {
            let Moonglow { scripts, ws, game, script_nav, .. } = self;
            let mut src = source(scripts, ws.as_ref(), game.as_deref());
            script_nav.index.rename(d, new, &files, &mut src)
        };
        let places = match places {
            Ok(p) => p,
            Err(e) => {
                self.log.error(format!("Rename {}: {e}", d.name));
                return;
            }
        };
        let Some(ws) = &self.ws else { return };
        let mut by_file: HashMap<String, Vec<std::ops::Range<usize>>> = HashMap::new();
        for p in &places {
            by_file.entry(p.file.clone()).or_default().push(p.span.clone());
        }
        let mut edits = Vec::new();
        for (file, mut spans) in by_file {
            let Some(k) = tools::nss(&file) else { continue };
            let Some(raw) = ws.module.get(&k).map(decode) else { continue };
            // (The places are in the text as the editors have it, with
            // plain newlines: written back with the file's line ends.)
            let text = crate::text::to_editor(&raw).0;
            spans.sort_by_key(|s| s.start);
            // (A place that does not hold the old name is a place in
            // another text than the module's: nothing is written.)
            let Some(out) = renamed(&text, &spans, &d.name, new) else {
                self.log.error(format!(
                    "Rename {}: {file} is not as it was read: nothing was renamed, try again",
                    d.name
                ));
                return;
            };
            let out = crate::text::with_line_ends(&out, &raw);
            edits.push(Edit::SetResource { key: k, data: Some(encode(&out)) });
        }
        let n = edits.len();
        self.actions
            .push(Action::Apply(Command::new(format!("Rename {} to {new}", d.name), edits)));
        self.log.info(format!(
            "Renamed {} to {new}: {} place(s) in {n} script(s)",
            d.name,
            places.len()
        ));
    }
}

/// Compiles `name` (an include file needs no `main`) against saved module overrides
/// and loaded game resources for live error checking.
fn compile_live_check(
    name: &str,
    source: Vec<u8>,
    module_files: HashMap<String, Vec<u8>>,
    game: Option<Arc<mg_rules::GameData>>,
) -> Option<mg_script::CompileError> {
    let root = name.to_owned();
    let mut c = Compiler::new(move |n: &str, t: ResType| -> Option<Vec<u8>> {
        if t == ResType::NSS && n.eq_ignore_ascii_case(&root) {
            return Some(source.clone());
        }
        let k = ResKey::parse(n, t)?;
        if t == ResType::NSS
            && let Some(bytes) = module_files.get(&k.resref.to_lowercase().to_string())
        {
            return Some(bytes.clone());
        }
        game.as_deref()?.resman.get(&k).ok().map(|d| d.into_owned())
    });
    c.set_require_entry_point(false);
    c.compile(name).err()
}

impl Moonglow {
    fn apply_live_check(
        &mut self,
        key: ResKey,
        h: u64,
        error: Option<mg_script::CompileError>,
        ctx: &egui::Context,
        repaint_if_changed: bool,
    ) {
        let name = key.resref.to_lowercase().to_string();
        let logged = self.log.entries.len();
        let live = self.script_nav.live.entry(key).or_default();
        if live.text != h {
            return;
        }
        live.checked = Some(h);
        if error.as_ref().map(|e| &e.message) != live.error.as_ref().map(|e| &e.1) {
            live.logged = logged;
        }
        let new_include = error
            .as_ref()
            .and_then(|e| e.location())
            .filter(|(file, _)| !file.trim_end_matches(".nss").eq_ignore_ascii_case(&name))
            .map(|(file, line)| (file.trim_end_matches(".nss").to_string(), line));
        let new_error = error.map(|e| {
            let line = e
                .location()
                .filter(|(file, _)| file.trim_end_matches(".nss").eq_ignore_ascii_case(&name))
                .map(|(_, line)| line.saturating_sub(1));
            (line, e.message)
        });
        let changed = new_error != live.error || new_include != live.include;
        live.include = new_include;
        live.error = new_error;
        if repaint_if_changed && changed {
            ctx.request_repaint();
        }
    }

    /// Polls background live-check compilation results and applies the first
    /// error (if any) back onto [`Nav::live`].
    pub(crate) fn poll_live_check(&mut self, ctx: &egui::Context, repaint_if_changed: bool) {
        if self.script_nav.pending_checks.is_empty() {
            return;
        }
        let keys: Vec<ResKey> = self.script_nav.pending_checks.keys().copied().collect();
        let cur_gen = self.script_nav.live_gen;
        for key in keys {
            let recv = {
                let Some((_, _, rx)) = self.script_nav.pending_checks.get(&key) else { continue };
                rx.try_recv()
            };
            match recv {
                Ok(error) => {
                    let Some((h, job_gen, _)) = self.script_nav.pending_checks.remove(&key) else {
                        continue;
                    };
                    if job_gen != cur_gen {
                        continue;
                    }
                    self.apply_live_check(key, h, error, ctx, repaint_if_changed);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.script_nav.pending_checks.remove(&key);
                }
            }
        }
    }

    /// Checks a script a moment after typing stops: compiles its text (an
    /// include file needs no `main`) on a worker thread when [`Self::background_jobs`]
    /// is enabled (inline otherwise) and keeps the first error.
    pub(crate) fn live_check(&mut self, key: ResKey, h: u64, ctx: &egui::Context) {
        self.poll_live_check(ctx, true);
        let live = self.script_nav.live.entry(key).or_default();
        if live.text != h {
            // A script just opened is checked at once; edits wait for a pause.
            let first = live.checked.is_none() && live.changed.is_none();
            live.text = h;
            live.changed = (!first).then(Instant::now);
        }
        if live.checked == Some(h) {
            return;
        }
        let waited = live.changed.map_or(PAUSE, |t| t.elapsed());
        // (The first look at a script has no pause: `changed` is unset.)
        if waited < PAUSE {
            self.frame_after(ctx, PAUSE - waited);
            return;
        }
        let cur_gen = self.script_nav.live_gen;
        if self
            .script_nav
            .pending_checks
            .get(&key)
            .is_some_and(|&(pending_h, pending_gen, _)| pending_h == h && pending_gen == cur_gen)
        {
            return;
        }
        let name = key.resref.to_lowercase().to_string();
        // (Its includes as Compile reads them: the module's, not their
        // editors' unsaved text.)
        let source = self.scripts.get(&key).map(ScriptBuffer::bytes).unwrap_or_default();
        let module = self.ws.as_ref().map(|w| &w.module);
        let resman = self.game.as_deref().map(|g| &g.resman);
        let module_files = collect_module_includes(module, resman, &name, &source);
        let game = self.game.clone();
        if !self.background_jobs {
            let error = compile_live_check(&name, source, module_files, game);
            self.apply_live_check(key, h, error, ctx, true);
            return;
        }
        let ctx_bg = ctx.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(compile_live_check(&name, source, module_files, game));
            ctx_bg.request_repaint();
        });
        self.script_nav.pending_checks.insert(key, (h, cur_gen, rx));
        self.script_nav.checks_started += 1;
    }

    /// Sets an open script editor's text, as typing would (for tests).
    #[doc(hidden)]
    pub fn scripts_mut_for_test(&mut self, key: ResKey, text: &str) {
        if let Some(b) = self.scripts.get_mut(&key) {
            b.set_text(text.to_string());
        }
    }

    /// The error a script's latest check found (for tests): its 0-based
    /// line, if in the script, and message.
    #[doc(hidden)]
    pub fn script_error_for_test(&self, key: ResKey) -> Option<(Option<usize>, String)> {
        self.live_error(key).cloned()
    }

    /// Where the error a script's latest check found is, where it is in
    /// an include: the include's name and its line (from 1).
    pub(crate) fn live_include(&self, key: ResKey) -> Option<&(String, usize)> {
        self.live_error(key)?;
        self.script_nav.live.get(&key)?.include.as_ref()
    }

    /// The error a script's latest check found; none while checking as
    /// you type is off (Options › Script Editor).
    pub(crate) fn live_error(&self, key: ResKey) -> Option<&(Option<usize>, String)> {
        if self.settings.no_live_check {
            return None;
        }
        self.script_nav.live.get(&key)?.error.as_ref()
    }
}

/// The Rename Symbol window.
pub(crate) fn rename_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some((decl, mut name)) = app.script_nav.rename.take() else { return };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    let footed = crate::widgets::dialog(ctx, &format!("Rename {}", decl.name), &mut open)
        .collapsible(false)
        .resizable(false)
        .show_foot(ctx, |ui| {
            ui.label(format!("{} — every use in the module's scripts", decl.signature));
            ui.horizontal(|ui| {
                ui.label("New name");
                let r = ui.add(mg_uikit::text_field(&mut name).desired_width(220.0));
                crate::widgets::autofocus(ui, &r);
            });
            let can = !name.trim().is_empty() && name.trim() != decl.name;
            ((), crate::widgets::Foot::new().main("Rename", can).cancel())
        });
    if let Some(footed) = footed {
        ok = footed.pressed.main();
        cancel = footed.pressed.cancelled();
    }
    if ok {
        let new = name.trim().to_string();
        app.rename_symbol(&decl, &new);
    } else if open && !cancel {
        app.script_nav.rename = Some((decl, name));
    }
}
