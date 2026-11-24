//! A script's Reference and Help panels: Reference (M4's U12, the Script
//! board's Right Dock; in the script's window where it floats, the
//! Floating board's right pane) lists the functions, variables and
//! constants the script can use (its own and its includes', then
//! `nwscript.nss`'s) and the templates, chosen by a segmented control
//! over a filter; Help (under the text since 5.2.0, a tab of the Bottom
//! Dock or of the script's window's bottom pane, as 4.2.1's pane was: the
//! owner's ruling on Jasperre's report) shows the one chosen there or
//! under the cursor, its documentation and declaration as the source has
//! them, in the editor's font. Both are drawn before the
//! script's editor (the docks are drawn before the editor area, a
//! window's panes before its tab): they read what the editor left
//! (`ScriptTools::symbols`, `help`), and what they do is for the editor
//! to do once it is drawn (`ScriptTools::insert`).

use egui::{Color32, Ui};
use mg_resman::ResKey;

use crate::script_tools::{SideTab, Symbol, SymbolKind};
use crate::script_view::{Palette, highlight};
use crate::{Moonglow, Tab};

/// The Reference panel's body's group (a screen reader's, and the tests'
/// where a name is the list's and the header's too: "Add").
pub const SCRIPT_REFERENCE: &str = "Script reference";
/// The Help panel's body's group.
pub const SCRIPT_HELP: &str = "Script help";

/// What Reference's header says after its title: where its lists are
/// from.
pub(crate) const REFERENCE_DETAIL: &str = "nwscript and includes";

/// What Help's header says after its title: the name it shows, or that
/// none is chosen.
pub(crate) fn help_detail(app: &Moonglow) -> String {
    match &app.script_tools.help {
        Some(s) => s.name.clone(),
        None => "Nothing chosen".to_string(),
    }
}

/// The script `editor` is (the active docked editor, or the floating one
/// in its window).
fn script_of(editor: Option<&Tab>) -> Option<ResKey> {
    match editor? {
        Tab::Script(key) => Some(*key),
        _ => None,
    }
}

/// The choices of Reference's segmented control.
const LISTS: [(SideTab, &str, &str); 4] = [
    (SideTab::Functions, "Functions", ""),
    (SideTab::Variables, "Variables", ""),
    (SideTab::Constants, "Constants", ""),
    (SideTab::Templates, "Templates", ""),
];

/// The gap between Reference's control, its filter and its list.
const GAP: f32 = 6.0;

/// The Reference panel's body (module comment), drawn for `editor`.
pub(crate) fn reference(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(key) = script_of(editor) else { return };
    mg_uikit::name_group(ui, SCRIPT_REFERENCE);
    let keys = crate::palette_view::Keys::Reference;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    let focus = std::mem::take(&mut app.script_tools.reference_focus);
    // (The script's symbols as its editor last made them: none before it
    // is first drawn, which draws the panel again once it has them.)
    let symbols = app.script_tools.symbols.get(&key).map(|(_, s)| s.clone());
    let shown = app.script_tools.help.as_ref().map(|h| h.name.clone());
    let pass = ui.ctx().cumulative_pass_nr();
    app.script_tools.reference_drawn = Some((pass, key, symbols.clone(), shown));
    let mut insert: Option<String> = None;
    let t = &mut app.script_tools;
    ui.spacing_mut().item_spacing.y = GAP;
    // (In a row of its own: laid out left to right, a segmented control
    // stands in the middle of the room it is given. Where the room is
    // short of the four words, a list to choose from, so that none is
    // cut: a window's pane narrowed to half a small window.)
    let words: f32 = LISTS
        .iter()
        .map(|(_, w, _)| {
            let font = egui::TextStyle::Button.resolve(ui.style());
            ui.painter().layout_no_wrap(w.to_string(), font, Color32::WHITE).size().x + 8.0
        })
        .sum();
    if words + 3.0 <= ui.available_width() {
        ui.horizontal(|ui| mg_uikit::segmented_fill(ui, "What to list", &mut t.side, &LISTS));
    } else {
        let name = |side: &SideTab| LISTS.iter().find(|l| l.0 == *side).map_or("", |l| l.1);
        let values: Vec<SideTab> = LISTS.iter().map(|l| l.0).collect();
        let width = ui.available_width();
        let r = crate::widgets::dropdown(("script-reference-lists", key))
            .width(width)
            .selected_text(name(&t.side))
            .show(ui, &mut t.side, &values, name);
        // (Named as the segmented control is.)
        ui.ctx().accesskit_node_builder(r.id, |node| node.set_label("What to list"));
    }
    // (Not Find's keys: in the script editor they are its Find bar's.)
    let what = match t.side {
        SideTab::Functions => "Find a function",
        SideTab::Variables => "Find a variable",
        SideTab::Constants => "Find a constant",
        SideTab::Templates => "Find a template",
    };
    let id = egui::Id::new(("script-reference-find", key));
    let found = crate::widgets::SearchField::new(id, &mut t.filter).hint(what).show(ui);
    if focus {
        found.response.request_focus();
    }
    let filter = mg_core::text::Needle::new(&t.filter);
    // (Long names end in "…" in a narrow panel: Help gives them whole.)
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
    if t.side == SideTab::Templates {
        let now = ui.input(|i| i.time);
        let (install, extra) = (app.install.as_ref(), app.settings.script_templates.as_deref());
        let templates = t.templates(install, extra, now);
        // (As wide as the panel: the scroll bar stays at its edge.)
        let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        area.id_salt("templates").show(ui, |ui| {
            mg_uikit::list_rows(ui);
            for (name, path) in templates.iter().filter(|(n, _)| filter.finds(n)) {
                let r = list_row(ui, name, false, false).on_hover_text("Double-click to insert");
                if r.double_clicked() {
                    insert = std::fs::read(path)
                        .ok()
                        .map(|b| crate::text::to_editor(&crate::text::decode_script(&b)).0);
                }
            }
        });
    } else {
        let kind = match t.side {
            SideTab::Functions => SymbolKind::Function,
            SideTab::Variables => SymbolKind::Variable,
            _ => SymbolKind::Constant,
        };
        let empty = crate::script_tools::SymbolSet::default();
        let set = symbols.as_ref().unwrap_or(&empty);
        if !t.ref_cache.as_ref().is_some_and(|c| c.fits(key, set, kind, &t.filter)) {
            t.ref_cache = Some(RefCache::new(key, set, kind, &t.filter, &filter));
        }
        let entries = &t.ref_cache.as_ref().expect("just cached").entries;
        let entry = |&(is_own, idx): &(bool, usize)| -> &Symbol {
            if is_own { &set.own[idx] } else { &set.spec[idx] }
        };
        let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        // (The name Help shows, chosen elsewhere (the text's cursor, Go to
        // Definition) since the list last showed it: the list scrolled to
        // it, in its middle, once.)
        let shown = t.help.as_ref().map(|h| h.name.clone());
        if shown.is_some() && shown != t.list_at {
            let at = entries.iter().position(|e| Some(&entry(e).name) == shown.as_ref());
            if let Some(at) = at {
                let (_, pitch) = mg_uikit::row_metrics(ui);
                let room = ui.available_height();
                area = area
                    .vertical_scroll_offset((at as f32 * pitch - (room - pitch) / 2.0).max(0.0));
            }
            t.list_at = shown;
        }
        let area = area.id_salt(("symbols", kind));
        mg_uikit::show_rows(area, ui, entries.len(), |ui, range| {
            // (Each row as wide as the list, so that its width doesn't
            // change with the names in view.)
            ui.set_min_width(ui.available_width());
            for e in &entries[range] {
                let s = entry(e);
                // (The module's own in the theme's text, the game's in its
                // secondary text. Chosen, the selection's own color.)
                let chosen = t.help.as_ref().is_some_and(|h| h.name == s.name);
                let quiet = !s.custom;
                let r = list_row(ui, &s.name, chosen, quiet).on_hover_text(&s.signature);
                // (A click shows it in Help, under the text, brought to the
                // front there as 4.2.1's click did; the list stays where it
                // is, so that a double click can put it in.)
                if r.clicked() {
                    t.help = Some(s.clone());
                    t.list_at = Some(s.name.clone());
                    t.help_forward = Some(key);
                }
                if r.double_clicked() {
                    insert = Some(match s.kind {
                        SymbolKind::Function => format!("{}(", s.name),
                        _ => s.name.clone(),
                    });
                }
            }
        });
    }
    if let Some(text) = insert {
        app.script_tools.insert = Some((key, text));
        ui.ctx().request_repaint();
    }
}

/// A row of Reference's list (`.list-row`), as wide as the list: the name
/// 8 points in, cut with "…" at the row's end (Help gives it whole); in
/// the theme's text, or its secondary text where `quiet` (the game's);
/// `chosen` (the one Help shows) on the accent's fill, the row's hover
/// fill under the pointer. Said as a selectable label of its name.
fn list_row(ui: &mut Ui, name: &str, chosen: bool, quiet: bool) -> egui::Response {
    let t = mg_uikit::tokens(ui.ctx());
    let height = mg_uikit::row_height(ui);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, chosen, name)
    });
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered() || response.has_focus();
        let (fill, text) = match (chosen, hovered, quiet) {
            (true, _, _) => (t.accent_fill, t.on_fill),
            (false, true, _) => (t.hover_row, t.text),
            (false, false, true) => (Color32::TRANSPARENT, t.text_2),
            (false, false, false) => (Color32::TRANSPARENT, t.text),
        };
        ui.painter().rect_filled(rect, 0.0, fill);
        let font = egui::TextStyle::Body.resolve(ui.style());
        let room = (rect.width() - 2.0 * ROW_PADDING).max(0.0);
        let mut job = egui::text::LayoutJob::simple_singleline(name.to_string(), font, text);
        job.wrap = egui::text::TextWrapping::truncate_at_width(room);
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        let at = egui::pos2(rect.left() + ROW_PADDING, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, text);
    }
    response
}

/// The room before a list row's name and after it.
const ROW_PADDING: f32 = 8.0;

/// The Help panel's body (module comment), drawn for `editor`: as 4.2.1
/// had it under the text (5.2.0, the owner's ruling on Jasperre's report),
/// the name's documentation and declaration verbatim, as the source has
/// them (`nwscript.nss`'s comment with its `//`, its spaces and its
/// columns, then the declaration), in the editor's font and colors on its
/// ground, not wrapped: the pane scrolls both ways.
pub(crate) fn help(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    if script_of(editor).is_none() {
        return;
    }
    mg_uikit::name_group(ui, SCRIPT_HELP);
    let keys = crate::palette_view::Keys::Help;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    // (F6 to it: the keyboard's focus on the panel itself, taken from the
    // script's text, so that what is typed goes nowhere, as F6 to any
    // panel takes it from where it was. A stop of Tab of its own.)
    let id = ui.id().with("script-help-focus");
    let focus = ui.interact(ui.max_rect(), id, egui::Sense::FOCUSABLE);
    focus.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Help's text"));
    if std::mem::take(&mut app.script_tools.help_focus) {
        focus.request_focus();
    }
    let pass = ui.ctx().cumulative_pass_nr();
    let shown = app.script_tools.help.as_ref().map(|h| h.name.clone());
    app.script_tools.help_drawn = Some((pass, shown));
    let palette = Palette::for_ui(&app.settings.script_style, ui);
    let t = mg_uikit::tokens(ui.ctx());
    let ground = palette.background.unwrap_or(t.inset);
    ui.painter().rect_filled(ui.max_rect(), 0.0, ground);
    let margin = egui::Margin { left: 10, right: 10, top: 6, bottom: 6 };
    let Some(h) = app.script_tools.help.as_ref() else {
        egui::Frame::new().inner_margin(margin).show(ui, |ui| {
            ui.weak("Put the cursor on a function or constant, or choose one in Reference.");
        });
        return;
    };
    // (Laid out once for the name shown and the editor's colors, not every
    // frame: `HelpRead`.)
    let colors =
        egui::epaint::util::hash((palette.colors, palette.size.to_bits(), palette.background));
    let job = match &app.script_tools.help_read {
        Some(r) if r.signature == h.signature && r.doc == h.doc && r.colors == colors => {
            r.job.clone()
        }
        _ => {
            let read = std::sync::Arc::new(HelpRead {
                signature: h.signature.clone(),
                doc: h.doc.clone(),
                colors,
                job: verbatim_job(h, &palette),
            });
            let job = read.job.clone();
            app.script_tools.help_read = Some(read);
            job
        }
    };
    // (egui's own cache gives the galley laid out last frame back.)
    let galley = ui.fonts_mut(|f| f.layout_job((*job).clone()));
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        let area = egui::ScrollArea::both().id_salt("script-help").auto_shrink([false, false]);
        area.show(ui, |ui| {
            ui.add(egui::Label::new(galley).extend());
        });
    });
}

/// A name's Help as the source has it (4.2.1's): its documentation comment
/// as written, then its declaration, highlighted in `palette`, not wrapped.
pub(crate) fn verbatim(s: &Symbol) -> String {
    match s.doc.trim_end() {
        "" => s.signature.clone(),
        doc => format!("{doc}\n{}", s.signature),
    }
}

/// Cached filtered list of symbols for [`reference`]: places in the
/// symbol set it was made from, which it holds (compared by address, as
/// the set is made again whenever its script or nwscript changes). Keyed
/// by the sets' addresses alone, a set dropped and another made where it
/// had been took the old list's places, for other names or past its end:
/// review D3 of the adversarial branch.
#[derive(Debug, Clone)]
pub(crate) struct RefCache {
    pub(crate) key: ResKey,
    made: crate::script_tools::SymbolSet,
    kind: SymbolKind,
    filter: String,
    entries: Vec<(bool, usize)>,
}

impl RefCache {
    /// The `kind` of `set`'s symbols `finds` keeps (the panel's `filter`
    /// typed), in the order the panel lists them.
    fn new(
        key: ResKey,
        set: &crate::script_tools::SymbolSet,
        kind: SymbolKind,
        filter: &str,
        finds: &mg_core::text::Needle,
    ) -> RefCache {
        let mut entries = Vec::new();
        let mut iter = set.iter();
        while let Some((is_own, idx, s)) = iter.next_indexed() {
            if s.kind == kind && finds.finds(&s.name) {
                entries.push((is_own, idx));
            }
        }
        RefCache { key, made: set.clone(), kind, filter: filter.to_owned(), entries }
    }

    /// Whether it is the list of `key`'s `set`, `kind` and `filter`.
    fn fits(
        &self,
        key: ResKey,
        set: &crate::script_tools::SymbolSet,
        kind: SymbolKind,
        filter: &str,
    ) -> bool {
        self.key == key && self.made.same(set) && self.kind == kind && self.filter == filter
    }
}

/// [`verbatim`] laid out as the editor's text is (its colors and font).
fn verbatim_job(s: &Symbol, palette: &Palette) -> std::sync::Arc<egui::text::LayoutJob> {
    let mut job = highlight(&verbatim(s), palette);
    job.wrap.max_width = f32::INFINITY;
    std::sync::Arc::new(job)
}

/// What Help laid out for the name it shows ([`help`]): its documentation
/// and declaration verbatim in the editor's colors (`colors`, their hash),
/// kept while the name and the colors are those shown
/// (`ScriptTools::help_read`): not laid out again every frame.
#[derive(Debug)]
pub(crate) struct HelpRead {
    signature: String,
    doc: String,
    colors: u64,
    job: std::sync::Arc<egui::text::LayoutJob>,
}

/// The first line of a name's documentation, as prose (its comment's
/// markers off): what the completion list's detail and the tip over a name
/// say under its declaration. `None` where it has none.
pub(crate) fn first_doc_line(doc: &str) -> Option<String> {
    if doc.trim().is_empty() {
        return None;
    }
    // (BioWare's note mark, `* Returns…`, off too: the line is prose.)
    let prose = |l: String| l.trim().trim_start_matches("* ").trim().to_string();
    comment_lines(doc).into_iter().map(prose).find(|l| !l.is_empty())
}

/// A comment's lines without their markers: a line's `//` and one space
/// after it (and BioWare's `::` after that, `//:: Hidden`); a block's `/*`
/// and `*/`, the ` * ` down its side and the indentation all its lines
/// share. Rules (a line of `/`, `*`, `:`, `-` or `=` alone) are dropped,
/// as are empty lines at the ends. A `//` line and a block may follow
/// each other (BioWare's includes: `//:: Name` over a block).
fn comment_lines(doc: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    // (The block's lines, dedented together once it ends.)
    let mut block: Option<Vec<String>> = None;
    let end_block = |block: Vec<String>, lines: &mut Vec<String>| {
        let indent = |l: &String| l.len() - l.trim_start().len();
        let common = block.iter().filter(|l| !l.trim().is_empty()).map(indent).min();
        let common = common.unwrap_or(0);
        lines.extend(block.into_iter().map(|l| l.get(common..).unwrap_or("").to_string()));
    };
    for raw in doc.lines() {
        let line = raw.trim_end();
        if let Some(mut inner) = block.take() {
            let (text, ends) = match line.find("*/") {
                Some(at) => (&line[..at], true),
                None => (line, false),
            };
            // (` * ` down the side of a documentation comment.)
            let trimmed = text.trim_start();
            let text = match trimmed.strip_prefix('*') {
                Some(rest) if rest.is_empty() || rest.starts_with(' ') => {
                    rest.strip_prefix(' ').unwrap_or(rest)
                }
                _ => text,
            };
            inner.push(text.trim_end().to_string());
            match ends {
                true => end_block(inner, &mut lines),
                false => block = Some(inner),
            }
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("/*") {
            match rest.find("*/") {
                Some(at) => lines.push(rest[..at].trim().to_string()),
                None => block = Some(vec![rest.trim().to_string()]),
            }
            continue;
        }
        let text = trimmed.strip_prefix("//").unwrap_or(trimmed);
        let text = text.strip_prefix("::").unwrap_or(text);
        let text = text.strip_prefix(' ').unwrap_or(text);
        lines.push(text.to_string());
    }
    if let Some(inner) = block {
        end_block(inner, &mut lines);
    }
    let rule = |l: &String| {
        let t = l.trim();
        t.len() >= 3 && t.chars().all(|c| matches!(c, '/' | '*' | ':' | '-' | '=' | '#'))
    };
    let mut lines: Vec<String> =
        lines.into_iter().map(|l| if rule(&l) { String::new() } else { l }).collect();
    while lines.first().is_some_and(|l| l.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn function(signature: &str, doc: &str) -> Symbol {
        Symbol {
            name: "F".into(),
            kind: SymbolKind::Function,
            signature: signature.into(),
            doc: doc.into(),
            custom: false,
        }
    }

    /// Help as 4.2.1 had it: the comment as the source has it (its `//`,
    /// its spaces and its columns), then the declaration; a name with no
    /// comment, the declaration alone.
    #[test]
    fn help_is_the_comment_and_the_declaration_as_written() {
        let doc = "// Get the reputation.\n// 0-10   :  Hostile.\n// - oSource: the one";
        let f = function("int GetReputation(object oSource)", doc);
        assert_eq!(
            verbatim(&f),
            "// Get the reputation.\n// 0-10   :  Hostile.\n// - oSource: the one\n\
             int GetReputation(object oSource)"
        );
        assert_eq!(verbatim(&function("int TRUE = 1", "")), "int TRUE = 1");
    }

    /// The first line of the documentation, without its comment's markers:
    /// a `//` line's, a block's first words (BioWare's `//::` name over a
    /// block, the rules dropped); none for a name with no comment.
    #[test]
    fn the_first_line_of_the_documentation_is_its_first_words() {
        let doc = "// Cause the action subject to move.\n// - oMoveTo: there";
        assert_eq!(first_doc_line(doc).as_deref(), Some("Cause the action subject to move."));
        let doc = "/*\n * Sets it.\n *\n * Then more.\n */";
        assert_eq!(first_doc_line(doc).as_deref(), Some("Sets it."));
        let doc = "//::////////////\n//:: Hidden\n/*\n    Returns TRUE\n*/";
        assert_eq!(first_doc_line(doc).as_deref(), Some("Hidden"));
        let doc = "// * Returns TRUE if oObject is open.";
        assert_eq!(first_doc_line(doc).as_deref(), Some("Returns TRUE if oObject is open."));
        assert_eq!(first_doc_line(""), None);
        assert_eq!(first_doc_line("//\n//   \n"), None);
    }

    /// The Reference list's cache holds the symbol set its places are in:
    /// a set made again (the script typed in) is another list, whatever
    /// address it is given. Keyed by addresses it did not hold, a set made
    /// where the dropped one had been took its places (review D3).
    #[test]
    fn the_reference_list_holds_the_set_it_lists() {
        use crate::script_tools::SymbolSet;
        let key = ResKey::parse("hello", mg_core::ResType::NSS).unwrap();
        let named = |n: &str| Symbol { name: n.into(), ..function("void F()", "") };
        let spec = Arc::new(vec![named("GetTag")]);
        let set = SymbolSet { own: Arc::new(vec![named("Mine"), named("Other")]), spec };
        let all = mg_core::text::Needle::new("");
        let cache = RefCache::new(key, &set, SymbolKind::Function, "", &all);
        assert_eq!(cache.entries.len(), 3);
        assert!(cache.fits(key, &set, SymbolKind::Function, ""));
        assert!(!cache.fits(key, &set, SymbolKind::Constant, ""));
        assert!(!cache.fits(key, &set, SymbolKind::Function, "Get"));
        // The script typed in: its set made again, the old one let go.
        let old = Arc::downgrade(&set.own);
        let typed = SymbolSet { own: Arc::new(vec![named("Mine")]), spec: set.spec.clone() };
        drop(set);
        assert!(old.upgrade().is_some(), "the list's set is held while the list is");
        assert!(!Arc::ptr_eq(&old.upgrade().unwrap(), &typed.own));
        assert!(!cache.fits(key, &typed, SymbolKind::Function, ""));
    }
}
