//! Moonglow's cache folder for a test binary of mg-ui: `moonglow-cache`
//! in the tests' output folder (`mg_testkit::test_output`, so a second
//! run beside the first with `MOONGLOW_TEST_OUTPUT` has its own), set as
//! `MOONGLOW_CACHE_DIR` before `main`, so that no test writes the
//! user's (`~/.cache/moonglow`: the NWSync cache, Test Module's
//! `test-user`, a build's scratch folder). Before `main` because the
//! tests make their windows on threads of their own, hundreds of ways,
//! and the environment can be changed safely only while one thread runs.
//! Not emptied from run to run. Included by each binary's root
//! (`#[path = "support/own_cache.rs"] mod own_cache;`, from `app/`
//! `../support/own_cache.rs`).

/// The folder.
pub(crate) fn dir() -> std::path::PathBuf {
    mg_testkit::test_output().join("moonglow-cache")
}

#[allow(unsafe_code)]
extern "C" fn set() {
    // SAFETY: called once by the system's loader before `main`, while
    // the process has one thread: nothing reads the environment at once.
    unsafe { std::env::set_var("MOONGLOW_CACHE_DIR", dir()) };
}

/// `set`, in the section of what the loader runs before `main`.
#[used]
#[allow(unsafe_code)]
#[cfg_attr(any(target_os = "linux", target_os = "freebsd"), unsafe(link_section = ".init_array"))]
#[cfg_attr(target_vendor = "apple", unsafe(link_section = "__DATA,__mod_init_func"))]
#[cfg_attr(windows, unsafe(link_section = ".CRT$XCU"))]
static SET: extern "C" fn() = set;
