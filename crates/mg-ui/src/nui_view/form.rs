//! The designer's panels' form, as the NUI board draws Properties
//! (`Nui_right.png`, COMPONENTS.md's `.form`): a bold section heading over
//! rows of a label in its column and its field beside it; the label over
//! its field only where the row has not the room for both (the room tested
//! is the room given: a row never taller where wider).
use super::*;

/// The label column (the board's `--lbl-w`, 104).
pub(super) const LABEL: f32 = 104.0;
/// The least a field keeps beside its label: under it, the label stands
/// over the field.
pub(super) const FIELD_LEAST: f32 = 120.0;
/// The room between two rows (the board's `row-gap`).
pub(super) const ROW_GAP: f32 = 6.0;

/// The width `label` takes in its column: the column's, or its words' where
/// they are wider.
fn label_width(ui: &Ui, label: &str) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let words = ui.painter().layout_no_wrap(label.to_string(), font, egui::Color32::WHITE);
    LABEL.max(words.size().x.ceil() + 8.0)
}

/// Whether a row of `label` has the room for its field beside it, `least`
/// the room the field asks.
pub(super) fn beside(ui: &Ui, label: &str, least: f32) -> bool {
    ui.available_width() - label_width(ui, label) - ui.spacing().item_spacing.x >= least
}

/// A label of the form (`text-2`), cut short where it has not the room, its
/// `tip` under the pointer where it has one.
pub(super) fn label(ui: &mut Ui, words: &str, tip: Option<&str>) -> egui::Response {
    let t = mg_uikit::tokens(ui.ctx());
    let text = egui::RichText::new(words).color(t.text_2);
    let r = ui.add(egui::Label::new(text).truncate());
    match tip {
        Some(tip) => r.on_hover_text(tip),
        None => r,
    }
}

/// A row of the form: `label` in its column and what `add` draws beside it
/// (given the label's response, to name the field by), or the label over it
/// where the room beside it is less than [`FIELD_LEAST`].
pub(super) fn row<R>(
    ui: &mut Ui,
    words: &str,
    tip: Option<&str>,
    add: impl FnOnce(&mut Ui, &egui::Response) -> R,
) -> R {
    row_least(ui, words, tip, FIELD_LEAST, add)
}

/// [`row`] whose field asks `least` beside its label.
pub(super) fn row_least<R>(
    ui: &mut Ui,
    words: &str,
    tip: Option<&str>,
    least: f32,
    add: impl FnOnce(&mut Ui, &egui::Response) -> R,
) -> R {
    let out = if beside(ui, words, least) {
        let width = label_width(ui, words);
        ui.horizontal(|ui| {
            let size = egui::vec2(width, mg_uikit::metrics::CONTROL_HEIGHT);
            let left = egui::Layout::left_to_right(egui::Align::Center);
            let lead = ui
                .allocate_ui_with_layout(size, left, |ui| {
                    ui.set_min_size(size);
                    label(ui, words, tip)
                })
                .inner;
            add(ui, &lead)
        })
        .inner
    } else {
        let lead = label(ui, words, tip);
        add(ui, &lead)
    };
    ui.add_space(ROW_GAP);
    out
}

/// A section of the form: its heading in bold (`.sec-h`), a click on which
/// folds and unfolds what is under it, as the sections did with their
/// arrows (a chevron after it says so); `open` forces it open (a property
/// just added in it is shown). Its heading's response.
pub(super) fn section(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    default_open: bool,
    open: Option<bool>,
    tip: Option<&str>,
    body: impl FnOnce(&mut Ui),
) -> egui::Response {
    let id = ui.make_persistent_id(id_salt);
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        id,
        default_open,
    );
    if let Some(open) = open {
        state.set_open(open);
    }
    let t = mg_uikit::tokens(ui.ctx());
    ui.add_space(8.0);
    let size = egui::TextStyle::Body.resolve(ui.style()).size + 1.0;
    let mut text = egui::RichText::new(title).size(size).color(t.text);
    // (Until the kit's fonts are in, the first frame, the size alone.)
    let bold = mg_uikit::fonts::bold();
    if ui.fonts(|f| f.families().contains(&bold)) {
        text = text.family(bold);
    }
    let header = ui
        .horizontal(|ui| {
            let r = ui.add(egui::Label::new(text).truncate().sense(egui::Sense::click()));
            let opened = state.is_open();
            r.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, true, title)
            });
            // (Its fold, said by a small chevron after its words.)
            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
            let icon =
                if opened { mg_uikit::Icon::ChevronDown } else { mg_uikit::Icon::ChevronRight };
            icon.paint(ui.painter(), rect, t.text_3);
            r
        })
        .inner;
    let header = match tip {
        Some(tip) => header.on_hover_text(tip),
        None => header,
    };
    if header.clicked() {
        state.toggle(ui);
    }
    ui.add_space(4.0);
    state.show_body_unindented(ui, body);
    header
}

/// A button with the kit's drawn plus before its words (Add Property,
/// Create Bind, Add Handler…), named by its words.
pub(super) fn plus(words: &str) -> mg_uikit::IconButton {
    mg_uikit::IconButton::new(mg_uikit::Icon::Plus, words).framed(true)
}
