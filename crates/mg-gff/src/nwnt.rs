//! NWNT, the line-per-field text of a GFF that WilliamDraco's `nwn_nwnt`
//! writes and nasher keeps a project's GFF files in (`gffFormat = nwnt`):
//!
//! ```text
//! data_type = UTC
//! ClassList[]$List = 2
//! ClassList[].Class$Int = 11
//! FirstName$CExoLocString = -1
//! FirstName[0] = Black Bishop
//! ```
//!
//! [`to_nwnt`] writes what `nwn_nwnt` 1.3 writes, to the byte: a struct's
//! fields sorted by label (bytes), a nested field named by the path to it
//! (`Struct.Label`, a list's item `List[].Label`), each list item opened by
//! a line of its own with the item's struct id, an empty list as
//! `List[]$List = _`, text as UTF-8 from the game's codepage with line
//! breaks written `\n` and `\r`, floats to a number of places (`-p`, nasher's
//! `truncateFloats`) with a `Bearing` of -π written as π, voids in base64,
//! a localized string's entries in the order of Nim's hash table, lines
//! ending in CRLF. [`from_nwnt`] reads that back (and lines ending in LF,
//! indented, or in another order), as `nwn_nwnt` does.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use mg_core::{Codepage, LocString, LocStringKey, StrRef};
use thiserror::Error;

use crate::{Field, Gff, Label, ROOT_STRUCT_ID, Struct, Value};

/// NWNT that can't be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("line {line}: {message}")]
pub struct NwntError {
    /// The line (from 1) it was found at.
    pub line: usize,
    pub message: String,
}

/// `gff` as NWNT, its text read in `codepage` and its floats written to
/// `float_places` places (`nwn_nwnt -p`, 4 by default).
pub fn to_nwnt(gff: &Gff, codepage: Codepage, float_places: u8) -> String {
    let mut out = String::new();
    out.push_str("data_type = ");
    out.push_str(String::from_utf8_lossy(&gff.file_type).replace('\0', "").trim_end());
    out.push_str("\r\n");
    write_struct(&mut out, "", &gff.root, codepage, usize::from(float_places));
    out
}

fn line(out: &mut String, name: &str, kind: &str, value: &str) {
    out.push_str(name);
    if !kind.is_empty() {
        out.push('$');
        out.push_str(kind);
    }
    out.push_str(" =");
    if !value.is_empty() {
        out.push(' ');
        out.push_str(value);
    }
    out.push_str("\r\n");
}

fn escape(text: &str) -> String {
    text.replace('\n', "\\n").replace('\r', "\\r")
}

fn float(f: f64, places: usize) -> String {
    if f.is_nan() {
        "nan".into()
    } else if f.is_infinite() {
        if f > 0.0 { "inf".into() } else { "-inf".into() }
    } else {
        format!("{f:.places$}")
    }
}

fn write_struct(out: &mut String, prefix: &str, s: &Struct, cp: Codepage, places: usize) {
    let mut fields: Vec<&Field> = s.fields.iter().collect();
    fields.sort_by(|a, b| a.label.as_bytes().cmp(b.label.as_bytes()));
    for field in fields {
        let label = field.label.to_string_lossy();
        let name = format!("{prefix}{label}");
        let n = |v: &dyn std::fmt::Display| v.to_string();
        match &field.value {
            Value::Byte(v) => line(out, &name, "Byte", &n(v)),
            Value::Char(v) => line(out, &name, "Char", &n(v)),
            Value::Word(v) => line(out, &name, "Word", &n(v)),
            Value::Short(v) => line(out, &name, "Short", &n(v)),
            Value::Dword(v) => line(out, &name, "Dword", &n(v)),
            Value::Int(v) => line(out, &name, "Int", &n(v)),
            Value::Dword64(v) => line(out, &name, "Dword64", &n(v)),
            Value::Int64(v) => line(out, &name, "Int64", &n(v)),
            Value::Float(v) => {
                let mut text = float(f64::from(*v), places);
                // (`nwn_nwnt`'s own: a bearing of -π, as π.)
                if label == "Bearing" && text == float(-std::f64::consts::PI, places) {
                    text.remove(0);
                }
                line(out, &name, "Float", &text)
            }
            Value::Double(v) => line(out, &name, "Double", &float(*v, places)),
            Value::String(v) => line(out, &name, "CExoString", &escape(&cp.decode(v))),
            Value::ResRef(v) => line(out, &name, "ResRef", &cp.decode(v)),
            Value::Void(v) => line(out, &name, "Void", &BASE64.encode(v)),
            Value::LocString(ls) => {
                let strref = if ls.strref.is_none() { "-1".to_string() } else { n(&ls.strref.0) };
                line(out, &name, "CExoLocString", &strref);
                for (key, text) in nim_table_order(&ls.strings) {
                    line(out, &format!("{name}[{key}]"), "", &escape(&cp.decode(text)));
                }
            }
            Value::Struct(child) => {
                line(out, &name, "Struct", &n(&(child.id as i32)));
                write_struct(out, &format!("{name}."), child, cp, places);
            }
            Value::List(items) if items.is_empty() => line(out, &format!("{name}[]"), "List", "_"),
            Value::List(items) => {
                let item_prefix = format!("{name}[].");
                for item in items {
                    line(out, &format!("{name}[]"), "List", &n(&(item.id as i32)));
                    write_struct(out, &item_prefix, item, cp, places);
                }
            }
        }
    }
}

/// The path of a field whose label NWNT can't keep (one with a dot in it,
/// which `nwn_nwnt` reads as a struct's), if `s` has one; `None` if all of
/// its fields can be written and read back.
pub fn nwnt_unkept_label(s: &Struct) -> Option<String> {
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        if label.contains('.') {
            return Some(label);
        }
        let inner = match &f.value {
            Value::Struct(c) => nwnt_unkept_label(c),
            Value::List(items) => items.iter().find_map(nwnt_unkept_label),
            _ => None,
        };
        if let Some(path) = inner {
            return Some(format!("{label}/{path}"));
        }
    }
    None
}

/// Nim's hash of an integer (`hashWangYi1`, Nim 1.4 and after).
fn nim_hash(x: u64) -> u64 {
    const P0: u64 = 0xa076_1d64_78bd_642f;
    const P1: u64 = 0xe703_7ed1_a0b4_28db;
    const P58: u64 = 0xeb44_acca_b455_d165 ^ 8;
    fn hi_xor_lo(a: u64, b: u64) -> u64 {
        let p = u128::from(a) * u128::from(b);
        ((p >> 64) as u64) ^ (p as u64)
    }
    hi_xor_lo(hi_xor_lo(P0, x ^ P1), P58)
}

/// A localized string's entries in the order `nwn_nwnt` writes them: that of
/// a Nim `Table[int, string]` they were put into one by one (32 slots,
/// open addressing, grown as Nim grows it), a later entry of a key taking
/// the earlier one's text.
fn nim_table_order(entries: &[(LocStringKey, Vec<u8>)]) -> Vec<(u32, &[u8])> {
    type Slots<'a> = Vec<Option<(u32, &'a [u8])>>;
    fn place<'a>(slots: &mut Slots<'a>, entry: (u32, &'a [u8])) {
        let mask = slots.len() - 1;
        let mut hc = nim_hash(u64::from(entry.0));
        if hc == 0 {
            hc = 314_159_265;
        }
        let mut i = (hc as usize) & mask;
        while slots[i].is_some() {
            i = (i + 1) & mask;
        }
        slots[i] = Some(entry);
    }
    let mut slots: Slots<'_> = vec![None; 32];
    let mut count = 0usize;
    for (key, text) in entries {
        if let Some(had) = slots.iter_mut().flatten().find(|(k, _)| *k == key.0) {
            had.1 = text;
            continue;
        }
        if slots.len() * 2 < count * 3 || slots.len() - count < 4 {
            let grown = vec![None; slots.len() * 2];
            let old = std::mem::replace(&mut slots, grown);
            for entry in old.into_iter().flatten() {
                place(&mut slots, entry);
            }
        }
        place(&mut slots, (key.0, text));
        count += 1;
    }
    slots.into_iter().flatten().collect()
}

/// One line of NWNT: `name$kind = value`, or a localized string's entry
/// `name[n] = text` (no kind).
struct Line<'a> {
    at: usize,
    name: &'a str,
    kind: Option<String>,
    value: &'a str,
}

/// Reads NWNT (as [`to_nwnt`] writes it, or written by hand: lines ending in
/// LF, indented, fields in any order), its text written into `codepage`.
pub fn from_nwnt(text: &str, codepage: Codepage) -> Result<Gff, NwntError> {
    let mut lines = Vec::new();
    let mut file_type = None;
    for (n, raw) in text.split('\n').enumerate() {
        let at = n + 1;
        let l = raw.strip_suffix('\r').unwrap_or(raw).trim_start();
        if l.is_empty() {
            continue;
        }
        let bad = |message: &str| NwntError { line: at, message: message.into() };
        let (left, value) = l.split_once('=').ok_or_else(|| bad("no \"=\" in the line"))?;
        let value = value.strip_prefix(' ').unwrap_or(value);
        let left = left.trim_end();
        if file_type.is_none() {
            if left != "data_type" {
                return Err(bad("the file doesn't begin with its data_type"));
            }
            let t = value.trim().as_bytes();
            if t.is_empty() || t.len() > 4 {
                return Err(bad("a data_type is one to four letters"));
            }
            let mut ft = *b"    ";
            ft[..t.len()].copy_from_slice(t);
            file_type = Some(ft);
            continue;
        }
        let (name, kind) = match left.split_once('$') {
            Some((name, kind)) => (name, Some(kind.trim().to_ascii_lowercase())),
            None => (left, None),
        };
        lines.push(Line { at, name, kind, value });
    }
    let file_type = file_type.ok_or(NwntError { line: 1, message: "no data_type".into() })?;
    let mut gff = Gff::new(file_type);
    let mut reader = Reader { lines: &lines, next: 0, cp: codepage };
    gff.root = reader.read_struct("", ROOT_STRUCT_ID)?;
    if let Some(l) = lines.get(reader.next) {
        return Err(NwntError {
            line: l.at,
            message: format!("{}: a field of a struct or list not named before it", l.name),
        });
    }
    Ok(gff)
}

struct Reader<'a> {
    lines: &'a [Line<'a>],
    next: usize,
    cp: Codepage,
}

impl Reader<'_> {
    /// The fields of the struct whose fields' names begin with `prefix`, up
    /// to the first line that isn't one.
    fn read_struct(&mut self, prefix: &str, id: u32) -> Result<Struct, NwntError> {
        let mut s = Struct::new(id);
        let lines = self.lines;
        while let Some(l) = lines.get(self.next) {
            let Some(rest) = l.name.strip_prefix(prefix) else { break };
            let at = l.at;
            let bad = |message: String| NwntError { line: at, message };
            // (An empty label is one, as BioWare's palettes have.)
            let label = |text: &str| {
                if text.contains('.') {
                    return Err(bad(format!(
                        "{}: a field of a struct or list not named before it",
                        l.name
                    )));
                }
                Label::new(text).map_err(|e| bad(e.to_string()))
            };
            self.next += 1;
            let Some(kind) = &l.kind else {
                // A localized string's entry: `Name[n] = text`.
                let entry = rest.strip_suffix(']').and_then(|r| r.rsplit_once('['));
                let Some((of, key)) = entry else {
                    return Err(bad(format!("{}: no $ and type", l.name)));
                };
                let key: u32 = key.parse().map_err(|_| bad(format!("bad language {key:?}")))?;
                let of = label(of)?;
                let Some(Value::LocString(ls)) =
                    (s.fields.iter_mut().rev()).find(|f| f.label == of).map(|f| &mut f.value)
                else {
                    return Err(bad(format!("{}: no localized string before it", l.name)));
                };
                let text = self.text(&unescape(l.value), at)?;
                ls.strings.push((LocStringKey(key), text));
                continue;
            };
            let int = |min: i128, max: i128| -> Result<i128, NwntError> {
                let v: i128 = (l.value.trim().parse())
                    .map_err(|_| bad(format!("{}: {:?} is no whole number", l.name, l.value)))?;
                if v < min || v > max {
                    return Err(bad(format!("{}: {v} out of range", l.name)));
                }
                Ok(v)
            };
            let real = || -> Result<f64, NwntError> {
                (l.value.trim().parse())
                    .map_err(|_| bad(format!("{}: {:?} is no number", l.name, l.value)))
            };
            // (A struct's id as `nwn_nwnt` writes it, signed, or not.)
            let id = || int(i128::from(i32::MIN), i128::from(u32::MAX)).map(|v| v as u32);
            let value = match kind.as_str() {
                "byte" => Value::Byte(int(0, 255)? as u8),
                "char" => Value::Char(int(-128, 127)? as i8),
                "word" => Value::Word(int(0, 0xFFFF)? as u16),
                "short" => Value::Short(int(-0x8000, 0x7FFF)? as i16),
                "dword" => Value::Dword(int(0, i128::from(u32::MAX))? as u32),
                "int" => Value::Int(int(i128::from(i32::MIN), i128::from(i32::MAX))? as i32),
                "dword64" => Value::Dword64(int(0, i128::from(u64::MAX))? as u64),
                "int64" => Value::Int64(int(i128::from(i64::MIN), i128::from(i64::MAX))? as i64),
                "float" => Value::Float(real()? as f32),
                "double" => Value::Double(real()?),
                "cexostring" => Value::String(self.text(&unescape(l.value), at)?),
                "resref" => Value::ResRef(self.text(l.value, at)?),
                "void" => Value::Void(
                    BASE64
                        .decode(l.value.trim())
                        .map_err(|e| bad(format!("{}: bad base64: {e}", l.name)))?,
                ),
                "cexolocstring" => {
                    let strref = int(-1, i128::from(u32::MAX))?;
                    let strref = if strref < 0 { StrRef::NONE } else { StrRef(strref as u32) };
                    Value::LocString(LocString { strref, strings: Vec::new() })
                }
                "struct" => {
                    let label = label(rest)?;
                    let id = id()?;
                    let child = self.read_struct(&format!("{}.", l.name), id)?;
                    s.fields.push(Field { label, value: Value::Struct(child) });
                    continue;
                }
                "list" => {
                    let Some(of) = rest.strip_suffix("[]") else {
                        return Err(bad(format!("{}: a list's name ends in []", l.name)));
                    };
                    let of = label(of)?;
                    let item = if l.value.trim() == "_" {
                        None
                    } else {
                        let id = id()?;
                        Some(self.read_struct(&format!("{}.", l.name), id)?)
                    };
                    let list = s.fields.iter_mut().rev().find_map(|f| match &mut f.value {
                        Value::List(items) if f.label == of => Some(items),
                        _ => None,
                    });
                    match list {
                        Some(items) => items.extend(item),
                        None => s.fields.push(Field {
                            label: of,
                            value: Value::List(item.into_iter().collect()),
                        }),
                    }
                    continue;
                }
                other => return Err(bad(format!("{}: no type {other:?}", l.name))),
            };
            let label = label(rest)?;
            s.fields.push(Field { label, value });
        }
        Ok(s)
    }

    fn text(&self, text: &str, at: usize) -> Result<Vec<u8>, NwntError> {
        match self.cp.encode(text) {
            Some(b) => Ok(b.into_owned()),
            None => Err(NwntError {
                line: at,
                message: "text has characters the codepage cannot represent".into(),
            }),
        }
    }
}

/// `\n` and `\r` as the line breaks they stand for (and nothing else:
/// `nwn_nwnt` escapes no backslash).
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('\\', Some('n')) => {
                chars.next();
                out.push('\n');
            }
            ('\\', Some('r')) => {
                chars.next();
                out.push('\r');
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::{Gender, Language};

    fn sample() -> Gff {
        let mut g = Gff::new(*b"UTI ");
        g.root.set("Tag", Value::String(b"caf\xE9 line1\nline2\r end".to_vec()));
        g.root.set("Big", Value::Float(123_456_789.0));
        g.root.set("Bearing", Value::Float(-std::f32::consts::PI));
        g.root.set("Orientation", Value::Float(-std::f32::consts::PI));
        g.root.set("D", Value::Double(0.125));
        g.root.set("V", Value::Void(vec![0x41, 0x41, 0x45, 0x43]));
        g.root.set("C", Value::Char(-5));
        g.root.set("_u", Value::Byte(1));
        g.root.set("lower", Value::Byte(1));
        g.root.set("Upper", Value::Byte(1));
        g.root.set("Empty", Value::String(Vec::new()));
        g.root.set("Sp", Value::String(b"  lead".to_vec()));
        let mut name = LocString::from_text(Language::ENGLISH, Gender::Male, b"h\xE9".to_vec());
        name.strref = StrRef(12);
        name.strings.push((LocStringKey(1), b"fem".to_vec()));
        name.strings.push((LocStringKey(4), b"de".to_vec()));
        g.root.set("Name", Value::LocString(name));
        let mut inner = Struct::new(9);
        inner.set("Y", Value::Word(2));
        let mut s = Struct::new(u32::MAX);
        s.set("X", Value::Byte(1));
        s.set("In", Value::Struct(inner));
        g.root.set("S", Value::Struct(s));
        let mut item = Struct::new(3);
        item.set("A", Value::Int(1));
        item.set("Sub", Value::List(Vec::new()));
        g.root.set("L", Value::List(vec![item, Struct::new(4)]));
        g
    }

    /// What `nwn_nwnt` 1.3 wrote of the same file (made with `nwn_gff` from
    /// JSON), to the byte.
    #[test]
    fn a_file_is_written_as_nwn_nwnt_writes_it() {
        let text = to_nwnt(&sample(), Codepage::WINDOWS_1252, 4);
        let expected = "data_type = UTI\r\n\
            Bearing$Float = 3.1416\r\n\
            Big$Float = 123456792.0000\r\n\
            C$Char = -5\r\n\
            D$Double = 0.1250\r\n\
            Empty$CExoString =\r\n\
            L[]$List = 3\r\n\
            L[].A$Int = 1\r\n\
            L[].Sub[]$List = _\r\n\
            L[]$List = 4\r\n\
            Name$CExoLocString = 12\r\n\
            Name[4] = de\r\n\
            Name[0] = h\u{e9}\r\n\
            Name[1] = fem\r\n\
            Orientation$Float = -3.1416\r\n\
            S$Struct = -1\r\n\
            S.In$Struct = 9\r\n\
            S.In.Y$Word = 2\r\n\
            S.X$Byte = 1\r\n\
            Sp$CExoString =   lead\r\n\
            Tag$CExoString = caf\u{e9} line1\\nline2\\r end\r\n\
            Upper$Byte = 1\r\n\
            V$Void = QUFFQw==\r\n\
            _u$Byte = 1\r\n\
            lower$Byte = 1\r\n";
        assert_eq!(text, expected);
        // At two places a tie goes to the even digit, as C's printf has it.
        assert!(to_nwnt(&sample(), Codepage::WINDOWS_1252, 2).contains("D$Double = 0.12\r\n"));
    }

    /// Twelve entries come out in the order `nwn_nwnt` wrote them.
    #[test]
    fn a_localized_string_s_entries_are_in_nim_s_order() {
        let entries: Vec<(LocStringKey, Vec<u8>)> =
            (0..12).map(|k| (LocStringKey(k), vec![b'a' + k as u8])).collect();
        let keys: Vec<u32> = nim_table_order(&entries).iter().map(|e| e.0).collect();
        assert_eq!(keys, [4, 9, 6, 5, 7, 10, 0, 11, 3, 2, 8, 1]);
        // Past the table's first size it grows, and every entry stays.
        let many: Vec<(LocStringKey, Vec<u8>)> =
            (0..60).map(|k| (LocStringKey(k), Vec::new())).collect();
        assert_eq!(nim_table_order(&many).len(), 60);
    }

    /// Read back, the file is the same but for its fields' order (sorted)
    /// and the bearing.
    #[test]
    fn a_file_reads_back() {
        let text = to_nwnt(&sample(), Codepage::WINDOWS_1252, 7);
        let back = from_nwnt(&text, Codepage::WINDOWS_1252).unwrap();
        assert_eq!(back.file_type, *b"UTI ");
        assert_eq!(back.root.string("Tag"), Some(&b"caf\xE9 line1\nline2\r end"[..]));
        assert_eq!(back.root.string("Empty"), Some(&b""[..]));
        assert_eq!(back.root.string("Sp"), Some(&b"  lead"[..]));
        assert_eq!(back.root.void("V"), Some(&[0x41, 0x41, 0x45, 0x43][..]));
        assert_eq!(back.root.float("Big"), Some(123_456_792.0));
        assert_eq!(back.root.get("C").and_then(Value::as_integer), Some(-5));
        let name = back.root.locstring("Name").unwrap();
        assert_eq!(name.strref, StrRef(12));
        assert_eq!(name.strings.len(), 3);
        let s = back.root.child("S").unwrap();
        assert_eq!(s.id, u32::MAX);
        assert_eq!(s.child("In").unwrap().id, 9);
        let l = back.root.list("L").unwrap();
        assert_eq!(l.iter().map(|i| i.id).collect::<Vec<_>>(), [3, 4]);
        assert_eq!(l[0].list("Sub"), Some(&[][..]));
        // Written again, the same text.
        assert_eq!(to_nwnt(&back, Codepage::WINDOWS_1252, 7), text);
    }

    /// Lines ending in LF, indented, in another order, read as `nwn_nwnt`
    /// reads them.
    #[test]
    fn a_file_written_by_hand_is_read() {
        let text = "data_type = UTI\n  Z$Byte = 1\nA$CExoString = hi\\nthere\nL[]$List = 2\n   \
                    L[].Q$Int = -4\nL[]$List = 2\n";
        let g = from_nwnt(text, Codepage::WINDOWS_1252).unwrap();
        assert_eq!(g.root.string("A"), Some(&b"hi\nthere"[..]));
        assert_eq!(g.root.list("L").unwrap().len(), 2);
        assert_eq!(g.root.list("L").unwrap()[0].integer("Q"), Some(-4));
        assert_eq!(g.root.fields.len(), 3);
    }

    #[test]
    fn what_isn_t_nwnt_is_refused() {
        let cp = Codepage::WINDOWS_1252;
        let refused = |text: &str| from_nwnt(text, cp).unwrap_err().to_string();
        assert!(refused("").contains("data_type"));
        assert!(refused("Tag$CExoString = x").contains("data_type"));
        assert!(refused("data_type = UTI\nTag").contains("line 2"));
        assert!(refused("data_type = UTI\nS.X$Byte = 1").contains("not named before it"));
        assert!(refused("data_type = UTI\nX$Byte = 256").contains("out of range"));
        assert!(refused("data_type = UTI\nX$Bite = 1").contains("no type"));
        assert!(refused("data_type = UTI\nAVeryLongLabelIndeed$Byte = 1").contains("16"));
        assert!(refused("data_type = UTI\nN[0] = x").contains("no localized string"));
        assert!(refused("data_type = UTI\nV$Void = !!").contains("base64"));
        assert!(refused("data_type = UTI\nT$CExoString = \u{4e2d}").contains("codepage"));
    }

    #[test]
    fn a_label_with_a_dot_is_found() {
        let mut g = sample();
        assert_eq!(nwnt_unkept_label(&g.root), None);
        let mut item = Struct::new(0);
        item.set("a.b", Value::Byte(1));
        g.root.set("L", Value::List(vec![item]));
        assert_eq!(nwnt_unkept_label(&g.root).as_deref(), Some("L/a.b"));
    }

    /// Cut anywhere, a file is read or refused, never a panic.
    #[test]
    fn a_truncated_file_never_panics() {
        let text = to_nwnt(&sample(), Codepage::WINDOWS_1252, 4);
        for cut in 0..text.len() {
            if text.is_char_boundary(cut) {
                let _ = from_nwnt(&text[..cut], Codepage::WINDOWS_1252);
            }
        }
    }
}
