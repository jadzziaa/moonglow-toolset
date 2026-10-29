//! The archives content is downloaded as (zip, 7z and RAR), read where
//! they are: what each holds, and its files' bytes streamed to writers in
//! one pass over the archive (a solid archive is decoded once, not once a
//! file).

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::Path;

/// The kind of archive a file is, by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Zip,
    SevenZ,
    Rar,
}

impl Kind {
    pub(crate) fn of(path: &Path) -> Option<Kind> {
        let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
        match ext.as_str() {
            "zip" => Some(Kind::Zip),
            "7z" => Some(Kind::SevenZ),
            "rar" => Some(Kind::Rar),
            _ => None,
        }
    }
}

/// A file in an archive: its name there and its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Listed {
    pub(crate) name: String,
    pub(crate) size: u64,
}

/// What opens the writer an archive's file `name` goes to.
pub(crate) type Open<'a> = dyn FnMut(&str) -> Result<Box<dyn Write>, String> + 'a;

/// The error for an archive with a password.
fn locked(path: &Path) -> String {
    format!(
        "{}: is protected by a password, which Moonglow doesn't ask for: unpack it first",
        path.display()
    )
}

/// The files of the archive at `path` (not its folders). `wanted` says
/// which matter: one of those that can't be read without a password, or
/// is split across volumes, is an error (the rest are left alone).
pub(crate) fn list(
    kind: Kind,
    path: &Path,
    wanted: &dyn Fn(&str) -> bool,
) -> Result<Vec<Listed>, String> {
    match kind {
        Kind::Zip => list_zip(path, wanted),
        Kind::SevenZ => list_7z(path),
        Kind::Rar => list_rar(path, wanted),
    }
}

/// Streams the files named `wanted` of the archive at `path`, each to the
/// writer `open` gives for it, in one pass; a name not in it is an error.
pub(crate) fn extract(
    kind: Kind,
    path: &Path,
    wanted: &BTreeSet<String>,
    open: &mut Open<'_>,
) -> Result<(), String> {
    if wanted.is_empty() {
        return Ok(());
    }
    let mut seen = BTreeSet::new();
    match kind {
        Kind::Zip => extract_zip(path, wanted, open, &mut seen)?,
        Kind::SevenZ => extract_7z(path, wanted, open, &mut seen)?,
        Kind::Rar => extract_rar(path, wanted, open, &mut seen)?,
    }
    match wanted.difference(&seen).next() {
        Some(missing) => Err(format!("{}: has no {missing}", path.display())),
        None => Ok(()),
    }
}

fn open_zip(path: &Path) -> Result<zip::ZipArchive<std::io::BufReader<std::fs::File>>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| format!("{}: not a zip archive that can be read ({e})", path.display()))
}

fn list_zip(path: &Path, wanted: &dyn Fn(&str) -> bool) -> Result<Vec<Listed>, String> {
    let mut zip = open_zip(path)?;
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let f = zip.by_index_raw(i).map_err(|e| format!("{}: {e}", path.display()))?;
        if f.is_dir() {
            continue;
        }
        if f.encrypted() && wanted(f.name()) {
            return Err(locked(path));
        }
        out.push(Listed { name: f.name().to_string(), size: f.size() });
    }
    Ok(out)
}

fn extract_zip(
    path: &Path,
    wanted: &BTreeSet<String>,
    open: &mut Open<'_>,
    seen: &mut BTreeSet<String>,
) -> Result<(), String> {
    let mut zip = open_zip(path)?;
    for name in wanted {
        let at = |e: &dyn std::fmt::Display| format!("{} ({name}): {e}", path.display());
        let Ok(mut f) = zip.by_name(name) else { continue };
        let mut to = open(name)?;
        std::io::copy(&mut f, &mut to).map_err(|e| at(&e))?;
        to.flush().map_err(|e| at(&e))?;
        seen.insert(name.clone());
    }
    Ok(())
}

/// A 7z error said for the archive at `path`, a password's as such.
fn error_7z(path: &Path, e: sevenz_rust2::Error) -> String {
    use sevenz_rust2::Error as E;
    match &e {
        E::PasswordRequired | E::MaybeBadPassword(_) => locked(path),
        E::UnsupportedCompressionMethod(m) if m.starts_with("AES") => locked(path),
        _ => format!("{}: not a 7z archive that can be read ({e})", path.display()),
    }
}

fn list_7z(path: &Path) -> Result<Vec<Listed>, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let archive = sevenz_rust2::Archive::read(&mut file, &sevenz_rust2::Password::empty())
        .map_err(|e| error_7z(path, e))?;
    let aes = sevenz_rust2::EncoderMethod::ID_AES256_SHA256;
    if archive.blocks.iter().any(|b| b.coders.iter().any(|c| c.encoder_method_id() == aes)) {
        return Err(locked(path));
    }
    Ok(archive
        .files
        .iter()
        .filter(|f| !f.is_directory() && !f.is_anti_item())
        .map(|f| Listed { name: f.name().to_string(), size: f.size() })
        .collect())
}

fn extract_7z(
    path: &Path,
    wanted: &BTreeSet<String>,
    open: &mut Open<'_>,
    seen: &mut BTreeSet<String>,
) -> Result<(), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut reader = sevenz_rust2::ArchiveReader::new(
        std::io::BufReader::new(file),
        sevenz_rust2::Password::empty(),
    )
    .map_err(|e| error_7z(path, e))?;
    // (On this thread: LZMA2's threads look for each other's errors every
    // tenth of a second, and a hak is seldom in the chunks they share.)
    reader.set_thread_count(1);
    // (What `open` refuses, said as it said it, not as a 7z error.)
    let mut refused = None;
    let done = reader.for_each_entries(|entry, data| {
        let name = entry.name();
        if entry.is_directory() || !wanted.contains(name) {
            // (A solid block's files follow one another: those not wanted
            // are read through all the same.)
            std::io::copy(data, &mut std::io::sink())?;
            return Ok(true);
        }
        let mut to = match open(name) {
            Ok(to) => to,
            Err(e) => {
                refused = Some(e);
                return Ok(false);
            }
        };
        std::io::copy(data, &mut to)?;
        to.flush()?;
        seen.insert(name.to_string());
        Ok(seen.len() < wanted.len())
    });
    if let Some(e) = refused {
        return Err(e);
    }
    done.map_err(|e| error_7z(path, e))
}

/// A RAR error said for the archive at `path`: a password's, and a
/// volume's of a set, as such.
fn error_rar(path: &Path, e: rars::Error) -> String {
    let encryption = matches!(e.root_cause(), rars::Error::FeatureDisabled { feature } if feature.contains("encrypt"));
    match e.kind() {
        rars::ErrorKind::PasswordRequired | rars::ErrorKind::BadPassword => locked(path),
        _ if encryption => locked(path),
        _ => format!("{}: not a RAR archive that can be read ({e})", path.display()),
    }
}

/// The error for one volume of a set.
fn a_volume(path: &Path) -> String {
    format!(
        "{}: is one volume of a set, which Moonglow doesn't put together: unpack the set first",
        path.display()
    )
}

fn open_rar(path: &Path) -> Result<rars::Archive, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    rars::ArchiveReader::read_reader(std::io::BufReader::new(file)).map_err(|e| error_rar(path, e))
}

/// A RAR archive's file's name, `/` between its folders.
fn rar_name(member: &rars::ArchiveMember) -> Result<String, rars::Error> {
    Ok(String::from_utf8_lossy(&member.decoded_name(None)?).into_owned())
}

fn list_rar(path: &Path, wanted: &dyn Fn(&str) -> bool) -> Result<Vec<Listed>, String> {
    let archive = open_rar(path)?;
    let mut out = Vec::new();
    for m in archive.members() {
        if m.meta.is_directory || m.meta.is_redirection {
            continue;
        }
        let name = rar_name(&m).map_err(|e| error_rar(path, e))?;
        if wanted(&name) {
            if m.meta.is_encrypted {
                return Err(locked(path));
            }
            if m.meta.is_split_before || m.meta.is_split_after {
                return Err(a_volume(path));
            }
        }
        out.push(Listed { name, size: m.meta.unpacked_size });
    }
    Ok(out)
}

fn extract_rar(
    path: &Path,
    wanted: &BTreeSet<String>,
    open: &mut Open<'_>,
    seen: &mut BTreeSet<String>,
) -> Result<(), String> {
    use rars::ExtractionDecision::{Extract, Skip, Stop};
    let archive = open_rar(path)?;
    let mut refused = None;
    let done = archive.extract_with_control(rars::ArchiveReadOptions::new(), |m| {
        if m.meta.is_directory || m.meta.is_redirection {
            return Ok(Skip);
        }
        if seen.len() == wanted.len() {
            return Ok(Stop);
        }
        let name = rar_name(m)?;
        if !wanted.contains(&name) {
            // (A solid archive's files follow one another: those not
            // wanted are decoded all the same.)
            return Ok(Extract(Box::new(std::io::sink())));
        }
        match open(&name) {
            Ok(to) => {
                seen.insert(name);
                Ok(Extract(to))
            }
            Err(e) => {
                refused = Some(e);
                Ok(Stop)
            }
        }
    });
    if let Some(e) = refused {
        return Err(e);
    }
    done.map(|_| ()).map_err(|e| error_rar(path, e))
}

/// A writer that compares what it is given with a file's bytes, and
/// notes whether they differ (or the file has more or fewer).
pub(crate) struct Compare {
    file: std::io::BufReader<std::fs::File>,
    same: std::rc::Rc<std::cell::Cell<bool>>,
    buf: Vec<u8>,
}

impl Compare {
    /// A writer comparing with `file`, and the flag it clears when the
    /// bytes differ.
    pub(crate) fn new(
        file: &Path,
    ) -> std::io::Result<(Compare, std::rc::Rc<std::cell::Cell<bool>>)> {
        let same = std::rc::Rc::new(std::cell::Cell::new(true));
        let file = std::io::BufReader::new(std::fs::File::open(file)?);
        Ok((Compare { file, same: same.clone(), buf: Vec::new() }, same))
    }
}

impl Write for Compare {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.same.get() {
            self.buf.resize(bytes.len(), 0);
            if self.file.read_exact(&mut self.buf).is_err() || self.buf != bytes {
                self.same.set(false);
            }
        }
        Ok(bytes.len())
    }

    /// (At the end: the file has no more than was given.)
    fn flush(&mut self) -> std::io::Result<()> {
        if self.same.get() && self.file.read(&mut [0u8; 1]).is_ok_and(|n| n > 0) {
            self.same.set(false);
        }
        Ok(())
    }
}
