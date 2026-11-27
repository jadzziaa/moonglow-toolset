//! A server's user directory made ready for a module (`mg serve`, `mg
//! test`, `mg watch --serve`, Test Module's local server): the module as
//! an archive in `modules`, its haks in `hak` and its talk table in
//! `tlk` (from the player's folders and the game's, which are only read,
//! or from the module's NWSync content), and the player's `development`
//! folder linked into it.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use mg_core::ResType;
use mg_resman::{GameInstall, ResKey};

use crate::ModuleLocation;

/// What was compiled of a module's scripts, as `mg compile` and the
/// window remember it.
fn remembered(module: &Path, made_with: u64) -> crate::build::Remembered {
    let dir = crate::build::compiled_dir();
    crate::build::Remembered::kept(dir.as_deref(), Some(module), made_with)
}

fn same(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The server's name for a module: what `-module` takes, and the file in
/// `modules`. (Lower case, the letters a resource name may have.)
pub fn module_name(module: &Path) -> String {
    let is_nasher = module.file_name().is_some_and(|n| n == "nasher.cfg");
    let dir_or_file = if is_nasher {
        module.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."))
    } else {
        module
    };
    let resolved = if dir_or_file.file_name().is_none() {
        dir_or_file.canonicalize().ok().or_else(|| {
            let cur = std::env::current_dir().ok()?;
            if dir_or_file.as_os_str().is_empty() || dir_or_file == Path::new(".") {
                Some(cur)
            } else {
                cur.join(dir_or_file).canonicalize().ok()
            }
        })
    } else {
        None
    };
    let path = resolved.as_deref().unwrap_or(dir_or_file);
    let stem = if path.is_dir() || is_nasher { path.file_name() } else { path.file_stem() };
    let name: String = stem
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    if name.is_empty() { "module".into() } else { name }
}

/// Whether two paths are one file: the same file reached by two paths,
/// or two hard links of it (which canonical paths can't tell).
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if let (Ok(a), Ok(b)) = (std::fs::metadata(a), std::fs::metadata(b)) {
            return a.dev() == b.dev() && a.ino() == b.ino();
        }
        false
    }
    #[cfg(not(unix))]
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Where a file is placed for the server: whether the folder is the
/// player's own NWN folder (`--real-user-dir`), and whether the file is
/// one of Moonglow's NWSync cache.
#[derive(Debug, Clone, Copy, Default)]
struct Placing {
    players: bool,
    from_cache: bool,
}

/// What stops a file of the player's own folder being replaced.
fn players_file(to: &Path) -> anyhow::Error {
    anyhow::anyhow!(
        "{} is a file of your own NWN folder, which Moonglow doesn't replace: move it away, or          serve from a folder of Moonglow's (leave out --dir and --real-user-dir)",
        to.display()
    )
}

/// Whether there is a file (not a link) at `to`.
fn is_regular_file(to: &Path) -> bool {
    std::fs::symlink_metadata(to).is_ok_and(|m| m.file_type().is_file())
}

/// A file beside `to` that a link or a copy is made as before it is moved
/// into place: `to` is replaced whole, never written through (a link
/// there of an earlier run's, to the player's file or the cache's).
fn part_of(to: &Path) -> PathBuf {
    let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    to.with_file_name(format!(".{name}.{}.mg-part", std::process::id()))
}

/// A way of putting a file at a path (a link, a copy).
type Way<'a> = &'a dyn Fn(&Path, &Path) -> std::io::Result<()>;

/// Puts a file where the server reads it without writing the original.
/// A file of the cache is hard linked (it stays while the server runs,
/// whatever a prune of the cache does), a file elsewhere linked, else
/// either is copied; in the player's own folder, a link only where the
/// system allows one (a hard link would be a file of theirs to the next
/// run), and an existing file there (not a link) is never replaced.
fn place(from: &Path, to: &Path, how: Placing) -> Result<()> {
    if same_file(from, to) {
        return Ok(());
    }
    if how.players && is_regular_file(to) {
        return Err(players_file(to));
    }
    let part = part_of(to);
    let _ = std::fs::remove_file(&part);
    let symlink = |from: &Path, to: &Path| -> std::io::Result<()> {
        #[cfg(unix)]
        return std::os::unix::fs::symlink(from, to);
        #[cfg(not(unix))]
        {
            let _ = (from, to);
            Err(std::io::Error::other("no links"))
        }
    };
    let hard = |from: &Path, to: &Path| std::fs::hard_link(from, to);
    let copy = |from: &Path, to: &Path| std::fs::copy(from, to).map(|_| ());
    let ways: [Way; 3] = match how {
        Placing { players: true, .. } => [&symlink, &copy, &copy],
        Placing { from_cache: true, .. } => [&hard, &symlink, &copy],
        _ => [&symlink, &hard, &copy],
    };
    let mut last = None;
    for way in ways {
        match way(from, &part) {
            Ok(()) => {
                last = None;
                break;
            }
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                last = Some(e);
            }
        }
    }
    if let Some(e) = last {
        return Err(anyhow::Error::new(e).context(to.display().to_string()));
    }
    std::fs::rename(&part, to).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        anyhow::Error::new(e).context(to.display().to_string())
    })
}

/// Writes `bytes` as the file `to` for the server, through a file beside
/// it moved into place (never through a link at `to`); in the player's
/// own folder (`players`), an existing file there is never replaced.
fn place_bytes(bytes: &[u8], to: &Path, players: bool) -> Result<()> {
    if std::fs::read(to).is_ok_and(|had| had == bytes) {
        return Ok(());
    }
    if players && is_regular_file(to) {
        return Err(players_file(to));
    }
    let part = part_of(to);
    std::fs::write(&part, bytes).and_then(|()| std::fs::rename(&part, to)).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        anyhow::Error::new(e).context(to.display().to_string())
    })
}

/// Whether `dir` is the player's own NWN folder (`gi`'s user folder, or
/// where the game keeps it).
pub fn is_players(gi: &GameInstall, dir: &Path) -> bool {
    players_dir(gi.user_dir.as_deref(), dir).is_some()
}

/// The player's own NWN folder `dir` is, where it is one: `user` (the
/// user folder `mg` was given or found) or where the game keeps it.
pub fn players_dir(user: Option<&Path>, dir: &Path) -> Option<PathBuf> {
    let dir = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    (user.map(Path::to_path_buf).into_iter())
        .chain(mg_resman::default_user_dir())
        .find(|p| same(p, &dir))
}

/// What the server's `development` folder is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Development {
    /// A link to the player's own, as Test Module's game has it.
    #[default]
    Players,
    /// The folder `mg watch --serve` writes into: the server's own.
    Watched,
    /// None of the player's (`--no-development`): a link an earlier run
    /// left is taken away.
    Off,
}

impl Development {
    /// The player's, linked, unless `off` (`--no-development`).
    pub fn players_unless(off: bool) -> Development {
        if off { Development::Off } else { Development::Players }
    }
}

/// Gives the server's folder `dir` the player's `development` folder
/// (in `user`, the user folder `mg` was given or found), as Test Module
/// gives its game: a link to it (a junction on Windows without the right
/// to make a symbolic link), made again at every start so that it
/// follows `--user-dir`. Nothing is written into the player's folder.
/// With none to give (`--no-development`, no user folder), a link an
/// earlier run made is taken away. Nothing is done where the player has
/// none, where `dir` is the
/// player's own (`--real-user-dir`), or for `mg watch`'s folder. A
/// folder (or file) of `dir`'s own there is kept, and said in the note
/// (an empty folder, as the server makes at a start without the link,
/// gives way)
/// returned; in Moonglow's own `scratch` folder a folder there was left
/// by `mg watch --serve`, and gives way.
pub fn link_development(
    user: Option<&Path>,
    dir: &Path,
    development: Development,
    scratch: bool,
) -> Result<Option<String>> {
    // (None given, `--no-development` or no user folder: a link an earlier
    // run made, into a player's folder, is taken away too.)
    let Some(user) = user.filter(|_| development == Development::Players) else {
        unlink_development(dir)?;
        return Ok(None);
    };
    if players_dir(Some(user), dir).is_some() {
        return Ok(None);
    }
    let to = dir.join("development");
    let from = user.join("development");
    let from = std::path::absolute(&from).unwrap_or(from);
    let context = || to.display().to_string();
    if crate::links::is_link(&to) {
        // (The link an earlier run made: made again, to where the player's
        // folder is now.)
        crate::links::unlink(&to).with_context(context)?;
    } else if let Ok(meta) = std::fs::symlink_metadata(&to) {
        let empty = meta.is_dir() && std::fs::read_dir(&to).is_ok_and(|mut d| d.next().is_none());
        if empty {
            // (The server's own, made at a start without the link: nothing
            // of anybody's in it.)
            std::fs::remove_dir(&to).with_context(context)?;
        } else if scratch && meta.is_dir() {
            std::fs::remove_dir_all(&to).with_context(context)?;
        } else {
            return Ok(from.is_dir().then(|| {
                format!(
                    "the server's folder has a development folder of its own ({}): kept, so \
                     yours ({}) is not given to the server",
                    to.display(),
                    from.display()
                )
            }));
        }
    }
    if from.is_dir() {
        crate::links::link_dir(&from, &to).with_context(context)?;
    }
    Ok(None)
}

/// Takes away a link to the player's `development` folder that `mg
/// serve` left in the server's folder `dir`, before `mg watch --serve`
/// makes the folder it writes into there: never written through.
pub fn unlink_development(dir: &Path) -> Result<()> {
    let to = dir.join("development");
    if crate::links::is_link(&to) {
        crate::links::unlink(&to).with_context(|| to.display().to_string())?;
    }
    Ok(())
}

/// Saves a module's copy for a server as an archive at `to`, keeping no
/// backup of the copy it takes the place of. (A `.bak` there that this
/// save did not make is somebody's, and stays.)
pub fn save_copy(m: &mut crate::Module, to: &Path) -> Result<()> {
    let replaced = to.is_file();
    m.save_as(&ModuleLocation::Archive(to.to_path_buf()))
        .with_context(|| to.display().to_string())?;
    if replaced {
        let _ = std::fs::remove_file(to.with_extension("mod.bak"));
    }
    Ok(())
}

/// The user directory made ready: the module as an archive in `modules`,
/// its haks in `hak` and its talk table in `tlk` (from the player's
/// folders and the game's, which are only read). Returns the module's
/// name there and what to say.
///
/// A module whose project names an NWSync server (`[nwsync] source`;
/// not with `no_nwsync`) has the haks that server hands out written from
/// its content into Moonglow's cache
/// ([`crate::nwsync_content::write_haks`]: once, and again only for a
/// hak a newer manifest changed) and placed here, over any of the same
/// name on disk, as the game takes them; and its talk table (and its
/// feminine table), where the manifest has it. By Moonglow's rule a hak
/// or talk table on disk newer than the manifest is taken from disk
/// instead, and said ([`crate::nwsync_content`]). `progress` is told how
/// far a write is.
pub fn prepare(
    gi: &GameInstall,
    module: &Path,
    dir: &Path,
    project: Option<&Path>,
    progress: &(dyn Fn(mg_nwsync::haks::Progress) + Sync),
    no_nwsync: bool,
) -> Result<(String, Vec<String>)> {
    let mut notes = Vec::new();
    let name = module_name(module);
    // (`--real-user-dir`: none of the player's own files is replaced.)
    let players = is_players(gi, dir);
    for sub in ["modules", "hak", "tlk"] {
        std::fs::create_dir_all(dir.join(sub))
            .with_context(|| dir.join(sub).display().to_string())?;
    }
    let target = dir.join("modules").join(format!("{name}.mod"));
    let mut m = crate::Module::open(module).with_context(|| module.display().to_string())?;
    if let Some(folder) = crate::folder::beside_mod(module) {
        notes.push(format!(
            "note: {} has its module folder beside it ({}): this is the .mod, as the game's \
             server reads it, and the folder is left as it is (name the folder to work on it)",
            module.display(),
            folder.display()
        ));
    }
    if no_nwsync {
        m.project_file.nwsync_source = None;
    }
    let (haks, tlk) = (m.haks()?, m.custom_tlk()?);
    // (Read before the copy is saved: the copy is a module of its own.)
    let nwsync = m.project_file.nwsync_settings();
    // A talk table the project keeps as JSON, made into the table as
    // nasher packs it (the game's language's, where it keeps one a
    // language): the server finds no other.
    let language = mg_core::Language::from_short_code(&gi.language);
    // (Or the project's an archive was built from, `mg watch`'s.)
    let project_tlk = |table_name: &str| -> Result<_> {
        Ok(match (m.project.as_deref(), project) {
            (Some(p), _) => crate::nasher::in_language(p.talk_tables(table_name)?, language),
            (None, Some(root)) if crate::nasher::Package::is_package(root) => {
                let package = crate::nasher::Package::read(root)?;
                let settings = crate::nasher::Settings::read(root);
                let tables = crate::nasher::talk_tables(&package, &settings, table_name)?;
                crate::nasher::in_language(tables, language)
            }
            _ => None,
        })
    };
    let own_tlk = tlk.as_deref().map(&project_tlk).transpose()?.flatten();
    let own_fem_tlk = match tlk.as_deref().filter(|t| mg_resman::named_file(t, "tlk").is_some()) {
        Some(t) => project_tlk(&crate::talk::feminine(t))?,
        None => None,
    };
    if matches!(m.location, Some(ModuleLocation::Archive(_))) {
        if !same(module, &target) {
            std::fs::copy(module, &target).with_context(|| target.display().to_string())?;
        }
    } else {
        // A project or a folder: its scripts that have no compiled script,
        // or are not as they were when last compiled (by `mg compile` or
        // the window: what they remember), are compiled, as `mg build`
        // would (the server runs compiled scripts only, and a module
        // packed without them loads and does nothing). The folder or
        // project itself is not written.
        let mut rm = mg_resman::ResMan::for_game(gi)?;
        let (_, wrong) = m.stack_onto(&mut rm, gi);
        if let Some(e) = wrong {
            return Err(e.into());
        }
        let made_with = crate::build::made_with(Some(module), None, Some(&gi.root), &rm);
        let changed = crate::build::changed(&m, &remembered(module, made_with), false);
        if !changed.is_empty() {
            let results = crate::build::compile_named_scripts(&mut m, &rm, &changed, None);
            let compiled = (results.iter())
                .filter(|r| r.result.is_ok())
                .filter(|r| m.contains(&ResKey::new(r.script.resref, ResType::NCS)));
            notes.push(format!(
                "compiled {} of the module's scripts that had no compiled script or changed \
                 since they were compiled",
                compiled.count()
            ));
            for r in &results {
                if let Err(e) = &r.result {
                    notes.push(format!("warning: {} did not compile: {}", r.script, e.message));
                }
            }
        }
        // (The server loads an archive only.)
        save_copy(&mut m, &target)?;
    }
    // What is on disk, which the rule weighs against the manifest: the
    // haks as the game finds them, the project's own talk tables (else
    // the `tlk` folders').
    let own_paths = (own_tlk.iter().chain(&own_fem_tlk)).map(|t| t.path.clone()).collect();
    // (After the compile: its scripts compiled now are the module's own.)
    let module_keys: std::collections::HashSet<ResKey> = m.keys().copied().collect();
    let on_disk =
        crate::nwsync_content::OnDisk::of(gi, &haks, tlk.as_deref(), own_paths, Some(module_keys));
    let synced = match no_nwsync {
        true => None,
        false => nwsync_haks(gi, nwsync, project, &haks, tlk.as_deref(), &on_disk, progress)?,
    };
    if let Some(s) = &synced {
        notes.push(s.summary());
        notes.extend(s.notes());
        if !s.not_listed.is_empty() {
            notes.push(format!(
                "the origins file names haks the module doesn't list, not written: {}",
                s.not_listed.join(", ")
            ));
        }
    }
    let cached = Placing { players, from_cache: true };
    let elsewhere = Placing { players, from_cache: false };
    for hak in &haks {
        // (A name with a folder in it is placed nowhere: `../x` would be
        // a file outside the server's hak folder.)
        let Some(file) = mg_resman::named_file(hak, "hak") else {
            notes.push(format!("warning: {}", gi.hak_not_found(hak)));
            continue;
        };
        let to = dir.join("hak").join(file);
        if let Some(written) = synced.as_ref().and_then(|s| s.hak(hak)) {
            place(&written.path, &to, cached)?;
            continue;
        }
        match gi.find_hak(hak) {
            Some(from) => place(&from, &to, elsewhere)?,
            None if to.is_file() => {}
            None => notes.push(format!("warning: {}", gi.hak_not_found(hak))),
        }
    }
    if let Some(tlk) = tlk {
        let mut place_tlk = |table_name: &str,
                             synced: Option<&Path>,
                             own: Option<crate::nasher::TalkTable>,
                             required: bool|
         -> Result<()> {
            let Some(file) = mg_resman::named_file(table_name, "tlk") else {
                if required {
                    notes.push(format!(
                        "warning: talk table {table_name} not found: {}",
                        mg_resman::NO_FILE_NAME
                    ));
                }
                return Ok(());
            };
            let to = dir.join("tlk").join(&file);
            // (The manifest's, as the game data has it, where the one on
            // disk is not newer.)
            if let Some(path) = synced {
                return place(path, &to, cached);
            }
            if let Some(own) = own {
                let bytes = own.tlk.to_bytes_as_nwn_tlk()?;
                place_bytes(&bytes, &to, players)?;
                notes.push(format!("talk table {table_name} made from {}", own.path.display()));
                return Ok(());
            }
            match gi.tlk_dirs().iter().map(|d| d.join(&file)).find(|p| p.is_file()) {
                Some(from) => place(&from, &to, elsewhere)?,
                None if to.is_file() => {}
                None if required => {
                    notes.push(format!("warning: talk table {table_name} not found"));
                }
                None => {}
            }
            Ok(())
        };
        // (The feminine table from where the table comes from: the
        // manifest's where it has one, else none.)
        let from_manifest = synced.as_ref().and_then(|s| s.tlk.as_ref()).is_some();
        let synced_tlk = synced.as_ref().and_then(|s| s.tlk.as_ref()).map(|(_, p)| p.as_path());
        let synced_fem = synced.as_ref().and_then(|s| s.tlk_f.as_ref()).map(|(_, p)| p.as_path());
        place_tlk(&tlk, synced_tlk, own_tlk, true)?;
        if mg_resman::named_file(&tlk, "tlk").is_some() {
            let fem = crate::talk::feminine(&tlk);
            match (from_manifest, synced_fem) {
                (true, Some(path)) => place_tlk(&fem, Some(path), None, false)?,
                // (None from where the table comes: one an earlier run put
                // there goes, never a file of the player's.)
                (true, None) => {
                    if let Some(file) = mg_resman::named_file(&fem, "tlk") {
                        let to = dir.join("tlk").join(file);
                        if !(players && is_regular_file(&to)) {
                            let _ = std::fs::remove_file(&to);
                        }
                    }
                }
                (false, _) => place_tlk(&fem, None, own_fem_tlk, false)?,
            }
        }
    }
    Ok((name, notes))
}

/// The module's haks from its NWSync content, written into Moonglow's
/// cache by the rule with what is `on_disk`, where its project's
/// `settings` (or those of the project it was built from) name a server:
/// `None` where none does.
fn nwsync_haks(
    gi: &GameInstall,
    settings: Option<mg_nwsync::Settings>,
    project: Option<&Path>,
    haks: &[String],
    tlk: Option<&str>,
    on_disk: &crate::nwsync_content::OnDisk,
    progress: &(dyn Fn(mg_nwsync::haks::Progress) + Sync),
) -> Result<Option<crate::nwsync_content::HakFiles>> {
    let settings = settings.or_else(|| {
        // (A project's folder, or its nasher.cfg.)
        let root = project.map(|p| if p.is_file() { p.parent().unwrap_or(p) } else { p })?;
        crate::project_file::ProjectFile::read(root).nwsync_settings()
    });
    let Some(settings) = settings else { return Ok(None) };
    let game = crate::nwsync_content::game_storage(gi).map_err(anyhow::Error::msg)?;
    let o = crate::nwsync_content::WriteOptions { progress, ..Default::default() };
    let files = crate::nwsync_content::write_haks(&settings, game.as_ref(), haks, tlk, on_disk, &o)
        .map_err(|e| anyhow::anyhow!("the module's NWSync content ({}): {e}", settings.url))?;
    Ok(Some(files))
}
