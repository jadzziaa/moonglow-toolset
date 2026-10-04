//! Carrying on when one model fails.
//!
//! Custom content holds surprises no parser test has met: a model whose
//! data is read without an error and then breaks what builds or poses it.
//! A panic there would end the toolset, and with it the module being
//! edited. [`guarded`] runs such a step and, if it panics, notes what
//! failed and gives `None`: the caller shows the object without its model,
//! as it shows one whose model is missing.
//!
//! The failures are kept ([`take_failures`]) for the application to tell
//! the user; while a guarded step runs, [`guarding`] is true, so that a
//! panic hook can tell a failure Moonglow carries on from a crash.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;

thread_local! {
    /// How many guarded steps are running on this thread, one inside
    /// another.
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// What failed, since last taken.
static FAILURES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The model that fails on purpose (tests).
static FAILING: Mutex<Option<String>> = Mutex::new(None);

/// Whether a guarded step is running on this thread: a panic now is
/// caught, and Moonglow carries on.
pub fn guarding() -> bool {
    DEPTH.with(|d| d.get() > 0)
}

/// Runs `step` (loading, building or posing `what`: a model's name). If it
/// panics, the failure is noted for [`take_failures`] and `None` returned.
pub fn guarded<T>(what: &str, step: impl FnOnce() -> T) -> Option<T> {
    DEPTH.with(|d| d.set(d.get() + 1));
    let result = catch_unwind(AssertUnwindSafe(step));
    DEPTH.with(|d| d.set(d.get() - 1));
    match result {
        Ok(value) => Some(value),
        Err(payload) => {
            let why = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "an unknown failure".into());
            let failure = format!("{what}: {why}");
            if let Ok(mut failures) = FAILURES.lock()
                && !failures.contains(&failure)
            {
                failures.push(failure);
            }
            None
        }
    }
}

/// The failures noted since the last call (each once), for the log.
pub fn take_failures() -> Vec<String> {
    FAILURES.lock().map(|mut f| std::mem::take(&mut *f)).unwrap_or_default()
}

/// Makes building the model named `name` fail (`None`: none), to test what
/// carries on from it.
#[doc(hidden)]
pub fn fail_on(name: Option<&str>) {
    if let Ok(mut failing) = FAILING.lock() {
        *failing = name.map(str::to_ascii_lowercase);
    }
}

/// Panics if `name` is the model [`fail_on`] names.
pub(crate) fn fail_if_asked(name: &str) {
    let asked = FAILING.lock().ok().and_then(|f| f.clone());
    if asked.is_some_and(|a| a == name.to_ascii_lowercase()) {
        panic!("made to fail");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_that_panics_is_noted_and_gives_nothing() {
        assert_eq!(guarded("fine", || 3), Some(3));
        assert!(!guarding());
        let inner = guarded("c_broken", || {
            assert!(guarding());
            let empty: Vec<u8> = Vec::new();
            empty[std::hint::black_box(4)]
        });
        assert_eq!(inner, None);
        assert!(!guarding(), "over once the step is");
        let failures = take_failures();
        let ours: Vec<&String> = failures.iter().filter(|f| f.starts_with("c_broken: ")).collect();
        assert_eq!(ours.len(), 1, "{failures:?}");
        assert!(ours[0].contains("index out of bounds"), "{ours:?}");
        assert!(!take_failures().iter().any(|f| f.starts_with("c_broken")), "taken once");
    }
}
