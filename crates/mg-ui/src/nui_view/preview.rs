//! The canvas: the window drawn with the game's own skin, pictures and
//! fonts, laid out here as the client was measured to. This file holds how a
//! property's value is read (a bind's value in its place: [`val`], [`num`],
//! [`flag`], [`string`]); the submodules hold the layout, the stage and each
//! kind of control.
use super::*;
use egui::{Align2, Color32, Painter, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use std::collections::BTreeSet;

const GOLD: Color32 = Color32::from_rgb(173, 142, 96);

mod buttons;
mod chart;
mod color_picker;
mod combo;
mod containers;
mod layout;
mod node;
mod picture;
mod sliders;
mod stage;
mod text_controls;
mod typeset;

pub(super) use layout::window_size;
use layout::*;
use node::{Control, Renderer};
#[cfg(test)]
use picture::image_rect;
pub(super) use picture::{Props, draw_image};
pub(super) use stage::{canvas, still};
use typeset::*;
pub(super) use typeset::{TextStyle, text};

pub(super) fn val<'a>(v: &'a Value, s: &'a Settings, row: Option<usize>) -> &'a Value {
    let out = resolved(v, s);
    if v.get("bind").is_some()
        && let (Some(row), Some(a)) = (row, out.as_array())
    {
        return a.get(row).unwrap_or(&Value::Null);
    }
    out
}
/// A number of the window, within ±`HUGE`.
pub(super) fn num(v: &Value, s: &Settings, row: Option<usize>, default: f32) -> f32 {
    val(v, s, row)
        .as_f64()
        .filter(|n| n.is_finite())
        .map_or(default, |n| n.clamp(-HUGE, HUGE) as f32)
}

/// Past any size or value a window has, and short enough of what an `f32`
/// holds that sums and products of two stay finite: a number written larger
/// (1e308) would be infinite as an `f32`, and a rectangle made of it not a
/// number at all.
const HUGE: f64 = 1e18;
pub(super) fn flag(v: &Value, s: &Settings, row: Option<usize>, default: bool) -> bool {
    val(v, s, row).as_bool().unwrap_or(default)
}
pub(super) fn string(v: &Value, s: &Settings, row: Option<usize>) -> String {
    format_string(v, s, row, None)
}
pub(super) fn localized_string(
    v: &Value,
    s: &Settings,
    row: Option<usize>,
    assets: &skin::Assets,
) -> String {
    format_string(v, s, row, Some(assets))
}
fn format_string(
    v: &Value,
    s: &Settings,
    row: Option<usize>,
    assets: Option<&skin::Assets>,
) -> String {
    let resolved = val(v, s, row);
    let mut text = match resolved {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => match v.get("strref").and_then(Value::as_i64) {
            Some(id) => assets.map_or_else(|| format!("[Missing StrRef {id}]"), |a| a.string(id)),
            None => v.to_string(),
        },
    };
    if v["bind"].is_string() {
        if let Some(n) = resolved.as_i64() {
            if v["number_flags"].as_i64().unwrap_or(0) & 1 != 0 {
                text = format!("{n:x}");
            }
        } else if let Some(n) = resolved.as_f64() {
            let precision =
                v["number_precision"].as_u64().unwrap_or(0).min(MOST_PLACES as u64) as usize;
            text = native_decimal(n, precision);
        }
        let flags = v["text_flags"].as_i64().unwrap_or(0);
        // Native bind text_flags operate on ASCII case, preserving UTF-8
        // characters (Creator-authored nui_format_s: Żółć stays unchanged).
        if flags & 1 != 0 {
            text.make_ascii_lowercase();
        }
        if flags & 2 != 0 {
            text.make_ascii_uppercase();
        }
    }
    text
}

/// The most decimal places a number's text is written to: past every
/// digit a 32-bit float has (149), where the client writes zeros.
const MOST_PLACES: usize = 400;

/// A number in text as the client writes it: the bind's value as a 32-bit
/// float, to `precision` places with no cap of its own, ties to even as C's
/// `printf` (NWN EE 8193.37, `nui_readings_in_the_client`: 1/3 at 17 places
/// is 0.33333334326744080, 0.5 and 2.5 at none 0 and 2).
fn native_decimal(n: f64, precision: usize) -> String {
    // (Rust writes a float's exact value, ties to even, as printf does.)
    let n = f64::from(n as f32);
    let precision = precision.min(MOST_PLACES);
    format!("{n:.precision$}")
}

pub(super) fn rgba(v: &Value, default: Color32) -> Color32 {
    if !v.is_object() {
        return default;
    }
    let [r, g, b, a] = rgba_channels(v);
    Color32::from_rgba_unmultiplied(r, g, b, a)
}
fn rgba_channels(v: &Value) -> [u8; 4] {
    ["r", "g", "b", "a"].map(|k| v[k].as_f64().map_or(255, |x| x.clamp(0.0, 255.0) as u8))
}
fn alignment(h: i64, v: i64) -> Align2 {
    use egui::Align;
    let a = |v| match v {
        1 => Align::Min,
        2 => Align::Max,
        _ => Align::Center,
    };
    Align2([a(h), a(v)])
}

#[cfg(test)]
mod tests;
