//! Attaching content to a module in one step (the job of the PRC installer
//! and NWN Installer Tools): haks and a talk table, wherever they were
//! downloaded to, are copied into the user's `hak` and `tlk` folders, where
//! the game looks for them, and the module lists the haks (at the top, in
//! the order given) and names the talk table. Content downloaded as an
//! archive (zip, 7z or RAR) is taken from it as it is, not unpacked first.

mod archive;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use mg_gff::{Struct, Value};
use mg_schema::{ExoString, StructExt, ifo};

/// What a file of the same name in the user's folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum There {
    /// Nothing: the file is copied.
    Nothing,
    /// The same file, or one with the same bytes: nothing to copy.
    Same,
    /// Another file of that name: replaced only if asked.
    Different,
}

/// A hak or talk table and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The file, or the archive (zip, 7z or RAR) it is in.
    pub from: PathBuf,
    /// Its name in the archive `from` (content downloaded as an archive
    /// is taken as it is, not unpacked first).
    pub entry: Option<String>,
    pub to: PathBuf,
    pub there: There,
    /// The name the module knows it by (the file's, without its extension).
    pub name: String,
    pub is_tlk: bool,
}

/// The file's name of an archive's entry `name` (without the folders it
/// is in) when it is a hak or a talk table; `None` for the rest, and for
/// a name that would lead out of a folder (none of ours).
fn content_file(name: &str) -> Option<&str> {
    if name.ends_with(['/', '\\']) || name.split(['/', '\\']).any(|part| part == "..") {
        return None;
    }
    let file = name.rsplit(['/', '\\']).next().filter(|f| !f.is_empty())?;
    let ext = Path::new(file).extension()?.to_string_lossy().to_ascii_lowercase();
    matches!(ext.as_str(), "hak" | "tlk").then_some(file)
}

/// The haks and talk tables in the archive at `path`: each entry's name
/// in it, its file's name and its size. An archive with neither is an
/// error, as is one that can't be read without a password.
fn archive_entries(kind: archive::Kind, path: &Path) -> Result<Vec<(String, String, u64)>, String> {
    let listed = archive::list(kind, path, &|name| content_file(name).is_some())?;
    let mut out: Vec<(String, String, u64)> = listed
        .into_iter()
        .filter_map(|l| Some((content_file(&l.name)?.to_string(), l)))
        .map(|(file, l)| (l.name, file, l.size))
        .collect();
    out.sort();
    if out.is_empty() {
        return Err(format!("{}: has no hak or talk table in it", path.display()));
    }
    Ok(out)
}

/// Whether two files have the same bytes.
fn same_bytes(a: &Path, b: &Path) -> bool {
    let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) else { return false };
    if ma.len() != mb.len() {
        return false;
    }
    let (Ok(mut fa), Ok(mut fb)) = (std::fs::File::open(a), std::fs::File::open(b)) else {
        return false;
    };
    let (mut ba, mut bb) = (vec![0u8; 1 << 16], vec![0u8; 1 << 16]);
    loop {
        let n = match fa.read(&mut ba) {
            Ok(n) => n,
            Err(_) => return false,
        };
        if n == 0 {
            return true;
        }
        if fb.read_exact(&mut bb[..n]).is_err() || ba[..n] != bb[..n] {
            return false;
        }
    }
}

/// Which of `placed` (taken from the archive `from`, each with its size
/// there) have the bytes of the file in their place: those of the same
/// size are read from the archive, in one pass, and compared.
fn same_in_archive(
    kind: archive::Kind,
    from: &Path,
    placed: &[(&Placement, u64)],
) -> Result<BTreeSet<String>, String> {
    let alike: BTreeMap<String, &Path> = placed
        .iter()
        .filter(|(p, size)| std::fs::metadata(&p.to).is_ok_and(|m| m.is_file() && m.len() == *size))
        .filter_map(|(p, _)| Some((p.entry.clone()?, p.to.as_path())))
        .collect();
    let mut flags = Vec::new();
    let wanted: BTreeSet<String> = alike.keys().cloned().collect();
    archive::extract(kind, from, &wanted, &mut |name| {
        let to = alike[name];
        let (compare, same) =
            archive::Compare::new(to).map_err(|e| format!("{}: {e}", to.display()))?;
        flags.push((name.to_string(), same));
        Ok(Box::new(compare))
    })?;
    Ok(flags.into_iter().filter(|(_, same)| same.get()).map(|(name, _)| name).collect())
}

/// Where each file goes in the user folder `user_dir`: `.hak` files to
/// `hak`, `.tlk` files to `tlk`; a `.zip`, `.7z` or `.rar` archive's haks
/// and talk table alike, from the archive (wherever in it they are). An
/// error names a file that's none of these, an archive with neither in
/// it (or that can't be read: one protected by a password, one volume of
/// a set), or a second talk table (a module has one).
pub fn placements(user_dir: &Path, files: &[PathBuf]) -> Result<Vec<Placement>, String> {
    let mut out: Vec<Placement> = Vec::new();
    for from in files {
        let ext = from.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
        let kind = archive::Kind::of(from);
        // (A file's name as it is, bytes the system's text can't spell
        // too: a hak's name is its file's bytes, `mg_resman::name_of_file`.)
        let found: Vec<(Option<String>, std::ffi::OsString, u64)> = match (kind, ext.as_deref()) {
            (Some(kind), _) => archive_entries(kind, from)?
                .into_iter()
                .map(|(entry, file, size)| (Some(entry), file.into(), size))
                .collect(),
            (None, Some("hak" | "tlk")) => {
                vec![(None, from.file_name().unwrap_or_default().to_os_string(), 0)]
            }
            _ => {
                return Err(format!(
                    "{}: not a hak, a talk table or an archive of them (zip, 7z or RAR)",
                    from.display()
                ));
            }
        };
        let first = out.len();
        let mut sizes = Vec::new();
        for (entry, file, size) in found {
            let file_path = Path::new(&file);
            let ext = file_path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
            let is_tlk = ext.as_deref() == Some("tlk");
            let stem = file_path.file_stem().unwrap_or_default();
            let name = mg_resman::name_of_file(stem);
            if is_tlk && name.len() > 16 {
                return Err(format!("{name}.tlk: a talk table's name has at most 16 characters"));
            }
            if is_tlk && out.iter().any(|p| p.is_tlk) {
                return Err("a module has one talk table: choose one".into());
            }
            let to = user_dir.join(if is_tlk { "tlk" } else { "hak" }).join(&file);
            let same_file = entry.is_none()
                && std::fs::canonicalize(from)
                    .ok()
                    .is_some_and(|f| std::fs::canonicalize(&to).ok().is_some_and(|t| t == f));
            let there = if same_file || (entry.is_none() && to.is_file() && same_bytes(from, &to)) {
                There::Same
            } else if to.exists() {
                There::Different
            } else {
                There::Nothing
            };
            out.push(Placement { from: from.clone(), entry, to, there, name, is_tlk });
            sizes.push(size);
        }
        // (An archive's files are compared with those in their places once
        // all of them are known: one pass over it.)
        if let Some(kind) = kind {
            let placed: Vec<(&Placement, u64)> = out[first..].iter().zip(sizes).collect();
            let same = same_in_archive(kind, from, &placed)?;
            for p in &mut out[first..] {
                if p.entry.as_ref().is_some_and(|e| same.contains(e)) {
                    p.there = There::Same;
                }
            }
        }
    }
    // Two of one name would go to one place.
    let source = |p: &Placement| match &p.entry {
        Some(entry) => format!("{entry} in {}", p.from.display()),
        None => p.from.display().to_string(),
    };
    for (i, p) in out.iter().enumerate() {
        let file = p.to.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
        let twin = out[..i].iter().find(|q| {
            q.is_tlk == p.is_tlk
                && q.to
                    .file_name()
                    .is_some_and(|f| f.to_string_lossy().to_ascii_lowercase() == file)
        });
        if let Some(q) = twin {
            return Err(format!(
                "two files are named {file} ({} and {}): choose one",
                source(q),
                source(p)
            ));
        }
    }
    Ok(out)
}

/// Copies the files that need copying (those that differ only with
/// `replace`); returns how many were copied. Each is written beside its
/// place and moved there, so a failed copy leaves nothing half-written;
/// an archive's are read from it in one pass.
pub fn copy(placements: &[Placement], replace: bool) -> Result<usize, String> {
    let go: Vec<&Placement> = placements
        .iter()
        .filter(|p| match p.there {
            There::Nothing => true,
            There::Different => replace,
            There::Same => false,
        })
        .collect();
    let part = |p: &Placement| p.to.with_extension("part");
    for p in &go {
        let dir = p.to.parent().expect("in a folder");
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut written = Ok(());
    // Files as they are, then each archive's.
    for p in go.iter().filter(|p| p.entry.is_none()) {
        written = std::fs::copy(&p.from, part(p))
            .map(|_| ())
            .map_err(|e| format!("{}: {e}", p.to.display()));
        if written.is_err() {
            break;
        }
    }
    let mut archives: BTreeMap<&Path, BTreeMap<String, PathBuf>> = BTreeMap::new();
    for p in &go {
        if let Some(entry) = &p.entry {
            archives.entry(&p.from).or_default().insert(entry.clone(), part(p));
        }
    }
    for (from, parts) in &archives {
        if written.is_err() {
            break;
        }
        let Some(kind) = archive::Kind::of(from) else {
            written = Err(format!("{}: not an archive Moonglow reads", from.display()));
            break;
        };
        let wanted: BTreeSet<String> = parts.keys().cloned().collect();
        written = archive::extract(kind, from, &wanted, &mut |name| {
            let to = &parts[name];
            let f = std::fs::File::create(to).map_err(|e| format!("{}: {e}", to.display()))?;
            Ok(Box::new(f))
        });
    }
    if let Err(e) = written {
        for p in &go {
            let _ = std::fs::remove_file(part(p));
        }
        return Err(e);
    }
    for (i, p) in go.iter().enumerate() {
        if let Err(e) = std::fs::rename(part(p), &p.to) {
            for q in &go[i..] {
                let _ = std::fs::remove_file(part(q));
            }
            return Err(format!("{}: {e}", p.to.display()));
        }
    }
    Ok(go.len())
}

/// The module's hak list with `haks` at the top, in order (one already
/// listed moves up); each name written as [`mg_resman::file_name_bytes`]
/// has it, the bytes of its file's name.
pub fn hak_list(info: &Struct, haks: &[String]) -> Value {
    let name =
        |s: &Struct| mg_resman::file_name_text(s.read(&ifo::mod_hak_list::MOD_HAK).as_bytes());
    let mut items: Vec<Struct> = haks
        .iter()
        .map(|h| {
            let mut item = ifo::MOD_HAK_LIST.new_item();
            item.write(&ifo::mod_hak_list::MOD_HAK, ExoString(mg_resman::file_name_bytes(h)));
            item
        })
        .collect();
    items.extend(
        info.items(&ifo::MOD_HAK_LIST)
            .iter()
            .filter(|s| !haks.iter().any(|h| h.eq_ignore_ascii_case(&name(s))))
            .cloned(),
    );
    Value::List(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Content downloaded as a zip archive: its haks and its talk table,
    /// wherever in it, go to the user folder as files would, unpacked from
    /// it; what else it holds stays in it, and a name that climbs out of
    /// its folder is left out. One the same as the archive's is not copied.
    #[test]
    fn a_zip_s_haks_and_talk_table_go_to_the_user_folder() {
        use std::io::Write as _;
        let dir = mg_testkit::scratch_dir("mg-attach-zip");
        let user = dir.join("user");
        let zipped = dir.join("content.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zipped).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for (name, data) in [
            ("Content/haks/z_one.hak", "one"),
            ("Content/z_two.HAK", "two"),
            ("Content/tlk/z_text.tlk", "text"),
            ("Content/readme.txt", "read me"),
            ("../z_evil.hak", "evil"),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(data.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        let p = placements(&user, std::slice::from_ref(&zipped)).unwrap();
        let names: Vec<(&str, bool)> = p.iter().map(|p| (p.name.as_str(), p.is_tlk)).collect();
        assert_eq!(names, [("z_one", false), ("z_text", true), ("z_two", false)]);
        assert_eq!(p[1].to, user.join("tlk/z_text.tlk"));
        assert_eq!(copy(&p, false).unwrap(), 3);
        assert_eq!(std::fs::read_to_string(user.join("hak/z_one.hak")).unwrap(), "one");
        assert_eq!(std::fs::read_to_string(user.join("hak/z_two.HAK")).unwrap(), "two");
        assert!(!dir.join("z_evil.hak").exists() && !user.join("hak/z_evil.hak").exists());
        let again = placements(&user, std::slice::from_ref(&zipped)).unwrap();
        assert!(again.iter().all(|p| p.there == There::Same), "{again:?}");
        // An archive with neither in it, or no archive at all.
        let empty = dir.join("empty.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&empty).unwrap());
        zip.start_file("readme.txt", options).unwrap();
        zip.finish().unwrap();
        assert!(placements(&user, &[empty]).unwrap_err().contains("no hak or talk table"));
        std::fs::write(dir.join("bad.zip"), b"not a zip").unwrap();
        assert!(placements(&user, &[dir.join("bad.zip")]).is_err());
        // Two files of one name, in it and beside it, would go to one place.
        std::fs::write(dir.join("z_one.hak"), "one beside").unwrap();
        let twice = placements(&user, &[zipped.clone(), dir.join("z_one.hak")]).unwrap_err();
        assert!(twice.contains("two files are named z_one.hak"), "{twice}");
        // Packed in a way this build doesn't read (BZip2, method 12): said
        // so, not taken for missing.
        let odd = dir.join("odd.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&odd).unwrap());
        let stored = options.compression_method(zip::CompressionMethod::Stored);
        zip.start_file("z_odd.hak", stored).unwrap();
        zip.write_all(b"odd").unwrap();
        zip.finish().unwrap();
        let mut bytes = std::fs::read(&odd).unwrap();
        let central = bytes.windows(4).position(|w| w == b"PK\x01\x02").unwrap();
        for at in [8, central + 10] {
            bytes[at] = 12;
        }
        std::fs::write(&odd, bytes).unwrap();
        let p = placements(&user, &[odd]).unwrap();
        let e = copy(&p, false).unwrap_err();
        assert!(e.contains("z_odd.hak") && !e.contains("has no"), "{e}");
        assert!(!user.join("hak/z_odd.hak").exists() && !user.join("hak/z_odd.part").exists());
    }

    /// The archive fixtures (`tests/fixtures/archives`), our own text in
    /// each: `Content/haks/z_one.hak`, `Content/z_two.HAK`,
    /// `Content/tlk/z_text.tlk` and `Content/readme.txt`. The 7z ones were
    /// made by 7-Zip 26 (solid LZMA2; `locked.7z` with a password,
    /// `hidden.7z` its names hidden too), the RAR ones by rars 0.12.3
    /// (`content-rar4.rar` in RAR 2.9's format, which WinRAR 3 and 4 write,
    /// solid, with a comment; `content-rar5.rar` solid, with a recovery
    /// record; `locked.rar`, `hidden.rar` and `split.part01.rar`, the first
    /// of two volumes), each tested by the genuine UnRAR 7.30 and 7-Zip.
    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/archives").join(name)
    }

    /// The fixtures' files, as they are in each.
    fn fixture_files() -> [(&'static str, String); 3] {
        let text = |name: &str, lines: usize| -> String {
            (0..lines)
                .map(|i| {
                    format!(
                        "{name} line {i}: Moonglow's archive fixture, {} of thirteen.\n",
                        i * 7 % 13
                    )
                })
                .collect()
        };
        [
            ("hak/z_one.hak", text("one", 60)),
            ("hak/z_two.HAK", text("two", 40)),
            ("tlk/z_text.tlk", text("text", 30)),
        ]
    }

    /// Content downloaded as a 7z archive, or a RAR one (RAR 4 or 5):
    /// its haks and talk table go to the user folder as a zip's do, read
    /// from the archive (a solid one, decoded once); compared with those
    /// there by their bytes.
    #[test]
    fn a_7z_s_and_a_rar_s_haks_and_talk_table_go_to_the_user_folder() {
        for archive in ["content.7z", "content-rar4.rar", "content-rar5.rar"] {
            let dir = mg_testkit::scratch_dir(&format!("mg-attach-{archive}"));
            let user = dir.join("user");
            let from = fixture(archive);
            let p = placements(&user, std::slice::from_ref(&from)).unwrap();
            let names: Vec<(&str, bool)> = p.iter().map(|p| (p.name.as_str(), p.is_tlk)).collect();
            assert_eq!(names, [("z_one", false), ("z_text", true), ("z_two", false)], "{archive}");
            assert_eq!(p[0].entry.as_deref(), Some("Content/haks/z_one.hak"), "{archive}");
            assert!(p.iter().all(|p| p.there == There::Nothing), "{archive}: {p:?}");
            assert_eq!(copy(&p, false).unwrap(), 3, "{archive}");
            for (file, text) in fixture_files() {
                assert_eq!(std::fs::read_to_string(user.join(file)).unwrap(), text, "{archive}");
            }
            assert!(!user.join("hak/readme.txt").exists());
            let again = placements(&user, std::slice::from_ref(&from)).unwrap();
            assert!(again.iter().all(|p| p.there == There::Same), "{archive}: {again:?}");
            // Of the same size and other bytes: different, and replaced
            // only when asked.
            let mut other = fixture_files()[1].1.clone().into_bytes();
            other[0] = b'T';
            std::fs::write(user.join("hak/z_two.HAK"), &other).unwrap();
            let p = placements(&user, std::slice::from_ref(&from)).unwrap();
            let there: Vec<There> = p.iter().map(|p| p.there).collect();
            assert_eq!(there, [There::Same, There::Same, There::Different], "{archive}");
            assert_eq!(copy(&p, false).unwrap(), 0);
            assert_eq!(std::fs::read(user.join("hak/z_two.HAK")).unwrap(), other);
            assert_eq!(copy(&p, true).unwrap(), 1);
            let two = std::fs::read_to_string(user.join("hak/z_two.HAK")).unwrap();
            assert_eq!(two, fixture_files()[1].1, "{archive}");
            assert!(
                std::fs::read_dir(user.join("hak"))
                    .unwrap()
                    .all(|f| { f.unwrap().path().extension().is_none_or(|e| e != "part") })
            );
        }
    }

    /// An archive Moonglow can't read whole, said so and why: one with a
    /// password (its files' or its names too), one volume of a set.
    #[test]
    fn locked_and_split_archives_are_refused_and_said_why() {
        let user = mg_testkit::scratch_dir("mg-attach-locked").join("user");
        for archive in ["locked.7z", "hidden.7z", "locked.rar", "hidden.rar"] {
            let e = placements(&user, &[fixture(archive)]).unwrap_err();
            assert!(e.contains(archive) && e.contains("password"), "{archive}: {e}");
        }
        let e = placements(&user, &[fixture("split.part01.rar")]).unwrap_err();
        assert!(e.contains("one volume of a set"), "{e}");
        assert!(!user.exists(), "nothing written");
    }

    /// A cut or damaged archive (every length it can be cut at, every
    /// byte changed in turn) is an error, or (where the change is in what
    /// nothing checks) gives the files as they are: never a panic, never
    /// another file's bytes or half of one in the user folder.
    #[test]
    fn a_cut_or_damaged_archive_is_an_error_never_a_panic() {
        use std::io::Write as _;
        let dir = mg_testkit::scratch_dir("mg-attach-damaged");
        let zipped = dir.join("made.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&zipped).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for (file, text) in fixture_files() {
            zip.start_file(format!("Content/{}", &file[4..]), options).unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        let expected = fixture_files();
        for archive in [
            zipped,
            fixture("content.7z"),
            fixture("content-rar4.rar"),
            fixture("content-rar5.rar"),
        ] {
            let bytes = std::fs::read(&archive).unwrap();
            let name = archive.file_name().unwrap().to_string_lossy().into_owned();
            let damaged = dir.join(&name);
            let user = dir.join("user");
            let check = |data: &[u8], what: &str| {
                std::fs::write(&damaged, data).unwrap();
                let _ = std::fs::remove_dir_all(&user);
                let copied =
                    placements(&user, std::slice::from_ref(&damaged)).and_then(|p| copy(&p, false));
                let Ok(_) = copied else { return };
                for (file, text) in &expected {
                    let path = user.join(file);
                    if let Ok(got) = std::fs::read(&path) {
                        assert_eq!(got, text.as_bytes(), "{name} {what}: {file}");
                    }
                }
            };
            for len in 0..bytes.len() {
                check(&bytes[..len], &format!("cut at {len}"));
            }
            for at in 0..bytes.len() {
                let mut data = bytes.clone();
                data[at] ^= 0x5a;
                check(&data, &format!("byte {at} changed"));
            }
        }
    }

    #[test]
    fn haks_and_a_talk_table_go_to_the_user_folder() {
        let dir = std::env::temp_dir().join(format!("mg-attach-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (dl, user) = (dir.join("download"), dir.join("user"));
        std::fs::create_dir_all(&dl).unwrap();
        std::fs::create_dir_all(user.join("hak")).unwrap();
        for (name, data) in [("a.hak", "a"), ("b.hak", "b"), ("c.hak", "c"), ("t.tlk", "t")] {
            std::fs::write(dl.join(name), data).unwrap();
        }
        std::fs::write(user.join("hak/b.hak"), "b").unwrap();
        std::fs::write(user.join("hak/c.hak"), "old c").unwrap();
        let files: Vec<PathBuf> =
            ["a.hak", "b.hak", "c.hak", "t.tlk"].iter().map(|n| dl.join(n)).collect();
        let p = placements(&user, &files).unwrap();
        let there: Vec<There> = p.iter().map(|p| p.there).collect();
        assert_eq!(there, [There::Nothing, There::Same, There::Different, There::Nothing]);
        assert_eq!(p[3].to, user.join("tlk/t.tlk"));
        assert_eq!(copy(&p, false).unwrap(), 2);
        assert_eq!(std::fs::read_to_string(user.join("hak/c.hak")).unwrap(), "old c");
        assert_eq!(copy(&placements(&user, &files).unwrap(), true).unwrap(), 1);
        assert_eq!(std::fs::read_to_string(user.join("hak/c.hak")).unwrap(), "c");
        // Already in place: nothing to copy.
        let again = placements(&user, &[user.join("hak/a.hak")]).unwrap();
        assert_eq!(again[0].there, There::Same);
        assert!(placements(&user, &[dl.join("x.erf")]).is_err());
        assert!(placements(&user, &[dl.join("t.tlk"), dl.join("t.tlk")]).is_err());

        let mut info = Struct::new(0);
        let old = hak_list(&info, &["b".into(), "z".into()]);
        info.set("Mod_HakList", old);
        let Value::List(list) = hak_list(&info, &["a".into(), "b".into()]) else { panic!() };
        let names: Vec<String> = list
            .iter()
            .map(|s| {
                String::from_utf8_lossy(s.read(&ifo::mod_hak_list::MOD_HAK).as_bytes()).into_owned()
            })
            .collect();
        assert_eq!(names, ["a", "b", "z"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
