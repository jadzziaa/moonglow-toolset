//! Game text in the UI: bytes in the game codepage to and from UTF-8.

use std::cell::Cell;

use mg_core::{Codepage, Gender, Language, LocString};

thread_local! {
    /// The language editors show and change (Options › Language): English
    /// unless another is chosen. The UI thread's.
    static EDIT_LANGUAGE: Cell<u32> = const { Cell::new(0) };
}

/// The editing language.
pub(crate) fn edit_language() -> Language {
    Language(EDIT_LANGUAGE.with(Cell::get))
}

/// Chooses the editing language (for the calling thread: the UI's).
pub fn set_edit_language(language: Language) {
    EDIT_LANGUAGE.with(|l| l.set(language.0));
}

/// A localized string's text in the editing language (empty if none).
pub(crate) fn edited_text(ls: &LocString) -> String {
    ls.text(edit_language(), Gender::Male).map(|t| t.into_owned()).unwrap_or_default()
}

/// A localized string's text for a list or a title: in the editing
/// language, else, where it has none in it, in another (English first).
pub(crate) fn shown_text(ls: &LocString) -> String {
    let text = edited_text(ls);
    if !text.is_empty() {
        return text;
    }
    ls.elsewhere(edit_language()).map(|(_, text)| text).unwrap_or_default()
}

/// Game bytes as text (Windows-1252 maps every byte, so nothing is lost).
pub(crate) fn decode(bytes: &[u8]) -> String {
    Codepage::WINDOWS_1252.decode(bytes).into_owned()
}

/// Text as game bytes; characters the codepage lacks become `?`.
pub(crate) fn encode(text: &str) -> Vec<u8> {
    encode_in(Codepage::WINDOWS_1252, text)
}

/// Text as bytes in a codepage; characters it lacks become `?`.
fn encode_in(codepage: Codepage, text: &str) -> Vec<u8> {
    match codepage.encode(text) {
        Some(b) => b.into_owned(),
        None => text
            .chars()
            .flat_map(|c| codepage.encode(&c.to_string()).map_or(vec![b'?'], |b| b.into_owned()))
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

/// A localized string with its text in the editing language replaced (or
/// set; English unless Options › Language chooses another).
pub(crate) fn with_english(mut s: LocString, text: &str) -> LocString {
    let language = edit_language();
    if text.is_empty() {
        s.remove(language, Gender::Male);
    } else {
        // (In the language's own codepage, as it is read back: Polish
        // text is Windows-1250, and its "ł" was a "?" in Windows-1252.)
        s.set(language, Gender::Male, encode_in(language.codepage(), text));
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
        // Another language's text in its own codepage.
        set_edit_language(Language::POLISH);
        let s = with_english(LocString::default(), "Kryształowa czaszka");
        set_edit_language(Language::ENGLISH);
        assert_eq!(s.text(Language::POLISH, Gender::Male).unwrap(), "Kryształowa czaszka");
    }
}
