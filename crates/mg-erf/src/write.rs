//! ERF writer (`V1.0`).

use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{self, Write};

use mg_core::bin::WriteLe;
use mg_core::{ResRef, ResType};
use thiserror::Error;

use crate::Description;

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("{0} is added twice")]
    Duplicate(String),
    #[error("the archive would be larger than 4 GiB")]
    TooLarge,
    #[error(transparent)]
    Io(#[from] io::Error),
}

const HEADER_SIZE: usize = 160;

/// Builds a `V1.0` archive. Entry names are written lowercase, in the order
/// they were added.
#[derive(Debug, Clone)]
pub struct ErfWriter<'a> {
    pub file_type: [u8; 4],
    /// Years since 1900.
    pub build_year: u32,
    /// Day of the year (0-based).
    pub build_day: u32,
    pub description: Description,
    entries: Vec<(ResRef, ResType, Cow<'a, [u8]>)>,
    names: HashSet<(ResRef, ResType)>,
}

impl<'a> ErfWriter<'a> {
    /// An empty archive of the given type (`b"MOD "`, `b"HAK "`, `b"ERF "`).
    /// The build date defaults to 0; set it for anything users will see.
    pub fn new(file_type: [u8; 4]) -> ErfWriter<'a> {
        ErfWriter {
            file_type,
            build_year: 0,
            build_day: 0,
            description: Description::default(),
            entries: Vec::new(),
            names: HashSet::new(),
        }
    }

    /// Adds a resource. Names are case-insensitive; adding one twice is an
    /// error.
    pub fn add(
        &mut self,
        resref: ResRef,
        restype: ResType,
        data: impl Into<Cow<'a, [u8]>>,
    ) -> Result<(), WriteError> {
        if !self.names.insert((resref, restype)) {
            return Err(WriteError::Duplicate(format!("{resref}.{restype}")));
        }
        self.entries.push((resref, restype, data.into()));
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Writes the archive.
    pub fn write_to(&self, w: &mut impl Write) -> Result<(), WriteError> {
        let n = self.entries.len();
        let strings_size: usize = self.description.strings.iter().map(|(_, s)| 8 + s.len()).sum();
        let keys_offset = HEADER_SIZE + strings_size;
        let resources_offset = keys_offset + n * 24;
        let data_offset = resources_offset + n * 8;
        let total = data_offset + self.entries.iter().map(|(_, _, d)| d.len()).sum::<usize>();
        if u32::try_from(total).is_err() {
            return Err(WriteError::TooLarge);
        }

        let mut head = Vec::with_capacity(data_offset);
        head.put_bytes(&self.file_type);
        head.put_bytes(b"V1.0");
        head.put_u32_usize(self.description.strings.len());
        head.put_u32_usize(strings_size);
        head.put_u32_usize(n);
        head.put_u32_usize(HEADER_SIZE);
        head.put_u32_usize(keys_offset);
        head.put_u32_usize(resources_offset);
        head.put_u32(self.build_year);
        head.put_u32(self.build_day);
        head.put_u32(self.description.strref.0);
        head.resize(HEADER_SIZE, 0);
        for (lang, s) in &self.description.strings {
            head.put_u32(*lang);
            head.put_u32_usize(s.len());
            head.put_bytes(s);
        }
        for (i, (resref, restype, _)) in self.entries.iter().enumerate() {
            head.put_fixed(resref.to_lowercase().as_bytes(), 16);
            head.put_u32_usize(i);
            head.put_u16(restype.0);
            head.put_u16(0);
        }
        let mut offset = data_offset;
        for (_, _, data) in &self.entries {
            head.put_u32_usize(offset);
            head.put_u32_usize(data.len());
            offset += data.len();
        }
        debug_assert_eq!(head.len(), data_offset);
        w.write_all(&head)?;
        for (_, _, data) in &self.entries {
            w.write_all(data)?;
        }
        Ok(())
    }

    /// The archive as bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, WriteError> {
        let mut v = Vec::new();
        self.write_to(&mut v)?;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use mg_core::StrRef;

    use super::*;
    use crate::{Erf, ErfVersion};

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    #[test]
    fn round_trip() {
        let mut w = ErfWriter::new(*b"HAK ");
        w.build_year = 125;
        w.build_day = 42;
        w.description = Description { strref: StrRef(7), strings: vec![(0, b"My hak".to_vec())] };
        w.add(rr("Module"), ResType::IFO, &b"ifo data"[..]).unwrap();
        w.add(rr("x"), ResType::NSS, vec![1, 2, 3]).unwrap();
        w.add(rr("empty"), ResType::TXT, Vec::new()).unwrap();
        assert!(matches!(
            w.add(rr("MODULE"), ResType::IFO, Vec::new()),
            Err(WriteError::Duplicate(_))
        ));
        let bytes = w.to_bytes().unwrap();

        let erf = Erf::read(&bytes).unwrap();
        assert_eq!(&erf.file_type, b"HAK ");
        assert_eq!(erf.version, ErfVersion::V1);
        assert_eq!((erf.build_year, erf.build_day), (125, 42));
        assert_eq!(erf.description, w.description);
        assert_eq!(erf.entries.len(), 3);
        assert_eq!(erf.entries[0].filename(), "module.ifo");
        assert_eq!(erf.get(&rr("MODULE"), ResType::IFO).unwrap().unwrap().as_ref(), b"ifo data");
        assert_eq!(erf.get(&rr("x"), ResType::NSS).unwrap().unwrap().as_ref(), &[1, 2, 3]);
        assert!(erf.get(&rr("x"), ResType::NCS).is_none());
    }

    #[test]
    fn truncated_input_is_an_error_not_a_panic() {
        let mut w = ErfWriter::new(*b"ERF ");
        w.add(rr("a"), ResType::TXT, &b"abc"[..]).unwrap();
        let bytes = w.to_bytes().unwrap();
        for len in 0..bytes.len() {
            assert!(Erf::read(&bytes[..len]).is_err(), "len {len}");
        }
    }
}
