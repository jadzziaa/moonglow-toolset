//! Resource names ("resrefs"): up to 16 bytes, compared case-insensitively.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use thiserror::Error;

/// Why a name is not a valid [`ResRef`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResRefError {
    #[error("resref {0:?} is longer than 16 bytes")]
    TooLong(String),
    #[error("resref {0:?} contains a NUL byte")]
    ContainsNul(String),
}

/// A resource name as the game stores it: at most [`ResRef::MAX_LEN`] bytes.
///
/// The original case is kept (so files round-trip), but equality, ordering
/// and hashing ignore ASCII case, as the engine does. An empty resref is valid
/// and means "none" (e.g. an unset script slot).
///
/// Names are bytes, not text: nearly all are ASCII, and anything else is shown
/// with [`ResRef::to_string_lossy`] rather than rejected.
#[derive(Clone, Copy)]
pub struct ResRef {
    len: u8,
    buf: [u8; ResRef::MAX_LEN],
}

impl ResRef {
    /// The engine's resref length limit.
    pub const MAX_LEN: usize = 16;

    /// The empty resref.
    pub const EMPTY: ResRef = ResRef { len: 0, buf: [0; ResRef::MAX_LEN] };

    /// A resref from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<ResRef, ResRefError> {
        if bytes.len() > Self::MAX_LEN {
            return Err(ResRefError::TooLong(String::from_utf8_lossy(bytes).into_owned()));
        }
        if bytes.contains(&0) {
            return Err(ResRefError::ContainsNul(String::from_utf8_lossy(bytes).into_owned()));
        }
        let mut buf = [0; Self::MAX_LEN];
        buf[..bytes.len()].copy_from_slice(bytes);
        Ok(ResRef { len: bytes.len() as u8, buf })
    }

    /// A resref from text.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<ResRef, ResRefError> {
        Self::from_bytes(s.as_bytes())
    }

    /// The name's bytes, in their original case.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }

    /// The name as text, if it is ASCII (it nearly always is).
    pub fn as_str(&self) -> Option<&str> {
        let b = self.as_bytes();
        if b.is_ascii() { std::str::from_utf8(b).ok() } else { None }
    }

    /// The name as text; non-ASCII bytes are decoded as Latin-1.
    pub fn to_string_lossy(&self) -> String {
        self.as_bytes().iter().map(|&b| b as char).collect()
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// A copy with ASCII letters lowercased (the canonical form for files).
    pub fn to_lowercase(&self) -> ResRef {
        let mut r = *self;
        r.buf.make_ascii_lowercase();
        r
    }

    /// Whether the name is in canonical lowercase form.
    pub fn is_lowercase(&self) -> bool {
        !self.as_bytes().iter().any(u8::is_ascii_uppercase)
    }

    fn folded(&self) -> impl Iterator<Item = u8> + '_ {
        self.as_bytes().iter().map(u8::to_ascii_lowercase)
    }
}

impl Default for ResRef {
    fn default() -> Self {
        ResRef::EMPTY
    }
}

impl PartialEq for ResRef {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes().eq_ignore_ascii_case(other.as_bytes())
    }
}

impl Eq for ResRef {}

impl Hash for ResRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(self.len);
        for b in self.folded() {
            state.write_u8(b);
        }
    }
}

impl Ord for ResRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.folded().cmp(other.folded())
    }
}

impl PartialOrd for ResRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for ResRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResRef({:?})", self.to_string_lossy())
    }
}

impl fmt::Display for ResRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

impl TryFrom<&str> for ResRef {
    type Error = ResRefError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        ResRef::from_str(s)
    }
}

impl std::str::FromStr for ResRef {
    type Err = ResRefError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ResRef::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

    #[test]
    fn case_insensitive_identity_keeps_original_case() {
        let a = ResRef::from_str("NW_Chicken").unwrap();
        let b = ResRef::from_str("nw_chicken").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.as_str(), Some("NW_Chicken"));
        assert_eq!(a.to_lowercase().as_str(), Some("nw_chicken"));
        assert!(!a.is_lowercase());
        let set: HashSet<_> = [a, b].into_iter().collect();
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn limits() {
        assert!(ResRef::from_str("0123456789abcdef").is_ok());
        assert!(matches!(ResRef::from_str("0123456789abcdefg"), Err(ResRefError::TooLong(_))));
        assert!(matches!(ResRef::from_bytes(b"a\0b"), Err(ResRefError::ContainsNul(_))));
        assert!(ResRef::from_str("").unwrap().is_empty());
    }

    #[test]
    fn non_ascii_is_kept_as_bytes() {
        let r = ResRef::from_bytes(&[b'a', 0xe9]).unwrap();
        assert_eq!(r.as_str(), None);
        assert_eq!(r.to_string_lossy(), "aé");
    }

    proptest! {
        #[test]
        fn eq_hash_ord_agree(a in "[A-Za-z0-9_]{0,16}", b in "[A-Za-z0-9_]{0,16}") {
            let (ra, rb) = (ResRef::from_str(&a).unwrap(), ResRef::from_str(&b).unwrap());
            let same = a.eq_ignore_ascii_case(&b);
            prop_assert_eq!(ra == rb, same);
            prop_assert_eq!(ra.cmp(&rb) == Ordering::Equal, same);
            if same {
                use std::hash::BuildHasher;
                let s = std::collections::hash_map::RandomState::new();
                prop_assert_eq!(s.hash_one(ra), s.hash_one(rb));
            }
        }
    }
}
