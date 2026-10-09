//! Localized strings (`CExoLocString`): a talk-table reference plus optional
//! per-language, per-gender overrides stored in the file itself.

use std::borrow::Cow;

use crate::lang::{Codepage, Gender, Language};
use crate::strref::StrRef;

/// Identifies one variant of a localized string: `language * 2 + gender`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocStringKey(pub u32);

impl LocStringKey {
    pub fn new(language: Language, gender: Gender) -> LocStringKey {
        LocStringKey(language.0 * 2 + gender as u32)
    }

    pub fn language(self) -> Language {
        Language(self.0 / 2)
    }

    pub fn gender(self) -> Gender {
        if self.0.is_multiple_of(2) { Gender::Male } else { Gender::Female }
    }
}

/// A localized string. The game shows the embedded variant for the player's
/// language if there is one, otherwise the talk-table string.
///
/// Variants keep their file order and their raw bytes (see [`Codepage`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct LocString {
    pub strref: StrRef,
    pub strings: Vec<(LocStringKey, Vec<u8>)>,
}

impl LocString {
    /// A string with only a talk-table reference.
    pub fn from_strref(strref: StrRef) -> LocString {
        LocString { strref, strings: Vec::new() }
    }

    /// A string with one embedded variant and no talk-table reference.
    pub fn from_text(language: Language, gender: Gender, text: impl Into<Vec<u8>>) -> LocString {
        LocString {
            strref: StrRef::NONE,
            strings: vec![(LocStringKey::new(language, gender), text.into())],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.strref.is_none() && self.strings.is_empty()
    }

    /// The embedded bytes for a variant.
    pub fn get(&self, language: Language, gender: Gender) -> Option<&[u8]> {
        let key = LocStringKey::new(language, gender);
        self.strings.iter().find(|(k, _)| *k == key).map(|(_, v)| v.as_slice())
    }

    /// The embedded variant decoded with the language's codepage.
    pub fn text(&self, language: Language, gender: Gender) -> Option<Cow<'_, str>> {
        self.get(language, gender).map(|b| language.codepage().decode(b))
    }

    /// [`text`](Self::text) where `game` is the game's codepage: a module's
    /// own table (`encoding.2da`) reads every language it stands for.
    pub fn text_in(
        &self,
        language: Language,
        gender: Gender,
        game: Codepage,
    ) -> Option<Cow<'_, str>> {
        self.get(language, gender).map(|b| game.for_language(language).decode(b))
    }

    /// The text to show where `language` has none: another language's,
    /// English if it has any, else the first variant with text; and the
    /// language it is in. (Aurora shows a builder the English text of a
    /// name written in English alone, whatever language it edits in.)
    pub fn elsewhere(&self, language: Language) -> Option<(Language, String)> {
        self.elsewhere_in(language, language.codepage())
    }

    /// [`elsewhere`](Self::elsewhere) where `game` is the game's codepage.
    pub fn elsewhere_in(&self, language: Language, game: Codepage) -> Option<(Language, String)> {
        let text = |l: Language, bytes: &[u8]| (l, game.for_language(l).decode(bytes).into_owned());
        let english = LocStringKey::new(Language::ENGLISH, Gender::Male);
        let others = self.strings.iter().filter(|(k, b)| k.language() != language && !b.is_empty());
        others
            .clone()
            .find(|(k, _)| *k == english)
            .or_else(|| others.clone().next())
            .map(|(k, b)| text(k.language(), b))
    }

    /// Sets a variant, replacing it in place if present, else appending it.
    pub fn set(&mut self, language: Language, gender: Gender, bytes: impl Into<Vec<u8>>) {
        let key = LocStringKey::new(language, gender);
        let bytes = bytes.into();
        match self.strings.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => *v = bytes,
            None => self.strings.push((key, bytes)),
        }
    }

    /// Sets a variant from text; `None` if the language's codepage cannot
    /// represent it.
    pub fn set_text(&mut self, language: Language, gender: Gender, text: &str) -> Option<()> {
        self.set_text_in(language, gender, text, language.codepage())
    }

    /// [`set_text`](Self::set_text) where `game` is the game's codepage.
    pub fn set_text_in(
        &mut self,
        language: Language,
        gender: Gender,
        text: &str,
        game: Codepage,
    ) -> Option<()> {
        let bytes = game.for_language(language).encode(text)?.into_owned();
        self.set(language, gender, bytes);
        Some(())
    }

    /// Removes a variant; returns whether it existed.
    pub fn remove(&mut self, language: Language, gender: Gender) -> bool {
        let key = LocStringKey::new(language, gender);
        let before = self.strings.len();
        self.strings.retain(|(k, _)| *k != key);
        self.strings.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_in_another_language_is_english_first() {
        let polish = Language(5);
        let mut s = LocString::default();
        assert_eq!(s.elsewhere(polish), None);
        s.set(Language::GERMAN, Gender::Male, b"Laden".to_vec());
        s.set(Language::ENGLISH, Gender::Male, b"Shop".to_vec());
        // English, though German comes first in the file.
        assert_eq!(s.elsewhere(polish), Some((Language::ENGLISH, "Shop".to_string())));
        // Not the language asked about, and not a variant without text.
        assert_eq!(s.elsewhere(Language::ENGLISH), Some((Language::GERMAN, "Laden".to_string())));
        s.set(Language::ENGLISH, Gender::Male, Vec::new());
        assert_eq!(s.elsewhere(polish), Some((Language::GERMAN, "Laden".to_string())));
    }

    #[test]
    fn keys() {
        let k = LocStringKey::new(Language::GERMAN, Gender::Female);
        assert_eq!(k.0, 5);
        assert_eq!(k.language(), Language::GERMAN);
        assert_eq!(k.gender(), Gender::Female);
    }

    #[test]
    fn set_replaces_in_place() {
        let mut s = LocString::from_text(Language::ENGLISH, Gender::Male, "a");
        s.set(Language::FRENCH, Gender::Male, "b");
        s.set(Language::ENGLISH, Gender::Male, "c");
        assert_eq!(s.strings.len(), 2);
        assert_eq!(s.strings[0].1, b"c");
        assert!(s.remove(Language::FRENCH, Gender::Male));
        assert!(!s.remove(Language::FRENCH, Gender::Male));
    }

    #[test]
    fn text_uses_language_codepage() {
        let mut s = LocString::default();
        s.set_text(Language::POLISH, Gender::Male, "Łódź").unwrap();
        assert_eq!(s.get(Language::POLISH, Gender::Male).unwrap(), &[0xa3, 0xf3, 0x64, 0x9f]);
        assert_eq!(s.text(Language::POLISH, Gender::Male).unwrap(), "Łódź");
        assert!(s.set_text(Language::ENGLISH, Gender::Male, "Łódź").is_none());
    }
}
