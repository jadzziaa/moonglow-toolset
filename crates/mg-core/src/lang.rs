//! Game languages, genders and the codepages text is stored in.
//!
//! The game stores text as bytes in the codepage of the language it runs in
//! (Windows-1252 for the western languages, Windows-1250 for Polish). Moonglow
//! keeps text as bytes in files and decodes it with a [`Codepage`] only for
//! display and editing, so nothing is lost on a round trip.

use std::borrow::Cow;
use std::fmt;

use encoding_rs::Encoding;

/// A game language id, as used in TLK headers and localized strings.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Language(pub u32);

impl Language {
    pub const ENGLISH: Language = Language(0);
    pub const FRENCH: Language = Language(1);
    pub const GERMAN: Language = Language(2);
    pub const ITALIAN: Language = Language(3);
    pub const SPANISH: Language = Language(4);
    pub const POLISH: Language = Language(5);
    // Only in the original release; EE does not ship these.
    pub const KOREAN: Language = Language(128);
    pub const CHINESE_TRADITIONAL: Language = Language(129);
    pub const CHINESE_SIMPLIFIED: Language = Language(130);
    pub const JAPANESE: Language = Language(131);

    /// The languages EE ships, in id order.
    pub const EE: [Language; 6] = [
        Language::ENGLISH,
        Language::FRENCH,
        Language::GERMAN,
        Language::ITALIAN,
        Language::SPANISH,
        Language::POLISH,
    ];

    /// The English name, if the id is known.
    pub fn name(self) -> Option<&'static str> {
        Some(match self.0 {
            0 => "English",
            1 => "French",
            2 => "German",
            3 => "Italian",
            4 => "Spanish",
            5 => "Polish",
            128 => "Korean",
            129 => "Chinese (Traditional)",
            130 => "Chinese (Simplified)",
            131 => "Japanese",
            _ => return None,
        })
    }

    /// The two-letter code the game uses for its `lang/` folders.
    pub fn short_code(self) -> Option<&'static str> {
        Some(match self.0 {
            0 => "en",
            1 => "fr",
            2 => "de",
            3 => "it",
            4 => "es",
            5 => "pl",
            _ => return None,
        })
    }

    /// The codepage the game uses for text in this language.
    pub fn codepage(self) -> Codepage {
        match self.0 {
            5 => Codepage::WINDOWS_1250,
            128 => Codepage(encoding_rs::EUC_KR),
            129 => Codepage(encoding_rs::BIG5),
            130 => Codepage(encoding_rs::GBK),
            131 => Codepage(encoding_rs::SHIFT_JIS),
            _ => Codepage::WINDOWS_1252,
        }
    }
}

impl fmt::Debug for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(n) => write!(f, "Language({n})"),
            None => write!(f, "Language({})", self.0),
        }
    }
}

/// Grammatical gender of a localized string variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Gender {
    Male = 0,
    Female = 1,
}

/// A text encoding for game strings.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Codepage(pub &'static Encoding);

impl Codepage {
    pub const WINDOWS_1252: Codepage = Codepage(encoding_rs::WINDOWS_1252);
    pub const WINDOWS_1250: Codepage = Codepage(encoding_rs::WINDOWS_1250);

    /// Decodes game bytes to text. Windows-1252 and -1250 map every byte, so
    /// for the EE languages this never loses information.
    pub fn decode<'a>(&self, bytes: &'a [u8]) -> Cow<'a, str> {
        self.0.decode_without_bom_handling(bytes).0
    }

    /// Encodes text to game bytes. Returns `None` if the text has characters
    /// the codepage cannot represent.
    pub fn encode<'a>(&self, text: &'a str) -> Option<Cow<'a, [u8]>> {
        let (bytes, _, unmappable) = self.0.encode(text);
        (!unmappable).then_some(bytes)
    }
}

impl Default for Codepage {
    fn default() -> Self {
        Codepage::WINDOWS_1252
    }
}

impl fmt::Debug for Codepage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Codepage({})", self.0.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn western_codepages_round_trip_every_byte() {
        let all: Vec<u8> = (0..=255).collect();
        for cp in [Codepage::WINDOWS_1252, Codepage::WINDOWS_1250] {
            let text = cp.decode(&all);
            assert_eq!(cp.encode(&text).unwrap().as_ref(), &all[..], "{cp:?}");
        }
    }

    #[test]
    fn unmappable_text_is_rejected() {
        assert!(Codepage::WINDOWS_1252.encode("déjà vu").is_some());
        assert!(Codepage::WINDOWS_1252.encode("日本").is_none());
    }

    #[test]
    fn language_metadata() {
        assert_eq!(Language::POLISH.codepage(), Codepage::WINDOWS_1250);
        assert_eq!(Language::GERMAN.short_code(), Some("de"));
        assert_eq!(Language(77).name(), None);
    }
}
