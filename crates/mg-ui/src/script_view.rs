//! The script editor: syntax-highlighted text, save, and compile.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Ui};
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_resman::ResKey;
use mg_script::Compiler;
use mg_script::lex::{TokenKind, tokenize};

use crate::script_tools::{
    self as tools, Completion, InfoTab, Message, SideTab, Symbol, SymbolKind,
};
use crate::settings::ScriptStyle;
use crate::text::{decode, encode};
use crate::{Action, Moonglow, Tab};

/// A script's text in its editor.
#[derive(Debug, Clone)]
pub(crate) struct ScriptBuffer {
    pub(crate) text: String,
    /// The text as last loaded or saved.
    pub(crate) saved: String,
    /// The module's bytes the text was last compared with (address and
    /// length), to decode them again only when they change.
    source: (usize, usize),
    /// Bookmarked lines (0-based).
    pub(crate) bookmarks: std::collections::BTreeSet<usize>,
    /// Numbered bookmarks 1 to 9 (index 0 unused): Ctrl+Shift+N sets one,
    /// Ctrl+N goes there.
    pub(crate) numbered: [Option<usize>; 10],
    /// The file the external script editor has open, and when it was last
    /// written: the text is read back whenever it changes.
    pub(crate) external: Option<(std::path::PathBuf, Option<std::time::SystemTime>)>,
}

/// The laid-out text of a script editor, reused while the text, width and
/// colours stay the same (a large script takes milliseconds to highlight and
/// lay out).
#[derive(Debug, Clone)]
pub(crate) struct LaidOut {
    text: String,
    wrap: f32,
    palette: Palette,
    /// The line underlined as an error.
    error_line: Option<usize>,
    galley: std::sync::Arc<egui::Galley>,
}

/// The room a side list's row takes beside its name: the scroll bar and
/// the panel's and the row's margins.
const SIDE_MARGINS: f32 = 44.0;

/// The longest name the side lists open wide enough for, characters.
const LONG_NAME: usize = 44;

/// Underlines (as editors mark errors) what is on a 0-based line.
fn underline_line(job: &mut LayoutJob, text: &str, line: usize, color: Color32) {
    let start = if line == 0 {
        0
    } else {
        text.match_indices('\n').nth(line - 1).map_or(text.len(), |(i, _)| i + 1)
    };
    let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    for s in &mut job.sections {
        let (a, b) = (usize::from(s.byte_range.start), usize::from(s.byte_range.end));
        let overlaps = a < end && b > start;
        if overlaps && !text[a..b].trim().is_empty() {
            s.format.underline = egui::Stroke::new(1.5, color);
        }
    }
}

impl ScriptBuffer {
    pub(crate) fn is_dirty(&self) -> bool {
        self.text != self.saved
    }

    /// A buffer as an editor would hold it: `text` typed over `saved`.
    #[cfg(test)]
    pub(crate) fn for_tests(text: String, saved: String) -> ScriptBuffer {
        ScriptBuffer {
            text,
            saved,
            source: (0, 0),
            bookmarks: Default::default(),
            numbered: [None; 10],
            external: None,
        }
    }
}

/// The script editor's colours (one per [`SCRIPT_ELEMENTS`](crate::settings::SCRIPT_ELEMENTS))
/// and font size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Palette {
    pub(crate) colors: [Color32; 8],
    pub(crate) size: f32,
}

impl Palette {
    /// The theme's colours for each element.
    pub(crate) fn defaults(dark: bool, text: Color32) -> [Color32; 8] {
        let rgb = Color32::from_rgb;
        if dark {
            [
                text,
                rgb(106, 153, 85),
                rgb(197, 134, 192),
                rgb(86, 156, 214),
                rgb(206, 145, 120),
                rgb(206, 145, 120),
                rgb(79, 193, 255),
                Color32::RED,
            ]
        } else {
            [
                text,
                rgb(0, 128, 0),
                rgb(128, 0, 128),
                rgb(0, 0, 200),
                rgb(163, 21, 21),
                rgb(163, 21, 21),
                rgb(0, 112, 193),
                Color32::RED,
            ]
        }
    }

    /// The colours of a style: its own where set, else the theme's.
    pub(crate) fn new(style: &ScriptStyle, dark: bool, text: Color32) -> Palette {
        let mut colors = Palette::defaults(dark, text);
        for (c, own) in colors.iter_mut().zip(style.colors) {
            if let Some([r, g, b]) = own {
                *c = Color32::from_rgb(r, g, b);
            }
        }
        Palette { colors, size: f32::from(style.font_size.clamp(6, 48)) }
    }

    /// For a `Ui`: the app's style and the `Ui`'s theme.
    pub(crate) fn for_ui(style: &ScriptStyle, ui: &Ui) -> Palette {
        Palette::new(style, ui.visuals().dark_mode, ui.visuals().text_color())
    }

    fn font(&self) -> FontId {
        FontId::monospace(self.size)
    }
}

/// The element a token is coloured as.
fn element(kind: TokenKind) -> usize {
    match kind {
        TokenKind::LineComment | TokenKind::BlockComment { .. } => 1,
        TokenKind::Directive => 2,
        TokenKind::Keyword => 3,
        TokenKind::Int | TokenKind::Float => 4,
        TokenKind::String { .. } | TokenKind::RawString { .. } | TokenKind::HashedString { .. } => {
            5
        }
        TokenKind::BuiltinConstant => 6,
        TokenKind::Unknown => 7,
        _ => 0,
    }
}

/// Lays the text out with token colours.
pub(crate) fn highlight(text: &str, palette: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font = palette.font();
    for t in tokenize(text.as_bytes()) {
        let c = palette.colors[element(t.kind)];
        // Tokens end on character boundaries except inside malformed UTF-8,
        // which the text (a String) cannot contain.
        job.append(&text[t.span], 0.0, TextFormat::simple(font.clone(), c));
    }
    job
}

/// Text for HTML.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A script as an HTML page that prints itself (Print: the browser's print
/// dialog), highlighted in the editor's colours on white paper.
pub(crate) fn print_html(name: &str, text: &str, style: &crate::settings::ScriptStyle) -> String {
    use std::fmt::Write;
    let job = highlight(text, &Palette::new(style, false, Color32::BLACK));
    let mut body = String::new();
    for s in &job.sections {
        let c = s.format.color;
        let t = escape(&job.text[s.byte_range.start.0..s.byte_range.end.0]);
        let _ = write!(
            body,
            "<span style=\"color:#{:02x}{:02x}{:02x}\">{t}</span>",
            c.r(),
            c.g(),
            c.b()
        );
    }
    let n = escape(name);
    format!(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>{n}</title>\
         <style>body{{font-family:monospace;font-size:10pt}}pre{{white-space:pre-wrap}}</style>\
         </head><body onload=\"window.print()\"><h3>{n}</h3><pre>{body}</pre></body></html>\n"
    )
}

/// A file's `file://` URL.
pub(crate) fn file_url(path: &std::path::Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    let p = p.replace('%', "%25").replace(' ', "%20").replace('#', "%23");
    if p.starts_with('/') { format!("file://{p}") } else { format!("file:///{p}") }
}

fn editor_id(key: ResKey) -> egui::Id {
    egui::Id::new(("script", key))
}

/// Moves an editor's cursor (and selection) and focuses it.
fn set_cursor(ctx: &egui::Context, key: ResKey, from: usize, to: usize) {
    let id = editor_id(key);
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
        egui::text::CCursor::new(from),
        egui::text::CCursor::new(to),
    )));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

/// The editor's cursor as a character range (start, end).
fn cursor(ctx: &egui::Context, key: ResKey) -> Option<(usize, usize)> {
    let state = egui::text_edit::TextEditState::load(ctx, editor_id(key))?;
    let r = state.cursor.char_range()?;
    let (a, b) = (r.primary.index, r.secondary.index);
    let (a, b): (usize, usize) = (a.into(), b.into());
    Some((a.min(b), a.max(b)))
}

/// Replaces a character range of the buffer and puts the cursor after it.
fn insert(
    buf: &mut ScriptBuffer,
    ctx: &egui::Context,
    key: ResKey,
    from: usize,
    to: usize,
    text: &str,
) {
    let (bf, bt) = (tools::byte_index(&buf.text, from), tools::byte_index(&buf.text, to));
    buf.text.replace_range(bf..bt, text);
    let at = from + text.chars().count();
    set_cursor(ctx, key, at, at);
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    // Fit the pane, whose clip rectangle is its true size: the tab does not
    // scroll (`Viewer::scroll_bars`), and the dock's non-scrolling scroll area
    // grows to the size its content had before (so the side lists ran past
    // the pane's edge). A child area, as a Ui never shrinks below its size.
    let margin = ui.max_rect().left() - ui.clip_rect().left();
    let mut fit = ui.max_rect();
    fit.max.x = fit.max.x.min(ui.clip_rect().max.x - margin).max(fit.min.x + 120.0);
    fit.max.y = fit.max.y.min(ui.clip_rect().max.y - margin).max(fit.min.y + 120.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(fit), |ui| editor(app, ui, key));
}

fn editor(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    // The external editor's saves, if it has the script (looked for twice a
    // second).
    reload_external(app, key);
    if app.scripts.get(&key).is_some_and(|b| b.external.is_some()) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
    let Some(ws) = &app.ws else { return };
    let Some(bytes) = ws.module.get(&key) else {
        ui.label(format!("{key} is no longer in the module."));
        return;
    };
    let source = (bytes.as_ptr() as usize, bytes.len());
    app.scripts.entry(key).or_insert_with(|| {
        let text = decode(bytes);
        ScriptBuffer {
            text: text.clone(),
            saved: text,
            source,
            bookmarks: Default::default(),
            numbered: [None; 10],
            external: None,
        }
    });
    if app.script_tools.open_externally.remove(&key)
        && let Some(editor) = app.settings.external_editor.clone()
    {
        open_external(app, key, &editor);
    }
    let Some(ws) = &app.ws else { return };
    let Some(bytes) = ws.module.get(&key) else { return };
    {
        let buf = app.scripts.get_mut(&key).expect("just inserted");
        if buf.source != source {
            buf.source = source;
            let module_text = decode(bytes);
            if !buf.is_dirty() && buf.saved != module_text {
                // Changed underneath (undo, import): follow the module.
                buf.text = module_text.clone();
                buf.saved = module_text;
            }
        }
    }
    let ctx = ui.ctx().clone();

    // The symbols the script can use (its own, its includes', nwscript's).
    let symbols = {
        let text = &app.scripts[&key].text;
        let (game, ws) = (&app.game, &app.ws);
        let mut source = |name: &str| -> Option<String> {
            let k = tools::nss(name)?;
            if let Some(b) = app.scripts.get(&k) {
                return Some(b.text.clone());
            }
            ws.as_ref()
                .and_then(|w| w.module.get(&k).map(decode))
                .or_else(|| game.as_ref()?.resman.get(&k).ok().map(|d| decode(&d)))
        };
        app.script_tools.symbols_for(
            key,
            text,
            || {
                let g = game.as_ref()?;
                let data = g.resman.get_named("nwscript", ResType::NSS).ok()?;
                Some(mg_script::spec::Spec::parse(&data))
            },
            &mut source,
        )
    };

    // Shortcuts while the editor has focus.
    let focused = ctx.memory(|m| m.has_focus(editor_id(key)));
    let mut open_find = false;
    let mut open_replace = false;
    let mut find_next = false;
    let mut toggle_bookmark = false;
    let mut complete = false;
    let mut set_numbered = None;
    let mut go_numbered = None;
    let (mut definition, mut references, mut rename) = (false, false, false);
    if focused {
        use crate::keys::Cmd;
        use egui::{Key, KeyboardShortcut, Modifiers};
        let pressed = |m, k| ui.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        // The keys of Options › Keyboard.
        let keys = &app.keymap;
        let cmd = |c: Cmd| ui.input_mut(|i| keys.consume(i, c));
        rename = cmd(Cmd::RenameSymbol);
        open_find = cmd(Cmd::Find);
        open_replace = cmd(Cmd::Replace);
        find_next = cmd(Cmd::FindNext);
        toggle_bookmark = cmd(Cmd::Bookmark);
        complete = cmd(Cmd::Complete);
        references = cmd(Cmd::References);
        definition = cmd(Cmd::Definition);
        const DIGITS: [Key; 9] = [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
        ];
        for (i, k) in DIGITS.into_iter().enumerate() {
            if pressed(Modifiers::COMMAND | Modifiers::SHIFT, k) {
                set_numbered = Some(i + 1);
            } else if pressed(Modifiers::COMMAND, k) {
                go_numbered = Some(i + 1);
            }
        }
    }

    let mut save = false;
    let mut compile = false;
    let mut save_as = false;
    let mut to_scratch = false;
    let dirty = app.scripts[&key].is_dirty();
    // Wrapping: a narrow pane keeps its width (the row would widen the
    // editor past the pane, side lists and all).
    ui.horizontal_wrapped(|ui| {
        save = ui
            .add_enabled(dirty, egui::Button::new("Save"))
            .on_hover_text("Save the script into the module")
            .clicked();
        compile = ui
            .button("Compile")
            .on_hover_text("Save and compile (F7 compiles all scripts)")
            .clicked();
        save_as = ui.button("Save As…").clicked();
        if ui.button("Print…").on_hover_text("Print the script (through the web browser)").clicked()
        {
            let text = app.scripts[&key].text.clone();
            let html =
                print_html(&format!("{}.nss", key.resref), &text, &app.settings.script_style);
            let path = app.print_dir.join(format!("{}.html", key.resref));
            match std::fs::create_dir_all(&app.print_dir).and_then(|()| std::fs::write(&path, html))
            {
                Ok(()) => ui.ctx().open_url(egui::OpenUrl::new_tab(file_url(&path))),
                Err(e) => app.log.error(format!("Print: {e}")),
            }
        }
        if ui
            .button("Open…")
            .on_hover_text(
                "Open another script: the module's, its haks' or the game's, found by name",
            )
            .clicked()
        {
            open_script_window(app);
        }
        let editor = app.settings.external_editor.clone();
        if ui
            .add_enabled(editor.is_some(), egui::Button::new("External Editor"))
            .on_hover_text("Edit in the external script editor (Options › Script Editor)")
            .on_disabled_hover_text("Choose an external script editor in Options › Script Editor")
            .clicked()
            && let Some(editor) = editor
        {
            open_external(app, key, &editor);
        }
        let tip = crate::transfer::scratch_tip(
            app.settings.scratch_dir.as_deref(),
            "the script, saved and compiled, with its compiled script",
        );
        to_scratch = ui.button("To Scratch").on_hover_text(tip).clicked();
        ui.separator();
        use crate::keys::Cmd;
        let keymap = &app.keymap;
        let ctx = ui.ctx().clone();
        let tip = |text: &str, cmd: Cmd| keymap.titled(text, cmd, &ctx);
        let find_tip = format!("{}; {}", tip("Find", Cmd::Find), tip("Find Next", Cmd::FindNext));
        open_find |= ui.button("Find…").on_hover_text(find_tip).clicked();
        open_replace |= ui.button("Replace…").on_hover_text(tip("Replace", Cmd::Replace)).clicked();
        toggle_bookmark |= ui
            .button("Bookmark")
            .on_hover_text(tip("Toggle a bookmark on the cursor's line", Cmd::Bookmark))
            .clicked();
        ui.separator();
        definition |= ui
            .button("Definition")
            .on_hover_text(format!(
                "{}, or Ctrl+click",
                tip("Go to the definition of the name at the cursor", Cmd::Definition)
            ))
            .clicked();
        references |= ui
            .button("References")
            .on_hover_text(tip(
                "Where the name at the cursor is used in the module's scripts",
                Cmd::References,
            ))
            .clicked();
        rename |= ui
            .button("Rename Symbol…")
            .on_hover_text(tip(
                "Rename the name at the cursor everywhere it's used",
                Cmd::RenameSymbol,
            ))
            .clicked();
        if ui
            .button("Used By")
            .on_hover_text("Where the module runs or includes this script")
            .clicked()
        {
            app.actions.push(Action::FindReferences(key));
        }
        if dirty {
            ui.weak("modified");
        }
    });
    // The error the latest check found, under the toolbar.
    app.live_check(key, &ctx);
    if let Some((line, text)) = app.live_error(key) {
        let at = line.map_or_else(|| "in an include".to_string(), |l| format!("line {}", l + 1));
        ui.colored_label(ui.visuals().error_fg_color, format!("⚠ {at}: {text}"))
            .on_hover_text("Found as you type (the script is compiled when typing pauses)");
    }
    ui.separator();

    if definition || references || rename {
        let at = cursor(&ctx, key).map_or(0, |c| c.0);
        match app.declaration_at(key, at) {
            Some(d) if definition => app.go_to_declaration(&d),
            Some(d) if references => app.show_references(&d),
            Some(d) => {
                let name = d.name.clone();
                app.script_nav.rename = Some((d, name));
            }
            None => app.log.info("No declaration for the name at the cursor"),
        }
    }

    let tools_state = &mut app.script_tools;
    if open_find || open_replace {
        let s = &mut tools_state.search;
        s.open = true;
        s.replace_mode = open_replace;
        s.script = Some(key);
        if let Some((a, b)) = cursor(&ctx, key)
            && a != b
        {
            let t = &app.scripts[&key].text;
            s.find = t.chars().skip(a).take(b - a).collect();
        }
    }
    if find_next {
        tools_state.search.script = Some(key);
        find_in_script(app, &ctx, key);
    }
    if toggle_bookmark {
        let at = cursor(&ctx, key).map_or(0, |c| c.0);
        let buf = app.scripts.get_mut(&key).expect("open");
        let line = tools::line_of(&buf.text, at);
        if !buf.bookmarks.remove(&line) {
            buf.bookmarks.insert(line);
        }
        app.script_tools.info = InfoTab::Bookmarks;
    }
    if let Some(n) = set_numbered {
        let at = cursor(&ctx, key).map_or(0, |c| c.0);
        let buf = app.scripts.get_mut(&key).expect("open");
        let line = tools::line_of(&buf.text, at);
        // Setting a number again on its line clears it.
        buf.numbered[n] = if buf.numbered[n] == Some(line) { None } else { Some(line) };
        app.script_tools.info = InfoTab::Bookmarks;
    }
    if let Some(line) = go_numbered.and_then(|n| app.scripts[&key].numbered[n]) {
        let at = tools::line_start(&app.scripts[&key].text, line);
        app.script_tools.jump = Some((key, at));
    }
    if complete && let Some((at, _)) = cursor(&ctx, key) {
        let (start, prefix) = tools::word_before(&app.scripts[&key].text, at);
        let items = tools::completions(&symbols, &prefix);
        if !items.is_empty() {
            app.script_tools.completion =
                Some(Completion { script: key, start, items, selected: 0 });
        }
    }

    // Side lists.
    let mut insert_text: Option<String> = None;
    // At most half the tab, so a narrow tab keeps room for the text (and the
    // lists stay inside it).
    let half = (ui.available_width() / 2.0).max(120.0);
    // Wide enough, to begin with, for the longest name the lists have (with
    // the scroll bar and the margins beside it): but for the few constants
    // longer than any function's name (`PLAYER_DEVICE_PROPERTY_…`), which
    // would take half the window; those end in "…" until the panel is
    // widened.
    // (Measured: the longest few, since letters differ in width.)
    let mut longest: Vec<&str> = symbols
        .iter()
        .map(|s| s.name.as_str())
        .filter(|n| n.chars().count() <= LONG_NAME)
        .collect();
    longest.sort_unstable_by_key(|n| std::cmp::Reverse(n.len()));
    let font = egui::TextStyle::Body.resolve(ui.style());
    let widest = longest.iter().take(16).fold(0.0_f32, |widest, name| {
        let text = (*name).to_owned();
        let galley = ui.fonts_mut(|f| f.layout_no_wrap(text, font.clone(), egui::Color32::WHITE));
        widest.max(galley.size().x + SIDE_MARGINS)
    });
    egui::Panel::right(egui::Id::new(("script-side", key)))
        .resizable(true)
        .default_size(widest.max(260.0).min(half))
        .max_size(half)
        .show(ui, |ui| {
            let t = &mut app.script_tools;
            ui.horizontal_wrapped(|ui| {
                for (tab, label) in [
                    (SideTab::Functions, "Functions"),
                    (SideTab::Variables, "Variables"),
                    (SideTab::Constants, "Constants"),
                    (SideTab::Templates, "Templates"),
                ] {
                    ui.selectable_value(&mut t.side, tab, label);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.text_edit_singleline(&mut t.filter);
            });
            let filter = t.filter.to_ascii_lowercase();
            let row = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
            // Long names end in "…" in a narrow panel (Help gives them whole).
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            if t.side == SideTab::Templates {
                let templates = tools::templates(
                    app.install.as_ref(),
                    app.settings.script_templates.as_deref(),
                );
                // (As wide as the panel: the scroll bar stays at its edge.)
                let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
                area.id_salt("templates").show(ui, |ui| {
                    for (name, path) in
                        templates.iter().filter(|(n, _)| n.to_ascii_lowercase().contains(&filter))
                    {
                        if ui
                            .selectable_label(false, name)
                            .on_hover_text("Double-click to insert")
                            .double_clicked()
                        {
                            insert_text = std::fs::read(path).ok().map(|b| decode(&b));
                        }
                    }
                });
            } else {
                let kind = match t.side {
                    SideTab::Functions => SymbolKind::Function,
                    SideTab::Variables => SymbolKind::Variable,
                    _ => SymbolKind::Constant,
                };
                let mut list: Vec<&Symbol> = symbols
                    .iter()
                    .filter(|s| {
                        s.kind == kind
                            && (filter.is_empty() || s.name.to_ascii_lowercase().contains(&filter))
                    })
                    .collect();
                list.sort_by(|a, b| a.name.cmp(&b.name));
                let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
                area.id_salt(("symbols", kind)).show_rows(ui, row, list.len(), |ui, range| {
                    // (Each row as wide as the list, so that its
                    // width doesn't change with the names in view.)
                    ui.set_min_width(ui.available_width());
                    for s in &list[range] {
                        let text = if s.custom {
                            egui::RichText::new(&s.name).strong()
                        } else {
                            egui::RichText::new(&s.name)
                        };
                        let r = ui
                            .selectable_label(
                                t.help.as_ref().is_some_and(|h| h.name == s.name),
                                text,
                            )
                            .on_hover_text(&s.signature);
                        if r.clicked() {
                            t.help = Some((*s).clone());
                            t.info = InfoTab::Help;
                        }
                        if r.double_clicked() {
                            insert_text = Some(if s.kind == SymbolKind::Function {
                                format!("{}(", s.name)
                            } else {
                                s.name.clone()
                            });
                        }
                    }
                });
            }
        });

    // Compiler, Help, Bookmarks, Search Results.
    let mut jump: Option<(ResKey, usize)> = None;
    let mut open_other: Option<(String, usize)> = None;
    // (At most three fifths of the editor: the text keeps its room.)
    let most = (ui.available_height() * 0.6).max(60.0);
    egui::Panel::bottom(egui::Id::new(("script-info", key)))
        .resizable(true)
        .default_size(140.0f32.min(most))
        .max_size(most)
        .show(ui, |ui| {
            let t = &mut app.script_tools;
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (InfoTab::Compiler, "Compiler"),
                    (InfoTab::Help, "Help"),
                    (InfoTab::Bookmarks, "Bookmarks"),
                    (InfoTab::SearchResults, "Search Results"),
                ] {
                    ui.selectable_value(&mut t.info, tab, label);
                }
            });
            egui::ScrollArea::vertical()
                .id_salt("script-info-rows")
                .auto_shrink([false, false])
                .show(ui, |ui| match t.info {
                    InfoTab::Compiler => {
                        for m in t.messages.iter().filter(|m| m.script == key) {
                            let color = if m.error {
                                ui.visuals().error_fg_color
                            } else {
                                ui.visuals().text_color()
                            };
                            let r = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&m.text).monospace().color(color),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if r.on_hover_text("Click to go to the line").clicked()
                                && let Some((file, line)) = &m.location
                            {
                                open_other = Some((file.clone(), *line));
                            }
                        }
                    }
                    InfoTab::Help => match &t.help {
                        Some(h) => {
                            ui.monospace(&h.signature);
                            if !h.doc.is_empty() {
                                ui.label(&h.doc);
                            }
                        }
                        None => {
                            ui.weak("Select a function or constant, or put the cursor on one.");
                        }
                    },
                    InfoTab::Bookmarks => {
                        let buf = &app.scripts[&key];
                        for (n, line) in buf.numbered.iter().enumerate() {
                            let Some(line) = line else { continue };
                            let text = buf.text.lines().nth(*line).unwrap_or_default().trim();
                            if ui
                                .selectable_label(false, format!("[{n}] {:>5}: {text}", line + 1))
                                .on_hover_text(format!("Ctrl+{n}"))
                                .clicked()
                            {
                                jump = Some((key, tools::line_start(&buf.text, *line)));
                            }
                        }
                        for line in &buf.bookmarks {
                            let text = buf.text.lines().nth(*line).unwrap_or_default().trim();
                            if ui
                                .selectable_label(false, format!("{:>5}: {text}", line + 1))
                                .clicked()
                            {
                                jump = Some((key, tools::line_start(&buf.text, *line)));
                            }
                        }
                    }
                    InfoTab::SearchResults => {
                        for (k, line, text) in &t.search.results {
                            if ui
                                .selectable_label(
                                    false,
                                    format!("{}({}): {text}", k.resref, line + 1),
                                )
                                .clicked()
                            {
                                open_other = Some((k.resref.to_string(), line + 1));
                            }
                        }
                    }
                });
        });

    // The editor with line numbers.
    let palette = Palette::for_ui(&app.settings.script_style, ui);
    let error_line = app.live_error(key).and_then(|e| e.0);
    let error_color = ui.visuals().error_fg_color;
    let mut ctrl_click = None;
    let Moonglow { scripts, laid_out, script_tools, .. } = app;
    let buf = scripts.get_mut(&key).expect("open");
    if let Some(text) = insert_text {
        let (a, b) =
            cursor(&ctx, key).unwrap_or((buf.text.chars().count(), buf.text.chars().count()));
        insert(buf, &ctx, key, a, b, &text);
    }
    // Completion keys go to the list, not the text.
    let mut accept = None;
    if let Some(c) = script_tools.completion.as_mut().filter(|c| c.script == key) {
        use egui::{Key, Modifiers};
        let n = c.items.len();
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::NONE, Key::ArrowDown) {
                c.selected = (c.selected + 1) % n;
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowUp) {
                c.selected = (c.selected + n - 1) % n;
            }
            if i.consume_key(Modifiers::NONE, Key::Enter)
                || i.consume_key(Modifiers::NONE, Key::Tab)
            {
                accept = Some(c.selected);
            }
        });
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            script_tools.completion = None;
        }
    }
    if let Some(i) = accept
        && let Some(c) = script_tools.completion.take()
    {
        let s = &c.items[i];
        let text =
            if s.kind == SymbolKind::Function { format!("{}(", s.name) } else { s.name.clone() };
        let at = cursor(&ctx, key).map_or(c.start, |x| x.0);
        insert(buf, &ctx, key, c.start, at, &text);
    }
    // Tab on a selection of several lines indents them; Shift+Tab outdents
    // the selected lines, or the cursor's.
    if script_tools.completion.is_none()
        && ui.memory(|m| m.has_focus(editor_id(key)))
        && let Some((a, b)) = cursor(&ctx, key)
    {
        use egui::{Key, Modifiers};
        let lines = buf.text.chars().skip(a).take(b - a).any(|c| c == '\n');
        let outdent = ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Tab));
        let indent =
            !outdent && lines && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Tab));
        if outdent || indent {
            let (text, from, to) = tools::indent_lines(&buf.text, a, b, outdent);
            buf.text = text;
            set_cursor(&ctx, key, from, to);
        }
    }
    let pending = script_tools.jump.take_if(|(k, _)| *k == key).or(jump).map(|(_, at)| at);
    if let Some(at) = pending {
        set_cursor(&ctx, key, at, at);
        script_tools.scroll = Some(key);
    }
    let cached = laid_out.entry(key).or_insert(None);
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap: f32| {
        if let Some(c) = cached.as_ref().filter(|c| {
            c.wrap == wrap
                && c.palette == palette
                && c.error_line == error_line
                && c.text == text.as_str()
        }) {
            return c.galley.clone();
        }
        // Code does not wrap (the editor scrolls sideways), which also
        // keeps the line numbers beside their lines.
        let mut job = highlight(text.as_str(), &palette);
        job.wrap.max_width = f32::INFINITY;
        if let Some(line) = error_line {
            underline_line(&mut job, text.as_str(), line, error_color);
        }
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        *cached = Some(LaidOut {
            text: text.as_str().to_string(),
            wrap,
            palette,
            error_line,
            galley: galley.clone(),
        });
        galley
    };
    let lines = buf.text.split('\n').count();
    let numbers: String = (1..=lines)
        .map(|n| {
            if let Some(b) = buf.numbered.iter().position(|l| *l == Some(n - 1)) {
                format!("{b}{n:>5}\n")
            } else if buf.bookmarks.contains(&(n - 1)) {
                format!("◆{n:>5}\n")
            } else {
                format!("{n:>6}\n")
            }
        })
        .collect();
    let mut completion_at = None;
    egui::ScrollArea::both().id_salt(("script-scroll", key)).auto_shrink([false, false]).show(
        ui,
        |ui| {
            // The editor fills the room down to the pane under it, however
            // short the script.
            let room = ui.available_height();
            ui.horizontal_top(|ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(numbers).font(palette.font()).weak())
                        .selectable(false),
                );
                let out = egui::TextEdit::multiline(&mut buf.text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(1)
                    .min_size(egui::vec2(0.0, room))
                    .id(editor_id(key))
                    .layouter(&mut layouter)
                    .show(ui);
                if let Some(r) = out.cursor_range {
                    let rect =
                        out.galley.pos_from_cursor(r.primary).translate(out.galley_pos.to_vec2());
                    if script_tools.scroll == Some(key) {
                        ui.scroll_to_rect(rect.expand(40.0), Some(egui::Align::Center));
                        script_tools.scroll = None;
                    }
                    completion_at = Some(rect.left_bottom());
                    // Help follows the identifier under the cursor.
                    if out.response.response.changed() || out.response.response.clicked() {
                        let word = tools::word_at(&buf.text, r.primary.index.into());
                        if let Some(s) = word.and_then(|w| symbols.iter().find(|s| s.name == w)) {
                            script_tools.help = Some(s.clone());
                            // A double click on it brings its help forward
                            // (over the compiler's messages, say), as in
                            // Aurora.
                            if out.response.response.double_clicked() {
                                script_tools.info = InfoTab::Help;
                            }
                        }
                    }
                    if out.response.response.changed() {
                        script_tools.completion = None;
                    }
                    // Ctrl+click goes to the definition.
                    if out.response.response.clicked() && ctx.input(|i| i.modifiers.command) {
                        ctrl_click = Some(usize::from(r.primary.index));
                    }
                }
            });
        },
    );

    // The completion list at the cursor.
    if let (Some(c), Some(pos)) =
        (script_tools.completion.as_mut().filter(|c| c.script == key), completion_at)
    {
        let mut pick = None;
        egui::Area::new(egui::Id::new(("completion", key)))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (i, s) in c.items.iter().enumerate() {
                            let r = ui
                                .selectable_label(i == c.selected, &s.name)
                                .on_hover_text(&s.signature);
                            if i == c.selected {
                                r.scroll_to_me(None);
                            }
                            if r.clicked() {
                                pick = Some(i);
                            }
                        }
                    });
                });
            });
        if let Some(i) = pick {
            let s = c.items[i].clone();
            let start = c.start;
            script_tools.completion = None;
            let text = if s.kind == SymbolKind::Function { format!("{}(", s.name) } else { s.name };
            let at = cursor(&ctx, key).map_or(start, |x| x.0);
            insert(buf, &ctx, key, start, at, &text);
        }
    }

    // A compiler message or search result in another script opens it.
    if let Some((file, line)) = open_other {
        match tools::nss(&file) {
            Some(k) if k == key => {
                let at = tools::line_start(&buf.text, line.saturating_sub(1));
                script_tools.jump = Some((key, at));
            }
            Some(k) if app.ws.as_ref().is_some_and(|w| w.module.contains(&k)) => {
                let text = app
                    .scripts
                    .get(&k)
                    .map(|b| b.text.clone())
                    .or_else(|| app.ws.as_ref()?.module.get(&k).map(decode))
                    .unwrap_or_default();
                app.script_tools.jump = Some((k, tools::line_start(&text, line.saturating_sub(1))));
                app.actions.push(Action::OpenTab(Tab::Script(k)));
            }
            Some(k) => app.actions.push(Action::OpenTab(Tab::Resource(k))),
            None => {}
        }
    }

    if let Some(at) = ctrl_click
        && let Some(d) = app.declaration_at(key, at)
    {
        app.go_to_declaration(&d);
    }

    if save_as {
        app.script_save_as = Some((key, String::new()));
    }
    if save || compile || to_scratch {
        let buf = app.scripts.get_mut(&key).expect("open");
        let text = buf.text.clone();
        if buf.is_dirty() {
            buf.saved = text.clone();
            app.actions.push(Action::Apply(Command::new(
                format!("Edit {key}"),
                vec![Edit::SetResource { key, data: Some(encode(&text)) }],
            )));
        }
        // Options > Script Editor: saving compiles too.
        let compiled =
            (compile || to_scratch || app.settings.auto_compile) && compile_one(app, key, &text);
        // (After the text is the module's: the scripts that include it.)
        if compile || app.settings.auto_compile {
            app.actions.push(Action::CompileIncluders(key));
        }
        // To Scratch: what was just compiled (nothing, if it failed: the
        // folder keeps what it has).
        if to_scratch && compiled {
            app.actions.push(Action::ExportFiles {
                keys: vec![key],
                dependencies: false,
                scratch: true,
            });
        }
    }
}

/// The file of a nasher project or a module folder that holds `key`'s
/// source, where the external editor can work on it in place: its own file, as
/// long as Moonglow has nothing of the script that the file lacks (text
/// typed and not saved, or a change not yet written to the project).
fn project_file(app: &Moonglow, key: ResKey) -> Option<std::path::PathBuf> {
    let ws = app.ws.as_ref()?;
    let path = ws.module.file_of(&key)?.to_path_buf();
    let buf = app.scripts.get(&key)?;
    let on_disk = std::fs::read(&path).ok()?;
    (!buf.is_dirty() && decode(&on_disk) == buf.text).then_some(path)
}

/// Opens the script in the external editor: a nasher project's own source
/// file, in place ([`project_file`]); else a scratch copy. The editor's
/// saves come back into the buffer ([`reload_external`]), or, for a
/// project's file, as its files changed outside do (Options › General).
fn open_external(app: &mut Moonglow, key: ResKey, editor: &std::path::Path) {
    let in_place = project_file(app, key);
    let in_project =
        app.ws.as_ref().is_some_and(|ws| ws.module.file_of(&key).is_some()) && in_place.is_none();
    let reloads = !app.settings.no_auto_reload;
    let Some(buf) = app.scripts.get_mut(&key) else { return };
    let path = match in_place {
        Some(path) => path,
        None => {
            let dir = std::env::temp_dir().join(format!("moonglow-{}", std::process::id()));
            let path = dir.join(format!("{}.nss", key.resref.to_lowercase()));
            let written = std::fs::create_dir_all(&dir)
                .and_then(|()| std::fs::write(&path, encode(&buf.text)));
            if let Err(e) = written {
                app.log.error(format!("{}: {e}", path.display()));
                return;
            }
            if in_project {
                app.log.info(format!(
                    "{key}: a copy is opened, not the project's file: Moonglow has changes to \
                     it that the file lacks (save first to edit the file in place)"
                ));
            }
            path
        }
    };
    // (A project's file read again as files changed outside are: not by
    // this buffer as well, which would then differ from the module's.)
    if in_place_of(&path) && reloads {
        buf.external = None;
        match std::process::Command::new(editor).arg(&path).spawn() {
            Ok(running) => {
                crate::test_module::let_run(running);
                app.log.info(format!("{key}: editing {} in {}", path.display(), editor.display()));
            }
            Err(e) => app.log.error(format!("{}: {e}", editor.display())),
        }
        return;
    }
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    buf.external = Some((path.clone(), modified));
    match std::process::Command::new(editor).arg(&path).spawn() {
        Ok(running) => {
            crate::test_module::let_run(running);
            app.log.info(format!("{key}: editing in {}", editor.display()));
        }
        Err(e) => app.log.error(format!("{}: {e}", editor.display())),
    }
}

/// Whether `path` is a file of its own (a project's), not Moonglow's
/// scratch copy.
fn in_place_of(path: &std::path::Path) -> bool {
    !path.starts_with(std::env::temp_dir().join(format!("moonglow-{}", std::process::id())))
}

/// Reads back a script the external editor has saved since.
pub(crate) fn reload_external(app: &mut Moonglow, key: ResKey) {
    let Some(buf) = app.scripts.get_mut(&key) else { return };
    let Some((path, seen)) = &mut buf.external else { return };
    let modified = std::fs::metadata(&*path).and_then(|m| m.modified()).ok();
    if modified.is_none() || modified == *seen {
        return;
    }
    *seen = modified;
    if let Ok(bytes) = std::fs::read(&*path) {
        let text = decode(&bytes);
        if text != buf.text {
            buf.text = text;
            app.log.info(format!("{key}: changed in the external editor"));
        }
    }
}

/// Finds the next match of the Find window's text in a script.
pub(crate) fn find_in_script(app: &mut Moonglow, ctx: &egui::Context, key: ResKey) {
    let Some(buf) = app.scripts.get(&key) else { return };
    let s = &app.script_tools.search;
    let from = match cursor(ctx, key) {
        Some((a, _)) if s.options.backwards => a,
        Some((_, b)) => b,
        None => 0,
    };
    match tools::find(&buf.text, &s.find, from, &s.options) {
        Some((a, b)) => {
            set_cursor(ctx, key, a, b);
            app.script_tools.scroll = Some(key);
        }
        None => app.log.info(format!("{:?} not found in {key}", s.find)),
    }
}

/// Compiles a script's source, resolving includes in the module, then the
/// game.
pub(crate) fn compile_source(
    app: &Moonglow,
    key: ResKey,
    text: &str,
) -> Result<Vec<u8>, mg_script::compiler::CompileError> {
    // (What Moonglow writes itself, a wizard's script: by the built-in
    // compiler.)
    compile_with_debug(app, key, text, false, None).map(|(ncs, _)| ncs)
}

/// [`compile_source`], with the debug information (`.ndb`) if `debug`.
fn compile_with_debug(
    app: &Moonglow,
    key: ResKey,
    text: &str,
    debug: bool,
    external: Option<&mg_module::external_compiler::ExternalCompiler>,
) -> Result<(Vec<u8>, Option<Vec<u8>>), mg_script::compiler::CompileError> {
    let source = encode(text);
    let module = app.ws.as_ref().map(|w| &w.module);
    let resman = app.game.as_deref().map(|g| &g.resman);
    let name = key.resref.to_lowercase().to_string();
    // By the external compiler, when one is chosen (Options › Script
    // Editor): the module's scripts as they are, this one as in the editor.
    if let (Some(external), Some(module)) = (external, module) {
        let mut sources: Vec<(String, &[u8])> = module
            .keys_of(ResType::NSS)
            .filter(|k| **k != key)
            .filter_map(|k| Some((k.resref.to_lowercase().to_string(), module.get(k)?)))
            .collect();
        sources.push((name.clone(), &source));
        let failed = |message: String| mg_script::compiler::CompileError { code: -1, message };
        let made = external
            .compile(&sources, std::slice::from_ref(&name))
            .map_err(|e| failed(format!("External compiler: {e}")))?;
        let Some(made) = made.into_iter().next() else {
            return Err(failed(format!("{name}.nss: the external compiler made nothing")));
        };
        return match made.ncs {
            Some(ncs) => Ok((ncs, made.ndb.filter(|_| debug))),
            None => Err(failed(made.message)),
        };
    }
    let mut c = Compiler::new(|n: &str, t: ResType| {
        if t == ResType::NSS && n.eq_ignore_ascii_case(&name) {
            return Some(source.clone());
        }
        let k = ResKey::parse(n, t)?;
        module
            .and_then(|m| m.get(&k))
            .map(<[u8]>::to_vec)
            .or_else(|| resman?.get(&k).ok().map(|d| d.into_owned()))
    });
    c.set_debug_output(debug);
    c.compile(&name).map(|out| (out.ncs, out.ndb))
}

/// Compiles again, into the module, those of `scripts` whose compiled
/// script isn't what their source compiles to now (changed since, or an
/// include of theirs was), so that what goes out with them is current.
/// Returns the names of those compiled again, and of those that have a
/// compiled script but no longer compile (theirs is older than the
/// source). Without the game's data nothing can be compiled, and nothing
/// is said.
pub(crate) fn compile_stale(app: &mut Moonglow, scripts: &[ResKey]) -> (Vec<String>, Vec<String>) {
    let (mut compiled, mut broken) = (Vec::new(), Vec::new());
    if app.game.is_none() {
        return (compiled, broken);
    }
    let mut edits = Vec::new();
    let external = app.external_compiler();
    for &key in scripts {
        let Some(ws) = app.ws.as_ref() else { break };
        let Some(text) = ws.module.get(&key).map(decode) else { continue };
        let ncs = ResKey::new(key.resref, ResType::NCS);
        let old = ws.module.get(&ncs).map(<[u8]>::to_vec);
        match compile_with_debug(app, key, &text, app.settings.debug_info, external.as_ref()) {
            Ok((new, ndb)) if old.as_deref() != Some(&new[..]) => {
                edits.push(Edit::SetResource { key: ncs, data: Some(new) });
                if let Some(ndb) = ndb {
                    let key = ResKey::new(key.resref, ResType::NDB);
                    edits.push(Edit::SetResource { key, data: Some(ndb) });
                }
                compiled.push(key.resref.to_string());
            }
            Ok(_) => {}
            // (One without a compiled script is an include, or is named
            // by the caller as not compiled.)
            Err(_) if old.is_some() => broken.push(key.resref.to_string()),
            Err(_) => {}
        }
    }
    if !edits.is_empty()
        && let Err(e) = app.apply(Command::new("Compile", edits))
    {
        app.log.error(e.to_string());
    }
    (compiled, broken)
}

/// How many scripts a save (here or by another program) compiles on the
/// window's thread: more would hold it, and are left to Compile All.
pub(crate) const COMPILED_AT_ONCE: usize = 24;

impl Moonglow {
    /// `scripts` and the module's scripts that include any of them,
    /// however indirectly: what a change to `scripts` leaves with an
    /// older compiled script. `None` where that is more than
    /// [`COMPILED_AT_ONCE`], or more than one for an external compiler:
    /// those are compiled as a job instead, off the window's thread (the
    /// log says so).
    pub(crate) fn with_includers(&mut self, scripts: &[ResKey]) -> Option<Vec<ResKey>> {
        let ws = self.ws.as_mut()?;
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return None;
        }
        let mut all = scripts.to_vec();
        all.extend(mg_module::refs::includers(&ws.module, scripts));
        // (An external compiler is run once for each script compiled
        // here, with the module's scripts written out for it each time:
        // more than one go to the job, which runs it once for them all.)
        let slow = self.settings.external_compiler.is_some() && all.len() > 1;
        if all.len() > COMPILED_AT_ONCE || slow {
            self.log.info(format!(
                "{} scripts (those changed and those that include them) are compiled in the \
                 background",
                all.len()
            ));
            self.compile_job(Some(all));
            return None;
        }
        Some(all)
    }

    /// Compiles again the scripts that include `script`, saved or
    /// compiled just now, and names them in the log.
    pub(crate) fn compile_includers(&mut self, script: ResKey) {
        if self.game.is_none() {
            return;
        }
        let Some(mut all) = self.with_includers(&[script]) else { return };
        all.retain(|k| *k != script);
        if all.is_empty() {
            return;
        }
        let (compiled, _) = compile_stale(self, &all);
        if !compiled.is_empty() {
            self.log.info(format!(
                "Compiled, as they include {}: {}",
                script.resref,
                crate::transfer::listed(&compiled)
            ));
        }
        let failed: Vec<String> = all
            .iter()
            .filter(|k| !compiled.contains(&k.resref.to_string()))
            .filter(|k| self.script_fails(**k))
            .map(|k| k.resref.to_string())
            .collect();
        if !failed.is_empty() {
            self.log.error(format!(
                "Did not compile (Compile in the script's editor says why): {}",
                crate::transfer::listed(&failed)
            ));
        }
    }

    /// Whether a script of the module that should compile (it has a
    /// `main` or a `StartingConditional`) doesn't.
    pub(crate) fn script_fails(&mut self, key: ResKey) -> bool {
        let Some(text) = self.ws.as_ref().and_then(|ws| ws.module.get(&key).map(decode)) else {
            return false;
        };
        if self.game.is_none() || !mg_script::outline::has_entry_point(&encode(&text)) {
            return false;
        }
        let external = self.external_compiler();
        compile_with_debug(self, key, &text, false, external.as_ref()).is_err()
    }
}

/// Compiles one script (its text as in the editor) into the module;
/// whether it compiled.
fn compile_one(app: &mut Moonglow, key: ResKey, text: &str) -> bool {
    if app.ws.is_none() {
        return false;
    }
    app.script_tools.messages.retain(|m| m.script != key);
    app.script_tools.info = InfoTab::Compiler;
    // An include file (no `main`, no `StartingConditional`) isn't compiled
    // on its own: it is compiled into the scripts that include it, and
    // may lean on what they bring. (Aurora's build compiles it too.)
    if !mg_script::outline::has_entry_point(&encode(text)) {
        let said = format!("{key}: an include file (no main or StartingConditional), not compiled");
        app.log.info(said.clone());
        app.script_tools.messages.push(Message {
            script: key,
            text: said,
            location: None,
            error: false,
        });
        return false;
    }
    let external = app.external_compiler();
    let result = compile_with_debug(app, key, text, app.settings.debug_info, external.as_ref());
    match result {
        Ok((out, ndb)) => {
            let ncs = ResKey::new(key.resref, ResType::NCS);
            let mut edits = vec![Edit::SetResource { key: ncs, data: Some(out) }];
            if let Some(ndb) = ndb {
                edits.push(Edit::SetResource {
                    key: ResKey::new(key.resref, ResType::NDB),
                    data: Some(ndb),
                });
            }
            app.actions.push(Action::Apply(Command::new(format!("Compile {key}"), edits)));
            app.log.info(format!("{key}: compiled"));
            app.script_tools.messages.push(Message {
                script: key,
                text: format!("{key}: compiled"),
                location: None,
                error: false,
            });
            true
        }
        Err(e) => {
            app.log.error(e.message.clone());
            let location = e.location();
            app.script_tools.messages.push(Message {
                script: key,
                text: e.message,
                location,
                error: true,
            });
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_pages_are_highlighted_and_escaped() {
        let style = crate::settings::ScriptStyle::default();
        let html = print_html("x<y>.nss", "// a < b\nvoid main() {}", &style);
        assert!(html.contains("<title>x&lt;y&gt;.nss</title>"));
        assert!(html.contains("// a &lt; b"));
        assert!(html.contains("onload=\"window.print()\""));
        assert!(html.matches("<span style=\"color:#").count() > 3, "{html}");
        assert_eq!(file_url(std::path::Path::new("/tmp/a b/c.html")), "file:///tmp/a%20b/c.html");
        assert_eq!(file_url(std::path::Path::new("C:\\x\\y.html")), "file:///C:/x/y.html");
    }

    #[test]
    fn highlighting_keeps_the_text() {
        let src = "void main() { // hi\n  int x = 0x1F; string s = \"é\"; }";
        let style = ScriptStyle::default();
        let job = highlight(src, &Palette::new(&style, true, Color32::WHITE));
        assert_eq!(job.text, src);
        assert!(job.sections.len() > 10);
    }
}

#[cfg(test)]
mod perf {
    /// Timing only: `cargo test --release -p mg-ui --lib highlight_speed -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn highlight_speed() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let game =
            mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        let data = game.resman.get_named("nwscript", mg_core::ResType::NSS).unwrap();
        let text = String::from_utf8_lossy(&data).into_owned();
        let t = std::time::Instant::now();
        let palette = super::Palette::new(
            &crate::settings::ScriptStyle::default(),
            true,
            egui::Color32::WHITE,
        );
        let job = super::highlight(&text, &palette);
        println!("highlight: {:?}, {} sections", t.elapsed(), job.sections.len());
    }
}

/// Opens the Open Script window, with every script there is to open: the
/// module's, and what the game would load besides (its haks' and its own).
pub(crate) fn open_script_window(app: &mut Moonglow) {
    use crate::script_tools::{OpenScript, ScriptFrom};
    let mut scripts: Vec<(ResKey, ScriptFrom)> = Vec::new();
    if let Some(ws) = &app.ws {
        scripts.extend(ws.module.keys_of(ResType::NSS).map(|k| (*k, ScriptFrom::Module)));
    }
    if let Some(game) = app.game.as_deref() {
        let layers = game.resman.layers();
        for (key, layer) in game.resman.entries() {
            if key.restype != ResType::NSS || scripts.iter().any(|(k, _)| *k == key) {
                continue;
            }
            let label = layers.get(layer).map_or("", |l| l.label.as_str());
            // (The module's own layer: listed already.)
            if label == "module" {
                continue;
            }
            let from = if label.starts_with("hak:") { ScriptFrom::Hak } else { ScriptFrom::Game };
            scripts.push((key, from));
        }
    }
    scripts.sort_by_key(|(k, _)| k.resref.to_string());
    let shown = app.script_tools.open_script.take().map(|o| o.shown).unwrap_or_default();
    app.script_tools.open_script = Some(OpenScript { filter: String::new(), shown, scripts });
}

/// The Open Script window: a name to find, which scripts to show (as
/// Aurora's Resources to Show), and the list; a click, or Enter for the
/// first listed, opens one (the game's and a hak's to read only).
fn open_script_ui(app: &mut Moonglow, ctx: &egui::Context) {
    use crate::script_tools::{ScriptFrom, ScriptsShown};
    let Some(mut w) = app.script_tools.open_script.take() else { return };
    let mut open = true;
    let mut chosen = None;
    egui::Window::new("Open Script")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .default_size([420.0, 440.0])
        .collapsible(false)
        .open(crate::widgets::open_unless_escape(ctx, "Open Script", &mut open))
        .show(ctx, |ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut w.filter)
                    .hint_text("Find by name")
                    .desired_width(f32::INFINITY),
            );
            crate::widgets::autofocus(ui, &field);
            let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal_wrapped(|ui| {
                for (shown, label) in [
                    (ScriptsShown::All, "All Resources"),
                    (ScriptsShown::Module, "Module Resources Only"),
                    (ScriptsShown::Haks, "Hak Pak Resources Only"),
                ] {
                    ui.radio_value(&mut w.shown, shown, label);
                }
            });
            let needle = w.filter.trim().to_lowercase();
            let listed: Vec<&(ResKey, ScriptFrom)> = w
                .scripts
                .iter()
                .filter(|(_, from)| match w.shown {
                    ScriptsShown::All => true,
                    ScriptsShown::Module => *from == ScriptFrom::Module,
                    ScriptsShown::Haks => *from == ScriptFrom::Hak,
                })
                .filter(|(k, _)| needle.is_empty() || k.resref.to_string().contains(&needle))
                .collect();
            ui.weak(format!("{} scripts", listed.len()));
            if entered && let Some(first) = listed.first() {
                chosen = Some(**first);
            }
            let row = ui.spacing().interact_size.y;
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                ui,
                row,
                listed.len(),
                |ui, range| {
                    for (key, from) in listed[range].iter().copied() {
                        ui.horizontal(|ui| {
                            let r = ui.selectable_label(false, key.resref.to_string());
                            match from {
                                ScriptFrom::Module => {}
                                ScriptFrom::Hak => _ = ui.weak("hak"),
                                ScriptFrom::Game => _ = ui.weak("game"),
                            }
                            if r.clicked() {
                                chosen = Some((*key, *from));
                            }
                        });
                    }
                },
            );
        });
    match chosen {
        Some((key, ScriptFrom::Module)) => app.actions.push(Action::OpenTab(Tab::Script(key))),
        // (Not the module's to change: read as the resource browser shows it.)
        Some((key, _)) => app.actions.push(Action::OpenTab(Tab::Resource(key))),
        None if open => app.script_tools.open_script = Some(w),
        None => {}
    }
}

/// The script editor's windows: Find Text / Replace, Open Script, New
/// Script, Save As.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    open_script_ui(app, &ctx);
    if app.script_tools.search.open {
        let mut s = app.script_tools.search.clone();
        let mut find_next = false;
        let mut replace = false;
        let mut replace_all = false;
        let mut find_all = false;
        let mut entered = false;
        let title = if s.replace_mode { "Replace Text" } else { "Find Text" };
        egui::Window::new(title)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .open(crate::widgets::open_unless_escape(&ctx, title, &mut s.open))
            .show(&ctx, |ui| {
                egui::Grid::new("find-grid").num_columns(2).show(ui, |ui| {
                    crate::widgets::field_label(ui, "Find What");
                    let field = ui.text_edit_singleline(&mut s.find);
                    crate::widgets::autofocus(ui, &field);
                    // Enter finds (the next, or all), and the field keeps
                    // the keys for the next Enter.
                    if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        entered = true;
                        field.request_focus();
                    }
                    ui.end_row();
                    if s.replace_mode {
                        ui.label("Replace With");
                        ui.text_edit_singleline(&mut s.replace);
                        ui.end_row();
                    }
                });
                ui.checkbox(&mut s.options.match_case, "Match Case");
                ui.checkbox(&mut s.options.whole_word, "Match Whole Word Only");
                ui.checkbox(&mut s.options.backwards, "Search Backward");
                if !s.replace_mode {
                    if ui
                        .checkbox(&mut s.in_open, "Find In Currently Open Scripts")
                        .on_hover_text("Every match in the scripts open now, in Search Results")
                        .changed()
                        && s.in_open
                    {
                        s.in_files = false;
                    }
                    if ui
                        .checkbox(&mut s.in_files, "Find In Files (all scripts in the module)")
                        .changed()
                        && s.in_files
                    {
                        s.in_open = false;
                    }
                }
                let all = (s.in_files || s.in_open) && !s.replace_mode;
                ui.horizontal(|ui| {
                    if all {
                        find_all = ui.button("Find All").clicked() || entered;
                    } else {
                        find_next = ui.button("Find Next").clicked() || entered;
                    }
                    if s.replace_mode {
                        replace = ui.button("Replace").clicked();
                        replace_all = ui.button("Replace All").clicked();
                    }
                });
            });
        app.script_tools.search = s.clone();
        let target = s.script.filter(|k| app.scripts.contains_key(k));
        if find_next && let Some(k) = target {
            find_in_script(app, &ctx, k);
        }
        if replace && let Some(k) = target {
            // Replace the selection when it is a match, then find the next.
            if let Some((a, b)) = cursor(&ctx, k) {
                let buf = app.scripts.get_mut(&k).expect("open");
                let selected: String = buf.text.chars().skip(a).take(b - a).collect();
                let same = if s.options.match_case {
                    selected == s.find
                } else {
                    selected.eq_ignore_ascii_case(&s.find)
                };
                if a != b && same {
                    insert(buf, &ctx, k, a, b, &s.replace);
                }
            }
            find_in_script(app, &ctx, k);
        }
        if replace_all && let Some(k) = target {
            let buf = app.scripts.get_mut(&k).expect("open");
            let (text, n) = tools::replace_all(&buf.text, &s.find, &s.replace, &s.options);
            buf.text = text;
            app.log.info(format!("Replaced {n} in {k}"));
        }
        if find_all {
            let Some(ws) = &app.ws else { return };
            let mut keys: Vec<ResKey> = if s.in_open {
                app.scripts.keys().copied().collect()
            } else {
                ws.module.keys_of(ResType::NSS).copied().collect()
            };
            keys.sort();
            let mut results = Vec::new();
            for k in keys {
                let text = app
                    .scripts
                    .get(&k)
                    .map(|b| b.text.clone())
                    .or_else(|| ws.module.get(&k).map(decode))
                    .unwrap_or_default();
                for (line, t) in tools::find_lines(&text, &s.find, &s.options) {
                    results.push((k, line, t));
                }
            }
            app.log.info(format!("{:?}: {} lines found", s.find, results.len()));
            app.script_tools.search.results = results;
            app.script_tools.info = InfoTab::SearchResults;
        }
    }

    if let Some((from, mut name)) = app.script_save_as.clone() {
        let mut close = false;
        egui::Window::new("Save Script As")
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .show(&ctx, |ui| {
                ui.label("New name (up to 16 characters)");
                let field = ui.add(egui::TextEdit::singleline(&mut name).char_limit(16));
                crate::widgets::autofocus(ui, &field);
                let key = mg_core::ResRef::from_str(name.trim())
                    .ok()
                    .filter(|r| !r.is_empty())
                    .map(|r| ResKey::new(r, ResType::NSS));
                let exists =
                    key.is_some_and(|k| app.ws.as_ref().is_some_and(|w| w.module.contains(&k)));
                if exists {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "The module has a script of this name; it will be replaced.",
                    );
                }
                ui.horizontal(|ui| {
                    if (ui.add_enabled(key.is_some(), egui::Button::new("Save")).clicked()
                        || (key.is_some() && crate::widgets::enter(ui)))
                        && let Some(k) = key
                    {
                        let text =
                            app.scripts.get(&from).map(|b| b.text.clone()).unwrap_or_default();
                        app.actions.push(Action::Apply(Command::new(
                            format!("Save {from} as {k}"),
                            vec![Edit::SetResource { key: k, data: Some(encode(&text)) }],
                        )));
                        app.actions.push(Action::OpenTab(Tab::Script(k)));
                        close = true;
                    }
                    if crate::widgets::cancel(ui) {
                        close = true;
                    }
                });
            });
        app.script_save_as = if close { None } else { Some((from, name)) };
    }

    if let Some(mut name) = app.new_script.clone() {
        let mut close = false;
        egui::Window::new("New Script")
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .show(&ctx, |ui| {
                ui.label("Name (up to 16 characters)");
                let field = ui.add(egui::TextEdit::singleline(&mut name).char_limit(16));
                crate::widgets::autofocus(ui, &field);
                let key = mg_core::ResRef::from_str(name.trim())
                    .ok()
                    .filter(|r| !r.is_empty())
                    .map(|r| ResKey::new(r, ResType::NSS));
                let exists =
                    key.is_some_and(|k| app.ws.as_ref().is_some_and(|w| w.module.contains(&k)));
                if exists {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        "The module already has a script of this name.",
                    );
                }
                ui.horizontal(|ui| {
                    if (ui
                        .add_enabled(key.is_some() && !exists, egui::Button::new("Create"))
                        .clicked()
                        || (key.is_some() && !exists && crate::widgets::enter(ui)))
                        && let Some(k) = key
                    {
                        app.actions.push(Action::Apply(Command::new(
                            format!("New script {k}"),
                            vec![Edit::SetResource {
                                key: k,
                                data: Some(NEW_SCRIPT.as_bytes().to_vec()),
                            }],
                        )));
                        app.actions.push(Action::OpenTab(Tab::Script(k)));
                        close = true;
                    }
                    if crate::widgets::cancel(ui) {
                        close = true;
                    }
                });
            });
        app.new_script = if close { None } else { Some(name) };
    }
}

/// A new script that answers whether a conversation line shows.
pub(crate) const NEW_CONDITION: &str =
    "int StartingConditional()\r\n{\r\n\r\n    return TRUE;\r\n}\r\n";

impl Moonglow {
    /// [`Action::EditScript`].
    pub(crate) fn edit_script(&mut self, name: mg_core::ResRef, condition: bool) {
        let key = ResKey::new(name, ResType::NSS);
        if name.is_empty() {
            return;
        }
        if self.ws.as_ref().is_some_and(|w| w.module.contains(&key)) {
            // (As a script opened from the module tree: in the external
            // editor, where Options has scripts open there.)
            self.actions.push(Action::OpenResource(key));
        } else if self.game.as_deref().is_some_and(|g| g.resman.contains(&key)) {
            self.actions.push(Action::OpenTab(Tab::Resource(key)));
        } else if self.ws.is_some() {
            let text = if condition { NEW_CONDITION } else { NEW_SCRIPT };
            let edit = Edit::SetResource { key, data: Some(text.as_bytes().to_vec()) };
            match self.apply(Command::new(format!("New script {key}"), vec![edit])) {
                Ok(_) => {
                    self.log.info(format!("Script '{name}' not found: created it"));
                    self.actions.push(Action::OpenResource(key));
                }
                Err(e) => self.log.error(e.to_string()),
            }
        }
    }
}

/// A new script, as Aurora starts one.
pub(crate) const NEW_SCRIPT: &str = "void main()\r\n{\r\n\r\n}\r\n";
