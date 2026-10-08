//! Game languages, genders and the codepages text is stored in.
//!
//! The game stores text as bytes in the codepage of the language it runs in
//! (Windows-1252 for the western languages, Windows-1250 for Polish). Moonglow
//! keeps text as bytes in files and decodes it with a [`Codepage`] only for
//! display and editing, so nothing is lost on a round trip. A module can
//! replace the game's table of 256 characters with its own (`encoding.2da`,
//! EE 1.87): [`Codepage::table`].

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

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
            128 => Codepage(Kind::Standard(encoding_rs::EUC_KR)),
            129 => Codepage(Kind::Standard(encoding_rs::BIG5)),
            130 => Codepage(Kind::Standard(encoding_rs::GBK)),
            131 => Codepage(Kind::Standard(encoding_rs::SHIFT_JIS)),
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

/// A text encoding for game strings: one of the game's codepages, or a
/// module's own table (`encoding.2da`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Codepage(Kind);

#[derive(Clone, Copy)]
enum Kind {
    Standard(&'static Encoding),
    Table(&'static ByteTable),
}

impl PartialEq for Kind {
    fn eq(&self, other: &Kind) -> bool {
        match (self, other) {
            (Kind::Standard(a), Kind::Standard(b)) => a == b,
            // (Tables are interned: the same characters, the same table.)
            (Kind::Table(a), Kind::Table(b)) => std::ptr::eq(*a, *b),
            _ => false,
        }
    }
}

impl Eq for Kind {}

/// The character each byte stands for, and the byte of each character.
struct ByteTable {
    chars: [char; 256],
    bytes: HashMap<char, u8>,
    /// Whether the first 128 bytes are ASCII's characters.
    ascii: bool,
}

/// Every table met so far (a module's is the same one each time it opens).
static TABLES: Mutex<Vec<&'static ByteTable>> = Mutex::new(Vec::new());

impl Codepage {
    pub const WINDOWS_1252: Codepage = Codepage(Kind::Standard(encoding_rs::WINDOWS_1252));
    pub const WINDOWS_1250: Codepage = Codepage(Kind::Standard(encoding_rs::WINDOWS_1250));

    /// A codepage by its name (`windows-1252`, `cp1250`, `latin1`, ...).
    pub fn for_label(label: &str) -> Option<Codepage> {
        let label = label.trim();
        let label = match label.to_ascii_lowercase().strip_prefix("cp") {
            Some(n) if n.chars().all(|c| c.is_ascii_digit()) => format!("windows-{n}"),
            _ => label.to_string(),
        };
        Encoding::for_label(label.as_bytes()).map(|e| Codepage(Kind::Standard(e)))
    }

    /// A codepage of the character each of the 256 bytes stands for: a
    /// module's `encoding.2da`. A character two bytes stand for is written
    /// as the last of them, as the game writes it.
    pub fn table(chars: [char; 256]) -> Codepage {
        let mut tables = TABLES.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = tables.iter().find(|t| t.chars == chars) {
            return Codepage(Kind::Table(t));
        }
        let mut bytes = HashMap::new();
        for (b, c) in chars.iter().enumerate() {
            bytes.insert(*c, b as u8);
        }
        let ascii = chars[..128].iter().enumerate().all(|(b, c)| *c as usize == b);
        let t: &'static ByteTable = Box::leak(Box::new(ByteTable { chars, bytes, ascii }));
        tables.push(t);
        Codepage(Kind::Table(t))
    }

    /// The character each byte stands for; `None` for a codepage with
    /// characters of more than one byte.
    pub fn chars(&self) -> Option<[char; 256]> {
        match self.0 {
            Kind::Table(t) => Some(t.chars),
            Kind::Standard(e) if e.is_single_byte() => {
                let mut chars = ['\0'; 256];
                for (b, c) in chars.iter_mut().enumerate() {
                    *c = e.decode_without_bom_handling(&[b as u8]).0.chars().next()?;
                }
                Some(chars)
            }
            Kind::Standard(_) => None,
        }
    }

    /// Whether this is a module's own table.
    pub fn is_table(&self) -> bool {
        matches!(self.0, Kind::Table(_))
    }

    /// The codepage of a language's text where this one is the game's: a
    /// module's table for every language of one byte a character (it
    /// replaces the game's own table), the language's own otherwise.
    pub fn for_language(&self, language: Language) -> Codepage {
        let own = language.codepage();
        match (self.0, own.0) {
            (Kind::Table(_), Kind::Standard(e)) if e.is_single_byte() => *self,
            _ => own,
        }
    }

    /// `text` as `other` reads the bytes this codepage writes for it (a
    /// "?" for a character this one lacks): what to hand to code that
    /// writes in `other`, so that the bytes it writes are this one's.
    /// The text itself where the two are one codepage.
    pub fn spelled_in<'a>(&self, text: &'a str, other: Codepage) -> Cow<'a, str> {
        if *self == other {
            return Cow::Borrowed(text);
        }
        Cow::Owned(other.decode(&self.encode_lossy(text)).into_owned())
    }

    /// Encodes text to game bytes, a "?" for each character the codepage
    /// cannot represent.
    pub fn encode_lossy(&self, text: &str) -> Vec<u8> {
        match self.encode(text) {
            Some(b) => b.into_owned(),
            None => (text.chars())
                .flat_map(|c| self.encode(&c.to_string()).map_or(vec![b'?'], |b| b.into_owned()))
                .collect(),
        }
    }

    /// Decodes game bytes to text. Windows-1252 and -1250 map every byte, so
    /// for the EE languages this never loses information.
    pub fn decode<'a>(&self, bytes: &'a [u8]) -> Cow<'a, str> {
        match self.0 {
            Kind::Standard(e) => e.decode_without_bom_handling(bytes).0,
            Kind::Table(t) if t.ascii && bytes.is_ascii() => {
                Cow::Borrowed(std::str::from_utf8(bytes).expect("ASCII is UTF-8"))
            }
            Kind::Table(t) => Cow::Owned(bytes.iter().map(|b| t.chars[*b as usize]).collect()),
        }
    }

    /// Encodes text to game bytes. Returns `None` if the text has characters
    /// the codepage cannot represent.
    pub fn encode<'a>(&self, text: &'a str) -> Option<Cow<'a, [u8]>> {
        match self.0 {
            Kind::Standard(e) => {
                let (bytes, _, unmappable) = e.encode(text);
                (!unmappable).then_some(bytes)
            }
            Kind::Table(t) => text
                .chars()
                .map(|c| t.bytes.get(&c).copied())
                .collect::<Option<Vec<u8>>>()
                .map(Cow::Owned),
        }
    }
}

impl Default for Codepage {
    fn default() -> Self {
        Codepage::WINDOWS_1252
    }
}

impl fmt::Debug for Codepage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Kind::Standard(e) => write!(f, "Codepage({})", e.name()),
            Kind::Table(_) => write!(f, "Codepage(encoding.2da)"),
        }
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
        assert_eq!(Codepage::WINDOWS_1252.encode_lossy("dé日本"), b"d\xe9??");
    }

    #[test]
    fn a_table_of_a_codepage_s_own_characters_is_that_codepage() {
        let all: Vec<u8> = (0..=255).collect();
        for cp in [Codepage::WINDOWS_1252, Codepage::WINDOWS_1250] {
            let table = Codepage::table(cp.chars().unwrap());
            assert!(table.is_table() && table != cp);
            assert_eq!(table.decode(&all), cp.decode(&all), "{cp:?}");
            assert_eq!(table.encode(&cp.decode(&all)).unwrap().as_ref(), &all[..], "{cp:?}");
            assert_eq!(table.decode(b"plain"), "plain");
            assert!(table.encode("日本").is_none());
            // The same characters, the same table.
            assert_eq!(table, Codepage::table(cp.chars().unwrap()));
        }
        assert!(Language(131).codepage().chars().is_none());
    }

    #[test]
    fn a_table_s_changed_byte() {
        // The wiki's Turkish example: byte 240 is "ğ", not "ð".
        let mut chars = Codepage::WINDOWS_1252.chars().unwrap();
        chars[240] = 'ğ';
        let turkish = Codepage::table(chars);
        assert_eq!(turkish.decode(b"da\xf0"), "dağ");
        assert_eq!(turkish.encode("dağ").unwrap().as_ref(), b"da\xf0");
        assert!(turkish.encode("ð").is_none());
        assert_eq!(turkish.decode(b"\xe9"), "é");
        // It is the table of every language of one byte a character, and
        // of none of the others.
        assert_eq!(turkish.for_language(Language::ENGLISH), turkish);
        assert_eq!(turkish.for_language(Language::POLISH), turkish);
        assert_eq!(turkish.for_language(Language(131)), Language(131).codepage());
        let plain = Codepage::WINDOWS_1252;
        assert_eq!(plain.for_language(Language::POLISH), Codepage::WINDOWS_1250);
        assert_eq!(plain.for_language(Language::FRENCH), plain);
        // Text for code that writes Windows-1252, and text it read.
        assert_eq!(turkish.spelled_in("dağ é ð", plain), "dað é ?");
        assert_eq!(plain.spelled_in("dað é", turkish), "dağ é");
        assert!(matches!(plain.spelled_in("dağ", plain), Cow::Borrowed("dağ")));
        // A table that moves a letter of ASCII moves it in plain text too.
        let mut odd = Codepage::WINDOWS_1252.chars().unwrap();
        odd[b'a' as usize] = 'α';
        assert_eq!(Codepage::table(odd).decode(b"bad"), "bαd");
        // Two bytes of one character: the last writes it.
        let mut twice = Codepage::WINDOWS_1252.chars().unwrap();
        twice[250] = 'é';
        assert_eq!(Codepage::table(twice).encode("é").unwrap().as_ref(), b"\xfa");
    }

    #[test]
    fn language_metadata() {
        assert_eq!(Language::POLISH.codepage(), Codepage::WINDOWS_1250);
        assert_eq!(Language::GERMAN.short_code(), Some("de"));
        assert_eq!(Language(77).name(), None);
    }
}
