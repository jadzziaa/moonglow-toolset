//! The GPU hold is one set of places whichever path a test reaches it by:
//! `mg_testkit::gpu` (re-exported) and `mg_testkit_gpu::gpu` (its own
//! crate, which a client takes alone).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[test]
fn both_paths_share_one_set_of_places() {
    static HELD: AtomicBool = AtomicBool::new(false);
    // Alone by one path: a hold by the other waits until it is given back.
    mg_testkit::gpu::alone();
    let other = std::thread::spawn(|| {
        mg_testkit_gpu::gpu::hold();
        HELD.store(true, Ordering::SeqCst);
    });
    std::thread::sleep(Duration::from_millis(200));
    assert!(!HELD.load(Ordering::SeqCst), "a hold by the other path ran beside one alone");
    mg_testkit::gpu::release();
    other.join().unwrap();
    assert!(HELD.load(Ordering::SeqCst));
    assert_eq!(mg_testkit::gpu::places(), mg_testkit_gpu::gpu::places());
}
