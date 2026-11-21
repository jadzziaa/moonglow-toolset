//! A few at a time: places that tests of a binary take turns at (GPU
//! devices, servers), waiting while they are all taken.

use std::sync::{Condvar, Mutex, PoisonError};

/// A set of places, as many as the taker says (the count may be read from
/// the environment once, so it is not fixed here).
#[derive(Debug)]
pub struct Places {
    /// The places taken, and how many takers wait for all of them (new
    /// takers wait behind those, so that they get their turn).
    taken: Mutex<(usize, usize)>,
    freed: Condvar,
}

/// Places held, given back when dropped (also when the test holding them
/// fails: a poisoned count is still right).
#[derive(Debug)]
pub struct Taken {
    places: &'static Places,
    pub(crate) count: usize,
}

impl Places {
    pub const fn new() -> Places {
        Places { taken: Mutex::new((0, 0)), freed: Condvar::new() }
    }

    /// Takes `wanted` of `of` places, waiting until they are free. Taking
    /// them all waits for every other taker, and new ones wait for it.
    pub fn take(&'static self, wanted: usize, of: usize) -> Taken {
        let all = wanted >= of;
        let wanted = wanted.min(of);
        let mut taken = self.taken.lock().unwrap_or_else(PoisonError::into_inner);
        if all {
            taken.1 += 1;
        }
        while taken.0 + wanted > of || (!all && taken.1 > 0) {
            taken = self.freed.wait(taken).unwrap_or_else(PoisonError::into_inner);
        }
        if all {
            taken.1 -= 1;
        }
        taken.0 += wanted;
        Taken { places: self, count: wanted }
    }

    #[cfg(test)]
    pub(crate) fn state(&self) -> (usize, usize) {
        *self.taken.lock().unwrap()
    }
}

impl Default for Places {
    fn default() -> Places {
        Places::new()
    }
}

impl Drop for Taken {
    fn drop(&mut self) {
        self.places.taken.lock().unwrap_or_else(PoisonError::into_inner).0 -= self.count;
        self.places.freed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    use super::Places;

    #[test]
    fn no_more_than_the_places_at_once_and_one_taking_all_is_alone() {
        static PLACES: Places = Places::new();
        static INSIDE: AtomicUsize = AtomicUsize::new(0);
        static MOST: AtomicUsize = AtomicUsize::new(0);
        static ALL: AtomicBool = AtomicBool::new(false);
        let ms = |n: u64| Duration::from_millis(n);
        let mut threads: Vec<_> = (0..16)
            .map(|i| {
                spawn(move || {
                    sleep(ms(i % 8));
                    let _one = PLACES.take(1, 4);
                    assert!(!ALL.load(Ordering::SeqCst), "one ran beside one taking all");
                    let now = INSIDE.fetch_add(1, Ordering::SeqCst) + 1;
                    MOST.fetch_max(now, Ordering::SeqCst);
                    sleep(ms(3));
                    INSIDE.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect();
        for _ in 0..2 {
            threads.push(spawn(move || {
                sleep(ms(4));
                let _all = PLACES.take(4, 4);
                assert_eq!(INSIDE.load(Ordering::SeqCst), 0);
                assert!(!ALL.swap(true, Ordering::SeqCst), "two took all at once");
                sleep(ms(5));
                ALL.store(false, Ordering::SeqCst);
            }));
        }
        for t in threads {
            t.join().unwrap();
        }
        let most = MOST.load(Ordering::SeqCst);
        assert!((2..=4).contains(&most), "{most} at once");
        assert_eq!(PLACES.state(), (0, 0), "all given back");
    }
}
