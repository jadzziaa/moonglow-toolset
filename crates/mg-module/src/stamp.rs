//! Where an exported ERF came from: the module's git commit (when the
//! module is kept in a git checkout), Moonglow's version and the module's
//! name. Export writes it into the archive's description, after the
//! builder's own text; Import reads it back and says when the module has
//! moved on since (an ERF merged from where the builder started, never
//! copied over newer work).
//!
//! A `.mod` outside any checkout (a builder's download) names its base
//! itself: a module variable [`BASE_VARIABLE`], a string holding the
//! commit the download was made from, which the download's script
//! writes. A `.mod`'s own variable is its base before the folder it lies
//! in (a download put in a clone is the download's); a folder's or a
//! project's checkout is its base before the variable.
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

/// The module variable (a string, on the module: `module.ifo`'s
/// `VarTable`) naming the commit a module was made from, where it isn't
/// in a checkout of its own.
pub const BASE_VARIABLE: &str = "moonglow_base";

/// The commit a module's files were at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// `git rev-parse HEAD`: the full hash.
    pub hash: String,
    /// Whether the module's files had changes not committed (`git status`
    /// names any, new files too).
    pub dirty: bool,
    /// Whether the module named it ([`BASE_VARIABLE`]) rather than its
    /// checkout: its changes since are not known.
    pub named: bool,
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
            commit: base_of(module),
            module: info.as_ref().map(|i| i.name.clone()).unwrap_or_default(),
            tag: info.map(|i| i.tag).unwrap_or_default(),
        }
    }

    /// The stamp as the description's lines.
    pub fn to_text(&self) -> String {
        let base = match &self.commit {
            Some(c) if c.named => format!("git {} (the module's {BASE_VARIABLE})", c.hash),
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
            Some(c) if c.named => {
                format!("commit {} (the module's {BASE_VARIABLE})", short(&c.hash))
            }
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
            // (A commit's hash alone: the description is another's text,
            // and what is read is handed to git.)
            let mut words = v.split_whitespace();
            stamp.commit = words.next().filter(|h| is_hash(h)).map(|hash| Commit {
                hash: hash.to_string(),
                dirty: v.contains("uncommitted"),
                named: v.contains(BASE_VARIABLE),
            });
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

/// Whether `text` is a commit's hash (or the start of one, 7 digits on).
fn is_hash(text: &str) -> bool {
    (7..=64).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The commit the module names itself ([`BASE_VARIABLE`]), if it names
/// one.
fn named_base(module: &Module) -> Option<Commit> {
    use mg_gff::Value;
    let info = module.info().ok()?;
    let Some(Value::List(vars)) = info.root.get("VarTable") else { return None };
    let var = vars.iter().find(|v| {
        v.string("Name") == Some(BASE_VARIABLE.as_bytes())
            && v.integer("Type") == Some(i64::from(crate::vars::VarType::String.code()))
    })?;
    let hash = String::from_utf8_lossy(var.string("Value")?).trim().to_ascii_lowercase();
    is_hash(&hash).then_some(Commit { hash, dirty: false, named: true })
}

/// A module's base: an archive's own variable, else its checkout's
/// commit, else (a folder or project not in one) the variable.
fn base_of(module: &Module) -> Option<Commit> {
    let checkout = || module_dir(module).and_then(|d| commit_at(&d));
    match module.location.as_ref() {
        Some(ModuleLocation::Archive(_)) => named_base(module).or_else(checkout),
        _ => checkout().or_else(|| named_base(module)),
    }
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
    Some(Commit { hash, dirty, named: false })
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
            commit: commit.map(|(h, d)| Commit { hash: h.into(), dirty: d, named: false }),
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

        // A builder's `.mod`, out of any checkout, naming its base itself
        // (the download's script wrote the variable): stamped with that
        // commit, which Import compares as the checkout's own.
        let mut download = Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        let var = |name: &str, ty: u32, value: &str| {
            let mut v = mg_gff::Struct::new(0);
            v.set("Name", mg_gff::Value::String(name.as_bytes().to_vec()));
            v.set("Type", mg_gff::Value::Dword(ty));
            v.set("Value", mg_gff::Value::String(value.as_bytes().to_vec()));
            v
        };
        let vars = vec![var(BASE_VARIABLE, 1, "7"), var(BASE_VARIABLE, 3, &c.hash.to_uppercase())];
        info.root.set("VarTable", mg_gff::Value::List(vars));
        download.set_info(&info).unwrap();
        download.save_as(&ModuleLocation::Archive(elsewhere.join("download.mod"))).unwrap();
        let named = Stamp::of(&download);
        let base = named.commit.clone().expect("the module's own base");
        assert_eq!((base.hash.as_str(), base.dirty, base.named), (c.hash.as_str(), false, true));
        let text = describe("Gate", &named);
        assert!(text.contains(&format!("Base: git {} (the module's moonglow_base)", c.hash)));
        assert_eq!(read(&text).1, Some(named.clone()));
        assert!(named.summary().contains("(the module's moonglow_base)"), "{}", named.summary());
        let warning = compare(&named, &Module::open(&dir.join("mod")).unwrap()).unwrap();
        assert!(warning.contains("older work: the module is 1 commits on"), "{warning}");
        // A `.mod` in a checkout that names its own base: its own first (a
        // download put in a clone); a folder's checkout before its own.
        download.save_as(&ModuleLocation::Archive(dir.join("download.mod"))).unwrap();
        assert!(Stamp::of(&download).commit.unwrap().named);
        download.save_as(&ModuleLocation::Folder(dir.join("download"))).unwrap();
        assert!(!Stamp::of(&download).commit.unwrap().named);
        // A value that is no commit's hash isn't a base; nor is text after
        // "Base: git " in another's description that isn't a hash.
        info.root.set("VarTable", mg_gff::Value::List(vec![var(BASE_VARIABLE, 3, "--all")]));
        download.set_info(&info).unwrap();
        download.save_as(&ModuleLocation::Archive(elsewhere.join("download.mod"))).unwrap();
        assert!(Stamp::of(&download).commit.is_none_or(|c| !c.named), "not the variable");
        assert_eq!(read("Moonglow-Export: 2.0\nBase: git --output=x").1.unwrap().commit, None);
    }
}
