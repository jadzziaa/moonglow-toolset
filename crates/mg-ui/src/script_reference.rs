//! A script's Reference and Help panels (M4's U12, the Script board's
//! Right Dock; in the script's window where it floats, the Floating
//! board's right pane): Reference lists the functions, variables and
//! constants the script can use (its own and its includes', then
//! `nwscript.nss`'s) and the templates, chosen by a segmented control
//! over a filter; Help shows the one chosen there or under the cursor:
//! its declaration, one parameter to a line, and its documentation as
//! prose with a row for each parameter. Both are drawn before the
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
    let made = symbols.as_ref().map_or(0, |s| s.ptr_id());
    let shown = app.script_tools.help.as_ref().map(|h| h.name.clone());
    let pass = ui.ctx().cumulative_pass_nr();
    app.script_tools.reference_drawn = Some((pass, key, made, shown));
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
        let cache_key = set.cache_key();
        let hit = t.ref_cache.as_ref().is_some_and(|c| {
            c.key == key && c.made == cache_key && c.kind == kind && c.filter == t.filter
        });
        if !hit {
            let mut entries = Vec::new();
            let mut iter = set.iter();
            while let Some((is_own, idx, s)) = iter.next_indexed() {
                if s.kind == kind && filter.finds(&s.name) {
                    entries.push((is_own, idx));
                }
            }
            t.ref_cache =
                Some(RefCache { key, made: cache_key, kind, filter: t.filter.clone(), entries });
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
                // (A click shows it in Help, beside the list: the one tab
                // bar they share under 1400 stays on the list, so that a
                // double click can put it in.)
                if r.clicked() {
                    t.help = Some(s.clone());
                    t.list_at = Some(s.name.clone());
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

/// The room between the parts of Help's documentation (`.help-doc`'s
/// gap).
const DOC_GAP: f32 = 6.0;
/// The least width of the column of parameter names (`.param`'s 60 px).
const PARAM_COLUMN: f32 = 60.0;
/// The gap between a parameter's name and what it says.
const PARAM_GAP: f32 = 8.0;

/// The Help panel's body (module comment), drawn for `editor`.
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
    let Some(h) = app.script_tools.help.clone() else {
        ui.weak("Choose a function or constant in Reference, or put the cursor on one.");
        return;
    };
    let palette = Palette::for_ui(&app.settings.script_style, ui);
    let t = mg_uikit::tokens(ui.ctx());
    // (Read once for the name shown, not every frame: `HelpRead`.)
    let read = match &app.script_tools.help_read {
        Some(r) if r.signature == h.signature && r.doc == h.doc => r.clone(),
        _ => {
            let r = std::sync::Arc::new(HelpRead {
                signature: h.signature.clone(),
                doc: h.doc.clone(),
                declaration: declaration(&h),
                blocks: doc_blocks_with(&h.doc, &param_names(&h.signature)),
            });
            app.script_tools.help_read = Some(r.clone());
            r
        }
    };
    let read_ptr = std::sync::Arc::as_ptr(&read) as usize;
    let ppp = ui.ctx().pixels_per_point();
    let fonts = crate::script_layout::fonts_made(ui.ctx());
    let mono = egui::FontId::monospace(12.0);
    let body = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
    let layout = match &mut app.script_tools.help_layout {
        Some(cached) if cached.read_ptr == read_ptr => {
            if cached.palette != palette {
                cached.palette = palette;
                cached.decl_job = decl_layout(&read.declaration, &palette);
            }
            if cached.ppp != ppp || cached.fonts != fonts {
                cached.ppp = ppp;
                cached.fonts = fonts;
                (cached.param_widths, cached.widest) = measure_params(ui, &read.blocks, &mono);
            }
            cached
        }
        slot => {
            let (param_widths, widest) = measure_params(ui, &read.blocks, &mono);
            slot.insert(HelpLayout {
                read_ptr,
                palette,
                decl_job: decl_layout(&read.declaration, &palette),
                ppp,
                fonts,
                param_widths,
                widest,
            })
        }
    };
    let area = egui::ScrollArea::vertical().id_salt("script-help").auto_shrink([false, false]);
    area.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = DOC_GAP;
        let width = ui.available_width();
        // The declaration (`.sig`): a parameter to a line, in the editor's
        // colors and font, on its ground, its lines whole (scrolled across
        // where the panel is narrower), 18 points apart.
        let ground = palette.background.unwrap_or(t.inset);
        egui::Frame::new()
            .fill(ground)
            .stroke(egui::Stroke::new(1.0, t.line_soft))
            .corner_radius(3)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                egui::ScrollArea::horizontal().id_salt("script-help-declaration").show(ui, |ui| {
                    ui.add(egui::Label::new(layout.decl_job.clone()).extend());
                });
            });
        // (Each parameter's name in a column as wide as the widest, from
        // the board's 60 to two fifths of the panel; a longer one has what
        // it says on the line under it.)
        let column = layout.widest.clamp(PARAM_COLUMN, (width * 0.4).max(PARAM_COLUMN));
        let mut param_idx = 0;
        for (i, block) in read.blocks.iter().enumerate() {
            match block {
                Block::Prose(text) | Block::Item(text) => {
                    ui.add(egui::Label::new(paragraph(text, &body, t.text_2, width)).wrap());
                }
                Block::Note(text) => {
                    ui.add(egui::Label::new(paragraph(text, &body, t.text_3, width)).wrap());
                }
                Block::Param { name, text } => {
                    let name_w =
                        layout.param_widths.get(param_idx).copied().unwrap_or(column + 1.0);
                    param_idx += 1;
                    let fonts = (&mono, &body);
                    param_row(ui, name, name_w, text, column, fonts, (t.text, t.text_2));
                }
                // (As written, its columns kept: nwscript.nss lines up its
                // lists. Scrolled across where the panel is narrower.)
                Block::Lines(lines) => {
                    let job = paragraph(&lines.join("\n"), &mono, t.text_2, f32::INFINITY);
                    let area = egui::ScrollArea::horizontal().id_salt(("help-lines", i));
                    area.show(ui, |ui| ui.add(egui::Label::new(job).extend()));
                }
            }
        }
    });
}

/// Cached filtered list of symbols for [`reference`].
#[derive(Debug, Clone)]
pub(crate) struct RefCache {
    pub(crate) key: ResKey,
    made: (usize, usize),
    kind: SymbolKind,
    filter: String,
    entries: Vec<(bool, usize)>,
}

/// Cached syntax-highlighted declaration and parameter width measurements for [`help`].
#[derive(Debug, Clone)]
pub(crate) struct HelpLayout {
    read_ptr: usize,
    palette: Palette,
    decl_job: egui::text::LayoutJob,
    ppp: f32,
    fonts: u64,
    param_widths: Vec<f32>,
    widest: f32,
}

fn decl_layout(declaration: &str, palette: &Palette) -> egui::text::LayoutJob {
    let mut job = highlight(declaration, palette);
    job.wrap.max_width = f32::INFINITY;
    for section in &mut job.sections {
        section.format.line_height = Some(LINE_HEIGHT);
    }
    job
}

fn measure_params(ui: &Ui, blocks: &[Block], mono: &egui::FontId) -> (Vec<f32>, f32) {
    let mut widths = Vec::new();
    let mut widest = 0.0_f32;
    let painter = ui.painter();
    for b in blocks {
        if let Block::Param { name, .. } = b {
            let w = painter
                .layout_no_wrap(name.to_string(), mono.clone(), Color32::WHITE)
                .size()
                .x
                .ceil();
            widest = widest.max(w);
            widths.push(w);
        }
    }
    (widths, widest)
}

/// The room from one line of Help's text to the next (`.help-doc` and
/// `.sig`: `line-height: 18px`).
const LINE_HEIGHT: f32 = 18.0;

/// What Help read of the name it shows ([`help`]): its declaration as
/// shown and its documentation's parts, kept while the name is shown
/// (`ScriptTools::help_read`): not read again every frame.
#[derive(Debug)]
pub(crate) struct HelpRead {
    signature: String,
    doc: String,
    declaration: String,
    blocks: Vec<Block>,
}

/// `text` in `font` and `color`, its lines [`LINE_HEIGHT`] apart, wrapped
/// at `width`.
fn paragraph(text: &str, font: &egui::FontId, color: Color32, width: f32) -> egui::text::LayoutJob {
    let format = egui::TextFormat {
        font_id: font.clone(),
        color,
        line_height: Some(LINE_HEIGHT),
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::single_section(text.to_string(), format);
    job.wrap.max_width = width;
    job
}

/// A parameter's row (`.param`): its name in `column`, in `colors.0`,
/// what it says beside it in `colors.1`, wrapped; under it where the name
/// is wider than the column.
fn param_row(
    ui: &mut Ui,
    name: &str,
    name_w: f32,
    text: &str,
    column: f32,
    (mono, body): (&egui::FontId, &egui::FontId),
    colors: (Color32, Color32),
) {
    let fits = name_w <= column + 0.5;
    let name = paragraph(name, mono, colors.0, f32::INFINITY);
    if !fits {
        ui.add(egui::Label::new(name));
        if !text.is_empty() {
            ui.horizontal_top(|ui| {
                ui.add_space(column + PARAM_GAP);
                let room = ui.available_width();
                ui.add(egui::Label::new(paragraph(text, body, colors.1, room)).wrap());
            });
        }
        return;
    }
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(column, 0.0), egui::Sense::hover());
        let mut cell = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_size(rect.min, egui::vec2(column, 400.0))),
        );
        cell.add(egui::Label::new(name));
        ui.add_space(PARAM_GAP);
        let room = ui.available_width();
        ui.add(egui::Label::new(paragraph(text, body, colors.1, room)).wrap());
        // (As high as the name, where what it says is shorter.)
        ui.set_min_height(cell.min_rect().height());
    });
}

/// A function's or constant's declaration as Help shows it: a function
/// with parameters one to a line, indented, after its name and `(`; else
/// as declared.
pub(crate) fn declaration(s: &Symbol) -> String {
    let sig = s.signature.as_str();
    if s.kind != SymbolKind::Function {
        return sig.to_string();
    }
    let (Some(open), Some(close)) = (sig.find('('), sig.rfind(')')) else {
        return sig.to_string();
    };
    if close < open {
        return sig.to_string();
    }
    let params = split_params(&sig[open + 1..close]);
    if params.is_empty() {
        return sig.to_string();
    }
    let lines: Vec<String> = params.iter().map(|p| format!("    {p}")).collect();
    format!("{}\n{}{}", &sig[..=open], lines.join(",\n"), &sig[close..])
}

/// A parameter list's parameters, by the commas outside brackets and
/// strings (a vector's default, `[0.0, 0.0, 0.0]`), each trimmed.
fn split_params(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut depth, mut quoted, mut from) = (0i32, false, 0);
    let bytes = list.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        match b {
            b'"' if i == 0 || bytes[i - 1] != b'\\' => quoted = !quoted,
            b'(' | b'[' | b'{' if !quoted => depth += 1,
            b')' | b']' | b'}' if !quoted => depth -= 1,
            b',' if !quoted && depth == 0 => {
                out.push(list[from..i].trim().to_string());
                from = i + 1;
            }
            _ => {}
        }
    }
    let last = list[from..].trim();
    if !last.is_empty() || !out.is_empty() {
        out.push(last.to_string());
    }
    out
}

/// A part of Help's documentation ([`doc_blocks`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Block {
    /// Lines of prose, joined into one paragraph (wrapped to the panel).
    Prose(String),
    /// A parameter's line, `- nFeat: FEAT_*` (and the lines indented under
    /// it): its name and what it says (nothing, `- oCreature`).
    Param { name: String, text: String },
    /// A line of a list that is not a parameter's (`- nYear should be from
    /// 0 to 32000 inclusive`, `-> 0-10 means…`, `1) Time can only…`), as
    /// written with its mark, and the lines indented under it: a line of
    /// its own, never joined with the next.
    Item(String),
    /// A note, BioWare's `* Returns…` (and the lines indented under it).
    Note(String),
    /// Lines kept as written, their columns and indentation: a list lined
    /// up in columns, or code (`void main()`, `DelayCommand(…);`):
    /// monospace.
    Lines(Vec<String>),
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

/// Whether a line is laid out in columns: three spaces or more between
/// two of its words (nwscript.nss's lists, `0-10   :  Hostile.`).
fn in_columns(line: &str) -> bool {
    line.trim().contains("   ")
}

/// NWScript's types, a line of code's first word where it declares one.
const TYPES: [&str; 16] = [
    "void",
    "int",
    "float",
    "string",
    "object",
    "effect",
    "itemproperty",
    "location",
    "vector",
    "struct",
    "talent",
    "event",
    "json",
    "sqlquery",
    "cassowary",
    "const",
];

/// Whether a line is code, an example to be kept as written: a brace
/// alone, a statement (`AssignCommand(…);`, `effect e = …;`), a
/// declaration (`void main()`, `object oChair = OBJECT_SELF;`), a line
/// ending in `{`, or a `//` comment inside the example.
fn code_like(line: &str) -> bool {
    let t = line.trim();
    if matches!(t, "{" | "}" | "};") || t.starts_with("//") || t.ends_with('{') {
        return true;
    }
    let first = t.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
    let declares = TYPES.contains(&first)
        && t[first.len()..].starts_with(' ')
        && t[first.len()..].trim_start().starts_with(|c: char| c.is_alphabetic() || c == '_')
        && (t.ends_with(';') || t.ends_with(')') || t.contains('='));
    // (A statement: a call or an assignment ending in `;`, with no space
    // before its first bracket: `DelayCommand(fDelay, …);`.)
    let statement = t.ends_with(';')
        && t.split('(').next().is_some_and(|f| !f.is_empty() && !f.trim_end().contains(' '));
    declares || statement
}

/// The mark a line of a list begins with, where it has one: `-`, `->`,
/// `1)`, `a)`, `1.` followed by a space.
fn list_mark(text: &str) -> bool {
    if text.starts_with("- ") || text.starts_with("-> ") {
        return true;
    }
    let mark: String = text.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    let after = &text[mark.len()..];
    !mark.is_empty()
        && mark.len() <= 2
        && (mark.chars().all(|c| c.is_ascii_digit()) || mark.len() == 1)
        && (after.starts_with(") ")
            || after.starts_with(". ") && mark.chars().all(|c| c.is_ascii_digit()))
}

/// A function's or constant's documentation (its comment as written,
/// `Symbol::doc`) as Help shows it, the Script board's `.help-doc`:
/// prose joined into paragraphs; a row for each parameter (`- name:
/// text`, `- name`); a line of its own for each other line of a list
/// (`- …`, `-> …`, `1) …`); a note for each `* …` line; and as written,
/// monospace, what is laid out in columns (so that a list lined up in
/// columns keeps them: Jasperre's report) and code (an example, `void
/// main() {…}`). A line indented under a paragraph, a list's line, a
/// parameter or a note goes on with it (a sentence wrapped in the
/// source); an empty line ends a paragraph.
#[cfg(test)]
pub(crate) fn doc_blocks(doc: &str) -> Vec<Block> {
    doc_blocks_with(doc, &[])
}

/// The names of a function's parameters, by its declaration
/// (`void F(object oItem, int bRun=FALSE)`: `oItem`, `bRun`).
pub(crate) fn param_names(signature: &str) -> Vec<String> {
    let (Some(open), Some(close)) = (signature.find('('), signature.rfind(')')) else {
        return Vec::new();
    };
    if close < open {
        return Vec::new();
    }
    let params = split_params(&signature[open + 1..close]);
    let name = |p: &String| {
        let before = p.split('=').next().unwrap_or("").trim();
        before.split_whitespace().last().map(str::to_string)
    };
    params.iter().filter_map(name).collect()
}

/// [`doc_blocks`] of a function with the parameters `params`: a line that
/// begins with one of their names and ` - ` or `: ` (BioWare's `oItem -
/// item to copy`, without the dash before it) is that parameter's row too.
pub(crate) fn doc_blocks_with(doc: &str, params: &[String]) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    // (Whether the last block may go on with the next line.)
    let mut open = false;
    for line in comment_lines(doc) {
        if line.trim().is_empty() {
            open = false;
            continue;
        }
        let indented = line.starts_with("  ") || line.starts_with('\t');
        let text = line.trim();
        let columns = in_columns(text);
        let code = code_like(text);
        let last_lines = matches!(out.last(), Some(Block::Lines(_)));
        // A parameter's line first: its words may stand in columns.
        if let Some(rest) = text.strip_prefix("- ").or_else(|| (text == "-").then_some(""))
            && !indented
        {
            let rest = rest.trim();
            let (name, said) = match rest.split_once(':') {
                Some((name, said)) => (name.trim(), said.trim()),
                None => (rest, ""),
            };
            // (A parameter's name is one word; a dash before a sentence
            // is a list's line.)
            if !name.is_empty() && !name.contains(char::is_whitespace) {
                out.push(Block::Param { name: name.to_string(), text: said.to_string() });
                open = true;
                continue;
            }
        }
        // A line naming a parameter first: `oItem - item to copy`.
        let named = (!indented).then(|| {
            params.iter().find_map(|p| {
                let rest = text.strip_prefix(p.as_str())?;
                let said = rest.strip_prefix(" - ").or_else(|| rest.strip_prefix(": "))?;
                Some((p.clone(), said.trim().to_string()))
            })
        });
        if let Some((name, text)) = named.flatten() {
            out.push(Block::Param { name, text });
            open = true;
            continue;
        }
        // Code, and what is indented in it, goes on with the code before.
        if code || (open && last_lines && (indented || text.contains("  "))) {
            match out.last_mut() {
                Some(Block::Lines(lines)) if open => lines.push(line.clone()),
                _ => out.push(Block::Lines(vec![line.clone()])),
            }
            open = true;
            continue;
        }
        if columns {
            out.push(Block::Lines(vec![line.clone()]));
            open = true;
            continue;
        }
        // A line indented under prose, a list's line, a parameter or a
        // note goes on with it: a sentence wrapped in the source.
        if indented && open {
            let said = match out.last_mut() {
                Some(Block::Param { text: said, .. })
                | Some(Block::Note(said))
                | Some(Block::Item(said))
                | Some(Block::Prose(said)) => Some(said),
                _ => None,
            };
            if let Some(said) = said {
                if !said.is_empty() {
                    said.push(' ');
                }
                said.push_str(text);
                continue;
            }
        }
        if list_mark(text) {
            out.push(Block::Item(text.to_string()));
            open = true;
            continue;
        }
        if let Some(rest) = text.strip_prefix('*') {
            out.push(Block::Note(rest.trim().to_string()));
            open = true;
            continue;
        }
        match out.last_mut() {
            Some(Block::Prose(p)) if open => {
                p.push(' ');
                p.push_str(text);
            }
            _ => out.push(Block::Prose(text.to_string())),
        }
        open = true;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(signature: &str, doc: &str) -> Symbol {
        Symbol {
            name: "F".into(),
            kind: SymbolKind::Function,
            signature: signature.into(),
            doc: doc.into(),
            custom: false,
        }
    }

    /// The Script board's Help: prose joined, a row per parameter, the
    /// note with the line indented under it.
    #[test]
    fn documentation_is_prose_with_a_row_for_each_parameter() {
        let doc = "// Cause the action subject to move to a certain distance from oMoveTo.\n\
                   // If there is no path to oMoveTo, this command will do nothing.\n\
                   // - oMoveTo: This is the object we wish the action subject to move to\n\
                   // - bRun: If this is TRUE, the action subject will run rather than walk\n\
                   // - fRange: This is the desired distance\n\
                   // * No return value, but if an error occurs the log file will contain\n\
                   //   \"ActionMoveToObject failed.\"";
        assert_eq!(
            doc_blocks(doc),
            [
                Block::Prose(
                    "Cause the action subject to move to a certain distance from oMoveTo. \
                     If there is no path to oMoveTo, this command will do nothing."
                        .into()
                ),
                Block::Param {
                    name: "oMoveTo".into(),
                    text: "This is the object we wish the action subject to move to".into()
                },
                Block::Param {
                    name: "bRun".into(),
                    text: "If this is TRUE, the action subject will run rather than walk".into()
                },
                Block::Param { name: "fRange".into(), text: "This is the desired distance".into() },
                Block::Note(
                    "No return value, but if an error occurs the log file will contain \
                     \"ActionMoveToObject failed.\""
                        .into()
                ),
            ]
        );
    }

    /// Lines in columns, and lines indented under prose, are kept as
    /// written (Jasperre's report: lists lost their alignment); a
    /// parameter with no words and one indented under it.
    #[test]
    fn columns_and_indented_lines_keep_their_layout() {
        let doc = "// Get the reputation.\n\
                   // 0-10   :  Hostile.\n\
                   // 11-89  :  Neutral.\n\
                   // - oSource\n\
                   // - nA:    the first\n\
                   //           (any)\n\
                   // Effects:\n\
                   //      ABILITYDRAIN      :nSpecial is the ability";
        assert_eq!(
            doc_blocks(doc),
            [
                Block::Prose("Get the reputation.".into()),
                Block::Lines(vec!["0-10   :  Hostile.".into(), "11-89  :  Neutral.".into()]),
                Block::Param { name: "oSource".into(), text: String::new() },
                Block::Param { name: "nA".into(), text: "the first (any)".into() },
                Block::Prose("Effects:".into()),
                Block::Lines(vec!["     ABILITYDRAIN      :nSpecial is the ability".into()]),
            ]
        );
    }

    /// A block comment loses its markers and the `*`s down its side; an
    /// empty line ends a paragraph.
    #[test]
    fn a_block_comment_reads_as_its_lines() {
        let doc = "/*\n * Sets it.\n *\n * Then more.\n */";
        assert_eq!(
            doc_blocks(doc),
            [Block::Prose("Sets it.".into()), Block::Prose("Then more.".into())]
        );
        // (BioWare's: a `//::` name over an indented block, between rules.)
        let doc = "//::////////////\n//:: Hidden\n/*\n    Returns TRUE if oSelf\n    is hidden\n*/";
        assert_eq!(
            doc_blocks(doc),
            [Block::Prose("Hidden".into()), Block::Prose("Returns TRUE if oSelf is hidden".into())]
        );
        assert_eq!(doc_blocks(""), []);
    }

    /// A function's parameters one to a line; none, or a constant, as
    /// declared; a vector's default keeps its commas.
    #[test]
    fn a_declaration_has_a_parameter_to_a_line() {
        let f = function("void ActionMoveToObject(object oMoveTo, int bRun=FALSE)", "");
        assert_eq!(
            declaration(&f),
            "void ActionMoveToObject(\n    object oMoveTo,\n    int bRun=FALSE)"
        );
        let none = function("object GetModuleItemAcquired()", "");
        assert_eq!(declaration(&none), "object GetModuleItemAcquired()");
        let v = function("void F(vector v=[0.0, 1.0, 2.0], string s=\"a,b\")", "");
        assert_eq!(
            declaration(&v),
            "void F(\n    vector v=[0.0, 1.0, 2.0],\n    string s=\"a,b\")"
        );
        let c = Symbol { kind: SymbolKind::Constant, ..function("int TRUE = 1", "") };
        assert_eq!(declaration(&c), "int TRUE = 1");
    }
}

/// Help over the game's own `nwscript.nss` (a corpus test: skipped
/// without a game install), as the review of M4's U12 read it: the cases
/// it named, and a sweep of every documented function and constant.
#[cfg(test)]
mod corpus {
    use super::*;

    /// Every documented function's and constant's (name, declaration,
    /// documentation as written), of the install at `root`.
    fn documented(root: std::path::PathBuf) -> Vec<(String, String, String)> {
        let install = mg_resman::GameInstall::new(root, None, "en");
        let game = mg_rules::GameData::open(&install).expect("the game's data");
        let data = game.resman.get_named("nwscript", mg_core::ResType::NSS).expect("nwscript");
        let spec = mg_script::spec::Spec::parse(&data);
        let mut out: Vec<(String, String, String)> = (spec.functions.iter())
            .map(|f| (f.name.clone(), f.signature(), f.doc_source.clone()))
            .collect();
        out.extend(
            spec.constants.iter().map(|c| (c.name.clone(), String::new(), c.doc_source.clone())),
        );
        out.retain(|(_, _, doc)| !doc.is_empty());
        out
    }

    fn blocks_of(all: &[(String, String, String)], name: &str) -> Vec<Block> {
        let (_, sig, doc) =
            all.iter().find(|(n, ..)| n == name).unwrap_or_else(|| panic!("{name}"));
        doc_blocks_with(doc, &param_names(sig))
    }

    fn item(text: &str) -> Block {
        Block::Item(text.into())
    }

    /// The cases the review named: a list's lines each its own (SetCalendar,
    /// SetTime, GetReputation's `->`), a sentence wrapped in the source
    /// joined to its paragraph (GetEnteringObject, CopyItem,
    /// EffectDamageIncrease), code kept as written (ActionSit,
    /// DelayCommand), and Jasperre's columns
    /// (GetStandardFactionReputation, ItemPropertyOnHitProps).
    #[test]
    fn help_reads_nwscript_s_lists_sentences_code_and_columns() {
        let root = mg_testkit::corpus!();
        let all = documented(root);
        let has = |name: &str, b: &Block| {
            let blocks = blocks_of(&all, name);
            assert!(blocks.contains(b), "{name}: {b:?} not in {blocks:#?}");
        };
        has("SetCalendar", &item("- nYear should be from 0 to 32000 inclusive"));
        has("SetCalendar", &item("- nDay should be from 1 to 28 inclusive"));
        has(
            "SetCalendar",
            &item(
                "1) Time can only be advanced forwards; attempting to set the time backwards \
                 will result in no change to the calendar.",
            ),
        );
        has("SetTime", &item("- nMillisecond should be from 0 to 999 inclusive"));
        has("GetReputation", &item("-> 0-10 means oSource is hostile to oTarget"));
        has("GetReputation", &item("-> 90-100 means oSource is friendly to oTarget"));
        has(
            "GetEnteringObject",
            &item("1) If the caller is a door it returns the object that last triggered it."),
        );
        has(
            "CopyItem",
            &Block::Param {
                name: "oTargetInventory".into(),
                text: "create item in this object's inventory. If this parameter is not \
                       valid, the item will be created in oItem's location"
                    .into(),
            },
        );
        has(
            "EffectDamageIncrease",
            &Block::Prose(
                "NOTE! You *must* use the DAMAGE_BONUS_* constants! Using other values may \
                 result in odd behaviour."
                    .into(),
            ),
        );
        has(
            "ActionSit",
            &Block::Lines(vec![
                "void main()".into(),
                "{".into(),
                "   object oChair = OBJECT_SELF;".into(),
                "   AssignCommand(GetLastUsedBy(),ActionSit(oChair));".into(),
                "}".into(),
            ]),
        );
        let sit = blocks_of(&all, "ActionSit");
        assert!(
            matches!(&sit[0], Block::Prose(p) if p.ends_with("marked as usable in the toolset.")),
            "{sit:#?}"
        );
        has(
            "DelayCommand",
            &Block::Lines(vec![
                "effect eDamage = EffectDamage(nDamage, DAMAGE_TYPE_MAGICAL);".into(),
                "DelayCommand(fDelay, ApplyEffectToObject(DURATION_TYPE_INSTANT, eDamage, oTarget);"
                    .into(),
            ]),
        );
        has(
            "GetStandardFactionReputation",
            &Block::Lines(vec![
                "0-10   :  Hostile.".into(),
                "11-89  :  Neutral.".into(),
                "90-100 :  Friendly.".into(),
            ]),
        );
        let hit = blocks_of(&all, "ItemPropertyOnHitProps");
        let columns = hit.iter().find_map(|b| match b {
            Block::Lines(l) if l[0].contains("ABILITYDRAIN") => Some(l.clone()),
            _ => None,
        });
        let columns = columns.unwrap_or_else(|| panic!("{hit:#?}"));
        assert!(columns.len() > 10 && columns[1].trim() == "constant(IP_CONST_ABILITY_*)");
    }

    /// No documented function or constant gives a monospace block of one
    /// line that is neither in columns nor code: a sentence wrapped in the
    /// source, cut from its paragraph and set apart.
    #[test]
    fn no_sentence_of_nwscript_s_is_set_apart_as_a_line() {
        let root = mg_testkit::corpus!();
        let all = documented(root);
        let mut stray = Vec::new();
        for (name, sig, doc) in &all {
            for b in doc_blocks_with(doc, &param_names(sig)) {
                if let Block::Lines(lines) = &b
                    && lines.len() == 1
                    && !in_columns(&lines[0])
                    && !code_like(&lines[0])
                {
                    stray.push(format!("{name}: {:?}", lines[0]));
                }
            }
        }
        assert!(all.len() > 1000, "{} documented", all.len());
        assert!(stray.is_empty(), "{} set apart:\n{}", stray.len(), stray.join("\n"));
    }
}
