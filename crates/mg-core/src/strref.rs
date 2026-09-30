//! Talk-table string references.

use std::fmt;

/// An index into the talk table. `0xFFFFFFFF` means "no string"; bit 24 set
/// means the string comes from the module's custom TLK instead of
/// `dialog.tlk`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StrRef(pub u32);

impl StrRef {
    /// No string.
    pub const NONE: StrRef = StrRef(u32::MAX);
    /// The flag that selects the custom (module) talk table.
    pub const CUSTOM_FLAG: u32 = 0x0100_0000;

    pub fn is_none(self) -> bool {
        self == StrRef::NONE
    }

    /// Whether this references the custom talk table.
    pub fn is_custom(self) -> bool {
        !self.is_none() && self.0 & Self::CUSTOM_FLAG != 0
    }

    /// The row in whichever talk table it refers to.
    pub fn index(self) -> u32 {
        self.0 & !Self::CUSTOM_FLAG
    }

    /// A reference to row `index` of the custom talk table.
    pub fn custom(index: u32) -> StrRef {
        StrRef(index | Self::CUSTOM_FLAG)
    }
}

impl Default for StrRef {
    fn default() -> Self {
        StrRef::NONE
    }
}

impl fmt::Debug for StrRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_none() {
            f.write_str("StrRef(none)")
        } else if self.is_custom() {
            write!(f, "StrRef(custom {})", self.index())
        } else {
            write!(f, "StrRef({})", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags() {
        assert!(StrRef::NONE.is_none());
        assert!(!StrRef::NONE.is_custom());
        let c = StrRef::custom(12);
        assert!(c.is_custom());
        assert_eq!(c.index(), 12);
        assert_eq!(c.0, 0x0100_000c);
        assert!(!StrRef(5).is_custom());
    }
}
