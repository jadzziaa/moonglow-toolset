//! GFF writer.
//!
//! Layout: structs are numbered depth-first in pre-order (the root is 0),
//! fields are numbered in the order they are written, labels are shared, and
//! the arrays follow the header in the order the format documents.

use std::collections::HashMap;

use mg_core::bin::WriteLe;
use thiserror::Error;

use crate::Gff;
use crate::value::{Label, Struct, Value};

/// A value the format cannot represent.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WriteError {
    #[error("resref field {label} is {len} bytes; GFF resrefs hold at most 255")]
    ResRefTooLong { label: String, len: usize },
    #[error("the file is larger than 4 GiB")]
    TooLarge,
}

#[derive(Default)]
struct Builder {
    structs: Vec<[u32; 3]>,
    fields: Vec<[u32; 3]>,
    labels: Vec<Label>,
    label_index: HashMap<Label, u32>,
    field_data: Vec<u8>,
    field_indices: Vec<u8>,
    list_indices: Vec<u8>,
}

pub(crate) fn write(gff: &Gff) -> Result<Vec<u8>, WriteError> {
    let mut b = Builder::default();
    b.add_struct(&gff.root)?;

    let header_len = 56usize;
    let sizes = [
        b.structs.len() * 12,
        b.fields.len() * 12,
        b.labels.len() * 16,
        b.field_data.len(),
        b.field_indices.len(),
        b.list_indices.len(),
    ];
    let total = header_len + sizes.iter().sum::<usize>();
    if u32::try_from(total).is_err() {
        return Err(WriteError::TooLarge);
    }

    let mut out = Vec::with_capacity(total);
    out.put_bytes(&gff.file_type);
    out.put_bytes(&gff.version);
    let counts = [
        b.structs.len(),
        b.fields.len(),
        b.labels.len(),
        b.field_data.len(),
        b.field_indices.len(),
        b.list_indices.len(),
    ];
    let mut offset = header_len;
    for (size, count) in sizes.iter().zip(counts) {
        out.put_u32_usize(offset);
        out.put_u32_usize(count);
        offset += size;
    }
    for s in &b.structs {
        s.iter().for_each(|&v| out.put_u32(v));
    }
    for f in &b.fields {
        f.iter().for_each(|&v| out.put_u32(v));
    }
    for l in &b.labels {
        out.put_fixed(l.as_bytes(), 16);
    }
    out.put_bytes(&b.field_data);
    out.put_bytes(&b.field_indices);
    out.put_bytes(&b.list_indices);
    debug_assert_eq!(out.len(), total);
    Ok(out)
}

impl Builder {
    /// Adds a struct and everything below it; returns its index.
    fn add_struct(&mut self, s: &Struct) -> Result<u32, WriteError> {
        let index = self.structs.len() as u32;
        self.structs.push([s.id, 0, 0]);
        let mut field_ids = Vec::with_capacity(s.fields.len());
        for f in &s.fields {
            field_ids.push(self.add_field(&f.label, &f.value)?);
        }
        let data = match field_ids.as_slice() {
            [] => u32::MAX,
            [only] => *only,
            many => {
                let offset = self.field_indices.len() as u32;
                many.iter().for_each(|&i| self.field_indices.put_u32(i));
                offset
            }
        };
        self.structs[index as usize] = [s.id, data, field_ids.len() as u32];
        Ok(index)
    }

    fn label(&mut self, label: &Label) -> u32 {
        *self.label_index.entry(*label).or_insert_with(|| {
            self.labels.push(*label);
            self.labels.len() as u32 - 1
        })
    }

    fn add_field(&mut self, label: &Label, value: &Value) -> Result<u32, WriteError> {
        let label_index = self.label(label);
        // Reserve the field's slot first so a struct's fields keep their order
        // even when children are added while writing the value.
        let index = self.fields.len() as u32;
        self.fields.push([0, 0, 0]);
        let data_offset = self.field_data.len() as u32;
        let fd = &mut self.field_data;
        let data = match value {
            Value::Byte(v) => *v as u32,
            Value::Char(v) => *v as u8 as u32,
            Value::Word(v) => *v as u32,
            Value::Short(v) => *v as u16 as u32,
            Value::Dword(v) => *v,
            Value::Int(v) => *v as u32,
            Value::Float(v) => v.to_bits(),
            Value::Dword64(v) => {
                fd.put_u64(*v);
                data_offset
            }
            Value::Int64(v) => {
                fd.put_i64(*v);
                data_offset
            }
            Value::Double(v) => {
                fd.put_f64(*v);
                data_offset
            }
            Value::String(v) | Value::Void(v) => {
                fd.put_u32_usize(v.len());
                fd.put_bytes(v);
                data_offset
            }
            Value::ResRef(v) => {
                let len = u8::try_from(v.len()).map_err(|_| WriteError::ResRefTooLong {
                    label: label.to_string_lossy(),
                    len: v.len(),
                })?;
                fd.put_u8(len);
                fd.put_bytes(v);
                data_offset
            }
            Value::LocString(ls) => {
                let body: usize = ls.strings.iter().map(|(_, s)| 8 + s.len()).sum();
                fd.put_u32_usize(8 + body);
                fd.put_u32(ls.strref.0);
                fd.put_u32_usize(ls.strings.len());
                for (key, s) in &ls.strings {
                    fd.put_u32(key.0);
                    fd.put_u32_usize(s.len());
                    fd.put_bytes(s);
                }
                data_offset
            }
            Value::Struct(s) => self.add_struct(s)?,
            Value::List(list) => {
                // Children first, so their indices are known.
                let ids = list.iter().map(|s| self.add_struct(s)).collect::<Result<Vec<_>, _>>()?;
                let offset = self.list_indices.len() as u32;
                self.list_indices.put_u32_usize(ids.len());
                ids.iter().for_each(|&i| self.list_indices.put_u32(i));
                offset
            }
        };
        self.fields[index as usize] = [value.field_type() as u32, label_index, data];
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{Gender, Language, LocString, StrRef};
    use proptest::prelude::*;

    use super::*;
    use crate::value::Field;

    fn sample() -> Gff {
        let mut g = Gff::new(*b"UTC ");
        let r = &mut g.root;
        r.set("Byte", Value::Byte(200));
        r.set("Char", Value::Char(-5));
        r.set("Word", Value::Word(60000));
        r.set("Short", Value::Short(-30000));
        r.set("Dword", Value::Dword(4_000_000_000));
        r.set("Int", Value::Int(-2_000_000_000));
        r.set("Dword64", Value::Dword64(u64::MAX - 1));
        r.set("Int64", Value::Int64(i64::MIN + 1));
        r.set("Float", Value::Float(1.25));
        r.set("Double", Value::Double(-2.5e100));
        r.set("String", Value::String(b"Hello \xe9".to_vec()));
        r.set("ResRef", Value::ResRef(b"nw_chicken".to_vec()));
        let mut ls = LocString::from_strref(StrRef(12345));
        ls.set(Language::ENGLISH, Gender::Male, "Chicken");
        ls.set(Language::GERMAN, Gender::Female, "Huhn");
        r.set("LocString", Value::LocString(ls));
        r.set("Void", Value::Void(vec![0, 1, 2, 255]));
        let mut child = Struct::new(7);
        child.set("Inner", Value::Int(1));
        r.set("Struct", Value::Struct(child));
        let items = (0..3)
            .map(|i| {
                let mut s = Struct::new(i);
                s.set("Idx", Value::Byte(i as u8));
                s.set("Empty", Value::List(vec![]));
                s
            })
            .collect();
        r.set("List", Value::List(items));
        r.set("EmptyStruct", Value::Struct(Struct::new(3)));
        g
    }

    #[test]
    fn round_trips_every_field_type() {
        let g = sample();
        let bytes = g.to_bytes().unwrap();
        assert_eq!(Gff::read(&bytes).unwrap(), g);
    }

    #[test]
    fn writing_is_deterministic_and_idempotent() {
        let bytes = sample().to_bytes().unwrap();
        assert_eq!(Gff::read(&bytes).unwrap().to_bytes().unwrap(), bytes);
    }

    #[test]
    fn duplicate_labels_survive() {
        let mut g = Gff::new(*b"GFF ");
        for v in [1, 2] {
            g.root.fields.push(Field { label: Label::new("Dup").unwrap(), value: Value::Int(v) });
        }
        let back = Gff::read(&g.to_bytes().unwrap()).unwrap();
        assert_eq!(back, g);
    }

    #[test]
    fn rejects_oversized_resref() {
        let mut g = Gff::new(*b"GFF ");
        g.root.set("R", Value::ResRef(vec![b'a'; 256]));
        assert!(matches!(g.to_bytes(), Err(WriteError::ResRefTooLong { .. })));
    }

    #[test]
    fn truncated_input_is_an_error_not_a_panic() {
        let bytes = sample().to_bytes().unwrap();
        for len in 0..bytes.len() {
            let _ = Gff::read(&bytes[..len]);
        }
    }

    fn arb_value() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            any::<u8>().prop_map(Value::Byte),
            any::<i8>().prop_map(Value::Char),
            any::<u16>().prop_map(Value::Word),
            any::<i16>().prop_map(Value::Short),
            any::<u32>().prop_map(Value::Dword),
            any::<i32>().prop_map(Value::Int),
            any::<u64>().prop_map(Value::Dword64),
            any::<i64>().prop_map(Value::Int64),
            any::<u32>().prop_map(|b| Value::Float(f32::from_bits(b))),
            any::<u64>().prop_map(|b| Value::Double(f64::from_bits(b))),
            proptest::collection::vec(any::<u8>(), 0..20).prop_map(Value::String),
            proptest::collection::vec(any::<u8>(), 0..17).prop_map(Value::ResRef),
            proptest::collection::vec(any::<u8>(), 0..20).prop_map(Value::Void),
            (any::<u32>(), proptest::collection::vec((0u32..12, "[a-z]{0,8}"), 0..3)).prop_map(
                |(sr, strings)| {
                    Value::LocString(LocString {
                        strref: StrRef(sr),
                        strings: strings
                            .into_iter()
                            .map(|(k, s)| (mg_core::LocStringKey(k), s.into_bytes()))
                            .collect(),
                    })
                }
            ),
        ];
        leaf.prop_recursive(4, 64, 6, |inner| {
            let st = (any::<u32>(), proptest::collection::vec(("[A-Za-z]{1,16}", inner), 0..6))
                .prop_map(|(id, fields)| Struct {
                    id,
                    fields: fields
                        .into_iter()
                        .map(|(l, value)| Field { label: Label::new(&l).unwrap(), value })
                        .collect(),
                });
            prop_oneof![
                st.clone().prop_map(Value::Struct),
                proptest::collection::vec(st, 0..4).prop_map(Value::List),
            ]
        })
    }

    proptest! {
        #[test]
        fn arbitrary_trees_round_trip(fields in proptest::collection::vec(("[A-Za-z_]{1,16}", arb_value()), 0..8)) {
            let mut g = Gff::new(*b"GFF ");
            g.root.fields = fields.into_iter().map(|(l, value)| Field { label: Label::new(&l).unwrap(), value }).collect();
            let bytes = g.to_bytes().unwrap();
            prop_assert_eq!(Gff::read(&bytes).unwrap(), g);
        }
    }
}
