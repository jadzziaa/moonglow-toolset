//! The GFF tree: structs, fields, labels and values.

use std::fmt;

use mg_core::{LocString, ResRef};
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
}
