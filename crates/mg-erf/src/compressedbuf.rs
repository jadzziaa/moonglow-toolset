//! NWCompressedBuf: EE's wrapper for compressed resources (ERF/BIF `E1`
//! entries use the `XRES` magic, NWSync uses `NSYC`).
//!
//! Layout: magic, header version (3), algorithm (0 none, 1 zlib, 2 zstd),
//! uncompressed size, then per algorithm: raw bytes; or a version word (1)
//! and a zlib stream; or a version word (1), a dictionary id (0) and a zstd
//! frame.

use std::io::Read;

use mg_core::bin::{BinError, Reader};
use thiserror::Error;

/// `XRES`, the magic of compressed ERF/BIF entries.
pub const MAGIC_XRES: [u8; 4] = *b"XRES";
/// `NSYC`, the magic of NWSync resources.
pub const MAGIC_NSYC: [u8; 4] = *b"NSYC";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CompressedBufError {
    #[error("truncated compressed buffer: {0}")]
    Bin(#[from] BinError),
    #[error("expected magic {expected:?}, found {found:?}")]
    BadMagic { expected: String, found: String },
    #[error("unsupported compressed buffer header version {0}")]
    BadVersion(u32),
    #[error("unsupported compression algorithm {0} (zstd and uncompressed are supported)")]
    UnsupportedAlgorithm(u32),
    #[error("zstd dictionaries are not supported")]
    Dictionary,
    #[error("zstd data is corrupt: {0}")]
    Zstd(String),
    #[error("decompressed {actual} bytes, header says {expected}")]
    SizeMismatch { expected: usize, actual: usize },
}

/// Decompresses a buffer, checking its magic.
pub fn decompress(data: &[u8], magic: [u8; 4]) -> Result<Vec<u8>, CompressedBufError> {
    let mut r = Reader::new(data);
    let found: [u8; 4] = r.array()?;
    if found != magic {
        return Err(CompressedBufError::BadMagic {
            expected: String::from_utf8_lossy(&magic).into_owned(),
            found: String::from_utf8_lossy(&found).into_owned(),
        });
    }
    let version = r.u32()?;
    if version != 3 {
        return Err(CompressedBufError::BadVersion(version));
    }
    let algorithm = r.u32()?;
    let size = r.u32_usize()?;
    if size == 0 {
        return Ok(Vec::new());
    }
    let out = match algorithm {
        0 => r.bytes(size)?.to_vec(),
        2 => {
            let v = r.u32()?;
            if v != 1 {
                return Err(CompressedBufError::BadVersion(v));
            }
            if r.u32()? != 0 {
                return Err(CompressedBufError::Dictionary);
            }
            let frame = r.bytes(r.remaining())?;
            let dec = ruzstd::decoding::StreamingDecoder::new(frame)
                .map_err(|e| CompressedBufError::Zstd(e.to_string()))?;
            // (The header's size is the file's word: room made for no more
            // than a frame of this length could plausibly hold, and no
            // more read than it says, so a crafted size or a frame that
            // expands without end runs out of nothing.)
            let mut out = Vec::with_capacity(size.min(frame.len().saturating_mul(64)));
            dec.take(size as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|e| CompressedBufError::Zstd(e.to_string()))?;
            out
        }
        other => return Err(CompressedBufError::UnsupportedAlgorithm(other)),
    };
    if out.len() != size {
        return Err(CompressedBufError::SizeMismatch { expected: size, actual: out.len() });
    }
    Ok(out)
}

/// Wraps bytes uncompressed (algorithm 0).
pub fn store(data: &[u8], magic: [u8; 4]) -> Vec<u8> {
    use mg_core::bin::WriteLe;
    let mut out = Vec::with_capacity(16 + data.len());
    out.put_bytes(&magic);
    out.put_u32(3);
    out.put_u32(0);
    out.put_u32_usize(data.len());
    out.put_bytes(data);
    out
}

/// The zstd levels data may be compressed at: the reference library's,
/// 1 (fastest) to 22 (smallest), its own default 3 (`nwn_nwsync_write`'s).
pub const LEVELS: std::ops::RangeInclusive<i32> = 1..=22;
/// The level [`compress`] uses.
pub const DEFAULT_LEVEL: i32 = 3;

/// Compresses bytes with zstd (algorithm 2), as NWSync's data files are,
/// at [`DEFAULT_LEVEL`].
pub fn compress(data: &[u8], magic: [u8; 4]) -> Result<Vec<u8>, String> {
    compress_at(data, magic, DEFAULT_LEVEL)
}

/// [`compress`] at a zstd level of [`LEVELS`] (one outside them is taken
/// to the nearest), with the reference library (zstd-sys's own sources,
/// built in: no system library is needed). The frame declares its content
/// size, which one-shot decoders (the reference library's
/// `ZSTD_decompress`, which neverwinter.nim uses) need.
pub fn compress_at(data: &[u8], magic: [u8; 4], level: i32) -> Result<Vec<u8>, String> {
    use mg_core::bin::WriteLe;
    let level = level.clamp(*LEVELS.start(), *LEVELS.end());
    let frame = zstd::bulk::compress(data, level).map_err(|e| format!("zstd: {e}"))?;
    let mut out = Vec::with_capacity(24 + frame.len());
    out.put_bytes(&magic);
    out.put_u32(3);
    out.put_u32(2);
    out.put_u32_usize(data.len());
    out.put_u32(1);
    out.put_u32(0);
    out.put_bytes(&frame);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cut buffer, a size header out of all proportion and a frame that
    /// says less than it holds are errors, not a panic or a huge
    /// allocation.
    #[test]
    fn bad_buffers_are_errors() {
        let data: Vec<u8> = (0..4096u32).map(|i| (i % 7) as u8).collect();
        let packed = compress(&data, MAGIC_XRES).unwrap();
        for n in [0, 3, 8, 15, 20, 27, packed.len() - 1] {
            assert!(decompress(&packed[..n], MAGIC_XRES).is_err(), "cut at {n}");
        }
        // The header says 4 GiB: refused once the frame ends.
        let mut huge = packed.clone();
        huge[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decompress(&huge, MAGIC_XRES),
            Err(CompressedBufError::SizeMismatch { actual: 4096, .. })
        ));
        // The header says 100 bytes of a frame that holds 4096: no more
        // than 101 read.
        let mut small = packed.clone();
        small[12..16].copy_from_slice(&100u32.to_le_bytes());
        assert!(matches!(
            decompress(&small, MAGIC_XRES),
            Err(CompressedBufError::SizeMismatch { expected: 100, actual: 101 })
        ));
    }

    #[test]
    fn compressed_round_trip() {
        let data: Vec<u8> = (0..20_000u32).flat_map(|i| (i % 251).to_le_bytes()).collect();
        let buf = compress(&data, MAGIC_NSYC).unwrap();
        assert!(buf.len() < data.len() / 2, "{}", buf.len());
        assert_eq!(decompress(&buf, MAGIC_NSYC).unwrap(), data);
        assert_eq!(decompress(&compress(b"", MAGIC_NSYC).unwrap(), MAGIC_NSYC).unwrap(), b"");
        // The frame says how big the content is.
        let frame = &buf[24..];
        let size = zstd::zstd_safe::get_frame_content_size(frame).unwrap();
        assert_eq!(size, Some(data.len() as u64));
        // Every level reads back; a higher one is no bigger, a level past
        // them is the nearest.
        let mut sizes = Vec::new();
        for level in [1, 3, 9, 19, 22] {
            let buf = compress_at(&data, MAGIC_NSYC, level).unwrap();
            assert_eq!(decompress(&buf, MAGIC_NSYC).unwrap(), data, "level {level}");
            sizes.push(buf.len());
        }
        assert!(sizes.windows(2).all(|w| w[1] <= w[0]), "{sizes:?}");
        assert_eq!(compress_at(&data, MAGIC_NSYC, 99), compress_at(&data, MAGIC_NSYC, 22));
    }

    #[test]
    fn stored_round_trip_and_magic_check() {
        let buf = store(b"hello", MAGIC_XRES);
        assert_eq!(decompress(&buf, MAGIC_XRES).unwrap(), b"hello");
        assert!(matches!(decompress(&buf, MAGIC_NSYC), Err(CompressedBufError::BadMagic { .. })));
    }

    #[test]
    fn zstd_frame() {
        // `printf 'hello hello hello' | zstd -19 --no-check`, wrapped by hand.
        let frame: &[u8] = &[
            0x28, 0xb5, 0x2f, 0xfd, 0x00, 0x68, 0x6d, 0x00, 0x00, 0x38, 0x68, 0x65, 0x6c, 0x6c,
            0x6f, 0x20, 0x68, 0x01, 0x00, 0xd9, 0x8a, 0x11,
        ];
        let mut buf = Vec::new();
        use mg_core::bin::WriteLe;
        buf.put_bytes(b"XRES");
        buf.put_u32(3);
        buf.put_u32(2);
        buf.put_u32(17);
        buf.put_u32(1);
        buf.put_u32(0);
        buf.put_bytes(frame);
        match decompress(&buf, MAGIC_XRES) {
            Ok(v) => assert_eq!(v, b"hello hello hello"),
            Err(e) => panic!("{e}"),
        }
    }
}
