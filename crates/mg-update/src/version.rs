//! A release's version, as its tag names it.

use std::cmp::Ordering;

/// A release's version: `major.minor.patch`, compared as numbers (2.4.0 is
/// before 2.10.0), a pre-release (`2.5.0-rc.1`) before its release and
/// pre-releases among themselves by semver's precedence (`rc.2` after
/// `rc.1`, `rc.10` after `rc.2`, `beta` before `rc`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// The pre-release part (`rc.1`), if there is one.
    pub pre: Option<String>,
}

impl Version {
    /// `2.4.0`, `v2.4.0`, `2.5.0-rc.1`, `2.4.0+build` (the build part is
    /// passed over, as semver has it).
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let text = text.strip_prefix(['v', 'V']).unwrap_or(text);
        let text = text.split_once('+').map_or(text, |(v, _)| v);
        let (numbers, pre) = match text.split_once('-') {
            Some((n, p)) => {
                let ok = |id: &str| {
                    !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                };
                if !p.split('.').all(ok) {
                    return None;
                }
                (n, Some(p.to_string()))
            }
            None => (text, None),
        };
        let mut parts = numbers.split('.');
        let mut number = || {
            let p = parts.next()?;
            (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())).then(|| p.parse().ok())?
        };
        let (major, minor, patch) = (number()?, number()?, number()?);
        parts.next().is_none().then_some(Version { major, minor, patch, pre })
    }

    /// This program's version (`mg_core::release`).
    pub fn current() -> Version {
        Version::parse(mg_core::release::version()).expect("the release is semver")
    }

    /// A pre-release.
    pub fn is_pre(&self) -> bool {
        self.pre.is_some()
    }

    /// As the packages' names have it: `3.3.0`, `3.3.0-rc.1`.
    pub fn file_part(&self) -> String {
        match &self.pre {
            Some(p) => format!("{}.{}.{}-{p}", self.major, self.minor, self.patch),
            None => format!("{}.{}.{}", self.major, self.minor, self.patch),
        }
    }
}

/// Two pre-release parts by semver's precedence.
fn pre_order(a: &str, b: &str) -> Ordering {
    let mut a = a.split('.');
    let mut b = b.split('.');
    loop {
        match (a.next(), b.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    // (Numbers before words.)
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => x.cmp(y),
                };
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => pre_order(a, b),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `2.10.3`, `3.3.0-rc.1` (which [`Version::parse`] reads back).
impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.file_part())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert!(v("2.4.0") < v("2.10.0"));
        assert!(v("2.9.9") < v("3.0.0"));
        assert!(v("v2.4.1") > v("2.4.0"));
        assert_eq!(v("v2.4.0"), v("2.4.0"));
        assert_eq!(v("2.4.0+linux"), v("2.4.0"));
        // A pre-release comes before its release.
        assert!(v("2.5.0-rc.1") < v("2.5.0"));
        assert!(v("2.5.0-rc.1") > v("2.4.0"));
        assert!(v("2.5.0-rc.1").is_pre() && !v("2.5.0").is_pre());
        for bad in
            ["", "2.4", "2.4.0.1", "two.4.0", "2..0", "2.4.0-", "+2.4.0", "2.-4.0", "2.4.0-a..b"]
        {
            assert_eq!(Version::parse(bad), None, "{bad:?}");
        }
        assert_eq!(Version::current(), v(mg_core::release::version()));
        assert_eq!(v("2.10.3").to_string(), "2.10.3");
    }

    #[test]
    fn pre_releases_compare_by_semver_precedence() {
        assert!(v("3.3.0-rc.1") < v("3.3.0-rc.2"));
        assert!(v("3.3.0-rc.2") < v("3.3.0-rc.10"));
        assert!(v("3.3.0-beta.9") < v("3.3.0-rc.1"));
        assert!(v("3.3.0-rc") < v("3.3.0-rc.1"));
        assert!(v("3.3.0-1") < v("3.3.0-alpha"));
        assert_eq!(v("v3.3.0-rc.1").to_string(), "3.3.0-rc.1");
        // (What is shown is read back: a skipped version is kept so.)
        assert_eq!(v(&v("3.3.0-rc.1").to_string()), v("3.3.0-rc.1"));
    }
}
