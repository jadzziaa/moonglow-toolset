//! The talk table editor's panels (M4's U17, the Talk board's Right Dock):
//! **String** (the chosen line's text, the feminine text where the table
//! has one, its sound and the sound's length: typed in, each burst one step
//! of the table's Undo; read-only for a table only looked at), **References**
//! (what names the line: found on a thread of its own, as a 2DA's row's
//! users are, `table_view::panels`) and **Talk Table** (the table's lines,
//! language, feminine table and file; the project's other languages; and,
//! for a module with no table of its own, Module's Table: what makes one).
//! Drawn before the editor (the docks first; a floating table's window's
//! pane before its editor), they show what the list left the frame before.
//! The Status Bar's count and hints are [`shown`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use egui::Ui;
use mg_module::rename::{HakCell, Usage};
use mg_module::talk::{self, Line, Table};
use mg_uikit::Icon;

use super::{TalkView, Which, parts, parts_of};
use crate::{Action, Moonglow, Tab};

/// What Find References found for a line: the line, the module's revision
/// it looked in (a module changed since, or another opened, lets it go),
/// the module's fields, scripts and 2DAs that name it, and the haks' 2DAs'
/// cells.
pub(crate) type Found = (usize, u64, Vec<Usage>, Vec<HakCell>);

/// Find References for a line, on a thread of its own: the line, the flag
/// that stops it (asked between resources), and what it finds.
#[derive(Debug)]
pub(crate) struct Looking {
    row: usize,
    revision: u64,
    stop: Arc<AtomicBool>,
    #[allow(clippy::type_complexity)]
    found: Option<std::thread::JoinHandle<Option<(Vec<Usage>, Vec<HakCell>)>>>,
}

/// (Let go of anyhow, its table closed or another chosen: stopped, not
/// left running through a large module for no one.)
impl Drop for Looking {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The talk table an editor shown in the docks or its window is.
fn which_of(app: &Moonglow, editor: Option<&Tab>) -> Option<Which> {
    Which::of(app, editor?)
}

/// The panels' header's detail: String's "StrRef 18608, line 18608",
/// References' "Talk-table line 18608", Talk Table's file's name.
pub(crate) fn detail(
    app: &Moonglow,
    editor: Option<&Tab>,
    panel: crate::shell::Panel,
) -> Option<String> {
    use crate::shell::Panel;
    let which = which_of(app, editor)?;
    let Some((t, v)) = parts_of(app, which) else {
        return (panel == Panel::TalkFile).then(|| "No talk table".to_string());
    };
    match panel {
        Panel::TalkFile => Some(format!("{}.tlk", t.name)),
        _ => {
            let Some(row) = v.selected else { return Some("No line chosen".to_string()) };
            Some(match panel {
                Panel::References => format!("Talk-table line {}", which.strref(row)),
                _ => format!("StrRef {}, line {row}", which.strref(row)),
            })
        }
    }
}

/// The panels' own room around what they hold (the board's 8 and 6).
const MARGIN: egui::Margin = egui::Margin { left: 8, right: 8, top: 6, bottom: 6 };
/// The board's label column (`.str-form`'s 76).
const LABEL: f32 = 76.0;
/// The least a field keeps beside its label: under it, the label stands
/// over the field.
const FIELD_LEAST: f32 = 120.0;
/// The text's box (`.text-memo`): 128 high, a longer text scrolling in it;
/// with a feminine table, each of the two 72 (the section keeps its
/// sound's rows in sight).
const TEXT_HIGH: f32 = 128.0;
const TEXTS_HIGH: f32 = 72.0;

/// A row of the form: `label` in its column and what `add` draws beside
/// it, or the label over it where the room beside it is less than
/// [`FIELD_LEAST`] (the room tested is the room given, the row's gap
/// counted: never taller where wider). `top`: the label at the field's top
/// (a box of several lines).
fn form_row<R>(ui: &mut Ui, label: &str, top: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let words = ui.painter().layout_no_wrap(label.to_string(), font, egui::Color32::WHITE);
    let width = LABEL.max(words.size().x.ceil() + 8.0);
    let gap = ui.spacing().item_spacing.x;
    let t = mg_uikit::tokens(ui.ctx());
    let text = egui::RichText::new(label).color(t.text_2);
    if ui.available_width() - width - gap >= FIELD_LEAST {
        let row = |ui: &mut Ui, add: Box<dyn FnOnce(&mut Ui) -> R + '_>| match top {
            true => ui.horizontal_top(add).inner,
            false => ui.horizontal(add).inner,
        };
        row(
            ui,
            Box::new(|ui: &mut Ui| {
                let size = egui::vec2(width, mg_uikit::metrics::CONTROL_HEIGHT);
                let left = egui::Layout::left_to_right(egui::Align::Center);
                ui.allocate_ui_with_layout(size, left, |ui| {
                    ui.set_min_size(size);
                    ui.add(egui::Label::new(text).truncate());
                });
                add(ui)
            }),
        )
    } else {
        ui.add(egui::Label::new(text).truncate());
        add(ui)
    }
}

/// A weak note that wraps at the panel's width.
fn note(ui: &mut Ui, text: impl Into<String>) -> egui::Response {
    let t = mg_uikit::tokens(ui.ctx());
    ui.add(egui::Label::new(egui::RichText::new(text.into()).color(t.text_3)).wrap())
}

/// A section's heading (`.sec-h`): bold, a soft line under it.
fn heading(ui: &mut Ui, name: &str) {
    let t = mg_uikit::tokens(ui.ctx());
    ui.add_space(10.0);
    let body = egui::TextStyle::Body.resolve(ui.style());
    let bold = egui::FontId::new(body.size + 2.0, mg_uikit::fonts::bold());
    ui.add(egui::Label::new(egui::RichText::new(name).font(bold).color(t.text)).truncate());
    ui.add_space(6.0);
}

/// A panel begins: outlined where F6 gave it the keys; whether F6 came to
/// it (its first control then takes the keyboard's focus).
fn begin_panel(app: &mut Moonglow, ui: &Ui, keys: crate::palette_view::Keys) -> bool {
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    let to = app.dialog_panel_focus == Some(keys);
    if to {
        app.dialog_panel_focus = None;
    }
    to
}

/// The ids of the String panel's fields: their table's and their line's,
/// so that what is typed for one line is never shown or set in another's.
fn field_id(which: Which, field: &str, row: usize) -> egui::Id {
    egui::Id::new(("talk-string", which, field, row))
}

/// The String panel's Text field's id for `row` (the tests', F6's, Enter's).
pub fn text_id(which: Which, row: usize) -> egui::Id {
    field_id(which, "text", row)
}

/// The String panel (M4's U17, the Talk board's): the chosen line's Text
/// (Text (masculine) and Feminine where the table has a feminine one), its
/// Sound and Length (s); typed in, each burst of typing one step of the
/// table's Undo, set as it is typed (a line chosen meanwhile never takes
/// it: the fields are the line's own). Read-only for a table only looked
/// at, which says why. Copy puts the line's StrRef on the clipboard.
pub(crate) fn string(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(which) = which_of(app, editor) else { return };
    let editable = super::editable(app, which);
    if which == Which::Module {
        crate::side_doc::note_pointer(app, ui, &Tab::TalkTable);
    }
    let focus = begin_panel(app, ui, crate::palette_view::Keys::TalkString);
    // (Read-only: where it is kept, and where Moonglow edits tables.)
    let why = match which {
        Which::Game => Some(
            "Read-only: the game's own talk table. Moonglow edits tables in the user folder's \
             tlk and in the module."
                .to_string(),
        ),
        Which::Module => app.talk.as_ref().filter(|t| !t.editable()).map(|t| {
            let source = t.feminine.iter().map(|(s, _)| s).find(|s| !s.writable());
            let source = source.filter(|_| t.source.writable()).unwrap_or(&t.source);
            format!(
                "Read-only: this table is in {source}: change it where that is built (Moonglow \
                 edits tables in the user folder's tlk and in the module)."
            )
        }),
    };
    let scroll =
        egui::ScrollArea::vertical().id_salt(("talk-string-panel", which)).auto_shrink(false);
    let mut set: Option<(usize, Line, Option<u8>)> = None;
    // (A field left: its burst of typing ends.)
    let mut left = false;
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(MARGIN).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let Some((t, v)) = parts(app, which) else {
                note(ui, "No talk table is open.");
                return;
            };
            let Some(row) = v.selected.filter(|r| *r < t.len()) else {
                note(ui, "Choose a line to see its text here.");
                return;
            };
            if let Some(why) = &why {
                note(ui, why);
                ui.add_space(4.0);
            }
            let current = t.line(row);
            if v.edit.as_ref().is_none_or(|(r, _, applied)| *r != row || *applied != current) {
                v.edit = Some((row, super::shown_line(&current), current));
                v.error = None;
            }
            let (_, line, _) = v.edit.as_mut().expect("set");
            let before = line.clone();
            let to_text = v.to_text.take().is_some() || focus;
            let feminine = line.feminine.is_some();
            let high = if feminine { TEXTS_HIGH } else { TEXT_HIGH };
            let mut typing = None;
            // A text's box: 128 high, a longer text scrolling in it.
            let text_box =
                |ui: &mut Ui, id: egui::Id, text: &mut String, name: &str, hint: &str| {
                    let width = ui.available_width();
                    egui::ScrollArea::vertical()
                        .id_salt(id.with("scroll"))
                        .max_height(high)
                        .min_scrolled_height(high)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut looked = text.as_str();
                            let edit = match editable {
                                true => egui::TextEdit::multiline(text),
                                // (Looked at: its words chosen and copied, none
                                // changed.)
                                false => egui::TextEdit::multiline(&mut looked),
                            };
                            let edit = edit
                                .desired_width(width)
                                .min_size(egui::vec2(0.0, high))
                                .hint_text(if editable { hint } else { "" });
                            let out = crate::widgets::text_area(ui, id, edit);
                            // (Named by its label, as a screen reader says it.)
                            ui.ctx().accesskit_node_builder(out.response.id, |n| n.set_label(name));
                            out.response
                        })
                        .inner
                };
            let text_label = if feminine { "Text (masculine)" } else { "Text" };
            let r = form_row(ui, text_label, true, |ui| {
                text_box(ui, text_id(which, row), &mut line.text, text_label, "the line's text")
            });
            if to_text {
                r.request_focus();
            }
            left |= r.lost_focus();
            if r.changed() {
                typing = Some(0);
            }
            if let Some(f) = &mut line.feminine {
                ui.add_space(4.0);
                let r = form_row(ui, "Feminine", true, |ui| {
                    text_box(
                        ui,
                        field_id(which, "feminine", row),
                        f,
                        "Feminine",
                        "the feminine text",
                    )
                });
                left |= r.lost_focus();
                if r.changed() {
                    typing = Some(1);
                }
            }
            ui.add_space(4.0);
            if !editable {
                // (Looked at: Sound and Length as text that can be chosen
                // and copied, as the Talk board draws them, none changed.)
                form_row(ui, "Sound", false, |ui| {
                    looked_field(
                        ui,
                        field_id(which, "sound", row),
                        "Sound",
                        &line.sound,
                        150.0,
                        true,
                    )
                });
                form_row(ui, "Length (s)", false, |ui| {
                    let length = format!("{:.2}", line.sound_length);
                    looked_field(
                        ui,
                        field_id(which, "length", row),
                        "Length (s)",
                        &length,
                        72.0,
                        false,
                    )
                });
            } else {
                form_row(ui, "Sound", false, |ui| {
                    let mut sound = crate::widgets::ResRefText(&mut line.sound);
                    let field = crate::widgets::resref_edit(&mut sound)
                        .id(field_id(which, "sound", row))
                        .font(egui::TextStyle::Monospace)
                        .desired_width(150.0_f32.min(ui.available_width()));
                    let r = ui.add(field);
                    let value = line.sound.clone();
                    r.widget_info(|| {
                        let mut info = egui::WidgetInfo::text_edit(true, &value, &value, "");
                        info.label = Some("Sound".to_string());
                        info
                    });
                    left |= r.lost_focus();
                    if r.changed() {
                        typing = Some(2);
                    }
                });
                form_row(ui, "Length (s)", false, |ui| {
                    let drag = egui::DragValue::new(&mut line.sound_length)
                        .speed(0.05)
                        .range(0.0..=600.0)
                        .fixed_decimals(2);
                    // (Its id its line's, as the other fields': what is typed
                    // in it never goes to another line's.)
                    let r = ui
                        .push_id((which, "length", row), |ui| {
                            ui.add_sized([72.0, mg_uikit::metrics::CONTROL_HEIGHT], drag)
                        })
                        .inner;
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, "Length (s)")
                    });
                    left |= r.lost_focus();
                    if r.changed() {
                        typing = Some(3);
                    }
                });
            }
            if *line != before && editable {
                set = Some((row, line.clone(), typing));
            }
            if let Some(e) = &v.error {
                let color = ui.visuals().error_fg_color;
                ui.add(egui::Label::new(egui::RichText::new(e).color(color)).wrap());
            }
        });
    });
    // (Set after the panel let go of the table: the list's lines, Find and
    // Only lines with text follow it.)
    if let Some((row, line, typing)) = set
        && let Moonglow { talk: Some(t), talk_view: v, .. } = app
        && let Some((_, _, applied)) = v.edit.clone()
    {
        match t.set_line(row, &super::stored_line(&line, &applied), typing) {
            Ok(()) => {
                let now = t.line(row);
                if let Some((_, _, a)) = v.edit.as_mut() {
                    *a = now;
                }
                v.error = None;
                v.line_changed(t, row);
            }
            Err(e) => v.error = Some(e),
        }
    }
    // (After what was typed is set: the next typing a step of its own.)
    if left
        && which == Which::Module
        && let Some(t) = app.talk.as_mut()
    {
        t.end_typing();
    }
}

/// String's header's tool (the Talk board's `.panel-tabs .tools`): Copy
/// the StrRef of the line chosen, onto the clipboard.
pub(crate) fn copy_tool(app: &Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let which = which_of(app, editor);
    let chosen = which.and_then(|w| Some((w, parts_of(app, w)?.1.selected?)));
    let button = mg_uikit::IconButton::new(Icon::Copy, COPY_STRREF).icon_only(true);
    let r = ui.add_enabled(chosen.is_some(), button).on_hover_text(COPY_STRREF_TIP);
    if r.clicked()
        && let Some((which, row)) = chosen
    {
        ui.ctx().copy_text(which.strref(row).to_string());
    }
}

/// Copy the StrRef's name and tip.
pub const COPY_STRREF: &str = "Copy the StrRef";
const COPY_STRREF_TIP: &str = "Copy the StrRef of the line chosen";

/// `path` broken into lines at its separators (each line as many of its
/// parts, a separator ending each, as fit in `width`; a part longer than
/// the width a line of its own, cut by the panel's edge).
fn at_separators(ui: &Ui, path: &str, font: &egui::FontId, width: f32) -> String {
    let wide = |s: &str| {
        let g = ui.painter().layout_no_wrap(s.to_string(), font.clone(), egui::Color32::WHITE);
        g.size().x
    };
    crate::shown_path::lines_at_separators(path, crate::shown_path::WINDOWS, |s| wide(s) <= width)
}

/// A place that names a line, as the Talk board's References row says it:
/// the line's text quoted (its first words) before the place's last part
/// ("m1q1faribeth › NPC line 41 “Welcome, <FullName>. I regret I did not ”
/// › Text").
fn quoted(place: &str, text: &str) -> String {
    let first = text.lines().next().unwrap_or_default();
    let words: String = first.chars().take(40).collect();
    match place.rsplit_once(" › ") {
        Some((before, last)) if !words.is_empty() => format!("{before} “{words}” › {last}"),
        _ => place.to_string(),
    }
}

/// A field only looked at (the Talk board's `.field.ro`): its text can be
/// chosen and copied, never changed; named `name`, `width` wide at most,
/// in the monospace font where `mono`.
fn looked_field(ui: &mut Ui, id: egui::Id, name: &str, text: &str, width: f32, mono: bool) {
    let mut shown = text;
    let mut edit = egui::TextEdit::singleline(&mut shown)
        .id(id)
        .desired_width(width.min(ui.available_width()));
    if mono {
        edit = edit.font(egui::TextStyle::Monospace);
    }
    let r = ui.add(edit);
    let value = text.to_string();
    r.widget_info(|| {
        let mut info = egui::WidgetInfo::text_edit(true, &value, &value, "");
        info.label = Some(name.to_string());
        info
    });
}

/// Starts Find References for line `row` of `which` on a thread of its
/// own: the module's fields, scripts, 2DAs and sound sets that name its
/// StrRef (`UsageIndex::strref_usages_until`, read a resource at a time,
/// the stop flag asked between them), then the haks' 2DAs' cells. One under
/// way is stopped first.
pub(crate) fn find_references(app: &mut Moonglow, which: Which, row: usize) {
    let strref = which.strref(row);
    // (The module as its open documents have it, as the References tab
    // reads it.)
    let module = app.ws.as_mut().and_then(|ws| ws.flush().ok().map(|()| ws.module.clone()));
    let revision = app.ws.as_ref().map_or(0, |ws| ws.revision());
    let game = app.game.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let hold = app.hold_reference_looks.clone();
    let found = std::thread::spawn(move || {
        let stop = || stopped.load(Ordering::Relaxed);
        // (Held at its start by a test, until it lets go or it is stopped.)
        while hold.load(Ordering::Relaxed) && !stop() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let uses = match &module {
            Some(m) => mg_module::rename::UsageIndex::strref_usages_until(m, strref, &stop)?,
            None => Vec::new(),
        };
        if stop() {
            return None;
        }
        let cells = game.as_deref().map(|g| mg_module::rename::hak_strrefs(&g.resman, strref));
        Some((uses, cells.unwrap_or_default()))
    });
    let Some((_, v)) = parts(app, which) else { return };
    v.looking = Some(Looking { row, revision, stop, found: Some(found) });
    v.references = None;
}

/// Each frame, for each talk table: a Find References that has ended is
/// taken in, one for a line no longer chosen (or a table changed) is
/// stopped, and where one is under way a frame is asked for soon.
pub(crate) fn follow_looking(v: &mut TalkView, ctx: &egui::Context) {
    let Some(looking) = &v.looking else { return };
    if v.selected != Some(looking.row) {
        v.looking = None;
        return;
    }
    if !looking.found.as_ref().is_some_and(std::thread::JoinHandle::is_finished) {
        ctx.request_repaint_after(std::time::Duration::from_millis(30));
        return;
    }
    let mut looking = v.looking.take().expect("looking");
    let (row, revision) = (looking.row, looking.revision);
    if let Some(Ok(Some((uses, cells)))) = looking.found.take().map(|f| f.join()) {
        v.references = Some((row, revision, uses, cells));
    }
}

/// How many of each group the References panel lists at most.
const LISTED: usize = 200;

/// The References panel (M4's U17, the Talk board's, under String): Find
/// References to Line N, and what it found for the line: "Used in N
/// places:", the module's fields, scripts and 2DAs, each a link that goes
/// there, and the haks' 2DAs' cells. It looks on a thread of its own, the
/// window going on: a spinner and "Looking through the module…" while it
/// does, stopped as another line is chosen or the table closes; "Nothing
/// names line N" where nothing does.
pub(crate) fn references(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(which) = which_of(app, editor) else { return };
    let ctx = ui.ctx().clone();
    let focus = begin_panel(app, ui, crate::palette_view::Keys::TableReferences);
    let has_module = app.ws.is_some();
    let revision = app.ws.as_ref().map_or(0, |ws| ws.revision());
    let Some((table, v)) = parts(app, which) else { return };
    follow_looking(v, &ctx);
    // (Found in another module, or this one as it was: let go of.)
    if v.references.as_ref().is_some_and(|r| r.1 != revision) {
        v.references = None;
    }
    let mut find = None;
    let mut go: Option<Usage> = None;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("talk-references-panel", which)).auto_shrink(false);
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(MARGIN).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let Some(row) = v.selected else {
                note(ui, "Choose a line: the blueprints, areas, conversations, scripts and 2DAs that name it are found here.");
                return;
            };
            let strref = which.strref(row);
            if !has_module {
                note(ui, "Open a module: what names the line is found in it and its haks.");
                return;
            }
            let looking = v.looking.as_ref().is_some_and(|l| l.row == row);
            let found = v.references.as_ref().filter(|r| r.0 == row);
            if found.is_none() || looking {
                let button = mg_uikit::KitButton::new(Some(Icon::References), format!("Find References to Line {strref}"))
                    .tip("Where the module's blueprints, areas, conversations, scripts and 2DAs name this line");
                let r = ui.add_enabled(!looking, button);
                if focus {
                    r.request_focus();
                }
                if r.clicked() {
                    find = Some(row);
                }
                ui.add_space(4.0);
            }
            if looking {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(14.0));
                    note(ui, "Looking through the module and its haks…");
                });
                return;
            }
            let Some((_, _, uses, cells)) = found else { return };
            if uses.is_empty() && cells.is_empty() {
                note(ui, format!("Nothing names line {strref}: no field, script or 2DA of the module, nor a hak's 2DA."));
                return;
            }
            let t = mg_uikit::tokens(ui.ctx());
            let places = uses.len() + cells.len();
            let words = match places {
                1 => "Used in 1 place:".to_string(),
                n => format!("Used in {n} places:"),
            };
            ui.add(egui::Label::new(egui::RichText::new(words).color(t.text_2)).truncate());
            let line = |ui: &mut Ui, icon: Icon, words: &str, link: bool| -> egui::Response {
                ui.horizontal(|ui| {
                    let size = egui::Vec2::splat(mg_uikit::metrics::ICON);
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    icon.paint(ui.painter(), rect, t.text_3);
                    match link {
                        true => {
                            let text = egui::RichText::new(words).color(t.accent);
                            let label = egui::Label::new(text).truncate().sense(egui::Sense::click());
                            let r = ui.add(label);
                            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, words));
                            r.on_hover_cursor(egui::CursorIcon::PointingHand)
                        }
                        false => ui.add(egui::Label::new(words).truncate()),
                    }
                })
                .inner
            };
            let mut first = true;
            for u in uses.iter().take(LISTED) {
                let icon = match u.instance() {
                    Some((list, _)) => mg_area::ObjectKind::ALL
                        .into_iter()
                        .find(|k| k.list() == list)
                        .map_or(Icon::Area, crate::icons::object),
                    None => crate::browser::kind_icon(u.from.restype),
                };
                let words = quoted(&u.place, &table.line(row).text);
                let r = line(ui, icon, &words, true).on_hover_text("Go there");
                if focus && first {
                    r.request_focus();
                }
                first = false;
                if r.clicked() {
                    go = Some(u.clone());
                }
            }
            if uses.len() > LISTED {
                note(ui, format!("and {} more in the module", uses.len() - LISTED));
            }
            for c in cells.iter().take(LISTED) {
                line(ui, Icon::Table, &c.place(), false);
            }
            if cells.len() > LISTED {
                note(ui, format!("and {} more in the haks", cells.len() - LISTED));
            }
            // (The game's table's lines are named by the game's own 2DAs
            // too, which are not looked in: said so.)
            if which == Which::Game {
                note(ui, "The game's own 2DAs are not searched.");
            }
        });
    });
    if let Some(row) = find {
        find_references(app, which, row);
        ctx.request_repaint();
    }
    if let Some(u) = go {
        app.go_to_usage(&u);
    }
}

/// The Talk Table panel (M4's U17, the Talk board's): the table's Lines,
/// Language (the project's other languages of it in a dropdown, where it
/// keeps several: the old info line's), Feminine and File; and Module's
/// Table, the page that makes the module's own where it has none (or one
/// the game can't find).
pub(crate) fn file(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(which) = which_of(app, editor) else { return };
    if which == Which::Module {
        crate::side_doc::note_pointer(app, ui, &Tab::TalkTable);
    }
    let focus = begin_panel(app, ui, crate::palette_view::Keys::TalkFile);
    // (The module's table, read once here for those after: whether it has
    // one, and which.)
    let named = if app.ws.is_some() { super::load(app) } else { None };
    let own_open = app.talk.as_ref().is_some_and(|t| Some(&t.name) == named.as_ref());
    let missing = named.is_some() && !own_open && !app.talk_view.outside;
    let none = app.ws.is_some() && named.is_none();
    let mut language = None;
    let scroll =
        egui::ScrollArea::vertical().id_salt(("talk-file-panel", which)).auto_shrink(false);
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(MARGIN).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let t = mg_uikit::tokens(ui.ctx());
            let value = |ui: &mut Ui, text: String| {
                ui.add(egui::Label::new(egui::RichText::new(text).color(t.text_2)).wrap())
            };
            let shown = parts_of(app, which)
                .filter(|_| which == Which::Game || own_open || app.talk_view.outside);
            if let Some((tbl, v)) = shown {
                form_row(ui, "Lines", false, |ui| value(ui, tbl.len().to_string()));
                form_row(ui, "Language", false, |ui| {
                    let name = tbl.tlk.language.name().unwrap_or("?").to_string();
                    if v.languages.is_empty() || which == Which::Game {
                        value(ui, name);
                        return;
                    }
                    // The nasher project's other languages of it, a file
                    // each (Save or undo to switch).
                    let open =
                        tbl.source.file().map(std::path::Path::to_path_buf).unwrap_or_default();
                    let mut chosen = open.clone();
                    let paths: Vec<std::path::PathBuf> =
                        v.languages.iter().map(|(_, p)| p.clone()).collect();
                    let named = |p: &std::path::PathBuf| {
                        let l = v.languages.iter().find(|(_, q)| q == p).map(|(l, _)| *l);
                        l.and_then(mg_core::Language::name).unwrap_or("?").to_string()
                    };
                    ui.add_enabled_ui(!tbl.is_dirty(), |ui| {
                        crate::widgets::dropdown("talk-language")
                            .selected_text(named(&open))
                            .width(ui.available_width().min(200.0))
                            .show(ui, &mut chosen, &paths, named)
                            .on_hover_text(
                                "The project keeps this table in several languages. Save or undo \
                                 changes to switch.",
                            );
                    });
                    language = (chosen != open).then_some(chosen);
                });
                form_row(ui, "Feminine", false, |ui| match &tbl.feminine {
                    None => value(ui, "no feminine table".into()),
                    // (Its file's name: its folder is the table's.)
                    Some((s, _)) => {
                        let name = s.file().and_then(|f| f.file_name());
                        match name {
                            Some(n) => value(ui, n.to_string_lossy().into_owned()),
                            None => value(ui, format!("with a feminine table ({s})")),
                        }
                    }
                });
                let whole = match (which, &tbl.source) {
                    (Which::Game, _) => app.game_talk.as_ref().and_then(|g| g.path.clone()),
                    (_, s) => s.file().map(std::path::Path::to_path_buf),
                };
                let place = match &whole {
                    Some(p) => shorter(p),
                    None => tbl.source.to_string(),
                };
                form_row(ui, "File", true, |ui| {
                    let mono = egui::FontId::monospace(12.0);
                    // (Broken at its separators, never inside a folder's
                    // name: "…/Neverwinter Nights/lang/en/data/" over
                    // "dialog.tlk", as the board draws it.)
                    let shown = at_separators(ui, &place, &mono, ui.available_width());
                    let label =
                        egui::Label::new(egui::RichText::new(shown).font(mono).color(t.text_2));
                    let r = ui.add(label.extend());
                    match &whole {
                        Some(p) => r.on_hover_text(p.display().to_string()),
                        None => r,
                    }
                });
            } else if !(none || missing) {
                note(ui, "No talk table is open.");
            }
            if none || missing {
                heading(ui, crate::talk_view::MODULE_S_TABLE);
                module_s_table(app, ui, named.clone(), focus);
            }
        });
    });
    if let Some(path) = language {
        super::open_language(app, &path);
    }
}

/// A file's path as the panel shows it (the Talk board's): its last five
/// parts after "…" and a separator (the path's own: `\` on Windows)
/// where it has more; the whole in its tip.
fn shorter(path: &std::path::Path) -> String {
    crate::shown_path::last_parts(&path.display().to_string(), 5, crate::shown_path::WINDOWS)
}

/// Module's Table (M4's U17, the Talk Table panel's section; before it, the
/// tab's page): the module has no talk table, or one the game can't find:
/// make one.
fn module_s_table(app: &mut Moonglow, ui: &mut Ui, name: Option<String>, focus: bool) {
    let t = mg_uikit::tokens(ui.ctx());
    match &name {
        None => {
            note(
                ui,
                "This module has no talk table of its own. A talk table holds text that 2DAs, \
                 conversations and scripts name by StrRef: needed for new classes, feats, spells \
                 or items in haks.",
            );
        }
        Some(n) => {
            let color = ui.visuals().error_fg_color;
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!(
                        "The module names the talk table {n}, which isn't in its haks, the module \
                         or the tlk folder: the game won't load the module."
                    ))
                    .color(color),
                )
                .wrap(),
            );
            if talk::name_hint(n).is_none() && app.talk_view.new_name.is_empty() {
                app.talk_view.new_name = n.clone();
            }
        }
    }
    ui.add_space(6.0);
    // (A nasher project keeps its table in it, as nasher does: its file
    // where the rules put it, and a target of its own.)
    let project = app.ws.as_ref().and_then(|ws| ws.module.project.as_deref());
    let in_project = project.filter(|p| mg_module::nasher::kept_as(&p.settings).is_some());
    let user_dir = app.install.as_ref().and_then(|i| i.user_folder("tlk"));
    let Some(dir) = user_dir.or_else(|| in_project.map(|p| p.root().join("tlk"))) else {
        note(ui, "Making a talk table needs the game's user folder (Tools › Options).");
        return;
    };
    let file_of = |n: &str| match in_project {
        Some(p) => p.talk_table_file(n),
        None => Some(dir.join(format!("{n}.tlk"))),
    };
    // (A name by the rule of the names Moonglow makes, in lower case: the
    // file's on every system.)
    let typed = app.talk_view.new_name.trim();
    let valid = mg_core::ResRef::new_name(typed);
    let new = valid.as_ref().map_or_else(|_| typed.to_string(), |r| r.to_string_lossy());
    let made_in = file_of(&new);
    // (In the project, as its nasher.cfg names it: `src/mytlk.tlk.json`.)
    let shown = |f: &std::path::Path| match in_project {
        Some(p) => f.strip_prefix(p.root()).unwrap_or(f).display().to_string().replace('\\', "/"),
        None => f.display().to_string(),
    };
    let made_shown = made_in.as_deref().map(shown);
    let in_project = in_project.is_some();
    let v = &mut app.talk_view;
    form_row(ui, "New talk table", false, |ui| {
        let field = mg_uikit::text_field(&mut v.new_name)
            .desired_width(ui.available_width())
            .hint_text("name");
        let r = ui.add(field);
        r.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "New talk table")
        });
        // (F6 here: the name takes the keyboard.)
        if focus {
            r.request_focus();
        }
    });
    form_row(ui, "", false, |ui| {
        ui.add(egui::Checkbox::new(&mut v.new_feminine, "With a feminine table"))
            .on_hover_text("For languages whose text differs for female characters");
    });
    let exists_id = egui::Id::new("talk-new-file-exists");
    let made_exists = made_in.as_ref().is_some_and(|f| {
        let now = std::time::Instant::now();
        let ttl = std::time::Duration::from_secs(2);
        match ui.ctx().data(|d| {
            d.get_temp::<Option<(std::path::PathBuf, bool, std::time::Instant)>>(exists_id)
                .flatten()
        }) {
            Some((p, ex, at)) if p == *f && now.duration_since(at) < ttl => ex,
            _ => {
                let ex = f.exists();
                ui.ctx().data_mut(|d| d.insert_temp(exists_id, Some((f.clone(), ex, now))));
                ex
            }
        }
    });
    let problem = if new.is_empty() {
        Some(format!("Name it: {}.", mg_core::NewName::RULE))
    } else if let Err(e) = &valid {
        Some(format!("Not a name: {e}."))
    } else if made_in.is_none() {
        Some("The project's rules discard talk tables: nowhere to make one.".to_string())
    } else if made_exists {
        let f = made_shown.as_deref().unwrap_or_default();
        Some(format!("{f} is there already: choose it in Module Properties."))
    } else {
        None
    };
    let create = form_row(ui, "", false, |ui| {
        ui.add_enabled(problem.is_none(), egui::Button::new("Create")).clicked()
    });
    let said = match (&problem, &made_shown) {
        (Some(p), _) => Some(p.clone()),
        (None, Some(f)) if in_project => {
            Some(format!("Made in the project as {f}, with a target of its own"))
        }
        (None, Some(_)) => Some(format!("Made in {}", dir.display())),
        _ => None,
    };
    if let Some(said) = said {
        ui.add(egui::Label::new(egui::RichText::new(said).color(t.text_3)).wrap());
    }
    // (Looked for again as it is made: the file looked for above may be
    // two seconds old, and one that came since is not written over.)
    let landed = create && made_in.as_ref().is_some_and(|f| f.exists());
    if landed {
        let f = made_shown.as_deref().unwrap_or_default();
        app.log.error(format!("Talk table: {f} is there already: choose it in Module Properties"));
    }
    if create {
        ui.ctx().data_mut(|d| {
            d.remove_temp::<Option<(std::path::PathBuf, bool, std::time::Instant)>>(exists_id)
        });
    }
    if create && !landed {
        let language = app.game.as_deref().map_or(mg_core::Language::ENGLISH, |g| g.language);
        let feminine = app.talk_view.new_feminine;
        let made = match in_project {
            true => super::in_project_table(app, &new, language, feminine),
            false => {
                let mut t = Table::create(&new, &dir, language, feminine);
                t.save().map(|()| t)
            }
        };
        match made {
            Ok(t) => {
                app.log.info(format!("Made talk table {}", t.source));
                app.talk = Some(t);
                app.talk_view = TalkView::default();
                // (Shown in the tab, the game's dialog.tlk let go of.)
                super::hide_game(app);
                if name.as_deref() != Some(new.as_str()) {
                    use mg_schema::{ExoString, GffValue, ifo};
                    app.actions.push(Action::Apply(mg_edit::Command::new(
                        "Set the talk table",
                        vec![mg_edit::Edit::SetField {
                            key: crate::module_props::info_key(),
                            path: mg_edit::GffPath::root(),
                            label: ifo::MOD_CUSTOM_TLK.label.to_string(),
                            value: Some(ExoString(crate::text::encode(&new)).into_value()),
                        }],
                    )));
                } else {
                    app.reload_custom_tlk();
                }
                app.actions.push(Action::OpenTab(Tab::TalkTable));
            }
            Err(e) => app.log.error(format!("Talk table: {e}")),
        }
    }
}

/// What the Status Bar shows beside a talk table (M4's U17, the Talk
/// board's): its lines and, only looked at, so ("112228 lines,
/// read-only"); and the list's keys, fixed today: Up / Down Choose line,
/// Page Up / Page Down By ten, Enter To its text, Double Click To its text
/// (always), Esc Leave list: the keys while the list has them and nothing
/// is typed, none while a modal, a dialog or a menu is in front.
pub(crate) fn shown(app: &Moonglow, ctx: &egui::Context, which: Which) -> crate::shell::Shown {
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let Some((t, _)) = parts_of(app, which) else { return Shown::default() };
    let editable = super::editable(app, which);
    let lines = match editable {
        true => format!("{} lines", t.len()),
        false => format!("{} lines, read-only", t.len()),
    };
    let counts = vec![Count::new(lines)];
    let mut hints = Vec::new();
    // (A kit popover open, the header's More or Export: not egui's popups,
    // which `keys_blocked` sees.)
    if crate::shell::keys_blocked(ctx) || mg_uikit::popover::Popover::any_open(ctx) {
        return Shown { hints, counts, note: None };
    }
    let keys = super::list_has_keys(app, ctx, which) && !ctx.egui_wants_keyboard_input();
    if keys {
        hints.push(Hint::either(["Up", "Down"], "Choose line"));
        hints.push(Hint::either(["Page Up", "Page Down"], "By ten"));
        hints.push(Hint::keys(["Enter"], "To its text"));
    }
    hints.push(Hint::keys(["Double Click"], "To its text"));
    if keys {
        hints.push(Hint::keys(["Esc"], "Leave list"));
    }
    Shown { hints, counts, note: None }
}
