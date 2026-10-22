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

/// The codepage of one language's text of a localized string, as String
/// Edit reads and writes it: the language's, but with Polish the editing
/// language, the text of a Windows-1252 language (English, French...) is
/// Windows-1250 too. A Polish game shows such text by its own codepage,
/// and Polish modules keep Polish names in the English text (GitHub issue
/// 13: they showed as mojibake there, and Polish letters typed could not
/// be saved); the lists read it so already ([`elsewhere`]).
pub(crate) fn entry_codepage(language: Language) -> Codepage {
    let (own, edited) = (codepage_of(language), codepage_of(edit_language()));
    match own == Codepage::WINDOWS_1252 && edited == Codepage::WINDOWS_1250 {
        true => edited,
        false => own,
    }
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

/// The codepage of scripts' text: the editing language's (Options ›
/// Language), Windows-1250 for Polish. NWScript has no codepage of its
/// own: the compiler copies a string's bytes into the compiled script, and
/// the game reads them by its language's codepage, so Polish builders keep
/// their scripts in Windows-1250 (GitHub issue 27: they showed and saved
/// as mojibake, read as Windows-1252). A module's own table
/// (`encoding.2da`), where it has one.
pub(crate) fn script_codepage() -> Codepage {
    codepage_of(edit_language())
}

/// A script's bytes as text, in [`script_codepage`] (which maps every
/// byte: nothing is lost).
pub(crate) fn decode_script(bytes: &[u8]) -> String {
    script_codepage().decode(bytes).into_owned()
}

/// A script's text as bytes in [`script_codepage`]; characters it lacks
/// become `?`.
pub(crate) fn encode_script(text: &str) -> Vec<u8> {
    script_codepage().encode_lossy(text)
}

/// The characters of `text` that `codepage` cannot write, each once, in
/// the order met: what saving it would turn into `?`.
pub(crate) fn unwritable(codepage: Codepage, text: &str) -> Vec<char> {
    if codepage.encode(text).is_some() {
        return Vec::new();
    }
    let mut out: Vec<char> = Vec::new();
    for c in text.chars() {
        if !out.contains(&c) && codepage.encode(c.encode_utf8(&mut [0; 4])).is_none() {
            out.push(c);
        }
    }
    out
}

/// The warning for a script saved with characters its codepage cannot
/// write (they were written as `?`); `None` if it has none.
pub(crate) fn unwritable_warning(
    script: impl std::fmt::Display,
    codepage: Codepage,
    text: &str,
) -> Option<String> {
    let lacking = unwritable(codepage, text);
    if lacking.is_empty() {
        return None;
    }
    let shown: String = lacking.iter().take(12).collect();
    let more = if lacking.len() > 12 { "…" } else { "" };
    Some(format!(
        "{script}: {shown}{more} can't be written in the script's codepage ({}) and were \
         saved as ?. The game reads a script's text by its language's codepage (Options › \
         Language).",
        codepage_name(codepage)
    ))
}

/// A codepage's name, as builders know it.
pub(crate) fn codepage_name(codepage: Codepage) -> String {
    if codepage.is_table() {
        return "the module's encoding.2da".into();
    }
    // (`windows-1250` as `Windows-1250`.)
    let debug = format!("{codepage:?}");
    let name = debug.trim_start_matches("Codepage(").trim_end_matches(')');
    let mut chars = name.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
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

/// Whether a text's line ends are CRLF, as most of its are (BioWare's
/// scripts and Aurora's are); one with none, too, as Aurora writes.
pub(crate) fn uses_crlf(text: &str) -> bool {
    let all = text.matches('\n').count();
    all == 0 || text.matches("\r\n").count() * 2 >= all
}

/// An editor's text (plain newlines, [`to_editor`]) with the line ends of
/// `original`, the file's text it was read from: the lines before and
/// after those changed keep their own ends, and the others take the
/// file's ([`uses_crlf`]). An untouched text is the original again, byte
/// for byte, whatever its line ends; a file with both kinds keeps them
/// but on the lines changed.
pub(crate) fn with_line_ends(text: &str, original: &str) -> String {
    let crlf = uses_crlf(original);
    let old: Vec<&str> = original.split('\n').collect();
    let new: Vec<&str> = text.split('\n').collect();
    let (m, n) = (old.len(), new.len());
    // (Each segment but the last ends a line: its "\r", if any, is the
    // line end's, as `to_editor` took it.)
    let content =
        |k: usize| if k + 1 < m { old[k].strip_suffix('\r').unwrap_or(old[k]) } else { old[k] };
    let mut prefix = 0;
    while prefix + 1 < n && prefix + 1 < m && new[prefix] == content(prefix) {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < n - prefix
        && suffix < m - prefix
        && new[n - 1 - suffix] == content(m - 1 - suffix)
    {
        suffix += 1;
    }
    let mut out = String::with_capacity(text.len() + n);
    for (k, line) in new.iter().enumerate() {
        out.push_str(line);
        if k + 1 == n {
            break;
        }
        let kept = if k < prefix {
            Some(k)
        } else if k >= n - suffix {
            Some(k + m - n)
        } else if n == m {
            // (Lines changed in place, as many as there were: each keeps
            // the end it had, in a file of mixed ends too.)
            Some(k)
        } else {
            None
        };
        let end_crlf = kept.map_or(crlf, |o| old[o].ends_with('\r'));
        out.push_str(if end_crlf { "\r\n" } else { "\n" });
    }
    out
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

    /// Scripts are read in the editing language's codepage: Polish's
    /// letters round trip in Windows-1250, byte for byte, and those it
    /// lacks are named, each once.
    #[test]
    fn scripts_follow_the_editing_language_s_codepage() {
        set_game_codepage(None);
        set_edit_language(Language::POLISH);
        let text = "Zażółć gęślą jaźń. Źródło.";
        let bytes = Codepage::WINDOWS_1250.encode(text).unwrap().into_owned();
        assert_eq!(decode_script(&bytes), text);
        assert_eq!(encode_script(&decode_script(&bytes)), bytes);
        assert_eq!(unwritable(script_codepage(), "ąé日ñ日"), ['日', 'ñ']);
        assert!(unwritable_warning("x.nss", script_codepage(), text).is_none());
        let warning = unwritable_warning("x.nss", script_codepage(), "日").unwrap();
        assert!(warning.contains("Windows-1250"), "{warning}");
        set_edit_language(Language::ENGLISH);
        assert_eq!(script_codepage(), Codepage::WINDOWS_1252);
        assert_ne!(decode_script(&bytes), text);
    }

    #[test]
    fn line_ends() {
        let (shown, crlf) = to_editor("a\r\nb");
        assert_eq!((shown.as_str(), crlf), ("a\nb", true));
        assert_eq!(from_editor("a\nb\nc", true), "a\r\nb\r\nc");
        assert_eq!(from_editor("a\nb", false), "a\nb");
    }

    /// An editor's text goes back with the file's line ends: untouched,
    /// byte for byte (both kinds, a stray `\r`); edited, the lines kept
    /// keep theirs and the others take the file's (most of its lines').
    #[test]
    fn edited_text_takes_the_file_s_line_ends() {
        for file in [
            "void main()\r\n{\r\n}\r\n",
            "void main()\n{\n}\n",
            "a\r\nb\nc\r\nd",
            "a\r\r\nb\r",
            "",
            "\r\n",
            "x",
        ] {
            assert_eq!(with_line_ends(&to_editor(file).0, file), file, "{file:?}");
        }
        // CRLF everywhere, the new line too; LF stays LF.
        let crlf = "void main()\r\n{\r\n\r\n}\r\n";
        let typed = "void main()\n{\n    int n;\n    n = 1;\n}\n";
        assert_eq!(
            with_line_ends(typed, crlf),
            "void main()\r\n{\r\n    int n;\r\n    n = 1;\r\n}\r\n"
        );
        assert_eq!(with_line_ends(typed, &crlf.replace('\r', "")), typed);
        // Both kinds: the lines before and after the change keep theirs.
        let mixed = "a\r\nb\nc\r\nd\r\n";
        assert_eq!(with_line_ends("a\nb\nX\nY\nd\n", mixed), "a\r\nb\nX\r\nY\r\nd\r\n");
        // Lines changed in place keep their own ends: an LF line of a CRLF
        // file stays LF (a builder's git diff of a whole script).
        let mostly = "a\r\nb\nc\r\nd\r\n";
        assert_eq!(with_line_ends("a\nB\nc\nd\n", mostly), "a\r\nB\nc\r\nd\r\n");
        // No line ends at all: CRLF, as Aurora writes.
        assert!(uses_crlf("void main() {}"));
        assert!(!uses_crlf("a\nb\r\nc\n"));
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
