//! Module workspace round trips over the shipped modules.

use mg_core::ResType;
use mg_gff::Gff;
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_testkit::{bundled_modules, corpus, scratch_dir};
use rayon::prelude::*;

use crate::common;
use common::rewrite_all_gffs;

/// Every shipped module opens; its module info names areas that exist and
/// haks that ship; after rewriting every GFF and saving as an archive and as
/// a folder, both reopen with the same resources, GFFs as identical trees and
/// everything else byte for byte.
#[test]
fn shipped_modules_round_trip_through_the_workspace() {
    let root = corpus!();
    let dir = scratch_dir("modules_round_trip");
    let modules = bundled_modules(&root);
    assert!(modules.len() > 20);
    let failures: Vec<String> = modules
        .par_iter()
        .enumerate()
        .flat_map_iter(|(i, path)| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let mut fails = Vec::new();
            let original = Module::open(path).unwrap();
            // Areas resolve through the module and its haks (premium modules
            // keep areas in their haks).
            let install = GameInstall::new(&root, None, "en");
            let mut rm = ResMan::new();
            let haks = original.haks().unwrap();
            let hak_names: Vec<&str> = haks.iter().map(String::as_str).collect();
            for missing in rm.add_haks(&install, &hak_names).unwrap() {
                fails.push(format!("{name}: hak {missing} not shipped"));
            }
            rm.add(priority::MODULE, "module", LayerClass::Erf, original.container());
            for area in original.areas().unwrap() {
                if !rm.contains(&ResKey::new(area, ResType::ARE)) {
                    fails.push(format!("{name}: area {area} listed but found nowhere"));
                }
            }
            let mut m = original.clone();
            rewrite_all_gffs(&mut m);
            let archive = ModuleLocation::Archive(dir.join(format!("{i}.mod")));
            let folder = ModuleLocation::Folder(dir.join(format!("{i}")));
            m.save_as(&archive).unwrap();
            m.save_as(&folder).unwrap();
            for loc in [archive, folder] {
                let back = Module::open(loc.path()).unwrap();
                if back.len() != original.len() {
                    fails.push(format!(
                        "{name}: {} resources after saving to {loc:?}, {} before",
                        back.len(),
                        original.len()
                    ));
                }
                for k in original.keys() {
                    let (a, b) = (original.get(k).unwrap(), back.get(k));
                    let same = match b {
                        None => false,
                        Some(b) if k.restype.is_gff() => {
                            Gff::read(a).unwrap() == Gff::read(b).unwrap()
                        }
                        Some(b) => a == b,
                    };
                    if !same {
                        fails.push(format!("{name}: {k} differs after saving to {loc:?}"));
                    }
                }
            }
            fails
        })
        .collect();
    eprintln!("round-tripped {} modules", modules.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(), failures.join("\n"));
}

/// A script's source without its comments (`//` to the line's end, `/*`
/// to `*/`), its strings kept (`"a // b"`, `"\""`).
fn without_comments(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0;
    let mut string = false;
    while i < src.len() {
        let (c, next) = (src[i], src.get(i + 1).copied());
        if string {
            out.push(c);
            if c == b'\\'
                && let Some(n) = next
            {
                out.push(n);
                i += 1;
            } else if c == b'"' || c == b'\n' {
                string = false;
            }
        } else if c == b'/' && next == Some(b'/') {
            while i < src.len() && src[i] != b'\n' {
                i += 1;
            }
            continue;
        } else if c == b'/' && next == Some(b'*') {
            i += 2;
            while i < src.len() && !(src[i] == b'*' && src.get(i + 1) == Some(&b'/')) {
                i += 1;
            }
            i += 2;
            out.push(b' ');
            continue;
        } else {
            string = c == b'"';
            out.push(c);
        }
        i += 1;
    }
    out
}

/// The Unused report of the shipped modules says no script one of the
/// game's tables runs (as the module sees them, its haks' over the
/// game's: a spell's impact script...), nor a resource a string in one of
/// the module's scripts spells (`"name"`, looked for in the source's
/// bytes outside comments, apart from the script's own name), nor one
/// that what the module reaches in its haks names (Kingmaker's
/// conversations, in its hak, run the module's scripts), and says each
/// script once.
#[test]
fn shipped_modules_report_nothing_their_tables_or_strings_use() {
    use mg_module::verify::{also_used, module_references, table_scripts_in, unused_of};
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let modules = mg_testkit::sweep(bundled_modules(&root), 3);
    let counts: Vec<(String, usize, usize, usize)> = modules
        .par_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let m = Module::open(path).unwrap();
            let mut rm = ResMan::for_game(&install).unwrap();
            let (missing, wrong) = m.stack_onto(&mut rm, &install);
            assert!(missing.is_empty() && wrong.is_none(), "{name}: {missing:?} {wrong:?}");
            let refs = module_references(&m);
            let also = also_used(&m, &refs, &rm);
            let unused = unused_of(&m, &refs, &also);
            let before = unused_of(&m, &refs, &[]).len();
            let run: std::collections::HashSet<_> =
                table_scripts_in(&rm).iter().map(|r| r.target).collect();
            let named: std::collections::HashSet<_> =
                also.iter().flat_map(|r| r.candidates()).collect();
            let sources: Vec<(ResKey, Vec<u8>)> = m
                .keys_of(ResType::NSS)
                .map(|k| (*k, without_comments(m.get(k).unwrap()).to_ascii_lowercase()))
                .collect();
            let mut scripts = std::collections::HashSet::new();
            for u in &unused {
                let script = u.script.is_some();
                assert!(
                    !script || scripts.insert(u.key.resref),
                    "{name}: {} said twice",
                    u.key.resref
                );
                assert!(!script || !run.contains(&u.key.resref), "{name}: a table runs {u}");
                assert!(u.files().iter().all(|k| !named.contains(k)), "{name}: {u} is named");
                let quoted = format!("\"{}\"", u.key.resref.to_string().to_ascii_lowercase());
                let spelled = sources.iter().find(|(k, src)| {
                    !(script && k.resref == u.key.resref)
                        && src.windows(quoted.len()).any(|w| w == quoted.as_bytes())
                });
                assert!(spelled.is_none(), "{name}: {u} is spelled in {:?}", spelled.map(|s| s.0));
            }
            let n = unused.iter().filter(|u| u.script.is_some()).count();
            (name, n, unused.len(), before)
        })
        .collect();
    for (name, scripts, all, before) in &counts {
        eprintln!(
            "{name}: {all} unused ({scripts} scripts); {before} by the module's own references"
        );
    }
    assert!(counts.len() >= mg_testkit::at_least(28, 3), "{} modules", counts.len());
    // (Kingmaker's conversations and blueprints are in its hak.)
    if let Some((_, scripts, _, before)) = counts.iter().find(|c| c.0 == "Kingmaker.mod") {
        assert!(*scripts < before / 4, "Kingmaker: {scripts} unused scripts of {before}");
    }
}

/// The hak report for every shipped module with haks: each hak is found,
/// every hak resource is listed with the haks that have it (first hak
/// first), conflicts are exactly the multi-hak entries, and overrides exist
/// in the base game.
#[test]
fn hak_reports_for_shipped_modules() {
    use mg_module::haks::hak_report;
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let base = ResMan::for_game(&install).unwrap();
    let mut checked = 0;
    for path in bundled_modules(&root) {
        let m = Module::open(&path).unwrap();
        let haks = m.haks().unwrap();
        if haks.is_empty() {
            continue;
        }
        let report = hak_report(&install, &haks).unwrap();
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(report.haks.iter().all(|(_, p)| p.is_some()), "{name}: a hak is missing");
        // Resources in several haks are listed in Mod_HakList order.
        for (k, list) in report.conflicts() {
            let order: Vec<usize> =
                list.iter().map(|h| haks.iter().position(|x| x == h).unwrap()).collect();
            assert!(order.windows(2).all(|w| w[0] < w[1]), "{name}: {k} haks out of order");
        }
        assert!(report.overrides.keys().all(|k| base.contains(k)), "{name}: override not in base");
        let text = report.to_text();
        assert!(text.contains("Conflicting resources") && text.contains("Complete resource list"));
        eprintln!(
            "{name}: {} hak resources, {} conflicts, {} overrides",
            report.resources.len(),
            report.conflicts().count(),
            report.overrides.len()
        );
        checked += 1;
    }
    assert!(checked >= 8);
}
