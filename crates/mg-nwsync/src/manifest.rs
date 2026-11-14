//! NWSync manifests (`NSYM`, version 3), as neverwinter.nim's `nwsync.nim`
//! reads and writes them:
//!
//! ```text
//! "NSYM" | u32 version = 3 | u32 entries | u32 mappings
//! entry   (42 bytes, sorted by SHA-1, name, type): sha1[20] | u32 size | resref[16] | u16 type
//! mapping (22 bytes): u32 index of an entry | resref[16] | u16 type
//! ```
//!
//! A mapping is a second name for an entry's bytes. A resource's SHA-1 is
//! that of its uncompressed bytes, and names its data file on the server.

use std::collections::BTreeMap;

use mg_core::ResType;
use mg_core::bin::{BinError, Reader, WriteLe};
use mg_resman::ResKey;

use crate::Sha1;

/// A resource of a manifest: its name and type, the SHA-1 and size of its
/// bytes (a size of 0: not known, as for a manifest read from the game's
/// storage without its file).
pub type Entry = (ResKey, Sha1, u32);

/// The manifest's bytes for resources (name and type, SHA-1, size):
/// sorted by SHA-1, name and type, resources of the same bytes listed
/// once and mapped to the first, as `nwn_nwsync_write` writes it.
pub fn write(entries: &[Entry]) -> Vec<u8> {
    let mut sorted: Vec<(&Sha1, String, u16, u32)> = entries
        .iter()
        .map(|(k, sha, size)| (sha, k.resref.to_lowercase().to_string(), k.restype.0, *size))
        .collect();
    sorted.sort_by(|a, b| (a.0, &a.1, a.2).cmp(&(b.0, &b.1, b.2)));
    let mut unique: Vec<&(&Sha1, String, u16, u32)> = Vec::new();
    let mut mappings = Vec::new();
    let mut first: BTreeMap<&Sha1, usize> = BTreeMap::new();
    for e in &sorted {
        match first.get(e.0) {
            Some(&i) => mappings.push((i, e)),
            None => {
                first.insert(e.0, unique.len());
                unique.push(e);
            }
        }
    }
    let mut out = Vec::new();
    out.put_bytes(b"NSYM");
    out.put_u32(3);
    out.put_u32_usize(unique.len());
    out.put_u32_usize(mappings.len());
    for (sha, name, restype, size) in unique {
        out.put_bytes(&sha[..]);
        out.put_u32(*size);
        out.put_fixed(name.as_bytes(), 16);
        out.put_u16(*restype);
    }
    for (i, (_, name, restype, _)) in mappings {
        out.put_u32_usize(i);
        out.put_fixed(name.as_bytes(), 16);
        out.put_u16(*restype);
    }
    out
}

/// Reads a manifest: each resource with its SHA-1 and size, the entries
/// first, then the mappings.
pub fn read(data: &[u8]) -> Result<Vec<Entry>, String> {
    let mut r = Reader::new(data);
    let err = |e: BinError| format!("not a manifest: {e}");
    if r.bytes(4).map_err(err)? != b"NSYM" {
        return Err("not an NWSync manifest".into());
    }
    if r.u32().map_err(err)? != 3 {
        return Err("not a version 3 manifest".into());
    }
    let (n, mapped) = (r.u32_usize().map_err(err)?, r.u32_usize().map_err(err)?);
    // (Room made for no more than the bytes there can hold.)
    let room = n.saturating_add(mapped).min(r.remaining() / 22);
    let mut out: Vec<Entry> = Vec::with_capacity(room);
    let name = |r: &mut Reader<'_>| -> Result<ResKey, String> {
        let resref = mg_core::ResRef::from_bytes(r.fixed_str(16).map_err(err)?)
            .map_err(|e| e.to_string())?;
        Ok(ResKey::new(resref, ResType(r.u16().map_err(err)?)))
    };
    for _ in 0..n {
        let sha: Sha1 = r.array().map_err(err)?;
        let size = r.u32().map_err(err)?;
        out.push((name(&mut r)?, sha, size));
    }
    for _ in 0..mapped {
        let i = r.u32_usize().map_err(err)?;
        let (_, sha, size) = *out.get(i).filter(|_| i < n).ok_or("a mapping past the entries")?;
        out.push((name(&mut r)?, sha, size));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str, restype: ResType) -> ResKey {
        ResKey::parse(name, restype).unwrap()
    }

    /// Written and read back: the same resources, a mapping for bytes
    /// listed twice, sorted by SHA-1.
    #[test]
    fn written_and_read_back() {
        let a = mg_core::sha1::sha1(b"a");
        let b = mg_core::sha1::sha1(b"b");
        let entries = vec![
            (key("Zeta", ResType::TWODA), b, 1),
            (key("alpha", ResType::MDL), a, 1),
            (key("beta", ResType::MDL), a, 1),
        ];
        let bytes = write(&entries);
        assert_eq!(bytes.len(), 16 + 2 * 42 + 22);
        let back = read(&bytes).unwrap();
        assert_eq!(back.len(), 3);
        let mut sorted = entries.clone();
        sorted.sort_by_key(|(k, s, _)| (*s, k.resref.to_lowercase().to_string()));
        let mut got = back.clone();
        got.sort_by_key(|(k, s, _)| (*s, k.resref.to_lowercase().to_string()));
        assert_eq!(got, sorted);
        // (Names are written in lowercase.)
        assert!(bytes.windows(4).any(|w| w == b"zeta"));
    }

    /// Cut anywhere, under another magic or version, or with a mapping
    /// past the entries: an error, never a panic.
    #[test]
    fn bad_manifests_are_errors() {
        let sha = mg_core::sha1::sha1(b"x");
        let bytes = write(&[(key("a", ResType::MDL), sha, 1), (key("b", ResType::MDL), sha, 1)]);
        for n in 0..bytes.len() {
            assert!(read(&bytes[..n]).is_err(), "cut at {n}");
        }
        let mut other = bytes.clone();
        other[0] = b'X';
        assert!(read(&other).is_err());
        let mut version = bytes.clone();
        version[4] = 2;
        assert_eq!(read(&version).unwrap_err(), "not a version 3 manifest");
        let mut past = bytes.clone();
        past[16 + 42] = 1;
        assert_eq!(read(&past).unwrap_err(), "a mapping past the entries");
        // (A count of four billion entries in 16 bytes: refused, no room made.)
        let mut huge = bytes;
        huge[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read(&huge).is_err());
    }
}
