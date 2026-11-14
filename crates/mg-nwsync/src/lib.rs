//! NWSync content read: a module whose haks a server hands out with
//! NWSync opened with that content, without the hak files.
//!
//! A server's repository (`<url>/latest`, `<url>/manifests/<sha1>`,
//! `<url>/data/sha1/ab/cd/<sha1>`) lists in a manifest every resource of
//! the module's haks and its talk table, the copy of each name that wins,
//! by the SHA-1 of its bytes ([`manifest`]). The game downloads them into
//! its own storage before it joins the server; Moonglow reads them from
//! there, read only ([`GameStorage`]), and downloads what it lacks into a
//! cache of its own ([`Cache`], [`fetch`]), never into the game's: the
//! game checks its storage at start-up and downloads everything again when
//! it was changed by something else.
//!
//! [`NwSyncContainer`] is a manifest as a layer of the resource manager,
//! at `priority::NWSYNC`, above the haks and below loose folders, where
//! the game puts it. [`find`] picks the manifest from what is here (no
//! network); [`fetch::resolve`] asks the server first.
//!
//! What is read is checked against the manifest's size; what is
//! downloaded against its SHA-1 too before it is kept. The game's storage
//! is taken as the game checked it.

mod cache;
mod container;
pub mod fetch;
#[cfg(feature = "fixtures")]
pub mod fixtures;
pub mod game;
pub mod manifest;
pub mod origins;
mod source;

pub use cache::{Cache, server_key};
pub use container::{LABEL_PREFIX, NwSyncContainer, label};
pub use game::{GameManifest, GameStorage};
pub use origins::{Origin, Origins};
pub use source::{Found, ManifestFrom, Settings, Survey, Wanted, find, survey};

use mg_erf::compressedbuf::{MAGIC_NSYC, decompress};

/// A SHA-1 digest: a resource's (of its uncompressed bytes), or a
/// manifest's (of its file).
pub type Sha1 = [u8; 20];

/// A SHA-1 written in hexadecimal (40 digits, either case); `None` for
/// anything else.
pub fn parse_sha1(text: &str) -> Option<Sha1> {
    let text = text.as_bytes();
    if text.len() != 40 {
        return None;
    }
    let digit = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = [0u8; 20];
    for (i, pair) in text.chunks(2).enumerate() {
        out[i] = digit(pair[0])? << 4 | digit(pair[1])?;
    }
    Some(out)
}

/// A SHA-1 in lowercase hexadecimal, as NWSync names files.
pub fn sha1_hex(sha1: &Sha1) -> String {
    mg_core::sha1::hex(sha1)
}

/// A resource's bytes from its data as stored: a compressed buffer
/// (`NSYC`: zstd, zlib or none), or the bytes themselves (the game's
/// storage may keep them so).
pub fn unpack(stored: &[u8]) -> Result<Vec<u8>, String> {
    if stored.starts_with(&MAGIC_NSYC) {
        decompress(stored, MAGIC_NSYC).map_err(|e| e.to_string())
    } else {
        Ok(stored.to_vec())
    }
}

/// [`unpack`], checked: the bytes must be `size` long (where it is not 0)
/// and have this SHA-1.
pub fn unpack_checked(stored: &[u8], sha1: &Sha1, size: u32) -> Result<Vec<u8>, String> {
    let data = unpack(stored)?;
    if size != 0 && data.len() != size as usize {
        return Err(format!("{} bytes, where the manifest says {size}", data.len()));
    }
    let got = mg_core::sha1::sha1(&data);
    if got != *sha1 {
        return Err(format!("its SHA-1 is {}, not {}", sha1_hex(&got), sha1_hex(sha1)));
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_in_hexadecimal() {
        let sha = mg_core::sha1::sha1(b"abc");
        let hex = sha1_hex(&sha);
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(parse_sha1(&hex), Some(sha));
        assert_eq!(parse_sha1(&hex.to_uppercase()), Some(sha));
        assert_eq!(parse_sha1(&hex[1..]), None);
        assert_eq!(parse_sha1(&format!("{}g", &hex[1..])), None);
        assert_eq!(parse_sha1("é9993e364706816aba3e25717850c26c9cd0d89"), None);
    }

    /// Stored bytes of every kind unpack and are checked.
    #[test]
    fn stored_data_is_unpacked_and_checked() {
        let data = b"the bytes of a resource, the bytes of a resource".repeat(20);
        let sha = mg_core::sha1::sha1(&data);
        let zstd = mg_erf::compressedbuf::compress(&data, MAGIC_NSYC).unwrap();
        let none = mg_erf::compressedbuf::store(&data, MAGIC_NSYC);
        for stored in [&zstd[..], &none[..], &data[..]] {
            assert_eq!(unpack_checked(stored, &sha, data.len() as u32).unwrap(), data);
            assert_eq!(unpack_checked(stored, &sha, 0).unwrap(), data);
        }
        assert!(unpack_checked(&zstd, &sha, 5).unwrap_err().contains("manifest says 5"));
        let other = mg_core::sha1::sha1(b"other");
        assert!(unpack_checked(&zstd, &other, 0).unwrap_err().contains("SHA-1"));
        assert!(unpack(&zstd[..zstd.len() - 3]).is_err());
    }
}
