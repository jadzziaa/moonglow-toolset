//! ERF reader.

use std::borrow::Cow;

use mg_core::bin::{BinError, Reader, slice};
use mg_core::{ResRef, ResType, StrRef};
use thiserror::Error;

use crate::compressedbuf::{self, CompressedBufError, MAGIC_XRES};
use crate::{Compression, Description, Entry, Erf, ErfVersion};

/// Why an archive or entry could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReadError {
    #[error("unsupported ERF version {0:?}")]
    BadVersion(String),
    #[error("truncated or corrupt ERF: {0}")]
    Bin(#[from] BinError),
    #[error("entry {index} has an invalid resref: {message}")]
    BadResRef { index: usize, message: String },
    #[error("entry {index} uses unknown compression type {kind}")]
    BadCompression { index: usize, kind: u32 },
    #[error("compressed entry: {0}")]
    CompressedBuf(#[from] CompressedBufError),
}

pub(crate) fn read(data: &[u8]) -> Result<Erf<'_>, ReadError> {
    let mut r = Reader::new(data);
    let file_type: [u8; 4] = r.array()?;
    let version = match &r.array::<4>()? {
        b"V1.0" => ErfVersion::V1,
        b"E1.0" => ErfVersion::E1,
        other => return Err(ReadError::BadVersion(String::from_utf8_lossy(other).into_owned())),
    };
    let language_count = r.u32_usize()?;
    let _localized_size = r.u32()?; // wrong in some shipped archives; not needed
    let entry_count = r.u32_usize()?;
    let offset_to_strings = r.u32_usize()?;
    let offset_to_keys = r.u32_usize()?;
    let offset_to_resources = r.u32_usize()?;
    let build_year = r.u32()?;
    let build_day = r.u32()?;
    let strref = StrRef(r.u32()?);

    let mut description = Description { strref, strings: Vec::new() };
    let mut sr = Reader::at(data, offset_to_strings)?;
    for _ in 0..language_count {
        let lang = sr.u32()?;
        let len = sr.u32_usize()?;
        description.strings.push((lang, sr.bytes(len)?.to_vec()));
    }

    let (key_size, res_size) = match version {
        ErfVersion::V1 => (24, 8),
        ErfVersion::E1 => (44, 16),
    };
    // Validate both tables up front so a bad count cannot trigger a huge
    // allocation.
    slice(data, offset_to_keys, entry_count.saturating_mul(key_size))?;
    slice(data, offset_to_resources, entry_count.saturating_mul(res_size))?;

    let mut keys = Reader::at(data, offset_to_keys)?;
    let mut res = Reader::at(data, offset_to_resources)?;
    let mut entries = Vec::with_capacity(entry_count);
    for index in 0..entry_count {
        let name = keys.fixed_str(16)?;
        let _res_id = keys.u32()?;
        let restype = ResType(keys.u16()?);
        let _unused = keys.u16()?;
        if version == ErfVersion::E1 {
            keys.skip(20)?; // SHA-1 of the uncompressed data
        }
        let offset = res.u32()?;
        let disk_size = res.u32()?;
        let (compression, size) = match version {
            ErfVersion::V1 => (Compression::None, disk_size),
            ErfVersion::E1 => {
                let kind = res.u32()?;
                let size = res.u32()?;
                let c = match kind {
                    0 => Compression::None,
                    1 => Compression::CompressedBuf,
                    kind => return Err(ReadError::BadCompression { index, kind }),
                };
                (c, size)
            }
        };
        // Unused slots.
        if restype == ResType::INVALID {
            continue;
        }
        let resref = ResRef::from_bytes(name)
            .map_err(|e| ReadError::BadResRef { index, message: e.to_string() })?;
        slice(data, offset as usize, disk_size as usize)?;
        entries.push(Entry { resref, restype, offset, disk_size, size, compression });
    }

    Ok(Erf { data, file_type, version, build_year, build_day, description, entries })
}

/// The bytes of an entry of the archive `data`, decompressed if needed.
pub fn entry_data<'a>(data: &'a [u8], e: &Entry) -> Result<Cow<'a, [u8]>, ReadError> {
    let raw = slice(data, e.offset as usize, e.disk_size as usize)?;
    Ok(match e.compression {
        Compression::None => Cow::Borrowed(raw),
        Compression::CompressedBuf => Cow::Owned(compressedbuf::decompress(raw, MAGIC_XRES)?),
    })
}
