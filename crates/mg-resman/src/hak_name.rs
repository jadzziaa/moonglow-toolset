//! A module's hak names (`Mod_HakList`'s `Mod_Hak`, or the older single
//! `Mod_Hak`: CExoString bytes) and the files the game opens for them.
//!
//! The game takes a hak name's bytes as the file name's bytes, `.hak`
//! after them, and converts nothing: on Linux, a name with the byte 0xF1
//! (`ñ` in Windows-1252) opens the file whose name has the byte 0xF1, not
//! the one named `ñ` in UTF-8 (0xC3 0xB1), and the other way about
//! (`engine_hak_names`). Every reader and writer of hak names goes through
//! these functions, so that the name shown is the hak layered: a name is
//! held as text by Windows-1252, which gives every byte a character of its
//! own and back, so no name loses a byte on its way to the file.
//!
//! On Windows the game opens the file by the text of the name's bytes in
//! the system's codepage, Windows-1252 on a Western system (taken, not
//! seen: the engine test runs on Linux).

use std::ffi::{OsStr, OsString};

use mg_core::Codepage;

/// The codepage a hak name's bytes are held as text by: one character for
/// each byte, and back.
const NAMES: Codepage = Codepage::WINDOWS_1252;

/// A hak name's bytes (as a module's IFO holds them) as text.
pub fn hak_name(bytes: &[u8]) -> String {
    NAMES.decode(bytes).into_owned()
}

/// The bytes a module names the hak `name` by ([`hak_name`] back); a
/// character that has no byte (a name typed, not read) is `?`.
pub fn hak_name_bytes(name: &str) -> Vec<u8> {
    NAMES.encode_lossy(name)
}

/// The name of the file the game opens for the hak `name`: `<name>.hak`,
/// on Linux and macOS the name's own bytes.
pub fn hak_file_name(name: &str) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let mut bytes = hak_name_bytes(name);
        bytes.extend_from_slice(b".hak");
        OsStr::from_bytes(&bytes).to_os_string()
    }
    #[cfg(not(unix))]
    {
        OsString::from(format!("{name}.hak"))
    }
}

/// The hak name a module names the file of this stem (its name without
/// `.hak`) by: [`hak_file_name`] back.
pub fn hak_name_of_file(stem: &OsStr) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        hak_name(stem.as_bytes())
    }
    #[cfg(not(unix))]
    {
        stem.to_string_lossy().into_owned()
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
        assert_eq!(hak_name_bytes(&hak_name(&all)), all);
        // `ñ` in Windows-1252, and in UTF-8: two names, two files.
        let (cp, utf8) = (hak_name(b"mg_\xf1hak"), hak_name("mg_ñhak".as_bytes()));
        assert_eq!(cp, "mg_ñhak");
        assert_eq!(utf8, "mg_Ã±hak");
        assert_eq!(hak_name_bytes(&cp), b"mg_\xf1hak");
        assert_eq!(hak_name_bytes(&utf8), "mg_ñhak".as_bytes());
        assert_eq!(hak_name(b"plain"), "plain");
        // A character no byte is written as `?`.
        assert_eq!(hak_name_bytes("mg_ğ"), b"mg_?");
    }

    #[cfg(unix)]
    #[test]
    fn a_hak_s_file_is_its_name_s_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let cp = hak_name(b"mg_\xf1hak");
        assert_eq!(hak_file_name(&cp).as_bytes(), b"mg_\xf1hak.hak");
        let utf8 = hak_name("mg_ñhak".as_bytes());
        assert_eq!(hak_file_name(&utf8).as_bytes(), "mg_ñhak.hak".as_bytes());
        assert_eq!(hak_file_name("plain"), OsStr::new("plain.hak"));
        // And back: a file's name is the hak's.
        assert_eq!(hak_name_of_file(OsStr::from_bytes(b"mg_\xf1hak")), cp);
        assert_eq!(hak_name_of_file(OsStr::new("mg_ñhak")), utf8);
    }

    #[cfg(not(unix))]
    #[test]
    fn a_hak_s_file_is_its_name_s_text() {
        assert_eq!(hak_file_name("mg_ñhak"), OsStr::new("mg_ñhak.hak"));
        assert_eq!(hak_name_of_file(OsStr::new("mg_ñhak")), "mg_ñhak");
        assert_eq!(hak_name_bytes(&hak_name_of_file(OsStr::new("mg_ñhak"))), b"mg_\xf1hak");
    }
}
