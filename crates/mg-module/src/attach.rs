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
    pub from: PathBuf,
    pub to: PathBuf,
    pub there: There,
    /// The name the module knows it by (the file's, without its extension).
    pub name: String,
    pub is_tlk: bool,
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
/// `hak`, `.tlk` files to `tlk`. An error names a file that's neither, or
/// a second talk table (a module has one).
pub fn placements(user_dir: &Path, files: &[PathBuf]) -> Result<Vec<Placement>, String> {
    let mut out: Vec<Placement> = Vec::new();
    for from in files {
        let name = from.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let ext = from.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
        let is_tlk = match ext.as_deref() {
            Some("hak") => false,
            Some("tlk") => true,
            _ => return Err(format!("{}: not a hak or a talk table", from.display())),
        };
        if is_tlk && name.len() > 16 {
            return Err(format!("{name}.tlk: a talk table's name has at most 16 characters"));
        }
        if is_tlk && out.iter().any(|p| p.is_tlk) {
            return Err("a module has one talk table: choose one".into());
        }
        let to = user_dir.join(if is_tlk { "tlk" } else { "hak" }).join(from.file_name().unwrap());
        let same_file = std::fs::canonicalize(from)
            .ok()
            .is_some_and(|f| std::fs::canonicalize(&to).ok().is_some_and(|t| t == f));
        let there = if same_file || (to.is_file() && same_bytes(from, &to)) {
            There::Same
        } else if to.exists() {
            There::Different
        } else {
            There::Nothing
        };
        out.push(Placement { from: from.clone(), to, there, name, is_tlk });
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
        std::fs::copy(&p.from, &part)
            .and_then(|_| std::fs::rename(&part, &p.to))
            .map_err(|e| format!("{}: {e}", p.to.display()))?;
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
