//! GFF as JSON, in the format of neverwinter.nim's `nwn_gff` (the nwn-lib
//! format nasher projects keep in version control).
//!
//! Each field is `{"type": "<kind>", "value": ...}`; structs carry
//! `"__struct_id"`, the root carries `"__data_type"`, voids are base64 in
//! `"value64"` and localized strings are `{"<key>": "text", "id": strref}`.
//! Strings are converted between the game codepage and UTF-8.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use mg_core::{Codepage, LocString, LocStringKey, StrRef};
use serde_json::{Map, Value as Json, json};
use thiserror::Error;

use crate::value::{Field, FieldType, Label, Struct, Value};
use crate::{Gff, ROOT_STRUCT_ID};

/// JSON that does not describe a GFF, or a GFF that JSON cannot describe.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JsonError {
    #[error("{path}: {message}")]
    Invalid { path: String, message: String },
}

fn err<T>(path: &str, message: impl Into<String>) -> Result<T, JsonError> {
    Err(JsonError::Invalid { path: path.to_string(), message: message.into() })
}

/// Converts a GFF to nwn-lib JSON.
///
/// Fails if a struct has two fields with the same label (JSON objects cannot
/// hold both) or a float is NaN or infinite.
pub fn to_json(gff: &Gff, codepage: Codepage) -> Result<Json, JsonError> {
    let mut obj = struct_to_json(&gff.root, codepage, "")?;
    obj.insert(
        "__data_type".into(),
        Json::String(String::from_utf8_lossy(&gff.file_type).into_owned()),
    );
    // Put the data type first, as nwn_gff does.
    let mut out = Map::new();
    out.insert("__data_type".into(), obj.remove("__data_type").unwrap());
    out.extend(obj);
    Ok(Json::Object(out))
}

fn struct_to_json(s: &Struct, cp: Codepage, path: &str) -> Result<Map<String, Json>, JsonError> {
    let mut obj = Map::new();
    if s.id != ROOT_STRUCT_ID {
        // nwn_gff keeps struct ids as signed 32-bit numbers.
        obj.insert("__struct_id".into(), json!(s.id as i32));
    }
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        let fpath = format!("{path}/{label}");
        if obj.contains_key(&label) {
            return err(&fpath, "duplicate label cannot be represented in JSON");
        }
        let mut field = Map::new();
        field.insert("type".into(), json!(f.value.field_type().json_name()));
        let float = |v: f64| {
            if v.is_finite() { Ok(json!(v)) } else { err(&fpath, "NaN or infinite float") }
        };
        match &f.value {
            Value::Byte(v) => _ = field.insert("value".into(), json!(v)),
            Value::Char(v) => _ = field.insert("value".into(), json!(v)),
            Value::Word(v) => _ = field.insert("value".into(), json!(v)),
            Value::Short(v) => _ = field.insert("value".into(), json!(v)),
            Value::Dword(v) => _ = field.insert("value".into(), json!(v)),
            Value::Int(v) => _ = field.insert("value".into(), json!(v)),
            Value::Dword64(v) => _ = field.insert("value".into(), json!(v)),
            Value::Int64(v) => _ = field.insert("value".into(), json!(v)),
            Value::Float(v) => _ = field.insert("value".into(), float(*v as f64)?),
            Value::Double(v) => _ = field.insert("value".into(), float(*v)?),
            Value::String(v) => _ = field.insert("value".into(), json!(cp.decode(v))),
            Value::ResRef(v) => _ = field.insert("value".into(), json!(cp.decode(v))),
            Value::LocString(ls) => {
                let mut entries = Map::new();
                for (key, text) in &ls.strings {
                    entries.insert(key.0.to_string(), json!(cp.decode(text)));
                }
                if !ls.strref.is_none() {
                    entries.insert("id".into(), json!(ls.strref.0));
                }
                field.insert("value".into(), Json::Object(entries));
            }
            Value::Void(v) => _ = field.insert("value64".into(), json!(BASE64.encode(v))),
            Value::Struct(child) => {
                field.insert("__struct_id".into(), json!(child.id as i32));
                field.insert("value".into(), Json::Object(struct_to_json(child, cp, &fpath)?));
            }
            Value::List(items) => {
                let list = items
                    .iter()
                    .enumerate()
                    .map(|(i, s)| struct_to_json(s, cp, &format!("{fpath}[{i}]")).map(Json::Object))
                    .collect::<Result<Vec<_>, _>>()?;
                field.insert("value".into(), Json::Array(list));
            }
        }
        obj.insert(label, Json::Object(field));
    }
    Ok(obj)
}

/// Reads nwn-lib JSON into a GFF.
pub fn from_json(json: &Json, codepage: Codepage) -> Result<Gff, JsonError> {
    let Json::Object(obj) = json else { return err("", "expected an object") };
    let Some(Json::String(data_type)) = obj.get("__data_type") else {
        return err("", "missing \"__data_type\"");
    };
    let file_type: [u8; 4] = match data_type.as_bytes().try_into() {
        Ok(t) => t,
        Err(_) => return err("/__data_type", "must be exactly 4 characters"),
    };
    let mut root = struct_from_json(obj, codepage, "")?;
    root.id = ROOT_STRUCT_ID;
    Ok(Gff { file_type, version: Gff::VERSION, root })
}

fn struct_from_json(
    obj: &Map<String, Json>,
    cp: Codepage,
    path: &str,
) -> Result<Struct, JsonError> {
    let id = match obj.get("__struct_id") {
        None => ROOT_STRUCT_ID,
        // Signed, as nwn_gff writes them, or unsigned.
        Some(v) => match as_int(v, path)? {
            id @ -0x8000_0000..0 => id as i32 as u32,
            id => id.try_into().or_else(|_| err(path, "__struct_id out of range"))?,
        },
    };
    let mut s = Struct::new(id);
    for (label, field) in obj {
        if label.starts_with("__") {
            continue;
        }
        let fpath = format!("{path}/{label}");
        let Json::Object(field) = field else { return err(&fpath, "field must be an object") };
        let Some(Json::String(ty)) = field.get("type") else {
            return err(&fpath, "missing \"type\"");
        };
        let Some(ty) = FieldType::from_json_name(ty) else {
            return err(&fpath, format!("unknown field type {ty:?}"));
        };
        let value = field.get("value");
        let need = || {
            value.ok_or_else(|| JsonError::Invalid {
                path: fpath.clone(),
                message: "missing \"value\"".into(),
            })
        };
        let int = |min: i128, max: i128| -> Result<i128, JsonError> {
            let v = as_int(need()?, &fpath)?;
            if v < min || v > max { err(&fpath, format!("{v} out of range")) } else { Ok(v) }
        };
        let text = || -> Result<Vec<u8>, JsonError> {
            let Json::String(s) = need()? else { return err(&fpath, "expected a string") };
            match cp.encode(s) {
                Some(b) => Ok(b.into_owned()),
                None => err(&fpath, "text has characters the codepage cannot represent"),
            }
        };
        let v = match ty {
            FieldType::Byte => Value::Byte(int(0, u8::MAX as i128)? as u8),
            FieldType::Char => Value::Char(int(i8::MIN as i128, i8::MAX as i128)? as i8),
            FieldType::Word => Value::Word(int(0, u16::MAX as i128)? as u16),
            FieldType::Short => Value::Short(int(i16::MIN as i128, i16::MAX as i128)? as i16),
            FieldType::Dword => Value::Dword(int(0, u32::MAX as i128)? as u32),
            FieldType::Int => Value::Int(int(i32::MIN as i128, i32::MAX as i128)? as i32),
            FieldType::Dword64 => Value::Dword64(int(0, u64::MAX as i128)? as u64),
            FieldType::Int64 => Value::Int64(int(i64::MIN as i128, i64::MAX as i128)? as i64),
            FieldType::Float => Value::Float(as_float(need()?, &fpath)? as f32),
            FieldType::Double => Value::Double(as_float(need()?, &fpath)?),
            FieldType::String => Value::String(text()?),
            FieldType::ResRef => Value::ResRef(text()?),
            FieldType::Void => match (field.get("value64"), value) {
                (Some(Json::String(b64)), _) => match BASE64.decode(b64) {
                    Ok(b) => Value::Void(b),
                    Err(e) => return err(&fpath, format!("bad base64: {e}")),
                },
                (_, Some(Json::String(raw))) => Value::Void(raw.as_bytes().to_vec()),
                _ => return err(&fpath, "void needs \"value64\""),
            },
            FieldType::LocString => {
                let Json::Object(entries) = need()? else {
                    return err(&fpath, "expected an object");
                };
                // The strref used to live next to "value"; nwn_gff still reads it there.
                let mut ls = LocString::from_strref(match field.get("id") {
                    Some(id) => StrRef(as_int(id, &fpath)? as u32),
                    None => StrRef::NONE,
                });
                for (k, v) in entries {
                    if k == "id" {
                        ls.strref = StrRef(as_int(v, &fpath)? as u32);
                        continue;
                    }
                    let Ok(key) = k.parse::<u32>() else {
                        return err(&fpath, format!("bad localized string key {k:?}"));
                    };
                    let Json::String(s) = v else { return err(&fpath, "expected a string") };
                    let Some(bytes) = cp.encode(s) else {
                        return err(&fpath, "text has characters the codepage cannot represent");
                    };
                    ls.strings.push((LocStringKey(key), bytes.into_owned()));
                }
                Value::LocString(ls)
            }
            FieldType::Struct => {
                let Json::Object(o) = need()? else { return err(&fpath, "expected an object") };
                let mut child = struct_from_json(o, cp, &fpath)?;
                if !o.contains_key("__struct_id") {
                    child.id = match field.get("__struct_id") {
                        Some(v) => as_int(v, &fpath)? as i64 as u32,
                        None => 0,
                    };
                }
                Value::Struct(child)
            }
            FieldType::List => {
                let Json::Array(items) = need()? else { return err(&fpath, "expected an array") };
                let list = items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let p = format!("{fpath}[{i}]");
                        match item {
                            Json::Object(o) => struct_from_json(o, cp, &p),
                            _ => err(&p, "list items must be objects"),
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Value::List(list)
            }
        };
        let Ok(label) = Label::new(label) else { return err(&fpath, "label longer than 16 bytes") };
        s.fields.push(Field { label, value: v });
    }
    Ok(s)
}

/// How [`to_json_text`] lays JSON out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextStyle {
    /// Sorts object keys, ignoring ASCII case and keeping the order of keys
    /// that differ only in case, as nasher does (`nwn_gff` keeps the file's
    /// field order).
    pub sort_keys: bool,
    /// Rounds floats to this many decimal places, as nasher's
    /// `truncateFloats` does (default 4); under a `Bearing` or `Orientation`
    /// key, -π rounded becomes +π.
    pub float_places: Option<u8>,
    /// Prints floats with 16 significant digits (C's `%.16g`, then `.0` if
    /// nothing marks the number as a float), as nasher's build of Nim does,
    /// rather than with the shortest digits that read back exactly, as
    /// `nwn_gff` does.
    pub g16_floats: bool,
}

impl TextStyle {
    /// nasher's default: sorted keys, floats to 4 places, printed `%.16g`.
    pub const NASHER: TextStyle =
        TextStyle { sort_keys: true, float_places: Some(4), g16_floats: true };
}

/// JSON as text, laid out as neverwinter.nim (Nim's `pretty`) writes it: two
/// spaces of indent, `"key": value`, Nim's string escapes and float
/// notation, and a final newline. With [`TextStyle::NASHER`] the text is what
/// nasher writes into a source tree.
pub fn to_json_text(json: &Json, style: TextStyle) -> String {
    let mut out = String::new();
    write_pretty(&mut out, json, style, 0, false);
    out.push('\n');
    out
}

fn write_pretty(out: &mut String, v: &Json, style: TextStyle, indent: usize, bearing: bool) {
    let pad = |out: &mut String, n: usize| out.extend(std::iter::repeat_n(' ', n));
    match v {
        Json::Object(map) if !map.is_empty() => {
            let mut entries: Vec<(&String, &Json)> = map.iter().collect();
            if style.sort_keys {
                // Stable, so keys equal but for case keep their order.
                entries.sort_by(|a, b| {
                    a.0.bytes()
                        .map(|c| c.to_ascii_lowercase())
                        .cmp(b.0.bytes().map(|c| c.to_ascii_lowercase()))
                });
            }
            out.push_str("{\n");
            for (i, (k, child)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                pad(out, indent + 2);
                escape_nim(out, k);
                out.push_str(": ");
                let bearing = bearing || k == "Bearing" || k == "Orientation";
                write_pretty(out, child, style, indent + 2, bearing);
            }
            out.push('\n');
            pad(out, indent);
            out.push('}');
        }
        Json::Object(_) => out.push_str("{}"),
        Json::Array(items) if !items.is_empty() => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                pad(out, indent + 2);
                write_pretty(out, item, style, indent + 2, bearing);
            }
            out.push('\n');
            pad(out, indent);
            out.push(']');
        }
        Json::Array(_) => out.push_str("[]"),
        Json::String(s) => escape_nim(out, s),
        Json::Number(n) => match (n.as_i64(), n.as_u64()) {
            (Some(i), _) if !n.is_f64() => out.push_str(&i.to_string()),
            (_, Some(u)) if !n.is_f64() => out.push_str(&u.to_string()),
            _ => {
                let f = n.as_f64().unwrap_or_default();
                let f = match style.float_places {
                    Some(places) => truncate_float(f, places, bearing),
                    None => f,
                };
                out.push_str(&if style.g16_floats { g16_float(f) } else { nim_float(f) });
            }
        },
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Null => out.push_str("null"),
    }
}

/// nasher's `truncateFloats`: the float printed with `places` decimals,
/// trailing zeros trimmed, read back; -π becomes π under a bearing.
fn truncate_float(f: f64, places: u8, bearing: bool) -> f64 {
    let places = usize::from(places);
    let s = format!("{f:.places$}");
    let mut trimmed = s.as_str();
    if trimmed.contains('.') {
        trimmed = trimmed.trim_end_matches('0');
    }
    let v: f64 = trimmed.trim_end_matches('.').parse().unwrap_or(f);
    if bearing && trimmed == format!("{:.places$}", -std::f64::consts::PI) { v.abs() } else { v }
}

/// A float as Nim's `addFloat` writes it: the shortest digits that read back
/// to it, fixed when the decimal point falls within -6 to 17 places of them
/// (with `.0` for whole numbers), else scientific (`1.5e+17`, `1e-8`).
fn nim_float(f: f64) -> String {
    if !f.is_finite() {
        return if f.is_nan() {
            "nan".into()
        } else if f > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    let sci = format!("{:e}", f.abs()); // shortest digits: "1.2345e-9"
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let exp: i32 = exp.parse().unwrap_or(0);
    // The decimal point's position after the first digit's.
    let point = exp + 1;
    let mut out = String::new();
    if f.is_sign_negative() {
        out.push('-');
    }
    let n = digits.len() as i32;
    if f == 0.0 {
        out.push_str("0.0");
    } else if (-6..=17).contains(&point) {
        if point <= 0 {
            out.push_str("0.");
            out.extend(std::iter::repeat_n('0', (-point) as usize));
            out.push_str(&digits);
        } else if point < n {
            out.push_str(&digits[..point as usize]);
            out.push('.');
            out.push_str(&digits[point as usize..]);
        } else {
            out.push_str(&digits);
            out.extend(std::iter::repeat_n('0', (point - n) as usize));
            out.push_str(".0");
        }
    } else {
        out.push_str(&digits[..1]);
        if n > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if exp < 0 { '-' } else { '+' });
        out.push_str(&exp.abs().to_string());
    }
    out
}

/// A float as Nim 1 wrote it: C's `%.16g` (16 significant digits, trailing
/// zeros dropped, scientific below 1e-4 and from 1e16, with a two-digit
/// exponent at least), then `.0` if that left neither a point nor an
/// exponent.
fn g16_float(f: f64) -> String {
    if !f.is_finite() {
        return if f.is_nan() {
            "nan".into()
        } else if f > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    let mut out = String::new();
    if f.is_sign_negative() {
        out.push('-');
    }
    if f == 0.0 {
        out.push_str("0.0");
        return out;
    }
    let sci = format!("{:.15e}", f.abs()); // "d.ddddddddddddddde-X", correctly rounded
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let trim = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.into()
        }
    };
    if !(-4..16).contains(&exp) {
        out.push_str(&trim(&format!("{}.{}", &digits[..1], &digits[1..])));
        out.push_str(&format!("e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs()));
        return out;
    }
    let fixed = if exp < 0 {
        format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits)
    } else {
        let point = (exp + 1) as usize;
        format!("{}.{}", &digits[..point], &digits[point..])
    };
    let fixed = trim(&fixed);
    out.push_str(&fixed);
    if !fixed.contains('.') {
        out.push_str(".0");
    }
    out
}

/// A string as Nim's `escapeJson` writes it.
fn escape_nim(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\u000b"),
            '\r' => out.push_str("\\r"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn as_int(v: &Json, path: &str) -> Result<i128, JsonError> {
    match v {
        Json::Number(n) => n
            .as_i64()
            .map(i128::from)
            .or_else(|| n.as_u64().map(i128::from))
            .map_or_else(|| err(path, format!("{n} is not an integer")), Ok),
        _ => err(path, "expected an integer"),
    }
}

fn as_float(v: &Json, path: &str) -> Result<f64, JsonError> {
    match v {
        Json::Number(n) => n.as_f64().map_or_else(|| err(path, "bad number"), Ok),
        _ => err(path, "expected a number"),
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{Gender, Language};

    use super::*;

    #[test]
    fn json_round_trip() {
        let mut g = Gff::new(*b"UTI ");
        g.root.set("Tag", Value::String(vec![0xc9, b'p', 0xe9, b'e']));
        g.root.set("Cost", Value::Dword(u32::MAX));
        g.root.set("F", Value::Float(0.1));
        g.root.set("V", Value::Void(vec![1, 2, 3]));
        let mut ls = LocString::from_strref(StrRef(5));
        ls.set(Language::ENGLISH, Gender::Male, "x");
        g.root.set("Name", Value::LocString(ls));
        let mut child = Struct::new(0);
        child.set("A", Value::Char(-1));
        g.root.set("S", Value::Struct(child));
        g.root.set("L", Value::List(vec![Struct::new(9), Struct::new(ROOT_STRUCT_ID)]));

        let j = to_json(&g, Codepage::WINDOWS_1252).unwrap();
        assert_eq!(j["Tag"]["value"], "Épée");
        assert_eq!(j["__data_type"], "UTI ");
        assert_eq!(j["Name"]["value"]["id"], 5);
        assert_eq!(j["V"]["value64"], "AQID");
        let back = from_json(&j, Codepage::WINDOWS_1252).unwrap();
        assert_eq!(back, g);
    }

    #[test]
    fn reads_legacy_locstring_id_and_rejects_bad_input() {
        let j = json!({"__data_type": "GFF ", "N": {"type": "cexolocstring", "id": 7, "value": {"0": "a"}}});
        let g = from_json(&j, Codepage::default()).unwrap();
        assert_eq!(g.root.locstring("N").unwrap().strref, StrRef(7));

        let bad = json!({"__data_type": "GFF ", "B": {"type": "byte", "value": 256}});
        assert!(from_json(&bad, Codepage::default()).is_err());
        let bad = json!({"__data_type": "GFF ", "B": {"type": "wat", "value": 1}});
        assert!(from_json(&bad, Codepage::default()).is_err());
    }

    /// Measured from nasher (`unpack --truncateFloats:32`, which leaves
    /// these values as they are).
    #[test]
    fn floats_print_as_nasher_prints_them() {
        for (v, text) in [
            (1.5707942247390747, "1.570794224739075"),
            (95.9779, "95.97790000000001"),
            (0.1, "0.1"),
            (1e20, "1e+20"),
            (1e-5, "1e-05"),
            (1e16, "1e+16"),
            (1e17, "1e+17"),
            (3.0, "3.0"),
            (0.07, "0.07000000000000001"),
            (123456789012345678.0, "1.234567890123457e+17"),
            (1e-8, "1e-08"),
            (2.5e-7, "2.5e-07"),
            (0.0001, "0.0001"),
            (-0.0, "-0.0"),
            (12345678.0, "12345678.0"),
            (1e15, "1000000000000000.0"),
            (9.999999999999998e16, "9.999999999999998e+16"),
            (100.0, "100.0"),
            (0.3, "0.3"),
        ] {
            assert_eq!(g16_float(v), text, "{v:e}");
        }
    }

    /// Measured from `nwn_gff -l json -k json -p` (Nim's float notation).
    #[test]
    fn floats_print_as_nim_prints_them() {
        for (v, text) in [
            (1e20, "1e+20"),
            (1e16, "10000000000000000.0"),
            (1e17, "1e+17"),
            (1.5e17, "1.5e+17"),
            (123456789012345678.0, "1.2345678901234568e+17"),
            (9.999999999999998e16, "99999999999999980.0"),
            (1e-5, "0.00001"),
            (1e-7, "0.0000001"),
            (2.5e-7, "0.00000025"),
            (1e-8, "1e-8"),
            (1.2345e-9, "1.2345e-9"),
            (-1e-8, "-1e-8"),
            (1.17549435e-38, "1.17549435e-38"),
            (5e-324, "5e-324"),
            (-0.0, "-0.0"),
            (0.0, "0.0"),
            (3.0, "3.0"),
            (-123.5, "-123.5"),
            (12345678.0, "12345678.0"),
            (1.5707942247390747, "1.5707942247390747"),
        ] {
            assert_eq!(nim_float(v), text, "{v:e}");
        }
    }

    #[test]
    fn nasher_style_text() {
        let j = json!({
            "__data_type": "ARE ",
            "b": {"type": "float", "value": 1.5707942247390747},
            "A": {"type": "cexostring", "value": "a\u{1}\u{b}\u{e}\u{1b}\"\\/\n\té"},
            "Bearing": {"type": "float", "value": -3.1415812969207764},
            "L": {"type": "list", "value": []},
            "E": {"type": "struct", "__struct_id": 0, "value": {}},
            "W": {"type": "float", "value": -3.1415812969207764},
            "I": {"type": "int", "value": -2},
        });
        let text = to_json_text(&j, TextStyle::NASHER);
        assert_eq!(
            text,
            r#"{
  "__data_type": "ARE ",
  "A": {
    "type": "cexostring",
    "value": "a\u0001\u000b\u000E\u001B\"\\/\n\té"
  },
  "b": {
    "type": "float",
    "value": 1.5708
  },
  "Bearing": {
    "type": "float",
    "value": 3.1416
  },
  "E": {
    "__struct_id": 0,
    "type": "struct",
    "value": {}
  },
  "I": {
    "type": "int",
    "value": -2
  },
  "L": {
    "type": "list",
    "value": []
  },
  "W": {
    "type": "float",
    "value": -3.1416
  }
}
"#
        );
        // Unsorted and untruncated, as nwn_gff writes it.
        let plain = to_json_text(&j, TextStyle::default());
        assert!(plain.starts_with("{\n  \"__data_type\": \"ARE \",\n  \"b\": {"));
        assert!(plain.contains("1.5707942247390747"));
    }

    #[test]
    fn struct_ids_are_signed_as_in_nwn_gff() {
        let mut g = Gff::new(*b"GFF ");
        g.root.set("L", Value::List(vec![Struct::new(0xFFFF_FFFE)]));
        let j = to_json(&g, Codepage::default()).unwrap();
        assert_eq!(j["L"]["value"][0]["__struct_id"], -2);
        assert_eq!(from_json(&j, Codepage::default()).unwrap(), g);
    }
}
