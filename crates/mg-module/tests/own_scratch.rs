//! The run's own folder as the other crates' tests see it: the library built
//! without `cfg(test)`, as mg-ui's and `mg`'s tests link it.

/// The workspace's tests make their run folders where `.cargo/config.toml`
/// puts them (`MOONGLOW_SCRATCH_DIR`, under `target/test-output/`), never in
/// the user's cache folder: hundreds of them had gathered in
/// `~/.cache/moonglow/scratch` from the UI tests' script editor.
#[test]
fn a_test_run_s_own_folder_is_under_the_test_output() {
    let base = std::env::var_os("MOONGLOW_SCRATCH_DIR")
        .expect("cargo sets MOONGLOW_SCRATCH_DIR for the workspace's tests (.cargo/config.toml)");
    let base = std::path::PathBuf::from(base);
    assert!(base.ends_with("target/test-output/own-scratch"), "{}", base.display());
    let dir = mg_module::build::own_scratch_dir().unwrap();
    assert_eq!(dir.parent(), Some(base.as_path()));
    assert!(dir.is_dir());
    if let Some(cache) = mg_module::build::cache_dir() {
        assert!(!dir.starts_with(cache), "{}", dir.display());
    }
}
