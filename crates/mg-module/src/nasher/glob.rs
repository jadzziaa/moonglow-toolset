//! Globs as nasher matches them: a port of the `glob` Nim package (the
//! version nasher pins) — its translation to a regular expression and its
//! walk of the file system.
//!
//! `*` and `?` stay within a folder, `**` spans folders, `[...]` and
//! `[!...]` are character sets, `{a,b}` alternatives. Walking skips hidden
//! entries (names starting with `.`) and doesn't follow links to folders.

use std::path::{Component, Path, PathBuf};

use regex_lite::Regex;

const GLOB_META: &[char] = &['\\', '*', '?', '[', '{'];
const REGEX_META: &[char] = &['.', '^', '$', '+', '{', '[', ']', '|', '(', ')'];

/// Whether a pattern has glob syntax (`*`, `?`, `[` or `{`, or an extglob).
pub(crate) fn has_magic(s: &str) -> bool {
    s.contains(['*', '?', '[', '{'])
        || s.as_bytes()
            .windows(2)
            .any(|w| matches!(w[0], b'?' | b'!' | b'@' | b'+') && w[1] == b'(')
}

/// The regular expression a glob stands for, anchored at both ends.
pub(crate) fn to_regex(pattern: &str, ignore_case: bool) -> Result<String, String> {
    let chars: Vec<char> = pattern.chars().collect();
    let peek = |i: usize| chars.get(i).copied().unwrap_or('\0');
    let mut rx = String::from("^");
    if ignore_case {
        rx.push_str("(?i)");
    }
    let (mut globstar, mut in_range, mut in_group) = (false, false, false);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                if i + 1 == chars.len() {
                    return Err(format!("{pattern}: no character to escape"));
                }
                let next = peek(i + 1);
                if GLOB_META.contains(&next) || REGEX_META.contains(&next) {
                    rx.push('\\');
                }
                rx.push(next);
                i += 1;
            }
            // After a `**`, the glob package drops every later separator
            // (nasher 1.1.3 lists `src/x/subd.nss` for `src/**/sub/*.nss`).
            '/' if globstar => {}
            '/' => rx.push('/'),
            '(' | '|' | ')' | '+' | '@' => {
                rx.push('\\');
                rx.push(c);
            }
            '!' if in_range => rx.push('^'),
            '!' => rx.push_str("\\!"),
            '?' => rx.push_str("[^/]"),
            '[' => {
                let next = peek(i + 1);
                rx.push('[');
                match next {
                    '!' => rx.push('^'),
                    '-' => rx.push('-'),
                    '^' => rx.push_str("\\^"),
                    ']' => rx.push_str("\\]"),
                    _ => {
                        in_range = true;
                        i += 1;
                        continue;
                    }
                }
                i += 1;
                in_range = true;
            }
            ']' => {
                in_range = false;
                rx.push(']');
            }
            '-' => rx.push('-'),
            '{' if in_group => return Err(format!("{pattern}: groups can't be nested")),
            '{' => {
                rx.push_str("(?:(?:");
                in_group = true;
            }
            '}' if in_group => {
                rx.push_str("))");
                in_group = false;
            }
            ',' if in_group => rx.push_str(")|(?:"),
            '*' if peek(i + 1) == '*' => {
                globstar = true;
                rx.push_str("(?:[^/]*(?:/|$))*");
                i += 1;
            }
            '*' => rx.push_str("[^/]*"),
            c => {
                if REGEX_META.contains(&c) {
                    rx.push('\\');
                }
                rx.push(c);
            }
        }
        i += 1;
    }
    if in_range {
        return Err(format!("{pattern}: missing ']'"));
    }
    if in_group {
        return Err(format!("{pattern}: missing '}}'"));
    }
    rx.push('$');
    Ok(rx)
}

/// Whether `name` matches `pattern` (nasher's rules: case-sensitive, except
/// on Windows).
pub(crate) fn matches(name: &str, pattern: &str) -> bool {
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<String, Option<Regex>>> =
            Default::default();
    }
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(rx) = cache.get(pattern) {
            return rx.as_ref().is_some_and(|rx| rx.is_match(name));
        }
        if cache.len() > 256 {
            cache.clear();
        }
        let compiled = to_regex(pattern, cfg!(windows)).ok().and_then(|rx| Regex::new(&rx).ok());
        let matched = compiled.as_ref().is_some_and(|rx| rx.is_match(name));
        cache.insert(pattern.to_string(), compiled);
        matched
    })
}

/// Whether `pattern` finds the file `path`, as [`walk`] would (whether it
/// is there or not: a file about to be written). For many files, a
/// [`Cover`] made once.
#[cfg(test)]
pub(crate) fn covers(root: &Path, pattern: &str, path: &Path) -> bool {
    Cover::new(root, pattern).covers(path)
}

/// A pattern under a root, made ready to be asked of many files whether
/// it finds them ([`covers`]): its regular expression is compiled once,
/// not for each file (a project of 30,000 sources compiled 90,000 at
/// every open).
#[derive(Debug, Clone)]
pub(crate) struct Cover {
    /// The folder the pattern's wildcards begin in; for a pattern without
    /// any, the file or folder it names.
    base: PathBuf,
    /// The wildcards' expression, for the path from `base` (none: a
    /// pattern without wildcards; `Some(None)`: one that is no pattern,
    /// which finds nothing).
    rest: Option<Option<Regex>>,
}

impl Cover {
    pub(crate) fn new(root: &Path, pattern: &str) -> Cover {
        let pattern = pattern.replace('\\', "/");
        if !has_magic(&pattern) {
            return Cover { base: root.join(&pattern), rest: None };
        }
        let (base, magic) = base_and_rest(&pattern);
        let base = if base.is_empty() { root.to_path_buf() } else { root.join(base) };
        let rest = to_regex(magic, true).ok().and_then(|rx| Regex::new(&rx).ok());
        Cover { base, rest: Some(rest) }
    }

    pub(crate) fn covers(&self, path: &Path) -> bool {
        let Some(rel) = strip_prefix_ci(path, &self.base) else { return false };
        let Some(rest) = &self.rest else {
            return rel.as_os_str().is_empty() || !has_hidden_component(&rel);
        };
        if rel.as_os_str().is_empty() || has_hidden_component(&rel) {
            return false;
        }
        let rel = rel.to_string_lossy().replace('\\', "/");
        rest.as_ref().is_some_and(|rx| rx.is_match(&rel))
    }
}

fn strip_prefix_ci(path: &Path, base: &Path) -> Option<PathBuf> {
    let mut pc = path.components();
    for bc in base.components() {
        let p = pc.next()?;
        let same = match (p, bc) {
            (Component::Normal(a), Component::Normal(b)) => {
                a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
            }
            (Component::Prefix(a), Component::Prefix(b)) => a
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&b.as_os_str().to_string_lossy()),
            (a, b) => a == b,
        };
        if !same {
            return None;
        }
    }
    Some(pc.as_path().to_path_buf())
}

fn has_hidden_component(rel: &Path) -> bool {
    rel.components().any(|c| match c {
        Component::Normal(s) => {
            let s = s.to_string_lossy();
            s.len() >= 2 && s.starts_with('.') && s != ".."
        }
        _ => false,
    })
}

/// A pattern's leading folders without glob syntax, and the rest from
/// its first wildcard's folder on (`a/b/*.nss`: `a/b` and `*.nss`).
fn base_and_rest(pattern: &str) -> (&str, &str) {
    match pattern.find(|c| ['*', '?', '[', '{'].contains(&c)) {
        Some(first) => match pattern[..first].rfind('/') {
            Some(slash) => (&pattern[..slash], &pattern[slash + 1..]),
            None => ("", pattern),
        },
        None => ("", pattern),
    }
}

/// `rel` under `root`, each folder or file name found ignoring case
/// where the exact spelling is not there (nasher 1.1.3 on Linux lists
/// `src/areas/f.nss` for `SRC/**/*.nss` and for `src/AREAS`), as
/// [`Cover::covers`] compares the base folder.
fn resolve_ignoring_case(root: &Path, rel: &str) -> PathBuf {
    let mut p = root.to_path_buf();
    for c in Path::new(rel).components() {
        let Component::Normal(name) = c else {
            p.push(c.as_os_str());
            continue;
        };
        // The name as the folder lists it, not as the pattern spells it: on
        // Windows and macOS `SRC` opens `src`, and the paths found must be
        // the files' own (as `Cover` and the walk below give them).
        let wanted = name.to_string_lossy();
        let found = std::fs::read_dir(&p).ok().and_then(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name())
                .filter(|n| n.to_string_lossy().eq_ignore_ascii_case(&wanted))
                .min_by_key(|n| (n != name, n.clone()))
        });
        p = found.map_or_else(|| p.join(name), |n| p.join(n));
    }
    p
}

/// The files a pattern finds, relative to `root` unless absolute, as
/// absolute paths in the order the walk meets them. As in nasher's build,
/// names compare ignoring case, the leading folders' too.
pub(crate) fn walk(root: &Path, pattern: &str) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let pattern = pattern.replace('\\', "/");
    if !has_magic(&pattern) {
        let p = resolve_ignoring_case(root, &pattern);
        if p.is_file() {
            out.push(p);
            return Ok(out);
        }
        if !p.is_dir() {
            return Ok(out);
        }
        return walk(root, &format!("{}/**", pattern.trim_end_matches('/')));
    }
    // The leading folders without glob syntax, and the rest.
    let (base, magic) = base_and_rest(&pattern);
    let dir = if base.is_empty() { root.to_path_buf() } else { resolve_ignoring_case(root, base) };
    let rx = Regex::new(&to_regex(magic, true)?).map_err(|e| e.to_string())?;
    let mut stack = vec![dir.clone()];
    while let Some(sub) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&sub) else { continue };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.len() >= 2 && name.starts_with('.') && name != ".." {
                continue;
            }
            let path = e.path();
            let Ok(kind) = e.file_type() else { continue };
            let rel = path.strip_prefix(&dir).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_symlink() && path.is_dir() {
                // Links to folders are neither yielded nor followed.
            } else if rx.is_match(&rel) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pattern made ready once finds the files the walk finds, and no
    /// others; asked again and again, the same.
    #[test]
    fn a_cover_finds_what_the_walk_finds() {
        let root = mg_testkit::scratch_dir("mg-glob-cover");
        let files = [
            "src/a.nss",
            "src/b.json",
            "src/areas/c.are.json",
            "src/areas/deep/d.git.json",
            "src/win.jui",
            "src/old/e.nss",
            "other/f.nss",
            "g.nss",
        ];
        for f in files {
            let path = root.join(f);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"").unwrap();
        }
        for pattern in [
            "src/**/*.{nss,json}",
            "src/*.nss",
            "src/**/*",
            "src\\areas\\**\\*.json",
            "src/old/**",
            "**/*.nss",
            "*.nss",
            "src/win.jui",
            "src/areas",
            "src/[ab].*",
            "src/{a,win}.{nss,jui}",
            "src/[",
            "nothing/**/*",
            "SRC/**/*.nss",
            "src/AREAS",
            "Src/Areas/**/*.json",
            "**/areas/*.json",
        ] {
            let cover = Cover::new(&root, pattern);
            let mut walked = walk(&root, pattern).unwrap_or_default();
            walked.sort();
            let mut covered: Vec<PathBuf> =
                files.iter().map(|f| root.join(f)).filter(|p| cover.covers(p)).collect();
            covered.sort();
            assert_eq!(covered, walked, "{pattern}");
            assert!(
                files
                    .iter()
                    .all(|f| cover.covers(&root.join(f)) == covers(&root, pattern, &root.join(f)))
            );
        }
        // A file not there yet, about to be written.
        assert!(Cover::new(&root, "src/**/*.jui").covers(&root.join("src/new/x.jui")));
        assert!(!Cover::new(&root, "src/**/*.jui").covers(&root.join("elsewhere/x.jui")));
    }

    #[test]
    fn globs_translate_as_the_glob_package_does() {
        assert_eq!(
            to_regex("src/**/*.{nss,json}", false).unwrap(),
            r"^src/(?:[^/]*(?:/|$))*[^/]*\.(?:(?:nss)|(?:json))$"
        );
        assert_eq!(to_regex("hook_?.nss", false).unwrap(), r"^hook_[^/]\.nss$");
        assert_eq!(to_regex("[!a-c]x", false).unwrap(), r"^[^a-c]x$");
        assert!(matches("hook_a.nss", "hook_*.nss"));
        assert!(!matches("dir/hook_a.nss", "hook_*.nss"));
        assert!(matches("x.utc", "*"));
        assert!(matches("area.git", "*.{are,git,gic}"));
        assert!(to_regex("{a,{b}}", false).is_err());
        assert!(has_magic("a/*.nss") && has_magic("+(x)") && !has_magic("a/b.nss"));
    }

    #[test]
    fn walking_finds_files_by_pattern() {
        let root = mg_testkit::scratch_dir("mg-glob");
        let _ = std::fs::remove_dir_all(&root);
        for f in [
            "src/a.nss",
            "src/sub/b.utc.json",
            "src/sub/deep/c.nss",
            "src/.hidden/d.nss",
            "README.md",
        ] {
            let p = root.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, "").unwrap();
        }
        let rel = |v: Vec<PathBuf>| -> Vec<String> {
            let mut v: Vec<String> = v
                .iter()
                .map(|p| p.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/"))
                .collect();
            v.sort();
            v
        };
        assert_eq!(
            rel(walk(&root, "src/**/*.{nss,json}").unwrap()),
            ["src/a.nss", "src/sub/b.utc.json", "src/sub/deep/c.nss"]
        );
        assert_eq!(rel(walk(&root, "src/*.nss").unwrap()), ["src/a.nss"]);
        assert_eq!(rel(walk(&root, "src/*.NSS").unwrap()), ["src/a.nss"]);
        assert_eq!(
            rel(walk(&root, "src/sub").unwrap()),
            ["src/sub/b.utc.json", "src/sub/deep/c.nss"]
        );
        assert_eq!(rel(walk(&root, "README.md").unwrap()), ["README.md"]);
        assert!(walk(&root, "nothing/*.nss").unwrap().is_empty());
        // The leading folders found ignoring case too, as nasher does on Linux.
        assert_eq!(rel(walk(&root, "SRC/*.nss").unwrap()), ["src/a.nss"]);
        assert_eq!(
            rel(walk(&root, "Src/SUB").unwrap()),
            ["src/sub/b.utc.json", "src/sub/deep/c.nss"]
        );
        assert_eq!(rel(walk(&root, "readme.MD").unwrap()), ["README.md"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn globstar_drops_later_slashes_as_nasher_and_cover_matches_case_and_skips_hidden() {
        // The glob package drops every separator after a `**`: nasher 1.1.3
        // lists `src/suba.nss` and `src/x/subd.nss` for `src/**/sub/*.nss`,
        // and not `src/sub/a.nss` or `src/x/sub/b.nss`.
        assert_eq!(to_regex("**/sub/*.nss", false).unwrap(), r"^(?:[^/]*(?:/|$))*sub[^/]*\.nss$");
        assert_eq!(
            to_regex("src/**/areas/*.json", false).unwrap(),
            r"^src/(?:[^/]*(?:/|$))*areas[^/]*\.json$"
        );
        let sub_rx = Regex::new(&to_regex("src/**/sub/*.nss", false).unwrap()).unwrap();
        for (path, nasher) in [
            ("src/suba.nss", true),
            ("src/x/subd.nss", true),
            ("src/sub/a.nss", false),
            ("src/x/sub/b.nss", false),
            ("src/x/y/sub/c.nss", false),
        ] {
            assert_eq!(sub_rx.is_match(path), nasher, "{path}");
        }

        // Work with a dot-prefixed ancestor root to ensure only components
        // under `base` are treated as hidden.
        let dot_root = mg_testkit::scratch_dir(".worktree-cover-root").join("proj");
        let cover = Cover::new(&dot_root, "src/**/*.{nss,json}");
        assert!(cover.covers(&dot_root.join("SRC/Areas/Foo.JSON")));
        assert!(!cover.covers(&dot_root.join("src/.hidden/foo.nss")));
        assert!(!cover.covers(&dot_root.join("src/.git/hooks.nss")));

        let plain = Cover::new(&dot_root, "src/areas");
        assert!(plain.covers(&dot_root.join("SRC/AREAS/c.are.json")));
        assert!(!plain.covers(&dot_root.join("SRC/AREAS/.secret.json")));
    }
}
