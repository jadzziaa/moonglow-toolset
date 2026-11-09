//! An update downloaded, checked and made ready to install: the package
//! written to a `.part` file in the folder it is used in, its hash
//! checked against the release's signed list, then renamed (or unpacked)
//! beside what it replaces.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::http::{Http, Progress, get};
use crate::verify::{self, Hash, Hasher, PublicKey};
use crate::{Kind, Release, Version};

/// What every file and folder of an update is named from, so that what a
/// failed or interrupted one left is found and removed ([`clean`]).
pub const PREFIX: &str = ".moonglow-update-";

/// The file in the updates' cache folder that a run of Moonglow keeps
/// locked from the start of a download until it ends ([`hold`]): what it
/// writes and stages is another run's to leave alone ([`clean`]).
pub const LOCK: &str = ".moonglow-update-lock";

/// What a second run is told when it would download too.
pub const BUSY: &str =
    "another Moonglow that is running has this update: it is installed as that one closes";

/// The locks this run holds, by cache folder (one, but for the tests).
static HELD: std::sync::Mutex<Vec<(PathBuf, File)>> = std::sync::Mutex::new(Vec::new());

fn held() -> std::sync::MutexGuard<'static, Vec<(PathBuf, File)>> {
    HELD.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Takes the lock of `cache` for this run, until it ends or a download fails:
/// whether it was taken now (not held already), or [`BUSY`] where another
/// run holds it. Where the file can't be made or the system has no locks,
/// the update goes on without one, as it did before there was one.
fn hold(cache: &Path) -> Result<bool, String> {
    let mut held = held();
    if held.iter().any(|(c, _)| c == cache) {
        return Ok(false);
    }
    let _ = std::fs::create_dir_all(cache);
    let options = {
        let mut o = std::fs::OpenOptions::new();
        o.read(true).write(true).create(true).truncate(false);
        o
    };
    let Ok(file) = options.open(cache.join(LOCK)) else { return Ok(false) };
    match file.try_lock() {
        Ok(()) => {
            held.push((cache.to_path_buf(), file));
            Ok(true)
        }
        Err(std::fs::TryLockError::WouldBlock) => Err(BUSY.into()),
        Err(std::fs::TryLockError::Error(_)) => Ok(false),
    }
}

/// Lets go of the lock of `cache` (a download that failed left nothing).
fn release_lock(cache: &Path) {
    held().retain(|(c, _)| c != cache);
}

/// The largest package taken (the AppImage is about 60 MB).
const PACKAGE_LIMIT: u64 = 1 << 30;

/// An update ready to install as Moonglow closes, or installed already.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    pub version: Version,
    pub kind: Kind,
    pub ready: Ready,
}

/// What is ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ready {
    /// The AppImage checked, beside the image (renamed over it).
    Image { file: PathBuf, hash: Hash },
    /// The AppImage was replaced already.
    Installed,
    /// The Windows installer checked, in the cache folder (run).
    Installer { file: PathBuf, hash: Hash },
    /// The zip's files, unpacked into a folder beside the program's (each
    /// swapped in).
    Files { dir: PathBuf },
    /// The disk image's app, copied out beside the app (swapped in).
    App { app: PathBuf },
}

impl Staged {
    /// The update is in place already: a restart runs it.
    pub fn installed(&self) -> bool {
        self.ready == Ready::Installed
    }
}

/// Why an update can't be had, said in the window beside the page's link.
pub fn unwritable(folder: &Path) -> String {
    format!("Moonglow can't write to {}: update it as you installed it", folder.display())
}

/// A file being written: removed unless it is kept.
struct Part {
    path: PathBuf,
    kept: bool,
}

impl Drop for Part {
    fn drop(&mut self) {
        if !self.kept {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// `path` made anew for writing, or why it can't be (a folder that can't
/// be written is said so).
fn create(path: &Path) -> Result<(Part, File), String> {
    let folder = path.parent().unwrap_or(Path::new("."));
    // (One a failed or interrupted run left.)
    let _ = std::fs::remove_file(path);
    match std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => Ok((Part { path: path.to_path_buf(), kept: false }, file)),
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem
            ) =>
        {
            Err(unwritable(folder))
        }
        Err(e) => Err(format!("can't write {}: {e}", path.display())),
    }
}

/// Writes to a file and hashes what it writes.
struct Hashing<'a> {
    file: &'a mut File,
    hasher: Hasher,
}

impl Write for Hashing<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let n = self.file.write(bytes)?;
        self.hasher.update(&bytes[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

/// Downloads this copy's package of `release`, checks it against the
/// release's `SHA256SUMS` signed by `key`, and makes it ready to install
/// (`cache`: Moonglow's cache folder for updates). This run holds the
/// cache's lock from here on, so that a second Moonglow started meanwhile
/// leaves the download and what is staged alone; where another run holds
/// it, nothing is downloaded ([`BUSY`]).
pub fn fetch(
    http: &dyn Http,
    key: &PublicKey,
    release: &Release,
    kind: &Kind,
    cache: &Path,
    progress: Progress,
) -> Result<Staged, String> {
    let taken = hold(cache)?;
    let staged = fetch_held(http, key, release, kind, cache, progress);
    if staged.is_err() && taken {
        release_lock(cache);
    }
    staged
}

fn fetch_held(
    http: &dyn Http,
    key: &PublicKey,
    release: &Release,
    kind: &Kind,
    cache: &Path,
    progress: Progress,
) -> Result<Staged, String> {
    let version = &release.version;
    let package = kind
        .package(version)
        .ok_or_else(|| format!("Moonglow doesn't update {} itself", kind.what()))?;
    let asset = release.asset(&package).ok_or_else(|| format!("the release has no {package}"))?;
    let (Some(sums), Some(signature)) =
        (release.asset(verify::SUMS), release.asset(verify::SIGNATURE))
    else {
        return Err("the release isn't signed".into());
    };
    let v = version.file_part();
    // Where the download is written: beside what it replaces, or in the
    // cache folder for what is run or unpacked from there.
    let part = match kind {
        Kind::AppImage { .. } | Kind::WindowsZip { .. } => {
            kind.folder().expect("a folder").join(format!("{PREFIX}{package}.part"))
        }
        _ => {
            std::fs::create_dir_all(cache)
                .map_err(|e| format!("can't make {}: {e}", cache.display()))?;
            cache.join(format!("{package}.part"))
        }
    };
    // (The folder is found unwritable before anything is downloaded; the
    // macOS app's too, where its copy goes.)
    if let Kind::MacApp { app } = kind {
        let folder = app.parent().unwrap_or(Path::new("/"));
        drop(create(&folder.join(format!("{PREFIX}{v}.probe")))?);
    }
    let (mut kept, mut file) = create(&part)?;
    let sums = get(http, &sums.url, 1 << 20)?;
    let signature = get(http, &signature.url, 1 << 14)?;
    let signature =
        String::from_utf8(signature).map_err(|_| "the release's signature can't be read")?;
    let expected = verify::checked_hash(key, &sums, &signature, &release.tag, &package)?;
    let limit = asset.size.unwrap_or(PACKAGE_LIMIT).min(PACKAGE_LIMIT);
    let mut writing = Hashing { file: &mut file, hasher: Hasher::default() };
    http.download(&asset.url, &mut writing, limit, progress)?;
    let hash = writing.hasher.finish();
    file.sync_all().map_err(|e| format!("can't write {}: {e}", part.display()))?;
    drop(file);
    if hash != expected {
        return Err(format!(
            "the downloaded {package} isn't the one the release signed (its checksum differs)"
        ));
    }
    let ready = match kind {
        Kind::AppImage { image, .. } => {
            let staged = image.with_file_name(format!("{PREFIX}{v}.AppImage"));
            executable_like(&part, image)?;
            rename(&part, &staged)?;
            kept.kept = true;
            Ready::Image { file: staged, hash }
        }
        Kind::WindowsInstaller { .. } => {
            let staged = cache.join(&package);
            rename(&part, &staged)?;
            kept.kept = true;
            Ready::Installer { file: staged, hash }
        }
        Kind::WindowsZip { dir } => {
            let staged = dir.join(format!("{PREFIX}{v}"));
            unzip(&part, &staged)?;
            Ready::Files { dir: staged }
        }
        Kind::MacApp { app } => {
            let dmg = cache.join(&package);
            rename(&part, &dmg)?;
            let staged = app.with_file_name(format!("{PREFIX}{v}.app"));
            let mount = cache.join(format!("{PREFIX}{v}.mount"));
            let steps = mac_copy_out(&dmg, &mount, &staged, app);
            let done = crate::install::run_all(&steps);
            let _ = std::fs::remove_file(&dmg);
            done?;
            Ready::App { app: staged }
        }
        Kind::Managed(_) | Kind::Other => unreachable!("no package"),
    };
    Ok(Staged { version: version.clone(), kind: kind.clone(), ready })
}

fn rename(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::rename(from, to)
        .map_err(|e| format!("can't rename {} to {}: {e}", from.display(), to.display()))
}

/// `file` given `like`'s permissions (and run by its owner at least).
fn executable_like(file: &Path, like: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(like).map(|m| m.permissions().mode()).unwrap_or(0o755);
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(mode | 0o700))
            .map_err(|e| format!("can't make {} runnable: {e}", file.display()))?;
    }
    #[cfg(not(unix))]
    let _ = (file, like);
    Ok(())
}

/// The zip at `zip` unpacked into `into` (made anew), its files only, none
/// outside it.
pub fn unzip(zip: &Path, into: &Path) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(into);
    let bad = |e: &dyn std::fmt::Display| format!("can't unpack the zip: {e}");
    let file = File::open(zip).map_err(|e| bad(&e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| bad(&e))?;
    let unpacked = (|| {
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| bad(&e))?;
            let Some(name) = entry.enclosed_name() else {
                return Err(bad(&format!("{} lies outside it", entry.name())));
            };
            let path = into.join(name);
            if entry.is_dir() {
                std::fs::create_dir_all(&path).map_err(|e| bad(&e))?;
                continue;
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| bad(&e))?;
            }
            let mut out = File::create(&path).map_err(|e| bad(&e))?;
            std::io::copy(&mut entry, &mut out).map_err(|e| bad(&e))?;
        }
        Ok(())
    })();
    if unpacked.is_err() {
        let _ = std::fs::remove_dir_all(into);
    }
    let _ = std::fs::remove_file(zip);
    unpacked
}

/// The steps that copy the disk image's app out to `staged`: mounted
/// read-only where Finder doesn't show it, copied with `ditto` (which keeps
/// the app's signature, links and attributes), and let go of.
pub fn mac_copy_out(dmg: &Path, mount: &Path, staged: &Path, app: &Path) -> Vec<crate::Step> {
    use crate::Step;
    let name = app.file_name().map_or_else(|| "Moonglow Toolset.app".into(), |n| n.to_owned());
    vec![
        Step::RemoveAll(staged.to_path_buf()),
        Step::Run {
            program: "hdiutil".into(),
            args: vec![
                "attach".into(),
                "-nobrowse".into(),
                "-readonly".into(),
                "-noautoopen".into(),
                "-mountpoint".into(),
                mount.into(),
                dmg.into(),
            ],
            wait: true,
        },
        Step::Run {
            program: "ditto".into(),
            args: vec![mount.join("Moonglow Toolset.app").into(), staged.into()],
            wait: true,
        },
        // (Let go of even when the copy failed: these steps are each run,
        // `run_all`, and the first failure is said.)
        Step::Run {
            program: "hdiutil".into(),
            args: vec!["detach".into(), mount.into(), "-force".into()],
            wait: true,
        },
        Step::Exists(staged.join("Contents/MacOS/moonglow"), name.to_string_lossy().into()),
    ]
}

/// Removes what a failed or interrupted update left: in `cache` (the
/// updates' cache folder) all but the last install's report, and beside
/// the program what is named [`PREFIX`]. Nothing staged outlives the run
/// that staged it; and while another run is downloading or has an update
/// staged (it holds the cache's lock, [`LOCK`]), nothing is removed: it is
/// that run's, and the start after it cleans.
pub fn clean(kind: &Kind, cache: &Path) {
    // (Locked here for as long as it takes, so that no download begins
    // in the middle of it; the lock file itself is never removed, or two
    // runs could each lock one of their own.)
    let _lock = if held().iter().any(|(c, _)| c == cache) {
        None
    } else {
        match File::open(cache.join(LOCK)) {
            Ok(file) => match file.try_lock() {
                Err(std::fs::TryLockError::WouldBlock) => return,
                _ => Some(file),
            },
            Err(_) => None,
        }
    };
    if let Ok(entries) = std::fs::read_dir(cache) {
        for e in entries.flatten() {
            let name = e.file_name();
            if name != crate::install::REPORT && name != LOCK {
                remove(&e.path());
            }
        }
    }
    if let Some(folder) = kind.folder()
        && let Ok(entries) = std::fs::read_dir(folder)
    {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with(PREFIX) {
                remove(&e.path());
            }
        }
    }
}

fn remove(path: &Path) {
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::from_fn;
    use crate::release::Asset;
    use crate::verify::testing::TestKey;

    /// A release `tag` with `files` at `mem://tag/name`.
    fn release(tag: &str, files: &[(String, Vec<u8>)]) -> Release {
        Release {
            version: Version::parse(tag).unwrap(),
            tag: tag.into(),
            page: crate::RELEASES_PAGE.into(),
            notes: String::new(),
            assets: files
                .iter()
                .map(|(name, body)| Asset {
                    name: name.clone(),
                    url: format!("mem://{tag}/{name}"),
                    size: Some(body.len() as u64),
                })
                .collect(),
        }
    }

    fn serving(files: Vec<(String, Vec<u8>)>, tag: &str) -> std::sync::Arc<dyn Http> {
        let tag = tag.to_string();
        from_fn(move |url| {
            let name = url.strip_prefix(&format!("mem://{tag}/")).ok_or("elsewhere")?;
            files.iter().find(|(n, _)| n == name).map(|(_, b)| b.clone()).ok_or("404".into())
        })
    }

    fn image_kind(dir: &Path) -> (Kind, PathBuf) {
        let image = dir.join("Moonglow-3.2.0-x86_64.AppImage");
        std::fs::write(&image, b"the old AppImage").unwrap();
        (Kind::AppImage { image: image.clone(), arch: "x86_64".into() }, image)
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut n: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        n.sort();
        n
    }

    #[test]
    fn an_appimage_is_downloaded_beside_the_image_checked_and_staged() {
        let dir = crate::tests::scratch("stage-appimage");
        let cache = dir.join("cache");
        let (kind, image) = image_kind(&dir);
        let key = TestKey::new();
        let files =
            crate::serve::signed(&key, "v3.3.0", &[("Moonglow-3.3.0-x86_64.AppImage", b"new")]);
        let release = release("v3.3.0", &files);
        let http = serving(files.clone(), "v3.3.0");
        let mut seen = Vec::new();
        let staged = fetch(&*http, &key.public(), &release, &kind, &cache, &mut |got, of| {
            seen.push((got, of))
        })
        .unwrap();
        assert_eq!(seen.last(), Some(&(3, Some(3))));
        let Ready::Image { file, .. } = &staged.ready else { panic!("{staged:?}") };
        assert_eq!(file, &dir.join(".moonglow-update-3.3.0.AppImage"));
        assert_eq!(std::fs::read(file).unwrap(), b"new");
        // (Runnable, as the image is.)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(file).unwrap().permissions().mode();
            assert_eq!(mode & 0o700, 0o700, "{mode:o}");
        }
        // The image itself isn't touched until it is installed.
        assert_eq!(std::fs::read(&image).unwrap(), b"the old AppImage");
        assert_eq!(
            names(&dir),
            [".moonglow-update-3.3.0.AppImage", "Moonglow-3.2.0-x86_64.AppImage", "cache"]
        );
        // Installed: renamed over the image, its path kept.
        let restart = crate::install::install(&staged, true, &cache).unwrap();
        assert_eq!(restart, crate::Restart::Exe(image.clone()));
        assert_eq!(std::fs::read(&image).unwrap(), b"new");
        assert_eq!(names(&dir), ["Moonglow-3.2.0-x86_64.AppImage", "cache"]);
    }

    #[test]
    fn a_package_that_doesn_t_verify_is_never_kept() {
        let dir = crate::tests::scratch("stage-refused");
        let cache = dir.join("cache");
        let (kind, image) = image_kind(&dir);
        let key = TestKey::new();
        let name = "Moonglow-3.3.0-x86_64.AppImage";
        let files = crate::serve::signed(&key, "v3.3.0", &[(name, b"new")]);
        let fetch_with = |files: Vec<(String, Vec<u8>)>, key: &PublicKey| {
            let release = release("v3.3.0", &files);
            fetch(&*serving(files, "v3.3.0"), key, &release, &kind, &cache, &mut |_, _| {})
        };
        // The package changed after it was signed.
        let mut tampered = files.clone();
        tampered[0].1 = b"evil".to_vec();
        let e = fetch_with(tampered, &key.public()).unwrap_err();
        assert!(e.contains("isn't the one the release signed"), "{e}");
        // Signed by another key.
        let e = fetch_with(files.clone(), &TestKey::new().public()).unwrap_err();
        assert!(e.contains("another key"), "{e}");
        // Not signed at all.
        let unsigned: Vec<_> = files.iter().filter(|(n, _)| n == name).cloned().collect();
        let e = fetch_with(unsigned, &key.public()).unwrap_err();
        assert_eq!(e, "the release isn't signed");
        // Without this system's package.
        let other = crate::serve::signed(&key, "v3.3.0", &[("Moonglow-3.3.0-macos.dmg", b"x")]);
        let e = fetch_with(other, &key.public()).unwrap_err();
        assert_eq!(e, format!("the release has no {name}"));
        // A download that broke off.
        let release = release("v3.3.0", &files);
        let broken = from_fn(|url| {
            if url.ends_with(".AppImage") {
                Err("the download broke off".into())
            } else {
                Ok(Vec::new())
            }
        });
        let http = serving(files.clone(), "v3.3.0");
        let half = from_fn(move |url| {
            if url.ends_with(".AppImage") {
                broken.ask(url).map(String::into_bytes)
            } else {
                crate::http::get(&*http, url, 1 << 20)
            }
        });
        let e = fetch(&*half, &key.public(), &release, &kind, &cache, &mut |_, _| {}).unwrap_err();
        assert_eq!(e, "the download broke off");
        // Nothing left but the image as it was (and the lock's file, let
        // go of).
        assert_eq!(names(&dir), ["Moonglow-3.2.0-x86_64.AppImage", "cache"]);
        assert_eq!(names(&cache), [LOCK]);
        assert!(held().iter().all(|(c, _)| *c != cache));
        assert_eq!(std::fs::read(&image).unwrap(), b"the old AppImage");
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_that_can_t_be_written_is_said_before_downloading() {
        use std::os::unix::fs::PermissionsExt;
        let dir = crate::tests::scratch("stage-read-only");
        let (kind, _) = image_kind(&dir);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        // (As root, which the tests aren't run as, it is writable all the same.)
        let writable = std::fs::File::create(dir.join("probe")).is_ok();
        let key = TestKey::new();
        let files =
            crate::serve::signed(&key, "v3.3.0", &[("Moonglow-3.3.0-x86_64.AppImage", b"new")]);
        let release = release("v3.3.0", &files);
        let asked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = asked.clone();
        let http = from_fn(move |_| {
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Vec::new())
        });
        let e = fetch(&*http, &key.public(), &release, &kind, &dir.join("cache"), &mut |_, _| {});
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        if writable {
            return;
        }
        assert_eq!(e.unwrap_err(), unwritable(&dir));
        assert_eq!(asked.load(std::sync::atomic::Ordering::SeqCst), 0, "nothing downloaded");
    }

    #[test]
    fn the_zip_is_unpacked_beside_the_program_and_its_files_swapped_in() {
        let dir = crate::tests::scratch("stage-zip");
        let program = dir.join("Moonglow");
        std::fs::create_dir_all(program.join("manual")).unwrap();
        for (p, b) in [
            ("moonglow.exe", "old moonglow"),
            ("mg.exe", "old mg"),
            ("manual/01.md", "old manual"),
            ("my-notes.txt", "the builder's"),
        ] {
            std::fs::write(program.join(p), b).unwrap();
        }
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for (p, b) in [
            ("moonglow.exe", "new moonglow"),
            ("mg.exe", "new mg"),
            ("manual/01.md", "new manual"),
            ("manual/02.md", "a new chapter"),
        ] {
            zip.start_file(p, options).unwrap();
            zip.write_all(b.as_bytes()).unwrap();
        }
        let zip = zip.finish().unwrap().into_inner();
        let key = TestKey::new();
        let name = "Moonglow-3.3.0-windows-x64.zip";
        let files = crate::serve::signed(&key, "v3.3.0", &[(name, &zip)]);
        let kind = Kind::WindowsZip { dir: program.clone() };
        let staged = fetch(
            &*serving(files.clone(), "v3.3.0"),
            &key.public(),
            &release("v3.3.0", &files),
            &kind,
            &dir.join("cache"),
            &mut |_, _| {},
        )
        .unwrap();
        assert_eq!(staged.ready, Ready::Files { dir: program.join(".moonglow-update-3.3.0") });
        assert_eq!(
            names(&program),
            [".moonglow-update-3.3.0", "manual", "mg.exe", "moonglow.exe", "my-notes.txt"]
        );
        let restart = crate::install::install(&staged, true, &dir.join("cache")).unwrap();
        assert_eq!(restart, crate::Restart::Exe(program.join("moonglow.exe")));
        let read = |p: &str| std::fs::read_to_string(program.join(p)).unwrap();
        assert_eq!(read("moonglow.exe"), "new moonglow");
        assert_eq!(read("mg.exe"), "new mg");
        assert_eq!(read("manual/01.md"), "new manual");
        assert_eq!(read("manual/02.md"), "a new chapter");
        // The builder's own file is left; the old ones are set aside, to
        // be removed at the next start (Windows can't remove a program
        // that runs).
        assert_eq!(read("my-notes.txt"), "the builder's");
        assert_eq!(read(".moonglow-update-old/moonglow.exe"), "old moonglow");
        assert!(!program.join(".moonglow-update-3.3.0").exists());
        clean(&kind, &dir.join("cache"));
        assert_eq!(names(&program), ["manual", "mg.exe", "moonglow.exe", "my-notes.txt"]);
    }

    #[test]
    fn a_zip_entry_outside_its_folder_is_refused() {
        let dir = crate::tests::scratch("stage-zip-slip");
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("../escaped.exe", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"x").unwrap();
        let path = dir.join("z.zip");
        std::fs::write(&path, zip.finish().unwrap().into_inner()).unwrap();
        let e = unzip(&path, &dir.join("into")).unwrap_err();
        assert!(e.contains("lies outside it"), "{e}");
        assert!(!dir.join("escaped.exe").exists() && !dir.join("into").exists());
    }

    #[test]
    fn the_disk_image_s_app_is_copied_out_by_these_steps() {
        use crate::Step;
        let steps = mac_copy_out(
            Path::new("/c/Moonglow-3.3.0-macos.dmg"),
            Path::new("/c/m"),
            Path::new("/Applications/.moonglow-update-3.3.0.app"),
            Path::new("/Applications/Moonglow Toolset.app"),
        );
        let run: Vec<String> = steps
            .iter()
            .filter_map(|s| match s {
                Step::Run { program, args, .. } => Some(format!(
                    "{} {}",
                    program.display(),
                    args.iter().map(|a| a.to_string_lossy()).collect::<Vec<_>>().join(" ")
                )),
                _ => None,
            })
            .collect();
        // (The app's path joined as the system joins it: with `\` where the
        // test runs on Windows.)
        let app = Path::new("/c/m").join("Moonglow Toolset.app");
        assert_eq!(
            run,
            [
                "hdiutil attach -nobrowse -readonly -noautoopen -mountpoint /c/m /c/Moonglow-3.3.0-macos.dmg"
                    .to_string(),
                format!("ditto {} /Applications/.moonglow-update-3.3.0.app", app.display()),
                "hdiutil detach /c/m -force".to_string(),
            ]
        );
        assert_eq!(steps[0], Step::RemoveAll("/Applications/.moonglow-update-3.3.0.app".into()));
        assert!(
            matches!(steps.last(), Some(Step::Exists(p, _)) if p.ends_with("Contents/MacOS/moonglow"))
        );
    }

    #[test]
    fn what_an_interrupted_update_left_is_removed_at_start() {
        let dir = crate::tests::scratch("stage-clean");
        let cache = dir.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let (kind, _) = image_kind(&dir);
        for f in [
            ".moonglow-update-Moonglow-3.3.0-x86_64.AppImage.part",
            ".moonglow-update-3.3.0.AppImage",
            "Other.AppImage",
            ".hidden",
        ] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        std::fs::write(cache.join("Moonglow-3.3.0-windows-x64-setup.exe.part"), b"x").unwrap();
        std::fs::write(cache.join(crate::install::REPORT), b"x").unwrap();
        clean(&kind, &cache);
        assert_eq!(
            names(&dir),
            [".hidden", "Moonglow-3.2.0-x86_64.AppImage", "Other.AppImage", "cache"]
        );
        assert_eq!(names(&cache), [crate::install::REPORT]);
    }

    /// A second Moonglow started while the first downloads or has an
    /// update staged for its close: it removes nothing of the first's
    /// (which holds the cache's lock) and doesn't download over it; once
    /// the first has ended, the next start cleans as before.
    #[test]
    fn another_run_s_staged_update_is_left_alone() {
        let dir = crate::tests::scratch("stage-two-runs");
        let cache = dir.join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let (kind, _) = image_kind(&dir);
        let staged = dir.join(".moonglow-update-3.3.0.AppImage");
        let part = cache.join("Moonglow-3.3.0-macos.dmg.part");
        std::fs::write(&staged, b"new").unwrap();
        std::fs::write(&part, b"half").unwrap();
        // The other run: a handle of its own on the lock, as another
        // process has.
        let theirs = File::create(cache.join(LOCK)).unwrap();
        theirs.try_lock().unwrap();
        clean(&kind, &cache);
        assert!(staged.exists() && part.exists());
        let key = TestKey::new();
        let files =
            crate::serve::signed(&key, "v3.3.0", &[("Moonglow-3.3.0-x86_64.AppImage", b"new")]);
        let release = release("v3.3.0", &files);
        let http = serving(files.clone(), "v3.3.0");
        let e = fetch(&*http, &key.public(), &release, &kind, &cache, &mut |_, _| {}).unwrap_err();
        assert_eq!(e, BUSY);
        assert_eq!(std::fs::read(&staged).unwrap(), b"new");
        // It has ended.
        drop(theirs);
        clean(&kind, &cache);
        assert!(!staged.exists() && !part.exists());
        assert_eq!(names(&cache), [LOCK]);
        // This run's own staged update is its own to clean (a new
        // download begins with it), and held against the next run.
        fetch(&*http, &key.public(), &release, &kind, &cache, &mut |_, _| {}).unwrap();
        assert!(staged.exists());
        assert!(matches!(
            File::open(cache.join(LOCK)).unwrap().try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        clean(&kind, &cache);
        assert!(!staged.exists());
        release_lock(&cache);
    }
}
