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
/// module's own table, else Windows-1252 (`GameData::text_codepage`, as
/// `mg` and plugins read it).
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

/// Names in a line: all of them where there are `room` or fewer, else the
/// first and how many more, in `room` places ("a, b, c and 3 more" for
/// four): never "and 1 more".
pub(crate) fn some_of(names: &[impl AsRef<str>], room: usize) -> String {
    let all = || names.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    if names.len() <= room.max(1) {
        return all().join(", ");
    }
    let shown = room.max(2) - 1;
    format!("{} and {} more", all()[..shown].join(", "), names.len() - shown)
}

/// Text for an editor: CRLF line ends shown as plain newlines (a `\r`
/// the field kept would be a stop of the caret no one sees, and End put
/// what is typed after it). Returns whether the original used CRLF. What
/// is typed goes back with the original's line ends by [`with_line_ends`].
pub(crate) fn to_editor(text: &str) -> (String, bool) {
    (text.replace("\r\n", "\n"), text.contains("\r\n"))
}

/// Whether a text's line ends are CRLF, as most of its are (BioWare's
/// scripts and Aurora's are); one with none, too, as Aurora writes a
/// script or a `CExoString`.
pub(crate) fn uses_crlf(text: &str) -> bool {
    let all = text.matches('\n').count();
    all == 0 || text.matches("\r\n").count() * 2 >= all
}

/// An editor's text (plain newlines, [`to_editor`]) with the line ends of
/// `original`, the text it was read from (a file's, a GFF field's): every
/// line not changed keeps its own end, found by a diff of the lines; a
/// line changed in place (between two kept, as many as there were) keeps
/// the end it had; a line added takes the text's ([`uses_crlf`]: most of
/// its lines', CRLF with none, as Aurora writes a script or a GFF field's
/// `CExoString`, a comment). An untouched text is the original again, byte
/// for byte, whatever its line ends. A localized string's text:
/// [`with_loc_line_ends`].
pub(crate) fn with_line_ends(text: &str, original: &str) -> String {
    with_line_ends_or(text, original, true)
}

/// [`with_line_ends`] for the text of a localized string (`CExoLocString`:
/// a description, a name, a conversation line's text): a text of one line
/// given a second gets a plain line end, as Aurora writes one typed there
/// (its comments, `CExoString`s, get CRLF), and as a talk table has it.
pub(crate) fn with_loc_line_ends(text: &str, original: &str) -> String {
    with_line_ends_or(text, original, false)
}

/// [`with_line_ends`], with `crlf` the line end of a text that has none
/// yet: plain for a localized string's or a talk table's (the game's own
/// `dialog.tlk` has no CRLF at all), CRLF for a file's or a `CExoString`'s.
pub(crate) fn with_line_ends_or(text: &str, original: &str, crlf: bool) -> String {
    let crlf = if original.contains('\n') { uses_crlf(original) } else { crlf };
    let old: Vec<&str> = original.split('\n').collect();
    let new: Vec<&str> = text.split('\n').collect();
    let m = old.len();
    // (Each segment but the last ends a line: its "\r", if any, is the
    // line end's, as `to_editor` took it.)
    let content: Vec<&str> = (old.iter().enumerate())
        .map(|(k, s)| if k + 1 < m { s.strip_suffix('\r').unwrap_or(s) } else { s })
        .collect();
    let kept = kept_lines(&content, &new);
    let mut out = String::with_capacity(text.len() + new.len());
    for (k, line) in new.iter().enumerate() {
        out.push_str(line);
        if k + 1 == new.len() {
            break;
        }
        // (The old last line had no end of its own.)
        let end_crlf = match kept[k] {
            Some(o) if o + 1 < m => old[o].ends_with('\r'),
            _ => crlf,
        };
        out.push_str(if end_crlf { "\r\n" } else { "\n" });
    }
    out
}

/// The most cells of one table of [`common_lines`]: a run of changed lines
/// bigger than that (thousands each side, a script rewritten whole) is cut
/// at the lines each side has once first.
const DIFF_CELLS: usize = 4_000_000;

/// The most work of one diff ([`kept_lines`]): the cells of its tables and
/// the lines looked at for the lines each side has once. What is left
/// past it is taken as changed in place, so a diff takes a bounded time
/// however the text was changed.
const DIFF_WORK: usize = 4 * DIFF_CELLS;

/// For each line of `new`, the line of `old` it is: the same text, by a
/// diff of the lines ([`common_lines`]), or among the lines changed between
/// two kept ones, the old line in its place (the first lines of the run, as
/// many as both have). `None`: a line added.
fn kept_lines(old: &[&str], new: &[&str]) -> Vec<Option<usize>> {
    let (m, n) = (old.len(), new.len());
    let mut prefix = 0;
    while prefix < m && prefix < n && old[prefix] == new[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < m - prefix && suffix < n - prefix && old[m - 1 - suffix] == new[n - 1 - suffix] {
        suffix += 1;
    }
    let (a, b) = (&old[prefix..m - suffix], &new[prefix..n - suffix]);
    // The lines of the changed run that are the same in both, in order.
    let mut same = Vec::new();
    let mut work = DIFF_WORK;
    common_lines(a, b, (0, 0), &mut work, &mut same);
    same.push((a.len(), b.len()));
    let mut out = vec![None; n];
    for (k, o) in out.iter_mut().enumerate().take(prefix) {
        *o = Some(k);
    }
    for k in 0..suffix {
        out[n - 1 - k] = Some(m - 1 - k);
    }
    let (mut i0, mut j0) = (0, 0);
    for (i, j) in same {
        for d in 0..(i - i0).min(j - j0) {
            out[prefix + j0 + d] = Some(prefix + i0 + d);
        }
        if j < b.len() {
            out[prefix + j] = Some(prefix + i);
        }
        (i0, j0) = (i + 1, j + 1);
    }
    out
}

/// The lines `a` and `b` have in common, in order, pushed onto `same` as
/// pairs of places (`at` added: where the two begin in the run): by the
/// table of their longest common subsequence if it has at most
/// [`DIFF_CELLS`] cells; else cut at the lines each has once and both have,
/// in the order both have them (a patience diff), the lines between two
/// such taken the same way, their equal ends first. `work`: what is left
/// of [`DIFF_WORK`]; lines past it are left out (changed).
fn common_lines(
    a: &[&str],
    b: &[&str],
    at: (usize, usize),
    work: &mut usize,
    same: &mut Vec<(usize, usize)>,
) {
    let cells = (a.len() + 1).saturating_mul(b.len() + 1);
    if cells <= DIFF_CELLS && cells <= *work {
        *work -= cells;
        common_by_table(a, b, at, same);
        return;
    }
    if a.len() + b.len() > *work {
        return;
    }
    *work -= a.len() + b.len();
    let anchors = unique_common(a, b);
    if anchors.is_empty() {
        // (No line to cut at: the lines are taken as changed.)
        return;
    }
    let (mut i0, mut j0) = (0, 0);
    for (i, j) in anchors.into_iter().chain([(a.len(), b.len())]) {
        // (Between two lines found once: the lines both begin and end
        // with, then the rest.)
        let (ga, gb) = (&a[i0..i], &b[j0..j]);
        let mut p = 0;
        while p < ga.len() && p < gb.len() && ga[p] == gb[p] {
            same.push((at.0 + i0 + p, at.1 + j0 + p));
            p += 1;
        }
        let mut s = 0;
        while s < ga.len() - p && s < gb.len() - p && ga[ga.len() - 1 - s] == gb[gb.len() - 1 - s] {
            s += 1;
        }
        let inner = (&ga[p..ga.len() - s], &gb[p..gb.len() - s]);
        common_lines(inner.0, inner.1, (at.0 + i0 + p, at.1 + j0 + p), work, same);
        for k in (0..s).rev() {
            same.push((at.0 + i - 1 - k, at.1 + j - 1 - k));
        }
        if i < a.len() {
            same.push((at.0 + i, at.1 + j));
        }
        (i0, j0) = (i + 1, j + 1);
    }
}

/// [`common_lines`] by the table of the longest common subsequence of `a`
/// and `b`, `(a.len() + 1) × (b.len() + 1)` cells.
fn common_by_table(a: &[&str], b: &[&str], at: (usize, usize), same: &mut Vec<(usize, usize)>) {
    // (Each cell: the longest common subsequence of a[i..] and b[j..].)
    let w = b.len() + 1;
    let mut len = vec![0u32; (a.len() + 1) * w];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            len[i * w + j] = if a[i] == b[j] {
                len[(i + 1) * w + j + 1] + 1
            } else {
                len[(i + 1) * w + j].max(len[i * w + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            same.push((at.0 + i, at.1 + j));
            (i, j) = (i + 1, j + 1);
        } else if len[(i + 1) * w + j] >= len[i * w + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
}

/// The lines found once in `a` and once in `b`, as places in both, the most
/// of them that are in the same order in both (the longest increasing run
/// of their places in `b`, by patience sorting).
fn unique_common(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    // (A line's count and last place in each.)
    let mut seen: std::collections::HashMap<&str, [(u32, usize); 2]> =
        std::collections::HashMap::new();
    for (side, lines) in [a, b].into_iter().enumerate() {
        for (k, line) in lines.iter().enumerate() {
            let e = &mut seen.entry(line).or_default()[side];
            *e = (e.0 + 1, k);
        }
    }
    let mut pairs: Vec<(usize, usize)> = (seen.into_values())
        .filter(|[x, y]| x.0 == 1 && y.0 == 1)
        .map(|[x, y]| (x.1, y.1))
        .collect();
    pairs.sort_unstable();
    // Patience sorting: `tops[k]`, the pair ending the best run of k + 1
    // found so far (the one whose place in `b` is least); `back`, each
    // pair's predecessor in its run.
    let mut tops: Vec<usize> = Vec::new();
    let mut back = vec![usize::MAX; pairs.len()];
    for (k, &(_, j)) in pairs.iter().enumerate() {
        let pile = tops.partition_point(|&t| pairs[t].1 < j);
        if pile > 0 {
            back[k] = tops[pile - 1];
        }
        if pile == tops.len() {
            tops.push(k);
        } else {
            tops[pile] = k;
        }
    }
    let mut run = Vec::with_capacity(tops.len());
    let mut k = tops.last().copied().unwrap_or(usize::MAX);
    while k != usize::MAX {
        run.push(pairs[k]);
        k = back[k];
    }
    run.reverse();
    run
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
    fn a_few_names_and_how_many_more() {
        assert_eq!(some_of(&["a", "b"], 4), "a, b");
        assert_eq!(some_of(&["a", "b", "c", "d"], 4), "a, b, c, d");
        assert_eq!(some_of(&["a", "b", "c", "d", "e", "f"], 4), "a, b, c and 3 more");
        assert_eq!(some_of(&["a", "b", "c"], 1), "a and 2 more");
        assert_eq!(some_of(&[String::from("a")], 0), "a");
        assert_eq!(some_of(&[] as &[&str], 4), "");
    }

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
        assert_eq!(with_line_ends("a\nb\nc", "a\r\nb"), "a\r\nb\r\nc");
        assert_eq!(with_line_ends("a\nb", "a\nb"), "a\nb");
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

    /// In a file of both kinds, every line not changed keeps its own line
    /// end, wherever the changes are (Jasperre's report): those between
    /// two changes too, a line added or taken away among them.
    #[test]
    fn unchanged_lines_between_changes_keep_their_ends() {
        let mixed = "a\r\nb\nc\r\nd\r\n";
        // A line changed, an LF line kept, a line added at the end.
        assert_eq!(with_line_ends("A\nb\nc\nd\nE\n", mixed), "A\r\nb\nc\r\nd\r\nE\r\n");
        // A line added between two kept ones takes the file's.
        assert_eq!(with_line_ends("a\nb\nnew\nc\nd\n", mixed), "a\r\nb\nnew\r\nc\r\nd\r\n");
        // Lines taken away: those left keep theirs.
        let five = "a\r\nb\nc\r\nd\ne\r\n";
        assert_eq!(with_line_ends("b\nc\ne\n", five), "b\nc\r\ne\r\n");
        // A line changed and one added in its place: the changed one keeps
        // the end it had, the added one takes the file's.
        assert_eq!(with_line_ends("a\nB\nB2\nc\nd\n", mixed), "a\r\nB\nB2\r\nc\r\nd\r\n");
        // Two lines swapped: the diff keeps one (`c`, its own end), and
        // the other is taken for added (the file's).
        assert_eq!(with_line_ends("a\nc\nb\nd\n", mixed), "a\r\nc\r\nb\r\nd\r\n");
        // The last line, which had no end, gets the file's once one follows.
        assert_eq!(with_line_ends("a\nb\nc", "a\nb"), "a\nb\nc");
        assert_eq!(with_line_ends("x\ny\nz", "x\r\ny"), "x\r\ny\r\nz");
    }

    /// The line ends of a text with none yet: CRLF for a file's or a
    /// GFF field's ([`with_line_ends`]), plain for a talk table's.
    #[test]
    fn a_text_without_line_ends_takes_its_kind_s() {
        assert_eq!(with_line_ends("a\nb", "a"), "a\r\nb");
        assert_eq!(with_line_ends_or("a\nb", "a", false), "a\nb");
        assert_eq!(with_line_ends_or("a\nb\nc", "a\r\nb", false), "a\r\nb\r\nc");
        assert_eq!(with_line_ends_or("one\ntwo", "one\ntwo", false), "one\ntwo");
    }

    /// A text changed past what the diff takes on (a script rewritten
    /// whole) still keeps the ends of the lines before and after the
    /// changes, and of those changed in place.
    #[test]
    fn a_large_change_keeps_the_ends_around_it() {
        let old: String = (0..3000).map(|k| format!("{k}{}", ["\r\n", "\n"][k % 2])).collect();
        let new: String = (0..3000).map(|k| format!("x{k}\n")).collect();
        let out = with_line_ends(&new, &old);
        let expected: String =
            (0..3000).map(|k| format!("x{k}{}", ["\r\n", "\n"][k % 2])).collect();
        assert_eq!(out, expected);
    }

    /// A large change of a text of both kinds (past what one table of the
    /// diff takes on: thousands of lines each side between the first change
    /// and the last), lines added among the lines kept: each line kept
    /// keeps its own end, found by the lines each side has once.
    #[test]
    fn a_large_change_keeps_each_kept_line_s_end() {
        let end = |k: usize| ["\r\n", "\n", "\n"][k % 3];
        let old: String = (0..6000).map(|k| format!("line {k}{}", end(k))).collect();
        let mut new = Vec::new();
        for k in 0..6000 {
            match k {
                // Changed near both ends, so that the run between is long.
                3 | 5996 => new.push(format!("changed {k}")),
                // Lines added among the kept, others taken away.
                _ if k % 397 == 0 => new.extend([format!("added {k}"), format!("line {k}")]),
                _ if k % 541 == 0 => {}
                _ => new.push(format!("line {k}")),
            }
        }
        let new = new.join("\n") + "\n";
        let out = with_line_ends(&new, &old);
        assert_eq!(out.replace("\r\n", "\n"), new);
        for line in out.split_inclusive('\n') {
            if let Some(k) = line.strip_prefix("line ") {
                let k: usize = k.trim_end().parse().unwrap();
                assert_eq!(line.ends_with("\r\n"), end(k) == "\r\n", "{line:?}");
            }
        }
    }

    /// A part of a large change with no line found once on each side, too
    /// big for one table, is taken as changed at once: the rest of the
    /// change is still diffed (it was diffed again whole, a level a line,
    /// until the diff's bound of work was spent, and the lines after it
    /// were matched by place).
    #[test]
    fn a_part_without_lines_found_once_leaves_the_rest_diffed() {
        let alike = |phase: usize| (0..2100).map(move |k| ["{", "}", ""][(k + phase) % 3]);
        // After "mid": lines alike but for their ends, one added before.
        let tail = (0..200).map(|k| ["x\r\n", "y\n"][k % 2]);
        let old: String = ["begin a\n".to_string()]
            .into_iter()
            .chain(alike(0).map(|l| format!("{l}\n")))
            .chain(["mid\n".into()])
            .chain(tail.clone().map(String::from))
            .chain(["end a".into()])
            .collect();
        let new: String = ["begin b\n".to_string()]
            .into_iter()
            .chain(alike(1).map(|l| format!("{l}\n")))
            .chain(["mid\n".into(), "z\n".into()])
            .chain(tail.clone().map(|l| l.replace('\r', "")))
            .chain(["end b".into()])
            .collect();
        let out = with_line_ends(&new, &old);
        // ("z" added: the text's line end, plain as most of its lines'.)
        let after = out.split_once("mid\nz\n").expect(&out[..200]).1;
        let expected: String = tail.collect::<String>() + "end b";
        assert_eq!(after, expected);
    }

    /// The diff takes a bounded time however large the text and its
    /// change: lines all different, lines all alike, every other line
    /// changed, the lines reversed.
    #[test]
    fn a_diff_takes_a_bounded_time() {
        let n = 200_000;
        let unique: Vec<String> = (0..n).map(|k| format!("int n{k} = {k};")).collect();
        let alike: Vec<String> = (0..n).map(|k| ["{", "}", ""][k % 3].to_string()).collect();
        let every_other: Vec<String> = (unique.iter().enumerate())
            .map(|(k, l)| if k % 2 == 0 { format!("{l} // x") } else { l.clone() })
            .collect();
        let reversed: Vec<String> = unique.iter().rev().cloned().collect();
        let other: Vec<String> = (0..n).map(|k| ["}", "", "{", "{"][k % 4].to_string()).collect();
        for (old, new) in
            [(&unique, &every_other), (&unique, &reversed), (&alike, &other), (&unique, &alike)]
        {
            let (old, new) = (old.join("\r\n"), new.join("\n"));
            let start = std::time::Instant::now();
            let out = with_line_ends(&new, &old);
            // (Generous: an unoptimized build, other tests at once.)
            assert!(start.elapsed().as_secs_f32() < 20.0, "{:?}", start.elapsed());
            assert_eq!(out.replace("\r\n", "\n"), new);
        }
    }

    /// A line typed into a text of one line gets the line end Aurora
    /// writes for one typed in its field (capture `line-ends/typed.mod`:
    /// docs/findings.md, "Line ends"): plain in a localized string (a
    /// placeable's Description, the module's, a conversation line's Text),
    /// CRLF in a `CExoString` (a comment).
    #[test]
    fn a_new_line_takes_the_line_end_aurora_writes() {
        use mg_core::{ResRef, ResType};
        let path = mg_testkit::aurora_capture!("line-ends/typed.mod");
        let bytes = std::fs::read(path).unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        let gff = |name: &str, ty: ResType| {
            let data = erf.get(&ResRef::from_str(name).unwrap(), ty).unwrap().unwrap();
            mg_gff::Gff::read(&data).unwrap()
        };
        let english = |ls: Option<&LocString>| {
            let ls = ls.unwrap();
            String::from_utf8(ls.get(Language::ENGLISH, Gender::Male).unwrap().to_vec()).unwrap()
        };
        let dlg = gff("capdlg", ResType::DLG);
        let line = &dlg.root.list("EntryList").unwrap()[0];
        let utp = gff("mgp_utp", ResType::UTP);
        let ifo = gff("module", ResType::IFO);
        let string = |s: &mg_gff::Struct| String::from_utf8(s.string("Comment").unwrap().to_vec());
        let localized = [
            english(line.locstring("Text")),
            english(ifo.root.locstring("Mod_Description")),
            english(utp.root.locstring("Description")),
        ];
        let comments = [string(line).unwrap()];
        for (fields, loc) in [(&localized[..], true), (&comments[..], false)] {
            let ours = if loc { with_loc_line_ends } else { with_line_ends };
            for aurora in fields {
                // The text was one line before; a second typed after it.
                let first = &aurora[..aurora.find(['\r', '\n']).expect(aurora)];
                let second = aurora[first.len()..].trim_start_matches(['\r', '\n']);
                let typed = format!("{first}\n{second}");
                assert_eq!(ours(&typed, first), *aurora, "{aurora:?}");
            }
        }
        assert!(localized.iter().all(|t| !t.contains('\r')) && comments[0].contains("\r\n"));
        // (A blueprint's comment, its line end plain before, is CRLF once
        // Aurora saves the blueprint, untouched.)
        assert_eq!(string(&utp.root).unwrap(), "lf one\r\nlf two");
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
