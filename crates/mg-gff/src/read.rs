//! GFF reader.
//!
//! Every offset and index in the file is checked, each struct and field may be
//! used only once (so hostile files cannot loop), and nesting depth is capped.

use mg_core::bin::{BinError, Reader, slice};
use mg_core::{LocString, LocStringKey, StrRef};
use thiserror::Error;

use crate::value::{Field, FieldType, Label, Struct, Value};
use crate::{Gff, ROOT_STRUCT_ID};

/// Nesting deeper than this is treated as corruption. Real files stay well
/// below 16.
const MAX_DEPTH: usize = 128;

/// Why a GFF file could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReadError {
    #[error("not a GFF V3.2 file (version {0:?})")]
    BadVersion(String),
    #[error("truncated or corrupt GFF: {0}")]
    Bin(#[from] BinError),
    #[error("GFF has no root struct")]
    NoRoot,
    #[error("struct index {0} is out of range")]
    BadStructIndex(u32),
    #[error("struct {0} is referenced more than once")]
    StructReused(u32),
    #[error("field index {0} is out of range")]
    BadFieldIndex(u32),
    #[error("field {0} is referenced more than once")]
    FieldReused(u32),
    #[error("label index {0} is out of range")]
    BadLabelIndex(u32),
    #[error("field {field} has unknown type {type_id}")]
    BadFieldType { field: u32, type_id: u32 },
    #[error("struct nesting is deeper than {MAX_DEPTH}")]
    TooDeep,
}

struct Tables<'a> {
    structs: &'a [u8],
    fields: &'a [u8],
    labels: &'a [u8],
    field_data: &'a [u8],
    field_indices: &'a [u8],
    list_indices: &'a [u8],
    struct_used: Vec<bool>,
    field_used: Vec<bool>,
}

pub(crate) fn read(data: &[u8]) -> Result<Gff, ReadError> {
    let mut r = Reader::new(data);
    let file_type: [u8; 4] = r.array()?;
    let version: [u8; 4] = r.array()?;
    if &version != b"V3.2" {
        return Err(ReadError::BadVersion(String::from_utf8_lossy(&version).into_owned()));
    }
    let mut table = |elem_size: usize| -> Result<&[u8], ReadError> {
        let offset = r.u32_usize()?;
        let count = r.u32_usize()?;
        let len = count
            .checked_mul(elem_size)
            .ok_or(BinError::OutOfBounds { offset, len: data.len() })?;
        Ok(slice(data, offset, len)?)
    };
    let structs = table(12)?;
    let fields = table(12)?;
    let labels = table(16)?;
    let field_data = table(1)?;
    let field_indices = table(1)?;
    let list_indices = table(1)?;

    let mut t = Tables {
        structs,
        fields,
        labels,
        field_data,
        field_indices,
        list_indices,
        struct_used: vec![false; structs.len() / 12],
        field_used: vec![false; fields.len() / 12],
    };
    if t.struct_used.is_empty() {
        return Err(ReadError::NoRoot);
    }
    let mut root = t.read_struct(0, 0)?;
    // The root's id is always 0xFFFFFFFF; some writers store something else,
    // which the game ignores.
    root.id = ROOT_STRUCT_ID;
    Ok(Gff { file_type, version, root })
}

impl Tables<'_> {
    fn read_struct(&mut self, index: u32, depth: usize) -> Result<Struct, ReadError> {
        if depth > MAX_DEPTH {
            return Err(ReadError::TooDeep);
        }
        let used =
            self.struct_used.get_mut(index as usize).ok_or(ReadError::BadStructIndex(index))?;
        if std::mem::replace(used, true) {
            return Err(ReadError::StructReused(index));
        }
        let mut r = Reader::at(self.structs, index as usize * 12)?;
        let id = r.u32()?;
        let data_or_offset = r.u32()?;
        let field_count = r.u32_usize()?;

        let mut s = Struct { id, fields: Vec::with_capacity(field_count.min(1024)) };
        match field_count {
            0 => {}
            1 => s.fields.push(self.read_field(data_or_offset, depth)?),
            n => {
                let len = n.checked_mul(4).ok_or(BinError::OutOfBounds {
                    offset: data_or_offset as usize,
                    len: self.field_indices.len(),
                })?;
                let indices = slice(self.field_indices, data_or_offset as usize, len)?;
                for chunk in indices.as_chunks::<4>().0 {
                    let fi = u32::from_le_bytes(*chunk);
                    s.fields.push(self.read_field(fi, depth)?);
                }
            }
        }
        Ok(s)
    }

    fn read_field(&mut self, index: u32, depth: usize) -> Result<Field, ReadError> {
        let used =
            self.field_used.get_mut(index as usize).ok_or(ReadError::BadFieldIndex(index))?;
        if std::mem::replace(used, true) {
            return Err(ReadError::FieldReused(index));
        }
        let mut r = Reader::at(self.fields, index as usize * 12)?;
        let type_id = r.u32()?;
        let label_index = r.u32()?;
        let data = r.u32()?;

        let label_bytes = slice(self.labels, label_index as usize * 16, 16)
            .map_err(|_| ReadError::BadLabelIndex(label_index))?;
        let label_len = label_bytes.iter().position(|&b| b == 0).unwrap_or(16);
        let label = Label::from_bytes(&label_bytes[..label_len]).expect("at most 16 bytes");

        let ty =
            FieldType::from_id(type_id).ok_or(ReadError::BadFieldType { field: index, type_id })?;
        let value = match ty {
            FieldType::Byte => Value::Byte(data as u8),
            FieldType::Char => Value::Char(data as u8 as i8),
            FieldType::Word => Value::Word(data as u16),
            FieldType::Short => Value::Short(data as u16 as i16),
            FieldType::Dword => Value::Dword(data),
            FieldType::Int => Value::Int(data as i32),
            FieldType::Float => Value::Float(f32::from_bits(data)),
            FieldType::Dword64 => Value::Dword64(self.data_at(data)?.u64()?),
            FieldType::Int64 => Value::Int64(self.data_at(data)?.i64()?),
            FieldType::Double => Value::Double(self.data_at(data)?.f64()?),
            FieldType::String => {
                let mut r = self.data_at(data)?;
                let len = r.u32_usize()?;
                Value::String(r.bytes(len)?.to_vec())
            }
            FieldType::ResRef => {
                let mut r = self.data_at(data)?;
                let len = r.u8()? as usize;
                Value::ResRef(r.bytes(len)?.to_vec())
            }
            FieldType::LocString => Value::LocString(self.read_locstring(data)?),
            FieldType::Void => {
                let mut r = self.data_at(data)?;
                let len = r.u32_usize()?;
                Value::Void(r.bytes(len)?.to_vec())
            }
            FieldType::Struct => Value::Struct(self.read_struct(data, depth + 1)?),
            FieldType::List => {
                let mut r = Reader::at(self.list_indices, data as usize)?;
                let count = r.u32_usize()?;
                let mut list = Vec::with_capacity(count.min(r.remaining() / 4));
                for _ in 0..count {
                    let si = r.u32()?;
                    list.push(self.read_struct(si, depth + 1)?);
                }
                Value::List(list)
            }
        };
        Ok(Field { label, value })
    }

    fn data_at(&self, offset: u32) -> Result<Reader<'_>, ReadError> {
        Ok(Reader::at(self.field_data, offset as usize)?)
    }

    fn read_locstring(&self, offset: u32) -> Result<LocString, ReadError> {
        let mut r = self.data_at(offset)?;
        let _total_size = r.u32()?;
        let strref = StrRef(r.u32()?);
        let count = r.u32_usize()?;
        let mut strings = Vec::with_capacity(count.min(r.remaining() / 8));
        for _ in 0..count {
            let key = LocStringKey(r.u32()?);
            let len = r.u32_usize()?;
            strings.push((key, r.bytes(len)?.to_vec()));
        }
        Ok(LocString { strref, strings })
    }
}
