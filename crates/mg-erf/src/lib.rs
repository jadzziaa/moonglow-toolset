//! ERF archives: modules (`.mod`, `.nwm`), haks (`.hak`), exported
//! resources (`.erf`) and saved games (`.sav`).
//!
//! [`Erf::read`] indexes an archive held in memory (usually a memory map)
//! without copying resource data; [`ErfWriter`] writes the classic `V1.0`
//! format every tool and game version reads. EE's `E1.0` variant (zstd
//! compressed entries with SHA-1s) can be read.

pub mod compressedbuf;
mod read;
mod write;

use std::borrow::Cow;

use mg_core::{ResRef, ResType, StrRef};

pub use read::{ReadError, entry_data};
pub use write::{ErfWriter, Header, WriteError, past_read_limit, write_streamed};

/// The archive layout version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErfVersion {
    /// `V1.0`: the original format.
    V1,
    /// `E1.0`: EE's format with optional per-entry compression and SHA-1s.
    E1,
}

/// How an entry's bytes are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    /// An NWCompressedBuf with the `XRES` magic (see [`compressedbuf`]).
    CompressedBuf,
}

/// One resource in an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub resref: ResRef,
    pub restype: ResType,
    /// Byte offset of the stored data in the archive.
    pub offset: u32,
    /// Stored size.
    pub disk_size: u32,
    /// Size after decompression (equals `disk_size` when uncompressed).
    pub size: u32,
    pub compression: Compression,
}

impl Entry {
    /// `resref.ext`, lowercased, as the entry would be named on disk.
    pub fn filename(&self) -> String {
        format!("{}.{}", self.resref.to_lowercase(), self.restype)
    }
}

/// A localized description stored in the archive header (language id and
/// raw bytes in the game codepage).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Description {
    pub strref: StrRef,
    pub strings: Vec<(u32, Vec<u8>)>,
}

/// An indexed archive over borrowed bytes.
#[derive(Debug, Clone)]
pub struct Erf<'a> {
    data: &'a [u8],
    /// `b"MOD "`, `b"HAK "`, `b"ERF "`, `b"NWM "`, `b"SAV "`, ...
    pub file_type: [u8; 4],
    pub version: ErfVersion,
    /// Years since 1900.
    pub build_year: u32,
    /// Day of the year (0-based).
    pub build_day: u32,
    pub description: Description,
    /// Entries in archive order. Duplicates (seen in some shipped archives)
    /// are kept; lookups return the first.
    pub entries: Vec<Entry>,
}

impl<'a> Erf<'a> {
    /// Indexes an archive.
    pub fn read(data: &'a [u8]) -> Result<Erf<'a>, ReadError> {
        read::read(data)
    }

    /// The first entry with this name and type.
    pub fn find(&self, resref: &ResRef, restype: ResType) -> Option<&Entry> {
        self.entries.iter().find(|e| e.restype == restype && e.resref == *resref)
    }

    /// The stored bytes of an entry, decompressed if needed.
    pub fn data(&self, entry: &Entry) -> Result<Cow<'a, [u8]>, ReadError> {
        read::entry_data(self.data, entry)
    }

    /// Looks up and returns a resource's bytes.
    pub fn get(
        &self,
        resref: &ResRef,
        restype: ResType,
    ) -> Option<Result<Cow<'a, [u8]>, ReadError>> {
        self.find(resref, restype).map(|e| self.data(e))
    }
}
