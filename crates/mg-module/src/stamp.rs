//! Where an exported ERF came from: the module's git commit (when the
//! module is kept in a git checkout), Moonglow's version and the module's
//! name. Export writes it into the archive's description, after the
//! builder's own text; Import reads it back and says when the module has
//! moved on since (an ERF merged from where the builder started, never
//! copied over newer work).
//!
//! The description is the place: the game and Aurora read an ERF's
//! entries and pass its description by (Aurora shows it), so the stamp
//! changes nothing that is imported. A resource of its own would be
//! imported into the module with the rest.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Module, ModuleLocation};

/// The first line of the stamp in a description.
const HEAD: &str = "Moonglow-Export: ";

/// The commit a module's files were at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// `git rev-parse HEAD`: the full hash.
    pub hash: String,
    /// Whether the module's files had changes not committed (`git status`
    /// names any, new files too).
    pub dirty: bool,
}

/// Where an export came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    /// The Moonglow that wrote it.
    pub version: String,
    /// The commit of the module's git checkout; `None`: not in one.
    pub commit: Option<Commit>,
    /// The module's name and tag (module.ifo).
    pub module: String,
    pub tag: String,
}

impl Stamp {
    /// The stamp of an export from `module` as it is on disk now.
    pub fn of(module: &Module) -> Stamp {
        let info = crate::query::info(module).ok();
        Stamp {
            version: mg_core::release::version().to_string(),
            commit: module_dir(module).and_then(|d| commit_at(&d)),
            module: info.as_ref().map(|i| i.name.clone()).unwrap_or_default(),
            tag: info.map(|i| i.tag).unwrap_or_default(),
        }
    }

    /// The stamp as the description's lines.
    pub fn to_text(&self) -> String {
        let base = match &self.commit {
            Some(c) if c.dirty => format!("git {} with uncommitted changes", c.hash),
            Some(c) => format!("git {}", c.hash),
            None => "none (not in a git checkout)".to_string(),
        };
        format!("{HEAD}{}\nBase: {base}\nModule: {} ({})", self.version, self.module, self.tag)
    }

    /// The short form, for a line of a log: `commit 5f96447 (with
    /// uncommitted changes) of NAME, Moonglow 2.0.0`.
    pub fn summary(&self) -> String {
        let base = match &self.commit {
            Some(c) if c.dirty => format!("commit {} (with uncommitted changes)", short(&c.hash)),
            Some(c) => format!("commit {}", short(&c.hash)),
            None => "no commit (not in a git checkout)".to_string(),
        };
        format!("{base} of {}, Moonglow {}", self.module, self.version)
    }
}

/// An abbreviated hash, as git shows one.
pub fn short(hash: &str) -> &str {
    &hash[..hash.len().min(7)]
}

/// A description with the stamp after the builder's own text (a blank
/// line between). A stamp already in `comment` (an archive's old
/// description, added to) is replaced.
pub fn describe(comment: &str, stamp: &Stamp) -> String {
    let (own, _) = read(comment);
    match own.trim_end() {
        "" => stamp.to_text(),
        own => format!("{own}\n\n{}", stamp.to_text()),
    }
}

/// A description's own text and the stamp in it, if it has one.
pub fn read(description: &str) -> (String, Option<Stamp>) {
    let text = description.replace("\r\n", "\n");
    let at = if text.starts_with(HEAD) { Some(0) } else { text.find(&format!("\n{HEAD}")) };
    let Some(at) = at else { return (text, None) };
    let own = text[..at].trim_end().to_string();
    let mut stamp =
        Stamp { version: String::new(), commit: None, module: String::new(), tag: String::new() };
    for line in text[at..].trim_start_matches('\n').lines() {
        if let Some(v) = line.strip_prefix(HEAD) {
            stamp.version = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("Base: git ") {
            let mut words = v.split_whitespace();
            stamp.commit = words
                .next()
                .map(|hash| Commit { hash: hash.to_string(), dirty: v.contains("uncommitted") });
        } else if let Some(v) = line.strip_prefix("Module: ") {
            let v = v.trim_end();
            match v.rfind(" (").filter(|_| v.ends_with(')')) {
                Some(i) => {
                    stamp.module = v[..i].to_string();
                    stamp.tag = v[i + 2..v.len() - 1].to_string();
                }
                None => stamp.module = v.to_string(),
            }
        }
    }
    (own, Some(stamp))
}

/// The folder a module's git checkout would hold it in: a project's root,
/// a module folder, an archive's folder.
fn module_dir(module: &Module) -> Option<PathBuf> {
    match module.location.as_ref()? {
        ModuleLocation::Project { root, .. } => Some(root.clone()),
        ModuleLocation::Folder(p) => Some(p.clone()),
        ModuleLocation::Archive(p) => p.parent().map(Path::to_path_buf),
    }
}

/// How long a git command may take before it is stopped (one asking for
/// credentials, or on a hung network folder, never ends).
const GIT_TIME_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir).args(args);
    let out = crate::process::output_within(&mut command, GIT_TIME_LIMIT).ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The commit of the git checkout `dir` is in, and whether the files
/// under `dir` have changes not committed. `None`: no checkout (or no
/// git).
pub fn commit_at(dir: &Path) -> Option<Commit> {
    let hash = git(dir, &["rev-parse", "--verify", "HEAD"])?;
    let dirty = git(dir, &["status", "--porcelain", "--", "."]).is_some_and(|s| !s.is_empty());
    Some(Commit { hash, dirty })
}

/// What an import is told of the export's base, against the module
/// imported into: `None` when there is nothing to warn of.
pub fn compare(stamp: &Stamp, module: &Module) -> Option<String> {
    let made = stamp.commit.as_ref()?;
    let dir = module_dir(module)?;
    let now = commit_at(&dir)?;
    let mut out = Vec::new();
    if made.hash != now.hash {
        let relation = if git(&dir, &["merge-base", "--is-ancestor", &made.hash, &now.hash])
            .is_some()
        {
            let behind =
                git(&dir, &["rev-list", "--count", &format!("{}..{}", made.hash, now.hash)])
                    .unwrap_or_else(|| "some".into());
            format!(
                "it was made from older work: the module is {behind} commits on, at {}. \
                 Import it on a branch from {} and merge, or what changed since is undone",
                short(&now.hash),
                short(&made.hash)
            )
        } else if git(&dir, &["cat-file", "-e", &format!("{}^{{commit}}", made.hash)]).is_some() {
            format!(
                "it was made from other work than the module's (commit {} is not behind {}): \
                 import it on a branch from {} and merge",
                short(&made.hash),
                short(&now.hash),
                short(&made.hash)
            )
        } else {
            format!(
                "it was made from commit {}, which this checkout doesn't have (fetch it), and \
                 the module is at {}",
                short(&made.hash),
                short(&now.hash)
            )
        };
        out.push(relation);
    }
    if made.dirty {
        out.push("it was made with changes that were not committed".to_string());
    }
    (!out.is_empty()).then(|| out.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(commit: Option<(&str, bool)>) -> Stamp {
        Stamp {
            version: "2.0.0".into(),
            commit: commit.map(|(h, d)| Commit { hash: h.into(), dirty: d }),
            module: "The Domains (of) Dread".into(),
            tag: "TDOD".into(),
        }
    }

    /// The stamp follows the builder's text and reads back as written; an
    /// old stamp is replaced, not added to.
    #[test]
    fn a_stamp_reads_back_from_the_description() {
        let s = stamp(Some(("5f96447aa", true)));
        let text = describe("Keep and gate", &s);
        assert_eq!(
            text,
            "Keep and gate\n\nMoonglow-Export: 2.0.0\nBase: git 5f96447aa with uncommitted \
             changes\nModule: The Domains (of) Dread (TDOD)"
        );
        assert_eq!(read(&text), ("Keep and gate".to_string(), Some(s.clone())));
        let again = describe(&text, &stamp(None));
        assert_eq!(read(&again), ("Keep and gate".to_string(), Some(stamp(None))));
        assert_eq!(read(&describe("", &s)), (String::new(), Some(s)));
        assert_eq!(read("Aurora's own text"), ("Aurora's own text".to_string(), None));
        assert_eq!(
            read("CRLF\r\n\r\nMoonglow-Export: 1.0\r\nBase: none").1.unwrap().version,
            "1.0"
        );
    }

    fn run(dir: &Path, args: &[&str]) -> bool {
        Command::new("git").arg("-C").arg(dir).args(args).output().is_ok_and(|o| o.status.success())
    }

    /// In a git checkout the stamp names its commit and whether files
    /// changed; Import says when the module has moved on since.
    #[test]
    fn the_commit_is_stamped_and_compared() {
        let dir = mg_testkit::scratch_dir("stamp-git");
        if !run(&dir, &["init", "-q"]) {
            eprintln!("skipped: no git");
            return;
        }
        for c in [["config", "user.email", "t@example.com"], ["config", "user.name", "T"]] {
            assert!(run(&dir, &c));
        }
        let mut m = Module::new();
        m.save_as(&ModuleLocation::Folder(dir.join("mod"))).unwrap();
        std::fs::write(dir.join("mod/a.txt"), "one").unwrap();
        // Before the first commit there is none.
        assert_eq!(Stamp::of(&m).commit, None);
        assert!(run(&dir, &["add", "."]) && run(&dir, &["commit", "-qm", "one"]));
        let first = Stamp::of(&m);
        let c = first.commit.clone().unwrap();
        assert_eq!((c.hash.len(), c.dirty), (40, false));
        assert_eq!(compare(&first, &m), None);
        std::fs::write(dir.join("mod/b.txt"), "new").unwrap();
        assert!(Stamp::of(&m).commit.unwrap().dirty);
        assert!(run(&dir, &["add", "."]) && run(&dir, &["commit", "-qm", "two"]));
        let warning = compare(&first, &m).unwrap();
        assert!(warning.contains("older work: the module is 1 commits on"), "{warning}");
        let gone = stamp(Some(("0123456789abcdef0123456789abcdef01234567", true)));
        let warning = compare(&gone, &m).unwrap();
        assert!(warning.contains("doesn't have") && warning.contains("not committed"), "{warning}");
        // Not in a checkout: nothing to compare.
        let elsewhere = mg_testkit::scratch_dir("stamp-no-git");
        let mut m = Module::new();
        m.save_as(&ModuleLocation::Folder(elsewhere.join("mod"))).unwrap();
        if Stamp::of(&m).commit.is_none() {
            assert_eq!(compare(&first, &m), None);
        }
    }
}
