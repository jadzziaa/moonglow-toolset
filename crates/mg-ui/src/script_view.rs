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
}

/// The laid-out text of a script editor, reused while the text, width and
/// colours stay the same (a large script takes milliseconds to highlight and
/// lay out).
#[derive(Debug, Clone)]
pub(crate) struct LaidOut {
    text: String,
    wrap: f32,
    palette: Palette,
    galley: std::sync::Arc<egui::Galley>,
}

impl ScriptBuffer {
    pub(crate) fn is_dirty(&self) -> bool {
        self.text != self.saved
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
        }
    });
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
        let text = app.scripts[&key].text.clone();
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
            &text,
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
    if focused {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let pressed = |m, k| ui.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        open_find = pressed(Modifiers::COMMAND, Key::F);
        open_replace = pressed(Modifiers::COMMAND, Key::R);
        find_next = pressed(Modifiers::NONE, Key::F3);
        toggle_bookmark = pressed(Modifiers::NONE, Key::F5);
        complete = pressed(Modifiers::NONE, Key::F2) || pressed(Modifiers::COMMAND, Key::Space);
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
    let dirty = app.scripts[&key].is_dirty();
    ui.horizontal(|ui| {
        save = ui
            .add_enabled(dirty, egui::Button::new("Save"))
            .on_hover_text("Save the script into the module")
            .clicked();
        compile = ui
            .button("Compile")
            .on_hover_text("Save and compile (F7 compiles all scripts)")
            .clicked();
        save_as = ui.button("Save As…").clicked();
        ui.separator();
        open_find |= ui.button("Find…").on_hover_text("Ctrl+F; F3 finds again").clicked();
        open_replace |= ui.button("Replace…").on_hover_text("Ctrl+R").clicked();
        toggle_bookmark |= ui
            .button("Bookmark")
            .on_hover_text("Toggle a bookmark on the cursor's line (F5)")
            .clicked();
        if dirty {
            ui.weak("modified");
        }
    });
    ui.separator();

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
    egui::Panel::right(egui::Id::new(("script-side", key)))
        .resizable(true)
        .default_size(260.0)
        .show(ui, |ui| {
            let t = &mut app.script_tools;
            ui.horizontal(|ui| {
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
            if t.side == SideTab::Templates {
                let templates = tools::templates(app.install.as_ref());
                egui::ScrollArea::vertical().id_salt("templates").show(ui, |ui| {
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
                egui::ScrollArea::vertical().id_salt(("symbols", kind)).show_rows(
                    ui,
                    row,
                    list.len(),
                    |ui, range| {
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
                    },
                );
            }
        });

    // Compiler, Help, Bookmarks, Search Results.
    let mut jump: Option<(ResKey, usize)> = None;
    let mut open_other: Option<(String, usize)> = None;
    egui::Panel::bottom(egui::Id::new(("script-info", key)))
        .resizable(true)
        .default_size(140.0)
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
    let pending = script_tools.jump.take_if(|(k, _)| *k == key).or(jump).map(|(_, at)| at);
    if let Some(at) = pending {
        set_cursor(&ctx, key, at, at);
        script_tools.scroll = Some(key);
    }
    let cached = laid_out.entry(key).or_insert(None);
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap: f32| {
        if let Some(c) = cached
            .as_ref()
            .filter(|c| c.wrap == wrap && c.palette == palette && c.text == text.as_str())
        {
            return c.galley.clone();
        }
        // Code does not wrap (the editor scrolls sideways), which also
        // keeps the line numbers beside their lines.
        let mut job = highlight(text.as_str(), &palette);
        job.wrap.max_width = f32::INFINITY;
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        *cached = Some(LaidOut {
            text: text.as_str().to_string(),
            wrap,
            palette,
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
            ui.horizontal_top(|ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(numbers).font(palette.font()).weak())
                        .selectable(false),
                );
                let out = egui::TextEdit::multiline(&mut buf.text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
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
                        }
                    }
                    if out.response.response.changed() {
                        script_tools.completion = None;
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
            let text = if s.kind == SymbolKind::Function {
                format!("{}(", s.name)
            } else {
                s.name.clone()
            };
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

    if save_as {
        app.script_save_as = Some((key, String::new()));
    }
    if save || compile {
        let buf = app.scripts.get_mut(&key).expect("open");
        let text = buf.text.clone();
        if buf.is_dirty() {
            buf.saved = text.clone();
            app.actions.push(Action::Apply(Command::new(
                format!("Edit {key}"),
                vec![Edit::SetResource { key, data: Some(encode(&text)) }],
            )));
        }
        if compile {
            compile_one(app, key, &text);
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
    let source = encode(text);
    let module = app.ws.as_ref().map(|w| &w.module);
    let resman = app.game.as_ref().map(|g| &g.resman);
    let name = key.resref.to_lowercase().to_string();
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
    c.compile(&name).map(|out| out.ncs)
}

/// Compiles one script (its text as in the editor) and stores the bytecode.
fn compile_one(app: &mut Moonglow, key: ResKey, text: &str) {
    if app.ws.is_none() {
        return;
    }
    let result = compile_source(app, key, text);
    app.script_tools.messages.retain(|m| m.script != key);
    app.script_tools.info = InfoTab::Compiler;
    match result {
        Ok(out) => {
            let ncs = ResKey::new(key.resref, ResType::NCS);
            app.actions.push(Action::Apply(Command::new(
                format!("Compile {key}"),
                vec![Edit::SetResource { key: ncs, data: Some(out) }],
            )));
            app.log.info(format!("{key}: compiled"));
            app.script_tools.messages.push(Message {
                script: key,
                text: format!("{key}: compiled"),
                location: None,
                error: false,
            });
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

/// The script editor's windows: Find Text / Replace, New Script, Save As.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    if app.script_tools.search.open {
        let mut s = app.script_tools.search.clone();
        let mut find_next = false;
        let mut replace = false;
        let mut replace_all = false;
        let mut find_all = false;
        let title = if s.replace_mode { "Replace Text" } else { "Find Text" };
        egui::Window::new(title).collapsible(false).resizable(false).open(&mut s.open).show(
            &ctx,
            |ui| {
                egui::Grid::new("find-grid").num_columns(2).show(ui, |ui| {
                    ui.label("Find What");
                    ui.text_edit_singleline(&mut s.find);
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
                    ui.checkbox(&mut s.in_files, "Find In Files (all scripts in the module)");
                }
                ui.horizontal(|ui| {
                    if s.in_files && !s.replace_mode {
                        find_all = ui.button("Find All").clicked();
                    } else {
                        find_next = ui.button("Find Next").clicked();
                    }
                    if s.replace_mode {
                        replace = ui.button("Replace").clicked();
                        replace_all = ui.button("Replace All").clicked();
                    }
                });
            },
        );
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
            let mut keys: Vec<ResKey> = ws.module.keys_of(ResType::NSS).copied().collect();
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
        egui::Window::new("Save Script As").collapsible(false).resizable(false).show(&ctx, |ui| {
            ui.label("New name (up to 16 characters)");
            ui.add(egui::TextEdit::singleline(&mut name).char_limit(16));
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
                if ui.add_enabled(key.is_some(), egui::Button::new("Save")).clicked()
                    && let Some(k) = key
                {
                    let text = app.scripts.get(&from).map(|b| b.text.clone()).unwrap_or_default();
                    app.actions.push(Action::Apply(Command::new(
                        format!("Save {from} as {k}"),
                        vec![Edit::SetResource { key: k, data: Some(encode(&text)) }],
                    )));
                    app.actions.push(Action::OpenTab(Tab::Script(k)));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        app.script_save_as = if close { None } else { Some((from, name)) };
    }

    if let Some(mut name) = app.new_script.clone() {
        let mut close = false;
        egui::Window::new("New Script").collapsible(false).resizable(false).show(&ctx, |ui| {
            ui.label("Name (up to 16 characters)");
            ui.add(egui::TextEdit::singleline(&mut name).char_limit(16));
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
                if ui.add_enabled(key.is_some() && !exists, egui::Button::new("Create")).clicked()
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
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        app.new_script = if close { None } else { Some(name) };
    }
}

/// A new script, as Aurora starts one.
pub(crate) const NEW_SCRIPT: &str = "void main()\r\n{\r\n\r\n}\r\n";
