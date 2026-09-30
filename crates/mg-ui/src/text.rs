//! Game text in the UI: bytes in the game codepage to and from UTF-8.

use mg_core::{Codepage, Gender, Language, LocString};

/// Game bytes as text (Windows-1252 maps every byte, so nothing is lost).
pub(crate) fn decode(bytes: &[u8]) -> String {
    Codepage::WINDOWS_1252.decode(bytes).into_owned()
}

/// Text as game bytes; characters the codepage lacks become `?`.
pub(crate) fn encode(text: &str) -> Vec<u8> {
    match Codepage::WINDOWS_1252.encode(text) {
        Some(b) => b.into_owned(),
        None => text
            .chars()
            .map(|c| Codepage::WINDOWS_1252.encode(&c.to_string()).map_or(b'?', |b| b[0]))
            .collect(),
    }
}

/// Text for an editor: CRLF line ends shown as plain newlines. Returns
/// whether the original used CRLF, to restore it with [`from_editor`].
pub(crate) fn to_editor(text: &str) -> (String, bool) {
    (text.replace("\r\n", "\n"), text.contains("\r\n"))
}

/// Editor text back in the original's line-end style.
pub(crate) fn from_editor(text: &str, crlf: bool) -> String {
    if crlf { text.replace("\r\n", "\n").replace('\n', "\r\n") } else { text.to_string() }
}

/// A localized string with its English text replaced (or set).
pub(crate) fn with_english(mut s: LocString, text: &str) -> LocString {
    if text.is_empty() {
        s.remove(Language::ENGLISH, Gender::Male);
    } else {
        s.set(Language::ENGLISH, Gender::Male, encode(text));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ends() {
        let (shown, crlf) = to_editor("a\r\nb");
        assert_eq!((shown.as_str(), crlf), ("a\nb", true));
        assert_eq!(from_editor("a\nb\nc", true), "a\r\nb\r\nc");
        assert_eq!(from_editor("a\nb", false), "a\nb");
    }

    #[test]
    fn round_trip_and_fallback() {
        assert_eq!(decode(&encode("Épée")), "Épée");
        assert_eq!(encode("a→b"), b"a?b");
        let s = with_english(LocString::default(), "Hi");
        assert_eq!(s.text(Language::ENGLISH, Gender::Male).unwrap(), "Hi");
        assert!(with_english(s, "").strings.is_empty());
    }
}
