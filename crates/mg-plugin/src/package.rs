//! A plugin as a file to hand around: a zip archive of its folder, read
//! and installed into a plugins folder.
//!
//! The archive is taken as untrusted. Nothing is written until all of it
//! is read and its manifest is good; a name that leads out of the plugin's
//! folder, a link, an encrypted entry, too many files or too much data is
//! a fault that refuses the whole archive. Reading it runs none of the
//! plugin's code, and installing it enables nothing.

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{Manifest, Plugin, PluginError};

/// The most files a plugin's archive may hold.
pub const MAX_FILES: usize = 500;

/// The most a plugin's archive may unpack to, in bytes.
pub const MAX_BYTES: u64 = 16 << 20;

/// The most entries of any kind looked at (folders, what is skipped).
const MAX_ENTRIES: usize = 4000;

/// The biggest archive read, in bytes.
pub const MAX_ARCHIVE: u64 = 64 << 20;

/// In a folder an archive was installed to, the file that says so: only a
/// folder with it is replaced by installing again.
pub const MARKER: &str = ".moonglow-installed";

/// A plugin read from an archive, not yet on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub manifest: Manifest,
    /// Its files: each one's path in the plugin's folder (parts joined
    /// with `/`) and bytes.
    files: Vec<(String, Vec<u8>)>,
}

/// What a plugins folder has of a package's plugin already.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Existing {
    Nothing,
    /// Installed from an archive before: this version (as its manifest
    /// says; empty if that does not read).
    Installed {
        version: String,
    },
    /// There, but not installed from an archive (an author's own folder,
    /// say): installing leaves it alone.
    Foreign {
        dir: PathBuf,
    },
}

fn fault<T>(message: impl Into<String>) -> Result<T, PluginError> {
    Err(PluginError::Package(message.into()))
}

/// An entry's name as the parts of a path inside the archive. `None` for
/// what is skipped: a folder's own entry, hidden files and folders
/// (`.git`, `.DS_Store`), and macOS's `__MACOSX`. A name that could lead
/// out of the folder it is unpacked to, or that some system cannot have
/// as a file's name, is an error.
fn parts(name: &str) -> Result<Option<Vec<String>>, String> {
    // (A backslash is a separator to Windows, whatever wrote the archive.)
    let name = name.replace('\\', "/");
    let bad = |why: &str| Err(format!("the entry {name:?} {why}"));
    if name.starts_with('/') {
        return bad("is an absolute path");
    }
    let Some(path) = name.strip_suffix('/').or(Some(name.as_str())).filter(|p| !p.is_empty())
    else {
        return bad("has no name");
    };
    if path.len() > 200 {
        return bad("has too long a name");
    }
    let mut out = Vec::new();
    for part in path.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return bad("leads out of the plugin's folder");
        }
        if part
            .chars()
            .any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
        {
            return bad("has a character a file's name cannot have");
        }
        if part.ends_with('.') || part.ends_with(' ') {
            return bad("ends with a dot or a space");
        }
        let stem = part.split('.').next().unwrap_or_default().to_ascii_uppercase();
        let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.ends_with(|c: char| c.is_ascii_digit()));
        if device {
            return bad("has a name Windows keeps for a device");
        }
        out.push(part.to_string());
    }
    if out.len() > 8 {
        return bad("is in too deep a folder");
    }
    let hidden = out.iter().any(|p| p.starts_with('.') || p == "__MACOSX");
    Ok((!hidden && !name.ends_with('/')).then_some(out))
}

impl Package {
    /// Reads a plugin's archive: a zip with `plugin.cfg` at its top, or
    /// in the one folder at its top that has one (as zipping a folder, or
    /// a repository's download, makes it). What is outside the plugin's
    /// folder is ignored.
    pub fn read(bytes: &[u8]) -> Result<Package, PluginError> {
        if bytes.len() as u64 > MAX_ARCHIVE {
            return fault(format!("it is over {} MB", MAX_ARCHIVE >> 20));
        }
        let Ok(mut zip) = ZipArchive::new(Cursor::new(bytes)) else {
            return fault("it is not a zip archive");
        };
        if zip.len() > MAX_ENTRIES {
            return fault(format!("it has over {MAX_ENTRIES} entries"));
        }
        // What is in it, before any of it is unpacked.
        let mut entries: Vec<(usize, Vec<String>)> = Vec::new();
        for i in 0..zip.len() {
            let Ok(entry) = zip.by_index_raw(i) else {
                return fault(format!("its entry {i} does not read"));
            };
            let name = entry.name().to_string();
            if entry.is_symlink() {
                return fault(format!("the entry {name:?} is a link"));
            }
            if entry.encrypted() {
                return fault(format!("the entry {name:?} is encrypted"));
            }
            if entry.is_dir() {
                parts(&name).map_err(PluginError::Package)?;
                continue;
            }
            if let Some(path) = parts(&name).map_err(PluginError::Package)? {
                entries.push((i, path));
            }
        }

        // The plugin's folder in it: the top, or the one folder with a
        // manifest.
        let is_manifest = |p: &[String]| p.last().is_some_and(|n| n == "plugin.cfg");
        let root: Vec<String> = if entries.iter().any(|(_, p)| p.len() == 1 && is_manifest(p)) {
            Vec::new()
        } else {
            let folders: BTreeSet<&str> = entries
                .iter()
                .filter(|(_, p)| p.len() == 2 && is_manifest(p))
                .map(|(_, p)| p[0].as_str())
                .collect();
            match folders.len() {
                0 => return fault("it has no plugin.cfg at its top, or in a folder at its top"),
                1 => vec![folders.into_iter().next().unwrap_or_default().to_string()],
                _ => {
                    return fault(format!(
                        "it holds several plugins ({}): install each from an archive of its own",
                        folders.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
            }
        };
        let inside: Vec<(usize, String)> = entries
            .into_iter()
            .filter(|(_, p)| p.len() > root.len() && p[..root.len()] == root[..])
            .map(|(i, p)| (i, p[root.len()..].join("/")))
            .collect();
        if inside.len() > MAX_FILES {
            return fault(format!("it has over {MAX_FILES} files"));
        }
        // Names that are one to a system that ignores case, or a file
        // where another's folder is, cannot both be unpacked.
        let mut seen = BTreeSet::new();
        for (_, path) in &inside {
            if !seen.insert(path.to_lowercase()) {
                return fault(format!("it has {path:?} twice"));
            }
        }
        for (_, path) in &inside {
            let mut folder = path.to_lowercase();
            while let Some(at) = folder.rfind('/') {
                folder.truncate(at);
                if seen.contains(&folder) {
                    return fault(format!("{folder:?} is a file and a folder in it"));
                }
            }
        }

        // Its data, to the limit: by what is read, not by what it says.
        let mut files = Vec::new();
        let mut left = MAX_BYTES;
        for (i, path) in inside {
            let entry = match zip.by_index(i) {
                Ok(entry) => entry,
                Err(e) => return fault(format!("{path:?} does not unpack: {e}")),
            };
            let mut data = Vec::new();
            if let Err(e) = entry.take(left + 1).read_to_end(&mut data) {
                return fault(format!("{path:?} does not unpack: {e}"));
            }
            if data.len() as u64 > left {
                return fault(format!("it unpacks to over {} MB", MAX_BYTES >> 20));
            }
            left -= data.len() as u64;
            files.push((path, data));
        }
        files.sort();

        let find = |name: &str| files.iter().find(|(p, _)| p == name).map(|(_, d)| d);
        let Some(Ok(text)) = find("plugin.cfg").map(|d| std::str::from_utf8(d)) else {
            return fault("its plugin.cfg is not text (UTF-8)");
        };
        let manifest = Manifest::parse(text)
            .map_err(|e| PluginError::Manifest("plugin.cfg in the archive".into(), e))?;
        if find(&manifest.entry).is_none() {
            return fault(format!("its entry {} is not in it", manifest.entry));
        }
        // (Its id names the folder it is installed to.)
        if !matches!(parts(&manifest.id), Ok(Some(_))) {
            return fault(format!("its id {:?} cannot name a folder", manifest.id));
        }
        Ok(Package { manifest, files })
    }

    /// Its files' paths in the plugin's folder, sorted.
    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(|(p, _)| p.as_str())
    }

    /// Where it goes in the plugins folder `plugins`: a folder named by
    /// its id.
    pub fn folder(&self, plugins: &Path) -> PathBuf {
        plugins.join(&self.manifest.id)
    }

    /// What `plugins` has of this plugin already.
    pub fn existing(&self, plugins: &Path) -> Existing {
        let target = self.folder(plugins);
        // Its id in another folder: two of one plugin would be listed.
        for dir in crate::folders(plugins) {
            if dir != target && Plugin::load(&dir).is_ok_and(|p| p.manifest.id == self.manifest.id)
            {
                return Existing::Foreign { dir };
            }
        }
        if !target.exists() {
            return Existing::Nothing;
        }
        if !target.join(MARKER).is_file() {
            return Existing::Foreign { dir: target };
        }
        let version = Plugin::load(&target).map(|p| p.manifest.version).unwrap_or_default();
        Existing::Installed { version }
    }

    /// Installs it into `plugins` (made if it is not there), as the
    /// folder [`Package::folder`]. One installed from an archive before
    /// is replaced if `replace` says so; a folder that was not is never
    /// touched. All of it is written beside the place first and moved in
    /// at once, so that a failure leaves what was there.
    pub fn install(&self, plugins: &Path, replace: bool) -> Result<Plugin, PluginError> {
        let name = &self.manifest.name;
        match self.existing(plugins) {
            Existing::Nothing => {}
            Existing::Installed { .. } if replace => {}
            Existing::Installed { version } => {
                return fault(format!("{name} {version} is installed already"));
            }
            Existing::Foreign { dir } => {
                return fault(format!(
                    "{name} is in the plugins folder already, as {}, and was not installed from \
                     a file: remove that folder first",
                    dir.display()
                ));
            }
        }
        let io = |what: &str, e: std::io::Error| PluginError::Package(format!("{what}: {e}"));
        let target = self.folder(plugins);
        // (Hidden folders, which no listing of plugins shows.)
        let staged = plugins.join(format!(".installing-{}", self.manifest.id));
        let old = plugins.join(format!(".replaced-{}", self.manifest.id));
        for stale in [&staged, &old] {
            if stale.exists() {
                std::fs::remove_dir_all(stale).map_err(|e| io("clearing an earlier attempt", e))?;
            }
        }
        let write = || -> std::io::Result<()> {
            std::fs::create_dir_all(&staged)?;
            for (path, data) in &self.files {
                let file = staged.join(path);
                if let Some(parent) = file.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(file, data)?;
            }
            std::fs::write(
                staged.join(MARKER),
                "Moonglow installed this plugin from an archive; installing it again replaces \
                 this folder.\n",
            )
        };
        if let Err(e) = write() {
            let _ = std::fs::remove_dir_all(&staged);
            return Err(io("writing the plugin", e));
        }
        let replacing = target.exists();
        if replacing && let Err(e) = std::fs::rename(&target, &old) {
            let _ = std::fs::remove_dir_all(&staged);
            return Err(io("moving the installed plugin aside", e));
        }
        if let Err(e) = std::fs::rename(&staged, &target) {
            // (Back to what was there.)
            if replacing {
                let _ = std::fs::rename(&old, &target);
            }
            let _ = std::fs::remove_dir_all(&staged);
            return Err(io("moving the plugin into place", e));
        }
        if replacing {
            let _ = std::fs::remove_dir_all(&old);
        }
        Plugin::load(&target)
    }
}

/// A plugin's folder as an archive to hand around: its files (hidden ones
/// and links left out) in a folder named by its id. What it makes,
/// [`Package::read`] reads: a file the reader would refuse is an error
/// here, where the author sees it.
pub fn pack(plugin: &Plugin) -> Result<Vec<u8>, PluginError> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let (path, kind) = (entry.path(), entry.file_type()?);
            if entry.file_name().to_string_lossy().starts_with('.') || kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                walk(&path, base, out)?;
            } else if let Ok(inside) = path.strip_prefix(base) {
                let name: Vec<_> =
                    inside.components().map(|c| c.as_os_str().to_string_lossy()).collect();
                out.push((name.join("/"), path));
            }
        }
        Ok(())
    }
    let io =
        |what: &Path, e: std::io::Error| PluginError::Package(format!("{}: {e}", what.display()));
    let mut files = Vec::new();
    walk(&plugin.dir, &plugin.dir, &mut files).map_err(|e| io(&plugin.dir, e))?;
    files.sort();
    if files.len() > MAX_FILES {
        return fault(format!("the plugin has over {MAX_FILES} files"));
    }
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let written = |e: zip::result::ZipError| PluginError::Package(format!("writing it: {e}"));
    let mut total = 0;
    for (name, path) in &files {
        parts(name).map_err(PluginError::Package)?;
        let data = std::fs::read(path).map_err(|e| io(path, e))?;
        total += data.len() as u64;
        if total > MAX_BYTES {
            return fault(format!("the plugin is over {} MB", MAX_BYTES >> 20));
        }
        zip.start_file(format!("{}/{name}", plugin.manifest.id), options).map_err(written)?;
        zip.write_all(&data).map_err(|e| PluginError::Package(format!("writing it: {e}")))?;
    }
    let bytes = zip.finish().map_err(written)?.into_inner();
    // (It reads back, or it is no archive to give anyone.)
    Package::read(&bytes)?;
    Ok(bytes)
}
