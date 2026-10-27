//! The release of Moonglow this program is: the version of the two
//! programs, `mg` and `moonglow`, which each sets here as it starts
//! ([`set`]). The libraries have a version of their own that a release
//! leaves alone ([`LIBRARIES`]), so that a release rebuilds the two
//! programs and nothing else; what they write of the release (an
//! export's stamp, an NWSync manifest's `created_with`, the About box, the
//! debug log, the update check, the compiled shaders' cache) reads it from
//! here.

use std::sync::OnceLock;

static RELEASE: OnceLock<&'static str> = OnceLock::new();

/// The libraries' own version, which a release does not change: the
/// release where no program set one (a library's tests, Moonglow
/// Viewer).
pub const LIBRARIES: &str = env!("CARGO_PKG_VERSION");

/// Sets the release, as a program starts: its own version,
/// `env!("CARGO_PKG_VERSION")`. Only the first call counts.
pub fn set(version: &'static str) {
    let _ = RELEASE.set(version);
}

/// The release the program set, else [`LIBRARIES`].
pub fn version() -> &'static str {
    RELEASE.get().copied().unwrap_or(LIBRARIES)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_first_release_set_is_the_one() {
        super::set("2.8.0");
        super::set("9.9.9");
        assert_eq!(super::version(), "2.8.0");
    }
}
