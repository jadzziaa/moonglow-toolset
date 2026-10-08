//! Game text in the UI: bytes in the game codepage to and from UTF-8.

use std::cell::Cell;

use mg_core::{Codepage, Gender, Language, LocString};

thread_local! {
    /// The language editors show and change (Options › Language): English
    /// unless another is chosen. The UI thread's.
    static EDIT_LANGUAGE: Cell<u32> = const { Cell::new(0) };
}

thread_local! {
    /// The codepage of the open module's game: its language's, or the
    /// table of an `encoding.2da` in its haks. The UI thread's, set each
    /// frame (and in the threads of its jobs).
    static GAME_CODEPAGE: Cell<Option<Codepage>> = const { Cell::new(None) };
}

/// The codepage of the game's text other than a localized string's: a
/// module's own table, else Windows-1252.
pub(crate) fn game_codepage() -> Codepage {
    GAME_CODEPAGE.with(Cell::get).unwrap_or_default().for_language(Language::ENGLISH)
}

/// The codepage of a language's text in the open module's game.
pub(crate) fn codepage_of(language: Language) -> Codepage {
    GAME_CODEPAGE.with(Cell::get).unwrap_or_default().for_language(language)
}

/// Sets the game's codepage (for the calling thread), as
/// `GameData::codepage` has it; `None` without a game.
pub(crate) fn set_game_codepage(codepage: Option<Codepage>) {
    GAME_CODEPAGE.with(|c| c.set(codepage));
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
    let language = edit_language();
    ls.text_in(language, Gender::Male, codepage_of(language))
        .map(|t| t.into_owned())
        .unwrap_or_default()
}

/// A localized string's text for a list or a title: in the editing
/// language, else, where it has none in it, in another (English first).
pub(crate) fn shown_text(ls: &LocString) -> String {
    let text = edited_text(ls);
    if !text.is_empty() {
        return text;
    }
    elsewhere(ls).map(|(_, text)| text).unwrap_or_default()
}

/// A localized string's text in another language than the editing one
/// (English first), and the language: [`LocString::elsewhere`], read as
/// the open module's game reads it.
pub(crate) fn elsewhere(ls: &LocString) -> Option<(Language, String)> {
    let language = edit_language();
    ls.elsewhere_in(language, GAME_CODEPAGE.with(Cell::get).unwrap_or(language.codepage()))
}

/// Game bytes as text (Windows-1252, or the module's own table: either
/// maps every byte, so nothing is lost).
pub(crate) fn decode(bytes: &[u8]) -> String {
    game_codepage().decode(bytes).into_owned()
}

/// Text as game bytes; characters the codepage lacks become `?`.
pub(crate) fn encode(text: &str) -> Vec<u8> {
    encode_in(game_codepage(), text)
}

/// Text as bytes in a codepage; characters it lacks become `?`.
fn encode_in(codepage: Codepage, text: &str) -> Vec<u8> {
    codepage.encode_lossy(text)
}

/// Text for code that writes Windows-1252 whatever the module (a
/// conversation the Store Wizard or an import makes): spelled so that the
/// bytes it writes are the module's own table's. The text itself without
/// a table.
pub(crate) fn for_windows_1252(text: &str) -> String {
    game_codepage().spelled_in(text, Codepage::WINDOWS_1252).into_owned()
}

/// Text that code read as Windows-1252 (a conversation's export), as the
/// module's own table reads the same bytes.
pub(crate) fn from_windows_1252(text: &str) -> String {
    Codepage::WINDOWS_1252.spelled_in(text, game_codepage()).into_owned()
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
        // text is Windows-1250, and its "ł" was a "?" in Windows-1252.
        // A module's own table writes them all.)
        s.set(language, Gender::Male, encode_in(codepage_of(language), text));
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

    #[test]
    fn a_module_s_own_table() {
        // The wiki's Turkish example: byte 240 is "ğ".
        let mut chars = Codepage::WINDOWS_1252.chars().unwrap();
        chars[240] = 'ğ';
        set_game_codepage(Some(Codepage::table(chars)));
        assert_eq!(encode("dağ"), b"da\xf0");
        assert_eq!(decode(b"da\xf0 \xe9"), "dağ é");
        assert_eq!(encode("ð"), b"?");
        let s = with_english(LocString::default(), "Dağ");
        assert_eq!(s.get(Language::ENGLISH, Gender::Male).unwrap(), b"Da\xf0");
        assert_eq!(edited_text(&s), "Dağ");
        // Another language's text is read by it too.
        let german = LocString::from_text(Language::GERMAN, Gender::Male, b"Da\xf0".to_vec());
        assert_eq!(shown_text(&german), "Dağ");
        // Code that writes Windows-1252 (a conversation the Store Wizard or
        // an import makes) is handed text that comes out as the table's
        // bytes, and what it read is read back by the table.
        let line = mg_module::dialog::new_node(
            mg_module::dialog::Kind::Entry,
            &for_windows_1252("Dağ é ð"),
        );
        let text = line.locstring("Text").unwrap();
        assert_eq!(text.get(Language::ENGLISH, Gender::Male).unwrap(), b"Da\xf0 \xe9 ?");
        assert_eq!(from_windows_1252("Dað"), "Dağ");
        // Without it, all is as it was: Polish in its own codepage.
        set_game_codepage(None);
        assert_eq!(for_windows_1252("Dağ é ð"), "Dağ é ð");
        assert_eq!(from_windows_1252("Dað"), "Dað");
        assert_eq!(decode(b"da\xf0"), "dað");
        set_edit_language(Language::POLISH);
        let s = with_english(LocString::default(), "Kryształowa czaszka");
        assert_eq!(edited_text(&s), "Kryształowa czaszka");
        assert_eq!(s.get(Language::POLISH, Gender::Male).unwrap(), b"Kryszta\xb3owa czaszka");
        set_edit_language(Language::ENGLISH);
    }
}
