//! The files a module names (its haks, `Mod_HakList`'s `Mod_Hak` or the
//! older single `Mod_Hak`, and its custom talk table, `Mod_CustomTlk`:
//! CExoString bytes) and the names it names them by.
//!
//! The game takes a name's bytes as the file name's bytes, the extension
//! after them, and converts nothing: on Linux, a name with the byte 0xF1
//! (`ñ` in Windows-1252) opens the file whose name has the byte 0xF1, not
//! the one named `ñ` in UTF-8 (0xC3 0xB1), and the other way about, a hak
//! and a talk table alike (`engine_hak_names`). Nor does it fold case: a
//! module naming `MG_PLAIN` doesn't open `mg_plain.hak` on Linux, whose
//! file names are of a case ([`differs_in_case`]). Every reader and writer
//! of these names goes through these functions, so that the name shown is
//! the file read: a name is held as text by Windows-1252, which gives every
//! byte a character of its own and back, so no name loses a byte on its way
//! to the file.
//!
//! On Windows the game opens the file by the text of the name's bytes in
//! the system's codepage, Windows-1252 on a Western system (taken, not
//! seen: the engine test runs on Linux), and the file system ignores case.

use std::ffi::{OsStr, OsString};

use mg_core::Codepage;

/// The codepage a name's bytes are held as text by: one character for
/// each byte, and back.
const NAMES: Codepage = Codepage::WINDOWS_1252;

/// A name's bytes (a hak's or the talk table's, as a module's IFO holds
/// them) as text.
pub fn file_name_text(bytes: &[u8]) -> String {
    NAMES.decode(bytes).into_owned()
}

/// The bytes a module names the file `name` by ([`file_name_text`] back);
/// a character that has no byte (a name typed, not read) is `?`.
pub fn file_name_bytes(name: &str) -> Vec<u8> {
    NAMES.encode_lossy(name)
}

/// The name of the file the game opens for `name` with the extension
/// `ext` (`hak`, `tlk`): `<name>.<ext>`, on Linux and macOS the name's own
/// bytes.
pub fn named_file(name: &str, ext: &str) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let mut bytes = file_name_bytes(name);
        bytes.push(b'.');
        bytes.extend_from_slice(ext.as_bytes());
        OsStr::from_bytes(&bytes).to_os_string()
    }
    #[cfg(not(unix))]
    {
        OsString::from(format!("{name}.{ext}"))
    }
}

/// The name a module names the file of this stem (its name without the
/// extension) by: [`named_file`] back.
pub fn name_of_file(stem: &OsStr) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        file_name_text(stem.as_bytes())
    }
    #[cfg(not(unix))]
    {
        stem.to_string_lossy().into_owned()
    }
}

/// Whether the game finds a file named `listed` (a name as
/// [`name_of_file`] reads it) for `name` only where the file system
/// ignores case: the two differ in the case of their ASCII letters alone.
/// The game opens files by their exact names (on Linux, a name in another
/// case is another file); Windows' and macOS's file systems take one for
/// the other.
pub fn differs_in_case(name: &str, listed: &str) -> bool {
    name != listed && name.eq_ignore_ascii_case(listed)
}

/// Whether this system's file system takes a file name in another case for
/// the same file, as Windows' and macOS's do (Linux's doesn't).
pub const CASE_IGNORED: bool = cfg!(any(windows, target_os = "macos"));

/// Whether the game finds the file a module names `name` among those
/// `listed` (names as [`name_of_file`] reads them, of one extension).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listed {
    Found,
    /// Not found: the file listed of its name in another case, which the
    /// game doesn't open (on Linux; elsewhere it is [`Listed::Found`]).
    OtherCase(String),
    Missing,
}

/// Whether the game finds the file a module names `name` among those
/// `listed`, as the file system takes names ([`CASE_IGNORED`]).
pub fn listed(listed: &[String], name: &str) -> Listed {
    listed_as(listed, name, CASE_IGNORED)
}

fn listed_as(listed: &[String], name: &str, case_ignored: bool) -> Listed {
    if listed.iter().any(|l| l == name) {
        return Listed::Found;
    }
    match listed.iter().find(|l| differs_in_case(name, l)) {
        Some(_) if case_ignored => Listed::Found,
        Some(other) => Listed::OtherCase(other.clone()),
        None => Listed::Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every byte a name can hold comes back from its text, and its file
    /// has those bytes (on Linux and macOS).
    #[test]
    fn a_name_s_bytes_come_back_from_its_text() {
        let all: Vec<u8> = (1..=255).collect();
        assert_eq!(file_name_bytes(&file_name_text(&all)), all);
        // `ñ` in Windows-1252, and in UTF-8: two names, two files.
        let (cp, utf8) = (file_name_text(b"mg_\xf1hak"), file_name_text("mg_ñhak".as_bytes()));
        assert_eq!(cp, "mg_ñhak");
        assert_eq!(utf8, "mg_Ã±hak");
        assert_eq!(file_name_bytes(&cp), b"mg_\xf1hak");
        assert_eq!(file_name_bytes(&utf8), "mg_ñhak".as_bytes());
        assert_eq!(file_name_text(b"plain"), "plain");
        // A character no byte is written as `?`.
        assert_eq!(file_name_bytes("mg_ğ"), b"mg_?");
    }

    #[cfg(unix)]
    #[test]
    fn a_file_is_its_name_s_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let cp = file_name_text(b"mg_\xf1hak");
        assert_eq!(named_file(&cp, "hak").as_bytes(), b"mg_\xf1hak.hak");
        assert_eq!(named_file(&cp, "tlk").as_bytes(), b"mg_\xf1hak.tlk");
        let utf8 = file_name_text("mg_ñhak".as_bytes());
        assert_eq!(named_file(&utf8, "hak").as_bytes(), "mg_ñhak.hak".as_bytes());
        assert_eq!(named_file("plain", "hak"), OsStr::new("plain.hak"));
        // And back: a file's name is the module's.
        assert_eq!(name_of_file(OsStr::from_bytes(b"mg_\xf1hak")), cp);
        assert_eq!(name_of_file(OsStr::new("mg_ñhak")), utf8);
    }

    #[cfg(not(unix))]
    #[test]
    fn a_file_is_its_name_s_text() {
        assert_eq!(named_file("mg_ñhak", "hak"), OsStr::new("mg_ñhak.hak"));
        assert_eq!(name_of_file(OsStr::new("mg_ñhak")), "mg_ñhak");
        assert_eq!(file_name_bytes(&name_of_file(OsStr::new("mg_ñhak"))), b"mg_\xf1hak");
    }

    /// Only ASCII letters' case: `ñ` and `Ñ` are other bytes, not another
    /// case of one name.
    #[test]
    fn a_name_in_another_case() {
        assert!(differs_in_case("MG_PLAIN", "mg_plain"));
        assert!(!differs_in_case("mg_plain", "mg_plain"));
        assert!(!differs_in_case("mg_plain", "mg_other"));
        assert!(!differs_in_case("mg_Ñ", "mg_ñ"));
    }

    /// A name in another case is not found on Linux, the file named; on
    /// Windows and macOS it is found.
    #[test]
    fn a_name_listed_in_another_case() {
        let all = ["mg_plain".to_string(), "Other".to_string()];
        assert_eq!(listed_as(&all, "mg_plain", false), Listed::Found);
        assert_eq!(listed_as(&all, "MG_PLAIN", false), Listed::OtherCase("mg_plain".into()));
        assert_eq!(listed_as(&all, "MG_PLAIN", true), Listed::Found);
        assert_eq!(listed_as(&all, "gone", false), Listed::Missing);
        assert_eq!(listed_as(&all, "gone", true), Listed::Missing);
        assert_eq!(listed(&all, "MG_PLAIN") == Listed::Found, CASE_IGNORED);
    }
}
