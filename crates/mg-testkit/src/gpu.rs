//! One GPU device at a time in a test binary.
//!
//! libtest runs a binary's tests on many threads; tests that each open a
//! wgpu device (the area viewer, the renderer, previews) would otherwise
//! have a dozen or more Vulkan devices open at once, a load that has
//! brought a developer's machine down (a kernel panic during the test
//! suite, cause unlogged). A test calls [`hold`] before it opens a device
//! and keeps the GPU until it ends; the other GPU tests wait their turn.
//! Tests that do not use the GPU run in parallel as before.

use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard};

static GPU: Mutex<()> = Mutex::new(());

thread_local! {
    /// The lock, while this thread's test holds it; dropped when the test's
    /// thread ends (libtest runs each test on its own thread), also when it
    /// fails.
    static HELD: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
}

/// Takes the GPU for the rest of the calling test, waiting for any other
/// test that has it. Calling it again in the same test does nothing.
pub fn hold() {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        if held.is_none() {
            // A test that failed while holding it poisons it; the GPU is
            // still free to use.
            *held = Some(GPU.lock().unwrap_or_else(|e| e.into_inner()));
        }
    });
}

/// Gives the GPU back before the test ends (for a test whose GPU part is
/// over); [`hold`] takes it again.
pub fn release() {
    HELD.with(|held| held.borrow_mut().take());
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static INSIDE: AtomicUsize = AtomicUsize::new(0);
    static MOST: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn one_test_at_a_time_holds_the_gpu() {
        let threads: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    super::hold();
                    super::hold();
                    let now = INSIDE.fetch_add(1, Ordering::SeqCst) + 1;
                    MOST.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    INSIDE.fetch_sub(1, Ordering::SeqCst);
                    // Released when the thread ends.
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(MOST.load(Ordering::SeqCst), 1);
        // And free again afterwards.
        super::hold();
        super::release();
    }
}
