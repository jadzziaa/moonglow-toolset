//! Moonglow updating itself (GitHub issue 28; the design in
//! `docs/update-install-proposal.md`): the releases GitHub lists, the
//! package of a release this copy takes, its checks against the release's
//! `SHA256SUMS` signed with minisign, the download beside what it
//! replaces, and the install.
//!
//! Nothing here reaches the network by itself: the requests are an
//! [`Http`]'s, which the desktop app gives ([`http::Client`], the feature
//! `http`) and the tests make up or point at a server of their own on this
//! computer (`serve`, the feature `serve`).

pub mod http;
pub mod install;
pub mod kind;
pub mod release;
#[cfg(any(test, feature = "serve"))]
pub mod serve;
pub mod stage;
pub mod verify;
pub mod version;

pub use http::Http;
pub use install::{Restart, Step};
pub use kind::{Facts, Installer, Kind};
pub use release::{Api, Asset, LATEST_API, LIST_API, RELEASES_PAGE, Release};
pub use stage::{Ready, Staged};
pub use verify::PublicKey;
pub use version::Version;

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    /// A fresh scratch folder under `target/test-output/mg-update/` (or
    /// `MOONGLOW_TEST_OUTPUT`'s), as `mg_testkit::scratch_dir` makes them.
    pub(crate) fn scratch(name: &str) -> PathBuf {
        let root = match std::env::var_os("MOONGLOW_TEST_OUTPUT") {
            Some(dir) if !dir.is_empty() => PathBuf::from(dir),
            _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-output"),
        };
        let dir = root.join("mg-update").join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
