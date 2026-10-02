//! Globs as nasher matches them: a port of the `glob` Nim package (the
//! version nasher pins) — its translation to a regular expression and its
//! walk of the file system.
//!
//! `*` and `?` stay within a folder, `**` spans folders, `[...]` and
//! `[!...]` are character sets, `{a,b}` alternatives. Walking skips hidden
//! entries (names starting with `.`) and doesn't follow links to folders.

use std::path::{Path, PathBuf};

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
            // After a `**`, the glob package drops separators.
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
    to_regex(pattern, cfg!(windows))
        .ok()
        .and_then(|rx| Regex::new(&rx).ok())
        .is_some_and(|rx| rx.is_match(name))
}

/// The files a pattern finds, relative to `root` unless absolute, as
/// absolute paths in the order the walk meets them. As in nasher's build,
/// names compare ignoring case.
pub(crate) fn walk(root: &Path, pattern: &str) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let pattern = pattern.replace('\\', "/");
    if !has_magic(&pattern) {
        let p = root.join(&pattern);
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
    let (base, magic) = match pattern.find(|c| ['*', '?', '[', '{'].contains(&c)) {
        Some(first) => match pattern[..first].rfind('/') {
            Some(slash) => (&pattern[..slash], &pattern[slash + 1..]),
            None => ("", &pattern[..]),
        },
        None => ("", &pattern[..]),
    };
    let dir = if base.is_empty() { root.to_path_buf() } else { root.join(base) };
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
        let root = std::env::temp_dir().join(format!("mg-glob-{}", std::process::id()));
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
        let _ = std::fs::remove_dir_all(&root);
    }
}
