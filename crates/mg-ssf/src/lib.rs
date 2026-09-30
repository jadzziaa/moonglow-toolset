//! SSF V1.0 soundsets: for each sound slot (a row of `soundsettype.2da`), a
//! sound resref and a talk-table string.

use mg_core::StrRef;
use mg_core::bin::{BinError, Reader, WriteLe, slice};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SsfError {
    #[error("not an SSF V1.0 file")]
    NotSsf,
    #[error("truncated or corrupt SSF: {0}")]
    Bin(#[from] BinError),
    #[error("slot {0}'s sound name is longer than 16 bytes")]
    NameTooLong(usize),
}

/// One slot: sound resref (raw bytes, up to 16) and text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsfEntry {
    pub sound: Vec<u8>,
    pub strref: StrRef,
}

/// A soundset: slots in order (the index is the slot's meaning).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ssf {
    pub entries: Vec<SsfEntry>,
}

const HEADER: usize = 40;

impl Ssf {
    pub fn read(data: &[u8]) -> Result<Ssf, SsfError> {
        let mut r = Reader::new(data);
        if r.bytes(8)? != b"SSF V1.0" {
            return Err(SsfError::NotSsf);
        }
        let count = r.u32_usize()?;
        let table = r.u32_usize()?;
        slice(data, table, count.saturating_mul(4))?;
        let mut t = Reader::at(data, table)?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let mut e = Reader::at(data, t.u32_usize()?)?;
            let sound = e.fixed_str(16)?.to_vec();
            entries.push(SsfEntry { sound, strref: StrRef(e.u32()?) });
        }
        Ok(Ssf { entries })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, SsfError> {
        let n = self.entries.len();
        let mut out = Vec::with_capacity(HEADER + n * 24);
        out.put_bytes(b"SSF V1.0");
        out.put_u32_usize(n);
        out.put_u32_usize(HEADER);
        out.resize(HEADER, 0);
        for i in 0..n {
            out.put_u32_usize(HEADER + n * 4 + i * 20);
        }
        for (i, e) in self.entries.iter().enumerate() {
            if e.sound.len() > 16 {
                return Err(SsfError::NameTooLong(i));
            }
            out.put_fixed(&e.sound, 16);
            out.put_u32(e.strref.0);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_truncation() {
        let s = Ssf {
            entries: vec![
                SsfEntry { sound: b"vs_nx_attack".to_vec(), strref: StrRef(12) },
                SsfEntry { sound: Vec::new(), strref: StrRef::NONE },
            ],
        };
        let bytes = s.to_bytes().unwrap();
        assert_eq!(Ssf::read(&bytes).unwrap(), s);
        for len in 0..bytes.len() {
            assert!(Ssf::read(&bytes[..len]).is_err());
        }
    }
}
