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
        obj.insert("__struct_id".into(), json!(s.id));
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
                field.insert("__struct_id".into(), json!(child.id));
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
        Some(v) => {
            as_int(v, path)?.try_into().or_else(|_| err(path, "__struct_id out of range"))?
        }
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
                        Some(v) => as_int(v, &fpath)? as u32,
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
}
