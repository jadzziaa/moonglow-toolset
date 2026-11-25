//! The GPU hold of Moonglow's tests, alone: [`gpu::hold`] before a test
//! opens a device, so that a test binary has a few open at a time
//! ([`gpu`]), and the places tests take turns at ([`places`]).
//!
//! mg-testkit re-exports both (`mg_testkit::gpu`, `mg_testkit::places`):
//! one crate, so one process-wide set of places, whichever path a test
//! binary reaches it by. A client of Moonglow's crates (Moonglow Client,
//! Moonglow Viewer) that wants the hold without the rest of the test kit
//! (its server runner, updater and signing) takes this crate; it must take
//! it from the same release as mg-testkit, or Cargo builds two and the
//! binary has two sets of places.
//!
//! Beyond the process, a test also takes one slot of the machine
//! ([`slots`]), so that test binaries run at once keep the same few devices
//! open between them.

pub mod gpu;
pub mod places;
pub mod slots;

pub use gpu::{alone, hold, holding, open, release};
