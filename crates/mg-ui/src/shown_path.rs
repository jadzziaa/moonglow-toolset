//! A path's text as panels show it cut or broken at its separators:
//! never inside a folder's name, and always with the path's own
//! separators (`\` on Windows, where a path may mix in `/`, which Windows
//! takes too: a test's `…\mg-testkit\../../target\…`), as the rest of the
//! program shows a path (`Path::display`). The functions take whether the
//! path is Windows', so that their tests run on any system; the panels
//! pass [`WINDOWS`].

/// Whether paths here are Windows' (`\` and `/` both separate their parts).
pub(crate) const WINDOWS: bool = cfg!(windows);

/// Whether `c` separates a path's parts: `/`, and `\` in a Windows path.
pub(crate) fn separates(c: char, windows: bool) -> bool {
    c == '/' || (windows && c == '\\')
}

/// The ends of `text` from each of its separators but a first one on,
/// longest first, each with the separator before it, as it is in the
/// text: `\b/c.hak` and `/c.hak` for `a\b/c.hak` (Windows').
fn ends(text: &str, windows: bool) -> impl Iterator<Item = &str> {
    text.char_indices()
        .filter(move |&(i, c)| i > 0 && separates(c, windows))
        .map(|(i, _)| &text[i..])
}

/// `text` with whole folders dropped from its start, the fewest for what
/// is left to `fit`, after "…": `…\data\hk\id_resources.hak`. `None`
/// where not even its last part fits after "…" (or it has but one part).
pub(crate) fn cut_folders(text: &str, windows: bool, fit: impl Fn(&str) -> bool) -> Option<String> {
    ends(text, windows).map(|end| format!("…{end}")).find(|s| fit(s))
}

/// `text`'s last `parts` parts after "…", or the whole where it has no
/// more than that: `…\Neverwinter Nights\lang\en\data\dialog.tlk`.
pub(crate) fn last_parts(text: &str, parts: usize, windows: bool) -> String {
    let ends: Vec<&str> = ends(text, windows).collect();
    match parts > 0 && ends.len() >= parts {
        true => format!("…{}", ends[ends.len() - parts]),
        false => text.to_string(),
    }
}

/// `text` broken into lines after its separators, each line as many of
/// its parts as `fit` (a part that doesn't fit a line of its own).
pub(crate) fn lines_at_separators(text: &str, windows: bool, fit: impl Fn(&str) -> bool) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for part in text.split_inclusive(|c| separates(c, windows)) {
        if !line.is_empty() && !fit(&format!("{line}{part}")) {
            lines.push(std::mem::take(&mut line));
        }
        line.push_str(part);
    }
    lines.push(line);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As wide as `n` characters at the most.
    fn within(n: usize) -> impl Fn(&str) -> bool {
        move |s: &str| s.chars().count() <= n
    }

    /// The Windows CI's path, `\` and `/` mixed (`mg_testkit::test_output`
    /// joins `../../target` to the crate's folder).
    const MIXED: &str =
        r"D:\a\m\crates\mg-testkit\../../target/test-output\ui-u21-long/a_folder\zz_ours.hak";

    #[test]
    fn a_windows_path_is_cut_at_its_own_separators() {
        let path = r"C:\Users\builder\Documents\Neverwinter Nights\hak\id_resources.hak";
        let cut = cut_folders(path, true, within(24)).unwrap();
        assert_eq!(cut, r"…\hak\id_resources.hak");
        assert!(path.ends_with(&cut["…".len()..]));
        assert_eq!(cut_folders(path, true, within(19)).unwrap(), r"…\id_resources.hak");
        assert_eq!(cut_folders(path, true, within(10)), None, "not even its name fits");
    }

    #[test]
    fn a_mixed_path_keeps_each_separator_as_it_is() {
        let cut = cut_folders(MIXED, true, within(22)).unwrap();
        assert_eq!(cut, r"…/a_folder\zz_ours.hak");
        assert!(MIXED.ends_with(&cut["…".len()..]), "the end of the path itself");
        assert_eq!(cut_folders(MIXED, true, within(13)).unwrap(), r"…\zz_ours.hak");
        // (Every cut is the path's own end.)
        for n in 13..MIXED.len() {
            let cut = cut_folders(MIXED, true, within(n)).unwrap();
            assert!(MIXED.ends_with(&cut["…".len()..]), "{cut}");
        }
    }

    #[test]
    fn a_unix_path_is_cut_at_slashes_only() {
        let path = "/home/builder/hak/a\\b.hak";
        assert_eq!(cut_folders(path, false, within(13)).unwrap(), "…/hak/a\\b.hak");
        assert_eq!(cut_folders(path, false, within(9)).unwrap(), "…/a\\b.hak", "a name's `\\`");
        // (The root's `/` is no folder dropped.)
        assert_eq!(cut_folders("/one", false, within(9)), None);
        assert_eq!(cut_folders(path, false, within(100)).unwrap(), "…/builder/hak/a\\b.hak");
    }

    #[test]
    fn the_last_parts_are_the_path_s_own() {
        let tlk = r"C:\Users\b\Documents\Neverwinter Nights\lang\en\data\dialog.tlk";
        assert_eq!(last_parts(tlk, 5, true), r"…\Neverwinter Nights\lang\en\data\dialog.tlk");
        assert_eq!(
            last_parts(r"C:\lang\en\data\dialog.tlk", 5, true),
            r"C:\lang\en\data\dialog.tlk"
        );
        assert_eq!(last_parts(MIXED, 2, true), r"…/a_folder\zz_ours.hak");
        let unix = "/home/b/.local/share/Neverwinter Nights/lang/en/data/dialog.tlk";
        assert_eq!(last_parts(unix, 5, false), "…/Neverwinter Nights/lang/en/data/dialog.tlk");
        assert_eq!(last_parts("/a/b/c/d/e", 5, false), "/a/b/c/d/e");
        assert_eq!(last_parts(r"C:\a\b", 5, false), r"C:\a\b", "one part elsewhere");
    }

    #[test]
    fn a_windows_path_breaks_into_lines_at_its_separators() {
        let path = r"C:\Users\b\Documents\Neverwinter Nights\lang\en\data\dialog.tlk";
        let lines = lines_at_separators(path, true, within(24));
        assert_eq!(
            lines,
            "C:\\Users\\b\\Documents\\\nNeverwinter Nights\\lang\\\nen\\data\\dialog.tlk"
        );
        assert_eq!(lines.replace('\n', ""), path, "nothing lost");
        assert_eq!(lines_at_separators(path, false, within(24)), path, "unbroken elsewhere");
        let unix = lines_at_separators("/a/bb/ccc", false, within(4));
        assert_eq!(unix, "/a/\nbb/\nccc");
    }
}
