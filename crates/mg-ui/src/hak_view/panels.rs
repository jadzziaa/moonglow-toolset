//! What a hak brings beside its editor (M4's U21, the Hak board): the
//! Right Dock's **Preview** (the resource chosen: a picture, a sound with
//! Play or Stop, a table, a model in 3D, paused at first, with its facts,
//! the rest as text or fields; the model in a tab of its own from the
//! header's icon, its Close putting it away) over **Details** (File,
//! Contents, Built from with Update from Folder, Description); its
//! contents by type in the **Outliner** (`outliner::hak`); and the Status
//! Bar's hints and counts.

use super::*;

/// The hak an editor's tab is, if it is one's.
pub(crate) fn hak_of(editor: Option<&Tab>) -> Option<u32> {
    match editor? {
        Tab::Hak(id) => Some(*id),
        _ => None,
    }
}

/// The hak an editor's tab shows, and its place among those open.
fn doc_of(app: &Moonglow, editor: Option<&Tab>) -> Option<usize> {
    let id = hak_of(editor)?;
    app.haks.iter().position(|d| d.id == id)
}

/// The name of the Preview's Open the Model in a Tab (its icon's).
pub const OPEN_MODEL: &str = "Open the Model in a Tab";
/// The name of the Preview's Close (its icon's).
pub const PREVIEW_CLOSE: &str = "Close the Preview";
/// Update from Folder's name (in Details and in the header's More).
pub const UPDATE_FROM_FOLDER: &str = "Update from Folder";
/// The Description's name in Details.
pub const DETAILS_DESCRIPTION: &str = "Description";

/// What a hak's Preview and Details say after their titles (the board's
/// "c_masterius.mdl" and "id_resources.hak").
pub(crate) fn panel_detail(
    app: &Moonglow,
    editor: Option<&Tab>,
    panel: crate::shell::Panel,
) -> Option<String> {
    let d = &app.haks[doc_of(app, editor)?];
    Some(match panel {
        crate::shell::Panel::Details => d.title().trim_end_matches(" *").to_string(),
        _ => match d.preview_key() {
            Some(key) => key.to_string(),
            None => "Nothing chosen".to_string(),
        },
    })
}

/// Where a model shown in the Preview is read by the game (the module's
/// hak as saved): it has a tab of its own to open in.
fn model_read(app: &Moonglow, editor: Option<&Tab>) -> Option<ResKey> {
    let d = &app.haks[doc_of(app, editor)?];
    match &d.viewing {
        Some((key, _, Viewing::Model { read: true, .. })) if d.preview_key() == Some(*key) => {
            Some(*key)
        }
        _ => None,
    }
}

/// The Preview's header's icons (the board's `open-in-tab` and `close`):
/// the model in a tab of its own (off where the game reads not this copy,
/// its tip saying why), and Close. Whether Open was clicked; Close is noted
/// for the body, drawn next.
pub(crate) fn open_model_button(app: &Moonglow, ui: &mut Ui, editor: Option<&Tab>) -> bool {
    let Some(at) = doc_of(app, editor) else { return false };
    let d = &app.haks[at];
    let read = model_read(app, editor).is_some();
    let tip = match (&d.viewing, d.preview_key()) {
        (_, None) => "Choose a model to open it in a tab",
        (Some((_, _, Viewing::Model { .. })), Some(_)) if read => "Open the model in a tab",
        (Some((_, _, Viewing::Model { .. })), Some(_)) => {
            "Only a model the game reads from the hak as saved opens in a tab: \
             attach the hak to the module and save it"
        }
        _ => "Only a model opens in a tab",
    };
    let b = mg_uikit::IconButton::new(mg_uikit::Icon::OpenInTab, OPEN_MODEL).icon_only(true);
    let opened = ui.add_enabled(read, b).on_hover_text(tip).on_disabled_hover_text(tip).clicked();
    let shows = d.preview_key().is_some();
    let close = mg_uikit::IconButton::new(mg_uikit::Icon::Close, PREVIEW_CLOSE).icon_only(true);
    let tip = "Put the resource away: choosing another shows it";
    if ui.add_enabled(shows, close).on_hover_text(tip).on_disabled_hover_text(tip).clicked() {
        ui.data_mut(|m| m.insert_temp(egui::Id::new(("hak-preview-closed", d.id)), true));
    }
    opened
}

/// The model the Preview shows opened in a tab of its own.
pub(crate) fn open_model(app: &mut Moonglow, editor: Option<&Tab>) {
    if let Some(k) = model_read(app, editor) {
        app.actions.push(crate::Action::OpenTab(Tab::Model(k)));
    }
}

/// The least room left for a model's 3D view over its facts.
const MODEL_LEAST: f32 = 120.0;

/// A hak's Preview's body (the Hak board's): the resource chosen (the one
/// clicked last of those selected), made once as it is chosen and kept
/// while the hak stays as it is: a picture fitted with its size under it,
/// a sound's Play or Stop, a table's rows and columns, a model in 3D
/// (paused until its Play) over its facts, the rest as text or fields.
/// With none chosen, it says so. Its sound stops where another is chosen.
pub(crate) fn preview(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(id) = hak_of(editor) else { return };
    let tab = Tab::Hak(id);
    crate::side_doc::note_pointer(app, ui, &tab);
    let t = mg_uikit::tokens(ui.ctx());
    let room = ui.max_rect();
    ui.painter().rect_filled(room, 0.0, t.base);
    let Some(at) = doc_of(app, editor) else { return };
    app.haks[at].preview_pass = ui.ctx().cumulative_pass_nr();
    let closed = egui::Id::new(("hak-preview-closed", id));
    if ui.data_mut(|m| m.remove_temp::<bool>(closed)).is_some() {
        let d = &mut app.haks[at];
        d.closed = d.preview_key();
        stop_sound(app, at);
    }
    let d = &app.haks[at];
    let Some(key) = d.preview_key() else {
        let words = match d.selected.is_empty() {
            true => "Choose a resource to see it.",
            false => "Double-click a resource to see it.",
        };
        say(ui, words);
        return;
    };
    // (Made as it is chosen, once for the hak as it is.)
    let fresh = d.viewing.as_ref().is_some_and(|(k, c, _)| *k == key && *c == d.revision());
    if !fresh {
        match app.haks[at].hak.data(key) {
            Ok(data) => {
                let view = viewing(app, ui.ctx(), at, key, data);
                let revision = app.haks[at].revision();
                app.haks[at].viewing = Some((key, revision, view));
            }
            Err(e) => {
                say(ui, &format!("{key}: {e}"));
                return;
            }
        }
    }
    // (Taken out while drawn: a table's view and the model viewer take
    // the window's state.)
    let Some((key, revision, view)) = app.haks[at].viewing.take() else { return };
    // (Its views of other resources, or of the hak as it was, go: arrowed
    // through, a hak's models were each kept.)
    let a_hak_model = matches!(view, Viewing::Model { read: false, .. });
    app.model_views.retain(|s, _| match s {
        crate::model_view::Source::Hak { hak, revision: r, key: k } if *hak == id => {
            a_hak_model && *r == revision && *k == key
        }
        _ => true,
    });
    match &view {
        // (The viewers the read-only tab shows them with too: M5's W6.)
        Viewing::Picture(picture) => picture.ui(ui, key, room),
        Viewing::Sound(data) => sound(app, ui, at, key, data),
        Viewing::Table(table, made) => {
            let label = table.copies[0].0.clone();
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(room.shrink(6.0)));
            crate::browser::two_da(app, &mut child, key, Some(&label), (table, *made), &tab);
        }
        Viewing::Model { read, facts } => model(app, ui, at, key, *read, facts),
        Viewing::Plain(plain) => {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(room.shrink(6.0)));
            let names = crate::ro_gff::Names { game: app.game.as_deref(), factions: &[] };
            crate::browser::plain_ui(&mut child, key, plain, names);
        }
    }
    if let Some(d) = app.haks.get_mut(at)
        && d.viewing.is_none()
    {
        d.viewing = Some((key, revision, view));
    }
}

/// Words at the panel's top left, quiet.
fn say(ui: &mut Ui, words: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.weak(words);
    });
}

use crate::viewers::caption_at;

/// The sound the Preview plays stopped, where it is hak `at`'s (and
/// still holds the channel).
fn stop_sound(app: &mut Moonglow, at: usize) {
    if app.haks[at].playing.take().is_some() {
        app.hak_sound = None;
        crate::viewers::stop_by(app, &crate::viewers::Player::Hak(app.haks[at].id));
    }
}

/// A sound's Preview: the sound viewer the read-only tab plays with too
/// (`viewers::sound_ui`: its Play or Stop, its size, a WAVE under .bmu
/// said), the hak noting that it plays, for its sound to stop with the
/// hak closed or another module open (`follow_all`).
fn sound(app: &mut Moonglow, ui: &mut Ui, at: usize, key: ResKey, data: &Arc<[u8]>) {
    use crate::viewers::{Player, Pressed};
    let by = Player::Hak(app.haks[at].id);
    // (Ended, or another Play took the channel: the hak lets it go.)
    if app.haks[at].playing == Some(key) && !crate::viewers::plays(app, &by, key) {
        app.haks[at].playing = None;
        app.hak_sound = None;
    }
    match crate::viewers::sound_ui(app, ui, ui.max_rect(), by, key, data) {
        Some(Pressed::Play) => {
            app.haks[at].playing = Some(key);
            app.hak_sound = Some((app.haks[at].id, app.module_path()));
        }
        Some(Pressed::Stop) => {
            app.haks[at].playing = None;
            app.hak_sound = None;
        }
        None => {}
    }
}

/// A model's Preview: its 3D view, paused until its Play (a model's
/// emitters and the game's programs that read the clock asked for a frame
/// every step), over its facts (the board's `.facts`) and where it is
/// drawn from.
fn model(app: &mut Moonglow, ui: &mut Ui, at: usize, key: ResKey, read: bool, facts: &ModelFacts) {
    use crate::model_view::Source;
    let room = ui.max_rect();
    let t = mg_uikit::tokens(ui.ctx());
    let rows = facts.rows();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let pitch = 20.0;
    let note = match read {
        true => "Drawn as the game reads it, from the hak as saved.",
        false => "Drawn from the hak as it is now.",
    };
    let facts_high = rows.len() as f32 * pitch + 12.0 + 22.0;
    let view_room = room.height() - facts_high;
    let facts_top = match view_room >= MODEL_LEAST {
        true => room.bottom() - facts_high,
        false => room.top() + 10.0,
    };
    if view_room >= MODEL_LEAST {
        let view = room.with_max_y(facts_top);
        let tab = Tab::Model(key);
        let popped = read
            && (app.open_models.contains(&tab)
                || app.panes_tabs.iter().flatten().any(|t| *t == tab));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(view).id_salt("hak-model"));
        child.set_clip_rect(view.intersect(ui.clip_rect()));
        if popped {
            child.add_space(6.0);
            child.horizontal(|ui| {
                ui.add_space(10.0);
                ui.weak("The model is in a tab of its own.");
            });
        } else {
            let source = match read {
                true => Source::Resource(key),
                false => {
                    resources(app, at);
                    let (hak, revision) = (app.haks[at].id, app.haks[at].revision());
                    Source::Hak { hak, revision, key }
                }
            };
            let shown = crate::model_view::Shown {
                info: false,
                pop_out: false,
                restore: false,
                paused: true,
            };
            crate::model_view::panel(app, &mut child, source, shown);
        }
    }
    // The facts: a word at the right of its column, the value at its left.
    let words_width = 84.0;
    let left = room.center().x - (words_width + 8.0 + 90.0) / 2.0;
    let mut y = facts_top + 10.0;
    for (word, value) in &rows {
        let w = ui.painter().layout_no_wrap(word.to_string(), font.clone(), t.text_3);
        ui.painter().galley(egui::pos2(left + words_width - w.size().x, y), w, t.text_3);
        let mut job =
            egui::text::LayoutJob::simple_singleline(value.clone(), font.clone(), t.text_2);
        job.wrap = egui::text::TextWrapping::truncate_at_width(
            (room.right() - left - words_width - 18.0).max(1.0),
        );
        let v = ui.painter().layout_job(job);
        ui.painter().galley(egui::pos2(left + words_width + 8.0, y), v, t.text_2);
        y += pitch;
    }
    let facts_rect = egui::Rect::from_x_y_ranges(room.x_range(), facts_top..=y);
    let r = ui.interact(facts_rect, ui.id().with("hak-model-facts"), egui::Sense::hover());
    r.widget_info(|| {
        let said: Vec<String> = rows.iter().map(|(w, v)| format!("{w} {v}")).collect();
        egui::WidgetInfo::labeled(
            egui::WidgetType::Label,
            true,
            format!("Facts: {}", said.join(", ")),
        )
    });
    caption_at(ui, room, note);
}

/// A hak's Details' body (the Hak board's): File (the path cut at its
/// start, whole in its tip), Contents ("2706 resources, 183.8 MB"), Built
/// from (the folder, or that it was not built from one) with Update from
/// Folder under it, and the Description (set when the keyboard leaves it,
/// on Save, on closing, with another hak in front: one step to undo) with
/// its note.
pub(crate) fn details(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(id) = hak_of(editor) else { return };
    crate::side_doc::note_pointer(app, ui, &Tab::Hak(id));
    let keys = crate::palette_view::Keys::HakDetails;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    // (F6 or a press here: Undo and Redo are the hak's, as with its
    // table's keys; the Top Bar's Undo pressed after typing takes back
    // what was typed.)
    if crate::palette_view::keys_are(ui.ctx(), keys) {
        app.undo_keys = Some(Tab::Hak(id));
    }
    let focus = app.dialog_panel_focus.take_if(|k| *k == keys).is_some();
    let Some(at) = doc_of(app, editor) else { return };
    let facts = app.haks[at].facts();
    let d = &mut app.haks[at];
    let t = mg_uikit::tokens(ui.ctx());
    let label_width = 76.0;
    let mut update = false;
    egui::ScrollArea::vertical().id_salt(("hak-details", id)).auto_shrink([false, false]).show(
        ui,
        |ui| {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 8)).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                let font = egui::TextStyle::Body.resolve(ui.style());
                let row = |ui: &mut Ui, label: &str, add: &mut dyn FnMut(&mut Ui, f32)| {
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(label_width, mg_uikit::metrics::CONTROL_HEIGHT),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.set_min_width(label_width);
                                crate::widgets::field_label(ui, label);
                            },
                        );
                        let room = ui.available_width();
                        add(ui, room);
                    });
                };
                let path = d.hak.path.as_ref().map(|p| p.display().to_string());
                row(ui, "File", &mut |ui, room| {
                    let (shown, tip) = match &path {
                        Some(p) => (cut_start(ui, p, &font, room - 4.0), p.clone()),
                        None => {
                            ("not saved yet".to_string(), "The hak is not saved yet".to_string())
                        }
                    };
                    ui.add(egui::Label::new(egui::RichText::new(shown).color(t.text_2)).truncate())
                        .on_hover_text(tip);
                });
                row(ui, "Contents", &mut |ui, _| {
                    ui.add(egui::Label::new(egui::RichText::new(facts.words()).color(t.text_2)));
                });
                let folder = d.folder.as_ref().map(|f| f.display().to_string());
                row(ui, "Built from", &mut |ui, room| match &folder {
                    Some(f) => {
                        let shown = cut_start(ui, f, &font, room - 4.0);
                        ui.add(
                            egui::Label::new(egui::RichText::new(shown).color(t.text_2)).truncate(),
                        )
                        .on_hover_text(f);
                    }
                    None => {
                        ui.add(egui::Label::new(
                            egui::RichText::new("(not built from a folder)").color(t.text_3),
                        ));
                    }
                });
                row(ui, "", &mut |ui, _| {
                    let tip = match &folder {
                        Some(f) => {
                            format!("Read {f} again: the hak's files as the folder has them now")
                        }
                        None => "For a hak built with Build Hak from Folder".to_string(),
                    };
                    let b = mg_uikit::KitButton::new(None, UPDATE_FROM_FOLDER).tip(tip.clone());
                    let r = ui.add_enabled(folder.is_some(), b).on_disabled_hover_text(tip);
                    update = r.clicked();
                });
                // The Description: typed text kept to its hak (its field's
                // id is the hak's), set as the keyboard leaves it.
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(label_width, mg_uikit::metrics::CONTROL_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_min_width(label_width);
                            crate::widgets::field_label(ui, DETAILS_DESCRIPTION);
                        },
                    );
                    ui.vertical(|ui| {
                        let width = ui.available_width();
                        let text = d.description_text();
                        let field = egui::TextEdit::multiline(text)
                            .desired_rows(3)
                            .desired_width(width)
                            .min_size(egui::vec2(width, 62.0));
                        let out = crate::widgets::text_area(ui, description_id(id), field);
                        let r = out.response;
                        r.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::TextEdit,
                                true,
                                DETAILS_DESCRIPTION,
                            )
                        });
                        if focus {
                            r.request_focus();
                        }
                        crate::widgets::leaves_at_a_press(ui.ctx(), &r);
                        if r.changed() {
                            d.typed = true;
                        }
                        if r.lost_focus() {
                            d.set_typed();
                        }
                        ui.add(egui::Label::new(
                            egui::RichText::new("For people: the game doesn't show it.")
                                .size(11.0)
                                .color(t.text_3),
                        ));
                    });
                });
            });
        },
    );
    if update {
        update_from_folder(app, at);
    }
}

/// The Outliner's rows of hak `id`: its types with how many of each, the
/// most first (counted once for each change of the hak).
pub(crate) fn outline_types(app: &mut Moonglow, id: u32) -> Option<Arc<Facts>> {
    let d = app.haks.iter_mut().find(|d| d.id == id)?;
    Some(d.facts())
}

/// What the Status Bar shows beside a hak (the Hak board's): with the
/// table with the keys, Up Down Move in the list and Enter View; with the
/// pointer over its editor, Click Select, Ctrl + Click Add to selection (Cmd
/// on macOS),
/// Shift + Click Select range, Double Click View, Right Click More; none
/// while a field is typed in, a popover, dialog or menu is open. Its
/// counts: "2706 resources, 183.8 MB" (its path in their tip) and "3
/// selected" where any are.
pub(crate) fn shown(
    app: &Moonglow,
    k: &crate::shell::HintKeys<'_>,
    ctx: &egui::Context,
    id: u32,
) -> crate::shell::Shown {
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let Some(d) = app.haks.iter().find(|d| d.id == id) else { return Shown::default() };
    let mut counts = Vec::new();
    if let Some(f) = d.facts_seen() {
        let mut c = Count::new(f.words());
        c.tip = d.hak.path.as_ref().map(|p| p.display().to_string());
        counts.push(c);
    }
    if !d.selected.is_empty() {
        counts.push(Count::new(format!("{} selected", d.selected.len())));
    }
    let mut hints = Vec::new();
    let typing = ctx.memory(|m| m.focused().is_some()) && ctx.egui_wants_keyboard_input();
    if crate::shell::keys_blocked(ctx) || mg_uikit::popover::Popover::any_open(ctx) || typing {
        return Shown { hints, counts, note: None };
    }
    // (The table with the keys: the arrows and Enter, then the pointer's.)
    if d.has_rows_shown() && super::table_has_keys(app, ctx, id) {
        hints.push(Hint::either(["Up", "Down"], "Move in the list"));
        hints.push(Hint::keys(["Enter"], "View"));
    }
    if d.hovered && d.has_rows_shown() {
        hints.push(Hint::keys(["Click"], "Select"));
        hints.push(Hint::keys([k.ctrl(), "Click"], "Add to selection"));
        hints.push(Hint::keys(["Shift", "Click"], "Select range"));
        hints.push(Hint::keys(["Double Click"], "View"));
        hints.push(Hint::keys(["Right Click"], "More"));
    }
    // (No Save's keys: they are the module's Save, as beside a 2DA, a talk
    // table or a tileset (the owner's ruling of 2026-11-18); the header's
    // Save saves the hak.)
    Shown { hints, counts, note: None }
}
