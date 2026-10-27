//! A few GPU devices at a time in a test binary.
//!
//! libtest runs a binary's tests on many threads; tests that each open a
//! wgpu device (the area viewer, the renderer, previews) would otherwise
//! have a dozen or more Vulkan devices open at once, a load that once
//! brought a developer's machine down (a kernel panic during the test
//! suite, cause unlogged). A test calls [`hold`] before it opens a device
//! and keeps its place until it ends; past [`places`] at once, the other
//! GPU tests wait their turn. Tests that do not use the GPU run in
//! parallel as before.
//!
//! One device at a time made the UI tests one long queue (each opens and
//! draws in about a fifth of a second: 42 s of the UI tests' 42), so a few
//! share the GPU now: `MG_TEST_GPUS` sets how many (1 for one at a time).

use std::cell::RefCell;
use std::sync::OnceLock;

use crate::places::{Places, Taken};

static GPU: Places = Places::new();

/// How many tests may hold the GPU at once: `MG_TEST_GPUS`, else 4.
pub fn places() -> usize {
    static PLACES: OnceLock<usize> = OnceLock::new();
    *PLACES.get_or_init(|| {
        let set = std::env::var("MG_TEST_GPUS").ok().and_then(|v| v.parse().ok());
        set.filter(|&n| n > 0).unwrap_or(4)
    })
}

thread_local! {
    /// This thread's places, while its test holds them; given back when
    /// the test's thread ends (libtest runs each test on its own thread),
    /// also when it fails.
    static HELD: RefCell<Option<Taken>> = const { RefCell::new(None) };
}

/// Takes a place on the GPU for the rest of the calling test, waiting while
/// [`places`] others have one. Calling it again in the same test does
/// nothing.
pub fn hold() {
    take(1);
}

/// Takes the GPU alone for the rest of the calling test, waiting for every
/// other GPU test to end: for a test that changes what every render does
/// (`mg_render::guard::fail_on`). Called after [`hold`], it waits for the
/// others too.
pub fn alone() {
    take(places());
}

fn take(wanted: usize) {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        if held.as_ref().is_some_and(|t| t.count >= wanted) {
            return;
        }
        // (What it held is given back while it waits for more, so that two
        // tests waiting to be alone do not wait for each other.)
        *held = None;
        *held = Some(GPU.take(wanted, places()));
    });
}

/// Gives the places back before the test ends (for a test whose GPU part
/// is over); [`hold`] takes one again.
pub fn release() {
    HELD.with(|held| held.borrow_mut().take());
}
