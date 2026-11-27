//! Links to the player's folders and files, for a user folder of
//! Moonglow's that the game or its server is given (Test Module's
//! `test-user`, `mg serve`'s own): what they lead to is only read.

use std::path::Path;

/// Links `to` to the folder `from`: a symbolic link, else (Windows,
/// without the right to make one) a junction.
pub fn link_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(from, to)
    }
    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(from, to).is_ok() {
            return Ok(());
        }
        let made = std::process::Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(to)
            .arg(from)
            .output()?;
        if made.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(String::from_utf8_lossy(&made.stderr).trim().to_string()))
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (from, to);
        Err(std::io::Error::other("no links on this system"))
    }
}

/// Links `to` to the file `from`: a symbolic link (Unix); a hard link,
/// else a symbolic link, else a copy (Windows).
pub fn link_file(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(from, to)
    }
    #[cfg(not(unix))]
    {
        std::fs::hard_link(from, to)
            .or_else(|_| {
                #[cfg(windows)]
                return std::os::windows::fs::symlink_file(from, to);
                #[cfg(not(windows))]
                Err(std::io::Error::other("no links"))
            })
            .or_else(|_| std::fs::copy(from, to).map(|_| ()))
    }
}

/// Whether `path` is a link (a symbolic link, or on Windows a junction),
/// whether or not what it leads to is there.
pub fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// Takes away the link `path` (to a file or a folder, a junction
/// included), never what it leads to.
pub fn unlink(path: &Path) -> std::io::Result<()> {
    // (A link to a folder is a folder's entry on Windows, a file's on Unix.)
    std::fs::remove_file(path).or_else(|e| std::fs::remove_dir(path).map_err(|_| e))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder linked is read through its link, and the link taken away
    /// leaves the folder and its files as they were.
    #[test]
    fn a_folder_linked_and_unlinked() {
        let dir = mg_testkit::scratch_dir("mg-module-links");
        let from = dir.join("from");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(from.join("a.txt"), b"a").unwrap();
        let to = dir.join("to");
        assert!(!is_link(&to));
        link_dir(&from, &to).unwrap();
        assert!(is_link(&to));
        assert_eq!(std::fs::read(to.join("a.txt")).unwrap(), b"a");
        unlink(&to).unwrap();
        assert!(std::fs::symlink_metadata(&to).is_err());
        assert_eq!(std::fs::read(from.join("a.txt")).unwrap(), b"a");
        // (A folder of its own is no link.)
        assert!(!is_link(&from));
    }
}
