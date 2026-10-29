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
//!
//! What the GPU refuses is the same: a texture it won't take is left out
//! ([`note_gpu_error`]), and an error of a frame's drawing, which wgpu
//! would end the program for, is noted instead on a device given to
//! [`note_gpu_errors`] (the application's: the frame draws what the GPU
//! takes). Each is said once ([`take_gpu_errors`]).
//!
//! [`blank_draws`] makes every scene draw nothing (tests): what a driver
//! that draws nothing does, for the startup check ([`crate::probe`]) and
//! the area view's watch ([`crate::watch`]) to be proved on a GPU that
//! draws.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    /// How many guarded steps are running on this thread, one inside
    /// another.
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// What failed, since last taken.
static FAILURES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// What the GPU refused, since last taken; and every error said, each
/// once (a frame's error comes again every frame).
static GPU_ERRORS: Mutex<(Vec<String>, Vec<String>)> = Mutex::new((Vec::new(), Vec::new()));

/// How many of the GPU's errors are said: past them, it says no more.
const GPU_ERRORS_SAID: usize = 64;

/// The model that fails on purpose (tests).
static FAILING: Mutex<Option<String>> = Mutex::new(None);

/// The scene's draws are made to draw nothing ([`blank_draws`]).
static BLANK: AtomicBool = AtomicBool::new(false);

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

/// Notes that the GPU refused what was asked of it for `what` (a
/// texture, a frame's drawing), with its error: once, for
/// [`take_gpu_errors`].
pub fn note_gpu_error(what: &str, error: &str) {
    let said = format!("{what}: {}", one_line(error));
    let Ok(mut errors) = GPU_ERRORS.lock() else { return };
    let (new, all) = &mut *errors;
    if all.len() < GPU_ERRORS_SAID && !all.contains(&said) {
        all.push(said.clone());
        new.push(said);
    }
}

/// Makes `device`'s errors that no error scope takes noted
/// ([`note_gpu_error`]) where wgpu would panic: a frame the GPU refuses
/// is drawn as far as it takes it, and Moonglow goes on. For the
/// application's device; tests keep wgpu's panic, which shows a mistake
/// where it is made.
pub fn note_gpu_errors(device: &wgpu::Device) {
    device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| {
        let what = match &e {
            wgpu::Error::OutOfMemory { .. } => "the GPU's memory",
            _ => "drawing",
        };
        note_gpu_error(what, &e.to_string());
    }));
}

/// What the GPU refused since the last call (each said once), for the log.
pub fn take_gpu_errors() -> Vec<String> {
    GPU_ERRORS.lock().map(|mut e| std::mem::take(&mut e.0)).unwrap_or_default()
}

/// wgpu's error on one line: its lines, each trimmed, but "Caused by:".
fn one_line(error: &str) -> String {
    let lines = error.lines().map(str::trim).filter(|l| !l.is_empty() && *l != "Caused by:");
    lines.collect::<Vec<_>>().join(": ")
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

/// The game's program whose pipelines are made to fail (tests).
static FAILING_PROGRAM: Mutex<Option<String>> = Mutex::new(None);

/// Makes making a pipeline with the game's program `name` fail (`None`:
/// none), as wgpu fails it (a pipeline it refuses), to test what falls
/// back from it: as Direct3D 12 would where FXC can't compile a program.
#[doc(hidden)]
pub fn fail_program(name: Option<&str>) {
    if let Ok(mut failing) = FAILING_PROGRAM.lock() {
        *failing = name.map(str::to_ascii_lowercase);
    }
}

/// Whether [`fail_program`] names `name`.
pub(crate) fn program_made_to_fail(name: &str) -> bool {
    let asked = FAILING_PROGRAM.lock().ok().and_then(|f| f.clone());
    asked.is_some_and(|a| a == name.to_ascii_lowercase())
}

/// The game's programs are held back ([`hold_programs`]).
static HELD: AtomicBool = AtomicBool::new(false);

/// Holds back the worker that makes the game's pipelines (`false`: lets
/// it go on), to test what waits for them for as long as the test wants,
/// whatever the machine's speed. Process-wide: a test that sets it takes
/// the GPU alone.
#[doc(hidden)]
pub fn hold_programs(on: bool) {
    HELD.store(on, Ordering::SeqCst);
}

/// Whether [`hold_programs`] is on.
pub(crate) fn programs_held() -> bool {
    HELD.load(Ordering::SeqCst)
}

/// Textures read apart are never given back ([`stall_textures`]).
static STALLED: AtomicBool = AtomicBool::new(false);

/// Makes the textures read apart never come back (`false`: they come
/// again; those asked for meanwhile are lost), to test what tells a frame
/// that waits for ever: its instances are left out, and it is never
/// ready. Process-wide: a test that sets it takes the GPU alone.
#[doc(hidden)]
pub fn stall_textures(on: bool) {
    STALLED.store(on, Ordering::SeqCst);
}

/// Whether [`stall_textures`] is on.
pub(crate) fn textures_stalled() -> bool {
    STALLED.load(Ordering::SeqCst)
}

/// Makes every scene pass draw nothing (`false`: draw again), to test
/// what tells a renderer that draws nothing: each pass's draws are cut to
/// its first pixel (a scissor of one pixel at its top left corner), the
/// sky, Moonglow's meshes, the game's programs, lines and particles alike,
/// as Heed's Radeon drew through Direct3D 12 (the egui overlays drawn over
/// the picture, nothing in it). Process-wide: a test that sets it takes
/// the GPU alone (`mg_testkit::gpu::alone`).
#[doc(hidden)]
pub fn blank_draws(on: bool) {
    BLANK.store(on, Ordering::SeqCst);
}

/// Whether [`blank_draws`] is on.
pub fn draws_blanked() -> bool {
    BLANK.load(Ordering::SeqCst)
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

    #[test]
    fn what_the_gpu_refuses_is_said_once_on_a_line() {
        let error = "Validation Error\n\nCaused by:\n  In Device::create_texture\n    too big\n";
        note_gpu_error("the texture c_refused", error);
        note_gpu_error("the texture c_refused", error);
        let said = take_gpu_errors();
        let ours: Vec<&String> = said.iter().filter(|e| e.contains("c_refused")).collect();
        assert_eq!(
            ours,
            ["the texture c_refused: Validation Error: In Device::create_texture: too big"]
        );
        note_gpu_error("the texture c_refused", error);
        assert!(!take_gpu_errors().iter().any(|e| e.contains("c_refused")), "said once");
    }
}
