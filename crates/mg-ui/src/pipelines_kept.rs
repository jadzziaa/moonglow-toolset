//! A module's pipelines of the game's programs, kept from one session to
//! the next (the owner's choice for the shaders proposal's M3, 2026-11-02:
//! "An area changes its look a moment after it opens"): the pipelines
//! made while the module was open (`mg_render::Renderer::game_pipelines_made`,
//! a line each) are written to a file of the module's in Moonglow's cache
//! folder ([`file`]), and made again in the background as the module
//! opens next ([`mg_render::Renderer::prepare_game_pipelines`]), so that an
//! area opened later draws the game's programs from its first frame. No
//! models are read for it. The list is bounded ([`MOST`]); a stale one is
//! harmless: a pair whose programs a hak no longer has is passed over, and
//! one that fails is reported only where a draw needs it.

use std::path::{Path, PathBuf};

use crate::Moonglow;

/// The most pipelines kept for a module (the newest), and the most lines
/// read: the stock areas need a few dozen, a builder's custom shaders a
/// few hundred.
pub const MOST: usize = 1000;

/// What the window did with the file of the open module's pipelines.
#[derive(Debug, Default)]
pub(crate) struct PipelinesKept {
    /// The module whose list was given to the renderer, and its file
    /// (worked out once: the module's path is made canonical for it).
    prepared: Option<(PathBuf, PathBuf)>,
    /// The renderer's count of pipelines made when the list was last
    /// written, and for which module.
    written: Option<(PathBuf, usize)>,
    /// The open module's list as its file had it, and how many pipelines
    /// the renderer had made when the module opened: its own are those
    /// and the ones made since ([`own_lines`]).
    had: Vec<String>,
    since: usize,
}

/// The pipelines to keep for a module: those of its kept list (`had`)
/// that are made (`made`: every pipeline made in this run, the earliest
/// first), then those made since it opened (from `since` on). The whole
/// of `made` was written: a second module opened in one run had the
/// first's pipelines written as its own before a view of it had drawn.
pub fn own_lines(had: &[String], made: &[String], since: usize) -> Vec<String> {
    let all: std::collections::HashSet<&String> = made.iter().collect();
    let mut seen = std::collections::HashSet::new();
    (had.iter().filter(|l| all.contains(l)))
        .chain(&made[since.min(made.len())..])
        .filter(|l| seen.insert(*l))
        .cloned()
        .collect()
}

/// The file in `dir` that keeps the pipelines of the module at `module`
/// (named by its canonical path, as its open tabs' file is).
pub fn file(dir: &Path, module: &Path) -> PathBuf {
    mg_module::locks::file(dir, module).with_extension("txt")
}

/// The lines of a module's file, at most [`MOST`] (none where there is
/// none, or it isn't text). (The file is read whole: it is the window's
/// own, written at most [`MOST`] lines long.)
pub fn read(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    text.lines().filter(|l| !l.trim().is_empty()).take(MOST).map(str::to_string).collect()
}

/// Writes `lines` (the newest [`MOST`] of them, the earliest first) as
/// the module's file at `path`.
pub fn write(path: &Path, lines: &[String]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let lines = &lines[lines.len().saturating_sub(MOST)..];
    let mut text = lines.join("\n");
    text.push('\n');
    std::fs::write(path, text)
}

impl Moonglow {
    /// Once a frame: the open module's kept pipelines given to the
    /// renderer, once, as it opens (with the Shaders switch on); and the
    /// pipelines made written for it, when more have been made and the
    /// worker has none left to make.
    pub(crate) fn keep_pipelines(&mut self) {
        let (Some(dir), Some(module)) = (self.pipelines_dir.clone(), self.module_path()) else {
            return;
        };
        let Some(vp) = self.viewport.as_mut() else { return };
        if !vp.renderer.game_shaders {
            return;
        }
        let path = match &self.pipelines_kept.prepared {
            Some((m, path)) if *m == module => path.clone(),
            _ => {
                let Some(game) = self.game.as_deref() else { return };
                let path = file(&dir, &module);
                let lines = read(&path);
                // (Before its own are asked for: those made so far are
                // another module's, or none.)
                self.pipelines_kept.since = vp.renderer.game_shader_stats().pipelines;
                self.pipelines_kept.had = lines.clone();
                let asked = vp.renderer.prepare_game_pipelines(&vp.gpu, &game.resman, &lines);
                log::debug!(
                    "{asked} of {} kept pipelines asked for, {}",
                    lines.len(),
                    path.display()
                );
                self.pipelines_kept.prepared = Some((module.clone(), path.clone()));
                path
            }
        };
        let made = vp.renderer.game_shader_stats().pipelines;
        let written = self.pipelines_kept.written.as_ref();
        let more = written.is_none_or(|(m, n)| *m != module || *n != made);
        if more && made > 0 && vp.renderer.drawn.game_pending == 0 {
            let kept = &self.pipelines_kept;
            let lines = own_lines(&kept.had, &vp.renderer.game_pipelines_made(), kept.since);
            if let Err(e) = write(&path, &lines) {
                log::warn!("the module's pipelines not kept in {}: {e}", path.display());
            }
            self.pipelines_kept.written = Some((module, made));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A module's list is written and read back the same, its newest
    /// [`MOST`] alone; a file that isn't there reads as none.
    #[test]
    fn a_module_s_pipelines_are_kept_and_read_back() {
        let dir = mg_testkit::scratch_dir("pipelines-kept");
        let module = dir.join("castle.mod");
        let path = file(&dir, &module);
        assert!(read(&path).is_empty());
        assert!(path.extension().is_some_and(|e| e == "txt"), "{}", path.display());
        let lines: Vec<String> =
            (0..MOST + 5).map(|i| format!("vs{i} fslit opaque 1000")).collect();
        write(&path, &lines).unwrap();
        let back = read(&path);
        assert_eq!(back.len(), MOST);
        assert_eq!(back[0], "vs5 fslit opaque 1000", "the newest kept");
        assert_eq!(back.last(), lines.last());
        assert_ne!(file(&dir, &dir.join("other.mod")), path, "a file a module");
    }

    /// A module's own pipelines are those of its kept list that are made
    /// and those made since it opened: not the ones a module open before
    /// it in the same run had made (the review of November 2026's note on
    /// `pipelines_kept`).
    #[test]
    fn a_module_s_own_pipelines_are_not_the_one_before_s() {
        let line = |n: &str| n.to_string();
        // (Made for the first module: a, b. The second opened then; its
        // list had b and z, of which z is no longer made; c was made for
        // it since.)
        let made = [line("a"), line("b"), line("c")];
        let had = [line("b"), line("z")];
        assert_eq!(own_lines(&had, &made, 2), ["b", "c"]);
        // The first module of a run: all that is made, its list's first.
        assert_eq!(own_lines(&[line("b")], &made, 0), ["b", "a", "c"]);
        assert!(own_lines(&[], &made, 3).is_empty(), "nothing of its own yet");
        assert!(own_lines(&had, &[], 5).is_empty());
    }
}
