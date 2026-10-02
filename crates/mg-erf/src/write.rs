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

    /// The resources that would start 2 GiB or further into the archive,
    /// which the game can't read (`mg_resman::ERF_READ_LIMIT`).
    pub fn past_read_limit(&self) -> Vec<(ResRef, ResType)> {
        let header = Header { description: self.description.clone(), ..Default::default() };
        let entries: Vec<(ResRef, ResType, u64)> =
            self.entries.iter().map(|(r, t, d)| (*r, *t, d.len() as u64)).collect();
        past_read_limit(&header, &entries)
    }

    /// Writes the archive.
    pub fn write_to(&self, w: &mut impl Write) -> Result<(), WriteError> {
        let header = Header {
            file_type: self.file_type,
            build_year: self.build_year,
            build_day: self.build_day,
            description: self.description.clone(),
        };
        let entries: Vec<(ResRef, ResType, u64)> =
            self.entries.iter().map(|(r, t, d)| (*r, *t, d.len() as u64)).collect();
        write_streamed(w, &header, &entries, |i| Ok(Cow::Borrowed(&self.entries[i].2[..])))
    }

    /// The archive as bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, WriteError> {
        let mut v = Vec::new();
        self.write_to(&mut v)?;
        Ok(v)
    }
}

/// An archive's header, but for its entries.
#[derive(Debug, Clone, Default)]
pub struct Header {
    pub file_type: [u8; 4],
    /// Years since 1900.
    pub build_year: u32,
    /// Day of the year (0-based).
    pub build_day: u32,
    pub description: Description,
}

fn data_start(header: &Header, entries: usize) -> u64 {
    let strings: usize = header.description.strings.iter().map(|(_, s)| 8 + s.len()).sum();
    (HEADER_SIZE + strings + entries * (24 + 8)) as u64
}

/// The entries (name, type, size) that would start 2 GiB or further into
/// an archive, which the game can't read (`mg_resman::ERF_READ_LIMIT`).
pub fn past_read_limit(
    header: &Header,
    entries: &[(ResRef, ResType, u64)],
) -> Vec<(ResRef, ResType)> {
    let mut at = data_start(header, entries.len());
    let mut out = Vec::new();
    for (resref, restype, size) in entries {
        if at >= 1 << 31 {
            out.push((*resref, *restype));
        }
        at += size;
    }
    out
}

/// Writes a `V1.0` archive whose resources are read one at a time: `data`
/// gives entry `i`'s bytes, which must be the size `entries` says. For
/// archives too big to hold in memory (a hak built from files and another
/// archive).
pub fn write_streamed<'d>(
    w: &mut impl Write,
    header: &Header,
    entries: &[(ResRef, ResType, u64)],
    mut data: impl FnMut(usize) -> io::Result<Cow<'d, [u8]>>,
) -> Result<(), WriteError> {
    let mut names = HashSet::new();
    for (resref, restype, _) in entries {
        if !names.insert((*resref, *restype)) {
            return Err(WriteError::Duplicate(format!("{resref}.{restype}")));
        }
    }
    let n = entries.len();
    let strings_size: usize = header.description.strings.iter().map(|(_, s)| 8 + s.len()).sum();
    let keys_offset = HEADER_SIZE + strings_size;
    let resources_offset = keys_offset + n * 24;
    let data_offset = data_start(header, n);
    let total = data_offset + entries.iter().map(|e| e.2).sum::<u64>();
    if u32::try_from(total).is_err() {
        return Err(WriteError::TooLarge);
    }

    let mut head = Vec::with_capacity(data_offset as usize);
    head.put_bytes(&header.file_type);
    head.put_bytes(b"V1.0");
    head.put_u32_usize(header.description.strings.len());
    head.put_u32_usize(strings_size);
    head.put_u32_usize(n);
    head.put_u32_usize(HEADER_SIZE);
    head.put_u32_usize(keys_offset);
    head.put_u32_usize(resources_offset);
    head.put_u32(header.build_year);
    head.put_u32(header.build_day);
    head.put_u32(header.description.strref.0);
    head.resize(HEADER_SIZE, 0);
    for (lang, s) in &header.description.strings {
        head.put_u32(*lang);
        head.put_u32_usize(s.len());
        head.put_bytes(s);
    }
    for (i, (resref, restype, _)) in entries.iter().enumerate() {
        head.put_fixed(resref.to_lowercase().as_bytes(), 16);
        head.put_u32_usize(i);
        head.put_u16(restype.0);
        head.put_u16(0);
    }
    let mut offset = data_offset;
    for (_, _, size) in entries {
        // Fits: the total does.
        head.put_u32(offset as u32);
        head.put_u32(*size as u32);
        offset += size;
    }
    debug_assert_eq!(head.len() as u64, data_offset);
    w.write_all(&head)?;
    for (i, (resref, restype, size)) in entries.iter().enumerate() {
        let bytes = data(i)?;
        if bytes.len() as u64 != *size {
            return Err(WriteError::Io(io::Error::other(format!(
                "{resref}.{restype} changed size while the archive was written"
            ))));
        }
        w.write_all(&bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use mg_core::StrRef;

    use super::*;
    use crate::{Erf, ErfVersion};

    #[test]
    fn resources_past_2_gib_are_named() {
        // Borrowed zeros: nothing is allocated.
        let big = vec![0u8; 1 << 20];
        let mut w = ErfWriter::new(*b"HAK ");
        let name = |i: usize| ResRef::from_str(&format!("r{i}")).unwrap();
        for i in 0..2048 {
            w.add(name(i), ResType::TGA, &big[..]).unwrap();
        }
        // The last starts just under the mark (the header's size under it)
        // and ends past it.
        assert!(w.past_read_limit().is_empty());
        w.add(name(2048), ResType::TGA, &big[..1]).unwrap();
        assert_eq!(w.past_read_limit(), [(name(2048), ResType::TGA)]);
    }

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
