//! The GFF tree: structs, fields, labels and values.

use std::fmt;

use mg_core::{Codepage, Gender, Language, LocString, ResRef};
use thiserror::Error;

/// A field label: at most 16 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label {
    len: u8,
    buf: [u8; Label::MAX_LEN],
}

/// A label longer than 16 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("GFF label {0:?} is longer than 16 bytes")]
pub struct LabelError(pub String);

impl Label {
    pub const MAX_LEN: usize = 16;

    pub fn new(label: &str) -> Result<Label, LabelError> {
        Label::from_bytes(label.as_bytes())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Label, LabelError> {
        if bytes.len() > Label::MAX_LEN {
            return Err(LabelError(String::from_utf8_lossy(bytes).into_owned()));
        }
        let mut buf = [0; Label::MAX_LEN];
        buf[..bytes.len()].copy_from_slice(bytes);
        Ok(Label { len: bytes.len() as u8, buf })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }

    /// The label as text (labels are ASCII in practice; other bytes are
    /// decoded as Latin-1).
    pub fn to_string_lossy(&self) -> String {
        self.as_bytes().iter().map(|&b| b as char).collect()
    }
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string_lossy())
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

/// The type ids GFF stores for each field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum FieldType {
    Byte = 0,
    Char = 1,
    Word = 2,
    Short = 3,
    Dword = 4,
    Int = 5,
    Dword64 = 6,
    Int64 = 7,
    Float = 8,
    Double = 9,
    String = 10,
    ResRef = 11,
    LocString = 12,
    Void = 13,
    Struct = 14,
    List = 15,
}

impl FieldType {
    /// The least and the greatest number an integer type holds; `None`
    /// for a type that is no integer.
    pub fn int_range(self) -> Option<(i128, i128)> {
        Some(match self {
            FieldType::Byte => (0, u8::MAX.into()),
            FieldType::Char => (i8::MIN.into(), i8::MAX.into()),
            FieldType::Word => (0, u16::MAX.into()),
            FieldType::Short => (i16::MIN.into(), i16::MAX.into()),
            FieldType::Dword => (0, u32::MAX.into()),
            FieldType::Int => (i32::MIN.into(), i32::MAX.into()),
            FieldType::Dword64 => (0, u64::MAX.into()),
            FieldType::Int64 => (i64::MIN.into(), i64::MAX.into()),
            _ => return None,
        })
    }

    /// `v` as a value of this integer type; `None` where it does not fit
    /// ([`FieldType::int_range`]) or the type is no integer.
    pub fn int_value(self, v: i128) -> Option<Value> {
        let (min, max) = self.int_range()?;
        if v < min || v > max {
            return None;
        }
        Some(match self {
            FieldType::Byte => Value::Byte(v as u8),
            FieldType::Char => Value::Char(v as i8),
            FieldType::Word => Value::Word(v as u16),
            FieldType::Short => Value::Short(v as i16),
            FieldType::Dword => Value::Dword(v as u32),
            FieldType::Int => Value::Int(v as i32),
            FieldType::Dword64 => Value::Dword64(v as u64),
            FieldType::Int64 => Value::Int64(v as i64),
            _ => return None,
        })
    }

    pub fn from_id(id: u32) -> Option<FieldType> {
        use FieldType::*;
        Some(match id {
            0 => Byte,
            1 => Char,
            2 => Word,
            3 => Short,
            4 => Dword,
            5 => Int,
            6 => Dword64,
            7 => Int64,
            8 => Float,
            9 => Double,
            10 => String,
            11 => ResRef,
            12 => LocString,
            13 => Void,
            14 => Struct,
            15 => List,
            _ => return None,
        })
    }

    /// The lowercase name neverwinter.nim's JSON uses (`"cexostring"`, ...).
    pub fn json_name(self) -> &'static str {
        use FieldType::*;
        match self {
            Byte => "byte",
            Char => "char",
            Word => "word",
            Short => "short",
            Dword => "dword",
            Int => "int",
            Dword64 => "dword64",
            Int64 => "int64",
            Float => "float",
            Double => "double",
            String => "cexostring",
            ResRef => "resref",
            LocString => "cexolocstring",
            Void => "void",
            Struct => "struct",
            List => "list",
        }
    }

    pub fn from_json_name(name: &str) -> Option<FieldType> {
        (0..16).filter_map(FieldType::from_id).find(|t| t.json_name() == name)
    }
}

/// A field value.
///
/// `String` is a `CExoString` and `ResRef` a `CResRef`, both as raw bytes;
/// resrefs may be up to 255 bytes in the file even though the engine only
/// uses 16 (see [`Value::as_resref`]).
#[derive(Debug, Clone)]
pub enum Value {
    Byte(u8),
    Char(i8),
    Word(u16),
    Short(i16),
    Dword(u32),
    Int(i32),
    Dword64(u64),
    Int64(i64),
    Float(f32),
    Double(f64),
    String(Vec<u8>),
    ResRef(Vec<u8>),
    LocString(LocString),
    Void(Vec<u8>),
    Struct(Struct),
    List(Vec<Struct>),
}

impl Value {
    /// `text`, as typed on a command line or in a field, as a value of type
    /// `ty`: a whole number (decimal, or hexadecimal after `0x`) within the
    /// type's range, a finite number, text in `codepage` (as the game reads
    /// a module's strings), a resource name, or a localized string's
    /// English (the rest of `old`'s kept; empty text takes it away). The
    /// one reading of typed text `mg set`, `mg areas` and the Variables
    /// window go through: `mg areas` wrote UTF-8, and took any float.
    pub fn from_text(
        ty: FieldType,
        text: &str,
        old: Option<&Value>,
        codepage: Codepage,
    ) -> Result<Value, String> {
        let t = text.trim();
        let encoded = || {
            codepage
                .encode(text)
                .map(|b| b.into_owned())
                .ok_or_else(|| format!("{text:?} has characters the codepage cannot hold"))
        };
        let float = || -> Result<f64, String> {
            match t.parse::<f64>() {
                Ok(v) if v.is_finite() => Ok(v),
                _ => Err(format!("{text:?} is not a number")),
            }
        };
        Ok(match ty {
            FieldType::Byte
            | FieldType::Char
            | FieldType::Word
            | FieldType::Short
            | FieldType::Dword
            | FieldType::Int
            | FieldType::Dword64
            | FieldType::Int64 => {
                let (min, max) = ty.int_range().expect("an integer type");
                let v = match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                    Some(hex) => i128::from_str_radix(hex, 16).ok(),
                    None => t.parse::<i128>().ok(),
                };
                let v = v.ok_or_else(|| format!("{text:?} is not a whole number"))?;
                ty.int_value(v).ok_or_else(|| {
                    format!("{v} does not fit a {} ({min} to {max})", ty.json_name())
                })?
            }
            FieldType::Float => Value::Float(float()? as f32),
            FieldType::Double => Value::Double(float()?),
            FieldType::String => Value::String(encoded()?),
            FieldType::ResRef => Value::resref(
                ResRef::from_str(t).map_err(|e| format!("{text:?} is not a resource name: {e}"))?,
            ),
            FieldType::LocString => {
                let mut s = match old {
                    Some(Value::LocString(s)) => s.clone(),
                    _ => LocString::default(),
                };
                if text.is_empty() {
                    s.remove(Language::ENGLISH, Gender::Male);
                } else {
                    s.set(Language::ENGLISH, Gender::Male, encoded()?);
                }
                Value::LocString(s)
            }
            FieldType::Void | FieldType::Struct | FieldType::List => {
                return Err(format!("a {} isn't set from text", ty.json_name()));
            }
        })
    }

    pub fn field_type(&self) -> FieldType {
        match self {
            Value::Byte(_) => FieldType::Byte,
            Value::Char(_) => FieldType::Char,
            Value::Word(_) => FieldType::Word,
            Value::Short(_) => FieldType::Short,
            Value::Dword(_) => FieldType::Dword,
            Value::Int(_) => FieldType::Int,
            Value::Dword64(_) => FieldType::Dword64,
            Value::Int64(_) => FieldType::Int64,
            Value::Float(_) => FieldType::Float,
            Value::Double(_) => FieldType::Double,
            Value::String(_) => FieldType::String,
            Value::ResRef(_) => FieldType::ResRef,
            Value::LocString(_) => FieldType::LocString,
            Value::Void(_) => FieldType::Void,
            Value::Struct(_) => FieldType::Struct,
            Value::List(_) => FieldType::List,
        }
    }

    /// The value of a whole number of any width (BYTE, CHAR, WORD, SHORT,
    /// DWORD, INT, DWORD64, INT64; a DWORD64 past `i64::MAX` is none);
    /// `None` for any other type.
    pub fn as_integer(&self) -> Option<i64> {
        match *self {
            Value::Byte(v) => Some(i64::from(v)),
            Value::Char(v) => Some(i64::from(v)),
            Value::Word(v) => Some(i64::from(v)),
            Value::Short(v) => Some(i64::from(v)),
            Value::Dword(v) => Some(i64::from(v)),
            Value::Int(v) => Some(i64::from(v)),
            Value::Dword64(v) => i64::try_from(v).ok(),
            Value::Int64(v) => Some(v),
            _ => None,
        }
    }

    /// A number, whole or not, written as Rust writes it (`42`, `-1`,
    /// `0.5`; a DWORD64 whole); `None` for a value that is no number.
    pub fn number_text(&self) -> Option<String> {
        Some(match self {
            Value::Dword64(v) => v.to_string(),
            Value::Float(v) => v.to_string(),
            Value::Double(v) => v.to_string(),
            v => v.as_integer()?.to_string(),
        })
    }

    /// A resref value from a validated [`ResRef`].
    pub fn resref(r: ResRef) -> Value {
        Value::ResRef(r.as_bytes().to_vec())
    }

    /// The value as a [`ResRef`], if it is a `CResRef` of at most 16 bytes.
    pub fn as_resref(&self) -> Option<ResRef> {
        match self {
            Value::ResRef(b) => ResRef::from_bytes(b).ok(),
            _ => None,
        }
    }
}

/// Floats compare by bit pattern, so a value always equals itself (NaN
/// included) and round-trip checks are exact.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Byte(a), Byte(b)) => a == b,
            (Char(a), Char(b)) => a == b,
            (Word(a), Word(b)) => a == b,
            (Short(a), Short(b)) => a == b,
            (Dword(a), Dword(b)) => a == b,
            (Int(a), Int(b)) => a == b,
            (Dword64(a), Dword64(b)) => a == b,
            (Int64(a), Int64(b)) => a == b,
            (Float(a), Float(b)) => a.to_bits() == b.to_bits(),
            (Double(a), Double(b)) => a.to_bits() == b.to_bits(),
            (String(a), String(b)) => a == b,
            (ResRef(a), ResRef(b)) => a == b,
            (LocString(a), LocString(b)) => a == b,
            (Void(a), Void(b)) => a == b,
            (Struct(a), Struct(b)) => a == b,
            (List(a), List(b)) => a == b,
            _ => false,
        }
    }
}

/// A labelled value.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub label: Label,
    pub value: Value,
}

/// A struct: a type id and an ordered list of fields.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Struct {
    pub id: u32,
    pub fields: Vec<Field>,
}

macro_rules! getters {
    ($($name:ident: $variant:ident -> $ty:ty),* $(,)?) => {$(
        #[doc = concat!("The value of a `", stringify!($variant), "` field; `None` if missing or of another type.")]
        pub fn $name(&self, label: &str) -> Option<$ty> {
            match self.get(label)? {
                Value::$variant(v) => Some(*v),
                _ => None,
            }
        }
    )*};
}

/// A field met by [`Struct::walk_fields`].
#[derive(Debug)]
pub struct FieldAt<'a> {
    /// Where it is, from the root: `/Creature List[0]/Tag` (a field of a
    /// list's first item), `/Tag` (a field of the root).
    pub path: &'a str,
    /// The list it is in an item of (the nearest, if lists are nested).
    pub list: Option<&'a str>,
    pub label: &'a str,
    pub value: &'a Value,
    /// The struct it is a field of.
    pub owner: &'a Struct,
}

impl Struct {
    pub fn new(id: u32) -> Struct {
        Struct { id, fields: Vec::new() }
    }

    /// The first field with this label.
    pub fn field(&self, label: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.label.as_bytes() == label.as_bytes())
    }

    /// The value of the first field with this label.
    pub fn get(&self, label: &str) -> Option<&Value> {
        self.field(label).map(|f| &f.value)
    }

    pub fn get_mut(&mut self, label: &str) -> Option<&mut Value> {
        self.fields
            .iter_mut()
            .find(|f| f.label.as_bytes() == label.as_bytes())
            .map(|f| &mut f.value)
    }

    pub fn contains(&self, label: &str) -> bool {
        self.field(label).is_some()
    }

    /// Sets a field: replaces the first field with this label in place (even
    /// if its type differs), or appends a new one.
    ///
    /// # Panics
    /// If the label is longer than 16 bytes; labels are compile-time names.
    pub fn set(&mut self, label: &str, value: Value) {
        match self.get_mut(label) {
            Some(v) => *v = value,
            None => {
                let label = Label::new(label).expect("GFF label longer than 16 bytes");
                self.fields.push(Field { label, value });
            }
        }
    }

    /// Removes every field with this label; returns the first removed value.
    pub fn remove(&mut self, label: &str) -> Option<Value> {
        let pos = self.fields.iter().position(|f| f.label.as_bytes() == label.as_bytes())?;
        let removed = self.fields.remove(pos).value;
        self.fields.retain(|f| f.label.as_bytes() != label.as_bytes());
        Some(removed)
    }

    getters! {
        byte: Byte -> u8,
        char: Char -> i8,
        word: Word -> u16,
        short: Short -> i16,
        dword: Dword -> u32,
        int: Int -> i32,
        dword64: Dword64 -> u64,
        int64: Int64 -> i64,
        float: Float -> f32,
        double: Double -> f64,
    }

    /// An integer field of any integer type (BYTE, CHAR, WORD, SHORT,
    /// DWORD, INT, DWORD64, INT64), for fields whose type differs between
    /// game versions.
    pub fn integer(&self, label: &str) -> Option<i64> {
        self.get(label)?.as_integer()
    }

    /// Every field of the struct and of the structs under it, each with
    /// where it is: a field before what is inside it, lists' items in
    /// order.
    pub fn walk_fields(&self, each: &mut dyn FnMut(&FieldAt<'_>)) {
        fn walk(s: &Struct, list: Option<&str>, path: &str, each: &mut dyn FnMut(&FieldAt<'_>)) {
            for f in &s.fields {
                let label = f.label.to_string_lossy();
                let at = format!("{path}/{label}");
                each(&FieldAt { path: &at, list, label: &label, value: &f.value, owner: s });
                match &f.value {
                    Value::Struct(c) => walk(c, list, &at, each),
                    Value::List(items) => {
                        for (i, c) in items.iter().enumerate() {
                            walk(c, Some(&label), &format!("{at}[{i}]"), each);
                        }
                    }
                    _ => {}
                }
            }
        }
        walk(self, None, "", each);
    }

    /// The bytes of a `CExoString` field.
    pub fn string(&self, label: &str) -> Option<&[u8]> {
        match self.get(label)? {
            Value::String(v) => Some(v),
            _ => None,
        }
    }

    /// A `CResRef` field as a [`ResRef`] (`None` if missing, of another type
    /// or longer than 16 bytes).
    pub fn resref(&self, label: &str) -> Option<ResRef> {
        self.get(label)?.as_resref()
    }

    pub fn locstring(&self, label: &str) -> Option<&LocString> {
        match self.get(label)? {
            Value::LocString(v) => Some(v),
            _ => None,
        }
    }

    pub fn void(&self, label: &str) -> Option<&[u8]> {
        match self.get(label)? {
            Value::Void(v) => Some(v),
            _ => None,
        }
    }

    pub fn child(&self, label: &str) -> Option<&Struct> {
        match self.get(label)? {
            Value::Struct(v) => Some(v),
            _ => None,
        }
    }

    pub fn child_mut(&mut self, label: &str) -> Option<&mut Struct> {
        match self.get_mut(label)? {
            Value::Struct(v) => Some(v),
            _ => None,
        }
    }

    pub fn list(&self, label: &str) -> Option<&[Struct]> {
        match self.get(label)? {
            Value::List(v) => Some(v),
            _ => None,
        }
    }

    pub fn list_mut(&mut self, label: &str) -> Option<&mut Vec<Struct>> {
        match self.get_mut(label)? {
            Value::List(v) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Typed text as a field's value: text in the codepage (`café` as
    /// Windows-1252's one byte for é, not UTF-8's two), whole numbers
    /// within the type, hexadecimal, no NaN.
    #[test]
    fn typed_text_becomes_a_value_of_the_field_s_type() {
        let cp = Codepage::WINDOWS_1252;
        let of = |ty, text: &str| Value::from_text(ty, text, None, cp);
        assert_eq!(of(FieldType::String, "café"), Ok(Value::String(b"caf\xe9".to_vec())));
        assert_eq!(of(FieldType::Byte, " 255 "), Ok(Value::Byte(255)));
        assert_eq!(of(FieldType::Byte, "256"), Err("256 does not fit a byte (0 to 255)".into()));
        assert_eq!(of(FieldType::Dword, "0x10"), Ok(Value::Dword(16)));
        assert_eq!(of(FieldType::Dword64, &u64::MAX.to_string()), Ok(Value::Dword64(u64::MAX)));
        assert!(of(FieldType::Float, "NaN").is_err());
        assert_eq!(of(FieldType::Float, "1.5"), Ok(Value::Float(1.5)));
        assert!(of(FieldType::String, "日本").is_err(), "not in Windows-1252");
        assert!(of(FieldType::List, "x").is_err());
    }

    #[test]
    fn set_replaces_in_place_and_remove_drops_duplicates() {
        let mut s = Struct::new(0);
        s.set("A", Value::Byte(1));
        s.set("B", Value::Int(2));
        s.set("A", Value::Word(3));
        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.fields[0].value, Value::Word(3));
        assert_eq!(s.byte("A"), None, "type mismatch is None");
        assert_eq!(s.word("A"), Some(3));
        s.fields.push(Field { label: Label::new("B").unwrap(), value: Value::Int(9) });
        assert_eq!(s.remove("B"), Some(Value::Int(2)));
        assert!(!s.contains("B"));
    }

    #[test]
    fn floats_compare_by_bits() {
        assert_eq!(Value::Float(f32::NAN), Value::Float(f32::NAN));
        assert_ne!(Value::Float(0.0), Value::Float(-0.0));
    }

    #[test]
    fn whole_numbers_of_every_width_and_numbers_as_text() {
        let whole = [
            (Value::Byte(200), 200),
            (Value::Char(-5), -5),
            (Value::Word(60000), 60000),
            (Value::Short(-300), -300),
            (Value::Dword(4_000_000_000), 4_000_000_000),
            (Value::Int(-7), -7),
            (Value::Dword64(1 << 40), 1 << 40),
            (Value::Int64(-(1 << 50)), -(1 << 50)),
        ];
        for (v, n) in whole {
            assert_eq!(v.as_integer(), Some(n), "{v:?}");
            assert_eq!(v.number_text(), Some(n.to_string()), "{v:?}");
        }
        // Past what an i64 holds: no whole number, the text still.
        assert_eq!(Value::Dword64(u64::MAX).as_integer(), None);
        assert_eq!(Value::Dword64(u64::MAX).number_text(), Some(u64::MAX.to_string()));
        assert_eq!(Value::Float(0.5).as_integer(), None);
        assert_eq!(Value::Float(0.5).number_text().as_deref(), Some("0.5"));
        assert_eq!(Value::Double(-2.25).number_text().as_deref(), Some("-2.25"));
        assert_eq!(Value::String(b"7".to_vec()).as_integer(), None);
        assert_eq!(Value::String(b"7".to_vec()).number_text(), None);
    }

    #[test]
    fn labels() {
        assert!(Label::new("0123456789abcdef").is_ok());
        assert!(Label::new("0123456789abcdefg").is_err());
    }

    #[test]
    fn json_names_round_trip() {
        for id in 0..16 {
            let t = FieldType::from_id(id).unwrap();
            assert_eq!(FieldType::from_json_name(t.json_name()), Some(t));
        }
        assert_eq!(FieldType::from_id(16), None);
    }
    #[test]
    fn fields_are_walked_with_where_they_are() {
        let mut item = Struct::new(0);
        item.set("Tag", Value::String(b"b".to_vec()));
        let mut inner = Struct::new(0);
        inner.set("Deep", Value::Int(1));
        item.set("Inner", Value::Struct(inner));
        let mut root = Struct::new(0);
        root.set("Tag", Value::String(b"a".to_vec()));
        root.set("List", Value::List(vec![Struct::new(0), item]));
        let mut seen = Vec::new();
        root.walk_fields(&mut |f| seen.push((f.path.to_string(), f.list.map(str::to_string))));
        let list = Some("List".to_string());
        assert_eq!(
            seen,
            [
                ("/Tag".to_string(), None),
                ("/List".to_string(), None),
                ("/List[1]/Tag".to_string(), list.clone()),
                ("/List[1]/Inner".to_string(), list.clone()),
                ("/List[1]/Inner/Deep".to_string(), list),
            ]
        );
    }
}
