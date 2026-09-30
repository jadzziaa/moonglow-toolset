//! Little-endian binary reading and writing for the game's file formats.
//!
//! [`Reader`] is a bounds-checked cursor over a byte slice: every read either
//! succeeds or returns a [`BinError`] naming the offset, so parsers never panic
//! on truncated or hostile input. [`WriteLe`] adds little-endian writers to
//! `Vec<u8>`.

use thiserror::Error;

/// A read past the end of the input, or a value that cannot be represented.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BinError {
    #[error(
        "unexpected end of data at offset {offset:#x}: needed {needed} bytes, {available} left"
    )]
    UnexpectedEof { offset: usize, needed: usize, available: usize },
    #[error("offset {offset:#x} is outside the data (length {len:#x})")]
    OutOfBounds { offset: usize, len: usize },
}

/// A bounds-checked little-endian cursor over a byte slice.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

macro_rules! read_le {
    ($($name:ident -> $ty:ty),* $(,)?) => {$(
        #[doc = concat!("Reads a little-endian `", stringify!($ty), "`.")]
        #[inline]
        pub fn $name(&mut self) -> Result<$ty, BinError> {
            Ok(<$ty>::from_le_bytes(self.array()?))
        }
    )*};
}

impl<'a> Reader<'a> {
    /// A reader positioned at the start of `data`.
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    /// A reader positioned at `offset` in `data`.
    pub fn at(data: &'a [u8], offset: usize) -> Result<Self, BinError> {
        let mut r = Reader::new(data);
        r.seek(offset)?;
        Ok(r)
    }

    /// The whole underlying slice.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// The current offset.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Bytes left after the current offset.
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Moves to an absolute offset (the end of the data is allowed).
    pub fn seek(&mut self, offset: usize) -> Result<(), BinError> {
        if offset > self.data.len() {
            return Err(BinError::OutOfBounds { offset, len: self.data.len() });
        }
        self.pos = offset;
        Ok(())
    }

    /// Advances by `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<(), BinError> {
        self.bytes(n).map(|_| ())
    }

    /// Reads `n` bytes.
    #[inline]
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], BinError> {
        let available = self.remaining();
        if n > available {
            return Err(BinError::UnexpectedEof { offset: self.pos, needed: n, available });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    /// Reads a fixed-size array.
    #[inline]
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], BinError> {
        let b = self.bytes(N)?;
        Ok(b.try_into().expect("slice has length N"))
    }

    /// Reads a fixed-size, NUL-padded field and returns the bytes before the
    /// first NUL.
    pub fn fixed_str(&mut self, n: usize) -> Result<&'a [u8], BinError> {
        let b = self.bytes(n)?;
        Ok(b.iter().position(|&c| c == 0).map_or(b, |end| &b[..end]))
    }

    read_le! {
        u8 -> u8, i8 -> i8,
        u16 -> u16, i16 -> i16,
        u32 -> u32, i32 -> i32,
        u64 -> u64, i64 -> i64,
        f32 -> f32, f64 -> f64,
    }

    /// Reads a `u32` and widens it to `usize` (for counts and offsets).
    #[inline]
    pub fn u32_usize(&mut self) -> Result<usize, BinError> {
        Ok(self.u32()? as usize)
    }
}

/// Returns `data[offset..offset + len]`, or an error naming the range.
pub fn slice(data: &[u8], offset: usize, len: usize) -> Result<&[u8], BinError> {
    offset.checked_add(len).and_then(|end| data.get(offset..end)).ok_or(BinError::UnexpectedEof {
        offset,
        needed: len,
        available: data.len().saturating_sub(offset),
    })
}

/// Little-endian writers for byte buffers.
pub trait WriteLe {
    fn put_u8(&mut self, v: u8);
    fn put_u16(&mut self, v: u16);
    fn put_i16(&mut self, v: i16);
    fn put_u32(&mut self, v: u32);
    fn put_i32(&mut self, v: i32);
    fn put_u64(&mut self, v: u64);
    fn put_i64(&mut self, v: i64);
    fn put_f32(&mut self, v: f32);
    fn put_f64(&mut self, v: f64);
    fn put_bytes(&mut self, v: &[u8]);
    /// Writes `v` into a fixed-size field, padded with NULs.
    ///
    /// # Panics
    /// If `v` is longer than `n`; callers validate lengths first.
    fn put_fixed(&mut self, v: &[u8], n: usize);
    /// Writes a `usize` as `u32`.
    ///
    /// # Panics
    /// If the value does not fit; the formats cannot represent it.
    fn put_u32_usize(&mut self, v: usize);
}

impl WriteLe for Vec<u8> {
    fn put_u8(&mut self, v: u8) {
        self.push(v);
    }
    fn put_u16(&mut self, v: u16) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_i16(&mut self, v: i16) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_u32(&mut self, v: u32) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_i32(&mut self, v: i32) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_u64(&mut self, v: u64) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_i64(&mut self, v: i64) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_f32(&mut self, v: f32) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_f64(&mut self, v: f64) {
        self.extend_from_slice(&v.to_le_bytes());
    }
    fn put_bytes(&mut self, v: &[u8]) {
        self.extend_from_slice(v);
    }
    fn put_fixed(&mut self, v: &[u8], n: usize) {
        assert!(v.len() <= n, "value of {} bytes does not fit a {n}-byte field", v.len());
        self.extend_from_slice(v);
        self.resize(self.len() + (n - v.len()), 0);
    }
    fn put_u32_usize(&mut self, v: usize) {
        self.put_u32(u32::try_from(v).expect("value exceeds u32"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_and_reports_eof() {
        let data = [0x01, 0x02, 0x03, 0x04, 0xff];
        let mut r = Reader::new(&data);
        assert_eq!(r.u32().unwrap(), 0x0403_0201);
        assert_eq!(r.i8().unwrap(), -1);
        assert_eq!(r.u16(), Err(BinError::UnexpectedEof { offset: 5, needed: 2, available: 0 }));
    }

    #[test]
    fn fixed_str_stops_at_nul() {
        let mut r = Reader::new(b"abc\0\0\0xyz");
        assert_eq!(r.fixed_str(6).unwrap(), b"abc");
        assert_eq!(r.fixed_str(3).unwrap(), b"xyz");
    }

    #[test]
    fn seek_and_slice_bounds() {
        let data = [0u8; 4];
        assert!(Reader::at(&data, 4).is_ok());
        assert!(Reader::at(&data, 5).is_err());
        assert!(slice(&data, 2, 2).is_ok());
        assert!(slice(&data, 3, 2).is_err());
        assert!(slice(&data, usize::MAX, 2).is_err());
    }

    #[test]
    fn writes_round_trip() {
        let mut v = Vec::new();
        v.put_u32(7);
        v.put_f32(1.5);
        v.put_fixed(b"ab", 4);
        v.put_i64(-3);
        let mut r = Reader::new(&v);
        assert_eq!(r.u32().unwrap(), 7);
        assert_eq!(r.f32().unwrap(), 1.5);
        assert_eq!(r.fixed_str(4).unwrap(), b"ab");
        assert_eq!(r.i64().unwrap(), -3);
        assert_eq!(r.remaining(), 0);
    }
}
