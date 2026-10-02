//! Writing archives larger than 2 GiB without using the disk space: an
//! ERF (`V1.0`) whose largest resource is zeros, left as a hole in a
//! sparse file, so the resources after it sit past the 2 GiB mark. (Linux
//! and macOS file systems leave the hole unwritten; NTFS writes it.)

use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;

/// A resource: name (a resref), type number and data.
pub type Entry<'a> = (&'a str, u16, &'a [u8]);

const HEADER_SIZE: u64 = 160;

/// Writes an archive of `file_type` (`b"HAK "`): the `before` resources,
/// then `padding` (name, type, size) as that many zero bytes not written to
/// disk, then the `after` resources.
pub fn write_padded(
    path: &Path,
    file_type: &[u8; 4],
    before: &[Entry<'_>],
    padding: (&str, u16, u64),
    after: &[Entry<'_>],
) -> io::Result<()> {
    let names: Vec<(&str, u16)> = before
        .iter()
        .map(|e| (e.0, e.1))
        .chain([(padding.0, padding.1)])
        .chain(after.iter().map(|e| (e.0, e.1)))
        .collect();
    let n = names.len() as u64;
    let keys_at = HEADER_SIZE;
    let list_at = keys_at + 24 * n;
    let mut at = list_at + 8 * n;
    let mut placed = Vec::new();
    for size in before
        .iter()
        .map(|e| e.2.len() as u64)
        .chain([padding.2])
        .chain(after.iter().map(|e| e.2.len() as u64))
    {
        placed.push((at, size));
        at += size;
    }
    let too_large = || io::Error::new(io::ErrorKind::InvalidInput, "larger than 4 GiB");
    let u32_of = |v: u64| u32::try_from(v).map_err(|_| too_large());
    u32_of(at)?;

    let mut head = Vec::new();
    head.extend_from_slice(file_type);
    head.extend_from_slice(b"V1.0");
    for v in [0, 0, n, HEADER_SIZE, keys_at, list_at, 126, 0, u64::from(u32::MAX)] {
        head.extend_from_slice(&u32_of(v)?.to_le_bytes());
    }
    head.resize(HEADER_SIZE as usize, 0);
    for (i, (name, restype)) in names.iter().enumerate() {
        let mut resref = [0u8; 16];
        let bytes = name.as_bytes();
        if bytes.len() > 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "a name over 16 characters"));
        }
        resref[..bytes.len()].copy_from_slice(bytes);
        head.extend_from_slice(&resref);
        head.extend_from_slice(&(i as u32).to_le_bytes());
        head.extend_from_slice(&restype.to_le_bytes());
        head.extend_from_slice(&[0, 0]);
    }
    for (offset, size) in &placed {
        head.extend_from_slice(&u32_of(*offset)?.to_le_bytes());
        head.extend_from_slice(&u32_of(*size)?.to_le_bytes());
    }
    let mut f = File::create(path)?;
    f.write_all(&head)?;
    for e in before {
        f.write_all(e.2)?;
    }
    f.seek(SeekFrom::Current(i64::try_from(padding.2).map_err(|_| too_large())?))?;
    for e in after {
        f.write_all(e.2)?;
    }
    // A hole at the end (no `after`) still counts toward the length.
    f.set_len(at)?;
    Ok(())
}
