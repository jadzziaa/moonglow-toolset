//! Typed access to the game's GFF resources, without giving up losslessness.
//!
//! Each resource type has a module ([`ifo`], [`are`], [`utc`], ...) of field
//! descriptors: [`Field<T>`] for values, [`ListField`] and [`StructField`] for
//! nested structs, with a nested module per list or struct for the fields
//! inside it. Descriptors read from and write to a raw [`mg_gff::Struct`]
//! through [`StructExt`]; reads fall back to the field's default when the
//! field is missing or has another type, and writes replace or append only
//! that field, so everything else in the file survives.
//!
//! ```
//! use mg_gff::Gff;
//! use mg_schema::{StructExt, ifo};
//! let mut gff = Gff::new(*b"IFO ");
//! assert_eq!(gff.root.read(&ifo::MOD_XP_SCALE), 0);
//! gff.root.write(&ifo::MOD_XP_SCALE, 10);
//! assert_eq!(gff.root.read(&ifo::MOD_XP_SCALE), 10);
//! ```
//!
//! The descriptor modules are generated from the schema observed in the
//! shipped game data (`docs/research/gff-observed-schema.md`); see
//! `crates/mg-corpus-tests/examples/schema_codegen.rs`.

// Nested list modules can share their parent's name (e.g. `ItemList`).
#[allow(clippy::module_inception)]
mod generated;

use std::fmt;

use mg_core::{LocString, ResRef};
use mg_gff::{FieldType, Label, Struct, Value};

pub use generated::*;

/// A `CExoString` value: bytes in the game codepage.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ExoString(pub Vec<u8>);

impl ExoString {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<&str> for ExoString {
    /// ASCII text; use [`mg_core::Codepage::encode`] for anything else.
    fn from(s: &str) -> Self {
        ExoString(s.as_bytes().to_vec())
    }
}

/// A `VOID` value: raw bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Void(pub Vec<u8>);

/// A Rust type that maps to one GFF field type.
pub trait GffValue: Sized {
    const TYPE: FieldType;
    fn from_value(v: &Value) -> Option<Self>;
    fn into_value(self) -> Value;
}

macro_rules! scalar {
    ($($t:ty => $variant:ident),* $(,)?) => {$(
        impl GffValue for $t {
            const TYPE: FieldType = FieldType::$variant;
            fn from_value(v: &Value) -> Option<Self> {
                match v {
                    Value::$variant(x) => Some(*x),
                    _ => None,
                }
            }
            fn into_value(self) -> Value {
                Value::$variant(self)
            }
        }
    )*};
}

scalar! {
    u8 => Byte, i8 => Char, u16 => Word, i16 => Short, u32 => Dword, i32 => Int,
    u64 => Dword64, i64 => Int64, f32 => Float, f64 => Double,
}

impl GffValue for ExoString {
    const TYPE: FieldType = FieldType::String;
    fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::String(b) => Some(ExoString(b.clone())),
            _ => None,
        }
    }
    fn into_value(self) -> Value {
        Value::String(self.0)
    }
}

impl GffValue for ResRef {
    const TYPE: FieldType = FieldType::ResRef;
    /// `None` also for resrefs longer than 16 bytes, which the engine cannot
    /// use; they stay untouched in the tree.
    fn from_value(v: &Value) -> Option<Self> {
        v.as_resref()
    }
    fn into_value(self) -> Value {
        Value::resref(self)
    }
}

impl GffValue for LocString {
    const TYPE: FieldType = FieldType::LocString;
    fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::LocString(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn into_value(self) -> Value {
        Value::LocString(self)
    }
}

impl GffValue for Void {
    const TYPE: FieldType = FieldType::Void;
    fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::Void(b) => Some(Void(b.clone())),
            _ => None,
        }
    }
    fn into_value(self) -> Value {
        Value::Void(self.0)
    }
}

/// A value field: label, type (from `T`) and the default used when it is
/// missing.
pub struct Field<T> {
    pub label: &'static str,
    default: fn() -> T,
}

impl<T> Field<T> {
    pub const fn new(label: &'static str, default: fn() -> T) -> Field<T> {
        Field { label, default }
    }

    pub fn default_value(&self) -> T {
        (self.default)()
    }
}

impl<T> fmt::Debug for Field<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Field({})", self.label)
    }
}

/// A list field and the struct id its items carry.
#[derive(Debug, Clone, Copy)]
pub struct ListField {
    pub label: &'static str,
    pub item_id: u32,
}

/// A struct field and the struct id it carries.
#[derive(Debug, Clone, Copy)]
pub struct StructField {
    pub label: &'static str,
    pub struct_id: u32,
}

impl ListField {
    /// A new, empty list item with the right struct id.
    pub fn new_item(&self) -> Struct {
        Struct::new(self.item_id)
    }
}

/// Typed reads and writes on a raw struct.
pub trait StructExt {
    /// The field's value, or its default if missing or of another type.
    fn read<T: GffValue>(&self, f: &Field<T>) -> T;
    /// The field's value if present with the right type.
    fn try_read<T: GffValue>(&self, f: &Field<T>) -> Option<T>;
    /// Sets the field (in place, or appended), with the field's GFF type.
    fn write<T: GffValue>(&mut self, f: &Field<T>, v: T);
    /// The list's items (empty if missing).
    fn items(&self, f: &ListField) -> &[Struct];
    /// The list's items, creating the list if missing.
    fn items_mut(&mut self, f: &ListField) -> &mut Vec<Struct>;
    /// The struct field, if present.
    fn child_struct(&self, f: &StructField) -> Option<&Struct>;
    /// The struct field, creating it if missing.
    fn child_struct_mut(&mut self, f: &StructField) -> &mut Struct;
}

impl StructExt for Struct {
    fn read<T: GffValue>(&self, f: &Field<T>) -> T {
        self.try_read(f).unwrap_or_else(|| f.default_value())
    }

    fn try_read<T: GffValue>(&self, f: &Field<T>) -> Option<T> {
        self.get(f.label).and_then(T::from_value)
    }

    fn write<T: GffValue>(&mut self, f: &Field<T>, v: T) {
        self.set(f.label, v.into_value());
    }

    fn items(&self, f: &ListField) -> &[Struct] {
        self.list(f.label).unwrap_or(&[])
    }

    fn items_mut(&mut self, f: &ListField) -> &mut Vec<Struct> {
        if !matches!(self.get(f.label), Some(Value::List(_))) {
            self.set(f.label, Value::List(Vec::new()));
        }
        self.list_mut(f.label).expect("list just set")
    }

    fn child_struct(&self, f: &StructField) -> Option<&Struct> {
        self.child(f.label)
    }

    fn child_struct_mut(&mut self, f: &StructField) -> &mut Struct {
        if !matches!(self.get(f.label), Some(Value::Struct(_))) {
            self.set(f.label, Value::Struct(Struct::new(f.struct_id)));
        }
        self.child_mut(f.label).expect("struct just set")
    }
}

/// A field of any kind, for listing a type's fields (verification, generic
/// property views).
#[derive(Debug, Clone, Copy)]
pub struct FieldInfo {
    pub label: &'static str,
    pub field_type: FieldType,
}

/// Fields of a struct that its schema does not list (EE, NWNX or
/// third-party additions). They are kept on save; this is for reporting.
pub fn unknown_fields<'a>(s: &'a Struct, known: &[FieldInfo]) -> Vec<&'a Label> {
    s.fields
        .iter()
        .map(|f| &f.label)
        .filter(|l| !known.iter().any(|k| k.label.as_bytes() == l.as_bytes()))
        .collect()
}

#[cfg(test)]
mod tests {
    use mg_core::StrRef;

    use super::*;

    #[test]
    fn reads_default_on_missing_or_mismatched_type() {
        let mut s = Struct::new(0);
        assert_eq!(s.read(&ifo::MOD_XP_SCALE), 0);
        s.set("Mod_XPScale", Value::Int(5));
        assert_eq!(s.try_read(&ifo::MOD_XP_SCALE), None, "wrong type is not read");
        s.write(&ifo::MOD_XP_SCALE, 10);
        assert_eq!(s.get("Mod_XPScale"), Some(&Value::Byte(10)), "write fixes the type in place");
        assert_eq!(s.fields.len(), 1);
    }

    #[test]
    fn strings_resrefs_locstrings() {
        let mut s = Struct::new(0);
        s.write(&ifo::MOD_TAG, ExoString::from("MODULE"));
        s.write(&ifo::MOD_ON_MOD_LOAD, ResRef::from_str("x_load").unwrap());
        s.write(&ifo::MOD_NAME, LocString::from_strref(StrRef(5)));
        assert_eq!(s.read(&ifo::MOD_TAG).as_bytes(), b"MODULE");
        assert_eq!(s.read(&ifo::MOD_ON_MOD_LOAD).as_str(), Some("x_load"));
        assert_eq!(s.read(&ifo::MOD_NAME).strref, StrRef(5));
        s.set("Mod_OnHeartbeat", Value::ResRef(vec![b'a'; 20]));
        assert!(s.try_read(&ifo::MOD_ON_HEARTBEAT).is_none(), "over-long resrefs are not read");
    }

    #[test]
    fn lists_are_created_with_item_ids() {
        let mut s = Struct::new(0);
        assert!(s.items(&ifo::MOD_HAK_LIST).is_empty());
        let mut hak = ifo::MOD_HAK_LIST.new_item();
        hak.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from("my_hak"));
        s.items_mut(&ifo::MOD_HAK_LIST).push(hak);
        assert_eq!(s.items(&ifo::MOD_HAK_LIST)[0].id, 8);
        assert_eq!(
            s.items(&ifo::MOD_HAK_LIST)[0].read(&ifo::mod_hak_list::MOD_HAK).as_bytes(),
            b"my_hak"
        );
    }

    #[test]
    fn unknown_fields_are_reported() {
        let mut s = Struct::new(0);
        s.write(&ifo::MOD_XP_SCALE, 1);
        s.set("NWNX_Custom", Value::Int(1));
        let unknown = unknown_fields(&s, ifo::FIELDS);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].as_bytes(), b"NWNX_Custom");
    }
}
