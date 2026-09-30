//! KEY/BIF archives: the base game's resources.
//!
//! A KEY file lists resource names and points each at a slot in one of its
//! BIF files. [`KeyTable`] and [`Bif`] parse the two formats from bytes;
//! [`KeySet`] opens a KEY from a game install together with all its BIFs
//! (memory-mapped) and serves resource data.
//!
//! Only the `V1` layout is supported; EE ships nothing else.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use mg_core::bin::{BinError, Reader, slice};
use mg_core::{ResRef, ResType};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("not a KEY V1 file (found {0:?})")]
    NotKey(String),
    #[error("not a BIF V1 file (found {0:?})")]
    NotBif(String),
    #[error("truncated or corrupt data: {0}")]
    Bin(#[from] BinError),
    #[error("key entry {index} has an invalid resref: {message}")]
    BadResRef { index: usize, message: String },
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{path}: {source}")]
    Bif { path: PathBuf, source: Box<KeyError> },
    #[error("{resref}.{restype} points at BIF {bif} slot {slot}, which does not exist")]
    Dangling { resref: ResRef, restype: ResType, bif: usize, slot: u32 },
}

/// A BIF listed in a KEY file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BifRef {
    /// Path relative to the install root, with `/` separators
    /// (e.g. `data/base_2da.bif`).
    pub filename: String,
    pub size: u32,
    pub drives: u16,
}

/// A resource listed in a KEY file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEntry {
    pub resref: ResRef,
    pub restype: ResType,
    /// Index into [`KeyTable::bifs`].
    pub bif: usize,
    /// Variable-resource slot inside that BIF.
    pub slot: u32,
}

/// A parsed KEY file.
#[derive(Debug, Clone)]
pub struct KeyTable {
    pub build_year: u32,
    pub build_day: u32,
    pub bifs: Vec<BifRef>,
    pub entries: Vec<KeyEntry>,
}

impl KeyTable {
    pub fn read(data: &[u8]) -> Result<KeyTable, KeyError> {
        let mut r = Reader::new(data);
        let magic = r.bytes(8)?;
        if magic != b"KEY V1  " {
            return Err(KeyError::NotKey(String::from_utf8_lossy(magic).into_owned()));
        }
        let bif_count = r.u32_usize()?;
        let key_count = r.u32_usize()?;
        let file_table = r.u32_usize()?;
        let key_table = r.u32_usize()?;
        let build_year = r.u32()?;
        let build_day = r.u32()?;

        slice(data, file_table, bif_count.saturating_mul(12))?;
        let mut ft = Reader::at(data, file_table)?;
        let mut bifs = Vec::with_capacity(bif_count);
        for _ in 0..bif_count {
            let size = ft.u32()?;
            let name_offset = ft.u32_usize()?;
            let name_len = ft.u16()? as usize;
            let drives = ft.u16()?;
            let raw = slice(data, name_offset, name_len)?;
            let raw = raw.iter().position(|&b| b == 0).map_or(raw, |end| &raw[..end]);
            let filename = String::from_utf8_lossy(raw).replace('\\', "/");
            bifs.push(BifRef { filename, size, drives });
        }

        slice(data, key_table, key_count.saturating_mul(22))?;
        let mut kt = Reader::at(data, key_table)?;
        let mut entries = Vec::with_capacity(key_count);
        for index in 0..key_count {
            let name = kt.fixed_str(16)?;
            let restype = ResType(kt.u16()?);
            let id = kt.u32()?;
            let resref = ResRef::from_bytes(name)
                .map_err(|e| KeyError::BadResRef { index, message: e.to_string() })?;
            entries.push(KeyEntry {
                resref,
                restype,
                bif: (id >> 20) as usize,
                slot: id & 0xF_FFFF,
            });
        }
        Ok(KeyTable { build_year, build_day, bifs, entries })
    }
}

/// A variable resource slot in a BIF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BifResource {
    pub id: u32,
    pub offset: u32,
    pub size: u32,
    pub restype: ResType,
}

/// A parsed BIF resource table.
#[derive(Debug, Clone)]
pub struct Bif {
    /// Indexed by slot (`id & 0xFFFFF`).
    pub resources: Vec<BifResource>,
}

impl Bif {
    pub fn read(data: &[u8]) -> Result<Bif, KeyError> {
        let mut r = Reader::new(data);
        let magic = r.bytes(8)?;
        if magic != b"BIFFV1  " {
            return Err(KeyError::NotBif(String::from_utf8_lossy(magic).into_owned()));
        }
        let count = r.u32_usize()?;
        let _fixed_count = r.u32()?; // never used by NWN
        let table = r.u32_usize()?;
        slice(data, table, count.saturating_mul(16))?;
        let mut t = Reader::at(data, table)?;
        let mut resources = Vec::with_capacity(count);
        for _ in 0..count {
            let id = t.u32()?;
            let offset = t.u32()?;
            let size = t.u32()?;
            let restype = ResType(t.u32()? as u16);
            resources.push(BifResource { id, offset, size, restype });
        }
        Ok(Bif { resources })
    }

    /// The resource in `slot`, if any.
    pub fn resource(&self, slot: u32) -> Option<&BifResource> {
        // Slots are stored in order, but look the id up if a file ever isn't.
        match self.resources.get(slot as usize) {
            Some(r) if r.id & 0xF_FFFF == slot => Some(r),
            _ => self.resources.iter().find(|r| r.id & 0xF_FFFF == slot),
        }
    }
}

struct OpenBif {
    map: Mmap,
    table: Bif,
}

/// A KEY file with its BIFs opened (memory-mapped), ready to serve data.
pub struct KeySet {
    pub path: PathBuf,
    pub table: KeyTable,
    bifs: Vec<OpenBif>,
    index: HashMap<(ResRef, ResType), usize>,
}

impl std::fmt::Debug for KeySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeySet")
            .field("path", &self.path)
            .field("bifs", &self.bifs.len())
            .field("entries", &self.table.entries.len())
            .finish()
    }
}

#[allow(unsafe_code)]
fn map_file(path: &Path) -> Result<Mmap, KeyError> {
    let file = File::open(path).map_err(|source| KeyError::Io { path: path.into(), source })?;
    // SAFETY: game archives are read-only data; if another process truncates
    // one while it is mapped, reads may fault, which is the accepted risk of
    // memory-mapping (every Aurora-engine tool does the same).
    unsafe { Mmap::map(&file) }.map_err(|source| KeyError::Io { path: path.into(), source })
}

impl KeySet {
    /// Opens `key` (e.g. `<install>/data/nwn_base.key`); BIF paths in the key
    /// are resolved against `install_root`.
    pub fn open(key: &Path, install_root: &Path) -> Result<KeySet, KeyError> {
        let key_map = map_file(key)?;
        let table = KeyTable::read(&key_map)?;
        let bifs = table
            .bifs
            .iter()
            .map(|b| {
                let path = install_root.join(&b.filename);
                let map = map_file(&path)?;
                let table = Bif::read(&map)
                    .map_err(|e| KeyError::Bif { path: path.clone(), source: Box::new(e) })?;
                Ok(OpenBif { map, table })
            })
            .collect::<Result<Vec<_>, KeyError>>()?;
        // Later entries win, as in the engine's key table lookup.
        let index =
            table.entries.iter().enumerate().map(|(i, e)| ((e.resref, e.restype), i)).collect();
        Ok(KeySet { path: key.into(), table, bifs, index })
    }

    /// The key entry for a resource.
    pub fn find(&self, resref: &ResRef, restype: ResType) -> Option<&KeyEntry> {
        self.index.get(&(*resref, restype)).map(|&i| &self.table.entries[i])
    }

    /// The bytes of a listed resource.
    pub fn data(&self, entry: &KeyEntry) -> Result<&[u8], KeyError> {
        let dangling = || KeyError::Dangling {
            resref: entry.resref,
            restype: entry.restype,
            bif: entry.bif,
            slot: entry.slot,
        };
        let bif = self.bifs.get(entry.bif).ok_or_else(dangling)?;
        let res = bif.table.resource(entry.slot).ok_or_else(dangling)?;
        Ok(slice(&bif.map, res.offset as usize, res.size as usize)?)
    }

    /// Looks up and returns a resource's bytes.
    pub fn get(&self, resref: &ResRef, restype: ResType) -> Option<Result<&[u8], KeyError>> {
        self.find(resref, restype).map(|e| self.data(e))
    }
}

#[cfg(test)]
mod tests {
    use mg_core::bin::WriteLe;

    use super::*;

    fn tiny_key_and_bif() -> (Vec<u8>, Vec<u8>) {
        let mut bif = Vec::new();
        bif.put_bytes(b"BIFFV1  ");
        bif.put_u32(2);
        bif.put_u32(0);
        bif.put_u32(20);
        let data_start = 20 + 2 * 16;
        for (i, (len, ty)) in [(3u32, 2017u32), (2, 10)].iter().enumerate() {
            bif.put_u32(i as u32);
            bif.put_u32(data_start + if i == 0 { 0 } else { 3 });
            bif.put_u32(*len);
            bif.put_u32(*ty);
        }
        bif.put_bytes(b"abcde");

        let mut key = Vec::new();
        let name = b"data\\x.bif\0";
        key.put_bytes(b"KEY V1  ");
        key.put_u32(1);
        key.put_u32(2);
        key.put_u32(64);
        key.put_u32(64 + 12 + name.len() as u32);
        key.put_u32(124);
        key.put_u32(0);
        key.resize(64, 0);
        key.put_u32(bif.len() as u32);
        key.put_u32(64 + 12);
        key.put_u16(name.len() as u16);
        key.put_u16(1);
        key.put_bytes(name);
        for (i, (n, ty)) in [("Classes", 2017u16), ("readme", 10)].iter().enumerate() {
            key.put_fixed(n.as_bytes(), 16);
            key.put_u16(*ty);
            key.put_u32(i as u32);
        }
        (key, bif)
    }

    #[test]
    fn parses_key_and_bif() {
        let (key, bif) = tiny_key_and_bif();
        let k = KeyTable::read(&key).unwrap();
        assert_eq!(k.bifs[0].filename, "data/x.bif");
        assert_eq!(k.entries.len(), 2);
        assert_eq!(k.entries[0].resref.as_str(), Some("Classes"));
        let b = Bif::read(&bif).unwrap();
        let r = b.resource(k.entries[1].slot).unwrap();
        assert_eq!(&bif[r.offset as usize..][..r.size as usize], b"de");
    }

    #[test]
    fn opens_from_disk() {
        let (key, bif) = tiny_key_and_bif();
        let dir = mg_testkit::scratch_dir("mg-key-open");
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::write(dir.join("data/x.bif"), bif).unwrap();
        std::fs::write(dir.join("data/t.key"), key).unwrap();
        let ks = KeySet::open(&dir.join("data/t.key"), &dir).unwrap();
        let rr = ResRef::from_str("classes").unwrap();
        assert_eq!(ks.get(&rr, ResType::TWODA).unwrap().unwrap(), b"abc");
        assert!(ks.get(&rr, ResType::TXT).is_none());
    }

    #[test]
    fn truncated_input_is_an_error_not_a_panic() {
        let (key, bif) = tiny_key_and_bif();
        for len in 0..key.len() {
            let _ = KeyTable::read(&key[..len]);
        }
        for len in 0..bif.len() {
            let _ = Bif::read(&bif[..len]);
        }
    }
}
