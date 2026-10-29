//! The round trip (`mg roundtrip`, Build › Round-Trip Check): a module
//! saved as Moonglow saves it and read back, and each GFF and 2DA of it
//! written anew as an edit writes it, compared with what it was. The
//! module itself is not touched.
//!
//! A saved copy is compared byte for byte; a GFF written anew by its
//! fields, not its bytes (Moonglow lays a GFF out its own way, as every
//! edit does), and written twice it must come out the same; a 2DA by its
//! cells. The other formats (scripts, models...) are kept as their bytes,
//! and the saved copy holds them to that.

use std::path::Path;

use mg_core::{Codepage, ResType};
use mg_gff::Gff;

use crate::{Module, ModuleLocation};

/// What the round trip found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The module's resources, saved and read back.
    pub resources: usize,
    /// The GFFs and 2DAs written anew.
    pub gffs: usize,
    pub tables: usize,
    /// A resource (`name.ext`) and how it differs.
    pub differences: Vec<(String, String)>,
    /// A resource that can't be read as its type says (kept as its bytes,
    /// never written anew), and why.
    pub unread: Vec<(String, String)>,
}

impl Report {
    /// One line: how much was compared, and whether it all came back.
    pub fn summary(&self) -> String {
        let some = |n: usize, one: &str| match n {
            1 => format!("1 {one}"),
            n => format!("{n} {one}s"),
        };
        format!(
            "{} saved and read back, {} and {} written anew: {}",
            some(self.resources, "resource"),
            some(self.gffs, "GFF"),
            some(self.tables, "2DA"),
            match self.differences.len() {
                0 => "all the same".to_string(),
                n => format!("{n} differ"),
            }
        )
    }
}

/// The round trip of `original`, its copy saved at `copy` (a `.mod`, taken
/// away after unless `keep`). `progress` is told how many resources of
/// how many are written anew, and stops it where it says so (`false`).
pub fn check(
    original: &Module,
    copy: &Path,
    keep: bool,
    progress: &dyn Fn(usize, usize) -> bool,
) -> Result<Report, String> {
    let mut report = Report { resources: original.len(), ..Report::default() };
    let mut differs = |key: &mg_resman::ResKey, what: &str| {
        report.differences.push((key.to_string(), what.into()));
    };

    // Saved as it is: every resource byte for byte.
    let saved = original.clone().save_as(&ModuleLocation::Archive(copy.to_path_buf()));
    let back = saved.and_then(|()| Module::open(copy));
    if !keep {
        let _ = std::fs::remove_file(copy);
    }
    let back = back.map_err(|e| format!("the copy could not be saved and read back: {e}"))?;
    for k in original.keys() {
        match back.get(k) {
            None => differs(k, "missing from the saved copy"),
            Some(b) if Some(b) != original.get(k) => differs(k, "other bytes in the saved copy"),
            Some(_) => {}
        }
    }
    for k in back.keys().filter(|k| !original.contains(k)) {
        differs(k, "in the saved copy alone");
    }

    // Written anew, as an edit writes it: the same when read back.
    let cp = Codepage::WINDOWS_1252;
    let (mut gffs, mut tables, mut unread) = (0, 0, Vec::new());
    let total = original.len();
    for (n, k) in original.keys().enumerate() {
        if n % 64 == 0 && !progress(n, total) {
            return Err("stopped".into());
        }
        let data = original.get(k).expect("its own key");
        if k.restype.is_gff() {
            let read = match Gff::read(data) {
                Ok(g) => g,
                Err(e) => {
                    unread.push((k.to_string(), e.to_string()));
                    continue;
                }
            };
            gffs += 1;
            match read.to_bytes() {
                Err(e) => differs(k, &format!("cannot be written: {e}")),
                Ok(written) => match Gff::read(&written) {
                    Err(e) => differs(k, &format!("written, it cannot be read: {e}")),
                    Ok(again) if again != read => differs(k, "other fields once written"),
                    Ok(again) if again.to_bytes().ok().as_ref() != Some(&written) => {
                        differs(k, "other bytes each time it is written")
                    }
                    Ok(_) => {}
                },
            }
        } else if k.restype == ResType::TWODA {
            let read = match mg_2da::TwoDa::parse(data, cp) {
                Ok(t) => t,
                Err(e) => {
                    unread.push((k.to_string(), e.to_string()));
                    continue;
                }
            };
            tables += 1;
            match read.to_bytes(cp) {
                Err(e) => differs(k, &format!("cannot be written: {e}")),
                Ok(written) => match mg_2da::TwoDa::parse(&written, cp) {
                    Err(e) => differs(k, &format!("written, it cannot be read: {e}")),
                    Ok(again) if again != read => differs(k, "other cells once written"),
                    Ok(_) => {}
                },
            }
        }
    }
    report.gffs = gffs;
    report.tables = tables;
    report.unread = unread;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_gff::Value;
    use mg_resman::ResKey;

    #[test]
    fn a_module_comes_back_and_a_bad_file_is_said() {
        let dir = std::env::temp_dir().join(format!("mg-roundtrip-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let key = |name: &str| ResKey::from_filename(name).unwrap();
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        let mut utc = Gff::new(*b"UTC ");
        utc.root.set("Tag", Value::String(b"guard".to_vec()));
        m.set_gff(key("guard.utc"), &utc).unwrap();
        m.set(key("things.2da"), &b"2DA V2.0\n\n   Label\n0  one\n"[..]);
        m.set(key("broken.utp"), &b"not a gff"[..]);
        let copy = dir.join("copy.mod");
        let report = check(&m, &copy, false, &|_, _| true).unwrap();
        assert_eq!((report.resources, report.gffs, report.tables), (4, 2, 1));
        assert!(report.differences.is_empty(), "{report:?}");
        assert_eq!(report.unread[0].0, "broken.utp");
        assert!(!copy.exists(), "the copy taken away");
        assert_eq!(
            report.summary(),
            "4 resources saved and read back, 2 GFFs and 1 2DA written anew: all the same"
        );
        // Stopped when asked.
        assert_eq!(check(&m, &copy, false, &|_, _| false), Err("stopped".into()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
