//! TLK V3.0 talk tables: `dialog.tlk` (and `dialogf.tlk`, the feminine
//! variants some languages ship) and modules' custom talk tables.
//!
//! [`Tlk`] holds every entry (text as bytes in the language's codepage) and
//! writes the layout the game ships: header, entry table, strings in entry
//! order.

use mg_core::bin::{BinError, Reader, WriteLe, slice};
use mg_core::{Language, ResRef, StrRef};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TlkError {
    #[error("not a TLK V3.0 file")]
    NotTlk,
    #[error("truncated or corrupt TLK: {0}")]
    Bin(#[from] BinError),
    #[error("the table is larger than 4 GiB")]
    TooLarge,
}

/// Entry flag: the entry has text.
pub const FLAG_TEXT: u32 = 0x1;
/// Entry flag: the entry has a sound resref.
pub const FLAG_SOUND: u32 = 0x2;
/// Entry flag: the entry has a sound length.
pub const FLAG_SOUND_LENGTH: u32 = 0x4;

/// One talk-table row.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TlkEntry {
    pub flags: u32,
    pub text: Vec<u8>,
    /// Up to 16 bytes; kept raw because shipped tables contain junk here.
    pub sound: Vec<u8>,
    /// Unused by the game; kept for round trips.
    pub volume_variance: u32,
    /// Unused by the game; kept for round trips.
    pub pitch_variance: u32,
    pub sound_length: f32,
}

impl TlkEntry {
    /// An entry with text only.
    pub fn text(text: impl Into<Vec<u8>>) -> TlkEntry {
        TlkEntry { flags: FLAG_TEXT, text: text.into(), ..Default::default() }
    }

    /// The sound resref, if the entry has a valid one.
    pub fn sound_resref(&self) -> Option<ResRef> {
        (self.flags & FLAG_SOUND != 0 && !self.sound.is_empty())
            .then(|| ResRef::from_bytes(&self.sound).ok())
            .flatten()
    }
}

/// A talk table.
#[derive(Debug, Clone, PartialEq)]
pub struct Tlk {
    pub language: Language,
    pub entries: Vec<TlkEntry>,
}

impl Tlk {
    pub fn new(language: Language) -> Tlk {
        Tlk { language, entries: Vec::new() }
    }

    pub fn read(data: &[u8]) -> Result<Tlk, TlkError> {
        let mut r = Reader::new(data);
        if r.bytes(8)? != b"TLK V3.0" {
            return Err(TlkError::NotTlk);
        }
        let language = Language(r.u32()?);
        let count = r.u32_usize()?;
        let strings_offset = r.u32_usize()?;
        slice(data, 20, count.saturating_mul(40))?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let flags = r.u32()?;
            let sound = r.fixed_str(16)?.to_vec();
            let volume_variance = r.u32()?;
            let pitch_variance = r.u32()?;
            let offset = r.u32_usize()?;
            let size = r.u32_usize()?;
            let sound_length = r.f32()?;
            // The game reads text only when the flag is set; some shipped
            // entries have stale offsets otherwise.
            let text = if size > 0 {
                slice(data, strings_offset.saturating_add(offset), size)?.to_vec()
            } else {
                Vec::new()
            };
            entries.push(TlkEntry {
                flags,
                text,
                sound,
                volume_variance,
                pitch_variance,
                sound_length,
            });
        }
        Ok(Tlk { language, entries })
    }

    /// The entry for a row (ignores the custom-table flag bit; see
    /// [`StrRef::index`]).
    pub fn get(&self, strref: StrRef) -> Option<&TlkEntry> {
        if strref.is_none() {
            return None;
        }
        self.entries.get(strref.index() as usize)
    }

    /// The text of a row, decoded with the table's codepage. Empty when the
    /// row has no text.
    pub fn text(&self, strref: StrRef) -> Option<String> {
        let e = self.get(strref)?;
        (e.flags & FLAG_TEXT != 0).then(|| self.language.codepage().decode(&e.text).into_owned())
    }

    /// Serializes: header, entry table, then the strings in entry order.
    pub fn to_bytes(&self) -> Result<Vec<u8>, TlkError> {
        let n = self.entries.len();
        let strings_offset = 20 + n * 40;
        let total = strings_offset + self.entries.iter().map(|e| e.text.len()).sum::<usize>();
        if u32::try_from(total).is_err() {
            return Err(TlkError::TooLarge);
        }
        let mut out = Vec::with_capacity(total);
        out.put_bytes(b"TLK V3.0");
        out.put_u32(self.language.0);
        out.put_u32_usize(n);
        out.put_u32_usize(strings_offset);
        let mut offset = 0usize;
        for e in &self.entries {
            out.put_u32(e.flags);
            out.put_fixed(&e.sound[..e.sound.len().min(16)], 16);
            out.put_u32(e.volume_variance);
            out.put_u32(e.pitch_variance);
            // Shipped tables disagree on what empty entries point at (0 or the
            // running offset); the game never reads it.
            out.put_u32_usize(if e.text.is_empty() { 0 } else { offset });
            out.put_u32_usize(e.text.len());
            out.put_f32(e.sound_length);
            offset += e.text.len();
        }
        for e in &self.entries {
            out.put_bytes(&e.text);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut t = Tlk::new(Language::POLISH);
        t.entries.push(TlkEntry::text("Bad Strref"));
        t.entries.push(TlkEntry::default());
        t.entries.push(TlkEntry {
            flags: FLAG_TEXT | FLAG_SOUND | FLAG_SOUND_LENGTH,
            text: vec![0xa3, b'o', b'd', 0x9f],
            sound: b"vs_hello".to_vec(),
            sound_length: 1.5,
            ..Default::default()
        });
        let bytes = t.to_bytes().unwrap();
        let back = Tlk::read(&bytes).unwrap();
        assert_eq!(back, t);
        assert_eq!(back.text(StrRef(2)).as_deref(), Some("Łodź"));
        assert_eq!(back.text(StrRef::custom(0)).as_deref(), Some("Bad Strref"));
        assert_eq!(back.text(StrRef(1)), None);
        assert_eq!(back.get(StrRef(2)).unwrap().sound_resref().unwrap().as_str(), Some("vs_hello"));
        assert!(back.get(StrRef::NONE).is_none());
    }

    #[test]
    fn truncated_input_is_an_error_not_a_panic() {
        let mut t = Tlk::new(Language::ENGLISH);
        t.entries.push(TlkEntry::text("abc"));
        let bytes = t.to_bytes().unwrap();
        for len in 0..bytes.len() {
            assert!(Tlk::read(&bytes[..len]).is_err());
        }
    }
}
