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

type MetricStamp = (u32, u32, u64);

fn metric_stamp(ui: &Ui, font: &egui::FontId) -> MetricStamp {
    (
        font.size.to_bits(),
        ui.ctx().pixels_per_point().to_bits(),
        crate::script_layout::fonts_made(ui.ctx()),
    )
}

#[derive(Clone)]
struct LabelWidthCache {
    stamp: MetricStamp,
    widths: std::collections::HashMap<String, f32>,
}

/// The width `label` takes in its column: the column's, or its words' where
/// they are wider.
fn label_width(ui: &Ui, label: &str) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let stamp = metric_stamp(ui, &font);
    let id = egui::Id::new("nui-form-label-widths");
    if let Some(w) = ui.ctx().data_mut(|d| {
        let cache = d.get_temp_mut_or_insert_with(id, || LabelWidthCache {
            stamp,
            widths: Default::default(),
        });
        if cache.stamp != stamp {
            cache.stamp = stamp;
            cache.widths.clear();
        }
        cache.widths.get(label).copied()
    }) {
        return w;
    }
    let words = ui.painter().layout_no_wrap(label.to_string(), font, egui::Color32::WHITE);
    let w = LABEL.max(words.size().x.ceil() + 8.0);
    ui.ctx().data_mut(|d| {
        let cache = d.get_temp_mut_or_insert_with(id, || LabelWidthCache {
            stamp,
            widths: Default::default(),
        });
        cache.widths.insert(label.to_string(), w);
    });
    w
}

/// Cached width of a segmented control whose labels and count stay constant for
/// a given font size, display scale, and font atlas generation.
pub(super) fn segmented_width(ui: &Ui, id: egui::Id, labels: &[&str]) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let stamp = metric_stamp(ui, &font);
    if let Some((was, w)) = ui.ctx().data(|d| d.get_temp::<(MetricStamp, f32)>(id))
        && was == stamp
    {
        return w;
    }
    let pad = 2.0 * mg_uikit::metrics::CONTROL_PADDING_X;
    let sum: f32 = ui.fonts_mut(|f| {
        labels
            .iter()
            .map(|w| {
                f.layout_no_wrap((*w).to_string(), font.clone(), egui::Color32::WHITE).size().x
                    + pad
            })
            .sum()
    });
    let w = sum.ceil() + labels.len().saturating_sub(1) as f32;
    ui.ctx().data_mut(|d| d.insert_temp(id, (stamp, w)));
    w
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
    let width = label_width(ui, words);
    let out = if ui.available_width() - width - ui.spacing().item_spacing.x >= least {
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

/// A bound property's chip (M4's U19, the NUI board's `.bind-chip`): the
/// link icon in `accent` and the bind's name in the code font, on a ground
/// of `accent` faint and its rim, 22 high; a button named "Bound to NAME",
/// cut short to the room it is given.
pub(super) fn bind_chip(ui: &mut Ui, name: &str) -> egui::Response {
    let t = mg_uikit::tokens(ui.ctx());
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let icon = mg_uikit::metrics::ICON;
    let pad = 8.0;
    let room = (ui.available_width() - 2.0 * pad - icon - 4.0).max(0.0);
    let mut job = egui::text::LayoutJob::simple_singleline(name.to_string(), font, t.text);
    job.wrap = egui::text::TextWrapping::truncate_at_width(room);
    let galley = ui.painter().layout_job(job);
    let size =
        egui::vec2(2.0 * pad + icon + 4.0 + galley.size().x, mg_uikit::metrics::CONTROL_HEIGHT);
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
    let said = format!("Bound to {name}");
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &said));
    if ui.is_rect_visible(rect) {
        let fill =
            if r.hovered() { t.accent.gamma_multiply(0.28) } else { t.accent.gamma_multiply(0.16) };
        ui.painter().rect_filled(rect, 3.0, fill);
        ui.painter().rect_stroke(rect, 3.0, (1.0, t.accent), egui::StrokeKind::Inside);
        let at = egui::Rect::from_center_size(
            egui::pos2(rect.left() + pad + icon / 2.0, rect.center().y),
            egui::vec2(icon, icon),
        );
        mg_uikit::Icon::Link.paint(ui.painter(), at, t.accent);
        let y = rect.center().y - galley.size().y / 2.0;
        ui.painter().galley(egui::pos2(at.right() + 4.0, y), galley, t.text);
    }
    r
}

/// Where a bound property's chip asks Property Binding to show its
/// property ([`bind_chip`]): the property's key, taken as Property Binding
/// is drawn next.
/// Keyed by the designer whose Properties draw it (`chip_owner`, set as
/// they begin): two designers drawn in one frame (one docked, one in its
/// window) never take each other's.
pub(super) fn chip_asked(ctx: &egui::Context) -> egui::Id {
    let owner = ctx.data(|d| d.get_temp::<ResKey>(chip_owner()));
    egui::Id::new(("nui-bind-chip-asked", owner))
}

/// Where the designer whose Properties are being drawn is noted.
pub(super) fn chip_owner() -> egui::Id {
    egui::Id::new("nui-bind-chip-owner")
}

/// A button with the kit's drawn plus before its words (Add Property,
/// Create Bind, Add Handler…), named by its words.
pub(super) fn plus(words: &str) -> mg_uikit::IconButton {
    mg_uikit::IconButton::new(mg_uikit::Icon::Plus, words).framed(true)
}
