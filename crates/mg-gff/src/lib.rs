//! GFF V3.2, the Generic File Format behind module info, areas, blueprints,
//! dialogs, journals, palettes and characters.
//!
//! A [`Gff`] is a tree of [`Struct`]s. Each struct keeps its fields in file
//! order, including fields Moonglow does not know about and (malformed but
//! seen) duplicate labels, so reading and writing a file loses nothing.
//! Strings are kept as bytes in the game's codepage; see [`mg_core::Codepage`].
//!
//! ```
//! use mg_gff::{Gff, Value};
//! let mut gff = Gff::new(*b"UTI ");
//! gff.root.set("Tag", Value::String(b"my_item".to_vec()));
//! gff.root.set("Cost", Value::Dword(10));
//! let bytes = gff.to_bytes().unwrap();
//! let back = Gff::read(&bytes).unwrap();
//! assert_eq!(back, gff);
//! assert_eq!(back.root.dword("Cost"), Some(10));
//! ```

mod diff;
mod json;
mod read;
mod value;
mod write;

pub use diff::{diff, diff_structs};
pub use json::{JsonError, from_json, to_json};
pub use read::ReadError;
pub use value::{Field, FieldType, Label, LabelError, Struct, Value};
pub use write::WriteError;

/// Struct id the root struct carries.
pub const ROOT_STRUCT_ID: u32 = u32::MAX;

/// A GFF file: a four-byte type tag (e.g. `b"UTC "`), a version and the root
/// struct.
#[derive(Debug, Clone, PartialEq)]
pub struct Gff {
    pub file_type: [u8; 4],
    pub version: [u8; 4],
    pub root: Struct,
}

impl Gff {
    /// The only version the game reads and writes.
    pub const VERSION: [u8; 4] = *b"V3.2";

    /// An empty file of the given type.
    pub fn new(file_type: [u8; 4]) -> Gff {
        Gff { file_type, version: Gff::VERSION, root: Struct::new(ROOT_STRUCT_ID) }
    }

    /// Parses a GFF file.
    pub fn read(data: &[u8]) -> Result<Gff, ReadError> {
        read::read(data)
    }

    /// Serializes to GFF bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, WriteError> {
        write::write(self)
    }

    /// The file type as text, without trailing spaces (e.g. `"UTC"`).
    pub fn file_type_str(&self) -> String {
        String::from_utf8_lossy(&self.file_type).trim_end().to_string()
    }
}
