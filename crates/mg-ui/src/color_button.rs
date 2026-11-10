//! A color button with egui's picker, and a color pasted into it: the
//! picker's own 📋 copies `158, 151, 211`, and nothing pasted one back
//! (a builder's report). Paste beside the picker, Ctrl+V while it is
//! open or the pointer is on the swatch, and the swatch's right-click
//! menu take a color from the clipboard: `158, 151, 211` (with a fourth
//! for alpha), `#9e97d3`, `rgb(158, 151, 211)`, or the picker's 0 to 1
//! values (`0.620, 0.592, 0.827`).

use egui::color_picker::{Alpha, color_picker_hsva_2d, show_color_at};
use egui::ecolor::Hsva;
use egui::{Color32, Popup, PopupCloseBehavior, Response, Ui};

/// A color read from text as a clipboard holds it, or `None`.
pub fn parse(text: &str) -> Option<Color32> {
    let t = text.trim();
    let hex = t.strip_prefix('#').unwrap_or(t);
    if let Some([r, g, b]) = mg_core::text::hex_bytes::<3>(hex) {
        return Some(Color32::from_rgb(r, g, b));
    }
    if let Some([r, g, b, a]) = mg_core::text::hex_bytes::<4>(hex) {
        return Some(Color32::from_rgba_unmultiplied(r, g, b, a));
    }
    let inner = t
        .strip_prefix("rgba(")
        .or_else(|| t.strip_prefix("rgb("))
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(t);
    let parts: Vec<&str> =
        inner.split([',', ' ', '\t', ';']).map(str::trim).filter(|s| !s.is_empty()).collect();
    if !matches!(parts.len(), 3 | 4) {
        return None;
    }
    let numbers: Vec<f64> = parts.iter().map(|s| s.parse().ok()).collect::<Option<_>>()?;
    // (The picker's other copy: values of 0 to 1, with their points.)
    let fractions = parts.iter().any(|s| s.contains('.')) && numbers.iter().all(|n| *n <= 1.0);
    let byte = |n: f64| {
        let v = if fractions { n * 255.0 } else { n };
        (0.0..=255.0).contains(&v.round()).then(|| v.round() as u8)
    };
    let a = match numbers.get(3) {
        Some(&n) => byte(n)?,
        None => 255,
    };
    Some(Color32::from_rgba_unmultiplied(
        byte(numbers[0])?,
        byte(numbers[1])?,
        byte(numbers[2])?,
        a,
    ))
}

/// `c` as #rrggbb (#rrggbbaa where the color has an alpha), in lower case.
fn hex_of(c: Color32, alpha: Alpha) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if alpha == Alpha::Opaque {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

/// The text the picker's 📋 copies for `c`.
fn text_of(c: Color32, alpha: Alpha) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    if alpha == Alpha::Opaque { format!("{r}, {g}, {b}") } else { format!("{r}, {g}, {b}, {a}") }
}

/// A color pasted this frame, taken from the frame's input.
fn take_paste(ui: &Ui) -> Option<Color32> {
    ui.input_mut(|i| {
        let mut found = None;
        i.events.retain(|e| match e {
            egui::Event::Paste(text) if found.is_none() => match parse(text) {
                Some(c) => {
                    found = Some(c);
                    false
                }
                None => true,
            },
            _ => true,
        });
        found
    })
}

/// A button showing `srgba`; a click opens egui's picker with Paste
/// beside it. Its popup has egui's own id (`ui.auto_id_with("popup")`),
/// so a caller that watches the picker open sees it as before. The
/// response is changed when the color is.
pub fn edit_srgba(ui: &mut Ui, srgba: &mut Color32, alpha: Alpha) -> Response {
    let popup_id = ui.auto_id_with("popup");
    let hsva_id = popup_id.with("hsva");
    let open = Popup::is_id_open(ui.ctx(), popup_id);
    // (A gray kept with the hue it was dragged from, as egui's own
    // button keeps it, while the picker is open.)
    let mut hsva = ui
        .data(|d| d.get_temp::<Hsva>(hsva_id))
        .filter(|h| Color32::from(*h) == *srgba)
        .unwrap_or_else(|| Hsva::from(*srgba));
    let mut button = show_color_button(ui, *srgba, open);
    if ui.style().explanation_tooltips {
        button = button.on_hover_text("Click to edit the color. Ctrl+V pastes one.");
    }
    let mut pasted = if open || button.hovered() { take_paste(ui) } else { None };
    // (Its own id: egui's for a context menu is the picker's, which kept
    // the menu from opening.)
    Popup::context_menu(&button).id(popup_id.with("menu")).show(|ui| {
        if ui.button("Copy Color").on_hover_text("Its values, as 158, 151, 211").clicked() {
            ui.copy_text(text_of(*srgba, alpha));
            ui.close();
        }
        if ui
            .button("Paste Color")
            .on_hover_text("From the clipboard: 158, 151, 211, #9e97d3 or rgb(158, 151, 211)")
            .clicked()
        {
            crate::widgets::ask_paste(ui.ctx(), popup_id.with("asked"));
            ui.close();
        }
    });
    // (Paste asked for from the menu: the clipboard comes as the next
    // frame's paste event, wherever the pointer is then; text that isn't
    // a color changes nothing.)
    if pasted.is_none()
        && let Some(text) = crate::widgets::asked_paste(ui.ctx(), popup_id.with("asked"))
    {
        pasted = parse(&text);
    }
    const COLOR_SLIDER_WIDTH: f32 = 275.0;
    Popup::menu(&button).id(popup_id).close_behavior(PopupCloseBehavior::CloseOnClickOutside).show(
        |ui| {
            ui.spacing_mut().slider_width = COLOR_SLIDER_WIDTH;
            if color_picker_hsva_2d(ui, &mut hsva, alpha) {
                button.mark_changed();
            }
            ui.horizontal(|ui| {
                // The color as #rrggbb, to read, type or paste into (an
                // owner's ask): it follows the picker, and a color typed
                // there sets it.
                let hex_id = popup_id.with("hex");
                let shown = hex_of(Color32::from(hsva), alpha);
                let mut text: String = ui.data(|d| d.get_temp(hex_id)).unwrap_or(shown);
                let field = ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .desired_width(if alpha == Alpha::Opaque { 70.0 } else { 90.0 })
                            .font(egui::TextStyle::Monospace),
                    )
                    .on_hover_text("The color as #rrggbb: type or paste one here");
                if field.changed()
                    && let Some(mut c) = parse(&text)
                {
                    if alpha == Alpha::Opaque {
                        c = Color32::from_rgb(c.r(), c.g(), c.b());
                    }
                    hsva = Hsva::from(c);
                    button.mark_changed();
                }
                if field.has_focus() {
                    ui.data_mut(|d| d.insert_temp(hex_id, text));
                } else {
                    ui.data_mut(|d| d.remove::<String>(hex_id));
                }
                if ui
                    .button("Paste Color")
                    .on_hover_text(
                        "Paste a color copied as 158, 151, 211, #9e97d3 or rgb(158, 151, 211)",
                    )
                    .clicked()
                {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    ui.ctx().request_repaint();
                }
            });
        },
    );
    if let Some(mut c) = pasted {
        if alpha == Alpha::Opaque {
            c = Color32::from_rgb(c.r(), c.g(), c.b());
        }
        hsva = Hsva::from(c);
        button.mark_changed();
    }
    *srgba = Color32::from(hsva);
    if Popup::is_id_open(ui.ctx(), popup_id) {
        ui.data_mut(|d| d.insert_temp(hsva_id, hsva));
    } else {
        ui.data_mut(|d| d.remove::<Hsva>(hsva_id));
    }
    button
}

/// [`edit_srgba`] of an opaque sRGB color.
pub fn edit_srgb(ui: &mut Ui, srgb: &mut [u8; 3]) -> Response {
    let mut c = Color32::from_rgb(srgb[0], srgb[1], srgb[2]);
    let r = edit_srgba(ui, &mut c, Alpha::Opaque);
    *srgb = [c.r(), c.g(), c.b()];
    r
}

/// [`edit_srgba`] of an sRGBA color with its alpha not multiplied in.
pub fn edit_srgba_unmultiplied(ui: &mut Ui, srgba: &mut [u8; 4]) -> Response {
    let mut c = Color32::from_rgba_unmultiplied(srgba[0], srgba[1], srgba[2], srgba[3]);
    let r = edit_srgba(ui, &mut c, Alpha::OnlyBlend);
    *srgba = c.to_srgba_unmultiplied();
    r
}

/// [`edit_srgba`] of an opaque color of 0 to 1 values in sRGB.
pub fn edit_rgb(ui: &mut Ui, rgb: &mut [f32; 3]) -> Response {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut c = Color32::from_rgb(byte(rgb[0]), byte(rgb[1]), byte(rgb[2]));
    let before = c;
    let r = edit_srgba(ui, &mut c, Alpha::Opaque);
    if c != before {
        *rgb = [c.r(), c.g(), c.b()].map(|b| f32::from(b) / 255.0);
    }
    r
}

/// The swatch button, as egui draws it.
fn show_color_button(ui: &mut Ui, color: Color32, open: bool) -> Response {
    let size = ui.spacing().interact_size;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::ColorButton));
    if ui.is_rect_visible(rect) {
        let visuals =
            if open { ui.visuals().widgets.open } else { *ui.style().interact(&response) };
        let rect = rect.expand(visuals.expansion);
        let stroke_width = 1.0;
        show_color_at(ui.painter(), color, rect.shrink(stroke_width));
        let corner = visuals.corner_radius.at_most(2);
        ui.painter().rect_stroke(
            rect,
            corner,
            (stroke_width, visuals.bg_fill),
            egui::StrokeKind::Inside,
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the picker's 📋 copies, and the other ways a color is written,
    /// read back; other text isn't a color.
    #[test]
    fn colors_are_read_as_clipboards_hold_them() {
        let c = Color32::from_rgb(158, 151, 211);
        for text in ["158, 151, 211", " 158,151,211 ", "#9e97d3", "9E97D3", "rgb(158, 151, 211)"] {
            assert_eq!(parse(text), Some(c), "{text:?}");
        }
        assert_eq!(parse(&text_of(c, Alpha::Opaque)), Some(c));
        assert_eq!(parse("0.620, 0.592, 0.827"), Some(c));
        assert_eq!(
            parse("158, 151, 211, 128"),
            Some(Color32::from_rgba_unmultiplied(158, 151, 211, 128))
        );
        assert_eq!(parse("1, 1, 1"), Some(Color32::from_rgb(1, 1, 1)), "whole numbers are bytes");
        for text in ["", "hello", "300, 0, 0", "1, 2", "#12345", "1, 2, 3, 4, 5"] {
            assert_eq!(parse(text), None, "{text:?}");
        }
    }
}
