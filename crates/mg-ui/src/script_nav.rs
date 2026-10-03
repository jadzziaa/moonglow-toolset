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

use crate::script_tools::{self as tools, InfoTab};
use crate::script_view::ScriptBuffer;
use crate::text::{decode, encode};
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
    /// The Rename Symbol window: the declaration and the new name.
    pub rename: Option<(Declaration, String)>,
}

fn hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// Scripts by name: editors' text, then the module's, then the game's.
fn source<'a>(
    scripts: &'a HashMap<ResKey, ScriptBuffer>,
    ws: Option<&'a mg_edit::Workspace>,
    game: Option<&'a mg_rules::GameData>,
) -> impl FnMut(&str) -> Option<Arc<str>> + 'a {
    move |name: &str| {
        let k = tools::nss(name)?;
        if let Some(b) = scripts.get(&k) {
            return Some(Arc::from(b.text.as_str()));
        }
        let text = ws
            .and_then(|w| w.module.get(&k).map(decode))
            .or_else(|| game?.resman.get(&k).ok().map(|d| decode(&d)))?;
        Some(Arc::from(text.as_str()))
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
        for (k, b) in &self.scripts {
            let h = hash(&b.text);
            if self.script_nav.texts.get(k) != Some(&h) {
                self.script_nav.index.set(&k.resref.to_string(), Arc::from(b.text.as_str()));
                self.script_nav.texts.insert(*k, h);
            }
        }
    }

    /// What the name at character `at` of a script declares.
    pub fn declaration_at(&mut self, key: ResKey, at: usize) -> Option<Declaration> {
        self.sync_nav();
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
            doc: d.doc.clone(),
            custom: d.at.file != NWSCRIPT,
        });
        self.script_tools.info = InfoTab::Help;
        if self.ws.as_ref().is_some_and(|w| w.module.contains(&k)) {
            let text = self
                .scripts
                .get(&k)
                .map(|b| b.text.clone())
                .or_else(|| self.ws.as_ref()?.module.get(&k).map(decode))
                .unwrap_or_default();
            self.script_tools.jump = Some((k, tools::char_index(&text, d.at.span.start)));
            self.actions.push(Action::OpenTab(Tab::Script(k)));
        } else {
            self.actions.push(Action::OpenTab(Tab::Resource(k)));
        }
    }

    /// The module's scripts (the files references are looked for in).
    fn module_scripts(&self) -> Vec<String> {
        self.ws
            .as_ref()
            .map(|w| w.module.keys_of(ResType::NSS).map(|k| k.resref.to_string()).collect())
            .unwrap_or_default()
    }

    /// Lists where a declaration is used in the Search Results tab.
    pub fn show_references(&mut self, d: &Declaration) {
        self.sync_nav();
        let files = self.module_scripts();
        let Moonglow { scripts, ws, game, script_nav, script_tools, .. } = self;
        let mut src = source(scripts, ws.as_ref(), game.as_deref());
        let refs = script_nav.index.references(d, &files, &mut src);
        let mut results = Vec::new();
        for r in &refs {
            let (Some(k), Some(text)) = (tools::nss(&r.file), src(&r.file)) else { continue };
            let line = text[..r.span.start].matches('\n').count();
            let shown = text.lines().nth(line).unwrap_or("").trim().to_string();
            results.push((k, line, shown));
        }
        let n = results.len();
        script_tools.search.results = results;
        script_tools.info = InfoTab::SearchResults;
        self.log.info(format!("{}: {n} reference(s) in the module", d.name));
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
            let Some(text) = ws.module.get(&k).map(decode) else { continue };
            spans.sort_by_key(|s| s.start);
            let mut out = String::with_capacity(text.len());
            let mut at = 0;
            for s in spans {
                out.push_str(&text[at..s.start]);
                out.push_str(new);
                at = s.end;
            }
            out.push_str(&text[at..]);
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

    /// Checks a script a moment after typing stops: compiles its text (an
    /// include file needs no `main`) and keeps the first error.
    pub(crate) fn live_check(&mut self, key: ResKey, ctx: &egui::Context) {
        let Some(text) = self.scripts.get(&key).map(|b| b.text.clone()) else { return };
        let h = hash(&text);
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
            ctx.request_repaint_after(PAUSE - waited);
            return;
        }
        let name = key.resref.to_lowercase().to_string();
        let error = {
            let module = self.ws.as_ref().map(|w| &w.module);
            let resman = self.game.as_deref().map(|g| &g.resman);
            let scripts = &self.scripts;
            let mut c = Compiler::new(|n: &str, t: ResType| {
                let k = mg_resman::ResKey::parse(n, t)?;
                if t == ResType::NSS
                    && let Some(b) = scripts.get(&k)
                {
                    return Some(encode(&b.text));
                }
                module
                    .and_then(|m| m.get(&k))
                    .map(<[u8]>::to_vec)
                    .or_else(|| resman?.get(&k).ok().map(|d| d.into_owned()))
            });
            c.set_require_entry_point(false);
            c.compile(&name).err()
        };
        let live = self.script_nav.live.entry(key).or_default();
        live.checked = Some(h);
        live.error = error.map(|e| {
            let line = e
                .location()
                .filter(|(file, _)| file.trim_end_matches(".nss").eq_ignore_ascii_case(&name))
                .map(|(_, line)| line.saturating_sub(1));
            (line, e.message)
        });
    }

    /// Sets an open script editor's text, as typing would (for tests).
    #[doc(hidden)]
    pub fn scripts_mut_for_test(&mut self, key: ResKey, text: &str) {
        if let Some(b) = self.scripts.get_mut(&key) {
            b.text = text.to_string();
        }
    }

    /// The error a script's latest check found (for tests): its 0-based
    /// line, if in the script, and message.
    #[doc(hidden)]
    pub fn script_error_for_test(&self, key: ResKey) -> Option<(Option<usize>, String)> {
        self.live_error(key).cloned()
    }

    /// The error a script's latest check found.
    pub(crate) fn live_error(&self, key: ResKey) -> Option<&(Option<usize>, String)> {
        self.script_nav.live.get(&key)?.error.as_ref()
    }
}

/// The Rename Symbol window.
pub(crate) fn rename_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some((decl, mut name)) = app.script_nav.rename.take() else { return };
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    egui::Window::new(format!("Rename {}", decl.name))
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(format!("{} — every use in the module's scripts", decl.signature));
            ui.horizontal(|ui| {
                ui.label("New name");
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(220.0));
                crate::widgets::autofocus(ui, &r);
            });
            ui.horizontal(|ui| {
                let can = !name.trim().is_empty() && name.trim() != decl.name;
                ok = ui.add_enabled(can, egui::Button::new("Rename")).clicked()
                    || (can && crate::widgets::enter(ui));
                cancel = ui.button("Cancel").clicked();
            });
        });
    if ok {
        let new = name.trim().to_string();
        app.rename_symbol(&decl, &new);
    } else if open && !cancel {
        app.script_nav.rename = Some((decl, name));
    }
}
