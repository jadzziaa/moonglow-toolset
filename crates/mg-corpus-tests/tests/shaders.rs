//! The game's own shaders compiled as Moonglow will draw with them
//! (`docs/shaders-proposal.md`, M0): every stock program, and every program
//! in the installed haks, to a validated naga module. Each failure is named
//! with its file, line and glslang's message.
//!
//! A folder's haks that hold shaders are layered together over the game, as
//! a module that lists them would have them: The Domains of Dread's
//! `tdod_shaders` replaces the stock `inc_lighting`, which its other haks'
//! shaders include. Where a hak replaces a stock file, the stock programs
//! are compiled again with it. Haks beyond the user's `hak/` and the
//! game's: `MOONGLOW_HAKS` (the development machine's baseline, 2026-10-21:
//! 65 stock programs, and 37 in `~/Projects/tdod-module/build/hak`).

use std::collections::BTreeSet;
use std::path::Path;

use mg_core::ResType;
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_shader::{Part, Preamble, Stage, compile, program_stage};
use mg_testkit::{at_least, builder_hak_dirs, corpus, files_with_ext, sweep};

/// Every 3rd program on an everyday run.
const ONE_IN: usize = 3;

/// The programs among `keys` (`.shd` that define `main`), by name.
fn programs(rm: &ResMan, keys: impl IntoIterator<Item = ResKey>) -> Vec<(String, Stage)> {
    let mut out: Vec<(String, Stage)> = keys
        .into_iter()
        .filter(|k| k.restype == ResType::SHD)
        .filter_map(|k| {
            let name = k.resref.to_lowercase().to_string();
            let data = rm.get(&k).ok()?;
            Some((name.clone(), program_stage(&name, &data)?))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Compiles each program and splits its uniforms as a renderer does (M3:
/// the draw's, the frame's, the bones'; here by a rough rule of names, as
/// what is split is the program, not the rule); the failures, one line
/// each. (And how many read the engine's lights by their places in the
/// frame's, `mg_shader::LIGHT_INDEX`, in `INDEXED`; how many read the
/// lights' arrays as they are, in `UNINDEXED`.)
fn compile_all(rm: &ResMan, list: &[(String, Stage)]) -> Vec<String> {
    let pre = Preamble::default();
    let part = |m: &mg_shader::Member| match m.name.as_str() {
        "m_bones" => Part::Bones,
        n if n.starts_with("m_") || n.starts_with("material") || n.starts_with("light") => {
            Part::Draw
        }
        _ => Part::Frame,
    };
    list.iter()
        .filter_map(|(name, stage)| {
            let split = compile(name, *stage, &pre, rm).and_then(|mut p| {
                let had = !p.reflection.uniforms.is_empty();
                let direct = mg_shader::LIGHT_ARRAYS.iter().any(|a| p.reads_uniform(a));
                let split = p.split_uniforms(part)?;
                if p.reflection.uniform(mg_shader::LIGHT_INDEX).is_some() {
                    INDEXED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                if direct {
                    eprintln!("{name}: its lights read as arrays, not by index");
                    UNINDEXED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(if had && !split { Some("its uniforms not split".to_string()) } else { None })
            });
            match split {
                Ok(None) => None,
                Ok(Some(why)) => Some(format!("{name} ({stage}): {why}")),
                Err(e) => {
                    let d: Vec<String> = e.diagnostics().iter().map(ToString::to_string).collect();
                    Some(format!("{name} ({stage}): {}", d.join("; ")))
                }
            }
        })
        .collect()
}

/// The programs [`compile_all`] found reading the lights by index.
static INDEXED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static UNINDEXED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The game alone: no user directory (its `override` would win).
fn game(root: &Path) -> ResMan {
    ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap()
}

/// The stock programs: those of the game's keys.
fn stock(rm: &ResMan) -> Vec<(String, Stage)> {
    let keys = rm
        .layers()
        .iter()
        .filter(|l| l.class == LayerClass::Key)
        .flat_map(|l| l.container.keys())
        .collect::<Vec<_>>();
    programs(rm, keys)
}

#[test]
fn every_stock_program_compiles() {
    let root = corpus!();
    let rm = game(&root);
    let all = stock(&rm);
    assert!(all.len() >= 60, "{} stock programs", all.len());
    // (`fslit` and its kind, which the area views draw most: their lights
    // read by index, in a sample too.)
    let mut list = sweep(all, ONE_IN);
    if !list.iter().any(|(n, _)| n == "fslit") {
        list.push(("fslit".into(), Stage::Fragment));
    }
    use std::sync::atomic::Ordering::Relaxed;
    let (before, before_not) = (INDEXED.load(Relaxed), UNINDEXED.load(Relaxed));
    let failed = compile_all(&rm, &list);
    let indexed = INDEXED.load(Relaxed) - before;
    // (A program the pass passes over draws a kilobyte of lights a draw:
    // none of the stock ones.)
    assert_eq!(
        UNINDEXED.load(Relaxed) - before_not,
        0,
        "stock programs read their lights as arrays"
    );
    assert!(failed.is_empty(), "{} of {} failed:\n{}", failed.len(), list.len(), failed.join("\n"));
    assert!(list.len() >= at_least(65, ONE_IN), "{} compiled", list.len());
    assert!(indexed >= 1, "no stock program reads its lights by index");
    eprintln!("{} stock programs compiled, {indexed} reading the lights by index", list.len());
}

#[test]
fn every_program_in_the_installed_haks_compiles() {
    let root = corpus!();
    let user = GameInstall::detect().and_then(|g| g.user_dir);
    let gi = GameInstall::new(&root, user, "en");
    let mut dirs = gi.hak_dirs();
    dirs.extend(builder_hak_dirs());
    let (mut compiled, mut failed) = (0, Vec::new());
    for dir in dirs {
        let mut rm = game(&root);
        let stock_names: BTreeSet<ResKey> = rm
            .entries()
            .into_iter()
            .map(|(k, _)| k)
            .filter(|k| k.restype == ResType::SHD)
            .collect();
        let mut mine = Vec::new();
        for path in files_with_ext(&dir, "hak") {
            let Ok(c) = ErfContainer::open(&path) else { continue };
            let shd: Vec<ResKey> =
                mg_resman::Container::keys(&c).filter(|k| k.restype == ResType::SHD).collect();
            if shd.is_empty() {
                continue;
            }
            let name = path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
            rm.add(priority::HAK_USER, format!("hak:{name}"), LayerClass::Erf, c);
            mine.extend(shd);
        }
        if mine.is_empty() {
            continue;
        }
        let mut list = programs(&rm, mine.iter().copied());
        // Stock files the haks replace: the stock programs again, with them.
        let replaced: Vec<&ResKey> = mine.iter().filter(|k| stock_names.contains(k)).collect();
        if !replaced.is_empty() {
            eprintln!("{}: {} replace stock shaders", dir.display(), replaced.len());
            list.extend(stock(&rm));
            list.sort();
            list.dedup();
        }
        let list = sweep(list, ONE_IN);
        let (indexed, unindexed) = (
            INDEXED.load(std::sync::atomic::Ordering::Relaxed),
            UNINDEXED.load(std::sync::atomic::Ordering::Relaxed),
        );
        let f = compile_all(&rm, &list);
        eprintln!(
            "{}: {} programs, {} failed, {} reading the lights by index, {} as arrays",
            dir.display(),
            list.len(),
            f.len(),
            INDEXED.load(std::sync::atomic::Ordering::Relaxed) - indexed,
            UNINDEXED.load(std::sync::atomic::Ordering::Relaxed) - unindexed,
        );
        compiled += list.len() - f.len();
        failed.extend(f.into_iter().map(|f| format!("{}: {f}", dir.display())));
    }
    assert!(failed.is_empty(), "{} failed:\n{}", failed.len(), failed.join("\n"));
    eprintln!("{compiled} programs in the haks compiled");
}
