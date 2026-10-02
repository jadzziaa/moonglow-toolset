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
            let mut dec = ruzstd::decoding::StreamingDecoder::new(frame)
                .map_err(|e| CompressedBufError::Zstd(e.to_string()))?;
            let mut out = Vec::with_capacity(size);
            dec.read_to_end(&mut out).map_err(|e| CompressedBufError::Zstd(e.to_string()))?;
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

/// Compresses bytes with zstd (algorithm 2), as NWSync's data files are.
/// The frame declares its content size, which one-shot decoders (the
/// reference library's `ZSTD_decompress`, which neverwinter.nim uses) need.
pub fn compress(data: &[u8], magic: [u8; 4]) -> Vec<u8> {
    use mg_core::bin::WriteLe;
    let mut frame =
        ruzstd::encoding::compress_to_vec(data, ruzstd::encoding::CompressionLevel::Fastest);
    with_content_size(&mut frame, data.len());
    let mut out = Vec::with_capacity(24 + frame.len());
    out.put_bytes(&magic);
    out.put_u32(3);
    out.put_u32(2);
    out.put_u32_usize(data.len());
    out.put_u32(1);
    out.put_u32(0);
    out.put_bytes(&frame);
    out
}

/// Adds the content size to a zstd frame header that has none: ruzstd
/// writes a window descriptor and no size. The header is the magic, a
/// descriptor byte (bits 7–6 the size field's width: 2 is 4 bytes; bit 5
/// single segment; bits 1–0 the dictionary id's width), the window
/// descriptor, then the size.
fn with_content_size(frame: &mut Vec<u8>, size: usize) {
    let Ok(size) = u32::try_from(size) else { return };
    let Some(&descriptor) = frame.get(4) else { return };
    if frame.len() < 6 || descriptor & 0b1110_0011 != 0 {
        return;
    }
    frame[4] = descriptor | 0b1000_0000;
    frame.splice(6..6, size.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compressed_round_trip() {
        let data: Vec<u8> = (0..20_000u32).flat_map(|i| (i % 251).to_le_bytes()).collect();
        let buf = compress(&data, MAGIC_NSYC);
        assert!(buf.len() < data.len() / 2, "{}", buf.len());
        assert_eq!(decompress(&buf, MAGIC_NSYC).unwrap(), data);
        assert_eq!(decompress(&compress(b"", MAGIC_NSYC), MAGIC_NSYC).unwrap(), b"");
        // The frame says how big the content is: the descriptor's size
        // field is 4 bytes, after the window descriptor.
        let frame = &buf[24..];
        assert_eq!(frame[4] >> 6, 2);
        assert_eq!(u32::from_le_bytes(frame[6..10].try_into().unwrap()) as usize, data.len());
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
