//! Attaching content to a module in one step (the job of the PRC installer
//! and NWN Installer Tools): haks and a talk table, wherever they were
//! downloaded to, are copied into the user's `hak` and `tlk` folders, where
//! the game looks for them, and the module lists the haks (at the top, in
//! the order given) and names the talk table.

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
    /// The file, or the zip archive it is in.
    pub from: PathBuf,
    /// Its name in the zip archive `from` (content downloaded as an
    /// archive is taken as it is, not unpacked first).
    pub entry: Option<String>,
    pub to: PathBuf,
    pub there: There,
    /// The name the module knows it by (the file's, without its extension).
    pub name: String,
    pub is_tlk: bool,
}

/// The haks and talk tables in the zip archive at `path`: each entry's
/// name in it and its file's name (without the folders it is in). Those
/// whose names would lead out of a folder are left out; an archive with
/// neither is an error.
fn zip_entries(path: &Path) -> Result<Vec<(String, String)>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let zip = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| format!("{}: not a zip archive that can be read ({e})", path.display()))?;
    let mut out = Vec::new();
    for name in zip.file_names() {
        // (A name that climbs out of its folder, or names one: none of
        // ours.)
        if name.ends_with('/') || name.split(['/', '\\']).any(|part| part == "..") {
            continue;
        }
        let Some(file) = name.rsplit(['/', '\\']).next().filter(|f| !f.is_empty()) else {
            continue;
        };
        let ext = Path::new(file).extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
        if matches!(ext.as_deref(), Some("hak" | "tlk")) {
            out.push((name.to_string(), file.to_string()));
        }
    }
    out.sort();
    if out.is_empty() {
        return Err(format!("{}: has no hak or talk table in it", path.display()));
    }
    Ok(out)
}

/// The bytes of entry `entry` of the zip archive at `path`, streamed into
/// `to`.
fn unzip(path: &Path, entry: &str, to: &mut dyn std::io::Write) -> Result<u64, String> {
    let at = |e: &dyn std::fmt::Display| format!("{} ({entry}): {e}", path.display());
    let file = std::fs::File::open(path).map_err(|e| at(&e))?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(|e| at(&e))?;
    let mut f = zip.by_name(entry).map_err(|e| at(&e))?;
    std::io::copy(&mut f, to).map_err(|e| at(&e))
}

/// Whether entry `entry` of the zip archive `zip` has the bytes of file
/// `b`.
fn same_entry(zip: &Path, entry: &str, b: &Path) -> bool {
    let Ok(theirs) = std::fs::read(b) else { return false };
    let mut ours = Vec::with_capacity(theirs.len());
    unzip(zip, entry, &mut ours).is_ok() && ours == theirs
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

/// Where each file goes in the user folder `user_dir`: `.hak` files to
/// `hak`, `.tlk` files to `tlk`; a `.zip` archive's haks and talk table
/// alike, from the archive (wherever in it they are). An error names a
/// file that's none of these, an archive with neither in it, or a second
/// talk table (a module has one).
pub fn placements(user_dir: &Path, files: &[PathBuf]) -> Result<Vec<Placement>, String> {
    let mut out: Vec<Placement> = Vec::new();
    for from in files {
        let ext = from.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
        let found = match ext.as_deref() {
            Some("zip") => zip_entries(from)?.into_iter().map(|(e, f)| (Some(e), f)).collect(),
            Some("hak" | "tlk") => {
                vec![(None, from.file_name().unwrap_or_default().to_string_lossy().into_owned())]
            }
            _ => {
                return Err(format!("{}: not a hak, a talk table or a zip of them", from.display()));
            }
        };
        for (entry, file) in found {
            let file_path = Path::new(&file);
            let name = file_path.file_stem().map(|n| n.to_string_lossy().into_owned());
            let name = name.unwrap_or_default();
            let ext = file_path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
            let is_tlk = ext.as_deref() == Some("tlk");
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
            let same = to.is_file()
                && match &entry {
                    None => same_bytes(from, &to),
                    Some(e) => same_entry(from, e, &to),
                };
            let there = if same_file || same {
                There::Same
            } else if to.exists() {
                There::Different
            } else {
                There::Nothing
            };
            out.push(Placement { from: from.clone(), entry, to, there, name, is_tlk });
        }
    }
    Ok(out)
}

/// Copies the files that need copying (those that differ only with
/// `replace`); returns how many were copied. Each is written beside its
/// place and moved there, so a failed copy leaves nothing half-written.
pub fn copy(placements: &[Placement], replace: bool) -> Result<usize, String> {
    let mut n = 0;
    for p in placements {
        let go = match p.there {
            There::Nothing => true,
            There::Different => replace,
            There::Same => false,
        };
        if !go {
            continue;
        }
        let dir = p.to.parent().expect("in a folder");
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let part = p.to.with_extension("part");
        let written = match &p.entry {
            None => std::fs::copy(&p.from, &part).map_err(|e| format!("{}: {e}", p.to.display())),
            Some(entry) => std::fs::File::create(&part)
                .map_err(|e| format!("{}: {e}", part.display()))
                .and_then(|mut f| unzip(&p.from, entry, &mut f)),
        };
        if let Err(e) = written {
            let _ = std::fs::remove_file(&part);
            return Err(e);
        }
        std::fs::rename(&part, &p.to).map_err(|e| format!("{}: {e}", p.to.display()))?;
        n += 1;
    }
    Ok(n)
}

/// The module's hak list with `haks` at the top, in order (one already
/// listed moves up).
pub fn hak_list(info: &Struct, haks: &[String]) -> Value {
    let name = |s: &Struct| {
        String::from_utf8_lossy(s.read(&ifo::mod_hak_list::MOD_HAK).as_bytes()).into_owned()
    };
    let mut items: Vec<Struct> = haks
        .iter()
        .map(|h| {
            let mut item = ifo::MOD_HAK_LIST.new_item();
            item.write(&ifo::mod_hak_list::MOD_HAK, ExoString::from(h.as_str()));
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
