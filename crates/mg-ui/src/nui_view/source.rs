//! The NUI designer's source group (M4's U19, the NUI board's second editor
//! group, `crier.jui` beside the design): a tab of its own
//! ([`crate::Tab::NuiSource`]), a part of the designer of its window. Its
//! header holds the switch (JUI Source: the window's text as the module
//! holds it; Binding JSON: its settings'), Event Script… and Opener
//! Script; its body the text, numbered a line at a time and laid out once
//! for each text ([`crate::script_layout::LaidOut`]).
//!
//! What is typed is set when the field is left (another switch position,
//! a press elsewhere, the group closed: [`super::follow`]), one Undo step,
//! also text that doesn't read as JSON, which is kept in the module as typed
//! (the designer's pages then ask for it to be put right); the design's
//! canvas shows it as it is typed, where it reads. The design and the source
//! follow each other: the control chosen is chosen in the text, and the
//! cursor in the text chooses the control it is in.
use super::*;

/// The source group's header (the kit's group, the tests'), the board's.
pub const SOURCE_HEADER: &str = "Source header";
/// Its switch's name, and its two positions' words.
pub const SOURCE: &str = "Source";
pub const JUI_SOURCE: &str = "JUI Source";
pub const BINDING_JSON: &str = "Binding JSON";
/// Its text field's name, on each switch position.
pub const SOURCE_TEXT: &str = "JUI Source text";
pub const SETTINGS_TEXT: &str = "Binding JSON text";
/// Its two buttons' names.
pub const EVENT_SCRIPT: &str = "Event Script…";
pub const OPENER_SCRIPT: &str = "Opener Script";

/// The switch moved to `of`: what was typed on the other position is set
/// (by the designer's frame's end, `end`), as a field left.
pub(super) fn switch_to(state: &mut State, of: SourceOf) {
    if state.source_of == of {
        return;
    }
    if let Some(draft) = state.source_draft.take()
        && draft.typed
    {
        match draft.of {
            SourceOf::Window => state.typed_raw = Some(draft.text),
            SourceOf::Settings => state.typed_config = Some(draft.text),
        }
    }
    state.source_of = of;
    state.source_shown = None;
    state.source_cursor = None;
    state.source_lines = None;
}

/// What was typed in the source group set: by the designer's frame's end
/// where it has yet to come this frame (its Cancel noting it with the rest),
/// else as a command of its own now, which its Cancel notes too.
fn set(app: &mut Moonglow, ctx: &egui::Context, key: ResKey, state: &mut State) {
    if state.typed_raw.is_none() && state.typed_config.is_none() {
        return;
    }
    let pass = ctx.cumulative_pass_nr();
    if state.began == Some(pass) && !state.ended {
        return;
    }
    let texts = (state.typed_raw.take(), state.typed_config.take());
    let edits = written(app, key, texts, state);
    if edits.is_empty() {
        return;
    }
    let cmd = Command::new(format!("Edit NUI {}", key.resref), edits);
    let targets = [(key, mg_edit::GffPath::root())];
    crate::blueprint::note_command(app, &Tab::Nui(key), &targets, &cmd);
    app.actions.push(Action::Apply(cmd));
    ctx.request_repaint();
}

/// A designer's source group not drawn this frame (closed, behind another
/// tab, its window folded): what was typed in it set, as a field left.
pub(super) fn left(app: &mut Moonglow, ctx: &egui::Context, key: ResKey, state: &mut State) {
    let pass = ctx.cumulative_pass_nr();
    if state.source_drawn == Some(pass) {
        return;
    }
    if let Some(draft) = state.source_draft.take_if(|d| d.typed) {
        // (Only onto the bytes it was typed over: changed from elsewhere,
        // or gone, what was typed is let go, as a field's is.)
        if !typed_over(app, key, &draft) {
            return;
        }
        match draft.of {
            SourceOf::Window => state.typed_raw = Some(draft.text),
            SourceOf::Settings => state.typed_config = Some(draft.text),
        }
        set(app, ctx, key, state);
    }
}

/// Whether the module still holds the bytes `draft` was typed over.
fn typed_over(app: &Moonglow, key: ResKey, draft: &SourceDraft) -> bool {
    let Some(ws) = app.ws.as_ref() else { return false };
    let now = held_of(&ws.module, key, draft.of);
    same(std::slice::from_ref(&draft.from), std::slice::from_ref(&now))
        && (now.is_some() || draft.of == SourceOf::Settings)
}

/// Save with the source typed in (`super::store_typed`): what it shows set
/// first, onto the bytes it was typed over, the field keeping the keyboard
/// (its draft read from what it set).
pub(super) fn store(app: &mut Moonglow, key: ResKey, state: &mut State) {
    let Some(draft) = state.source_draft.as_mut().filter(|d| d.typed) else { return };
    let text = draft.text.clone();
    let of = draft.of;
    if !typed_over(app, key, state.source_draft.as_ref().expect("just seen")) {
        return;
    }
    let texts = match of {
        SourceOf::Window => (Some(text), None),
        SourceOf::Settings => (None, Some(text)),
    };
    let edits = written(app, key, texts, state);
    if edits.is_empty() {
        return;
    }
    let cmd = Command::new(format!("Edit NUI {}", key.resref), edits);
    let targets = [(key, mg_edit::GffPath::root())];
    crate::blueprint::note_command(app, &Tab::Nui(key), &targets, &cmd);
    app.run_now(Action::Apply(cmd));
    // (Typed in still: from the bytes set now.)
    let now = app.ws.as_ref().map(|ws| held_of(&ws.module, key, of)).unwrap_or_default();
    if let Some(draft) = state.source_draft.as_mut() {
        draft.typed = false;
        draft.from = now;
    }
}

/// The lines the control chosen takes in the text (`Moonglow::nui_source_lines`).
pub(crate) fn lines(state: &State) -> Option<(usize, usize)> {
    state.source_lines
}

impl crate::Moonglow {
    /// Where the NUI source view drew its chosen control's band last, top
    /// to bottom, for tests.
    #[doc(hidden)]
    pub fn nui_source_band_for_test(ctx: &egui::Context) -> Option<egui::Rangef> {
        ctx.data(|d| d.get_temp::<Option<egui::Rangef>>(band_id())).flatten()
    }
}

/// What [`crate::Moonglow::nui_source_band_for_test`] reads.
fn band_id() -> egui::Id {
    egui::Id::new("nui-source-band")
}

/// The name of the switch's position designer `state` shows (its open-tabs
/// record's `page`).
pub(crate) fn switch_name(state: &State) -> &'static str {
    match state.source_of {
        SourceOf::Window => JUI_SOURCE,
        SourceOf::Settings => BINDING_JSON,
    }
}

/// The source group's tab: its header over the text.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Some(ws) = &app.ws else {
        ui.label("Open a module first.");
        return;
    };
    let ctx = ui.ctx().clone();
    let mut state = app.nui_views.remove(&key).unwrap_or_default();
    // (Its switch as the open-tabs record kept it, once: a module opened
    // again.)
    let kept = app.open_tabs.take_view(&Tab::NuiSource(key));
    if let Some(page) = kept.as_ref().and_then(|v| v.page()) {
        let of = if page == BINDING_JSON { SourceOf::Settings } else { SourceOf::Window };
        switch_to(&mut state, of);
    }
    state.source_drawn = Some(ctx.cumulative_pass_nr());
    // (A part of the designer: its frame begun or gone on with; its new
    // errors open Checks as the design's do, the design hidden or not.)
    let checked = match begin(&ws.module, &ctx, key, &mut state) {
        Ok(checked) => checked,
        Err(why) => {
            ui.label(why);
            app.nui_views.insert(key, state);
            return;
        }
    };
    checks::note_failures(app, key, &checked, &mut state);
    // (A variant chosen to edit: the switch to where it is written, once.)
    if state.source_variant != state.edit_view {
        if state.edit_view.is_some() {
            switch_to(&mut state, SourceOf::Settings);
        }
        state.source_variant = state.edit_view.clone();
    }
    let asked = header(app, ui, key, &mut state);
    if let Some(of) = asked.switch {
        switch_to(&mut state, of);
    }
    if asked.event_script
        && let Some(s) = state.work.as_ref().map(|w| w.settings.clone())
    {
        state.event_script = Some(events::EventScript { settings: s, handler: None, code: None });
    }
    if asked.opener
        && let Some(k) = ResKey::parse(&format!("{}_o", key.resref), ResType::NSS)
    {
        app.actions.push(Action::OpenResource(k));
    }
    body(app, ui, key, &mut state);
    set(app, &ctx, key, &mut state);
    // (Event Script… where the designer's frame has ended: its script
    // written and opened now.)
    if state.event_script.is_some() && state.ended {
        let edits = written(app, key, (None, None), &mut state);
        if !edits.is_empty() {
            let cmd = Command::new(format!("Edit NUI {}", key.resref), edits);
            app.actions.push(Action::Apply(cmd));
        }
    }
    app.nui_views.insert(key, state);
}

/// What the source group's header asked for.
#[derive(Default)]
struct Asked {
    switch: Option<SourceOf>,
    event_script: bool,
    opener: bool,
}

/// The source group's header (the board's): the switch (JUI Source, Binding
/// JSON), and at its right Event Script… (off while the settings don't
/// read) and Opener Script (off where the module has no opener script yet:
/// Build & Compile makes it).
fn header(app: &Moonglow, ui: &mut Ui, key: ResKey, state: &mut State) -> Asked {
    use mg_uikit::Icon;
    use mg_uikit::header::{EditorHeader, Item};
    let name = key.resref.to_string();
    let choices = [
        (SourceOf::Window, JUI_SOURCE, "The window's layout as JSON"),
        (
            SourceOf::Settings,
            BINDING_JSON,
            "The window's settings as JSON: binds, handlers, variants",
        ),
    ];
    let width = form::segmented_width(
        ui,
        egui::Id::new("nui-source-switch-w"),
        &[JUI_SOURCE, BINDING_JSON],
    );
    let chosen = std::cell::Cell::new(state.source_of);
    let switch = |ui: &mut Ui| {
        let mut of = chosen.get();
        let room = egui::vec2(width, mg_uikit::metrics::CONTROL_HEIGHT);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(room, layout, |ui| {
            mg_uikit::segmented(ui, SOURCE, &mut of, &choices);
        });
        chosen.set(of);
    };
    let settings_read = state.source.as_ref().is_some_and(|s| s.settings.is_ok());
    let opener = ResKey::parse(&format!("{name}_o"), ResType::NSS);
    let has_opener = opener.is_some_and(|k| app.ws.as_ref().is_some_and(|w| w.module.contains(&k)));
    let out = EditorHeader::new(SOURCE_HEADER)
        .left(Item::custom(SOURCE, width, switch).more(width, switch).primary())
        .right(
            Item::button(Icon::Script, EVENT_SCRIPT)
                .enabled(settings_read)
                .tip(format!("Open the event script, {name}_e.nss")),
        )
        .right(Item::icon(Icon::File, OPENER_SCRIPT).enabled(has_opener).tip(match has_opener {
            true => format!("Opener Script: open {name}_o.nss"),
            false => format!("Opener Script: Build & Compile makes {name}_o.nss"),
        }))
        .show(ui);
    Asked {
        switch: Some(chosen.get()).filter(|of| *of != state.source_of),
        event_script: out.clicked(EVENT_SCRIPT),
        opener: out.clicked(OPENER_SCRIPT),
    }
}

/// The bytes the switch shows, as the module holds them (a pointer, no
/// copy): the window's, or its settings' (`None`: none kept, the settings a
/// window without them would have).
fn held_of(module: &mg_module::Module, key: ResKey, of: SourceOf) -> Held {
    match of {
        SourceOf::Window => module.shared(&key).cloned(),
        SourceOf::Settings => module.shared(&ResKey::new(key.resref, ResType::TXT)).cloned(),
    }
}

/// The text of `held` (`None`: not UTF-8), the default settings' written
/// out once for the run where none are kept.
fn text_of(held: &Held, of: SourceOf) -> Option<&str> {
    static DEFAULT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    match (held, of) {
        (Some(bytes), _) => std::str::from_utf8(bytes).ok(),
        (None, SourceOf::Settings) => Some(
            DEFAULT
                .get_or_init(|| String::from_utf8(Settings::default().bytes()).unwrap_or_default()),
        ),
        (None, SourceOf::Window) => None,
    }
}

/// The source's text, editable: its line numbers and its text, laid out a
/// line at a time, kept for the text it is (`State::source_laid`); typed in,
/// a draft until the field is left; the control chosen shown in it (the
/// cursor at its first line, the cursor's line marked), and the design
/// following the text where it is clicked, or as the cursor moves while the
/// text chose what the design shows. Nothing of the whole text is gone
/// through in a frame that changes nothing: the draft is read again only
/// where the module holds other bytes, its lines counted and its galley
/// asked only where it changed (M4's U19 review).
fn body(app: &mut Moonglow, ui: &mut Ui, key: ResKey, state: &mut State) {
    let Some(ws) = app.ws.as_ref() else { return };
    let of = state.source_of;
    let held = held_of(&ws.module, key, of);
    let Some(current) = text_of(&held, of) else {
        ui.weak("This text is not UTF-8: it is kept as it is and can't be edited here.");
        return;
    };
    let ctx = ui.ctx().clone();
    let fresh = |text: &str| SourceDraft {
        of,
        text: text.to_owned(),
        typed: false,
        read: None,
        from: held.clone(),
        lines: text.bytes().filter(|b| *b == b'\n').count() + 1,
        made: 0,
    };
    let draft = state.source_draft.take_if(|d| d.of == of).unwrap_or_else(|| fresh(current));
    let draft = state.source_draft.insert(draft);
    // (Changed underneath while not typed in: Undo, the design, a page.)
    let id = ui.id().with(("nui-source-field", key, of));
    let focused = ui.memory(|m| m.has_focus(id));
    if !focused
        && !draft.typed
        && !same(std::slice::from_ref(&draft.from), std::slice::from_ref(&held))
    {
        let made = draft.made + 1;
        *draft = fresh(current);
        draft.made = made;
        // (The control chosen found again in the new text.)
        state.source_shown = None;
    }
    // The control chosen in the design (on the canvas, in Layers, by a
    // link) shown in the text: the cursor at its first line, where it
    // changed since the source last showed it and the text isn't typed in.
    let mut select = None;
    if !focused && state.source_shown.as_ref() != Some(&state.selected) {
        state.source_shown = Some(state.selected.clone());
        let at = pointer_in(of, state.edit_view.as_deref(), &state.selected);
        select = at.and_then(|p| span(&draft.text, &p)).map(|(start, _)| start);
    }
    // (In the script editor's font and colors, Options › Script Editor's.)
    let palette = crate::script_view::Palette::for_ui(&app.settings.script_style, ui);
    let laid = &mut state.source_laid;
    laid.begin(ui, &palette);
    laid.numbers(ui, draft.lines, &palette);
    let gutter_width = laid.gutter_width();
    // (Not typed in, the galley laid out for this text: the text is not
    // compared again. Typed in, `LaidOut` finds what changed.)
    let made_with = (draft.made, crate::script_layout::fonts_made(&ctx), ctx.pixels_per_point());
    let kept = (state.source_galley.clone())
        .filter(|(with, look, _)| !focused && *with == made_with && *look == palette)
        .map(|(_, _, galley)| galley);
    let mut laid_out = None;
    if let Some(start) = select {
        let mut s = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
        let a = char_at(&draft.text, start);
        s.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(a))));
        s.store(&ctx, id);
        state.source_cursor = Some(a);
        state.source_lines = Some({
            let line = draft.text[..start].bytes().filter(|b| *b == b'\n').count();
            (line, line)
        });
    }
    let mut cursor = None;
    let mut changed = false;
    let mut clicked = false;
    egui::ScrollArea::both().id_salt(("nui-source-scroll", key)).auto_shrink([false, false]).show(
        ui,
        |ui| {
            let room = ui.available_height();
            ui.horizontal_top(|ui| {
                // (The script editor's gutter and text, side by side:
                // `script_layout`'s.)
                ui.spacing_mut().item_spacing.x = 0.0;
                // (The cursor's line marked, under the text: the board's.)
                let under = ui.painter().add(egui::Shape::Noop);
                let (gutter, numbers) =
                    ui.allocate_exact_size(egui::vec2(gutter_width, room), egui::Sense::hover());
                crate::script_layout::name_gutter(&numbers, draft.lines);
                let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| match &kept {
                    Some(galley) => galley.clone(),
                    None => {
                        let galley = laid.text(ui, text.as_str(), &palette);
                        laid_out = Some(galley.clone());
                        galley
                    }
                };
                let out = egui::TextEdit::multiline(&mut draft.text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(1)
                    .min_size(egui::vec2(0.0, room))
                    .id(id)
                    .frame(crate::script_layout::text_frame())
                    .layouter(&mut layouter)
                    .show(ui);
                let none = Default::default();
                let marks = crate::script_layout::Marks { bookmarks: &none, numbered: &[None; 10] };
                let color = palette.colors[0];
                laid.paint_gutter(
                    ui,
                    gutter.left(),
                    &out.galley,
                    out.galley_pos,
                    None,
                    color,
                    marks,
                );
                // (A line's place: its row's, as the text is laid out.)
                let rows = |line: usize| {
                    let r = out.galley.rows.get(line).or(out.galley.rows.last())?;
                    let top = out.galley_pos.y + r.pos.y;
                    Some(egui::Rangef::new(top, top + r.size.y))
                };
                // (Named for a screen reader by what it shows.)
                let said = match of {
                    SourceOf::Window => SOURCE_TEXT,
                    SourceOf::Settings => SETTINGS_TEXT,
                };
                ui.ctx().accesskit_node_builder(id, |node| node.set_label(said));
                crate::widgets::page_keys(ui, &out);
                crate::widgets::leaves_at_a_press(&ctx, &out.response.response);
                changed = out.response.response.changed();
                clicked = out.response.response.clicked()
                    || ui.input(|i| i.pointer.any_pressed())
                        && out.response.response.contains_pointer();
                cursor = out.cursor_range.map(|r| usize::from(r.primary.index));
                // (The field's cursor, kept while it has no keyboard too:
                // the design's choice puts it on its control's line.)
                let kept = egui::text_edit::TextEditState::load(&ctx, id);
                let at = (out.cursor_range.map(|r| r.primary))
                    .or_else(|| kept.and_then(|s| s.cursor.char_range()).map(|r| r.primary))
                    .unwrap_or_default();
                let line = out.galley.layout_from_cursor(at).row + 1;
                crate::script_layout::describe_line(&out.response.response, line, draft.lines);
                let drawn = state.source_lines.and_then(|(a, _)| rows(a));
                if let Some(y) = drawn {
                    let field = out.response.response.rect;
                    let band = egui::Rect::from_x_y_ranges(field.x_range(), y);
                    let fill = mg_uikit::tokens(&ctx).accent.gamma_multiply(0.14);
                    ui.painter().set(under, egui::Shape::rect_filled(band, 0.0, fill));
                }
                // (What was drawn this frame, none included: never a band
                // an earlier frame drew.)
                ctx.data_mut(|d| d.insert_temp(band_id(), drawn));
                if let Some(y) =
                    state.source_lines.filter(|_| select.is_some()).and_then(|(l, _)| rows(l))
                {
                    let left = out.response.response.rect.left();
                    let at = egui::Rect::from_x_y_ranges(left..=left + 1.0, y);
                    ui.scroll_to_rect(at, Some(egui::Align::Center));
                }
            });
        },
    );
    if let Some(galley) = laid_out {
        state.source_galley = Some((made_with, palette, galley));
    }
    if changed {
        draft.typed = true;
        draft.read = None;
        draft.made += 1;
        draft.lines = draft.text.bytes().filter(|b| *b == b'\n').count() + 1;
    }
    let focused = ui.memory(|m| m.has_focus(id));
    // The cursor moved in the text: its line marked; the control it is in
    // chosen in the design (as the frame ends, as Layers' choice is) where
    // the text was clicked, or where the design shows nothing or one control
    // and it was the text that chose it (a choice made on the canvas or in
    // Layers is not replaced by reading with the keys; several chosen are
    // never let go by the cursor).
    if focused && cursor.is_some() && cursor != state.source_cursor {
        state.source_cursor = cursor;
        let byte = cursor.map(|c| byte_at(&draft.text, c)).unwrap_or_default();
        let line = draft.text[..byte].bytes().filter(|b| *b == b'\n').count();
        state.source_lines = Some((line, line));
        let several = state.selected_many.len() > 1;
        let ours =
            state.selected.is_empty() || state.source_chose.as_ref() == Some(&state.selected);
        let follows = !several && (clicked || ours);
        let path = control_at(&draft.text, byte)
            .and_then(|p| design_path(of, state.edit_view.as_deref(), &p))
            .filter(|p| state.work.as_ref().is_some_and(|w| w.window.pointer(p).is_some()));
        if let Some(path) = path.filter(|p| follows && *p != state.selected) {
            let mut chosen = Chosen::of(state);
            chosen.selected = path.clone();
            chosen.selected_many.clear();
            state.picked.chosen = Some(chosen);
            state.source_shown = Some(path.clone());
            state.source_chose = Some(path);
            ctx.request_repaint();
        }
    }
    // Left: what was typed is set.
    let Some(draft) = state.source_draft.as_mut() else { return };
    if !focused && draft.typed {
        draft.typed = false;
        if draft.text != current {
            let text = draft.text.clone();
            match of {
                SourceOf::Window => state.typed_raw = Some(text),
                SourceOf::Settings => state.typed_config = Some(text),
            }
        }
    }
}

/// Where a control's path in the design (`/root/children/0`, with the
/// variant `view` edited in the root's place) is in the text the switch
/// shows: the window's own, or the variant's in the settings
/// (`/views/<name>`). `None` where the text doesn't hold it.
fn pointer_in(of: SourceOf, view: Option<&str>, path: &str) -> Option<String> {
    let rest = path.strip_prefix("/root")?;
    match (of, view) {
        (SourceOf::Window, None) => Some(path.to_owned()),
        (SourceOf::Settings, Some(name)) => {
            let name = name.replace('~', "~0").replace('/', "~1");
            Some(format!("/views/{name}{rest}"))
        }
        _ => None,
    }
}

/// [`pointer_in`] the other way: a pointer into the text the switch shows,
/// as the design's path.
fn design_path(of: SourceOf, view: Option<&str>, pointer: &str) -> Option<String> {
    match (of, view) {
        (SourceOf::Window, None) => pointer.starts_with("/root").then(|| pointer.to_owned()),
        (SourceOf::Settings, Some(name)) => {
            let name = name.replace('~', "~0").replace('/', "~1");
            let rest = pointer.strip_prefix(&format!("/views/{name}"))?;
            Some(format!("/root{rest}"))
        }
        _ => None,
    }
}

/// The char index of byte `at` of `text`.
fn char_at(text: &str, at: usize) -> usize {
    text.get(..at).map_or(0, |t| t.chars().count())
}

/// The byte index of char `at` of `text`.
fn byte_at(text: &str, at: usize) -> usize {
    text.char_indices().nth(at).map_or(text.len(), |(i, _)| i)
}

/// Where the value at JSON `pointer` is in `text` (its first byte and the
/// one after its last), as read; `None` where the text doesn't read so far
/// or has no such value.
fn span(text: &str, pointer: &str) -> Option<(usize, usize)> {
    let mut found = None;
    walk_json(text.as_bytes(), &mut |p, start, end| {
        if p == pointer {
            found = Some((start, end));
        }
    });
    found
}

/// The deepest control (an object of the window's tree: the root, a child,
/// a list's cell) whose text holds byte `at`: its JSON pointer.
fn control_at(text: &str, at: usize) -> Option<String> {
    let mut best: Option<(String, usize)> = None;
    let index = |s: &str| s.parse::<usize>().is_ok();
    let control = |p: &str| {
        if p == "/root" || (p.starts_with("/views/") && p.matches('/').count() == 2) {
            return true;
        }
        let Some((parent, last)) = p.rsplit_once('/').filter(|(_, last)| index(last)) else {
            return false;
        };
        // (A child, or a list's cell: `row_template/N/0`.)
        parent.ends_with("/children")
            || (last == "0"
                && parent
                    .rsplit_once('/')
                    .is_some_and(|(up, i)| index(i) && up.ends_with("/row_template")))
    };
    walk_json(text.as_bytes(), &mut |p, start, end| {
        // (Seen inner first: the deepest is the last to begin.)
        let deeper = best.as_ref().is_none_or(|(_, s)| start >= *s);
        if control(p) && (start..end).contains(&at) && deeper {
            best = Some((p.to_owned(), start));
        }
    });
    best.map(|(p, _)| p)
}

/// Goes through the JSON in `bytes`, calling `seen` for each object and
/// array with its pointer and its span (first byte, one past its last). It
/// stops where the text stops reading as JSON: what was read so far is seen.
fn walk_json(bytes: &[u8], seen: &mut dyn FnMut(&str, usize, usize)) {
    let mut at = 0;
    let mut pointer = String::new();
    let _ = value(bytes, &mut at, &mut pointer, seen, 0);
}

fn skip_ws(b: &[u8], at: &mut usize) {
    while *at < b.len() && b[*at].is_ascii_whitespace() {
        *at += 1;
    }
}

/// Skips a JSON string literal starting at its opening quote without allocating.
fn skip_string(b: &[u8], at: &mut usize) -> Option<()> {
    if b.get(*at) != Some(&b'"') {
        return None;
    }
    *at += 1;
    while *at < b.len() {
        match b[*at] {
            b'\\' => *at += 2,
            b'"' => {
                *at += 1;
                return Some(());
            }
            _ => *at += 1,
        }
    }
    None
}

/// A string's text, read from its opening quote at `at` (escapes kept as
/// written but for `~` and `/`, escaped as a pointer's step).
fn string(b: &[u8], at: &mut usize) -> Option<String> {
    if b.get(*at) != Some(&b'"') {
        return None;
    }
    *at += 1;
    let start = *at;
    let mut has_escape = false;
    while *at < b.len() {
        match b[*at] {
            b'\\' => {
                has_escape = true;
                *at += 2;
            }
            b'"' => {
                let raw = std::str::from_utf8(&b[start..*at]).ok()?;
                *at += 1;
                if !has_escape {
                    return Some(if raw.contains(['~', '/']) {
                        raw.replace('~', "~0").replace('/', "~1")
                    } else {
                        raw.to_owned()
                    });
                }
                let text: String = serde_json::from_str(&format!("\"{raw}\"")).ok()?;
                return Some(text.replace('~', "~0").replace('/', "~1"));
            }
            _ => *at += 1,
        }
    }
    None
}

fn value(
    b: &[u8],
    at: &mut usize,
    pointer: &mut String,
    seen: &mut dyn FnMut(&str, usize, usize),
    depth: usize,
) -> Option<()> {
    if depth > 128 {
        return None;
    }
    skip_ws(b, at);
    let start = *at;
    match b.get(*at)? {
        b'{' => {
            *at += 1;
            loop {
                skip_ws(b, at);
                if b.get(*at) == Some(&b'}') {
                    *at += 1;
                    break;
                }
                let k = string(b, at)?;
                skip_ws(b, at);
                (b.get(*at) == Some(&b':')).then_some(())?;
                *at += 1;
                let len = pointer.len();
                pointer.push('/');
                pointer.push_str(&k);
                value(b, at, pointer, seen, depth + 1)?;
                pointer.truncate(len);
                skip_ws(b, at);
                match b.get(*at)? {
                    b',' => *at += 1,
                    b'}' => {
                        *at += 1;
                        break;
                    }
                    _ => return None,
                }
            }
            seen(pointer, start, *at);
        }
        b'[' => {
            use std::fmt::Write;
            *at += 1;
            let mut i = 0;
            loop {
                skip_ws(b, at);
                if b.get(*at) == Some(&b']') {
                    *at += 1;
                    break;
                }
                let len = pointer.len();
                let _ = write!(pointer, "/{i}");
                value(b, at, pointer, seen, depth + 1)?;
                pointer.truncate(len);
                i += 1;
                skip_ws(b, at);
                match b.get(*at)? {
                    b',' => *at += 1,
                    b']' => {
                        *at += 1;
                        break;
                    }
                    _ => return None,
                }
            }
            seen(pointer, start, *at);
        }
        b'"' => {
            skip_string(b, at)?;
        }
        _ => {
            while *at < b.len()
                && !matches!(b[*at], b',' | b'}' | b']')
                && !b[*at].is_ascii_whitespace()
            {
                *at += 1;
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_control_s_text_is_found_both_ways() {
        let text = "{\n \"root\": {\"type\": \"col\", \"children\": [\n  {\"type\": \"label\", \"id\": \"a/b\"},\n  {\"type\": \"button\"}\n ]}\n}";
        let (start, end) = span(text, "/root/children/1").unwrap();
        assert_eq!(&text[start..end], "{\"type\": \"button\"}");
        let inside = text.find("button").unwrap();
        assert_eq!(control_at(text, inside).as_deref(), Some("/root/children/1"));
        let outer = text.find("children").unwrap();
        assert_eq!(control_at(text, outer).as_deref(), Some("/root"));
        // (A variant in the settings, as the design names its controls.)
        let settings =
            "{\"views\": {\"left\": {\"type\": \"col\", \"children\": [{\"type\": \"label\"}]}}}";
        let at = settings.find("label").unwrap();
        let p = control_at(settings, at).unwrap();
        assert_eq!(
            design_path(SourceOf::Settings, Some("left"), &p).as_deref(),
            Some("/root/children/0")
        );
        assert_eq!(
            pointer_in(SourceOf::Settings, Some("left"), "/root").as_deref(),
            Some("/views/left")
        );
        // (Text that stops reading: what was read so far.)
        assert_eq!(span("{\"root\": {\"a\": [1, 2]}, oops", "/root"), Some((9, 22)));
    }
}
