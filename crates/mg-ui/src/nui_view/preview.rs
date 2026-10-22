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
            let precision = v["number_precision"].as_u64().unwrap_or(0).min(16) as usize;
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

fn native_decimal(mut n: f64, precision: usize) -> String {
    // NWN rounds exact decimal half ties away from zero. Rust uses ties-to-even.
    // Detect the tie in the exact binary value, not with rounded n * 10^p:
    // that multiplication can incorrectly turn a neighbouring value into a tie.
    let bits = n.abs().to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if n.is_finite() && exponent != 0 {
        let mantissa = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
        let numerator = u128::from(mantissa) * 5_u128.pow(precision as u32);
        let denominator_bits = 1075 - exponent - precision as i32;
        if (1..128).contains(&denominator_bits) {
            let denominator = 1_u128 << denominator_bits;
            if numerator % denominator == denominator / 2 {
                n = if n.is_sign_negative() { n.next_down() } else { n.next_up() };
            }
        }
    }
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
